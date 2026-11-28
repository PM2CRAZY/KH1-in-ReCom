//! KH2 Riku's hair on Re:CoM Riku's head (the hair option `riku_hair = kh2`; fork owner, 2026-11-24: "o cabelo começa sim pelo do
//! Riku do KH2"). It needs only the 1.5+2.5 collection (KH2 FM's `obj/P_EH000.mdlx` and its remastered hair atlas), the swap
//! itself is [`crate::ddd_hair::with_hair`].
//!
//! Source (notes: the workspace's `knowledge/KH2/kh2-riku-hair-models.md`): `P_EH000.mdlx`'s group 3 is the hair alone (613
//! triangles, one bone per vertex, 27 bones), drawn with texture 3, whose remastered DDS is the asset `-5.dds` (1024 x 1024).
//! Checked (files, that note).
//!
//! Fit (Assumption until seen in game; the renders in `local/assets/recom/riku-hair/out/kh2/` are the check):
//! - Frame: a KH2 vertex's bind world position `W` becomes `H + s * (W - K)` where `K` is KH2's head bone (98) and `H` Re:CoM's
//!   `head` bone, both at rest; `s` is [`SCALE`], the head-size ratio of the note (0.0086, two heights compared).
//! - Bones: KH2's cap bones (98, 101, 104) follow Re:CoM's `head` (rigid with the skull). Every other KH2 bone takes the Re:CoM
//!   bone (`head` or one with "hair" in its name) nearest to the KH2 bone's own rest position in that frame, so a chain keeps one
//!   Re:CoM bone and its strands move together. This is the 0.188.0 look, the one the owner liked ("ficou MUITO bom o cabelo do
//!   KH2"); 0.188.1 to 0.191.0 tried a rigid head binding, one sway bone, neck-bone bands and KH2's own motion baked into the
//!   hair bones, and the owner preferred this one ("a primeira versão é a melhor até agora"): Stage 363 went back to it.
//! - Width: [`WIDTH`] stretches x a little, Re:CoM's head is wider than KH2's at the temples.

use std::collections::HashMap;

use kh2_formats_lite::{mdlx, skeleton};

use crate::ddd_hair::{HairSource, SrcHair, SrcTri, SrcVert, HEAD_BONE};
use crate::math::Aff;
use crate::texture::decode_dds;
use crate::{bad, Error, Result};

/// KH2's model file of Riku (kh2_sixth). Checked (files).
pub const RIKU_MDLX: &str = "obj/P_EH000.mdlx";
/// The group that holds the hair. Checked (files).
pub const HAIR_GROUP: usize = 3;
/// The hair atlas: the remastered DDS of PS2 texture 3. Checked (files).
pub const ATLAS_ASSET: &str = "-5.dds";
/// KH2's head bone and the cap bones that follow Re:CoM's head. Checked (files: 752 + 183 + 98 of 1839 corners).
pub const HEAD_BONE_KH2: usize = 98;
/// Cap bones of the hair (the rigid skull part).
pub const CAP_BONES: [usize; 3] = [98, 101, 104];
/// KH2 units to Re:CoM metres (the note: 0.0086, hair width +-16.5 against +-0.142).
pub const SCALE: f64 = 0.0092;
/// Extra width factor (x only): Re:CoM's head is wider than KH2's at the temples.
pub const WIDTH: f64 = 1.1;
/// Offset added after the head alignment, in Re:CoM metres (x, y, z), fitted on the renders (see [`SHIFT`]). z was 0.012 until 0.196.6;
/// 0.196.7 (fork owner: "eu acho que o encaixe do cabelo deveria ser um pouco mais pra trás pq ainda to vendo o gap") 0.006: the hair 6 mm further back;
/// 0.196.8 ("da pra ir mais para trás") 0.0: 6 mm more, 12 mm back from 0.196.6's;
/// 0.196.9 ("vai mais uns 24 pra tras") -0.024: 24 mm more, 36 mm back from 0.196.6's;
/// 0.196.10 ("teste ficou escrota move 50 pra frente") 0.026: 50 mm forward again, 14 mm in front of 0.196.6's;
/// 0.196.11 ("tira o deslocamento deixa em 0 e desloca para baixo uns 10") [0, -0.010, 0]: no shift to the front, 10 mm down;
/// 0.196.12 ("desloca 5 pra cima") [0, -0.005, 0]: 5 mm up again;
/// 0.196.13 ("sobe 3") [0, -0.002, 0]: 3 mm up again;
/// 0.196.14 ("pelo visto deslocar não é a estratégia, os vãos ainda aparecem... pode zerar") [0, 0, 0]: no shift.
pub const SHIFT: [f64; 3] = [0.0, 0.0, 0.0];

/// The face fit (0.195.4; fork owner on 0.195.3: "umas das mechas que marquei estão entrando na bochecha, e ainda aparece ta estranho
/// a marcação que fiz", with a picture of KH2's PS2 Riku: the ear shows, the front locks before it, the long hair behind). For each
/// direction from the head's centre, the outermost surface of Re:CoM's face and of KH2's (its head-bone skin, carried by the same
/// fit as the hair) are found by ray; a hair vertex moves along its direction by the difference, averaged over the directions
/// within [`FIT_SIGMA_DEG`] and faded where only one face exists (Re:CoM's back of the head is gone with his hair), held within
/// [`FIT_MIN`]..[`FIT_MAX`] (metres). Replaces 0.195.3's ear push.
pub const FIT_SIGMA_DEG: f64 = 10.0;
/// See [`FIT_SIGMA_DEG`].
pub const FIT_MIN: f64 = -0.004;
/// See [`FIT_SIGMA_DEG`].
pub const FIT_MAX: f64 = 0.025;
/// Angular step of the sampled directions (degrees).
pub const FIT_STEP_DEG: f64 = 3.0;

