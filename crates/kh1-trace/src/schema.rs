//! The trace CSV: one row per 60 Hz frame, written by the KH1 logger DLL and by
//! `kh1-playground --trace`, read by `kh1-tracediff` and `kh1-playground --replay`.
//!
//! Row `n` holds frame `n`'s **inputs** (what the frame read) and the **state after** it.
//! Units are KH1's and kh1-core's: about 1 cm, y down, yaw in radians (0 faces +z), 60 Hz frames.
//! Floats are written in Rust's shortest round-trip form, so a file read back gives the exact
//! values; an empty float cell is "not available" (NaN). Flags are hex (`0x…`).
//!
//! Where each KH1 column comes from: `docs/kh1-trace.md` (addresses and tags). Columns that one
//! side cannot fill are written empty / -1 / 0 as noted.

use std::collections::HashSet;
use std::io::{self, Write};

/// A 32-bit flag word, written as hex.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Hex(pub u32);

/// One cell's text form.
pub trait Cell: Sized {
    fn put<W: Write>(&self, w: &mut W) -> io::Result<()>;
    fn get(s: &str) -> Result<Self, String>;
}

impl Cell for f32 {
    fn put<W: Write>(&self, w: &mut W) -> io::Result<()> {
        if self.is_nan() {
            Ok(())
        } else {
            write!(w, "{self}")
        }
    }
    fn get(s: &str) -> Result<Self, String> {
        if s.is_empty() {
            Ok(f32::NAN)
        } else {
            s.parse().map_err(|_| format!("not a number: {s:?}"))
        }
    }
}

impl Cell for i32 {
    fn put<W: Write>(&self, w: &mut W) -> io::Result<()> {
        write!(w, "{self}")
    }
    fn get(s: &str) -> Result<Self, String> {
        if s.is_empty() {
            return Ok(-1);
        }
        s.parse().map_err(|_| format!("not an integer: {s:?}"))
    }
}

impl Cell for u64 {
    fn put<W: Write>(&self, w: &mut W) -> io::Result<()> {
        write!(w, "{self}")
    }
    fn get(s: &str) -> Result<Self, String> {
        s.parse().map_err(|_| format!("not a frame number: {s:?}"))
    }
}

impl Cell for bool {
    fn put<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(if *self { b"1" } else { b"0" })
    }
    fn get(s: &str) -> Result<Self, String> {
        match s {
            "1" | "true" => Ok(true),
            "0" | "false" | "" => Ok(false),
            _ => Err(format!("not 0/1: {s:?}")),
        }
    }
}

impl Cell for Hex {
    fn put<W: Write>(&self, w: &mut W) -> io::Result<()> {
        write!(w, "{:#x}", self.0)
    }
    fn get(s: &str) -> Result<Self, String> {
        let v = match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
            Some(h) => u32::from_str_radix(h, 16),
            None if s.is_empty() => Ok(0),
            None => s.parse(),
        };
        v.map(Hex).map_err(|_| format!("not a flag word: {s:?}"))
    }
}

impl Cell for String {
    fn put<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(b"\"")?;
        w.write_all(self.replace(['"', '\n', '\r'], "'").as_bytes())?;
        w.write_all(b"\"")
    }
    fn get(s: &str) -> Result<Self, String> {
        Ok(s.to_string())
    }
}

