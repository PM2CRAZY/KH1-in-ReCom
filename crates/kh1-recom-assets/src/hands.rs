//! Riku's right hand relaxed when his weapon is put away (0.198.10; fork owner, 2026-11-26: "ficou PERFEITO empunhando a espada,
//! mas sem espada na mão pode deixar o movimento das mãos mais natural, no andar corrida").
//!
//! - Where (Checked (renders `hand_grip_checks` with `NO_WEAPON`)): in the field walk and run (`p002_002`, `p002_003`) the right hand
//!   keeps the handle's hollow "C" with nothing in it; the left hand hangs relaxed.
//! - When those clips play (Checked (exe)): `0x1401F9380` picks rows 2 / 3 when player `+0x418` bit 1 is set, else 0x12 / 0x13 (and
//!   0x2B / 0x2C in the battle stance); the player's draw-weapon `0x1401F5D30` clears that bit (`and dword [+0x418], ~2` at
//!   `0x1401F5D4A`), Riku's put-away sets it (`or … 2` at `0x140216090`, just before his draw `0x1402160A0`). So rows 2 / 3 play only
//!   with the weapon put away. The field idle (row 1) also plays with the weapon out (`0x1401F9380`, cases 2 / 0x1F) and is left as it is.
//! - [`relax`]: in those clips, each of the 15 right-finger bones takes its left twin's rotation at the same frame, mirrored: with
//!   `P_b = Rw_r · S · Rw_lᵀ` from the rest pose (`S` the model's left-right mirror), the right local is `P_c · L_l · P_parent⁻¹`. The
//!   right hand itself, the arm and the rest of the body are untouched; the fingers move with the gait as the left hand's do.
//! - Clearance (model-reviewer of 0.198.10, no-go: on the Dark Form skin, run frame 11, the opened fingers went ~2 cm through the coat
//!   tail): per frame, the right-finger vertices may come no nearer the coat's triangles (bones `*_skt*`) than [`CLEAR`] or the
//!   clip's own distance, whichever is less; the mirror's weight is lowered in tenths where they would, then eased (the lowest weight
//!   within [`EASE`] frames, averaged over the same window), so the hand closes back toward the clip's own shape smoothly there. The
//!   re-review found a tip through the tail and out the far side, which the distance counts as clear: no finger vertex may change
//!   side of the coat either ([`Clearance::sides`]: the parity of the coat crossings from the wrist, against the clip's own pose).

use crate::ann::{Ann, Clip};
use crate::math::{euler_xyz, m3, rot_xyz, tr, unscale, M3};
use crate::mdl::{split_parts, MdlPart, RecomSkeleton};
use crate::Result;

/// The clips played only with the weapon put away (Riku's action rows 2 / 3, the field walk and run): slot, clip name suffix.
pub const SHEATHED: [(usize, &str); 2] = [(2, "_002"), (3, "_003")];

/// The least distance (metres) the relaxed fingers keep from the coat tails, unless the clip's own hand is nearer.
pub const CLEAR: f64 = 0.015;
/// Where the clip's own hand is nearer the coat than [`CLEAR`], the relaxed fingers still keep this much (metres; second re-review:
/// "no nearer than its own" at 3 mm let a tip through the tail at 1.2 mm).
pub const MIN_GAP: f64 = 0.006;
/// Frames on either side over which a lowered weight is eased.
pub const EASE: usize = 3;

