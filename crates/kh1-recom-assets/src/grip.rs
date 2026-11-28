//! Riku's right hand closed on his weapon's handle (0.198.9; fork owner, 2026-11-26: "melhorar o encaixe da espada na mão do Riku tanto
//! darkmode quanto skin default, acho que tem muita folga na mão é não é acurado com a realidade"; the normal Riku's grip in every mode:
//! "Always"; "precisa ficar natural", "e anatomicamente correto", "sem dentruir com as animações, lógico"; "sim" to taking the grip from
//! his own clips).
//!
//! - Where (the owner's screenshots, KH1 gameplay, standing in battle; Checked (renders `hand_grip_checks`, the battle stance
//!   `p002_031`, Riku's action row 0x2A)): an open "C": the thumb tip 2.7 cm from the index (test `riku_skin::closed_grip_search`).
//! - The grip (Checked (files, same test)): Riku's own clip `p002_053`, frame 16, holds the handle with a closed fist (thumb tip 0.2 cm
//!   from the index; renders `src53_*`). Dark Riku's own pack: the frame of his clips with the thumb nearest the index.
//! - [`close`]: per clip and frame, the 15 right-finger bones (`r_fing1..5a/b/c`, local rotations under `r_hand` / each other) are turned
//!   toward the grip's by a weight that grows with the frame's "C" opening (0 at [`OPEN_FROM`], 1 at [`OPEN_FULL`]), smoothed over the
//!   frames: a hand the clip already closes stays as it is; the rest of the body and the hand's own motion are untouched.
//! - A first try (bending each finger at its joints until it met the handle) closed Dark Riku's hand but not Riku's: his fingers already
//!   touch the handle's side, the opening is between the thumb and the index.

use crate::ann::{Ann, Clip};
use crate::math::{euler_xyz, rot_xyz, Aff};
use crate::mdl::{split_parts, MdlPart, RecomSkeleton};
use crate::Result;

/// The thumb tip to index distance (metres) at which the grip starts being blended in …
pub const OPEN_FROM: f64 = 0.008;
/// … and is all in.
pub const OPEN_FULL: f64 = 0.018;

/// Above this opening (metres) the handle is not in the hand (the clip lays the hand along the weapon, `p002_078..081`; the
/// model-reviewer of 0.198.9: an empty fist): the weight fades from [`OPEN_FULL`] … to 0 here.
pub const NOT_HOLDING: f64 = 0.06;
/// The width of the fade below [`NOT_HOLDING`] (metres): all in up to 4.5 cm (Dark Riku's open "C" in the battle stance is 4.5).
pub const FADE: f64 = 0.015;
/// The most the thumb-index opening may grow over the clip's own (metres).
pub const MAX_WIDER: f64 = 0.005;
/// The deepest a finger may go into the handle's surface (metres) before the weight is lowered.
pub const MAX_DEPTH: f64 = 0.003;

