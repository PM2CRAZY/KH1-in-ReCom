//! The per-frame pose: `FUN_140391C70` (motion change `FUN_14038F550`, curves, helpers, bone
//! pass `FUN_1403900B0`, IK solver `FUN_14038FC20`, model scale and translation).
//! Spec: `kh1-posing.md` §1-§4 (all branches Checked (decomp); outputs Checked (files) against
//! Re:CoM's bakes, see the crate tests).

use kh1_formats::mdls::Skeleton;
use kh1_formats::mset::Motion;

use crate::blend::BlendState;
use crate::math::{add, cross, len, mat_rt, mat_srt, mulv, norm, rx, sub, wrap, Affine, Vec3};
use crate::model::ModelInfo;

/// Bone flag bits (motion +0x14), as the bone pass reads them.
pub mod flag {
    /// Bit 26: the bone is skipped entirely.
    pub const SKIP: u32 = 0x400_0000;
    /// Bit 25: the local matrix is not rebuilt from channels (built at the motion change).
    pub const NOT_REBUILT: u32 = 0x200_0000;
    /// Bit 24: rotation in model space (`U = L · H0`).
    pub const MODEL_ROT: u32 = 0x100_0000;
    /// Bit 23: take the position an earlier IK of this pass wrote into `W[i].t`.
    pub const KEEP_IK_POS: u32 = 0x80_0000;
    /// Bits 21-22: IK (1 one-bone aim, 2 two-bone IK, 3 two-bone IK bent the other way).
    pub const IK_MASK: u32 = 0x60_0000;
}

/// 1-based helper index the bone takes its position from (0 = none).
#[inline]
pub(crate) fn hpos_of(v: u32) -> usize {
    (((v & 0xF_F800) | ((v >> 8) & 0x10_0000)) >> 11) as usize
}

/// 1-based helper index for aim / orientation / pole (0 = none).
#[inline]
pub(crate) fn aim_of(v: u32) -> usize {
    (((v & 0x7FC) | ((v >> 16) & 0x800)) >> 2) as usize
}

