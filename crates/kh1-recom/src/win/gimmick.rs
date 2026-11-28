//! Re:CoM's gimmick cards at Combat = KH1, in Sora's story (`crate::gimmick`,
//! `docs/recom-gimmick-cards.md`).
//!
//! - **The drop** is Re:CoM's own: each boss's code names itself the receiver (CBTL `+0x200`) and
//!   makes the card's prize with Re:CoM's prize factory `0x1401A89C0`. Our wrapper on the factory
//!   only logs a gimmick card's drop (its descriptor's kind), in every mode.
//! - **The pickup** (Re:CoM's prize add `0x1401F4AB0`, wrapped by `win::drops::prize_add`): at
//!   Combat = KH1 for the main `CPlayerSora`, Re:CoM's revolver add's rule is kept on CBTL's own
//!   gimmick count (`+0x1D0`: one at a time, `0x14015D5B0(1)`), the card is held here, and the rest
//!   of Re:CoM's prize add runs with Sora's revolver pointer hidden for the call (its type 5 / 6
//!   branch then skips only the revolver add: no deck at KH1, decision 29; its notice
//!   `0x14010A0C0(4, kind)` and the event variable still run).
//! - **The prompt**: the command menu's world asks [`offer`] each frame it runs (two loads while no
//!   card is held): "Stagger" when Re:CoM's own play would take the card.
//! - **Triangle** ([`use_card`]): the play's own writes, in Re:CoM's order: the revolver play's
//!   type-6 step `0x140161D60(&card, 1)`, the used card's count − 1 (`0x14015D5B0(-1)`), then CBTL's
//!   message 1 to Sora's card handler `0x1401F5580` (vtable `+0x48`) with the card-play record:
//!   `+0x7F4` = the kind, `+0x428 |= 0x200`. [`request`] (in the request-bits wrapper) is the
//!   builder's battle-branch conversion `+0x424 |= 0x40` the KH1 field branch skips; Re:CoM's
//!   move exe then starts its gimmick action `0x19`, which plays the card on the boss (message 6)
//!   and ends it.

use super::movement::{self as mv, func, orig, p, rd, wr};
use super::{base, log, va};
use crate::drops::PrizeDesc;
use crate::gimmick::{self as gm, cbtl, player, Held, PickUp, State};
use crate::sites::data;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

pub static ORIG_PRIZE_SPAWN: AtomicUsize = AtomicUsize::new(0);

static ON: AtomicBool = AtomicBool::new(false);
/// The held card (0: none) and its battle's CBTL.
static HELD_CARD: AtomicU32 = AtomicU32::new(0);
static HELD_CBTL: AtomicUsize = AtomicUsize::new(0);
/// The card "Stagger" is shown for (0: not shown), for the prompt's log lines.
static SHOWN: AtomicU32 = AtomicU32::new(0);
/// A card played and not yet seen through Re:CoM's action (for its log lines): the card, and
/// 1 once action 0x19 started, 2 once the boss cleared CBTL `+0x1E4`.
static PLAYED: AtomicU32 = AtomicU32::new(0);
static PLAY_STAGE: AtomicU32 = AtomicU32::new(0);
static LOG_LEFT: AtomicU32 = AtomicU32::new(200);
/// Option (c), the card break: its sites and the five classes are as read.
static BREAK_OK: AtomicBool = AtomicBool::new(false);
/// A break posted and not yet seen through (for the drop roll's log line): the boss's handle (0:
/// none), its card, frames since, the frame state 13 was first seen (0: not yet; stored + 1).
static BREAK_HANDLE: AtomicU32 = AtomicU32::new(0);
static BREAK_KIND: AtomicU32 = AtomicU32::new(0);
static BREAK_FRAMES: AtomicU32 = AtomicU32::new(0);
static BREAK_ENTERED: AtomicU32 = AtomicU32::new(0);
/// The last gimmick card kind the prize factory made (for the watch).
static DROP_SEEN: AtomicU32 = AtomicU32::new(0);
/// Re:CoM's message post (`sites::GIMMICK_BREAK_CALLEES`) and CBTL's queue / handle fields
/// (`0x14015EAC0` posts its results with them). Checked (decomp + disasm).
const POST: u32 = 0x001D_2770;
const CBTL_QUEUE: usize = 0x58;
const HANDLE: usize = 0x20;
/// An enemy's flags (`+0x260`: bit 0 attacking, `0x2000000` the card interrupt) and current state
/// (`+0x60`), HP (`+0x274`).
const EN_FLAGS: usize = 0x260;
const EN_STATE: usize = 0x60;
const EN_NEXT: usize = 0x68;
const EN_HP: usize = 0x274;

