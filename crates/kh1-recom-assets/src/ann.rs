//! Re:CoM's `.ANN` motion pack and its clip format: reader, writer and sampler.
//!
//! Layout (Checked (decomp + files); loader `FUN_14029F3A0`, clip parser `FUN_14027ABC0`,
//! sampler `FUN_14027BEC0`, curve evaluator `FUN_14027B320` + cursor `FUN_14027C1D0`):
//!
//! - container: `u32 count`, `count × {u32 size; char name[16]}`, then the non-empty clips back
//!   to back in slot order (`size` bytes each; size 0 = empty slot). Clip index = slot number.
//!   Re:CoM's own file ends with 0xCD padding to a 16-byte boundary (not read by the loader).
//! - clip: `+0x00 char name[16]`, `+0x10 s16 bones`, `+0x12 s16 frames`, `+0x14 s16 loop start`
//!   (1-based, clamped to frames; 0 = plays once), `+0x16 u8 flags` (bit 1 = scale channels;
//!   bits 2-4 = d, fps = 60/(d+1)), `+0x17 u8` has-blend (Re:CoM writes 2 in every clip): if
//!   non-zero, `u16 blend` (60 Hz frames) at `+0x18` and the data at `+0x1C`, else blend 8 and
//!   data at `+0x18`.
//! - data: constant mask (`ceil(N/8)` rounded up to 4 bytes; bit set = channel constant;
//!   N = bones × channels), `f32 const[N]` (rotations in degrees), `i32 size[N]` (byte size of
//!   each animated channel's curve block; Re:CoM writes 0 for constants), then one curve block
//!   per animated channel: `i32 n; f32 base; f32 range; idx[n]` (u8 padded to even when
//!   frames < 256, else u16) `; u16 q[n]; bits[ceil(n/8) rounded up to even]` (bit k set =
//!   segment k linear). Value = `q · range / 65535 + base`.
//! - channel order per bone: tx ty tz rx ry rz [sx sy sz]; Euler XYZ in degrees, rotation
//!   `Rx·Ry·Rz` (row vectors) = quaternion `qz·qy·qx` (`FUN_140293F60`).
//!
//! Re:CoM's files carry bytes the game never reads (the const slot of an animated channel, the
//! u8 index pad byte, the linear bits past the last segment, the clip's 4 trailing bytes, the
//! file's tail). They are kept here so that decode → encode is byte-identical.

use crate::{bad, Result};

/// Bytes of one slot table entry.
const SLOT_SIZE: usize = 20;

/// One `.ANN` file.
#[derive(Clone, Debug, PartialEq)]
pub struct Ann {
    /// Every slot, in index order.
    pub slots: Vec<Slot>,
    /// Bytes after the last clip (Re:CoM: 0xCD padding to 16 bytes).
    pub tail: Vec<u8>,
}

/// One slot of the table.
#[derive(Clone, Debug, PartialEq)]
pub struct Slot {
    /// The 16 name bytes of the table entry (zero padded).
    pub name: [u8; 16],
    /// The clip, or `None` for an empty slot (size 0).
    pub clip: Option<Clip>,
}

impl Slot {
    /// The slot name as text.
    pub fn name_str(&self) -> String {
        cstr(&self.name)
    }
}

/// One clip (file form).
#[derive(Clone, Debug, PartialEq)]
pub struct Clip {
    /// `+0x00` name bytes.
    pub name: [u8; 16],
    /// `+0x10` bone count.
    pub bones: i16,
    /// `+0x12` frame count.
    pub frames: i16,
    /// `+0x14` loop start as stored (the parser clamps it to `frames`).
    pub loop_start: i16,
    /// `+0x16` flags.
    pub flags: u8,
    /// `+0x17` has-blend byte (Re:CoM writes 2).
    pub has_blend: u8,
    /// `+0x18` default blend in 60 Hz frames (only stored when `has_blend != 0`).
    pub blend: u16,
    /// `+0x1A..+0x1C` (only stored when `has_blend != 0`).
    pub blend_pad: [u8; 2],
    /// Mask bits past the last channel, as stored (zero in Re:CoM's files).
    pub mask_pad: Vec<u8>,
    /// Channels, `bones × channels_per_bone`, bone-major.
    pub channels: Vec<Channel>,
    /// Bytes after the last curve block (Re:CoM: 4 zero bytes).
    pub trailer: Vec<u8>,
}