thread_local! {
    /// Tests: builds without [`relax`] on this thread.
    pub static OFF: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// Tests: builds without [`loosen`] only (the 0.198.12 pack) on this thread; the renders use the env var `NO_LOOSE`.
    pub static LOOSE_OFF: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// What [`relax`] did.
#[derive(Debug, Default, Clone)]
pub struct RelaxReport {
    /// Clip names changed.
    pub clips: Vec<String>,
    /// Per clip, the lowest eased weight of the mirror (1 = the left hand's shape in every frame).
    pub min_weight: Vec<f64>,
    /// The worst mismatch of the rest pose's mirror over the finger pairs (metres, world positions): the model's symmetry.
    pub asymmetry: f64,
}

fn finger_pairs(sk: &RecomSkeleton) -> Vec<(usize, usize)> {
    (1..=5).flat_map(|n| ["c", "b", "a"].map(|s| sk.find(&format!("l_fing{n}{s}")).zip(sk.find(&format!("r_fing{n}{s}"))))).flatten().collect()
}

/// Relaxes the right hand in [`SHEATHED`] clips of `ann` (a pack on `mdl`'s skeleton) by mirroring the left hand's fingers.
pub fn relax(ann: &mut Ann, mdl: &[u8]) -> Result<RelaxReport> {
    let mut rep = RelaxReport::default();
    if OFF.with(|x| x.get()) {
        return Ok(rep);
    }
    let sk = RecomSkeleton::parse(mdl)?;
    let pairs = finger_pairs(&sk);
    let (Some(lh), Some(rh)) = (sk.find("l_hand"), sk.find("r_hand")) else { return Ok(rep) };
    if pairs.len() != 15 {
        return Ok(rep);
    }
    let w = sk.rest_world();
    // the mirror axis: the coordinate in which the two hands' rest positions differ in sign
    let ax = (0..3).max_by(|&a, &b| (w[lh].t[a] - w[rh].t[a]).abs().total_cmp(&(w[lh].t[b] - w[rh].t[b]).abs())).unwrap_or(0);
    let mut s = crate::math::I3;
    s[ax][ax] = -1.0;
    let mirror = |p: [f64; 3]| { let mut q = p; q[ax] = -q[ax]; q };
    let rot = |b: usize| unscale(&w[b].r).0;
    let p_of = |l: usize, r: usize| -> M3 { m3(&m3(&rot(r), &s), &tr(&rot(l))) };
    let mut all = pairs.clone();
    all.push((lh, rh));
    for &(l, r) in &all {
        rep.asymmetry = rep.asymmetry.max(crate::math::dist(mirror(w[l].t), w[r].t));
    }
    let p: Vec<(M3, M3, usize, usize)> = pairs
        .iter()
        .map(|&(l, r)| {
            let par_r = sk.bones[r].parent as usize;
            let par_l = sk.bones[l].parent as usize;
            (p_of(l, r), tr(&p_of(par_l, par_r)), l, r)
        })
        .collect();
    let clear = Clearance::new(&sk, mdl)?;
    for (slot, suffix) in SHEATHED {
        let Some(c) = ann.slots.get(slot).and_then(|s| s.clip.as_ref()) else { continue };
        if c.bones as usize != sk.bones.len() || !c.name_str().ends_with(suffix) {
            continue;
        }
        let n = c.frames.max(1) as usize;
        let orig: Vec<Vec<[f64; 9]>> = (0..n).map(|f| row(c, f)).collect();
        let posed = |f: usize, k: f64| -> Vec<[f64; 9]> {
            let mut r = orig[f].clone();
            for (pc, pp_inv, l, b) in &p {
                let cur = rot_xyz(orig[f][*b][3], orig[f][*b][4], orig[f][*b][5]);
                let ll = rot_xyz(orig[f][*l][3], orig[f][*l][4], orig[f][*l][5]);
                let target = m3(&m3(pc, &ll), pp_inv);
                let m = if k >= 1.0 { target } else { crate::kh2_hair_rig::rot_toward(&cur, &target, k) };
                let e = euler_xyz(&m, Some([orig[f][*b][3], orig[f][*b][4], orig[f][*b][5]]));
                r[*b][3..6].copy_from_slice(&e);
            }
            r
        };
        let eased = weights(n, &orig, &posed, &clear);
        let rows: Vec<Vec<[f64; 9]>> = (0..n).map(|f| posed(f, eased[f])).collect();
        let enc = crate::riku_skin::encode_rows(c, &rows);
        rep.clips.push(c.name_str());
        rep.min_weight.push(eased.iter().copied().fold(1.0, f64::min));
        if std::env::var("HANDS_DEBUG").is_ok() {
            eprintln!("HANDS {}: weights {:?}", c.name_str(), eased.iter().map(|w| (w * 100.0).round() / 100.0).collect::<Vec<_>>());
        }
        if let Some(s) = ann.slots.get_mut(slot) {
            s.clip = Some(enc);
        }
    }
    Ok(rep)
}

/// The Dark Form skin's clips whose left hand takes Dark Riku's own loose hand (0.198.13): skin slot, skin clip suffix, Dark Riku's
/// own clip. The walk and run take his walk `p003_002` and run `p003_003`; the idle takes his walk's hand too (it holds one pose
/// through that clip): his own idle `p003_001` (the Dark Mode, battle only) has the thumb splayed (tip 0.85 hand lengths from the
/// index's middle joint, rest 0.48) and its last thumb joint bent back 12 deg (probe `probe_curl`).
pub const LOOSE: [(usize, &str, &str); 3] = [(1, "_001", "p003_002"), (2, "_002", "p003_002"), (3, "_003", "p003_003")];

/// What [`loosen`] did.
#[derive(Debug, Default, Clone)]
pub struct LoosenReport {
    /// Per clip changed: its name, Dark Riku's clip, the frame shift that lines his arm swing up with the clip's (his frames), the
    /// lowest weight.
    pub clips: Vec<(String, String, usize, f64)>,
}

/// The Dark Form skin only (0.198.13; fork owner, 2026-11-26: "as mãos o RIku Darkmode são muito estão muito grosseiras no andar ...
/// também precisa melhorar, parado a mão esquerda tmb fica meio esquisita"; "no riku dark mode a animação de andar até sem esada ficou
/// estranha ninguém caminha fazendo um C com a mão"; "não atravessar nem mesmo deixar muita folga.. e também deixar natural!").
///
/// - Found (Checked (probe `probe_curl`, renders `hand_0201_v12_*`)): Riku's idle / walk / run curl the fingers into a fist (walk: the
///   index's three joints 118-212 deg, rest 45); his thumb closes over it. Retargeted onto Dark Riku's skeleton the rotations are
///   the same, but his thumb rests 70 deg further out (sum 147 against Riku's 77), so it stays off the curled fingers: the "C". Dark
///   Riku's own walk and run hold the hand loose (walk `p003_002`: index 81 deg, constant; run `p003_003` 60-96).
/// - Change: in [`LOOSE`] clips, the 15 left-finger bones take Dark Riku's own clip's local rotations at the same phase (his frame
///   `f · n_own / n + shift`, `shift` lining his `l_hand` swing in `waist`'s frame up with the clip's); the hand, arm and body keep
///   Riku's motion. [`relax`] then mirrors the walk's and run's left onto the right. The idle's right hand (weapon out) is untouched.
/// - Clearance: the left fingers against the coat tails as [`relax`]'s right ([`weights`]).
pub fn loosen(ann: &mut Ann, mdl: &[u8], own: &Ann) -> Result<LoosenReport> {
    let mut rep = LoosenReport::default();
    // `NO_LOOSE` (renders): the 0.198.12 pack (grip and relax, no loosen), the "before" of this change
    if OFF.with(|x| x.get()) || LOOSE_OFF.with(|x| x.get()) || std::env::var("NO_LOOSE").is_ok() {
        return Ok(rep);
    }
    let sk = RecomSkeleton::parse(mdl)?;
    let left: Vec<usize> = finger_pairs(&sk).iter().map(|p| p.0).collect();
    let (Some(lh), Some(waist)) = (sk.find("l_hand"), sk.find("waist")) else { return Ok(rep) };
    if left.len() != 15 {
        return Ok(rep);
    }
    let clear = Clearance::side(&sk, mdl, true)?;
    // the left hand in waist's frame
    let swing = |r: &[[f64; 9]]| {
        let w = world(&sk, r);
        let wr = crate::math::Aff { r: unscale(&w[waist].r).0, t: w[waist].t };
        crate::kh2_hair_rig::rigid_inv(&wr).point(w[lh].t)
    };
    for (slot, suffix, src) in LOOSE {
        let Some(c) = ann.slots.get(slot).and_then(|s| s.clip.as_ref()) else { continue };
        let Some(o) = own.find(src).and_then(|i| own.slots[i].clip.as_ref()) else { continue };
        if c.bones as usize != sk.bones.len() || o.bones != c.bones || !c.name_str().ends_with(suffix) {
            continue;
        }
        let n = c.frames.max(1) as usize;
        let no = o.frames.max(1) as usize;
        let orig: Vec<Vec<[f64; 9]>> = (0..n).map(|f| row(c, f)).collect();
        let own_rows: Vec<Vec<[f64; 9]>> = (0..no).map(|f| row(o, f)).collect();
        let at = |f: usize, s: usize| (f * no / n + s) % no;
        let sw: Vec<[f64; 3]> = orig.iter().map(|r| swing(r)).collect();
        let osw: Vec<[f64; 3]> = own_rows.iter().map(|r| swing(r)).collect();
        let shift = (0..no).min_by(|&a, &b| {
            let cost = |s: usize| (0..n).map(|f| crate::math::dist(sw[f], osw[at(f, s)])).sum::<f64>();
            cost(a).total_cmp(&cost(b))
        }).unwrap_or(0);
        let posed = |f: usize, k: f64| -> Vec<[f64; 9]> {
            let mut r = orig[f].clone();
            let g = at(f, shift);
            for &b in &left {
                let cur = rot_xyz(orig[f][b][3], orig[f][b][4], orig[f][b][5]);
                let target = rot_xyz(own_rows[g][b][3], own_rows[g][b][4], own_rows[g][b][5]);
                let m = if k >= 1.0 { target } else { crate::kh2_hair_rig::rot_toward(&cur, &target, k) };
                let e = euler_xyz(&m, Some([orig[f][b][3], orig[f][b][4], orig[f][b][5]]));
                r[b][3..6].copy_from_slice(&e);
            }
            r
        };
        let eased = weights(n, &orig, &posed, &clear);
        let rows: Vec<Vec<[f64; 9]>> = (0..n).map(|f| posed(f, eased[f])).collect();
        let enc = crate::riku_skin::encode_rows(c, &rows);
        let low = eased.iter().copied().fold(1.0, f64::min);
        if std::env::var("HANDS_DEBUG").is_ok() {
            eprintln!("LOOSE {} from {src} shift {shift}: weights {:?}", c.name_str(), eased.iter().map(|w| (w * 100.0).round() / 100.0).collect::<Vec<_>>());
        }
        rep.clips.push((c.name_str(), src.to_string(), shift, low));
        if let Some(s) = ann.slots.get_mut(slot) {
            s.clip = Some(enc);
        }
    }
    Ok(rep)
}

/// Per frame, the mirror's (or the loose shape's) weight that keeps the clearance: the largest in tenths, eased over [`EASE`] frames.
fn weights(n: usize, orig: &[Vec<[f64; 9]>], posed: &dyn Fn(usize, f64) -> Vec<[f64; 9]>, clear: &Clearance) -> Vec<f64> {
    let near = |row: &[[f64; 9]]| clear.near(row);
    // per frame, the largest weight (in tenths) that keeps the clearance
    let raw: Vec<f64> = (0..n)
        .map(|f| {
            let Some(d0) = near(&orig[f]) else { return 1.0 };
            // second re-review: near the clip's own contact (d0 a few mm) "no nearer than its own" still let a tip show through
            // the tail from the outer side (1.2 mm away); the relaxed shape keeps CLEAR or gives way to the clip's own hand
            let need = d0.max(MIN_GAP).min(CLEAR);
            let side0 = clear.sides(&orig[f]);
            let bad = |k: f64| { let r = posed(f, k); near(&r).is_some_and(|d| d < need) || clear.sides(&r) != side0 };
            let mut k: f64 = 1.0;
            while k > 0.0 && bad(k) {
                k = ((k - 0.1) * 10.0).round() / 10.0;
            }
            k.max(0.0)
        })
        .collect();
    // eased: the lowest within EASE frames (the clip loops), then averaged over the same window (never above a frame's own)
    let at = |v: &[f64], i: isize| v[i.rem_euclid(n as isize) as usize];
    let win = EASE as isize;
    let low: Vec<f64> = (0..n as isize).map(|i| (-win..=win).map(|d| at(&raw, i + d)).fold(1.0, f64::min)).collect();
    let eased: Vec<f64> = (0..n as isize).map(|i| (-win..=win).map(|d| at(&low, i + d)).sum::<f64>() / (2 * win + 1) as f64).collect();
    // an in-between weight is not always within the limits where the full one is: each eased weight is checked too, lowered in
    // tenths where it fails (the clip's own hand, weight 0, always passes)
    let eased: Vec<f64> = (0..n)
        .map(|f| {
            let Some(d0) = near(&orig[f]) else { return eased[f] };
            // in between, the fingers pass from the clip's own hand to the relaxed one: no nearer the coat than either end allows
            let need = d0.max(MIN_GAP).min(CLEAR).min(d0);
            let side0 = clear.sides(&orig[f]);
            let ok = |k: f64| { if k <= 0.0 { return true; } let r = posed(f, k); near(&r).is_none_or(|d| d >= need) && clear.sides(&r) == side0 };
            let mut k = eased[f];
            while !ok(k) {
                k = ((k - 0.1) * 10.0).floor() / 10.0;
            }
            k.max(0.0)
        })
        .collect();
    eased
}

/// The right fingers' distance to the coat tails (bones `*_skt*`) in a pose.
pub(crate) struct Clearance<'a> {
    sk: &'a RecomSkeleton,
    rh: usize,
    fingers: Vec<(usize, [f64; 3])>,
    coat: Vec<[(usize, [f64; 3]); 3]>,
}

