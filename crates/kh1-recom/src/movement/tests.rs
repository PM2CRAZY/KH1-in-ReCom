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
        }
    }

    /// Re:CoM after the state exe: motion advance (+ root motion), then the mover.
    fn after_exe(&mut self) {
        let step = self.step;
        if let Some(c) = &mut self.clip {
            if c.just {
                c.just = false;
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
                        let travel = (m.root_at(new) - m.root_at(old)).rotate_yaw(self.facing);
                        self.pos += travel;
                    }
                }
                c.time = new;
            }
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
}

#[derive(Clone, Copy, Default)]
struct Pad {
    stick: [f32; 2],
    jump: bool,
    square: bool,
}

struct Pair {
    oracle: Controller,
    world: Floor,
    ours: Kh1Sora,
    recom: FakeRecom,
    prev: Pad,
    first: bool,
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
        let params = Params::kh1_defaults();
        Pair {
            oracle: Controller::new(params.clone(), ab.clone(), Moveset::kh1_defaults(), Vec3::ZERO),
            world: floor,
            ours: Kh1Sora::new(params, ab),
            recom: FakeRecom::new(floor),
            prev: Pad::default(),
            first: true,
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
        self.prev = pad;
        let o = self.oracle.step(&Input { move_dir: dir, tilt, jump, square, ..Default::default() }, &[], &mut self.world, step);
        let out = self.ours.frame(&mut self.recom, &FrameInput { move_dir: dir, tilt, jump, square, lock_target: None }, self.first);
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
    let frames = p.run_same(160, |k| Pad { jump: k < 2, square: k > 20, stick: [0.0, 1.0] });
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
fn dodge_roll_travels_350_one_frame_ahead_of_kh1() {
    // Running, Square: Dodge Roll (Guard is equipped, so it needs the run tilt).
    let mut p = Pair::new(all(), Floor::flat());
    let pad = |k: usize| Pad { square: k == 10, stick: if k <= 10 { [0.0, 1.0] } else { [0.0, 0.0] }, ..Default::default() };
    let mut frames = Vec::new();
    for k in 0..80 {
        frames.push(p.step(pad(k)));
    }
    assert!(frames[10].out.events.contains(&Event::Rolled));
    for (k, f) in frames.iter().enumerate() {
        assert_eq!(f.state, f.o_state, "frame {k}");
        assert_eq!(f.lock, f.o_lock, "frame {k}");
    }
    // Root motion lands in Re:CoM's motion update of the same frame; KH1 applies it in the next
    // frame's physics: Re:CoM is one frame of root travel ahead, and the same in the end.
    for k in 10..60 {
        let lead = frames[k].ours.z - frames[k].oracle.z;
        let ahead = frames[k + 1].oracle.z - frames[k].oracle.z;
        assert!((lead - ahead).abs() < 1e-2, "frame {k}: Re:CoM leads by {lead}, KH1's next step is {ahead}");
    }
    assert!((frames[79].ours - frames[79].oracle).length() < 1e-3);
    // 350 units of root travel plus the run frame before and the roll's first frame at speed 8.
    let travel = frames[79].ours.z - frames[9].ours.z;
    assert!((travel - (350.0 + 8.0)).abs() < 1e-2, "{travel}");
    // KH1's 12 no-flinch frames reach Re:CoM's roll flag.
    assert!(!p.recom.no_flinch);
    assert_eq!(p.recom.act.id, action::STAND);
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
        let pad = Pad { jump: (10..30).contains(&k) || k > 70, square: k == 50, stick: [0.3, 1.0] };
        let f = p.step_dt(pad, step);
        assert!((f.oracle - f.ours).length() < 1e-3, "frame {k}: {:?} vs {:?}", f.ours, f.oracle);
        assert_eq!((f.state, f.lock), (f.o_state, f.o_lock), "frame {k}");
    }
}
