//! The logger inside `KINGDOM HEARTS FINAL MIX.exe` (Steam). It changes nothing the game reads:
//! one inline hook on the per-actor task `0x140297FE0` (checked first bytes), which only reads
//! Sora's fields and globals while recording. F9 starts / stops recording; each start writes a
//! new `kh1-trace-<n>.csv` next to the exe. `kh1-trace.log` there says what happened.
//!
//! Row n = frame n's inputs (read when Sora's task starts, before his think consumes the command
//! word) + the state after frame n (read when his task starts on frame n+1, after the motion
//! update `0x140298730` and everything else that frame ran).

use std::ffi::c_void;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, Once};

use crate::kh1::{self, actor as a};
use crate::schema::{self, Hex, Row};

#[repr(C)]
struct MemoryBasicInformation {
    base: usize,
    alloc_base: usize,
    alloc_protect: u32,
    partition: u16,
    region_size: usize,
    state: u32,
    protect: u32,
    kind: u32,
}

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(name: *const u16) -> *mut c_void;
    fn GetModuleFileNameW(module: *mut c_void, buf: *mut u16, size: u32) -> u32;
    fn VirtualAlloc(addr: *mut c_void, size: usize, kind: u32, prot: u32) -> *mut c_void;
    fn VirtualProtect(addr: *mut c_void, size: usize, prot: u32, old: *mut u32) -> i32;
    fn VirtualQuery(addr: *const c_void, info: *mut MemoryBasicInformation, size: usize) -> usize;
    fn FlushInstructionCache(process: *mut c_void, addr: *const c_void, size: usize) -> i32;
    fn GetCurrentProcess() -> *mut c_void;
    fn GetCurrentProcessId() -> u32;
}

#[link(name = "user32")]
extern "system" {
    fn GetAsyncKeyState(vk: i32) -> i16;
    fn GetForegroundWindow() -> *mut c_void;
    fn GetWindowThreadProcessId(window: *mut c_void, pid: *mut u32) -> u32;
}

const DLL_PROCESS_ATTACH: u32 = 1;
const DLL_PROCESS_DETACH: u32 = 0;
const VK_F9: i32 = 0x78;

static BASE: AtomicUsize = AtomicUsize::new(0);
static ORIGINAL: AtomicUsize = AtomicUsize::new(0);
static RECORDING: AtomicBool = AtomicBool::new(false);
static REC: Mutex<Option<Recorder>> = Mutex::new(None);
static INSTALL: Once = Once::new();
static LOG: Mutex<Option<File>> = Mutex::new(None);

fn exe_path() -> PathBuf {
    let mut buf = vec![0u16; 1024];
    // SAFETY: a null module names the exe; the buffer length is passed.
    let n = unsafe { GetModuleFileNameW(std::ptr::null_mut(), buf.as_mut_ptr(), buf.len() as u32) } as usize;
    PathBuf::from(String::from_utf16_lossy(&buf[..n.min(buf.len())]))
}

fn exe_dir() -> PathBuf {
    exe_path().parent().map(|p| p.to_path_buf()).unwrap_or_default()
}

fn log(line: &str) {
    if let Ok(mut g) = LOG.lock() {
        if g.is_none() {
            *g = File::create(exe_dir().join("kh1-trace.log")).ok();
        }
        if let Some(f) = g.as_mut() {
            let _ = writeln!(f, "{line}");
            let _ = f.flush();
        }
    }
}

/// Whether `len` bytes at `at` are committed, readable memory.
fn readable(at: usize, len: usize) -> bool {
    if at < 0x10000 {
        return false;
    }
    let mut m = MemoryBasicInformation { base: 0, alloc_base: 0, alloc_protect: 0, partition: 0, region_size: 0, state: 0, protect: 0, kind: 0 };
    // SAFETY: VirtualQuery only writes the struct whose size is passed.
    let n = unsafe { VirtualQuery(at as *const c_void, &mut m, std::mem::size_of::<MemoryBasicInformation>()) };
    n != 0 && m.state == 0x1000 && m.protect & 0xFF != 0 && m.protect & 0x101 == 0 && at + len <= m.base + m.region_size
}

