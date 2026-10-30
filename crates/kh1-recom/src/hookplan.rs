//! The bytes of an inline hook, planned off-target so they can be tested.
//!
//! A hooked function starts with a `jmp rel32` to a stub near the exe (an absolute jump to our
//! function); a trampoline holds the moved first bytes and jumps back after them. The moved
//! bytes must be whole, position-independent instructions: `sites::HOOKS` lists exactly those.

/// `jmp [rip+0]; dq to` (14 bytes).
pub fn abs_jmp(to: u64) -> [u8; 14] {
    let mut b = [0u8; 14];
    b[0] = 0xFF;
    b[1] = 0x25;
    b[6..14].copy_from_slice(&to.to_le_bytes());
    b
}

/// `jmp rel32` from `from` to `to`, if in range.
pub fn rel_jmp(from: u64, to: u64) -> Option<[u8; 5]> {
    let rel = to as i64 - (from as i64 + 5);
    let rel = i32::try_from(rel).ok()?;
    let mut b = [0u8; 5];
    b[0] = 0xE9;
    b[1..].copy_from_slice(&rel.to_le_bytes());
    Some(b)
}

/// What goes over the first `len` bytes of the target: the jump to the stub, then `nop`s.
pub fn entry_patch(target: u64, len: usize, stub: u64) -> Option<Vec<u8>> {
    if len < 5 {
        return None;
    }
    let mut v = rel_jmp(target, stub)?.to_vec();
    v.resize(len, 0x90);
    Some(v)
}

/// The trampoline: the moved bytes, then an absolute jump back to `target + moved.len()`.
pub fn trampoline(moved: &[u8], target: u64) -> Vec<u8> {
    let mut v = moved.to_vec();
    v.extend_from_slice(&abs_jmp(target + moved.len() as u64));
    v
}

/// Layout of the 64-byte block allocated near each target: trampoline at 0, stub at 32.
pub const BLOCK: usize = 64;
pub const STUB_AT: usize = 32;

/// Candidate base addresses for that block, every 64 KB below `near` (within ±1.75 GB so a
/// rel32 reaches it), nearest first.
pub fn near_candidates(near: u64) -> impl Iterator<Item = u64> {
    let start = (near & !0xFFFF).saturating_sub(0x1000_0000);
    let low = near.saturating_sub(0x7000_0000);
    (0..).map(move |i: u64| start.wrapping_sub(i * 0x10000)).take_while(move |&a| a > low && a <= start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jumps_encode() {
        let j = abs_jmp(0x1122_3344_5566_7788);
        assert_eq!(&j[..6], &[0xFF, 0x25, 0, 0, 0, 0]);
        assert_eq!(u64::from_le_bytes(j[6..].try_into().unwrap()), 0x1122_3344_5566_7788);
        assert_eq!(rel_jmp(0x1000, 0x1005), Some([0xE9, 0, 0, 0, 0]));
        assert_eq!(rel_jmp(0x1000, 0x0F00), Some([0xE9, 0xFB, 0xFE, 0xFF, 0xFF]));
        assert_eq!(rel_jmp(0x1_0000_0000, 0x10), None);
    }

    #[test]
    fn entry_patch_pads_with_nops() {
        let p = entry_patch(0x1_400D_3EB0, 10, 0x1_300D_0020).unwrap();
        assert_eq!(p.len(), 10);
        assert_eq!(p[0], 0xE9);
        let rel = i32::from_le_bytes(p[1..5].try_into().unwrap()) as i64;
        assert_eq!(0x1_400D_3EB0i64 + 5 + rel, 0x1_300D_0020);
        assert!(p[5..].iter().all(|&b| b == 0x90));
        assert_eq!(entry_patch(0, 4, 0), None);
    }

    #[test]
    fn trampoline_returns_after_moved_bytes() {
        let moved = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20];
        let t = trampoline(&moved, 0x1_400D_3EB0);
        assert_eq!(&t[..10], &moved);
        assert_eq!(u64::from_le_bytes(t[16..24].try_into().unwrap()), 0x1_400D_3EBA);
        assert!(t.len() <= STUB_AT && STUB_AT + 14 <= BLOCK);
    }

    #[test]
    fn candidates_stay_in_rel32_range() {
        let near = 0x1_400D_F1B0u64;
        let c: Vec<u64> = near_candidates(near).collect();
        assert!(!c.is_empty());
        assert!(c.iter().all(|&a| a % 0x10000 == 0 && near - a < 0x7000_0000 && a < near));
        assert!(c.windows(2).all(|w| w[0] > w[1]));
        // Every candidate block can be reached both ways.
        for &a in [c[0], *c.last().unwrap()].iter() {
            assert!(rel_jmp(near, a + STUB_AT as u64).is_some());
        }
        assert_eq!(near_candidates(0x1000).count(), 0);
    }
}
