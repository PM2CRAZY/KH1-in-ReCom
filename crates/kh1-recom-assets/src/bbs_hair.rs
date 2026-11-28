//! BBS FM's kid Riku's hair on Re:CoM Riku's head (the third hair option, `riku_hair = bbs`; fork owner, 2026-11-24: "agora
//! quero implementar o cabelo do kid Riku do BBS"). It needs only the 1.5+2.5 collection (BBS FM's `bbs_fourth` archive); the swap
//! itself is [`crate::ddd_hair::with_hair`], as for [`crate::kh2_hair`].
//!
//! Source (notes: the workspace's `knowledge/BBS/bbs-kid-riku-hair-models.md`, Checked (files)): `chara/npc/n02di/00/n02di00.pmo`,
//! a PMO v1 (the format of `OpenKh.Bbs/Pmo.cs`; its skeleton is read by [`crate::pam::parse_skeleton`]), 35 bones, 1392 triangles,
//! positions in bind-pose model space (`s16 / 32768` times the header's scale 1.18934), weights `u8 / 128`. Its atlas is the PMO's
//! texture 0, `n02di00_00`, a 1024 x 1024 uncompressed DDS (the entry's HD asset `-0.dds`): face top left, the hair spray at the
//! top and about 20 hair shards on the right, the body below.
//!
//! Which triangles are hair: the note's colour rule (the triangle's centroid texel is grey-lavender: hue 0.55..0.85 with saturation
//! 0.05..0.5, or grey with saturation 0.05 or less, value 0.12..0.9) on the triangles of texture 0 in the head region (a corner at
//! or above `atama`'s y minus 0.03), then every UV island of the atlas that is mostly hair by that rule is taken whole (the island
//! rule of the DDD hair: it closes the holes the colour rule leaves, e.g. the 20 grey triangles of the crown).
//!
//! Fit (Assumption, the renders are the check): a vertex's bind position `P` (scaled) becomes `H + SCALE * (P - A) + SHIFT` (per axis) where
//! `A` is the kid's `atama` bone and `H` Re:CoM's `head` bone, both at rest. Bones (0.194.1): every vertex follows Re:CoM's `head`
//! rigidly (0.194.0 mapped the kid's `B_kami` / `L_kamiA` / ... to the nearest Re:CoM hair bone, whose clips split the short hair's
//! layers in game).

use crate::ddd_hair::{HairSource, SrcHair, SrcTri, SrcVert, HEAD_BONE};
use crate::math::Aff;
use crate::texture::{decode_dds, Image};
use crate::{bad, Error, Result};

/// The model (kid Riku, the field / NPC one). Checked (files).
pub const MODEL: &str = "chara/npc/n02di/00/n02di00.pmo";
/// The PMO's texture 0 and its HD asset in the archive entry. Checked (files).
pub const ATLAS_TEXTURE: &str = "n02di00_00";
/// See [`ATLAS_TEXTURE`].
pub const ATLAS_ASSET: &str = "-0.dds";
/// The kid's head bone.
pub const HEAD_BONE_BBS: &str = "atama";
/// The scale about the head bone per axis (x, y, z). 0.194.0 had the note's x1.1 on all three; in game the hair sat too high.
/// Height 1.0 keeps the crown at Re:CoM's own hair top; width stays wider (Re:CoM's face is about 1.2 times the kid's, at 1.0
/// its temples show beside the hair), fitted on the renders.
pub const SCALE: [f64; 3] = [1.15, 1.0, 1.1];
/// Offset added after the head alignment, in Re:CoM metres (x, y, z), fitted on the renders (the fringe to the brows).
pub const SHIFT: [f64; 3] = [0.0, -0.008, 0.0];
/// Passes of [`bleed`] on the atlas (pixels of shard colour grown into the black fill).
pub const BLEED: usize = 8;
/// A texel darker than this in every channel is fill (the hair's texels are 80 and up; the fill is 0 with a rim of 16..80
/// around the shards, Checked (files: a histogram of the atlas's hair area)).
pub const BLEED_BELOW: u8 = 72;
/// 0.198.12 (fork owner on 0.198.11: "cabelo do bbs parece escamado tem como melhorar?"): the kid's shards are painted with dark
/// roots, and the back of the hair is rows of shards, each row's tips over the next row's roots, so every row ends on a dark band.
/// [`soften`] lifts the hair's texels darker than this brightness (mean of r, g, b; the hair's median is 79, its 75th percentile
/// 100, its 10th 35, Checked (files: the filled atlas)) toward it; brighter texels are kept.
pub const SOFTEN_KNEE: f64 = 110.0;
/// The part of a dark texel's distance under [`SOFTEN_KNEE`] that [`soften`] keeps (0.5: 40 becomes 75, 80 becomes 95).
pub const SOFTEN_KEEP: f64 = 0.5;
/// Re:CoM's painted temple lock (the `head` triangles whose texels are the lock, 3-4 a side, centroids 7.8-9.2 cm out): |x| 6-11 cm,
/// y -1 to 9 cm, z -2 to 6 cm from Re:CoM's `head` (0.198.13's unpaint window; the test `sideburn_probe` measures the lock there).
pub const TEMPLE_WINDOW: [f64; 6] = [0.06, 0.11, -0.01, 0.09, -0.02, 0.06];
/// The kid's sideburn, in front of each ear: the hair's vertices in this window (head-relative after the fit, metres: least |x|, y
/// range, z range) are its root and its tip (two a side, Checked (files, test `sideburn_probe`): +x tip (77, -7, 31) mm, root (89, 27,
/// 16) mm; the same mirrored). The strand is the kid's islands 7 (front) and 17 (back): from the root and the tip up to four corners in
/// the hair 4.5-7.7 cm over the head bone.
/// 0.198.36 (fork owner on 0.198.34/35, a screenshot with an arrow at the pointed lock at Riku's right temple: "cabelo ta quase
/// perfeito do bbs essa costeleta ainda ta estranha, parece do wolverine, não acho que vc precisava ter tirado a costeleta pinta da
/// cabeça base"): every hair triangle with a corner in this window is left out (6 a side, see [`HairSource::hair`] here), and
/// Re:CoM's painted lock stays on the face as Re:CoM paints it (0.198.13 to 0.198.35 covered it and kept this strand, laid 3.5 mm
/// off the cheek).
pub const SIDEBURN: [f64; 5] = [0.06, -0.06, 0.03, -0.02, 0.06];
/// Corners this far under `atama` (model units) or higher are the head region.
pub const HEAD_MARGIN: f64 = 0.03;
/// The lining under the back of the hair (0.194.2): how far each vertex is pulled toward the head's centre (fraction).
pub const LINING: f64 = 0.08;
/// The head's centre for the lining, from `atama` (model units; x, y, z), the middle of the hair's bounds.
pub const LINING_CENTRE: [f64; 3] = [0.0, 0.08, -0.03];

/// The PMO and its atlas.
pub struct BbsHairSource {
    /// `n02di00.pmo`.
    pub pmo: Vec<u8>,
    /// `-0.dds`.
    pub dds: Vec<u8>,
    /// The kid's idle, walk and run clips ([`SWAY_CLIPS`]), when all three were read (none: the hair rigid on `head`, 0.194.1).
    pub sway: Option<[crate::pam::PamClip; 3]>,
    /// The sway's rig, made by [`HairSource::prepare_mdl`] from Re:CoM's model: per kid hair bone, the Re:CoM bone that carries it.
    rig: std::cell::RefCell<Option<SwayRig>>,
}

/// The hair's sway (0.197.3, item 6 of Stage 391; the workspace's `knowledge/BBS/bbs-kid-riku-hair-motion.md`, Checked (files)): in
/// BBS the kid's hair hangs on five bones under `atama`, keyed in every clip (idle ±1-2 deg, walk ±4-7, run ±13-20). Each takes one
/// Re:CoM hair bone whose parent is `head` (no bone is added: Riku keeps 102, `PL4002`), moved to the kid bone's pivot; its
/// clips turn it as the kid's clip turns the kid bone against `atama`, relative to the kid's idle at frame 0 (the fit's pose).
pub const SWAY_BONES: [&str; 5] = ["B_kami", "L_kamiA", "L_kamiB", "R_kamiA", "R_kamiB"];
/// The kid's clips (loose PAMs in `bbs_fourth`, one clip each, 30 fps): idle (60 frames), walk (30), run (20). Checked (files).
pub const SWAY_CLIPS: [&str; 3] = ["chara/npc/n02di/event_di/n02di00001e.pam", "chara/npc/n02di/event_di/n02di00002e.pam", "chara/npc/n02di/event_di/n02di00003e.pam"];

/// See [`SWAY_BONES`]: per kid hair bone (its index in the PMO), the Re:CoM bone, the conjugation into Re:CoM's head frame, and the
/// Re:CoM bone's rest local (rotation kept, origin moved to the pivot).
#[derive(Clone, Debug)]
pub struct SwayRig {
    /// (kid bone, Re:CoM bone, Re:CoM rest local).
    pub pairs: Vec<(usize, usize, Aff)>,
    /// `atama`'s rest world rotation times the inverse of Re:CoM `head`'s (row vectors: kid head frame → Re:CoM head frame).
    pub c: crate::math::M3,
}

fn inv3(m: &crate::math::M3) -> crate::math::M3 {
    let d = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]) + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    [
        [(m[1][1] * m[2][2] - m[1][2] * m[2][1]) / d, (m[0][2] * m[2][1] - m[0][1] * m[2][2]) / d, (m[0][1] * m[1][2] - m[0][2] * m[1][1]) / d],
        [(m[1][2] * m[2][0] - m[1][0] * m[2][2]) / d, (m[0][0] * m[2][2] - m[0][2] * m[2][0]) / d, (m[0][2] * m[1][0] - m[0][0] * m[1][2]) / d],
        [(m[1][0] * m[2][1] - m[1][1] * m[2][0]) / d, (m[0][1] * m[2][0] - m[0][0] * m[2][1]) / d, (m[0][0] * m[1][1] - m[0][1] * m[1][0]) / d],
    ]
}

impl BbsHairSource {
    /// From the player's BBS archives (next to the game, as the Terra reads).
    pub fn read(game: &std::path::Path, keys: &kh1_formats::egs::KeyTable) -> Result<BbsHairSource> {
        for a in crate::terra::ARCHIVES {
            let hed = game.join("Image").join("dt").join(format!("{a}.hed"));
            if !hed.is_file() {
                continue;
            }
            let arc = kh1_formats::egs::Archive::open(&hed)?;
            if let Some(e) = arc.find(MODEL) {
                let raw = arc.read_raw_entry(e)?;
                let (pmo, assets) = kh1_formats::egs::decode_entry_full(keys, &raw)?;
                let dds = assets.into_iter().find(|x| x.name == ATLAS_ASSET).ok_or_else(|| Error::Format(format!("BBS: {MODEL} has no {ATLAS_ASSET}")))?.data;
                // 0.197.3: the kid's three clips for the sway (all three or none)
                let clip = |name: &str| -> Option<crate::pam::PamClip> {
                    let e = arc.find(name)?;
                    let (d, _) = kh1_formats::egs::decode_entry_full(keys, &arc.read_raw_entry(e).ok()?).ok()?;
                    crate::pam::parse_pam(&d).ok()?.into_iter().next()
                };
                let sway = match (clip(SWAY_CLIPS[0]), clip(SWAY_CLIPS[1]), clip(SWAY_CLIPS[2])) {
                    (Some(a), Some(b), Some(c)) => Some([a, b, c]),
                    _ => None,
                };
                return Ok(BbsHairSource { pmo, dds, sway, rig: Default::default() });
            }
        }
        bad(format!("BBS: {MODEL} in none of the bbs archives"))
    }
}

