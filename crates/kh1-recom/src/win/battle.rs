//! M6 stage 1, battles at the KH1 setting (decisions 29 / 30; `crate::battle`,
//! `docs/recom-m6-battles.md`).
//!
//! - `manager_ctor` (`0x1401859B0`, once per battle): decides whether this battle is a KH1 one
//!   (Combat = KH1 and Sora's story) and switches the three serialiser patches P2–P4 to match
//!   with checked writes, all or none. At the Re:CoM setting the bytes are Re:CoM's own.
//! - `card_play` (`0x140198870`, enemy vtable `+0x98`, once per granted attack): in a KH1 battle
//!   the enemy enters its chosen attack state as message 2 would, with no card. Re:CoM's own
//!   state code runs the attack. CBTL's lock gates keep Re:CoM's refusal.
//! - `set_play_card` (`0x140163830`, CBTL::setPlayCard): a pass-through that logs, once per
//!   class, who still plays a side-2 card in a KH1 battle (human bosses, Marluxia 3's sleights),
//!   and (stage 3) each side-2 card, per battle, with Sora's action and what bosses read.
//! - Stage 3 (`crate::boss`): P5, Re:CoM's "Sora's action" getter `0x14017F720` rewritten in
//!   place by the same switch (ours in a KH1 battle: action 0xA reads as 0xB); boss attacks
//!   without a card logged per battle; `duel_init` (`0x140220C40`, log only): Sora's card duel.
//!
//! - `end_watch` (Re:CoM's battle end, read only; `crate::battle::end`): from Sora's frame start
//!   (the request-bits wrapper), only while Re:CoM's win is pending or Sora is in its win action:
//!   a log line when what Re:CoM waits for changes (and every 10 s while it doesn't), and the F9
//!   trace's `end …` token.
//!
//! Per frame: nothing but `end_watch`'s two loads. The trace (F9 only) reads the enemy list
//! through `trace_token`.

use super::movement::{self as mv, func, orig, rd, wr};
use super::{base, log, va};
use crate::battle::{self as b, cbtl, enemy, Bytes, Gate, Handler, Patch, Path, Play, Why};
use crate::battle::end as be;
use crate::boss;
use crate::sites::{data, M6_BOSS_PATCHES, M6_PATCHES};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

pub static ORIG_CARD_PLAY: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_MANAGER_CTOR: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_SET_PLAY_CARD: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_DUEL_INIT: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_ENEMY_ATTACK_END: AtomicUsize = AtomicUsize::new(0);

/// This battle is a KH1 one (decided when Re:CoM built its battle enemy manager).
static KH1_BATTLE: AtomicBool = AtomicBool::new(false);
static ATTACK_LINES: AtomicU32 = AtomicU32::new(0);
static BATTLE_LINES: AtomicU32 = AtomicU32::new(0);
static REFUSAL_LINES: AtomicU32 = AtomicU32::new(0);
/// Stage 3: battles so far (manager ctors), the boss lines of this battle and in all, the side-2
/// cards of this battle, the duel lines.
static BATTLE_NO: AtomicU32 = AtomicU32::new(0);
static BOSS_BATTLE_LINES: AtomicU32 = AtomicU32::new(0);
static BOSS_TOTAL_LINES: AtomicU32 = AtomicU32::new(0);
static SIDE2_CARDS: AtomicU32 = AtomicU32::new(0);
static DUEL_LINES: AtomicU32 = AtomicU32::new(0);

/// Re:CoM's "Sora's action" getter (P5's site; `sites::M6_BOSS_PATCHES`).
const ACTION_GETTER: u32 = 0x0017_F720;
/// Sora's action (`player+0x40C`).
const PL_ACTION: usize = 0x40C;
type GetterFn = unsafe extern "win64" fn() -> u32;
type DuelInitFn = unsafe extern "win64" fn(usize, i32);
/// (vtable, path) pairs already logged: 0 cardless, 1 Re:CoM's card play (fallback), 2 a side-2
/// card set in a KH1 battle.
static CLASSES: Mutex<Vec<(usize, u8)>> = Mutex::new(Vec::new());

/// `0x1402AD300` (Checked (exe), `sites::M6_CALLEES`).
const TASK_BY_ID: u32 = 0x002A_D300;

type TaskFn = unsafe extern "win64" fn(u32) -> usize;
type PlayFn = unsafe extern "win64" fn(usize) -> u64;
type CtorFn = unsafe extern "win64" fn(usize, usize, usize, usize) -> usize;
type SetPlayFn = unsafe extern "win64" fn(usize, u32, i32, usize, usize, i32) -> u64;
type TypeFn = unsafe extern "win64" fn(usize) -> u32;
type AttackEndFn = unsafe extern "win64" fn(usize, usize, i32);

fn capped(counter: &AtomicU32, cap: u32, line: impl FnOnce() -> String) {
    if counter.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| (n < cap).then_some(n + 1)).is_ok() {
        log(&format!("m6: {}", line()));
    }
}

/// A stage-3 boss line: per battle and in all ([`boss::boss_line`]). `line` returns None for a
/// line that doesn't count (not a boss).
fn boss_capped(line: impl FnOnce() -> Option<String>) {
    if !boss::boss_line(BOSS_BATTLE_LINES.load(Ordering::Relaxed), BOSS_TOTAL_LINES.load(Ordering::Relaxed)) {
        return;
    }
    if let Some(l) = line() {
        BOSS_BATTLE_LINES.fetch_add(1, Ordering::Relaxed);
        BOSS_TOTAL_LINES.fetch_add(1, Ordering::Relaxed);
        log(&format!("m6: {l}"));
    }
}

unsafe fn main_player() -> usize {
    rd(va(base(), data::MAIN_PLAYER))
}

/// Sora's action and what the getter answers now (P5 or Re:CoM's bytes).
unsafe fn action_and_answer() -> (Option<u32>, u32) {
    let pl = main_player();
    let a = (pl != 0).then(|| rd::<u32>(pl + PL_ACTION));
    (a, func::<GetterFn>(ACTION_GETTER)())
}

unsafe fn task(id: u32) -> usize {
    func::<TaskFn>(TASK_BY_ID)(id)
}

/// Whether a card battle is running now (Re:CoM's CBTL task is there): false in field rooms and
/// after a battle's end, while the battle number still names the last battle (0.139.2).
pub(super) unsafe fn cbtl_live() -> bool {
    task(b::TASK_CBTL) != 0
}

/// Boss revenge (`super::revenge`): whether this battle is a KH1 battle (Combat = KH1, Sora's
/// story), and its number.
pub(super) fn kh1_battle() -> bool {
    KH1_BATTLE.load(Ordering::Acquire)
}
pub(super) fn battle_number() -> u32 {
    BATTLE_NO.load(Ordering::Relaxed)
}

