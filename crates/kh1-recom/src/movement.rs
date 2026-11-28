//! KH1 Sora's movement as handlers of Re:CoM's own player actions (roadmap M4,
//! `docs/recom-kh1-movement.md`).
//!
//! Re:CoM owns Sora: its Field / Battle state exes read the pad and build the request bits, its
//! action switch (`player+0x40C`) calls the action's exe, its motion system plays, advances and
//! ends the clips, and its mover (`CMovement`, `player+0xF0`) moves and collides the body. With
//! `combat = kh1`, the exes of the movement actions call [`Kh1Sora::frame`] instead of Re:CoM's
//! logic for that action:
//!
//! | Re:CoM action (`+0x40C`) | phase `+0x604` | KH1 state |
//! |---|---|---|
//! | 2 / 3 / 4 stand / walk / run (move exe `0x1401EEDA0`, through its move step `0x1401EF3C0`) | 0 | 0, free |
//! | 6 avoid (`CPlayerSora` exe `0x140217310`), `+0x414` = 0 roll / 1 guard | 3 / 0 | 0, locked by the roll / Guard |
//! | 7 jump (`0x1401EDA60`) | 1 crouch, 2 rise, 3 fall, 6 land | 1, 2, 2, 0 locked by the land |
//! | 8 glide (`CPlayerSora` exe `0x140217B50`) | 4 | 0x15 |
//!
//! At `dodge_roll = recom` (the default) KH1's rule only asks for the roll: Re:CoM's own roll then
//! runs action 6 sub 0 whole, from its avoid init to its exe's end ([`roll`]).
//!
//! [`Kh1Sora::frame`] is kh1-core's `Controller::step` (the oracle) for these states, with KH1's
//! numbers. Its control step also runs KH1's Attack command (M5, `crate::attack`): Re:CoM's attack
//! action `0xA` plays the swing. What it reads and writes goes through [`Host`]: Re:CoM's body
//! position, its ground test (`0x1401F3C00`), its facing (`body+0x54`), its velocity (`body+0x80`,
//! which Re:CoM's mover then integrates and collides), its action fields, and Re:CoM's motion
//! request (`0x1401F7180`, rows from [`crate::rows`]) and motion clock (`actor+0xD8`).
//!
//! KH1's fields Re:CoM has no place for (the air arc, the jump-cut and glide timers, the target
//! yaw, the KH1 request queue) are kept in [`Kh1Sora`], one per player: they are new fields of
//! Re:CoM's player, not a second controller (Re:CoM's action decides which handler runs, and
//! Re:CoM's motion and mover do the work).
//!
//! **Frame order.** KH1's frame is think → physics (move, next state) → motion update. Re:CoM's is
//! state exe (our handler) → motion advance + root motion → mover. So the mover's result for frame
//! N (landing, walking off) and KH1's motion update for frame N are seen at the start of frame
//! N + 1's handler, before its think: [`Kh1Sora::frame`] runs them there, in KH1's order, which
//! gives the same state for the think. Checked (decomp, both games); see the doc for the two
//! one-frame differences that remain (root motion, queued motions).

use kh1_core::arc::Arc;
use kh1_core::controller::{state, Button, Event};
use kh1_core::math::{clamp_sym, turn_toward, wrap_angle};
use kh1_core::motion::{flag, req, request, MotionInfo, MotionSet};
use kh1_core::{Abilities, Moveset, Params, Vec3};
use crate::settings::Mode;

/// Re:CoM action ids (`player+0x40C`). Checked (decomp, both switches).
pub mod action {
    pub const STAND: u32 = 2;
    pub const WALK: u32 = 3;
    pub const RUN: u32 = 4;
    pub const AVOID: u32 = 6;
    pub const JUMP: u32 = 7;
    pub const GLIDE: u32 = 8;
    /// The field swing, KH1's Attack in the field and in battle (M5, `crate::attack`).
    pub const ATTACK: u32 = 0xA;
    /// `player+0x414` within action 6.
    pub const SUB_ROLL: u32 = 0;
    pub const SUB_GUARD: u32 = 1;
}

/// `player+0x604` as Re:CoM's own handlers set it (read by `0x1401F0920`: 1/2/5 rising, 3/4
/// falling, 6 landing). Checked (decomp).
pub mod phase {
    pub const NONE: u32 = 0;
    pub const CROUCH: u32 = 1;
    pub const RISE: u32 = 2;
    pub const FALL: u32 = 3;
    pub const GLIDE: u32 = 4;
    pub const LAND: u32 = 6;
}

/// KH1 applies a motion's root travel in the next frame's physics; Re:CoM applies it in the motion
/// update of the same frame (`0x1401A59A0` → `0x14024DCA0`). A wrapper on `0x14024DCA0` (which
/// hands the update's local root travel `ctrl+0xA0` to the entity task) gives Sora's controller the
/// previous update's travel instead (decision 17).
///
/// KH1 applies the last update's travel even on the frame a new motion starts (its physics runs
/// before the update that starts the new motion; kh1-core `MotionPlayer::update`), so the travel
/// carries across a motion change. It is dropped when an update passed without a root step (Re:CoM
/// calls `0x14024DCA0` only for root-motion clips): `tick` counts Sora's frames.
/// [`RootDelay::position`]'s safety net in Re:CoM metres: above every KH1 root travel of a frame
/// (Sonic Blade's dash and finisher: 0.61 m a frame, M9 stage 9c.1; Sora's own motions at most
/// 0.18 m). At 0.5 (before 9c.1) the net let Re:CoM's own same-update travel through on the dash's
/// fastest frames, so one frame's travel was applied twice (the test caught 623 for KH1's 562).
pub const ROOT_DELAY_MAX_GAP: f32 = 1.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RootDelay {
    ctrl: usize,
    tick: u64,
    pending: Vec3,
}

impl RootDelay {
    pub const fn new() -> RootDelay {
        RootDelay { ctrl: 0, tick: 0, pending: Vec3::ZERO }
    }

    /// Re:CoM's root step hands the entity task an **absolute** position: `ctrl+0xA0` = the
    /// model's position `ctrl+0xF0` (the anchor) + this update's rotated root travel
    /// (`0x14024C960`), and the task sets the body to `P + mask∘(out − P)` (`0x1401A59A0`:
    /// `0x14023E6E0` a − b, `0x14023E630` a ∘ b, `0x14023E0D0` a + b). Checked (decomp). The
    /// delayed position is the anchor plus the last update's travel: with nothing pending it is
    /// the anchor itself (no travel), never a jump to the world origin.
    pub fn position(&mut self, ctrl: usize, tick: u64, anchor: Vec3, root: Vec3, max_gap: f32) -> Vec3 {
        let out = anchor + self.swap(ctrl, tick, root - anchor);
        // Safety net: the delayed position never differs from Re:CoM's own by more than
        // `max_gap` (more than any frame's root travel); otherwise Re:CoM's own stands.
        if (out - root).length() > max_gap {
            root
        } else {
            out
        }
    }

    /// This update's travel in, the travel to apply now out.
    pub fn swap(&mut self, ctrl: usize, tick: u64, now: Vec3) -> Vec3 {
        if self.ctrl != ctrl || tick != self.tick.wrapping_add(1) {
            self.pending = Vec3::ZERO;
        }
        self.ctrl = ctrl;
        self.tick = tick;
        std::mem::replace(&mut self.pending, now)
    }
}

/// Re:CoM's springs (jump pads, gimmick object type `0x2002`; `docs/recom-kh1-movement.md`
/// § Springs). KH1 has no such gimmick, so Re:CoM's own spring jump runs whole: its jump init
/// `0x1401F29F0` launches Sora at the spring's speed (`player+0x600`, from `0x1405A0100[level]`,
/// set by the spring through `0x1401F5370`), with row 0x14 and `player+0x418 |= 0x1000000`, and its
/// jump exe `0x1401EDA60` flies (no jump cut) and lands. Checked (decomp).
pub mod spring {
    /// `player+0x41C`: the ability bit Re:CoM's jump init tests before it looks for a spring.
    pub const ABILITY: u32 = 0x2_0000;
    /// The gimmick type `0x1401E80C0` is asked for, and its answer when Sora stands on one (the
    /// body's contact bit 4, `body+0x120 & 4`).
    pub const GIMMICK: u32 = 0x2002;
    pub const ON_SPRING: u8 = 7;
    /// `player+0x418`: set by Re:CoM's jump init for a spring jump, cleared by every jump init,
    /// read only by the jump exe (its row-0x14 branch: no jump cut). Checked (decomp: the only
    /// writers are `0x1401F29F0`'s two lines; the only reader is `0x1401EDA60`).
    pub const FLAG_418: u32 = 0x100_0000;

    /// Re:CoM's own test in its jump init (`0x1401F29F0`): `gimmick` runs `0x1401E80C0(player,
    /// 0x2002)` and is only asked when the ability bit is on, as Re:CoM does.
    pub fn launches(ability_41c: u32, gimmick: impl FnOnce() -> u8) -> bool {
        ability_41c & ABILITY != 0 && gimmick() == ON_SPRING
    }

    /// Re:CoM owns this jump-action frame: a spring jump is flying or landing.
    pub fn recom_owns_jump(action: u32, flags_418: u32) -> bool {
        action == super::action::JUMP && flags_418 & FLAG_418 != 0
    }
}

pub mod roll;
pub mod speeds;

/// The Re:CoM action that stands for a KH1 state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Action {
    pub id: u32,
    /// `player+0x414`.
    pub sub: u32,
    /// `player+0x604`.
    pub phase: u32,
}

/// One frame's input, in kh1-core's terms (the host builds it from Re:CoM's pad).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameInput {
    /// World move direction (unit or zero) and KH1 tilt (KH1's dead zone, `stick_from_raw`).
    pub move_dir: Vec3,
    pub tilt: f32,
    /// Re:CoM's own stick magnitude (`player+0x384`, the pad wrapper's `|(x, y)| / 4096`, at most
    /// 1): what `run_speed = recom` scales Re:CoM's run by ([`speeds`]).
    pub recom_tilt: f32,
    pub jump: Button,
    pub square: Button,
    /// The lock-on target's position, KH1 units.
    pub lock_target: Option<Vec3>,
    /// The Attack command (Re:CoM's confirm press, `player+0x424 & 4`).
    pub attack: Button,
    /// Re:CoM's attack target (`player+0x84C`) as a KH1 body part, and whether it is the lock-on.
    pub target: Option<kh1_core::TargetPart>,
    pub locked: bool,
    /// A KH1 magic command this frame (the command menu's, M7 stage 7.4; a shortcut's in 7.12).
    pub magic: Option<crate::magic::MagicCommand>,
    /// A KH1 item command this frame (the command menu's Items list, M8 stage 8a.1).
    pub item: Option<crate::items::ItemCommand>,
    /// A KH1 special command this frame (the context tab's Triangle, M9 stage 9c.1): Sonic Blade,
    /// Rave, Blast (`crate::specials::cmd`).
    pub special: Option<u8>,
}

