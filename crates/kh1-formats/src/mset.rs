//! KH1 motion sets (`.mset`), e.g. Sora's `xa_ex_0010.mset` (118 motions, 352 requests).
//!
//! Layout (offsets from the file start unless said; Checked (files), readers in the Steam exe
//! named per item, see `kh1-data-values.md` §4 and `motion-conversion.md` §4):
//! - +0x00 u32 section count (3), then u32 section offsets: motions (`"MMTN"`), the request map,
//!   an extra section (Sora's: 0x30 bytes, not decoded).
//! - motion section +0x30 u32: offset (from the section) of the entry table: u32 offsets of
//!   each motion relative to the table, ended by 0xFFFFFFFF.
//! - request map: one u32 per request id ([`MapEntry`]; resolver `0x1402A01E0`).
//! - motion header (every offset in it is relative to the motion start):
//!   +0x04 u32 frames, +0x08 f32 fps (60.0), +0x0C u32 (0/1, meaning not traced),
//!   +0x10 u32 bone count, +0x14 per-bone flags (u32 each, [`BoneFlags`]),
//!   +0x18/+0x1C bone channel count/table, +0x20/+0x24 helper channel count/table
//!   (6-byte [`Channel`]s), +0x28 keys (16-byte [`Key`]s), +0x2C/+0x30 helper count/table
//!   (0x28-byte [`Helper`]s), +0x34/+0x38 starting-pose count/table (8-byte [`InitialValue`]s),
//!   +0x3C flag-track list ([`FlagTrack`]).
//!
//! Sampling follows the motion evaluator `0x140391C70`: time in frames (1 per 60 Hz frame) is
//! taken as `int(t * 256)`, keys are found by `0x14038F820`'s search, and Hermite tangents are
//! per 1/256 frame (the x256 factor reproduces Re:CoM's PS2 bake of the same motions to 0.01 deg;
//! without it curves miss by up to 6 deg; Checked (files)). The interpolation body itself is cut
//! off in the decomp: hold / linear / Hermite by the key's interp byte is an Assumption carried
//! from the research parser and verified only through that bake comparison.

use crate::reader::Bytes;
use crate::{bad, Result};

/// Flag-track types (the `kind` of a [`FlagTrack`]), as `0x140391C70` / `0x140394900` read them.
pub mod track {
    /// Free: the actor may act again (bit [`super::bits::FREE`]); sub 0.
    pub const FREE: u16 = 0;
    /// Hit active for attack slot `sub` 0..=3 (bit [`super::bits::HIT0`] << sub).
    pub const HIT: u16 = 1;
    /// Read by `0x140394690` with any sub (type 2: its first key's byte +4 goes to the model);
    /// Sora's motions have none.
    pub const TYPE2: u16 = 2;
    /// Read by `0x140394690` with any sub; Sora's motions have none.
    pub const TYPE3: u16 = 3;
    /// Effect trigger, subs 0..=3 (payload u16 +0, u16 +2 to `0x1402C2660`; effect + bone is an
    /// Assumption).
    pub const EFFECT: u16 = 4;
    /// Combo window (bit [`super::bits::COMBO`]); sub 0.
    pub const COMBO: u16 = 5;
    /// Sound / event trigger, subs 0..=3 (payload u16 op, u16 arg, u32 to `0x140295420`; op 0x19
    /// swing sound, 0x1A-0x1D voices, 0-7 footsteps).
    pub const SOUND: u16 = 6;
    /// Weapon trail on (bit [`super::bits::TRAIL`]); sub 0.
    pub const TRAIL: u16 = 7;
}

/// Bits of the model flag word (instance +0x14, copied to actor +0x1CC) that the flag tracks set.
pub mod bits {
    /// Hit slot 0; slot s is `HIT0 << s`.
    pub const HIT0: u32 = 0x10000;
    /// Free.
    pub const FREE: u32 = 0x100000;
    /// Combo window.
    pub const COMBO: u32 = 0x400000;
    /// Weapon trail.
    pub const TRAIL: u32 = 0x800000;
}

/// The request map's "no motion" entry value.
pub const NO_ENTRY: u16 = 0xFFF;

/// One request-map word: `entry bits 0-11 | type bits 12-14 | root motion bit 15 | extra id
/// bits 16-27` (Sora's extra ids are all 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapEntry(pub u32);