/// Reads a `T` at an address the caller knows is mapped (the exe's globals, Sora's actor).
unsafe fn rd<T: Copy>(at: usize) -> T {
    std::ptr::read_unaligned(at as *const T)
}

/// A handle → pointer, as `0x14038AF40` does (0 for none or an empty table slot).
unsafe fn handle(base: usize, h: u32) -> usize {
    if h == 0 {
        return 0;
    }
    let slot: u64 = rd(base + kh1::HANDLE_TABLE + (((h & 0x7FFF_FFFF) >> 25) as usize) * 8);
    if slot == u64::MAX {
        return 0;
    }
    (slot | (h & 0x1FF_FFFF) as u64) as usize
}

struct Recorder {
    out: BufWriter<File>,
    path: PathBuf,
    rows: u64,
    /// Frame n's inputs, waiting for its end state.
    pending: Option<Row>,
    actor: usize,
}

impl Recorder {
    /// Sora's task is starting: the previous frame is over (write it), then this frame's inputs.
    unsafe fn before(&mut self, base: usize, sora: usize) {
        if let Some(mut row) = self.pending.take() {
            if self.actor == sora {
                state(&mut row, base, sora);
                row.frame = self.rows;
                if row.write_csv(&mut self.out).is_ok() {
                    self.rows += 1;
                }
                if self.rows.is_multiple_of(60) {
                    let _ = self.out.flush();
                }
            }
        }
        self.actor = sora;
        self.pending = Some(inputs(base, sora));
    }

    /// Sora's task is done: the move vector his think left this frame.
    unsafe fn after(&mut self, base: usize) {
        if let Some(r) = &mut self.pending {
            let v = base + kh1::MOVE_VEC;
            r.in_move_x = rd(v);
            r.in_move_z = rd(v + 8);
            r.in_move_w = rd(v + 12);
        }
    }
}

/// Frame inputs, read before Sora's think.
unsafe fn inputs(base: usize, sora: usize) -> Row {
    let mut r = Row { step: rd(base + kh1::FRAME_STEP), ..Default::default() };
    r.freeze = rd::<u32>(sora + a::PAUSE) & 1 != 0;
    let pad = base + kh1::LOGICAL_PAD;
    let held: u32 = rd(pad + 4);
    let pressed: u32 = rd(pad + 8);
    let tilt: f32 = rd(pad + 0x4C);
    let s = kh1::stick(rd(pad + 0x60), rd(pad + 0x64), tilt);
    r.in_stick_x = s[0];
    r.in_stick_y = s[1];
    r.in_tilt = if s == [0.0, 0.0] { 0.0 } else { tilt };
    r.in_held = Hex(held);
    r.in_pressed = Hex(pressed);
    r.in_jump = schema::button(held, pressed, schema::BTN_JUMP);
    r.in_attack = schema::button(held, pressed, schema::BTN_COMMAND);
    r.in_square = schema::button(held, pressed, schema::BTN_SQUARE);
    r.in_command = Hex(rd(base + kh1::COMMAND));
    let dev: usize = rd(base + kh1::DEVICE_PTR);
    if readable(dev + 0x18, 0x20) && rd::<u8>(dev + 0x18 + 0x1F) == 1 {
        r.in_raw_x = rd::<u8>(dev + 0x18 + 4) as i32;
        r.in_raw_y = rd::<u8>(dev + 0x18 + 5) as i32;
    }
    r.in_cam_raw = rd(base + kh1::CAM_YAW);
    r.in_cam_raw2 = rd(base + kh1::CAM_YAW2);
    r.in_cam_mode = rd(base + kh1::CAM_MODE);
    r.in_cam_yaw = schema::kh1_camera_to_core(r.in_cam_raw, r.in_cam_raw2);
    // can_act (0x140297870) as the frame starts.
    let flags: u32 = rd(sora + a::FLAGS);
    r.in_can_act = flags & 4 == 0 && rd::<u32>(sora + a::PAUSE) & 0x20 == 0 && rd::<u32>(sora + a::STAT_EXTRA) & 1 == 0;
    r.lock_state = rd(base + kh1::LOCK_STATE);
    r.in_lock = r.lock_state != 0;
    r.abilities = Hex(rd(base + kh1::ABILITIES));
    let save: usize = rd(base + kh1::SAVE_PTR);
    if readable(save + kh1::ABILITY_LIST, 48) {
        r.combo_master = kh1::combo_master(std::slice::from_raw_parts((save + kh1::ABILITY_LIST) as *const u8, 48));
    }
    let st = handle(base, rd(sora + a::STATS));
    if readable(st + a::ST_AIR_COMBO_MAX, 1) {
        r.combo_max = rd::<u8>(st + a::ST_COMBO_MAX) as i32;
        r.air_combo_max = rd::<u8>(st + a::ST_AIR_COMBO_MAX) as i32;
    }
    r
}

