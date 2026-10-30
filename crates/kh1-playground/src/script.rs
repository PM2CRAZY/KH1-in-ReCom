//! `--script <name>`: scripted input for headless checks and screenshots.
//!
//! - `combo`: lock on to the nearest dummy, then press Attack at once and again whenever the
//!   combo window is open, until three swings have started (the 3-hit ground combo).
//! - `air`: jump, then the air combo the same way.
//! - `run`: run forward for a second, then stop.
//! - `roll`: a Dodge Roll forward.
//! - `idle`: nothing.

use kh1_core::Event;

use crate::sim::{Key, Pad, Sim};

pub struct Script {
    kind: Kind,
    started: u32,
    attack_held: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Combo,
    Air,
    Run,
    Roll,
    Idle,
}

impl Script {
    pub fn new(name: &str) -> Option<Script> {
        let kind = match name {
            "combo" => Kind::Combo,
            "air" => Kind::Air,
            "run" => Kind::Run,
            "roll" => Kind::Roll,
            "idle" => Kind::Idle,
            _ => return None,
        };
        Some(Script { kind, started: 0, attack_held: false })
    }

    pub const NAMES: &'static str = "combo, air, run, roll, idle";

    /// The pad for the frame `sim` is about to simulate.
    pub fn pad(&mut self, sim: &Sim) -> Pad {
        if let Some(o) = &sim.last {
            self.started += o.events.iter().filter(|e| matches!(e, Event::AttackStarted { .. })).count() as u32;
        }
        let f = sim.frame;
        let mut pad = Pad::default();
        let mut attack = false;
        match self.kind {
            Kind::Combo => {
                pad.lock_toggle = f == 0;
                let can = sim.ctrl.accepts_attack() && (f == 2 || sim.ctrl.in_window());
                attack = self.started < 3 && f >= 2 && can;
            }
            Kind::Air => {
                pad.jump = Key { held: f < 20, pressed: f == 0 };
                let can = sim.ctrl.accepts_attack() && (f == 12 || sim.ctrl.in_window());
                attack = self.started < 3 && f >= 12 && can;
            }
            Kind::Run => {
                if f < 60 {
                    pad.stick = [0.0, 1.0];
                }
            }
            Kind::Roll => {
                pad.stick = if f < 30 { [0.0, 1.0] } else { [0.0; 2] };
                pad.square = Key { held: f == 10, pressed: f == 10 };
            }
            Kind::Idle => {}
        }
        // A fresh press each time (release for a frame in between).
        let press = attack && !self.attack_held;
        pad.attack = Key { held: press, pressed: press };
        self.attack_held = press;
        pad
    }
}