impl MapEntry {
    /// The motion entry, or `None` for 0xFFF.
    pub fn entry(self) -> Option<u16> {
        let e = (self.0 & 0xFFF) as u16;
        (e != NO_ENTRY).then_some(e)
    }
    /// The type (bits 12-14): 0 adds play flag 0x8000000, 1 adds 0x10000000 (weapon-out stance),
    /// 2 plays as is, 3 picks entry, +1 or +2, 4 picks entry or +1.
    pub fn kind(self) -> u8 {
        ((self.0 >> 12) & 7) as u8
    }
    /// Bit 15: the motion moves the actor by its root helper.
    pub fn root_motion(self) -> bool {
        self.0 & 0x8000 != 0
    }
    /// Bits 16-27.
    pub fn extra(self) -> u16 {
        ((self.0 >> 16) & 0xFFF) as u16
    }
    /// Every entry this request can play (its type-3/4 variants included).
    pub fn variants(self) -> Vec<u16> {
        let Some(e) = self.entry() else { return Vec::new() };
        match self.kind() {
            3 => vec![e, e + 1, e + 2],
            4 => vec![e, e + 1],
            _ => vec![e],
        }
    }
    /// The entry `0x1402A01E0` picks. `a` / `b` are its two int arguments and `alt` the global
    /// flag `DAT_142867364 & 0x1000`. Type 3: `a` off gives entry; else entry+1 if `alt` or `b`
    /// is off, else entry+2. Type 4: entry+1 when `a`, `b` and not `alt`. (Which of `a` / `b` is
    /// stance and which weapon: Assumption, not traced; `pose.rs` reads field 0/2/5, battle
    /// 1/3/6.)
    pub fn resolve(self, a: bool, b: bool, alt: bool) -> Option<u16> {
        let e = self.entry()?;
        Some(match self.kind() {
            3 if a => {
                if alt || !b {
                    e + 1
                } else {
                    e + 2
                }
            }
            4 if a && b && !alt => e + 1,
            _ => e,
        })
    }
}

/// Per-bone flags (motion +0x14), read by the bone pass `0x1403900B0`. The branch structure is
/// Checked (decomp); the meanings of the IK and keep bits are Assumptions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoneFlags(pub u32);

impl BoneFlags {
    /// Bit 26: the bone is skipped.
    pub fn skipped(self) -> bool {
        self.0 & 0x400_0000 != 0
    }
    /// Bit 25: the local matrix is not rebuilt from channels (set exactly on unkeyed bones).
    pub fn not_rebuilt(self) -> bool {
        self.0 & 0x200_0000 != 0
    }
    /// Bits 0-1: 0 FK, 1 aim X at the aim helper (then the bone's own X twist), 2 copy the aim
    /// helper's orientation, 3 (with IK) use the aim helper as the pole.
    pub fn mode(self) -> u8 {
        (self.0 & 3) as u8
    }
    /// Helper (0-based) whose position the bone takes: `((v & 0xFF800) | (v >> 8 & 0x100000))
    /// >> 11`, 1-based in the file.
    pub fn position_helper(self) -> Option<u16> {
        let h = ((self.0 & 0xF_F800) | ((self.0 >> 8) & 0x10_0000)) >> 11;
        (h != 0).then(|| (h - 1) as u16)
    }
    /// Helper (0-based) for aim / orientation: `((v & 0x7FC) | (v >> 16 & 0x800)) >> 2`,
    /// 1-based in the file.
    pub fn aim_helper(self) -> Option<u16> {
        let h = ((self.0 & 0x7FC) | ((self.0 >> 16) & 0x800)) >> 2;
        (h != 0).then(|| (h - 1) as u16)
    }
    /// Bits 21-22: 0 none, 1 two-bone IK start (`0x14038FC20`, law of cosines), 2/3 other IK
    /// modes (Assumption: 3 bends the other way, as in KH2).
    pub fn ik(self) -> u8 {
        ((self.0 >> 21) & 3) as u8
    }
    /// Bit 23 (Assumption: keep world position).
    pub fn keep_world(self) -> bool {
        self.0 & 0x80_0000 != 0
    }
    /// Bit 24 (Assumption: model-space rotation).
    pub fn model_rotation(self) -> bool {
        self.0 & 0x100_0000 != 0
    }
}

/// One animated channel: `u16 target | u8 kind | u8 key count | u16 first key`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Channel {
    /// Bone (bone channels) or helper (helper channels).
    pub target: u16,
    /// Raw kind byte: component bits 0-3, pre-extrapolation bits 4-5, post bits 6-7.
    pub kind: u8,
    /// Number of keys.
    pub count: u8,
    /// Index of the first key in [`Motion::keys`].
    pub first: u16,
}

impl Channel {
    /// The component: 1-3 scale xyz, 4-6 rotation xyz, 7-9 translation xyz.
    pub fn component(self) -> u8 {
        self.kind & 0xF
    }
    /// Before the first key: 2 loops, 0/1 hold (3: Assumption hold).
    pub fn pre(self) -> u8 {
        (self.kind >> 4) & 3
    }
    /// After the last key: 2 loops, 0/1 hold (3: Assumption hold).
    pub fn post(self) -> u8 {
        self.kind >> 6
    }
    /// True for rotation components (wrapped to [-pi, pi] after sampling).
    pub fn is_rotation(self) -> bool {
        (4..=6).contains(&self.component())
    }
}

