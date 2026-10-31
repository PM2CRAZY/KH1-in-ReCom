//! KH1 Keyblade → Re:CoM weapon model: a KH1 `.wpn` (its PC HD mesh `.cvbl` and DXT5 textures)
//! becomes a Re:CoM `.MDL` laid out like `PLAYER/0002/PL0002.MDL`, a `.RTM` of 8-bit TIM2s, and
//! the HD textures named `<texture>_tm2.dds`. Notes: `docs/recom-keyblade-models.md`.
//!
//! Rules (each Checked (files) unless marked):
//! - **Mesh**: the `.cvbl` (what KH1 PC draws; its UVs belong to the HD textures). Its vertex
//!   (48 bytes): normal xyz, flags (low byte = matrix slot, `0x10000000` = strip (re)start,
//!   `0x10000` / `0x20000` = draws a triangle with the two before, one facing or the other), u,
//!   v, two words, position xyz, 1.0. The slot names a KH1 joint through the mesh's `0x11`
//!   chunks.
//! - **Bones**: KH1 joints 2–11 are Re:CoM's `wep0, k9 … k1` (bones 2–11 of PL0002): same index,
//!   positions × 0.01 (metres), same axes, one bone per vertex. Any other joint is refused (no
//!   guessed remap).
//! - **Skeleton**: PL0002's 12 bones, rest matrices, part flags and filler, copied from the
//!   template (Re:CoM's own PL0002.MDL), so PL0002.ANN drives the new model unchanged.
//! - **Part 0** (textured, fmt 6): list A = triangles whose texels are all opaque; list B =
//!   triangles that touch a texel with alpha < 255, and every triangle of a translucent cvbl mesh
//!   (mesh flag bit 0). This reproduces Square's split on PL0002 for all 322 list-A triangles and
//!   67 of 70 list-B ones. Strips are kept where consecutive triangles stay in one list; a packet
//!   holds one texture and at most [`MAX_PACKET_QW`] quadwords, as Square's.
//! - **Part 1** (untextured, fmt 0x10): every triangle again, positions only, list A only, as in
//!   PL0002 (its use is not traced: Assumption).
//! - **Facing flag**: cvbl `0x20000` → Re:CoM 0, cvbl `0x10000` → Re:CoM `0x20` (statistics on
//!   PL0002 and 5010; the PC's use of it: Assumption).
//! - **Textures**: one per cvbl texture slot used, named `<KH1 model>_<n>` (e.g. `xw_ex_5030_0`;
//!   texture names are looked up globally, `0x1402A5140`, so they must be unique). TIM2: the HD
//!   texture halved until both sides are ≤ 256, PS2 alpha, 256 colours. HD: KH1's DXT5 file
//!   unchanged (Re:CoM's loader takes DXT5, `0x1402D9760`).
//! - UVs are copied (s = u, t = v, q = 1), the same convention as PL0002's against its HD
//!   texture (all its list-A vertices land on opaque texels).

use std::collections::BTreeSet;

use kh1_formats::cvbl::{Chunk, Cvbl};
use kh1_formats::egs::HdAsset;
use kh1_formats::wpn::Wpn;

use crate::mdl::{Mdl, MdlPart, MeshPacket, MeshVertex, FMT_PLAIN, FMT_TEXTURED, MAX_PACKET_QW};
use crate::rtm::{tim2_indexed8, Rtm, RtmEntry};
use crate::texture::{decode_dds, downscale_to, ps2_alpha, quantise, Image};
use crate::{bad, Result};

/// The KH1 joints a Keyblade may use (= Re:CoM bones `wep0, k9 … k1`).
pub const JOINTS: std::ops::RangeInclusive<u32> = 2..=11;
/// The largest TIM2 side.
pub const TIM2_MAX: usize = 256;
/// Tag of the HD asset of every Re:CoM RTM texture but one (`0x2F7D2`). It can't be an offset;
/// copied as Square writes it. Checked (files) / its use: Assumption.
pub const HD_TAG: i32 = 0x2F7D2;
/// Cvbl strip-start flag.
const START: u32 = 0x1000_0000;
const F32_ONE: u32 = 0x3F80_0000;

/// One converted Keyblade.
#[derive(Clone, Debug)]
pub struct Converted {
    /// The `.MDL`.
    pub mdl: Vec<u8>,
    /// The `.RTM`.
    pub rtm: Vec<u8>,
    /// Texture names, in texture-index order.
    pub textures: Vec<String>,
    /// HD textures: `(<texture>_tm2.dds, bytes)`.
    pub hd: Vec<(String, Vec<u8>)>,
    /// What was converted.
    pub report: Report,
}

