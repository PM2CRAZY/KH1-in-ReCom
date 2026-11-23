//! KH1 movement in Re:CoM's player actions (`crate::movement`, `docs/recom-kh1-movement.md`).
//!
//! Hooks (`sites::MOVE_HOOKS`), each a pass-through unless `combat = kh1` and the player is the
//! main player's `CPlayerSora`:
//!
//! - move exe `0x1401EEDA0` (actions 2/3/4): Re:CoM's own exe still runs its card / attack /
//!   interaction dispatch and its ledge checks; we only step in when Sora is in the air (KH1's
//!   walk-off, before Re:CoM's ground commands), and through the two leaves below.
//! - stick class `0x1401E9630` (inside the move exe): KH1's speed snap (tilt line 0.8, KH1's dead
//!   zone) → action 2/3/4 and `player+0x4DC`.
//! - move step `0x1401EF3C0` (inside the move exe): KH1's ground frame.
//! - jump init `0x1401F29F0`, avoid init `0x1401F10C0`: KH1's frame with the jump / Square press
//!   (jump, Guard, Dodge Roll); `jump init(player, 1)` (Re:CoM starting a fall) enters KH1's air.
//! - jump exe `0x1401EDA60` (action 7), `CPlayerSora` avoid exe `0x140217310` (6) and glide exe
//!   `0x140217B50` (8): KH1's frame. The jump exe keeps Re:CoM's air commands (attack card or field
//!   swing, magic card) until M5 replaces them.
//! - `dodge_roll = recom` (the default): when KH1's rule asks for the roll, the KH1 frame calls
//!   Re:CoM's own avoid init (trampoline) and the avoid exe hook runs Re:CoM's exe whole while
//!   Re:CoM owns that roll (`crate::movement::roll`); the next KH1 frame takes Sora back afresh.
//!
//! Re:CoM then plays the requested rows, advances and ends the motions, applies the roll's root
//! motion and moves / collides the body with the velocity we leave in `body+0x80`.

use super::{base, log, va};
use crate::movement::{Action, FrameOut, Host, Kh1Sora};
use crate::rows::{self, Extension, Stance};
use crate::settings::Kh1Abilities;
use crate::sites::data;
use crate::trace::{self, PadView};
use crate::units::{frames_to_units, to_kh1, to_recom, units_to_frames, yaw_to_kh1, yaw_to_recom};
use kh1_core::{Button, Params, Vec3};
use crate::settings::RollMode;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

pub static ORIG_MOVE_EXE: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_STICK_CLASS: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_MOVE_STEP: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_JUMP_INIT: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_AVOID_INIT: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_JUMP_EXE: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_AVOID_EXE: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_RIKU_AVOID_EXE: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_DRIKU_AVOID_EXE: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_GLIDE_EXE: AtomicUsize = AtomicUsize::new(0);

/// `combat = kh1`, and the hooks are in.
static KH1: AtomicBool = AtomicBool::new(false);
static INSTALLED: AtomicBool = AtomicBool::new(false);
/// The ini's abilities (`Kh1Abilities::bits`, and all of them): the fallback while the ability
/// state isn't live (M9 9b).
static ABILITY_BITS: AtomicU32 = AtomicU32::new(0x30);
static ABILITIES: Mutex<Option<Kh1Abilities>> = Mutex::new(None);
/// `dodge_roll` from the ini (read at start): 0 Re:CoM's own roll, 1 KH1's, 2 none.
static ROLL_MODE: AtomicU8 = AtomicU8::new(0);
/// `run_speed`, `jump_height`, `air_speed` (bit set: Re:CoM's), from the ini at start.
static SPEEDS: AtomicU8 = AtomicU8::new(1);
/// The player whose roll Re:CoM runs ([`crate::movement::roll`]), 0 when none was handed over.
static RECOM_ROLL: AtomicUsize = AtomicUsize::new(0);
static ROLL_WATCH: Mutex<crate::movement::roll::Watch> = Mutex::new(crate::movement::roll::Watch { from: 0, frames: 0.0, calls: 0, untouchable: None, untouchable_at: None, clock: 0 });
static ROLL_LINES: AtomicU32 = AtomicU32::new(0);
/// The appended action-table rows (KH1-only clips).
pub static EXT: OnceLock<Extension> = OnceLock::new();
/// The player whose move exe is running (its leaves run our frame), and whether it takes over.
static IN_MOVE_EXE: AtomicUsize = AtomicUsize::new(0);
static MOVE_FRESH: AtomicBool = AtomicBool::new(false);

/// Re:CoM's action fields as our last frame left them: the next frame continues from them only
/// if Re:CoM ran nothing else in between.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sync {
    action: u32,
    prev: u32,
    time: i32,
}

pub struct Slot {
    pub player: usize,
    pub sora: Kh1Sora,
    pub sync: Option<Sync>,
    pub last: FrameOut,
}

pub static SORA: Mutex<Option<Slot>> = Mutex::new(None);

// CPlayer fields. Checked (decomp) unless noted; `docs/recom-kh1-movement.md`.
pub(super) mod p {
    pub const BODY: usize = 0xF0;
    pub const CTRL: usize = 0xD8;
    pub const PAD: usize = 0x230;
    pub const BATTLE: usize = 0x390;
    pub const TARGET_CURSOR: usize = 0x3F8;
    pub const PREV_ACTION: usize = 0x408;
    pub const ACTION: usize = 0x40C;
    pub const ACTION_TIME: usize = 0x410;
    pub const ACTION_SUB: usize = 0x414;
    pub const FLAGS_418: usize = 0x418;
    pub const ABILITY: usize = 0x41C;
    pub const FLAGS_420: usize = 0x420;
    pub const REQUEST: usize = 0x424;
    pub const CARD: usize = 0x428;
    pub const DT: usize = 0x460;
    pub const STEP: usize = 0x464;
    pub const ROW_PREV: usize = 0x470;
    pub const ROW: usize = 0x474;
    pub const FACING: usize = 0x4D4;
    pub const SPEED: usize = 0x4DC;
    pub const VSPEED: usize = 0x5D4;
    pub const JUMP_FROM: usize = 0x5E0;
    pub const JUMP_TOP: usize = 0x5F4;
    pub const PHASE: usize = 0x604;
    pub const LOCK_TARGET: usize = 0x84C;
    /// Sora's parameter block (`0x1407C5B60`; `0x1401E9630` reads `+0x10` / `+0x14`, the jump init
    /// `+0x1C` / `+0x20`).
    pub const PARAMS: usize = 0x3C0;
}

/// Sora's parameter block: run m/s, jump and higher-jump speeds. Checked (exe + decomp).
mod params {
    pub const RUN: usize = 0x14;
    pub const JUMP: usize = 0x1C;
    pub const HIGH_JUMP: usize = 0x20;
    /// The jump's crouch, Re:CoM time units (int, 100; `0x1401EDA60` phase 1). Checked (exe + decomp).
    pub const CROUCH: usize = 0x24;
}
/// Re:CoM's jump take-off scale and gravity (`0x1401FAB40` writes them). Checked (decomp).
const RISE_SCALE: u64 = 0x1_4087_B39C;
const GRAVITY: u64 = 0x1_4087_B398;

// CMovement (`player+0xF0`, ctor `0x1401D4200`).
pub(super) mod body {
    pub const POS: usize = 0x30;
    pub const YAW: usize = 0x54;
    pub const VEL: usize = 0x80;
    pub const CONTACT: usize = 0x120;
    pub const COLL_PARAM: usize = 0x134;
    pub const COLL: usize = 0x158;
}

