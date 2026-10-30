//! Cross-fades between two **frozen** snapshots (`kh1-posing.md` §5; Checked (decomp)):
//! - `FUN_140394080` takes the snapshots (old pose, then the new motion posed at its start frame);
//! - `FUN_140393B30` does it again while already blending: the old snapshot is then the
//!   previous blend's *new* snapshot, not the blended pose on screen;
//! - `FUN_140392560` poses one blended frame with weight `w` of the new pose.
//!
//! Rows of the locals (relative to the nearest blend-bone ancestor, mdls link bit 30) are lerped
//! and Gram-Schmidt re-orthonormalised; translations and scales lerp; the foot IK (the bones of
//! the model's foot list whose IK type is the same in both motions) is re-solved toward a goal
//! lerped in model space, with a pole that lerps the knee plane. Bones that are neither keep the
//! new motion's start-frame pose (Sora: 0, 16, 19, 101 = `WeaponR`, …).
//!
//! The model-specific extra IK list (`*model + (model[+0xC] + 1) * 0x10`) is empty for Sora
//! (Checked (files)) and not read here.

use kh1_formats::mset::Motion;

use crate::math::{add, cross, lerp, mulv, norm, wrap, Affine, Vec3};
use crate::rig::{ik_solve, Pose, Rig};

#[derive(Clone, Debug)]
pub(crate) struct IkEntry {
    pub bone: usize,
    /// IK type 1..=3 (`(flags >> 21) & 3`).
    pub kind: u32,
    pub old_goal: Vec3,
    pub new_goal: Vec3,
}

