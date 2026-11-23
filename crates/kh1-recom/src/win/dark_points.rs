//! Riku's Dark Points and Dark Mode at Combat = KH1 (`crate::dark_points`; fork branch, 0.151.0).
//!
//! Re:CoM (Checked (exe: disasm), `docs/recom-riku-kh1.md` § Research):
//! - Dark Riku (`CPlayerDRiku`, vtable `0x1405A50E8`) is made with Riku's battle player (Riku's
//!   `0x14021223D`, when his ability word `+0x41C & 0x4000000` is set) and kept at Riku `+0x8A0`
//!   (and Riku at his `+0x8A0`).
//! - Riku's vtable `+0x198` `0x140215FB0(riku, 1)`: action 0x20 sub-state 1, his transformation;
//!   its exe (`+0x1A0` `0x140215380`) hands over to Dark Riku (`0x140206810`), whose `+0x198`
//!   `0x1402046D0(dark, 1)` writes him to the main player `0x14087B380`. `0x1402046D0(dark, 0)`:
//!   action 0x20 sub-state 4, his way back.
//!
//! Ours: the DP of `crate::dark_points` from Riku's (or Dark Riku's) swings that land, the enemies
//! he defeats and the hits he takes; at 30 DP, Re:CoM's `+0x198(riku, 1)` when he is free (stand /
//! walk / run); at 0 in Dark Mode, `+0x198(dark, 0)`. Battle rooms only, reset at every battle start.
//! The first Dark Mode is kept in his save block's spare byte ([`DARK_SEEN`]: the skin's unlock).

use super::movement::{self as mv, func, p, rd, wr};
use super::{base, log, va};
use crate::dark_points::{self as dp, Change, Dp, Event};
use crate::sites::data;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

/// `CPlayerDRiku` vtable (RTTI `.?AVCPlayerDRiku@PL@@`). Checked (exe).
const DRIKU_VTABLE: u64 = 0x1_405A_50E8;
/// The vtable slot of the Dark Mode switch.
const VT_SWITCH: usize = 0x198;
/// Riku's and Dark Riku's switch functions and their first bytes. Checked (exe).
const RIKU_SWITCH: u32 = 0x0021_5FB0;
const RIKU_SWITCH_BYTES: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20];
const DRIKU_SWITCH: u32 = 0x0020_46D0;
const DRIKU_SWITCH_BYTES: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x18, 0x57, 0x48, 0x83, 0xEC, 0x70];
/// Each one's partner (`+0x8A0`). Checked (exe: `0x14021223D`, `0x1401FF8C6`).
const PARTNER: usize = 0x8A0;
/// A dead player (`+0x7B0`; the switch functions do nothing while it is set). Checked (exe).
const DEAD: usize = 0x7B0;
/// The player's weapon and its attack id (`CPlayerWeapon +0x23C`, `0x1402250B0`). Checked (exe).
const WEAPON: usize = 0x3D0;
const ATTACK_ID: usize = 0x23C;
/// The pending hit's reaction kind, its HP, its body's attacker id. Checked (decomp, `win::attack`).
const HIT_KIND: usize = 0x710;
const HP: usize = 0x274;
const BODY_OWNER: usize = 0x690 + 0x98;
/// Re:CoM's actions: the swing 0xA, the card attack 0xB, the roll 6, the transformation 0x20.
const ATTACK: u32 = 0xA;
const CARD_ATTACK: u32 = 0xB;
const AVOID: u32 = 6;
const SWITCHING: u32 = 0x20;
/// The spare byte's bit for "Dark Mode reached once" (`crate::mp::off::SPARE`; bits 1 / 2 are
/// `crate::unlocks`'): the Dark Form skin's unlock (0.152.0).
pub const DARK_SEEN: u8 = 4;

