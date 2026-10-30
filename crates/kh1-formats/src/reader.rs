//! Tiny bounds-checked little-endian reads.

use crate::{bad, Result};

/// Reads at absolute offsets of a byte slice, failing (not panicking) past the end.
#[derive(Clone, Copy)]
pub(crate) struct Bytes<'a>(pub &'a [u8]);

impl<'a> Bytes<'a> {
    pub fn slice(&self, at: usize, n: usize) -> Result<&'a [u8]> {
        match at.checked_add(n) {
            Some(end) if end <= self.0.len() => Ok(&self.0[at..end]),
            _ => bad(format!("unexpected end of data at offset {at:#x} (+{n:#x}, size {:#x})", self.0.len())),
        }
    }
    pub fn arr<const N: usize>(&self, at: usize) -> Result<[u8; N]> {
        Ok(self.slice(at, N)?.try_into().expect("length checked"))
    }
    pub fn u8(&self, at: usize) -> Result<u8> {
        Ok(self.arr::<1>(at)?[0])
    }
    pub fn i8(&self, at: usize) -> Result<i8> {
        Ok(self.u8(at)? as i8)
    }
    pub fn u16(&self, at: usize) -> Result<u16> {
        Ok(u16::from_le_bytes(self.arr(at)?))
    }
    pub fn i16(&self, at: usize) -> Result<i16> {
        Ok(i16::from_le_bytes(self.arr(at)?))
    }
    pub fn u32(&self, at: usize) -> Result<u32> {
        Ok(u32::from_le_bytes(self.arr(at)?))
    }
    pub fn i32(&self, at: usize) -> Result<i32> {
        Ok(i32::from_le_bytes(self.arr(at)?))
    }
    pub fn i64(&self, at: usize) -> Result<i64> {
        Ok(i64::from_le_bytes(self.arr(at)?))
    }
    pub fn f32(&self, at: usize) -> Result<f32> {
        Ok(f32::from_bits(self.u32(at)?))
    }
    pub fn f32x3(&self, at: usize) -> Result<[f32; 3]> {
        Ok([self.f32(at)?, self.f32(at + 4)?, self.f32(at + 8)?])
    }
    /// Fixed-size, NUL-padded ASCII string.
    pub fn cstr(&self, at: usize, n: usize) -> Result<String> {
        let b = self.slice(at, n)?;
        let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
        Ok(String::from_utf8_lossy(&b[..end]).into_owned())
    }
    /// `base + rel`, checked against overflow and the data size.
    pub fn rel(&self, base: usize, rel: u32) -> Result<usize> {
        match base.checked_add(rel as usize) {
            Some(p) if p <= self.0.len() => Ok(p),
            _ => bad(format!("offset {rel:#x} from {base:#x} is outside the data")),
        }
    }
}