/// The output of a pose: the instance arrays KH1 keeps.
#[derive(Clone, Debug, PartialEq)]
pub struct Pose {
    /// `+0x1B4` world matrices, KH1 units (≈ cm), Y down, model scale and translation applied.
    /// Skipped bones (flag bit 26) are not maintained (KH1 still adds the model translation to
    /// them every frame; so does this crate).
    pub world: Vec<Affine>,
    /// `+0x1B8` local matrices (parent-relative, unscaled). During a blend KH1 reuses this array
    /// as the *unscaled world* matrices of the blended bones; so does this crate.
    pub local: Vec<Affine>,
    /// `+0x100` root travel: model scale ⊙ the last helper's translation (motion space, Y down).
    /// Kept out of the bone matrices' meaning: the caller moves the actor by its change.
    pub root_travel: Vec3,
    /// `+0xF0`: model translation + root travel (in a blend: model scale ⊙ root travel + model
    /// translation, as the exe writes it).
    pub travel_and_model: Vec3,
    /// Motion flag-track bits at this time (`kh1_formats::mset::bits`: hit slots, free, combo,
    /// trail). Not re-evaluated during a blend (the new motion's start-frame bits stay).
    pub flags: u32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct HelperRec {
    pub scale: Vec3,
    pub parent: i32,
    pub rot: Vec3,
    pub trans: Vec3,
}

/// One posing instance (KH1's motion instance, `actor+0x15C`) for one skeleton.
#[derive(Clone, Debug)]
pub struct Rig {
    pub(crate) n: usize,
    pub(crate) parent: Vec<Option<usize>>,
    pub(crate) rest_s: Vec<Vec3>,
    pub(crate) rest_r: Vec<Vec3>,
    pub(crate) rest_t: Vec<Vec3>,
    /// mdls link bit 30 ("blend bone").
    pub(crate) link30: Vec<bool>,
    pub(crate) info: ModelInfo,
    // locals (+0x1B0): scale, twist stash, accumulated scale, rotation, translation
    pub(crate) s: Vec<Vec3>,
    pub(crate) tw: Vec<f32>,
    pub(crate) acc: Vec<Vec3>,
    pub(crate) r: Vec<Vec3>,
    pub(crate) t: Vec<Vec3>,
    /// `DAT_142EE8F50`: unscaled world rotations.
    pub(crate) u: Vec<Affine>,
    pub(crate) pose: Pose,
    /// The current motion's bone flags, with the look-at bone's bit 25 cleared (the bone-pass
    /// prologue writes that into the motion data in KH1).
    pub(crate) flags: Vec<u32>,
    pub(crate) helpers: Vec<HelperRec>,
    /// `DAT_142EF1F50`: helper world matrices; `h[0]` = H0.
    pub(crate) h: Vec<Affine>,
    /// `+0x130`: the model matrix with rows 1, 2 negated (not normalised, translation kept).
    pub(crate) h0_full: Affine,
    pub(crate) cur: Option<(usize, usize)>,
    pub(crate) blend: Option<BlendState>,
}

pub(crate) fn motion_id(m: &Motion) -> (usize, usize) {
    (m as *const Motion as usize, m.offset)
}

impl Rig {
    /// A rig for `skel` with the model info block of the same `.mdls`.
    pub fn new(skel: &Skeleton, info: &ModelInfo) -> Rig {
        let n = skel.len();
        let id = Affine::IDENTITY;
        Rig {
            n,
            parent: skel.bones.iter().map(|b| b.parent.map(|p| p as usize).filter(|&p| p < n)).collect(),
            rest_s: skel.bones.iter().map(|b| b.scale).collect(),
            rest_r: skel.bones.iter().map(|b| b.rot).collect(),
            rest_t: skel.bones.iter().map(|b| b.trans).collect(),
            link30: skel.bones.iter().map(|b| b.link_flags & 1 != 0).collect(),
            info: info.clone(),
            s: skel.bones.iter().map(|b| b.scale).collect(),
            tw: vec![0.0; n],
            acc: vec![[1.0; 3]; n],
            r: skel.bones.iter().map(|b| b.rot).collect(),
            t: skel.bones.iter().map(|b| b.trans).collect(),
            u: vec![id; n],
            pose: Pose { world: vec![id; n], local: vec![id; n], root_travel: [0.0; 3], travel_and_model: [0.0; 3], flags: 0 },
            flags: vec![flag::SKIP; n],
            helpers: Vec::new(),
            h: Vec::new(),
            h0_full: id,
            cur: None,
            blend: None,
        }
    }

    /// Reads skeleton and model info from a whole `.mdls` file.
    pub fn from_mdls(file: &[u8]) -> kh1_formats::Result<Rig> {
        Ok(Rig::new(&Skeleton::parse(file)?, &ModelInfo::parse(file)?))
    }

    /// Number of bones.
    pub fn bone_count(&self) -> usize {
        self.n
    }

    /// Parent of a bone.
    pub fn parent(&self, bone: usize) -> Option<usize> {
        self.parent.get(bone).copied().flatten()
    }

    /// The model info block.
    pub fn info(&self) -> &ModelInfo {
        &self.info
    }

    /// The last pose (or blend) output.
    pub fn current(&self) -> &Pose {
        &self.pose
    }

    /// Whether the current motion poses this bone (its flag bit 26 is clear).
    pub fn is_posed(&self, bone: usize) -> bool {
        self.flags.get(bone).is_some_and(|f| f & flag::SKIP == 0)
    }

    /// The current motion's bone flags (look-at bone's bit 25 cleared).
    pub fn bone_flags(&self) -> &[u32] {
        &self.flags
    }

    /// The helper world matrices of the last pose (`H[0]` = the normalised model matrix with
    /// rows 1, 2 negated, translation removed).
    pub fn helpers(&self) -> &[Affine] {
        &self.h
    }