macro_rules! schema {
    ($( $(#[doc = $doc:literal])* $name:ident : $ty:ty ),* $(,)?) => {
        /// One frame. See the column docs; `docs/kh1-trace.md` lists the KH1 address of each.
        #[derive(Clone, Debug, PartialEq)]
        pub struct Row {
            $( $(#[doc = $doc])* pub $name: $ty, )*
        }

        /// The column names, in file order.
        pub const COLUMNS: &[&str] = &[$(stringify!($name)),*];

        impl Row {
            /// Writes the row and a newline.
            pub fn write_csv<W: Write>(&self, w: &mut W) -> io::Result<()> {
                let mut first = true;
                $(
                    if !first {
                        w.write_all(b",")?;
                    }
                    first = false;
                    Cell::put(&self.$name, w)?;
                )*
                let _ = first;
                w.write_all(b"\n")
            }

            /// Sets the column called `name` from its text. `Ok(false)`: not a column of ours.
            fn set(&mut self, name: &str, text: &str) -> Result<bool, String> {
                match name {
                    $( stringify!($name) => { self.$name = Cell::get(text).map_err(|e| format!("{}: {e}", stringify!($name)))?; Ok(true) } )*
                    _ => Ok(false),
                }
            }
        }
    };
}

schema! {
    /// Row index since the recording started (0 = first frame).
    frame: u64,
    /// The frame step (KH1 `0x14233FBDC`: 1.0 at 60 fps, 2.0 at 30).
    step: f32,
    /// KH1 actor +0x88 bit 0: Sora frozen (his think is skipped, motion step 0). Playground: 0.
    freeze: bool,
    /// Movement state (+0x70) and the previous one (+0x378).
    state: i32,
    prev_state: i32,
    /// Motion request id (+0x164), entry (+0x168, -1 none), frame (+0x16C), blend length and
    /// clock (+0x170 / +0x174).
    request: i32,
    entry: i32,
    motion_frame: f32,
    blend: f32,
    blend_clock: f32,
    /// The motion's per-frame flags (+0x1CC).
    motion_flags: Hex,
    /// Actor flags (+0x00). Playground: only bits 4 (action lock) and 0x80 (no flinch).
    flags: Hex,
    /// Flag 4; motion flags 0xF0000 (hit active) and 0x400000 (combo window).
    action_lock: bool,
    hit_active: bool,
    combo_window: bool,
    /// Ground contact (+0x2D0 bit 1) and height above the floor (+0x3D4).
    grounded: bool,
    height: f32,
    /// Position (+0x10).
    pos_x: f32,
    pos_y: f32,
    pos_z: f32,
    /// This frame's displacement before collision (playground only; KH1: empty).
    disp_x: f32,
    disp_y: f32,
    disp_z: f32,
    /// Move direction x / z (+0x20 / +0x28) and speed (+0x2C).
    move_x: f32,
    move_z: f32,
    speed: f32,
    /// Air velocity (+0x3C0).
    air_vx: f32,
    air_vy: f32,
    air_vz: f32,
    /// Facing yaw (+0x3C) and target yaw (+0x150).
    facing: f32,
    target_yaw: f32,
    /// Air time t (+0x3B0) and rise time T (+0x3B8).
    air_t: f32,
    rise_t: f32,
    /// No-flinch timer (+0x0C).
    no_flinch: f32,
    /// Combo record (actor +0x14C): counter +0x41, row +0x40 (-1 none), flags +0x42, this swing's
    /// hit bits +0x50.
    combo_counter: i32,
    row: i32,
    cb_flags: Hex,
    swing_hits: Hex,
    /// MP charge gauge units (playground only; KH1: -1).
    mp_gauge: i32,
    /// Stick as kh1-core takes it: x right, y forward, length = tilt (dead zone applied).
    in_stick_x: f32,
    in_stick_y: f32,
    in_tilt: f32,
    /// The raw stick bytes the move stick came from (KH1 only; playground: -1).
    in_raw_x: i32,
    in_raw_y: i32,
    /// The camera yaw in kh1-core's terms (stick forward goes this way; see [`kh1_camera_to_core`]).
    in_cam_yaw: f32,
    /// KH1's raw camera values: yaw `0x14299BB04`, second yaw `0x14299C194`, mode `0x14299BB08`.
    in_cam_raw: f32,
    in_cam_raw2: f32,
    in_cam_mode: i32,
    /// KH1's move vector `0x142D5EC30` (x, z, w = tilt) as this frame's think left it. It is only
    /// rewritten when Sora can act; `in_can_act` is that test at the start of the frame.
    in_move_x: f32,
    in_move_z: f32,
    in_move_w: f32,
    in_can_act: bool,
    /// Logical pad buttons held (`0x14298CA94`) and pressed this frame (`0x14298CA98`).
    in_held: Hex,
    in_pressed: Hex,
    /// The command word Sora's think consumed this frame (`0x142D53A40`; byte 0 = command, 1 =
    /// Attack; 0x1000000 jump, 0x4000000 Square). Playground: built the same way.
    in_command: Hex,
    /// Buttons for the replay: bit 0 held, bit 1 pressed (from `in_held` / `in_pressed`, see
    /// [`BTN_JUMP`] etc.).
    in_jump: i32,
    in_attack: i32,
    in_square: i32,
    /// Locked on (KH1 `0x142DB3510` ≠ 0; playground: its lock).
    in_lock: bool,
    /// KH1's lock state word `0x142DB3510` (playground: 1 locked, 0 not).
    lock_state: i32,
    /// The lock target's position (its actor +0x10) and part (`0x142DB351C`); empty / -1 when
    /// there is none.
    target_x: f32,
    target_y: f32,
    target_z: f32,
    target_part: i32,
    /// Ability bits `0x142D5EC10`.
    abilities: Hex,
    /// Combo lengths (stats +0xD4 ground, +0xD5 air) and Combo Master equipped.
    combo_max: i32,
    air_combo_max: i32,
    combo_master: bool,
    /// Playground events and hits of this frame (KH1: empty).
    events: String,
    hits: String,
}

impl Default for Row {
    fn default() -> Self {
        Row {
            frame: 0,
            step: 1.0,
            freeze: false,
            state: 0,
            prev_state: 0,
            request: 0,
            entry: -1,
            motion_frame: f32::NAN,
            blend: f32::NAN,
            blend_clock: f32::NAN,
            motion_flags: Hex(0),
            flags: Hex(0),
            action_lock: false,
            hit_active: false,
            combo_window: false,
            grounded: false,
            height: f32::NAN,
            pos_x: f32::NAN,
            pos_y: f32::NAN,
            pos_z: f32::NAN,
            disp_x: f32::NAN,
            disp_y: f32::NAN,
            disp_z: f32::NAN,
            move_x: f32::NAN,
            move_z: f32::NAN,
            speed: f32::NAN,
            air_vx: f32::NAN,
            air_vy: f32::NAN,
            air_vz: f32::NAN,
            facing: f32::NAN,
            target_yaw: f32::NAN,
            air_t: f32::NAN,
            rise_t: f32::NAN,
            no_flinch: f32::NAN,
            combo_counter: -1,
            row: -1,
            cb_flags: Hex(0),
            swing_hits: Hex(0),
            mp_gauge: -1,
            in_stick_x: 0.0,
            in_stick_y: 0.0,
            in_tilt: 0.0,
            in_raw_x: -1,
            in_raw_y: -1,
            in_cam_yaw: 0.0,
            in_cam_raw: f32::NAN,
            in_cam_raw2: f32::NAN,
            in_cam_mode: -1,
            in_move_x: f32::NAN,
            in_move_z: f32::NAN,
            in_move_w: f32::NAN,
            in_can_act: false,
            in_held: Hex(0),
            in_pressed: Hex(0),
            in_command: Hex(0),
            in_jump: 0,
            in_attack: 0,
            in_square: 0,
            in_lock: false,
            lock_state: 0,
            target_x: f32::NAN,
            target_y: f32::NAN,
            target_z: f32::NAN,
            target_part: -1,
            abilities: Hex(0),
            combo_max: -1,
            air_combo_max: -1,
            combo_master: false,
            events: String::new(),
            hits: String::new(),
        }
    }
}

/// KH1's logical pad bits as the command dispatcher `0x1402937F0` reads them (Checked
/// (decomp)): jump 4 (sets command bit 0x1000000), Square 0x10 (0x4000000), the command button
/// 0x4000000 (selects the command-menu entry; Attack when the cursor is on Attack).
pub const BTN_JUMP: u32 = 0x4;
pub const BTN_SQUARE: u32 = 0x10;
pub const BTN_COMMAND: u32 = 0x400_0000;

/// Command word bits (`0x142D53A40`): byte 0 = command (1 = Attack), jump, Square.
pub const CMD_ATTACK: u32 = 1;
pub const CMD_JUMP: u32 = 0x100_0000;
pub const CMD_SQUARE: u32 = 0x400_0000;

/// Motion flag bits (`+0x1CC`) as kh1-core reads them.
pub const MF_HIT_ANY: u32 = 0xF_0000;
pub const MF_COMBO_WINDOW: u32 = 0x40_0000;

/// The replay value of one button: bit 0 held, bit 1 pressed.
pub fn button(held: u32, pressed: u32, bit: u32) -> i32 {
    (held & bit != 0) as i32 | (((pressed & bit != 0) as i32) << 1)
}

/// KH1's camera globals → kh1-core's camera yaw. KH1's `0x14028C7E0` turns the pad vector
/// (x, 0, y-down) by −yaw (`0x14299BB04`), negates x, then turns it by the second yaw
/// (`0x14299C194`), with `0x1400E6C80`'s matrix (rows (c,0,−s), (0,1,0), (s,0,c), row-vector
/// product `0x1400E5AD0`; Checked (decomp)). Worked through, that equals kh1-core's
/// `Input::move_from_stick(stick, π + yaw + yaw2)`. Assumption: the base matrix `0x1428672D0`
/// is the identity (it is in .bss; not readable from the file). `kh1-tracediff` checks this
/// against KH1's own move vector.
pub fn kh1_camera_to_core(yaw: f32, yaw2: f32) -> f32 {
    std::f32::consts::PI + yaw + yaw2
}

/// kh1-core's `Input::move_from_stick` direction (x, z) for a stick and camera yaw (the same
/// formula; `kh1-playground` tests that they agree).
pub fn move_from_stick(stick: [f32; 2], camera_yaw: f32) -> Option<[f32; 2]> {
    let tilt = (stick[0] * stick[0] + stick[1] * stick[1]).sqrt();
    if tilt <= 0.0 {
        return None;
    }
    let (s, c) = camera_yaw.sin_cos();
    let x = s * stick[1] + c * stick[0];
    let z = c * stick[1] - s * stick[0];
    let l = (x * x + z * z).sqrt();
    (l > 0.0).then(|| [x / l, z / l])
}

/// The header line (no newline).
pub fn header() -> String {
    COLUMNS.join(",")
}

pub fn write_header<W: Write>(w: &mut W) -> io::Result<()> {
    w.write_all(header().as_bytes())?;
    w.write_all(b"\n")
}

/// A trace read back.
#[derive(Clone, Debug, Default)]
pub struct Trace {
    pub rows: Vec<Row>,
    /// The columns the file had (others are at their defaults).
    pub present: HashSet<String>,
}

impl Trace {
    pub fn has(&self, col: &str) -> bool {
        self.present.contains(col)
    }
}

/// Splits one CSV line (double-quoted cells may hold commas; `""` is a quote).
fn split(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out
}

/// Reads a trace. Unknown columns are ignored and missing ones keep their defaults, so older
/// playground traces (fewer columns) still read.
pub fn parse(text: &str) -> Result<Trace, String> {
    let mut lines = text.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#'));
    let head = lines.next().ok_or("empty trace")?;
    let names: Vec<String> = split(head).into_iter().map(|s| s.trim().to_string()).collect();
    if !names.iter().any(|n| n == "frame") {
        return Err("no `frame` column: not a trace".into());
    }
    let present: HashSet<String> = names.iter().filter(|n| COLUMNS.contains(&n.as_str())).cloned().collect();
    let mut rows = Vec::new();
    for (i, line) in lines.enumerate() {
        let cells = split(line);
        let mut r = Row::default();
        for (n, c) in names.iter().zip(cells.iter()) {
            r.set(n, c.trim()).map_err(|e| format!("row {}: {e}", i + 1))?;
        }
        rows.push(r);
    }
    Ok(Trace { rows, present })
}

pub fn read_file(path: &std::path::Path) -> Result<Trace, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse(&text).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(i: u64) -> Row {
        Row {
            frame: i,
            step: 1.0,
            state: 2,
            prev_state: 1,
            request: 0x10004,
            entry: 12,
            motion_frame: 3.25,
            blend: 8.0,
            blend_clock: 1.0,
            motion_flags: Hex(0x40_0000),
            flags: Hex(0x6_0004),
            action_lock: true,
            combo_window: true,
            height: 37.125,
            pos_x: 1.0 / 3.0,
            pos_y: -190.0,
            pos_z: 1e-7,
            move_x: 0.6,
            move_z: -0.8,
            speed: 8.0,
            air_vx: 4.8,
            air_vy: -6.06,
            air_vz: -6.4,
            facing: -2.5,
            target_yaw: 3.1,
            air_t: 6.0,
            rise_t: 28.0,
            no_flinch: 0.0,
            combo_counter: 2,
            row: 13,
            swing_hits: Hex(1),
            in_stick_x: 0.25,
            in_stick_y: -0.5,
            in_tilt: 0.559_017,
            in_raw_x: 200,
            in_raw_y: 3,
            in_cam_yaw: 4.0,
            in_cam_raw: 0.858_407,
            in_cam_raw2: 0.0,
            in_cam_mode: 0,
            in_move_x: 0.1,
            in_move_z: 0.9,
            in_move_w: 0.55,
            in_can_act: true,
            in_held: Hex(4),
            in_pressed: Hex(BTN_COMMAND),
            in_command: Hex(CMD_ATTACK),
            in_jump: 1,
            in_attack: 2,
            in_lock: true,
            lock_state: 1,
            target_x: 10.0,
            target_y: 0.0,
            target_z: 250.0,
            target_part: 0,
            abilities: Hex(0x2B0),
            combo_max: 3,
            air_combo_max: 3,
            combo_master: true,
            events: "attack(row=13 req=200 n=1), \"x\"".into(),
            hits: "d1:12:Flinch".into(),
            ..Default::default()
        }
    }

    #[test]
    fn csv_round_trip_is_exact() {
        let rows: Vec<Row> = (0..5).map(sample).chain([Row::default()]).collect();
        let mut buf = Vec::new();
        write_header(&mut buf).unwrap();
        for r in &rows {
            r.write_csv(&mut buf).unwrap();
        }
        let t = parse(std::str::from_utf8(&buf).unwrap()).unwrap();
        assert_eq!(t.rows.len(), rows.len());
        for (a, b) in rows.iter().zip(&t.rows) {
            // Quotes inside text become apostrophes; NaN != NaN, so compare the written forms.
            let (mut wa, mut wb) = (Vec::new(), Vec::new());
            a.write_csv(&mut wa).unwrap();
            b.write_csv(&mut wb).unwrap();
            assert_eq!(String::from_utf8(wa).unwrap(), String::from_utf8(wb).unwrap());
        }
        assert_eq!(t.rows[1].pos_x.to_bits(), (1.0f32 / 3.0).to_bits());
        assert!(t.rows[5].pos_x.is_nan());
        assert_eq!(t.rows[0].events, "attack(row=13 req=200 n=1), 'x'");
        assert!(COLUMNS.iter().all(|c| t.has(c)));
    }

    #[test]
    fn missing_and_unknown_columns() {
        let t = parse("frame,state,pos_x,whatever\n0,2,1.5,zz\n1,0,,\n").unwrap();
        assert_eq!(t.rows.len(), 2);
        assert_eq!(t.rows[0].state, 2);
        assert_eq!(t.rows[0].pos_x, 1.5);
        assert!(t.rows[1].pos_x.is_nan());
        assert_eq!(t.rows[0].entry, -1);
        assert!(t.has("pos_x") && !t.has("in_tilt") && !t.has("whatever"));
        assert!(parse("a,b\n1,2\n").is_err());
    }

    #[test]
    fn buttons_and_camera() {
        assert_eq!(button(BTN_JUMP, BTN_JUMP, BTN_JUMP), 3);
        assert_eq!(button(BTN_JUMP, 0, BTN_JUMP), 1);
        assert_eq!(button(0, BTN_COMMAND, BTN_COMMAND), 2);
        // Stick forward with the core yaw 0 goes +z; full right goes +x.
        let f = move_from_stick([0.0, 1.0], 0.0).unwrap();
        assert!((f[0]).abs() < 1e-6 && (f[1] - 1.0).abs() < 1e-6);
        let r = move_from_stick([1.0, 0.0], 0.0).unwrap();
        assert!((r[0] - 1.0).abs() < 1e-6 && r[1].abs() < 1e-6);
        assert!(move_from_stick([0.0, 0.0], 1.0).is_none());
    }

    /// The camera formula against KH1's own steps (`0x14028C7E0` as read in the decomp):
    /// v = (x, 0, y_down); v1 = v·R(−yaw); v1.x = −v1.x; v2 = v1·R(yaw2), with R(θ)'s rows
    /// (c,0,−s), (0,1,0), (s,0,c).
    #[test]
    fn camera_formula_matches_kh1_steps() {
        let rot = |v: [f32; 2], th: f32| {
            let (s, c) = th.sin_cos();
            // row vector (x, z) times rows r0 = (c, −s), r2 = (s, c) (y dropped)
            [v[0] * c + v[1] * s, -v[0] * s + v[1] * c]
        };
        for &(yaw, yaw2) in &[(0.0f32, 0.0f32), (0.7, 0.0), (-2.1, 0.3), (3.0, -1.0)] {
            for &(sx, sy) in &[(0.0f32, 1.0f32), (1.0, 0.0), (-0.6, 0.8), (0.3, -0.95)] {
                // kh1-core stick (x right, y forward) → KH1 pad (x right, y down).
                let pad = [sx, -sy];
                let mut v = rot(pad, -yaw);
                v[0] = -v[0];
                let v = rot(v, yaw2);
                let l = (v[0] * v[0] + v[1] * v[1]).sqrt();
                let kh1 = [v[0] / l, v[1] / l];
                let core = move_from_stick([sx, sy], kh1_camera_to_core(yaw, yaw2)).unwrap();
                assert!((kh1[0] - core[0]).abs() < 1e-5 && (kh1[1] - core[1]).abs() < 1e-5, "{yaw} {yaw2} {sx} {sy}: {kh1:?} {core:?}");
            }
        }
    }
}