// The motion controller (`actor+0xD8`).
mod ctrl {
    pub const END: usize = 0x50;
    pub const TIME: usize = 0x3C;
    pub const BLEND_TOTAL: usize = 0x54;
    pub const BLEND_LEFT: usize = 0x58;
}

pub(super) const MOTION_REQUEST: u32 = 0x001F_7180;
const GROUND_TEST: u32 = 0x001F_3C00;
/// Sora's vtable `+0x110` (`0x14021FEF0`, slot `0x1405A9A48`): the `PL0001.SES` variant for the
/// action in `+0x638` (Re:CoM's spells: Fire / Blizzard / Thunder 0, Cure / Gravity 1, Stop 3,
/// Aero 4; -ra / -ga the same). `initMagic` passes it, with 4, as the motion request's sixth
/// argument (`0x140220B15..0x140220B6E`), and the clips Re:CoM's spells share (`p001_064` / `068`
/// Thunder, Cure, Stop, Aero; `p001_065` / `069` Blizzard, Gravity) carry one voice event per
/// variant. Checked (decomp + files).
const VOICE_VARIANT_SLOT: usize = 0x110;
const VOICE_VARIANT_FN: u32 = 0x0021_FEF0;
static VOICE_LOGGED: AtomicBool = AtomicBool::new(false);

/// The SES variant for a KH1 cast's motion (owner, 2026-11-01: "casting cure makes sora say
/// "thunder!"; we passed 0, Thunder's / Blizzard's voice on the shared clips): Re:CoM's own
/// pick, through Sora's vtable as `initMagic` calls it, on the attack id the cast wrote
/// (`+0x638`). 0 (and logged once) when the slot is not Re:CoM's function.
unsafe fn cast_voice_variant(pl: usize) -> i32 {
    type VariantFn = unsafe extern "win64" fn(usize, i32) -> i64;
    let f: usize = rd(rd::<usize>(pl) + VOICE_VARIANT_SLOT);
    if f != base() + VOICE_VARIANT_FN as usize {
        if !VOICE_LOGGED.swap(true, Ordering::Relaxed) {
            log(&format!("magic: Sora's vtable +0x110 is {f:#x}, not Re:CoM's 0x14021FEF0: cast voices stay variant 0"));
        }
        return 0;
    }
    let v = std::mem::transmute::<usize, VariantFn>(f)(pl, 4) as i32;
    if !VOICE_LOGGED.swap(true, Ordering::Relaxed) {
        log(&format!("magic: cast voice: Re:CoM's SES variant {v} for attack id {:#x} (0x14021FEF0)", rd::<u32>(pl + 0x638)));
    }
    v
}
pub(super) const COLLIDE: u32 = 0x0017_0330;
pub(super) const HANDLE_OBJ: u32 = 0x002A_D350;
const JUMP_HELD: u32 = 0x001F_F4D0;
const BUTTONS_HELD: u32 = 0x0029_C1D0;
pub(super) const FIELD_SWING: u32 = 0x001F_0F70;
const INIT_ATTACK: u32 = 0x001F_0D90;
const RELOAD_CHECK: u32 = 0x001E_6C40;
const GIMMICK_CHECK: u32 = 0x001E_80C0;
// Re:CoM's ledge checks and inits, as its jump exe calls them (`0x1401EE0E6..0x1401EE109`).
const LEDGE_CLIMB_CHECK: u32 = 0x001E_8C00;
const LEDGE_CLIMB_INIT: u32 = 0x001F_2C20;
const LEDGE_HANG_CHECK: u32 = 0x001E_8420;
const LEDGE_HANG_INIT: u32 = 0x001F_26A0;
/// `CPlayerSora`'s "draw the Keyblade" (vtable `+0xF8`; Checked (exe: `0x1405A9A30` holds it)).
const SHOW_WEAPON: u32 = 0x001F_5D30;

pub(super) type PlayerFn = unsafe extern "win64" fn(usize);
type ShowFn = unsafe extern "win64" fn(usize, i32);
type ClassFn = unsafe extern "win64" fn(usize) -> u64;
pub(super) type InitFn = unsafe extern "win64" fn(usize, i32) -> u64;
pub(super) type MotionFn = unsafe extern "win64" fn(usize, i32, i32, u64, i32, i32, i32);
type GroundFn = unsafe extern "win64" fn(usize) -> u32;
pub(super) type CollideFn = unsafe extern "win64" fn(usize, i32) -> u64;
pub(super) type HandleFn = unsafe extern "win64" fn(u32) -> usize;
type PadHeldFn = unsafe extern "win64" fn(usize) -> u32;
type HeldFn = unsafe extern "win64" fn(u32, u32) -> u8;
type GimmickFn = unsafe extern "win64" fn(usize, u64, u64, u64) -> u8;
type LedgeCheckFn = unsafe extern "win64" fn(usize, i32) -> i32;
type HangCheckFn = unsafe extern "win64" fn(usize) -> i32;
type ClimbInitFn = unsafe extern "win64" fn(usize, i32);

pub(super) unsafe fn rd<T: Copy>(at: usize) -> T {
    std::ptr::read_unaligned(at as *const T)
}

pub(super) unsafe fn wr<T: Copy>(at: usize, v: T) {
    std::ptr::write_unaligned(at as *mut T, v)
}

pub(super) unsafe fn rd_vec(at: usize) -> Vec3 {
    Vec3::new(rd(at), rd(at + 4), rd(at + 8))
}

pub(super) unsafe fn func<T: Copy>(rva: u32) -> T {
    let f = base() + rva as usize;
    std::mem::transmute_copy(&f)
}

pub(super) unsafe fn orig<T: Copy>(a: &AtomicUsize) -> T {
    let f = a.load(Ordering::Acquire);
    std::mem::transmute_copy(&f)
}

pub fn set_mode(kh1: bool) {
    KH1.store(kh1 && INSTALLED.load(Ordering::Relaxed), Ordering::Release);
}

pub fn set_abilities(a: Kh1Abilities) {
    ABILITY_BITS.store(a.bits(), Ordering::Relaxed);
    if let Ok(mut g) = ABILITIES.lock() {
        *g = Some(a);
    }
}

/// `dodge_roll` (owner, 2026-11-02; read at start): Re:CoM's own roll, KH1's, or none.
pub fn set_roll_mode(m: RollMode) {
    ROLL_MODE.store(
        match m {
            RollMode::Recom => 0,
            RollMode::Kh1 => 1,
            RollMode::Off => 2,
        },
        Ordering::Relaxed,
    );
}

pub fn set_speeds(s: crate::movement::speeds::Speeds) {
    use crate::settings::Mode::Recom;
    SPEEDS.store((s.run == Recom) as u8 | ((s.jump == Recom) as u8) << 1 | ((s.air == Recom) as u8) << 2, Ordering::Relaxed);
}

pub fn speeds() -> crate::movement::speeds::Speeds {
    use crate::settings::Mode::{Kh1, Recom};
    let b = SPEEDS.load(Ordering::Relaxed);
    let m = |bit: u8| if b & bit != 0 { Recom } else { Kh1 };
    crate::movement::speeds::Speeds { run: m(1), jump: m(2), air: m(4) }
}

pub fn roll_mode() -> RollMode {
    match ROLL_MODE.load(Ordering::Relaxed) {
        0 => RollMode::Recom,
        1 => RollMode::Kh1,
        _ => RollMode::Off,
    }
}