/// Outermost hit of the ray from `c` along unit `d` with `tris` (distance), if any.
pub fn outermost(c: [f64; 3], d: [f64; 3], tris: &[[[f64; 3]; 3]]) -> Option<f64> {
    let sub = |a: [f64; 3], b: [f64; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let cross = |a: [f64; 3], b: [f64; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let mut best: Option<f64> = None;
    for t in tris {
        let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
        let p = cross(d, e2);
        let det = dot(e1, p);
        if det.abs() < 1e-12 {
            continue;
        }
        let s = sub(c, t[0]);
        let u = dot(s, p) / det;
        if !(0.0..=1.0).contains(&u) {
            continue;
        }
        let q = cross(s, e1);
        let v = dot(d, q) / det;
        if v < 0.0 || u + v > 1.0 {
            continue;
        }
        let dist = dot(e2, q) / det;
        if dist > 0.0 && best.is_none_or(|b| dist > b) {
            best = Some(dist);
        }
    }
    best
}

/// The face fit: sampled directions with Re:CoM's minus KH2's face radius.
#[derive(Clone, Debug, Default)]
pub struct FaceFit {
    /// The head's centre (world, rest).
    pub c: [f64; 3],
    /// `(direction, difference)`.
    pub samples: Vec<([f64; 3], f64)>,
}

impl FaceFit {
    /// From the two faces (world rest triangles) about centre `c`.
    pub fn new(c: [f64; 3], recom: &[[[f64; 3]; 3]], kh2: &[[[f64; 3]; 3]]) -> FaceFit {
        let mut samples = vec![];
        let step = FIT_STEP_DEG.to_radians();
        let mut el = -70f64.to_radians();
        while el <= 80f64.to_radians() {
            let n = ((2.0 * std::f64::consts::PI * el.cos()) / step).ceil().max(1.0) as usize;
            for i in 0..n {
                let az = i as f64 * 2.0 * std::f64::consts::PI / n as f64;
                let d = [el.cos() * az.sin(), el.sin(), el.cos() * az.cos()];
                if let (Some(a), Some(b)) = (outermost(c, d, recom), outermost(c, d, kh2)) {
                    samples.push((d, a - b));
                }
            }
            el += step;
        }
        FaceFit { c, samples }
    }

    /// How far a point moves along its direction from the centre.
    pub fn offset(&self, p: [f64; 3]) -> f64 {
        let r = [p[0] - self.c[0], p[1] - self.c[1], p[2] - self.c[2]];
        let l = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
        if l < 1e-9 || self.samples.is_empty() {
            return 0.0;
        }
        let d = [r[0] / l, r[1] / l, r[2] / l];
        let s2 = 2.0 * FIT_SIGMA_DEG.to_radians().powi(2);
        let (mut wsum, mut vsum, mut full) = (0.0, 0.0, 0.0);
        // the weight a fully covered neighbourhood gets (samples spaced FIT_STEP_DEG): its sum, to fade where coverage is partial
        let cell = FIT_STEP_DEG.to_radians().powi(2);
        for (q, v) in &self.samples {
            let a = (d[0] * q[0] + d[1] * q[1] + d[2] * q[2]).clamp(-1.0, 1.0).acos();
            let w = (-a * a / s2).exp();
            wsum += w;
            vsum += w * v;
        }
        full += std::f64::consts::PI * s2 / cell;
        if wsum < 1e-9 {
            return 0.0;
        }
        let cover = (wsum / full).min(1.0);
        (vsum / wsum * cover).clamp(FIT_MIN, FIT_MAX)
    }

    /// The point moved.
    pub fn apply(&self, p: [f64; 3]) -> [f64; 3] {
        let o = self.offset(p);
        if o == 0.0 {
            return p;
        }
        let r = [p[0] - self.c[0], p[1] - self.c[1], p[2] - self.c[2]];
        let l = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
        [p[0] + r[0] / l * o, p[1] + r[1] / l * o, p[2] + r[2] / l * o]
    }
}

/// The ears (0.196.1; fork owner on 0.196.0: "não resolveu o problemna da foto que te mandei da orelha e da mexa", with KH2's
/// PS2 model as reference). Checked (files: KH2's head-bone skin carried by the hair's fit, Re:CoM's kept face; both head-relative,
/// metres): KH2's model has an ear on the +x side only (26 skin vertices past |x| 0.085 there, centre y 0.074 z 0.015; on -x its
/// hair covers the side and its skin has none), Re:CoM has both (centre y 0.063 z -0.001). So on -x Re:CoM's ear is flattened
/// under the hair ([`EAR_FLAT_X`]), and on +x the hair around KH2's ear moves by Re:CoM's ear minus KH2's ([`EAR_SHIFT`]) so that
/// the gap KH2's ear sits in lands on Re:CoM's.
pub const EAR_FLAT_X: f64 = 0.085;
/// The ear region flattened on -x (head-relative y, z ranges, metres).
pub const EAR_FLAT_Y: [f64; 2] = [0.0, 0.125];
/// See [`EAR_FLAT_Y`].
pub const EAR_FLAT_Z: [f64; 2] = [-0.09, 0.04];
/// KH2's ear centre on +x (head-relative y, z) and the shift of the hair around it (y, z).
pub const KH2_EAR: [f64; 2] = [0.074, 0.015];
/// See [`KH2_EAR`].
pub const EAR_SHIFT: [f64; 2] = [-0.011, -0.016];
/// The shift's reach (Gaussian sigma over y, z, metres).
pub const EAR_SHIFT_REACH: f64 = 0.035;

/// Hair that crosses Re:CoM's skin (0.196.4; fork owner on 0.196.3, two screenshots: "uma mecha ta passando por dentro da orelha";
/// "essa [mecha] aqui entrando na bochecha"). Checked (files: the 0.196.3 swap, hair triangles against the kept face triangles,
/// rest and idle frames 0, 25, 50, 75): `FaceFit` pushes a vertex along its direction from the head's centre and never asks whether
/// it lies behind a local flap; the lock `b_hair2` (2649 / 2650) is rooted 15-17 mm inside the head under the +x ear and its tip
/// stands 55-78 mm outside, so it cuts the ear, and the front lock `l_hair4` runs 1-3 mm into the cheek; 54-63 hair triangles per
/// frame have a corner more than 2 mm inside a skin triangle and another more than 5 mm outside. A vertex of such a triangle that
/// lies behind a kept face triangle (its projection inside it) by at most [`SKIN_REACH`] is lifted along that triangle's outward
/// normal to [`SKIN_MARGIN`] in front of it. Triangles wholly inside the head (hidden scalp hair) stay.
pub const SKIN_REACH: f64 = 0.030;
/// See [`SKIN_REACH`]: where a lifted vertex ends up, in front of the skin (metres).
pub const SKIN_MARGIN: f64 = 0.003;
/// A corner this far behind the skin counts as inside (metres).
pub const SKIN_INSIDE: f64 = 0.002;
/// A corner this far in front of the skin (or with no skin under it) counts as a tip outside (metres).
pub const SKIN_TIP: f64 = 0.005;
/// The most passes of [`lift_out_of_skin`].
pub const SKIN_PASSES: usize = 4;

fn v_sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn v_dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn v_cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// The signed distance (outward positive, metres) of `p` from the nearest face triangle whose plane is within [`SKIN_REACH`] and
/// whose projection contains `p`, with that triangle's outward unit normal. `centre` is the head's centre (outward = away from it).
pub fn skin_depth(p: [f64; 3], face: &[[[f64; 3]; 3]], centre: [f64; 3]) -> Option<(f64, [f64; 3])> {
    let mut best: Option<(f64, [f64; 3])> = None;
    for t in face {
        let e1 = v_sub(t[1], t[0]);
        let e2 = v_sub(t[2], t[0]);
        let mut n = v_cross(e1, e2);
        let len = v_dot(n, n).sqrt();
        if len < 1e-12 {
            continue;
        }
        n = [n[0] / len, n[1] / len, n[2] / len];
        let g = [(t[0][0] + t[1][0] + t[2][0]) / 3.0, (t[0][1] + t[1][1] + t[2][1]) / 3.0, (t[0][2] + t[1][2] + t[2][2]) / 3.0];
        if v_dot(n, v_sub(g, centre)) < 0.0 {
            n = [-n[0], -n[1], -n[2]];
        }
        let d = v_dot(n, v_sub(p, t[0]));
        if d.abs() > SKIN_REACH || best.is_some_and(|(b, _)| b.abs() <= d.abs()) {
            continue;
        }
        // barycentric test of the projection
        let w = v_sub(p, t[0]);
        let (d11, d12, d22) = (v_dot(e1, e1), v_dot(e1, e2), v_dot(e2, e2));
        let (dw1, dw2) = (v_dot(w, e1), v_dot(w, e2));
        let den = d11 * d22 - d12 * d12;
        if den.abs() < 1e-18 {
            continue;
        }
        let (u, v) = ((d22 * dw1 - d12 * dw2) / den, (d11 * dw2 - d12 * dw1) / den);
        if u >= 0.0 && v >= 0.0 && u + v <= 1.0 {
            best = Some((d, n));
        }
    }
    best
}

/// Lifts the vertices of hair triangles that cross the skin out of it (see [`SKIN_REACH`]); returns the vertices moved. Decided per
/// vertex position, so triangles that share a vertex keep sharing it. A lifted vertex can end behind the next skin triangle at a fold,
/// so it repeats (at most [`SKIN_PASSES`] times) until no triangle crosses.
pub fn lift_out_of_skin(tris: &mut [crate::ddd_hair::SrcTri], face: &[[[f64; 3]; 3]], centre: [f64; 3]) -> usize {
    use std::collections::HashMap;
    if face.is_empty() {
        return 0;
    }
    let key = |p: [f64; 3]| [p[0].to_bits(), p[1].to_bits(), p[2].to_bits()];
    let mut moved = 0;
    for _ in 0..SKIN_PASSES {
        let mut depth: HashMap<[u64; 3], Option<(f64, [f64; 3])>> = HashMap::new();
        let mut lift: HashMap<[u64; 3], (f64, [f64; 3])> = HashMap::new();
        for t in tris.iter() {
            let ds = t.v.each_ref().map(|v| *depth.entry(key(v.pos)).or_insert_with(|| skin_depth(v.pos, face, centre)));
            let tip = ds.iter().any(|d| d.is_none_or(|(d, _)| d > SKIN_TIP));
            let inside = ds.iter().any(|d| d.is_some_and(|(d, _)| d < -SKIN_INSIDE));
            if tip && inside {
                for (v, d) in t.v.iter().zip(&ds) {
                    if let Some((d, n)) = *d {
                        if d < -SKIN_INSIDE {
                            lift.insert(key(v.pos), (d, n));
                        }
                    }
                }
            }
        }
        if lift.is_empty() {
            break;
        }
        moved += lift.len();
        for t in tris.iter_mut() {
            for v in t.v.iter_mut() {
                if let Some(&(d, n)) = lift.get(&key(v.pos)) {
                    let k = SKIN_MARGIN - d;
                    v.pos = [v.pos[0] + n[0] * k, v.pos[1] + n[1] * k, v.pos[2] + n[2] * k];
                }
            }
        }
    }
    moved
}

/// The skirt's edges: the head's open edges at the ear's height and above, on either side (0.196.15). Head-relative metres: |x| past
/// [`SKIRT_X`], y and z in the ranges. Checked (files): at the +x ear top the head's skin ends 11 mm from the nearest hair triangle.
pub const SKIRT_X: f64 = 0.05;
/// See [`SKIRT_X`].
pub const SKIRT_Y: [f64; 2] = [0.04, 0.17];
/// See [`SKIRT_X`].
pub const SKIRT_Z: [f64; 2] = [-0.08, 0.06];

/// Whether an open head edge with midpoint `p` (world rest; `head` = the head bone's at rest) is in the skirt's window.
pub fn in_skirt_window(p: [f64; 3], head: [f64; 3]) -> bool {
    let r = [p[0] - head[0], p[1] - head[1], p[2] - head[2]];
    r[0].abs() > SKIRT_X && (SKIRT_Y[0]..=SKIRT_Y[1]).contains(&r[1]) && (SKIRT_Z[0]..=SKIRT_Z[1]).contains(&r[2])
}

/// The plain proxy's head triangles around the ears that are dropped (0.196.4; fork owner on 0.196.3, ring, then a diagnostic build that
/// painted the neighbours: "o ponto vazio ainda está lá! [..] só precisa cobrir o ponto que aparece o fundo!"; "logo acima da orelha antes
/// do cabelo começar tem um espaço vazio que da pra ver o interior da parte de traz do cabelo"). Checked (decomp:
/// knowledge/ReCoM/recom-model-culling.md: part 2 is drawn without a facing cull; files: the 0.196.3 swap keeps 211 head-bone-only proxy
/// triangles, 9 within 25 mm of the ear-top corner (centroids 95-112 mm from the head bone along x, on or outside the ear's skin, which
/// is 86-112 mm); the owner's diagnostic screenshot shows an unpainted flat grey-lilac polygon at the ear top, bounded by the painted
/// temple triangle 1412, the ear triangle 1928 and the hair triangle 2708, which none of part 3's triangles is). The proxy hid under
/// Re:CoM's bulky hair; under KH2's thinner hair it shows as a flat polygon. Assumption: that polygon is a proxy triangle (not shown in
/// game). Head-relative metres: |x| past [`PROXY_DROP_X`] (both sides), y and z in the ranges.
pub const PROXY_DROP_X: f64 = 0.06;
/// See [`PROXY_DROP_X`].
pub const PROXY_DROP_Y: [f64; 2] = [0.03, 0.17];
/// See [`PROXY_DROP_X`].
pub const PROXY_DROP_Z: [f64; 2] = [-0.10, 0.10];

/// Whether a proxy head triangle centred at `p` (world rest; `head` = the head bone's at rest) is in the ear window of either side.
pub fn in_proxy_drop_window(p: [f64; 3], head: [f64; 3]) -> bool {
    let r = [p[0] - head[0], p[1] - head[1], p[2] - head[2]];
    r[0].abs() > PROXY_DROP_X && (PROXY_DROP_Y[0]..=PROXY_DROP_Y[1]).contains(&r[1]) && (PROXY_DROP_Z[0]..=PROXY_DROP_Z[1]).contains(&r[2])
}

/// The +x temple / ear window whose kept head triangles are written twice, with both facing flags (0.196.3; fork owner on
/// 0.196.2: "um pequeno ponto que fica transparente mostrando o fundo do cenário" at the top of the ear's gap). Checked (render
/// `local/work/holefind/out/cull_one_sign_ear_f0.png`, the pinhole finder with one facing sign kept): behind that point there
/// is one single-copy head triangle (centroid x +0.09 from the head bone, 'head, head, head', facing flag 0) that a facing
/// cull removes from some views; the hair's own triangles are all written twice. Assumption: that is the point the owner sees
/// (not checked in game). Head-relative metres: x from [`EAR_TWO_SIDED_X`], y and z the ear's [`EAR_FLAT_Y`] / [`EAR_FLAT_Z`].
pub const EAR_TWO_SIDED_X: f64 = 0.06;

/// Whether a head triangle centred at `p` (world rest; `head` = the head bone's at rest) lies in the +x ear window.
pub fn in_ear_two_sided_window(p: [f64; 3], head: [f64; 3]) -> bool {
    let r = [p[0] - head[0], p[1] - head[1], p[2] - head[2]];
    r[0] > EAR_TWO_SIDED_X && (EAR_FLAT_Y[0]..=EAR_FLAT_Y[1]).contains(&r[1]) && (EAR_FLAT_Z[0]..=EAR_FLAT_Z[1]).contains(&r[2])
}

/// A hair rest position with the +x ear gap moved to Re:CoM's ear.
pub fn ear_shift(p: [f64; 3], head: [f64; 3]) -> [f64; 3] {
    let r = [p[0] - head[0], p[1] - head[1], p[2] - head[2]];
    if r[0] < 0.03 {
        return p;
    }
    let d2 = (r[1] - KH2_EAR[0]).powi(2) + (r[2] - KH2_EAR[1]).powi(2);
    let w = (-d2 / (2.0 * EAR_SHIFT_REACH * EAR_SHIFT_REACH)).exp();
    [p[0], p[1] + EAR_SHIFT[0] * w, p[2] + EAR_SHIFT[1] * w]
}

/// The lining (0.197.1, the BBS hair's of 0.194.2, which closed its gap in game: fork owner on 0.194.3, "sumiu o gap"): the hair's
/// triangles over the back and sides of the skull ([`in_lining_window`]) are written again on `head`, pulled [`LINING`] of the way
/// toward the head's centre, same texels. Re:CoM's scalp is gone with his hair and his head ends before the KH2 hair starts (Stage
/// 389); the lining is a skull of hair under it, so a lock that swings away shows hair, not the background.
pub const LINING: f64 = 0.08;
/// Whether a lining corner follows its hair corner's bone instead of `head` (0.197.7; fork owner on 0.197.6 in game, four screenshots
/// with arrows at straight edges across the locks: "algumas mexas ficaram esquisitas com a animação"): the lining rigid on `head`
/// showed over the locks that swing inward (reproduced on the idle's back render). Following its lock it stays 8 % inside it in
/// every frame; the corners at the roots are on `head` already (KH2's cap bones), so the scalp seam stays covered. Tried and not
/// kept: rigid at 20 % (also clean on the renders, but a lock that swings far can still uncover it).
pub const LINING_FOLLOWS: bool = true;
/// The lining's window, head-relative (metres): centroids at or above this height…
pub const LINING_Y: f64 = -0.03;
/// …and behind this depth (the face and fringe in front of it are left alone).
pub const LINING_Z: f64 = 0.06;

/// Whether a hair triangle with centroid `p` (world, rest) is lined; `head` is the head bone's rest origin.
pub fn in_lining_window(p: [f64; 3], head: [f64; 3]) -> bool {
    p[1] - head[1] >= LINING_Y && p[2] - head[2] <= LINING_Z
}

/// The lining's triangles for `tris` (see [`LINING`]), on bone `bone`; a copy with a corner behind Re:CoM's skin (`face`, kept
/// triangles) or within [`SKIN_MARGIN`] of it is left out (it would cut the face or the ears).
pub fn lining(tris: &[SrcTri], head: [f64; 3], centre: [f64; 3], bone: &str, face: &[[[f64; 3]; 3]]) -> Vec<SrcTri> {
    tris.iter()
        .filter(|t| in_lining_window([0, 1, 2].map(|k| (t.v[0].pos[k] + t.v[1].pos[k] + t.v[2].pos[k]) / 3.0), head))
        .map(|t| SrcTri {
            v: t.v.clone().map(|v| SrcVert { pos: [0, 1, 2].map(|k| centre[k] + (1.0 - LINING) * (v.pos[k] - centre[k])), uv: v.uv, bone: if LINING_FOLLOWS { v.bone.clone() } else { bone.to_string() } }),
            island: t.island,
            cont: false,
            lining: true,
        })
        .filter(|t| t.v.iter().all(|v| skin_depth(v.pos, face, centre).is_none_or(|(d, _)| d > SKIN_MARGIN)))
        .collect()
}

/// The nape (0.198.2; fork owner on 0.198.1's hair copy: "tampo mas ta um pouco estranho e ta estático tbm", a screenshot from a
/// low camera). Checked (files: the swapped model's open edges): Re:CoM's head ends at the back of the skull base (open `head` /
/// `neck1` edges 0-4 cm under the head bone, 3-7 cm behind it) and the next surface is the collar (`chest1`, about 8 cm lower):
/// Re:CoM's own hair covered the nape, and the KH2 hair lifts off it in the run. Each open head edge in [`in_nape_window`] gets a
/// curtain of skin down to [`NAPE_BOTTOM`] (the swap's [`crate::ddd_hair::HairSource::nape_curtain`]): its top corners are the edge's
/// own (their bones, their texel), its bottom corners straight under them on `neck3` (the neck's base, at the collar), so it
/// stretches with the neck as skin does. 0.198.1's copy of the hair over the nape (rigid on `neck3`) is gone.
/// 0.198.3: not used (the KH2 source gives no curtain): in game it showed as a dark strip over the collar (fork owner: "vazou pelo
/// pescoço"), reproduced in the textured camera checks.
pub const NAPE_BOTTOM: f64 = -0.095;
/// See [`NAPE_BOTTOM`]: the open edges with their midpoint within this height range (head-relative, metres) …
pub const NAPE_Y: [f64; 2] = [-0.03, 0.06];
/// … behind this depth …
pub const NAPE_Z: f64 = -0.02;
/// … and within this half-width.
pub const NAPE_X: f64 = 0.09;
/// The neck's base bone.
pub const NAPE_BONE: &str = "neck3";

/// 0.198.6 (fork owner on 0.198.5: "eu testei andando e corrento com a camera baixa", the walk's nape: neck skin and the hair's
/// lining seen from below; then "ele dando no-go tem que prosseguir para o que o agente apontou"): the curtain again, on the hair's
/// texels (the nearest hair vertex's) and ending inside the collar, not over it (0.198.2's skin curtain crossed it: "vazou pelo
/// pescoço"). Checked (files, rest): the vest's top behind the neck is 4.5 cm under the head bone, on `chest1`. Its bottom corners
/// are 1 cm under that, pulled [`NAPE_HAIR_TUCK`] toward `chest1`'s origin, on `chest1`, so the vest covers them and the strip
/// stretches as the neck bends. Assumption: the 1 cm and the tuck, from the renders.
pub const NAPE_HAIR_BOTTOM: f64 = -0.055;
/// See [`NAPE_HAIR_BOTTOM`].
pub const NAPE_HAIR_BONE: &str = "chest1";
/// See [`NAPE_HAIR_BOTTOM`] (metres).
pub const NAPE_HAIR_TUCK: f64 = 0.015;
/// The curtain's half-width (metres; 9 cm (0.198.2's [`NAPE_X`]) and 5 cm showed it beside the neck from the front).
pub const NAPE_HAIR_X: f64 = 0.03;

/// Whether an open head edge with midpoint `p` (world, rest) gets the nape curtain; `head` is the head bone's rest origin.
pub fn in_nape_window(p: [f64; 3], head: [f64; 3]) -> bool {
    let r = [p[0] - head[0], p[1] - head[1], p[2] - head[2]];
    (NAPE_Y[0]..NAPE_Y[1]).contains(&r[1]) && r[2] < NAPE_Z && r[0].abs() < NAPE_X
}

/// KH2 Riku's motion set (0.195.0: the hair's own motion). Checked (files: kh2_sixth).
pub const RIKU_MSET: &str = "obj/P_EH000.mset";

/// KH2's `P_EH000.mdlx` and its hair atlas.
pub struct Kh2HairSource {
    /// The `.mdlx`.
    pub mdlx: Vec<u8>,
    /// `-5.dds`.
    pub dds: Vec<u8>,
    /// `P_EH000.mset` (none: the hair follows Re:CoM's own hair bones as in 0.188.0).
    pub mset: Option<Vec<u8>>,
    /// The rig, made from Re:CoM's original skeleton by [`HairSource::prepare_mdl`].
    /// 0.198.7: one rig per Re:CoM skeleton (key: its bone names), and the one last asked for. Until 0.198.7 a single rig was kept
    /// from the first model the source saw: the asset step builds Riku's groups first, so the Dark Form skin (another skeleton, 128
    /// bones) got Riku's rig, his bone indices and fit (in game: KH2's hair on the Dark Form in long straight sticks; the tests, with a
    /// fresh source, did not show it).
    rig: std::cell::RefCell<Vec<(Vec<String>, &'static crate::kh2_hair_rig::Rig)>>,
    current: std::cell::Cell<Option<&'static crate::kh2_hair_rig::Rig>>,
    /// Re:CoM's face that stays, from [`HairSource::set_face`].
    face: std::cell::RefCell<Vec<[[f64; 3]; 3]>>,
}

impl Kh2HairSource {
    /// From the player's KH2 archives.
    pub fn read(inst: &kh1_formats::egs::Install) -> Result<Kh2HairSource> {
        let (mdlx, assets, _) = inst.read_full(RIKU_MDLX)?;
        let dds = assets.into_iter().find(|a| a.name == ATLAS_ASSET).ok_or_else(|| Error::Format(format!("KH2: {RIKU_MDLX} has no {ATLAS_ASSET}")))?.data;
        let mset = inst.read_full(RIKU_MSET).ok().map(|x| x.0);
        Ok(Kh2HairSource { mdlx, dds, mset, rig: Default::default(), current: Default::default(), face: Default::default() })
    }

    /// From the files themselves.
    pub fn new(mdlx: Vec<u8>, dds: Vec<u8>, mset: Option<Vec<u8>>) -> Kh2HairSource {
        Kh2HairSource { mdlx, dds, mset, rig: Default::default(), current: Default::default(), face: Default::default() }
    }

    /// The face [`HairSource::set_face`] was given (tests).
    pub fn face_for_tests(&self) -> Vec<[[f64; 3]; 3]> {
        self.face.borrow().clone()
    }

    /// The KH2 motions the rig uses, by `.mset` slot.
    pub fn motions(&self) -> Result<HashMap<usize, crate::kh2_motion::Kh2Motion>> {
        use crate::kh2_hair_rig as r;
        let mut out = HashMap::new();
        if let Some(m) = &self.mset {
            // 0.197.5: every KH2 slot `CLIP_MAP` names
            let mut slots: Vec<usize> = std::iter::once(r::KH2_IDLE).chain(r::CLIP_MAP.iter().map(|x| x.1)).collect();
            slots.sort_unstable();
            slots.dedup();
            for s in slots {
                if let Some(x) = crate::kh2_motion::read_slot(m, s)? {
                    out.insert(s, x);
                }
            }
        }
        Ok(out)
    }

    /// The rig for Re:CoM's original model `mdl` (made once).
    pub fn rig(&self, mdl: &[u8]) -> Result<Option<&crate::kh2_hair_rig::Rig>> {
        use crate::kh2_hair_rig as r;
        let key: Vec<String> = crate::mdl::RecomSkeleton::parse(mdl)?.bones.iter().map(|b| b.name.clone()).collect();
        if let Some(&(_, x)) = self.rig.borrow().iter().find(|(k, _)| *k == key) {
            self.current.set(Some(x));
            return Ok(Some(x));
        }
        let motions = self.motions()?;
        if !motions.contains_key(&r::KH2_IDLE) {
            return Ok(None);
        }
        let model = mdlx::parse(&self.mdlx).map_err(|e| Error::Format(format!("KH2 {RIKU_MDLX}: {e}")))?;
        let sk0 = crate::mdl::RecomSkeleton::parse(mdl)?;
        let head = sk0.find(HEAD_BONE).ok_or_else(|| Error::Format("hair: no head bone".into()))?;
        let pose = skeleton::bind_pose(&model.bones);
        let carry = r::Carry { k: r::aff(&pose[HEAD_BONE_KH2]).r, h: crate::math::unscale(&sk0.rest_world()[head].r).0, c: [WIDTH * SCALE, SCALE, SCALE], shift: SHIFT };
        // 0.196.5: the skeleton with the 7 extra KH2 hair bones, so that every KH2 hair bone moves its own vertices
        let sk = r::extend_skeleton(&sk0, head, &model, &carry);
        let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
        let world = sk.rest_world();
        let on: Vec<&crate::kh2_motion::Kh2Motion> = r::CHOOSE_ON.iter().filter_map(|s| motions.get(s)).collect();
        let host = r::hosts(&model, HAIR_GROUP, &on, &r::HAIR_BONES);
        let rig = r::Rig::new(&names, &world, &model, carry, host)?;
        // leaked: a few rigs per asset build (one per skeleton), alive as long as the source
        let rig: &'static crate::kh2_hair_rig::Rig = Box::leak(Box::new(rig));
        self.rig.borrow_mut().push((key, rig));
        self.current.set(Some(rig));
        Ok(Some(rig))
    }
}

fn dist2(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| (a[k] - b[k]).powi(2)).sum()
}

impl HairSource for Kh2HairSource {
    fn set_face(&self, face: &[[[f64; 3]; 3]]) {
        *self.face.borrow_mut() = face.to_vec();
    }

    fn adjust_face(&self, p: [f64; 3], head: [f64; 3]) -> [f64; 3] {
        let r = [p[0] - head[0], p[1] - head[1], p[2] - head[2]];
        if r[0] < -EAR_FLAT_X && (EAR_FLAT_Y[0]..=EAR_FLAT_Y[1]).contains(&r[1]) && (EAR_FLAT_Z[0]..=EAR_FLAT_Z[1]).contains(&r[2]) {
            return [head[0] - EAR_FLAT_X, p[1], p[2]];
        }
        p
    }

    fn drop_plain_hair_copy(&self, t: &crate::ddd_hair::SrcTri, head: [f64; 3]) -> bool {
        let c = [0, 1, 2].map(|k| (t.v[0].pos[k] + t.v[1].pos[k] + t.v[2].pos[k]) / 3.0);
        t.v.iter().all(|v| v.bone == HEAD_BONE) && in_proxy_drop_window(c, head)
    }

    fn skirt_edge(&self, p: [f64; 3], head: [f64; 3]) -> bool {
        in_skirt_window(p, head)
    }

    fn drop_proxy_triangle(&self, p: [f64; 3], head: [f64; 3]) -> bool {
        in_proxy_drop_window(p, head)
    }

    fn two_sided_face(&self, p: [f64; 3], head: [f64; 3]) -> bool {
        in_ear_two_sided_window(p, head)
    }

    fn lining_in_plain(&self) -> bool {
        false
    }

    fn nape_curtain(&self, p: [f64; 3], head: [f64; 3]) -> Option<(f64, &'static str)> {
        (in_nape_window(p, head) && (p[0] - head[0]).abs() < NAPE_HAIR_X).then_some((NAPE_HAIR_BOTTOM, NAPE_HAIR_BONE))
    }

    fn nape_curtain_hair(&self) -> Option<f64> {
        Some(NAPE_HAIR_TUCK)
    }

    fn prepare_mdl(&self, mdl: &[u8]) -> Result<Vec<u8>> {
        use crate::kh2_hair_rig as r;
        let Some(rig) = self.rig(mdl)? else { return Ok(mdl.to_vec()) };
        let model = mdlx::parse(&self.mdlx).map_err(|e| Error::Format(format!("KH2 {RIKU_MDLX}: {e}")))?;
        let sk0 = crate::mdl::RecomSkeleton::parse(mdl)?;
        let head = sk0.find(HEAD_BONE).ok_or_else(|| Error::Format("hair: no head bone".into()))?;
        let sk = r::extend_skeleton(&sk0, head, &model, &rig.carry);
        let parents: Vec<i16> = sk.bones.iter().map(|b| b.parent).collect();
        let rest = rig.locals(&parents, head, &rig.head_rel(&r::rel(&skeleton::bind_pose(&model.bones))));
        let n0 = sk0.bones.len();
        let mut out = mdl.to_vec();
        r::patch_rest(&mut out, &rest.iter().filter(|(b, _)| *b < n0).cloned().collect::<Vec<_>>())?;
        // the 7 extra bones go after Re:CoM's last bone, in every part
        if sk.bones.len() > n0 {
            out = r::append_bones(&out, &sk.bones[n0..])?;
        }
        Ok(out)
    }

    fn bake_ann(&self, ann: &[u8], mdl: &[u8]) -> Result<Option<Vec<u8>>> {
        self.bake_pack(ann, mdl, true)
    }

    fn hair(&self, names: &[String], world: &[Aff]) -> Result<SrcHair> {
        self.hair_tris(names, world)
    }
}

impl Kh2HairSource {
    /// A pack made for the KH2-hair skeleton (the 7 extra bones, the hair's channels), `by_slot` as [`crate::kh2_hair_rig::bake`].
    /// 0.197.4: also Riku's event packs (`PLAYER/40x2`), so they play on a 109-bone Riku ([`crate::riku_skin::EVENT_PACKS`]).
    pub fn bake_pack(&self, ann: &[u8], mdl: &[u8], by_slot: bool) -> Result<Option<Vec<u8>>> {
        self.bake_pack_slots(ann, mdl, if by_slot { crate::kh2_hair_rig::RIKU_SLOTS } else { crate::kh2_hair_rig::NO_SLOTS })
    }

    /// [`Self::bake_pack`] with any pack's slots (0.198.32: Dark Riku's own pack, [`crate::riku_skin::DARK_SLOTS`]).
    pub fn bake_pack_slots(&self, ann: &[u8], mdl: &[u8], slots: crate::kh2_hair_rig::Slots) -> Result<Option<Vec<u8>>> {
        let Some(rig) = self.rig(mdl)? else { return Ok(None) };
        let model = mdlx::parse(&self.mdlx).map_err(|e| Error::Format(format!("KH2 {RIKU_MDLX}: {e}")))?;
        let sk0 = crate::mdl::RecomSkeleton::parse(mdl)?;
        let head = sk0.find(HEAD_BONE).ok_or_else(|| Error::Format("hair: no head bone".into()))?;
        let sk = crate::kh2_hair_rig::extend_skeleton(&sk0, head, &model, &rig.carry);
        let mut a = crate::ann::Ann::parse(ann)?;
        // every clip has as many bones as the model: the 7 extra ones, then the hair's channels
        crate::kh2_hair_rig::extend_ann(&mut a, sk.bones.len() - sk0.bones.len());
        crate::kh2_hair_rig::bake_slots(&mut a, rig, &sk, head, &model, &self.motions()?, slots)?;
        Ok(Some(a.to_bytes()))
    }

    fn hair_tris(&self, names: &[String], world: &[Aff]) -> Result<SrcHair> {
        let model = mdlx::parse(&self.mdlx).map_err(|e| Error::Format(format!("KH2 {RIKU_MDLX}: {e}")))?;
        let pose = skeleton::bind_pose(&model.bones);
        let group = model.groups.get(HAIR_GROUP).ok_or_else(|| Error::Format(format!("KH2 {RIKU_MDLX}: no group {HAIR_GROUP}")))?;
        if group.texture != 3 || pose.len() <= HEAD_BONE_KH2 {
            return bad(format!("KH2 {RIKU_MDLX}: group {HAIR_GROUP} is texture {} with {} bones, not the hair", group.texture, pose.len()));
        }
        let img = decode_dds(&self.dds)?;
        let head = names.iter().position(|n| n == HEAD_BONE).ok_or_else(|| Error::Format("hair: no head bone".into()))?;
        let k = skeleton::point(&pose[HEAD_BONE_KH2], [0.0; 3]);
        let h = world[head].t;
        let frame = |w: [f64; 3]| [h[0] + WIDTH * SCALE * (w[0] - k[0]) + SHIFT[0], h[1] + SCALE * (w[1] - k[1]) + SHIFT[1], h[2] + SCALE * (w[2] - k[2]) + SHIFT[2]];
        // Re:CoM's candidate bones: the head and every hair bone, at rest.
        let cands: Vec<usize> = names.iter().enumerate().filter(|(i, n)| *i == head || n.contains("hair")).map(|(i, _)| i).collect();
        let mut bone_name: HashMap<u16, String> = HashMap::new();
        // the rig of this skeleton (`names` is the extended one: the base skeleton's names come first)
        let rig = self.rig.borrow().iter().find(|(k, _)| names.len() >= k.len() && names[..k.len()] == k[..]).map(|x| x.1).or(self.current.get());
        // the face fit: KH2's head skin (non-hair groups, vertices on the head bone) carried by the same fit
        let fit = {
            let rf = self.face.borrow();
            let mut kf = vec![];
            for (gi, g) in model.groups.iter().enumerate() {
                if gi == HAIR_GROUP {
                    continue;
                }
                for t in &g.triangles {
                    if t.iter().all(|v| v.bone as usize == HEAD_BONE_KH2) {
                        kf.push(t.clone().map(|v| frame(skeleton::point(&pose[v.bone as usize], v.position.map(|x| x as f64)))));
                    }
                }
            }
            let c = [h[0] + crate::ddd_hair::HEAD_CENTRE[0], h[1] + crate::ddd_hair::HEAD_CENTRE[1], h[2] + crate::ddd_hair::HEAD_CENTRE[2]];
            if rf.is_empty() || kf.is_empty() { FaceFit::default() } else { FaceFit::new(c, &rf, &kf) }
        };
        let mut name_of = |b: u16| -> String {
            bone_name
                .entry(b)
                .or_insert_with(|| {
                    if let Some(rig) = rig {
                        // 0.195.0: the rig's bones (KH2's own motion)
                        let b = b as usize;
                        if b == crate::kh2_hair_rig::NECK {
                            return crate::kh2_hair_rig::NECK_RECOM.to_string();
                        }
                        return rig.recom_of(b).map_or(HEAD_BONE.to_string(), |r| names[r].clone());
                    }
                    if CAP_BONES.contains(&(b as usize)) {
                        return HEAD_BONE.to_string();
                    }
                    let p = frame(skeleton::point(&pose[b as usize], [0.0; 3]));
                    let best = cands.iter().min_by(|&&x, &&y| dist2(world[x].t, p).total_cmp(&dist2(world[y].t, p))).copied().unwrap_or(head);
                    names[best].clone()
                })
                .clone()
        };
        let q = |x: f32| (x as f64 * 4096.0).round() as i32;
        let corners: Vec<[(i32, i32); 3]> = group.triangles.iter().map(|t| t.map(|v| (q(v.uv[0]), q(v.uv[1])))).collect();
        let (isl, n) = crate::ddd_hair::islands(&corners);
        let mut tris = Vec::with_capacity(group.triangles.len());
        for (t, &island) in group.triangles.iter().zip(&isl) {
            let mk = |v: &mdlx::Vertex, bn: &mut dyn FnMut(u16) -> String| {
                let pos = fit.apply(ear_shift(frame(skeleton::point(&pose[v.bone as usize], v.position.map(|x| x as f64))), h));
                let mut bone = bn(v.bone);
                // 0.198.6: the long back locks off the cap ([`crate::kh2_hair_rig::SPLIT`]): a vertex that follows the Re:CoM bone of
                // KH2 bone 101 or 104 (their own vertices and those of the KH2 bones they host), far enough from the head's centre
                if let Some(rig) = rig {
                    let c = [h[0] + crate::ddd_hair::HEAD_CENTRE[0], h[1] + crate::ddd_hair::HEAD_CENTRE[1], h[2] + crate::ddd_hair::HEAD_CENTRE[2]];
                    let r = ((pos[0] - c[0]).powi(2) + (pos[1] - c[1]).powi(2) + (pos[2] - c[2]).powi(2)).sqrt();
                    let behind = pos[2] - h[2] < crate::kh2_hair_rig::SPLIT_Z;
                    for &(k, n, f) in &crate::kh2_hair_rig::SPLIT {
                        let Some(src) = rig.recom_of(k) else { continue };
                        if bone != names[src] || !names.iter().any(|x| x == n) || !behind {
                            continue;
                        }
                        let band = if k == 104 || r >= crate::kh2_hair_rig::SPLIT_R[1] { 1.0 } else if r >= crate::kh2_hair_rig::SPLIT_R[0] { 2.0 / 3.0 } else { 1.0 / 3.0 };
                        if (band - f).abs() < 1e-9 {
                            bone = n.to_string();
                            break;
                        }
                    }
                }
                SrcVert { pos, uv: [v.uv[0] as f64, v.uv[1] as f64], bone }
            };
            tris.push(SrcTri { v: [mk(&t[0], &mut name_of), mk(&t[1], &mut name_of), mk(&t[2], &mut name_of)], island, cont: false, lining: false });
        }
        let centre = [h[0] + crate::ddd_hair::HEAD_CENTRE[0], h[1] + crate::ddd_hair::HEAD_CENTRE[1], h[2] + crate::ddd_hair::HEAD_CENTRE[2]];
        lift_out_of_skin(&mut tris, &self.face.borrow(), centre);
        let lined = lining(&tris, h, centre, HEAD_BONE, &self.face.borrow());
        tris.extend(lined);
        Ok(SrcHair { tris, islands: n, skin: 0, atlas: img, one_sided: false })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::ddd_hair::{tests as dt, with_hair, RIKU_0101_KH2};
    use crate::install;
    use crate::mdl::{split_parts, MdlPart, RecomSkeleton};

    /// 0.196.4: the proxy drop window is the ears' (both sides), not the face's middle, the neck or the top of the head.
    #[test]
    fn proxy_drop_window_is_the_ears() {
        let head = [0.0, 1.38, -0.04];
        let at = |x: f64, y: f64, z: f64| [head[0] + x, head[1] + y, head[2] + z];
        assert!(in_proxy_drop_window(at(0.11, 0.08, -0.005), head), "the +x ear top");
        assert!(in_proxy_drop_window(at(-0.11, 0.08, -0.005), head), "the -x ear top");
        assert!(!in_proxy_drop_window(at(0.02, 0.08, 0.0), head), "the middle of the head");
        assert!(!in_proxy_drop_window(at(0.11, 0.25, 0.0), head), "the top of the head");
        assert!(!in_proxy_drop_window(at(0.11, 0.08, 0.15), head), "the face");
        assert!(!in_proxy_drop_window(at(0.11, -0.05, 0.0), head), "the neck");
    }

    /// 0.196.3: the two-sided window is the +x ear's (head-relative), not the -x side, the front hairline or the neck.
    #[test]
    fn ear_two_sided_window_is_the_plus_x_ear() {
        let head = [0.0, 1.40, 0.0];
        let at = |x: f64, y: f64, z: f64| [head[0] + x, head[1] + y, head[2] + z];
        assert!(in_ear_two_sided_window(at(0.09, 0.063, -0.001), head), "Re:CoM's ear centre on +x");
        assert!(in_ear_two_sided_window(at(0.095, 0.06, -0.045), head), "the temple triangle behind the point (centroid x +0.09)");
        assert!(!in_ear_two_sided_window(at(-0.09, 0.063, -0.001), head), "-x side");
        assert!(!in_ear_two_sided_window(at(0.03, 0.063, -0.001), head), "too close to the middle");
        assert!(!in_ear_two_sided_window(at(0.09, 0.2, -0.001), head), "above the ear window");
        assert!(!in_ear_two_sided_window(at(0.09, 0.063, 0.1), head), "in front of the ear window");
    }

    /// 0.188.0 (owner's install, `RECOM_GAME`; only the 1.5+2.5 collection is needed): the swap runs, and the debug renders
    /// of the head with KH2's hair go to the workspace's `local/assets/recom/riku-hair/out/kh2/`.
    #[test]
    fn swaps_rikus_hair_for_kh2s() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let mdl = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        let e = arc.find("PLAYER/0101/PL0101.RTM").unwrap();
        let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
        let src = Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap();
        {
            let sk = RecomSkeleton::parse(&mdl).unwrap();
            let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
            let h = src.hair(&names, &sk.rest_world()).unwrap();
            let mut bb: std::collections::BTreeMap<usize, ([f64; 4], usize)> = Default::default();
            for t in &h.tris {
                for v in &t.v {
                    let b = bb.entry(t.island).or_insert(([9.0, 9.0, -9.0, -9.0], 0));
                    b.0 = [b.0[0].min(v.uv[0]), b.0[1].min(v.uv[1]), b.0[2].max(v.uv[0]), b.0[3].max(v.uv[1])];
                }
                bb.get_mut(&t.island).unwrap().1 += 1;
            }
            eprintln!("islands {} of {}", bb.len(), h.islands);
            for (i, (bx, n)) in &bb {
                eprintln!("island {i}: {n} tris uv [{:.3} {:.3} {:.3} {:.3}] -> {:.0}x{:.0} px", bx[0], bx[1], bx[2], bx[3], (bx[2] - bx[0]) * 1024.0, (bx[3] - bx[1]) * 1024.0);
            }
            let mut by: std::collections::BTreeMap<String, usize> = Default::default();
            for t in &h.tris { for v in &t.v { *by.entry(v.bone.clone()).or_default() += 1; } }
            eprintln!("bones {by:?}");
            let (mut lo, mut hi) = ([9.0f64; 3], [-9.0f64; 3]);
            for t in &h.tris { for v in &t.v { for k in 0..3 { lo[k] = lo[k].min(v.pos[k]); hi[k] = hi[k].max(v.pos[k]); } } }
            eprintln!("hair bounds {lo:?} {hi:?}");
            let (rlo, rhi) = dt::head_front_bounds(&mdl);
            eprintln!("Re:CoM head-bone front triangles bounds {rlo:?} {rhi:?}");
            let model = kh2_formats_lite::mdlx::parse(&src.mdlx).unwrap();
            let pose = kh2_formats_lite::skeleton::bind_pose(&model.bones);
            let k = kh2_formats_lite::skeleton::point(&pose[98], [0.0; 3]);
            eprintln!("kh2 head bone {k:?}");
            for (gi, g) in model.groups.iter().enumerate() {
                let (mut a, mut b) = ([1e9f64; 3], [-1e9f64; 3]);
                for t in &g.triangles { for v in t { let q = kh2_formats_lite::skeleton::point(&pose[v.bone as usize], v.position.map(|x| x as f64)); for c in 0..3 { a[c] = a[c].min(q[c]); b[c] = b[c].max(q[c]); } } }
                eprintln!("kh2 group {gi} ({} tris) bounds {:?} {:?}", g.triangles.len(), a.map(|x| ((x - k[0 ]) * 1.0).round()), b.map(|x| x.round()));
            }
        }
        let swap = with_hair(&mdl, &rtm, &hd, &RIKU_0101_KH2, &src).unwrap();
        eprintln!("{:?}", swap.report);
        // 0.197.1: KH2's 613 hair triangles and the lining's copies over the back and sides of the skull
        let lined = src.hair(&RecomSkeleton::parse(&mdl).unwrap().bones.iter().map(|b| b.name.clone()).collect::<Vec<_>>(), &RecomSkeleton::parse(&mdl).unwrap().rest_world()).unwrap().tris.iter().filter(|t| t.lining).count();
        eprintln!("lining triangles: {lined}");
        assert_eq!(swap.report.ddd_hair, 613 + lined);
        assert!((50..400).contains(&lined), "{lined} lining triangles");
        let again = with_hair(&mdl, &rtm, &hd, &RIKU_0101_KH2, &src).unwrap();
        assert!(swap.mdl == again.mdl && swap.dds == again.dds, "deterministic");
        crate::mdl::Mdl::parse(&swap.mdl).unwrap();
        let (p3, img) = dt::test_view(&swap);
        assert!(p3.texture_names().contains(&RIKU_0101_KH2.new_texture.to_string()));
        let sk = RecomSkeleton::parse(&swap.mdl).unwrap();
        let world = sk.rest_world();
        let out = dt::out_dir().join("kh2");
        std::fs::create_dir_all(&out).unwrap();
        dt::render(&p3, &world, &img, &out);
        std::fs::write(out.join("rk_hair_kh2_atlas.png"), dt::png(img.w, img.h, &img.px.iter().flat_map(|p| [p[0], p[1], p[2]]).collect::<Vec<_>>())).unwrap();
        std::fs::write(out.join("PL0101_hair_kh2.MDL"), &swap.mdl).unwrap();
        let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
        // body vs hair behind the back: per 5 cm band of y, the most backward body vertex and hair vertex (|x| < 0.15)
        {
            let (b3, hr) = (MdlPart::parse_bytes(&split_parts(&mdl).unwrap()[3]).unwrap(), src.hair(&names, &world).unwrap());
            let mut body: std::collections::BTreeMap<i32, f64> = Default::default();
            let mut hairz: std::collections::BTreeMap<i32, f64> = Default::default();
            for t in dt::collect(&b3) {
                for v in &t.v {
                    let w = world[v.bone()].point(v.pos().map(|x| x as f64));
                    let nm = &names[v.bone()];
                    if w[0].abs() < 0.15 && !nm.contains("hair") && nm != "head" {
                        let e = body.entry((w[1] * 20.0).floor() as i32).or_insert(9.0);
                        *e = e.min(w[2]);
                    }
                }
            }
            for t in &hr.tris {
                for v in &t.v {
                    let e = hairz.entry((v.pos[1] * 20.0).floor() as i32).or_insert(9.0);
                    *e = e.min(v.pos[2]);
                }
            }
            for (k, z) in &hairz {
                eprintln!("band y {:.2}: hair back z {:.3}, body back z {:?}", *k as f64 / 20.0, z, body.get(k));
            }
        }
        let h = dt::top_holes(&p3, &world);
        eprintln!("top-down holes: {}", h.len());
        for (i, n) in names.iter().enumerate().filter(|(_, n)| n.contains("hair") || *n == "head") {
            eprintln!("bone {i} {n}: {:?}", world[i].t);
        }
    }


    /// Debug (0.188.1): Re:CoM's `PL0101.ANN` clips and where the hair bones go in them, relative to the head.
    #[test]
    fn hair_bones_in_riku_clips() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let mdl = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        let ann = crate::ann::Ann::parse(&install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.ANN").unwrap()).unwrap();
        let sk = RecomSkeleton::parse(&mdl).unwrap();
        let head = sk.find("head").unwrap();
        let hb: Vec<usize> = sk.bones.iter().enumerate().filter(|(_, b)| b.name.contains("hair")).map(|(i, _)| i).collect();
        eprintln!("slots {}", ann.slots.len());
        for &b in &hb { eprintln!("hair bone {b} {} parent {}", sk.bones[b].name, sk.bones[b].parent); }
        let rest = sk.rest_world();
        let rel = |w: &[Aff], b: usize| -> crate::math::M3 {
            let (rb, _) = crate::math::unscale(&w[b].r);
            let (rh, _) = crate::math::unscale(&w[head].r);
            crate::math::m3(&rb, &crate::math::tr(&rh))
        };
        for (i, s) in ann.slots.iter().enumerate() {
            let Some(c) = &s.clip else { continue };
            let mut worst = vec![0.0f64; hb.len()];
            for f in 0..c.frames.max(1) as usize {
                let w = posed_world(&sk, c, f as f64);
                for (k, &b) in hb.iter().enumerate() {
                    let d = crate::math::angle_deg(&crate::math::m3(&rel(&w, b), &crate::math::tr(&rel(&rest, b))));
                    worst[k] = worst[k].max(d);
                }
            }
            eprintln!("clip {i}: max hair-bone turn vs rest, relative to head, deg {:?}", worst.iter().map(|x| x.round()).collect::<Vec<_>>());
        }
    }

    /// The skeleton posed by `clip` at frame `t`: world matrices (rotation / translation / scale of the clip's channels).
    pub(crate) fn posed_world(sk: &RecomSkeleton, c: &crate::ann::Clip, t: f64) -> Vec<Aff> {
        let mut w: Vec<Aff> = Vec::with_capacity(sk.bones.len());
        for (i, b) in sk.bones.iter().enumerate() {
            let local = if i < c.bones as usize {
                let s = c.sample(i, t);
                let mut r = crate::math::rot_xyz(s.r[0], s.r[1], s.r[2]);
                for (row, sc) in r.iter_mut().zip(s.s) { for v in row.iter_mut() { *v *= sc; } }
                Aff { r, t: s.t }
            } else { b.local };
            w.push(if b.parent < 0 { local } else { local.mul(&w[b.parent as usize]) });
        }
        w
    }

    /// 0.198.2 (owner's install, `RECOM_GAME`): the owner's in-game checks as renders (`camera_checks`) on the KH2 hair as built
    /// (its own `.ANN`), in `out/kh2/cam/` (`sheet.png`); and the nape curtain is there.
    #[test]
    fn kh2_camera_checks() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let mdl = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        let ann0 = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.ANN").unwrap();
        let e = arc.find("PLAYER/0101/PL0101.RTM").unwrap();
        let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
        let src = Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap();
        let swap = with_hair(&src.prepare_mdl(&mdl).unwrap(), &rtm, &hd, &RIKU_0101_KH2, &src).unwrap();
        // 0.198.6: the nape curtain on the hair's texels, inside the collar
        eprintln!("nape curtain edges: {}", swap.report.nape);
        assert!(swap.report.nape > 0, "the nape is closed");
        {
            let sk = RecomSkeleton::parse(&swap.mdl).unwrap();
            let w = sk.rest_world();
            let hd = sk.find("head").unwrap();
            for n in ["head", "neck1", "neck2", "neck3", "chest1"] { let i = sk.find(n).unwrap(); eprintln!("REST {n}: y {:.3} z {:.3}", w[i].t[1] - w[hd].t[1], w[i].t[2] - w[hd].t[2]); }
            // the collar's top behind the neck: the highest vertex of part 3 on chest1 (texture 0, the vest) within 6 cm of the middle, behind
            let p3 = MdlPart::parse_bytes(&split_parts(&swap.mdl).unwrap()[3]).unwrap();
            let mut top = f64::MIN;
            for t in dt::collect(&p3) { if t.tex != 0 { continue; } for v in &t.v { let q = w[v.bone()].point(v.pos().map(|x| x as f64)); if (q[0] - w[hd].t[0]).abs() < 0.06 && q[2] < w[hd].t[2] { top = top.max(q[1] - w[hd].t[1]); } } }
            eprintln!("REST collar top behind: y {top:.3}");
            let mut bones: std::collections::BTreeMap<String, usize> = Default::default();
            for t in dt::collect(&p3) { if t.tex != 0 { continue; } for v in &t.v { let q = w[v.bone()].point(v.pos().map(|x| x as f64)); if (q[0] - w[hd].t[0]).abs() < 0.06 && q[2] < w[hd].t[2] && q[1] - w[hd].t[1] > top - 0.02 { *bones.entry(names_of(&sk, v.bone())).or_default() += 1; } } }
            eprintln!("REST collar top bones: {bones:?}");
        }
        {
            let names: Vec<String> = RecomSkeleton::parse(&swap.mdl).unwrap().bones.iter().map(|b| b.name.clone()).collect();
            let p3 = MdlPart::parse_bytes(&split_parts(&swap.mdl).unwrap()[3]).unwrap();
            let mut by: std::collections::BTreeMap<String, usize> = Default::default();
            for t in dt::collect(&p3) { for v in &t.v { if names[v.bone()].starts_with("k2_hair_10") || names[v.bone()].starts_with("b_hair") { *by.entry(names[v.bone()].clone()).or_default() += 1; } } }
            eprintln!("split: {by:?}");
            if std::env::var("BONE_COLOURS").is_ok() { for (n, _) in &by { let i = names.iter().position(|x| x == n).unwrap(); eprintln!("colour {i} % 16 = {} : {n}", i % 16); } }
        }
        let ann = crate::ann::Ann::parse(&src.bake_ann(&ann0, &mdl).unwrap().unwrap()).unwrap();
        let (p3, img) = dt::test_view(&swap);
        let sk = RecomSkeleton::parse(&swap.mdl).unwrap();
        let dir = dt::out_dir().join("kh2").join(std::env::var("CAM_TAG").unwrap_or_else(|_| "cam".into()));
        let body = body_textures(&swap);
        camera_checks(&p3, &sk, &ann, &img, &body, &dir);
    }

    /// 0.198.4 (fork owner on 0.198.3, a box at the nape over the collar: "c", both the grey piece and the bare nape): Re:CoM's own
    /// Riku (no hair swap), textured, in the same cameras, `out/kh2/cam_recom/`; and the head / neck triangles whose centroid sits over
    /// the nape (behind the head bone, between it and the collar) by bone and texture.
    #[test]
    fn recom_own_nape() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let mdl = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        let ann = crate::ann::Ann::parse(&install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.ANN").unwrap()).unwrap();
        let e = arc.find("PLAYER/0101/PL0101.RTM").unwrap();
        let (rtm, _hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
        let rtm = crate::rtm::Rtm::parse(&rtm).unwrap();
        let p3 = MdlPart::parse_bytes(&crate::mdl::split_parts(&mdl).unwrap()[3]).unwrap();
        let body: Vec<crate::texture::Image> = p3.textures.iter().map(|n| {
            let n = String::from_utf8_lossy(n.split(|&b| b == 0).next().unwrap_or(&[])).to_string();
            let e = rtm.entries.iter().find(|e| e.name_str().trim_end_matches(".tm2") == n.trim_end_matches(".tm2")).unwrap();
            let (w, h, px) = crate::rtm::tim2_decode8(&e.data).unwrap();
            crate::texture::Image { w, h, px }
        }).collect();
        let sk = RecomSkeleton::parse(&mdl).unwrap();
        let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
        let dir = dt::out_dir().join("kh2").join("cam_recom");
        std::fs::create_dir_all(&dir).unwrap();
        let c = ann.slots[3].clip.as_ref().unwrap();
        for f in [6.0, 17.0] {
            for pitch in [-15.0, 0.0] {
                render_cam(&p3, &posed_world(&sk, c, f), &names, &body[1], &body, 180.0, pitch, true, &dir.join(format!("nape_f{f}_p{pitch}.png")));
            }
        }
        let c1 = ann.slots[1].clip.as_ref().unwrap();
        render_cam(&p3, &posed_world(&sk, c1, 25.0), &names, &body[1], &body, 180.0, 0.0, true, &dir.join("idle_back.png"));
        let w = sk.rest_world();
        let head = sk.find("head").unwrap();
        let h0 = w[head].t;
        let mut by: std::collections::BTreeMap<(String, u32), usize> = Default::default();
        for t in dt::collect(&p3) {
            let q: Vec<[f64; 3]> = t.v.iter().map(|v| w[v.bone()].point(v.pos().map(|x| x as f64))).collect();
            let m = [0, 1, 2].map(|k| (q[0][k] + q[1][k] + q[2][k]) / 3.0 - h0[k]);
            if m[1] < 0.0 && m[1] > -0.11 && m[2] < -0.02 && m[0].abs() < 0.06 {
                *by.entry((t.v.iter().map(|v| names[v.bone()].clone()).collect::<Vec<_>>().join("/"), t.tex)).or_default() += 1;
            }
        }
        for ((b, tx), n) in by { eprintln!("NAPE tris {b} tex {tx}: {n}"); }
    }

    fn names_of(sk: &RecomSkeleton, b: usize) -> String { sk.bones[b].name.clone() }

    /// Debug (0.188.1): the swapped head and hair posed by Re:CoM's own clips, renders in `out/kh2/pose_<tag>/<clip>_<frame>/`
    /// (`tag` = env `HAIR_TAG`, default `now`).
    #[test]
    fn posed_hair_renders() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let mdl = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        let e = arc.find("PLAYER/0101/PL0101.RTM").unwrap();
        let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
        let ann = crate::ann::Ann::parse(&install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.ANN").unwrap()).unwrap();
        
        let src = Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap();
        let swap = with_hair(&mdl, &rtm, &hd, &RIKU_0101_KH2, &src).unwrap();
        let (p3, img) = dt::test_view(&swap);
        let sk = RecomSkeleton::parse(&swap.mdl).unwrap();
        let tag = std::env::var("HAIR_TAG").unwrap_or_else(|_| "now".into());
        let base = dt::out_dir().join("kh2").join(format!("pose_{tag}"));
        let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
        {
            let d = base.join("rest");
            std::fs::create_dir_all(&d).unwrap();
            let w = sk.rest_world();
            let _ = render_body(&p3, &w, &names, &img, 0, &d.join("body_side.png"));
            let _ = render_body(&p3, &w, &names, &img, 1, &d.join("body_back.png"));
            let mut by: std::collections::BTreeMap<String, (usize, f64, f64)> = Default::default();
            for t in dt::collect(&p3) { if t.tex != 1 { continue; } for v in &t.v { let n = names[v.bone()].clone(); let y = w[v.bone()].point(v.pos().map(|x| x as f64))[1]; let e = by.entry(n).or_insert((0, 9.0, -9.0)); e.0 += 1; e.1 = e.1.min(y); e.2 = e.2.max(y); } }
            for (n, (c, lo, hi)) in &by { if n.starts_with("neck") || n.starts_with("chest") || n == "head" { eprintln!("REST tex1 corners on {n}: {c}, y {lo:.3}..{hi:.3}"); } }
        }
        for (clip, frames) in [(1usize, vec![0.0, 25.0, 50.0, 75.0]), (3, vec![0.0, 6.0, 11.0, 17.0]), (4, vec![8.0]), (14, vec![5.0]), (17, vec![7.0]), (26, vec![20.0]), (37, vec![10.0]), (41, vec![16.0])] {
            let Some(c) = ann.slots[clip].clip.as_ref() else { continue };
            for f in frames {
                let w = posed_world(&sk, c, f);
                let d = base.join(format!("c{clip}_f{f}"));
                std::fs::create_dir_all(&d).unwrap();
                let _ = render_body(&p3, &w, &names, &img, 0, &d.join("body_side.png"));
                let _ = render_body(&p3, &w, &names, &img, 1, &d.join("body_back.png"));
            }
        }
    }

    /// Debug (0.188.3): z-buffered render of the upper body (head, hair, neck, chest, collars, upper arms) of part 3 posed by
    /// `world`; hair texels from `atlas`, the rest flat grey-blue shaded by its normal. `view`: 0 = side (looking along -x), 1 = back, 2 = front, 3 = the other side.
    pub(crate) fn render_body(p3: &MdlPart, world: &[Aff], names: &[String], atlas: &crate::texture::Image, view: usize, path: &std::path::Path) -> usize {
        let yaw = match view { 0 => 90.0, 2 => 0.0, 3 => -90.0, _ => 180.0 };
        render_body_cam(p3, world, names, atlas, yaw, 0.0, false, path)
    }

    /// [`render_body`] from any camera (0.198.2; fork owner: "sempre que fizer essas alterações tenta fazer um teste no jogo correndo
    /// com a camera pra cima e outra para baixo e outra circulando o boneco em idle"): `yaw` degrees around the body (0 front, 90 its
    /// +x side, 180 back), `pitch` degrees (+ from above, − from below); `light`: a light background, as Re:CoM's halls, so a hole shows.
    pub(crate) fn render_body_cam(p3: &MdlPart, world: &[Aff], names: &[String], atlas: &crate::texture::Image, yaw: f64, pitch: f64, light: bool, path: &std::path::Path) -> usize {
        render_cam(p3, world, names, atlas, &[], yaw, pitch, light, path)
    }

    /// Part 3's textures by index (0.198.3: the body textured in the camera checks, so skin over the vest shows; 0.198.2's flat body
    /// hid the nape curtain's leak): texture `i` is the `.RTM` entry named as part 3's texture table entry `i` (Checked (files):
    /// part 3's index 0 is `n_zz110_02`, the vest, and 1 is `n_zz110_01`, the face / skin atlas, the reverse of the `.RTM` order).
    /// Index 1 is drawn from [`dt::test_view`]'s picture instead.
    pub(crate) fn body_textures(swap: &crate::ddd_hair::HairSwap) -> Vec<crate::texture::Image> {
        let rtm = crate::rtm::Rtm::parse(&swap.rtm).unwrap();
        let p3 = MdlPart::parse_bytes(&crate::mdl::split_parts(&swap.mdl).unwrap()[3]).unwrap();
        p3.textures.iter().map(|n| {
            let n = String::from_utf8_lossy(n.split(|&b| b == 0).next().unwrap_or(&[])).to_string();
            let e = rtm.entries.iter().find(|e| e.name_str().trim_end_matches(".tm2") == n.trim_end_matches(".tm2"));
            let (w, h, px) = e.and_then(|e| crate::rtm::tim2_decode8(&e.data).ok()).unwrap_or((1, 1, vec![[90, 120, 200, 255]]));
            eprintln!("part 3 texture {n}: {}", if e.is_some() { "found" } else { "missing" });
            crate::texture::Image { w, h, px }
        }).collect()
    }

    /// [`render_body_cam`] with the body's own textures (`body[i]` for texture `i`, see [`body_textures`]); texture 1 is `atlas`
    /// (hair and skin); an index `body` lacks is drawn flat grey-blue.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_cam(p3: &MdlPart, world: &[Aff], names: &[String], atlas: &crate::texture::Image, body: &[crate::texture::Image], yaw: f64, pitch: f64, light: bool, path: &std::path::Path) -> usize {
        render_view(p3, world, names, atlas, body, yaw, pitch, light, None, path)
    }

    /// [`render_cam`] as a close-up (0.198.5; fork owner on 0.198.4: "você não viu isso???", hair lines through the cheek and Re:CoM's
    /// kept hair tips over the vest, both too small for the 350 px cells): `focus` = (a world point, the half-width in metres shown
    /// around it), 700 px.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_focus(p3: &MdlPart, world: &[Aff], names: &[String], atlas: &crate::texture::Image, body: &[crate::texture::Image], yaw: f64, pitch: f64, focus: ([f64; 3], f64), path: &std::path::Path) -> usize {
        render_view(p3, world, names, atlas, body, yaw, pitch, true, Some(focus), path)
    }

    #[allow(clippy::too_many_arguments)]
    fn render_view(p3: &MdlPart, world: &[Aff], names: &[String], atlas: &crate::texture::Image, body: &[crate::texture::Image], yaw: f64, pitch: f64, light: bool, focus: Option<([f64; 3], f64)>, path: &std::path::Path) -> usize {
        // `VIEWS=a,b`: only the views whose file name contains one of these (fast tuning; the validation renders all)
        if let Ok(v) = std::env::var("VIEWS") {
            let f = path.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
            if !v.split(',').any(|x| f.contains(x)) {
                return 0;
            }
        }
        // 0.198.8: `FULL_BODY` set: every bone, and part 3's list B (Dark Riku's coat tails), for the coat's checks
        let full = std::env::var("FULL_BODY").is_ok();
        let ok = |n: &str| full || n == "head" || n.contains("hair") || n.starts_with("neck") || n.starts_with("chest") || n == "waist" || n.ends_with("collar") || n.ends_with("uparm");
        let size = 700usize;
        let (sy, cy, sp, cp) = (yaw.to_radians().sin(), yaw.to_radians().cos(), pitch.to_radians().sin(), pitch.to_radians().cos());
        let tx = |q: [f64; 3]| -> [f64; 3] {
            let (x, y, z) = (q[0] * cy + q[2] * sy, q[1], -q[0] * sy + q[2] * cy);
            [x, y * cp - z * sp, y * sp + z * cp]
        };
        let mut tris: Vec<([[f64; 3]; 3], [[f64; 2]; 3], bool, u32)> = vec![];
        let mut tri_bones: Vec<[String; 3]> = vec![];
        // 0.198.33: `GAME_CULL` set: part 3 drawn one-sided as the game does (flag 0 drawn iff its strip order is clockwise on the
        // monitor, 0x20 iff counter-clockwise; knowledge/ReCoM/recom-model-culling.md), so see-through openings show the background
        let cull = std::env::var("GAME_CULL").is_ok();
        let mut flags: Vec<u32> = vec![];
        let mut all_tris = dt::collect(p3);
        if full {
            if let Some(lb) = p3.list_b.as_ref() {
                all_tris.extend(dt::extract_list(lb));
            }
        }
        for t in all_tris {
            if !t.v.iter().all(|v| ok(&names[v.bone()])) { continue; }
            let p = t.v.clone().map(|v| tx(world[v.bone()].point(v.pos().map(|x| x as f64))));
            let hair = t.tex == 1;
            let uv = t.v.clone().map(|v| v.uv().map_or([0.0; 2], |u| [u[0] as f64, u[1] as f64]));
            // debug (0.198.6): `BONE_COLOURS` set: hair drawn flat in one colour per bone of its first corner (texture index 1000 + bone)
            // `BONE_COLOURS=<bone name>`: only that bone red, the rest of the hair grey
            let ti = match std::env::var("BONE_COLOURS") {
                Ok(n) if hair && n.len() > 1 => if n.split(',').any(|x| names[t.v[0].bone()] == x) { 1000 } else { 1015 },
                Ok(_) if hair => 1000 + t.v[0].bone() as u32,
                _ => t.tex,
            };
            tris.push((p, uv, hair, ti));
            tri_bones.push(t.v.clone().map(|v| names[v.bone()].clone()));
            flags.push(t.v[2].q0[0]);
        }
        let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
        // 0.198.7: framed by the body (no head or hair corner) with fixed margins, so a change of hair does not move the picture
        // (it framed everything: a hair edit shifted the body and `render-diff` boxed whole views)
        for ((p, _, _, _), b) in tris.iter().zip(&tri_bones) {
            if b.iter().any(|n| n == "head" || n.contains("hair")) { continue; }
            for q in p { for k in 0..2 { lo[k] = lo[k].min(q[k]); hi[k] = hi[k].max(q[k]); } }
        }
        if lo[0] > hi[0] {
            for (p, _, _, _) in &tris { for q in p { for k in 0..2 { lo[k] = lo[k].min(q[k]); hi[k] = hi[k].max(q[k]); } } }
        } else {
            let m = 0.12;
            lo[0] -= m;
            hi[0] += m;
            hi[1] += 0.32;
        }
        if let Some((c, half)) = focus {
            let c = tx(c);
            lo = [c[0] - half, c[1] - half];
            hi = [c[0] + half, c[1] + half];
        }
        let sc = 640.0 / (hi[0] - lo[0]).max(hi[1] - lo[1]);
        let mut rgb: Vec<u8> = if light { [214u8, 212, 230].repeat(size * size) } else { vec![30u8; size * size * 3] };
        let mut zb = vec![f64::MIN; size * size];
        // body first, then hair: a hair fragment farther than the body surface is hidden by it (inside the torso or arm)
        let mut hidden = 0usize;
        let mut sd_body = vec![false; size * size];
        let mut order: Vec<usize> = (0..tris.len()).collect();
        order.sort_by_key(|&i| tris[i].2);
        let tris: Vec<_> = order.iter().map(|&i| tris[i].clone()).collect();
        let tri_bones: Vec<_> = order.iter().map(|&i| tri_bones[i].clone()).collect();
        let flags: Vec<u32> = order.iter().map(|&i| flags[i]).collect();
        // debug (0.198.6): `PICK=x,y` prints the triangle drawn at that pixel (its corners' bones and texture)
        let pick: Option<(usize, usize)> = std::env::var("PICK").ok().and_then(|v| { let mut it = v.split(',').filter_map(|x| x.parse().ok()); Some((it.next()?, it.next()?)) });
        let mut picked: Option<usize> = None;
        for (k, (p, uv, hair, ti)) in tris.iter().enumerate() {
            let s: Vec<[f64; 2]> = p.iter().map(|q| [30.0 + (q[0] - lo[0]) * sc, 670.0 - (q[1] - lo[1]) * sc]).collect();
            let n = { let a = [p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]]; let b = [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]]; let c = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]; let l = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt().max(1e-12); c[2].abs() / l };
            let det = (s[1][1] - s[2][1]) * (s[0][0] - s[2][0]) + (s[2][0] - s[1][0]) * (s[0][1] - s[2][1]);
            if det.abs() < 1e-9 { continue; }
            if cull && (((s[1][0] - s[0][0]) * (s[2][1] - s[0][1]) - (s[1][1] - s[0][1]) * (s[2][0] - s[0][0]) > 0.0) != (flags[k] == 0)) { continue; }
            let (x0, x1) = (s.iter().map(|q| q[0]).fold(f64::MAX, f64::min).floor().max(0.0) as usize, (s.iter().map(|q| q[0]).fold(f64::MIN, f64::max).ceil() as usize).min(size - 1));
            let (y0, y1) = (s.iter().map(|q| q[1]).fold(f64::MAX, f64::min).floor().max(0.0) as usize, (s.iter().map(|q| q[1]).fold(f64::MIN, f64::max).ceil() as usize).min(size - 1));
            for y in y0..=y1 { for x in x0..=x1 {
                let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
                let l0 = ((s[1][1] - s[2][1]) * (px - s[2][0]) + (s[2][0] - s[1][0]) * (py - s[2][1])) / det;
                let l1 = ((s[2][1] - s[0][1]) * (px - s[2][0]) + (s[0][0] - s[2][0]) * (py - s[2][1])) / det;
                let l2 = 1.0 - l0 - l1;
                if l0 < 0.0 || l1 < 0.0 || l2 < 0.0 { continue; }
                let z = l0 * p[0][2] + l1 * p[1][2] + l2 * p[2][2];
                if z <= zb[y * size + x] { if *hair && z < zb[y * size + x] - 0.004 && sd_body[y * size + x] { hidden += 1; } continue; }
                zb[y * size + x] = z;
                if pick == Some((x, y)) { picked = Some(k); }
                sd_body[y * size + x] = !*hair;
                let c = if *hair && *ti >= 1000 {
                    let k = 0.5 + 0.5 * n;
                    let c = bone_colour((*ti - 1000) as usize);
                    [(c[0] as f64 * k) as u8, (c[1] as f64 * k) as u8, (c[2] as f64 * k) as u8]
                } else if *hair {
                    let u = l0 * uv[0][0] + l1 * uv[1][0] + l2 * uv[2][0]; let v = l0 * uv[0][1] + l1 * uv[1][1] + l2 * uv[2][1];
                    let c = atlas.px[((v * atlas.h as f64) as usize).min(atlas.h - 1) * atlas.w + ((u * atlas.w as f64) as usize).min(atlas.w - 1)];
                    [c[0], c[1], c[2]]
                } else if let Some(b) = body.get(*ti as usize).filter(|_| *ti != 1) {
                    let u = l0 * uv[0][0] + l1 * uv[1][0] + l2 * uv[2][0]; let v = l0 * uv[0][1] + l1 * uv[1][1] + l2 * uv[2][1];
                    let c = b.px[((v.rem_euclid(1.0) * b.h as f64) as usize).min(b.h - 1) * b.w + ((u.rem_euclid(1.0) * b.w as f64) as usize).min(b.w - 1)];
                    let k = 0.55 + 0.45 * n;
                    [(c[0] as f64 * k) as u8, (c[1] as f64 * k) as u8, (c[2] as f64 * k) as u8]
                } else { let k = 60.0 + 140.0 * n; [(k * 0.5) as u8, (k * 0.7) as u8, k as u8] };
                rgb[3 * (y * size + x)..3 * (y * size + x) + 3].copy_from_slice(&c);
            } }
        }
        if let Some(k) = picked { eprintln!("PICK {}: bones {:?} texture {}", path.display(), tri_bones[k], tris[k].3); }
        // the colour legend beside the image (`BONE_COLOURS=1`): bone, RGB, the triangles drawn in it
        if std::env::var("BONE_COLOURS").is_ok_and(|v| v == "1") {
            let mut seen: std::collections::BTreeMap<usize, (String, usize)> = Default::default();
            for (t, b) in tris.iter().zip(&tri_bones) { if t.3 >= 1000 { let e = seen.entry((t.3 - 1000) as usize).or_insert((b[0].clone(), 0)); e.1 += 1; } }
            let txt: String = seen.iter().map(|(i, (n, c))| format!("{n} (bone {i}): rgb {:?}, {c} triangles\n", bone_colour(*i))).collect();
            let _ = std::fs::write(path.with_extension("legend.txt"), txt);
        }
        std::fs::write(path, dt::png(size, size, &rgb)).unwrap();
        hidden
    }

    /// A colour of its own for bone `b` (`BONE_COLOURS=1`; 0.198.6: the 16-colour palette by `b % 16` gave several bones one colour):
    /// hues spread by the golden ratio, alternating light and dark; index 15 of the old palette (grey-white) is `BONE_COLOURS=a,b`'s rest.
    pub(crate) fn bone_colour(b: usize) -> [u8; 3] {
        if b == 15 && std::env::var("BONE_COLOURS").is_ok_and(|v| v != "1") { return [235, 235, 235]; }
        if b == 0 && std::env::var("BONE_COLOURS").is_ok_and(|v| v != "1") { return [230, 25, 75]; }
        let h = (b as f64 * 0.618_033_988_75).fract() * 6.0;
        let (v, s) = if b % 2 == 0 { (1.0, 0.85) } else { (0.75, 1.0) };
        let x = 1.0 - (h % 2.0 - 1.0).abs();
        let (r, g, bl) = match h as usize { 0 => (1.0, x, 0.0), 1 => (x, 1.0, 0.0), 2 => (0.0, 1.0, x), 3 => (0.0, x, 1.0), 4 => (x, 0.0, 1.0), _ => (1.0, 0.0, x) };
        let f = |c: f64| ((1.0 - s + s * c) * v * 255.0) as u8;
        [f(r), f(g), f(bl)]
    }

    /// 0.198.7: [`dt::test_view`] + [`body_textures`] for a model whose atlas (the face's texture, `atlas`) is not texture 1 (Dark Riku's
    /// `n_zz120_01` is 2): the atlas and the hair are drawn as texture 1 (atlas left, hair right, as `test_view`), the old texture 1 as
    /// the atlas's index; `body` follows the same swap.
    pub(crate) fn view_at(swap: &crate::ddd_hair::HairSwap, atlas: &str) -> (MdlPart, crate::texture::Image, Vec<crate::texture::Image>) {
        let mut p3 = MdlPart::parse_bytes(&split_parts(&swap.mdl).unwrap()[3]).unwrap();
        let a = p3.texture_names().iter().position(|n| n == atlas).unwrap() as u32;
        let mut body = body_textures(swap);
        if a != 1 {
            for k in p3.list_a.iter_mut().chain(p3.list_b.iter_mut().flatten()) {
                for v in k.verts.iter_mut() {
                    if let Some(q) = v.q2.as_mut() {
                        if q[3] == a { q[3] = 1; } else if q[3] == 1 { q[3] = a; }
                    }
                }
            }
            body.swap(1, a as usize);
        }
        let hair = crate::texture::decode_dds(&swap.dds).unwrap();
        let old = crate::texture::decode_dds(&swap.atlas_dds).unwrap();
        let hw = hair.w * old.h / hair.h;
        let (w, h) = (old.w + hw, old.h);
        let px: Vec<[u8; 4]> = (0..w * h).map(|i| { let (x, y) = (i % w, i / w); if x < old.w { old.px[y * old.w + x] } else { hair.px[(y * hair.h / h) * hair.w + (x - old.w) * hair.w / hw] } }).collect();
        let (fa, fh) = (old.w as f32 / w as f32, hw as f32 / w as f32);
        for k in p3.list_a.iter_mut() {
            for v in k.verts.iter_mut() {
                if let Some(q) = v.q2.as_mut() {
                    let u = f32::from_bits(q[0]);
                    if q[3] == swap.hair_tex { q[0] = (fa + u * fh).to_bits(); q[3] = 1; } else if q[3] == 1 { q[0] = (u * fa).to_bits(); }
                }
            }
        }
        (p3, crate::texture::Image { w, h, px }, body)
    }

    /// 0.198.9 (fork owner: "melhorar o encaixe da espada na mão do Riku ... precisa ficar natural ... anatomicamente correto"): the right
    /// hand with the weapon (part 1 drawn with part 3), close-ups around `r_hand` from six sides, idle 0 / run 6 / first swing 8 and 16,
    /// before (`hand_<group>_before`) and after [`crate::grip::close`] (`hand_<group>_<tag>`), for Riku (`0101`) and Dark Riku (`0201`).
    #[test]
    fn hand_grip_checks() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        std::env::set_var("FULL_BODY", "1");
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let tag = std::env::var("CAM_TAG").unwrap_or_else(|_| "cam".into());
        for (grp, atlas_name, weapon_name) in [("0101", "n_zz110_01", "weapon_01"), ("0201", "n_zz120_01", "weapon_01")] {
            let mdl = install::read_recom(&arc, &keys, &format!("PLAYER/{grp}/PL{grp}.MDL")).unwrap();
            // the pack the game plays: Riku's own; for Dark Riku's model, the Dark Form skin's (Riku's motions on it)
            let ann0 = if grp == "0201" {
                // before: the skin without the grip (`NO_GRIP`); the grip is applied below as the build does
                crate::grip::OFF.with(|x| x.set(true));
                let skin = crate::riku_skin::build_with(&arc, &keys, 101, 201, 121, None).unwrap();
                crate::grip::OFF.with(|x| x.set(false));
                crate::ann::Ann::parse(&skin.files.iter().find(|f| f.0 == crate::riku_skin::path(121, "ANN")).unwrap().1).unwrap()
            } else {
                crate::ann::Ann::parse(&install::read_recom(&arc, &keys, &format!("PLAYER/{grp}/PL{grp}.ANN")).unwrap()).unwrap()
            };
            let e = arc.find(&format!("PLAYER/{grp}/PL{grp}.RTM")).unwrap();
            let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
            let rtm_p = crate::rtm::Rtm::parse(&rtm).unwrap();
            let parts = split_parts(&mdl).unwrap();
            let mut p3 = MdlPart::parse_bytes(&parts[3]).unwrap();
            let p1 = MdlPart::parse_bytes(&parts[1]).unwrap();
            let dec = |n: &str| { let e = rtm_p.entries.iter().find(|e| e.name_str().trim_end_matches(".tm2") == n.trim_end_matches(".tm2")).unwrap(); let (w, h, px) = crate::rtm::tim2_decode8(&e.data).unwrap(); crate::texture::Image { w, h, px } };
            let mut body: Vec<crate::texture::Image> = p3.texture_names().iter().map(|n| dec(n)).collect();
            let a = p3.texture_names().iter().position(|n| n == atlas_name).unwrap() as u32;
            for k in p3.list_a.iter_mut().chain(p3.list_b.iter_mut().flatten()) { for v in k.verts.iter_mut() { if let Some(q) = v.q2.as_mut() { if q[3] == a { q[3] = 1; } else if q[3] == 1 { q[3] = a; } } } }
            body.swap(1, a as usize);
            // the weapon's packets, on a texture index of their own
            let wi = body.len() as u32;
            body.push(dec(weapon_name));
            let mut wl = p1.list_a.clone();
            for k in wl.iter_mut() { for v in k.verts.iter_mut() { if let Some(q) = v.q2.as_mut() { q[3] = wi; } } }
            // `NO_WEAPON`: the hand as the field shows it (no weapon drawn)
            if std::env::var("NO_WEAPON").is_err() { p3.list_a.extend(wl); }
            let img = hd.iter().find(|x| x.name == crate::keyblade::hd_name(atlas_name)).and_then(|x| crate::texture::decode_dds(&x.data).ok()).unwrap_or_else(|| body[1].clone());
            let sk = RecomSkeleton::parse(&mdl).unwrap();
            let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
            let hand = sk.find(&std::env::var("HAND_BONE").unwrap_or_else(|_| "r_hand".into())).unwrap();
            let mut ann1 = ann0.clone();
            let own = crate::ann::Ann::parse(&install::read_recom(&arc, &keys, &format!("PLAYER/{grp}/PL{grp}.ANN")).unwrap()).unwrap();
            let rep = crate::grip::close(&mut ann1, &mdl, &own).unwrap();
            eprintln!("{grp}: grip from {:?}, {} clips, max weight {:.2}", rep.source, rep.clips, rep.max_weight);
            let rel = if std::env::var("NO_RELAX").is_ok() { Default::default() } else { crate::hands::relax(&mut ann1, &mdl).unwrap() };
            eprintln!("{grp}: relaxed {:?}, rest asymmetry {:.4} m", rel.clips, rel.asymmetry);
            // 0.198.10: the Dark Form skin's "after" is the pack the build writes (grip and relax on the skin's own clips)
            if grp == "0201" {
                // `NO_RELAX`: the 0.198.9 pack (grip, no relax), the reference for the relax's reviews
                let nr = std::env::var("NO_RELAX").is_ok();
                crate::hands::OFF.with(|x| x.set(nr));
                let skin = crate::riku_skin::build_with(&arc, &keys, 101, 201, 121, None).unwrap();
                crate::hands::OFF.with(|x| x.set(false));
                ann1 = crate::ann::Ann::parse(&skin.files.iter().find(|f| f.0 == crate::riku_skin::path(121, "ANN")).unwrap().1).unwrap();
            }
            for (which, ann) in [("before", &ann0), (tag.as_str(), &ann1)] {
                let dir = dt::out_dir().join("hand").join(format!("hand_{grp}_{which}"));
                std::fs::create_dir_all(&dir).unwrap();
                // `HAND_POSES=label:slot:frame,...`: only these poses
                let extra: Vec<(String, usize, f32)> = std::env::var("HAND_POSES").ok().map(|v| v.split(',').map(|p| { let q: Vec<&str> = p.split(':').collect(); (q[0].to_string(), q[1].parse().unwrap(), q[2].parse().unwrap()) }).collect()).unwrap_or_default();
                let fixed: Vec<(String, usize, f32)> = [("idle", 1usize, 0.0), ("idle79", 79, 8.0), ("s10f2", 10, 2.0), ("s10f7", 10, 7.0), ("s81", 81, 5.0), ("item34", 34, 20.0), ("s39", 39, 8.0), ("jump16", 4, 16.0), ("stance", 31, 0.0), ("stance40", 31, 40.0), ("run", 3, 6.0), ("swing8", 44, 8.0), ("swing16", 44, 16.0)].iter().map(|(a, b, c)| (a.to_string(), *b, *c as f32)).collect();
                for (label, slot, f) in if extra.is_empty() { fixed } else { extra } {
                    let f = f as f64;
                    let label = label.as_str();
                    let Some(c) = ann.slots.get(slot).and_then(|s| s.clip.as_ref()) else { continue };
                    let w = posed_world(&sk, c, f);
                    // 0.198.9: the owner's view: along the handle (wep0's x axis), from either end
                    let ax = { let r = crate::math::unscale(&w[sk.find("wep0").unwrap()].r).0; r[0] };
                    let yaw_h = ax[0].atan2(ax[2]).to_degrees();
                    let pitch_h = (-ax[1]).asin().to_degrees();
                    for (yn, yaw, pitch) in [("front", 0.0, 0.0), ("side", 90.0, 0.0), ("back", 180.0, 0.0), ("other", 270.0, 0.0), ("top", 0.0, 60.0), ("under", 0.0, -60.0), ("axis", yaw_h, pitch_h), ("axis2", yaw_h + 180.0, -pitch_h)] {
                        render_focus(&p3, &w, &names, &img, &body, yaw, pitch, (w[hand].t, std::env::var("HAND_HALF").ok().and_then(|x| x.parse().ok()).unwrap_or(0.07)), &dir.join(format!("{label}_{yn}.png")));
                    }
                }
            }
        }
    }

    /// 0.198.8 (fork owner: "no darkmode queria ver se é possível animar a saia do riku"): the Dark Form skin's whole body (coat tails
    /// included), `out/dark/skirt_<tag>/`: idle, walk, run, jump and the first ground swing, from the front, both sides and behind, at
    /// four frames each; `SKIRT_BEFORE=1` renders the skin without [`crate::tails::animate`] (the reference).
    #[test]
    fn dark_skirt_checks() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        std::env::set_var("FULL_BODY", "1");
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        // 0.198.13: built as the asset step builds it (KH1 Riku's guard, BBS Terra's Block), so the Guard clip `p002_225` is there
        let k1 = install::kh1(&g, keys.clone()).unwrap();
        let (gm, gd) = (k1.read("xa_ex_1010.mset").unwrap(), k1.read("xa_ex_1010.mdls").unwrap());
        let (am, ad) = (k1.read("xa_ex_1580.mset").unwrap(), k1.read("xa_ex_1580.mdls").unwrap());
        let terra = crate::terra::read_terra(&g, &keys).ok();
        let gsrc = crate::riku_skin::GuardSource { mset: &gm, mdls: &gd, ansem: Some((&am, &ad)), terra: terra.as_ref().map(|(k, c)| (k, c.as_slice())) };
        let skin = crate::riku_skin::build_with(&arc, &keys, 101, 201, 121, Some(&gsrc)).unwrap();
        let get = |ext: &str| skin.files.iter().find(|f| f.0 == crate::riku_skin::path(121, ext)).unwrap().1.clone();
        let mdl0 = get("MDL");
        let mut ann = crate::ann::Ann::parse(&get("ANN")).unwrap();
        let e = arc.find("PLAYER/0201/PL0201.RTM").unwrap();
        let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
        let sk = RecomSkeleton::parse(&mdl0).unwrap();
        if std::env::var("SKIRT_BEFORE").is_ok() {
            // the tails at rest, as before 0.198.8: every tail channel from the rest pose
            let tails: Vec<usize> = sk.bones.iter().enumerate().filter(|(_, b)| b.name.contains("_skt")).map(|(i, _)| i).collect();
            for s in ann.slots.iter_mut() {
                let Some(c) = s.clip.as_mut() else { continue };
                if c.bones as usize != sk.bones.len() { continue; }
                let rows: Vec<Vec<[f64; 9]>> = (0..c.frames.max(1) as usize).map(|f| (0..c.bones as usize).map(|b| {
                    let x = c.sample(b, f as f64);
                    if tails.contains(&b) { let r = crate::math::euler_xyz(&crate::math::unscale(&sk.bones[b].local.r).0, None); [x.t[0], x.t[1], x.t[2], r[0], r[1], r[2], x.s[0], x.s[1], x.s[2]] } else { [x.t[0], x.t[1], x.t[2], x.r[0], x.r[1], x.r[2], x.s[0], x.s[1], x.s[2]] }
                }).collect()).collect();
                *c = crate::riku_skin::encode_rows(c, &rows);
            }
        }
        let mut p3 = MdlPart::parse_bytes(&split_parts(&mdl0).unwrap()[3]).unwrap();
        let rtm_p = crate::rtm::Rtm::parse(&rtm).unwrap();
        let mut body: Vec<crate::texture::Image> = p3.textures.iter().map(|n| {
            let n = String::from_utf8_lossy(n.split(|&b| b == 0).next().unwrap_or(&[])).to_string();
            let e = rtm_p.entries.iter().find(|e| e.name_str().trim_end_matches(".tm2") == n.trim_end_matches(".tm2")).unwrap();
            let (w, h, px) = crate::rtm::tim2_decode8(&e.data).unwrap();
            crate::texture::Image { w, h, px }
        }).collect();
        let a = p3.texture_names().iter().position(|n| n == "n_zz120_01").unwrap() as u32;
        for k in p3.list_a.iter_mut().chain(p3.list_b.iter_mut().flatten()) { for v in k.verts.iter_mut() { if let Some(q) = v.q2.as_mut() { if q[3] == a { q[3] = 1; } else if q[3] == 1 { q[3] = a; } } } }
        body.swap(1, a as usize);
        let img = hd.iter().find(|x| x.name == crate::keyblade::hd_name("n_zz120_01")).and_then(|x| crate::texture::decode_dds(&x.data).ok()).unwrap_or_else(|| body[1].clone());
        let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
        let tag = std::env::var("CAM_TAG").unwrap_or_else(|_| "cam".into());
        let dir = dt::out_dir().join("dark").join(format!("skirt_{tag}"));
        std::fs::create_dir_all(&dir).unwrap();
        // 0.198.13 (fork owner: the skirt through the legs in idle, walk, guard, a stance and the jump's start): the battle stance
        // (31), the battle idle (79) and the Guard too; `SKIRT_AT=<slot>:<frame>,...` adds frames
        let guard = ann.find(crate::riku_skin::RIKU_GUARD).unwrap_or(usize::MAX);
        let at: Vec<(usize, usize)> = std::env::var("SKIRT_AT").ok().map(|v| v.split(',').filter_map(|x| { let (a, b) = x.split_once(':')?; Some((a.parse().ok()?, b.parse().ok()?)) }).collect()).unwrap_or_default();
        for (label, slot) in [("idle", 1usize), ("walk", 2), ("run", 3), ("jump", 4), ("apex", 5), ("fall", 6), ("land", 7), ("swing", 44), ("stance", 31), ("bidle", 79), ("guard", guard)] {
            let Some(c) = ann.slots.get(slot).and_then(|s| s.clip.as_ref()) else { continue };
            let n = c.frames.max(1) as usize;
            let mut frames = vec![0, n / 4, n / 2, 3 * n / 4];
            frames.extend(at.iter().filter(|x| x.0 == slot).map(|x| x.1));
            for f in frames {
                let w = posed_world(&sk, c, f as f64);
                for yaw in [0.0, 90.0, 180.0, 270.0] {
                    render_cam(&p3, &w, &names, &img, &body, yaw, 0.0, true, &dir.join(format!("{label}_f{f}_y{yaw}.png")));
                }
            }
        }
    }

    /// 0.198.7 (fork owner: "agora implementa todos os cabelos para o a Skin do riku Darkmode tbm"): the camera checks on the Dark Form
    /// skin (Dark Riku's model with Riku's motions, group 121) with KH2's and BBS's hair, `out/dark/<kh2|bbs>_<tag>/`; and the skin
    /// without a new hair (`out/dark/recom_<tag>/`) as the reference.
    #[test]
    fn dark_camera_checks() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let skin = crate::riku_skin::build_with(&arc, &keys, 101, 201, 121, None).unwrap();
        let get = |ext: &str| skin.files.iter().find(|f| f.0 == crate::riku_skin::path(121, ext)).unwrap().1.clone();
        let (mdl0, ann0) = (get("MDL"), get("ANN"));
        let e = arc.find("PLAYER/0201/PL0201.RTM").unwrap();
        let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
        let tag = std::env::var("CAM_TAG").unwrap_or_else(|_| "cam".into());
        let base = dt::out_dir().join("dark");
        let kh2 = Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap();
        let bbs = crate::bbs_hair::BbsHairSource::read(&g, &keys).unwrap();
        let which = std::env::var("DARK_HAIR").unwrap_or_else(|_| "kh2,bbs,recom".into());
        for (name, src, target) in [("kh2", &kh2 as &dyn HairSource, crate::ddd_hair::RIKU_0201_KH2), ("bbs", &bbs as &dyn HairSource, crate::ddd_hair::RIKU_0201_BBS)] {
            if !which.contains(name) { continue; }
            let swap = with_hair(&src.prepare_mdl(&mdl0).unwrap(), &rtm, &hd, &target, src).unwrap();
            let ann = crate::ann::Ann::parse(&src.bake_ann(&ann0, &mdl0).unwrap().unwrap_or(ann0.clone())).unwrap();
            let (p3, img, body) = view_at(&swap, target.texture);
            let sk = RecomSkeleton::parse(&swap.mdl).unwrap();
            camera_checks(&p3, &sk, &ann, &img, &body, &base.join(format!("{name}_{tag}")));
        }
        if which.contains("recom") {
            // the skin as it is: no new hair; drawn with the same tool (its hair on its own textures)
            let sk = RecomSkeleton::parse(&mdl0).unwrap();
            let ann = crate::ann::Ann::parse(&ann0).unwrap();
            let mut p3 = MdlPart::parse_bytes(&split_parts(&mdl0).unwrap()[3]).unwrap();
            let rtm_p = crate::rtm::Rtm::parse(&rtm).unwrap();
            let mut body: Vec<crate::texture::Image> = p3.textures.iter().map(|n| {
                let n = String::from_utf8_lossy(n.split(|&b| b == 0).next().unwrap_or(&[])).to_string();
                let e = rtm_p.entries.iter().find(|e| e.name_str().trim_end_matches(".tm2") == n.trim_end_matches(".tm2")).unwrap();
                let (w, h, px) = crate::rtm::tim2_decode8(&e.data).unwrap();
                crate::texture::Image { w, h, px }
            }).collect();
            let a = p3.texture_names().iter().position(|n| n == "n_zz120_01").unwrap() as u32;
            for k in p3.list_a.iter_mut().chain(p3.list_b.iter_mut().flatten()) { for v in k.verts.iter_mut() { if let Some(q) = v.q2.as_mut() { if q[3] == a { q[3] = 1; } else if q[3] == 1 { q[3] = a; } } } }
            body.swap(1, a as usize);
            // the face's texture from its HD asset, as `view_at` draws it (model-reviewer of 0.198.7: the SD picture here made every
            // face close-up differ)
            let img = hd.iter().find(|x| x.name == crate::keyblade::hd_name("n_zz120_01")).and_then(|x| crate::texture::decode_dds(&x.data).ok()).unwrap_or_else(|| body[1].clone());
            camera_checks(&p3, &sk, &ann, &img, &body, &base.join(format!("recom_{tag}")));
        }
    }

    /// The owner's three in-game checks as renders (see [`render_body_cam`]), into `dir`: the run (slot 3) at four frames from behind
    /// with the camera low (`run_low_*`) and high (`run_high_*`), and the idle (slot 1, frame 25) circled every 45 deg a little from
    /// below (`orbit_*`); and `sheet.png`, all of them side by side (12 + 8 cells).
    pub(crate) fn camera_checks(p3: &MdlPart, sk: &RecomSkeleton, ann: &crate::ann::Ann, atlas: &crate::texture::Image, body: &[crate::texture::Image], dir: &std::path::Path) {
        std::fs::create_dir_all(dir).unwrap();
        let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
        let mut cells: Vec<std::path::PathBuf> = vec![];
        if let Some(c) = ann.slots.get(3).and_then(|s| s.clip.as_ref()) {
            for (tag, pitch) in [("low", -35.0), ("high", 35.0)] {
                for f in [0.0, 6.0, 11.0, 17.0] {
                    let pth = dir.join(format!("run_{tag}_f{f}.png"));
                    render_cam(p3, &posed_world(sk, c, f), &names, atlas, body, 180.0, pitch, true, &pth);
                    cells.push(pth);
                }
            }
        }
        if let Some(c) = ann.slots.get(1).and_then(|s| s.clip.as_ref()) {
            let w = posed_world(sk, c, 25.0);
            for k in 0..8 {
                let pth = dir.join(format!("orbit_{:03}.png", k * 45));
                render_cam(p3, &w, &names, atlas, body, k as f64 * 45.0, -15.0, true, &pth);
                cells.push(pth);
            }
        }
        // 0.198.3 (in every camera check since 0.198.7): the run from its sides, and the nape from below
        if let Some(c) = ann.slots.get(3).and_then(|s| s.clip.as_ref()) {
            for f in [0.0, 6.0, 11.0, 17.0] {
                let w = posed_world(sk, c, f);
                for yaw in [90.0, 270.0] {
                    render_cam(p3, &w, &names, atlas, body, yaw, 0.0, true, &dir.join(format!("run_side_f{f}_y{yaw}.png")));
                }
                if f == 6.0 || f == 17.0 {
                    for pitch in [-15.0, -25.0] {
                        render_cam(p3, &w, &names, atlas, body, 180.0, pitch, true, &dir.join(format!("nape_f{f}_p{pitch}.png")));
                    }
                }
            }
        }
        // 0.198.6: the walk (slot 2; the owner tested walking and running with the camera low): from behind low / high and both sides
        if let Some(c) = ann.slots.get(2).and_then(|s| s.clip.as_ref()) {
            for f in [0.0, (c.frames / 4) as f64, (c.frames / 2) as f64, (3 * c.frames / 4) as f64] {
                let w = posed_world(sk, c, f);
                for (tag, yaw, pitch) in [("low", 180.0, -35.0), ("high", 180.0, 35.0), ("side90", 90.0, 0.0), ("side270", 270.0, 0.0)] {
                    render_cam(p3, &w, &names, atlas, body, yaw, pitch, true, &dir.join(format!("walk_{tag}_f{f}.png")));
                }
                // the owner's near-level camera behind (model-reviewer, 0.198.6: no walk nape view)
                let neck = names.iter().position(|n| n == "neck3").unwrap_or(0);
                for pitch in [-15.0, -25.0, 0.0] {
                    render_focus(p3, &w, &names, atlas, body, 180.0, pitch, (w[neck].t, 0.16), &dir.join(format!("closeup_back_walk_f{f}_p{pitch}.png")));
                }
            }
        }
        // 0.198.5: close-ups (the owner's two misses on 0.198.4): the face from the front and a little to each side, the nape and the
        // back from behind (idle and run), at 1.5 px per mm; `closeup_*.png`, not in the sheet
        let head = names.iter().position(|n| n == "head").unwrap();
        let neck = names.iter().position(|n| n == "neck3").unwrap_or(head);
        if let Some(c) = ann.slots.get(1).and_then(|s| s.clip.as_ref()) {
            for f in [0.0, 25.0, 50.0, 75.0] {
                let w = posed_world(sk, c, f);
                let face = w[head].point([0.0, 0.05, 0.09]);
                for yaw in [0.0, 25.0, -25.0] {
                    render_focus(p3, &w, &names, atlas, body, yaw, 0.0, (face, 0.11), &dir.join(format!("closeup_face_idle_f{f}_y{yaw}.png")));
                }
                // 0.198.13 (fork owner: two sideburns side by side in front of the ear): the face from each side
                let side = w[head].point([0.0, 0.03, 0.03]);
                for yaw in [90.0, 270.0] {
                    render_focus(p3, &w, &names, atlas, body, yaw, 0.0, (side, 0.09), &dir.join(format!("closeup_face_side_idle_f{f}_y{yaw}.png")));
                }
                render_focus(p3, &w, &names, atlas, body, 180.0, 5.0, (w[neck].t, 0.16), &dir.join(format!("closeup_back_idle_f{f}.png")));
            }
        }
        if let Some(c) = ann.slots.get(3).and_then(|s| s.clip.as_ref()) {
            for f in [0.0, 6.0, 11.0, 17.0] {
                let w = posed_world(sk, c, f);
                let face = w[head].point([0.0, 0.05, 0.09]);
                render_focus(p3, &w, &names, atlas, body, 0.0, 0.0, (face, 0.11), &dir.join(format!("closeup_face_run_f{f}.png")));
                let side = w[head].point([0.0, 0.03, 0.03]);
                for yaw in [90.0, 270.0] {
                    render_focus(p3, &w, &names, atlas, body, yaw, 0.0, (side, 0.09), &dir.join(format!("closeup_face_side_run_f{f}_y{yaw}.png")));
                }
                render_focus(p3, &w, &names, atlas, body, 180.0, 5.0, (w[neck].t, 0.16), &dir.join(format!("closeup_back_run_f{f}.png")));
            }
        }
        if std::env::var("VIEWS").is_ok() {
            return;
        }
        // the sheet: 4 per row, 350 px cells
        let cell = 350usize;
        let rows = cells.len().div_ceil(4);
        let mut sheet = vec![0u8; 4 * cell * rows * cell * 3];
        for (i, pth) in cells.iter().enumerate() {
            let img = crate::texture::decode_png(&std::fs::read(pth).unwrap()).unwrap();
            let (ox, oy) = ((i % 4) * cell, (i / 4) * cell);
            for y in 0..cell {
                for x in 0..cell {
                    let p = img.px[(y * img.h / cell) * img.w + x * img.w / cell];
                    let o = 3 * ((oy + y) * 4 * cell + ox + x);
                    sheet[o..o + 3].copy_from_slice(&p[..3]);
                }
            }
        }
        std::fs::write(dir.join("sheet.png"), dt::png(4 * cell, rows * cell, &sheet)).unwrap();
    }




}