/// Boss revenge: whether an enemy may start an attack as P1 lets a card play start one (alive,
/// CBTL there and not locked: `0x1401634B0`'s test). The transition itself is P1's
/// (`b::cardless_flags`, `b::state_address`).
pub(super) unsafe fn may_start_attack(e: usize) -> bool {
    let c = task(b::TASK_CBTL);
    if c == 0 || rd::<i32>(e + enemy::HP) <= 0 {
        return false;
    }
    !Gate { lock: rd(c + cbtl::LOCK), gimmick: rd(c + cbtl::GIMMICK), duel: rd(c + cbtl::DUEL) }.locked()
}

use super::stats::object_class as class_name;

/// Logs a class's path once.
unsafe fn class_once(obj: usize, path: u8, what: impl FnOnce(&str) -> String) {
    let vt: usize = if obj != 0 { rd(obj) } else { 0 };
    let new = CLASSES
        .lock()
        .map(|mut v| {
            let n = !v.contains(&(vt, path)) && v.len() < b::CLASS_LOG_MAX;
            if n {
                v.push((vt, path));
            }
            n
        })
        .unwrap_or(false);
    if new {
        let name = class_name(obj);
        log(&format!("m6: class {name}: {}", what(&name)));
    }
}

/// The enemies in Re:CoM's battle enemy manager (task type 0x1004), at most `max`.
pub(super) unsafe fn enemies(max: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let m = task(b::TASK_ENEMY_MANAGER);
    if m == 0 {
        return out;
    }
    let mut node: usize = rd(m + b::MANAGER_LIST);
    let mut guard = 0;
    while node != 0 && guard < 256 && out.len() < max {
        let obj: usize = rd(node + b::NODE_OBJECT);
        if obj != 0 {
            let vt: usize = rd(obj);
            let ty: TypeFn = std::mem::transmute_copy(&rd::<usize>(vt + enemy::VT_TYPE));
            if ty(obj) == b::ENEMY_TYPE {
                out.push(obj);
            }
        }
        node = rd(node + b::NODE_NEXT);
        guard += 1;
    }
    out
}

unsafe fn state_of(e: usize) -> Option<usize> {
    crate::guard::state_index(e, rd(e + enemy::STATE))
}

/// This battle's `enemy_pace = kh1` (each enemy attacks as if alone).
static PACE_KH1: AtomicBool = AtomicBool::new(false);

unsafe fn attacking_now() -> usize {
    enemies(256).into_iter().filter(|&e| b::attacking(rd(e + enemy::FLAGS), state_of(e))).count()
}

unsafe fn handler_of(e: usize) -> Handler {
    let vt: usize = rd(e);
    let h: usize = rd(vt + enemy::VT_MESSAGE);
    let bs = base();
    if h == bs + b::HANDLER_GENERIC as usize {
        Handler::Generic
    } else if h == bs + b::HANDLER_MARLUXIA3 as usize {
        Handler::Marluxia3
    } else {
        Handler::Other
    }
}

/// Re:CoM's Heartless card play (`0x140198870`). In a KH1 battle: message 2's work, no card.
pub unsafe extern "win64" fn card_play(e: usize) -> u64 {
    let o: PlayFn = orig(&ORIG_CARD_PLAY);
    if !KH1_BATTLE.load(Ordering::Acquire) {
        return o(e);
    }
    let c = task(b::TASK_CBTL);
    let gate = (c != 0).then(|| Gate { lock: rd(c + cbtl::LOCK), gimmick: rd(c + cbtl::GIMMICK), duel: rd(c + cbtl::DUEL) });
    let handler = handler_of(e);
    let p = Play {
        kh1_battle: true,
        flags: rd(e + enemy::FLAGS),
        hp: rd(e + enemy::HP),
        chosen: rd(e + enemy::CHOSEN),
        handler,
        marluxia3_mode: if handler == Handler::Marluxia3 { rd(e + enemy::MARLUXIA3_MODE) } else { 0 },
        gate,
    };
    match b::decide(&p) {
        Path::Cardless { flags, state } => {
            // 0.175.0: Re:CoM's pace without cards: one ordinary enemy attacking at a time.
            if b::one_at_a_time(PACE_KH1.load(Ordering::Relaxed), boss::is_boss_class(&class_name(e)), attacking_now().saturating_sub(usize::from(b::attacking(p.flags, state_of(e))))) {
                capped(&REFUSAL_LINES, b::REFUSAL_LOG_LINES, || format!("Re:CoM's pace: {} id={} waits (another enemy is attacking)", class_name(e), rd::<u32>(e + enemy::ID)));
                return 0;
            }
            wr(e + enemy::FLAGS, flags);
            wr(e + enemy::NEXT_STATE, b::state_address(e, state));
            class_once(e, 0, |_| "attacks without a card (P1)".into());
            boss_capped(|| {
                let name = class_name(e);
                boss::is_boss_class(&name).then(|| {
                    format!("boss attack without a card (battle {}): {name} id={} state={state} hp={} cooldown={}", BATTLE_NO.load(Ordering::Relaxed), rd::<u32>(e + enemy::ID), p.hp, rd::<i32>(e + enemy::COOLDOWN))
                })
            });
            capped(&ATTACK_LINES, b::ATTACK_LOG_LINES, || {
                format!(
                    "attack without a card: {} id={} state={state} attacking_now={} flags {:#x}->{flags:#x} cooldown={}",
                    class_name(e),
                    rd::<u32>(e + enemy::ID),
                    attacking_now(),
                    p.flags,
                    rd::<i32>(e + enemy::COOLDOWN)
                )
            });
            1
        }
        Path::Recom(why) => {
            match why {
                Why::Locked(g) => capped(&REFUSAL_LINES, b::REFUSAL_LOG_LINES, || {
                    format!("CBTL locked (+0x208={:#x}, gimmick={}, duel={}): Re:CoM refuses {} id={} as it refuses a card", g.lock, g.gimmick, g.duel, class_name(e), rd::<u32>(e + enemy::ID))
                }),
                why => {
                    class_once(e, 1, |_| format!("Re:CoM's card play ({why:?}); whether its card shows: the `boss card` / `side-2` lines"));
                    boss_capped(|| {
                        let name = class_name(e);
                        boss::is_boss_class(&name).then(|| format!("boss takes Re:CoM's card play (battle {}, {why:?}): {name} id={} state={}", BATTLE_NO.load(Ordering::Relaxed), rd::<u32>(e + enemy::ID), p.chosen))
                    });
                }
            }
            o(e)
        }
    }
}

