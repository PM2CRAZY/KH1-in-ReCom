//! Re:CoM `.MDL` models: the skeleton the motion converter needs ([`RecomSkeleton`]), and the
//! whole file read and written ([`Mdl`]), for the Keyblade models (`docs/recom-keyblade-models.md`).
//!
//! Skeleton layout (Checked (files)): `+0x10 u16` bone count, `+0x14 u32` name-table offset and
//! `+0x18 u32` rest-matrix offset (both from +0x10). Name table: 20 bytes per bone
//! `{char name[16]; s16 parent; s16 index}`. Rest matrices: 4×4 f32 per bone, row-vector,
//! parent-relative, metres, Y up.

use crate::ann::{cstr, Rd};
use crate::math::Aff;
use crate::{bad, Result};

/// One Re:CoM bone.
#[derive(Clone, Debug, PartialEq)]
pub struct RBone {
    /// Name (`Root`, `ex_0010_sk`, `hip`, … `WeaponR`).
    pub name: String,
    /// Parent index, −1 for none.
    pub parent: i16,
    /// Rest local matrix (parent-relative).
    pub local: Aff,
}

/// A Re:CoM skeleton.
#[derive(Clone, Debug, PartialEq)]
pub struct RecomSkeleton {
    /// Bones in file order (parents come first).
    pub bones: Vec<RBone>,
}

impl RecomSkeleton {
    /// Parses a whole `.MDL` file.
    pub fn parse(d: &[u8]) -> Result<RecomSkeleton> {
        let r = Rd(d);
        let n = r.u16(0x10)? as usize;
        let nt = 0x10 + r.u32(0x14)? as usize;
        let mt = 0x10 + r.u32(0x18)? as usize;
        let mut bones = Vec::with_capacity(n);
        for i in 0..n {
            let o = nt + 20 * i;
            let parent = r.i16(o + 16)?;
            if parent >= i as i16 {
                return bad(format!("mdl: bone {i} has parent {parent}"));
            }
            let m = |k: usize| -> Result<f64> { Ok(r.f32(mt + 64 * i + 4 * k)? as f64) };
            let local = Aff { r: [[m(0)?, m(1)?, m(2)?], [m(4)?, m(5)?, m(6)?], [m(8)?, m(9)?, m(10)?]], t: [m(12)?, m(13)?, m(14)?] };
            bones.push(RBone { name: cstr(r.slice(o, 16)?), parent, local });
        }
        Ok(RecomSkeleton { bones })
    }

    /// Rest world matrices.
    pub fn rest_world(&self) -> Vec<Aff> {
        let mut w: Vec<Aff> = Vec::with_capacity(self.bones.len());
        for b in &self.bones {
            let m = if b.parent < 0 { b.local } else { b.local.mul(&w[b.parent as usize]) };
            w.push(m);
        }
        w
    }

    /// Index of a bone by name.
    pub fn find(&self, name: &str) -> Option<usize> {
        self.bones.iter().position(|b| b.name == name)
    }
}

// ---------------------------------------------------------------------------------------------
// The whole file.
// ---------------------------------------------------------------------------------------------

/// Packet format of textured triangles (3 quadwords per vertex). Checked (files).
pub const FMT_TEXTURED: u16 = 6;
/// Packet format of untextured triangles (2 quadwords per vertex). Checked (files).
pub const FMT_PLAIN: u16 = 0x10;
/// The largest UNPACK `num` Square wrote (0xE0: 74 textured or 111 untextured vertices).
/// The field is 8 bits, so 255 is the hard limit. Checked (files: PL0002, PL0011, PL0101).
pub const MAX_PACKET_QW: usize = 0xE0;

const DMA_CNT: u32 = 0x1000_0000;
const DMA_RET: u32 = 0x6000_0000;
const VIF_STCYCL_4_4: u32 = 0x0100_0404;
const VIF_UNPACK_V4_32: u32 = 0x6C00_8000;
const VIF_MSCNT: u32 = 0x1700_0000;

/// A whole Re:CoM `.MDL` (resource type 0xB).
///
/// File: u32 part offsets (absolute), 0-terminated, padded to 16; parts 16-aligned; the file
/// ends 16-aligned. Loader `0x1402A06E0` → `0x1402A1E40` per part. Checked (files, decomp).
#[derive(Clone, Debug, PartialEq)]
pub struct Mdl {
    /// The parts, in file order.
    pub parts: Vec<MdlPart>,
}