    /// Local scale / rotation / translation of a bone as the last pose left them (instance
    /// locals +0x00, +0x20, +0x30), plus the accumulated scale (+0x10).
    pub fn locals(&self, bone: usize) -> Option<(Vec3, Vec3, Vec3, Vec3)> {
        (bone < self.n).then(|| (self.s[bone], self.r[bone], self.t[bone], self.acc[bone]))
    }

    /// `FUN_14038F550`, the motion change: locals = the mdls rest values, then the motion's
    /// starting-pose table (rotations not wrapped), then `L = RT(rot, trans)` for every bone
    /// with flag bit 25 set and bit 26 clear. Also ends any blend (`FUN_140393F60`).
    pub fn start_motion(&mut self, m: &Motion) {
        self.blend = None;
        self.motion_change(m);
    }

    pub(crate) fn motion_change(&mut self, m: &Motion) {
        self.s.copy_from_slice(&self.rest_s);
        self.r.copy_from_slice(&self.rest_r);
        self.t.copy_from_slice(&self.rest_t);
        for iv in &m.initial_pose {
            self.put(iv.bone as usize, iv.component as u8, iv.value);
        }
        for i in 0..self.n {
            self.flags[i] = m.bone_flags.get(i).map_or(flag::SKIP, |f| f.0);
        }
        if let Some(b) = self.info.look_at_bone.filter(|&b| b < self.n) {
            self.flags[b] &= !flag::NOT_REBUILT;
        }
        for i in 0..self.n {
            let f = self.flags[i];
            if f & flag::SKIP == 0 && f & flag::NOT_REBUILT != 0 {
                self.pose.local[i] = mat_rt(self.r[i], self.t[i]);
            }
        }
        self.cur = Some(motion_id(m));
    }

    #[inline]
    fn put(&mut self, bone: usize, component: u8, v: f32) {
        if bone >= self.n {
            return;
        }
        match component {
            1..=3 => self.s[bone][component as usize - 1] = v,
            4..=6 => self.r[bone][component as usize - 4] = v,
            7..=9 => self.t[bone][component as usize - 7] = v,
            _ => {}
        }
    }

    /// Poses `m` at time `t` (60 Hz frames) with an identity model matrix.
    pub fn pose(&mut self, m: &Motion, t: f32) -> &Pose {
        self.pose_in(m, t, &Affine::IDENTITY)
    }

    /// Poses `m` at time `t` (60 Hz frames) under model matrix `model` (KH1's actor matrix,
    /// row vectors, Y up; [`Affine::IDENTITY`] for model space). A different motion than the
    /// last one triggers the motion change ([`Rig::start_motion`]); motions are told apart by
    /// address and file offset. An active blend ends first as at KH1's blend end
    /// (`FUN_140393F60` drops the snapshots and re-runs the motion change).
    pub fn pose_in(&mut self, m: &Motion, t: f32, model: &Affine) -> &Pose {
        if self.blend.take().is_some() {
            self.motion_change(m);
        }
        if self.cur != Some(motion_id(m)) {
            self.motion_change(m);
        }
        self.pose_core(m, t, model);
        &self.pose
    }

    /// H0 and the model scale/translation from a model matrix (2.1 step 1 and 4).
    pub(crate) fn model_h0(model: &Affine) -> (Affine, Affine, Vec3, Vec3) {
        let sc = [len(model.r[0]), len(model.r[1]), len(model.r[2])];
        let mut full = *model;
        for r in 1..3 {
            for v in full.r[r].iter_mut() {
                *v = -*v;
            }
        }
        let mut h0 = full;
        for (r, row) in h0.r.iter_mut().enumerate() {
            for v in row.iter_mut() {
                *v /= sc[r];
            }
        }
        let mt = h0.t;
        h0.t = [0.0; 3];
        (h0, full, sc, mt)
    }