/// Re:CoM's enemy attack end (`0x14017E540(enemy, next, cooldown)`, once per finished attack;
/// `crate::battle::pace`). In a KH1 battle, an ordinary Heartless on Re:CoM's own AI keeps the
/// cooldown its class passed instead of the 250 Re:CoM writes without a next state. Everything
/// else (bosses, our KH1-AI enemies, Re:CoM battles) passes through untouched.
pub unsafe extern "win64" fn attack_end(e: usize, next: usize, cooldown: i32) {
    orig::<AttackEndFn>(&ORIG_ENEMY_ATTACK_END)(e, next, cooldown);
    if !KH1_BATTLE.load(Ordering::Acquire) || e == 0 {
        return;
    }
    let vt: usize = rd(e);
    let class = vt.checked_sub(base()).and_then(|r| b::pace::heartless(r as u64));
    let cage = is_cage(vt);
    let end = b::pace::End {
        kh1_battle: true,
        heartless: class.is_some(),
        cage,
        next,
        passed: cooldown,
        flags_after: rd(e + enemy::FLAGS),
        cooldown_after: rd(e + enemy::COOLDOWN),
    };
    if let Some(cd) = b::pace::kept(&end) {
        wr(e + enemy::COOLDOWN, cd);
        if cage {
            once_per_battle(&CAGE_WAIT_LOGGED, |n| b::cage::wait_line(n, cooldown));
        } else {
            class_once(e, 3, |_| b::pace::class_line(cd));
        }
    }
    if is_oogie(vt) {
        oogie_after_end(e);
    }
}

/// Oogie's vtable (absolute), once [`install_oogie`] checked it and wrote his state-13 slot; 0: he
/// stays Re:CoM's at KH1.
static OOGIE_VT: AtomicUsize = AtomicUsize::new(0);
/// His Re:CoM state-13 begin (`0x1403D7A40`, absolute).
static OOGIE_BROKEN_BEGIN: AtomicUsize = AtomicUsize::new(0);
/// Unbroken dice throws since the last break, and the battle they were counted in.
static OOGIE_COUNT: AtomicU32 = AtomicU32::new(0);
static OOGIE_BATTLE: AtomicU32 = AtomicU32::new(0);
/// The boss we just sent Re:CoM's card break (his state-13 begin then reads it as ours).
static OOGIE_SENT: AtomicUsize = AtomicUsize::new(0);
/// Oogie's lines this battle (capped).
static OOGIE_LINES: AtomicU32 = AtomicU32::new(0);
const OOGIE_LOG_LINES: u32 = 100;

type MessageFn = unsafe extern "win64" fn(usize, i32, usize);
type BeginFn = unsafe extern "win64" fn(usize);

fn is_oogie(vt: usize) -> bool {
    let o = OOGIE_VT.load(Ordering::Relaxed);
    o != 0 && vt == o
}

fn oogie_line(line: impl FnOnce(u32) -> String) {
    let n = BATTLE_NO.load(Ordering::Relaxed);
    if OOGIE_LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |k| (k < OOGIE_LOG_LINES).then_some(k + 1)).is_ok() {
        log(&line(n));
    }
}

/// The unbroken throws counted this battle (0 in a new battle).
fn oogie_count() -> u32 {
    let n = BATTLE_NO.load(Ordering::Relaxed);
    if OOGIE_BATTLE.swap(n, Ordering::Relaxed) != n {
        OOGIE_COUNT.store(0, Ordering::Relaxed);
    }
    OOGIE_COUNT.load(Ordering::Relaxed)
}

/// After Re:CoM's attack end on Oogie in a KH1 battle (`crate::battle::oogie`, owner 2026-11-03):
/// his dice throw (state 2) ended unbroken → once [`oogie::THROWS_TO_LOWER`] are counted, Re:CoM's
/// own card break: his message handler (`+0x48`, Re:CoM's base `0x140181620`, checked) with message 3
/// and no record, as CBTL's message manager delivers a broken card. Its case 3 sets his next state to
/// 13 over the attack end's state 1, so his state 2 goes straight to his state-13 begin with state
/// 2's mark still set, and Re:CoM's begin lowers the gate. Once per attack end: a few reads.
unsafe fn oogie_after_end(e: usize) {
    use crate::battle::oogie as oo;
    let gate_obj: usize = rd(e + oo::GATE_OFF);
    let gate: i32 = if gate_obj != 0 { rd(gate_obj + oo::GATE_COUNT) } else { 0 };
    let end = oo::End {
        kh1_battle: true,
        oogie: true,
        dice_state: rd::<usize>(e + enemy::STATE) == e + crate::gimmick::state_at(oo::DICE_STATE),
        ended: rd::<u64>(e + enemy::FLAGS) & b::bits::PLAYING == 0,
        alive: rd::<i32>(e + enemy::HP) > 0 && gate_obj != 0,
        marked: rd::<u32>(e + oo::MARK_OFF) & oo::MARK_BIT != 0,
        gate,
    };
    let (count, step) = oo::after_end(end, oogie_count());
    OOGIE_COUNT.store(count, Ordering::Relaxed);
    let shown: i32 = if gate_obj != 0 { rd(gate_obj + oo::GATE_SHOWN) } else { 0 };
    match step {
        oo::Step::Pass => {}
        oo::Step::Count(n) => oogie_line(|b| oo::count_line(b, n, gate)),
        oo::Step::Down => oogie_line(|b| oo::down_line(b, gate)),
        oo::Step::Break => {
            let handler: MessageFn = std::mem::transmute(rd::<usize>(rd::<usize>(e) + enemy::VT_MESSAGE));
            OOGIE_SENT.store(e, Ordering::Relaxed);
            handler(e, oo::BREAK_MESSAGE, 0);
            let sent = rd::<usize>(e + enemy::NEXT_STATE) == e + crate::gimmick::state_at(oo::BROKEN_STATE);
            if !sent {
                OOGIE_SENT.store(0, Ordering::Relaxed);
            }
            oogie_line(|b| oo::break_line(b, gate, shown, sent));
        }
    }
}

/// Oogie's state-13 begin slot (`+0x220`, ours since [`install_oogie`]): Re:CoM's `0x1403D7A40` as
/// is, with a log line of what it did to the gate in a KH1 battle, and the count of unbroken throws
/// started again (a real break or ours). Once per card break.
pub unsafe extern "win64" fn oogie_broken_begin(e: usize) {
    use crate::battle::oogie as oo;
    let theirs: BeginFn = std::mem::transmute(OOGIE_BROKEN_BEGIN.load(Ordering::Acquire));
    if !KH1_BATTLE.load(Ordering::Acquire) {
        return theirs(e);
    }
    let ours = OOGIE_SENT.swap(0, Ordering::Relaxed) == e;
    let marked = rd::<u32>(e + oo::MARK_OFF) & oo::MARK_BIT != 0;
    let gate_obj: usize = rd(e + oo::GATE_OFF);
    let before: i32 = if gate_obj != 0 { rd(gate_obj + oo::GATE_COUNT) } else { 0 };
    theirs(e);
    let after: i32 = if gate_obj != 0 { rd(gate_obj + oo::GATE_COUNT) } else { 0 };
    let down: f32 = rd(e + oo::DOWN_TIME_OFF);
    OOGIE_COUNT.store(oo::after_break(ours, oogie_count()), Ordering::Relaxed);
    oogie_line(|b| oo::broken_line(b, ours, marked, before, after, down));
}

