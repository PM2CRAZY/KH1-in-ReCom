//! KH1's ability state at the KH1 setting (M9 stages 9a / 9b; `crate::abilities`,
//! `docs/recom-m9-abilities.md`).
//!
//! - **Store:** our ability block in Re:CoM's save (`crate::abilities::BLOCK_FIRST`). Re:CoM saves,
//!   loads, snapshots and zeroes it with its save block.
//! - **Sync** ([`sync`]): at each Attack press Re:CoM's dispatch hands to KH1's frame, and when the
//!   camp's Abilities page opens, never per frame: the abilities not yet unlocked are checked
//!   against Re:CoM's own learned-sleight test (`0x14023AB00`), Re:CoM's level and Re:CoM's
//!   ability variable (0, 0x24) read with its getter `0x1401B08C0` (new ones arrive unequipped,
//!   KH1's rule; Re:CoM's High Jump / Glide / Superglide equipped, owner); a block older than layout
//!   v5 is migrated once (`crate::abilities::migrate`: Guard and Dodge Roll leave it, they are
//!   Sora's kit; M12: the AP Up count starts at the save's past CP Up picks, from Re:CoM's CP);
//!   written back, logged. New level abilities (KH1's level list: Combo Plus, Air Combo Plus, …)
//!   are queued as notices (`super::notices`), unless the block was never written (a save's
//!   first sync: everything would be "new").
//! - **AP Up** ([`add_ap_up`], M12): the level-up screen's AP Up pick adds one to the count.
//! - **Sleight abilities (M12 step 4):** the 27 of `crate::abilities::SLEIGHTS` unlock with
//!   Re:CoM's learned bit like the modifiers (read only, never set: trophy 0x45), unequipped; each
//!   gets Re:CoM's own learned screen as a notice (`crate::notices::sleight_abilities`) unless
//!   Re:CoM's screen just showed it (`super::notices::shown_by_recom`) or the block was older than
//!   v6 (learned before this build). The words' bits 32.. live in the extension
//!   (`crate::abilities::EXT_FIRST`, layout v6). [`equipped_sleights`] for the Sleights menu.
//! - **Enemy Skills (plan stage 1):** the `Source::Card` abilities (`crate::enemy_skills`) unlock in
//!   the same sync from Re:CoM's discovery state (`0x14015B3F0` through
//!   `super::keyblades::ever_obtained`), unequipped, each with a "Sora learned X" notice
//!   (`crate::notices::card_abilities`) unless the block was older than v7 (cards had before this
//!   build: no notices). The first-obtained wrapper (`super::unlocks::first_obtained`) runs the sync
//!   when an enemy card is picked up for the first time. Bits 64.. live in the skill extension
//!   (`crate::abilities::SKILL_EXT_FIRST`, layout v7); the unbuilt ones are never equipped.
//! - **Use:** [`equipped_live`] reads the equipped word from the block (15 + 8 + 17 bytes) for the
//!   abilities the KH1 frame passes on (`win::movement::abilities`), so what the camp's page
//!   changes applies at once, and a loaded save is followed with no event of ours.
//! - **The page** (`win::keyblades`, the KH1 entry's Abilities row): [`page_state`] and
//!   [`set_equipped`].

use super::movement::{func, rd, wr};
use super::{base, log, va};
use crate::abilities::{self as ab, State};
use crate::settings::Kh1Abilities;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// The modifiers whose rows the attack table holds (`crate::attack::modifier_rows_ready`).
static READY: AtomicU64 = AtomicU64::new(0);
/// `kh1_all_abilities = on` (debug): every ability equipped, nothing written.
static ALL: AtomicBool = AtomicBool::new(false);
/// The ini's movement abilities for the migration of a v0 / v1 block
/// (`crate::abilities::ini_equipped`).
static INI_EQUIPPED: AtomicU64 = AtomicU64::new(0);
static LOGGED_FIRST: AtomicBool = AtomicBool::new(false);

