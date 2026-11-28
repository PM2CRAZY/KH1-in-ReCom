//! Boss revenge at Combat = KH1 (`crate::revenge`, `docs/recom-boss-revenge.md`).
//!
//! - **Counter:** a 0x30-byte tail after each revenge boss body (`crate::revenge::tail`), made by
//!   raising the size Re:CoM's enemy factory `0x140188F40` allocates for the class (one 5-byte
//!   `mov ecx, size` each, written once at install) and set up when Re:CoM's stats init
//!   `0x14017FF50` runs for the boss (our existing wrapper). A tail from another battle or class is
//!   set up again on its first use (its magic and battle number).
//! - **Add:** [`enemy_hit`] wraps the existing KH1 enemy-hit wrapper (`super::attack::enemy_hit`,
//!   Re:CoM's `0x14017F7E0`): a landed hit adds its value (`crate::revenge::value_of`; the source is
//!   noted by `super::attack`). Marluxia's parts add to Marluxia's body. Re:CoM's boss damage
//!   falloff (`+0x288` / `+0x28C` / `+0x290`, written by `0x14017F7E0`) is put back after the call
//!   for bosses whose revenge acts (owner: off for them).
//! - **Decay:** lazy, 1 per 60 Hz frame of the game clock ([`frame`], Sora's own dt added in the
//!   existing request-bits wrapper), minus the frames the boss spent in its flinch / blow-away.
//! - **Check:** hooks on Re:CoM's flinch step `0x140184100` (state 11) and blow-away step
//!   `0x140182E50` (state 12), shared by every enemy: they return at once for other enemies.
//!   At KH2's frame 20 (Hades 30) of the state, or at its end when sooner (the bosses' state-change
//!   callback `0x1401A3B80`, written in their state machines' tables: [`state_change`]),
//!   limit ≤ counter starts the boss's own attack state with the transition Re:CoM's message 2
//!   makes (the cardless card play P1's: `crate::battle::cardless_flags`, `state_address`).
//!
//! - **Untouchable revenge move** (owner, 2026-11-02): the start marks the tail pending; when
//!   Re:CoM begins that attack state its "hurt bodies off" (`0x14017DA80`, `super::attack::hurt_off`)
//!   runs for this attack ([`begin_untouchable`]), and hits on the boss go to Re:CoM's handler
//!   without the card-interrupt bit (`super::attack::untouchable_hit`), so Re:CoM's own rule drops
//!   them (`+0x260 & 1`). No Guard reaction either (`super::guard`). The window closes in Re:CoM's
//!   attack-state end `0x14017DA90` (hooked: [`attack_end`]), which turns the bodies back on.
//!   Nothing adds to the counter during it; the counter keeps draining.
//!
//! - **Room-dimming moves untouchable** (owner, 2026-11-03, `crate::dim`): the same window, opened
//!   when the boss's own code asks Re:CoM for a dark room during an attack state (its begin: slot 0
//!   snapshot in [`state_change`], compared in [`begin_untouchable`]; its step: [`EXES`]'s four
//!   exe slots; Marluxia 3: its own slot dark at the begin), closed by the same state end. Whatever
//!   `boss_revenge` says.
//!
//! - **The Wizard and the Aquatank** (owner, 2026-11-03: "the same rule", `crate::dim::NORMAL`):
//!   normal enemies on Re:CoM's AI whose attack begin always dims the room (Wizard state 4,
//!   Aquatank state 3). No tail: at that begin's "hurt bodies off" ([`begin_untouchable`]) a fresh
//!   dark request in slot 0 lets Re:CoM's own call run and opens a window kept in a small list
//!   (`NORMAL_WINDOWS`); hits on it go through `super::attack::untouchable_hit`, no Guard reaction;
//!   Re:CoM's state end (`0x14017DA90`, [`attack_end`]) turns the bodies on and closes it.
//!
//! Only in a KH1 battle (Combat = KH1, Sora's story); the revenge value with `boss_revenge = on`.

use super::movement::{func, orig, rd, wr};
use super::{base, log};
use crate::revenge::{self as rv, flag, tail, Boss, Counter, Ignored, IgnoredKind, Source};
use crate::dim::{self, EXES, SLOT_SIZE};
use crate::sites::{PtrSite, DIM_EXE_SLOTS, DIM_NORMAL_PTRS, REVENGE_CHECKED, REVENGE_FSM_SLOTS, REVENGE_HOOKS, REVENGE_PTRS, REVENGE_WINDUP_SLOTS};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;

pub static ORIG_FLINCH: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_BLOW: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_ATTACK_END: AtomicUsize = AtomicUsize::new(0);
static INSTALLED: AtomicBool = AtomicBool::new(false);
static ENABLED: AtomicBool = AtomicBool::new(true);
/// The game clock: Re:CoM time units (50 a 60 Hz frame), Sora's dt summed each frame.
static CLOCK: AtomicU64 = AtomicU64::new(0);
/// The source of the hit being taken (0 none; 1 << 16 | row (0xFFFF: no row); 2 << 16 | spell).
static NOTE: AtomicU32 = AtomicU32::new(0);
/// The last Marluxia 2 / 3 body set up (its parts' hits count on it).
static MARLUXIA_BODY: AtomicUsize = AtomicUsize::new(0);
static MARLUXIA_BATTLE: AtomicU32 = AtomicU32::new(u32::MAX);
static LINES: AtomicU32 = AtomicU32::new(0);
const LINE_CAP: u32 = 200;
/// The dim windows' own log lines (start and end of each move), apart from the revenge lines.
static DIM_LINES: AtomicU32 = AtomicU32::new(0);
/// Slot 0 of the scene colour table at the last revenge-boss state change (the boss, its bytes):
/// the begin that follows is compared with it.
static DIM_SNAP: Mutex<(usize, [u8; SLOT_SIZE])> = Mutex::new((0, [0; SLOT_SIZE]));
/// The boss whose window a step opens now: its `+0x438` call reaches the hurt-off hook, which then
/// runs Re:CoM's own call.
static DIM_OPENING: AtomicUsize = AtomicUsize::new(0);
/// The Wizard's and the Aquatank's dim windows (`crate::dim::NORMAL`): their classes and slots
/// checked at install (`sites::DIM_NORMAL_PTRS`), the open windows, and how many (one load on the
/// hit handler's path while none is open).
static NORMAL_OK: AtomicBool = AtomicBool::new(false);
static NORMAL_WINDOWS: Mutex<dim::Windows> = Mutex::new(dim::Windows::new());
static NORMAL_OPEN: AtomicUsize = AtomicUsize::new(0);
/// Re:CoM's enemy vtable slot run by every attack state's begin: "hurt bodies off" (`0x14017DA80`,
/// hooked: `super::attack::hurt_off`; Vexen's and Marluxia 2's own call it). Checked (exe:
/// `REVENGE_PTRS`).
const EN_HURT_OFF_SLOT: usize = 0x438;

