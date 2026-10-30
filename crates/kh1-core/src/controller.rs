//! Sora's 60 Hz state machine, frame for frame as KH1 FM Steam runs it. One [`Controller::step`]
//! is one actor task (`0x140297FE0`):
//!
//! 1. speed +0x2C = 0;
//! 2. think `0x1402A3300`: pre-step (`0x1402A43A0`: motions for a state the last physics step
//!    entered), control step (`0x1402A6220`: pad, `0x1402A7B30` jump / Square / jump cut, the
//!    Attack command `0x1402A7510`), state input, facing turn;
//! 3. the "state changed" flag is cleared (so a state the think entered never reaches the
//!    pre-step: a jump's take-off keeps its jump motion);
//! 4. the flinch-immunity timer;
//! 5. physics `0x14029A3C0`: state move, root travel, the host's mover ([`World`]), next state;
//! 6. motion update `0x14029F0A0`.
//!
//! The host owns collision through [`World`] and poses the skeleton for [`Output::request`] /
//! [`Output::motion_frame`]; hits are resolved with [`crate::combat`].

use crate::arc::Arc;
use crate::combo::{pick_air, pick_ground, AirCtx, GroundCtx, TargetPart};
use crate::data::{row, Moveset};
use crate::math::{clamp_sym, turn_toward, wrap_angle, Vec3};
use crate::motion::{flag, req, request, MotionPlayer};
use crate::params::{Abilities, Params};
use crate::rng::Kh1Rand;
use crate::shape::BladeVolume;

/// Movement state ids (actor +0x70). Checked (decomp).
pub mod state {
    pub const GROUND: u32 = 0;
    pub const TAKE_OFF: u32 = 1;
    pub const AIR: u32 = 2;
    pub const GLIDE: u32 = 0x15;
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Button {
    /// Went down this frame.
    pub pressed: bool,
    pub held: bool,
}

impl Button {
    pub fn from_held(prev: bool, now: bool) -> Button {
        Button { pressed: now && !prev, held: now }
    }
}

/// One frame of input.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Input {
    /// World move direction on the ground plane (unit, or zero), from the stick and camera.
    pub move_dir: Vec3,
    /// Stick tilt 0..=1 (see [`crate::params::stick_from_raw`]).
    pub tilt: f32,
    pub jump: Button,
    /// The Attack command (the command menu's confirm).
    pub attack: Button,
    /// Square: Guard / Dodge Roll, Superglide.
    pub square: Button,
    /// The lock-on target: an index into the parts passed to [`Controller::step`]. It is the
    /// Attack command's target (`0x140293480` takes the lock pair `0x142DB3518`) and Guard's.
    pub lock: Option<usize>,
}

impl Input {
    /// A stick vector (x right, y forward, length = tilt; from [`crate::params::stick_from_raw`]
    /// pass `[x, -y]`, its y is the pad's, down positive) turned by the camera yaw. Assumption:
    /// KH1's `0x14028C7E0` rotation is "stick forward = the camera's forward (sin yaw, 0,
    /// cos yaw), stick right = (cos yaw, 0, −sin yaw)"; the matrix layout of `0x1400E6C80` was not
    /// checked.
    pub fn move_from_stick(stick: [f32; 2], camera_yaw: f32) -> (Vec3, f32) {
        let tilt = (stick[0] * stick[0] + stick[1] * stick[1]).sqrt().min(1.0);
        if tilt <= 0.0 {
            return (Vec3::ZERO, 0.0);
        }
        let fwd = Vec3::from_yaw(camera_yaw);
        let right = Vec3::new(camera_yaw.cos(), 0.0, -camera_yaw.sin());
        ((fwd * stick[1] + right * stick[0]).normalized(), tilt)
    }
}

/// Where the mover put the body.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MoveResult {
    pub pos: Vec3,
    /// Ground contact after the move (actor +0x2D0 bit 1).
    pub grounded: bool,
    /// Height above the floor below (+0x3D4).
    pub height_above_floor: f32,
}

