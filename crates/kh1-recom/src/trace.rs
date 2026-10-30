//! The Re:CoM side of the trace (`docs/kh1-trace.md`): F9 in Re:CoM writes
//! `kh1-recom-trace-<n>.csv` next to the game in kh1-trace's CSV schema, so `kh1-tracediff` and
//! `kh1-playground --replay` compare Re:CoM against the oracle and against KH1 traces.
//!
//! The schema is kh1-trace's own file (`crates/kh1-trace/src/schema.rs`), compiled in here: a
//! crate dependency would also link kh1-trace's own DLL entry points into this DLL.
//!
//! Units are KH1's ([`crate::units`]). Row n = frame n's inputs + the state after frame n, as in
//! KH1's traces. Columns Re:CoM cannot fill (combo, hits, MP) keep the schema's "not available".
//! The `events` cell carries Re:CoM's own action / sub / phase / row and the KH1 events.

#[path = "../../kh1-trace/src/schema.rs"]
#[allow(dead_code)]
pub mod schema;

use crate::movement::{FrameInput, FrameOut, Kh1Sora};
use kh1_core::controller::{state, Button};
use kh1_core::math::wrap_angle;
use kh1_core::{stick_from_raw, Vec3};
use schema::{Hex, Row};
use std::f32::consts::PI;

/// KH1's stick dead zone (`0x14298CB00`), on the raw byte (`kh1-sora.md`). Checked (decomp).
pub const KH1_DEAD_ZONE: f32 = 60.0;

/// What the handlers and the trace read from Re:CoM's pad for one frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PadView {
    /// The player's pad wrapper is on (`player+0x230` bit 0).
    pub enabled: bool,
    /// Re:CoM's stick ints (`device+0x74` / `+0x78`, ±4096), the wrapper's negate flag applied.
    pub stick_int: [i32; 2],
    /// The wrapper's stick angle (`pad+4`, `atan2(x, y)`) and its world yaw (`pad+0x34`, the
    /// camera's yaw + angle − π, `0x1401FF150`).
    pub angle: f32,
    pub world_yaw: f32,
    pub jump: Button,
    pub square: Button,
    pub confirm: Button,
    /// The lock-on target (KH1 units).
    pub lock: Option<Vec3>,
}

/// KH1's view of that pad: kh1-core's stick (x right, y forward, KH1's dead zone on the raw
/// bytes), tilt, camera yaw and move direction.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Kh1Pad {
    pub raw: [u8; 2],
    pub stick: [f32; 2],
    pub tilt: f32,
    pub camera_yaw: f32,
    pub move_dir: Vec3,
}

/// Re:CoM's stick → KH1's. Re:CoM moves Sora toward `world_yaw` = camera + angle − π (Re:CoM
/// yaw); the same pad direction in KH1's terms is kh1-core's `move_from_stick(stick, angle −
/// world_yaw)`, so that is the camera yaw. KH1's own dead zone and per-axis rescale apply to the
/// bytes behind Re:CoM's ints ([`crate::units::byte_from_recom_axis`]).
pub fn kh1_pad(p: &PadView) -> Kh1Pad {
    if !p.enabled {
        return Kh1Pad { raw: [0x80, 0x80], ..Default::default() };
    }
    let raw = [crate::units::byte_from_recom_axis(p.stick_int[0]), crate::units::byte_from_recom_axis(p.stick_int[1])];
    let (v, tilt) = stick_from_raw(raw[0], raw[1], KH1_DEAD_ZONE);
    let stick = [v[0], -v[1]];
    let camera_yaw = wrap_angle(p.angle - p.world_yaw);
    let (move_dir, tilt) = if tilt > 0.0 { kh1_core::Input::move_from_stick(stick, camera_yaw) } else { (Vec3::ZERO, 0.0) };
    Kh1Pad { raw, stick, tilt, camera_yaw, move_dir }
}

pub fn frame_input(p: &PadView) -> FrameInput {
    let k = kh1_pad(p);
    FrameInput { move_dir: k.move_dir, tilt: k.tilt, jump: p.jump, square: p.square, lock_target: p.lock }
}

fn replay(b: Button) -> i32 {
    b.held as i32 | (b.pressed as i32) << 1
}

