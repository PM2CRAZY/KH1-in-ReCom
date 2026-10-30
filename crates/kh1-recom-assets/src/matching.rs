//! Which KH1 motions Re:CoM already has (owner decision 2026-10-30: "just use RECOM's default
//! animations, and KH1 animations for anything that isnt included in ReCOM").
//!
//! About 50 of Re:CoM's Sora clips are KH1 motions that Square Enix baked at 30 fps onto the
//! same skeleton (`docs/motion-conversion.md`). This module finds them from the player's own
//! files at build time, deterministically:
//!
//! - every KH1 entry is posed with `kh1-anim` at `t = 2f` (Re:CoM frame `f` at 30 fps = KH1
//!   frame `2f` at 60 Hz) and collapsed onto Re:CoM's bones exactly as the converter does
//!   ([`Converter::targets`], with the entry's own request-map root flag);
//! - it is compared with every Re:CoM clip whose length fits (`|2F − N| ≤` [`FRAME_SLACK`]) on
//!   all 135 bones, **only at the clip's key frames** (Re:CoM's clips are key-reduced; at a key
//!   the bake is exact up to u16 rounding, between keys the reduction error shows): the local
//!   rotation (geodesic angle, where all three rotation channels are keyed or constant) and the
//!   local translation (per keyed channel);
//! - per bone, the worst error over the compared frames. A bone **agrees** when it is within
//!   [`BONE_ROT_DEG`] and [`BONE_POS_MM`]. The translations of `Root` (0) and `ex_0010_sk` (1)
//!   are left out of the agreement: where Re:CoM keeps the travel (in `Root`, in the hips, or
//!   as a constant offset between the two) is its own per-clip choice (e.g. KH1 entries 14, 15,
//!   16, 74 against p001_015/016/017/045). Their worst error is reported as
//!   [`PairScore::root_split_mm`].
//!
//! Rules (all Checked (files): test `f_…` prints the margins):
//! - (a) **bake**: every bone agrees. Its match error ([`PairScore::match_error`], the worst
//!   bone rotation) is ≤ 0.74° (the spine-aim residual motions of `kh1-posing.md` §8; most are
//!   ≤ 0.03°, entry 98 0.019°).
//! - (b) **KH1 motion with Re:CoM-only edits**: at most [`MAX_EDITED_BONES`] bones disagree,
//!   the entry has no bake, and the clip is not a bake of another KH1 entry. These are KH1's
//!   motion as Re:CoM plays it (`kh1-posing.md` §8: bones KH1 leaves unkeyed are animated in
//!   Re:CoM), so they count as present. Found: entries 0 (keychain), 18, 20–23, 26 (arms,
//!   legs, hips height), 57 (legs), 65, 66 (jacket tails): 4–16 bones.
//! - Anything else is not a match. The closest non-match is 17 disagreeing bones (KH1 76/77/78
//!   against each other's bakes 155–157, which rule (b) excludes as owned), then 26 (86/87
//!   against 84's bake 150), then ≥ 74 for every other pair.

#![allow(clippy::needless_range_loop)]

use kh1_formats::mset::Motion;

use crate::ann::{Ann, Channel, Clip};
use crate::convert::{Converter, Target, WeaponRig};
use crate::math::{angle_deg, m3, rot_xyz, tr, vm, I3, M3};
use crate::mdl::RecomSkeleton;

/// A Re:CoM clip of `F` frames (30 fps) is compared with a KH1 motion of `N` frames (60 Hz)
/// when `|2F − N| ≤ FRAME_SLACK` (Square Enix's bakes have `F = N/2`, or `N/2 + 1` for three
/// play-once clips; Checked (files)).
pub const FRAME_SLACK: i64 = 4;

/// A bone agrees when its worst local rotation error is within this (degrees). True bakes:
/// ≤ 0.74°; Re:CoM's edits: ≥ 1.0°.
pub const BONE_ROT_DEG: f64 = 0.9;

