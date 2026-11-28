//! 0.198.21 (fork owner, 2026-11-27: "a animação do riku quando termina uma luta ta meio estranha na calça com o traje normal";
//! on being told it is Re:CoM's own: "quero corrigir sim"): Riku's level-up screen motion with his pant panels where his model
//! has them.
//!
//! Re:CoM's `PLAYER/4042/PL4042.ANN` (one clip, `p002_042`, 90 frames, 102 bones; the level-up screen `0x14010EF10` plays it as
//! group 14042, Checked (decomp) in the workspace's `knowledge/ReCoM/recom-skeleton-limits.md`) stores four children of each
//! femur in another order than Riku's `PL0101.MDL` / `PL0111.MDL`: the channels in slot `l_belt3` are `l_belt4`'s, `l_belt4`'s
//! are `l_cuffs1`'s, `l_cuffs1`'s are `l_cuffs2`'s and `l_cuffs2`'s are `l_belt3`'s (the same on the right). Each slot's
//! translation is a constant equal, within 1 mm, to the rest offset of the bone it belongs to, while every clip of
//! `PL0101.ANN` carries each bone's own rest offset. Checked (files). So the game draws the outer blue panel (`*_cuffs1`) on
//! the other side of the thigh, the inner one (`*_cuffs2`) at the belt strap's place (the top edge jutting off the upper
//! thigh) and the straps off the hip: the panels end mid-thigh as stiff flaps over the dark trousers.
//!
//! [`fix_femur_children`] puts each child's channels back on the bone whose rest offset they carry: whole channel blocks move,
//! no value is changed or made up, so each panel plays the keys Re:CoM authored for it.
//!
//! 0.198.22 (fork owner, on the two found and left: `PL4062` with the same pants order, and the left hand's fingers 4 / 5 swapped
//! in both: "corrige ai"): `PL4062` too, and the left hand's finger chains `l_fing4c/b/a` and `l_fing5c/b/a` back on their bones
//! (each slot's constant translation is the other finger's rest offset, Checked (files)).

use crate::ann::{Ann, Channel, Clip};
use crate::mdl::RecomSkeleton;
use crate::Result;

/// The packs fixed: Riku's level-up screen motion and, 0.198.22 (fork owner: "corrige ai"), `PL4062`'s `p002_020`, which has
/// the same order (its screen: not identified).
pub const FIXED_PACKS: [u16; 2] = [4042, 4062];

/// The model whose skeleton the fix reads (Riku's default attire; `PL0111` has the same bones and offsets).
pub const MODEL_GROUP: u16 = 101;

/// A translation within this of a bone's rest offset is that bone's (metres).
const SAME_T: f64 = 1e-3;

/// One move: (bone index that gets the channels, clip slot they came from).
pub type Move = (usize, usize);

