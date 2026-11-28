//! KH2's interpolated motions (the `.mset` of a KH2 model), read and sampled for the bones they animate (0.195.0, the KH2 hair's
//! own motion on Re:CoM's hair bones; fork owner, 2026-11-24: "A", the 18 hair bones re-posed on KH2's).
//!
//! Layout (OpenKH `docs/kh2/file/anb/motion.md` and `OpenKh.Kh2/Motion.cs`, `InterpolatedMotion`): an `.mset` is a BAR whose
//! entries are ANBs (BARs); an ANB's entry of type 9 is the motion: `0x90` reserved bytes, a 16-byte header (type 0 =
//! interpolated), then the interpolated header (offsets from `0x90`): the initial pose (`bone u16, channel u16, value f32`), the
//! forward F-curves (`joint u16, channel 4 bits | pre 2 | post 2, key count u8, first key u16`), the keys (`interpolation 2 bits
//! | time index 14 bits, value index, tangent-in index, tangent-out index`, all u16), key times / values / tangents (f32).
//! Channels 0..8 = scale, rotation, translation x / y / z. A bone's local transform is the model's bind values with the initial
//! pose and the curves' values put over them, composed as the model's bind pose is (`kh2_formats_lite::skeleton`).
//!
//! What is read: the forward curves of the model's bones only. The inverse (IK helper) curves, constraints and expressions are
//! not: the hair's bones have no constraint (Assumption, checked by comparing with OpenKH's emulator on the hair bones, the
//! test [`tests::matches_openkh_emulator_on_the_hair`]).

use kh2_formats_lite::bar::Bar;
use kh2_formats_lite::skeleton::{self, Bone, Mat};

use crate::{bad, Error, Result};

/// One key of a curve.
#[derive(Clone, Copy, Debug)]
pub struct Key {
    /// 0 constant, 1 linear, 2 Hermite (3 is read as Hermite too).
    pub interp: u8,
    /// Time in the motion's frames.
    pub time: f32,
    /// Value.
    pub value: f32,
    /// Tangent on the left (ease in).
    pub tin: f32,
    /// Tangent on the right (ease out).
    pub tout: f32,
}

/// One forward F-curve.
#[derive(Clone, Debug)]
pub struct Curve {
    /// Bone index.
    pub joint: usize,
    /// 0..8 (scale, rotation, translation x / y / z).
    pub channel: u8,
    /// Cycle before the first key (0 first key, 1 subtractive, 2 repeat, 3 zero).
    pub pre: u8,
    /// Cycle after the last key.
    pub post: u8,
    /// Keys in time order.
    pub keys: Vec<Key>,
}

/// One interpolated motion.
#[derive(Clone, Debug)]
pub struct Kh2Motion {
    /// Bones the motion is made for.
    pub bones: usize,
    /// `(bone, channel, value)`.
    pub initial: Vec<(usize, u8, f32)>,
    /// Forward curves.
    pub curves: Vec<Curve>,
    /// Frame start / end / return (loop start), frames per second.
    pub frame_start: f32,
    /// See `frame_start`.
    pub frame_end: f32,
    /// See `frame_start`.
    pub frame_return: f32,
    /// See `frame_start`.
    pub fps: f32,
}

fn u16_at(b: &[u8], o: usize) -> Result<u16> {
    b.get(o..o + 2).map(|x| u16::from_le_bytes([x[0], x[1]])).ok_or_else(|| Error::Format("KH2 motion: truncated".into()))
}
fn i32_at(b: &[u8], o: usize) -> Result<i32> {
    b.get(o..o + 4).map(|x| i32::from_le_bytes(x.try_into().unwrap())).ok_or_else(|| Error::Format("KH2 motion: truncated".into()))
}
fn f32_at(b: &[u8], o: usize) -> Result<f32> {
    Ok(f32::from_bits(i32_at(b, o)? as u32))
}