/// Re:CoM's own DP (Checked (exe: disasm)): the player's `+0x44C` (at most 30: `0x1401F57A4`, the
/// card values' add) and its gauge's max `+0x450`; its Dark gauge `+0x3E0`, set by
/// `0x14020BFA0(gauge, dp, max)` (u16s at gauge `+0xAC` / `+0xAE`, the control `+0x80` at
/// dp × 100 / max). Re:CoM's damage check (`0x1401E7255`, Dark Mode unlocked) adds 1 per hit taken,
/// in Dark Mode takes 5 and at 0 sets the way-back flag `+0x424 & 0x100000`: undone around the
/// check, the owner's tables stand alone.
const RECOM_DP: usize = 0x44C;
const RECOM_DP_MAX: usize = 0x450;
const DARK_GAUGE: usize = 0x3E0;
const FLAGS_424: usize = 0x424;
const DP_EXIT: u32 = 0x10_0000;
const GAUGE_SET: u32 = 0x0020_BFA0;
const GAUGE_SET_BYTES: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x10, 0x56, 0x48, 0x83, 0xEC, 0x30];
type GaugeSetFn = unsafe extern "win64" fn(usize, u32, u32);

type SwitchFn = unsafe extern "win64" fn(usize, u32) -> u64;
type HandleFn = unsafe extern "win64" fn(u32) -> usize;

struct State {
    dp: Dp,
    /// Frames counted this battle.
    frame: u32,
    /// The frame the last roll ended (0 none).
    roll_end: u32,
    was_rolling: bool,
    /// The last swing counted: (attack id, frame).
    last_swing: (u32, u32),
    /// A switch asked of Re:CoM and not seen yet (frames left to wait).
    pending: u32,
    /// The frame his Guard started (0 none waiting).
    guard_at: u32,
}

static STATE: Mutex<State> = Mutex::new(State { dp: Dp { dp: 0, dark: false, idle: 0, streak: 0 }, frame: 0, roll_end: 0, was_rolling: false, last_swing: (0, 0), pending: 0, guard_at: 0 });
static CHECKED: AtomicU32 = AtomicU32::new(0);
static NO_DARK_LOGGED: AtomicBool = AtomicBool::new(false);
static LINES: AtomicU32 = AtomicU32::new(200);

fn note(line: impl FnOnce() -> String) {
    if LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("dark points: {}", line()));
    }
}

/// The DP rules are on: Combat = KH1, Riku's story, `riku_kh1_movement`.
fn on() -> bool {
    mv::kh1_on() && mv::riku_on() && super::stats::riku_story()
}

/// The main player is Dark Riku.
pub unsafe fn is_dark(pl: usize) -> bool {
    pl != 0 && rd::<usize>(pl) == va(base(), DRIKU_VTABLE)
}

/// `pl` is the main player and Riku or Dark Riku, DP on, in a battle room.
unsafe fn ours(pl: usize) -> bool {
    on() && pl != 0 && pl == rd::<usize>(va(base(), data::MAIN_PLAYER)) && (mv::is_main_riku(pl) || is_dark(pl)) && rd::<i32>(pl + p::BATTLE) != 0
}

/// Both switch functions are Re:CoM Steam's (checked once; a mismatch turns the switches off).
unsafe fn switches_ok() -> bool {
    match CHECKED.load(Ordering::Relaxed) {
        1 => return true,
        2 => return false,
        _ => {}
    }
    let b = base();
    let ok = rd::<[u8; 10]>(b + RIKU_SWITCH as usize) == RIKU_SWITCH_BYTES && rd::<[u8; 10]>(b + DRIKU_SWITCH as usize) == DRIKU_SWITCH_BYTES && rd::<[u8; 10]>(b + GAUGE_SET as usize) == GAUGE_SET_BYTES;
    CHECKED.store(if ok { 1 } else { 2 }, Ordering::Relaxed);
    log(&format!("dark points: Re:CoM's Dark Mode switches (0x140215FB0, 0x1402046D0) and Dark gauge setter (0x14020BFA0) {}", if ok { "checked" } else { "NOT Re:CoM Steam's: DP count, Dark Mode never starts, no gauge" }));
    ok
}