/// The state after a frame, read when the next one starts.
unsafe fn state(r: &mut Row, base: usize, s: usize) {
    let flags: u32 = rd(s + a::FLAGS);
    let mflags: u32 = rd(s + a::MOTION_FLAGS);
    r.flags = Hex(flags);
    r.no_flinch = rd(s + a::NO_FLINCH);
    r.pos_x = rd(s + a::POS);
    r.pos_y = rd(s + a::POS + 4);
    r.pos_z = rd(s + a::POS + 8);
    r.move_x = rd(s + a::MOVE_DIR);
    r.move_z = rd(s + a::MOVE_DIR + 8);
    r.speed = rd(s + a::SPEED);
    r.facing = rd(s + a::FACING);
    r.state = rd(s + a::STATE);
    r.prev_state = rd(s + a::PREV_STATE);
    r.target_yaw = rd(s + a::TARGET_YAW);
    r.request = rd(s + a::REQUEST);
    r.entry = rd(s + a::ENTRY);
    r.motion_frame = rd(s + a::MOTION_FRAME);
    r.blend = rd(s + a::BLEND);
    r.blend_clock = rd(s + a::BLEND_CLOCK);
    r.motion_flags = Hex(mflags);
    r.action_lock = flags & 4 != 0;
    r.hit_active = mflags & schema::MF_HIT_ANY != 0;
    r.combo_window = mflags & schema::MF_COMBO_WINDOW != 0;
    // The physics step's airborne test (0x14029A3C0): +0x2D0 bit 0 set and no supporting object.
    r.grounded = !(rd::<u32>(s + a::CONTACT) & 1 != 0 && rd::<u32>(s + a::SUPPORT) == 0);
    r.height = rd(s + a::HEIGHT);
    r.air_t = rd(s + a::AIR_T);
    r.rise_t = rd(s + a::RISE_T);
    r.air_vx = rd(s + a::AIR_VEL);
    r.air_vy = rd(s + a::AIR_VEL + 4);
    r.air_vz = rd(s + a::AIR_VEL + 8);
    let cb = handle(base, rd(s + a::COMBO));
    if readable(cb + a::CB_ROW, 0x14) {
        r.row = rd::<u8>(cb + a::CB_ROW) as i32;
        r.combo_counter = rd::<u8>(cb + a::CB_COUNTER) as i32;
        r.cb_flags = Hex(rd::<u16>(cb + a::CB_FLAGS) as u32);
        r.swing_hits = Hex(rd(cb + a::CB_HITS));
    }
    if r.in_lock {
        let t = handle(base, rd(base + kh1::LOCK_TARGET));
        if t != s && readable(t + a::POS, 12) {
            r.target_x = rd(t + a::POS);
            r.target_y = rd(t + a::POS + 4);
            r.target_z = rd(t + a::POS + 8);
            r.target_part = rd(base + kh1::LOCK_TARGET + 4);
        }
    }
}