/// CBTL (`CCardBattle`, task 0x16). Checked (decomp + exe: every getter reads it).
const CBTL_PTR: u64 = 0x1_4087_A818;
/// The main player the prize add reads (`0x1401F4BDA`; the same object as
/// [`data::MAIN_PLAYER`], both set by the player constructor). Checked (decomp + exe).
const PRIZE_PLAYER: u64 = 0x1_4087_B390;
/// The card-info copy the revolver play reads the type byte from (`0x14016D029`).
const CARD_INFO: u64 = 0x1_4052_8C30;
/// `sites::GIMMICK_CALLEES`.
const NOW_SET: u32 = 0x0016_1D60;
const NOW_CLEAR: u32 = 0x0016_1D30;
const COUNT_ADD: u32 = 0x0015_D5B0;
const CARD_MESSAGE: u32 = 0x001F_5580;
/// Re:CoM's system flag and variable getters (`sites::M6_DECK_CALLEES`): the revolver play's
/// event gates.
const FLAG_GET: u32 = 0x001B_0870;
const VAR_GET: u32 = 0x001B_08C0;
/// CBTL's message 1 ("your card is played", `0x14015EAC0` posts it; `0x1401F5580` case 0).
const MESSAGE_PLAYED: i32 = 1;

type FlagFn = unsafe extern "win64" fn(u32, u32) -> u8;
type VarFn = unsafe extern "win64" fn(u32, u32) -> u32;
type NowSetFn = unsafe extern "win64" fn(*const u32, u64);
type ClearFn = unsafe extern "win64" fn();
type CountFn = unsafe extern "win64" fn(i32);
type MessageFn = unsafe extern "win64" fn(usize, i32, *const u32);
type AddFn = unsafe extern "win64" fn(u32, u32, i32);
type PostFn = unsafe extern "win64" fn(usize, u32, u32, u32, u32, usize) -> u32;
type SpawnFn = unsafe extern "win64" fn(usize, *const PrizeDesc, usize) -> usize;

fn note(line: impl FnOnce() -> String) {
    if LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("gimmick: {}", line()));
    }
}

/// Checks the Re:CoM code the cards rely on and Sora's handler slot. Nothing is written.
pub unsafe fn install(b: usize) -> bool {
    let have = |at: usize, n: usize| std::slice::from_raw_parts(at as *const u8, n).to_vec();
    for c in crate::sites::GIMMICK_CALLEES.iter().chain(crate::sites::GIMMICK_CHECKED) {
        if have(b + c.rva as usize, c.expect.len()) != c.expect {
            log(&format!("gimmick: {} is not as expected; gimmick cards stay Re:CoM's at combat = kh1 (held in the hidden revolver, no Stagger)", c.name));
            return false;
        }
    }
    let ps = &crate::sites::GIMMICK_PTRS[0];
    if rd::<usize>(b + ps.rva as usize) != b + ps.target as usize {
        log("gimmick: CPlayerSora's card message slot is not 0x1401F5580; no Stagger");
        return false;
    }
    install_break(b);
    true
}

/// Option (c): the post, the enemy's message 3, the five classes (RTTI) and their two slots. Without
/// them only the card break is left out.
unsafe fn install_break(b: usize) {
    let have = |at: usize, n: usize| std::slice::from_raw_parts(at as *const u8, n).to_vec();
    for c in crate::sites::GIMMICK_BREAK_CALLEES.iter().chain(crate::sites::GIMMICK_BREAK_CHECKED) {
        if have(b + c.rva as usize, c.expect.len()) != c.expect {
            log(&format!("gimmick: {} is not as expected; no card break for the five bosses (their cards don't drop at combat = kh1)", c.name));
            return;
        }
    }
    for p in crate::sites::GIMMICK_BREAK_PTRS {
        if rd::<usize>(b + p.rva as usize) != b + p.target as usize {
            log(&format!("gimmick: {} is not {:#x}; no card break for the five bosses", p.name, 0x1_4000_0000u64 + p.target as u64));
            return;
        }
    }
    for c in crate::sites::GIMMICK_HP_CALLEES.iter().chain(crate::sites::GIMMICK_HP_CHECKED) {
        if have(b + c.rva as usize, c.expect.len()) != c.expect {
            log(&format!("gimmick: {} is not as expected; no card breaks or HP drops for the five bosses", c.name));
            return;
        }
    }
    for k in gm::BREAK_BOSSES {
        let want = format!(".?AV{}@@", k.rtti);
        if super::stats::rtti_name(b, b + k.vtable as usize).as_deref() != Some(want.as_str()) {
            log(&format!("gimmick: the vtable at {:#x} is not {}'s; no card break for the five bosses", 0x1_4000_0000u64 + k.vtable as u64, k.rtti));
            return;
        }
    }
    BREAK_OK.store(true, Ordering::Release);
}