/// A KH1 swing for Re:CoM to start (`crate::attack`): Re:CoM's field-swing init with the KH1 row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SwingStart {
    /// KH1's row (btltbl Sora rows) and the Re:CoM attack id whose clip is the same motion.
    pub kh1_row: usize,
    pub recom_id: u32,
    /// KH1's motion request (200..=206).
    pub request: u16,
    /// Played with root motion.
    pub root: bool,
    /// KH1's combo counter after this press, and the combo length.
    pub counter: u8,
    pub length: u8,
    pub finisher: bool,
}

/// What the handlers use of Re:CoM. KH1 units and yaw (see [`crate::units`]).
pub trait Host {
    /// Re:CoM's movement step (`player+0x464`), KH1's frame step.
    fn step(&self) -> f32;
    /// Body position (`body+0x30`).
    fn pos(&mut self) -> Vec3;
    /// Re:CoM's ground test after the last move (`0x1401F3C00` ≠ 0).
    fn airborne(&mut self) -> bool;
    /// Height above the floor below, probed by Re:CoM's collision; more than `max` when there is no
    /// floor within `max`.
    fn height_above_floor(&mut self, max: f32) -> f32;
    /// Facing (`body+0x54`).
    fn facing(&mut self) -> f32;
    fn set_facing(&mut self, yaw: f32);
    /// This frame's velocity (`body+0x80`, a displacement per 60 Hz frame).
    fn set_velocity(&mut self, v: Vec3);
    fn action(&mut self) -> Action;
    fn set_action(&mut self, a: Action);
    /// Re:CoM plays the clip for a KH1 request (`0x1401F7180`), `blend` and `start` in KH1 frames.
    /// Returns false when no Re:CoM row plays that request (the record then keeps KH1's clock).
    fn play(&mut self, request: u16, blend: f32, start: f32) -> bool;
    /// Re:CoM's motion clock in KH1 frames, and whether its clip has ended (`ctrl+0x50 & 3`).
    fn motion_frame(&mut self) -> f32;
    fn motion_ended(&mut self) -> bool;
    /// Re:CoM's own roll protection (`player+0x418` bit 0) carries KH1's no-flinch window.
    fn set_no_flinch(&mut self, on: bool);
    /// Re:CoM's run, jump and gravity numbers as it holds them now ([`speeds::RecomNumbers`]).
    fn recom_numbers(&mut self) -> speeds::RecomNumbers {
        speeds::RecomNumbers::EXE
    }
    /// Re:CoM's spring (jump-pad gimmick, object type `0x2002`): when Re:CoM's own jump init
    /// would launch Sora from a spring here (`player+0x41C & 0x20000` and `0x1401E80C0(player,
    /// 0x2002) == 7`), the host runs Re:CoM's own jump init and returns true; Re:CoM then owns
    /// the whole jump until it lands (`docs/recom-kh1-movement.md` § Springs).
    fn spring_jump(&mut self) -> bool {
        false
    }
    /// Re:CoM's own Dodge Roll ([`roll`], `dodge_roll = recom`): its avoid init `0x1401F10C0`
    /// through the trampoline. Re:CoM then owns Sora (its action 6 and exe) until its action
    /// changes; the next KH1 frame takes over afresh.
    fn start_recom_roll(&mut self) {}
    /// Re:CoM starts the swing: its field-swing init `0x1401F0F70`, whose row pick (Sora's vtable
    /// `+0x128`) takes this KH1 row. True when Re:CoM plays its clip.
    fn start_swing(&mut self, _s: &SwingStart) -> bool {
        false
    }
    /// Sora's body guard (Re:CoM's `body+0x9C & 1`, ±100°) on or off: KH1's guard object is live.
    fn set_guard(&mut self, _on: bool) {}
    /// Re:CoM's apply guarded a hit on Sora's body since the last frame (`body+0x9C & 0x20`).
    fn guard_blocked(&mut self) -> bool {
        false
    }
    /// 0.183.1: the guard side's KH1 clash record of that block (kind 6 / 7, the guard's priority − the
    /// attack's: `0x1402B4BC0`), when the attack was a KH1 enemy's; None: kind 6 with no push (Re:CoM's
    /// attacks have no priority, decision 20).
    fn guard_clash(&mut self) -> Option<(u8, i32)> {
        None
    }
    /// M11, the shared enemy-guard step: Sora's KH1 swing met a KH1 enemy's guard row since the last
    /// frame (Re:CoM's apply guarded it on the enemy's body): KH1's clash kind for Sora (6 / 7) and
    /// his swing's priority − the row's.
    fn take_swing_clash(&mut self) -> Option<(u8, i32)> {
        None
    }
    /// No damage and no reaction (KH1's flag 0x80 after a guard success; Re:CoM `+0x418 & 1`).
    fn set_untouchable(&mut self, _on: bool) {}
    /// Re:CoM's guard spark and its clink at the guarded point (once per guard success).
    fn guard_clink(&mut self) {}
    /// Re:CoM's own "draw the Keyblade" (Sora's vtable `+0xF8` = `0x1401F5D30` with 1, as the
    /// swing init `0x1401F0F70` calls it): the `kb_appear` effect when it was put away. Re:CoM's
    /// own rules put it away again (state terms, carry, climb).
    fn draw_weapon(&mut self) {}
    /// Whether Re:CoM can play this KH1 row (its attack row exists).
    fn row_available(&mut self, _kh1_row: usize) -> bool {
        true
    }
    /// The current swing has hit (`player+0x644`, set by `procAttackCollision`).
    fn swing_hit(&mut self) -> bool {
        false
    }
    /// Re:CoM ends its attack action (`0x1401F6660`): the weapon's volume, its effects, its fields.
    /// True when Re:CoM's end action entered an action of its own instead of the idle / fall: its
    /// win action `0x1C` (the battle is won: `player+0x428 & 4`, `0x1401F6794` →
    /// `0x1401F1B20`). The KH1 frame then yields to it ([`Kh1Sora::frame`]).
    fn end_swing(&mut self, _airborne: bool) -> bool {
        false
    }
    /// KH1's usable test for a spell (`0x14028EFD0`: learned, MP for its cost, `0x1402A4C10`).
    fn can_cast(&mut self, _spell: u8, _level: u8) -> bool {
        false
    }
    /// Re:CoM's magic action starts for this KH1 cast (`crate::magic`): its fields as `initMagic`
    /// sets them, cardless, with the KH1 row. False when it can't (no rows).
    fn start_cast(&mut self, _c: &crate::magic::CastStart) -> bool {
        false
    }
    /// KH1's release frame: Re:CoM's shot runtime fires the spell from the KH1 row and KH1's MP is
    /// paid. Re:CoM's shot id, if one was made.
    fn release_cast(&mut self, _c: &crate::magic::CastStart) -> Option<u32> {
        None
    }
    /// Re:CoM ends its magic action (`0x1401F6660`, as `exeMagic` does). True as for
    /// [`Host::end_swing`]: Re:CoM entered its win action, the KH1 frame yields.
    fn end_cast(&mut self, _airborne: bool) -> bool {
        false
    }
    /// M8 stage 8a.1: Re:CoM's item action starts for this KH1 item use (`crate::items`): the item
    /// leaves its slot (`0x1402C2760`), KH1's use task starts, and Sora's fields are `initItem`'s,
    /// cardless. False when the slot doesn't hold the item (nothing changes).
    fn start_item(&mut self, _c: &crate::items::ItemCommand) -> bool {
        false
    }
    /// Re:CoM ends its item action (`0x1401F6660`, as `exeItem` does). True as for
    /// [`Host::end_swing`].
    fn end_item(&mut self, _airborne: bool) -> bool {
        false
    }
    /// Re:CoM's battle is won: the last enemy is gone and its win waits for Sora's action to end
    /// (`player+0x428 & 4`, set by `0x1401F4A60`). Re:CoM's card-use test `0x1401E9180` refuses
    /// every card from then on; the KH1 frame takes no new command either. Checked (decomp).
    fn battle_won(&mut self) -> bool {
        false
    }
    /// M9 stage 9c.1: Re:CoM's sleight action `0x16` starts for a KH1 special: `initStockSkill`'s
    /// writes, cardless; KH1's MP paid (the press frame). False when it can't (no rows, no MP).
    fn start_special(&mut self, _s: &crate::specials::SpecialStart) -> bool {
        false
    }
    /// A special row starts on Re:CoM's sleight action: the last row's volume ends, the KH1 row in
    /// the attack row pointer, Re:CoM's attack id of it.
    fn special_row(&mut self, _r: &crate::specials::RowStart) {}
    /// M12 step 7: Ragnarok's release (`0x14039F330`): its shots from the Keyblade's tip.
    fn ragnarok_fire(&mut self, _f: &crate::specials::ragnarok::FireStart) {}
    /// M12 step 7: a special row's attack is over (Strike Raid's throw ended into its catch): the
    /// weapon's volume ends and the attack row pointer holds no window, so the catch and the wait
    /// hit nothing (KH1's attack object goes with the throw's motion).
    fn special_row_end(&mut self) {}
    /// Re:CoM ends its sleight action (`0x1401F6660`). True as for [`Host::end_swing`].
    fn end_special(&mut self, _airborne: bool) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Queued {
    request: u32,
    start: f32,
    blend: f32,
}

/// KH1's motion request record (actor +0x15C..), kept on top of Re:CoM's motion: what KH1 asked
/// for, its queue, and its flags. Re:CoM's clip and clock are the motion itself.
#[derive(Clone, Debug, PartialEq)]
pub struct MotionRec {
    /// The KH1 request playing (0xFFFF: unknown, after a Re:CoM action).
    pub req: u16,
    /// Its KH1 frame as this frame's think sees it.
    pub frame: f32,
    pub flags: u32,
    /// KH1's blend length and clock (glide motions only switch once the blend is done).
    pub blend: f32,
    pub blend_clock: f32,
    pub(crate) start_now: bool,
    queue: Vec<Queued>,
    /// The pending head has been sent to Re:CoM, and whether a Re:CoM row plays it.
    pub(crate) issued: bool,
    pub(crate) issued_ok: bool,
    /// No Re:CoM clip plays the current request: its KH1 frame is counted here instead.
    own_clock: bool,
}

impl MotionRec {
    fn unknown() -> MotionRec {
        MotionRec { req: 0xFFFF, frame: 0.0, flags: 0, blend: 0.0, blend_clock: 0.0, start_now: false, queue: Vec::new(), issued: false, issued_ok: false, own_clock: false }
    }

    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    /// The request that will be playing after this frame.
    pub fn head(&self) -> u16 {
        if self.start_now {
            self.queue.first().map_or(self.req, |q| (q.request & req::ID_MASK) as u16)
        } else {
            self.req
        }
    }
}

/// What a frame did (for the trace).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrameOut {
    pub displacement: Vec3,
    pub events: Vec<Event>,
    /// The height probed this frame, if any.
    pub height: Option<f32>,
    /// A jump press launched Re:CoM's own spring jump: Re:CoM owns Sora until it lands, and this
    /// frame wrote nothing to Re:CoM after the launch.
    pub recom_spring: bool,
    /// What the cast did (M7 stage 7.4).
    pub cast: Vec<crate::magic::CastEvent>,
    /// What the item command did (M8 stage 8a.1).
    pub items: Vec<crate::items::ItemEvent>,
    /// What the special did (M9 stage 9c.1).
    pub specials: Vec<crate::specials::SpecialEvent>,
    /// Re:CoM's end action entered its win action (`0x1C`) in this frame: the KH1 frame wrote
    /// nothing to Re:CoM after it (no action, no motion), and the next one takes over afresh.
    pub recom_took: bool,
    /// KH1's rule asked for the Dodge Roll and Re:CoM's own roll started ([`roll`]): Re:CoM owns
    /// Sora from its avoid init on, and the next KH1 frame takes over afresh.
    pub recom_roll: bool,
}

