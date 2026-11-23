//! Riku's Dark Form skin: Dark Riku's model (`PLAYER/0201/PL0201.MDL` / `.RTM`, unchanged) playing
//! Riku's own motions (`PLAYER/0101/PL0101.ANN`) retargeted onto Dark Riku's skeleton (fork owner,
//! 2026-11-22: "pode colocar a skin mas os movimentos ser o normal mesmo").
//!
//! - Skeletons (Checked (files)): Riku `ex_1010_sk`, 102 bones; Dark Riku `ex_1580_sk`, 128 bones.
//!   Every body bone has the same name in both. The Dark model has 43 more bones: the coat tails
//!   `*_skt*`, `l_uparm2` / `r_uparm2`, and `wep1` / `wep2`, which stand for Riku's `k10` / `k11` on
//!   the Soul Eater's chain. Riku's belts / cuffs / pants have no Dark bone. The rest poses differ
//!   by up to 8 cm and the bones' rest axes differ, so channels can't be copied.
//! - Retarget, per frame: Riku's world rotation of a bone, taken relative to his rest world
//!   rotation, is applied to the Dark bone's rest world rotation (`U_b = R_b · R_aᵀ · U_a`). Bone
//!   translations are Dark's rest translations, except `Root`, which is Riku's (root motion), and
//!   bone 1, which is Dark's rest plus Riku's move off his rest (hip height, jumps). Dark-only bones
//!   keep their rest pose relative to their parent.
//! - Riku's `.SES` / `.EET` events name bones by index (`docs/recom-m13-recom-fx.md`): they are
//!   remapped by name to Dark Riku's indices; an event on a bone with no Dark match goes to its
//!   nearest mapped ancestor.
//!
//! The skin is written as new player groups ([`SKIN_GROUPS`]) so Riku's own files stay as they are.

use crate::ann::{Ann, Clip, Slot};
use crate::convert::{encode_channel, locals_for, Target};
use crate::math::{m3, rot_xyz, tr, unscale, M3};
use crate::mdl::RecomSkeleton;
use crate::{bad, Result};

/// (Riku's group, Dark Riku's group, the skin's group). Groups 101 / 111 are Riku's two player
/// groups and 201 / 211 Dark Riku's (`0x1405A6910`, Checked (exe)). 121 and 131 are not in
/// Recom.hed (Checked (files)).
pub const SKIN_GROUPS: [(u16, u16, u16); 2] = [(101, 201, 121), (111, 211, 131)];

/// Dark bone name → Riku bone name, where they differ.
fn alias(dark: &str) -> &str {
    match dark {
        "ex_1580_sk" => "ex_1010_sk",
        "wep1" => "k10",
        "wep2" => "k11",
        x => x,
    }
}

/// For each Dark bone, Riku's bone of the same role.
pub fn bone_map(riku: &RecomSkeleton, dark: &RecomSkeleton) -> Vec<Option<usize>> {
    dark.bones.iter().map(|b| riku.find(alias(&b.name))).collect()
}

/// For each Riku bone, the Dark bone for its events: the same role, else the nearest ancestor
/// that has one (`Root` at worst).
pub fn event_map(riku: &RecomSkeleton, dark: &RecomSkeleton) -> Vec<usize> {
    let inv: Vec<Option<usize>> = riku.bones.iter().map(|b| {
        let n = match b.name.as_str() {
            "ex_1010_sk" => "ex_1580_sk",
            "k10" => "wep1",
            "k11" => "wep2",
            x => x,
        };
        dark.find(n)
    }).collect();
    (0..riku.bones.len())
        .map(|mut i| loop {
            if let Some(j) = inv[i] {
                break j;
            }
            let p = riku.bones[i].parent;
            if p < 0 {
                break 0;
            }
            i = p as usize;
        })
        .collect()
}

fn world_rot(skel: &RecomSkeleton) -> Vec<M3> {
    skel.rest_world().iter().map(|w| unscale(&w.r).0).collect()
}

