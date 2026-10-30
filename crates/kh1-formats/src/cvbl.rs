//! KH1 PC's remastered meshes (`.cvbl`, an HD asset of a `.mdls` entry), read and written.
//!
//! The PC draws these, not the `.mdls` PS2 packets (KH2-Combat-in-KH1 `docs/kh1-tint.md`). The
//! main model's mesh is the asset `-c0.cvbl` (tag: the model's offset in the file); a second
//! model has its own (e.g. `-50ea0.cvbl`).
//!
//! Layout, Checked (files: all 941 `.cvbl` assets of the 500 `xa_*.mdls` entries):
//! - Header: `0x4126 | mesh count | layout word | 0`. When the layout word's high half is 1,
//!   its low half counts 0x20-byte entries that follow (texture patches; kept as words).
//! - Mesh table, 16 bytes each: `flags | texture | texture again | offset`, the offset counted
//!   from file `+0x10`. `flags >> 16` is the mesh kind: 8 textured, 9 weighted, 0xA plain.
//! - Each mesh: 16 bytes of `0x12345678`, then chunks, then the 16-byte end chunk
//!   (`0x8000`, zeros). The mesh offset points at the first chunk.
//! - Chunk `0x11` (matrix slot): `0x11 | size | joint | slot`, then `size - 16` bytes of
//!   floats (size 0x80 textured, 0xC0 weighted, 0x50 plain).
//! - Chunk 1 (vertices): `1 | size | count | 0`, a format word (0x3C / 0x7C textured or
//!   weighted, 0x0C plain) and 12 zero bytes, then `count` vertices of `(size - 32) / count`
//!   bytes: 48 textured (normal xyz, flags, u, v, two words, position xyzw), 80 weighted, 16
//!   plain (position xyz, flags). Vertices are kept as words; [`Cvbl::for_each_position`]
//!   knows where the positions of the 48- and 16-byte forms are.

use crate::reader::Bytes;
use crate::{bad, Result};

/// The header magic.
pub const CVBL_MAGIC: u32 = 0x4126;
const FILL: u32 = 0x1234_5678;
const BASE: usize = 0x10;

/// A parsed `.cvbl`.
#[derive(Clone, Debug, PartialEq)]
pub struct Cvbl {
    /// Header word 2 (layout: high half 1 = patch entries follow, low half = their count or a
    /// value of unknown meaning).
    pub layout: u32,
    /// The 0x20-byte patch entries (as words).
    pub patches: Vec<[u32; 8]>,
    /// The meshes.
    pub meshes: Vec<CvMesh>,
}

/// One mesh.
#[derive(Clone, Debug, PartialEq)]
pub struct CvMesh {
    /// Table word 0 (`>> 16`: 8 textured, 9 weighted, 0xA plain).
    pub flags: u32,
    /// Texture slot (stored twice).
    pub texture: u32,
    /// The chunks before the end chunk.
    pub chunks: Vec<Chunk>,
}

/// One chunk of a mesh.
#[derive(Clone, Debug, PartialEq)]
pub enum Chunk {
    /// `0x11`: a joint matrix into a slot.
    Matrix {
        /// Joint index.
        joint: u32,
        /// Matrix slot.
        slot: u32,
        /// The floats after the header (matrix, then per-kind extras), as bits.
        data: Vec<u32>,
    },
    /// 1: vertices.
    Vertices {
        /// The format word (0x3C, 0x7C or 0x0C seen).
        format: u32,
        /// Bytes per vertex (48, 80 or 16 seen).
        stride: usize,
        /// The vertices' words, `stride / 4` per vertex.
        words: Vec<u32>,
    },
}

impl Chunk {
    /// Vertex count of a vertex chunk (0 for others).
    pub fn vertex_count(&self) -> usize {
        match self {
            Chunk::Vertices { stride, words, .. } => words.len() * 4 / stride,
            _ => 0,
        }
    }
}

