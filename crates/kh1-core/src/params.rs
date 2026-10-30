//! Tunable numbers: Sora's stats block defaults (`0x1402A5D70`), the `btltbl.bin` globals at
//! file 0x58 (`G`, read by `0x1402A6180`), and constants KH1 keeps in code. Every field is plain
//! data; `kh1-data` fills them from the player's files. Units: KH1 units, 60 Hz frames.
//! [`Params::kh1_defaults`] holds the values read from the Steam exe and the owner's files
//! (`docs/kh1-sora.md`, `docs/kh1-sora-data.md`).

use std::f32::consts::PI;

/// KH1's gravity, units per frame² (a literal in `0x1402AF6A0`, `0x1402AF960`, `0x1402AF8F0`).
pub const GRAVITY: f32 = 0.484_693_88;

#[derive(Clone, Debug, PartialEq)]
pub struct Params {
    // ---- stats block (actor +0x6C) ----
    /// +0x04: walk speed (tilt in (0, run_tilt]).
    pub walk_speed: f32,
    /// +0x08: run speed (tilt over run_tilt); also the air-speed cap and the guard/roll line.
    pub run_speed: f32,
    /// The tilt line between walk and run (`0x1402A5F10`: 0.8 literal).
    pub run_tilt: f32,
    /// +0xD0: facing turn per frame, radians (π/16).
    pub turn_rate: f32,
    /// +0x14: air acceleration factor (× speed, per frame, not step-scaled).
    pub air_accel: f32,
    /// +0x18: air drag per 30 Hz step (its square root per frame when step < 2).
    pub air_drag: f32,
    // ---- jump ----
    pub gravity: f32,
    /// Largest fall per frame (× step).
    pub fall_cap: f32,
    /// btltbl +0xAC / +0xAE (Sora): stats +0x10, the jump height and the jump-up reach.
    pub jump_height: f32,
    pub high_jump_height: f32,
    /// Take-off crouch frames (`0x1402AFDA0`: 4.0).
    pub crouch_frames: f32,
    /// Releasing jump cuts the rise only once t ≥ this (`0x1402A7B30`: 6.0)...
    pub cut_min_t: f32,
    /// ...and while T > t + this (5.0).
    pub cut_margin: f32,
    /// The cut's new rise time (`0x1402AF8F0(5.0)`: H = −g·T²/2 = 6.06 units).
    pub cut_rise_time: f32,
    /// Air speed cap after walking off a ledge (`0x14029A3C0`: 8.0).
    pub walk_off_cap: f32,
    /// A jump pressed during the land motion restarts the take-off at this t (`0x1402A6220`:
    /// 2.0, and the jump motion starts at frame 2).
    pub land_jump_t: f32,
    pub glide: GlideParams,
    /// Dodge Roll flinch-immunity timer (`0x1402C9E60`: `0x140297F30(12.0)`).
    pub roll_no_flinch: f32,
    pub combo: ComboParams,
    pub blend: Blends,
}

/// Glide / Superglide (state 0x15; `0x1402B2A20`, `0x1402B2880`, `0x1402B2D30`, `0x1402B2780`,
/// `0x1402B2BF0`, `0x1402B2D00`, `0x1402B29F0`). All Checked (decomp).
#[derive(Clone, Debug, PartialEq)]
pub struct GlideParams {
    /// Glide starts only more than this far above the floor (+0x3D4).
    pub min_height: f32,
    /// Air velocity × this on start.
    pub start_keep: f32,
    /// Forward speed gain per frame, and the top speeds.
    pub accel: f32,
    pub speed: f32,
    pub super_speed: f32,
    /// Turn per frame (radians): 2° / 1°.
    pub turn: f32,
    pub super_turn: f32,
    /// Descent caps per frame; Superglide's shrinks below `low_height`:
    /// `((h/low)²·0.8 + 0.2)·2`.
    pub descent: f32,
    pub super_descent: f32,
    pub low_height: f32,
    /// Rising speed damping per 30 Hz step (its square root per frame).
    pub rise_damp: f32,
    /// Timer on enter (counts down twice per frame while gliding) and on exit (frames before the
    /// next glide).
    pub enter_timer: f32,
    pub exit_timer: f32,
    /// On end: velocity × this and a hop this high.
    pub end_keep: f32,
    pub end_hop: f32,
    /// Ends when it moved less than this share of the intended distance (or rose).
    pub stall_ratio: f32,
    /// Turn motions 0x73 / 0x74 above this angle (30°); 0x72 above this speed (8).
    pub turn_motion_angle: f32,
    pub fast_motion_speed: f32,
}

