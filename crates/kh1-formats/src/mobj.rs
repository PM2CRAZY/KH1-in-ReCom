//! A whole KH1 `.mdls` file, read and written: the section container, the `MOBJ` model
//! section (main model, optional second model, textures) and the PS2 mesh packets.
//!
//! Built for the "Re:CoM's content in KH1" feasibility test (`docs/kh1-model-writer.md`).
//! [`MdlsFile::write`] rebuilds every size, count and offset from the parsed content; the test
//! is that `write(parse(f)) == f` byte for byte on every `.mdls` in the player's install.
//!
//! Layout, Checked (files: all 500 `xa_*.mdls` of the Steam install):
//! - Container: `u32 n` (8), `n + 1` u32 offsets, zeros to `offs[0]` (0x80). Section `i` is
//!   `offs[i]..offs[i+1]`; bytes after `offs[n]` are a tail. An offset at or past the end of
//!   the file is written as the end rounded up to 0x80 (the last section with data is stored
//!   shorter than its slot). Section 0 is `MOBJ`. The others
//!   (AI scripts and more, Stage 13) are kept as bytes.
//! - `MOBJ` (offsets from `MOBJ`): `"MOBJ" | size` (to the end of the CLUTs, minus 8) | texture
//!   info offset, size | texture data offset, size | CLUT offset, size | model offset (0x40),
//!   size | second model offset, size | end of texture info | 12 zero bytes`. Then the model,
//!   the second model, the texture info (16 bytes each), zeros up to a 0x80 boundary, the
//!   texture pixels (8-bit, w × h each), the CLUTs (0x400 each).
//! - Model: `joint count | joint table offset | pair table offset | mesh count`, 16-byte mesh
//!   headers (`flags | texture | texture again | packets offset`), 32 zero bytes, then (main
//!   model) a 0xF0-byte table of 60 u32 and the 0x30-byte joints, then each mesh's packets. The
//!   second model has no tables (both offsets 0xFFFFFFFF).
//! - Packet: a DMA tag (`qwc u16 | 0 | id` with id 0x10 or 0x70 for the last | 0), VIF
//!   `STCYCL 1,1` (0x01000101), VIF `UNPACK V4-32` (`0x6C008000 | qw << 16`), the unpacked
//!   items, then a 16-byte DMA `ret` tag (0x17000000).
//! - Items (first u32 = kind): 0 joint matrix slot (`joint | slot | group`, then 0x70 zero
//!   bytes the VU fills), 1 strip (`size | count | flag`, a GIF tag, then vertices), 2 weight
//!   list (`size | count | 0`, then 16-byte entries), 0x8000 end (three u32).
//! - Vertex: normal (3 f32 + u32 word), one or more positions (x, y, z, weight; the weights sum
//!   to 1 and are never 0), then the UV quadword (u, v, 1.0, 0). A UV quadword is told from a
//!   position by its last word being 0.
//! - Plain meshes (mesh flag bit 17, the second model's): matrix items are 0x50 bytes (0x40
//!   zero bytes) and vertices 16 (x, y, z, a word 0 / 1 / 2).

use crate::mdls::{parse_bones, Bone, BONE_SIZE, NO_LINK};
use crate::reader::Bytes;
use crate::{bad, Result};

