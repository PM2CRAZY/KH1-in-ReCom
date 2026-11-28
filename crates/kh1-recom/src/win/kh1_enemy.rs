//! M10 / M11: KH1's enemies on Re:CoM's own enemy states (`crate::kh1_enemy`,
//! `docs/recom-m10-shadow.md`, `docs/recom-m11-enemies.md`). The shared runtime, generalized from
//! M10's Shadow runtime; one table (`crate::kh1_enemy::CLASSES`) drives it.
//!
//! - **Class swap.** A wrapper on each class's ctor (its only caller is the enemy factory
//!   `0x140188F40`): in a KH1 battle (Combat = KH1, Sora's story, decided by the battle enemy
//!   manager's ctor) with `enemy_ai = kh1`, the new enemy's vtable becomes our copy of Re:CoM's (the
//!   RTTI locator kept at `−8`), whose state slots run its KH1 behaviour. A 0x20-byte tail after the
//!   body (the factory's size raised) marks it (magic, battle, our index). Field enemies and Re:CoM
//!   battles are never touched.
//! - **States** (`crate::kh1_enemy::state`, each class's attack states): Re:CoM's state machine
//!   calls them; each runs KH1's frame for this enemy: the behaviour (`kh1_core::enemy::Behaviour`)
//!   once a frame when no action plays, KH1's body physics (turn, walk, arc, air steering, push
//!   slide) written into Re:CoM's mover (`+0xF0`: yaw `+0x54`, velocity `+0x80`), KH1's motions
//!   played through Re:CoM's own motion play (vtable `+0x70` `0x14017D750`: the clip, its sounds
//!   (`.SES`) and its attack records (`.ASD`), AP from Re:CoM's `@ENP`).
//! - **Reactions.** Re:CoM's hit handler `0x14017F7E0` takes the hit, applies Re:CoM's damage and
//!   picks state 11 / 12 / 30 as always; our state 11 / 12 begin turns the hit into KH1's reaction
//!   (`kh1_core::enemy::react`), state 30 runs Re:CoM's defeat with the KH1 motion.
//! - **Guard.** `super::guard` hands a blocked enemy to [`guard_block`]: KH1's clash recoil.
//!
//! Per frame: one tail check per converted enemy's state call, the behaviour's frame, a few reads
//! and writes. The Shadow's runtime is M10's, unchanged (its log lines and trace token too).

use super::movement::{func, orig, rd, rd_vec, wr};
use super::{base, log};
use crate::kh1_enemy::{self as ce, state, tail, Appear, Manifest, CLASSES};
use crate::sites::data;
use kh1_core::enemy::{self as ke, Behaviour, Cmd, Hit, HitNews, Phase, World};
use kh1_core::Vec3;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

const N: usize = CLASSES.len();

#[allow(clippy::declare_interior_mutable_const)]
const ZERO: AtomicUsize = AtomicUsize::new(0);
#[allow(clippy::declare_interior_mutable_const)]
const OFF: AtomicBool = AtomicBool::new(false);
#[allow(clippy::declare_interior_mutable_const)]
const NOLINE: AtomicU32 = AtomicU32::new(0);

pub static ORIG_CTOR: [AtomicUsize; N] = [ZERO; N];
static INSTALLED: [AtomicBool; N] = [OFF; N];
static MANIFEST: [OnceLock<Manifest>; N] = [const { OnceLock::new() }; N];
/// Our vtable copies (slot 0's address) and Re:CoM's slots.
static COPY: [AtomicUsize; N] = [ZERO; N];
static ORIGINAL: [OnceLock<Vec<usize>>; N] = [const { OnceLock::new() }; N];
static LINES: [AtomicU32; N] = [NOLINE; N];
const LINE_CAP: u32 = 20_000;
static SEED: AtomicU32 = AtomicU32::new(0x1234_5678);

/// Re:CoM functions (bytes checked: `sites::SHADOW_CALLEES` and the lists named there).
mod f {
    pub const ATTACK_END: u32 = 0x0017_E540;
    pub const DEFAULT_STATE: u32 = 0x0017_D6F0;
    pub const INTRO_ROOM: u32 = 0x001A_1680;
    pub const INTRO_FLAG: u32 = 0x001A_1670;
    pub const INTRO_COUNT: u32 = 0x001A_14B0;
    pub const HURT_OFF: u32 = 0x001A_2D70;
    /// `0x1401A2D90` / `0x1401A2DD0`: the pair Re:CoM's dig switches with the hurt bodies
    /// (`0x140192E60` case 0 / 4); checked in `M6`'s / our lists.
    pub const OTHER_OFF: u32 = 0x001A_2D90;
    pub const HURT_ON: u32 = 0x001A_2DB0;
    pub const OTHER_ON: u32 = 0x001A_2DD0;
    pub const CLOCK: u32 = 0x001A_47C0;
    pub const BONE: u32 = 0x002A_BB20;
    pub const HANDLE_OBJ: u32 = 0x002A_D350;
    pub const SCALE: u32 = 0x001A_4A20;
    pub const SCALE_SET: u32 = 0x001A_7050;
    /// Re:CoM's max-HP getter (`0x14017F5A0`, checked in the stats lists).
    pub const MAX_HP: u32 = 0x0017_F5A0;
    /// Re:CoM's enemy heal and its defeat EXP drop (`sites::KH1_ENEMY_REWARD_CALLEES`).
    pub const HEAL: u32 = 0x0018_1CE0;
    pub const EXP_DROP: u32 = 0x0018_10A0;
    /// Re:CoM's attack-set radius setter (`sites::KH1_ENEMY_RING_CALLEES`), on the enemy's attack set `+0x100`.
    pub const SET_RADIUS: u32 = 0x0014_E5C0;
    /// Re:CoM's attack check on an attack set (`sites::GRAB_CALLEES`).
    pub const ATTACK_CHECK: u32 = 0x0014_E6A0;
}
/// Enemy fields (Checked (decomp): `0x14017F7E0`, `0x14017C530`, `0x140185320`).
mod e {
    pub const ID: usize = 0x20;
    pub const DT: usize = 0x14;
    pub const STATE: usize = 0x60;
    pub const NEXT: usize = 0x68;
    pub const MOVER: usize = 0xF0;
    pub const HIT_INFO: usize = 0xF8;
    pub const FLAGS: usize = 0x260;
    pub const HP: usize = 0x274;
    pub const CHOSEN: usize = 0x278;
    /// Re:CoM's EXP for the enemy, which its defeat EXP drop splits into prizes (`0x1401810A0`). Checked
    /// (decomp).
    pub const EXP: usize = 0x2E4;
    /// The `@ENP` row: Re:CoM makes no prize of an enemy without it (`0x1401810A0`). Checked (decomp).
    pub const ENP_ROW: usize = 0x310;
    /// The enemy's attack set (its `.ASD` records' collisions), as Re:CoM's Large Body's state 3 step passes it to
    /// `0x14014E5C0` (`0x14018F280`). Checked (decomp).
    pub const ATTACKS: usize = 0x100;
    /// Re:CoM's state 0 alternate path (`0x140192850` / `0x14017B590`: `+0x260 & 0x20000000008`).
    pub const APPEAR_ALT: u64 = 0x200_0000_0008;
}
mod mv {
    pub const POS: usize = 0x30;
    pub const YAW: usize = 0x54;
    pub const VEL: usize = 0x80;
    pub const ACCEL: usize = 0x90;
    pub const CONTACT: usize = 0x120;
    pub const SPEED_SCALE: usize = 0x13C;
}
/// The hit info (`enemy+0xF8`): the hit attack `+0x50` (its Re:CoM kind `+0x80`, attacker handle
/// `+0x98`), the body parts `+0x40` (count `+0x38`, 0x120 each; contact point `+0x50`).
mod hi {
    pub const ATTACK: usize = 0x50;
    pub const KIND: usize = 0x80;
    pub const OWNER: usize = 0x98;
    pub const PARTS: usize = 0x40;
    pub const COUNT: usize = 0x38;
    pub const CONTACT: usize = 0x50;
}

type CtorFn = unsafe extern "win64" fn(usize, usize, usize, usize) -> usize;
type StateFn = unsafe extern "win64" fn(usize);
/// `0x14014E6A0(set) -> u32`.
type CheckFn = unsafe extern "win64" fn(usize) -> u32;
type EndFn = unsafe extern "win64" fn(usize, usize, i32);
type IntFn = unsafe extern "win64" fn() -> i32;
type BodiesFn = unsafe extern "win64" fn(usize, i32);
type ClockFn = unsafe extern "win64" fn(usize) -> i32;
type BoneFn = unsafe extern "win64" fn(usize, i32) -> usize;
type HandleFn = unsafe extern "win64" fn(u32) -> usize;
type ScaleFn = unsafe extern "win64" fn(usize) -> usize;
type ScaleSetFn = unsafe extern "win64" fn(usize, *const [f32; 4]);
type PlayFn = unsafe extern "win64" fn(usize, u32, u64, u64, i32, u32, u32);
type MaxHpFn = unsafe extern "win64" fn(usize) -> i32;
type HealFn = unsafe extern "win64" fn(usize, i32, usize, u32, *const [f32; 4], usize) -> u32;
type ExpDropFn = unsafe extern "win64" fn(usize, *const [f32; 4]) -> u64;
type PointFn = unsafe extern "win64" fn(usize, *mut [f32; 4]);

/// A running action (an attack, or a jump start, an appearance): the AI waits for it, unless it
/// is `steered` (KH1's plain request made with attack rows, `kh1_core::enemy::Cmd::Attack`: the AI
/// keeps running and steering the body while it plays; M11 fliers).
#[derive(Clone, Copy, Debug)]
struct Act {
    request: u16,
    then: u16,
    attack: Option<u8>,
    blend: f32,
    steered: bool,
    /// The KH1 frame its motion starts from (`kh1_core::enemy::Cmd::AttackAt`; 0 otherwise).
    start: f32,
}

struct Runtime {
    class: usize,
    enemy: usize,
    battle: u32,
    ai: Box<dyn Behaviour>,
    current: Option<u16>,
    queued: Option<(u16, f32)>,
    /// The start frame of the queued request when the behaviour gave one (`Cmd::QueueAt`, M11: the Yellow Opera's Thunder),
    /// for that request only.
    queued_start: Option<(u16, f32)>,
    action: Option<Act>,
    /// The action motion has started (its clock now counts).
    action_playing: bool,
    reaction: Option<Phase>,
    /// A launch's lie request (`base + 3` / `+ 8`) while lying.
    lie: Option<u16>,
    clash: Option<u8>,
    last_pos: Vec3,
    /// Re:CoM's own state 0 (its alternate appearance) runs.
    recom_appear: bool,
    appear_sub: u8,
    dead: bool,
    /// Frames since the defeat (the trace stops listing it once Re:CoM removed it).
    dead_frames: u32,
    /// KH1's flag 0x40 clear (Re:CoM's hurt bodies on).
    hittable: bool,
    /// The last launch's down time (frames).
    down: f32,
    /// The motion playing: its clock's wrap (a clip Re:CoM loops), its age.
    watch: ce::ClipWatch,
    /// KH1 frames since the action was requested (until its motion starts), then since its motion
    /// started: the safety net (`ce::watchdog`).
    action_age: f32,
    /// Re:CoM's state at the last KH1 frame (the log of a state change while unhittable).
    last_state: Option<usize>,
    /// A Guard blocked an attack whose recoil the model lacks: the next state 11 is Re:CoM's own
    /// flinch (owner 2026-11-02, "Soldier choices"), not KH1's reaction.
    recom_flinch: bool,
    /// Re:CoM's own flinch runs (state 11's Re:CoM begin / step); ends at the next state 1.
    recom_state: bool,
    /// We asked for the next state 1 (an attack's end, a reaction's end, the appearance's end):
    /// any other state 1 begin follows a state Re:CoM owns (stun, Stop, freeze, a Re:CoM flinch,
    /// the first-strike stun) and the KH1 frame is resumed cleanly (`take_over`).
    handing_back: bool,
    /// KH1 frames in state 1 with the AI polled and issuing nothing (`ce::watchdog::QUIET_LIMIT`).
    quiet: f32,
    /// KH1 frames the running reaction has taken (`ce::watchdog::REACTION_LIMIT`).
    reaction_age: f32,
    /// The battle floor's height (KH1 y) for a flying body's `sys 89` (M11 fliers): Sora's height
    /// the last time he stood on the floor (Assumption: Re:CoM's battle floors are flat).
    floor: Option<f32>,
    /// Re:CoM's own model scale, read before our first `Cmd::Scale` (KH1's scale is a factor on it).
    base_scale: Option<[f32; 4]>,
    /// KH1's hit counter (`actor+0x3E8`): one more per hit record (clashes too, `0x1402BFE60`),
    /// zeroed at the end of a KH1 frame with no action playing (`0x1402B7260`: actor flag 4
    /// clear: no action of ours waiting and no reaction). The behaviour hears it as the record's
    /// `+0x24` (`Behaviour::set_hit_count`; M11, the Bandit).
    hits: u16,
    /// The shared enemy-guard step: the guard row the behaviour made last (`Cmd::Guard`), and
    /// whether its bodies carry Re:CoM's guard flag now ([`guard_bodies`]).
    guard: Option<GuardOn>,
    /// A KH1 mode-3 row (`ke::guard::ALWAYS`) that a windowed row was made over (M11, the Defender: its shield's front
    /// row stays while its spin's all-round row lives; KH1 tests both, `0x1402B47A0`): it comes back when that row ends.
    guard_under: Option<GuardOn>,
    /// The top row's window is on this frame ([`guard_step`]; a mode-3 row always is).
    guard_live: bool,
    guarded: bool,
    /// Re:CoM's body guard fields before ours (per hurt part: `+0x9C & 1`, `+0x8C`, `+0xB0`).
    guard_saved: Vec<(u32, u32, f32)>,
    /// The enemy's side of the last clash on its guard row (KH1's `0x1402B4BC0` from the swing's
    /// priority, `pair_dispatch`: kind 6 / 7, or a repel), until its guard event plays it.
    guard_side: Option<(ke::guard::Side, bool)>,
    /// A guard row's clash: its recoil and KH1's kind, played by the next state 11 begin.
    clash_recoil: Option<(u16, u8)>,
    /// The Re:CoM bone a behaviour looks from (the manifest's `sight_bone`, M11: the Search Ghost's
    /// cone), read once at the ctor; None for the other classes (no per-frame read).
    sight_bone: Option<i32>,
    /// KH1's `+0x3EA`: the base request of the last launch it played (KH1's own or a script's), 0 before
    /// any (`kh1_core::enemy::Behaviour::armoured`, M11: the Pirate's `sys 70`).
    launch_base: u16,
    /// M11, attack meets attack: the record one of its attack rows got from Sora's swing this frame
    /// ([`swing_meets_attacks`]), played at its next hit-handler call ([`attack_clash_watch`]).
    attack_clash: Option<ce::EnemyGets>,
}

/// A guard row made with a request (`kh1_core::enemy::Cmd::Guard`): KH1's row lives while that
/// request plays (it takes the owner's request at its first tick and ends when it changes,
/// `0x1402B47A0`) and is on while its HIT slot is.
#[derive(Clone, Copy, Debug)]
struct GuardOn {
    row: u8,
    request: u16,
    /// The request has played since the row was made.
    seen: bool,
    /// KH1 frames since it was made (a request that never plays drops it: `ce::watchdog::START_LIMIT`).
    age: f32,
}

// SAFETY: a runtime (its behaviour holds `Rc`s that never leave it) is only touched under `RT`'s
// lock; the ctor may run on a loading thread and the states on the game thread.
unsafe impl Send for Runtime {}

/// Every KH1 enemy's runtime (this battle's; older ones are dropped at the next ctor).
static RT: Mutex<Vec<Runtime>> = Mutex::new(Vec::new());

fn line(k: usize, s: impl FnOnce() -> String) {
    if LINES[k].fetch_add(1, Ordering::Relaxed) < LINE_CAP {
        log(&format!("{}: {}", CLASSES[k].name, s()));
    }
}

pub fn installed(k: usize) -> bool {
    INSTALLED[k].load(Ordering::Relaxed)
}

fn any_installed() -> bool {
    (0..N).any(installed)
}

fn live_battle(k: usize) -> bool {
    installed(k) && super::battle::kh1_battle() && super::settings().enemy_ai == crate::settings::Mode::Kh1
}

/// `super::stats`: our vtable copies stand for their classes' vtables.
pub(super) fn canonical_vtable(vt: usize) -> usize {
    for (k, c) in CLASSES.iter().enumerate() {
        let copy = COPY[k].load(Ordering::Relaxed);
        if copy != 0 && vt == copy {
            return base() + c.vtable as usize;
        }
    }
    vt
}

/// Loads the class's manifest, checks its vtable and factory size, builds the vtable copy, patches
/// the size. The ctor hook is installed by the caller after this.
pub unsafe fn prepare(bs: usize, k: usize, manifest: Option<&std::path::Path>) -> bool {
    let c = &CLASSES[k];
    let Some(p) = manifest else {
        if k == 0 {
            log("shadow: no kh1-shadow.json (run kh1-recom-build): Shadows keep Re:CoM's behaviour");
        } else {
            log(&format!("{}: no {} (run kh1-recom-build): Re:CoM's behaviour kept", c.name, c.manifest));
        }
        return false;
    };
    let text = match std::fs::read_to_string(p) {
        Ok(t) => t,
        Err(e) => {
            log(&format!("{}: {}: {e}; Re:CoM's behaviour kept", c.name, p.display()));
            return false;
        }
    };
    let m = match ce::parse_for(k, &text) {
        Ok(m) => m,
        Err(e) => {
            log(&format!("{}: {}: {e}; Re:CoM's behaviour kept", c.name, p.display()));
            return false;
        }
    };
    let vt = bs + c.vtable as usize;
    if super::stats::rtti_name(bs, vt).as_deref() != Some(c.rtti) {
        log(&format!("{}: {}'s vtable is not at {:#x}: not installed", c.name, c.rtti, 0x1_4000_0000u64 + c.vtable as u64));
        return false;
    }
    if let Some(p) = c.ptrs.iter().find(|p| rd::<usize>(bs + p.rva as usize) != bs + p.target as usize) {
        log(&format!("{}: {} is not the function expected: not installed", c.name, p.name));
        return false;
    }
    // The copy: Re:CoM's locator, then its slots with ours in the state slots.
    let original: Vec<usize> = (0..c.slots).map(|i| rd::<usize>(vt + 8 * i)).collect();
    let mut copy: Vec<usize> = Vec::with_capacity(c.slots + 1);
    copy.push(rd::<usize>(vt - 8));
    copy.extend_from_slice(&original);
    let mut set = |off: usize, f: usize| copy[1 + off / 8] = f;
    set(ce::begin_slot(state::APPEAR), appear_begin as *const () as usize);
    set(ce::step_slot(state::APPEAR), appear_step as *const () as usize);
    set(ce::begin_slot(state::THINK), think_begin as *const () as usize);
    set(ce::step_slot(state::THINK), think_step as *const () as usize);
    for &s in c.attack_states {
        set(ce::begin_slot(s), attack_begin as *const () as usize);
        set(ce::step_slot(s), attack_step as *const () as usize);
    }
    for s in [state::FLINCH, state::BLOW] {
        set(ce::begin_slot(s), react_begin as *const () as usize);
        set(ce::step_slot(s), react_step as *const () as usize);
    }
    set(ce::begin_slot(state::DEFEAT), defeat_begin as *const () as usize);
    set(ce::step_slot(state::DEFEAT), defeat_step as *const () as usize);
    set(ce::THINK_SLOT, think_none as *const () as usize);
    // The spawn's init (`+0x38`): Re:CoM's, then the pack check (`init`).
    set(ce::pack::INIT_SLOT, init as *const () as usize);
    // The shared enemy-guard step: Re:CoM's hit handler calls `+0x3D8` when a hit met a guarded body
    // (`0x14017F7E0`: the guard spark, popup 3, then this; `CBattleEnemy`'s is a no-op `0x140181600`).
    set(GUARD_EVENT_SLOT, guard_event as *const () as usize);
    let leaked: &'static mut [usize] = Box::leak(copy.into_boxed_slice());
    let at = leaked.as_ptr() as usize + 8;
    // The allocation size (all-or-nothing with the rest: a failed write leaves everything off).
    let a = c.alloc_site;
    if !super::hook::patch(bs + a.rva as usize, &ce::alloc_bytes(c.alloc), &ce::alloc_bytes(c.alloc + tail::SIZE), a.name) {
        log(&format!("{}: the factory's size held other bytes: not installed", c.name));
        return false;
    }
    let _ = ORIGINAL[k].set(original);
    if m.has_never_hurting() {
        NEVER_HURTING.store(true, Ordering::Release);
    }
    let _ = MANIFEST[k].set(m);
    COPY[k].store(at, Ordering::Release);
    true
}

