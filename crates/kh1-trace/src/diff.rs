//! Comparing two traces frame by frame (a KH1 trace and a playground trace).
//!
//! Alignment: each trace starts at its first frame with input (stick tilted or a button held /
//! pressed), so idle frames before the first input do not matter. Positions are compared
//! relative to Sora's position on the frame before that, turned into his facing then (his start
//! frame of reference); facings relative to that facing.

use crate::schema::Row;

/// The first row with input: tilt, or any button bit.
pub fn first_input(rows: &[Row]) -> Option<usize> {
    rows.iter().position(|r| r.in_tilt > 0.0 || r.in_jump != 0 || r.in_attack != 0 || r.in_square != 0)
}

/// Tolerances.
#[derive(Clone, Copy, Debug)]
pub struct Tolerance {
    /// Position (units, distance).
    pub pos: f32,
    /// Facing (radians).
    pub angle: f32,
    /// Motion frame.
    pub frame: f32,
}

impl Default for Tolerance {
    fn default() -> Self {
        Tolerance { pos: 0.01, angle: 1e-3, frame: 1e-3 }
    }
}

/// How the traces are lined up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    /// By the first input in each.
    FirstInput,
    /// Row 0 with row 0.
    Start,
}

/// The compared fields.
pub const FIELDS: &[&str] = &["state", "request", "entry", "motion_frame", "pos", "facing", "combo_counter", "row"];

/// One field's result.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FieldStats {
    pub name: &'static str,
    /// Frames where it differs beyond the tolerance.
    pub mismatches: usize,
    /// The first such frame (offset from the aligned start).
    pub first: Option<usize>,
    /// The largest difference (numbers: absolute; ids: 1 if any differ).
    pub max_error: f32,
}

/// A difference on one frame.
#[derive(Clone, Debug, PartialEq)]
pub struct Divergence {
    /// Offset from the aligned start, and each trace's own frame column.
    pub k: usize,
    pub frame_a: u64,
    pub frame_b: u64,
    pub field: &'static str,
    pub a: String,
    pub b: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    /// The aligned start rows.
    pub start_a: usize,
    pub start_b: usize,
    pub compared: usize,
    pub fields: Vec<FieldStats>,
    pub first: Option<Divergence>,
    /// Frames whose inputs differ (stick, buttons, lock): the comparison past them is not
    /// like for like. (offset, what).
    pub input_mismatch: Option<(usize, String)>,
    /// Trace A frames where KH1 froze Sora (actor +0x88 bit 0).
    pub freeze_frames: usize,
}

impl Report {
    pub fn matches(&self) -> bool {
        self.first.is_none()
    }
}

fn wrap(a: f32) -> f32 {
    let t = std::f32::consts::TAU;
    let mut x = (a + std::f32::consts::PI).rem_euclid(t) - std::f32::consts::PI;
    if x < -std::f32::consts::PI {
        x += t;
    }
    x
}

/// Position relative to `origin`, in the frame of `yaw` (x right, z forward of that facing).
pub fn relative(r: &Row, origin: &Row) -> [f32; 3] {
    let d = [r.pos_x - origin.pos_x, r.pos_y - origin.pos_y, r.pos_z - origin.pos_z];
    let yaw = if origin.facing.is_finite() { origin.facing } else { 0.0 };
    // Yaw 0 faces +z; turn the offset by −yaw about y.
    let (s, c) = yaw.sin_cos();
    [d[0] * c - d[2] * s, d[1], d[0] * s + d[2] * c]
}

fn origin_index(start: usize) -> usize {
    start.saturating_sub(1)
}

