//! Verification against Re:CoM's PS2 bakes of the same KH1 motions (`kh1-posing.md` §8):
//! Re:CoM frame f = KH1 time 2f, units / 100, Y up. Needs `KH1_TEST_DATA` with
//! `xa_ex_0010.mdls`, `xa_ex_0010.mset`, `PL0001.MDL`, `PL0001.ANN`; skips when unset.
//!
//! (A) per Re:CoM bone, local rotation (geodesic angle, all three rotation channels keyed at
//!     that frame) and local translation channels, on unit-scale chains;
//! (B) world positions on chains whose every channel (bone and ancestors) is keyed.
//! Both only where the bake stores an exact value (u16 quantisation), so key reduction does not
//! enter. The Re:CoM readers here are test-only ports of the research scripts `recom_mdl.py`,
//! `recom_ann.py` and `skel_map.py`.

#![allow(clippy::needless_range_loop)]

use std::path::PathBuf;

use kh1_anim::{Affine, Rig, WeaponLine, WEAPON_ATTACH};
use kh1_formats::mdls::Skeleton;
use kh1_formats::mset::MotionSet;

type M3 = [[f64; 3]; 3];

#[derive(Clone, Copy)]
struct Aff {
    r: M3,
    t: [f64; 3],
}

impl Aff {
    fn mul(&self, b: &Aff) -> Aff {
        let mut r = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                r[i][j] = (0..3).map(|k| self.r[i][k] * b.r[k][j]).sum();
            }
        }
        let mut t = b.t;
        for j in 0..3 {
            t[j] += (0..3).map(|k| self.t[k] * b.r[k][j]).sum::<f64>();
        }
        Aff { r, t }
    }
    fn from(a: &Affine) -> Aff {
        let mut r = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                r[i][j] = a.r[i][j] as f64;
            }
        }
        Aff { r, t: [a.t[0] as f64, a.t[1] as f64, a.t[2] as f64] }
    }
}

fn rot_xyz(rx: f64, ry: f64, rz: f64) -> M3 {
    let (sx, cx) = rx.sin_cos();
    let (sy, cy) = ry.sin_cos();
    let (sz, cz) = rz.sin_cos();
    let a = [[1.0, 0.0, 0.0], [0.0, cx, sx], [0.0, -sx, cx]];
    let b = [[cy, 0.0, -sy], [0.0, 1.0, 0.0], [sy, 0.0, cy]];
    let c = [[cz, sz, 0.0], [-sz, cz, 0.0], [0.0, 0.0, 1.0]];
    m3(&m3(&a, &b), &c)
}

fn m3(a: &M3, b: &M3) -> M3 {
    let mut o = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            o[i][j] = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    o
}

fn tr(a: &M3) -> M3 {
    let mut o = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            o[i][j] = a[j][i];
        }
    }
    o
}

fn normed_rows(a: &M3) -> M3 {
    let mut o = *a;
    for row in o.iter_mut() {
        let l = (row[0] * row[0] + row[1] * row[1] + row[2] * row[2]).sqrt();
        for v in row.iter_mut() {
            *v /= l;
        }
    }
    o
}

/// Rotation angle of `q` in degrees, via atan2(|skew|, (trace − 1)/2): robust for small angles
/// (f32 non-orthogonality does not leak into it as acos of the trace would).
fn angle_deg(q: &M3) -> f64 {
    let s = [(q[1][2] - q[2][1]) / 2.0, (q[2][0] - q[0][2]) / 2.0, (q[0][1] - q[1][0]) / 2.0];
    let sn = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt();
    let c = (q[0][0] + q[1][1] + q[2][2] - 1.0) / 2.0;
    sn.atan2(c).to_degrees()
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

// ------------------------------------------------------------------ Re:CoM PL0001.MDL
struct RBone {
    name: String,
    parent: i16,
    local: Aff,
}

fn rd_u16(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}
fn rd_i16(b: &[u8], o: usize) -> i16 {
    rd_u16(b, o) as i16
}
fn rd_u32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn rd_f32(b: &[u8], o: usize) -> f32 {
    f32::from_bits(rd_u32(b, o))
}
fn cname(b: &[u8]) -> String {
    let e = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..e]).into_owned()
}