impl FrameOut {
    /// Re:CoM took Sora in this frame (its win action or its roll): the next KH1 frame starts from
    /// Re:CoM's state, not from ours.
    pub fn handed_to_recom(&self) -> bool {
        self.recom_took || self.recom_roll
    }
}

/// KH1 Sora's fields Re:CoM has no place for, and the KH1 data the handlers use.
#[derive(Clone, Debug)]
pub struct Kh1Sora {
    pub params: Params,
    pub abilities: Abilities,
    pub motions: MotionSet,
    pub state: u32,
    pub prev_state: u32,
    state_changed: bool,
    pub lock: bool,
    jump_held: bool,
    cut_ok: bool,
    pub arc: Arc,
    pub jump_offset: u16,
    pub glide_timer: f32,
    pub superglide: bool,
    glide_intended: f32,
    pub no_flinch: f32,
    pub no_flinch_on: bool,
    no_flinch_counting: bool,
    no_flinch_reported: bool,
    /// KH1 facing (kept here so it does not drift through Re:CoM's yaw convention; re-read when
    /// Re:CoM turned Sora itself) and target yaw.
    pub facing: f32,
    pub target_yaw: f32,
    pub move_dir: Vec3,
    pub speed: f32,
    pub air_vel: Vec3,
    pub rec: MotionRec,
    /// The last frame's displacement, step and start position (the mover's inputs).
    pub last_disp: Vec3,
    pub last_step: f32,
    last_start: Vec3,
    /// The last height probe (KH1 +0x3D4), when one ran.
    pub height: f32,
    /// Sora's rows and attack records (the Attack, M5).
    pub moveset: Moveset,
    /// KH1's combo counter (CB+0x41).
    pub combo_counter: u8,
    /// The swing Re:CoM is playing for us (its attack action `0xA`), if any.
    pub swing: Option<SwingStart>,
    /// KH1's random float (the finisher choice).
    pub(crate) rng: kh1_core::Kh1Rand,
    /// The guard object is live and Re:CoM's body guard is on (reported to the host).
    pub guard_on: bool,
    /// CB+0x50's clash bit (0x10): set by a guard success, cleared at each row start.
    pub clashed: bool,
    /// KH1's flag 0x80 after a guard success: on, its 8-frame countdown running, frames left.
    pub untouchable: bool,
    untouchable_counting: bool,
    untouchable_left: f32,
    untouchable_reported: bool,
    /// The KH1 cast Re:CoM's magic action is playing for us (M7 stage 7.4), if any.
    pub cast: Option<crate::magic::Cast>,
    pub(crate) cast_log: Vec<crate::magic::CastEvent>,
    /// The KH1 item use whose motion Re:CoM's item action is playing for us (M8 stage 8a.1), if
    /// any. KH1's use task itself runs on its own (`win::items`), as KH1's does.
    pub item: Option<crate::items::ItemCommand>,
    pub(crate) item_log: Vec<crate::items::ItemEvent>,
    /// The KH1 special running on Re:CoM's sleight action (M9 stage 9c.1): KH1's state 0x20.
    pub special: Option<crate::specials::Special>,
    pub(crate) special_log: Vec<crate::specials::SpecialEvent>,
    /// M12 step 7: this frame's Re:CoM attack target, for a special's callbacks (Strike Raid's
    /// first throw aims from its wind-up's end).
    pub(crate) special_target: Option<kh1_core::TargetPart>,
    /// Re:CoM's end action (from [`Kh1Sora::leave_swing`] / `leave_cast`) entered its own win
    /// action: this frame yields to it.
    pub(crate) recom_took: bool,
    /// `dodge_roll = recom` (the default; the host sets it): KH1's rule asks for the roll, Re:CoM's
    /// own roll runs ([`roll`]). False: KH1's roll on Re:CoM's action 6 (`dodge_roll = kh1`).
    pub recom_roll: bool,
    /// KH1's push slide (`0x1402C7300` → `0x1402C7070`: direction, speed; × 0.8 a frame, ends under
    /// 1): a ground kind-6 clash's push back (M11, the shared enemy-guard step).
    pub push: Option<(Vec3, f32)>,
    /// KH1's rule asked for the roll this frame; Re:CoM's starts after the frame's commands, if
    /// none replaced it (as a command replaces KH1's roll).
    recom_roll_wanted: bool,
    /// `run_speed`, `jump_height`, `air_speed` (the host sets them; all KH1's by default) and
    /// Re:CoM's numbers they use, read from Re:CoM each frame ([`speeds`]).
    pub speeds: speeds::Speeds,
    pub recom: speeds::RecomNumbers,
    /// `jump_height = recom`: Re:CoM's vertical jump in place of KH1's arc, from the take-off
    /// until anything sets a new arc (a hop, a walk-off, a landing, a take-over).
    pub recom_rise: Option<speeds::RecomRise>,
    /// `jump_height = recom`: Re:CoM's crouch in place of KH1's 4 frames, from the jump's start to its
    /// take-off (owner 2026-11-02, "Pace / movement follow-ups").
    pub recom_crouch: Option<speeds::RecomCrouch>,
    /// This frame's Re:CoM stick magnitude and jump button (held).
    recom_tilt: f32,
    jump_down: bool,
    /// The take-off of a jump happened in this frame's state input (`air_speed = recom` keeps 0.8 of
    /// the ground speed then, `0x1401F29F0`).
    took_off: bool,
    /// Riku's own moves on KH1's rules (fork owner, 2026-11-27): `Some(dark)` while the player is Riku
    /// (Dark Riku in Dark Mode); his swing pick and swing timings ([`Kh1Sora::set_riku_moves`]).
    pub riku_moves: Option<bool>,
    /// Re:CoM's situation of Riku's last swing (`+0x634`), 0 on a new combo.
    pub riku_prev_sit: u8,
    /// Sora's own motion entries of the swing requests, put back when Riku's are taken off.
    sora_swing_entries: Vec<(u16, Option<usize>)>,
}

/// btltbl's globals the player's clash reactions read (Checked (files): `G+0x1C` 2.5, `G+0x30` 40,
/// `G+0x34` 8; `0x14029D720`, `0x14029D510`) and the air recoil's hop (`0x1402AF960(actor, 20)`).
pub const CLASH_PUSH: f32 = 2.5;
pub const CLASH_AIR_BACK: f32 = 40.0;
pub const CLASH_AIR_BACK_MAX: f32 = 8.0;
pub const CLASH_AIR_HOP: f32 = 20.0;

/// The facing Re:CoM holds may differ from ours by rounding; more than this means Re:CoM turned
/// Sora (an event, a Re:CoM action) and its value wins.
const FACING_SYNC: f32 = 1.0e-3;
/// How far down the glide checks look (Superglide's low height, the largest height KH1 compares).
const PROBE: f32 = 70.0;

impl Kh1Sora {
    pub fn new(params: Params, abilities: Abilities) -> Kh1Sora {
        let mut motions = Moveset::kh1_defaults().motions;
        // KH1's cast motions (M7 stage 7.4), from the player's mset.
        crate::magic::add_motions(&mut motions);
        // KH1's item use motion (M8 stage 8a.1).
        crate::items::add_motions(&mut motions);
        // KH1's specials' motions (M9 stage 9c.1), keyed by xl entry.
        crate::specials::add_motions(&mut motions);
        Kh1Sora {
            params,
            abilities,
            motions,
            state: state::GROUND,
            prev_state: state::GROUND,
            state_changed: false,
            lock: false,
            jump_held: false,
            cut_ok: false,
            arc: Arc::default(),
            jump_offset: 0,
            glide_timer: 0.0,
            superglide: false,
            glide_intended: 0.0,
            no_flinch: 0.0,
            no_flinch_on: false,
            no_flinch_counting: false,
            no_flinch_reported: false,
            facing: 0.0,
            target_yaw: 0.0,
            move_dir: Vec3::ZERO,
            speed: 0.0,
            air_vel: Vec3::ZERO,
            rec: MotionRec::unknown(),
            last_disp: Vec3::ZERO,
            last_step: 0.0,
            last_start: Vec3::ZERO,
            height: f32::NAN,
            moveset: Moveset::kh1_defaults(),
            combo_counter: 1,
            swing: None,
            rng: kh1_core::Kh1Rand::default(),
            guard_on: false,
            clashed: false,
            untouchable: false,
            untouchable_counting: false,
            untouchable_left: 0.0,
            untouchable_reported: false,
            cast: None,
            cast_log: Vec::new(),
            item: None,
            item_log: Vec::new(),
            special: None,
            special_log: Vec::new(),
            special_target: None,
            recom_took: false,
            recom_roll: false,
            recom_roll_wanted: false,
            push: None,
            speeds: speeds::Speeds::default(),
            recom: speeds::RecomNumbers::EXE,
            recom_rise: None,
            recom_crouch: None,
            recom_tilt: 0.0,
            jump_down: false,
            took_off: false,
            riku_moves: None,
            riku_prev_sit: 0,
            sora_swing_entries: Vec::new(),
        }
    }

    /// Riku's (`Some(dark)`) or Sora's (`None`) swing timings in the motion set: Riku's clip lengths,
    /// hit windows and Re:CoM's chain window (`crate::riku::swing_motions`) on the requests 200..=206.
    pub fn set_riku_moves(&mut self, moves: Option<bool>) {
        if self.riku_moves == moves {
            return;
        }
        if self.sora_swing_entries.is_empty() {
            self.sora_swing_entries = (200u16..=206).chain([crate::rows::GUARD]).map(|r| (r, self.motions.entry(r))).collect();
        }
        match moves {
            Some(dark) => {
                let base = Moveset::kh1_defaults().motions;
                for (r, info) in crate::riku::swing_motions(dark, &base) {
                    self.motions.add(&[r], info);
                }
                if let Some(g) = crate::riku::guard_motion(&base) {
                    self.motions.add(&[crate::rows::GUARD], g);
                }
            }
            None => {
                for &(r, e) in &self.sora_swing_entries {
                    if let Some(e) = e {
                        self.motions.map_request(r, e);
                    }
                }
            }
        }
        self.riku_moves = moves;
        self.riku_prev_sit = 0;
    }

