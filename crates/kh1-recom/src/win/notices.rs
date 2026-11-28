//! M12 step 2: "Sora learned X" notices on Re:CoM's own sleight-learned screen (`crate::notices`,
//! `crate::level_up`, `docs/recom-m12-sleights.md` § 8).
//!
//! - [`notify_learned`] queues one (later steps call it for sleights too).
//! - **After a Re:CoM screen:** when Re:CoM's level-up screen (state 0xB → 0xD) or its own
//!   sleight-learned screen (0x11 → 0xD) ends at Combat = KH1 in Sora's story, the same `CLevelUp`
//!   task is turned to its sleight-learned mode for the next notice (`+0x258` = the screen id,
//!   state 1): Re:CoM's own states 0xE..0x11 show it, and whatever waited for that task (the
//!   level-up's game step `0x1401AE340`, a chest) waits for it too. Called from the level-up
//!   wrapper (`super::level_up`), so nothing new runs per frame there.
//! - **Otherwise:** at the next frame Re:CoM's own field gate for opening the camp holds
//!   (`0x1401A96C0` without its Start press: the game state `*0x14087B0B0` with `+0x18 & 0x8008`
//!   and `+0x5A0` bit 11 clear, no `(1, 0x33)` + `(1, 0x34)` flags, `0x1401AAA90` (its `+0x5A4`,
//!   the event flag `(1, 6)`, `0x140234570`, `+0x5A0 & 0x80201C01`), `+0x580` = 9) with Sora in the
//!   field: a new `CLevelUp(new 0x2C0, 0x1402AD300(1), id)`, as Re:CoM's event `0xAD 7` makes
//!   it. Per frame: one atomic load ([`frame`]) unless a notice waits.
//! - **The name:** when that screen leaves state 1 / 0 for 0xE (it then holds Re:CoM's name for
//!   the id in `+0x250`), `+0x250` is pointed at our name (static; the line is formatted at 0xE),
//!   once per notice (the task's pointer is forgotten then, so a later screen at a reused address
//!   is never touched).
//! - **No `CTechHelp` at KH1** (owner): when a sleight-learned screen goes 0xF → 0x10, or a level-up
//!   whose Sleight row was picked goes 8 → 9 (owner, 2026-11-03: "skip the card help page after the
//!   level-up Sleight pick too"), it has just made its `CTechHelp`, the last child of its task list:
//!   set to state 5 with no page, Re:CoM's own update ends it on its first run (nothing loaded, none
//!   unloaded) and its dtor sets `0x14087A4B0`, which 0x10 / 9 wait for.
//! - **Shown by Re:CoM** (M12 step 4, at any Combat setting): a sleight ability whose screen Re:CoM
//!   itself just showed gets no second notice ([`shown_by_recom`]): a Sora level-up's Sleight pick
//!   (6 → 7 on row 2: the sleight `crate::level_up::SLEIGHT_LIST[+0x240]` it learns) and every
//!   sleight-learned screen (state 1 / 0 → 0xE with `+0x258` = the id: its event `0xAD 7`, its
//!   chest) mark the id for the session; a notice for it still waiting is taken out.

use super::movement::{func, rd, wr};
use super::{base, log, va};
use crate::level_up::{off, state, tech_help};
use crate::notices::{Learned, Queue};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;

static QUEUE: Mutex<Queue> = Mutex::new(Queue::new());
/// A notice waits (one atomic load per frame in [`frame`]).
static PENDING: AtomicBool = AtomicBool::new(false);
/// The `CLevelUp` about to show our notice (0: none), until its name is in; and that name.
static CURRENT: AtomicUsize = AtomicUsize::new(0);
static NAME: Mutex<[u8; crate::notices::NAME_MAX + 1]> = Mutex::new([0; crate::notices::NAME_MAX + 1]);
static HAS_NAME: AtomicBool = AtomicBool::new(false);
static LOG_LEFT: AtomicU32 = AtomicU32::new(60);
/// `crate::sites::M12_PTRS` checked once (a mismatch turns the notices off, logged).
static PTRS_OK: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
/// The sleight ids (0..128) whose screen Re:CoM showed this session ([`shown_by_recom`]).
static SHOWN: [AtomicU64; 2] = [AtomicU64::new(0), AtomicU64::new(0)];

/// Whether Re:CoM's own screen showed sleight `id` this session (its level-up Sleight pick, its
/// sleight-learned screen).
pub fn shown_by_recom(id: u16) -> bool {
    SHOWN.get(id as usize / 64).is_some_and(|w| w.load(Ordering::Relaxed) & (1 << (id % 64)) != 0)
}

