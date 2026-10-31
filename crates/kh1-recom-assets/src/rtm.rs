//! Re:CoM texture packs (`.RTM`, resource type 0xC) and the 8-bit TIM2 pictures in them.
//!
//! - Header: 32-byte entries `u32 data offset | char name[20] ("w_ex010_01.tm2") | u32
//!   0x01020000 | u32 0x80000000 | size`, then one all-zero entry; the data follows, unpadded.
//!   Loader `CRsrcPackT<CRsrcDataDD>` (`0x1402A2150`); each entry becomes its own resource named
//!   by the entry (`0x1402A2A60`, `.tm2` → a texture type through `0x1402A51B0`). Bit 31 of the
//!   size word is the "offset-first" layout (`0x1402A0600` converts the other one). 1395 of the
//!   1421 RTMs Re:CoM's resource tables list use it; 26 enemy packs (e.g. `ENEMY/0300/EN0300.RTM`)
//!   use the other, name-first layout (`name[16] | offset | 0x01020000 | … | size`), which this
//!   reader refuses. Checked (files: the owner's Recom.pkg; PL0001/0002/0011/0012/0101.RTM
//!   rewrite byte for byte; decomp).
//! - TIM2: version 4, one picture, 8-bit indexed (image type 5), 256-colour 32-bit CLUT (type 3)
//!   stored in the PS2's CSM1 order (entries 8–15 and 16–23 of every 32 swapped), alpha 0..0x80,
//!   header 0x30, GsTex0 0, GsTex1 0x260, image before CLUT. Checked (files: all 13 pictures of
//!   those RTMs; the CSM1 order by decoding PL0002's picture against its HD texture: mean error
//!   8 with the swap, 62 without).
//! - The PC shows the HD texture `<name>_tm2.dds` instead when the RTM entry has one (an HD
//!   asset of the RTM, matched by name). Checked (files) / the matching code: Assumption.

use crate::ann::{cstr, Rd};
use crate::{bad, Result};

/// The word after an entry's name in every Re:CoM RTM. Checked (files).
pub const ENTRY_KIND: u32 = 0x0102_0000;

/// A texture pack.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rtm {
    /// The entries, in file order.
    pub entries: Vec<RtmEntry>,
}

/// One entry: a named TIM2.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RtmEntry {
    /// Zero-padded name, e.g. `w_ex010_01.tm2`.
    pub name: [u8; 20],
    /// The word after the name ([`ENTRY_KIND`]).
    pub kind: u32,
    /// The data (a TIM2).
    pub data: Vec<u8>,
}

impl RtmEntry {
    /// A TIM2 entry named `<texture>.tm2`.
    pub fn new(texture: &str, data: Vec<u8>) -> Result<RtmEntry> {
        let n = format!("{texture}.tm2");
        if n.len() > 19 {
            return bad(format!("rtm: name {n} is longer than 19 bytes"));
        }
        let mut name = [0u8; 20];
        name[..n.len()].copy_from_slice(n.as_bytes());
        Ok(RtmEntry { name, kind: ENTRY_KIND, data })
    }
    /// The name as text.
    pub fn name_str(&self) -> String {
        cstr(&self.name)
    }
}

impl Rtm {
    /// Parses a pack (offset-first layout only; data must follow the header in entry order).
    pub fn parse(d: &[u8]) -> Result<Rtm> {
        let r = Rd(d);
        let mut entries = Vec::new();
        let mut i = 0;
        let mut raw = Vec::new();
        while r.u32(32 * i)? != 0 {
            let at = 32 * i;
            let size = r.u32(at + 28)?;
            if size & 0x8000_0000 == 0 {
                return bad("rtm: entry without the offset-first bit (other layout not supported)");
            }
            raw.push((r.u32(at)? as usize, r.arr::<20>(at + 4)?, r.u32(at + 24)?, (size & 0x7FFF_FFFF) as usize));
            i += 1;
            if i > 4096 {
                return bad("rtm: too many entries");
            }
        }
        if r.slice(32 * i, 32)?.iter().any(|&x| x != 0) {
            return bad("rtm: end entry is not zero");
        }
        let mut at = 32 * (i + 1);
        for (off, name, kind, size) in raw {
            if off != at {
                return bad(format!("rtm: data at {off:#x}, expected {at:#x}"));
            }
            entries.push(RtmEntry { name, kind, data: r.slice(off, size)?.to_vec() });
            at += size;
        }
        if at != d.len() {
            return bad(format!("rtm: {} bytes after the last entry", d.len() as isize - at as isize));
        }
        Ok(Rtm { entries })
    }

    /// Writes the pack.
    pub fn write(&self) -> Vec<u8> {
        let hdr = 32 * (self.entries.len() + 1);
        let mut out = vec![0u8; hdr];
        let mut off = hdr;
        for (i, e) in self.entries.iter().enumerate() {
            let at = 32 * i;
            out[at..at + 4].copy_from_slice(&(off as u32).to_le_bytes());
            out[at + 4..at + 24].copy_from_slice(&e.name);
            out[at + 24..at + 28].copy_from_slice(&e.kind.to_le_bytes());
            out[at + 28..at + 32].copy_from_slice(&(0x8000_0000 | e.data.len() as u32).to_le_bytes());
            off += e.data.len();
        }
        for e in &self.entries {
            out.extend_from_slice(&e.data);
        }
        out
    }
}

