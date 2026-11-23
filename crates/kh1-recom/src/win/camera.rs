//! M12 camera: KH1's and KH2's cameras as modes of Re:CoM's own `CPlayerCamera` (`crate::camera`,
//! `docs/recom-m12-camera-plan.md`).
//!
//! - **Records:** a block near the exe holds two stubs at Re:CoM's record-table reads (its mode
//!   setters `0x1401FC4D0` / `0x1401FBE60`): modes 6..9 read our records (Re:CoM's format), every
//!   other mode Re:CoM's table, and each of our mode sets is counted (an entry for our step).
//! - **Dispatch:** the camera init's two `lea`s (the only references to its field / battle execs)
//!   point at our dispatcher. Camera = Re:CoM with a Re:CoM mode: Re:CoM's exec (one atomic load and
//!   one compare). Our mode: Re:CoM's battle-exec record restore, our KH1 / KH2 step, then Re:CoM's own
//!   tail (collision object `0x1401FC0D0`, the eye node, the view `0x14024FA40`), as its execs end.
//!   Re:CoM's base follows (1 / 3 / 4) switch to ours with Re:CoM's own `0x1401FBE60`; a sleight's
//!   request (`+0x188 & 0x2000`) and Re:CoM's timed view 2 are never replaced; sleights give our mode
//!   back through Re:CoM's `+0x17C`.
//! - **Init:** after Re:CoM's init `0x1401FB530`, our base mode when the setting asks for it, and
//!   the camera placed behind Sora on its first step.

use super::hook;
use super::log;
use super::movement::{func, rd, rd_vec, wr};
use crate::camera::{self, kh1, kh2, mode, Dispatch, Family};
use kh1_core::Vec3;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

/// The setting (0 Re:CoM, 1 KH1, 2 KH2), read by the dispatcher each frame.
static CHOICE: AtomicU32 = AtomicU32::new(0);
/// The Auto camera row (stage 2).
static AUTO: AtomicBool = AtomicBool::new(true);
/// Riku's story, read at each camera init (`0x1401AAAE0`).
static RIKU: AtomicBool = AtomicBool::new(false);
/// The block near the exe (`camera::stub::layout`); 0 not installed.
static BLOCK: AtomicUsize = AtomicUsize::new(0);
static ORIG_INIT: AtomicUsize = AtomicUsize::new(0);
static LOGS: AtomicU32 = AtomicU32::new(0);

/// `CPlayerCamera` fields. Checked (decomp `0x1401FB530`, `0x1401FD750`, `0x1401FECE0`).
mod f {
    pub const DT: usize = 0x14;
    pub const STATE: usize = 0xA0;
    pub const BATTLE_STATE: usize = 0x118;
    /// The sleight actor whose records override modes 3 / 4 (`0x1401FEB50`).
    pub const OVERRIDE_ACTOR: usize = 0xD0;
    pub const PLAYER: usize = 0x160;
    pub const MODE: usize = 0x178;
    pub const SAVED_MODE: usize = 0x17C;
    pub const FLAGS: usize = 0x188;
    pub const AIM: usize = 0x220;
    pub const EYE: usize = 0x230;
    pub const AIM_GOAL: usize = 0x240;
    pub const EYE_GOAL: usize = 0x250;
    pub const PITCH: usize = 0x260;
    pub const YAW: usize = 0x264;
    pub const ANGLES: usize = 0x270;
    pub const SUBJECT: usize = 0x2B4;
    /// The subject override's object this frame (`0x1401FCCF0`); Re:CoM's battle exec switches
    /// 3 ↔ 4 only without one.
    pub const SUBJECT_OBJ: usize = 0x2B8;
    /// The lock handle Re:CoM's battle exec copies each frame.
    pub const LOCK: usize = 0x2A4;
    pub const INV_H: usize = 0x2F0;
    pub const INV_V: usize = 0x2F4;
    pub const KBM: usize = 0x308;
    /// `+0x188` bits: input locked, a sleight's request holds the mode, a sweep's contact, the
    /// close-wall tilt.
    pub const LOCKED: u32 = 1;
    pub const EXTERNAL: u32 = 0x2000;
    pub const CONTACT: u32 = 0x400;
    pub const TILT: u32 = 0x80;
}

