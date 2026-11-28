//! The Dark Form skin's coat tails (0.198.8; fork owner, 2026-11-26: "no darkmode queria ver se é possível animar a saia do riku fica
//! muito estática", "tem como melhorar ainda mais a animação da saia?"; "yes" to the plan: Dark Riku's own tail motion where a clip's
//! role is known, a baked follow-through elsewhere).
//!
//! - Checked (files, test `riku_skin::skirt_probe`): Dark Riku's `PL0201` has 38 coat-tail bones (`*_skt*`); in the skin's clips
//!   (Riku's motions retargeted) they never leave their rest pose (0.0 deg in all 64 clips), while Dark Riku's own clips turn them
//!   up to 10.7 deg idle, 19.2 walk, 110 run, 120-130 jump / fall.
//! - Slots pair by number (`p002_NNN` ↔ `p003_NNN`); the locomotion roles also pair by length and loop (Checked (files): idle 1
//!   100 / 80 loop, walk 2 32 / 34 loop, run 3 22 / 20 loop, jump 4 17 / 17). [`COPY`] uses Dark Riku's slot for Riku's slots of
//!   the same role (the KH2 hair's roles, `kh2_hair_rig::CLIP_MAP`), time-scaled to the clip.
//! - Every other clip whose tails are still at rest gets a follow-through: each tail chain turns about its root, a damped spring
//!   driven by the root's acceleration (inertia), baked per frame ([`follow_through`]). Assumption: the spring's constants.
//! - Clips whose tails already move (Dark Riku's own casts and flip in the skin) are left as they are.

use crate::ann::{Ann, Clip};
use crate::math::{euler_xyz, m3, rot_xyz, tr, unscale, Aff, M3};
use crate::mdl::RecomSkeleton;
use crate::Result;

/// (Dark Riku's slot, Riku's slots of the same role): idle, walk, run, jump, apex, fall, land. The quick jump (slot 4) is right in game
/// (fork owner, on 0.198.8: "quickjump está perfeito tbm"); the held jump was not ("o problema foi pulando", "pulando segurando o
/// B"): it goes on through the apex, the fall loop and the landing, which took the follow-through from rest right after the copied
/// jump's spread tails. Checked (exe: the action tables `0x1407C6370` / `0x1407C67F0`, rows 4-7): Riku's jump / apex / fall / land
/// are `p002_004` / `_005` / `_006` / `_007`, Dark Riku's `p003_004` / `_006` / `_007` / `_008`.
pub const COPY: [(usize, &[usize]); 7] = [(1, &[1, 31, 79]), (2, &[2, 28, 32, 80]), (3, &[3, 21, 33]), (4, &[4, 26]), (6, &[5]), (7, &[6]), (8, &[7])];

/// The follow-through's spring: natural frequency (Hz), damping ratio, the lever (metres) the acceleration acts on. Assumption: from
/// the renders.
pub const SPRING: (f64, f64, f64) = (1.6, 0.35, 0.35);
/// The follow-through's limits (degrees): tip forward (toward the legs), tip back, sideways. 0.198.8 review (model-reviewer: the rear
/// thigh through the back tail in the first ground swing, frame 12, when the spring swung it forward to vertical): forward 15.
pub const SWING_LIMITS: (f64, f64, f64) = (15.0, 55.0, 35.0);
/// The follow-through's fastest turn (degrees per second). 0.198.8 review of the first fix: acceleration spikes (fast moves at 30
/// fps) took the spring from rest to its limit in one frame (26-53 deg steps in slots 26, 44, 45, 57); 300 = 10 deg a frame at 30.
pub const SWING_RATE: f64 = 300.0;

/// The frame a looped clip goes back to (`loop_start − 1`; 0 = the whole clip loops), `None` for a one-shot clip.
fn loop_from(c: &Clip) -> Option<usize> {
    c.looped().then(|| (c.loop_start_clamped() as usize).saturating_sub(1))
}

