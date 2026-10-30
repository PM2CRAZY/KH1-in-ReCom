//! Motions and KH1's motion-request player (`0x14029F7C0` / `0x14029F980` / `0x14029FA30`
//! requests, `0x14029F0A0` per-frame update). Frames are 60 Hz motion-clock frames.
//!
//! Each motion's flag tracks become the per-frame flag word (actor +0x1CC) the game logic reads:
//! hit active (0x10000 << slot), combo window (0x400000), free (0x100000), trail (0x800000).

use crate::math::Vec3;

/// actor +0x1CC bits (`0x140391C70` → `0x14029F0A0`). Checked (decomp).
pub mod flag {
    /// Hit slot 0; slot n is `HIT0 << n`.
    pub const HIT0: u32 = 0x10000;
    pub const HIT_ANY: u32 = 0xF0000;
    /// Type 0 track: clears the action lock.
    pub const FREE: u32 = 0x100000;
    /// Type 5 track: Attack (and jump) accepted.
    pub const COMBO_WINDOW: u32 = 0x400000;
    /// Type 7 track: Keyblade trail.
    pub const TRAIL: u32 = 0x800000;
}

/// Request word bits. Checked (decomp).
pub mod req {
    pub const ID_MASK: u32 = 0xFFFF;
    /// Hold at the end instead of looping.
    pub const HOLD: u32 = 0x10000;
    /// Sets the action lock (actor flag 4) when requested.
    pub const ACTION: u32 = 0x100_0000;
    /// Releases the action lock when it starts.
    pub const RELEASE: u32 = 0x4000_0000;
}

/// Request ids used by the code (`docs/kh1-sora.md`).
pub mod request {
    pub const IDLE: u16 = 0;
    pub const WALK: u16 = 1;
    pub const RUN: u16 = 2;
    pub const JUMP: u16 = 4;
    pub const APEX: u16 = 5;
    pub const FALL: u16 = 6;
    pub const LAND: u16 = 7;
    pub const HIGH_JUMP: u16 = 9;
    pub const HIGH_LAND: u16 = 0xC;
    pub const GLIDE: u16 = 0x71;
    pub const GLIDE_FAST: u16 = 0x72;
    pub const GLIDE_TURN_A: u16 = 0x73;
    pub const GLIDE_TURN_B: u16 = 0x74;
    pub const DODGE_ROLL: u16 = 0xDC;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlagKind {
    /// Hit active for an attack object watching this slot (0..=3).
    Hit(u8),
    ComboWindow,
    Free,
    Trail,
}

impl FlagKind {
    pub fn bit(self) -> u32 {
        match self {
            FlagKind::Hit(s) => flag::HIT0 << (s & 3),
            FlagKind::ComboWindow => flag::COMBO_WINDOW,
            FlagKind::Free => flag::FREE,
            FlagKind::Trail => flag::TRAIL,
        }
    }
}

/// A flag track as on/off spans: on while `start <= t < end` (`0x140394770`: the last key with
/// time ≤ t decides, a non-zero value is on). Checked (decomp).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlagWindow {
    pub kind: FlagKind,
    pub start: f32,
    /// `f32::INFINITY` = until the motion ends.
    pub end: f32,
}

impl FlagWindow {
    pub fn new(kind: FlagKind, start: f32, end: f32) -> FlagWindow {
        FlagWindow { kind, start, end }
    }
}

/// One motion entry of the motion set.
#[derive(Clone, Debug, PartialEq)]
pub struct MotionInfo {
    /// Frame count (header +4), 60 Hz frames.
    pub length: f32,
    /// The request map's root-motion bit (bit 15): the root travel moves Sora.
    pub root_motion: bool,
    /// Cumulative root travel at integer frames 0..=length, local (x right, y, z forward), KH1
    /// units. Empty = none. Between samples it is linear. Assumption: linear between keys (every
    /// root key in Sora's mset has interp 1; `0x14038F820`'s body is cut off in the decomp).
    pub root: Vec<Vec3>,
    pub flags: Vec<FlagWindow>,
}

