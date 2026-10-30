//! `--replay <trace.csv>`: feeds a recorded trace's per-frame inputs (stick, camera yaw, buttons,
//! lock, frame step) into the controller from the recorded start (position 0, the recorded
//! facing and abilities) and writes our own trace, for `kh1-tracediff`. Headless: no window.
//!
//! The KH1 world is not rebuilt: Sora replays on the arena's flat floor. With the `kh1` layout
//! (the default) the platform is moved out of the way and the only dummy stands where the KH1
//! lock target stood (relative to Sora's start), or there is none when the trace has no lock
//! target. Assumption: the dummy's body (one column, radius 40) stands in for the KH1 enemy's;
//! its parts and radius differ, so combo rows picked by range can differ near the thresholds.

use kh1_core::{Abilities, Dummy, Vec3};
use kh1_trace::schema::{Row, Trace};

use crate::sim::{DummyInfo, Key, Pad, Sim};

/// Where the dummies stand for the replay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// One dummy at the KH1 lock target, or none; no platform.
    Kh1,
    /// The playground's own arena (for replaying a playground trace).
    Playground,
}

/// A frame stepper the replay drives: the full [`Sim`], or a bare controller in tests.
pub trait Stepper {
    /// Steps one frame and returns its trace row.
    fn step_row(&mut self, pad: &Pad, step: f32) -> Row;
    fn locked(&self) -> bool;
    /// Something stands that a lock-on could take.
    fn can_lock(&self) -> bool;
}

impl Stepper for Sim {
    fn step_row(&mut self, pad: &Pad, step: f32) -> Row {
        let out = self.step_dt(pad, step).clone();
        crate::trace::row(self, &out, pad, step)
    }
    fn locked(&self) -> bool {
        self.lock.is_some()
    }
    fn can_lock(&self) -> bool {
        self.dummies.iter().any(|d| d.stats.hp > 0)
    }
}

fn key(v: i32) -> Key {
    Key { held: v & 1 != 0, pressed: v & 2 != 0 }
}

/// The pad and frame step for one recorded row. `locked`: whether the stepper is locked on now;
/// the lock is toggled when the recorded lock differs and a target exists.
pub fn pad_for(r: &Row, locked: bool, can_lock: bool) -> (Pad, f32) {
    let pad = Pad {
        stick: [r.in_stick_x, r.in_stick_y],
        camera_yaw: r.in_cam_yaw,
        jump: key(r.in_jump),
        attack: key(r.in_attack),
        square: key(r.in_square),
        lock_toggle: r.in_lock != locked && (locked || can_lock),
    };
    let step = if r.step.is_finite() && r.step > 0.0 { r.step } else { 1.0 };
    (pad, step)
}

/// Replays every row's inputs; returns the stepper's rows.
pub fn run<S: Stepper>(s: &mut S, rows: &[Row]) -> Vec<Row> {
    rows.iter()
        .map(|r| {
            let (pad, step) = pad_for(r, s.locked(), s.can_lock());
            s.step_row(&pad, step)
        })
        .collect()
}

/// KH1's ability bits (`0x142D5EC10`) and combo lengths back to kh1-core's abilities.
pub fn abilities_from(r: &Row) -> Abilities {
    let b = r.abilities.0;
    let on = |m: u32| b & m != 0;
    Abilities {
        high_jump: on(0x200),
        glide: on(0x80),
        superglide: on(0x100),
        guard: on(0x10),
        dodge_roll: on(0x20),
        combo_plus: (r.combo_max - 3).clamp(0, 255) as u8,
        air_combo_plus: (r.air_combo_max - 3).clamp(0, 255) as u8,
        critical_plus: 0,
        combo_master: r.combo_master,
        vortex: on(0x1),
        aerial_sweep: on(0x2),
        blitz: on(0x8),
        sliding_dash: on(0x100_0000),
        hurricane_blast: on(0x200_0000),
        ripple_drive: on(0x400_0000),
        stun_impact: on(0x800_0000),
        gravity_break: on(0x1000_0000),
        zantetsuken: on(0x2000_0000),
    }
}

