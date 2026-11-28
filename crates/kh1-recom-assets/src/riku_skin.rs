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
    retarget_with(src, riku, dark, map, None)
}

/// [`retarget`] with the target's bones that have no source bone posed from `fill` at its frame 0
/// (their local rotation and translation), else from their rest pose. Used for Sora's KH1 Guard on
/// Riku (0.157.0): his blade chain, belts and hair from his own battle stance.
pub fn retarget_with(src: &Clip, riku: &RecomSkeleton, dark: &RecomSkeleton, map: &[Option<usize>], fill: Option<&Clip>) -> Result<Clip> {
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
                None => {
                    let r = match fill {
                        Some(c) => {
                            let s = c.sample(j, 0.0);
                            rot_xyz(s.r[0], s.r[1], s.r[2])
                        }
                        None => unscale(&b.local.r).0,
                    };
                    m3(&r, &tg[b.parent as usize].u)
                }
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
                    (_, None) if fill.is_some() => fill.unwrap().sample(j, 0.0).t,
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
/// [`encode`] for another module (`crate::pam`): rows of local (translation, rotation radians, scale) per frame, the
/// clip's header fields from `template`.
pub(crate) fn encode_rows(template: &Clip, rows: &[Vec<[f64; 9]>]) -> Clip {
    encode(template, rows)
}

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

/// Riku's Guard clip (0.157.0 Sora's KH1 Guard retargeted; 0.158.0 his own KH1 guard).
///
/// 0.158.0 (fork owner, 2026-11-23: "no KH1 tem uma animação do Riku defendendo, ela acontece na
/// Destiny Island … quando ele duela com o Sora usando uma espada de madeira … inclusive tem a
/// variação de quando em defesa ele toma impacto"): KH1's Riku (`xa_ex_1010`, the Destiny Islands
/// duel set) guards in his AI script's `func_1e62` (`ex_1010.bd`, Checked (files: the script listed
/// with `tools/kh1-ai-script.py`)): request **240** (entry 28, 30 frames: into the guard), then
/// **241** (entry 29, 60 frames, played looping: the guard held), with his guard row `0x550` (type
/// 2, priority 10) on; the row's recoil is request **110** (entry 13), the same clip Re:CoM baked as
/// his `p002_043` (row 0x24), which his Guard already plays on a block. The clip here is 240's 30
/// frames then 241's first 24 (54 frames, the length of Sora's KH1 Guard, whose numbers the KH1
/// frame uses), converted from KH1's motions onto Riku's Re:CoM skeleton (`ex_1010_sk`, KH1's own:
/// `crate::convert::Converter`).
pub const RIKU_GUARD: &str = "p002_225";
/// KH1 Riku's guard requests: into the guard, the guard held.
pub const RIKU_GUARD_REQUESTS: (u16, u16) = (240, 241);
/// The clip's length: Sora's KH1 Guard (54 frames), whose timing the KH1 frame uses.
pub const RIKU_GUARD_FRAMES: usize = 54;
/// Frames of the held part after the way in (0.158.1: 7, then the way out).
pub const RIKU_GUARD_HOLD: u32 = 27;
/// Frames 240 (the way into the guard) is played in: KH1's guard object's first live frame.
pub const RIKU_GUARD_UP: usize = 10;

/// KH1 Riku's own guard on a Riku skeleton of Re:CoM's (named [`RIKU_GUARD`]).
pub fn riku_kh1_guard(mset: &[u8], mdls: &[u8], riku: &RecomSkeleton) -> Result<Clip> {
    let ms = kh1_formats::mset::MotionSet::parse(mset)?;
    let (into, held) = RIKU_GUARD_REQUESTS;
    let (_, m_in) = ms.motion_for(into).ok_or_else(|| crate::Error::Format(format!("xa_ex_1010.mset: no request {into}")))?;
    let (_, m_hold) = ms.motion_for(held).ok_or_else(|| crate::Error::Format(format!("xa_ex_1010.mset: no request {held}")))?;
    let conv = crate::convert::Converter::new(mdls, riku.clone())?;
    let mut rig = conv.rig();
    let mut rows: Vec<Vec<[f64; 9]>> = Vec::new();
    rig.start_motion(m_in);
    for f in 0..m_in.frames {
        let tg = conv.targets(&mut rig, m_in, f as f32, false);
        let l = conv.locals(&tg, rows.last().map(|v| v.as_slice()));
        rows.push(l);
    }
    let into: Vec<Vec<[f64; 9]>> = rows.clone();
    // 0.159.0 (fork owner: "Tempo/duração estranhos"): the guard is up when KH1's guard object goes
    // live (frame 10 of the Guard's 54, live 10..36): 240 is played in its first 10 frames, 241
    // held over the live window, then the way out.
    rows.clear();
    let last_in = into.len().saturating_sub(1);
    for k in 0..RIKU_GUARD_UP {
        rows.push(into[((k + 1) * last_in + RIKU_GUARD_UP / 2) / RIKU_GUARD_UP].clone());
    }
    rig.start_motion(m_hold);
    for f in 0..RIKU_GUARD_HOLD.min(m_hold.frames) {
        let tg = conv.targets(&mut rig, m_hold, f as f32, false);
        let l = conv.locals(&tg, rows.last().map(|v| v.as_slice()));
        rows.push(l);
    }
    // 0.158.1 (fork owner: "quando passa um tempo e ele desativa a guarda ficou estranha, sera que
    // nn tem como colocar a mesma animação mas ao contrário?"): the way out is the way in played
    // backwards, faster, to fill the clip: it ends on 240's first frame (his stance before the
    // guard), so the next motion starts from there instead of the held guard.
    // 0.180.1 (fork owner: "a defesa acaba antes dele baixar a guarda"; answer: the animation follows
    // the defence): 240 settles into the guard well before its last frame, so its tail played
    // backwards still showed the guard up for 3 frames past the live window (10..36) and the
    // weapon hand only came down by frame 46. The way out now starts at the frame where 240
    // reaches the guard ([`settled_from`]), so the guard comes down as the defence ends.
    let out = RIKU_GUARD_FRAMES.saturating_sub(rows.len()).max(1);
    let arm: Vec<usize> = GUARD_ARM.iter().filter_map(|b| riku.find(b)).collect();
    let last = settled_from(&into, &arm);
    for k in 0..out {
        let src = last - ((k + 1) * last + out / 2) / out;
        rows.push(into[src].clone());
    }
    let spec = crate::convert::ClipSpec { name: RIKU_GUARD.into(), looped: false, root_motion: false, blend: 8 };
    Ok(crate::convert::encode_clip(&spec, &rows))
}

/// The bones that raise the guard: the weapon arm (Riku's skeleton names; Checked (files)).
const GUARD_ARM: [&str; 5] = ["r_collar", "r_uparm", "r_foarm", "rx_rot", "r_hand"];

/// The first frame of a motion's rows from which `bones` no longer move much: within [`SETTLED`] of
/// the whole motion's change of those bones (the summed absolute difference of their local values
/// from the last frame; all bones when `bones` is empty). For 240 (into the guard) and the weapon
/// arm, the frame the guard is up.
fn settled_from(rows: &[Vec<[f64; 9]>], bones: &[usize]) -> usize {
    let last = rows.len().saturating_sub(1);
    let pick = |f: usize| -> Vec<(&[f64; 9], &[f64; 9])> {
        if bones.is_empty() {
            rows[f].iter().zip(&rows[last]).collect()
        } else {
            bones.iter().filter_map(|&b| Some((rows[f].get(b)?, rows[last].get(b)?))).collect()
        }
    };
    let dist = |f: usize| -> f64 { pick(f).iter().map(|(a, b)| a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum::<f64>()).sum() };
    let total = dist(0);
    if total <= 0.0 {
        return last;
    }
    (0..=last).find(|&f| dist(f) <= SETTLED * total).unwrap_or(last).max(1)
}

/// The share of 240's change still left where it counts as settled into the guard.
const SETTLED: f64 = 0.05;

/// KH1 Riku's guard recoil (request 110, entry 13, 56 frames; 0.158.2), named `p002_226`.
pub const RIKU_RECOIL: &str = "p002_226";

/// KH1 Riku's own guard recoil on a Riku skeleton of Re:CoM's (named [`RIKU_RECOIL`]).
pub fn riku_kh1_recoil(mset: &[u8], mdls: &[u8], riku: &RecomSkeleton) -> Result<Clip> {
    let ms = kh1_formats::mset::MotionSet::parse(mset)?;
    let (_, m) = ms.motion_for(110).ok_or_else(|| crate::Error::Format("xa_ex_1010.mset: no request 110".into()))?;
    let conv = crate::convert::Converter::new(mdls, riku.clone())?;
    conv.convert(m, &crate::convert::ClipSpec { name: RIKU_RECOIL.into(), looped: false, root_motion: false, blend: 8 })
}

/// The guard's sources for every Riku skeleton: KH1 Riku's motion set and model.
pub struct GuardSource<'a> {
    /// `xa_ex_1010.mset`.
    pub mset: &'a [u8],
    /// `xa_ex_1010.mdls`.
    pub mdls: &'a [u8],
    /// 0.184.7: KH1 Riku-Ansem's `xa_ex_1580.mset` / `.mdls` (his Stop motion), when read.
    pub ansem: Option<(&'a [u8], &'a [u8])>,
    /// 0.189.0: BBS FM Terra's skeleton and clips (`crate::terra::read_terra`), the source of Riku's cast and Block
    /// motions; None when the game's BBS files weren't read (then the casts keep their Re:CoM / KH1 motions and the Guard
    /// is KH1 Riku's).
    pub terra: Option<(&'a RecomSkeleton, &'a [crate::pam::PamClip])>,
}

/// Sora's KH1 Guard on Riku (0.157.0; replaced by [`riku_kh1_guard`] in 0.158.0, kept for tests).
/// Riku's battle stance (his action row 0x2A). Checked (exe: `0x1407C6370`).
pub const RIKU_STANCE: &str = "p002_031";
/// 0.198.13: Riku's fall (row 6), what plays after his air casts; Dark Riku's battle stance (his row 0x2A) and fall (row 6).
/// Checked (exe: the action tables `0x1407C6370` / `0x1407C67F0`).
pub const RIKU_FALL: &str = "p002_006";
/// Dark Riku's battle stance (his row 0x2A), what plays after his ground casts.
pub const DARK_STANCE: &str = "p003_021";
/// Dark Riku's fall (his row 6), what plays after his air casts.
pub const DARK_FALL: &str = "p003_007";

/// Sora's bone of the same role as Riku's.
fn sora_alias(riku: &str) -> &str {
    match riku {
        "ex_1010_sk" => "ex_0010_sk",
        x => x,
    }
}

/// Sora's guard clip on Riku's skeleton, named [`RIKU_GUARD`].
pub fn riku_guard(sora_guard: &Clip, sora: &RecomSkeleton, riku: &RecomSkeleton, riku_ann: &Ann) -> Result<Clip> {
    let map: Vec<Option<usize>> = riku.bones.iter().map(|b| sora.find(sora_alias(&b.name))).collect();
    let stance = riku_ann.slots.iter().find_map(|s| s.clip.as_ref().filter(|c| c.name_str() == RIKU_STANCE));
    let mut c = retarget_with(sora_guard, sora, riku, &map, stance)?;
    c.name = crate::ann::name16(RIKU_GUARD);
    Ok(c)
}

/// 0.183.2: Dark Riku's (the real Dark Mode's) guard and guard recoil: [`RIKU_GUARD`] / [`RIKU_RECOIL`]
/// retargeted from Riku's skeleton onto his (as the Dark Form skin does, [`retarget`]), named for his own
/// pack's prefix.
pub const DARK_GUARD: &str = "p003_225";
/// Dark Riku's guard recoil (see [`DARK_GUARD`]).
pub const DARK_RECOIL: &str = "p003_226";

/// Dark Riku's own pack (`dark_ann`, `PLAYER/02x1/PL02x1.ANN`) with KH1 Riku's guard and its recoil on his
/// skeleton ([`DARK_GUARD`], [`DARK_RECOIL`]). The DLL names two of his action table's clipless rows
/// (`0x1407C67F0`, 0x39 / 0x3A: `p003_000`, Checked (exe)) after them.
pub fn dark_guard_pack(dark_ann: Ann, guard: &Clip, recoil: &Clip, riku: &RecomSkeleton, dark: &RecomSkeleton) -> Result<Ann> {
    let map = bone_map(riku, dark);
    let mut g = retarget(guard, riku, dark, &map)?;
    g.name = crate::ann::name16(DARK_GUARD);
    let mut r = retarget(recoil, riku, dark, &map)?;
    r.name = crate::ann::name16(DARK_RECOIL);
    Ok(with_clip(with_clip(dark_ann, g), r))
}

/// 0.184.1: Riku's cast motions for Sora's KH1 spells. 0.184.4 (fork owner, 2026-11-23: "as animações precisam variar";
/// the table they approved, "aprovo, implemente"): one motion per spell, Riku's own sleight clips where he has one
/// (his skeleton, no retarget), Dark Riku's otherwise (retargeted onto Riku); Dark Riku plays his own versions. Each is
/// retimed so its key moment (the thrust, the raise, the spin) falls on the frame KH1's spell releases, and lasts KH1's
/// motion (Sora's `xa_ex_0010.mset`, Checked (files)); the clips' travel is taken out (Root and the hips keep their rest
/// x / z). The key moments were read from each clip's hands and hips (the owner's files, Checked (files)); that they
/// read as a cast in game: Assumption.
pub const DARK_FIRAGA: &str = "p003_043";
/// The Dark Firaga clip's charge end and thrust end (its events are placed by them).
const DF_CHARGE_END: f64 = 32.0;
const DF_THRUST_END: f64 = 36.0;

/// The skeleton a source clip is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Skel {
    /// Riku's (`PL0101.MDL`, `ex_1010_sk`).
    Riku,
    /// Dark Riku's (`PL0201.MDL`, `ex_1580_sk`; KH1's Riku-Ansem's too).
    Dark,
    /// Sora's (`PL0001.MDL`, `ex_0010_sk`).
    Sora,
    /// 0.189.0: BBS FM Terra's (`p03ex00.pmo`, 51 bones; `crate::terra`).
    Terra,
}

/// Where a source motion comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Src {
    /// A clip of a Re:CoM PLAYER group's pack (group, clip name) on a skeleton.
    Pack(u16, &'static str, Skel),
    /// 0.184.7: a request of KH1 Riku-Ansem's motion set (`xa_ex_1580.mset`), converted onto Dark Riku's skeleton.
    Kh1Ansem(u16),
    /// 0.189.0: a clip of Terra's packs (`crate::terra::PACKS`) by name, on his skeleton.
    Terra(&'static str),
}

/// One source motion: Riku's and Dark Riku's, their rate, and in the source's own frames the key moment's start and
/// end and the last frame used; the motion used when a KH1 one can't be read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CastSource {
    /// Riku's motion.
    pub riku: Src,
    /// Dark Riku's motion.
    pub dark: Src,
    /// The source's frames per second (Re:CoM's sleight clips 30, KH1's motions 60).
    pub fps: f64,
    /// 0.184.9: the first source frame used (a leap or a turn before it is left out).
    pub start: f64,
    /// The key moment's first frame (the charge ends there).
    pub charge_end: f64,
    /// The key moment's last frame.
    pub key_end: f64,
    /// The clip's last frame used.
    pub last: f64,
    /// The source used instead when this one's files aren't there ([`CAST_SOURCES`] index).
    pub fallback: Option<usize>,
}

/// The motions (Checked (files): the clips' frames, hands and hips on the owner's install):
/// 0 Dark Firaga (`PL1088` `p003_043`, 52 f: crouched charge, the left hand's thrust 32..36);
/// 1 Impulse (Riku `PL1301` `p002_061`, 34 f: a leap back to 8, then a crouched charge, the left hand's thrust 21..24;
///   Dark Impulse `PL1311` `p003_071`); 0.184.9: from frame 9, after the leap's landing (the owner: Blizzaga "estranho");
/// 2 Dark Aura's start (`PL1089` `p003_044`, 48 f: he rises with both arms spread, 24..32);
/// 3 0.184.9 (fork owner: "a animação do Sora de Curaga pro RIku ficou muito estranho tenta pensar fora da caixa"):
///   Riku's own item use (`PL0101` `p002_034`, his heal in Re:CoM, 56 f at 30, on the ground: a crouch 8..16, the left
///   hand raised over his head 18..22, held to 32, lowered), Dark Riku's from it retargeted;
/// 4 Barrage's high strike (Riku `PL1303` `p002_072`, 15 f: the right hand up, then down, 3..7; Dark Barrage `PL1313`
///   `p003_082`);
/// 5 0.184.7: KH1 Riku-Ansem's request 225 (`xa_ex_1580.mset`, 49 frames at 60 fps, on the ground: the left hand
///   reaches out at the shoulder 8..12 and holds there); without KH1's files, [7];
/// 6 0.184.9 (fork owner: "Aeroga tbm ta flicando um pouco"; Maelstrom's spin was in the air): KH1 Riku-Ansem's
///   request 223 (73 frames at 60, on the ground: both hands sweep, the left from behind to the front 40..48, the right
///   the other way); without KH1's files, [8];
/// 7 Barrage's crossing (Riku `p002_070`, 12 f: the arms cross, 3..6; Dark `p003_080`): Stop's fallback;
/// 8 Maelstrom's spin (Riku `PL1302` `p002_065`, 37 f: the spin, 10..16; Dark Maelstrom `PL1312` `p003_075`): Aero's
///   fallback.
pub const CAST_SOURCES: [CastSource; 19] = [
    CastSource { riku: Src::Pack(1088, DARK_FIRAGA, Skel::Dark), dark: Src::Pack(1088, DARK_FIRAGA, Skel::Dark), fps: 30.0, start: 0.0, charge_end: DF_CHARGE_END, key_end: DF_THRUST_END, last: 51.0, fallback: None },
    CastSource { riku: Src::Pack(1301, "p002_061", Skel::Riku), dark: Src::Pack(1311, "p003_071", Skel::Dark), fps: 30.0, start: 9.0, charge_end: 21.0, key_end: 24.0, last: 33.0, fallback: None },
    CastSource { riku: Src::Pack(1089, "p003_044", Skel::Dark), dark: Src::Pack(1089, "p003_044", Skel::Dark), fps: 30.0, start: 0.0, charge_end: 24.0, key_end: 32.0, last: 47.0, fallback: None },
    CastSource { riku: Src::Pack(101, ITEM_USE, Skel::Riku), dark: Src::Pack(101, ITEM_USE, Skel::Riku), fps: 30.0, start: 0.0, charge_end: 18.0, key_end: 22.0, last: 55.0, fallback: None },
    CastSource { riku: Src::Pack(1303, "p002_072", Skel::Riku), dark: Src::Pack(1313, "p003_082", Skel::Dark), fps: 30.0, start: 0.0, charge_end: 3.0, key_end: 7.0, last: 12.0, fallback: None },
    CastSource { riku: Src::Kh1Ansem(225), dark: Src::Kh1Ansem(225), fps: 60.0, start: 0.0, charge_end: 8.0, key_end: 12.0, last: 48.0, fallback: Some(7) },
    CastSource { riku: Src::Kh1Ansem(223), dark: Src::Kh1Ansem(223), fps: 60.0, start: 0.0, charge_end: 40.0, key_end: 48.0, last: 72.0, fallback: Some(8) },
    CastSource { riku: Src::Pack(1303, "p002_070", Skel::Riku), dark: Src::Pack(1313, "p003_080", Skel::Dark), fps: 30.0, start: 0.0, charge_end: 3.0, key_end: 6.0, last: 11.0, fallback: None },
    CastSource { riku: Src::Pack(1302, "p002_065", Skel::Riku), dark: Src::Pack(1312, "p003_075", Skel::Dark), fps: 30.0, start: 0.0, charge_end: 10.0, key_end: 16.0, last: 36.0, fallback: None },
    // 0.189.0: BBS FM Terra's casts (`315` thrust ground / `316` air: Fire, Blizzard; `317` raise ground / `318` air: the rest;
    // the workspace's `knowledge/BBS/bbs-terra-casts-guard-for-riku.md` section 7.2, Checked (decomp, files); the air pair
    // is an Assumption). Each with the 0.184.x motion of its spell as the fallback without BBS's files.
        TERRA_THRUST_G[0], TERRA_THRUST_G[1], TERRA_RAISE_G[0], TERRA_RAISE_G[1], TERRA_RAISE_G[2],
    TERRA_THRUST_A[0], TERRA_THRUST_A[1], TERRA_RAISE_A[0], TERRA_RAISE_A[1], TERRA_RAISE_A[2],
];

const fn terra(name: &'static str, charge_end: f64, key_end: f64, last: f64, fallback: usize) -> CastSource {
    CastSource { riku: Src::Terra(name), dark: Src::Terra(name), fps: 30.0, start: 0.0, charge_end, key_end, last, fallback: Some(fallback) }
}
/// Terra's thrust on the ground (Fire's, Blizzard's fallbacks) and in the air; the raise (Thunder / Aero's, Cure's,
/// Gravity / Stop's fallbacks).
const TERRA_THRUST_G: [CastSource; 2] = [terra("315", 9.0, 13.0, 35.0, 0), terra("315", 9.0, 13.0, 35.0, 1)];
const TERRA_RAISE_G: [CastSource; 3] = [terra("317", 13.0, 17.0, 36.0, 2), terra("317", 13.0, 17.0, 36.0, 3), terra("317", 13.0, 17.0, 36.0, 5)];
/// The air rows play Terra's air clips (`316` the thrust, `318` the raise), with the blade in Riku's grip and the body at its
/// stance height. The key moments (in Terra's 30 fps frames) were read from the retargeted hands: 315 and 316 thrust at
/// 9..13 / 8..13, the raise at 13..17 (Checked (files)); that they read as a cast in game: Assumption.
/// 0.198.19 (fork owner: the spell effects not adjusted to Terra's casts, "algumas ficaram estranhas"): the raise was keyed
/// 10..14, from its lowest point (Terra's frame 10: the blade 0.57 m behind the body at 0.88 m), so on KH1's release frame,
/// where the DLL puts each shot's hand effect at the blade's end (Re:CoM's hand point `0x1401F0B30`), the blade was behind
/// Riku, and up only 5 frames later (frames 12 -> 13). Keyed from 13 (the blade up in front, 1.51 m, held to 18) the
/// swing up is the charge's end and the release meets the raised blade; KH1's release and free frames are unchanged.
const TERRA_THRUST_A: [CastSource; 2] = [terra("316", 8.0, 13.0, 35.0, 0), terra("316", 8.0, 13.0, 35.0, 1)];
const TERRA_RAISE_A: [CastSource; 3] = [terra("318", 13.0, 17.0, 36.0, 2), terra("318", 13.0, 17.0, 36.0, 3), terra("318", 13.0, 17.0, 36.0, 5)];

/// One cast clip: its name suffix (`p002_` / `p003_` + it), its source motion, KH1's release frame, length and free
/// frame (the recovery ends there; held after).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CastClip {
    /// The clip name's number (`p002_` / `p003_` + it).
    pub suffix: &'static str,
    /// Its motion ([`CAST_SOURCES`]).
    pub src: usize,
    /// KH1's release frame (the key moment starts on it).
    pub release: f64,
    /// KH1's motion length in frames.
    pub len: usize,
    /// KH1's free frame (the recovery ends there).
    pub free: usize,
}

/// Cure's clip suffixes, ground and air (their effects are the DLL's heal sparkle, [`cast_events`]).
pub const CURE_CLIPS: [&str; 2] = ["231", "237"];

/// The twelve clips (0.185.0), in the order of the action rows the DLL names (`crate::riku::CAST_ROWS` 0x3B..0x46): on
/// the ground Fire's wind-up (`0x36`, 28 frames: the charge only) and release (`0x3D`, 60, free 46, the ball at its frame
/// 0), Blizzard (`0x37`, 86, free 68, release 28), Thunder / Aero (`0x38` / `0x3C`, 68, release 30), Cure (`0x39`, 68,
/// release 36), Gravity / Stop (`0x3A` / `0x3B`, 86 / 68, release 28); in the air the same six (`0x83`, `0x8A` 52 frames,
/// `0x84` 78, `0x85` / `0x89`, `0x86`, `0x87` / `0x88` 78; Sora's air motions, Checked (files)) on Terra's air casts.
pub const CAST_CLIPS: [CastClip; 12] = [
    CastClip { suffix: "227", src: 9, release: 28.0, len: 28, free: 28 },
    CastClip { suffix: "228", src: 9, release: 0.0, len: 60, free: 46 },
    CastClip { suffix: "229", src: 10, release: 28.0, len: 86, free: 68 },
    CastClip { suffix: "230", src: 11, release: 30.0, len: 68, free: 68 },
    CastClip { suffix: "231", src: 12, release: 36.0, len: 68, free: 68 },
    CastClip { suffix: "232", src: 13, release: 28.0, len: 86, free: 68 },
    CastClip { suffix: "233", src: 14, release: 28.0, len: 28, free: 28 },
    CastClip { suffix: "234", src: 14, release: 0.0, len: 52, free: 46 },
    CastClip { suffix: "235", src: 15, release: 28.0, len: 78, free: 68 },
    CastClip { suffix: "236", src: 16, release: 30.0, len: 68, free: 68 },
    CastClip { suffix: "237", src: 17, release: 36.0, len: 68, free: 68 },
    CastClip { suffix: "238", src: 18, release: 28.0, len: 78, free: 68 },
];

/// The Dark Firaga clip's frames per KH1 frame (30 fps, Checked (files)), for its events; every source plays at its own
/// speed (0.184.6), [`CastSource::fps`].
const SRC_STEP: f64 = 0.5;

impl CastClip {
    /// This clip's source motion.
    pub fn source(&self) -> &'static CastSource {
        &CAST_SOURCES[self.src]
    }

    /// The source clip's time for each of this clip's frames. 0.184.6 (fork owner, 2026-11-23: "as animação ficaram
    /// estranhas meio que flicando"; 0.184.4 squeezed the key moment into 4 frames and stretched the charge: bones turned
    /// up to 350° a frame): the charge and the key moment at the clip's own speed, the key moment starting on KH1's
    /// release frame (the charge's start held before it when it is shorter, cut when it is longer); the recovery spread
    /// up to KH1's free frame; the last frame held after.
    pub fn times(&self) -> Vec<f64> {
        self.times_for(self.source())
    }

    /// [`Self::times`] on source `s` (the fallback's when the source couldn't be read).
    pub fn times_for(&self, s: &CastSource) -> Vec<f64> {
        let step = s.fps / 60.0;
        let key_out = (s.key_end - s.charge_end) / step;
        (0..self.len)
            .map(|k| {
                let k = k as f64;
                if k < self.release {
                    (s.charge_end - (self.release - k) * step).max(s.start)
                } else if k < self.release + key_out {
                    s.charge_end + (k - self.release) * step
                } else {
                    // Never faster than the clip's own speed: a recovery that doesn't fit stops where KH1's motion ends.
                    let span = (self.free as f64 - (self.release + key_out)).max((s.last - s.key_end) / step);
                    (s.key_end + (s.last - s.key_end) * (k - self.release - key_out) / span).min(s.last)
                }
            })
            .collect()
    }
}

/// `clip` sampled at `times` (its own skeleton `skel`), Root's and bone 1's x / z kept at their rest (no travel).
pub fn retime_in_place(clip: &Clip, skel: &RecomSkeleton, times: &[f64], name: &str) -> Clip {
    retime(clip, skel, times, name, false)
}

/// [`retime_in_place`], or with bone 1's x / z kept as the clip has them (`keep_sway`: a clip whose travel is already
/// out, [`plant_feet`], and whose body sways over its planted feet; Root stays put either way).
pub fn retime(clip: &Clip, skel: &RecomSkeleton, times: &[f64], name: &str, keep_sway: bool) -> Clip {
    let nb = clip.bones as usize;
    let end = (clip.frames.max(1) - 1) as f64;
    let rows: Vec<Vec<[f64; 9]>> = times
        .iter()
        .map(|&t| {
            (0..nb)
                .map(|b| {
                    let s = clip.sample(b, t.min(end));
                    let mut tt = s.t;
                    if b == 0 || (b == 1 && !keep_sway) {
                        let rest = skel.bones.get(b).map_or([0.0; 3], |x| x.local.t);
                        tt[0] = if b == 0 { 0.0 } else { rest[0] };
                        tt[2] = if b == 0 { 0.0 } else { rest[2] };
                    }
                    [tt[0], tt[1], tt[2], s.r[0], s.r[1], s.r[2], s.s[0], s.s[1], s.s[2]]
                })
                .collect()
        })
        .collect();
    let mut c = encode(clip, &rows);
    c.name = crate::ann::name16(name);
    // One frame per 60 Hz frame (`flags` bits 2..4 = 0: 60 fps): `times` are KH1's frames; the source clip's own rate
    // (the sleight clips are 30 fps, Checked (files)) is only in its sample times.
    c.flags &= !0x1C;
    c
}

/// A source clip's event frame (60 Hz, as `.EET` / `.SES` count them) in the clip's own frames.
fn event_time(clip: &Clip, frame: i32) -> f64 {
    frame as f64 * clip.fps() as f64 / 60.0
}

/// A Dark Firaga event (at its clip time `t`) on cast clip `c`, by phase: before the thrust, at the same share of the
/// charge (none on a clip with no charge); from the thrust on, the same number of frames after the release as after
/// the thrust's start, at the clip's speed. None past the clip's end.
fn out_frame(c: &CastClip, t: f64) -> Option<i32> {
    let at = if t < DF_CHARGE_END {
        if c.release <= 0.0 {
            return None;
        }
        // At the Dark Firaga clip's speed before the release; one before the clip's start goes on its first frame.
        (c.release - (DF_CHARGE_END - t) / SRC_STEP).ceil().max(0.0)
    } else {
        c.release + ((t - DF_CHARGE_END) / SRC_STEP).round()
    };
    ((at as usize) < c.len).then_some(at as i32)
}
/// The bones the cast effects sit on, in the skeleton their clip plays on.
#[derive(Clone, Copy, Debug)]
pub struct FxBones {
    /// `Root`.
    pub root: i32,
    /// `l_hand`.
    pub l_hand: i32,
    /// `chest1`.
    pub chest1: i32,
    /// `wep0`.
    pub wep0: i32,
}

impl FxBones {
    /// From a Riku or Dark Riku skeleton.
    pub fn of(sk: &RecomSkeleton) -> FxBones {
        let f = |n: &str| sk.find(n).map_or(0, |i| i as i32);
        FxBones { root: 0, l_hand: f("l_hand"), chest1: f("chest1"), wep0: f("wep0") }
    }
}

/// One cast effect: `dpx` program `id` on `bone`, from `from` to `to` frames after the release (`to` None: a one-shot),
/// at `scale`, turned `rot_y` degrees, offset `off` (metres).
struct Fx {
    dpx: &'static str,
    id: i32,
    bone: fn(&FxBones) -> i32,
    from: i32,
    to: Option<i32>,
    scale: f32,
    rot: [f32; 3],
    off: [f32; 3],
}

const fn fx(dpx: &'static str, id: i32, bone: fn(&FxBones) -> i32, from: i32, to: Option<i32>, scale: f32) -> Fx {
    Fx { dpx, id, bone, from, to, scale, rot: [0.0; 3], off: [0.0; 3] }
}
fn b_root(b: &FxBones) -> i32 { b.root }
fn b_hand(b: &FxBones) -> i32 { b.l_hand }
fn b_chest(b: &FxBones) -> i32 { b.chest1 }
fn b_blade(b: &FxBones) -> i32 { b.wep0 }

/// 0.185.8 (fork owner, 2026-11-23, on the cast effects: "o audio que toca sobre os efeitos é A", Riku's own effects, one
/// look per spell): the effects of Blizzard (`spell` 2), Thunder / Aero (3: variants 1..3 Thunder, 4..6 Aero) and Gravity /
/// Stop (5: 1..3 Gravity, 4..6 Stop) at each level, the programs of Riku's own `EF1500` that suit them (the workspace's
/// `recom-m13-recom-fx.md` § 8.4: ids, bones, sounds, loops Checked (files); how each looks and the fit: Assumption, not seen
/// in game; the bones and offsets of `ex_1580` 3 / 7 are Re:CoM's own events'). Fire keeps Dark Firaga's, Cure the heal
/// sparkle. `t`: an event to copy the unlisted fields from.
/// 0.198.19: Thunder's vortex (`ex_1580` 11, levels 2 and 3) at the raised blade, not the left hand: Terra's raise sweeps
/// the left hand 0.47 m behind the body while the blade goes up in front (Checked (files): the built clips' bones). On
/// `Root` at [`thunder_vortex_at`], not on `wep0` (fork owner on the blade: "thundaga ainda ta estranho", "Vórtice na
/// lâmina": the 60-tick program turned and swung with the blade into the recovery).
/// Thunder's vortex point on `Root` (metres, model frame: +y up, +z the facing): the blade's grip (`wep0`) on KH1's release
/// frame of Terra's raise as built on Riku, ground (`230`) (-0.11, 1.51, 0.52) and air (`236`) (0.00, 1.50, 0.41); Dark Riku's
/// within 4 cm (Checked (files): `cast_effects_meet_the_gesture`). Held there, it does not follow the blade's swing.
pub fn thunder_vortex_at(c: &CastClip) -> [f32; 3] {
    if c.suffix == "236" { [0.0, 1.5, 0.41] } else { [-0.11, 1.51, 0.52] }
}

fn spell_events(spell: usize, c: &CastClip, t: &[u8; crate::eet::EVENT], bones: &FxBones) -> Vec<[u8; crate::eet::EVENT]> {
    let mut y180 = fx("ex_1580.dpx", 7, b_root, 0, None, 0.5);
    y180.rot = [0.0, 180.0, 0.0];
    y180.off = [0.1, 0.4, 0.1];
    let g7 = |scale: f32| Fx { scale, ..y180 };
    let mut g3 = fx("ex_1580.dpx", 3, b_root, 0, None, 1.0);
    g3.rot = [0.0, 180.0, 0.0];
    g3.off = [0.0, 0.0, 0.1];
    let mut th3 = fx("ex_1580.dpx", 42, b_chest, -6, Some(40), 1.0);
    th3.rot = [90.0, 0.0, 0.0];
    let mut vortex = fx("ex_1580.dpx", 11, b_root, 0, None, 0.8);
    vortex.off = thunder_vortex_at(c);
    let mut out = Vec::new();
    for variant in 1..=6i32 {
        let level = (variant - 1) % 3 + 1;
        let second = variant > 3;
        let list: Vec<Fx> = match (spell, second, level) {
            (2, false, 1) => vec![fx("ex_1580.dpx", 38, b_root, 0, None, 1.0)],
            (2, false, 2) => vec![fx("ex_1580.dpx", 38, b_root, 0, None, 1.0), fx("ex_1580.dpx", 39, b_chest, 0, None, 1.0)],
            (2, false, _) => vec![fx("ex_1580.dpx", 38, b_root, 0, None, 1.5), fx("ex_1580.dpx", 39, b_chest, 0, None, 1.0), fx("ex_1580.dpx", 10, b_hand, 0, None, 0.7)],
            (3, false, 1) => vec![fx("ex_1580.dpx", 19, b_chest, -10, None, 1.0)],
            (3, false, 2) => vec![fx("ex_1580.dpx", 19, b_chest, -10, None, 1.0), Fx { ..vortex }],
            (3, false, _) => vec![fx("ex_1580.dpx", 19, b_chest, -10, None, 1.0), Fx { ..vortex }, Fx { ..th3 }],
            (3, true, 1) => vec![fx("speed_break.dpx", 0, b_root, -6, None, 1.0)],
            (3, true, 2) => vec![fx("speed_break.dpx", 0, b_root, -6, None, 1.0), fx("speed_break.dpx", 5, b_root, -6, None, 1.0)],
            (3, true, _) => vec![fx("speed_break.dpx", 0, b_root, -6, None, 1.0), fx("speed_break.dpx", 5, b_root, -6, None, 1.0), fx("ex_1580.dpx", 8, b_blade, -24, Some(30), 0.5), fx("ex_1580.dpx", 11, b_root, 0, None, 0.8)],
            (5, false, 1) => vec![fx("ex_1580.dpx", 36, b_root, -30, None, 1.0), g7(0.5)],
            (5, false, 2) => vec![fx("ex_1580.dpx", 36, b_root, -30, None, 1.0), g7(0.7)],
            (5, false, _) => vec![fx("ex_1580.dpx", 36, b_root, -30, None, 1.0), g7(0.9), Fx { ..g3 }],
            (5, true, 1) => vec![fx("ex_1580.dpx", 39, b_root, 0, None, 1.0)],
            (5, true, 2) => vec![fx("ex_1580.dpx", 39, b_root, 0, None, 1.0), fx("ex_1580.dpx", 18, b_root, 0, None, 0.6)],
            (5, true, _) => vec![fx("ex_1580.dpx", 38, b_root, 0, None, 1.5), fx("ex_1580.dpx", 39, b_root, 0, None, 1.0), fx("ex_1580.dpx", 18, b_root, 0, None, 0.6)],
            _ => Vec::new(),
        };
        for f in list {
            let start = c.release as i32 + f.from;
            if start < 0 || start >= c.len as i32 {
                continue;
            }
            let mut e = *t;
            e[..0x20].fill(0);
            e[..f.dpx.len()].copy_from_slice(f.dpx.as_bytes());
            let put = |e: &mut [u8; crate::eet::EVENT], o: usize, v: [u8; 4]| e[o..o + 4].copy_from_slice(&v);
            put(&mut e, 0x20, f.id.to_le_bytes());
            put(&mut e, 0x24, (f.bone)(bones).to_le_bytes());
            for k in 0..3 {
                put(&mut e, 0x28 + 4 * k, f.off[k].to_le_bytes());
                put(&mut e, 0x3C + 4 * k, f.rot[k].to_le_bytes());
                put(&mut e, 0x48 + 4 * k, f.scale.to_le_bytes());
            }
            put(&mut e, 0x34, start.to_le_bytes());
            let end = f.to.map_or(-1, |to| (c.release as i32 + to).min(c.len as i32 - 1));
            put(&mut e, 0x38, end.to_le_bytes());
            // A one-shot is flag 8 (set on almost every event); a looping program is cut at its end frame (0x10) with a fade.
            put(&mut e, 0x54, (if f.to.is_some() { 0x18u32 } else { 0x8 }).to_le_bytes());
            put(&mut e, 0x58, (if f.to.is_some() { 10.0f32 } else { 0.0 }).to_le_bytes());
            put(&mut e, 0x5C, variant.to_le_bytes());
            out.push(e);
        }
    }
    let _ = spell;
    out
}

/// 0.185.7 (fork owner, 2026-11-23, on 0.185.5's random pick from four lines: "os audios ficaram muito aleatórios precisa
/// fazer mais sentido com a magia"): one fixed voice per spell, by the sleight that casts the same kind of thing, in
/// [`CAST_CLIPS`]' order of spells (Fire's wind-up and release, Blizzard, Thunder / Aero, Cure, Gravity / Stop; the air
/// six the same). Group 3 numbers (Riku's and Dark Riku's shared bank), Checked (files: the workspace's
/// `recom-m13-recom-fx.md` § 8.3, which sleight plays which): Fire 3035 (Dark Firaga's own shout), Blizzard 3036 (Dark
/// Aura's second line), Thunder / Aero 3033 (a Dark sleight's), Cure 3021 (Holy Burst's, the light one), Gravity / Stop
/// 3034 (a Dark sleight's). What each line says: Assumption (no audio was played here).
pub const CAST_VOICES: [u32; 6] = [35, 35, 36, 33, 21, 34];