/// Counts and checks of one conversion.
#[derive(Clone, Debug, Default)]
pub struct Report {
    /// The KH1 model name.
    pub model: String,
    /// cvbl meshes (KH1) or mdlx groups (KH2).
    pub meshes: usize,
    /// cvbl vertices read.
    pub cvbl_vertices: usize,
    /// Triangles.
    pub triangles: usize,
    /// Triangles in list A / list B of part 0.
    pub list_a: usize,
    /// See `list_a`.
    pub list_b: usize,
    /// Of list B: from translucent cvbl meshes.
    pub translucent: usize,
    /// Packets in part 0 (A + B) and part 1.
    pub packets: (usize, usize),
    /// Vertices written in part 0 and part 1.
    pub vertices: (usize, usize),
    /// KH1 joints used by the cvbl.
    pub joints: BTreeSet<u32>,
    /// KH1 joints used by the PS2 mesh of the `.wpn` (for comparison).
    pub ps2_joints: BTreeSet<u32>,
    /// `(HD w, h, TIM2 w, h)` per texture.
    pub texture_sizes: Vec<(usize, usize, usize, usize)>,
    /// UV range over all vertices: `(min u, max u, min v, max v)`.
    pub uv_range: (f32, f32, f32, f32),
    /// Triangles that span more than one bone.
    pub mixed_bone_triangles: usize,
    /// List-A triangles that touch a texel with alpha < 255 (none by the KH1 rule; KH2's
    /// opaque-phase groups can have some, e.g. 8 on KH2's Kingdom Key, all of which Square's
    /// PL0002 keeps in list A too).
    pub alpha_in_a: usize,
}

impl Report {
    /// One line for the build log.
    pub fn line(&self) -> String {
        format!(
            "{}: {} meshes, {} triangles (list A {} of which {} touch alpha < 255, list B {} of which {} translucent), joints {:?}, {} textures {:?}, uv {:.3}..{:.3} × {:.3}..{:.3}",
            self.model,
            self.meshes,
            self.triangles,
            self.list_a,
            self.alpha_in_a,
            self.list_b,
            self.translucent,
            self.joints,
            self.texture_sizes.len(),
            self.texture_sizes,
            self.uv_range.0,
            self.uv_range.1,
            self.uv_range.2,
            self.uv_range.3
        )
    }
}

#[derive(Clone, Copy, Debug)]
struct Vtx {
    pos: [f32; 3],
    uv: [f32; 2],
    bone: u32,
}

#[derive(Clone, Copy, Debug)]
struct Tri {
    v: [Vtx; 3],
    /// Re:CoM strip flag of the drawing vertex (0 or 0x20).
    facing: u32,
    /// Texture index (into the converted texture list).
    tex: u32,
    /// Source strip id and position, to keep strips together.
    strip: usize,
    pos: usize,
    translucent: bool,
}

/// The texture name for slot index `n` of KH1 model `model` (≤ 15 bytes).
pub fn texture_name(model: &str, n: usize) -> Result<String> {
    let s = format!("{model}_{n}");
    if s.len() > 15 || !s.is_ascii() {
        return bad(format!("texture name {s} is longer than 15 bytes"));
    }
    Ok(s)
}

/// The HD asset name Re:CoM looks up for texture `name` (`w_ex010_01` → `w_ex010_01_tm2.dds`).
pub fn hd_name(name: &str) -> String {
    format!("{name}_tm2.dds")
}

fn find<'a>(assets: &'a [HdAsset], name: &str) -> Result<&'a HdAsset> {
    assets.iter().find(|a| a.name == name).ok_or_else(|| crate::Error::Format(format!("HD asset {name} not found")))
}

