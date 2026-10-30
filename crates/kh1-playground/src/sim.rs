//! The demo's 60 Hz simulation, engine-free: kh1-core's [`Controller`] drives Sora, kh1-anim
//! poses his skeleton from the motion request and frame the controller outputs, the Keyblade's
//! blade points come from the posed weapon bone, and [`resolve_hits`] / [`Dummy::step`] run the
//! hits and the dummies' reactions. One [`Sim::step`] is one KH1 frame, in kh1-core's order
//! (`kh1-core` crate docs): controller, pose, hits, dummies.
//!
//! Coordinates: kh1-core's world (KH1 units ≈ 1 cm, y down, yaw 0 faces +z). kh1-anim poses in
//! its own model space (identity model matrix): y down, Sora facing −z, root travel inside the
//! bone positions (Checked (files): the hips move −48.33 in z by frame 14 of motion 200, the
//! same as its root curve). So a posed point P goes to the world as
//! `pos + rotate_yaw((−(P − R).x, (P − R).y, −(P − R).z), facing)` with R the pose's root
//! travel when the motion moves Sora by its root (the controller already moved him by it).
//! Assumption: KH1 keeps the root travel out of the model the same way (how it does is not
//! traced, `motion-conversion.md`).

use std::collections::VecDeque;
use std::sync::Arc;

use kh1_anim::{Rig, WEAPON_ATTACH};
use kh1_core::combat::{blade_params, resolve_hits, Blade, HitReport};
use kh1_core::{Abilities, AttackerStats, Button, Controller, DefenderStats, Dummy, DummyState, Event, Input, Output, Shape, Vec3};
use kh1_data::SoraKit;

use crate::arena::{Arena, ArenaWorld, Body};

/// One button as the host latched it between two simulation frames.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Key {
    pub held: bool,
    /// Went down since the last frame (kept even if released again before it).
    pub pressed: bool,
}

/// One frame of player input.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pad {
    /// Stick (x right, y forward), length = tilt (0..=1), dead zone already applied.
    pub stick: [f32; 2],
    /// The camera's yaw in kh1-core terms (stick forward goes this way).
    pub camera_yaw: f32,
    pub jump: Key,
    pub attack: Key,
    pub square: Key,
    /// Lock on to the nearest dummy, or let go.
    pub lock_toggle: bool,
}

/// A dummy's place and what happened to it, for the HUD.
#[derive(Clone, Debug, PartialEq)]
pub struct DummyInfo {
    pub name: &'static str,
    pub home: Vec3,
    /// The last hit: (damage, critical, frame).
    pub last_hit: Option<(i32, bool, u64)>,
    pub total: i32,
    /// Frames left before a beaten dummy stands up again with full HP.
    pub respawn: u32,
}

/// Start-up choices.
#[derive(Clone, Debug)]
pub struct Options {
    pub abilities: Abilities,
    pub attacker: AttackerStats,
    pub dummy: DefenderStats,
    /// Push Sora out of the dummies' bodies (off: he passes through, as with kh1-core's
    /// `FlatFloor`).
    pub push: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            abilities: Abilities { guard: true, dodge_roll: true, glide: true, ..Default::default() },
            attacker: AttackerStats::default(),
            // Demo values (not an enemy's data): 300 HP, Defence 2, level 1.
            dummy: DefenderStats { hp: 300, max_hp: 300, defence: 2, level: 1, ..Default::default() },
            push: true,
        }
    }
}

/// The 60 Hz world.
pub struct Sim {
    pub kit: Arc<SoraKit>,
    pub arena: Arena,
    pub opts: Options,
    pub ctrl: Controller,
    pub rig: Rig,
    pub attacker: AttackerStats,
    pub dummies: Vec<Dummy>,
    pub info: Vec<DummyInfo>,
    /// Frames simulated since start (or the last reset).
    pub frame: u64,
    pub last: Option<Output>,
    /// The locked dummy's id.
    pub lock: Option<u32>,
    /// This frame's hits.
    pub hits: Vec<HitReport>,
    /// This frame's live Keyblade hit spheres (world).
    pub hit_shapes: Vec<Shape>,
    /// The Keyblade line (grip, tip) in the world, from the posed weapon bone.
    pub blade_line: Option<(Vec3, Vec3)>,
    /// Recent events, newest last: (frame, text).
    pub log: VecDeque<(u64, String)>,
    posed_entry: Option<usize>,
}

/// Where Sora starts.
pub const START: Vec3 = Vec3::new(0.0, 0.0, 0.0);
/// Frames a beaten dummy stays down before it stands up with full HP.
const RESPAWN_FRAMES: u32 = 120;