/// The host's collision.
pub trait World {
    /// `0x1402998A0`: move the body by `disp` (one frame's displacement) with collision.
    fn move_body(&mut self, from: Vec3, disp: Vec3) -> MoveResult;
    /// `0x1402C6E70`: is a wall within the body radius + `extra` along `dir` from `pos`?
    fn wall_ahead(&mut self, _pos: Vec3, _dir: Vec3, _extra: f32) -> bool {
        false
    }
}

/// An endless flat floor at `floor_y` (y down: above it is y < floor_y).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FlatFloor {
    pub floor_y: f32,
}

impl World for FlatFloor {
    fn move_body(&mut self, from: Vec3, disp: Vec3) -> MoveResult {
        let mut p = from + disp;
        let grounded = p.y >= self.floor_y;
        if grounded {
            p.y = self.floor_y;
        }
        MoveResult { pos: p, grounded, height_above_floor: self.floor_y - p.y }
    }
}

/// Why an Attack press did nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// Neither free to act nor inside a combo window (`0x1402A2EC0`), or in the take-off.
    NotAccepted,
    /// Inside the window, but the last swing hit nothing and Combo Master is off.
    LastSwingMissed,
    /// Accepted in a state with no attack rows (glide): consumed.
    NoRows,
    /// Lost to a jump press on the same frame.
    Jump,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Jumped { high: bool },
    TookOff,
    JumpCut,
    WalkedOff,
    Landed,
    GlideStarted { superglide: bool },
    GlideEnded,
    Rolled,
    Guarded,
    AttackStarted { row: usize, request: i16, counter: u8, finisher: bool },
    AttackRefused(Refused),
    MotionStarted(u16),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Output {
    pub pos: Vec3,
    /// This frame's displacement before collision.
    pub displacement: Vec3,
    /// Air velocity (+0x3C0); y is the last vertical step.
    pub air_velocity: Vec3,
    /// Facing yaw (+0x3C) and target yaw (+0x150).
    pub facing: f32,
    pub target_yaw: f32,
    pub state: u32,
    /// Actor flag 4.
    pub action_lock: bool,
    /// The motion playing (request id), its entry and clock.
    pub request: u16,
    pub entry: Option<usize>,
    pub motion_frame: f32,
    pub motion_flags: u32,
    pub hit_active: bool,
    pub combo_window: bool,
    pub trail: bool,
    /// Flinch immunity as the next frame's damage step will see it (actor flag 0x80).
    pub flinch_immune: bool,
    /// CB+0x41 and CB+0x40.
    pub combo_counter: u8,
    pub row: Option<usize>,
    pub grounded: bool,
    pub events: Vec<Event>,
}

/// One attack object (pool `0x142DB4710`) bound to Sora's motion.
#[derive(Clone, Debug, PartialEq)]
pub struct Swing {
    /// Unique per attack start.
    pub id: u64,
    pub row: usize,
    /// btltbl offset of its attack record.
    pub attack: u32,
    /// The motion it was born in (recorded on its first task frame).
    pub motion: Option<u16>,
    fresh: bool,
    pub volume: BladeVolume,
    /// Actors it has hit (the defender's +0x3E0 mask).
    pub hit_owners: Vec<u32>,
}

/// The combo record (actor +0x14C).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ComboRecord {
    /// +0x41.
    pub counter: u8,
    /// +0x40.
    pub row: Option<usize>,
    /// +0x50: hit bits of the current swing (1 = landed, 4 = on an immune target, 0x40 = crit).
    pub hits: u32,
}

#[derive(Clone, Debug)]
pub struct Controller {
    pub params: Params,
    pub abilities: Abilities,
    pub moveset: Moveset,
    /// actor +0x10.
    pub pos: Vec3,
    /// +0x3C / +0x150.
    pub facing: f32,
    pub target_yaw: f32,
    /// +0x20..+0x2C: move direction and speed.
    pub move_dir: Vec3,
    pub speed: f32,
    /// +0x3C0.
    pub air_vel: Vec3,
    /// +0x70 / +0x378.
    pub state: u32,
    pub prev_state: u32,
    /// Flag 0x400.
    state_changed: bool,
    /// Flag 4.
    pub lock: bool,
    /// Flags 0x20000 (jump still held) and 0x40000 (the rise may be cut).
    jump_held: bool,
    cut_ok: bool,
    pub arc: Arc,
    /// +0x3D0: 0, or 5 after a High Jump.
    pub jump_offset: u16,
    pub grounded: bool,
    pub height_above_floor: f32,
    /// work +0x100 and work +0x104 bit 1.
    pub glide_timer: f32,
    pub superglide: bool,
    glide_intended: f32,
    /// actor +0x0C and flags 0x80 / 0x100000.
    pub no_flinch: f32,
    no_flinch_on: bool,
    no_flinch_counting: bool,
    pub motion: MotionPlayer,
    pub combo: ComboRecord,
    pub swing: Option<Swing>,
    swing_seq: u64,
    pub rng: Kh1Rand,
}