pub fn mark_installed() {
    ON.store(true, Ordering::Release);
}

pub fn start_line() -> String {
    let brk = if BREAK_OK.load(Ordering::Acquire) {
        "; Guard Armor, Trickmaster, Jafar, Parasite Cage and Dragon Maleficent: a KH1 combo finisher that interrupts their attack, or Sora's Guard blocking their attack or shot, posts CBTL's card-break message 3 (0x1401D2770), so their own state 13 staggers them and rolls the card; at 50 % of their max HP they drop it without a stagger (their own drop code's descriptor and conditions, Re:CoM's prize factory)"
    } else {
        "; no card breaks or HP drops (sites not as read): those five bosses' cards don't drop"
    };
    format!("{}{brk}", start_base())
}

fn start_base() -> String {
    "gimmick: Re:CoM's gimmick cards at combat = kh1 in Sora's story: Re:CoM's drops (logged in the prize factory 0x1401A89C0), held on CBTL's gimmick count (+0x1D0, one at a time) instead of the revolver, \"Stagger\" in the context tab (after an object's command, before Sonic Blade), Triangle plays it through Re:CoM's play writes (0x140161D60, 0x14015D5B0, Sora's card handler 0x1401F5580) and Re:CoM's gimmick action 0x19".into()
}

unsafe fn cbtl_now() -> usize {
    rd(va(base(), CBTL_PTR))
}

unsafe fn count(c: usize) -> i32 {
    if c == 0 {
        0
    } else {
        rd(c + cbtl::COUNT)
    }
}

/// The held card, when it is still this battle's (else it is forgotten: the battle ended, as
/// Re:CoM's battle-only cards end).
unsafe fn held() -> Option<u32> {
    let card = HELD_CARD.load(Ordering::Acquire);
    if card == 0 {
        return None;
    }
    let c = cbtl_now();
    let h = Held { cbtl: HELD_CBTL.load(Ordering::Acquire), card };
    let now = gm::held(Some(h), c, count(c));
    if now.is_none() {
        HELD_CARD.store(0, Ordering::Release);
        note(|| format!("{} no longer held (the battle ended: CBTL {:#x} → {c:#x}, count {})", gm::describe(card), h.cbtl, count(c)));
    }
    now
}

unsafe fn state(pl: usize, c: usize, held: bool) -> State {
    State {
        held,
        battle: rd::<i32>(pl + p::BATTLE) != 0,
        locked: c != 0 && rd::<u32>(c + cbtl::FLAGS) & cbtl::LOCKS != 0,
        playing: c != 0 && rd::<u8>(c + cbtl::NOW) != 0,
        refused: rd::<u32>(pl + player::CARD) & player::CARD_WON != 0 || rd::<i32>(pl + player::BUSY_7B0) != 0,
        event: func::<FlagFn>(FLAG_GET)(gm::EVENT_FLAG.0, gm::EVENT_FLAG.1) != 0 || func::<VarFn>(VAR_GET)(gm::HIDE_MASK.0, gm::HIDE_MASK.1) & gm::HIDE_SORA != 0,
    }
}

fn prompt(card: u32) {
    let was = SHOWN.swap(card, Ordering::AcqRel);
    if was != card {
        if card != 0 {
            note(|| format!("prompt: \"Stagger\" shown for the {}", gm::describe(card)));
        } else {
            note(|| format!("prompt: \"Stagger\" gone ({} played, ended or not playable now)", gm::describe(was)));
        }
    }
}

/// The context command for the command menu's world: [`gm::STAGGER`] while a held card can be
/// played now, else 0.
pub unsafe fn offer(pl: usize) -> u8 {
    if !ON.load(Ordering::Acquire) || HELD_CARD.load(Ordering::Acquire) == 0 {
        if SHOWN.load(Ordering::Relaxed) != 0 {
            prompt(0);
        }
        return 0;
    }
    let Some(card) = held() else {
        prompt(0);
        return 0;
    };
    let on = gm::offer(state(pl, cbtl_now(), true));
    prompt(if on { card } else { 0 });
    if on {
        gm::STAGGER
    } else {
        0
    }
}