fn recom_mdl(d: &[u8]) -> Vec<RBone> {
    let n = rd_u16(d, 0x10) as usize;
    let nt = 0x10 + rd_u32(d, 0x14) as usize;
    let mt = 0x10 + rd_u32(d, 0x18) as usize;
    (0..n)
        .map(|i| {
            let o = nt + 20 * i;
            let m = |k: usize| rd_f32(d, mt + 64 * i + 4 * k) as f64;
            let local = Aff { r: [[m(0), m(1), m(2)], [m(4), m(5), m(6)], [m(8), m(9), m(10)]], t: [m(12), m(13), m(14)] };
            RBone { name: cname(&d[o..o + 16]), parent: rd_i16(d, o + 16), local }
        })
        .collect()
}

// ------------------------------------------------------------------ Re:CoM .ANN clips
struct Curve {
    idx: Vec<u16>,
    v: Vec<f64>,
    bits: Vec<u8>,
    looped: bool,
}

impl Curve {
    fn linear_seg(&self, k: usize) -> bool {
        (self.bits[k >> 3] >> (k & 7)) & 1 != 0
    }
    /// FUN_14027b320 + FUN_14027c1d0, stateless (port of recom_ann.Curve.eval).
    fn eval(&self, t: f64) -> f64 {
        let (n, idx, v) = (self.v.len(), &self.idx, &self.v);
        let ti = |k: usize| idx[k] as f64;
        if n == 2 {
            return (v[1] - v[0]) * (t - ti(0)) / (ti(1) - ti(0)) + v[0];
        }
        if n == 1 {
            return v[0];
        }
        if let Some(k) = (0..n).find(|&k| ti(k) == t) {
            return v[k];
        }
        if t > ti(n - 1) {
            return if self.looped { v[n - 1] + (v[0] - v[n - 1]) * (t - ti(n - 1)) } else { v[n - 1] };
        }
        let e = (0..n).find(|&k| ti(k) >= t).unwrap();
        let s = e - 1;
        let (t0, t1, v0, v1) = (ti(s), ti(e), v[s], v[e]);
        let u = (t - t0) / (t1 - t0);
        if self.linear_seg(s) {
            return v0 + (v1 - v0) * u;
        }
        let m0 = if s == 0 { (v1 - v0) / (t1 - t0) } else { (v0 - v[s - 1]) / (t0 - ti(s - 1)) };
        let m1 = if s == n - 2 || e + 1 >= n { (v1 - v0) / (t1 - t0) } else { (v[e + 1] - v1) / (ti(e + 1) - t1) };
        let (u2, u3) = (u * u, u * u * u);
        (2.0 * u3 - 3.0 * u2 + 1.0) * v0 + (3.0 * u2 - 2.0 * u3) * v1 + (u3 - 2.0 * u2 + u) * m0 + (u3 - u2) * m1
    }
}

enum Ch {
    Const(f64),
    Anim(Curve),
}

struct Clip {
    name: String,
    frames: usize,
    chans: usize,
    ch: Vec<Ch>,
}

impl Clip {
    fn parse(b: &[u8]) -> Clip {
        let name = cname(&b[..16]);
        let bones = rd_i16(b, 0x10) as usize;
        let frames = rd_i16(b, 0x12) as usize;
        let loop_start = (rd_i16(b, 0x14) as usize).min(frames);
        let flags = b[0x16];
        let chans = if flags & 2 != 0 { 9 } else { 6 };
        let p = if b[0x17] != 0 { 0x1C } else { 0x18 };
        let nn = bones * chans;
        let msz = (((nn + 7) >> 3) + 3) & !3;
        let mask = &b[p..p + msz];
        let cst = p + msz;
        let sizes = cst + 4 * nn;
        let mut q = cst + 8 * nn;
        let mut ch = Vec::with_capacity(nn);
        for i in 0..nn {
            let rot = (3..6).contains(&(i % chans));
            if (mask[i >> 3] >> (i & 7)) & 1 != 0 {
                let c = rd_f32(b, cst + 4 * i) as f64;
                ch.push(Ch::Const(if rot { c.to_radians() } else { c }));
            } else {
                let n = rd_u32(b, q) as usize;
                let (mut base, mut rng) = (rd_f32(b, q + 4) as f64, rd_f32(b, q + 8) as f64);
                let mut o = q + 12;
                let idx: Vec<u16> = if frames < 0x100 {
                    let v = b[o..o + n].iter().map(|&x| x as u16).collect();
                    o += (n + 1) & !1;
                    v
                } else {
                    let v = (0..n).map(|k| rd_u16(b, o + 2 * k)).collect();
                    o += 2 * n;
                    v
                };
                let qq: Vec<u16> = (0..n).map(|k| rd_u16(b, o + 2 * k)).collect();
                o += 2 * n;
                let bl = (((n + 7) >> 3) + 1) & !1;
                let bits = b[o..o + bl].to_vec();
                if rot {
                    base = base.to_radians();
                    rng = rng.to_radians();
                }
                let v = qq.iter().map(|&x| x as f64 * rng * (1.0 / 65535.0) + base).collect();
                ch.push(Ch::Anim(Curve { idx, v, bits, looped: loop_start >= 1 }));
                q += rd_u32(b, sizes + 4 * i) as usize;
            }
        }
        Clip { name, frames, chans, ch }
    }
    fn value(&self, bone: usize, c: usize, t: f64) -> f64 {
        match &self.ch[bone * self.chans + c] {
            Ch::Const(x) => *x,
            Ch::Anim(cv) => cv.eval(t),
        }
    }
    /// (S, R, T) of a bone at clip frame f.
    fn srt(&self, bone: usize, f: usize) -> ([f64; 3], [f64; 3], [f64; 3]) {
        let v = |c| self.value(bone, c, f as f64);
        let s = if self.chans == 9 { [v(6), v(7), v(8)] } else { [1.0; 3] };
        (s, [v(3), v(4), v(5)], [v(0), v(1), v(2)])
    }
    /// Channel c of bone is constant or keyed at frame f.
    fn keyed(&self, bone: usize, c: usize, f: usize) -> bool {
        match &self.ch[bone * self.chans + c] {
            Ch::Const(_) => true,
            Ch::Anim(cv) => cv.idx.contains(&(f as u16)),
        }
    }
    fn animated(&self, bone: usize, c: usize) -> bool {
        matches!(self.ch[bone * self.chans + c], Ch::Anim(_))
    }
}

