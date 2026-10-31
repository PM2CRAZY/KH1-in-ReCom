//! The handlers against the oracle: kh1-core's controller and [`Kh1Sora`] on a fake Re:CoM that
//! does what Re:CoM does around the handler (motion clock that does not move on a clip's first
//! update, root motion applied in the motion update, then the mover integrating the velocity).

use super::*;
use crate::rows::{row_for, Extension, Stance};
use kh1_core::{Controller, Input, MoveResult, World};

/// A floor at y = 0 up to `edge_z`, then `low` (y down: a drop of `low`).
#[derive(Clone, Copy)]
struct Floor {
    edge_z: f32,
    low: f32,
}

impl Floor {
    fn flat() -> Floor {
        Floor { edge_z: f32::INFINITY, low: 0.0 }
    }
    fn y(&self, p: Vec3) -> f32 {
        if p.z > self.edge_z {
            self.low
        } else {
            0.0
        }
    }
}

impl World for Floor {
    fn move_body(&mut self, from: Vec3, disp: Vec3) -> MoveResult {
        let mut p = from + disp;
        let f = self.y(p);
        let grounded = p.y >= f;
        if grounded {
            p.y = f;
        }
        MoveResult { pos: p, grounded, height_above_floor: f - p.y }
    }
}

struct Clip {
    req: u16,
    time: f32,
    len: f32,
    once: bool,
    root: bool,
    just: bool,
}

/// Re:CoM around the handler.
struct FakeRecom {
    floor: Floor,
    pos: Vec3,
    vel: Vec3,
    facing: f32,
    contact: bool,
    step: f32,
    act: Action,
    clip: Option<Clip>,
    ext: Extension,
    motions: MotionSet,
    no_flinch: bool,
    plays: Vec<(u16, f32, f32)>,
    /// Sora stands on a Re:CoM spring (the ability bit and `0x1401E80C0` say so).
    on_spring: bool,
    spring_launches: usize,
    /// Our root-motion wrapper (`0x14024DCA0`, `crate::movement::RootDelay`): each update applies
    /// the last update's local root travel.
    root_delay: RootDelay,
    /// Re:CoM's `player+0x644` for the swing playing, and the swings started.
    hit: bool,
    tick: u64,
    /// The model's position relative to the body (the root step's anchor, `ctrl+0xF0`).
    model_offset: Vec3,
    swings: Vec<SwingStart>,
    swing_ends: usize,
    /// Re:CoM's body guard as we set it, a guarded hit to report, and the no-damage bit.
    guard: bool,
    block_next: bool,
    untouchable: bool,
    clinks: u32,
    draws: u32,
}

impl FakeRecom {
    fn new(floor: Floor) -> FakeRecom {
        FakeRecom {
            floor,
            pos: Vec3::ZERO,
            vel: Vec3::ZERO,
            facing: 0.0,
            contact: true,
            step: 1.0,
            act: Action { id: action::STAND, sub: 0, phase: 0 },
            clip: None,
            ext: Extension::plan(|_| true),
            motions: Moveset::kh1_defaults().motions,
            no_flinch: false,
            plays: Vec::new(),
            on_spring: false,
            spring_launches: 0,
            root_delay: RootDelay::default(),
            hit: false,
            tick: 0,
            model_offset: Vec3::ZERO,
            swings: Vec::new(),
            swing_ends: 0,
            guard: false,
            block_next: false,
            untouchable: false,
            clinks: 0,
            draws: 0,
        }
    }

    /// Re:CoM's root step as it is: the motion update makes an absolute position (the model's
    /// position + the rotated travel, `0x14024C960`), our wrapper on `0x14024DCA0` delays it, and
    /// the entity task moves the body to it (`P + mask∘(out − P)`, mask (1, 1, 1) here).
    fn root_step(&mut self, local: Vec3) {
        let anchor = self.pos + self.model_offset;
        let root = anchor + local.rotate_yaw(self.facing);
        let out = self.root_delay.position(1, self.tick, anchor, root, 50.0);
        self.pos += out - (self.pos + self.model_offset);
    }

    /// Re:CoM after the state exe: motion advance (+ root motion), then the mover.
    fn after_exe(&mut self) {
        self.tick += 1;
        let step = self.step;
        let mut travel = None;
        if let Some(c) = &mut self.clip {
            if c.just {
                c.just = false;
                // Re:CoM's root step still runs on a clip's first update (no travel of its own):
                // the delay hands out the last motion's pending travel.
                if c.root {
                    travel = Some(Vec3::ZERO);
                }
            } else {
                let old = c.time;
                let mut new = old + step;
                if c.once {
                    new = new.min(c.len);
                } else if c.len > 0.0 && new >= c.len {
                    new -= c.len;
                }
                if c.root {
                    if let Some(m) = self.motions.info(c.req) {
                        travel = Some(m.root_at(new) - m.root_at(old));
                    }
                }
                c.time = new;
            }
        }
        if let Some(local) = travel {
            self.root_step(local);
        }
        let mut p = self.pos + self.vel * step;
        let f = self.floor.y(p);
        self.contact = p.y >= f;
        if self.contact {
            p.y = f;
        }
        self.pos = p;
    }
}

