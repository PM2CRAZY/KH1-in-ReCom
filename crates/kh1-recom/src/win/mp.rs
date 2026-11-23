//! KH1's MP at the KH1 setting (M7 stage 7.1, decision 34; `crate::mp`, `docs/recom-m7-mp.md`).
//!
//! No per-frame work in steady state, and one hook of its own (the save point update, below):
//!
//! - **Store:** our block in Re:CoM's own save (`save + 0x2675 ..= 0x2684`, `crate::mp::SavedMp`),
//!   read and written only when something happens. Re:CoM saves, checksums, loads, snapshots and
//!   zeroes it with the rest of its save block.
//! - **Charge from hits:** the existing enemy-hit wrapper (`0x14017F7E0`, `super::attack`) calls
//!   [`on_kh1_hit`] for each KH1 hit an enemy takes (one per KH1 damage record, as KH1's
//!   `0x1402BF330`).
//! - **Refill:** KH1 makes at most one point per frame (`0x1402A4CB0` in the actor pre-step).
//!   After a change that leaves a point to make, [`frame`] (from the existing request-bits wrapper
//!   `0x1401F8150`, which Re:CoM's Field / Battle exes call each frame after their early-out)
//!   runs one step per frame; otherwise it is one atomic load.
//! - **Level-ups** are applied when the block is next read (`crate::mp::live`).
//! - **The MP bar** (stage 7.2, `super::mp_bar`) is told every read and write's result; it
//!   rescrubs only when that differs from what it shows.
//!
//! The API for later stages: [`current`], [`can_pay`], [`pay`], [`gain_charge`],
//! [`restore_full`]. All are no-ops (None / false) at the Re:CoM setting, in Riku's story, or
//! with no save block.

use super::movement::{self as mv, rd, wr};
use super::{base, log, va};
use crate::mp::{self as m, Live, Mp, SavedMp, BLOCK_FIRST, BLOCK_LEN};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

/// A point is waiting to be made by KH1's per-frame refill.
static REFILL_PENDING: AtomicBool = AtomicBool::new(false);
/// Log lines left this session (gains, payments, points, first read).
static LOG_LEFT: AtomicU32 = AtomicU32::new(80);
static FIRST_READ_LOGGED: AtomicBool = AtomicBool::new(false);
/// The trace's MP events since its last row (capped).
static EVENTS: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn note(line: impl FnOnce() -> String) {
    if LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("mp: {}", line()));
    }
}

fn event(e: String) {
    if let Ok(mut g) = EVENTS.lock() {
        if g.len() < 16 {
            g.push(e);
        }
    }
}

/// Re:CoM's save block, when it is there.
fn save() -> Option<usize> {
    // SAFETY: a static pointer of the image.
    let s: usize = unsafe { rd(va(base(), crate::keyblades::SAVE_PTR)) };
    (s != 0).then_some(s)
}

/// KH1 MP is live for Sora: Combat = KH1, Sora's story, a save block. Everything of Sora's that
/// rides on it (AP Up, abilities, items, the shop, notices, the command menu) reads this.
pub fn active() -> bool {
    mv::kh1_on() && !super::stats::riku_story() && save().is_some()
}

/// Riku's KH1 MP (`riku_kh1_movement`, `crate::riku`): Combat = KH1 in Riku's story, in the same
/// block of his story's save. Only the MP itself (pay, charge, refill, the bar) reads it.
pub fn riku_live() -> bool {
    mv::kh1_on() && mv::riku_on() && super::stats::riku_story() && save().is_some()
}

/// The MP block is read and written: Sora's ([`active`]) or Riku's ([`riku_live`]).
pub fn live() -> bool {
    active() || riku_live()
}

/// The equipped Keyblade's KH1 max-MP bonus (KH1 weapon `+0x38`, the stats file's `magic`). The
/// six KH2 Keyblades use their KH2 Magic stat as the bonus (owner, 2026-10-31, decision 35). The
/// Kingdom Key: 0.
pub fn keyblade_bonus() -> i32 {
    super::stats::keyblade(super::stats::equipped_card()).map_or(0, |k| k.magic)
}