thread_local! {
    /// Tests: builds without the grip on this thread (`riku_skin::build_with`).
    pub static OFF: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// What [`close`] did.
#[derive(Debug, Default, Clone)]
pub struct GripReport {
    /// The grip's source: clip name, frame, its thumb-index distance (metres).
    pub source: (String, usize, f64),
    /// Clips changed.
    pub clips: usize,
    /// The largest weight used.
    pub max_weight: f64,
    /// The most a frame's opening grew over the clip's own (metres; ≤ [`MAX_WIDER`]).
    pub worst_wider: f64,
}

fn finger_bones(sk: &RecomSkeleton) -> Vec<usize> {
    (1..=5).flat_map(|n| ["c", "b", "a"].map(|s| sk.find(&format!("r_fing{n}{s}")))).flatten().collect()
}

fn row(c: &Clip, f: usize) -> Vec<[f64; 9]> {
    (0..c.bones as usize).map(|b| { let x = c.sample(b, f as f64); [x.t[0], x.t[1], x.t[2], x.r[0], x.r[1], x.r[2], x.s[0], x.s[1], x.s[2]] }).collect()
}

fn local(r: &[f64; 9]) -> Aff {
    let mut m = rot_xyz(r[3], r[4], r[5]);
    for (line, sc) in m.iter_mut().zip([r[6], r[7], r[8]]) {
        for v in line.iter_mut() {
            *v *= sc;
        }
    }
    Aff { r: m, t: [r[0], r[1], r[2]] }
}

/// The thumb tip's distance to the index's middle and tip segments (metres), in `r_hand`'s frame, for one frame's rows.
fn opening(sk: &RecomSkeleton, verts: &[(usize, [f64; 3])], row: &[[f64; 9]]) -> Option<f64> {
    let hand = sk.find("r_hand")?;
    let chain = |b: usize| -> Aff {
        let mut path = vec![];
        let mut x = b;
        while x != hand && sk.bones[x].parent >= 0 {
            path.push(x);
            x = sk.bones[x].parent as usize;
        }
        let mut m = Aff { r: crate::math::I3, t: [0.0; 3] };
        for &p in path.iter() {
            m = m.mul(&local(&row[p]));
        }
        m
    };
    let (t1a, i2b, i2a) = (sk.find("r_fing1a")?, sk.find("r_fing2b")?, sk.find("r_fing2a")?);
    let (mt, mb, ma) = (chain(t1a), chain(i2b), chain(i2a));
    let tip: Vec<[f64; 3]> = verts.iter().filter(|(b, _)| *b == t1a).map(|(_, p)| mt.point(*p)).collect();
    let idx: Vec<[f64; 3]> = verts.iter().filter(|(b, _)| *b == i2b || *b == i2a).map(|(b, p)| if *b == i2b { mb.point(*p) } else { ma.point(*p) }).collect();
    if tip.is_empty() || idx.is_empty() {
        return None;
    }
    Some(tip.iter().map(|a| idx.iter().map(|b| crate::math::dist(*a, *b)).fold(f64::MAX, f64::min)).fold(f64::MAX, f64::min))
}

/// Closes the right hand on the weapon in every clip of `ann` (the model `mdl`) with the grip of `src` (a pack on the same skeleton:
/// the model's own), picked as its frame with the thumb nearest the index.
pub fn close(ann: &mut Ann, mdl: &[u8], src: &Ann) -> Result<GripReport> {
    let mut rep = GripReport::default();
    let sk = RecomSkeleton::parse(mdl)?;
    let fb = finger_bones(&sk);
    if fb.len() != 15 {
        return Ok(rep);
    }
    let parts = split_parts(mdl)?;
    let verts: Vec<(usize, [f64; 3])> = parts
        .iter()
        .filter_map(|pb| MdlPart::parse_bytes(pb).ok())
        .filter(|p| p.textures.len() > 1)
        .flat_map(|p| p.list_a.into_iter().flat_map(|k| k.verts.into_iter()))
        .filter(|v| fb.contains(&v.bone()))
        .map(|v| (v.bone(), v.pos().map(|x| x as f64)))
        .collect();
    let mut best: Option<(f64, String, usize, Vec<[f64; 9]>)> = None;
    for s in &src.slots {
        let Some(c) = &s.clip else { continue };
        if c.bones as usize != sk.bones.len() {
            continue;
        }
        for f in 0..c.frames.max(1) as usize {
            let r = row(c, f);
            if let Some(d) = opening(&sk, &verts, &r) {
                if best.as_ref().is_none_or(|b| d < b.0) {
                    best = Some((d, c.name_str(), f, r));
                }
            }
        }
    }
    let Some((d0, name, f0, grip)) = best else { return Ok(rep) };
    rep.source = (name, f0, d0);
    let gr: Vec<crate::math::M3> = fb.iter().map(|&b| rot_xyz(grip[b][3], grip[b][4], grip[b][5])).collect();
    // the handle: wep0's x axis, the median radius of the weapon's vertices within 7 cm of its origin (rings at its ends)
    let w0 = sk.find("wep0");
    let hand = sk.find("r_hand");
    let mut rad: Vec<f64> = parts.iter().filter_map(|pb| MdlPart::parse_bytes(pb).ok()).flat_map(|p| p.list_a.into_iter().flat_map(|k| k.verts.into_iter())).filter(|v| Some(v.bone()) == w0).map(|v| v.pos().map(|x| x as f64)).filter(|p| p[0].abs() < 0.07).map(|p| (p[1] * p[1] + p[2] * p[2]).sqrt()).filter(|r| *r < 0.03).collect();
    rad.sort_by(|a, b| a.total_cmp(b));
    let rh = rad.get(rad.len() / 2).copied().unwrap_or(0.0);
    let rel = |row: &[[f64; 9]], b: usize| -> Aff {
        let mut path = vec![];
        let mut x = b;
        while Some(x) != hand && sk.bones[x].parent >= 0 { path.push(x); x = sk.bones[x].parent as usize; }
        let mut m = Aff { r: crate::math::I3, t: [0.0; 3] };
        for &p in path.iter() { m = m.mul(&local(&row[p])); }
        m
    };
    let depth = |row: &[[f64; 9]]| -> f64 {
        let (Some(w0), true) = (w0, rh > 0.0) else { return 0.0 };
        let wm = rel(row, w0);
        let inv = crate::kh2_hair_rig::rigid_inv(&Aff { r: crate::math::unscale(&wm.r).0, t: wm.t });
        let mut worst = f64::MIN;
        for &b in &fb {
            let m = rel(row, b);
            for (_, p) in verts.iter().filter(|(vb, _)| *vb == b) {
                let q = inv.point(m.point(*p));
                if q[0].abs() < 0.07 { worst = worst.max(rh - (q[1] * q[1] + q[2] * q[2]).sqrt()); }
            }
        }
        worst
    };    let grip_depth = depth(&grip);

    for s in ann.slots.iter_mut() {
        let Some(c) = s.clip.as_ref() else { continue };
        if c.bones as usize != sk.bones.len() {
            continue;
        }
        let n = c.frames.max(1) as usize;
        let mut rows: Vec<Vec<[f64; 9]>> = (0..n).map(|f| row(c, f)).collect();
        let opens: Vec<Option<f64>> = rows.iter().map(|r| opening(&sk, &verts, r)).collect();
        let raw: Vec<f64> = opens.iter().map(|o| o.map_or(0.0, |d| {
            let up = ((d - OPEN_FROM) / (OPEN_FULL - OPEN_FROM)).clamp(0.0, 1.0);
            let down = ((NOT_HOLDING - d) / FADE).clamp(0.0, 1.0);
            up.min(down)
        })).collect();
        // the clip's own finger motion is kept: the grip is an offset from its least-open holding frame (0.198.9 review: a full
        // replacement froze the fingers in guard, item use and some attacks)
        let least = (0..n).filter(|&f| raw[f] > 0.0 || opens[f].is_some_and(|d| d <= OPEN_FULL)).min_by(|&a, &b| opens[a].unwrap_or(9.0).total_cmp(&opens[b].unwrap_or(9.0)));
        let offset: Vec<crate::math::M3> = match least {
            Some(l) => fb.iter().enumerate().map(|(k, &b)| crate::math::m3(&gr[k], &crate::math::tr(&rot_xyz(rows[l][b][3], rows[l][b][4], rows[l][b][5])))).collect(),
            None => vec![crate::math::I3; fb.len()],
        };
        if raw.iter().all(|w| *w == 0.0) {
            continue;
        }
        // 0.198.9 review: one weight for the whole clip, from its median opening (a per-frame weight closed and opened the fist inside a
        // clip, twice a walk cycle); the per-frame limits below only lower it
        let clip_w = {
            let mut o: Vec<f64> = opens.iter().flatten().copied().collect();
            o.sort_by(|a, b| a.total_cmp(b));
            o.get(o.len() / 2).map_or(0.0, |&d| {
                let up = ((d - OPEN_FROM) / (OPEN_FULL - OPEN_FROM)).clamp(0.0, 1.0);
                let down = ((NOT_HOLDING - d) / FADE).clamp(0.0, 1.0);
                up.min(down)
            })
        };
        let _ = &raw;
        // the weight for the whole clip: the largest (from `clip_w` down in tenths) that keeps every frame within the limits — no finger
        // deeper into the handle than the clip's own or the grip pose's own + MAX_DEPTH, the opening no wider than the clip's own +
        // MAX_WIDER (0.198.9 reviews: a per-frame lowering moved still fingers, 8 deg/frame in the skin's jump)
        let orig = rows.clone();
        let posed = |f: usize, w: f64| -> Vec<[f64; 9]> {
            let mut r = orig[f].clone();
            for (k, &b) in fb.iter().enumerate() {
                let cur = rot_xyz(orig[f][b][3], orig[f][b][4], orig[f][b][5]);
                let target = crate::math::m3(&offset[k], &cur);
                let m = crate::kh2_hair_rig::rot_toward(&cur, &target, w);
                let e = euler_xyz(&m, Some([orig[f][b][3], orig[f][b][4], orig[f][b][5]]));
                r[b][3..6].copy_from_slice(&e);
            }
            r
        };
        // each finger's tip no farther from the handle's axis than the clip's own + MAX_WIDER (fourth review: in the skin's `p002_010`
        // fingers 3-5 stood straight off the handle, which the thumb-index opening does not see)
        let tips_ok = |r: &[[f64; 9]], o: &[[f64; 9]]| -> bool {
            let Some(w0) = w0 else { return true };
            let far = |row: &[[f64; 9]], b: usize| -> f64 {
                let wm = rel(row, w0);
                let inv = crate::kh2_hair_rig::rigid_inv(&Aff { r: crate::math::unscale(&wm.r).0, t: wm.t });
                let m = rel(row, b);
                verts.iter().filter(|(vb, _)| *vb == b).map(|(_, p)| { let q = inv.point(m.point(*p)); (q[1] * q[1] + q[2] * q[2]).sqrt() }).fold(0.0, f64::max)
            };
            (1..=5).filter_map(|k| sk.find(&format!("r_fing{k}a"))).all(|b| far(r, b) <= far(o, b) + MAX_WIDER)
        };
        let ok = |w: f64| (0..n).all(|f| {
            let r = posed(f, w);
            let d0 = depth(&orig[f]).max(grip_depth).max(0.0);
            let wider = opening(&sk, &verts, &r).zip(opens[f]).is_some_and(|(a, b)| a > b + MAX_WIDER);
            depth(&r) <= d0 + MAX_DEPTH && !wider && tips_ok(&r, &orig[f])
        });
        let mut w = clip_w;
        while w > 0.0 && !ok(w) {
            w = (w - 0.1).max(0.0);
        }
        if w <= 0.0 {
            continue;
        }
        rep.max_weight = rep.max_weight.max(w);
        if std::env::var("GRIP_DEBUG").is_ok() { eprintln!("GRIP {}: clip weight {clip_w:.2} -> {w:.2}", c.name_str()); }
        for (f, r) in rows.iter_mut().enumerate() {
            *r = posed(f, w);
        }
        let enc = crate::riku_skin::encode_rows(c, &rows);
        if std::env::var("GRIP_DEBUG").is_ok() { eprintln!("GRIP {} after: opening rows {:?}, encoded {:?}", c.name_str(), opening(&sk, &verts, &rows[0]).map(|d| d * 100.0), opening(&sk, &verts, &row(&enc, 0)).map(|d| d * 100.0)); }
        for f in 0..n {
            if let (Some(a), Some(b)) = (opening(&sk, &verts, &row(&enc, f)), opens[f]) {
                rep.worst_wider = rep.worst_wider.max(a - b);
            }
        }
        s.clip = Some(enc);
        rep.clips += 1;
    }
    Ok(rep)
}