impl Host for FakeRecom {
    fn step(&self) -> f32 {
        self.step
    }
    fn pos(&mut self) -> Vec3 {
        self.pos
    }
    fn airborne(&mut self) -> bool {
        !self.contact
    }
    fn height_above_floor(&mut self, _max: f32) -> f32 {
        self.floor.y(self.pos) - self.pos.y
    }
    fn facing(&mut self) -> f32 {
        self.facing
    }
    fn set_facing(&mut self, yaw: f32) {
        self.facing = yaw;
    }
    fn set_velocity(&mut self, v: Vec3) {
        self.vel = v;
    }
    fn action(&mut self) -> Action {
        self.act
    }
    fn set_action(&mut self, a: Action) {
        self.act = a;
    }
    fn play(&mut self, request: u16, blend: f32, start: f32) -> bool {
        let stance = Stance { flag_418_2: true, ..Default::default() };
        let Some((_row, root)) = row_for(request, &stance, &self.ext) else { return false };
        let looped = matches!(request, 0 | 1 | 2 | 6 | 0xB | 0x71..=0x74);
        let len = self.motions.info(request).map_or(60.0, |m| m.length);
        self.clip = Some(Clip { req: request, time: start, len, once: !looped, root, just: true });
        self.plays.push((request, blend, start));
        true
    }
    fn motion_frame(&mut self) -> f32 {
        self.clip.as_ref().map_or(0.0, |c| c.time)
    }
    fn motion_ended(&mut self) -> bool {
        self.clip.as_ref().is_some_and(|c| c.once && c.time >= c.len)
    }
    fn set_no_flinch(&mut self, on: bool) {
        self.no_flinch = on;
    }
    fn start_swing(&mut self, s: &SwingStart) -> bool {
        // Re:CoM's field-swing init: action 0xA, the clip of the row, its hit flag cleared.
        let len = self.motions.info(s.request).map_or(60.0, |m| m.length);
        self.clip = Some(Clip { req: s.request, time: 0.0, len, once: true, root: s.root, just: true });
        self.act = Action { id: action::ATTACK, sub: 0, phase: crate::attack::ATTACK_PHASE };
        self.swings.push(*s);
        true
    }
    fn swing_hit(&mut self) -> bool {
        self.hit
    }
    fn set_guard(&mut self, on: bool) {
        self.guard = on;
    }
    fn guard_blocked(&mut self) -> bool {
        // Re:CoM's apply only guards while the body guard is on.
        let b = self.block_next && self.guard;
        self.block_next = false;
        b
    }
    fn guard_clink(&mut self) {
        self.clinks += 1;
    }
    fn draw_weapon(&mut self) {
        self.draws += 1;
    }
    fn set_untouchable(&mut self, on: bool) {
        self.untouchable = on;
    }
    fn end_swing(&mut self, _airborne: bool) {
        self.swing_ends += 1;
        self.act = Action { id: action::STAND, sub: 0, phase: 0 };
    }
    fn spring_jump(&mut self) -> bool {
        if !spring::launches(spring::ABILITY, || if self.on_spring { spring::ON_SPRING } else { 0 }) {
            return false;
        }
        // Re:CoM's own jump init: action 7, crouch, row 0x14.
        self.spring_launches += 1;
        self.act = Action { id: action::JUMP, sub: 0, phase: phase::CROUCH };
        self.vel = Vec3::ZERO;
        true
    }
}

#[derive(Clone, Copy, Default)]
struct Pad {
    stick: [f32; 2],
    jump: bool,
    square: bool,
    attack: bool,
}

struct Pair {
    oracle: Controller,
    world: Floor,
    ours: Kh1Sora,
    recom: FakeRecom,
    prev: Pad,
    first: bool,
    o_out: Option<kh1_core::Output>,
}

#[derive(Debug)]
struct Frame {
    oracle: Vec3,
    ours: Vec3,
    o_facing: f32,
    facing: f32,
    o_state: u32,
    state: u32,
    o_lock: bool,
    lock: bool,
    head: u16,
    out: FrameOut,
}

impl Pair {
    fn new(ab: Abilities, floor: Floor) -> Pair {
        Pair::new_at(ab, floor, Vec3::ZERO)
    }