/// Checks Oogie's vtable by its RTTI name, the Re:CoM code the rule relies on and his slots, then
/// writes his state-13 begin slot with [`oogie_broken_begin`] (old pointer checked; no code bytes
/// move). Needs the attack-end wrapper ([`install_pace`]). Without it he stays Re:CoM's at KH1.
pub fn install_oogie(bs: usize) -> bool {
    use crate::battle::oogie as oo;
    let vt = bs + oo::VTABLE as usize;
    // SAFETY: reads of the mapped exe's vtable, its RTTI locator and the checked sites.
    let ok = unsafe { super::stats::rtti_name(bs, vt) }.as_deref() == Some(oo::RTTI)
        && crate::sites::OOGIE_CHECKED.iter().all(|s| unsafe { std::slice::from_raw_parts((bs + s.rva as usize) as *const u8, s.expect.len()) } == s.expect)
        && crate::sites::OOGIE_PTRS.iter().all(|p| unsafe { rd::<usize>(bs + p.rva as usize) } == bs + p.target as usize);
    if !ok {
        return false;
    }
    let p = crate::sites::OOGIE_BROKEN_SLOT;
    let theirs = bs + p.target as usize;
    OOGIE_BROKEN_BEGIN.store(theirs, Ordering::Release);
    let ours = oogie_broken_begin as *const () as usize;
    // SAFETY: the slot's old pointer is checked inside before the write.
    if unsafe { super::hook::patch(bs + p.rva as usize, &(theirs as u64).to_le_bytes(), &(ours as u64).to_le_bytes(), p.name) } {
        OOGIE_VT.store(vt, Ordering::Release);
        true
    } else {
        OOGIE_BROKEN_BEGIN.store(0, Ordering::Release);
        false
    }
}

/// The Parasite Cage's vtable (absolute), once its RTTI name was checked ([`install_cage`]); 0: not
/// checked, the cage stays Re:CoM's.
static CAGE_VT: AtomicUsize = AtomicUsize::new(0);
/// The battle number its lines were last logged for.
static CAGE_ARMOUR_LOGGED: AtomicU32 = AtomicU32::new(0);
static CAGE_WAIT_LOGGED: AtomicU32 = AtomicU32::new(0);

fn is_cage(vt: usize) -> bool {
    let c = CAGE_VT.load(Ordering::Relaxed);
    c != 0 && vt == c
}

/// `what(battle)` logged once per battle (`last`: the battle it was last logged for).
fn once_per_battle(last: &AtomicU32, what: impl FnOnce(u32) -> String) {
    let n = BATTLE_NO.load(Ordering::Relaxed);
    if last.swap(n, Ordering::Relaxed) != n {
        log(&what(n));
    }
}

/// The Parasite Cage's flags before an armoured call of the hit handler.
pub(super) struct CageArmour {
    e: usize,
    flags: u64,
}

/// Before Re:CoM's hit handler (`super::attack::enemy_hit`, which every call of `0x14017F7E0` goes
/// through): in a KH1 battle, the Parasite Cage takes Re:CoM's own super armour (bits 20 / 21) for
/// the call (`crate::battle::cage`, owner 2026-11-03), unless Sora's Guard blocked it this frame
/// (the block's reaction runs inside the call). Per call: one atomic load while the cage isn't
/// checked or the battle isn't a KH1 one, else one vtable read.
pub(super) unsafe fn cage_armour_on(e: usize) -> Option<CageArmour> {
    if CAGE_VT.load(Ordering::Relaxed) == 0 || !KH1_BATTLE.load(Ordering::Acquire) || e == 0 || !is_cage(rd(e)) {
        return None;
    }
    let ask = b::cage::Ask { kh1_battle: true, cage: true, guard_block: super::guard::block_pending(e) };
    if !b::cage::armoured(ask) {
        return None;
    }
    let flags: u64 = rd(e + enemy::FLAGS);
    wr(e + enemy::FLAGS, b::cage::flags_for_call(flags));
    Some(CageArmour { e, flags })
}

/// After the call: Re:CoM's two bits back as they were; the battle's line once a hit landed.
pub(super) unsafe fn cage_armour_off(a: Option<CageArmour>, result: u64) {
    let Some(CageArmour { e, flags }) = a else { return };
    let now: u64 = rd(e + enemy::FLAGS);
    wr(e + enemy::FLAGS, b::cage::flags_after(now, flags));
    if result == 1 {
        once_per_battle(&CAGE_ARMOUR_LOGGED, b::cage::armour_line);
    }
}

/// Checks the Parasite Cage's vtable by its RTTI name (nothing written; no hook of its own: the
/// armour rides the enemy-hit wrapper, the wait the attack-end wrapper). Without it the cage stays
/// Re:CoM's at KH1 (it flinches, 250 ticks between attacks).
pub fn install_cage(bs: usize) -> bool {
    let vt = bs + b::cage::VTABLE as usize;
    // SAFETY: reads of the mapped exe's vtable, its RTTI locator and the checked sites.
    let ok = unsafe { super::stats::rtti_name(bs, vt) }.as_deref() == Some(b::cage::RTTI)
        && crate::sites::CAGE_CHECKED.iter().all(|s| unsafe { std::slice::from_raw_parts((bs + s.rva as usize) as *const u8, s.expect.len()) } == s.expect)
        && crate::sites::CAGE_PTRS.iter().all(|p| unsafe { rd::<usize>(bs + p.rva as usize) } == bs + p.target as usize);
    if ok {
        CAGE_VT.store(vt, Ordering::Release);
        // SAFETY: the callback's bytes and the slot's old pointer are checked before the write.
        if unsafe { install_cage_faint(bs) } {
            log("m6: Parasite Cage hit callback 0x1403C4060 wrapped (its vtable +0x3E0): in a KH1 battle its faint (state 17) only from Fire; a finisher-type hit that isn't Fire takes no AtkFaint roll");
        } else {
            log("m6: Parasite Cage hit callback not as read: its faint stays Re:CoM's (Fire or a finisher-type hit, by chance)");
        }
    }
    ok
}

/// Re:CoM's hit callback of the cage (absolute), once its slot holds ours; 0: not installed.
static CAGE_CALLBACK: AtomicUsize = AtomicUsize::new(0);
static CAGE_SKIP_LOGGED: AtomicU32 = AtomicU32::new(0);
static CAGE_FIRE_LOGGED: AtomicU32 = AtomicU32::new(0);

type CallbackFn = unsafe extern "win64" fn(usize);