    /// `FUN_140391C70` after the motion-change check.
    pub(crate) fn pose_core(&mut self, m: &Motion, t: f32, model: &Affine) {
        // Bone channels (rotations wrapped).
        for ch in &m.channels {
            if let Some(v) = m.channel_value(ch, t) {
                self.put(ch.target as usize, ch.component(), v);
            }
        }
        // Helper channels.
        self.helpers.clear();
        self.helpers.extend(m.helpers.iter().map(|h| HelperRec { scale: h.scale, parent: h.parent, rot: h.rot, trans: h.trans }));
        for ch in &m.helper_channels {
            let (Some(v), Some(h)) = (m.channel_value(ch, t), self.helpers.get_mut(ch.target as usize)) else { continue };
            let c = ch.component() as usize;
            match c {
                1..=3 => h.scale[c - 1] = v,
                4..=6 => h.rot[c - 4] = v,
                7..=9 => h.trans[c - 7] = v,
                _ => {}
            }
        }
        // H0 and the helper chain.
        let (h0, full, sc, mt) = Rig::model_h0(model);
        self.h0_full = full;
        let nh = self.helpers.len().max(1);
        self.h.clear();
        self.h.push(h0);
        for k in 1..nh {
            let hr = self.helpers[k];
            let p = hr.parent;
            let base = if p >= 0 && (p as usize) < k { self.h[p as usize] } else { h0 };
            self.h.push(mat_srt(hr.scale, hr.rot, hr.trans).mul(&base));
        }
        let last = self.h[nh - 1].t;
        self.pose.root_travel = mulv(sc, last);
        self.bone_pass();
        self.pose.flags = m.flag_bits(t);
        if sc.iter().any(|s| (s - 1.0).abs() > 0.005) {
            for i in 0..self.n {
                if self.flags[i] & flag::SKIP == 0 {
                    self.pose.world[i] = self.pose.world[i].scale_cols(sc);
                }
            }
        }
        for w in self.pose.world.iter_mut() {
            w.t = add(w.t, mt);
        }
        self.pose.travel_and_model = add(mt, self.pose.root_travel);
    }

    /// Helper `H[k − 1]` for a 1-based index (identity for 0 or out of range).
    #[inline]
    pub(crate) fn helper1(&self, k: usize) -> Affine {
        if k == 0 {
            return Affine::IDENTITY;
        }
        self.h.get(k - 1).copied().unwrap_or(Affine::IDENTITY)
    }

    /// `FUN_1403900B0`.
    fn bone_pass(&mut self) {
        let n = self.n;
        let h0 = self.h[0];
        let mut i = 0;
        while i < n {
            let v = self.flags[i];
            if v & flag::SKIP != 0 {
                i += 1;
                continue;
            }
            let p = self.parent[i];
            let (pw, up) = match p {
                None => (h0, h0),
                Some(p) => (self.pose.world[p], self.u[p]),
            };
            self.acc[i] = match p {
                None => self.s[i],
                Some(p) => mulv(self.acc[p], self.s[i]),
            };
            self.u[i] = Affine::IDENTITY;
            let ik = v & flag::IK_MASK;
            let mut nxt = i;
            if ik == 0x20_0000 && i + 1 < n {
                self.one_bone_aim(i, v, &pw);
            } else if ik != 0 && ik != 0x20_0000 && i + 2 < n {
                self.two_bone_ik(i, v, &pw);
                nxt = i + 1;
            } else if ik == 0 {
                self.plain(i, v, &pw, &up, &h0);
            }
            self.u[i].t = [0.0; 3];
            i = nxt + 1;
        }
    }

