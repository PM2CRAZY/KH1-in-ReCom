//! Row-vector affine maths exactly as KH1's matrix helpers do it (`p' = p · M`, translation in
//! row 3). f32 like the game.

#![allow(clippy::needless_range_loop)]

/// A 3-vector.
pub type Vec3 = [f32; 3];

/// Row-vector affine transform: `p' = p · r + t`. Rows 0..2 of `r` are the (scaled) X, Y, Z
/// axes; `t` is row 3. Equal to a 4×4 row-vector matrix whose column 3 is `(0, 0, 0, 1)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine {
    /// Rows 0..2.
    pub r: [[f32; 3]; 3],
    /// Row 3 (translation).
    pub t: Vec3,
}

impl Default for Affine {
    fn default() -> Self {
        Affine::IDENTITY
    }
}

impl Affine {
    /// The identity.
    pub const IDENTITY: Affine = Affine { r: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], t: [0.0; 3] };

    /// A pure translation.
    pub fn from_translation(t: Vec3) -> Affine {
        Affine { t, ..Affine::IDENTITY }
    }

    /// `self · parent` (apply `self` first).
    #[inline]
    pub fn mul(&self, b: &Affine) -> Affine {
        let a = self;
        let mut r = [[0.0f32; 3]; 3];
        for (i, row) in r.iter_mut().enumerate() {
            for (j, v) in row.iter_mut().enumerate() {
                *v = a.r[i][0] * b.r[0][j] + a.r[i][1] * b.r[1][j] + a.r[i][2] * b.r[2][j];
            }
        }
        Affine { r, t: b.point(a.t) }
    }

    /// `(p, 1) · M` (`FUN_1401DAD10`).
    #[inline]
    pub fn point(&self, p: Vec3) -> Vec3 {
        let m = &self.r;
        [
            p[0] * m[0][0] + p[1] * m[1][0] + p[2] * m[2][0] + self.t[0],
            p[0] * m[0][1] + p[1] * m[1][1] + p[2] * m[2][1] + self.t[1],
            p[0] * m[0][2] + p[1] * m[1][2] + p[2] * m[2][2] + self.t[2],
        ]
    }

    /// `(v, 0) · M` (direction, no translation).
    #[inline]
    pub fn vector(&self, v: Vec3) -> Vec3 {
        let m = &self.r;
        [
            v[0] * m[0][0] + v[1] * m[1][0] + v[2] * m[2][0],
            v[0] * m[0][1] + v[1] * m[1][1] + v[2] * m[2][1],
            v[0] * m[0][2] + v[1] * m[1][2] + v[2] * m[2][2],
        ]
    }

    /// KH1's inverse `FUN_1401DB130`: normalise the rows, transpose, divide the rows of the
    /// transpose by the original row lengths, translation `-(t · R')`. Exact only for uniform
    /// scale; used as written.
    pub fn inverse_kh1(&self) -> Affine {
        let l = [len(self.r[0]), len(self.r[1]), len(self.r[2])];
        let mut n = self.r;
        for i in 0..3 {
            if l[i] != 0.0 {
                for v in n[i].iter_mut() {
                    *v /= l[i];
                }
            }
        }
        let mut r = [[0.0f32; 3]; 3];
        for i in 0..3 {
            let d = if l[i] != 0.0 { l[i] } else { 1e-6 };
            for j in 0..3 {
                r[i][j] = n[j][i] / d;
            }
        }
        let o = Affine { r, t: [0.0; 3] };
        let tt = o.vector(self.t);
        Affine { r, t: [-tt[0], -tt[1], -tt[2]] }
    }

    /// `FUN_1401DB340`: rows normalised, plus the row lengths.
    pub fn unscale(&self) -> (Affine, Vec3) {
        let mut o = *self;
        let mut l = [0.0f32; 3];
        for i in 0..3 {
            l[i] = len(o.r[i]);
            let d = if l[i] != 0.0 { l[i] } else { 1e-6 };
            for v in o.r[i].iter_mut() {
                *v /= d;
            }
        }
        (o, l)
    }

    /// `FUN_140394CF0`: rows 0..2 times `s`, translation kept.
    #[inline]
    pub fn scale_rows(&self, s: Vec3) -> Affine {
        let mut o = *self;
        for i in 0..3 {
            for v in o.r[i].iter_mut() {
                *v *= s[i];
            }
        }
        o
    }

    /// `M · diag(s, 1)`: every row's columns scaled (translation included).
    #[inline]
    pub fn scale_cols(&self, s: Vec3) -> Affine {
        let mut o = *self;
        for row in o.r.iter_mut().chain(std::iter::once(&mut o.t)) {
            for j in 0..3 {
                row[j] *= s[j];
            }
        }
        o
    }

    /// The matrix as a flat 4×4 row-major array of the row-vector form. Read as column-major,
    /// this is the column-vector matrix (e.g. `glam::Mat4::from_cols_array`).
    pub fn to_array(&self) -> [f32; 16] {
        let r = &self.r;
        [
            r[0][0], r[0][1], r[0][2], 0.0, r[1][0], r[1][1], r[1][2], 0.0, r[2][0], r[2][1], r[2][2], 0.0, self.t[0], self.t[1],
            self.t[2], 1.0,
        ]
    }

    /// Converts a KH1 world matrix (identity model matrix: KH1 units ≈ cm, Y down) into the
    /// model's Y-up space in metres: `W · diag(1, −1, −1)`, translation × 0.01. This is Re:CoM's
    /// space for the same skeleton (`kh1-posing.md` §2.1, Checked (files)).
    pub fn to_yup_metres(&self) -> Affine {
        let mut o = *self;
        for row in o.r.iter_mut() {
            row[1] = -row[1];
            row[2] = -row[2];
        }
        o.t = point_to_yup_metres(self.t);
        o
    }
}