/// The cage's hit callback slot (`+0x3E0`, `sites::CAGE_FAINT_SLOT`) written with
/// [`cage_hit_callback`], after its first bytes and the reads the rule mirrors
/// (`sites::CAGE_FAINT_CHECKED`) and the slot's old pointer are checked. No code bytes move.
unsafe fn install_cage_faint(bs: usize) -> bool {
    if let Some(s) = crate::sites::CAGE_FAINT_CHECKED.iter().find(|s| std::slice::from_raw_parts((bs + s.rva as usize) as *const u8, s.expect.len()) != s.expect) {
        log(&format!("m6: {} holds other bytes", s.name));
        return false;
    }
    let p = crate::sites::CAGE_FAINT_SLOT;
    let theirs = bs + p.target as usize;
    CAGE_CALLBACK.store(theirs, Ordering::Release);
    let ours = cage_hit_callback as *const () as usize;
    if super::hook::patch(bs + p.rva as usize, &(theirs as u64).to_le_bytes(), &(ours as u64).to_le_bytes(), p.name) {
        true
    } else {
        CAGE_CALLBACK.store(0, Ordering::Release);
        false
    }
}

/// The cage's hit callback (its vtable `+0x3E0`; called by Re:CoM's hit handler `0x14017F7E0` before
/// the reaction picker, and by its stop exe `0x1401843A0` for a held hit). At Combat = KH1 its faint
/// only from Fire (`crate::battle::cage::callback`): a finisher-type hit (body kind 2) that isn't
/// Fire skips the call (its only work there is the `AtkFaint` roll); our KH1 Fire's hit (attribute
/// neutral for KH1's damage) reads as Fire for the call. Combat = Re:CoM, or in its faint: Re:CoM's.
/// Per hit on the cage: one atomic load at Combat = Re:CoM, else a few reads.
pub unsafe extern "win64" fn cage_hit_callback(e: usize) {
    let theirs: CallbackFn = std::mem::transmute(CAGE_CALLBACK.load(Ordering::Acquire));
    if !KH1_BATTLE.load(Ordering::Acquire) {
        return theirs(e);
    }
    let info: usize = rd(e + 0xF8);
    let body: usize = if info != 0 { rd(info + 0x50) } else { 0 };
    if body == 0 {
        return theirs(e);
    }
    let attr: u32 = rd(body + super::magic::PART_ATTR);
    let neutral_fire = attr == super::magic::NEUTRAL_ATTR && super::magic::shot_spell(rd(body + 0x98)) == Some(crate::magic::spell::FIRE);
    let ask = b::cage::Faint {
        kh1_battle: true,
        fainted: rd::<u8>(e + b::cage::FAINTED_OFF) & b::cage::FAINTED_BIT != 0,
        fire: attr == b::cage::FIRE_ATTR,
        neutral_fire,
        finisher: rd::<u32>(body + 0x80) == b::cage::FINISHER_KIND,
    };
    match b::cage::callback(ask) {
        b::cage::Callback::Run => theirs(e),
        b::cage::Callback::RunAsFire => {
            wr(body + super::magic::PART_ATTR, b::cage::FIRE_ATTR);
            theirs(e);
            wr(body + super::magic::PART_ATTR, attr);
            once_per_battle(&CAGE_FIRE_LOGGED, b::cage::faint_fire_line);
        }
        b::cage::Callback::Skip => once_per_battle(&CAGE_SKIP_LOGGED, b::cage::faint_skip_line),
    }
}

/// Installs the attack-end wrapper on its own (a failure leaves Re:CoM's 250 and the rest of M6).
/// Checks the 32 classes' vtables by their RTTI names first.
pub fn install_pace(bs: usize) -> bool {
    // SAFETY: reads of the mapped exe's vtables and their RTTI locators.
    if let Some(h) = b::pace::HEARTLESS.iter().find(|h| unsafe { super::stats::rtti_name(bs, bs + h.vtable as usize) }.as_deref() != Some(h.rtti)) {
        log(&format!("m6: {}'s vtable is not at {:#x}: attack-end cooldown not hooked", h.rtti, 0x1_4000_0000u64 + h.vtable as u64));
        return false;
    }
    let hooks = [super::hook::Hook { site: &crate::sites::M6_PACE_HOOKS[0], ours: attack_end as *const () as usize, original: &ORIG_ENEMY_ATTACK_END }];
    // SAFETY: the target's bytes are checked again inside, before anything is written.
    unsafe { super::hook::install_all(bs, &hooks) }
}

/// Re:CoM's battle enemy manager ctor (once per battle): the battle's setting and P2–P4.
pub unsafe extern "win64" fn manager_ctor(this: usize, a: usize, c: usize, d: usize) -> usize {
    // Riku with `riku_kh1_movement` (`crate::riku`): his battles are KH1 battles too (cardless enemies).
    let kh1 = mv::kh1_on() && (!super::stats::riku_story() || mv::riku_on());
    let pace_kh1 = super::settings().enemy_pace == crate::settings::Mode::Kh1;
    PACE_KH1.store(pace_kh1, Ordering::Relaxed);
    let want = b::wanted(kh1, pace_kh1);
    let switched = switch_patches(M6_PATCHES, &want);
    // Stage 3, P5: "Sora is attacking" for the bosses, on its own (a foreign getter leaves P2-P4).
    let p5 = switch_patches(M6_BOSS_PATCHES, &[kh1]);
    KH1_BATTLE.store(kh1, Ordering::Release);
    super::dark_points::battle_start();
    let battle = BATTLE_NO.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
    BOSS_BATTLE_LINES.store(0, Ordering::Relaxed);
    SIDE2_CARDS.store(0, Ordering::Relaxed);
    END_LINES.store(0, Ordering::Relaxed);
    OOGIE_LINES.store(0, Ordering::Relaxed);
    capped(&BATTLE_LINES, b::BATTLE_LOG_LINES, || {
        format!(
            "battle start (battle {battle}): {}; no-card attacks (P1) {}; think gate (P2) and grant all (P3): {}; crowd pace (P4): {}; Sora attacking for bosses (P5): {}",
            if kh1 && super::stats::riku_story() { "KH1 (Combat = KH1, Riku's story)" } else if kh1 { "KH1 (Combat = KH1, Sora's story)" } else if mv::kh1_on() { "Re:CoM (Riku's story)" } else { "Re:CoM (Combat = Re:CoM)" },
            if kh1 { "on" } else { "off" },
            match switched {
                Some(_) if want[0] => "ours",
                Some(_) => "Re:CoM's",
                None => "UNCHANGED: a site held unexpected bytes (see above); Re:CoM's turn-taking may stay",
            },
            match switched {
                Some(n) => format!(
                    "{} ({n} site(s) written this battle)",
                    if want[2] { "ours, each enemy as if alone (enemy_pace = kh1)" } else if kh1 { "Re:CoM's crowd sharing (enemy_pace = recom)" } else { "Re:CoM's" }
                ),
                None => "UNCHANGED".into(),
            },
            boss::p5_part(kh1, p5)
        )
    });
    // Sora's damage cap: no slot carries over from the last battle.
    super::damage_cap::battle_start();
    // M13 stage 1: once per session, whether Re:CoM has the KH1 effect sets (a log line).
    super::effects::battle_check();
    // M13 (0.131.0): the per-battle finisher lines start again.
    super::fx::battle_start();
    // M13 stage 2: KH1's sound banks in Re:CoM's list (registered when missing; combat = kh1).
    super::sounds::battle_start();
    // Sleights outside battle (owner, 2026-11-04): a field sleight its first strike cut is over.
    super::field_sleights::battle_start();
    // Enemy Skills stage 5: Black Fungus's skill and the deck skills' KH1 meanings for this battle
    // (before stage 4's arming, which counts Black Fungus's skill).
    super::enemy_skills_deck::battle_start(kh1);
    // Enemy Skills stage 4: the direct compares' bytes for this battle.
    super::enemy_skills_cmp::battle_start(kh1);
    // Enemy Skills: Re:CoM's enemy-card numbers the owner changed, per battle (`enemy_skills`).
    super::enemy_skills::battle_start(kh1);
    orig::<CtorFn>(&ORIG_MANAGER_CTOR)(this, a, c, d)
}

