//! Enemy reactions to Sora's KH1 Guard (decision 27; `crate::guard`, `docs/recom-kh1-attack.md`
//! § Enemy reactions to Guard). No hook of its own: it runs from the two existing wrappers.
//!
//! - `on_result`: from the pair-dispatch wrapper (`0x14016EAD0`), only when Re:CoM's dispatch
//!   returned non-zero. A 2 on the player side's list with Sora's body guarded by this attack is a
//!   block event (once per attack instance: the body records the serial). A shot is turned back
//!   there (its own collision, Re:CoM's setter, one answer of 0); a melee attacker is remembered.
//! - `after_enemy_hit`: from the enemy-hit wrapper (`0x14017F7E0`, which Re:CoM runs in each
//!   enemy's update right after its attack check). A remembered attacker that wasn't hit itself
//!   gets Re:CoM's own picker with kind 1 (its flinch, super armour respected), and its card ends
//!   through Re:CoM's own `0x14017E1D0` when the picker chose the flinch.
//!
//! Per frame: nothing beyond a compare of the dispatch result and two atomic loads in the hit
//! wrapper while nothing is pending.

use super::movement::{self as mv, func, rd, wr};
use super::{base, log, va};
use crate::guard::{self as g, attack_motion as am, body, bullet, coll, enemy, Retarget, Shot};
use crate::sites::data;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

/// The Config row "Guard reactions" (ini `guard_reactions`).
static ON: AtomicBool = AtomicBool::new(true);
/// Every class check passed at install.
static READY: AtomicBool = AtomicBool::new(false);
/// Melee attackers whose attack Sora's Guard blocked, until their own hit handler runs: (id,
/// hit-handler calls left).
static PENDING: Mutex<Vec<(u32, u32)>> = Mutex::new(Vec::new());
static PENDING_N: AtomicUsize = AtomicUsize::new(0);
/// Turned-back shots: (their collision, the bullet's id).
static TURNED: Mutex<Vec<(usize, u32)>> = Mutex::new(Vec::new());
static TURNED_N: AtomicUsize = AtomicUsize::new(0);
const PENDING_MAX: usize = 8;
const TURNED_MAX: usize = 16;
static BLOCKS: AtomicU32 = AtomicU32::new(0);
static HITS: AtomicU32 = AtomicU32::new(0);
/// Owner classes whose blocked attack is neither a melee entry nor a shot (logged once each).
static OTHER_LOGGED: Mutex<Vec<usize>> = Mutex::new(Vec::new());

type PickerFn = unsafe extern "win64" fn(usize, i32);
type EndCardFn = unsafe extern "win64" fn(usize) -> u64;
type SetListFn = unsafe extern "win64" fn(usize, u32);

pub(super) fn set_on(on: bool) {
    ON.store(on, Ordering::Relaxed);
}

/// Checks the classes the reactions recognise by their vtables (RTTI names) and that
/// `CCollAttack`'s `+0x20` slot is Re:CoM's list setter. False (and no reaction ever) otherwise.
pub(super) fn install(b: usize) -> bool {
    let want = [
        (coll::VTABLE, ".?AVCCollAttack@@"),
        (am::VTABLE, ".?AVCAttackMotion@@"),
        (bullet::VTABLE, ".?AVCBullet@@"),
        (bullet::HOMING_VTABLE, ".?AVCHomingBullet@@"),
    ];
    for (vt, name) in want {
        // SAFETY: inside the mapped image (the build was checked at start).
        let got = unsafe { super::stats::rtti_name(b, va(b, vt)) };
        if got.as_deref() != Some(name) {
            log(&format!("guard reactions: the vtable at {vt:#x} has RTTI {got:?}, not {name}; reactions off"));
            return false;
        }
    }
    // SAFETY: as above (a relocated pointer in .rdata).
    let slot: usize = unsafe { rd(va(b, coll::SET_LIST_SLOT)) };
    if slot != b + g::func::SET_LIST as usize {
        log(&format!("guard reactions: CCollAttack +0x20 is {slot:#x}, not 0x14014E5A0; reactions off"));
        return false;
    }
    READY.store(true, Ordering::Release);
    true
}