fn layout(arena: &Arena) -> Vec<(&'static str, Vec3)> {
    let p = arena.platform;
    vec![
        // Gap 110 (< close range 180): the normal swing.
        ("Close", Vec3::new(0.0, 0.0, 150.0)),
        // Gap 210 (between 180 and the lunge range 300): the lunge, off to Sora's right.
        ("Lunge", Vec3::new(250.0, 0.0, 0.0)),
        // Far away, ahead on the right.
        ("Far", Vec3::new(350.0, 0.0, 900.0)),
        // On the platform, 200 up: part centre 280 above Sora (> 160): the jump-up attack.
        ("High", Vec3::new(p.x, p.top(), p.z)),
    ]
}

impl Sim {
    pub fn new(kit: Arc<SoraKit>, opts: Options) -> Sim {
        let arena = Arena::default();
        let ctrl = Controller::new(kit.params.clone(), opts.abilities.clone(), kit.moveset.clone(), START);
        let rig = kit.rig.clone();
        let mut s = Sim {
            kit,
            arena,
            attacker: opts.attacker.clone(),
            opts,
            ctrl,
            rig,
            dummies: Vec::new(),
            info: Vec::new(),
            frame: 0,
            last: None,
            lock: None,
            hits: Vec::new(),
            hit_shapes: Vec::new(),
            blade_line: None,
            log: VecDeque::new(),
            posed_entry: None,
        };
        s.reset();
        s
    }

    /// Sora back to the start (abilities kept), dummies back home with full HP.
    pub fn reset(&mut self) {
        let abilities = self.ctrl.abilities.clone();
        self.ctrl = Controller::new(self.kit.params.clone(), abilities, self.kit.moveset.clone(), START);
        self.attacker = self.opts.attacker.clone();
        self.rig = self.kit.rig.clone();
        self.posed_entry = None;
        self.dummies.clear();
        self.info.clear();
        for (i, (name, pos)) in layout(&self.arena).into_iter().enumerate() {
            let mut d = Dummy::standing(i as u32 + 1, pos, self.opts.dummy.clone());
            d.facing = (START - pos).yaw();
            self.dummies.push(d);
            self.info.push(DummyInfo { name, home: pos, last_hit: None, total: 0, respawn: 0 });
        }
        self.frame = 0;
        self.lock = None;
        self.last = None;
        self.hits.clear();
        self.hit_shapes.clear();
        self.blade_line = None;
        self.note("reset".into());
        self.pose_current(&[]);
    }

    fn note(&mut self, s: String) {
        self.log.push_back((self.frame, s));
        while self.log.len() > 10 {
            self.log.pop_front();
        }
    }

    /// A kh1-anim model-space point in the world (see the module docs).
    pub fn to_world(&self, p: [f32; 3]) -> Vec3 {
        let r = if self.ctrl.motion.root_on { self.rig.current().root_travel } else { [0.0; 3] };
        let local = Vec3::new(-(p[0] - r[0]), p[1] - r[1], -(p[2] - r[2]));
        self.ctrl.pos + local.rotate_yaw(self.ctrl.facing)
    }

    /// A bone's world position in the current pose.
    pub fn bone_world(&self, bone: usize) -> Vec3 {
        self.to_world(self.rig.current().world[bone].t)
    }

    /// The parts the controller and the lock see (one per dummy still standing).
    fn parts(&self) -> (Vec<kh1_core::TargetPart>, Option<usize>) {
        let mut parts = Vec::new();
        let mut lock = None;
        for d in self.dummies.iter().filter(|d| d.stats.hp > 0) {
            if Some(d.id) == self.lock {
                lock = Some(parts.len());
            }
            parts.extend(d.target_parts());
        }
        (parts, lock)
    }

    /// The nearest standing dummy to Sora.
    fn nearest(&self) -> Option<u32> {
        self.dummies
            .iter()
            .filter(|d| d.stats.hp > 0)
            .min_by(|a, b| (a.pos - self.ctrl.pos).length().total_cmp(&(b.pos - self.ctrl.pos).length()))
            .map(|d| d.id)
    }

    /// One 60 Hz frame.
    pub fn step(&mut self, pad: &Pad) -> &Output {
        self.step_dt(pad, 1.0)
    }