/// KH1's ability bits (`0x142D5EC10` layout) for the trace: the equipped abilities of the ability
/// state (M9 9b), or the ini's movement abilities while the state isn't live.
pub fn ability_bits() -> u32 {
    let b = match super::abilities::equipped_live() {
        Some(e) => crate::abilities::kh1_bits(e),
        None => ABILITY_BITS.load(Ordering::Relaxed),
    };
    // `dodge_roll = off` takes the kit's roll away (KH1's Dodge Roll bit 0x20).
    if roll_mode().on() {
        b
    } else {
        b & !0x20
    }
}

/// The abilities the KH1 frame passes on: the equipped abilities of the ability state (M9 9b,
/// `super::abilities`: movement, Combo Plus copies, the combo modifiers), or the ini's while the
/// state isn't live (no save block yet). `dodge_roll = off` takes the kit's Dodge Roll away.
pub(crate) fn abilities() -> kh1_core::Abilities {
    let mut a = match super::abilities::equipped_live() {
        Some(e) => crate::abilities::to_core(e),
        None => ABILITIES.lock().ok().and_then(|g| *g).unwrap_or_default().to_core(),
    };
    a.dodge_roll &= roll_mode().on();
    a
}

/// Re:CoM runs this player's roll now ([`crate::movement::roll::recom_owns`]).
pub(super) unsafe fn recom_roll_owns(pl: usize) -> bool {
    crate::movement::roll::recom_owns(RECOM_ROLL.load(Ordering::Relaxed), pl, rd(pl + p::ACTION), rd(pl + p::ACTION_SUB))
}

fn roll_log(line: impl FnOnce() -> String) {
    let n = ROLL_LINES.fetch_add(1, Ordering::Relaxed);
    if n < crate::movement::roll::LOG_LINES {
        log(&line());
    } else if n == crate::movement::roll::LOG_LINES {
        log("roll: further roll lines not logged");
    }
}

/// Re:CoM's roll of this player is over (its action changed, or something of ours runs Sora
/// again): its end line, once.
unsafe fn close_recom_roll(pl: usize, why: &str) {
    if RECOM_ROLL.load(Ordering::Relaxed) != pl || pl == 0 {
        return;
    }
    RECOM_ROLL.store(0, Ordering::Relaxed);
    let w = ROLL_WATCH.lock().map(|g| *g).unwrap_or_default();
    let now: u32 = rd(pl + p::ACTION);
    roll_log(|| w.end_line(now, why));
}

/// Our handler's view of the motion clock Re:CoM's exe reads (`0x1401A47C0`: `ctrl+0x3C`).
unsafe fn motion_clock(pl: usize) -> i32 {
    let c: usize = rd(pl + p::CTRL);
    if c == 0 {
        0
    } else {
        rd(c + ctrl::TIME)
    }
}

pub(super) fn kh1_on() -> bool {
    KH1.load(Ordering::Acquire)
}

/// Writes our last frame's velocity and facing again (after a Re:CoM exe that set its own).
pub(super) unsafe fn reapply(pl: usize) {
    let Ok(g) = SORA.try_lock() else { return };
    let Some(slot) = g.as_ref().filter(|s| s.player == pl) else { return };
    let step = slot.sora.last_step;
    if step <= 0.0 {
        return;
    }
    let mut h = RecomHost::new(pl);
    h.set_velocity(slot.sora.last_disp * (1.0 / step));
    h.set_facing(slot.sora.facing);
}

pub fn installed() -> bool {
    INSTALLED.load(Ordering::Acquire)
}

pub fn mark_installed() {
    INSTALLED.store(true, Ordering::Release);
}

/// The main player, if it is Sora (`CPlayerSora`; Riku keeps Re:CoM's handlers).
pub unsafe fn is_main_sora(pl: usize) -> bool {
    let b = base();
    pl != 0 && pl == rd::<usize>(va(b, data::MAIN_PLAYER)) && rd::<usize>(pl) == va(b, data::SORA_VTABLE)
}

pub(super) unsafe fn kh1_sora(pl: usize) -> bool {
    KH1.load(Ordering::Acquire) && is_main_sora(pl)
}

/// `riku_kh1_movement` (testing, `crate::riku`): KH1's movement for Riku in Reverse/Rebirth.
static RIKU: AtomicBool = AtomicBool::new(false);

pub fn set_riku(on: bool) {
    RIKU.store(on, Ordering::Release);
}

/// The main player, if it is Riku (`CPlayerRiku`, vtable `0x1405A8868`).
/// 0.152.0: Dark Riku (`CPlayerDRiku`, Dark Mode's player) counts as Riku for every KH1 rule
/// (owner, 2026-11-22: "a mesma adequação que fizemos ao Riku, precisa ser feita ao Riku Dark");
/// [`is_main_dark_riku`] tells them apart where their data differ (rows, attack ids, sleights).
pub unsafe fn is_main_riku(pl: usize) -> bool {
    let b = base();
    pl != 0 && pl == rd::<usize>(va(b, data::MAIN_PLAYER)) && (rd::<usize>(pl) == va(b, data::RIKU_VTABLE) || rd::<usize>(pl) == va(b, data::DRIKU_VTABLE))
}

/// The main player is Dark Riku.
pub unsafe fn is_main_dark_riku(pl: usize) -> bool {
    let b = base();
    pl != 0 && pl == rd::<usize>(va(b, data::MAIN_PLAYER)) && rd::<usize>(pl) == va(b, data::DRIKU_VTABLE)
}

/// `riku_kh1_movement` is on (read at start): Riku's story gets KH1's movement and Attack, the
/// empty revolver and cardless enemies (`docs/recom-riku-kh1.md`).
pub fn riku_on() -> bool {
    RIKU.load(Ordering::Acquire)
}

/// Riku with KH1's movement on (`riku_kh1_movement` at `combat = kh1`).
pub(super) unsafe fn kh1_riku(pl: usize) -> bool {
    KH1.load(Ordering::Acquire) && RIKU.load(Ordering::Acquire) && is_main_riku(pl)
}

/// KH1's Attack runs for this player (the attack hooks): Sora, or Riku with `riku_kh1_movement`.
pub(super) unsafe fn kh1_fighter(pl: usize) -> bool {
    kh1_sora(pl) || kh1_riku(pl)
}

/// The shared movement handlers (actions 2/3/4, 7 and the roll's init) run KH1's frame for this
/// player: Sora, or Riku when `riku_kh1_movement` is on. Everything else stays Sora's only.
unsafe fn kh1_mover(pl: usize) -> bool {
    kh1_sora(pl) || kh1_riku(pl)
}

/// The KH1 abilities of this player's frame: Sora's, or Riku's ([`crate::riku::abilities`]).
unsafe fn player_abilities(pl: usize) -> kh1_core::Abilities {
    if is_main_riku(pl) {
        crate::riku::abilities(rd(pl + p::ABILITY))
    } else {
        abilities()
    }
}

/// Re:CoM's pad for this frame (after the state exe's pad update).
pub unsafe fn read_pad(pl: usize) -> PadView {
    let pad = pl + p::PAD;
    let enabled = rd::<u32>(pad) & 1 != 0;
    let dev: usize = rd(va(base(), data::PAD_DEVICES));
    let (mut x, mut y) = if dev != 0 { (rd::<i32>(dev + 0x74), rd::<i32>(dev + 0x78)) } else { (0, 0) };
    if rd::<u8>(pad) & 4 != 0 {
        x = -x;
        y = -y;
    }
    let req: u32 = rd(pl + p::REQUEST);
    let jump_held: PadHeldFn = func(JUMP_HELD);
    let held: HeldFn = func(BUTTONS_HELD);
    PadView {
        enabled,
        stick_int: [x, y],
        angle: rd(pad + 4),
        world_yaw: rd(pad + 0x34),
        jump: Button { pressed: req & 1 != 0, held: jump_held(pad) != 0 },
        square: Button { pressed: req & 2 != 0, held: enabled && held(0x80, 0) != 0 },
        confirm: Button { pressed: req & 4 != 0, held: false },
        lock: lock_target(pl),
        target: if req & 4 != 0 { super::attack::attack_target(pl) } else { None },
    }
}