/// At every battle start (`win::battle::manager_ctor`): DP back to 0.
pub fn battle_start() {
    if let Ok(mut g) = STATE.lock() {
        let was = g.dp.dp;
        *g = State { dp: Dp::default(), frame: 0, roll_end: 0, was_rolling: false, last_swing: (0, 0), pending: 0, guard_at: 0 };
        if was != 0 && on() {
            note(|| format!("battle start: DP {was} -> 0"));
        }
    }
}

/// Re:CoM's switch through the player's own vtable slot, when it is the expected function.
unsafe fn switch(pl: usize, want: u32, on_: u32) -> bool {
    let vt: usize = rd(pl);
    let slot: usize = if vt != 0 { rd(vt + VT_SWITCH) } else { 0 };
    if slot != base() + want as usize || !switches_ok() {
        note(|| format!("switch: the player's vtable +0x198 is {slot:#x}, not Re:CoM's {:#x}", 0x1_4000_0000u64 + want as u64));
        return false;
    }
    std::mem::transmute::<usize, SwitchFn>(slot)(pl, on_);
    true
}

/// The player is free to switch: standing, walking or running, alive.
unsafe fn free(pl: usize) -> bool {
    let a: u32 = rd(pl + p::ACTION);
    (crate::movement::action::STAND..=crate::movement::action::RUN).contains(&a) && rd::<i32>(pl + DEAD) == 0
}

/// One battle frame of the main player (Riku's or Dark Riku's request bits).
pub unsafe fn frame(pl: usize) {
    if !ours(pl) {
        return;
    }
    let dark = is_dark(pl);
    let action: u32 = rd(pl + p::ACTION);
    let Ok(mut g) = STATE.lock() else { return };
    g.frame = g.frame.wrapping_add(1);
    if g.dp.dark != dark {
        g.dp.dark = dark;
        g.pending = 0;
        note(|| format!("Dark Mode {} (DP {})", if dark { "on" } else { "off" }, g.dp.dp));
        if dark {
            mark_seen();
        }
    }
    let rolling = action == AVOID;
    if g.was_rolling && !rolling {
        g.roll_end = g.frame;
    }
    g.was_rolling = rolling;
    if action == SWITCHING {
        return;
    }
    // A Guard that blocked nothing by its end (KH1's live frames end at 36, the motion at 60): a
    // missed parry.
    let mut ch = Change::None;
    if g.guard_at != 0 && g.frame.wrapping_sub(g.guard_at) > GUARD_SPAN {
        g.guard_at = 0;
        let before = g.dp.dp;
        ch = g.dp.event(Event::ParryMiss);
        let after = g.dp.dp;
        if before != after {
            note(|| format!("ParryMiss: DP {before} -> {after}"));
        }
    }
    if ch == Change::None {
        ch = g.dp.frame(action == ATTACK || action == CARD_ATTACK);
    }
    show(pl, g.dp.dp);
    apply(&mut g, pl, ch);
}

/// Our DP in Re:CoM's own field and Dark gauge (when they differ).
unsafe fn show(pl: usize, dp: i32) {
    if !switches_ok() {
        return;
    }
    if rd::<i32>(pl + RECOM_DP) != dp || rd::<i32>(pl + RECOM_DP_MAX) != dp::MAX {
        wr(pl + RECOM_DP, dp);
        wr(pl + RECOM_DP_MAX, dp::MAX);
        let g: usize = rd(pl + DARK_GAUGE);
        if g != 0 {
            func::<GaugeSetFn>(GAUGE_SET)(g, dp as u32, dp::MAX as u32);
        }
    }
}

/// What a change asks, when the player is free (one try a frame; Re:CoM's own action 0x20 takes it
/// from there).
unsafe fn apply(g: &mut State, pl: usize, ch: Change) {
    if g.pending > 0 {
        g.pending -= 1;
        return;
    }
    match ch {
        Change::Enter if !is_dark(pl) && free(pl) => {
            if rd::<usize>(pl + PARTNER) == 0 {
                if !NO_DARK_LOGGED.swap(true, Ordering::Relaxed) {
                    log("dark points: 30 DP, but Re:CoM made no Dark Riku for this battle (Dark Mode not unlocked in the story yet: Riku's +0x41C & 0x4000000): DP stay at 30");
                }
                return;
            }
            if switch(pl, RIKU_SWITCH, 1) {
                g.pending = 120;
                note(|| "30 DP: Dark Mode (Re:CoM's +0x198(riku, 1))".into());
            }
        }
        Change::Exit if is_dark(pl) && free(pl) => {
            if switch(pl, DRIKU_SWITCH, 0) {
                g.pending = 120;
                note(|| "0 DP: Dark Mode ends (Re:CoM's +0x198(dark Riku, 0))".into());
            }
        }
        _ => {}
    }
}