type StepFn = unsafe extern "win64" fn(usize);
type MaxHpFn = unsafe extern "win64" fn(usize) -> i32;
/// Re:CoM's enemy max-HP getter (hooked by `super::stats`; first bytes checked there).
const ENEMY_MAX_HP: u32 = 0x0017_F5A0;
/// Sora's dt (Re:CoM time units; `super::movement` `p::DT`).
const PL_DT: usize = 0x460;
/// The enemy's dt (`CEntity +0x14`, what the enemy timers subtract; `0x140185320`).
const EN_DT: usize = 0x14;
const EN_STATE: usize = 0x60;
const EN_NEXT: usize = 0x68;
const EN_FLAGS: usize = 0x260;
const EN_HP: usize = 0x274;
const EN_CHOSEN: usize = 0x278;
/// Re:CoM's flinch and blow-away states (`+0x4C0 + k × 0x40`). Checked (decomp).
const FLINCH_STATE: i32 = 11;
const BLOW_STATE: i32 = 12;
/// Re:CoM's "every hurt body on" (`0x1401A2DB0(enemy, -1)`, the call its attack-state end makes).
const BODIES_ON: u32 = 0x001A_2DB0;
type BodiesFn = unsafe extern "win64" fn(usize, i32);
/// Re:CoM's boss damage falloff (`0x14017F7E0` for ids 100..199: hit count, reset timer,
/// multiplier). Checked (decomp).
const FALLOFF_COUNT: usize = 0x288;
const FALLOFF_TIMER: usize = 0x28C;
const FALLOFF_MULT: usize = 0x290;

/// Re:CoM's boss falloff fields of one enemy (`+0x288` hit count, `+0x28C` reset timer, `+0x290`
/// multiplier), saved before a hit and put back after it: the hit neither takes the falloff
/// (with [`Falloff::neutral`]) nor feeds it. Revenge bosses (owner: falloff off) and, at Damage =
/// Re:CoM, Gravity Break / Zantetsuken hits (`super::attack`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Falloff {
    count: i32,
    timer: i32,
    mult: f32,
}

impl Falloff {
    pub(super) unsafe fn save(enemy: usize) -> Falloff {
        Falloff { count: rd(enemy + FALLOFF_COUNT), timer: rd(enemy + FALLOFF_TIMER), mult: rd(enemy + FALLOFF_MULT) }
    }

    /// The multiplier at 1.0 for the coming call (`0x14017DAB0` multiplies the damage by it).
    pub(super) unsafe fn neutral(enemy: usize) {
        wr(enemy + FALLOFF_MULT, 1.0f32);
    }

    pub(super) unsafe fn restore(self, enemy: usize) {
        wr(enemy + FALLOFF_COUNT, self.count);
        wr(enemy + FALLOFF_TIMER, self.timer);
        wr(enemy + FALLOFF_MULT, self.mult);
    }

    pub(super) fn mult(&self) -> f32 {
        self.mult
    }
}

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

fn live() -> bool {
    active() && ENABLED.load(Ordering::Relaxed)
}

/// A KH1 battle with the tails in place: the untouchable windows (revenge moves, dimming moves).
fn active() -> bool {
    INSTALLED.load(Ordering::Relaxed) && super::battle::kh1_battle()
}

/// Sora's frame (from the request-bits wrapper): the clock advances by his dt.
pub(super) unsafe fn frame(pl: usize) {
    if INSTALLED.load(Ordering::Relaxed) {
        CLOCK.fetch_add(rd::<i32>(pl + PL_DT).clamp(0, 1000) as u64, Ordering::Relaxed);
    }
}

fn now() -> f64 {
    CLOCK.load(Ordering::Relaxed) as f64 / rv::TICKS_PER_FRAME
}

/// `super::attack`: the hit being taken is a KH1 swing with this row.
pub(super) fn note_swing(row: Option<usize>) {
    NOTE.store(1 << 16 | row.map_or(0xFFFF, |r| (r as u32) & 0xFFFF), Ordering::Relaxed);
}

/// `super::attack`: the hit being taken is a KH1 spell's.
pub(super) fn note_spell(s: u8) {
    NOTE.store(2 << 16 | s as u32, Ordering::Relaxed);
}

fn take_note() -> Source {
    let n = NOTE.swap(0, Ordering::Relaxed);
    match n >> 16 {
        1 if n & 0xFFFF == 0xFFFF => Source::Swing(None),
        1 => Source::Swing(Some((n & 0xFFFF) as usize)),
        2 => Source::Spell((n & 0xFF) as u8),
        _ => Source::Other,
    }
}

unsafe fn vt_rva(e: usize) -> u32 {
    let vt: usize = rd(e);
    vt.wrapping_sub(base()) as u32
}

unsafe fn boss_of(e: usize) -> Option<&'static Boss> {
    if e == 0 {
        return None;
    }
    rv::boss_by_vtable(vt_rva(e))
}