/// One part: skeleton, textures, two draw lists, per-bone bounds.
///
/// Header (from the part start): `+0 u16` bones, `+2 u8 u8` (byte 3: 3 textured / 1 untextured;
/// overwritten at load by `0x1402750E0` / `0x140274B00`), `+4` names, `+8` rest matrices,
/// `+0xC` texture count, `+0x10` textures, `+0x14` list A, `+0x18` list B (0 = none),
/// `+0x1C` bounds, `+0x20` 16 filler bytes (`0xCC`). Checked (files, decomp `0x1402A1EF0`).
#[derive(Clone, Debug, PartialEq)]
pub struct MdlPart {
    /// Header bytes 2 and 3.
    pub flags: [u8; 2],
    /// The 16 bytes at `+0x20`.
    pub filler: [u8; 16],
    /// Bones: name, parent, index.
    pub bones: Vec<MdlBone>,
    /// Rest matrices (4×4 f32 as bits), one per bone.
    pub rest: Vec<[u32; 16]>,
    /// Texture table entries: `char name[16]` + 16 bytes the loader fills.
    pub textures: Vec<[u8; 32]>,
    /// List A, the first draw pass (`0x14026E6A0`).
    pub list_a: Vec<MeshPacket>,
    /// List B, the second draw pass (`0x14026EB00`), if any.
    pub list_b: Option<Vec<MeshPacket>>,
    /// Per-bone bounds `minX maxX minY maxY minZ maxZ` (f32 bits), in bone space.
    pub bounds: Vec<[u32; 6]>,
}

/// One bone record of a part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MdlBone {
    /// Zero-padded name.
    pub name: [u8; 16],
    /// Parent, −1 for none.
    pub parent: i16,
    /// Index.
    pub index: i16,
}

/// One DMA packet of a draw list: a VIF UNPACK of a header plus vertices, then MSCNT.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshPacket {
    /// [`FMT_TEXTURED`] or [`FMT_PLAIN`].
    pub fmt: u16,
    /// GIF tag word 1 (PRE, PRIM, NREG).
    pub gif: u32,
    /// GIF tag register list.
    pub regs: u32,
    /// The vertices.
    pub verts: Vec<MeshVertex>,
}

/// One vertex (quadwords as words).
///
/// - `q0`: strip flag (0xFFFF on the two vertices that start a strip, else 0 or 0x20, the
///   triangle's facing), weight (f32 1.0), 0, kick (0xA000 start, 0x2000 draw).
/// - `q1`: x y z (f32, bone space, metres), `bone << 16 | bone * 4`.
/// - `q2` (textured only): s t (f32), q (f32 1.0), texture index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeshVertex {
    /// Quadword 0.
    pub q0: [u32; 4],
    /// Quadword 1.
    pub q1: [u32; 4],
    /// Quadword 2, textured packets only.
    pub q2: Option<[u32; 4]>,
}

impl MeshVertex {
    /// The bone (`q1.w & 0xFFFF) >> 2`, as the PC converter `0x140275190` reads it).
    pub fn bone(&self) -> usize {
        ((self.q1[3] & 0xFFFF) >> 2) as usize
    }
    /// Position in bone space (metres).
    pub fn pos(&self) -> [f32; 3] {
        [f32::from_bits(self.q1[0]), f32::from_bits(self.q1[1]), f32::from_bits(self.q1[2])]
    }
    /// `s, t`, if textured.
    pub fn uv(&self) -> Option<[f32; 2]> {
        self.q2.map(|q| [f32::from_bits(q[0]), f32::from_bits(q[1])])
    }
    /// True for a vertex that draws a triangle with the two before it (kick 0x2000).
    pub fn draws(&self) -> bool {
        self.q0[3] & 0x8000 == 0
    }
}

fn align16(n: usize) -> usize {
    (n + 15) & !15
}

fn stride_qw(fmt: u16) -> Result<usize> {
    match fmt {
        FMT_TEXTURED => Ok(3),
        FMT_PLAIN => Ok(2),
        f => bad(format!("mdl: packet format {f:#x} not supported")),
    }
}