    /// Both start at `pos` (the fake Re:CoM's body too).
    fn new_at(ab: Abilities, floor: Floor, pos: Vec3) -> Pair {
        let params = Params::kh1_defaults();
        let mut recom = FakeRecom::new(floor);
        recom.pos = pos;
        Pair {
            oracle: Controller::new(params.clone(), ab.clone(), Moveset::kh1_defaults(), pos),
            world: floor,
            ours: Kh1Sora::new(params, ab),
            recom,
            prev: Pad::default(),
            first: true,
            o_out: None,
        }
    }

    fn step(&mut self, pad: Pad) -> Frame {
        self.step_dt(pad, 1.0)
    }

    /// One frame with a frame step (2.0 = Re:CoM's 30 fps rooms, dt 100).
    fn step_dt(&mut self, pad: Pad, step: f32) -> Frame {
        self.recom.step = step;
        let (dir, tilt) = Input::move_from_stick(pad.stick, 0.0);
        let jump = Button::from_held(self.prev.jump, pad.jump);
        let square = Button::from_held(self.prev.square, pad.square);
        let attack = Button::from_held(self.prev.attack, pad.attack);
        self.prev = pad;
        let o = self.oracle.step(&Input { move_dir: dir, tilt, jump, square, attack, ..Default::default() }, &[], &mut self.world, step);
        self.o_out = Some(o.clone());
        let out = self.ours.frame(&mut self.recom, &FrameInput { move_dir: dir, tilt, jump, square, lock_target: None, attack, target: None, locked: false }, self.first);
        self.first = false;
        self.recom.after_exe();
        let airborne = !self.recom.contact;
        let (state, lock) = self.ours.after_frame(&mut self.recom, airborne);
        Frame {
            oracle: o.pos,
            ours: self.recom.pos,
            o_facing: o.facing,
            facing: self.recom.facing,
            o_state: o.state,
            state,
            o_lock: o.action_lock,
            lock,
            head: self.ours.rec.head(),
            out,
        }
    }

    /// Steps and asserts the same position, facing, state and lock as the oracle each frame.
    fn run_same(&mut self, n: usize, pad: impl Fn(usize) -> Pad) -> Vec<Frame> {
        (0..n)
            .map(|k| {
                let f = self.step(pad(k));
                assert!((f.oracle - f.ours).length() < 1e-3, "frame {k}: pos {:?} vs oracle {:?}", f.ours, f.oracle);
                assert!(wrap_angle(f.facing - f.o_facing).abs() < 1e-5, "frame {k}: facing {} vs {}", f.facing, f.o_facing);
                assert_eq!(f.state, f.o_state, "frame {k}: state");
                assert_eq!(f.lock, f.o_lock, "frame {k}: lock");
                f
            })
            .collect()
    }
}

fn stick(x: f32, y: f32) -> Pad {
    Pad { stick: [x, y], ..Default::default() }
}

fn all() -> Abilities {
    Abilities { high_jump: false, glide: true, superglide: true, guard: true, dodge_roll: true, ..Default::default() }
}

#[test]
fn walk_run_stop_and_turn_match_the_oracle() {
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    let frames = p.run_same(240, |k| match k {
        0..=9 => Pad::default(),
        10..=49 => stick(0.0, 0.5),  // walk forward
        50..=99 => stick(0.0, 1.0),  // run
        100..=139 => stick(1.0, 0.0), // run right: instant direction, facing turns 11.25°/frame
        140..=159 => stick(-0.3, -0.6),
        _ => Pad::default(),
    });
    // Run speed 8 units per frame.
    let d = frames[80].ours - frames[79].ours;
    assert!((d.length() - 8.0).abs() < 1e-3, "{d:?}");
    // Walk 2.
    let d = frames[30].ours - frames[29].ours;
    assert!((d.length() - 2.0).abs() < 1e-3, "{d:?}");
    // The Re:CoM action follows the speed class.
    let mut q = Pair::new(Abilities::default(), Floor::flat());
    q.run_same(3, |_| stick(0.0, 1.0));
    assert_eq!(q.recom.act.id, action::RUN);
}

#[test]
fn jump_and_high_jump_match_the_oracle() {
    for (high, top) in [(false, 190.0), (true, 290.0)] {
        let mut p = Pair::new(Abilities { high_jump: high, ..Default::default() }, Floor::flat());
        let frames = p.run_same(90, |k| Pad { jump: k < 60, stick: if k > 20 { [0.0, 1.0] } else { [0.0, 0.0] }, ..Default::default() });
        let peak = frames.iter().map(|f| -f.ours.y).fold(0.0, f32::max);
        assert!((peak - top).abs() < 1e-2, "high {high}: peak {peak}");
        assert_eq!(frames[3].ours.y, 0.0, "4 crouch frames");
        assert!(frames.iter().any(|f| f.out.events.contains(&Event::Landed)));
    }
}