#[cfg(test)]
mod dark_probe {
    use super::*;
    use crate::ddd_hair::{with_hair, HairSource, HairTarget};
    use crate::install;

    /// 0.198.7 probe (fork owner: "implementa todos os cabelos para a Skin do riku Darkmode tbm"): where the KH2 and BBS hair swaps
    /// stop on Dark Riku's model (`PL0201`, `PL0211`).
    #[test]
    fn dark_probe() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        for grp in ["0201", "0211"] {
            let mdl = install::read_recom(&arc, &keys, &format!("PLAYER/{grp}/PL{grp}.MDL")).unwrap();
            let e = arc.find(&format!("PLAYER/{grp}/PL{grp}.RTM")).unwrap();
            let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
            let r = crate::rtm::Rtm::parse(&rtm).unwrap();
            eprintln!("{grp}: RTM {:?}; HD {:?}", r.entries.iter().map(|e| e.name_str()).collect::<Vec<_>>(), hd.iter().map(|a| a.name.clone()).collect::<Vec<_>>());
            let parts = crate::mdl::split_parts(&mdl).unwrap();
            for (i, p) in parts.iter().enumerate() {
                match crate::mdl::MdlPart::parse_bytes(p) {
                    Ok(pp) => eprintln!("  part {i}: {} bones, textures {:?}, list A {} packets, list B {:?}", pp.bones.len(), pp.texture_names(), pp.list_a.len(), pp.list_b.as_ref().map(|l| l.len())),
                    Err(e) => eprintln!("  part {i}: parse error {e}"),
                }
            }
            for (i, p) in parts.iter().enumerate() {
                let pp = crate::mdl::MdlPart::parse_bytes(p).unwrap();
                let back = pp.write_bytes().unwrap();
                eprintln!("  part {i}: write(parse) == original: {}", back == *p);
            }
            {
                let sk = crate::mdl::RecomSkeleton::parse(&mdl).unwrap();
                let p3 = crate::mdl::MdlPart::parse_bytes(&parts[3]).unwrap();
                let tn = p3.texture_names();
                let mut by: std::collections::BTreeMap<(String, String), usize> = Default::default();
                for pk in p3.list_b.as_ref().unwrap() { for v in &pk.verts { *by.entry((sk.bones[v.bone()].name.clone(), v.q2.map_or("-".to_string(), |q| tn.get(q[3] as usize).cloned().unwrap_or_default()))).or_default() += 1; } }
                eprintln!("  list B vertices by (bone, texture): {by:?}");
                let mut ha: std::collections::BTreeMap<String, usize> = Default::default();
                for pk in &p3.list_a { for v in &pk.verts { if sk.bones[v.bone()].name.contains("hair") { *ha.entry(v.q2.map_or("-".to_string(), |q| tn.get(q[3] as usize).cloned().unwrap_or_default())).or_default() += 1; } } }
                eprintln!("  list A hair-bone vertices by texture: {ha:?}");
            }
            {
                // 0.198.7 review: `_02` triangles with a head / neck corner, by place (head-relative centroid) and mean texel
                let sk = crate::mdl::RecomSkeleton::parse(&mdl).unwrap();
                let w = sk.rest_world();
                let hd = sk.find("head").unwrap();
                let p3 = crate::mdl::MdlPart::parse_bytes(&parts[3]).unwrap();
                let t2 = p3.texture_names().iter().position(|n| n.ends_with("_02") || n.ends_with("_12")).unwrap() as u32;
                let rt = crate::rtm::Rtm::parse(&rtm).unwrap();
                let name = &p3.texture_names()[t2 as usize];
                let e = rt.entries.iter().find(|e| e.name_str().trim_end_matches(".tm2") == name).unwrap();
                let (tw, th, px) = crate::rtm::tim2_decode8(&e.data).unwrap();
                let mut rows = vec![];
                for t in crate::ddd_hair::tests::collect(&p3) {
                    if t.tex != t2 { continue; }
                    let bn: Vec<String> = t.v.iter().map(|v| sk.bones[v.bone()].name.clone()).collect();
                    if !bn.iter().any(|n| n == "head" || n.starts_with("neck")) { continue; }
                    let q: Vec<[f64; 3]> = t.v.iter().map(|v| w[v.bone()].point(v.pos().map(|x| x as f64))).collect();
                    let c = [0, 1, 2].map(|k| (q[0][k] + q[1][k] + q[2][k]) / 3.0 - w[hd].t[k]);
                    let uv = t.v.iter().map(|v| v.uv().unwrap()).fold([0.0f32; 2], |a, u| [a[0] + u[0] / 3.0, a[1] + u[1] / 3.0]);
                    let tx = px[((uv[1].rem_euclid(1.0) * th as f32) as usize).min(th - 1) * tw + ((uv[0].rem_euclid(1.0) * tw as f32) as usize).min(tw - 1)];
                    rows.push(format!("  _02 tri bones {bn:?} c ({:.3},{:.3},{:.3}) uv ({:.3},{:.3}) texel {:?}", c[0], c[1], c[2], uv[0], uv[1], &tx[..3]));
                }
                eprintln!("  _02 head/neck triangles: {}", rows.len());
                for r in rows { eprintln!("{r}"); }
            }
            let src = Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap();
            let (t1, t2): (&'static str, &'static str) = if grp == "0201" { ("n_zz120_01", "n_zz120_02") } else { ("n_zz120_11", "n_zz120_12") };
            let tg = HairTarget { texture: t1, new_texture: "rk_hair_kh2", also: Some(t2), neck_skirt: false };
            match src.prepare_mdl(&mdl).and_then(|m| with_hair(&m, &rtm, &hd, &tg, &src)) {
                Ok(s) => {
                    eprintln!("  KH2: ok, removed {} (of {} on the atlas), skirt {}, nape {}", s.report.removed, s.report.riku_on_atlas, s.report.skirt, s.report.nape);
                    let sk = crate::mdl::RecomSkeleton::parse(&s.mdl).unwrap();
                    let w = sk.rest_world();
                    let hd = sk.find("head").unwrap();
                    let p3 = crate::mdl::MdlPart::parse_bytes(&crate::mdl::split_parts(&s.mdl).unwrap()[3]).unwrap();
                    let t2i = p3.texture_names().iter().position(|n| n == t2).unwrap() as u32;
                    for t in crate::ddd_hair::tests::collect(&p3) {
                        if t.tex != t2i { continue; }
                        let bn: Vec<String> = t.v.iter().map(|v| sk.bones[v.bone()].name.clone()).collect();
                        if !bn.iter().any(|n| n == "head" || n.starts_with("neck")) { continue; }
                        let q: Vec<[f64; 3]> = t.v.iter().map(|v| w[v.bone()].point(v.pos().map(|x| x as f64))).collect();
                        let c = [0, 1, 2].map(|k| (q[0][k] + q[1][k] + q[2][k]) / 3.0 - w[hd].t[k]);
                        eprintln!("  LEFT _02 {bn:?} c ({:.3},{:.3},{:.3})", c[0], c[1], c[2]);
                    }
                }
                Err(e) => eprintln!("  KH2: {e}"),
            }
        }
    }
}


