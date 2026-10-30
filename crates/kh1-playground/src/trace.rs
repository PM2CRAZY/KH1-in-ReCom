//! `--trace out.csv`: one row per 60 Hz frame, for frame-by-frame comparison with KH1 traces.
//! Units are kh1-core's (KH1 units, y down, radians).

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use kh1_core::{Event, Output};

use crate::sim::Sim;

pub const HEADER: &str = "frame,state,request,entry,motion_frame,pos_x,pos_y,pos_z,disp_x,disp_y,disp_z,air_vx,air_vy,air_vz,facing,combo_counter,row,action_lock,hit_active,combo_window,grounded,mp_gauge,events,hits";

pub struct Trace {
    out: BufWriter<File>,
}

impl Trace {
    pub fn create(path: &Path) -> std::io::Result<Trace> {
        let mut out = BufWriter::new(File::create(path)?);
        writeln!(out, "{HEADER}")?;
        Ok(Trace { out })
    }

    /// Writes the frame `sim` just simulated.
    pub fn frame(&mut self, sim: &Sim, o: &Output) -> std::io::Result<()> {
        writeln!(self.out, "{}", row(sim, o))
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        self.out.flush()
    }
}

fn event_text(e: &Event) -> String {
    match e {
        Event::AttackStarted { row, request, counter, finisher } => format!("attack(row={row} req={request} n={counter}{})", if *finisher { " fin" } else { "" }),
        Event::AttackRefused(r) => format!("refused({r:?})"),
        Event::MotionStarted(r) => format!("motion({r})"),
        other => format!("{other:?}").to_lowercase(),
    }
}

/// One CSV row (no newline).
pub fn row(sim: &Sim, o: &Output) -> String {
    let events: Vec<String> = o.events.iter().map(event_text).collect();
    let hits: Vec<String> = sim.hits.iter().map(|h| format!("d{}:{}{}:{:?}", h.dummy, h.damage, if h.critical { "c" } else { "" }, h.reaction).replace(',', ";")).collect();
    format!(
        "{},{},{},{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.5},{},{},{},{},{},{},{},\"{}\",\"{}\"",
        sim.frame,
        o.state,
        o.request,
        o.entry.map_or(-1, |e| e as i64),
        o.motion_frame,
        o.pos.x,
        o.pos.y,
        o.pos.z,
        o.displacement.x,
        o.displacement.y,
        o.displacement.z,
        o.air_velocity.x,
        o.air_velocity.y,
        o.air_velocity.z,
        o.facing,
        o.combo_counter,
        o.row.map_or(-1, |r| r as i64),
        o.action_lock as u8,
        o.hit_active as u8,
        o.combo_window as u8,
        o.grounded as u8,
        sim.attacker.mp_gauge,
        events.join(" "),
        hits.join(" ")
    )
}