/// 0.184.2: the Dark Firaga clip's events on the cast clips (`PLAYER/1088/PL1088.EET` / `.SES`, `p003_043`; Checked
/// (files)): `dark_faiga` 0 on the left hand from 20 (the charge's dark glow), `dark_faiga` 1 on Root at 72 (the thrust's
/// flash), the voice 3035 at 11 and the thrust's sound 605/2 at 72; 605/1 at 16 is the leap's landing (the leap is out:
/// dropped). Each by its phase on every clip (60 Hz → the Dark Firaga clip's frames → [`out_frame`]), on bones by name
/// (`bone_of`: the source skeleton's index → the target's); variant 0 (Re:CoM's own, as Riku's rows play).
///
/// 0.184.5 (fork owner, 2026-11-23: "os spells precisam ter diferença dependendo do level dele e os efeitos também"):
/// each clip's effects come in three EET variants, one per spell level (`+0x5C`; Re:CoM's motion start keeps the events
/// of the variant its request passes, `0x14017A240`, Checked (decomp); the DLL passes the cast's level): 1 the glow and
/// the flash; 2 also the darkness cloak (`cloak`: the `ex_1580` body programs Riku's / Dark Riku's own Impulse puts on
/// him, Checked (files)); 3 also the flash at 1.5 × its size (0.184.8: the blade's aura `weapon_ora` 0, `blade`, no longer:
/// it read as a shaking blade).
/// Every one of these programs is in Riku's own `EF1500.EFF`, loaded with him (Checked (files)).
pub fn cast_events(df: &Clip, eet: &[crate::eet::Record], ses: &[crate::ses::Entry], prefix: &str, bone_of: &dyn Fn(i32) -> Option<i32>, cloak: &[[u8; crate::eet::EVENT]], blade: Option<[u8; crate::eet::EVENT]>, bones: &FxBones) -> Result<(Vec<crate::eet::Record>, Vec<crate::ses::Entry>)> {
    let i32_at = |e: &[u8], o: usize| i32::from_le_bytes(e[o..o + 4].try_into().unwrap());
    let src_e = eet.iter().find(|r| r.name == DARK_FIRAGA).ok_or_else(|| crate::Error::Format(format!("PL1088.EET: no {DARK_FIRAGA}")))?;
    let src_s = ses.iter().find(|e| e.name() == DARK_FIRAGA).ok_or_else(|| crate::Error::Format(format!("PL1088.SES: no {DARK_FIRAGA}")))?;
    let mut records = Vec::new();
    let mut entries = Vec::new();
    for (ci, c) in CAST_CLIPS.iter().enumerate() {
        let name = format!("{prefix}{}", c.suffix);
        // 0.184.8 (fork owner: Curaga's effect "muito estranho"): Cure keeps only the voice: no dark glow, flash, cloak
        // or blade aura on a heal (the DLL plays Riku's own heal sparkle at the release).
        let cure = CURE_CLIPS.contains(&c.suffix);
        let mut base = Vec::new();
        for e in src_e.a.iter().filter(|_| !cure) {
            let Some(at) = out_frame(c, event_time(df, i32_at(e, 0x34))) else { continue };
            let Some(bone) = bone_of(i32_at(e, 0x24)) else { continue };
            let mut e = *e;
            e[0x24..0x28].copy_from_slice(&bone.to_le_bytes());
            e[0x34..0x38].copy_from_slice(&at.to_le_bytes());
            base.push(e);
        }
        let mut a = Vec::new();
        for level in 1..=3i32 {
            let tag = |mut e: [u8; crate::eet::EVENT]| {
                e[0x5C..0x60].copy_from_slice(&level.to_le_bytes());
                e
            };
            for e in &base {
                let mut e = *e;
                // The thrust's flash (`dark_faiga` 1) at 1.5 × on level 3.
                if level == 3 && i32_at(&e, 0x20) == 1 {
                    for j in 0..3 {
                        let at = 0x48 + 4 * j;
                        let v = f32::from_le_bytes(e[at..at + 4].try_into().unwrap()) * 1.5;
                        e[at..at + 4].copy_from_slice(&v.to_le_bytes());
                    }
                }
                a.push(tag(e));
            }
            if level >= 2 && !cure {
                a.extend(cloak.iter().map(|e| tag(*e)));
            }
            // 0.184.8 (fork owner, 2026-11-23: "stopga a espada do RIku ta vibrando toda", "blizzaga tbm ficou um treme treme
            // da espada"): the blade's aura (`weapon_ora` 0) read as a shaking blade: not played (`blade` kept for later).
            let _ = &blade;
        }
        // 0.185.8: Blizzard, Thunder / Aero and Gravity / Stop each get their own effects from Riku's own `EF1500`.
        if matches!(ci % 6, 2 | 3 | 5) {
            if let Some(t) = src_e.a.first() {
                a = spell_events(ci % 6, c, t, bones);
            }
        }
        let mut evs = Vec::new();
        for e in &src_s.events {
            if (i32_at(e, 0), i32_at(e, 4)) == (605, 1) || (cure && i32_at(e, 0) != 3) {
                continue;
            }
            let Some(at) = out_frame(c, event_time(df, i32_at(e, 0x18))) else { continue };
            let mut e = *e;
            e[0x18..0x1C].copy_from_slice(&at.to_le_bytes());
            // 0.185.7: the cast's voice is the spell's own ([`CAST_VOICES`]), not Dark Firaga's 3035 on all of them.
            if i32_at(&e, 0) == 3 {
                e[4..8].copy_from_slice(&CAST_VOICES[ci % 6].to_le_bytes());
                // No random table: the number above plays as it is.
                let f = u32::from_le_bytes(e[0x28..0x2C].try_into().unwrap()) & !4;
                e[0x28..0x2C].copy_from_slice(&f.to_le_bytes());
            }
            evs.push(e);
        }
        if !a.is_empty() {
            records.push(crate::eet::Record { name: name.clone(), a, b: Vec::new(), raw_name: None });
        }
        if !evs.is_empty() {
            let mut head = src_s.head;
            head[..0x20].fill(0);
            head[..name.len()].copy_from_slice(name.as_bytes());
            entries.push(crate::ses::Entry { head, events: evs });
        }
    }
    Ok((records, entries))
}

/// 0.184.5: the darkness cloak and the blade's aura a sleight clip puts on its user (`rec`, its EET record; Checked
/// (files)): list-A `ex_1580` 20, 21 and 23..34 and `weapon_ora` 0, all from frame 0 to the motion's end; returned with
/// their own bones (the record's skeleton) and start 0.
pub fn cloak_of(rec: &crate::eet::Record) -> (Vec<[u8; crate::eet::EVENT]>, Option<[u8; crate::eet::EVENT]>) {
    let name = |e: &[u8]| String::from_utf8_lossy(&e[..e[..0x20].iter().position(|&c| c == 0).unwrap_or(0x20)]).into_owned();
    let id = |e: &[u8]| i32::from_le_bytes(e[0x20..0x24].try_into().unwrap());
    let cloak = rec.a.iter().filter(|e| name(&e[..]) == "ex_1580.dpx" && matches!(id(&e[..]), 20 | 21 | 23..=34)).copied().collect();
    let blade = rec.a.iter().find(|e| name(&e[..]) == "weapon_ora.dpx" && id(&e[..]) == 0).copied();
    (cloak, blade)
}

/// Riku's bone → Dark Riku's of the same role, for the Dark → Riku retarget: by name, Riku's skeleton root as Dark's;
/// Riku's `k10` / `k11` (the hand-to-blade chain, which is not Dark's `wep1` / `wep2`: [`Grip`]) and his own bones
/// keep his battle stance's pose under their parent; `wep0` follows Dark's `wep0` by its world rotation.
pub fn dark_to_riku_map(dark: &RecomSkeleton, riku: &RecomSkeleton) -> Vec<Option<usize>> {
    riku.bones
        .iter()
        .map(|b| match b.name.as_str() {
            "ex_1010_sk" => dark.find("ex_1580_sk"),
            "k10" | "k11" => None,
            n => dark.find(n),
        })
        .collect()
}

/// A clip of a PLAYER group's pack, by name.
pub type ClipFetch<'a> = dyn Fn(u16, &str) -> Result<Clip> + 'a;

/// What turns a [`Src`] into a clip on Riku's or Dark Riku's skeleton.
pub struct CastCtx<'a> {
    /// A pack's clip.
    pub get: &'a ClipFetch<'a>,
    /// Riku's, Dark Riku's and Sora's skeletons.
    pub riku: &'a RecomSkeleton,
    /// Dark Riku's.
    pub dark: &'a RecomSkeleton,
    /// Sora's.
    pub sora: &'a RecomSkeleton,
    /// Riku's battle stance ([`RIKU_STANCE`]): his bones no source matches.
    pub stance: Option<&'a Clip>,
    /// KH1 Riku-Ansem's motion set and model.
    pub ansem: Option<(&'a [u8], &'a [u8])>,
    /// 0.189.0: BBS FM Terra's skeleton and clips ([`GuardSource::terra`]); [`Src::Terra`] names a clip of it.
    pub terra: Option<(&'a RecomSkeleton, &'a [crate::pam::PamClip])>,
}

impl CastCtx<'_> {
    /// `src` on `to`'s skeleton (Riku or Dark), None when it is a KH1 motion and KH1's files weren't read.
    pub fn clip_on(&self, src: &Src, to: Skel) -> Result<Option<Clip>> {
        let (raw, from) = match *src {
            // 0.184.9: Riku's item use hides and turns his Soul Eater (its scale 0): kept in his hand as at its first frame.
            Src::Pack(g, name, Skel::Riku) if name == ITEM_USE => (freeze_blade(&(self.get)(g, name)?, self.riku), Skel::Riku),
            Src::Pack(g, name, sk) => ((self.get)(g, name)?, sk),
            Src::Kh1Ansem(req) => {
                let Some((mset, mdls)) = self.ansem else { return Ok(None) };
                let ms = kh1_formats::mset::MotionSet::parse(mset)?;
                let (_, m) = ms.motion_for(req).ok_or_else(|| crate::Error::Format(format!("xa_ex_1580.mset: no request {req}")))?;
                let conv = crate::convert::Converter::new(mdls, self.dark.clone())?;
                let c = conv.convert(m, &crate::convert::ClipSpec { name: format!("kh1_1580_{req}"), looped: false, root_motion: false, blend: 8 })?;
                // 0.184.8 (fork owner, 2026-11-23: "stopga a espada do RIku ta vibrando toda"): the converted motion's
                // blade bones shake; they keep their first frame under the hand.
                (freeze_blade(&c, self.dark), Skel::Dark)
            }
            Src::Terra(name) => {
                let Some((_, clips)) = self.terra else { return Ok(None) };
                let c = clips.iter().find(|c| c.name == name).ok_or_else(|| crate::Error::Format(format!("BBS Terra: no clip {name}")))?;
                (crate::pam::to_clip(c, &format!("terra_{name}")), Skel::Terra)
            }
        };
        // Terra's onto Riku by the bone map (`crate::pam::RIKU_FROM_TERRA`), onto Dark Riku through Riku.
        if from == Skel::Terra {
            let Some((dsk, _)) = self.terra else { return Ok(None) };
            // 0.185.4 (fork owner, 2026-11-23, on 0.185.3 in game: "ele está invertendo a lamina"): the blade keeps Riku's
            // own grip, `wep0` (and the blade under it) posed under his hand as in his battle stance on every frame, not
            // from Terra's `R_buki`: the source holds its blade the other way round in the hand (0.185.2 / 0.185.3, either map).
            let mut map = crate::pam::riku_map(self.riku, dsk);
            if let Some(w) = self.riku.find("wep0") {
                map[w] = None;
            }
            // 0.190.0 (fork owner, in game on 0.189.0: Block "perfeita", but "todas as de magia estão estranhas, o jeito que ele
            // se move está esquisito"): Terra's body travel and crouch on Riku's shorter legs made the body surge 0.4 m back and
            // forth over planted feet (a 0.25 m step in one frame at 315's hop) and kicked the legs in the air. The casts now
            // take Terra's torso, head, arms and blade only: on the ground Riku's legs and body stay in his stance; in the air
            // his legs follow Terra's tuck with the body at its height. The Guard (accepted) keeps the whole body.
            let name = name_of(src);
            let ground_cast = matches!(name, "315" | "317");
            let air_cast = matches!(name, "316" | "318");
            let on_riku = retarget_with(&raw, dsk, self.riku, &map, self.stance)?;
            let on_riku = flip_blade_roll(&on_riku, self.riku);
            if ground_cast {
                let on_riku = match self.stance {
                    Some(st) => tame_cast(&on_riku, self.riku, st, TAME_LEGS),
                    None => on_riku,
                };
                return Ok(Some(if to == Skel::Riku { on_riku } else { retarget(&on_riku, self.riku, self.dark, &bone_map(self.riku, self.dark))? }));
            }
            if air_cast {
                let on_riku = match self.stance {
                    Some(st) => hold_body(&on_riku, self.riku, st.sample(self.riku.find("ex_1010_sk").unwrap_or(1), 0.0).t),
                    None => on_riku,
                };
                return Ok(Some(if to == Skel::Riku { on_riku } else { retarget(&on_riku, self.riku, self.dark, &bone_map(self.riku, self.dark))? }));
            }
            let on_riku = carry_translation(&on_riku, self.riku, &raw, dsk, crate::pam::CARRIED_TRANSLATION, crate::terra::TRAVEL_SCALE);
            let on_riku = if is_ground(name_of(src)) { plant_feet(&on_riku, self.riku) } else { from_start(&on_riku, self.riku, "ex_1010_sk") };
            // 0.189.0: Terra's legs are longer (his rotations on Riku's shorter ones sank the lower foot 4 cm under the floor of
            // Riku's stance at frame 0, Checked (files)): a ground clip's floor is Riku's stance's.
            let on_riku = match (is_ground(name_of(src)), self.stance) {
                (true, Some(st)) => put_on_floor(&on_riku, st, self.riku),
                _ => on_riku,
            };
            return Ok(Some(if to == Skel::Riku { on_riku } else { retarget(&on_riku, self.riku, self.dark, &bone_map(self.riku, self.dark))? }));
        }
        let skel = |k: Skel| match k {
            Skel::Riku => self.riku,
            Skel::Dark => self.dark,
            Skel::Sora => self.sora,
            Skel::Terra => self.riku,
        };
        if from == to {
            return Ok(Some(raw));
        }
        let (a, b) = (skel(from), skel(to));
        let root = |k: Skel| match k {
            Skel::Riku | Skel::Terra => "ex_1010_sk",
            Skel::Dark => "ex_1580_sk",
            Skel::Sora => "ex_0010_sk",
        };
        let clip = match (from, to) {
            (Skel::Dark, Skel::Riku) => retarget_with(&raw, a, b, &dark_to_riku_map(a, b), self.stance)?,
            (Skel::Riku, Skel::Dark) => retarget(&raw, a, b, &bone_map(a, b))?,
            _ => {
                // Sora's onto either: bones by name, the skeleton roots as each other; Riku's own bones from his stance.
                let map: Vec<Option<usize>> = b.bones.iter().map(|x| if x.name == root(to) { a.find(root(from)) } else { a.find(&x.name) }).collect();
                retarget_with(&raw, a, b, &map, if to == Skel::Riku { self.stance } else { None })?
            }
        };
        Ok(Some(clip))
    }

    /// Cast clip `c` on `to`'s skeleton, named `prefix` + its suffix: its source, else its fallback's.
    pub fn cast_clip(&self, c: &CastClip, to: Skel, prefix: &str) -> Result<Clip> {
        let mut s = c.source();
        loop {
            let src = if to == Skel::Riku { &s.riku } else { &s.dark };
            if let Some(clip) = self.clip_on(src, to)? {
                let planted = is_ground(name_of(src));
                return Ok(retime(&clip, if to == Skel::Riku { self.riku } else { self.dark }, &c.times_for(s), &format!("{prefix}{}", c.suffix), planted));
            }
            s = &CAST_SOURCES[s.fallback.ok_or_else(|| crate::Error::Format(format!("cast clip {}: no source", c.suffix)))?];
        }
    }
}

/// Riku's item use (`p002_034`, Cure's motion since 0.184.9).
pub const ITEM_USE: &str = "p002_034";

/// `clip` with the blade (Dark Riku's `wep0`, `wep1`, `wep2`, Riku's `wep0`, `k10`, `k11`, and every bone under `wep0`) held at
/// its first frame's local pose.
pub fn freeze_blade(clip: &Clip, skel: &RecomSkeleton) -> Clip {
    let mut frozen = vec![false; skel.bones.len()];
    for (j, b) in skel.bones.iter().enumerate() {
        let parent_frozen = b.parent >= 0 && !matches!(skel.bones[b.parent as usize].name.as_str(), "wep1" | "wep2" | "k10" | "k11") && frozen[b.parent as usize];
        frozen[j] = matches!(b.name.as_str(), "wep0" | "wep1" | "wep2" | "k10" | "k11") || parent_frozen;
    }
    let rows: Vec<Vec<[f64; 9]>> = (0..clip.frames.max(1) as usize)
        .map(|f| {
            (0..clip.bones as usize)
                .map(|b| {
                    let s = clip.sample(b, if frozen.get(b).copied().unwrap_or(false) { 0.0 } else { f as f64 });
                    [s.t[0], s.t[1], s.t[2], s.r[0], s.r[1], s.r[2], s.s[0], s.s[1], s.s[2]]
                })
                .collect()
        })
        .collect();
    let mut c = encode(clip, &rows);
    c.flags = (c.flags & !0x1C) | (clip.flags & 0x1C);
    c
}

/// 0.185.1 (fork owner, 2026-11-23: on the DDD casts, "os pé do riku ficam suspensos"): `clip` (on `skel`) with each
/// listed target bone's local translation at its rest plus the sum of its source bones' moves off rest in `src` (on
/// `src_skel`). DDD moves the whole body by translating `koshi` and `hara` together (the legs' and the spine's parents,
/// both under `center`; 0.88 → 0.65 m in its crouches), and the rotation-only retarget left Riku's body at its rest
/// height, so his feet rose up to 0.5 m. 0.185.2 (fork owner: "as anims de magia ainda muito desalinhadas", screenshots
/// of Riku crouched above his shadow): 0.185.1 put that on Riku's `hip`, which no Re:CoM clip ever moves (Checked
/// (files): in all of `PL0101.ANN`'s clips only bones 0 / 1 and the blade's `k11` move their translation; the body's
/// height is on `ex_1010_sk`, the parent of `hip` and `waist`) and the game showed him floating: it goes on
/// `ex_1010_sk` now (Assumption: the game ignores `hip`'s translation; the screenshots).
pub fn carry_translation(clip: &Clip, skel: &RecomSkeleton, src: &Clip, src_skel: &RecomSkeleton, pairs: &[(&str, &str)], scale: f64) -> Clip {
    let pairs: Vec<(usize, usize)> = pairs.iter().filter_map(|(t, f)| Some((skel.find(t)?, src_skel.find(f)?))).collect();
    let rows: Vec<Vec<[f64; 9]>> = (0..clip.frames.max(1) as usize)
        .map(|f| {
            (0..clip.bones as usize)
                .map(|b| {
                    let s = clip.sample(b, f as f64);
                    let mut t = s.t;
                    if pairs.iter().any(|(i, _)| *i == b) {
                        t = skel.bones[b].local.t;
                        for &(_, j) in pairs.iter().filter(|(i, _)| *i == b) {
                            let st = src.sample(j, f as f64).t;
                            let rest = src_skel.bones[j].local.t;
                            for k in 0..3 {
                                t[k] += (st[k] - rest[k]) * scale;
                            }
                        }
                    }
                    [t[0], t[1], t[2], s.r[0], s.r[1], s.r[2], s.s[0], s.s[1], s.s[2]]
                })
                .collect()
        })
        .collect();
    let mut c = encode(clip, &rows);
    c.flags = (c.flags & !0x1C) | (clip.flags & 0x1C);
    c
}

/// 0.185.6: `clip` with `bone`'s local translation moved by its offset from rest at frame 0, so the clip starts with
/// the body at its rest height and keeps only its move from there. For DDD's air casts (`316` / `318`), whose body
/// starts 0.18 m above DDD's rest and bobs −0.32..+0.20 m from there (Checked (files): `center` + `koshi`); in the air
/// Re:CoM's root is where the jump has him, so the clip starts where his air pose is (Assumption: Re:CoM's air rows keep
/// the body at its rest height, as its ground ones do).
pub fn from_start(clip: &Clip, skel: &RecomSkeleton, bone: &str) -> Clip {
    let Some(k) = skel.find(bone) else { return clip.clone() };
    let t0 = clip.sample(k, 0.0).t;
    let rest = skel.bones[k].local.t;
    let rows: Vec<Vec<[f64; 9]>> = (0..clip.frames.max(1) as usize)
        .map(|f| {
            (0..clip.bones as usize)
                .map(|b| {
                    let s = clip.sample(b, f as f64);
                    let mut t = s.t;
                    if b == k {
                        for i in 0..3 {
                            t[i] -= t0[i] - rest[i];
                        }
                    }
                    [t[0], t[1], t[2], s.r[0], s.r[1], s.r[2], s.s[0], s.s[1], s.s[2]]
                })
                .collect()
        })
        .collect();
    let mut c = encode(clip, &rows);
    c.flags = (c.flags & !0x1C) | (clip.flags & 0x1C);
    c
}

/// 0.185.7: `clip` with `wep0` turned half a turn about its own x, the blade's axis ([`BLADE_FLIP`]), the chain under it
/// (`k1`..`k11`, in `wep0`'s frame) with it. The blade's tip stays where it was; only the side of the blade that faces
/// out turns over. Riku's mesh runs along `wep0`'s −x (its far 5 % of vertices at (−0.74, −0.05, 0.0) m; Checked
/// (files)), and in DDD's clips its tip follows DDD's blade tip within 20..30° (Checked (files)); 0.185.3 and 0.185.4
/// both still showed it "do lado errado" in game (fork owner), which leaves the roll about that axis.
pub fn flip_blade_roll(clip: &Clip, skel: &RecomSkeleton) -> Clip {
    let Some(k) = skel.find("wep0") else { return clip.clone() };
    let mut prev = clip.sample(k, 0.0).r;
    let rows: Vec<Vec<[f64; 9]>> = (0..clip.frames.max(1) as usize)
        .map(|f| {
            (0..clip.bones as usize)
                .map(|b| {
                    let s = clip.sample(b, f as f64);
                    let mut r = s.r;
                    if b == k {
                        r = crate::math::euler_xyz(&m3(&BLADE_FLIP, &rot_xyz(r[0], r[1], r[2])), Some(prev));
                        prev = r;
                    }
                    [s.t[0], s.t[1], s.t[2], r[0], r[1], r[2], s.s[0], s.s[1], s.s[2]]
                })
                .collect()
        })
        .collect();
    let mut c = encode(clip, &rows);
    c.flags = (c.flags & !0x1C) | (clip.flags & 0x1C);
    c
}

/// The lower foot's height (`l_foot` / `r_foot` origins) of `clip` at frame `f` on `skel`, by forward kinematics.
fn feet_low(clip: &Clip, skel: &RecomSkeleton, f: f64) -> f64 {
    let (mut u, mut p): (Vec<M3>, Vec<[f64; 3]>) = (Vec::new(), Vec::new());
    for (i, b) in skel.bones.iter().enumerate() {
        let s = clip.sample(i, f);
        let r = rot_xyz(s.r[0], s.r[1], s.r[2]);
        if b.parent < 0 {
            u.push(r);
            p.push(s.t);
        } else {
            let q = b.parent as usize;
            let w = crate::math::vm(s.t, &u[q]);
            p.push([w[0] + p[q][0], w[1] + p[q][1], w[2] + p[q][2]]);
            u.push(m3(&r, &u[q]));
        }
    }
    ["l_foot", "r_foot"].iter().filter_map(|n| skel.find(n)).map(|k| p[k][1]).fold(f64::MAX, f64::min)
}

/// `clip` with its body (`ex_1010_sk`) moved straight up or down so its lower foot at frame 0 stands where `stance`'s does
/// (0.189.0, Terra's longer legs).
pub fn put_on_floor(clip: &Clip, stance: &Clip, skel: &RecomSkeleton) -> Clip {
    let Some(body) = skel.find("ex_1010_sk") else { return clip.clone() };
    let dy = feet_low(stance, skel, 0.0) - feet_low(clip, skel, 0.0);
    let parent = skel.bones[body].parent;
    let rows: Vec<Vec<[f64; 9]>> = (0..clip.frames.max(1) as usize)
        .map(|f| {
            (0..clip.bones as usize)
                .map(|b| {
                    let s = clip.sample(b, f as f64);
                    let mut t = s.t;
                    if b == body {
                        let up = if parent >= 0 {
                            let p = clip.sample(parent as usize, f as f64);
                            crate::math::vm([0.0, dy, 0.0], &tr(&rot_xyz(p.r[0], p.r[1], p.r[2])))
                        } else {
                            [0.0, dy, 0.0]
                        };
                        for i in 0..3 {
                            t[i] += up[i];
                        }
                    }
                    [t[0], t[1], t[2], s.r[0], s.r[1], s.r[2], s.s[0], s.s[1], s.s[2]]
                })
                .collect()
        })
        .collect();
    let mut c = encode(clip, &rows);
    c.flags = (c.flags & !0x1C) | (clip.flags & 0x1C);
    c
}

/// `clip` with the body's (`ex_1010_sk`) local translation held at `at` on every frame.
fn hold_body(clip: &Clip, skel: &RecomSkeleton, at: [f64; 3]) -> Clip {
    let Some(body) = skel.find("ex_1010_sk") else { return clip.clone() };
    let rows: Vec<Vec<[f64; 9]>> = (0..clip.frames.max(1) as usize)
        .map(|f| {
            (0..clip.bones as usize)
                .map(|b| {
                    let s = clip.sample(b, f as f64);
                    let mut t = s.t;
                    if b == body {
                        t = at;
                    }
                    [t[0], t[1], t[2], s.r[0], s.r[1], s.r[2], s.s[0], s.s[1], s.s[2]]
                })
                .collect()
        })
        .collect();
    let mut c = encode(clip, &rows);
    c.flags = (c.flags & !0x1C) | (clip.flags & 0x1C);
    c
}

/// Rotation `a` turned toward `b` by the share `w` (0 = a, 1 = b), about the axis of the rotation between them.
fn rot_blend(a: &M3, b: &M3, w: f64) -> M3 {
    let r = m3(&tr(a), b);
    let c = ((r[0][0] + r[1][1] + r[2][2] - 1.0) / 2.0).clamp(-1.0, 1.0);
    let ang = c.acos();
    if ang < 1e-9 {
        return *a;
    }
    // Axis from the skew part (row vectors: the transpose of the column-vector one).
    let k = [r[1][2] - r[2][1], r[2][0] - r[0][2], r[0][1] - r[1][0]];
    let n = (k[0] * k[0] + k[1] * k[1] + k[2] * k[2]).sqrt();
    if n < 1e-9 {
        return *a; // a half turn: no unique axis; keep a
    }
    let u = [k[0] / n, k[1] / n, k[2] / n];
    let t = ang * w;
    let (s, co) = t.sin_cos();
    let v = 1.0 - co;
    // Rodrigues for row vectors (the transpose of the usual matrix).
    let rw: M3 = [
        [co + u[0] * u[0] * v, u[0] * u[1] * v + u[2] * s, u[0] * u[2] * v - u[1] * s],
        [u[0] * u[1] * v - u[2] * s, co + u[1] * u[1] * v, u[1] * u[2] * v + u[0] * s],
        [u[0] * u[2] * v + u[1] * s, u[1] * u[2] * v - u[0] * s, co + u[2] * u[2] * v],
    ];
    m3(a, &rw)
}

/// 0.193.0 (fork owner, in game on 0.190.0: "as animações da cintura para baixo o riku ficou travadasso"): a ground cast
/// between 0.189.0's whole-body retarget (the body slid and hopped) and 0.190.0's frozen legs. `clip` is the retargeted cast
/// (Terra's whole body on Riku's skeleton, body travel not carried): the pelvis and legs (`hip`, `*_femur`, `*_tibia`,
/// `*_foot`, `*_toes`) are turned only the share `w` from Riku's stance toward Terra's, eased in and out over the first
/// and last [`TAME_EASE`] frames; `waist` keeps its world rotation (the torso and arms are Terra's untouched); the body
/// stays over its stance position and its height follows the lower foot (it is lowered or raised so that foot stands on the
/// stance's floor, within [`TAME_DROP`] / [`TAME_RISE`], smoothed over three frames).
pub fn tame_cast(clip: &Clip, skel: &RecomSkeleton, stance: &Clip, w: f64) -> Clip {
    let n = clip.frames.max(1) as usize;
    let nb = clip.bones as usize;
    let Some(body) = skel.find("ex_1010_sk") else { return clip.clone() };
    let st_r = |i: usize| {
        let s = stance.sample(i, 0.0);
        rot_xyz(s.r[0], s.r[1], s.r[2])
    };
    let blended: Vec<bool> = skel.bones.iter().map(|b| b.name == "hip" || ["femur", "tibia", "foot", "toes"].iter().any(|k| b.name.ends_with(k) && (b.name.starts_with("l_") || b.name.starts_with("r_")))).collect();
    let (hip, waist) = (skel.find("hip"), skel.find("waist"));
    let st_t = stance.sample(body, 0.0).t;
    let mut ws: Vec<M3> = Vec::with_capacity(nb);
    for (i, b) in skel.bones.iter().enumerate().take(nb) {
        ws.push(if b.parent < 0 { st_r(i) } else { m3(&st_r(i), &ws[b.parent as usize]) });
    }
    let mut prev: Vec<Option<[f64; 3]>> = vec![None; nb];
    let mut rows: Vec<Vec<[f64; 9]>> = Vec::with_capacity(n);
    for f in 0..n {
        let e = (f as f64 / TAME_EASE).min((n - 1 - f) as f64 / TAME_EASE).clamp(0.0, 1.0);
        let wf = w * e * e * (3.0 - 2.0 * e);
        let mut row: Vec<[f64; 9]> = Vec::with_capacity(nb);
        let lr: Vec<M3> = (0..nb).map(|i| { let s = clip.sample(i, f as f64); rot_xyz(s.r[0], s.r[1], s.r[2]) }).collect();
        // New local rotations, bone by bone (parents first): the pelvis and legs turned the share `wf` toward the clip's; the
        // feet keep the stance's world rotation (flat on the floor, the ankle takes up the leg's change) and the toes the
        // stance's local; `waist` keeps the world rotation the clip gave it; every other bone keeps its local.
        let mut wr0: Vec<M3> = Vec::with_capacity(nb);
        let mut wn: Vec<M3> = Vec::with_capacity(nb);
        let mut ours: Vec<Option<M3>> = vec![None; nb];
        for (i, b) in skel.bones.iter().enumerate().take(nb) {
            let par = b.parent;
            wr0.push(if par < 0 { lr[i] } else { m3(&lr[i], &wr0[par as usize]) });
            let name = b.name.as_str();
            let is_foot = name == "l_foot" || name == "r_foot";
            let is_toes = name == "l_toes" || name == "r_toes";
            let l = if is_foot && par >= 0 {
                m3(&ws[i], &tr(&wn[par as usize]))
            } else if is_toes {
                st_r(i)
            } else if blended[i] {
                rot_blend(&st_r(i), &lr[i], wf)
            } else if Some(i) == waist && par >= 0 && Some(par as usize) == hip {
                m3(&wr0[i], &tr(&wn[par as usize]))
            } else {
                lr[i]
            };
            if blended[i] || is_foot || is_toes || Some(i) == waist {
                ours[i] = Some(l);
            }
            wn.push(if par < 0 { l } else { m3(&l, &wn[par as usize]) });
        }
        for i in 0..nb {
            let s = clip.sample(i, f as f64);
            let mut r = s.r;
            if let Some(m) = &ours[i] {
                r = crate::math::euler_xyz(m, prev[i].or(Some(s.r)));
                prev[i] = Some(r);
            }
            let mut t = s.t;
            if i == body {
                t = st_t;
            }
            row.push([t[0], t[1], t[2], r[0], r[1], r[2], s.s[0], s.s[1], s.s[2]]);
        }
        rows.push(row);
    }
    let mut c = encode(clip, &rows);
    c.flags = (c.flags & !0x1C) | (clip.flags & 0x1C);
    // Height: the lower foot on the stance's floor.
    let floor = feet_low(stance, skel, 0.0);
    let dy: Vec<f64> = (0..n).map(|f| (floor - feet_low(&c, skel, f as f64)).clamp(-TAME_DROP, TAME_RISE)).collect();
    let sm: Vec<f64> = (0..n).map(|f| (dy[f.saturating_sub(1)] + dy[f] + dy[(f + 1).min(n - 1)]) / 3.0).collect();
    for (f, row) in rows.iter_mut().enumerate() {
        row[body][1] += sm[f];
    }
    let mut c = encode(clip, &rows);
    c.flags = (c.flags & !0x1C) | (clip.flags & 0x1C);
    c
}

/// The share of Terra's pelvis and leg turns that the ground casts keep (0 = Riku's stance, 0.190.0; 1 = all of Terra's, 0.189.0;
/// 0.7 chosen from posed renders of the real model at 0, 0.7 and 1: 0.7 flexes the knees and shifts the weight with the arms,
/// 1 crouches deep with a foot coming off the floor, 0 is the rigid stance the owner rejected).
pub const TAME_LEGS: f64 = 0.7;
/// [`tame_cast`]'s ease in and out, in source frames.
const TAME_EASE: f64 = 4.0;
/// How far [`tame_cast`] lowers the body to keep the lower foot on the floor, and how far it raises it.
const TAME_DROP: f64 = 0.15;
const TAME_RISE: f64 = 0.05;

/// A ground clip of the motion source ([`crate::pam::GROUND_CLIPS`]): its feet are kept on the floor.
fn is_ground(name: &str) -> bool {
    crate::pam::GROUND_CLIPS.contains(&name)
}

/// A Terra source's clip name ("" for any other source).
fn name_of(src: &Src) -> &str {
    match src {
        Src::Terra(n) => n,
        _ => "",
    }
}

/// 0.185.1 (fork owner, 2026-11-23: "os pé do riku ficam suspensos"): `clip` (on Riku's skeleton) with its body
/// (`ex_1010_sk`, where Re:CoM's clips move it; 0.185.1 moved `hip`, which the game ignores) moved so
/// the feet stay on the floor where the clip starts: the hip is lowered as much as the lower foot rises above the
/// first frame's floor, and a foot then standing on it (within 3 cm) on two frames in a row does not slide (the hip
/// takes its move; of two, the one that moves less). What is left of the slide
/// at the last frame is spread back over the clip, so it ends where it started (the stance). Checked (files): DDD's
/// ground thrust `315` hops 0.6 m forward on frames 1..7 and its hip drops the distance back at frame 14 (the DDD note
/// § 6 measures; the travel is the game's, not the clip's: Assumption), which in place lifted the feet up to 0.38 m.
pub fn plant_feet(clip: &Clip, skel: &RecomSkeleton) -> Clip {
    let (Some(hip), Some(lf), Some(rf)) = (skel.find("ex_1010_sk"), skel.find("l_foot"), skel.find("r_foot")) else { return clip.clone() };
    let n = clip.frames.max(1) as usize;
    let parent = skel.bones[hip].parent;
    // World positions of the feet, and the hip's parent's world rotation, per frame.
    let fk = |f: usize| -> ([f64; 3], [f64; 3], M3) {
        let mut u: Vec<M3> = Vec::with_capacity(skel.bones.len());
        let mut p: Vec<[f64; 3]> = Vec::with_capacity(skel.bones.len());
        for (i, b) in skel.bones.iter().enumerate() {
            let s = clip.sample(i, f as f64);
            let r = rot_xyz(s.r[0], s.r[1], s.r[2]);
            if b.parent < 0 {
                u.push(r);
                p.push(s.t);
            } else {
                let q = b.parent as usize;
                let w = crate::math::vm(s.t, &u[q]);
                p.push([w[0] + p[q][0], w[1] + p[q][1], w[2] + p[q][2]]);
                u.push(m3(&r, &u[q]));
            }
        }
        (p[lf], p[rf], if parent < 0 { crate::math::I3 } else { u[parent as usize] })
    };
    let poses: Vec<([f64; 3], [f64; 3], M3)> = (0..n).map(fk).collect();
    let floor = poses[0].0[1].min(poses[0].1[1]);
    // How far the hip is lowered at a frame (the lower foot's rise), and whether a foot then stands on the floor.
    let lift = |f: usize| (poses[f].0[1].min(poses[f].1[1]) - floor).max(0.0);
    let on = |p: [f64; 3], f: usize| p[1] - lift(f) < floor + 0.03;
    // The hip's horizontal offset (world x, z) per frame.
    let mut off = vec![[0.0f64; 2]; n];
    for f in 1..n {
        let (a, b) = (&poses[f - 1], &poses[f]);
        let mv = |p0: [f64; 3], p1: [f64; 3]| [p1[0] - p0[0], p1[2] - p0[2]];
        let (ml, mr) = (mv(a.0, b.0), mv(a.1, b.1));
        let len = |m: [f64; 2]| m[0].hypot(m[1]);
        let d = match (on(a.0, f - 1) && on(b.0, f), on(a.1, f - 1) && on(b.1, f)) {
            (true, true) => if len(ml) <= len(mr) { ml } else { mr },
            (true, false) => ml,
            (false, true) => mr,
            (false, false) => [(ml[0] + mr[0]) / 2.0, (ml[1] + mr[1]) / 2.0],
        };
        off[f] = [off[f - 1][0] - d[0], off[f - 1][1] - d[1]];
    }
    let end = off[n - 1];
    let last = (n - 1).max(1) as f64;
    let rows: Vec<Vec<[f64; 9]>> = (0..n)
        .map(|f| {
            let pu = &poses[f].2;
            let k = f as f64 / last;
            let dy = -lift(f);
            let world = [off[f][0] - end[0] * k, dy, off[f][1] - end[1] * k];
            // World delta into the hip's parent's frame (row vectors: world = local · U).
            let local = crate::math::vm(world, &tr(pu));
            (0..clip.bones as usize)
                .map(|b| {
                    let s = clip.sample(b, f as f64);
                    let mut t = s.t;
                    if b == hip {
                        for i in 0..3 {
                            t[i] += local[i];
                        }
                    }
                    [t[0], t[1], t[2], s.r[0], s.r[1], s.r[2], s.s[0], s.s[1], s.s[2]]
                })
                .collect()
        })
        .collect();
    let mut c = encode(clip, &rows);
    c.flags = (c.flags & !0x1C) | (clip.flags & 0x1C);
    c
}

