//! KH1 units ↔ Re:CoM units. `docs/recom-kh1-movement.md` § Units.
//!
//! - **Length.** KH1 world units are centimetres with y down; Re:CoM world units are metres with
//!   y up. KH1's world matrices are `W · diag(1, −1, −1)` × 0.01 in Re:CoM's (same skeleton,
//!   `docs/kh1-recom-assets.md`), so a KH1 point (x, y, z) is Re:CoM (x, −y, −z) / 100.
//!   Checked (files: the skeletons match bone for bone). Movement speeds agree: Re:CoM's jump rises
//!   28 frames to 1.92 m (`0x1401EDA60` with `0x1407C5B38`/`+0x1C` of `0x1407C5B60`), KH1's to
//!   190 units in 28 frames. Checked (exe + decomp).
//! - **Yaw.** Both games face (sin yaw, 0, cos yaw) (KH1 `0x140297E10`; Re:CoM `atan2(x, z)` in
//!   `0x1401E8C00` / `0x1401F2C20` and the forward vector `(0, 0, 1)` at `0x1407C5870` turned by the
//!   body yaw in `0x1401D5670`). With z negated, KH1 yaw ψ is Re:CoM yaw π − ψ. Checked (decomp).
//! - **Time.** Both count 60 Hz frames: Re:CoM's task delta is in 1/3000 s (50 per frame) and
//!   its movement step is `dt × 0.02` (`player+0x464`, set by the Field / Battle exes); KH1's frame
//!   step is 1.0 per 60 Hz frame (`recom-timing.md`). Checked (decomp).
//! - **Velocity.** Re:CoM's mover (`CMovement`, `0x1401D8190` → `0x1401D4E00` / `0x1401D7520` →
//!   `0x1401D4CF0`) moves `pos += v × step` with `v` per 60 Hz frame, so a KH1 displacement `d`
//!   over a step `s` is the velocity `d / s` in Re:CoM units. Checked (decomp).

use kh1_core::math::wrap_angle;
use kh1_core::Vec3;
use std::f32::consts::PI;

/// Re:CoM metres per KH1 unit.
pub const METRES_PER_UNIT: f32 = 0.01;
/// Re:CoM time units per 60 Hz frame (`0x140AC5538 = 3000 / 60`).
pub const UNITS_PER_FRAME: f32 = 50.0;
/// Re:CoM's movement step from a task delta (`player+0x464 = dt × 0.020000001`).
pub const STEP_PER_UNIT: f32 = 0.020_000_001;

/// A KH1 position or displacement → Re:CoM.
pub fn to_recom(v: Vec3) -> Vec3 {
    Vec3::new(v.x * METRES_PER_UNIT, -v.y * METRES_PER_UNIT, -v.z * METRES_PER_UNIT)
}

/// A Re:CoM position or displacement → KH1.
pub fn to_kh1(v: Vec3) -> Vec3 {
    Vec3::new(v.x / METRES_PER_UNIT, -v.y / METRES_PER_UNIT, -v.z / METRES_PER_UNIT)
}

/// KH1 yaw → Re:CoM yaw (and back: the map is its own inverse).
pub fn yaw_to_recom(kh1: f32) -> f32 {
    wrap_angle(PI - kh1)
}

pub fn yaw_to_kh1(recom: f32) -> f32 {
    wrap_angle(PI - recom)
}

/// KH1 frames → Re:CoM time units (`frames × 50`, `recom-timing.md`), rounded to Re:CoM's ints.
pub fn frames_to_units(frames: f32) -> i32 {
    (frames * UNITS_PER_FRAME).round() as i32
}

pub fn units_to_frames(units: i32) -> f32 {
    units as f32 / UNITS_PER_FRAME
}

/// Re:CoM's movement step for a task delta (what KH1 calls the frame step).
pub fn step_from_dt(dt_units: i32) -> f32 {
    dt_units as f32 * STEP_PER_UNIT
}

/// KH1 speed (units / frame) → Re:CoM's `player+0x4DC` "speed" in m/s (its walk is 1.3, run
/// 6.2 × tilt; `0x1401E9630` divides by 60 later).
pub fn speed_to_recom_mps(units_per_frame: f32) -> f32 {
    units_per_frame * METRES_PER_UNIT * 60.0
}