#[test]
fn released_jump_is_cut_like_the_oracle() {
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    let frames = p.run_same(70, |k| Pad { jump: k == 0, ..Default::default() });
    let cut = frames.iter().position(|f| f.out.events.contains(&Event::JumpCut));
    assert_eq!(cut, Some(10));
}

#[test]
fn landing_lock_and_the_land_jump_match() {
    // Hold forward through the landing: the 12-frame lock holds Sora, then he runs.
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    let frames = p.run_same(110, |k| Pad { jump: k == 0, stick: if k > 40 { [0.0, 1.0] } else { [0.0, 0.0] }, ..Default::default() });
    let landed = frames.iter().position(|f| f.out.events.contains(&Event::Landed)).unwrap();
    assert_eq!(p.recom.act.id, action::RUN);
    // Locked for 12 frames from the land request, as KH1's land motion (free from frame 12).
    let locked = frames[landed..].iter().take_while(|f| f.lock).count();
    assert_eq!(locked, 12, "the land motion is free from its frame 12 (as the oracle)");
    assert_eq!(frames[landed].head, request::LAND);
    // A jump pressed during the land lock restarts the take-off at t = 2.
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    let land = 1 + 4 + 28 + 28;
    p.run_same(land + 30, |k| Pad { jump: k == 0 || (k > land + 1 && k < land + 4), ..Default::default() });
}

#[test]
fn walking_off_a_ledge_matches() {
    let mut p = Pair::new(Abilities::default(), Floor { edge_z: 100.0, low: 300.0 });
    let frames = p.run_same(80, |_| stick(0.0, 1.0));
    assert!(frames.iter().any(|f| f.out.events.contains(&Event::WalkedOff)));
    assert!(frames.iter().any(|f| f.out.events.contains(&Event::Landed)));
    assert!((frames[79].ours.y - 300.0).abs() < 1e-3);
}

#[test]
fn glide_and_superglide_match() {
    // Jump, press jump again in the air and hold: Glide.
    let mut p = Pair::new(all(), Floor { edge_z: 50.0, low: 2000.0 });
    let frames = p.run_same(160, |k| Pad { jump: !(2..=20).contains(&k), stick: [0.0, 1.0], ..Default::default() });
    assert!(frames.iter().any(|f| f.out.events.contains(&Event::GlideStarted { superglide: false })));
    assert!(frames.iter().any(|f| f.state == state::GLIDE));
    assert_eq!(p.recom.act.id, action::GLIDE);
    // Superglide on Square in the air, held.
    let mut p = Pair::new(all(), Floor { edge_z: 50.0, low: 2000.0 });
    let frames = p.run_same(160, |k| Pad { jump: k < 2, square: k > 20, stick: [0.0, 1.0], ..Default::default() });
    assert!(frames.iter().any(|f| f.out.events.contains(&Event::GlideStarted { superglide: true })));
}

#[test]
fn guard_holds_sora_for_its_motion() {
    let mut p = Pair::new(all(), Floor::flat());
    let frames = p.run_same(90, |k| Pad { square: k == 5, stick: if k > 5 { [0.0, 1.0] } else { [0.0, 0.0] }, ..Default::default() });
    assert!(frames[5].out.events.contains(&Event::Guarded));
    assert_eq!(frames[5].head, crate::rows::GUARD);
    assert_eq!(frames[6].ours, frames[5].ours);
    let free = frames[5..].iter().position(|f| !f.lock).unwrap() + 5;
    assert_eq!(free, 5 + 54, "Guard's 54 frames, then the queued idle releases (as the oracle)");
    assert!(p.recom.plays.iter().any(|&(r, _, _)| r == crate::rows::GUARD));
}

#[test]
fn dodge_roll_root_motion_matches_kh1_frame_for_frame() {
    // Running, Square: Dodge Roll (Guard is equipped, so it needs the run tilt). Re:CoM's root
    // step runs one update late for Sora (decision 17), so its travel lands on KH1's frames.
    let mut p = Pair::new(all(), Floor::flat());
    let pad = |k: usize| Pad { square: k == 10, stick: if k <= 10 { [0.0, 1.0] } else { [0.0, 0.0] }, ..Default::default() };
    let frames = p.run_same(80, pad);
    assert!(frames[10].out.events.contains(&Event::Rolled));
    // 350 units of root travel plus the run frame before and the roll's first frame at speed 8.
    let travel = frames[79].ours.z - frames[9].ours.z;
    assert!((travel - (350.0 + 8.0)).abs() < 1e-2, "{travel}");
    // KH1's 12 no-flinch frames went on and off.
    assert!(!p.recom.no_flinch);
    assert_eq!(p.recom.act.id, action::STAND);
}

