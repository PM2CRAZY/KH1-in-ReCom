//! Lock-on at Combat = KH1 on a controller: R1 (0.133.0) and KH1's lock rules (0.137.0)
//! (`crate::lock_on`, `docs/recom-lock-on.md`).
//!
//! - A wrapper on Re:CoM's lock-on `0x1401E8DA0` (`sites::LOCK_ON_HOOK`): before Re:CoM's own
//!   runs, its button field (the player's pad object `+0x44`) gets [`crate::lock_on::button`]: R1
//!   for KH1 Sora with the pad in use, else Re:CoM's R2.
//! - With the pad in use, the wrapper marks the call as KH1's ([`KH1_RULES`]) and the lock-on's
//!   five input calls (`sites::LOCK_ON_CALLS`, retargeted at install through one stub block) get
//!   KH1's answers: R1 with no target: not pressed; locked, R1 pressed: "released after a second
//!   press" (Re:CoM's unlock, its sound 8); locked, L2 pressed: "held and flicked", and the target
//!   switch answers KH1's next in its lock list (Re:CoM's switch, cursor and sound 7); otherwise
//!   not held (no right-stick switch). Every other call: Re:CoM's own callee, unchanged.
//! - After Re:CoM's lock-on, KH1's lock list follows its cursor ([`crate::lock_on::History`]).
//!
//! Per frame: Re:CoM's lock-on once: an atomic load and two compares (KH1 Sora); at KH1 with the
//! pad one flag read, one store, two atomic stores; unlocked one more load; locked the list's
//! per-frame drop (a handle lookup and a point read per listed target). Per lock-on call each
//! retargeted call adds one atomic load.

use super::movement::{self as mv, func, orig, rd, wr};
use super::{base, hook, log};
use crate::lock_on::{self as lo, recom, Candidate, History};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

pub static ORIG_LOCK_ON: AtomicUsize = AtomicUsize::new(0);
/// The last device logged (0 none yet, 1 pad, 2 keyboard / mouse).
static LOGGED: AtomicUsize = AtomicUsize::new(0);
/// The player whose lock-on runs with KH1's rules right now (0: none; Re:CoM's answers).
static KH1_RULES: AtomicUsize = AtomicUsize::new(0);
/// The five calls are retargeted.
static RULES_ON: AtomicBool = AtomicBool::new(false);
/// KH1's lock list, and whether it holds anything (read without the lock each frame).
static LIST: Mutex<History> = Mutex::new(History::EMPTY);
static LISTED: AtomicBool = AtomicBool::new(false);
/// Log lines left for the L2 steps.
static STEP_LOG: AtomicUsize = AtomicUsize::new(24);
/// 0.172.0: the target our L2 step chose this lock-on call (0: none).
static STEPPED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
/// The player's locked handle (`+0x850`, the switch's argument).
const LOCK_HANDLE: usize = 0x850;

type PlFn = unsafe extern "win64" fn(usize) -> u64;
type PadFn = unsafe extern "win64" fn(usize) -> u32;
type SwitchFn = unsafe extern "win64" fn(u32) -> i32;
type PadTestFn = unsafe extern "win64" fn(u32, u32) -> u32;
type PointFn = unsafe extern "win64" fn(usize, *mut V4);
type PlayerPointFn = unsafe extern "win64" fn(*mut V4, i32) -> *mut V4;

/// Re:CoM's vectors (written with aligned stores).
#[repr(C, align(16))]
#[derive(Default)]
struct V4([f32; 4]);

pub fn mark_installed() {
    log("lock-on: at combat = kh1 in Sora's story the controller's R1 is Re:CoM's lock-on (its lock-on 0x1401E8DA0 reads the pad object's +0x44 = 8, R1, instead of Re:CoM's 2, R2); keyboard / mouse keep Re:CoM's binding");
}

