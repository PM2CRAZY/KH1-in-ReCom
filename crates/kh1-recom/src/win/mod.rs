//! Windows side: start-up (from Panacea's `OnInit`, or a fallback thread), hooks, and the glue
//! between our Config row and `kh1-recom.ini`.

mod hook;
mod menu;
mod movement;
mod sys;
mod trace;

use crate::settings::{self, Settings};
use std::ffi::c_void;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, Once};
use sys::*;

static BASE: AtomicUsize = AtomicUsize::new(0);
static MODULE: AtomicUsize = AtomicUsize::new(0);
static START: Once = Once::new();
static LOG_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
static INI_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
static SETTINGS: Mutex<Option<Settings>> = Mutex::new(None);

/// The exe's image base.
pub(crate) fn base() -> usize {
    BASE.load(Ordering::Relaxed)
}

/// A Steam virtual address (image base 0x140000000) in the running image.
pub(crate) fn va(base: usize, addr: u64) -> usize {
    base + (addr - 0x1_4000_0000) as usize
}

/// Appends a line to `kh1-recom.log`.
pub(crate) fn log(msg: &str) {
    let Ok(g) = LOG_PATH.lock() else { return };
    if let Some(p) = g.as_ref() {
        if let Ok(mut f) = std::fs::OpenOptions::new().append(true).create(true).open(p) {
            let _ = writeln!(f, "{msg}");
        }
    }
}

pub(crate) fn settings() -> Settings {
    SETTINGS.lock().ok().and_then(|g| g.clone()).unwrap_or_default()
}

/// Our Config row `k` was flipped: change the setting, rewrite the ini.
pub(crate) fn toggle(_k: usize) {
    let mut s = settings();
    s.combat = match s.combat {
        settings::Mode::Kh1 => settings::Mode::Recom,
        settings::Mode::Recom => settings::Mode::Kh1,
    };
    if let Ok(mut g) = SETTINGS.lock() {
        *g = Some(s.clone());
    }
    if let Some(p) = INI_PATH.lock().ok().and_then(|g| g.clone()) {
        let text = std::fs::read_to_string(&p).unwrap_or_else(|_| settings::DEFAULT_FILE.to_string());
        if let Err(e) = std::fs::write(&p, settings::store(&text, &s)) {
            log(&format!("config menu: could not write {}: {e}", p.display()));
        }
    }
    movement::set_mode(s.combat == settings::Mode::Kh1);
    log(&format!("config menu: combat = {}", s.combat.as_str()));
}

/// The KH1-only clips the asset step added (High Jump, its top, Guard): the manifest
/// `kh1-motions.json` and the `PL0001.ANN` header Panacea will serve, looked for under the mod
/// folder Panacea passed, its `recom` folder, the DLL's folders and the game folder.
fn kh1_clips(dirs: &[PathBuf]) -> (crate::rows::Extension, Vec<(u32, String)>) {
    use crate::{motions, rows};
    let mut bases: Vec<PathBuf> = Vec::new();
    for d in dirs {
        bases.push(d.clone());
        if let Ok(rd) = std::fs::read_dir(d) {
            for e in rd.flatten() {
                if e.file_name().to_string_lossy().eq_ignore_ascii_case("recom") && e.path().is_dir() {
                    bases.push(e.path());
                }
            }
        }
    }
    let Some(manifest) = bases.iter().map(|b| b.join("kh1-motions.json")).find(|p| p.is_file()) else {
        log("movement: no kh1-motions.json found (asset step not run?): High Jump uses Re:CoM's jump clip, Guard has no clip");
        return (rows::Extension::default(), Vec::new());
    };
    let m = match std::fs::read_to_string(&manifest).map_err(|e| e.to_string()).and_then(|t| motions::parse_manifest(&t)) {
        Ok(m) => m,
        Err(e) => {
            log(&format!("movement: {}: {e}", manifest.display()));
            return (rows::Extension::default(), Vec::new());
        }
    };
    let ann = bases.iter().map(|b| b.join("PLAYER").join("0001").join("PL0001.ANN")).find(|p| p.is_file()).and_then(|p| {
        use std::io::Read;
        let mut f = std::fs::File::open(&p).ok()?;
        let mut head = [0u8; 4];
        f.read_exact(&mut head).ok()?;
        let n = u32::from_le_bytes(head).min(4096);
        let mut rest = vec![0u8; motions::ann_header_len(n) - 4];
        f.read_exact(&mut rest).ok()?;
        let mut all = head.to_vec();
        all.extend_from_slice(&rest);
        motions::ann_header(&all).ok()
    });
    let Some(ann) = ann else {
        log("movement: no extended PL0001.ANN found next to the manifest: KH1-only clips not used");
        return (rows::Extension::default(), Vec::new());
    };
    let ext = rows::Extension::plan(|e| motions::clip_for_entry(&m, &ann, e).is_ok());
    let names = rows::appended_rows(&ext, |e| motions::clip_for_entry(&m, &ann, e).ok().map(|c| c.name.clone()));
    log(&format!("movement: manifest {}; KH1-only clips {:?}", manifest.display(), names));
    (ext, names)
}