/// Reads the cvbl's triangles (KH1 joints, KH1 slots) in source order.
fn read_triangles(cv: &Cvbl, slot_to_tex: &dyn Fn(u32) -> u32, rep: &mut Report) -> Result<Vec<Tri>> {
    let mut tris = Vec::new();
    let mut strip = 0usize;
    for (mi, mesh) in cv.meshes.iter().enumerate() {
        let mut slots = [u32::MAX; 256];
        let translucent = mesh.flags & 1 != 0;
        for ch in &mesh.chunks {
            match ch {
                Chunk::Matrix { joint, slot, .. } => {
                    *slots.get_mut(*slot as usize).ok_or_else(|| crate::Error::Format(format!("mesh {mi}: matrix slot {slot}")))? = *joint;
                }
                Chunk::Vertices { stride, words, .. } => {
                    if *stride != 48 {
                        return bad(format!("mesh {mi}: {stride}-byte vertices (only the 48-byte textured form is converted)"));
                    }
                    // A start vertex draws nothing but enters the vertex queue (as the PS2's
                    // ADC bit); a drawing vertex makes a triangle with the two before it.
                    let mut prev: Vec<Vtx> = Vec::new();
                    let mut pos = 0usize;
                    let mut last_drew = true;
                    for w in words.chunks(12) {
                        let fl = w[3];
                        let joint = slots[(fl & 0xFF) as usize];
                        if joint == u32::MAX {
                            return bad(format!("mesh {mi}: vertex on unset matrix slot {}", fl & 0xFF));
                        }
                        rep.joints.insert(joint);
                        if !JOINTS.contains(&joint) {
                            return bad(format!("mesh {mi}: vertex on KH1 joint {joint}, outside 2..11 (no Re:CoM bone; not remapped)"));
                        }
                        rep.cvbl_vertices += 1;
                        let f = |k: usize| f32::from_bits(w[k]);
                        let v = Vtx { pos: [f(8) * 0.01, f(9) * 0.01, f(10) * 0.01], uv: [f(4), f(5)], bone: joint };
                        match fl & 0xFFFF_0000 {
                            START => {
                                if last_drew {
                                    strip += 1;
                                    pos = 0;
                                }
                                last_drew = false;
                                prev.push(v);
                            }
                            k @ (0x1_0000 | 0x2_0000) => {
                                if prev.len() < 2 {
                                    return bad(format!("mesh {mi}: a triangle without two vertices before it"));
                                }
                                let n = prev.len();
                                tris.push(Tri { v: [prev[n - 2], prev[n - 1], v], facing: if k == 0x2_0000 { 0 } else { 0x20 }, tex: slot_to_tex(mesh.texture), strip, pos, translucent });
                                pos += 1;
                                last_drew = true;
                                prev.push(v);
                            }
                            k => return bad(format!("mesh {mi}: vertex flags {k:#x} not understood")),
                        }
                    }
                }
            }
        }
    }
    Ok(tris)
}

fn min_alpha(img: &Image, t: &Tri) -> u8 {
    triangle_min_alpha(img, [t.v[0].uv, t.v[1].uv, t.v[2].uv])
}

/// The lowest alpha of the texels a UV triangle covers (texel centres inside it, and its
/// corners): the list A / list B rule (< 255 → list B).
pub fn triangle_min_alpha(img: &Image, uv: [[f32; 2]; 3]) -> u8 {
    let (w, h) = (img.w as f32, img.h as f32);
    let p: [[f32; 2]; 3] = core::array::from_fn(|i| [uv[i][0] * w, uv[i][1] * h]);
    let mut mn = 255u8;
    for q in &p {
        mn = mn.min(img.at(q[0].floor() as isize, q[1].floor() as isize)[3]);
    }
    let den = (p[1][1] - p[2][1]) * (p[0][0] - p[2][0]) + (p[2][0] - p[1][0]) * (p[0][1] - p[2][1]);
    if den.abs() < 1e-9 {
        return mn;
    }
    let x0 = p.iter().map(|q| q[0]).fold(f32::MAX, f32::min).floor().max(0.0) as usize;
    let x1 = (p.iter().map(|q| q[0]).fold(f32::MIN, f32::max).ceil() as usize).min(img.w);
    let y0 = p.iter().map(|q| q[1]).fold(f32::MAX, f32::min).floor().max(0.0) as usize;
    let y1 = (p.iter().map(|q| q[1]).fold(f32::MIN, f32::max).ceil() as usize).min(img.h);
    for y in y0..y1 {
        for x in x0..x1 {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let a = ((p[1][1] - p[2][1]) * (px - p[2][0]) + (p[2][0] - p[1][0]) * (py - p[2][1])) / den;
            let b = ((p[2][1] - p[0][1]) * (px - p[2][0]) + (p[0][0] - p[2][0]) * (py - p[2][1])) / den;
            if a >= -1e-6 && b >= -1e-6 && a + b <= 1.0 + 1e-6 {
                mn = mn.min(img.px[y * img.w + x][3]);
            }
        }
    }
    mn
}

/// A strip segment: consecutive triangles of one source strip, in one list, one texture.
fn segments(tris: &[&Tri]) -> Vec<Vec<Tri>> {
    let mut out: Vec<Vec<Tri>> = Vec::new();
    for t in tris {
        match out.last_mut() {
            Some(s) if s.last().is_some_and(|l| l.strip == t.strip && l.pos + 1 == t.pos && l.tex == t.tex) => s.push(**t),
            _ => out.push(vec![**t]),
        }
    }
    out
}

fn vertex(v: &Vtx, start: bool, facing: u32, textured: bool, tex: u32) -> MeshVertex {
    let q0 = if start { [0xFFFF, F32_ONE, 0, 0xA000] } else { [facing, F32_ONE, 0, 0x2000] };
    let q1 = [v.pos[0].to_bits(), v.pos[1].to_bits(), v.pos[2].to_bits(), (v.bone << 16) | (v.bone * 4)];
    let q2 = textured.then_some([v.uv[0].to_bits(), v.uv[1].to_bits(), F32_ONE, tex]);
    MeshVertex { q0, q1, q2 }
}