/// Makes each patch of `patches` ours or Re:CoM's as `want` says, all or none (P2–P4 together,
/// P5 on its own). The number of sites written, or None.
unsafe fn switch_patches(patches: &[Patch], want: &[bool]) -> Option<usize> {
    let bs = base();
    let now: Vec<Bytes> = patches
        .iter()
        .map(|p| b::bytes_state(p, std::slice::from_raw_parts((bs + p.site.rva as usize) as *const u8, p.site.expect.len())))
        .collect();
    let pairs: Vec<(&[u8], &[u8])> = patches.iter().map(|p| (p.site.expect, p.new)).collect();
    let Some(writes) = b::plan_each(&pairs, &now, want) else {
        for (p, s) in patches.iter().zip(&now) {
            if *s == Bytes::Other {
                log(&format!("m6: {} holds bytes that are neither Re:CoM's nor ours; no patch switched", p.site.name));
            }
        }
        return None;
    };
    let mut done: Vec<b::Write> = Vec::new();
    for (i, from, to) in writes {
        let p = &patches[i];
        if !super::hook::patch(bs + p.site.rva as usize, from, to, p.site.name) {
            // Undo what this call wrote, so the set stays together.
            for &(j, from2, to2) in done.iter().rev() {
                let q = &patches[j];
                super::hook::patch(bs + q.site.rva as usize, to2, from2, q.site.name);
            }
            log(&format!("m6: a patch write failed; {} left as they were", patches.iter().map(|p| p.site.name).collect::<Vec<_>>().join(", ")));
            return None;
        }
        done.push((i, from, to));
    }
    let n = done.len();
    Some(n)
}

/// CBTL::setPlayCard (`0x140163830(cbtl, owner id, side, card, item, …)`): logs, once per class,
/// who still sets a side-2 card in a KH1 battle. Pass-through.
pub unsafe extern "win64" fn set_play_card(cb: usize, id: u32, side: i32, card: usize, item: usize, p6: i32) -> u64 {
    let r = orig::<SetPlayFn>(&ORIG_SET_PLAY_CARD)(cb, id, side, card, item, p6);
    if side == 2 && r as i32 > 0 && KH1_BATTLE.load(Ordering::Relaxed) {
        let obj = func::<mv::HandleFn>(mv::HANDLE_OBJ)(id);
        class_once(obj, 2, |_| format!("plays a side-2 card at KH1 (Re:CoM's card path: id {id}, {})", if card != 0 { "a revolver card" } else { "a new card" }));
        // Stage 3: each side-2 card of this battle (a human boss's move list at work).
        let n = SIDE2_CARDS.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
        boss_capped(|| {
            let (a, ans) = action_and_answer();
            Some(boss::card_line(&boss::CardLine {
                battle: BATTLE_NO.load(Ordering::Relaxed),
                n,
                class: &class_name(obj),
                id,
                revolver_card: card != 0,
                action: a.unwrap_or(0),
                answer: ans,
                duel_mode: rd(cb + cbtl::DUEL),
                hidden: super::deck::enemy_ui_hidden(),
            }))
        });
    }
    r
}

/// Sora's card-duel init (`0x140220C40(player, start)`, action 0x24; Marluxia 3's countdown).
/// Log only, pass-through.
pub unsafe extern "win64" fn duel_init(pl: usize, start: i32) {
    if start != 0 && KH1_BATTLE.load(Ordering::Relaxed) {
        let rev: usize = rd(pl + crate::deck::player::REVOLVER);
        let cards: i16 = if rev != 0 { rd(rev + crate::deck::revolver::CARDS) } else { 0 };
        capped(&DUEL_LINES, boss::DUEL_LOG_LINES, || boss::duel_line(BATTLE_NO.load(Ordering::Relaxed), cards));
    }
    orig::<DuelInitFn>(&ORIG_DUEL_INIT)(pl, start)
}

/// The trace's M6 token (F9 recording only).
pub unsafe fn trace_token() -> String {
    let es = enemies(256);
    let attacking = es.iter().filter(|&&e| b::attacking(rd(e + enemy::FLAGS), state_of(e))).count();
    let cells: Vec<String> = es
        .iter()
        .take(b::TRACE_ENEMIES)
        .map(|&e| b::trace_enemy(rd(e + enemy::ID), state_of(e), rd(e + enemy::FLAGS), rd(e + enemy::COOLDOWN)))
        .collect();
    let c = task(b::TASK_CBTL);
    let slot = (c != 0).then(|| rd::<i32>(c + cbtl::SLOT_ENEMY));
    let m6 = b::trace_token(KH1_BATTLE.load(Ordering::Relaxed), attacking, slot, &cells);
    // Stage 3: P5's bytes now, Sora's action, what the bosses read, CBTL's duel mode, the enemy
    // card UI.
    let p = &M6_BOSS_PATCHES[0];
    let have = std::slice::from_raw_parts((base() + p.site.rva as usize) as *const u8, p.site.expect.len());
    let p5 = match b::bytes_state(p, have) {
        Bytes::Ours => "ours",
        Bytes::Recom => "recom",
        Bytes::Other => "other",
    };
    let (a, ans) = action_and_answer();
    let duel = (c != 0).then(|| rd::<u32>(c + cbtl::DUEL));
    let end_t = end_view_now().map(|v| format!("; {}", be::token(&v))).unwrap_or_default();
    // M10 / M11: the KH1 enemies' requests, reactions and actions (`shadow …; soldier …`).
    let shadows = super::kh1_enemy::trace_token();
    let shadows = if shadows.is_empty() { String::new() } else { format!("; {shadows}") };
    format!("{m6}; {}{end_t}{shadows}", boss::trace_token(p5, a, ans, duel, super::deck::enemy_ui_hidden()))
}

