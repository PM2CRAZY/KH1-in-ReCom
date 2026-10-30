//! KH1's attack object numbers (`0x1402B2DC0` + `0x1402B3270`), the damage function
//! (`0x1402BFA80`) and the MP charge on hits (`0x1402BF330`, gauge `0x1402A4AE0`).
//! Checked (decomp) unless marked.

use crate::data::{AttackData, DamageCurve, WeaponData};
use crate::rng::Kh1Rand;

/// The attacker's stats block fields the attack reads.
#[derive(Clone, Debug, PartialEq)]
pub struct AttackerStats {
    /// +0x4C: Strength (the refresh adds the weapon's +0x30; capped at 100).
    pub strength: i32,
    /// +0x48: max MP (the Magic-source records read it too).
    pub max_mp: i32,
    /// Save record byte 0: level. Assumption (notes) for the field.
    pub level: i32,
    /// +0xD8: 1 + Critical Plus copies.
    pub crit_factor: f32,
    /// +0xE0: the min/max clamp scale (default 1.0, `0x1402A5D70`).
    pub bound_scale: f32,
    /// +0xBE: MP charge gauge (30 units = 1 MP; capped at max MP × 30).
    pub mp_gauge: i32,
    /// work +0x184 bit 0: +1.5 to the MP charge factor (which ability sets it: unknown).
    pub mp_charge_bonus: bool,
    /// work +0x184 bit 0x20 with the party check in `0x1402B2DC0`: Strength + G+0xFE (4) for
    /// Strength-source records. Assumption: which state sets it is not traced.
    pub strength_bonus: Option<i32>,
}

impl Default for AttackerStats {
    fn default() -> Self {
        AttackerStats { strength: 5, max_mp: 2, level: 1, crit_factor: 1.0, bound_scale: 1.0, mp_gauge: 0, mp_charge_bonus: false, strength_bonus: None }
    }
}

/// The defender's stats block fields the damage reads (an enemy's).
#[derive(Clone, Debug, PartialEq)]
pub struct DefenderStats {
    /// +0x3C / +0x40.
    pub hp: i32,
    pub max_hp: i32,
    /// +0x50.
    pub defence: i32,
    /// +0xDC: level (the MP charge uses it; 0 = no charge).
    pub level: i32,
    /// +0xC0 / +0xC2: damage floor / cap (0 = none).
    pub min_damage: u16,
    pub max_damage: u16,
    /// +0x78 + element × 4: damage multiplier per element.
    pub resist: [f32; 16],
    /// +0x98: the % of HP element's multiplier (element 8).
    pub hp_percent_scale: f32,
    /// actor +0x88 bit 8: halves damage (Assumption (notes): Aero).
    pub aero: bool,
    /// work +0x184 bit 0x10: cannot be brought below 1 HP.
    pub cannot_die: bool,
    /// +0xC4: launch weight, 0..1 (speed = min + (max − min) × this).
    pub launch_weight: f32,
}

impl Default for DefenderStats {
    fn default() -> Self {
        DefenderStats {
            hp: 100,
            max_hp: 100,
            defence: 0,
            level: 1,
            min_damage: 0,
            max_damage: 0,
            resist: [1.0; 16],
            hp_percent_scale: 1.0,
            aero: false,
            cannot_die: false,
            launch_weight: 1.0,
        }
    }
}

/// An attack object as built for one swing: the record plus the weapon and the attacker.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveAttack {
    pub data: AttackData,
    /// obj+0x2C: the stat term.
    pub stat: i32,
    /// obj+0x0C / +0x24 / +0x28.
    pub radius: f32,
    pub frac_a: f32,
    pub frac_b: f32,
    /// obj+0x68, clamped 1..=99 for weapon attacks.
    pub priority: i32,
    /// obj+0x94 / +0x98.
    pub crit_chance: f32,
    pub crit_bonus: i32,
    /// obj+0xAC.
    pub bound_scale: f32,
}