/// Packs strip segments into packets of one texture each and at most [`MAX_PACKET_QW`].
fn packets(segs: &[Vec<Tri>], fmt: u16, gif: u32, regs: u32) -> Vec<MeshPacket> {
    let textured = fmt == FMT_TEXTURED;
    let stride = if textured { 3 } else { 2 };
    let max_v = (MAX_PACKET_QW - 2) / stride;
    let mut out: Vec<MeshPacket> = Vec::new();
    let mut cur_tex = u32::MAX;
    for seg in segs {
        let tex = if textured { seg[0].tex } else { 0 };
        // The strip as vertices: two starts, then one per triangle.
        let mut sv: Vec<(Vtx, u32)> = vec![(seg[0].v[0], u32::MAX), (seg[0].v[1], u32::MAX)];
        sv.extend(seg.iter().map(|t| (t.v[2], t.facing)));
        let mut i = 0;
        while i + 2 < sv.len() {
            let need_new = match out.last() {
                None => true,
                Some(p) => tex != cur_tex || p.verts.len() + 3 > max_v,
            };
            if need_new {
                out.push(MeshPacket { fmt, gif, regs, verts: Vec::new() });
                cur_tex = tex;
            }
            let p = out.last_mut().unwrap();
            let room = max_v - p.verts.len();
            let take = (sv.len() - i).min(room);
            for (k, (v, f)) in sv[i..i + take].iter().enumerate() {
                p.verts.push(vertex(v, k < 2, *f, textured, tex));
            }
            // Continue a split strip from its last two vertices.
            i += take;
            if i < sv.len() {
                i -= 2;
            }
        }
    }
    out
}

fn bounds(nb: usize, lists: &[&[MeshPacket]]) -> Vec<[u32; 6]> {
    let mut b = vec![[16_700_000f32, -16_700_000f32, 16_700_000f32, -16_700_000f32, 16_700_000f32, -16_700_000f32]; nb];
    for l in lists {
        for p in l.iter() {
            for v in &p.verts {
                let (bone, pos) = (v.bone(), v.pos());
                if let Some(bb) = b.get_mut(bone) {
                    for k in 0..3 {
                        bb[2 * k] = bb[2 * k].min(pos[k]);
                        bb[2 * k + 1] = bb[2 * k + 1].max(pos[k]);
                    }
                }
            }
        }
    }
    b.iter().map(|x| x.map(f32::to_bits)).collect()
}

/// Checks that `template` is PL0002-shaped: two parts (textured, untextured), 12 bones with
/// `wep0` at 2, and returns the GIF words of both packet formats.
fn template_gif(t: &Mdl) -> Result<((u32, u32), (u32, u32))> {
    let ok = t.parts.len() == 2
        && t.parts.iter().all(|p| p.bones.len() == 12)
        && crate::ann::cstr(&t.parts[0].bones[2].name) == "wep0"
        && t.parts[0].list_a.first().is_some_and(|p| p.fmt == FMT_TEXTURED)
        && t.parts[1].list_a.first().is_some_and(|p| p.fmt == FMT_PLAIN);
    if !ok {
        return bad("template: not shaped like PL0002.MDL (2 parts, 12 bones, wep0 = bone 2, fmt 6 / 0x10)");
    }
    let a = &t.parts[0].list_a[0];
    let b = &t.parts[1].list_a[0];
    Ok(((a.gif, a.regs), (b.gif, b.regs)))
}

/// Converts one KH1 Keyblade. `template` is Re:CoM's own `PL0002.MDL`; `wpn` the KH1 file and
/// `assets` its HD assets; `model` the KH1 model name (`xw_ex_5030`), which names the textures.
pub fn convert(template: &Mdl, model: &str, wpn: &[u8], assets: &[HdAsset]) -> Result<Converted> {
    template_gif(template)?;
    let w = Wpn::parse(wpn)?;
    let mut rep = Report { model: model.to_string(), ..Default::default() };
    for mesh in &w.model()?.meshes {
        for p in &mesh.packets {
            for it in &p.items {
                if let kh1_formats::mobj::Item::Matrix { joint, .. } = it {
                    rep.ps2_joints.insert(*joint);
                }
            }
        }
    }
    let cv = Cvbl::parse(&find(assets, &w.cvbl_name())?.data)?;
    rep.meshes = cv.meshes.len();
    // Textures: the cvbl slots used, in slot order.
    let slots: Vec<u32> = cv.meshes.iter().map(|m| m.texture).collect::<BTreeSet<_>>().into_iter().collect();
    let slot_to_tex = |s: u32| slots.iter().position(|&x| x == s).unwrap() as u32;
    let tris = read_triangles(&cv, &slot_to_tex, &mut rep)?;
    let mut textures = Vec::new();
    for (n, &slot) in slots.iter().enumerate() {
        let a = find(assets, &w.texture_name(slot))?;
        textures.push((texture_name(model, n)?, a.data.clone()));
    }
    // List B: a translucent cvbl mesh, or a texel with alpha < 255 under the triangle.
    assemble(template, rep, tris, textures, |t, img| t.translucent || min_alpha(img, t) < 255)
}