/// The `MOBJ` magic.
pub const MOBJ: &[u8; 4] = b"MOBJ";
/// Bytes of the per-model pair table (60 u32).
pub const PAIR_TABLE_WORDS: usize = 60;
/// DMA tag id of a packet that continues (`cnt`).
pub const DMA_CNT: u8 = 0x10;
/// DMA tag id of the model's last packet (`end`).
pub const DMA_END: u8 = 0x70;
const STCYCL: u32 = 0x0100_0101;
const UNPACK_V4_32: u32 = 0x6C00_8000;
const DMA_RET: [u8; 16] = [0, 0, 0, 0x17, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
const ONE: u32 = 0x3F80_0000;

/// A parsed `.mdls` file.
#[derive(Clone, Debug, PartialEq)]
pub struct MdlsFile {
    /// The model section.
    pub mobj: Mobj,
    /// Bytes after `MOBJ`'s data in section 0, kept as stored: a `_KN5` marker block (0x80 or
    /// 0x10 bytes, 493 files), a `BONE` block (6) or a `URSL` block (1). Not read here.
    pub mobj_trailer: Vec<u8>,
    /// Sections 1.. as stored (other formats, not read here).
    pub sections: Vec<Vec<u8>>,
    /// Bytes after the last section offset.
    pub tail: Vec<u8>,
}

/// The `MOBJ` section.
#[derive(Clone, Debug, PartialEq)]
pub struct Mobj {
    /// The skinned model.
    pub model: Model,
    /// A second, joint-less model (441 of the 500 files have one).
    pub second: Option<Model>,
    /// The PS2 textures.
    pub textures: Vec<Texture>,
}

/// One PS2 texture: 8-bit indices and a 256-colour CLUT.
#[derive(Clone, Debug, PartialEq)]
pub struct Texture {
    /// Info bytes 0 and 1 (0, 4 in 3631 of 3679; meaning not traced).
    pub kind: [u8; 2],
    /// Width in pixels (a power of two).
    pub width: u16,
    /// Height in pixels (a power of two).
    pub height: u16,
    /// `width × height` indices, in the PS2 layout.
    pub pixels: Vec<u8>,
    /// 0x400 bytes: 256 RGBA colours, PS2 order.
    pub clut: Vec<u8>,
}

/// A model: joints and meshes.
#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    /// The header's joint count (also set on the second model, which has no joint table).
    pub joint_count: u32,
    /// The pair table and joints (main model only).
    pub skeleton: Option<ModelSkeleton>,
    /// The meshes.
    pub meshes: Vec<Mesh>,
}

/// The main model's tables.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelSkeleton {
    /// The 60-word table between the mesh headers and the joints (meaning not traced).
    pub pairs: [u32; PAIR_TABLE_WORDS],
    /// The joints.
    pub joints: Vec<Bone>,
}

/// One mesh: a texture and a packet stream.
#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    /// Mesh header word 0 (bit 16: weighted vertices; bit 17: the second model; low bits vary).
    pub flags: u32,
    /// Texture index.
    pub texture: u32,
    /// The packets.
    pub packets: Vec<Packet>,
}

/// One DMA packet.
#[derive(Clone, Debug, PartialEq)]
pub struct Packet {
    /// DMA tag id ([`DMA_CNT`] or [`DMA_END`]).
    pub dma_id: u8,
    /// The unpacked items.
    pub items: Vec<Item>,
}

/// One item of a packet.
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    /// Kind 0: a joint matrix loaded into a VU slot.
    Matrix {
        /// Joint index.
        joint: u32,
        /// Matrix slot.
        slot: u32,
        /// Third word (0 for single weights; a group for weighted meshes).
        group: u32,
    },
    /// Kind 1: a triangle strip.
    Strip(Strip),
    /// Kind 2: a list of 16-byte entries (weighted meshes).
    Weights(Vec<[u32; 4]>),
    /// Kind 0x8000: end of a run (`48, 0, 0` or `0, 0, 0`).
    End([u32; 3]),
}

/// A triangle strip.
#[derive(Clone, Debug, PartialEq)]
pub struct Strip {
    /// Strip flag (0 or 1: winding).
    pub flag: u32,
    /// The GIF tag's words, with NLOOP (low 15 bits of word 0) left out (it is the count).
    pub gif: [u32; 4],
    /// The vertices.
    pub vertices: Vertices,
}

/// A strip's vertices, by mesh kind.
#[derive(Clone, Debug, PartialEq)]
pub enum Vertices {
    /// Textured vertices (normal, positions, UV).
    Full(Vec<Vertex>),
    /// Plain vertices (position and a word), in meshes with flag bit 17.
    Plain(Vec<PlainVertex>),
}

impl Vertices {
    fn len(&self) -> usize {
        match self {
            Vertices::Full(v) => v.len(),
            Vertices::Plain(v) => v.len(),
        }
    }
}

/// One vertex of a plain mesh.
#[derive(Clone, Debug, PartialEq)]
pub struct PlainVertex {
    /// Position in joint space.
    pub pos: [f32; 3],
    /// The fourth word (0, 1 or 2).
    pub word: u32,
}

/// Mesh flag bit of plain meshes.
pub const MESH_PLAIN: u32 = 0x20000;

/// One vertex of a PS2 strip.
#[derive(Clone, Debug, PartialEq)]
pub struct Vertex {
    /// Normal.
    pub normal: [f32; 3],
    /// The normal's fourth word (0, 1 or 2).
    pub normal_w: u32,
    /// Positions in joint space with their weights (`[x, y, z, weight]`), one per influence.
    pub influences: Vec<[f32; 4]>,
    /// Texture coordinates.
    pub uv: [f32; 2],
}

// ---- writing helpers ----------------------------------------------------------------------