/// The motion of `.mset` entry `slot` (none: an empty or dummy slot, or a raw motion).
pub fn read_slot(mset: &[u8], slot: usize) -> Result<Option<Kh2Motion>> {
    let bar = Bar::parse(mset).map_err(|e| Error::Format(format!("KH2 mset: {e}")))?;
    let Some(e) = bar.entries.get(slot) else { return bad(format!("KH2 mset: no slot {slot}")) };
    if e.data.is_empty() {
        return Ok(None);
    }
    let anb = Bar::parse(e.data).map_err(|x| Error::Format(format!("KH2 anb {slot}: {x}")))?;
    let Some(m) = anb.entries.iter().find(|x| x.kind == 9) else { return Ok(None) };
    parse(m.data).map(Some).or_else(|x| if matches!(x, Error::Format(ref s) if s == "raw") { Ok(None) } else { Err(x) })
}

/// Parses one motion entry (interpolated only).
pub fn parse(b: &[u8]) -> Result<Kh2Motion> {
    const B: usize = 0x90;
    if i32_at(b, B)? != 0 {
        return Err(Error::Format("raw".into()));
    }
    let h = B + 0x10;
    let at = |o: usize| -> Result<usize> { Ok(B + i32_at(b, h + o)? as usize) };
    let bones = u16_at(b, h)? as usize;
    let key_time_count = i32_at(b, h + 0x10)? as usize;
    let (ip, ipn) = (at(0x14)?, i32_at(b, h + 0x18)? as usize);
    let (fc, fcn) = (at(0x20)?, i32_at(b, h + 0x24)? as usize);
    let (keys, times, values, tangents) = (at(0x30)?, at(0x34)?, at(0x38)?, at(0x3C)?);
    let mut initial = Vec::with_capacity(ipn);
    for i in 0..ipn {
        let o = ip + 8 * i;
        initial.push((u16_at(b, o)? as usize, u16_at(b, o + 2)? as u8, f32_at(b, o + 4)?));
    }
    let time = |i: usize| -> Result<f32> {
        if i >= key_time_count {
            return bad(format!("KH2 motion: key time {i} of {key_time_count}"));
        }
        f32_at(b, times + 4 * i)
    };
    let mut curves = Vec::with_capacity(fcn);
    for i in 0..fcn {
        let o = fc + 6 * i;
        let joint = u16_at(b, o)? as usize;
        let ch = *b.get(o + 2).ok_or_else(|| Error::Format("KH2 motion: truncated".into()))?;
        let n = *b.get(o + 3).ok_or_else(|| Error::Format("KH2 motion: truncated".into()))? as usize;
        let first = u16_at(b, o + 4)? as usize;
        let mut ks = Vec::with_capacity(n);
        for k in first..first + n {
            let q = keys + 8 * k;
            let tt = u16_at(b, q)?;
            let (vi, ti, to) = (u16_at(b, q + 2)? as usize, u16_at(b, q + 4)? as usize, u16_at(b, q + 6)? as usize);
            ks.push(Key { interp: (tt & 3) as u8, time: time((tt >> 2) as usize)?, value: f32_at(b, values + 4 * vi)?, tin: f32_at(b, tangents + 4 * ti)?, tout: f32_at(b, tangents + 4 * to)? });
        }
        curves.push(Curve { joint, channel: ch & 0xF, pre: (ch >> 4) & 3, post: (ch >> 6) & 3, keys: ks });
    }
    Ok(Kh2Motion {
        bones,
        initial,
        curves,
        frame_start: f32_at(b, h + 0x80)?,
        frame_end: f32_at(b, h + 0x84)?,
        fps: f32_at(b, h + 0x88)?,
        frame_return: f32_at(b, h + 0x8C)?,
    })
}