/// Puts the size back (the ctor hook failed).
pub unsafe fn unprepare(bs: usize, k: usize) {
    let c = &CLASSES[k];
    let a = c.alloc_site;
    super::hook::patch(bs + a.rva as usize, &ce::alloc_bytes(c.alloc + tail::SIZE), &ce::alloc_bytes(c.alloc), a.name);
    COPY[k].store(0, Ordering::Release);
}

pub fn mark_installed(k: usize) {
    INSTALLED[k].store(true, Ordering::Release);
}

pub fn start_line(k: usize) -> String {
    let m = MANIFEST[k].get();
    let c = &CLASSES[k];
    if k == 0 {
        return format!(
            "shadow: KH1's Shadow (M10) at combat = kh1 in Sora's story with enemy_ai = kh1 (read at each battle start): ctor 0x140192770 wrapped, vtable copy at {:#x} (states 0, 1, 2, 3, 4, 11, 12, 30, think 0x3D0), size 0xCE0 -> 0xD00 at 0x140188FAD; {} clips, recoil {:?}; {}",
            COPY[k].load(Ordering::Relaxed),
            m.map_or(0, |m| m.clips.len()),
            m.map(|m| [m.recoil[0], m.recoil[1], m.recoil[2], m.recoil[2]]),
            clash_line(m)
        );
    }
    let states: Vec<String> = ce::replaced_states(c).iter().map(|s| s.to_string()).collect();
    format!(
        "{}: KH1's {} (M11) at combat = kh1 in Sora's story with enemy_ai = kh1 (read at each battle start): ctor {:#x} wrapped, vtable copy at {:#x} (states {}, think 0x3D0), size {:#X} -> {:#X} at {:#x}; {} clips, recoil {:?}; {}",
        c.name,
        c.name,
        0x1_4000_0000u64 + c.ctor.rva as u64,
        COPY[k].load(Ordering::Relaxed),
        states.join(", "),
        c.alloc,
        c.alloc + tail::SIZE,
        0x1_4000_0000u64 + c.alloc_site.rva as u64,
        m.map_or(0, |m| m.clips.len()),
        m.map(|m| m.recoil.clone()),
        clash_line(m)
    )
}

/// The start line's attack-meets-attack part (M11): the rows a swing can meet, or why there are none.
fn clash_line(m: Option<&Manifest>) -> String {
    match m.map(|m| &m.clash) {
        Some(c) if !c.is_empty() => format!("{} attack rows meet Sora's swings (priorities {:?})", c.len(), c.iter().map(|r| r.priority).collect::<Vec<_>>()),
        _ => "no attack rows for clashes (a manifest from an older asset step: run kh1-recom-build again)".into(),
    }
}

// ---------------------------------------------------------------------------------------------
// The ctors and the runtime lookup.
// ---------------------------------------------------------------------------------------------

/// Class `K`'s ctor: Re:CoM's, then (KH1 battle, `enemy_ai = kh1`) our vtable and tail.
pub unsafe extern "win64" fn ctor<const K: usize>(this: usize, a: usize, b: usize, c: usize) -> usize {
    // `ctor_ptr` names a wrapper for 40 indices; only installed classes' are ever called.
    let (Some(o), Some(cl)) = (ORIG_CTOR.get(K), CLASSES.get(K)) else { return 0 };
    let r = orig::<CtorFn>(o)(this, a, b, c);
    if this == 0 || !live_battle(K) {
        return r;
    }
    let copy = COPY[K].load(Ordering::Acquire);
    let Some(m) = MANIFEST[K].get() else { return r };
    if copy == 0 || rd::<usize>(this) != base() + cl.vtable as usize {
        return r;
    }
    let battle = super::battle::battle_number();
    let seed = SEED.fetch_add(0x9E37_79B9, Ordering::Relaxed) ^ (this as u32);
    // Re:CoM's ctor doesn't make the mover (`+0xF0` is still 0 here: 0.67.0's crash at every battle
    // with a Shadow, owner 2026-11-02); the facing is taken again at state 0's begin.
    let m0: usize = rd(this + e::MOVER);
    let yaw = if m0 != 0 { crate::units::yaw_to_kh1(rd::<f32>(m0 + mv::YAW)) } else { 0.0 };
    let ai = ce::make(K, m, seed, yaw);
    // Scripts that share a work block among a room's enemies share it among the battle's (M11, the Neoshadow).
    ai.set_battle(battle);
    let idx = (|| {
        let Ok(mut v) = RT.lock() else { return None };
        if v.iter().any(|x| x.battle != battle) {
            // A new battle: the last one's thrown blades are gone.
            super::kh1_thrown::forget();
            super::kh1_flame::forget();
        }
        // Other battles' runtimes go (they come before this battle's, so no live index moves); an
        // enemy rebuilt at the same address this battle replaces its old runtime **in place**
        // (0.73.0's crash: removing it shifted every later runtime, their tails' indices went stale,
        // the lookup failed and Re:CoM's own state 1 ran without its think: a divide by zero at
        // `0x140197B60`). The lookup goes by address too (`with_rt`).
        // A runtime that goes while its guard flag is on (a KH1 mode-3 row lives until the defeat: the Large Body
        // left behind by a fled battle) gives its count back.
        let gone = v.iter().filter(|x| x.guarded && (x.battle != battle || x.enemy == this)).count();
        if gone > 0 {
            let _ = GUARDED.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| Some(n.saturating_sub(gone)));
        }
        // So does a clash with Sora's swing still waiting (M11, attack meets attack).
        let waiting = v.iter().filter(|x| x.attack_clash.is_some() && (x.battle != battle || x.enemy == this)).count();
        if waiting > 0 {
            let _ = CLASH_WAITING.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| Some(n.saturating_sub(waiting)));
        }
        v.retain(|x| x.battle == battle);
        let slot = v.iter().position(|x| x.enemy == this);
        let fresh = Runtime {
            class: K,
            enemy: this,
            battle,
            ai,
            current: None,
            queued: None,
            queued_start: None,
            action: None,
            action_playing: false,
            reaction: None,
            lie: None,
            clash: None,
            last_pos: Vec3::ZERO,
            recom_appear: false,
            appear_sub: 0,
            dead: false,
            dead_frames: 0,
            hittable: true,
            down: 0.0,
            watch: ce::ClipWatch::default(),
            action_age: 0.0,
            last_state: None,
            recom_flinch: false,
            recom_state: false,
            handing_back: false,
            quiet: 0.0,
            reaction_age: 0.0,
            floor: None,
            base_scale: None,
            hits: 0,
            guard: None,
            guard_under: None,
            guard_live: false,
            guarded: false,
            guard_saved: Vec::new(),
            guard_side: None,
            clash_recoil: None,
            sight_bone: sight_bone(m),
            launch_base: 0,
            attack_clash: None,
        };
        Some(match slot {
            Some(i) => {
                v[i] = fresh;
                i
            }
            None => {
                v.push(fresh);
                v.len() - 1
            }
        })
    })();
    let Some(idx) = idx else { return r };
    wr(this, copy);
    wr(this + cl.alloc as usize + tail::OFF_MAGIC, cl.magic);
    wr(this + cl.alloc as usize + tail::OFF_BATTLE, battle);
    wr(this + cl.alloc as usize + tail::OFF_INDEX, idx as u32);
    wr(this + cl.alloc as usize + tail::OFF_UNHITTABLE, 0u32);
    let name = cl.title;
    line(K, || format!("battle {battle}: {name} {this:#x} gets KH1's AI (seed {seed:#x})"));
    r
}

/// (class, Re:CoM id) pairs whose pack line was logged (`ce::pack::first_time`).
static PACK_SEEN: Mutex<Vec<(usize, u32)>> = Mutex::new(Vec::new());

/// What the enemy's own motion pack holds for its class's clips (`ce::pack`; Re:CoM's set as `0x14024DC60` reads it).
unsafe fn pack_verdict(en: usize, k: usize) -> ce::pack::Verdict {
    use ce::pack as p;
    let ctrl: usize = rd(en + p::CONTROLLER);
    let set: usize = if ctrl != 0 { rd(ctrl + p::SET) } else { 0 };
    let table: usize = if set != 0 { rd(set + p::TABLE) } else { 0 };
    let clips: usize = if table != 0 { rd(table + p::CLIPS) } else { 0 };
    let count = (set != 0).then(|| rd::<i32>(set + p::COUNT));
    p::check(count, &manifest(k).clips, |s| clips != 0 && rd::<usize>(clips + 8 * s as usize) != 0)
}

/// Undoes the ctor's swap: Re:CoM's vtable, no tail magic, no runtime. Before any state of ours ran (the spawn's
/// init), so the enemy is Re:CoM's from its first state on, as at `enemy_ai = recom`.
unsafe fn give_back(en: usize, k: usize) {
    let c = CLASSES[k];
    wr(en, base() + c.vtable as usize);
    wr(en + c.alloc as usize + tail::OFF_MAGIC, 0u32);
    if let Ok(mut v) = RT.lock() {
        // Lookups go by address when an index no longer matches (`with_rt`), so later runtimes may move.
        v.retain(|x| x.enemy != en);
    }
}

/// The spawn's init (vtable `+0x38`, `ce::pack::INIT_SLOT`) of a converted enemy: Re:CoM's (it builds the model and
/// its motion controller), then the enemy keeps KH1's AI only when its own pack carries every clip its manifest
/// plays (owner 2026-11-03: T-posed Shadows of Re:CoM id 32, whose `ENEMY/0032` pack the asset step doesn't extend);
/// else it goes back to Re:CoM's class (logged once per class and id). A failed init (the spawn deletes the enemy)
/// gives the runtime back too. Once per spawned enemy.
pub unsafe extern "win64" fn init(en: usize, kind: usize, id: usize, d: usize) -> usize {
    let vt: usize = rd(en);
    let Some(k) = (0..N).find(|&k| COPY[k].load(Ordering::Relaxed) == vt) else { return 0 };
    let f = original(k, ce::pack::INIT_SLOT);
    if f == 0 {
        return 0;
    }
    let g: CtorFn = std::mem::transmute_copy(&f);
    let r = g(en, kind, id, d);
    if r as u32 == 0 {
        give_back(en, k);
        return r;
    }
    let v = pack_verdict(en, k);
    let id = id as u32;
    let c = CLASSES[k];
    if PACK_SEEN.lock().is_ok_and(|mut s| ce::pack::first_time(&mut s, k, id)) {
        log(&ce::pack::line(c.name, c.title, id, v));
    }
    if !v.carries() {
        give_back(en, k);
        let battle = super::battle::battle_number();
        line(k, || format!("battle {battle}: {} {en:#x} (Re:CoM id {id}) back to Re:CoM's AI: its motion pack lacks the KH1 clips ({v:?})", c.title));
    }
    r
}

/// Class `k`'s ctor wrapper (`ctor::<K>` for every class index; room for 40 classes so a new class
/// needs no line here).
pub fn ctor_ptr(k: usize) -> usize {
    const _: () = assert!(N <= 40);
    match k {
        0 => ctor::<0> as *const () as usize,
        1 => ctor::<1> as *const () as usize,
        2 => ctor::<2> as *const () as usize,
        3 => ctor::<3> as *const () as usize,
        4 => ctor::<4> as *const () as usize,
        5 => ctor::<5> as *const () as usize,
        6 => ctor::<6> as *const () as usize,
        7 => ctor::<7> as *const () as usize,
        8 => ctor::<8> as *const () as usize,
        9 => ctor::<9> as *const () as usize,
        10 => ctor::<10> as *const () as usize,
        11 => ctor::<11> as *const () as usize,
        12 => ctor::<12> as *const () as usize,
        13 => ctor::<13> as *const () as usize,
        14 => ctor::<14> as *const () as usize,
        15 => ctor::<15> as *const () as usize,
        16 => ctor::<16> as *const () as usize,
        17 => ctor::<17> as *const () as usize,
        18 => ctor::<18> as *const () as usize,
        19 => ctor::<19> as *const () as usize,
        20 => ctor::<20> as *const () as usize,
        21 => ctor::<21> as *const () as usize,
        22 => ctor::<22> as *const () as usize,
        23 => ctor::<23> as *const () as usize,
        24 => ctor::<24> as *const () as usize,
        25 => ctor::<25> as *const () as usize,
        26 => ctor::<26> as *const () as usize,
        27 => ctor::<27> as *const () as usize,
        28 => ctor::<28> as *const () as usize,
        29 => ctor::<29> as *const () as usize,
        30 => ctor::<30> as *const () as usize,
        31 => ctor::<31> as *const () as usize,
        32 => ctor::<32> as *const () as usize,
        33 => ctor::<33> as *const () as usize,
        34 => ctor::<34> as *const () as usize,
        35 => ctor::<35> as *const () as usize,
        36 => ctor::<36> as *const () as usize,
        37 => ctor::<37> as *const () as usize,
        38 => ctor::<38> as *const () as usize,
        39 => ctor::<39> as *const () as usize,
        _ => 0,
    }
}

/// The class of an enemy that is one of ours (our vtable and tail).
pub(super) unsafe fn ours(en: usize) -> Option<usize> {
    let vt: usize = rd(en);
    (0..N).find(|&k| {
        let c = COPY[k].load(Ordering::Relaxed);
        c != 0 && vt == c && rd::<u32>(en + CLASSES[k].alloc as usize + tail::OFF_MAGIC) == CLASSES[k].magic
    })
}

/// KH1's "can it be hit" for one of our KH1-AI enemies (its runtime's flag 0x40, through the tail's copy: no lock, so
/// another enemy's command can ask inside its own runtime call): `Some(false)` while it can't be hit, `Some(true)` while it
/// can, `None` for any other enemy (Re:CoM's AI, or a runtime of another battle).
pub(super) unsafe fn kh1_hittable(en: usize) -> Option<bool> {
    let k = ours(en)?;
    let at = en + CLASSES[k].alloc as usize;
    (rd::<u32>(at + tail::OFF_BATTLE) == super::battle::battle_number()).then(|| rd::<u32>(at + tail::OFF_UNHITTABLE) == 0)
}

/// Runs `f` on the enemy's runtime; None when it has none (then the caller runs Re:CoM's slot).
unsafe fn with_rt<T>(en: usize, f: impl FnOnce(&mut Runtime) -> T) -> Option<T> {
    let k = ours(en)?;
    let at = CLASSES[k].alloc as usize;
    let battle: u32 = rd(en + at + tail::OFF_BATTLE);
    let idx: u32 = rd(en + at + tail::OFF_INDEX);
    // `try_lock`: a state call re-entered from inside another (never expected) runs Re:CoM's slot.
    let mut v = match RT.try_lock() {
        Ok(v) => v,
        Err(_) => {
            miss(en, k, "the runtime table is busy (a re-entered state call)");
            return None;
        }
    };
    // By index when it still matches, else by address (an index can only go stale if the table
    // was rebuilt).
    let i = if v.get(idx as usize).is_some_and(|r| r.enemy == en && r.battle == battle) {
        Some(idx as usize)
    } else {
        v.iter().position(|r| r.enemy == en && r.battle == battle)
    };
    let Some(i) = i else {
        drop(v);
        miss(en, k, "no runtime for it");
        return None;
    };
    Some(f(&mut v[i]))
}

/// Logged once per enemy: one of ours whose runtime the lookup didn't find.
unsafe fn miss(en: usize, k: usize, why: &str) {
    static SEEN: Mutex<Vec<usize>> = Mutex::new(Vec::new());
    if let Ok(mut s) = SEEN.lock() {
        if !s.contains(&en) {
            if s.len() > 256 {
                s.clear();
            }
            s.push(en);
            log(&format!("{}: e{}: KH1 runtime lookup failed ({why}); this frame skips its KH1 state (Re:CoM's own state 1 needs the think we replaced)", CLASSES[k].name, rd::<u32>(en + e::ID)));
        }
    }
}

unsafe fn original(k: usize, off: usize) -> usize {
    ORIGINAL[k].get().map_or(0, |o| o[off / 8])
}

/// Calls Re:CoM's own slot of the enemy's class (`k`: its class; the Shadow's when unknown).
unsafe fn call_original(en: usize, off: usize) {
    let k = ours(en).unwrap_or(0);
    let f = original(k, off);
    if f != 0 {
        let g: StateFn = std::mem::transmute_copy(&f);
        g(en);
    }
}

// ---------------------------------------------------------------------------------------------
// The KH1 frame.
// ---------------------------------------------------------------------------------------------

fn manifest(k: usize) -> &'static Manifest {
    MANIFEST[k].get().expect("installed")
}

unsafe fn mover(en: usize) -> usize {
    rd(en + e::MOVER)
}

/// The enemy's KH1 place (its Re:CoM mover's position in KH1 units, as each frame's `World::pos`), for a thrown object that
/// starts from it (M11, the Red Nocturne's fires: `sys 19(parent, 0x8001, t)`).
pub(super) unsafe fn kh1_place(en: usize) -> Option<Vec3> {
    let m = mover(en);
    (m != 0).then(|| crate::units::to_kh1(rd_vec(m + mv::POS)))
}

pub(super) unsafe fn sora_pos() -> Option<Vec3> {
    let pl: usize = rd(super::va(base(), data::MAIN_PLAYER));
    if pl == 0 {
        return None;
    }
    let m: usize = rd(pl + e::MOVER);
    (m != 0).then(|| crate::units::to_kh1(rd_vec(m + mv::POS)))
}

unsafe fn clock(en: usize) -> i32 {
    func::<ClockFn>(f::CLOCK)(en)
}

/// Plays a KH1 request through Re:CoM's motion play (vtable `+0x70`): its clip, root flag, the
/// blend in Re:CoM's clock units (KH1 frames × 50; Assumption: the play's blend argument counts
/// the same units as the clock).
unsafe fn play(en: usize, rt: &mut Runtime, request: u16, blend: f32) {
    play_at(en, rt, request, blend, 0.0);
}

/// [`play`] from `start` KH1 frames (`kh1_core::enemy::Cmd::MotionAt`): the play's third argument is
/// the start time in clock units (`0x1401A3470` → `0x14024D840` → `0x14024C760` → `0x14024E9F0`:
/// the controller's time `+0x3C` = it × its speed `+0x4C`, which the clock `0x1401A47C0` reads;
/// Checked (decomp + disasm)). Assumption: the speed is 1 for our clips.
unsafe fn play_at(en: usize, rt: &mut Runtime, request: u16, blend: f32, start: f32) {
    let Some(c) = manifest(rt.class).clip(request) else {
        line(rt.class, || format!("{en:#x}: no clip for KH1 request {request}"));
        return;
    };
    let vt: usize = rd(en);
    let p: PlayFn = std::mem::transmute_copy(&rd::<usize>(vt + 0x70));
    let from = crate::units::frames_to_units(start.max(0.0)) as u64;
    p(en, c.slot, from, c.root as u64, crate::shadow::blend_units(blend), 0, 0);
    if !rt.hittable && rt.current != Some(request) {
        // Diagnostic (the sunk Shadow that shows standing): every motion change while unhittable.
        let from = rt.current;
        line(rt.class, || format!("e{}: plays {request} (slot {}) after {from:?} while unhittable", rd::<u32>(en + e::ID), c.slot));
    }
    rt.current = Some(request);
    rt.watch.restart();
}

/// The current request's clip has played to KH1's length, or (a clip Re:CoM loops) its clock
/// wrapped (`ce::ClipWatch`).
unsafe fn ended(en: usize, rt: &mut Runtime) -> bool {
    match rt.current.and_then(|r| manifest(rt.class).clip(r)) {
        Some(c) => rt.watch.ended(clock(en), c.frames as f32),
        None => true,
    }
}

/// The safety net: the running action is stuck (`ce::watchdog`), with the loud log line's detail.
unsafe fn stuck(en: usize, rt: &Runtime) -> Option<(ce::watchdog::Stuck, String)> {
    let a = rt.action?;
    if a.steered && rt.action_playing {
        // The AI runs during a steered attack (M11 fliers): nothing waits on it.
        return None;
    }
    let frames = manifest(rt.class).clip(a.request).map_or(0.0, |c| c.frames as f32);
    let why = ce::watchdog::check(rt.action_playing, rt.action_age, frames)?;
    let st = crate::guard::state_index(en, rd(en + e::STATE)).map_or("?".to_string(), |s| s.to_string());
    let detail = match why {
        ce::watchdog::Stuck::NeverStarted => match a.attack {
            Some(i) => format!("Re:CoM state {st}, its attack state {} never began, {:.0} frames", CLASSES[rt.class].attack_states.get(i as usize).copied().unwrap_or(2), rt.action_age),
            None => format!("Re:CoM state {st}, {:.0} frames", rt.action_age),
        },
        ce::watchdog::Stuck::NeverEnded => format!(
            "Re:CoM state {st}, playing {:?}, clock {} of {} units, {:.0} of {frames:.0} frames",
            rt.current,
            clock(en),
            crate::units::frames_to_units(frames),
            rt.action_age
        ),
    };
    Some((why, detail))
}