#[test]
fn root_delay_never_moves_sora_to_the_anchors_origin() {
    // 0.6.0's in-game bug: the hook took Re:CoM's absolute root position for a travel and, with
    // nothing pending, handed out (0, 0, 0): the body jumped to the room's origin and back.
    let mut d = RootDelay::default();
    let p = Vec3::new(3.0, 1.0, -7.0);
    let step = Vec3::new(0.0, 0.0, 0.2);
    // First update of a swing after a frame without a root step: no travel, at the anchor.
    assert_eq!(d.position(9, 10, p, p + step, 0.5), p);
    // Next update: the anchor moved by the travel the body took; the last travel arrives now.
    let p2 = p;
    let out = d.position(9, 11, p2, p2 + step * 2.0, 0.5);
    assert!((out - (p2 + step)).length() < 1e-6, "{out:?}");
    // A gap drops what was pending (no root step ran in between).
    assert_eq!(d.position(9, 13, p2, p2 + step, 0.5), p2);
}

#[test]
fn root_delay_hands_each_update_the_last_ones_travel() {
    let mut d = RootDelay::default();
    let v = |z| Vec3::new(0.0, 0.0, z);
    assert_eq!(d.swap(7, 1, v(0.0)), v(0.0));
    assert_eq!(d.swap(7, 2, v(17.8)), v(0.0));
    assert_eq!(d.swap(7, 3, v(10.0)), v(17.8));
    // A new root motion on the next frame gets the old one's last travel (KH1's physics order).
    assert_eq!(d.swap(7, 4, v(5.0)), v(10.0));
    // A frame without a root step (a clip without root motion) drops it; so does another controller.
    assert_eq!(d.swap(7, 6, v(1.0)), v(0.0));
    assert_eq!(d.swap(8, 7, v(2.0)), v(0.0));
    assert_eq!(d.swap(8, 8, v(0.0)), v(2.0));
}

#[test]
fn speed_class_uses_kh1s_tilt_line() {
    let s = Kh1Sora::new(Params::kh1_defaults(), Abilities::default());
    assert_eq!(s.speed_class(0.0), (0, 0.0));
    let (c, mps) = s.speed_class(0.8);
    assert_eq!(c, 1);
    assert!((mps - 1.2).abs() < 1e-5);
    let (c, mps) = s.speed_class(0.81);
    assert_eq!(c, 2);
    assert!((mps - 4.8).abs() < 1e-5);
}

#[test]
fn recom_actions_for_kh1_states() {
    let mut p = Pair::new(all(), Floor::flat());
    p.step(Pad { jump: true, ..Default::default() });
    assert_eq!(p.recom.act, Action { id: action::JUMP, sub: 0, phase: phase::CROUCH });
    for _ in 0..6 {
        p.step(Pad { jump: true, ..Default::default() });
    }
    assert_eq!(p.recom.act.phase, phase::RISE);
    for _ in 0..30 {
        p.step(Pad { jump: true, ..Default::default() });
    }
    assert_eq!(p.recom.act.phase, phase::FALL);
}

#[test]
fn variable_step_matches_the_oracle() {
    // Re:CoM's step is dt × 0.02 (2.0 in its 30 fps rooms, up to 4.0 after a stall).
    let mut p = Pair::new(all(), Floor { edge_z: 400.0, low: 500.0 });
    for k in 0..120 {
        let step = [1.0, 2.0, 1.0, 3.0][k % 4];
        let pad = Pad { jump: (10..30).contains(&k) || k > 70, square: k == 50, stick: [0.3, 1.0], ..Default::default() };
        let f = p.step_dt(pad, step);
        assert!((f.oracle - f.ours).length() < 1e-3, "frame {k}: {:?} vs {:?}", f.ours, f.oracle);
        assert_eq!((f.state, f.lock), (f.o_state, f.o_lock), "frame {k}");
    }
}

#[test]
fn spring_decision_follows_recoms_jump_init() {
    // The ability bit gates the gimmick query, as in `0x1401F29F0`.
    let mut asked = false;
    assert!(!spring::launches(0, || {
        asked = true;
        spring::ON_SPRING
    }));
    assert!(!asked, "no ability bit: the gimmick is not asked");
    assert!(spring::launches(spring::ABILITY | 0x10, || spring::ON_SPRING));
    assert!(!spring::launches(spring::ABILITY, || 0));
    assert!(!spring::launches(spring::ABILITY, || 0x14), "another gimmick answer is not a spring");
    // Re:CoM owns the jump action while its spring flag is set; any other action is ours.
    assert!(spring::recom_owns_jump(action::JUMP, spring::FLAG_418 | 1));
    assert!(!spring::recom_owns_jump(action::JUMP, 1));
    assert!(!spring::recom_owns_jump(action::STAND, spring::FLAG_418));
}