/// Frame n's inputs.
pub fn input_row(frame_step: f32, p: &PadView, abilities: u32) -> Row {
    let k = kh1_pad(p);
    let mut r = Row { step: frame_step, ..Default::default() };
    r.in_stick_x = k.stick[0];
    r.in_stick_y = k.stick[1];
    r.in_tilt = k.tilt;
    r.in_raw_x = k.raw[0] as i32;
    r.in_raw_y = k.raw[1] as i32;
    r.in_cam_yaw = k.camera_yaw;
    // Re:CoM's camera yaw (its convention), the angle the stick is turned by.
    r.in_cam_raw = wrap_angle(p.world_yaw - p.angle + PI);
    r.in_cam_raw2 = f32::NAN;
    r.in_cam_mode = -1;
    r.in_move_x = k.move_dir.x;
    r.in_move_z = k.move_dir.z;
    r.in_move_w = k.tilt;
    let bits = |b: Button, m: u32| if b.held { m } else { 0 };
    let pressed = |b: Button, m: u32| if b.pressed { m } else { 0 };
    r.in_held = Hex(bits(p.jump, schema::BTN_JUMP) | bits(p.square, schema::BTN_SQUARE) | bits(p.confirm, schema::BTN_COMMAND));
    r.in_pressed = Hex(pressed(p.jump, schema::BTN_JUMP) | pressed(p.square, schema::BTN_SQUARE) | pressed(p.confirm, schema::BTN_COMMAND));
    r.in_jump = replay(p.jump);
    r.in_square = replay(p.square);
    r.in_attack = replay(p.confirm);
    r.in_lock = p.lock.is_some();
    r.lock_state = p.lock.is_some() as i32;
    if let Some(t) = p.lock {
        r.target_x = t.x;
        r.target_y = t.y;
        r.target_z = t.z;
    }
    r.abilities = Hex(abilities);
    r.combo_max = 3;
    r.air_combo_max = 3;
    r
}

/// Re:CoM's player as the trace reads it after a frame (KH1 units).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RecomView {
    pub pos: Vec3,
    pub facing: f32,
    pub velocity: Vec3,
    pub airborne: bool,
    pub action: u32,
    pub sub: u32,
    pub phase: u32,
    pub row: i32,
    /// Motion clock, blend total and remaining (Re:CoM units).
    pub motion_time: i32,
    pub blend_total: i32,
    pub blend_left: i32,
}

/// The KH1 state a Re:CoM action stands for (when Re:CoM's own handlers run: `combat = recom`,
/// or a Re:CoM action such as a card): ground 0, take-off 1, air 2, glide 0x15, else 0x100 +
/// action.
pub fn state_of_action(v: &RecomView) -> u32 {
    match v.action {
        2..=4 | 6 => state::GROUND,
        7 => match v.phase {
            1 => state::TAKE_OFF,
            6 => state::GROUND,
            _ => state::AIR,
        },
        8 => state::GLIDE,
        a => 0x100 + a,
    }
}

/// The state after frame n. `kh1`: our handler's record and its end-of-frame state / lock
/// ([`Kh1Sora::after_frame`]), when our handlers drive Sora.
pub fn state_into(r: &mut Row, v: &RecomView, kh1: Option<(&Kh1Sora, (u32, bool), &FrameOut)>) {
    r.pos_x = v.pos.x;
    r.pos_y = v.pos.y;
    r.pos_z = v.pos.z;
    r.facing = v.facing;
    r.grounded = !v.airborne;
    r.motion_frame = crate::units::units_to_frames(v.motion_time);
    r.blend = crate::units::units_to_frames(v.blend_total);
    r.blend_clock = crate::units::units_to_frames(v.blend_total - v.blend_left);
    r.row = -1;
    r.combo_counter = -1;
    r.mp_gauge = -1;
    let recom = format!("recom action={:#x} sub={} phase={} row={:#x}", v.action, v.sub, v.phase, v.row);
    match kh1 {
        Some((s, (st, lock), out)) => {
            r.state = st as i32;
            r.prev_state = s.prev_state as i32;
            let req = s.rec.head();
            r.request = if req == 0xFFFF { -1 } else { req as i32 };
            r.entry = s.motions.entry(req).map_or(-1, |e| e as i32);
            let mf = s.motions.info(req).map_or(0, |m| m.flags_at(r.motion_frame));
            r.motion_flags = Hex(mf);
            r.flags = Hex(if lock { 4 } else { 0 } | if s.no_flinch_on { 0x80 } else { 0 });
            r.action_lock = lock;
            r.combo_window = mf & schema::MF_COMBO_WINDOW != 0;
            r.height = out.height.unwrap_or(f32::NAN);
            r.disp_x = out.displacement.x;
            r.disp_y = out.displacement.y;
            r.disp_z = out.displacement.z;
            r.move_x = s.move_dir.x;
            r.move_z = s.move_dir.z;
            r.speed = s.speed;
            r.air_vx = s.air_vel.x;
            r.air_vy = s.air_vel.y;
            r.air_vz = s.air_vel.z;
            r.target_yaw = s.target_yaw;
            r.air_t = s.arc.t;
            r.rise_t = s.arc.rise_time;
            r.no_flinch = if s.no_flinch_on { s.no_flinch } else { 0.0 };
            let ev: Vec<String> = out.events.iter().map(|e| format!("{e:?}")).collect();
            r.events = if ev.is_empty() { recom } else { format!("{recom}; {}", ev.join("; ")) };
        }
        None => {
            r.state = state_of_action(v) as i32;
            r.prev_state = -1;
            r.request = -1;
            r.entry = -1;
            r.target_yaw = f32::NAN;
            r.speed = v.velocity.flat().length();
            r.air_vx = v.velocity.x;
            r.air_vy = v.velocity.y;
            r.air_vz = v.velocity.z;
            r.events = format!("{recom}; Re:CoM's own handler");
        }
    }
}

