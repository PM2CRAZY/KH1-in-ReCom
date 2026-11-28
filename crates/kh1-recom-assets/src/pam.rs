//! PAM animation packs (BBS FM's Terra, version 1; version 2 is read too) and PMO skeletons, for Riku's cast and Guard
//! motions (0.189.0: BBS FM's Terra, `crate::terra`; this file was `ddd.rs` until 0.193.0, when the Dream Drop Distance source
//! was removed; fork owner, 2026-11-24: "pode remover o do DDD").
//!
//! Formats (the workspace's `knowledge/BBS/bbs-terra-casts-guard-for-riku.md`, Checked (files) on the owner's install against
//! OpenKh@41ac7b5 `OpenKh.Bbs/Pam.cs`):
//! - **PAM**: `"PAM\0"`, u32 count, version u16 at 0x0E, then 16-byte entries from 0x10: `u32 offset` (0 = an empty slot) and
//!   the name, inline as `char[12]` (version 1, BBS) or `u32` offset into a string table at the file's end (version 2); each
//!   clip: u16 flag, u8 fps (30), u8 blend-in, u16 loop, u8 bone count, u8 pad, u16 frames, u16 return frame; u16 channel mask per
//!   bone (bits 0..8: tx ty tz rx ry rz sx sy sz); per present channel `f32 max, f32 min`, a key count (u8, u16 when frames
//!   > 255), then the keys (frame id u8 / u16, absent when the count equals the frames; u16 value) unless the count is 1
//!   (a constant: max); value = min + (max - min) x q / 65535, linear between keys;
//! - a bone's local transform: S . Rx . Ry . Rz + T, row vectors, radians (Re:CoM's clips' own convention); a missing
//!   channel is 0 (translation, rotation) or 1 (scale);
//! - **PMO** skeleton: `"PMO\0"`, its skeleton offset at +0xC; at it `"BON\0"`, u32, u16 bone count, then 160-byte
//!   bones from +0x10: u16 index, u16 pad, u16 parent (0xFFFF none), u16 pad, u16 skinned index, u16 pad, u32 pad,
//!   char name[16], f32 local[16] (row-major, row vectors), f32 inverse world[16].
//!
//! Nothing of the game is in this file: the step reads the player's files.

use crate::ann::Clip;
use crate::math::{m3, rot_xyz, Aff, M3};
use crate::mdl::{RBone, RecomSkeleton};
use crate::{bad, Result};

/// One animation channel.
#[derive(Clone, Debug, PartialEq)]
pub struct Channel {
    max: f32,
    min: f32,
    /// (frame, quantized value); empty: a constant (`max`).
    keys: Vec<(u16, u16)>,
}

impl Channel {
    fn dq(&self, q: u16) -> f64 {
        self.min as f64 + (self.max as f64 - self.min as f64) * q as f64 / 65535.0
    }

    /// The value at frame `t` (fractional: linear between keys).
    pub fn value(&self, t: f64) -> f64 {
        let Some(first) = self.keys.first() else { return self.max as f64 };
        if t <= first.0 as f64 {
            return self.dq(first.1);
        }
        for w in self.keys.windows(2) {
            let (f0, f1) = (w[0].0 as f64, w[1].0 as f64);
            if t <= f1 {
                let u = if f1 > f0 { (t - f0) / (f1 - f0) } else { 0.0 };
                return self.dq(w[0].1) + (self.dq(w[1].1) - self.dq(w[0].1)) * u;
            }
        }
        self.dq(self.keys.last().unwrap().1)
    }
}

/// One animation of a PAM pack.
#[derive(Clone, Debug, PartialEq)]
pub struct PamClip {
    /// Its name (Terra's are 3 digits, `"315"`).
    pub name: String,
    /// Its frame rate (30 in every pack read).
    pub fps: u8,
    /// Its frame count.
    pub frames: u16,
    /// Per bone, the nine channels (tx ty tz rx ry rz sx sy sz), None when absent.
    pub bones: Vec<[Option<Channel>; 9]>,
}

impl PamClip {
    /// A bone's local (translation, rotation radians, scale) at frame `t`.
    pub fn local(&self, bone: usize, t: f64) -> ([f64; 3], [f64; 3], [f64; 3]) {
        let ch = |c: usize, d: f64| self.bones.get(bone).and_then(|b| b[c].as_ref()).map_or(d, |x| x.value(t));
        ([ch(0, 0.0), ch(1, 0.0), ch(2, 0.0)], [ch(3, 0.0), ch(4, 0.0), ch(5, 0.0)], [ch(6, 1.0), ch(7, 1.0), ch(8, 1.0)])
    }
}