    fn info(&self, r: u16) -> Option<&MotionInfo> {
        self.motions.info(r)
    }

    /// Takes over from whatever Re:CoM did last (first frame, or after a Re:CoM action such as a
    /// card, a hit or an event): KH1's state from Re:CoM's.
    pub fn resync(&mut self, h: &mut dyn Host, airborne: bool) {
        self.push = None;
        let f = h.facing();
        self.facing = f;
        self.target_yaw = f;
        self.move_dir = Vec3::from_yaw(f);
        self.speed = 0.0;
        let pos = h.pos();
        self.state = if airborne { state::AIR } else { state::GROUND };
        self.prev_state = self.state;
        self.state_changed = false;
        self.lock = false;
        self.jump_held = false;
        self.cut_ok = false;
        self.jump_offset = 0;
        self.arc = Arc::timed(0.0, pos.y, self.params.gravity);
        self.recom_rise = None;
        self.air_vel = Vec3::ZERO;
        self.recom_roll_wanted = false;
        self.no_flinch_on = false;
        self.no_flinch_counting = false;
        if self.no_flinch_reported {
            self.no_flinch_reported = false;
            h.set_no_flinch(false);
        }
        self.rec = MotionRec::unknown();
        self.last_disp = Vec3::ZERO;
        self.last_step = 0.0;
        self.last_start = pos;
        // Whatever Re:CoM ran in between ended any swing of ours, and any cast: a cast cut before
        // its release casts nothing and pays no MP (KH1's script ends at its release check). An item
        // use's motion is over too (its use task goes on by itself, as KH1's).
        self.swing = None;
        self.item = None;
        // A special Re:CoM interrupted (an event, a Re:CoM action) is over; its MP stays paid, as
        // KH1's (paid on the press).
        if let Some(s) = self.special.take() {
            self.special_log.push(crate::specials::SpecialEvent::Ended { why: "Re:CoM ran another action", frame: s.age });
        }
        if let Some(c) = self.cast.take() {
            if !c.released && !c.cancelled {
                if c.start.spell == crate::magic::spell::AERO {
                    // KH1's Aero (M7 stage 7.10) casts at once when its motion is cut.
                    let shot = h.release_cast(&c.start);
                    self.cast_log.push(crate::magic::CastEvent::Released { spell: c.start.spell, level: c.start.level, frame: c.age, shot });
                } else {
                    self.cast_log.push(crate::magic::CastEvent::Interrupted { spell: c.start.spell, level: c.start.level, frame: c.age, by: 0xFFFF });
                }
            }
        }
    }

    /// Re:CoM started a fall on its own (`0x1401F29F0(player, 1)`, e.g. the end of a knock-back in
    /// the air): KH1 falls from here. The Re:CoM action becomes 7 / falling.
    pub fn enter_fall(&mut self, h: &mut dyn Host) {
        if self.swing.take().is_some() && self.state == state::AIR {
            // Re:CoM ended an air swing before KH1's motion did: KH1's arc goes on into the
            // swing's queued fall.
            self.lock = false;
            self.rec = MotionRec::unknown();
            self.rec.req = request::FALL;
            h.set_action(self.recom_action());
            return;
        }
        self.resync(h, true);
        self.rec.req = request::FALL;
        h.set_action(self.recom_action());
    }

    /// KH1's state as a Re:CoM action.
    pub fn recom_action(&self) -> Action {
        use action::*;
        if let Some(a) = self.swing_action() {
            return a;
        }
        if let Some(a) = self.cast_action() {
            return a;
        }
        if let Some(a) = self.item_action() {
            return a;
        }
        if let Some(a) = self.special_action() {
            return a;
        }
        let a = |id, sub, phase| Action { id, sub, phase };
        match self.state {
            state::TAKE_OFF => a(JUMP, 0, phase::CROUCH),
            state::AIR => a(JUMP, 0, if self.arc.rising() { phase::RISE } else { phase::FALL }),
            state::GLIDE => a(GLIDE, 0, phase::GLIDE),
            _ if self.lock => match self.rec.head() {
                request::DODGE_ROLL => a(AVOID, SUB_ROLL, phase::FALL),
                crate::rows::GUARD | crate::rows::RECOIL | crate::rows::STRONG_RECOIL => a(AVOID, SUB_GUARD, phase::NONE),
                _ => a(JUMP, 0, phase::LAND),
            },
            _ => {
                let id = if self.params.run_speed <= self.speed {
                    RUN
                } else if 0.0 < self.speed {
                    WALK
                } else {
                    STAND
                };
                a(id, 0, phase::NONE)
            }
        }
    }

    // ---- KH1's motion requests, issued to Re:CoM at the end of the frame ----

    /// `0x14029F980`.
    pub(crate) fn play(&mut self, r: u32, blend: f32) {
        self.rec.queue.clear();
        self.rec.queue.push(Queued { request: r & !req::ACTION, start: 0.0, blend });
        self.rec.start_now = true;
        self.rec.issued = false;
        if r & req::ACTION != 0 {
            self.lock = true;
        }
    }

    /// `0x14029FA30`.
    pub(crate) fn append(&mut self, r: u32, blend: f32) {
        self.rec.queue.push(Queued { request: r & !req::ACTION, start: 0.0, blend });
        if r & req::ACTION != 0 {
            self.lock = true;
        }
    }

    /// `0x14029F7C0` (no request when the entry already plays). Re:CoM's walk / run clips are its
    /// own, so KH1's phase-keeping start is not passed on (Re:CoM starts them at 0, as it does).
    fn locomotion(&mut self, r: u32, blend: f32) {
        let id = (r & req::ID_MASK) as u16;
        let entry = self.motions.entry(id);
        if entry.is_some() && entry == self.motions.entry(self.rec.req) {
            return;
        }
        self.play(r, blend);
    }

    fn set_last_start(&mut self, start: f32) {
        if let Some(q) = self.rec.queue.last_mut() {
            q.start = start;
        }
    }

    fn start_head(&mut self, dt: f32) -> bool {
        if self.rec.queue.is_empty() {
            return false;
        }
        let q = self.rec.queue.remove(0);
        self.rec.req = (q.request & req::ID_MASK) as u16;
        self.rec.frame = q.start;
        if q.blend <= dt {
            self.rec.blend = 0.0;
            self.rec.blend_clock = 0.0;
        } else {
            self.rec.blend = q.blend;
            self.rec.blend_clock = dt;
        }
        q.request & req::RELEASE != 0
    }

    /// KH1's motion update of the last frame (`0x14029F0A0`), from Re:CoM's clock: what started,
    /// the flags, the lock release. `dt` is the last frame's step, `step` this one's.
    fn motion_update(&mut self, h: &mut dyn Host, dt: f32, step: f32) {
        let mut release = false;
        if self.rec.start_now {
            // Re:CoM started our request in the last frame's update (its clock does not move on
            // the first update, like KH1's). A request made after that frame's think (in the
            // physics step: a glide's end) reaches Re:CoM only now, one step on.
            self.rec.start_now = false;
            let late = !self.rec.issued;
            if late {
                if let Some(q) = self.rec.queue.first().copied() {
                    self.rec.issued_ok = h.play((q.request & req::ID_MASK) as u16, q.blend, q.start + step);
                }
            }
            self.rec.own_clock = !self.rec.issued_ok;
            release = self.start_head(dt);
        } else if self.rec.blend > 0.0 {
            self.rec.blend_clock += dt;
            if self.rec.blend_clock - self.rec.blend >= 0.0 {
                self.rec.blend = 0.0;
                self.rec.blend_clock = 0.0;
            }
            self.rec.frame = self.clock(h, dt);
        } else {
            self.rec.frame = self.clock(h, dt);
            let len = self.info(self.rec.req).map(|m| m.length);
            let ended = match len {
                Some(l) => l <= self.rec.frame || (!self.rec.own_clock && h.motion_ended()),
                // Assumption (kh1-core): a request with no KH1 entry ends at once.
                None => true,
            };
            if ended && !self.rec.queue.is_empty() {
                let over = len.map_or(0.0, |l| (self.rec.frame - l).max(0.0));
                self.rec.queue[0].start += over;
                let ended_req = self.rec.req;
                release = self.start_head(dt);
                // A special's motion ended into its end callback (M9 stage 9c.1): after the
                // wind-up the first dash takes the motion (the lock stays); after a row the special
                // ends and its queued idle plays.
                if kh1_core::data::xl::entry_of(ended_req).is_some() && self.special_motion_end(h, ended_req) {
                    self.rec.flags = 0;
                    return;
                }
                // A swing's motion ended into its queued idle / fall: Re:CoM ends its attack; a
                // cast's: Re:CoM ends its magic action.
                self.leave_swing(h);
                self.leave_cast(h);
                self.leave_item(h);
                if self.recom_took {
                    // Re:CoM's end action entered its win action: its motion, not our idle.
                    return;
                }
                // KH1 started it in that update; Re:CoM starts it now, one step on.
                let (r, b, s) = (self.rec.req, self.rec.blend, self.rec.frame);
                self.rec.own_clock = !h.play(r, b, s + step);
            }
        }
        self.rec.flags = self.info(self.rec.req).map_or(0, |m| m.flags_at(self.rec.frame));
        if release || self.rec.flags & flag::FREE != 0 {
            self.lock = false;
        }
    }

    /// The playing request's KH1 frame: Re:CoM's clock, or ours when no Re:CoM clip plays it.
    fn clock(&mut self, h: &mut dyn Host, dt: f32) -> f32 {
        if self.rec.own_clock {
            self.rec.frame + dt
        } else {
            h.motion_frame()
        }
    }

    /// Sends the pending head to Re:CoM (end of the frame's think).
    fn issue(&mut self, h: &mut dyn Host) {
        if self.rec.start_now && !self.rec.issued {
            if let Some(q) = self.rec.queue.first().copied() {
                self.rec.issued_ok = h.play((q.request & req::ID_MASK) as u16, q.blend, q.start);
                self.rec.issued = true;
            }
        }
    }

    pub(crate) fn set_state(&mut self, new: u32) -> bool {
        if new == self.state {
            return false;
        }
        self.state_changed = true;
        self.prev_state = self.state;
        if self.state == state::GLIDE {
            self.glide_timer = self.params.glide.exit_timer;
        }
        self.state = new;
        if new == state::GLIDE {
            self.glide_timer = self.params.glide.enter_timer;
        }
        true
    }

    fn reach(&self) -> f32 {
        self.params.reach(&self.abilities)
    }