/// Re:CoM's "sleight learned" (`0x14023AB00(u16 id)`, `sites::ABILITY_CALLEES`).
const SLEIGHT_LEARNED: u32 = 0x0023_AB00;
pub(super) type LearnedFn = unsafe extern "win64" fn(u16) -> u64;
/// Re:CoM's int variable getter (`0x1401B08C0(set, id)`, `sites::M6_DECK_CALLEES`, bytes checked at
/// start), for its ability variable (`crate::abilities::recom::VAR`).
const VAR_GET: u32 = 0x001B_08C0;
type VarGetFn = unsafe extern "win64" fn(u32, u32) -> u64;

/// Re:CoM's "sleight learned" test, for Riku's log line (`super::command_menu`).
pub unsafe fn sleight_learned_fn() -> LearnedFn {
    func::<LearnedFn>(SLEIGHT_LEARNED)
}

pub fn set_rows_ready(bits: ab::Bits) {
    // The modifiers' bits are all below 64.
    READY.store((bits & ab::MODIFIER_BITS) as u64, Ordering::Relaxed);
}

pub fn set_all(on: bool) {
    ALL.store(on, Ordering::Relaxed);
    if on {
        log("abilities: kh1_all_abilities = on: every ability equipped, AP ignored (debug; not saved)");
    }
}

/// The ini's movement abilities, read once per save at its migration from v0 / v1. (The ini's
/// `high_jump`, `glide`, `superglide`, `dodge_roll`, `combo_master` no longer unlock anything:
/// only the KH1 frame's fallback with no save block reads them, `win::movement::abilities`.)
pub fn set_ini(a: &Kh1Abilities) {
    // Counterattack and the copies: all below 64.
    INI_EQUIPPED.store(ab::ini_equipped(a) as u64, Ordering::Relaxed);
}

fn save() -> Option<usize> {
    // SAFETY: a static pointer of the image.
    let s: usize = unsafe { rd(va(base(), crate::keyblades::SAVE_PTR)) };
    (s != 0).then_some(s)
}

/// Writes the state into the block and its extensions (layout `crate::abilities::BLOCK_NOW`).
fn store(save: usize, s: &State) {
    let (at, ext_at, sk_at) = (save + ab::BLOCK_FIRST, save + ab::EXT_FIRST, save + ab::SKILL_EXT_FIRST);
    // SAFETY: inside the 0x2AD4-byte save block (BLOCK_LAST, EXT_LAST, SKILL_EXT_LAST < 0x2818).
    unsafe {
        let mut block: [u8; ab::BLOCK_LEN] = rd(at);
        let mut ext: [u8; ab::EXT_LEN] = rd(ext_at);
        let mut skills: [u8; ab::SKILL_EXT_LEN] = rd(sk_at);
        ab::write(s, &mut block, &mut ext, &mut skills);
        wr(at, block);
        wr(ext_at, ext);
        wr(sk_at, skills);
    }
}

/// The modifiers without rows never reach the pickers; an unbuilt ability never acts.
fn usable(e: ab::Bits) -> ab::Bits {
    e & (READY.load(Ordering::Relaxed) as ab::Bits | !ab::MODIFIER_BITS) & !ab::UNBUILT_BITS
}

