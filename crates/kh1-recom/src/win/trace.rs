//! F9 in Re:CoM: records `kh1-recom-trace-<n>.csv` next to the game (`crate::trace`).
//!
//! Sampling points: the player's Field and Battle state exes (`sites::TRACE_HOOKS`). Before the
//! exe: the state after the last frame (Re:CoM's motion and mover have run). After it: this
//! frame's inputs, as the exe's pad update and request bits left them. When not recording, the
//! hooks only test one flag and call Re:CoM's exe.

use super::movement::{self as mv, RecomHost, SORA};
use super::sys::*;
use super::log;
use crate::trace::{self, schema, schema::Row};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

pub static ORIG_FIELD: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_BATTLE: AtomicUsize = AtomicUsize::new(0);
static RECORDING: AtomicBool = AtomicBool::new(false);
static REC: Mutex<Option<Recorder>> = Mutex::new(None);
static DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

const VK_F9: i32 = 0x78;

struct Recorder {
    out: BufWriter<File>,
    path: PathBuf,
    rows: u64,
    pending: Option<Row>,
    player: usize,
}

type PlayerFn = unsafe extern "win64" fn(usize);

impl Recorder {
    /// Before the exe: finish the last row with the state after its frame.
    unsafe fn before(&mut self, pl: usize) {
        let Some(mut row) = self.pending.take() else { return };
        if self.player != pl {
            return;
        }
        let mut host = RecomHost::new(pl);
        let v = host.view();
        let g = SORA.try_lock().ok();
        let kh1 = g.as_ref().and_then(|g| g.as_ref()).filter(|s| mv::ran_last_frame(pl, s));
        match kh1 {
            Some(s) => {
                let after = s.sora.after_frame(&mut host, v.airborne);
                trace::state_into(&mut row, &v, Some((&s.sora, after, &s.last)));
            }
            None => trace::state_into(&mut row, &v, None),
        }
        if let Ok(mut h) = super::attack::HIT_LOG.lock() {
            row.hits = h.join(" ");
            h.clear();
        }
        row.frame = self.rows;
        if row.write_csv(&mut self.out).is_ok() {
            self.rows += 1;
        }
        if self.rows.is_multiple_of(60) {
            let _ = self.out.flush();
        }
    }

    /// After the exe: this frame's inputs.
    unsafe fn after(&mut self, pl: usize) {
        self.player = pl;
        let step: f32 = std::ptr::read_unaligned((pl + 0x464) as *const f32);
        self.pending = Some(trace::input_row(step, &mv::read_pad(pl), mv::ability_bits()));
    }
}

unsafe fn state_exe(pl: usize, o: &AtomicUsize) {
    let f: PlayerFn = std::mem::transmute_copy(&o.load(Ordering::Acquire));
    let sora = mv::is_main_sora(pl);
    if sora {
        // Sora's frame count, for the root-motion delay.
        super::attack::tick();
    }
    if !RECORDING.load(Ordering::Relaxed) || !sora {
        return f(pl);
    }
    if let Ok(mut g) = REC.try_lock() {
        if let Some(r) = g.as_mut() {
            r.before(pl);
        }
    }
    f(pl);
    if let Ok(mut g) = REC.try_lock() {
        if let Some(r) = g.as_mut() {
            r.after(pl);
        }
    }
}

pub unsafe extern "win64" fn field_exe(pl: usize) {
    state_exe(pl, &ORIG_FIELD)
}

pub unsafe extern "win64" fn battle_exe(pl: usize) {
    state_exe(pl, &ORIG_BATTLE)
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

fn toggle() {
    let Ok(mut g) = REC.lock() else { return };
    match g.take() {
        Some(mut r) => {
            RECORDING.store(false, Ordering::Relaxed);
            let _ = r.out.flush();
            log(&format!("trace: stopped, {} frames in {}", r.rows, r.path.display()));
        }
        None => {
            let dir = DIR.lock().ok().and_then(|d| d.clone()).unwrap_or_default();
            let Some(path) = (1..10000).map(|n| dir.join(trace::file_name(n))).find(|p| !p.exists()) else { return };
            match File::create(&path) {
                Ok(f) => {
                    let mut out = BufWriter::with_capacity(1 << 16, f);
                    if schema::write_header(&mut out).is_ok() {
                        log(&format!("trace: recording to {}", path.display()));
                        *g = Some(Recorder { out, path, rows: 0, pending: None, player: 0 });
                        RECORDING.store(true, Ordering::Relaxed);
                    }
                }
                Err(e) => log(&format!("trace: cannot write {}: {e}", path.display())),
            }
        }
    }
}

/// Starts the F9 watcher (its own thread; the game thread only sees the recording flag).
pub fn start(dir: PathBuf) {
    if let Ok(mut d) = DIR.lock() {
        *d = Some(dir);
    }
    std::thread::spawn(|| {
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
    });
}