impl Curve {
    /// The curve's value at time `t` (motion frames).
    pub fn eval(&self, t: f32) -> f32 {
        let k = &self.keys;
        match k.len() {
            0 => return 0.0,
            1 => return k[0].value,
            _ => {}
        }
        let (t0, t1) = (k[0].time, k[k.len() - 1].time);
        let span = t1 - t0;
        // cycles: repeat (2) wraps the time; first / last key (0) and the others hold the end value
        let mut t = t;
        let mut offset = 0.0f32;
        if t < t0 || t > t1 {
            let mode = if t < t0 { self.pre } else { self.post };
            match mode {
                1 | 2 if span > 0.0 => {
                    let n = ((t - t0) / span).floor();
                    t -= n * span;
                    if mode == 1 {
                        offset = n * (k[k.len() - 1].value - k[0].value);
                    }
                }
                3 => return 0.0,
                _ => return if t < t0 { k[0].value } else { k[k.len() - 1].value },
            }
        }
        let i = k.iter().rposition(|x| x.time <= t).unwrap_or(0).min(k.len() - 2);
        let (a, b) = (&k[i], &k[i + 1]);
        let dt = b.time - a.time;
        if dt <= 0.0 {
            return a.value + offset;
        }
        let s = ((t - a.time) / dt).clamp(0.0, 1.0);
        offset
            + match a.interp {
                0 => a.value,
                1 => a.value + (b.value - a.value) * s,
                _ => {
                    let (s2, s3) = (s * s, s * s * s);
                    (2.0 * s3 - 3.0 * s2 + 1.0) * a.value + (s3 - 2.0 * s2 + s) * a.tout * dt + (-2.0 * s3 + 3.0 * s2) * b.value + (s3 - s2) * b.tin * dt
                }
            }
    }
}

impl Kh2Motion {
    /// Key-time units per motion frame. The keys' times are in 60 Hz units while the frame start / end / return are in the motion's
    /// own frames (`fps`): `P_EH000`'s idle ends at frame 120 at 30 fps and its hair keys run 0..240. Checked (files; 0.195.2:
    /// 0.195.0 / 0.195.1 sampled the keys in motion frames, i.e. the first half of each motion at half speed, and every loop
    /// jumped back: the hair's seam was 6.4 KH2 units in the idle against 0.58 per frame).
    pub fn keys_per_frame(&self) -> f32 {
        if self.fps > 0.0 { 60.0 / self.fps } else { 1.0 }
    }

    /// The model's bones at motion frame `frame` (the motion's `fps`).
    pub fn locals(&self, bind: &[Bone], frame: f32) -> Vec<Bone> {
        self.locals_key(bind, frame * self.keys_per_frame())
    }

    /// The model's bones with this motion's values at key time `t` (60 Hz units) put over the bind values.
    pub fn locals_key(&self, bind: &[Bone], t: f32) -> Vec<Bone> {
        let mut out = bind.to_vec();
        let set = |out: &mut Vec<Bone>, j: usize, ch: u8, v: f32| {
            let Some(bn) = out.get_mut(j) else { return };
            match ch {
                0..=2 => bn.scale[ch as usize] = v,
                3..=5 => bn.rotation[ch as usize - 3] = v,
                6..=8 => bn.translation[ch as usize - 6] = v,
                _ => {}
            }
        };
        for &(j, ch, v) in &self.initial {
            set(&mut out, j, ch, v);
        }
        for c in &self.curves {
            set(&mut out, c.joint, c.channel, c.eval(t));
        }
        out
    }

    /// World matrices of the model's bones at motion frame `frame` (model space, as [`skeleton::bind_pose`]).
    pub fn world(&self, bind: &[Bone], frame: f32) -> Vec<Mat> {
        skeleton::bind_pose(&self.locals(bind, frame))
    }