/// Brings the state up to Re:CoM's (sleights learned, level, its ability variable), migrating an
/// older block once; writes it back when it changed. None unless KH1's MP is live (the KH1
/// setting, Sora's story, a save).
pub fn sync() -> Option<State> {
    // 0.164.0 (fork owner, 2026-11-23: Enemy Skills for Riku, "as cartas que o Riku ganha durante a
    // play dele"): Riku's state in his story's save. Only the Enemy Skills (card sources) reach it:
    // no KH1 level grants, no Re:CoM movement abilities, no sleight abilities (Sora's).
    if !super::mp::live() {
        return None;
    }
    let riku = super::mp::riku_live();
    let save = save()?;
    let at = save + ab::BLOCK_FIRST;
    let ext_at = save + ab::EXT_FIRST;
    let sk_at = save + ab::SKILL_EXT_FIRST;
    // SAFETY: inside the 0x2AD4-byte save block (BLOCK_LAST, EXT_LAST, SKILL_EXT_LAST < 0x2818).
    let mut block: [u8; ab::BLOCK_LEN] = unsafe { rd(at) };
    let mut ext: [u8; ab::EXT_LEN] = unsafe { rd(ext_at) };
    let mut skills: [u8; ab::SKILL_EXT_LEN] = unsafe { rd(sk_at) };
    let version = ab::version(&block);
    let before = ab::read(&block, &ext, &skills);
    // The words as stored (an older block may still hold the kit's bits, which `read` hides).
    let raw = ab::raw(&block, &ext, &skills);
    let mut s: State = before;
    let level = if riku { 0 } else { super::stats::sora_level() };
    // SAFETY: Re:CoM's own getter (bytes checked at start), a pure read of its save variables.
    let recom_var = if riku { 0 } else { (unsafe { func::<VarGetFn>(VAR_GET)(ab::recom::VAR.0, ab::recom::VAR.1) }) as u32 };
    // SAFETY: Re:CoM's own test (`sites::ABILITY_CALLEES`, bytes checked at start), a pure read of
    // its save.
    let learned = unsafe { func::<LearnedFn>(SLEIGHT_LEARNED) };
    let mut ask = |id: u16| !riku && unsafe { learned(id) } & 0xFF != 0;
    // Enemy Skills: Re:CoM's "obtained at least once" (`0x14015B3F0`) for the skill's card.
    // SAFETY: Re:CoM's own read of its collection (Keyblades' checked call), guarded by the collection.
    let mut card = |kind: u16| unsafe { super::keyblades::ever_obtained(kind) };
    let migrating = version < ab::BLOCK_NOW;
    // M12: an older block's past CP Up picks count as AP Ups (owner), read once from Re:CoM's CP.
    let cp = super::stats::sora_cp().unwrap_or(ab::RECOM_START_CP);
    // 0.164.0: Riku's past DP Up picks count as his AP Ups (his DP row is his AP Up).
    let cp_ups = if riku { ab::ap_ups_from_dp(super::stats::riku_dp().unwrap_or(ab::RIKU_START_DP), super::stats::sora_level()) } else { ab::ap_ups_from_cp(cp, level) };
    let ini = if riku { 0 } else { INI_EQUIPPED.load(Ordering::Relaxed) as ab::Bits };
    let mut r = if migrating {
        ab::migrate_with(&mut s, raw, version, level, recom_var, ini, cp_ups, &mut ask, &mut card)
    } else {
        ab::sync_with(&mut s, level, recom_var, &mut ask, &mut card)
    };
    if riku {
        s.unlocked &= ab::CARD_BITS;
        s.equipped &= s.unlocked;
        r.new &= ab::CARD_BITS;
        r.new_equipped &= ab::CARD_BITS;
    }
    if s != before || migrating {
        ab::write(&s, &mut block, &mut ext, &mut skills);
        // SAFETY: as above.
        unsafe {
            wr(at, block);
            wr(ext_at, ext);
            wr(sk_at, skills);
        }
    }
    if migrating {
        log(&format!(
            "abilities: block v{version} -> v{} at Re:CoM level {level} (Re:CoM's ability variable {recom_var:#x}){}: max AP {}, equipped {:?} ({} AP){}{}{}",
            ab::BLOCK_NOW,
            if version < ab::BLOCK_V5 { format!("; Re:CoM's CP {cp}: {} past CP Up pick(s) counted as AP Up(s) (({cp} - {}) / {}, at most level - 1)", s.ap_ups, ab::RECOM_START_CP, ab::RECOM_CP_PER_UP) } else { String::new() },
            s.max_ap(),
            ab::names(s.equipped),
            ab::used_ap(s.equipped),
            if r.kit_dropped != 0 { format!("; Sora's kit now, out of the block (their AP back) {:?}", ab::names(r.kit_dropped)) } else { String::new() },
            if r.taken != 0 { format!("; not yet from their source (the ini's story abilities at v2; v7: the KH1 matches now from their enemy cards), taken back {:?}", ab::names(r.taken)) } else { String::new() },
            if r.fitted != 0 { format!("; over the AP, unequipped from the most expensive down {:?}", ab::names(r.fitted)) } else { String::new() }
        ));
    }
    // M12 step 2: the level abilities get Re:CoM's learned screen (not on a save's first sync).
    if version != 0 {
        for l in crate::notices::level_abilities(r.new | r.new_equipped) {
            super::notices::notify_learned(l);
        }
    } else if r.new | r.new_equipped != 0 {
        log("abilities: a save's first sync: its abilities arrive without notices");
    }
    // Enemy Skills: a skill learned from its card gets Re:CoM's learned screen with the card's name
    // (not from a block older than v7: those cards were picked up before this build).
    let new_skills = (r.new | r.new_equipped) & ab::CARD_BITS;
    if new_skills != 0 {
        for l in crate::notices::card_abilities(new_skills, version) {
            super::notices::notify_learned(l);
        }
        log(&format!(
            "abilities: Enemy Skills learned from their cards (Re:CoM's discovery state 0x14015b3f0; unequipped): {}{}",
            ab::ALL.iter().enumerate().filter(|(i, _)| new_skills & (1 << i) != 0).map(|(i, a)| format!("{} ({}, {} AP{})", a.name, crate::enemy_skills::of_ability(i).map_or("?", |k| k.effect), a.ap, if ab::built(i) { "" } else { ", not built yet" })).collect::<Vec<_>>().join(", "),
            if version < ab::BLOCK_V7 { format!(" (block v{version}: cards had before, no notices)") } else { String::new() }
        ));
    }
    // M12 step 4: a sleight ability Re:CoM taught gets Re:CoM's own learned screen, unless Re:CoM
    // just showed one for it (level-up pick, event, chest) or the block was older than v6.
    let new_sleights = (r.new | r.new_equipped) & ab::SLEIGHT_BITS;
    if new_sleights != 0 {
        let mut skipped = Vec::new();
        let notices = crate::notices::sleight_abilities(new_sleights, version, &mut |id| {
            let shown = super::notices::shown_by_recom(id);
            if shown {
                skipped.push(id);
            }
            shown
        });
        for &l in &notices {
            super::notices::notify_learned(l);
        }
        log(&format!(
            "abilities: sleight abilities Re:CoM taught (its learned bits; unequipped): {:?}{}{}",
            ab::names(new_sleights),
            if version < ab::BLOCK_V6 { format!(" (block v{version}: learned before, no notices)") } else { String::new() },
            if skipped.is_empty() { String::new() } else { format!("; no notice for {skipped:x?} (Re:CoM's own screen showed it)") }
        ));
    }
    if r.new & !ab::CARD_BITS != 0 {
        log(&format!("abilities: learned at Re:CoM level {level} (unequipped, KH1's rule; Re:CoM's ability variable {recom_var:#x}): {:?}", ab::names(r.new & !ab::CARD_BITS)));
    }
    // M12: the learned notice's call point (see [`on_learned`]).
    for i in (0..ab::ALL.len()).filter(|&i| (r.new | r.new_equipped) & (1 << i) != 0) {
        on_learned(i);
    }
    if r.new_equipped != 0 {
        log(&format!("abilities: learned at Re:CoM level {level} and equipped (owner: Re:CoM gave it; removable): {:?}", ab::names(r.new_equipped)));
    }
    if !LOGGED_FIRST.swap(true, Ordering::Relaxed) {
        log(&format!(
            "abilities: level {level}, Re:CoM's ability variable {recom_var:#x}, AP {}/{} left ({} AP Up pick(s)); unlocked {:?}; equipped {:?}; modifiers the pickers get {:?}",
            ab::ap_left(s.equipped, s.max_ap()),
            s.max_ap(),
            s.ap_ups,
            ab::names(s.unlocked),
            ab::names(s.equipped),
            ab::names(usable(s.equipped) & ab::MODIFIER_BITS)
        ));
    }
    Some(s)
}

