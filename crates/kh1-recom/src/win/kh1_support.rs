//! KH1's Rod-list support abilities at Combat = KH1, Sora's story (M12 step 3; rules in
//! `crate::kh1_support`, which ones are equipped in `crate::abilities`, notes in
//! `docs/recom-m9-abilities.md` § M12 step 3).
//!
//! Nothing of ours per frame: each part runs inside a wrapper that already exists, and only when
//! something happens there (a hit reaching Sora in Re:CoM's player damage check `0x1401E6DE0`, a
//! KH1 swing's start or hit). One code patch: Second Chance's stub in that check's HP store.
//!
//! - **MP Rage** ([`after_damage`]): after Re:CoM's damage check took HP, KH1's charge for the HP
//!   lost (`0x14029BBD0`), through KH1's gauge add (`super::mp::gain_charge`, `0x1402A4AE0`).
//! - **Berserk** ([`swing_started`], [`berserk_bonus`], [`berserk_recom`]): KH1's +4 Strength on
//!   Strength-source records while Sora's HP is low, tested at the swing's start (KH1 builds the
//!   attack object then); at Damage = KH1 in KH1's damage, at Damage = Re:CoM as the share it gives
//!   on the hit's Re:CoM value (owner).
//! - **Second Chance** ([`install_second_chance`], [`arm`], [`disarm`]): Re:CoM's damage check's
//!   HP store (`0x1401E7238`) runs through a stub that keeps 1 HP when a hit would kill Sora from
//!   above 1 HP (KH1's `0x1402BFA80` rule; Re:CoM's own card-0x6C rule sits just above it). The
//!   wrapper arms it for Sora's check only while Second Chance is equipped and a hit arrived.
//! - **Leaf Bracer** ([`leaf_bracer`]): while Sora's KH1 Cure casts, the damage check runs with
//!   Re:CoM's own protection bit `player+0x418 & 1` (its exit zeroes the hit: no damage, no
//!   reaction), as the summons' invincibility does; KH1 sets its flag 0x80 for the cast
//!   (`0x14028F140` → `0x140297F30(Sora, 0)`).

use super::movement::rd;
use super::{log, mp};
use crate::abilities as ab;
use crate::kh1_support as ks;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

/// Sora's HP and max HP (`player+0x42C` / `+0x430`; the damage check's own fields). Checked
/// (decomp `0x1401E6DE0`, disasm `0x1401E71AB` / `0x1401E7241`).
const HP: usize = 0x42C;
const MAX_HP: usize = 0x430;

static LINES: AtomicU32 = AtomicU32::new(40);

fn note(line: impl FnOnce() -> String) {
    if LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("support: {}", line()));
    }
}

/// Sora's HP before Re:CoM's damage check, for [`after_damage`] (the wrapper reads it only when a
/// hit arrived).
pub unsafe fn hp(pl: usize) -> i32 {
    rd(pl + HP)
}

/// After Re:CoM's damage check (`win::attack::damage_check`, a hit having arrived), at Combat =
/// KH1: MP Rage's charge for the HP the check took.
pub unsafe fn after_damage(pl: usize, hp_before: i32) {
    let after: i32 = rd(pl + HP);
    if after >= hp_before || !mp::live() {
        return;
    }
    let Some(e) = super::abilities::equipped_live() else { return };
    let copies = ab::mp_rage(e);
    if copies == 0 {
        return;
    }
    let max: i32 = rd(pl + MAX_HP);
    let level = super::stats::sora_level();
    let units = ks::mp_rage::units(hp_before, after, max, level, copies);
    let kh1_lost = ks::mp_rage::kh1_hp_lost(hp_before - after.max(0), max, level);
    note(|| format!("MP Rage x{copies}: HP {hp_before} -> {after} of {max} = {kh1_lost} of KH1's {} at level {level}: charge +{units}", ks::mp_rage::kh1_max_hp(level)));
    if units > 0 {
        mp::gain_charge(units, &format!("MP Rage x{copies}, {kh1_lost} KH1 HP lost"));
    }
}

static BRACER_LINES: AtomicU32 = AtomicU32::new(20);