    /// This frame's push slide (`0x1402C7070`: `dir × step × v`, then `v ×= 0.8^step`, ended under 1).
    fn push_step(&mut self, step: f32) -> Vec3 {
        let Some((dir, v)) = self.push.as_mut() else { return Vec3::ZERO };
        let d = *dir * (step * *v);
        *v *= 0.8f32.powf(step);
        if *v < 1.0 {
            self.push = None;
        }
        d
    }

    pub(crate) fn hop(&mut self, h: f32, y: f32) {
        self.arc = Arc::hop(h, y, self.params.gravity);
        self.recom_rise = None;
        self.set_state(state::AIR);
    }

    /// `0x1402A5F10`. `ground`: Sora's ground speed for the stick (the run per `run_speed`); else
    /// KH1's (the air at `air_speed = kh1`).
    fn speed_map(&self, dir: &mut Vec3, w: &mut f32, ground: bool) {
        if *w <= 0.0 {
            if dir.is_zero_xz() {
                *dir = self.move_dir;
            }
            dir.y = 0.0;
            *w = 0.0;
        } else if *w <= self.params.run_tilt {
            *w = self.params.walk_speed;
        } else {
            *w = if ground { self.run_speed() } else { self.params.run_speed };
        }
    }

    /// Sora's run, KH1 units a frame: KH1's 8, or at `run_speed = recom` Re:CoM's `magnitude × 6.2`
    /// m/s (`0x1401E9630`), always above KH1's 8 where KH1's snap runs ([`speeds`]' tests).
    /// Enemy Skills (Larxene's Dash): × Re:CoM's stick-class factor (`speeds::RecomNumbers::run_factor`).
    pub fn run_speed(&self) -> f32 {
        let run = match self.speeds.run {
            Mode::Recom => speeds::recom_run(&self.recom, self.recom_tilt),
            Mode::Kh1 => self.params.run_speed,
        };
        run * self.recom.run_factor
    }

    /// The air at `air_speed = recom` while Sora is free in it (not in an attack or another action).
    fn recom_air_now(&self) -> bool {
        self.state == state::AIR && !self.lock && self.speeds.air == Mode::Recom
    }

    pub fn speed_class(&self, tilt: f32) -> (u32, f32) {
        speed_class_run(&self.params, tilt, self.run_speed())
    }

    /// One 60 Hz frame of KH1 for the movement actions. `fresh`: Re:CoM ran something else last
    /// frame (take over its state first).
    pub fn frame(&mut self, h: &mut dyn Host, input: &FrameInput, fresh: bool) -> FrameOut {
        let step = h.step();
        let mut out = FrameOut::default();
        if step <= 0.0 {
            h.set_velocity(Vec3::ZERO);
            return out;
        }
        let pos = h.pos();
        let airborne = h.airborne();
        self.recom = h.recom_numbers();
        self.recom_tilt = input.recom_tilt;
        self.jump_down = input.jump.held;
        self.took_off = false;
        self.special_target = input.target;
        if fresh {
            self.resync(h, airborne);
        } else {
            self.special_tick(if self.last_step > 0.0 { self.last_step } else { step });
            // Re:CoM turned Sora itself (rare): its facing wins.
            let f = h.facing();
            if wrap_angle(f - self.facing).abs() > FACING_SYNC {
                self.facing = f;
            }
            // ---- the last frame's physics: next state from Re:CoM's mover ----
            self.next_state(h, airborne, pos, &mut out.events);
            // ---- the last frame's motion update ----
            let dt = if self.last_step > 0.0 { self.last_step } else { step };
            self.motion_update(h, dt, step);
            // KH1's spell script of the last frame (M7 stage 7.4): its task runs after the actors'
            // update (priority 137000, `0x14028F6B0`), so it sees that frame's motion update.
            self.cast_step(h, dt);
        }
        if self.yield_to_recom(&mut out) {
            return out;
        }
        let ev = &mut out.events;
        // A guarded hit since the last frame (Re:CoM's apply on Sora's body guard).
        if self.guard_on && h.guard_blocked() {
            self.guard_success(h, ev);
        }
        // M11, the shared enemy-guard step: the swing met a KH1 enemy's guard row last frame (Re:CoM's
        // apply guarded it on the enemy's body): KH1's clash on Sora (`0x1402B4BC0` → his kind 6 / 7).
        if let Some((kind, diff)) = h.take_swing_clash() {
            self.swing_clash(h, kind, diff, ev);
            if self.yield_to_recom(&mut out) {
                return out;
            }
        }
        let ev = &mut out.events;
        // Re:CoM's battle is won (`player+0x428 & 4`): Re:CoM starts no card play from then on (its
        // card-use test `0x1401E9180` answers no), so no KH1 command starts either; what is
        // running ends as it does and Re:CoM's end action enters the win ([`Host::battle_won`]).
        let won = h.battle_won();
        // Command accept (KH1's menu task, before Sora's think). The Counterattack replaces the
        // Attack command while it is offered (`0x140294220`).
        let counter_cmd = !won && input.attack.pressed && self.counter_offered(input.target.is_some()) && h.row_available(crate::attack::COUNTERATTACK_ROW);
        let attack_cmd = !won && input.attack.pressed && !counter_cmd && self.accepts_attack();
        if input.attack.pressed && !attack_cmd && !counter_cmd {
            ev.push(Event::AttackRefused(kh1_core::Refused::NotAccepted));
        }
        let why = if won { "battle won" } else { "not now" };
        // A magic command (M7 stage 7.4): the same accept (`0x140293480`), with the Fire re-cast.
        let cast_cmd = input.magic.filter(|m| !won && self.accepts_spell(m.spell));
        if let Some(m) = input.magic.filter(|_| cast_cmd.is_none()) {
            self.cast_log.push(crate::magic::CastEvent::Refused { spell: m.spell, level: m.level, why });
        }
        // An item command (M8 stage 8a.1): the same accept (`0x140293480`, type 3).
        let item_cmd = input.item.filter(|_| !won && self.accepts_item());
        if let Some(i) = input.item.filter(|_| item_cmd.is_none()) {
            self.item_log.push(crate::items::ItemEvent::Refused { item: i.item, why });
        }
        // A special command (M9 stage 9c.1): the context tab's Triangle (its own accept inside).
        let special_cmd = input.special.filter(|_| !won);
        if let Some(c) = input.special.filter(|_| won) {
            self.special_log.push(crate::specials::SpecialEvent::Refused { command: c, why });
        }

        // ---- think: pre-step ----
        self.speed = 0.0;
        self.glide_timer = dec_to_zero(self.glide_timer, step);
        let mut dir = self.move_dir;
        let mut w = self.speed;
        if self.state_changed {
            match self.state {
                state::GROUND if matches!(self.prev_state, 2 | 3 | 0x10 | 0x15) => {
                    self.locomotion((self.jump_offset as u32 + request::LAND as u32) | req::ACTION, 0.0);
                    self.append(req::RELEASE | request::IDLE as u32, self.params.blend.land_to_idle);
                    dir = Vec3::from_yaw(self.facing);
                    w = 0.0;
                    ev.push(Event::Landed);
                }
                state::AIR => self.locomotion(request::FALL as u32, self.params.blend.fall),
                _ => {}
            }
            self.state_changed = false;
        }

        // ---- control step ----
        if !self.lock {
            dir = input.move_dir;
            w = input.tilt;
            self.pad_handler(h, input, &mut dir, &mut w, pos, step, ev, &mut out.height, &mut out.recom_spring);
        }
        if !input.jump.pressed {
            if let Some(c) = special_cmd {
                self.command_special(h, c, input.target, pos, &mut dir, &mut w);
            } else if counter_cmd {
                self.command_counterattack(h, &mut dir, &mut w, input.target, pos, ev);
            } else if attack_cmd {
                self.command_attack(h, &mut dir, &mut w, input.target, input.locked, pos, ev);
            } else if let Some(m) = cast_cmd {
                self.command_magic(h, &mut dir, &mut w, input.target, pos, m);
            } else if let Some(i) = item_cmd {
                self.command_item(h, &mut dir, &mut w, i);
            }
        } else {
            if attack_cmd {
                ev.push(Event::AttackRefused(kh1_core::Refused::Jump));
            }
            let r = self.rec.req;
            if (r == request::LAND || r == request::HIGH_LAND) && self.state == state::GROUND && self.lock {
                // `0x1402A6220`: a jump during the land lock: unlock and jump at t = 2.
                self.lock = false;
                self.jump_offset = 0;
                if self.jump_start(h, ev) {
                    self.set_last_start(self.params.land_jump_t);
                    self.arc.t = self.params.land_jump_t;
                } else {
                    out.recom_spring = true;
                }
            }
        }

        if out.recom_spring {
            // Re:CoM's spring jump took over (its init set the action, motion and launch speed).
            self.rec = MotionRec::unknown();
            return out;
        }
        if std::mem::take(&mut self.recom_roll_wanted) && self.rec.head() == request::DODGE_ROLL {
            // No command of this frame replaced the roll: Re:CoM's own roll from here.
            self.hand_roll_to_recom(h, &mut out);
            return out;
        }

        // ---- state input on the flattened move vector ----
        let flat = Vec3::new(dir.x, 0.0, dir.z).normalized();
        let mut fdir = Vec3::new(flat.x, dir.y, flat.z);
        let mut fw = w;
        match self.state {
            state::GROUND => self.ground_input(fdir, fw),
            state::TAKE_OFF => self.takeoff_input(fdir, &mut fw, step, pos, ev),
            state::AIR => self.fall_input(fdir, fw),
            state::GLIDE => self.glide_input(&mut fdir, fw, step),
            _ => {}
        }
        // `air_speed = recom`: Re:CoM eases the facing in the air itself ([`Self::air_move`]).
        if !self.recom_air_now() {
            self.facing = turn_toward(self.facing, self.target_yaw, self.params.turn_rate, step);
        }
        if self.state_changed && self.state == state::AIR {
            let keep = if std::mem::take(&mut self.took_off) && self.speeds.air == Mode::Recom { speeds::AIR_KEEP } else { 1.0 };
            self.air_vel = self.move_dir * (self.speed * keep);
        }
        self.state_changed = false;

        // ---- flinch-immunity timer ----
        if self.no_flinch_on {
            if !self.no_flinch_counting {
                if !self.lock {
                    self.no_flinch_counting = true;
                }
            } else {
                self.no_flinch -= step;
                if self.no_flinch <= 0.0 {
                    self.no_flinch_on = false;
                    self.no_flinch_counting = false;
                }
            }
        }
        if self.no_flinch_on != self.no_flinch_reported {
            self.no_flinch_reported = self.no_flinch_on;
            h.set_no_flinch(self.no_flinch_on);
        }
        self.untouchable_step(h, step);

        // ---- physics: this frame's displacement; Re:CoM's mover moves and collides ----
        let disp = match self.state {
            state::AIR => self.air_move(step, pos),
            // M12 step 7: Ragnarok's state 0x25 runs KH1's air step with speed 0 (its hops and the
            // charge's hover), whatever the air toggle says (Re:CoM has no such hop).
            s if s == crate::specials::STATE_OF[crate::specials::RAGNAROK] && self.special.is_some() => self.ragnarok_move(step, pos),
            state::GLIDE => self.glide_move(h, step, &mut out.height),
            _ => self.move_dir.flat().normalized() * (step * self.speed) + self.push_step(step),
        };
        h.set_facing(self.facing);
        h.set_velocity(disp * (1.0 / step));
        self.last_disp = disp;
        self.last_step = step;
        self.last_start = pos;
        out.displacement = disp;

        // ---- Re:CoM's action for the new state; the motion KH1 asked for ----
        if self.swing.is_some_and(|s| self.rec.head() != s.request) {
            // A landing or walk-off replaced the swing's motion.
            self.leave_swing(h);
        }
        // A landing, walk-off or another command replaced the cast's motion (or the item's).
        self.leave_cast(h);
        self.leave_item(h);
        if self.yield_to_recom(&mut out) {
            return out;
        }
        h.set_action(self.recom_action());
        self.issue(h);
        // KH1's guard object: live on the Guard motion's hit track (frames 10–36).
        let guard = self.rec.head() == crate::rows::GUARD && self.rec.flags & kh1_core::motion::flag::HIT_ANY != 0;
        if guard != self.guard_on {
            self.guard_on = guard;
            h.set_guard(guard);
        }
        out.cast = self.take_cast_log();
        out.items = self.take_item_log();
        out.specials = self.take_special_log();
        out
    }