/// One raw stick axis as Re:CoM's device code sees it (`0x14029C560`): byte → ±4096 with a dead
/// zone of 32 and a scale of 94. Used to rebuild the byte from the int (the inverse below).
pub fn recom_axis_from_byte(b: u8) -> i32 {
    let c = b as i32 - 0x80;
    let u = if c < 0 { !c } else { c };
    let v = if u <= 0x1F {
        0
    } else if u == 0x7F {
        0x1000
    } else {
        (u - 0x20) * 0x1000 / 0x5E
    };
    if c < 0 {
        -v
    } else {
        v
    }
}

/// The raw byte behind a Re:CoM stick int (`device+0x74` / `+0x78`), so KH1's own dead zone (60 of
/// 127.5, `kh1_core::stick_from_raw`) can be applied. Exact wherever the byte is outside Re:CoM's
/// dead zone (|c| ≥ 32), except that Re:CoM gives 4096 for both 126 and 127 off centre (bytes
/// 254 / 255 and 1 / 0), read here as the full byte (KH1 tilt 1.0 instead of 0.985). Inside the
/// dead zone it returns the centre (KH1's dead zone is wider, so the tilt is 0 either way).
pub fn byte_from_recom_axis(v: i32) -> u8 {
    if v == 0 {
        return 0x80;
    }
    let neg = v < 0;
    let a = v.unsigned_abs() as i32;
    // The smallest u with (u − 32)·4096/94 == a (or 127 for 4096).
    let u = if a >= 0x1000 { 0x7F } else { (0x20..0x7F).find(|&u| (u - 0x20) * 0x1000 / 0x5E == a).unwrap_or_else(|| (0x20..0x7F).rev().find(|&u| (u - 0x20) * 0x1000 / 0x5E <= a).unwrap_or(0x20)) };
    if neg {
        (0x7F - u) as u8
    } else {
        (0x80 + u) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_round_trip_and_flip_y_z() {
        let k = Vec3::new(100.0, -190.0, 250.0);
        let r = to_recom(k);
        assert!((r.x - 1.0).abs() < 1e-6 && (r.y - 1.9).abs() < 1e-6 && (r.z + 2.5).abs() < 1e-6);
        let back = to_kh1(r);
        assert!((back - k).length() < 1e-3);
    }

    #[test]
    fn yaw_maps_directions() {
        for &psi in &[0.0f32, 0.5, -2.0, 3.0] {
            let d = Vec3::from_yaw(psi);
            let r = to_recom(d);
            let phi = yaw_to_recom(psi);
            // Re:CoM's forward for phi, (sin, 0, cos), is the same direction (scaled to metres).
            assert!((r.x / METRES_PER_UNIT - phi.sin()).abs() < 1e-5, "{psi}");
            assert!((r.z / METRES_PER_UNIT - phi.cos()).abs() < 1e-5, "{psi}");
            assert!(wrap_angle(yaw_to_kh1(phi) - psi).abs() < 1e-6);
        }
    }

    #[test]
    fn speeds_and_time() {
        // KH1 run 8 units/frame = 4.8 m/s; walk 2 = 1.2 m/s (Re:CoM's own: 1.3 walk, 6.2 run).
        assert!((speed_to_recom_mps(8.0) - 4.8).abs() < 1e-5);
        assert!((speed_to_recom_mps(2.0) - 1.2).abs() < 1e-5);
        assert_eq!(frames_to_units(12.0), 600);
        assert_eq!(units_to_frames(1400), 28.0);
        assert!((step_from_dt(50) - 1.0).abs() < 1e-6);
        assert!((step_from_dt(100) - 2.0).abs() < 1e-6);
    }

    #[test]
    fn stick_bytes_round_trip_outside_recoms_dead_zone() {
        for b in 0u8..=255 {
            let v = recom_axis_from_byte(b);
            let back = byte_from_recom_axis(v);
            let c = b as i32 - 0x80;
            let u = if c < 0 { !c } else { c };
            if u == 0x7E {
                // Re:CoM maps both 126 and 127 off centre to 4096: read as the full byte.
                assert_eq!(v.abs(), 0x1000);
                assert_eq!(back, if c < 0 { 0 } else { 255 });
            } else if u > 0x20 {
                assert_eq!(back, b, "byte {b} → {v} → {back}");
            } else {
                assert_eq!(v, 0);
                // KH1's dead zone (60) is wider than Re:CoM's (32): the tilt is 0 either way.
                assert_eq!(kh1_core::stick_from_raw(back, 128, 60.0).1, 0.0);
                assert_eq!(kh1_core::stick_from_raw(b, 128, 60.0).1, 0.0);
            }
        }
    }
}