/// The hook: `0x140297FE0(task)`.
unsafe extern "system" fn actor_task(task: usize) -> u64 {
    // SAFETY: ORIGINAL holds the trampoline before the jump to here is written.
    let original: unsafe extern "system" fn(usize) -> u64 = std::mem::transmute(ORIGINAL.load(Ordering::Acquire));
    if !RECORDING.load(Ordering::Relaxed) {
        return original(task);
    }
    let base = BASE.load(Ordering::Relaxed);
    let actor: usize = rd(task + 0x18);
    let sora: usize = rd(base + kh1::SORA_PTR);
    if actor == 0 || actor != sora {
        return original(task);
    }
    if let Ok(mut g) = REC.try_lock() {
        if let Some(r) = g.as_mut() {
            r.before(base, sora);
        }
    }
    let ret = original(task);
    if let Ok(mut g) = REC.try_lock() {
        if let Some(r) = g.as_mut() {
            r.after(base);
        }
    }
    ret
}

/// Executable memory within 2 GB of `near` (for the `jmp rel32`).
unsafe fn alloc_near(near: usize, size: usize) -> Option<usize> {
    let mut at = (near & !0xFFFF).saturating_sub(0x1000_0000);
    let low = near.saturating_sub(0x7000_0000);
    while at > low {
        let p = VirtualAlloc(at as *mut c_void, size, 0x3000, 0x40);
        if !p.is_null() {
            return Some(p as usize);
        }
        at -= 0x10000;
    }
    None
}

fn abs_jmp(to: usize) -> [u8; 14] {
    let mut b = [0u8; 14];
    b[0] = 0xFF;
    b[1] = 0x25;
    b[6..14].copy_from_slice(&(to as u64).to_le_bytes());
    b
}

/// Hooks `target` (first bytes `expect`, whole position-independent instructions, 8 bytes,
/// 8-aligned so the patch is one atomic store) to `ours`.
unsafe fn install(target: usize, expect: &[u8; 8], ours: usize) -> Result<(), String> {
    let have: [u8; 8] = rd(target);
    if &have != expect {
        return Err(format!("unexpected first bytes {have:02x?} at {target:#x}, not hooked"));
    }
    if !target.is_multiple_of(8) {
        return Err(format!("{target:#x} is not 8-aligned"));
    }
    let mem = alloc_near(target, 64).ok_or("no memory near the exe")?;
    // Trampoline: the moved instructions, then back to the rest of the function.
    std::ptr::copy_nonoverlapping(expect.as_ptr(), mem as *mut u8, 8);
    let back = abs_jmp(target + 8);
    std::ptr::copy_nonoverlapping(back.as_ptr(), (mem + 8) as *mut u8, back.len());
    // Stub (rel32-reachable): an absolute jump to our function.
    let stub = mem + 32;
    let go = abs_jmp(ours);
    std::ptr::copy_nonoverlapping(go.as_ptr(), stub as *mut u8, go.len());
    ORIGINAL.store(mem, Ordering::Release);
    let mut patch = [0x90u8; 8];
    patch[0] = 0xE9;
    patch[1..5].copy_from_slice(&((stub as i64 - (target as i64 + 5)) as i32).to_le_bytes());
    let mut old = 0u32;
    if VirtualProtect(target as *mut c_void, 8, 0x40, &mut old) == 0 {
        return Err("VirtualProtect failed".into());
    }
    std::ptr::write_volatile(target as *mut u64, u64::from_le_bytes(patch));
    let mut tmp = 0u32;
    VirtualProtect(target as *mut c_void, 8, old, &mut tmp);
    FlushInstructionCache(GetCurrentProcess(), target as *const c_void, 8);
    Ok(())
}

