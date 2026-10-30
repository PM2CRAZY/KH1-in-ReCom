//! KH1's collision shapes and overlap test (`0x1402B5EF0`), and the Keyblade's hit volume: a
//! chain of spheres along the blade plus a chain swept from last frame's blade point
//! (`0x1402B3800` / `0x1402B3D50`). Checked (decomp). Coordinates are KH1's (y down).

use crate::math::Vec3;

/// Shape types as KH1 numbers them in the shape records.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShapeKind {
    /// Type 1: an ellipsoid (radius across, `half_height` up/down). Attack spheres are type 1 with
    /// half height = radius.
    Ellipsoid = 1,
    /// Type 2: an upright column.
    Column = 2,
    /// Type 3: an upright box (half sizes radius × half height × radius).
    Box = 3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shape {
    pub kind: ShapeKind,
    pub centre: Vec3,
    pub radius: f32,
    pub half_height: f32,
}

impl Shape {
    pub fn sphere(centre: Vec3, radius: f32) -> Shape {
        Shape { kind: ShapeKind::Ellipsoid, centre, radius, half_height: radius }
    }

    pub fn new(kind: ShapeKind, centre: Vec3, radius: f32, half_height: f32) -> Shape {
        Shape { kind, centre, radius, half_height }
    }

    /// `0x1402B5EF0`. Symmetric; the lower type number goes first.
    pub fn overlaps(&self, other: &Shape) -> bool {
        let (a, b) = if other.kind < self.kind { (other, self) } else { (self, other) };
        use ShapeKind::*;
        match (a.kind, b.kind) {
            (Ellipsoid, Ellipsoid) => {
                let d = b.centre - a.centre;
                let r = 1.0 / (b.radius + a.radius);
                let h = 1.0 / (b.half_height + a.half_height);
                let s = Vec3::new(d.x * r, d.y * h, d.z * r);
                s.dot(s) < 1.0
            }
            (Ellipsoid, Column) => {
                let d = b.centre - a.centre;
                let dy = d.y.abs();
                if a.half_height + b.half_height <= dy {
                    return false;
                }
                let r = if dy <= b.half_height {
                    a.radius + b.radius
                } else {
                    let t = (dy - b.half_height) / a.half_height;
                    (1.0 - t * t).abs().sqrt() * a.radius + b.radius
                };
                d.x * d.x + d.z * d.z < r * r
            }
            (Column, Column) => {
                let d = b.centre - a.centre;
                if b.half_height + a.half_height <= d.y.abs() {
                    return false;
                }
                let r = b.radius + a.radius;
                d.x * d.x + d.z * d.z < r * r
            }
            (Ellipsoid, Box) => {
                let d = a.centre - b.centre;
                let dy = d.y.abs();
                if a.half_height + b.half_height <= dy {
                    return false;
                }
                let mut er = a.radius;
                if b.half_height < dy {
                    let t = (dy - b.half_height) / a.half_height;
                    er *= (1.0 - t * t).abs().sqrt();
                }
                rounded_square(d, b.radius, er)
            }
            (Column, Box) => {
                let d = a.centre - b.centre;
                if b.half_height + a.half_height <= d.y.abs() {
                    return false;
                }
                rounded_square(d, b.radius, a.radius)
            }
            (Box, Box) => {
                let d = a.centre - b.centre;
                let r = b.radius + a.radius;
                let h = b.half_height + a.half_height;
                d.x <= r && -r <= d.x && d.z <= r && -r <= d.z && d.y <= h && -h <= d.y
            }
            _ => false,
        }
    }
}

/// A round shape of radius `r` against a square of half size `half` on the ground plane.
fn rounded_square(d: Vec3, half: f32, r: f32) -> bool {
    let (ax, az) = (d.x.abs(), d.z.abs());
    let (lo, hi) = if ax < az { (ax, az) } else { (az, ax) };
    if half + r <= lo {
        return false;
    }
    if lo <= half {
        hi < half + r
    } else {
        let u = (lo - half) / r;
        hi < half + (1.0 - u * u).abs().sqrt() * r
    }
}

/// `0x1402B3D50`: spheres from `from` (radius `r_from`) toward `to` (radius `r_to`), spaced by
/// the mean radius (doubled when `wide`), until the remaining distance is used up. The first is
/// at `from`; the radius steps linearly.
pub fn sphere_chain(from: Vec3, r_from: f32, to: Vec3, r_to: f32, wide: bool) -> Vec<Shape> {
    let mut spacing = (r_from + r_to) * 0.5;
    if wide {
        spacing += spacing;
    }
    let mut out = Vec::new();
    if spacing == 0.0 {
        return out;
    }
    let (dir, mut left) = (to - from).normalize_len();
    let step = dir * spacing;
    let dr = if left == 0.0 { 0.0 } else { (r_to - r_from) * spacing / left };
    let mut p = from;
    let mut r = r_from;
    loop {
        out.push(Shape::sphere(p, r));
        p += step;
        left -= spacing;
        r += dr;
        if left <= 0.0 || out.len() >= 0xFE {
            break;
        }
    }
    out
}