/// One channel.
#[derive(Clone, Debug, PartialEq)]
pub enum Channel {
    /// A constant (file units: metres, degrees, scale).
    Const {
        /// The f32 bits.
        bits: u32,
        /// The size field stored for it (0 in Re:CoM's files).
        size_field: i32,
    },
    /// A curve.
    Curve(Curve),
}

impl Channel {
    /// A constant from a value.
    pub fn constant(v: f32) -> Channel {
        Channel::Const { bits: v.to_bits(), size_field: 0 }
    }
}

/// An animated channel's curve block.
#[derive(Clone, Debug, PartialEq)]
pub struct Curve {
    /// The f32 in the const table for this channel (not read by the game).
    pub const_slot: u32,
    /// Key frames.
    pub keys: Vec<u16>,
    /// Base (file units).
    pub base: f32,
    /// Range (file units).
    pub range: f32,
    /// Quantised values.
    pub q: Vec<u16>,
    /// Segment bits (bit k set = segment k linear), full stored length.
    pub bits: Vec<u8>,
    /// The u8 key array's pad byte (odd key counts with u8 keys).
    pub idx_pad: u8,
    /// Bytes after the block's content within its size field.
    pub block_pad: Vec<u8>,
}

impl Curve {
    /// Key values in file units (`q · range / 65535 + base`, f64).
    pub fn values(&self) -> Vec<f64> {
        self.q.iter().map(|&x| x as f64 * self.range as f64 * (1.0 / 65535.0) + self.base as f64).collect()
    }

    fn linear_seg(&self, k: usize) -> bool {
        self.bits.get(k >> 3).is_some_and(|b| (b >> (k & 7)) & 1 != 0)
    }

    /// The value at clip frame `t` (`FUN_14027B320` + `FUN_14027C1D0`, stateless; port of
    /// `recom_ann.py`). Re:CoM only ever evaluates whole frames (`FUN_14027BEC0`).
    pub fn eval(&self, t: f64, looped: bool) -> f64 {
        let v = self.values();
        let n = v.len();
        let ti = |k: usize| self.keys[k] as f64;
        match n {
            0 => return 0.0,
            1 => return v[0],
            2 => return (v[1] - v[0]) * (t - ti(0)) / (ti(1) - ti(0)) + v[0],
            _ => {}
        }
        if let Some(k) = (0..n).find(|&k| ti(k) == t) {
            return v[k];
        }
        if t > ti(n - 1) {
            return if looped { v[n - 1] + (v[0] - v[n - 1]) * (t - ti(n - 1)) } else { v[n - 1] };
        }
        let Some(e) = (0..n).find(|&k| ti(k) >= t) else { return v[n - 1] };
        if e == 0 {
            return v[0];
        }
        let s = e - 1;
        let (t0, t1, v0, v1) = (ti(s), ti(e), v[s], v[e]);
        let u = (t - t0) / (t1 - t0);
        if self.linear_seg(s) {
            return v0 + (v1 - v0) * u;
        }
        // FUN_14027C0E0: neighbour slopes, used per unit of u (not scaled by the gap).
        let m0 = if s == 0 { (v1 - v0) / (t1 - t0) } else { (v0 - v[s - 1]) / (t0 - ti(s - 1)) };
        let m1 = if s == n - 2 || e + 1 >= n { (v1 - v0) / (t1 - t0) } else { (v[e + 1] - v1) / (ti(e + 1) - t1) };
        let (u2, u3) = (u * u, u * u * u);
        (2.0 * u3 - 3.0 * u2 + 1.0) * v0 + (3.0 * u2 - 2.0 * u3) * v1 + (u3 - 2.0 * u2 + u) * m0 + (u3 - u2) * m1
    }
}