/// The player: his facing (`+0x4D4`, the follow's), action `+0x40C` (8 his glide), the ground
/// test's cache `+0x538` / `+0x53C` and floor height `+0x5F8`, his mover `+0xF0` (position `+0x30`,
/// velocity `+0x80` in metres a frame, contact flags `+0x120`). Checked (decomp `0x1401FD750`,
/// `0x1401F3C00`; `win::movement`).
const FACING: usize = 0x4D4;
/// Re:CoM's lock-on handle (`player+0x850`, the battle exec's 3 ↔ 4 switch). Checked (decomp).
const LOCK_HANDLE: usize = 0x850;
/// Re:CoM's lock target point for the camera (`0x1401FC400`: the look-target override's, else
/// `player+0x870`, the target's `CamTrgOfs` point `0x14017F770`). Checked (decomp).
const LOCK_POINT: u32 = 0x001F_C400;
const ACTION: usize = 0x40C;
const GROUND_CACHED: usize = 0x538;
const GROUND_AIR: usize = 0x53C;
const FLOOR_Y: usize = 0x5F8;
const BODY: usize = 0xF0;
const ACTION_GLIDE: i32 = 8;
/// The boss-camera count (`0x1401FEC50` sets it only when the boss's pack holds `BCam0000`; the
/// camera's dtor clears it). Checked (decomp).
const BOSS_CAMERA: u64 = 0x1_4087_B468;
/// Re:CoM's camera stack, slot 0 the camera drawn (`0x14028F270`, the getter returning it). Checked
/// (decomp); "slot 0 is drawn": Assumption (plan § 2.1).
const CAMERA_STACK: u64 = 0x1_40AC_4370;

const SET_PLACE: u32 = 0x001F_C4D0;
const SET_KEEP: u32 = 0x001F_BE60;
const FIELD_EXEC: u32 = 0x001F_EEB0;
const BATTLE_EXEC: u32 = 0x001F_ECE0;
const TAIL: u32 = 0x001F_C0D0;
const VIEW: u32 = 0x0024_FA40;
const SWEEP: u32 = 0x001F_BC60;
const ANGLES_FN: u32 = 0x001F_B890;
const STICK_X: u32 = 0x0029_B280;
const STICK_Y: u32 = 0x0029_B290;
const PRESSED: u32 = 0x0029_B2F0;
const KBM_FN: u32 = 0x0034_71C0;
const RECORD_OVERRIDE: u32 = 0x001F_EB50;
const HANDLE: u32 = 0x002A_D350;
const NODE: u32 = 0x002A_BB20;
/// Re:CoM's camera reset button (`0x1401FEEB0`: `0x14029C240(0x400, 0)`); R3: Assumption (plan § 2.4).
const RESET_BIT: u32 = 0x400;

type CamFn = unsafe extern "win64" fn(usize);
type SetPlaceFn = unsafe extern "win64" fn(usize, i32, i32, usize);
type SetKeepFn = unsafe extern "win64" fn(usize, i32);
type TailFn = unsafe extern "win64" fn(usize, *const f32, *const f32);
type ViewFn = unsafe extern "win64" fn(usize, *const f32, i32);
type SweepFn = unsafe extern "win64" fn(usize, *mut f32, *const f32, f32, f32) -> u64;
type AnglesFn = unsafe extern "win64" fn(usize, *mut f32, *const f32, *const f32);
type AxisFn = unsafe extern "win64" fn(u32) -> i32;
type PressedFn = unsafe extern "win64" fn(u32, u32) -> i32;
type IntFn = unsafe extern "win64" fn() -> i32;
type OverrideFn = unsafe extern "win64" fn(i32, usize);
type HandleFn = unsafe extern "win64" fn(u32) -> usize;
type NodeFn = unsafe extern "win64" fn(usize, i32) -> usize;
type PointFn = unsafe extern "win64" fn(usize) -> usize;