fn event(pl: usize, e: Event) {
    let Ok(mut g) = STATE.lock() else { return };
    let before = g.dp.dp;
    let ch = g.dp.event(e);
    let after = g.dp.dp;
    if before != after {
        note(|| format!("{e:?}{}: DP {before} -> {after}", if g.dp.dark { " (Dark Mode)" } else { "" }));
    }
    // SAFETY: the main player, as `frame` reads it.
    unsafe {
        show(pl, after);
        apply(&mut g, pl, ch)
    };
}

/// A hit on an enemy from the main player's own attack (Re:CoM's hit handler, `win::attack`): once
/// per swing (its attack id, not again within 20 frames).
pub unsafe fn enemy_hit(enemy_body_owner: u32) {
    let pl: usize = rd(va(base(), data::MAIN_PLAYER));
    if !ours(pl) || enemy_body_owner == 0 {
        return;
    }
    let w: usize = rd(pl + WEAPON);
    let ids = [rd::<u32>(pl + 0x20), if w != 0 { rd::<u32>(w + 0x20) } else { 0 }];
    if !ids.contains(&enemy_body_owner) {
        return;
    }
    let id: u32 = if w != 0 { rd(w + ATTACK_ID) } else { 0 };
    {
        let Ok(mut g) = STATE.lock() else { return };
        let f = g.frame;
        if g.last_swing.0 == id && f.wrapping_sub(g.last_swing.1) < 20 {
            return;
        }
        g.last_swing = (id, f);
    }
    event(pl, Event::Swing { finisher: dp::is_finisher(id) });
}

/// An enemy defeated (`win::drops::defeat_prize`): +3, a boss +5 in Dark Mode.
pub unsafe fn defeat(enemy: usize) {
    let pl: usize = rd(va(base(), data::MAIN_PLAYER));
    if !ours(pl) || enemy == 0 {
        return;
    }
    event(pl, Event::Defeat { boss: is_boss(enemy) });
}

unsafe fn is_boss(obj: usize) -> bool {
    let vt: usize = rd(obj);
    vt != 0 && super::stats::rtti_name(base(), vt).is_some_and(|n| n.starts_with(".?AVCBoss"))
}

/// What [`before_damage`] saw.
pub struct Before {
    hp: i32,
    kind: u16,
    boss: bool,
    action: u32,
    recom_dp: i32,
    exit_flag: bool,
    cb_flag: bool,
}

/// The card battle's flag Re:CoM's check sets with its way back (`+0x3E8 → +0x208 & 0x400000`).
const CARD_BATTLE: usize = 0x3E8;
const CB_FLAGS: usize = 0x208;
const CB_DARK_END: u32 = 0x40_0000;

unsafe fn cb_flag(pl: usize) -> bool {
    let cb: usize = rd(pl + CARD_BATTLE);
    cb != 0 && rd::<u32>(cb + CB_FLAGS) & CB_DARK_END != 0
}

/// Before Re:CoM's damage check on the main player (`win::attack::damage_check`): what the hit is,
/// and Re:CoM's own DP (its check moves them: undone after).
pub unsafe fn before_damage(pl: usize) -> Option<Before> {
    if !ours(pl) {
        return None;
    }
    let attacker = match rd::<u32>(pl + BODY_OWNER) {
        0 => 0,
        id => func::<HandleFn>(mv::HANDLE_OBJ)(id),
    };
    Some(Before { hp: rd(pl + HP), kind: rd(pl + HIT_KIND), boss: attacker != 0 && is_boss(attacker), action: rd(pl + p::ACTION), recom_dp: rd(pl + RECOM_DP), exit_flag: rd::<u32>(pl + FLAGS_424) & DP_EXIT != 0, cb_flag: cb_flag(pl) })
}

