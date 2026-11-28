//! M7 stage 7.11, spell unlocks (decision 34; `crate::unlocks`, `docs/recom-m7-unlocks.md`).
//!
//! - **The wrapper** on Re:CoM's "first obtained" `0x14015C7F0(id)` (the collection add
//!   `0x140158FC0` → `0x14015CDB0` calls it after the discovery step; it turns a card id's state
//!   from 1 to 2 the first time): after Re:CoM's own function, a magic card or a table boss's enemy
//!   card syncs Sora's spell levels from Re:CoM's discovery state ([`sync`]). In Sora's story, at
//!   either Combat setting (it writes only our save block, which Re:CoM never reads).
//! - **Older saves** (and saves played without the mod or at the Re:CoM setting before): the first
//!   read of the spell levels at the KH1 setting (`super::mp::spell_state`, the command menu's)
//!   runs the same sync once, then marks the block (`crate::unlocks::SYNCED`).
//! - **Shortcuts** (stage 7.12): each spell learned (level 0 → more) fills the first empty shortcut
//!   slot (KH1's `0x140285260`); a save whose slots were never set up (spare bit
//!   `crate::unlocks::SHORTCUTS_SET` clear) first gets the spells it already had, in KH1's order
//!   (`crate::unlocks::shortcuts`).
//! - **Enemy Skills**: an enemy card's first pickup runs the ability sync (`super::abilities::sync`),
//!   which learns its skill from the same discovery state (`crate::enemy_skills`).
//! - Each learn is logged. Nothing of ours runs per frame (one byte test in a read the menu already
//!   makes).

use super::movement::{orig, rd, wr};
use super::{base, log, va};
use crate::mp::{off, BLOCK_FIRST, BLOCK_LEN};
use crate::unlocks as u;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

pub static ORIG_FIRST_OBTAINED: AtomicUsize = AtomicUsize::new(0);
static INSTALLED: AtomicBool = AtomicBool::new(false);
static LOG_LEFT: AtomicU32 = AtomicU32::new(60);

/// Sora's card collection (`save + 0x1A74`, set by `0x140158F20`), read by `0x14015B3F0`. Checked
/// (exe).
const COLLECTION: u64 = 0x1_4087_A7E8;
const COLLECTION_IN_SAVE: usize = 0x1A74;

type IdFn = unsafe extern "win64" fn(u64);

fn note(line: impl FnOnce() -> String) {
    if LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("unlocks: {}", line()));
    }
}

pub fn mark_installed() {
    INSTALLED.store(true, Ordering::Release);
}

pub fn installed() -> bool {
    INSTALLED.load(Ordering::Acquire)
}

pub fn start_line() -> String {
    format!(
        "unlocks: KH1 spell unlocks in Sora's story (M7 stage 7.11): a spell's base level with its Re:CoM magic card's first pickup, -ra / -ga with {} Re:CoM bosses (their enemy cards' first pickup, two per spell), wrapper on Re:CoM's first-obtained 0x14015C7F0; levels at save+{:#x} (7 bytes), sync flag save+{:#x} bit 0; stage 7.12: a learned spell fills the first empty shortcut slot (save+{:#x}, 3 bytes; set up once from the spells already learned, flag bit 1)",
        u::BOSS_LEARNS.iter().map(|b| b.card).collect::<std::collections::BTreeSet<_>>().len(),
        BLOCK_FIRST + off::SPELLS,
        BLOCK_FIRST + off::SPARE,
        BLOCK_FIRST + off::SHORTCUTS
    )
}

