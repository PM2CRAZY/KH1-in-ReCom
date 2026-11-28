//! KH2 Riku's hair on Re:CoM Riku's 18 hair bones, moved by KH2's own motion (0.195.0; fork owner, 2026-11-24: "A" — the 18
//! hair bones re-posed on KH2's and KH2's motion written into them — then "ss" to the plan).
//!
//! - KH2's hair (`P_EH000` group 3) hangs on 25 bones besides the head (98) and one neck bone (95), with unweighted bones between
//!   them and the head (Checked (files): the workspace's `knowledge/KH2/kh2-riku-hair-models.md`). Re:CoM Riku has 18 hair bones
//!   (`b_hair1-3`, `f_hair`, `l/r_hair1-7`; 14 on `head`, 4 on `f_hair`). [`SELECTED`] are the 18 KH2 bones kept: each of the 7
//!   others hands its vertices to the kept bone (or the head) whose motion moves them least away from where KH2 puts them, over
//!   KH2's idle, walk, run and jump ([`choose`], test `selection_is_the_greedy_one`).
//! - Frames: a KH2 bone's transform relative to KH2's head is carried into Re:CoM's head frame: rotation conjugated by the two
//!   heads' rest rotations, origin through the hair's rest fit (scale, width, shift of [`crate::kh2_hair`]). The rest pose of each
//!   Re:CoM hair bone is its KH2 bone's at KH2's bind pose, so the vertices placed by the rest fit are exact at rest; in motion a
//!   bone moves its vertices rigidly (the width stretch is not rotated with them: Assumption, small).
//! - Every clip of Riku's `PL0101.ANN` keeps every other bone; its 18 hair-bone channels are rewritten: KH2's idle / walk / run /
//!   jump / fall where [`CLIP_MAP`] says so (time stretched so a loop closes on the clip's own length), else KH2's idle at its
//!   first frame (the hair still, carried by the head).

use std::collections::HashMap;

use kh2_formats_lite::mdlx::Model;
use kh2_formats_lite::skeleton::{self, Mat};

use crate::ann::Ann;
use crate::kh2_motion::Kh2Motion;
use crate::math::{euler_xyz, m3, rot_xyz, tr, unscale, vm, Aff};
use crate::{bad, Error, Result};

/// KH2's head bone.
pub const HEAD: usize = 98;
/// KH2's neck bone that carries a few hair corners (they follow Re:CoM's `neck3`).
pub const NECK: usize = 95;
/// Re:CoM's bone for them.
pub const NECK_RECOM: &str = "neck3";
/// KH2's hair bones with vertices, besides the head and the neck. Checked (files).
pub const HAIR_BONES: [usize; 25] = [101, 104, 106, 108, 110, 112, 114, 116, 118, 119, 121, 122, 124, 125, 127, 129, 131, 133, 135, 137, 139, 142, 144, 146, 148];
/// KH2's `P_EH000.mset` slots (battle, with weapon: OpenKH `MotionSet.GetMotionSetIndex`): idle, walk, run, jump, fall.
/// Checked (files: the slots hold A000, A001, A002, A003, A004).
pub const KH2_IDLE: usize = 0;
/// See [`KH2_IDLE`].
pub const KH2_WALK: usize = 4;
/// See [`KH2_IDLE`].
pub const KH2_RUN: usize = 8;
/// See [`KH2_IDLE`].
pub const KH2_JUMP: usize = 14;
/// See [`KH2_IDLE`].
pub const KH2_FALL: usize = 18;
/// 0.197.5: KH2 slots of the motions item 4 of Stage 391 maps (the workspace's `knowledge/KH2/kh2-riku-motion-roles.md`): LAND
/// (A005), HANG (A100), DAMAGE_S_FRONT_HIGH (A200), A202 (pushed forward from behind), A301 (a right-to-left sweep). Checked (files:
/// the slots' names); the roles are OpenKH's names and measured profiles.
pub const KH2_LAND: usize = 22;
/// See [`KH2_LAND`].
pub const KH2_HANG: usize = 30;
/// See [`KH2_LAND`].
pub const KH2_DAMAGE_FRONT: usize = 42;
/// See [`KH2_LAND`].
pub const KH2_DAMAGE_BACK: usize = 50;
/// See [`KH2_LAND`].
pub const KH2_SWING: usize = 608;
/// Riku's `PL0101.ANN` slot → KH2 slot. Assumption (files: Re:CoM's clips are named by number only; picked by the feet: idle =
/// looped, feet still on the floor; walk / run = looped, feet apart 0.47-0.61 / 0.81-0.82 m, the lower foot rising 0.06 / 0.18-0.31 m;
/// jump = the lower foot leaving the floor; fall = looped clips in the air).
/// 0.197.5 (asset-analyst, `kh2-riku-motion-roles.md`; Riku's action table `0x1407C6370` rows 4-7 and KH1 Riku's bakes, Checked
/// (exe, files)): his jump / apex / fall / land are slots 4 / 5 / 6 / 7; 10 / 12 / 13 (0.195.0's jump and fall) are rows 9 / 0xB /
/// 0xC, the ladder set (hands above the head), now on the idle sway. Added the pairs the note rates high or medium: 16 ledge grab →
/// HANG, 37 flinch from the front → A200, 38 from behind → A202, 44 the first ground swing → A301 (low-medium). Every other clip
/// keeps KH2's idle sway (KH2 Riku has no guard, cast, card or dodge motion).
pub const CLIP_MAP: [(usize, usize); 19] = [
    (1, KH2_IDLE),
    (31, KH2_IDLE),
    (79, KH2_IDLE),
    (2, KH2_WALK),
    (28, KH2_WALK),
    (32, KH2_WALK),
    (80, KH2_WALK),
    (3, KH2_RUN),
    (21, KH2_RUN),
    (33, KH2_RUN),
    (4, KH2_JUMP),
    (26, KH2_JUMP),
    (5, KH2_FALL),
    (6, KH2_FALL),
    (7, KH2_LAND),
    (16, KH2_HANG),
    (37, KH2_DAMAGE_FRONT),
    (38, KH2_DAMAGE_BACK),
    (44, KH2_SWING),
];
/// Extra forward turn (degrees; negative = the hanging strands drop) per KH2 motion, the owner's ask on 0.195.0 in game: "a animação
/// quando corre ficou quase perfeito da parte de trás do cabelo mas ele sobe Muito". Not KH2's (in KH2's own poses the back of the
/// hair sits higher still: KH2's head is bent 28-46 deg forward of its rest pose, Re:CoM's 1-31, measured on the emulator and the clips,
/// test `head_tilts`).
/// Assumption: the amount, chosen from the renders.
pub const EXTRA_TILT: [(usize, f64); 2] = [(KH2_RUN, -36.0), (KH2_WALK, -24.0)];
#[cfg(test)]
thread_local! {
    /// Test only (0.198.6, `back_lock_angle`): (KH2 motion, degrees) replacing [`EXTRA_TILT`]'s amount for that motion.
    pub static TILT_OVERRIDE: std::cell::Cell<Option<(usize, f64)>> = const { std::cell::Cell::new(None) };
    /// Test only: (KH2 motion, fraction) replacing [`MOVE_KEEP`]'s.
    pub static KEEP_OVERRIDE: std::cell::Cell<Option<(usize, f64)>> = const { std::cell::Cell::new(None) };
}

/// 0.198.6: the fraction of KH2's walk / run the back-rooted locks keep in those clips (the rest is KH2's idle at the same time);
/// 1.0 = KH2's motion. Assumption: the amounts, from the renders and `back_lock_angle`.
pub const MOVE_KEEP: [(usize, f64); 2] = [(KH2_RUN, 0.5), (KH2_WALK, 0.5)];

/// Hair bones rooted this far or more in front of the head bone (metres, the head's forward axis at rest) are left out of the extra
/// turn.
pub const TILT_BEHIND: f64 = -0.02;
/// 0.196.16, experiment (fork owner, 2026-11-24, on 0.196.15 in game, of the locks behind and below the ear: "quero fazer um
/// experimento, deixe a animação dessas mechas mais retas"; "pq parece que a animação que está revelando o gap"): KH2 bones whose
/// motion is scaled toward their rest pose on the head (1.0 = KH2's motion, 0.0 = no sway: the lock moves with the head only).
/// KH2's +x side (the side of the owner's screenshot, the renders' `body_side2`): 131 (below and behind the ear), 133 (above and
/// behind it), 106 (the side of the back). Assumption: these are the locks the owner boxed (bone positions, not a per-lock render).
pub const DAMP: [(usize, f64); 3] = [(131, 0.0), (133, 0.0), (106, 0.0)];

/// A rotation `k` of the way from `a` to `b` (axis-angle of `a`'s inverse times `b`, scaled).
pub fn rot_toward(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3], k: f64) -> [[f64; 3]; 3] {
    let d = m3(&tr(a), b);
    let c = ((d[0][0] + d[1][1] + d[2][2] - 1.0) / 2.0).clamp(-1.0, 1.0);
    let ang = c.acos();
    let v = [d[2][1] - d[1][2], d[0][2] - d[2][0], d[1][0] - d[0][1]];
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if n < 1e-12 || ang.abs() < 1e-12 {
        return if k >= 0.5 && ang.abs() >= 1e-12 { *b } else { *a };
    }
    let u = [v[0] / n, v[1] / n, v[2] / n];
    let (sn, cs) = (k * ang).sin_cos();
    let mut r = [[0.0; 3]; 3];
    let kx = [[0.0, -u[2], u[1]], [u[2], 0.0, -u[0]], [-u[1], u[0], 0.0]];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = if i == j { cs } else { 0.0 } + sn * kx[i][j] + (1.0 - cs) * u[i] * u[j];
        }
    }
    m3(a, &r)
}

/// Applies [`DAMP`] to one frame's head-relative transforms (`rest`: the same bones at KH2's bind pose).
pub fn damp_hair(rig: &Rig, hr: &mut [(usize, Aff)], rest: &[(usize, Aff)]) {
    for (rb, a) in hr.iter_mut() {
        let Some(k) = rig.kh2_of(*rb).and_then(|kb| DAMP.iter().find(|x| x.0 == kb)).map(|x| x.1) else { continue };
        let Some(r0) = rest.iter().find(|x| x.0 == *rb).map(|x| x.1) else { continue };
        a.r = rot_toward(&r0.r, &a.r, k);
        a.t = core::array::from_fn(|i| r0.t[i] + k * (a.t[i] - r0.t[i]));
    }
}

/// KH2 bones the extra turn leaves alone (the skull cap: turning it would lift it off the head).
pub const NO_TILT: [usize; 2] = [101, 104];
/// The KH2 motions the selection is measured on.
pub const CHOOSE_ON: [usize; 4] = [KH2_IDLE, KH2_WALK, KH2_RUN, KH2_JUMP];
/// Re:CoM's hair bones.
pub const RECOM_HAIR: [&str; 18] = ["b_hair1", "b_hair2", "b_hair3", "f_hair", "l_hair1", "l_hair2", "r_hair1", "r_hair2", "l_hair3", "l_hair4", "l_hair5", "l_hair6", "l_hair7", "r_hair3", "r_hair4", "r_hair5", "r_hair6", "r_hair7"];
/// Bones always kept (0.195.1: 142, the strand by the ear, moves 9-13 KH2 units off the head where 139, which carried it, moves 3-5;
/// fork owner on 0.195.0: "uma mecha ficou cobrindo a orelha", "essa mecha desloque de um jeito não natural").
pub const FORCED: [usize; 1] = [142];
/// The 18 KH2 bones kept ([`choose`]'s result on `P_EH000` with [`FORCED`], pinned by test `selection_is_the_greedy_one`).
pub const SELECTED: [usize; 18] = [101, 104, 106, 108, 110, 112, 114, 116, 119, 125, 127, 129, 131, 135, 137, 139, 142, 146];
/// 0.196.5 (fork owner, 2026-11-24: "usa a mesma animação do cabelo do KH2 e encaixa certo na cabeça do riku do kh recom"): the 7 KH2 hair
/// bones [`SELECTED`] leaves out get a Re:CoM bone of their own, appended after Re:CoM's last bone (parent: the head; rest: KH2's bind pose
/// carried), so every KH2 hair bone moves its own vertices with its own motion (0.195.0-0.196.3 handed their vertices to a kept bone: the
/// worst vertex was 3.8 cm off KH2's idle, 3.5 walk, 4.3 run, 7.9 jump, Checked (test `kh2_hair_moves_as_kh2`)). Checked (decomp:
/// `knowledge/ReCoM/recom-skeleton-limits.md`): a model may take more bones when every clip played on it has the same count.
pub const EXTRA: [usize; 7] = [118, 121, 122, 124, 133, 144, 148];
/// 0.198.6 (fork owner: "sim" to separating the long back locks from the cap, "continue com a correção da nuca"; on 0.198.5, a side
/// view of the run: "Cabelo levanta DEMAIS!"). Checked (vertex counts per bone, run frame 6, Stage 402): the long back hair rides on
/// KH2 bones 101 and 104 with the skull cap, which [`NO_TILT`] leaves alone. Their vertices behind and below the skull base go to
/// these bones (appended after [`EXTRA`], parent `head`, the same rest and the same KH2 motion as their source), which take the
/// fraction of [`EXTRA_TILT`] given here: half in the band just off the skull, all of it farther ([`SPLIT_R`], [`SPLIT_Z`]).
/// 0.198.6 review (model-reviewer: a straight seam across the run's back where 101's half band, 68 corners, met the full one): three
/// steps for 101 (1/3, 2/3, all); 104's vertices are all far from the skull (28.7-41.9 cm), so it takes all of it.
pub const SPLIT: [(usize, &str, f64); 4] = [(101, "k2_hair_101a", 1.0 / 3.0), (101, "k2_hair_101b", 2.0 / 3.0), (101, "k2_hair_101c", 1.0), (104, "k2_hair_104c", 1.0)];
/// See [`SPLIT`]: a vertex (fitted, world rest) that follows the Re:CoM bone of KH2 bone 101 / 104, behind [`SPLIT_Z`] (relative
/// to the head bone), takes the 1/3 band under `SPLIT_R[0]` from the head's centre (`ddd_hair::HEAD_CENTRE`), the 2/3 band under
/// `SPLIT_R[1]` and the full one beyond (metres; 101's vertices are 15.7-22.9 cm out, quartiles 18.8 / 19.9 / 20.1). Assumption: the radii.
pub const SPLIT_R: [f64; 2] = [0.175, 0.195];
/// See [`SPLIT_R`].
pub const SPLIT_Z: f64 = -0.03;

/// The Re:CoM bone name of an [`EXTRA`] KH2 bone (at most 15 characters).
pub fn extra_name(kh2: usize) -> String {
    format!("k2_hair_{kh2}")
}

/// `sk` with the [`EXTRA`] bones appended (parent `head`, rest local = the KH2 bone's bind pose relative to KH2's head, carried).
/// Unchanged when they are already there.
pub fn extend_skeleton(sk: &crate::mdl::RecomSkeleton, head: usize, model: &Model, carry: &Carry) -> crate::mdl::RecomSkeleton {
    if sk.find(&extra_name(EXTRA[0])).is_some() {
        return sk.clone();
    }
    let bind = rel(&skeleton::bind_pose(&model.bones));
    let mut out = sk.clone();
    for &k in &EXTRA {
        out.bones.push(crate::mdl::RBone { name: extra_name(k), parent: head as i16, local: carry.aff(&bind[k]) });
    }
    for &(k, n, _) in &SPLIT {
        out.bones.push(crate::mdl::RBone { name: n.to_string(), parent: head as i16, local: carry.aff(&bind[k]) });
    }
    out
}

