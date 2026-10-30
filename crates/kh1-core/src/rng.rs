//! KH1's random float (`0x14031BA30`): three CRT `rand()` calls, one byte each,
//! `((r1 & 0xFF) << 16 | (r2 & 0xFF) << 8 | (r3 & 0xFF)) × 2^-24`. Checked (decomp).
//!
//! Assumption: the PC build's `rand()` is the MSVC CRT linear congruential generator
//! (`s = s × 214013 + 2531011; (s >> 16) & 0x7FFF`). The game's seed and the other callers that
//! share the stream are not reproduced, so a seeded run matches KH1 only in distribution.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kh1Rand {
    state: u32,
}

impl Kh1Rand {
    pub fn new(seed: u32) -> Kh1Rand {
        Kh1Rand { state: seed }
    }

    /// One CRT `rand()`: 0..=0x7FFF.
    pub fn rand(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(214_013).wrapping_add(2_531_011);
        (self.state >> 16) & 0x7FFF
    }

    /// `0x14031BA30`: a float in [0, 1).
    pub fn next_f32(&mut self) -> f32 {
        let a = self.rand() & 0xFF;
        let b = self.rand() & 0xFF;
        let c = self.rand() & 0xFF;
        ((a << 16) | (b << 8) | c) as f32 * 5.960_464_5e-8
    }
}

impl Default for Kh1Rand {
    /// Seed 1, the CRT's default seed.
    fn default() -> Self {
        Kh1Rand::new(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crt_sequence_and_range() {
        // MSVC rand() with seed 1 starts 41, 18467, 6334.
        let mut r = Kh1Rand::new(1);
        assert_eq!([r.rand(), r.rand(), r.rand()], [41, 18467, 6334]);
        let mut r = Kh1Rand::new(7);
        for _ in 0..1000 {
            let f = r.next_f32();
            assert!((0.0..1.0).contains(&f));
        }
    }
}