#[cfg(test)]
mod game_files_probe {
    /// 0.198.7 (fork owner, in game: KH2's hair on the Dark Form with long straight sticks, "só aconteceu com o cabelo do KH2 no darkmode"):
    /// the group the game loaded (`GAME_GROUP_DIR` = the folder of `PL<g>.MDL` / `.ANN`): bones, clips' bone counts, and per hair bone
    /// the farthest its origin goes from the head in each clip's frames.
    #[test]
    fn game_group_probe() {
        let Ok(dir) = std::env::var("GAME_GROUP_DIR") else { return };
        let dir = std::path::PathBuf::from(dir);
        let g = dir.file_name().unwrap().to_string_lossy().to_string();
        let mdl = std::fs::read(dir.join(format!("PL{g}.MDL"))).unwrap();
        let ann = crate::ann::Ann::parse(&std::fs::read(dir.join(format!("PL{g}.ANN"))).unwrap()).unwrap();
        let sk = crate::mdl::RecomSkeleton::parse(&mdl).unwrap();
        let head = sk.find("head").unwrap();
        eprintln!("model {} bones; clips {}", sk.bones.len(), ann.slots.iter().filter(|s| s.clip.is_some()).count());
        let mut counts: std::collections::BTreeMap<i16, usize> = Default::default();
        for s in &ann.slots { if let Some(c) = &s.clip { *counts.entry(c.bones).or_default() += 1; } }
        eprintln!("clip bone counts {counts:?}");
        for (i, pb) in crate::mdl::split_parts(&mdl).unwrap().iter().enumerate() {
            let p = crate::mdl::MdlPart::parse_bytes(pb).unwrap();
            let qw = |l: &[crate::mdl::MeshPacket]| l.iter().map(|k| 2 + if k.fmt == crate::mdl::FMT_TEXTURED { 3 } else { 2 } * k.verts.len()).collect::<Vec<_>>();
            let a = qw(&p.list_a);
            let b = p.list_b.as_ref().map(|l| qw(l)).unwrap_or_default();
            let budget = 2 * (0xF3 - p.bones.len());
            let distinct = |l: &[crate::mdl::MeshPacket]| l.iter().map(|k| k.verts.iter().map(|v| v.bone()).collect::<std::collections::BTreeSet<_>>().len()).collect::<Vec<_>>();
            let da = distinct(&p.list_a);
            eprintln!("part {i}: distinct bones per packet: largest {}, over 48: {}; packets with a bone >= 128: {}", da.iter().max().unwrap_or(&0), da.iter().filter(|&&x| x > 48).count(), p.list_a.iter().filter(|k| k.verts.iter().any(|v| v.bone() >= 128)).count());
            eprintln!("part {i}: {} bones, budget {budget} qw; list A largest {} ({} over); list B largest {} ({} over)", p.bones.len(), a.iter().max().unwrap_or(&0), a.iter().filter(|&&x| x > budget).count(), b.iter().max().unwrap_or(&0), b.iter().filter(|&&x| x > budget).count());
        }
        let mut worst: Vec<(f64, String, usize, usize)> = vec![];
        for (slot, s) in ann.slots.iter().enumerate() {
            let Some(c) = &s.clip else { continue };
            if c.bones as usize != sk.bones.len() { continue; }
            for f in (0..c.frames.max(1) as usize).step_by(5) {
                let w = super::tests::posed_world(&sk, c, f as f64);
                for (b, bone) in sk.bones.iter().enumerate() {
                    if !bone.name.contains("hair") { continue; }
                    let d = crate::math::dist(w[b].t, w[head].t);
                    if !d.is_finite() || d > 0.4 { worst.push((d, bone.name.clone(), slot, f)); }
                }
            }
        }
        worst.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut seen = std::collections::BTreeSet::new();
        for (d, n, slot, f) in worst { if seen.insert((n.clone(), slot)) && seen.len() < 40 { eprintln!("  {n} slot {slot} frame {f}: {d:.2} m from the head"); } }
    }
}