/// Re:CoM's lock-on, wrapped.
pub unsafe extern "win64" fn lock_on(pl: usize) -> u64 {
    let o: PlFn = orig(&ORIG_LOCK_ON);
    if pl == 0 || !mv::kh1_fighter(pl) {
        return o(pl);
    }
    // 0.181.0: KH1's auto lock's time (state 4).
    hold_tick(pl);
    let kbm = rd::<u32>(base() + recom::KBM_IN_USE as usize) != 0;
    let b = lo::button(true, kbm);
    wr::<u32>(pl + recom::PAD + recom::LOCK_BUTTON, b);
    let dev = if kbm { 2 } else { 1 };
    if LOGGED.swap(dev, Ordering::Relaxed) != dev {
        let rules = if RULES_ON.load(Ordering::Acquire) && !kbm { "KH1's rules (R1 again unlocks, L2 steps)" } else { "Re:CoM's rules" };
        log(&format!("lock-on: {} in use: the lock-on button is {}, {rules}", if kbm { "keyboard / mouse" } else { "the controller" }, if b == lo::bits::R1 { "R1" } else { "Re:CoM's (R2 / its key)" }));
    }
    if !lo::kh1_rules(true, kbm) || !RULES_ON.load(Ordering::Acquire) {
        if LISTED.load(Ordering::Relaxed) {
            with_list(|l| l.clear());
        }
        return o(pl);
    }
    KH1_RULES.store(pl, Ordering::Release);
    STEPPED.store(0, Ordering::Relaxed);
    // 0.173.0 (fork owner, 2026-11-23, KH1 FM's Config screenshot: "precisa implementar essas duas
    // configs e features"): KH1's Auto Lock (`*(0x142E1F4D8)+0` = 0, Checked (KH1 exe: disasm)):
    // an attack with no lock locks its target (`0x14029365F` → `0x1402B08A0(…, 4)`), and a lock
    // whose target is gone (not the player's unlock) moves to the next one (`0x1402B1348` →
    // `0x1402B0A00`). Here: Re:CoM's own lock of its pick (`+0x84C`) without R1, on those frames.
    // 0.175.0: KH1's Auto Lock is its soft target (no lock: `soft_target`), not a lock of Re:CoM's
    // cursor (0.173.0's arming removed).
    let r = o(pl);
    KH1_RULES.store(0, Ordering::Release);
    stepped(pl);
    after(pl);
    r
}

fn with_list<T>(f: impl FnOnce(&mut History) -> T) -> T {
    let mut g = LIST.lock().unwrap_or_else(|e| e.into_inner());
    let r = f(&mut g);
    LISTED.store(!g.is_empty(), Ordering::Relaxed);
    r
}

/// The cursor's locked target after Re:CoM's lock-on (None: not locked, or locked on nothing).
unsafe fn locked_target(pl: usize) -> Option<u32> {
    let cur: usize = rd(pl + recom::CURSOR);
    if cur == 0 || rd::<i32>(cur + recom::CURSOR_LOCKED) == 0 {
        return None;
    }
    let t: i32 = rd(cur + recom::CURSOR_TARGET);
    (t > 0).then_some(t as u32)
}

unsafe fn battle_room(pl: usize) -> bool {
    let cur: usize = rd(pl + recom::CURSOR);
    cur != 0 && rd::<u32>(cur + recom::CURSOR_FLAGS) & recom::CURSOR_BATTLE != 0
}

/// KH1's list follows Re:CoM's cursor, then drops what is gone or out of Re:CoM's take reach
/// (KH1's per-frame drop, `0x1402B11B0`).
unsafe fn after(pl: usize) {
    let locked = locked_target(pl);
    if locked.is_none() && !LISTED.load(Ordering::Relaxed) {
        return;
    }
    let reach = lo::take_reach(battle_room(pl));
    let origin = player_point();
    with_list(|l| {
        l.sync(locked);
        if l.entries().len() > 1 {
            l.prune(|h| target_point(pl, h).is_some_and(|p| lo::distance(origin, p) <= reach));
        }
    });
}