/// Our step's state for the live camera.
struct Live {
    cam: usize,
    follow: kh1::Follow,
    /// The block's mode-set count last seen (a new one: Re:CoM's setter ran for our mode).
    seen: u32,
    /// The camera was drawn (slot 0) last frame.
    drawn: bool,
    /// Placed behind Sora at its next step (a new camera).
    behind: bool,
    /// The mode our step ran last frame, and the lock handle then.
    mode: i32,
    handle: i32,
    /// KH2's camera (modes 8 / 9).
    kh2: kh2::Cam,
}

static LIVE: Mutex<Option<Live>> = Mutex::new(None);

fn note(msg: &str) {
    if LOGS.fetch_add(1, Ordering::Relaxed) < 64 {
        log(msg);
    }
}

/// The Camera setting (live: the dispatcher switches at the next frame).
pub(super) fn set_choice(c: crate::settings::Camera) {
    let v = match Family::from_setting(c) {
        Family::Recom => 0,
        Family::Kh1 => 1,
        Family::Kh2 => 2,
    };
    CHOICE.store(v, Ordering::Release);
}

/// The Auto camera row (live).
pub(super) fn set_auto(on: bool) {
    AUTO.store(on, Ordering::Release);
}

fn choice() -> Family {
    match CHOICE.load(Ordering::Acquire) {
        1 => Family::Kh1,
        2 => Family::Kh2,
        _ => Family::Recom,
    }
}

fn family_name(f: Family) -> &'static str {
    match f {
        Family::Recom => "Re:CoM's",
        Family::Kh1 => "KH1",
        Family::Kh2 => "KH2",
    }
}