/// The CSM1 CLUT order: entry `i` of the palette is stored at `csm1(i)` (an involution).
pub fn csm1(i: usize) -> usize {
    (i & !0x18) | ((i & 8) << 1) | ((i & 0x10) >> 1)
}

/// Builds a TIM2 like Square's: one 8-bit picture `w`×`h`, `pixels` palette indices row by
/// row, `palette` 256 RGBA entries with PS2 alpha (0..=0x80).
pub fn tim2_indexed8(w: u16, h: u16, pixels: &[u8], palette: &[[u8; 4]; 256]) -> Result<Vec<u8>> {
    let n = w as usize * h as usize;
    if pixels.len() != n || w == 0 || h == 0 {
        return bad("tim2: pixel count does not match the size");
    }
    let total = 0x30 + n + 1024;
    let mut out = Vec::with_capacity(16 + total);
    out.extend_from_slice(b"TIM2");
    out.extend_from_slice(&[4, 0, 1, 0]);
    out.extend_from_slice(&[0; 8]);
    for v in [total as u32, 1024, n as u32] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.extend_from_slice(&0x30u16.to_le_bytes());
    out.extend_from_slice(&256u16.to_le_bytes());
    out.extend_from_slice(&[0, 1, 3, 5]);
    out.extend_from_slice(&w.to_le_bytes());
    out.extend_from_slice(&h.to_le_bytes());
    out.extend_from_slice(&0u64.to_le_bytes()); // GsTex0
    out.extend_from_slice(&0x260u64.to_le_bytes()); // GsTex1
    out.extend_from_slice(&[0; 8]); // GsRegs, GsTexClut
    out.extend_from_slice(pixels);
    let mut clut = [[0u8; 4]; 256];
    for (i, c) in palette.iter().enumerate() {
        clut[csm1(i)] = *c;
    }
    clut.iter().for_each(|c| out.extend_from_slice(c));
    Ok(out)
}

/// Decodes an 8-bit TIM2 written in Square's layout to `(w, h, RGBA with PS2 alpha)`.
pub fn tim2_decode8(t: &[u8]) -> Result<(usize, usize, Vec<[u8; 4]>)> {
    let r = Rd(t);
    if r.slice(0, 4)? != b"TIM2" || r.u16(6)? != 1 {
        return bad("tim2: not a one-picture TIM2");
    }
    let (clut, img, hdr, cols) = (r.u32(0x14)? as usize, r.u32(0x18)? as usize, r.u16(0x1C)? as usize, r.u16(0x1E)? as usize);
    if r.u8(0x23)? != 5 || r.u8(0x22)? != 3 || cols != 256 || clut != 1024 {
        return bad("tim2: not 8-bit with a 256-colour 32-bit CLUT");
    }
    let (w, h) = (r.u16(0x24)? as usize, r.u16(0x26)? as usize);
    if img != w * h {
        return bad("tim2: image size");
    }
    let px = r.slice(0x10 + hdr, img)?;
    let cl = r.slice(0x10 + hdr + img, clut)?;
    Ok((w, h, px.iter().map(|&i| cl[4 * csm1(i as usize)..4 * csm1(i as usize) + 4].try_into().unwrap()).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csm1_swaps_the_middle_blocks() {
        assert_eq!(csm1(8), 16);
        assert_eq!(csm1(16), 8);
        assert_eq!(csm1(7), 7);
        assert_eq!(csm1(31), 31);
        assert_eq!(csm1(40), 48);
        assert!((0..256).all(|i| csm1(csm1(i)) == i));
    }

    #[test]
    fn tim2_and_pack_round_trip() {
        let mut pal = [[0u8; 4]; 256];
        for (i, c) in pal.iter_mut().enumerate() {
            *c = [i as u8, 255 - i as u8, 7, 0x80];
        }
        let px: Vec<u8> = (0..32u32).map(|i| (i * 9) as u8).collect();
        let t = tim2_indexed8(8, 4, &px, &pal).unwrap();
        assert_eq!(t.len(), 16 + 0x30 + 32 + 1024);
        let (w, h, rgba) = tim2_decode8(&t).unwrap();
        assert_eq!((w, h), (8, 4));
        assert!(rgba.iter().zip(&px).all(|(c, &i)| *c == pal[i as usize]));
        let p = Rtm { entries: vec![RtmEntry::new("xw_ex_5030_0", t.clone()).unwrap(), RtmEntry::new("xw_ex_5030_1", t).unwrap()] };
        let b = p.write();
        assert_eq!(Rtm::parse(&b).unwrap(), p);
        assert_eq!(p.entries[1].name_str(), "xw_ex_5030_1.tm2");
        assert!(RtmEntry::new("a_name_far_too_long", vec![]).is_err());
    }
}
