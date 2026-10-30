//! KH1's Attack row pickers: ground `0x1402A6EE0` (+ scan `0x1402A7320`), air `0x1402A6C70`.
//! Checked (decomp). Each builds a flag per row, then takes the **lowest flagged row whose motion
//! differs from the one playing**; with none, a fallback. That rule is what alternates swings.

use crate::data::{row, Moveset};
use crate::math::Vec3;
use crate::params::{Abilities, Params};

/// One enemy body part as the pickers and hit tests see it (`0x1402B04C0` / `0x1402B5E50`): the
/// point on its bone less the part's height offset, and its radius (× actor scale).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TargetPart {
    /// The enemy actor (one actor can have several parts).
    pub owner: u32,
    pub centre: Vec3,
    pub radius: f32,
}

/// What the ground picker decided.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundPick {
    pub row: usize,
    /// The attack direction (the move vector's xyz): toward the target, or the stick's.
    pub aim: Vec3,
    /// The move vector's y: the jump-up attack's hop height.
    pub hop: f32,
    /// The move vector's w: the speed word (jump-up speed, else 0; row 2 sets 8 in the exec).
    pub speed: f32,
}

pub struct GroundCtx<'a> {
    pub params: &'a Params,
    pub abilities: &'a Abilities,
    pub moveset: &'a Moveset,
    /// CB+0x41 after this press's update, and stats +0xD4.
    pub counter: u8,
    pub length: u8,
    /// actor +0x164.
    pub current_request: i32,
    /// Sora's position (actor +0x10).
    pub pos: Vec3,
    /// The move vector the command step passes in (stick direction / facing; its w is 0).
    pub aim: Vec3,
    /// The command's target (the lock-on), if still valid.
    pub target: Option<TargetPart>,
    /// Every enemy body part, for the scan.
    pub parts: &'a [TargetPart],
}

/// Rows of `flags` that are flagged, lowest first, whose motion differs from the current one.
fn first_flagged(flags: &[i32], moveset: &Moveset, current: i32) -> Option<usize> {
    (0..flags.len()).find(|&r| flags[r] > 0 && moveset.row_request(r) != current)
}

/// `0x1402A7320`: counts parts in the cones around `aim`. Flags 14 (narrow cone, < lunge range),
/// 13 (narrow, < close), 11 (wide but not narrow, < close); Vortex copies 11 into 10.
pub fn scan(params: &Params, abilities: &Abilities, pos: Vec3, aim: Vec3, parts: &[TargetPart], f: &mut [i32]) {
    let narrow = params.narrow_cos();
    let wide = params.wide_cos();
    for p in parts {
        let (dir, len) = (p.centre - pos).flat().normalize_len();
        let gap = len - p.radius;
        let d = dir.dot(aim);
        if d <= narrow {
            if wide < d && gap < params.combo.close_range {
                f[row::SWING_2] += 1;
            }
        } else if gap < params.combo.lunge_range {
            f[row::LUNGE] += 1;
            if gap < params.combo.close_range {
                f[row::SWING_1] += 1;
            }
        }
        if abilities.vortex {
            f[row::VORTEX] = f[row::SWING_2];
        }
    }
}