/// Whether the tail was set up in this battle.
unsafe fn tail_ok(e: usize, b: &Boss) -> bool {
    let t = e + b.size as usize;
    rd::<u32>(t + tail::MAGIC) == rv::MAGIC && rd::<u32>(t + tail::BATTLE) == super::battle::battle_number()
}

/// The tail's counter (set up again when it is another battle's or never was).
unsafe fn counter(e: usize, b: &Boss) -> Counter {
    let t = e + b.size as usize;
    if !tail_ok(e, b) {
        setup(e, b);
    }
    Counter {
        value: rd(t + tail::COUNTER),
        last: rd(t + tail::LAST),
        flinch_since: rd(t + tail::FLINCH_SINCE),
        flinch_elapsed: rd(t + tail::FLINCH_ELAPSED),
        flags: rd(t + tail::FLAGS),
    }
}

unsafe fn store(e: usize, b: &Boss, c: &Counter) {
    let t = e + b.size as usize;
    wr(t + tail::COUNTER, c.value);
    wr(t + tail::LAST, c.last);
    wr(t + tail::FLINCH_SINCE, c.flinch_since);
    wr(t + tail::FLINCH_ELAPSED, c.flinch_elapsed);
    wr(t + tail::FLAGS, c.flags);
}

unsafe fn setup(e: usize, b: &Boss) {
    let t = e + b.size as usize;
    std::ptr::write_bytes(t as *mut u8, 0, rv::TAIL as usize);
    wr(t + tail::MAGIC, rv::MAGIC);
    wr(t + tail::BATTLE, super::battle::battle_number());
    store(e, b, &Counter::new(now()));
    if rv::owns_parts(b) {
        MARLUXIA_BODY.store(e, Ordering::Relaxed);
        MARLUXIA_BATTLE.store(super::battle::battle_number(), Ordering::Relaxed);
    }
}

/// Re:CoM's stats init ran for this enemy (`super::stats::stat_init`): a boss body's tail is set
/// up (the allocation's tail holds whatever the heap had).
pub(super) unsafe fn on_init(e: usize) {
    if !INSTALLED.load(Ordering::Relaxed) {
        return;
    }
    if let Some(b) = boss_of(e) {
        setup(e, b);
    }
}

unsafe fn max_hp(e: usize) -> i32 {
    func::<MaxHpFn>(ENEMY_MAX_HP)(e)
}

/// The limit now (Hades: re-checked against his HP, as KH2 does on each hit).
unsafe fn limit_now(e: usize, b: &Boss, c: &mut Counter) -> f32 {
    let (l, low) = rv::limit(b, rd(e + EN_HP), max_hp(e), c.flags & flag::HADES_LOW != 0);
    if low {
        c.flags |= flag::HADES_LOW;
    }
    l
}

fn line(f: impl FnOnce() -> String) {
    if LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| (n < LINE_CAP).then_some(n + 1)).is_ok() {
        log(&f());
    }
}

fn dim_line(f: impl FnOnce() -> String) {
    if DIM_LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| (n < LINE_CAP).then_some(n + 1)).is_ok() {
        log(&f());
    }
}

/// The trace's hits column: `rv <counter>` on this enemy's hit.
fn trace_tag(e: usize, value: f32) {
    if let Ok(mut g) = super::attack::HIT_LOG.lock() {
        let key = format!("e{:x}", e & 0x00FF_FFFF);
        let n = g.len();
        match g.last_mut() {
            Some(l) if l.starts_with(&key) => l.push_str(&format!(":rv {value:.1}")),
            _ if n < 16 => g.push(format!("{key}:rv {value:.1}")),
            _ => {}
        }
    }
}

/// The enemy-hit hook (Re:CoM's `0x14017F7E0`): the KH1 wrapper, then the revenge add.
pub unsafe extern "win64" fn enemy_hit(enemy: usize) -> u64 {
    if !active() {
        return super::attack::enemy_hit(enemy);
    }
    let (body, b) = match boss_of(enemy) {
        Some(b) => (enemy, b),
        None if rv::is_part(vt_rva(enemy)) => {
            let same = MARLUXIA_BATTLE.load(Ordering::Relaxed) == super::battle::battle_number();
            let body = if same { MARLUXIA_BODY.load(Ordering::Relaxed) } else { 0 };
            match boss_of(body).filter(|b| rv::owns_parts(b)) {
                Some(b) => (body, b),
                None => return super::attack::enemy_hit(enemy),
            }
        }
        // The Wizard / Aquatank in a dimming move (`crate::dim::NORMAL`): Re:CoM's rule for an
        // attacking enemy, as for a boss.
        None if normal_window(enemy) => return normal_hit(enemy),
        None => return super::attack::enemy_hit(enemy),
    };
    // Revenge moves untouchable (owner, 2026-11-02): while the body plays the attack its revenge
    // started, Re:CoM's own rule decides a hit on it, and nothing adds to the counter (its hurt
    // bodies are off, so Sora's attacks don't meet it: KH2 adds only on a hit that meets the enemy,
    // `0x1403DB730` through its on-hit slot `+0xC0`). A part's own hits keep their Re:CoM rules.
    if untouchable(body) {
        if body != enemy {
            return super::attack::enemy_hit(enemy);
        }
        let (r, kind) = super::attack::untouchable_hit(enemy);
        if let Some(kind) = kind {
            let t = enemy + b.size as usize;
            let mut ig = Ignored::from_bytes(rd(t + tail::IGNORED));
            ig.add(kind);
            wr(t + tail::IGNORED, ig.to_bytes());
        }
        if r != 0 {
            line(|| format!("revenge: {} {enemy:#x} took a hit in its untouchable move: Re:CoM's handler applied it (flags {:#x})", b.name, rd::<u64>(enemy + EN_FLAGS)));
        }
        return r;
    }
    if !ENABLED.load(Ordering::Relaxed) {
        // `boss_revenge = off`: no counter, Re:CoM's falloff (only the dim windows above).
        return super::attack::enemy_hit(enemy);
    }
    let falloff = b.falloff_off().then(|| Falloff::save(enemy));
    NOTE.store(0, Ordering::Relaxed);
    let r = super::attack::enemy_hit(enemy);
    let src = take_note();
    if let Some(f) = falloff {
        f.restore(enemy);
    }
    // A landed hit (Re:CoM applied it). Held hits of a stopped enemy count when they land.
    if r != 0 && !super::magic::stopped(enemy) {
        let v = rv::value_of(src);
        let mut c = counter(body, b);
        if body == enemy {
            c.hit(now(), v);
        } else {
            // A part's hit: on the body's counter; the body's own flinch goes on.
            c.decay(now());
            c.value += v;
        }
        limit_now(body, b, &mut c);
        store(body, b, &c);
        trace_tag(enemy, c.value);
    }
    r
}