/// The lock-on target: in battle, with Re:CoM's lock-on on (`*(player+0x3F8)+0xE0`, toggled in
/// `0x1401E8DA0`), the target `player+0x84C`. Assumption: that pair is Re:CoM's lock-on.
unsafe fn lock_target(pl: usize) -> Option<Vec3> {
    if rd::<i32>(pl + p::BATTLE) == 0 {
        return None;
    }
    let cur: usize = rd(pl + p::TARGET_CURSOR);
    if cur == 0 || rd::<i32>(cur + 0xE0) == 0 {
        return None;
    }
    let h: u32 = rd(pl + p::LOCK_TARGET);
    if h == 0 {
        return None;
    }
    let obj = func::<HandleFn>(HANDLE_OBJ)(h);
    if obj == 0 {
        return None;
    }
    let b: usize = rd(obj + p::BODY);
    (b != 0).then(|| to_kh1(rd_vec(b + body::POS)))
}

pub(super) unsafe fn read_sync(pl: usize) -> Sync {
    Sync { action: rd(pl + p::ACTION), prev: rd(pl + p::PREV_ACTION), time: rd(pl + p::ACTION_TIME) }
}

/// Our last frame's action fields, one frame on (the exe added its delta to the action time).
pub(super) unsafe fn sync_ok(pl: usize, s: &Slot) -> bool {
    let Some(want) = s.sync else { return false };
    let now = read_sync(pl);
    s.player == pl && now.action == want.action && now.prev == want.prev && now.time == want.time.wrapping_add(rd::<i32>(pl + p::DT))
}

/// Our handler ran the last frame (read before the state exe, for the trace).
pub unsafe fn ran_last_frame(pl: usize, s: &Slot) -> bool {
    s.player == pl && s.sync == Some(read_sync(pl))
}

/// Re:CoM, as `crate::movement::Host`.
pub struct RecomHost {
    pl: usize,
}

impl RecomHost {
    pub unsafe fn new(pl: usize) -> RecomHost {
        RecomHost { pl }
    }

    fn body(&self) -> usize {
        unsafe { rd(self.pl + p::BODY) }
    }

    pub fn contact(&self) -> bool {
        unsafe { rd::<u32>(self.body() + body::CONTACT) & 5 != 0 }
    }

    pub fn view(&mut self) -> trace::RecomView {
        unsafe {
            let pl = self.pl;
            let b = self.body();
            let c: usize = rd(pl + p::CTRL);
            let (t, bt, bl) = if c != 0 { (rd(c + ctrl::TIME), rd(c + ctrl::BLEND_TOTAL), rd(c + ctrl::BLEND_LEFT)) } else { (0, 0, 0) };
            trace::RecomView {
                pos: to_kh1(rd_vec(b + body::POS)),
                facing: yaw_to_kh1(rd(b + body::YAW)),
                velocity: to_kh1(rd_vec(b + body::VEL)),
                airborne: !self.contact(),
                action: rd(pl + p::ACTION),
                sub: rd(pl + p::ACTION_SUB),
                phase: rd(pl + p::PHASE),
                row: rd(pl + p::ROW),
                motion_time: t,
                blend_total: bt,
                blend_left: bl,
                weapon_live: {
                    let w: usize = rd(pl + crate::attack::field::WEAPON);
                    w != 0 && rd::<i32>(w + 0x23C) != 0
                },
                swing_hit: rd::<i32>(pl + crate::attack::field::HIT) != 0,
                attack_id: rd(pl + crate::attack::field::ATTACK_ID),
            }
        }
    }
}

impl Host for RecomHost {
    fn step(&self) -> f32 {
        unsafe { rd(self.pl + p::STEP) }
    }

    fn pos(&mut self) -> Vec3 {
        unsafe { to_kh1(rd_vec(self.body() + body::POS)) }
    }

    fn airborne(&mut self) -> bool {
        unsafe { func::<GroundFn>(GROUND_TEST)(self.pl) != 0 }
    }

    /// Re:CoM's collision (`0x140170330` on the body's collision object, as `CMovement` uses it:
    /// segment ends at `obj[3]+0x10` / `+0x20`, hit point `+0x30`, normal `+0x40`; a floor has
    /// normal y ≥ cos 50°, `0x1401D4E00`) along a segment down from Sora.
    fn height_above_floor(&mut self, max: f32) -> f32 {
        unsafe {
            let b = self.body();
            let coll: usize = rd(b + body::COLL);
            if coll == 0 {
                return f32::INFINITY;
            }
            let seg: usize = rd(coll + 0x18);
            if seg == 0 {
                return f32::INFINITY;
            }
            let pos = rd_vec(b + body::POS);
            let lift = 0.05;
            let down = max * crate::units::METRES_PER_UNIT + 0.05;
            for (o, y) in [(0x10, pos.y + lift), (0x20, pos.y - down)] {
                wr(seg + o, pos.x);
                wr(seg + o + 4, y);
                wr(seg + o + 8, pos.z);
                wr(seg + o + 12, 1.0f32);
            }
            let hit = func::<CollideFn>(COLLIDE)(coll, rd::<i32>(b + body::COLL_PARAM)) != 0;
            let seg: usize = rd(coll + 0x18);
            if !hit || seg == 0 {
                return f32::INFINITY;
            }
            let hy: f32 = rd(seg + 0x34);
            let ny: f32 = rd(seg + 0x44);
            if ny < 0.642_787_64 {
                return f32::INFINITY;
            }
            (pos.y - hy) / crate::units::METRES_PER_UNIT
        }
    }

    fn facing(&mut self) -> f32 {
        unsafe { yaw_to_kh1(rd(self.body() + body::YAW)) }
    }

    fn set_facing(&mut self, yaw: f32) {
        unsafe {
            let r = yaw_to_recom(yaw);
            wr(self.body() + body::YAW, r);
            wr(self.pl + p::FACING, r);
        }
    }

    fn set_velocity(&mut self, v: Vec3) {
        unsafe {
            let r = to_recom(v);
            let b = self.body();
            wr(b + body::VEL, r.x);
            wr(b + body::VEL + 4, r.y);
            wr(b + body::VEL + 8, r.z);
            wr(b + body::VEL + 12, 1.0f32);
            // Re:CoM's own readers: horizontal speed per frame and vertical speed.
            wr(self.pl + p::SPEED, r.flat().length());
            wr(self.pl + p::VSPEED, r.y);
        }
    }

    fn action(&mut self) -> Action {
        unsafe { Action { id: rd(self.pl + p::ACTION), sub: rd(self.pl + p::ACTION_SUB), phase: rd(self.pl + p::PHASE) } }
    }