    /// World matrices at key time `t` (60 Hz units).
    pub fn world_key(&self, bind: &[Bone], t: f32) -> Vec<Mat> {
        skeleton::bind_pose(&self.locals_key(bind, t))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The modding workspace (env `KH_WORKSPACE`, else the owner's path): local test inputs and debug renders.
    pub(crate) fn workspace() -> String {
        std::env::var("KH_WORKSPACE").unwrap_or_else(|_| "/home/johnmoore/Projects/.own/KH-Modding-Workspace".into())
    }
    /// The workspace's extracted `P_EH000` files (local only; the tests are skipped without them).
    pub(crate) fn raw() -> String {
        format!("{}/local/assets/kh2/riku/raw", workspace())
    }
    /// OpenKH emulator dumps of `P_EH000` (first half of each motion; see `emu_full`).
    pub(crate) fn emu() -> String {
        format!("{}/local/assets/kh2/riku/emu", workspace())
    }
    /// OpenKH emulator dumps of whole motions.
    pub(crate) fn emu_full() -> String {
        format!("{}/local/assets/kh2/riku/emu_full", workspace())
    }

    fn inv_mul(a: &Mat, b: &Mat) -> Mat {
        // a^-1 · b for affine column matrices (a may be scaled)
        let m = [[a[0][0], a[1][0], a[2][0]], [a[0][1], a[1][1], a[2][1]], [a[0][2], a[1][2], a[2][2]]];
        let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]) + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
        let inv = [
            [(m[1][1] * m[2][2] - m[1][2] * m[2][1]) / det, (m[0][2] * m[2][1] - m[0][1] * m[2][2]) / det, (m[0][1] * m[1][2] - m[0][2] * m[1][1]) / det],
            [(m[1][2] * m[2][0] - m[1][0] * m[2][2]) / det, (m[0][0] * m[2][2] - m[0][2] * m[2][0]) / det, (m[0][2] * m[1][0] - m[0][0] * m[1][2]) / det],
            [(m[1][0] * m[2][1] - m[1][1] * m[2][0]) / det, (m[0][1] * m[2][0] - m[0][0] * m[2][1]) / det, (m[0][0] * m[1][1] - m[0][1] * m[1][0]) / det],
        ];
        let t = [a[3][0], a[3][1], a[3][2]];
        let it: [f64; 3] = core::array::from_fn(|r| -(0..3).map(|k| inv[r][k] * t[k]).sum::<f64>());
        let ai: Mat = [[inv[0][0], inv[1][0], inv[2][0], 0.0], [inv[0][1], inv[1][1], inv[2][1], 0.0], [inv[0][2], inv[1][2], inv[2][2], 0.0], [it[0], it[1], it[2], 1.0]];
        skeleton::mul(&ai, b)
    }

    /// 0.195.0: the hair bones of `P_EH000` relative to its head (98), sampled by this reader, against OpenKH's emulator
    /// (`OpenKh.Kh2AnimEmu`, the workspace's `local/tools/kh2anim`, which runs the PS2 code) on the idle, walk, run, jump, fall
    /// and land motions.
    #[test]
    fn matches_openkh_emulator_on_the_hair() {
        let Ok(mset) = std::fs::read(format!("{}/obj__P_EH000.mset", crate::kh2_motion::tests::raw())) else { return };
        let mdlx = std::fs::read(format!("{}/obj__P_EH000.mdlx", crate::kh2_motion::tests::raw())).unwrap();
        let model = kh2_formats_lite::mdlx::parse(&mdlx).unwrap();
        let hair: Vec<usize> = vec![101, 104, 106, 108, 110, 112, 114, 116, 118, 119, 121, 122, 124, 125, 127, 129, 131, 133, 135, 137, 139, 142, 144, 146, 148];
        let mut worst_all = 0.0f64;
        for slot in [0usize, 4, 8, 14, 18, 22] {
            let Ok(raw) = std::fs::read(format!("{}/slot{slot}.f32", crate::kh2_motion::tests::emu())) else { continue };
            let m = read_slot(&mset, slot).unwrap().unwrap();
            let fl: Vec<f32> = raw.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect();
            let per = (fl.len() / 16) / (m.frame_end.round() as usize + 1);
            let mut worst = (0.0f64, 0.0f64);
            for f in 0..=(m.frame_end.round() as usize) {
                let emu = |b: usize| -> Mat {
                    let o = (f * per + b) * 16;
                    core::array::from_fn(|i| core::array::from_fn(|j| fl[o + i * 4 + j] as f64))
                };
                // the emulator's sample index is a 60 Hz time (the CLI samples 0..frame_end of it: the first half at 30 fps)
                let ours = m.world_key(&model.bones, f as f32);
                for &b in &hair {
                    let (e, o) = (inv_mul(&emu(98), &emu(b)), inv_mul(&ours[98], &ours[b]));
                    for i in 0..3 {
                        for j in 0..3 {
                            worst.0 = worst.0.max((e[i][j] - o[i][j]).abs());
                        }
                        worst.1 = worst.1.max((e[3][i] - o[3][i]).abs());
                    }
                }
            }
            eprintln!("slot {slot}: frames 0..{} fps {} return {}: worst rotation element {:.5}, translation {:.4} (KH2 units)", m.frame_end, m.fps, m.frame_return, worst.0, worst.1);
            worst_all = worst_all.max(worst.0);
        }
        assert!(worst_all < 0.01, "hair rotations differ from the emulator by {worst_all}");
    }
}