/// Retargets one of Riku's clips onto Dark Riku's skeleton. Frame count, fps, loop and blend are
/// the clip's own.
pub fn retarget(src: &Clip, riku: &RecomSkeleton, dark: &RecomSkeleton, map: &[Option<usize>]) -> Result<Clip> {
    let na = riku.bones.len();
    if src.bones as usize != na {
        return bad(format!("{}: {} bones, Riku's skeleton has {na}", src.name_str(), src.bones));
    }
    let ra = world_rot(riku);
    let rb = world_rot(dark);
    let nb = dark.bones.len();
    let grip = Grip::new(riku, dark);
    let mut rows: Vec<Vec<[f64; 9]>> = Vec::with_capacity(src.frames as usize);
    for f in 0..src.frames.max(1) as usize {
        // Riku's world rotations and positions at this frame.
        let mut ua: Vec<M3> = Vec::with_capacity(na);
        let mut pa: Vec<[f64; 3]> = Vec::with_capacity(na);
        let mut loc_t: Vec<[f64; 3]> = Vec::with_capacity(na);
        for (i, b) in riku.bones.iter().enumerate() {
            let s = src.sample(i, f as f64);
            let r = rot_xyz(s.r[0], s.r[1], s.r[2]);
            loc_t.push(s.t);
            if b.parent < 0 {
                ua.push(r);
                pa.push(s.t);
            } else {
                let p = b.parent as usize;
                let u = m3(&r, &ua[p]);
                let pos = crate::math::vm(s.t, &ua[p]);
                pa.push([pos[0] + pa[p][0], pos[1] + pa[p][1], pos[2] + pa[p][2]]);
                ua.push(u);
            }
        }
        // Dark's targets.
        let mut tg: Vec<Target> = Vec::with_capacity(nb);
        let mut gripped: Vec<Option<Target>> = vec![None; nb];
        for (j, b) in dark.bones.iter().enumerate() {
            if let Some(g) = &grip {
                if j == g.first {
                    g.place(&tg, &ua, &pa, &mut gripped);
                }
                if let Some(t) = gripped[j] {
                    tg.push(t);
                    continue;
                }
            }
            let u = match map[j] {
                // The blade past `wep0`: relative to `wep0`, as Riku's is to his (the rest frames
                // relative to `wep0` corrected), so it never twists against the grip.
                Some(i) if grip.as_ref().is_some_and(|g| g.below[j]) => {
                    let g = grip.as_ref().unwrap();
                    let qa = m3(&ua[i], &tr(&ua[g.wep0_a]));
                    let qa0 = m3(&ra[i], &tr(&ra[g.wep0_a]));
                    let qb0 = m3(&rb[j], &tr(&rb[g.wep0_b]));
                    // Under the un-turned `wep0`: the chain (trails, hits) stays where Riku's is;
                    // only the blade mesh on `wep0` is turned over.
                    m3(&m3(&m3(&qb0, &tr(&qa0)), &qa), &m3(&BLADE_FLIP, &tg[g.wep0_b].u))
                }
                Some(i) => m3(&m3(&rb[j], &tr(&ra[i])), &ua[i]),
                None if b.parent < 0 => rb[j],
                None => m3(&unscale(&b.local.r).0, &tg[b.parent as usize].u),
            };
            let p = if b.parent < 0 {
                map[j].map_or([0.0; 3], |i| pa[i])
            } else {
                let par = &tg[b.parent as usize];
                let lt = match (j, map[j]) {
                    (1, Some(i)) => {
                        let rt = riku.bones[i].local.t;
                        let d = [loc_t[i][0] - rt[0], loc_t[i][1] - rt[1], loc_t[i][2] - rt[2]];
                        [b.local.t[0] + d[0], b.local.t[1] + d[1], b.local.t[2] + d[2]]
                    }
                    _ => b.local.t,
                };
                let w = crate::math::vm(lt, &par.u);
                [w[0] + par.p[0], w[1] + par.p[1], w[2] + par.p[2]]
            };
            tg.push(Target { u, s: [1.0; 3], p });
        }
        let mut l = locals_for(dark, &tg, None, rows.last().map(|v| v.as_slice()));
        // Riku's own scale channels, bone for bone (0.154.4): his menu / draw clips hide the
        // Soul Eater by scaling `wep0` and `k1`..`k9` to 0 (Checked (files): 4 clips, 117 frames).
        // The translations above stay unscaled; the local scale is Riku's as his clip has it.
        if src.chans() == 9 {
            for (j, m) in map.iter().enumerate() {
                if let Some(i) = m {
                    let sc = src.sample(*i, f as f64).s;
                    l[j][6..9].copy_from_slice(&sc);
                }
            }
        }
        rows.push(l);
    }
    Ok(encode(src, &rows))
}