    fn set_action(&mut self, a: Action) {
        unsafe {
            let pl = self.pl;
            let cur: u32 = rd(pl + p::ACTION);
            let sub: u32 = rd(pl + p::ACTION_SUB);
            if cur != a.id || sub != a.sub {
                // As Re:CoM's own setters (`0x1401F8D70` and the inline 8-byte writes).
                wr(pl + p::PREV_ACTION, cur);
                wr(pl + p::ACTION, a.id);
                wr(pl + p::ACTION_TIME, 0i32);
                wr(pl + p::ACTION_SUB, a.sub);
                if a.id == crate::movement::action::JUMP && cur != a.id {
                    // As every Re:CoM jump init: a jump we start is never a spring jump.
                    wr(pl + p::FLAGS_418, rd::<u32>(pl + p::FLAGS_418) & !crate::movement::spring::FLAG_418);
                    // Re:CoM's jump init keeps the take-off point and the height gained.
                    let b = self.body();
                    for k in 0..4 {
                        wr(pl + p::JUMP_FROM + 4 * k, rd::<u32>(b + body::POS + 4 * k));
                    }
                    wr(pl + p::JUMP_TOP, 0f32);
                }
            }
            wr(pl + p::PHASE, a.phase);
        }
    }

    fn play(&mut self, request: u16, blend: f32, start: f32) -> bool {
        unsafe {
            let pl = self.pl;
            let stance = Stance { battle: rd::<u32>(pl + p::CARD) & 1 != 0, flag_418_2: rd::<u32>(pl + p::FLAGS_418) & 2 != 0, prev_row: rd(pl + p::ROW_PREV), row: rd(pl + p::ROW) };
            let ext = EXT.get().copied().unwrap_or_default();
            let Some((row, root)) = rows::row_for(request, &stance, &ext) else { return false };
            // 0.152.0: Riku's Guard plays his block-recoil row (`crate::riku::GUARD_ROW`).
            let row = if is_main_riku(pl) && request & 0x3FF == rows::GUARD { crate::riku::GUARD_ROW } else { row };
            if is_main_riku(pl) {
                // Only rows Riku's own action table plays (`crate::riku::row_ok`); Re:CoM's own
                // effects and sounds of his clips (variant 0).
                if !crate::riku::row_ok_for(is_main_dark_riku(pl), row) {
                    log_riku_row(request, row);
                    return false;
                }
                func::<MotionFn>(MOTION_REQUEST)(pl, row as i32, frames_to_units(blend), root as u64, 0, 0, frames_to_units(start));
                return true;
            }
            let voice = if crate::magic::is_cast_request(request) && super::magic::kh1_cast(pl) { cast_voice_variant(pl) } else { 0 };
            // M13 stage 3: KH1's effects / sounds of the move (the asset step's events at KH1's
            // variant), by the Effects / Sounds settings; Re:CoM's own (0) otherwise.
            let (eet, ses) = super::fx::variants(request, voice);
            func::<MotionFn>(MOTION_REQUEST)(pl, row as i32, frames_to_units(blend), root as u64, eet, ses, frames_to_units(start));
            // M9 9c.1: a KH1 special's motion starts base Re:CoM's sleight events for it.
            if let Some(e) = kh1_core::data::xl::entry_of(request) {
                super::specials::motion_events(pl, e);
            }
            true
        }
    }

    fn motion_frame(&mut self) -> f32 {
        unsafe {
            let c: usize = rd(self.pl + p::CTRL);
            if c == 0 {
                0.0
            } else {
                units_to_frames(rd(c + ctrl::TIME))
            }
        }
    }

    fn motion_ended(&mut self) -> bool {
        unsafe {
            let c: usize = rd(self.pl + p::CTRL);
            c != 0 && rd::<u16>(c + ctrl::END) & 3 != 0
        }
    }

    fn start_swing(&mut self, s: &crate::movement::SwingStart) -> bool {
        unsafe { super::attack::start_swing(self.pl, s) }
    }

    fn swing_hit(&mut self) -> bool {
        unsafe { rd::<i32>(self.pl + crate::attack::field::HIT) != 0 }
    }

    fn end_swing(&mut self, airborne: bool) -> bool {
        unsafe { super::attack::end_swing(self.pl, airborne) }
    }

    fn spring_jump(&mut self) -> bool {
        unsafe {
            let pl = self.pl;
            let ability: u32 = rd(pl + p::ABILITY);
            if !crate::movement::spring::launches(ability, || func::<GimmickFn>(GIMMICK_CHECK)(pl, crate::movement::spring::GIMMICK as u64, 0, 0)) {
                return false;
            }
            // Re:CoM's own jump init (the trampoline: no hook runs) launches the spring jump.
            let o: InitFn = orig(&ORIG_JUMP_INIT);
            o(pl, 0);
            log_once_spring();
            true
        }
    }

    fn start_recom_roll(&mut self) {
        unsafe {
            let pl = self.pl;
            let from: u32 = rd(pl + p::ACTION);
            // Re:CoM's own avoid init (the trampoline: no hook runs), as its move exe calls it.
            let o: PlayerFn = orig(&ORIG_AVOID_INIT);
            o(pl);
            let a: u32 = rd(pl + p::ACTION);
            let sub: u32 = rd(pl + p::ACTION_SUB);
            if a != crate::movement::roll::ACTION || sub != crate::movement::roll::SUB {
                // Its footing branch (`0x1401F2310(pl, 2)`) ran instead: Re:CoM's too.
                roll_log(|| format!("roll: Re:CoM's avoid init from action {from:#x} started action {a:#x} sub {sub} (its footing branch), not its roll"));
                return;
            }
            RECOM_ROLL.store(pl, Ordering::Relaxed);
            let w = crate::movement::roll::Watch::start(from);
            if let Ok(mut g) = ROLL_WATCH.lock() {
                *g = w;
            }
            let b = self.body();
            let facing: f32 = rd(b + body::YAW);
            let stick: f32 = rd(pl + 0x384);
            let bit = rd::<u32>(pl + p::FLAGS_418) & crate::movement::roll::UNTOUCHABLE != 0;
            roll_log(|| w.start_line(facing, stick, bit));
        }
    }

    fn recom_numbers(&mut self) -> crate::movement::speeds::RecomNumbers {
        let mut n = crate::movement::speeds::RecomNumbers::EXE;
        unsafe {
            let block: usize = rd(self.pl + p::PARAMS);
            if block != 0 {
                n.run_mps = rd(block + params::RUN);
                n.jump = rd(block + params::JUMP);
                n.high_jump = rd(block + params::HIGH_JUMP);
                n.crouch = rd(block + params::CROUCH);
            }
            // Enemy Skills stage 4: Larxene's Dash (battle only), Re:CoM's stick-class factor × 1.25.
            n.run_factor = crate::enemy_skills_cmp::dash_run(1.0, super::enemy_skills_cmp::dash_on(self.pl));
            let bs = base();
            let (k, g): (f32, f32) = (rd(va(bs, RISE_SCALE)), rd(va(bs, GRAVITY)));
            // Set by Re:CoM's player set-up (`0x1401FAB40`); the exe's until then.
            if k.is_finite() && k > 0.0 && g.is_finite() && g < 0.0 {
                n.rise_scale = k;
                // Wight Knight's Float (Enemy Skills): Sora's gravity × 0.75 (his own only).
                n.gravity = if is_main_riku(self.pl) { g } else { g * super::enemy_skills::gravity_factor() };
            }
        }
        n
    }

    fn set_no_flinch(&mut self, on: bool) {
        // KH1's no-flinch (damage, no reaction) for the damage-check wrapper; not Re:CoM's
        // untouchable roll bit `+0x418 & 1` (KH1's roll has no untouchable frames).
        super::attack::set_no_flinch(self.pl, on);
    }

    fn set_guard(&mut self, on: bool) {
        unsafe { super::attack::set_guard(self.pl, on) }
    }