/// Re:CoM's prize add for a gimmick card at Combat = KH1 (from `win::drops::prize_add`). False:
/// not ours (Re:CoM's prize add runs as it is).
pub unsafe fn prize_add(kind: u32, arg: u32, mode: i32) -> bool {
    if !ON.load(Ordering::Acquire) || gm::boss(kind).is_none() {
        return false;
    }
    let b = base();
    let pl: usize = rd(va(b, data::MAIN_PLAYER));
    if !mv::kh1_fighter(pl) || rd::<usize>(va(b, PRIZE_PLAYER)) != pl {
        // Combat = Re:CoM or Riku's story: Re:CoM's own pickup.
        return false;
    }
    let type_byte: u8 = rd(va(b, CARD_INFO) + kind as usize * 16);
    if !gm::is_gimmick(kind, type_byte) {
        return false;
    }
    let c = cbtl_now();
    let card = gm::card(kind as u8, type_byte);
    match gm::pick_up(c, count(c), card) {
        PickUp::Held(h) => {
            func::<CountFn>(COUNT_ADD)(1);
            HELD_CBTL.store(h.cbtl, Ordering::Release);
            HELD_CARD.store(h.card, Ordering::Release);
            note(|| format!("pick-up: {} held (CBTL {c:#x} gimmick count {}; receiver handle {:#x}); Re:CoM's revolver add skipped (no deck at combat = kh1)", gm::describe(card), count(c), rd::<u32>(c + cbtl::RECEIVER)));
        }
        PickUp::AlreadyHolding => {
            note(|| format!("pick-up: {} lost: one is already held (Re:CoM's rule, CBTL gimmick count {})", gm::describe(card), count(c)));
        }
        PickUp::NoBattle => {
            note(|| format!("pick-up: {} with no CBTL: Re:CoM's own prize add", gm::describe(card)));
            return false;
        }
    }
    // The rest of Re:CoM's prize add (its notice, the event variable), its revolver add skipped.
    let rev: usize = rd(pl + player::REVOLVER);
    wr(pl + player::REVOLVER, 0usize);
    orig::<AddFn>(&super::drops::ORIG_PRIZE_ADD)(kind, arg, mode);
    wr(pl + player::REVOLVER, rev);
    true
}

/// Triangle on "Stagger": the held card is played through Re:CoM's own writes.
pub unsafe fn use_card(pl: usize) {
    if !ON.load(Ordering::Acquire) || !mv::kh1_fighter(pl) {
        return;
    }
    let Some(card) = held() else {
        note(|| "use: Triangle on Stagger with no card held: nothing".into());
        return;
    };
    let c = cbtl_now();
    let s = state(pl, c, true);
    if !gm::offer(s) {
        note(|| format!("use: {} not played now ({s:?})", gm::describe(card)));
        return;
    }
    // The revolver play's type-6 step, then the used card leaves Sora's count.
    let v = card;
    func::<NowSetFn>(NOW_SET)(&v, 1);
    func::<CountFn>(COUNT_ADD)(-1);
    // CBTL's message 1 with the card-play record (Sora's card handler, vtable +0x48).
    let rec = gm::record(card);
    func::<MessageFn>(CARD_MESSAGE)(pl, MESSAGE_PLAYED, rec.as_ptr());
    let bits: u32 = rd(pl + player::CARD);
    if bits & player::CARD_GIMMICK == 0 {
        // The handler refused it (its own early return): undone, still held.
        func::<ClearFn>(NOW_CLEAR)();
        func::<CountFn>(COUNT_ADD)(1);
        note(|| format!("use: Sora's card handler refused the {} (+0x428 {bits:#x}, +0x7B0 {}); still held", gm::describe(card), rd::<i32>(pl + player::BUSY_7B0)));
        return;
    }
    HELD_CARD.store(0, Ordering::Release);
    PLAYED.store(card, Ordering::Release);
    PLAY_STAGE.store(0, Ordering::Release);
    note(|| {
        format!(
            "use: {} played (CBTL +0x1E4 {:#x}, gimmick count {}, receiver handle {:#x}; Sora +0x7F4 {:#x}, +0x428 {bits:#x}, action {:#x}): Re:CoM's gimmick action 0x19 starts from its move exe",
            gm::describe(card),
            rd::<u32>(c + cbtl::NOW),
            count(c),
            rd::<u32>(c + cbtl::RECEIVER),
            rd::<u32>(pl + player::CARD_ID),
            rd::<u32>(pl + p::ACTION)
        )
    });
}