/// The parts every conversion shares: the textures (TIM2 + HD), the lists, the packets, part 1,
/// the bounds and the file. `in_b` says whether a triangle goes to list B (it sees the
/// triangle's decoded HD texture).
fn assemble(template: &Mdl, mut rep: Report, tris: Vec<Tri>, textures_in: Vec<(String, Vec<u8>)>, in_b: impl Fn(&Tri, &Image) -> bool) -> Result<Converted> {
    let ((g6, r6), (g10, r10)) = template_gif(template)?;
    if tris.is_empty() {
        return bad(format!("{}: no triangles", rep.model));
    }
    let mut textures = Vec::new();
    let mut images = Vec::new();
    let mut hd = Vec::new();
    let mut rtm = Rtm { entries: Vec::new() };
    for (name, data) in textures_in {
        let img = decode_dds(&data).map_err(|e| crate::Error::Format(format!("{name}: {e}")))?;
        let small = downscale_to(&img, TIM2_MAX);
        let ps2 = Image { w: small.w, h: small.h, px: small.px.iter().map(|c| [c[0], c[1], c[2], ps2_alpha(c[3])]).collect() };
        let (pal, idx) = quantise(&ps2);
        rtm.entries.push(RtmEntry::new(&name, tim2_indexed8(ps2.w as u16, ps2.h as u16, &idx, &pal)?)?);
        rep.texture_sizes.push((img.w, img.h, ps2.w, ps2.h));
        hd.push((hd_name(&name), data));
        textures.push(name);
        images.push(img);
    }
    // Lists.
    let mut uv = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    let mut b_of = Vec::with_capacity(tris.len());
    for t in &tris {
        for v in &t.v {
            uv = (uv.0.min(v.uv[0]), uv.1.max(v.uv[0]), uv.2.min(v.uv[1]), uv.3.max(v.uv[1]));
        }
        if t.v.iter().any(|v| v.bone != t.v[0].bone) {
            rep.mixed_bone_triangles += 1;
        }
        let img = images.get(t.tex as usize).ok_or_else(|| crate::Error::Format(format!("{}: texture {} of {}", rep.model, t.tex, images.len())))?;
        rep.translucent += t.translucent as usize;
        let b = in_b(t, img);
        rep.alpha_in_a += (!b && min_alpha(img, t) < 255) as usize;
        b_of.push(b);
    }
    rep.uv_range = uv;
    rep.triangles = tris.len();
    let a_tris: Vec<&Tri> = tris.iter().zip(&b_of).filter(|(_, b)| !**b).map(|(t, _)| t).collect();
    let b_tris: Vec<&Tri> = tris.iter().zip(&b_of).filter(|(_, b)| **b).map(|(t, _)| t).collect();
    rep.list_a = a_tris.len();
    rep.list_b = b_tris.len();
    // Group by texture (one texture per packet), keeping source order within a texture.
    let n_tex = textures.len() as u32;
    let by_tex = |v: &[&Tri]| -> Vec<Vec<Tri>> { (0..n_tex).flat_map(|k| segments(&v.iter().copied().filter(|t| t.tex == k).collect::<Vec<_>>())).collect() };
    let list_a = packets(&by_tex(&a_tris), FMT_TEXTURED, g6, r6);
    let list_b = packets(&by_tex(&b_tris), FMT_TEXTURED, g6, r6);
    let all: Vec<&Tri> = tris.iter().collect();
    let plain = packets(&segments(&all), FMT_PLAIN, g10, r10);
    let tp0 = &template.parts[0];
    let tp1 = &template.parts[1];
    let mut tex_table = Vec::new();
    for n in &textures {
        let mut e = [0u8; 32];
        e[..n.len()].copy_from_slice(n.as_bytes());
        tex_table.push(e);
    }
    let lb = (!list_b.is_empty()).then_some(list_b);
    let p0 = MdlPart {
        flags: tp0.flags,
        filler: tp0.filler,
        bones: tp0.bones.clone(),
        rest: tp0.rest.clone(),
        textures: tex_table,
        bounds: bounds(12, &[&list_a, lb.as_deref().unwrap_or(&[])]),
        list_a,
        list_b: lb,
    };
    let p1 = MdlPart { flags: tp1.flags, filler: tp1.filler, bones: tp1.bones.clone(), rest: tp1.rest.clone(), textures: vec![], bounds: bounds(12, &[&plain]), list_a: plain, list_b: None };
    let count = |l: &[MeshPacket]| (l.len(), l.iter().map(|p| p.verts.len()).sum::<usize>());
    let (pa, va) = count(&p0.list_a);
    let (pb, vb) = count(p0.list_b.as_deref().unwrap_or(&[]));
    let (p1n, v1) = count(&p1.list_a);
    rep.packets = (pa + pb, p1n);
    rep.vertices = (va + vb, v1);
    let mdl = Mdl { parts: vec![p0, p1] }.write()?;
    Ok(Converted { mdl, rtm: rtm.write(), textures, hd, report: rep })
}