fn active() -> bool {
    READY.load(Ordering::Acquire) && ON.load(Ordering::Relaxed) && mv::kh1_on()
}

unsafe fn class_name(obj: usize) -> String {
    if obj == 0 {
        return "none".into();
    }
    let vt: usize = rd(obj);
    super::stats::rtti_name(base(), vt).map(|n| n.trim_start_matches(".?AV").trim_end_matches("@@").to_string()).unwrap_or_else(|| format!("vt {vt:#x}"))
}

fn block_log(line: impl FnOnce() -> String) {
    if BLOCKS.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| (n < g::BLOCK_LOG_LINES).then_some(n + 1)).is_ok() {
        log(&format!("guard reactions: {}", line()));
    }
}

fn hit_log(line: impl FnOnce() -> String) {
    if HITS.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| (n < g::HIT_LOG_LINES).then_some(n + 1)).is_ok() {
        log(&format!("guard reactions: {}", line()));
    }
}

/// From the pair-dispatch wrapper, for a non-zero result of Re:CoM's dispatch: the result to
/// hand back (0 for a shot turned back on this test, else `r`).
pub(super) unsafe fn on_result(c: usize, list: usize, r: i32) -> i32 {
    if TURNED_N.load(Ordering::Relaxed) != 0 {
        turned_hit(c, r);
    }
    if r != 2 || !active() {
        return r;
    }
    let b = base();
    if list == 0 || list != rd::<usize>(va(b, g::PLAYER_LIST)) {
        return r;
    }
    let pl: usize = rd(va(b, data::MAIN_PLAYER));
    if !mv::is_main_sora(pl) || !super::attack::guard_live(pl) {
        return r;
    }
    let owner: u32 = rd(c + coll::OWNER);
    if rd::<u32>(pl + body::SORA + body::FLAGS) & body::GUARDED == 0 || rd::<u32>(pl + body::SORA + body::ATTACKER) != owner {
        return r;
    }
    let obj = func::<mv::HandleFn>(mv::HANDLE_OBJ)(owner);
    if obj == 0 {
        other(0, c, "owner not found");
        return r;
    }
    let vt: usize = rd(obj);
    let shot = if vt == va(b, bullet::VTABLE) {
        Some(Shot::Plain)
    } else if vt == va(b, bullet::HOMING_VTABLE) {
        Some(Shot::Homing)
    } else {
        None
    };
    if let Some(s) = shot {
        if rd::<usize>(obj + bullet::COLL) == c && turn_back(obj, c, s) {
            return 0;
        }
        other(obj, c, "a bullet's collision that isn't its own, or without a shape");
        return r;
    }
    if is_melee(b, obj, c) {
        remember(obj, c, owner);
        return r;
    }
    other(obj, c, "not one of its CAttackMotion entries (its own collision): left as Re:CoM");
    r
}

/// Whether `c` is one of the actor's `CAttackMotion` entries (its melee attack collisions).
unsafe fn is_melee(b: usize, obj: usize, c: usize) -> bool {
    let m: usize = rd(obj + am::IN_ACTOR);
    if m == 0 || rd::<usize>(m) != va(b, am::VTABLE) {
        return false;
    }
    let n: i32 = rd(m + am::COUNT);
    let arr: usize = rd(m + am::ENTRIES);
    if arr == 0 {
        return false;
    }
    (0..n.clamp(0, 64) as usize).any(|i| {
        let e = arr + i * am::STRIDE;
        rd::<usize>(e + am::RECORD) != 0 && rd::<usize>(e + am::COLL) == c
    })
}