fn words<const N: usize>(r: &Rd, at: usize) -> Result<[u32; N]> {
    let mut w = [0u32; N];
    for (k, x) in w.iter_mut().enumerate() {
        *x = r.u32(at + 4 * k)?;
    }
    Ok(w)
}

fn parse_list(r: &Rd, mut p: usize, bones: usize) -> Result<(Vec<MeshPacket>, usize)> {
    let mut out = Vec::new();
    loop {
        let tag = r.u32(p)?;
        if tag >> 28 == 6 {
            if words::<4>(r, p)? != [DMA_RET, 0, 0, 0] {
                return bad(format!("mdl: unexpected DMA ret at {p:#x}"));
            }
            return Ok((out, p + 16));
        }
        let up = r.u32(p + 12)?;
        if tag >> 28 != 1 || r.u32(p + 4)? != 0 || r.u32(p + 8)? != VIF_STCYCL_4_4 || up & 0xFF00_FFFF != VIF_UNPACK_V4_32 {
            return bad(format!("mdl: unexpected packet at {p:#x}"));
        }
        let num = ((up >> 16) & 0xFF) as usize;
        let h = words::<8>(r, p + 16)?;
        let nv = (h[1] & 0xFFFF) as usize;
        let fmt = (h[3] & 0xFFFF) as u16;
        let s = stride_qw(fmt)?;
        let ok = tag & 0xFFFF == num as u32 + 1
            && h[0] as usize == num
            && h[1] == 0x1_0000 | nv as u32
            && h[2] == ((0xF3 - bones as u32) << 16) | nv as u32
            && h[3] >> 16 == 0x9C00
            && num == 2 + s * nv
            && h[4] == 0x8000 | nv as u32
            && h[7] == 0;
        if !ok {
            return bad(format!("mdl: packet header at {p:#x} not as Square writes it"));
        }
        let mut o = p + 48;
        let mut verts = Vec::with_capacity(nv);
        for _ in 0..nv {
            let q0 = words::<4>(r, o)?;
            let q1 = words::<4>(r, o + 16)?;
            let q2 = if s == 3 { Some(words::<4>(r, o + 32)?) } else { None };
            verts.push(MeshVertex { q0, q1, q2 });
            o += 16 * s;
        }
        if words::<4>(r, o)? != [VIF_MSCNT, 0, 0, 0] {
            return bad(format!("mdl: no MSCNT at {o:#x}"));
        }
        out.push(MeshPacket { fmt, gif: h[5], regs: h[6], verts });
        p = o + 16;
    }
}

fn write_list(out: &mut Vec<u8>, list: &[MeshPacket], bones: usize) -> Result<()> {
    let w = |out: &mut Vec<u8>, v: &[u32]| v.iter().for_each(|x| out.extend_from_slice(&x.to_le_bytes()));
    for pk in list {
        let s = stride_qw(pk.fmt)?;
        let nv = pk.verts.len();
        let num = 2 + s * nv;
        if num > 0xFF {
            return bad(format!("mdl: packet of {nv} vertices is too big for one UNPACK"));
        }
        if pk.verts.iter().any(|v| v.q2.is_some() != (s == 3)) {
            return bad("mdl: vertex quadwords do not match the packet format");
        }
        let (num, nv) = (num as u32, nv as u32);
        w(out, &[DMA_CNT | (num + 1), 0, VIF_STCYCL_4_4, VIF_UNPACK_V4_32 | (num << 16)]);
        w(out, &[num, 0x1_0000 | nv, ((0xF3 - bones as u32) << 16) | nv, 0x9C00_0000 | pk.fmt as u32, 0x8000 | nv, pk.gif, pk.regs, 0]);
        for v in &pk.verts {
            w(out, &v.q0);
            w(out, &v.q1);
            if let Some(q2) = v.q2 {
                w(out, &q2);
            }
        }
        w(out, &[VIF_MSCNT, 0, 0, 0]);
    }
    w(out, &[DMA_RET, 0, 0, 0]);
    Ok(())
}

