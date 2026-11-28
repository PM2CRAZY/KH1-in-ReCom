//! KH1's item drops from enemies inside Re:CoM's prize system (M8 stage 8a.4; `crate::drops`,
//! `docs/recom-m8-items.md` § Stage 8a.4).
//!
//! - **The look:** Re:CoM's prize look table (`0x1405B3150`, 13 rows) gets a 14th row for the new
//!   prize kind `0xD8` (a KH1 item): the table is copied with the row after Re:CoM's own, and the
//!   two `lea`s that load it (`0x140231676` in the prize init, `0x140230700` in its uncalled look
//!   set-up) point at the copy. Re:CoM's kinds read the same rows as before.
//! - **The drop:** a wrapper on the defeat's card prize `0x140180D20(enemy, &point)` (every defeat
//!   path calls it last): after Re:CoM's own, at Combat = KH1 in Sora's story, the enemy's KH1
//!   twin's table is rolled as KH1 does (`crate::drops::roll`) and each battle item that drops is
//!   made by Re:CoM's prize factory `0x1401A89C0` at the same point, launched as Re:CoM's EXP
//!   prizes are. Re:CoM then steps, draws, collects and times it out (its prize state machine
//!   `0x140231010`), as a child of the battle manager like its other prizes.
//! - **The pickup:** Re:CoM's reward `0x140230F90` tail-calls the prize add `0x1401F4AB0(kind,
//!   arg, 1)`; our wrapper takes kind `0xD8` (KH1's pickup rule, `items::pick_up`) and passes every
//!   other kind to Re:CoM. Re:CoM's pickup sound (6) plays in its own state machine.

use super::movement::{func, orig, rd};
use super::{log, va};
use crate::drops::{self as dr, site, Dropped, PrizeDesc};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

pub static ORIG_DEFEAT_PRIZE: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_PRIZE_ADD: AtomicUsize = AtomicUsize::new(0);

/// The look row is in and the hooks are installed: drops are made.
static ON: AtomicBool = AtomicBool::new(false);
/// Where the look table with the KH1 item row lives (for the start line).
static TABLE: AtomicUsize = AtomicUsize::new(0);
static LOG_LEFT: AtomicU32 = AtomicU32::new(120);
static BAD_POINT_LINES: AtomicU32 = AtomicU32::new(20);

/// Re:CoM's prize factory, the entity factory getter, the angle normaliser and its random float.
/// Byte-checked (`sites::DROP_CALLEES`, `MAGIC_CALLEES`).
const SPAWN: u32 = 0x001A_89C0;
const FACTORY: u32 = 0x001A_8A80;
const ANGLES: u32 = 0x0029_3F20;
const RAND: u32 = 0x0029_CF00;
/// An enemy's `@ENP` row (`+0x310`): every defeat prize of Re:CoM's needs it (`0x1401810A0`,
/// `0x140180D20` return at once without it). Checked (decomp).
const ENP_ROW: usize = 0x310;

type DefeatFn = unsafe extern "win64" fn(usize, usize) -> u64;
type AddFn = unsafe extern "win64" fn(u32, u32, i32);
type SpawnFn = unsafe extern "win64" fn(usize, *const PrizeDesc, usize) -> usize;
type FactoryFn = unsafe extern "win64" fn() -> usize;
type AnglesFn = unsafe extern "win64" fn(*mut f32);
type RandFn = unsafe extern "win64" fn() -> f32;

fn note(line: impl FnOnce() -> String) {
    if LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("drops: {}", line()));
    }
}

/// Builds the look table with the KH1 item row and retargets Re:CoM's two `lea`s at it. False
/// (nothing written) when a byte or a row isn't Re:CoM's.
pub unsafe fn install(b: usize) -> bool {
    let sites = crate::sites::DROP_DATA;
    let at = |s: &crate::sites::Site| b + s.rva as usize;
    let have = |p: usize, n: usize| std::slice::from_raw_parts(p as *const u8, n).to_vec();
    if sites.iter().any(|s| have(at(s), s.expect.len()) != s.expect) {
        log("drops: the prize look table's lea bytes are not Re:CoM's; no KH1 drops");
        return false;
    }
    let live = va(b, site::LOOK_TABLE);
    let rows: [u8; dr::LOOK_ROWS * dr::LOOK_ROW] = rd(live);
    // Each row: a pointer to "rec_prise" (relocated), the effect id, the scale.
    for r in 0..dr::LOOK_ROWS {
        let name: usize = rd(live + r * dr::LOOK_ROW);
        if name < b || name >= b + crate::sites::STEAM_SIZE_OF_IMAGE as usize || have(name, 10) != b"rec_prise\0" {
            log(&format!("drops: prize look row {r} doesn't name rec_prise; no KH1 drops"));
            return false;
        }
    }
    let table = dr::look_table(&rows);
    let Some(copy) = super::hook::place_data_near(at(&sites[0]), &table) else {
        log("drops: no memory near the prize init for the look table; no KH1 drops");
        return false;
    };
    let mut new = Vec::new();
    for s in sites {
        match dr::lea_rax(at(s) as u64, copy as u64) {
            Some(code) => new.push(code),
            None => {
                log("drops: the look table copy is out of the lea's reach; no KH1 drops");
                return false;
            }
        }
    }
    for (s, code) in sites.iter().zip(&new) {
        if !super::hook::patch(at(s), s.expect, code, s.name) {
            return false;
        }
    }
    TABLE.store(copy, Ordering::Release);
    true
}