/// Logs a released action (always, past the class's line cap).
unsafe fn release_line(en: usize, rt: &Runtime, why: ce::watchdog::Stuck, detail: &str) {
    let req = rt.action.map_or(0, |a| a.request);
    log(&ce::watchdog::line(CLASSES[rt.class].name, rd(en + e::ID), req, why, detail));
}

/// KH1's hittable flag (actor flag `0x40`, `kh1_core::enemy::Cmd::Hittable`) as Re:CoM's own
/// hurt-body switch: every hurt body off (`0x1401A2D70(enemy, −1)`) or on (`0x1401A2DB0`), what
/// Re:CoM's dig does (`0x140192E60` case 0 / 5). Re:CoM's hit handler, the KH1 swings' pair tests
/// and the spells' overlap tests only meet bodies that are on. The lock-on flag (`+0x154` bit
/// 0x10000, `0x1401CDB90`) stays: KH1's lock button (`0x1402B1540`) can lock it too.
unsafe fn hurt(en: usize, on: bool) {
    func::<BodiesFn>(if on { f::HURT_ON } else { f::HURT_OFF })(en, -1);
}

unsafe fn bodies(en: usize, on: bool) {
    if on {
        func::<BodiesFn>(f::HURT_ON)(en, -1);
        func::<BodiesFn>(f::OTHER_ON)(en, -1);
    } else {
        func::<BodiesFn>(f::HURT_OFF)(en, -1);
        func::<BodiesFn>(f::OTHER_OFF)(en, -1);
    }
}

/// KH1's `+0x44` (the y of the scale) as Re:CoM's model scale y (`0x1401A4A20` / `0x1401A7050`,
/// mover `+0x70`: what Re:CoM's own dig squashes, `0x140193420`).
unsafe fn set_height(en: usize, h: f32) {
    let p = func::<ScaleFn>(f::SCALE)(en);
    if p == 0 {
        return;
    }
    let mut s: [f32; 4] = rd(p);
    s[1] = h.max(0.0);
    func::<ScaleSetFn>(f::SCALE_SET)(en, &s);
}

/// KH1's scale (`+0x40..+0x48`, `kh1_core::enemy::Cmd::Scale`) as a factor on Re:CoM's own model
/// scale (read before the first write), through Re:CoM's scale getter / setter (`0x1401A4A20` /
/// `0x1401A7050`, as [`set_height`]). A factor under 0.001 is written as 0.001 (Assumption: a zero
/// scale would leave Re:CoM's model matrix singular; the model is as invisible at 0.001).
unsafe fn set_scale(en: usize, rt: &mut Runtime, k: f32) {
    let p = func::<ScaleFn>(f::SCALE)(en);
    if p == 0 {
        return;
    }
    let base = *rt.base_scale.get_or_insert_with(|| rd(p));
    let k = if k.is_finite() { k.max(0.001) } else { 1.0 };
    let s = [base[0] * k, base[1] * k, base[2] * k, base[3]];
    func::<ScaleSetFn>(f::SCALE_SET)(en, &s);
}

/// KH1's actor flag `0x1` (the model drawn, `kh1_core::enemy::Cmd::Visible`) as Re:CoM's own hide
/// bit `enemy+0x154` bit 0: the actor draw `0x1401A39B0` (every actor class's slot) draws nothing
/// while it is set, as the player's hide state sets it (`0x1401E5E80`) and `CEnemyShadow`'s state 0
/// clears it (`0x140192850`). Checked (decomp).
unsafe fn set_visible(en: usize, on: bool) {
    let f: u32 = rd(en + HIDE);
    wr(en + HIDE, if on { f & !1 } else { f | 1 });
}

/// Re:CoM's actor flags holding the hide bit (bit 0).
const HIDE: usize = 0x154;

/// Enters a Re:CoM attack state as P1's cardless play does (`crate::battle::cardless_flags`).
unsafe fn enter_state(en: usize, k: i32) {
    let flags: u64 = rd(en + e::FLAGS);
    wr(en + e::FLAGS, crate::battle::cardless_flags(flags));
    wr(en + e::CHOSEN, k);
    wr(en + e::NEXT, crate::battle::state_address(en, k));
}

unsafe fn apply(en: usize, rt: &mut Runtime, c: &Cmd<u8>) {
    match *c {
        Cmd::Motion { request, blend } => {
            if rt.current != Some(request) {
                rt.queued = None;
                play(en, rt, request, blend);
            }
        }
        Cmd::Queue { request, blend } => {
            rt.queued = Some((request, blend));
            rt.queued_start = None;
        }
        // KH1's `sys 14` with a start frame (M11, the Yellow Opera's Thunder: 206 from frame 2, 207 from frame 3).
        Cmd::QueueAt { request, blend, start } => {
            rt.queued = Some((request, blend));
            rt.queued_start = Some((request, start));
        }
        Cmd::Action { request, attack, then, blend } => {
            rt.action = Some(Act { request, then, attack, blend, steered: false, start: 0.0 });
            rt.action_playing = false;
            rt.action_age = 0.0;
            match attack {
                // An attack: Re:CoM's attack state; its begin plays the motion.
                Some(a) => enter_state(en, CLASSES[rt.class].attack_states.get(a as usize).copied().unwrap_or(2)),
                None => {
                    play(en, rt, request, blend);
                    rt.queued = Some((then, 0.0));
                    rt.action_playing = true;
                }
            }
        }
        // Log only: what can hit it is KH1's flag 0x40 (`Hittable`), not the underground state.
        Cmd::Underground(_) => {}
        Cmd::Hittable(h) => {
            hurt(en, h);
            rt.hittable = h;
            wr(en + CLASSES[rt.class].alloc as usize + tail::OFF_UNHITTABLE, (!h) as u32);
        }
        Cmd::Height(h) => set_height(en, h),
        Cmd::FadeIn(n) => {
            // KH1's alpha fade-in (`0x2570` / `0x2010`): no Re:CoM model alpha found to write yet
            // (`docs/recom-m10-shadow.md` § Not built).
            let _ = n;
        }
        // KH1's flag 0x1000: the action's root motion carries the body (`kh1_core::enemy::Body::held`).
        Cmd::Held => rt.ai.body().held = true,
        // A plain request with attack rows (M11 fliers): Re:CoM's attack state, the AI keeps
        // running; its begin plays the motion (no queue: KH1's request replaces the queue).
        Cmd::Attack { request, attack, blend } | Cmd::AttackAt { request, attack, blend, .. } => {
            // M11 (the Blue Rhapsody's melee): a steered attack from a later frame (`sys 13`'s start), played from there by
            // its state's begin (Re:CoM's records tick on the same clock, `0x14014E830` reads `0x1401A47C0`).
            let start = if let Cmd::AttackAt { start, .. } = *c { start } else { 0.0 };
            rt.action = Some(Act { request, then: ke::react::req::IDLE, attack: Some(attack), blend, steered: true, start });
            rt.action_playing = false;
            rt.action_age = 0.0;
            enter_state(en, CLASSES[rt.class].attack_states.get(attack as usize).copied().unwrap_or(2));
            if ke::host::starts_at_once(rt.ai.grab_attack(), attack, crate::guard::state_index(en, rd(en + e::STATE))) {
                // KH1's grab (M11, the Neoshadow): played now (its records attach, `0x14017D750`), its record tested
                // against Sora once this state step returns ([`grab_test_now`]); the attack state's begin keeps it.
                play_at(en, rt, request, blend, start);
                rt.queued = None;
                rt.action_playing = true;
                GRAB_NOW.store(en, Ordering::Relaxed);
            }
        }
        Cmd::Scale(k) => set_scale(en, rt, k),
        Cmd::Visible(v) => set_visible(en, v),
        Cmd::MotionAt { request, blend, start } => {
            if rt.current != Some(request) {
                rt.queued = None;
                play_at(en, rt, request, blend, start);
            }
        }
        // KH1's `sys 13` of the request playing: it starts again (M11, the Fat Bandit's fireballs).
        Cmd::Restart { request, blend, start } => {
            rt.queued = None;
            play_at(en, rt, request, blend, start);
        }
        Cmd::Guard(Some((row, request))) => {
            let m = manifest(rt.class);
            match m.guards.get(row as usize) {
                Some(g) if g.half_angle().is_some() => {
                    // KH1's mode-3 row isn't removed by another row the script makes (it lives until `sys 213(row, 0)`):
                    // it waits under a windowed one (the Defender's front row under its spin's).
                    if request == ke::guard::ALWAYS {
                        rt.guard_under = None;
                    } else if rt.guard.is_some_and(|o| o.request == ke::guard::ALWAYS && o.row != row) {
                        rt.guard_under = rt.guard;
                    }
                    rt.guard = Some(GuardOn { row, request, seen: false, age: 0.0 });
                }
                Some(g) => line(rt.class, || format!("{}: guard row {} has a turned cone ({} rad): Re:CoM's body guard can't hold it; not built", rd::<u32>(en + e::ID), g.name, g.turn)),
                None => line(rt.class, || format!("{}: no guard row {row} in the manifest", rd::<u32>(en + e::ID))),
            }
        }
        Cmd::Guard(None) => {
            rt.guard = None;
            rt.guard_under = None;
            guard_bodies(en, rt, None);
        }
        // M11, the Green Requiem: its spell child is a support spell (Re:CoM's own heal on the ally KH1 picked).
        Cmd::Throw(n) if super::kh1_cure::owns(CLASSES[rt.class].name) => super::kh1_cure::cure(en, n),
        Cmd::Throw(n) => {
            let yaw = rt.ai.body().yaw;
            let _ = super::kh1_thrown::throw(en, CLASSES[rt.class].name, manifest(rt.class), n, yaw);
        }
        // M11, the Yellow Opera's Thunder: Re:CoM's own bolt at KH1's mark.
        Cmd::Strike { child, at } => {
            let _ = super::kh1_thrown::strike(en, CLASSES[rt.class].name, manifest(rt.class), child, at);
        }
        // M11, the Yellow Opera's Thunder marked its place: one of Re:CoM's own effects there until the strike (the stand-in
        // for KH1's warning cloud, owner 2026-11-03).
        Cmd::Warn { child, at } => super::kh1_thrown::warn(en, CLASSES[rt.class].name, manifest(rt.class), child, at),
        // Log only (KH1 does something here the port doesn't build: the line below says what).
        Cmd::Note(_) => {}
        Cmd::HealFull => heal_full(en, rt),
        Cmd::Support { child, draw } => super::kh1_cure::pick(en, manifest(rt.class), child, draw),
        Cmd::Reward(r) => reward(en, rt, r),
        Cmd::Yaw(_) | Cmd::Jump { .. } | Cmd::Target(_) | Cmd::State(_) => {}
    }
    if ce::logged(c) {
        let id: u32 = rd(en + e::ID);
        let k = rt.class;
        let tok = ce::cmd_token(c, CLASSES[k].attack_names, &|s| rt.ai.state_name(s));
        line(k, || format!("e{id}: {tok}"));
    }
}

/// The host rules' view of an action (`kh1_core::enemy::host`).
fn running(a: Act, playing: bool) -> ke::host::Running {
    ke::host::Running { request: a.request, attack: a.attack.is_some(), steered: a.steered, playing }
}

/// The Re:CoM attack state an action's attack plays in (the class's map; 2 for none).
fn attack_state(class: usize, a: Act) -> i32 {
    a.attack.and_then(|i| CLASSES[class].attack_states.get(i as usize).copied()).unwrap_or(2)
}

/// The action that played ended: the AI goes on; KH1's flag 0x1000 goes with its motion
/// (Assumption: cleared when the next motion, requested without bit 23, starts).
fn end_action(rt: &mut Runtime) {
    rt.action = None;
    rt.action_playing = false;
    rt.ai.body().held = false;
}

/// One KH1 frame of this enemy: landing, the motion queue, the action's end, the behaviour (when
/// `poll`), the body physics into Re:CoM's mover.
unsafe fn tick(en: usize, rt: &mut Runtime, poll: bool) {
    let mvr = mover(en);
    if mvr == 0 {
        return;
    }
    let dt: i32 = rd(en + e::DT);
    let step = crate::units::step_from_dt(dt.clamp(0, 1000));
    let pos = crate::units::to_kh1(rd_vec(mvr + mv::POS));
    let moved = (pos - rt.last_pos).length();
    rt.last_pos = pos;
    let contact = rd::<u32>(mvr + mv::CONTACT) & 5 != 0;
    let flying = rt.ai.body().flying;
    if flying {
        floor_note(rt);
    }
    rt.watch.tick(step);
    // Diagnostic (owner 2026-11-02: a sunk Shadow sometimes shows standing mid-travel): a Re:CoM
    // state change while it cannot be hit (sinking, underground, rising) is logged with the KH1
    // motion playing.
    let st = crate::guard::state_index(en, rd(en + e::STATE));
    if st != rt.last_state {
        if !rt.hittable && rt.last_state.is_some() {
            let (from, cur) = (rt.last_state, rt.current);
            line(rt.class, || format!("e{}: Re:CoM state {from:?} -> {st:?} while unhittable (KH1 motion {cur:?})", rd::<u32>(en + e::ID)));
        }
        rt.last_state = st;
    }
    if let Some(a) = rt.action.filter(|a| a.attack.is_some() && !rt.action_playing) {
        // An attack whose Re:CoM attack state has not begun: asked again each frame while Re:CoM's
        // next state is another (something overwrote it, e.g. a default state in the same frame).
        let want = CLASSES[rt.class].attack_states.get(a.attack.unwrap_or(0) as usize).copied().unwrap_or(2);
        let is = |v: usize| crate::guard::state_index(en, v).map(|k| k as i32);
        if is(rd(en + e::NEXT)) != Some(want) && is(rd(en + e::STATE)) != Some(want) && rt.action_age > 0.0 {
            line(rt.class, || format!("e{}: attack state {want} asked again (Re:CoM's next state was {:?})", rd::<u32>(en + e::ID), is(rd(en + e::NEXT))));
            enter_state(en, want);
        }
    }
    if poll && rt.action.is_some_and(|a| ke::host::attack_state_left(running(a, rt.action_playing), st, attack_state(rt.class, a))) {
        // A playing attack in a polled frame outside its Re:CoM attack state (state 0 / 1): its
        // state was left without its end (a Re:CoM state took over and handed back): released. A
        // steered attack polled in its own attack state (`attack_step`) is kept.
        let detail = format!("its Re:CoM attack state was left without its end; now Re:CoM state {:?}", st);
        release_line(en, rt, ce::watchdog::Stuck::NeverEnded, &detail);
        end_action(rt);
        rt.queued = None;
        play(en, rt, ke::react::req::IDLE, manifest(rt.class).consts.get("attack_blend"));
    }
    if rt.action.is_some() {
        rt.action_age += step;
        // The safety net (an attack that plays is `attack_step`'s: it leaves the attack state).
        if let Some((why, detail)) = stuck(en, rt) {
            if why == ce::watchdog::Stuck::NeverStarted || rt.action.is_some_and(|a| a.attack.is_none()) {
                release_line(en, rt, why, &detail);
                // As if its motion had finished: the queued motion plays, the AI goes on (a steered
                // attack queues nothing: the AI plays its next motion itself).
                let then = rt.action.filter(|a| !a.steered).map(|a| (a.then, a.blend));
                end_action(rt);
                if let Some((q, b)) = then {
                    rt.queued = None;
                    play(en, rt, q, b);
                }
            }
        }
    }
    // A flier's timed hop (KH1's movement state 5, M11: the Blue Rhapsody's) ends back in flight once it falls to the height
    // above the battle floor it left from, or touches the floor (`0x1402AFAA0`); one load for every other body.
    if rt.ai.body().flight_hop.is_some() {
        let h = rt.floor.map(|f| f - pos.y);
        let _ = rt.ai.body().flight_hop_done(h, contact);
    }
    if contact && rt.ai.body().falling() {
        rt.ai.body().land();
        landed(en, rt);
    }
    // The motion that ended: the queued one plays; an action ends.
    if ended(en, rt) {
        if let Some(a) = rt.action.filter(|_| rt.action_playing) {
            if rt.current == Some(a.request) && a.attack.is_none() {
                end_action(rt);
            }
        }
        if let Some((q, b)) = rt.queued.take() {
            let start = rt.queued_start.take().filter(|s| s.0 == q).map_or(0.0, |s| s.1);
            play_at(en, rt, q, b, start);
        }
    }
    let airborne = rt.ai.body().airborne();
    // A behaviour that looks from one of its bones (M11, the Search Ghost's cone): the bone's place
    // and axis and Sora's lock point, from Re:CoM's own animated bones.
    if let Some(s) = sight(en, rt.sight_bone) {
        rt.ai.set_sight(s);
    }
    if !(poll && rt.action.is_none_or(|a| a.steered)) {
        // KH1's child objects run every frame, also while the parent's script waits (M11), and after its death while
        // Re:CoM keeps it (state 30; M11, the Red Nocturne's spells test its motion, not its HP: `Behaviour::dead_frame`,
        // nothing for the other classes).
        let w = World { step, pos, target: sora_pos(), grounded: contact && !airborne && !flying, airborne, moved, motion: rt.current };
        let cmds = if rt.dead { rt.ai.dead_frame(w) } else { rt.ai.side_frame(w) };
        for c in &cmds {
            apply(en, rt, c);
        }
    }
    if poll && rt.action.is_none_or(|a| a.steered) {
        // KH1's movement state 4 (flight) is neither state 0 nor state 2.
        let grounded = contact && !airborne && !flying;
        if let Some(f) = rt.floor {
            rt.ai.set_floor(f);
        }
        rt.ai.set_targeted(sora_targets(en));
        rt.ai.set_target_grounded(sora_grounded());
        // M11 (the Neoshadow's grab): Sora's side of KH1's grab, for a behaviour that reads it.
        if rt.ai.wants_hold() {
            rt.ai.set_target_hold(super::held::hold());
        }
        let w = World { step, pos, target: sora_pos(), grounded, airborne, moved, motion: rt.current };
        let cmds = rt.ai.frame(w);
        for c in &cmds {
            apply(en, rt, c);
        }
        // A flier's script that steers its body this frame (`+0x20` × `+0x2C`) is acting too (the
        // Darkball's invisible glide issues no command for its whole length; M11).
        let steering = rt.ai.body().walk != Vec3::ZERO;
        // The safety net for a silent AI (no command at all for `QUIET_LIMIT` frames, no action and
        // no reaction pending): logged past the cap, the behaviour restarts from its engaged state.
        if !cmds.is_empty() || steering || rt.reaction.is_some() || st != Some(state::THINK as usize) {
            rt.quiet = 0.0;
        } else {
            rt.quiet += step;
            if rt.quiet >= ce::watchdog::QUIET_LIMIT {
                rt.quiet = 0.0;
                log(&format!("{}: e{}: the AI issued nothing for {:.0} frames in Re:CoM state {st:?} (playing {:?}), restarted", CLASSES[rt.class].name, rd::<u32>(en + e::ID), ce::watchdog::QUIET_LIMIT, rt.current));
                rt.ai.reset();
            }
        }
        let draws = rt.ai.take_draws();
        if !draws.is_empty() {
            let id: u32 = rd(en + e::ID);
            line(rt.class, || format!("e{id}: rand {}", draws.iter().map(|d| format!("{}={:.4}", d.what, d.value)).collect::<Vec<_>>().join(", ")));
        }
    }
    body_step(en, rt, step, pos);
    guard_step(en, rt, step);
    ring_step(en, rt);
    // M11, the Fat Bandit's flame (KH1's child `ex_2060_01.bd`): its spheres on Re:CoM's bullets (one atomic load while
    // no flame burns).
    let yaw = rt.ai.body().yaw;
    super::kh1_flame::step(en, manifest(rt.class), rt.current, !rt.dead, yaw, step);
    // M11, the Gargoyle's ball (KH1's child `ex_2220_00.bd`): its parent's request, points, facing and floor (one atomic
    // load while no ball lives).
    super::kh1_thrown::ball_parent_step(en, manifest(rt.class), rt.current, !rt.dead, yaw, rt.floor, step);
    // KH1's hit counter goes back to 0 at the end of an update with no action playing (actor flag
    // 4: an action of ours the AI waits on, or a reaction).
    if rt.reaction.is_none() && rt.action.is_none_or(|a| a.steered) {
        rt.hits = 0;
    }
}

