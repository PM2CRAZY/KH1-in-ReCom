//! A tiny 3-vector. KH1 world units (about 1 cm), **y down** (up is −y), as in the game.
//! Yaw is `atan2(x, z)`; forward for a yaw is `(sin yaw, 0, cos yaw)` (`0x140297E10`).

use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 0.0 };

    pub const fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }

    pub fn dot(self, o: Vec3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    pub fn length(self) -> f32 {
        self.dot(self).abs().sqrt()
    }

    /// Length on the ground plane (`0x1402A9260`).
    pub fn horizontal_length(self) -> f32 {
        (self.x * self.x + self.z * self.z).abs().sqrt()
    }

    /// The same vector with y = 0.
    pub fn flat(self) -> Vec3 {
        Vec3::new(self.x, 0.0, self.z)
    }

    /// Unit vector, or zero for a zero vector.
    pub fn normalized(self) -> Vec3 {
        let l = self.length();
        if l > 0.0 {
            self * (1.0 / l)
        } else {
            Vec3::ZERO
        }
    }

    /// `0x1402A93D0`: the unit vector and the length. A zero vector gives (0, 0, 1) and 0.
    pub fn normalize_len(self) -> (Vec3, f32) {
        let l = self.length();
        if l != 0.0 {
            (self * (1.0 / l), l)
        } else {
            (Vec3::new(0.0, 0.0, 1.0), 0.0)
        }
    }

    pub fn is_zero_xz(self) -> bool {
        self.x == 0.0 && self.z == 0.0
    }

    /// Forward vector for a yaw (`0x140297E10`).
    pub fn from_yaw(yaw: f32) -> Vec3 {
        Vec3::new(yaw.sin(), 0.0, yaw.cos())
    }

    /// Yaw of a vector (`0x140297FB0`: `atan2(x, z)`).
    pub fn yaw(self) -> f32 {
        self.x.atan2(self.z)
    }

    /// Rotates a local (x right, z forward) offset by a yaw. Assumption: KH1's actor matrix maps
    /// local +z to `from_yaw(yaw)` and local +x to `(cos yaw, 0, −sin yaw)`.
    pub fn rotate_yaw(self, yaw: f32) -> Vec3 {
        let (s, c) = yaw.sin_cos();
        Vec3::new(self.x * c + self.z * s, self.y, -self.x * s + self.z * c)
    }

    pub fn lerp(self, o: Vec3, t: f32) -> Vec3 {
        self + (o - self) * t
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl AddAssign for Vec3 {
    fn add_assign(&mut self, o: Vec3) {
        *self = *self + o;
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl SubAssign for Vec3 {
    fn sub_assign(&mut self, o: Vec3) {
        *self = *self - o;
    }
}

impl Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, k: f32) -> Vec3 {
        Vec3::new(self.x * k, self.y * k, self.z * k)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}

/// `0x1402A8280`: wraps an angle into [−π, π].
pub fn wrap_angle(mut a: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    let mut n = 0;
    while n < 0x400 {
        if a > PI {
            a -= TAU;
        } else if a < -PI {
            a += TAU;
        } else {
            break;
        }
        n += 1;
    }
    a
}

/// `0x1402A8300`: moves `from` toward `to` by at most `rate × step`.
pub fn turn_toward(from: f32, to: f32, rate: f32, step: f32) -> f32 {
    let d = wrap_angle(to - from);
    wrap_angle(from + clamp_sym(d, rate * step))
}

/// The exe's symmetric clamp: `d` limited to [−m, m] (a NaN `d` gives `m`).
pub fn clamp_sym(d: f32, m: f32) -> f32 {
    if d <= m {
        if -m <= d {
            d
        } else {
            -m
        }
    } else {
        m
    }
}