/// The trace file name for recording `n`.
pub fn file_name(n: u32) -> String {
    format!("kh1-recom-trace-{n}.csv")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad_for(stick_bytes: [u8; 2], camera: f32) -> PadView {
        // What Re:CoM's device and pad wrapper hold for these bytes and camera (`0x14029C560`,
        // `0x1401FF150`): ints, angle atan2(x, y), world yaw = camera + angle − π.
        let ints = [crate::units::recom_axis_from_byte(stick_bytes[0]), crate::units::recom_axis_from_byte(stick_bytes[1])];
        let angle = (ints[0] as f32 / 4096.0).atan2(ints[1] as f32 / 4096.0);
        PadView { enabled: true, stick_int: ints, angle, world_yaw: wrap_angle(camera + angle - PI), ..Default::default() }
    }

    #[test]
    fn kh1_stick_moves_where_recoms_would() {
        for &camera in &[0.0f32, 1.0, -2.5] {
            for &(bx, by) in &[(255u8, 128u8), (128, 0), (0, 255), (230, 30)] {
                let p = pad_for([bx, by], camera);
                let k = kh1_pad(&p);
                assert!(k.tilt > 0.0);
                // Re:CoM's own direction for this angle, in KH1 terms (yaw π − world yaw).
                let recom_dir = Vec3::from_yaw(crate::units::yaw_to_kh1(p.world_yaw));
                // KH1's rescale can turn the angle a little off the axes; on them it is exact.
                if bx == 128 || by == 128 || bx == by {
                    assert!((k.move_dir - recom_dir).length() < 1e-4, "{bx},{by} cam {camera}: {:?} vs {:?}", k.move_dir, recom_dir);
                } else {
                    assert!(k.move_dir.dot(recom_dir) > 0.98);
                }
                // The trace's camera yaw replays to the same direction through kh1-core.
                let (d, _) = kh1_core::Input::move_from_stick(k.stick, k.camera_yaw);
                assert!((d - k.move_dir).length() < 1e-5);
            }
        }
    }

    #[test]
    fn kh1_dead_zone_and_tilt_line() {
        // 0x80 + 59: inside KH1's dead zone (60), outside Re:CoM's (32).
        assert_eq!(kh1_pad(&pad_for([0x80 + 59, 0x80], 0.0)).tilt, 0.0);
        let k = kh1_pad(&pad_for([255, 0x80], 0.0));
        assert!((k.tilt - 1.0).abs() < 1e-6);
        // The walk / run line: tilt 0.8 = byte 127.5 + 60 + 54.
        let k = kh1_pad(&pad_for([0x80 + 113, 0x80], 0.0));
        assert!(k.tilt <= 0.8 + 1e-3 && k.tilt > 0.79, "{}", k.tilt);
        assert_eq!(kh1_pad(&PadView::default()).tilt, 0.0);
    }

    #[test]
    fn rows_round_trip_through_the_schema() {
        let p = PadView { jump: Button { pressed: true, held: true }, square: Button { pressed: false, held: true }, ..pad_for([255, 128], 0.5) };
        let mut r = input_row(1.0, &p, 0x230);
        assert_eq!(r.in_jump, 3);
        assert_eq!(r.in_square, 1);
        assert_eq!(r.in_held.0, schema::BTN_JUMP | schema::BTN_SQUARE);
        let s = Kh1Sora::new(kh1_core::Params::kh1_defaults(), kh1_core::Abilities::default());
        let v = RecomView { pos: Vec3::new(1.0, -2.0, 3.0), action: 7, phase: 2, row: 4, motion_time: 150, blend_total: 400, blend_left: 100, ..Default::default() };
        state_into(&mut r, &v, Some((&s, (state::AIR, false), &FrameOut::default())));
        assert_eq!(r.motion_frame, 3.0);
        assert_eq!(r.blend_clock, 6.0);
        assert!(r.events.contains("action=0x7"));
        let mut buf = Vec::new();
        schema::write_header(&mut buf).unwrap();
        r.write_csv(&mut buf).unwrap();
        let t = schema::parse(std::str::from_utf8(&buf).unwrap()).unwrap();
        // NaN cells ("not available") make `==` false; the written text must be the same.
        let mut again = Vec::new();
        t.rows[0].write_csv(&mut again).unwrap();
        let mut first = Vec::new();
        r.write_csv(&mut first).unwrap();
        assert_eq!(again, first);
        assert_eq!(file_name(3), "kh1-recom-trace-3.csv");
    }

    #[test]
    fn recom_actions_to_states() {
        let v = |action, phase| RecomView { action, phase, ..Default::default() };
        assert_eq!(state_of_action(&v(3, 0)), 0);
        assert_eq!(state_of_action(&v(7, 1)), 1);
        assert_eq!(state_of_action(&v(7, 3)), 2);
        assert_eq!(state_of_action(&v(7, 6)), 0);
        assert_eq!(state_of_action(&v(8, 4)), 0x15);
        assert_eq!(state_of_action(&v(0xB, 0)), 0x10B);
    }
}