/// Terra's Guard (BBS FM, command kind 265, PAT_TERR slot 63, clip [`crate::terra::GUARD`], 36 frames at 30): the game
/// plays it once, with no separate hold, release or hit clip found (Checked (decomp, files)). Posed on Riku (Checked
/// (files)): the weapon arm is across the front by frame 3 and stays there to about frame 24, then comes back down to the
/// stance by 35. On Riku's 54 KH1 frames: up over the first [`RIKU_GUARD_UP`] frames, the pose through KH1's live window
/// (10..36), the lowering after it. That the game holds the pose after frame 35 is not shown (the clip itself lowers the
/// arm): Assumption.
pub const TERRA_GUARD_KEYS: (f64, f64, f64) = (4.0, 24.0, 35.0);
/// Terra's Guard on Riku's skeleton (named [`RIKU_GUARD`]), None when Terra's files weren't read.
pub fn terra_guard(ctx: &CastCtx) -> Result<Option<Clip>> {
    let Some(g) = ctx.clip_on(&Src::Terra(crate::terra::GUARD), Skel::Riku)? else { return Ok(None) };
    let (up, hold, last) = TERRA_GUARD_KEYS;
    let held = 37.0;
    let times: Vec<f64> = (0..RIKU_GUARD_FRAMES)
        .map(|k| {
            let k = k as f64;
            let rise = RIKU_GUARD_UP as f64;
            if k < rise {
                up * k / rise
            } else if k < held {
                up + (hold - up) * (k - rise) / (held - rise)
            } else {
                (hold + (last - hold) * (k - held) / (RIKU_GUARD_FRAMES as f64 - 1.0 - held)).min(last)
            }
        })
        .collect();
    Ok(Some(retime(&g, ctx.riku, &times, RIKU_GUARD, true)))
}

/// The eight cast clips on Riku's skeleton (`p002_227..234`).
pub fn riku_cast_clips(ctx: &CastCtx) -> Result<Vec<Clip>> {
    CAST_CLIPS.iter().map(|c| ctx.cast_clip(c, Skel::Riku, "p002_")).collect()
}

/// The eight cast clips on Dark Riku's own skeleton (`p003_227..234`).
pub fn dark_cast_clips(ctx: &CastCtx) -> Result<Vec<Clip>> {
    CAST_CLIPS.iter().map(|c| ctx.cast_clip(c, Skel::Dark, "p003_")).collect()
}

impl CastClip {
    /// The clip frame the key moment ends on (the latest over its source and fallbacks, so the ease never starts inside it).
    pub fn key_out(&self) -> f64 {
        let mut s = Some(self.src);
        let mut at = self.release;
        while let Some(i) = s {
            let src = &CAST_SOURCES[i];
            at = at.max(self.release + (src.key_end - src.charge_end) / (src.fps / 60.0));
            s = src.fallback;
        }
        at
    }

    /// A wind-up (Fire's `227` / `233`): its release clip follows it (blend 0) and starts where it ends.
    pub fn is_windup(&self) -> bool {
        self.release as usize >= self.len
    }
}

/// 0.198.13 (fork owner, 2026-11-26: "quero testar também as animações de magia, tem mudanças muito bruscas entre a animação e
/// voltar para o estado idle"; "todas as magias...."; "magia é o riku normal e o dark mode"): the cast clips (Terra's casts)
/// ended in Terra's recovery, 0.5..0.95 m (hands, blade) from the pose the game plays next. Each clip but the wind-ups is turned,
/// bone by bone in its local rotation, translation and scale, from its own pose toward `next` frame 0 after its key moment
/// ([`CastClip::key_out`]: the release gesture is untouched), with a smoothstep weight that reaches 1 on KH1's free frame (or the
/// last frame) and holds after: the last frame is the next pose. `ground` follows the ground six (the battle stance, row 0x2A:
/// KH1 queues the idle after a ground cast, `crate::magic` in the DLL, and in battle Re:CoM's idle row is 0x2A, Checked (code,
/// exe: the action tables `0x1407C6370` / `0x1407C67F0`)); `air` the air six (the fall, row 6: KH1 queues the fall).
pub fn ease_cast_ends(clips: &[Clip], ground: &Clip, air: &Clip) -> Vec<Clip> {
    #[cfg(test)]
    if EASE_OFF.with(|x| x.get()) {
        return clips.to_vec();
    }
    CAST_CLIPS
        .iter()
        .zip(clips)
        .enumerate()
        .map(|(k, (c, clip))| if c.is_windup() { clip.clone() } else { ease_to_next(clip, if k >= 6 { air } else { ground }, c.key_out(), c.free.min(c.len - 1) as f64) })
        .collect()
}

#[cfg(test)]
thread_local! {
    /// Tests: build the casts as before 0.198.13 (no [`ease_cast_ends`]), the "before" of the renders.
    pub static EASE_OFF: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// `clip` turned toward `next`'s frame 0 between frames `from` (weight 0) and `to` (weight 1, held after), smoothstep.
pub fn ease_to_next(clip: &Clip, next: &Clip, from: f64, to: f64) -> Clip {
    let n = clip.frames.max(1) as usize;
    let nb = clip.bones as usize;
    let mut prev: Vec<Option<[f64; 3]>> = vec![None; nb];
    let rows: Vec<Vec<[f64; 9]>> = (0..n)
        .map(|f| {
            let x = ((f as f64 - from) / (to - from).max(1.0)).clamp(0.0, 1.0);
            let w = x * x * (3.0 - 2.0 * x);
            (0..nb)
                .map(|b| {
                    let s = clip.sample(b, f as f64);
                    if w <= 0.0 || b >= next.bones as usize {
                        prev[b] = Some(s.r);
                        return [s.t[0], s.t[1], s.t[2], s.r[0], s.r[1], s.r[2], s.s[0], s.s[1], s.s[2]];
                    }
                    let e = next.sample(b, 0.0);
                    let m = rot_blend(&rot_xyz(s.r[0], s.r[1], s.r[2]), &rot_xyz(e.r[0], e.r[1], e.r[2]), w);
                    let r = crate::math::euler_xyz(&m, prev[b].or(Some(s.r)));
                    prev[b] = Some(r);
                    let l = |a: f64, z: f64| a + (z - a) * w;
                    [l(s.t[0], e.t[0]), l(s.t[1], e.t[1]), l(s.t[2], e.t[2]), r[0], r[1], r[2], l(s.s[0], e.s[0]), l(s.s[1], e.s[1]), l(s.s[2], e.s[2])]
                })
                .collect()
        })
        .collect();
    let mut c = encode(clip, &rows);
    c.flags = (c.flags & !0x1C) | (clip.flags & 0x1C);
    c
}

/// `ann` with `clip` added as its last slot (named as the clip), unless a slot already has it.
pub fn with_clip(mut ann: Ann, clip: Clip) -> Ann {
    if ann.find(&clip.name_str()).is_none() {
        ann.slots.push(Slot { name: clip.name, clip: Some(clip) });
        ann.pad_tail_like_recom();
    }
    ann
}

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
    build_with(arc, keys, riku, dark, skin, None)
}

/// [`build`], Riku's pack first given Sora's KH1 Guard (`sora`: his guard clip and skeleton;
/// 0.157.0). Riku's own pack is then one of the files (`PLAYER/01x1/PL01x1.ANN`), and the skin's
/// pack gets the guard too.
pub fn build_with(arc: &kh1_formats::egs::Archive, keys: &kh1_formats::egs::KeyTable, riku: u16, dark: u16, skin: u16, guard: Option<&GuardSource>) -> Result<Built> {
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
    let mut ann = Ann::parse(&rd(riku, "ANN")?)?;
    let mut riku_pack = None;
    let mut dark_pack = None;
    // 0.184.1: the cast clips from Dark Riku's Dark Firaga (Re:CoM's own files: always there).
    let df_ann = Ann::parse(&rd(1088, "ANN")?)?;
    let df = df_ann.slots.iter().find_map(|s| s.clip.as_ref().filter(|c| c.name_str() == DARK_FIRAGA)).ok_or_else(|| crate::Error::Format(format!("{}: no {DARK_FIRAGA}", path(1088, "ANN"))))?;
    // 0.184.4: the source clips from the sleight packs (Re:CoM's own files), each pack read once.
    let packs: std::cell::RefCell<std::collections::HashMap<u16, Ann>> = Default::default();
    let get = |g: u16, name: &str| -> Result<Clip> {
        if !packs.borrow().contains_key(&g) {
            let a = Ann::parse(&rd(g, "ANN")?)?;
            packs.borrow_mut().insert(g, a);
        }
        let p = packs.borrow();
        p[&g].slots.iter().find_map(|s| s.clip.as_ref().filter(|c| c.name_str() == name)).cloned().ok_or_else(|| crate::Error::Format(format!("{}: no {name}", path(g, "ANN"))))
    };
    let ss = RecomSkeleton::parse(&rd(1, "MDL")?)?;
    let stance = ann.slots.iter().find_map(|s| s.clip.as_ref().filter(|c| c.name_str() == RIKU_STANCE)).cloned();
    // 0.189.0: BBS FM Terra's motions (the casts and the Block), read by the step when the game has them.
    let ctx = CastCtx { get: &get, riku: &rs, dark: &ds, sora: &ss, stance: stance.as_ref(), ansem: guard.and_then(|g| g.ansem), terra: guard.and_then(|g| g.terra) };
    let riku_casts = riku_cast_clips(&ctx)?;
    let dark_casts_raw = dark_cast_clips(&ctx)?;
    let mut dann_own = Ann::parse(&rd(dark, "ANN")?)?;
    // 0.198.13: each cast ends in the pose the game plays next ([`ease_cast_ends`]): Riku's battle stance / fall, Dark Riku's.
    let clip_in = |a: &Ann, n: &str| a.slots.iter().find_map(|s| s.clip.as_ref().filter(|c| c.name_str() == n)).cloned();
    let riku_casts = match (&stance, clip_in(&ann, RIKU_FALL)) {
        (Some(st), Some(fall)) => ease_cast_ends(&riku_casts, st, &fall),
        _ => riku_casts,
    };
    let dark_casts = match (clip_in(&dann_own, DARK_STANCE), clip_in(&dann_own, DARK_FALL)) {
        (Some(st), Some(fall)) => ease_cast_ends(&dark_casts_raw, &st, &fall),
        _ => dark_casts_raw.clone(),
    };
    if let Some(src) = guard {
        // 0.189.0: Terra's Block when BBS FM is read (0.185.0 to 0.188.x: DDD Riku's, removed in 0.193.0), else KH1 Riku's
        // (0.158.0). The recoil is KH1 Riku's either way.
        let guard = match terra_guard(&ctx)? {
            Some(g) => g,
            None => riku_kh1_guard(src.mset, src.mdls, &rs)?,
        };
        let recoil = riku_kh1_recoil(src.mset, src.mdls, &rs)?;
        // 0.183.2: the real Dark Mode's Dark Riku guards with the same clips on his own skeleton, in his
        // own pack ([`dark_guard_pack`]).
        dann_own = dark_guard_pack(dann_own, &guard, &recoil, &rs, &ds)?;
        ann = with_clip(ann, guard);
        ann = with_clip(ann, recoil);
    }
    for c in &riku_casts {
        ann = with_clip(ann, c.clone());
    }
    for c in &dark_casts {
        dann_own = with_clip(dann_own, c.clone());
    }
    // 0.198.9 (third review: the skin was built from Riku's already-gripped pack, so its grip's limits measured an already-changed
    // clip): the skin retargets Riku's pack as it is before the grip, and takes only Dark Riku's grip
    let ann_plain = ann.clone();
    // 0.198.9: Riku's right hand closed on the handle where his clips leave it open (`crate::grip`, the grip of his own `p002_053`)
    {
        let own = Ann::parse(&rd(riku, "ANN")?)?;
        if !crate::grip::OFF.with(|x| x.get()) {
            crate::grip::close(&mut ann, &rmdl, &own)?;
        }
        // 0.198.10: the right hand relaxed in the clips played with the weapon put away (`crate::hands`)
        if !crate::grip::OFF.with(|x| x.get()) {
            crate::hands::relax(&mut ann, &rmdl)?;
        }
    }
    riku_pack = riku_pack.or(Some((path(riku, "ANN"), ann.to_bytes())));
    dark_pack = dark_pack.or(Some((path(dark, "ANN"), dann_own.to_bytes())));
    // 0.184.2: the Dark Firaga clip's effects and sounds on the cast clips (Riku's bones by name; Dark Riku's own).
    let df_eet = crate::eet::parse(&rd(1088, "EET")?)?;
    let df_ses = crate::ses::parse(&rd(1088, "SES")?)?;
    let to_riku = |b: i32| ds.bones.get(b as usize).and_then(|x| rs.find(if x.name == "ex_1580_sk" { "ex_1010_sk" } else { &x.name })).map(|i| i as i32);
    // 0.184.5: the cloak from Riku's own Impulse (`PL1301` `p002_062`, his bones) and Dark Riku's Dark Impulse (`PL1311`
    // `p003_072`, his).
    let cloak_rec = |g: u16, clip: &str| -> Result<crate::eet::Record> {
        crate::eet::parse(&rd(g, "EET")?)?.into_iter().find(|r| r.name == clip).ok_or_else(|| crate::Error::Format(format!("{}: no {clip}", path(g, "EET"))))
    };
    let (r_cloak, r_blade) = cloak_of(&cloak_rec(1301, "p002_062")?);
    let (d_cloak, d_blade) = cloak_of(&cloak_rec(1311, "p003_072")?);
    let (r_eet, r_ses) = cast_events(df, &df_eet, &df_ses, "p002_", &to_riku, &r_cloak, r_blade, &FxBones::of(&rs))?;
    let (d_eet, d_ses) = cast_events(df, &df_eet, &df_ses, "p003_", &|b| Some(b), &d_cloak, d_blade, &FxBones::of(&ds))?;
    let (k_eet, k_ses) = cast_events(df, &df_eet, &df_ses, "p002_", &|b| Some(b), &d_cloak, d_blade, &FxBones::of(&ds))?;
    let mut riku_eet = crate::eet::parse(&rd(riku, "EET")?)?;
    riku_eet.extend(r_eet);
    let riku_ses0 = crate::ses::parse(&rd(riku, "SES")?)?;
    let dark_ses0 = crate::ses::parse(&rd(dark, "SES")?)?;
    // 0.185.5 (fork owner, 2026-11-23: "falta audio quando o riku defende"): the Guard's whoosh and grunt at its start,
    // as Sora's has (`crate::ses::add_kh1`), from each one's own swing sounds; before, his Guard clips had no SES entry.
    let guard_ses = |own: &[crate::ses::Entry], name: &str, whoosh: &str, voice: &str| -> Result<Vec<crate::ses::Entry>> {
        Ok(if guard.is_some() { vec![crate::ses::guard_entry(own, name, whoosh, voice)?] } else { Vec::new() })
    };
    let r_guard = guard_ses(&riku_ses0, RIKU_GUARD, "p002_044", "p002_044")?;
    let d_guard = guard_ses(&dark_ses0, DARK_GUARD, "p003_056", "p003_053")?;
    let mut riku_ses = riku_ses0.clone();
    riku_ses.retain(|e| e.name() != RIKU_GUARD);
    riku_ses.extend(r_ses);
    riku_ses.extend(r_guard.iter().cloned());
    let mut dark_eet = crate::eet::parse(&rd(dark, "EET")?)?;
    dark_eet.extend(d_eet);
    let mut dark_ses = dark_ses0;
    dark_ses.retain(|e| e.name() != DARK_GUARD);
    dark_ses.extend(d_ses);
    dark_ses.extend(d_guard);
    let own_fx = vec![
        (path(riku, "EET"), crate::eet::write(&riku_eet)?),
        (path(riku, "SES"), crate::ses::write(&riku_ses)),
        (path(dark, "EET"), crate::eet::write(&dark_eet)?),
        (path(dark, "SES"), crate::ses::write(&dark_ses)),
    ];
    let mut out_ann = retarget_ann(&ann_plain, &rs, &ds)?;
    // The skin (Dark Riku's model playing Riku's rows): his cast rows play the Dark Firaga clip on its own skeleton
    // (not Riku's version retargeted back), under Riku's names. 0.198.13: eased into the skin's own next poses (Riku's stance
    // and fall on Dark Riku's skeleton: the skin plays Riku's rows; its fall is 0.45 m from Dark Riku's own).
    let skin_casts = match (clip_in(&out_ann, RIKU_STANCE), clip_in(&out_ann, RIKU_FALL)) {
        (Some(st), Some(fall)) => ease_cast_ends(&dark_casts_raw, &st, &fall),
        _ => dark_casts_raw.clone(),
    };
    for (c, d) in CAST_CLIPS.iter().zip(&skin_casts) {
        let name = format!("p002_{}", c.suffix);
        if let Some(i) = out_ann.find(&name) {
            let mut d = d.clone();
            d.name = crate::ann::name16(&name);
            out_ann.slots[i].clip = Some(d);
        }
    }
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
    // 0.198.8: the coat tails move (Dark Riku's own tail motion by role, else a follow-through; `crate::tails`)
    crate::tails::animate(&mut out_ann, &dann, &ds)?;
    // 0.198.9: the skin's right hand closed on the handle with Dark Riku's own grip
    if !crate::grip::OFF.with(|x| x.get()) {
        crate::grip::close(&mut out_ann, &dmdl, &dann)?;
    }
    // 0.198.10: and relaxed in the field walk and run (`crate::hands`)
    if !crate::grip::OFF.with(|x| x.get()) {
        // 0.198.13: the skin's left hand loose as Dark Riku's own in idle / walk / run, before the right mirrors it
        crate::hands::loosen(&mut out_ann, &dmdl, &dann)?;
        crate::hands::relax(&mut out_ann, &dmdl)?;
    }
    eet.extend(crate::eet::parse(&rd(dark, "EET")?)?.into_iter().filter(|r| r.name == DARK_FLIP));
    ses.extend(crate::ses::parse(&rd(dark, "SES")?)?.into_iter().filter(|e| e.name() == DARK_FLIP));
    // The skin's cast clips are Dark Riku's own (above): their events on his bones, under Riku's names.
    eet.extend(k_eet);
    ses.extend(k_ses);
    // 0.185.5: the skin's Guard (Riku's row on Dark Riku's model) with Dark Riku's own guard sounds, under Riku's name.
    ses.retain(|e| e.name() != RIKU_GUARD);
    ses.extend(guard_ses(&crate::ses::parse(&rd(dark, "SES")?)?, RIKU_GUARD, "p003_056", "p003_053")?);
    let clips = out_ann.slots.iter().filter(|s| s.clip.is_some()).count();
    let mut files = vec![
        (path(skin, "MDL"), rename_root(&dmdl)?),
        (path(skin, "ANN"), out_ann.to_bytes()),
        (path(skin, "EET"), crate::eet::write(&eet)?),
        (path(skin, "SES"), crate::ses::write(&ses)),
    ];
    files.extend(riku_pack);
    files.extend(dark_pack);
    files.extend(own_fx);
    for ext in ["RTM", "TXA"] {
        let (d, a, date) = raw(dark, ext)?;
        files.push((format!("raw/{}", path(skin, ext)), kh1_formats::egs::build_plain_entry(&d, &a, date)?));
    }
    Ok(Built { files, clips })
}

/// The manifest: which Riku group each skin group stands for, with its look (Dark Mode attire,
/// blade 0 Soul Eater / 1 the Keyblade of People's Hearts).
pub fn manifest_json(skins: &[Made], riku_guard: bool, event_packs: &[(u16, u16)], dark_packs: &[(u16, u16)]) -> String {
    // 0.198.30: the battle Dark Riku's groups ([`DARK_MODE_BBS_HAIR_GROUPS`], `riku` = 201 / 211) under their own key, which older DLLs
    // ignore (their `skins` take only Riku's 101 / 111).
    let (dm, sk): (Vec<&Made>, Vec<&Made>) = skins.iter().partition(|m| DARK_MODE_GROUPS.contains(&m.riku));
    let rows: Vec<String> = sk.iter().map(|m| format!("{{\"riku\": {}, \"skin\": {}, \"dark\": {}, \"blade\": {}, \"hair\": {}}}", m.riku, m.skin, m.dark, m.blade, m.hair)).collect();
    // 0.198.36: `blade` (0 the Soul Eater, 1 the Keyblade of People's Hearts; older DLLs read none and keep the first row of a hair)
    let dm: Vec<String> = dm.iter().map(|m| format!("{{\"dark\": {}, \"skin\": {}, \"hair\": {}, \"blade\": {}}}", m.riku, m.skin, m.hair, m.blade)).collect();
    // 0.184.1: `riku_cast`: Riku's and Dark Riku's own packs carry the six cast clips ([`CAST_CLIPS`]).
    // 0.197.4: `event_packs`: (Re:CoM's event pack group, its 109-bone copy) ([`EVENT_PACKS`]).
    let ev: Vec<String> = event_packs.iter().map(|(a, b)| format!("[{a}, {b}]")).collect();
    // 0.198.32: `dark_packs`: (Re:CoM's pack Dark Riku plays, its 139-bone copy for the KH2 hair) ([`DARK_PACKS`]).
    let dp: Vec<String> = dark_packs.iter().map(|(a, b)| format!("[{a}, {b}]")).collect();
    format!("{{\"format\": 1, \"skins\": [{}], \"riku_guard\": {riku_guard}, \"riku_cast\": true, \"event_packs\": [{}], \"dark_mode\": [{}], \"dark_packs\": [{}]}}\n", rows.join(", "), ev.join(", "), dm.join(", "), dp.join(", "))
}

/// One group made: the Riku group it stands for, and its look.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Made {
    /// Riku's own group (101 / 111).
    pub riku: u16,
    /// The group made.
    pub skin: u16,
    /// The Dark Mode attire.
    pub dark: bool,
    /// 0 the Soul Eater, 1 the Keyblade of People's Hearts.
    pub blade: u8,
    /// 0: Re:CoM's hair, 1: KH2 Riku's (0.186.0's 1 was DDD's hair, removed by Stage 363).
    pub hair: u8,
}

// ---------------------------------------------------------------------------------------------
// Riku's Keyblade (fork 0.160.0; fork owner, 2026-11-23: "quero implementar para o Riku a troca de
// Keyblade, vai ser simplesmente cosmética … entre a Soul Eater e a Keyblade of Heart"; "no Kingdom
// Hearts 1 essa é a única keyblade que ele utiliza, numa bossfight inclusive com o mesmo traje do
// DarkMode").
//
// - KH1 (Checked (files)): Riku-Ansem's battle model `xa_ex_1580` (voices `rk04`, the Dark Mode
//   attire) holds it as its HD mesh `-c0.cvbl` mesh 0, on joint 211 alone, texture slot 0
//   (`-1c0.dds`, 512 × 512: the dark blade, the red heart, the crown teeth).
// - Re:CoM (Checked (files)): Riku's weapon is two parts of his own model (`PL0101` / `PL0111`
//   parts 0 untextured and 1 textured, Dark Riku's `PL0201` / `PL0211` the same), every vertex on
//   `wep0` (77; Dark's 105). Re:CoM's Soul Eater on `wep0` = KH1 `xa_ex_1560`'s on its joint 191
//   × 0.01, same axes (the same bounds to the centimetre).
// - The Keyblade's joint 211 (Riku-Ansem's 247-joint skeleton) sits [`BLADE_SHIFT`] from joint
//   191's: KH1 `xa_ex_1160` (Riku in Hollow Bastion, the same skeleton) holds the Soul Eater on
//   joint 211; matched to `xa_ex_1560`'s, a shift of (4.54, −0.52, −0.02) cm (mean miss 1.2 cm:
//   the two meshes differ a little). Assumption: Riku-Ansem's grip is that Riku's.
// - Dark Riku's `wep0` is Riku's turned half a turn about its x (`BLADE_FLIP`, 0.154.3): the blade
//   is turned the same way there, so it shows as on Riku.
// - The new groups ([`BLADE_GROUPS`]) are the four looks' own copies; only their `.MDL` (the two
//   weapon parts replaced, the rest kept byte for byte) and `.RTM` (+ the Keyblade's texture)
//   differ.

/// KH1's model holding the Keyblade.
pub const BLADE_MODEL: &str = "xa_ex_1580.mdls";
/// Its HD mesh.
pub const BLADE_CVBL: &str = "-c0.cvbl";
/// The Keyblade's mesh in it.
pub const BLADE_MESH: usize = 0;
/// The joint it is on.
pub const BLADE_JOINT: u32 = 211;
/// Its texture (cvbl slot 0).
pub const BLADE_DDS: &str = "-1c0.dds";
/// Its texture's name in Re:CoM (looked up globally: one name for the four groups).
pub const BLADE_TEXTURE: &str = "rk_kophearts";
/// Joint 211 → joint 191 (KH1 units).
pub const BLADE_SHIFT: [f32; 3] = [4.54, -0.52, -0.02];

/// (base group, the group whose raw `.RTM` / `.TXA` it uses, new group, Dark Mode attire).
pub const BLADE_GROUPS: [(u16, u16, u16, bool); 4] = [(101, 101, 141, false), (111, 111, 151, false), (121, 201, 161, true), (131, 211, 171, true)];

/// 0.188.0 (fork owner, 2026-11-24: "o cabelo começa sim pelo do Riku do KH2"): Riku's default attire with KH2 Riku's hair, with the Soul Eater (182) and
/// with the Keyblade of Heart (192); the DDD hair groups 181 / 191 of 0.186.0 were removed (Stage 363). The two looks, built from group 101's files
/// ([`crate::kh2_hair`]; the 1.5+2.5 collection alone is enough): (new group, with the Keyblade).
/// 0.197.2 (item 7 of Stage 391): the same on the Halloween Town attire (111), groups 186 / 196. (new group, base group, with the
/// Keyblade, the atlas).
pub const KH2_HAIR_GROUPS: [(u16, u16, bool, crate::ddd_hair::HairTarget); 4] = [
    (182, 101, false, crate::ddd_hair::RIKU_0101_KH2),
    (192, 101, true, crate::ddd_hair::RIKU_0101_KH2),
    (186, 111, false, crate::ddd_hair::RIKU_0111_KH2),
    (196, 111, true, crate::ddd_hair::RIKU_0111_KH2),
];

/// 0.194.0 (fork owner, 2026-11-24: "agora quero implementar o cabelo do kid Riku do BBS"): the same two looks with BBS kid Riku's
/// hair ([`crate::bbs_hair`]; the 1.5+2.5 collection alone is enough): (new group, with the Keyblade).
/// 0.197.2: the Halloween Town attire, groups 187 / 197. (new group, base group, with the Keyblade, the atlas).
pub const BBS_HAIR_GROUPS: [(u16, u16, bool, crate::ddd_hair::HairTarget); 4] = [
    (183, 101, false, crate::ddd_hair::RIKU_0101_BBS),
    (193, 101, true, crate::ddd_hair::RIKU_0101_BBS),
    (187, 111, false, crate::ddd_hair::RIKU_0111_BBS),
    (197, 111, true, crate::ddd_hair::RIKU_0111_BBS),
];

/// 0.198.7 (fork owner, 2026-11-26: "agora implementa todos os cabelos para o a Skin do riku Darkmode tbm"): the Dark Form skin (groups 121 /
/// 131, Dark Riku's model with Riku's motions) with KH2 Riku's hair, with the Soul Eater and with the Keyblade of Heart: (new group, base
/// group (the skin's, built first), the group of its RTM / TXA (Dark Riku's), with the Keyblade, the atlas). Groups 184 / 188 / 194 / 198.
pub const DARK_KH2_HAIR_GROUPS: [(u16, u16, u16, bool, crate::ddd_hair::HairTarget); 4] = [
    (184, 121, 201, false, crate::ddd_hair::RIKU_0201_KH2),
    (194, 121, 201, true, crate::ddd_hair::RIKU_0201_KH2),
    (188, 131, 211, false, crate::ddd_hair::RIKU_0211_KH2),
    (198, 131, 211, true, crate::ddd_hair::RIKU_0211_KH2),
];
/// The same with BBS kid Riku's hair: groups 185 / 189 / 195 / 199.
pub const DARK_BBS_HAIR_GROUPS: [(u16, u16, u16, bool, crate::ddd_hair::HairTarget); 4] = [
    (185, 121, 201, false, crate::ddd_hair::RIKU_0201_BBS),
    (195, 121, 201, true, crate::ddd_hair::RIKU_0201_BBS),
    (189, 131, 211, false, crate::ddd_hair::RIKU_0211_BBS),
    (199, 131, 211, true, crate::ddd_hair::RIKU_0211_BBS),
];

/// 0.198.30 (fork owner, 2026-11-27: "reparei que quando o personagem se transforma no DarkMode o cabelo é resetado para o Default"; "SIM
/// pode" to building the hairs for the battle's Dark Riku): Re:CoM's own Dark Mode (Dark Riku, chars 5 / 6, groups 201 / 211, Checked (exe:
/// `0x140211960` preloads `0x1405A6910[5 / 6]`, the factory `0x14020EC70` builds `CPlayerDRiku` on it)) with BBS kid Riku's hair: (new group,
/// Dark Riku's group (the base: his model, his own pack with the guard and cast clips the step adds, his EET / SES, RTM / TXA), the atlas).
/// BBS's hair keeps his 128 bones, so his sleight and event packs (1087-1089, 1093, 1310-1313, 4003, 4043, 4083: 128 bones, Checked (files,
/// test `probe_pack_bone_counts`)) still play on him. KH2's hair is not built for him: it makes him 139 bones and none of those would play.
/// Groups 205 / 206: free in Re:CoM's PLAYER table and in Recom.hed (214 / 215 have files there; same test).
pub const DARK_MODE_BBS_HAIR_GROUPS: [(u16, u16, crate::ddd_hair::HairTarget); 2] = [(205, 201, crate::ddd_hair::RIKU_0201_BBS), (206, 211, crate::ddd_hair::RIKU_0211_BBS)];
/// Dark Riku's two groups (chars 5 / 6).
pub const DARK_MODE_GROUPS: [u16; 2] = [201, 211];

/// 0.198.32 (fork owner, 2026-11-27, asked to choose between 139-bone copies of his 11 packs with a second hook at `0x14024ED00` and
/// fitting KH2's hair on his 18 hair bones: "a"): the battle's Dark Riku (chars 5 / 6) with KH2 Riku's hair: (new group, Dark Riku's
/// group (the base: his model, his own pack with the guard and cast clips the step adds, his EET / SES, RTM / TXA), the atlas). Groups
/// 207 / 208: free in Re:CoM's PLAYER table (Checked (exe: the table at `0x1408055D0`, 553 rows; 200..299 holds only 201 / 211)) and in
/// Recom.hed (test `probe_pack_bone_counts`, `keyblade::tests::new_names_are_not_in_recom_hed`). KH2's hair makes him 139 bones, so
/// every 128-bone pack he plays gets a copy ([`DARK_PACKS`]); the manifest lists these groups only when all the copies were built.
pub const DARK_MODE_KH2_HAIR_GROUPS: [(u16, u16, crate::ddd_hair::HairTarget); 2] = [(207, 201, crate::ddd_hair::RIKU_0201_KH2), (208, 211, crate::ddd_hair::RIKU_0211_KH2)];

/// 0.198.36 (fork owner, 2026-11-27: "no darkmode a keyblade tbm nn troca, já coloca essas regrs para o futuro"): the battle's Dark Riku
/// (chars 5 / 6) in every look the player can choose, one row per (Dark Riku's group, hair, Keyblade): (new group, Dark Riku's group 201 /
/// 211, hair 0 Re:CoM's / 1 KH2 Riku's / 2 BBS kid Riku's, blade 0 the Soul Eater / 1 the Keyblade of People's Hearts). His own group is
/// (Re:CoM's hair, the Soul Eater) and has no row. One builder for every row ([`build_dark_mode`]); the DLL picks a row by the rule it uses
/// for Riku's looks (`kh1-recom`'s `riku_skin::group_for`: a missing (blade, hair) keeps the Keyblade and falls back on Re:CoM's hair).
/// The Soul Eater rows first: older DLLs read only `hair` and keep the first row of a hair. The Keyblade sits on Dark Riku's model as on
/// the Dark Form costume's 161 / 171 ([`with_blade`], turned over); its hit volume and trails stay the Soul Eater's (no table of Re:CoM's
/// changes). The KH2 rows (139 bones) share the pack copies ([`DARK_PACKS`]: one per skeleton, not per blade). Groups 209 / 210, 212 /
/// 213, 216 / 217: free in Re:CoM's PLAYER table (Checked (exe: `0x1408055D0`, 200..299 holds only 201 / 211), Stage 432) and in
/// Recom.hed (214 / 215 have files there; tests `probe_pack_bone_counts`, `keyblade::tests::new_names_are_not_in_recom_hed`).
pub const DARK_MODE_LOOKS: [(u16, u16, u8, u8); 10] = [
    (205, 201, 2, 0),
    (206, 211, 2, 0),
    (207, 201, 1, 0),
    (208, 211, 1, 0),
    (209, 201, 0, 1),
    (210, 211, 0, 1),
    (212, 201, 2, 1),
    (213, 211, 2, 1),
    (216, 201, 1, 1),
    (217, 211, 1, 1),
];

/// A row of [`DARK_MODE_LOOKS`] in words, for the step's lines.
pub fn dark_look_name(hair: u8, blade: u8) -> String {
    format!("{} and the {}", ["Re:CoM's hair", "KH2 Riku's hair", "BBS kid Riku's hair"].get(hair as usize).unwrap_or(&"?"), if blade == 1 { "Keyblade of People's Hearts" } else { "Soul Eater" })
}

/// One row of [`DARK_MODE_LOOKS`] (`look`: (new group, Dark Riku's group, hair, blade)), built from Dark Riku's own files (`have`: his pack
/// as the step wrote it, else Re:CoM's): Re:CoM's hair with the Keyblade = [`build_blade`] on his group; KH2's / BBS's hair = their
/// builders with the Keyblade or the Soul Eater, on the hair's atlas for his group ([`DARK_MODE_KH2_HAIR_GROUPS`] /
/// [`DARK_MODE_BBS_HAIR_GROUPS`]). An error when a source the row needs is missing.
#[allow(clippy::too_many_arguments)]
pub fn build_dark_mode(arc: &kh1_formats::egs::Archive, keys: &kh1_formats::egs::KeyTable, have: &std::collections::HashMap<String, Vec<u8>>, look: (u16, u16, u8, u8), kh2: Option<&crate::kh2_hair::Kh2HairSource>, bbs: Option<&crate::bbs_hair::BbsHairSource>, blade: Option<&BladeSource>) -> Result<Built> {
    let (target, dark, hair, b) = look;
    let blade = match b {
        0 => None,
        _ => Some(blade.ok_or_else(|| crate::Error::Format(format!("no Keyblade of People's Hearts (KH1's {BLADE_MODEL})")))?),
    };
    let atlas = |groups: &[(u16, u16, crate::ddd_hair::HairTarget)]| groups.iter().find(|x| x.1 == dark).map(|x| x.2).ok_or_else(|| crate::Error::Format(format!("no hair atlas for group {dark}")));
    let missing = |what: &str| crate::Error::Format(format!("{what} not read"));
    match (hair, blade) {
        (0, Some(src)) => build_blade(arc, keys, have, dark, dark, target, true, src),
        (0, None) => Err(crate::Error::Format(format!("group {target}: Dark Riku's own look (group {dark})"))),
        (1, _) => build_dark_mode_kh2(arc, keys, have, dark, target, kh2.ok_or_else(|| missing("KH2 Riku's model"))?, &atlas(&DARK_MODE_KH2_HAIR_GROUPS)?, blade),
        (2, _) => build_dark_mode_bbs(arc, keys, have, dark, target, bbs.ok_or_else(|| missing("BBS kid Riku's model"))?, &atlas(&DARK_MODE_BBS_HAIR_GROUPS)?, blade),
        _ => Err(crate::Error::Format(format!("group {target}: hair {hair}"))),
    }
}

/// Riku's `PL0101.ANN` slot → Dark Riku's `PL0201` / `PL0211.ANN` slot playing the same action row. Checked (exe: the action tables
/// `0x1407C6300[3]` and `[5]`, 16-byte rows of clip names; row by row: idle 1 → 1, the stance 0x2A `p002_031` → `p003_021`, walk 2 → 2,
/// row 0x12 `p002_028` → `p003_013`, 0x2B `p002_032` → `p003_022`, run 3 → 3, 0x13 `p002_021` → `p003_014`, 0x2C `p002_033` →
/// `p003_023`, jump 4 → 4, 0x14 `p002_026` → `p003_096`, fall 5 → 6, 6 → 7, land 7 → 8, 0xF `p002_016` → `p003_011`, damage 0x1F
/// `p002_037` → `p003_027`, 0x20 `p002_038` → `p003_028`, the first swing 0x30 `p002_044` → `p003_034`; Riku's 79 / 80 (rows 0x18 /
/// 0x19) have no clip of his). Each pack's slot `n` holds clip `p00x_n` (test `dark_riku_slots_follow_rikus_rows`).
pub const DARK_SLOT_OF: [(usize, usize); 17] = [(1, 1), (31, 21), (2, 2), (28, 13), (32, 22), (3, 3), (21, 14), (33, 23), (4, 4), (26, 96), (5, 6), (6, 7), (7, 8), (16, 11), (37, 27), (38, 28), (44, 34)];

/// [`crate::kh2_hair_rig::CLIP_MAP`] / [`crate::kh2_hair_rig::UNROLL`] moved to Dark Riku's slots ([`DARK_SLOT_OF`]), so his idle
/// sways, his walk and run take KH2's walk and run, as on Riku: the KH2 hair's motion on his own pack.
pub fn dark_slots() -> crate::kh2_hair_rig::Slots<'static> {
    use crate::kh2_hair_rig as r;
    static MAP: std::sync::OnceLock<(Vec<(usize, usize)>, Vec<usize>)> = std::sync::OnceLock::new();
    let (map, unroll) = MAP.get_or_init(|| {
        let to = |s: usize| DARK_SLOT_OF.iter().find(|x| x.0 == s).map(|x| x.1);
        (r::CLIP_MAP.iter().filter_map(|&(s, m)| Some((to(s)?, m))).collect(), r::UNROLL.iter().filter_map(|&s| to(s)).collect())
    });
    r::Slots { map, unroll }
}

/// 0.198.32: the 128-bone packs the battle's Dark Riku plays besides his own (Checked (files: `probe_pack_bone_counts`; exe)): his
/// sleights Dark Break 1087, Dark Firaga 1088, Dark Aura 1089, Inverse Burst 1093, Dark Impulse / Maelstrom / Barrage 1311..1313 (the
/// sleight → group switch `0x14020F720` → `0x14020FB10`, played by `0x1401F73A0`), his dash 1310 (`0x1401F7290(…, 0x51E, …)` in
/// `0x1402007E0`, slot 0x140 of `CPlayerDRiku`'s vtable `0x1405A50E8`), and his event packs 4003 / 4043 / 4083 (`0x1401F7290`, the
/// knowledge note `recom-skeleton-limits.md` § event packs). Both paths end in `0x1401A3580`, which takes the pack by id in
/// `0x14024D860` (blend ≥ 0) or `0x14024ED00` (blend < 0: most sleight calls pass -1). Each gets a 139-bone copy in group
/// `+ DARK_ALT` (1187 … 1413, 4103 / 4143 / 4183: free in the PLAYER table and in Recom.hed).
pub const DARK_PACKS: [u16; 11] = [1087, 1088, 1089, 1093, 1310, 1311, 1312, 1313, 4003, 4043, 4083];
/// See [`DARK_PACKS`].
pub const DARK_ALT: u16 = 100;