/// One vertex: position (model space, scaled), UV, dominant bone index.
#[derive(Clone, Debug)]
pub struct PmoVert {
    /// Bind-pose position times the header scale.
    pub pos: [f64; 3],
    /// Texture coordinates (0..1 over the atlas).
    pub uv: [f64; 2],
    /// Index of the dominant-weight bone in the skeleton (none: the model has no skeleton).
    pub bone: usize,
}

/// One mesh chunk.
#[derive(Clone, Debug)]
pub struct PmoMesh {
    /// Its texture index.
    pub tex: usize,
    /// Its vertices.
    pub verts: Vec<PmoVert>,
    /// Its triangles (indices into `verts`).
    pub tris: Vec<[usize; 3]>,
}

/// A parsed PMO v1.
#[derive(Clone, Debug)]
pub struct Pmo {
    /// Texture names.
    pub textures: Vec<String>,
    /// Mesh chunks.
    pub meshes: Vec<PmoMesh>,
}

fn u16_at(b: &[u8], o: usize) -> Result<u16> {
    b.get(o..o + 2).map(|x| u16::from_le_bytes([x[0], x[1]])).ok_or_else(|| Error::Format("PMO: truncated".into()))
}
fn u32_at(b: &[u8], o: usize) -> Result<u32> {
    b.get(o..o + 4).map(|x| u32::from_le_bytes(x.try_into().unwrap())).ok_or_else(|| Error::Format("PMO: truncated".into()))
}
fn f32_at(b: &[u8], o: usize) -> Result<f32> {
    Ok(f32::from_bits(u32_at(b, o)?))
}
fn bits(v: u32, s: u32, n: u32) -> u32 {
    (v >> s) & ((1 << n) - 1)
}

/// Reads a PMO v1 (the layout of `OpenKh.Bbs/Pmo.cs`, `ReadMeshData`).
pub fn parse_pmo(b: &[u8]) -> Result<Pmo> {
    if b.get(..4) != Some(b"PMO\0") {
        return bad("PMO: no magic");
    }
    let ntex = *b.get(8).ok_or_else(|| Error::Format("PMO: short".into()))? as usize;
    let skel = u32_at(b, 0xC)? as usize;
    let (m0, m1) = (u32_at(b, 0x10)? as usize, u32_at(b, 0x1C)? as usize);
    let scale = f32_at(b, 0x18)? as f64;
    let mut textures = vec![];
    for i in 0..ntex {
        let o = 0xA0 + i * 0x20 + 4;
        let n = b.get(o..o + 12).ok_or_else(|| Error::Format("PMO: texture name outside".into()))?;
        textures.push(String::from_utf8_lossy(&n[..n.iter().position(|&c| c == 0).unwrap_or(12)]).into_owned());
    }
    let mut meshes = vec![];
    for off in [m0, m1] {
        if off == 0 {
            continue;
        }
        let mut p = off;
        loop {
            let vcnt = u16_at(b, p)? as usize;
            let tex = *b.get(p + 2).ok_or_else(|| Error::Format("PMO: short".into()))? as usize;
            let vsz = *b.get(p + 3).ok_or_else(|| Error::Format("PMO: short".into()))? as usize;
            let fl = u32_at(b, p + 4)?;
            let nstrips = *b.get(p + 9).ok_or_else(|| Error::Format("PMO: short".into()))? as usize;
            p += 12;
            if vcnt == 0 {
                break;
            }
            let mut bt = [0u8; 8];
            if skel != 0 {
                bt.copy_from_slice(b.get(p..p + 8).ok_or_else(|| Error::Format("PMO: bone table".into()))?);
                p += 8;
            }
            if bits(fl, 24, 1) != 0 {
                p += 4;
            }
            let mut strips = vec![];
            for i in 0..nstrips {
                strips.push(u16_at(b, p + 2 * i)? as usize);
            }
            p += 2 * nstrips;
            let (tf, cf, nf, pf, wf, nw, prim) = (bits(fl, 0, 2), bits(fl, 2, 3), bits(fl, 5, 2), bits(fl, 7, 2), bits(fl, 9, 2), bits(fl, 14, 3) as usize, bits(fl, 28, 4));
            let mut verts = Vec::with_capacity(vcnt);
            for v in 0..vcnt {
                let s = p + v * vsz;
                let mut q = s;
                let mut w = vec![];
                if skel != 0 && wf != 0 {
                    for _ in 0..=nw {
                        match wf {
                            1 => {
                                w.push(*b.get(q).ok_or_else(|| Error::Format("PMO: vertex".into()))? as f64 / 128.0);
                                q += 1;
                            }
                            2 => {
                                w.push(u16_at(b, q)? as f64 / 32768.0);
                                q += 2;
                            }
                            _ => {
                                w.push(f32_at(b, q)? as f64);
                                q += 4;
                            }
                        }
                    }
                }
                let al = |q: usize, a: usize| q + ((a - ((q - s) & (a - 1))) & (a - 1));
                let mut uv = [0.0; 2];
                match tf {
                    1 => {
                        uv = [*b.get(q).ok_or_else(|| Error::Format("PMO: vertex".into()))? as f64 / 128.0, *b.get(q + 1).ok_or_else(|| Error::Format("PMO: vertex".into()))? as f64 / 128.0];
                        q += 2;
                    }
                    2 => {
                        q = al(q, 2);
                        uv = [u16_at(b, q)? as f64 / 32768.0, u16_at(b, q + 2)? as f64 / 32768.0];
                        q += 4;
                    }
                    3 => {
                        q = al(q, 4);
                        uv = [f32_at(b, q)? as f64, f32_at(b, q + 4)? as f64];
                        q += 8;
                    }
                    _ => {}
                }
                match cf {
                    7 => {
                        q = al(q, 4) + 4;
                    }
                    4..=6 => {
                        q = al(q, 2) + 2;
                    }
                    _ => {}
                }
                if nf != 0 {
                    match nf {
                        1 => q += 3,
                        2 => q = al(q, 2) + 6,
                        _ => q = al(q, 4) + 12,
                    }
                }
                let pos = match pf {
                    1 => {
                        let g = |k: usize| -> Result<f64> { Ok(*b.get(q + k).ok_or_else(|| Error::Format("PMO: vertex".into()))? as i8 as f64 / 128.0) };
                        [g(0)?, g(1)?, g(2)?]
                    }
                    2 => {
                        q = al(q, 2);
                        let g = |k: usize| -> Result<f64> { Ok(u16_at(b, q + 2 * k)? as i16 as f64 / 32768.0) };
                        [g(0)?, g(1)?, g(2)?]
                    }
                    _ => {
                        q = al(q, 4);
                        [f32_at(b, q)? as f64, f32_at(b, q + 4)? as f64, f32_at(b, q + 8)? as f64]
                    }
                };
                let dom = w.iter().enumerate().fold((0usize, f64::MIN), |a, (k, &x)| if x > a.1 { (k, x) } else { a }).0;
                verts.push(PmoVert { pos: pos.map(|x| x * scale), uv, bone: bt[dom.min(7)] as usize });
            }
            p += vcnt * vsz;
            p += p % 4;
            let n = verts.len();
            let mut tris = vec![];
            if !strips.is_empty() {
                let mut base = 0;
                for c in &strips {
                    for s in 0..c.saturating_sub(2) {
                        tris.push(if s % 2 == 0 { [base + s, base + s + 1, base + s + 2] } else { [base + s, base + s + 2, base + s + 1] });
                    }
                    base += c;
                }
            } else if prim == 4 {
                for s in 0..n.saturating_sub(2) {
                    tris.push(if s % 2 == 0 { [s, s + 1, s + 2] } else { [s + 1, s, s + 2] });
                }
            } else if prim == 3 {
                let mut i = 0;
                while i + 2 < n {
                    tris.push([i, i + 1, i + 2]);
                    i += 3;
                }
            }
            meshes.push(PmoMesh { tex, verts, tris });
        }
    }
    Ok(Pmo { textures, meshes })
}

fn hsv(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
    let d = mx - mn;
    let s = if mx == 0.0 { 0.0 } else { d / mx };
    let h = if d == 0.0 {
        0.0
    } else if mx == r {
        ((g - b) / d).rem_euclid(6.0) / 6.0
    } else if mx == g {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };
    (h, s, mx)
}

/// The note's colour rule for one texel.
pub fn is_hair_texel(px: [u8; 4]) -> bool {
    let (h, s, v) = hsv(px[0] as f64 / 255.0, px[1] as f64 / 255.0, px[2] as f64 / 255.0);
    (((0.55..=0.85).contains(&h) && 0.05 < s && s < 0.5) || s <= 0.05) && 0.12 < v && v < 0.9
}

fn texel(img: &Image, uv: [f64; 2]) -> [u8; 4] {
    let (u, v) = (uv[0].rem_euclid(1.0), uv[1].rem_euclid(1.0));
    img.px[((v * img.h as f64) as usize % img.h) * img.w + ((u * img.w as f64) as usize % img.w)]
}

/// The hair triangles of the head region: `(mesh, triangle)` pairs, by the colour rule then whole UV islands, and how many the
/// colour rule alone gave.
pub fn classify(pmo: &Pmo, skel_atama_y: f64, atlas: &Image) -> (Vec<(usize, usize)>, usize) {
    let tex0 = pmo.textures.iter().position(|n| n == ATLAS_TEXTURE);
    let mut cand: Vec<(usize, usize)> = vec![];
    for (mi, m) in pmo.meshes.iter().enumerate() {
        if Some(m.tex) != tex0 {
            continue;
        }
        for (ti, t) in m.tris.iter().enumerate() {
            let p = t.map(|i| m.verts[i].pos);
            if p[0] == p[1] || p[1] == p[2] || p[0] == p[2] {
                continue;
            }
            if p.iter().map(|q| q[1]).fold(f64::MIN, f64::max) < skel_atama_y - HEAD_MARGIN {
                continue;
            }
            cand.push((mi, ti));
        }
    }
    let by_colour: Vec<bool> = cand
        .iter()
        .map(|&(mi, ti)| {
            let m = &pmo.meshes[mi];
            let uv = {
                let t = m.tris[ti];
                let u: [f64; 2] = [0, 1].map(|k| (m.verts[t[0]].uv[k] + m.verts[t[1]].uv[k] + m.verts[t[2]].uv[k]) / 3.0);
                u
            };
            is_hair_texel(texel(atlas, uv))
        })
        .collect();
    let q = |x: f64| (x * 4096.0).round() as i32;
    let corners: Vec<[(i32, i32); 3]> = cand.iter().map(|&(mi, ti)| pmo.meshes[mi].tris[ti].map(|i| (q(pmo.meshes[mi].verts[i].uv[0]), q(pmo.meshes[mi].verts[i].uv[1])))).collect();
    let (isl, n) = crate::ddd_hair::islands(&corners);
    let (mut hair, mut all) = (vec![0usize; n], vec![0usize; n]);
    for (k, &i) in isl.iter().enumerate() {
        all[i] += 1;
        hair[i] += by_colour[k] as usize;
    }
    let out = cand.iter().enumerate().filter(|(k, _)| hair[isl[*k]] * 2 > all[isl[*k]]).map(|(_, &c)| c).collect();
    (out, by_colour.iter().filter(|&&x| x).count())
}