/// A bone's sampled local transform (Re:CoM units: metres, radians).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoneSample {
    /// Translation.
    pub t: [f64; 3],
    /// Euler XYZ, radians (`Rx·Ry·Rz`, row vectors).
    pub r: [f64; 3],
    /// Scale (1 when the clip has no scale channels).
    pub s: [f64; 3],
}

impl Clip {
    /// Channels per bone (9 with scale, else 6).
    pub fn chans(&self) -> usize {
        if self.flags & 2 != 0 {
            9
        } else {
            6
        }
    }

    /// Clip frames per second (`60 / (d + 1)`).
    pub fn fps(&self) -> u32 {
        60 / (((self.flags as u32 >> 2) & 7) + 1)
    }

    /// The loop start as the parser keeps it (clamped to frames).
    pub fn loop_start_clamped(&self) -> i16 {
        self.loop_start.min(self.frames)
    }

    /// Whether the clip loops.
    pub fn looped(&self) -> bool {
        self.loop_start_clamped() >= 1
    }

    /// Default blend in 60 Hz frames (8 when no field is stored).
    pub fn default_blend(&self) -> u16 {
        if self.has_blend != 0 {
            self.blend
        } else {
            8
        }
    }

    /// The name as text.
    pub fn name_str(&self) -> String {
        cstr(&self.name)
    }

    /// Value of channel `c` of `bone` at clip frame `t`, in file units (degrees for
    /// rotations).
    pub fn value(&self, bone: usize, c: usize, t: f64) -> f64 {
        match &self.channels[bone * self.chans() + c] {
            Channel::Const { bits, .. } => f32::from_bits(*bits) as f64,
            Channel::Curve(cv) => cv.eval(t, self.looped()),
        }
    }

    /// Whether channel `c` of `bone` is constant or has a key at frame `f`.
    pub fn keyed(&self, bone: usize, c: usize, f: usize) -> bool {
        match &self.channels[bone * self.chans() + c] {
            Channel::Const { .. } => true,
            Channel::Curve(cv) => cv.keys.contains(&(f as u16)),
        }
    }

    /// A bone's local transform at clip frame `t` (metres, radians).
    pub fn sample(&self, bone: usize, t: f64) -> BoneSample {
        let v = |c| self.value(bone, c, t);
        let s = if self.chans() == 9 { [v(6), v(7), v(8)] } else { [1.0; 3] };
        BoneSample { t: [v(0), v(1), v(2)], r: [v(3).to_radians(), v(4).to_radians(), v(5).to_radians()], s }
    }