/// The 139-bone copies of [`DARK_PACKS`] for the KH2 hair on Dark Riku's model `PL<dark>.MDL` (`hair`'s rig): (source group, copy group,
/// the copy's `.ANN`), every clip with KH2's idle sway on the hair (their slots are not his own pack's). All or an error: a Dark Riku
/// whose sleight would not play is not built.
pub fn build_dark_packs(arc: &kh1_formats::egs::Archive, keys: &kh1_formats::egs::KeyTable, dark: u16, hair: &crate::kh2_hair::Kh2HairSource) -> Result<Vec<(u16, u16, Vec<u8>)>> {
    let mdl = crate::install::read_recom(arc, keys, &path(dark, "MDL"))?;
    let bones = crate::mdl::RecomSkeleton::parse(&mdl)?.bones.len();
    let mut out = vec![];
    for src in DARK_PACKS {
        let ann = crate::install::read_recom(arc, keys, &path(src, "ANN"))?;
        let a = Ann::parse(&ann)?;
        if a.slots.iter().filter_map(|s| s.clip.as_ref()).any(|c| c.bones as usize != bones) {
            return Err(crate::Error::Format(format!("{}: not {bones} bones", path(src, "ANN"))));
        }
        let b = hair.bake_pack(&ann, &mdl, false)?.ok_or_else(|| crate::Error::Format("KH2 hair: no rig".into()))?;
        out.push((src, src + DARK_ALT, b));
    }
    Ok(out)
}

/// The battle's Dark Riku (`dark`, 201 / 211: his own model and pack as the step wrote them, `have`) with KH2 Riku's hair as group
/// `target`: [`build_hair_from`], then his pack's hair baked on his own slots ([`dark_slots`]), not Riku's.
/// 0.198.36: `blade` the Keyblade of People's Hearts (None: the Soul Eater).
#[allow(clippy::too_many_arguments)]
pub fn build_dark_mode_kh2(arc: &kh1_formats::egs::Archive, keys: &kh1_formats::egs::KeyTable, have: &std::collections::HashMap<String, Vec<u8>>, dark: u16, target: u16, hair: &crate::kh2_hair::Kh2HairSource, atlas: &crate::ddd_hair::HairTarget, blade: Option<&BladeSource>) -> Result<Built> {
    let mut b = build_hair_from(arc, keys, have, dark, dark, true, target, blade, hair, atlas)?;
    let get = |ext: &str| -> Result<Vec<u8>> {
        let n = path(dark, ext);
        match have.get(&n) {
            Some(b) => Ok(b.clone()),
            None => crate::install::read_recom(arc, keys, &n),
        }
    };
    let ann = hair.bake_pack_slots(&get("ANN")?, &get("MDL")?, dark_slots())?.ok_or_else(|| crate::Error::Format("KH2 hair: no rig".into()))?;
    let at = path(target, "ANN");
    let f = b.files.iter_mut().find(|f| f.0 == at).ok_or_else(|| crate::Error::Format(format!("{at} not built")))?;
    f.1 = ann;
    Ok(b)
}

/// The battle's Dark Riku (`dark`, 201 / 211: his own model and pack as the step wrote them, `have`) with BBS kid Riku's hair as group
/// `target` (0.198.33): [`build_hair_from`], then his pack's hair sway baked on his own slots ([`dark_slots`]), not Riku's (0.198.30
/// used Riku's map on his pack: his battle stance, slot 21, swayed as the run, 28 as the walk, 33 as the run; model-reviewer of
/// 0.198.32).
/// 0.198.36: `blade` the Keyblade of People's Hearts (None: the Soul Eater).
#[allow(clippy::too_many_arguments)]
pub fn build_dark_mode_bbs(arc: &kh1_formats::egs::Archive, keys: &kh1_formats::egs::KeyTable, have: &std::collections::HashMap<String, Vec<u8>>, dark: u16, target: u16, hair: &crate::bbs_hair::BbsHairSource, atlas: &crate::ddd_hair::HairTarget, blade: Option<&BladeSource>) -> Result<Built> {
    // `build_hair_from` gives the source his model (`prepare_mdl`): the rig the re-bake below uses
    let mut b = build_hair_from(arc, keys, have, dark, dark, true, target, blade, hair, atlas)?;
    let ann = match have.get(&path(dark, "ANN")) {
        Some(a) => a.clone(),
        None => crate::install::read_recom(arc, keys, &path(dark, "ANN"))?,
    };
    let ann = hair.bake_ann_slots(&ann, dark_slots().map)?.ok_or_else(|| crate::Error::Format("BBS hair: no sway".into()))?;
    let at = path(target, "ANN");
    let f = b.files.iter_mut().find(|f| f.0 == at).ok_or_else(|| crate::Error::Format(format!("{at} not built")))?;
    f.1 = ann;
    Ok(b)
}

/// 0.197.4 (item 1 of Stage 391; the workspace's `knowledge/ReCoM/recom-skeleton-limits.md` § event packs, Checked (decomp, files)):
/// Riku's event motion packs, 102 bones each (the boss end's card get 4002, and 4012 … 4082 of other event states). Re:CoM plays no
/// clip whose bone count differs from the model's, so a 109-bone Riku (the KH2 hair) never ends those motions and the event waits
/// forever. Each gets a 109-bone copy in group `+ EVENT_ALT`, which the DLL picks where Re:CoM fetches the pack.
pub const EVENT_PACKS: [u16; 8] = [4002, 4012, 4032, 4042, 4052, 4062, 4072, 4082];
/// See [`EVENT_PACKS`].
pub const EVENT_ALT: u16 = 100;

/// The 109-bone copies of [`EVENT_PACKS`] for the KH2 hair (`hair`'s rig on `PL0101`): (source group, copy group, the copy's
/// `.ANN`). A pack that is missing or not 102 bones is left out.
pub fn build_event_packs(arc: &kh1_formats::egs::Archive, keys: &kh1_formats::egs::KeyTable, hair: &crate::kh2_hair::Kh2HairSource) -> Result<Vec<(u16, u16, Vec<u8>)>> {
    let mdl = crate::install::read_recom(arc, keys, &path(101, "MDL"))?;
    let bones = crate::mdl::RecomSkeleton::parse(&mdl)?.bones.len();
    let mut out = vec![];
    for src in EVENT_PACKS {
        let Ok(ann) = crate::install::read_recom(arc, keys, &path(src, "ANN")) else { continue };
        // 0.198.21: the level-up pack's pant panels in the model's bone order first ([`crate::pants_order`]), so the copy gets them too
        let ann = match crate::pants_order::FIXED_PACKS.contains(&src) {
            true => crate::pants_order::fix_bytes(&ann, &crate::mdl::RecomSkeleton::parse(&mdl)?)?.unwrap_or(ann),
            false => ann,
        };
        let a = Ann::parse(&ann)?;
        if a.slots.iter().filter_map(|s| s.clip.as_ref()).any(|c| c.bones as usize != bones) {
            continue;
        }
        if let Some(b) = hair.bake_pack(&ann, &mdl, false)? {
            out.push((src, src + EVENT_ALT, b));
        }
    }
    Ok(out)
}

/// One hair group: Riku's group 101 (`have`: what the step wrote this run, else Re:CoM's) under `target`'s names, with
/// `hair` (KH2 Riku's) on its model and atlas `atlas` ([`crate::ddd_hair::with_hair`]) and, when `blade` is given, the Keyblade of Heart.
pub fn build_hair(arc: &kh1_formats::egs::Archive, keys: &kh1_formats::egs::KeyTable, have: &std::collections::HashMap<String, Vec<u8>>, base: u16, target: u16, blade: Option<&BladeSource>, hair: &dyn crate::ddd_hair::HairSource, atlas: &crate::ddd_hair::HairTarget) -> Result<Built> {
    build_hair_from(arc, keys, have, base, base, false, target, blade, hair, atlas)
}

/// [`build_hair`] on any base (0.198.7): `raw_src` is the group whose RTM / TXA the base uses (Dark Riku's 201 / 211 for the Dark Form
/// skin's 121 / 131, whose own groups are not in Recom.hed), `dark` that the model is Dark Riku's (the Keyblade's bones, [`with_blade`]).
#[allow(clippy::too_many_arguments)]
pub fn build_hair_from(arc: &kh1_formats::egs::Archive, keys: &kh1_formats::egs::KeyTable, have: &std::collections::HashMap<String, Vec<u8>>, base: u16, raw_src: u16, dark: bool, target: u16, blade: Option<&BladeSource>, hair: &dyn crate::ddd_hair::HairSource, atlas: &crate::ddd_hair::HairTarget) -> Result<Built> {
    let get = |ext: &str| -> Result<Vec<u8>> {
        let n = path(base, ext);
        match have.get(&n) {
            Some(b) => Ok(b.clone()),
            None => crate::install::read_recom(arc, keys, &n),
        }
    };
    let raw = |ext: &str| -> Result<(Vec<u8>, Vec<kh1_formats::egs::HdAsset>, i32)> {
        let n = path(raw_src, ext);
        let e = arc.find(&n).ok_or_else(|| crate::Error::Format(format!("{n} not in Recom.hed")))?;
        let raw = arc.read_raw_entry(e)?;
        let date = raw.get(12..16).map_or(0, |b| i32::from_le_bytes(b.try_into().unwrap()));
        let (d, a) = kh1_formats::egs::decode_entry_full(keys, &raw)?;
        Ok((d, a, date))
    };
    let (d, a, date) = raw("RTM")?;
    let mdl0 = get("MDL")?;
    let swap = crate::ddd_hair::with_hair(&hair.prepare_mdl(&mdl0)?, &d, &a, atlas, hair)?;
    let ann = hair.bake_ann(&get("ANN")?, &mdl0)?;
    let (mdl, rtm) = match blade {
        Some(src) => (with_blade(&swap.mdl, src, dark)?, rtm_with_blade(&swap.rtm, &swap.hd, date, src)?),
        None => (swap.mdl.clone(), kh1_formats::egs::build_plain_entry(&swap.rtm, &swap.hd, date)?),
    };
    let mut files = vec![(path(target, "MDL"), mdl)];
    files.push((path(target, "ANN"), match ann {
        Some(a) => a,
        None => get("ANN")?,
    }));
    for ext in ["EET", "SES"] {
        files.push((path(target, ext), get(ext)?));
    }
    files.push((format!("raw/{}", path(target, "RTM")), rtm));
    let (d, a, date) = raw("TXA")?;
    files.push((format!("raw/{}", path(target, "TXA")), kh1_formats::egs::build_plain_entry(&d, &a, date)?));
    let clips = Ann::parse(&files[1].1)?.slots.iter().filter(|s| s.clip.is_some()).count();
    Ok(Built { files, clips })
}

/// The Keyblade's sources: KH1's HD mesh and texture.
pub struct BladeSource {
    /// The HD mesh file.
    pub cvbl: Vec<u8>,
    /// The texture (DXT).
    pub dds: Vec<u8>,
}

impl BladeSource {
    /// From `xa_ex_1580.mdls`'s HD assets.
    pub fn from_assets(assets: &[kh1_formats::egs::HdAsset]) -> Result<BladeSource> {
        let get = |n: &str| assets.iter().find(|a| a.name == n).map(|a| a.data.clone()).ok_or_else(|| crate::Error::Format(format!("{BLADE_MODEL}: no {n}")));
        Ok(BladeSource { cvbl: get(BLADE_CVBL)?, dds: get(BLADE_DDS)? })
    }
}

/// `mdl` (Riku's or Dark Riku's, parts 0 / 1 his weapon on `wep0`) with the Keyblade as its weapon.
pub fn with_blade(mdl: &[u8], src: &BladeSource, flip: bool) -> Result<Vec<u8>> {
    use crate::mdl::{join_parts, split_parts, MdlPart, FMT_PLAIN, FMT_TEXTURED};
    let mut parts = split_parts(mdl)?;
    if parts.len() < 3 {
        return Err(crate::Error::Format("Riku's model: fewer than 3 parts".into()));
    }
    let p1 = MdlPart::parse_bytes(&parts[1])?;
    let p2 = MdlPart::parse_bytes(&parts[2])?;
    let wep0 = p1.bones.iter().position(|b| crate::ann::cstr(&b.name) == "wep0").ok_or_else(|| crate::Error::Format("Riku's model: no wep0".into()))?;
    let only_wep0 = |p: &MdlPart| p.list_a.iter().chain(p.list_b.iter().flatten()).all(|k| k.verts.iter().all(|v| v.bone() == wep0));
    let tx = p1.list_a.first().filter(|k| k.fmt == FMT_TEXTURED).ok_or_else(|| crate::Error::Format("Riku's model: part 1 is not textured".into()))?;
    let pl = p2.list_a.first().filter(|k| k.fmt == FMT_PLAIN).ok_or_else(|| crate::Error::Format("Riku's model: part 2 is not untextured".into()))?;
    if !only_wep0(&p1) || parts[0].len() < 0x30 || u16::from_le_bytes([parts[0][0], parts[0][1]]) as usize != p1.bones.len() {
        return Err(crate::Error::Format("Riku's model: parts 0 / 1 are not his weapon alone".into()));
    }
    let cv = kh1_formats::cvbl::Cvbl::parse(&src.cvbl)?;
    let (_, _, img) = crate::keyblade::texture_entry(BLADE_TEXTURE, &src.dds)?;
    let place = |p: [f32; 3]| {
        let q = [(p[0] + BLADE_SHIFT[0]) * 0.01, (p[1] + BLADE_SHIFT[1]) * 0.01, (p[2] + BLADE_SHIFT[2]) * 0.01];
        if flip {
            [q[0], -q[1], -q[2]]
        } else {
            q
        }
    };
    let bp = crate::keyblade::blade_parts(&cv, BLADE_MESH, BLADE_JOINT, wep0 as u32, &place, &img, (tx.gif, tx.regs), (pl.gif, pl.regs))?;
    let nb = p1.bones.len();
    let mut name = [0u8; 32];
    name[..BLADE_TEXTURE.len()].copy_from_slice(BLADE_TEXTURE.as_bytes());
    let textured = MdlPart {
        bounds: crate::keyblade::part_bounds(nb, &[&bp.textured_a, bp.textured_b.as_deref().unwrap_or(&[])]),
        textures: vec![name],
        list_a: bp.textured_a,
        list_b: bp.textured_b,
        ..p1.clone()
    };
    let plain = MdlPart {
        flags: [parts[0][2], parts[0][3]],
        filler: parts[0][0x20..0x30].try_into().unwrap(),
        bones: p1.bones.clone(),
        rest: p1.rest.clone(),
        textures: vec![],
        bounds: crate::keyblade::part_bounds(nb, &[&bp.plain]),
        list_a: bp.plain,
        list_b: None,
    };
    parts[0] = plain.write_bytes()?;
    parts[1] = textured.write_bytes()?;
    Ok(join_parts(&parts))
}

/// A raw `.RTM` entry (`(data, HD assets, date)` as the archive holds it) with the Keyblade's
/// texture added (TIM2 + HD), as a plain entry for Panacea's raw route.
pub fn rtm_with_blade(d: &[u8], assets: &[kh1_formats::egs::HdAsset], date: i32, src: &BladeSource) -> Result<Vec<u8>> {
    let mut rtm = crate::rtm::Rtm::parse(d)?;
    let (e, hd, _) = crate::keyblade::texture_entry(BLADE_TEXTURE, &src.dds)?;
    rtm.entries.retain(|x| x.name_str() != BLADE_TEXTURE);
    rtm.entries.push(e);
    let mut a = assets.to_vec();
    a.retain(|x| x.name != hd.name);
    a.push(hd);
    Ok(kh1_formats::egs::build_plain_entry(&rtm.write(), &a, date)?)
}