/// Compares `a` (usually KH1) with `b` (usually the playground).
pub fn compare(a: &[Row], b: &[Row], align: Align, tol: Tolerance, max_frames: Option<usize>) -> Report {
    let (sa, sb) = match align {
        Align::Start => (0, 0),
        Align::FirstInput => (first_input(a).unwrap_or(0), first_input(b).unwrap_or(0)),
    };
    let mut fields: Vec<FieldStats> = FIELDS.iter().map(|n| FieldStats { name: n, ..Default::default() }).collect();
    let mut first: Option<Divergence> = None;
    let mut input_mismatch = None;
    let n = (a.len().saturating_sub(sa)).min(b.len().saturating_sub(sb));
    let n = max_frames.map_or(n, |m| n.min(m));
    let (oa, ob) = if a.is_empty() || b.is_empty() { (Row::default(), Row::default()) } else { (a[origin_index(sa)].clone(), b[origin_index(sb)].clone()) };
    let freeze_frames = a.iter().filter(|r| r.freeze).count();
    for k in 0..n {
        let (ra, rb) = (&a[sa + k], &b[sb + k]);
        if input_mismatch.is_none() {
            let stick = (ra.in_stick_x - rb.in_stick_x).abs().max((ra.in_stick_y - rb.in_stick_y).abs());
            let what = if stick > 1e-4 {
                Some(format!("stick ({}, {}) vs ({}, {})", ra.in_stick_x, ra.in_stick_y, rb.in_stick_x, rb.in_stick_y))
            } else if (ra.in_jump, ra.in_attack, ra.in_square) != (rb.in_jump, rb.in_attack, rb.in_square) {
                Some(format!("buttons jump/attack/square {}/{}/{} vs {}/{}/{}", ra.in_jump, ra.in_attack, ra.in_square, rb.in_jump, rb.in_attack, rb.in_square))
            } else if ra.in_lock != rb.in_lock {
                Some(format!("lock {} vs {}", ra.in_lock as u8, rb.in_lock as u8))
            } else {
                None
            };
            if let Some(w) = what {
                input_mismatch = Some((k, w));
            }
        }
        let (pa, pb) = (relative(ra, &oa), relative(rb, &ob));
        let pos_err = ((pa[0] - pb[0]).powi(2) + (pa[1] - pb[1]).powi(2) + (pa[2] - pb[2]).powi(2)).sqrt();
        let face_err = wrap((ra.facing - oa.facing) - (rb.facing - ob.facing)).abs();
        let frame_err = (ra.motion_frame - rb.motion_frame).abs();
        let id = |x: i32, y: i32| if x == y { 0.0 } else { 1.0 };
        let row_err = if ra.row >= 0 && rb.row >= 0 { id(ra.row, rb.row) } else { 0.0 };
        let errs: [(f32, f32, String, String); 8] = [
            (id(ra.state, rb.state), 0.5, ra.state.to_string(), rb.state.to_string()),
            (id(ra.request & 0xFFFF, rb.request & 0xFFFF), 0.5, format!("{:#x}", ra.request & 0xFFFF), format!("{:#x}", rb.request & 0xFFFF)),
            (id(ra.entry, rb.entry), 0.5, ra.entry.to_string(), rb.entry.to_string()),
            (nan_err(frame_err), tol.frame, ra.motion_frame.to_string(), rb.motion_frame.to_string()),
            (nan_err(pos_err), tol.pos, format!("({:.3}, {:.3}, {:.3})", pa[0], pa[1], pa[2]), format!("({:.3}, {:.3}, {:.3})", pb[0], pb[1], pb[2])),
            (nan_err(face_err), tol.angle, format!("{:.5}", wrap(ra.facing - oa.facing)), format!("{:.5}", wrap(rb.facing - ob.facing))),
            (id(ra.combo_counter, rb.combo_counter), 0.5, ra.combo_counter.to_string(), rb.combo_counter.to_string()),
            (row_err, 0.5, ra.row.to_string(), rb.row.to_string()),
        ];
        for (f, (err, t, va, vb)) in fields.iter_mut().zip(errs) {
            if err > f.max_error {
                f.max_error = err;
            }
            if err > t {
                f.mismatches += 1;
                f.first.get_or_insert(k);
                if first.is_none() {
                    first = Some(Divergence { k, frame_a: ra.frame, frame_b: rb.frame, field: f.name, a: va, b: vb });
                }
            }
        }
    }
    Report { start_a: sa, start_b: sb, compared: n, fields, first, input_mismatch, freeze_frames }
}

/// A NaN difference (a value missing on one side) counts as a mismatch.
fn nan_err(e: f32) -> f32 {
    if e.is_nan() {
        f32::INFINITY
    } else {
        e
    }
}

/// How well the camera formula ([`crate::schema::kh1_camera_to_core`]) reproduces KH1's own move
/// vector, over frames where Sora could act at the start and the stick was tilted:
/// (frames checked, frames within `tol` radians, max error).
pub fn camera_check(rows: &[Row], tol: f32) -> (usize, usize, f32) {
    let (mut n, mut ok, mut max) = (0, 0, 0.0f32);
    for r in rows {
        if !r.in_can_act || r.in_tilt <= 0.05 || !r.in_move_x.is_finite() || r.in_move_w.is_nan() || r.in_move_w <= 0.0 {
            continue;
        }
        let Some(m) = crate::schema::move_from_stick([r.in_stick_x, r.in_stick_y], r.in_cam_yaw) else { continue };
        let e = wrap(m[0].atan2(m[1]) - r.in_move_x.atan2(r.in_move_z)).abs();
        n += 1;
        if e <= tol {
            ok += 1;
        }
        max = max.max(e);
    }
    (n, ok, max)
}