#[test]
fn jumping_on_a_spring_hands_the_jump_to_recom() {
    let mut p = Pair::new(Abilities { high_jump: true, ..Default::default() }, Floor::flat());
    p.recom.on_spring = true;
    p.step(Pad::default());
    let f = p.step(Pad { jump: true, ..Default::default() });
    assert!(f.out.recom_spring);
    assert!(!f.out.events.iter().any(|e| matches!(e, Event::Jumped { .. })), "no KH1 jump");
    assert_eq!(p.recom.spring_launches, 1);
    // Re:CoM's action stands as its init left it; we wrote no velocity and no motion.
    assert_eq!(p.recom.act, Action { id: action::JUMP, sub: 0, phase: phase::CROUCH });
    assert_eq!(p.recom.vel, Vec3::ZERO);
    assert_eq!(p.ours.rec.req, 0xFFFF);
    // Off the spring, the same press is KH1's (High) Jump again.
    let mut q = Pair::new(Abilities { high_jump: true, ..Default::default() }, Floor::flat());
    q.step(Pad::default());
    let f = q.step(Pad { jump: true, ..Default::default() });
    assert!(f.out.events.contains(&Event::Jumped { high: true }));
    assert_eq!(q.recom.spring_launches, 0);
}

#[test]
fn a_land_jump_on_a_spring_is_recoms_too() {
    // Jump, land, then press jump during the land lock while standing on a spring.
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    let land = 1 + 4 + 28 + 28;
    p.run_same(land + 2, |k| Pad { jump: k == 0, ..Default::default() });
    p.recom.on_spring = true;
    let f = p.step(Pad { jump: true, ..Default::default() });
    assert!(f.out.recom_spring);
    assert_eq!(p.recom.spring_launches, 1);
}

// ---- KH1's Attack in Re:CoM's attack action (M5) ----

fn attack_rows(events: &[Event]) -> Vec<(usize, u8, bool)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::AttackStarted { row, counter, finisher, .. } => Some((*row, *counter, *finisher)),
            _ => None,
        })
        .collect()
}

fn refusals(events: &[Event]) -> Vec<kh1_core::Refused> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::AttackRefused(r) => Some(*r),
            _ => None,
        })
        .collect()
}

/// Steps both and checks position, facing, state, lock, the rows started and the refusals.
fn run_attack(p: &mut Pair, n: usize, pad: impl Fn(usize) -> Pad, hits_land: bool) -> Vec<(usize, usize, u8, bool)> {
    let mut started = Vec::new();
    for k in 0..n {
        let f = p.step(pad(k));
        let o = p.o_out.clone().unwrap();
        if std::env::var("DBG").is_ok() {
            eprintln!("{k}: ours {:.3} oracle {:.3} req {} oreq {} ev {:?} oev {:?}", f.ours.z, f.oracle.z, f.head, o.request, f.out.events, o.events);
        }
        assert!((f.oracle - f.ours).length() < 1e-3, "frame {k}: pos {:?} vs oracle {:?}", f.ours, f.oracle);
        assert!(wrap_angle(f.facing - f.o_facing).abs() < 1e-5, "frame {k}: facing {} vs {}", f.facing, f.o_facing);
        assert_eq!(f.state, f.o_state, "frame {k}: state");
        assert_eq!(f.lock, f.o_lock, "frame {k}: lock");
        assert_eq!(attack_rows(&f.out.events), attack_rows(&o.events), "frame {k}: rows");
        assert_eq!(refusals(&f.out.events), refusals(&o.events), "frame {k}: refusals");
        for (row, counter, fin) in attack_rows(&f.out.events) {
            started.push((k, row, counter, fin));
            if hits_land {
                // The swing hits: KH1's CB+0x50 and Re:CoM's +0x644.
                p.oracle.register_hit(1, 1);
                p.recom.hit = true;
            } else {
                p.recom.hit = false;
            }
        }
        if hits_land && attack_rows(&f.out.events).is_empty() && p.oracle.combo.hits != 0 {
            p.recom.hit = true;
        }
    }
    started
}

fn mash(k: usize) -> Pad {
    Pad { attack: k.is_multiple_of(2) && k > 2, ..Default::default() }
}