/// KH1's body physics for the frame, written into Re:CoM's mover: the facing, and the velocity
/// that moves it by KH1's displacement (horizontal always; vertical while in a KH1 arc, with
/// Re:CoM's acceleration cancelled). While KH1's flag 0x1000 holds the body, Re:CoM's root motion
/// moves it (the clip's root flag) and our velocity is only what cancels the mover's acceleration.
unsafe fn body_step(en: usize, rt: &mut Runtime, step: f32, pos: Vec3) {
    let mvr = mover(en);
    let c = &manifest(rt.class).consts;
    let steer = rt.ai.take_steer();
    let airborne = rt.ai.body().airborne();
    let flying = rt.ai.body().flying;
    let d = rt.ai.body().step(step, pos.y, steer, c);
    let yaw = rt.ai.body().yaw;
    wr(mvr + mv::YAW, crate::units::yaw_to_recom(yaw));
    let dr = crate::units::to_recom(d);
    let accel = rd_vec(mvr + mv::ACCEL);
    let v = crate::shadow::velocity(dr, accel, step);
    let scale: f32 = rd(mvr + mv::SPEED_SCALE);
    let k = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
    let mut vel: [f32; 4] = rd(mvr + mv::VEL);
    vel[0] = v.x * k;
    vel[2] = v.z * k;
    if airborne {
        vel[1] = v.y;
    } else if flying {
        // KH1's flight moves the body vertically too (`0x1402CC000`), at the same speed scale.
        vel[1] = v.y * k;
    }
    wr(mvr + mv::VEL, vel);
}

/// The bone sight of a class whose manifest names a `sight_bone` (M11, the Search Ghost's cone,
/// `kh1_core::enemy::Sight`), in KH1's axes: the bone's world position (Re:CoM's bone node `+0xD0`)
/// and its world row 0 (`+0xA0`: the node's scaled world rows `+0xA0..+0xC0`, `0x14023EB00`; Checked
/// (decomp)); the converter poses each Re:CoM bone at its KH1 twin's world (`kh1_recom_assets::convert`),
/// so these are KH1's own. Sora's KH1 lock point 0 is his bone 77, Re:CoM's `chest1` (Checked (files):
/// `xa_ex_0010.mdls` part 4's last group, code 8 at t 0 → bone 78's parent 77 ↔ `PL0001.MDL` bone 37),
/// whose index Re:CoM keeps at `player+0x3A8` (`0x1401A4450(player, "chest1")`, Checked (decomp)).
unsafe fn sight(en: usize, bone: Option<i32>) -> Option<ke::Sight> {
    let node = func::<BoneFn>(f::BONE)(en, bone?);
    if node == 0 {
        return None;
    }
    let eye = crate::units::to_kh1(rd_vec(node + 0xD0));
    let r0 = rd_vec(node + 0xA0);
    let axis = Vec3::new(r0.x, -r0.y, -r0.z);
    let pl: usize = rd(super::va(base(), data::MAIN_PLAYER));
    let target = if pl != 0 {
        let i: i32 = rd(pl + SORA_CHEST);
        let n = if (0..512).contains(&i) { func::<BoneFn>(f::BONE)(pl, i) } else { 0 };
        (n != 0).then(|| crate::units::to_kh1(rd_vec(n + 0xD0)))
    } else {
        None
    };
    (eye.length().is_finite() && axis.length().is_finite()).then_some(ke::Sight { eye, axis, target: target.filter(|t| t.length().is_finite()) })
}

/// The manifest's `sight_bone` (a Re:CoM bone index), if it names one.
fn sight_bone(m: &Manifest) -> Option<i32> {
    let b = m.consts.get("sight_bone");
    (b.is_finite() && (0.0..256.0).contains(&b) && b.fract() == 0.0).then_some(b as i32)
}

/// `player+0x3A8`: Re:CoM's index of Sora's `chest1` bone (see [`sight`]).
const SORA_CHEST: usize = 0x3A8;

/// Whether Sora's target (Re:CoM's attack target `player+0x84C`, a handle; KH1's party 0 `+0x74`)
/// is this enemy.
unsafe fn sora_targets(en: usize) -> bool {
    let pl: usize = rd(super::va(base(), data::MAIN_PLAYER));
    if pl == 0 {
        return false;
    }
    let h: i32 = rd(pl + 0x84C);
    h > 0 && func::<HandleFn>(f::HANDLE_OBJ)(h as u32) == en
}

/// Whether Sora stands on the floor (his mover's contact `+0x120 & 5`, as [`floor_note`] reads it):
/// KH1's target movement state `*(target + 0x70)` = 0 (M11, the Air Pirate). No Sora: on the floor.
unsafe fn sora_grounded() -> bool {
    let pl: usize = rd(super::va(base(), data::MAIN_PLAYER));
    if pl == 0 {
        return true;
    }
    let m: usize = rd(pl + e::MOVER);
    m == 0 || rd::<u32>(m + mv::CONTACT) & 5 != 0
}

/// The battle floor for a flying body (`sys 89`): Sora's height while he stands on the floor
/// (his mover's contact `+0x120 & 5`), kept from the last time (Assumption: flat battle floors;
/// Re:CoM's fliers keep no floor height: their gravity is off, `+0x260 & 0x100`).
unsafe fn floor_note(rt: &mut Runtime) {
    let pl: usize = rd(super::va(base(), data::MAIN_PLAYER));
    if pl == 0 {
        return;
    }
    let m: usize = rd(pl + e::MOVER);
    if m != 0 && rd::<u32>(m + mv::CONTACT) & 5 != 0 {
        let y = crate::units::to_kh1(rd_vec(m + mv::POS)).y;
        if y.is_finite() {
            rt.floor = Some(y);
        }
    }
}