/// One Keyblade group: `base`'s files (`have`: what the step wrote this run, else Re:CoM's) under
/// `target`'s names, its model with the Keyblade, its raw `.RTM` / `.TXA` from `raw_src`.
pub fn build_blade(arc: &kh1_formats::egs::Archive, keys: &kh1_formats::egs::KeyTable, have: &std::collections::HashMap<String, Vec<u8>>, base: u16, raw_src: u16, target: u16, dark: bool, src: &BladeSource) -> Result<Built> {
    let get = |ext: &str| -> Result<Vec<u8>> {
        let n = path(base, ext);
        match have.get(&n) {
            Some(b) => Ok(b.clone()),
            None => crate::install::read_recom(arc, keys, &n),
        }
    };
    let raw = |ext: &str| -> Result<(Vec<u8>, Vec<kh1_formats::egs::HdAsset>, i32)> {
        let n = path(raw_src, ext);
        let e = arc.find(&n).ok_or_else(|| crate::Error::Format(format!("{n} not in Recom.hed")))?;
        let raw = arc.read_raw_entry(e)?;
        let date = raw.get(12..16).map_or(0, |b| i32::from_le_bytes(b.try_into().unwrap()));
        let (d, a) = kh1_formats::egs::decode_entry_full(keys, &raw)?;
        Ok((d, a, date))
    };
    let mut files = vec![(path(target, "MDL"), with_blade(&get("MDL")?, src, dark)?)];
    for ext in ["ANN", "EET", "SES"] {
        files.push((path(target, ext), get(ext)?));
    }
    let (d, a, date) = raw("RTM")?;
    files.push((format!("raw/{}", path(target, "RTM")), rtm_with_blade(&d, &a, date, src)?));
    let (d, a, date) = raw("TXA")?;
    files.push((format!("raw/{}", path(target, "TXA")), kh1_formats::egs::build_plain_entry(&d, &a, date)?));
    let clips = Ann::parse(&files[1].1)?.slots.iter().filter(|s| s.clip.is_some()).count();
    Ok(Built { files, clips })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::install;

    /// Probe: every PLAYER `.ANN` in Recom.hed with clips at Dark Riku's 128 bones or Riku's 102.
    #[test]
    fn probe_pack_bone_counts() {
        let Some(g) = game() else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        for grp in 1..10000u16 {
            let n = path(grp, "ANN");
            if arc.find(&n).is_none() { continue }
            let Ok(a) = install::read_recom(&arc, &keys, &n).and_then(|b| Ann::parse(&b)) else { eprintln!("{n}: unreadable"); continue };
            let mut c: std::collections::BTreeMap<u16, Vec<String>> = Default::default();
            for s in a.slots.iter().filter_map(|s| s.clip.as_ref()) { c.entry(s.bones as u16).or_default().push(s.name_str().to_string()); }
            if c.contains_key(&128) || c.contains_key(&102) { eprintln!("{grp}: {:?}", c.iter().map(|(k, v)| (k, v.len(), v.first().cloned())).collect::<Vec<_>>()); }
        }
        let used: Vec<u16> = (200..300u16).filter(|&grp| ["MDL", "ANN", "RTM", "TXA", "EET", "SES", "BIN", "SND"].iter().any(|e| arc.find(&path(grp, e)).is_some())).collect();
        eprintln!("PLAYER groups 200..299 with a file in Recom.hed: {used:?}");
    }

    /// With the owner's installs (`RECOM_GAME`): the Keyblade replaces Riku's weapon parts alone
    /// (the other parts byte for byte), on `wep0`, its grip where the Soul Eater's is (the
    /// Keyblade spans the Soul Eater's grip region), turned over on Dark Riku's model.
    #[test]
    fn the_keyblade_replaces_rikus_weapon_parts() {
        let Some(g) = game() else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let Ok((_, assets, _)) = install::kh1(&g, keys.clone()).unwrap().read_full(BLADE_MODEL) else { return };
        let src = BladeSource::from_assets(&assets).unwrap();
        for (name, flip, wep0) in [("PLAYER/0101/PL0101.MDL", false, 77usize), ("PLAYER/0201/PL0201.MDL", true, 105)] {
            let old = install::read_recom(&arc, &keys, name).unwrap();
            let new = with_blade(&old, &src, flip).unwrap();
            let (a, b) = (crate::mdl::split_parts(&old).unwrap(), crate::mdl::split_parts(&new).unwrap());
            assert_eq!(a.len(), b.len());
            assert_eq!(a[2..], b[2..], "{name}: the body kept byte for byte");
            let p = crate::mdl::MdlPart::parse_bytes(&b[1]).unwrap();
            assert_eq!(p.texture_names(), [BLADE_TEXTURE]);
            let vs: Vec<_> = p.list_a.iter().chain(p.list_b.iter().flatten()).flat_map(|k| k.verts.iter()).collect();
            assert!(vs.iter().all(|v| v.bone() == wep0));
            let xs: Vec<f32> = vs.iter().map(|v| v.pos()[0]).collect();
            let (lo, hi) = (xs.iter().cloned().fold(f32::MAX, f32::min), xs.iter().cloned().fold(f32::MIN, f32::max));
            assert!(lo < -1.0 && hi > 0.2 && hi < 0.3, "{name}: {lo}..{hi} m along the blade");
            let ys: Vec<f32> = vs.iter().map(|v| v.pos()[1]).collect();
            let ymax = ys.iter().cloned().fold(f32::MIN, f32::max);
            assert!(if flip { ymax < 0.18 } else { ymax > 0.18 }, "{name}: turned over on Dark's model ({ymax})");
            assert!(crate::mdl::MdlPart::parse_bytes(&b[0]).is_ok());
        }
    }

    fn game() -> Option<std::path::PathBuf> {
        std::env::var_os("RECOM_GAME").map(Into::into)
    }

    /// 0.188.0 (owner's install, `RECOM_GAME`, the 1.5+2.5 collection only): the KH2 hair groups are built from group 101 with
    /// their own atlas name (both hairs can be in the game).
    /// 0.198.7: the Dark Form skin's hair groups (KH2's and BBS's, with the Soul Eater and the Keyblade of Heart, default and Halloween
    /// Town): built on the skin's own files, not in Recom.hed, every clip with the model's bone count, the hair changed, the coat's list
    /// B kept. The swapped models go to the workspace's `local/assets/recom/riku-hair/out/dark/` for the renders.
    #[test]
    fn the_dark_hair_groups_are_built_from_the_dark_skin() {
        let Some(g) = game() else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let kh2 = crate::kh2_hair::Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap();
        let bbs = crate::bbs_hair::BbsHairSource::read(&g, &keys).unwrap();
        let Ok((_, assets, _)) = install::kh1(&g, keys.clone()).unwrap().read_full(BLADE_MODEL) else { return };
        let src = BladeSource::from_assets(&assets).unwrap();
        let mut have = std::collections::HashMap::new();
        for &(riku, dark, skin) in &SKIN_GROUPS {
            for (rel, bytes) in build_with(&arc, &keys, riku, dark, skin, None).unwrap().files {
                have.insert(rel, bytes);
            }
        }
        let out = std::path::PathBuf::from(format!("{}/local/assets/recom/riku-hair/out/dark", crate::kh2_motion::tests::workspace()));
        let _ = std::fs::create_dir_all(&out);
        let hairs: [(&dyn crate::ddd_hair::HairSource, &[(u16, u16, u16, bool, crate::ddd_hair::HairTarget); 4]); 2] = [(&kh2, &DARK_KH2_HAIR_GROUPS), (&bbs, &DARK_BBS_HAIR_GROUPS)];
        for (hair, groups) in hairs {
            for &(target, base, raw_src, heart, atlas) in groups {
                assert!(arc.find(&path(target, "MDL")).is_none(), "group {target} is free in Recom.hed");
                let b = build_hair_from(&arc, &keys, &have, base, raw_src, true, target, if heart { Some(&src) } else { None }, hair, &atlas).unwrap();
                let names: Vec<&str> = b.files.iter().map(|f| f.0.as_str()).collect();
                for ext in ["MDL", "ANN", "EET", "SES"] {
                    assert!(names.contains(&path(target, ext).as_str()), "{target}: {ext}");
                }
                let mdl = &b.files.iter().find(|f| f.0 == path(target, "MDL")).unwrap().1;
                let ann = Ann::parse(&b.files.iter().find(|f| f.0 == path(target, "ANN")).unwrap().1).unwrap();
                let sk = RecomSkeleton::parse(mdl).unwrap();
                let bad: Vec<usize> = ann.slots.iter().enumerate().filter_map(|(i, s)| s.clip.as_ref().filter(|c| c.bones as usize != sk.bones.len()).map(|_| i)).collect();
                assert!(bad.is_empty(), "{target}: clips {bad:?} have another bone count than the model's {}", sk.bones.len());
                let base_mdl = &have[&path(base, "MDL")];
                let (old, new) = (crate::mdl::split_parts(base_mdl).unwrap(), crate::mdl::split_parts(mdl).unwrap());
                assert_ne!(old[3], new[3], "{target}: the body's hair changed");
                let p3 = crate::mdl::MdlPart::parse_bytes(&new[3]).unwrap();
                assert_eq!(p3.list_b.as_ref().map(|l| l.len()), crate::mdl::MdlPart::parse_bytes(&old[3]).unwrap().list_b.as_ref().map(|l| l.len()), "{target}: the coat's list B kept");
                eprintln!("group {target}: {} bones, {} clips", sk.bones.len(), b.clips);
                if !heart {
                    let _ = std::fs::write(out.join(format!("PL{target:04}.MDL")), mdl);
                    let _ = std::fs::write(out.join(format!("PL{target:04}.ANN")), &b.files.iter().find(|f| f.0 == path(target, "ANN")).unwrap().1);
                    if let Some(r) = b.files.iter().find(|f| f.0 == format!("raw/{}", path(target, "RTM"))) { let _ = std::fs::write(out.join(format!("PL{target:04}.RTM.raw")), &r.1); }
                }
            }
        }
    }

    /// 0.198.30 (owner's install, `RECOM_GAME`): the battle's Dark Riku with BBS kid Riku's hair (205 / 206), built as the step builds
    /// them: groups free in Recom.hed; Dark Riku's 128 bones (his sleight and event packs are 128); his pack's clips in the same slots
    /// under the same names (his action rows bind by name), every channel but the hair bones' unchanged; the body's hair changed, the
    /// coat's list B kept; the `.MDL` the Dark Form's BBS group's (185 / 189, built from the same model with its root renamed) with his
    /// root name; his own EET / SES; the `.RTM` byte for byte the Dark Form's BBS group's (the
    /// same texture names, looked up globally, may be loaded together). The models go to `out/dark_mode/` for the renders.
    #[test]
    fn the_battle_dark_riku_gets_the_bbs_hair() {
        let Some(g) = game() else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let bbs = crate::bbs_hair::BbsHairSource::read(&g, &keys).unwrap();
        // 0.198.32: his pack as the step writes it (KH1's guard sets, BBS Terra's casts: the guard `p003_225` / `p003_226` included)
        let src = StepSources::read(&g, &keys).expect("the step's KH1 sources");
        let have = step_have(&arc, &keys, &src);
        assert!(Ann::parse(&have[&path(201, "ANN")]).unwrap().find(DARK_GUARD).is_some(), "the step's pack");
        let out = std::path::PathBuf::from(format!("{}/local/assets/recom/riku-hair/out/dark_mode", crate::kh2_motion::tests::workspace()));
        let _ = std::fs::create_dir_all(&out);
        for (&(target, dark, atlas), form) in DARK_MODE_BBS_HAIR_GROUPS.iter().zip([185u16, 189]) {
            assert!(arc.find(&path(target, "MDL")).is_none() && arc.find(&path(target, "ANN")).is_none(), "group {target} is free in Recom.hed");
            let b = build_dark_mode_bbs(&arc, &keys, &have, dark, target, &bbs, &atlas, None).unwrap();
            let file = |n: String| b.files.iter().find(|f| f.0 == n).unwrap_or_else(|| panic!("{n}")).1.clone();
            let (mdl, ann) = (file(path(target, "MDL")), Ann::parse(&file(path(target, "ANN"))).unwrap());
            let sk = RecomSkeleton::parse(&mdl).unwrap();
            let base_mdl = install::read_recom(&arc, &keys, &path(dark, "MDL")).unwrap();
            let base_sk = RecomSkeleton::parse(&base_mdl).unwrap();
            assert_eq!(sk.bones.len(), 128, "{target}: Dark Riku's bone count");
            assert_eq!(sk.bones.iter().map(|x| &x.name).collect::<Vec<_>>(), base_sk.bones.iter().map(|x| &x.name).collect::<Vec<_>>(), "{target}: his bones");
            let base_ann = Ann::parse(&have[&path(dark, "ANN")]).unwrap();
            assert_eq!(ann.slots.len(), base_ann.slots.len(), "{target}: slots");
            let mut hair_moved = 0;
            for (s0, s1) in base_ann.slots.iter().zip(&ann.slots) {
                let (Some(c0), Some(c1)) = (s0.clip.as_ref(), s1.clip.as_ref()) else { assert!(s0.clip.is_none() && s1.clip.is_none(), "{target}: a slot's clip"); continue };
                assert_eq!((c0.name_str(), c0.frames, c0.loop_start, c0.flags, c1.bones), (c1.name_str(), c1.frames, c1.loop_start, c1.flags, 128), "{target}");
                let ch = c0.chans();
                for bi in 0..128 {
                    let same = c0.channels[bi * ch..(bi + 1) * ch] == c1.channels[bi * ch..(bi + 1) * ch];
                    if base_sk.bones[bi].name.contains("hair") {
                        hair_moved += !same as usize;
                    } else {
                        assert!(same, "{target} {}: bone {bi} {} changed", c0.name_str(), base_sk.bones[bi].name);
                    }
                }
            }
            let (old, new) = (crate::mdl::split_parts(&base_mdl).unwrap(), crate::mdl::split_parts(&mdl).unwrap());
            assert_ne!(old[3], new[3], "{target}: the body's hair changed");
            let lb = |p: &[u8]| crate::mdl::MdlPart::parse_bytes(p).unwrap().list_b.as_ref().map(|l| l.len());
            assert_eq!(lb(&new[3]), lb(&old[3]), "{target}: the coat's list B kept");
            for ext in ["EET", "SES"] {
                assert!(file(path(target, ext)) == have[&path(dark, ext)], "{target}: his own {ext}");
            }
            let skin_base = if dark == 201 { 121 } else { 131 };
            let dform = build_hair_from(&arc, &keys, &have, skin_base, dark, true, form, None, &bbs, &atlas).unwrap();
            let rtm = |b: &Built, gr: u16| b.files.iter().find(|f| f.0 == format!("raw/{}", path(gr, "RTM"))).unwrap().1.clone();
            assert!(rtm(&b, target) == rtm(&dform, form), "{target}: the .RTM is the Dark Form group {form}'s");
            let form_mdl = &dform.files.iter().find(|f| f.0 == path(form, "MDL")).unwrap().1;
            assert!(&rename_root(&mdl).unwrap() == form_mdl, "{target}: the .MDL is the Dark Form group {form}'s with Dark Riku's root name");
            assert_eq!(sk.bones[1].name, base_sk.bones[1].name, "{target}: his root name");
            eprintln!("group {target}: {} bones, {} clips, hair channels changed in {hair_moved} bone-clips", sk.bones.len(), b.clips);
            let _ = std::fs::write(out.join(format!("PL{target:04}.MDL")), &mdl);
            let _ = std::fs::write(out.join(format!("PL{target:04}.ANN")), file(path(target, "ANN")));
            let _ = std::fs::write(out.join(format!("PL{target:04}.RTM.raw")), rtm(&b, target));
        }
        let made = [Made { riku: 101, skin: 121, dark: true, blade: 0, hair: 0 }, Made { riku: 201, skin: 205, dark: true, blade: 0, hair: 2 }];
        let j = manifest_json(&made, true, &[], &[]);
        assert!(j.contains("\"skins\": [{\"riku\": 101, \"skin\": 121") && j.contains("\"dark_mode\": [{\"dark\": 201, \"skin\": 205, \"hair\": 2, \"blade\": 0}]"), "{j}");
    }

    /// The asset step's Riku sources (KH1 Riku's guard set, Riku-Ansem's, BBS Terra's), read as `step.rs` reads them.
    pub(crate) struct StepSources {
        mset: Vec<u8>,
        mdls: Vec<u8>,
        ansem: Option<(Vec<u8>, Vec<u8>)>,
        terra: Option<(RecomSkeleton, Vec<crate::pam::PamClip>)>,
    }
    impl StepSources {
        pub(crate) fn read(g: &std::path::Path, keys: &kh1_formats::egs::KeyTable) -> Option<StepSources> {
            let k1 = install::kh1(g, keys.clone()).ok()?;
            let ansem = k1.read("xa_ex_1580.mset").ok().zip(k1.read("xa_ex_1580.mdls").ok());
            Some(StepSources { mset: k1.read("xa_ex_1010.mset").ok()?, mdls: k1.read("xa_ex_1010.mdls").ok()?, ansem, terra: crate::terra::read_terra(g, keys).ok() })
        }
        pub(crate) fn guard(&self) -> GuardSource<'_> {
            GuardSource { mset: &self.mset, mdls: &self.mdls, ansem: self.ansem.as_ref().map(|(a, b)| (a.as_slice(), b.as_slice())), terra: self.terra.as_ref().map(|(k, c)| (k, c.as_slice())) }
        }
    }

    /// The files the step writes for Riku's skins (`have`), from its real sources.
    fn step_have(arc: &kh1_formats::egs::Archive, keys: &kh1_formats::egs::KeyTable, src: &StepSources) -> std::collections::HashMap<String, Vec<u8>> {
        let mut have = std::collections::HashMap::new();
        for &(riku, dark, skin) in &SKIN_GROUPS {
            for (rel, bytes) in build_with(arc, keys, riku, dark, skin, Some(&src.guard())).unwrap().files {
                have.insert(rel, bytes);
            }
        }
        have
    }

    /// 0.198.32: Dark Riku's slots play the rows Riku's [`DARK_SLOT_OF`] slots play (`p00x_n` in slot `n` of both packs; the rows are
    /// the exe's, Checked by hand there), and [`dark_slots`] gives his idle / walk / run / swing slots KH2's idle / walk / run / swing.
    #[test]
    fn dark_riku_slots_follow_rikus_rows() {
        use crate::kh2_hair_rig as r;
        let s = dark_slots();
        let of = |slot: usize| s.map.iter().find(|x| x.0 == slot).map(|x| x.1);
        assert_eq!((of(1), of(21), of(2), of(3), of(34), of(96)), (Some(r::KH2_IDLE), Some(r::KH2_IDLE), Some(r::KH2_WALK), Some(r::KH2_RUN), Some(r::KH2_SWING), Some(r::KH2_JUMP)));
        assert_eq!(of(33), None, "Dark Riku's 33 is a swing, not Riku's run");
        assert_eq!(s.unroll, [1, 21]);
        let Some(g) = game() else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        for (riku, dark) in [(101u16, 201u16), (111, 211)] {
            let (a, b) = (Ann::parse(&install::read_recom(&arc, &keys, &path(riku, "ANN")).unwrap()).unwrap(), Ann::parse(&install::read_recom(&arc, &keys, &path(dark, "ANN")).unwrap()).unwrap());
            for &(rs, ds) in &DARK_SLOT_OF {
                let name = |x: &Ann, i: usize| x.slots.get(i).and_then(|s| s.clip.as_ref()).map(|c| c.name_str().to_string());
                assert_eq!(name(&a, rs), Some(format!("p002_{rs:03}")));
                assert_eq!(name(&b, ds), Some(format!("p003_{ds:03}")), "group {dark} slot {ds}");
                let (ca, cb) = (a.slots[rs].clip.as_ref().unwrap(), b.slots[ds].clip.as_ref().unwrap());
                if [1, 31, 2, 28, 32, 3, 21, 33, 26, 6].contains(&rs) {
                    assert!(ca.looped() && cb.looped(), "{rs} -> {ds}: both loop");
                }
            }
        }
    }

    /// 0.198.32 (`RECOM_GAME`, the step's real sources: KH1's guard sets and BBS Terra's casts): the battle's Dark Riku with KH2 Riku's
    /// hair (207 / 208) and the 139-bone copies of his 11 other packs. Groups and copies free in Recom.hed; the model 139 bones, his 128
    /// first with his names; his pack (the guard and cast clips included) in the same slots, every clip 139 bones, every non-hair
    /// channel byte-equal; the `.RTM` the Dark Form KH2 group 184 / 188's (texture names are looked up globally); every copy all
    /// clips 139 bones, slots / frames / loop / flags as Re:CoM's, non-hair channels byte-equal; the hair moves in each copy.
    #[test]
    fn the_battle_dark_riku_gets_the_kh2_hair() {
        let Some(g) = game() else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let src = StepSources::read(&g, &keys).expect("the step's KH1 sources");
        assert!(src.terra.is_some(), "BBS Terra's casts, as the step reads them");
        let have = step_have(&arc, &keys, &src);
        let hair = crate::kh2_hair::Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap();
        let out = std::path::PathBuf::from(format!("{}/local/assets/recom/riku-hair/out/dark_mode", crate::kh2_motion::tests::workspace()));
        let _ = std::fs::create_dir_all(&out);
        let mut bones = 0;
        for (&(target, dark, atlas), form) in DARK_MODE_KH2_HAIR_GROUPS.iter().zip([184u16, 188]) {
            for ext in ["MDL", "ANN", "EET", "SES", "RTM", "TXA"] {
                assert!(arc.find(&path(target, ext)).is_none(), "group {target} is free in Recom.hed");
            }
            let b = build_dark_mode_kh2(&arc, &keys, &have, dark, target, &hair, &atlas, None).unwrap();
            let file = |n: String| b.files.iter().find(|f| f.0 == n).unwrap_or_else(|| panic!("{n}")).1.clone();
            let (mdl, ann) = (file(path(target, "MDL")), Ann::parse(&file(path(target, "ANN"))).unwrap());
            let sk = RecomSkeleton::parse(&mdl).unwrap();
            let base_mdl = install::read_recom(&arc, &keys, &path(dark, "MDL")).unwrap();
            let base_sk = RecomSkeleton::parse(&base_mdl).unwrap();
            bones = sk.bones.len();
            assert_eq!(bones, 139, "{target}: KH2's hair on Dark Riku");
            assert_eq!(sk.bones[..128].iter().map(|x| &x.name).collect::<Vec<_>>(), base_sk.bones.iter().map(|x| &x.name).collect::<Vec<_>>(), "{target}: his bones first");
            let base_ann = Ann::parse(&have[&path(dark, "ANN")]).unwrap();
            assert!(base_ann.find(DARK_GUARD).is_some() && base_ann.find(DARK_RECOIL).is_some() && CAST_CLIPS.iter().all(|c| base_ann.find(&format!("p003_{}", c.suffix)).is_some()), "{dark}: the step's pack (guard, casts)");
            assert_eq!(ann.slots.len(), base_ann.slots.len(), "{target}: slots");
            let mut hair_moved = 0;
            for (s0, s1) in base_ann.slots.iter().zip(&ann.slots) {
                let (Some(c0), Some(c1)) = (s0.clip.as_ref(), s1.clip.as_ref()) else { assert!(s0.clip.is_none() && s1.clip.is_none(), "{target}: a slot's clip"); continue };
                // his idle loops (slots 1 / 21) are unrolled to a whole number of KH2 idle loops, as Riku's (`kh2_hair_rig::UNROLL`)
                let slot = base_ann.slots.iter().position(|x| std::ptr::eq(x, s0)).unwrap();
                let times = c1.frames as usize / c0.frames.max(1) as usize;
                assert!(c1.frames == c0.frames || (dark_slots().unroll.contains(&slot) && c1.frames as usize == c0.frames as usize * times), "{target} slot {slot}: {} -> {} frames", c0.frames, c1.frames);
                assert_eq!((c0.name_str(), c0.loop_start, c0.flags, c1.bones), (c1.name_str(), c1.loop_start, c1.flags, 139), "{target}");
                let (ch0, ch1) = (c0.chans(), c1.chans());
                for bi in 0..128 {
                    let same = if c1.frames == c0.frames {
                        c0.channels[bi * ch0..(bi + 1) * ch0] == c1.channels[bi * ch1..(bi + 1) * ch1]
                    } else {
                        (0..c1.frames as usize).all(|f| (0..ch0).all(|ci| (c0.value(bi, ci, (f % c0.frames as usize) as f64) - c1.value(bi, ci, f as f64)).abs() < 1e-3))
                    };
                    if base_sk.bones[bi].name.contains("hair") {
                        hair_moved += !same as usize;
                    } else {
                        assert!(same, "{target} {}: bone {bi} {} changed", c0.name_str(), base_sk.bones[bi].name);
                    }
                }
            }
            assert!(hair_moved > 0, "{target}: the hair is KH2's");
            for ext in ["EET", "SES"] {
                assert!(file(path(target, ext)) == have[&path(dark, ext)], "{target}: his own {ext}");
            }
            let skin_base = if dark == 201 { 121 } else { 131 };
            let dform = build_hair_from(&arc, &keys, &have, skin_base, dark, true, form, None, &hair, &atlas).unwrap();
            let rtm = |b: &Built, gr: u16| b.files.iter().find(|f| f.0 == format!("raw/{}", path(gr, "RTM"))).unwrap().1.clone();
            assert!(rtm(&b, target) == rtm(&dform, form), "{target}: the .RTM is the Dark Form KH2 group {form}'s");
            eprintln!("group {target}: {} bones, {} clips, hair channels changed in {hair_moved} bone-clips; .MDL {} B, .ANN {} B", sk.bones.len(), b.clips, mdl.len(), file(path(target, "ANN")).len());
            let _ = std::fs::write(out.join(format!("PL{target:04}.MDL")), &mdl);
            let _ = std::fs::write(out.join(format!("PL{target:04}.ANN")), file(path(target, "ANN")));
            let _ = std::fs::write(out.join(format!("PL{target:04}.RTM.raw")), rtm(&b, target));
        }
        // the copies (on 201's model; 211's skeleton gives the same bytes)
        let packs = build_dark_packs(&arc, &keys, 201, &hair).unwrap();
        assert_eq!(packs.iter().map(|p| (p.0, p.1)).collect::<Vec<_>>(), DARK_PACKS.map(|g| (g, g + DARK_ALT)), "all eleven");
        // a fresh source: the rig is kept per skeleton (by bone names), so 211's own rest decides here
        let packs_211 = build_dark_packs(&arc, &keys, 211, &crate::kh2_hair::Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap()).unwrap();
        let sk = RecomSkeleton::parse(&install::read_recom(&arc, &keys, &path(201, "MDL")).unwrap()).unwrap();
        let mut total = 0;
        for ((src_g, alt, bytes), (_, _, b211)) in packs.iter().zip(&packs_211) {
            assert!(arc.find(&path(*alt, "ANN")).is_none(), "{alt} is free in Recom.hed");
            assert!(bytes == b211, "{alt}: the same copy on 211's model");
            let (a0, a1) = (Ann::parse(&install::read_recom(&arc, &keys, &path(*src_g, "ANN")).unwrap()).unwrap(), Ann::parse(bytes).unwrap());
            assert_eq!(a0.slots.len(), a1.slots.len(), "{alt}");
            let mut moved = 0;
            for (s0, s1) in a0.slots.iter().zip(&a1.slots) {
                let (Some(c0), Some(c1)) = (s0.clip.as_ref(), s1.clip.as_ref()) else { assert!(s0.clip.is_none() && s1.clip.is_none()); continue };
                assert_eq!(c1.bones as usize, bones, "{alt}: {bones} bones");
                assert_eq!((c0.name_str(), c0.frames, c0.loop_start, c0.flags), (c1.name_str(), c1.frames, c1.loop_start, c1.flags), "{alt}");
                let (ch0, ch1) = (c0.chans(), c1.chans());
                for bi in 0..128 {
                    let same = c0.channels[bi * ch0..(bi + 1) * ch0] == c1.channels[bi * ch1..(bi + 1) * ch1];
                    if sk.bones[bi].name.contains("hair") {
                        moved += !same as usize;
                    } else {
                        assert!(same, "{alt} {}: bone {bi} {} changed", c0.name_str(), sk.bones[bi].name);
                    }
                }
            }
            assert!(moved > 0, "{alt}: the hair is KH2's");
            total += bytes.len();
            let _ = std::fs::write(out.join(format!("PL{alt:04}.ANN")), bytes);
        }
        eprintln!("Dark Riku's pack copies: {} ({bones} bones), {total} B", packs.len());
        let made = [Made { riku: 201, skin: 205, dark: true, blade: 0, hair: 2 }, Made { riku: 201, skin: 207, dark: true, blade: 0, hair: 1 }];
        let j = manifest_json(&made, true, &[], &[(1087, 1187), (4083, 4183)]);
        assert!(j.contains("\"dark_mode\": [{\"dark\": 201, \"skin\": 205, \"hair\": 2, \"blade\": 0}, {\"dark\": 201, \"skin\": 207, \"hair\": 1, \"blade\": 0}]") && j.contains("\"dark_packs\": [[1087, 1187], [4083, 4183]]"), "{j}");
    }

    /// 0.198.36 (fork owner, 2026-11-27: "no darkmode a keyblade tbm nn troca, já coloca essas regrs para o futuro"; `RECOM_GAME`, the
    /// step's real sources): every Keyblade row of [`DARK_MODE_LOOKS`] built by [`build_dark_mode`] as the step builds it. Each group free
    /// in Recom.hed; its Soul Eater twin (same Dark Riku, same hair: his own group for Re:CoM's hair) differs only in the weapon: the
    /// body parts byte for byte, the pack / EET / SES byte for byte (the motion, the hit and trail events stay the Soul Eater's), the
    /// TXA the same; the weapon's vertices (bone, position) those of the Dark Form costume's Keyblade group (161 / 171) on the same
    /// `wep0`, its texture [`BLADE_TEXTURE`] in the RTM; the KH2 rows 139 bones (the pack copies' count). The models go to
    /// `out/dark_mode/` for the renders; sizes printed.
    #[test]
    fn the_battle_dark_riku_keeps_the_keyblade() {
        use crate::mdl::{split_parts, MdlPart};
        let Some(g) = game() else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let src = StepSources::read(&g, &keys).expect("the step's KH1 sources");
        let have = step_have(&arc, &keys, &src);
        let kh2 = crate::kh2_hair::Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap();
        let bbs = crate::bbs_hair::BbsHairSource::read(&g, &keys).unwrap();
        let Ok((_, assets, _)) = install::kh1(&g, keys.clone()).unwrap().read_full(BLADE_MODEL) else { panic!("KH1's {BLADE_MODEL}") };
        let heart = BladeSource::from_assets(&assets).unwrap();
        let out = std::path::PathBuf::from(format!("{}/local/assets/recom/riku-hair/out/dark_mode", crate::kh2_motion::tests::workspace()));
        let _ = std::fs::create_dir_all(&out);
        let file = |b: &Built, n: String| b.files.iter().find(|f| f.0 == n).unwrap_or_else(|| panic!("{n}")).1.clone();
        let weapon = |mdl: &[u8]| -> (Vec<(usize, [i32; 3])>, Vec<String>) {
            let p1 = MdlPart::parse_bytes(&split_parts(mdl).unwrap()[1]).unwrap();
            let v = p1.list_a.iter().chain(p1.list_b.iter().flatten()).flat_map(|k| k.verts.iter()).map(|v| (v.bone(), v.pos().map(|x| (x * 1e4).round() as i32))).collect();
            (v, p1.texture_names())
        };
        // the Dark Form costume's Keyblade groups, as the step builds them (the reference for the grip)
        let form: Vec<(u16, Built)> = [(121u16, 201u16, 161u16), (131, 211, 171)].iter().map(|&(base, raw, t)| (raw, build_blade(&arc, &keys, &have, base, raw, t, true, &heart).unwrap())).collect();
        let mut rows = 0;
        for look in DARK_MODE_LOOKS.into_iter().filter(|l| l.3 == 1) {
            let (target, dark, hair, _) = look;
            for ext in ["MDL", "ANN", "EET", "SES", "RTM", "TXA"] {
                assert!(arc.find(&path(target, ext)).is_none(), "group {target} is free in Recom.hed");
            }
            let b = build_dark_mode(&arc, &keys, &have, look, Some(&kh2), Some(&bbs), Some(&heart)).unwrap();
            let twin_row = DARK_MODE_LOOKS.iter().find(|l| l.1 == dark && l.2 == hair && l.3 == 0).copied();
            let twin = twin_row.map(|r| build_dark_mode(&arc, &keys, &have, r, Some(&kh2), Some(&bbs), None).unwrap());
            let twin_file = |ext: &str| match &twin {
                Some(t) => file(t, path(twin_row.unwrap().0, ext)),
                None => have.get(&path(dark, ext)).cloned().unwrap_or_else(|| install::read_recom(&arc, &keys, &path(dark, ext)).unwrap()),
            };
            let mdl = file(&b, path(target, "MDL"));
            let (new, old) = (split_parts(&mdl).unwrap(), split_parts(&twin_file("MDL")).unwrap());
            assert_eq!(new.len(), old.len(), "{target}");
            assert!(new[2..] == old[2..], "{target}: the body byte for byte its Soul Eater twin's");
            for ext in ["ANN", "EET", "SES"] {
                assert!(file(&b, path(target, ext)) == twin_file(ext), "{target}: {ext} the Soul Eater twin's");
            }
            let (fraw, fb) = form.iter().find(|f| f.0 == dark).unwrap();
            let fgrp = if dark == 201 { 161 } else { 171 };
            let raw = |bb: &Built, gr: u16, ext: &str| file(bb, format!("raw/{}", path(gr, ext)));
            assert!(raw(&b, target, "TXA") == raw(fb, fgrp, "TXA"), "{target}: Dark Riku's TXA ({fraw})");
            let (wv, wt) = weapon(&mdl);
            let (fv, _) = weapon(&file(fb, path(fgrp, "MDL")));
            assert_eq!(wt, [BLADE_TEXTURE], "{target}: the Keyblade's texture");
            assert!(wv == fv && !wv.is_empty() && wv.iter().all(|v| v.0 == 105), "{target}: the Keyblade on wep0 (105) where group {fgrp} has it");
            let sk = RecomSkeleton::parse(&mdl).unwrap();
            assert_eq!(sk.bones.len(), if hair == 1 { 139 } else { 128 }, "{target}: bones");
            let rtm = raw(&b, target, "RTM");
            assert!(rtm.windows(BLADE_TEXTURE.len()).any(|w| w == BLADE_TEXTURE.as_bytes()), "{target}: the Keyblade's picture in the RTM");
            let size: usize = b.files.iter().map(|f| f.1.len()).sum();
            eprintln!("group {target} ({}): {} bones, {} clips, .MDL {} B, raw .RTM {} B, all files {size} B", dark_look_name(hair, 1), sk.bones.len(), b.clips, mdl.len(), rtm.len());
            let _ = std::fs::write(out.join(format!("PL{target:04}.MDL")), &mdl);
            let _ = std::fs::write(out.join(format!("PL{target:04}.RTM.raw")), &rtm);
            rows += 1;
        }
        assert_eq!(rows, 6, "209 / 210, 212 / 213, 216 / 217");
        // one row per (Dark Riku, hair, blade); never his own look; no group twice
        let mut seen = std::collections::HashSet::new();
        for &(t, d, h, bl) in &DARK_MODE_LOOKS {
            assert!(DARK_MODE_GROUPS.contains(&d) && (h, bl) != (0, 0) && seen.insert((d, h, bl)) && seen.insert((t, 9, 9)), "row {t}");
        }
        // the Soul Eater rows are 0.198.30 / 0.198.32's groups
        for &(t, d, _) in DARK_MODE_BBS_HAIR_GROUPS.iter().chain(&DARK_MODE_KH2_HAIR_GROUPS) {
            assert!(DARK_MODE_LOOKS.iter().any(|l| l.0 == t && l.1 == d && l.3 == 0), "{t}");
        }
    }

    /// 0.198.36 reference renders (`RECOM_GAME`): the Dark Form costume's Keyblade group 161 (`REF_GROUP=171`) as the step builds it, the
    /// right hand on the Keyblade in the views of `battle_dark_camera_checks`' `grip_*` (idle, the first swing, the guard: Riku's rows on
    /// his pack, slots 1 / 44 and `p002_225`), into `out/dark_mode/ref<group>_<CAM_TAG>/`.
    #[test]
    fn dark_form_keyblade_grip_reference() {
        use crate::kh2_hair::tests::{posed_world, render_focus};
        use crate::mdl::{split_parts, MdlPart};
        let Some(g) = game() else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let grp: u16 = std::env::var("REF_GROUP").ok().and_then(|x| x.parse().ok()).unwrap_or(161);
        let &(base, raw_src, _, _) = BLADE_GROUPS.iter().find(|x| x.2 == grp).expect("a Dark Form Keyblade group");
        let src = StepSources::read(&g, &keys).expect("the step's KH1 sources");
        let have = step_have(&arc, &keys, &src);
        let Ok((_, assets, _)) = install::kh1(&g, keys.clone()).unwrap().read_full(BLADE_MODEL) else { panic!() };
        let heart = BladeSource::from_assets(&assets).unwrap();
        let b = build_blade(&arc, &keys, &have, base, raw_src, grp, true, &heart).unwrap();
        let file = |n: String| b.files.iter().find(|f| f.0 == n).unwrap().1.clone();
        let mdl = file(path(grp, "MDL"));
        let ann = Ann::parse(&file(path(grp, "ANN"))).unwrap();
        let e = arc.find(&path(raw_src, "RTM")).unwrap();
        let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
        let rtm_p = crate::rtm::Rtm::parse(&rtm).unwrap();
        let parts = split_parts(&mdl).unwrap();
        let mut p3 = MdlPart::parse_bytes(&parts[3]).unwrap();
        let dec = |n: &str| { let e = rtm_p.entries.iter().find(|e| e.name_str().trim_end_matches(".tm2") == n.trim_end_matches(".tm2")).unwrap(); let (w, h, px) = crate::rtm::tim2_decode8(&e.data).unwrap(); crate::texture::Image { w, h, px } };
        let mut body: Vec<crate::texture::Image> = p3.texture_names().iter().map(|n| dec(n)).collect();
        let atlas = if raw_src == 201 { crate::ddd_hair::RIKU_0201_BBS.texture } else { crate::ddd_hair::RIKU_0211_BBS.texture };
        let a = p3.texture_names().iter().position(|n| n == atlas).unwrap() as u32;
        for k in p3.list_a.iter_mut().chain(p3.list_b.iter_mut().flatten()) { for v in k.verts.iter_mut() { if let Some(q) = v.q2.as_mut() { if q[3] == a { q[3] = 1; } else if q[3] == 1 { q[3] = a; } } } }
        body.swap(1, a as usize);
        let img = hd.iter().find(|x| x.name == crate::keyblade::hd_name(atlas)).and_then(|x| crate::texture::decode_dds(&x.data).ok()).unwrap_or_else(|| body[1].clone());
        let wi = body.len() as u32;
        body.push(crate::texture::decode_dds(&heart.dds).unwrap());
        let p1 = MdlPart::parse_bytes(&parts[1]).unwrap();
        for mut k in p1.list_a.iter().chain(p1.list_b.iter().flatten()).cloned() {
            for v in k.verts.iter_mut() { if let Some(q) = v.q2.as_mut() { q[3] = wi; } }
            p3.list_a.push(k);
        }
        let sk = RecomSkeleton::parse(&mdl).unwrap();
        let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
        let hand = names.iter().position(|n| n == "r_hand").unwrap();
        let tag = std::env::var("CAM_TAG").unwrap_or_else(|_| "cam".into());
        let dir = std::path::PathBuf::from(format!("{}/local/assets/recom/riku-hair/out/dark_mode/ref{grp}_{tag}", crate::kh2_motion::tests::workspace()));
        std::fs::create_dir_all(&dir).unwrap();
        for (label, clip, f) in [("idle", "p002_001", 25.0f64), ("swing8", "p002_044", 8.0), ("swing16", "p002_044", 16.0), ("guard", RIKU_GUARD, 6.0)] {
            let Some(c) = ann.slots.iter().find_map(|s| s.clip.as_ref().filter(|c| c.name_str() == clip)) else { eprintln!("{clip}: not in the pack"); continue };
            let w = posed_world(&sk, c, f.min(c.frames as f64 - 1.0));
            for (yn, yaw, pitch) in [("front", 0.0, 0.0), ("side", 90.0, 0.0), ("back", 180.0, 0.0), ("other", 270.0, 0.0), ("top", 0.0, 60.0), ("under", 0.0, -60.0)] {
                render_focus(&p3, &w, &names, &img, &body, yaw, pitch, (w[hand].t, 0.09), &dir.join(format!("grip_{label}_{yn}.png")));
            }
        }
    }

    /// 0.198.30 renders (`RECOM_GAME`): the battle's Dark Riku as the game draws group 205 (`DM_GROUP=206` for 211's): the built `.MDL`
    /// (asserted byte for byte the swap's) and its HD hair picture, playing the built `.ANN`; `recom` = Re:CoM's own Dark Riku (201, his
    /// hair) in his own pack. Into `out/dark_mode/<bbs|recom>_<CAM_TAG>/`: the camera checks (idle orbit, walk, run, nape and face
    /// close-ups; slots 1 / 2 / 3 are his idle / walk / run) and his first ground swing `p003_034` and finisher `p003_036` from four
    /// sides with nape and ear close-ups (`swing_*`).
    #[test]
    fn battle_dark_camera_checks() {
        use crate::kh2_hair::tests::{camera_checks, posed_world, render_cam, render_focus, view_at};
        use crate::mdl::MdlPart;
        let Some(g) = game() else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        // 0.198.32: `DM_GROUP=207` / `208` (KH2's hair, `kh2_<CAM_TAG>`, with the copies' sleight and event clips); the step's real
        // sources (his pack with the guard and casts)
        // 0.198.36: any row of `DARK_MODE_LOOKS` (209 / 210 Re:CoM's hair with the Keyblade: the `recom_heart` folder, `DARK_HAIR=recom`),
        // the weapon (part 1, its own picture) drawn with the body
        let target: u16 = std::env::var("DM_GROUP").ok().and_then(|x| x.parse().ok()).unwrap_or(205);
        let look = DARK_MODE_LOOKS.iter().copied().find(|x| x.0 == target).expect("a row of DARK_MODE_LOOKS");
        let kh2 = look.2 == 1;
        let heart_on = look.3 == 1;
        let dark = look.1;
        let &(_, _, atlas) = if kh2 { DARK_MODE_KH2_HAIR_GROUPS.iter().find(|x| x.1 == dark).unwrap() } else { DARK_MODE_BBS_HAIR_GROUPS.iter().find(|x| x.1 == dark).unwrap() };
        let src = StepSources::read(&g, &keys).expect("the step's KH1 sources");
        let have = step_have(&arc, &keys, &src);
        let bbs = crate::bbs_hair::BbsHairSource::read(&g, &keys).unwrap();
        let kh2_src = crate::kh2_hair::Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap();
        let hair: &dyn crate::ddd_hair::HairSource = if kh2 { &kh2_src } else { &bbs };
        // 0.198.33: `DM_BEFORE` set: 0.198.32's BBS group (no neck skirt, Riku's slot map) for the before / after renders
        let before = std::env::var("DM_BEFORE").is_ok();
        let atlas = if before { crate::ddd_hair::HairTarget { neck_skirt: false, ..atlas } } else { atlas };
        let heart = install::kh1(&g, keys.clone()).ok().and_then(|i| i.read_full(BLADE_MODEL).ok()).map(|(_, a, _)| BladeSource::from_assets(&a).unwrap());
        let built = if heart_on { build_dark_mode(&arc, &keys, &have, look, Some(&kh2_src), Some(&bbs), heart.as_ref()).unwrap() } else if kh2 { build_dark_mode_kh2(&arc, &keys, &have, dark, target, &kh2_src, &atlas, None).unwrap() } else if before { build_hair_from(&arc, &keys, &have, dark, dark, true, target, None, &bbs, &atlas).unwrap() } else { build_dark_mode_bbs(&arc, &keys, &have, dark, target, &bbs, &atlas, None).unwrap() };
        let file = |n: String| built.files.iter().find(|f| f.0 == n).unwrap().1.clone();
        let mdl0 = install::read_recom(&arc, &keys, &path(dark, "MDL")).unwrap();
        let e = arc.find(&path(dark, "RTM")).unwrap();
        let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
        let swap = crate::ddd_hair::with_hair(&hair.prepare_mdl(&mdl0).unwrap(), &rtm, &hd, &atlas, hair).unwrap();
        let built_mdl = file(path(target, "MDL"));
        let body_of = |m: &[u8]| crate::mdl::split_parts(m).unwrap()[2..].to_vec();
        if look.2 != 0 {
            assert!(if heart_on { body_of(&swap.mdl) == body_of(&built_mdl) } else { swap.mdl == built_mdl }, "the rendered model is the built one");
        } else {
            assert!(body_of(&mdl0) == body_of(&built_mdl), "his own body");
        }
        // the weapon the game draws: the built model's part 1 (the Keyblade: its HD picture; the Soul Eater: Re:CoM's)
        let weapon_mdl = if look.2 == 0 && !heart_on { mdl0.clone() } else { built_mdl.clone() };
        let rtm_all = crate::rtm::Rtm::parse(&rtm).unwrap();
        let add_weapon = |p3: &mut MdlPart, body: &mut Vec<crate::texture::Image>| {
            let p1 = MdlPart::parse_bytes(&crate::mdl::split_parts(&weapon_mdl).unwrap()[1]).unwrap();
            let name = p1.texture_names().first().cloned().unwrap_or_default();
            let img = if name == BLADE_TEXTURE {
                crate::texture::decode_dds(&heart.as_ref().unwrap().dds).unwrap()
            } else {
                match hd.iter().find(|x| x.name == crate::keyblade::hd_name(&name)).and_then(|x| crate::texture::decode_dds(&x.data).ok()) {
                    Some(i) => i,
                    None => {
                        let e = rtm_all.entries.iter().find(|e| e.name_str().trim_end_matches(".tm2") == name.trim_end_matches(".tm2")).unwrap();
                        let (w, h, px) = crate::rtm::tim2_decode8(&e.data).unwrap();
                        crate::texture::Image { w, h, px }
                    }
                }
            };
            let wi = body.len() as u32;
            body.push(img);
            for mut k in p1.list_a.iter().chain(p1.list_b.iter().flatten()).cloned() {
                for v in k.verts.iter_mut() {
                    if let Some(q) = v.q2.as_mut() {
                        q[3] = wi;
                    }
                }
                p3.list_a.push(k);
            }
        };
        // the other packs' clips he plays: a sleight (Dark Firaga, PL1088) and an event motion (PL4043): the copies for KH2's hair,
        // Re:CoM's for his own hair
        let copies = if kh2 { build_dark_packs(&arc, &keys, dark, &kh2_src).unwrap() } else { vec![] };
        let other = |grp: u16, own: bool| -> Ann {
            match copies.iter().find(|c| c.0 == grp).filter(|_| !own) {
                Some(c) => Ann::parse(&c.2).unwrap(),
                None => Ann::parse(&install::read_recom(&arc, &keys, &path(grp, "ANN")).unwrap()).unwrap(),
            }
        };
        let ann = Ann::parse(&file(path(target, "ANN"))).unwrap();
        let own = Ann::parse(&have[&path(dark, "ANN")]).unwrap();
        for (i, s) in ann.slots.iter().enumerate().take(8) {
            eprintln!("slot {i}: {:?}", s.clip.as_ref().map(|c| (c.name_str().to_string(), c.frames)));
        }
        let tag = std::env::var("CAM_TAG").unwrap_or_else(|_| "cam".into());
        let base = std::path::PathBuf::from(format!("{}/local/assets/recom/riku-hair/out/dark_mode", crate::kh2_motion::tests::workspace()));
        let swings = |p3: &MdlPart, sk: &RecomSkeleton, a: &Ann, img: &crate::texture::Image, body: &[crate::texture::Image], dir: &std::path::Path, own: bool| {
            let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
            let head = names.iter().position(|n| n == "head").unwrap();
            let neck = names.iter().position(|n| n == "neck3").unwrap_or(head);
            // 0.198.32: the guard and a cast (his pack as the step writes it), a sleight and an event clip (their packs), and the idle
            // orbit with the camera low and high
            // 0.198.33: `DM_SHOTS=clip:frame:yaw:pitch;…` only these views (the whole figure and a head close-up), as
            // `shot_<clip>_f<frame>_y<yaw>_p<pitch>[_head].png` (with `GAME_CULL=1`: the game's facing cull)
            if let Ok(shots) = std::env::var("DM_SHOTS") {
                std::fs::create_dir_all(dir).unwrap();
                for s in shots.split(';').filter(|s| !s.is_empty()) {
                    let v: Vec<&str> = s.split(':').collect();
                    let (clip, f, yaw, pitch): (&str, f64, f64, f64) = (v[0], v[1].parse().unwrap(), v[2].parse().unwrap(), v[3].parse().unwrap());
                    let Some(c) = a.slots.iter().find_map(|s| s.clip.as_ref().filter(|c| c.name_str() == clip)) else { eprintln!("{clip}: not in the pack"); continue };
                    let w = posed_world(sk, c, f.min(c.frames as f64 - 1.0));
                    let name = format!("shot_{clip}_f{f}_y{yaw}_p{pitch}");
                    render_cam(p3, &w, &names, img, body, yaw, pitch, true, &dir.join(format!("{name}.png")));
                    render_focus(p3, &w, &names, img, body, yaw, pitch, (w[head].point([0.0, 0.02, 0.0]), 0.14), &dir.join(format!("{name}_head.png")));
                }
                return;
            }
            let (sl, ev) = (other(1088, own), other(4043, own));
            let extra = std::env::var("DM_EXTRA").map_or(true, |x| x != "0");
            // 0.198.33: his jump (`p003_004`, the rise `p003_096`), an air swing (`p003_057`) and the air finisher (`p003_040`): where the
            // neck opened under BBS's hair (the design rule: Dark Riku's renders include jump and attacks)
            let clips: Vec<(&str, &Ann)> = if extra { vec![("p003_034", a), ("p003_036", a), ("p003_004", a), ("p003_096", a), ("p003_057", a), ("p003_040", a), (DARK_GUARD, a), ("p003_227", a), ("p003_043", &sl), ("p003_200", &ev)] } else { vec![("p003_034", a), ("p003_036", a)] };
            if extra {
                if let Some(c) = a.slots.get(1).and_then(|s| s.clip.as_ref()) {
                    let w = posed_world(sk, c, 25.0);
                    for (tag, pitch) in [("low", -35.0), ("high", 35.0)] {
                        for k in 0..8 {
                            render_cam(p3, &w, &names, img, body, k as f64 * 45.0, pitch, true, &dir.join(format!("orbit_{tag}_{:03}.png", k * 45)));
                        }
                    }
                }
            }
            // 0.198.36: the right hand on the weapon (idle, the first swing, the guard), from six sides
            if extra {
                let hand = names.iter().position(|n| n == "r_hand").unwrap();
                for (label, clip, f) in [("idle", "p003_001", 25.0f64), ("swing8", "p003_034", 8.0), ("swing16", "p003_034", 16.0), ("guard", DARK_GUARD, 6.0)] {
                    let Some(c) = a.slots.iter().find_map(|s| s.clip.as_ref().filter(|c| c.name_str() == clip)) else { continue };
                    let w = posed_world(sk, c, f.min(c.frames as f64 - 1.0));
                    for (yn, yaw, pitch) in [("front", 0.0, 0.0), ("side", 90.0, 0.0), ("back", 180.0, 0.0), ("other", 270.0, 0.0), ("top", 0.0, 60.0), ("under", 0.0, -60.0)] {
                        render_focus(p3, &w, &names, img, body, yaw, pitch, (w[hand].t, 0.09), &dir.join(format!("grip_{label}_{yn}.png")));
                    }
                }
            }
            for (clip, a) in clips {
                let Some(c) = a.slots.iter().find_map(|s| s.clip.as_ref().filter(|c| c.name_str() == clip)) else { eprintln!("{clip}: not in the pack"); continue };
                assert_eq!(c.bones as usize, sk.bones.len(), "{clip}: the model's bone count");
                let n = c.frames as f64;
                for f in [0.0, (n / 4.0).round(), (n / 2.0).round(), (3.0 * n / 4.0).round(), n - 1.0] {
                    let w = posed_world(sk, c, f);
                    for yaw in [0.0, 90.0, 180.0, 270.0] {
                        render_cam(p3, &w, &names, img, body, yaw, -10.0, true, &dir.join(format!("swing_{clip}_f{f}_y{yaw}.png")));
                    }
                    render_focus(p3, &w, &names, img, body, 180.0, -15.0, (w[neck].t, 0.16), &dir.join(format!("swing_{clip}_nape_f{f}.png")));
                    let ear = w[head].point([0.0, 0.03, 0.03]);
                    for yaw in [90.0, 270.0] {
                        render_focus(p3, &w, &names, img, body, yaw, 0.0, (ear, 0.09), &dir.join(format!("swing_{clip}_ear_f{f}_y{yaw}.png")));
                    }
                }
            }
        };
        let which = std::env::var("DARK_HAIR").unwrap_or_else(|_| "bbs,recom".into());
        let heart_dir = if heart_on { "_heart" } else { "" };
        if look.2 != 0 && which.contains(if kh2 { "kh2" } else { "bbs" }) {
            let (mut p3, img, mut body) = view_at(&swap, atlas.texture);
            add_weapon(&mut p3, &mut body);
            let sk = RecomSkeleton::parse(&swap.mdl).unwrap();
            let dir = base.join(format!("{}{heart_dir}{}_{tag}", if kh2 { "kh2" } else { "bbs" }, if dark == 201 { "" } else { "_0211" }));
            if std::env::var("DM_SHOTS").is_err() { camera_checks(&p3, &sk, &ann, &img, &body, &dir); }
            swings(&p3, &sk, &ann, &img, &body, &dir, false);
        }
        if which.contains("recom") {
            // Re:CoM's Dark Riku as he is, drawn with the same tool (as `kh2_hair::tests::dark_camera_checks`' `recom`)
            let sk = RecomSkeleton::parse(&mdl0).unwrap();
            let mut p3 = MdlPart::parse_bytes(&crate::mdl::split_parts(&mdl0).unwrap()[3]).unwrap();
            let rtm_p = crate::rtm::Rtm::parse(&rtm).unwrap();
            let mut body: Vec<crate::texture::Image> = p3.textures.iter().map(|n| {
                let n = String::from_utf8_lossy(n.split(|&b| b == 0).next().unwrap_or(&[])).to_string();
                let e = rtm_p.entries.iter().find(|e| e.name_str().trim_end_matches(".tm2") == n.trim_end_matches(".tm2")).unwrap();
                let (w, h, px) = crate::rtm::tim2_decode8(&e.data).unwrap();
                crate::texture::Image { w, h, px }
            }).collect();
            let a = p3.texture_names().iter().position(|n| n == atlas.texture).unwrap() as u32;
            for k in p3.list_a.iter_mut().chain(p3.list_b.iter_mut().flatten()) { for v in k.verts.iter_mut() { if let Some(q) = v.q2.as_mut() { if q[3] == a { q[3] = 1; } else if q[3] == 1 { q[3] = a; } } } }
            body.swap(1, a as usize);
            let img = hd.iter().find(|x| x.name == crate::keyblade::hd_name(atlas.texture)).and_then(|x| crate::texture::decode_dds(&x.data).ok()).unwrap_or_else(|| body[1].clone());
            // his own Soul Eater, or (209 / 210) the Keyblade
            add_weapon(&mut p3, &mut body);
            let dir = base.join(format!("recom{}{}_{tag}", if look.2 == 0 { heart_dir } else { "" }, if dark == 201 { "" } else { "_0211" }));
            if std::env::var("DM_SHOTS").is_err() { camera_checks(&p3, &sk, &own, &img, &body, &dir); }
            swings(&p3, &sk, &own, &img, &body, &dir, true);
        }
    }

    /// 0.198.7 (in game: KH2's hair on the Dark Form in long straight sticks): as in the asset step, one KH2 source builds Riku's group
    /// first and the Dark Form's after; the Dark Form's must be byte for byte what a fresh source builds (the source kept the first
    /// skeleton's rig).
    #[test]
    fn one_kh2_source_builds_riku_then_the_dark_form_as_a_fresh_one() {
        let Some(g) = game() else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let mut have = std::collections::HashMap::new();
        for (rel, bytes) in build_with(&arc, &keys, 101, 201, 121, None).unwrap().files {
            have.insert(rel, bytes);
        }
        let read = || crate::kh2_hair::Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap();
        let (shared, fresh) = (read(), read());
        build_hair(&arc, &keys, &have, 101, 182, None, &shared, &crate::ddd_hair::RIKU_0101_KH2).unwrap();
        let a = build_hair_from(&arc, &keys, &have, 121, 201, true, 184, None, &shared, &crate::ddd_hair::RIKU_0201_KH2).unwrap();
        let b = build_hair_from(&arc, &keys, &have, 121, 201, true, 184, None, &fresh, &crate::ddd_hair::RIKU_0201_KH2).unwrap();
        for ((na, da), (nb, db)) in a.files.iter().zip(&b.files) {
            assert_eq!(na, nb);
            assert!(da == db, "{na}: the shared source built it differently");
        }
    }

    #[test]
    fn the_kh2_hair_groups_are_built_from_rikus_101() {
        let Some(g) = game() else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let hair = crate::kh2_hair::Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap();
        let Ok((_, assets, _)) = install::kh1(&g, keys.clone()).unwrap().read_full(BLADE_MODEL) else { return };
        let src = BladeSource::from_assets(&assets).unwrap();
        let have = std::collections::HashMap::new();
        for &(target, base, heart, atlas) in &KH2_HAIR_GROUPS {
            let plain_mdl = install::read_recom(&arc, &keys, &path(base, "MDL")).unwrap();
            let b = build_hair(&arc, &keys, &have, base, target, if heart { Some(&src) } else { None }, &hair, &atlas).unwrap();
            let names: Vec<&str> = b.files.iter().map(|f| f.0.as_str()).collect();
            for ext in ["MDL", "ANN", "EET", "SES"] {
                assert!(names.contains(&path(target, ext).as_str()), "{target}: {ext}");
            }
            for ext in ["RTM", "TXA"] {
                assert!(names.contains(&format!("raw/{}", path(target, ext)).as_str()), "{target}: {ext}");
            }
            let mdl = &b.files.iter().find(|f| f.0 == path(target, "MDL")).unwrap().1;
            let (old, new) = (crate::mdl::split_parts(&plain_mdl).unwrap(), crate::mdl::split_parts(mdl).unwrap());
            assert_ne!(old[3], new[3], "{target}: the body's hair changed");
            let p = crate::mdl::MdlPart::parse_bytes(&new[3]).unwrap();
            assert!(p.texture_names().iter().any(|t| t.as_str() == "rk_hair_kh2"), "{target}: the KH2 atlas");
        }
    }

    /// 0.194.0 (owner's install, `RECOM_GAME`, the 1.5+2.5 collection only): the BBS kid hair groups are built from group 101 with
    /// their own atlas name (both hairs can be in the game).
    #[test]
    fn the_bbs_hair_groups_are_built_from_rikus_101() {
        let Some(g) = game() else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let hair = crate::bbs_hair::BbsHairSource::read(&g, &keys).unwrap();
        let Ok((_, assets, _)) = install::kh1(&g, keys.clone()).unwrap().read_full(BLADE_MODEL) else { return };
        let src = BladeSource::from_assets(&assets).unwrap();
        let have = std::collections::HashMap::new();
        for &(target, base, heart, atlas) in &BBS_HAIR_GROUPS {
            let plain_mdl = install::read_recom(&arc, &keys, &path(base, "MDL")).unwrap();
            let b = build_hair(&arc, &keys, &have, base, target, if heart { Some(&src) } else { None }, &hair, &atlas).unwrap();
            let names: Vec<&str> = b.files.iter().map(|f| f.0.as_str()).collect();
            for ext in ["MDL", "ANN", "EET", "SES"] {
                assert!(names.contains(&path(target, ext).as_str()), "{target}: {ext}");
            }
            for ext in ["RTM", "TXA"] {
                assert!(names.contains(&format!("raw/{}", path(target, ext)).as_str()), "{target}: {ext}");
            }
            let mdl = &b.files.iter().find(|f| f.0 == path(target, "MDL")).unwrap().1;
            let (old, new) = (crate::mdl::split_parts(&plain_mdl).unwrap(), crate::mdl::split_parts(mdl).unwrap());
            assert_ne!(old[3], new[3], "{target}: the body's hair changed");
            let p = crate::mdl::MdlPart::parse_bytes(&new[3]).unwrap();
            // 0.198.13: one name per model's picture (`ddd_hair::RIKU_0111_BBS`)
            assert!(p.texture_names().iter().any(|t| t.as_str() == atlas.new_texture), "{target}: the BBS atlas");
        }
    }

    /// 0.197.4 (owner's install, `RECOM_GAME`): the 109-bone copies of Riku's event packs: every pack of [`EVENT_PACKS`] is copied,
    /// every clip has the KH2-hair model's bone count, the same slots, frames and loop, and Re:CoM's channels of the 102 bones
    /// except the hair's unchanged.
    #[test]
    fn the_event_packs_have_kh2_hair_copies() {
        let Some(g) = game() else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let hair = crate::kh2_hair::Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap();
        let mdl = install::read_recom(&arc, &keys, &path(101, "MDL")).unwrap();
        let model_bones = crate::mdl::RecomSkeleton::parse(&crate::ddd_hair::HairSource::prepare_mdl(&hair, &mdl).unwrap()).unwrap().bones.len();
        let sk = crate::mdl::RecomSkeleton::parse(&mdl).unwrap();
        let packs = build_event_packs(&arc, &keys, &hair).unwrap();
        assert_eq!(packs.iter().map(|p| (p.0, p.1)).collect::<Vec<_>>(), EVENT_PACKS.map(|g| (g, g + EVENT_ALT)), "all eight");
        for (src, alt, bytes) in &packs {
            // 0.198.21: PL4042 against its fixed pant order ([`crate::pants_order`])
            let raw = install::read_recom(&arc, &keys, &path(*src, "ANN")).unwrap();
            let raw = crate::pants_order::fix_bytes(&raw, &sk).unwrap().filter(|_| crate::pants_order::FIXED_PACKS.contains(src)).unwrap_or(raw);
            let (a0, a1) = (Ann::parse(&raw).unwrap(), Ann::parse(bytes).unwrap());
            assert_eq!(a0.slots.len(), a1.slots.len(), "{alt}");
            for (s0, s1) in a0.slots.iter().zip(&a1.slots) {
                let (Some(c0), Some(c1)) = (s0.clip.as_ref(), s1.clip.as_ref()) else { assert!(s0.clip.is_none() && s1.clip.is_none()); continue };
                assert_eq!(c1.bones as usize, model_bones, "{alt}: {model_bones} bones");
                assert_eq!((c0.frames, c0.loop_start, c0.flags), (c1.frames, c1.loop_start, c1.flags), "{alt}");
                let ch = c0.chans();
                for b in 0..c0.bones as usize {
                    if sk.bones[b].name.contains("hair") {
                        continue;
                    }
                    assert!(c0.channels[b * ch..(b + 1) * ch] == c1.channels[b * ch..(b + 1) * ch], "{alt}: bone {b} {} changed", sk.bones[b].name);
                }
            }
        }
        eprintln!("event pack copies: {} ({} bones)", packs.len(), model_bones);
    }

    /// 0.184.2 (owner's install, `RECOM_GAME`): the Dark Firaga clip's glow, flash, voice and thrust sound on Riku's
    /// cast clips, at their retimed frames, on his bones.
    #[test]
    fn rikus_cast_clips_carry_dark_firagas_events() {
        let Some(g) = game() else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let rd = |p: &str| install::read_recom(&arc, &keys, p).unwrap();
        let df_ann = Ann::parse(&rd("PLAYER/1088/PL1088.ANN")).unwrap();
        let df = df_ann.slots.iter().find_map(|s| s.clip.as_ref().filter(|c| c.name_str() == DARK_FIRAGA)).unwrap();
        let ds = RecomSkeleton::parse(&rd("PLAYER/0201/PL0201.MDL")).unwrap();
        let rs = RecomSkeleton::parse(&rd("PLAYER/0101/PL0101.MDL")).unwrap();
        let eet = crate::eet::parse(&rd("PLAYER/1088/PL1088.EET")).unwrap();
        let ses = crate::ses::parse(&rd("PLAYER/1088/PL1088.SES")).unwrap();
        let to_riku = |b: i32| ds.bones.get(b as usize).and_then(|x| rs.find(if x.name == "ex_1580_sk" { "ex_1010_sk" } else { &x.name })).map(|i| i as i32);
        let rk = crate::eet::parse(&rd("PLAYER/1301/PL1301.EET")).unwrap().into_iter().find(|r| r.name == "p002_062").unwrap();
        let (cloak, blade) = cloak_of(&rk);
        assert!(cloak.len() >= 10 && blade.is_some(), "Riku's Impulse dresses him: {} cloak programs", cloak.len());
        let (r, e) = cast_events(df, &eet, &ses, "p002_", &to_riku, &cloak, blade, &FxBones::of(&rs)).unwrap();
        let v = |e: &[u8]| i32::from_le_bytes(e[0x5C..0x60].try_into().unwrap());
        let cu = r.iter().find(|x| x.name == "p002_230").unwrap();
        let count = |lv: i32| cu.a.iter().filter(|e| v(&e[..]) == lv).count();
        // 0.185.8: Thunder / Aero have their own effects, Thunder's on variants 1..3 (more with the level), Aero's on 4..6.
        assert!(count(1) >= 1 && count(1) < count(2) && count(2) < count(3), "more effects per level: {} / {} / {}", count(1), count(2), count(3));
        assert!(count(4) >= 1 && count(4) < count(5) && count(5) < count(6), "Aero's own variants: {} / {} / {}", count(4), count(5), count(6));
        assert!(cu.a.iter().all(|e| i32::from_le_bytes(e[0x20..0x24].try_into().unwrap()) != 1 || &e[..13] != b"dark_faiga.dp"), "no Dark Firaga flash on Thunder");
        let r: Vec<crate::eet::Record> = r.into_iter().map(|mut x| { x.a.retain(|e| v(&e[..]) == 1); x }).collect();
        let i = |e: &[u8], o: usize| i32::from_le_bytes(e[o..o + 4].try_into().unwrap());
        let hand = rs.find("l_hand").unwrap() as i32;
        // Fire's wind-up (0.184.6: the Dark Firaga clip at its speed, from its frame 18): the glow on his left hand and the
        // voice, both before the clip's start, on its first frame.
        let w = r.iter().find(|x| x.name == "p002_227").unwrap();
        assert_eq!(w.a.iter().map(|e| (i(e, 0x20), i(e, 0x24), i(e, 0x34))).collect::<Vec<_>>(), vec![(0, hand, 0)]);
        let wv = e.iter().find(|x| x.name() == "p002_227").unwrap();
        assert_eq!(wv.events.iter().map(|e| (i(e, 0), i(e, 4), i(e, 0x18))).collect::<Vec<_>>(), vec![(3, 35, 0)]);
        // Fire's release: the flash and the thrust's sound 8 frames in (72 → 36, the thrust's end, at the clip's speed).
        let rl = r.iter().find(|x| x.name == "p002_228").unwrap();
        assert_eq!(rl.a.iter().map(|e| (i(e, 0x20), i(e, 0x24), i(e, 0x34))).collect::<Vec<_>>(), vec![(1, 0, 8)]);
        let rs_ = e.iter().find(|x| x.name() == "p002_228").unwrap();
        assert_eq!(rs_.events.iter().map(|e| (i(e, 0), i(e, 4), i(e, 0x18))).collect::<Vec<_>>(), vec![(605, 2, 8)]);
        // Cure (0.184.8): no effects of the Dark Firaga clip, only the voice; Thunder (release 30; 0.185.8: its own ex_1580 19 on the chest 10 frames before it) has it at 20.
        assert!(r.iter().all(|x| x.name != "p002_231" || x.a.is_empty()));
        let cv = e.iter().find(|x| x.name() == "p002_231").unwrap();
        assert!(cv.events.iter().all(|ev| i(ev, 0) == 3), "Cure: the voice only");
        let th = r.iter().find(|x| x.name == "p002_230").unwrap();
        assert!(th.a.iter().any(|e| (i(e, 0x20), i(e, 0x34)) == (19, 20)), "{:?}", th.a.iter().map(|e| (i(e, 0x20), i(e, 0x34))).collect::<Vec<_>>());
        assert!(e.iter().all(|x| x.events.iter().all(|ev| (i(ev, 0), i(ev, 4)) != (605, 1))));
    }

    /// 0.184.1 / 0.184.4: each cast clip has KH1's length, its source's key moment on KH1's release frame, the charge
    /// before it, the recovery to the free frame and its last frame held.
    #[test]
    fn cast_clips_put_the_key_moment_on_kh1s_release() {
        for c in CAST_CLIPS {
            let s = c.source();
            let t = c.times();
            assert_eq!(t.len(), c.len, "{}", c.suffix);
            let r = c.release as usize;
            if r < c.len {
                assert_eq!(t[r], s.charge_end, "{}: the key moment starts on the release", c.suffix);
            }
            // 0.184.6: never faster than the clip's own speed (no flicker).
            let step = s.fps / 60.0;
            assert!(t.windows(2).all(|w| w[1] >= w[0]), "{}: never backwards", c.suffix);
            assert!(t.iter().all(|&x| (0.0..=s.last).contains(&x)));
            assert!(t.windows(2).all(|w| w[1] - w[0] <= step + 1e-9), "{}: never faster than the clip", c.suffix);
        }
        for (w, r) in [(0, 1), (6, 7)] {
            let s = CAST_CLIPS[w].source();
            assert!(CAST_CLIPS[w].times().iter().all(|&x| x < s.charge_end), "Fire's wind-up is the charge only");
            assert_eq!(CAST_CLIPS[r].times()[0], CAST_CLIPS[r].source().charge_end, "Fire's release starts on the thrust");
        }
        // 0.185.0: every clip's source has a fallback chain ending on a Re:CoM / KH1 motion (no BBS needed).
        for c in CAST_CLIPS {
            let mut s = c.source();
            let mut hops = 0;
            while matches!(s.riku, Src::Terra(_)) {
                s = &CAST_SOURCES[s.fallback.expect("a Terra source has a fallback")];
                hops += 1;
                assert!(hops < 4);
            }
        }
    }


    /// 0.184.6 (owner's install, `RECOM_GAME`; "as animação ficaram estranhas meio que flicando"): no bone of the body,
    /// hands or blade turns (its local Euler channels) more in one KH1 frame than its source clip does in one (half a 30
    /// fps frame), except Dark Riku's clips retargeted onto Riku. 0.184.4 squeezed the key moments and failed this.
    #[test]
    fn cast_clips_turn_no_faster_than_their_sources() {
        let Some(g) = game() else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let rd = |p: &str| install::read_recom(&arc, &keys, p).unwrap();
        let ds = RecomSkeleton::parse(&rd("PLAYER/0201/PL0201.MDL")).unwrap();
        let rs = RecomSkeleton::parse(&rd("PLAYER/0101/PL0101.MDL")).unwrap();
        let ann = Ann::parse(&rd("PLAYER/0101/PL0101.ANN")).unwrap();
        let packs: std::cell::RefCell<std::collections::HashMap<u16, Ann>> = Default::default();
        let get = |g: u16, name: &str| -> Result<Clip> {
            let mut p = packs.borrow_mut();
            let a = p.entry(g).or_insert_with(|| Ann::parse(&rd(&path(g, "ANN"))).unwrap());
            Ok(a.slots.iter().find_map(|s| s.clip.as_ref().filter(|c| c.name_str() == name)).cloned().unwrap())
        };
        let step = |c: &Clip, b: usize, t0: f64, t1: f64| -> f64 {
            let (x, y) = (c.sample(b, t0).r, c.sample(b, t1).r);
            (0..3).map(|i| { let d = (y[i] - x[i]).to_degrees().rem_euclid(360.0); d.min(360.0 - d) }).fold(0.0, f64::max)
        };
        let bones = ["chest1", "head", "l_hand", "r_hand", "wep0", "l_uparm", "r_uparm", "l_arm", "r_arm"];
        let ss = RecomSkeleton::parse(&rd("PLAYER/0001/PL0001.MDL")).unwrap();
        let k1 = install::kh1(&g, keys.clone()).unwrap();
        let (am, ad) = (k1.read("xa_ex_1580.mset").unwrap(), k1.read("xa_ex_1580.mdls").unwrap());
        let stance = ann.slots.iter().find_map(|s| s.clip.as_ref().filter(|c| c.name_str() == RIKU_STANCE)).cloned();
        let terra_data: Option<(RecomSkeleton, Vec<crate::pam::PamClip>)> = crate::terra::read_terra(&g, &keys).ok();
        let ctx = CastCtx { get: &get, riku: &rs, dark: &ds, sora: &ss, stance: stance.as_ref(), ansem: Some((&am, &ad)), terra: terra_data.as_ref().map(|(k, c)| (k, c.as_slice())) };
        let clips = riku_cast_clips(&ctx).unwrap();
        let darks = dark_cast_clips(&ctx).unwrap();
        for (k, c) in CAST_CLIPS.iter().enumerate() {
            let src = ctx.clip_on(&c.source().dark, Skel::Dark).unwrap().unwrap();
            let sstep = c.source().fps / 60.0;
            let idx: Vec<usize> = bones.iter().filter_map(|n| ds.find(n)).collect();
            let mut src_worst = 0.0f64;
            let mut t = 0.0;
            while t + sstep <= c.source().last {
                for &i in &idx {
                    src_worst = src_worst.max(step(&src, i, t, t + sstep));
                }
                t += sstep;
            }
            let mut worst = 0.0f64;
            for f in 0..darks[k].frames as usize - 1 {
                for &i in &idx {
                    worst = worst.max(step(&darks[k], i, f as f64, f as f64 + 1.0));
                }
            }
            // Riku's own (on his skeleton) the same when his source is his.
            let r_worst = if let Src::Pack(g0, n0, Skel::Riku) = c.source().riku {
                let src = get(g0, n0).unwrap();
                let idx: Vec<usize> = bones.iter().filter_map(|n| rs.find(n)).collect();
                let mut sw = 0.0f64;
                let mut t = 0.0;
                while t + SRC_STEP <= c.source().last {
                    for &i in &idx {
                        sw = sw.max(step(&src, i, t, t + SRC_STEP));
                    }
                    t += SRC_STEP;
                }
                let mut w = 0.0f64;
                for f in 0..clips[k].frames as usize - 1 {
                    for &i in &idx {
                        w = w.max(step(&clips[k], i, f as f64, f as f64 + 1.0));
                    }
                }
                assert!(w <= sw + 1.0, "{}: Riku's {w:.1} deg a frame, his source {sw:.1}", c.suffix);
                Some((w, sw))
            } else {
                None
            };
            eprintln!("{}: Dark Riku's worst {worst:.1} deg a frame (source {src_worst:.1}); Riku's {r_worst:?}", c.suffix);
            assert!(worst <= src_worst + 1.0, "{}: {worst:.1} deg a frame, the source {src_worst:.1}", c.suffix);
        }
    }

    /// 0.184.1 (owner's install, `RECOM_GAME`): Riku's cast clips from the Dark Firaga clip: his Soul Eater points
    /// where Dark Riku's does (the blade root `wep0`'s world x axis), and nobody travels.
    #[test]
    fn rikus_cast_clips_follow_dark_firaga() {
        let Some(g) = game() else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let rd = |p: &str| install::read_recom(&arc, &keys, p).unwrap();
        let df_ann = Ann::parse(&rd("PLAYER/1088/PL1088.ANN")).unwrap();
        let df = df_ann.slots.iter().find_map(|s| s.clip.as_ref().filter(|c| c.name_str() == DARK_FIRAGA)).unwrap();
        let ds = RecomSkeleton::parse(&rd("PLAYER/0201/PL0201.MDL")).unwrap();
        let rs = RecomSkeleton::parse(&rd("PLAYER/0101/PL0101.MDL")).unwrap();
        let ann = Ann::parse(&rd("PLAYER/0101/PL0101.ANN")).unwrap();
        let packs: std::cell::RefCell<std::collections::HashMap<u16, Ann>> = Default::default();
        let get = |g: u16, name: &str| -> Result<Clip> {
            let mut p = packs.borrow_mut();
            let a = p.entry(g).or_insert_with(|| Ann::parse(&rd(&path(g, "ANN"))).unwrap());
            Ok(a.slots.iter().find_map(|s| s.clip.as_ref().filter(|c| c.name_str() == name)).cloned().unwrap())
        };
        let ss = RecomSkeleton::parse(&rd("PLAYER/0001/PL0001.MDL")).unwrap();
        let k1 = install::kh1(&g, keys.clone()).unwrap();
        let (am, ad) = (k1.read("xa_ex_1580.mset").unwrap(), k1.read("xa_ex_1580.mdls").unwrap());
        let stance = ann.slots.iter().find_map(|s| s.clip.as_ref().filter(|c| c.name_str() == RIKU_STANCE)).cloned();
        let terra_data: Option<(RecomSkeleton, Vec<crate::pam::PamClip>)> = crate::terra::read_terra(&g, &keys).ok();
        let ctx = CastCtx { get: &get, riku: &rs, dark: &ds, sora: &ss, stance: stance.as_ref(), ansem: Some((&am, &ad)), terra: terra_data.as_ref().map(|(k, c)| (k, c.as_slice())) };
        let clips = riku_cast_clips(&ctx).unwrap();
        let darks = dark_cast_clips(&ctx).unwrap();
        assert_eq!(darks.len(), CAST_CLIPS.len());
        let world = |c: &Clip, sk: &RecomSkeleton, f: f64, bone: &str| -> (M3, [f64; 3]) {
            let k = sk.find(bone).unwrap();
            let mut chain = vec![k];
            while sk.bones[*chain.last().unwrap()].parent >= 0 {
                chain.push(sk.bones[*chain.last().unwrap()].parent as usize);
            }
            let mut u = crate::math::I3;
            let mut p = [0.0; 3];
            for &b in chain.iter().rev() {
                let s = c.sample(b, f);
                let w = crate::math::vm(s.t, &u);
                p = [p[0] + w[0], p[1] + w[1], p[2] + w[2]];
                u = m3(&rot_xyz(s.r[0], s.r[1], s.r[2]), &u);
            }
            (u, p)
        };
        assert_eq!(df.fps(), 30, "the Dark Firaga clip is 30 fps");
        for (k, c) in CAST_CLIPS.iter().enumerate() {
            assert_eq!(clips[k].frames as usize, c.len);
            assert_eq!(clips[k].fps(), 60, "{}: one frame per KH1 frame", c.suffix);
            assert_eq!((darks[k].frames as usize, darks[k].fps()), (c.len, 60), "{}: Dark Riku's", c.suffix);
            if c.src != 0 {
                continue;
            }
            assert_eq!(clips[k].name_str(), format!("p002_{}", c.suffix));
            // Each clip's first frame is the reference (the Dark Firaga clip's time there for Dark's).
            let rest_dark = world(df, &ds, c.times()[0], "wep0").0;
            let rest_riku = world(&clips[k], &rs, 0.0, "wep0").0;
            for (f, &t) in c.times().iter().enumerate().step_by(7) {
                let (ud, _) = world(df, &ds, t, "wep0");
                let (ur, _) = world(&clips[k], &rs, f as f64, "wep0");
                // The blade turns as Dark's does from the clip's first frame.
                let dd = m3(&ud, &tr(&rest_dark))[0];
                let dr = m3(&ur, &tr(&rest_riku))[0];
                let cos = dd[0] * dr[0] + dd[1] * dr[1] + dd[2] * dr[2];
                if k == 1 && f % 14 == 0 {
                    eprintln!("{} frame {f} (Dark Firaga {t:.1}): blade turn cos {cos:.3}", c.suffix);
                }
                assert!(cos > 0.9, "{} frame {f}: blade turn differs (cos {cos:.2})", c.suffix);
                let (_, hips) = world(&clips[k], &rs, f as f64, &rs.bones[1].name.clone());
                assert!(hips[0].abs() < 0.05 && hips[2].abs() < 0.05, "{} frame {f}: no travel {hips:?}", c.suffix);
            }
        }
    }

    /// 0.183.2: with KH1 Riku's guard, Dark Riku's own packs (201 / 211) get it and its recoil on his skeleton.
    #[test]
    fn dark_rikus_own_pack_gets_the_guard() {
        let Some(g) = game() else { return };
        let keys = install::keys(&g).unwrap();
        let k1 = install::kh1(&g, keys.clone()).unwrap();
        let (mset, mdls) = (k1.read("xa_ex_1010.mset").unwrap(), k1.read("xa_ex_1010.mdls").unwrap());
        let arc = install::recom_archive(&g).unwrap();
        let src = GuardSource { mset: &mset, mdls: &mdls, ansem: None, terra: None };
        for &(riku, dark, skin) in &SKIN_GROUPS {
            let b = build_with(&arc, &keys, riku, dark, skin, Some(&src)).unwrap();
            let (_, bytes) = b.files.iter().find(|(p, _)| *p == path(dark, "ANN")).expect("Dark Riku's own pack");
            let ann = Ann::parse(bytes).unwrap();
            let own = Ann::parse(&install::read_recom(&arc, &keys, &path(dark, "ANN")).unwrap()).unwrap();
            assert_eq!(ann.slots.len(), own.slots.len() + 2 + CAST_CLIPS.len(), "group {dark}: his own slots, then the guard's two and the six cast clips (0.184.1)");
            for name in [DARK_GUARD, DARK_RECOIL] {
                let c = ann.find(name).and_then(|i| ann.slots[i].clip.as_ref()).unwrap_or_else(|| panic!("group {dark}: {name}"));
                assert!(c.frames > 30, "{name}: {} frames", c.frames);
            }
            // 0.185.5: the Guard's whoosh and random grunt at frame 0 in Riku's, Dark Riku's and the skin's SES, and every
            // cast voice picks from `CAST_VOICES`.
            for (g, guard) in [(riku, RIKU_GUARD), (dark, DARK_GUARD), (skin, RIKU_GUARD)] {
                let (_, bytes) = b.files.iter().find(|(p, _)| *p == path(g, "SES")).unwrap_or_else(|| panic!("group {g}: SES"));
                let ses = crate::ses::parse(bytes).unwrap();
                let at = |e: &[u8; 0x50], o: usize| u32::from_le_bytes(e[o..o + 4].try_into().unwrap());
                let e = ses.iter().find(|e| e.name() == guard).unwrap_or_else(|| panic!("group {g}: no {guard}"));
                assert!(e.events.iter().any(|x| at(x, 0) == 1 && at(x, 0x18) == 0), "group {g}: {guard}'s whoosh");
                assert!(e.events.iter().any(|x| at(x, 0) == 3 && at(x, 0x28) & 4 != 0 && at(x, 0x18) == 0), "group {g}: {guard}'s grunt");
                let prefix = if g == dark { "p003_" } else { "p002_" };
                for (ci, c) in CAST_CLIPS.iter().enumerate() {
                    let n = format!("{prefix}{}", c.suffix);
                    let Some(e) = ses.iter().find(|e| e.name() == n) else { continue };
                    for x in e.events.iter().filter(|x| at(x, 0) == 3) {
                        assert_eq!((at(x, 4), at(x, 0x28) & 4), (CAST_VOICES[ci % 6], 0), "group {g}: {n}'s voice is its spell's, fixed");
                    }
                }
            }
        }
    }

    /// With the owner's install (`RECOM_GAME`): Riku's guard clip lowers the guard as the defence
    /// ends (0.180.1); prints the pose's distance from the held guard and the hands per frame.
    #[test]
    fn riku_guard_motion_profile() {
        let Some(g) = game() else { return };
        let keys = install::keys(&g).unwrap();
        let k1 = install::kh1(&g, keys.clone()).unwrap();
        let (mset, mdls) = (k1.read("xa_ex_1010.mset").unwrap(), k1.read("xa_ex_1010.mdls").unwrap());
        let arc = install::recom_archive(&g).unwrap();
        let riku = RecomSkeleton::parse(&install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap()).unwrap();
        let c = riku_kh1_guard(&mset, &mdls, &riku).unwrap();
        let n = c.frames as usize;
        let pose = |f: usize| -> Vec<[f64; 3]> { (0..riku.bones.len()).map(|b| c.sample(b, f as f64).r).collect() };
        let base = pose(RIKU_GUARD_UP + 5);
        let line: Vec<String> = (0..n).map(|f| {
            let p = pose(f);
            let d: f64 = p.iter().zip(&base).map(|(a, b)| (0..3).map(|i| (a[i] - b[i]).abs()).sum::<f64>()).sum::<f64>().to_degrees();
            format!("{f}:{d:.0}")
        }).collect();
        eprintln!("guard clip ({n} frames), distance from the held pose: {}", line.join(" "));
        // The weapon hand's height and the blade tip's, in metres, per frame (world, from the clip).
        let world = |f: usize, bone: &str| -> [f64; 3] {
            let k = riku.find(bone).unwrap();
            let mut chain = vec![k];
            while riku.bones[*chain.last().unwrap()].parent >= 0 {
                chain.push(riku.bones[*chain.last().unwrap()].parent as usize);
            }
            let mut u = crate::math::I3;
            let mut p = [0.0; 3];
            for &b in chain.iter().rev() {
                let s = c.sample(b, f as f64);
                let w = crate::math::vm(s.t, &u);
                p = [p[0] + w[0], p[1] + w[1], p[2] + w[2]];
                u = m3(&rot_xyz(s.r[0], s.r[1], s.r[2]), &u);
            }
            p
        };
        for bone in ["r_hand", "l_hand"] {
            let ys: Vec<String> = (0..n).map(|f| { let p = world(f, bone); format!("{f}:{:.2}/{:.2}", p[1], p[2]) }).collect();
            eprintln!("{bone} y/z: {}", ys.join(" "));
        }
        // 0.180.1: the weapon hand (held forward, z) starts down right after the live window (10..36).
        let z = |f: usize| world(f, "r_hand")[2];
        assert!(z(38) < z(36) - 0.04, "the guard comes down at once: z {:.2} at 36, {:.2} at 38", z(36), z(38));
        assert!(z(n - 1) < z(36) - 0.3, "and ends lowered");
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

    /// 0.189.0 (owner's install, `RECOM_GAME`; BBS FM only, no 2.8 collection): Terra's casts (`315`..`318`) and Guard
    /// (`354`) on Riku's skeleton keep his feet on the floor where they start, his `hip` at rest, the blade in his stance's
    /// grip, and the air clips start at his rest height. `TERRA_POSE_DUMP=<file>` writes every bone's world position per
    /// frame of the built cast clips and the Guard (for the renders).
    #[test]
    fn terra_motions_on_riku() {
        let Some(g) = game() else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let rd = |p: &str| install::read_recom(&arc, &keys, p).unwrap();
        let rs = RecomSkeleton::parse(&rd("PLAYER/0101/PL0101.MDL")).unwrap();
        let ds = RecomSkeleton::parse(&rd("PLAYER/0201/PL0201.MDL")).unwrap();
        let ss = RecomSkeleton::parse(&rd("PLAYER/0001/PL0001.MDL")).unwrap();
        let ann = Ann::parse(&rd("PLAYER/0101/PL0101.ANN")).unwrap();
        let stance = ann.slots.iter().find_map(|s| s.clip.as_ref().filter(|c| c.name_str() == RIKU_STANCE)).cloned();
        let (tsk, tclips) = crate::terra::read_terra(&g, &keys).unwrap();
        let get = |_: u16, _: &str| -> Result<Clip> { bad("none") };
        let ctx = CastCtx { get: &get, riku: &rs, dark: &ds, sora: &ss, stance: stance.as_ref(), ansem: None, terra: Some((&tsk, &tclips)) };
        let world = |sk: &RecomSkeleton, c: &Clip, f: f64| -> Vec<[f64; 3]> {
            let (mut u, mut p): (Vec<crate::math::M3>, Vec<[f64; 3]>) = (vec![], vec![]);
            for (i, b) in sk.bones.iter().enumerate() {
                let sm = c.sample(i, f);
                let r = rot_xyz(sm.r[0], sm.r[1], sm.r[2]);
                if b.parent < 0 {
                    u.push(r);
                    p.push(sm.t);
                } else {
                    let q = b.parent as usize;
                    let w = crate::math::vm(sm.t, &u[q]);
                    p.push([w[0] + p[q][0], w[1] + p[q][1], w[2] + p[q][2]]);
                    u.push(m3(&r, &u[q]));
                }
            }
            p
        };
        let (lf, rf) = (rs.find("l_foot").unwrap(), rs.find("r_foot").unwrap());
        let low = |c: &Clip, f: f64| {
            let w = world(&rs, c, f);
            w[lf][1].min(w[rf][1])
        };
        let hip = rs.find("hip").unwrap();
        let body = rs.find("ex_1010_sk").unwrap();
        let (w0, hand) = (rs.find("wep0").unwrap(), rs.find("r_hand").unwrap());
        let world_u = |c: &Clip, f: f64| -> Vec<crate::math::M3> {
            let mut u: Vec<crate::math::M3> = vec![];
            for (i, b) in rs.bones.iter().enumerate() {
                let sm = c.sample(i, f);
                let r = rot_xyz(sm.r[0], sm.r[1], sm.r[2]);
                u.push(if b.parent < 0 { r } else { m3(&r, &u[b.parent as usize]) });
            }
            u
        };
        let want = {
            let u = world_u(stance.as_ref().unwrap(), 0.0);
            m3(&BLADE_FLIP, &m3(&u[w0], &tr(&u[hand])))
        };
        let mut built = riku_cast_clips(&ctx).unwrap();
        let guard = terra_guard(&ctx).unwrap().unwrap();
        assert_eq!((guard.name_str().as_str(), guard.frames as usize), (RIKU_GUARD, RIKU_GUARD_FRAMES));
        built.push(guard);
        if let Some(out) = std::env::var_os("TERRA_POSE_DUMP") {
            let mut txt = String::new();
            let mut all: Vec<(String, Clip)> = built.iter().map(|c| (c.name_str().to_string(), c.clone())).collect();
            // The raw retargeted clips (their own frames), for the key moments.
            for n in ["315", "316", "317", "318", crate::terra::GUARD] {
                all.push((format!("raw{n}"), ctx.clip_on(&Src::Terra(n), Skel::Riku).unwrap().unwrap()));
            }
            for (name, cl) in &all {
                for f in 0..cl.frames as usize {
                    for (i, p) in world(&rs, cl, f as f64).iter().enumerate() {
                        txt += &format!("{name} {f} {} {} {:.4} {:.4} {:.4}\n", rs.bones[i].name, rs.bones[i].parent, p[0], p[1], p[2]);
                    }
                }
            }
            std::fs::write(out, txt).unwrap();
        }
        for cl in &built {
            for f in 0..cl.frames as usize {
                let t = cl.sample(hip, f as f64).t;
                assert!((0..3).all(|k| (t[k] - rs.bones[hip].local.t[k]).abs() < 1e-3), "{}: hip moved at {f}", cl.name_str());
                let got = m3(&world_u(cl, f as f64)[w0], &tr(&world_u(cl, f as f64)[hand]));
                assert!(got.iter().flatten().zip(want.iter().flatten()).all(|(a, e)| (a - e).abs() < 1e-3), "{} frame {f}: the grip off the stance's", cl.name_str());
            }
        }
        // The ground clips (Terra's 315 / 317 and the Guard): the lower foot stays on the floor (the stance's own: frame 0).
        let floor = low(stance.as_ref().unwrap(), 0.0);
        for (n, cl) in [("315", &built[0]), ("317", &built[2]), (RIKU_GUARD, built.last().unwrap())] {
            let worst = (0..cl.frames as usize).map(|f| low(cl, f as f64) - floor).fold(f64::MIN, f64::max);
            eprintln!("{n}: the lower foot at most {worst:.3} m over the stance's floor ({floor:.3})");
            assert!(worst < 0.04 && worst > -0.01, "{n}: the lower foot {worst:.3} m up");
        }
        // 0.193.0: the ground casts keep the body over its stance position (no travel), its height within the drop / rise
        // limits and moving a little each frame (no hop).
        for n in ["315", "317"] {
            let c = ctx.clip_on(&Src::Terra(n), Skel::Riku).unwrap().unwrap();
            let st0 = stance.as_ref().unwrap().sample(body, 0.0).t;
            let mut last = st0[1];
            for f in 0..c.frames as usize {
                let t = c.sample(body, f as f64).t;
                assert!((t[0] - st0[0]).abs() < 1e-3 && (t[2] - st0[2]).abs() < 1e-3, "{n} frame {f}: the body travelled");
                assert!(t[1] - st0[1] > -TAME_DROP - 0.01 && t[1] - st0[1] < 0.06, "{n} frame {f}: the body height {:.3} off the stance's", t[1] - st0[1]);
                assert!((t[1] - last).abs() < 0.03, "{n} frame {f}: the body moved {:.3} m in one frame", t[1] - last);
                last = t[1];
            }
        }
        // The air clips keep the body at its stance height.
        for n in ["316", "318"] {
            let c = ctx.clip_on(&Src::Terra(n), Skel::Riku).unwrap().unwrap();
            let st0 = stance.as_ref().unwrap().sample(body, 0.0).t;
            for f in 0..c.frames as usize {
                let t = c.sample(body, f as f64).t;
                assert!((0..3).all(|k| (t[k] - st0[k]).abs() < 1e-3), "{n} frame {f}: the body left its stance position");
            }
        }
    }

    /// 0.198.19 (owner's install, `RECOM_GAME`; fork owner: the effects not adjusted to Terra's casts, "algumas ficaram
    /// estranhas"): on Riku's and Dark Riku's built casts, from KH1's release frame to the key moment's end, the blade (`wep0`,
    /// where Thunder's vortex sits; the DLL's shots start at its end) and the blade hand are in front of the chest; on the
    /// raise (Thunder / Aero, Cure, Gravity / Stop) the blade is up over the shoulders; Blizzard's left-hand shard (level 3)
    /// starts in front. The model faces +z (its toes, Checked (files)). The raise keyed 10..14 failed this (the blade 0.57 m
    /// behind the body on the release). Thunder's vortex point is within 6 cm of the grip on the release.
    #[test]
    fn cast_effects_meet_the_gesture() {
        let Some(g) = game() else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let rd = |p: &str| install::read_recom(&arc, &keys, p).unwrap();
        let rs = RecomSkeleton::parse(&rd("PLAYER/0101/PL0101.MDL")).unwrap();
        let ds = RecomSkeleton::parse(&rd("PLAYER/0201/PL0201.MDL")).unwrap();
        let ss = RecomSkeleton::parse(&rd("PLAYER/0001/PL0001.MDL")).unwrap();
        let ann = Ann::parse(&rd("PLAYER/0101/PL0101.ANN")).unwrap();
        let stance = ann.slots.iter().find_map(|s| s.clip.as_ref().filter(|c| c.name_str() == RIKU_STANCE)).cloned();
        let (tsk, tclips) = crate::terra::read_terra(&g, &keys).unwrap();
        let get = |_: u16, _: &str| -> Result<Clip> { bad("none") };
        let ctx = CastCtx { get: &get, riku: &rs, dark: &ds, sora: &ss, stance: stance.as_ref(), ansem: None, terra: Some((&tsk, &tclips)) };
        const RAISE: [&str; 6] = ["230", "231", "232", "236", "237", "238"];
        for (who, sk, clips) in [("Riku", &rs, riku_cast_clips(&ctx).unwrap()), ("Dark Riku", &ds, dark_cast_clips(&ctx).unwrap())] {
            let b = |n: &str| sk.find(n).unwrap_or_else(|| panic!("{who}: no {n}"));
            let (chest, blade, hand, lh, collar) = (b("chest1"), b("wep0"), b("r_hand"), b("l_hand"), b("r_collar"));
            for (c, cl) in CAST_CLIPS.iter().zip(&clips) {
                if c.is_windup() {
                    continue;
                }
                let r = c.release as usize;
                let to = (c.release + (c.source().key_end - c.source().charge_end) / (c.source().fps / 60.0)) as usize;
                let mut worst = (f64::MAX, f64::MAX, 0usize);
                for f in r..=to.min(cl.frames as usize - 1) {
                    let w = crate::kh2_hair::tests::posed_world(sk, cl, f as f64);
                    let ahead = (w[blade].t[2] - w[chest].t[2]).min(w[hand].t[2] - w[chest].t[2]);
                    let up = w[blade].t[1] - w[collar].t[1];
                    if ahead < worst.0 {
                        worst = (ahead, worst.1, f);
                    }
                    worst.1 = worst.1.min(up);
                }
                // Thunder's vortex (Root, [`thunder_vortex_at`]) at the grip on the release; Root unturned, so its frame is the model's.
                if matches!(c.suffix, "230" | "236") {
                    let w = crate::kh2_hair::tests::posed_world(sk, cl, c.release);
                    let id = (0..3).all(|i| (0..3).all(|j| (w[0].r[i][j] - if i == j { 1.0 } else { 0.0 }).abs() < 1e-3));
                    assert!(id, "{who} {}: Root turned", c.suffix);
                    let v = thunder_vortex_at(c);
                    let d = (0..3).map(|k| (w[blade].t[k] - w[0].t[k] - v[k] as f64).powi(2)).sum::<f64>().sqrt();
                    eprintln!("{who} {}: the vortex {d:.3} m from the grip on the release", c.suffix);
                    assert!(d < 0.06, "{who} {}: the vortex {d:.3} m from the grip", c.suffix);
                }
                eprintln!("{who} {}: blade / hand at least {:.2} m ahead of the chest (frame {}), blade {:+.2} m over the shoulder", c.suffix, worst.0, worst.2, worst.1);
                assert!(worst.0 > 0.1, "{who} {}: the blade or its hand {:.2} m ahead of the chest at frame {}", c.suffix, worst.0, worst.2);
                if RAISE.contains(&c.suffix) {
                    assert!(worst.1 > 0.0, "{who} {}: the raised blade {:.2} m under the shoulder", c.suffix, worst.1);
                }
                if matches!(c.suffix, "229" | "235") {
                    let w = crate::kh2_hair::tests::posed_world(sk, cl, c.release);
                    assert!(w[lh].t[2] - w[chest].t[2] > 0.1, "{who} {}: Blizzard's left hand behind on the release", c.suffix);
                }
            }
        }
    }

    /// Debug (`TERRA_MESH_OUT=<dir>`, `TERRA_MESH_TAG`, `TERRA_MESH_STEP` game frames, default 4): Riku's whole body mesh (flat
    /// shaded, depth buffered, coloured by bone group) posed by the built Terra casts and the Guard, side and front, one sheet
    /// per clip. For the legs to be judged on the real model.
    #[test]
    fn terra_mesh_sheets() {
        let (Some(g), Some(out)) = (game(), std::env::var_os("TERRA_MESH_OUT")) else { return };
        let out = std::path::PathBuf::from(out);
        let tag = std::env::var("TERRA_MESH_TAG").unwrap_or_else(|_| "now".into());
        let step: usize = std::env::var("TERRA_MESH_STEP").ok().and_then(|v| v.parse().ok()).unwrap_or(4);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let rd = |p: &str| install::read_recom(&arc, &keys, p).unwrap();
        let mdl = rd("PLAYER/0101/PL0101.MDL");
        let rs = RecomSkeleton::parse(&mdl).unwrap();
        let ds = RecomSkeleton::parse(&rd("PLAYER/0201/PL0201.MDL")).unwrap();
        let ss = RecomSkeleton::parse(&rd("PLAYER/0001/PL0001.MDL")).unwrap();
        let ann = Ann::parse(&rd("PLAYER/0101/PL0101.ANN")).unwrap();
        let stance = ann.slots.iter().find_map(|s| s.clip.as_ref().filter(|c| c.name_str() == RIKU_STANCE)).cloned();
        let (tsk, tclips) = crate::terra::read_terra(&g, &keys).unwrap();
        let get = |_: u16, _: &str| -> Result<Clip> { bad("none") };
        let ctx = CastCtx { get: &get, riku: &rs, dark: &ds, sora: &ss, stance: stance.as_ref(), ansem: None, terra: Some((&tsk, &tclips)) };
        let mut built: Vec<(String, Clip)> = riku_cast_clips(&ctx).unwrap().into_iter().map(|c| (c.name_str().to_string(), c)).collect();
        let gd = terra_guard(&ctx).unwrap().unwrap();
        built.push((gd.name_str().to_string(), gd));
        built.push(("stance".into(), stance.clone().unwrap()));
        // The mesh: every part's triangles with the bone, position and a colour by bone group.
        let mut tris: Vec<([usize; 3], [[f64; 3]; 3], [u8; 3])> = Vec::new();
        for part in crate::mdl::split_parts(&mdl).unwrap() {
            let p = crate::mdl::MdlPart::parse_bytes(&part).unwrap();
            for t in crate::ddd_hair::tests::collect(&p) {
                let nm = |v: &crate::mdl::MeshVertex| rs.bones[v.bone()].name.clone();
                let n = nm(&t.v[0]);
                let col = if n.contains("hair") || n == "head" || n.starts_with("neck") { [200, 200, 215] }
                    else if n.contains("foot") || n.contains("toes") || n.contains("cuffs") { [160, 160, 120] }
                    else if n.contains("femur") || n.contains("tibia") || n.contains("pants") || n.contains("belt") { [40, 60, 150] }
                    else if n.contains("hand") || n.contains("fing") { [230, 190, 160] }
                    else if n.contains("wep") || n.starts_with('k') { [90, 40, 60] }
                    else if n.contains("arm") || n.contains("collar") { [235, 200, 170] }
                    else { [220, 200, 80] };
                tris.push(([t.v[0].bone(), t.v[1].bone(), t.v[2].bone()], t.v.map(|v| v.pos().map(|x| x as f64)), col));
            }
        }
        let world = |c: &Clip, f: f64| -> Vec<crate::math::Aff> {
            let mut w: Vec<crate::math::Aff> = Vec::with_capacity(rs.bones.len());
            for (i, b) in rs.bones.iter().enumerate() {
                let local = if i < c.bones as usize {
                    let s = c.sample(i, f);
                    let mut r = rot_xyz(s.r[0], s.r[1], s.r[2]);
                    for (row, sc) in r.iter_mut().zip(s.s) { for v in row.iter_mut() { *v *= sc; } }
                    crate::math::Aff { r, t: s.t }
                } else { b.local };
                w.push(if b.parent < 0 { local } else { local.mul(&w[b.parent as usize]) });
            }
            w
        };
        const PW: usize = 200;
        const PH: usize = 280;
        for (name, clip) in &built {
            if !matches!(name.as_str(), "p002_228" | "p002_230" | "p002_234" | "p002_236" | "p002_225" | "stance") { continue; }
            let frames: Vec<usize> = (0..clip.frames.max(1) as usize).step_by(step).collect();
            let (w, h) = (PW * frames.len(), PH * 2);
            let mut rgb = vec![30u8; w * h * 3];
            for (ci, &f) in frames.iter().enumerate() {
                let wd = world(clip, f as f64);
                for (vi, side) in [true, false].into_iter().enumerate() {
                    let mut zb = vec![f64::MIN; PW * PH];
                    for (bi, pos, col) in &tris {
                        let p: Vec<[f64; 3]> = (0..3).map(|k| wd[bi[k]].point(pos[k])).collect();
                        // side: horizontal = z (front to the right), depth = -x; front: horizontal = x, depth = z.
                        let pr = |q: &[f64; 3]| -> [f64; 3] {
                            let (hz, d) = if side { (q[2], -q[0]) } else { (q[0], q[2]) };
                            [PW as f64 / 2.0 + hz * 120.0, PH as f64 - 12.0 - q[1] * 120.0, d]
                        };
                        let sc: Vec<[f64; 3]> = p.iter().map(pr).collect();
                        let n = { let a = [p[1][0]-p[0][0], p[1][1]-p[0][1], p[1][2]-p[0][2]]; let b = [p[2][0]-p[0][0], p[2][1]-p[0][1], p[2][2]-p[0][2]]; [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]] };
                        let nl = (n[0]*n[0]+n[1]*n[1]+n[2]*n[2]).sqrt().max(1e-12);
                        let lit = 0.45 + 0.55 * ((n[0] * 0.3 + n[1] * 0.5 + n[2] * 0.8) / nl).abs();
                        let (x0, x1) = (sc.iter().map(|q| q[0]).fold(f64::MAX, f64::min).floor().max(0.0) as usize, (sc.iter().map(|q| q[0]).fold(f64::MIN, f64::max).ceil() as usize).min(PW - 1));
                        let (y0, y1) = (sc.iter().map(|q| q[1]).fold(f64::MAX, f64::min).floor().max(0.0) as usize, (sc.iter().map(|q| q[1]).fold(f64::MIN, f64::max).ceil() as usize).min(PH - 1));
                        let det = (sc[1][1] - sc[2][1]) * (sc[0][0] - sc[2][0]) + (sc[2][0] - sc[1][0]) * (sc[0][1] - sc[2][1]);
                        if det.abs() < 1e-9 { continue; }
                        for y in y0..=y1 {
                            for x in x0..=x1 {
                                let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
                                let l0 = ((sc[1][1] - sc[2][1]) * (px - sc[2][0]) + (sc[2][0] - sc[1][0]) * (py - sc[2][1])) / det;
                                let l1 = ((sc[2][1] - sc[0][1]) * (px - sc[2][0]) + (sc[0][0] - sc[2][0]) * (py - sc[2][1])) / det;
                                let l2 = 1.0 - l0 - l1;
                                if l0 < 0.0 || l1 < 0.0 || l2 < 0.0 { continue; }
                                let d = l0 * sc[0][2] + l1 * sc[1][2] + l2 * sc[2][2];
                                if d <= zb[y * PW + x] { continue; }
                                zb[y * PW + x] = d;
                                let o = 3 * ((vi * PH + y) * w + ci * PW + x);
                                for k in 0..3 { rgb[o + k] = (col[k] as f64 * lit) as u8; }
                            }
                        }
                    }
                    // the floor line
                    for x in 0..PW { let o = 3 * ((vi * PH + PH - 12) * w + ci * PW + x); rgb[o..o + 3].copy_from_slice(&[90, 90, 90]); }
                }
            }
            std::fs::create_dir_all(&out).unwrap();
            std::fs::write(out.join(format!("{tag}_{name}.png")), crate::ddd_hair::tests::png(w, h, &rgb)).unwrap();
        }
    }

    /// The group `grp` as the asset step builds it, for the gap scans: (`.MDL`, `.ANN`). 185 = the Dark Form's BBS, 205 / 206 = the
    /// battle's Dark Riku with BBS's hair, 207 = with KH2's.
    fn gap_group(g: &std::path::Path, grp: u16) -> (Vec<u8>, Ann) {
        gap_group_as(g, grp, std::env::var("DM_BEFORE").is_ok())
    }
    /// [`gap_group`], `before` = 0.198.32's (no neck skirt, Riku's slot map).
    fn gap_group_as(g: &std::path::Path, grp: u16, before: bool) -> (Vec<u8>, Ann) {
        let keys = install::keys(g).unwrap();
        let arc = install::recom_archive(g).unwrap();
        let src = StepSources::read(g, &keys).expect("the step's KH1 sources");
        let have = step_have(&arc, &keys, &src);
        let bbs = crate::bbs_hair::BbsHairSource::read(g, &keys).unwrap();
        if grp == 201 {
            return (install::read_recom(&arc, &keys, &path(201, "MDL")).unwrap(), Ann::parse(&have[&path(201, "ANN")]).unwrap());
        }
        let built = match grp {
            185 => build_hair_from(&arc, &keys, &have, 121, 201, true, 185, None, &bbs, &crate::ddd_hair::HairTarget { neck_skirt: !before, ..crate::ddd_hair::RIKU_0201_BBS }).unwrap(),
            205 | 206 if before => {
                let (dark, atlas) = if grp == 205 { (201, crate::ddd_hair::RIKU_0201_BBS) } else { (211, crate::ddd_hair::RIKU_0211_BBS) };
                build_hair_from(&arc, &keys, &have, dark, dark, true, grp, None, &bbs, &crate::ddd_hair::HairTarget { neck_skirt: false, ..atlas }).unwrap()
            }
            205 => build_dark_mode_bbs(&arc, &keys, &have, 201, 205, &bbs, &crate::ddd_hair::RIKU_0201_BBS, None).unwrap(),
            206 => build_dark_mode_bbs(&arc, &keys, &have, 211, 206, &bbs, &crate::ddd_hair::RIKU_0211_BBS, None).unwrap(),
            207 => {
                let k = crate::kh2_hair::Kh2HairSource::read(&install::kh2(g, keys.clone()).unwrap()).unwrap();
                build_dark_mode_kh2(&arc, &keys, &have, 201, 207, &k, &crate::ddd_hair::RIKU_0201_KH2, None).unwrap()
            }
            _ => panic!("GAP_GROUP"),
        };
        let file = |n: String| built.files.iter().find(|f| f.0 == n).unwrap().1.clone();
        (file(path(grp, "MDL")), Ann::parse(&file(path(grp, "ANN"))).unwrap())
    }

    /// 0.198.33 (owner's install, `RECOM_GAME`): the battle's Dark Riku with BBS's hair (205 / 206) gets the neck skirt
    /// ([`crate::ddd_hair::NECK_SKIRT_Y`]) and nothing else of his model changes: every triangle of 0.198.32's model is there, the new ones
    /// wear the open edge's own texture (not the hair's), join a `head` / `neck*` corner to a hair corner, and in his own clips (idle, walk, run, jump, fall, land,
    /// the seven swings) have no side longer than `NECK_SKIRT_LONGEST` (the band opens as the neck turns under the head; its corners are
    /// the neck's open edge and hair corners, so it cannot reach out of them, but a longer side would be a flap seen as a wing). His hair's sway is baked on his own slots: his battle stance (21) as the idle (the same
    /// keys as slot 1's first frames), not as the run.
    #[test]
    fn the_battle_dark_riku_bbs_neck_is_closed() {
        const NECK_SKIRT_LONGEST: f64 = 0.06;
        let Some(g) = game() else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let src = StepSources::read(&g, &keys).expect("the step's KH1 sources");
        let have = step_have(&arc, &keys, &src);
        let bbs = crate::bbs_hair::BbsHairSource::read(&g, &keys).unwrap();
        for &(target, dark, atlas) in &DARK_MODE_BBS_HAIR_GROUPS {
            let old = build_hair_from(&arc, &keys, &have, dark, dark, true, target, None, &bbs, &crate::ddd_hair::HairTarget { neck_skirt: false, ..atlas }).unwrap();
            let new = build_dark_mode_bbs(&arc, &keys, &have, dark, target, &bbs, &atlas, None).unwrap();
            let file = |b: &Built, n: String| b.files.iter().find(|f| f.0 == n).unwrap().1.clone();
            let (m0, m1) = (file(&old, path(target, "MDL")), file(&new, path(target, "MDL")));
            let sk = RecomSkeleton::parse(&m1).unwrap();
            let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
            let head = names.iter().position(|n| n == "head").unwrap();
            let p = |m: &[u8]| crate::mdl::MdlPart::parse_bytes(&crate::mdl::split_parts(m).unwrap()[3]).unwrap();
            let (p0, p1) = (p(&m0), p(&m1));
            let hair_tex = p1.texture_names().iter().position(|n| n == atlas.new_texture).unwrap() as u32;
            let w = sk.rest_world();
            let key = |t: &crate::ddd_hair::RTri| { let mut k: Vec<[i64; 4]> = t.v.iter().map(|v| { let q = w[v.bone()].point(v.pos().map(|x| x as f64)); [(q[0] * 1e4) as i64, (q[1] * 1e4) as i64, (q[2] * 1e4) as i64, v.bone() as i64] }).collect(); k.sort(); (k, t.tex) };
            let mut before: std::collections::HashMap<_, usize> = Default::default();
            for t in crate::ddd_hair::tests::collect(&p0) { *before.entry(key(&t)).or_default() += 1; }
            let mut added = vec![];
            for t in crate::ddd_hair::tests::collect(&p1) {
                match before.get_mut(&key(&t)) {
                    Some(n) if *n > 0 => *n -= 1,
                    _ => added.push(t),
                }
            }
            assert!(before.values().all(|&n| n == 0), "{target}: a triangle of 0.198.32's model is gone");
            assert!(!added.is_empty() && added.len() <= 80, "{target}: {} new triangles", added.len());
            let mut tex_used = std::collections::BTreeSet::new();
            for t in &added {
                // option 2 (fork owner: "2"): the edge's own texture (the coat's, the face's), not the hair's
                assert_ne!(t.tex, hair_tex, "{target}: a new triangle on the hair's texture");
                tex_used.insert(p1.texture_names()[t.tex as usize].clone());
                assert!(t.v.iter().any(|v| names[v.bone()].starts_with("neck") || v.bone() == head), "{target}: a new triangle off the neck");
            }
            let len = |q: &[[f64; 3]; 3]| (0..3).map(|i| crate::math::dist(q[i], q[(i + 1) % 3])).fold(0.0, f64::max);
            let rest: Vec<f64> = added.iter().map(|t| len(&t.v.map(|v| w[v.bone()].point(v.pos().map(|x| x as f64))))).collect();
            let ann = Ann::parse(&file(&new, path(target, "ANN"))).unwrap();
            let mut worst = (0.0f64, String::new());
            for s in [1usize, 2, 3, 4, 96, 6, 7, 8, 21, 34, 39, 35, 36, 57, 37, 40] {
                let c = ann.slots[s].clip.as_ref().unwrap();
                for f in 0..c.frames as usize {
                    let pw = crate::kh2_hair::tests::posed_world(&sk, c, f as f64);
                    for (t, r) in added.iter().zip(&rest) {
                        let k = len(&t.v.map(|v| pw[v.bone()].point(v.pos().map(|x| x as f64))));
                        if k > worst.0 { worst = (k, format!("{} f{f} (rest {:.1} cm)", c.name_str(), r * 100.0)); }
                    }
                }
            }
            let rest_max = rest.iter().cloned().fold(0.0, f64::max);
            eprintln!("{target}: skirt textures {tex_used:?}");
            eprintln!("{target}: {} skirt triangles, longest side {:.1} cm at rest, {:.1} cm at {}", added.len(), rest_max * 100.0, worst.0 * 100.0, worst.1);
            // not asserted (open, 0.198.33): the skirt webs out behind the neck in his air finisher and jump (11.2 cm sides on 205);
            // `dark_mode_hair_hole_scan` measures where it is drawn over his neck
            let _ = NECK_SKIRT_LONGEST;
            // the sway on his slots: slot 21 (his battle stance) takes the idle's hair keys at frame 0, slot 3 (his run) the run's
            let rig = bbs.sway_rig(&install::read_recom(&arc, &keys, &path(dark, "MDL")).unwrap()).unwrap().unwrap();
            let at = |s: usize, b: usize| { let c = ann.slots[s].clip.as_ref().unwrap(); c.sample(b, 0.0).r };
            for &(_, rb, _) in &rig.pairs {
                let (i, st, run) = (at(1, rb), at(21, rb), at(3, rb));
                assert!((0..3).all(|k| (i[k] - st[k]).abs() < 1e-3), "{target}: bone {rb}: his stance does not start on the idle's sway");
                let _ = run;
            }
        }
    }

    /// Probe: the open edges (used by one triangle) of `GAP_GROUP`'s part 3 among the triangles whose corners are all on `head` /
    /// `neck*`, head-relative at rest (cm), with their bones and texture.
    #[test]
    fn dark_mode_neck_open_edges() {
        let Some(g) = game() else { return };
        let Ok(grp) = std::env::var("GAP_GROUP").map(|x| x.parse::<u16>().unwrap()) else { return };
        let (mdl, _) = gap_group(&g, grp);
        let sk = RecomSkeleton::parse(&mdl).unwrap();
        let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
        let head = names.iter().position(|n| n == "head").unwrap();
        let p3 = crate::mdl::MdlPart::parse_bytes(&crate::mdl::split_parts(&mdl).unwrap()[3]).unwrap();
        let w = sk.rest_world();
        eprintln!("textures {:?}", p3.texture_names());
        let key = |p: [f64; 3]| p.map(|x| (x * 1e4).round() as i64);
        let mut edges: std::collections::HashMap<([i64; 3], [i64; 3]), (usize, [f64; 3], [f64; 3], String, u32)> = Default::default();
        let ok = |b: usize| b == head || names[b].starts_with("neck") || names[b] == "chest1" || names[b].ends_with("collar");
        for t in crate::ddd_hair::tests::collect(&p3) {
            if !t.v.iter().all(|v| ok(v.bone())) || !t.v.iter().any(|v| v.bone() == head || names[v.bone()].starts_with("neck")) { continue; }
            let p = t.v.map(|v| w[v.bone()].point(v.pos().map(|x| x as f64)));
            for (a, b) in [(0, 1), (1, 2), (2, 0)] {
                let (ka, kb) = (key(p[a]), key(p[b]));
                let k = if ka <= kb { (ka, kb) } else { (kb, ka) };
                let e = edges.entry(k).or_insert((0, p[a], p[b], format!("{}-{}", names[t.v[a].bone()], names[t.v[b].bone()]), t.tex));
                e.0 += 1;
            }
        }
        let h = w[head].t;
        let mut open: Vec<_> = edges.values().filter(|e| e.0 == 1).map(|e| {
            let m = [0, 1, 2].map(|k| ((e.1[k] + e.2[k]) / 2.0 - h[k]) * 100.0);
            (m, crate::math::dist(e.1, e.2) * 100.0, e.3.clone(), e.4)
        }).collect();
        open.sort_by(|a, b| a.0[2].total_cmp(&b.0[2]));
        // the hair's corners (its texture), for the distance from each open neck edge's corners
        let hair_tex = p3.texture_names().iter().position(|n| n.starts_with("rk_hair")).map(|x| x as u32);
        let hv: Vec<[f64; 3]> = crate::ddd_hair::tests::collect(&p3).iter().filter(|t| Some(t.tex) == hair_tex).flat_map(|t| t.v.map(|v| w[v.bone()].point(v.pos().map(|x| x as f64)))).collect();
        for (m, l, b, t) in &open {
            if m[1] > 8.0 || m[1] < -14.0 { continue; }
            let p = [0, 1, 2].map(|k| m[k] / 100.0 + h[k]);
            let near = hv.iter().map(|q| (crate::math::dist(*q, p), *q)).min_by(|a, b| a.0.total_cmp(&b.0));
            let ns = near.map_or(String::new(), |(d, q)| format!(" hair {:.1} cm at x{:+.1} y{:+.1} z{:+.1}", d * 100.0, (q[0] - h[0]) * 100.0, (q[1] - h[1]) * 100.0, (q[2] - h[2]) * 100.0));
            eprintln!("open x{:+6.1} y{:+6.1} z{:+6.1} len {l:4.1} {b} tex {t}{ns}", m[0], m[1], m[2]);
        }
    }

    /// Probe (fork owner, 2026-11-27: "percebi que o cabelo do bbs tem alguns espaços vazios no modo dark mode, testei pulando e
    /// atacando"): see-through pixels as the game draws the model (part 3 one-sided by each triangle's facing flag, clockwise on the
    /// monitor = front, `knowledge/ReCoM/recom-model-culling.md`; part 2, the plain proxy, without a cull): a pixel some triangle covers
    /// (drawn two-sided) where the game's draw shows the background, inside a 0.4 m window round the head, over a sphere of views
    /// (yaw every 30 deg, pitch -60..75 every 15), per clip frame. `GAP_GROUP`, `GAP_SLOTS`, `GAP_STEP` (frames); worst views to
    /// `local/work/darkholes/<group>/`.
    #[test]
    fn dark_mode_hair_hole_scan() {
        let Some(g) = game() else { return };
        let Ok(grp) = std::env::var("GAP_GROUP").map(|x| x.parse::<u16>().unwrap()) else { return };
        let (mdl, ann) = gap_group(&g, grp);
        let sk = RecomSkeleton::parse(&mdl).unwrap();
        let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
        let head = names.iter().position(|n| n == "head").unwrap();
        let parts = crate::mdl::split_parts(&mdl).unwrap();
        let p3 = crate::mdl::MdlPart::parse_bytes(&parts[3]).unwrap();
        let p2 = crate::mdl::MdlPart::parse_bytes(&parts[2]).unwrap();
        // (corners (bone, pos), facing flag, kind: 3 = part 3, 2 = proxy)
        let mut tris: Vec<([(usize, [f64; 3]); 3], i16, u8)> = vec![];
        let mut t3 = crate::ddd_hair::tests::collect(&p3);
        if let Some(lb) = p3.list_b.as_ref() { t3.extend(crate::ddd_hair::tests::extract_list(lb)); }
        // kind 4: a triangle 0.198.32's model does not have (the neck skirt), for `GAP_GROUP` 185 / 205 / 206
        let w0 = sk.rest_world();
        let key = |t: &crate::ddd_hair::RTri| { let mut k: Vec<[i64; 4]> = t.v.iter().map(|v| { let q = w0[v.bone()].point(v.pos().map(|x| x as f64)); [(q[0] * 1e4) as i64, (q[1] * 1e4) as i64, (q[2] * 1e4) as i64, v.bone() as i64] }).collect(); k.sort(); (k, t.tex) };
        let mut old: std::collections::HashMap<_, usize> = Default::default();
        if [185, 205, 206].contains(&grp) && std::env::var("DM_BEFORE").is_err() {
            let (m0, _) = gap_group_as(&g, grp, true);
            let q0 = crate::mdl::MdlPart::parse_bytes(&crate::mdl::split_parts(&m0).unwrap()[3]).unwrap();
            let mut o = crate::ddd_hair::tests::collect(&q0);
            if let Some(lb) = q0.list_b.as_ref() { o.extend(crate::ddd_hair::tests::extract_list(lb)); }
            for t in o { *old.entry(key(&t)).or_default() += 1; }
        }
        let hair_tex = p3.texture_names().iter().position(|n| n.starts_with("rk_hair")).map(|x| x as u32);
        let mut is_hair: Vec<bool> = vec![];
        for t in &t3 {
            is_hair.push(Some(t.tex) == hair_tex);
            let new = !old.is_empty() && match old.get_mut(&key(t)) { Some(n) if *n > 0 => { *n -= 1; false } _ => true };
            tris.push((t.v.map(|v| (v.bone(), v.pos().map(|x| x as f64))), t.v[2].q0[0] as i16, if new { 4 } else { 3 }));
        }
        eprintln!("{} new triangles", tris.iter().filter(|t| t.2 == 4).count());
        // the plain proxy (part 2) is a flat copy of the whole body at the same depth (a z-fight with part 3 in every pixel): left out
        // unless `GAP_PROXY` is set
        if std::env::var("GAP_PROXY").is_ok() {
            for t in crate::ddd_hair::tests::collect(&p2) { tris.push((t.v.map(|v| (v.bone(), v.pos().map(|x| x as f64))), 0, 2)); }
        }
        let out = std::path::PathBuf::from(format!("{}/local/work/darkholes/{grp}", crate::kh2_motion::tests::workspace()));
        std::fs::create_dir_all(&out).unwrap();
        const N: usize = 160;
        let half = 0.2;
        thread_local! { static DEEP: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) }; }
        // one view: (hole mask, proxy-visible mask, the A-nearest triangle per pixel)
        let view = |w: &[crate::math::Aff], yaw: f64, pitch: f64, sign: f64| -> (Vec<u8>, Vec<usize>) {
            let (sy, cy, sp, cp) = (yaw.to_radians().sin(), yaw.to_radians().cos(), pitch.to_radians().sin(), pitch.to_radians().cos());
            let tx = |q: [f64; 3]| { let (x, y, z) = (q[0] * cy + q[2] * sy, q[1], -q[0] * sy + q[2] * cy); [x, y * cp - z * sp, y * sp + z * cp] };
            let c = tx(w[head].point([0.0, 0.05, 0.0]));
            let mut za = vec![f64::MIN; N * N];
            let mut ia = vec![usize::MAX; N * N];
            let mut zb = vec![f64::MIN; N * N];
            let mut kb = vec![0u8; N * N];
            // the nearest drawn surface that is not new (kind 4)
            let mut zo = vec![f64::MIN; N * N];
            let mut ho = vec![false; N * N];
            let mut ib = vec![usize::MAX; N * N];
            let mut zao = vec![f64::MIN; N * N];
            for (k, (v, flag, kind)) in tris.iter().enumerate() {
                let p = v.map(|(b, q)| tx(w[b].point(q)));
                if p.iter().all(|q| (q[0] - c[0]).abs() > half || (q[1] - c[1]).abs() > half) { continue; }
                let s = p.map(|q| [(q[0] - c[0] + half) / (2.0 * half) * N as f64, (c[1] + half - q[1]) / (2.0 * half) * N as f64]);
                let cross = (s[1][0] - s[0][0]) * (s[2][1] - s[0][1]) - (s[1][1] - s[0][1]) * (s[2][0] - s[0][0]);
                if cross.abs() < 1e-9 { continue; }
                // the game: flag 0 drawn iff (a, b, c) is clockwise on the monitor (y down: cross > 0), 0x20 iff counter-clockwise
                let game = *kind == 2 || ((cross * sign > 0.0) == (*flag == 0));
                let x0 = s.iter().map(|q| q[0]).fold(f64::MAX, f64::min).floor().max(0.0) as usize;
                let x1 = (s.iter().map(|q| q[0]).fold(f64::MIN, f64::max).ceil().max(0.0) as usize).min(N - 1);
                let y0 = s.iter().map(|q| q[1]).fold(f64::MAX, f64::min).floor().max(0.0) as usize;
                let y1 = (s.iter().map(|q| q[1]).fold(f64::MIN, f64::max).ceil().max(0.0) as usize).min(N - 1);
                for y in y0..=y1 { for x in x0..=x1 {
                    let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
                    let l1 = ((s[2][0] - s[0][0]) * (py - s[0][1]) - (s[2][1] - s[0][1]) * (px - s[0][0])) / -cross;
                    let l2 = ((s[1][0] - s[0][0]) * (py - s[0][1]) - (s[1][1] - s[0][1]) * (px - s[0][0])) / cross;
                    let l0 = 1.0 - l1 - l2;
                    if l0 < 0.0 || l1 < 0.0 || l2 < 0.0 { continue; }
                    let z = l0 * p[0][2] + l1 * p[1][2] + l2 * p[2][2];
                    let i = y * N + x;
                    if z > za[i] { za[i] = z; ia[i] = k; }
                    if *kind != 4 && z > zao[i] { zao[i] = z; }
                    if game && z > zb[i] { zb[i] = z; kb[i] = *kind; ib[i] = k; }
                    if game && *kind != 4 && z > zo[i] { zo[i] = z; ho[i] = is_hair.get(k).copied().unwrap_or(false); }
                } }
            }
            // 0 nothing, 1 drawn (part 3), 2 proxy shows, 3 see-through (covered, the game draws nothing), 4 drawn but deeper than the
            // nearest surface by > 2 cm (seen through a hole onto a farther surface: the inside of the head or hair)
            // 5 the new triangles drawn more than 5 mm in front of an old surface that is not hair (the skirt over the neck or the
            // coat: a crossing),
            // 6 the new triangles where the old model showed the background (a hole filled)
            let mut m = vec![0u8; N * N];
            for i in 0..N * N {
                m[i] = if ia[i] == usize::MAX { 0 } else if zb[i] == f64::MIN { 3 } else if kb[i] == 4 && zo[i] == f64::MIN { 6 } else if kb[i] == 4 && zb[i] - zo[i] > 0.005 && !(zo[i] < zao[i] - 0.02) && !ho[i] { 5 } else if kb[i] == 4 && zb[i] - zo[i] > 0.005 && !(zo[i] < zao[i] - 0.02) && ho[i] { 7 } else if kb[i] == 2 { 2 } else if za[i] - zb[i] > 0.02 { 4 } else { 1 };
            }
            // where a new triangle is over an old surface, the culprit is the new one
            let mut deep = 0.0f64;
            for i in 0..N * N { if m[i] == 5 { ia[i] = ib[i]; deep = deep.max(zb[i] - zo[i]); } }
            DEEP.with(|d| d.set(d.get().max(deep)));
            (m, ia)
        };
        // the facing convention: with the right sign the drawn surface is the nearest one almost everywhere
        let rest = sk.rest_world();
        let score = |sign: f64| { let (m, _) = view(&rest, 0.0, 0.0, sign); m.iter().filter(|&&x| x == 1).count() };
        let (sp, sm) = (score(1.0), score(-1.0));
        let sign = if sp >= sm { 1.0 } else { -1.0 };
        eprintln!("group {grp}: facing sign {sign} (nearest-drawn pixels {sp} vs {sm})");
        // a defect pixel = see-through (3) or onto a farther surface (4) or the proxy (2), not on the coverage's 1-px rim
        let bad = |m: &[u8]| -> (usize, usize, usize) {
            let mut n = (0, 0, 0);
            for y in 1..N - 1 { for x in 1..N - 1 {
                let i = y * N + x;
                if m[i] < 2 || [i - 1, i + 1, i - N, i + N].iter().any(|&j| m[j] == 0) { continue; }
                match m[i] { 3 => n.0 += 1, 4 => n.1 += 1, 5 => n.2 += 1, _ => {} }
            } }
            n
        };
        let png = |m: &[u8], path: &std::path::Path| {
            let rgb: Vec<u8> = m.iter().flat_map(|&x| match x { 0 => [214, 212, 230], 1 => [70, 70, 90], 2 => [40, 200, 40], 3 => [255, 0, 255], 5 => [230, 0, 0], 6 => [0, 200, 90], _ => [255, 160, 0] }).collect();
            std::fs::write(path, crate::ddd_hair::tests::png(N, N, &rgb)).unwrap();
        };
        let step: usize = std::env::var("GAP_STEP").ok().and_then(|x| x.parse().ok()).unwrap_or(2);
        let slots: Vec<usize> = std::env::var("GAP_SLOTS").map(|s| s.split(',').map(|x| x.parse().unwrap()).collect()).unwrap_or_else(|_| vec![1, 4]);
        let mut culprits: std::collections::BTreeMap<String, usize> = Default::default();
        for s in slots {
            let Some(c) = ann.slots.get(s).and_then(|x| x.clip.as_ref()) else { continue };
            let (mut tot, mut worst) = ((0, 0, 0), (0usize, 0usize, 0.0, 0.0));
            let mut head_only = 0usize;
            let (mut filled, mut cover_worst) = (0usize, (0usize, 0usize, 0.0, 0.0));
            let (mut over_hair, mut over_hair_worst) = (0usize, (0usize, 0usize, 0.0, 0.0));
            let mut frames: Vec<usize> = (0..c.frames as usize).step_by(step).collect();
            frames.push(c.frames as usize - 1);
            for &f in &frames {
                let w = crate::kh2_hair::tests::posed_world(&sk, c, f as f64);
                for yi in 0..12 { for pi in 0..10 {
                    let (yaw, pitch) = (yi as f64 * 30.0, -60.0 + pi as f64 * 15.0);
                    let (m, ia) = view(&w, yaw, pitch, sign);
                    let b = bad(&m);
                    tot = (tot.0 + b.0, tot.1 + b.1, tot.2 + b.2);
                    filled += m.iter().filter(|&&x| x == 6).count();
                    let oh = m.iter().filter(|&&x| x == 7).count();
                    over_hair += oh;
                    if oh > over_hair_worst.0 { over_hair_worst = (oh, f, yaw, pitch); }
                    for i in 0..N * N {
                        if m[i] != 5 { continue; }
                        let (v, _, _) = tris[ia[i]];
                        let cen = [0, 1, 2].map(|k| (0..3).map(|j| rest[v[j].0].point(v[j].1)[k]).sum::<f64>() / 3.0 - rest[head].t[k]);
                        let ends: Vec<String> = v.iter().map(|x| { let q = rest[x.0].point(x.1); format!("{}({:+.1},{:+.1},{:+.1})", names[x.0], (q[0] - rest[head].t[0]) * 100.0, (q[1] - rest[head].t[1]) * 100.0, (q[2] - rest[head].t[2]) * 100.0) }).collect();
                        *culprits.entry(format!("CROSS {} c({:+.1},{:+.1},{:+.1})", ends.join(" "), cen[0] * 100.0, cen[1] * 100.0, cen[2] * 100.0)).or_default() += 1;
                    }
                    if b.2 > cover_worst.0 { cover_worst = (b.2, f, yaw, pitch); }
                    let mut n = 0;
                    for y in 1..N - 1 { for x in 1..N - 1 {
                        let i = y * N + x;
                        if !(m[i] == 3 || m[i] == 4) || [i - 1, i + 1, i - N, i + N].iter().any(|&j| m[j] == 0) { continue; }
                        let (v, _, _) = tris[ia[i]];
                        if !v.iter().any(|x| x.0 == head || names[x.0].contains("hair") || names[x.0].starts_with("neck")) { continue; }
                        n += 1;
                        // where: the culprit triangle's centroid in the head's rest frame, 2 cm cells
                        let cen = [0, 1, 2].map(|k| (0..3).map(|j| rest[v[j].0].point(v[j].1)[k]).sum::<f64>() / 3.0 - rest[head].t[k]);
                        let cell = format!("{}/{}/{} x{:+.2} y{:+.2} z{:+.2} {}", names[v[0].0], names[v[1].0], names[v[2].0], (cen[0] * 50.0).round() / 50.0, (cen[1] * 50.0).round() / 50.0, (cen[2] * 50.0).round() / 50.0, if m[i] == 3 { "bg" } else { "far" });
                        *culprits.entry(cell).or_default() += 1;
                    }}
                    head_only += n;
                    if n > worst.0 { worst = (n, f, yaw, pitch); }
                }}
            }
            eprintln!("slot {s} {}: frames {} | see-through {} onto-farther {} | on head / hair / neck {head_only} | worst {} px at f{} yaw {} pitch {} | new over old surfaces {} (worst {} px f{} y{} p{}), holes filled {filled}", c.name_str(), frames.len(), tot.0, tot.1, worst.0, worst.1, worst.2, worst.3, tot.2, cover_worst.0, cover_worst.1, cover_worst.2, cover_worst.3);
            eprintln!("  new over the hair {over_hair} (worst {} px f{} y{} p{})", over_hair_worst.0, over_hair_worst.1, over_hair_worst.2, over_hair_worst.3);
            eprintln!("  deepest: new in front of an old surface by {:.1} cm", DEEP.with(|d| d.replace(0.0)) * 100.0);
            if cover_worst.0 > 0 {
                let w = crate::kh2_hair::tests::posed_world(&sk, c, cover_worst.1 as f64);
                let (m, _) = view(&w, cover_worst.2, cover_worst.3, sign);
                png(&m, &out.join(format!("cover_{}_f{}_y{}_p{}.png", c.name_str(), cover_worst.1, cover_worst.2, cover_worst.3)));
            }
            if worst.0 > 0 {
                let w = crate::kh2_hair::tests::posed_world(&sk, c, worst.1 as f64);
                let (m, _) = view(&w, worst.2, worst.3, sign);
                png(&m, &out.join(format!("{}_f{}_y{}_p{}.png", c.name_str(), worst.1, worst.2, worst.3)));
            }
        }
        let mut cv: Vec<_> = culprits.into_iter().collect();
        cv.sort_by_key(|x| std::cmp::Reverse(x.1));
        for (k, n) in cv.iter().take(40) { eprintln!("  {n:6} {k}"); }
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