    /// Parses one clip blob.
    pub fn parse(b: &[u8]) -> Result<Clip> {
        let r = Rd(b);
        let name: [u8; 16] = r.arr(0)?;
        let bones = r.i16(0x10)?;
        let frames = r.i16(0x12)?;
        let loop_start = r.i16(0x14)?;
        let flags = r.u8(0x16)?;
        let has_blend = r.u8(0x17)?;
        let (blend, blend_pad, p) = if has_blend != 0 { (r.u16(0x18)?, r.arr::<2>(0x1A)?, 0x1C) } else { (8, [0; 2], 0x18) };
        if bones <= 0 || frames <= 0 {
            return bad(format!("clip {}: bones {bones} frames {frames}", cstr(&name)));
        }
        let chans = if flags & 2 != 0 { 9 } else { 6 };
        let nn = bones as usize * chans;
        let msz = mask_size(nn);
        let mask = r.slice(p, msz)?;
        let mut mask_pad = vec![0u8; msz];
        for k in nn..msz * 8 {
            mask_pad[k >> 3] |= mask[k >> 3] & (1 << (k & 7));
        }
        let cst = p + msz;
        let sizes = cst + 4 * nn;
        let mut q = cst + 8 * nn;
        let u16_keys = frames >= 0x100;
        let mut channels = Vec::with_capacity(nn);
        for i in 0..nn {
            let size_field = r.i32(sizes + 4 * i)?;
            let slot = r.u32(cst + 4 * i)?;
            if (mask[i >> 3] >> (i & 7)) & 1 != 0 {
                channels.push(Channel::Const { bits: slot, size_field });
                continue;
            }
            let n = r.i32(q)?;
            if n <= 0 || size_field < 0 {
                return bad(format!("clip {}: channel {i}: {n} keys, size {size_field}", cstr(&name)));
            }
            let n = n as usize;
            let base = r.f32(q + 4)?;
            let range = r.f32(q + 8)?;
            let mut o = q + 12;
            let mut idx_pad = 0;
            let keys: Vec<u16> = if !u16_keys {
                let k = r.slice(o, n)?.iter().map(|&x| x as u16).collect();
                if n % 2 == 1 {
                    idx_pad = r.u8(o + n)?;
                }
                o += (n + 1) & !1;
                k
            } else {
                let k = (0..n).map(|j| r.u16(o + 2 * j)).collect::<Result<_>>()?;
                o += 2 * n;
                k
            };
            let qv = (0..n).map(|j| r.u16(o + 2 * j)).collect::<Result<Vec<_>>>()?;
            o += 2 * n;
            let bl = bits_size(n);
            let bits = r.slice(o, bl)?.to_vec();
            o += bl;
            let end = q + size_field as usize;
            if end < o {
                return bad(format!("clip {}: channel {i}: block size {size_field} < content {}", cstr(&name), o - q));
            }
            let block_pad = r.slice(o, end - o)?.to_vec();
            channels.push(Channel::Curve(Curve { const_slot: slot, keys, base, range, q: qv, bits, idx_pad, block_pad }));
            q = end;
        }
        if q > b.len() {
            return bad(format!("clip {}: data runs past the blob", cstr(&name)));
        }
        Ok(Clip { name, bones, frames, loop_start, flags, has_blend, blend, blend_pad, mask_pad, channels, trailer: b[q..].to_vec() })
    }

    /// Encodes the clip blob.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut o = Vec::new();
        o.extend_from_slice(&self.name);
        o.extend_from_slice(&self.bones.to_le_bytes());
        o.extend_from_slice(&self.frames.to_le_bytes());
        o.extend_from_slice(&self.loop_start.to_le_bytes());
        o.push(self.flags);
        o.push(self.has_blend);
        if self.has_blend != 0 {
            o.extend_from_slice(&self.blend.to_le_bytes());
            o.extend_from_slice(&self.blend_pad);
        }
        let nn = self.channels.len();
        let msz = mask_size(nn);
        let mut mask = vec![0u8; msz];
        for (i, c) in self.channels.iter().enumerate() {
            if matches!(c, Channel::Const { .. }) {
                mask[i >> 3] |= 1 << (i & 7);
            }
        }
        for (m, p) in mask.iter_mut().zip(&self.mask_pad) {
            *m |= p;
        }
        o.extend_from_slice(&mask);
        let u16_keys = self.frames >= 0x100;
        let mut blocks: Vec<Vec<u8>> = Vec::new();
        for c in &self.channels {
            match c {
                Channel::Const { bits, .. } => o.extend_from_slice(&bits.to_le_bytes()),
                Channel::Curve(cv) => o.extend_from_slice(&cv.const_slot.to_le_bytes()),
            }
        }
        for c in &self.channels {
            match c {
                Channel::Const { size_field, .. } => o.extend_from_slice(&size_field.to_le_bytes()),
                Channel::Curve(cv) => {
                    let blk = cv.block_bytes(u16_keys);
                    o.extend_from_slice(&(blk.len() as i32).to_le_bytes());
                    blocks.push(blk);
                }
            }
        }
        for blk in blocks {
            o.extend_from_slice(&blk);
        }
        o.extend_from_slice(&self.trailer);
        o
    }
}

