//! A training dummy and KH1's enemy hit reactions: flinch (`0x1402B8CC0`, ground) and launch
//! (`0x1402B8530`). Checked (decomp) for the numbers; the dummy's own body (its motions, how long
//! it stays down) is host data.

use crate::arc::Arc;
use crate::damage::DefenderStats;
use crate::math::{wrap_angle, Vec3};
use crate::rng::Kh1Rand;
use crate::shape::{Shape, ShapeKind};

/// The btltbl G numbers the reactions read, and the dummy's own timings.
#[derive(Clone, Debug, PartialEq)]
pub struct ReactionParams {
    /// G+0x14 / +0x16: launch speed = min + (max − min) × weight (20 / 35).
    pub launch_speed_min: f32,
    pub launch_speed_max: f32,
    /// G+0x18: launch hop = speed × this (1.35).
    pub launch_height: f32,
    /// Down time = rand × this (30, `0x1402B8530`).
    pub down_time_max: f32,
    /// Air flinch (`0x1402B77D0`): hop 20, speed G+0x3C (16).
    pub air_flinch_hop: f32,
    pub air_flinch_speed: f32,
    /// Motion requests: flinch 0x4C (front) / 0x4D (behind); launch 0x52 on the ground, 0x5C in
    /// the air, +5 when hit from behind.
    pub flinch_request: u16,
    pub launch_request: u16,
    pub air_launch_request: u16,
    /// Assumption: the dummy's flinch motion length (an enemy's own motion data).
    pub flinch_frames: f32,
    /// The enemy-side air physics. Assumption: enemies use Sora's stats-block defaults
    /// (`0x1402A5D70`: accel 0.1, drag 0.95, cap 8) and the same arc (`0x1402AF6A0`).
    pub gravity: f32,
    pub fall_cap: f32,
    pub air_drag: f32,
}

impl Default for ReactionParams {
    fn default() -> Self {
        ReactionParams {
            launch_speed_min: 20.0,
            launch_speed_max: 35.0,
            launch_height: 1.35,
            down_time_max: 30.0,
            air_flinch_hop: 20.0,
            air_flinch_speed: 16.0,
            flinch_request: 0x4C,
            launch_request: 0x52,
            air_launch_request: 0x5C,
            flinch_frames: 30.0,
            gravity: crate::params::GRAVITY,
            fall_cap: 20.0,
            air_drag: 0.95,
        }
    }
}