/// In the request-bits wrapper, after Re:CoM's builder and the menu: the battle branch's
/// conversion of a pending gimmick card (`+0x428 & 0x200` → `+0x424 |= 0x40`) when the KH1
/// Attack took the field branch (`forced`); then the played card's progress, for the log (two
/// loads unless one was just played).
pub unsafe fn request(pl: usize, forced: bool) {
    if !ON.load(Ordering::Acquire) {
        return;
    }
    if forced {
        let bits: u32 = rd(pl + player::CARD);
        if bits & player::CARD_GIMMICK != 0 {
            let r: u32 = rd(pl + player::REQUEST);
            wr(pl + player::REQUEST, gm::request(bits, r));
        }
    }
    if BREAK_HANDLE.load(Ordering::Acquire) != 0 {
        watch_break();
    }
    let card = PLAYED.load(Ordering::Acquire);
    if card == 0 {
        return;
    }
    let action: u32 = rd(pl + p::ACTION);
    let c = cbtl_now();
    match PLAY_STAGE.load(Ordering::Acquire) {
        0 if action == player::GIMMICK_ACTION => {
            PLAY_STAGE.store(1, Ordering::Release);
            note(|| format!("use: Re:CoM's gimmick action 0x19 started for the {}", gm::describe(card)));
        }
        1 if c == 0 || rd::<u32>(c + cbtl::NOW) == 0 => {
            PLAY_STAGE.store(2, Ordering::Release);
            note(|| format!("use: the boss took the {} (CBTL +0x1E4 cleared by its message-6 reaction)", gm::describe(card)));
        }
        s if s >= 1 && action != player::GIMMICK_ACTION => {
            PLAYED.store(0, Ordering::Release);
            note(|| format!("use: Re:CoM's gimmick action ended (action {action:#x}, stage {s})"));
        }
        _ => {}
    }
}

/// Re:CoM's prize factory: logs a gimmick card's drop; Re:CoM's factory as it is.
pub unsafe extern "win64" fn prize_spawn(factory: usize, desc: *const PrizeDesc, parent: usize) -> usize {
    let prize = orig::<SpawnFn>(&ORIG_PRIZE_SPAWN)(factory, desc, parent);
    if !desc.is_null() {
        let kind = (*desc).kind;
        if gm::boss(kind).is_some() {
            DROP_SEEN.store(kind, Ordering::Release);
            let pos = (*desc).pos;
            let c = cbtl_now();
            let pl: usize = rd(va(base(), data::MAIN_PLAYER));
            let mode = if mv::kh1_fighter(pl) { "combat = kh1: held on pick-up, Stagger" } else { "Re:CoM's (combat = recom or Riku's story)" };
            note(|| format!("drop: {} at ({:.2}, {:.2}, {:.2}), prize {prize:#x}, receiver handle {:#x}; {mode}", gm::describe(kind), pos[0], pos[1], pos[2], if c != 0 { rd::<u32>(c + cbtl::RECEIVER) } else { 0 }));
        }
    }
    prize
}

/// Option (c) (owner, 2026-11-01), from the KH1 enemy-hit wrapper after Re:CoM's hit handler: a KH1
/// combo finisher (`row`) that interrupted (`interrupted`: the enemy was attacking and our
/// interrupt bit was set for the handler) one of the five card-break bosses' attacks gets Re:CoM's
/// own card break: CBTL's message 3 posted to it on CBTL's queue as the card-battle resolver
/// `0x14015EAC0` posts a break (delay 0, from CBTL's handle, no record). Re:CoM delivers it to the
/// boss's message handler `0x140181620`, whose case 3 sets state 13; the boss's state-13 begin
/// (vtable `+0x220`) staggers it and rolls its card.
pub unsafe fn after_kh1_hit(enemy: usize, row: Option<usize>, interrupted: bool) {
    if !interrupted || !BREAK_OK.load(Ordering::Acquire) || !ON.load(Ordering::Acquire) {
        return;
    }
    let b = base();
    let Some(boss) = gm::break_boss((rd::<usize>(enemy).wrapping_sub(b)) as u32) else {
        return;
    };
    if !mv::kh1_fighter(rd(va(b, data::MAIN_PLAYER))) {
        return;
    }
    let hit = gm::Hit {
        boss: true,
        finisher: row.is_some_and(|r| gm::is_finisher(r, super::attack::moveset())),
        attacking: true,
        ended: rd::<u64>(enemy + EN_FLAGS) & 1 == 0,
        alive: rd::<i32>(enemy + EN_HP) > 0,
    };
    if !gm::breaks(hit) {
        if hit.finisher {
            note(|| format!("break: {}'s attack, KH1 finisher row {}: no break ({hit:?})", boss.rtti, row.unwrap_or(0)));
        }
        return;
    }
    if let Some(to) = post_break(enemy, boss, "break") {
        note(|| {
            format!(
                "break: KH1 finisher row {} interrupted {}'s attack (HP {}): CBTL message 3 posted to handle {to:#x}; its state 13 ({:#x}) staggers it and rolls the {}",
                row.unwrap_or(0),
                boss.rtti,
                rd::<i32>(enemy + EN_HP),
                0x1_4000_0000u64 + boss.stagger as u64,
                gm::describe(boss.kind as u32)
            )
        });
    }
}