#[derive(Default)]
struct W(Vec<u8>);

impl W {
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn f32(&mut self, v: f32) {
        self.u32(v.to_bits());
    }
    fn zeros(&mut self, n: usize) {
        self.0.resize(self.0.len() + n, 0);
    }
    fn pad_to(&mut self, align: usize) {
        let n = self.0.len().div_ceil(align) * align;
        self.0.resize(n, 0);
    }
    fn len(&self) -> usize {
        self.0.len()
    }
    fn put_u32(&mut self, at: usize, v: u32) {
        self.0[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }
}

fn zero(b: &Bytes, at: usize, n: usize, what: &str) -> Result<()> {
    if b.slice(at, n)?.iter().any(|&x| x != 0) {
        return bad(format!("mdls: non-zero bytes in {what} at {at:#x}"));
    }
    Ok(())
}

fn u32_at(v: usize) -> Result<u32> {
    u32::try_from(v).or_else(|_| bad("mdls: offset too large"))
}

// ---- container ----------------------------------------------------------------------------

impl MdlsFile {
    /// Parses a whole `.mdls` file.
    pub fn parse(file: &[u8]) -> Result<MdlsFile> {
        let b = Bytes(file);
        let n = b.u32(0)? as usize;
        if n == 0 || n > 64 {
            return bad(format!("mdls: implausible section count {n}"));
        }
        let mut offs = Vec::with_capacity(n + 1);
        for i in 0..=n {
            offs.push(b.u32(4 + 4 * i)? as usize);
        }
        // Offsets past the end of the file all point at the end rounded up to 0x80: the last
        // section with data is stored shorter than its slot.
        let len = file.len();
        let end_slot = len.div_ceil(0x80) * 0x80;
        if offs.windows(2).any(|w| w[0] > w[1]) || offs.iter().any(|&o| o > len && o != end_slot) || offs[0] < 4 + 4 * (n + 1) || offs[0] > len {
            return bad("mdls: section offsets out of order");
        }
        for o in &mut offs {
            *o = (*o).min(len);
        }
        zero(&b, 4 + 4 * (n + 1), offs[0] - 4 - 4 * (n + 1), "container header")?;
        let m = offs[0];
        let (mobj, used) = Mobj::parse(&file[m..offs[1]])?;
        let mobj_trailer = file[m + used..offs[1]].to_vec();
        let sections = (1..n).map(|i| file[offs[i]..offs[i + 1]].to_vec()).collect();
        Ok(MdlsFile { mobj, mobj_trailer, sections, tail: file[offs[n]..].to_vec() })
    }