/// The hooks are in: drops are made from now on.
/// The KH1 item drops' hooks are in (the prize add wrapper among them).
pub fn installed() -> bool {
    ON.load(Ordering::Acquire)
}

pub fn mark_installed() {
    ON.store(true, Ordering::Release);
}

/// The defeat's card prize (`0x140180D20(enemy, &point)`): Re:CoM's own first, then, at Combat =
/// KH1 in Sora's story, the twin's KH1 item drops at the same point.
pub unsafe extern "win64" fn defeat_prize(enemy: usize, point: usize) -> u64 {
    // Battle-end diagnostics: the defeat's point (`0x14017BBC0`: the killing hit's body shape
    // `+0x50`, Re:CoM's contact) is where all the enemy's prizes were just made. A non-finite one
    // makes prizes that can never be picked up (the battle-end soft-lock).
    if point != 0 {
        let p: [f32; 3] = rd(point);
        if !crate::magic::strike_contact::finite(p) && BAD_POINT_LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
            log(&format!("battle end: enemy {enemy:#x} defeated at a non-finite point ({}, {}, {}): its prizes can't be picked up", p[0], p[1], p[2]));
        }
    }
    let r = orig::<DefeatFn>(&ORIG_DEFEAT_PRIZE)(enemy, point);
    // 0.151.0: Riku's DP for an enemy defeated.
    super::dark_points::defeat(enemy);
    if ON.load(Ordering::Acquire) && enemy != 0 && point != 0 && super::movement::kh1_on() && rd::<usize>(enemy + ENP_ROW) != 0 {
        drop_items(enemy, point);
    }
    r
}

unsafe fn rand() -> f32 {
    func::<RandFn>(RAND)()
}

/// KH1's roll for this enemy's twin, then a prize per battle item that dropped.
unsafe fn drop_items(enemy: usize, point: usize) {
    let Some((class, world, table)) = super::stats::drops(enemy) else { return };
    let out = dr::outcome(table, dr::lucky_factor(dr::LUCKY_STRIKES), || rand());
    if out.is_empty() {
        return;
    }
    let pos: [f32; 4] = rd(point);
    for d in out {
        match d {
            Dropped::Spawned(item) => {
                let (r1, r2) = (rand(), rand());
                let mut desc = dr::item_prize(pos, item, r1, r2);
                func::<AnglesFn>(ANGLES)(desc.angles.as_mut_ptr());
                let prize = func::<SpawnFn>(SPAWN)(func::<FactoryFn>(FACTORY)(), &desc, 0);
                note(|| format!("{class} (world {world}; {}): {} dropped at ({:.2}, {:.2}, {:.2}): prize {prize:#x}", dr::describe(table), crate::items::name(item), pos[0], pos[1], pos[2]));
            }
            Dropped::NotAnItemHere(item) => note(|| format!("{class} (world {world}): KH1 item #{item} dropped; not one of the mod's items: no prize")),
            Dropped::Removed(item) => note(|| format!("{class} (world {world}): KH1's {} dropped; removed at Combat = KH1: no prize", crate::items::name(item))),
        }
    }
}

/// A prize of Re:CoM's `kind` at `pos`, made as the defeat's EXP drop makes its prizes (Re:CoM's random
/// float for the launch, its angle normaliser, its prize factory `0x1401A89C0(0x1401A8A80(), &desc, 0)`):
/// M11, a KH1 script's reward (`win::kh1_enemy`). Re:CoM steps, draws, collects and times it out. The
/// prize's address (0: none made).
pub unsafe fn spawn_reward_prize(pos: [f32; 4], kind: u32) -> usize {
    let (r1, r2) = (rand(), rand());
    let mut desc = dr::reward_prize(pos, kind, r1, r2);
    func::<AnglesFn>(ANGLES)(desc.angles.as_mut_ptr());
    func::<SpawnFn>(SPAWN)(func::<FactoryFn>(FACTORY)(), &desc, 0)
}

/// A KH1 item prize at `pos`, launched as the enemy drops' (Re:CoM's look row, Re:CoM's pickup →
/// KH1's rule): Lucky Bounty at KH1 (`super::party::pluto_find`). None when the drops' hooks (the
/// prize add that gives the item) are not in.
pub unsafe fn spawn_item_prize(pos: [f32; 4], item: u8) -> Option<usize> {
    if !ON.load(Ordering::Acquire) {
        return None;
    }
    let (r1, r2) = (rand(), rand());
    let mut desc = dr::item_prize(pos, item, r1, r2);
    func::<AnglesFn>(ANGLES)(desc.angles.as_mut_ptr());
    Some(func::<SpawnFn>(SPAWN)(func::<FactoryFn>(FACTORY)(), &desc, 0))
}