/// After it: Re:CoM's own DP change undone (its +1 / −5 and its way-back flag); a hit that took HP
/// moves ours once.
pub unsafe fn after_damage(pl: usize, before: Option<Before>) {
    let Some(Before { hp, kind, boss, action, recom_dp, exit_flag, cb_flag: cb_was }) = before else { return };
    if !cb_was && cb_flag(pl) {
        let cb: usize = rd(pl + CARD_BATTLE);
        wr(cb + CB_FLAGS, rd::<u32>(cb + CB_FLAGS) & !CB_DARK_END);
    }
    if rd::<i32>(pl + RECOM_DP) != recom_dp {
        wr(pl + RECOM_DP, recom_dp);
    }
    if !exit_flag && rd::<u32>(pl + FLAGS_424) & DP_EXIT != 0 {
        wr(pl + FLAGS_424, rd::<u32>(pl + FLAGS_424) & !DP_EXIT);
    }
    if rd::<i32>(pl + HP) >= hp {
        return;
    }
    let after_roll = STATE.lock().is_ok_and(|g| g.roll_end != 0 && g.frame.wrapping_sub(g.roll_end) <= dp::DODGE_WINDOW);
    event(pl, Event::Hurt { hit: dp::hit_of(kind, boss), in_combo: action == ATTACK || action == CARD_ATTACK, after_roll });
}

/// Frames of the Guard after which an unblocked one is a missed parry (KH1's guard motion: 60).
const GUARD_SPAN: u32 = 60;
/// A block within this many frames of the Guard's start is perfect (the first 10 of its live
/// frames 10..36: the owner's "parry perfeito", our definition).
const PERFECT_BY: u32 = 20;

/// His KH1 frame's events (`win::movement::run_frame`): a Guard started, a Guard that blocked.
pub unsafe fn guard_events(pl: usize, events: &[kh1_core::controller::Event]) {
    use kh1_core::controller::Event as E;
    if !ours(pl) {
        return;
    }
    for e in events {
        match e {
            E::Guarded => {
                if let Ok(mut g) = STATE.lock() {
                    g.guard_at = g.frame.max(1);
                }
            }
            E::GuardSucceeded => {
                let perfect = STATE.lock().ok().and_then(|mut g| {
                    let at = std::mem::replace(&mut g.guard_at, 0);
                    (at != 0).then(|| g.frame.wrapping_sub(at) <= PERFECT_BY)
                });
                if perfect == Some(true) {
                    event(pl, Event::PerfectParry);
                } else {
                    note(|| "a block, not perfect: no DP".into());
                }
            }
            _ => {}
        }
    }
}

/// Riku's save block's spare byte gets [`DARK_SEEN`] (the first Dark Mode).
unsafe fn mark_seen() {
    let save: usize = rd(va(base(), crate::keyblades::SAVE_PTR));
    if save == 0 {
        return;
    }
    let at = save + crate::mp::BLOCK_FIRST + crate::mp::off::SPARE;
    let b: u8 = rd(at);
    if b & DARK_SEEN == 0 {
        wr(at, b | DARK_SEEN);
        log("dark points: first Dark Mode: kept in Riku's save (the Dark Form skin unlocks)");
    }
}

/// Riku has reached Dark Mode once (his save). For the Dark Form skin (next stage).
#[allow(dead_code)]
pub unsafe fn dark_seen() -> bool {
    let save: usize = rd(va(base(), crate::keyblades::SAVE_PTR));
    save != 0 && super::stats::riku_story() && rd::<u8>(save + crate::mp::BLOCK_FIRST + crate::mp::off::SPARE) & DARK_SEEN != 0
}

/// The DP now (for the Dark gauge, next stage; 0 outside a battle).
#[allow(dead_code)]
pub fn current() -> i32 {
    STATE.lock().map_or(0, |g| g.dp.dp)
}