impl Controller {
    pub fn new(params: Params, abilities: Abilities, moveset: Moveset, pos: Vec3) -> Controller {
        let mut c = Controller {
            params,
            abilities,
            moveset,
            pos,
            facing: 0.0,
            target_yaw: 0.0,
            move_dir: Vec3::ZERO,
            speed: 0.0,
            air_vel: Vec3::ZERO,
            state: state::GROUND,
            prev_state: state::GROUND,
            state_changed: false,
            lock: false,
            jump_held: false,
            cut_ok: false,
            arc: Arc::default(),
            jump_offset: 0,
            grounded: true,
            height_above_floor: 0.0,
            glide_timer: 0.0,
            superglide: false,
            glide_intended: 0.0,
            no_flinch: 0.0,
            no_flinch_on: false,
            no_flinch_counting: false,
            motion: MotionPlayer::new(),
            combo: ComboRecord { counter: 1, ..Default::default() },
            swing: None,
            swing_seq: 0,
            rng: Kh1Rand::default(),
        };
        c.motion.play(request::IDLE as u32, 0.0);
        c.motion.update(&c.moveset.motions, 1.0, 0.0);
        c
    }

    /// `0x140297870`.
    pub fn can_act(&self) -> bool {
        !self.lock
    }

    /// The combo window bit (+0x1CC & 0x400000).
    pub fn in_window(&self) -> bool {
        self.motion.flags & flag::COMBO_WINDOW != 0
    }

    /// Flinch immunity (flag 0x80): the damage step gives no hit reaction.
    pub fn flinch_immune(&self) -> bool {
        self.no_flinch_on
    }

    /// `0x140293480` for the Attack command: not in the take-off, and free or inside the window.
    pub fn accepts_attack(&self) -> bool {
        self.state != state::TAKE_OFF && (self.can_act() || self.in_window())
    }

    /// The current swing's hit test is live this frame (`0x1402B47A0`): still in the motion it
    /// was born in, not that motion's first frame, its hit slot's flag on, and not a pure guard.
    pub fn hit_live(&self) -> bool {
        let Some(s) = &self.swing else { return false };
        let Some(m) = s.motion else { return false };
        let Some(a) = self.moveset.attack(s.attack) else { return false };
        !s.fresh && m == self.motion.request && !self.motion.just_started && a.clash_type() != 2 && self.motion.flags & (flag::HIT0 << (a.hit_slot & 3)) != 0
    }

    /// The guard object is live (row 0x3D's motion hit track).
    pub fn guard_live(&self) -> bool {
        let Some(s) = &self.swing else { return false };
        s.row == row::GUARD && s.motion == Some(self.motion.request) && !s.fresh && self.motion.flags & flag::HIT_ANY != 0
    }

    /// A hit landed for the current swing (`0x1402B4060`: CB+0x50 |= bits).
    pub fn register_hit(&mut self, owner: u32, bits: u32) {
        self.combo.hits |= bits;
        if let Some(s) = &mut self.swing {
            if !s.hit_owners.contains(&owner) {
                s.hit_owners.push(owner);
            }
        }
    }