/// The Soul Eater's hold. Riku's `k11` / `k10` and Dark Riku's `wep2` / `wep1` both join the hand
/// to the blade root `wep0`, but they are not the same chain: Riku's goes 17 cm down the hand's
/// −z and back, Dark's 22 cm up its +z and back (Checked (files): rest positions in the hand's
/// frame). Mapped bone to bone they turn the blade the other way round (the owner's screenshot,
/// 0.154.1). At rest `wep0` sits the same in both hands (rotation identical, position 1.6 cm
/// apart). So: Dark's `wep0` takes Riku's `wep0` relative to the hand (in the hand's own axes, plus
/// the rest difference), and `wep2` / `wep1` follow `wep0` rigidly as in Dark's rest pose.
/// Half a turn about `wep0`'s x (the blade's axis), in its own frame (row vectors: on the left).
const BLADE_FLIP: M3 = [[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]];

struct Grip {
    /// Dark's first chain bone (`wep2`): from here the three are placed together.
    first: usize,
    hand_b: usize,
    hand_a: usize,
    wep0_a: usize,
    wep0_b: usize,
    /// Dark's bones below `wep0` (the blade segments).
    below: Vec<bool>,
    /// Dark's chain bones with their rest rotation and offset relative to Dark's `wep0` (wep0 itself:
    /// identity, 0).
    chain: Vec<(usize, M3, [f64; 3])>,
    /// Dark's rest `wep0` position in Dark's hand minus Riku's in Riku's (hand axes).
    d: [f64; 3],
}