/// What [`animate`] did: clips given Dark Riku's tail motion, clips given the follow-through, clips left (tails already moving).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TailReport {
    /// Clips given Dark Riku's own tail motion.
    pub copied: usize,
    /// Clips given the follow-through.
    pub simulated: usize,
    /// Clips left as they were (their tails already move).
    pub kept: usize,
}

fn tail_bones(sk: &RecomSkeleton) -> Vec<usize> {
    sk.bones.iter().enumerate().filter(|(_, b)| b.name.contains("_skt")).map(|(i, _)| i).collect()
}

/// Per frame, per bone: (translation, rotation radians, scale), the clip's own samples.
fn rows_of(c: &Clip) -> Vec<Vec<[f64; 9]>> {
    (0..c.frames.max(1) as usize)
        .map(|f| {
            (0..c.bones as usize)
                .map(|b| {
                    let s = c.sample(b, f as f64);
                    [s.t[0], s.t[1], s.t[2], s.r[0], s.r[1], s.r[2], s.s[0], s.s[1], s.s[2]]
                })
                .collect()
        })
        .collect()
}

fn local(row: &[f64; 9]) -> Aff {
    let mut r = rot_xyz(row[3], row[4], row[5]);
    for (line, sc) in r.iter_mut().zip([row[6], row[7], row[8]]) {
        for v in line.iter_mut() {
            *v *= sc;
        }
    }
    Aff { r, t: [row[0], row[1], row[2]] }
}

fn world(sk: &RecomSkeleton, row: &[[f64; 9]]) -> Vec<Aff> {
    let mut w: Vec<Aff> = Vec::with_capacity(sk.bones.len());
    for (i, b) in sk.bones.iter().enumerate() {
        let l = if i < row.len() { local(&row[i]) } else { b.local };
        w.push(if b.parent < 0 { l } else { l.mul(&w[b.parent as usize]) });
    }
    w
}

/// Whether the clip's tails stay at their rest local rotation (within 0.01 rad) on every frame.
fn tails_still(rows: &[Vec<[f64; 9]>], sk: &RecomSkeleton, tails: &[usize]) -> bool {
    tails.iter().all(|&b| {
        let rest = euler_xyz(&unscale(&sk.bones[b].local.r).0, None);
        rows.iter().all(|r| (0..3).all(|k| {
            let d = (r[b][3 + k] - rest[k]).rem_euclid(std::f64::consts::TAU);
            d.min(std::f64::consts::TAU - d) < 0.01
        }))
    })
}

/// Dark Riku's tail channels of `src` on `rows` (time-scaled: a looped clip cycle onto cycle, else start to end).
fn copy_tails(rows: &mut [Vec<[f64; 9]>], c: &Clip, src: &Clip, tails: &[usize]) {
    let (n, m) = (rows.len().max(1) as f64, src.frames.max(1) as f64);
    for (f, row) in rows.iter_mut().enumerate() {
        let t = if c.looped() && src.looped() { f as f64 * m / n } else if n > 1.0 { f as f64 * (m - 1.0) / (n - 1.0) } else { 0.0 };
        // between two of Dark Riku's frames, the rotations are blended (not the Euler angles: across a ±180 deg wrap the angles'
        // straight blend swung the tails through the other way, a 123 deg step on the apex, 0.198.8)
        let (t0, w) = (t.floor(), t - t.floor());
        let t1 = if src.looped() { (t0 + 1.0) % m } else { (t0 + 1.0).min(m - 1.0) };
        for &b in tails {
            let (a, z) = (src.sample(b, t0).r, src.sample(b, t1).r);
            let r = crate::kh2_hair_rig::rot_toward(&rot_xyz(a[0], a[1], a[2]), &rot_xyz(z[0], z[1], z[2]), w);
            let e = euler_xyz(&r, Some(a));
            row[b][3..6].copy_from_slice(&e);
        }
    }
}

