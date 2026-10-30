//! One frame of Keyblade hits against dummies, in KH1's order: the attack object's task
//! (`0x1402B4460` → `0x1402B47A0` → `0x1402B3800` shapes → `0x1402B3BE0` tests → `0x1402B4060`
//! apply: `0x1402BF330` damage record + MP charge, hit direction, CB+0x50 bits). Call it after
//! [`Controller::step`] with the blade points of the posed skeleton.

use crate::controller::Controller;
use crate::damage::{add_mp_gauge, compute_damage, mp_charge, AttackerStats, LiveAttack, MpChargeParams};
use crate::data::{DamageCurve, WeaponData};
use crate::math::{clamp_sym, wrap_angle, Vec3};
use crate::reaction::{Dummy, Reaction, ReactionParams};

#[derive(Clone, Debug, PartialEq)]
pub struct CombatParams {
    pub curve: DamageCurve,
    pub mp: MpChargeParams,
    pub reaction: ReactionParams,
    /// The push direction is kept within this of the attacker's facing (`0x1402B4060`: π/4).
    pub hit_dir_clamp: f32,
}

impl CombatParams {
    pub fn kh1_defaults() -> CombatParams {
        CombatParams { curve: DamageCurve::kh1_defaults(), mp: MpChargeParams::default(), reaction: ReactionParams::default(), hit_dir_clamp: std::f32::consts::FRAC_PI_4 }
    }
}

impl Default for CombatParams {
    fn default() -> Self {
        CombatParams::kh1_defaults()
    }
}

/// This frame's blade points at the live attack's fractions (see [`blade_params`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blade {
    pub a: Vec3,
    pub b: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitReport {
    pub dummy: u32,
    pub damage: i32,
    pub critical: bool,
    pub mp_units: i32,
    pub dir: Vec3,
    pub reaction: Reaction,
    pub killed: bool,
}

/// The current swing's attack object numbers and the weapon-bone parameters of A and B, for the
/// host to place the blade points. `None` without a swing.
pub fn blade_params(ctrl: &Controller, weapon: &WeaponData, attacker: &AttackerStats, scale: f32) -> Option<(LiveAttack, f32, f32)> {
    let s = ctrl.swing.as_ref()?;
    let data = ctrl.moveset.attack(s.attack)?;
    let live = LiveAttack::new(data, weapon, attacker, scale);
    let (ta, tb) = live.blade_t(weapon);
    Some((live, ta, tb))
}

/// Tests the live swing against the dummies and applies the hits. `owner_centre` seeds the
/// first frame's swept chain (Sora's body shape centre, bone 0x8003). `scale` = actor +0x40.
#[allow(clippy::too_many_arguments)]
pub fn resolve_hits(ctrl: &mut Controller, blade: Blade, owner_centre: Vec3, weapon: &WeaponData, attacker: &mut AttackerStats, scale: f32, dummies: &mut [Dummy], cp: &CombatParams) -> Vec<HitReport> {
    let mut out = Vec::new();
    if !ctrl.hit_live() {
        if let Some(s) = &mut ctrl.swing {
            s.volume.skip();
        }
        return out;
    }
    let Some((live, _, _)) = blade_params(ctrl, weapon, attacker, scale) else { return out };
    let (shapes, already) = {
        let s = ctrl.swing.as_ref().expect("live swing");
        (s.volume.shapes(blade.a, blade.b, live.radius, live.data.sweeps(), owner_centre), s.hit_owners.clone())
    };
    let facing = ctrl.facing;
    let sora = ctrl.pos;
    for d in dummies.iter_mut() {
        if d.untouchable || d.stats.hp <= 0 || already.contains(&d.id) {
            continue;
        }
        let body = d.shapes();
        if !shapes.iter().any(|a| body.iter().any(|b| a.overlaps(b))) {
            continue;
        }
        let dmg = compute_damage(&live, &cp.curve, &d.stats, &mut ctrl.rng);
        d.stats.hp = (d.stats.hp - dmg.damage).max(0);
        let units = mp_charge(&cp.mp, live.data.mp_charge, d.stats.level, attacker);
        if units > 0 {
            add_mp_gauge(&cp.mp, attacker, units);
        }
        let to = (d.pos - sora).flat().normalized();
        let yaw = wrap_angle(clamp_sym(wrap_angle(to.yaw() - facing), cp.hit_dir_clamp) + facing);
        let dir = Vec3::from_yaw(yaw);
        let reaction = d.react(live.data.kind, dir, &cp.reaction, &mut ctrl.rng);
        ctrl.register_hit(d.id, if dmg.critical { 0x40 } else { 1 });
        out.push(HitReport { dummy: d.id, damage: dmg.damage, critical: dmg.critical, mp_units: units, dir, reaction, killed: d.stats.hp == 0 });
    }
    if let Some(s) = &mut ctrl.swing {
        s.volume.commit(blade.a);
    }
    out
}
