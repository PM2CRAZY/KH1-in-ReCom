//! KH1 motion → Re:CoM clip: KH1's final pose every 60 Hz frame (`kh1-anim`), collapsed onto
//! Re:CoM's 135 bones the way Re:CoM's own bakes encode it, written as a 60 fps clip.
//!
//! Re:CoM's bone composition (`FUN_14023EB00(bone, parent)`, Checked (decomp)): the unscaled
//! world rotation is `R_local · U_parent`; the world rows are that times the bone's own scale
//! (absolute, not inherited); the world position is `T_local · W_parent + P_parent` with the
//! parent's *scaled* world. So for target worlds (U, S, P) per bone the locals are
//! `R = U · Uᵀ_parent`, `S = S`, `T = (P − P_parent) · Uᵀ_parent · diag(1 / S_parent)`.
//! KH1's world matrices have the same structure (`kh1-posing.md`, "Scale semantics"): rows =
//! accumulated scale × unscaled rotation.
//!
//! Special bones (Checked (files), `docs/motion-conversion.md`): `Root` (0) is KH1's root
//! travel (the last helper's translation, `Pose::root_travel`), identity rotation; Re:CoM only
//! samples it when the motion is played with the root flag. `ex_0010_sk` (1) sits at KH1
//! bone 0's position with an identity rotation (Re:CoM moved bone 0's rotation into `hip`).

#![allow(clippy::needless_range_loop)]

use kh1_anim::Rig;
use kh1_formats::mdls::Skeleton;
use kh1_formats::mset::Motion;

use crate::ann::{name16, Channel, Clip, Curve};
use crate::math::{dist, euler_xyz, m3, rot_xyz, tr, unscale, vm, Aff, M3, I3};
use crate::mdl::RecomSkeleton;
use crate::{bad, Result};

/// Re:CoM bone → KH1 bone, by rest pose (port of the research script `skel_map.py`, as in
/// `kh1-anim`'s `recom_bake` test): same rest position (×0.01, < 0.1 mm) and, where one
/// exists, same rest world rotation (< 1e-3); among those the deepest that is still an
/// ancestor of every child's match. `Root` has none.
pub fn bone_map(k: &Skeleton, rs: &RecomSkeleton) -> Vec<Option<usize>> {
    let mut kw: Vec<Aff> = Vec::with_capacity(k.bones.len());
    for b in &k.bones {
        let mut r = rot_xyz(b.rot[0] as f64, b.rot[1] as f64, b.rot[2] as f64);
        for i in 0..3 {
            for v in r[i].iter_mut() {
                *v *= b.scale[i] as f64;
            }
        }
        let l = Aff { r, t: [b.trans[0] as f64, b.trans[1] as f64, b.trans[2] as f64] };
        let w = match b.parent {
            Some(p) if (p as usize) < kw.len() => l.mul(&kw[p as usize]),
            _ => l,
        };
        kw.push(w);
    }
    let rw = rs.rest_world();
    let rb = &rs.bones;
    let good: Vec<(Vec<usize>, Vec<usize>)> = rw
        .iter()
        .map(|w| {
            let cands: Vec<usize> = (0..kw.len()).filter(|&j| dist(w.t, [kw[j].t[0] * 0.01, kw[j].t[1] * 0.01, kw[j].t[2] * 0.01]) < 1e-4).collect();
            let g = cands.iter().copied().filter(|&j| (0..3).all(|a| (0..3).all(|c| (w.r[a][c] - kw[j].r[a][c]).abs() < 1e-3))).collect();
            (cands, g)
        })
        .collect();
    let anc = |mut j: usize| {
        let mut out = vec![j];
        while let Some(p) = k.bones[j].parent {
            j = p as usize;
            out.push(j);
        }
        out
    };
    (0..rb.len())
        .map(|i| {
            let g = if good[i].1.is_empty() { &good[i].0 } else { &good[i].1 };
            if g.is_empty() {
                return None;
            }
            let kids: Vec<usize> = (0..rb.len()).filter(|&j| rb[j].parent == i as i16 && !good[j].1.is_empty()).collect();
            let ok: Vec<usize> = g.iter().copied().filter(|&kb| kids.iter().all(|&j| anc(*good[j].1.last().unwrap()).contains(&kb))).collect();
            if ok.is_empty() {
                g.iter().max().copied()
            } else {
                ok.iter().max().copied()
            }
        })
        .collect()
}