/// Combo picking (btltbl G block and `0x1402A6EE0` / `0x1402A6C70` / `0x1402A68F0` literals).
#[derive(Clone, Debug, PartialEq)]
pub struct ComboParams {
    /// G+0x00 / +0x04, whole degrees. The exe halves them as **integers** before the cosine
    /// (`0x1402A6180`: `cos((G[0] >> 1) / 180 · π)`): 45 → 22°, 240 → 120°.
    pub narrow_cone_deg: u32,
    pub wide_cone_deg: u32,
    /// G+0x08 / +0x0C.
    pub close_range: f32,
    pub lunge_range: f32,
    /// Combo length before Combo Plus / Air Combo Plus (`0x1402A5130`: 3).
    pub base_length: u8,
    /// Jump-up attack: target more than this above (160), less than reach + `jump_up_reach_extra`
    /// (150) above; speed clamp((gap − 50) / 14, ±8); hop clamp(h − 30, 100, reach).
    pub jump_up_min_height: f32,
    pub jump_up_reach_extra: f32,
    pub jump_up_speed_offset: f32,
    pub jump_up_speed_div: f32,
    pub jump_up_speed_cap: f32,
    pub jump_up_hop_offset: f32,
    pub jump_up_hop_min: f32,
    /// Air row 17 when the target is more than this above Sora (`0x1402A6C70`: y-down dy < −100).
    pub air_target_above: f32,
    /// Air swing speeds toward the target: first hit, middle hits, finishers (rows 15/16).
    pub air_speed_first: f32,
    pub air_speed_middle: f32,
    pub air_speed_finisher: f32,
    /// Row 2 (obstacle hop) ground speed.
    pub hop_speed: f32,
    /// The obstacle probe reaches this far beyond the body radius (`0x1402A6EE0`: 20.0).
    pub obstacle_probe: f32,
    /// Ability finisher slices of the random draw (Stun Impact, Gravity Break, Zantetsuken).
    pub finisher_slice: f32,
}

/// Motion blend lengths the code passes (60 Hz frames).
#[derive(Clone, Debug, PartialEq)]
pub struct Blends {
    pub idle: f32,
    pub walk_run: f32,
    pub fall: f32,
    pub land_to_idle: f32,
    pub glide: f32,
    pub roll_to_idle: f32,
}

impl Params {
    /// The numbers from KH1 FM Steam's exe and the owner's `btltbl.bin`.
    pub fn kh1_defaults() -> Params {
        Params {
            walk_speed: 2.0,
            run_speed: 8.0,
            run_tilt: 0.8,
            turn_rate: PI / 16.0,
            air_accel: 0.1,
            air_drag: 0.95,
            gravity: GRAVITY,
            fall_cap: 20.0,
            jump_height: 190.0,
            high_jump_height: 290.0,
            crouch_frames: 4.0,
            cut_min_t: 6.0,
            cut_margin: 5.0,
            cut_rise_time: 5.0,
            walk_off_cap: 8.0,
            land_jump_t: 2.0,
            glide: GlideParams {
                min_height: 50.0,
                start_keep: 0.5,
                accel: 0.4,
                speed: 8.0,
                super_speed: 16.0,
                turn: 0.034_906_59,
                super_turn: 0.017_453_294,
                descent: 1.2,
                super_descent: 2.0,
                low_height: 70.0,
                rise_damp: 0.8,
                enter_timer: 46.0,
                exit_timer: 24.0,
                end_keep: 0.7,
                end_hop: 8.0,
                stall_ratio: 0.7,
                turn_motion_angle: std::f32::consts::FRAC_PI_6,
                fast_motion_speed: 8.0,
            },
            roll_no_flinch: 12.0,
            combo: ComboParams {
                narrow_cone_deg: 45,
                wide_cone_deg: 240,
                close_range: 180.0,
                lunge_range: 300.0,
                base_length: 3,
                jump_up_min_height: 160.0,
                jump_up_reach_extra: 150.0,
                jump_up_speed_offset: 50.0,
                jump_up_speed_div: 14.0,
                jump_up_speed_cap: 8.0,
                jump_up_hop_offset: 30.0,
                jump_up_hop_min: 100.0,
                air_target_above: 100.0,
                air_speed_first: 8.0,
                air_speed_middle: 4.0,
                air_speed_finisher: 12.0,
                hop_speed: 8.0,
                obstacle_probe: 20.0,
                finisher_slice: 0.3,
            },
            blend: Blends { idle: 10.0, walk_run: 8.0, fall: 8.0, land_to_idle: 8.0, glide: 16.0, roll_to_idle: 8.0 },
        }
    }

    /// The narrow cone's cosine as the exe computes it (f32, integer half-angle).
    pub fn narrow_cos(&self) -> f32 {
        cone_cos(self.combo.narrow_cone_deg)
    }

    pub fn wide_cos(&self) -> f32 {
        cone_cos(self.combo.wide_cone_deg)
    }