impl BbsHairSource {
    /// The sway's rig for Re:CoM's model `mdl` (none without the kid's clips).
    pub fn sway_rig(&self, mdl: &[u8]) -> Result<Option<SwayRig>> {
        if self.sway.is_none() {
            return Ok(None);
        }
        let sk = crate::mdl::RecomSkeleton::parse(mdl)?;
        let world = sk.rest_world();
        let head = sk.find(HEAD_BONE).ok_or_else(|| Error::Format("hair: no head bone".into()))?;
        let skel = crate::pam::parse_skeleton(&self.pmo)?;
        let kw = skel.rest_world();
        let atama = skel.find(HEAD_BONE_BBS).ok_or_else(|| Error::Format("BBS: no atama bone".into()))?;
        let (a, h) = (kw[atama].t, world[head].t);
        let frame = |p: [f64; 3]| [0, 1, 2].map(|k| h[k] + SCALE[k] * (p[k] - a[k]) + SHIFT[k]);
        let hinv = inv3(&world[head].r);
        // Re:CoM's free hair bones: children of `head` with "hair" in the name and no child (`f_hair` carries the front locks).
        let mut free: Vec<usize> = (0..sk.bones.len()).filter(|&i| sk.bones[i].parent as usize == head && sk.bones[i].name.contains("hair") && !sk.bones.iter().any(|b| b.parent as usize == i)).collect();
        let mut pairs = vec![];
        for name in SWAY_BONES {
            let kb = skel.find(name).ok_or_else(|| Error::Format(format!("BBS: no bone {name}")))?;
            let pivot = frame(kw[kb].t);
            let (k, &rb) = free.iter().enumerate().min_by(|x, y| crate::math::dist(world[*x.1].t, pivot).total_cmp(&crate::math::dist(world[*y.1].t, pivot))).ok_or_else(|| Error::Format("hair: too few free Re:CoM hair bones".into()))?;
            free.remove(k);
            let t = crate::math::vm([pivot[0] - h[0], pivot[1] - h[1], pivot[2] - h[2]], &hinv);
            pairs.push((kb, rb, Aff { r: sk.bones[rb].local.r, t }));
        }
        let c = crate::math::m3(&crate::math::unscale(&kw[atama].r).0, &crate::math::tr(&crate::math::unscale(&world[head].r).0));
        Ok(Some(SwayRig { pairs, c }))
    }

    /// The Re:CoM local rotation of `rig`'s pair `k` at kid frame `t` of `clip` (see [`SWAY_BONES`]).
    pub fn sway_local(&self, rig: &SwayRig, k: usize, clip: &crate::pam::PamClip, t: f64) -> Option<crate::math::M3> {
        use crate::math::{m3, rot_xyz, tr};
        let idle = &self.sway.as_ref()?[0];
        let (kb, _, rest) = rig.pairs[k];
        // 0.197.7: between two whole frames the kid's angles are interpolated the short way (`R_kamiB`'s z crosses ±180 deg
        // between frames 58 and 59 of the idle: the raw interpolation turned the lock 72 deg at frame 58.8, seen in game as a jerk).
        let rot = |c: &crate::pam::PamClip, t: f64| {
            let last = (c.frames.max(1) - 1) as f64;
            let t = t.clamp(0.0, last);
            let (f0, f1) = (t.floor(), t.ceil().min(last));
            let (_, a, _) = c.local(kb, f0);
            let (_, b, _) = c.local(kb, f1);
            let w = t - f0;
            let tau = std::f64::consts::TAU;
            let e = [0, 1, 2].map(|k| {
                let d = (b[k] - a[k] + std::f64::consts::PI).rem_euclid(tau) - std::f64::consts::PI;
                a[k] + w * d
            });
            rot_xyz(e[0], e[1], e[2])
        };
        // the turn in the kid's head frame about the bone's pivot, from the idle's first frame; then in Re:CoM's head frame
        let q = m3(&tr(&rot(idle, 0.0)), &rot(clip, t));
        let qr = m3(&m3(&tr(&rig.c), &q), &rig.c);
        Some(m3(&rest.r, &qr))
    }
}

/// Signed distance (+ away from `centre`) of `p` to the plane of the `face` triangle whose centroid is nearest, and that plane's unit
/// normal (pointing away from `centre`).
pub fn plane_dist(face: &[[[f64; 3]; 3]], p: [f64; 3], centre: [f64; 3]) -> Option<(f64, [f64; 3])> {
    let f = face.iter().min_by(|a, b| {
        let d = |t: &[[f64; 3]; 3]| (0..3).map(|k| ((t[0][k] + t[1][k] + t[2][k]) / 3.0 - p[k]).powi(2)).sum::<f64>();
        d(a).total_cmp(&d(b))
    })?;
    let (u, w) = ([f[1][0] - f[0][0], f[1][1] - f[0][1], f[1][2] - f[0][2]], [f[2][0] - f[0][0], f[2][1] - f[0][1], f[2][2] - f[0][2]]);
    let n = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
    let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if l < 1e-12 {
        return None;
    }
    let mut n = n.map(|x| x / l);
    let c = [0, 1, 2].map(|k| (f[0][k] + f[1][k] + f[2][k]) / 3.0);
    if (0..3).map(|k| n[k] * (c[k] - centre[k])).sum::<f64>() < 0.0 {
        n = n.map(|x| -x);
    }
    Some(((0..3).map(|k| n[k] * (p[k] - f[0][k])).sum::<f64>(), n))
}

impl BbsHairSource {
    /// [`HairSource::bake_ann`] on any pack's slots: `map` = (slot, KH2 role) as [`crate::kh2_hair_rig::CLIP_MAP`] (Riku's
    /// `PL0101.ANN`); 0.198.33: the battle's Dark Riku's own slots ([`crate::riku_skin::dark_slots`]), so his battle stance (slot 21)
    /// sways as the idle, not as the run Riku's slot 21 is. Uses the rig of the model last given to [`HairSource::prepare_mdl`].
    pub fn bake_ann_slots(&self, ann: &[u8], map: &[(usize, usize)]) -> Result<Option<Vec<u8>>> {
        use crate::kh2_hair_rig::{KH2_RUN, KH2_WALK};
        let rig = self.rig.borrow();
        let (Some(rig), Some(clips)) = (rig.as_ref(), self.sway.as_ref()) else { return Ok(None) };
        let mut a = crate::ann::Ann::parse(ann)?;
        for (slot, s) in a.slots.iter_mut().enumerate() {
            let Some(c) = s.clip.as_mut() else { continue };
            // the clip's role as the KH2 hair reads it (Assumption there): walk, run, else the kid's idle
            let src = match map.iter().find(|x| x.0 == slot).map(|x| x.1) {
                Some(KH2_WALK) => &clips[1],
                Some(KH2_RUN) => &clips[2],
                _ => &clips[0],
            };
            let (chans, frames, fps) = (c.chans(), c.frames.max(1) as usize, c.fps().max(1) as f64);
            let n = src.frames.max(1) as f64;
            // a looped clip runs a whole number of the kid's loops stretched to its length; a one-shot the kid's own time, looped
            let loops = (frames as f64 / fps * 30.0 / n).round().max(1.0);
            let looped = c.looped();
            let time = |f: usize| if looped { (f as f64 / frames as f64 * loops * n) % n } else { (f as f64 / fps * 30.0) % n };
            for (k, &(_, rb, rest)) in rig.pairs.iter().enumerate() {
                if rb >= c.bones as usize {
                    return bad(format!("hair: bone {rb} past clip {slot}'s {}", c.bones));
                }
                let mut vals = vec![vec![1.0f64; frames]; chans];
                let mut prev = Some(crate::math::euler_xyz(&rest.r, None));
                for f in 0..frames {
                    let r = self.sway_local(rig, k, src, time(f)).unwrap_or(rest.r);
                    let e = crate::math::euler_xyz(&r, prev);
                    prev = Some(e);
                    for i in 0..3 {
                        vals[i][f] = rest.t[i];
                        vals[3 + i][f] = e[i].to_degrees();
                    }
                }
                for (ci, v) in vals.iter().enumerate() {
                    let eps = [1e-7, 1e-5, 1e-6][ci / 3];
                    c.channels[rb * chans + ci] = crate::convert::encode_channel(v, eps);
                }
            }
        }
        Ok(Some(a.to_bytes()))
    }
}

impl HairSource for BbsHairSource {
    // (no `painted_temple_window`: 0.198.13 to 0.198.35 covered Re:CoM's painted temple lock, [`TEMPLE_WINDOW`] (fork owner then: "tem a
    // do cabelo mesmo e tem a da cabeça base ao lado, ficou estranho"); since 0.198.36 the painted lock is the one that stays, [`SIDEBURN`])
    fn prepare_mdl(&self, mdl: &[u8]) -> Result<Vec<u8>> {
        let rig = self.sway_rig(mdl)?;
        let mut out = mdl.to_vec();
        if let Some(rig) = &rig {
            crate::kh2_hair_rig::patch_rest(&mut out, &rig.pairs.iter().map(|x| (x.1, x.2)).collect::<Vec<_>>())?;
        }
        *self.rig.borrow_mut() = rig;
        Ok(out)
    }

    fn bake_ann(&self, ann: &[u8], _mdl: &[u8]) -> Result<Option<Vec<u8>>> {
        self.bake_ann_slots(ann, &crate::kh2_hair_rig::CLIP_MAP)
    }

    fn hair(&self, names: &[String], world: &[Aff]) -> Result<SrcHair> {
        let pmo = parse_pmo(&self.pmo)?;
        let skel = crate::pam::parse_skeleton(&self.pmo)?;
        let kw = skel.rest_world();
        let atama = skel.find(HEAD_BONE_BBS).ok_or_else(|| Error::Format("BBS: no atama bone".into()))?;
        let img = decode_dds(&self.dds)?;
        let head = names.iter().position(|n| n == HEAD_BONE).ok_or_else(|| Error::Format("hair: no head bone".into()))?;
        let (a, h) = (kw[atama].t, world[head].t);
        let frame = |p: [f64; 3]| [0, 1, 2].map(|k| h[k] + SCALE[k] * (p[k] - a[k]) + SHIFT[k]);
        let (hair, _) = classify(&pmo, a[1], &img);
        // 0.198.36: the kid's sideburn strand left out, Re:CoM's painted lock shows in its place ([`SIDEBURN`])
        let hair = without_sideburn(&pmo, hair, a);
        // 0.194.1: every corner on `head` (rigid). Re:CoM's hair bones are keyed for its long hair (idle: `l_hair4` / `r_hair4` 18 to
        // 19 deg off rest against the head; run: `b_hair2` 69 deg); on the kid's short closed shell they split the layers (side tufts
        // off the face, the nape's under-layer spread sideways, seen in game on 0.194.0).
        // 0.197.3: with the sway, the vertices whose dominant bone is a kid hair bone follow its Re:CoM bone.
        let mut to_recom: Vec<usize> = vec![head; skel.bones.len()];
        if let Some(rig) = self.rig.borrow().as_ref() {
            for &(kb, rb, _) in &rig.pairs {
                to_recom[kb] = rb;
            }
        }
        let q = |x: f64| (x * 4096.0).round() as i32;
        let corners: Vec<[(i32, i32); 3]> = hair.iter().map(|&(mi, ti)| pmo.meshes[mi].tris[ti].map(|i| (q(pmo.meshes[mi].verts[i].uv[0]), q(pmo.meshes[mi].verts[i].uv[1])))).collect();
        let (isl, n) = crate::ddd_hair::islands(&corners);
        let mut tris = Vec::with_capacity(hair.len());
        for (k, &(mi, ti)) in hair.iter().enumerate() {
            let m = &pmo.meshes[mi];
            let mk = |i: usize| -> Result<SrcVert> {
                let v = &m.verts[i];
                let bone = to_recom.get(v.bone).copied().ok_or_else(|| Error::Format(format!("BBS: bone {} out of range", v.bone)))?;
                Ok(SrcVert { pos: frame(v.pos), uv: v.uv, bone: names[bone].clone() })
            };
            let t = m.tris[ti];
            tris.push(SrcTri { v: [mk(t[0])?, mk(t[1])?, mk(t[2])?], island: isl[k], cont: false, lining: false });
        }
        // 0.194.2: a lining under the back of the hair. Re:CoM's face is a one-sided mask (its back and top were his hair, now
        // removed), so from behind nothing is drawn under the kid's thin shards and a crack between them showed the background
        // (fork owner's screenshot, 0.194.1). The hair triangles whose centroid is behind `atama` are written again, pulled
        // [`LINING`] of the way toward the head's centre, same UVs and island (no new atlas space).
        let c = [a[0], a[1] + LINING_CENTRE[1], a[2] + LINING_CENTRE[2]];
        for (k, &(mi, ti)) in hair.iter().enumerate() {
            let m = &pmo.meshes[mi];
            let t = m.tris[ti];
            if (0..3).map(|j| m.verts[t[j]].pos[2]).sum::<f64>() / 3.0 >= a[2] {
                continue;
            }
            let mk = |i: usize| {
                let p = m.verts[i].pos;
                SrcVert { pos: frame([0, 1, 2].map(|j| c[j] + (1.0 - LINING) * (p[j] - c[j]))), uv: m.verts[i].uv, bone: names[head].clone() }
            };
            tris.push(SrcTri { v: [mk(t[0]), mk(t[1]), mk(t[2])], island: isl[k], cont: false, lining: true });
        }
        // 0.194.3: one-sided, as BBS draws it (two-sided, the shell's inner layer, painted dark and facing in, showed from outside as
        // flat dark triangles, fork owner's screenshots); the black fill between the atlas's shards is filled with the shards'
        // colour (strand tips at an island's edge took it in when the islands were scaled into Re:CoM's atlas).
        // 0.198.12: the shards' dark roots lifted ([`soften`]).
        Ok(SrcHair { tris, islands: n, skin: 0, atlas: soften(&bleed(&img, BLEED)), one_sided: true })
    }
}

