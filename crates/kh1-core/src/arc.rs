//! KH1's air arc (actor +0x3B0..+0x3BC), used by jumps, hops, launches and falls. Positions are
//! y down; the arc moves the body to an absolute height each frame, so the step is
//! "target height − current height" (`0x1402AF6A0`). Checked (decomp).

use std::f32::consts::PI;

/// t (+0x3B0), y0 (+0x3B4), T (+0x3B8), H (+0x3BC, negative = up).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Arc {
    pub t: f32,
    pub y0: f32,
    pub rise_time: f32,
    pub rise: f32,
}

impl Arc {
    /// `0x1402AF960(h)`: rise `h` units over `T = sqrt(2|h| / g)`, from height `y`.
    pub fn hop(h: f32, y: f32, g: f32) -> Arc {
        let t = ((h.abs() + h.abs()) / g).abs().sqrt();
        Arc { t: 0.0, y0: y, rise_time: t, rise: -h }
    }

    /// `0x1402AF8F0(T)`: a rise of T frames (H = −g·T²/2) from height `y`. T = 0 falls at once.
    pub fn timed(rise_time: f32, y: f32, g: f32) -> Arc {
        Arc { t: 0.0, y0: y, rise_time, rise: rise_time * rise_time * -g * 0.5 }
    }

    pub fn rising(&self) -> bool {
        self.t < self.rise_time
    }

    /// The arc's height at time `t`.
    pub fn y_at(&self, t: f32, g: f32) -> f32 {
        if self.rise_time <= t {
            (t - self.rise_time) * g * (t - self.rise_time) * 0.5 + self.rise + self.y0
        } else {
            let s = ((t / self.rise_time) * PI * 0.5).sin();
            (s * PI * 0.5).sin() * self.rise + self.y0
        }
    }

    /// Advances by `step` and returns the vertical step from `y_now`: a double-eased sine rise,
    /// then the parabola, the fall capped at `fall_cap × step` per frame.
    pub fn advance(&mut self, y_now: f32, step: f32, g: f32, fall_cap: f32) -> f32 {
        self.t += step;
        let dy = self.y_at(self.t, g) - y_now;
        if self.rise_time <= self.t && step * fall_cap <= dy {
            step * fall_cap
        } else {
            dy
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::GRAVITY;

    #[test]
    fn jump_190_peaks_at_28() {
        let a = Arc::hop(190.0, 0.0, GRAVITY);
        assert!((a.rise_time - 28.0).abs() < 1e-3);
        assert!((a.y_at(28.0, GRAVITY) + 190.0).abs() < 1e-3);
        let c = Arc::timed(5.0, 0.0, GRAVITY);
        assert!((c.rise + 6.058_673).abs() < 1e-4);
    }
}