/// One curve key (16 bytes): `i32 time << 8 | interp`, `f32 value`, `f32 +8`, `f32 +12`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Key {
    /// Time in 1/256 frame.
    pub time: i32,
    /// Interp byte: 0 hold, 1 linear, else Hermite (2 is the common one; 3/4 not verified).
    pub interp: u8,
    /// Value.
    pub value: f32,
    /// +8: tangent used arriving at this key, per 1/256 frame.
    pub tangent_in: f32,
    /// +12: tangent used leaving this key, per 1/256 frame.
    pub tangent_out: f32,
}

impl Key {
    /// Time in frames.
    pub fn frame(&self) -> f32 {
        self.time as f32 / 256.0
    }
}

/// A rig helper (0x28 bytes): `scale xyz | i32 parent | rot xyz | trans xyz`. Helper 0 is the
/// model; the last helper carries root motion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Helper {
    /// Scale.
    pub scale: [f32; 3],
    /// Parent helper.
    pub parent: i32,
    /// Euler rotation, radians (X, Y, Z, row vectors).
    pub rot: [f32; 3],
    /// Translation.
    pub trans: [f32; 3],
}

impl Helper {
    fn set(&mut self, component: u8, v: f32) {
        match component {
            1..=3 => self.scale[component as usize - 1] = v,
            4..=6 => self.rot[component as usize - 4] = v,
            7..=9 => self.trans[component as usize - 7] = v,
            _ => {}
        }
    }
}

/// One starting-pose value (8 bytes): set on every motion change after the model's rest values
/// (`0x14038F550`); keyed channels then overwrite. Holds e.g. the fist pose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InitialValue {
    /// Bone.
    pub bone: u16,
    /// Component, as [`Channel::component`].
    pub component: u16,
    /// Value.
    pub value: f32,
}

/// One flag-track key (16 bytes): `f32 time (frames) | payload[12]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlagKey {
    /// Time in frames.
    pub time: f32,
    /// Bytes +4..+16.
    pub payload: [u8; 12],
}

impl FlagKey {
    /// Byte +4: a flag key is "on" when it is non-zero.
    pub fn value(&self) -> u8 {
        self.payload[0]
    }
    /// The payload as u16 pairs (+4, +6) and u32 (+8), as the trigger code reads it.
    pub fn args(&self) -> (u16, u16, u32) {
        let p = &self.payload;
        (u16::from_le_bytes([p[0], p[1]]), u16::from_le_bytes([p[2], p[3]]), u32::from_le_bytes([p[4], p[5], p[6], p[7]]))
    }
}

/// One flag / trigger track: list entry `u16 sub | u16 kind | i32 count | u32 key offset`.
#[derive(Clone, Debug, PartialEq)]
pub struct FlagTrack {
    /// Sub-id (hit slot, trigger set).
    pub sub: u16,
    /// Type, see [`track`].
    pub kind: u16,
    /// Keys in time order.
    pub keys: Vec<FlagKey>,
}

/// A trigger key that fired in a time step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trigger {
    /// [`track::EFFECT`] or [`track::SOUND`].
    pub kind: u16,
    /// Track sub-id.
    pub sub: u16,
    /// The key.
    pub key: FlagKey,
}

/// One motion.
#[derive(Clone, Debug, PartialEq)]
pub struct Motion {
    /// File offset of the motion (for reference).
    pub offset: usize,
    /// Length in frames (+0x04).
    pub frames: u32,
    /// +0x08 (60.0 in all of Sora's).
    pub fps: f32,
    /// +0x0C raw (0 or 1; meaning not traced).
    pub header_0c: u32,
    /// Per-bone flags (+0x14; +0x10 entries).
    pub bone_flags: Vec<BoneFlags>,
    /// Keyed bone channels.
    pub channels: Vec<Channel>,
    /// Keyed helper channels.
    pub helper_channels: Vec<Channel>,
    /// All keys.
    pub keys: Vec<Key>,
    /// The rig helpers' base values.
    pub helpers: Vec<Helper>,
    /// Starting-pose table.
    pub initial_pose: Vec<InitialValue>,
    /// Flag and trigger tracks, in list order.
    pub tracks: Vec<FlagTrack>,
}

/// Wraps an angle into [-pi, pi] the way `0x140391C70` does.
pub fn wrap_angle(mut v: f32) -> f32 {
    use std::f32::consts::PI;
    if !v.is_finite() {
        return v;
    }
    loop {
        while v > PI {
            v -= 2.0 * PI;
        }
        if v >= -PI {
            return v;
        }
        v += 2.0 * PI;
    }
}