/// Reads the block, syncs it (level-ups, Keyblade), runs `f`, writes back what KH1 would have
/// written. None when MP isn't live.
fn with_state<R>(f: impl FnOnce(&mut Mp) -> R) -> Option<R> {
    if !live() {
        REFILL_PENDING.store(false, Ordering::Relaxed);
        return None;
    }
    let s = save()?;
    let at = s + BLOCK_FIRST;
    // SAFETY: inside the 0x2AD4-byte save block (BLOCK_LAST < 0x2818).
    let mut block: [u8; BLOCK_LEN] = unsafe { rd(at) };
    let saved = SavedMp::read(&block);
    let level = super::stats::sora_level();
    // Riku has no Keyblade bonus (Soul Eater; `crate::riku`).
    let bonus = if riku_live() { 0 } else { keyblade_bonus() };
    let mut l: Live = m::live(&saved, level, bonus);
    if !FIRST_READ_LOGGED.swap(true, Ordering::Relaxed) {
        let (mp, max, c) = l.mp.view();
        log(&format!(
            "mp: save+{BLOCK_FIRST:#x}: layout v{} ({}), Sora level {level}, Keyblade MP bonus {bonus:+}: MP {mp}/{max}, charge {c}{}",
            saved.version,
            if l.fresh { "never written: full MP" } else { "written" },
            if l.level_ups > 0 { format!(", {} max-MP level-up(s) since level {}", l.level_ups, saved.level) } else { String::new() }
        ));
    }
    let r = f(&mut l.mp);
    if let Some(new) = m::stored(&saved, &l) {
        new.write(&mut block);
        // SAFETY: as above.
        unsafe { wr(at, block) };
    }
    REFILL_PENDING.store(l.mp.refill_ready(), Ordering::Relaxed);
    // The MP bar (stage 7.2) shows what was just read or written: only a compare when unchanged.
    super::mp_bar::sync(l.mp.view());
    // The KH1 HUD's MP ring (decision 39, H2): likewise.
    super::kh1_hud::mp_seen(l.mp.view());
    // The command menu (stage 7.3) greys spells by it: one atomic store.
    super::command_menu::mp_seen(l.mp.view());
    Some(r)
}

/// Sora's KH1 spell levels from the block (`crate::mp::off::SPELLS`; stage 7.11 writes them),
/// Fire … Aero, 0 = not learned, and his three shortcut slots (`crate::mp::off::SHORTCUTS`, stage
/// 7.12: 0 = none, else spell + 1), from one read of the block. All zero when MP isn't live. A read, but
/// for the one-time sync of a save never synced or whose shortcuts were never set up
/// (`super::unlocks::sync_first`).
pub fn spell_state() -> ([u8; 7], [u8; 3]) {
    if !active() {
        return ([0; 7], [0; 3]);
    }
    let Some(s) = save() else { return ([0; 7], [0; 3]) };
    // SAFETY: inside the 0x2AD4-byte save block (BLOCK_LAST < 0x2818).
    let mut block: [u8; BLOCK_LEN] = unsafe { rd(s + BLOCK_FIRST) };
    // Stage 7.11: a save never synced from Re:CoM's card collection gets its levels now, once;
    // stage 7.12: likewise its shortcut slots.
    let both = crate::unlocks::SYNCED | crate::unlocks::SHORTCUTS_SET;
    if block[m::off::SPARE] & both != both && super::unlocks::sync_first() {
        // SAFETY: as above.
        block = unsafe { rd(s + BLOCK_FIRST) };
    }
    let mut out = [0u8; 7];
    out.copy_from_slice(&block[m::off::SPELLS..m::off::SPELLS + m::off::SPELL_COUNT]);
    let mut slots = [0u8; 3];
    slots.copy_from_slice(&block[m::off::SHORTCUTS..m::off::SHORTCUTS + m::off::SHORTCUT_COUNT]);
    (out.map(|l| if l <= 3 { l } else { 0 }), slots)
}