/// The body touched the floor at the end of a KH1 arc (`0x1402B7260`'s landing).
unsafe fn landed(en: usize, rt: &mut Runtime) {
    match rt.reaction {
        Some(Phase::Air { base, behind }) => {
            let (r, then, lying) = ke::landing(base, behind);
            rt.queued = None;
            play(en, rt, r, 0.0);
            rt.queued = Some((then, 0.0));
            rt.reaction = Some(match (lying, base) {
                (true, Some(b)) => {
                    rt.lie = Some(then);
                    Phase::Down { base: b, behind, left: rt.down }
                }
                _ => Phase::Motion,
            });
        }
        // Movement state 3 (a timed jump, the Powerwild's hops) lands straight to state 0
        // (`0x1402AFAD0`): no landing motion; the queue the jump set plays on.
        _ if rt.ai.body().landed_timed => {}
        _ => {
            // A plain landing (a leap, a held attack's end): KH1's request 7 then the queue
            // (`0x1402B7260`).
            if rt.action.is_none() && rt.reaction.is_none() {
                let q = rt.queued.take();
                play(en, rt, ke::react::req::LAND, 0.0);
                rt.queued = q.or(Some((ke::react::req::IDLE, 0.0)));
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The states.
// ---------------------------------------------------------------------------------------------

/// State 0 begin: Re:CoM's alternate appearance runs as Re:CoM's; else the class's appearance:
/// the Shadow's begin without its motion (bodies off until the rise) and KH1's rise; a base class's
/// Re:CoM begin as it is, then KH1's appearance.
pub unsafe extern "win64" fn appear_begin(en: usize) {
    let alt = rd::<u64>(en + e::FLAGS) & e::APPEAR_ALT != 0;
    let world = super::stats::recom_world();
    let done = with_rt(en, |rt| {
        rt.recom_appear = alt;
        rt.appear_sub = 0;
        // The KH1 world its script's world rules read (decision 21's mapping).
        let w = crate::kh1_enemy::kh1_world(world);
        rt.ai.set_world(w);
        let k = rt.class;
        line(k, || format!("e{}: KH1 world {w:?} (Re:CoM world {world})", rd::<u32>(en + e::ID)));
        let m = mover(en);
        if m != 0 {
            rt.last_pos = crate::units::to_kh1(rd_vec(m + mv::POS));
            // The facing the spawn gave it (set after the ctor).
            let y = crate::units::yaw_to_kh1(rd(m + mv::YAW));
            let mut b = rt.ai.body();
            b.yaw = y;
            b.yaw_target = y;
            drop(b);
        }
        if alt {
            rt.ai.skip_rise();
        }
        CLASSES[rt.class].appear
    });
    match done {
        None => return call_original(en, ce::begin_slot(state::APPEAR)),
        Some(_) if alt => return call_original(en, ce::begin_slot(state::APPEAR)),
        Some(Appear::Base) => {
            // `0x14017B590` as it is (its writes, the battle message 0x96, its appear motion if the
            // class has one); KH1's appearance replaces the motion from the next frame.
            call_original(en, ce::begin_slot(state::APPEAR));
            let k = ours(en).unwrap_or(1);
            line(k, || format!("e{}: appears (KH1's appearance)", rd::<u32>(en + e::ID)));
        }
        Some(Appear::Shadow) => {
            // `0x140192850`'s writes but its motion: `+0x154 &= ~1`, both body sets off.
            let f154: u32 = rd(en + 0x154);
            wr(en + 0x154, f154 & !1);
            bodies(en, false);
            line(0, || format!("e{}: appears (KH1's rise)", rd::<u32>(en + e::ID)));
        }
    }
}

/// State 0 step: KH1's appearance; when it is done, the Shadow's battle-start handshake
/// (`0x140192B00`) or, for a base class, Re:CoM's default state.
pub unsafe extern "win64" fn appear_step(en: usize) {
    let r = with_rt(en, |rt| {
        if rt.recom_appear {
            return None;
        }
        let kind = CLASSES[rt.class].appear;
        match rt.appear_sub {
            0 => {
                tick(en, rt, true);
                if !rt.ai.risen() {
                    return Some(false);
                }
                // An attack the behaviour asked for in this same frame (its first after the rise)
                // already set Re:CoM's next state: the default state would overwrite it and the
                // attack state would never begin (owner 2026-11-02: a Shadow froze after its first
                // claw, log `e213`). It leaves state 0 for the attack state instead.
                let pending = rt.action.is_some_and(|a| a.attack.is_some()) && !rt.action_playing;
                if kind == Appear::Shadow && func::<IntFn>(f::INTRO_ROOM)() != 0 && func::<IntFn>(f::INTRO_FLAG)() != 0 {
                    func::<IntFn>(f::INTRO_COUNT)();
                    rt.appear_sub = 1;
                } else if !pending {
                    rt.handing_back = true;
                    return Some(true);
                }
            }
            _ => {
                if func::<IntFn>(f::INTRO_FLAG)() == 0 {
                    rt.handing_back = true;
                    return Some(true);
                }
            }
        }
        Some(false)
    });
    match r.flatten() {
        // Re:CoM's default state outside our lock (its state 1 begin is ours: `think_begin`).
        Some(true) => func::<StateFn>(f::DEFAULT_STATE)(en),
        Some(false) => {}
        None => call_original(en, ce::step_slot(state::APPEAR)),
    }
}

/// State 1 begin: no Re:CoM idle / walk pick; KH1's motion carries on.
pub unsafe extern "win64" fn think_begin(en: usize) {
    if with_rt(en, |rt| {
        if !std::mem::take(&mut rt.handing_back) {
            take_over(en, rt);
        }
        rt.recom_state = false;
        rt.quiet = 0.0;
        if rt.current.is_none() {
            play(en, rt, ke::react::req::IDLE, 8.0);
        }
    })
    .is_none()
        && ours(en).is_none()
    {
        call_original(en, ce::begin_slot(state::THINK));
    }
}

/// A state Re:CoM owns ran (stun, Stop, freeze, Gravity's flatten, a Re:CoM flinch, the
/// first-strike stun) and handed back to state 1 (owner 2026-11-02, 0.68.2: "once they got out of
/// the stun they are doing the same issue of just standing still": the kick the stun cut was still
/// the running action, so the AI was never polled again). As in KH1, where a status ends the
/// current action: the action, the reaction, the queued motion and the clash are dropped, the
/// motion Re:CoM played is forgotten (the idle plays), and the AI goes on from its wait.
unsafe fn take_over(en: usize, rt: &mut Runtime) {
    let had = (rt.action.map(|a| a.request), rt.reaction.is_some(), rt.queued.map(|q| q.0));
    if had != (None, false, None) {
        let st = crate::guard::state_index(en, rd(en + e::STATE));
        log(&format!(
            "{}: e{}: back in state 1 after a Re:CoM state ({st:?}): action {:?}, reaction {}, queued {:?} dropped; the AI goes on",
            CLASSES[rt.class].name,
            rd::<u32>(en + e::ID),
            had.0,
            had.1,
            had.2
        ));
    }
    end_action(rt);
    rt.reaction = None;
    rt.lie = None;
    rt.clash = None;
    rt.queued = None;
    rt.recom_flinch = false;
    rt.current = None;
    // A Re:CoM state ran (Stop, stun, freeze…): the behaviour drops what waits on it (M11, the Yellow Opera's marked
    // Thunder; owner 2026-11-03: Re:CoM's Stop keeps cancelling a caster's spell).
    rt.ai.on_recom_state();
    // Its marked Thunder's warning goes with it (`kh1_thrown::warn`; nothing when it has none).
    super::kh1_thrown::end_warnings(en, None);
    // KH1's mode-3 guard row (the Large Body's front) isn't tied to an action: it stays (`0x1402B47A0` tests it
    // whatever plays; a status doesn't remove it: Checked (decomp), its owner's `+0x88 & 1` skips only non-guard rows).
    if !rt.guard.is_some_and(|g| g.request == ke::guard::ALWAYS) {
        guard_end(en, rt);
    }
    rt.clash_recoil = None;
}

/// State 1 step: KH1's frame with the behaviour.
pub unsafe extern "win64" fn think_step(en: usize) {
    // One of ours never runs Re:CoM's state 1 (its think is replaced; `+0xCA4` stays 0 and its
    // walk divides by it, 0.73.0's crash): a failed lookup skips the frame (logged by `miss`).
    if with_rt(en, |rt| tick(en, rt, true)).is_none() && ours(en).is_none() {
        call_original(en, ce::step_slot(state::THINK));
    }
    grab_test_now(en);
}

/// The enemy whose behaviour started its grab at once this step (`kh1_core::enemy::host::starts_at_once`), or 0.
static GRAB_NOW: AtomicUsize = AtomicUsize::new(0);

/// After a state step of `en` (outside the runtime lock: the hit's callbacks take it): when its behaviour started its
/// grab at once (M11, the Neoshadow: KH1 asks for Sora's held motion in the frame of its test), Re:CoM's attack
/// windows (`CAttackMotion` `+0x18`, `0x14014E830`, what the enemy's slot `+0x90` runs after the state machine) and its
/// attack check (`0x14014E6A0`, what its slot `+0x78` runs before it, then its "connected" bit `+0x260 & 0x40`) run now,
/// so the grab's record meets Sora in this update, and Sora, updated after the enemies, takes it in this frame (KH1's).
/// Re:CoM's own passes then find the record's window open (no new serial) and its hit in Sora's hit-once ring
/// (`0x14016F1C0`): no second hit. Checked (decomp + disasm). One atomic load per step otherwise.
unsafe fn grab_test_now(en: usize) {
    if GRAB_NOW.load(Ordering::Relaxed) != en || GRAB_NOW.compare_exchange(en, 0, Ordering::Relaxed, Ordering::Relaxed).is_err() {
        return;
    }
    let id: u32 = rd(en + e::ID);
    let name = ours(en).map_or("kh1 enemy", |k| CLASSES[k].name);
    let b = base();
    let set: usize = rd(en + e::ATTACKS);
    let w = &crate::sites::ATTACK_WINDOWS_PTRS[0];
    if set == 0 || rd::<usize>(set) != super::va(b, crate::guard::attack_motion::VTABLE) || rd::<usize>(rd::<usize>(set) + 0x18) != b + w.target as usize {
        log(&format!("{name}: e{id}: the grab's attack set is not Re:CoM's CAttackMotion with 0x14014E830 at +0x18: its record is tested by Re:CoM's own passes (the hold a few frames late)"));
        return;
    }
    func::<StateFn>(w.target)(set);
    let r = func::<CheckFn>(f::ATTACK_CHECK)(set);
    if r != 0 {
        wr(en + e::FLAGS, rd::<u64>(en + e::FLAGS) | 0x40);
    }
    log(&format!("{name}: e{id}: the grab's record tested at once (Re:CoM's windows and attack check, result {r}): {}", if r != 0 { "it met Sora: his update takes it this frame (KH1's)" } else { "it met nothing" }));
}

/// The think (`+0x3D0`): nothing; KH1's behaviour picks the attacks.
pub unsafe extern "win64" fn think_none(_en: usize) {}

/// An attack state's begin: the attack's KH1 motion (its records attack through the `.ASD`).
pub unsafe extern "win64" fn attack_begin(en: usize) {
    let k = crate::guard::state_index(en, rd(en + e::NEXT)).or_else(|| crate::guard::state_index(en, rd(en + e::STATE)));
    let done = with_rt(en, |rt| {
        let Some(a) = rt.action.filter(|a| a.attack.is_some()) else { return false };
        if !ke::host::begin_plays(running(a, rt.action_playing)) {
            // Started at once (KH1's grab): it plays on.
            return true;
        }
        play_at(en, rt, a.request, a.blend, a.start);
        // An action queues its `then`; KH1's plain request (a steered attack) replaces the queue.
        rt.queued = (!a.steered).then_some((a.then, a.blend));
        rt.action_playing = true;
        rt.action_age = 0.0;
        true
    });
    if done != Some(true) {
        // Not started by our behaviour (never expected): Re:CoM's begin.
        let s = k.map_or(2, |k| k as i32);
        call_original(en, ce::begin_slot(s));
    }
}

/// An attack state's step: KH1's frame without the behaviour; at the motion's end, Re:CoM's attack
/// end (`0x14017E540`: bit 0 off, the default state). A steered attack (M11 fliers) runs the
/// behaviour and ends when the behaviour plays another motion (KH1's rows end with their request,
/// `0x1402B47A0`) or starts something else, or when a one-shot clip has played; it doesn't end at a
/// looping clip's wrap (the script decides how long it loops) and the watchdog's end margin doesn't
/// apply to it (the AI is not waiting on it).
pub unsafe extern "win64" fn attack_step(en: usize) {
    let r = with_rt(en, |rt| {
        let steered = rt.action.is_some_and(|a| a.steered && rt.action_playing);
        let mut end = false;
        if steered {
            let looped = rt.current.and_then(|q| manifest(rt.class).clip(q)).is_some_and(|c| c.looped);
            if !looped && ended(en, rt) {
                end = true;
            }
        } else {
            end = rt.action.is_some() && ended(en, rt);
            if !end {
                // The safety net: an attack that never ends, or none at all in an attack state (its
                // begin was Re:CoM's): leave the attack state as at the motion's end.
                match stuck(en, rt) {
                    Some((why, detail)) => {
                        release_line(en, rt, why, &detail);
                        end = true;
                    }
                    None if rt.action.is_none() => {
                        let st = crate::guard::state_index(en, rd(en + e::STATE)).map_or("?".to_string(), |s| s.to_string());
                        log(&ce::watchdog::line(CLASSES[rt.class].name, rd(en + e::ID), 0, ce::watchdog::Stuck::NeverStarted, &format!("Re:CoM attack state {st} without a KH1 action")));
                        end = true;
                    }
                    None => {}
                }
            }
        }
        if end {
            end_action(rt);
            rt.handing_back = true;
        }
        let poll = steered && !end;
        let req = rt.action.map(|a| a.request);
        tick(en, rt, poll);
        if poll {
            match ke::host::after_poll(req, rt.action.map(|a| running(a, rt.action_playing)), rt.current) {
                ke::host::AfterPoll::Keep => {}
                // Another motion replaced it.
                ke::host::AfterPoll::Replaced => {
                    end_action(rt);
                    end = true;
                }
                // A new attack the behaviour started (`tick` asks for its state again), a plain
                // action (a jump start…: it carries on in state 1), or none left.
                ke::host::AfterPoll::NewAttack | ke::host::AfterPoll::Plain | ke::host::AfterPoll::Gone => end = true,
            }
            // Our own leave to state 1 (merge of Stage 127's hand-back with the fliers' steered
            // attacks): not a Re:CoM takeover, so the pending action / queue are kept.
            if end {
                rt.handing_back = true;
            }
        }
        let again = rt.action.filter(|a| a.attack.is_some() && !rt.action_playing).and_then(|a| CLASSES[rt.class].attack_states.get(a.attack.unwrap_or(0) as usize).copied());
        (end, again)
    });
    match r {
        Some((true, again)) => {
            func::<EndFn>(f::ATTACK_END)(en, 0, 0);
            if let Some(k) = again {
                enter_state(en, k);
            }
        }
        Some((false, _)) => {}
        None => {
            let s = crate::guard::state_index(en, rd(en + e::STATE)).map_or(2, |k| k as i32);
            call_original(en, ce::step_slot(s));
        }
    }
    grab_test_now(en);
}

/// The hit Re:CoM's handler took (its hit info): KH1's kind (a KH1 swing's from its row, else from
/// Re:CoM's kind), the push direction, the hurt part.
unsafe fn hit_of(en: usize, k: usize) -> Option<Hit> {
    let info: usize = rd(en + e::HIT_INFO);
    if info == 0 {
        return None;
    }
    let atk: usize = rd(info + hi::ATTACK);
    if mover(en) == 0 {
        return None;
    }
    let me = crate::units::to_kh1(rd_vec(mover(en) + mv::POS));
    let swing = take_swing(en);
    let kind = swing.or_else(|| (atk != 0).then(|| crate::shadow::kh1_kind_of_recom(rd(atk + hi::KIND))))?;
    // The attacker: its handle's actor (`0x14017C530`'s rule), else Sora.
    let owner = if atk != 0 { func::<HandleFn>(f::HANDLE_OBJ)(rd(atk + hi::OWNER)) } else { 0 };
    let pl: usize = rd(super::va(base(), data::MAIN_PLAYER));
    let dir = if owner != 0 && owner == pl {
        let pm: usize = rd(pl + e::MOVER);
        let sp = crate::units::to_kh1(rd_vec(pm + mv::POS));
        let sy = crate::units::yaw_to_kh1(rd(pm + mv::YAW));
        crate::shadow::swing_dir(sp, sy, me, std::f32::consts::FRAC_PI_4)
    } else {
        let from = if owner != 0 { attacker_point(owner) } else { sora_pos().unwrap_or(me) };
        let d = (me - from).flat();
        if d.horizontal_length() > 1e-3 {
            d.normalized()
        } else {
            Vec3::from_yaw(with_rt(en, |rt| rt.ai.body().yaw).unwrap_or(0.0) + std::f32::consts::PI)
        }
    };
    // The part: the contact point against the hurt parts' bones.
    let n: i32 = rd(info + hi::COUNT);
    let parts: usize = rd(info + hi::PARTS);
    let bones = &manifest(k).parts;
    let part = if n > 0 && parts != 0 && !bones.is_empty() {
        let contact = rd_vec(parts + hi::CONTACT);
        let at: Vec<Vec3> = bones.iter().map(|&b| rd_vec(func::<BoneFn>(f::BONE)(en, b) + 0xD0)).collect();
        ke::react::nearest_part(contact, &at)
    } else {
        0
    };
    Some(Hit { kind, dir, part })
}

/// A non-player attacker's place, KH1 units: a Re:CoM shot (`CBullet` / `CHomingBullet`: no model of its own) its mover's
/// position (M11: a turned-back or batted fireball, the Defender's shield test); any other actor its bone 0 (as before).
unsafe fn attacker_point(owner: usize) -> Vec3 {
    let (vt, b) = (rd::<usize>(owner), base());
    let m: usize = rd(owner + e::MOVER);
    if (vt == super::va(b, crate::guard::bullet::VTABLE) || vt == super::va(b, crate::guard::bullet::HOMING_VTABLE)) && m != 0 {
        return crate::units::to_kh1(rd_vec(m + mv::POS));
    }
    crate::units::to_kh1(rd_vec(func::<BoneFn>(f::BONE)(owner, 0) + 0xD0))
}

/// KH1 swings noted by the enemy-hit wrapper (`note_swing`), by enemy.
static SWING: std::sync::Mutex<Vec<(usize, u8)>> = std::sync::Mutex::new(Vec::new());

/// `super::attack`: a KH1 swing hit this enemy with KH1's reaction kind `kind`.
pub(super) unsafe fn note_swing(en: usize, kind: u8) {
    if !any_installed() || ours(en).is_none() {
        return;
    }
    if let Ok(mut g) = SWING.lock() {
        g.retain(|x| x.0 != en);
        if g.len() < 64 {
            g.push((en, kind));
        }
    }
}

/// Re:CoM's super armour around one call of its hit handler (`kh1_core::enemy::host::RECOM_ARMOUR`), for a
/// KH1 enemy whose script takes no reaction for a damaging hit now (`Behaviour::armoured`: KH1's
/// `sys 42`; M11, the Pirate): the enemy, its flags and its next state before the call.
pub(super) struct Armour {
    en: usize,
    flags: u64,
    next: usize,
    /// KH1's kind of the hit the behaviour was asked about (the fallback when `hit_of` has none).
    kind: Option<u8>,
}

/// Before Re:CoM's hit handler (`super::attack::enemy_hit`, the wrapper every call of `0x14017F7E0` goes
/// through): bits 20 / 21 on for the call while the behaviour's hit handler would take no reaction for
/// the damaging hit about to be handled (`incoming`: None when none waits; else its KH1 kind, if known).
/// Not while Sora's Guard blocked the enemy this frame (`super::guard::block_pending`): the block's
/// reaction runs inside this call (`after_enemy_hit`) and Re:CoM's picker would only play the damage voice
/// under the armour (a clash KH1 plays: kinds 6 / 7 come before every `hp[0xC]` zeroing and `sys 42`).
/// Per call (every frame, every enemy): one atomic load while no KH1 class is installed, else one tail
/// check, a scan of the hit bodies and, with a hit waiting, one question.
pub(super) unsafe fn armour_on(en: usize, incoming: impl FnOnce() -> Option<(Option<u8>, u32)>) -> Option<Armour> {
    if !any_installed() {
        return None;
    }
    let k = ours(en)?;
    let (kind, owner) = incoming()?;
    // M11, the Defender: KH1's push direction and whether the hit comes from one of its class's own rows (a fireball of a
    // same-class enemy sent back), which its handler reads before it decides (`sys 100`, `sys 188`).
    let from = incoming_from(en, k, owner);
    if super::guard::block_pending(en) {
        return None;
    }
    if !with_rt(en, |rt| {
        if let Some(k) = kind {
            rt.ai.set_hit_kind(k);
        }
        if let Some((dir, own)) = from {
            rt.ai.set_hit_dir(dir);
            rt.ai.set_hit_own_row(own);
        }
        rt.ai.set_playing(rt.current);
        !rt.dead && rt.ai.armoured(rt.current, rt.launch_base, kind)
    })? {
        return None;
    }
    let flags: u64 = rd(en + e::FLAGS);
    wr(en + e::FLAGS, ke::host::armour_flags(flags));
    Some(Armour { en, flags, next: rd(en + e::NEXT), kind })
}

/// The waiting hit's KH1 push direction (record `+0x10`: target − attacker, flat, unit; Sora's swing clamped within 45° of
/// his facing, `hit_of`'s rule) and whether it comes from one of the class's own rows (`super::kh1_thrown::own_shot`: a
/// fireball a same-class enemy threw, sent back by Sora's swing or Guard), for a class that reads it (one with
/// `own_row_first` in its manifest: the Defender); None for the others (nothing read).
unsafe fn incoming_from(en: usize, k: usize, owner: u32) -> Option<(Vec3, bool)> {
    if !manifest(k).consts.get("own_row_first").is_finite() || mover(en) == 0 || owner == 0 {
        return None;
    }
    let me = crate::units::to_kh1(rd_vec(mover(en) + mv::POS));
    let own = own_row(k, owner);
    let o = func::<HandleFn>(f::HANDLE_OBJ)(owner);
    if o == 0 {
        return Some((Vec3::ZERO, own));
    }
    let pl: usize = rd(super::va(base(), data::MAIN_PLAYER));
    let om: usize = rd(o + e::MOVER);
    let dir = if o == pl && om != 0 {
        let sp = crate::units::to_kh1(rd_vec(om + mv::POS));
        let sy = crate::units::yaw_to_kh1(rd(om + mv::YAW));
        crate::shadow::swing_dir(sp, sy, me, std::f32::consts::FRAC_PI_4)
    } else if om != 0 {
        let d = (me - attacker_point(o)).flat();
        if d.horizontal_length() > 1e-3 {
            d.normalized()
        } else {
            Vec3::ZERO
        }
    } else {
        Vec3::ZERO
    };
    Some((dir, own))
}

/// Whether attacker `owner` is a fireball an enemy of class `k` threw, now on the enemies' side (`super::kh1_thrown::own_shot`).
unsafe fn own_row(k: usize, owner: u32) -> bool {
    owner != 0
        && super::kh1_thrown::own_shot(owner).is_some_and(|shooter| {
            let s = func::<HandleFn>(f::HANDLE_OBJ)(shooter);
            s != 0 && ours(s) == Some(k)
        })
}

/// The `@ENP` record (`+0x308`) and its stun gate `+0x28` (Re:CoM's hit handler and picker read it for kind 7). Checked
/// (decomp `0x14017F7E0`, `0x14017D260`; files: `EN0003.EPD`'s `@ENP` `+0x28` = 1, the Red Nocturne can be stunned).
const ENP: usize = 0x308;
const STUN_GATE: usize = 0x28;

/// Re:CoM's stun asked for a KH1 enemy's own row ([`own_stun_on`]): the enemy.
pub(super) struct OwnStun {
    en: usize,
}

/// Before Re:CoM's hit handler (`super::attack::enemy_hit`): a damaging hit waiting on one of our enemies whose behaviour
/// takes Re:CoM's stun for its own rows (`Behaviour::own_row_stun`: the Red Nocturne hit by its own fire sent back, KH1's
/// `sys 188` then `sys 179(1800)`, owner 2026-11-03) gets Re:CoM's stun kind 7 on the hurt body Re:CoM's gather takes
/// (`+0x80`), so Re:CoM's handler applies the damage as a plain hit and its picker `0x14017D260` sends it to its own stun
/// state (`+0x900`), as Stun Impact's hits do (`kh1_core::enemy::host::RECOM_STUN_KIND`). Only while Re:CoM's gate allows
/// the stun. Per call: one atomic load while no class is installed, then the class's answer only with a hit waiting.
pub(super) unsafe fn own_stun_on(en: usize, body: impl FnOnce() -> Option<usize>) -> Option<OwnStun> {
    if !any_installed() {
        return None;
    }
    let k = ours(en)?;
    if !with_rt(en, |rt| rt.ai.own_row_stun())? {
        return None;
    }
    let at = body()?;
    let owner: u32 = rd(at + hi::OWNER);
    let enp: usize = rd(en + ENP);
    let gate: i32 = if enp != 0 { rd(enp + STUN_GATE) } else { 0 };
    let alive = rd::<i32>(en + e::HP) > 0 && with_rt(en, |rt| !rt.dead).unwrap_or(false);
    let kind = ke::host::own_row_stun_kind(true, own_row(k, owner), alive, gate)?;
    let was: u32 = rd(at + hi::KIND);
    wr(at + hi::KIND, kind);
    let id: u32 = rd(en + e::ID);
    line(k, || format!("e{id}: hit by its own fire (shot {owner}): Re:CoM's stun kind {kind} for its handler (was kind {was}; gate {gate})"));
    Some(OwnStun { en })
}

/// After the call: when Re:CoM took it into its stun state, the behaviour's hit handler hears the hit (KH1's EXP bonus,
/// its hit state for when the stun hands back: `take_over` drops what played, the AI goes on from the hit state). Any
/// other outcome (the defeat, a guard) is Re:CoM's and our states' as before.
pub(super) unsafe fn own_stun_off(s: Option<OwnStun>) {
    let Some(OwnStun { en }) = s else { return };
    let next = crate::guard::state_index(en, rd(en + e::NEXT));
    if !ke::host::stun_took(rd(en + e::HP), next) {
        let id: u32 = rd(en + e::ID);
        if let Some(k) = ours(en) {
            line(k, || format!("e{id}: Re:CoM's handler did not stun it (next state {next:?}): as Re:CoM took it"));
        }
        return;
    }
    with_rt(en, |rt| {
        rt.hits = rt.hits.saturating_add(1);
        rt.ai.set_hit_count(rt.hits);
        rt.ai.set_hit_own_row(true);
        rt.ai.set_playing(rt.current);
        rt.ai.on_hit(HitNews::Damage);
        run_now(en, rt);
        let (id, k) = (rd::<u32>(en + e::ID), rt.class);
        line(k, || format!("e{id}: Re:CoM's stun (state {}) for its own fire; KH1's EXP bonus and its hit state after it", ke::host::RECOM_STUN_STATE));
    });
}

/// After the call: Re:CoM's two bits back as they were; a damaging hit it took without a reaction (a
/// living enemy, no state asked for) goes to the behaviour's hit handler (`Behaviour::on_armoured_hit`:
/// KH1's push slide) and KH1's hit counter. A Re:CoM state (Stop, stun, freeze…) or the defeat is left
/// to Re:CoM, as before.
pub(super) unsafe fn armour_off(a: Option<Armour>, result: u64) {
    let Some(a) = a else { return };
    let en = a.en;
    let now: u64 = rd(en + e::FLAGS);
    wr(en + e::FLAGS, ke::host::restore_armour(now, a.flags));
    if !ke::host::armour_took(result, rd(en + e::HP), a.next, rd(en + e::NEXT)) {
        return;
    }
    let Some(k) = ours(en) else { return };
    let hit = hit_of(en, k);
    let cut = with_rt(en, |rt| {
        rt.hits = rt.hits.saturating_add(1);
        rt.ai.set_hit_count(rt.hits);
        if CLASSES[rt.class].reads_hp {
            rt.ai.set_hp_ratio(hp_ratio(en));
        }
        match (hit, a.kind) {
            (Some(h), _) => {
                rt.ai.set_hit_kind(h.kind);
                rt.ai.on_armoured_hit(h.dir);
            }
            // No hit info after the call (not expected): the kind it was armoured for, no push.
            (None, Some(kind)) => {
                rt.ai.set_hit_kind(kind);
                rt.ai.on_armoured_hit(Vec3::ZERO);
            }
            (None, None) => {}
        }
        // What KH1's handler does in itself (the Large Body's front row made again).
        run_now(en, rt);
        let (id, cur) = (rd::<u32>(en + e::ID), rt.current);
        line(rt.class, || format!("e{id}: hit kind {:?} without a reaction while {cur:?} plays (KH1's no-reaction hit on Re:CoM's super armour)", hit.map(|h| h.kind).or(a.kind)));
        if !rt.ai.ends_action() {
            return false;
        }
        // KH1's handler cleared actor flag 4 (the Large Body's rage): the action ends now and the AI runs next frame.
        // In one of our attack states, Re:CoM's attack end (our own leave to state 1); a plain action in state 1
        // just stops being waited on; a reaction (states 11 / 12) plays on (Assumption: KH1's script would run
        // during it, the rage starts after it here).
        let st = crate::guard::state_index(en, rd(en + e::STATE)).map(|s| s as i32);
        let in_attack = st.is_some_and(|s| CLASSES[rt.class].attack_states.contains(&s));
        if rt.reaction.is_none() {
            end_action(rt);
            rt.queued = None;
        }
        if in_attack {
            rt.handing_back = true;
        }
        line(rt.class, || format!("e{id}: its hit handler ends the action ({cur:?}, Re:CoM state {st:?}): KH1's flag 4 cleared"));
        in_attack
    });
    if cut == Some(true) {
        func::<EndFn>(f::ATTACK_END)(en, 0, 0);
    }
}

fn take_swing(en: usize) -> Option<u8> {
    let mut g = SWING.lock().ok()?;
    let i = g.iter().position(|x| x.0 == en)?;
    Some(g.swap_remove(i).1)
}

/// Starts KH1's reaction for the hit (or the clash) on this enemy; false: nothing to play.
unsafe fn start_reaction(en: usize, rt: &mut Runtime, hit: Option<Hit>) -> bool {
    let m = manifest(rt.class);
    let c = &m.consts;
    let y = crate::units::to_kh1(rd_vec(mover(en) + mv::POS)).y;
    let airborne = rt.ai.body().airborne();
    end_action(rt);
    if let Some((recoil, kind)) = rt.clash_recoil.take() {
        // The shared enemy-guard step: its guard row met Sora's swing (`0x1402B4BC0`): KH1's clash
        // (`0x1402B8E10` cases 6 / 7 → `0x1402B9140`: the row's `+0x40`).
        rt.clash = None;
        let s = ke::react::clash(recoil, airborne, &mut rt.ai.body(), y, c);
        rt.queued = None;
        play(en, rt, s.request, 0.0);
        rt.queued = Some((s.then, c.get("attack_blend")));
        rt.reaction = Some(if airborne { Phase::Air { base: None, behind: false } } else { Phase::Motion });
        line(rt.class, || format!("e{}: its guard row clashed (kind {kind}): recoil {}", rd::<u32>(en + e::ID), s.request));
        return true;
    }
    if let Some(a) = rt.clash.take() {
        let names = CLASSES[rt.class].attack_names;
        let aname = names.get(a as usize).copied().unwrap_or("?");
        let Some(recoil) = m.recoil_of(a) else {
            // Not expected (`guard_block` hands such a block to Re:CoM's flinch): the idle.
            rt.queued = None;
            play(en, rt, ke::react::req::IDLE, c.get("attack_blend"));
            rt.reaction = Some(Phase::Motion);
            line(rt.class, || format!("e{}: clash on its {aname}: no recoil motion ({:?}); idle", rd::<u32>(en + e::ID), m.recoil.get(a as usize)));
            return true;
        };
        let s = ke::react::clash(recoil, airborne, &mut rt.ai.body(), y, c);
        rt.queued = None;
        play(en, rt, s.request, 0.0);
        rt.queued = Some((s.then, c.get("attack_blend")));
        rt.reaction = Some(if airborne { Phase::Air { base: None, behind: false } } else { Phase::Motion });
        line(rt.class, || format!("e{}: clash recoil {} ({aname})", rd::<u32>(en + e::ID), s.request));
        return true;
    }
    let Some(hit) = hit else { return false };
    let lying = matches!(rt.reaction, Some(Phase::Down { .. })) && rt.current == rt.lie;
    let current = rt.current;
    // The behaviour's random stream (KH1's one `rand`; the launch's down time draws from it).
    let mut rng = rt.ai.rng();
    let yaw = rt.ai.body().yaw;
    let mut start = {
        let mut body = rt.ai.body();
        if body.flying {
            // Movement state 4 (M11 fliers): KH1's ground forms, kind 5 as kind 4.
            ke::react_flying(hit, yaw, current, m, c, &mut rng, &mut body, y)
        } else {
            ke::react(hit, yaw, current, airborne, lying, m, c, &mut rng, &mut body, y)
        }
    };
    // A launch the script's hit handler asked for (`sys 38`, the Powerwild) replaces the reaction.
    // KH1 runs the handler first (`0x1402B7B90` → `0x1402BCD60`, then `0x1402B8E10`'s switch) and
    // `0x1402B79F0` marks the record (`*hit |= 2`) so KH1's own reaction is skipped, its random
    // draw too (Checked (decomp), M11 Bandit pass); here KH1's reaction is chosen first and
    // replaced, so its draw stays drawn (a known difference, `docs/recom-m11-enemies.md`).
    if let Some(l) = rt.ai.scripted_launch() {
        let mut body = rt.ai.body();
        if let Some(s) = ke::react::scripted_launch(hit, l, airborne, yaw, m, c, &mut body, y) {
            start = Some(s);
        }
    }
    rt.ai.set_rng(rng);
    let id: u32 = rd(en + e::ID);
    match start {
        None => {
            line(rt.class, || format!("e{id}: hit kind {} part {} while lying: push only", hit.kind, hit.part));
            true
        }
        Some(s) => {
            rt.down = s.down;
            if let Some(b) = s.base {
                rt.launch_base = b;
            }
            rt.queued = None;
            play(en, rt, s.request, 0.0);
            rt.queued = Some((s.then, c.get("attack_blend")));
            let behind = s.base.is_some_and(|b| s.request >= b + 5);
            rt.reaction = Some(if rt.ai.body().airborne() { Phase::Air { base: s.base, behind } } else { Phase::Motion });
            line(rt.class, || format!("e{id}: reaction kind {} part {} dir ({:.2},{:.2}) -> {} then {}{}", hit.kind, hit.part, hit.dir.x, hit.dir.z, s.request, s.then, if s.base.is_some() { format!(" down {:.1}", s.down) } else { String::new() }));
            true
        }
    }
}

/// Re:CoM's HP / max HP (`+0x274`, `0x14017F5A0`), for scripts that read their HP.
unsafe fn hp_ratio(en: usize) -> f32 {
    let hp: i32 = rd(en + e::HP);
    let max = func::<MaxHpFn>(f::MAX_HP)(en);
    if max > 0 {
        hp as f32 / max as f32
    } else {
        1.0
    }
}

/// States 11 / 12 begin: KH1's reaction.
pub unsafe extern "win64" fn react_begin(en: usize) {
    // Re:CoM's own flinch for a blocked attack without a KH1 recoil motion (owner 2026-11-02): as
    // before M10, Re:CoM's state 11 begin and step (`0x14017C530` / `0x140184100`); the KH1 frame
    // waits (no velocity of ours, no KH1 motion) until the next state 1.
    let recom = with_rt(en, |rt| {
        rt.recom_state = false;
        if !std::mem::take(&mut rt.recom_flinch) {
            return false;
        }
        end_action(rt);
        rt.clash = None;
        rt.queued = None;
        rt.current = None;
        rt.reaction = None;
        rt.recom_state = true;
        rt.hits = rt.hits.saturating_add(1);
        rt.ai.set_hit_count(rt.hits);
        rt.ai.set_playing(rt.current);
        rt.ai.on_hit(HitNews::Clash);
        run_now(en, rt);
        let k = rt.class;
        line(k, || format!("e{}: clash without a KH1 recoil motion: Re:CoM's flinch", rd::<u32>(en + e::ID)));
        true
    });
    if recom == Some(true) {
        return call_original(en, ce::begin_slot(state::FLINCH));
    }
    let hit = ours(en).and_then(|k| hit_of(en, k));
    let done = with_rt(en, |rt| {
        // KH1's hit handler: a damaging hit or a clash (the Shadow ignores a clash).
        if CLASSES[rt.class].reads_hp {
            rt.ai.set_hp_ratio(hp_ratio(en));
        }
        if let Some(h) = hit {
            rt.ai.set_hit_kind(h.kind);
            rt.ai.set_hit_dir(h.dir);
        }
        if let Some((_, k)) = rt.clash_recoil {
            rt.ai.set_hit_kind(k);
        }
        rt.ai.set_playing(rt.current);
        rt.hits = rt.hits.saturating_add(1);
        rt.ai.set_hit_count(rt.hits);
        let clash = rt.clash.is_some() || rt.clash_recoil.is_some();
        rt.ai.on_hit(if clash { HitNews::Clash } else { HitNews::Damage });
        // What KH1's handler does in itself (the Search Ghost's reward): now, not after the reaction.
        run_now(en, rt);
        // The handler ran before the reaction and may have rewritten the record's kind.
        let hit = if clash { hit } else { hit.map(|h| Hit { kind: rt.ai.reaction_kind(h.kind), ..h }) };
        if !clash && !rt.ai.keeps_reaction() {
            // KH1's hit handler zeroed the reaction kind (M11, the Darkball's armour): no reaction.
            // A reaction already running carries on in this state; otherwise back to state 1 (Re:CoM's
            // handler has already cut the attack it hit: the AI goes on).
            let id: u32 = rd(en + e::ID);
            let k = rt.class;
            let running = rt.reaction.is_some();
            line(k, || format!("e{id}: hit without a reaction (KH1's hit handler){}", if running { ": the reaction carries on" } else { "" }));
            if !running {
                rt.handing_back = true;
            }
            return running;
        }
        rt.reaction_age = 0.0;
        let started = start_reaction(en, rt, hit);
        if !started {
            rt.handing_back = true;
        }
        started
    });
    match done {
        Some(true) => {}
        Some(false) => func::<StateFn>(f::DEFAULT_STATE)(en),
        None => {
            let s = crate::guard::state_index(en, rd(en + e::NEXT)).map_or(state::FLINCH, |k| k as i32);
            call_original(en, ce::begin_slot(s));
        }
    }
}

/// States 11 / 12 step: KH1's frame without the behaviour; the reaction's phases.
pub unsafe extern "win64" fn react_step(en: usize) {
    if with_rt(en, |rt| rt.recom_state) == Some(true) {
        return call_original(en, ce::step_slot(state::FLINCH));
    }
    let r = with_rt(en, |rt| {
        let dt: i32 = rd(en + e::DT);
        let step = crate::units::step_from_dt(dt.clamp(0, 1000));
        let mut done = false;
        rt.reaction_age += step;
        if rt.reaction_age >= ce::watchdog::REACTION_LIMIT {
            // The safety net: a reaction that never ends (a launch that never lands, a motion that
            // never ends) is released as if it had ended.
            log(&format!("{}: e{}: reaction {:?} never ended ({:.0} frames, playing {:?}), released", CLASSES[rt.class].name, rd::<u32>(en + e::ID), rt.reaction, rt.reaction_age, rt.current));
            rt.reaction = None;
            rt.lie = None;
            rt.queued = None;
            play(en, rt, ke::react::req::IDLE, manifest(rt.class).consts.get("attack_blend"));
            rt.handing_back = true;
            return true;
        }
        let motion_ended = matches!(rt.reaction, Some(Phase::Motion)) && ended(en, rt);
        match rt.reaction {
            // The reaction's motion ended (this frame: `tick` plays the queued one): done.
            Some(Phase::Motion) => {
                if motion_ended {
                    done = true;
                }
            }
            Some(Phase::Down { base, behind, left }) => {
                if rt.current == rt.lie {
                    let l = left - step;
                    if l <= 0.0 {
                        rt.queued = None;
                        play(en, rt, ke::get_up(base, behind), 0.0);
                        rt.queued = Some((ke::react::req::IDLE, manifest(rt.class).consts.get("attack_blend")));
                        rt.reaction = Some(Phase::Motion);
                        rt.lie = None;
                    } else {
                        rt.reaction = Some(Phase::Down { base, behind, left: l });
                    }
                }
            }
            _ => {}
        }
        tick(en, rt, false);
        if done {
            rt.reaction = None;
            rt.handing_back = true;
        }
        done
    });
    match r {
        Some(true) => func::<StateFn>(f::DEFAULT_STATE)(en),
        Some(false) => {}
        None => {
            let s = crate::guard::state_index(en, rd(en + e::STATE)).map_or(state::FLINCH, |k| k as i32);
            call_original(en, ce::step_slot(s));
        }
    }
}

/// State 30 begin: Re:CoM's defeat (prizes, EXP, its removal timer), then the killing hit's KH1
/// reaction motion.
pub unsafe extern "win64" fn defeat_begin(en: usize) {
    let hit = ours(en).and_then(|k| hit_of(en, k));
    // KH1's hit handler runs for the killing hit too, before its damage and death (the Search Ghost's
    // reward, `0x28A6`): before Re:CoM's defeat prizes.
    with_rt(en, |rt| {
        if !rt.dead {
            rt.ai.on_defeat();
            run_now(en, rt);
        }
    });
    call_original(en, ce::begin_slot(state::DEFEAT));
    with_rt(en, |rt| {
        // KH1's hit handler rewrites the killing hit's kind too (it runs before the reaction).
        let hit = hit.map(|h| Hit { kind: rt.ai.reaction_kind(h.kind), ..h });
        rt.dead = true;
        rt.guard = None;
        rt.guard_under = None;
        rt.clash_recoil = None;
        guard_bodies(en, rt, None);
        rt.recom_flinch = false;
        rt.recom_state = false;
        if let Some(r) = rt.ai.death_motion() {
            // The script's own death motion (its handler zeroed KH1's reaction; M11, the Neoshadow's 72).
            end_action(rt);
            rt.queued = None;
            play(en, rt, r, 0.0);
            rt.reaction = Some(Phase::Motion);
        } else if !start_reaction(en, rt, hit) {
            play(en, rt, ke::react::req::FLINCH, 0.0);
            rt.reaction = Some(Phase::Motion);
        }
        let cur = rt.current;
        line(rt.class, || format!("e{}: defeated (Re:CoM's defeat, KH1's motion {:?})", rd::<u32>(en + e::ID), cur));
    });
}

/// State 30 step: KH1's body (the launch arc of the last hit), then Re:CoM's step (removal).
pub unsafe extern "win64" fn defeat_step(en: usize) {
    with_rt(en, |rt| {
        rt.dead_frames = rt.dead_frames.saturating_add(1);
        tick(en, rt, false)
    });
    call_original(en, ce::step_slot(state::DEFEAT));
}

// ---------------------------------------------------------------------------------------------
// A script's heal and reward on Re:CoM's own HP, EXP and prizes (M11, the Search Ghost's drain;
// `docs/recom-m11-enemies.md` § The Search Ghost).
// ---------------------------------------------------------------------------------------------

/// Re:CoM's enemy vtable slot for its prize point (`0x1401A4700` for the Search Ghost, `sites::SEARCHGHOST_PTRS`).
const PRIZE_POINT_SLOT: usize = 0x50;

/// Re:CoM's prize point of the enemy (its vtable `+0x50`, as Re:CoM's defeat `0x14017BBC0` and its Search
/// Ghost's drain heal `0x140192630` read it); None when not finite.
unsafe fn prize_point(en: usize, rt: &Runtime) -> Option<[f32; 4]> {
    let f = original(rt.class, PRIZE_POINT_SLOT);
    if f == 0 {
        return None;
    }
    let g: PointFn = std::mem::transmute_copy(&f);
    let mut p = [0.0f32, 0.0, 0.0, 1.0];
    g(en, &mut p);
    p[..3].iter().all(|x| x.is_finite()).then_some(p)
}

/// `Cmd::HealFull` (KH1's HP = max HP): Re:CoM's own enemy heal `0x140181CE0(enemy, max − HP, no effect,
/// 0xB, &point, 0)`, the call its Search Ghost's drain makes for its 1/8 (`0x140192630`), by what is
/// missing to Re:CoM's max HP (`0x14017F5A0`): HP, its clamp and the HP gauge (`+0x430`) are Re:CoM's.
unsafe fn heal_full(en: usize, rt: &mut Runtime) {
    let hp: i32 = rd(en + e::HP);
    let max = func::<MaxHpFn>(f::MAX_HP)(en);
    let id: u32 = rd(en + e::ID);
    if hp <= 0 || hp >= max {
        line(rt.class, || format!("e{id}: KH1's full heal: HP {hp} / {max}, nothing to heal"));
        return;
    }
    let p = prize_point(en, rt).unwrap_or([0.0, 0.0, 0.0, 1.0]);
    func::<HealFn>(f::HEAL)(en, max - hp, 0, 0xB, &p, 0);
    let after: i32 = rd(en + e::HP);
    line(rt.class, || format!("e{id}: KH1's full heal (Re:CoM's enemy heal 0x140181CE0, +{}): HP {hp} -> {after} / {max}", max - hp));
}

/// `Cmd::Reward` (KH1's `sys 194` and `sys 122` from a handler): KH1's EXP bonus on Re:CoM's EXP for the
/// enemy (`+0x2E4`, decision M10's Re:CoM stats), split into EXP prizes by Re:CoM's own defeat EXP drop
/// `0x1401810A0` (its EXP field holds the bonus for the call), and each KH1 prize as Re:CoM's prize of the
/// same use (`crate::drops::reward_prize_kind`: KH1's HP balls → Re:CoM's HP prize), launched as the defeat's
/// prizes, all at Re:CoM's prize point. Nothing without the enemy's `@ENP` row (Re:CoM's rule for its prizes).
unsafe fn reward(en: usize, rt: &mut Runtime, r: ke::Reward) {
    let id: u32 = rd(en + e::ID);
    let k = rt.class;
    if rd::<usize>(en + e::ENP_ROW) == 0 {
        line(k, || format!("e{id}: KH1's reward: no @ENP row (Re:CoM makes no prize without one)"));
        return;
    }
    let Some(p) = prize_point(en, rt) else {
        line(k, || format!("e{id}: KH1's reward: no finite prize point"));
        return;
    };
    let exp: i32 = rd(en + e::EXP);
    let bonus = r.exp_bonus(exp.max(0)).max(0);
    if bonus > 0 {
        wr(en + e::EXP, bonus);
        func::<ExpDropFn>(f::EXP_DROP)(en, &p);
        wr(en + e::EXP, exp);
    }
    let mut made = Vec::new();
    for (kind, &n) in r.prizes.iter().enumerate() {
        if n == 0 {
            continue;
        }
        // KH1's counts × the Jackpot factor (`0x142D60FA4`): 1, no such ability yet (M9).
        match crate::drops::reward_prize_kind(kind) {
            Some(recom) => {
                for _ in 0..n {
                    made.push((recom, super::drops::spawn_reward_prize(p, recom)));
                }
            }
            None => line(k, || format!("e{id}: KH1's reward: {n} prizes of KH1 kind {kind}: no Re:CoM prize for it yet")),
        }
    }
    line(k, || format!("e{id}: KH1's reward at ({:.2}, {:.2}, {:.2}): EXP bonus {bonus} (Re:CoM's EXP {exp} / {} + {}) as EXP prizes (0x1401810A0); prizes {made:x?}", p[0], p[1], p[2], r.exp_div, r.exp_add));
}

/// What the behaviour's handler did that KH1 does inside the handler (`Behaviour::take_now`): now.
unsafe fn run_now(en: usize, rt: &mut Runtime) {
    for c in rt.ai.take_now() {
        apply(en, rt, &c);
    }
}

// ---------------------------------------------------------------------------------------------
// The shared enemy-guard step (M11; `docs/recom-m11-enemies.md` § Enemy guard).
// ---------------------------------------------------------------------------------------------

/// Re:CoM's enemy vtable slot the hit handler calls for a hit that met a guarded body
/// (`0x14017F7E0`, the `+8 & 0x18` branch: after the guard spark `0x140182260` and popup 3,
/// `call [rax+0x3D8]`; its other callers: none found). Checked (decomp).
const GUARD_EVENT_SLOT: usize = 0x3D8;
/// Re:CoM's body (`CCollBody`, a hurt part: `enemy+0xF8` → `+0x40`, 0x120 each, count `+0x38`) guard
/// fields its hit apply `0x14016F1C0` reads: `+0x9C` bit 0 the guard, `+0x8C` the elements it guards
/// (`1 << attack +0x50`), `+0xB0` its half angle about the owner's facing (to the attack's `+0x60`); the
/// body ctor's defaults all elements and π (`0x14016F820`). A guarded hit: result 2, no damage, the
/// body's `+0x9C |= 0x20` (the hit handler's guard branch). Checked (decomp).
mod body {
    pub const FLAGS: usize = 0x9C;
    pub const GUARD: u32 = 1;
    pub const ELEMENTS: usize = 0x8C;
    pub const HALF_ANGLE: usize = 0xB0;
    pub const OWNER: usize = 0x20;
    pub const STRIDE: usize = 0x120;
}
/// The elements a KH1 enemy's guard row keeps out of Re:CoM's attacks: the physical (0). KH1's type
/// rule decides for Sora's KH1 swings (`guard_for_swing`); every other attack reaching the apply is
/// Re:CoM's (our spells carry Re:CoM's spell attributes 1..6, `win::magic::RECOM_ATTR`, KH1 types 5 / 6:
/// a type-2 row meets neither). Assumption: Re:CoM's other physical attacks (a turned-back shot, a
/// summon) count as KH1's type 0.
const GUARD_ELEMENTS: u32 = 1;

/// How many KH1 enemies carry the guard flag now (the per-swing and per-frame checks return at once
/// while it is 0).
static GUARDED: AtomicUsize = AtomicUsize::new(0);

/// The enemy's hurt parts.
unsafe fn parts(en: usize) -> Vec<usize> {
    let info: usize = rd(en + e::HIT_INFO);
    if info == 0 {
        return Vec::new();
    }
    let arr: usize = rd(info + hi::PARTS);
    let n: i32 = rd(info + hi::COUNT);
    if arr == 0 {
        return Vec::new();
    }
    (0..n.clamp(0, 32) as usize).map(|i| arr + i * body::STRIDE).collect()
}

/// Re:CoM's own body guard on every hurt part (`half`: the row's half angle), or back to what the
/// parts held before.
unsafe fn guard_bodies(en: usize, rt: &mut Runtime, half: Option<f32>) {
    match half {
        Some(h) if !rt.guarded => {
            rt.guard_saved = parts(en).iter().map(|&p| (rd::<u32>(p + body::FLAGS) & body::GUARD, rd::<u32>(p + body::ELEMENTS), rd::<f32>(p + body::HALF_ANGLE))).collect();
            for p in parts(en) {
                wr(p + body::FLAGS, rd::<u32>(p + body::FLAGS) | body::GUARD);
                wr(p + body::ELEMENTS, GUARD_ELEMENTS);
                wr(p + body::HALF_ANGLE, h);
            }
            rt.guarded = true;
            GUARDED.fetch_add(1, Ordering::Relaxed);
        }
        // Another row's half angle (M11, the Defender: the front row's 70° under the spin's all round).
        Some(h) if rt.guarded => {
            for p in parts(en) {
                if rd::<f32>(p + body::HALF_ANGLE) != h {
                    wr(p + body::HALF_ANGLE, h);
                }
            }
        }
        None if rt.guarded => {
            for (p, &(g, el, a)) in parts(en).into_iter().zip(&rt.guard_saved) {
                wr(p + body::FLAGS, rd::<u32>(p + body::FLAGS) & !body::GUARD | g);
                wr(p + body::ELEMENTS, el);
                wr(p + body::HALF_ANGLE, a);
            }
            rt.guard_saved.clear();
            rt.guarded = false;
            let _ = GUARDED.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1));
        }
        _ => {}
    }
}

/// One KH1 frame of the guard row: it lives while its request plays (ends once another plays, or
/// when its request never plays), and is on while its HIT slot is (KH1 frames of Re:CoM's motion clock).
unsafe fn guard_step(en: usize, rt: &mut Runtime, step: f32) {
    let Some(mut g) = rt.guard else {
        guard_bodies(en, rt, None);
        return;
    };
    let playing = g.request == ke::guard::ALWAYS || rt.current == Some(g.request);
    if playing {
        g.seen = true;
    } else if g.seen || g.age >= ce::watchdog::START_LIMIT {
        guard_end(en, rt);
        return;
    }
    g.age += step;
    rt.guard = Some(g);
    let m = manifest(rt.class);
    let Some(row) = m.guards.get(g.row as usize) else { return };
    // KH1's mode 3 (`ke::guard::ALWAYS` = `HELD`: the Large Body's front, the Neoshadow's body row): `live`
    // is on every frame until `Cmd::Guard(None)`.
    let on = playing && row.live(g.request, crate::units::units_to_frames(clock(en)));
    rt.guard_live = on;
    // A mode-3 row under it (the Defender's front) guards whatever the top row's window says; the bodies take the wider
    // of the two half angles (Re:CoM's one guard flag; KH1's own cone decides for Sora's swings, [`guard_for_swing`]).
    let under = rt.guard_under.and_then(|u| m.guards.get(u.row as usize)).and_then(|r| r.half_angle());
    let top = if on { row.half_angle() } else { None };
    let half = match (top, under) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    };
    guard_bodies(en, rt, half);
}

/// The guard row on top ends: the mode-3 row under it (if any) is the guard again (its bodies keep the flag with its
/// half angle), else Re:CoM's own body values come back.
unsafe fn guard_end(en: usize, rt: &mut Runtime) {
    rt.guard = rt.guard_under.take();
    let half = rt.guard.and_then(|g| manifest(rt.class).guards.get(g.row as usize)).and_then(|r| r.half_angle());
    guard_bodies(en, rt, half);
}

/// `win::attack`'s enemy-hit wrapper, each frame for each enemy before Re:CoM's hit handler: a KH1
/// enemy whose guard is on while a state Re:CoM owns runs (Stop, a stun, freeze: our states' steps
/// don't run, so its row's request can't be watched) drops its guard: KH1's statuses end the action
/// and its rows with it.
pub(super) unsafe fn guard_watch(en: usize) {
    if GUARDED.load(Ordering::Relaxed) == 0 {
        return;
    }
    let Some(k) = ours(en) else { return };
    let st = crate::guard::state_index(en, rd(en + e::STATE)).map(|s| s as i32);
    if st.is_some_and(|s| ce::replaced_states(CLASSES[k]).contains(&s)) {
        return;
    }
    with_rt(en, |rt| {
        if rt.guarded && !rt.guard.is_some_and(|g| g.request == ke::guard::ALWAYS) {
            guard_end(en, rt);
            line(rt.class, || format!("e{}: Re:CoM state {st:?} runs: its guard row ends", rd::<u32>(en + e::ID)));
        }
    });
}

/// Re:CoM's attack-set radius setter (`0x14014E5C0`: the set, the radius in metres in xmm1).
type RadiusFn = unsafe extern "win64" fn(usize, f32);

/// A KH1 child ring on Re:CoM's own shockwave record (M11, the Large Body's stomp; `kh1_enemy::Ring`): while its
/// request plays, inside the ring's frames, the record's radius is KH1's (`growth` × the frames since its start), set
/// as Re:CoM's own Large Body grows its shockwave (`0x14018F280` → `0x14014E5C0(enemy+0x100, r)`). One option test a
/// frame for every other class.
unsafe fn ring_step(en: usize, rt: &mut Runtime) {
    let Some(ring) = manifest(rt.class).ring else { return };
    if rt.current != Some(ring.request) || rt.dead {
        return;
    }
    let t = crate::units::units_to_frames(clock(en));
    let Some(r) = ring.radius(t) else { return };
    let set: usize = rd(en + e::ATTACKS);
    if set == 0 {
        return;
    }
    func::<RadiusFn>(f::SET_RADIUS)(set, r);
    // KH1's band on the ring's circle (its live type-11 entry: the only circle record on the ring's clip), so
    // Re:CoM's circle test runs KH1's ring rule on it ([`circle_test`]). Every ring frame: Re:CoM makes the entry's
    // shape anew when the record's window opens (`0x14014E830` → the factory `0x1401720E0`, `+0x74` = 0).
    RING_HEIGHT.store(ring.height_m().to_bits(), Ordering::Relaxed);
    let band = ring.band_m();
    let entries: usize = rd(set + set::ENTRIES);
    let live = rd::<u32>(set + set::LIVE).min(set::MADE) as usize;
    let mut marked = 0;
    if entries != 0 {
        for i in 0..live {
            let at = entries + i * set::ENTRY;
            let coll: usize = rd(at + set::E_COLL);
            let shape: usize = if coll == 0 { 0 } else { rd(coll + set::C_SHAPE) };
            if shape != 0 && rd::<u32>(at + set::E_STATE) == 1 && rd::<u32>(shape + circle::KIND) == circle::CIRCLE {
                wr::<f32>(shape + circle::BAND, band);
                marked += 1;
            }
        }
    }
    if t - ring.start < 1.0 {
        let k = rt.class;
        line(k, || format!("e{}: its ring starts (KH1's child at frame {:.0} of {}; Re:CoM's circle record grown {} a frame for {} frames, KH1's band {band} m and height {} m on {marked} circle(s))", rd::<u32>(en + e::ID), t, ring.request, ring.growth, ring.life, ring.height_m()));
    }
}

/// The ring's height window in metres (`f32` bits), set by [`ring_step`] before it marks a circle.
static RING_HEIGHT: AtomicU32 = AtomicU32::new(0);
pub static ORIG_CIRCLE: AtomicUsize = AtomicUsize::new(0);
/// Our rings' hits so far (the log names the first few).
static RING_HITS: AtomicU32 = AtomicU32::new(0);

/// Re:CoM's shape fields the circle test reads (`0x140177190`, Checked (decomp)): `+0x10` the place (a circle's centre,
/// a column's lower end), `+0x20` a column's other end / a sphere's centre, `+0x50` the contact point, `+0x60` the type,
/// `+0x70` the radius, `+0x74` a sphere's lift (a circle's: 0 from the factory `0x1401720E0`, read by nothing: our
/// band).
mod circle {
    pub const P10: usize = 0x10;
    pub const P20: usize = 0x20;
    pub const CONTACT: usize = 0x50;
    pub const KIND: usize = 0x60;
    pub const RADIUS: usize = 0x70;
    pub const BAND: usize = 0x74;
    pub const CIRCLE: u32 = 11;
}

/// Re:CoM's circle-against-body test (`0x140177190`: the collision context, the body's shape, the circle; 1 on a hit).
type CircleFn = unsafe extern "win64" fn(usize, usize, usize) -> u64;

/// The wrapper on Re:CoM's circle-against-body test (`sites::KH1_ENEMY_RING_HOOKS`; the pair dispatch `0x14016EAD0`
/// cases 0xB2 / 0xB4 / 0xB5). A circle with a band (`+0x74` > 0: only our stomp ring, [`ring_step`]) is KH1's ring
/// (`ex_2050_00.bd` `0x18`, [`ce::ring_hits`]) on Re:CoM's body: the body's position point ([`ce::body_foot`]) within
/// the height window of the circle's centre and its flat distance strictly inside (r − band, r), no body radius; on a
/// hit the contact point written as Re:CoM's test writes it (the body's x / z at the ring's height, on the circle and
/// the body). Every other circle: Re:CoM's test, unchanged.
pub unsafe extern "win64" fn circle_test(ctx: usize, body: usize, shape: usize) -> u64 {
    let band: f32 = if shape == 0 { 0.0 } else { rd(shape + circle::BAND) };
    if !(band > 0.0) || body == 0 {
        return orig::<CircleFn>(&ORIG_CIRCLE)(ctx, body, shape);
    }
    let v = |at: usize| -> [f32; 3] { [rd(at), rd(at + 4), rd(at + 8)] };
    let centre = v(shape + circle::P10);
    let Some(foot) = ce::body_foot(rd(body + circle::KIND), v(body + circle::P10), v(body + circle::P20), rd(body + circle::RADIUS), rd(body + circle::BAND)) else {
        return orig::<CircleFn>(&ORIG_CIRCLE)(ctx, body, shape);
    };
    let (r, height) = (rd::<f32>(shape + circle::RADIUS), f32::from_bits(RING_HEIGHT.load(Ordering::Relaxed)));
    if !ce::ring_hits(centre, r, band, height, foot) {
        return 0;
    }
    let contact = [foot[0], centre[1], foot[2], 1.0f32];
    for (i, c) in contact.iter().enumerate() {
        wr::<f32>(shape + circle::CONTACT + 4 * i, *c);
        wr::<f32>(body + circle::CONTACT + 4 * i, *c);
    }
    if RING_HITS.fetch_add(1, Ordering::Relaxed) < 20 {
        let (dx, dz) = (foot[0] - centre[0], foot[2] - centre[2]);
        log(&format!("kh1 enemies: the stomp ring hit (KH1's ring: r {r:.2} m, band {band} m, distance {:.2} m, height {:+.2} m within {height} m)", (dx * dx + dz * dz).sqrt(), foot[1] - centre[1]));
    }
    1
}

/// Wraps Re:CoM's circle-against-body test (once, after the classes are in) when an installed class has a ring
/// (the Large Body). Without it our ring is Re:CoM's solid disk (logged).
pub unsafe fn install_ring_test(bs: usize) {
    if !(0..N).any(|k| installed(k) && manifest(k).ring.is_some()) {
        return;
    }
    let hooks = [super::hook::Hook { site: &crate::sites::KH1_ENEMY_RING_HOOKS[0], ours: circle_test as *const () as usize, original: &ORIG_CIRCLE }];
    if super::hook::install_all(bs, &hooks) {
        log("kh1 enemies: Re:CoM's circle test wrapped: the Large Body's stomp ring hits as KH1's ring (its 1 m edge, 0.5 m height); every other circle Re:CoM's");
    } else {
        log("kh1 enemies: Re:CoM's circle test not wrapped: the Large Body's stomp ring stays Re:CoM's solid disk");
    }
}

/// What `pair_dispatch` needs of a guarded body a KH1 swing touched.
pub(super) struct SwingGuard {
    /// The guarding enemy.
    enemy: usize,
    /// Its live guard rows in KH1's list order (a mode-3 row made first, under a windowed one: M11, the Defender) that
    /// meet the swing (KH1's types and front cone): clash type, priority, the row under.
    rows: Vec<(u32, i32, bool)>,
    /// One of them meets it (`0x140528CB8`, `0x1402B4640`): Re:CoM's apply then guards it whatever its element.
    pub meets: bool,
    /// The body's fields to put back after the apply.
    saved: (u32, u32),
    body: usize,
    /// Sora's side of the clash, filled by [`guard_after_swing`].
    pub sora: Option<(u8, i32)>,
}

/// Before Re:CoM's apply of a KH1 swing (type `kind`) on body `node`: when it is a hurt part of a KH1
/// enemy whose guard row is on, the apply guards it only if KH1's types meet (else the guard bit is
/// off for the call), and then for the swing's element (`coll+0x50`, whatever the Damage setting
/// gave it). None (nothing changed) otherwise.
pub(super) unsafe fn guard_for_swing(node: usize, coll: usize, kind: u32) -> Option<SwingGuard> {
    if GUARDED.load(Ordering::Relaxed) == 0 {
        return None;
    }
    let owner: u32 = rd(node + body::OWNER);
    let en = func::<HandleFn>(f::HANDLE_OBJ)(owner);
    if en == 0 {
        return None;
    }
    ours(en)?;
    let rows = with_rt(en, |rt| {
        if !rt.guarded {
            return None;
        }
        let m = manifest(rt.class);
        // KH1 tests the pair only when the types meet (`0x1402B3BE0`) and the swing comes from inside the row's front
        // cone (`0x1402B4640`); a row that doesn't meet it leaves the apply a plain hit.
        let top = rt.guard.filter(|g| g.request == ke::guard::ALWAYS || rt.guard_live);
        let v: Vec<(u32, i32, bool)> = [(rt.guard_under, true), (top, false)]
            .into_iter()
            .filter_map(|(g, under)| m.guards.get(g?.row as usize).map(|r| (r, under)))
            .filter(|(r, _)| ke::guard::meets(kind, r.clash_type) && kh1_front(en, r))
            .map(|(r, under)| (r.clash_type, r.priority, under))
            .collect();
        Some(v)
    })??;
    let meets = !rows.is_empty();
    let saved = (rd::<u32>(node + body::FLAGS), rd::<u32>(node + body::ELEMENTS));
    if meets {
        let el: u32 = rd(coll + crate::guard::coll::ELEMENT);
        wr(node + body::ELEMENTS, saved.1 | 1u32.checked_shl(el).unwrap_or(0));
    } else {
        wr(node + body::FLAGS, saved.0 & !body::GUARD);
    }
    Some(SwingGuard { enemy: en, rows, meets, saved, body: node, sora: None })
}

/// KH1's front cone of a guard row (`0x1402B4640`: the 3D cosine between the enemy's facing (`actor+0x30`,
/// `(sin yaw, 0, cos yaw)`, `0x140297E10`) turned by row `+0x60`, and the vector from it to the attacker's
/// owner, at least row `+0x64`; 0 when either is zero). Here: Re:CoM's mover positions of the enemy and Sora
/// and its facing (Assumption: KH1's `+0x50` points are the actors' positions). Re:CoM's own half angle
/// (`+0xB0`) is a yaw-only test; KH1's is 3D, which matters for a cone like the Search Ghost's burst rows
/// (cos 1.0: never, as in KH1, where Re:CoM's `|Δyaw| ≤ 0` would pass a Sora exactly ahead at any height).
unsafe fn kh1_front(en: usize, row: &ke::guard::GuardRow) -> bool {
    let m = mover(en);
    let Some(sora) = sora_pos() else { return false };
    if m == 0 {
        return false;
    }
    let me = crate::units::to_kh1(rd_vec(m + mv::POS));
    let yaw = crate::units::yaw_to_kh1(rd(m + mv::YAW));
    row.front(yaw, sora - me)
}

/// After the apply (`r`): the body's fields back; when it guarded (2), KH1's clash for both sides
/// (`0x1402B4BC0` both orders): Sora's kind and difference into `g.sora`, the enemy's kind kept for
/// its guard event.
pub(super) unsafe fn guard_after_swing(g: &mut SwingGuard, r: i32, kind: u32, priority: i32) {
    wr(g.body + body::FLAGS, (rd::<u32>(g.body + body::FLAGS) & !body::GUARD) | (g.saved.0 & body::GUARD));
    wr(g.body + body::ELEMENTS, g.saved.1);
    if r != 2 || !g.meets {
        return;
    }
    // Each row that meets the swing resolves both ways (`0x1402B4A20`): the swing keeps its first record (it is marked
    // after it, `+4 & 0x30`); the enemy hears a repel before a clash (the Defender's front row repels while its spin's
    // row, at 100, moves neither).
    let mut sora = ke::guard::Side::None;
    let mut enemy: Option<(ke::guard::Side, bool)> = None;
    for &(t, p, under) in &g.rows {
        let s = ke::guard::resolve(kind, priority, p);
        if matches!(sora, ke::guard::Side::None) {
            sora = s;
        }
        let e = ke::guard::resolve(t, p, priority);
        enemy = match (enemy, e) {
            (None, e) => Some((e, under)),
            (Some((ke::guard::Side::Repels, u)), _) => Some((ke::guard::Side::Repels, u)),
            (Some(_), ke::guard::Side::Repels) => Some((ke::guard::Side::Repels, under)),
            (Some((ke::guard::Side::None, _)), e @ ke::guard::Side::Clash { .. }) => Some((e, under)),
            (x, _) => x,
        };
    }
    if let ke::guard::Side::Clash { kind, diff } = sora {
        g.sora = Some((kind, diff));
    }
    let en = g.enemy;
    let prios: Vec<i32> = g.rows.iter().map(|r| r.1).collect();
    with_rt(en, |rt| {
        rt.guard_side = enemy;
        let k = rt.class;
        line(k, || format!("e{}: its guard row stopped Sora's swing (priority {priority} vs {prios:?}): Sora {sora:?}, it {enemy:?}", rd::<u32>(en + e::ID)));
    });
}

/// Re:CoM's actor popup (`CCommonGauge`, `0x14020C9A0(handle, kind)`) and its kind 3, "Guard" (the
/// hit handler's guarded branch; Checked (decomp)).
const POPUP: u32 = 0x0020_C9A0;
const POPUP_GUARD: i32 = 3;
type PopupFn = unsafe extern "win64" fn(i32, i32);

/// The enemy hit handler's guard popup (`sites::M11_GUARD_POPUP_CALLS`, retargeted at install):
/// over one of our KH1 enemies Re:CoM's "Guard" popup is skipped (owner 2026-11-02: "hide Re:CoM's
/// "Guard" popup over KH1 enemies"); the spark, the clink and the guard event still run. Any other
/// actor or kind: Re:CoM's popup, as its call was.
pub unsafe extern "win64" fn guard_popup(handle: i32, kind: i32) {
    if kind == POPUP_GUARD && handle > 0 {
        let en = func::<HandleFn>(f::HANDLE_OBJ)(handle as u32);
        if let Some(k) = (en != 0).then(|| ours(en)).flatten() {
            line(k, || format!("e{}: Re:CoM's Guard popup skipped", rd::<u32>(en + e::ID)));
            return;
        }
    }
    func::<PopupFn>(POPUP)(handle, kind)
}

/// Retargets the hit handler's guard popup call to [`guard_popup`] (once, after the classes are in;
/// its context checked first). Without it Re:CoM's popup shows over KH1 enemies too (logged).
pub unsafe fn install_guard_popup(bs: usize) {
    if !any_installed() {
        return;
    }
    for c in crate::sites::M11_GUARD_POPUP_CHECKED {
        if std::slice::from_raw_parts((bs + c.rva as usize) as *const u8, c.expect.len()) != c.expect {
            log(&format!("kh1 enemies: {} is not as expected: Re:CoM's Guard popup stays over KH1 enemies", c.name));
            return;
        }
    }
    let s = &crate::sites::M11_GUARD_POPUP_CALLS[0];
    let at = bs + s.rva as usize;
    let call = super::hook::place_near(at, &crate::hookplan::abs_jmp(guard_popup as *const () as u64)).and_then(|stub| crate::hookplan::call_rel32(at as u64, stub as u64).map(|c| (stub, c)));
    let Some((stub, call)) = call else {
        log("kh1 enemies: no stub within reach of the guard popup call: Re:CoM's Guard popup stays over KH1 enemies");
        return;
    };
    if super::hook::patch(at, s.expect, &call, s.name) {
        log(&format!("kh1 enemies: {} retargeted through {stub:#x}: no Re:CoM Guard popup over KH1 enemies", s.name));
    }
}

/// The guard event (vtable `+0x3D8`, Re:CoM's hit handler on a guarded hit): a KH1 enemy whose guard
/// row is on gets its side of KH1's `0x1402B4BC0`:
/// - a clash (kind 6 / 7; 6 for an attacker with no KH1 priority, a Re:CoM attack): its row's recoil
///   through our state 11, its script hears the clash; its row ends (KH1: a clashed row meets nothing more,
///   `+4 & 0x30`);
/// - a repel (its type 1 / 3 / 4 row has the higher priority): KH1 gives it the attacker's hit record
///   flagged 0x20 (`0x1402BF630`): no damage (Re:CoM's guard already kept it out), no reaction, its attack
///   goes on; its script's hit handler hears it ([`HitNews::Repelled`]). The row that met is done (`+4 |=
///   0x10`): a plain row ends; a merged one (`GuardRow::rows` > 1) stays on (Assumption: its other KH1 rows,
///   the other hand or the other strike, can still meet a later swing; Re:CoM's body guard can't tell them
///   apart);
/// - nothing (KH1: a side at 100, or both 98 or more): no reaction; its row stays on.
///
/// Anything else: Re:CoM's slot.
pub unsafe extern "win64" fn guard_event(en: usize) {
    enum Then {
        Clash(bool),
        Done,
    }
    let r = with_rt(en, |rt| {
        if !rt.guarded {
            return None;
        }
        // KH1's side for the enemy (`pair_dispatch`'s, with the row it came from), a kind-6 clash for any other attacker
        // (a Re:CoM attack has no KH1 priority) on the top row while its window is on, else the row under; a row at 98 or
        // more is never moved by anything (`0x1402B4BC0` records only a side under 98: the Large Body's front (100) and the
        // Defender's (99) keep out Re:CoM's physical attacks too, and stay).
        let top_live = rt.guard.is_some_and(|g| g.request == ke::guard::ALWAYS || rt.guard_live);
        let (side, under) = match rt.guard_side.take() {
            Some(x) => x,
            None => {
                let under = !top_live && rt.guard_under.is_some();
                let g = if under { rt.guard_under } else { rt.guard }?;
                let row = manifest(rt.class).guards.get(g.row as usize)?;
                (if row.priority >= ke::guard::CLASH_BELOW { ke::guard::Side::None } else { ke::guard::Side::Clash { kind: 6, diff: 0 } }, under)
            }
        };
        let g = if under { rt.guard_under } else { rt.guard }?;
        let row = manifest(rt.class).guards.get(g.row as usize)?;
        let (k, name, id) = (rt.class, row.name.clone(), rd::<u32>(en + e::ID));
        match side {
            ke::guard::Side::Clash { kind, .. } => {
                rt.clash_recoil = Some((row.recoil, kind));
                if under {
                    rt.guard_under = None;
                    let half = rt.guard.filter(|_| top_live).and_then(|t| manifest(rt.class).guards.get(t.row as usize)).and_then(|r| r.half_angle());
                    if half.is_none() {
                        guard_bodies(en, rt, None);
                    }
                } else {
                    guard_end(en, rt);
                }
                let in_attack = rt.action.is_some_and(|a| a.attack.is_some() && rt.action_playing);
                line(k, || format!("e{id}: guard event on its {name} row: KH1's clash (kind {kind}), recoil {}", row.recoil));
                Some(Then::Clash(in_attack))
            }
            ke::guard::Side::Repels => {
                // KH1's hit counter counts the record (`0x1402BF630` → `0x1402BFE60`).
                rt.hits = rt.hits.saturating_add(1);
                rt.ai.set_hit_count(rt.hits);
                if CLASSES[rt.class].reads_hp {
                    rt.ai.set_hp_ratio(hp_ratio(en));
                }
                rt.ai.set_playing(rt.current);
                if row.rows <= 1 && g.request != ke::guard::ALWAYS {
                    guard_end(en, rt);
                }
                rt.ai.on_hit(HitNews::Repelled);
                run_now(en, rt);
                line(k, || format!("e{id}: guard event on its {name} row: KH1's repel (no damage, no reaction; its hit handler hears it)"));
                Some(Then::Done)
            }
            ke::guard::Side::None => {
                line(k, || format!("e{id}: guard event on its {name} row: KH1 moves neither side"));
                Some(Then::Done)
            }
        }
    });
    match r.flatten() {
        // Re:CoM's attack end with state 11 next (bit 0 off), as a Guard block of its attack.
        Some(Then::Clash(true)) => func::<EndFn>(f::ATTACK_END)(en, crate::battle::state_address(en, state::FLINCH), 0),
        Some(Then::Clash(false)) => wr(en + e::NEXT, crate::battle::state_address(en, state::FLINCH)),
        Some(Then::Done) => {}
        None => call_original(en, GUARD_EVENT_SLOT),
    }
}

// ---------------------------------------------------------------------------------------------
// Attack meets attack (M11; `docs/recom-m11-enemies.md` § Attack meets attack (clashes)).
// ---------------------------------------------------------------------------------------------

/// Re:CoM's attack set (`enemy+0x100`, `CAttackMotion`): its live entries' count `+0x14`, entries `+0x18` (0x40
/// bytes each, 12 made: `+0x10`), its `.ASD` records `+0x28` (`+0x24` of them). An entry: `+0x14` its window state
/// (1: on, `0x14014E830`), `+0x18` its record, `+0x20` its collision (`CCollAttack`: `+0x10` on / off, set by
/// vtable `+0x10` / `+8`, `0x14014DE90` / `0x14014DE80`; `+0x18` its shape, `+0x60` the shape's type). Checked
/// (decomp: `0x14014DBE0`, `0x14014E010`, `0x14014E830`, `0x14014E6A0`, `0x14016F110`).
mod set {
    pub const LIVE: usize = 0x14;
    pub const ENTRIES: usize = 0x18;
    pub const RECORD_COUNT: usize = 0x24;
    pub const RECORDS: usize = 0x28;
    pub const MADE: u32 = 12;
    pub const ENTRY: usize = 0x40;
    pub const E_STATE: usize = 0x14;
    pub const E_RECORD: usize = 0x18;
    pub const E_COLL: usize = 0x20;
    pub const C_ON: usize = 0x10;
    pub const C_SHAPE: usize = 0x18;
}

/// The collisions of KH1 enemy rows a clash marked this frame (KH1's `+4 & 0x30`: they meet no hurt body), and
/// their enemies, until each enemy's hit-handler call ([`attack_clash_watch`]); [`suppressed`] answers for
/// `win::attack::pair_dispatch`.
static SUPPRESS: Mutex<Vec<(usize, usize)>> = Mutex::new(Vec::new());
static SUPPRESSED: AtomicUsize = AtomicUsize::new(0);
/// Enemies with an `attack_clash` waiting (one load per enemy update otherwise).
static CLASH_WAITING: AtomicUsize = AtomicUsize::new(0);
/// KH1's met masks: (Sora's swing, the enemy row's collision) pairs that met; two attack objects meet once while
/// both live (`0x1402B4BC0` sets `+0x9C` both ways, `0x1402B3BE0` tests it). Kept for the swing (`gen`).
static MET: Mutex<Vec<(u32, usize)>> = Mutex::new(Vec::new());

/// A live KH1 enemy row a swing may meet.
struct Candidate {
    enemy: usize,
    coll: usize,
    shape: usize,
    row: ce::ClashRow,
    front: bool,
}

/// 0.183.1: the KH1 row (clash type, priority) of a live KH1 enemy attack collision `coll`, from the asset
/// step's records ([`live_rows`]); None for Re:CoM's own attacks and anything else.
pub(super) unsafe fn attack_row(coll: usize) -> Option<(u32, i32)> {
    if coll == 0 || !any_installed() {
        return None;
    }
    live_rows().into_iter().find(|c| c.coll == coll).map(|c| (c.row.clash_type, c.row.priority))
}

/// KH1's front cone (`0x1402B4640`) of a row of clash type `t` against Sora (only guard types test it), with
/// [`kh1_front`]'s positions.
unsafe fn row_front(en: usize, r: &ce::ClashRow) -> bool {
    if !ke::guard::is_guard_type(r.clash_type) {
        return true;
    }
    let m = mover(en);
    let Some(sora) = sora_pos() else { return false };
    if m == 0 {
        return false;
    }
    let me = crate::units::to_kh1(rd_vec(m + mv::POS));
    ke::guard::front(r.clash_type, r.turn, r.cos, crate::units::yaw_to_kh1(rd(m + mv::YAW)), sora - me)
}

/// The live rows of this battle's KH1 enemies: each Re:CoM attack entry on (window state 1, collision on) whose
/// record the asset step made from a KH1 row (the manifest's `clash`), with a sphere or line shape (types 2 / 7).
unsafe fn live_rows() -> Vec<Candidate> {
    // 0.139.2 (the owner's crash, 2026-11-05: a field swing after a room change read a freed
    // enemy's attack set): the last battle's runtimes keep its number after the battle ends, so
    // only a running card battle's enemies are read, and only while each is still ours.
    if !super::battle::cbtl_live() {
        return Vec::new();
    }
    let Ok(v) = RT.try_lock() else { return Vec::new() };
    let battle = super::battle::battle_number();
    let mut out = Vec::new();
    for rt in v.iter().filter(|r| r.battle == battle && !r.dead) {
        let m = manifest(rt.class);
        if m.clash.is_empty() {
            continue;
        }
        let en = rt.enemy;
        if ours(en) != Some(rt.class) {
            continue;
        }
        let s: usize = rd(en + e::ATTACKS);
        if s == 0 {
            continue;
        }
        let entries: usize = rd(s + set::ENTRIES);
        let (records, rcount): (usize, u32) = (rd(s + set::RECORDS), rd(s + set::RECORD_COUNT));
        let live = rd::<u32>(s + set::LIVE).min(set::MADE) as usize;
        if entries == 0 || records == 0 {
            continue;
        }
        for i in 0..live {
            let at = entries + i * set::ENTRY;
            let (rec, coll): (usize, usize) = (rd(at + set::E_RECORD), rd(at + set::E_COLL));
            if rec == 0 || coll == 0 || rd::<u32>(at + set::E_STATE) != 1 || rd::<i32>(coll + set::C_ON) == 0 {
                continue;
            }
            let Some(row) = ce::record_index(rec, records, rcount).and_then(|r| m.clash_of(r)) else { continue };
            let shape: usize = rd(coll + set::C_SHAPE);
            if shape == 0 || !matches!(rd::<u32>(shape + 0x60), 2 | 7) {
                continue;
            }
            out.push(Candidate { enemy: en, coll, shape, row: row.clone(), front: row_front(en, row) });
        }
    }
    out
}

/// One KH1 swing frame against the KH1 enemies' live attack rows (`win::attack::pair_dispatch`, before the hurt
/// bodies, as KH1's attack pass comes before its body pass, `0x1402B4460`). `swing`: KH1's clash type and priority;
/// `gen`: the swing; `touch`: the swing's spheres against a Re:CoM shape (Re:CoM's own overlap tests), the contact
/// point when they meet. Returns Sora's clash record and whether the swing is marked (it meets no hurt body this
/// frame). Each enemy that got a record keeps it for its hit-handler call; each marked row's collision answers 0
/// against Sora until then. Re:CoM's guard spark at each meeting. Nothing while no KH1 enemy row is live.
pub(super) unsafe fn swing_meets_attacks(swing: (u32, i32), gen: u32, touch: &mut dyn FnMut(usize) -> Option<[f32; 4]>) -> (Option<(u8, i32)>, bool) {
    if !any_installed() {
        return (None, false);
    }
    let cands = live_rows();
    if cands.is_empty() {
        return (None, false);
    }
    let met: Vec<usize> = match MET.lock() {
        Ok(mut g) => {
            g.retain(|x| x.0 == gen);
            g.iter().map(|x| x.1).collect()
        }
        Err(_) => return (None, false),
    };
    let points: Vec<Option<[f32; 4]>> = cands.iter().map(|c| if met.contains(&c.coll) || !c.front || !ke::guard::meets(swing.0, c.row.clash_type) { None } else { touch(c.shape) }).collect();
    let rows = cands.iter().zip(&points).map(|(c, p)| ce::LiveRow { enemy: c.enemy, coll: c.coll, row: &c.row, touches: p.is_some(), front: c.front, met_before: met.contains(&c.coll) });
    let (sora, marked, meetings) = ce::sweep(swing, rows);
    // Enemy Skills stage 5: Luxord's Omni Break: Sora's swing wins every meeting
    // (`crate::enemy_skills_deck::omni_break`).
    let (sora, marked, meetings) = if super::enemy_skills_deck::luxord() && !meetings.is_empty() {
        let won = meetings
            .into_iter()
            .map(|m| match cands.iter().find(|c| c.coll == m.coll) {
                Some(c) => crate::enemy_skills_deck::omni_break(m, c.row.attack, c.row.priority, swing.1),
                None => m,
            })
            .collect::<Vec<_>>();
        (None, false, won)
    } else {
        (sora, marked, meetings)
    };
    for m in &meetings {
        if let Ok(mut g) = MET.lock() {
            g.push((gen, m.coll));
        }
        if m.stops {
            if let Ok(mut g) = SUPPRESS.lock() {
                g.push((m.coll, m.enemy));
                SUPPRESSED.store(g.len(), Ordering::Relaxed);
            }
        }
        let c = cands.iter().position(|c| c.coll == m.coll).expect("a meeting is one of the candidates");
        if let Some(p) = points[c] {
            super::attack::spark_at(p);
        }
        let row = &cands[c].row;
        let en = m.enemy;
        let gets = m.gets;
        with_rt(en, |rt| {
            let before = rt.attack_clash;
            rt.attack_clash = ce::stronger(before, gets);
            if before.is_none() && rt.attack_clash.is_some() {
                CLASH_WAITING.fetch_add(1, Ordering::Relaxed);
            }
            let (k, id) = (rt.class, rd::<u32>(en + e::ID));
            let name = CLASSES[k].attack_names.get(row.attack as usize).copied().unwrap_or("?");
            line(k, || format!("e{id}: its {name} row (type {}, priority {}) met Sora's swing (type {}, priority {}): Sora {sora:?}, it {gets:?}{}", row.clash_type, row.priority, swing.0, swing.1, if m.stops { ", its row stops this frame" } else { ", its row goes on" }));
        });
    }
    (sora, marked)
}

/// `win::attack::pair_dispatch`: this attack collision belongs to a KH1 enemy row a clash marked this frame (it
/// meets no hurt body: Re:CoM's test answers 0). One load while none is marked.
pub(super) fn suppressed(coll: usize) -> bool {
    SUPPRESSED.load(Ordering::Relaxed) != 0 && SUPPRESS.lock().is_ok_and(|g| g.iter().any(|x| x.0 == coll))
}

/// Some installed class's manifest has an attack row that never hurts ([`Manifest::has_never_hurting`]: the
/// Soldier's spin-kick chest, the Powerwild's flurry body); set at [`prepare`].
static NEVER_HURTING: AtomicBool = AtomicBool::new(false);

/// The owner handle of an attack collision (`CCollAttack +0x20`: the attack set's entry `+0x30`, its owner's
/// `+0x20`, written by `0x1401720E0` when `0x14016F720` makes the shape; the pair dispatch `0x14016EAD0` skips
/// bodies of that handle). Checked (decomp: `0x14014DBE0`, `0x14014E830`, `0x14016F720`, `0x1401720E0`).
const COLL_OWNER: usize = 0x20;

/// `win::attack::pair_dispatch`, Re:CoM's test of an attack collision against the body list `list`: the
/// collision is a live entry of one of our enemies' attack sets whose `.ASD` record the asset step made from a
/// KH1 row of clash type 1, 2 or 4, and `list` is the player side's (`*0x14087A838`). KH1 never tests such a row
/// against a hurt body (`0x1402B3BE0`): it only meets Sora's swings (the clash test, [`live_rows`], still sees
/// it), so Re:CoM's test answers 0 (owner 2026-11-02, "Clash choices": block only). Sora's KH1 Guard (type 2)
/// doesn't meet types 1 / 2 either (`0x140528CB8`), so no block is lost. One load while no installed class has
/// such a row; else, for a test against the player side, the owner lookup and, for one of our enemies with
/// such a row, a walk of its live entries (at most 12).
pub(super) unsafe fn never_hurts(coll: usize, list: usize) -> bool {
    if !NEVER_HURTING.load(Ordering::Relaxed) || coll == 0 || list == 0 || list != rd::<usize>(super::va(base(), crate::guard::PLAYER_LIST)) {
        return false;
    }
    let owner: i32 = rd(coll + COLL_OWNER);
    if owner <= 0 {
        return false;
    }
    let en = func::<HandleFn>(f::HANDLE_OBJ)(owner as u32);
    if en == 0 {
        return false;
    }
    let Some(k) = ours(en) else { return false };
    let Some(m) = MANIFEST[k].get().filter(|m| m.has_never_hurting()) else { return false };
    let s: usize = rd(en + e::ATTACKS);
    if s == 0 {
        return false;
    }
    let entries: usize = rd(s + set::ENTRIES);
    let (records, rcount): (usize, u32) = (rd(s + set::RECORDS), rd(s + set::RECORD_COUNT));
    if entries == 0 || records == 0 {
        return false;
    }
    let live = rd::<u32>(s + set::LIVE).min(set::MADE) as usize;
    for i in 0..live {
        let at = entries + i * set::ENTRY;
        if rd::<usize>(at + set::E_COLL) == coll {
            let rec: usize = rd(at + set::E_RECORD);
            let never = rec != 0 && ce::record_index(rec, records, rcount).is_some_and(|r| m.never_hurts(r));
            // Logged when the record or the collision changes, not each frame.
            static LAST: AtomicUsize = AtomicUsize::new(0);
            if never && LAST.swap(rec ^ coll.rotate_left(32), Ordering::Relaxed) != rec ^ coll.rotate_left(32) {
                let id: u32 = rd(en + e::ID);
                let r = ce::record_index(rec, records, rcount);
                line(k, || format!("e{id}: its record {r:?} (a KH1 row of clash type 1 / 2 / 4) is on: Re:CoM's tests of it against Sora answer 0 (it only meets swings)"));
            }
            return never;
        }
    }
    false
}

/// `win::attack`'s enemy-hit wrapper, each enemy update before Re:CoM's hit handler (Re:CoM tests the enemy's
/// attacks just before, `0x140185630`: its marked rows have answered 0): the record its attack row got from Sora's
/// swing plays, as KH1's reaction step plays a clash record (`0x1402B8E10` cases 6 / 7):
/// - a clash: its row's recoil through our state 11 (Re:CoM's attack end with state 11 next, as for a guard row's
///   clash); its script's hit handler hears the clash there. A row whose recoil the model lacks: Re:CoM's flinch
///   (the owner's rule for a guarded attack, 2026-11-02);
/// - a repel: its script's hit handler hears the 0x20 record; no damage, no reaction, its attack goes on.
///
/// Only in one of our states (a Re:CoM state such as Stop or a stun has ended its attack: dropped).
pub(super) unsafe fn attack_clash_watch(en: usize) {
    if CLASH_WAITING.load(Ordering::Relaxed) == 0 && SUPPRESSED.load(Ordering::Relaxed) == 0 {
        return;
    }
    let Some(k) = ours(en) else { return };
    if let Ok(mut g) = SUPPRESS.lock() {
        g.retain(|x| x.1 != en);
        SUPPRESSED.store(g.len(), Ordering::Relaxed);
    }
    let st = crate::guard::state_index(en, rd(en + e::STATE)).map(|s| s as i32);
    let ours_now = st.is_some_and(|s| ce::replaced_states(CLASSES[k]).contains(&s));
    let alive = rd::<i32>(en + e::HP) > 0;
    enum Then {
        End(bool),
        Done,
    }
    let then = with_rt(en, |rt| {
        let gets = rt.attack_clash.take()?;
        let _ = CLASH_WAITING.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1));
        let id: u32 = rd(en + e::ID);
        if !ours_now || !alive || rt.dead {
            line(rt.class, || format!("e{id}: its clash with Sora's swing dropped (Re:CoM state {st:?}, alive {alive})"));
            return Some(Then::Done);
        }
        match gets {
            ce::EnemyGets::Clash { attack, kind, diff } => {
                let name = CLASSES[rt.class].attack_names.get(attack as usize).copied().unwrap_or("?");
                match manifest(rt.class).recoil_of(attack) {
                    Some(r) => {
                        rt.clash_recoil = Some((r, kind));
                        line(rt.class, || format!("e{id}: its {name} clashed with Sora's swing (kind {kind}, {diff:+}): recoil {r}"));
                    }
                    None => {
                        rt.recom_flinch = true;
                        line(rt.class, || format!("e{id}: its {name} clashed with Sora's swing (kind {kind}): no KH1 recoil motion, Re:CoM's flinch"));
                    }
                }
                Some(Then::End(rt.action.is_some_and(|a| a.attack.is_some() && rt.action_playing)))
            }
            ce::EnemyGets::Repelled { attack } => {
                rt.hits = rt.hits.saturating_add(1);
                rt.ai.set_hit_count(rt.hits);
                if CLASSES[rt.class].reads_hp {
                    rt.ai.set_hp_ratio(hp_ratio(en));
                }
                rt.ai.set_playing(rt.current);
                rt.ai.on_hit(HitNews::Repelled);
                run_now(en, rt);
                let name = CLASSES[rt.class].attack_names.get(attack as usize).copied().unwrap_or("?");
                line(rt.class, || format!("e{id}: its {name} repelled Sora's swing (no damage, no reaction; its hit handler hears it)"));
                Some(Then::Done)
            }
            ce::EnemyGets::Nothing => Some(Then::Done),
        }
    });
    match then.flatten() {
        // Re:CoM's attack end with state 11 next (bit 0 off), as for a guard row's clash.
        Some(Then::End(true)) => func::<EndFn>(f::ATTACK_END)(en, crate::battle::state_address(en, state::FLINCH), 0),
        Some(Then::End(false)) => wr(en + e::NEXT, crate::battle::state_address(en, state::FLINCH)),
        _ => {}
    }
}

