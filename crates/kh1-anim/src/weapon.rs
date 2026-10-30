//! Attach points and the Keyblade hit line (`kh1-attack-combo.md` §5, `kh1-data-values.md` §2;
//! Checked (decomp): `FUN_14029EB20`, `FUN_14029FAB0`, `FUN_1401D62E0`, `FUN_1401D6320`,
//! `FUN_1402B3270`).
//!
//! Attack data +0x10 names the bone by attach id; 0x30 is the weapon. For Sora the attach table
//! maps 0x30 to bone 283 (Checked (files)), whose parent 282 sits at the Keyblade's grip; bone
//! 283 is 21.1 units along 282's X axis. A point at line parameter `t` is
//! `lerp(W[282].t, T[283] · W[282], t)`: `t = 0` is the grip, negative `t` runs the other way,
//! along the blade.

use crate::math::Vec3;
use crate::rig::Rig;

/// The weapon attach id (attack data +0x10).
pub const WEAPON_ATTACH: u16 = 0x30;

/// The fields of a weapon (Keyblade) record `W = CB+0x20` (btltbl weapon rows, file 0x94F8,
/// 0x58 bytes each) that place the hit line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeaponLine {
    /// W+0x28: line parameter at blade fraction 0 (0.0 in every Sora weapon row).
    pub t0: f32,
    /// W+0x24: line parameter at blade fraction 1 (Kingdom Key −3.7, Ultima −5.6).
    pub t1: f32,
    /// W+0x2C: blade fraction scale (0.8 for most rows).
    pub frac_scale: f32,
    /// W+0x20: radius added to the hit spheres, × actor scale (Kingdom Key 40).
    pub radius: f32,
}

impl Rig {
    /// `FUN_14029EB20(actor, id, f)` for a bone attach id: the bone's point at line parameter
    /// `f`; with `weapon` and id 0x30, `f` is first mapped through the weapon line
    /// (`t = t0 + (t1 − t0)·f`). `None` when the id names no bone (the exe then uses the actor
    /// position).
    pub fn attach_point(&self, id: u16, f: f32, weapon: Option<&WeaponLine>) -> Option<Vec3> {
        let bone = self.attach_bone(id)?;
        let t = match weapon {
            Some(w) if id == WEAPON_ATTACH => (w.t1 - w.t0) * f + w.t0,
            _ => f,
        };
        Some(self.point_on_bone(bone, t))
    }

    /// The Keyblade hit segment of an attack (`FUN_1402B3270` scales the attack data's blade
    /// fractions +0x54 / +0x58 by W+0x2C): `(A, B)` with A at `frac_a · W+0x2C` and B at
    /// `frac_b · W+0x2C`. Kingdom Key with fractions 1.0 / 0.0: A at line t −2.96, B at the
    /// grip. KH1 fills it with spheres of radius `data+0x5C + actor scale · W+0x20` (spacing =
    /// the mean radius) plus a swept chain from last frame's A; that part is the hit code's.
    pub fn blade_segment(&self, weapon: &WeaponLine, frac_a: f32, frac_b: f32) -> Option<(Vec3, Vec3)> {
        let a = self.attach_point(WEAPON_ATTACH, frac_a * weapon.frac_scale, Some(weapon))?;
        let b = self.attach_point(WEAPON_ATTACH, frac_b * weapon.frac_scale, Some(weapon))?;
        Some((a, b))
    }
}