// ---- Re:CoM's battle end, read only (`crate::battle::end`) ----

/// Re:CoM functions and data the watch reads (`sites::BATTLE_END_CHECKED`; the getters are checked by `M6_DECK_CALLEES` and `KEYBLADE_CALLEES`).
const FLAG_GETTER: u32 = 0x001B_0870;
const VAR_GETTER: u32 = 0x001B_08C0;
const LAYER_DONE: u32 = 0x0024_80C0;
/// `Gauge::CPrizeGauge`'s singleton (`0x140209830`'s load) and the gauge pause (`0x140232050`'s).
const PRIZE_GAUGE: u64 = 0x1_4087_B690;
const GAUGE_PAUSE: u64 = 0x1_4087_B808;
/// `CTask` (ctor `0x1402AB850`, step `0x1402AC3C0`): time banked, the last step it ran with,
/// flags (u16; 0x10 off, 0x8000 dying: its update and its children's are skipped), the parent.
mod task_f {
    pub const BANKED: usize = 0x10;
    pub const STEP: usize = 0x14;
    pub const FLAGS: usize = 0x18;
    pub const PARENT: usize = 0x28;
}
/// `CPrizeGauge`'s controls: the EXP display's (`+0x70`, `0x14020A710`) and the get-card
/// display's (`+0xA0`, `0x1402094F0`). Checked (decomp).
const PRIZE_EXP_CTRL: usize = 0x70;
const PRIZE_CARD_CTRL: usize = 0xA0;
/// The C2DSeqCtrl's layer count and first layer's instance id (`0x1402480C0`).
const CTRL_LAYERS: usize = 0x16D;
const CTRL_LAYER0: usize = 0x144;

type FlagFn = unsafe extern "win64" fn(u32, u32) -> u8;
type VarFn = unsafe extern "win64" fn(u32, u32) -> u64;
type LayerDoneFn = unsafe extern "win64" fn(usize, u32) -> u64;

static END_LINES: AtomicU32 = AtomicU32::new(0);
/// The watch's last key and the frames since it last logged; None when no battle end is going.
static END_STATE: Mutex<Option<(be::Key, u32)>> = Mutex::new(None);
/// A battle end is being watched (the per-frame test's only state otherwise).
static END_ON: AtomicBool = AtomicBool::new(false);

fn end_line(line: impl FnOnce() -> String) {
    if END_LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| (n < be::LOG_LINES).then_some(n + 1)).is_ok() {
        log(&format!("battle end: {}", line()));
    }
}

/// Re:CoM's battle end as it is now, if the main player is in it (win pending or win action).
unsafe fn end_view_now() -> Option<be::View> {
    let pl = main_player();
    if pl == 0 {
        return None;
    }
    let pending = rd::<u32>(pl + mv::p::CARD) & be::WIN_PENDING != 0;
    let action: u32 = rd(pl + mv::p::ACTION);
    (pending || action == be::WIN_ACTION).then(|| end_view(pl))
}

unsafe fn end_view(pl: usize) -> be::View {
    let bs = base();
    let m = task(b::TASK_ENEMY_MANAGER);
    let manager = (m != 0).then(|| be::Manager {
        state: rd(m + be::manager::STATE),
        timer: rd(m + be::manager::TIMER),
        children: rd(m + be::manager::CHILDREN),
        count: rd(m + be::manager::COUNT),
        roulette: (rd(m + be::manager::ROULETTE_A), rd(m + be::manager::ROULETTE_B)),
    });
    let g: usize = rd(va(bs, PRIZE_GAUGE));
    be::View {
        action: rd(pl + mv::p::ACTION),
        sub: rd(pl + mv::p::ACTION_SUB),
        phase: rd(pl + mv::p::PHASE),
        dead: rd::<i32>(pl + be::DEAD) != 0,
        win_pending: rd::<u32>(pl + mv::p::CARD) & be::WIN_PENDING != 0,
        manager,
        prize: (g != 0).then(|| (rd(g + be::prize::CARD), rd(g + be::prize::EXP))),
        gauge_pause: rd(va(bs, GAUGE_PAUSE)),
        roulette_task: task(be::TASK_ROULETTE) != 0,
        over: func::<FlagFn>(FLAG_GETTER)(1, 2) != 0,
        hud_mask: func::<VarFn>(VAR_GETTER)(1, 0xE2) as u32,
    }
}

/// A task's step state: (flags, banked, last step), and its parent's flags.
unsafe fn task_line(t: usize) -> String {
    if t == 0 {
        return "none".into();
    }
    let parent: usize = rd(t + task_f::PARENT);
    let pf = if parent != 0 { format!("{:#x}", rd::<u16>(parent + task_f::FLAGS)) } else { "-".into() };
    format!(
        "{} flags {:#x} banked {} step {} parent {} flags {pf}",
        class_name(t),
        rd::<u16>(t + task_f::FLAGS),
        rd::<i32>(t + task_f::BANKED),
        rd::<i32>(t + task_f::STEP),
        if parent != 0 { class_name(parent) } else { "none".into() }
    )
}

/// A C2DSeqCtrl's first layer: (layers, instance id, ended).
unsafe fn ctrl_line(c: usize) -> String {
    if c == 0 {
        return "none".into();
    }
    format!("layers {} id {:#x} done {}", rd::<u8>(c + CTRL_LAYERS), rd::<u32>(c + CTRL_LAYER0), func::<LayerDoneFn>(LAYER_DONE)(c, 0) != 0)
}

/// The player prizes home to (`0x140231010`, `0x140230C70`: `DAT_14087B380`). Checked (decomp).
const PRIZE_PLAYER: u64 = 0x1_4087_B380;

/// A point, "NON-FINITE" when it is one (`crate::magic::strike_contact::finite`).
fn v3(p: [f32; 3]) -> String {
    let s = format!("({:.2},{:.2},{:.2})", p[0], p[1], p[2]);
    if crate::magic::strike_contact::finite(p) {
        s
    } else {
        format!("{s} NON-FINITE")
    }
}