    /// Stats +0x10: the jump height with or without High Jump.
    pub fn reach(&self, a: &Abilities) -> f32 {
        if a.high_jump {
            self.high_jump_height
        } else {
            self.jump_height
        }
    }

    /// Ground combo length (stats +0xD4) and air combo length (+0xD5).
    pub fn ground_length(&self, a: &Abilities) -> u8 {
        self.combo.base_length.saturating_add(a.combo_plus)
    }

    pub fn air_length(&self, a: &Abilities) -> u8 {
        self.combo.base_length.saturating_add(a.air_combo_plus)
    }
}

impl Default for Params {
    fn default() -> Self {
        Params::kh1_defaults()
    }
}

fn cone_cos(deg: u32) -> f32 {
    (((deg >> 1) as f32) / 180.0 * PI).cos()
}

/// Sora's equipped abilities, as the bits of `0x142D5EC10` (`0x1402A6380`) and the counts of
/// `0x140284BE0`. Ability names follow the Archipelago list (Assumption, strong).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Abilities {
    /// bit 0x200 (id 1).
    pub high_jump: bool,
    /// bit 0x80.
    pub glide: bool,
    /// bit 0x100: Superglide on Square in the air.
    pub superglide: bool,
    /// bit 0x10 (id 21).
    pub guard: bool,
    /// bit 0x20 (id 22).
    pub dodge_roll: bool,
    /// Copies of ability 6 / 7 (+1 combo length each).
    pub combo_plus: u8,
    pub air_combo_plus: u8,
    /// Copies of ability 8: crit factor 1 + n (stats +0xD8).
    pub critical_plus: u8,
    /// Ability 0x41: a press inside the window goes on even when the last swing missed.
    pub combo_master: bool,
    /// bit 0x1 (17).
    pub vortex: bool,
    /// bit 0x2 (18).
    pub aerial_sweep: bool,
    /// bit 0x8 (20).
    pub blitz: bool,
    /// bit 0x1000000 (54).
    pub sliding_dash: bool,
    /// bit 0x2000000 (55).
    pub hurricane_blast: bool,
    /// bit 0x4000000 (56).
    pub ripple_drive: bool,
    /// bit 0x8000000 (57).
    pub stun_impact: bool,
    /// bit 0x10000000 (58).
    pub gravity_break: bool,
    /// bit 0x20000000 (59).
    pub zantetsuken: bool,
}

/// One stick axis byte pair → a stick vector (`0x14028B4F0`). Per axis: c = raw − 127.5; inside
/// ±`dead_zone` it is 0, otherwise (|c| − dead_zone) / (127.5 − dead_zone) with its sign. The
/// result's length is the tilt, clamped to 1. KH1's dead zone is 60 (`0x14298CB00`); the
/// alternate device path uses 1. Returns (x, y) with y down on the stick as the pad bytes are.
pub fn stick_from_raw(raw_x: u8, raw_y: u8, dead_zone: f32) -> ([f32; 2], f32) {
    let axis = |b: u8| {
        let c = b as f32 - 127.5;
        let v = if c <= 0.0 {
            let v = c + dead_zone;
            if v <= 0.0 {
                v
            } else {
                0.0
            }
        } else {
            let v = c - dead_zone;
            if v >= 0.0 {
                v
            } else {
                0.0
            }
        };
        v / (127.5 - dead_zone)
    };
    let (x, y) = (axis(raw_x), axis(raw_y));
    let len = (x * x + y * y).sqrt();
    let tilt = len.min(1.0);
    if len > 0.0 {
        ([x / len * tilt, y / len * tilt], tilt)
    } else {
        ([0.0, 0.0], 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stick_dead_zone_and_rescale() {
        assert_eq!(stick_from_raw(128, 128, 60.0).1, 0.0);
        assert_eq!(stick_from_raw(187, 128, 60.0).1, 0.0); // 59.5 inside the dead zone
        let (v, t) = stick_from_raw(255, 128, 60.0);
        assert!((t - 1.0).abs() < 1e-6 && (v[0] - 1.0).abs() < 1e-6);
        // 127.5 + 60 + 33.75 = half way on the rescaled axis
        let (_, t) = stick_from_raw(221, 128, 60.0);
        assert!((t - 33.5 / 67.5).abs() < 1e-5);
        let (_, t) = stick_from_raw(255, 0, 60.0);
        assert_eq!(t, 1.0, "the diagonal is clamped to 1");
    }

    #[test]
    fn cones_use_integer_half_angles() {
        let p = Params::kh1_defaults();
        assert!((p.narrow_cos() - 22f32.to_radians().cos()).abs() < 1e-6);
        assert!((p.wide_cos() + 0.5).abs() < 1e-6);
    }
}