/// Weapon bone → KH1 bone, bones 2.. only: among the KH1 descendants of `hand`, the bone with
/// the same rest world rotation (< 0.01) and the nearest rest position within 1 cm (PL0002's rest
/// grip sits ≈ 5 mm from KH1's; Checked (files)), the deepest of equals. Each mapped bone whose
/// parent is also mapped by this rule must then have the same rest segment length (×0.01,
/// < 0.1 mm), else it is left unmapped.
pub fn weapon_bone_map(k: &Skeleton, ws: &RecomSkeleton, hand: usize) -> Vec<Option<usize>> {
    let mut kw: Vec<Aff> = Vec::with_capacity(k.bones.len());
    for b in &k.bones {
        let mut r = rot_xyz(b.rot[0] as f64, b.rot[1] as f64, b.rot[2] as f64);
        for i in 0..3 {
            for v in r[i].iter_mut() {
                *v *= b.scale[i] as f64;
            }
        }
        let l = Aff { r, t: [b.trans[0] as f64, b.trans[1] as f64, b.trans[2] as f64] };
        kw.push(match b.parent {
            Some(p) if (p as usize) < kw.len() => l.mul(&kw[p as usize]),
            _ => l,
        });
    }
    let under = |mut j: usize| {
        while let Some(p) = k.bones[j].parent {
            if p as usize == hand {
                return true;
            }
            j = p as usize;
        }
        false
    };
    let rw = ws.rest_world();
    let mut map: Vec<Option<usize>> = vec![None; ws.bones.len()];
    for b in 2..ws.bones.len() {
        let w = &rw[b];
        let mut best: Option<(f64, usize)> = None;
        for j in (0..k.bones.len()).filter(|&j| under(j)) {
            let d = dist(w.t, [kw[j].t[0] * 0.01, kw[j].t[1] * 0.01, kw[j].t[2] * 0.01]);
            let rot_ok = (0..3).all(|a| (0..3).all(|c| (w.r[a][c] - kw[j].r[a][c]).abs() < 0.01));
            if d < 0.01 && rot_ok && best.is_none_or(|(bd, _)| d < bd - 1e-9 || (d - bd).abs() <= 1e-9) {
                best = Some((d, j));
            }
        }
        map[b] = best.map(|(_, j)| j);
        let p = ws.bones[b].parent;
        if p >= 2 {
            if let (Some(j), Some(pj)) = (map[b], map[p as usize]) {
                let lw = { let t = ws.bones[b].local.t; (t[0] * t[0] + t[1] * t[1] + t[2] * t[2]).sqrt() };
                let lk = dist(kw[j].t, kw[pj].t) * 0.01;
                if (lw - lk).abs() > 1e-4 {
                    map[b] = None;
                }
            }
        }
    }
    map
}

/// How one KH1 motion becomes a clip.
#[derive(Clone, Debug, PartialEq)]
pub struct ClipSpec {
    /// Clip name (`p001_161` …; must agree with its slot, `FUN_1401A47E0`).
    pub name: String,
    /// Loop: `N` frames, loop start 1. Else `N + 1` frames (KH1 holds at `t = N`).
    pub looped: bool,
    /// KH1 request-map bit 15: root travel goes into `Root`; else it stays in `ex_0010_sk`.
    pub root_motion: bool,
    /// Default blend written in the clip (60 Hz frames).
    pub blend: u16,
}

/// Re:CoM-space target of one bone at one frame.
#[derive(Clone, Copy, Debug)]
pub struct Target {
    /// Unscaled world rotation.
    pub u: M3,
    /// World scale (row lengths).
    pub s: [f64; 3],
    /// World position (metres, Y up).
    pub p: [f64; 3],
}

/// Converter state: the KH1 rig template, Re:CoM's skeleton and the bone map.
#[derive(Clone, Debug)]
pub struct Converter {
    rig: Rig,
    mdls: Vec<u8>,
    skel: RecomSkeleton,
    map: Vec<Option<usize>>,
}

/// Tolerances under which a channel is written as a constant (its spread is below a u16 step
/// of any real channel: 1e-5 deg, 0.1 µm, 1e-6 scale).
const CONST_EPS: [f64; 3] = [1e-7, 1e-5, 1e-6];