    /// 3a/3b: FK, aim, copy orientation, helper position.
    fn plain(&mut self, i: usize, v: u32, pw: &Affine, up: &Affine, h0: &Affine) {
        let hp = hpos_of(v);
        let am = aim_of(v);
        let keep = v & flag::KEEP_IK_POS != 0;
        let pinv = if keep || hp != 0 || v & 0x100_0003 != 0 { pw.inverse_kh1() } else { Affine::IDENTITY };
        let pos;
        if !keep {
            if hp != 0 {
                pos = self.helper1(hp).t;
                self.pose.local[i].t = pinv.point(pos);
            } else {
                pos = pw.point(self.t[i]);
            }
        } else {
            pos = self.pose.world[i].t;
            self.pose.local[i].t = pinv.point(pos);
        }
        let mode = v & 3;
        match mode {
            0 => {
                if v & flag::MODEL_ROT == 0 {
                    if v & flag::NOT_REBUILT == 0 {
                        self.pose.local[i] = mat_rt(self.r[i], self.t[i]);
                    }
                    self.u[i] = self.pose.local[i].mul(up);
                } else {
                    self.pose.local[i] = mat_rt(self.r[i], self.t[i]);
                    self.u[i] = self.pose.local[i].mul(h0);
                }
                self.pose.world[i] = self.u[i].scale_rows(self.acc[i]);
            }
            1 => {
                let tgt = pinv.point(self.helper1(am).t);
                let d = if !keep && hp == 0 {
                    self.pose.local[i].t = self.t[i];
                    sub(tgt, self.t[i])
                } else {
                    sub(tgt, self.pose.local[i].t)
                };
                let x = norm(d);
                let zp = cross(d, [0.0, 1.0, 0.0]);
                let y = norm(cross(zp, d));
                let z = norm(cross(d, y));
                let mut l = self.pose.local[i];
                l.r = [x, y, z];
                l = rx(self.r[i][0]).mul(&l);
                self.pose.local[i] = l;
                self.u[i] = l.mul(up);
                self.pose.world[i] = self.u[i].scale_rows(self.acc[i]);
            }
            2 => {
                let ha = self.helper1(am);
                self.u[i] = Affine { r: [norm(ha.r[0]), norm(ha.r[1]), norm(ha.r[2])], t: [0.0; 3] };
                self.pose.world[i] = self.u[i].scale_rows(self.acc[i]);
            }
            _ => {}
        }
        self.pose.world[i].t = pos;
        if mode != 0 || v & flag::MODEL_ROT != 0 {
            self.pose.local[i] = self.pose.world[i].mul(&pinv).unscale().0;
        }
    }

    /// 3c: one-bone aim (`0x200000`); bone i+1 is processed normally afterwards.
    fn one_bone_aim(&mut self, i: usize, v: u32, pw: &Affine) {
        self.pose.local[i].t = self.t[i];
        let pinv = pw.inverse_kh1();
        let goal = pinv.point(self.helper1(hpos_of(self.flags[i + 1])).t);
        let pole = (v & 3 == 3).then(|| pinv.point(self.helper1(aim_of(v)).t));
        let l1 = self.rest_t[i + 1][0] * self.s[i][0];
        let mut lu = self.pose.local[i];
        ik_solve(goal, l1, 0.0, false, &mut lu, None, self.r[i][0], 0.0, pole);
        self.pose.local[i] = lu;
        let mut w = lu.scale_rows(self.s[i]).mul(pw);
        w.t = pw.point(self.t[i]);
        self.pose.world[i] = w;
        (self.u[i], self.acc[i]) = w.unscale();
        self.pose.world[i + 1].t = w.point(self.rest_t[i + 1]);
    }

    /// 3d: two-bone IK over i, i+1 (effector i+2).
    fn two_bone_ik(&mut self, i: usize, v: u32, pw: &Affine) {
        let flip = ((v >> 21) & 3) == 3;
        self.pose.local[i].t = self.t[i];
        let pinv = pw.inverse_kh1();
        let goal = pinv.point(self.helper1(hpos_of(self.flags[i + 2])).t);
        let pole = (v & 3 == 3).then(|| pinv.point(self.helper1(aim_of(v)).t));
        let (s0, s1) = (self.s[i], self.s[i + 1]);
        let l1 = self.rest_t[i + 1][0] * s0[0];
        let l2 = s1[0] * s0[0] * self.rest_t[i + 2][0];
        let mut lu = self.pose.local[i];
        let mut ll = self.pose.local[i + 1];
        ik_solve(goal, l1, l2, flip, &mut lu, Some(&mut ll), self.r[i][0], self.r[i + 1][0], pole);
        self.pose.local[i] = lu;
        let mut wi = lu.scale_rows(s0).mul(pw);
        wi.t = pw.point(self.t[i]);
        self.pose.world[i] = wi;
        (self.u[i], self.acc[i]) = wi.unscale();
        self.acc[i + 1] = mulv(s0, s1);
        let m2 = ll.scale_cols([1.0 / s0[0], 1.0 / s0[1], 1.0 / s0[2]]).scale_rows(self.acc[i + 1]);
        let mut w1 = m2.mul(&wi);
        w1.t = wi.point(self.rest_t[i + 1]);
        self.pose.world[i + 1] = w1;
        (self.u[i + 1], self.acc[i + 1]) = w1.unscale();
        self.pose.world[i + 2].t = w1.point(self.rest_t[i + 2]);
        self.pose.local[i + 1] = w1.mul(&wi.inverse_kh1());
        self.tw[i + 1] = self.r[i + 1][0];
    }

