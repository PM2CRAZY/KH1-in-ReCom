//! `--trace out.csv`: one row per 60 Hz frame in the shared trace format (`kh1_trace::schema`,
//! the same columns the KH1 logger writes), for frame-by-frame comparison with KH1 traces
//! (`kh1-tracediff`). Units are kh1-core's (KH1 units, y down, radians). Row n holds frame n's
//! inputs and the state after it.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use kh1_core::{Abilities, Controller, Event, Output, Refused};
use kh1_trace::schema::{self, Hex, Row};

use crate::sim::{Pad, Sim};

pub struct Trace {
    out: BufWriter<File>,
}

impl Trace {
    pub fn create(path: &Path) -> std::io::Result<Trace> {
        let mut out = BufWriter::new(File::create(path)?);
        schema::write_header(&mut out)?;
        Ok(Trace { out })
    }

    /// Writes the frame `sim` just simulated from `pad` with frame step `step`.
    pub fn frame(&mut self, sim: &Sim, o: &Output, pad: &Pad, step: f32) -> std::io::Result<()> {
        row(sim, o, pad, step).write_csv(&mut self.out)
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        self.out.flush()
    }
}

/// Writes rows to a file.
pub fn write_rows(path: &Path, rows: &[Row]) -> std::io::Result<()> {
    let mut out = BufWriter::new(File::create(path)?);
    schema::write_header(&mut out)?;
    for r in rows {
        r.write_csv(&mut out)?;
    }
    out.flush()
}

fn event_text(e: &Event) -> String {
    match e {
        Event::AttackStarted { row, request, counter, finisher } => format!("attack(row={row} req={request} n={counter}{})", if *finisher { " fin" } else { "" }),
        Event::AttackRefused(r) => format!("refused({r:?})"),
        Event::MotionStarted(r) => format!("motion({r})"),
        other => format!("{other:?}").to_lowercase(),
    }
}

/// kh1-core's abilities as KH1's ability bits (`0x142D5EC10`, `0x1402A6380`).
pub fn ability_bits(a: &Abilities) -> u32 {
    [
        (a.vortex, 0x1),
        (a.aerial_sweep, 0x2),
        (a.blitz, 0x8),
        (a.guard, 0x10),
        (a.dodge_roll, 0x20),
        (a.glide, 0x80),
        (a.superglide, 0x100),
        (a.high_jump, 0x200),
        (a.sliding_dash, 0x100_0000),
        (a.hurricane_blast, 0x200_0000),
        (a.ripple_drive, 0x400_0000),
        (a.stun_impact, 0x800_0000),
        (a.gravity_break, 0x1000_0000),
        (a.zantetsuken, 0x2000_0000),
    ]
    .into_iter()
    .filter(|(on, _)| *on)
    .fold(0, |m, (_, b)| m | b)
}