/// The revenge decision for this reaction (once: `Counter::decide` marks it): limit ≤ counter starts
/// the boss's own attack state with message 2's transition. `at_end`: the reaction is ending (Re:CoM's
/// state machine is between its end and the next begin, and re-reads `+0x68` after: `0x140242F75`).
unsafe fn decide(e: usize, b: &Boss, c: &mut Counter, at_end: bool) {
    let l = limit_now(e, b, c);
    let at = c.flinch_elapsed;
    if !c.decide(now(), l) {
        return;
    }
    let value = c.value;
    match b.attack {
        Some((state, what)) if super::battle::may_start_attack(e) => {
            let flags: u64 = rd(e + EN_FLAGS);
            wr(e + EN_FLAGS, crate::battle::cardless_flags(flags));
            wr(e + EN_CHOSEN, state);
            wr(e + EN_NEXT, crate::battle::state_address(e, state));
            // The untouchable window opens when Re:CoM begins this state.
            c.flags = (c.flags | flag::PENDING) & !(flag::UNTOUCHABLE | flag::OWN);
            wr(e + b.size as usize + tail::STATE, state);
            line(|| format!("revenge: {} {e:#x} counter {value:.1} >= {l} at frame {at:.0} of its {} -> state {state} ({what})", b.name, if at_end { "reaction's end" } else { "reaction" }));
        }
        Some((state, _)) => line(|| format!("revenge: {} {e:#x} counter {value:.1} >= {l}, but it may not attack now (dead or CBTL locked): state {state} not started", b.name)),
        None => line(|| format!("revenge: {} {e:#x} counter {value:.1} >= {l}: no attack state of its own, nothing starts", b.name)),
    }
}

/// A frame of Re:CoM's flinch / blow-away: counts it; at KH2's frame (20, Hades 30) decides. An
/// earlier end is decided in [`state_change`].
unsafe fn step(e: usize, o: StepFn) {
    if !live() {
        return o(e);
    }
    let Some(b) = boss_of(e) else { return o(e) };
    let dt = rd::<i32>(e + EN_DT).clamp(0, 1000) as f32 / rv::TICKS_PER_FRAME as f32;
    o(e);
    let mut c = counter(e, b);
    c.flinch_frame(dt);
    if c.due(b.check_frame(), false) {
        decide(e, b, &mut c, false);
    }
    store(e, b, &c);
}

/// Re:CoM's state-change callback, as written in each revenge boss class's state-machine vtable
/// (`sites::REVENGE_FSM_SLOTS`; `fsm` = the boss + 0x58). Re:CoM's own `0x1401A3B80` runs first.
/// When a flinch / blow-away that hasn't decided yet is left for anything but a new reaction (its
/// own end, the boss's think a quarter in, a stop…), the revenge check runs here, at its end (owner,
/// 2026-11-02: KH2's frame 20, or the end if sooner). Runs on the bosses' state changes only.
pub unsafe extern "win64" fn state_change(fsm: usize) {
    func::<StepFn>(rv::FSM_CHANGE)(fsm);
    if !active() {
        return;
    }
    let e = fsm - rv::FSM_AT;
    let Some(b) = boss_of(e) else { return };
    // The room's colour before the new state's begin runs (`crate::dim`).
    if let (Some(now), Ok(mut g)) = (env_slot(dim::BOSS_SLOT), DIM_SNAP.lock()) {
        *g = (e, now);
    }
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let old: usize = rd(e + EN_STATE);
    let next: usize = rd(e + EN_NEXT);
    let reaction = |s: usize| s == crate::battle::state_address(e, FLINCH_STATE) || s == crate::battle::state_address(e, BLOW_STATE);
    if !rv::reaction_left(reaction(old), reaction(next)) || rd::<i32>(e + EN_HP) <= 0 {
        return;
    }
    let mut c = counter(e, b);
    if c.due(b.check_frame(), true) {
        decide(e, b, &mut c, true);
    }
    store(e, b, &c);
}

/// Re:CoM's flinch step (state 11, `0x140184100`).
pub unsafe extern "win64" fn flinch_step(e: usize) {
    step(e, orig(&ORIG_FLINCH))
}

/// Re:CoM's blow-away step (state 12, `0x140182E50`).
pub unsafe extern "win64" fn blow_step(e: usize) {
    step(e, orig(&ORIG_BLOW))
}

/// Whether `e` is a revenge boss body in (or about to begin) the attack its revenge started: Re:CoM's
/// own rule for an attacking enemy then decides hits on it (`crate::revenge::holds`).
pub(super) unsafe fn untouchable(e: usize) -> bool {
    if !active() {
        return false;
    }
    let Some(b) = boss_of(e) else { return normal_window(e) };
    tail_ok(e, b) && rv::holds(rd(e + b.size as usize + tail::FLAGS), rd(e + EN_FLAGS), rd(e + EN_HP))
}