fn u8_at(b: &[u8], o: usize) -> Result<u8> {
    b.get(o).copied().ok_or_else(|| crate::Error::Format("PAM: outside the file".into()))
}
fn u16_at(b: &[u8], o: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(b.get(o..o + 2).ok_or_else(|| crate::Error::Format("PAM: outside the file".into()))?.try_into().unwrap()))
}
fn u32_at(b: &[u8], o: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(b.get(o..o + 4).ok_or_else(|| crate::Error::Format("PAM/PMO: outside the file".into()))?.try_into().unwrap()))
}
fn f32_at(b: &[u8], o: usize) -> Result<f32> {
    Ok(f32::from_bits(u32_at(b, o)?))
}

/// Parses a PAM pack's clips (empty slots skipped). Version 1 (BBS: names inline) and 2 (names in a string table).
pub fn parse_pam(b: &[u8]) -> Result<Vec<PamClip>> {
    if b.get(..4) != Some(b"PAM\0") {
        return bad("PAM: no magic");
    }
    let count = u32_at(b, 4)? as usize;
    let version = u16_at(b, 0xE)?;
    let mut out = Vec::new();
    for i in 0..count {
        let e = 0x10 + i * 16;
        let off = u32_at(b, e)? as usize;
        if off == 0 {
            continue;
        }
        // Version 1 (BBS FM, Terra's packs): the entry is `u32 offset` + `char[12]` name inline. Version 2: the name
        // is at an offset in a string table. `OpenKh@41ac7b5:OpenKh.Bbs/Pam.cs`; Checked (files).
        let name = if version == 1 {
            let n = b.get(e + 4..e + 16).ok_or_else(|| crate::Error::Format("PAM: name outside".into()))?;
            String::from_utf8_lossy(&n[..n.iter().position(|&c| c == 0).unwrap_or(12)]).into_owned()
        } else {
            let no = u32_at(b, e + 4)? as usize;
            let end = b.get(no..).and_then(|s| s.iter().position(|&c| c == 0)).ok_or_else(|| crate::Error::Format("PAM: name outside".into()))?;
            String::from_utf8_lossy(&b[no..no + end]).into_owned()
        };
        let fps = u8_at(b, off + 2)?;
        let nb = u8_at(b, off + 6)? as usize;
        let frames = u16_at(b, off + 8)?;
        let mut p = off + 12;
        let mut masks = Vec::with_capacity(nb);
        for _ in 0..nb {
            masks.push(u16_at(b, p)?);
            p += 2;
        }
        let mut bones = Vec::with_capacity(nb);
        for mask in masks {
            let mut chans: [Option<Channel>; 9] = Default::default();
            for (c, slot) in chans.iter_mut().enumerate() {
                if mask >> c & 1 == 0 {
                    continue;
                }
                let (max, min) = (f32_at(b, p)?, f32_at(b, p + 4)?);
                p += 8;
                let kc = if frames > 255 {
                    p += 2;
                    u16_at(b, p - 2)? as usize
                } else {
                    p += 1;
                    u8_at(b, p - 1)? as usize
                };
                let mut keys = Vec::new();
                if kc != 1 {
                    for z in 0..kc {
                        let fid = if kc == frames as usize {
                            z as u16
                        } else if frames > 255 {
                            p += 2;
                            u16_at(b, p - 2)?
                        } else {
                            p += 1;
                            u8_at(b, p - 1)? as u16
                        };
                        keys.push((fid, u16_at(b, p)?));
                        p += 2;
                    }
                }
                *slot = Some(Channel { max, min, keys });
            }
            bones.push(chans);
        }
        out.push(PamClip { name, fps, frames, bones });
    }
    Ok(out)
}

/// A PMO v4 model's skeleton, as a Re:CoM-style skeleton (its rest local matrices).
pub fn parse_skeleton(b: &[u8]) -> Result<RecomSkeleton> {
    if b.get(..4) != Some(b"PMO\0") {
        return bad("PMO: no magic");
    }
    let sk = u32_at(b, 0xC)? as usize;
    if b.get(sk..sk + 4) != Some(b"BON\0") {
        return bad("PMO: no skeleton");
    }
    let n = u16_at(b, sk + 8)? as usize;
    let mut bones = Vec::with_capacity(n);
    for i in 0..n {
        let o = sk + 16 + i * 160;
        let parent = u16_at(b, o + 4)?;
        let name = b.get(o + 16..o + 32).ok_or_else(|| crate::Error::Format("PMO: bone outside".into()))?;
        let name = String::from_utf8_lossy(&name[..name.iter().position(|&c| c == 0).unwrap_or(16)]).into_owned();
        let m = |k: usize| -> Result<f64> { Ok(f32_at(b, o + 32 + 4 * k)? as f64) };
        let r: M3 = [[m(0)?, m(1)?, m(2)?], [m(4)?, m(5)?, m(6)?], [m(8)?, m(9)?, m(10)?]];
        let t = [m(12)?, m(13)?, m(14)?];
        bones.push(RBone { name, parent: if parent == 0xFFFF { -1 } else { parent as i16 }, local: Aff { r, t } });
    }
    Ok(RecomSkeleton { bones })
}

