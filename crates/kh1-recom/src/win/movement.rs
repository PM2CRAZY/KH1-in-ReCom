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
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

pub static ORIG_MOVE_EXE: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_STICK_CLASS: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_MOVE_STEP: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_JUMP_INIT: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_AVOID_INIT: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_JUMP_EXE: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_AVOID_EXE: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_GLIDE_EXE: AtomicUsize = AtomicUsize::new(0);

/// `combat = kh1`, and the hooks are in.
static KH1: AtomicBool = AtomicBool::new(false);
static INSTALLED: AtomicBool = AtomicBool::new(false);
/// The abilities (`Kh1Abilities::bits`), set from the ini, and all of them.
static ABILITY_BITS: AtomicU32 = AtomicU32::new(0x30);
static ABILITIES: Mutex<Option<Kh1Abilities>> = Mutex::new(None);
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
}

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
const COLLIDE: u32 = 0x0017_0330;
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

pub(super) type PlayerFn = unsafe extern "win64" fn(usize);
type ClassFn = unsafe extern "win64" fn(usize) -> u64;
pub(super) type InitFn = unsafe extern "win64" fn(usize, i32) -> u64;
pub(super) type MotionFn = unsafe extern "win64" fn(usize, i32, i32, u64, i32, i32, i32);
type GroundFn = unsafe extern "win64" fn(usize) -> u32;
type CollideFn = unsafe extern "win64" fn(usize, i32) -> u64;
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

pub fn ability_bits() -> u32 {
    ABILITY_BITS.load(Ordering::Relaxed)
}

pub(crate) fn abilities() -> kh1_core::Abilities {
    ABILITIES.lock().ok().and_then(|g| *g).unwrap_or_default().to_core()
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
unsafe fn sync_ok(pl: usize, s: &Slot) -> bool {
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
            func::<MotionFn>(MOTION_REQUEST)(pl, row as i32, frames_to_units(blend), root as u64, 0, 0, frames_to_units(start));
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

    fn end_swing(&mut self, airborne: bool) {
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

    fn set_untouchable(&mut self, on: bool) {
        // KH1's flag 0x80 after a guard success: Re:CoM's `+0x418 & 1`, which makes the damage
        // check `0x1401E6DE0` skip hits (Checked (decomp)).
        unsafe {
            let f: u32 = rd(self.pl + p::FLAGS_418);
            wr(self.pl + p::FLAGS_418, if on { f | 1 } else { f & !1 });
        }
    }

    fn guard_clink(&mut self) {
        unsafe { super::attack::guard_clink(self.pl) }
    }

    fn row_available(&mut self, r: usize) -> bool {
        super::attack::row_ready(r)
    }
}

/// One KH1 frame for the player.
pub(super) unsafe fn run_frame(pl: usize, fresh: bool) {
    let Ok(mut g) = SORA.try_lock() else { return };
    if g.as_ref().is_none_or(|s| s.player != pl) {
        *g = Some(Slot { player: pl, sora: Kh1Sora::new(Params::kh1_defaults(), abilities()), sync: None, last: FrameOut::default() });
    }
    let Some(slot) = g.as_mut() else { return };
    slot.sora.abilities = abilities();
    let input = trace::frame_input(&read_pad(pl));
    let mut host = RecomHost::new(pl);
    slot.last = slot.sora.frame(&mut host, &input, fresh);
    slot.sync = Some(read_sync(pl));
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
    if !kh1_sora(pl) {
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
    if IN_MOVE_EXE.load(Ordering::Relaxed) != pl || !kh1_sora(pl) {
        return o(pl);
    }
    static PARAMS: OnceLock<Params> = OnceLock::new();
    let tilt = trace::kh1_pad(&read_pad(pl)).tilt;
    let (class, mps) = crate::movement::speed_class(PARAMS.get_or_init(Params::kh1_defaults), tilt);
    wr(pl + p::SPEED, mps);
    class as u64
}

pub unsafe extern "win64" fn move_step(pl: usize) {
    let o: PlayerFn = orig(&ORIG_MOVE_STEP);
    if IN_MOVE_EXE.load(Ordering::Relaxed) != pl || !kh1_sora(pl) {
        return o(pl);
    }
    run_frame(pl, init_fresh(pl));
}

fn log_once_spring() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| log("movement: a Re:CoM spring launched Sora; Re:CoM's own jump runs until it lands"));
}

pub unsafe extern "win64" fn jump_init(pl: usize, falling: i32) {
    let o: InitFn = orig(&ORIG_JUMP_INIT);
    if !kh1_sora(pl) {
        o(pl, falling);
        return;
    }
    // Re:CoM's jump init clears the spring flag first; ours does the same (a spring launch below
    // runs Re:CoM's own init, which sets it again).
    wr(pl + p::FLAGS_418, rd::<u32>(pl + p::FLAGS_418) & !crate::movement::spring::FLAG_418);
    if falling != 0 {
        if let Ok(mut g) = SORA.try_lock() {
            if g.as_ref().is_none_or(|s| s.player != pl) {
                *g = Some(Slot { player: pl, sora: Kh1Sora::new(Params::kh1_defaults(), abilities()), sync: None, last: FrameOut::default() });
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
    if !kh1_sora(pl) || rd::<u32>(pl + p::FLAGS_420) & 0x1040_0000 == 0x1000_0000 {
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
    if !kh1_sora(pl) || crate::movement::spring::recom_owns_jump(rd(pl + p::ACTION), rd(pl + p::FLAGS_418)) {
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