    fn set_state(&mut self, new: u32) -> bool {
        if new == self.state {
            return false;
        }
        self.state_changed = true;
        self.prev_state = self.state;
        // exit / enter (`0x1402B29F0` / `0x1402B2D00`)
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

    fn play(&mut self, r: u32, blend: f32) {
        if self.motion.play(r, blend) {
            self.lock = true;
        }
    }

    fn append(&mut self, r: u32, blend: f32) {
        if self.motion.append(r, blend) {
            self.lock = true;
        }
    }

    fn locomotion(&mut self, r: u32, blend: f32, step: f32) {
        if let Some(true) = self.motion.locomotion(&self.moveset.motions, r, blend, step) {
            self.lock = true;
        }
    }

    /// `0x1402AF960`: a hop arc from here; state 2.
    fn hop(&mut self, h: f32) {
        self.arc = Arc::hop(h, self.pos.y, self.params.gravity);
        self.set_state(state::AIR);
    }

    /// `0x1402A5F10`: tilt → speed (0 / walk / run); no stick keeps the last direction.
    fn speed_map(&self, dir: &mut Vec3, w: &mut f32) {
        if *w <= 0.0 {
            if dir.is_zero_xz() {
                *dir = self.move_dir;
            }
            dir.y = 0.0;
            *w = 0.0;
        } else if *w <= self.params.run_tilt {
            *w = self.params.walk_speed;
        } else {
            *w = self.params.run_speed;
        }
    }

    /// One 60 Hz frame. `step` is KH1's frame step (1.0 at 60 fps). `parts` are the enemy body
    /// parts (the lock indexes them).
    pub fn step(&mut self, input: &Input, parts: &[TargetPart], world: &mut dyn World, step: f32) -> Output {
        let mut ev = Vec::new();
        self.speed = 0.0;
        // Command accept (the menu task; Assumption: it runs before Sora's task in the frame).
        let attack_cmd = input.attack.pressed && self.accepts_attack();
        if input.attack.pressed && !attack_cmd {
            ev.push(Event::AttackRefused(Refused::NotAccepted));
        }
        let lock_target = input.lock.and_then(|i| parts.get(i).copied());

        // ---- think: pre-step ----
        self.glide_timer = dec_to_zero(self.glide_timer, step);
        let mut dir = self.move_dir;
        let mut w = self.speed;
        if self.state_changed {
            match self.state {
                state::GROUND if matches!(self.prev_state, 2 | 3 | 0x10 | 0x15) => {
                    self.locomotion((self.jump_offset as u32 + request::LAND as u32) | req::ACTION, 0.0, step);
                    self.append(req::RELEASE | request::IDLE as u32, self.params.blend.land_to_idle);
                    dir = Vec3::from_yaw(self.facing);
                    w = 0.0;
                    ev.push(Event::Landed);
                }
                state::AIR => self.locomotion(request::FALL as u32, self.params.blend.fall, step),
                _ => {}
            }
            self.state_changed = false;
        }

        // ---- control step ----
        let jump_cmd = input.jump.pressed;
        if self.can_act() {
            dir = input.move_dir;
            w = input.tilt;
            self.pad_handler(input, &mut dir, &mut w, lock_target, step, &mut ev);
        }
        if !jump_cmd {
            if attack_cmd {
                self.command_attack(&mut dir, &mut w, lock_target, parts, world, &mut ev);
            }
        } else {
            if attack_cmd {
                ev.push(Event::AttackRefused(Refused::Jump));
            }
            let r = self.motion.request;
            if (r == request::LAND || r == request::HIGH_LAND) && self.state == state::GROUND && !self.can_act() {
                // `0x1402A6220`: a jump during the land lock: unlock and jump at t = 2.
                self.lock = false;
                self.jump_offset = 0;
                if self.jump_start(&mut ev) {
                    self.motion.set_last_start(self.params.land_jump_t);
                    self.arc.t = self.params.land_jump_t;
                }
            }
        }

        // ---- state input (+0x18) on the flattened move vector ----
        let flat = Vec3::new(dir.x, 0.0, dir.z).normalized();
        let mut fdir = Vec3::new(flat.x, dir.y, flat.z);
        let mut fw = w;
        match self.state {
            state::GROUND => self.ground_input(fdir, fw, step),
            state::TAKE_OFF => self.takeoff_input(fdir, &mut fw, step, &mut ev),
            state::AIR => self.fall_input(fdir, fw, step),
            state::GLIDE => self.glide_input(&mut fdir, fw, step),
            _ => {}
        }
        // Facing turn (`0x1402A8300`), then a think-entered state 2 takes the move as velocity.
        self.facing = turn_toward(self.facing, self.target_yaw, self.params.turn_rate, step);
        if self.state_changed && self.state == state::AIR {
            self.air_vel = self.move_dir * self.speed;
        }
        // The task clears the state-changed flag after the think.
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

        // ---- physics ----
        let start = self.pos;
        let mut disp = match self.state {
            state::AIR => self.air_move(step),
            state::GLIDE => self.glide_move(step),
            _ => self.move_dir.flat().normalized() * (step * self.speed),
        };
        if self.motion.root_on {
            disp += self.motion.root_delta;
        }
        let moved = world.move_body(self.pos, disp);
        self.pos = moved.pos;
        self.grounded = moved.grounded;
        self.height_above_floor = moved.height_above_floor;
        let airborne = !moved.grounded;
        let next = match self.state {
            state::GROUND => {
                if airborne {
                    if self.motion.request == request::DODGE_ROLL {
                        self.roll_off_ledge(step);
                    }
                    state::AIR
                } else {
                    state::GROUND
                }
            }
            // Assumption: state 1's next-state label (`0x1402AFD90`) keeps the state.
            state::TAKE_OFF => state::TAKE_OFF,
            state::AIR => {
                if airborne {
                    state::AIR
                } else {
                    state::GROUND
                }
            }
            state::GLIDE => self.glide_next(airborne, start, step, &mut ev),
            s => s,
        };
        if self.set_state(next) && next == state::AIR {
            // Walk-off (`0x14029A3C0`): fall at once with this frame's travel, at most 8.
            self.arc = Arc::timed(0.0, self.pos.y, self.params.gravity);
            self.lock = false;
            let (d, len) = (disp * (1.0 / step)).flat().normalize_len();
            self.air_vel = d * len.min(self.params.walk_off_cap);
            ev.push(Event::WalkedOff);
        }

        // ---- motion update ----
        let r = self.motion.update(&self.moveset.motions, step, self.facing);
        if r.release_lock {
            self.lock = false;
        }
        if let Some(id) = r.started {
            ev.push(Event::MotionStarted(id));
        }
        // The attack object's task: record the motion on its first frame, die when it changes.
        if let Some(s) = &mut self.swing {
            match s.motion {
                None => {
                    s.motion = Some(self.motion.request);
                    s.fresh = true;
                }
                Some(m) => {
                    s.fresh = false;
                    if m != self.motion.request {
                        self.swing = None;
                    }
                }
            }
        }

        Output {
            pos: self.pos,
            displacement: disp,
            air_velocity: self.air_vel,
            facing: self.facing,
            target_yaw: self.target_yaw,
            state: self.state,
            action_lock: self.lock,
            request: self.motion.request,
            entry: self.motion.entry,
            motion_frame: self.motion.frame,
            motion_flags: self.motion.flags,
            hit_active: self.hit_live(),
            combo_window: self.in_window(),
            trail: self.motion.flags & flag::TRAIL != 0,
            flinch_immune: self.no_flinch_on,
            combo_counter: self.combo.counter,
            row: self.combo.row,
            grounded: self.grounded,
            events: ev,
        }
    }

    /// `0x1402A7B30`.
    fn pad_handler(&mut self, input: &Input, dir: &mut Vec3, w: &mut f32, lock: Option<TargetPart>, step: f32, ev: &mut Vec<Event>) {
        match self.state {
            state::AIR => {
                self.speed_map(dir, w);
                if self.abilities.superglide && input.square.pressed {
                    self.glide_start(true, ev);
                    return;
                }
            }
            state::GROUND | state::TAKE_OFF => self.speed_map(dir, w),
            state::GLIDE => self.glide_button(input, dir, w, step, ev),
            _ => {}
        }
        if input.jump.pressed {
            self.jump_offset = 0; // `0x1402A3AD0`
            match self.state {
                state::GROUND => {
                    if self.can_act() || self.in_window() {
                        self.jump_start(ev);
                    }
                }
                state::AIR if self.abilities.glide => {
                    self.glide_start(false, ev);
                }
                _ => {}
            }
            return;
        }
        if input.square.pressed && self.state == state::GROUND {
            if self.abilities.guard && (!self.abilities.dodge_roll || *w < self.params.run_speed) {
                // `0x1402A3B00`: face the lock-on target (or keep facing).
                let d = match lock {
                    Some(t) => (t.centre - self.pos).flat().normalized(),
                    None => Vec3::from_yaw(self.facing),
                };
                *dir = d;
                *w = 0.0;
                self.target_yaw = d.yaw();
                self.row_reader(row::GUARD);
                self.append(req::RELEASE | request::IDLE as u32, 0.0);
                self.combo.counter = 1;
                ev.push(Event::Guarded);
                return;
            }
            if self.abilities.dodge_roll {
                let y = dir.yaw();
                self.target_yaw = y;
                self.facing = y;
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
                    self.arc = Arc::timed(self.params.cut_rise_time, self.pos.y, self.params.gravity);
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

    /// `0x1402AF9F0`.
    fn jump_start(&mut self, ev: &mut Vec<Event>) -> bool {
        let high = self.abilities.high_jump;
        let (off, r) = if high { (5, request::HIGH_JUMP) } else { (0, request::JUMP) };
        self.jump_offset = off;
        self.play(req::HOLD | r as u32, 0.0);
        self.set_state(state::TAKE_OFF);
        self.arc.t = 0.0;
        self.glide_timer = 0.0;
        ev.push(Event::Jumped { high });
        true
    }

    /// `0x1402CC1A0` (base 0).
    fn ground_input(&mut self, dir: Vec3, w: f32, step: f32) {
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
            // Assumption: the field / battle stance never changes (blend 16 on a change).
            if self.motion.flags & flag::FREE == 0 && self.motion.queue_len() == 0 {
                self.locomotion(request::IDLE as u32, self.params.blend.idle, step);
            }
        } else {
            let r = if self.params.run_speed <= w { request::RUN } else { request::WALK };
            self.locomotion(r as u32, self.params.blend.walk_run, step);
        }
    }

    /// `0x1402AFDA0`.
    fn takeoff_input(&mut self, dir: Vec3, w: &mut f32, step: f32, ev: &mut Vec<Event>) {
        if self.arc.t < self.params.crouch_frames {
            self.arc.t += step;
            *w = 0.0;
        } else if self.motion.request == request::JUMP || self.motion.request == request::HIGH_JUMP {
            let off = self.jump_offset;
            self.arc = Arc::hop(self.reach(), self.pos.y, self.params.gravity);
            self.set_state(state::AIR);
            self.jump_offset = off;
            self.jump_held = true;
            self.cut_ok = true;
            ev.push(Event::TookOff);
        } else {
            self.set_state(state::GROUND);
            *w = 0.0;
            self.jump_held = true;
            self.cut_ok = true;
        }
        self.move_dir = dir;
        self.speed = *w;
    }

    /// `0x1402AFC20` (ledge / bar / swim grabs are not modelled).
    fn fall_input(&mut self, dir: Vec3, w: f32, step: f32) {
        if 0.0 < w && !self.state_changed {
            self.target_yaw = dir.yaw();
            if self.motion.request == request::DODGE_ROLL {
                self.locomotion(request::FALL as u32, self.params.blend.fall, step);
            }
        }
        self.move_dir = dir;
        self.speed = w;
    }

    /// `0x1402AF6A0`: air control, drag, the arc.
    fn air_move(&mut self, step: f32) -> Vec3 {
        let p = &self.params;
        let v = if self.speed <= 0.0 {
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
        disp.y = self.arc.advance(self.pos.y, step, p.gravity, p.fall_cap);
        self.air_vel.y = disp.y;
        // `0x1402AF630`: the apex motion 6 frames before the top, then the fall loop.
        if self.arc.rising() && self.arc.rise_time <= self.arc.t + 6.0 && self.motion.queue_len() == 0 && self.motion.request == self.jump_offset + request::JUMP {
            // Assumption: blend 0 (the decomp lost the call's blend argument).
            self.play((self.jump_offset + request::APEX) as u32, 0.0);
            self.append((self.jump_offset + request::FALL) as u32, 0.0);
        }
        disp
    }

    /// `0x1402B2D30` (from `0x1402B26C0` Glide / `0x1402B2750` Superglide).
    fn glide_start(&mut self, superglide: bool, ev: &mut Vec<Event>) -> bool {
        if self.glide_timer <= 0.0 && self.params.glide.min_height < self.height_above_floor {
            let b = self.params.blend.glide;
            self.locomotion(request::GLIDE as u32, b, 1.0);
            self.air_vel = self.air_vel * self.params.glide.start_keep;
            self.set_state(state::GLIDE);
            self.superglide = superglide;
            ev.push(Event::GlideStarted { superglide });
            return true;
        }
        false
    }

    /// The glide's end hop (`0x1402B2670` and the same code in `0x1402B2BF0` / `0x1402B2780`).
    fn glide_end(&mut self, step: f32) {
        self.air_vel = self.air_vel * self.params.glide.end_keep;
        self.hop(self.params.glide.end_hop);
        self.locomotion(request::FALL as u32, self.params.blend.fall, step);
    }

    /// `0x1402B2BF0`: the glide's pad slot.
    fn glide_button(&mut self, input: &Input, dir: &mut Vec3, w: &mut f32, step: f32, ev: &mut Vec<Event>) {
        if dir.x == 0.0 && dir.y == 0.0 && dir.z == 0.0 {
            *dir = Vec3::from_yaw(self.facing);
            *w = 0.0;
        }
        if self.glide_timer <= 0.0 {
            let held = if self.superglide { input.square.held } else { input.jump.held };
            if !held {
                self.glide_end(step);
                ev.push(Event::GlideEnded);
            }
        } else {
            self.glide_timer -= step;
        }
    }

    /// `0x1402B2880`: glide turn and motions.
    fn glide_input(&mut self, dir: &mut Vec3, w: f32, step: f32) {
        let g = &self.params.glide;
        let delta = if w <= 0.0 { 0.0 } else { wrap_angle(dir.yaw() - self.facing) };
        if self.motion.blend == 0.0 {
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
            self.locomotion(r as u32, b, step);
        }
        let turn = if self.superglide { self.params.glide.super_turn } else { self.params.glide.turn } * step;
        let y = wrap_angle(clamp_sym(delta, turn) + self.facing);
        self.facing = y;
        self.target_yaw = y;
    }

    /// `0x1402B2A20`.
    fn glide_move(&mut self, step: f32) -> Vec3 {
        let g = &self.params.glide;
        let (top, cap) = if self.superglide {
            let h = self.height_above_floor;
            let c = if h < g.low_height {
                let r = h / g.low_height;
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
        // Assumption: `0x1402A9240` is a 3-component length.
        self.glide_intended = disp.length();
        disp
    }

    /// `0x1402B2780`.
    fn glide_next(&mut self, airborne: bool, start: Vec3, step: f32, ev: &mut Vec<Event>) -> u32 {
        if !airborne {
            self.glide_end(step);
        } else if self.glide_timer <= 0.0 {
            let moved = self.pos - start;
            if moved.y < 0.0 || moved.length() / self.glide_intended < self.params.glide.stall_ratio {
                self.glide_end(step);
            }
        }
        if self.state != state::GLIDE {
            ev.push(Event::GlideEnded);
        }
        self.state
    }

    /// `0x1402C9EB0`: rolling off a ledge keeps the roll, then falls.
    fn roll_off_ledge(&mut self, step: f32) {
        self.locomotion(req::ACTION | req::HOLD | request::DODGE_ROLL as u32, 0.0, step);
        self.append(req::RELEASE | request::FALL as u32, self.params.blend.fall);
        // Assumption: the queued fall's end callback (`0x1402C9F20`) is not modelled.
        self.hop(0.0);
        self.state_changed = false;
        self.air_vel = Vec3::ZERO;
    }

    /// `0x1402A3980`: play the row's motion and build its attack object.
    fn row_reader(&mut self, r: usize) {
        let Some(rw) = self.moveset.row(r).cloned() else { return };
        if rw.request >= 0 {
            self.play(rw.request as u32 | req::ACTION, 0.0);
        }
        self.swing_seq += 1;
        self.swing = Some(Swing { id: self.swing_seq, row: r, attack: rw.attack, motion: None, fresh: true, volume: BladeVolume::new(), hit_owners: Vec::new() });
        self.combo.hits = 0;
        self.combo.row = Some(r);
    }

    /// `0x1402A7510`, command type 1 (Attack).
    #[allow(clippy::too_many_arguments)]
    fn command_attack(&mut self, dir: &mut Vec3, w: &mut f32, lock: Option<TargetPart>, parts: &[TargetPart], world: &mut dyn World, ev: &mut Vec<Event>) {
        // `0x1402A38E0`. Assumption: the Attack command's table entry has flag 2 (face target).
        match lock {
            Some(t) => {
                let d = (t.centre - self.pos).flat().normalized();
                *dir = d;
                self.target_yaw = d.yaw();
            }
            None => {
                if dir.is_zero_xz() {
                    *dir = Vec3::from_yaw(self.facing);
                }
            }
        }
        *w = 0.0;
        if !self.in_window() {
            self.combo.counter = 1;
        } else {
            if self.combo.hits == 0 && !self.abilities.combo_master {
                ev.push(Event::AttackRefused(Refused::LastSwingMissed));
                return;
            }
            self.combo.counter = self.combo.counter.wrapping_add(1);
        }
        let current = self.motion.request as i32;
        let pos = self.pos;
        let length = if self.state == state::AIR { self.params.air_length(&self.abilities) } else { self.params.ground_length(&self.abilities) };
        let finisher = length <= self.combo.counter;
        let picked = match self.state {
            state::GROUND => {
                let ctx = GroundCtx {
                    params: &self.params,
                    abilities: &self.abilities,
                    moveset: &self.moveset,
                    counter: self.combo.counter,
                    length,
                    current_request: current,
                    pos,
                    aim: *dir,
                    target: lock,
                    parts,
                };
                let rng = &mut self.rng;
                let probe = self.params.combo.obstacle_probe;
                let g = pick_ground(&ctx, &mut || rng.next_f32(), &mut |d| world.wall_ahead(pos, d, probe));
                *dir = Vec3::new(g.aim.x, g.hop, g.aim.z);
                *w = g.speed;
                g.row
            }
            state::AIR => {
                let ctx = AirCtx { params: &self.params, abilities: &self.abilities, moveset: &self.moveset, counter: self.combo.counter, length, current_request: current, in_state_2: true, pos, aim: *dir, target: lock };
                let (r, aim) = pick_air(&ctx);
                *dir = aim;
                r
            }
            _ => {
                ev.push(Event::AttackRefused(Refused::NoRows));
                return;
            }
        };
        self.row_exec(picked, dir, w, finisher, ev);
    }

    /// `0x1402A68F0`.
    fn row_exec(&mut self, r: usize, dir: &mut Vec3, w: &mut f32, finisher: bool, ev: &mut Vec<Event>) {
        self.row_reader(r);
        let rw = self.moveset.row(r).cloned().unwrap_or_else(crate::data::ActionRow::none);
        let mut hop = rw.hop;
        match r {
            row::AERIAL_SWEEP | row::JUMP_UP | row::HOP => {
                if r == row::HOP {
                    *w = self.params.combo.hop_speed;
                } else {
                    hop = dir.y;
                }
                self.append(req::RELEASE | request::FALL as u32, 0.0);
                self.combo.counter = 1;
            }
            row::HURRICANE_BLAST..=row::AIR_SWING_1 => {
                self.append(req::RELEASE | request::FALL as u32, 0.0);
                *dir = dir.normalized();
                let c = &self.params.combo;
                let v = if self.combo.counter == 1 {
                    c.air_speed_first
                } else if r == row::HURRICANE_BLAST || r == row::AIR_FINISHER {
                    c.air_speed_finisher
                } else {
                    c.air_speed_middle
                };
                self.air_vel = *dir * v;
                *w = 0.0;
            }
            // Assumption: flight rows 19–21 and the vertical homing of 0x16 / 0x18–0x1A are not
            // modelled (no flight state here); they run as ground rows.
            _ => {
                self.append(req::RELEASE | request::IDLE as u32, 0.0);
                *w = 0.0;
            }
        }
        if 0.0 < hop {
            self.hop(hop);
        }
        self.target_yaw = dir.yaw();
        ev.push(Event::AttackStarted { row: r, request: rw.request, counter: self.combo.counter, finisher });
    }
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