impl MotionInfo {
    pub fn new(length: f32) -> MotionInfo {
        MotionInfo { length, root_motion: false, root: Vec::new(), flags: Vec::new() }
    }

    pub fn with_flag(mut self, kind: FlagKind, start: f32, end: f32) -> MotionInfo {
        self.flags.push(FlagWindow::new(kind, start, end));
        self
    }

    /// Root travel forward (+z) from `(frame, units)` keys, linear between keys, sampled at each
    /// integer frame. Sets the root-motion bit.
    pub fn with_forward_keys(mut self, keys: &[(f32, f32)]) -> MotionInfo {
        let n = self.length.max(0.0) as usize;
        self.root = (0..=n).map(|f| Vec3::new(0.0, 0.0, sample_keys(keys, f as f32))).collect();
        self.root_motion = true;
        self
    }

    /// The flag word at motion time `t`.
    pub fn flags_at(&self, t: f32) -> u32 {
        self.flags.iter().filter(|w| w.start <= t && t < w.end).fold(0, |a, w| a | w.kind.bit())
    }

    /// First frame a flag opens.
    pub fn first(&self, kind: FlagKind) -> Option<f32> {
        self.flags.iter().filter(|w| w.kind == kind).map(|w| w.start).reduce(f32::min)
    }

    /// Cumulative root position at time `t`.
    pub fn root_at(&self, t: f32) -> Vec3 {
        if self.root.is_empty() {
            return Vec3::ZERO;
        }
        let t = t.max(0.0);
        let i = t.floor() as usize;
        if i + 1 >= self.root.len() {
            return *self.root.last().unwrap_or(&Vec3::ZERO);
        }
        self.root[i].lerp(self.root[i + 1], t - i as f32)
    }
}

fn sample_keys(keys: &[(f32, f32)], t: f32) -> f32 {
    match keys {
        [] => 0.0,
        [k] => k.1,
        _ => {
            if t <= keys[0].0 {
                return keys[0].1;
            }
            for w in keys.windows(2) {
                if t <= w[1].0 {
                    let span = w[1].0 - w[0].0;
                    let f = if span > 0.0 { (t - w[0].0) / span } else { 1.0 };
                    return w[0].1 + (w[1].1 - w[0].1) * f;
                }
            }
            keys[keys.len() - 1].1
        }
    }
}

/// The motion set: entries and the request map (mset section 2: request → entry).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MotionSet {
    pub entries: Vec<MotionInfo>,
    /// `map[request] = entry`. Assumption: the build step resolves KH1's variant types 3 / 4
    /// (field / battle / no-weapon) to the battle entry, so the stance pick is not modelled.
    pub map: Vec<Option<usize>>,
}

impl MotionSet {
    /// Adds an entry and maps the requests to it.
    pub fn add(&mut self, requests: &[u16], info: MotionInfo) -> usize {
        let e = self.entries.len();
        self.entries.push(info);
        for &r in requests {
            self.map_request(r, e);
        }
        e
    }

    pub fn map_request(&mut self, request: u16, entry: usize) {
        let r = request as usize;
        if self.map.len() <= r {
            self.map.resize(r + 1, None);
        }
        self.map[r] = Some(entry);
    }

    pub fn entry(&self, request: u16) -> Option<usize> {
        self.map.get(request as usize).copied().flatten()
    }

    pub fn info(&self, request: u16) -> Option<&MotionInfo> {
        self.entry(request).and_then(|e| self.entries.get(e))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Queued {
    request: u32,
    start: f32,
    blend: f32,
}

/// The playing motion and its queue (actor +0x15C..+0x28C).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MotionPlayer {
    /// +0x164: request id playing (without bits). 0xFFFF before the first start.
    pub request: u16,
    /// +0x168: entry playing.
    pub entry: Option<usize>,
    /// +0x16C: motion clock.
    pub frame: f32,
    /// +0x170 / +0x174: cross-fade length and clock; the new motion holds at its start frame.
    pub blend: f32,
    pub blend_clock: f32,
    /// +0x1CC: this frame's flag word.
    pub flags: u32,
    /// +0x160 bit 1: start the queue head on the next update.
    start_now: bool,
    /// +0x160 bit 0x100: the motion started on the last update (no hit tests).
    pub just_started: bool,
    /// +0x160 bit 8 (hold at end) and bit 4 (root motion on).
    hold: bool,
    pub root_on: bool,
    /// +0x200: root travel of the last update, in world space (applied by the next physics step).
    pub root_delta: Vec3,
    queue: Vec<Queued>,
}

/// What a motion update did to the action lock.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UpdateResult {
    /// A request with the release bit started, or the free flag is on: actor flags 4 | 0x1000
    /// are cleared.
    pub release_lock: bool,
    /// A new entry started on this update.
    pub started: Option<u16>,
}