/// `mdl` with `add` appended to the bone table of every part (names, parents, rest matrices, bounds of the new bones as a bone without
/// vertices has them); the parts' vertices are untouched.
pub fn append_bones(mdl: &[u8], add: &[crate::mdl::RBone]) -> Result<Vec<u8>> {
    use crate::mdl::{join_parts, split_parts, MdlBone, MdlPart};
    let parts = split_parts(mdl)?;
    let mut out = Vec::with_capacity(parts.len());
    for pb in &parts {
        let mut p = MdlPart::parse_bytes(pb)?;
        let n0 = p.bones.len();
        for (j, b) in add.iter().enumerate() {
            if b.name.len() > 15 {
                return bad(format!("mdl: bone name {} is longer than 15", b.name));
            }
            let mut name = [0u8; 16];
            name[..b.name.len()].copy_from_slice(b.name.as_bytes());
            p.bones.push(MdlBone { name, parent: b.parent, index: (n0 + j) as i16 });
            let a = b.local;
            let m = [a.r[0][0], a.r[0][1], a.r[0][2], 0.0, a.r[1][0], a.r[1][1], a.r[1][2], 0.0, a.r[2][0], a.r[2][1], a.r[2][2], 0.0, a.t[0], a.t[1], a.t[2], 1.0];
            p.rest.push(m.map(|v| (v as f32).to_bits()));
        }
        let nb = p.bones.len();
        let lists: Vec<&[crate::mdl::MeshPacket]> = std::iter::once(&p.list_a[..]).chain(p.list_b.iter().map(|l| &l[..])).collect();
        let fresh = crate::keyblade::part_bounds(nb, &lists);
        p.bounds.extend_from_slice(&fresh[n0..]);
        out.push(p.write_bytes()?);
    }
    Ok(join_parts(&out))
}

/// Gives every clip of `ann` `add` more bones (their channels start as zero constants; [`bake`] writes the hair's).
pub fn extend_ann(ann: &mut Ann, add: usize) {
    for s in ann.slots.iter_mut() {
        let Some(c) = s.clip.as_mut() else { continue };
        let chans = c.chans();
        c.bones += add as i16;
        c.channels.extend((0..add * chans).map(|_| crate::ann::Channel::Const { bits: 0, size_field: 0 }));
    }
}

/// A column matrix (`kh2_formats_lite::skeleton::Mat`) as a row-vector affine, rotation unscaled.
pub fn aff(m: &Mat) -> Aff {
    let r = [[m[0][0], m[0][1], m[0][2]], [m[1][0], m[1][1], m[1][2]], [m[2][0], m[2][1], m[2][2]]];
    Aff { r: unscale(&r).0, t: [m[3][0], m[3][1], m[3][2]] }
}

/// Inverse of a rigid affine.
pub fn rigid_inv(a: &Aff) -> Aff {
    let r = tr(&a.r);
    let t = vm(a.t, &r);
    Aff { r, t: [-t[0], -t[1], -t[2]] }
}

/// Every bone's transform relative to KH2's head (rigid, KH2 units).
pub fn rel(world: &[Mat]) -> Vec<Aff> {
    let k = rigid_inv(&aff(&world[HEAD]));
    world.iter().map(|w| aff(w).mul(&k)).collect()
}

/// The hair's vertices per KH2 bone, in that bone's bind frame (KH2 units).
pub fn vertices_by_bone(model: &Model, group: usize) -> HashMap<usize, Vec<[f64; 3]>> {
    let pose = skeleton::bind_pose(&model.bones);
    let mut out: HashMap<usize, Vec<[f64; 3]>> = HashMap::new();
    if let Some(g) = model.groups.get(group) {
        for t in &g.triangles {
            for v in t {
                let b = v.bone as usize;
                let w = skeleton::point(&pose[b], v.position.map(|x| x as f64));
                let l = rigid_inv(&aff(&pose[b])).point(w);
                out.entry(b).or_default().push(l);
            }
        }
    }
    out
}

/// Sampled head-relative transforms of the motions in `on` (every other frame).
pub fn samples(model: &Model, motions: &[&Kh2Motion]) -> Vec<Vec<Aff>> {
    let mut out = vec![];
    for m in motions {
        let mut f = 0.0f32;
        while f <= m.frame_end {
            out.push(rel(&m.world(&model.bones, f)));
            f += 2.0;
        }
    }
    out
}