#[cfg(test)]
mod high_bone_probe {
    use crate::install;
    /// 0.198.7: do Re:CoM's own models skin vertices to bones 128 and up? (the KH2 hair on the Dark Form is the mod's first that does)
    #[test]
    fn game_models_with_vertices_on_high_bones() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        for f in ["PLAYER/0001/PL0001.MDL", "PLAYER/0405/PL0405.MDL", "PLAYER/0407/PL0407.MDL", "ENEMY/0117/EN0117.MDL", "ENEMY/0114/EN0114.MDL", "NPC/0051/NP0051.MDL"] {
            let Ok(mdl) = install::read_recom(&arc, &keys, f) else { eprintln!("{f}: not read"); continue };
            let Ok(parts) = crate::mdl::split_parts(&mdl) else { continue };
            for (i, pb) in parts.iter().enumerate() {
                let Ok(p) = crate::mdl::MdlPart::parse_bytes(pb) else { eprintln!("{f} part {i}: not parsed"); continue };
                let hi = p.list_a.iter().chain(p.list_b.iter().flatten()).flat_map(|k| k.verts.iter().map(|v| v.bone())).max().unwrap_or(0);
                let n = p.list_a.iter().chain(p.list_b.iter().flatten()).flat_map(|k| k.verts.iter()).filter(|v| v.bone() >= 128).count();
                eprintln!("{f} part {i}: {} bones, highest skinned bone {hi}, vertices on bones >= 128: {n}", p.bones.len());
            }
        }
    }
}