impl IkEntry {
    fn effector(&self) -> usize {
        self.bone + if self.kind == 1 { 1 } else { 2 }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct BlendState {
    pub l_old: Vec<Affine>,
    pub l_new: Vec<Affine>,
    /// `{S, twist}` per bone.
    pub s4_old: Vec<[f32; 4]>,
    pub s4_new: Vec<[f32; 4]>,
    pub ik: Vec<IkEntry>,
    pub root_old: Vec3,
    pub root_new: Vec3,
    /// Link bit 30 with the IK bones' parents set (scratch, rebuilt each frame).
    pub flag30: Vec<bool>,
}

impl Rig {
    fn s4(&self) -> Vec<[f32; 4]> {
        self.s.iter().zip(&self.tw).map(|(s, &t)| [s[0], s[1], s[2], t]).collect()
    }

    /// Whether a blend is in progress.
    pub fn is_blending(&self) -> bool {
        self.blend.is_some()
    }

    /// Starts a blend into `new` (posed once at `t_start`, then frozen) and poses the first
    /// blended frame with weight `w0` (KH1: `step · rate / blend length`). Identity model.
    pub fn blend_start(&mut self, new: &Motion, t_start: f32, w0: f32) -> &Pose {
        self.blend_start_in(new, t_start, w0, &Affine::IDENTITY)
    }

    /// [`Rig::blend_start`] under a model matrix. With no motion posed yet this poses `new`
    /// at once, as `FUN_1402A0350` does. Deciding whether to blend at all (KH1: only when the
    /// blend length exceeds one frame step and the actor allows it) is the caller's.
    pub fn blend_start_in(&mut self, new: &Motion, t_start: f32, w0: f32, model: &Affine) -> &Pose {
        if self.cur.is_none() {
            return self.pose_in(new, t_start, model);
        }
        let state = match self.blend.take() {
            None => {
                // FUN_140394080
                let hci = self.h0_full.inverse_kh1();
                let mut ik = Vec::new();
                for &b in &self.info.foot_ik_bones {
                    if b + 2 >= self.n {
                        continue;
                    }
                    let a = (self.flags[b] >> 21) & 3;
                    let c = new.bone_flags.get(b).map_or(0, |f| (f.0 >> 21) & 3);
                    if a != 0 && a == c {
                        let mut e = IkEntry { bone: b, kind: a, old_goal: [0.0; 3], new_goal: [0.0; 3] };
                        e.old_goal = hci.point(self.pose.world[e.effector()].t);
                        ik.push(e);
                    }
                }
                BlendState {
                    l_old: self.pose.local.clone(),
                    l_new: Vec::new(),
                    s4_old: self.s4(),
                    s4_new: Vec::new(),
                    ik,
                    root_old: self.pose.root_travel,
                    root_new: [0.0; 3],
                    flag30: self.link30.clone(),
                }
            }
            Some(prev) => {
                // FUN_140393B30
                let mut ik = prev.ik;
                ik.retain_mut(|e| {
                    let ty = new.bone_flags.get(e.bone).map_or(0, |f| (f.0 >> 21) & 3);
                    if ty == 0 || ty != e.kind {
                        false
                    } else {
                        e.old_goal = e.new_goal;
                        true
                    }
                });
                BlendState {
                    l_old: prev.l_new,
                    l_new: Vec::new(),
                    s4_old: prev.s4_new,
                    s4_new: Vec::new(),
                    ik,
                    root_old: prev.root_new,
                    root_new: [0.0; 3],
                    flag30: prev.flag30,
                }
            }
        };
        let mut state = state;
        self.motion_change(new);
        self.pose_core(new, t_start, model);
        state.l_new = self.pose.local.clone();
        state.s4_new = self.s4();
        state.root_new = self.pose.root_travel;
        let hci = self.h0_full.inverse_kh1();
        for e in state.ik.iter_mut() {
            e.new_goal = hci.point(self.pose.world[e.effector()].t);
        }
        self.blend = Some(state);
        self.blend_in(w0, model)
    }

    /// One blended frame with weight `w` of the new pose (identity model). Does nothing but
    /// return the last pose when no blend is active.
    pub fn blend(&mut self, w: f32) -> &Pose {
        self.blend_in(w, &Affine::IDENTITY)
    }

    /// `FUN_140392560(model, inst, w)`.
    pub fn blend_in(&mut self, w: f32, model: &Affine) -> &Pose {
        let Some(mut b) = self.blend.take() else { return &self.pose };
        self.blend_frame(&mut b, w, model);
        self.blend = Some(b);
        &self.pose
    }

    fn blend_frame(&mut self, b: &mut BlendState, w: f32, model: &Affine) {
        let n = self.n;
        let (h0, _full, sc, mt) = Rig::model_h0(model);
        let root = lerp(b.root_old, b.root_new, w);
        self.pose.root_travel = root;
        self.pose.travel_and_model = add(mulv(sc, root), mt);
        b.flag30.copy_from_slice(&self.link30);
        for e in &b.ik {
            if let Some(p) = self.parent[e.bone] {
                b.flag30[p] = true;
            }
        }
        let mut i = 0;
        while i < n {
            let s4 = lerp4(b.s4_old[i], b.s4_new[i], w);
            self.s[i] = [s4[0], s4[1], s4[2]];
            self.tw[i] = s4[3];
            let p = self.parent[i];
            self.acc[i] = match p {
                None => self.s[i],
                Some(p) => mulv(self.acc[p], self.s[i]),
            };
            let mut nxt = i;
            if let Some(e) = b.ik.iter().find(|e| e.bone == i) {
                let typ = e.kind;
                let goal = h0.point(lerp(e.old_goal, e.new_goal, w));
                self.pose.local[i].t = self.t[i];
                let pole = add(lerp(b.l_old[i].r[1], b.l_new[i].r[1], w), self.t[i]);
                let pw = p.map_or(h0, |p| self.pose.world[p]);
                let pinv = pw.inverse_kh1();
                let gl = pinv.point(goal);
                let mut ub = self.pose.local[i];
                if typ == 1 || i + 2 >= n {
                    ik_solve(gl, self.rest_t[i + 1][0] * self.s[i][0], 0.0, false, &mut ub, None, 0.0, 0.0, Some(pole));
                } else {
                    let (o, nw) = (b.s4_old[i + 1], b.s4_new[i + 1]);
                    let d = wrap(nw[3] - o[3]);
                    let s1 = lerp([o[0], o[1], o[2]], [nw[0], nw[1], nw[2]], w);
                    self.s[i + 1] = s1;
                    let tw = wrap(o[3] + w * d);
                    self.tw[i + 1] = tw;
                    let l1 = self.rest_t[i + 1][0] * self.s[i][0];
                    let l2 = self.s[i][0] * s1[0] * self.rest_t[i + 2][0];
                    let mut ub1 = self.pose.local[i + 1];
                    ik_solve(gl, l1, l2, typ == 3, &mut ub, Some(&mut ub1), 0.0, tw, Some(pole));
                    self.pose.local[i + 1] = ub1;
                }
                let si = self.s[i];
                let mut wi = ub.scale_rows(si).mul(&pw);
                wi.t = pw.point(self.t[i]);
                self.pose.world[i] = wi;
                (self.pose.local[i], self.acc[i]) = wi.unscale();
                if typ != 1 && i + 2 < n {
                    let m2 = self.pose.local[i + 1].scale_cols([1.0 / si[0], 1.0 / si[1], 1.0 / si[2]]);
                    self.acc[i + 1] = mulv(si, self.s[i + 1]);
                    let mut w1 = m2.scale_rows(self.acc[i + 1]).mul(&wi);
                    w1.t = wi.point(self.rest_t[i + 1]);
                    self.pose.world[i + 1] = w1;
                    (self.pose.local[i + 1], self.acc[i + 1]) = w1.unscale();
                    nxt = i + 1;
                }
            } else if b.flag30[i] {
                let mut a = b.l_old[i];
                let mut c = b.l_new[i];
                let mut q = p;
                while let Some(qq) = q {
                    if b.flag30[qq] {
                        break;
                    }
                    a = a.mul(&b.l_old[qq]);
                    c = c.mul(&b.l_new[qq]);
                    q = self.parent[qq];
                }
                let tl = lerp(a.t, c.t, w);
                let x = norm(lerp(a.r[0], c.r[0], w));
                let y = norm(lerp(a.r[1], c.r[1], w));
                let z = norm(cross(x, y));
                let y = norm(cross(z, x));
                let mb = Affine { r: [x, y, z], t: tl };
                let (base_u, base_w) = match q {
                    None => (h0, h0),
                    Some(q) => (self.pose.local[q], self.pose.world[q]),
                };
                let ub = mb.mul(&base_u);
                self.pose.local[i] = ub;
                let mut wi = ub.scale_rows(self.acc[i]);
                wi.t = base_w.point(tl);
                self.pose.world[i] = wi;
            }
            i = nxt + 1;
        }
        if sc.iter().any(|s| (s - 1.0).abs() > 0.005) {
            for wm in self.pose.world.iter_mut() {
                *wm = wm.scale_cols(sc);
            }
        }
        for wm in self.pose.world.iter_mut() {
            wm.t = add(wm.t, mt);
        }
    }
}

fn lerp4(a: [f32; 4], b: [f32; 4], w: f32) -> [f32; 4] {
    let u = 1.0 - w;
    [u * a[0] + w * b[0], u * a[1] + w * b[1], u * a[2] + w * b[2], u * a[3] + w * b[3]]
}