impl MdlPart {
    /// Part layout: names at 0x30, then matrices, textures, list A, list B, bounds. Every
    /// Re:CoM file read so far has exactly this layout (`write(parse(f)) == f`); anything else
    /// is refused rather than silently re-laid.
    fn parse(d: &[u8], b: usize) -> Result<(MdlPart, usize)> {
        let r = Rd(d);
        let nb = r.u16(b)? as usize;
        let flags = [r.u8(b + 2)?, r.u8(b + 3)?];
        let o: [u32; 7] = words::<7>(&r, b + 4)?;
        let (o_names, o_mats, ntex, o_tex, o_a, o_b, o_bb) = (o[0] as usize, o[1] as usize, o[2] as usize, o[3] as usize, o[4] as usize, o[5] as usize, o[6] as usize);
        if nb == 0 || o_names != 0x30 || o_mats != 0x30 + align16(20 * nb) || o_tex != o_mats + 64 * nb || o_a != o_tex + 32 * ntex {
            return bad(format!("mdl: part at {b:#x} has an unexpected layout"));
        }
        let filler: [u8; 16] = r.arr(b + 0x20)?;
        let mut bones = Vec::with_capacity(nb);
        for i in 0..nb {
            let at = b + o_names + 20 * i;
            bones.push(MdlBone { name: r.arr(at)?, parent: r.i16(at + 16)?, index: r.i16(at + 18)? });
        }
        if r.slice(b + o_names + 20 * nb, o_mats - 0x30 - 20 * nb)?.iter().any(|&x| x != 0) {
            return bad("mdl: non-zero name-table padding");
        }
        let rest = (0..nb).map(|i| words::<16>(&r, b + o_mats + 64 * i)).collect::<Result<Vec<_>>>()?;
        let textures = (0..ntex).map(|i| r.arr::<32>(b + o_tex + 32 * i)).collect::<Result<Vec<_>>>()?;
        let (list_a, end_a) = parse_list(&r, b + o_a, nb)?;
        let (list_b, end) = if o_b != 0 {
            if b + o_b != end_a {
                return bad("mdl: list B does not follow list A");
            }
            let (l, e) = parse_list(&r, b + o_b, nb)?;
            (Some(l), e)
        } else {
            (None, end_a)
        };
        if b + o_bb != end {
            return bad("mdl: bounds do not follow the lists");
        }
        let bounds = (0..nb).map(|i| words::<6>(&r, b + o_bb + 24 * i)).collect::<Result<Vec<_>>>()?;
        Ok((MdlPart { flags, filler, bones, rest, textures, list_a, list_b, bounds }, b + o_bb + 24 * nb))
    }

    fn write(&self) -> Result<Vec<u8>> {
        let nb = self.bones.len();
        if nb == 0 || nb > 0xF3 || self.rest.len() != nb || self.bounds.len() != nb {
            return bad("mdl: bones, rest matrices and bounds disagree");
        }
        let mut a = Vec::new();
        write_list(&mut a, &self.list_a, nb)?;
        let mut lb = Vec::new();
        if let Some(l) = &self.list_b {
            write_list(&mut lb, l, nb)?;
        }
        let o_mats = 0x30 + align16(20 * nb);
        let o_tex = o_mats + 64 * nb;
        let o_a = o_tex + 32 * self.textures.len();
        let o_b = if self.list_b.is_some() { o_a + a.len() } else { 0 };
        let o_bb = o_a + a.len() + lb.len();
        let mut out = Vec::with_capacity(o_bb + 24 * nb);
        out.extend_from_slice(&(nb as u16).to_le_bytes());
        out.extend_from_slice(&self.flags);
        for v in [0x30, o_mats, self.textures.len(), o_tex, o_a, o_b, o_bb] {
            out.extend_from_slice(&(v as u32).to_le_bytes());
        }
        out.extend_from_slice(&self.filler);
        for bn in &self.bones {
            out.extend_from_slice(&bn.name);
            out.extend_from_slice(&bn.parent.to_le_bytes());
            out.extend_from_slice(&bn.index.to_le_bytes());
        }
        out.resize(o_mats, 0);
        for m in &self.rest {
            m.iter().for_each(|x| out.extend_from_slice(&x.to_le_bytes()));
        }
        for t in &self.textures {
            out.extend_from_slice(t);
        }
        out.extend_from_slice(&a);
        out.extend_from_slice(&lb);
        for bb in &self.bounds {
            bb.iter().for_each(|x| out.extend_from_slice(&x.to_le_bytes()));
        }
        Ok(out)
    }