impl Converter {
    /// From KH1's `.mdls` and Re:CoM's skeleton. Checks the map covers every Re:CoM bone but
    /// `Root`, that bones 0/1 are `Root`/`ex_0010_sk`, and that every parent is mapped.
    pub fn new(mdls: &[u8], skel: RecomSkeleton) -> Result<Converter> {
        let kskel = Skeleton::parse(mdls)?;
        let rig = Rig::from_mdls(mdls)?;
        if skel.bones.len() < 2 || skel.bones[0].name != "Root" || skel.bones[1].name != "ex_0010_sk" {
            return bad("Re:CoM skeleton: bone 0/1 are not Root/ex_0010_sk");
        }
        let map = bone_map(&kskel, &skel);
        let missing: Vec<&str> = (1..skel.bones.len()).filter(|&i| map[i].is_none()).map(|i| skel.bones[i].name.as_str()).collect();
        if !missing.is_empty() {
            return bad(format!("Re:CoM bones with no KH1 match: {missing:?}"));
        }
        Ok(Converter { rig, mdls: mdls.to_vec(), skel, map })
    }

    /// Re:CoM bone → KH1 bone.
    pub fn map(&self) -> &[Option<usize>] {
        &self.map
    }

    /// Re:CoM's skeleton.
    pub fn skeleton(&self) -> &RecomSkeleton {
        &self.skel
    }

    /// A fresh KH1 rig (rest state).
    pub fn rig(&self) -> Rig {
        self.rig.clone()
    }

    /// Re:CoM-space targets of every Re:CoM bone for KH1's pose of `m` at `t` (60 Hz frames).
    pub fn targets(&self, rig: &mut Rig, m: &Motion, t: f32, root_motion: bool) -> Vec<Target> {
        let pose = rig.pose(m, t);
        let n = self.skel.bones.len();
        let mut out = Vec::with_capacity(n);
        for b in 0..n {
            let tg = match (b, self.map[b]) {
                (0, _) | (_, None) => {
                    let rt = pose.root_travel;
                    let p = if root_motion { [rt[0] as f64 * 0.01, -rt[1] as f64 * 0.01, -rt[2] as f64 * 0.01] } else { [0.0; 3] };
                    Target { u: I3, s: [1.0; 3], p }
                }
                (1, Some(k)) => {
                    let w = Aff::kh1_to_recom(&pose.world[k]);
                    let (_, s) = unscale(&w.r);
                    Target { u: out_u(&out, 0), s, p: w.t }
                }
                (_, Some(k)) => {
                    let w = Aff::kh1_to_recom(&pose.world[k]);
                    let (u, s) = unscale(&w.r);
                    Target { u, s, p: w.t }
                }
            };
            out.push(tg);
        }
        out
    }

    /// Re:CoM locals (T metres, R Euler radians, S) from targets; `prev` keeps Euler curves
    /// continuous.
    pub fn locals(&self, tg: &[Target], prev: Option<&[[f64; 9]]>) -> Vec<[f64; 9]> {
        locals_for(&self.skel, tg, None, prev)
    }

    /// Converts one KH1 motion.
    pub fn convert(&self, m: &Motion, spec: &ClipSpec) -> Result<Clip> {
        let frames = clip_frames(m, spec)?;
        let mut rig = self.rig();
        rig.start_motion(m);
        let mut rows: Vec<Vec<[f64; 9]>> = Vec::with_capacity(frames);
        for f in 0..frames {
            let tg = self.targets(&mut rig, m, f as f32, spec.root_motion);
            let l = self.locals(&tg, rows.last().map(|v| v.as_slice()));
            rows.push(l);
        }
        Ok(encode_clip(spec, &rows))
    }

