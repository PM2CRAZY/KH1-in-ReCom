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
//! [`Kh1Sora::frame`] is kh1-core's `Controller::step` (the oracle) for these states, minus the
//! Attack (M5), with KH1's numbers. What it reads and writes goes through [`Host`]: Re:CoM's body
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

/// Re:CoM action ids (`player+0x40C`). Checked (decomp, both switches).
pub mod action {
    pub const STAND: u32 = 2;
    pub const WALK: u32 = 3;
    pub const RUN: u32 = 4;
    pub const AVOID: u32 = 6;
    pub const JUMP: u32 = 7;
    pub const GLIDE: u32 = 8;
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
    pub jump: Button,
    pub square: Button,
    /// The lock-on target's position, KH1 units.
    pub lock_target: Option<Vec3>,
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
    start_now: bool,
    queue: Vec<Queued>,
    /// The pending head has been sent to Re:CoM, and whether a Re:CoM row plays it.
    issued: bool,
    issued_ok: bool,
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
}

/// The facing Re:CoM holds may differ from ours by rounding; more than this means Re:CoM turned
/// Sora (an event, a Re:CoM action) and its value wins.
const FACING_SYNC: f32 = 1.0e-3;
/// How far down the glide checks look (Superglide's low height, the largest height KH1 compares).
const PROBE: f32 = 70.0;

impl Kh1Sora {
    pub fn new(params: Params, abilities: Abilities) -> Kh1Sora {
        Kh1Sora {
            params,
            abilities,
            motions: Moveset::kh1_defaults().motions,
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
        }
    }

    fn info(&self, r: u16) -> Option<&MotionInfo> {
        self.motions.info(r)
    }

    /// Takes over from whatever Re:CoM did last (first frame, or after a Re:CoM action such as a
    /// card, a hit or an event): KH1's state from Re:CoM's.
    pub fn resync(&mut self, h: &mut dyn Host, airborne: bool) {
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
        self.air_vel = Vec3::ZERO;
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
    }

    /// Re:CoM started a fall on its own (`0x1401F29F0(player, 1)`, e.g. the end of a knock-back in
    /// the air): KH1 falls from here. The Re:CoM action becomes 7 / falling.
    pub fn enter_fall(&mut self, h: &mut dyn Host) {
        self.resync(h, true);
        self.rec.req = request::FALL;
        h.set_action(self.recom_action());
    }