impl Grip {
    fn new(riku: &RecomSkeleton, dark: &RecomSkeleton) -> Option<Grip> {
        let (hand_a, wep0_a) = (riku.find("r_hand")?, riku.find("wep0")?);
        let (hand_b, w0, w1, w2) = (dark.find("r_hand")?, dark.find("wep0")?, dark.find("wep1")?, dark.find("wep2")?);
        let (wa, wb) = (riku.rest_world(), dark.rest_world());
        let rot = |w: &crate::math::Aff| unscale(&w.r).0;
        let in_frame = |p: [f64; 3], o: &crate::math::Aff| crate::math::vm([p[0] - o.t[0], p[1] - o.t[1], p[2] - o.t[2]], &tr(&rot(o)));
        let pa = in_frame(wa[wep0_a].t, &wa[hand_a]);
        let pb = in_frame(wb[w0].t, &wb[hand_b]);
        let u0 = rot(&wb[w0]);
        let chain = [w0, w1, w2].iter().map(|&k| (k, m3(&rot(&wb[k]), &tr(&u0)), in_frame(wb[k].t, &wb[w0]))).collect();
        let mut below = vec![false; dark.bones.len()];
        for (j, b) in dark.bones.iter().enumerate() {
            below[j] = b.parent >= 0 && (b.parent as usize == w0 || below[b.parent as usize]);
        }
        Some(Grip { first: w2.min(w1).min(w0), hand_b, hand_a, wep0_a, wep0_b: w0, below, chain, d: [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]] })
    }

    /// The three chain bones' targets, from Dark's hand target (already placed) and Riku's pose.
    fn place(&self, tg: &[Target], ua: &[M3], pa: &[[f64; 3]], out: &mut [Option<Target>]) {
        let hb = &tg[self.hand_b];
        // Riku's blade root in his hand's axes, then in Dark's hand.
        let c = crate::math::vm(
            [pa[self.wep0_a][0] - pa[self.hand_a][0], pa[self.wep0_a][1] - pa[self.hand_a][1], pa[self.wep0_a][2] - pa[self.hand_a][2]],
            &tr(&ua[self.hand_a]),
        );
        let c = [c[0] + self.d[0], c[1] + self.d[1], c[2] + self.d[2]];
        let w = crate::math::vm(c, &hb.u);
        let p0 = [hb.p[0] + w[0], hb.p[1] + w[1], hb.p[2] + w[2]];
        // Riku's blade's rotation in his hand, in Dark's hand (the rest rotations match), turned
        // half a turn about the blade's own axis: Dark Riku's blade mesh is Riku's turned over
        // (its bounds on `wep0`: y −0.12..0.13 against Riku's −0.14..0.10), and Dark Riku's own
        // clips hold it so (his grip against Riku's: 137..178° about `wep0`'s x in 12 clips,
        // Checked (files)).
        let u0 = m3(&BLADE_FLIP, &m3(&m3(&ua[self.wep0_a], &tr(&ua[self.hand_a])), &hb.u));
        for &(k, rel, off) in &self.chain {
            let o = crate::math::vm(off, &u0);
            out[k] = Some(Target { u: m3(&rel, &u0), s: [1.0; 3], p: [p0[0] + o[0], p0[1] + o[1], p0[2] + o[2]] });
        }
    }
}

/// The clip from per-frame locals, keeping the source clip's header fields (no scale channels).
fn encode(src: &Clip, rows: &[Vec<[f64; 9]>]) -> Clip {
    const EPS: [f64; 3] = [1e-7, 1e-5, 1e-6];
    let nb = rows[0].len();
    let scale = rows.iter().any(|r| r.iter().any(|b| (6..9).any(|c| (b[c] - 1.0).abs() > EPS[2])));
    let chans = if scale { 9 } else { 6 };
    let mut channels = Vec::with_capacity(nb * chans);
    for b in 0..nb {
        for c in 0..chans {
            let kind = c / 3;
            let vals: Vec<f64> = rows.iter().map(|r| if kind == 1 { r[b][c].to_degrees() } else { r[b][c] }).collect();
            channels.push(encode_channel(&vals, EPS[kind]));
        }
    }
    Clip {
        name: src.name,
        bones: nb as i16,
        frames: rows.len() as i16,
        loop_start: src.loop_start,
        flags: if scale { src.flags | 2 } else { src.flags & !2 },
        has_blend: src.has_blend,
        blend: src.blend,
        blend_pad: src.blend_pad,
        mask_pad: vec![0; (((nb * chans + 7) >> 3) + 3) & !3],
        channels,
        trailer: vec![0; 4],
    }
}