    fn guard_blocked(&mut self) -> bool {
        unsafe { super::attack::take_guard_hit(self.pl) }
    }

    fn take_swing_clash(&mut self) -> Option<(u8, i32)> {
        super::attack::take_swing_clash(self.pl)
    }

    fn set_untouchable(&mut self, on: bool) {
        // KH1's flag 0x80 after a guard success: Re:CoM's `+0x418 & 1`, which makes the damage
        // check `0x1401E6DE0` skip hits (Checked (decomp)).
        unsafe {
            let f: u32 = rd(self.pl + p::FLAGS_418);
            wr(self.pl + p::FLAGS_418, if on { f | 1 } else { f & !1 });
        }
    }

    fn draw_weapon(&mut self) {
        unsafe {
            if is_main_riku(self.pl) {
                // His own "draw the weapon" (`CPlayerRiku` vtable `+0xF8` = `0x1402160A0`).
                let vt: usize = rd(self.pl);
                let f: ShowFn = std::mem::transmute_copy(&rd::<usize>(vt + 0xF8));
                f(self.pl, 1)
            } else {
                func::<ShowFn>(SHOW_WEAPON)(self.pl, 1)
            }
        }
    }

    fn guard_clink(&mut self) {
        unsafe { super::attack::guard_clink(self.pl) }
    }

    fn row_available(&mut self, r: usize) -> bool {
        super::attack::row_ready(r)
    }

    fn can_cast(&mut self, spell: u8, level: u8) -> bool {
        super::magic::can_cast(spell, level)
    }

    fn start_cast(&mut self, c: &crate::magic::CastStart) -> bool {
        unsafe { super::magic::start_cast(self.pl, c) }
    }

    fn release_cast(&mut self, c: &crate::magic::CastStart) -> Option<u32> {
        unsafe { super::magic::release(self.pl, c) }
    }

    fn end_cast(&mut self, airborne: bool) -> bool {
        unsafe { super::magic::end_cast(self.pl, airborne) }
    }

    fn start_item(&mut self, c: &crate::items::ItemCommand) -> bool {
        unsafe { super::items::start_item(self.pl, c) }
    }

    fn end_item(&mut self, airborne: bool) -> bool {
        unsafe { super::items::end_item(self.pl, airborne) }
    }

    fn start_special(&mut self, s: &crate::specials::SpecialStart) -> bool {
        unsafe { super::specials::start_special(self.pl, s) }
    }

    fn special_row(&mut self, r: &crate::specials::RowStart) {
        unsafe { super::specials::special_row(self.pl, r) }
    }

    fn special_row_end(&mut self) {
        unsafe { super::specials::special_row_end(self.pl) }
    }

    fn ragnarok_fire(&mut self, f: &crate::specials::ragnarok::FireStart) {
        unsafe { super::specials::ragnarok_fire(self.pl, f) }
    }

    fn end_special(&mut self, airborne: bool) -> bool {
        unsafe { super::specials::end_special(self.pl, airborne) }
    }

    fn battle_won(&mut self) -> bool {
        unsafe { rd::<u32>(self.pl + p::CARD) & crate::battle::end::WIN_PENDING != 0 }
    }
}

/// One KH1 frame for the player.
pub(super) unsafe fn run_frame(pl: usize, fresh: bool) {
    // A Re:CoM roll handed over earlier is over (our frame runs Sora again).
    close_recom_roll(pl, "a KH1 frame runs Sora again");
    let Ok(mut g) = SORA.try_lock() else { return };
    if g.as_ref().is_none_or(|s| s.player != pl) {
        *g = Some(Slot { player: pl, sora: Kh1Sora::new(Params::kh1_defaults(), player_abilities(pl)), sync: None, last: FrameOut::default() });
    }
    let Some(slot) = g.as_mut() else { return };
    if is_main_riku(pl) {
        // Riku (`crate::riku`): KH1's movement and KH1's Attack (the same picker and rows as
        // Sora's, on his own swing clips: his action table's rows 0x30..). His roll is Re:CoM's own
        // (his roll exe `0x140212350`); no Enemy Skills, menus, magic, items or specials.
        slot.sora.abilities = player_abilities(pl);
        slot.sora.params.gravity = kh1_core::params::GRAVITY;
        slot.sora.recom_roll = true;
        slot.sora.speeds = speeds();
        let input = trace::frame_input(&read_pad(pl));
        let mut host = RecomHost::new(pl);
        slot.last = slot.sora.frame(&mut host, &input, fresh);
        slot.sync = if slot.last.handed_to_recom() { None } else { Some(read_sync(pl)) };
        // 0.152.0: his Guard's start and success move his DP (perfect / missed parries).
        super::dark_points::guard_events(pl, &slot.last.events);
        log_once_riku();
        return;
    }
    // Enemy Skills: the first KH1 frame of this player applies them (one compare otherwise).
    super::enemy_skills::kh1_frame(pl);
    slot.sora.abilities = abilities();
    // Wight Knight's Float (Enemy Skills): KH1's gravity for Sora × 0.75.
    slot.sora.params.gravity = kh1_core::params::GRAVITY * super::enemy_skills::gravity_factor();
    slot.sora.recom_roll = roll_mode() == RollMode::Recom;
    slot.sora.speeds = speeds();
    let mut input = trace::frame_input(&read_pad(pl));
    // A magic command from the command menu this frame (M7 stage 7.4), aimed at Re:CoM's target.
    input.magic = super::magic::take_command();
    if input.magic.is_some() && input.target.is_none() {
        input.target = super::attack::attack_target(pl);
    }
    // An item from the command menu's Items list this frame (M8 stage 8a.1), on Sora.
    input.item = super::items::take_command();
    // A special from the command menu this frame (M9 stage 9c.1), aimed at Re:CoM's target.
    input.special = super::specials::take_command();
    if input.special.is_some() && input.target.is_none() {
        input.target = super::attack::attack_target(pl);
    }
    let mut host = RecomHost::new(pl);
    slot.last = slot.sora.frame(&mut host, &input, fresh);
    // Re:CoM's end action entered its win action, or Re:CoM's own roll started: it owns Sora; a
    // later KH1 frame takes over afresh.
    slot.sync = if slot.last.handed_to_recom() { None } else { Some(read_sync(pl)) };
    if slot.last.recom_took {
        log_once_win();
    }
    if !slot.last.cast.is_empty() {
        super::magic::on_events(&slot.last.cast);
    }
    if !slot.last.items.is_empty() {
        super::items::on_events(&slot.last.items);
    }
    if !slot.last.specials.is_empty() {
        super::specials::on_events(&slot.last.specials);
    }
}

/// Whether our last frame left this player's actions as they are now.
pub(super) unsafe fn continues(pl: usize) -> bool {
    match SORA.try_lock() {
        Ok(g) => g.as_ref().is_some_and(|s| sync_ok(pl, s)),
        Err(_) => false,
    }
}

/// An attack command dispatched by Re:CoM (its move exe or our jump exe's air commands): continues
/// our last frame when nothing else ran in between.
pub(super) unsafe fn dispatch_fresh(pl: usize) -> bool {
    if IN_MOVE_EXE.load(Ordering::Relaxed) == pl {
        MOVE_FRESH.swap(false, Ordering::Relaxed)
    } else {
        !continues(pl)
    }
}