/// Leaf Bracer covers this hit: Sora's KH1 Cure is casting (`super::magic::kh1_cure_casting`:
/// Re:CoM's magic action with a KH1 Cure row) and Leaf Bracer is equipped. KH1: the list start
/// `0x14028F140` for list 3 (Cure: spell index 3, its cast motion `0x36 + 3` / `0x83 + 3`) cast by
/// the actor on itself, with `+0x184` bit 6, calls `0x140297F30(actor, 0)`: flag 0x80 with a zero
/// timer, which the actor tick (`0x140297FE0`, its `0x1402984BA` branch) clears the frame after the cast
/// motion's lock (flag 4, set by `0x14029F980` with 0x1000000) is released. While it is set, the
/// damage function `0x14029BBD0` / `0x1402BEEE0` takes no HP and starts no reaction (but for the
/// no-damage record kinds 6 / 7). Checked (decomp). Ours ends with Re:CoM's magic action (the
/// cast motion's end): ±1 frame (Assumption).
pub unsafe fn leaf_bracer(pl: usize) -> bool {
    super::magic::kh1_cure_casting(pl) && super::abilities::equipped_live().is_some_and(ab::leaf_bracer)
}

/// A hit Leaf Bracer took (logged, 20 lines).
pub fn bracer_hit(value: i16) {
    if BRACER_LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("support: Leaf Bracer: a hit of {value} on Sora during his Cure: no damage, no reaction (KH1's flag 0x80)"));
    }
}

/// The Berserk test of the swing that started last: `(KH1 row + 1) << 1 | on`. KH1 makes the swing's
/// attack object (and adds Berserk's Strength, `0x1402B2DC0`) at its start, so a swing keeps the
/// state it began with.
static SWING: AtomicU32 = AtomicU32::new(0);
static BERSERK_LINES: AtomicU32 = AtomicU32::new(30);

/// Berserk is on now: Sora's HP in KH1's low-HP state (`crate::kh1_support::berserk::low_hp` on
/// Re:CoM's HP and max HP) and Berserk equipped.
unsafe fn berserk_now(pl: usize) -> bool {
    ks::berserk::low_hp(rd(pl + HP), rd(pl + MAX_HP)) && mp::live() && super::abilities::equipped_live().is_some_and(ab::berserk)
}

/// A KH1 swing row starts (`win::attack::apply_row`): KH1's moment for Berserk's test.
pub unsafe fn swing_started(pl: usize, kh1_row: usize) {
    let on = berserk_now(pl);
    SWING.store(((kh1_row as u32 + 1) << 1) | on as u32, Ordering::Relaxed);
}

/// The Strength Berserk adds to a hit of `kh1_row` (KH1's G`+0xFE` = 4, or 0): the state its swing
/// started with, else (a row that didn't start through [`swing_started`], e.g. a special's dash)
/// the state now.
pub unsafe fn berserk_bonus(pl: usize, kh1_row: usize) -> i32 {
    let s = SWING.load(Ordering::Relaxed);
    let on = if s >> 1 == kh1_row as u32 + 1 { s & 1 != 0 } else { berserk_now(pl) };
    if on {
        ks::berserk::STRENGTH
    } else {
        0
    }
}

/// Damage = Re:CoM (owner, 2026-11-03: "a % damage bonus (the share KH1's +4 Strength gives at that
/// level)"): the hit part's Re:CoM value (`+0x92`, before Re:CoM's resistance, weak part and boss
/// falloff) × KH1's damage with Berserk ÷ without, for this row on this enemy.
pub unsafe fn berserk_recom(enemy: usize, part: usize, kh1_row: usize, hp_now: i32, max_mp: i32) {
    let Some((with, without)) = super::stats::berserk_share(enemy, kh1_row, super::stats::sora_level(), hp_now, max_mp) else { return };
    let v: i16 = rd(part + 0x92);
    let n = ks::berserk::scale(v, with, without);
    if n != v {
        super::movement::wr(part + 0x92, n);
    }
    if BERSERK_LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("support: Berserk (Damage = Re:CoM): row {kh1_row:#x} on {:#x}: KH1 {with} / {without} with / without +4 Strength: Re:CoM's {v} -> {n}", enemy & 0x00FF_FFFF));
    }
}