impl MotionPlayer {
    pub fn new() -> MotionPlayer {
        MotionPlayer { request: 0xFFFF, ..Default::default() }
    }

    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    /// The queued (not yet playing) head request, if a start is pending.
    pub fn pending(&self) -> Option<u16> {
        if self.start_now {
            self.queue.first().map(|q| (q.request & req::ID_MASK) as u16)
        } else {
            None
        }
    }

    /// `0x14029F980`: replace the queue with this request and start it on the next update.
    /// Returns true when the request carries the action bit (the caller sets the lock).
    pub fn play(&mut self, request: u32, blend: f32) -> bool {
        self.queue.clear();
        self.queue.push(Queued { request: request & !req::ACTION, start: 0.0, blend });
        self.start_now = true;
        request & req::ACTION != 0
    }

    /// `0x14029FA30`: append; it plays when the current one ends. Returns the action bit.
    pub fn append(&mut self, request: u32, blend: f32) -> bool {
        self.queue.push(Queued { request: request & !req::ACTION, start: 0.0, blend });
        request & req::ACTION != 0
    }

    /// Sets the start frame of the last queued entry (the land-jump uses 2.0).
    pub fn set_last_start(&mut self, start: f32) {
        if let Some(q) = self.queue.last_mut() {
            q.start = start;
        }
    }

    /// `0x14029F7C0`: a locomotion request. Does nothing if the entry is already playing; else
    /// empties the queue and starts it on the next update. Walk/run ↔ walk/run keep their phase;
    /// idle → walk starts at blend × 0.5. Returns `Some(action bit)` when it requested.
    pub fn locomotion(&mut self, set: &MotionSet, request: u32, blend: f32, step: f32) -> Option<bool> {
        let id = (request & req::ID_MASK) as u16;
        let entry = set.entry(id);
        if entry.is_some() && entry == self.entry {
            return None;
        }
        let walkrun = |r: u16| r == 1 || r == 2 || r == 0xAC || r == 0xAD;
        let mut start = 0.0;
        if self.entry.is_some() && walkrun(self.request) && walkrun(id) {
            let old_len = self.entry.and_then(|e| set.entries.get(e)).map_or(1.0, |m| m.length);
            let new_len = entry.and_then(|e| set.entries.get(e)).map_or(1.0, |m| m.length);
            if old_len > 0.0 && new_len > 0.0 {
                start = (step + self.frame + blend) / old_len * new_len;
                while new_len <= start {
                    start -= new_len;
                }
            }
        } else if id == 1 && self.request == 0 {
            start = blend * 0.5;
        }
        self.queue.clear();
        self.queue.push(Queued { request: request & !req::ACTION, start, blend });
        self.start_now = true;
        Some(request & req::ACTION != 0)
    }

    fn start_head(&mut self, set: &MotionSet, dt: f32) -> UpdateResult {
        let mut out = UpdateResult::default();
        if self.queue.is_empty() {
            return out;
        }
        let q = self.queue.remove(0);
        let id = (q.request & req::ID_MASK) as u16;
        let had = self.entry.is_some();
        self.request = id;
        self.entry = set.entry(id);
        self.frame = q.start;
        self.hold = q.request & req::HOLD != 0;
        self.root_on = self.info(set).is_some_and(|m| m.root_motion);
        if q.blend <= dt || !had {
            self.blend = 0.0;
            self.blend_clock = 0.0;
        } else {
            self.blend = q.blend;
            self.blend_clock = dt;
        }
        if q.request & req::RELEASE != 0 {
            out.release_lock = true;
        }
        out.started = Some(id);
        out
    }