    /// Re:CoM's end action (a swing's or a cast's) entered Re:CoM's win action (the battle is
    /// won, `0x1401F6794` → `0x1401F1B20`): this frame writes nothing more to Re:CoM after it (no
    /// action, no motion), so Re:CoM's battle end runs as it does in Re:CoM; the next KH1 frame, if
    /// one runs, takes over afresh. Checked (decomp `0x1401F6660`, `0x1401F1B20`).
    fn yield_to_recom(&mut self, out: &mut FrameOut) -> bool {
        if !self.recom_took {
            return false;
        }
        self.recom_took = false;
        self.swing = None;
        self.cast = None;
        self.item = None;
        self.special = None;
        if crate::specials::is_state(self.state) {
            self.state = state::GROUND;
        }
        self.lock = false;
        self.rec = MotionRec::unknown();
        out.cast = self.take_cast_log();
        out.items = self.take_item_log();
        out.specials = self.take_special_log();
        out.recom_took = true;
        true
    }

    /// `dodge_roll = recom`: KH1's rule asked for the roll and no command replaced it. What it
    /// replaced ends as before KH1's roll (a swing's, cast's or item's Re:CoM end action, which may
    /// enter Re:CoM's win instead: then no roll), our own pieces of Sora are put down without
    /// writing Re:CoM's untouchable bit (Re:CoM's roll sets and clears it itself), and Re:CoM's
    /// avoid init starts its roll. This frame writes nothing else; the next KH1 frame takes over
    /// afresh once Re:CoM's action changes ([`FrameOut::handed_to_recom`]).
    fn hand_roll_to_recom(&mut self, h: &mut dyn Host, out: &mut FrameOut) {
        if self.swing.is_some_and(|s| self.rec.head() != s.request) {
            self.leave_swing(h);
        }
        self.leave_cast(h);
        self.leave_item(h);
        if self.yield_to_recom(out) {
            return;
        }
        if self.guard_on {
            self.guard_on = false;
            h.set_guard(false);
        }
        // KH1's roll protection is not used (decision 17's no-flinch frames are KH1's roll's).
        self.no_flinch_on = false;
        self.no_flinch_counting = false;
        if self.no_flinch_reported {
            self.no_flinch_reported = false;
            h.set_no_flinch(false);
        }
        // KH1's flag 0x80 after a guard success (≤ 8 frames, Re:CoM's `+0x418 & 1`): Re:CoM's
        // init sets the same bit and its exe clears it after 600 units, so the rest of KH1's
        // window lies inside Re:CoM's; nothing of ours writes the bit from here.
        self.untouchable = false;
        self.untouchable_counting = false;
        self.untouchable_reported = false;
        self.lock = false;
        self.rec = MotionRec::unknown();
        h.start_recom_roll();
        out.recom_roll = true;
        out.cast = self.take_cast_log();
        out.items = self.take_item_log();
        out.specials = self.take_special_log();
    }

    /// KH1's guard success, the guard object's clash record read on the ground (`0x14029BBD0`). Kind 6
    /// (`0x14029D720`): request 0x6E at once (the guard motion and its object end), the idle queued, flag
    /// 0x80 for 8 frames (frozen while the recoil holds the lock), CB+0x41 − 1, CB+0x50 |= 0x10, and a
    /// push back of `−diff × 2.5` when `diff` < 0. Kind 7 (`0x14029D5F0`, 0.183.1): request 0x6F, the idle
    /// queued, flag 0x80 for 8 frames, CB+0x41 − 1 (no Counterattack: it is offered only during 0x6E).
    /// The kind and `diff` come from [`Host::guard_clash`] (a KH1 enemy's attack); Re:CoM's attacks have
    /// no priority: kind 6, no push (decision 20). Checked (decomp), as [`Self::swing_clash`]'s ground case.
    pub(crate) fn guard_success(&mut self, h: &mut dyn Host, ev: &mut Vec<Event>) {
        let (kind, diff) = h.guard_clash().unwrap_or((6, 0));
        let r = if kind == 7 { crate::rows::STRONG_RECOIL } else { crate::rows::RECOIL };
        self.play(r as u32 | req::ACTION, 0.0);
        self.append(req::RELEASE | request::IDLE as u32, 0.0);
        self.lock = true;
        self.combo_counter = self.combo_counter.saturating_sub(1);
        if kind != 7 && diff < 0 {
            // `0x1402C7300(actor, dir, p)`: a slide totalling p, speed p × −ln 0.8 (as `swing_clash`).
            let p = -diff as f32 * CLASH_PUSH;
            self.push = Some((-Vec3::from_yaw(self.facing), p * 0.223_143_55));
        }
        self.clashed = true;
        self.untouchable = true;
        self.untouchable_counting = false;
        self.untouchable_left = 8.0;
        if self.guard_on {
            self.guard_on = false;
            h.set_guard(false);
        }
        h.guard_clink();
        ev.push(Event::GuardSucceeded);
    }

    /// KH1's clash on Sora's own attack (M11, the shared enemy-guard step): his swing met an enemy's
    /// guard row and the row stopped it (`0x1402B4BC0`: a type-0 swing never wins against a guard row,
    /// it gets a clash record, kind 7 when 40 or more under the row's priority, else 6; `diff` = his
    /// swing's priority − the row's). The swing ends (Re:CoM's end action), then, by his state when the
    /// record is read (`0x14029BBD0`):
    /// - on the ground, kind 6 (`0x14029D720`): request 0x6E, the idle queued, flag 0x80 for 8 frames,
    ///   CB+0x41 − 1, a push back of `−diff × G+0x1C × stats+0xC4` when `diff` < 0 (G+0x1C = 2.5,
    ///   Sora's `+0xC4` = 1.0: `0x1402A5130`);
    /// - on the ground, kind 7 (`0x14029D5F0`): request 0x6F (root travel back), the idle queued, flag
    ///   0x80 for 8 frames, CB+0x41 − 1;
    /// - in the air, either kind (`0x14029D510`): request 0x70, the fall queued, flag 0x80 for 8
    ///   frames, a hop of 20 (`0x1402AF960`), the air velocity backward `max(diff / 100 × G+0x30,
    ///   −G+0x34)` (40, 8) when `diff` < 0, else none.
    ///
    /// CB+0x50 gets the clash bit (0x10, 0x20 for kind 7) from the record (`0x1402B4BC0`), which offers
    /// the Counterattack during a kind-6 recoil (`0x140294220` reads request 0x6E). Checked (decomp +
    /// files: btltbl's globals).
    pub(crate) fn swing_clash(&mut self, h: &mut dyn Host, kind: u8, diff: i32, ev: &mut Vec<Event>) {
        // M12 step 7: Strike Raid's thrown Keyblade (KH1's reaction in state 0x27 → `0x1403A00A0`).
        if self.special_clash(h) {
            ev.push(Event::SwingClashed { kind });
            return;
        }
        // M12 step 7: in state 0x20 (Sonic Blade, Ars Arcanum) KH1's reaction ends the special
        // (`0x14029BBD0`'s case 0x20: state 0, the state's exit) and then recoils as from a swing.
        if self.special.is_some() {
            self.end_special(h, "clash");
            if self.recom_took {
                return;
            }
        }
        self.leave_swing(h);
        if self.recom_took {
            return;
        }
        self.speed = 0.0;
        self.clashed = true;
        self.untouchable = true;
        self.untouchable_counting = false;
        self.untouchable_left = 8.0;
        if self.state == state::AIR {
            self.play(crate::rows::AIR_RECOIL as u32 | req::ACTION, 0.0);
            self.append(req::RELEASE | request::FALL as u32, 0.0);
            let y = h.pos().y;
            self.hop(CLASH_AIR_HOP, y);
            let back = if diff < 0 { (diff as f32 / 100.0 * CLASH_AIR_BACK).max(-CLASH_AIR_BACK_MAX) } else { 0.0 };
            self.air_vel = Vec3::from_yaw(self.facing) * back;
        } else {
            let r = if kind == 7 { crate::rows::STRONG_RECOIL } else { crate::rows::RECOIL };
            self.play(r as u32 | req::ACTION, 0.0);
            self.append(req::RELEASE | request::IDLE as u32, 0.0);
            self.combo_counter = self.combo_counter.saturating_sub(1);
            if kind != 7 && diff < 0 {
                // `0x1402C7300(actor, dir, p)`: a slide totalling p, speed p × −ln 0.8.
                let p = -diff as f32 * CLASH_PUSH;
                self.push = Some((-Vec3::from_yaw(self.facing), p * 0.223_143_55));
            }
        }
        self.lock = true;
        ev.push(Event::SwingClashed { kind });
    }

    /// `0x140294220`: the Counterattack is offered while the recoil 0x6E plays after a clash, with
    /// a target, and the accept `0x140293480` takes it from motion time 12. Checked (decomp).
    pub fn counter_offered(&self, has_target: bool) -> bool {
        self.abilities.counterattack && self.clashed && has_target && self.rec.head() == crate::rows::RECOIL && 12.0 <= self.rec.frame
    }