/// `super::guard`: Sora's Guard blocked this enemy's attack. A KH1 enemy plays KH1's clash recoil
/// (`0x1402B8E10` cases 6 / 7 → `0x1402B9140`) through state 11; its class's title when handled.
pub(super) unsafe fn guard_block(en: usize) -> Option<&'static str> {
    if !any_installed() {
        return None;
    }
    let k = ours(en)?;
    let attack = with_rt(en, |rt| {
        let a = rt.action.and_then(|a| a.attack);
        rt.clash = a;
        a
    })
    .flatten();
    let a = attack?;
    // Owner 2026-11-02: a blocked attack whose recoil the model lacks gets Re:CoM's flinch, as
    // before M10 (`super::guard`'s picker path); our state 11 then runs Re:CoM's.
    if manifest(k).recoil_of(a).is_none() {
        with_rt(en, |rt| {
            rt.clash = None;
            rt.recom_flinch = true;
        });
        let name = CLASSES[k].attack_names.get(a as usize).copied().unwrap_or("?");
        line(k, || format!("e{}: Guard blocked its {name} (no KH1 recoil motion): Re:CoM's flinch", rd::<u32>(en + e::ID)));
        return None;
    }
    // Re:CoM's attack end with state 11 next (bit 0 off, its card end).
    func::<EndFn>(f::ATTACK_END)(en, crate::battle::state_address(en, state::FLINCH), 0);
    let name = CLASSES[k].attack_names.get(a as usize).copied().unwrap_or("?");
    line(k, || format!("e{}: Guard blocked its {name}", rd::<u32>(en + e::ID)));
    Some(CLASSES[k].title)
}