/// Re:CoM's random float in [0, 1) (`0x14029CF00`).
pub unsafe fn random() -> f32 {
    rand()
}

/// Re:CoM's prize add (`0x1401F4AB0(kind, arg, mode)`): a gimmick card at Combat = KH1 is held
/// (`super::gimmick`); a KH1 item prize is KH1's pickup; every other kind is Re:CoM's. Installed
/// by the drops, or by the gimmick cards alone.
pub unsafe extern "win64" fn prize_add(kind: u32, arg: u32, mode: i32) {
    // Re:CoM's gimmick cards at Combat = KH1 (`super::gimmick`).
    if super::gimmick::prize_add(kind, arg, mode) {
        return;
    }
    if let Some(amount) = exp_amount(kind) {
        exp_note(kind, amount, || orig::<AddFn>(&ORIG_PRIZE_ADD)(kind, arg, mode));
        return;
    }
    if kind != dr::KH1_ITEM_KIND || !ON.load(Ordering::Acquire) {
        return orig::<AddFn>(&ORIG_PRIZE_ADD)(kind, arg, mode);
    }
    let item = (arg & 0xFFFF) as u16;
    if item == 0 || item > 0xFF {
        note(|| format!("a KH1 item prize with item {item}: nothing"));
        return;
    }
    let r = super::items::pick_up(item as u8, "picked up (enemy drop)");
    if r.is_none() {
        note(|| format!("{} picked up but refused (stock at 99, or KH1 items not live): lost", crate::items::name(item as u8)));
    }
}

/// Re:CoM's EXP prizes: kinds 0xCB..=0xD1 add 1 / 10 / 60 / 5 / 30 / 99 / 1400 EXP through
/// `0x1401E5730` (the prize player's `+0x440` EXP, `+0x444` level). Checked (decomp `0x1401F4AB0`'s
/// switch, `0x1401E5730`), 2026-11-27.
fn exp_amount(kind: u32) -> Option<u32> {
    const AMOUNTS: [u32; 7] = [1, 10, 60, 5, 30, 99, 1400];
    kind.checked_sub(0xCB).and_then(|i| AMOUNTS.get(i as usize)).copied()
}

/// The prize player (`DAT_14087B390`, the EXP add's player). Checked (decomp `0x1401E5730`).
const PRIZE_PLAYER: u64 = 0x1_4087_B390;
const EXP: usize = 0x440;
const LEVEL: usize = 0x444;
/// The player's HP and action (`win::kh1_cure::HP`, `items::ACTION`).
const HP: usize = 0x274;
const ACTION: usize = 0x40C;
static EXP_LOG_LEFT: AtomicU32 = AtomicU32::new(200);

/// One log line per EXP prize the player takes (fork owner, 2026-11-27: "tenho a impressão de que só ao
/// tomar danho estou ganhando xp"): the amount, EXP and level before / after, his HP and action.
unsafe fn exp_note(kind: u32, amount: u32, add: impl FnOnce()) {
    let pl: usize = rd(va(super::base(), PRIZE_PLAYER));
    let read = |pl: usize| if pl != 0 { (rd::<i32>(pl + EXP), rd::<i32>(pl + LEVEL)) } else { (0, 0) };
    let before = read(pl);
    add();
    if EXP_LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_err() {
        return;
    }
    let after = read(pl);
    let (hp, action) = if pl != 0 { (rd::<i32>(pl + HP), rd::<u32>(pl + ACTION)) } else { (0, 0) };
    log(&format!("exp: prize {kind:#x} (+{amount}): EXP {} -> {}, level {} -> {}; HP {hp}, action {action:#x}", before.0, after.0, before.1, after.1));
}

/// The start line.
pub fn start_line() -> String {
    format!(
        "drops: KH1's item drops (M8 stage 8a.4) at combat = kh1 in Sora's story: {} twin classes with KH1 defeat tables; rolled after Re:CoM's defeat prizes (0x140180D20), KH1's chances x Lucky Strike {} (no ability yet); the items in play only (Potion, Hi-Potion, Ether, Elixir; the Mega items, Tent and Camping Set give nothing); Re:CoM prize kind {:#x} (look: kind {:#x}'s rec_prise row, table at {:#x}); pickup through the prize add 0x1401F4AB0 by KH1's rule",
        super::stats::drop_classes(),
        dr::lucky_factor(dr::LUCKY_STRIKES),
        dr::KH1_ITEM_KIND,
        dr::LOOK_FROM_KIND,
        TABLE.load(Ordering::Acquire)
    )
}