    /// The 8-frame countdown of KH1's flag 0x80 (`0x140297FE0`): it starts at the next action
    /// request or when the lock releases. Assumption: leaving the recoil early (its free frames
    /// 28–60) counts as the release. A special keeps the same flag on for its whole run (its
    /// state's enter / exit, `0x14039EC50` / `0x14039EC10`: flags `0x1200080`).
    fn untouchable_step(&mut self, h: &mut dyn Host, step: f32) {
        if self.untouchable {
            if !self.untouchable_counting {
                // 0.183.1: the strong recoil 0x6F (kind 7) holds the lock as 0x6E does (`0x14029D5F0`).
                if !self.lock || !matches!(self.rec.head(), crate::rows::RECOIL | crate::rows::STRONG_RECOIL) {
                    self.untouchable_counting = true;
                }
            } else {
                self.untouchable_left -= step;
                if self.untouchable_left <= 0.0 {
                    self.untouchable = false;
                    self.untouchable_counting = false;
                }
            }
        }
        let want = self.untouchable || self.special.is_some_and(|s| s.protected);
        if want != self.untouchable_reported {
            self.untouchable_reported = want;
            h.set_untouchable(want);
        }
    }

    /// KH1's state and lock at the end of this frame as KH1 reports them, i.e. after its physics
    /// next-state and motion update, which here run at the start of the next handler call. Only
    /// reads: for the trace, which samples between frames. `airborne`: Re:CoM's contact now.
    pub fn after_frame(&self, h: &mut dyn Host, airborne: bool) -> (u32, bool) {
        let pos = h.pos();
        let state = match self.state {
            state::GROUND if airborne => state::AIR,
            state::AIR if !airborne => state::GROUND,
            state::GLIDE => {
                let moved = pos - self.last_start;
                if !airborne || (self.glide_timer <= 0.0 && (moved.y < 0.0 || moved.length() / self.glide_intended < self.params.glide.stall_ratio)) {
                    state::AIR
                } else {
                    state::GLIDE
                }
            }
            s => s,
        };
        let mut lock = self.lock && !(state == state::AIR && self.state == state::GROUND && self.rec.req != request::DODGE_ROLL);
        if lock {
            let release = |q: &Queued| q.request & req::RELEASE != 0;
            if self.rec.start_now {
                let q = self.rec.queue.first();
                let r = q.map_or(self.rec.req, |q| (q.request & req::ID_MASK) as u16);
                let start = q.map_or(0.0, |q| q.start);
                if q.is_some_and(release) || self.info(r).is_some_and(|m| m.flags_at(start) & flag::FREE != 0) {
                    lock = false;
                }
            } else if self.rec.blend <= 0.0 {
                let frame = if self.rec.own_clock { self.rec.frame + self.last_step } else { h.motion_frame() };
                if let Some(m) = self.info(self.rec.req) {
                    let ended = m.length <= frame || (!self.rec.own_clock && h.motion_ended());
                    if m.flags_at(frame) & flag::FREE != 0 || (ended && self.rec.queue.first().is_some_and(release)) {
                        lock = false;
                    }
                }
            }
        }
        (state, lock)
    }

    /// The physics step's next state (`0x14029A3C0`), on Re:CoM's mover result.
    fn next_state(&mut self, h: &mut dyn Host, airborne: bool, pos: Vec3, ev: &mut Vec<Event>) {
        let next = match self.state {
            state::GROUND => {
                if airborne {
                    if self.rec.req == request::DODGE_ROLL {
                        self.roll_off_ledge(pos);
                    }
                    state::AIR
                } else {
                    state::GROUND
                }
            }
            // Assumption (kh1-core): state 1's next-state label keeps the state.
            state::TAKE_OFF => state::TAKE_OFF,
            state::AIR => {
                if airborne {
                    state::AIR
                } else {
                    state::GROUND
                }
            }
            state::GLIDE => self.glide_next(h, airborne, pos, ev),
            // M9 stage 9c.1, `0x14039EBA0`: in the air the special ends, Sora falls (state 2); M12
            // step 7: Strike Raid's air callback (`0x1402B1840`) keeps its state.
            s if s == crate::specials::STATE_OF[crate::specials::RAGNAROK] => {
                // `0x14039F6C0`: down after the hits, the landing (state 0); else the state holds.
                if self.ragnarok_landed(h, airborne) {
                    state::GROUND
                } else {
                    s
                }
            }
            s if crate::specials::is_state(s) => {
                if airborne && self.special_fall(h) {
                    state::AIR
                } else {
                    s
                }
            }
            s => s,
        };
        if self.set_state(next) && next == state::AIR {
            // Walk-off: fall at once with the last frame's travel, at most 8.
            self.arc = Arc::timed(0.0, pos.y, self.params.gravity);
            self.recom_rise = None;
            self.lock = false;
            let s = if self.last_step > 0.0 { self.last_step } else { 1.0 };
            let (d, len) = (self.last_disp * (1.0 / s)).flat().normalize_len();
            // `air_speed = recom`: Re:CoM's fall keeps the whole ground speed (`0x1401F1F20`).
            let cap = if self.speeds.air == Mode::Recom { len } else { len.min(self.params.walk_off_cap) };
            self.air_vel = d * cap;
            ev.push(Event::WalkedOff);
        }
    }

    /// `0x1402A7B30` (Guard / Dodge Roll / Glide / Superglide / jump cut).
    #[allow(clippy::too_many_arguments)]
    fn pad_handler(&mut self, h: &mut dyn Host, input: &FrameInput, dir: &mut Vec3, w: &mut f32, pos: Vec3, step: f32, ev: &mut Vec<Event>, probed: &mut Option<f32>, spring: &mut bool) {
        match self.state {
            state::AIR => {
                self.speed_map(dir, w, self.speeds.air == Mode::Recom);
                if self.abilities.superglide && input.square.pressed {
                    self.glide_start(h, true, ev, probed);
                    return;
                }
            }
            state::GROUND | state::TAKE_OFF => self.speed_map(dir, w, true),
            state::GLIDE => self.glide_button(input, dir, w, step, pos, ev),
            _ => {}
        }
        if input.jump.pressed {
            self.jump_offset = 0;
            match self.state {
                state::GROUND => {
                    // Attack's combo window also allows it (M5).
                    if !self.lock && !self.jump_start(h, ev) {
                        *spring = true;
                    }
                }
                state::AIR if self.abilities.glide => {
                    self.glide_start(h, false, ev, probed);
                }
                _ => {}
            }
            return;
        }
        if input.square.pressed && self.state == state::GROUND {
            if self.abilities.guard && (!self.abilities.dodge_roll || *w < self.params.run_speed) {
                // `0x1402A3B00`: face the lock pair `0x142DB3518` (the target's lock point, flat), or keep
                // facing. 0.183.0: the pair holds the soft target too (lock states 1–4, so Auto Lock's
                // orange ring without R1: the workspace's `knowledge/KH1/kh1-lock-on.md`, Checked
                // (decomp)): `lock_target` is R1's lock, else on a Square press the soft target
                // (`win::movement::read_pad`).
                let d = match input.lock_target.map(|t| (t - pos).flat()).filter(|d| !d.is_zero_xz()) {
                    Some(d) => d.normalized(),
                    None => Vec3::from_yaw(self.facing),
                };
                *dir = d;
                *w = 0.0;
                self.target_yaw = d.yaw();
                // `0x1402A3980` for row 0x3D: the Guard motion (its repel is M5's attack object).
                self.clashed = false;
                h.draw_weapon();
                self.play(crate::rows::GUARD as u32 | req::ACTION, 0.0);
                self.append(req::RELEASE | request::IDLE as u32, 0.0);
                // 0.183.0: `0x1402A3B00` sets the combo counter (`CB + 0x41`) to 1, as `kh1-core`'s
                // controller does (Checked (decomp)): a combo doesn't go on through a Guard.
                self.combo_counter = 1;
                ev.push(Event::Guarded);
                return;
            }
            if self.abilities.dodge_roll {
                let y = dir.yaw();
                self.target_yaw = y;
                self.facing = y;
                if self.recom_roll {
                    // `dodge_roll = recom` (owner, 2026-11-02): KH1's rule asked for the roll;
                    // Re:CoM's own roll runs, with its own untouchable frames (no KH1 no-flinch).
                    // The request stands in the queue as KH1's does, so a command later in this
                    // frame still replaces it; [`Kh1Sora::frame`] then hands Sora to Re:CoM.
                    self.recom_roll_wanted = true;
                    self.play(req::ACTION | req::HOLD | request::DODGE_ROLL as u32, 0.0);
                    ev.push(Event::Rolled);
                    return;
                }
                // `0x1402C9E60`
                self.no_flinch = self.params.roll_no_flinch;
                self.no_flinch_on = true;
                self.no_flinch_counting = true;
                self.play(req::ACTION | req::HOLD | request::DODGE_ROLL as u32, 0.0);
                self.append(req::RELEASE | request::IDLE as u32, self.params.blend.roll_to_idle);
                ev.push(Event::Rolled);
                return;
            }
            return;
        }
        if self.cut_ok {
            if self.arc.rise_time <= self.arc.t + self.params.cut_margin || self.state != state::AIR {
                self.cut_ok = false;
            } else if !self.jump_held {
                if self.params.cut_min_t <= self.arc.t {
                    let off = self.jump_offset;
                    self.arc = Arc::timed(self.params.cut_rise_time, pos.y, self.params.gravity);
                    self.set_state(state::AIR);
                    self.cut_ok = false;
                    self.jump_offset = off;
                    ev.push(Event::JumpCut);
                }
            } else if !input.jump.held {
                self.jump_held = false;
            }
        }
    }

    /// `0x1402AF9F0`. False when Re:CoM's own spring jump launched instead (KH1 has no springs:
    /// Re:CoM's gimmick keeps its own height and arc).
    fn jump_start(&mut self, h: &mut dyn Host, ev: &mut Vec<Event>) -> bool {
        if h.spring_jump() {
            return false;
        }
        let high = self.abilities.high_jump;
        let (off, r) = if high { (5, request::HIGH_JUMP) } else { (0, request::JUMP) };
        self.jump_offset = off;
        self.play(req::HOLD | r as u32, 0.0);
        self.set_state(state::TAKE_OFF);
        self.arc.t = 0.0;
        // `jump_height = recom`: Re:CoM's crouch from 0 (its every jump init zeroes `+0x5DC`, a land jump's too).
        self.recom_crouch = (self.speeds.jump == Mode::Recom).then(|| speeds::RecomCrouch::start(&self.recom));
        self.glide_timer = 0.0;
        ev.push(Event::Jumped { high });
        true
    }

    /// `0x1402CC1A0`.
    fn ground_input(&mut self, dir: Vec3, w: f32) {
        if dir.x == 0.0 && dir.y == 0.0 && dir.z == 0.0 {
            self.move_dir = Vec3::from_yaw(self.facing);
            self.speed = 0.0;
            return;
        }
        self.target_yaw = dir.yaw();
        self.move_dir = dir;
        self.speed = w;
        if self.lock {
            return;
        }
        if w == 0.0 {
            if self.rec.flags & flag::FREE == 0 && self.rec.queue_len() == 0 {
                self.locomotion(request::IDLE as u32, self.params.blend.idle);
            }
        } else {
            let r = if self.params.run_speed <= w { request::RUN } else { request::WALK };
            self.locomotion(r as u32, self.params.blend.walk_run);
        }
    }