    /// The Keyblade's skeleton (`PLAYER/0002/PL0002.MDL`) prepared for conversion. `attach` is
    /// the Sora bone the weapon's bone 0 hangs from (`WeaponR`).
    ///
    /// Map (Checked (files)): bones 2.. by rest pose ([`weapon_bone_map`]: `wep0` = KH1 282,
    /// `k9`…`k1` = 283…291, the blade bones); bone 1 (`ex_5010_sk`, the grip, identity
    /// rotation in every Re:CoM clip) = the KH1 child of the hand bone on the way to `wep0`'s
    /// bone (280); bone 0 (`Root`) = the attach bone itself (never sampled: the weapon plays
    /// with rootFlag 0, `FUN_1401F7180`).
    pub fn weapon(&self, skel: RecomSkeleton, attach: &str) -> Result<WeaponRig> {
        let Some(sora_bone) = self.skel.find(attach) else { return bad(format!("Sora has no bone {attach}")) };
        let Some(hand) = self.map[sora_bone] else { return bad(format!("{attach} has no KH1 bone")) };
        if skel.bones.len() < 3 {
            return bad("weapon skeleton has fewer than 3 bones");
        }
        let kskel = Skeleton::parse(&self.mdls)?;
        let mut map = weapon_bone_map(&kskel, &skel, hand);
        map[0] = Some(hand);
        let Some(first) = map[2] else { return bad(format!("weapon bone {} has no KH1 match", skel.bones[2].name)) };
        let mut j = first;
        let grip = loop {
            match self.rig.parent(j) {
                Some(p) if p == hand => break j,
                Some(p) => j = p,
                None => return bad(format!("KH1 bone {first} is not under the hand bone {hand}")),
            }
        };
        map[1] = Some(grip);
        let missing: Vec<&str> = (0..skel.bones.len()).filter(|&i| map[i].is_none()).map(|i| skel.bones[i].name.as_str()).collect();
        if !missing.is_empty() {
            return bad(format!("weapon bones with no KH1 match: {missing:?}"));
        }
        Ok(WeaponRig { skel, map })
    }

    /// Weapon-space targets for KH1's current pose (the rig must have been posed): bone 0 = the
    /// attach bone's world (scale 1), bone 1 = the grip's position with bone 0's rotation,
    /// others = their KH1 bone.
    pub fn weapon_targets(&self, w: &WeaponRig, rig: &Rig) -> Vec<Target> {
        let world = &rig.current().world;
        let mut out: Vec<Target> = Vec::with_capacity(w.skel.bones.len());
        for (b, k) in w.map.iter().enumerate() {
            let wk = Aff::kh1_to_recom(&world[k.unwrap_or(0)]);
            let (u, s) = unscale(&wk.r);
            out.push(match b {
                0 => Target { u, s: [1.0; 3], p: wk.t },
                1 => Target { u: out[0].u, s, p: wk.t },
                _ => Target { u, s, p: wk.t },
            });
        }
        out
    }

    /// Converts one KH1 motion into the weapon's clip (same frames and loop as Sora's).
    pub fn convert_weapon(&self, w: &WeaponRig, m: &Motion, spec: &ClipSpec) -> Result<Clip> {
        let frames = clip_frames(m, spec)?;
        let mut rig = self.rig();
        rig.start_motion(m);
        let mut rows: Vec<Vec<[f64; 9]>> = Vec::with_capacity(frames);
        for f in 0..frames {
            rig.pose(m, f as f32);
            let tg = self.weapon_targets(w, &rig);
            // Bone 0's parent is Sora's attach bone, whose world bone 0 copies: local identity.
            let l = locals_for(&w.skel, &tg, Some(&tg[0]), rows.last().map(|v| v.as_slice()));
            rows.push(l);
        }
        Ok(encode_clip(spec, &rows))
    }
}

/// The Keyblade's skeleton and its KH1 bones.
#[derive(Clone, Debug)]
pub struct WeaponRig {
    /// `PL0002.MDL`'s skeleton.
    pub skel: RecomSkeleton,
    /// Weapon bone → KH1 bone (bone 0 = the hand bone, 1 = the grip).
    pub map: Vec<Option<usize>>,
}

fn clip_frames(m: &Motion, spec: &ClipSpec) -> Result<usize> {
    let n = m.frames as usize;
    let frames = if spec.looped { n } else { n + 1 };
    if frames == 0 || frames > i16::MAX as usize {
        return bad(format!("{}: {frames} frames", spec.name));
    }
    Ok(frames)
}

/// Locals of `skel`'s bones from targets by `FUN_14023EB00`'s rule; `root_parent` is the
/// target of bone 0's external parent (the weapon's attach bone), else the origin.
pub fn locals_for(skel: &RecomSkeleton, tg: &[Target], root_parent: Option<&Target>, prev: Option<&[[f64; 9]]>) -> Vec<[f64; 9]> {
    let mut out = Vec::with_capacity(tg.len());
    for (b, bone) in skel.bones.iter().enumerate() {
        let me = &tg[b];
        let (pu, ps, pp) = if bone.parent < 0 {
            root_parent.map_or((I3, [1.0; 3], [0.0; 3]), |p| (p.u, p.s, p.p))
        } else {
            let p = &tg[bone.parent as usize];
            (p.u, p.s, p.p)
        };
        let rl = m3(&me.u, &tr(&pu));
        let near = prev.map(|p| [p[b][3], p[b][4], p[b][5]]);
        let e = euler_xyz(&rl, near);
        let d = [me.p[0] - pp[0], me.p[1] - pp[1], me.p[2] - pp[2]];
        let tl = vm(d, &tr(&pu));
        let t = [tl[0] / ps[0], tl[1] / ps[1], tl[2] / ps[2]];
        out.push([t[0], t[1], t[2], e[0], e[1], e[2], me.s[0], me.s[1], me.s[2]]);
    }
    out
}