/// The frame of an init called from inside the move exe continues it; from elsewhere (a Re:CoM
/// action cancelling into a jump or roll) it takes over.
fn init_fresh(pl: usize) -> bool {
    if IN_MOVE_EXE.load(Ordering::Relaxed) == pl {
        MOVE_FRESH.swap(false, Ordering::Relaxed)
    } else {
        true
    }
}

pub unsafe extern "win64" fn move_exe(pl: usize) {
    let o: PlayerFn = orig(&ORIG_MOVE_EXE);
    if !kh1_mover(pl) {
        return o(pl);
    }
    let fresh = !continues(pl);
    if RecomHost::new(pl).airborne() {
        // KH1's walk-off: the physics step put Sora in the air before any ground command.
        return run_frame(pl, fresh);
    }
    IN_MOVE_EXE.store(pl, Ordering::Relaxed);
    MOVE_FRESH.store(fresh, Ordering::Relaxed);
    o(pl);
    IN_MOVE_EXE.store(0, Ordering::Relaxed);
}

pub unsafe extern "win64" fn stick_class(pl: usize) -> u64 {
    let o: ClassFn = orig(&ORIG_STICK_CLASS);
    if IN_MOVE_EXE.load(Ordering::Relaxed) != pl || !kh1_mover(pl) {
        return o(pl);
    }
    static PARAMS: OnceLock<Params> = OnceLock::new();
    let pad = read_pad(pl);
    let tilt = trace::kh1_pad(&pad).tilt;
    let params = PARAMS.get_or_init(Params::kh1_defaults);
    // `run_speed = recom`: Re:CoM's run for its own stick magnitude (`crate::movement::speeds`).
    let run = if speeds().run == crate::settings::Mode::Recom {
        let n = RecomHost::new(pl).recom_numbers();
        crate::movement::speeds::recom_run(&n, crate::movement::speeds::recom_tilt(pad.stick_int[0], pad.stick_int[1]))
    } else {
        params.run_speed
    };
    // Enemy Skills stage 4: Larxene's Dash, run × 1.25 in battle (owner), as Re:CoM's stick class.
    let run = crate::enemy_skills_cmp::dash_run(run, super::enemy_skills_cmp::dash_on(pl));
    let (class, mps) = crate::movement::speed_class_run(params, tilt, run);
    wr(pl + p::SPEED, mps);
    class as u64
}

pub unsafe extern "win64" fn move_step(pl: usize) {
    let o: PlayerFn = orig(&ORIG_MOVE_STEP);
    if IN_MOVE_EXE.load(Ordering::Relaxed) != pl || !kh1_mover(pl) {
        return o(pl);
    }
    run_frame(pl, init_fresh(pl));
}

/// Re:CoM's win action and Sora's "win pending" bit (`crate::battle::end`).
/// A KH1 request with no clip in Riku's table (logged per row, first 32).
fn log_riku_row(request: u16, row: u32) {
    static SEEN: Mutex<Vec<u32>> = Mutex::new(Vec::new());
    if let Ok(mut g) = SEEN.lock() {
        if g.len() < 32 && !g.contains(&row) {
            g.push(row);
            log(&format!("riku: KH1 request {request:#x} wants row {row:#x}, which Riku's action table has no clip for: the current motion stays"));
        }
    }
}

pub(super) unsafe fn in_recom_win(pl: usize) -> bool {
    rd::<u32>(pl + p::ACTION) == crate::battle::end::WIN_ACTION
}

fn log_once_riku() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| log("riku: KH1 movement runs on Riku (riku_kh1_movement = on): his own Re:CoM animations, KH1's Attack, Re:CoM's roll"));
}

fn log_once_win() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| log("movement: Re:CoM's end action entered its win action (0x1c) as a KH1 swing or cast ended; the KH1 frame yielded to it"));
}

fn log_once_spring() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| log("movement: a Re:CoM spring launched Sora; Re:CoM's own jump runs until it lands"));
}

pub unsafe extern "win64" fn jump_init(pl: usize, falling: i32) {
    let o: InitFn = orig(&ORIG_JUMP_INIT);
    if !kh1_mover(pl) {
        o(pl, falling);
        return;
    }
    // Re:CoM's jump init clears the spring flag first; ours does the same (a spring launch below
    // runs Re:CoM's own init, which sets it again).
    wr(pl + p::FLAGS_418, rd::<u32>(pl + p::FLAGS_418) & !crate::movement::spring::FLAG_418);
    if falling != 0 {
        if let Ok(mut g) = SORA.try_lock() {
            if g.as_ref().is_none_or(|s| s.player != pl) {
                *g = Some(Slot { player: pl, sora: Kh1Sora::new(Params::kh1_defaults(), player_abilities(pl)), sync: None, last: FrameOut::default() });
            }
            if let Some(slot) = g.as_mut() {
                slot.sora.enter_fall(&mut RecomHost::new(pl));
                // The caller (`0x1401F1F20`) goes on in this frame; the next frame continues.
                slot.sync = Some(read_sync(pl));
            }
        }
        return;
    }
    run_frame(pl, init_fresh(pl));
}

pub unsafe extern "win64" fn avoid_init(pl: usize) {
    let o: PlayerFn = orig(&ORIG_AVOID_INIT);
    // Re:CoM's footing special case (`+0x420 & 0x10400000 == 0x10000000`) stays Re:CoM's.
    if !kh1_mover(pl) || rd::<u32>(pl + p::FLAGS_420) & 0x1040_0000 == 0x1000_0000 {
        return o(pl);
    }
    run_frame(pl, init_fresh(pl));
}

/// Re:CoM's jump-exe commands while rising or falling (`0x1401EDA60` after its phase checks):
/// reload, magic card (battle), attack card (battle) or field swing. True when one started.
unsafe fn air_command(pl: usize) -> bool {
    let req: u32 = rd(pl + p::REQUEST);
    let battle = rd::<i32>(pl + p::BATTLE) != 0;
    if rd::<u32>(pl + p::ABILITY) & 0x200_0000 != 0 && req & 0x10 != 0 {
        func::<InitFn>(RELOAD_CHECK)(pl, 0);
    }
    if req & 0x20 != 0 && battle {
        let vt: usize = rd(pl);
        let f: InitFn = std::mem::transmute_copy(&rd::<usize>(vt + data::VT_INIT_MAGIC));
        f(pl, 0);
        return true;
    }
    if req & 4 != 0 {
        if battle {
            func::<InitFn>(INIT_ATTACK)(pl, 0);
        } else {
            func::<InitFn>(FIELD_SWING)(pl, 0);
        }
        return true;
    }
    false
}

pub unsafe extern "win64" fn jump_exe(pl: usize) {
    let o: PlayerFn = orig(&ORIG_JUMP_EXE);
    if !kh1_mover(pl) || crate::movement::spring::recom_owns_jump(rd(pl + p::ACTION), rd(pl + p::FLAGS_418)) {
        // Re:CoM's spring jump: its own flight and landing (`docs/recom-kh1-movement.md` § Springs).
        return o(pl);
    }
    let fresh = !continues(pl);
    let ph: u32 = rd(pl + p::PHASE);
    if !fresh && matches!(ph, crate::movement::phase::RISE | crate::movement::phase::FALL) && air_command(pl) {
        return;
    }
    run_frame(pl, fresh);
    air_ledge(pl);
}