/// `v · M` turns `v` by `angle` (radians) about the unit `axis` (row vectors, as [`Aff`]).
fn axis_angle(axis: [f64; 3], angle: f64) -> M3 {
    let (s, c) = angle.sin_cos();
    let [x, y, z] = axis;
    let t = 1.0 - c;
    // the column-vector rotation matrix, transposed
    [
        [t * x * x + c, t * x * y + s * z, t * x * z - s * y],
        [t * x * y - s * z, t * y * y + c, t * y * z + s * x],
        [t * x * z + s * y, t * y * z - s * x, t * z * z + c],
    ]
}

/// The follow-through: each chain root (a tail bone whose parent is not one) turns about its origin by two damped springs, one about
/// the body's side axis (the tip back / forward) and one about its forward axis (sideways), driven by minus the root's acceleration
/// along the body's forward and side axes (the coat lags behind the body); the chain's other bones follow as its children. The body's
/// axes are the parent's (`hip` / `waist`) world axes that point forward (+z) and to the side (+x) at rest. A looped clip's loop
/// segment is simulated until it repeats (its end meets its start), the frames before it run from rest and are eased into it.
fn follow_through(rows: &mut [Vec<[f64; 9]>], c: &Clip, sk: &RecomSkeleton, tails: &[usize]) {
    let roots: Vec<usize> = tails.iter().copied().filter(|&b| !tails.contains(&(sk.bones[b].parent as usize))).collect();
    let dt = 1.0 / (c.fps() as f64).max(1.0);
    let (hz, zeta, lever) = SPRING;
    let (fwd_max, back_max, side_max) = (SWING_LIMITS.0.to_radians(), SWING_LIMITS.1.to_radians(), SWING_LIMITS.2.to_radians());
    let k = (std::f64::consts::TAU * hz).powi(2);
    let d = 2.0 * zeta * k.sqrt();
    let n = rows.len();
    if n < 2 {
        return;
    }
    let ls = loop_from(c);
    let worlds: Vec<Vec<Aff>> = rows.iter().map(|r| world(sk, r)).collect();
    let rest = sk.rest_world();
    // the frame after / before `f`: a loop goes from its last frame back to its start
    let next = |f: usize| if f + 1 < n { f + 1 } else { ls.unwrap_or(n - 1) };
    let prev = |f: usize| match ls { Some(l) if f == l => n - 1, _ => f.saturating_sub(1) };
    for &root in &roots {
        let parent = sk.bones[root].parent as usize;
        let rp = unscale(&rest[parent].r).0;
        // the body's forward and side axes in the parent's frame (from the rest pose's world +z / +x)
        let (fl, sl) = (crate::math::vm([0.0, 0.0, 1.0], &tr(&rp)), crate::math::vm([1.0, 0.0, 0.0], &tr(&rp)));
        let axes = |f: usize| { let pw = unscale(&worlds[f][parent].r).0; (crate::math::vm(fl, &pw), crate::math::vm(sl, &pw)) };
        let drive = |f: usize| -> [f64; 2] {
            let (p0, p1, p2) = (worlds[prev(f)][root].t, worlds[f][root].t, worlds[next(f)][root].t);
            let acc = [0, 1, 2].map(|i| (p2[i] - 2.0 * p1[i] + p0[i]) / (dt * dt));
            let (fw, sd) = axes(f);
            let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
            // forward acceleration: the tip back (+); sideways acceleration: the tip to the other side
            [dot(acc, fw) / lever, -dot(acc, sd) / lever]
        };
        let rate = SWING_RATE.to_radians();
        let step = |st: &mut ([f64; 2], [f64; 2]), f: usize| {
            let dr = drive(f);
            for i in 0..2 {
                // the drive no larger than what holds the spring at its widest swing
                let lim = k * back_max.max(side_max);
                let a = -k * st.0[i] - d * st.1[i] + dr[i].clamp(-lim, lim);
                st.1[i] = (st.1[i] + a * dt).clamp(-rate, rate);
                st.0[i] += st.1[i] * dt;
            }
            st.0[0] = st.0[0].clamp(-fwd_max, back_max);
            st.0[1] = st.0[1].clamp(-side_max, side_max);
            st.0
        };
        let mut th: Vec<[f64; 2]> = vec![[0.0; 2]; n];
        let mut st = ([0.0f64; 2], [0.0f64; 2]);
        for (f, t) in th.iter_mut().enumerate() {
            *t = step(&mut st, f);
        }
        if let Some(l) = ls {
            // the loop segment until it repeats: at most 12 more passes, stop when its start moves less than 0.1 deg
            let mut seg_st = st;
            for _ in 0..12 {
                let start_before = th[l];
                for (f, t) in th.iter_mut().enumerate().skip(l) {
                    *t = step(&mut seg_st, f);
                }
                if (0..2).all(|i| (th[l][i] - start_before[i]).abs() < 0.1f64.to_radians()) {
                    break;
                }
            }
            // the frames before the loop: from rest, eased into the loop's start
            if l > 0 {
                let mut st0 = ([0.0f64; 2], [0.0f64; 2]);
                let pre: Vec<[f64; 2]> = (0..l).map(|f| step(&mut st0, f)).collect();
                let target = th[l];
                let end = step(&mut st0.clone(), l);
                for (f, p) in pre.iter().enumerate() {
                    let w = (f + 1) as f64 / l as f64;
                    th[f] = [0, 1].map(|i| p[i] + w * (target[i] - end[i]));
                }
            }
        }
        for f in 0..n {
            // world turn Q about the root's origin: local' = local · Pw · Q · Pwᵀ (row vectors; Pw the parent's world rotation)
            let pw = unscale(&worlds[f][parent].r).0;
            let (fw, sd) = axes(f);
            let q: M3 = m3(&axis_angle(sd, th[f][0]), &axis_angle(fw, th[f][1]));
            let (l0r, _) = unscale(&local(&rows[f][root]).r);
            let lr = m3(&m3(&m3(&l0r, &pw), &q), &tr(&pw));
            let near = [rows[f][root][3], rows[f][root][4], rows[f][root][5]];
            let e = euler_xyz(&lr, Some(near));
            rows[f][root][3..6].copy_from_slice(&e);
        }
    }
}