/// Stage 7.13 (Customize, KH1's `0x1402857C0`): writes Sora's three shortcut slots into the block
/// (`crate::mp::off::SHORTCUTS`; 0 = none, else spell + 1). The command menu reads them every
/// frame (`spell_state`), so the L1 list shows the change at once. The block is read first, so a
/// save never synced gets its one-time sync (and slot set-up) before our write. False when MP isn't
/// live.
pub fn set_shortcuts(slots: [u8; 3]) -> bool {
    if !active() {
        return false;
    }
    let _ = spell_state();
    let Some(s) = save() else { return false };
    // SAFETY: inside the 0x2AD4-byte save block (BLOCK_LAST < 0x2818).
    unsafe { wr(s + BLOCK_FIRST + m::off::SHORTCUTS, slots) };
    true
}

/// Riku's three shortcut slots (`crate::riku`), the same bytes of his story's save block: his
/// sleights' ids, 0 = none. None when his MP isn't live. No spell sync (he has no spells).
pub fn riku_shortcuts() -> Option<[u8; 3]> {
    if !riku_live() {
        return None;
    }
    let s = save()?;
    // SAFETY: inside the 0x2AD4-byte save block (BLOCK_LAST < 0x2818).
    Some(unsafe { rd(s + BLOCK_FIRST + m::off::SHORTCUTS) })
}

/// Writes Riku's three shortcut slots, and marks them set up (the spare byte's
/// `crate::unlocks::SHORTCUTS_SET`, in his story's block: Sora's never shares it), so slots he
/// emptied on his Customize page are not filled again. False when his MP isn't live.
pub fn set_riku_shortcuts(slots: [u8; 3]) -> bool {
    let Some(s) = save().filter(|_| riku_live()) else { return false };
    // SAFETY: as above.
    unsafe {
        wr(s + BLOCK_FIRST + m::off::SHORTCUTS, slots);
        let spare = s + BLOCK_FIRST + m::off::SPARE;
        wr(spare, rd::<u8>(spare) | crate::unlocks::SHORTCUTS_SET);
    }
    true
}

/// Riku's slots were set up once (filled with his first sleights, or set on his Customize page).
pub fn riku_shortcuts_set() -> bool {
    let Some(s) = save().filter(|_| riku_live()) else { return false };
    // SAFETY: as above.
    unsafe { rd::<u8>(s + BLOCK_FIRST + m::off::SPARE) & crate::unlocks::SHORTCUTS_SET != 0 }
}

/// (MP, max MP, charge), or None when MP isn't live.
pub fn current() -> Option<(i32, i32, u16)> {
    with_state(|mp| mp.view())
}

/// KH1's MP test (`0x1402A4C10`), cost in KH1's encoding (`crate::mp::cost`). The cast's usable
/// test (7.4, `super::magic::can_cast`); the menu tests its cached view (`crate::mp::Mp`).
pub fn can_pay(cost: i32) -> bool {
    with_state(|mp| mp.can_pay(cost)).unwrap_or(false)
}

/// KH1's payment (`0x1402A5B60`). False when MP isn't live. Like KH1 it doesn't test
/// [`can_pay`] itself. The cast pays at its release frame (7.4, `super::magic::release`).
pub fn pay(cost: i32, why: &str) -> bool {
    let r = with_state(|mp| {
        let before = mp.view();
        mp.pay(cost);
        (before, mp.view())
    });
    let Some((b, a)) = r else { return false };
    event(format!("pay{cost}"));
    note(|| format!("paid {cost} ({why}): MP {}/{} c{} -> {}/{} c{}", b.0, b.1, b.2, a.0, a.1, a.2));
    // Enemy Skills stage 5: Aquatank's / Barrel Spider's refill once a battle.
    super::enemy_skills_deck::after_pay(a);
    true
}