/// A bone agrees when its worst local translation error is within this (millimetres). True
/// bakes: ≤ 0.002 mm outside bones 0/1.
pub const BONE_POS_MM: f64 = 1.0;

/// At most this many bones may disagree for rule (b).
pub const MAX_EDITED_BONES: usize = 20;

/// Bones whose translation is left out of the agreement (`Root`, `ex_0010_sk`).
pub const ROOT_SPLIT_BONES: usize = 2;

/// One KH1 entry against one Re:CoM clip.
#[derive(Clone, Debug, PartialEq)]
pub struct PairScore {
    /// Re:CoM slot.
    pub slot: usize,
    /// Per bone: worst local rotation error at the clip's key frames (degrees).
    pub rot_deg: Vec<f64>,
    /// Per bone: worst local translation error at keyed channels (millimetres).
    pub pos_mm: Vec<f64>,
    /// Clip frames compared.
    pub frames: usize,
}

impl PairScore {
    /// Bones that disagree.
    pub fn disagreeing(&self) -> Vec<usize> {
        (0..self.rot_deg.len()).filter(|&b| self.rot_deg[b] > BONE_ROT_DEG || (b >= ROOT_SPLIT_BONES && self.pos_mm[b] > BONE_POS_MM)).collect()
    }

    /// The match error: worst rotation error over the agreeing bones (degrees).
    pub fn match_error(&self) -> f64 {
        (0..self.rot_deg.len()).filter(|&b| self.rot_deg[b] <= BONE_ROT_DEG).map(|b| self.rot_deg[b]).fold(0.0, f64::max)
    }

    /// Worst translation error of `Root` / `ex_0010_sk` (mm): how differently Re:CoM splits
    /// the travel.
    pub fn root_split_mm(&self) -> f64 {
        self.pos_mm.iter().take(ROOT_SPLIT_BONES).copied().fold(0.0, f64::max)
    }

    /// Worst translation error of the other bones (mm).
    pub fn body_pos_mm(&self) -> f64 {
        self.pos_mm.iter().skip(ROOT_SPLIT_BONES).copied().fold(0.0, f64::max)
    }

    /// Worst rotation error over all bones (degrees).
    pub fn max_rot(&self) -> f64 {
        self.rot_deg.iter().copied().fold(0.0, f64::max)
    }
}

/// How a Re:CoM clip holds a KH1 motion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchKind {
    /// Every bone agrees (rule (a)).
    Bake,
    /// A few bones are Re:CoM's own edit (rule (b)).
    Edited,
}

impl MatchKind {
    /// Manifest text.
    pub fn as_str(self) -> &'static str {
        match self {
            MatchKind::Bake => "bake",
            MatchKind::Edited => "edited",
        }
    }
}

/// A clip decoded for comparison: per frame, per bone, the local rotation (`None` unless all
/// rotation channels are keyed or constant there) and the translation channels keyed there.
#[derive(Clone, Debug)]
pub struct DecodedClip {
    /// Slot.
    pub slot: usize,
    /// Frames.
    pub frames: usize,
    rot: Vec<Vec<Option<M3>>>,
    pos: Vec<Vec<[Option<f64>; 3]>>,
}

impl DecodedClip {
    /// Decodes the clip at its key frames.
    pub fn new(slot: usize, c: &Clip) -> DecodedClip {
        let nb = c.bones as usize;
        let ch = c.chans();
        let f = c.frames as usize;
        let keyed: Vec<Vec<bool>> = (0..nb * ch)
            .map(|i| match &c.channels[i] {
                Channel::Const { .. } => vec![true; f],
                Channel::Curve(cv) => {
                    let mut v = vec![false; f];
                    for &k in &cv.keys {
                        if let Some(x) = v.get_mut(k as usize) {
                            *x = true;
                        }
                    }
                    v
                }
            })
            .collect();
        let mut rot = Vec::with_capacity(f);
        let mut pos = Vec::with_capacity(f);
        for fr in 0..f {
            let mut r = Vec::with_capacity(nb);
            let mut p = Vec::with_capacity(nb);
            for b in 0..nb {
                let s = c.sample(b, fr as f64);
                r.push((3..6).all(|k| keyed[b * ch + k][fr]).then(|| rot_xyz(s.r[0], s.r[1], s.r[2])));
                p.push([0, 1, 2].map(|k| keyed[b * ch + k][fr].then_some(s.t[k])));
            }
            rot.push(r);
            pos.push(p);
        }
        DecodedClip { slot, frames: f, rot, pos }
    }
}