    /// One frame with KH1's frame step `dt` (`0x14233FBDC`: 1.0 at 60 fps), as a replay feeds it.
    pub fn step_dt(&mut self, pad: &Pad, dt: f32) -> &Output {
        if pad.lock_toggle {
            self.lock = if self.lock.is_some() { None } else { self.nearest() };
            let s = match self.lock {
                Some(id) => format!("lock on: {}", self.info[id as usize - 1].name),
                None => "lock off".into(),
            };
            self.note(s);
        }
        if self.lock.is_some_and(|id| self.dummies.iter().any(|d| d.id == id && d.stats.hp <= 0)) {
            self.lock = None;
        }
        let (parts, lock) = self.parts();
        let (move_dir, tilt) = Input::move_from_stick(pad.stick, pad.camera_yaw);
        let input = Input {
            move_dir,
            tilt,
            jump: Button { pressed: pad.jump.pressed, held: pad.jump.held },
            attack: Button { pressed: pad.attack.pressed, held: pad.attack.held },
            square: Button { pressed: pad.square.pressed, held: pad.square.held },
            lock,
        };
        let bodies = if self.opts.push {
            self.dummies
                .iter()
                .filter(|d| d.stats.hp > 0 && !d.airborne())
                .flat_map(|d| d.shapes())
                .map(|s| Body { x: s.centre.x, z: s.centre.z, radius: s.radius, top: s.centre.y - s.half_height, bottom: s.centre.y + s.half_height })
                .collect()
        } else {
            Vec::new()
        };
        let mut world = ArenaWorld { arena: &self.arena, bodies };
        let out = self.ctrl.step(&input, &parts, &mut world, dt);
        self.frame += 1;
        for e in &out.events {
            if !matches!(e, Event::MotionStarted(_)) {
                self.note(format!("{e:?}"));
            }
        }

        // Pose the skeleton for this frame's motion and clock.
        self.pose_current(&out.events);

        // The Keyblade: its line, and the live attack's blade points A / B.
        let kit = self.kit.clone();
        let bone = self.rig.attach_bone(WEAPON_ATTACH);
        self.blade_line = bone.and_then(|_| {
            let grip = self.rig.attach_point(WEAPON_ATTACH, 0.0, Some(&kit.weapon_line))?;
            let tip = self.rig.attach_point(WEAPON_ATTACH, 1.0, Some(&kit.weapon_line))?;
            Some((self.to_world(grip), self.to_world(tip)))
        });
        self.hit_shapes.clear();
        self.hits.clear();
        if let (Some(bone), Some((live, ta, tb))) = (bone, blade_params(&self.ctrl, &kit.weapon, &self.attacker, 1.0)) {
            let a = self.to_world(self.rig.point_on_bone(bone, ta));
            let b = self.to_world(self.rig.point_on_bone(bone, tb));
            // Assumption: the swept chain's start with no previous frame is the hips (bone 0).
            let centre = self.bone_world(0);
            if self.ctrl.hit_live() {
                if let Some(s) = &self.ctrl.swing {
                    self.hit_shapes = s.volume.shapes(a, b, live.radius, live.data.sweeps(), centre);
                }
            }
            self.hits = resolve_hits(&mut self.ctrl, Blade { a, b }, centre, &kit.weapon, &mut self.attacker, 1.0, &mut self.dummies, &kit.combat);
        }
        let hits = self.hits.clone();
        for h in &hits {
            if let Some(i) = self.dummies.iter().position(|d| d.id == h.dummy) {
                let info = &mut self.info[i];
                info.last_hit = Some((h.damage, h.critical, self.frame));
                info.total += h.damage;
                let name = info.name;
                self.note(format!("hit {name}: {} dmg{} {:?}{}", h.damage, if h.critical { " (crit)" } else { "" }, h.reaction, if h.killed { " - beaten" } else { "" }));
            }
        }

        // The dummies' own frame.
        for (d, info) in self.dummies.iter_mut().zip(self.info.iter_mut()) {
            let floor = self.arena.floor_under(d.pos.x, d.pos.z, d.pos.y);
            d.step(&kit.combat.reaction, floor, dt);
            if d.stats.hp <= 0 && matches!(d.state, DummyState::Idle | DummyState::Down { .. }) {
                info.respawn += 1;
                if info.respawn >= RESPAWN_FRAMES {
                    let id = d.id;
                    let facing = (START - info.home).yaw();
                    *d = Dummy::standing(id, info.home, self.opts.dummy.clone());
                    d.facing = facing;
                    info.respawn = 0;
                }
            }
        }
        self.last = Some(out);
        self.last.as_ref().expect("just set")
    }