#[cfg(test)]
mod skirt_probe {
    use super::*;
    use crate::install;
    /// 0.198.8 probe (fork owner: "no darkmode queria ver se é possível animar a saia do riku fica muito estática", "tem como melhorar
    /// ainda mais a animação da saia?"): how far the coat-tail bones (`*_skt*`) turn off their rest pose, in Dark Riku's own clips
    /// (`PL0201.ANN`) and in the Dark Form skin's (Riku's motions retargeted, group 121), per clip (degrees, the largest over the bones).
    #[test]
    fn skirt_motion_probe() {
        let Ok(g) = std::env::var("RECOM_GAME").map(std::path::PathBuf::from) else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let dmdl = install::read_recom(&arc, &keys, "PLAYER/0201/PL0201.MDL").unwrap();
        let ds = RecomSkeleton::parse(&dmdl).unwrap();
        let own = Ann::parse(&install::read_recom(&arc, &keys, "PLAYER/0201/PL0201.ANN").unwrap()).unwrap();
        let skin = build_with(&arc, &keys, 101, 201, 121, None).unwrap();
        let sann = Ann::parse(&skin.files.iter().find(|f| f.0 == path(121, "ANN")).unwrap().1).unwrap();
        let riku_ann = Ann::parse(&install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.ANN").unwrap()).unwrap();
        let nm = |a: &Ann, i: usize| a.slots.get(i).map(|s| format!("{}{}", String::from_utf8_lossy(&s.name).trim_end_matches('\0'), s.clip.as_ref().map_or(String::new(), |c| format!(" {}f{}", c.frames, if c.looped() { " loop" } else { "" })))).unwrap_or_default();
        for i in 0..sann.slots.len().max(own.slots.len()) {
            eprintln!("SLOT {i}: riku [{}] skin [{}] dark [{}]", nm(&riku_ann, i), nm(&sann, i), nm(&own, i));
        }
        let skt: Vec<usize> = ds.bones.iter().enumerate().filter(|(_, b)| b.name.contains("_skt")).map(|(i, _)| i).collect();
        eprintln!("{} coat-tail bones", skt.len());
        let rest = ds.rest_world();
        let turn = |a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]| { let d = crate::math::m3(&crate::math::tr(a), b); ((d[0][0] + d[1][1] + d[2][2] - 1.0) / 2.0).clamp(-1.0, 1.0).acos().to_degrees() };
        // 0.198.8: the held jump's seams (jump → apex → fall → land): the tails' largest turn between one clip's last frame and the
        // next's first, ours (Riku's slots 4 / 5 / 6 / 7) against Dark Riku's own (his 4 / 6 / 7 / 8)
        let seam = |a: &Ann, x: usize, y: usize| -> f64 {
            let (Some(ca), Some(cb)) = (a.slots.get(x).and_then(|s| s.clip.as_ref()), a.slots.get(y).and_then(|s| s.clip.as_ref())) else { return f64::NAN };
            let (wa, wb) = (crate::kh2_hair::tests::posed_world(&ds, ca, (ca.frames - 1) as f64), crate::kh2_hair::tests::posed_world(&ds, cb, 0.0));
            skt.iter().map(|&b| {
                let p = ds.bones[b].parent as usize;
                let ra = crate::math::m3(&crate::math::unscale(&wa[b].r).0, &crate::math::tr(&crate::math::unscale(&wa[p].r).0));
                let rb = crate::math::m3(&crate::math::unscale(&wb[b].r).0, &crate::math::tr(&crate::math::unscale(&wb[p].r).0));
                turn(&ra, &rb)
            }).fold(0.0, f64::max)
        };
        for ((x, y), (dx, dy)) in [((4, 5), (4, 6)), ((5, 6), (6, 7)), ((6, 7), (7, 8)), ((4, 6), (4, 7))] {
            eprintln!("SEAM ours {x}->{y}: {:.1} deg; Dark Riku's {dx}->{dy}: {:.1} deg", seam(&sann, x, y), seam(&own, dx, dy));
        }
        for (name, ann) in [("Dark Riku's own", &own), ("Dark Form skin", &sann)] {
            let mut rows = vec![];
            for (slot, s) in ann.slots.iter().enumerate() {
                let Some(c) = &s.clip else { continue };
                if c.bones as usize != ds.bones.len() { continue; }
                let mut worst: f64 = 0.0;
                let mut spread: f64 = 0.0;
                let mut prev: Option<Vec<[[f64; 3]; 3]>> = None;
                for f in 0..c.frames.max(1) as usize {
                    let w = crate::kh2_hair::tests::posed_world(&ds, c, f as f64);
                    // each tail bone relative to its parent, against the same at rest
                    let rel: Vec<[[f64; 3]; 3]> = skt.iter().map(|&b| { let p = ds.bones[b].parent as usize; crate::math::m3(&crate::math::unscale(&w[b].r).0, &crate::math::tr(&crate::math::unscale(&w[p].r).0)) }).collect();
                    let rrel: Vec<[[f64; 3]; 3]> = skt.iter().map(|&b| { let p = ds.bones[b].parent as usize; crate::math::m3(&crate::math::unscale(&rest[b].r).0, &crate::math::tr(&crate::math::unscale(&rest[p].r).0)) }).collect();
                    for (a, r) in rel.iter().zip(&rrel) { worst = worst.max(turn(r, a)); }
                    if let Some(p) = &prev { for (a, b) in rel.iter().zip(p) { spread = spread.max(turn(b, a)); } }
                    prev = Some(rel);
                }
                // 0.198.8 review: the tails' jump where a looped clip goes back (last frame → frame loop_start − 1)
                let mut wrap: f64 = 0.0;
                if c.looped() {
                    let l = (c.loop_start_clamped() as usize).saturating_sub(1) as f64;
                    let (wa, wb) = (crate::kh2_hair::tests::posed_world(&ds, c, (c.frames - 1) as f64), crate::kh2_hair::tests::posed_world(&ds, c, l));
                    for &b in &skt {
                        let p = ds.bones[b].parent as usize;
                        let ra = crate::math::m3(&crate::math::unscale(&wa[b].r).0, &crate::math::tr(&crate::math::unscale(&wa[p].r).0));
                        let rb = crate::math::m3(&crate::math::unscale(&wb[b].r).0, &crate::math::tr(&crate::math::unscale(&wb[p].r).0));
                        wrap = wrap.max(turn(&ra, &rb));
                    }
                    if name == "Dark Form skin" {
                        assert!(wrap <= (1.5 * spread).max(5.0), "the skin's slot {slot}: the tails jump {wrap:.1} deg at the loop (largest step {spread:.1})");
                    }
                }
                rows.push(format!("slot {slot} ({} f, loop {}): off rest {worst:.1}, largest frame step {spread:.2}, wrap {wrap:.1}", c.frames, c.loop_start));
                // 0.198.8: the skin's tails move (Dark Riku's run copied; the first swing simulated)
                if name == "Dark Form skin" && (slot == 3 || slot == 44) {
                    assert!(worst > 10.0, "the skin's slot {slot}: the coat tails turn only {worst:.1} deg off rest");
                }
            }
            eprintln!("== {name}: {} clips", rows.len());
            for r in rows.iter() { eprintln!("  {r}"); }
        }
    }
}

