//! An MDLX's skeleton: one 0x40-byte record per bone (`index, sibling, parent, child` i16,
//! reserved, flags, then scale / rotation / translation as three f32×4 rows), at the model's
//! bone offset. Layout after OpenKH's `docs/kh2/file/type/mdlx.md` (Apache-2.0), ported from
//! KH2-Combat-in-KH1's `kh2-formats/src/anim.rs`.
//!
//! The bind pose follows that project's `kh2-anim` (`Pose::bind`, checked there against KH2's
//! own motions): rotations are Euler angles in radians applied X, then Y, then Z (column
//! vectors: `R = Rz · Ry · Rx`); a bone's unscaled world matrix is its parent's times its own
//! rotation + translation; scale accumulates per axis and multiplies the world matrix's axes;
//! the bone's position is its translation through its parent's scaled matrix.

use crate::bar::Bar;
use crate::reader::Reader;
use crate::{bad, Result};

/// BAR entry type of a model inside an MDLX.
pub const BAR_TYPE_MODEL: u16 = 4;
pub(crate) const RESERVED: usize = 0x90;

/// One joint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bone {
    pub index: i16,
    pub sibling: i16,
    pub parent: i16,
    pub child: i16,
    pub flags: i32,
    pub scale: [f32; 3],
    pub rotation: [f32; 3],
    pub translation: [f32; 3],
}

impl Bone {
    pub const SIZE: usize = 0x40;

    fn read(buf: &[u8], at: usize) -> Result<Bone> {
        let mut r = Reader::at(buf, at);
        let index = r.i16()?;
        let sibling = r.i16()?;
        let parent = r.i16()?;
        let child = r.i16()?;
        let _reserved = r.i32()?;
        let flags = r.i32()?;
        let mut v = [[0f32; 4]; 3];
        for row in &mut v {
            for x in row.iter_mut() {
                *x = r.f32()?;
            }
        }
        let three = |a: [f32; 4]| [a[0], a[1], a[2]];
        Ok(Bone { index, sibling, parent, child, flags, scale: three(v[0]), rotation: three(v[1]), translation: three(v[2]) })
    }
}

/// Reads the bone list of the first model of an MDLX.
pub fn parse_skeleton(mdlx: &[u8]) -> Result<Vec<Bone>> {
    let bar = Bar::parse(mdlx)?;
    let Some(model) = bar.entries.iter().find(|e| e.kind == BAR_TYPE_MODEL) else {
        return bad("MDLX has no model entry");
    };
    let d = model.data;
    let mut h = Reader::at(d, RESERVED + 0x10);
    let count = h.u16()? as usize;
    let _textures = h.u16()?;
    let offset = h.u32()? as usize;
    (0..count).map(|i| Bone::read(d, RESERVED + offset + i * Bone::SIZE)).collect()
}

/// A 4×4 matrix, columns (`m[c][r]`), acting on column vectors.
pub type Mat = [[f64; 4]; 4];

/// The identity.
pub const IDENTITY: Mat = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];

/// `a · b`.
pub fn mul(a: &Mat, b: &Mat) -> Mat {
    let mut o = [[0.0; 4]; 4];
    for (c, col) in o.iter_mut().enumerate() {
        for (r, x) in col.iter_mut().enumerate() {
            *x = (0..4).map(|k| a[k][r] * b[c][k]).sum();
        }
    }
    o
}

/// `m · (p, 1)`.
pub fn point(m: &Mat, p: [f64; 3]) -> [f64; 3] {
    core::array::from_fn(|r| m[0][r] * p[0] + m[1][r] * p[1] + m[2][r] * p[2] + m[3][r])
}

/// `Rz(z) · Ry(y) · Rx(x)` with translation `t`.
pub fn rt(rot: [f32; 3], t: [f32; 3]) -> Mat {
    let (sx, cx) = (rot[0] as f64).sin_cos();
    let (sy, cy) = (rot[1] as f64).sin_cos();
    let (sz, cz) = (rot[2] as f64).sin_cos();
    // Columns of Rz·Ry·Rx.
    [
        [cz * cy, sz * cy, -sy, 0.0],
        [cz * sy * sx - sz * cx, sz * sy * sx + cz * cx, cy * sx, 0.0],
        [cz * sy * cx + sz * sx, sz * sy * cx - cz * sx, cy * cx, 0.0],
        [t[0] as f64, t[1] as f64, t[2] as f64, 1.0],
    ]
}

/// Each bone's bind-pose world matrix (model space; scale applied), as `kh2-anim`'s
/// `Pose::bind`. A parent index outside the list (or the bone itself) means a root.
pub fn bind_pose(bones: &[Bone]) -> Vec<Mat> {
    let n = bones.len();
    let mut unscaled = vec![IDENTITY; n];
    let mut scale = vec![[1.0f64; 3]; n];
    let mut out = vec![IDENTITY; n];
    for (i, b) in bones.iter().enumerate() {
        let p = (b.parent >= 0 && (b.parent as usize) < n && b.parent as usize != i && (b.parent as usize) < i).then_some(b.parent as usize);
        let own = rt(b.rotation, b.translation);
        let s = b.scale.map(|x| x as f64);
        scale[i] = p.map_or(s, |p| core::array::from_fn(|k| scale[p][k] * s[k]));
        unscaled[i] = p.map_or(own, |p| mul(&unscaled[p], &own));
        let pos = p.map_or(b.translation.map(|x| x as f64), |p| point(&out[p], b.translation.map(|x| x as f64)));
        let u = &unscaled[i];
        out[i] = [
            [u[0][0] * scale[i][0], u[0][1] * scale[i][0], u[0][2] * scale[i][0], 0.0],
            [u[1][0] * scale[i][1], u[1][1] * scale[i][1], u[1][2] * scale[i][1], 0.0],
            [u[2][0] * scale[i][2], u[2][1] * scale[i][2], u[2][2] * scale[i][2], 0.0],
            [pos[0], pos[1], pos[2], 1.0],
        ];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f64; 3], b: [f64; 3]) -> bool {
        (0..3).all(|k| (a[k] - b[k]).abs() < 1e-6)
    }

    #[test]
    fn euler_is_x_then_y_then_z() {
        let h = std::f32::consts::FRAC_PI_2;
        // X by 90° takes +y to +z; then Z by 90° takes +z to +z (unchanged).
        let m = rt([h, 0.0, h], [0.0; 3]);
        assert!(close(point(&m, [0.0, 1.0, 0.0]), [0.0, 0.0, 1.0]));
        // +x: X leaves it, Z by 90° takes it to +y.
        assert!(close(point(&m, [1.0, 0.0, 0.0]), [0.0, 1.0, 0.0]));
    }

    #[test]
    fn bind_chains_parents() {
        let b = |parent: i16, rot: [f32; 3], t: [f32; 3]| Bone { index: 0, sibling: -1, parent, child: -1, flags: 0, scale: [1.0; 3], rotation: rot, translation: t };
        let h = std::f32::consts::FRAC_PI_2;
        let bones = [b(-1, [0.0, 0.0, h], [1.0, 0.0, 0.0]), b(0, [0.0; 3], [2.0, 0.0, 0.0])];
        let p = bind_pose(&bones);
        // Child at parent + Rz(90°)·(2,0,0) = (1,2,0).
        assert!(close(point(&p[1], [0.0; 3]), [1.0, 2.0, 0.0]));
    }
}