impl LiveAttack {
    /// `0x1402B2DC0` then `0x1402B3270` for a weapon-bone record. `scale` = actor +0x40.
    pub fn new(data: &AttackData, weapon: &WeaponData, attacker: &AttackerStats, scale: f32) -> LiveAttack {
        let stat = match data.stat_source {
            0 => attacker.strength + attacker.strength_bonus.unwrap_or(0),
            1 => attacker.max_mp,
            2 => 0,
            3 => attacker.max_mp * 2,
            4 => attacker.max_mp * 3,
            5 => attacker.max_mp * 5,
            _ => attacker.strength, // 6: the leader's Strength (Sora leads)
        };
        let mut a = LiveAttack {
            data: data.clone(),
            stat,
            radius: data.radius,
            frac_a: data.frac_a,
            frac_b: data.frac_b,
            priority: data.priority,
            crit_chance: data.crit as f32 * 0.05 * attacker.crit_factor,
            crit_bonus: data.crit_bonus,
            bound_scale: attacker.bound_scale,
        };
        if data.bone == 0x30 {
            a.radius += scale * weapon.radius;
            a.frac_a *= weapon.frac_scale;
            a.frac_b *= weapon.frac_scale;
            a.priority = (a.priority + weapon.priority as i32).clamp(1, 99);
            a.crit_chance *= weapon.crit as f32 * 0.05;
            a.crit_bonus += weapon.crit_bonus as i32;
        }
        a
    }