/// Riku's whole motion pack on Dark Riku's skeleton: the same slots, names and order.
pub fn retarget_ann(ann: &Ann, riku: &RecomSkeleton, dark: &RecomSkeleton) -> Result<Ann> {
    let map = bone_map(riku, dark);
    let slots = ann
        .slots
        .iter()
        .map(|s| {
            Ok(Slot {
                name: s.name,
                clip: match &s.clip {
                    Some(c) => Some(retarget(c, riku, dark, &map)?),
                    None => None,
                },
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut out = Ann { slots, tail: Vec::new() };
    out.pad_tail_like_recom();
    Ok(out)
}

fn remap_u32(ev: &mut [u8], at: usize, map: &[usize]) {
    let v = u32::from_le_bytes(ev[at..at + 4].try_into().unwrap()) as usize;
    if let Some(&n) = map.get(v) {
        ev[at..at + 4].copy_from_slice(&(n as u32).to_le_bytes());
    }
}

/// Riku's `.SES` with each event's bone (`+0x08`) moved to Dark Riku's index.
pub fn remap_ses(d: &[u8], map: &[usize]) -> Result<Vec<u8>> {
    let mut es = crate::ses::parse(d)?;
    for e in &mut es {
        for ev in &mut e.events {
            remap_u32(ev, 0x08, map);
        }
    }
    Ok(crate::ses::write(&es))
}

/// Riku's `.EET` with list A's bone (`+0x24`) and list B's three bones (`+0x24`, `+0x34`,
/// `+0x44`) moved to Dark Riku's indices.
pub fn remap_eet(d: &[u8], map: &[usize]) -> Result<Vec<u8>> {
    let mut rs = crate::eet::parse(d)?;
    for r in &mut rs {
        for ev in &mut r.a {
            remap_u32(ev, 0x24, map);
        }
        for ev in &mut r.b {
            for at in [0x24, 0x34, 0x44] {
                remap_u32(ev, at, map);
            }
        }
    }
    crate::eet::write(&rs)
}

/// `PLAYER/<g>/PL<g>.<ext>`.
pub fn path(g: u16, ext: &str) -> String {
    format!("PLAYER/{g:04}/PL{g:04}.{ext}")
}


/// Dark Riku's own entry flip (his action row 0x37, played by his Dark Mode switch-in
/// `0x14020485B`; Checked (exe)), added to the skin's pack on his own skeleton (the skin's), with
/// its EET / SES events (`ex_1580.dpx` 18 at bone 0, `transform.dpx` 1 at bone 51; Checked (files)).
/// The costume change plays it after Riku's own transformation crouch (`win::riku_skin`).
pub const DARK_FLIP: &str = "p003_067";

/// The skin manifest's file name (next to the other manifests).
pub const MANIFEST: &str = "kh1-riku-skin.json";

/// Dark Riku's bone 1 as Riku's name, so Riku's own bone lookups (`0x1401A4450` by name:
/// `ex_1010_sk`, `wep0`, `chest1`, `head`, `l_toes`, `r_toes`, from the strings at
/// `0x1407C5E10..0x1407C5E50`; Checked (exe)) find it. Same length; every part's name table.
pub fn rename_root(mdl: &[u8]) -> Result<Vec<u8>> {
    let from = b"ex_1580_sk\0";
    let to = b"ex_1010_sk\0";
    let mut out = mdl.to_vec();
    let mut n = 0;
    let mut i = 0;
    while i + from.len() <= out.len() {
        if &out[i..i + from.len()] == from {
            out[i..i + to.len()].copy_from_slice(to);
            n += 1;
            i += from.len();
        } else {
            i += 1;
        }
    }
    if n == 0 {
        return bad("Dark Riku's model has no ex_1580_sk bone");
    }
    Ok(out)
}

/// The files of one skin group, relative to the mod's `recom` root: the MDL / ANN / EET / SES
/// loose, the RTM / TXA as raw entries with their PC textures.
pub struct Built {
    /// (relative path, bytes).
    pub files: Vec<(String, Vec<u8>)>,
    /// Clips retargeted.
    pub clips: usize,
}

/// Reads Riku's group `riku` and Dark Riku's group `dark` from Recom.pkg and builds skin group
/// `skin`.
pub fn build(arc: &kh1_formats::egs::Archive, keys: &kh1_formats::egs::KeyTable, riku: u16, dark: u16, skin: u16) -> Result<Built> {
    let rd = |g: u16, ext: &str| crate::install::read_recom(arc, keys, &path(g, ext));
    let raw = |g: u16, ext: &str| -> Result<(Vec<u8>, Vec<kh1_formats::egs::HdAsset>, i32)> {
        let n = path(g, ext);
        let e = arc.find(&n).ok_or_else(|| crate::Error::Format(format!("{n} not in Recom.hed")))?;
        let raw = arc.read_raw_entry(e)?;
        let date = raw.get(12..16).map_or(0, |b| i32::from_le_bytes(b.try_into().unwrap()));
        let (d, a) = kh1_formats::egs::decode_entry_full(keys, &raw)?;
        Ok((d, a, date))
    };
    let rmdl = rd(riku, "MDL")?;
    let dmdl = rd(dark, "MDL")?;
    let rs = RecomSkeleton::parse(&rmdl)?;
    let ds = RecomSkeleton::parse(&dmdl)?;
    let ann = Ann::parse(&rd(riku, "ANN")?)?;
    let mut out_ann = retarget_ann(&ann, &rs, &ds)?;
    let em = event_map(&rs, &ds);
    let mut eet = crate::eet::parse(&remap_eet(&rd(riku, "EET")?, &em)?)?;
    let mut ses = crate::ses::parse(&remap_ses(&rd(riku, "SES")?, &em)?)?;
    // Dark Riku's entry flip with its own events (his skeleton is the skin's).
    let dann = Ann::parse(&rd(dark, "ANN")?)?;
    let flip = dann.slots.iter().find(|s| s.clip.as_ref().is_some_and(|c| c.name_str() == DARK_FLIP)).ok_or_else(|| crate::Error::Format(format!("{}: no {DARK_FLIP}", path(dark, "ANN"))))?;
    if out_ann.find(DARK_FLIP).is_none() {
        let tail = std::mem::take(&mut out_ann.tail);
        out_ann.slots.push(flip.clone());
        out_ann.tail = tail;
        out_ann.pad_tail_like_recom();
    }
    eet.extend(crate::eet::parse(&rd(dark, "EET")?)?.into_iter().filter(|r| r.name == DARK_FLIP));
    ses.extend(crate::ses::parse(&rd(dark, "SES")?)?.into_iter().filter(|e| e.name() == DARK_FLIP));
    let clips = out_ann.slots.iter().filter(|s| s.clip.is_some()).count();
    let mut files = vec![
        (path(skin, "MDL"), rename_root(&dmdl)?),
        (path(skin, "ANN"), out_ann.to_bytes()),
        (path(skin, "EET"), crate::eet::write(&eet)?),
        (path(skin, "SES"), crate::ses::write(&ses)),
    ];
    for ext in ["RTM", "TXA"] {
        let (d, a, date) = raw(dark, ext)?;
        files.push((format!("raw/{}", path(skin, ext)), kh1_formats::egs::build_plain_entry(&d, &a, date)?));
    }
    Ok(Built { files, clips })
}

/// The manifest: which Riku group each skin group stands for.
pub fn manifest_json(skins: &[(u16, u16)]) -> String {
    let rows: Vec<String> = skins.iter().map(|(r, s)| format!("{{\"riku\": {r}, \"skin\": {s}}}")).collect();
    format!("{{\"format\": 1, \"skins\": [{}]}}\n", rows.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::install;

    fn game() -> Option<std::path::PathBuf> {
        std::env::var_os("RECOM_GAME").map(Into::into)
    }

    /// With the owner's install (`RECOM_GAME`): Riku's idle on the Dark skeleton keeps its feet
    /// where Riku's are (within Dark's longer legs' difference), and an idle pose's bones
    /// stay near Dark's own rest.
    #[test]
    fn riku_idle_on_the_dark_skeleton() {
        let Some(g) = game() else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let rd = |n: &str| install::read_recom(&arc, &keys, n).unwrap();
        let riku = RecomSkeleton::parse(&rd("PLAYER/0101/PL0101.MDL")).unwrap();
        let dark = RecomSkeleton::parse(&rd("PLAYER/0201/PL0201.MDL")).unwrap();
        let ann = Ann::parse(&rd("PLAYER/0101/PL0101.ANN")).unwrap();
        let map = bone_map(&riku, &dark);
        let idle = ann.slots.iter().find_map(|s| s.clip.as_ref().filter(|c| c.name_str() == "p002_001")).unwrap();
        let out = retarget(idle, &riku, &dark, &map).unwrap();
        assert_eq!(out.bones as usize, dark.bones.len());
        let world_y = |c: &Clip, sk: &RecomSkeleton, name: &str| {
            let k = sk.find(name).unwrap();
            let mut chain = vec![k];
            while sk.bones[*chain.last().unwrap()].parent >= 0 {
                chain.push(sk.bones[*chain.last().unwrap()].parent as usize);
            }
            let mut u = crate::math::I3;
            let mut p = [0.0; 3];
            for &b in chain.iter().rev() {
                let s = c.sample(b, 0.0);
                let w = crate::math::vm(s.t, &u);
                p = [p[0] + w[0], p[1] + w[1], p[2] + w[2]];
                u = m3(&rot_xyz(s.r[0], s.r[1], s.r[2]), &u);
            }
            p[1]
        };
        for foot in ["l_foot", "r_foot", "head"] {
            let a = world_y(idle, &riku, foot);
            let b = world_y(&out, &dark, foot);
            eprintln!("{foot}: riku {a:.3} dark {b:.3}");
            let ra = riku.rest_world()[riku.find(foot).unwrap()].t[1];
            let rb = dark.rest_world()[dark.find(foot).unwrap()].t[1];
            assert!(((a - ra) - (b - rb)).abs() < 0.03, "{foot}: Riku {a} (rest {ra}), Dark {b} (rest {rb})");
        }
        // The Soul Eater: on every frame of every clip, the blade root and tip in the hand's axes
        // match Riku's (within the rest difference, 2 cm).
        let in_hand = |c: &Clip, sk: &RecomSkeleton, f: f64, bone: &str| -> [f64; 3] {
            let mut u = vec![crate::math::I3; sk.bones.len()];
            let mut p = vec![[0.0; 3]; sk.bones.len()];
            for (i, b) in sk.bones.iter().enumerate() {
                let s = c.sample(i, f);
                let r = rot_xyz(s.r[0], s.r[1], s.r[2]);
                if b.parent < 0 {
                    u[i] = r;
                    p[i] = s.t;
                } else {
                    let q = b.parent as usize;
                    let w = crate::math::vm(s.t, &u[q]);
                    p[i] = [p[q][0] + w[0], p[q][1] + w[1], p[q][2] + w[2]];
                    u[i] = m3(&r, &u[q]);
                }
            }
            let h = sk.find("r_hand").unwrap();
            let k = sk.find(bone).unwrap();
            crate::math::vm([p[k][0] - p[h][0], p[k][1] - p[h][1], p[k][2] - p[h][2]], &tr(&u[h]))
        };
        let mut worst: f64 = 0.0;
        let mut worst_rot: f64 = 0.0;
        let rot_in_hand = |c: &Clip, sk: &RecomSkeleton, f: f64| -> M3 {
            let mut u = vec![crate::math::I3; sk.bones.len()];
            for (i, b) in sk.bones.iter().enumerate() {
                let s = c.sample(i, f);
                let r = rot_xyz(s.r[0], s.r[1], s.r[2]);
                u[i] = if b.parent < 0 { r } else { m3(&r, &u[b.parent as usize]) };
            }
            m3(&u[sk.find("wep0").unwrap()], &tr(&u[sk.find("r_hand").unwrap()]))
        };
        for c in ann.slots.iter().filter_map(|s| s.clip.as_ref()) {
            let o = retarget(c, &riku, &dark, &map).unwrap();
            for f in 0..c.frames as usize {
                let (ra_, rb_) = (rot_in_hand(c, &riku, f as f64), rot_in_hand(&o, &dark, f as f64));
                let want = m3(&BLADE_FLIP, &ra_);
                worst_rot = worst_rot.max(crate::math::angle_deg(&m3(&rb_, &tr(&want))));
                // The root where Riku's is in the hand; the tip where Riku's is from the root (in
                // Dark's turned-over `wep0` axes).
                worst = worst.max(crate::math::dist(in_hand(c, &riku, f as f64, "wep0"), in_hand(&o, &dark, f as f64, "wep0")));
                let tip = |cl: &Clip, sk: &RecomSkeleton, r: &M3| {
                    let (k, w) = (in_hand(cl, sk, f as f64, "k1"), in_hand(cl, sk, f as f64, "wep0"));
                    crate::math::vm([k[0] - w[0], k[1] - w[1], k[2] - w[2]], &tr(r))
                };
                let ta = crate::math::vm(tip(c, &riku, &ra_), &BLADE_FLIP);
                let tb = tip(&o, &dark, &rb_);
                worst = worst.max(crate::math::dist(ta, tb));
            }
        }
        eprintln!("Soul Eater in the hand: worst {worst:.4} m");
        // Riku's hidden blade stays hidden: every frame where his `wep0` is scaled, Dark's is too.
        let (wa, wb) = (riku.find("wep0").unwrap(), dark.find("wep0").unwrap());
        let mut scaled = 0;
        for c in ann.slots.iter().filter_map(|s| s.clip.as_ref()).filter(|c| c.chans() == 9) {
            let o = retarget(c, &riku, &dark, &map).unwrap();
            assert_eq!(o.chans(), 9, "{}", c.name_str());
            for f in 0..c.frames as usize {
                let (a, b) = (c.sample(wa, f as f64).s, o.sample(wb, f as f64).s);
                assert!((0..3).all(|k| (a[k] - b[k]).abs() < 1e-3), "{} frame {f}: {a:?} {b:?}", c.name_str());
                scaled += (a[0] < 0.5) as usize;
            }
        }
        eprintln!("frames with Riku's blade hidden, kept hidden: {scaled}");
        assert!(scaled > 0);
        assert!(worst < 0.03, "the blade moved in the hand by {worst} m");
        eprintln!("Soul Eater's turn in the hand against Riku's turned over: worst {worst_rot:.2} deg");
        assert!(worst_rot < 0.5);
        for &(r, d, k) in &SKIN_GROUPS {
            let b = build(&arc, &keys, r, d, k).unwrap();
            let get = |ext: &str| &b.files.iter().find(|f| f.0.ends_with(ext)).unwrap().1;
            let sk = RecomSkeleton::parse(get(".MDL")).unwrap();
            assert_eq!(sk.bones[1].name, "ex_1010_sk");
            let a = Ann::parse(get(".ANN")).unwrap();
            assert!(a.slots.iter().flatten_clips().all(|c| c.bones as usize == sk.bones.len()));
            assert!(a.find(DARK_FLIP).is_some(), "the flip in the pack");
            assert!(crate::eet::parse(get(".EET")).unwrap().iter().any(|r| r.name == DARK_FLIP));
            assert!(crate::ses::parse(get(".SES")).unwrap().iter().any(|e| e.name() == DARK_FLIP));
            eprintln!("group {k}: {} clips, files {:?}", b.clips, b.files.iter().map(|f| (f.0.clone(), f.1.len())).collect::<Vec<_>>());
        }
    }

    trait Clips<'a> {
        fn flatten_clips(self) -> Box<dyn Iterator<Item = &'a Clip> + 'a>;
    }
    impl<'a, I: Iterator<Item = &'a Slot> + 'a> Clips<'a> for I {
        fn flatten_clips(self) -> Box<dyn Iterator<Item = &'a Clip> + 'a> {
            Box::new(self.filter_map(|s| s.clip.as_ref()))
        }
    }
}