    /// Writes the file. Every offset, size and count is rebuilt from the content.
    pub fn write(&self) -> Result<Vec<u8>> {
        let n = self.sections.len() + 1;
        let first = 0x80usize.max((4 + 4 * (n + 1)).div_ceil(16) * 16);
        let mut out = W::default();
        out.u32(n as u32);
        out.zeros(4 * (n + 1));
        out.pad_to(first);
        let mut offs = vec![out.len()];
        out.0.extend_from_slice(&self.mobj.write()?);
        out.0.extend_from_slice(&self.mobj_trailer);
        offs.push(out.len());
        for s in &self.sections {
            out.0.extend_from_slice(s);
            offs.push(out.len());
        }
        out.0.extend_from_slice(&self.tail);
        let len = out.len();
        for (i, o) in offs.iter().enumerate() {
            let o = if *o == len { len.div_ceil(0x80) * 0x80 } else { *o };
            out.put_u32(4 + 4 * i, u32_at(o)?);
        }
        Ok(out.0)
    }
}

// ---- MOBJ ---------------------------------------------------------------------------------

const MOBJ_HEADER: usize = 0x40;
const TEX_INFO: usize = 16;
const CLUT_SIZE: usize = 0x400;

impl Mobj {
    /// Parses a `MOBJ` section (starting at its magic). Returns it and the bytes used.
    pub fn parse(s: &[u8]) -> Result<(Mobj, usize)> {
        let b = Bytes(s);
        if b.slice(0, 4)? != MOBJ {
            return bad("mdls: section 0 is not MOBJ");
        }
        let size = b.u32(4)? as usize;
        let h: Vec<usize> = (0..11).map(|i| b.u32(8 + 4 * i).map(|v| v as usize)).collect::<Result<_>>()?;
        let (ti, tis, td, tds, cl, cls, mo, ms, so, ss, tie) = (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7], h[8], h[9], h[10]);
        zero(&b, 0x34, 0x0C, "MOBJ header")?;
        if mo != MOBJ_HEADER {
            return bad(format!("mdls: model at {mo:#x}, expected 0x40"));
        }
        let model = Model::parse(b.slice(mo, ms)?, true)?;
        let second = if ss != 0 {
            if so != mo + ms {
                return bad("mdls: second model not after the model");
            }
            Some(Model::parse(b.slice(so, ss)?, false)?)
        } else {
            if so != mo + ms {
                return bad("mdls: empty second model offset not after the model");
            }
            None
        };
        if ti != so + ss || tie != ti + tis || tis % TEX_INFO != 0 {
            return bad("mdls: texture info not where expected");
        }
        let count = tis / TEX_INFO;
        if td != tie.div_ceil(0x80) * 0x80 {
            return bad("mdls: texture data not at the next 0x80 boundary");
        }
        zero(&b, tie, td - tie, "texture padding")?;
        if cl != td + tds || cls != count * CLUT_SIZE || size != cl + cls - 8 {
            return bad("mdls: CLUTs or size not where expected");
        }
        let mut textures = Vec::with_capacity(count);
        let mut px = td;
        for i in 0..count {
            let at = ti + TEX_INFO * i;
            let kind = [b.u8(at)?, b.u8(at + 1)?];
            let (lw, lh) = (b.u8(at + 2)?, b.u8(at + 3)?);
            let (width, height) = (b.u16(at + 4)?, b.u16(at + 6)?);
            if lw >= 16 || lh >= 16 || 1u16 << lw != width || 1u16 << lh != height {
                return bad(format!("mdls: texture {i} size {width}x{height} is not 2^{lw} x 2^{lh}"));
            }
            zero(&b, at + 8, 8, "texture info")?;
            let n = width as usize * height as usize;
            let pixels = b.slice(px, n)?.to_vec();
            px += n;
            let clut = b.slice(cl + CLUT_SIZE * i, CLUT_SIZE)?.to_vec();
            textures.push(Texture { kind, width, height, pixels, clut });
        }
        if px != cl {
            return bad("mdls: texture data size does not match the textures");
        }
        Ok((Mobj { model, second, textures }, size + 8))
    }

    /// Writes the section.
    pub fn write(&self) -> Result<Vec<u8>> {
        let mut out = W::default();
        out.0.extend_from_slice(MOBJ);
        out.zeros(MOBJ_HEADER - 4);
        let model = self.model.write()?;
        out.0.extend_from_slice(&model);
        let so = out.len();
        let second = match &self.second {
            Some(m) => m.write()?,
            None => Vec::new(),
        };
        out.0.extend_from_slice(&second);
        let ti = out.len();
        for t in &self.textures {
            let lw = t.width.trailing_zeros();
            let lh = t.height.trailing_zeros();
            if !t.width.is_power_of_two() || !t.height.is_power_of_two() {
                return bad("mdls: texture sizes must be powers of two");
            }
            if t.pixels.len() != t.width as usize * t.height as usize || t.clut.len() != CLUT_SIZE {
                return bad("mdls: texture pixel or CLUT size does not match");
            }
            out.0.extend_from_slice(&[t.kind[0], t.kind[1], lw as u8, lh as u8]);
            out.0.extend_from_slice(&t.width.to_le_bytes());
            out.0.extend_from_slice(&t.height.to_le_bytes());
            out.zeros(8);
        }
        let tie = out.len();
        out.pad_to(0x80);
        let td = out.len();
        for t in &self.textures {
            out.0.extend_from_slice(&t.pixels);
        }
        let cl = out.len();
        for t in &self.textures {
            out.0.extend_from_slice(&t.clut);
        }
        let end = out.len();
        let fields = [
            end - 8,
            ti,
            tie - ti,
            td,
            cl - td,
            cl,
            end - cl,
            MOBJ_HEADER,
            model.len(),
            so,
            second.len(),
            tie,
        ];
        for (i, v) in fields.iter().enumerate() {
            out.put_u32(4 + 4 * i, u32_at(*v)?);
        }
        Ok(out.0)
    }
}

// ---- model --------------------------------------------------------------------------------

const MESH_HEADER: usize = 16;
const GAP_AFTER_MESHES: usize = 32;