fn read_ann(d: &[u8]) -> Vec<Clip> {
    let n = rd_u32(d, 0) as usize;
    let mut p = 4 + 20 * n;
    let mut out = Vec::new();
    for i in 0..n {
        let sz = rd_u32(d, 4 + 20 * i) as usize;
        if sz != 0 {
            out.push(Clip::parse(&d[p..p + sz]));
            p += sz;
        }
    }
    out
}

/// Re:CoM world matrices of a clip frame, composed the way the bake encodes KH1: rotations chain
/// unscaled, the stored scale is the accumulated scale, offsets use the parent's scaled world
/// (port of kh1_pose.recom_world).
fn recom_world(rb: &[RBone], c: &Clip, f: usize) -> Vec<Aff> {
    let mut wr: Vec<M3> = Vec::with_capacity(rb.len());
    let mut w: Vec<Aff> = Vec::with_capacity(rb.len());
    for (i, b) in rb.iter().enumerate() {
        let (s, r, t) = c.srt(i, f);
        let rm = rot_xyz(r[0], r[1], r[2]);
        let (rot, tt) = if b.parent < 0 {
            (rm, t)
        } else {
            let p = b.parent as usize;
            (m3(&rm, &wr[p]), w[p].mul_point(t))
        };
        wr.push(rot);
        let mut m = rot;
        for k in 0..3 {
            for v in m[k].iter_mut() {
                *v *= s[k];
            }
        }
        w.push(Aff { r: m, t: tt });
    }
    w
}

impl Aff {
    fn mul_point(&self, p: [f64; 3]) -> [f64; 3] {
        let mut o = self.t;
        for (j, oj) in o.iter_mut().enumerate() {
            *oj += (0..3).map(|k| p[k] * self.r[k][j]).sum::<f64>();
        }
        o
    }
}

// ------------------------------------------------------------------ bone map (skel_map.py)
/// Re:CoM bone → KH1 bone (None for the Root).
fn bone_map(k: &Skeleton, rb: &[RBone]) -> Vec<Option<usize>> {
    // KH1 rest world with a plain S·R·T chain, Re:CoM rest world from its local matrices.
    let mut kw: Vec<Aff> = Vec::new();
    for b in &k.bones {
        let r = rot_xyz(b.rot[0] as f64, b.rot[1] as f64, b.rot[2] as f64);
        let mut m = r;
        for i in 0..3 {
            for v in m[i].iter_mut() {
                *v *= b.scale[i] as f64;
            }
        }
        let l = Aff { r: m, t: [b.trans[0] as f64, b.trans[1] as f64, b.trans[2] as f64] };
        kw.push(match b.parent {
            None => l,
            Some(p) => l.mul(&kw[p as usize]),
        });
    }
    let mut rw: Vec<Aff> = Vec::new();
    for b in rb {
        rw.push(if b.parent < 0 { b.local } else { b.local.mul(&rw[b.parent as usize]) });
    }
    let mut good: Vec<(Vec<usize>, Vec<usize>)> = Vec::new();
    for w in &rw {
        let cands: Vec<usize> =
            (0..kw.len()).filter(|&j| dist(w.t, [kw[j].t[0] * 0.01, kw[j].t[1] * 0.01, kw[j].t[2] * 0.01]) < 1e-4).collect();
        let g: Vec<usize> = cands
            .iter()
            .copied()
            .filter(|&j| (0..3).all(|a| (0..3).all(|c| (w.r[a][c] - kw[j].r[a][c]).abs() < 1e-3)))
            .collect();
        good.push((cands, g));
    }
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
            if ok.is_empty() { g.iter().max().copied() } else { ok.iter().max().copied() }
        })
        .collect()
}