/// Every non-empty clip of a pack, decoded.
pub fn decode_pack(ann: &Ann) -> Vec<DecodedClip> {
    ann.slots.iter().enumerate().filter_map(|(i, s)| s.clip.as_ref().map(|c| DecodedClip::new(i, c))).collect()
}

/// Locals (rotation, translation) of every bone from targets by `FUN_14023EB00`'s rule (as
/// [`crate::convert::locals_for`], without the Euler step); `root_parent` is bone 0's external
/// parent (the Keyblade's `WeaponR`), else the origin.
pub fn local_rt(skel: &RecomSkeleton, tg: &[Target], root_parent: Option<&Target>) -> Vec<(M3, [f64; 3])> {
    skel.bones
        .iter()
        .enumerate()
        .map(|(b, bone)| {
            let me = &tg[b];
            let (pu, ps, pp) = match (bone.parent, root_parent) {
                (p, _) if p >= 0 => (tg[p as usize].u, tg[p as usize].s, tg[p as usize].p),
                (_, Some(r)) => (r.u, r.s, r.p),
                _ => (I3, [1.0; 3], [0.0; 3]),
            };
            let d = [me.p[0] - pp[0], me.p[1] - pp[1], me.p[2] - pp[2]];
            let tl = vm(d, &tr(&pu));
            (m3(&me.u, &tr(&pu)), [tl[0] / ps[0], tl[1] / ps[1], tl[2] / ps[2]])
        })
        .collect()
}

/// KH1's Sora locals of `m` at `t = 2f`, `f = 0..frames`.
pub fn kh1_locals(conv: &Converter, m: &Motion, root_motion: bool, frames: usize) -> Vec<Vec<(M3, [f64; 3])>> {
    let mut rig = conv.rig();
    rig.start_motion(m);
    (0..frames).map(|f| local_rt(conv.skeleton(), &conv.targets(&mut rig, m, 2.0 * f as f32, root_motion), None)).collect()
}

/// KH1's Keyblade locals of `m` at `t = 2f`, `f = 0..frames` (bone 0 hangs from `WeaponR`).
pub fn kh1_weapon_locals(conv: &Converter, w: &WeaponRig, m: &Motion, frames: usize) -> Vec<Vec<(M3, [f64; 3])>> {
    let mut rig = conv.rig();
    rig.start_motion(m);
    (0..frames)
        .map(|f| {
            rig.pose(m, 2.0 * f as f32);
            let tg = conv.weapon_targets(w, &rig);
            local_rt(&w.skel, &tg, Some(&tg[0]))
        })
        .collect()
}

/// Whether a clip of `clip_frames` (30 fps) can be a bake of a KH1 motion of `n` frames.
pub fn frames_fit(clip_frames: usize, n: u32) -> bool {
    (2 * clip_frames as i64 - n as i64).abs() <= FRAME_SLACK
}