/// A Guard reaction (decision 27) the window skipped (`super::guard`).
pub(super) unsafe fn guard_skipped(e: usize) {
    let Some(b) = boss_of(e) else {
        // A Wizard / Aquatank in its window.
        if let Ok(mut g) = NORMAL_WINDOWS.lock() {
            if let Some(w) = g.get_mut(e, super::battle::battle_number()) {
                w.ignored.add(IgnoredKind::Guard);
            }
        }
        return;
    };
    let t = e + b.size as usize;
    let mut ig = Ignored::from_bytes(rd(t + tail::IGNORED));
    ig.add(IgnoredKind::Guard);
    wr(t + tail::IGNORED, ig.to_bytes());
}

/// Re:CoM's "hurt bodies off" at an attack state's begin (`super::attack::hurt_off`, at the KH1
/// setting, which otherwise skips it: decision 26): true when the state beginning is the boss's
/// revenge move, so Re:CoM's own call runs and the window opens: the state a revenge start marked,
/// or (owner, 2026-11-02: "untouchable whenever played") the same state chosen by the boss itself.
/// A pending start that another attack state replaced is dropped.
pub(super) unsafe fn begin_untouchable(e: usize) -> bool {
    if !active() || e == 0 {
        return false;
    }
    // A step opening a dim window now (its `+0x438` call).
    if DIM_OPENING.load(Ordering::Relaxed) == e {
        return true;
    }
    let Some(b) = boss_of(e) else { return normal_begin(e) };
    if live() && revenge_begin(e, b) {
        return true;
    }
    dim_begin(e, b)
}

/// The revenge-move part of [`begin_untouchable`].
unsafe fn revenge_begin(e: usize, b: &Boss) -> bool {
    let Some((move_state, what)) = b.attack else { return false };
    let current: usize = rd(e + EN_STATE);
    let is_move = current == crate::battle::state_address(e, move_state);
    if !tail_ok(e, b) {
        if !is_move {
            return false;
        }
        // The boss's first use, before any hit set its tail up for this battle.
        setup(e, b);
    }
    let t = e + b.size as usize;
    let flags: u32 = rd(t + tail::FLAGS);
    let Some(origin) = rv::window_opens(flags, current, crate::battle::state_address(e, move_state)) else {
        if flags & flag::PENDING != 0 {
            wr(t + tail::FLAGS, flags & !flag::PENDING);
            let state: i32 = rd(t + tail::STATE);
            line(|| format!("revenge: {} {e:#x}: another attack state began (+{:#x}) before its revenge state {state}: not untouchable", b.name, current.wrapping_sub(e)));
        }
        return false;
    };
    let own = if origin == rv::Origin::Own { flag::OWN } else { 0 };
    wr(t + tail::FLAGS, (flags & !(flag::PENDING | flag::OWN | flag::DIM)) | flag::UNTOUCHABLE | own);
    wr(t + tail::STATE, move_state);
    wr(t + tail::IGNORED, [0u8; 4]);
    wr(t + tail::START, now() as f32);
    match origin {
        rv::Origin::Revenge => line(|| format!("revenge: {} untouchable (state {move_state}: {what}) {e:#x}: Re:CoM's rule for an attacking enemy (hurt bodies off, its hit handler drops hits while +0x260 bit 0 is set)", b.name)),
        rv::Origin::Own => line(|| format!("revenge: {} untouchable (own choice, state {move_state}: {what}) {e:#x}: its revenge move, played by itself; Re:CoM's rule for an attacking enemy", b.name)),
    }
    true
}

/// Re:CoM's attack-state end (`0x14017DA90`, vtable `+0x440` of every enemy; Marluxia 2's own
/// `0x140400460` calls it): clears bits 0 / 2 and turns the hurt bodies back on. Closes the window.
pub unsafe extern "win64" fn attack_end(e: usize) {
    if INSTALLED.load(Ordering::Relaxed) {
        if let Some(b) = boss_of(e) {
            close_window(e, b, "");
        } else if NORMAL_OPEN.load(Ordering::Relaxed) != 0 {
            close_normal(e);
        }
    }
    orig::<StepFn>(&ORIG_ATTACK_END)(e)
}

/// Closes the window (if open) and logs it; true when it was open.
unsafe fn close_window(e: usize, b: &Boss, why: &str) -> bool {
    if !tail_ok(e, b) {
        return false;
    }
    let t = e + b.size as usize;
    let flags: u32 = rd(t + tail::FLAGS);
    if flags & (flag::UNTOUCHABLE | flag::PENDING | flag::OWN | flag::DIM) != 0 {
        wr(t + tail::FLAGS, flags & !(flag::UNTOUCHABLE | flag::PENDING | flag::OWN | flag::DIM));
    }
    if flags & flag::UNTOUCHABLE == 0 {
        return false;
    }
    let state: i32 = rd(t + tail::STATE);
    let ig = Ignored::from_bytes(rd(t + tail::IGNORED));
    let frames = now() as f32 - rd::<f32>(t + tail::START);
    let hp: i32 = rd(e + EN_HP);
    let own = if flags & flag::OWN != 0 { "own choice, " } else { "" };
    let dimmed = flags & flag::DIM != 0;
    let say = |f: &dyn Fn() -> String| if dimmed { dim_line(f) } else { line(f) };
    say(&|| {
        format!(
            "{}: {} untouchable ends ({own}state {state}{why}) {e:#x} after {frames:.0} frames: ignored {} hits ({} KH1 swings, {} KH1 spells, {} other), {} Guard reactions skipped; HP {hp}{}",
            if dimmed { "dim" } else { "revenge" },
            b.name,
            ig.hits(),
            ig.swings,
            ig.spells,
            ig.other,
            ig.guards,
            if hp <= 0 { " (dead)" } else { "" }
        )
    });
    true
}