impl<'a> Clearance<'a> {
    pub(crate) fn new(sk: &'a RecomSkeleton, mdl: &[u8]) -> Result<Clearance<'a>> {
        Self::side(sk, mdl, false)
    }

    /// The left fingers' (`left`) or the right fingers' distance to the coat tails (0.198.13: [`loosen`] moves the left hand).
    pub(crate) fn side(sk: &'a RecomSkeleton, mdl: &[u8], left: bool) -> Result<Clearance<'a>> {
        let fset: Vec<usize> = finger_pairs(sk).iter().map(|p| if left { p.0 } else { p.1 }).collect();
        let body: Vec<MdlPart> = split_parts(mdl)?.iter().filter_map(|pb| MdlPart::parse_bytes(pb).ok()).filter(|p| p.textures.len() > 1).collect();
        let fingers = body.iter().flat_map(|p| p.list_a.iter().flat_map(|k| k.verts.iter())).filter(|v| fset.contains(&v.bone())).map(|v| (v.bone(), v.pos().map(|x| x as f64))).collect();
        let coat = body
            .iter()
            .flat_map(|p| {
                let mut t = crate::ddd_hair::extract(&p.list_a).unwrap_or_default();
                if let Some(lb) = p.list_b.as_ref() {
                    t.extend(crate::ddd_hair::extract(lb).unwrap_or_default());
                }
                t
            })
            .filter(|t| t.v.iter().any(|v| sk.bones[v.bone()].name.contains("_skt")))
            .map(|t| t.v.map(|v| (v.bone(), v.pos().map(|x| x as f64))))
            .collect();
        Ok(Clearance { sk, rh: sk.find(if left { "l_hand" } else { "r_hand" }).unwrap_or(0), fingers, coat })
    }

    /// The least distance (metres) of a right-finger vertex to a coat triangle within 30 cm of the hand; None without one.
    pub(crate) fn near(&self, row: &[[f64; 9]]) -> Option<f64> {
        if self.coat.is_empty() {
            return None;
        }
        let w = world(self.sk, row);
        let fp: Vec<[f64; 3]> = self.fingers.iter().map(|(b, p)| w[*b].point(*p)).collect();
        let hp = w[self.rh].t;
        let mut best: Option<f64> = None;
        for t in &self.coat {
            let q = t.map(|(b, p)| w[b].point(p));
            if q.iter().all(|x| crate::math::dist(*x, hp) > 0.3) {
                continue;
            }
            for p in &fp {
                let d = point_tri(*p, q);
                best = Some(best.map_or(d, |b: f64| b.min(d)));
            }
        }
        best
    }

    /// Per right-finger vertex, whether the segment from `r_hand`'s origin to it crosses the coat an odd number of times (re-review of
    /// 0.198.10: an unsigned distance let a tip that had gone through the coat tail and out past [`CLEAR`] count as clear; a vertex
    /// whose parity differs from the clip's own pose is on the other side of the coat).
    pub(crate) fn sides(&self, row: &[[f64; 9]]) -> Vec<bool> {
        let w = world(self.sk, row);
        let hp = w[self.rh].t;
        let tris: Vec<[[f64; 3]; 3]> = self.coat.iter().map(|t| t.map(|(b, p)| w[b].point(p))).filter(|q| q.iter().any(|x| crate::math::dist(*x, hp) <= 0.3)).collect();
        self.fingers.iter().map(|(b, p)| { let v = w[*b].point(*p); tris.iter().filter(|q| seg_tri(hp, v, **q)).count() % 2 == 1 }).collect()
    }
}

/// Whether the segment `a`-`b` crosses the triangle `t` (Möller-Trumbore).
fn seg_tri(a: [f64; 3], b: [f64; 3], t: [[f64; 3]; 3]) -> bool {
    let sub = |x: [f64; 3], y: [f64; 3]| [x[0] - y[0], x[1] - y[1], x[2] - y[2]];
    let dot = |x: [f64; 3], y: [f64; 3]| x[0] * y[0] + x[1] * y[1] + x[2] * y[2];
    let cross = |x: [f64; 3], y: [f64; 3]| [x[1] * y[2] - x[2] * y[1], x[2] * y[0] - x[0] * y[2], x[0] * y[1] - x[1] * y[0]];
    let d = sub(b, a);
    let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
    let h = cross(d, e2);
    let det = dot(e1, h);
    if det.abs() < 1e-12 {
        return false;
    }
    let inv = 1.0 / det;
    let s = sub(a, t[0]);
    let u = dot(s, h) * inv;
    if !(0.0..=1.0).contains(&u) {
        return false;
    }
    let q = cross(s, e1);
    let v = dot(d, q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return false;
    }
    let k = dot(e2, q) * inv;
    (0.0..=1.0).contains(&k)
}

fn world(sk: &RecomSkeleton, row: &[[f64; 9]]) -> Vec<crate::math::Aff> {
    let mut w: Vec<crate::math::Aff> = Vec::with_capacity(sk.bones.len());
    for (i, b) in sk.bones.iter().enumerate() {
        let r = &row[i];
        let mut m = rot_xyz(r[3], r[4], r[5]);
        for (line, sc) in m.iter_mut().zip([r[6], r[7], r[8]]) {
            for v in line.iter_mut() {
                *v *= sc;
            }
        }
        let l = crate::math::Aff { r: m, t: [r[0], r[1], r[2]] };
        w.push(if b.parent < 0 { l } else { l.mul(&w[b.parent as usize]) });
    }
    w
}

/// The distance from `p` to the triangle `t` (closest point on the triangle; Ericson, Real-Time Collision Detection, 5.1.5).
fn point_tri(p: [f64; 3], t: [[f64; 3]; 3]) -> f64 {
    let sub = |a: [f64; 3], b: [f64; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let add = |a: [f64; 3], b: [f64; 3], k: f64| [a[0] + b[0] * k, a[1] + b[1] * k, a[2] + b[2] * k];
    let [a, b, c] = t;
    let (ab, ac, ap) = (sub(b, a), sub(c, a), sub(p, a));
    let (d1, d2) = (dot(ab, ap), dot(ac, ap));
    let q = if d1 <= 0.0 && d2 <= 0.0 {
        a
    } else {
        let bp = sub(p, b);
        let (d3, d4) = (dot(ab, bp), dot(ac, bp));
        let cp = sub(p, c);
        let (d5, d6) = (dot(ab, cp), dot(ac, cp));
        let (vc, vb, va) = (d1 * d4 - d3 * d2, d5 * d2 - d1 * d6, d3 * d6 - d5 * d4);
        if d3 >= 0.0 && d4 <= d3 {
            b
        } else if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
            add(a, ab, d1 / (d1 - d3))
        } else if d6 >= 0.0 && d5 <= d6 {
            c
        } else if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
            add(a, ac, d2 / (d2 - d6))
        } else if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
            add(b, sub(c, b), (d4 - d3) / ((d4 - d3) + (d5 - d6)))
        } else {
            let den = 1.0 / (va + vb + vc);
            add(add(a, ab, vb * den), ac, vc * den)
        }
    };
    crate::math::dist(p, q)
}