#[test]
fn ground_combo_matches_the_oracle() {
    let mut p = Pair::new(Abilities { combo_master: true, ..all() }, Floor::flat());
    let started = run_attack(&mut p, 260, mash, false);
    let rows: Vec<(usize, u8, bool)> = started.iter().map(|&(_, r, c, f)| (r, c, f)).collect();
    // Swing 1 (row 13), swing 2 (row 11, the alternation), the finisher (row 8), then a new combo.
    assert_eq!(&rows[..4], &[(13, 1, false), (11, 2, false), (8, 3, true), (13, 1, false)]);
    // Each chained swing starts the frame its combo window opens: swing 1's window is 22–40.
    assert!(started[1].0 - started[0].0 >= 23, "the window opens on the think after frame 22's update");
    // Re:CoM played the rows' clips (ids 1, 2, 4) with root motion, and ended the attack itself.
    let ids: Vec<u32> = p.recom.swings.iter().map(|s| s.recom_id).take(3).collect();
    assert_eq!(ids, vec![1, 2, 4]);
    assert!(p.recom.swings.iter().all(|s| s.root));
    assert!(p.recom.swing_ends >= 1);
}

#[test]
fn a_missed_swing_ends_the_combo_like_kh1() {
    // No Combo Master and nothing hit: the press in the window is refused (KH1's rule), and the
    // next swing only comes once the motion is over (a fresh combo).
    let mut p = Pair::new(all(), Floor::flat());
    let started = run_attack(&mut p, 120, mash, false);
    assert!(started.len() >= 2);
    assert_eq!(started[1].2, 1, "a fresh combo: {started:?}");
    // With the hits landing it chains.
    let mut p = Pair::new(all(), Floor::flat());
    let started = run_attack(&mut p, 120, mash, true);
    assert_eq!((started[1].1, started[1].2), (11, 2), "{started:?}");
}

#[test]
fn air_combo_matches_the_oracle() {
    // Jump, then mash Attack in the air: rows 18, 17, 16 with KH1's hops and air speeds.
    let mut p = Pair::new(Abilities { combo_master: true, ..all() }, Floor::flat());
    let pad = |k: usize| Pad { jump: k < 3, attack: k > 12 && k.is_multiple_of(2), stick: if k < 12 { [0.0, 1.0] } else { [0.0, 0.0] }, ..Default::default() };
    let started = run_attack(&mut p, 200, pad, false);
    let rows: Vec<usize> = started.iter().map(|s| s.1).collect();
    assert_eq!(&rows[..3], &[18, 17, 16], "{started:?}");
    assert!(p.recom.swings.iter().take(3).all(|s| !s.root), "the air swings have no root travel");
}

#[test]
fn landing_cuts_an_air_swing_like_kh1() {
    // A short hop and one air swing late in the fall: the landing replaces the swing.
    let mut p = Pair::new(Abilities { combo_master: true, ..all() }, Floor::flat());
    let pad = |k: usize| Pad { jump: k < 2, attack: k == 16, ..Default::default() };
    run_attack(&mut p, 120, pad, false);
    assert_eq!(p.recom.swings.len(), 1);
    assert_eq!(p.recom.swing_ends, 1);
    assert_eq!(p.recom.act.id, action::STAND);
}

#[test]
fn attack_and_jump_on_one_frame_jump() {
    let mut p = Pair::new(all(), Floor::flat());
    run_attack(&mut p, 10, |k| Pad { jump: k == 3, attack: k == 3, ..Default::default() }, false);
    assert!(p.recom.swings.is_empty());
}


#[test]
fn swing_and_roll_away_from_the_origin_keep_soras_position() {
    // 0.6.0's in-game traces: Sora idle at (−416.35, −750.35, 1363.55) (KH1 units); the first
    // Attack press put him at (10.6, −597.0, −34.7) and he fell through the floor. The same
    // happened on the Dodge Roll. Here: a non-origin start, a model position offset from the body
    // (Re:CoM's anchor), and Re:CoM's absolute root step through the delay.
    let start = Vec3::new(-416.35, 0.0, 1363.55);
    for roll in [false, true] {
        let mut p = Pair::new_at(Abilities { combo_master: true, ..all() }, Floor::flat(), start);
        p.recom.model_offset = Vec3::new(0.3, -12.0, 0.7);
        let pad = |k: usize| {
            if roll {
                Pad { square: k == 10, stick: if k <= 10 { [0.0, 1.0] } else { [0.0, 0.0] }, ..Default::default() }
            } else {
                Pad { attack: k == 10, ..Default::default() }
            }
        };
        let mut last = start;
        for k in 0..70 {
            let f = p.step(pad(k));
            assert!((f.oracle - f.ours).length() < 1e-3, "roll {roll} frame {k}: {:?} vs {:?}", f.ours, f.oracle);
            // Never more than a frame's travel (run 8, the roll's 17.8 root travel) per frame.
            assert!((f.ours - last).length() < 20.0, "roll {roll} frame {k}: jumped from {last:?} to {:?}", f.ours);
            assert_eq!(f.ours.y, 0.0, "roll {roll} frame {k}: left the floor");
            last = f.ours;
        }
        if !roll {
            // The swing's root travel only: KH1's 100 units of motion 200 (to frame 34 and still).
            assert!(((last - start).length() - 100.0).abs() < 1e-2, "{:?}", last - start);
        }
    }
}