/// The KH1 movement handlers (`movement`) and the trace's sampling points (`trace`).
fn install_movement(base: usize, s: &Settings, dirs: &[PathBuf], game_dir: &Path) {
    let (ext, names) = kh1_clips(dirs);
    // SAFETY: data patch with checked old values, before the first room load.
    let ext = if unsafe { movement::extend_action_table(&ext, &names) } { ext } else { crate::rows::Extension::default() };
    let _ = movement::EXT.set(ext);
    let site = |name: &str| crate::sites::MOVE_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [
        hook::Hook { site: site("move exe (actions 2/3/4)"), ours: movement::move_exe as *const () as usize, original: &movement::ORIG_MOVE_EXE },
        hook::Hook { site: site("stick class (0x1401E9630)"), ours: movement::stick_class as *const () as usize, original: &movement::ORIG_STICK_CLASS },
        hook::Hook { site: site("move step (0x1401EF3C0)"), ours: movement::move_step as *const () as usize, original: &movement::ORIG_MOVE_STEP },
        hook::Hook { site: site("jump init (0x1401F29F0)"), ours: movement::jump_init as *const () as usize, original: &movement::ORIG_JUMP_INIT },
        hook::Hook { site: site("avoid init (0x1401F10C0)"), ours: movement::avoid_init as *const () as usize, original: &movement::ORIG_AVOID_INIT },
        hook::Hook { site: site("jump exe (action 7)"), ours: movement::jump_exe as *const () as usize, original: &movement::ORIG_JUMP_EXE },
        hook::Hook { site: site("CPlayerSora avoid exe (action 6)"), ours: movement::avoid_exe as *const () as usize, original: &movement::ORIG_AVOID_EXE },
        hook::Hook { site: site("CPlayerSora glide exe (action 8)"), ours: movement::glide_exe as *const () as usize, original: &movement::ORIG_GLIDE_EXE },
    ];
    // SAFETY: the targets' bytes are checked again inside, before anything is written.
    if unsafe { hook::install_all(base, &hooks) } {
        movement::mark_installed();
        movement::set_abilities(s.abilities);
        movement::set_mode(s.combat == settings::Mode::Kh1);
        log(&format!(
            "movement: KH1 handlers in Re:CoM's actions 2/3/4, 6, 7, 8 ({}); abilities {:?}",
            if s.combat == settings::Mode::Kh1 { "on: combat = kh1" } else { "pass-through: combat = recom" },
            s.abilities
        ));
    } else {
        log("movement: not hooked; Sora moves as in Re:CoM");
    }
    let th = &crate::sites::TRACE_HOOKS;
    let hooks = [
        hook::Hook { site: &th[0], ours: trace::field_exe as *const () as usize, original: &trace::ORIG_FIELD },
        hook::Hook { site: &th[1], ours: trace::battle_exe as *const () as usize, original: &trace::ORIG_BATTLE },
    ];
    // SAFETY: as above.
    if unsafe { hook::install_all(base, &hooks) } {
        trace::start(game_dir.to_path_buf());
        log("trace: press F9 in game to start / stop recording kh1-recom-trace-<n>.csv next to the game");
    } else {
        log("trace: not hooked");
    }
}

fn module_path(m: *mut c_void) -> PathBuf {
    let mut buf = vec![0u16; 1024];
    // SAFETY: the buffer length is passed; a null module names the exe.
    let n = unsafe { GetModuleFileNameW(m, buf.as_mut_ptr(), buf.len() as u32) } as usize;
    PathBuf::from(String::from_utf16_lossy(&buf[..n.min(buf.len())]))
}

fn wide_to_path(p: *const u16) -> Option<PathBuf> {
    if p.is_null() {
        return None;
    }
    // SAFETY: Panacea passes a NUL-terminated wide string (its mod folder).
    let s = unsafe {
        let mut n = 0;
        while *p.add(n) != 0 && n < 4096 {
            n += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(p, n))
    };
    (!s.is_empty()).then(|| PathBuf::from(s))
}