impl Model {
    /// Parses a model block (offsets relative to its start). `main` = the skinned model.
    pub fn parse(m: &[u8], main: bool) -> Result<Model> {
        let b = Bytes(m);
        let joint_count = b.u32(0)?;
        let (jo, po, mc) = (b.u32(4)?, b.u32(8)?, b.u32(12)? as usize);
        if mc == 0 || mc > 4096 {
            return bad(format!("mdls: implausible mesh count {mc}"));
        }
        let hdr_end = 16 + MESH_HEADER * mc;
        let mut heads = Vec::with_capacity(mc);
        for i in 0..mc {
            let at = 16 + MESH_HEADER * i;
            let (flags, t1, t2, off) = (b.u32(at)?, b.u32(at + 4)?, b.u32(at + 8)?, b.u32(at + 12)? as usize);
            if t1 != t2 {
                return bad(format!("mdls: mesh {i} has two texture indices {t1} / {t2}"));
            }
            heads.push((flags, t1, off));
        }
        zero(&b, hdr_end, GAP_AFTER_MESHES, "model header gap")?;
        let tables_end = hdr_end + GAP_AFTER_MESHES;
        let (skeleton, data_start) = if main {
            if po as usize != tables_end || jo as usize != tables_end + 4 * PAIR_TABLE_WORDS {
                return bad("mdls: model tables not where expected");
            }
            let mut pairs = [0u32; PAIR_TABLE_WORDS];
            for (i, p) in pairs.iter_mut().enumerate() {
                *p = b.u32(po as usize + 4 * i)?;
            }
            let joints = parse_bones(m, jo as usize, joint_count as usize)?;
            (Some(ModelSkeleton { pairs, joints }), jo as usize + BONE_SIZE * joint_count as usize)
        } else {
            if jo != u32::MAX || po != u32::MAX {
                return bad("mdls: second model with joint tables");
            }
            (None, tables_end)
        };
        let mut meshes = Vec::with_capacity(mc);
        let mut at = data_start;
        for (i, (flags, texture, off)) in heads.iter().copied().enumerate() {
            if off != at {
                return bad(format!("mdls: mesh {i} packets at {off:#x}, expected {at:#x}"));
            }
            let end = heads.get(i + 1).map_or(m.len(), |h| h.2);
            let packets = parse_packets(m, off, end, flags & MESH_PLAIN != 0)?;
            at = end;
            meshes.push(Mesh { flags, texture, packets });
        }
        Ok(Model { joint_count, skeleton, meshes })
    }

    /// Writes the model block.
    pub fn write(&self) -> Result<Vec<u8>> {
        let mc = self.meshes.len();
        let mut out = W::default();
        out.u32(self.joint_count);
        out.zeros(12 + MESH_HEADER * mc + GAP_AFTER_MESHES);
        let tables_end = out.len();
        match &self.skeleton {
            Some(s) => {
                if s.joints.len() != self.joint_count as usize {
                    return bad("mdls: joint count does not match the joints");
                }
                out.put_u32(4, u32_at(tables_end + 4 * PAIR_TABLE_WORDS)?);
                out.put_u32(8, u32_at(tables_end)?);
                for p in s.pairs {
                    out.u32(p);
                }
                for j in &s.joints {
                    write_bone(&mut out, j);
                }
            }
            None => {
                out.put_u32(4, u32::MAX);
                out.put_u32(8, u32::MAX);
            }
        }
        out.put_u32(12, u32_at(mc)?);
        for (i, mesh) in self.meshes.iter().enumerate() {
            let at = 16 + MESH_HEADER * i;
            out.put_u32(at, mesh.flags);
            out.put_u32(at + 4, mesh.texture);
            out.put_u32(at + 8, mesh.texture);
            out.put_u32(at + 12, u32_at(out.len())?);
            for p in &mesh.packets {
                write_packet(&mut out, p, mesh.flags & MESH_PLAIN != 0)?;
            }
        }
        Ok(out.0)
    }
}

fn write_bone(out: &mut W, j: &Bone) {
    for v in j.scale {
        out.f32(v);
    }
    out.u32(j.index);
    for v in j.rot {
        out.f32(v);
    }
    out.f32(j.rot_extra);
    for v in j.trans {
        out.f32(v);
    }
    let parent = j.parent.unwrap_or(NO_LINK) as u32;
    out.u32(parent | (j.sibling as u32 & 0x3FF) << 10 | (j.child as u32 & 0x3FF) << 20 | (j.link_flags as u32) << 30);
}

// ---- packets ------------------------------------------------------------------------------