/// Re:CoM's `0x14015C7F0(id)` (the id in `cx`; passed through whole), then the unlocks for a card
/// that matters to them.
pub unsafe extern "win64" fn first_obtained(id: u64) {
    let o: IdFn = orig(&ORIG_FIRST_OBTAINED);
    o(id);
    let id = (id & 0xFFFF) as u16;
    if u::tracked(id) {
        sync(&format!("card {id:#x} ({}) obtained for the first time", card_name(id)));
    }
    // 0.198.0: Riku's story: a boss card of his unlocks
    if crate::riku_unlocks::tracked(id) && super::stats::riku_story() {
        riku_sync(&format!("card {id:#x} obtained for the first time"));
    }
    // M8b: a summon card's first pickup learns its summon (KH1's list, learn order).
    if let Some(i) = crate::summons::of_card(id) {
        if !super::stats::riku_story() {
            super::summons::sync(&format!("{} card {id:#x} obtained for the first time", crate::summons::ALL[i].name));
        }
    }
    // Enemy Skills: an enemy card's first pickup learns its skill (unequipped, with its notice) at
    // once. The ability sync does nothing unless KH1's state is live (Combat = KH1, Sora's story, a
    // save); otherwise the next KH1 read learns it from Re:CoM's discovery state.
    if let Some(s) = crate::enemy_skills::of_card(id) {
        // 0.164.0: Riku's too (his story's own cards, his save's state).
        if !super::stats::riku_story() || super::movement::riku_on() {
            let live = super::abilities::sync().is_some();
            note(|| format!("{} card {id:#x} obtained for the first time: Enemy Skill {} ({}){}", s.name, s.name, s.effect, if live { "" } else { " (KH1's ability state not live: learned at the next KH1 read)" }));
        }
    }
}

fn card_name(id: u16) -> String {
    if let Some(s) = u::spell_of_card(id) {
        return format!("{}{} magic card", u::level_name(s, 1), if id >= u::PREMIUM { " premium" } else { "" });
    }
    u::BOSS_LEARNS.iter().find(|b| b.card == id).map_or_else(|| "?".into(), |b| format!("{} enemy card: {} beaten", b.boss, b.boss))
}

/// The first read of the spell levels at the KH1 setting for a save that was never synced: the
/// sync, once. True when it ran (the block is marked).
pub fn sync_first() -> bool {
    // SAFETY: reads and writes inside Re:CoM's save block, behind the checks in `sync`.
    unsafe { sync("the first read of this save's spells at the KH1 setting (Re:CoM's discovery state)") }
}

/// Re:CoM's int variable getter (`0x1401B08C0(set, id)`, as `super::abilities`).
const VAR_GET: u32 = 0x001B_08C0;
type VarGetFn = unsafe extern "win64" fn(u32, u32) -> u64;

/// Re:CoM's story int (0, 0x2A) in Riku's story (its hall scripts' boss-win bits: Vexen 0x80, Riku Replica I 0x400), None outside
/// his story or before the variable reader is checked. For his earned hairs (`crate::riku_unlocks::hair_unlocked`).
pub unsafe fn riku_hall_bits() -> Option<u32> {
    if !INSTALLED.load(Ordering::Acquire) || !super::stats::riku_story() {
        return None;
    }
    if rd::<usize>(va(base(), crate::keyblades::SAVE_PTR)) == 0 {
        return None;
    }
    Some(super::movement::func::<VarGetFn>(VAR_GET)(0, 0x2A) as u32)
}

/// Riku's spell levels in his story (0.198.0, `crate::riku_unlocks`): derived from Re:CoM's save (the floor label (0, 0x35), his
/// boss cards' discovery state, the hall scripts' bits (0, 0x2A)), merged into his save's spell bytes of our block (free in Riku's
/// save; never lowered), each rise logged. Run at a boss card's first pickup and at each room / battle build
/// (`super::models::group_load`), not per frame. Nothing outside Riku's story or without a save.
pub unsafe fn riku_sync(why: &str) {
    if !INSTALLED.load(Ordering::Acquire) || !super::stats::riku_story() {
        return;
    }
    let save: usize = rd(va(base(), crate::keyblades::SAVE_PTR));
    let coll: usize = rd(va(base(), COLLECTION));
    if save == 0 || coll != save + COLLECTION_IN_SAVE {
        return;
    }
    let get = super::movement::func::<VarGetFn>(VAR_GET);
    let label = get(0, 0x35) as u32 as i32;
    let bits = get(0, 0x2A) as u32;
    let (derived, wins) = crate::riku_unlocks::derive(label, |c| super::keyblades::ever_obtained(c), bits);
    let at = save + BLOCK_FIRST;
    let old: [u8; BLOCK_LEN] = rd(at);
    let mut stored = [0u8; u::SPELLS];
    stored.copy_from_slice(&old[off::SPELLS..off::SPELLS + off::SPELL_COUNT]);
    let before = stored.map(|l| if l <= 3 { l } else { 0 });
    let merged = u::merged(before, derived);
    for (s, from, to) in u::learns(before, merged) {
        let what = if from == 0 { format!("learned {}", u::level_name(s, to)) } else { format!("{} -> {}", u::level_name(s, from), u::level_name(s, to)) };
        note(|| format!("Riku {what} (spell {s} level {from} -> {to}): floor label {label}, {wins} of 14 counted bosses beaten; {why}"));
    }
    if merged != stored {
        let mut block = old;
        block[off::SPELLS..off::SPELLS + off::SPELL_COUNT].copy_from_slice(&merged);
        wr(at, block);
    }
}