/// Re:CoM's card break for one of the five: CBTL's message 3 posted on CBTL's queue as the
/// resolver `0x14015EAC0` posts it (delay 0, from CBTL's handle, no record), then watched for the
/// log. The boss's handle, when posted.
unsafe fn post_break(enemy: usize, boss: &gm::BreakBoss, tag: &str) -> Option<u32> {
    let c = cbtl_now();
    if c == 0 {
        note(|| format!("{tag}: {}: no CBTL, no card break", boss.rtti));
        return None;
    }
    let to: u32 = rd(enemy + HANDLE);
    let ok = func::<PostFn>(POST)(rd(c + CBTL_QUEUE), 0, rd(c + HANDLE), to, gm::MESSAGE_BROKEN, 0);
    if ok == 0 {
        note(|| format!("{tag}: {}: CBTL's message queue is full; no card break", boss.rtti));
        return None;
    }
    BREAK_KIND.store(boss.kind as u32, Ordering::Release);
    BREAK_FRAMES.store(0, Ordering::Release);
    BREAK_ENTERED.store(0, Ordering::Release);
    DROP_SEEN.store(0, Ordering::Release);
    BREAK_HANDLE.store(to, Ordering::Release);
    Some(to)
}

/// The last blocked attack that broke a boss: (its collision, the collision's hit serial).
static GUARD_LAST: std::sync::Mutex<(usize, i32)> = std::sync::Mutex::new((0, 0));
/// A bullet's shooter handle (`CBullet +0x238`, `crate::guard::bullet::SHOOTER`).
const BULLET_SHOOTER: usize = crate::guard::bullet::SHOOTER;
/// A collision's hit serial (`crate::guard::coll::SERIAL`): one per attack instance.
const COLL_SERIAL: usize = crate::guard::coll::SERIAL;

/// Trigger (a) (owner, 2026-11-01), from the Guard block detection (`win::guard::on_result`, the
/// pair-dispatch wrapper, independent of the Guard reactions toggle): Sora's KH1 Guard blocked an
/// attack whose owner is one of the five card-break bosses (its own collision: melee and body
/// attacks), or a Re:CoM shot (`CBullet` / `CHomingBullet`) one of them fired (`+0x238`): the boss
/// gets Re:CoM's card break ([`post_break`]). Once per blocked attack (its collision and hit
/// serial); never a boss in state 13 (or with it next), a dead one, or one whose break is pending.
pub unsafe fn on_guard_block(owner: usize, c: usize) {
    if owner == 0 || !BREAK_OK.load(Ordering::Acquire) || !ON.load(Ordering::Acquire) {
        return;
    }
    let b = base();
    let vt: usize = rd(owner);
    let (enemy, via) = if vt == va(b, crate::guard::bullet::VTABLE) || vt == va(b, crate::guard::bullet::HOMING_VTABLE) {
        (func::<mv::HandleFn>(mv::HANDLE_OBJ)(rd::<u32>(owner + BULLET_SHOOTER)), "its shot")
    } else {
        (owner, "its attack")
    };
    if enemy == 0 {
        return;
    }
    let Some(boss) = gm::break_boss((rd::<usize>(enemy).wrapping_sub(b)) as u32) else {
        return;
    };
    if !mv::kh1_fighter(rd(va(b, data::MAIN_PLAYER))) {
        return;
    }
    let serial: i32 = rd(c + COLL_SERIAL);
    if let Ok(mut g) = GUARD_LAST.lock() {
        if *g == (c, serial) {
            return;
        }
        *g = (c, serial);
    }
    let s13 = enemy + gm::state_at(gm::STATE_BROKEN);
    let why = if rd::<i32>(enemy + EN_HP) <= 0 {
        Some("dead")
    } else if rd::<usize>(enemy + EN_STATE) == s13 || rd::<usize>(enemy + EN_NEXT) == s13 {
        Some("already in its card-break stagger")
    } else if BREAK_HANDLE.load(Ordering::Acquire) == rd::<u32>(enemy + HANDLE) {
        Some("a break is already on its way")
    } else {
        None
    };
    if let Some(w) = why {
        note(|| format!("guard break: Guard blocked {}'s {via}: no break ({w})", boss.rtti));
        return;
    }
    if let Some(to) = post_break(enemy, boss, "guard break") {
        note(|| {
            format!(
                "guard break: Guard blocked {} ({via}, coll {c:#x} serial {serial}; HP {}): CBTL message 3 posted to handle {to:#x}; its state 13 ({:#x}) staggers it and rolls the {}",
                boss.rtti,
                rd::<i32>(enemy + EN_HP),
                0x1_4000_0000u64 + boss.stagger as u64,
                gm::describe(boss.kind as u32)
            )
        });
    }
}