/// A blocked attack that is neither: Re:CoM's result stands. Logged once per owner class.
unsafe fn other(obj: usize, c: usize, why: &str) {
    let vt: usize = if obj != 0 { rd(obj) } else { 0 };
    let first = OTHER_LOGGED.lock().map(|mut v| {
        let new = !v.contains(&vt) && v.len() < 32;
        if new {
            v.push(vt);
        }
        new
    });
    if first.unwrap_or(false) {
        log(&format!("guard reactions: blocked attack of {} (coll {c:#x}): {why}; no reaction (once per class)", class_name(obj)));
    }
}

unsafe fn remember(obj: usize, c: usize, id: u32) {
    let Ok(mut v) = PENDING.lock() else { return };
    if v.iter().any(|&(x, _)| x == id) {
        return; // another entry of the same attacker in the same frame: one reaction
    }
    if v.len() >= PENDING_MAX {
        v.remove(0);
    }
    v.push((id, g::PENDING_CALLS));
    PENDING_N.store(v.len(), Ordering::Relaxed);
    drop(v);
    block_log(|| {
        let f: u64 = rd(obj + enemy::FLAGS);
        format!(
            "block melee {} id={id} coll={c:#x} kind={} elem={} ap={} serial={} flags={f:#x} state={:?}",
            class_name(obj),
            rd::<u32>(c + coll::KIND),
            rd::<u32>(c + coll::ELEMENT),
            rd::<i16>(c + coll::AP),
            rd::<i32>(c + coll::SERIAL),
            g::state_index(obj, rd(obj + enemy::STATE)),
        )
    });
}

/// A blocked shot: Re:CoM's own list setter moves its collision to the enemies' list, its
/// velocity and aim angles turn around, a homing shot aims at its shooter.
unsafe fn turn_back(obj: usize, c: usize, s: Shot) -> bool {
    let b = base();
    let shape: usize = rd(c + coll::SHAPE);
    if shape == 0 || rd::<usize>(c) != va(b, coll::VTABLE) {
        return false;
    }
    let id: u32 = rd(c + coll::OWNER);
    let flags: u64 = rd(obj + bullet::FLAGS);
    let list_before: u32 = rd(shape + coll::SHAPE_LIST_FLAGS);
    func::<SetListFn>(g::func::SET_LIST)(c, 0);
    let list_after: u32 = rd(shape + coll::SHAPE_LIST_FLAGS);
    let mover: usize = rd(obj + bullet::MOVER);
    let (mut v0, mut v1, mut a0, mut a1, mut roll) = ([0f32; 4], [0f32; 4], (0f32, 0f32), (0f32, 0f32), 0f32);
    if mover != 0 {
        v0 = rd(mover + bullet::VELOCITY);
        v1 = g::reversed(v0);
        wr(mover + bullet::VELOCITY, v1);
        a0 = (rd(mover + bullet::PITCH), rd(mover + bullet::YAW));
        roll = rd(mover + bullet::ROLL);
        if let Some((p, y)) = g::aim_angles([v1[0], v1[1], v1[2]]) {
            wr(mover + bullet::PITCH, p);
            wr(mover + bullet::YAW, y);
            a1 = (p, y);
        } else {
            a1 = a0;
        }
    }
    let shooter: i32 = rd(obj + bullet::SHOOTER);
    let homing_now: i32 = if s == Shot::Homing { rd(obj + bullet::HOMING_TARGET) } else { rd(obj + bullet::PLAIN_TARGET) };
    let rt = g::retarget(s, flags, homing_now, shooter);
    let mut aimed = String::from("straight");
    match rt {
        Retarget::None => {}
        Retarget::Actor(at) => {
            wr(obj + at, shooter);
            aimed = format!("+{at:#x} {homing_now} -> shooter {shooter}");
        }
        Retarget::Point(at) => {
            let so = func::<mv::HandleFn>(mv::HANDLE_OBJ)(shooter as u32);
            let sm: usize = if so != 0 { rd(so + enemy::MOVER) } else { 0 };
            if sm != 0 {
                // The shooter's position (`0x1401A49B0`: its mover `+0x30`).
                let p: [f32; 4] = rd(sm + enemy::MOVER_POSITION);
                wr(obj + at, p);
                aimed = format!("point +{at:#x} -> shooter {shooter} at ({:.2},{:.2},{:.2})", p[0], p[1], p[2]);
            } else {
                aimed = format!("point-homing, shooter {shooter} not found: point kept");
            }
        }
    }
    if let Ok(mut t) = TURNED.lock() {
        t.retain(|&(x, _)| x != c);
        if t.len() >= TURNED_MAX {
            t.remove(0);
        }
        t.push((c, id));
        TURNED_N.store(t.len(), Ordering::Relaxed);
    }
    block_log(|| {
        format!(
            "block shot {} id={id} shooter={shooter} ({}) coll={c:#x} kind={} elem={} ap={} flags={flags:#x} list {list_before:#x}->{list_after:#x} vel ({:.3},{:.3},{:.3})->({:.3},{:.3},{:.3}) angles ({:.3},{:.3})->({:.3},{:.3}) roll {roll:.3} homing {aimed}",
            if s == Shot::Homing { "CHomingBullet" } else { "CBullet" },
            class_name(func::<mv::HandleFn>(mv::HANDLE_OBJ)(shooter.max(0) as u32)),
            rd::<u32>(c + coll::KIND),
            rd::<u32>(c + coll::ELEMENT),
            rd::<i16>(c + coll::AP),
            v0[0], v0[1], v0[2], v1[0], v1[1], v1[2], a0.0, a0.1, a1.0, a1.1,
        )
    });
    true
}