/// `super::guard`: Re:CoM's picker ran for a block `guard_block` handed back; without its flinch the
/// mark is dropped (a later hit is KH1's reaction again).
pub(super) unsafe fn recom_flinch_settled(en: usize, flinched: bool) {
    if !flinched && any_installed() {
        with_rt(en, |rt| rt.recom_flinch = false);
    }
}

/// `super::guard`: an attack collision owned by `owner` (a handle) hit Sora: a KH1 enemy's
/// behaviour hears it for the attack it runs (KH1's attack object hit callback, `+0x38`).
pub(super) unsafe fn attack_landed(owner: u32) {
    if !any_installed() {
        return;
    }
    let en = func::<HandleFn>(f::HANDLE_OBJ)(owner);
    if en == 0 || ours(en).is_none() {
        return;
    }
    with_rt(en, |rt| {
        if let Some(a) = rt.action.and_then(|a| a.attack) {
            rt.ai.on_landed(a);
            let name = CLASSES[rt.class].attack_names.get(a as usize).copied().unwrap_or("?");
            line(rt.class, || format!("e{}: its {name} hit Sora", rd::<u32>(en + e::ID)));
        }
    });
}

/// The trace's tokens (F9 recording): per class (`shadow …`, `soldier …`), every KH1 enemy's
/// request, reaction, action and hittable flag.
pub unsafe fn trace_token() -> String {
    if !any_installed() {
        return String::new();
    }
    let Ok(v) = RT.try_lock() else { return String::new() };
    let mut out = Vec::new();
    for (k, c) in CLASSES.iter().enumerate() {
        let t: Vec<String> = v
            .iter()
            // Re:CoM removes a defeated enemy after its 1500-unit (30-frame) timer: not listed after.
            .filter(|r| r.class == k && r.battle == super::battle::battle_number() && !(r.dead && r.dead_frames > 31))
            .take(8)
            .map(|r| {
                format!(
                    "{:x}:r{:?}:{:?}:{}:h{}",
                    r.enemy & 0xFFFFFF,
                    r.current,
                    r.reaction.map(|p| match p {
                        Phase::Motion => 'm',
                        Phase::Air { .. } => 'a',
                        Phase::Down { .. } => 'd',
                    }),
                    r.action.map_or(0, |a| a.request),
                    r.hittable as u8
                )
            })
            .collect();
        if !t.is_empty() {
            out.push(format!("{} {}", c.name, t.join(" ")));
        }
    }
    let blades = super::kh1_thrown::trace_token();
    if !blades.is_empty() {
        out.push(blades);
    }
    let flames = super::kh1_flame::trace_token();
    if !flames.is_empty() {
        out.push(flames);
    }
    out.join("; ")
}