/// KH2 Keyblades (decision 25): a KH2 FM `obj/W_EX010_*.mdlx` → the same Re:CoM layout.
/// Notes: `docs/recom-keyblade-models.md` § 12.
///
/// - **Skeleton**: KH2's Keyblades carry PL0002's 12 bones (root, grip, blade bone 2, keychain
///   3..11, the chain's lengths × 0.01 equal PL0002's to 1e-6 m). KH2's Kingdom Key
///   `W_EX010`'s mesh, in bone space × 0.01, *is* Re:CoM's PL0002 mesh (every corner within
///   0.08 mm of a PL0002 vertex on the same bone, UVs within 0.0003; 322 + 70 triangles as
///   PL0002's lists). So KH2 bone `j` = Re:CoM bone `j`, positions × 0.01, same axes, UVs as
///   they are. Checked (files). Every converted Keyblade must have W_EX010's skeleton exactly
///   ([`check_kh2_skeleton`]): KH2 poses all of base Sora's Keyblades with the same weapon
///   moveset, so the same bone-space mesh sits in the hand the same way.
/// - **Lists**: list B = the groups KH2 draws in its alpha phase (group attribute bit 0,
///   `DrawAlphaPhase` in OpenKH's names); list A = the rest. On W_EX010 this gives PL0002's
///   lists exactly ([`KH2_LIST_B`]).
/// - **Facing**: KH2's strip flags `0x20` / `0x30` (the two windings) → Re:CoM `0x20` / 0
///   ([`kh2_facing`]): of W_EX010's 202 list-A triangles that PL0002 has corner for corner, 174
///   carry the facing this gives, 28 the other (list B, the flat keychain: 33 / 37). The PC's
///   use of the flag is an Assumption (§ 1.3).
/// - **Textures**: the HD asset `-<texture index>.dds` of the `.mdlx` (A8R8G8B8, 512×256 on the
///   six), named `<model in lower case>_<n>` (`w_ex010_30_0`); none of Re:CoM's 1649 texture
///   names (Checked (files)).
pub const KH2_LIST_B: u32 = 1;

/// Re:CoM's facing flag for a KH2 strip flag (`0x20` / `0x30`).
pub fn kh2_facing(flag: u8) -> u32 {
    if flag == 0x30 {
        0
    } else {
        0x20
    }
}

/// Bone positions of two KH2 skeletons agree to this (KH2 units, cm) and rotations to this
/// (radians).
const KH2_BONE_EPS: f32 = 1e-4;

/// Checks a KH2 Keyblade skeleton: 12 bones, each with the same parent, scale, rotation and
/// translation as `reference` (KH2's Kingdom Key `W_EX010`), and the keychain bones 3..11
/// equal to the template's (PL0002) rest locals × 100 (identity rotation, same translation).
pub fn check_kh2_skeleton(bones: &[kh2_formats_lite::skeleton::Bone], reference: &[kh2_formats_lite::skeleton::Bone], template: &Mdl) -> Result<()> {
    if bones.len() != 12 || reference.len() != 12 {
        return bad(format!("{} bones (reference {}), not PL0002's 12", bones.len(), reference.len()));
    }
    for (i, (b, r)) in bones.iter().zip(reference).enumerate() {
        let near = |a: [f32; 3], c: [f32; 3]| (0..3).all(|k| (a[k] - c[k]).abs() <= KH2_BONE_EPS);
        if b.parent != r.parent || !near(b.scale, r.scale) || !near(b.rotation, r.rotation) || !near(b.translation, r.translation) {
            return bad(format!("bone {i} differs from the Kingdom Key's (W_EX010): {b:?} vs {r:?}"));
        }
    }
    let rest = &template.parts[0].rest;
    for (i, b) in bones.iter().enumerate().skip(3) {
        let m: Vec<f32> = rest[i].iter().map(|x| f32::from_bits(*x)).collect();
        let ident = (0..3).all(|r| (0..3).all(|c| (m[4 * r + c] - if r == c { 1.0 } else { 0.0 }).abs() < 1e-6));
        let t_ok = (0..3).all(|k| (m[12 + k] * 100.0 - b.translation[k]).abs() <= 1e-3);
        let r_ok = b.rotation.iter().all(|x| x.abs() <= KH2_BONE_EPS) && b.parent == i as i16 - 1;
        if !(ident && t_ok && r_ok) {
            return bad(format!("keychain bone {i}: KH2 t {:?} r {:?}, PL0002 rest {:?}", b.translation, b.rotation, m));
        }
    }
    Ok(())
}