/// KH1's gauge add (`0x1402A4AE0`).
pub fn gain_charge(units: i32, why: &str) {
    let r = with_state(|mp| {
        let before = mp.view();
        mp.gain(units);
        (before, mp.view())
    });
    let Some((b, a)) = r else { return };
    event(format!("{units:+}"));
    note(|| format!("charge {units:+} ({why}): MP {}/{} c{} -> {}/{} c{}", b.0, b.1, b.2, a.0, a.1, a.2));
}

/// KH1's item MP (`0x1402CD6B0`, M8 stage 8a.1): below max MP, `0x1402A4A00(actor, n)`: MP + n,
/// clamped to max; the charge untouched (Elixirs pass 9999). True when MP rose.
pub fn add_points(n: i32, why: &str) -> bool {
    let r = with_state(|mp| {
        let before = mp.view();
        if mp.mp < mp.max {
            mp.add_points(n);
        }
        (before, mp.view())
    });
    let Some((b, a)) = r else { return false };
    if a == b {
        return false;
    }
    event(format!("+{}mp", a.0 - b.0));
    note(|| format!("points +{} ({why}): MP {}/{} c{} -> {}/{} c{}", a.0 - b.0, b.0, b.1, b.2, a.0, a.1, a.2));
    true
}

/// KH1's full restore (`0x1402A56F0`): for a later stage, once the owner picks Re:CoM's
/// equivalent (not wired in 7.1).
pub fn restore_full(why: &str) {
    if let Some((b, a)) = with_state(|mp| {
        let before = mp.view();
        mp.restore_full();
        (before, mp.view())
    }) {
        event("full".into());
        note(|| format!("restore ({why}): MP {}/{} c{} -> {}/{} c{}", b.0, b.1, b.2, a.0, a.1, a.2));
    }
}

/// A KH1 hit on `enemy` from Sora's KH1 row `kh1_row` (`0x1402BF330`): the row's attack data
/// `+0x2C` × KH1's level factor, with the enemy's KH1 twin's level (decision 21) or, without a
/// twin, the provisional `crate::mp::no_twin_level`.
pub fn on_kh1_hit(enemy: usize, kh1_row: Option<usize>) {
    if !live() {
        return;
    }
    let ms = super::attack::moveset();
    let Some(record) = kh1_row.and_then(|r| ms.row(r)).and_then(|rw| ms.attack(rw.attack)) else {
        note(|| format!("a KH1 hit on {:#x} with no KH1 row: no charge", enemy & 0x00FF_FFFF));
        return;
    };
    let sora = super::stats::sora_level();
    let twin = super::stats::twin_level(enemy);
    let level = twin.unwrap_or_else(|| m::no_twin_level(sora));
    // M12 step 3: MP Haste (KH1 ability 23, ability block `+0x184` bit 0) on the factor.
    let haste = super::abilities::equipped_live().is_some_and(crate::abilities::mp_haste);
    // Enemy Skills stage 5: Tornado Step (+0.75 on the factor, `crate::enemy_skills_deck`).
    let tornado = super::enemy_skills_deck::tornado();
    let Some(units) = crate::enemy_skills_deck::hit_units(record.mp_charge, level, sora, haste, tornado) else {
        event("+0lv0".into());
        note(|| format!("hit on {:#x}: enemy level {level} (twin), no charge (KH1: level 0)", enemy & 0x00FF_FFFF));
        return;
    };
    let why = format!(
        "row {:#x} +0x2C {}, enemy level {level}{}, Sora level {sora}",
        kh1_row.unwrap_or(0),
        record.mp_charge,
        if twin.is_some() { " (twin)" } else { " (no twin)" }
    ) + if haste { ", MP Haste +1.5" } else { "" }
        + if tornado { ", Tornado Step +0.75" } else { "" };
    gain_charge(units, &why);
}