    /// Texture names (up to the first NUL).
    pub fn texture_names(&self) -> Vec<String> {
        self.textures.iter().map(|t| cstr(&t[..16])).collect()
    }
}

impl Mdl {
    /// Parses a whole `.MDL`.
    pub fn parse(d: &[u8]) -> Result<Mdl> {
        let r = Rd(d);
        let mut offs = Vec::new();
        loop {
            let o = r.u32(4 * offs.len())?;
            if o == 0 {
                break;
            }
            offs.push(o as usize);
            if offs.len() > 64 {
                return bad("mdl: too many parts");
            }
        }
        let mut at = align16(4 * (offs.len() + 1));
        if r.slice(4 * (offs.len() + 1), at - 4 * (offs.len() + 1))?.iter().any(|&x| x != 0) {
            return bad("mdl: non-zero header padding");
        }
        let mut parts = Vec::with_capacity(offs.len());
        for &o in &offs {
            if o != at {
                return bad(format!("mdl: part at {o:#x}, expected {at:#x}"));
            }
            let (p, end) = MdlPart::parse(d, o)?;
            parts.push(p);
            at = align16(end);
        }
        if at != d.len() {
            return bad(format!("mdl: file is {:#x} bytes, parts end at {at:#x}", d.len()));
        }
        Ok(Mdl { parts })
    }

    /// Writes the file. Offsets and sizes come from the content.
    pub fn write(&self) -> Result<Vec<u8>> {
        let bodies = self.parts.iter().map(MdlPart::write).collect::<Result<Vec<_>>>()?;
        let hdr = align16(4 * (bodies.len() + 1));
        let mut out = Vec::new();
        let mut o = hdr;
        for b in &bodies {
            out.extend_from_slice(&(o as u32).to_le_bytes());
            o += align16(b.len());
        }
        out.resize(hdr, 0);
        for b in &bodies {
            out.extend_from_slice(b);
            out.resize(align16(out.len()), 0);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vtx(bone: u32, draw: bool, tex: bool) -> MeshVertex {
        let q0 = if draw { [0, 0x3F80_0000, 0, 0x2000] } else { [0xFFFF, 0x3F80_0000, 0, 0xA000] };
        MeshVertex { q0, q1: [0x3F80_0000, 0, 0, (bone << 16) | (bone * 4)], q2: tex.then_some([0, 0, 0x3F80_0000, 0]) }
    }

    #[test]
    fn synthetic_round_trip() {
        let part = |tex: bool| MdlPart {
            flags: [1, if tex { 3 } else { 1 }],
            filler: [0xCC; 16],
            bones: (0..3).map(|i| MdlBone { name: crate::ann::name16(&format!("b{i}")), parent: i as i16 - 1, index: i as i16 }).collect(),
            rest: vec![[0; 16]; 3],
            textures: if tex { vec![[0x41; 32]] } else { vec![] },
            list_a: vec![MeshPacket { fmt: if tex { FMT_TEXTURED } else { FMT_PLAIN }, gif: 0x303E_4000, regs: 0x412, verts: vec![vtx(2, false, tex), vtx(2, false, tex), vtx(2, true, tex)] }],
            list_b: tex.then(Vec::new),
            bounds: vec![[0; 6]; 3],
        };
        let m = Mdl { parts: vec![part(true), part(false)] };
        let b = m.write().unwrap();
        assert_eq!(b.len() % 16, 0);
        let p = Mdl::parse(&b).unwrap();
        assert_eq!(p, m);
        assert_eq!(p.write().unwrap(), b);
        assert_eq!(p.parts[0].list_a[0].verts[2].bone(), 2);
        assert!(p.parts[0].list_a[0].verts[2].draws() && !p.parts[0].list_a[0].verts[0].draws());
    }

    #[test]
    fn refuses_oversized_packets() {
        let v = vtx(2, false, true);
        let pk = MeshPacket { fmt: FMT_TEXTURED, gif: 0, regs: 0, verts: vec![v; 85] };
        assert!(write_list(&mut Vec::new(), &[pk], 12).is_err());
    }
}