/// 0.172.0 (fork owner, 2026-11-23: "L2/trigger esquerdo não está alternando oponentes no modo
/// Kh1"; the owner's 0.171.0 log: every L2 stepped from the same target, the cursor never took the
/// next): after Re:CoM's lock-on, a target our L2 step chose that its cursor (`+0x3F8 → +0xD4`) or
/// the player's handle (`+0x850`) didn't take is written to both, the lock kept (`+0xE0`). Logged
/// (what Re:CoM left), so the next trace says why it refused. Assumption: Re:CoM's cursor follows a
/// written `+0xD4` (its locked branch `0x140209038` re-reads it each frame).
unsafe fn stepped(pl: usize) {
    let next = STEPPED.swap(0, Ordering::Relaxed);
    if next == 0 {
        return;
    }
    let cur: usize = rd(pl + recom::CURSOR);
    if cur == 0 {
        return;
    }
    let (d4, handle, locked): (i32, i32, i32) = (rd(cur + recom::CURSOR_TARGET), rd(pl + LOCK_HANDLE), rd(cur + recom::CURSOR_LOCKED));
    let took = d4 == next as i32 && handle == next as i32;
    if STEP_LOG.load(Ordering::Relaxed) > 0 {
        log(&format!("lock-on: L2 step to {next:#x}: Re:CoM left cursor +0xD4 {d4:#x}, +0x850 {handle:#x}, locked {locked}{}", if took { "" } else { "; written ours" }));
    }
    if !took {
        wr::<i32>(cur + recom::CURSOR_TARGET, next as i32);
        wr::<i32>(cur + recom::CURSOR_LOCKED, 1);
        wr::<i32>(pl + LOCK_HANDLE, next as i32);
    }
}

/// 0.181.0: an attack's auto lock (KH1's lock state 4, `lo::soft::Held`), None: none.
static HELD: Mutex<Option<lo::soft::Held>> = Mutex::new(None);

/// 0.175.0 (fork owner, 2026-11-23: "o teu Auto Lock está diferente da implementação do KH1 lá o
/// auto lock funciona sem lock on ao chegar proximo do oponente os ataques são direcionados a ele";
/// a tester's report: "Sora doesn't seem to automatically lock on to an enemy when doing a combo"):
/// KH1's soft target (Auto Lock On, `*(0x142E1F4D8)+0` = 0). 0.181.0 (fork owner: "o amarelo ele
/// aparece no oponente usando a regra do jogo, e se vc tiver com o auto lock ativo, os teus ataques
/// sempre serão na direção daquele oponente independente de onde tu esteja olhando"), as KH1's
/// `0x1402B11B0` (Checked (decomp), the workspace's `knowledge/KH1/kh1-lock-on.md`):
/// - an attack's auto lock (state 4, [`hold`]) keeps its enemy while it is within 15 m / 10 m;
/// - else, only while the highlighted command has KH1's flag 4 (`win::command_menu`), KH1's pick
///   (`lo::soft::pick`: 3 m flat less the radius, ±3.5 m, the facing and height score).
///
/// Off: none. Any story, Combat = KH1, Sora or Riku. No lock of Re:CoM's is checked here (the
/// callers do).
pub(super) unsafe fn soft_target(pl: usize) -> Option<u32> {
    if !super::settings().auto_lock || !mv::kh1_fighter(pl) {
        return None;
    }
    let all = targets(pl);
    let origin = player_point();
    let held = HELD.lock().ok().and_then(|g| *g);
    if let Some(h) = held {
        if all.iter().any(|c| c.handle == h.handle && lo::soft::holds(origin, c)) {
            return Some(h.handle);
        }
        // Gone or out of range: KH1 clears it and picks again in the same frame.
        release();
    }
    if all.is_empty() || !super::command_menu::highlighted_targets() {
        return None;
    }
    lo::soft::pick(origin, facing(pl), &all)
}

/// A command with KH1's flag 4 took the soft target `h`: KH1's auto lock (state 4) from now, its
/// time from 0 again (`0x1402B08A0(…, 4)`).
pub(super) fn hold(h: u32) {
    if let Ok(mut g) = HELD.lock() {
        if g.is_none_or(|x| x.handle != h) {
            log_hold(|| format!("auto lock: an attack took {h:#x} (KH1's state 4: held until 90 frames after the last attack, L2 / R2, or out of range)"));
        }
        *g = Some(lo::soft::Held::take(h));
    }
}

fn release() {
    if let Ok(mut g) = HELD.lock() {
        *g = None;
    }
}