/// `0x1402A6EE0`. `rand` is drawn only for a finisher press (the exe draws it first);
/// `wall_ahead(aim)` is the obstacle probe `0x1402C6E70` (body radius + 20 along the aim).
pub fn pick_ground(c: &GroundCtx, rand: &mut dyn FnMut() -> f32, wall_ahead: &mut dyn FnMut(Vec3) -> bool) -> GroundPick {
    let p = &c.params.combo;
    let a = c.abilities;
    let mut f = [0i32; row::GROUND_COUNT];
    if c.length <= c.counter {
        let mut r = rand();
        let mut chosen = None;
        for (on, rw) in [(a.stun_impact, row::STUN_IMPACT), (a.gravity_break, row::GRAVITY_BREAK), (a.zantetsuken, row::ZANTETSUKEN)] {
            if !on {
                continue;
            }
            r -= p.finisher_slice;
            if r < 0.0 {
                chosen = Some(rw);
                break;
            }
        }
        f[chosen.unwrap_or(row::GROUND_FINISHER)] = 1;
        if a.ripple_drive {
            f[row::RIPPLE_DRIVE] = 1;
        }
    }
    let mut aim = c.aim;
    let mut hop = aim.y;
    let speed;
    match c.target {
        None => {
            scan(c.params, a, c.pos, aim, c.parts, &mut f);
            speed = 0.0;
        }
        Some(t) => {
            let reach = c.params.reach(a);
            let d = t.centre - c.pos;
            let h = -d.y; // y-down: height of the target above Sora
            let (dir, len) = d.flat().normalize_len();
            aim = dir;
            hop = 0.0;
            let gap = len - t.radius;
            if h <= p.jump_up_min_height || reach + p.jump_up_reach_extra <= h || p.lunge_range <= gap {
                if p.lunge_range <= gap || gap <= p.close_range {
                    if !a.sliding_dash || p.jump_up_min_height <= h || gap < p.lunge_range {
                        scan(c.params, a, c.pos, aim, c.parts, &mut f);
                        speed = 0.0;
                    } else {
                        // The speed word keeps the gap here; the row exec zeroes it.
                        f[row::SLIDING_DASH] = 1;
                        speed = gap;
                    }
                } else {
                    if a.vortex {
                        f[row::VORTEX] = 1;
                    }
                    f[row::LUNGE] = 1;
                    speed = 0.0;
                }
            } else {
                f[if a.aerial_sweep { row::AERIAL_SWEEP } else { row::JUMP_UP }] = 1;
                let s = (gap - p.jump_up_speed_offset) / p.jump_up_speed_div;
                speed = if s < -p.jump_up_speed_cap { -p.jump_up_speed_cap } else if s > p.jump_up_speed_cap { p.jump_up_speed_cap } else { s };
                let hh = h - p.jump_up_hop_offset;
                let capped = if reach <= hh { reach } else { hh };
                hop = if capped < p.jump_up_hop_min { p.jump_up_hop_min } else if hh < reach { hh } else { reach };
            }
        }
    }
    if wall_ahead(aim) {
        f[row::HOP] = 1;
    }
    if a.blitz && f[row::GROUND_FINISHER] > 0 && f[row::SWING_2] == 0 {
        f[row::BLITZ] = 1;
    }
    let row_pick = first_flagged(&f, c.moveset, c.current_request).unwrap_or_else(|| {
        let cur = c.current_request;
        if c.counter < 2 || (cur == c.moveset.row_request(row::LUNGE) && f[row::LUNGE] > 0) || cur == c.moveset.row_request(row::SWING_2) {
            row::SWING_1
        } else {
            row::SWING_2
        }
    });
    GroundPick { row: row_pick, aim, hop, speed }
}

pub struct AirCtx<'a> {
    pub params: &'a Params,
    pub abilities: &'a Abilities,
    pub moveset: &'a Moveset,
    /// CB+0x41 after this press's update, and stats +0xD5.
    pub counter: u8,
    pub length: u8,
    pub current_request: i32,
    /// Movement state 2 (Hurricane Blast is only offered there).
    pub in_state_2: bool,
    pub pos: Vec3,
    pub aim: Vec3,
    pub target: Option<TargetPart>,
}