/// Installs the block, the init wrapper, the record reads and the exec leas, all or nothing.
pub(super) fn install(base: usize, s: &crate::settings::Settings) {
    use crate::sites::{CAMERA_CALLEES, CAMERA_HOOKS, CAMERA_PATCHES};
    set_choice(s.camera);
    set_auto(s.auto_camera);
    // SAFETY: reads of the mapped image's code.
    let ok = |site: &crate::sites::Site| unsafe { std::slice::from_raw_parts((base + site.rva as usize) as *const u8, site.expect.len()) == site.expect };
    if let Some(bad) = CAMERA_PATCHES.iter().chain(CAMERA_CALLEES).find(|s| !ok(s)) {
        log(&format!("camera: {} differs; Re:CoM's camera only", bad.name));
        return;
    }
    let at = |s: &crate::sites::Site| base + s.rva as usize;
    let near = at(&CAMERA_PATCHES[2]);
    let len = camera::stub::layout::LEN;
    // SAFETY: a fresh block, built for its own address.
    let Some(block) = (unsafe { hook::place_built_near(near, len, |b| camera::stub::block(b as u64, base as u64, field_exec as *const () as u64, battle_exec as *const () as u64)) }) else {
        log("camera: no memory near the exe; Re:CoM's camera only");
        return;
    };
    BLOCK.store(block, Ordering::Release);
    use camera::stub::layout as l;
    let patches: [(&crate::sites::Site, Vec<u8>); 4] = [
        (&CAMERA_PATCHES[2], camera::stub::jump(at(&CAMERA_PATCHES[2]) as u64, (block + l::STUB_PLACE) as u64, CAMERA_PATCHES[2].expect.len())),
        (&CAMERA_PATCHES[3], camera::stub::jump(at(&CAMERA_PATCHES[3]) as u64, (block + l::STUB_KEEP) as u64, CAMERA_PATCHES[3].expect.len())),
        (&CAMERA_PATCHES[0], camera::stub::lea_rax(at(&CAMERA_PATCHES[0]) as u64, (block + l::JUMP_FIELD) as u64).to_vec()),
        (&CAMERA_PATCHES[1], camera::stub::lea_rax(at(&CAMERA_PATCHES[1]) as u64, (block + l::JUMP_BATTLE) as u64).to_vec()),
    ];
    // The record reads first (they change nothing for Re:CoM's modes), then the init wrapper,
    // then the execs.
    let mut done: Vec<usize> = Vec::new();
    let undo = |done: &[usize]| {
        for &i in done.iter().rev() {
            let (site, code) = &patches[i];
            // SAFETY: our own bytes back to Re:CoM's.
            unsafe { hook::patch(at(site), code, site.expect, "camera undo") };
        }
    };
    for (i, (site, code)) in patches.iter().enumerate().take(2) {
        // SAFETY: the bytes are checked inside.
        if unsafe { hook::patch(at(site), site.expect, code, site.name) } {
            done.push(i);
        } else {
            undo(&done);
            log("camera: a record read not patched; Re:CoM's camera only");
            return;
        }
    }
    let hooks = [hook::Hook { site: &CAMERA_HOOKS[0], ours: init as *const () as usize, original: &ORIG_INIT }];
    // SAFETY: the target's bytes are checked again inside.
    if !unsafe { hook::install_all(base, &hooks) } {
        undo(&done);
        log("camera: init not wrapped; Re:CoM's camera only");
        return;
    }
    for (i, (site, code)) in patches.iter().enumerate().skip(2) {
        // SAFETY: as above.
        if unsafe { hook::patch(at(site), site.expect, code, site.name) } {
            done.push(i);
        } else {
            undo(&done);
            log("camera: an exec lea not patched; the init wrapper stays but no camera of ours runs");
            CHOICE.store(0, Ordering::Release);
            return;
        }
    }
    log(&format!("camera: Re:CoM's player camera gains modes 6..9 (records at {block:#x}); camera = {} (Settings > Camera)", s.camera.as_str()));
}

fn block() -> usize {
    BLOCK.load(Ordering::Acquire)
}

/// Re:CoM's mode-set count for our modes (the stubs' `inc`).
unsafe fn entries() -> u32 {
    let b = block();
    if b == 0 {
        0
    } else {
        std::ptr::read_volatile((b + camera::stub::layout::COUNTER) as *const u32)
    }
}

/// After Re:CoM's init: our base mode when wanted; the next step places the camera behind Sora.
unsafe extern "win64" fn init(cam: usize) {
    let orig: CamFn = std::mem::transmute(ORIG_INIT.load(Ordering::Acquire));
    orig(cam);
    // Riku's story keeps Re:CoM's camera, unless `riku_kh1_movement` (fork testing, `crate::riku`;
    // owner 2026-11-22: "as opções de câmera podem ter para o Riku tbm"): then the Camera setting.
    let riku = super::stats::riku_story() && !super::movement::riku_on();
    RIKU.store(riku, Ordering::Release);
    let battle = rd::<usize>(cam + f::STATE) == cam + f::BATTLE_STATE;
    let want = camera::wanted(choice(), riku, battle && boss_camera());
    if let Ok(mut g) = LIVE.lock() {
        *g = Some(Live { cam, follow: kh1::Follow::default(), seen: entries(), drawn: false, behind: true, mode: 0, handle: 0, kh2: kh2::Cam::default() });
    }
    if want != Family::Recom {
        let m = want.base(battle);
        func::<SetPlaceFn>(SET_PLACE)(cam, m, 0, 0);
        wr::<i32>(cam + f::SAVED_MODE, m);
        note(&format!("camera: {} camera made in mode {m} ({} follow)", if battle { "battle" } else { "field" }, family_name(want)));
    }
}

unsafe fn boss_camera() -> bool {
    rd::<i32>(super::va(super::base(), BOSS_CAMERA)) > 0
}