/// 0.198.36 ([`SIDEBURN`]): the hair triangles (as [`classify`] gives them) without the kid's sideburn strand: every triangle of a UV island
/// that has a corner in the [`SIDEBURN`] window after the fit (`a` = `atama`'s rest position; the window is head-relative, so the fit's
/// scale and shift about `atama` are all it needs). The whole island, not only the triangles with a corner there: the strand's top
/// triangle (4.5-7.7 cm over the head bone) has none and lay across the temple's skin (1.9 mm out to 0.2 mm in, test `sideburn_probe`).
pub fn without_sideburn(pmo: &Pmo, hair: Vec<(usize, usize)>, a: [f64; 3]) -> Vec<(usize, usize)> {
    let in_window = |p: [f64; 3]| {
        let r = [0, 1, 2].map(|k| SCALE[k] * (p[k] - a[k]) + SHIFT[k]);
        r[0].abs() >= SIDEBURN[0] && (SIDEBURN[1]..=SIDEBURN[2]).contains(&r[1]) && (SIDEBURN[3]..=SIDEBURN[4]).contains(&r[2])
    };
    let q = |x: f64| (x * 4096.0).round() as i32;
    let corners: Vec<[(i32, i32); 3]> = hair.iter().map(|&(mi, ti)| pmo.meshes[mi].tris[ti].map(|i| (q(pmo.meshes[mi].verts[i].uv[0]), q(pmo.meshes[mi].verts[i].uv[1])))).collect();
    let (isl, _) = crate::ddd_hair::islands(&corners);
    let strand: std::collections::HashSet<usize> = hair.iter().zip(&isl).filter(|(&(mi, ti), _)| pmo.meshes[mi].tris[ti].iter().any(|&i| in_window(pmo.meshes[mi].verts[i].pos))).map(|(_, &k)| k).collect();
    hair.into_iter().zip(isl).filter(|(_, k)| !strand.contains(k)).map(|(t, _)| t).collect()
}

/// The hair's texels ([`is_hair_texel`]) darker than [`SOFTEN_KNEE`] moved toward it, keeping [`SOFTEN_KEEP`] of the distance; the
/// colour is scaled as a whole (hue kept). Other texels (the kid's skin, eyes, clothes, which the hair does not use) are unchanged.
pub fn soften(img: &Image) -> Image {
    let mut out = img.clone();
    for p in out.px.iter_mut() {
        if !is_hair_texel(*p) {
            continue;
        }
        let l = (p[0] as f64 + p[1] as f64 + p[2] as f64) / 3.0;
        if l >= SOFTEN_KNEE || l <= 0.0 {
            continue;
        }
        let k = (SOFTEN_KNEE - (SOFTEN_KNEE - l) * SOFTEN_KEEP) / l;
        for c in p.iter_mut().take(3) {
            *c = (*c as f64 * k).round().min(255.0) as u8;
        }
    }
    out
}

/// The atlas with its black fill and its dark anti-aliased rim (every channel under [`BLEED_BELOW`]) replaced, `passes` times, by the mean of its non-fill 8 neighbours.
pub fn bleed(img: &Image, passes: usize) -> Image {
    let mut out = img.clone();
    let fill = |p: [u8; 4]| p[0].max(p[1]).max(p[2]) < BLEED_BELOW;
    let mut is_fill: Vec<bool> = out.px.iter().map(|&p| fill(p)).collect();
    for _ in 0..passes {
        let mut next = out.px.clone();
        let mut now = is_fill.clone();
        for y in 0..out.h {
            for x in 0..out.w {
                let i = y * out.w + x;
                if !is_fill[i] {
                    continue;
                }
                let (mut sum, mut n) = ([0u32; 3], 0u32);
                for dy in -1i64..=1 {
                    for dx in -1i64..=1 {
                        let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                        if (dx, dy) == (0, 0) || nx < 0 || ny < 0 || nx >= out.w as i64 || ny >= out.h as i64 {
                            continue;
                        }
                        let j = ny as usize * out.w + nx as usize;
                        if !is_fill[j] {
                            for k in 0..3 {
                                sum[k] += out.px[j][k] as u32;
                            }
                            n += 1;
                        }
                    }
                }
                if n > 0 {
                    next[i] = [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8, out.px[i][3]];
                    now[i] = false;
                }
            }
        }
        out.px = next;
        is_fill = now;
    }
    out
}

