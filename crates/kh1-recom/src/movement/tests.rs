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
    /// 0.183.1: the guard side's KH1 clash record for the next block.
    clash_next: Option<(u8, i32)>,
    untouchable: bool,
    clinks: u32,
    draws: u32,
    /// M7 stage 7.4: KH1's MP test, the casts Re:CoM started, released and ended.
    mp_ok: bool,
    casts: Vec<crate::magic::CastStart>,
    releases: Vec<(crate::magic::CastStart, u64)>,
    cast_ends: usize,
    /// Re:CoM's battle is won (`player+0x428 & 4`): its end action enters the win action 0x1C.
    win_pending: bool,
    /// M8 stage 8a.1: the item uses Re:CoM started and its item actions ended; a slot that no
    /// longer holds the item refuses.
    items: Vec<crate::items::ItemCommand>,
    item_ends: usize,
    item_missing: bool,
    /// M9 stage 9c.1: the specials Re:CoM started, their rows and its sleight actions ended.
    specials: Vec<crate::specials::SpecialStart>,
    special_rows: Vec<crate::specials::RowStart>,
    special_ends: usize,
    /// M12 step 7: special rows whose attack ended (Strike Raid's throws).
    special_row_ends: usize,
    /// M12 step 7: Ragnarok's releases.
    ragnarok_fires: Vec<crate::specials::ragnarok::FireStart>,
    /// `dodge_roll = recom`: Re:CoM's own avoid init runs (its roll), the order of Re:CoM's calls
    /// around it, and how often our frame wrote Re:CoM's untouchable bit.
    recom_rolls: usize,
    order: Vec<&'static str>,
    untouchable_writes: u32,
    /// M11, the shared enemy-guard step: a clash to report (KH1's kind, the priority difference).
    swing_clash: Option<(u8, i32)>,
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
            ext: Extension::plan(|_| true).with_xl(|_| true),
            motions: {
                let mut m = Moveset::kh1_defaults().motions;
                crate::magic::add_motions(&mut m);
                crate::items::add_motions(&mut m);
                crate::specials::add_motions(&mut m);
                m
            },
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
            clash_next: None,
            untouchable: false,
            clinks: 0,
            draws: 0,
            mp_ok: true,
            casts: Vec::new(),
            releases: Vec::new(),
            cast_ends: 0,
            win_pending: false,
            items: Vec::new(),
            item_ends: 0,
            item_missing: false,
            specials: Vec::new(),
            special_rows: Vec::new(),
            special_ends: 0,
            special_row_ends: 0,
            ragnarok_fires: Vec::new(),
            recom_rolls: 0,
            order: Vec::new(),
            untouchable_writes: 0,
            swing_clash: None,
        }
    }

    /// Re:CoM's end action `0x1401F6660`: the win action when the win is pending (`0x1401F6794` →
    /// `0x1401F1B20`, which also stops the motion), else the idle.
    fn end_action(&mut self) -> bool {
        if self.win_pending {
            self.act = Action { id: crate::battle::end::WIN_ACTION, sub: 2, phase: 3 };
            self.clip = None;
            return true;
        }
        self.act = Action { id: action::STAND, sub: 0, phase: 0 };
        false
    }

    /// Re:CoM's root step as it is: the motion update makes an absolute position (the model's
    /// position + the rotated travel, `0x14024C960`), our wrapper on `0x14024DCA0` delays it, and
    /// the entity task moves the body to it (`P + mask∘(out − P)`, mask (1, 1, 1) here).
    fn root_step(&mut self, local: Vec3) {
        let anchor = self.pos + self.model_offset;
        let root = anchor + local.rotate_yaw(self.facing);
        let out = self.root_delay.position(1, self.tick, anchor, root, ROOT_DELAY_MAX_GAP / crate::units::METRES_PER_UNIT);
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
    fn guard_clash(&mut self) -> Option<(u8, i32)> {
        self.clash_next.take()
    }
    fn take_swing_clash(&mut self) -> Option<(u8, i32)> {
        self.swing_clash.take()
    }
    fn draw_weapon(&mut self) {
        self.draws += 1;
    }
    fn set_untouchable(&mut self, on: bool) {
        self.untouchable = on;
        self.untouchable_writes += 1;
    }
    fn end_swing(&mut self, _airborne: bool) -> bool {
        self.swing_ends += 1;
        self.order.push("end swing");
        self.end_action()
    }
    fn start_recom_roll(&mut self) {
        // Re:CoM's avoid init `0x1401F10C0`: its untouchable bit, action 6 sub 0, the body's
        // velocity zeroed, row 0x16 with root motion (Re:CoM's own; not modelled here).
        self.recom_rolls += 1;
        self.order.push("Re:CoM's roll");
        self.untouchable = true;
        self.act = Action { id: action::AVOID, sub: action::SUB_ROLL, phase: phase::FALL };
        self.vel = Vec3::ZERO;
        self.clip = None;
    }
    fn can_cast(&mut self, _spell: u8, _level: u8) -> bool {
        self.mp_ok
    }
    fn start_cast(&mut self, c: &crate::magic::CastStart) -> bool {
        // Our cast init: Re:CoM's magic action (the KH1 frame then plays the clips).
        self.act = Action { id: crate::magic::RECOM_MAGIC_ACTION, sub: crate::magic::recom_attack_id(c.spell), phase: crate::magic::MAGIC_PHASE };
        self.casts.push(*c);
        true
    }
    fn release_cast(&mut self, c: &crate::magic::CastStart) -> Option<u32> {
        self.releases.push((*c, self.tick));
        Some(100 + self.releases.len() as u32)
    }
    fn end_cast(&mut self, _airborne: bool) -> bool {
        self.cast_ends += 1;
        self.end_action()
    }
    fn start_item(&mut self, c: &crate::items::ItemCommand) -> bool {
        if self.item_missing {
            return false;
        }
        self.act = Action { id: crate::items::RECOM_ITEM_ACTION, sub: crate::items::ITEM_SUB, phase: crate::items::ITEM_PHASE };
        self.items.push(*c);
        true
    }
    fn end_item(&mut self, _airborne: bool) -> bool {
        self.item_ends += 1;
        self.end_action()
    }
    fn battle_won(&mut self) -> bool {
        self.win_pending
    }
    fn start_special(&mut self, s: &crate::specials::SpecialStart) -> bool {
        if !self.mp_ok {
            return false;
        }
        self.act = Action { id: crate::specials::RECOM_ACTION, sub: crate::specials::RECOM_SUB, phase: crate::specials::RECOM_PHASE };
        self.specials.push(*s);
        true
    }
    fn special_row_end(&mut self) {
        self.special_row_ends += 1;
    }

    fn ragnarok_fire(&mut self, f: &crate::specials::ragnarok::FireStart) {
        self.ragnarok_fires.push(*f);
    }

    fn special_row(&mut self, r: &crate::specials::RowStart) {
        self.special_rows.push(*r);
    }
    fn end_special(&mut self, _airborne: bool) -> bool {
        self.special_ends += 1;
        self.end_action()
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
    /// A magic command for the next frame (M7 stage 7.4; the oracle has none).
    magic: Option<crate::magic::MagicCommand>,
    /// A special command for the next frame (M9 stage 9c.1; the oracle has none).
    special: Option<u8>,
    /// Re:CoM's attack target for every frame (M12 step 7; None by default).
    target: Option<kh1_core::TargetPart>,
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
            magic: None,
            special: None,
            target: None,
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
        let out = self.ours.frame(&mut self.recom, &FrameInput { move_dir: dir, tilt, recom_tilt: tilt, jump, square, lock_target: None, attack, target: self.target, locked: false, magic: self.magic.take(), item: None, special: self.special.take() }, self.first);
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

/// Enemy Skills (Bandit's Combo Finish): length 1, so every swing is the combo's finisher, with KH1's
/// own picker (the oracle runs KH1's controller on the same abilities).
#[test]
fn combo_finish_makes_every_swing_a_finisher() {
    let mut p = Pair::new(Abilities { combo_master: true, combo_finish: true, ..all() }, Floor::flat());
    let started = run_attack(&mut p, 260, mash, false);
    assert!(started.len() >= 2, "{started:?}");
    assert!(started.iter().all(|&(_, _, c, f)| c == 1 && f), "{started:?}");
    assert_eq!(started[0].1, 8, "KH1's ground finisher row");
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
    let out = p.ours.frame(&mut p.recom, &FrameInput { move_dir: Vec3::ZERO, tilt: 0.0, recom_tilt: 0.0, jump, square, lock_target: None, attack, target: Some(target), locked: false, magic: None, item: None, special: None }, p.first);
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

/// M11, the shared enemy-guard step: a swing that met an enemy's guard row ends and plays KH1's
/// clash (`0x1402B4BC0` → `0x14029BBD0`): kind 6 on the ground the recoil 0x6E (the Counterattack
/// offered from its frame 12), kind 7 the strong recoil 0x6F; flag 0x80 for 8 frames; CB+0x41 − 1.
#[test]
fn a_swing_on_a_guard_row_clashes() {
    for (kind, recoil) in [(6u8, crate::rows::RECOIL), (7, crate::rows::STRONG_RECOIL)] {
        let mut p = Pair::new(Abilities { counterattack: true, ..all() }, Floor::flat());
        for k in 0..4 {
            step_with_target(&mut p, Pad { attack: k == 2, ..Default::default() });
        }
        assert!(p.ours.swing.is_some(), "a swing plays");
        let counter = p.ours.combo_counter;
        p.recom.swing_clash = Some((kind, 0));
        let ends = p.recom.swing_ends;
        let out = step_with_target(&mut p, Pad::default());
        assert!(out.events.contains(&Event::SwingClashed { kind }), "{:?}", out.events);
        assert_eq!(p.recom.swing_ends, ends + 1, "Re:CoM's attack action ended");
        assert!(p.ours.swing.is_none() && p.ours.lock && p.ours.clashed);
        assert_eq!(p.ours.rec.head(), recoil);
        assert_eq!(p.ours.combo_counter, counter.saturating_sub(1));
        assert!(p.recom.untouchable, "KH1's flag 0x80");
        assert!(p.ours.push.is_none(), "no push at a difference of 0");
        // The Counterattack only during the kind-6 recoil 0x6E (`0x140294220`).
        let mut started = false;
        for k in 0..30 {
            let out = step_with_target(&mut p, Pad { attack: k % 2 == 0, ..Default::default() });
            started |= out.events.iter().any(|e| matches!(e, Event::AttackStarted { row, .. } if *row == crate::attack::COUNTERATTACK_ROW));
        }
        assert_eq!(started, kind == 6, "kind {kind}");
    }
}

/// A kind-6 clash under the row's priority pushes Sora back by −diff × 2.5 (`0x14029D720`).
#[test]
fn a_clash_under_the_rows_priority_pushes_back() {
    let mut p = Pair::new(all(), Floor::flat());
    for k in 0..4 {
        step_with_target(&mut p, Pad { attack: k == 2, ..Default::default() });
    }
    let from = p.recom.pos;
    p.recom.swing_clash = Some((6, -29));
    step_with_target(&mut p, Pad::default());
    assert!(p.ours.push.is_some());
    for _ in 0..40 {
        step_with_target(&mut p, Pad::default());
    }
    assert!(p.ours.push.is_none(), "the slide ends");
    // 72.5 in all along −facing (the swing's own root travel aside: a sum of 0.8^k).
    let back = (from - p.recom.pos).z;
    assert!(back > 40.0, "pushed back: {back}");
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

// ---- M7 stage 7.4: the cardless cast and Fire (`crate::magic`) ----

/// One handler frame without the oracle (kh1-core's controller has no magic), with an optional
/// magic command; returns the frame's output.
fn step_cast(p: &mut Pair, pad: Pad, magic: Option<crate::magic::MagicCommand>) -> FrameOut {
    let (dir, tilt) = Input::move_from_stick(pad.stick, 0.0);
    let jump = Button::from_held(p.prev.jump, pad.jump);
    let square = Button::from_held(p.prev.square, pad.square);
    let attack = Button::from_held(p.prev.attack, pad.attack);
    p.prev = pad;
    let out = p.ours.frame(&mut p.recom, &FrameInput { move_dir: dir, tilt, recom_tilt: tilt, jump, square, lock_target: None, attack, target: None, locked: false, magic, item: None, special: None }, p.first);
    p.first = false;
    p.recom.after_exe();
    out
}

const FIRE1: crate::magic::MagicCommand = crate::magic::MagicCommand { spell: 0, level: 1 };

fn released(out: &FrameOut) -> bool {
    out.cast.iter().any(|e| matches!(e, crate::magic::CastEvent::Released { .. }))
}

#[test]
fn ground_fire_releases_at_28_and_frees_sora_at_46_of_the_release_motion() {
    use crate::magic::{cast_req, CastEvent, RECOM_MAGIC_ACTION};
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    for _ in 0..5 {
        step_cast(&mut p, Pad::default(), None);
    }
    let out = step_cast(&mut p, Pad::default(), Some(FIRE1));
    assert_eq!(out.cast.first(), Some(&CastEvent::Started { spell: 0, level: 1, fast: false, air: false }));
    assert_eq!(p.ours.rec.head(), cast_req::FIRE_WINDUP);
    assert_eq!(p.recom.act.id, RECOM_MAGIC_ACTION, "Re:CoM's magic action runs the cast");
    assert_eq!(p.recom.draws, 1, "the cast draws the Keyblade through Re:CoM's own function, once, at the command");
    assert_eq!(p.recom.plays.last().map(|x| x.0), Some(cast_req::FIRE_WINDUP));
    // KH1's script: the ball leaves in the frame the wind-up (28 frames) ends; our handler sees
    // that frame's motion update (and the script run after it) at its start, one handler on.
    let mut rel = None;
    for k in 1..40 {
        let out = step_cast(&mut p, Pad::default(), None);
        if let Some(CastEvent::Released { frame, .. }) = out.cast.iter().find(|e| matches!(e, CastEvent::Released { .. })) {
            rel = Some((k, *frame));
            break;
        }
        assert!(p.recom.releases.is_empty(), "frame {k}");
    }
    assert_eq!(rel, Some((29, 28.0)), "released in KH1's frame 28 after the command");
    assert_eq!(p.ours.rec.head(), cast_req::FIRE_RELEASE);
    assert_eq!(p.recom.releases.len(), 1);
    assert_eq!(p.recom.act.id, RECOM_MAGIC_ACTION);
    // Locked until the release motion's free key (46), then the queued idle at its end (60).
    let mut free_at = None;
    for k in 1..80 {
        step_cast(&mut p, Pad::default(), None);
        if free_at.is_none() && !p.ours.lock {
            free_at = Some(k);
        }
    }
    assert!(matches!(free_at, Some(46) | Some(47)), "free near frame 46 of the release motion: {free_at:?}");
    assert_eq!(p.recom.cast_ends, 1, "Re:CoM's magic action ended once, at the motion's end");
    assert_eq!(p.recom.act.id, action::STAND);
    assert!(p.ours.cast.is_none());
}

#[test]
fn fire_rechains_from_frame_14_of_the_release_with_the_ball_8_frames_later() {
    use crate::magic::CastEvent;
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_cast(&mut p, Pad::default(), None);
    step_cast(&mut p, Pad::default(), Some(FIRE1));
    let mut k = 1;
    while !released(&step_cast(&mut p, Pad::default(), None)) {
        k += 1;
    }
    assert_eq!(k, 29);
    // Fire every frame from here: refused until the release motion's frame 14.
    let mut chained = None;
    for j in 1..30 {
        let out = step_cast(&mut p, Pad::default(), Some(FIRE1));
        if out.cast.iter().any(|e| matches!(e, CastEvent::Started { fast: true, .. })) {
            chained = Some(j);
            break;
        }
        assert!(out.cast.iter().any(|e| matches!(e, CastEvent::Refused { why: "not now", .. })), "frame {j}: {:?}", out.cast);
    }
    assert_eq!(chained, Some(14), "the re-cast is taken at frame 14 of the release motion");
    // No wind-up: the release motion again, and the ball 8 frames after the command.
    assert_eq!(p.ours.rec.head(), crate::magic::cast_req::FIRE_RELEASE);
    let mut n = 0;
    loop {
        n += 1;
        let out = step_cast(&mut p, Pad::default(), None);
        if let Some(CastEvent::Released { frame, .. }) = out.cast.iter().find(|e| matches!(e, CastEvent::Released { .. })) {
            assert_eq!(*frame, 8.0);
            break;
        }
        assert!(n < 20);
    }
    assert_eq!(n, 9);
    assert_eq!(p.recom.releases.len(), 2);
    assert!(p.recom.casts[1].fast);
    assert_eq!(p.recom.cast_ends, 0, "a re-cast keeps Re:CoM's magic action");
    // Chained Fires: each command plays the release motion anew, so the next is taken 15 frames
    // after the last (its motion's update at 14 is seen the frame after).
    let mut j = 0;
    loop {
        j += 1;
        let out = step_cast(&mut p, Pad::default(), Some(FIRE1));
        if out.cast.iter().any(|e| matches!(e, CastEvent::Started { fast: true, .. })) {
            break;
        }
        assert!(j < 30);
    }
    assert_eq!(n + j, 15, "the third Fire 15 frames after the second");
}

#[test]
fn fire_from_swing_1s_window_skips_the_wind_up() {
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_cast(&mut p, Pad::default(), None);
    // Swing 1 (row 13, request 200, combo window 22–40).
    step_cast(&mut p, Pad { attack: true, ..Default::default() }, None);
    assert_eq!(p.ours.rec.head(), 200);
    p.recom.hit = true;
    let mut started = None;
    for j in 1..45 {
        let out = step_cast(&mut p, Pad::default(), Some(FIRE1));
        if let Some(e) = out.cast.iter().find(|e| matches!(e, crate::magic::CastEvent::Started { .. })) {
            started = Some((j, e.clone(), p.ours.rec.frame));
            break;
        }
    }
    let (_, e, _) = started.expect("Fire taken in swing 1's window");
    assert_eq!(e, crate::magic::CastEvent::Started { spell: 0, level: 1, fast: true, air: false });
    assert_eq!(p.ours.rec.head(), crate::magic::cast_req::FIRE_RELEASE);
    assert_eq!(p.recom.swing_ends, 1, "Re:CoM's attack action ended before the cast");
    assert_eq!(p.recom.act.id, crate::magic::RECOM_MAGIC_ACTION);
}

#[test]
fn an_interrupted_cast_casts_nothing_and_pays_nothing() {
    use crate::magic::CastEvent;
    // Re:CoM ran something else (a hit reaction) before the release.
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_cast(&mut p, Pad::default(), None);
    step_cast(&mut p, Pad::default(), Some(FIRE1));
    for _ in 0..10 {
        step_cast(&mut p, Pad::default(), None);
    }
    let out = p.ours.frame(&mut p.recom, &FrameInput::default(), true);
    assert!(out.cast.iter().any(|e| matches!(e, CastEvent::Interrupted { .. })), "{:?}", out.cast);
    assert!(p.recom.releases.is_empty());
    assert!(p.ours.cast.is_none());
    // An air cast that lands before its release: the landing replaces the cast motion.
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_cast(&mut p, Pad::default(), None);
    step_cast(&mut p, Pad { jump: true, ..Default::default() }, None);
    let mut cast_at = None;
    for k in 0..80 {
        let falling = p.ours.state == state::AIR && !p.ours.arc.rising() && p.recom.pos.y > -40.0;
        let m = (falling && cast_at.is_none()).then_some(FIRE1);
        let out = step_cast(&mut p, Pad::default(), m);
        if m.is_some() {
            assert!(out.cast.iter().any(|e| matches!(e, CastEvent::Started { air: true, fast: false, .. })), "{:?}", out.cast);
            cast_at = Some(k);
        }
    }
    assert!(cast_at.is_some());
    assert!(p.recom.releases.is_empty(), "landed before the release: no ball");
    assert_eq!(p.recom.cast_ends, 1);
    // Without MP for the cost the command does nothing.
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    p.recom.mp_ok = false;
    step_cast(&mut p, Pad::default(), None);
    let out = step_cast(&mut p, Pad::default(), Some(FIRE1));
    assert!(out.cast.iter().any(|e| matches!(e, CastEvent::Refused { why: "mp", .. })));
    assert!(p.recom.casts.is_empty());
}

#[test]
fn an_air_cast_hops_30_and_halves_the_air_speed() {
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_cast(&mut p, Pad::default(), None);
    step_cast(&mut p, Pad { jump: true, stick: [0.0, 1.0], ..Default::default() }, None);
    for _ in 0..10 {
        step_cast(&mut p, Pad { jump: true, stick: [0.0, 1.0], ..Default::default() }, None);
    }
    assert_eq!(p.ours.state, state::AIR);
    let v = p.ours.air_vel.flat().length();
    let y = p.recom.pos.y;
    step_cast(&mut p, Pad::default(), Some(FIRE1));
    // × 0.5 at the command, then the frame's air drag with no stick (`0x1402AF6A0`).
    let want = v * crate::magic::AIR_SPEED_SCALE * p.ours.params.air_drag.sqrt();
    assert!((p.ours.air_vel.flat().length() - want).abs() < 1e-3, "{} vs {want}", p.ours.air_vel.flat().length());
    assert_eq!(p.ours.rec.head(), crate::magic::cast_req::FIRE_AIR_WINDUP);
    assert!((p.ours.arc.rise + crate::magic::AIR_HOP).abs() < 1e-3 && (p.ours.arc.y0 - y).abs() < 1e-3, "a 30 cm arc from here: {:?}", p.ours.arc);
    assert_eq!(p.recom.act.id, crate::magic::RECOM_MAGIC_ACTION);
}

// ---- M7 stage 7.5: Blizzard ----

const BLIZZARD1: crate::magic::MagicCommand = crate::magic::MagicCommand { spell: 1, level: 1 };

#[test]
fn ground_blizzard_releases_at_28_and_frees_sora_at_68() {
    use crate::magic::{cast_req, CastEvent, RECOM_MAGIC_ACTION};
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    for _ in 0..5 {
        step_cast(&mut p, Pad::default(), None);
    }
    let out = step_cast(&mut p, Pad::default(), Some(BLIZZARD1));
    assert_eq!(out.cast.first(), Some(&CastEvent::Started { spell: 1, level: 1, fast: false, air: false }));
    assert_eq!(p.ours.rec.head(), cast_req::BLIZZARD);
    assert_eq!(p.recom.act.id, RECOM_MAGIC_ACTION);
    // `L0086e(28)`: the shards leave 28 frames after the command (seen one handler on).
    let mut rel = None;
    for k in 1..40 {
        let out = step_cast(&mut p, Pad::default(), None);
        if let Some(CastEvent::Released { frame, .. }) = out.cast.iter().find(|e| matches!(e, CastEvent::Released { .. })) {
            rel = Some((k, *frame));
            break;
        }
    }
    assert_eq!(rel, Some((29, 28.0)));
    assert_eq!(p.recom.releases.len(), 1);
    assert_eq!(p.ours.rec.head(), cast_req::BLIZZARD, "still the cast motion (no second motion)");
    // Locked until the motion's free key (68), Re:CoM's action ended at its end (86).
    let mut free_at = None;
    for k in 30..100 {
        step_cast(&mut p, Pad::default(), None);
        if free_at.is_none() && !p.ours.lock {
            free_at = Some(k);
        }
    }
    assert!(matches!(free_at, Some(68) | Some(69)), "free near frame 68: {free_at:?}");
    assert_eq!(p.recom.cast_ends, 1);
    assert!(p.ours.cast.is_none());
}

#[test]
fn blizzard_is_never_a_fast_cast_and_a_cut_cast_pays_nothing() {
    use crate::magic::CastEvent;
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_cast(&mut p, Pad::default(), None);
    step_cast(&mut p, Pad::default(), Some(BLIZZARD1));
    // Re:CoM runs something else at frame 10 (a hit): the next KH1 frame drops the cast.
    for _ in 0..10 {
        step_cast(&mut p, Pad::default(), None);
    }
    let out = p.ours.frame(&mut p.recom, &FrameInput::default(), true);
    assert!(out.cast.iter().any(|e| matches!(e, CastEvent::Interrupted { .. })), "{:?}", out.cast);
    for _ in 0..40 {
        assert!(!released(&step_cast(&mut p, Pad::default(), None)));
    }
    assert!(p.recom.releases.is_empty(), "no shards, no MP");
    // While a Blizzard plays (before its free key) a Fire is refused: only Fire's own release
    // motion takes a re-cast.
    step_cast(&mut p, Pad::default(), Some(BLIZZARD1));
    for _ in 0..40 {
        let out = step_cast(&mut p, Pad::default(), Some(FIRE1));
        assert!(!out.cast.iter().any(|e| matches!(e, CastEvent::Started { .. })));
    }
}

// ---- M7 stage 7.6: Thunder ----

#[test]
fn ground_thunder_releases_at_30_and_locks_sora_to_68() {
    use crate::magic::{cast_req, CastEvent, MagicCommand};
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    for _ in 0..5 {
        step_cast(&mut p, Pad::default(), None);
    }
    let out = step_cast(&mut p, Pad::default(), Some(MagicCommand { spell: 2, level: 1 }));
    assert_eq!(out.cast.first(), Some(&CastEvent::Started { spell: 2, level: 1, fast: false, air: false }));
    assert_eq!(p.ours.rec.head(), cast_req::THUNDER);
    let mut rel = None;
    let mut free_at = None;
    for k in 1..100 {
        let out = step_cast(&mut p, Pad::default(), None);
        if let Some(CastEvent::Released { frame, .. }) = out.cast.iter().find(|e| matches!(e, CastEvent::Released { .. })) {
            rel = Some((k, *frame));
        }
        if free_at.is_none() && !p.ours.lock {
            free_at = Some(k);
        }
    }
    assert_eq!(rel, Some((31, 30.0)), "`L00c8e(30)`: the release 30 frames after the command");
    assert_eq!(p.recom.releases.len(), 1);
    // No free key: locked to the motion's end (68), then idle; Re:CoM's action ended once.
    assert!(matches!(free_at, Some(68) | Some(69)), "{free_at:?}");
    assert_eq!(p.recom.cast_ends, 1);
}

/// The owner's log of 2026-10-31 (0.24.2): the last enemies fell to a Thundaga's bolts while
/// Sora was locked in its 68-frame cast. Re:CoM's end action at the cast's end enters its win
/// action (`0x1401F6794` → `0x1401F1B20`); the KH1 frame must leave it to Re:CoM (no idle action,
/// no idle motion over the win pose).
#[test]
fn a_cast_that_ends_after_the_win_yields_to_re_coms_win_action() {
    use crate::battle::end::WIN_ACTION;
    use crate::magic::MagicCommand;
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    for _ in 0..5 {
        step_cast(&mut p, Pad::default(), None);
    }
    step_cast(&mut p, Pad::default(), Some(MagicCommand { spell: 2, level: 3 }));
    // The last bolt kills the last enemy mid-cast: Re:CoM's win is pending from then on.
    for _ in 0..40 {
        step_cast(&mut p, Pad::default(), None);
    }
    p.recom.win_pending = true;
    let mut took = None;
    for k in 0..60 {
        let plays = p.recom.plays.len();
        let out = step_cast(&mut p, Pad::default(), None);
        if out.recom_took {
            took = Some(k);
            assert_eq!(p.recom.plays.len(), plays, "no KH1 motion over Re:CoM's win pose");
            break;
        }
    }
    assert!(took.is_some(), "the cast's end entered Re:CoM's win action");
    assert_eq!(p.recom.cast_ends, 1);
    assert_eq!(p.recom.act.id, WIN_ACTION, "the KH1 frame left Re:CoM's win action alone");
    assert!(p.ours.cast.is_none() && p.ours.swing.is_none());
    // A swing ended by the win, the same.
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    for _ in 0..3 {
        step_cast(&mut p, Pad::default(), None);
    }
    step_cast(&mut p, Pad { attack: true, ..Default::default() }, None);
    assert!(p.ours.swing.is_some());
    p.recom.win_pending = true;
    let mut took = false;
    for _ in 0..90 {
        if step_cast(&mut p, Pad::default(), None).recom_took {
            took = true;
            break;
        }
    }
    assert!(took);
    assert_eq!(p.recom.swing_ends, 1);
    assert_eq!(p.recom.act.id, WIN_ACTION);
    // Without a won battle nothing changes: the cast ends into the idle as before.
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_cast(&mut p, Pad::default(), None);
    step_cast(&mut p, Pad::default(), Some(MagicCommand { spell: 2, level: 3 }));
    for _ in 0..100 {
        assert!(!step_cast(&mut p, Pad::default(), None).recom_took);
    }
    assert_eq!(p.recom.cast_ends, 1);
    assert_ne!(p.recom.act.id, WIN_ACTION);
}

/// The owner's 0.47.0 log: the last enemy fell to a fast Firaga and another Firaga was cast after
/// "battle end: started" (shot 670). Re:CoM starts no card once its battle is won (its card-use
/// test `0x1401E9180` answers no while `player+0x428 & 4`); the KH1 frame takes no new command
/// then: the Fire re-cast, Attack and items are refused, the running cast ends as it does and
/// Re:CoM's end action enters the win.
#[test]
fn no_new_kh1_command_once_re_coms_battle_is_won() {
    use crate::battle::end::WIN_ACTION;
    use crate::magic::CastEvent;
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_cast(&mut p, Pad::default(), None);
    step_cast(&mut p, Pad::default(), Some(FIRE1));
    while !released(&step_cast(&mut p, Pad::default(), None)) {}
    // The ball kills the last enemy: Re:CoM's win is pending from here on.
    p.recom.win_pending = true;
    let mut took = false;
    for k in 0..160 {
        // A fresh Attack press and a Fire command every other frame.
        let press = k % 2 == 0;
        let out = step_cast(&mut p, Pad { attack: press, ..Default::default() }, press.then_some(FIRE1));
        assert!(!out.cast.iter().any(|e| matches!(e, CastEvent::Started { .. })), "{:?}", out.cast);
        if out.recom_took {
            took = true;
            break;
        }
        if press {
            assert!(out.cast.iter().any(|e| matches!(e, CastEvent::Refused { why: "battle won", .. })), "{:?}", out.cast);
        }
        assert!(p.ours.swing.is_none(), "no swing after the win");
    }
    assert!(took, "the running cast ended into Re:CoM's win");
    assert_eq!(p.recom.casts.len(), 1, "no re-cast after the win");
    assert_eq!(p.recom.releases.len(), 1);
    assert_eq!(p.recom.cast_ends, 1);
    assert_eq!(p.recom.act.id, WIN_ACTION);
    // An item command once the battle is won: refused, no item action.
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    for _ in 0..5 {
        step_item(&mut p, Pad::default(), None);
    }
    p.recom.win_pending = true;
    let out = step_item(&mut p, Pad::default(), Some(POTION0));
    assert!(out.items.iter().any(|e| matches!(e, crate::items::ItemEvent::Refused { why: "battle won", .. })), "{:?}", out.items);
    assert!(p.recom.items.is_empty());
    // Before the win the same commands are taken as before.
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_cast(&mut p, Pad::default(), None);
    let out = step_cast(&mut p, Pad::default(), Some(FIRE1));
    assert!(out.cast.iter().any(|e| matches!(e, CastEvent::Started { .. })));
}

// ---- M7 stage 7.4c: the Fire release's root curve ----

#[test]
fn fire_release_steps_back_15_then_forward_to_10_like_kh1() {
    use crate::magic::cast_req;
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_cast(&mut p, Pad::default(), None);
    step_cast(&mut p, Pad::default(), Some(FIRE1));
    while p.ours.rec.head() != cast_req::FIRE_RELEASE {
        step_cast(&mut p, Pad::default(), None);
    }
    // Sora faces +z (yaw 0); the release steps him back along −z. Per-frame travel of Re:CoM's
    // root step (delayed one update, decision 17) while the release motion plays.
    let mut steps = Vec::new();
    let mut last = p.recom.pos.z;
    for _ in 0..62 {
        step_cast(&mut p, Pad::default(), None);
        steps.push(p.recom.pos.z - last);
        last = p.recom.pos.z;
    }
    // KH1's entry 57 (`kh1_mset.py`): 1.5 cm back a frame over frames 0–10, then 0.1 cm forward a
    // frame to −10 cm at 60. The replay hands the travel of frames 1–9 as 9 steps of −1.5, then
    // 0.1 forward for the rest (the old curve stood still after frame 10).
    let back = steps.iter().filter(|d| (**d + 1.5).abs() < 1e-3).count();
    let fwd = steps.iter().filter(|d| (**d - 0.1).abs() < 1e-3).count();
    assert!(back >= 9, "{steps:?}");
    assert!(fwd >= 48, "{steps:?}");
    let total: f32 = steps.iter().sum();
    assert!(total > -11.0 && total < -8.0, "{total}");
}

// ---- M7 stage 7.7: Cure ----

const CURE1: crate::magic::MagicCommand = crate::magic::MagicCommand { spell: 3, level: 1 };

#[test]
fn ground_cure_releases_at_36_and_locks_sora_to_68() {
    use crate::magic::{cast_req, CastEvent};
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    for _ in 0..5 {
        step_cast(&mut p, Pad::default(), None);
    }
    let out = step_cast(&mut p, Pad::default(), Some(CURE1));
    assert_eq!(out.cast.first(), Some(&CastEvent::Started { spell: 3, level: 1, fast: false, air: false }));
    assert_eq!(p.ours.rec.head(), cast_req::CURE);
    assert_eq!(p.recom.act.id, crate::magic::RECOM_MAGIC_ACTION);
    let mut rel = None;
    let mut free_at = None;
    for k in 1..100 {
        let out = step_cast(&mut p, Pad::default(), None);
        if let Some(CastEvent::Released { frame, .. }) = out.cast.iter().find(|e| matches!(e, CastEvent::Released { .. })) {
            rel = Some((k, *frame));
        }
        if free_at.is_none() && !p.ours.lock {
            free_at = Some(k);
        }
    }
    // `L004e8(36)`: the heal 36 frames after the command (seen one handler on).
    assert_eq!(rel, Some((37, 36.0)));
    assert_eq!(p.recom.releases.len(), 1);
    assert!(matches!(free_at, Some(68) | Some(69)), "{free_at:?}");
    assert_eq!(p.recom.cast_ends, 1);
}

#[test]
fn a_cure_cut_before_36_heals_nothing_and_an_air_cure_hops() {
    use crate::magic::CastEvent;
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_cast(&mut p, Pad::default(), None);
    step_cast(&mut p, Pad::default(), Some(CURE1));
    for _ in 0..30 {
        step_cast(&mut p, Pad::default(), None);
    }
    // A hit at frame 31 (Re:CoM ran its reaction): no heal, no MP.
    let out = p.ours.frame(&mut p.recom, &FrameInput::default(), true);
    assert!(out.cast.iter().any(|e| matches!(e, CastEvent::Interrupted { .. })), "{:?}", out.cast);
    for _ in 0..20 {
        assert!(!released(&step_cast(&mut p, Pad::default(), None)));
    }
    assert!(p.recom.releases.is_empty());
    // In the air: KH1's 30 cm hop and the air clip (0x86).
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_cast(&mut p, Pad::default(), None);
    step_cast(&mut p, Pad { jump: true, ..Default::default() }, None);
    for _ in 0..6 {
        step_cast(&mut p, Pad { jump: true, ..Default::default() }, None);
    }
    let out = step_cast(&mut p, Pad::default(), Some(CURE1));
    assert!(out.cast.iter().any(|e| matches!(e, CastEvent::Started { air: true, .. })), "{:?}", out.cast);
    assert_eq!(p.ours.rec.head(), crate::magic::cast_req::CURE_AIR);
    assert!((p.ours.arc.rise + crate::magic::AIR_HOP).abs() < 1e-3);
}

// ---- M7 stage 7.8: Gravity ----

#[test]
fn ground_gravity_releases_at_28_and_frees_sora_at_68() {
    use crate::magic::{cast_req, CastEvent, MagicCommand};
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    for _ in 0..5 {
        step_cast(&mut p, Pad::default(), None);
    }
    let out = step_cast(&mut p, Pad::default(), Some(MagicCommand { spell: 4, level: 2 }));
    assert_eq!(out.cast.first(), Some(&CastEvent::Started { spell: 4, level: 2, fast: false, air: false }));
    assert_eq!(p.ours.rec.head(), cast_req::GRAVITY);
    let mut rel = None;
    let mut free_at = None;
    for k in 1..100 {
        let out = step_cast(&mut p, Pad::default(), None);
        if let Some(CastEvent::Released { frame, .. }) = out.cast.iter().find(|e| matches!(e, CastEvent::Released { .. })) {
            rel = Some((k, *frame));
        }
        if free_at.is_none() && !p.ours.lock {
            free_at = Some(k);
        }
    }
    // `L009e2(28)`: the sphere 28 frames after the command; Sora free from the motion's key (68).
    assert_eq!(rel, Some((29, 28.0)));
    assert_eq!(p.recom.releases.len(), 1);
    assert!(matches!(free_at, Some(68) | Some(69)), "{free_at:?}");
    assert_eq!(p.recom.cast_ends, 1);
    // A Gravity cut before 28 casts nothing (no sphere, no MP).
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_cast(&mut p, Pad::default(), None);
    step_cast(&mut p, Pad::default(), Some(MagicCommand { spell: 4, level: 1 }));
    for _ in 0..20 {
        step_cast(&mut p, Pad::default(), None);
    }
    let out = p.ours.frame(&mut p.recom, &FrameInput::default(), true);
    assert!(out.cast.iter().any(|e| matches!(e, CastEvent::Interrupted { .. })));
    for _ in 0..20 {
        assert!(!released(&step_cast(&mut p, Pad::default(), None)));
    }
}

// ---- M7 stage 7.9: Stop ----

#[test]
fn ground_stop_releases_at_28_and_locks_sora_to_68() {
    use crate::magic::{cast_req, CastEvent, MagicCommand};
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    for _ in 0..5 {
        step_cast(&mut p, Pad::default(), None);
    }
    let out = step_cast(&mut p, Pad::default(), Some(MagicCommand { spell: 5, level: 3 }));
    assert_eq!(out.cast.first(), Some(&CastEvent::Started { spell: 5, level: 3, fast: false, air: false }));
    assert_eq!(p.ours.rec.head(), cast_req::STOP);
    let mut rel = None;
    let mut free_at = None;
    for k in 1..100 {
        let out = step_cast(&mut p, Pad::default(), None);
        if let Some(CastEvent::Released { frame, .. }) = out.cast.iter().find(|e| matches!(e, CastEvent::Released { .. })) {
            rel = Some((k, *frame));
        }
        if free_at.is_none() && !p.ours.lock {
            free_at = Some(k);
        }
    }
    // Wait 4, then the check each frame for 24: the thread at 28.
    assert_eq!(rel, Some((29, 28.0)));
    assert!(matches!(free_at, Some(68) | Some(69)), "{free_at:?}");
    assert_eq!(p.recom.cast_ends, 1);
    // Cut at frame 10 (inside the 24 checked frames): nothing, no MP.
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_cast(&mut p, Pad::default(), None);
    step_cast(&mut p, Pad::default(), Some(MagicCommand { spell: 5, level: 1 }));
    for _ in 0..10 {
        step_cast(&mut p, Pad::default(), None);
    }
    let out = p.ours.frame(&mut p.recom, &FrameInput::default(), true);
    assert!(out.cast.iter().any(|e| matches!(e, CastEvent::Interrupted { .. })));
    for _ in 0..30 {
        assert!(!released(&step_cast(&mut p, Pad::default(), None)));
    }
}

// ---- M7 stage 7.10: Aero ----

#[test]
fn aero_casts_at_30_or_at_once_when_its_motion_is_cut() {
    use crate::magic::{cast_req, CastEvent, MagicCommand};
    const AERO2: MagicCommand = MagicCommand { spell: 6, level: 2 };
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    for _ in 0..5 {
        step_cast(&mut p, Pad::default(), None);
    }
    step_cast(&mut p, Pad::default(), Some(AERO2));
    assert_eq!(p.ours.rec.head(), cast_req::AERO);
    let mut rel = None;
    let mut free_at = None;
    for k in 1..100 {
        let out = step_cast(&mut p, Pad::default(), None);
        if let Some(CastEvent::Released { frame, .. }) = out.cast.iter().find(|e| matches!(e, CastEvent::Released { .. })) {
            rel = Some((k, *frame));
        }
        if free_at.is_none() && !p.ours.lock {
            free_at = Some(k);
        }
    }
    // Up to 30 frames while its motion plays, then the barrier.
    assert_eq!(rel, Some((31, 30.0)));
    assert_eq!(p.recom.releases.len(), 1);
    assert!(matches!(free_at, Some(68) | Some(69)), "{free_at:?}");
    // Cut at frame 10 (Re:CoM's hit reaction): KH1's Aero still casts, at once.
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_cast(&mut p, Pad::default(), None);
    step_cast(&mut p, Pad::default(), Some(AERO2));
    for _ in 0..10 {
        step_cast(&mut p, Pad::default(), None);
    }
    let out = p.ours.frame(&mut p.recom, &FrameInput::default(), true);
    assert!(out.cast.iter().any(|e| matches!(e, CastEvent::Released { spell: 6, .. })), "{:?}", out.cast);
    assert!(!out.cast.iter().any(|e| matches!(e, CastEvent::Interrupted { .. })));
    assert_eq!(p.recom.releases.len(), 1);
    // An air Aero that lands at once: the landing replaces its motion, and it casts then.
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_cast(&mut p, Pad::default(), None);
    step_cast(&mut p, Pad { jump: true, ..Default::default() }, None);
    let mut cast = false;
    let mut at = None;
    for _ in 0..80 {
        let falling = p.ours.state == state::AIR && !p.ours.arc.rising() && p.recom.pos.y > -40.0;
        let m = (falling && !cast).then_some(AERO2);
        cast |= m.is_some();
        let out = step_cast(&mut p, Pad::default(), m);
        if let Some(CastEvent::Released { frame, .. }) = out.cast.iter().find(|e| matches!(e, CastEvent::Released { .. })) {
            at = Some(*frame);
        }
    }
    assert!(cast);
    assert_eq!(p.recom.releases.len(), 1);
    assert!(at.is_some_and(|f| f < 30.0), "the landing before 30 casts it then: {at:?}");
}

// ---- M8 stage 8a.1: items ----

fn step_item(p: &mut Pair, pad: Pad, item: Option<crate::items::ItemCommand>) -> FrameOut {
    let (dir, tilt) = Input::move_from_stick(pad.stick, 0.0);
    let jump = Button::from_held(p.prev.jump, pad.jump);
    let square = Button::from_held(p.prev.square, pad.square);
    let attack = Button::from_held(p.prev.attack, pad.attack);
    p.prev = pad;
    let out = p.ours.frame(&mut p.recom, &FrameInput { move_dir: dir, tilt, recom_tilt: tilt, jump, square, lock_target: None, attack, target: None, locked: false, magic: None, item, special: None }, p.first);
    p.first = false;
    p.recom.after_exe();
    out
}

const POTION0: crate::items::ItemCommand = crate::items::ItemCommand { item: 1, slot: 0 };

#[test]
fn an_item_plays_kh1s_use_motion_on_recoms_item_action() {
    use crate::items::{ItemEvent, RECOM_ITEM_ACTION, USE_SELF};
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    for _ in 0..5 {
        step_item(&mut p, Pad::default(), None);
    }
    // Sora runs; the item stops him and keeps his facing (the target is Sora).
    for _ in 0..10 {
        step_item(&mut p, Pad { stick: [0.0, 1.0], ..Default::default() }, None);
    }
    let facing = p.ours.facing;
    let out = step_item(&mut p, Pad { stick: [1.0, 0.0], ..Default::default() }, Some(POTION0));
    assert_eq!(out.items, vec![ItemEvent::Started { item: 1, slot: 0 }]);
    assert_eq!(p.recom.items, vec![POTION0]);
    assert_eq!(p.ours.rec.head(), USE_SELF);
    assert_eq!(p.recom.act.id, RECOM_ITEM_ACTION);
    assert!((p.ours.facing - facing).abs() < 1e-4, "no turn toward the stick");
    // Locked to frame 74 (the motion's free key), the action ended once when it ends at 82.
    let mut free_at = None;
    for k in 1..120 {
        step_item(&mut p, Pad::default(), None);
        if free_at.is_none() && !p.ours.lock {
            free_at = Some(k);
        }
        if k < 74 {
            assert_eq!(p.recom.act.id, RECOM_ITEM_ACTION, "frame {k}");
            assert!(p.recom.vel.flat().length() < 1e-6);
        }
    }
    assert!(matches!(free_at, Some(74) | Some(75)), "{free_at:?}");
    assert_eq!(p.recom.item_ends, 1);
    assert!(p.ours.item.is_none());
}

#[test]
fn items_are_ground_only_and_wait_for_kh1s_gate() {
    use crate::items::ItemEvent;
    // In the air: KH1's command dispatch drops it (state 2).
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_item(&mut p, Pad::default(), None);
    for _ in 0..8 {
        step_item(&mut p, Pad { jump: true, ..Default::default() }, None);
    }
    let out = step_item(&mut p, Pad { jump: true, ..Default::default() }, Some(POTION0));
    assert_eq!(out.items, vec![ItemEvent::Refused { item: 1, why: "state" }]);
    assert!(p.recom.items.is_empty());
    // During a locked use: refused by the gate ("not now"), the slot untouched.
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_item(&mut p, Pad::default(), None);
    step_item(&mut p, Pad::default(), Some(POTION0));
    let out = step_item(&mut p, Pad::default(), Some(crate::items::ItemCommand { item: 3, slot: 1 }));
    assert_eq!(out.items, vec![ItemEvent::Refused { item: 3, why: "not now" }]);
    assert_eq!(p.recom.items.len(), 1);
    // A slot that no longer holds it: refused, no motion.
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    p.recom.item_missing = true;
    step_item(&mut p, Pad::default(), None);
    let out = step_item(&mut p, Pad::default(), Some(POTION0));
    assert_eq!(out.items, vec![ItemEvent::Refused { item: 1, why: "not in its slot" }]);
    assert_ne!(p.ours.rec.head(), crate::items::USE_SELF);
}

#[test]
fn a_hit_during_the_use_ends_the_motion_only() {
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_item(&mut p, Pad::default(), None);
    step_item(&mut p, Pad::default(), Some(POTION0));
    for _ in 0..20 {
        step_item(&mut p, Pad::default(), None);
    }
    // Re:CoM ran its hit reaction: the next KH1 frame takes over afresh; the motion part is gone
    // (Re:CoM's own action replaced it), the use task is `win::items`'s and goes on.
    p.ours.frame(&mut p.recom, &FrameInput::default(), true);
    assert!(p.ours.item.is_none());
    assert_eq!(p.recom.item_ends, 0, "Re:CoM's hit action ended it, not us");
}

// ---- M9 stage 9c.1: Sonic Blade on Re:CoM's sleight action ----

fn step_special(p: &mut Pair, c: Option<u8>) -> FrameOut {
    p.special = c;
    p.step(Pad::default()).out
}

/// KH1's flow (`0x14039EAD0`, `0x14039ECA0`, `0x14039ED70`): the press starts the wind-up on
/// Re:CoM's sleight action, invincible; 30 frames later dash 1 (row 0x1B) without a blend; Rave
/// (offered from the dash's frame 10, the combo window) chains dashes 2..6 with blend 8, Blast the
/// finisher (row 0x21); its end returns Sora to state 0 and Re:CoM's action to the idle.
#[test]
fn sonic_blade_runs_kh1s_rows_on_recoms_sleight_action() {
    use crate::specials::{cmd, SpecialEvent, RECOM_ACTION, STATE};
    use kh1_core::data::xl;
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    for _ in 0..3 {
        step_special(&mut p, None);
    }
    let out = step_special(&mut p, Some(cmd::SONIC_BLADE));
    assert_eq!(out.specials, vec![SpecialEvent::Started { index: 0 }]);
    assert_eq!(p.recom.specials.len(), 1);
    assert_eq!((p.recom.act.id, p.ours.state), (RECOM_ACTION, STATE));
    assert_eq!(p.ours.rec.head(), xl::request(xl::WINDUP));
    assert!(p.recom.untouchable, "no damage, no reaction for the whole special");
    assert_eq!(p.ours.follow_up(), Some((cmd::RAVE, true)), "Rave in the Attack row, greyed in the wind-up");
    // The wind-up's 30 frames, then dash 1 (row 0x1B, counter 1) with blend 0.
    let mut k = 0;
    while p.recom.special_rows.is_empty() {
        step_special(&mut p, None);
        k += 1;
        assert!(k < 40);
    }
    // The wind-up's 30 frames end in the update of the press + 30 (its clock does not move on its
    // first update); KH1's end callback starts dash 1's row then and its motion on the next update.
    assert_eq!(k, 31, "dash 1 starts as the wind-up ends");
    assert_eq!((p.recom.special_rows[0].kh1_row, p.recom.special_rows[0].counter, p.recom.special_rows[0].recom_id), (0x1B, 1, crate::specials::recom::ID_DASH));
    let pos0 = p.recom.pos;
    // A Rave before the window (dash frame 10) is refused; the window opens at 10.
    let out = step_special(&mut p, Some(cmd::RAVE));
    assert!(matches!(out.specials[..], [SpecialEvent::Refused { command: cmd::RAVE, .. }]), "{:?}", out.specials);
    let mut opened = None;
    for f in 0..40 {
        step_special(&mut p, None);
        if p.ours.follow_up() == Some((cmd::RAVE, false)) {
            opened = Some(f);
            break;
        }
    }
    assert!(matches!(opened, Some(7..=10)), "{opened:?}");
    // Rave: dash 2 (row 0x1C) at once, blend 8; up to dash 6, then Blast: the finisher.
    for n in 2..=7u8 {
        while p.ours.follow_up().is_none_or(|f| f.1) {
            step_special(&mut p, None);
        }
        let c = if n == 7 { cmd::BLAST } else { cmd::RAVE };
        assert_eq!(p.ours.follow_up(), Some((c, false)), "dash {n}");
        let out = step_special(&mut p, Some(c));
        assert!(matches!(out.specials[..], [SpecialEvent::Row { counter, .. }] if counter == n), "{:?}", out.specials);
        assert_eq!(p.recom.special_rows.last().map(|r| r.kh1_row), Some(0x1A + n as usize));
    }
    assert_eq!(p.recom.special_rows.last().map(|r| r.recom_id), Some(crate::specials::recom::ID_FINISHER));
    assert!((p.recom.pos - pos0).flat().length() > 500.0, "root travel moves Sora");
    // The finisher (82 frames) ends the special: state 0, Re:CoM's action ended once, no flags.
    let mut k = 0;
    while p.ours.special.is_some() {
        step_special(&mut p, None);
        k += 1;
        assert!(k < 100);
    }
    assert_eq!((p.recom.special_ends, p.ours.state), (1, state::GROUND));
    assert_ne!(p.recom.act.id, RECOM_ACTION);
    step_special(&mut p, None);
    assert!(!p.recom.untouchable);
}

/// Without a Rave dash 1 ends the special after its 54 frames, its root travel KH1's 562.05; in the
/// air (a walk-off) Sora falls out (`0x14039EBA0`).
#[test]
fn sonic_blade_ends_after_one_dash_or_in_the_air() {
    use crate::specials::cmd;
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_special(&mut p, None);
    step_special(&mut p, Some(cmd::SONIC_BLADE));
    let start = p.recom.pos;
    let mut k = 0;
    while p.ours.special.is_some() {
        step_special(&mut p, None);
        k += 1;
        assert!(k < 120);
    }
    // Wind-up 30 + dash 54 (± the update the end is seen in).
    assert!((83..=86).contains(&k), "{k}");
    assert_eq!(p.recom.special_rows.len(), 1);
    for _ in 0..3 {
        step_special(&mut p, None);
    }
    let travel = (p.recom.pos - start).flat().length();
    assert!((travel - 562.05).abs() < 0.5, "{travel}");
    // Off a ledge mid-dash: the special ends, KH1's fall.
    let mut p = Pair::new(Abilities::default(), Floor { edge_z: 150.0, low: 400.0 });
    step_special(&mut p, None);
    step_special(&mut p, Some(cmd::SONIC_BLADE));
    for _ in 0..60 {
        step_special(&mut p, None);
    }
    assert!(p.ours.special.is_none());
    assert_eq!(p.ours.state, state::AIR);
}

/// Refusals: in the air (state 2), with no MP (Re:CoM's start refuses), and a second press.
#[test]
fn sonic_blade_refusals() {
    use crate::specials::{cmd, SpecialEvent};
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_special(&mut p, None);
    p.recom.mp_ok = false;
    let out = step_special(&mut p, Some(cmd::SONIC_BLADE));
    assert!(matches!(out.specials[..], [SpecialEvent::Refused { .. }]));
    assert!(p.ours.special.is_none());
    p.recom.mp_ok = true;
    step_special(&mut p, Some(cmd::SONIC_BLADE));
    let out = step_special(&mut p, Some(cmd::SONIC_BLADE));
    assert!(matches!(out.specials[..], [SpecialEvent::Refused { .. }]), "a press in the wind-up: refused");
    assert_eq!(p.recom.specials.len(), 1);
    // Magic and items during the special: KH1's dispatch drops them (state 0x20).
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_special(&mut p, None);
    for _ in 0..8 {
        p.step(Pad { jump: true, ..Default::default() });
    }
    let out = step_special(&mut p, Some(cmd::SONIC_BLADE));
    assert!(matches!(out.specials[..], [SpecialEvent::Refused { .. }]), "in the air");
}

// ---- M12 step 7: Strike Raid on Re:CoM's sleight action ----

/// Steps until `f` holds (at most `max` frames); the frames it took.
fn until(p: &mut Pair, max: usize, f: impl Fn(&Pair) -> bool) -> usize {
    let mut k = 0;
    while !f(p) {
        step_special(p, None);
        k += 1;
        assert!(k < max, "not within {max} frames");
    }
    k
}

/// KH1's flow (`0x14039FFD0`, `0x1403A0290`, `0x1403A0230`): the press starts the wind-up (0xE6, 30
/// frames) on Re:CoM's sleight action, invincible, in state 0x27; then throw 1 (row 0x2F, Re:CoM's
/// Strike Raid id 0xD), its catch and the wait, whose combo window offers the next throw ("Raid");
/// from n = 4 the follow-up is "Judgment" (0x60), the fifth throw the finishing one (row 0x33,
/// 0xE8); after it nothing more is offered and the wait ends the special. No travel. Each throw's
/// attack ends with its motion.
#[test]
fn strike_raid_runs_kh1s_throws_on_recoms_sleight_action() {
    use crate::specials::{cmd, recom, SpecialEvent, RECOM_ACTION, STATE_OF, STRIKE_RAID};
    use kh1_core::data::xl;
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    for _ in 0..3 {
        step_special(&mut p, None);
    }
    let pos0 = p.recom.pos;
    let out = step_special(&mut p, Some(cmd::STRIKE_RAID));
    assert_eq!(out.specials, vec![SpecialEvent::Started { index: STRIKE_RAID }]);
    assert_eq!(p.recom.specials.last().map(|s| (s.index, s.cost)), Some((STRIKE_RAID, 200)), "KH1's 2 MP on the press");
    assert_eq!((p.recom.act.id, p.ours.state), (RECOM_ACTION, STATE_OF[STRIKE_RAID]));
    assert_eq!((p.ours.state, p.ours.rec.head()), (0x27, xl::request(xl::SR_WINDUP)));
    assert!(p.recom.untouchable);
    assert_eq!(p.ours.follow_up(), Some((cmd::RAID, true)), "Raid in the Attack row, greyed in the wind-up");
    // The wind-up's 30 frames, then throw 1 (row 0x2F) at blend 0.
    let k = until(&mut p, 40, |p| !p.recom.special_rows.is_empty());
    assert_eq!(k, 31, "throw 1 as the wind-up ends");
    assert_eq!((p.recom.special_rows[0].kh1_row, p.recom.special_rows[0].counter, p.recom.special_rows[0].recom_id), (0x2F, 1, recom::ID_RAID));
    assert_eq!(p.ours.rec.head(), xl::request(xl::SR_THROW));
    // No window in the throw: the next one waits for the wait's combo window (after the 42-frame
    // throw and the 8-frame catch); the throw's attack ends with it.
    let out = step_special(&mut p, Some(cmd::RAID));
    assert!(matches!(out.specials[..], [SpecialEvent::Refused { command: cmd::RAID, .. }]), "{:?}", out.specials);
    let k = until(&mut p, 80, |p| p.ours.follow_up() == Some((cmd::RAID, false)));
    assert!((48..=53).contains(&k), "the wait's window opens after the throw and the catch: {k}");
    assert_eq!(p.ours.rec.req, xl::request(xl::SR_WAIT));
    assert_eq!(p.recom.special_row_ends, 1);
    for n in 2..=5u8 {
        let c = if n > 4 { cmd::RAID_LAST } else { cmd::RAID };
        until(&mut p, 120, |p| p.ours.follow_up() == Some((c, false)));
        let out = step_special(&mut p, Some(c));
        assert!(matches!(out.specials[..], [SpecialEvent::Row { counter, kh1_row, .. }] if counter == n && kh1_row == 0x2E + n as usize), "{:?}", out.specials);
        assert_eq!(p.recom.special_rows.last().map(|r| (r.kh1_row, r.recom_id)), Some((0x2E + n as usize, recom::ID_RAID)));
    }
    assert_eq!(p.ours.rec.head(), xl::request(xl::SR_FINISHER), "the fifth: the finishing throw 0xE8");
    // After the fifth nothing more is offered; the wait ends the special.
    let mut k = 0;
    while p.ours.special.is_some() {
        assert!(p.ours.follow_up().is_some_and(|f| f.1), "nothing after the fifth");
        step_special(&mut p, None);
        k += 1;
        assert!(k < 200);
    }
    assert!((145..=152).contains(&k), "76 + 8 + 64 frames: {k}");
    assert_eq!((p.recom.special_ends, p.ours.state, p.recom.special_row_ends), (1, state::GROUND, 5));
    assert_ne!(p.recom.act.id, RECOM_ACTION);
    assert!((p.recom.pos - pos0).flat().length() < 0.01, "Strike Raid doesn't move Sora");
    step_special(&mut p, None);
    assert!(!p.recom.untouchable);
}

/// One throw without a follow-up: wind-up 30 + throw 42 + catch 8 + wait 64; a target more than 200
/// above makes it the high throw and catch (`0x1403A0290`); toward the target; the fifth is never
/// offered after it ends.
#[test]
fn strike_raid_one_throw_high_and_aimed() {
    use crate::specials::cmd;
    use kh1_core::data::xl;
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_special(&mut p, None);
    step_special(&mut p, Some(cmd::STRIKE_RAID));
    let k = until(&mut p, 200, |p| p.ours.special.is_none());
    assert!((143..=147).contains(&k), "{k}");
    assert_eq!(p.recom.special_rows.len(), 1);
    // A target 250 above (KH1 y is down) and to the side: the high throw 0xEA, then the catch 0xEC.
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_special(&mut p, None);
    p.target = Some(kh1_core::TargetPart { owner: 1, centre: Vec3::new(400.0, -250.0, 0.0), radius: 30.0 });
    step_special(&mut p, Some(cmd::STRIKE_RAID));
    until(&mut p, 40, |p| !p.recom.special_rows.is_empty());
    assert_eq!(p.ours.rec.head(), xl::request(xl::SR_THROW_HIGH));
    assert!((wrap_angle(p.ours.target_yaw - Vec3::new(1.0, 0.0, 0.0).yaw())).abs() < 1e-3, "toward the target");
    until(&mut p, 60, |p| p.ours.rec.req == xl::request(xl::SR_CATCH_HIGH));
    // 200 above or less: the plain throw.
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_special(&mut p, None);
    p.target = Some(kh1_core::TargetPart { owner: 1, centre: Vec3::new(0.0, -200.0, 400.0), radius: 30.0 });
    step_special(&mut p, Some(cmd::STRIKE_RAID));
    until(&mut p, 40, |p| !p.recom.special_rows.is_empty());
    assert_eq!(p.ours.rec.head(), xl::request(xl::SR_THROW));
}

/// A clash of the thrown Keyblade (KH1's reaction in state 0x27 → `0x1403A00A0`): the knocked-back
/// catch 0xED, the wait, the end; no recoil; Sora stays in the special and may still throw again.
#[test]
fn strike_raid_clash_knocks_the_keyblade_back() {
    use crate::specials::{cmd, SpecialEvent};
    use kh1_core::data::xl;
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_special(&mut p, None);
    step_special(&mut p, Some(cmd::STRIKE_RAID));
    until(&mut p, 40, |p| !p.recom.special_rows.is_empty());
    for _ in 0..12 {
        step_special(&mut p, None);
    }
    p.recom.swing_clash = Some((6, -30));
    let out = step_special(&mut p, None);
    assert!(out.specials.iter().any(|e| matches!(e, SpecialEvent::Clashed { .. })), "{:?}", out.specials);
    assert_eq!(p.ours.rec.head(), xl::request(xl::SR_CLASH));
    assert!(p.ours.special.is_some() && p.ours.state == 0x27, "no recoil, still the special");
    assert_eq!(p.recom.special_row_ends, 1);
    let k = until(&mut p, 120, |p| p.ours.follow_up() == Some((cmd::RAID, false)));
    assert!((22..=27).contains(&k), "the 24-frame knocked-back catch, then the wait's window: {k}");
}

// ---- M12 step 7: Ars Arcanum on Re:CoM's sleight action ----

/// KH1's flow (`0x14039EEB0`, `0x14039F090` / `0x14039F100`, `0x14039F1E0` / `0x14039EDD0`): the
/// press starts the wind-up (0xD2, 16 frames) invincible in state 0x20; the first seven hits (rows
/// 0x22..0x28, Re:CoM's ids 0x12..0x18) follow by themselves, travelling by their root motion; in
/// the seventh's combo window "Bash" (0x58) runs the eighth, from which Sora can be hit; the
/// twelfth is followed by "Finis" (0x59), the finisher (row 0x2E, id 0x1E); its end ends the special.
#[test]
fn ars_arcanum_runs_kh1s_hits_on_recoms_sleight_action() {
    use crate::specials::{cmd, recom, SpecialEvent, ARS_ARCANUM, RECOM_ACTION, STATE};
    use kh1_core::data::xl;
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    for _ in 0..3 {
        step_special(&mut p, None);
    }
    let pos0 = p.recom.pos;
    let out = step_special(&mut p, Some(cmd::ARS_ARCANUM));
    assert_eq!(out.specials, vec![SpecialEvent::Started { index: ARS_ARCANUM }]);
    assert_eq!(p.recom.specials.last().map(|s| (s.index, s.cost)), Some((ARS_ARCANUM, 300)), "KH1's 3 MP on the press");
    assert_eq!((p.recom.act.id, p.ours.state, p.ours.rec.head()), (RECOM_ACTION, STATE, xl::request(xl::AA_WINDUP)));
    assert!(p.recom.untouchable);
    assert_eq!(p.ours.follow_up(), Some((cmd::ARCANUM, true)));
    let k = until(&mut p, 30, |p| !p.recom.special_rows.is_empty());
    assert_eq!(k, 17, "hit 1 as the 16-frame wind-up ends");
    assert_eq!((p.recom.special_rows[0].kh1_row, p.recom.special_rows[0].recom_id), (0x22, recom::ID_ARCANUM_HIT));
    // Seven by themselves, no input.
    until(&mut p, 200, |p| p.recom.special_rows.len() == 7);
    let ids: Vec<(usize, u32)> = p.recom.special_rows.iter().map(|r| (r.kh1_row, r.recom_id)).collect();
    assert_eq!(ids, (0..7).map(|k| (0x22 + k, 0x12 + k as u32)).collect::<Vec<_>>());
    assert_eq!(p.ours.rec.head(), xl::request(10), "the seventh hit (0xD9)");
    // Its window (22..52) offers "Bash"; until then greyed.
    let k = until(&mut p, 60, |p| p.ours.follow_up() == Some((cmd::ARCANUM, false)));
    assert!((19..=23).contains(&k), "{k}");
    let out = step_special(&mut p, Some(cmd::ARCANUM));
    assert!(matches!(out.specials[..], [SpecialEvent::Row { counter: 8, kh1_row: 0x29, .. }]), "{:?}", out.specials);
    assert_eq!(p.recom.special_rows.last().map(|r| r.recom_id), Some(0x19));
    step_special(&mut p, None);
    assert!(!p.recom.untouchable, "from the first follow-up on Sora can be hit (KH1 clears 0x1000080)");
    for n in 9..=13u8 {
        let c = if n == 13 { cmd::ARCANUM_LAST } else { cmd::ARCANUM };
        until(&mut p, 80, |p| p.ours.follow_up() == Some((c, false)));
        let out = step_special(&mut p, Some(c));
        assert!(matches!(out.specials[..], [SpecialEvent::Row { counter, kh1_row, .. }] if counter == n && kh1_row == 0x21 + n as usize), "{:?}", out.specials);
    }
    assert_eq!(p.recom.special_rows.last().map(|r| (r.kh1_row, r.recom_id)), Some((0x2E, 0x1E)));
    assert_eq!(p.ours.rec.head(), xl::request(xl::AA_FINISHER));
    let k = until(&mut p, 120, |p| p.ours.special.is_none());
    assert!((86..=91).contains(&k), "the 88-frame finisher: {k}");
    assert_eq!((p.recom.special_ends, p.ours.state), (1, state::GROUND));
    assert!((p.recom.pos - pos0).flat().length() > 800.0, "root travel moves Sora: {}", (p.recom.pos - pos0).flat().length());
}

/// Without a follow-up the special ends after the seventh hit (16 + the seven hits' 140 frames), its
/// root travel KH1's (620); Sora invincible throughout.
#[test]
fn ars_arcanum_ends_after_seven_hits() {
    use crate::specials::cmd;
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_special(&mut p, None);
    step_special(&mut p, Some(cmd::ARS_ARCANUM));
    let start = p.recom.pos;
    let mut k = 0;
    while p.ours.special.is_some() {
        assert!(p.recom.untouchable || k == 0);
        step_special(&mut p, None);
        k += 1;
        assert!(k < 250);
    }
    assert!((156..=164).contains(&k), "{k}");
    assert_eq!(p.recom.special_rows.len(), 7);
    for _ in 0..3 {
        step_special(&mut p, None);
    }
    let travel = (p.recom.pos - start).flat().length();
    assert!((travel - 620.0).abs() < 1.0, "{travel}");
}

/// A clash in state 0x20 (Ars Arcanum, Sonic Blade; KH1's reaction `0x14029BBD0` case 0x20): the
/// special ends (Re:CoM's sleight action ended) and Sora takes the swing's recoil.
#[test]
fn a_clash_ends_ars_arcanum_with_the_recoil() {
    use crate::specials::cmd;
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    step_special(&mut p, None);
    step_special(&mut p, Some(cmd::ARS_ARCANUM));
    until(&mut p, 30, |p| !p.recom.special_rows.is_empty());
    step_special(&mut p, None);
    p.recom.swing_clash = Some((6, -10));
    step_special(&mut p, None);
    assert!(p.ours.special.is_none() && p.ours.state == state::GROUND);
    assert_eq!(p.recom.special_ends, 1);
    assert_eq!(p.ours.rec.head(), crate::rows::RECOIL);
}

/// M12 step 7, Ragnarok (`0x14039F250`, `0x14039F7A0`, `0x14039F330`): the wind-up, five rising
/// hits by themselves (rows 0x36..=0x3A, Sora rising), the charge (hovering, no longer invincible,
/// "Impact" lit from charge time 10), Triangle at t ≥ 40: 17 shots, then the fall and the landing.
#[test]
fn ragnarok_runs_kh1s_hits_charge_and_shots() {
    use crate::specials::{cmd, Phase, STATE_OF, RAGNAROK};
    use kh1_core::data::{row, xl};
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    p.target = Some(kh1_core::TargetPart { owner: 1, centre: Vec3::new(0.0, -300.0, 300.0), radius: 30.0 });
    step_special(&mut p, None);
    let y0 = p.recom.pos.y;
    step_special(&mut p, Some(cmd::RAGNAROK));
    assert_eq!(p.ours.state, STATE_OF[RAGNAROK]);
    assert_eq!(p.ours.rec.head(), xl::request(xl::RAG_WINDUP));
    assert!(p.recom.untouchable);
    assert_eq!(p.ours.follow_up(), Some((cmd::IMPACT, true)), "Impact shown, greyed");
    until(&mut p, 120, |p| p.ours.special.is_some_and(|s| s.phase == Phase::Charge));
    let rows: Vec<usize> = p.recom.special_rows.iter().map(|r| r.kh1_row).collect();
    assert_eq!(rows, (row::RAGNAROK_HIT..=row::RAGNAROK_LAST).collect::<Vec<_>>());
    assert!(p.recom.special_rows.iter().all(|r| r.recom_id == crate::specials::recom::ID_RAGNAROK));
    assert!(p.recom.pos.y < y0 - 100.0, "risen: {}", y0 - p.recom.pos.y);
    assert!(!p.recom.untouchable, "the charge can be hit");
    assert_eq!(p.ours.rec.head(), xl::request(xl::RAG_CHARGE));
    let hover = p.recom.pos.y;
    until(&mut p, 20, |p| p.ours.follow_up() == Some((cmd::IMPACT, false)));
    let t = p.ours.special.unwrap().charge;
    assert!((10.0..11.5).contains(&t), "lit at {t}");
    assert!((p.recom.pos.y - hover).abs() < 1e-3, "hovering");
    until(&mut p, 40, |p| p.ours.special.is_some_and(|s| s.charge >= 42.0));
    step_special(&mut p, Some(cmd::IMPACT));
    assert_eq!(p.recom.ragnarok_fires.len(), 1);
    assert_eq!(p.recom.ragnarok_fires[0].shots, 17);
    assert_eq!(p.ours.rec.head(), xl::request(xl::RAG_FIRE));
    until(&mut p, 200, |p| p.ours.special.is_none());
    until(&mut p, 200, |p| p.ours.state == state::GROUND);
    assert_eq!(p.recom.ragnarok_fires.len(), 1);
}

/// Without Triangle the charge (76 frames) ends into the fall: the special ends, Sora falls.
#[test]
fn ragnarok_without_impact_falls_after_the_charge() {
    use crate::specials::{cmd, Phase};
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    p.target = Some(kh1_core::TargetPart { owner: 1, centre: Vec3::new(0.0, -300.0, 300.0), radius: 30.0 });
    step_special(&mut p, None);
    step_special(&mut p, Some(cmd::RAGNAROK));
    until(&mut p, 120, |p| p.ours.special.is_some_and(|s| s.phase == Phase::Charge));
    let k = until(&mut p, 120, |p| p.ours.special.is_none());
    assert!((70..=80).contains(&k), "{k}");
    assert!(p.recom.ragnarok_fires.is_empty());
    until(&mut p, 200, |p| p.ours.state == state::GROUND);
}

// ---- `dodge_roll = recom`: Re:CoM's own roll (owner, 2026-11-02, "The whole roll") ----

/// One handler frame with the stick (no oracle: the roll after the hand-over is Re:CoM's), an
/// enemy ahead when `target`. As the win layer does, a frame that handed Sora to Re:CoM leaves no
/// sync, so the next one takes over afresh.
fn step_free(p: &mut Pair, pad: Pad, target: bool) -> FrameOut {
    let (dir, tilt) = Input::move_from_stick(pad.stick, 0.0);
    let jump = Button::from_held(p.prev.jump, pad.jump);
    let square = Button::from_held(p.prev.square, pad.square);
    let attack = Button::from_held(p.prev.attack, pad.attack);
    p.prev = pad;
    let target = target.then(|| kh1_core::TargetPart { owner: 1, centre: Vec3::new(0.0, 0.0, 150.0), radius: 30.0 });
    let out = p.ours.frame(&mut p.recom, &FrameInput { move_dir: dir, tilt, recom_tilt: tilt, jump, square, lock_target: None, attack, target, locked: false, magic: None, item: None, special: None }, p.first);
    p.first = out.handed_to_recom();
    p.recom.after_exe();
    out
}

fn recom_roll_pair(ab: Abilities) -> Pair {
    let mut p = Pair::new(ab, Floor::flat());
    p.ours.recom_roll = true;
    p
}

fn run_square() -> Pad {
    Pad { square: true, stick: [0.0, 1.0], ..Default::default() }
}

#[test]
fn square_while_running_starts_recoms_own_roll_and_takes_sora_back_after() {
    let mut p = recom_roll_pair(all());
    for _ in 0..10 {
        step_free(&mut p, stick(0.0, 1.0), false);
    }
    assert_eq!(p.recom.act.id, action::RUN);
    let plays = p.recom.plays.len();
    let out = step_free(&mut p, run_square(), false);
    assert!(out.recom_roll && out.handed_to_recom());
    assert!(out.events.contains(&Event::Rolled));
    assert_eq!(p.recom.recom_rolls, 1, "Re:CoM's avoid init, once");
    assert_eq!(p.recom.act, Action { id: action::AVOID, sub: action::SUB_ROLL, phase: phase::FALL }, "Re:CoM's action as its init left it");
    assert_eq!(p.recom.plays.len(), plays, "no KH1 motion: Re:CoM's init plays its own row");
    assert!(!p.recom.no_flinch, "no KH1 no-flinch frames (they are KH1's roll's)");
    assert_eq!(p.recom.vel, Vec3::ZERO, "no KH1 velocity over Re:CoM's root motion");
    assert!(!p.ours.lock && p.ours.rec.req == 0xFFFF);
    // Re:CoM's exe ends the roll (its moving cancel into its run) and clears its own bit.
    p.recom.untouchable = false;
    p.recom.act = Action { id: action::RUN, sub: 0, phase: 0 };
    let writes = p.recom.untouchable_writes;
    let out = step_free(&mut p, stick(0.0, 1.0), false);
    assert!(!out.handed_to_recom());
    assert_eq!((p.ours.state, p.recom.act.id), (state::GROUND, action::RUN), "KH1's run again, from Re:CoM's state");
    for _ in 0..30 {
        step_free(&mut p, stick(0.0, 1.0), false);
    }
    assert_eq!(p.recom.untouchable_writes, writes, "nothing of ours writes Re:CoM's untouchable bit");
    // The next roll is Re:CoM's again.
    step_free(&mut p, run_square(), false);
    assert_eq!(p.recom.recom_rolls, 2);
    // Re:CoM's exe ended it in the air: its fall (jump init(pl, 1)) hands back through enter_fall.
    p.recom.act = Action { id: action::JUMP, sub: 0, phase: phase::FALL };
    p.recom.contact = false;
    p.ours.enter_fall(&mut p.recom);
    assert_eq!((p.ours.state, p.recom.act.phase), (state::AIR, phase::FALL));
}

#[test]
fn recoms_roll_keeps_kh1s_square_rule() {
    // Guard equipped: below the run tilt Square guards (KH1's `0x1402A7B30`), as before.
    let mut p = recom_roll_pair(all());
    for _ in 0..5 {
        step_free(&mut p, stick(0.0, 0.5), false);
    }
    let out = step_free(&mut p, Pad { square: true, stick: [0.0, 0.5], ..Default::default() }, false);
    assert!(out.events.contains(&Event::Guarded) && !out.recom_roll);
    assert_eq!(p.recom.recom_rolls, 0);
    // Standing still, the same.
    let mut p = recom_roll_pair(all());
    step_free(&mut p, Pad::default(), false);
    let out = step_free(&mut p, Pad { square: true, ..Default::default() }, false);
    assert!(out.events.contains(&Event::Guarded) && p.recom.recom_rolls == 0);
    // Without the roll (`dodge_roll = off`), running Square guards.
    let mut p = recom_roll_pair(Abilities { dodge_roll: false, ..all() });
    for _ in 0..10 {
        step_free(&mut p, stick(0.0, 1.0), false);
    }
    let out = step_free(&mut p, run_square(), false);
    assert!(out.events.contains(&Event::Guarded) && p.recom.recom_rolls == 0);
    // In the air Square is no roll (Superglide's, or nothing).
    let mut p = recom_roll_pair(Abilities { superglide: false, ..all() });
    step_free(&mut p, Pad { jump: true, ..Default::default() }, false);
    for _ in 0..10 {
        step_free(&mut p, Pad { jump: true, stick: [0.0, 1.0], ..Default::default() }, false);
    }
    step_free(&mut p, Pad { jump: true, square: true, stick: [0.0, 1.0], ..Default::default() }, false);
    assert_eq!(p.recom.recom_rolls, 0);
}

#[test]
fn an_attack_on_the_same_frame_replaces_the_roll_as_it_replaces_kh1s() {
    for recom in [true, false] {
        let mut p = Pair::new(all(), Floor::flat());
        p.ours.recom_roll = recom;
        for _ in 0..10 {
            step_free(&mut p, stick(0.0, 1.0), true);
        }
        let out = step_free(&mut p, Pad { attack: true, ..run_square() }, true);
        assert!(!out.recom_roll, "recom {recom}");
        assert_eq!(p.recom.recom_rolls, 0, "recom {recom}");
        assert_eq!(p.recom.swings.len(), 1, "recom {recom}: the Attack");
    }
}

/// Steps without input until KH1's lock is off for the next frame (a swing's free frames, the
/// recoil's), with an enemy ahead.
fn until_free(p: &mut Pair) {
    for _ in 0..120 {
        step_free(p, Pad::default(), true);
        if !p.ours.after_frame(&mut p.recom, false).1 {
            return;
        }
    }
    panic!("never free");
}

#[test]
fn rolling_out_of_a_swing_ends_it_through_recoms_end_action_first() {
    let mut p = recom_roll_pair(all());
    step_free(&mut p, Pad::default(), true);
    step_free(&mut p, Pad { attack: true, ..Default::default() }, true);
    assert!(p.ours.swing.is_some());
    until_free(&mut p);
    assert!(p.ours.swing.is_some(), "still the swing's motion in its free frames");
    let out = step_free(&mut p, run_square(), true);
    assert!(out.recom_roll);
    assert_eq!(p.recom.order, vec!["end swing", "Re:CoM's roll"]);
    assert!(p.ours.swing.is_none());
    // With Re:CoM's battle won, the swing's end enters Re:CoM's win instead: no roll.
    let mut p = recom_roll_pair(all());
    step_free(&mut p, Pad::default(), true);
    step_free(&mut p, Pad { attack: true, ..Default::default() }, true);
    until_free(&mut p);
    p.recom.win_pending = true;
    let out = step_free(&mut p, run_square(), true);
    assert!(out.recom_took && !out.recom_roll);
    assert_eq!(p.recom.recom_rolls, 0);
    assert_eq!(p.recom.act.id, crate::battle::end::WIN_ACTION);
}

#[test]
fn a_roll_out_of_guards_recoil_leaves_the_untouchable_bit_to_recom() {
    let mut p = recom_roll_pair(Abilities { counterattack: true, ..all() });
    for k in 0..20 {
        step_free(&mut p, Pad { square: k == 4, ..Default::default() }, true);
    }
    p.recom.block_next = true;
    let out = step_free(&mut p, Pad::default(), true);
    assert!(out.events.contains(&Event::GuardSucceeded));
    assert!(p.recom.untouchable, "KH1's flag 0x80 on Re:CoM's bit");
    until_free(&mut p);
    let writes = p.recom.untouchable_writes;
    let out = step_free(&mut p, run_square(), true);
    assert!(out.recom_roll);
    assert!(p.recom.untouchable && p.recom.untouchable_writes == writes, "Re:CoM's init owns the bit now");
    // Re:CoM's exe clears it; after the roll nothing of ours writes it (KH1's 8 frames were inside).
    p.recom.untouchable = false;
    p.recom.act = Action { id: action::STAND, sub: 0, phase: 0 };
    for _ in 0..20 {
        step_free(&mut p, Pad::default(), true);
    }
    assert!(!p.recom.untouchable && p.recom.untouchable_writes == writes);
    assert!(!p.recom.guard, "the guard object is off");
}

#[test]
fn a_kh1_swings_pending_travel_lands_once_on_recoms_roll() {
    // The root-step wrapper hands Re:CoM's roll its own travel, plus once the swing's last
    // update's travel KH1 would have applied in the next frame.
    let mut d = RootDelay::default();
    let a = Vec3::new(1.0, 0.0, 2.0);
    let v = Vec3::new(0.0, 0.0, 0.05);
    assert_eq!(d.position(5, 10, a, a + v, 1.0), a);
    assert!((d.swap(5, 11, Vec3::ZERO) - v).length() < 1e-6, "the roll's first update");
    assert_eq!(d.swap(5, 12, Vec3::ZERO), Vec3::ZERO, "then nothing of ours");
}

// ---- Speed toggles (`run_speed`, `jump_height`, `air_speed`; owner 2026-11-02, `speeds`) ----

fn with_speeds(ab: Abilities, run: Mode, jump: Mode, air: Mode) -> Pair {
    let mut p = Pair::new(ab, Floor::flat());
    p.ours.speeds = speeds::Speeds { run, jump, air };
    p
}

fn horizontal(a: Vec3, b: Vec3) -> f32 {
    (b - a).flat().length()
}

#[test]
fn run_speed_recom_runs_at_recoms_6_2_mps_and_walks_kh1s() {
    let mut p = with_speeds(Abilities::default(), Mode::Recom, Mode::Kh1, Mode::Kh1);
    let f: Vec<Frame> = (0..120).map(|k| p.step(match k { 0..=39 => stick(0.0, 0.5), 40..=79 => stick(0.0, 1.0), _ => stick(0.0, 0.9) })).collect();
    // Walk: KH1's 2 units (1.2 m/s).
    assert!((horizontal(f[29].ours, f[30].ours) - 2.0).abs() < 1e-3);
    // Run: Re:CoM's 6.2 m/s × the stick = 10.33 units a frame (KH1: 8).
    assert!((horizontal(f[69].ours, f[70].ours) - 10.333_333).abs() < 1e-3);
    assert!((horizontal(f[109].ours, f[110].ours) - 9.3).abs() < 1e-3);
    assert_eq!(p.recom.act.id, action::RUN);
    // Re:CoM's stick class sees Re:CoM's speed (`player+0x4DC`, m/s).
    let (class, mps) = speed_class_run(&p.ours.params, 1.0, speeds::recom_run(&speeds::RecomNumbers::EXE, 1.0));
    assert_eq!(class, 2);
    assert!((mps - 6.2).abs() < 1e-5);
    // At kh1, KH1's 8.
    let mut q = with_speeds(Abilities::default(), Mode::Kh1, Mode::Kh1, Mode::Kh1);
    let f: Vec<Frame> = (0..60).map(|_| q.step(stick(0.0, 1.0))).collect();
    assert!((horizontal(f[49].ours, f[50].ours) - 8.0).abs() < 1e-3);
}

#[test]
fn jump_height_recom_is_recoms_jump_after_recoms_crouch() {
    for (high, top) in [(false, 198.94), (true, 266.8)] {
        let mut p = with_speeds(Abilities { high_jump: high, ..Default::default() }, Mode::Kh1, Mode::Recom, Mode::Kh1);
        let f: Vec<Frame> = (0..120).map(|k| p.step(Pad { jump: k < 70, ..Default::default() })).collect();
        // Re:CoM's crouch (`0x1401EDA60` phase 1, `params+0x24` = 100 units): the press frame and 2 crouch frames
        // on the ground, the take-off on frame 3 (KH1: frame 4).
        assert_eq!((f[0].ours.y, f[1].ours.y, f[2].ours.y), (0.0, 0.0, 0.0), "Re:CoM's crouch");
        assert!(f[3].ours.y < 0.0, "the take-off on the 3rd frame after the press: {}", f[3].ours.y);
        assert!(f[3].out.events.contains(&Event::TookOff) && !f[2].out.events.contains(&Event::TookOff));
        let peak = f.iter().map(|x| -x.ours.y).fold(0.0, f32::max);
        assert!((peak - top).abs() < 0.1, "high {high}: peak {peak}");
        // The take-off frame rises at Re:CoM's take-off speed (13.72 units, 15.93 high).
        let first = f.iter().position(|x| x.ours.y < 0.0).unwrap();
        let v0 = if high { 32.5 } else { 28.0 } * 0.6 * 0.816_666_7;
        assert!((-f[first].ours.y - v0).abs() < 1e-3, "{}", f[first].ours.y);
        assert!(f.iter().any(|x| x.out.events.contains(&Event::Landed)));
    }
    // Released at once: Re:CoM's cut 12 frames after the take-off (speed × 0.43), lower than held.
    let mut p = with_speeds(Abilities::default(), Mode::Kh1, Mode::Recom, Mode::Kh1);
    let f: Vec<Frame> = (0..80).map(|k| p.step(Pad { jump: k == 0, ..Default::default() })).collect();
    let first = f.iter().position(|x| x.ours.y < 0.0).unwrap();
    let up = |k: usize| f[k - 1].ours.y - f[k].ours.y;
    assert!((up(first + 11) - (13.72 - 11.0 * 0.49)).abs() < 1e-2);
    assert!((up(first + 12) - (13.72 - 12.0 * 0.49) * 0.43).abs() < 1e-2);
    assert!(!f.iter().any(|x| x.out.events.contains(&Event::JumpCut)), "KH1's cut doesn't run");
    assert_eq!(first, 3, "a tapped jump takes off after Re:CoM's crouch too");
}

#[test]
fn jump_height_kh1_keeps_kh1s_4_frame_crouch() {
    for run in [Mode::Kh1, Mode::Recom] {
        for air in [Mode::Kh1, Mode::Recom] {
            let mut p = with_speeds(Abilities::default(), run, Mode::Kh1, air);
            let f: Vec<Frame> = (0..40).map(|k| p.step(Pad { jump: k < 30, ..Default::default() })).collect();
            assert!(f[..4].iter().all(|x| x.ours.y == 0.0), "KH1's 4 crouch frames");
            assert!(f[4].ours.y < 0.0 && f[4].out.events.contains(&Event::TookOff), "the take-off on frame 4");
            assert!(p.ours.recom_crouch.is_none());
        }
    }
}

#[test]
fn jump_height_recom_land_jump_has_recoms_whole_crouch() {
    // KH1's land jump starts its crouch at t = 2 (`0x1402A6220`: 2 frames on the ground); Re:CoM's land jump runs its
    // jump init (`0x1401EE360` → `0x1401F29F0`), whose crouch starts from 0 like any jump.
    for (jump, ground) in [(Mode::Kh1, 2), (Mode::Recom, 3)] {
        let mut p = with_speeds(Abilities::default(), Mode::Kh1, jump, Mode::Kh1);
        let mut k = 0;
        while !p.step(Pad { jump: k < 2, ..Default::default() }).out.events.contains(&Event::Landed) {
            k += 1;
            assert!(k < 200, "{jump:?}: never landed");
        }
        // Pressed 2 frames into the landing lock (12 frames).
        let f: Vec<Frame> = (0..30).map(|k| p.step(Pad { jump: k == 2, ..Default::default() })).collect();
        let took = f.iter().position(|x| x.out.events.contains(&Event::TookOff)).unwrap_or_else(|| panic!("{jump:?}: no land jump"));
        assert_eq!(took - 2, ground, "{jump:?}");
    }
}

#[test]
fn air_speed_recom_keeps_0_8_of_the_ground_speed_and_eases() {
    // A running jump: 0.8 of the run at the take-off, held there.
    let mut p = with_speeds(Abilities::default(), Mode::Kh1, Mode::Kh1, Mode::Recom);
    let f: Vec<Frame> = (0..80).map(|k| p.step(Pad { jump: (30..60).contains(&k), stick: [0.0, 1.0], ..Default::default() })).collect();
    let first = f.iter().position(|x| x.ours.y < 0.0).unwrap();
    assert!((horizontal(f[first - 1].ours, f[first].ours) - 6.4).abs() < 1e-3);
    assert!((horizontal(f[first + 5].ours, f[first + 6].ours) - 6.4).abs() < 1e-3);
    // A standing jump, then the stick: 5 % of the gap to 0.8 × run a frame.
    let mut p = with_speeds(Abilities::default(), Mode::Kh1, Mode::Kh1, Mode::Recom);
    let f: Vec<Frame> = (0..30).map(|k| p.step(Pad { jump: k < 25, stick: if k >= 10 { [0.0, 1.0] } else { [0.0, 0.0] }, ..Default::default() })).collect();
    assert!((horizontal(f[9].ours, f[10].ours) - 0.05 * 6.4).abs() < 1e-3);
    assert!((horizontal(f[10].ours, f[11].ours) - (0.32 + 0.05 * (6.4 - 0.32))).abs() < 1e-3);
    // With Re:CoM's run: 0.8 × 10.33.
    let mut p = with_speeds(Abilities::default(), Mode::Recom, Mode::Kh1, Mode::Recom);
    let f: Vec<Frame> = (0..80).map(|k| p.step(Pad { jump: (30..60).contains(&k), stick: [0.0, 1.0], ..Default::default() })).collect();
    let first = f.iter().position(|x| x.ours.y < 0.0).unwrap();
    assert!((horizontal(f[first + 3].ours, f[first + 4].ours) - 0.8 * 10.333_333).abs() < 1e-3);
}

#[test]
fn kh1_air_keeps_a_recom_run_through_the_jump() {
    // The defaults (run_speed = recom, air_speed = kh1): KH1's air rule keeps a take-off speed
    // above its run speed (`0x1402AF6A0`'s cap), so a running jump goes on at Re:CoM's run.
    let mut p = with_speeds(Abilities::default(), Mode::Recom, Mode::Kh1, Mode::Kh1);
    let f: Vec<Frame> = (0..80).map(|k| p.step(Pad { jump: (30..60).contains(&k), stick: [0.0, 1.0], ..Default::default() })).collect();
    let first = f.iter().position(|x| x.ours.y < 0.0).unwrap();
    for k in first..first + 20 {
        assert!((horizontal(f[k].ours, f[k + 1].ours) - 10.333_333).abs() < 1e-3, "frame {k}");
    }
    // A standing jump tops out at KH1's 8.
    let mut p = with_speeds(Abilities::default(), Mode::Recom, Mode::Kh1, Mode::Kh1);
    let f: Vec<Frame> = (0..60).map(|k| p.step(Pad { jump: k < 50, stick: if k >= 6 { [0.0, 1.0] } else { [0.0, 0.0] }, ..Default::default() })).collect();
    let fastest = (6..50).map(|k| horizontal(f[k].ours, f[k + 1].ours)).fold(0.0, f32::max);
    assert!((fastest - 8.0).abs() < 1e-3, "{fastest}");
}

#[test]
fn every_combination_runs_jumps_and_lands() {
    for b in 0..8u8 {
        let m = |bit: u8| if b & bit != 0 { Mode::Recom } else { Mode::Kh1 };
        let mut p = with_speeds(all(), m(1), m(2), m(4));
        let f: Vec<Frame> = (0..300)
            .map(|k| {
                let s = match k % 100 {
                    0..=19 => [0.0, 1.0],
                    20..=39 => [1.0, 0.0],
                    40..=59 => [-0.6, -0.6],
                    _ => [0.0, 0.0],
                };
                p.step(Pad { jump: k % 50 < 20, stick: s, ..Default::default() })
            })
            .collect();
        assert!(f.iter().all(|x| x.ours.x.is_finite() && x.ours.y.is_finite() && x.ours.z.is_finite()), "combination {b}");
        assert!(f.iter().filter(|x| x.out.events.contains(&Event::Landed)).count() >= 3, "combination {b}");
    }
}

/// 0.183.0: the Guard faces KH1's lock pair, which holds Auto Lock's soft target too (`0x1402A3B00`
/// reads `0x142DB3518`, states 1–4; `read_pad` puts it in `lock_target` on a Square press), and sets the
/// combo counter to 1.
#[test]
fn guard_faces_the_soft_target_and_restarts_the_combo() {
    let mut p = Pair::new(Abilities { counterattack: true, ..all() }, Floor::flat());
    let side = kh1_core::TargetPart { owner: 1, centre: Vec3::new(150.0, 0.0, 0.0), radius: 30.0 };
    let press = |p: &mut Pair, square: bool, locked: bool| {
        let sq = Button::from_held(p.prev.square, square);
        p.prev = Pad { square, ..Default::default() };
        let out = p.ours.frame(&mut p.recom, &FrameInput { move_dir: Vec3::ZERO, tilt: 0.0, recom_tilt: 0.0, jump: Button::default(), square: sq, lock_target: locked.then_some(side.centre), attack: Button::default(), target: None, locked: false, magic: None, item: None, special: None }, p.first);
        p.first = false;
        p.recom.after_exe();
        out
    };
    for _ in 0..4 {
        press(&mut p, false, true);
    }
    p.ours.combo_counter = 2;
    let out = press(&mut p, true, true);
    assert!(out.events.contains(&Event::Guarded), "{:?}", out.events);
    let want = Vec3::new(1.0, 0.0, 0.0).yaw();
    assert!(crate::movement::wrap_angle(p.ours.target_yaw - want).abs() < 1e-4, "turned to the soft target: {} vs {want}", p.ours.target_yaw);
    assert_eq!(p.ours.combo_counter, 1, "the Guard sets KH1's combo counter to 1");

    // Not locked (Auto Lock off, no R1): Re:CoM's attack target is not KH1's lock pair, he keeps his facing.
    let mut q = Pair::new(Abilities { counterattack: true, ..all() }, Floor::flat());
    for _ in 0..4 {
        press(&mut q, false, false);
    }
    let before = q.ours.facing;
    press(&mut q, true, false);
    assert!(crate::movement::wrap_angle(q.ours.target_yaw - before).abs() < 1e-4, "kept facing: {} vs {before}", q.ours.target_yaw);
}

/// 0.183.1: a KH1 enemy's attack 40+ over the guard gives kind 7 (request 0x6F, no Counterattack); one
/// under it, kind 6 with KH1's push back (`0x14029D720`).
#[test]
fn guard_success_takes_kh1s_clash_kind_and_push() {
    for (clash, want, push) in [(Some((7u8, -60)), crate::rows::STRONG_RECOIL, false), (Some((6, 20)), crate::rows::RECOIL, false), (Some((6, -20)), crate::rows::RECOIL, true), (None, crate::rows::RECOIL, false)] {
        let mut p = Pair::new(Abilities { counterattack: true, ..all() }, Floor::flat());
        for k in 0..5 {
            step_with_target(&mut p, Pad { square: k == 4, ..Default::default() });
        }
        for _ in 0..14 {
            step_with_target(&mut p, Pad::default());
        }
        assert!(p.recom.guard, "the guard object is live");
        p.recom.block_next = true;
        p.recom.clash_next = clash;
        let out = step_with_target(&mut p, Pad::default());
        assert!(out.events.contains(&Event::GuardSucceeded), "{clash:?}");
        assert_eq!(p.ours.rec.head(), want, "{clash:?}");
        assert_eq!(p.ours.push.is_some(), push, "{clash:?}");
        if want == crate::rows::STRONG_RECOIL {
            for _ in 0..12 {
                step_with_target(&mut p, Pad::default());
            }
            assert!(p.recom.untouchable && p.ours.lock, "flag 0x80 holds while 0x6F holds the lock");
            for _ in 0..8 {
                step_with_target(&mut p, Pad::default());
            }
            assert!(!p.ours.counter_offered(true), "no Counterattack after kind 7");
        }
    }
}

/// 0.183.4 (fork owner, 2026-11-23: the Guard is slow to start; "faça o teste do neoshadow com o Riku"): Riku's
/// Guard, frame by frame on the shared port, against the Neoshadow's KH1 hit windows (the player's files: the
/// asset step's `neoshadow::tests::builds_from_the_players_files`, Checked (files): the claw 202's hands at
/// frames 40..54 and 52..68 (priority 40), the rising slash 207 at 26..50 and the dive 205 at 0..40 (priority
/// 100: KH1's Guard never stops them, `kh1_core::enemy::guard::meeting`)). The guard object must be live from
/// the 10th frame after the press to the 36th (KH1's motion 212, Checked (files)), and the press window that
/// covers the claw's first hit frame follows from it.
#[test]
fn rikus_guard_against_the_neoshadows_windows() {
    let riku = crate::riku::abilities(0);
    let mut p = Pair::new(riku, Floor::flat());
    for _ in 0..4 {
        step_with_target(&mut p, Pad::default());
    }
    let out = step_with_target(&mut p, Pad { square: true, ..Default::default() });
    assert!(out.events.contains(&Event::Guarded), "Riku's Square at rest starts the Guard: {:?}", out.events);
    // Frames after the press (the press frame is 0) with the guard object live.
    let mut live = Vec::new();
    for k in 1..60 {
        step_with_target(&mut p, Pad::default());
        if p.recom.guard {
            live.push(k);
        }
    }
    let (first, last) = (*live.first().expect("the guard comes on"), *live.last().unwrap());
    assert_eq!(live.len(), last - first + 1, "one unbroken window: {live:?}");
    eprintln!("Riku's guard object: live on frames {first}..={last} after the press ({} frames)", live.len());
    // KH1: frames 10..36 of motion 212. The port runs a frame's motion update at the next handler call (the
    // oracle's order), so Re:CoM's collision of game frame k sees KH1's motion as it was at k − 1: one frame
    // (16 ms) after KH1, the same 26 frames.
    assert_eq!((first, last), (11, 36), "KH1's frames 10..36, one frame later");
    // The Neoshadow's attacks (motion, first and last hit frame, priority) against that window: the press
    // frames (counted from the attack's start) that have the guard live on its first hit frame.
    for (name, from, to, priority) in [("claw (202)", 40, 67, 40), ("rising slash (207)", 26, 49, 100), ("dive (205)", 0, 39, 100)] {
        let presses: Vec<i32> = (-60..from as i32).filter(|&q| (first as i32..=last as i32).contains(&(from as i32 - q))).collect();
        // 0.198.11: KH1's Guard holds against every one of them (kind 6 under 98, kind 7 with no harm at 100)
        let m = kh1_core::enemy::guard::meeting((2, 30, false), (0, priority, false));
        let stops = m.a_unharmed();
        eprintln!(
            "Neoshadow {name}: hits frames {from}..={to}; a press on attack frames {}..={} has the guard up on its first hit; KH1's Guard {}",
            presses.first().unwrap(),
            presses.last().unwrap(),
            if m.b_stops { "stops it" } else if stops { "holds (kind 7, the attack goes on but does no harm)" } else { "does not stop it" }
        );
        assert_eq!(presses.len(), last - first + 1);
        assert!(stops);
        assert_eq!(m.b_stops, priority < 98);
    }
}

/// 0.184.3 (fork owner, 2026-11-23: "e teste com o Neo SHadow todas as variações para ver se falta algo"): every spell,
/// level, on the ground and in the air, cast by Riku's KH1 frame (his abilities): it starts, every motion it asks
/// has one of Riku's cast rows (`crate::riku::cast_row`: no Sora-only clip left), it releases once at KH1's frame
/// (Fire 28 after the command; the others their script's wait), Re:CoM's magic action ends once, and Riku's row
/// table has the row (`crate::magic::riku_look`). Printed as a table with `--nocapture`. The Neoshadow's numbers for
/// each are `magic::tests::every_spell_and_level_on_kh1s_neoshadow`.
#[test]
fn riku_every_spell_level_and_air_casts_fully() {
    use crate::magic::{self as mg, CastEvent, MagicCommand};
    let rows = mg::riku_look(&mg::kh1_magic_table(&mg::recom_rows()));
    let names = ["Fire", "Blizzard", "Thunder", "Cure", "Gravity", "Stop", "Aero"];
    for spell in 0..7u8 {
        for level in 1..=3u8 {
            assert!(mg::table_index(spell, level).is_some_and(|i| rows[i][0] != 0), "{} {level}: Riku's row", names[spell as usize]);
            for air in [false, true] {
                let mut p = Pair::new(crate::riku::abilities(0), Floor::flat());
                for _ in 0..5 {
                    step_cast(&mut p, Pad::default(), None);
                }
                if air {
                    step_cast(&mut p, Pad { jump: true, ..Default::default() }, None);
                    for _ in 0..8 {
                        step_cast(&mut p, Pad { jump: true, ..Default::default() }, None);
                    }
                }
                let out = step_cast(&mut p, Pad { jump: air, ..Default::default() }, Some(MagicCommand { spell, level }));
                let started = out.cast.iter().find_map(|e| match e {
                    CastEvent::Started { air, .. } => Some(*air),
                    _ => None,
                });
                let Some(in_air) = started else { panic!("{} {level} air {air}: no start ({:?})", names[spell as usize], out.cast) };
                let mut release = None;
                for k in 1..200 {
                    let out = step_cast(&mut p, Pad { jump: air, ..Default::default() }, None);
                    if let Some(f) = out.cast.iter().find_map(|e| match e {
                        CastEvent::Released { frame, .. } => Some(*frame),
                        _ => None,
                    }) {
                        assert!(release.is_none(), "{} {level}: released twice", names[spell as usize]);
                        release = Some((k, f));
                    }
                    if p.ours.cast.is_none() && release.is_some() {
                        break;
                    }
                }
                let unmapped: Vec<u16> = p.recom.plays.iter().map(|x| x.0).filter(|&r| mg::is_cast_request(r) && crate::riku::cast_row(r).is_none()).collect();
                eprintln!("{} {level} {}: released {:?}, motions {:x?}", names[spell as usize], if in_air { "air" } else { "ground" }, release, p.recom.plays.iter().map(|x| x.0).filter(|&r| mg::is_cast_request(r)).collect::<Vec<_>>());
                assert!(unmapped.is_empty(), "{} {level}: cast motions without a Riku row {unmapped:x?}", names[spell as usize]);
                // Sora, the same cast on the same frame (the shared port): Riku must do what he does.
                let mut q = Pair::new(all(), Floor::flat());
                for _ in 0..5 {
                    step_cast(&mut q, Pad::default(), None);
                }
                if air {
                    for _ in 0..9 {
                        step_cast(&mut q, Pad { jump: true, ..Default::default() }, None);
                    }
                }
                step_cast(&mut q, Pad { jump: air, ..Default::default() }, Some(MagicCommand { spell, level }));
                for _ in 1..200 {
                    step_cast(&mut q, Pad { jump: air, ..Default::default() }, None);
                }
                assert_eq!(p.recom.releases.len(), q.recom.releases.len(), "{} {level} air {in_air}: Riku and Sora differ", names[spell as usize]);
                if !in_air {
                    assert!(release.is_some(), "{} {level}: never released", names[spell as usize]);
                    assert_eq!(p.recom.releases.len(), 1, "{} {level}", names[spell as usize]);
                } else if release.is_none() {
                    eprintln!("  (air: landed before the release frame; Sora too: KH1's air cast ends at the landing)");
                }
            }
        }
    }
}

// ---- Riku's own combo on KH1's rules (fork owner, 2026-11-27) ----

/// Riku's rows started over `n` frames (ours only: the KH1 oracle is Sora).
fn riku_rows(p: &mut Pair, n: usize, press: &[usize]) -> (Vec<(usize, usize)>, Vec<(usize, kh1_core::Refused)>) {
    let (mut rows, mut refused) = (Vec::new(), Vec::new());
    for k in 0..n {
        let f = p.step(Pad { attack: press.contains(&k), ..Default::default() });
        rows.extend(attack_rows(&f.out.events).into_iter().map(|(r, ..)| (k, r)));
        refused.extend(refusals(&f.out.events).into_iter().map(|r| (k, r)));
    }
    (rows, refused)
}

#[test]
fn rikus_ground_combo_is_his_own_order_and_rhythm() {
    use kh1_core::data::row;
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    p.ours.set_riku_moves(Some(false));
    // A Shadow-sized target 1.5 m ahead (KH1 units, y down).
    p.target = Some(kh1_core::TargetPart { owner: 1, centre: Vec3::new(0.0, -40.0, 150.0), radius: 40.0 });
    p.recom.hit = true;
    // Presses at 1, 20 (swing 1's chain window 13..48) and 45 (the lunge's 21..64, 25 frames in).
    let (rows, refused) = riku_rows(&mut p, 140, &[1, 20, 45]);
    assert_eq!(rows.iter().map(|r| r.1).collect::<Vec<_>>(), vec![row::SWING_1, row::LUNGE, row::GROUND_FINISHER], "{rows:?}");
    assert!(refused.is_empty(), "{refused:?}");
    // The swings start on the press (Re:CoM's chain cuts the swing once it has hit).
    assert_eq!(rows.iter().map(|r| r.0).collect::<Vec<_>>(), vec![1, 20, 45]);
}

#[test]
fn rikus_far_opener_and_a_miss() {
    use kh1_core::data::row;
    let mut p = Pair::new(Abilities::default(), Floor::flat());
    p.ours.set_riku_moves(Some(false));
    // 5 m away: the far opener (Re:CoM's situation 3, the lunge clip).
    p.target = Some(kh1_core::TargetPart { owner: 1, centre: Vec3::new(0.0, -40.0, 500.0), radius: 40.0 });
    // Nothing hit: KH1's rule takes no next press in the window.
    p.recom.hit = false;
    let (rows, refused) = riku_rows(&mut p, 60, &[1, 30]);
    assert_eq!(rows.iter().map(|r| r.1).collect::<Vec<_>>(), vec![row::LUNGE]);
    assert_eq!(refused, vec![(30, kh1_core::Refused::LastSwingMissed)]);
}