fn mark_shown(id: u16, how: &str) {
    if crate::abilities::of_sleight(id).is_none() {
        return;
    }
    let Some(w) = SHOWN.get(id as usize / 64) else { return };
    w.fetch_or(1 << (id % 64), Ordering::Relaxed);
    let removed = QUEUE.lock().map(|mut q| {
        let r = q.remove(Learned::Sleight(id));
        PENDING.store(!q.is_empty(), Ordering::Relaxed);
        r
    });
    note(|| format!("sleight {id:#x} shown by Re:CoM ({how}): no second notice{}", if removed.unwrap_or(false) { " (its waiting notice taken out)" } else { "" }));
}

fn ptrs_ok() -> bool {
    *PTRS_OK.get_or_init(|| {
        let b = base();
        // SAFETY: the exe's own data section (each a pointer-sized vtable slot).
        let bad = crate::sites::M12_PTRS.iter().find(|p| unsafe { rd::<usize>(b + p.rva as usize) } != b + p.target as usize);
        if let Some(p) = bad {
            log(&format!("notices: {} does not point at {:#x}; no learned notices", p.name, 0x1_4000_0000u64 + p.target as u64));
        }
        bad.is_none()
    })
}

const NEW: u32 = 0x004A_CE54;
const LEVEL_UP_CTOR: u32 = 0x0010_E9C0;
const TASK_ROOT: u32 = 0x002A_D300;
const MENU_GATE: u32 = 0x001A_AA90;
const FLAG_GET: u32 = 0x001B_0870;
/// Re:CoM's game state (the field's `CGame…`; `0x1401A96C0` reads it).
const GAME: u64 = 0x1_4087_B0B0;

type NewFn = unsafe extern "win64" fn(usize) -> *mut u8;
type CtorFn = unsafe extern "win64" fn(*mut u8, usize, u32, u64) -> *mut u8;
type RootFn = unsafe extern "win64" fn(u32) -> usize;
type GateFn = unsafe extern "win64" fn(usize) -> u32;
type FlagFn = unsafe extern "win64" fn(u32, u32) -> u8;

fn note(line: impl FnOnce() -> String) {
    if LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("notices: {}", line()));
    }
}

fn describe(l: Learned) -> String {
    match l {
        Learned::Ability(i) => format!("ability {}", crate::abilities::ALL.get(i).map_or("?", |a| a.name)),
        Learned::Sleight(id) => format!("sleight {id:#x}"),
    }
}

/// Queues "Sora learned X" (shown on Re:CoM's sleight-learned screen at the next natural moment).
pub fn notify_learned(l: Learned) {
    let Ok(mut q) = QUEUE.lock() else { return };
    if q.push(l) {
        PENDING.store(true, Ordering::Relaxed);
        note(|| format!("queued {} ({} waiting)", describe(l), q.len()));
    }
}

fn pop() -> Option<Learned> {
    let mut q = QUEUE.lock().ok()?;
    let l = q.pop();
    PENDING.store(!q.is_empty(), Ordering::Relaxed);
    l
}

/// Points the notice at `this` (its name, the task).
fn take_name(this: usize, l: Learned) {
    let name = l.name();
    if let (Some(n), Ok(mut g)) = (&name, NAME.lock()) {
        g.fill(0);
        let k = n.len().min(g.len() - 1);
        g[..k].copy_from_slice(&n[..k]);
    }
    HAS_NAME.store(name.is_some(), Ordering::Relaxed);
    CURRENT.store(this, Ordering::Relaxed);
}

/// Turns an ending `CLevelUp` (state 0xD just set) to its sleight-learned mode for `l`.
unsafe fn start_on(this: usize, l: Learned) {
    if !ptrs_ok() {
        return;
    }
    take_name(this, l);
    wr(this + off::SLEIGHT, l.screen_id() as u32);
    wr(this + off::STATE, state::INIT);
    note(|| format!("{} on Re:CoM's sleight-learned screen (id {:#x}), after the screen before it", describe(l), l.screen_id()));
}