/// The kept bones: starting from all of [`HAIR_BONES`], drop one at a time the bone whose vertices (and those it took over) move
/// least away from KH2's positions when they follow the best remaining bone or the head instead, until `keep` remain.
pub fn choose(model: &Model, group: usize, motions: &[&Kh2Motion], keep: usize, forced: &[usize]) -> Vec<usize> {
    let bind = rel(&skeleton::bind_pose(&model.bones));
    let verts = vertices_by_bone(model, group);
    let frames = samples(model, motions);
    // cost[d][h]: vertices of d following h (h = None: the head)
    let hosts: Vec<Option<usize>> = std::iter::once(None).chain(HAIR_BONES.iter().map(|&b| Some(b))).collect();
    let mut cost: HashMap<(usize, Option<usize>), f64> = HashMap::new();
    for &d in &HAIR_BONES {
        let vs = verts.get(&d).cloned().unwrap_or_default();
        for &h in &hosts {
            let mut c = 0.0;
            if h != Some(d) {
                // rest offset of d in h's frame
                let hb = h.map_or(Aff::IDENTITY, |h| bind[h]);
                let off = bind[d].mul(&rigid_inv(&hb));
                for fr in &frames {
                    let ht = h.map_or(Aff::IDENTITY, |h| fr[h]);
                    let approx = off.mul(&ht);
                    for v in &vs {
                        let (a, b) = (fr[d].point(*v), approx.point(*v));
                        c += ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
                    }
                }
            }
            cost.insert((d, h), c);
        }
    }
    let mut kept: Vec<usize> = HAIR_BONES.to_vec();
    let total = |kept: &[usize]| -> f64 {
        HAIR_BONES
            .iter()
            .map(|&d| if kept.contains(&d) { 0.0 } else { std::iter::once(None).chain(kept.iter().map(|&b| Some(b))).map(|h| cost[&(d, h)]).fold(f64::MAX, f64::min) })
            .sum()
    };
    while kept.len() > keep {
        let (i, _) = (0..kept.len())
            .filter(|&i| !forced.contains(&kept[i]))
            .map(|i| {
                let mut k = kept.clone();
                k.remove(i);
                (i, total(&k))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap();
        kept.remove(i);
    }
    kept.sort();
    kept
}

/// For a KH2 bone, the bone its vertices follow: itself when kept, else the kept bone (or the head, `None`) of least cost.
pub fn hosts(model: &Model, group: usize, motions: &[&Kh2Motion], kept: &[usize]) -> HashMap<usize, Option<usize>> {
    let bind = rel(&skeleton::bind_pose(&model.bones));
    let verts = vertices_by_bone(model, group);
    let frames = samples(model, motions);
    let mut out = HashMap::new();
    for &d in &HAIR_BONES {
        if kept.contains(&d) {
            out.insert(d, Some(d));
            continue;
        }
        let vs = verts.get(&d).cloned().unwrap_or_default();
        let best = std::iter::once(None)
            .chain(kept.iter().map(|&b| Some(b)))
            .map(|h| {
                let hb = h.map_or(Aff::IDENTITY, |h| bind[h]);
                let off = bind[d].mul(&rigid_inv(&hb));
                let mut c = 0.0;
                for fr in &frames {
                    let approx = off.mul(&h.map_or(Aff::IDENTITY, |h| fr[h]));
                    for v in &vs {
                        let (a, b) = (fr[d].point(*v), approx.point(*v));
                        c += ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
                    }
                }
                (h, c)
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|x| x.0)
            .unwrap_or(None);
        out.insert(d, best);
    }
    out
}

/// The carry from KH2's head frame to Re:CoM's: `A` (rotation, row vectors: `x_recom = x_kh2 · A`), the per-axis world scale
/// `c` (applied in world axes between the two rotations) and the world shift.
#[derive(Clone, Copy, Debug)]
pub struct Carry {
    /// KH2 head's rest rotation (row vectors).
    pub k: [[f64; 3]; 3],
    /// Re:CoM head's rest rotation (row vectors).
    pub h: [[f64; 3]; 3],
    /// World scale per axis (KH2 units → metres, with the width).
    pub c: [f64; 3],
    /// World shift (metres).
    pub shift: [f64; 3],
}

impl Carry {
    /// A point in KH2's head frame → Re:CoM's head frame.
    pub fn point(&self, q: [f64; 3]) -> [f64; 3] {
        let w = vm(q, &self.k);
        let w = [w[0] * self.c[0] + self.shift[0], w[1] * self.c[1] + self.shift[1], w[2] * self.c[2] + self.shift[2]];
        vm(w, &tr(&self.h))
    }
    /// A rotation in KH2's head frame → Re:CoM's head frame.
    pub fn rot(&self, r: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
        let a = m3(&self.k, &tr(&self.h));
        m3(&m3(&tr(&a), r), &a)
    }
    /// A KH2 head-relative transform → Re:CoM head-relative.
    pub fn aff(&self, x: &Aff) -> Aff {
        Aff { r: self.rot(&x.r), t: self.point(x.t) }
    }
}

/// The rig: which KH2 bone each Re:CoM hair bone carries.
#[derive(Clone, Debug)]
pub struct Rig {
    /// `(Re:CoM bone index, KH2 bone)`.
    pub pairs: Vec<(usize, usize)>,
    /// KH2 bone → the KH2 bone (kept) or head its vertices follow.
    pub host: HashMap<usize, Option<usize>>,
    /// The carry.
    pub carry: Carry,
    /// [`SPLIT`]'s Re:CoM bones and their fraction of the extra turn (0.198.6).
    pub split: Vec<(usize, f64)>,
}

impl Rig {
    /// Pairs the kept KH2 bones with Re:CoM's hair bones (each to the free Re:CoM bone nearest at rest, nearest pairs first).
    pub fn new(names: &[String], world: &[Aff], model: &Model, carry: Carry, host: HashMap<usize, Option<usize>>) -> Result<Rig> {
        let head = names.iter().position(|n| n == "head").ok_or_else(|| Error::Format("hair: no head bone".into()))?;
        let bind = rel(&skeleton::bind_pose(&model.bones));
        let hw = world[head];
        let recom: Vec<usize> = RECOM_HAIR.iter().map(|n| names.iter().position(|x| x == n).ok_or_else(|| Error::Format(format!("hair: no {n}")))).collect::<Result<_>>()?;
        // world positions at rest: KH2 bone origins carried, Re:CoM hair bones from the MDL
        let kpos: Vec<(usize, [f64; 3])> = SELECTED.iter().map(|&b| (b, Aff { r: crate::math::I3, t: carry.point(bind[b].t) }.mul(&hw).t)).collect();
        let mut cand: Vec<(f64, usize, usize)> = vec![];
        for &(b, p) in &kpos {
            for &r in &recom {
                let q = world[r].t;
                cand.push(((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2), r, b));
            }
        }
        cand.sort_by(|a, b| a.0.total_cmp(&b.0));
        let (mut used_r, mut used_k, mut pairs) = (vec![], vec![], vec![]);
        for (_, r, b) in cand {
            if used_r.contains(&r) || used_k.contains(&b) {
                continue;
            }
            used_r.push(r);
            used_k.push(b);
            pairs.push((r, b));
        }
        if pairs.len() != 18 {
            return bad(format!("hair: {} pairs", pairs.len()));
        }
        for &k in &EXTRA {
            let r = names.iter().position(|x| *x == extra_name(k)).ok_or_else(|| Error::Format(format!("hair: no {}", extra_name(k))))?;
            pairs.push((r, k));
        }
        let mut split = vec![];
        for &(k, n, f) in &SPLIT {
            let r = names.iter().position(|x| x == n).ok_or_else(|| Error::Format(format!("hair: no {n}")))?;
            pairs.push((r, k));
            split.push((r, f));
        }
        pairs.sort();
        Ok(Rig { pairs, host, carry, split })
    }

    /// The KH2 bone a Re:CoM bone carries.
    pub fn kh2_of(&self, r: usize) -> Option<usize> {
        self.pairs.iter().find(|p| p.0 == r).map(|p| p.1)
    }

    /// The Re:CoM bone a KH2 bone's vertices follow (`None`: the head).
    pub fn recom_of(&self, kh2: usize) -> Option<usize> {
        let h = self.host.get(&kh2).copied().flatten()?;
        self.pairs.iter().find(|p| p.1 == h).map(|p| p.0)
    }

    /// Head-relative (Re:CoM frame) transforms of the 18 Re:CoM hair bones for KH2 head-relative bone transforms `r`.
    pub fn head_rel(&self, r: &[Aff]) -> Vec<(usize, Aff)> {
        self.pairs.iter().map(|&(rb, kb)| (rb, self.carry.aff(&r[kb]))).collect()
    }

    /// Parent-relative locals of the 18 bones (their parent is `head` or another of them), given head-relative transforms.
    pub fn locals(&self, parents: &[i16], head: usize, hr: &[(usize, Aff)]) -> Vec<(usize, Aff)> {
        hr.iter()
            .map(|&(rb, a)| {
                let p = parents[rb] as usize;
                let pa = if p == head { Aff::IDENTITY } else { hr.iter().find(|x| x.0 == p).map_or(Aff::IDENTITY, |x| x.1) };
                (rb, a.mul(&rigid_inv(&pa)))
            })
            .collect()
    }
}

/// Writes the rest locals of the 18 bones into every model part's skeleton of a Re:CoM `.MDL` (Riku's: 4 parts).
pub fn patch_rest(mdl: &mut [u8], locals: &[(usize, Aff)]) -> Result<()> {
    let rd = |b: &[u8], o: usize| -> Result<u32> { b.get(o..o + 4).map(|x| u32::from_le_bytes(x.try_into().unwrap())).ok_or_else(|| Error::Format("mdl: truncated".into())) };
    let first = rd(mdl, 0)? as usize;
    let parts: Vec<usize> = (0..first / 4).map(|i| rd(mdl, 4 * i).map(|x| x as usize)).collect::<Result<_>>()?;
    for p in parts.into_iter().filter(|&p| p != 0) {
        let n = u16::from_le_bytes([mdl[p], mdl[p + 1]]) as usize;
        let mt = p + rd(mdl, p + 8)? as usize;
        for &(b, a) in locals {
            if b >= n {
                return bad(format!("mdl: bone {b} of {n}"));
            }
            let m = [a.r[0][0], a.r[0][1], a.r[0][2], 0.0, a.r[1][0], a.r[1][1], a.r[1][2], 0.0, a.r[2][0], a.r[2][1], a.r[2][2], 0.0, a.t[0], a.t[1], a.t[2], 1.0];
            for (k, v) in m.iter().enumerate() {
                mdl[mt + 64 * b + 4 * k..mt + 64 * b + 4 * k + 4].copy_from_slice(&(*v as f32).to_le_bytes());
            }
        }
    }
    Ok(())
}

/// 0.197.6 (item 5 of Stage 391: the hair at KH2's own speed): Re:CoM's looped idle clips with no events (no `PL0101.EET` record,
/// no `PL0101.SES` cue under their names: `p002_001`, `p002_031`, `p002_079`, Checked (files)). Their KH2 idle loop was stretched to
/// close on the clip (x1.20, x1.50, x3.00, Stage 380); each is repeated whole until it and KH2's loop close together ([`unroll_times`]),
/// so the hair plays at KH2's speed. Walk and run carry footstep cues in the `.SES` and keep their length.
pub const UNROLL: [usize; 3] = [1, 31, 79];
/// The most times a clip is repeated.
pub const UNROLL_MAX: usize = 6;

/// The fewest repeats (2..=[`UNROLL_MAX`]) of a clip of `frames` frames after which a whole number of KH2 loops of `l` frames fits
/// within 1 %, if any (none when one clip already closes on whole loops).
pub fn unroll_times(frames: usize, l: f32) -> Option<usize> {
    let one = frames as f32 / l;
    if one >= 1.0 && (one - one.round()).abs() < 0.01 {
        return None;
    }
    (2..=UNROLL_MAX).find(|&k| {
        let n = k as f32 * one;
        n >= 1.0 && (n - n.round()).abs() < 0.01
    })
}

/// The clip repeated `times` times: every channel sampled at its whole frames and written again over `times × frames` (the loop
/// start and everything else as they were).
pub fn unroll(c: &mut crate::ann::Clip, times: usize) {
    let (frames, chans) = (c.frames.max(1) as usize, c.chans());
    let total = frames * times;
    let mut out = Vec::with_capacity(c.channels.len());
    for b in 0..c.bones as usize {
        for ci in 0..chans {
            let v: Vec<f64> = (0..total).map(|f| c.value(b, ci, (f % frames) as f64)).collect();
            out.push(crate::convert::encode_channel(&v, [1e-7, 1e-5, 1e-6][ci / 3]));
        }
    }
    c.channels = out;
    c.frames = total as i16;
}

/// KH2 time (motion frames) for frame `f` of a Re:CoM clip of `frames` frames (30 fps both): a looped clip runs a whole number of
/// the KH2 motion's loops, stretched to close on the clip's length; otherwise KH2's own time, held at its end.
pub fn kh2_time(m: &Kh2Motion, looped: bool, frames: usize, f: usize) -> f32 {
    let (start, end, ret) = (m.frame_start, m.frame_end, m.frame_return.min(m.frame_end));
    if looped {
        let l = (end - ret).max(1.0);
        let n = ((frames as f32 / l).round()).max(1.0);
        let x = (f as f32 / frames as f32) * n * l;
        ret + x.rem_euclid(l)
    } else {
        (start + f as f32).min(end)
    }
}

/// The head's forward tilt (degrees) of a posed head rotation against the rest one (model y-z plane, + toward +z).
pub fn tilt(rest: &[[f64; 3]; 3], posed: &[[f64; 3]; 3]) -> f64 {
    let up = vm(vm([0.0, 1.0, 0.0], &tr(rest)), posed);
    up[2].atan2(up[1]).to_degrees()
}

/// Turns every hair bone but [`NO_TILT`]'s and `front`'s by `deg` forward (about the model's x axis seen from Re:CoM's head at rest,
/// `head_rest`; a positive turn lifts strands that hang backwards): [`EXTRA_TILT`] (0.195.1); [`SPLIT`]'s bones by their fraction.
/// 0.198.6 (model-reviewer: "make this a number"; test `back_lock_angle`): each turns about its lock's root (`roots`: a point in the
/// bone's own frame, the lock's vertex nearest the head bone at rest), so the root stays on the head and the whole lock drops by the
/// angle; 0.195.1-0.198.6 turned it about the bone's own origin, mid-lock, and -60 deg dropped the back hair 10-14 deg.
pub fn tilt_hair(rig: &Rig, hr: &mut [(usize, Aff)], head_rest: &[[f64; 3]; 3], deg: f64, front: &[usize], roots: &HashMap<usize, [f64; 3]>) {
    if deg == 0.0 {
        return;
    }
    // 0.198.6: the head-relative frame's turn about the model's x axis is `head_restᵀ · Rx · head_rest` (a negative `deg` drops the
    // strands). Checked (renders `t60orig` / `t60swap` / `fx60`, run 6 from the side, bones coloured): 0.195.1-0.198.6 used
    // `head_rest · Rx · head_restᵀ`, a turn about another axis (the locks twisted about their length; -60 deg dropped them 10-14 deg,
    // -90 knotted them); the swapped product keeps the turn in the side plane, and -Rx lifts them.
    let rl_of = |f: f64| { let rw = rot_xyz((deg * f).to_radians(), 0.0, 0.0); m3(&m3(head_rest, &rw), &tr(head_rest)) };
    for (rb, a) in hr.iter_mut() {
        let f = if let Some(&(_, f)) = rig.split.iter().find(|x| x.0 == *rb) {
            f
        } else if rig.kh2_of(*rb).is_some_and(|k| NO_TILT.contains(&k)) || front.contains(rb) {
            continue;
        } else {
            1.0
        };
        let rl = rl_of(f);
        let pivot = roots.get(rb).map_or(a.t, |l| a.point(*l));
        let d = vm([a.t[0] - pivot[0], a.t[1] - pivot[1], a.t[2] - pivot[2]], &rl);
        a.t = [pivot[0] + d[0], pivot[1] + d[1], pivot[2] + d[2]];
        a.r = m3(&a.r, &rl);
    }
}

/// Rewrites the 18 hair bones' channels of every clip of Riku's pack. `sk`: Riku's original skeleton (for the head's tilt).
/// `by_slot`: the clips are `PL0101.ANN`'s, so [`CLIP_MAP`] picks KH2's motion by slot; false for other packs (0.197.4: Riku's event
/// packs, whose slots mean other clips): every clip takes KH2's idle sway.
pub fn bake(ann: &mut Ann, rig: &Rig, sk: &crate::mdl::RecomSkeleton, head: usize, model: &Model, motions: &HashMap<usize, Kh2Motion>, by_slot: bool) -> Result<()> {
    let slots = if by_slot { RIKU_SLOTS } else { NO_SLOTS };
    bake_slots(ann, rig, sk, head, model, motions, slots)
}

/// Which KH2 motion each slot of a pack takes ([`bake_slots`]): (slot, KH2 motion), and the idle slots whose loop is unrolled.
#[derive(Clone, Copy, Debug)]
pub struct Slots<'a> {
    /// (slot, KH2 motion).
    pub map: &'a [(usize, usize)],
    /// The idle slots whose loop is unrolled ([`UNROLL`]'s role).
    pub unroll: &'a [usize],
}
/// `PL0101.ANN`'s slots ([`CLIP_MAP`], [`UNROLL`]).
pub const RIKU_SLOTS: Slots<'static> = Slots { map: &CLIP_MAP, unroll: &UNROLL };
/// A pack whose slots mean other clips: every clip takes KH2's idle sway.
pub const NO_SLOTS: Slots<'static> = Slots { map: &[], unroll: &[] };

/// [`bake`] with the slots of any pack (0.198.32: Dark Riku's own `PL0201` / `PL0211.ANN`, whose slots are not Riku's:
/// [`crate::riku_skin::DARK_SLOTS`]).
pub fn bake_slots(ann: &mut Ann, rig: &Rig, sk: &crate::mdl::RecomSkeleton, head: usize, model: &Model, motions: &HashMap<usize, Kh2Motion>, slots: Slots) -> Result<()> {
    let parents: Vec<i16> = sk.bones.iter().map(|b| b.parent).collect();
    let parents = &parents[..];
    let head_rest = unscale(&sk.rest_world()[head].r).0;
    let idle = motions.get(&KH2_IDLE).ok_or_else(|| Error::Format("KH2: no idle motion".into()))?;
    let still_hr = rig.head_rel(&rel(&idle.world(&model.bones, idle.frame_start)));
    // 0.195.1: every clip's Euler angles start on the branch of the bone's rest angles, so that a blend between two clips does not
    // cross a whole turn (0.195.0 wrote the same rotation up to 359 deg apart between clips: the back of the hair spun on a blend;
    // fork owner: "esse lance de uma mexida brusca é na parte de trás do cabelo inteiro")
    // 0.195.4: only bones rooted behind the head's middle take the extra turn (turning the front locks put them into the face;
    // fork owner on 0.195.3, walk: "traseira do cabelo ainda ta levantando muito quando anda")
    let rest_hr = rig.head_rel(&rel(&skeleton::bind_pose(&model.bones)));
    let front: Vec<usize> = rest_hr.iter().filter(|(_, a)| vm(a.t, &head_rest)[2] > TILT_BEHIND).map(|x| x.0).collect();
    // 0.198.6: each lock's root, in its Re:CoM bone's frame: the KH2 bone's vertex nearest the head bone at rest
    let roots: HashMap<usize, [f64; 3]> = {
        let verts = vertices_by_bone(model, crate::kh2_hair::HAIR_GROUP);
        let bind = rel(&skeleton::bind_pose(&model.bones));
        rest_hr
            .iter()
            .filter_map(|(rb, a)| {
                let kb = rig.kh2_of(*rb)?;
                let inv = rigid_inv(a);
                verts.get(&kb)?.iter().map(|v| inv.point(rig.carry.point(bind[kb].point(*v)))).map(|l| (a.point(l), l)).min_by(|x, y| crate::math::dist(x.0, [0.0; 3]).total_cmp(&crate::math::dist(y.0, [0.0; 3]))).map(|(_, l)| (*rb, l))
            })
            .collect()
    };
    let rest_e: HashMap<usize, [f64; 3]> = rig
        .locals(parents, head, &rig.head_rel(&rel(&skeleton::bind_pose(&model.bones))))
        .into_iter()
        .map(|(rb, a)| (rb, euler_xyz(&a.r, None)))
        .collect();
    for (slot, s) in ann.slots.iter_mut().enumerate() {
        let Some(c) = s.clip.as_mut() else { continue };
        let chans = c.chans();
        let kh2 = slots.map.iter().find(|x| x.0 == slot).map(|x| x.1);
        // 0.197.6: KH2's idle at its own speed on the event-free idle clips
        if slots.unroll.contains(&slot) && kh2 == Some(KH2_IDLE) && c.looped() && c.loop_start == 1 {
            if let Some(k) = unroll_times(c.frames.max(1) as usize, (idle.frame_end - idle.frame_return.min(idle.frame_end)).max(1.0)) {
                unroll(c, k);
            }
        }
        let frames = c.frames.max(1) as usize;
        // 0.196.6 (fork owner: "a animação DESSE CABELO precisa ser a MESMA do KH2"): a clip with no KH2 counterpart (attacks, guard, magic,
        // cards: 0.195.0-0.196.5 held the hair still at KH2's first idle frame) plays KH2's idle sway; a one-shot clip at KH2's own speed
        let src = kh2.and_then(|k| motions.get(&k)).or(Some(idle));
        let looped = c.looped() && c.loop_start == 1;
        let corr = EXTRA_TILT.iter().find(|x| Some(x.0) == kh2).map_or(0.0, |x| x.1);
        #[cfg(test)]
        let corr = TILT_OVERRIDE.with(|o| o.get()).filter(|o| Some(o.0) == kh2).map_or(corr, |o| o.1);
        #[cfg(test)]
        let corr = match (kh2, std::env::var("RUN_TILT").ok().and_then(|x| x.parse::<f64>().ok())) { (Some(KH2_RUN), Some(t)) => t, _ => corr };
        let rows: Vec<Vec<(usize, Aff)>> = (0..frames)
            .map(|f| {
                let mut hr = match src {
                    Some(m) => rig.head_rel(&rel(&m.world(&model.bones, kh2_time(m, looped, frames, f)))),
                    None => still_hr.clone(),
                };
                // 0.198.6 (fork owner: "Cabelo levanta DEMAIS!"; test `back_lock_angle`: against the back, KH2's run locks are already
                // matched, the lift comes from KH2's body leaning where Riku's is upright): in the walk and run, the back-rooted locks
                // take [`MOVE_KEEP`] of KH2's walk / run and the rest of KH2's idle at the same time (where the hair hangs)
                let keep = MOVE_KEEP.iter().find(|x| Some(x.0) == kh2).map(|x| x.1);
                #[cfg(test)]
                let keep = KEEP_OVERRIDE.with(|o| o.get()).filter(|o| Some(o.0) == kh2).map(|o| o.1).or(match (kh2, std::env::var("RUN_KEEP").ok().and_then(|x| x.parse::<f64>().ok())) { (Some(KH2_RUN), Some(k)) => Some(k), _ => keep });
                if let Some(k) = keep.filter(|k| *k < 1.0) {
                    let ih = rig.head_rel(&rel(&idle.world(&model.bones, kh2_time(idle, true, frames, f))));
                    for (rb, a) in hr.iter_mut() {
                        if front.contains(rb) && !rig.split.iter().any(|x| x.0 == *rb) {
                            continue;
                        }
                        let Some(b) = ih.iter().find(|x| x.0 == *rb).map(|x| x.1) else { continue };
                        a.r = rot_toward(&b.r, &a.r, k);
                        a.t = core::array::from_fn(|i| b.t[i] + k * (a.t[i] - b.t[i]));
                    }
                }
                tilt_hair(rig, &mut hr, &head_rest, corr, &front, &roots);
                damp_hair(rig, &mut hr, &rest_hr);
                rig.locals(parents, head, &hr)
            })
            .collect();
        for (k, &(rb, _)) in rows[0].iter().enumerate() {
            if rb >= c.bones as usize {
                return bad(format!("hair: bone {rb} past clip {slot}'s {}", c.bones));
            }
            let mut prev: Option<[f64; 3]> = rest_e.get(&rb).copied();
            let mut vals = vec![vec![0.0f64; frames]; chans];
            for (f, row) in rows.iter().enumerate() {
                let a = row[k].1;
                let e = euler_xyz(&a.r, prev);
                prev = Some(e);
                for i in 0..3 {
                    vals[i][f] = a.t[i];
                    vals[3 + i][f] = e[i].to_degrees();
                }
                for v in vals.iter_mut().skip(6) {
                    v[f] = 1.0;
                }
            }
            for (ci, v) in vals.iter().enumerate() {
                let eps = [1e-7, 1e-5, 1e-6][ci / 3];
                c.channels[rb * chans + ci] = crate::convert::encode_channel(v, eps);
            }
        }
    }
    Ok(())
}

/// The head's world matrix of a clip at frame `t` (the clip's channels, the skeleton's rest for bones past the clip).
pub fn pose_head(sk: &crate::mdl::RecomSkeleton, c: &crate::ann::Clip, t: f64, head: usize) -> Aff {
    let mut w: Vec<Aff> = Vec::with_capacity(head + 1);
    for (i, b) in sk.bones.iter().enumerate().take(head + 1) {
        let local = if i < c.bones as usize {
            let s = c.sample(i, t);
            let mut r = rot_xyz(s.r[0], s.r[1], s.r[2]);
            for (row, sc) in r.iter_mut().zip(s.s) {
                for v in row.iter_mut() {
                    *v *= sc;
                }
            }
            Aff { r, t: s.t }
        } else {
            b.local
        };
        w.push(if b.parent < 0 { local } else { local.mul(&w[b.parent as usize]) });
    }
    w[head]
}

/// Checks that `rot_xyz` of a local's Euler angles gives back the local (used by the tests).
pub fn euler_roundtrip(a: &Aff) -> f64 {
    let e = euler_xyz(&a.r, None);
    let r = rot_xyz(e[0], e[1], e[2]);
    (0..3).flat_map(|i| (0..3).map(move |j| (i, j))).map(|(i, j)| (r[i][j] - a.r[i][j]).abs()).fold(0.0, f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kh2_motion::read_slot;

    /// The 18 kept bones are the greedy choice on idle / walk / run / jump (local files; skipped without them).
    #[test]
    fn selection_is_the_greedy_one() {
        let Ok(mset) = std::fs::read(format!("{}/obj__P_EH000.mset", crate::kh2_motion::tests::raw())) else { return };
        let model = kh2_formats_lite::mdlx::parse(&std::fs::read(format!("{}/obj__P_EH000.mdlx", crate::kh2_motion::tests::raw())).unwrap()).unwrap();
        let ms: Vec<Kh2Motion> = CHOOSE_ON.iter().map(|&s| read_slot(&mset, s).unwrap().unwrap()).collect();
        let refs: Vec<&Kh2Motion> = ms.iter().collect();
        let kept = choose(&model, crate::kh2_hair::HAIR_GROUP, &refs, 18, &FORCED);
        let host = hosts(&model, crate::kh2_hair::HAIR_GROUP, &refs, &kept);
        let verts = vertices_by_bone(&model, crate::kh2_hair::HAIR_GROUP);
        eprintln!("kept {kept:?}");
        for d in HAIR_BONES { eprintln!("bone {d}: {} corners -> {:?}", verts.get(&d).map_or(0, |v| v.len()), host[&d]); }
        assert_eq!(kept, SELECTED.to_vec());
    }
}

#[cfg(test)]
mod unroll_tests {
    use super::*;

    #[test]
    fn unroll_times_closes_the_loops() {
        assert_eq!(unroll_times(100, 120.0), Some(6), "idle 1: 6 x 100 = 5 x 120");
        assert_eq!(unroll_times(80, 120.0), Some(3), "idle 31: 3 x 80 = 2 x 120");
        assert_eq!(unroll_times(40, 120.0), Some(3), "idle 79: 3 x 40 = 120");
        assert_eq!(unroll_times(120, 120.0), None, "already closes");
        assert_eq!(unroll_times(97, 120.0), None, "no fit within 6");
    }
}

#[cfg(test)]
mod damp_tests {
    use super::*;

    /// 0.196.16: `rot_toward` gives `a` at 0, `b` at 1 and half the turn at 0.5.
    #[test]
    fn rot_toward_ends_and_middle() {
        let a = rot_xyz(0.1, -0.2, 0.3);
        let b = rot_xyz(0.5, 0.4, -0.6);
        let near = |x: &[[f64; 3]; 3], y: &[[f64; 3]; 3]| (0..9).all(|i| (x[i / 3][i % 3] - y[i / 3][i % 3]).abs() < 1e-9);
        assert!(near(&rot_toward(&a, &b, 0.0), &a));
        assert!(near(&rot_toward(&a, &b, 1.0), &b));
        let h = rot_toward(&a, &b, 0.5);
        let (d1, d2) = (crate::math::angle_deg(&m3(&tr(&a), &h)), crate::math::angle_deg(&m3(&tr(&h), &b)));
        assert!((d1 - d2).abs() < 1e-6 && d1 > 1.0, "{d1} {d2}");
    }
}

#[cfg(test)]
mod render_tests {
    use super::*;
    use crate::ddd_hair::{tests as dt, with_hair, HairSource, RIKU_0101_KH2};
    use crate::install;
    use crate::mdl::{split_parts, MdlPart, RecomSkeleton};

    /// 0.195.0 (owner's install, `RECOM_GAME`): the KH2 hair with KH2's motion, posed by the baked clips; renders in the
    /// workspace's `local/assets/recom/riku-hair/out/kh2/rig/`; the deviation of every hair vertex from KH2's exact motion (all 25
    /// bones carried) is printed per clip.
    #[test]
    fn kh2_hair_moves_as_kh2() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let mdl0 = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        let ann0 = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.ANN").unwrap();
        let e = arc.find("PLAYER/0101/PL0101.RTM").unwrap();
        let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
        let src = crate::kh2_hair::Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap();
        assert!(src.mset.is_some());
        let mdl1 = src.prepare_mdl(&mdl0).unwrap();
        let swap = with_hair(&mdl1, &rtm, &hd, &RIKU_0101_KH2, &src).unwrap();
        let ann = crate::ann::Ann::parse(&src.bake_ann(&ann0, &mdl0).unwrap().unwrap()).unwrap();
        let orig = crate::ann::Ann::parse(&ann0).unwrap();
        // only hair bones change
        let sk = RecomSkeleton::parse(&swap.mdl).unwrap();
        let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
        let hairs: Vec<usize> = RECOM_HAIR.iter().map(|n| sk.find(n).unwrap()).collect();
        for (slot, (a, b)) in ann.slots.iter().zip(&orig.slots).enumerate() {
            let (Some(a), Some(b)) = (&a.clip, &b.clip) else { continue };
            // 0.196.5: the clip has the model's extra hair bones (7) and nothing else changed in shape
            // 0.197.6: the event-free idle clips are repeated whole (`UNROLL`): their body repeats Re:CoM's frame for frame
            let k = (a.frames / b.frames) as usize;
            assert!(a.frames % b.frames == 0 && (k == 1 || UNROLL.contains(&slot)), "clip {slot}: {} frames for {}", a.frames, b.frames);
            assert_eq!((a.bones as usize, a.loop_start, a.flags), (b.bones as usize + EXTRA.len() + SPLIT.len(), b.loop_start, b.flags));
            let ch = a.chans();
            for (i, (x, y)) in a.channels.iter().zip(&b.channels).enumerate() {
                if hairs.contains(&(i / ch)) {
                    continue;
                }
                if k == 1 {
                    assert!(x == y, "non-hair channel {i} changed");
                } else {
                    for f in 0..a.frames as usize {
                        let (va, vb) = (a.value(i / ch, i % ch, f as f64), b.value(i / ch, i % ch, (f % b.frames as usize) as f64));
                        assert!((va - vb).abs() < 1e-3, "clip {slot} channel {i} frame {f}: {va} vs {vb}");
                    }
                }
            }
            if k > 1 {
                eprintln!("clip {slot}: repeated {k} times ({} frames)", a.frames);
            }
        }
        // 0.196.5: every part of the model and every clip of its pack have the same bone count (the game poses a clip only then)
        let parts = split_parts(&swap.mdl).unwrap();
        for (pi, pb) in parts.iter().enumerate() {
            assert_eq!(MdlPart::parse_bytes(pb).unwrap().bones.len(), sk.bones.len(), "part {pi}'s bones");
        }
        assert_eq!(sk.bones.len(), RecomSkeleton::parse(&mdl0).unwrap().bones.len() + EXTRA.len() + SPLIT.len());
        for (slot, s) in ann.slots.iter().enumerate() {
            if let Some(c) = &s.clip {
                assert_eq!(c.bones as usize, sk.bones.len(), "clip {slot}'s bones");
            }
        }
        // 0.196.6: no clip of 8 frames or more holds the hair still: every one plays KH2's motion (its own, or the idle sway)
        let still: Vec<usize> = ann
            .slots
            .iter()
            .enumerate()
            .filter_map(|(slot, s)| {
                let c = s.clip.as_ref().filter(|c| c.frames >= 8)?;
                let ch = c.chans();
                let moving = hairs.iter().any(|&b| (0..ch).any(|k| matches!(c.channels[b * ch + k], crate::ann::Channel::Curve(_))));
                (!moving).then_some(slot)
            })
            .collect();
        eprintln!("clips of 8+ frames with the hair held still: {still:?}");
        assert!(still.is_empty(), "clips {still:?} hold the hair still");
        let (p3, img) = dt::test_view(&swap);
        let out = dt::out_dir().join("kh2").join("rig");
        std::fs::create_dir_all(&out).unwrap();
        // exact reference: every hair vertex carried by its own KH2 bone
        let model = kh2_formats_lite::mdlx::parse(&src.mdlx).unwrap();
        let rig = src.rig(&mdl0).unwrap().unwrap();
        let motions = src.motions().unwrap();
        let verts = vertices_by_bone(&model, crate::kh2_hair::HAIR_GROUP);
        let head = sk.find("head").unwrap();
        {
            let d = out.join("rest");
            std::fs::create_dir_all(&d).unwrap();
            let w = sk.rest_world();
            let _ = crate::kh2_hair::tests::render_body(&p3, &w, &names, &img, 0, &d.join("body_side.png"));
            let _ = crate::kh2_hair::tests::render_body(&p3, &w, &names, &img, 3, &d.join("body_side2.png"));
            // rest dump: hair (texture 1 on head / hair bones) 'h', the rest of the head 'r', world metres
            let mut txt = String::new();
            for t in dt::collect(&p3) {
                if !t.v.iter().all(|v| { let n = &names[v.bone()]; n == "head" || n.contains("hair") }) { continue; }
                let hair = t.v.iter().any(|v| names[v.bone()].contains("hair")) || t.tex == 1 && t.v.iter().all(|v| v.uv().is_some_and(|u| u[0] >= 0.0));
                let _ = hair;
                txt += &format!("{} {}\n", if t.v.iter().any(|v| names[v.bone()].contains("hair")) { 'h' } else { 'r' }, t.v.iter().flat_map(|v| w[v.bone()].point(v.pos().map(|x| x as f64))).map(|x| format!("{x:.5}")).collect::<Vec<_>>().join(" "));
            }
            std::fs::write(d.join("rest_tris.txt"), txt).unwrap();
        }
        for (clip, frames) in [(1usize, vec![0.0, 25.0, 50.0, 75.0]), (2, vec![0.0, 8.0, 16.0, 24.0]), (3, vec![0.0, 6.0, 11.0, 17.0]), (4, vec![8.0]), (6, vec![8.0]), (33, vec![5.0]), (37, vec![3.0, 10.0]), (44, vec![6.0, 12.0]), (41, vec![16.0])] {
            let Some(c) = ann.slots[clip].clip.as_ref() else { continue };
            let src_m = CLIP_MAP.iter().find(|x| x.0 == clip).and_then(|x| motions.get(&x.1));
            let mut worst = 0.0f64;
            for f in frames {
                let w = crate::kh2_hair::tests::posed_world(&sk, c, f);
                let d = out.join(format!("c{clip}_f{f}"));
                std::fs::create_dir_all(&d).unwrap();
                let _ = crate::kh2_hair::tests::render_body(&p3, &w, &names, &img, 0, &d.join("body_side.png"));
                let _ = crate::kh2_hair::tests::render_body(&p3, &w, &names, &img, 1, &d.join("body_back.png"));
                let _ = crate::kh2_hair::tests::render_body(&p3, &w, &names, &img, 2, &d.join("body_front.png"));
                let _ = crate::kh2_hair::tests::render_body(&p3, &w, &names, &img, 3, &d.join("body_side2.png"));
                {
                    // posed dump in the rest head's frame (the face lands where it is at rest)
                    let hrest = sk.rest_world()[head];
                    let back = |p: [f64; 3]| hrest.point(rigid_inv(&Aff { r: unscale(&w[head].r).0, t: w[head].t }).point(p));
                    let mut txt = String::new();
                    for t in dt::collect(&p3) {
                        if !t.v.iter().all(|v| { let n = &names[v.bone()]; n == "head" || n.contains("hair") }) { continue; }
                        txt += &format!("{} {}\n", if t.v.iter().any(|v| names[v.bone()].contains("hair")) { 'h' } else { 'r' }, t.v.iter().flat_map(|v| back(w[v.bone()].point(v.pos().map(|x| x as f64)))).map(|x| format!("{x:.5}")).collect::<Vec<_>>().join(" "));
                    }
                    std::fs::write(d.join("head_tris.txt"), txt).unwrap();
                }
                if let Some(m) = src_m {
                    let looped = c.looped() && c.loop_start == 1;
                    let r = rel(&m.world(&model.bones, kh2_time(m, looped, c.frames as usize, f as usize)));
                    let bind = rel(&kh2_formats_lite::skeleton::bind_pose(&model.bones));
                    // 0.196.5: what the GAME plays (the baked clip posed on the model), against KH2's exact motion, for all 25 hair bones
                    let hw = Aff { r: unscale(&w[head].r).0, t: w[head].t };
                    let hinv = rigid_inv(&hw);
                    for &kb in &HAIR_BONES {
                        // 0.196.16: the damped locks leave KH2's motion on purpose
                        if DAMP.iter().any(|x| x.0 == kb && x.1 < 1.0) { continue; }
                        let Some(rb) = rig.recom_of(kb) else { continue };
                        let ours_hr = w[rb].mul(&hinv);
                        let rest_hr = rig.carry.aff(&bind[kb]);
                        for v in verts.get(&kb).into_iter().flatten() {
                            let exact = rig.carry.point(r[kb].point(*v));
                            let u = rigid_inv(&rest_hr).point(rig.carry.point(bind[kb].point(*v)));
                            worst = worst.max(crate::math::dist(exact, ours_hr.point(u)));
                        }
                    }
                }
            }
            eprintln!("clip {clip}: worst vertex off KH2's exact motion {:.3} m", worst);
            // 0.196.5: the baked clip plays KH2's motion for all 25 hair bones (0.196.3: 3.5-7.9 cm off, 7 bones merged into others)
            // 0.198.6: clips on a KH2 motion with an extra turn ([`EXTRA_TILT`], walk and run) leave KH2's motion on purpose
            let tilted = CLIP_MAP.iter().find(|x| x.0 == clip).is_some_and(|x| EXTRA_TILT.iter().any(|t| t.0 == x.1 && t.1 != 0.0));
            if src_m.is_some() && !tilted {
                assert!(worst < 0.02, "clip {clip}: the baked hair is {worst:.3} m off KH2's motion");
            }
        }
    }
}

#[cfg(test)]
mod kh2_self_render {
    use super::*;
    use crate::kh2_motion::read_slot;

    /// Debug: KH2 Riku's whole model posed by his own motions with this reader (side views, hair red), to see how KH2's hair
    /// hangs in KH2's own world. `local/assets/recom/riku-hair/out/kh2/rig/kh2_self.png`.
    #[test]
    fn kh2_self_side_views() {
        let Ok(mset) = std::fs::read(format!("{}/obj__P_EH000.mset", crate::kh2_motion::tests::raw())) else { return };
        let model = kh2_formats_lite::mdlx::parse(&std::fs::read(format!("{}/obj__P_EH000.mdlx", crate::kh2_motion::tests::raw())).unwrap()).unwrap();
        let pose0 = kh2_formats_lite::skeleton::bind_pose(&model.bones);
        let views: Vec<(usize, f32)> = vec![(KH2_IDLE, 0.0), (KH2_IDLE, 60.0), (KH2_WALK, 10.0), (KH2_RUN, 6.0), (KH2_JUMP, 8.0), (KH2_FALL, 25.0)];
        let (w, h) = (400usize, 500usize);
        let mut sheet = vec![30u8; w * views.len() * h * 3];
        for (vi, &(slot, f)) in views.iter().enumerate() {
            let m = read_slot(&mset, slot).unwrap().unwrap();
            // the emulator's matrices (the whole body as KH2 poses it, IK and constraints included)
            let raw = std::fs::read(format!("{}/slot{slot}.f32", crate::kh2_motion::tests::emu())).unwrap();
            let fl: Vec<f32> = raw.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect();
            let per = (fl.len() / 16) / (m.frame_end.round() as usize + 1);
            let fi = (f.round() as usize).min(m.frame_end.round() as usize);
            let world: Vec<kh2_formats_lite::skeleton::Mat> = (0..model.bones.len()).map(|b| { let o = (fi * per + b) * 16; core::array::from_fn(|i| core::array::from_fn(|j| fl[o + i * 4 + j] as f64)) }).collect();
            let mut zb = vec![f64::MIN; w * h];
            for (gi, g) in model.groups.iter().enumerate() {
                for t in &g.triangles {
                    let p: Vec<[f64; 3]> = t.iter().map(|v| {
                        let wb = kh2_formats_lite::skeleton::point(&pose0[v.bone as usize], v.position.map(|x| x as f64));
                        let l = rigid_inv(&aff(&pose0[v.bone as usize])).point(wb);
                        aff(&world[v.bone as usize]).point(l)
                    }).collect();
                    // side view: screen x = z, y = y; depth = -x
                    let s: Vec<[f64; 3]> = p.iter().map(|q| [200.0 + q[2] * 2.2, 480.0 - q[1] * 2.2, -q[0]]).collect();
                    let det = (s[1][1] - s[2][1]) * (s[0][0] - s[2][0]) + (s[2][0] - s[1][0]) * (s[0][1] - s[2][1]);
                    if det.abs() < 1e-9 { continue; }
                    let (x0, x1) = (s.iter().map(|q| q[0]).fold(f64::MAX, f64::min).max(0.0) as usize, (s.iter().map(|q| q[0]).fold(f64::MIN, f64::max) as usize + 1).min(w - 1));
                    let (y0, y1) = (s.iter().map(|q| q[1]).fold(f64::MAX, f64::min).max(0.0) as usize, (s.iter().map(|q| q[1]).fold(f64::MIN, f64::max) as usize + 1).min(h - 1));
                    for y in y0..=y1 { for x in x0..=x1 {
                        let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
                        let l0 = ((s[1][1] - s[2][1]) * (px - s[2][0]) + (s[2][0] - s[1][0]) * (py - s[2][1])) / det;
                        let l1 = ((s[2][1] - s[0][1]) * (px - s[2][0]) + (s[0][0] - s[2][0]) * (py - s[2][1])) / det;
                        let l2 = 1.0 - l0 - l1;
                        if l0 < 0.0 || l1 < 0.0 || l2 < 0.0 { continue; }
                        let z = l0 * s[0][2] + l1 * s[1][2] + l2 * s[2][2];
                        if z <= zb[y * w + x] { continue; }
                        zb[y * w + x] = z;
                        let c = if gi == crate::kh2_hair::HAIR_GROUP { [200u8, 80, 80] } else { [110, 140, 200] };
                        let o = 3 * (y * w * views.len() + vi * w + x);
                        sheet[o..o + 3].copy_from_slice(&c);
                    } }
                }
            }
        }
        let out = crate::ddd_hair::tests::out_dir().join("kh2").join("rig");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("kh2_self.png"), crate::ddd_hair::tests::png(w * views.len(), h, &sheet)).unwrap();
    }
}

/// 0.198.3 (fork owner on 0.198.2: "não tem como melhorasr a parte de trás do cabelo como um todo?"): KH2 Riku as KH2 itself draws
/// him (his model, textures and the emulator's matrices of his own motions), from the cameras of `kh2_hair::tests::camera_checks`:
/// the run from behind, low and high, and the idle circled; `local/assets/recom/riku-hair/out/kh2/kh2_self_cam/`.
#[cfg(test)]
mod kh2_self_cam {
    use super::*;
    use crate::kh2_motion::read_slot;

    fn render(model: &kh2_formats_lite::mdlx::Model, pose0: &[kh2_formats_lite::skeleton::Mat], world: &[kh2_formats_lite::skeleton::Mat], yaw: f64, pitch: f64, path: &std::path::Path) {
        let (sy, cy, sp, cp) = (yaw.to_radians().sin(), yaw.to_radians().cos(), pitch.to_radians().sin(), pitch.to_radians().cos());
        let tx = |q: [f64; 3]| -> [f64; 3] {
            let (x, y, z) = (q[0] * cy + q[2] * sy, q[1], -q[0] * sy + q[2] * cy);
            [x, y * cp - z * sp, y * sp + z * cp]
        };
        let mut tris = vec![];
        for (gi, g) in model.groups.iter().enumerate() {
            for t in &g.triangles {
                let p: Vec<[f64; 3]> = t.iter().map(|v| {
                    let wb = kh2_formats_lite::skeleton::point(&pose0[v.bone as usize], v.position.map(|x| x as f64));
                    tx(aff(&world[v.bone as usize]).point(rigid_inv(&aff(&pose0[v.bone as usize])).point(wb)))
                }).collect();
                tris.push((p, t.iter().map(|v| v.uv).collect::<Vec<_>>(), g.texture, gi == crate::kh2_hair::HAIR_GROUP));
            }
        }
        // the view: the hair's box (KH2's hair group) grown 2.2 times about its centre: head, hair, neck, shoulders
        let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
        for (gi, t) in tris.iter().enumerate() { let _ = gi; if t.3 { for q in &t.0 { for k in 0..2 { lo[k] = lo[k].min(q[k]); hi[k] = hi[k].max(q[k]); } } } }
        let (c, r) = ([(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0], 1.1 * (hi[0] - lo[0]).max(hi[1] - lo[1]));
        let (lo, hi) = ([c[0] - r, c[1] - 1.3 * r], [c[0] + r, c[1] + 0.7 * r]);
        let size = 700usize;
        let sc = 640.0 / (hi[0] - lo[0]).max(hi[1] - lo[1]);
        let mut rgb = [214u8, 212, 230].repeat(size * size);
        let mut zb = vec![f64::MIN; size * size];
        for (p, uv, ti, _) in &tris {
            let s: Vec<[f64; 2]> = p.iter().map(|q| [30.0 + (q[0] - lo[0]) * sc, 670.0 - (q[1] - lo[1]) * sc]).collect();
            let n = { let a = [p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]]; let b = [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]]; let c = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]; let l = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt().max(1e-12); c[2].abs() / l };
            let det = (s[1][1] - s[2][1]) * (s[0][0] - s[2][0]) + (s[2][0] - s[1][0]) * (s[0][1] - s[2][1]);
            if det.abs() < 1e-9 { continue; }
            if s.iter().all(|q| q[0] < -50.0 || q[0] > size as f64 + 50.0 || q[1] < -50.0 || q[1] > size as f64 + 50.0) { continue; }
            let (x0, x1) = (s.iter().map(|q| q[0]).fold(f64::MAX, f64::min).floor().max(0.0) as usize, (s.iter().map(|q| q[0]).fold(f64::MIN, f64::max).ceil().max(0.0) as usize).min(size - 1));
            let (y0, y1) = (s.iter().map(|q| q[1]).fold(f64::MAX, f64::min).floor().max(0.0) as usize, (s.iter().map(|q| q[1]).fold(f64::MIN, f64::max).ceil().max(0.0) as usize).min(size - 1));
            let tex = model.textures.get(*ti);
            for y in y0..=y1 { for x in x0..=x1 {
                let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
                let l0 = ((s[1][1] - s[2][1]) * (px - s[2][0]) + (s[2][0] - s[1][0]) * (py - s[2][1])) / det;
                let l1 = ((s[2][1] - s[0][1]) * (px - s[2][0]) + (s[0][0] - s[2][0]) * (py - s[2][1])) / det;
                let l2 = 1.0 - l0 - l1;
                if l0 < 0.0 || l1 < 0.0 || l2 < 0.0 { continue; }
                let z = l0 * p[0][2] + l1 * p[1][2] + l2 * p[2][2];
                if z <= zb[y * size + x] { continue; }
                let c = match tex {
                    Some(t) if t.width > 0 => {
                        let u = (l0 * uv[0][0] as f64 + l1 * uv[1][0] as f64 + l2 * uv[2][0] as f64).rem_euclid(1.0);
                        let v = (l0 * uv[0][1] as f64 + l1 * uv[1][1] as f64 + l2 * uv[2][1] as f64).rem_euclid(1.0);
                        let (w, h) = (t.width as usize, t.height as usize);
                        let o = 4 * (((v * h as f64) as usize).min(h - 1) * w + ((u * w as f64) as usize).min(w - 1));
                        if t.rgba[o + 3] < 16 { continue; }
                        let k = 0.55 + 0.45 * n;
                        [(t.rgba[o] as f64 * k) as u8, (t.rgba[o + 1] as f64 * k) as u8, (t.rgba[o + 2] as f64 * k) as u8]
                    }
                    _ => { let k = 60.0 + 140.0 * n; [(k * 0.5) as u8, (k * 0.7) as u8, k as u8] }
                };
                zb[y * size + x] = z;
                rgb[3 * (y * size + x)..3 * (y * size + x) + 3].copy_from_slice(&c);
            } }
        }
        std::fs::write(path, crate::ddd_hair::tests::png(size, size, &rgb)).unwrap();
    }

    #[test]
    fn kh2_self_camera_checks() {
        let Ok(mset) = std::fs::read(format!("{}/obj__P_EH000.mset", crate::kh2_motion::tests::raw())) else { return };
        let model = kh2_formats_lite::mdlx::parse(&std::fs::read(format!("{}/obj__P_EH000.mdlx", crate::kh2_motion::tests::raw())).unwrap()).unwrap();
        let pose0 = kh2_formats_lite::skeleton::bind_pose(&model.bones);
        let dir = crate::ddd_hair::tests::out_dir().join("kh2").join("kh2_self_cam");
        std::fs::create_dir_all(&dir).unwrap();
        let frames = |slot: usize| -> (Vec<f32>, usize, usize) {
            let m = read_slot(&mset, slot).unwrap().unwrap();
            let raw = std::fs::read(format!("{}/slot{slot}.f32", crate::kh2_motion::tests::emu_full())).unwrap();
            let fl: Vec<f32> = raw.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect();
            // the dump has `frame_end × keys_per_frame + 1` samples (two per frame at 30 fps), as `kh2_and_recom_side_by_side` reads it
            let n = (m.frame_end * m.keys_per_frame()).round() as usize + 1;
            let per = (fl.len() / 16) / n;
            (fl, per, n)
        };
        let world_at = |fl: &[f32], per: usize, fi: usize| -> Vec<kh2_formats_lite::skeleton::Mat> {
            (0..model.bones.len()).map(|b| { let o = (fi * per + b) * 16; core::array::from_fn(|i| core::array::from_fn(|j| fl[o + i * 4 + j] as f64)) }).collect()
        };
        let (fl, per, n) = frames(KH2_RUN);
        eprintln!("KH2 run: {n} samples, {per} matrices each, {} groups, {} textures", model.groups.len(), model.textures.len());
        for (tag, pitch) in [("low", -35.0), ("level", 0.0), ("high", 35.0)] {
            for f in [0usize, 4, 9, 13] {
                for yaw in (0..8).map(|k| k * 45) {
                    render(&model, &pose0, &world_at(&fl, per, (2 * f).min(n - 1)), yaw as f64, pitch, &dir.join(format!("run_{tag}_f{f}_y{yaw:03}.png")));
                }
            }
        }
        let (fl, per, n) = frames(KH2_IDLE);
        for k in 0..8 {
            render(&model, &pose0, &world_at(&fl, per, 50.min(n - 1)), k as f64 * 45.0, -15.0, &dir.join(format!("orbit_{:03}.png", k * 45)));
        }
    }
}