    /// KH1's state as a Re:CoM action.
    pub fn recom_action(&self) -> Action {
        use action::*;
        let a = |id, sub, phase| Action { id, sub, phase };
        match self.state {
            state::TAKE_OFF => a(JUMP, 0, phase::CROUCH),
            state::AIR => a(JUMP, 0, if self.arc.rising() { phase::RISE } else { phase::FALL }),
            state::GLIDE => a(GLIDE, 0, phase::GLIDE),
            _ if self.lock => match self.rec.head() {
                request::DODGE_ROLL => a(AVOID, SUB_ROLL, phase::FALL),
                crate::rows::GUARD => a(AVOID, SUB_GUARD, phase::NONE),
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
    fn play(&mut self, r: u32, blend: f32) {
        self.rec.queue.clear();
        self.rec.queue.push(Queued { request: r & !req::ACTION, start: 0.0, blend });
        self.rec.start_now = true;
        self.rec.issued = false;
        if r & req::ACTION != 0 {
            self.lock = true;
        }
    }

    /// `0x14029FA30`.
    fn append(&mut self, r: u32, blend: f32) {
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
                release = self.start_head(dt);
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

    fn set_state(&mut self, new: u32) -> bool {
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

    fn hop(&mut self, h: f32, y: f32) {
        self.arc = Arc::hop(h, y, self.params.gravity);
        self.set_state(state::AIR);
    }

    /// `0x1402A5F10`.
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

    pub fn speed_class(&self, tilt: f32) -> (u32, f32) {
        speed_class(&self.params, tilt)
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
        if fresh {
            self.resync(h, airborne);
        } else {
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
        }
        let ev = &mut out.events;

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
            self.pad_handler(h, input, &mut dir, &mut w, pos, step, ev, &mut out.height);
        }
        if input.jump.pressed {
            let r = self.rec.req;
            if (r == request::LAND || r == request::HIGH_LAND) && self.state == state::GROUND && self.lock {
                // `0x1402A6220`: a jump during the land lock: unlock and jump at t = 2.
                self.lock = false;
                self.jump_offset = 0;
                self.jump_start(ev);
                self.set_last_start(self.params.land_jump_t);
                self.arc.t = self.params.land_jump_t;
            }
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
        self.facing = turn_toward(self.facing, self.target_yaw, self.params.turn_rate, step);
        if self.state_changed && self.state == state::AIR {
            self.air_vel = self.move_dir * self.speed;
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

        // ---- physics: this frame's displacement; Re:CoM's mover moves and collides ----
        let disp = match self.state {
            state::AIR => self.air_move(step, pos),
            state::GLIDE => self.glide_move(h, step, &mut out.height),
            _ => self.move_dir.flat().normalized() * (step * self.speed),
        };
        h.set_facing(self.facing);
        h.set_velocity(disp * (1.0 / step));
        self.last_disp = disp;
        self.last_step = step;
        self.last_start = pos;
        out.displacement = disp;

        // ---- Re:CoM's action for the new state; the motion KH1 asked for ----
        h.set_action(self.recom_action());
        self.issue(h);
        out
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
            s => s,
        };
        if self.set_state(next) && next == state::AIR {
            // Walk-off: fall at once with the last frame's travel, at most 8.
            self.arc = Arc::timed(0.0, pos.y, self.params.gravity);
            self.lock = false;
            let s = if self.last_step > 0.0 { self.last_step } else { 1.0 };
            let (d, len) = (self.last_disp * (1.0 / s)).flat().normalize_len();
            self.air_vel = d * len.min(self.params.walk_off_cap);
            ev.push(Event::WalkedOff);
        }
    }

    /// `0x1402A7B30` (Guard / Dodge Roll / Glide / Superglide / jump cut).
    #[allow(clippy::too_many_arguments)]
    fn pad_handler(&mut self, h: &mut dyn Host, input: &FrameInput, dir: &mut Vec3, w: &mut f32, pos: Vec3, step: f32, ev: &mut Vec<Event>, probed: &mut Option<f32>) {
        match self.state {
            state::AIR => {
                self.speed_map(dir, w);
                if self.abilities.superglide && input.square.pressed {
                    self.glide_start(h, true, ev, probed);
                    return;
                }
            }
            state::GROUND | state::TAKE_OFF => self.speed_map(dir, w),
            state::GLIDE => self.glide_button(input, dir, w, step, pos, ev),
            _ => {}
        }
        if input.jump.pressed {
            self.jump_offset = 0;
            match self.state {
                state::GROUND => {
                    // Attack's combo window also allows it (M5).
                    if !self.lock {
                        self.jump_start(ev);
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
                // `0x1402A3B00`: face the lock-on target, or keep facing.
                let d = match input.lock_target {
                    Some(t) => (t - pos).flat().normalized(),
                    None => Vec3::from_yaw(self.facing),
                };
                *dir = d;
                *w = 0.0;
                self.target_yaw = d.yaw();
                // `0x1402A3980` for row 0x3D: the Guard motion (its repel is M5's attack object).
                self.play(crate::rows::GUARD as u32 | req::ACTION, 0.0);
                self.append(req::RELEASE | request::IDLE as u32, 0.0);
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

    /// `0x1402AF9F0`.
    fn jump_start(&mut self, ev: &mut Vec<Event>) {
        let high = self.abilities.high_jump;
        let (off, r) = if high { (5, request::HIGH_JUMP) } else { (0, request::JUMP) };
        self.jump_offset = off;
        self.play(req::HOLD | r as u32, 0.0);
        self.set_state(state::TAKE_OFF);
        self.arc.t = 0.0;
        self.glide_timer = 0.0;
        ev.push(Event::Jumped { high });
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
        if self.arc.t < self.params.crouch_frames {
            self.arc.t += step;
            *w = 0.0;
        } else if self.rec.req == request::JUMP || self.rec.req == request::HIGH_JUMP {
            let off = self.jump_offset;
            self.arc = Arc::hop(self.reach(), pos.y, self.params.gravity);
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
        disp.y = self.arc.advance(pos.y, step, p.gravity, p.fall_cap);
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
    let w = if tilt <= 0.0 {
        0.0
    } else if tilt <= params.run_tilt {
        params.walk_speed
    } else {
        params.run_speed
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