/// Puts the sim in the recorded start state: abilities, Sora at 0 with the recorded facing, the
/// dummies for `layout`.
pub fn setup(sim: &mut Sim, trace: &Trace, layout: Layout) {
    let Some(r0) = trace.rows.first() else { return };
    if trace.has("abilities") && trace.has("combo_max") && r0.combo_max >= 3 {
        sim.ctrl.abilities = abilities_from(r0);
    }
    sim.reset();
    if r0.facing.is_finite() {
        sim.ctrl.facing = r0.facing;
        sim.ctrl.target_yaw = if r0.target_yaw.is_finite() { r0.target_yaw } else { r0.facing };
    }
    if layout == Layout::Kh1 {
        sim.arena.platform.x = 1.0e6;
        sim.dummies.clear();
        sim.info.clear();
        let origin = Vec3::new(r0.pos_x, r0.pos_y, r0.pos_z);
        let target = trace.rows.iter().find(|r| r.target_x.is_finite() && r.target_z.is_finite());
        if let (Some(t), true) = (target, origin.x.is_finite()) {
            let rel = Vec3::new(t.target_x, if t.target_y.is_finite() { t.target_y } else { origin.y }, t.target_z) - origin;
            let mut d = Dummy::standing(1, rel, sim.opts.dummy.clone());
            d.facing = (Vec3::ZERO - rel).yaw();
            sim.dummies.push(d);
            sim.info.push(DummyInfo { name: "KH1 target", home: rel, last_hit: None, total: 0, respawn: 0 });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::core_row;
    use kh1_core::{Button, Controller, FlatFloor, Input, Moveset, Params};
    use kh1_trace::schema;

    /// kh1-core's controller alone, on a flat floor, with KH1's default numbers (no game files).
    struct Core {
        ctrl: Controller,
        frame: u64,
    }

    impl Stepper for Core {
        fn step_row(&mut self, pad: &Pad, step: f32) -> Row {
            let (move_dir, tilt) = Input::move_from_stick(pad.stick, pad.camera_yaw);
            let b = |k: Key| Button { pressed: k.pressed, held: k.held };
            let input = Input { move_dir, tilt, jump: b(pad.jump), attack: b(pad.attack), square: b(pad.square), lock: None };
            let out = self.ctrl.step(&input, &[], &mut FlatFloor { floor_y: 0.0 }, step);
            let r = core_row(self.frame, &self.ctrl, &out, pad, step);
            self.frame += 1;
            r
        }
        fn locked(&self) -> bool {
            false
        }
        fn can_lock(&self) -> bool {
            false
        }
    }

    fn core() -> Core {
        let ab = Abilities { guard: true, dodge_roll: true, glide: true, high_jump: true, ..Default::default() };
        Core { ctrl: Controller::new(Params::kh1_defaults(), ab, Moveset::kh1_defaults(), Vec3::ZERO), frame: 0 }
    }

    /// A scripted run touching every input: walk and run under a turned camera, a jump with an
    /// early release, glide, a roll, Attack presses, a 30 fps stretch (step 2).
    fn scripted(f: u64) -> (Pad, f32) {
        let press = |at: &[u64]| Key { held: at.contains(&f), pressed: at.contains(&f) };
        let mut p = Pad { camera_yaw: 0.7 + f as f32 * 0.003, ..Default::default() };
        p.stick = match f {
            0..=9 => [0.0, 0.0],
            10..=39 => [0.25, 0.4],
            40..=99 => [-0.6, 0.8],
            100..=139 => [0.0, -1.0],
            _ => [0.0, 0.0],
        };
        p.jump = Key { held: (60..=66).contains(&f) || (110..=150).contains(&f) || (151..=175).contains(&f), pressed: f == 60 || f == 110 || f == 151 };
        p.square = press(&[200, 260]);
        p.attack = press(&[230, 238, 246, 300, 301]);
        let step = if (280..290).contains(&f) { 2.0 } else { 1.0 };
        (p, step)
    }

    fn csv(rows: &[Row]) -> String {
        let mut buf = Vec::new();
        schema::write_header(&mut buf).unwrap();
        for r in rows {
            r.write_csv(&mut buf).unwrap();
        }
        String::from_utf8(buf).unwrap()
    }

    /// Record a scripted run in the trace format, read it back, replay its inputs into a fresh
    /// controller: the replay's trace is the same file, byte for byte.
    #[test]
    fn replay_is_deterministic() {
        let mut a = core();
        let rows: Vec<Row> = (0..340).map(|f| {
            let (p, s) = scripted(f);
            a.step_row(&p, s)
        }).collect();
        let text = csv(&rows);
        let read = schema::parse(&text).unwrap();
        let mut b = core();
        let again = run(&mut b, &read.rows);
        assert_eq!(csv(&again), text);
        // The run did something: it moved, left the ground and attacked.
        assert!(rows.iter().any(|r| r.state == 2));
        assert!(rows.iter().any(|r| r.events.contains("attack(")));
        assert!(rows.last().unwrap().pos_z.abs() + rows.last().unwrap().pos_x.abs() > 100.0);
    }

    #[test]
    fn abilities_round_trip() {
        let a = Abilities { high_jump: true, superglide: true, dodge_roll: true, combo_plus: 2, air_combo_plus: 1, combo_master: true, vortex: true, zantetsuken: true, ..Default::default() };
        let r = Row { abilities: schema::Hex(crate::trace::ability_bits(&a)), combo_max: 5, air_combo_max: 4, combo_master: true, ..Default::default() };
        assert_eq!(abilities_from(&r), a);
    }

    /// The trace crate's stick formula is kh1-core's.
    #[test]
    fn move_formula_is_kh1_cores() {
        for &(s, yaw) in &[([0.3f32, 0.9f32], 0.8f32), ([-1.0, 0.0], -2.0), ([0.1, -0.2], 3.0)] {
            let (v, _) = Input::move_from_stick(s, yaw);
            let m = schema::move_from_stick(s, yaw).unwrap();
            assert!((v.x - m[0]).abs() < 1e-6 && (v.z - m[1]).abs() < 1e-6);
        }
    }

    fn kit() -> Option<std::sync::Arc<kh1_data::SoraKit>> {
        let dir = std::path::PathBuf::from(std::env::var_os("KH1_TEST_DATA")?);
        let read = |n: &str| std::fs::read(dir.join(n)).ok();
        let (m, d, b) = (read(kh1_data::MSET)?, read(kh1_data::MDLS)?, read(kh1_data::BTLTBL)?);
        Some(std::sync::Arc::new(kh1_data::SoraKit::from_files(&m, &d, b).unwrap()))
    }

    /// The full sim (posing, hits, dummies): the scripted combo, recorded and replayed in the
    /// playground's own arena, gives the same trace.
    #[test]
    fn sim_replay_is_deterministic() {
        let Some(kit) = kit() else { return };
        let mut sim = Sim::new(kit.clone(), crate::sim::Options::default());
        let mut script = crate::script::Script::new("combo").unwrap();
        let mut rows = Vec::new();
        for _ in 0..240 {
            let pad = script.pad(&sim);
            rows.push(sim.step_row(&pad, 1.0));
        }
        assert!(rows.iter().any(|r| !r.hits.is_empty()), "the combo hit");
        let text = csv(&rows);
        let trace = schema::parse(&text).unwrap();
        let mut again = Sim::new(kit, crate::sim::Options::default());
        setup(&mut again, &trace, Layout::Playground);
        assert_eq!(csv(&run(&mut again, &trace.rows)), text);
    }

    /// The KH1 layout: one dummy at the lock target relative to Sora's start, facing kept.
    #[test]
    fn kh1_layout_places_the_target() {
        let Some(kit) = kit() else { return };
        let mut sim = Sim::new(kit, crate::sim::Options::default());
        let rows = vec![
            Row { pos_x: 100.0, pos_y: -20.0, pos_z: 50.0, facing: 1.0, target_yaw: 1.0, abilities: schema::Hex(0x200), combo_max: 4, air_combo_max: 3, ..Default::default() },
            Row { pos_x: 100.0, pos_y: -20.0, pos_z: 50.0, facing: 1.0, in_lock: true, target_x: 100.0, target_y: -20.0, target_z: 300.0, ..Default::default() },
        ];
        let mut text = Vec::new();
        schema::write_header(&mut text).unwrap();
        for r in &rows {
            r.write_csv(&mut text).unwrap();
        }
        let trace = schema::parse(std::str::from_utf8(&text).unwrap()).unwrap();
        setup(&mut sim, &trace, Layout::Kh1);
        assert_eq!(sim.dummies.len(), 1);
        assert_eq!(sim.dummies[0].pos, Vec3::new(0.0, 0.0, 250.0));
        assert_eq!(sim.ctrl.facing, 1.0);
        assert!(sim.ctrl.abilities.high_jump && sim.ctrl.abilities.combo_plus == 1);
        let out = run(&mut sim, &trace.rows);
        assert!(out[1].in_lock, "locked on to the target");
    }
}