/// Log lines left for the auto lock.
static HOLD_LOG: AtomicUsize = AtomicUsize::new(40);

fn log_hold(f: impl FnOnce() -> String) {
    if HOLD_LOG.load(Ordering::Relaxed) > 0 {
        HOLD_LOG.fetch_sub(1, Ordering::Relaxed);
        log(&f());
    }
}

/// Each frame of a KH1 fighter (the lock-on wrapper): the auto lock's time (Re:CoM's frame delta,
/// `+0x460`, in 60 Hz frames), its end at 90 or on L2 / R2's press (KH1's `0x400000`; Assumption:
/// a press, not a hold), and a lock of Re:CoM's replaces it (KH1's state 3).
unsafe fn hold_tick(pl: usize) {
    let Ok(mut g) = HELD.lock() else { return };
    let Some(h) = g.as_mut() else { return };
    if locked(pl) {
        *g = None;
        return;
    }
    let step = crate::units::units_to_frames(rd::<i32>(pl + mv::p::DT));
    let pressed = |b: u32| func::<PadTestFn>(recom::PAD_PRESSED)(b, 0) != 0;
    let l2_r2 = pressed(lo::bits::L2) || pressed(lo::bits::R2);
    if !h.tick(step, l2_r2) {
        log_hold(|| format!("auto lock: {:#x} released ({})", h.handle, if l2_r2 { "L2 / R2" } else { "90 frames after the last attack" }));
        *g = None;
    }
}

/// Re:CoM's cursor holds a lock (R1).
pub(super) unsafe fn locked(pl: usize) -> bool {
    let cur: usize = rd(pl + recom::CURSOR);
    cur != 0 && rd::<i32>(cur + recom::CURSOR_LOCKED) != 0
}

/// The player's facing as a flat direction (x, z): his mover's yaw (`+0x54`), Re:CoM's
/// `atan2(x, z)` convention (`crate::riku::guard_faces`).
unsafe fn facing(pl: usize) -> [f32; 2] {
    let b: usize = rd(pl + mv::p::BODY);
    if b == 0 {
        return [0.0, 1.0];
    }
    let yaw: f32 = rd(b + mv::body::YAW);
    [yaw.sin(), yaw.cos()]
}

/// 0.180.0: the target of KH1's orange auto-target ring and its lock point. KH1 draws `lockon_d`
/// for lock states 1, 2 and 4 (`0x140274B20`): its soft target while nothing is locked (state 2,
/// set each frame by `0x1402B11B0` only with Auto Lock On) and an attack's auto lock (state 4),
/// on one target; a lock (state 3) draws the lock cursor instead. Checked (decomp, this
/// machine's KH1 project). Here: [`soft_target`] while Re:CoM's cursor holds no lock (its own
/// cursor shows a lock), None otherwise.
pub(super) unsafe fn ring_target(pl: usize) -> Option<(u32, [f32; 3])> {
    if locked(pl) {
        return None;
    }
    let h = soft_target(pl)?;
    target_point(pl, h).map(|p| (h, p))
}

/// The point Re:CoM's cursor measures from (`0x1401F0AB0(out, 0)`).
unsafe fn player_point() -> [f32; 3] {
    let mut v = V4::default();
    func::<PlayerPointFn>(recom::PLAYER_POINT)(&mut v, 0);
    [v.0[0], v.0[1], v.0[2]]
}

/// A lockable actor's lock point (vtable `+0x50`), as Re:CoM's picks and cursor read it.
unsafe fn point_of(obj: usize) -> [f32; 3] {
    let f: PointFn = std::mem::transmute_copy(&rd::<usize>(rd::<usize>(obj) + recom::VT_POINT));
    let mut v = V4::default();
    f(obj, &mut v);
    [v.0[0], v.0[1], v.0[2]]
}

/// Handle `h`'s lock point if it is a lockable actor other than the player.
unsafe fn target_point(pl: usize, h: u32) -> Option<[f32; 3]> {
    let obj = func::<mv::HandleFn>(mv::HANDLE_OBJ)(h);
    (obj != 0 && obj != pl && rd::<u32>(obj + recom::ACTOR_FLAGS) & recom::LOCKABLE != 0).then(|| point_of(obj))
}