impl Cvbl {
    /// Parses a `.cvbl`.
    pub fn parse(f: &[u8]) -> Result<Cvbl> {
        let b = Bytes(f);
        if b.u32(0)? != CVBL_MAGIC || b.u32(12)? != 0 {
            return bad("cvbl: bad header");
        }
        let mc = b.u32(4)? as usize;
        let layout = b.u32(8)?;
        if mc == 0 || mc > 4096 || layout >> 16 > 1 {
            return bad("cvbl: implausible header");
        }
        let np = if layout >> 16 == 1 { (layout & 0xFFFF) as usize } else { 0 };
        let mut patches = Vec::with_capacity(np);
        for i in 0..np {
            let at = 16 + 32 * i;
            patches.push(core::array::from_fn(|k| b.u32(at + 4 * k).unwrap_or(0)));
            b.slice(at, 32)?;
        }
        let table = 16 + 32 * np;
        let mut p = table + 16 * mc;
        let mut meshes = Vec::with_capacity(mc);
        for i in 0..mc {
            let at = table + 16 * i;
            let (flags, t1, t2, off) = (b.u32(at)?, b.u32(at + 4)?, b.u32(at + 8)?, b.u32(at + 12)? as usize);
            if t1 != t2 {
                return bad(format!("cvbl: mesh {i} has two texture slots"));
            }
            for k in 0..4 {
                if b.u32(p + 4 * k)? != FILL {
                    return bad(format!("cvbl: no fill before mesh {i}"));
                }
            }
            p += 16;
            if off + BASE != p {
                return bad(format!("cvbl: mesh {i} at {:#x}, expected {p:#x}", off + BASE));
            }
            let mut chunks = Vec::new();
            loop {
                let (tag, size) = (b.u32(p)?, b.u32(p + 4)? as usize);
                match tag {
                    0x8000 => {
                        if b.slice(p + 4, 12)?.iter().any(|&x| x != 0) {
                            return bad("cvbl: non-zero end chunk");
                        }
                        p += 16;
                        break;
                    }
                    0x11 => {
                        if !(16..=0x1000).contains(&size) || size % 4 != 0 {
                            return bad(format!("cvbl: matrix chunk size {size:#x}"));
                        }
                        let data = (0..(size - 16) / 4).map(|k| b.u32(p + 16 + 4 * k)).collect::<Result<_>>()?;
                        chunks.push(Chunk::Matrix { joint: b.u32(p + 8)?, slot: b.u32(p + 12)?, data });
                        p += size;
                    }
                    1 => {
                        let count = b.u32(p + 8)? as usize;
                        if b.u32(p + 12)? != 0 || b.slice(p + 20, 12)?.iter().any(|&x| x != 0) || count == 0 || size < 32 || (size - 32) % count != 0 {
                            return bad(format!("cvbl: bad vertex chunk at {p:#x}"));
                        }
                        let stride = (size - 32) / count;
                        if !stride.is_multiple_of(4) {
                            return bad("cvbl: vertex stride not in words");
                        }
                        let words = (0..(size - 32) / 4).map(|k| b.u32(p + 32 + 4 * k)).collect::<Result<_>>()?;
                        chunks.push(Chunk::Vertices { format: b.u32(p + 16)?, stride, words });
                        p += size;
                    }
                    t => return bad(format!("cvbl: unknown chunk {t:#x} at {p:#x}")),
                }
            }
            meshes.push(CvMesh { flags, texture: t1, chunks });
        }
        if p != f.len() {
            return bad(format!("cvbl: {} bytes after the last mesh", f.len() as isize - p as isize));
        }
        Ok(Cvbl { layout, patches, meshes })
    }