#[cfg(test)]
mod amp_tests {
    use super::*;
    use crate::kh2_motion::read_slot;
    /// Debug: per KH2 hair bone, its corners, and how far (KH2 units) its vertices move off the rigid head on each motion.
    #[test]
    fn bone_amplitudes() {
        let Ok(mset) = std::fs::read(format!("{}/obj__P_EH000.mset", crate::kh2_motion::tests::raw())) else { return };
        let model = kh2_formats_lite::mdlx::parse(&std::fs::read(format!("{}/obj__P_EH000.mdlx", crate::kh2_motion::tests::raw())).unwrap()).unwrap();
        let bind = rel(&kh2_formats_lite::skeleton::bind_pose(&model.bones));
        let verts = vertices_by_bone(&model, crate::kh2_hair::HAIR_GROUP);
        for &b in &HAIR_BONES {
            let mut line = format!("bone {b} at {:?} corners {}:", bind[b].t.map(|x| x.round()), verts.get(&b).map_or(0, |v| v.len()));
            for s in [KH2_IDLE, KH2_WALK, KH2_RUN, KH2_JUMP] {
                let m = read_slot(&mset, s).unwrap().unwrap();
                let mut worst = 0.0f64;
                let mut f = 0.0;
                while f <= m.frame_end {
                    let r = rel(&m.world(&model.bones, f));
                    for v in verts.get(&b).into_iter().flatten() { worst = worst.max(crate::math::dist(r[b].point(*v), bind[b].point(*v))); }
                    f += 2.0;
                }
                line += &format!(" {:.1}", worst);
            }
            eprintln!("{line}");
        }
    }
}

    /// 0.198.3 (fork owner, "a" to 3(a): the run's hair tilted by the lean difference): the forward tilt (deg, each bone against its
    /// own rest) of the head and its ancestors in KH2's run (emulator) and in Re:CoM's run clips, and the head against the bone
    /// three up (the upper chest): what the hair, head-relative, sits at against the back.
    #[test]
    fn run_lean() {
        let Ok(mset) = std::fs::read(format!("{}/obj__P_EH000.mset", crate::kh2_motion::tests::raw())) else { return };
        let model = kh2_formats_lite::mdlx::parse(&std::fs::read(format!("{}/obj__P_EH000.mdlx", crate::kh2_motion::tests::raw())).unwrap()).unwrap();
        let bind = kh2_formats_lite::skeleton::bind_pose(&model.bones);
        let mut chain = vec![HEAD];
        while chain.len() < 6 { let p = model.bones[*chain.last().unwrap()].parent; if p < 0 { break; } chain.push(p as usize); }
        let ks = std::env::var("LEAN_SLOT").ok().and_then(|x| x.parse().ok()).unwrap_or(KH2_RUN);
        let m = crate::kh2_motion::read_slot(&mset, ks).unwrap().unwrap();
        let raw = std::fs::read(format!("{}/slot{ks}.f32", crate::kh2_motion::tests::emu_full())).unwrap();
        let fl: Vec<f32> = raw.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect();
        let n = (m.frame_end * m.keys_per_frame()).round() as usize + 1;
        let per = (fl.len() / 16) / n;
        let at = |f: usize, b: usize| -> [[f64; 3]; 3] { let o = (f * per + b) * 16; let mm: kh2_formats_lite::skeleton::Mat = core::array::from_fn(|i| core::array::from_fn(|j| fl[o + i * 4 + j] as f64)); aff(&mm).r };
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        for &b in &chain {
            let v: Vec<f64> = (0..n).map(|f| tilt(&aff(&bind[b]).r, &at(f, b))).collect();
            eprintln!("KH2 run bone {b}: mean {:.1} range {:.1}..{:.1}", mean(&v), v.iter().cloned().fold(f64::MAX, f64::min), v.iter().cloned().fold(f64::MIN, f64::max));
        }
        let up = |r: &[[f64; 3]; 3]| vm([0.0, 1.0, 0.0], r);
        let ang = |a: [f64; 3], b: [f64; 3]| { let d = a[0] * b[0] + a[1] * b[1] + a[2] * b[2]; let la = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt(); let lb = (b[0] * b[0] + b[1] * b[1] + b[2] * b[2]).sqrt(); (d / la / lb).clamp(-1.0, 1.0).acos().to_degrees() };
        // the head's up against the chest's up, at rest and in the run (the bend of the neck)
        let c3 = chain[3.min(chain.len() - 1)];
        let r0 = ang(up(&aff(&bind[HEAD]).r), up(&aff(&bind[c3]).r));
        let v: Vec<f64> = (0..n).map(|f| ang(up(&at(f, HEAD)), up(&at(f, c3))) - r0).collect();
        eprintln!("KH2 run: head vs bone {c3}: bend {:.1} (range {:.1}..{:.1}) beyond rest", mean(&v), v.iter().cloned().fold(f64::MAX, f64::min), v.iter().cloned().fold(f64::MIN, f64::max));
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = crate::install::keys(&g).unwrap();
        let arc = crate::install::recom_archive(&g).unwrap();
        let sk = crate::mdl::RecomSkeleton::parse(&crate::install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap()).unwrap();
        let ann = crate::ann::Ann::parse(&crate::install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.ANN").unwrap()).unwrap();
        let head = sk.find("head").unwrap();
        let mut rc = vec![head];
        while rc.len() < 6 { let p = sk.bones[*rc.last().unwrap()].parent; if p < 0 { break; } rc.push(p as usize); }
        let rw = sk.rest_world();
        for slot in std::env::var("LEAN_CLIPS").ok().map(|x| x.split(',').filter_map(|y| y.parse().ok()).collect::<Vec<usize>>()).unwrap_or(vec![3, 21, 33]) {
            let c = ann.slots[slot].clip.as_ref().unwrap();
            let ws: Vec<Vec<Aff>> = (0..c.frames as usize).map(|f| crate::kh2_hair::tests::posed_world(&sk, c, f as f64)).collect();
            for &b in &rc {
                let v: Vec<f64> = ws.iter().map(|w| tilt(&unscale(&rw[b].r).0, &unscale(&w[b].r).0)).collect();
                eprintln!("Re:CoM {slot} bone {} ({}): mean {:.1} range {:.1}..{:.1}", b, sk.bones[b].name, mean(&v), v.iter().cloned().fold(f64::MAX, f64::min), v.iter().cloned().fold(f64::MIN, f64::max));
            }
            let c3 = rc[3.min(rc.len() - 1)];
            let r0 = ang(up(&unscale(&rw[head].r).0), up(&unscale(&rw[c3].r).0));
            let v: Vec<f64> = ws.iter().map(|w| ang(up(&unscale(&w[head].r).0), up(&unscale(&w[c3].r).0)) - r0).collect();
            eprintln!("Re:CoM {slot}: head vs {}: bend {:.1} (range {:.1}..{:.1}) beyond rest", sk.bones[c3].name, mean(&v), v.iter().cloned().fold(f64::MAX, f64::min), v.iter().cloned().fold(f64::MIN, f64::max));
        }
    }


#[cfg(test)]
mod tilt_tests {
    use super::*;
    use crate::kh2_motion::read_slot;
    /// Debug: the head's forward tilt (degrees, in the model's y-z plane, + = face down / forward) against its rest, KH2 (emulator
    /// matrices) and Re:CoM (the mapped clips).
    #[test]
    fn head_tilts() {
        let Ok(mset) = std::fs::read(format!("{}/obj__P_EH000.mset", crate::kh2_motion::tests::raw())) else { return };
        let model = kh2_formats_lite::mdlx::parse(&std::fs::read(format!("{}/obj__P_EH000.mdlx", crate::kh2_motion::tests::raw())).unwrap()).unwrap();
        let kb = aff(&kh2_formats_lite::skeleton::bind_pose(&model.bones)[HEAD]);
        // fwd tilt of a head: carry the rest head's world up (0,1,0) by (posed · rest^-1); angle in y-z, + toward +z
        let tilt = |rest: &[[f64; 3]; 3], posed: &[[f64; 3]; 3]| { let up = vm(vm([0.0, 1.0, 0.0], &tr(rest)), posed); up[2].atan2(up[1]).to_degrees() };
        for s in [KH2_IDLE, KH2_WALK, KH2_RUN, KH2_JUMP, KH2_FALL] {
            let m = read_slot(&mset, s).unwrap().unwrap();
            let Ok(raw) = std::fs::read(format!("{}/slot{s}.f32", crate::kh2_motion::tests::emu())) else { continue };
            let fl: Vec<f32> = raw.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect();
            let n = m.frame_end.round() as usize + 1;
            let per = (fl.len() / 16) / n;
            let v: Vec<f64> = (0..n).map(|f| { let o = (f * per + HEAD) * 16; let mm: kh2_formats_lite::skeleton::Mat = core::array::from_fn(|i| core::array::from_fn(|j| fl[o + i * 4 + j] as f64)); tilt(&kb.r, &aff(&mm).r) }).collect();
            eprintln!("KH2 slot {s}: mean {:.1} range {:.1}..{:.1}", v.iter().sum::<f64>() / n as f64, v.iter().cloned().fold(f64::MAX, f64::min), v.iter().cloned().fold(f64::MIN, f64::max));
        }
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = crate::install::keys(&g).unwrap();
        let arc = crate::install::recom_archive(&g).unwrap();
        let sk = crate::mdl::RecomSkeleton::parse(&crate::install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap()).unwrap();
        let ann = crate::ann::Ann::parse(&crate::install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.ANN").unwrap()).unwrap();
        let head = sk.find("head").unwrap();
        let rest = unscale(&sk.rest_world()[head].r).0;
        for &(slot, s) in CLIP_MAP.iter() {
            let c = ann.slots[slot].clip.as_ref().unwrap();
            let v: Vec<f64> = (0..c.frames as usize).map(|f| tilt(&rest, &unscale(&crate::kh2_hair::tests::posed_world(&sk, c, f as f64)[head].r).0)).collect();
            eprintln!("Re:CoM {slot} (KH2 {s}): mean {:.1} range {:.1}..{:.1}", v.iter().sum::<f64>() / v.len() as f64, v.iter().cloned().fold(f64::MAX, f64::min), v.iter().cloned().fold(f64::MIN, f64::max));
        }
    }
}

#[cfg(test)]
mod euler_tests {
    use super::*;
    use crate::ddd_hair::HairSource;
    use crate::install;
    /// Hair-bone Euler channels of the baked pack: the largest step between frames inside a clip, and the largest difference between
    /// clips' first frames (what a blend from one clip into another crosses), in degrees.
    #[test]
    fn baked_euler_is_continuous() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let mdl0 = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        let ann0 = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.ANN").unwrap();
        let src = crate::kh2_hair::Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap();
        let _ = src.prepare_mdl(&mdl0).unwrap();
        let ann = crate::ann::Ann::parse(&src.bake_ann(&ann0, &mdl0).unwrap().unwrap()).unwrap();
        let sk = crate::mdl::RecomSkeleton::parse(&mdl0).unwrap();
        let hairs: Vec<usize> = RECOM_HAIR.iter().map(|n| sk.find(n).unwrap()).collect();
        let (mut step, mut across) = (0.0f64, 0.0f64);
        let mut firsts: Vec<(usize, Vec<f64>)> = vec![];
        for (i, s) in ann.slots.iter().enumerate() {
            let Some(c) = &s.clip else { continue };
            let mut first = vec![];
            for &b in &hairs {
                for ch in 3..6 {
                    let v: Vec<f64> = (0..c.frames as usize).map(|f| c.value(b, ch, f as f64)).collect();
                    for w in v.windows(2) { let d = (w[1] - w[0]).abs(); if d > step { step = d; eprintln!("step {d:.1} deg: clip {i} bone {b} ch {ch}"); } }
                    first.push(v[0]);
                }
            }
            for (j, f) in &firsts { for (k, (a, b)) in f.iter().zip(&first).enumerate() { let d = (a - b).abs(); if d > 60.0 { eprintln!("big {d:.1} deg: clip {j} vs {i}, hair {} ch {}", k / 3, 3 + k % 3); } across = across.max(d); } }
            firsts.push((i, first));
        }
        eprintln!("worst step {step:.1}, worst across clips {across:.1}");
        // 0.195.1: no clip's first frame is a turn away from another's (0.195.0: up to 359 deg, the hair spun on a blend)
        assert!(across < 180.0, "hair-bone Euler angles {across:.1} deg apart between clips' first frames");
        // Seen, not asserted (0.196.2): in the run (clips 3, 21) `l_hair1` / `l_hair2` step up to 45 deg in one frame (KH2's run
        // at full speed, or an Euler near its gimbal: not settled).
    }
}

#[cfg(test)]
mod loop_tests {
    use super::*;
    use crate::kh2_motion::read_slot;
    /// How far each hair vertex is between a looped KH2 motion's loop start and its end (must be ~0: no jump at the loop), and the
    /// largest move between two consecutive frames inside the motion (KH2 units, printed).
    #[test]
    fn kh2_loops_close() {
        let Ok(mset) = std::fs::read(format!("{}/obj__P_EH000.mset", crate::kh2_motion::tests::raw())) else { return };
        let model = kh2_formats_lite::mdlx::parse(&std::fs::read(format!("{}/obj__P_EH000.mdlx", crate::kh2_motion::tests::raw())).unwrap()).unwrap();
        let verts = vertices_by_bone(&model, crate::kh2_hair::HAIR_GROUP);
        let at = |m: &Kh2Motion, t: f32| -> Vec<[f64; 3]> { let r = rel(&m.world(&model.bones, t)); HAIR_BONES.iter().flat_map(|&b| verts.get(&b).into_iter().flatten().map(move |v| (b, *v))).map(|(b, v)| r[b].point(v)).collect() };
        for s in [KH2_IDLE, KH2_WALK, KH2_RUN, KH2_FALL] {
            let m = read_slot(&mset, s).unwrap().unwrap();
            let (a, b) = (at(&m, m.frame_return), at(&m, m.frame_end));
            let gap = a.iter().zip(&b).map(|(x, y)| crate::math::dist(*x, *y)).fold(0.0, f64::max);
            let mut step = 0.0f64;
            let mut f = m.frame_return;
            while f < m.frame_end { let (x, y) = (at(&m, f), at(&m, f + 1.0)); step = step.max(x.iter().zip(&y).map(|(p, q)| crate::math::dist(*p, *q)).fold(0.0, f64::max)); f += 1.0; }
            eprintln!("KH2 slot {s}: loop {}..{}: seam {gap:.2}, largest frame step {step:.2}", m.frame_return, m.frame_end);
            // 0.195.2: a looped motion closes on itself (0.195.0 / 0.195.1 jumped 6.4 KH2 units here)
            assert!(gap < 0.05, "KH2 slot {s}: the hair jumps {gap:.2} KH2 units at the loop");
            let a0 = at(&m, m.frame_start);
            let mut best = (0.0f32, f64::MAX);
            let mut t = m.frame_start + 1.0;
            while t <= m.frame_end + 0.01 { let d = at(&m, t).iter().zip(&a0).map(|(x, y)| crate::math::dist(*x, *y)).fold(0.0, f64::max); if d < best.1 { best = (t, d); } t += 0.5; }
            let hair_curves: Vec<&crate::kh2_motion::Curve> = m.curves.iter().filter(|c| c.joint >= 99 && c.joint <= 148).collect();
            let kt0 = hair_curves.iter().filter_map(|c| c.keys.first().map(|k| k.time)).fold(f32::MAX, f32::min);
            let kt1 = hair_curves.iter().filter_map(|c| c.keys.last().map(|k| k.time)).fold(f32::MIN, f32::max);
            let posts: std::collections::BTreeSet<(u8, u8)> = hair_curves.iter().map(|c| (c.pre, c.post)).collect();
            eprintln!("   start {} end {} ret {} fps {}; closest to frame start at t {} ({:.2}); hair keys {kt0}..{kt1}, (pre, post) {posts:?}", m.frame_start, m.frame_end, m.frame_return, m.fps, best.0, best.1);
        }
    }
}

#[cfg(test)]
mod side_by_side {
    //! Debug (0.195.3): KH2 Riku as KH2 poses him (OpenKH emulator matrices, `local/assets/kh2/riku/emu_full`, whole motions in
    //! 60 Hz time) next to Re:CoM Riku with the baked KH2 hair, frame by frame, same head-centred camera turned to the face's
    //! heading, same scale (KH2 units × [`crate::kh2_hair::SCALE`]). Frames in the workspace's
    //! `local/assets/recom/riku-hair/out/kh2/sbs/<clip>/`, panels: KH2 side | Re:CoM side | KH2 back | Re:CoM back.
    use super::*;
    use crate::ddd_hair::{tests as dt, with_hair, HairSource, RIKU_0101_KH2};
    use crate::install;
    use crate::mdl::RecomSkeleton;
    use crate::texture::Image;

    const S: usize = 320;

    struct Tri { p: [[f64; 3]; 3], uv: Option<[[f64; 2]; 3]>, hair: bool }

    /// Rasterises `tris` (already in a head-centred, face-to-+z frame, metres) from the side (+x) or the back, z-buffered.
    fn raster(tris: &[Tri], atlas: &Image, back: bool, px_per_m: f64) -> Vec<u8> {
        let mut rgb = vec![28u8; S * S * 3];
        let mut zb = vec![f64::MIN; S * S];
        for t in tris {
            let q: Vec<[f64; 3]> = t.p.iter().map(|p| if back { [-p[0], p[1], -p[2]] } else { [p[2], p[1], -p[0]] }).collect();
            let s: Vec<[f64; 2]> = q.iter().map(|p| [S as f64 * 0.5 + p[0] * px_per_m, S as f64 * 0.42 - p[1] * px_per_m]).collect();
            let det = (s[1][1] - s[2][1]) * (s[0][0] - s[2][0]) + (s[2][0] - s[1][0]) * (s[0][1] - s[2][1]);
            if det.abs() < 1e-9 { continue; }
            let a = [q[1][0] - q[0][0], q[1][1] - q[0][1], q[1][2] - q[0][2]];
            let b = [q[2][0] - q[0][0], q[2][1] - q[0][1], q[2][2] - q[0][2]];
            let n = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
            let shade = 0.55 + 0.45 * (n[2].abs() / (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-12));
            let (x0, x1) = (s.iter().map(|p| p[0]).fold(f64::MAX, f64::min).floor().max(0.0) as usize, (s.iter().map(|p| p[0]).fold(f64::MIN, f64::max).ceil().max(0.0) as usize).min(S - 1));
            let (y0, y1) = (s.iter().map(|p| p[1]).fold(f64::MAX, f64::min).floor().max(0.0) as usize, (s.iter().map(|p| p[1]).fold(f64::MIN, f64::max).ceil().max(0.0) as usize).min(S - 1));
            if x0 > x1 || y0 > y1 { continue; }
            for y in y0..=y1 { for x in x0..=x1 {
                let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
                let l0 = ((s[1][1] - s[2][1]) * (px - s[2][0]) + (s[2][0] - s[1][0]) * (py - s[2][1])) / det;
                let l1 = ((s[2][1] - s[0][1]) * (px - s[2][0]) + (s[0][0] - s[2][0]) * (py - s[2][1])) / det;
                let l2 = 1.0 - l0 - l1;
                if l0 < 0.0 || l1 < 0.0 || l2 < 0.0 { continue; }
                let z = l0 * q[0][2] + l1 * q[1][2] + l2 * q[2][2];
                if z <= zb[y * S + x] { continue; }
                zb[y * S + x] = z;
                let c = match (t.hair, t.uv) {
                    (true, Some(uv)) => {
                        let u = l0 * uv[0][0] + l1 * uv[1][0] + l2 * uv[2][0];
                        let v = l0 * uv[0][1] + l1 * uv[1][1] + l2 * uv[2][1];
                        let (u, v) = (u.rem_euclid(1.0), v.rem_euclid(1.0));
                        let c = atlas.px[((v * atlas.h as f64) as usize).min(atlas.h - 1) * atlas.w + ((u * atlas.w as f64) as usize).min(atlas.w - 1)];
                        [(c[0] as f64 * shade) as u8, (c[1] as f64 * shade) as u8, (c[2] as f64 * shade) as u8]
                    }
                    _ => { let k = 60.0 + 150.0 * shade - 60.0; [(k * 0.55) as u8, (k * 0.7) as u8, k as u8] }
                };
                rgb[3 * (y * S + x)..3 * (y * S + x) + 3].copy_from_slice(&c);
            } }
        }
        rgb
    }

    /// The frame that puts `head` at the origin and turns `fwd` (a head-relative point ahead of the face) to +z about y.
    fn facing(head: [f64; 3], fwd: [f64; 3]) -> impl Fn([f64; 3]) -> [f64; 3] {
        let d = [fwd[0] - head[0], fwd[2] - head[2]];
        let yaw = d[0].atan2(d[1]); // angle from +z toward +x
        let (s, c) = yaw.sin_cos();
        move |p: [f64; 3]| { let x = p[0] - head[0]; let z = p[2] - head[2]; [x * c - z * s, p[1] - head[1], x * s + z * c] }
    }

    /// Debug (0.196.1): KH2's whole head (skin flat, hair textured) carried by the hair's fit (scale, width, shift; no face fit)
    /// next to Re:CoM's final head and hair at rest, both sides, the same camera. `out/kh2/sbs/head_rest.png`: KH2 right side |
    /// ours | KH2 left side | ours.
    #[test]
    fn kh2_head_vs_ours_at_rest() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let mdl0 = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        let e = arc.find("PLAYER/0101/PL0101.RTM").unwrap();
        let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
        let src = crate::kh2_hair::Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap();
        let mdl1 = src.prepare_mdl(&mdl0).unwrap();
        let swap = with_hair(&mdl1, &rtm, &hd, &RIKU_0101_KH2, &src).unwrap();
        let (p3, ratlas) = dt::test_view(&swap);
        let katlas = crate::texture::decode_dds(&src.dds).unwrap();
        let sk = RecomSkeleton::parse(&swap.mdl).unwrap();
        let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
        let w = sk.rest_world();
        let h = w[sk.find("head").unwrap()].t;
        let model = kh2_formats_lite::mdlx::parse(&src.mdlx).unwrap();
        let pose = kh2_formats_lite::skeleton::bind_pose(&model.bones);
        let k = kh2_formats_lite::skeleton::point(&pose[HEAD], [0.0; 3]);
        let (sc, wd, sh) = (crate::kh2_hair::SCALE, crate::kh2_hair::WIDTH, crate::kh2_hair::SHIFT);
        let frame = |p: [f64; 3]| [wd * sc * (p[0] - k[0]) + sh[0], sc * (p[1] - k[1]) + sh[1] - 0.04, sc * (p[2] - k[2]) + sh[2] + 0.03];
        let mut kt = vec![];
        for (gi, gr) in model.groups.iter().enumerate() {
            for t in &gr.triangles {
                let p = t.clone().map(|v| frame(kh2_formats_lite::skeleton::point(&pose[v.bone as usize], v.position.map(|x| x as f64))));
                if p.iter().any(|q| q[1] < -0.2) { continue; }
                kt.push(Tri { p, uv: Some(t.clone().map(|v| [v.uv[0] as f64, v.uv[1] as f64])), hair: gi == crate::kh2_hair::HAIR_GROUP });
            }
        }
        let keep = |n: &str| n == "head" || n.contains("hair") || n.starts_with("neck") || n.starts_with("chest");
        let mut rt = vec![];
        for t in dt::collect(&p3) {
            if !t.v.iter().all(|v| keep(&names[v.bone()])) { continue; }
            let p = t.v.clone().map(|v| { let q = w[v.bone()].point(v.pos().map(|x| x as f64)); [q[0] - h[0], q[1] - h[1] - 0.04, q[2] - h[2] + 0.03] });
            rt.push(Tri { p, uv: Some(t.v.clone().map(|v| v.uv().map_or([0.0; 2], |u| [u[0] as f64, u[1] as f64]))), hair: t.tex == 1 && t.v.iter().any(|v| names[v.bone()].contains("hair") || names[v.bone()] == "head") });
        }
        // dumps for the ear landmarks: KH2's head skin (non-hair groups, head bone) and Re:CoM's face, head-relative, metres
        {
            let mut txt = String::new();
            for (gi, gr) in model.groups.iter().enumerate() {
                if gi == crate::kh2_hair::HAIR_GROUP { continue; }
                for t in &gr.triangles {
                    if !t.iter().all(|v| v.bone as usize == HEAD) { continue; }
                    let p = t.clone().map(|v| { let q = kh2_formats_lite::skeleton::point(&pose[v.bone as usize], v.position.map(|x| x as f64)); [wd * sc * (q[0] - k[0]) + sh[0], sc * (q[1] - k[1]) + sh[1], sc * (q[2] - k[2]) + sh[2]] });
                    txt += &format!("k {}\n", p.iter().flatten().map(|x| format!("{x:.5}")).collect::<Vec<_>>().join(" "));
                }
            }
            std::fs::write(dt::out_dir().join("kh2").join("sbs").join("kh2_skin.txt"), txt).unwrap();
        }
        // the Re:CoM face is on the same atlas as the hair: texture all of ours; KH2's face flat
        for t in rt.iter_mut() { t.hair = true; }
        let mirror = |ts: &[Tri]| -> Vec<Tri> { ts.iter().map(|t| Tri { p: t.p.map(|q| [-q[0], q[1], q[2]]), uv: t.uv, hair: t.hair }).collect() };
        let ppm = 1300.0;
        let panels = [raster(&kt, &katlas, false, ppm), raster(&rt, &ratlas, false, ppm), raster(&mirror(&kt), &katlas, false, ppm), raster(&mirror(&rt), &ratlas, false, ppm)];
        let mut img = vec![0u8; 4 * S * S * 3];
        for (kk, pnl) in panels.iter().enumerate() {
            for y in 0..S { img[3 * (y * 4 * S + kk * S)..3 * (y * 4 * S + kk * S + S)].copy_from_slice(&pnl[3 * y * S..3 * (y + 1) * S]); }
        }
        let out = dt::out_dir().join("kh2").join("sbs");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("head_rest.png"), dt::png(4 * S, S, &img)).unwrap();
    }

    /// 0.198.6 (model-reviewer, second no-go: the owner's "Cabelo levanta DEMAIS!" reduced, not gone; "make this a number"): the mean
    /// angle (deg) between the long back hair (vertices 5 cm or more behind the head bone at rest) and the back's downward line
    /// (neck base → chest), seen from the head bone, in KH2's own walk / run (emulator) and in ours for several extra turns.
    /// 0.198.6: the convention of `head_rest` (bake's): for a bone whose parent is `head`, world.t = local.t · H + head.t?
    #[test]
    fn head_rest_convention() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let sk = RecomSkeleton::parse(&install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap()).unwrap();
        let w = sk.rest_world();
        let h = sk.find("head").unwrap();
        let (hr, sc) = unscale(&w[h].r);
        eprintln!("head scale {sc:?}");
        for (i, b) in sk.bones.iter().enumerate() {
            if b.parent as usize != h { continue; }
            let a = vm(b.local.t, &w[h].r);
            let c = vm(b.local.t, &tr(&w[h].r));
            eprintln!("{}: world {:?} | l·H+t {:?} | l·Hᵀ+t {:?}", b.name, w[i].t.map(|x| (x * 1000.0).round() / 1000.0), [a[0] + w[h].t[0], a[1] + w[h].t[1], a[2] + w[h].t[2]].map(|x| (x * 1000.0).round() / 1000.0), [c[0] + w[h].t[0], c[1] + w[h].t[1], c[2] + w[h].t[2]].map(|x| (x * 1000.0).round() / 1000.0));
            break;
        }
        eprintln!("H (unscaled) rows: {:?}", hr.map(|r| r.map(|x| (x * 1000.0).round() / 1000.0)));
    }

    thread_local! { static REL0: std::cell::RefCell<HashMap<String, [[f64; 3]; 3]>> = std::cell::RefCell::new(HashMap::new()); }

    /// Ours' tips: hair vertices this far or more from the head bone (metres; KH2's side uses 60 % of its farthest).
    const TIP_DIST: f64 = 0.18;

    #[test]
    fn back_lock_angle() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        if !std::path::Path::new(&format!("{}/slot8.f32", crate::kh2_motion::tests::emu_full())).exists() { return; }
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let mdl0 = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        let ann0 = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.ANN").unwrap();
        let e = arc.find("PLAYER/0101/PL0101.RTM").unwrap();
        let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
        let src = crate::kh2_hair::Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap();
        let mdl1 = src.prepare_mdl(&mdl0).unwrap();
        let swap = with_hair(&mdl1, &rtm, &hd, &RIKU_0101_KH2, &src).unwrap();
        let (p3, _) = dt::test_view(&swap);
        let sk = RecomSkeleton::parse(&swap.mdl).unwrap();
        let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
        let rhead = sk.find("head").unwrap();
        let (rn, rc) = (sk.find("neck3").unwrap(), sk.find("chest1").unwrap());
        let rest = sk.rest_world();
        let hz = rest[rhead].t[2];
        let model = kh2_formats_lite::mdlx::parse(&src.mdlx).unwrap();
        let pose0 = kh2_formats_lite::skeleton::bind_pose(&model.bones);
        let bind = rel(&pose0);
        let rig = src.rig(&mdl0).unwrap().unwrap();
        let hrest = Aff { r: unscale(&rest[rhead].r).0, t: rest[rhead].t };
        // KH2's chain under the head: 98 head, 97, 96, 95, 94 (run_lean); the back's line is 95 → 94
        let kc = 36usize; // KH2's chest (run_lean's chain: 98 head, 97, 96, 95, 94, 36; 95 and 94 share an origin)
        let ang = |a: [f64; 3], b: [f64; 3]| { let d = a[0] * b[0] + a[1] * b[1] + a[2] * b[2]; let la = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt(); let lb = (b[0] * b[0] + b[1] * b[1] + b[2] * b[2]).sqrt(); (d / la / lb).clamp(-1.0, 1.0).acos().to_degrees() };
        let sub = |a: [f64; 3], b: [f64; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        // KH2 hair vertices behind the head (rest, carried into Re:CoM's frame), in their bone's bind frame
        let mut kv: Vec<(usize, [f64; 3])> = vec![];
        if let Some(gr) = model.groups.get(crate::kh2_hair::HAIR_GROUP) {
            for t in &gr.triangles { for v in t {
                let b = v.bone as usize;
                let l = rigid_inv(&aff(&pose0[b])).point(kh2_formats_lite::skeleton::point(&pose0[b], v.position.map(|x| x as f64)));
                let wr = hrest.point(rig.carry.point(bind[b].point(l)));
                if wr[2] < hz - 0.05 { kv.push((b, l)); }
            } }
        }
        let motions = src.motions().unwrap();
        // 0.198.6: per lock (each back-rooted hair bone): root (its vertex nearest the head bone at rest) → tip (its vertex farthest from
        // the root), in the bone's own frame; the same KH2 vertices on both sides
        let verts = vertices_by_bone(&model, crate::kh2_hair::HAIR_GROUP);
        let rest_hr = rig.head_rel(&rel(&pose0));
        let head_rest = unscale(&rest[rhead].r).0;
        let locks: Vec<(usize, usize, [f64; 3], [f64; 3], [f64; 3], [f64; 3])> = rest_hr
            .iter()
            .filter(|(rb, a)| vm(a.t, &head_rest)[2] <= TILT_BEHIND || rig.split.iter().any(|x| x.0 == *rb))
            .filter_map(|(rb, a)| {
                let kb = rig.kh2_of(*rb)?;
                let vs = verts.get(&kb)?;
                let inv = rigid_inv(a);
                let ls: Vec<([f64; 3], [f64; 3])> = vs.iter().map(|v| (*v, inv.point(rig.carry.point(bind[kb].point(*v))))).collect();
                let root = ls.iter().min_by(|x, y| crate::math::dist(a.point(x.1), [0.0; 3]).total_cmp(&crate::math::dist(a.point(y.1), [0.0; 3])))?;
                let tip = ls.iter().max_by(|x, y| crate::math::dist(x.1, root.1).total_cmp(&crate::math::dist(y.1, root.1)))?;
                Some((*rb, kb, root.1, tip.1, root.0, tip.0))
            })
            .collect();
        eprintln!("locks: {}", locks.len());
        for (clip, kslot) in [(2usize, KH2_WALK), (3, KH2_RUN)] {
            let m = &motions[&kslot];
            let raw = std::fs::read(format!("{}/slot{kslot}.f32", crate::kh2_motion::tests::emu_full())).unwrap();
            let fl: Vec<f32> = raw.chunks_exact(4).map(|x| f32::from_le_bytes(x.try_into().unwrap())).collect();
            let samples = (m.frame_end * m.keys_per_frame()).round() as usize + 1;
            let per = (fl.len() / 16) / samples;
            let mut ka = vec![];
            for ki in 0..samples {
                let km = |b: usize| -> Aff { let o = (ki * per + b) * 16; aff(&core::array::from_fn(|r| core::array::from_fn(|q| fl[o + r * 4 + q] as f64))) };
                let (h, down) = (km(HEAD).t, sub(km(kc).t, km(HEAD).t));
                // the tips: vertices at least 60 % as far from the head as the farthest
                let d: Vec<([f64; 3], f64)> = kv.iter().map(|(b, l)| { let q = sub(km(*b).point(*l), h); (q, (q[0] * q[0] + q[1] * q[1] + q[2] * q[2]).sqrt()) }).collect();
                let far = d.iter().map(|x| x.1).fold(0.0, f64::max);
                for (q, l) in &d { if *l >= 0.6 * far { ka.push(ang(*q, down)); } }
            }
            ka.retain(|x| x.is_finite());
            let kmean = ka.iter().sum::<f64>() / ka.len() as f64;
            let mut kl = vec![];
            for ki in 0..samples {
                let km = |b: usize| -> Aff { let o = (ki * per + b) * 16; aff(&core::array::from_fn(|r| core::array::from_fn(|q| fl[o + r * 4 + q] as f64))) };
                let down = sub(km(kc).t, km(HEAD).t);
                for l in &locks { kl.push(ang(sub(km(l.1).point(l.5), km(l.1).point(l.4)), down)); }
            }
            kl.retain(|x| x.is_finite());
            eprintln!("KH2 slot {kslot}: locks (root → tip) vs the back's line, mean {:.1} deg", kl.iter().sum::<f64>() / kl.len() as f64);
            eprintln!("KH2 slot {kslot}: back hair vs the back's line, mean {kmean:.1} deg ({} vertices)", kv.len());
            for tilt in [0.0, -24.0, -36.0, -50.0, -60.0, -70.0, -80.0, -90.0] {
                TILT_OVERRIDE.with(|o| o.set(Some((kslot, tilt))));
                let ann = crate::ann::Ann::parse(&src.bake_ann(&ann0, &mdl0).unwrap().unwrap()).unwrap();
                TILT_OVERRIDE.with(|o| o.set(None));
                let c = ann.slots[clip].clip.as_ref().unwrap();
                let mut ra = vec![];
                let mut per_bone: std::collections::BTreeMap<String, (usize, f64)> = Default::default();
                for f in 0..c.frames as usize {
                    let w = crate::kh2_hair::tests::posed_world(&sk, c, f as f64);
                    let (h, down) = (w[rhead].t, sub(w[rc].t, w[rhead].t));
                    let _ = rn;
                    for t in dt::collect(&p3) {
                        if t.tex != 1 { continue; }
                        for v in &t.v {
                            if !names[v.bone()].contains("hair") { continue; }
                            let pr = rest[v.bone()].point(v.pos().map(|x| x as f64));
                            if pr[2] >= hz - 0.05 { continue; }
                            let q = sub(w[v.bone()].point(v.pos().map(|x| x as f64)), h);
                            if (q[0] * q[0] + q[1] * q[1] + q[2] * q[2]).sqrt() < TIP_DIST { continue; }
                            let x = ang(q, down);
                            ra.push(x);
                            let e = per_bone.entry(names[v.bone()].clone()).or_insert((0, 0.0)); e.0 += 1; e.1 += x;
                        }
                    }
                }
                let mut rl = vec![];
                for f in 0..c.frames as usize {
                    let w = crate::kh2_hair::tests::posed_world(&sk, c, f as f64);
                    let down = sub(w[rc].t, w[rhead].t);
                    for l in &locks { rl.push(ang(sub(w[l.0].point(l.3), w[l.0].point(l.2)), down)); }
                }
                eprintln!("  ours clip {clip}, extra turn {tilt}: tips {:.1} deg, locks {:.1} deg", ra.iter().sum::<f64>() / ra.len() as f64, rl.iter().sum::<f64>() / rl.len() as f64);
                {
                    let w0 = crate::kh2_hair::tests::posed_world(&sk, c, 0.0);
                    let hh = unscale(&w0[rhead].r).0;
                    let rel_of = |n: &str| { let b = sk.find(n).unwrap(); m3(&unscale(&w0[b].r).0, &tr(&hh)) };
                    REL0.with(|r0| {
                        let mut r0 = r0.borrow_mut();
                        for n in ["k2_hair_104c", "k2_hair_101c", "l_hair3", "b_hair3"] {
                            let r = rel_of(n);
                            if tilt == 0.0 { r0.insert(n.to_string(), r); continue; }
                            let Some(a) = r0.get(n) else { continue };
                            // the turn from tilt 0 in the head's frame: d = a^T · r (row vectors: x·a^T·r)
                            let d = m3(&tr(a), &r);
                            let tr_ = d[0][0] + d[1][1] + d[2][2];
                            let angle = ((tr_ - 1.0) / 2.0).clamp(-1.0, 1.0).acos().to_degrees();
                            let axis = [d[1][2] - d[2][1], d[2][0] - d[0][2], d[0][1] - d[1][0]];
                            eprintln!("    {n}: turned {angle:.1} deg about head-frame axis {:?}", axis.map(|v| (v * 100.0).round() / 100.0));
                        }
                    });
                }
                if std::env::var("PER_BONE").is_ok() && (tilt == 0.0 || tilt == -60.0) { for (n, (k, sum)) in &per_bone { eprintln!("    {n}: {k} corners, mean {:.1}", sum / *k as f64); } }
            }
        }
    }

    #[test]
    fn kh2_and_recom_side_by_side() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        if !std::path::Path::new(&format!("{}/slot0.f32", crate::kh2_motion::tests::emu_full())).exists() { return; }
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let mdl0 = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        let ann0 = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.ANN").unwrap();
        let e = arc.find("PLAYER/0101/PL0101.RTM").unwrap();
        let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
        let src = crate::kh2_hair::Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap();
        let mdl1 = src.prepare_mdl(&mdl0).unwrap();
        let swap = with_hair(&mdl1, &rtm, &hd, &RIKU_0101_KH2, &src).unwrap();
        let ann = crate::ann::Ann::parse(&src.bake_ann(&ann0, &mdl0).unwrap().unwrap()).unwrap();
        let (p3, ratlas) = dt::test_view(&swap);
        let katlas = crate::texture::decode_dds(&src.dds).unwrap();
        let sk = RecomSkeleton::parse(&swap.mdl).unwrap();
        let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
        let rhead = sk.find("head").unwrap();
        let model = kh2_formats_lite::mdlx::parse(&src.mdlx).unwrap();
        let pose0 = kh2_formats_lite::skeleton::bind_pose(&model.bones);
        let motions = src.motions().unwrap();
        let s = crate::kh2_hair::SCALE;
        let keep = |n: &str| n == "head" || n.contains("hair") || n.starts_with("neck") || n.starts_with("chest") || n == "waist" || n.ends_with("collar") || n.ends_with("uparm") || n.ends_with("foarm");
        let base = dt::out_dir().join("kh2").join("sbs");
        for (clip, kslot, n) in [(1usize, KH2_IDLE, 16usize), (2, KH2_WALK, 12), (3, KH2_RUN, 12), (10, KH2_JUMP, 10), (12, KH2_FALL, 8)] {
            let Some(c) = ann.slots[clip].clip.as_ref() else { continue };
            let m = &motions[&kslot];
            let raw = std::fs::read(format!("{}/slot{kslot}.f32", crate::kh2_motion::tests::emu_full())).unwrap();
            let fl: Vec<f32> = raw.chunks_exact(4).map(|x| f32::from_le_bytes(x.try_into().unwrap())).collect();
            let samples = (m.frame_end * m.keys_per_frame()).round() as usize + 1;
            let per = (fl.len() / 16) / samples;
            let looped = c.looped() && c.loop_start == 1;
            let d = base.join(format!("c{clip}"));
            std::fs::create_dir_all(&d).unwrap();
            let frames = c.frames as usize;
            for i in 0..n {
                let f = i * frames / n;
                // KH2 side: the emulator at the same KH2 time the bake used
                let kt = kh2_time(m, looped, frames, f) * m.keys_per_frame();
                let ki = (kt.round() as usize).min(samples - 1);
                let km: Vec<kh2_formats_lite::skeleton::Mat> = (0..model.bones.len()).map(|b| { let o = (ki * per + b) * 16; core::array::from_fn(|r| core::array::from_fn(|q| fl[o + r * 4 + q] as f64)) }).collect();
                let kw = |b: usize, l: [f64; 3]| aff(&km[b]).point(l);
                let khead = aff(&km[HEAD]).t;
                let kfwd = kw(HEAD, rigid_inv(&aff(&pose0[HEAD])).point(kh2_formats_lite::skeleton::point(&pose0[118], [0.0; 3])));
                let kf = facing(khead, kfwd);
                let mut kt_tris = vec![];
                for (gi, gr) in model.groups.iter().enumerate() {
                    for t in &gr.triangles {
                        let p = t.clone().map(|v| { let wb = kh2_formats_lite::skeleton::point(&pose0[v.bone as usize], v.position.map(|x| x as f64)); let l = rigid_inv(&aff(&pose0[v.bone as usize])).point(wb); let q = kf(kw(v.bone as usize, l)); [q[0] * s, q[1] * s, q[2] * s] });
                        if p.iter().any(|q| q[1] < -0.45) { continue; }
                        kt_tris.push(Tri { p, uv: Some(t.clone().map(|v| [v.uv[0] as f64, v.uv[1] as f64])), hair: gi == crate::kh2_hair::HAIR_GROUP });
                    }
                }
                // Re:CoM side
                let w = crate::kh2_hair::tests::posed_world(&sk, c, f as f64);
                let fh = sk.find("f_hair").unwrap();
                let rfwd = w[rhead].point(rigid_inv(&sk.rest_world()[rhead]).point([sk.rest_world()[rhead].t[0], sk.rest_world()[rhead].t[1], sk.rest_world()[rhead].t[2] + 0.1]));
                let _ = fh;
                let rf = facing(w[rhead].t, rfwd);
                let mut rt_tris = vec![];
                for t in dt::collect(&p3) {
                    if !t.v.iter().all(|v| keep(&names[v.bone()])) { continue; }
                    let p = t.v.clone().map(|v| rf(w[v.bone()].point(v.pos().map(|x| x as f64))));
                    rt_tris.push(Tri { p, uv: Some(t.v.clone().map(|v| v.uv().map_or([0.0; 2], |u| [u[0] as f64, u[1] as f64]))), hair: t.tex == 1 });
                }
                let ppm = 700.0;
                let panels = [raster(&kt_tris, &katlas, false, ppm), raster(&rt_tris, &ratlas, false, ppm), raster(&kt_tris, &katlas, true, ppm), raster(&rt_tris, &ratlas, true, ppm)];
                let mut img = vec![0u8; 4 * S * S * 3];
                for (k, pnl) in panels.iter().enumerate() {
                    for y in 0..S { img[3 * (y * 4 * S + k * S)..3 * (y * 4 * S + k * S + S)].copy_from_slice(&pnl[3 * y * S..3 * (y + 1) * S]); }
                }
                std::fs::write(d.join(format!("f{i:02}.png")), dt::png(4 * S, S, &img)).unwrap();
            }
        }
    }
}