/// Samples a key run at `ticks` (1/256 frame), with the pre/post modes of its channel.
pub fn sample_keys(keys: &[Key], pre: u8, post: u8, ticks: i32) -> Option<f32> {
    let n = keys.len();
    if n == 0 {
        return None;
    }
    if n == 1 {
        return Some(keys[0].value);
    }
    let first = keys[0].time;
    let last = keys[n - 1].time;
    let span = last - first;
    let mut t = ticks;
    if t <= first {
        if pre == 2 && span > 0 {
            while t < first {
                t += span;
            }
        } else {
            return Some(keys[0].value);
        }
    }
    if last <= t {
        if post == 2 && span > 0 {
            while last < t {
                t -= span;
            }
        } else {
            return Some(keys[n - 1].value);
        }
    }
    // 0x14038F820's search.
    let (mut lo, mut hi) = (0usize, n - 1);
    while hi - lo >= 2 {
        let mid = (lo + hi) / 2;
        let km = keys[mid].time;
        if km <= t {
            lo = mid;
            if t <= km {
                return Some(keys[mid].value);
            }
        } else {
            hi = mid;
        }
    }
    let (a, b) = (&keys[lo], &keys[hi]);
    let gap = b.time - a.time;
    if gap <= 0 || a.interp == 0 {
        return Some(a.value);
    }
    let x = (t - a.time) as f32;
    let g = gap as f32;
    if a.interp == 1 {
        return Some(a.value + (b.value - a.value) * x / g);
    }
    let s = x / g;
    let (s2, s3) = (s * s, s * s * s);
    let h00 = 2.0 * s3 - 3.0 * s2 + 1.0;
    let h01 = 3.0 * s2 - 2.0 * s3;
    let h10 = s3 - 2.0 * s2 + s;
    let h11 = s3 - s2;
    Some(h00 * a.value + h01 * b.value + g * (h10 * a.tangent_out + h11 * b.tangent_in))
}

/// Motion-time `t` (frames) to key ticks, as the exe does (`int(t * 256)`).
pub fn ticks(t: f32) -> i32 {
    (t * 256.0) as i32
}

/// Row-vector 3x4 affine (rotation/scale rows, then translation).
type Affine = ([[f32; 3]; 3], [f32; 3]);

fn local_matrix(h: &Helper) -> Affine {
    let (sx, cx) = h.rot[0].sin_cos();
    let (sy, cy) = h.rot[1].sin_cos();
    let (sz, cz) = h.rot[2].sin_cos();
    let rx = [[1.0, 0.0, 0.0], [0.0, cx, sx], [0.0, -sx, cx]];
    let ry = [[cy, 0.0, -sy], [0.0, 1.0, 0.0], [sy, 0.0, cy]];
    let rz = [[cz, sz, 0.0], [-sz, cz, 0.0], [0.0, 0.0, 1.0]];
    let mut r = mul3(&mul3(&rx, &ry), &rz);
    for (i, row) in r.iter_mut().enumerate() {
        for v in row.iter_mut() {
            *v *= h.scale[i];
        }
    }
    (r, h.trans)
}

fn mul3(a: &[[f32; 3]; 3], b: &[[f32; 3]; 3]) -> [[f32; 3]; 3] {
    let mut o = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            o[i][j] = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    o
}

/// `local * parent` with row vectors.
fn compose(local: &Affine, parent: &Affine) -> Affine {
    let r = mul3(&local.0, &parent.0);
    let mut t = parent.1;
    for (j, tj) in t.iter_mut().enumerate() {
        *tj += (0..3).map(|q| local.1[q] * parent.0[q][j]).sum::<f32>();
    }
    (r, t)
}