/// Trigger (d)'s marks for the boss being fought (a handle per boss; a new handle is a new fight).
static HP_MARKS: std::sync::Mutex<Vec<gm::HpMarks>> = std::sync::Mutex::new(Vec::new());
/// Re:CoM's enemy max-HP getter (hooked by the stats: the KH1 max at Damage = KH1), the receiver
/// set, Sora's point, the angle normaliser, the random float, the entity-factory getter, the prize
/// factory (through our drop-log wrapper).
const MAX_HP: u32 = 0x0017_F5A0;
const RECEIVER_SET: u32 = 0x0016_3740;
const SORA_POINT: u32 = 0x001F_0AB0;
const ANGLES: u32 = 0x0029_3F20;
const RAND: u32 = 0x0029_CF00;
const FACTORY: u32 = 0x001A_8A80;
const SPAWN: u32 = 0x001A_89C0;
type MaxHpFn = unsafe extern "win64" fn(usize) -> i32;
type ReceiverFn = unsafe extern "win64" fn(u32);
type PointFn = unsafe extern "win64" fn(*mut [f32; 4], i32) -> *const [f32; 4];
type AnglesFn = unsafe extern "win64" fn(*mut f32);
type RandFn = unsafe extern "win64" fn() -> f32;
type FactoryFn = unsafe extern "win64" fn() -> usize;
/// The boss fields its own drop code tests (Checked (decomp + disasm)).
const PC_FLAGS: usize = 0xCD4;
const TM_TABLE: usize = 0xCE8;
const TM_TABLE_FLAGS: usize = 0x2C8;
const JAFAR_STATE: usize = 0xE5C;
const DRAGON_FOOTING: usize = 0xD18;

/// The boss's own drop condition (its drop code's tests other than the chance), None = allowed.
unsafe fn own_condition(kind: u8, e: usize) -> Option<&'static str> {
    match kind {
        // Parasite Cage (`0x1403C2BDB`): none once its card was used (`+0xCD4 & 2`).
        0x3E if rd::<u8>(e + PC_FLAGS) & 2 != 0 => Some("its card was used already (+0xCD4 & 2)"),
        // Trickmaster (`0x1403CB1F4`): its table object's `+0x2C8 & 1`.
        0x3B => {
            let t: usize = rd(e + TM_TABLE);
            (t == 0 || rd::<u32>(t + TM_TABLE_FLAGS) & 1 == 0).then_some("its table's +0x2C8 & 1 is clear")
        }
        // Jafar (`0x1403CFE29`): not in its state 15 (`+0xE5C`).
        0x3C if rd::<i32>(e + JAFAR_STATE) == 0xF => Some("it is in its state 15 (+0xE5C)"),
        // Dragon Maleficent (`0x1403E16BD`): no footing of its own out (`+0xD18`, a live handle).
        0x40 => {
            let h: u32 = rd(e + DRAGON_FOOTING);
            (h != 0 && func::<mv::HandleFn>(mv::HANDLE_OBJ)(h) != 0).then_some("its footing is out (+0xD18)")
        }
        _ => None,
    }
}