/// Per bone, its vertex farthest from the bone's origin (bone-local; the fingertip on a distal finger bone), from the body parts.
pub(crate) fn tips(sk: &RecomSkeleton, mdl: &[u8]) -> Vec<[f64; 3]> {
    let mut t = vec![[0.0; 3]; sk.bones.len()];
    let body: Vec<MdlPart> = split_parts(mdl).unwrap_or_default().iter().filter_map(|pb| MdlPart::parse_bytes(pb).ok()).filter(|p| p.textures.len() > 1).collect();
    for v in body.iter().flat_map(|p| p.list_a.iter().flat_map(|k| k.verts.iter())) {
        let (b, p) = (v.bone(), v.pos().map(|x| x as f64));
        if b < t.len() && crate::math::dist(p, [0.0; 3]) > crate::math::dist(t[b], [0.0; 3]) {
            t[b] = p;
        }
    }
    t
}

/// One finger's chain, base to end (`<side>_fing<n>{a,b,c}` ordered by depth).
pub(crate) fn chain(sk: &RecomSkeleton, side: &str, n: usize) -> Option<[usize; 3]> {
    let mut v: Vec<usize> = ["a", "b", "c"].iter().filter_map(|s| sk.find(&format!("{side}_fing{n}{s}"))).collect();
    if v.len() != 3 {
        return None;
    }
    let depth = |mut b: usize| { let mut d = 0; while sk.bones[b].parent >= 0 { b = sk.bones[b].parent as usize; d += 1; } d };
    v.sort_by_key(|&b| depth(b));
    Some([v[0], v[1], v[2]])
}