/// A turned-back shot's later test hit something (its collision now walks the enemies' list).
unsafe fn turned_hit(c: usize, r: i32) {
    let id: u32 = rd(c + coll::OWNER);
    let known = TURNED.lock().map(|t| t.iter().any(|&(x, i)| x == c && i == id)).unwrap_or(false);
    if !known {
        return;
    }
    // The last body this attack was applied to (`coll+0x54`, written by the apply).
    let target: u32 = rd(c + coll::LAST_BODY_OWNER);
    hit_log(|| format!("turned-back shot id={id} met id={target} ({}) result {r}", class_name(func::<mv::HandleFn>(mv::HANDLE_OBJ)(target))));
}

/// From the enemy-hit wrapper, after Re:CoM's hit handler returned `r` for `e` (`hp` before).
pub(super) unsafe fn after_enemy_hit(e: usize, r: u64, hp: i32) {
    if TURNED_N.load(Ordering::Relaxed) != 0 {
        turned_damage(e, r, hp);
    }
    if PENDING_N.load(Ordering::Relaxed) == 0 {
        return;
    }
    let id: u32 = rd(e + enemy::ID);
    let found = PENDING.lock().map(|mut v| {
        let at = v.iter().position(|&(x, _)| x == id);
        if let Some(i) = at {
            v.swap_remove(i);
        }
        // Attackers that never run Re:CoM's hit handler (gimmicks) age out.
        v.retain_mut(|(_, left)| {
            *left = left.saturating_sub(1);
            *left > 0
        });
        PENDING_N.store(v.len(), Ordering::Relaxed);
        at.is_some()
    });
    if !found.unwrap_or(false) || !active() {
        return;
    }
    let name = class_name(e);
    let flags: u64 = rd(e + enemy::FLAGS);
    let now_hp: i32 = rd(e + enemy::HP);
    if r != 0 {
        block_log(|| format!("{name} id={id}: hit this frame too; Re:CoM's own reaction stands"));
        return;
    }
    if now_hp <= 0 {
        block_log(|| format!("{name} id={id}: HP {now_hp}; no reaction"));
        return;
    }
    if flags & enemy::NO_REACTION != 0 {
        block_log(|| format!("{name} id={id}: flags {flags:#x} (bit 13: Re:CoM's hit handler gives no reaction); none"));
        return;
    }
    let state = g::state_index(e, rd(e + enemy::STATE));
    let next0 = g::state_index(e, rd(e + enemy::NEXT_STATE));
    func::<PickerFn>(g::func::PICKER)(e, g::PICKER_KIND);
    let next1: usize = rd(e + enemy::NEXT_STATE);
    let flinch = g::picker_flinches(flags) && next1 == e + g::state_offset(enemy::FLINCH_STATE);
    let mut card = "none";
    if flags & enemy::CARD_PLAYING != 0 {
        if flinch {
            // Re:CoM's own "card ended by a hit" step (`0x14017F7E0`'s 0x2000001 path).
            let ended = func::<EndCardFn>(g::func::END_CARD)(e);
            wr(e + enemy::FLAGS, rd::<u64>(e + enemy::FLAGS) & !enemy::CARD_PLAYING);
            card = if ended & 0xFF != 0 { "ended (CBTL now-playing reset)" } else { "bit cleared (not CBTL's now-playing)" };
        } else {
            card = "kept (no flinch)";
        }
    }
    let after: u64 = rd(e + enemy::FLAGS);
    block_log(|| {
        format!(
            "{name} id={id}: picker(1) -> {} flags {flags:#x}->{after:#x} state {state:?} next {next0:?}->{:?} card {card}",
            if flinch { "flinch (state 11)" } else { "voice only (super armour / bit 19)" },
            g::state_index(e, next1)
        )
    });
}