fn run(mod_dir: Option<PathBuf>) {
    // SAFETY: a null name returns the exe's own (mapped) image base.
    let base_ptr = unsafe { GetModuleHandleW(std::ptr::null()) } as *const u8;
    if base_ptr.is_null() {
        return;
    }
    // SAFETY: the first 0x1000 bytes of a mapped PE image are its headers.
    let headers = unsafe { std::slice::from_raw_parts(base_ptr, 0x1000) };
    let pe = crate::sites::pe_info(headers);
    let size = pe.as_ref().map(|p| p.size_of_image).unwrap_or(0);
    let read_site = |rva: u32, n: usize| -> Vec<u8> {
        if (rva as u64 + n as u64) > size as u64 {
            return Vec::new();
        }
        // SAFETY: inside the mapped image (checked against its size); code is readable.
        unsafe { std::slice::from_raw_parts(base_ptr.add(rva as usize), n).to_vec() }
    };
    let exe = module_path(std::ptr::null_mut());
    let game_dir = exe.parent().map(Path::to_path_buf).unwrap_or_default();
    let dll = module_path(MODULE.load(Ordering::Relaxed) as *mut c_void);
    // The DLL sits in mod_path\Recom\dll: its parent's parent is the mod folder.
    let mod_dir = mod_dir.or_else(|| dll.parent().and_then(Path::parent).map(Path::to_path_buf)).unwrap_or_else(|| game_dir.clone());
    let ini_path = game_dir.join("kh1-recom.ini");
    let ini = match std::fs::read_to_string(&ini_path) {
        Ok(t) => Ok(t),
        Err(_) => {
            let _ = std::fs::write(&ini_path, settings::DEFAULT_FILE);
            Err(format!("{} was missing and has been written with the defaults", ini_path.display()))
        }
    };
    let s = match &ini {
        Ok(t) => settings::parse(t).0,
        Err(_) => Settings::default(),
    };
    let (report, ok) = crate::startup_report(&exe.to_string_lossy(), pe, &read_site, ini);
    let log_path = game_dir.join("kh1-recom.log");
    let _ = std::fs::write(&log_path, &report);
    if let Ok(mut g) = LOG_PATH.lock() {
        *g = Some(log_path);
    }
    if let Ok(mut g) = INI_PATH.lock() {
        *g = Some(ini_path);
    }
    if let Ok(mut g) = SETTINGS.lock() {
        *g = Some(s.clone());
    }
    log(&format!("dll: {}", dll.display()));
    log(&format!("mod folder: {}", mod_dir.display()));
    if !ok {
        return;
    }
    let base = base_ptr as usize;
    BASE.store(base, Ordering::Relaxed);

    // Config menu rows.
    if s.config_menu {
        let hooks = [
            hook::Hook { site: &crate::sites::HOOKS[0], ours: menu::cfg_input as *const () as usize, original: &menu::ORIG_INPUT },
            hook::Hook { site: &crate::sites::HOOKS[1], ours: menu::cfg_draw as *const () as usize, original: &menu::ORIG_DRAW },
        ];
        // SAFETY: the targets' bytes are checked again inside, before anything is written.
        if unsafe { hook::install_all(base, &hooks) } {
            log("config menu: row added (camp menu > Config, below Re:CoM's rows)");
        } else {
            log("config menu: not hooked; use kh1-recom.ini");
        }
    } else {
        log("config menu: off in the ini; Re:CoM's menu untouched");
    }

    let dll_dir = dll.parent().map(Path::to_path_buf).unwrap_or_default();
    let dirs = [mod_dir.clone(), dll_dir.parent().map(Path::to_path_buf).unwrap_or_default(), dll_dir, game_dir.clone()];
    install_movement(base, &s, &dirs, &game_dir);
}

fn start(mod_dir: Option<PathBuf>) {
    START.call_once(|| run(mod_dir));
}

/// Panacea calls this once after loading every DLL in `mod_path\Recom\dll`, with the mod folder.
///
/// # Safety
/// `folder` is null or a NUL-terminated wide string.
#[no_mangle]
pub unsafe extern "C" fn OnInit(folder: *const u16) {
    start(wide_to_path(folder));
}

/// # Safety
/// Called by the Windows loader.
#[no_mangle]
pub unsafe extern "system" fn DllMain(module: *mut c_void, reason: u32, _reserved: *mut c_void) -> i32 {
    const DLL_PROCESS_ATTACH: u32 = 1;
    if reason == DLL_PROCESS_ATTACH {
        MODULE.store(module as usize, Ordering::Relaxed);
        // If nothing calls OnInit (loaded some other way), start anyway after a moment.
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_secs(3));
            start(None);
        });
    }
    1
}
