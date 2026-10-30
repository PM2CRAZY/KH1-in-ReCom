//! f64 row-vector affine maths for the conversion (`p' = p · r + t`, like both games).

#![allow(clippy::needless_range_loop)]

/// 3×3, row-major, row vectors.
pub type M3 = [[f64; 3]; 3];

/// The identity.
pub const I3: M3 = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// Row-vector affine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aff {
    /// Rows 0..2.
    pub r: M3,
    /// Translation (row 3).
    pub t: [f64; 3],
}

impl Aff {
    /// The identity.
    pub const IDENTITY: Aff = Aff { r: I3, t: [0.0; 3] };

    /// `self · b` (apply self first).
    pub fn mul(&self, b: &Aff) -> Aff {
        Aff { r: m3(&self.r, &b.r), t: b.point(self.t) }
    }

    /// `(p, 1) · M`.
    pub fn point(&self, p: [f64; 3]) -> [f64; 3] {
        let mut o = self.t;
        for j in 0..3 {
            o[j] += (0..3).map(|k| p[k] * self.r[k][j]).sum::<f64>();
        }
        o
    }

    /// From a KH1 f32 affine.
    pub fn from_kh1(a: &kh1_anim::Affine) -> Aff {
        let mut r = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                r[i][j] = a.r[i][j] as f64;
            }
        }
        Aff { r, t: [a.t[0] as f64, a.t[1] as f64, a.t[2] as f64] }
    }

    /// A KH1 world matrix (KH1 units ≈ cm, Y down) in Re:CoM's model space (metres, Y up):
    /// `W · diag(1, −1, −1)`, translation × 0.01 (`kh1-posing.md` §2.1; Checked (files)).
    pub fn kh1_to_recom(a: &kh1_anim::Affine) -> Aff {
        let mut o = Aff::from_kh1(a);
        for row in o.r.iter_mut() {
            row[1] = -row[1];
            row[2] = -row[2];
        }
        o.t = [o.t[0] * 0.01, -o.t[1] * 0.01, -o.t[2] * 0.01];
        o
    }
}

/// `a · b`.
pub fn m3(a: &M3, b: &M3) -> M3 {
    let mut o = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            o[i][j] = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    o
}

/// Transpose.
pub fn tr(a: &M3) -> M3 {
    let mut o = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            o[i][j] = a[j][i];
        }
    }
    o
}

/// `v · M` (row vector).
pub fn vm(v: [f64; 3], m: &M3) -> [f64; 3] {
    let mut o = [0.0; 3];
    for j in 0..3 {
        o[j] = (0..3).map(|k| v[k] * m[k][j]).sum();
    }
    o
}

/// Rows normalised, and their lengths.
pub fn unscale(a: &M3) -> (M3, [f64; 3]) {
    let mut o = *a;
    let mut l = [0.0; 3];
    for i in 0..3 {
        l[i] = (o[i][0] * o[i][0] + o[i][1] * o[i][1] + o[i][2] * o[i][2]).sqrt();
        if l[i] != 0.0 {
            for v in o[i].iter_mut() {
                *v /= l[i];
            }
        }
    }
    (o, l)
}

/// `Rx(a) · Ry(b) · Rz(c)`, row vectors (both games' Euler order; Re:CoM's `qz·qy·qx`).
pub fn rot_xyz(a: f64, b: f64, c: f64) -> M3 {
    let (sx, cx) = a.sin_cos();
    let (sy, cy) = b.sin_cos();
    let (sz, cz) = c.sin_cos();
    let x = [[1.0, 0.0, 0.0], [0.0, cx, sx], [0.0, -sx, cx]];
    let y = [[cy, 0.0, -sy], [0.0, 1.0, 0.0], [sy, 0.0, cy]];
    let z = [[cz, sz, 0.0], [-sz, cz, 0.0], [0.0, 0.0, 1.0]];
    m3(&m3(&x, &y), &z)
}

/// Euler XYZ of a rotation `R = Rx(a)·Ry(b)·Rz(c)`, the solution closest to `near` (each angle
/// may also move by whole turns), so that curves stay continuous frame to frame.
pub fn euler_xyz(r: &M3, near: Option<[f64; 3]>) -> [f64; 3] {
    use std::f64::consts::PI;
    let sb = (-r[0][2]).clamp(-1.0, 1.0);
    let b = sb.asin();
    let cb = b.cos();
    let (a, c) = if cb > 1e-9 {
        (r[1][2].atan2(r[2][2]), r[0][1].atan2(r[0][0]))
    } else {
        // Gimbal: c = 0; row 1 = (sa·sb, ca, ·).
        ((r[1][0] / sb).atan2(r[1][1]), 0.0)
    };
    let Some(p) = near else { return [a, b, c] };
    let fit = |x: f64, p: f64| x + ((p - x) / (2.0 * PI)).round() * 2.0 * PI;
    let mut best = [a, b, c];
    let mut bd = f64::MAX;
    for cand in [[a, b, c], [a + PI, PI - b, c + PI]] {
        let f = [fit(cand[0], p[0]), fit(cand[1], p[1]), fit(cand[2], p[2])];
        let d = (0..3).map(|k| (f[k] - p[k]).abs()).sum::<f64>();
        if d < bd {
            bd = d;
            best = f;
        }
    }
    best
}

/// Rotation angle of `q` in degrees (atan2 of the skew part and the trace).
pub fn angle_deg(q: &M3) -> f64 {
    let s = [(q[1][2] - q[2][1]) / 2.0, (q[2][0] - q[0][2]) / 2.0, (q[0][1] - q[1][0]) / 2.0];
    let sn = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt();
    let c = (q[0][0] + q[1][1] + q[2][2] - 1.0) / 2.0;
    sn.atan2(c).to_degrees()
}

/// Distance between points.
pub fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn euler_round_trip_and_continuity() {
        for &(a, b, c) in &[(0.3, -0.2, 1.1), (-2.9, 1.2, 3.0), (0.0, std::f64::consts::FRAC_PI_2, 0.4), (1.0, -1.57, -2.0)] {
            let m = rot_xyz(a, b, c);
            let e = euler_xyz(&m, None);
            let m2 = rot_xyz(e[0], e[1], e[2]);
            assert!(angle_deg(&m3(&m, &tr(&m2))) < 1e-6, "{a} {b} {c} -> {e:?}");
        }
        // Continuity across ±180.
        let e1 = euler_xyz(&rot_xyz(3.1, 0.1, 0.0), None);
        let e2 = euler_xyz(&rot_xyz(3.2, 0.1, 0.0), Some(e1));
        assert!((e2[0] - 3.2).abs() < 1e-9, "{e2:?}");
    }
}