/// Once per frame of Sora's update at the KH1 setting (the request-bits wrapper): KH1's refill
/// step (`0x1402A4CB0`) while a point is waiting; one atomic load otherwise.
pub fn frame() {
    if !REFILL_PENDING.load(Ordering::Relaxed) {
        return;
    }
    if let Some((true, a)) = with_state(|mp| (mp.refill_step(), mp.view())) {
        event(format!("pt{}", a.0));
        note(|| format!("refill: point {} made: MP {}/{} c{}", a.0, a.0, a.1, a.2));
    }
}

/// The F9 trace's MP token: `mp <MP>/<max> c<charge>` and the events since the last row.
/// Recording only.
pub fn trace_token() -> String {
    // A read writes nothing (`crate::mp::stored` is None without a change).
    let view = current();
    let ev = EVENTS.lock().map(|mut g| std::mem::take(&mut *g)).unwrap_or_default();
    m::trace_token(view, &ev)
}

/// Clears the trace's events (a new recording starts).
pub fn clear_events() {
    if let Ok(mut g) = EVENTS.lock() {
        g.clear();
    }
}

// ---- Decision 35: MP refills to full where Re:CoM refills Sora's HP ----
//
// Re:CoM sets HP = max HP (`*0x14087CBF8 = [1]`) in three places (Checked, decomp + exe):
// - `0x1401AC9D0` (a game-state step): when its fade task reports 6, outside rooms 0x4B0..0x4E7
//   and unless `0x140235F60()` sends it to `0x1401A9D20`, it fills HP and sets state `+0x580 = 8`;
// - `0x140237CE0`: when `0x14087B84C & 2`, after the retry snapshot `0x1401B1410(0)`, it calls
//   `0x1402374C0(0)` (the call at `0x140237F3A`) and fills HP;
// - the level-up screen `0x14010EF10`: the HP choice raises max HP by 15 and fills HP.
// Assumption: these are all of Re:CoM's full heals (no other writer of HP = max HP was found).

pub static ORIG_HEAL_STATE: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_LEVEL_UP: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_SAVE_POINT: AtomicUsize = AtomicUsize::new(0);
/// `CGimmickSavePoint`: Sora is on it (set by Re:CoM's update the frame he steps on, cleared when
/// he leaves; `0x1401C5180`). Checked (decomp).
const SAVE_POINT_ON: usize = 0x2C0;
/// KH1's save point frame counter (`0x1402ACE20` steps it 0 → 1 → 2 → 0 every frame,
/// `0x1402AD081..0x1402AD0A2`; the handler adds MP only at 0, `0x1402AD208`). Checked (decomp +
/// exe). One counter for the room's save points (KH1 keeps one a save point; a room has one).
static SAVE_POINT_FRAME: AtomicU32 = AtomicU32::new(0);
static SAVE_POINT_LOGGED: AtomicBool = AtomicBool::new(false);

/// `CGimmickSavePoint` update (`0x1401C5180`, each frame in a room with a save point): Re:CoM's
/// (its HP heal on stepping on, the Save prompt), then KH1's MP part of `0x1402AD0B0` while Sora
/// is on it (`+0x2C0`): every third frame, below max MP, 1 MP and the charge emptied
/// (`crate::mp::Mp::save_point_tick`): 20 MP a second at 60 fps. KH1 asks Sora to be in a ground
/// state (`+0x70` 0 or 0x18); Re:CoM's on-it test (within 0.8 m, under 0.2 m above it) stands for
/// that (Assumption). KH1's HP part (+2 HP a frame) and the party's refill are not added: Re:CoM
/// heals HP itself on stepping on, and the party is not KH1's yet.
pub unsafe extern "win64" fn save_point(this: usize) {
    let o: ThisFn = mv::orig(&ORIG_SAVE_POINT);
    o(this);
    let f = SAVE_POINT_FRAME.fetch_add(1, Ordering::Relaxed) % 3;
    if f != 0 || rd::<u8>(this + SAVE_POINT_ON) != 1 || !live() {
        return;
    }
    if let Some((b, a)) = with_state(|mp| {
        let before = mp.view();
        mp.save_point_tick();
        (before, mp.view())
    }) {
        if a != b {
            event("save point".into());
            if !SAVE_POINT_LOGGED.swap(true, Ordering::Relaxed) {
                log(&format!("mp: save point: KH1's refill, MP {}/{} c{} -> {}/{} c{} (1 a third frame)", b.0, b.1, b.2, a.0, a.1, a.2));
            }
        }
    }
}
/// Re:CoM's `0x1402374C0` (the callee of the retargeted call at `0x140237F3A`).
const CHECKPOINT_CALLEE: u32 = 0x0023_74C0;
/// The room id (`0x140233700` returns the i16 here).
const ROOM: u64 = 0x1_4087_B838;
const STATS_PTR: u64 = 0x1_4087_CBF8;
const HEAL_STATE: usize = 0x580;