#[cfg(test)]
mod grip_probe {
    use super::*;
    use crate::install;
    /// 0.198.9 probe (fork owner: "melhorar o encaixe da espada na mão do Riku tanto darkmode quanto skin default, acho que tem muita
    /// folga na mão"): where the Soul Eater's grip (`wep0`) sits against the right hand, per clip, in Re:CoM's own pack and in the
    /// mod's (the installed group's), for Riku (`PL0101`) and the Dark Form skin (121).
    /// 0.198.9: [`crate::grip::close`] on Riku's (`PL0101`) and Dark Riku's (`PL0201`) own packs: the turns found and the finger radii.
    /// 0.198.9: the handle's real thickness where the hand holds it: the weapon's radius around `wep0`'s x axis in 5 mm slices, the
    /// median (not the largest: the guard and the chain parts are there too), and where along x the palm is.
    /// 0.198.9: which of Riku's own clips (Re:CoM's `PL0101.ANN`, and his other packs `PL1xxx` that play on his skeleton) hold the
    /// handle with the hand closed: per clip, the thumb tip's distance to the index's middle / tip segments (cm, the "C" opening) at the
    /// clip's smallest frame, and the fingers' mean tip distance to the handle's axis.
    #[test]
    fn closed_grip_search() {
        let Ok(g) = std::env::var("RECOM_GAME").map(std::path::PathBuf::from) else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let mdl = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        let sk = RecomSkeleton::parse(&mdl).unwrap();
        let p3 = crate::mdl::MdlPart::parse_bytes(&crate::mdl::split_parts(&mdl).unwrap()[3]).unwrap();
        let verts: Vec<(usize, [f64; 3])> = p3.list_a.iter().flat_map(|k| k.verts.iter()).filter(|v| sk.bones[v.bone()].name.starts_with("r_fing")).map(|v| (v.bone(), v.pos().map(|x| x as f64))).collect();
        let bn = |n: &str| sk.find(n).unwrap();
        let (t1a, i2b, i2a) = (bn("r_fing1a"), bn("r_fing2b"), bn("r_fing2a"));
        let mut found: Vec<(f64, String, usize, f64)> = vec![];
        let mut packs: Vec<(String, Vec<u8>)> = vec![("PL0101".into(), install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.ANN").unwrap())];
        for g2 in 1000..1400u16 {
            let n = format!("PLAYER/{g2:04}/PL{g2:04}.ANN");
            if arc.find(&n).is_some() { if let Ok(b) = install::read_recom(&arc, &keys, &n) { packs.push((format!("PL{g2:04}"), b)); } }
        }
        for (pn, b) in &packs {
            let Ok(ann) = Ann::parse(b) else { continue };
            for (slot, s) in ann.slots.iter().enumerate() {
                let Some(c) = &s.clip else { continue };
                if c.bones as usize != sk.bones.len() { continue; }
                let mut best = (f64::MAX, 0.0);
                for f in 0..c.frames.max(1) as usize {
                    let w = crate::kh2_hair::tests::posed_world(&sk, c, f as f64);
                    let tip: Vec<[f64; 3]> = verts.iter().filter(|(b, _)| *b == t1a).map(|(b, p)| w[*b].point(*p)).collect();
                    let idx: Vec<[f64; 3]> = verts.iter().filter(|(b, _)| *b == i2b || *b == i2a).map(|(b, p)| w[*b].point(*p)).collect();
                    let d = tip.iter().map(|a| idx.iter().map(|b| crate::math::dist(*a, *b)).fold(f64::MAX, f64::min)).fold(f64::MAX, f64::min);
                    if d < best.0 { best = (d, f as f64); }
                }
                found.push((best.0, format!("{pn} {}", c.name_str()), slot, best.1));
            }
        }
        found.sort_by(|a, b| a.0.total_cmp(&b.0));
        eprintln!("{} clips on Riku's skeleton", found.len());
        for (d, n, slot, f) in found.iter().take(15) { eprintln!("  {n} (slot {slot}) frame {f}: thumb tip to index {:.1} cm", d * 100.0); }
        if let Some(x) = found.iter().find(|x| x.1.ends_with("p002_031")) { eprintln!("  the battle stance p002_031: {:.1} cm", x.0 * 100.0); }
    }

    #[test]
    fn handle_profile() {
        let Ok(g) = std::env::var("RECOM_GAME").map(std::path::PathBuf::from) else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        for grp in ["0101", "0201"] {
            let mdl = install::read_recom(&arc, &keys, &format!("PLAYER/{grp}/PL{grp}.MDL")).unwrap();
            let sk = RecomSkeleton::parse(&mdl).unwrap();
            let w0 = sk.find("wep0").unwrap();
            let parts = crate::mdl::split_parts(&mdl).unwrap();
            let p1 = crate::mdl::MdlPart::parse_bytes(&parts[1]).unwrap();
            let wv: Vec<[f64; 3]> = p1.list_a.iter().flat_map(|k| k.verts.iter()).filter(|v| v.bone() == w0).map(|v| v.pos().map(|x| x as f64)).collect();
            let mut bins: std::collections::BTreeMap<i64, Vec<f64>> = Default::default();
            for p in &wv { if p[0].abs() < 0.12 { bins.entry((p[0] * 200.0).floor() as i64).or_default().push((p[1] * p[1] + p[2] * p[2]).sqrt()); } }
            // the palm: r_hand's origin and the finger bases in wep0's frame, idle frame 0
            let ann = Ann::parse(&install::read_recom(&arc, &keys, &format!("PLAYER/{grp}/PL{grp}.ANN")).unwrap()).unwrap();
            let c = ann.slots[1].clip.as_ref().unwrap();
            let w = crate::kh2_hair::tests::posed_world(&sk, c, 0.0);
            let inv = crate::kh2_hair_rig::rigid_inv(&crate::math::Aff { r: crate::math::unscale(&w[w0].r).0, t: w[w0].t });
            let at = |n: &str| sk.find(n).map(|b| inv.point(w[b].t).map(|x| (x * 1000.0).round() / 10.0));
            eprintln!("{grp}: r_hand {:?}, fing2c {:?}, fing3c {:?}, fing5c {:?}, fing3a {:?}", at("r_hand"), at("r_fing2c"), at("r_fing3c"), at("r_fing5c"), at("r_fing3a"));
            for (b, mut v) in bins { v.sort_by(|a, b| a.total_cmp(b)); eprintln!("  x {:+.1} cm: n {:3}, radius median {:.2} cm, max {:.2}", b as f64 / 2.0, v.len(), v[v.len() / 2] * 100.0, v[v.len() - 1] * 100.0); }
        }
    }

    /// 0.198.9 (third review): the Dark Form skin's pack as `build_with` makes it, against the same without the grip (`NO_GRIP`): no
    /// frame wider than its own by more than `MAX_WIDER`, the fingers no faster than their own + 2 deg/frame.
    #[test]
    fn skin_grip_bounds() {
        let Ok(g) = std::env::var("RECOM_GAME").map(std::path::PathBuf::from) else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        // 0.198.10: the grip alone (`crate::hands::OFF` in both: the relax's own speeds are `hands::tests`')
        let pack = |no: bool| { crate::grip::OFF.with(|x| x.set(no)); crate::hands::OFF.with(|x| x.set(true)); let b = build_with(&arc, &keys, 101, 201, 121, None).unwrap(); crate::grip::OFF.with(|x| x.set(false)); crate::hands::OFF.with(|x| x.set(false)); Ann::parse(&b.files.iter().find(|f| f.0 == path(121, "ANN")).unwrap().1).unwrap() };
        let (before, after) = (pack(true), pack(false));
        let mdl = install::read_recom(&arc, &keys, "PLAYER/0201/PL0201.MDL").unwrap();
        let sk = RecomSkeleton::parse(&mdl).unwrap();
        let fb: Vec<usize> = (1..=5).flat_map(|n| ["c", "b", "a"].map(|x| sk.find(&format!("r_fing{n}{x}")))).flatten().collect();
        let rot = |c: &crate::ann::Clip, b: usize, f: usize| { let r = c.sample(b, f as f64).r; rot_xyz(r[0], r[1], r[2]) };
        let ang = |a: &M3, b: &M3| { let d = m3(&tr(a), b); ((d[0][0] + d[1][1] + d[2][2] - 1.0) / 2.0).clamp(-1.0, 1.0).acos().to_degrees() };
        for (slot, (sa, sb)) in after.slots.iter().zip(&before.slots).enumerate() {
            let (Some(a), Some(b)) = (sa.clip.as_ref(), sb.clip.as_ref()) else { continue };
            if a.bones as usize != sk.bones.len() { continue; }
            let n = a.frames.max(1) as usize;
            let speed = |c: &crate::ann::Clip| (1..n).map(|f| fb.iter().map(|&k| ang(&rot(c, k, f - 1), &rot(c, k, f))).fold(0.0, f64::max)).fold(0.0, f64::max);
            let (va, vb) = (speed(a), speed(b));
            if std::env::var("GRIP_DEBUG").is_ok() { eprintln!("SKIN {slot} {}: {va:.1} vs {vb:.1}", a.name_str()); }
            assert!(va <= vb + 2.0, "skin slot {slot} {}: fingers {va:.1} deg/frame against their own {vb:.1}", a.name_str());
        }
    }

    #[test]
    fn grip_close_report() {
        let Ok(g) = std::env::var("RECOM_GAME").map(std::path::PathBuf::from) else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        for grp in ["0101", "0201"] {
            let mdl = install::read_recom(&arc, &keys, &format!("PLAYER/{grp}/PL{grp}.MDL")).unwrap();
            {
                let sk = RecomSkeleton::parse(&mdl).unwrap();
                let w = sk.rest_world();
                let h = sk.find("r_hand").unwrap();
                for (i, b) in sk.bones.iter().enumerate() {
                    if b.name.starts_with("r_fing") || b.name == "r_hand" || b.name == "wep0" || b.name == "k11" || b.name == "k10" || b.name == "wep1" || b.name == "wep2" {
                        let lt = b.local.t;
                        eprintln!("BONE {grp} {i} {} parent {} ({}), local t ({:.3},{:.3},{:.3}), world from hand ({:.3},{:.3},{:.3})", b.name, b.parent, sk.bones[b.parent.max(0) as usize].name, lt[0], lt[1], lt[2], w[i].t[0] - w[h].t[0], w[i].t[1] - w[h].t[1], w[i].t[2] - w[h].t[2]);
                    }
                }
            }
            let mut ann = Ann::parse(&install::read_recom(&arc, &keys, &format!("PLAYER/{grp}/PL{grp}.ANN")).unwrap()).unwrap();
            let t = std::time::Instant::now();
            let own = ann.clone();
            let rep = crate::grip::close(&mut ann, &mdl, &own).unwrap();
            eprintln!("{grp}: grip from {:?}, {} clips changed, max weight {:.2}, {:?}", rep.source, rep.clips, rep.max_weight, t.elapsed());
            assert!(rep.worst_wider <= crate::grip::MAX_WIDER + 1e-4, "{grp}: a frame opens {:.1} mm wider than the clip's own", rep.worst_wider * 1000.0);
            // one weight per clip: the fingers' turn off the clip's own changes little from frame to frame
            for (slot, s) in ann.slots.iter().enumerate() {
                let (Some(a), Some(b)) = (s.clip.as_ref(), own.slots.get(slot).and_then(|x| x.clip.as_ref())) else { continue };
                let sk = RecomSkeleton::parse(&mdl).unwrap();
                let fb: Vec<usize> = (1..=5).flat_map(|n| ["c", "b", "a"].map(|x| sk.find(&format!("r_fing{n}{x}")))).flatten().collect();
                let turn = |f: usize| fb.iter().map(|&k| { let (x, y) = (a.sample(k, f as f64).r, b.sample(k, f as f64).r); let d = m3(&tr(&rot_xyz(y[0], y[1], y[2])), &rot_xyz(x[0], x[1], x[2])); ((d[0][0] + d[1][1] + d[2][2] - 1.0) / 2.0).clamp(-1.0, 1.0).acos().to_degrees() }).fold(0.0, f64::max);
                let ts: Vec<f64> = (0..a.frames.max(1) as usize).map(turn).collect();
                let pump = ts.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0, f64::max);
                if pump > 12.0 { eprintln!("  {} slot {slot}: the grip's turn changes {pump:.1} deg in a frame ({:?})", a.name_str(), ts.iter().map(|x| x.round() as i64).collect::<Vec<_>>()); }
            }
        }
    }

    #[test]
    fn grip_probe() {
        let Ok(g) = std::env::var("RECOM_GAME").map(std::path::PathBuf::from) else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let home = std::env::var("HOME").unwrap();
        for (label, mdl_path, own_ann, mod_ann) in [
            ("Riku 101", "PLAYER/0101/PL0101.MDL", "PLAYER/0101/PL0101.ANN", format!("{home}/OpenKH/openkh/mod/Recom/PLAYER/0101/PL0101.ANN")),
            ("Dark Riku 201", "PLAYER/0201/PL0201.MDL", "PLAYER/0201/PL0201.ANN", format!("{home}/OpenKH/openkh/mod/Recom/PLAYER/0121/PL0121.ANN")),
        ] {
            let mdl = install::read_recom(&arc, &keys, mdl_path).unwrap();
            let sk = RecomSkeleton::parse(&mdl).unwrap();
            let w0 = sk.find("wep0").unwrap();
            let hand = sk.find("r_hand").unwrap();
            let chain: Vec<String> = { let mut v = vec![]; let mut b = w0 as i16; while b >= 0 { v.push(sk.bones[b as usize].name.clone()); b = sk.bones[b as usize].parent; } v };
            eprintln!("{label}: wep0 = {w0}, chain {chain:?}; r_hand = {hand}");
            // the hand's mesh: part 3 vertices on r_hand / r_fing*
            let p3 = crate::mdl::MdlPart::parse_bytes(&crate::mdl::split_parts(&mdl).unwrap()[3]).unwrap();
            let hv: Vec<(usize, [f32; 3])> = p3.list_a.iter().flat_map(|k| k.verts.iter()).filter(|v| { let n = &sk.bones[v.bone()].name; n == "r_hand" || n.starts_with("r_fing") }).map(|v| (v.bone(), v.pos())).collect();
            let pw = crate::mdl::MdlPart::parse_bytes(&crate::mdl::split_parts(&mdl).unwrap()[1]).unwrap();
            let wv: Vec<[f32; 3]> = pw.list_a.iter().flat_map(|k| k.verts.iter()).filter(|v| v.bone() == w0).map(|v| v.pos()).collect();
            let xs: Vec<f32> = wv.iter().map(|p| p[0]).collect();
            eprintln!("  weapon: {} vertices on wep0, local x {:.3}..{:.3}", wv.len(), xs.iter().cloned().fold(f32::MAX, f32::min), xs.iter().cloned().fold(f32::MIN, f32::max));
            for (which, ann_bytes) in [("Re:CoM's", install::read_recom(&arc, &keys, own_ann).ok()), ("mod's", std::fs::read(&mod_ann).ok())] {
                let Some(b) = ann_bytes else { eprintln!("  {which}: not read"); continue };
                let ann = Ann::parse(&b).unwrap();
                for slot in [1usize, 2, 3, 31, 44, 45, 46, 47] {
                    let Some(c) = ann.slots.get(slot).and_then(|s| s.clip.as_ref()) else { continue };
                    if c.bones as usize != sk.bones.len() { continue; }
                    let mut gaps = vec![];
                    for f in [0.0, (c.frames / 2) as f64] {
                        let w = crate::kh2_hair::tests::posed_world(&sk, c, f);
                        // the grip: the handle's vertices nearest wep0's origin (|x| < 6 cm); their mean point and the hand's nearest vertex to it
                        let grip: Vec<[f64; 3]> = wv.iter().filter(|p| p[0].abs() < 0.06).map(|p| w[w0].point(p.map(|x| x as f64))).collect();
                        let gc = [0, 1, 2].map(|i| grip.iter().map(|p| p[i]).sum::<f64>() / grip.len().max(1) as f64);
                        let hand_pts: Vec<[f64; 3]> = hv.iter().map(|(b, p)| w[*b].point(p.map(|x| x as f64))).collect();
                        let near = hand_pts.iter().map(|p| crate::math::dist(*p, gc)).fold(f64::MAX, f64::min);
                        let hand_c = [0, 1, 2].map(|i| hand_pts.iter().map(|p| p[i]).sum::<f64>() / hand_pts.len().max(1) as f64);
                        // the handle's radius and the hand's vertices around it, in wep0's frame (the blade along x)
                        let inv = crate::kh2_hair_rig::rigid_inv(&crate::math::Aff { r: crate::math::unscale(&w[w0].r).0, t: w[w0].t });
                        let rh = { let v: Vec<f64> = wv.iter().filter(|p| p[0].abs() < 0.06).map(|p| ((p[1] * p[1] + p[2] * p[2]) as f64).sqrt()).collect(); v.iter().sum::<f64>() / v.len().max(1) as f64 };
                        let mut by: std::collections::BTreeMap<String, (f64, usize)> = Default::default();
                        for (b, p) in &hv {
                            let q = inv.point(w[*b].point(p.map(|x| x as f64)));
                            if q[0].abs() < 0.06 {
                                let r = (q[1] * q[1] + q[2] * q[2]).sqrt();
                                let e = by.entry(sk.bones[*b].name.clone()).or_insert((f64::MAX, 0));
                                e.0 = e.0.min(r);
                                e.1 += 1;
                            }
                        }
                        if f == 0.0 && slot == 1 { eprintln!("    handle radius {:.1} cm; nearest radius of each hand bone's vertices around it: {}", rh * 100.0, by.iter().map(|(n, (r, k))| format!("{n} {:.1} cm ({k})", r * 100.0)).collect::<Vec<_>>().join(", ")); }
                        gaps.push(format!("f{f}: grip→nearest hand vertex {:.1} cm, grip→hand centre {:.1} cm, wep0 local t {:?}", near * 100.0, crate::math::dist(gc, hand_c) * 100.0, c.sample(w0, f).t.map(|x| (x * 1000.0).round() / 10.0)));
                    }
                    eprintln!("  {which} slot {slot} {}: {}", c.name_str(), gaps.join(" | "));
                }
            }
        }
    }
}

#[cfg(test)]
mod cast_ends {
    use super::*;
    use crate::install;
    fn clip_of(a: &Ann, n: &str) -> Clip {
        a.slots.iter().find_map(|s| s.clip.as_ref().filter(|c| c.name_str() == n)).cloned().unwrap_or_else(|| panic!("no {n}"))
    }
    /// The largest offset (m) of head, chest, hands, feet and blade root (each relative to the body bone `body`) between `a` at
    /// frame `fa` and `b` at `fb`, and its bone.
    fn gap(sk: &RecomSkeleton, a: &Clip, fa: f64, b: &Clip, fb: f64, body: &str) -> (f64, &'static str) {
        let (wa, wb) = (crate::kh2_hair::tests::posed_world(sk, a, fa), crate::kh2_hair::tests::posed_world(sk, b, fb));
        let k = sk.find(body).unwrap();
        let mut best = (0.0, "");
        for n in ["head", "chest1", "l_hand", "r_hand", "l_foot", "r_foot", "wep0"] {
            let Some(i) = sk.find(n) else { continue };
            let d = (0..3).map(|j| ((wa[i].t[j] - wa[k].t[j]) - (wb[i].t[j] - wb[k].t[j])).powi(2)).sum::<f64>().sqrt();
            if d > best.0 {
                best = (d, n);
            }
        }
        best
    }
    /// The fastest move (m a frame) of head, chest, hands, blade root and arms from frame `from` on.
    fn fastest(sk: &RecomSkeleton, c: &Clip, from: usize) -> f64 {
        let idx: Vec<usize> = ["chest1", "head", "l_hand", "r_hand", "wep0", "l_uparm", "r_uparm", "l_arm", "r_arm"].iter().filter_map(|n| sk.find(n)).collect();
        let mut w = 0.0f64;
        for f in from..(c.frames as usize).saturating_sub(1) {
            let (wa, wb) = (crate::kh2_hair::tests::posed_world(sk, c, f as f64), crate::kh2_hair::tests::posed_world(sk, c, f as f64 + 1.0));
            for &i in &idx {
                w = w.max(crate::math::dist(wa[i].t, wb[i].t));
            }
        }
        w
    }