/// Per finger (1 thumb .. 5 little), the signed bend in degrees at its three joints (base / MCP, middle / PIP, end / DIP) against
/// the rest pose: positive = toward the palm. The palm's segment is the hand's origin to the finger's base joint; the last segment
/// ends at the fingertip ([`tips`]). The bend's sign is about the knuckle line (index base to little-finger base), as in the rest pose.
pub(crate) fn joints(sk: &RecomSkeleton, w: &[crate::math::Aff], side: &str, tip: &[[f64; 3]]) -> Vec<[f64; 3]> {
    let bends = |w: &[crate::math::Aff]| -> Vec<[f64; 3]> {
        let Some(h) = sk.find(&format!("{side}_hand")) else { return Vec::new() };
        let sub = |a: [f64; 3], b: [f64; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let cross = |a: [f64; 3], b: [f64; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
        let (Some(i2), Some(i5)) = (chain(sk, side, 2), chain(sk, side, 5)) else { return Vec::new() };
        // the knuckle line, flipped on the right so that "toward the palm" has one sign on both hands
        let mut lat = sub(w[i5[0]].t, w[i2[0]].t);
        if side == "r" {
            lat = lat.map(|x| -x);
        }
        let signed = |a: [f64; 3], b: [f64; 3]| {
            let n = crate::math::dist(a, [0.0; 3]) * crate::math::dist(b, [0.0; 3]);
            let ang = if n < 1e-12 { 0.0 } else { (dot(a, b) / n).clamp(-1.0, 1.0).acos().to_degrees() };
            if dot(cross(a, b), lat) < 0.0 { -ang } else { ang }
        };
        (1..=5)
            .filter_map(|k| chain(sk, side, k))
            .map(|c| {
                let pts = [w[h].t, w[c[0]].t, w[c[1]].t, w[c[2]].t, w[c[2]].point(tip[c[2]])];
                [0, 1, 2].map(|i| signed(sub(pts[i + 1], pts[i]), sub(pts[i + 2], pts[i + 1])))
            })
            .collect()
    };
    let rest = bends(&sk.rest_world());
    bends(w).iter().zip(&rest).map(|(a, r)| [a[0] - r[0], a[1] - r[1], a[2] - r[2]]).collect()
}

pub(crate) fn row(c: &Clip, f: usize) -> Vec<[f64; 9]> {
    (0..c.bones as usize).map(|b| { let x = c.sample(b, f as f64); [x.t[0], x.t[1], x.t[2], x.r[0], x.r[1], x.r[2], x.s[0], x.s[1], x.s[2]] }).collect()
}

#[cfg(test)]
mod tests {
    /// The relaxed right fingers, in `r_hand`'s frame, are the left fingers in `l_hand`'s frame mirrored through the rest pose (worst
    /// over every frame of the field walk and run under 1 mm); only the right fingers change.
    #[test]
    fn relaxed_right_hand_mirrors_the_left() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = crate::install::keys(&g).unwrap();
        let arc = crate::install::recom_archive(&g).unwrap();
        let mdl = crate::install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        let orig = crate::ann::Ann::parse(&crate::install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.ANN").unwrap()).unwrap();
        let mut ann = orig.clone();
        let rep = super::relax(&mut ann, &mdl).unwrap();
        assert_eq!(rep.clips, ["p002_002", "p002_003"]);
        assert!(rep.asymmetry < 1e-4, "the rest pose is not mirror-symmetric: {}", rep.asymmetry);
        let sk = crate::mdl::RecomSkeleton::parse(&mdl).unwrap();
        let rw = sk.rest_world();
        let (lh, rh) = (sk.find("l_hand").unwrap(), sk.find("r_hand").unwrap());
        let ax = (0..3).max_by(|&a, &b| (rw[lh].t[a] - rw[rh].t[a]).abs().total_cmp(&(rw[lh].t[b] - rw[rh].t[b]).abs())).unwrap();
        let rigid = |a: &crate::math::Aff| crate::math::Aff { r: crate::math::unscale(&a.r).0, t: a.t };
        let pairs = super::finger_pairs(&sk);
        for (slot, _) in super::SHEATHED {
            let c = ann.slots[slot].clip.as_ref().unwrap();
            let o = orig.slots[slot].clip.as_ref().unwrap();
            let mut worst: f64 = 0.0;
            let mut body: f64 = 0.0;
            for f in 0..c.frames as usize {
                let w = crate::kh2_hair::tests::posed_world(&sk, c, f as f64);
                let wo = crate::kh2_hair::tests::posed_world(&sk, o, f as f64);
                let inv = |h: usize| crate::kh2_hair_rig::rigid_inv(&rigid(&w[h]));
                for &(l, r) in &pairs {
                    // the left finger carried to the rest pose's left hand, mirrored, read in the rest pose's right hand
                    let mut m = rigid(&rw[lh]).point(inv(lh).point(w[l].t));
                    m[ax] = -m[ax];
                    let pm = crate::kh2_hair_rig::rigid_inv(&rigid(&rw[rh])).point(m);
                    worst = worst.max(crate::math::dist(pm, inv(rh).point(w[r].t)));
                }
                for b in (0..sk.bones.len()).filter(|b| !pairs.iter().any(|p| p.1 == *b)) {
                    body = body.max(crate::math::dist(w[b].t, wo[b].t));
                }
            }
            assert!(worst < 1e-3, "slot {slot}: right finger {:.2} mm off the mirrored left", worst * 1000.0);
            assert!(body < 1e-3, "slot {slot}: a bone other than the right fingers moved {:.2} mm", body * 1000.0);
        }
    }

    /// The Dark Form skin (Riku's clips on Dark Riku's model, coat tails animated): the relaxed fingers keep [`super::CLEAR`] from the
    /// coat, or the clip's own distance where that is less (model-reviewer of 0.198.10: run frame 11 went through the coat tail).
    #[test]
    fn relaxed_skin_hand_keeps_clear_of_the_coat() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = crate::install::keys(&g).unwrap();
        let arc = crate::install::recom_archive(&g).unwrap();
        let mdl = crate::install::read_recom(&arc, &keys, "PLAYER/0201/PL0201.MDL").unwrap();
        let path = crate::riku_skin::path(121, "ANN");
        // before: the build's pack without the relax (the 0.198.9 grip on); after: the build's pack
        let pack = |off: bool| {
            super::OFF.with(|x| x.set(off));
            let b = crate::riku_skin::build_with(&arc, &keys, 101, 201, 121, None).unwrap();
            super::OFF.with(|x| x.set(false));
            crate::ann::Ann::parse(&b.files.iter().find(|f| f.0 == path).unwrap().1).unwrap()
        };
        let (before, after) = (pack(true), pack(false));
        let sk = crate::mdl::RecomSkeleton::parse(&mdl).unwrap();
        let d = super::Clearance::new(&sk, &mdl).unwrap();
        // the relaxed right fingers turn no faster than the left ones they mirror (+ 2 deg/frame)
        let ang = |a: &crate::math::M3, b: &crate::math::M3| { let d = crate::math::m3(&crate::math::tr(a), b); ((d[0][0] + d[1][1] + d[2][2] - 1.0) / 2.0).clamp(-1.0, 1.0).acos().to_degrees() };
        let rot = |c: &crate::ann::Clip, b: usize, f: usize| { let r = c.sample(b, f as f64).r; crate::math::rot_xyz(r[0], r[1], r[2]) };
        let pairs = super::finger_pairs(&sk);
        for (slot, _) in super::SHEATHED {
            let c = after.slots[slot].clip.as_ref().unwrap();
            let n = c.frames.max(1) as usize;
            let speed = |side: usize| (1..n).map(|f| pairs.iter().map(|p| { let b = if side == 0 { p.0 } else { p.1 }; ang(&rot(c, b, f - 1), &rot(c, b, f)) }).fold(0.0, f64::max)).fold(0.0, f64::max);
            let (l, r) = (speed(0), speed(1));
            assert!(r <= l + 2.0, "slot {slot}: right fingers {r:.1} deg/frame against the left's {l:.1}");
        }
        for (slot, _) in super::SHEATHED {
            let (o, c) = (before.slots[slot].clip.as_ref().unwrap(), after.slots[slot].clip.as_ref().unwrap());
            for f in 0..c.frames as usize {
                let (Some(d0), Some(d1)) = (d.near(&super::row(o, f)), d.near(&super::row(c, f))) else { continue };
                let (s0, s1) = (d.sides(&super::row(o, f)), d.sides(&super::row(c, f)));
                let flipped = s0.iter().zip(&s1).filter(|(a, b)| a != b).count();
                assert!(flipped == 0, "slot {slot} frame {f}: {flipped} finger vertices on the other side of the coat");
                // the relaxed shape keeps CLEAR; where it gives way the hand is the clip's own (its own distance)
                let changed = (0..c.bones as usize).any(|b| (0..3).any(|k| (super::row(o, f)[b][3 + k] - super::row(c, f)[b][3 + k]).abs() > 1e-3));
                assert!(!changed || d1 >= d0.min(super::CLEAR) - 1e-3, "slot {slot} frame {f}: fingers {:.1} mm from the coat (own {:.1})", d1 * 1000.0, d0 * 1000.0);
            }
        }
    }

    /// 0.198.13 (fork owner: "ninguém caminha fazendo um C com a mão"; "parado a mão esquerda tmb fica meio esquisita"): on the Dark
    /// Form skin, in the idle, walk and run, the loose hands (left in all three; right, its mirror, in the walk and run):
    /// - every finger joint (index to little; [`super::joints`]) within the hand's range (base 90, middle 100, end 80 deg, back no
    ///   more than 5), or no worse than the 0.198.12 pack (`NO_LOOSE`) in that frame (the run's fist frames, kept for the coat);
    /// - in the idle and walk, the thumb beside the index (its tip nearer the index's middle joint than in any 0.198.12 frame);
    /// - the left fingers: no vertex on the other side of the coat, and no nearer it than [`super::CLEAR`] or the clip's own;
    /// - the hand, arm and body unchanged. `PROBE=1` prints the joints per clip (also Riku's own pack and Dark Riku's).
    #[test]
    fn loose_skin_hands_are_relaxed() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = crate::install::keys(&g).unwrap();
        let arc = crate::install::recom_archive(&g).unwrap();
        let rd = |p: &str| crate::install::read_recom(&arc, &keys, p).unwrap();
        let pack = |no: bool| {
            super::LOOSE_OFF.with(|x| x.set(no));
            let b = crate::riku_skin::build_with(&arc, &keys, 101, 201, 121, None).unwrap();
            super::LOOSE_OFF.with(|x| x.set(false));
            (crate::ann::Ann::parse(&b.files.iter().find(|f| f.0 == crate::riku_skin::path(121, "ANN")).unwrap().1).unwrap(),
             crate::ann::Ann::parse(&b.files.iter().find(|f| f.0 == crate::riku_skin::path(101, "ANN")).unwrap().1).unwrap())
        };
        let ((before, riku_before), (after, riku_after)) = (pack(true), pack(false));
        let dm = rd("PLAYER/0201/PL0201.MDL");
        let sk = crate::mdl::RecomSkeleton::parse(&dm).unwrap();
        let tip = super::tips(&sk, &dm);
        let pw = |c: &crate::ann::Clip, f: usize| crate::kh2_hair::tests::posed_world(&sk, c, f as f64);
        let limit = [90.0, 100.0, 80.0];
        let fingers: Vec<usize> = super::finger_pairs(&sk).iter().flat_map(|p| [p.0, p.1]).collect();
        let clear = super::Clearance::side(&sk, &dm, true).unwrap();
        let thumb_gap = |w: &[crate::math::Aff]| {
            let (t, i2, i3) = (super::chain(&sk, "l", 1).unwrap(), super::chain(&sk, "l", 2).unwrap(), super::chain(&sk, "l", 3).unwrap());
            let h = sk.find("l_hand").unwrap();
            crate::math::dist(w[t[2]].point(tip[t[2]]), w[i2[1]].t) / crate::math::dist(w[h].t, w[i3[0]].t)
        };
        for (slot, _, _) in super::LOOSE {
            let (o, c) = (before.slots[slot].clip.as_ref().unwrap(), after.slots[slot].clip.as_ref().unwrap());
            let sides: &[&str] = if slot == 1 { &["l"] } else { &["l", "r"] };
            let (mut worst, mut gap_b, mut gap_a, mut near_a, mut near_b) = (0.0f64, f64::MAX, 0.0f64, f64::MAX, f64::MAX);
            for f in 0..c.frames as usize {
                let (wo, wc) = (pw(o, f), pw(c, f));
                for side in sides {
                    let (jo, jc) = (super::joints(&sk, &wo, side, &tip), super::joints(&sk, &wc, side, &tip));
                    for k in 1..5 {
                        for j in 0..3 {
                            let over = |v: f64| (v - limit[j]).max(-5.0 - v).max(0.0);
                            let (a, b) = (over(jc[k][j]), over(jo[k][j]));
                            assert!(a <= b + 0.5, "{} f{f} {side} finger {} joint {j}: {:.0} deg (0.198.12 {:.0})", c.name_str(), k + 1, jc[k][j], jo[k][j]);
                            worst = worst.max(a);
                        }
                    }
                }
                gap_b = gap_b.min(thumb_gap(&wo));
                gap_a = gap_a.max(thumb_gap(&wc));
                for b in (0..sk.bones.len()).filter(|b| !fingers.contains(b)) {
                    assert!(crate::math::dist(wo[b].t, wc[b].t) < 1e-3, "{} f{f}: {} moved", c.name_str(), sk.bones[b].name);
                }
                let (ro, rc) = (super::row(o, f), super::row(c, f));
                if let (Some(d0), Some(d1)) = (clear.near(&ro), clear.near(&rc)) {
                    assert_eq!(clear.sides(&ro), clear.sides(&rc), "{} f{f}: a left finger on the other side of the coat", c.name_str());
                    let changed = fingers.iter().any(|&b| (0..3).any(|k| (ro[b][3 + k] - rc[b][3 + k]).abs() > 1e-3));
                    assert!(!changed || d1 >= d0.min(super::CLEAR) - 1e-3, "{} f{f}: left fingers {:.1} mm from the coat (own {:.1})", c.name_str(), d1 * 1000.0, d0 * 1000.0);
                    near_a = near_a.min(d1);
                    near_b = near_b.min(d0);
                }
            }
            if slot != 3 {
                assert!(gap_a < gap_b, "{}: thumb tip {gap_a:.2} hand lengths from the index (0.198.12 nearest {gap_b:.2})", c.name_str());
            }
            eprintln!("{}: joints over range {worst:.0} deg, thumb-index gap max {gap_a:.2} (0.198.12 min {gap_b:.2}), left fingers to coat {:.1} mm (0.198.12 {:.1})", c.name_str(), near_a * 1000.0, near_b * 1000.0);
        }
        // Riku's default attire: his pack is not loosened
        for slot in [1usize, 2, 3] {
            let (a, b) = (riku_before.slots[slot].clip.as_ref().unwrap(), riku_after.slots[slot].clip.as_ref().unwrap());
            for f in 0..a.frames as usize {
                assert_eq!(super::row(a, f), super::row(b, f), "Riku's pack slot {slot} frame {f} changed");
            }
        }
        if std::env::var("PROBE").is_ok() {
            let rm = rd("PLAYER/0101/PL0101.MDL");
            let dark = crate::ann::Ann::parse(&rd("PLAYER/0201/PL0201.ANN")).unwrap();
            for (lab, mdl, ann) in [("riku", &rm, &riku_after), ("before", &dm, &before), ("after", &dm, &after), ("dark", &dm, &dark)] {
                let sk = crate::mdl::RecomSkeleton::parse(mdl).unwrap();
                let tip = super::tips(&sk, mdl);
                for slot in [1usize, 2, 3] {
                    let c = ann.slots[slot].clip.as_ref().unwrap();
                    for side in ["l", "r"] {
                        let rows: Vec<Vec<[f64; 3]>> = (0..c.frames as usize).map(|f| super::joints(&sk, &crate::kh2_hair::tests::posed_world(&sk, c, f as f64), side, &tip)).collect();
                        let txt: Vec<String> = (0..5).map(|k| (0..3).map(|j| { let v: Vec<f64> = rows.iter().map(|r| r[k][j]).collect(); format!("{:.0}..{:.0}", v.iter().copied().fold(f64::MAX, f64::min), v.iter().copied().fold(f64::MIN, f64::max)) }).collect::<Vec<_>>().join(" ")).collect();
                        eprintln!("J {lab:6} {} {side}: {}", c.name_str(), txt.join(" | "));
                    }
                }
            }
        }
    }
}