type ThisFn = unsafe extern "win64" fn(usize);
type IntFn = unsafe extern "win64" fn(i32);

/// Sora's (HP, max HP) from Re:CoM's stat block.
fn hp() -> Option<(i32, i32)> {
    // SAFETY: a static pointer of the image, then the stat block it points at.
    unsafe {
        let p: usize = rd(va(base(), STATS_PTR));
        (p != 0).then(|| (rd::<i32>(p), rd::<i32>(p + 4)))
    }
}

/// Wrapper on `0x1401AC9D0`: after Re:CoM's step, the transition into state 8 on the HP-filling
/// path (outside rooms 0x4B0..0x4E7, HP now full).
pub unsafe extern "win64" fn heal_state(this: usize) {
    let o: ThisFn = mv::orig(&ORIG_HEAL_STATE);
    let before: i32 = rd(this + HEAL_STATE);
    o(this);
    if before == 8 || rd::<i32>(this + HEAL_STATE) != 8 || !active() {
        return;
    }
    let room: i16 = rd(va(base(), ROOM));
    if (room as u16).wrapping_sub(0x4B0) <= 0x37 {
        return;
    }
    if hp().is_some_and(|(h, m)| h >= m) {
        restore_full("Re:CoM's HP refill (0x1401AC9D0)");
    }
}

/// The retargeted call at `0x140237F3A`: Re:CoM's `0x1402374C0(a)`, then (Re:CoM fills HP right
/// after it) MP to full.
pub unsafe extern "win64" fn checkpoint_call(a: i32) {
    let f: IntFn = mv::func(CHECKPOINT_CALLEE);
    f(a);
    if active() {
        restore_full("Re:CoM's HP refill (0x140237CE0)");
    }
}

/// Wrapper on the level-up screen `0x14010EF10`: the HP choice (max HP raised during the call);
/// M12: AP Up in place of CP Up and the learned notices (`super::level_up`, `super::notices`).
pub unsafe extern "win64" fn level_up(this: usize) {
    let o: ThisFn = mv::orig(&ORIG_LEVEL_UP);
    let before = hp();
    let m12 = super::level_up::before(this);
    o(this);
    super::level_up::after(this, m12);
    if let (Some((_, m0)), Some((h, m1))) = (before, hp()) {
        if m1 > m0 && h >= m1 && active() {
            restore_full("Re:CoM's level-up HP choice (0x14010EF10)");
        }
    }
}

/// Retargets the call at `0x140237F3A` to [`checkpoint_call`] through a jump placed near it.
/// The bytes there and around it are checked first (`sites::MP_CHECKPOINT_CALL`).
pub unsafe fn install_checkpoint(bs: usize) -> bool {
    let s = &crate::sites::MP_CHECKPOINT_CALL;
    let at = bs + s.rva as usize;
    let mut jmp = vec![0xFF, 0x25, 0, 0, 0, 0];
    jmp.extend_from_slice(&(checkpoint_call as *const () as u64).to_le_bytes());
    let Some(thunk) = super::hook::place_near(at, &jmp) else { return false };
    let Some(call) = crate::hookplan::call_rel32(at as u64, thunk as u64) else { return false };
    super::hook::patch(at, s.expect, &call, s.name)
}