/// M12 (owner, 2026-11-03: "… I want there to be a notification that lets the player know that
/// they've obtained those abilities"): called once for each ability [`sync`] newly unlocks (`i` =
/// `crate::abilities::ix`), e.g. MP Haste at Re:CoM level 21.
///
/// The notice itself comes from [`crate::notices::level_abilities`] in [`sync`] (M12 step 2, merged):
/// every level ability [`sync`] newly unlocks is queued there; this hook stays for later callers.
pub fn on_learned(i: usize) {
    let _ = i;
}

/// The equipped abilities the KH1 frame passes on (`ab::ix` bits, the modifiers only with rows),
/// or None at the Re:CoM setting or with no save block (the ini then stands, as before stage 9b).
/// Each KH1 frame: one 15-byte read of the block and an 8-byte read of its extension (M12 step 4);
/// a block older than the layout now is migrated by [`sync`] once.
pub fn equipped_live() -> Option<ab::Bits> {
    if ALL.load(Ordering::Relaxed) {
        return Some(usable(ab::ALL_BITS & !ab::SPECIALS_WAITING & !ab::KIT_BITS & !ab::UNBUILT_BITS));
    }
    // The KH1 frame runs only for Sora at the KH1 setting (never in Riku's story), so no story
    // test here: one atomic load, one pointer read and the block.
    if !super::movement::kh1_on() {
        return None;
    }
    let save = save()?;
    // SAFETY: inside the 0x2AD4-byte save block.
    let block: [u8; ab::BLOCK_LEN] = unsafe { rd(save + ab::BLOCK_FIRST) };
    let e = if ab::version(&block) < ab::BLOCK_NOW {
        sync()?.equipped
    } else {
        // SAFETY: as above (EXT_LAST, SKILL_EXT_LAST < 0x2818).
        let ext: [u8; ab::EXT_LEN] = unsafe { rd(save + ab::EXT_FIRST) };
        let skills: [u8; ab::SKILL_EXT_LEN] = unsafe { rd(save + ab::SKILL_EXT_FIRST) };
        ab::read(&block, &ext, &skills).equipped
    };
    // Enemy Skills stage 5: Black Fungus's skill counts as equipped in its battle (one atomic load
    // without it).
    Some(usable(e | super::enemy_skills_deck::live_bonus()))
}