fn parse_packets(m: &[u8], mut p: usize, end: usize, plain: bool) -> Result<Vec<Packet>> {
    let b = Bytes(m);
    let mat_zeros = if plain { 0x40 } else { 0x70 };
    let mut packets = Vec::new();
    while p < end {
        let qwc = b.u16(p)? as usize;
        let dma_id = b.u8(p + 3)?;
        if b.u8(p + 2)? != 0 || b.u32(p + 4)? != 0 || !(dma_id == DMA_CNT || dma_id == DMA_END) {
            return bad(format!("mdls: unexpected DMA tag at {p:#x}"));
        }
        if b.u32(p + 8)? != STCYCL {
            return bad(format!("mdls: no STCYCL at {:#x}", p + 8));
        }
        let unpack = b.u32(p + 12)?;
        let num = ((unpack >> 16) & 0xFF) as usize;
        if unpack & 0xFF00_FFFF != UNPACK_V4_32 || qwc != num + 1 || num == 0 {
            return bad(format!("mdls: unexpected UNPACK at {:#x}", p + 12));
        }
        let mut q = p + 16;
        let qe = q + 16 * num;
        let mut items = Vec::new();
        while q < qe {
            let kind = b.u32(q)?;
            match kind {
                0 => {
                    items.push(Item::Matrix { joint: b.u32(q + 4)?, slot: b.u32(q + 8)?, group: b.u32(q + 12)? });
                    zero(&b, q + 16, mat_zeros, "matrix slot")?;
                    q += 16 + mat_zeros;
                }
                1 => {
                    let (size, count, flag) = (b.u32(q + 4)? as usize, b.u32(q + 8)? as usize, b.u32(q + 12)?);
                    let g0 = b.u32(q + 16)?;
                    if g0 & 0x7FFF != count as u32 {
                        return bad(format!("mdls: GIF NLOOP is not the vertex count at {q:#x}"));
                    }
                    let gif = [g0 & !0x7FFF, b.u32(q + 20)?, b.u32(q + 24)?, b.u32(q + 28)?];
                    let mut v = q + 32;
                    if plain {
                        let mut vs = Vec::with_capacity(count);
                        for _ in 0..count {
                            vs.push(PlainVertex { pos: b.f32x3(v)?, word: b.u32(v + 12)? });
                            v += 16;
                        }
                        if v - q != size {
                            return bad(format!("mdls: plain strip size {size:#x} does not match at {q:#x}"));
                        }
                        items.push(Item::Strip(Strip { flag, gif, vertices: Vertices::Plain(vs) }));
                        q = v;
                        continue;
                    }
                    let mut vertices = Vec::with_capacity(count);
                    for _ in 0..count {
                        let normal = b.f32x3(v)?;
                        let normal_w = b.u32(v + 12)?;
                        v += 16;
                        let mut influences = Vec::with_capacity(1);
                        loop {
                            if v + 16 > qe {
                                return bad(format!("mdls: vertex runs past its packet at {v:#x}"));
                            }
                            if b.u32(v + 12)? == 0 {
                                break;
                            }
                            influences.push([b.f32(v)?, b.f32(v + 4)?, b.f32(v + 8)?, b.f32(v + 12)?]);
                            v += 16;
                        }
                        if influences.is_empty() || b.u32(v + 8)? != ONE {
                            return bad(format!("mdls: malformed vertex at {v:#x}"));
                        }
                        let uv = [b.f32(v)?, b.f32(v + 4)?];
                        v += 16;
                        vertices.push(Vertex { normal, normal_w, influences, uv });
                    }
                    if v - q != size {
                        return bad(format!("mdls: strip size {size:#x} does not match its vertices at {q:#x}"));
                    }
                    items.push(Item::Strip(Strip { flag, gif, vertices: Vertices::Full(vertices) }));
                    q = v;
                }
                2 => {
                    let (size, count) = (b.u32(q + 4)? as usize, b.u32(q + 8)? as usize);
                    if size != 16 + 16 * count || b.u32(q + 12)? != 0 {
                        return bad(format!("mdls: bad weight list at {q:#x}"));
                    }
                    let mut e = Vec::with_capacity(count);
                    for k in 0..count {
                        let a = q + 16 + 16 * k;
                        e.push([b.u32(a)?, b.u32(a + 4)?, b.u32(a + 8)?, b.u32(a + 12)?]);
                    }
                    items.push(Item::Weights(e));
                    q += size;
                }
                0x8000 => {
                    items.push(Item::End([b.u32(q + 4)?, b.u32(q + 8)?, b.u32(q + 12)?]));
                    q += 16;
                }
                k => return bad(format!("mdls: unknown packet item {k:#x} at {q:#x}")),
            }
        }
        if q != qe {
            return bad(format!("mdls: packet items overrun at {q:#x}"));
        }
        if b.slice(qe, 16)? != DMA_RET {
            return bad(format!("mdls: no DMA ret tag at {qe:#x}"));
        }
        packets.push(Packet { dma_id, items });
        p = qe + 16;
    }
    if p != end {
        return bad("mdls: packets overrun their mesh");
    }
    Ok(packets)
}