unsafe extern "win64" fn field_exec(cam: usize) {
    exec(cam, false);
}

unsafe extern "win64" fn battle_exec(cam: usize) {
    exec(cam, true);
}

unsafe fn recom_exec(cam: usize, battle: bool) {
    func::<CamFn>(if battle { BATTLE_EXEC } else { FIELD_EXEC })(cam);
}

unsafe fn exec(cam: usize, battle: bool) {
    let m: i32 = rd(cam + f::MODE);
    if m < mode::FIRST && CHOICE.load(Ordering::Relaxed) == 0 {
        return recom_exec(cam, battle);
    }
    let want = camera::wanted(choice(), RIKU.load(Ordering::Relaxed), battle && boss_camera());
    let flags: u32 = rd(cam + f::FLAGS);
    let player: usize = rd(cam + f::PLAYER);
    let handle: i32 = if player != 0 { rd(player + LOCK_HANDLE) } else { 0 };
    let locked = (rd::<usize>(cam + f::SUBJECT_OBJ) == 0).then_some(handle > 0);
    match camera::dispatch(m, want, battle, flags & f::EXTERNAL != 0, rd::<i32>(cam + f::SUBJECT) > 0, locked) {
        Dispatch::Recom => recom_exec(cam, battle),
        Dispatch::Ours => step(cam, battle),
        Dispatch::Switch(to) => {
            func::<SetKeepFn>(SET_KEEP)(cam, to);
            if flags & f::EXTERNAL == 0 {
                wr::<i32>(cam + f::SAVED_MODE, to);
            }
            note(&format!("camera: mode {m} -> {to} ({})", family_name(Family::of(to))));
            if mode::ours(to) {
                step(cam, battle);
            } else {
                recom_exec(cam, battle);
            }
        }
    }
}

/// KH1's collision through Re:CoM's camera collision object (`0x1401FBC60`: a sphere from the aim
/// toward the eye, the eye moved to its contact), with Re:CoM's contact / tilt flags kept as they
/// were (they are its follow's).
struct RecomWorld {
    cam: usize,
}

/// Rays of the wall probe (option (a)).
const PROBE_RAYS: usize = 8;

/// The sweep's shape object (`cam+0x1A8`, Re:CoM's sphere collision): the contact normal at
/// `+0x40` (Re:CoM's solver `0x1401731E0` writes it with the contact; its stuck default (0, 1, 0)).
/// Assumption: it is the surface normal (KH1's polygon normal for the wall-avoid).
const SHAPE: usize = 0x1A8;
const SHAPE_NORMAL: usize = 0x40;

impl kh1::World for RecomWorld {
    fn normal(&mut self) -> Vec3 {
        // SAFETY: the camera's own shape object, read after its sweep.
        unsafe {
            let sh: usize = rd(self.cam + SHAPE);
            if sh == 0 {
                return Vec3::new(0.0, -1.0, 0.0);
            }
            let n = rd_vec(sh + SHAPE_NORMAL);
            Vec3::new(n.x, -n.y, -n.z)
        }
    }

    /// Stage 2: pending the owner's choice for KH1's flat 250 probe (`0x140196300`). Option (a):
    /// 8 flat rays of the radius from the point.
    fn near_wall(&mut self, at: Vec3, radius: f32) -> bool {
        (0..PROBE_RAYS).any(|k| {
            let yaw = k as f32 * std::f32::consts::TAU / PROBE_RAYS as f32;
            self.sweep(at, at + Vec3::from_yaw(yaw) * radius, kh1::RAY).is_some()
        })
    }