/// A KH1 point (cm, Y down) in the model's Y-up space in metres.
pub fn point_to_yup_metres(p: Vec3) -> Vec3 {
    [p[0] * 0.01, -p[1] * 0.01, -p[2] * 0.01]
}

#[inline]
pub(crate) fn len(v: Vec3) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// `FUN_1401DB4E0`: normalise, unchanged when zero.
#[inline]
pub(crate) fn norm(v: Vec3) -> Vec3 {
    let l = len(v);
    if l != 0.0 {
        [v[0] / l, v[1] / l, v[2] / l]
    } else {
        v
    }
}

#[inline]
pub(crate) fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

#[inline]
pub(crate) fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[inline]
pub(crate) fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

#[inline]
pub(crate) fn mulv(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] * b[0], a[1] * b[1], a[2] * b[2]]
}

/// `(1 − w)·a + w·b`.
#[inline]
pub(crate) fn lerp(a: Vec3, b: Vec3, w: f32) -> Vec3 {
    let u = 1.0 - w;
    [u * a[0] + w * b[0], u * a[1] + w * b[1], u * a[2] + w * b[2]]
}

/// Rotation about X (row vectors).
#[inline]
pub(crate) fn rx(a: f32) -> Affine {
    let (s, c) = a.sin_cos();
    Affine { r: [[1.0, 0.0, 0.0], [0.0, c, s], [0.0, -s, c]], t: [0.0; 3] }
}

#[inline]
fn ry(a: f32) -> Affine {
    let (s, c) = a.sin_cos();
    Affine { r: [[c, 0.0, -s], [0.0, 1.0, 0.0], [s, 0.0, c]], t: [0.0; 3] }
}

#[inline]
fn rz(a: f32) -> Affine {
    let (s, c) = a.sin_cos();
    Affine { r: [[c, s, 0.0], [-s, c, 0.0], [0.0, 0.0, 1.0]], t: [0.0; 3] }
}

/// `FUN_1401DB080`: `diag(S) · Rx · Ry · Rz`, translation `t`; an axis is skipped when its
/// angle is exactly 0.
pub(crate) fn mat_srt(s: Vec3, r: Vec3, t: Vec3) -> Affine {
    let mut m = Affine { r: [[s[0], 0.0, 0.0], [0.0, s[1], 0.0], [0.0, 0.0, s[2]]], t: [0.0; 3] };
    if r[0] != 0.0 {
        m = m.mul(&rx(r[0]));
    }
    if r[1] != 0.0 {
        m = m.mul(&ry(r[1]));
    }
    if r[2] != 0.0 {
        m = m.mul(&rz(r[2]));
    }
    m.t = t;
    m
}

/// `FUN_1401DB430`: `Rx · Ry · Rz`, translation `t` (no scale).
pub(crate) fn mat_rt(r: Vec3, t: Vec3) -> Affine {
    mat_srt([1.0; 3], r, t)
}

/// Wraps to [−π, π] with the exe's constants 3.1415927 / 6.2831855.
pub(crate) fn wrap(v: f32) -> f32 {
    kh1_formats::mset::wrap_angle(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverse_and_compose() {
        let m = mat_srt([2.0, 2.0, 2.0], [0.3, -0.2, 1.1], [1.0, 2.0, 3.0]);
        let i = m.inverse_kh1();
        let p = [0.5, -4.0, 9.0];
        let q = i.point(m.point(p));
        for k in 0..3 {
            assert!((p[k] - q[k]).abs() < 1e-4);
        }
        let (u, l) = m.unscale();
        assert!((l[0] - 2.0).abs() < 1e-6 && (len(u.r[1]) - 1.0).abs() < 1e-6);
        let y = Affine::IDENTITY.to_yup_metres();
        assert_eq!(y.r[1][1], -1.0);
        assert_eq!(point_to_yup_metres([100.0, 200.0, 300.0]), [1.0, -2.0, -3.0]);
    }
}