/// The live volume of one Keyblade swing across frames.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BladeVolume {
    /// Last tested frame's point A (attack object +0x70), if it was tested.
    prev_a: Option<Vec3>,
}

impl BladeVolume {
    pub fn new() -> BladeVolume {
        BladeVolume::default()
    }

    /// This frame's spheres. `a` / `b` are the blade points at the record's fractions (A = the
    /// object's position), `radius` the record radius + scale × weapon radius. `sweep` = the
    /// record builds a swept chain. On the first tested frame the sweep starts from
    /// `owner_centre` (bone 0x8003: the owner's body shape centre), as the exe seeds it.
    /// Call [`BladeVolume::commit`] after testing, or [`BladeVolume::skip`] on frames not tested.
    pub fn shapes(&self, a: Vec3, b: Vec3, radius: f32, sweep: bool, owner_centre: Vec3) -> Vec<Shape> {
        let mut out = Vec::new();
        if sweep {
            out.extend(sphere_chain(a, radius, self.prev_a.unwrap_or(owner_centre), radius, false));
        }
        out.extend(sphere_chain(a, radius, b, radius, false));
        out
    }

    /// The frame was tested: remember A (`0x1402B4460`).
    pub fn commit(&mut self, a: Vec3) {
        self.prev_a = Some(a);
    }

    /// The frame was not tested: the next tested frame seeds again.
    pub fn skip(&mut self) {
        self.prev_a = None;
    }
}

/// `0x14029EB20` / `0x1401D6320` for the weapon bone: a point at bone parameter `t` on the
/// segment from `parent` (t = 0) to `bone` (t = 1). Assumption: `0x1401D6320` lerps the bone and
/// its parent as `docs` read it (`parent + (bone − parent) × t`); the posing crate owns the exact
/// skeleton maths.
pub fn blade_point(parent: Vec3, bone: Vec3, t: f32) -> Vec3 {
    parent.lerp(bone, t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ellipsoids_and_columns() {
        let a = Shape::sphere(Vec3::ZERO, 10.0);
        assert!(a.overlaps(&Shape::sphere(Vec3::new(19.9, 0.0, 0.0), 10.0)));
        assert!(!a.overlaps(&Shape::sphere(Vec3::new(20.0, 0.0, 0.0), 10.0)));
        let c = Shape::new(ShapeKind::Column, Vec3::ZERO, 10.0, 50.0);
        assert!(Shape::sphere(Vec3::new(29.0, -40.0, 0.0), 20.0).overlaps(&c));
        // 10 past the top: 20·sqrt(1 − 0.25) + 10 ≈ 27.3
        assert!(Shape::sphere(Vec3::new(27.0, -60.0, 0.0), 20.0).overlaps(&c));
        assert!(!c.overlaps(&Shape::sphere(Vec3::new(28.0, -60.0, 0.0), 20.0)));
        let bx = Shape::new(ShapeKind::Box, Vec3::ZERO, 10.0, 10.0);
        assert!(Shape::sphere(Vec3::new(19.0, 0.0, 0.0), 10.0).overlaps(&bx));
        assert!(!Shape::sphere(Vec3::new(17.5, 0.0, 17.5), 10.0).overlaps(&bx), "the corner is rounded");
    }

    #[test]
    fn chain_spacing() {
        let s = sphere_chain(Vec3::ZERO, 40.0, Vec3::new(0.0, 0.0, 100.0), 40.0, false);
        let z: Vec<f32> = s.iter().map(|s| s.centre.z).collect();
        assert_eq!(z, vec![0.0, 40.0, 80.0]);
        assert_eq!(sphere_chain(Vec3::ZERO, 40.0, Vec3::ZERO, 40.0, false).len(), 1);
        assert!(sphere_chain(Vec3::ZERO, 0.0, Vec3::ZERO, 0.0, false).is_empty());
    }

    #[test]
    fn sweep_seeds_from_owner_then_last_a() {
        let mut v = BladeVolume::new();
        let s = v.shapes(Vec3::new(0.0, 0.0, 100.0), Vec3::new(0.0, 0.0, 100.0), 40.0, true, Vec3::ZERO);
        assert_eq!(s.len(), 3 + 1);
        v.commit(Vec3::new(0.0, 0.0, 100.0));
        let s = v.shapes(Vec3::new(100.0, 0.0, 0.0), Vec3::new(100.0, 0.0, 0.0), 40.0, true, Vec3::ZERO);
        assert_eq!(s.len(), 4 + 1); // 141 units back to the last A
    }
}