impl Motion {
    fn parse(b: Bytes, m: usize) -> Result<Motion> {
        let at = |o: usize| -> Result<usize> { b.rel(m, b.u32(m + o)?) };
        let frames = b.u32(m + 4)?;
        let fps = b.f32(m + 8)?;
        let header_0c = b.u32(m + 0x0C)?;
        let bones = b.u32(m + 0x10)? as usize;
        if bones > 0x3FF {
            return bad(format!("mset motion at {m:#x}: implausible bone count {bones}"));
        }
        let flags_at = at(0x14)?;
        let bone_flags = (0..bones).map(|i| b.u32(flags_at + 4 * i).map(BoneFlags)).collect::<Result<_>>()?;
        let read_channels = |count_off: usize, table_off: usize| -> Result<Vec<Channel>> {
            let n = b.u32(m + count_off)? as usize;
            if n > 0x10000 {
                return bad(format!("mset motion at {m:#x}: implausible channel count {n}"));
            }
            let t = at(table_off)?;
            (0..n)
                .map(|i| {
                    let c = t + 6 * i;
                    Ok(Channel { target: b.u16(c)?, kind: b.u8(c + 2)?, count: b.u8(c + 3)?, first: b.u16(c + 4)? })
                })
                .collect()
        };
        let channels = read_channels(0x18, 0x1C)?;
        let helper_channels = read_channels(0x20, 0x24)?;
        let key_count = channels.iter().chain(&helper_channels).map(|c| c.first as usize + c.count as usize).max().unwrap_or(0);
        let keys_at = at(0x28)?;
        let keys = (0..key_count)
            .map(|k| {
                let p = keys_at + 16 * k;
                let head = b.i32(p)?;
                Ok(Key { time: head >> 8, interp: (head & 0xFF) as u8, value: b.f32(p + 4)?, tangent_in: b.f32(p + 8)?, tangent_out: b.f32(p + 12)? })
            })
            .collect::<Result<Vec<_>>>()?;
        let helper_count = b.u32(m + 0x2C)? as usize;
        if helper_count > 0x400 {
            return bad(format!("mset motion at {m:#x}: implausible helper count {helper_count}"));
        }
        let helpers_at = at(0x30)?;
        let helpers = (0..helper_count)
            .map(|i| {
                let r = helpers_at + 0x28 * i;
                Ok(Helper { scale: b.f32x3(r)?, parent: b.i32(r + 0x0C)?, rot: b.f32x3(r + 0x10)?, trans: b.f32x3(r + 0x1C)? })
            })
            .collect::<Result<Vec<_>>>()?;
        let init_count = b.u32(m + 0x34)? as usize;
        if init_count > 0x10000 {
            return bad(format!("mset motion at {m:#x}: implausible starting-pose count {init_count}"));
        }
        let init_at = if init_count > 0 { at(0x38)? } else { 0 };
        let initial_pose = (0..init_count)
            .map(|i| {
                let p = init_at + 8 * i;
                Ok(InitialValue { bone: b.u16(p)?, component: b.u16(p + 2)?, value: b.f32(p + 4)? })
            })
            .collect::<Result<Vec<_>>>()?;
        let mut tracks = Vec::new();
        let mut l = at(0x3C)?;
        while b.u32(l)? != 0xFFFF_FFFF {
            let sub = b.u16(l)?;
            let kind = b.u16(l + 2)?;
            let count = b.i32(l + 4)?;
            if !(0..=0x10000).contains(&count) {
                return bad(format!("mset motion at {m:#x}: implausible flag key count {count}"));
            }
            let ko = b.rel(m, b.u32(l + 8)?)?;
            let keys = (0..count as usize)
                .map(|k| {
                    let p = ko + 16 * k;
                    Ok(FlagKey { time: b.f32(p)?, payload: b.arr(p + 4)? })
                })
                .collect::<Result<Vec<_>>>()?;
            tracks.push(FlagTrack { sub, kind, keys });
            l += 12;
            if tracks.len() > 0x1000 {
                return bad(format!("mset motion at {m:#x}: flag-track list does not end"));
            }
        }
        Ok(Motion { offset: m, frames, fps, header_0c, bone_flags, channels, helper_channels, keys, helpers, initial_pose, tracks })
    }

    /// The keys of a channel.
    pub fn channel_keys(&self, ch: &Channel) -> &[Key] {
        let a = (ch.first as usize).min(self.keys.len());
        let e = (a + ch.count as usize).min(self.keys.len());
        &self.keys[a..e]
    }

    /// Samples a channel at motion time `t` (frames), raw (no angle wrap).
    pub fn sample(&self, ch: &Channel, t: f32) -> Option<f32> {
        sample_keys(self.channel_keys(ch), ch.pre(), ch.post(), ticks(t))
    }

    /// Samples a channel as the evaluator stores it: rotations wrapped to [-pi, pi].
    pub fn channel_value(&self, ch: &Channel, t: f32) -> Option<f32> {
        let v = self.sample(ch, t)?;
        Some(if ch.is_rotation() { wrap_angle(v) } else { v })
    }

    /// The helpers at time `t`: base values with the helper channels applied.
    pub fn helpers_at(&self, t: f32) -> Vec<Helper> {
        let mut h = self.helpers.clone();
        for ch in &self.helper_channels {
            if let (Some(v), Some(dst)) = (self.channel_value(ch, t), h.get_mut(ch.target as usize)) {
                dst.set(ch.component(), v);
            }
        }
        h
    }

    /// Model-space affine of every helper at `t`, helper 0 taken as identity (the evaluator
    /// starts the chain at helper 1 under the model matrix).
    fn helper_chain(&self, t: f32) -> Vec<Affine> {
        let h = self.helpers_at(t);
        let ident: Affine = ([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], [0.0; 3]);
        let mut out = vec![ident; h.len()];
        for i in 1..h.len() {
            let p = h[i].parent;
            let parent = if p > 0 && (p as usize) < i { out[p as usize] } else { ident };
            out[i] = compose(&local_matrix(&h[i]), &parent);
        }
        out
    }