fn write_packet(out: &mut W, p: &Packet, plain: bool) -> Result<()> {
    let start = out.len();
    out.zeros(8);
    out.u32(STCYCL);
    out.u32(0);
    for it in &p.items {
        match it {
            Item::Matrix { joint, slot, group } => {
                out.u32(0);
                out.u32(*joint);
                out.u32(*slot);
                out.u32(*group);
                out.zeros(if plain { 0x40 } else { 0x70 });
            }
            Item::Strip(s) => {
                let at = out.len();
                let count = s.vertices.len();
                if count > 0x7FFF || s.gif[0] & 0x7FFF != 0 {
                    return bad("mdls: strip too long for a GIF tag");
                }
                out.u32(1);
                out.u32(0);
                out.u32(count as u32);
                out.u32(s.flag);
                out.u32(s.gif[0] | count as u32);
                out.u32(s.gif[1]);
                out.u32(s.gif[2]);
                out.u32(s.gif[3]);
                let full = match (&s.vertices, plain) {
                    (Vertices::Full(v), false) => v,
                    (Vertices::Plain(v), true) => {
                        for pv in v {
                            for c in pv.pos {
                                out.f32(c);
                            }
                            out.u32(pv.word);
                        }
                        &Vec::new()
                    }
                    _ => return bad("mdls: vertex kind does not match the mesh kind"),
                };
                for v in full {
                    if v.influences.is_empty() || v.influences.iter().any(|i| i[3].to_bits() == 0) {
                        return bad("mdls: a vertex needs influences with non-zero weights");
                    }
                    for c in v.normal {
                        out.f32(c);
                    }
                    out.u32(v.normal_w);
                    for i in &v.influences {
                        for c in i {
                            out.f32(*c);
                        }
                    }
                    out.f32(v.uv[0]);
                    out.f32(v.uv[1]);
                    out.u32(ONE);
                    out.u32(0);
                }
                let size = out.len() - at;
                out.put_u32(at + 4, u32_at(size)?);
            }
            Item::Weights(e) => {
                out.u32(2);
                out.u32(u32_at(16 + 16 * e.len())?);
                out.u32(u32_at(e.len())?);
                out.u32(0);
                for w in e {
                    for x in w {
                        out.u32(*x);
                    }
                }
            }
            Item::End(w) => {
                out.u32(0x8000);
                for x in w {
                    out.u32(*x);
                }
            }
        }
    }
    let num = (out.len() - start - 16) / 16;
    if num == 0 || num > 0xFF {
        return bad(format!("mdls: packet of {num} quadwords does not fit one UNPACK"));
    }
    out.0[start..start + 2].copy_from_slice(&((num + 1) as u16).to_le_bytes());
    out.0[start + 3] = p.dma_id;
    out.put_u32(start + 12, UNPACK_V4_32 | (num as u32) << 16);
    out.0.extend_from_slice(&DMA_RET);
    Ok(())
}