impl Curve {
    fn block_bytes(&self, u16_keys: bool) -> Vec<u8> {
        let n = self.keys.len();
        let mut o = Vec::with_capacity(16 + 4 * n);
        o.extend_from_slice(&(n as i32).to_le_bytes());
        o.extend_from_slice(&self.base.to_le_bytes());
        o.extend_from_slice(&self.range.to_le_bytes());
        if u16_keys {
            for &k in &self.keys {
                o.extend_from_slice(&k.to_le_bytes());
            }
        } else {
            o.extend(self.keys.iter().map(|&k| k as u8));
            if n % 2 == 1 {
                o.push(self.idx_pad);
            }
        }
        for &v in &self.q {
            o.extend_from_slice(&v.to_le_bytes());
        }
        let bl = bits_size(n);
        let mut bits = self.bits.clone();
        bits.resize(bl, 0xFF);
        o.extend_from_slice(&bits);
        o.extend_from_slice(&self.block_pad);
        o
    }
}

impl Ann {
    /// Parses a whole `.ANN` file.
    pub fn parse(d: &[u8]) -> Result<Ann> {
        let r = Rd(d);
        let n = r.u32(0)? as usize;
        if n > 0x10000 {
            return bad(format!("ann: implausible slot count {n}"));
        }
        let mut p = 4 + SLOT_SIZE * n;
        let mut slots = Vec::with_capacity(n);
        for i in 0..n {
            let size = r.u32(4 + SLOT_SIZE * i)? as usize;
            let name: [u8; 16] = r.arr(8 + SLOT_SIZE * i)?;
            let clip = if size == 0 {
                None
            } else {
                let c = Clip::parse(r.slice(p, size)?).map_err(|e| crate::Error::Format(format!("slot {i}: {e}")))?;
                p += size;
                Some(c)
            };
            slots.push(Slot { name, clip });
        }
        Ok(Ann { slots, tail: d.get(p..).unwrap_or(&[]).to_vec() })
    }

    /// Encodes the file.
    pub fn to_bytes(&self) -> Vec<u8> {
        let blobs: Vec<Vec<u8>> = self.slots.iter().map(|s| s.clip.as_ref().map_or(Vec::new(), Clip::to_bytes)).collect();
        let mut o = Vec::with_capacity(4 + SLOT_SIZE * self.slots.len() + blobs.iter().map(Vec::len).sum::<usize>() + 16);
        o.extend_from_slice(&(self.slots.len() as u32).to_le_bytes());
        for (s, b) in self.slots.iter().zip(&blobs) {
            o.extend_from_slice(&(b.len() as u32).to_le_bytes());
            o.extend_from_slice(&s.name);
        }
        for b in blobs {
            o.extend_from_slice(&b);
        }
        o.extend_from_slice(&self.tail);
        o
    }

    /// Re-pads the tail like Re:CoM's own file: 0xCD up to a 16-byte boundary.
    pub fn pad_tail_like_recom(&mut self) {
        self.tail.clear();
        let len = self.to_bytes().len();
        self.tail = vec![0xCD; (16 - len % 16) % 16];
    }

    /// The slot whose name is `name`, by Re:CoM's rule (`FUN_1401A47E0`: the 3 trailing digits
    /// give the index, checked against the name, else a linear search).
    pub fn find(&self, name: &str) -> Option<usize> {
        let b = name.as_bytes();
        let names = |i: usize| self.slots.get(i).filter(|s| s.clip.is_some()).map(|s| s.name_str());
        if b.len() >= 3 && b[b.len() - 1].is_ascii_digit() {
            let d = |k: usize| b[b.len() - k] as i32;
            let idx = d(1) + (d(2) + d(3) * 10) * 10 - 0x14D0;
            if idx >= 0 && names(idx as usize).as_deref() == Some(name) {
                return Some(idx as usize);
            }
        }
        (0..self.slots.len()).find(|&i| names(i).as_deref() == Some(name))
    }
}