/// A PAM clip as a Re:CoM clip on its own (Terra's) skeleton, at its own rate: one row per frame (translation, rotation,
/// scale 1).
pub fn to_clip(c: &PamClip, name: &str) -> Clip {
    let frames = c.frames.max(1) as usize;
    let rows: Vec<Vec<[f64; 9]>> = (0..frames)
        .map(|f| {
            (0..c.bones.len())
                .map(|b| {
                    let (t, r, _) = c.local(b, f as f64);
                    [t[0], t[1], t[2], r[0], r[1], r[2], 1.0, 1.0, 1.0]
                })
                .collect()
        })
        .collect();
    // The fps is 60 / (d + 1) with d in `flags` bits 2..4 (`crate::ann`).
    let d = (60 / c.fps.max(1) as u32).saturating_sub(1).min(7) as u8;
    let template = Clip {
        name: crate::ann::name16(name),
        bones: c.bones.len() as i16,
        frames: frames as i16,
        loop_start: 0,
        flags: d << 2,
        has_blend: 2,
        blend: 8,
        blend_pad: [0; 2],
        mask_pad: Vec::new(),
        channels: Vec::new(),
        trailer: vec![0; 4],
    };
    crate::riku_skin::encode_rows(&template, &rows)
}

/// Re:CoM Riku's bone → Terra's (Checked (files): the names match for the whole body, the rest rotations within 0..9 deg except
/// the collars (90 deg) and the weapon bone (120 deg), which the world-rotation retarget handles). Riku's bones with no Terra counterpart
/// (fingers, belts, cuffs, twist bones, the Soul Eater chain under `wep0`, `neck1` / `neck2`, `chest2`) take his
/// battle stance under their parent.
pub const RIKU_FROM_TERRA: &[(&str, &str)] = &[
    ("Root", "Root"),
    ("ex_1010_sk", "center"),
    ("hip", "koshi"),
    ("r_femur", "R_momo"),
    ("r_tibia", "R_sune"),
    ("r_foot", "R_ashi1"),
    ("r_toes", "R_ashi2"),
    ("l_femur", "L_momo"),
    ("l_tibia", "L_sune"),
    ("l_foot", "L_ashi1"),
    ("l_toes", "L_ashi2"),
    ("waist", "hara"),
    ("chest1", "mune"),
    ("neck3", "kubi"),
    ("head", "atama"),
    ("r_collar", "R_kata"),
    ("r_uparm", "R_ude1"),
    ("r_foarm", "R_ude2"),
    ("r_hand", "R_te"),
    ("l_collar", "L_kata"),
    ("l_uparm", "L_ude1"),
    ("l_foarm", "L_ude2"),
    ("l_hand", "L_te"),
    ("wep0", "R_buki"),
];

/// The bones whose local translation moves with Terra's (target, source; a target's sources add up): Riku's body
/// `ex_1010_sk` with Terra's `center` and `koshi` (Terra moves the body by `koshi` and `hara` together, Checked (files);
/// Re:CoM by `ex_1010_sk`, the parent of `hip` and `waist`).
pub const CARRIED_TRANSLATION: &[(&str, &str)] = &[("ex_1010_sk", "center"), ("ex_1010_sk", "koshi")];

/// Terra's clips on the ground that the mod uses (the thrust `315`, the raise `317`, the guard `354`): their feet are
/// kept on the floor (`riku_skin`). Checked (files): posed on Riku's skeleton.
pub const GROUND_CLIPS: &[&str] = &["315", "317", "354"];

/// The retarget map (Riku's bones → Terra's indices).
pub fn riku_map(riku: &RecomSkeleton, src: &RecomSkeleton) -> Vec<Option<usize>> {
    riku.bones.iter().map(|b| RIKU_FROM_TERRA.iter().find(|(r, _)| *r == b.name).and_then(|(_, d)| src.find(d))).collect()
}

/// The world rotation of every bone of a Terra clip at frame `t` (tests).
pub fn world_rotations(c: &PamClip, skel: &RecomSkeleton, t: f64) -> Vec<M3> {
    let mut out: Vec<M3> = Vec::with_capacity(skel.bones.len());
    for (i, b) in skel.bones.iter().enumerate() {
        let (_, r, _) = c.local(i, t);
        let l = rot_xyz(r[0], r[1], r[2]);
        out.push(if b.parent < 0 { l } else { m3(&l, &out[b.parent as usize]) });
    }
    out
}