    /// Model-space position of every helper at `t` (helper 0 at the origin).
    pub fn helper_positions(&self, t: f32) -> Vec<[f32; 3]> {
        self.helper_chain(t).into_iter().map(|m| m.1).collect()
    }

    /// Root position at `t`: the last helper's position through the helper chain (motion space,
    /// +z forward). `[0; 3]` when there are fewer than two helpers.
    pub fn root_position(&self, t: f32) -> [f32; 3] {
        if self.helpers.len() < 2 {
            return [0.0; 3];
        }
        self.helper_chain(t).last().map_or([0.0; 3], |m| m.1)
    }

    /// Root travel from `t0` to `t1` (motion space; z is forward).
    pub fn root_travel(&self, t0: f32, t1: f32) -> [f32; 3] {
        let (a, b) = (self.root_position(t0), self.root_position(t1));
        [b[0] - a[0], b[1] - a[1], b[2] - a[2]]
    }

    /// The first track of a type and sub (as the exe's lookups pick).
    pub fn track(&self, kind: u16, sub: u16) -> Option<&FlagTrack> {
        self.tracks.iter().find(|tr| tr.kind == kind && tr.sub == sub)
    }

    /// The flag key in force at `t` (`0x140394770`): none before the first key, else the key
    /// the exe's binary search lands on (the last key with time <= t).
    pub fn flag(&self, kind: u16, sub: u16, t: f32) -> Option<&FlagKey> {
        let k = &self.track(kind, sub)?.keys;
        if k.is_empty() || t < k[0].time {
            return None;
        }
        let hi0 = k.len() - 1;
        if t >= k[hi0].time {
            return Some(&k[hi0]);
        }
        let (mut lo, mut hi) = (0usize, hi0);
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if k[mid].time <= t {
                lo = mid;
                if t <= k[mid].time {
                    break;
                }
            } else {
                hi = mid;
            }
        }
        Some(&k[lo])
    }

    /// Whether a flag is on at `t` (key byte +4 non-zero).
    pub fn flag_on(&self, kind: u16, sub: u16, t: f32) -> bool {
        self.flag(kind, sub, t).is_some_and(|k| k.value() != 0)
    }

    /// The model flag word the evaluator builds at `t` (hit slots, free, combo, trail; see
    /// [`bits`]).
    pub fn flag_bits(&self, t: f32) -> u32 {
        let mut v = 0;
        for s in 0..4u16 {
            if self.flag_on(track::HIT, s, t) {
                v |= bits::HIT0 << s;
            }
        }
        for (kind, bit) in [(track::FREE, bits::FREE), (track::COMBO, bits::COMBO), (track::TRAIL, bits::TRAIL)] {
            if self.flag_on(kind, 0, t) {
                v |= bit;
            }
        }
        v
    }

    /// On-intervals `[a, b)` of a flag inside the motion (a key with a non-zero value turns it
    /// on until the next key; the last one runs to the motion's end). Keys at or past the end
    /// are dropped.
    pub fn intervals(&self, kind: u16, sub: u16) -> Vec<(f32, f32)> {
        let Some(tr) = self.track(kind, sub) else { return Vec::new() };
        let end = self.frames as f32;
        let mut out = Vec::new();
        for (i, k) in tr.keys.iter().enumerate() {
            if k.value() == 0 || k.time >= end {
                continue;
            }
            let b = tr.keys.get(i + 1).map_or(end, |n| n.time).min(end);
            out.push((k.time, b));
        }
        out
    }

    /// Trigger keys with `prev <= time < cur` (`0x140394900`): effect tracks subs 0..=3, then
    /// sound tracks subs 0..=3, each from the first track of that type and sub.
    pub fn triggers(&self, prev: f32, cur: f32) -> Vec<Trigger> {
        let mut out = Vec::new();
        for kind in [track::EFFECT, track::SOUND] {
            for sub in 0..4u16 {
                if let Some(tr) = self.track(kind, sub) {
                    out.extend(tr.keys.iter().filter(|k| prev <= k.time && k.time < cur).map(|k| Trigger { kind, sub, key: *k }));
                }
            }
        }
        out
    }
}

/// A whole motion set.
#[derive(Clone, Debug, PartialEq)]
pub struct MotionSet {
    /// Section offsets (motions, request map, extra).
    pub sections: Vec<u32>,
    /// Motions by entry index.
    pub motions: Vec<Motion>,
    /// The request map, by request id.
    pub requests: Vec<MapEntry>,
    /// The extra section's raw bytes (not decoded; Sora's: u32 10 then 11 x u32 0x80).
    pub extra: Vec<u8>,
}