impl Model {
    /// Calls `f` on every vertex position (each influence; plain vertices with weight 1), for
    /// edits such as scaling.
    pub fn for_each_position(&mut self, mut f: impl FnMut(&mut [f32; 4])) {
        for m in &mut self.meshes {
            for p in &mut m.packets {
                for it in &mut p.items {
                    if let Item::Strip(s) = it {
                        match &mut s.vertices {
                            Vertices::Full(vs) => {
                                for v in vs {
                                    for i in &mut v.influences {
                                        f(i);
                                    }
                                }
                            }
                            Vertices::Plain(vs) => {
                                for v in vs {
                                    let mut p = [v.pos[0], v.pos[1], v.pos[2], 1.0];
                                    f(&mut p);
                                    v.pos = [p[0], p[1], p[2]];
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bone(i: u32, parent: Option<u16>) -> Bone {
        Bone {
            index: i,
            parent,
            sibling: NO_LINK,
            child: NO_LINK,
            link_flags: 0,
            scale: [1.0; 3],
            rot: [0.0, 0.5, 0.0],
            rot_extra: 0.0,
            trans: [0.0, 10.0 * i as f32, 0.0],
        }
    }

    fn vertex(x: f32, uv: f32, weights: &[f32]) -> Vertex {
        Vertex {
            normal: [0.0, 1.0, 0.0],
            normal_w: 1,
            influences: weights.iter().map(|w| [x, 1.0, 2.0, *w]).collect(),
            uv: [uv, 0.5],
        }
    }

    /// A small model with every item kind, written, parsed and written again.
    #[test]
    fn synthetic_round_trip() {
        let strip = Strip { flag: 1, gif: [0, 0x303E_4000, 0x412, 0], vertices: Vertices::Full(vec![vertex(1.0, 0.1, &[1.0]), vertex(2.0, 0.2, &[1.0]), vertex(3.0, 0.3, &[0.25, 0.75])]) };
        let plain_strip = Strip { flag: 0, gif: [0, 0x1002_4000, 5, 0], vertices: Vertices::Plain(vec![PlainVertex { pos: [1.0, 2.0, 3.0], word: 2 }]) };
        let packet = Packet {
            dma_id: DMA_END,
            items: vec![
                Item::Matrix { joint: 1, slot: 0, group: 0 },
                Item::Weights(vec![[1, 0, 1, 0]]),
                Item::Strip(strip),
                Item::End([48, 0, 0]),
            ],
        };
        let model = Model {
            joint_count: 2,
            skeleton: Some(ModelSkeleton { pairs: [0xFFFF_FFFF; PAIR_TABLE_WORDS], joints: vec![bone(0, None), bone(1, Some(0))] }),
            meshes: vec![Mesh { flags: 0x10000, texture: 0, packets: vec![packet.clone()] }],
        };
        let plain_packet = Packet { dma_id: DMA_END, items: vec![Item::Matrix { joint: 1, slot: 0, group: 0 }, Item::Strip(plain_strip), Item::End([0, 0, 0])] };
        let _ = packet.clone();
        let second = Model { joint_count: 2, skeleton: None, meshes: vec![Mesh { flags: MESH_PLAIN, texture: 0, packets: vec![plain_packet] }] };
        let tex = Texture { kind: [0, 4], width: 16, height: 8, pixels: (0..128).map(|i| i as u8).collect(), clut: vec![7; CLUT_SIZE] };
        let file = MdlsFile {
            mobj: Mobj { model, second: Some(second), textures: vec![tex.clone(), tex] },
            mobj_trailer: b"_KN5".iter().copied().chain([0; 12]).collect(),
            sections: vec![vec![1, 2, 3, 4], vec![], vec![9; 32]],
            tail: vec![5; 3],
        };
        let bytes = file.write().unwrap();
        let back = MdlsFile::parse(&bytes).unwrap();
        assert_eq!(back, file);
        assert_eq!(back.write().unwrap(), bytes);
        // The skeleton reader still reads the written joints.
        let s = crate::mdls::Skeleton::parse(&bytes).unwrap();
        assert_eq!(s.bones[1].parent, Some(0));
    }

    #[test]
    fn rejects_bad_vertices() {
        let mut v = vertex(1.0, 0.0, &[1.0]);
        v.influences[0][3] = 0.0;
        let p = Packet { dma_id: DMA_END, items: vec![Item::Strip(Strip { flag: 0, gif: [0; 4], vertices: Vertices::Full(vec![v]) })] };
        let mut w = W::default();
        assert!(write_packet(&mut w, &p, false).is_err());
        let ok = Packet { dma_id: DMA_END, items: vec![Item::Strip(Strip { flag: 0, gif: [0; 4], vertices: Vertices::Full(vec![vertex(1.0, 0.0, &[1.0])]) })] };
        assert!(write_packet(&mut W::default(), &ok, true).is_err(), "full vertices in a plain mesh");
    }

    /// Every `.mdls` in `KH1_TEST_DATA/mdls/` round-trips byte for byte.
    #[test]
    fn game_files_round_trip() {
        let Some(dir) = crate::testdata::dir() else { return };
        let Ok(rd) = std::fs::read_dir(dir.join("mdls")) else { return };
        let mut n = 0;
        for e in rd.flatten() {
            let data = std::fs::read(e.path()).unwrap();
            let f = MdlsFile::parse(&data).unwrap_or_else(|err| panic!("{}: {err}", e.path().display()));
            assert!(f.write().unwrap() == data, "{}", e.path().display());
            n += 1;
        }
        eprintln!("{n} .mdls files round-tripped");
    }
}