fn const_t(c: &Clip, b: usize) -> Option<[f64; 3]> {
    let ch = c.chans();
    let v = |k: usize| match &c.channels[b * ch + k] {
        Channel::Const { bits, .. } => Some(f32::from_bits(*bits) as f64),
        Channel::Curve(_) => None,
    };
    Some([v(0)?, v(1)?, v(2)?])
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// The parents whose children Re:CoM's level-up packs store out of order: the femurs (pant panels and straps) and, 0.198.22,
/// the left hand (fingers 4 and 5); the right hand is checked too (none found).
const PARENTS: [&str; 4] = ["l_femur", "r_femur", "l_hand", "r_hand"];

/// The single-child chain under `b` in `sk` (its child, grandchild, ... while each has exactly one child).
fn chain(sk: &RecomSkeleton, b: usize) -> Vec<usize> {
    let mut out = vec![];
    let mut at = b;
    loop {
        let kids: Vec<usize> = (0..sk.bones.len()).filter(|&j| sk.bones[j].parent >= 0 && sk.bones[j].parent as usize == at).collect();
        if kids.len() != 1 {
            return out;
        }
        out.push(kids[0]);
        at = kids[0];
    }
}

/// The moves that put `c`'s children of [`PARENTS`] back in `sk`'s order: for each child slot whose translation is a constant
/// equal to another child's rest offset (and not its own), that child gets the slot's channels, and so does each bone of its
/// single-child chain (a finger's joints) from the same depth of the slot's chain, when the chains have the same length and
/// every such slot's constant translation is its new bone's rest offset. Only a complete permutation per parent is returned
/// (every moved bone gets exactly one slot and every moved slot is given), else none for that parent.
pub fn femur_moves(c: &Clip, sk: &RecomSkeleton) -> Vec<Move> {
    if c.bones as usize != sk.bones.len() {
        return vec![];
    }
    let mut out = vec![];
    for par in PARENTS {
        let Some(f) = sk.find(par) else { continue };
        let kids: Vec<usize> = (0..sk.bones.len()).filter(|&j| sk.bones[j].parent >= 0 && sk.bones[j].parent as usize == f).collect();
        let mut moves: Vec<Move> = vec![];
        let mut ok = true;
        for &slot in &kids {
            let Some(t) = const_t(c, slot) else { continue };
            if dist(t, sk.bones[slot].local.t) < SAME_T {
                continue;
            }
            let Some(&bone) = kids.iter().find(|&&j| j != slot && dist(t, sk.bones[j].local.t) < SAME_T) else { continue };
            moves.push((bone, slot));
            let (cb, cs) = (chain(sk, bone), chain(sk, slot));
            if cb.len() != cs.len() {
                ok = false;
                continue;
            }
            for (&b, &s) in cb.iter().zip(&cs) {
                match const_t(c, s) {
                    Some(t) if dist(t, sk.bones[b].local.t) < SAME_T => moves.push((b, s)),
                    _ => ok = false,
                }
            }
        }
        let mut to: Vec<usize> = moves.iter().map(|m| m.0).collect();
        let mut from: Vec<usize> = moves.iter().map(|m| m.1).collect();
        to.sort_unstable();
        from.sort_unstable();
        let unique = to.windows(2).all(|w| w[0] != w[1]);
        if ok && !moves.is_empty() && unique && to == from {
            out.extend(moves);
        }
    }
    out
}

/// Applies [`femur_moves`] to every clip of `ann` with `sk`'s bone count; returns the moves per slot (empty: nothing changed).
pub fn fix_femur_children(ann: &mut Ann, sk: &RecomSkeleton) -> Vec<(usize, Vec<Move>)> {
    let mut done = vec![];
    for (i, s) in ann.slots.iter_mut().enumerate() {
        let Some(c) = s.clip.as_mut() else { continue };
        let moves = femur_moves(c, sk);
        if moves.is_empty() {
            continue;
        }
        let ch = c.chans();
        let old = c.channels.clone();
        for &(bone, slot) in &moves {
            c.channels[bone * ch..(bone + 1) * ch].clone_from_slice(&old[slot * ch..(slot + 1) * ch]);
        }
        done.push((i, moves));
    }
    done
}

/// Re:CoM's pack `group` with [`fix_femur_children`] on [`MODEL_GROUP`]'s skeleton: `Some(bytes)` when it changed.
pub fn fixed_pack(arc: &kh1_formats::egs::Archive, keys: &kh1_formats::egs::KeyTable, group: u16) -> Result<Option<Vec<u8>>> {
    let sk = RecomSkeleton::parse(&crate::install::read_recom(arc, keys, &crate::riku_skin::path(MODEL_GROUP, "MDL"))?)?;
    fix_bytes(&crate::install::read_recom(arc, keys, &crate::riku_skin::path(group, "ANN"))?, &sk)
}

/// [`fix_femur_children`] on a pack's bytes: `Some(bytes)` when it changed.
pub fn fix_bytes(ann: &[u8], sk: &RecomSkeleton) -> Result<Option<Vec<u8>>> {
    let mut a = Ann::parse(ann)?;
    if fix_femur_children(&mut a, sk).is_empty() {
        return Ok(None);
    }
    Ok(Some(a.to_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::install;
    use crate::math::{rot_xyz, Aff};
    use crate::mdl::{split_parts, MdlPart};
    use crate::riku_skin::path;

    fn game() -> Option<std::path::PathBuf> {
        std::env::var_os("RECOM_GAME").map(Into::into)
    }

    fn posed(sk: &RecomSkeleton, c: &Clip, t: f64) -> Vec<Aff> {
        let mut w: Vec<Aff> = Vec::with_capacity(sk.bones.len());
        for (i, b) in sk.bones.iter().enumerate() {
            let s = c.sample(i, t);
            let mut r = rot_xyz(s.r[0], s.r[1], s.r[2]);
            for (row, sc) in r.iter_mut().zip(s.s) {
                for v in row.iter_mut() {
                    *v *= sc;
                }
            }
            let local = Aff { r, t: s.t };
            w.push(if b.parent < 0 { local } else { local.mul(&w[b.parent as usize]) });
        }
        w
    }

    /// A world point in `bone`'s frame.
    fn to_local(w: &Aff, p: [f64; 3]) -> [f64; 3] {
        let q = [p[0] - w.t[0], p[1] - w.t[1], p[2] - w.t[2]];
        [0, 1, 2].map(|i| (q[0] * w.r[i][0] + q[1] * w.r[i][1] + q[2] * w.r[i][2]) / (w.r[i][0].powi(2) + w.r[i][1].powi(2) + w.r[i][2].powi(2)))
    }

    /// Per panel bone: (deepest crossing into the thigh, widest stand-off from the battle stance's hang), metres, over a clip.
    /// Crossing: a panel vertex's distance from the femur's axis less the thigh mesh's radius within 4 cm along the bone and
    /// 0.5 rad around it (negative = inside the thigh). Stand-off: how far a panel vertex sits, in the femur's frame, from where
    /// it hangs in Riku's battle stance `p002_031` frame 0.
    struct Probe {
        sk: RecomSkeleton,
        verts: Vec<(usize, [f64; 3])>,
        stance: Clip,
    }

    impl Probe {
        fn measure(&self, c: &Clip, bone: usize, frames: std::ops::Range<usize>) -> (f64, f64) {
            let fem = self.sk.bones[bone].parent as usize;
            let fv: Vec<[f64; 3]> = self.verts.iter().filter(|v| v.0 == fem).map(|v| v.1).collect();
            let pv: Vec<[f64; 3]> = self.verts.iter().filter(|v| v.0 == bone).map(|v| v.1).collect();
            let ws = posed(&self.sk, &self.stance, 0.0);
            let hang: Vec<[f64; 3]> = pv.iter().map(|p| to_local(&ws[fem], ws[bone].point(*p))).collect();
            let (mut deep, mut wide) = (f64::MAX, 0.0f64);
            for f in frames {
                let w = posed(&self.sk, c, f as f64);
                for (k, p) in pv.iter().enumerate() {
                    let q = to_local(&w[fem], w[bone].point(*p));
                    let (r, ang) = (q[1].hypot(q[2]), q[2].atan2(q[1]));
                    let fr = fv
                        .iter()
                        .filter(|v| {
                            let mut da = (v[2].atan2(v[1]) - ang).abs();
                            if da > std::f64::consts::PI {
                                da = 2.0 * std::f64::consts::PI - da;
                            }
                            (v[0] - q[0]).abs() < 0.04 && da < 0.5
                        })
                        .map(|v| v[1].hypot(v[2]))
                        .fold(0.0, f64::max);
                    if fr > 0.0 {
                        deep = deep.min(r - fr);
                    }
                    wide = wide.max(dist(q, hang[k]));
                }
            }
            (deep, wide)
        }
    }

    const PANELS: [&str; 4] = ["belt3", "belt4", "cuffs1", "cuffs2"];

    /// 0.198.21 (owner's install, `RECOM_GAME`): each fixed pack ([`FIXED_PACKS`]: `PL4042.ANN`, 0.198.22 `PL4062.ANN`) (1) moves
    /// only the eight femur children's channel blocks and, 0.198.22, the left hand's finger 4 / 5 chains (six blocks), each to the
    /// bone whose rest offset it carries, every other channel byte-identical, and every other Riku event pack is left alone;
    /// (2) no panel crosses into the thigh (≤ 0.5 cm) in any frame; (3) no panel stands off its battle-stance hang further than
    /// the same bone does anywhere in `PL0101.ANN`'s own clips (Re:CoM's own envelope); Re:CoM's packs fail (2) and (3).
    #[test]
    fn the_level_up_pants_are_in_the_models_order() {
        let Some(g) = game() else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let mdl = install::read_recom(&arc, &keys, &path(MODEL_GROUP, "MDL")).unwrap();
        let sk = RecomSkeleton::parse(&mdl).unwrap();
        let name = |b: usize| sk.bones[b].name.clone();
        let mut want: Vec<(String, String)> = ["l", "r"]
            .iter()
            .flat_map(|s| [("belt3", "cuffs2"), ("belt4", "belt3"), ("cuffs1", "belt4"), ("cuffs2", "cuffs1")].map(|(b, f)| (format!("{s}_{b}"), format!("{s}_{f}"))))
            .chain(["a", "b", "c"].iter().flat_map(|j| [(format!("l_fing4{j}"), format!("l_fing5{j}")), (format!("l_fing5{j}"), format!("l_fing4{j}"))]))
            .collect();
        want.sort();
        // (1) the moves, and nothing else
        let mut pairs = vec![];
        for g in FIXED_PACKS {
            let raw = install::read_recom(&arc, &keys, &path(g, "ANN")).unwrap();
            let van = Ann::parse(&raw).unwrap();
            assert_eq!(van.to_bytes(), raw, "PL{g:04} round-trips byte-identical");
            let fixed = Ann::parse(&fixed_pack(&arc, &keys, g).unwrap().expect("changes")).unwrap();
            assert_eq!(van.slots.len(), fixed.slots.len());
            for (sv, sf) in van.slots.iter().zip(&fixed.slots) {
                let (Some(c0), Some(c1)) = (sv.clip.as_ref(), sf.clip.as_ref()) else { continue };
                assert_eq!((c0.name_str(), c0.frames, c0.loop_start, c0.flags, c0.bones), (c1.name_str(), c1.frames, c1.loop_start, c1.flags, c1.bones));
                if c0.bones != 102 {
                    continue;
                }
                let moves = femur_moves(c0, &sk);
                let mut got: Vec<(String, String)> = moves.iter().map(|&(b, s)| (name(b), name(s))).collect();
                got.sort();
                println!("PL{g:04} {}: {} frames, {} moves", c0.name_str(), c0.frames, moves.len());
                assert_eq!(got, want, "PL{g:04} {}: each bone gets the slot holding its rest offset", c0.name_str());
                let ch = c0.chans();
                for b in 0..102 {
                    let from = moves.iter().find(|m| m.0 == b).map_or(b, |m| m.1);
                    assert!(c1.channels[b * ch..(b + 1) * ch] == c0.channels[from * ch..(from + 1) * ch], "bone {b} {}", name(b));
                }
                pairs.push((g, c0.clone(), c1.clone()));
            }
        }
        assert_eq!(pairs.len(), 2, "one 102-bone clip in each pack");
        for p in crate::riku_skin::EVENT_PACKS.into_iter().filter(|p| !FIXED_PACKS.contains(p)) {
            assert!(fixed_pack(&arc, &keys, p).unwrap().is_none(), "PL{p:04} is left as Re:CoM's");
        }
        assert!(fix_bytes(&install::read_recom(&arc, &keys, &path(101, "ANN")).unwrap(), &sk).unwrap().is_none(), "Riku's own pack is in the model's order");
        // (2) and (3)
        let parts = split_parts(&mdl).unwrap();
        let p3 = MdlPart::parse_bytes(&parts[3]).unwrap();
        let verts = p3.list_a.iter().chain(p3.list_b.iter().flatten()).flat_map(|pk| pk.verts.iter().map(|v| (v.bone(), v.pos().map(|x| x as f64)))).collect();
        let own = Ann::parse(&install::read_recom(&arc, &keys, &path(101, "ANN")).unwrap()).unwrap();
        let stance = own.slots[own.find("p002_031").unwrap()].clip.clone().unwrap();
        let probe = Probe { sk, verts, stance };
        let own_clips: Vec<&Clip> = own.slots.iter().filter_map(|s| s.clip.as_ref()).filter(|c| c.bones == 102).collect();
        for (g, c0, c1) in &pairs {
            let n = c0.frames as usize;
            let mut van_bad = 0;
            for side in ["l", "r"] {
                for pb in PANELS {
                    let b = probe.sk.find(&format!("{side}_{pb}")).unwrap();
                    let envelope = own_clips.iter().map(|c| probe.measure(c, b, 0..c.frames as usize).1).fold(0.0, f64::max);
                    let (deep, wide) = probe.measure(c1, b, 0..n);
                    let (vd, vw) = probe.measure(c0, b, 0..n);
                    println!("PL{g:04} {side}_{pb}: fixed clearance {:.1} cm, stand-off {:.1} cm | Re:CoM's {:.1} / {:.1} cm | PL0101's stand-off envelope {:.1} cm", deep * 100.0, wide * 100.0, vd * 100.0, vw * 100.0, envelope * 100.0);
                    assert!(deep >= -0.005, "PL{g:04} {side}_{pb}: crosses the thigh by {:.1} cm", -deep * 100.0);
                    assert!(wide <= envelope, "PL{g:04} {side}_{pb}: stands off {:.1} cm (PL0101's clips: {:.1} cm)", wide * 100.0, envelope * 100.0);
                    van_bad += (vd < -0.005 || vw > envelope) as usize;
                }
            }
            assert!(van_bad >= 6, "PL{g:04}: Re:CoM's pack fails the checks ({van_bad} of 8 panel bones)");
        }
    }
}