impl MotionSet {
    /// Parses a whole `.mset` file.
    pub fn parse(file: &[u8]) -> Result<MotionSet> {
        let b = Bytes(file);
        let n = b.u32(0)? as usize;
        if !(2..=16).contains(&n) {
            return bad(format!("mset: implausible section count {n}"));
        }
        let sections = (0..n).map(|i| b.u32(4 + 4 * i)).collect::<Result<Vec<_>>>()?;
        let s1 = sections[0] as usize;
        if b.slice(s1, 4)? != b"MMTN" {
            return bad("mset: motion section is not MMTN");
        }
        let table = b.rel(s1, b.u32(s1 + 0x30)?)?;
        let mut motions = Vec::new();
        loop {
            let o = b.u32(table + 4 * motions.len())?;
            if o == 0xFFFF_FFFF {
                break;
            }
            if motions.len() >= NO_ENTRY as usize {
                return bad("mset: entry table does not end");
            }
            motions.push(Motion::parse(b, b.rel(table, o)?)?);
        }
        let map = sections[1] as usize;
        let map_end = sections.get(2).map_or(file.len(), |&s| s as usize);
        if map_end < map || map_end > file.len() {
            return bad("mset: bad request-map bounds");
        }
        let requests = (0..(map_end - map) / 4).map(|i| b.u32(map + 4 * i).map(MapEntry)).collect::<Result<Vec<_>>>()?;
        let extra = sections.get(2).map_or(Vec::new(), |&s| file.get(s as usize..).unwrap_or(&[]).to_vec());
        Ok(MotionSet { sections, motions, requests, extra })
    }

    /// The map word of a request id.
    pub fn request(&self, id: u16) -> Option<MapEntry> {
        self.requests.get(id as usize).copied()
    }