/// Trigger (d), from the KH1 enemy-hit wrapper after every hit (`hp` before it): one of the five
/// crossing 50 % of its max HP drops its card without a stagger, made as its own drop
/// code makes it (the receiver, the descriptor, Re:CoM's prize factory). Once per threshold per
/// fight; one card per hit even across several thresholds; a threshold its own condition refuses
/// waits for a later hit. Not on the killing hit.
pub unsafe fn after_hp(enemy: usize, before: i32) {
    if !BREAK_OK.load(Ordering::Acquire) || !ON.load(Ordering::Acquire) {
        return;
    }
    let b = base();
    let Some(boss) = gm::break_boss((rd::<usize>(enemy).wrapping_sub(b)) as u32) else {
        return;
    };
    let after: i32 = rd(enemy + EN_HP);
    if after <= 0 || !mv::kh1_fighter(rd(va(b, data::MAIN_PLAYER))) {
        return;
    }
    let handle: u32 = rd(enemy + HANDLE);
    let Ok(mut marks) = HP_MARKS.lock() else { return };
    let prev = marks.iter().copied().find(|m| m.handle == handle);
    if prev.is_some_and(|m| m.done == 7 && m.owed == 0) || (prev.is_none() && after >= before) {
        return;
    }
    let max = func::<MaxHpFn>(MAX_HP)(enemy);
    let (mut m, new) = gm::hp_step(prev, handle, before, after, max);
    if new != 0 {
        note(|| format!("hp drop {}: {} at {after} / {max} HP (from {before})", gm::hp_names(new), boss.rtti));
    }
    if m.owed != 0 {
        match own_condition(boss.kind, enemy) {
            Some(w) => note(|| format!("hp drop {}: {} waits: {w}", gm::hp_names(m.owed), boss.rtti)),
            None => {
                let owed = m.owed;
                m.owed = 0;
                spawn_card(enemy, boss, owed);
            }
        }
    }
    marks.retain(|x| x.handle != handle);
    if marks.len() >= 8 {
        marks.remove(0);
    }
    marks.push(m);
}

/// The boss's card as its own drop code makes it: the receiver (`0x140163740`, the boss's handle),
/// the descriptor ([`gm::drop_desc`]; Re:CoM's random float, angle normaliser), Re:CoM's prize
/// factory `0x1401A89C0(0x1401A8A80(), &desc, 0)`.
unsafe fn spawn_card(enemy: usize, boss: &gm::BreakBoss, owed: u8) {
    func::<ReceiverFn>(RECEIVER_SET)(rd(enemy + HANDLE));
    let pos: [f32; 4] = rd(rd::<usize>(enemy + 0xF0) + 0x30);
    let mut sora = [0f32; 4];
    if boss.kind == 0x40 {
        func::<PointFn>(SORA_POINT)(&mut sora, 0);
    }
    let r = if boss.kind == 0x3C { 0.0 } else { func::<RandFn>(RAND)() };
    let Some((mut desc, normalise)) = gm::drop_desc(boss.kind, pos, sora, r) else { return };
    if normalise {
        func::<AnglesFn>(ANGLES)(desc.angles.as_mut_ptr());
    }
    let prize = func::<SpawnFn>(SPAWN)(func::<FactoryFn>(FACTORY)(), &desc, 0);
    note(|| format!("hp drop {}: {} made at ({:.2}, {:.2}, {:.2}), prize {prize:#x} (no stagger)", gm::hp_names(owed), gm::describe(boss.kind as u32), desc.pos[0], desc.pos[1], desc.pos[2]));
}

/// Each frame while a break is pending (from [`request`]): the boss's state 13 and its card's
/// prize, for the drop roll's log line.
unsafe fn watch_break() {
    let h = BREAK_HANDLE.load(Ordering::Acquire);
    let kind = BREAK_KIND.load(Ordering::Acquire);
    let frames = BREAK_FRAMES.fetch_add(1, Ordering::AcqRel) + 1;
    let obj = func::<mv::HandleFn>(mv::HANDLE_OBJ)(h);
    let in_state = obj != 0 && rd::<usize>(obj + EN_STATE) == obj + gm::state_at(gm::STATE_BROKEN);
    let mut e = BREAK_ENTERED.load(Ordering::Acquire);
    if in_state && e == 0 {
        e = frames + 1;
        BREAK_ENTERED.store(e, Ordering::Release);
        note(|| format!("break: the {}'s boss entered its state 13 (card-break stagger)", gm::describe(kind)));
    }
    let r = gm::roll_step(frames, e.checked_sub(1), obj != 0, in_state, DROP_SEEN.load(Ordering::Acquire) == kind);
    let line = match r {
        gm::Roll::Wait => return,
        gm::Roll::Dropped => "drop roll: the card dropped",
        gm::Roll::NoDrop => "drop roll: no card (its chance missed, or its own condition refused)",
        gm::Roll::NoStagger => "no state 13 within 120 frames (the boss didn't take message 3)",
        gm::Roll::Gone => "the boss is gone",
    };
    BREAK_HANDLE.store(0, Ordering::Release);
    note(|| format!("break: {}: {line} ({frames} frames)", gm::describe(kind)));
}