    /// `FUN_1401D6320(inst, bone, t)`: the point at fraction `t` from the bone's parent
    /// (`t = 0`) to the bone (`t = 1`, the bone's current local translation through the parent
    /// world). A root bone gives its own world position.
    pub fn point_on_bone(&self, bone: usize, t: f32) -> Vec3 {
        let w = &self.pose.world;
        match self.parent(bone) {
            None => w[bone].t,
            Some(p) => {
                let tip = w[p].point(self.t[bone]);
                crate::math::lerp(w[p].t, tip, t)
            }
        }
    }

    /// `FUN_14029FAB0`: the bone an attach id names (see [`ModelInfo::attach_bone`]).
    pub fn attach_bone(&self, id: u16) -> Option<usize> {
        self.info.attach_bone(id, self.n)
    }

    /// The angle wrap used by the evaluator (exposed for converters).
    pub fn wrap_angle(v: f32) -> f32 {
        wrap(v)
    }
}

/// `FUN_14038FC20(goal, L1, L2, flip, Lu, Ll, tw1, tw2, pole)`: everything in the parent space
/// of the upper bone; `lu.t` is the joint origin and is kept. `ll = None` is the one-bone aim.
#[allow(clippy::too_many_arguments)]
pub(crate) fn ik_solve(goal: Vec3, l1: f32, l2: f32, flip: bool, lu: &mut Affine, ll: Option<&mut Affine>, tw1: f32, tw2: f32, pole: Option<Vec3>) {
    let o = lu.t;
    let d0 = sub(goal, o);
    let dist = len(d0);
    let d = norm(d0);
    let (y, z) = match pole {
        None => {
            let y = norm([-d[1], d[0], 0.0]);
            (y, norm(cross(d, y)))
        }
        Some(p) => {
            let pp = norm(sub(p, o));
            let z = norm(cross(d, pp));
            (norm(cross(z, d)), z)
        }
    };
    let mut m = Affine { r: [d, y, z], t: [0.0; 3] };
    if tw1 != 0.0 {
        m = rx(tw1).mul(&m);
    }
    match ll {
        None => *lu = m,
        Some(ll) => {
            // f32::max/min ignore NaN like the exe's compare-and-select (clamp would keep it).
            #[allow(clippy::manual_clamp)]
            let c = ((dist * dist - l1 * l1 - l2 * l2) / ((l1 + l1) * l2)).max(-1.0).min(1.0);
            let mut s = (1.0 - c * c).max(0.0).abs().sqrt();
            if flip {
                s = -s;
            }
            let mut cm = Affine { r: [[c, s, 0.0], [-s, c, 0.0], [0.0, 0.0, 1.0]], t: [l1, 0.0, 0.0] };
            if tw2 != 0.0 {
                cm = rx(tw2).mul(&cm);
            }
            *ll = cm;
            let a = (c * l2 + l1) * dist;
            let b = -(s * l2 * dist);
            let nn = (b * b + a * a).abs().sqrt();
            let (a, b) = (a / nn, b / nn);
            let rz = Affine { r: [[a, b, 0.0], [-b, a, 0.0], [0.0, 0.0, 1.0]], t: [0.0; 3] };
            *lu = rz.mul(&m);
        }
    }
    lu.t = o;
}