    fn sweep(&mut self, from: Vec3, to: Vec3, radius: f32) -> Option<Vec3> {
        let a = camera::to_recom4(from);
        let mut b = camera::to_recom4(to);
        // SAFETY: Re:CoM's camera, its own sweep on stack vectors.
        unsafe {
            let keep = rd::<u32>(self.cam + f::FLAGS) & (f::CONTACT | f::TILT);
            let hit = func::<SweepFn>(SWEEP)(self.cam, b.as_mut_ptr(), a.as_ptr(), radius * crate::units::METRES_PER_UNIT, 0.0) & 0xFF != 0;
            let now = rd::<u32>(self.cam + f::FLAGS);
            wr(self.cam + f::FLAGS, (now & !(f::CONTACT | f::TILT)) | keep);
            hit.then(|| crate::units::to_kh1(Vec3::new(b[0], b[1], b[2])))
        }
    }
}

/// Sora's KH1 movement state for the camera's thresholds: 0x15 in his glide (Re:CoM's action 8),
/// 2 in the air, else 0 (ground). Re:CoM's ground test `0x1401F3C00` read without running it
/// (its cache, else its rule: no floor contact and above the floor). Assumption: those map KH1's
/// states 0 / 2 / 0x15; KH1's others (hit 4, flight 8, ladders …) are not told apart.
unsafe fn kh1_state(player: usize) -> u32 {
    if rd::<i32>(player + ACTION) == ACTION_GLIDE {
        return 0x15;
    }
    let air = if rd::<i32>(player + GROUND_CACHED) != 0 {
        rd::<i32>(player + GROUND_AIR) != 0
    } else {
        let b: usize = rd(player + BODY);
        b != 0 && rd::<u8>(b + 0x120) & 5 == 0 && rd::<f32>(player + FLOOR_Y) < rd::<f32>(b + 0x34)
    };
    if air {
        2
    } else {
        0
    }
}

unsafe fn wr4(at: usize, v: [f32; 4]) {
    for (i, x) in v.iter().enumerate() {
        wr::<f32>(at + 4 * i, *x);
    }
}