/// What a hit made the dummy do.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Reaction {
    None,
    /// Kind 3 on the ground: flinch request (front / behind), facing the attacker.
    Flinch { request: u16 },
    /// Kind 3 in the air: hop and drift away.
    AirFlinch { request: u16, speed: f32 },
    /// Kind 5: launched with `speed`, a hop of `hop`, then down for `down_time` frames.
    Launch { request: u16, speed: f32, hop: f32, down_time: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DummyState {
    Idle,
    Flinch { left: f32 },
    /// In the air (state 2 physics with the air velocity). `keep`: the first frame keeps its
    /// speed (a launch sets the speed word, so its first air step accelerates instead of drag).
    Air { arc: Arc, keep: bool },
    Down { left: f32 },
}

/// One collision part, relative to the dummy's feet (y down: −y is up).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DummyPart {
    pub kind: ShapeKind,
    pub offset: Vec3,
    pub radius: f32,
    pub half_height: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dummy {
    pub id: u32,
    pub pos: Vec3,
    pub facing: f32,
    pub stats: DefenderStats,
    pub parts: Vec<DummyPart>,
    pub state: DummyState,
    /// The last reaction's motion request.
    pub request: Option<u16>,
    /// +0x3C0 and +0x3E4.
    pub air_vel: Vec3,
    pub down_time: f32,
    /// Untouchable (flags & 0x8048 must be 0x8000 to be hit): the host sets it.
    pub untouchable: bool,
}

impl Dummy {
    /// A standing column 160 tall, radius 40 (an Assumption-free shape the demo can draw).
    pub fn standing(id: u32, pos: Vec3, stats: DefenderStats) -> Dummy {
        Dummy {
            id,
            pos,
            facing: 0.0,
            stats,
            parts: vec![DummyPart { kind: ShapeKind::Column, offset: Vec3::new(0.0, -80.0, 0.0), radius: 40.0, half_height: 80.0 }],
            state: DummyState::Idle,
            request: None,
            air_vel: Vec3::ZERO,
            down_time: 0.0,
            untouchable: false,
        }
    }

    pub fn shapes(&self) -> Vec<Shape> {
        self.parts.iter().map(|p| Shape::new(p.kind, self.pos + p.offset, p.radius, p.half_height)).collect()
    }

    /// The parts as the combo pickers see them (centre, radius).
    pub fn target_parts(&self) -> Vec<crate::combo::TargetPart> {
        self.parts.iter().map(|p| crate::combo::TargetPart { owner: self.id, centre: self.pos + p.offset, radius: p.radius }).collect()
    }

    pub fn airborne(&self) -> bool {
        matches!(self.state, DummyState::Air { .. })
    }

    /// The reaction for an enemy-side reaction kind (+0x0C) along `dir` (the hit's push
    /// direction, horizontal unit). Kinds other than 3 and 5 give none here (Assumption: kind 4
    /// knock-back is not traced for this demo).
    pub fn react(&mut self, kind: u8, dir: Vec3, p: &ReactionParams, rng: &mut Kh1Rand) -> Reaction {
        let behind = Vec3::from_yaw(self.facing).dot(dir) >= 0.0;
        let face = |d: Vec3| if behind { d.x.atan2(d.z) } else { (-d.x).atan2(-d.z) };
        match kind {
            3 if !self.airborne() => {
                let r = p.flinch_request + behind as u16;
                self.facing = face(dir);
                self.request = Some(r);
                self.state = DummyState::Flinch { left: p.flinch_frames };
                Reaction::Flinch { request: r }
            }
            3 => {
                // `0x1402B77D0`: 0x50 / 0x51, hop 20, velocity dir × 16.
                let r = if behind { 0x51 } else { 0x50 };
                self.facing = face(dir);
                self.request = Some(r);
                self.state = DummyState::Air { arc: Arc::hop(p.air_flinch_hop, self.pos.y, p.gravity), keep: false };
                self.air_vel = dir.normalized() * p.air_flinch_speed;
                Reaction::AirFlinch { request: r, speed: p.air_flinch_speed }
            }
            5 => {
                let speed = (p.launch_speed_max - p.launch_speed_min) * self.stats.launch_weight + p.launch_speed_min;
                let k = p.launch_height;
                let roll = rng.next_f32();
                let base = if self.airborne() { p.air_launch_request } else { p.launch_request };
                let r = if behind { base + 5 } else { base };
                self.facing = wrap_angle(face(dir));
                self.request = Some(r);
                let hop = speed * k;
                self.state = DummyState::Air { arc: Arc::hop(hop, self.pos.y, p.gravity), keep: true };
                self.air_vel = dir.normalized() * speed;
                self.down_time = roll * p.down_time_max;
                Reaction::Launch { request: r, speed, hop, down_time: self.down_time }
            }
            _ => Reaction::None,
        }
    }

    /// One frame of the dummy's own movement, on a floor at `floor_y`.
    pub fn step(&mut self, p: &ReactionParams, floor_y: f32, step: f32) {
        match &mut self.state {
            DummyState::Idle => {}
            DummyState::Flinch { left } => {
                *left -= step;
                if *left <= 0.0 {
                    self.state = DummyState::Idle;
                    self.request = None;
                }
            }
            DummyState::Down { left } => {
                *left -= step;
                if *left <= 0.0 {
                    self.state = DummyState::Idle;
                    self.request = None;
                }
            }
            DummyState::Air { arc, keep } => {
                let k = if step < 2.0 { p.air_drag.sqrt() } else { p.air_drag };
                if *keep {
                    *keep = false;
                } else {
                    self.air_vel = Vec3::new(self.air_vel.x * k, 0.0, self.air_vel.z * k);
                }
                let dy = arc.advance(self.pos.y, step, p.gravity, p.fall_cap);
                let mut d = self.air_vel * step;
                d.y = dy;
                self.pos += d;
                if self.pos.y >= floor_y && !arc.rising() {
                    self.pos.y = floor_y;
                    self.air_vel = Vec3::ZERO;
                    // Assumption: the down time (+0x3E4) counts once it lands.
                    self.state = DummyState::Down { left: self.down_time };
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_numbers() {
        let p = ReactionParams::default();
        let mut d = Dummy::standing(1, Vec3::ZERO, DefenderStats { launch_weight: 1.0, ..Default::default() });
        let mut r = Kh1Rand::new(9);
        match d.react(5, Vec3::new(0.0, 0.0, -1.0), &p, &mut r) {
            Reaction::Launch { request, speed, hop, down_time } => {
                assert_eq!(request, 0x52, "hit from the front");
                assert_eq!(speed, 35.0);
                assert!((hop - 47.25).abs() < 1e-4);
                assert!((0.0..30.0).contains(&down_time));
            }
            x => panic!("{x:?}"),
        }
        let mut d = Dummy::standing(1, Vec3::ZERO, DefenderStats { launch_weight: 0.0, ..Default::default() });
        match d.react(5, Vec3::new(0.0, 0.0, 1.0), &p, &mut r) {
            Reaction::Launch { request, speed, .. } => {
                assert_eq!((request, speed), (0x57, 20.0), "from behind: +5; weight 0: 20");
            }
            x => panic!("{x:?}"),
        }
        let mut n = 0;
        while d.airborne() && n < 200 {
            d.step(&p, 0.0, 1.0);
            n += 1;
        }
        assert!(matches!(d.state, DummyState::Down { .. }));
        assert!(d.pos.z > 20.0);
    }

    #[test]
    fn flinch_faces_the_attacker() {
        let p = ReactionParams::default();
        let mut d = Dummy::standing(1, Vec3::ZERO, DefenderStats::default());
        let mut r = Kh1Rand::new(1);
        assert_eq!(d.react(3, Vec3::new(0.0, 0.0, -1.0), &p, &mut r), Reaction::Flinch { request: 0x4C });
        assert!(d.facing.abs() < 1e-6, "faces +z, toward the attacker");
    }
}