/// From the level-up wrapper, after Re:CoM's update ran (`before` → `after` its state).
pub(super) unsafe fn after_step(this: usize, before: i32, after: i32) {
    // M12 step 4: Re:CoM's own screens for a sleight (any Combat setting).
    if before == state::PICK && after == state::PICKED {
        let character: u32 = rd(this + off::CHARACTER);
        if rd::<u32>(this + off::SLEIGHT) == 0 && character == 0 && rd::<i32>(this + off::CURSOR) == crate::level_up::SLEIGHT_ROW as i32 {
            if let Some(id) = crate::level_up::picked_sleight(rd(this + off::OFFER + 4 * crate::level_up::SLEIGHT_ROW)) {
                mark_shown(id, "its level-up Sleight pick");
            }
        }
    }
    if after == state::LEARNED && (before == state::INIT || before == state::RESOURCES) && CURRENT.load(Ordering::Relaxed) != this {
        let id: u32 = rd(this + off::SLEIGHT);
        if let Ok(id) = u16::try_from(id) {
            mark_shown(id, "its sleight-learned screen");
        }
    }
    // Our name into the line, once Re:CoM has set its own (state 1 / 0 → 0xE); used once.
    if after == state::LEARNED && (before == state::INIT || before == state::RESOURCES) && CURRENT.load(Ordering::Relaxed) == this {
        CURRENT.store(0, Ordering::Relaxed);
        if HAS_NAME.swap(false, Ordering::Relaxed) {
            if let Ok(g) = NAME.lock() {
                wr(this + off::NAME, g.as_ptr() as usize);
            }
        }
    }
    // No card-stock help page at KH1 (owner): the CTechHelp 0xF just made ends at once.
    if ((before == state::LEARNED_SHOW && after == state::LEARNED_HELP) || (before == state::RESULT && after == state::PICK_HELP)) && super::mp::live() {
        skip_tech_help(this);
    }
    if after != state::END || before == state::END {
        return;
    }
    // The next notice on the same task, when this screen was a level-up or a sleight-learned one.
    if (before == state::NEXT || before == state::LEARNED_CLOSE) && PENDING.load(Ordering::Relaxed) && super::mp::live() {
        if let Some(l) = pop() {
            start_on(this, l);
        }
    }
}

unsafe fn skip_tech_help(this: usize) {
    if !ptrs_ok() {
        return;
    }
    let node: usize = rd(this + off::CHILD_TAIL);
    if node == 0 {
        return;
    }
    let th: usize = rd(node + off::NODE_TASK);
    if th == 0 || rd::<usize>(th) != va(base(), tech_help::VTABLE) || rd::<u32>(th + tech_help::STATE) != 0 {
        note(|| format!("CLevelUp {this:#x}: its last child is not a fresh CTechHelp ({th:#x}); left as is"));
        return;
    }
    wr(th + tech_help::STATE, tech_help::DONE);
    wr(th + tech_help::PAGE, 0u32);
    note(|| {
        let id = rd::<u32>(this + off::SLEIGHT);
        let what = if id == 0 { "level-up (its Sleight pick)".to_string() } else { format!("sleight-learned screen (id {id:#x})") };
        format!("{what} {this:#x}: CTechHelp {th:#x} ended before its page (KH1: no card-stock help)")
    });
}

/// Re:CoM's gate for opening the camp in the field, without the Start press (`0x1401A96C0`).
unsafe fn field_gate() -> bool {
    let g: usize = rd(va(base(), GAME));
    if g == 0 || rd::<u16>(g + 0x18) & 0x8008 != 0 || rd::<u32>(g + 0x5A0) & 0x800 != 0 {
        return false;
    }
    let flag = func::<FlagFn>(FLAG_GET);
    if flag(1, 0x33) != 0 && flag(1, 0x34) != 0 {
        return false;
    }
    // Not while a field sleight / special / summon runs (the camp gate's rule, `super::field_sleights`).
    !super::field_sleights::camp_shut() && func::<GateFn>(MENU_GATE)(g) & 0xFF != 0 && rd::<i32>(g + 0x580) == 9
}

/// Each KH1 frame of Sora (the request-bits wrapper): one atomic load, unless a notice waits; then
/// in the field at Re:CoM's camp gate, Re:CoM's sleight-learned screen for it. (While any
/// `CLevelUp` is up the gate is shut: its ctor clears the game state's `+0x5A4` and its dtor sets it
/// again, `0x1401A9CC0`.)
pub(super) unsafe fn frame(pl: usize) {
    if !PENDING.load(Ordering::Relaxed) || !ptrs_ok() {
        return;
    }
    if rd::<i32>(pl + super::movement::p::BATTLE) != 0 || !super::mp::live() || !field_gate() {
        return;
    }
    let Some(l) = pop() else { return };
    let obj = func::<NewFn>(NEW)(0x2C0);
    if obj.is_null() {
        notify_learned(l);
        return;
    }
    let parent = func::<RootFn>(TASK_ROOT)(1);
    take_name(obj as usize, l);
    func::<CtorFn>(LEVEL_UP_CTOR)(obj, parent, l.screen_id() as u32, 0);
    note(|| format!("{} on Re:CoM's sleight-learned screen (id {:#x}): a new CLevelUp {:#x} at Re:CoM's field camp gate", describe(l), l.screen_id(), obj as usize));
}