/// One handler frame with an enemy in front (Re:CoM's target), no oracle (kh1-core's controller
/// has no clash yet; the numbers are KH1's from the decomp, `docs/recom-kh1-attack.md` § Guard).
fn step_with_target(p: &mut Pair, pad: Pad) -> FrameOut {
    let jump = Button::from_held(p.prev.jump, pad.jump);
    let square = Button::from_held(p.prev.square, pad.square);
    let attack = Button::from_held(p.prev.attack, pad.attack);
    p.prev = pad;
    let target = kh1_core::TargetPart { owner: 1, centre: Vec3::new(0.0, 0.0, 150.0), radius: 30.0 };
    let out = p.ours.frame(&mut p.recom, &FrameInput { move_dir: Vec3::ZERO, tilt: 0.0, jump, square, lock_target: None, attack, target: Some(target), locked: false }, p.first);
    p.first = false;
    p.recom.after_exe();
    out
}

#[test]
fn guard_success_recoils_and_offers_the_counterattack_from_frame_12() {
    let mut p = Pair::new(Abilities { counterattack: true, ..all() }, Floor::flat());
    for k in 0..5 {
        step_with_target(&mut p, Pad { square: k == 4, ..Default::default() });
    }
    assert_eq!(p.ours.rec.head(), crate::rows::GUARD);
    assert_eq!(p.recom.draws, 1, "Guard draws the Keyblade through Re:CoM's own function, once");
    // The guard object (and Re:CoM's body guard) is live on frames 10–36 of the Guard motion.
    let mut live_from = None;
    for k in 0..20 {
        step_with_target(&mut p, Pad::default());
        if p.recom.guard && live_from.is_none() {
            live_from = Some((k, p.ours.rec.frame));
        }
    }
    let (_, f) = live_from.expect("the body guard comes on");
    assert!((10.0..11.0).contains(&f), "on from the Guard motion's frame 10, got {f}");
    assert!(p.recom.guard, "still live inside 10–36");
    // A hit is guarded: the recoil 0x6E at once, no damage, the guard off.
    p.recom.block_next = true;
    let out = step_with_target(&mut p, Pad::default());
    assert!(out.events.contains(&Event::GuardSucceeded));
    assert_eq!(p.ours.rec.head(), crate::rows::RECOIL);
    assert!(p.recom.untouchable && !p.recom.guard);
    assert_eq!(p.recom.clinks, 1, "Re:CoM's guard spark plays once per success");
    // Before frame 12 of the recoil an Attack press is refused (no Counterattack yet).
    let out = step_with_target(&mut p, Pad { attack: true, ..Default::default() });
    assert!(out.events.contains(&Event::AttackRefused(kh1_core::Refused::NotAccepted)), "{:?}", out.events);
    step_with_target(&mut p, Pad::default());
    let mut started = None;
    for k in 0..20 {
        let out = step_with_target(&mut p, Pad { attack: k % 2 == 0, ..Default::default() });
        if let Some(e) = out.events.iter().find(|e| matches!(e, Event::AttackStarted { .. })) {
            started = Some((p.ours.rec.frame, e.clone()));
            break;
        }
    }
    let (_, e) = started.expect("the Counterattack starts");
    assert_eq!(e, Event::AttackStarted { row: crate::attack::COUNTERATTACK_ROW, request: 213, counter: 1, finisher: false });
    assert_eq!(p.recom.swings.last().unwrap().kh1_row, crate::attack::COUNTERATTACK_ROW);
    // Flag 0x80 counts 8 frames from the Counterattack's request, then ends.
    for _ in 0..7 {
        step_with_target(&mut p, Pad::default());
    }
    assert!(p.recom.untouchable);
    step_with_target(&mut p, Pad::default());
    step_with_target(&mut p, Pad::default());
    assert!(!p.recom.untouchable);
}

#[test]
fn without_the_ability_a_guard_success_offers_no_counterattack() {
    let mut p = Pair::new(Abilities { counterattack: false, ..all() }, Floor::flat());
    for k in 0..20 {
        step_with_target(&mut p, Pad { square: k == 4, ..Default::default() });
    }
    p.recom.block_next = true;
    step_with_target(&mut p, Pad::default());
    assert_eq!(p.ours.rec.head(), crate::rows::RECOIL);
    for k in 0..20 {
        let out = step_with_target(&mut p, Pad { attack: k % 2 == 0, ..Default::default() });
        assert!(!out.events.iter().any(|e| matches!(e, Event::AttackStarted { .. })), "frame {k}");
    }
}