    /// Writes the `.cvbl`. Offsets, sizes and counts are rebuilt from the content.
    pub fn write(&self) -> Result<Vec<u8>> {
        let np = if self.layout >> 16 == 1 { (self.layout & 0xFFFF) as usize } else { 0 };
        if np != self.patches.len() {
            return bad("cvbl: layout word does not match the patch entries");
        }
        let mut out: Vec<u8> = Vec::new();
        let put = |out: &mut Vec<u8>, v: u32| out.extend_from_slice(&v.to_le_bytes());
        for v in [CVBL_MAGIC, self.meshes.len() as u32, self.layout, 0] {
            put(&mut out, v);
        }
        for e in &self.patches {
            for v in e {
                put(&mut out, *v);
            }
        }
        let table = out.len();
        out.resize(table + 16 * self.meshes.len(), 0);
        for (i, m) in self.meshes.iter().enumerate() {
            for _ in 0..4 {
                put(&mut out, FILL);
            }
            let at = table + 16 * i;
            let off = (out.len() - BASE) as u32;
            for (k, v) in [m.flags, m.texture, m.texture, off].iter().enumerate() {
                out[at + 4 * k..at + 4 * k + 4].copy_from_slice(&v.to_le_bytes());
            }
            for c in &m.chunks {
                match c {
                    Chunk::Matrix { joint, slot, data } => {
                        for v in [0x11, (16 + 4 * data.len()) as u32, *joint, *slot] {
                            put(&mut out, v);
                        }
                        for v in data {
                            put(&mut out, *v);
                        }
                    }
                    Chunk::Vertices { format, stride, words } => {
                        let count = words.len() * 4 / stride.max(&1);
                        if *stride == 0 || count == 0 || count * stride != words.len() * 4 {
                            return bad("cvbl: vertex words are not whole vertices");
                        }
                        for v in [1, (32 + 4 * words.len()) as u32, count as u32, 0, *format, 0, 0, 0] {
                            put(&mut out, v);
                        }
                        for v in words {
                            put(&mut out, *v);
                        }
                    }
                }
            }
            for v in [0x8000, 0, 0, 0] {
                put(&mut out, v);
            }
        }
        Ok(out)
    }

    /// Calls `f` on the position (x, y, z) of every 48-byte (textured, word 8..11) and 16-byte
    /// (plain, word 0..3) vertex. Weighted 80-byte vertices are left alone (layout not traced).
    pub fn for_each_position(&mut self, mut f: impl FnMut(&mut [f32; 3])) {
        for m in &mut self.meshes {
            for c in &mut m.chunks {
                if let Chunk::Vertices { stride, words, .. } = c {
                    let at = match *stride {
                        48 => 8,
                        16 => 0,
                        _ => continue,
                    };
                    for v in words.chunks_mut(*stride / 4) {
                        let mut p = [f32::from_bits(v[at]), f32::from_bits(v[at + 1]), f32::from_bits(v[at + 2])];
                        f(&mut p);
                        for k in 0..3 {
                            v[at + k] = p[k].to_bits();
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

    #[test]
    fn synthetic_round_trip() {
        let mesh = |flags, stride: usize| CvMesh {
            flags,
            texture: 1,
            chunks: vec![
                Chunk::Matrix { joint: 3, slot: 0, data: vec![0x3F80_0000; 28] },
                Chunk::Vertices { format: 0x3C, stride, words: (0..(stride / 4 * 3) as u32).collect() },
            ],
        };
        for (layout, patches) in [(2u32, vec![]), (0x1_0001, vec![[1, 2, 3, 4, 0, 0, 0, 0]])] {
            let c = Cvbl { layout, patches, meshes: vec![mesh(0x80000, 48), mesh(0xA0000, 16)] };
            let b = c.write().unwrap();
            let back = Cvbl::parse(&b).unwrap();
            assert_eq!(back, c);
            assert_eq!(back.write().unwrap(), b);
        }
    }

    #[test]
    fn positions_scale() {
        let words: Vec<u32> = (0..12).map(|i| (i as f32).to_bits()).collect();
        let mut c = Cvbl { layout: 0, patches: vec![], meshes: vec![CvMesh { flags: 0x80000, texture: 0, chunks: vec![Chunk::Vertices { format: 0x3C, stride: 48, words }] }] };
        c.for_each_position(|p| p.iter_mut().for_each(|x| *x *= 2.0));
        let Chunk::Vertices { words, .. } = &c.meshes[0].chunks[0] else { panic!() };
        assert_eq!(f32::from_bits(words[8]), 16.0);
        assert_eq!(f32::from_bits(words[11]), 11.0);
        assert_eq!(f32::from_bits(words[0]), 0.0);
    }

    /// Every `.cvbl` in `KH1_TEST_DATA/cvbl/` round-trips byte for byte.
    #[test]
    fn game_files_round_trip() {
        let Some(dir) = crate::testdata::dir() else { return };
        let Ok(rd) = std::fs::read_dir(dir.join("cvbl")) else { return };
        let mut n = 0;
        for e in rd.flatten() {
            let data = std::fs::read(e.path()).unwrap();
            let c = Cvbl::parse(&data).unwrap_or_else(|err| panic!("{}: {err}", e.path().display()));
            assert!(c.write().unwrap() == data, "{}", e.path().display());
            n += 1;
        }
        eprintln!("{n} .cvbl files round-tripped");
    }
}