fn mask_size(nn: usize) -> usize {
    (((nn + 7) >> 3) + 3) & !3
}

fn bits_size(n: usize) -> usize {
    (((n + 7) >> 3) + 1) & !1
}

/// A NUL-terminated name.
pub(crate) fn cstr(b: &[u8]) -> String {
    let e = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..e]).into_owned()
}

/// A 16-byte zero-padded name.
pub fn name16(s: &str) -> [u8; 16] {
    let mut n = [0u8; 16];
    let b = s.as_bytes();
    let k = b.len().min(15);
    n[..k].copy_from_slice(&b[..k]);
    n
}

/// Little-endian bounds-checked reads.
pub(crate) struct Rd<'a>(pub &'a [u8]);

impl<'a> Rd<'a> {
    pub fn slice(&self, at: usize, n: usize) -> Result<&'a [u8]> {
        match at.checked_add(n) {
            Some(e) if e <= self.0.len() => Ok(&self.0[at..e]),
            _ => bad(format!("read of {n} bytes at {at:#x} past the end ({:#x})", self.0.len())),
        }
    }
    pub fn arr<const N: usize>(&self, at: usize) -> Result<[u8; N]> {
        Ok(self.slice(at, N)?.try_into().unwrap())
    }
    pub fn u8(&self, at: usize) -> Result<u8> {
        Ok(self.slice(at, 1)?[0])
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
    pub fn f32(&self, at: usize) -> Result<f32> {
        Ok(f32::from_le_bytes(self.arr(at)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny(frames: i16, keys: Vec<u16>) -> Clip {
        let n = keys.len();
        let cv = Curve { const_slot: 0, keys, base: -1.0, range: 2.0, q: (0..n as u16).map(|k| k * 1000).collect(), bits: vec![0xFF; bits_size(n)], idx_pad: 0, block_pad: vec![] };
        let mut channels = vec![Channel::constant(0.0); 6];
        channels[4] = Channel::Curve(cv);
        Clip { name: name16("p001_200"), bones: 1, frames, loop_start: 0, flags: 0, has_blend: 2, blend: 8, blend_pad: [0; 2], mask_pad: vec![0; 4], channels, trailer: vec![0; 4] }
    }

    #[test]
    fn round_trip_u8_and_u16_keys() {
        for (frames, keys) in [(10i16, vec![0u16, 4, 9]), (300, vec![0, 100, 299])] {
            let c = tiny(frames, keys);
            let b = c.to_bytes();
            let d = Clip::parse(&b).unwrap();
            assert_eq!(d, c);
            assert_eq!(d.to_bytes(), b);
        }
        let mut a = Ann { slots: vec![Slot { name: [0; 16], clip: None }, Slot { name: name16("p001_200"), clip: Some(tiny(10, vec![0, 9])) }], tail: vec![] };
        a.pad_tail_like_recom();
        let b = a.to_bytes();
        assert_eq!(b.len() % 16, 0);
        let p = Ann::parse(&b).unwrap();
        assert_eq!(p, a);
        assert_eq!(p.find("p001_200"), Some(1));
        assert_eq!(p.find("p001_000"), None);
    }

    #[test]
    fn curve_eval_rules() {
        let c = tiny(10, vec![0, 4, 9]);
        let Channel::Curve(cv) = &c.channels[4] else { unreachable!() };
        let v = cv.values();
        assert_eq!(cv.eval(4.0, false), v[1]);
        assert!((cv.eval(2.0, false) - (v[0] + v[1]) / 2.0).abs() < 1e-12);
        assert_eq!(cv.eval(12.0, false), v[2]);
    }
}