/// Re:CoM's lockable targets, from its `LockOnManager`'s list as its own picks walk it
/// (`0x1401CDB90`), each with its KH1 twin's lock-point radius.
unsafe fn targets(pl: usize) -> Vec<Candidate> {
    let mut out = Vec::new();
    let m: usize = rd(base() + recom::MANAGER as usize);
    if m == 0 {
        return out;
    }
    let mut node: usize = rd(m + recom::LIST);
    let mut guard = 0;
    while node != 0 && guard < 256 {
        let next: usize = rd(node + recom::NODE_NEXT);
        if next == 0 {
            break;
        }
        let h: u32 = rd(node);
        let obj = func::<mv::HandleFn>(mv::HANDLE_OBJ)(h);
        if obj != 0 && obj != pl && rd::<u32>(obj + recom::ACTOR_FLAGS) & recom::LOCKABLE != 0 {
            out.push(Candidate { handle: h, point: point_of(obj), radius: super::stats::lock_radius(obj).unwrap_or(0.0) });
        }
        node = next;
        guard += 1;
    }
    out
}

fn rules_for(pad: usize) -> bool {
    let pl = KH1_RULES.load(Ordering::Acquire);
    pl != 0 && pad == pl + recom::PAD
}

/// The lock button pressed, unlocked (`0x1401E8E44`). KH1: R1 locks only a target there is
/// (Re:CoM's pick `+0x84C`, within the reach its cursor takes it); with none, nothing (Re:CoM would
/// arm its lock on nothing, or play its lock sound and drop a target past its reach).
pub unsafe extern "win64" fn lock_pressed(pad: usize) -> u32 {
    let r = func::<PadFn>(recom::PRESSED)(pad);
    if r == 0 || !rules_for(pad) {
        return r;
    }
    let pl = pad - recom::PAD;
    let pick: i32 = rd(pl + recom::PICK);
    let reach = lo::take_reach(battle_room(pl));
    let takes = pick > 0 && target_point(pl, pick as u32).is_some_and(|p| lo::distance(player_point(), p) <= reach);
    u32::from(takes) * r
}

/// The lock button released (`0x1401E8E9C`, locked). KH1: R1 pressed again unlocks at once, so its
/// press is the release after a second press (Re:CoM's unlock runs).
pub unsafe extern "win64" fn lock_released(pad: usize) -> u32 {
    if !rules_for(pad) {
        return func::<PadFn>(recom::RELEASED)(pad);
    }
    // 0.173.0, KH1 FM's Target Lock: Manual (`*(0x142E1F4D8)+4` = 1, `0x1402B069C`): the lock ends
    // when R1 is released; Auto: R1 pressed again ends it.
    let r = if super::settings().target_lock_manual { func::<PadFn>(recom::RELEASED)(pad) } else { func::<PadFn>(recom::PRESSED)(pad) };
    if r != 0 {
        let pl = pad - recom::PAD;
        wr(pl + recom::REQUEST, rd::<u32>(pl + recom::REQUEST) | recom::SECOND_PRESS);
    }
    r
}

/// The lock button held (`0x1401E8EFF`, locked). KH1: only L2's press goes on to a switch (R1 held
/// does nothing, and the right stick stays the camera's).
pub unsafe extern "win64" fn lock_held(pad: usize) -> u32 {
    if !rules_for(pad) {
        return func::<PadFn>(recom::HELD)(pad);
    }
    let l2 = func::<PadTestFn>(recom::PAD_PRESSED)(lo::bits::L2, 0) != 0;
    (rd::<u32>(pad) & 1) * u32::from(l2)
}

/// The right stick flicked (`0x1401E8F2D`, held). KH1: reached only on L2's press: yes.
pub unsafe extern "win64" fn lock_flick(pad: usize) -> u32 {
    if !rules_for(pad) {
        return func::<PadFn>(recom::FLICK)(pad);
    }
    1
}

