//! The training arena as the controller's [`World`]: a flat floor at y = 0, one raised platform
//! (a box) for air and jump-up tests, and the dummies' bodies so Sora cannot walk through them.
//! Units are KH1's (about 1 cm, y down). This is the demo's own level geometry, not a KH1 map.

use kh1_core::{MoveResult, Vec3, World};

/// Sora's body radius for walls and dummies. Assumption: KH1's player collision radius was not
/// read for this demo.
pub const SORA_RADIUS: f32 = 30.0;

/// An axis-aligned box standing on the floor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Platform {
    /// Centre on the floor plane (x, z).
    pub x: f32,
    pub z: f32,
    pub half_x: f32,
    pub half_z: f32,
    /// Height above the floor (its top is at y = −height).
    pub height: f32,
}

impl Platform {
    pub fn top(&self) -> f32 {
        -self.height
    }

    /// Inside the footprint grown by `grow`.
    pub fn covers(&self, x: f32, z: f32, grow: f32) -> bool {
        (x - self.x).abs() < self.half_x + grow && (z - self.z).abs() < self.half_z + grow
    }
}

/// The arena's layout.
#[derive(Clone, Debug, PartialEq)]
pub struct Arena {
    pub platform: Platform,
    /// Half the size of the drawn floor.
    pub half_size: f32,
}

impl Default for Arena {
    fn default() -> Self {
        Arena { platform: Platform { x: -550.0, z: 200.0, half_x: 120.0, half_z: 120.0, height: 200.0 }, half_size: 1500.0 }
    }
}

impl Arena {
    /// The floor height under (x, z) for a body at height `y` (the platform top only once the
    /// body is on or above it).
    pub fn floor_under(&self, x: f32, z: f32, y: f32) -> f32 {
        let p = &self.platform;
        if p.covers(x, z, 0.0) && y <= p.top() + 1.0 {
            p.top()
        } else {
            0.0
        }
    }
}

/// A standing body Sora is pushed out of (a dummy's column).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    pub x: f32,
    pub z: f32,
    pub radius: f32,
    /// Vertical extent (y down: `top < bottom`).
    pub top: f32,
    pub bottom: f32,
}

/// The arena with this frame's dummy bodies.
pub struct ArenaWorld<'a> {
    pub arena: &'a Arena,
    pub bodies: Vec<Body>,
}

impl World for ArenaWorld<'_> {
    fn move_body(&mut self, from: Vec3, disp: Vec3) -> MoveResult {
        let p = self.arena.platform;
        let mut to = from + disp;
        // The platform's sides are walls below its top: keep the old x / z.
        let below_top = to.y > p.top() + 1.0;
        if below_top && p.covers(to.x, to.z, SORA_RADIUS) && !p.covers(from.x, from.z, SORA_RADIUS) {
            if !p.covers(from.x, to.z, SORA_RADIUS) {
                to.x = from.x;
            } else if !p.covers(to.x, from.z, SORA_RADIUS) {
                to.z = from.z;
            } else {
                to.x = from.x;
                to.z = from.z;
            }
        }
        // Dummies: push out sideways. Assumption: KH1's actor-vs-actor push is not traced; this
        // only keeps Sora from walking through them.
        for b in &self.bodies {
            if to.y < b.top || to.y - 150.0 > b.bottom {
                continue;
            }
            let (dx, dz) = (to.x - b.x, to.z - b.z);
            let r = b.radius + SORA_RADIUS;
            let d2 = dx * dx + dz * dz;
            if d2 < r * r {
                let d = d2.sqrt();
                let (nx, nz) = if d > 1e-3 { (dx / d, dz / d) } else { (0.0, -1.0) };
                to.x = b.x + nx * r;
                to.z = b.z + nz * r;
            }
        }
        let floor = if p.covers(to.x, to.z, 0.0) && from.y <= p.top() + 1.0 { p.top() } else { 0.0 };
        let grounded = to.y >= floor;
        if grounded {
            to.y = floor;
        }
        MoveResult { pos: to, grounded, height_above_floor: floor - to.y }
    }

    fn wall_ahead(&mut self, pos: Vec3, dir: Vec3, extra: f32) -> bool {
        let p = self.arena.platform;
        let ahead = pos + dir.flat().normalized() * (SORA_RADIUS + extra);
        pos.y > p.top() + 1.0 && p.covers(ahead.x, ahead.z, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floor_platform_and_walls() {
        let a = Arena::default();
        let mut w = ArenaWorld { arena: &a, bodies: vec![] };
        let r = w.move_body(Vec3::new(0.0, -10.0, 0.0), Vec3::new(0.0, 20.0, 0.0));
        assert!(r.grounded && r.pos.y == 0.0);
        // Walking into the platform's side is blocked.
        let start = Vec3::new(-550.0 + 120.0 + SORA_RADIUS + 1.0, 0.0, 200.0);
        let r = w.move_body(start, Vec3::new(-8.0, 0.0, 0.0));
        assert_eq!(r.pos.x, start.x);
        assert!(w.wall_ahead(start, Vec3::new(-1.0, 0.0, 0.0), 20.0));
        // Landing on its top.
        let r = w.move_body(Vec3::new(-550.0, -205.0, 200.0), Vec3::new(0.0, 10.0, 0.0));
        assert!(r.grounded && r.pos.y == -200.0);
        // A dummy pushes Sora out.
        w.bodies.push(Body { x: 0.0, z: 100.0, radius: 40.0, top: -160.0, bottom: 0.0 });
        let r = w.move_body(Vec3::new(0.0, 0.0, 20.0), Vec3::new(0.0, 0.0, 20.0));
        assert!((r.pos.z - 30.0).abs() < 1e-4, "{:?}", r.pos);
    }
}
