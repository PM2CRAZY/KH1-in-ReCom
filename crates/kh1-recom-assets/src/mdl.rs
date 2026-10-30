//! Re:CoM `.MDL` skeleton (`PL0001.MDL`), the part the converter needs.
//!
//! Layout (Checked (files); loader not traced): `+0x10 u16` bone count, `+0x14 u32` name-table
//! offset and `+0x18 u32` rest-matrix offset (both from +0x10). Name table: 20 bytes per bone
//! `{char name[16]; s16 parent; s16 index}`. Rest matrices: 4×4 f32 per bone, row-vector,
//! parent-relative, metres, Y up.

use crate::ann::{cstr, Rd};
use crate::math::Aff;
use crate::{bad, Result};

/// One Re:CoM bone.
#[derive(Clone, Debug, PartialEq)]
pub struct RBone {
    /// Name (`Root`, `ex_0010_sk`, `hip`, … `WeaponR`).
    pub name: String,
    /// Parent index, −1 for none.
    pub parent: i16,
    /// Rest local matrix (parent-relative).
    pub local: Aff,
}

/// A Re:CoM skeleton.
#[derive(Clone, Debug, PartialEq)]
pub struct RecomSkeleton {
    /// Bones in file order (parents come first).
    pub bones: Vec<RBone>,
}

impl RecomSkeleton {
    /// Parses a whole `.MDL` file.
    pub fn parse(d: &[u8]) -> Result<RecomSkeleton> {
        let r = Rd(d);
        let n = r.u16(0x10)? as usize;
        let nt = 0x10 + r.u32(0x14)? as usize;
        let mt = 0x10 + r.u32(0x18)? as usize;
        let mut bones = Vec::with_capacity(n);
        for i in 0..n {
            let o = nt + 20 * i;
            let parent = r.i16(o + 16)?;
            if parent >= i as i16 {
                return bad(format!("mdl: bone {i} has parent {parent}"));
            }
            let m = |k: usize| -> Result<f64> { Ok(r.f32(mt + 64 * i + 4 * k)? as f64) };
            let local = Aff { r: [[m(0)?, m(1)?, m(2)?], [m(4)?, m(5)?, m(6)?], [m(8)?, m(9)?, m(10)?]], t: [m(12)?, m(13)?, m(14)?] };
            bones.push(RBone { name: cstr(r.slice(o, 16)?), parent, local });
        }
        Ok(RecomSkeleton { bones })
    }

    /// Rest world matrices.
    pub fn rest_world(&self) -> Vec<Aff> {
        let mut w: Vec<Aff> = Vec::with_capacity(self.bones.len());
        for b in &self.bones {
            let m = if b.parent < 0 { b.local } else { b.local.mul(&w[b.parent as usize]) };
            w.push(m);
        }
        w
    }

    /// Index of a bone by name.
    pub fn find(&self, name: &str) -> Option<usize> {
        self.bones.iter().position(|b| b.name == name)
    }
}