/// The lining's triangles: the hair triangles whose centroid is behind `atama` (`a` = its rest position).
pub fn lining_count(pmo: &Pmo, hair: &[(usize, usize)], a: [f64; 3]) -> usize {
    hair.iter().filter(|&&(mi, ti)| { let m = &pmo.meshes[mi]; let t = m.tris[ti]; (0..3).map(|j| m.verts[t[j]].pos[2]).sum::<f64>() / 3.0 < a[2] }).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ddd_hair::{tests as dt, with_hair, HairTarget};
    use crate::install;
    use crate::mdl::RecomSkeleton;

    /// The new atlas's name.
    pub const TARGET: HairTarget = crate::ddd_hair::RIKU_0101_BBS;

    #[test]
    fn soften_lifts_only_dark_hair_texels() {
        let img = Image { w: 4, h: 1, px: vec![[40, 38, 46, 255], [140, 136, 150, 255], [200, 120, 90, 255], [80, 78, 86, 255]] };
        let out = soften(&img);
        let l = |p: [u8; 4]| (p[0] as f64 + p[1] as f64 + p[2] as f64) / 3.0;
        assert!((l(out.px[0]) - 75.0).abs() < 1.5, "40 -> 75: {:?}", out.px[0]);
        assert!((l(out.px[3]) - 95.0).abs() < 1.5, "80 -> 95: {:?}", out.px[3]);
        assert_eq!(out.px[1], img.px[1], "above the knee: kept");
        assert_eq!(out.px[2], img.px[2], "skin: kept");
        assert!(out.px[0][2] > out.px[0][0], "hue kept");
    }

    /// 0.198.13: every model's BBS hair picture has its own name (the pictures differ: their temple patches).
    #[test]
    fn bbs_hair_pictures_have_their_own_names() {
        use crate::ddd_hair::{RIKU_0101_BBS, RIKU_0111_BBS, RIKU_0201_BBS, RIKU_0211_BBS};
        let n: std::collections::HashSet<&str> = [RIKU_0101_BBS, RIKU_0111_BBS, RIKU_0201_BBS, RIKU_0211_BBS].iter().map(|t| t.new_texture).collect();
        assert_eq!(n.len(), 4);
        assert!(n.iter().all(|x| x.len() <= 15));
    }

    #[test]
    fn inv3_inverts() {
        let m = [[2.0, 0.3, -0.1], [0.2, 1.5, 0.4], [-0.3, 0.1, 0.9]];
        let p = crate::math::m3(&m, &inv3(&m));
        for i in 0..3 {
            for j in 0..3 {
                assert!((p[i][j] - if i == j { 1.0 } else { 0.0 }).abs() < 1e-12);
            }
        }
    }

    /// 0.197.3 (owner's install, `RECOM_GAME`): the sway. The baked clips turn each of the five Re:CoM bones as the kid's clip turns
    /// its kid bone (decoded from the written channels, within 0.5 deg), no bone is added, the rest pose does not move the hair, and
    /// posed renders go to `out/bbs/pose/`.
    #[test]
    fn bbs_hair_sways_as_the_kid() {
        use crate::kh2_hair::tests::{posed_world, render_body};
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let mdl = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        let ann = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.ANN").unwrap();
        let e = arc.find("PLAYER/0101/PL0101.RTM").unwrap();
        let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
        let src = BbsHairSource::read(&g, &keys).unwrap();
        assert!(src.sway.is_some(), "the kid's three clips are read");
        let m1 = src.prepare_mdl(&mdl).unwrap();
        let rig = src.rig.borrow().clone().unwrap();
        let sk = RecomSkeleton::parse(&m1).unwrap();
        eprintln!("sway pairs (kid bone, Re:CoM bone): {:?}", rig.pairs.iter().map(|x| (x.0, sk.bones[x.1].name.clone())).collect::<Vec<_>>());
        assert_eq!(sk.bones.len(), RecomSkeleton::parse(&mdl).unwrap().bones.len(), "Re:CoM's bone count");
        let swap = with_hair(&m1, &rtm, &hd, &TARGET, &src).unwrap();
        let baked = crate::ann::Ann::parse(&src.bake_ann(&ann, &mdl).unwrap().unwrap()).unwrap();
        // the written channels against the kid: run (slot 3) and idle (slot 1)
        let clips = src.sway.as_ref().unwrap();
        let mut worst = 0.0f64;
        for (slot, kid) in [(1usize, &clips[0]), (3, &clips[2])] {
            let c = baked.slots[slot].clip.as_ref().unwrap();
            let (frames, fps, n) = (c.frames.max(1) as usize, c.fps() as f64, kid.frames as f64);
            let loops = (frames as f64 / fps * 30.0 / n).round().max(1.0);
            for f in 0..frames {
                let t = (f as f64 / frames as f64 * loops * n) % n;
                for (k, &(_, rb, _)) in rig.pairs.iter().enumerate() {
                    let want = src.sway_local(&rig, k, kid, t).unwrap();
                    let sm = c.sample(rb, f as f64);
                    let got = crate::math::rot_xyz(sm.r[0], sm.r[1], sm.r[2]);
                    worst = worst.max(crate::math::angle_deg(&crate::math::m3(&got, &crate::math::tr(&want))));
                }
            }
        }
        eprintln!("worst written vs kid: {worst:.3} deg");
        // 0.197.7: no jump between frames (the kid turns at most 0.3 deg a frame in idle, a few in run)
        for (slot, most) in [(1usize, 1.0), (31, 1.0), (79, 1.0), (2, 4.0), (3, 12.0)] {
            let c = baked.slots[slot].clip.as_ref().unwrap();
            for &(_, rb, _) in &rig.pairs {
                let r = |f: usize| { let s = c.sample(rb, f as f64); crate::math::rot_xyz(s.r[0], s.r[1], s.r[2]) };
                for f in 0..c.frames as usize - 1 {
                    let a = crate::math::angle_deg(&crate::math::m3(&r(f + 1), &crate::math::tr(&r(f))));
                    assert!(a < most, "clip {slot} bone {rb} frame {f}: {a:.1} deg step");
                }
            }
        }
        assert!(worst < 0.5, "{worst} deg");
        // renders
        let (p3, img) = dt::test_view(&swap);
        let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
        let base = dt::out_dir().join("bbs").join("pose");
        for (clip, frames) in [(1usize, vec![0.0, 30.0]), (2, vec![0.0, 8.0, 16.0]), (3, vec![0.0, 5.0, 10.0, 15.0])] {
            let c = baked.slots[clip].clip.as_ref().unwrap();
            for f in frames {
                let w = posed_world(&sk, c, f);
                let d = base.join(format!("c{clip}_f{f}"));
                std::fs::create_dir_all(&d).unwrap();
                for v in 0..4 {
                    render_body(&p3, &w, &names, &img, v, &d.join(format!("body_{v}.png")));
                }
            }
        }
    }

    /// 0.194.0 (owner's install, `RECOM_GAME`; the 1.5+2.5 collection only): the kid's hair classified (the note: 392 triangles by
    /// colour), swapped in, the debug renders written to the workspace's `local/assets/recom/riku-hair/out/bbs/`.
    #[test]
    fn swaps_rikus_hair_for_the_kid_s() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let src = BbsHairSource::read(&g, &keys).unwrap();
        let pmo = parse_pmo(&src.pmo).unwrap();
        assert_eq!(pmo.textures[0], ATLAS_TEXTURE);
        let sk = crate::pam::parse_skeleton(&src.pmo).unwrap();
        let atama = sk.find(HEAD_BONE_BBS).unwrap();
        let img = decode_dds(&src.dds).unwrap();
        assert_eq!((img.w, img.h), (1024, 1024));
        let (hair, by_colour) = classify(&pmo, sk.rest_world()[atama].t[1], &img);
        eprintln!("kid hair: {} triangles by islands, {by_colour} by colour; atama {:?}", hair.len(), sk.rest_world()[atama].t);
        let arc = install::recom_archive(&g).unwrap();
        let mdl = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        let e = arc.find("PLAYER/0101/PL0101.RTM").unwrap();
        let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
        let swap = with_hair(&mdl, &rtm, &hd, &TARGET, &src).unwrap();
        eprintln!("{:?}", swap.report);
        // 0.198.36: without the kid's sideburn strand (islands 7 and 17: 4 + 3 triangles a side)
        let n_all = hair.len();
        let hair = without_sideburn(&pmo, hair, sk.rest_world()[atama].t);
        assert_eq!(n_all - hair.len(), 14, "the sideburn strand");
        let sk_n = lining_count(&pmo, &hair, sk.rest_world()[atama].t);
        eprintln!("lining: {sk_n} triangles");
        assert_eq!(swap.report.ddd_hair, hair.len() + sk_n);
        let again = with_hair(&mdl, &rtm, &hd, &TARGET, &src).unwrap();
        assert!(swap.mdl == again.mdl && swap.dds == again.dds, "deterministic");
        crate::mdl::Mdl::parse(&swap.mdl).unwrap();
        let (p3, img) = dt::test_view(&swap);
        assert!(p3.texture_names().contains(&TARGET.new_texture.to_string()));
        let rsk = RecomSkeleton::parse(&swap.mdl).unwrap();
        let out = dt::out_dir().join("bbs");
        std::fs::create_dir_all(&out).unwrap();
        dt::render(&p3, &rsk.rest_world(), &img, &out);
        std::fs::write(out.join("rk_hair_bbs_atlas.png"), dt::png(img.w, img.h, &img.px.iter().flat_map(|p| [p[0], p[1], p[2]]).collect::<Vec<_>>())).unwrap();
        // the hair's bones
        let names: Vec<String> = rsk.bones.iter().map(|b| b.name.clone()).collect();
        let h = src.hair(&names, &rsk.rest_world()).unwrap();
        let mut by: std::collections::BTreeMap<String, usize> = Default::default();
        for t in &h.tris {
            for v in &t.v {
                *by.entry(v.bone.clone()).or_default() += 1;
            }
        }
        eprintln!("corner bones {by:?}");
        // debug: hair (first) and lining (last `sk_n`) triangles plus Re:CoM's kept head as a text dump for outside renders
        {
            let mut txt = String::new();
            for (k, t) in h.tris.iter().enumerate() {
                let g = if k >= h.tris.len() - sk_n { 's' } else { 'h' };
                txt += &format!("{g} {}\n", t.v.iter().flat_map(|v| v.pos).map(|x| format!("{x:.5}")).collect::<Vec<_>>().join(" "));
            }
            let w = rsk.rest_world();
            for t in dt::collect(&p3) {
                if !t.v.iter().all(|v| names[v.bone()] == "head" || names[v.bone()].starts_with("neck")) { continue; }
                txt += &format!("r {} {} {}\n", t.v.iter().flat_map(|v| w[v.bone()].point(v.pos().map(|x| x as f64))).map(|x| format!("{x:.5}")).collect::<Vec<_>>().join(" "), t.tex, t.v.iter().flat_map(|v| v.uv().map_or([0.0f32; 2], |u| [u[0], u[1]])).map(|x| format!("{x:.5}")).collect::<Vec<_>>().join(" "));
            }
            std::fs::write(out.join("tris.txt"), txt).unwrap();
        }
        let (mut lo, mut hi) = ([9.0f64; 3], [-9.0f64; 3]);
        for t in &h.tris {
            for v in &t.v {
                for k in 0..3 {
                    lo[k] = lo[k].min(v.pos[k]);
                    hi[k] = hi[k].max(v.pos[k]);
                }
            }
        }
        eprintln!("hair bounds {lo:?} {hi:?}");
        // posed renders with the torso (the same clips as the KH2 hair's): idle, run, attack, jump
        let ann = crate::ann::Ann::parse(&install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.ANN").unwrap()).unwrap();
        let base = out.join("pose");
        for (clip, frames) in [(1usize, vec![0.0, 50.0]), (3, vec![0.0, 6.0, 11.0, 17.0]), (14, vec![5.0]), (37, vec![10.0]), (41, vec![16.0])] {
            let Some(c) = ann.slots[clip].clip.as_ref() else { continue };
            for f in frames {
                let w = crate::kh2_hair::tests::posed_world(&rsk, c, f);
                let d = base.join(format!("c{clip}_f{f}"));
                std::fs::create_dir_all(&d).unwrap();
                let _ = crate::kh2_hair::tests::render_body(&p3, &w, &names, &img, 0, &d.join("body_side.png"));
                let _ = crate::kh2_hair::tests::render_body(&p3, &w, &names, &img, 1, &d.join("body_back.png"));
                let _ = crate::kh2_hair::tests::render_body(&p3, &w, &names, &img, 2, &d.join("body_front.png"));
            }
        }
    }

    /// Debug (0.194.0): the classified triangles drawn straight from the source atlas (no repacking), two-sided, front / back / top,
    /// to tell a classification or UV problem from an atlas-packing one.
    #[test]
    fn source_renders() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let keys = install::keys(&std::path::PathBuf::from(&g)).unwrap();
        let src = BbsHairSource::read(std::path::Path::new(&g), &keys).unwrap();
        let (arc, mdl) = { let arc = install::recom_archive(std::path::Path::new(&g)).unwrap(); let m = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap(); (arc, m) };
        let _ = arc;
        let rsk = RecomSkeleton::parse(&mdl).unwrap();
        let names: Vec<String> = rsk.bones.iter().map(|b| b.name.clone()).collect();
        let h = src.hair(&names, &rsk.rest_world()).unwrap();
        let out = dt::out_dir().join("bbs");
        std::fs::create_dir_all(&out).unwrap();
        for (name, hx, hs, vy, dx, ds) in [("front", 0usize, 1.0f64, 1usize, 2usize, 1.0f64), ("back", 0, -1.0, 1, 2, -1.0), ("side", 2, 1.0, 1, 0, -1.0)] {
            let size = 800usize;
            let mut tris: Vec<([[f64; 3]; 3], [[f64; 2]; 3])> = h.tris.iter().map(|t| (t.v.clone().map(|v| v.pos), t.v.clone().map(|v| v.uv))).collect();
            tris.sort_by(|a, b| { let d = |t: &([[f64; 3]; 3], _)| t.0.iter().map(|p| p[dx] * ds).sum::<f64>(); d(a).total_cmp(&d(b)) });
            let (mut lo, mut hi) = ([9.0f64; 2], [-9.0f64; 2]);
            for (p, _) in &tris { for q in p { let t = [q[hx] * hs, q[vy]]; for k in 0..2 { lo[k] = lo[k].min(t[k]); hi[k] = hi[k].max(t[k]); } } }
            let sc = 700.0 / (hi[0] - lo[0]).max(hi[1] - lo[1]);
            let mut rgb = vec![30u8; size * size * 3];
            for (p, uv) in &tris {
                let s: Vec<[f64; 2]> = p.iter().map(|q| [50.0 + (q[hx] * hs - lo[0]) * sc, 750.0 - (q[vy] - lo[1]) * sc]).collect();
                let det = (s[1][1] - s[2][1]) * (s[0][0] - s[2][0]) + (s[2][0] - s[1][0]) * (s[0][1] - s[2][1]);
                if det.abs() < 1e-9 { continue; }
                let (x0, x1) = (s.iter().map(|q| q[0]).fold(f64::MAX, f64::min).max(0.0) as usize, (s.iter().map(|q| q[0]).fold(f64::MIN, f64::max) as usize + 1).min(size - 1));
                let (y0, y1) = (s.iter().map(|q| q[1]).fold(f64::MAX, f64::min).max(0.0) as usize, (s.iter().map(|q| q[1]).fold(f64::MIN, f64::max) as usize + 1).min(size - 1));
                for y in y0..=y1 { for x in x0..=x1 {
                    let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
                    let l0 = ((s[1][1] - s[2][1]) * (px - s[2][0]) + (s[2][0] - s[1][0]) * (py - s[2][1])) / det;
                    let l1 = ((s[2][1] - s[0][1]) * (px - s[2][0]) + (s[0][0] - s[2][0]) * (py - s[2][1])) / det;
                    let l2 = 1.0 - l0 - l1;
                    if l0 < 0.0 || l1 < 0.0 || l2 < 0.0 { continue; }
                    let u = l0 * uv[0][0] + l1 * uv[1][0] + l2 * uv[2][0]; let v = l0 * uv[0][1] + l1 * uv[1][1] + l2 * uv[2][1];
                    let c = texel(&h.atlas, [u, v]);
                    rgb[3 * (y * size + x)..3 * (y * size + x) + 3].copy_from_slice(&c[..3]);
                } }
            }
            std::fs::write(out.join(format!("source_{name}.png")), dt::png(size, size, &rgb)).unwrap();
        }
    }
    /// 0.198.13: the owner's camera checks ([`crate::kh2_hair::tests::camera_checks`]) for the BBS hair on Riku's default (`0101`) and
    /// Halloween Town (`0111`) attires, `out/bbs_cam/<group>_<CAM_TAG>/` (the face from each side is where the painted lock was).
    #[test]
    fn bbs_camera_checks() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let src = BbsHairSource::read(&g, &keys).unwrap();
        let tag = std::env::var("CAM_TAG").unwrap_or_else(|_| "cam".into());
        for (grp, target) in [("0101", crate::ddd_hair::RIKU_0101_BBS), ("0111", crate::ddd_hair::RIKU_0111_BBS)] {
            let mdl = install::read_recom(&arc, &keys, &format!("PLAYER/{grp}/PL{grp}.MDL")).unwrap();
            let ann0 = install::read_recom(&arc, &keys, &format!("PLAYER/{grp}/PL{grp}.ANN")).or_else(|_| install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.ANN")).unwrap();
            let e = arc.find(&format!("PLAYER/{grp}/PL{grp}.RTM")).unwrap();
            let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
            let swap = with_hair(&src.prepare_mdl(&mdl).unwrap(), &rtm, &hd, &target, &src).unwrap();
            eprintln!("{grp}: temple_unpainted {:?}", swap.report.temple_unpainted);
            let ann = crate::ann::Ann::parse(&src.bake_ann(&ann0, &mdl).unwrap().unwrap_or(ann0.clone())).unwrap();
            let (p3, img, body) = crate::kh2_hair::tests::view_at(&swap, target.texture);
            let sk = RecomSkeleton::parse(&swap.mdl).unwrap();
            crate::kh2_hair::tests::camera_checks(&p3, &sk, &ann, &img, &body, &dt::out_dir().join("bbs_cam").join(format!("{grp}_{tag}")));
        }
    }

    /// 0.198.13 (fork owner: "tem a do cabelo mesmo e tem a da cabeça base ao lado, ficou estranho"), 0.198.36 (fork owner: "essa
    /// costeleta ainda ta estranha, parece do wolverine, não acho que vc precisava ter tirado a costeleta pinta da cabeça base"): on Dark
    /// Riku's and Riku's models with the BBS hair, both sides, Re:CoM's painted lock is kept (nothing moved), no corner of the kid's
    /// sideburn strand is left, the painted lock shows from the side, and no strip of skin wider than 1 mm lies between the hair's lower
    /// edge and the lock's top (rest; hair and face are on `head`, so every frame; `SB_MAP` prints the side view's cells). Prints the
    /// kid's own sideburn in BBS and the kept head triangles that show grey texels.
    #[test]
    fn sideburn_probe() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let src = BbsHairSource::read(&g, &keys).unwrap();
        // the kid's own sideburn in BBS, as the fit places it (the same frame for his hair and his face): the reference
        {
            let pmo = parse_pmo(&src.pmo).unwrap();
            let ksk = crate::pam::parse_skeleton(&src.pmo).unwrap();
            let kw = ksk.rest_world();
            let a = kw[ksk.find(HEAD_BONE_BBS).unwrap()].t;
            let img = decode_dds(&src.dds).unwrap();
            let (hair, _) = classify(&pmo, a[1], &img);
            let hs: std::collections::HashSet<(usize, usize)> = hair.iter().copied().collect();
            let rel = |p: [f64; 3]| [0, 1, 2].map(|k| SCALE[k] * (p[k] - a[k]) + SHIFT[k]);
            let mut face = vec![];
            let mut hv = vec![];
            for (mi, m) in pmo.meshes.iter().enumerate() {
                for (ti, t) in m.tris.iter().enumerate() {
                    let q = t.map(|i| rel(m.verts[i].pos));
                    if hs.contains(&(mi, ti)) { hv.extend(q); } else if q.iter().all(|p| p[1] > -0.08) { face.push(q); }
                }
            }
            let centre = [0.0, 0.07, 0.04];
            for side in [1.0f64, -1.0] {
                let (mut lo, mut hi) = (f64::MAX, f64::MIN);
                for r in &hv {
                    if r[0] * side < SIDEBURN[0] || !(SIDEBURN[1]..=SIDEBURN[2]).contains(&r[1]) || !(SIDEBURN[3]..=SIDEBURN[4]).contains(&r[2]) { continue; }
                    let d = plane_dist(&face, *r, centre).map_or(f64::NAN, |x| x.0);
                    eprintln!("SBV kid {side} r ({:+.3},{:+.3},{:+.3}) d {:.1}", r[0], r[1], r[2], d * 1e3);
                    lo = lo.min(d);
                    hi = hi.max(d);
                }
                eprintln!("SB kid sideburn x{} clearance mm: deepest {:.1} widest {:.1}", if side > 0.0 { "+" } else { "-" }, lo * 1e3, hi * 1e3);
            }
        }
        let skin = crate::riku_skin::build_with(&arc, &keys, 101, 201, 121, None).unwrap();
        let dark = skin.files.iter().find(|f| f.0 == crate::riku_skin::path(121, "MDL")).unwrap().1.clone();
        let mut pics: Vec<(String, Image, Vec<[[f64; 2]; 3]>)> = vec![];
        for (grp, mdl, target) in [("0201", dark, crate::ddd_hair::RIKU_0201_BBS), ("0101", install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap(), TARGET)] {
            let e = arc.find(&format!("PLAYER/{grp}/PL{grp}.RTM")).unwrap();
            let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
            let swap = with_hair(&src.prepare_mdl(&mdl).unwrap(), &rtm, &hd, &target, &src).unwrap();
            let (p3, img, body) = crate::kh2_hair::tests::view_at(&swap, target.texture);
            eprintln!("SB {grp} unpainted {:?}", swap.report.temple_unpainted);
            if let Ok(dir) = std::env::var("SB_TEX") {
                // the moved triangles as the game reads them: HD DDS and the RTM's 8-bit TIM2 (256 px, palette)
                let hd_img = decode_dds(&swap.dds).unwrap();
                let rtm_p = crate::rtm::Rtm::parse(&swap.rtm).unwrap();
                let e = rtm_p.entries.iter().find(|e| e.name_str() == format!("{}.tm2", target.new_texture)).unwrap();
                let (tw, th, tpx) = crate::rtm::tim2_decode8(&e.data).unwrap();
                let tim = Image { w: tw, h: th, px: tpx };
                let p3n = crate::mdl::MdlPart::parse_bytes(&crate::mdl::split_parts(&swap.mdl).unwrap()[3]).unwrap();
                let ht = p3n.texture_names().iter().position(|n| n == target.new_texture).unwrap() as u32;
                let names_n: Vec<String> = RecomSkeleton::parse(&swap.mdl).unwrap().bones.iter().map(|b| b.name.clone()).collect();
                let mut boxes = vec![];
                for t in dt::collect(&p3n) {
                    if t.tex != ht || !t.v.iter().all(|v| names_n[v.bone()] == "head") { continue; }
                    let uv = t.v.map(|v| v.uv().map_or([0.0; 2], |u| [u[0] as f64, u[1] as f64]));
                    if uv.iter().all(|p| p[1] < 0.5) { continue; } // hair (top); the copies are in the bottom
                    let mean = |im: &Image| { let mut c = [0.0f64; 3]; let mut n = 0.0; for i in 0..=6 { for j in 0..=(6 - i) { let (l0, l1) = (i as f64 / 6.0, j as f64 / 6.0); let l2 = 1.0 - l0 - l1; let px = texel(im, [l0 * uv[0][0] + l1 * uv[1][0] + l2 * uv[2][0], l0 * uv[0][1] + l1 * uv[1][1] + l2 * uv[2][1]]); for k in 0..3 { c[k] += px[k] as f64; } n += 1.0; } } c.map(|x| (x / n).round()) };
                    eprintln!("TEX {grp} uv {:?} hd {:?} tim {:?}", uv.map(|p| [(p[0] * 1e3).round() / 1e3, (p[1] * 1e3).round() / 1e3]), mean(&hd_img), mean(&tim));
                    boxes.push(uv);
                }
                let (mut u0, mut v0, mut u1, mut v1) = (1.0f64, 1.0f64, 0.0f64, 0.0f64);
                for b in &boxes { for p in b { u0 = u0.min(p[0]); v0 = v0.min(p[1]); u1 = u1.max(p[0]); v1 = v1.max(p[1]); } }
                let (u0, v0, u1, v1) = (u0 - 0.03, v0 - 0.03, u1 + 0.03, v1 + 0.03);
                for (nm, im) in [("hd", &hd_img), ("tim", &tim)] {
                    let (w, h) = (600usize, ((v1 - v0) / (u1 - u0) * 600.0) as usize);
                    let mut rgb = vec![0u8; w * h * 3];
                    for y in 0..h { for x in 0..w { let px = texel(im, [u0 + (u1 - u0) * x as f64 / w as f64, v0 + (v1 - v0) * y as f64 / h as f64]); rgb[3 * (y * w + x)..3 * (y * w + x) + 3].copy_from_slice(&px[..3]); } }
                    for b in &boxes { for k in 0..3 { let (a, c) = (b[k], b[(k + 1) % 3]); for s in 0..200 { let f = s as f64 / 200.0; let (x, y) = (((a[0] + (c[0] - a[0]) * f - u0) / (u1 - u0) * w as f64) as usize, ((a[1] + (c[1] - a[1]) * f - v0) / (v1 - v0) * h as f64) as usize); if x < w && y < h { rgb[3 * (y * w + x)..3 * (y * w + x) + 3].copy_from_slice(&[0, 255, 0]); } } } }
                    std::fs::write(format!("{dir}/tex_{grp}_{nm}.png"), dt::png(w, h, &rgb)).unwrap();
                }
            }
            {
                // the moved triangles' UVs (on the hair texture, in its lower half, where the kid's own picture has his body)
                let p3n = crate::mdl::MdlPart::parse_bytes(&crate::mdl::split_parts(&swap.mdl).unwrap()[3]).unwrap();
                let ht = p3n.texture_names().iter().position(|n| n == target.new_texture).unwrap() as u32;
                let hb = RecomSkeleton::parse(&swap.mdl).unwrap().find("head").unwrap();
                let uvs: Vec<[[f64; 2]; 3]> = dt::collect(&p3n).into_iter().filter(|t| t.tex == ht && t.v.iter().all(|v| v.bone() == hb)).map(|t| t.v.map(|v| v.uv().map_or([0.0; 2], |u| [u[0] as f64, u[1] as f64]))).filter(|uv| uv.iter().all(|p| p[1] > 0.7)).collect();
                pics.push((target.new_texture.to_string(), decode_dds(&swap.dds).unwrap(), uvs));
            }
            // 0.198.36: the painted lock is Re:CoM's again (nothing moved) and the kid's strand is gone
            assert_eq!(swap.report.temple_unpainted, (0, 0), "{grp}: the painted lock is kept");
            {
                let rsk = RecomSkeleton::parse(&swap.mdl).unwrap();
                let rw = rsk.rest_world();
                let hb = rsk.find("head").unwrap();
                let h0 = rw[hb].t;
                let p3o = crate::mdl::MdlPart::parse_bytes(&crate::mdl::split_parts(&swap.mdl).unwrap()[3]).unwrap();
                let hair_tex = p3o.texture_names().iter().position(|n| n == target.new_texture).unwrap() as u32;
                let atlas_tex = p3o.texture_names().iter().position(|n| n == target.texture).unwrap() as u32;
                let atlas = decode_dds(&swap.atlas_dds).unwrap();
                let all = dt::collect(&p3o);
                let wp = |v: &crate::mdl::MeshVertex| rw[v.bone()].point(v.pos().map(|x| x as f64));
                let hair_tris: Vec<[[f64; 3]; 3]> = all.iter().filter(|t| t.tex == hair_tex).map(|t| t.v.map(|v| wp(&v))).collect();
                let face: Vec<([[f64; 3]; 3], [[f64; 2]; 3])> = all.iter().filter(|t| t.tex == atlas_tex && t.v.iter().all(|v| v.bone() == hb)).map(|t| (t.v.map(|v| wp(&v)), t.v.map(|v| v.uv().map_or([0.0; 2], |u| [u[0] as f64, u[1] as f64])))).collect();
                let grey = |px: [u8; 4]| { let (mx, mn) = (px[0].max(px[1]).max(px[2]) as f64, px[0].min(px[1]).min(px[2]) as f64); mx > 40.0 && (mx - mn) / mx < 0.3 };
                for side in [1.0f64, -1.0] {
                    // no hair corner left where the kid's strand was
                    let n_in = hair_tris.iter().flatten().filter(|p| { let r = [p[0] - h0[0], p[1] - h0[1], p[2] - h0[2]]; r[0] * side >= SIDEBURN[0] && (SIDEBURN[1]..=SIDEBURN[2]).contains(&r[1]) && (SIDEBURN[3]..=SIDEBURN[4]).contains(&r[2]) }).count();
                    // the temple seen from that side (rays along -x per side, 1 mm grid over the painted lock's window): per cell the
                    // outermost face hit's texel (L = the painted lock, S = skin) unless a hair triangle is outside it (H)
                    let w = TEMPLE_WINDOW;
                    let (mut lock_seen, mut lock_hidden, mut widest, mut cols) = (0usize, 0usize, 0usize, 0usize);
                    let mut map = String::new();
                    for zi in ((w[4] * 1e3) as i32..=(w[5] * 1e3) as i32).rev() {
                        let mut col = vec![];
                        for yi in ((w[2] * 1e3) as i32..=(w[3] * 1e3) as i32).rev() {
                            let o = [h0[0] + side * 0.2, h0[1] + yi as f64 * 1e-3, h0[2] + zi as f64 * 1e-3];
                            let d = [-side, 0.0, 0.0];
                            let fh = face.iter().filter_map(|(t, uv)| ray_bary(o, d, t).map(|(tt, b)| (tt, uv, b))).min_by(|a, b| a.0.total_cmp(&b.0));
                            let Some((tf, uv, b)) = fh else { col.push(' '); continue };
                            if 0.2 - tf < w[0] { col.push(' '); continue; }
                            let hh = hair_tris.iter().filter_map(|t| ray_tri(o, d, t)).fold(f64::MAX, f64::min);
                            let u = [b[0] * uv[0][0] + b[1] * uv[1][0] + b[2] * uv[2][0], b[0] * uv[0][1] + b[1] * uv[1][1] + b[2] * uv[2][1]];
                            let lock = grey(texel(&atlas, u));
                            col.push(if hh < tf { if lock { lock_hidden += 1; } 'H' } else if lock { lock_seen += 1; 'L' } else { 'S' });
                        }
                        // the gap: skin cells between the hair's lower edge and the top of the painted lock in this column
                        if let (Some(hl), Some(lt)) = (col.iter().rposition(|&c| c == 'H').and_then(|_| col.iter().position(|&c| c == 'H')), col.iter().position(|&c| c == 'L')) {
                            let hair_end = (hl..col.len()).take_while(|&k| col[k] == 'H').last().unwrap_or(hl);
                            if lt > hair_end {
                                widest = widest.max(col[hair_end + 1..lt].iter().filter(|&&c| c == 'S').count());
                            }
                            cols += 1;
                        }
                        map.push_str(&format!("{:+04} {}\n", zi, col.iter().collect::<String>()));
                    }
                    if std::env::var("SB_MAP").is_ok() { eprintln!("SB {grp} x{} map (rows z mm front to back, columns y top to bottom):\n{map}", if side > 0.0 { "+" } else { "-" }); }
                    eprintln!("SB {grp} x{}: strand corners {n_in}, painted lock cells seen {lock_seen} hidden {lock_hidden}, columns {cols}, widest skin gap under the hair {widest} mm", if side > 0.0 { "+" } else { "-" });
                    assert_eq!(n_in, 0, "{grp} x{side}: the kid's sideburn is left");
                    assert!(lock_seen >= 40, "{grp} x{side}: the painted lock shows ({lock_seen} cells)");
                    assert!(cols > 0 && widest <= 1, "{grp} x{side}: the painted lock meets the hair (skin gap {widest} mm)");
                }
            }
            let sk = RecomSkeleton::parse(&swap.mdl).unwrap();
            let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
            let w = sk.rest_world();
            let head = sk.find("head").unwrap();
            if std::env::var("SB_DUMP").is_ok() {
                let out = dt::out_dir().join("sideburn");
                std::fs::create_dir_all(&out).unwrap();
                std::fs::write(out.join(format!("view_{grp}.png")), dt::png(img.w, img.h, &img.px.iter().flat_map(|p| [p[0], p[1], p[2]]).collect::<Vec<_>>())).unwrap();
            }
            for t in dt::collect(&p3) {
                if !t.v.iter().all(|v| names[v.bone()] == "head" || names[v.bone()].starts_with("neck")) { continue; }
                let q = t.v.map(|v| w[v.bone()].point(v.pos().map(|x| x as f64)));
                let c = [0, 1, 2].map(|k| (q[0][k] + q[1][k] + q[2][k]) / 3.0 - w[head].t[k]);
                if c[0].abs() < 0.03 || c[1] < -0.08 || c[1] > 0.08 || c[2] < -0.04 || c[2] > 0.08 { continue; }
                let uv = t.v.map(|v| v.uv().map_or([0.0f32; 2], |u| u).map(|x| x as f64));
                let im = if t.tex == 1 { &img } else { &body[t.tex as usize] };
                if t.tex == 1 && (uv[0][0] + uv[1][0] + uv[2][0]) / 3.0 >= 0.5 { continue; } // the BBS hair's half of `view_at`'s picture
                let at = |u: f64, v: f64| im.px[((v.rem_euclid(1.0) * im.h as f64) as usize).min(im.h - 1) * im.w + ((u.rem_euclid(1.0) * im.w as f64) as usize).min(im.w - 1)];
                let grey = |px: [u8; 4]| { let (mx, mn) = (px[0].max(px[1]).max(px[2]) as f64, px[0].min(px[1]).min(px[2]) as f64); mx > 40.0 && (mx - mn) / mx < 0.3 };
                let (mut n, mut gr) = (0, 0);
                for i in 0..=8 { for j in 0..=(8 - i) { let (l0, l1) = (i as f64 / 8.0, j as f64 / 8.0); let l2 = 1.0 - l0 - l1; n += 1; if grey(at(l0 * uv[0][0] + l1 * uv[1][0] + l2 * uv[2][0], l0 * uv[0][1] + l1 * uv[1][1] + l2 * uv[2][1])) { gr += 1; } } }
                if gr == 0 { continue; }
                eprintln!("SB {grp} tex {} grey {gr}/{n} c ({:+.3},{:+.3},{:+.3}) uv {:?} corners {:?} bones {}", t.tex, c[0], c[1], c[2], uv.map(|x| [(x[0] * 1e3).round() / 1e3, (x[1] * 1e3).round() / 1e3]), q.map(|p| [0, 1, 2].map(|k| ((p[k] - w[head].t[k]) * 1e3).round() / 1e3)), t.v.iter().map(|v| names[v.bone()].as_str()).collect::<Vec<_>>().join("/"));
            }
        }
        // 0.198.13 in game (fork owner: "costeleta ficou até pior..."): Re:CoM looks textures up by name over all loaded models, so a
        // model's moved temple triangles may read another model's picture of the same name. Mean colour of each model's triangles in
        // every picture, and the names must differ where the pictures differ.
        for (na, _, uvs) in &pics {
            for (nb, pic, _) in &pics {
                let mut c = [0.0f64; 3];
                let mut n = 0.0f64;
                for uv in uvs { for i in 0..=6 { for j in 0..=(6 - i) { let (l0, l1) = (i as f64 / 6.0, j as f64 / 6.0); let l2 = 1.0 - l0 - l1; let px = texel(pic, [l0 * uv[0][0] + l1 * uv[1][0] + l2 * uv[2][0], l0 * uv[0][1] + l1 * uv[1][1] + l2 * uv[2][1]]); for k in 0..3 { c[k] += px[k] as f64; } n += 1.0; } } }
                eprintln!("CROSS {na} triangles in {nb}'s picture: mean {:?}", c.map(|x| (x / n.max(1.0)).round()));
            }
        }
        for (a, (na, pa, _)) in pics.iter().enumerate() {
            for (nb, pb, _) in &pics[a + 1..] {
                assert!(na != nb || pa.px == pb.px, "two hair pictures share the name {na}");
            }
        }
    }

    /// 0.198.13 in game (fork owner: "costeleta ficou até pior...", a yellow triangle and a flat brown patch at the Dark Form's temple):
    /// the temple close-ups as the game draws them, `out/temple/<tag>/<group>_<picture>_y<yaw>.png` (idle frame 25, both sides): the
    /// moved triangles on their own picture's HD DDS (`own`), on its 8-bit TIM2 (`tim`), and on the other model's picture
    /// (`other`: 0.198.13 gave all four models the one name `rk_hair_bbs`, and the game looks names up over every loaded model).
    /// Prints the mean texel of the moved triangles and of the face triangles that share a corner with them (the cheek).
    #[test]
    fn temple_game_view() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let src = BbsHairSource::read(&g, &keys).unwrap();
        let tag = std::env::var("CAM_TAG").unwrap_or_else(|_| "cam".into());
        let skin = crate::riku_skin::build_with(&arc, &keys, 101, 201, 121, None).unwrap();
        let get = |ext: &str| skin.files.iter().find(|f| f.0 == crate::riku_skin::path(121, ext)).unwrap().1.clone();
        let skin_h = crate::riku_skin::build_with(&arc, &keys, 111, 211, 131, None).unwrap();
        let get_h = |ext: &str| skin_h.files.iter().find(|f| f.0 == crate::riku_skin::path(131, ext)).unwrap().1.clone();
        let rd = |n: &str| install::read_recom(&arc, &keys, n).unwrap();
        let ann_h = install::read_recom(&arc, &keys, "PLAYER/0111/PL0111.ANN").unwrap_or_else(|_| rd("PLAYER/0101/PL0101.ANN"));
        let mut swaps = vec![];
        // in pairs (the "other" picture is the pair's: 0.198.13 named both `rk_hair_bbs`)
        for (grp, mdl, ann0, target) in [("0201", get("MDL"), get("ANN"), crate::ddd_hair::RIKU_0201_BBS), ("0101", rd("PLAYER/0101/PL0101.MDL"), rd("PLAYER/0101/PL0101.ANN"), TARGET), ("0211", get_h("MDL"), get_h("ANN"), crate::ddd_hair::RIKU_0211_BBS), ("0111", rd("PLAYER/0111/PL0111.MDL"), ann_h, crate::ddd_hair::RIKU_0111_BBS)] {
            let e = arc.find(&format!("PLAYER/{grp}/PL{grp}.RTM")).unwrap();
            let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
            let swap = with_hair(&src.prepare_mdl(&mdl).unwrap(), &rtm, &hd, &target, &src).unwrap();
            let ann = crate::ann::Ann::parse(&src.bake_ann(&ann0, &mdl).unwrap().unwrap_or(ann0.clone())).unwrap();
            swaps.push((grp, swap, ann, target));
        }
        let out = dt::out_dir().join("temple").join(&tag);
        std::fs::create_dir_all(&out).unwrap();
        let other_dds: Vec<Vec<u8>> = (0..swaps.len()).map(|k| swaps[k ^ 1].1.dds.clone()).collect();
        for (k, (grp, swap, ann, target)) in swaps.iter_mut().enumerate() {
            let own = swap.dds.clone();
            let tim = {
                let rtm_p = crate::rtm::Rtm::parse(&swap.rtm).unwrap();
                let e = rtm_p.entries.iter().find(|e| e.name_str() == format!("{}.tm2", target.new_texture)).unwrap();
                let (w, h, px) = crate::rtm::tim2_decode8(&e.data).unwrap();
                crate::ddd_hair::dds_argb(&Image { w, h, px })
            };
            // texel means: moved triangles (head, on the hair's picture, sharing a corner with the face) and the cheek around them
            {
                let p3n = crate::mdl::MdlPart::parse_bytes(&crate::mdl::split_parts(&swap.mdl).unwrap()[3]).unwrap();
                let ht = p3n.texture_names().iter().position(|n| n == target.new_texture).unwrap() as u32;
                let at = p3n.texture_names().iter().position(|n| n == target.texture).unwrap() as u32;
                let sk = RecomSkeleton::parse(&swap.mdl).unwrap();
                let hb = sk.find("head").unwrap();
                let key = |v: &crate::mdl::MeshVertex| v.pos().map(|x| (x * 1e4).round() as i64);
                let all = dt::collect(&p3n);
                let face: std::collections::HashSet<[i64; 3]> = all.iter().filter(|t| t.tex == at && t.v.iter().all(|v| v.bone() == hb)).flat_map(|t| t.v.iter().map(key).collect::<Vec<_>>()).collect();
                let moved: Vec<_> = all.iter().filter(|t| t.tex == ht && t.v.iter().all(|v| v.bone() == hb && face.contains(&key(v)))).collect();
                let mk: std::collections::HashSet<[i64; 3]> = moved.iter().flat_map(|t| t.v.iter().map(key).collect::<Vec<_>>()).collect();
                let cheek: Vec<_> = all.iter().filter(|t| t.tex == at && t.v.iter().all(|v| v.bone() == hb) && t.v.iter().any(|v| mk.contains(&key(v)))).collect();
                let mean = |ts: &[&crate::ddd_hair::RTri], im: &Image| { let mut c = [0.0f64; 3]; let mut n = 0.0f64; for t in ts { let uv = t.v.map(|v| v.uv().map_or([0.0; 2], |u| [u[0] as f64, u[1] as f64])); for i in 0..=6 { for j in 0..=(6 - i) { let (l0, l1) = (i as f64 / 6.0, j as f64 / 6.0); let l2 = 1.0 - l0 - l1; let px = texel(im, [l0 * uv[0][0] + l1 * uv[1][0] + l2 * uv[2][0], l0 * uv[0][1] + l1 * uv[1][1] + l2 * uv[2][1]]); for q in 0..3 { c[q] += px[q] as f64; } n += 1.0; } } } c.map(|x| (x / n.max(1.0)).round()) };
                let atlas = decode_dds(&swap.atlas_dds).unwrap();
                eprintln!("TONE {grp} moved {} own {:?} tim {:?} other {:?}; cheek {} {:?}", moved.len(), mean(&moved, &decode_dds(&own).unwrap()), mean(&moved, &decode_dds(&tim).unwrap()), mean(&moved, &decode_dds(&other_dds[k]).unwrap()), cheek.len(), mean(&cheek, &atlas));
            }
            let pics = [("own", own.clone()), ("tim", tim), ("other", other_dds[k].clone())];
            for (name, dds) in pics {
                swap.dds = dds;
                let (p3, img, body) = crate::kh2_hair::tests::view_at(swap, target.texture);
                let sk = RecomSkeleton::parse(&swap.mdl).unwrap();
                let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
                let head = names.iter().position(|n| n == "head").unwrap();
                let c = ann.slots.get(1).and_then(|s| s.clip.as_ref()).unwrap();
                let w = crate::kh2_hair::tests::posed_world(&sk, c, 25.0);
                for (yaw, x) in [(90.0, 0.08), (270.0, -0.08)] {
                    let focus = [w[head].t[0] + x, w[head].t[1] + 0.03, w[head].t[2] + 0.05];
                    crate::kh2_hair::tests::render_focus(&p3, &w, &names, &img, &body, yaw, 0.0, (focus, 0.08), &out.join(format!("{grp}_{name}_{}x.png", if x > 0.0 { "+" } else { "-" })));
                    // debug: `TEMPLE_PIX=x,y` lists every head triangle under that pixel of this view (as `render_view` projects it), nearest
                    // first: its texture in the built model (atlas / hair picture), whether it is a moved temple triangle, depth, UV there
                    if let Some((px, py)) = std::env::var("TEMPLE_PIX").ok().and_then(|v| { let mut it = v.split(',').filter_map(|x| x.parse::<f64>().ok()); Some((it.next()? + 0.5, it.next()? + 0.5)) }) {
                        if name != "own" { continue; }
                        let p3n = crate::mdl::MdlPart::parse_bytes(&crate::mdl::split_parts(&swap.mdl).unwrap()[3]).unwrap();
                        let ht = p3n.texture_names().iter().position(|n| n == target.new_texture).unwrap() as u32;
                        let (sy, cy) = ((yaw as f64).to_radians().sin(), (yaw as f64).to_radians().cos());
                        let tx = |q: [f64; 3]| [q[0] * cy + q[2] * sy, q[1], -q[0] * sy + q[2] * cy];
                        let c = tx(focus);
                        let sc = 640.0 / 0.16;
                        let mut hits = vec![];
                        for (orig, t) in dt::collect(&p3n).iter().zip(dt::collect(&p3)) {
                            if !t.v.iter().all(|v| v.bone() == head) { continue; }
                            let wp = t.v.map(|v| w[head].point(v.pos().map(|x| x as f64)));
                            let q = wp.map(tx);
                            let s = q.map(|q| [30.0 + (q[0] - c[0] + 0.08) * sc, 670.0 - (q[1] - c[1] + 0.08) * sc]);
                            let det = (s[1][1] - s[2][1]) * (s[0][0] - s[2][0]) + (s[2][0] - s[1][0]) * (s[0][1] - s[2][1]);
                            if det.abs() < 1e-9 { continue; }
                            let l0 = ((s[1][1] - s[2][1]) * (px - s[2][0]) + (s[2][0] - s[1][0]) * (py - s[2][1])) / det;
                            let l1 = ((s[2][1] - s[0][1]) * (px - s[2][0]) + (s[0][0] - s[2][0]) * (py - s[2][1])) / det;
                            let l2 = 1.0 - l0 - l1;
                            if l0 < 0.0 || l1 < 0.0 || l2 < 0.0 { continue; }
                            let z = l0 * q[0][2] + l1 * q[1][2] + l2 * q[2][2];
                            let uv = orig.v.map(|v| v.uv().map_or([0.0; 2], |u| [u[0] as f64, u[1] as f64]));
                            let rel = wp.map(|p| [0, 1, 2].map(|k| ((p[k] - w[head].t[k]) * 1e3).round()));
                            hits.push((z, orig.tex, uv, rel, [l0 * uv[0][0] + l1 * uv[1][0] + l2 * uv[2][0], l0 * uv[0][1] + l1 * uv[1][1] + l2 * uv[2][1]]));
                        }
                        hits.sort_by(|a, b| b.0.total_cmp(&a.0));
                        for (z, tex, uv, rel, at) in hits {
                            eprintln!("PIX {grp} {}x z {:.4} tex {} ({}) uv@ {:?} corners mm {:?} uv {:?}", if x > 0.0 { "+" } else { "-" }, z, tex, if tex == ht { "hair pic" } else { "atlas" }, at.map(|x| (x * 1e3).round() / 1e3), rel, uv.map(|p| p.map(|x| (x * 1e3).round() / 1e3)));
                        }
                    }
                }
            }
            swap.dds = own;
        }
    }

    /// 0.198.36 renders (fork owner's screenshot: Riku facing the camera, the lock at his right temple): the head framed as in game, Riku's
    /// default attire's BBS mesh (`183`'s: `PL0101` with the hair) and the Dark Form's (`185`'s: the skin 121 with the hair), playing
    /// Riku's idle (slot 1), walk (2), run (3), jump (4) and first swing (44), from the front, ±25, ±40, ±60 deg and both sides, and the idle
    /// with the camera low and high; `out/temple_owner/<CAM_TAG>/<group>_s<slot>_y<yaw>_p<pitch>.png`.
    #[test]
    fn temple_owner_views() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let src = BbsHairSource::read(&g, &keys).unwrap();
        let tag = std::env::var("CAM_TAG").unwrap_or_else(|_| "cam".into());
        let out = dt::out_dir().join("temple_owner").join(&tag);
        std::fs::create_dir_all(&out).unwrap();
        let skin = crate::riku_skin::build_with(&arc, &keys, 101, 201, 121, None).unwrap();
        let get = |grp: u16, ext: &str| skin.files.iter().find(|f| f.0 == crate::riku_skin::path(grp, ext)).map(|f| f.1.clone()).unwrap_or_else(|| install::read_recom(&arc, &keys, &crate::riku_skin::path(grp, ext)).unwrap());
        for (name, base, raw, target) in [("183", 101u16, 101u16, TARGET), ("185", 121, 201, crate::ddd_hair::RIKU_0201_BBS)] {
            let (mdl, ann0) = (get(base, "MDL"), get(base, "ANN"));
            let e = arc.find(&crate::riku_skin::path(raw, "RTM")).unwrap();
            let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
            let swap = with_hair(&src.prepare_mdl(&mdl).unwrap(), &rtm, &hd, &target, &src).unwrap();
            let ann = crate::ann::Ann::parse(&src.bake_ann(&ann0, &mdl).unwrap().unwrap_or(ann0.clone())).unwrap();
            let (p3, img, body) = crate::kh2_hair::tests::view_at(&swap, target.texture);
            let sk = RecomSkeleton::parse(&swap.mdl).unwrap();
            let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
            let head = sk.find("head").unwrap();
            for slot in [1usize, 2, 3, 4, 44] {
                let Some(c) = ann.slots.get(slot).and_then(|s| s.clip.as_ref()) else { continue };
                let f = if slot == 1 { 25.0 } else { (c.frames / 2) as f64 };
                let w = crate::kh2_hair::tests::posed_world(&sk, c, f);
                let focus = w[head].point([0.0, 0.05, 0.01]);
                let mut views: Vec<(f64, f64)> = [0.0, 25.0, -25.0, 40.0, -40.0, 60.0, -60.0, 90.0, 270.0].iter().map(|&y| (y, 0.0)).collect();
                if slot == 1 {
                    views.extend([(25.0, -30.0), (-25.0, -30.0), (25.0, 30.0), (-25.0, 30.0)]);
                }
                for (yaw, pitch) in views {
                    crate::kh2_hair::tests::render_focus(&p3, &w, &names, &img, &body, yaw, pitch, (focus, 0.15), &out.join(format!("{name}_s{slot}_y{yaw}_p{pitch}.png")));
                }
            }
        }
    }
    /// [`ray_tri`] with the hit's barycentric weights of `t`'s corners.
    fn ray_bary(o: [f64; 3], d: [f64; 3], t: &[[f64; 3]; 3]) -> Option<(f64, [f64; 3])> {
        let sub = |a: [f64; 3], b: [f64; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        let cross = |a: [f64; 3], b: [f64; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
        let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
        let h = cross(d, e2);
        let a = dot(e1, h);
        if a.abs() < 1e-12 { return None; }
        let f = 1.0 / a;
        let q = sub(o, t[0]);
        let u = f * dot(q, h);
        if !(0.0..=1.0).contains(&u) { return None; }
        let r = cross(q, e1);
        let v = f * dot(d, r);
        if v < 0.0 || u + v > 1.0 { return None; }
        let tt = f * dot(e2, r);
        (tt > 0.0).then_some((tt, [1.0 - u - v, u, v]))
    }
    /// Distance along `d` (unit) from `o` to triangle `t`, both faces (Moller-Trumbore).
    fn ray_tri(o: [f64; 3], d: [f64; 3], t: &[[f64; 3]; 3]) -> Option<f64> {
        let sub = |a: [f64; 3], b: [f64; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        let cross = |a: [f64; 3], b: [f64; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
        let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
        let h = cross(d, e2);
        let a = dot(e1, h);
        if a.abs() < 1e-12 { return None; }
        let f = 1.0 / a;
        let q = sub(o, t[0]);
        let u = f * dot(q, h);
        if !(0.0..=1.0).contains(&u) { return None; }
        let r = cross(q, e1);
        let v = f * dot(d, r);
        if v < 0.0 || u + v > 1.0 { return None; }
        Some(f * dot(e2, r))
    }

}