/// The columns kh1-core alone fills: the controller's output and state, and the pad.
pub fn core_row(frame: u64, ctrl: &Controller, o: &Output, pad: &Pad, step: f32) -> Row {
    let key = |k: crate::sim::Key, bit: u32| ((k.held as u32) * bit, (k.pressed as u32) * bit);
    let (jh, jp) = key(pad.jump, schema::BTN_JUMP);
    let (ah, ap) = key(pad.attack, schema::BTN_COMMAND);
    let (sh, sp) = key(pad.square, schema::BTN_SQUARE);
    let (held, pressed) = (jh | ah | sh, jp | ap | sp);
    // The command word as KH1 builds it before Sora's think: jump / Square presses, and Attack
    // when the command step takes it (`0x140293480`'s gate, kh1-core's `accepts_attack`).
    let refused = o.events.iter().any(|e| matches!(e, Event::AttackRefused(Refused::NotAccepted)));
    let bit = |on: bool, b: u32| if on { b } else { 0 };
    let command = bit(jp != 0, schema::CMD_JUMP) | bit(sp != 0, schema::CMD_SQUARE) | bit(ap != 0 && !refused, schema::CMD_ATTACK);
    let a = &ctrl.abilities;
    Row {
        frame,
        step,
        freeze: false,
        state: o.state as i32,
        prev_state: ctrl.prev_state as i32,
        request: o.request as i32,
        entry: o.entry.map_or(-1, |e| e as i32),
        motion_frame: o.motion_frame,
        blend: ctrl.motion.blend,
        blend_clock: ctrl.motion.blend_clock,
        motion_flags: Hex(o.motion_flags),
        flags: Hex(bit(o.action_lock, 4) | bit(o.flinch_immune, 0x80)),
        action_lock: o.action_lock,
        hit_active: o.hit_active,
        combo_window: o.combo_window,
        grounded: o.grounded,
        height: ctrl.height_above_floor,
        pos_x: o.pos.x,
        pos_y: o.pos.y,
        pos_z: o.pos.z,
        disp_x: o.displacement.x,
        disp_y: o.displacement.y,
        disp_z: o.displacement.z,
        move_x: ctrl.move_dir.x,
        move_z: ctrl.move_dir.z,
        speed: ctrl.speed,
        air_vx: o.air_velocity.x,
        air_vy: o.air_velocity.y,
        air_vz: o.air_velocity.z,
        facing: o.facing,
        target_yaw: o.target_yaw,
        air_t: ctrl.arc.t,
        rise_t: ctrl.arc.rise_time,
        no_flinch: ctrl.no_flinch,
        combo_counter: o.combo_counter as i32,
        row: o.row.map_or(-1, |r| r as i32),
        cb_flags: Hex(0),
        swing_hits: Hex(ctrl.combo.hits),
        mp_gauge: -1,
        in_stick_x: pad.stick[0],
        in_stick_y: pad.stick[1],
        in_tilt: (pad.stick[0] * pad.stick[0] + pad.stick[1] * pad.stick[1]).sqrt().min(1.0),
        in_raw_x: -1,
        in_raw_y: -1,
        in_cam_yaw: pad.camera_yaw,
        in_cam_raw: f32::NAN,
        in_cam_raw2: f32::NAN,
        in_cam_mode: -1,
        in_move_x: f32::NAN,
        in_move_z: f32::NAN,
        in_move_w: f32::NAN,
        in_can_act: false,
        in_held: Hex(held),
        in_pressed: Hex(pressed),
        in_command: Hex(command),
        in_jump: schema::button(held, pressed, schema::BTN_JUMP),
        in_attack: schema::button(held, pressed, schema::BTN_COMMAND),
        in_square: schema::button(held, pressed, schema::BTN_SQUARE),
        in_lock: false,
        lock_state: 0,
        target_x: f32::NAN,
        target_y: f32::NAN,
        target_z: f32::NAN,
        target_part: -1,
        abilities: Hex(ability_bits(a)),
        combo_max: 3 + a.combo_plus as i32,
        air_combo_max: 3 + a.air_combo_plus as i32,
        combo_master: a.combo_master,
        events: o.events.iter().map(event_text).collect::<Vec<_>>().join(" "),
        hits: String::new(),
    }
}

/// The row for the frame `sim` just simulated (frame number = frames simulated − 1).
pub fn row(sim: &Sim, o: &Output, pad: &Pad, step: f32) -> Row {
    let mut r = core_row(sim.frame.saturating_sub(1), &sim.ctrl, o, pad, step);
    r.mp_gauge = sim.attacker.mp_gauge;
    r.hits = sim.hits.iter().map(|h| format!("d{}:{}{}:{:?}", h.dummy, h.damage, if h.critical { "c" } else { "" }, h.reaction).replace(',', ";")).collect::<Vec<_>>().join(" ");
    if let Some(d) = sim.lock.and_then(|id| sim.dummies.iter().find(|d| d.id == id)) {
        r.in_lock = true;
        r.lock_state = 1;
        r.target_x = d.pos.x;
        r.target_y = d.pos.y;
        r.target_z = d.pos.z;
        r.target_part = 0;
    }
    r
}