#[cfg(test)]
mod face_tests {
    use super::*;
    use crate::ddd_hair::{with_hair, HairSource, RIKU_0101_KH2};
    use crate::install;

    /// 0.196.4 (owner's install, `RECOM_GAME`): no KH2 hair triangle cuts Re:CoM's skin anywhere on the head, at rest: none has a corner
    /// more than 5 mm outside (or with no skin under it) and another more than 2 mm behind a kept face triangle. 0.196.3 had 63 of them
    /// (the `b_hair2` lock through the +x ear, the `l_hair4` front lock into the cheek). `kh2_hair_sits_on_the_face` skips the
    /// directions behind the head's middle, where the ear is; this does not.
    #[test]
    fn kh2_hair_does_not_cross_the_skin() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let mdl0 = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        let e = arc.find("PLAYER/0101/PL0101.RTM").unwrap();
        let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
        let src = crate::kh2_hair::Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap();
        let mdl1 = src.prepare_mdl(&mdl0).unwrap();
        let _ = with_hair(&mdl1, &rtm, &hd, &RIKU_0101_KH2, &src).unwrap();
        let sk = crate::mdl::RecomSkeleton::parse(&mdl1).unwrap();
        let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
        let world = sk.rest_world();
        let head = world[sk.find("head").unwrap()].t;
        let c = [head[0] + crate::ddd_hair::HEAD_CENTRE[0], head[1] + crate::ddd_hair::HEAD_CENTRE[1], head[2] + crate::ddd_hair::HEAD_CENTRE[2]];
        let face = src.face_for_tests();
        let hair = src.hair(&names, &world).unwrap();
        let (mut crossing, mut deepest) = (0usize, 0.0f64);
        for t in &hair.tris {
            let ds = t.v.each_ref().map(|v| crate::kh2_hair::skin_depth(v.pos, &face, c).map(|(d, _)| d));
            let tip = ds.iter().any(|d| d.is_none_or(|d| d > crate::kh2_hair::SKIN_TIP));
            let inside = ds.iter().any(|d| d.is_some_and(|d| d < -crate::kh2_hair::SKIN_INSIDE));
            if tip && inside {
                crossing += 1;
            }
            for d in ds.into_iter().flatten() {
                deepest = deepest.max(-d);
            }
        }
        eprintln!("hair triangles crossing the skin: {crossing}; deepest vertex behind a skin triangle: {:.1} mm", deepest * 1000.0);
        assert_eq!(crossing, 0, "{crossing} hair triangles cut the skin");
    }

    /// 0.196.1 (owner's install, `RECOM_GAME`): the KH2 hair sits on Re:CoM's face at rest. At most 5 hair vertices of the head's
    /// front half lie more than 3 mm inside the face (0.195.4: 1-3, within 2.6 mm); on -x, where KH2's model has no ear, no face
    /// vertex of the ear region stands out past the flattening.
    #[test]
    fn kh2_hair_sits_on_the_face() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let mdl0 = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        let e = arc.find("PLAYER/0101/PL0101.RTM").unwrap();
        let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
        let src = crate::kh2_hair::Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap();
        let mdl1 = src.prepare_mdl(&mdl0).unwrap();
        let swap = with_hair(&mdl1, &rtm, &hd, &RIKU_0101_KH2, &src).unwrap();
        let sk = crate::mdl::RecomSkeleton::parse(&mdl1).unwrap();
        let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
        let world = sk.rest_world();
        let head = world[sk.find("head").unwrap()].t;
        let face = src.face_for_tests();
        // 0.196.3: the kept head triangles in the +x ear window are written twice (both facing flags); the count is the face's
        // triangles in that window, and small (the window is the ear's, not the head's).
        let in_window = face
            .iter()
            .filter(|t| crate::kh2_hair::in_ear_two_sided_window([0, 1, 2].map(|k| (t[0][k] + t[1][k] + t[2][k]) / 3.0), head))
            .count();
        // 0.196.4: the plain proxy's head triangles around the ears are dropped (9 within 25 mm of the +x ear top alone at 0.196.3's swap).
        eprintln!("plain proxy head triangles dropped: {}; plain copies of head-bound KH2 hair triangles left out at the ears: {}", swap.report.proxy_dropped, swap.report.plain_hair_dropped);
        // 0.196.4: the flat copy of the head-bound hair triangle 2711 (and its neighbours) at the +x ear top was the flat polygon the owner saw.
        assert!((1..=120).contains(&swap.report.plain_hair_dropped), "{} plain hair copies dropped", swap.report.plain_hair_dropped);
        assert!((4..=120).contains(&swap.report.proxy_dropped), "{} proxy triangles dropped", swap.report.proxy_dropped);
        eprintln!("skirt triangles between the head's open edges and the hair: {}", swap.report.skirt);
        assert!((1..=80).contains(&swap.report.skirt), "{} skirt triangles", swap.report.skirt);
        eprintln!("kept head triangles written twice: {} (face triangles in the +x ear window: {in_window})", swap.report.face_two_sided);
        assert_eq!(swap.report.face_two_sided, in_window);
        assert!((1..=60).contains(&in_window), "{in_window} triangles in the ear window");
        let hair = src.hair(&names, &world).unwrap();
        let c = [head[0] + crate::ddd_hair::HEAD_CENTRE[0], head[1] + crate::ddd_hair::HEAD_CENTRE[1], head[2] + crate::ddd_hair::HEAD_CENTRE[2]];
        let mut inside = vec![];
        for t in &hair.tris {
            for v in &t.v {
                let r = [v.pos[0] - c[0], v.pos[1] - c[1], v.pos[2] - c[2]];
                let l = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
                let d = [r[0] / l, r[1] / l, r[2] / l];
                if d[2] < -0.2 { continue; }
                if let Some(f) = crate::kh2_hair::outermost(c, d, &face) {
                    if l < f - 0.003 { inside.push(f - l); }
                }
            }
        }
        inside.sort_by(|a, b| b.total_cmp(a));
        inside.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        eprintln!("hair vertices more than 3 mm inside the face: {} (deepest {:?})", inside.len(), inside.first());
        assert!(inside.len() <= 5, "{} hair vertices inside the face", inside.len());
        let out: Vec<[f64; 3]> = face.iter().flatten().copied().filter(|p| {
            let r = [p[0] - head[0], p[1] - head[1], p[2] - head[2]];
            r[0] < -crate::kh2_hair::EAR_FLAT_X - 1e-4 && (crate::kh2_hair::EAR_FLAT_Y[0]..=crate::kh2_hair::EAR_FLAT_Y[1]).contains(&r[1]) && (crate::kh2_hair::EAR_FLAT_Z[0]..=crate::kh2_hair::EAR_FLAT_Z[1]).contains(&r[2])
        }).collect();
        assert!(out.is_empty(), "the -x ear stands out: {out:?}");
    }
}