/// Whether the hit Re:CoM's hit handler is about to take on `e` comes from a turned-back shot
/// (the enemy's hit info `+0xF8` has its hit flag, and the hit body's attacker is one of ours).
/// Decision 28: such a hit must do damage even while `e` plays a card, so the enemy-hit wrapper
/// gives it the same card-interrupt bit as a KH1 swing (`+0x260 & 0x2000000`; Re:CoM's own
/// "end the card, then take the hit" path). Owner's log on 0.11.0 (Checked): every turned-back
/// shot met its shooter with 0 damage, "card playing: Re:CoM's hit handler took no hit".
pub(super) unsafe fn turned_hit_pending(e: usize) -> bool {
    if TURNED_N.load(Ordering::Relaxed) == 0 || !active() {
        return false;
    }
    let info: usize = rd(e + enemy::INFO);
    if info == 0 || rd::<u32>(info + enemy::INFO_FLAGS) & enemy::INFO_HIT == 0 {
        return false;
    }
    let hit_body: usize = rd(info + enemy::INFO_BODY);
    if hit_body == 0 {
        return false;
    }
    let by: u32 = rd(hit_body + body::ATTACKER);
    TURNED.lock().map(|t| t.iter().any(|&(_, i)| i == by)).unwrap_or(false)
}

/// An enemy's hit this frame came from a turned-back shot: the damage Re:CoM's own path gave.
unsafe fn turned_damage(e: usize, r: u64, hp: i32) {
    let info: usize = rd(e + enemy::INFO);
    if info == 0 || rd::<u32>(info + enemy::INFO_FLAGS) & enemy::INFO_HIT == 0 {
        return;
    }
    let hit_body: usize = rd(info + enemy::INFO_BODY);
    if hit_body == 0 {
        return;
    }
    let by: u32 = rd(hit_body + body::ATTACKER);
    let known = TURNED.lock().map(|t| t.iter().any(|&(_, i)| i == by)).unwrap_or(false);
    if !known {
        return;
    }
    let after: i32 = rd(e + enemy::HP);
    let f: u64 = rd(e + enemy::FLAGS);
    hit_log(|| {
        format!(
            "turned-back shot id={by} hit {} id={}: damage {} hp {hp}->{after}{}",
            class_name(e),
            rd::<u32>(e + enemy::ID),
            hp - after,
            if r == 0 && f & enemy::CARD_PLAYING != 0 { " (card playing: Re:CoM's hit handler took no hit)" } else { "" }
        )
    });
}