    /// Drives the rig from the motion player: a new motion starts the rig's motion change, or a
    /// frozen-snapshot blend when the controller asked for one (weight = blend clock / blend
    /// length, `step / length` on the first frame as KH1's `0x1402A0350`); otherwise the motion
    /// is posed at the controller's motion clock.
    fn pose_current(&mut self, events: &[Event]) {
        let kit = self.kit.clone();
        let mp = &self.ctrl.motion;
        let Some(e) = mp.entry else { return };
        let Some(motion) = kit.motion(e) else { return };
        let started = self.posed_entry != Some(e) || events.iter().any(|ev| matches!(ev, Event::MotionStarted(_)));
        let blending = mp.blend > 0.0;
        let w = if blending { (mp.blend_clock / mp.blend).clamp(0.0, 1.0) } else { 1.0 };
        if started {
            if blending {
                self.rig.blend_start(motion, mp.frame, w);
            } else {
                self.rig.start_motion(motion);
                self.rig.pose(motion, mp.frame);
            }
        } else if blending && self.rig.is_blending() {
            self.rig.blend(w);
        } else {
            self.rig.pose(motion, mp.frame);
        }
        self.posed_entry = Some(e);
    }

    /// MP from the charge gauge (30 units = 1 MP) and its cap.
    pub fn mp(&self) -> (f32, i32) {
        (self.attacker.mp_gauge as f32 / self.kit.combat.mp.units_per_mp as f32, self.attacker.max_mp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::Script;

    fn kit() -> Option<Arc<SoraKit>> {
        let dir = std::path::PathBuf::from(std::env::var_os("KH1_TEST_DATA")?);
        let read = |n: &str| std::fs::read(dir.join(n)).ok();
        let (m, d, b) = (read(kh1_data::MSET)?, read(kh1_data::MDLS)?, read(kh1_data::BTLTBL)?);
        Some(Arc::new(SoraKit::from_files(&m, &d, b).unwrap()))
    }

    /// The scripted combo near the close dummy: rows 13 → 14 → 8, three hits with damage.
    #[test]
    fn scripted_combo_hits_the_close_dummy() {
        let Some(kit) = kit() else { return };
        let mut sim = Sim::new(kit, Options::default());
        let mut script = Script::new("combo").unwrap();
        let mut rows = Vec::new();
        let mut hits = Vec::new();
        for _ in 0..300 {
            let pad = script.pad(&sim);
            let out = sim.step(&pad).clone();
            rows.extend(out.events.iter().filter_map(|e| if let Event::AttackStarted { row, .. } = e { Some(*row) } else { None }));
            hits.extend(sim.hits.iter().map(|h| (h.dummy, h.damage)));
        }
        assert_eq!(rows, vec![13, 14, 8]);
        assert_eq!(hits.len(), 3, "{hits:?}");
        assert!(hits.iter().all(|h| h.0 == 1 && h.1 > 0));
        assert!(sim.dummies[0].stats.hp < 300);
        assert!(sim.attacker.mp_gauge > 0);
    }

    /// Next to the platform, locked on to the dummy standing on it: the jump-up attack (row 1).
    #[test]
    fn jump_up_attack_at_the_high_dummy() {
        let Some(kit) = kit() else { return };
        let mut sim = Sim::new(kit, Options::default());
        let p = sim.arena.platform;
        sim.ctrl.pos = Vec3::new(p.x + p.half_x + 60.0, 0.0, p.z);
        sim.ctrl.facing = -std::f32::consts::FRAC_PI_2;
        sim.ctrl.target_yaw = sim.ctrl.facing;
        sim.lock = Some(4);
        let out = sim.step(&Pad { attack: Key { held: true, pressed: true }, ..Default::default() }).clone();
        let rows: Vec<usize> = out.events.iter().filter_map(|e| if let Event::AttackStarted { row, .. } = e { Some(*row) } else { None }).collect();
        assert_eq!(rows, vec![1], "{:?}", out.events);
    }

    /// The posed swing moves with the controller: the hips stay near Sora's position while the
    /// root travel carries him (it is not applied twice).
    #[test]
    fn root_travel_is_not_doubled() {
        let Some(kit) = kit() else { return };
        let mut sim = Sim::new(kit, Options { push: false, ..Default::default() });
        let press = Pad { attack: Key { held: true, pressed: true }, ..Default::default() };
        sim.step(&press);
        let mut max_off: f32 = 0.0;
        for _ in 0..47 {
            sim.step(&Pad::default());
            let hips = sim.bone_world(0);
            max_off = max_off.max((hips - sim.ctrl.pos).flat().length());
        }
        assert!((sim.ctrl.pos.z - 100.0).abs() < 0.5, "swing 1 travels 100: {}", sim.ctrl.pos.z);
        assert!(max_off < 40.0, "hips stay over the feet: {max_off}");
        // The Keyblade is in front of Sora at the hit frames.
        let (grip, tip) = sim.blade_line.unwrap();
        assert!(grip.y < 0.0 && tip.y <= 10.0);
    }
}