// ------------------------------------------------------------------ the checks
struct Data {
    skel: Skeleton,
    mdls: Vec<u8>,
    ms: MotionSet,
    rb: Vec<RBone>,
    clips: Vec<Clip>,
}

fn data() -> Option<Data> {
    let dir = PathBuf::from(std::env::var_os("KH1_TEST_DATA")?);
    let rd = |n: &str| match std::fs::read(dir.join(n)) {
        Ok(b) => Some(b),
        Err(_) => {
            eprintln!("skipping: {} not found", dir.join(n).display());
            None
        }
    };
    let mdls = rd("xa_ex_0010.mdls")?;
    let ms = MotionSet::parse(&rd("xa_ex_0010.mset")?).unwrap();
    let rb = recom_mdl(&rd("PL0001.MDL")?);
    let clips = read_ann(&rd("PL0001.ANN")?);
    Some(Data { skel: Skeleton::parse(&mdls).unwrap(), mdls, ms, rb, clips })
}

#[derive(Default, Debug)]
struct Stats {
    rot_n: usize,
    rot_max: f64,
    rot_sum: f64,
    pos_n: usize,
    pos_max: f64,
    world_n: usize,
    world_max: f64,
    world_sum: f64,
}

fn check(d: &Data, entry: usize, clip: &str) -> Stats {
    assert_eq!((d.rb.len(), d.rb[0].name.as_str(), d.rb[1].name.as_str()), (135, "Root", "ex_0010_sk"));
    let map = bone_map(&d.skel, &d.rb);
    assert_eq!(map.iter().filter(|m| m.is_some()).count(), 134, "Re:CoM bones mapped (all but Root)");
    let c = d.clips.iter().find(|c| c.name == clip).expect("clip");
    let m = &d.ms.motions[entry];
    let mut rig = Rig::from_mdls(&d.mdls).unwrap();
    let rb = &d.rb;
    let mut st = Stats::default();
    let mut off: Option<[f64; 3]> = None;
    for f in 0..c.frames {
        let pose = rig.pose(m, 2.0 * f as f32);
        // model space, Y up, cm (W · diag(1, −1, −1))
        let kw: Vec<Aff> = pose
            .world
            .iter()
            .map(|w| {
                let mut a = Aff::from(w);
                for row in a.r.iter_mut().chain(std::iter::once(&mut a.t)) {
                    row[1] = -row[1];
                    row[2] = -row[2];
                }
                a
            })
            .collect();
        let rw = recom_world(rb, c, f);
        // (A) local channels at Re:CoM key frames, unit-scale chains.
        for b in 2..rb.len() {
            let pb = rb[b].parent as usize;
            let (Some(kb), Some(kp)) = (map[b], map[pb]) else { continue };
            let mut unit = true;
            let mut j = b as i16;
            while j >= 0 {
                if c.srt(j as usize, f).0.iter().any(|s| (s - 1.0).abs() > 1e-4) {
                    unit = false;
                    break;
                }
                j = rb[j as usize].parent;
            }
            if !unit {
                continue;
            }
            let a = &kw[kb];
            let ap = &kw[kp];
            let ar = normed_rows(&a.r);
            let apr = if pb >= 2 { normed_rows(&ap.r) } else { [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]] };
            let rloc = m3(&ar, &tr(&apr));
            let (_, rr, tt) = c.srt(b, f);
            let dt = [(a.t[0] - ap.t[0]) * 0.01, (a.t[1] - ap.t[1]) * 0.01, (a.t[2] - ap.t[2]) * 0.01];
            let tl: Vec<f64> = (0..3).map(|j| (0..3).map(|k| dt[k] * apr[j][k]).sum()).collect();
            for (ch, &tlc) in tl.iter().enumerate() {
                if c.animated(b, ch) && !c.keyed(b, ch, f) {
                    continue;
                }
                let e = (tlc - tt[ch]).abs() * 1000.0;
                st.pos_n += 1;
                st.pos_max = st.pos_max.max(e);
            }
            if (3..6).all(|ch| c.keyed(b, ch, f)) {
                let q = m3(&rloc, &tr(&rot_xyz(rr[0], rr[1], rr[2])));
                let e = angle_deg(&q);
                st.rot_n += 1;
                st.rot_sum += e;
                st.rot_max = st.rot_max.max(e);
            }
        }
        // (B) world positions on fully keyed chains.
        let mut keyed = vec![false; rb.len()];
        for b in 0..rb.len() {
            let own = (0..c.chans).all(|ch| c.keyed(b, ch, f));
            keyed[b] = own && (rb[b].parent < 0 || keyed[rb[b].parent as usize]);
        }
        let o = *off.get_or_insert_with(|| {
            let k0 = kw[0].t;
            [rw[1].t[0] - k0[0] * 0.01, rw[1].t[1] - k0[1] * 0.01, rw[1].t[2] - k0[2] * 0.01]
        });
        for b in 1..rb.len() {
            let Some(kb) = map[b] else { continue };
            if !keyed[b] {
                continue;
            }
            let k = kw[kb].t;
            let e = dist([k[0] * 0.01 + o[0], k[1] * 0.01 + o[1], k[2] * 0.01 + o[2]], rw[b].t) * 1000.0;
            st.world_n += 1;
            st.world_sum += e;
            st.world_max = st.world_max.max(e);
        }
    }
    eprintln!(
        "KH1 entry {entry} vs {clip} ({} frames): (A) rot n {} max {:.4} mean {:.5} deg, pos n {} max {:.4} mm | (B) world pos n {} max {:.4} mean {:.4} mm",
        c.frames,
        st.rot_n,
        st.rot_max,
        st.rot_sum / st.rot_n.max(1) as f64,
        st.pos_n,
        st.pos_max,
        st.world_n,
        st.world_max,
        st.world_sum / st.world_n.max(1) as f64
    );
    st
}