    /// A request's base entry and motion.
    pub fn motion_for(&self, id: u16) -> Option<(MapEntry, &Motion)> {
        let m = self.request(id)?;
        Some((m, self.motions.get(m.entry()? as usize)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testdata;

    fn key(time_frames: i32, interp: u8, value: f32, tin: f32, tout: f32) -> Key {
        Key { time: time_frames * 256, interp, value, tangent_in: tin, tangent_out: tout }
    }

    #[test]
    fn sampling_rules() {
        let k = [key(0, 1, 0.0, 0.0, 0.0), key(10, 2, 10.0, 0.0, 0.0), key(20, 0, 30.0, 0.0, 0.0)];
        assert_eq!(sample_keys(&k, 0, 0, ticks(5.0)), Some(5.0));
        assert_eq!(sample_keys(&k, 0, 0, ticks(10.0)), Some(10.0));
        // Hermite with zero tangents: smoothstep.
        assert!((sample_keys(&k, 0, 0, ticks(15.0)).unwrap() - 20.0).abs() < 1e-5);
        assert_eq!(sample_keys(&k, 0, 0, ticks(25.0)), Some(30.0));
        assert_eq!(sample_keys(&k, 0, 0, ticks(-5.0)), Some(0.0));
        // Looping both ways.
        assert_eq!(sample_keys(&k, 2, 2, ticks(25.0)), sample_keys(&k, 0, 0, ticks(5.0)));
        assert_eq!(sample_keys(&k, 2, 2, ticks(-5.0)), sample_keys(&k, 0, 0, ticks(15.0)));
        // Tangents are per 1/256 frame: a slope of 1/256 per tick is 1 per frame.
        let lin = [key(0, 2, 0.0, 1.0 / 256.0, 1.0 / 256.0), key(10, 2, 10.0, 1.0 / 256.0, 1.0 / 256.0)];
        assert!((sample_keys(&lin, 0, 0, ticks(3.0)).unwrap() - 3.0).abs() < 1e-4);
        assert_eq!(wrap_angle(4.0), 4.0 - 2.0 * std::f32::consts::PI);
    }

    #[test]
    fn map_words() {
        let m = MapEntry(0x9062);
        assert_eq!((m.entry(), m.kind(), m.root_motion(), m.extra()), (Some(98), 1, true, 0));
        assert_eq!(MapEntry(0x3005).variants(), vec![5, 6, 7]);
        assert_eq!(MapEntry(0x3005).resolve(true, true, false), Some(7));
        assert_eq!(MapEntry(0x4000).resolve(true, true, false), Some(1));
        assert_eq!(MapEntry(0xFFF).entry(), None);
        let f = BoneFlags(0x2002012);
        assert_eq!((f.position_helper(), f.aim_helper(), f.mode(), f.not_rebuilt()), (Some(3), Some(3), 2, true));
    }

    fn sora() -> Option<MotionSet> {
        Some(MotionSet::parse(&testdata::read("xa_ex_0010.mset")?).unwrap())
    }

    #[test]
    fn sora_motion_set() {
        let Some(ms) = sora() else { return };
        assert_eq!(ms.sections, vec![0x80, 0x252480, 0x252A00]);
        assert_eq!(ms.motions.len(), 118);
        assert_eq!(ms.requests.len(), 352);
        assert!(ms.motions.iter().all(|m| m.fps == 60.0 && m.bone_flags.len() == 293 && (11..=14).contains(&m.helpers.len())));
        assert_eq!(ms.motions.iter().filter(|m| m.helpers.len() == 11).count(), 112);
        assert!(ms.requests.iter().all(|r| r.extra() == 0));

        // Request 200 = entry 98, the first ground swing.
        let (m, s1) = ms.motion_for(200).unwrap();
        assert_eq!((m.entry(), m.root_motion()), (Some(98), true));
        assert_eq!(s1.frames, 48);
        assert_eq!(s1.intervals(track::HIT, 0), vec![(14.0, 24.0)]);
        assert_eq!(s1.intervals(track::COMBO, 0), vec![(22.0, 40.0)]);
        assert_eq!(s1.intervals(track::TRAIL, 0), vec![(4.0, 24.0)]);
        assert!(s1.intervals(track::FREE, 0).is_empty());
        let tr = s1.root_travel(0.0, 48.0);
        assert!((tr[2] - 100.0).abs() < 0.01, "{tr:?}");
        assert!((s1.root_travel(0.0, 14.0)[2] - 48.33).abs() < 0.01);
        assert_eq!(s1.flag_bits(13.9), bits::TRAIL);
        assert_eq!(s1.flag_bits(14.0), bits::TRAIL | bits::HIT0);
        assert_eq!(s1.flag_bits(23.0), bits::TRAIL | bits::HIT0 | bits::COMBO);
        assert_eq!(s1.flag_bits(30.0), bits::COMBO);
        assert_eq!(s1.flag_bits(45.0), 0);
        assert!(s1.initial_pose.len() > 100);
        assert_eq!(s1.bone_flags[0], BoneFlags(0x2002012));
        assert_eq!(s1.bone_flags[0].position_helper(), Some(3));

        // Dodge Roll (request 0xDC, entry 117).
        let (m, roll) = ms.motion_for(0xDC).unwrap();
        assert_eq!((m.0, m.entry(), m.root_motion()), (0xA075, Some(117), true));
        assert_eq!(roll.frames, 38);
        assert_eq!(roll.intervals(track::FREE, 0), vec![(30.0, 38.0)]);
        assert!(roll.intervals(track::COMBO, 0).is_empty());
        assert!((roll.root_travel(0.0, 38.0)[2] - 350.0).abs() < 0.01);
        assert!((roll.root_travel(0.0, 12.0)[2] - 213.82).abs() < 0.01);

        // Other motions from the notes.
        let (_, s2) = ms.motion_for(201).unwrap();
        assert_eq!((s2.frames, s2.intervals(track::HIT, 0), s2.intervals(track::COMBO, 0)), (48, vec![(18.0, 32.0)], vec![(26.0, 42.0)]));
        assert!((s2.root_travel(0.0, 48.0)[2] - 120.0).abs() < 0.01);
        let (_, fin) = ms.motion_for(203).unwrap();
        assert_eq!((fin.frames, fin.intervals(track::HIT, 0)), (80, vec![(26.0, 38.0)]));
        assert!((fin.root_travel(0.0, 80.0)[2] - 190.0).abs() < 0.01);
        let (_, land) = ms.motion_for(7).unwrap();
        assert_eq!((land.frames, land.intervals(track::FREE, 0)), (28, vec![(12.0, 28.0)]));
        let (_, guard) = ms.motion_for(212).unwrap();
        assert_eq!((guard.frames, guard.intervals(track::HIT, 0), guard.intervals(track::COMBO, 0)), (54, vec![(10.0, 36.0)], vec![(22.0, 54.0)]));
        // Normal jump set vs High Jump set.
        let e = |r| ms.request(r).unwrap().entry().unwrap();
        assert_eq!([e(4), e(5), e(6), e(7)], [8, 9, 10, 11]);
        assert_eq!([e(9), e(10), e(11), e(12)], [12, 13, 10, 11]);
        // Air swings move no root.
        let (m, air) = ms.motion_for(204).unwrap();
        assert!(!m.root_motion());
        assert_eq!((air.frames, air.intervals(track::HIT, 0)), (40, vec![(6.0, 20.0)]));

        // Every motion samples without panicking; triggers exist in the walk.
        for m in &ms.motions {
            for t in [0.0, m.frames as f32 * 0.5, m.frames as f32] {
                for ch in m.channels.iter().chain(&m.helper_channels) {
                    assert!(m.channel_value(ch, t).is_some_and(f32::is_finite));
                }
            }
        }
        let (_, walk) = ms.motion_for(1).unwrap();
        assert!(!walk.triggers(0.0, walk.frames as f32).is_empty());
    }
}