    /// The weapon-bone parameters of points A and B.
    pub fn blade_t(&self, weapon: &WeaponData) -> (f32, f32) {
        (weapon.blade_t(self.frac_a), weapon.blade_t(self.frac_b))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DamageResult {
    pub damage: i32,
    /// Record bit 0x20.
    pub critical: bool,
}

/// `0x1402BFA80` for a hit on the enemy side. Draws the crit roll only when the power is non-zero.
pub fn compute_damage(atk: &LiveAttack, curve: &DamageCurve, def: &DefenderStats, rng: &mut Kh1Rand) -> DamageResult {
    let mut power = atk.data.power + atk.stat;
    let (min, max) = (atk.data.min, atk.data.max);
    let mut out = DamageResult::default();
    if power == 0 {
        return out;
    }
    if rng.next_f32() < atk.crit_chance {
        power += atk.crit_bonus;
        out.critical = true;
    }
    let element = (atk.data.element & 0xF) as usize;
    match element {
        8 => {
            let floor = if def.min_damage < 2 { 1 } else { def.min_damage as i32 };
            let v = (def.hp as f32 * (power as f32 / 100.0) * def.hp_percent_scale) as i32;
            out.damage = if v < floor { floor } else { v };
            return out;
        }
        9 | 11 | 12 => {
            out.damage = power;
            return out;
        }
        _ => {}
    }
    let n = curve.0.len() as i32;
    let mut i = power - def.defence;
    if i < 0 {
        i = 0;
    } else if n <= i {
        i = n - 1;
    }
    let mut d = curve.0.get(i as usize).copied().unwrap_or(1) as i32;
    if min != 0 {
        let m = (min as f32 * atk.bound_scale) as i32;
        if d < m {
            d = m;
        }
    }
    if max != 0 {
        let m = (max as f32 * atk.bound_scale) as i32;
        if m < d {
            d = m;
        }
    }
    if def.min_damage == 0 || def.min_damage as i32 <= d {
        if def.max_damage != 0 && (def.max_damage as i32) < d {
            d = def.max_damage as i32;
        }
    } else {
        d = def.min_damage as i32;
    }
    let v = (d as f32 * def.resist[element]) as i32;
    d = if def.aero { v / 2 } else { v };
    if def.cannot_die && def.hp <= d {
        d = def.hp - 1;
    }
    out.damage = if d != 0 { d } else { 1 };
    out
}

/// The btltbl G numbers the MP charge reads.
#[derive(Clone, Debug, PartialEq)]
pub struct MpChargeParams {
    /// G+0x94: % per level of difference (20).
    pub per_level: u8,
    /// G+0x95: added % with the bonus bit (150).
    pub bonus: u8,
    /// G+0x96: cap, % of the record's charge (250).
    pub cap_percent: u16,
    /// 30 gauge units per MP (`0x1402A4AE0`).
    pub units_per_mp: i32,
}

impl Default for MpChargeParams {
    fn default() -> Self {
        MpChargeParams { per_level: 20, bonus: 150, cap_percent: 250, units_per_mp: 30 }
    }
}

/// `0x1402BF330`: gauge units one hit charges. 0 when the enemy's level is 0.
pub fn mp_charge(p: &MpChargeParams, record_charge: i32, enemy_level: i32, attacker: &AttackerStats) -> i32 {
    if enemy_level <= 0 {
        return 0;
    }
    let mut f = (enemy_level - attacker.level) as f32 * p.per_level as f32 / 100.0 + 1.0;
    if attacker.mp_charge_bonus {
        f += p.bonus as f32 / 100.0;
    }
    let v = (record_charge as f32 * f) as i32;
    let v = if 1 < v { v } else { 1 };
    let cap = (p.cap_percent as i32 * record_charge) / 100;
    if cap <= v {
        cap
    } else {
        v
    }
}

/// `0x1402A4AE0`: adds to the gauge, capped at max MP × 30. Returns the new gauge.
pub fn add_mp_gauge(p: &MpChargeParams, attacker: &mut AttackerStats, units: i32) -> i32 {
    let cap = attacker.max_mp * p.units_per_mp;
    attacker.mp_gauge = (attacker.mp_gauge + units).min(cap).max(0);
    attacker.mp_gauge
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Moveset;

    fn live(off: u32, str_: i32) -> LiveAttack {
        let m = Moveset::kh1_defaults();
        let a = AttackerStats { strength: str_, ..Default::default() };
        LiveAttack::new(m.attack(off).unwrap(), &WeaponData::kingdom_key(), &a, 1.0)
    }

    #[test]
    fn swing_and_finisher_damage() {
        let c = DamageCurve::kh1_defaults();
        let mut r = Kh1Rand::new(3);
        let d = DefenderStats { defence: 2, ..Default::default() };
        // Basic swing: power 0 + Strength 8 − Defence 2 → curve[6] = 6.
        assert_eq!(compute_damage(&live(0x4B78, 8), &c, &d, &mut r).damage, 6);
        // Finisher: +4 power → curve[10] = 12 (when it doesn't crit).
        let f = live(0x4C58, 8);
        assert!((f.crit_chance - 0.2).abs() < 1e-6, "4 × 0.05 × 1 × Kingdom Key 20 × 0.05");
        let mut seen = (false, false);
        for _ in 0..200 {
            let out = compute_damage(&f, &c, &d, &mut r);
            if out.critical {
                assert_eq!(out.damage, 24, "crit: 4 + 8 + 4 − 2 = 14 → curve[14]");
                seen.1 = true;
            } else {
                assert_eq!(out.damage, 12);
                seen.0 = true;
            }
        }
        assert!(seen.0 && seen.1);
    }

    #[test]
    fn floors_caps_and_resist() {
        let c = DamageCurve::kh1_defaults();
        let mut r = Kh1Rand::new(1);
        let a = live(0x4B78, 60);
        assert_eq!(compute_damage(&a, &c, &DefenderStats::default(), &mut r).damage, 300, "index clamps at 41");
        let d = DefenderStats { defence: 99, ..Default::default() };
        assert_eq!(compute_damage(&a, &c, &d, &mut r).damage, 1, "index clamps at 0 → 1");
        let d = DefenderStats { max_damage: 50, ..Default::default() };
        assert_eq!(compute_damage(&a, &c, &d, &mut r).damage, 50);
        let mut d = DefenderStats::default();
        d.resist[1] = 0.0;
        assert_eq!(compute_damage(&a, &c, &d, &mut r).damage, 1, "an immune enemy still takes 1");
        let z = live(0x4B78, 0);
        assert_eq!(compute_damage(&z, &c, &DefenderStats::default(), &mut r).damage, 0, "power 0 deals nothing");
    }

    #[test]
    fn mp_charge_by_level() {
        let p = MpChargeParams::default();
        let s = AttackerStats { level: 5, ..Default::default() };
        assert_eq!(mp_charge(&p, 3, 5, &s), 3);
        assert_eq!(mp_charge(&p, 6, 5, &s), 6);
        assert_eq!(mp_charge(&p, 3, 10, &s), 6, "(1 + 5 × 0.2) × 3");
        assert_eq!(mp_charge(&p, 3, 20, &s), 7, "4 × 3 = 12 capped at 250 % = 7");
        assert_eq!(mp_charge(&p, 3, 1, &s), 1, "0.2 × 3 floors at 1");
        assert_eq!(mp_charge(&p, 3, 0, &s), 0);
        let mut s = AttackerStats { max_mp: 2, ..Default::default() };
        assert_eq!(add_mp_gauge(&p, &mut s, 50), 50);
        assert_eq!(add_mp_gauge(&p, &mut s, 50), 60);
    }
}