/// Re:CoM's own ledge grab, after our KH1 air frame, exactly where its jump exe runs it: in the
/// air (rising, or falling without landing this frame), after the air commands and the velocity.
/// The wall climb `0x1401E8C00(pl, 1)`; above 1 it starts the climb `0x1401F2C20(pl, n)`. Else
/// the hang check `0x1401E8420`; when it passes, the hang init `0x1401F26A0` (the same writes and
/// calls as the jump exe's inline hang block, `0x1401EE10F..`). Re:CoM's hang, climb and drop
/// then run as Re:CoM's actions 9 / 0x10 (decision 16). Checked (disasm + decomp).
unsafe fn air_ledge(pl: usize) {
    use crate::movement::{action, phase};
    if rd::<u32>(pl + p::ACTION) != action::JUMP {
        return;
    }
    let ph: u32 = rd(pl + p::PHASE);
    if ph != phase::RISE && ph != phase::FALL {
        return;
    }
    // Re:CoM's jump exe lands instead when falling (`0x1401F0920` = 2) on the ground.
    if ph == phase::FALL && func::<GroundFn>(GROUND_TEST)(pl) == 0 {
        return;
    }
    let n = func::<LedgeCheckFn>(LEDGE_CLIMB_CHECK)(pl, 1);
    if n > 1 {
        func::<ClimbInitFn>(LEDGE_CLIMB_INIT)(pl, n);
        log_once_ledge("climb");
    } else if func::<HangCheckFn>(LEDGE_HANG_CHECK)(pl) != 0 {
        func::<PlayerFn>(LEDGE_HANG_INIT)(pl);
        log_once_ledge("hang");
    }
}

fn log_once_ledge(what: &str) {
    static CLIMB: std::sync::Once = std::sync::Once::new();
    static HANG: std::sync::Once = std::sync::Once::new();
    let once = if what == "climb" { &CLIMB } else { &HANG };
    once.call_once(|| log(&format!("movement: Re:CoM's ledge {what} started from a KH1 jump")));
}

pub unsafe extern "win64" fn avoid_exe(pl: usize) {
    let o: PlayerFn = orig(&ORIG_AVOID_EXE);
    if !kh1_sora(pl) {
        return o(pl);
    }
    avoid_kh1(pl, o)
}

/// 0.153.0 (fork owner, 2026-11-22: "still the guard is not effective"): KH1's Guard / its recoil put
/// Riku in Re:CoM's action 6, whose exe is **his own** (`CPlayerRiku +0x1C8` = `0x140212350`; Dark
/// Riku's `0x1401FFC30`), not Sora's `0x140217310` we wrap: so the KH1 frame (the guard object, the
/// block, the recoil) never ran while he guarded. Checked (exe: both vtables' slot, the first bytes).
/// Now his own exes wrap the same way: his Re:CoM roll (handed over by KH1's rule) runs his exe whole,
/// anything else of ours in action 6 runs the KH1 frame.
pub unsafe extern "win64" fn riku_avoid_exe(pl: usize) {
    let o: PlayerFn = orig(&ORIG_RIKU_AVOID_EXE);
    if !kh1_riku(pl) {
        return o(pl);
    }
    log_once_riku_avoid();
    avoid_kh1(pl, o)
}

/// Dark Riku's own avoid exe ([`riku_avoid_exe`]).
pub unsafe extern "win64" fn driku_avoid_exe(pl: usize) {
    let o: PlayerFn = orig(&ORIG_DRIKU_AVOID_EXE);
    if !kh1_riku(pl) {
        return o(pl);
    }
    log_once_riku_avoid();
    avoid_kh1(pl, o)
}

fn log_once_riku_avoid() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| log("riku guard: his own avoid exe (action 6) runs KH1's frame for the Guard and its recoil (his Re:CoM roll stays his exe)"));
}

/// Action 6 for a KH1 player: Re:CoM's own roll when KH1's rule handed it over, else the KH1 frame.
unsafe fn avoid_kh1(pl: usize, o: PlayerFn) {
    if recom_roll_owns(pl) {
        // `dodge_roll = recom`: Re:CoM's own roll, its exe whole (owner, 2026-11-02, "The whole
        // roll"). Only read around it, for the start / end log lines.
        let bit = |pl: usize| rd::<u32>(pl + p::FLAGS_418) & crate::movement::roll::UNTOUCHABLE != 0;
        let (before, clock, step) = (bit(pl), motion_clock(pl), rd::<f32>(pl + p::STEP));
        o(pl);
        let after = bit(pl);
        if let Ok(mut g) = ROLL_WATCH.lock() {
            g.exe(step, before, after, clock);
        }
        if !recom_roll_owns(pl) {
            let a: u32 = rd(pl + p::ACTION);
            let why = match a {
                crate::movement::action::STAND | crate::movement::action::WALK | crate::movement::action::RUN => "Re:CoM's exe: clip end or its moving cancel, into stand / walk / run",
                crate::movement::action::JUMP => "Re:CoM's exe: clip end in the air, into its fall",
                _ => "Re:CoM's exe started another action",
            };
            close_recom_roll(pl, why);
        }
        return;
    }
    // A roll handed over earlier that Re:CoM ended without its exe (a hit after its untouchable
    // frames, an event): logged now.
    close_recom_roll(pl, "Re:CoM ran another action");
    // As Re:CoM's avoid and glide exes do first.
    wr(pl + p::CARD, rd::<u32>(pl + p::CARD) & !8);
    run_frame(pl, !continues(pl));
}

pub unsafe extern "win64" fn glide_exe(pl: usize) {
    let o: PlayerFn = orig(&ORIG_GLIDE_EXE);
    if !kh1_sora(pl) {
        return o(pl);
    }
    wr(pl + p::CARD, rd::<u32>(pl + p::CARD) & !8);
    run_frame(pl, !continues(pl));
}

/// Appends the KH1-only rows (Guard; High Jump uses Re:CoM's rows 0x14 / 0x15) to Sora's action table: a copy of
/// Re:CoM's 0x47 rows plus ours, then `0x1407C6300[1]` and `[2]` repointed to it (checked old
/// values). Re:CoM resolves the names itself at the next player set-up (`0x1401FA6E0`). Data only.
pub unsafe fn extend_action_table(ext: &Extension, names: &[(u32, String)]) -> bool {
    let b = base();
    let table = va(b, data::SORA_TABLE);
    let ptrs = va(b, data::CHAR_TABLES);
    if names.is_empty() {
        return true;
    }
    if rd::<usize>(ptrs + 8) != table || rd::<usize>(ptrs + 16) != table || rd::<usize>(table + rows::RECOM_ROWS as usize * 16) != 0 {
        log("movement: Sora's action table is not as expected; KH1-only clips not added");
        return false;
    }
    let n = rows::RECOM_ROWS as usize + names.len() + 1;
    let new: &'static mut [u8] = Box::leak(vec![0u8; n * 16].into_boxed_slice());
    std::ptr::copy_nonoverlapping(table as *const u8, new.as_mut_ptr(), rows::RECOM_ROWS as usize * 16);
    for (row, name) in names {
        let c: &'static [u8] = Box::leak(format!("{name}\0").into_bytes().into_boxed_slice());
        let o = *row as usize * 16;
        new[o..o + 8].copy_from_slice(&(c.as_ptr() as usize as u64).to_le_bytes());
        new[o + 8..o + 12].copy_from_slice(&(-1i32).to_le_bytes());
    }
    let to = (new.as_ptr() as usize as u64).to_le_bytes();
    let from = (table as u64).to_le_bytes();
    let ok = super::hook::patch(ptrs + 8, &from, &to, "action table [1]") && super::hook::patch(ptrs + 16, &from, &to, "action table [2]");
    if ok {
        log(&format!("movement: action table extended with {} KH1 rows: {names:?} (Guard as available: {:?})", names.len(), ext.rows));
    }
    ok
}