/// Checks the build and installs the hook (once).
fn start() {
    INSTALL.call_once(|| {
        log(&format!("kh1-trace {}", env!("CARGO_PKG_VERSION")));
        let exe = exe_path();
        log(&format!("exe: {}", exe.display()));
        let name_ok = exe.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(kh1::EXE_NAME));
        // SAFETY: a null name returns the exe's own image base; its first 0x1000 bytes are the
        // PE headers, always mapped.
        let base = unsafe { GetModuleHandleW(std::ptr::null()) } as usize;
        let pe = if base == 0 { None } else { kh1::pe_info(unsafe { std::slice::from_raw_parts(base as *const u8, 0x1000) }) };
        let build_ok = pe.as_ref().is_some_and(kh1::is_steam_build);
        match &pe {
            Some(i) => log(&format!("build: time stamp {:#010x}, size of image {:#x} -> {}", i.timestamp, i.size_of_image, if build_ok { "KH1 FM Steam (supported)" } else { "NOT the KH1 FM Steam build: nothing hooked" })),
            None => log("build: PE headers unreadable: nothing hooked"),
        }
        if !name_ok {
            log(&format!("the exe is not {}: nothing hooked", kh1::EXE_NAME));
        }
        if !(name_ok && build_ok) {
            return;
        }
        BASE.store(base, Ordering::Relaxed);
        // SAFETY: the Steam build is confirmed; the target lies in its .text.
        match unsafe { install(base + kh1::HOOK_RVA, &kh1::HOOK_PROLOGUE, actor_task as *const () as usize) } {
            Ok(()) => log(&format!("hooked the actor task at {:#x}; press F9 in game to start / stop recording", base + kh1::HOOK_RVA)),
            Err(e) => {
                log(&format!("actor task: {e}"));
                return;
            }
        }
        std::thread::spawn(keys);
    });
}

fn game_focused() -> bool {
    // SAFETY: plain Win32 queries.
    unsafe {
        let w = GetForegroundWindow();
        if w.is_null() {
            return false;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(w, &mut pid);
        pid == GetCurrentProcessId()
    }
}

fn next_path() -> PathBuf {
    let dir = exe_dir();
    (1..).map(|n| dir.join(format!("kh1-trace-{n}.csv"))).find(|p| !p.exists()).expect("a free name")
}

/// F9 polling (its own thread): starts and stops recording.
fn keys() {
    let mut was = false;
    loop {
        std::thread::sleep(std::time::Duration::from_millis(10));
        // SAFETY: plain Win32 query.
        let down = unsafe { GetAsyncKeyState(VK_F9) } as u16 & 0x8000 != 0 && game_focused();
        if down && !was {
            toggle();
        }
        was = down;
    }
}

fn toggle() {
    let Ok(mut g) = REC.lock() else { return };
    match g.take() {
        Some(mut r) => {
            RECORDING.store(false, Ordering::Relaxed);
            let _ = r.out.flush();
            log(&format!("stopped: {} frames in {}", r.rows, r.path.display()));
        }
        None => {
            let path = next_path();
            match File::create(&path) {
                Ok(f) => {
                    let mut out = BufWriter::with_capacity(1 << 16, f);
                    if schema::write_header(&mut out).is_ok() {
                        log(&format!("recording to {}", path.display()));
                        *g = Some(Recorder { out, path, rows: 0, pending: None, actor: 0 });
                        RECORDING.store(true, Ordering::Relaxed);
                    }
                }
                Err(e) => log(&format!("cannot write {}: {e}", path.display())),
            }
        }
    }
}

/// Panacea calls this after loading every mod DLL (`OpenKh.Research.Panacea/Panacea.cpp`,
/// `LoadDLLs`), before the game runs its frames: the hook goes in there.
///
/// # Safety
/// Called by Panacea with its mod folder path.
#[no_mangle]
pub unsafe extern "C" fn OnInit(_mod_path: *const u16) {
    start();
}

/// # Safety
/// Called by the Windows loader.
#[no_mangle]
pub unsafe extern "system" fn DllMain(_module: *mut c_void, reason: u32, _reserved: *mut c_void) -> i32 {
    match reason {
        DLL_PROCESS_ATTACH => {
            // If nothing calls OnInit (another loader), install from a thread shortly after.
            std::thread::spawn(|| {
                std::thread::sleep(std::time::Duration::from_millis(2000));
                start();
            });
        }
        DLL_PROCESS_DETACH => {
            if let Ok(mut g) = REC.try_lock() {
                if let Some(r) = g.as_mut() {
                    let _ = r.out.flush();
                }
            }
        }
        _ => {}
    }
    1
}