/// `0x1402A6C70`. Returns the row and the aim (toward the target with y = 0, or the stick's).
pub fn pick_air(c: &AirCtx) -> (usize, Vec3) {
    let (params, abilities, moveset) = (c.params, c.abilities, c.moveset);
    let (current_request, pos, target) = (c.current_request, c.pos, c.target);
    let finisher = c.length <= c.counter;
    let mut f = [0i32; 19];
    f[row::AIR_FINISHER] = finisher as i32;
    let mut aim = c.aim;
    match target {
        None => f[row::AIR_SWING_1] = 1,
        Some(t) => {
            let d = t.centre - pos;
            // y-down: dy < −100 means the target is more than 100 above Sora.
            if -params.combo.air_target_above <= d.y {
                f[row::AIR_SWING_1] = 1;
                if c.in_state_2 && abilities.hurricane_blast {
                    f[row::HURRICANE_BLAST] = finisher as i32;
                }
            } else {
                f[row::AIR_SWING_2] = 1;
            }
            aim = d.flat();
        }
    }
    let picked = (row::HURRICANE_BLAST..=row::AIR_SWING_1).find(|&r| f[r] > 0 && moveset.row_request(r) != current_request);
    let r = picked.unwrap_or_else(|| (row::HURRICANE_BLAST..=row::AIR_SWING_1).rev().find(|&r| moveset.row_request(r) != current_request).unwrap_or(row::AIR_SWING_1));
    (r, aim)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx<'a>(p: &'a Params, a: &'a Abilities, m: &'a Moveset, counter: u8, cur: i32, target: Option<TargetPart>, parts: &'a [TargetPart]) -> GroundCtx<'a> {
        GroundCtx { params: p, abilities: a, moveset: m, counter, length: 3, current_request: cur, pos: Vec3::ZERO, aim: Vec3::new(0.0, 0.0, 1.0), target, parts }
    }

    fn part(z: f32, y: f32) -> TargetPart {
        TargetPart { owner: 1, centre: Vec3::new(0.0, y, z), radius: 30.0 }
    }

    #[test]
    fn ground_rows_by_range_and_alternation() {
        let p = Params::kh1_defaults();
        let a = Abilities::default();
        let m = Moveset::kh1_defaults();
        let mut no_rand = || 0.0;
        let mut open = |_: Vec3| false;
        // Nothing in front: 13, then 11 (fallback), then the finisher.
        assert_eq!(pick_ground(&ctx(&p, &a, &m, 1, 0, None, &[]), &mut no_rand, &mut open).row, 13);
        assert_eq!(pick_ground(&ctx(&p, &a, &m, 2, 200, None, &[]), &mut no_rand, &mut open).row, 11);
        assert_eq!(pick_ground(&ctx(&p, &a, &m, 3, 201, None, &[]), &mut no_rand, &mut open).row, 8);
        // A close enemy in front flags 13 and 14: the 2nd swing is the lunge motion.
        let t = part(100.0, -50.0);
        assert_eq!(pick_ground(&ctx(&p, &a, &m, 1, 0, Some(t), &[t]), &mut no_rand, &mut open).row, 13);
        assert_eq!(pick_ground(&ctx(&p, &a, &m, 2, 200, Some(t), &[t]), &mut no_rand, &mut open).row, 14);
        // Gap 250 (between 180 and 300): lunge.
        let t = part(280.0, -50.0);
        assert_eq!(pick_ground(&ctx(&p, &a, &m, 1, 0, Some(t), &[t]), &mut no_rand, &mut open).row, 14);
        // Side enemy (60° off, close): row 11 first.
        let s = TargetPart { owner: 2, centre: Vec3::new(100.0 * 60f32.to_radians().sin(), -50.0, 100.0 * 60f32.to_radians().cos()), radius: 0.0 };
        assert_eq!(pick_ground(&ctx(&p, &a, &m, 1, 0, None, &[s]), &mut no_rand, &mut open).row, 11);
        // Obstacle ahead: hop row 2.
        let mut wall = |_: Vec3| true;
        assert_eq!(pick_ground(&ctx(&p, &a, &m, 1, 0, None, &[]), &mut no_rand, &mut wall).row, 2);
    }

    #[test]
    fn jump_up_speed_and_hop() {
        let p = Params::kh1_defaults();
        let a = Abilities::default();
        let m = Moveset::kh1_defaults();
        let t = part(130.0, -250.0); // 250 above, gap 100
        let g = pick_ground(&ctx(&p, &a, &m, 1, 0, Some(t), &[t]), &mut || 0.0, &mut |_| false);
        assert_eq!(g.row, 1);
        assert!((g.speed - 50.0 / 14.0).abs() < 1e-5);
        assert_eq!(g.hop, 190.0, "h − 30 = 220 is clamped to the reach");
        let t = part(130.0, -170.0);
        let g = pick_ground(&ctx(&p, &a, &m, 1, 0, Some(t), &[t]), &mut || 0.0, &mut |_| false);
        assert_eq!(g.hop, 140.0);
    }

    #[test]
    fn air_rows() {
        let p = Params::kh1_defaults();
        let a = Abilities::default();
        let m = Moveset::kh1_defaults();
        let z = Vec3::new(0.0, 0.0, 1.0);
        let air = |counter: u8, cur: i32, target: Option<TargetPart>| {
            pick_air(&AirCtx { params: &p, abilities: &a, moveset: &m, counter, length: 3, current_request: cur, in_state_2: true, pos: Vec3::ZERO, aim: z, target }).0
        };
        assert_eq!(air(1, 4, None), 18);
        assert_eq!(air(2, 204, None), 17);
        assert_eq!(air(3, 205, None), 16);
        let above = TargetPart { owner: 1, centre: Vec3::new(0.0, -150.0, 50.0), radius: 0.0 };
        assert_eq!(air(1, 4, Some(above)), 17, "y down: more than 100 above Sora");
        let below = TargetPart { owner: 1, centre: Vec3::new(0.0, 150.0, 50.0), radius: 0.0 };
        assert_eq!(air(1, 4, Some(below)), 18);
    }
}