#[test]
fn matches_recom_bakes() {
    let Some(d) = data() else { return };
    for (entry, clip) in [(98, "p001_055"), (117, "p001_070"), (4, "p001_020"), (8, "p001_004")] {
        let st = check(&d, entry, clip);
        assert!(st.rot_n > 100 && st.pos_n > 1000 && st.world_n > 100, "{st:?}");
        assert!(st.rot_max < 0.03, "entry {entry}: local rotation {} deg", st.rot_max);
        assert!(st.pos_max < 0.005, "entry {entry}: local translation {} mm", st.pos_max);
        assert!(st.world_max < 0.1, "entry {entry}: world position {} mm", st.world_max);
    }
}

fn rig_and_set() -> Option<(Rig, MotionSet)> {
    let d = data()?;
    Some((Rig::from_mdls(&d.mdls).unwrap(), d.ms))
}

fn max_diff(a: &[Affine], b: &[Affine], bones: impl Iterator<Item = usize>) -> f32 {
    let mut e = 0.0f32;
    for i in bones {
        for r in 0..3 {
            for c in 0..3 {
                e = e.max((a[i].r[r][c] - b[i].r[r][c]).abs());
            }
            e = e.max((a[i].t[r] - b[i].t[r]).abs() / 100.0);
        }
    }
    e
}