/// The log line for a Berserk hit at Damage = KH1 (30 lines shared with [`berserk_recom`]).
pub fn berserk_kh1_line(enemy: usize, kh1_row: usize, d: Option<i32>) {
    if BERSERK_LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("support: Berserk (Damage = KH1): row {kh1_row:#x} on {:#x}: KH1's damage with Strength + 4 = {d:?}", enemy & 0x00FF_FFFF));
    }
}

/// What the Second Chance stub reads: the armed player (`+0`, 0 = none) and the hits it saved
/// (`+8`, `lock inc`). `repr(C)`: the stub's offsets.
#[repr(C)]
struct Armed {
    pl: AtomicUsize,
    saved: AtomicU32,
}
static ARMED: Armed = Armed { pl: AtomicUsize::new(0), saved: AtomicU32::new(0) };
static SECOND_CHANCE_ON: AtomicBool = AtomicBool::new(false);
static SAVED_BEFORE: AtomicU32 = AtomicU32::new(0);
static SECOND_LINES: AtomicU32 = AtomicU32::new(20);

/// Before Re:CoM's damage check (`win::attack::damage_check`, a hit having arrived, Combat = KH1):
/// arms the stub for Sora when Second Chance is equipped.
pub unsafe fn arm(pl: usize) {
    if !SECOND_CHANCE_ON.load(Ordering::Relaxed) || !mp::live() || !super::abilities::equipped_live().is_some_and(ab::second_chance) {
        return;
    }
    SAVED_BEFORE.store(ARMED.saved.load(Ordering::Relaxed), Ordering::Relaxed);
    ARMED.pl.store(pl, Ordering::Relaxed);
}

/// After the check: disarmed; a saved hit logged.
pub unsafe fn disarm(pl: usize, hp_before: i32) {
    if ARMED.pl.swap(0, Ordering::Relaxed) == 0 {
        return;
    }
    if ARMED.saved.load(Ordering::Relaxed) != SAVED_BEFORE.load(Ordering::Relaxed) && SECOND_LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("support: Second Chance: a hit that would have killed Sora at HP {hp_before} left him at {} (KH1's 0x1402BFA80)", rd::<i32>(pl + HP)));
    }
}

/// Puts Second Chance's rule into Re:CoM's damage check: its context and the HP store checked
/// (`sites::SECOND_CHANCE_CHECKED` / `SECOND_CHANCE_PATCH`), the stub placed near it
/// (`crate::kh1_support::second_chance::stub`), the store replaced by a jump to it. Once, after the
/// damage-check wrapper is in. Without it Second Chance does nothing (logged).
pub unsafe fn install_second_chance(bs: usize) {
    use crate::sites::{SECOND_CHANCE_CHECKED, SECOND_CHANCE_PATCH};
    for c in SECOND_CHANCE_CHECKED.iter().chain(SECOND_CHANCE_PATCH) {
        if std::slice::from_raw_parts((bs + c.rva as usize) as *const u8, c.expect.len()) != c.expect {
            log(&format!("support: {} is not as expected: Second Chance does nothing", c.name));
            return;
        }
    }
    let s = &SECOND_CHANCE_PATCH[0];
    let at = bs + s.rva as usize;
    let back = (at + s.expect.len()) as u64;
    let armed = &ARMED as *const Armed as u64;
    // The stub's jump back is relative: place a block, then write the stub for its address.
    let Some(len) = ks::second_chance::stub(armed, 0, 0).map(|v| v.len()) else { return };
    let filler = vec![0xCCu8; len];
    let Some(block) = super::hook::place_near(at, &filler) else {
        log("support: no memory near the damage check: Second Chance does nothing");
        return;
    };
    let Some(code) = ks::second_chance::stub(armed, block as u64, back) else {
        log("support: the damage check is out of the stub's reach: Second Chance does nothing");
        return;
    };
    let Some(entry) = crate::hookplan::entry_patch(at as u64, s.expect.len(), block as u64) else {
        log("support: the stub is out of the damage check's reach: Second Chance does nothing");
        return;
    };
    if !super::hook::patch(block, &filler, &code, "Second Chance stub") || !super::hook::patch(at, s.expect, &entry, s.name) {
        return;
    }
    SECOND_CHANCE_ON.store(true, Ordering::Relaxed);
    log(&format!("support: {} through the stub at {block:#x}: Second Chance keeps 1 HP (KH1's 0x1402BFA80) for Sora's check while it is equipped", s.name));
}