/// Slot `i` of Re:CoM's scene colour table (`crate::dim`; the table's global and layout are the
/// setter's, byte-checked at start: `sites::DIM_CHECKED`), or `None` while there is no table.
unsafe fn env_slot(i: usize) -> Option<[u8; SLOT_SIZE]> {
    if i >= dim::SLOTS {
        return None;
    }
    let table: usize = rd(base() + dim::ENV_TABLE as usize);
    (table != 0).then(|| rd::<[u8; SLOT_SIZE]>(table + i * SLOT_SIZE))
}

/// Opens a dim window on the boss (its tail flags; `crate::revenge::holds` then makes it untouchable
/// until Re:CoM's state end, [`attack_end`]) and logs it.
unsafe fn open_dim(e: usize, b: &Boss, state: i32, cause: dim::Cause, level: f32) {
    if !tail_ok(e, b) {
        setup(e, b);
    }
    let t = e + b.size as usize;
    let flags: u32 = rd(t + tail::FLAGS);
    wr(t + tail::FLAGS, (flags & !(flag::PENDING | flag::OWN)) | flag::UNTOUCHABLE | flag::DIM);
    wr(t + tail::STATE, state);
    wr(t + tail::IGNORED, [0u8; 4]);
    wr(t + tail::START, now() as f32);
    let how = match cause {
        dim::Cause::Begin => "its state begin dims the room",
        dim::Cause::Step => "the room dims partway through its state",
        dim::Cause::OwnSlot => "the room is dim (its own colour slot 1) as the state begins",
    };
    dim_line(|| format!("dim: {} untouchable (state {state}: {}) {e:#x}: {how} (Re:CoM's scene colour {level:.2}); hurt bodies off, its hit handler drops hits until the state ends", b.name, dim::what(b.id, state)));
}

/// The attack state (2..=10) the boss is in, by its current state object (`+0x60`).
unsafe fn attack_state(e: usize) -> Option<i32> {
    let current: usize = rd(e + EN_STATE);
    (2..=10).find(|&k| current == crate::battle::state_address(e, k))
}

/// The dim part of [`begin_untouchable`]: the state's begin (Re:CoM's class begin ran just before
/// this "hurt bodies off") asked for the dark room in slot 0 (compared with the snapshot of the state
/// change), or the boss's own slot is dark (Marluxia 3).
unsafe fn dim_begin(e: usize, b: &Boss) -> bool {
    let snap = DIM_SNAP.lock().ok().and_then(|mut g| {
        let (who, bytes) = *g;
        g.0 = 0;
        (who == e).then_some(bytes)
    });
    let Some(now) = env_slot(dim::BOSS_SLOT) else { return false };
    let own = dim::own_slot(b.id).and_then(|i| env_slot(i));
    let Some(cause) = dim::at_begin(snap.as_ref(), &now, own.as_ref()) else { return false };
    if rd::<i32>(e + EN_HP) <= 0 {
        return false;
    }
    let Some(state) = attack_state(e) else { return false };
    let level = dim::Slot::parse(if cause == dim::Cause::OwnSlot { own.as_ref().unwrap_or(&now) } else { &now }).level();
    open_dim(e, b, state, cause, level);
    true
}

/// A dimming step ([`EXES`]`[i]`, as written in the class's vtable: `sites::DIM_EXE_SLOTS`): Re:CoM's
/// exe always runs; when it asked for the dark room in its slot, the window opens and Re:CoM's own
/// "hurt bodies off" (`+0x438`) runs now, as at an attack begin. Runs every frame only while the boss
/// is in that state, and until its window is open: two 64-byte reads and a compare.
unsafe fn dim_exe(e: usize, i: usize) {
    let x = &EXES[i];
    let exe = func::<StepFn>(x.exe);
    let watch = active() && boss_of(e).is_some_and(|b| b.id == x.boss_id) && !untouchable(e);
    let before = if watch { env_slot(x.slot) } else { None };
    exe(e);
    let (Some(before), Some(b)) = (before, boss_of(e)) else { return };
    let Some(after) = env_slot(x.slot) else { return };
    if !dim::requested(&before, &after) {
        return;
    }
    // Not when the step also ended the move (a next state queued) or the boss is down.
    if rd::<usize>(e + EN_NEXT) != 0 || rd::<i32>(e + EN_HP) <= 0 || attack_state(e) != Some(x.state) {
        dim_line(|| format!("dim: {} {e:#x} state {}: the room dims, but its state is ending or it is down: not untouchable", b.name, x.state));
        return;
    }
    open_dim(e, b, x.state, dim::Cause::Step, dim::Slot::parse(&after).level());
    let vt: usize = rd(e);
    let hurt_off: StepFn = std::mem::transmute_copy(&rd::<usize>(vt + EN_HURT_OFF_SLOT));
    DIM_OPENING.store(e, Ordering::Relaxed);
    hurt_off(e);
    DIM_OPENING.store(0, Ordering::Relaxed);
}

pub unsafe extern "win64" fn dim_exe_0(e: usize) {
    dim_exe(e, 0)
}
pub unsafe extern "win64" fn dim_exe_1(e: usize) {
    dim_exe(e, 1)
}
pub unsafe extern "win64" fn dim_exe_2(e: usize) {
    dim_exe(e, 2)
}
pub unsafe extern "win64" fn dim_exe_3(e: usize) {
    dim_exe(e, 3)
}
const DIM_EXE_FNS: [unsafe extern "win64" fn(usize); 4] = [dim_exe_0, dim_exe_1, dim_exe_2, dim_exe_3];

/// The normal dimming class of `e` (`crate::dim::NORMAL`): Re:CoM's own vtable, so only while it
/// runs Re:CoM's AI (a KH1-AI enemy has our vtable copy).
unsafe fn normal_of(e: usize) -> Option<(usize, &'static dim::Normal)> {
    if e == 0 || !NORMAL_OK.load(Ordering::Relaxed) {
        return None;
    }
    dim::normal_by_vtable(vt_rva(e)).map(|i| (i, &dim::NORMAL[i]))
}