/// The target switch (`0x1401E8F3C`, `ecx` = the locked target `+0x850`). KH1: L2's next target
/// in its lock list ([`History::step`] with KH1's pick [`lo::nearest`]); 0: none (no switch).
pub unsafe extern "win64" fn lock_switch(current: u32) -> i32 {
    let pl = KH1_RULES.load(Ordering::Acquire);
    if pl == 0 {
        return func::<SwitchFn>(recom::SWITCH)(current);
    }
    let reach = lo::take_reach(battle_room(pl));
    let origin = player_point();
    let all = targets(pl);
    let (next, list) = with_list(|l| {
        // KH1's list starts at the lock; a list that missed it starts here.
        if l.is_empty() && current as i32 > 0 {
            l.sync(Some(current));
        }
        l.prune(|h| all.iter().any(|c| c.handle == h && lo::distance(origin, c.point) <= reach));
        let n = l.step(|listed| lo::nearest(origin, &all, reach, listed));
        (n, l.entries().to_vec())
    });
    if STEP_LOG.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("lock-on: L2 on {current:#x}: next {}; list {list:x?}; {} lockable within {reach} m", next.map_or("none".into(), |h| format!("{h:#x}")), all.iter().filter(|c| lo::distance(origin, c.point) <= reach).count()));
    }
    if let Some(h) = next {
        STEPPED.store(h, Ordering::Relaxed);
    }
    next.map_or(0, |h| h as i32)
}

/// Retargets the lock-on's five input calls (`sites::LOCK_ON_CALLS`) through one stub block, all
/// or nothing, after checking what they rely on (`sites::LOCK_ON_RULES_CHECKED`).
pub unsafe fn install_rules(b: usize) -> bool {
    use crate::sites::{LOCK_ON_CALLS, LOCK_ON_RULES_CHECKED};
    for s in LOCK_ON_CALLS.iter().chain(LOCK_ON_RULES_CHECKED) {
        if std::slice::from_raw_parts((b + s.rva as usize) as *const u8, s.expect.len()) != s.expect {
            log(&format!("lock-on: {} is not as expected: Re:CoM's lock rules stay on R1", s.name));
            return false;
        }
    }
    let ours: [usize; 5] = [lock_pressed as *const () as usize, lock_released as *const () as usize, lock_held as *const () as usize, lock_flick as *const () as usize, lock_switch as *const () as usize];
    let mut code = Vec::with_capacity(14 * ours.len());
    for f in ours {
        code.extend_from_slice(&crate::hookplan::abs_jmp(f as u64));
    }
    let first = b + LOCK_ON_CALLS[0].rva as usize;
    // Five 14-byte jumps are 70 bytes, past `place_near`'s 64-byte block (0.139.1: that refusal
    // was logged as "no stub block within reach"); placed as one built block instead.
    let Some(block) = hook::place_built_near(first, code.len(), |_| code.clone()) else {
        log("lock-on: no stub block within reach of Re:CoM's lock-on: Re:CoM's lock rules stay on R1");
        return false;
    };
    let mut calls = Vec::with_capacity(LOCK_ON_CALLS.len());
    for (i, s) in LOCK_ON_CALLS.iter().enumerate() {
        let at = b + s.rva as usize;
        let Some(c) = crate::hookplan::call_rel32(at as u64, (block + 14 * i) as u64) else {
            log("lock-on: a stub out of call reach: Re:CoM's lock rules stay on R1");
            return false;
        };
        calls.push((at, s, c));
    }
    for (n, (at, s, c)) in calls.iter().enumerate() {
        if !hook::patch(*at, s.expect, c, s.name) {
            for (at, s, c) in calls[..n].iter().rev() {
                hook::patch(*at, c, s.expect, "lock-on: undo");
            }
            log("lock-on: a call could not be retargeted (all undone): Re:CoM's lock rules stay on R1");
            return false;
        }
    }
    RULES_ON.store(true, Ordering::Release);
    log(&format!("lock-on: KH1's lock rules on R1 at combat = kh1 on a controller (Re:CoM's lock-on's pressed / released / held / flick / switch calls 0x1401E8E44 / E9C / EFF / F2D / F3C through {block:#x}): R1 with no target does nothing, R1 again unlocks at once, L2 steps through KH1's lock list, the right stick switches nothing"));
    true
}