/// The report as text.
pub fn format(r: &Report, a_name: &str, b_name: &str) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    let _ = writeln!(s, "aligned: {a_name} row {} with {b_name} row {}; {} frames compared", r.start_a, r.start_b, r.compared);
    if r.freeze_frames > 0 {
        let _ = writeln!(s, "note: {a_name} has {} frozen frames (actor +0x88 bit 0); kh1-core does not model them", r.freeze_frames);
    }
    if let Some((k, w)) = &r.input_mismatch {
        let _ = writeln!(s, "WARNING: inputs differ from frame +{k}: {w}");
    }
    match &r.first {
        None => {
            let _ = writeln!(s, "MATCH: every compared field is within tolerance");
        }
        Some(d) => {
            let _ = writeln!(s, "FIRST DIVERGENCE at frame +{} ({a_name} frame {}, {b_name} frame {}): {} = {} vs {}", d.k, d.frame_a, d.frame_b, d.field, d.a, d.b);
        }
    }
    let _ = writeln!(s, "{:<14} {:>10} {:>8} {:>12}", "field", "mismatches", "first", "max error");
    for f in &r.fields {
        let first = f.first.map_or("-".to_string(), |k| format!("+{k}"));
        let _ = writeln!(s, "{:<14} {:>10} {:>8} {:>12.5}", f.name, f.mismatches, first, f.max_error);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idle(frame: u64, x: f32) -> Row {
        Row { frame, pos_x: x, pos_y: 0.0, pos_z: 0.0, facing: 0.0, motion_frame: frame as f32, entry: 0, ..Default::default() }
    }

    /// The same motion recorded with different idle lead-ins, different start positions and a
    /// turned start facing lines up exactly.
    #[test]
    fn aligns_by_first_input_in_the_start_frame() {
        let walk = |lead: u64, origin: [f32; 2], yaw: f32| -> Vec<Row> {
            let mut v: Vec<Row> = (0..lead).map(|f| Row { pos_x: origin[0], pos_z: origin[1], facing: yaw, ..idle(f, 0.0) }).collect();
            for k in 0..10u64 {
                // 8 units a frame straight ahead of the start facing.
                let d = 8.0 * (k + 1) as f32;
                v.push(Row {
                    frame: lead + k,
                    pos_x: origin[0] + d * yaw.sin(),
                    pos_y: 0.0,
                    pos_z: origin[1] + d * yaw.cos(),
                    facing: yaw,
                    in_tilt: 1.0,
                    in_stick_y: 1.0,
                    motion_frame: k as f32,
                    entry: 5,
                    request: 2,
                    ..Default::default()
                });
            }
            v
        };
        let a = walk(7, [1000.0, -300.0], 1.2);
        let b = walk(2, [0.0, 0.0], -0.4);
        let r = compare(&a, &b, Align::FirstInput, Tolerance::default(), None);
        assert_eq!((r.start_a, r.start_b, r.compared), (7, 2, 10));
        assert!(r.matches(), "{}", format(&r, "a", "b"));
        assert!(r.input_mismatch.is_none());
        // Aligned from row 0 instead, they do not match.
        assert!(!compare(&a, &b, Align::Start, Tolerance::default(), None).matches());
    }

    #[test]
    fn reports_the_first_divergence() {
        let a: Vec<Row> = (0..20).map(|f| Row { in_tilt: 1.0, in_stick_y: 1.0, state: 0, ..idle(f, f as f32) }).collect();
        let mut b = a.clone();
        b[12].state = 2;
        b[15].pos_x += 0.5;
        let r = compare(&a, &b, Align::FirstInput, Tolerance::default(), None);
        let d = r.first.clone().unwrap();
        assert_eq!((d.k, d.field, d.a.as_str(), d.b.as_str()), (12, "state", "0", "2"));
        let pos = r.fields.iter().find(|f| f.name == "pos").unwrap();
        assert_eq!((pos.mismatches, pos.first), (1, Some(15)));
        assert!((pos.max_error - 0.5).abs() < 1e-5);
        // Different inputs are flagged.
        b[3].in_jump = 2;
        let r = compare(&a, &b, Align::FirstInput, Tolerance::default(), None);
        assert_eq!(r.input_mismatch.as_ref().map(|m| m.0), Some(3));
    }

    #[test]
    fn camera_check_counts() {
        let yaw = 0.8f32;
        let m = crate::schema::move_from_stick([0.3, 0.9], yaw).unwrap();
        let good = Row { in_can_act: true, in_tilt: 0.95, in_stick_x: 0.3, in_stick_y: 0.9, in_cam_yaw: yaw, in_move_x: m[0], in_move_z: m[1], in_move_w: 0.95, ..Default::default() };
        let bad = Row { in_move_x: -m[0], ..good.clone() };
        let (n, ok, max) = camera_check(&[good, bad], 1e-4);
        assert_eq!((n, ok), (2, 1));
        assert!(max > 0.1);
    }
}