    pub fn info<'a>(&self, set: &'a MotionSet) -> Option<&'a MotionInfo> {
        self.entry.and_then(|e| set.entries.get(e))
    }

    /// `0x14029F0A0(actor, step)`: one update. `facing` turns the root travel into world space.
    pub fn update(&mut self, set: &MotionSet, step: f32, facing: f32) -> UpdateResult {
        let dt = step; // × rate 1.0 (+0x284)
        let mut out = UpdateResult::default();
        self.just_started = false;
        let mut root = Vec3::ZERO;
        if self.start_now {
            self.start_now = false;
            out = self.start_head(set, dt);
            self.just_started = true;
        } else if self.blend > 0.0 {
            self.blend_clock += dt;
            let over = self.blend_clock - self.blend;
            if over >= 0.0 {
                // Assumption: the travel from the held start frame counts once the fade ends.
                let from = self.info(set).map_or(Vec3::ZERO, |m| m.root_at(self.frame));
                self.frame += over;
                self.blend = 0.0;
                self.blend_clock = 0.0;
                if let Some(m) = self.info(set) {
                    root = m.root_at(self.frame) - from;
                }
            }
        } else if self.info(set).is_none() && !self.queue.is_empty() {
            // Assumption: a request with no motion entry ends at once.
            out = self.start_head(set, dt);
        } else if let Some(m) = self.info(set) {
            let old = self.frame;
            let end = m.length;
            let new = (old + dt).min(end);
            self.frame = new;
            root = m.root_at(new) - m.root_at(old);
            let over = new - end;
            if over >= 0.0 {
                if self.queue.is_empty() {
                    if !self.hold {
                        // Loop: the travel past the end restarts from the loop start (0).
                        self.frame = over;
                        root += m.root_at(over) - m.root_at(0.0);
                    }
                } else {
                    self.queue[0].start += over;
                    let r = self.start_head(set, dt);
                    out.release_lock |= r.release_lock;
                    out.started = r.started;
                }
            }
        }
        // The root travel only counts for a motion with the root bit (+0x160 & 4).
        self.root_delta = if self.root_on && self.blend <= 0.0 { root.rotate_yaw(facing) } else { Vec3::ZERO };
        self.flags = self.info(set).map_or(0, |m| m.flags_at(self.frame));
        if self.flags & flag::FREE != 0 {
            out.release_lock = true;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set() -> MotionSet {
        let mut s = MotionSet::default();
        s.add(&[0], MotionInfo::new(120.0));
        s.add(&[200], MotionInfo::new(48.0).with_flag(FlagKind::ComboWindow, 22.0, 40.0).with_forward_keys(&[(0.0, 0.0), (8.0, 10.4), (14.0, 48.33), (34.0, 100.0), (48.0, 100.0)]));
        s
    }

    #[test]
    fn action_plays_then_queue_releases() {
        let s = set();
        let mut p = MotionPlayer::new();
        p.play(0, 0.0);
        p.update(&s, 1.0, 0.0);
        assert!(p.play(200 | req::ACTION, 0.0));
        p.append(req::RELEASE, 0.0);
        let r = p.update(&s, 1.0, 0.0);
        assert_eq!(r.started, Some(200));
        assert_eq!(p.frame, 0.0);
        let mut travel = 0.0;
        let mut released_at = None;
        for k in 1..=48 {
            let r = p.update(&s, 1.0, 0.0);
            travel += p.root_delta.z;
            if k == 22 {
                assert!(p.flags & flag::COMBO_WINDOW != 0);
            }
            if k == 40 {
                assert!(p.flags & flag::COMBO_WINDOW == 0);
            }
            if r.release_lock && released_at.is_none() {
                released_at = Some(k);
            }
        }
        assert_eq!(released_at, Some(48));
        assert_eq!(p.request, 0);
        assert!((travel - 100.0).abs() < 1e-3);
    }
}