/// Whether `e` is a Wizard / Aquatank in the dimming state its window opened for, alive, in this
/// battle. One load while no window is open.
unsafe fn normal_window(e: usize) -> bool {
    if NORMAL_OPEN.load(Ordering::Relaxed) == 0 || !active() {
        return false;
    }
    let Some((_, n)) = normal_of(e) else { return false };
    if rd::<usize>(e + EN_STATE) != crate::battle::state_address(e, n.state) || rd::<i32>(e + EN_HP) <= 0 {
        return false;
    }
    NORMAL_WINDOWS.lock().is_ok_and(|g| g.holds(e, super::battle::battle_number()))
}

/// The dim part of [`begin_untouchable`] for the Wizard / Aquatank: the state beginning is the
/// dimming one and its begin (run just before this "hurt bodies off": the begin wrapper) left a fresh
/// dark request in slot 0: the window opens and Re:CoM's own call runs.
unsafe fn normal_begin(e: usize) -> bool {
    let Some((class, n)) = normal_of(e) else { return false };
    if !active() || rd::<i32>(e + EN_HP) <= 0 || rd::<usize>(e + EN_STATE) != crate::battle::state_address(e, n.state) {
        return false;
    }
    let Some(now) = env_slot(dim::BOSS_SLOT) else { return false };
    if !dim::fresh_dark(&now) {
        dim_line(|| format!("dim: {} {e:#x} state {}: the room didn't dim at its begin (slot 0 holds no fresh dark request): hittable", n.name, n.state));
        return false;
    }
    let opened = NORMAL_WINDOWS.lock().is_ok_and(|mut g| {
        let ok = g.open(e, super::battle::battle_number(), class, now_frame());
        NORMAL_OPEN.store(g.len(), Ordering::Relaxed);
        ok
    });
    if !opened {
        dim_line(|| format!("dim: {} {e:#x}: too many open windows ({}): hittable", n.name, dim::Windows::CAP));
        return false;
    }
    let level = dim::Slot::parse(&now).level();
    dim_line(|| format!("dim: {} untouchable (state {}: {}) {e:#x}: its state begin dims the room (Re:CoM's scene colour {level:.2}); Re:CoM's hurt bodies off, its hit handler drops hits until the state ends", n.name, n.state, n.what));
    true
}

fn now_frame() -> f32 {
    now() as f32
}

/// The hit handler's call for a Wizard / Aquatank in its window: Re:CoM's rule for an attacking
/// enemy (`super::attack::untouchable_hit`), the ignored hit counted in its window.
unsafe fn normal_hit(e: usize) -> u64 {
    let (r, kind) = super::attack::untouchable_hit(e);
    if let Some(kind) = kind {
        if let Ok(mut g) = NORMAL_WINDOWS.lock() {
            if let Some(w) = g.get_mut(e, super::battle::battle_number()) {
                w.ignored.add(kind);
            }
        }
    }
    if r != 0 {
        dim_line(|| format!("dim: {e:#x} took a hit in its untouchable move: Re:CoM's handler applied it (flags {:#x})", rd::<u64>(e + EN_FLAGS)));
    }
    r
}

/// Re:CoM's state end for a Wizard / Aquatank: its window (if open) closes and is logged.
unsafe fn close_normal(e: usize) {
    let w = NORMAL_WINDOWS.lock().ok().and_then(|mut g| {
        let w = g.close(e);
        NORMAL_OPEN.store(g.len(), Ordering::Relaxed);
        w
    });
    let Some(w) = w else { return };
    let Some(n) = dim::NORMAL.get(w.class) else { return };
    let ig = w.ignored;
    let frames = now_frame() - w.start;
    let hp: i32 = rd(e + EN_HP);
    let stale = if w.battle != super::battle::battle_number() { " (another battle's window)" } else { "" };
    dim_line(|| {
        format!(
            "dim: {} untouchable ends (state {}) {e:#x} after {frames:.0} frames{stale}: ignored {} hits ({} KH1 swings, {} KH1 spells, {} other), {} Guard reactions skipped; HP {hp}{}",
            n.name,
            n.state,
            ig.hits(),
            ig.swings,
            ig.spells,
            ig.other,
            ig.guards,
            if hp <= 0 { " (dead)" } else { "" }
        )
    });
}

/// Checks the Wizard's and the Aquatank's classes and slots (`sites::DIM_NORMAL_PTRS`): their dim
/// windows only when all hold (the bosses' don't depend on it). Not called since 0.168.0 (kept for
/// a switch back).
#[allow(dead_code)]
unsafe fn install_normal(bs: usize) {
    for n in &dim::NORMAL {
        let want = format!(".?AV{}@@", n.class);
        if super::stats::rtti_name(bs, bs + n.vtable as usize).as_deref() != Some(want.as_str()) {
            log(&format!("dim: {} vtable {:#x} is not {want}: the Wizard / Aquatank stay hittable", n.name, 0x1_4000_0000u64 + n.vtable as u64));
            return;
        }
    }
    if let Some(p) = DIM_NORMAL_PTRS.iter().find(|p| rd::<usize>(bs + p.rva as usize) != bs + p.target as usize) {
        log(&format!("dim: {} holds another pointer: the Wizard / Aquatank stay hittable", p.name));
        return;
    }
    NORMAL_OK.store(true, Ordering::Release);
    log("dim: the Wizard (state 4) and the Aquatank (state 3) untouchable during their moves that dim the room (on Re:CoM's AI)");
}