/// How far KH2's Kingdom Key (`W_EX010.mdlx`), in bone space × 0.01, is from the template
/// (Re:CoM's PL0002): the largest distance (metres) from a KH2 corner to the nearest template
/// vertex on the same bone. The asset step requires it to be small ([`KH2_TEMPLATE_MATCH`])
/// before converting any KH2 Keyblade: it is the evidence that KH2's bone `j` is Re:CoM's bone
/// `j` with the same axes and units (0.08 mm on the owner's files). Checked (files).
pub fn kh2_template_distance(template: &Mdl, kingdom_key_mdlx: &[u8]) -> Result<f32> {
    let m = kh2_formats_lite::mdlx::parse(kingdom_key_mdlx).map_err(|e| crate::Error::Format(format!("W_EX010: {e}")))?;
    let p0 = &template.parts[0];
    let ours: Vec<(usize, [f32; 3])> = p0.list_a.iter().chain(p0.list_b.iter().flatten()).flat_map(|p| p.verts.iter().map(|v| (v.bone(), v.pos()))).collect();
    let mut worst = 0f32;
    for v in m.groups.iter().flat_map(|g| g.triangles.iter().flatten()) {
        let p = v.position.map(|x| x * 0.01);
        let d = ours.iter().filter(|(b, _)| *b == v.bone as usize).map(|(_, q)| ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt()).fold(f32::INFINITY, f32::min);
        worst = worst.max(d);
    }
    Ok(worst)
}

/// The most [`kh2_template_distance`] may be (0.5 mm).
pub const KH2_TEMPLATE_MATCH: f32 = 0.0005;

/// Converts one KH2 Keyblade. `template` is Re:CoM's own `PL0002.MDL`; `mdlx` the KH2 file and
/// `assets` its HD assets; `model` the KH2 model name (`W_EX010_30`), which names the textures
/// (in lower case); `reference` KH2's Kingdom Key skeleton (`W_EX010.mdlx`).
pub fn convert_kh2(template: &Mdl, model: &str, mdlx: &[u8], assets: &[HdAsset], reference: &[kh2_formats_lite::skeleton::Bone]) -> Result<Converted> {
    template_gif(template)?;
    let kh2 = |e: kh2_formats_lite::Error| crate::Error::Format(format!("{model}: {e}"));
    let bones = kh2_formats_lite::skeleton::parse_skeleton(mdlx).map_err(kh2)?;
    check_kh2_skeleton(&bones, reference, template).map_err(|e| crate::Error::Format(format!("{model}: {e}")))?;
    let m = kh2_formats_lite::mdlx::parse(mdlx).map_err(kh2)?;
    let lower = model.to_ascii_lowercase();
    let mut rep = Report { model: lower.clone(), meshes: m.groups.len(), ..Default::default() };
    // Textures: those the groups use, in index order.
    let used: Vec<usize> = m.groups.iter().map(|g| g.texture).collect::<BTreeSet<_>>().into_iter().collect();
    let tex_of = |t: usize| used.iter().position(|&x| x == t).unwrap() as u32;
    let mut tris = Vec::new();
    for (gi, g) in m.groups.iter().enumerate() {
        for t in &g.strip_triangles {
            let mut v = [Vtx { pos: [0.0; 3], uv: [0.0; 2], bone: 0 }; 3];
            for (k, c) in t.v.iter().enumerate() {
                let bone = c.bone as u32;
                rep.joints.insert(bone);
                if !JOINTS.contains(&bone) {
                    return bad(format!("{model}: group {gi}: vertex on bone {bone}, outside 2..11 (no Re:CoM bone; not remapped)"));
                }
                rep.cvbl_vertices += 1;
                v[k] = Vtx { pos: c.position.map(|x| x * 0.01), uv: c.uv, bone };
            }
            tris.push(Tri { v, facing: kh2_facing(t.flag), tex: tex_of(g.texture), strip: (gi << 20) | t.strip, pos: t.pos, translucent: g.attributes & KH2_LIST_B != 0 });
        }
    }
    let mut textures = Vec::new();
    for (n, &t) in used.iter().enumerate() {
        let a = find(assets, &format!("-{t}.dds"))?;
        textures.push((texture_name(&lower, n)?, a.data.clone()));
    }
    assemble(template, rep, tris, textures, |t, _| t.translucent)
}