/// One frame of our mode: what Re:CoM's battle exec does before its step (the sleight records'
/// restore), the KH1 or KH2 step, then Re:CoM's tail.
unsafe fn step(cam: usize, battle: bool) {
    let player: usize = rd(cam + f::PLAYER);
    if player == 0 {
        return;
    }
    if battle {
        let h: i32 = rd(cam + f::OVERRIDE_ACTOR);
        if h > 0 && func::<HandleFn>(HANDLE)(h as u32) == 0 {
            func::<OverrideFn>(RECORD_OVERRIDE)(0, 0);
        }
    }
    let mouse = func::<IntFn>(KBM_FN)() != 0;
    if !battle {
        wr::<i32>(cam + f::KBM, mouse as i32);
    }
    let node = func::<NodeFn>(NODE)(player, 0);
    if node == 0 {
        return;
    }
    let feet = rd_vec(node + 0xD0);
    let flags: u32 = rd(cam + f::FLAGS);
    let can_act = flags & f::LOCKED == 0;
    let inp = kh1::Input {
        pos: crate::units::to_kh1(feet),
        facing: crate::units::yaw_to_kh1(rd::<f32>(player + FACING)),
        stick: kh1::stick_from_recom(func::<AxisFn>(STICK_X)(0), func::<AxisFn>(STICK_Y)(0), mouse),
        mouse,
        can_act,
        recenter: can_act && func::<PressedFn>(PRESSED)(RESET_BIT, 0) != 0,
        inv: kh1::inversion(rd(cam + f::INV_H), rd(cam + f::INV_V)),
        step: crate::units::step_from_dt(rd(cam + f::DT)),
        auto: AUTO.load(Ordering::Relaxed),
        state: kh1_state(player),
        vel: {
            let b: usize = rd(player + BODY);
            if b == 0 {
                Vec3::ZERO
            } else {
                crate::units::to_kh1(rd_vec(b + 0x80))
            }
        },
    };
    let mut world = RecomWorld { cam };
    let count = entries();
    let drawn = rd::<usize>(super::va(super::base(), CAMERA_STACK)) == cam;
    let Ok(mut g) = LIVE.lock() else { return };
    if g.as_ref().map(|l| l.cam) != Some(cam) {
        *g = Some(Live { cam, follow: kh1::Follow::default(), seen: count, drawn, behind: true, mode: 0, handle: 0, kh2: kh2::Cam::default() });
    }
    let Some(l) = g.as_mut() else { return };
    let m: i32 = rd(cam + f::MODE);
    let handle: i32 = rd(player + LOCK_HANDLE);
    if battle && rd::<usize>(cam + f::SUBJECT_OBJ) == 0 {
        wr::<i32>(cam + f::LOCK, handle);
    }
    let drawn_eye = crate::units::to_kh1(rd_vec(cam + f::EYE));
    let drawn_aim = crate::units::to_kh1(rd_vec(cam + f::AIM));
    if Family::of(m) == Family::Kh2 {
        let (e, k) = kh2_step(l, cam, m, count, drawn, drawn_eye, drawn_aim, &inp, &mut world);
        drop(g);
        return finish(cam, e, k);
    }
    if l.behind {
        l.follow.behind(&inp, &mut world);
        l.behind = false;
        note("camera: KH1 follow placed behind Sora (new camera)");
    } else if drawn && !l.drawn {
        l.follow.behind(&inp, &mut world);
        note("camera: KH1 follow placed behind Sora (back from another camera)");
    } else if mode::ours(l.mode) && l.mode != m && count != l.seen {
        // Our own 6 ↔ 7 switch: KH1's lock start (60 blend) or unlock (45 blend, the follow's
        // entry at the drawn view's yaw).
        if m == mode::KH1_LOCK {
            l.follow.lock_on(false);
        } else {
            l.follow.unlock(&inp, &mut world);
        }
    } else if count != l.seen {
        // Re:CoM's setter gave us the mode (a sleight's restore, a switch from Re:CoM's modes):
        // the follow's entry; the lock camera carries on from the drawn view.
        if m == mode::KH1_LOCK {
            l.follow.eye = drawn_eye;
            l.follow.look = drawn_aim;
            l.follow.yaw = kh1::yaw_of(drawn_eye, drawn_aim);
            l.follow.blend.left = 0.0;
        } else {
            l.follow.enter(kh1::yaw_of(drawn_eye, drawn_aim), &inp, &mut world);
        }
    } else if m == mode::KH1_LOCK && l.mode == m && handle > 0 && l.handle > 0 && handle != l.handle {
        // A new target while locked: KH1's re-lock blend of the look point.
        l.follow.lock_on(true);
    }
    l.seen = count;
    l.drawn = drawn;
    l.mode = m;
    l.handle = handle;
    if m == mode::KH1_LOCK {
        let p = func::<PointFn>(LOCK_POINT)(cam);
        let point = crate::units::to_kh1(rd_vec(p));
        let target = func::<HandleFn>(HANDLE)(handle as u32);
        let radius = if target != 0 { super::stats::lock_radius(target).unwrap_or(0.0) } else { 0.0 };
        l.follow.lock_step(&inp, &kh1::Target { point, radius }, &mut world);
    } else {
        l.follow.step(&inp, &mut world);
    }
    let (e, k) = l.follow.view(inp.step);
    drop(g);
    finish(cam, e, k);
}