/// Gives the skin's clips (`ann`, on Dark Riku's skeleton `sk`) their coat tails' motion; `own` is Dark Riku's own pack.
pub fn animate(ann: &mut Ann, own: &Ann, sk: &RecomSkeleton) -> Result<TailReport> {
    let tails = tail_bones(sk);
    let mut rep = TailReport::default();
    if tails.is_empty() {
        return Ok(rep);
    }
    for slot in 0..ann.slots.len() {
        let Some(c) = ann.slots[slot].clip.clone() else { continue };
        if c.bones as usize != sk.bones.len() {
            continue;
        }
        let mut rows = rows_of(&c);
        if !tails_still(&rows, sk, &tails) {
            rep.kept += 1;
            continue;
        }
        // 0.198.8 review: a copy only where the clip loops whole or not at all (slot 26, the jump role, loops from frame 16: its
        // copied jump popped 73 deg at the loop)
        let src = COPY.iter().find(|x| x.1.contains(&slot)).and_then(|x| own.slots.get(x.0)).and_then(|s| s.clip.as_ref()).filter(|s| s.bones == c.bones).filter(|_| matches!(loop_from(&c), None | Some(0)));
        match src {
            Some(s) => {
                copy_tails(&mut rows, &c, s, &tails);
                rep.copied += 1;
            }
            None => {
                follow_through(&mut rows, &c, sk, &tails);
                rep.simulated += 1;
            }
        }
        ann.slots[slot].clip = Some(crate::riku_skin::encode_rows(&c, &rows));
    }
    Ok(rep)
}