/// A wind-up-only revenge state's exe (Hades' state 5, `crate::revenge::HADES_WINDUP`), as written in
/// the class's vtable (`sites::REVENGE_WINDUP_SLOTS`). Re:CoM's exe runs first; when it has moved its
/// sub-step off 0 (the wind-up motion ended, the spin and its timer started) the window closes and
/// Re:CoM's own "bodies on" runs, so the rest of the state is an ordinary attack at the KH1 setting
/// (decisions 14 / 26; his fire-up keeps Re:CoM's no-flinch bit 19). Runs only while the class is
/// in that state.
pub unsafe extern "win64" fn windup_exe(e: usize) {
    // Only Hades' table holds this slot; Re:CoM's exe always runs.
    let b = boss_of(e).filter(|b| b.windup.is_some());
    let w = b.and_then(|b| b.windup).unwrap_or(rv::HADES_WINDUP);
    func::<StepFn>(w.exe)(e);
    let Some(b) = b else { return };
    if !INSTALLED.load(Ordering::Relaxed) || !rv::windup_over(rd(e + w.step)) {
        return;
    }
    let open = tail_ok(e, b) && rd::<u32>(e + b.size as usize + tail::FLAGS) & flag::UNTOUCHABLE != 0;
    if open && close_window(e, b, &format!(", {}", w.what)) {
        func::<BodiesFn>(BODIES_ON)(e, -1);
    }
}

/// Checks the classes and their state steps, hooks the two steps and raises the bosses'
/// allocation sizes, then writes the state-machine, wind-up and dimming-step slots: all or nothing.
pub unsafe fn install(bs: usize) -> bool {
    for b in rv::BOSSES {
        let want = format!(".?AV{}@@", b.rtti);
        match super::stats::rtti_name(bs, bs + b.vtable as usize) {
            Some(n) if n == want => {}
            n => {
                log(&format!("revenge: {} vtable {:#x} is {n:?}, not {want}: not installed", b.name, 0x1_4000_0000u64 + b.vtable as u64));
                return false;
            }
        }
    }
    for p in rv::PARTS {
        let want = format!(".?AV{}@@", p.rtti);
        if super::stats::rtti_name(bs, bs + p.vtable as usize).as_deref() != Some(want.as_str()) {
            log(&format!("revenge: part vtable {:#x} is not {want}: not installed", 0x1_4000_0000u64 + p.vtable as u64));
            return false;
        }
    }
    if let Some(p) = REVENGE_PTRS.iter().find(|p| rd::<usize>(bs + p.rva as usize) != bs + p.target as usize) {
        log(&format!("revenge: {} is not the function we hook or expect: not installed", p.name));
        return false;
    }
    // The attack-state end's bytes after the 8 our hook moves (`mov edx,-1; jmp 0x1401A2DB0`).
    if let Some(s) = REVENGE_CHECKED.iter().find(|s| std::slice::from_raw_parts((bs + s.rva as usize) as *const u8, s.expect.len()) != s.expect) {
        log(&format!("revenge: {} holds other bytes: not installed", s.name));
        return false;
    }
    let site = |name: &str| REVENGE_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [
        super::hook::Hook { site: site("enemy flinch step (0x140184100)"), ours: flinch_step as *const () as usize, original: &ORIG_FLINCH },
        super::hook::Hook { site: site("enemy blow-away step (0x140182E50)"), ours: blow_step as *const () as usize, original: &ORIG_BLOW },
        super::hook::Hook { site: site("enemy attack-state end (0x14017DA90)"), ours: attack_end as *const () as usize, original: &ORIG_ATTACK_END },
    ];
    if !super::hook::install_all(bs, &hooks) {
        return false;
    }
    let mut done: Vec<&Boss> = Vec::new();
    for b in rv::BOSSES {
        if super::hook::patch(bs + b.alloc as usize, &rv::alloc_bytes(b.size), &rv::alloc_bytes_ours(b.size), b.rtti) {
            done.push(b);
        } else {
            for d in done {
                super::hook::patch(bs + d.alloc as usize, &rv::alloc_bytes_ours(d.size), &rv::alloc_bytes(d.size), d.rtti);
            }
            log("revenge: an allocation size held other bytes; sizes left as Re:CoM's, revenge off (the step hooks pass through)");
            return false;
        }
    }
    // The state-change callback of each class's state machine and Hades' state-5 exe: pointer
    // writes in those classes' own tables (Re:CoM's function checked as the old value).
    // The dimming moves' steps (`crate::dim::EXES`, same order as `DIM_EXE_SLOTS`) go with them.
    let ours: Vec<(&PtrSite, usize)> = REVENGE_FSM_SLOTS
        .iter()
        .map(|p| (p, state_change as *const () as usize))
        .chain(REVENGE_WINDUP_SLOTS.iter().map(|p| (p, windup_exe as *const () as usize)))
        .chain(DIM_EXE_SLOTS.iter().zip(DIM_EXE_FNS).map(|(p, f)| (p, f as usize)))
        .collect();
    let mut written: Vec<(&PtrSite, usize)> = Vec::new();
    for &(p, f) in &ours {
        let old = ((bs + p.target as usize) as u64).to_le_bytes();
        if super::hook::patch(bs + p.rva as usize, &old, &(f as u64).to_le_bytes(), p.name) {
            written.push((p, f));
            continue;
        }
        for (q, g) in written.iter().rev() {
            super::hook::patch(bs + q.rva as usize, &(*g as u64).to_le_bytes(), &((bs + q.target as usize) as u64).to_le_bytes(), q.name);
        }
        for b in rv::BOSSES {
            super::hook::patch(bs + b.alloc as usize, &rv::alloc_bytes_ours(b.size), &rv::alloc_bytes(b.size), b.rtti);
        }
        log(&format!("revenge: {} held another pointer; revenge off (the step hooks pass through)", p.name));
        return false;
    }
    // 0.168.0 (fork owner, 2026-11-23: "a jogabilidade quando estiver no KH1 deve usar as mesmas
    // regras, com exceção a bosses de fato, um wizard não é um boss"): the Wizard's / Aquatank's
    // dimming moves are ordinary attacks again (hittable, interrupted by hits, as their other
    // casts); only the bosses' dimming moves stay untouchable.
    log("dim: the Wizard / Aquatank stay hittable in their dimming moves (only bosses' are untouchable)");
    INSTALLED.store(true, Ordering::Release);
    true
}