    /// `0x1402AFDA0`.
    fn takeoff_input(&mut self, dir: Vec3, w: &mut f32, step: f32, pos: Vec3, ev: &mut Vec<Event>) {
        // `jump_height = recom`: Re:CoM's crouch (`0x1401EDA60` phase 1: 2 frames after the press frame); else
        // KH1's (`crouch_frames`, 4 with the press frame).
        let crouching = match self.recom_crouch.as_mut() {
            Some(c) => c.crouching(step),
            None => self.arc.t < self.params.crouch_frames,
        };
        if crouching {
            self.arc.t += step;
            *w = 0.0;
        } else if self.rec.req == request::JUMP || self.rec.req == request::HIGH_JUMP {
            let off = self.jump_offset;
            if self.speeds.jump == Mode::Recom {
                // `jump_height = recom`: Re:CoM's take-off speed and gravity; the arc only keeps
                // the time and the top's frame for KH1's motions and Re:CoM's phase.
                let r = speeds::RecomRise::start(&self.recom, off != 0);
                self.arc = Arc { t: 0.0, y0: pos.y, rise_time: r.rise_frames(), rise: 0.0 };
                self.recom_rise = Some(r);
            } else {
                self.arc = Arc::hop(self.reach(), pos.y, self.params.gravity);
                self.recom_rise = None;
            }
            self.set_state(state::AIR);
            self.recom_crouch = None;
            self.jump_offset = off;
            self.jump_held = true;
            // Re:CoM's own cut rule runs in its rise ([`speeds::RecomRise::advance`]).
            self.cut_ok = self.recom_rise.is_none();
            self.took_off = true;
            ev.push(Event::TookOff);
        } else {
            self.set_state(state::GROUND);
            self.recom_crouch = None;
            *w = 0.0;
            self.jump_held = true;
            self.cut_ok = true;
        }
        self.move_dir = dir;
        self.speed = *w;
    }

    /// `0x1402AFC20`.
    fn fall_input(&mut self, dir: Vec3, w: f32) {
        if 0.0 < w && !self.state_changed {
            self.target_yaw = dir.yaw();
            if self.rec.req == request::DODGE_ROLL {
                self.locomotion(request::FALL as u32, self.params.blend.fall);
            }
        }
        self.move_dir = dir;
        self.speed = w;
    }

    /// `0x1402AF6A0`.
    fn air_move(&mut self, step: f32, pos: Vec3) -> Vec3 {
        let recom_air = self.recom_air_now();
        let p = &self.params;
        let v = if recom_air {
            // `air_speed = recom`: Re:CoM's air control (`0x1401EDA60`).
            let stick = (self.speed > 0.0).then(|| (self.move_dir.yaw(), self.speed));
            let (s, facing, yaw) = speeds::recom_air(self.air_vel.horizontal_length(), self.facing, stick, step);
            self.facing = facing;
            Vec3::from_yaw(yaw) * s
        } else if self.speed <= 0.0 {
            let k = if step < 2.0 { p.air_drag.sqrt() } else { p.air_drag };
            self.air_vel * k
        } else {
            let before = self.air_vel.horizontal_length();
            let mut a = self.move_dir * (p.air_accel * self.speed) + self.air_vel;
            a.y = 0.0;
            let (d, mut len) = a.normalize_len();
            let cap = if before <= p.run_speed { p.run_speed } else { before };
            if cap < len {
                len = cap;
            }
            d * len
        };
        self.air_vel = v;
        let mut disp = v * step;
        disp.y = match self.recom_rise.as_mut() {
            Some(r) => {
                let was = r.rising;
                let dy = r.advance(step, self.jump_down);
                self.arc.t += step;
                if was && !r.rising {
                    // Re:CoM's top or cut: its phase 3 (falling) from here.
                    self.arc.rise_time = self.arc.t;
                }
                dy
            }
            None => self.arc.advance(pos.y, step, self.params.gravity, self.params.fall_cap),
        };
        self.air_vel.y = disp.y;
        // `0x1402AF630`: the apex motion 6 frames before the top, then the fall loop.
        if self.arc.rising() && self.arc.rise_time <= self.arc.t + 6.0 && self.rec.queue_len() == 0 && self.rec.req == self.jump_offset + request::JUMP {
            // Assumption (kh1-core): blend 0.
            self.play((self.jump_offset + request::APEX) as u32, 0.0);
            self.append((self.jump_offset + request::FALL) as u32, 0.0);
        }
        disp
    }

    /// `0x1402B2D30`.
    fn glide_start(&mut self, h: &mut dyn Host, superglide: bool, ev: &mut Vec<Event>, probed: &mut Option<f32>) -> bool {
        if self.glide_timer > 0.0 {
            return false;
        }
        let height = h.height_above_floor(PROBE);
        self.height = height;
        *probed = Some(height);
        if self.params.glide.min_height < height {
            let b = self.params.blend.glide;
            self.locomotion(request::GLIDE as u32, b);
            self.air_vel = self.air_vel * self.params.glide.start_keep;
            self.set_state(state::GLIDE);
            self.superglide = superglide;
            ev.push(Event::GlideStarted { superglide });
            return true;
        }
        false
    }

    fn glide_end(&mut self, y: f32) {
        self.air_vel = self.air_vel * self.params.glide.end_keep;
        self.hop(self.params.glide.end_hop, y);
        self.locomotion(request::FALL as u32, self.params.blend.fall);
    }

    /// `0x1402B2BF0`.
    fn glide_button(&mut self, input: &FrameInput, dir: &mut Vec3, w: &mut f32, step: f32, pos: Vec3, ev: &mut Vec<Event>) {
        if dir.x == 0.0 && dir.y == 0.0 && dir.z == 0.0 {
            *dir = Vec3::from_yaw(self.facing);
            *w = 0.0;
        }
        if self.glide_timer <= 0.0 {
            let held = if self.superglide { input.square.held } else { input.jump.held };
            if !held {
                self.glide_end(pos.y);
                ev.push(Event::GlideEnded);
            }
        } else {
            self.glide_timer -= step;
        }
    }

    /// `0x1402B2880`.
    fn glide_input(&mut self, dir: &mut Vec3, w: f32, step: f32) {
        let g = &self.params.glide;
        let delta = if w <= 0.0 { 0.0 } else { wrap_angle(dir.yaw() - self.facing) };
        if self.rec.blend == 0.0 {
            let r = if g.turn_motion_angle < delta {
                request::GLIDE_TURN_A
            } else if delta < -g.turn_motion_angle {
                request::GLIDE_TURN_B
            } else if g.fast_motion_speed < self.air_vel.horizontal_length() {
                request::GLIDE_FAST
            } else {
                request::GLIDE
            };
            let b = self.params.blend.glide;
            self.locomotion(r as u32, b);
        }
        let turn = if self.superglide { self.params.glide.super_turn } else { self.params.glide.turn } * step;
        let y = wrap_angle(clamp_sym(delta, turn) + self.facing);
        self.facing = y;
        self.target_yaw = y;
    }

    /// `0x1402B2A20`.
    fn glide_move(&mut self, h: &mut dyn Host, step: f32, probed: &mut Option<f32>) -> Vec3 {
        let g = self.params.glide.clone();
        let (top, cap) = if self.superglide {
            let hgt = h.height_above_floor(PROBE);
            self.height = hgt;
            *probed = Some(hgt);
            let c = if hgt < g.low_height {
                let r = hgt / g.low_height;
                let c = r * r * 0.8 + 0.2;
                c + c
            } else {
                g.super_descent
            };
            (g.super_speed, c)
        } else {
            (g.speed, g.descent)
        };
        let mut spd = self.air_vel.horizontal_length();
        if spd < top {
            let s2 = step * g.accel + spd;
            spd = if s2 <= top { s2 } else { top };
        }
        let mut disp = Vec3::from_yaw(self.facing) * (step * spd);
        let mut prev = self.air_vel.y;
        let dy = if prev <= step * cap {
            if prev < 0.0 {
                prev = if 2.0 <= step { prev * g.rise_damp } else { g.rise_damp.sqrt() * prev };
                self.air_vel.y = prev;
            }
            step * self.params.gravity + prev
        } else if step < 2.0 {
            cap * 0.5
        } else {
            cap
        };
        disp.y = dy;
        self.air_vel = disp * (1.0 / step);
        self.air_vel.y = dy;
        self.glide_intended = disp.length();
        disp
    }

    /// `0x1402B2780` (on the last frame's move).
    fn glide_next(&mut self, _h: &mut dyn Host, airborne: bool, pos: Vec3, ev: &mut Vec<Event>) -> u32 {
        if !airborne {
            self.glide_end(pos.y);
        } else if self.glide_timer <= 0.0 {
            let moved = pos - self.last_start;
            if moved.y < 0.0 || moved.length() / self.glide_intended < self.params.glide.stall_ratio {
                self.glide_end(pos.y);
            }
        }
        if self.state != state::GLIDE {
            ev.push(Event::GlideEnded);
        }
        self.state
    }

    /// `0x1402C9EB0`: rolling off a ledge keeps the roll, then falls.
    fn roll_off_ledge(&mut self, pos: Vec3) {
        self.locomotion(req::ACTION | req::HOLD | request::DODGE_ROLL as u32, 0.0);
        self.append(req::RELEASE | request::FALL as u32, self.params.blend.fall);
        self.hop(0.0, pos.y);
        self.state_changed = false;
        self.air_vel = Vec3::ZERO;
    }
}

/// KH1's speed class for Re:CoM's stick-class function (`0x1401E9630`): 0 stand, 1 walk, 2 run
/// (KH1's speed snap `0x1402A5F10`: tilt 0, up to 0.8, over), and the speed in Re:CoM's m/s for
/// `player+0x4DC`.
pub fn speed_class(params: &Params, tilt: f32) -> (u32, f32) {
    speed_class_run(params, tilt, params.run_speed)
}

/// [`speed_class`] with Sora's run (`run`, KH1 units a frame: KH1's, or Re:CoM's at `run_speed =
/// recom`).
pub fn speed_class_run(params: &Params, tilt: f32, run: f32) -> (u32, f32) {
    let w = if tilt <= 0.0 {
        0.0
    } else if tilt <= params.run_tilt {
        params.walk_speed
    } else {
        run
    };
    let class = if w <= 0.0 {
        0
    } else if w < params.run_speed {
        1
    } else {
        2
    };
    (class, crate::units::speed_to_recom_mps(w))
}

fn dec_to_zero(v: f32, step: f32) -> f32 {
    if 0.0 < v {
        if 0.0 <= v - step {
            v - step
        } else {
            0.0
        }
    } else {
        v
    }
}

#[cfg(test)]
mod tests;