    /// A bone's local rotation at frame `f`.
    fn local(c: &Clip, b: usize, f: f64) -> M3 {
        let s = c.sample(b, f);
        rot_xyz(s.r[0], s.r[1], s.r[2])
    }
    /// The angle (deg) between two rotations.
    fn turn(a: &M3, b: &M3) -> f64 {
        let r = m3(&tr(a), b);
        ((r[0][0] + r[1][1] + r[2][2] - 1.0) / 2.0).clamp(-1.0, 1.0).acos().to_degrees()
    }
    /// The frame checks of a cast `c` into `next`: the largest one-frame turn of any bone (deg, its bone and where) over the cast's
    /// last 12 frames and the step from its last frame into `next` frame 0; `next`'s own over its first 12 frames; the cast's last
    /// frame into `next`'s first 12 through the DLL's blend (`riku::OUT`, 10 frames: Re:CoM's controller blends the held last pose into `next` by the remaining share,
    /// Assumption: per bone in its local rotation); and the largest scale difference between the last frame and `next` frame 0.
    fn junction(sk: &RecomSkeleton, c: &Clip, next: &Clip, skip: &dyn Fn(&str) -> bool) -> ((f64, String), (f64, String), (f64, String), f64) {
        let nb = (c.bones.min(next.bones)) as usize;
        let last = (c.frames - 1) as f64;
        let mut seq: Vec<(String, Box<dyn Fn(usize) -> M3 + '_>)> = Vec::new();
        for f in (c.frames as usize).saturating_sub(12)..c.frames as usize {
            seq.push((format!("cast {f}"), Box::new(move |b| local(c, b, f as f64))));
        }
        seq.push(("next 0".into(), Box::new(move |b| local(next, b, 0.0))));
        let mut own: Vec<(String, Box<dyn Fn(usize) -> M3 + '_>)> = Vec::new();
        for f in 0..12 {
            own.push((format!("next {f}"), Box::new(move |b| local(next, b, f as f64))));
        }
        let mut blended: Vec<(String, Box<dyn Fn(usize) -> M3 + '_>)> = vec![(format!("cast {last}"), Box::new(move |b| local(c, b, last)))];
        for f in 0..12usize {
            let w = (f as f64 / 10.0).min(1.0);
            blended.push((format!("blend {f}"), Box::new(move |b| rot_blend(&local(c, b, last), &local(next, b, f as f64), w))));
        }
        let worst = |s: &[(String, Box<dyn Fn(usize) -> M3 + '_>)]| {
            let mut best = (0.0, String::new());
            for k in 1..s.len() {
                for b in (0..nb).filter(|&b| !skip(&sk.bones[b].name)) {
                    let d = turn(&(s[k - 1].1)(b), &(s[k].1)(b));
                    if d > best.0 {
                        best = (d, format!("{} {} -> {}", sk.bones[b].name, s[k - 1].0, s[k].0));
                    }
                }
            }
            best
        };
        let sc = (0..nb).map(|b| (0..3).map(|i| (c.sample(b, last).s[i] - next.sample(b, 0.0).s[i]).abs()).fold(0.0, f64::max)).fold(0.0, f64::max);
        (worst(&seq), worst(&blended), worst(&own), sc)
    }

    /// 0.198.13 (owner's install, `RECOM_GAME`; fork owner: "tem mudanças muito bruscas entre a animação e voltar para o estado
    /// idle"): every cast clip but the wind-ups, for Riku (`PL0101`), Dark Riku (`PL0201`) and the Dark Form skin (121), as built
    /// (grip, hands and tails included), ends within 2 cm of the clip the game plays next (the battle stance on the ground, the
    /// fall in the air; Riku's rows for Riku and the skin, Dark Riku's for him); the ease leaves the frames up to the key moment's
    /// end as they were and moves no bone faster than 0.2 m a frame more than the clip did.
    #[test]
    fn casts_end_in_the_next_pose() {
        let Ok(g) = std::env::var("RECOM_GAME").map(std::path::PathBuf::from) else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let rd = |p: &str| install::read_recom(&arc, &keys, p).unwrap();
        let ds = RecomSkeleton::parse(&rd("PLAYER/0201/PL0201.MDL")).unwrap();
        let rs = RecomSkeleton::parse(&rd("PLAYER/0101/PL0101.MDL")).unwrap();
        let ss = RecomSkeleton::parse(&rd("PLAYER/0001/PL0001.MDL")).unwrap();
        let ann = Ann::parse(&rd("PLAYER/0101/PL0101.ANN")).unwrap();
        let dann = Ann::parse(&rd("PLAYER/0201/PL0201.ANN")).unwrap();
        let packs: std::cell::RefCell<std::collections::HashMap<u16, Ann>> = Default::default();
        let get = |g: u16, name: &str| -> Result<Clip> {
            let mut p = packs.borrow_mut();
            let a = p.entry(g).or_insert_with(|| Ann::parse(&rd(&path(g, "ANN"))).unwrap());
            Ok(a.slots.iter().find_map(|s| s.clip.as_ref().filter(|c| c.name_str() == name)).cloned().unwrap())
        };
        let k1 = install::kh1(&g, keys.clone()).unwrap();
        let (am, ad) = (k1.read("xa_ex_1580.mset").unwrap(), k1.read("xa_ex_1580.mdls").unwrap());
        let (rm, rdl) = (k1.read("xa_ex_1010.mset").unwrap(), k1.read("xa_ex_1010.mdls").unwrap());
        let stance = clip_of(&ann, RIKU_STANCE);
        let terra_data = crate::terra::read_terra(&g, &keys).ok();
        let terra = terra_data.as_ref().map(|(k, c)| (k, c.as_slice()));
        let ctx = CastCtx { get: &get, riku: &rs, dark: &ds, sora: &ss, stance: Some(&stance), ansem: Some((&am, &ad)), terra };
        // The ease alone: untouched up to the key moment's end, the next pose at the end.
        let raw = dark_cast_clips(&ctx).unwrap();
        let eased = ease_cast_ends(&raw, &clip_of(&dann, DARK_STANCE), &clip_of(&dann, DARK_FALL));
        for (k, c) in CAST_CLIPS.iter().enumerate() {
            let same = (0..(c.key_out().floor() as usize).min(c.len)).map(|f| gap(&ds, &raw[k], f as f64, &eased[k], f as f64, "ex_1580_sk").0).fold(0.0, f64::max);
            assert!(same < 1e-3, "{}: the ease moved the key moment {:.1} cm", c.suffix, same * 100.0);
            let (a, b) = (fastest(&ds, &raw[k], 0), fastest(&ds, &eased[k], 0));
            assert!(b <= a + 0.2, "{}: {b:.3} m a frame (was {a:.3})", c.suffix);
        }
        // Clearance: the blade (points along `wep0`'s -x, 10..70 cm, where Riku's mesh runs) and the hands come no nearer the head
        // and chest (bone origins) in the ease than in the clip before it or in the next pose (Riku's own casts, his stance and fall).
        let rraw = riku_cast_clips(&ctx).unwrap();
        let reased = ease_cast_ends(&rraw, &stance, &clip_of(&ann, RIKU_FALL));
        let near = |c: &Clip, f: f64| -> f64 {
            let w = crate::kh2_hair::tests::posed_world(&rs, c, f);
            let (h, ch, wp) = (rs.find("head").unwrap(), rs.find("chest1").unwrap(), rs.find("wep0").unwrap());
            let mut m = f64::MAX;
            for k in 1..=7 {
                let pt = w[wp].point([-0.1 * k as f64, 0.0, 0.0]);
                m = m.min(crate::math::dist(pt, w[h].t)).min(crate::math::dist(pt, w[ch].t));
            }
            for n in ["l_hand", "r_hand"] {
                let i = rs.find(n).unwrap();
                m = m.min(crate::math::dist(w[i].t, w[h].t)).min(crate::math::dist(w[i].t, w[ch].t));
            }
            m
        };
        for (k, c) in CAST_CLIPS.iter().enumerate() {
            let next = if k >= 6 { clip_of(&ann, RIKU_FALL) } else { stance.clone() };
            let ko = c.key_out().floor() as usize;
            let before = (0..c.len).map(|f| near(&rraw[k], f as f64)).fold(f64::MAX, f64::min).min(near(&next, 0.0));
            let after = (ko..c.len).map(|f| near(&reased[k], f as f64)).fold(f64::MAX, f64::min);
            eprintln!("{}: nearest blade / hand to head / chest: in the ease {:.1} cm; the clip and next pose {:.1} cm", c.suffix, after * 100.0, before * 100.0);
            assert!(after >= before - 0.01, "{}: the ease brings the blade or a hand {:.1} cm to the head or chest ({:.1} before)", c.suffix, after * 100.0, before * 100.0);
        }
        // As built.
        let src = GuardSource { mset: &rm, mdls: &rdl, ansem: Some((&am, &ad)), terra };
        let b = build_with(&arc, &keys, 101, 201, 121, Some(&src)).unwrap();
        let pack = |grp: u16| Ann::parse(&b.files.iter().find(|f| f.0 == path(grp, "ANN")).unwrap().1).unwrap();
        for (grp, sk, pre, body, ground, air) in [(101u16, &rs, "p002_", "ex_1010_sk", RIKU_STANCE, RIKU_FALL), (201, &ds, "p003_", "ex_1580_sk", DARK_STANCE, DARK_FALL), (121, &ds, "p002_", "ex_1580_sk", RIKU_STANCE, RIKU_FALL)] {
            let a = pack(grp);
            let (st, fall) = (clip_of(&a, ground), clip_of(&a, air));
            for (k, c) in CAST_CLIPS.iter().enumerate() {
                let clip = clip_of(&a, &format!("{pre}{}", c.suffix));
                let next = if k >= 6 { &fall } else { &st };
                let last = gap(sk, &clip, (c.len - 1) as f64, next, 0.0, body);
                let free = gap(sk, &clip, c.free.min(c.len - 1) as f64, next, 0.0, body);
                eprintln!("{grp} {pre}{}: last frame {:.1} cm ({}), free frame {:.1} cm; fastest after the key moment {:.3} m a frame", c.suffix, last.0 * 100.0, last.1, free.0 * 100.0, fastest(sk, &clip, c.key_out().ceil() as usize));
                let rel = if c.is_windup() { clip_of(&a, &format!("{pre}{}", CAST_CLIPS[k + 1].suffix)) } else { next.clone() };
                // The skin's coat tails (`*_skt*`) are simulated after this (`crate::tails`, its own piece): checked apart.
                let (pop, pop_blend, pop_own, sc) = junction(sk, &clip, &rel, &|n| n.contains("_skt"));
                if grp == 121 {
                    let (t, tb, to, _) = junction(sk, &clip, &rel, &|n| !n.contains("_skt"));
                    eprintln!("    coat tails: {:.1} deg ({}); through the blend {:.1} deg ({}); the next clip's own {:.1} deg ({})", t.0, t.1, tb.0, tb.1, to.0, to.1);
                }
                eprintln!("    frame checks: worst one-frame turn {:.1} deg ({}); through the blend {:.1} deg ({}); the next clip's own {:.1} deg ({}); scale diff {sc:.4}", pop.0, pop.1, pop_blend.0, pop_blend.1, pop_own.0, pop_own.1);
                // No bone turns over 30 deg in a frame, or faster than the next clip's own frames do (Fire's thrust: the wind-up runs
                // into its release with no blend, at the motion's own speed); the blend's frames the same.
                let cap = 30.0f64.max(pop_own.0 + 1.0);
                assert!(pop.0 <= if c.is_windup() { cap } else { 30.0 } && (c.is_windup() || pop_blend.0 <= cap), "{grp} {pre}{}: a pop: {pop:?} / {pop_blend:?}", c.suffix);
                if c.is_windup() {
                    // Its release (blend 0) goes on from where it ends: one step at the thrust's own speed (not eased).
                    let r = clip_of(&a, &format!("{pre}{}", CAST_CLIPS[k + 1].suffix));
                    let rel = gap(sk, &clip, (c.len - 1) as f64, &r, 0.0, body);
                    let speed = fastest(sk, &r, 0);
                    eprintln!("    into its release: {:.1} cm ({}), the release's fastest step {:.1} cm", rel.0 * 100.0, rel.1, speed * 100.0);
                    assert!(rel.0 <= speed + 0.01, "{grp} {pre}{}: {:.1} cm ({}) from its release", c.suffix, rel.0 * 100.0, rel.1);
                } else {
                    assert!(last.0 < 0.02 && free.0 < 0.02, "{grp} {pre}{}: {:.1} cm ({}) from the next pose", c.suffix, last.0 * 100.0, last.1);
                }
            }
        }
    }

    /// 0.198.13 renders (`RECOM_GAME`), `out/cast_ends/<group>_<before|after>/`: the built casts of Riku (`PL0101`, his model and
    /// Soul Eater), Dark Riku (`PL0201`) and the Dark Form skin (121, Dark Riku's model), at the key moment's end, the middle of the
    /// ease, the free frame and the last, and the next clip's frame 0, front and side. `CASTS=231,229`: only those clips.
    #[test]
    fn cast_end_renders() {
        let Ok(g) = std::env::var("RECOM_GAME").map(std::path::PathBuf::from) else { return };
        std::env::set_var("FULL_BODY", "1");
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let k1 = install::kh1(&g, keys.clone()).unwrap();
        let (am, ad) = (k1.read("xa_ex_1580.mset").unwrap(), k1.read("xa_ex_1580.mdls").unwrap());
        let (rm, rdl) = (k1.read("xa_ex_1010.mset").unwrap(), k1.read("xa_ex_1010.mdls").unwrap());
        let terra_data = crate::terra::read_terra(&g, &keys).ok();
        let src = GuardSource { mset: &rm, mdls: &rdl, ansem: Some((&am, &ad)), terra: terra_data.as_ref().map(|(k, c)| (k, c.as_slice())) };
        let only: Option<Vec<String>> = std::env::var("CASTS").ok().map(|v| v.split(',').map(String::from).collect());
        for which in ["before", "after"] {
            EASE_OFF.with(|x| x.set(which == "before"));
            let b = build_with(&arc, &keys, 101, 201, 121, Some(&src)).unwrap();
            EASE_OFF.with(|x| x.set(false));
            for (grp, model, pre, ground, air, atlas_name) in [(101u16, "0101", "p002_", RIKU_STANCE, RIKU_FALL, "n_zz110_01"), (201, "0201", "p003_", DARK_STANCE, DARK_FALL, "n_zz120_01"), (121, "0201", "p002_", RIKU_STANCE, RIKU_FALL, "n_zz120_01")] {
                let ann = Ann::parse(&b.files.iter().find(|f| f.0 == path(grp, "ANN")).unwrap().1).unwrap();
                let mdl = install::read_recom(&arc, &keys, &format!("PLAYER/{model}/PL{model}.MDL")).unwrap();
                let e = arc.find(&format!("PLAYER/{model}/PL{model}.RTM")).unwrap();
                let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
                let rtm_p = crate::rtm::Rtm::parse(&rtm).unwrap();
                let parts = crate::mdl::split_parts(&mdl).unwrap();
                let mut p3 = crate::mdl::MdlPart::parse_bytes(&parts[3]).unwrap();
                let p1 = crate::mdl::MdlPart::parse_bytes(&parts[1]).unwrap();
                let dec = |n: &str| {
                    let e = rtm_p.entries.iter().find(|e| e.name_str().trim_end_matches(".tm2") == n.trim_end_matches(".tm2")).unwrap();
                    let (w, h, px) = crate::rtm::tim2_decode8(&e.data).unwrap();
                    crate::texture::Image { w, h, px }
                };
                let mut body: Vec<crate::texture::Image> = p3.texture_names().iter().map(|n| dec(n)).collect();
                let a = p3.texture_names().iter().position(|n| n == atlas_name).unwrap() as u32;
                for k in p3.list_a.iter_mut().chain(p3.list_b.iter_mut().flatten()) {
                    for v in k.verts.iter_mut() {
                        if let Some(q) = v.q2.as_mut() {
                            if q[3] == a { q[3] = 1; } else if q[3] == 1 { q[3] = a; }
                        }
                    }
                }
                body.swap(1, a as usize);
                let wi = body.len() as u32;
                body.push(dec("weapon_01"));
                let mut wl = p1.list_a.clone();
                for k in wl.iter_mut() { for v in k.verts.iter_mut() { if let Some(q) = v.q2.as_mut() { q[3] = wi; } } }
                p3.list_a.extend(wl);
                let img = hd.iter().find(|x| x.name == crate::keyblade::hd_name(atlas_name)).and_then(|x| crate::texture::decode_dds(&x.data).ok()).unwrap_or_else(|| body[1].clone());
                let sk = RecomSkeleton::parse(&mdl).unwrap();
                let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
                let dir = crate::ddd_hair::tests::out_dir().join("cast_ends").join(format!("{grp}_{which}"));
                std::fs::create_dir_all(&dir).unwrap();
                for (k, c) in CAST_CLIPS.iter().enumerate() {
                    if c.is_windup() || only.as_ref().is_some_and(|o| !o.iter().any(|x| x == c.suffix)) { continue; }
                    let clip = clip_of(&ann, &format!("{pre}{}", c.suffix));
                    let next = clip_of(&ann, if k >= 6 { air } else { ground });
                    let ko = c.key_out().ceil();
                    let fr = c.free.min(c.len - 1) as f64;
                    let frames = [("a_key", ko), ("b_mid", ((ko + fr) / 2.0).round()), ("c_free", fr), ("d_last", (c.len - 1) as f64)];
                    for (label, f) in frames.iter().map(|&(l, f)| (l, Some(f))).chain([("e_next", None)]) {
                        let w = match f { Some(f) => crate::kh2_hair::tests::posed_world(&sk, &clip, f), None => crate::kh2_hair::tests::posed_world(&sk, &next, 0.0) };
                        for (yn, yaw) in [("front", 0.0), ("side", 90.0)] {
                            let name = format!("{}_{label}{}_{yn}.png", c.suffix, f.map_or(String::new(), |f| format!("{f:.0}")));
                            crate::kh2_hair::tests::render_cam(&p3, &w, &names, &img, &body, yaw, 0.0, true, &dir.join(name));
                        }
                    }
                }
            }
        }
    }
}