/// A clip from per-frame locals.
fn encode_clip(spec: &ClipSpec, rows: &[Vec<[f64; 9]>]) -> Clip {
    let nb = rows[0].len();
    let scale = rows.iter().any(|r| r.iter().any(|b| (6..9).any(|c| (b[c] - 1.0).abs() > CONST_EPS[2])));
    let chans = if scale { 9 } else { 6 };
    let mut channels = Vec::with_capacity(nb * chans);
    for b in 0..nb {
        for c in 0..chans {
            let kind = c / 3; // 0 t, 1 r, 2 s
            let vals: Vec<f64> = rows.iter().map(|r| if kind == 1 { r[b][c].to_degrees() } else { r[b][c] }).collect();
            channels.push(encode_channel(&vals, CONST_EPS[kind]));
        }
    }
    Clip {
        name: name16(&spec.name),
        bones: nb as i16,
        frames: rows.len() as i16,
        loop_start: if spec.looped { 1 } else { 0 },
        flags: if scale { 2 } else { 0 },
        has_blend: 2,
        blend: spec.blend,
        blend_pad: [0; 2],
        mask_pad: vec![0; (((nb * chans + 7) >> 3) + 3) & !3],
        channels,
        trailer: vec![0; 4],
    }
}

fn out_u(out: &[Target], i: usize) -> M3 {
    out.get(i).map_or(I3, |t| t.u)
}

/// One channel from its per-frame values (file units): a constant when the spread is within
/// `eps`, else a linear curve with a key on every frame except the inside of runs of equal
/// quantised values (dropping those is exact: Re:CoM's linear segment between equal values
/// returns that value).
pub fn encode_channel(vals: &[f64], eps: f64) -> Channel {
    let lo = vals.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = vals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if hi - lo <= eps {
        let first = vals[0] as f32;
        let v = if vals.iter().all(|&x| x as f32 == first) { first } else { ((lo + hi) * 0.5) as f32 };
        // +0.0 for −0.0: the loader tags non-zero constant bits with `| 1` (FUN_14027ABC0).
        return Channel::constant(if v == 0.0 { 0.0 } else { v });
    }
    let base = lo as f32;
    let range = ((hi - base as f64) as f32).max(f32::MIN_POSITIVE);
    let q: Vec<u16> = vals.iter().map(|&v| ((v - base as f64) / range as f64 * 65535.0).round().clamp(0.0, 65535.0) as u16).collect();
    let n = q.len();
    let mut keys = Vec::with_capacity(n);
    let mut qs = Vec::with_capacity(n);
    for k in 0..n {
        if k > 0 && k + 1 < n && q[k - 1] == q[k] && q[k] == q[k + 1] {
            continue;
        }
        keys.push(k as u16);
        qs.push(q[k]);
    }
    let nk = keys.len();
    Channel::Curve(Curve {
        const_slot: (vals[0] as f32).to_bits(),
        keys,
        base,
        range,
        q: qs,
        bits: vec![0xFF; (((nk + 7) >> 3) + 1) & !1],
        idx_pad: 0,
        block_pad: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_encoding_is_exact_at_frames() {
        let vals: Vec<f64> = (0..300).map(|f| if (100..150).contains(&f) { 2.0 } else { (f as f64 * 0.1).sin() }).collect();
        let Channel::Curve(cv) = encode_channel(&vals, 1e-6) else { panic!() };
        assert!(cv.keys.len() < 300);
        let step = cv.range as f64 / 65535.0;
        for (f, v) in vals.iter().enumerate() {
            assert!((cv.eval(f as f64, false) - v).abs() <= step * 0.5 + 1e-7, "frame {f}");
        }
        assert!(matches!(encode_channel(&[1.0, 1.0, 1.0], 1e-6), Channel::Const { .. }));
    }
}