/// KH2's frame (modes 8 / 9): its entries, then its step. `eye` / `aim`: the view Re:CoM drew last.
#[allow(clippy::too_many_arguments)]
unsafe fn kh2_step(l: &mut Live, cam: usize, m: i32, count: u32, drawn: bool, eye: Vec3, aim: Vec3, k1: &kh1::Input, world: &mut RecomWorld) -> (Vec3, Vec3) {
    let mouse = k1.mouse;
    let dev: usize = rd(super::va(super::base(), crate::sites::data::PAD_DEVICES));
    let (lx, ly) = if dev != 0 { (rd::<i32>(dev + 0x74), rd::<i32>(dev + 0x78)) } else { (0, 0) };
    let (swap_h, swap_v) = kh2::swaps(rd(cam + f::INV_H), rd(cam + f::INV_V));
    let inp = kh2::Input {
        pos: k1.pos,
        facing: k1.facing,
        right: kh2::Stick::from_recom(func::<AxisFn>(STICK_X)(0), func::<AxisFn>(STICK_Y)(0), mouse),
        // KH2's left stick is the pad's own (`pad+0x30..0x3C`), never the mouse.
        left: kh2::Stick::from_recom(lx, ly, false),
        mouse,
        can_act: k1.can_act,
        reset: k1.recenter,
        swap_h,
        swap_v,
        step: k1.step,
        auto: k1.auto,
    };
    if l.behind {
        // A new camera: KH2's room start (the loader's 420, then its re-place).
        l.kh2.behind(&inp, true);
        l.behind = false;
        note("camera: KH2 camera placed behind Sora (new camera)");
    } else if drawn && !l.drawn {
        // Back from another camera (events, `.cam` paths): KH2's re-place `0x140164FA0` (owner Q4 (b)).
        l.kh2.behind(&inp, false);
        note("camera: KH2 camera placed behind Sora (back from another camera)");
    } else if count != l.seen && !(Family::of(l.mode) == Family::Kh2 && l.mode != m) {
        // Re:CoM's setter gave us the mode (a sleight's restore, a switch from Re:CoM's camera):
        // KH2 carries on from the view drawn.
        l.kh2.carry_on(eye, aim);
    }
    l.seen = count;
    l.drawn = drawn;
    l.mode = m;
    // Mode 9: KH2's lock camera on Re:CoM's own lock-on point (`0x1401FC400`: `player+0x870`, the
    // target's `CamTrgOfs` point) where KH2 takes its target's camera shape (`0x1403BDBE0`).
    // A new target needs nothing more: KH2's springs carry the view (`0x140176910`).
    let target = (m == mode::KH2_LOCK).then(|| kh2::Target { point: crate::units::to_kh1(rd_vec(func::<PointFn>(LOCK_POINT)(cam))) });
    l.kh2.step(&inp, target.as_ref(), world)
}

/// The step's eye and look (KH1 units) into Re:CoM's camera, then Re:CoM's tail.
unsafe fn finish(cam: usize, e: Vec3, k: Vec3) {
    let (look, eye) = (camera::to_recom4(k), camera::to_recom4(e));
    wr4(cam + f::AIM, look);
    wr4(cam + f::AIM_GOAL, look);
    wr4(cam + f::EYE, eye);
    wr4(cam + f::EYE_GOAL, eye);
    // Re:CoM's own angles of the result (its follow eases its yaw toward them), so a hand-over to
    // Re:CoM's modes starts where we are; its pitch at rest.
    func::<AnglesFn>(ANGLES_FN)(cam, (cam + f::ANGLES) as *mut f32, (cam + f::AIM) as *const f32, (cam + f::EYE) as *const f32);
    wr::<f32>(cam + f::YAW, rd(cam + f::ANGLES + 4));
    wr::<f32>(cam + f::PITCH, 0.0);
    // Re:CoM's tail, as its execs end (`0x1401FEEB0` / `0x1401FECE0`).
    wr::<f32>(cam + f::EYE + 0xC, 1.0);
    wr::<f32>(cam + f::AIM + 0xC, 1.0);
    func::<TailFn>(TAIL)(cam, (cam + f::EYE) as *const f32, (cam + f::AIM) as *const f32);
    let n = func::<NodeFn>(NODE)(cam, 0);
    if n != 0 {
        for i in 0..4 {
            wr::<f32>(n + 0x30 + 4 * i, rd(cam + f::EYE + 4 * i));
        }
    }
    func::<ViewFn>(VIEW)(cam, (cam + f::AIM) as *const f32, 0);
}