/// M12 step 4: the equipped sleight abilities' Re:CoM sleight ids, in Re:CoM's order, for the
/// Sleights menu (the next step reads it). Empty when the state isn't live.
#[allow(dead_code)]
pub fn equipped_sleights() -> Vec<u16> {
    equipped_live().map_or_else(Vec::new, ab::equipped_sleights)
}

/// The camp's Abilities page: the state after a sync, and Sora's level. None when not live.
pub fn page_state() -> Option<(State, u32)> {
    let s = sync()?;
    Some((s, super::stats::sora_level()))
}


/// M12: one AP Up pick on Re:CoM's level-up screen at Combat = KH1 (+2 max AP). The state is
/// synced first (an older block is migrated with its past CP picks before this one is added).
/// Returns (picks, max AP) after it, or None when the state isn't live.
pub fn add_ap_up() -> Option<(u8, i32)> {
    let mut s = sync()?;
    let save = save()?;
    s.ap_ups = s.ap_ups.saturating_add(1);
    store(save, &s);
    Some((s.ap_ups, s.max_ap()))
}

/// Sora's max AP now (for the level-up screen's panel), or None when the state isn't live. A read
/// (an older block reads its count as the migration will set it).
pub fn max_ap_now() -> Option<i32> {
    if !super::mp::live() {
        return None;
    }
    let save = save()?;
    // SAFETY: inside the 0x2AD4-byte save block.
    let block: [u8; ab::BLOCK_LEN] = unsafe { rd(save + ab::BLOCK_FIRST) };
    let old = || if super::mp::riku_live() { ab::ap_ups_from_dp(super::stats::riku_dp().unwrap_or(ab::RIKU_START_DP), super::stats::sora_level()) } else { ab::ap_ups_from_cp(super::stats::sora_cp().unwrap_or(ab::RECOM_START_CP), super::stats::sora_level()) };
    let ups = if ab::version(&block) >= ab::BLOCK_V5 { block[ab::field::AP_UPS] } else { old() };
    Some(ab::max_ap(ups as u32))
}

/// The camp's Abilities page writes the equipped set (a subset of the unlocked set). False when
/// the state isn't live.
pub fn set_equipped(equipped: ab::Bits) -> bool {
    let Some(mut s) = sync() else { return false };
    let Some(save) = save() else { return false };
    // An unbuilt ability is never equipped (the page greys and refuses it).
    s.equipped = equipped & s.unlocked & !ab::UNBUILT_BITS;
    store(save, &s);
    // Enemy Skills: the equipped set's effects on Sora now (`super::enemy_skills`).
    // SAFETY: Re:CoM's main player's fields, written as Re:CoM's own apply does.
    unsafe { super::enemy_skills::refresh("ability set changed") };
    true
}