/// Riku's stored spell levels (his save's spell bytes of our block; 0 outside his story or without a save).
pub unsafe fn riku_levels() -> [u8; u::SPELLS] {
    if !super::stats::riku_story() {
        return [0; u::SPELLS];
    }
    let save: usize = rd(va(base(), crate::keyblades::SAVE_PTR));
    if save == 0 {
        return [0; u::SPELLS];
    }
    let old: [u8; BLOCK_LEN] = rd(save + BLOCK_FIRST);
    let mut out = [0u8; u::SPELLS];
    for (o, &l) in out.iter_mut().zip(&old[off::SPELLS..off::SPELLS + off::SPELL_COUNT]) {
        *o = if l <= 3 { l } else { 0 };
    }
    out
}

/// Sora's spell levels from Re:CoM's discovery state (`crate::unlocks::levels`, Re:CoM's
/// `0x14015B3F0` through `super::keyblades::ever_obtained`), merged into our save block (never
/// lowered), the shortcut slots filled as KH1's learn fills them (stage 7.12), and the block marked
/// synced and set up; each rise and slot logged. Nothing in Riku's story, without a save
/// or a collection, or when the wrapper isn't in.
unsafe fn sync(why: &str) -> bool {
    if !INSTALLED.load(Ordering::Acquire) || super::stats::riku_story() {
        return false;
    }
    let save: usize = rd(va(base(), crate::keyblades::SAVE_PTR));
    let coll: usize = rd(va(base(), COLLECTION));
    if save == 0 || coll != save + COLLECTION_IN_SAVE {
        return false;
    }
    let at = save + BLOCK_FIRST;
    // SAFETY: inside the 0x2AD4-byte save block (BLOCK_LAST < 0x2818).
    let old: [u8; BLOCK_LEN] = rd(at);
    let mut stored = [0u8; u::SPELLS];
    stored.copy_from_slice(&old[off::SPELLS..off::SPELLS + off::SPELL_COUNT]);
    let derived = u::levels(|id| super::keyblades::ever_obtained(id));
    let merged = u::merged(stored, derived);
    let before = stored.map(|l| if l <= 3 { l } else { 0 });
    for (s, from, to) in u::learns(before, merged) {
        let what = if from == 0 { format!("learned {}", u::level_name(s, to)) } else { format!("{} -> {}", u::level_name(s, from), u::level_name(s, to)) };
        note(|| format!("{what} (spell {s} level {from} -> {to}): {why}"));
    }
    // Stage 7.12: KH1's learn fills the first empty shortcut slot (`0x140285260`); a save whose
    // slots were never set up first gets the spells it already had, in KH1's order.
    let mut slots = [0u8; 3];
    slots.copy_from_slice(&old[off::SHORTCUTS..off::SHORTCUTS + off::SHORTCUT_COUNT]);
    let set_up = old[off::SPARE] & u::SHORTCUTS_SET != 0;
    let (new_slots, filled) = u::shortcuts(slots, set_up, before, merged);
    for (k, s) in filled {
        note(|| {
            format!(
                "shortcut {} = {} ({})",
                k + 1,
                u::level_name(s, 1),
                if set_up || before[s as usize] == 0 { "KH1's learn: the first empty slot" } else { "set up once from the spells this save had" }
            )
        });
    }
    let mut block = old;
    block[off::SPELLS..off::SPELLS + off::SPELL_COUNT].copy_from_slice(&merged);
    block[off::SHORTCUTS..off::SHORTCUTS + off::SHORTCUT_COUNT].copy_from_slice(&new_slots);
    block[off::SPARE] |= u::SYNCED | u::SHORTCUTS_SET;
    if block != old {
        wr(at, block);
    }
    true
}