/// Re:CoM resource group for a Keyblade card kind: the Kingdom Key (1) stays on Re:CoM's own
/// group 2 (PL0002); every other Keyblade Sora can hold gets `19 + card`: KH1's (2..=0x11) groups
/// 21..=36, KH2's (0x13..=0x18) groups 38..=43. Soul Eater (0x12, Riku's) has none, so 37 stays
/// unused. Re:CoM's PLAYER resource table (`0x1408055D0`, 552 rows) uses 1, 2, 11–14, 101, 111,
/// 201, 211, 301–307, 401–409, 451, 452, 458, then 1001 and up, and `Recom.hed` has no
/// `PLAYER/00GG/PL00GG.*` for G = 15..100 (11 extensions). Checked (exe + files).
pub fn group_for_card(card: u8) -> Option<u16> {
    match card {
        1 => Some(2),
        2..=0x11 | 0x13..=0x18 => Some(19 + card as u16),
        _ => None,
    }
}

/// `PLAYER/<group>/PL<group>.<ext>` (groups are decimal, 4 digits: `PLAYER/0101/PL0101.MDL`).
pub fn resource_path(group: u16, ext: &str) -> String {
    format!("PLAYER/{group:04}/PL{group:04}.{ext}")
}

/// A Keyblade's model as the runtime finds it (`kh1-stats.json`, `keyblades` rows).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeybladeModel {
    /// Re:CoM card kind.
    pub card: u8,
    /// PLAYER resource group ([`group_for_card`]).
    pub group: u16,
    /// Texture names.
    pub textures: Vec<String>,
}

/// The files the build step writes for one converted Keyblade, as paths under Panacea's
/// `<mod_path>/recom/`:
/// - `PLAYER/<group>/PL<group>.MDL`: the model, a loose file (it has no HD assets, like Re:CoM's
///   own PL0002.MDL).
/// - `raw/PLAYER/<group>/PL<group>.RTM`: the textures as a whole plain PKG entry whose HD assets
///   are the `<texture>_tm2.dds` files. Panacea's loose route gives a file name the PKG doesn't
///   have no HD assets (`ScanRemasteredFolder` keeps as many as the PKG entry has: none), while
///   its `raw` route reads the asset table from the file (`Panacea::LoadFile`, `GetRawFile`).
///   Checked (OpenKH source) / in game: Assumption. Without HD the TIM2 inside is what shows.
pub fn mod_files(group: u16, c: &Converted) -> Result<Vec<(String, Vec<u8>)>> {
    let assets: Vec<HdAsset> = c.hd.iter().map(|(name, data)| HdAsset { name: name.clone(), orig_offset: HD_TAG, data: data.clone() }).collect();
    let entry = kh1_formats::egs::build_plain_entry(&c.rtm, &assets, 0)?;
    Ok(vec![(resource_path(group, "MDL"), c.mdl.clone()), (format!("raw/{}", resource_path(group, "RTM")), entry)])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(strip: usize, pos: usize, tex: u32) -> Tri {
        let v = Vtx { pos: [pos as f32, 0.0, 0.0], uv: [0.0; 2], bone: 2 };
        Tri { v: [v; 3], facing: 0, tex, strip, pos, translucent: false }
    }

    #[test]
    fn groups_and_paths() {
        assert_eq!(group_for_card(1), Some(2));
        assert_eq!(group_for_card(2), Some(21));
        assert_eq!(group_for_card(17), Some(36));
        assert_eq!(group_for_card(0x12), None, "Soul Eater");
        assert_eq!(group_for_card(0x13), Some(38));
        assert_eq!(group_for_card(0x18), Some(43));
        assert_eq!(group_for_card(0x19), None);
        assert_eq!(group_for_card(0), None);
        assert_eq!(resource_path(21, "MDL"), "PLAYER/0021/PL0021.MDL");
        assert_eq!(texture_name("xw_ex_5200", 1).unwrap(), "xw_ex_5200_1");
        assert_eq!(kh2_facing(0x20), 0x20);
        assert_eq!(kh2_facing(0x30), 0);
        assert_eq!(hd_name("w_ex010_01"), "w_ex010_01_tm2.dds");
    }

    #[test]
    fn long_strips_split_with_two_vertex_overlap() {
        // One strip of 100 triangles = 102 vertices: 72 fit in the first packet (74 max).
        let tris: Vec<Tri> = (0..100).map(|i| t(1, i, 0)).collect();
        let segs = segments(&tris.iter().collect::<Vec<_>>());
        assert_eq!(segs.len(), 1);
        let p = packets(&segs, FMT_TEXTURED, 0, 0);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].verts.len(), 74);
        let drawn: usize = p.iter().map(|k| k.verts.iter().filter(|v| v.draws()).count()).sum();
        assert_eq!(drawn, 100);
        assert!(p.iter().all(|k| !k.verts[0].draws() && !k.verts[1].draws()));
        // A new texture starts a new packet; a break in the strip starts a new segment.
        let tris = [t(1, 0, 0), t(1, 1, 0), t(2, 0, 0), t(3, 0, 1)];
        let segs = segments(&tris.iter().collect::<Vec<_>>());
        assert_eq!(segs.len(), 3);
        let p = packets(&segs, FMT_TEXTURED, 0, 0);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].verts.len(), 4 + 3);
    }
}