#[test]
fn blend_endpoints() {
    let Some((mut rig, ms)) = rig_and_set() else { return };
    let skel = Skeleton::parse(&std::fs::read(PathBuf::from(std::env::var_os("KH1_TEST_DATA").unwrap()).join("xa_ex_0010.mdls")).unwrap()).unwrap();
    let blend_bones: Vec<usize> = (0..rig.bone_count()).filter(|&i| skel.bones[i].link_flags & 1 != 0).collect();
    assert_eq!(blend_bones.len(), 142);
    for (old, t_old, new, t_new) in [(4usize, 13.0f32, 98usize, 0.0f32), (98, 30.0, 117, 0.0), (117, 20.0, 8, 0.0), (8, 6.0, 11, 0.0)] {
        let (mo, mn) = (&ms.motions[old], &ms.motions[new]);
        let old_w = rig.pose(mo, t_old).world.clone();
        let mut fresh = Rig::from_mdls(&std::fs::read(PathBuf::from(std::env::var_os("KH1_TEST_DATA").unwrap()).join("xa_ex_0010.mdls")).unwrap()).unwrap();
        let new_w = fresh.pose(mn, t_new).world.clone();
        let w0 = rig.blend_start(mn, t_new, 0.0).world.clone();
        let e0 = max_diff(&w0, &old_w, blend_bones.iter().copied());
        let w1 = rig.blend(1.0).world.clone();
        let e1 = max_diff(&w1, &new_w, blend_bones.iter().copied());
        let wm = rig.blend(0.5).world.clone();
        assert!(e0 < 2e-5 && e1 < 2e-5, "{old}->{new}: w=0 {e0}, w=1 {e1}");
        assert!(wm.iter().all(|m| m.t.iter().all(|v| v.is_finite())));
        assert!(rig.is_blending());
        // Restart while blending: the old snapshot is the previous NEW snapshot.
        let w2 = rig.blend_start(mo, t_old, 0.0).world.clone();
        let e2 = max_diff(&w2, &new_w, blend_bones.iter().copied());
        eprintln!("blend {old}->{new}: w=0 {e0:.1e}, w=1 {e1:.1e}, restart w=0 {e2:.1e}");
        assert!(e2 < 2e-5, "restart {old}->{new}->{old}: w=0 {e2}");
        // Blend end: pose drops the snapshots.
        rig.pose(mo, t_old);
        assert!(!rig.is_blending());
        let e3 = max_diff(&rig.current().world, &old_w, (0..rig.bone_count()).filter(|&i| rig.is_posed(i)));
        assert!(e3 < 2e-5, "after blend {e3}");
    }
}

#[test]
fn keyblade_line_and_conversion() {
    let Some((mut rig, ms)) = rig_and_set() else { return };
    assert_eq!(rig.attach_bone(WEAPON_ATTACH), Some(283));
    assert_eq!(rig.parent(283), Some(282));
    assert_eq!(rig.info().look_at_bone, Some(147));
    assert_eq!(rig.info().foot_ik_bones, vec![17, 43]);
    let key = WeaponLine { t0: 0.0, t1: -3.7, frac_scale: 0.8, radius: 40.0 };
    for t in [0.0, 14.0, 20.0, 24.0] {
        rig.pose(&ms.motions[98], t);
        let (a, b) = rig.blade_segment(&key, 1.0, 0.0).unwrap();
        let grip = rig.current().world[282].t;
        assert!((0..3).all(|k| (b[k] - grip[k]).abs() < 1e-4));
        let l = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
        // 2.96 × the 21.107 bone length (unit scale on the weapon chain)
        assert!((l - 2.96 * 21.107).abs() < 0.05, "blade length {l}");
    }
    // Model matrix: translation moves every posed bone; a 2x model scale doubles offsets.
    let base = rig.pose(&ms.motions[4], 10.0).world.clone();
    let model = Affine::from_translation([100.0, 0.0, -50.0]);
    let moved = rig.pose_in(&ms.motions[4], 10.0, &model).world.clone();
    for i in (0..rig.bone_count()).filter(|&i| rig.is_posed(i)) {
        assert!((moved[i].t[0] - base[i].t[0] - 100.0).abs() < 1e-3 && (moved[i].t[2] - base[i].t[2] + 50.0).abs() < 1e-3);
    }
    let y = base[0].to_yup_metres();
    assert!((y.t[1] + base[0].t[1] * 0.01).abs() < 1e-6);
}

#[test]
fn pose_speed() {
    let Some((mut rig, ms)) = rig_and_set() else { return };
    let m = &ms.motions[98];
    rig.pose(m, 0.0);
    let n = 2000;
    let t0 = std::time::Instant::now();
    for k in 0..n {
        rig.pose(m, (k % 48) as f32);
    }
    let per = t0.elapsed().as_secs_f64() / n as f64;
    let t1 = std::time::Instant::now();
    rig.blend_start(&ms.motions[117], 0.0, 0.1);
    for k in 0..n {
        rig.blend((k % 8) as f32 / 8.0);
    }
    let per_blend = t1.elapsed().as_secs_f64() / n as f64;
    eprintln!("pose {:.1} us, blend frame {:.1} us ({} bones)", per * 1e6, per_blend * 1e6, rig.bone_count());
    if !cfg!(debug_assertions) {
        assert!(per < 1e-3 && per_blend < 1e-3);
    }
}