/// One `CPrize` (kinds ≥ 0xCB step in `0x140231010`; Checked (decomp)): its state `+0x240`, kind
/// `+0x244`, amount `+0x248`, launch speed `+0x24C`, effect `+0x250`, `+0x254`, "may be pulled"
/// `+0x258`, "arrived" `+0x25C` (set once within 2 m of the prize target: the pickup's start),
/// reward mode `+0x260`; its timer `+0x160`, step `+0x14`; its mover (`+0xF0`): position `+0x30`,
/// velocity `+0x80`, flags `+0x118` (1: no map collision, 4: gravity, 8: frozen) and contact
/// `+0x120`.
unsafe fn prize_line(obj: usize) -> String {
    let w: Vec<String> = (0..9).map(|i| format!("{:x}", rd::<u32>(obj + 0x240 + 4 * i))).collect();
    let m: usize = rd(obj + 0xF0);
    let mover = if m != 0 {
        format!(
            "at {} v {} mover {:#x}/{:#x}",
            v3(rd(m + 0x30)),
            v3(rd(m + 0x80)),
            rd::<u32>(m + 0x118),
            rd::<u32>(m + 0x120)
        )
    } else {
        "no mover".into()
    };
    format!(
        "CPrize flags {:#x} kind {:#x} [{}] timer {} step {} {mover}",
        rd::<u16>(obj + task_f::FLAGS),
        rd::<i32>(obj + 0x244),
        w.join(" "),
        rd::<i32>(obj + 0x160),
        rd::<i32>(obj + task_f::STEP)
    )
}

/// What the line adds for the state Re:CoM waits in: the manager's children (state 1), the prize
/// gauge's task and displays (states 2, 3), our KH1 side.
unsafe fn end_detail(v: &be::View) -> String {
    let mut d = Vec::new();
    if let Some(m) = v.manager {
        if m.state == 1 && m.children > 0 {
            let t = task(b::TASK_ENEMY_MANAGER);
            let mut names = Vec::new();
            let mut node: usize = rd(t + b::MANAGER_LIST);
            let mut guard = 0;
            while node != 0 && guard < 16 {
                let obj: usize = rd(node + b::NODE_OBJECT);
                if obj != 0 {
                    let name = class_name(obj);
                    if name == "CPrize" {
                        names.push(prize_line(obj));
                    } else {
                        names.push(format!("{name} flags {:#x}", rd::<u16>(obj + task_f::FLAGS)));
                    }
                }
                node = rd(node + b::NODE_NEXT);
                guard += 1;
            }
            d.push(format!("manager children [{}]", names.join(", ")));
        }
    }
    let g: usize = rd(va(base(), PRIZE_GAUGE));
    if g != 0 {
        d.push(format!("prize gauge {g:#x}: {}; EXP ctrl {}; card ctrl {}", task_line(g), ctrl_line(rd(g + PRIZE_EXP_CTRL)), ctrl_line(rd(g + PRIZE_CARD_CTRL))));
    }
    // What the prizes home to and measure their arrival against (`0x140231010` state 1: the
    // prize player's vtable +0x50 = `+0x4B0`; the attraction radius `+0x550`, 50 m in the win).
    let pp: usize = rd(va(base(), PRIZE_PLAYER));
    if pp != 0 {
        let t: [f32; 3] = rd(pp + 0x4B0);
        let body: usize = rd(pp + 0xF0);
        let b: [f32; 3] = if body != 0 { rd(body + 0x30) } else { [0.0; 3] };
        d.push(format!("prize target {} body {} radius {:.1}", v3(t), v3(b), rd::<f32>(pp + 0x550)));
    }
    let (strikes, falls) = super::magic::threads_running();
    let ours = match mv::SORA.try_lock() {
        Ok(s) => s.as_ref().map_or("no KH1 frame yet".to_string(), |s| {
            format!(
                "KH1 cast {}, swing {}, lock {}",
                s.sora.cast.as_ref().map_or("none".to_string(), |c| format!("spell {} level {} age {:.0} released {}", c.start.spell, c.start.level, c.age, c.released)),
                s.sora.swing.map_or("none".to_string(), |w| format!("row {}", w.kh1_row)),
                s.sora.lock
            )
        }),
        Err(_) => "KH1 frame busy".into(),
    };
    d.push(format!("ours: {ours}; spell threads {strikes} strike(s), {falls} fall(s)"));
    // A prize display whose layer id is 0xffffffff could not get a pool entry: the pool's free
    // count says whether it ran out (`crate::d2`).
    d.push(format!("2D: {}", super::d2::census_text()));
    d.join("; ")
}

/// Sora's frame start (the request-bits wrapper, Combat = KH1): Re:CoM's battle end, logged.
/// Two loads unless Re:CoM's win is pending or Sora is in its win action.
pub unsafe fn end_watch(pl: usize) {
    let pending = rd::<u32>(pl + mv::p::CARD) & be::WIN_PENDING != 0;
    if !pending && rd::<u32>(pl + mv::p::ACTION) != be::WIN_ACTION {
        if END_ON.swap(false, Ordering::Relaxed) {
            if let Ok(mut g) = END_STATE.lock() {
                *g = None;
            }
            end_line(|| "Sora left Re:CoM's win (no longer pending, not in action 0x1c)".into());
        }
        return;
    }
    END_ON.store(true, Ordering::Relaxed);
    let v = end_view(pl);
    let key = be::key(&v);
    let Ok(mut g) = END_STATE.try_lock() else { return };
    let (log_now, first) = match g.as_mut() {
        None => (true, true),
        Some((k, _)) if *k != key => (true, false),
        Some((_, n)) => {
            *n += 1;
            (*n >= be::REPEAT_FRAMES, false)
        }
    };
    if log_now {
        *g = Some((key, 0));
        drop(g);
        let what = if first { "started" } else { "now" };
        end_line(|| format!("{what}: {}; {}", be::line(&v), end_detail(&v)));
    }
}

/// Installs the three wrappers, all or none (needs the movement handlers: the setting), then
/// stage 3's log-only duel hook apart.
pub fn install(bs: usize) -> bool {
    let site = |name: &str| crate::sites::M6_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [
        super::hook::Hook { site: site("enemy card play (0x140198870)"), ours: card_play as *const () as usize, original: &ORIG_CARD_PLAY },
        super::hook::Hook { site: site("battle enemy manager ctor (0x1401859B0)"), ours: manager_ctor as *const () as usize, original: &ORIG_MANAGER_CTOR },
        super::hook::Hook { site: site("CBTL setPlayCard (0x140163830)"), ours: set_play_card as *const () as usize, original: &ORIG_SET_PLAY_CARD },
    ];
    // SAFETY: the targets' bytes are checked again inside, before anything is written.
    if !unsafe { super::hook::install_all(bs, &hooks) } {
        return false;
    }
    log("m6: bosses (stage 3): P5 (Sora's action getter 0x14017F720) is switched with P2-P4 at each battle start; side-2 cards and boss attacks logged per battle");
    let duel = [super::hook::Hook { site: &crate::sites::M6_BOSS_HOOKS[0], ours: duel_init as *const () as usize, original: &ORIG_DUEL_INIT }];
    // SAFETY: as above.
    if unsafe { super::hook::install_all(bs, &duel) } {
        log("m6: card duel logged (0x140220C40, log only)");
    } else {
        log("m6: card duel not hooked (log only; the battle work stands)");
    }
    true
}