/// Compares KH1's locals with one decoded clip over their common frames.
pub fn score(kh1: &[Vec<(M3, [f64; 3])>], clip: &DecodedClip) -> PairScore {
    let nb = kh1.first().map_or(0, |v| v.len());
    let mut rot_deg = vec![0.0f64; nb];
    let mut pos_mm = vec![0.0f64; nb];
    let frames = clip.frames.min(kh1.len());
    for f in 0..frames {
        for b in 0..nb.min(clip.rot[f].len()) {
            let (rk, tk) = &kh1[f][b];
            if let Some(rc) = &clip.rot[f][b] {
                rot_deg[b] = rot_deg[b].max(angle_deg(&m3(rk, &tr(rc))));
            }
            for k in 0..3 {
                if let Some(v) = clip.pos[f][b][k] {
                    pos_mm[b] = pos_mm[b].max((tk[k] - v).abs() * 1000.0);
                }
            }
        }
    }
    PairScore { slot: clip.slot, rot_deg, pos_mm, frames }
}

/// One KH1 entry to look for.
#[derive(Clone, Copy, Debug)]
pub struct Query<'a> {
    /// KH1 entry.
    pub entry: usize,
    /// Its motion.
    pub motion: &'a Motion,
    /// Its request-map root flag.
    pub root_motion: bool,
}

/// The result for one KH1 entry.
#[derive(Clone, Debug, PartialEq)]
pub struct EntryMatch {
    /// KH1 entry.
    pub entry: usize,
    /// Re:CoM's clip for it, if any.
    pub found: Option<(PairScore, MatchKind)>,
    /// Other clips that also pass the chosen rule (ambiguity; the chosen one has the fewest
    /// disagreeing bones, then the smallest match error, then the lowest slot).
    pub ambiguous: Vec<usize>,
    /// The closest clip that is not chosen (fewest disagreeing bones), for the margin.
    pub runner_up: Option<PairScore>,
}

/// Finds Re:CoM's clip, if any, for every query. Two passes: bakes first (rule (a)); then
/// rule (b) for the entries without a bake, among the clips that are no entry's bake.
pub fn match_all(conv: &Converter, queries: &[Query], clips: &[DecodedClip]) -> Vec<EntryMatch> {
    let scores: Vec<Vec<PairScore>> = queries
        .iter()
        .map(|q| {
            let fit: Vec<&DecodedClip> = clips.iter().filter(|c| frames_fit(c.frames, q.motion.frames)).collect();
            let longest = fit.iter().map(|c| c.frames).max().unwrap_or(0);
            if longest == 0 {
                return Vec::new();
            }
            let kh1 = kh1_locals(conv, q.motion, q.root_motion, longest);
            fit.into_iter().map(|c| score(&kh1, c)).collect()
        })
        .collect();
    let owned: Vec<usize> = scores.iter().flatten().filter(|s| s.disagreeing().is_empty()).map(|s| s.slot).collect();
    let key = |s: &PairScore| (s.disagreeing().len(), s.match_error(), s.slot);
    queries
        .iter()
        .zip(scores)
        .map(|(q, mut ss)| {
            ss.sort_by(|a, b| {
                let (x, y) = (key(a), key(b));
                x.0.cmp(&y.0).then(x.1.total_cmp(&y.1)).then(x.2.cmp(&y.2))
            });
            let bakes: Vec<usize> = (0..ss.len()).filter(|&i| ss[i].disagreeing().is_empty()).collect();
            let (pick, kind, pass): (Option<usize>, MatchKind, Vec<usize>) = if !bakes.is_empty() {
                (Some(bakes[0]), MatchKind::Bake, bakes)
            } else {
                let ed: Vec<usize> = (0..ss.len()).filter(|&i| ss[i].disagreeing().len() <= MAX_EDITED_BONES && !owned.contains(&ss[i].slot)).collect();
                (ed.first().copied(), MatchKind::Edited, ed)
            };
            let ambiguous = pass.iter().skip(1).map(|&i| ss[i].slot).collect();
            let runner_up = (0..ss.len()).find(|&i| Some(i) != pick).map(|i| ss[i].clone());
            let found = pick.map(|i| (ss[i].clone(), kind));
            EntryMatch { entry: q.entry, found, ambiguous, runner_up }
        })
        .collect()
}
