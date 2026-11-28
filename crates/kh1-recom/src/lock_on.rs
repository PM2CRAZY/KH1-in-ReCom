//! Lock-on at Combat = KH1 on a controller: Re:CoM's own lock-on on **R1** (0.133.0) with **KH1's
//! lock rules** (0.137.0). Owner, 2026-11-04 (`design-philosophy-instructions.md` § Shop text,
//! lock-on and camera: "Let's have the lock on button be mapped to R1 if combat is set to KH1. If on
//! mouse and keyboard it stays the same."; § Lock-on rules and camera walls: "1. B": KH1's lock
//! rules on R1, R1 again unlocks at once, L2 steps through targets; no hold-style row; prompts as
//! they are). Notes: `docs/recom-lock-on.md`.
//!
//! **Re:CoM** (Checked (decomp + disasm + exe), `sites::LOCK_ON_HOOK` / `LOCK_ON_CHECKED` /
//! `LOCK_ON_CALLS` / `LOCK_ON_RULES_CHECKED`):
//!
//! - Re:CoM's lock-on is `0x1401E8DA0`, called once a frame from the player's Field and Battle exes
//!   (`0x1401F994B`, `0x1401FA1A2`). It stores its nearest lockable target to `player+0x84C`
//!   (`0x1401CDB90`), then works on the target cursor `player+0x3F8` (`+0xE0` locked, `+0xD4` the
//!   locked target, `+0xF8 & 4` a battle room). Unlocked: the lock button **pressed** locks
//!   (`0x1401E8E44`; sound 7). Locked: **released** after a second press (`+0x424 & 0x1000`)
//!   unlocks (`0x1401E8E9C`; sound 8); **held** (`0x1401E8EFF`) with a right-stick flick
//!   (`0x1401E8F2D`) switches to the target Re:CoM's `0x1401CD4B0` gives (`0x1401E8F3C`; sound 7).
//!   Its cursor update `0x140208EC0` takes a new target only within 14 m in a battle room (3 m
//!   elsewhere) of the player (`0x1401F0AB0`), holds it to 30 m (3 m), and unlocks when it is gone
//!   or no longer lockable (`+0x154 & 0x10000`).
//! - Its button is the player's pad object (`player + 0x230`) field `+0x44`, read only by the
//!   lock-on's readers (`0x1401FF740` pressed, `0x1401FF600` released, `0x1401FF4F0` held); the pad
//!   update `0x1401FF150` writes 2 (R2) there every frame (L2 1, R2 2, L1 4, R1 8, … in Re:CoM's
//!   PS2 order). Keyboard or pad: `0x140C2FF64` (1 keyboard / mouse).
//!
//! **KH1** (Checked (KH1 Steam decomp + exe)): the pad step `0x14028B0B0` remaps the PS2 pad word
//! through the table `0x140528290`: R1 `0x800` → `0x400`, L2 `0x100` → `0x800` (and `0x400000`), R2
//! `0x200` → `0x400000` only. Its lock-on `0x1402B0610` reads the **pressed** edges (`+8`, made by
//! `0x14028B430`): not locked, `0x400` locks its pick (`0x1402B1540`), nothing without one; locked
//! (state 3), `0x400` unlocks at once (sound 0x45; the release instead only with the config's hold
//! option, `0x1402FAFA0`), else `0x800` steps through its **lock list** (`0x142DB3520`, cursor
//! `0x142DB3830`, count `0x142DB382C`; reset at every unlock `0x1402B0970`): to the next entry if
//! the cursor is not at the end, else to its pick of the targets not in the list (appended), else
//! back to the first entry, else nothing. Its pick: the lowest `(int)max(0, distance − the lock
//! point's radius)` (first on a tie, `0x1402B0F90`). Each frame (`0x1402B11B0`) the current target
//! gone or out of its range unlocks (no step to another), and the list drops the entries gone or
//! out of range. R2 steps nothing; L2 does nothing unlocked.
//!
//! **Ours** ([`History`], [`nearest`], `win::lock_on`): at Combat = KH1 (Sora's story, the main
//! `CPlayerSora`) with the pad in use, Re:CoM's lock-on reads R1 ([`button`]) and its five input
//! questions get KH1's answers through its own call sites (`LOCK_ON_CALLS`): R1 with no target does
//! nothing; R1 locked unlocks at once (Re:CoM's unlock and sound); L2 locked steps ([`History::step`])
//! through Re:CoM's own switch (its cursor, its sound 7); the right stick switches nothing. Keyboard
//! / mouse, Combat = Re:CoM and Riku's story: Re:CoM's rules and buttons.

/// Re:CoM's side (Steam RVAs and offsets). Checked (decomp + disasm + exe).
pub mod recom {
    /// The lock-on `0x1401E8DA0` (the hook).
    pub const LOCK_ON: u32 = 0x001E_8DA0;
    /// The player's pad object, and its lock-on button field.
    pub const PAD: usize = 0x230;
    pub const LOCK_BUTTON: usize = 0x44;
    /// "Keyboard / mouse in use" (1), else the pad (0).
    pub const KBM_IN_USE: u32 = 0x00C2_FF64;

    /// The player's target cursor and its fields.
    pub const CURSOR: usize = 0x3F8;
    pub const CURSOR_LOCKED: usize = 0xE0;
    pub const CURSOR_TARGET: usize = 0xD4;
    pub const CURSOR_FLAGS: usize = 0xF8;
    /// `+0xF8 & 4`: set by the cursor's init `0x140208120` in a battle room (the enemy gauge's).
    pub const CURSOR_BATTLE: u32 = 4;
    /// Re:CoM's nearest lockable target this frame (`0x1401E8E00`).
    pub const PICK: usize = 0x84C;
    /// The request bits and the lock-on's "pressed again while locked".
    pub const REQUEST: usize = 0x424;
    pub const SECOND_PRESS: u32 = 0x1000;
    /// A lock-on's take of a new target: within 14 m in a battle room, 3 m elsewhere
    /// (`0x14020919B`: the hold reach 30 / 3 m, and 14 m while `+0xF4` is set, as at a lock or a
    /// switch). Data `0x140509258` = 14.0, `0x140509248` = 3.0.
    pub const TAKE_REACH_BATTLE: f32 = 14.0;
    pub const TAKE_REACH_FIELD: f32 = 3.0;

    /// Re:CoM's `LockOnManager` (`0x14087B1E0`) and its list (`+0x88`: nodes `{u32 handle, …,
    /// +0x10 next}`, the last one a sentinel), as its own picks walk it (`0x1401CDB90`).
    pub const MANAGER: u32 = 0x0087_B1E0;
    pub const LIST: usize = 0x88;
    pub const NODE_NEXT: usize = 0x10;
    /// An actor's flags and the lock-on flag every target of Re:CoM's lock-on has.
    pub const ACTOR_FLAGS: usize = 0x154;
    pub const LOCKABLE: u32 = 0x1_0000;
    /// An actor's lock point (vtable `+0x50(obj, out)`: what the cursor and Re:CoM's picks read).
    pub const VT_POINT: usize = 0x50;

    /// The lock-on's five calls KH1's rules answer (`call rel32` each).
    pub const CALL_PRESSED: u32 = 0x001E_8E44;
    pub const CALL_RELEASED: u32 = 0x001E_8E9C;
    pub const CALL_HELD: u32 = 0x001E_8EFF;
    pub const CALL_FLICK: u32 = 0x001E_8F2D;
    pub const CALL_SWITCH: u32 = 0x001E_8F3C;
    /// What they call: the lock button pressed / released / held, the right-stick flick, the
    /// target switch by the right stick.
    pub const PRESSED: u32 = 0x001F_F740;
    pub const RELEASED: u32 = 0x001F_F600;
    pub const HELD: u32 = 0x001F_F4F0;
    pub const FLICK: u32 = 0x001F_F4A0;
    pub const SWITCH: u32 = 0x001C_D4B0;
    /// The pad's pressed test `(mask, pad)` (`jmp 0x14029C240`: pad 0's pressed word `+0x14`).
    pub const PAD_PRESSED: u32 = 0x0029_B2F0;
    /// The player's point the cursor measures from (`(out, 0)`: the main player's mover `+0x30`).
    pub const PLAYER_POINT: u32 = 0x001F_0AB0;
}

/// KH1's side. Checked (KH1 Steam decomp + exe).
pub mod kh1 {
    /// The pad remap table and the lock-on.
    pub const PAD_TABLE: u64 = 0x1_4052_8290;
    pub const LOCK_ON: u64 = 0x1_402B_0610;
    /// The lock list's room: `0x142DB3520` up to its count `0x142DB382C`, 8 bytes an entry.
    pub const LIST_ROOM: usize = (0x382C - 0x3520) / 8;
    /// KH1 units (cm) a Re:CoM metre.
    pub const CM: f32 = 100.0;
}

/// Re:CoM's PS2-order button bits (the pad object's update `0x1401FF19D`).
pub mod bits {
    pub const L2: u32 = 1;
    pub const R2: u32 = 2;
    pub const L1: u32 = 4;
    pub const R1: u32 = 8;
}

/// The lock-on button Re:CoM's lock-on reads this frame: R1 at Combat = KH1 with the pad in use,
/// else Re:CoM's R2 (keyboard and mouse keep Re:CoM's binding; Combat = Re:CoM is Re:CoM's).
pub fn button(kh1_sora: bool, keyboard_mouse: bool) -> u32 {
    if kh1_sora && !keyboard_mouse {
        bits::R1
    } else {
        bits::R2
    }
}

/// Whether KH1's lock rules answer Re:CoM's lock-on this frame (the same rule as R1).
pub fn kh1_rules(kh1_sora: bool, keyboard_mouse: bool) -> bool {
    kh1_sora && !keyboard_mouse
}

/// The reach within which Re:CoM's cursor takes a new target (a lock or a switch).
pub fn take_reach(battle_room: bool) -> f32 {
    if battle_room {
        recom::TAKE_REACH_BATTLE
    } else {
        recom::TAKE_REACH_FIELD
    }
}

/// One of Re:CoM's lockable targets: its handle, lock point (Re:CoM metres) and KH1 lock-point
/// radius (KH1 units; 0 without a KH1 twin).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Candidate {
    pub handle: u32,
    pub point: [f32; 3],
    pub radius: f32,
}

/// KH1's score of a target (`0x1402B0F90`): the distance to its lock point less the point's radius,
/// at least 0, in whole KH1 units.
pub fn score(origin: [f32; 3], c: &Candidate) -> i32 {
    let d = distance(origin, c.point) * kh1::CM - c.radius;
    if d <= 0.0 {
        0
    } else {
        d as i32
    }
}

pub fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    let (x, y, z) = (b[0] - a[0], b[1] - a[1], b[2] - a[2]);
    (x * x + y * y + z * z).sqrt()
}

/// KH1's pick (`0x1402B1540`) among Re:CoM's targets: the lowest [`score`] (the first on a tie)
/// of those not in `listed`, within Re:CoM's take reach `reach` (metres; beyond it Re:CoM's cursor
/// would drop the lock instead of taking the target).
pub fn nearest(origin: [f32; 3], targets: &[Candidate], reach: f32, listed: &[u32]) -> Option<u32> {
    let mut best: Option<(i32, u32)> = None;
    for c in targets {
        if c.handle == 0 || listed.contains(&c.handle) || distance(origin, c.point) > reach {
            continue;
        }
        let s = score(origin, c);
        if best.is_none_or(|(b, _)| s < b) {
            best = Some((s, c.handle));
        }
    }
    best.map(|(_, h)| h)
}

/// 0.181.0: KH1's soft target (Auto Lock On, lock state 2) and an attack's auto lock (state 4), as
/// the workspace's `knowledge/KH1/kh1-lock-on.md` has them (Checked (decomp + exe)).
pub mod soft {
    use super::{kh1, Candidate};

    /// The pick's reach (`0x14042F998`): the flat distance to the lock point less its radius.
    pub const REACH: f32 = 300.0;
    /// The pick's height window (`0x1403EF4E8` / `0x1403EF6D8`): ±350.
    pub const HEIGHT: f32 = 350.0;
    /// The facing weight (`0x14042F7EC` = 0.5, squared) and the height weight (`0x1403D7CC4`).
    pub const FACING: f32 = 0.25;
    pub const HEIGHT_WEIGHT: f32 = 0.2;
    /// State 4 ends this long after the last command that took it (`0x14042F980`, the frame delta
    /// summed; Assumption: 60 Hz frames, so 1.5 s).
    pub const HOLD: f32 = 90.0;
    /// A held target past these ends it (`0x1402B13F0`: 1500 across, 1000 up).
    pub const HOLD_ACROSS: f32 = 1500.0;
    pub const HOLD_UP: f32 = 1000.0;

    /// The flat gap (KH1 units, less the radius, at least 0) and the height difference (KH1 units)
    /// from `origin` to `c`, with the flat distance (metres) and direction.
    fn gap(origin: [f32; 3], c: &Candidate) -> (f32, f32, f32, [f32; 2]) {
        let (dx, dz) = (c.point[0] - origin[0], c.point[2] - origin[2]);
        let flat = dx.hypot(dz);
        let g = (flat * kh1::CM - c.radius).max(0.0);
        (g, (c.point[1] - origin[1]) * kh1::CM, flat, if flat > 0.0 { [dx / flat, dz / flat] } else { [0.0, 0.0] })
    }

    /// KH1's pick (`0x1402B0A00`): among the targets within [`REACH`] and [`HEIGHT`], the lowest
    /// `((1 − cos) × 0.25 + 1) × gap + |dy| × 0.2`, `cos` the flat cosine between the player's
    /// facing `forward` (x, z) and the direction to the target (`0x1402A8D20`). Assumption: no line
    /// of sight test (`0x140193B60` has no Re:CoM twin here).
    pub fn pick(origin: [f32; 3], forward: [f32; 2], targets: &[Candidate]) -> Option<u32> {
        let mut best: Option<(f32, u32)> = None;
        for c in targets {
            if c.handle == 0 {
                continue;
            }
            let (g, dy, flat, dir) = gap(origin, c);
            if g > REACH || dy.abs() > HEIGHT {
                continue;
            }
            let cos = if flat > 0.0 { dir[0] * forward[0] + dir[1] * forward[1] } else { 1.0 };
            let s = ((1.0 - cos) * FACING + 1.0) * g + dy.abs() * HEIGHT_WEIGHT;
            if best.is_none_or(|(b, _)| s < b) {
                best = Some((s, c.handle));
            }
        }
        best.map(|(_, h)| h)
    }

    /// A held target stays while it is within [`HOLD_ACROSS`] / [`HOLD_UP`].
    pub fn holds(origin: [f32; 3], c: &Candidate) -> bool {
        let (g, dy, ..) = gap(origin, c);
        g <= HOLD_ACROSS && dy.abs() <= HOLD_UP
    }

    /// An attack's auto lock (state 4): the target a command took and the time since.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Held {
        pub handle: u32,
        pub frames: f32,
    }

    impl Held {
        /// A command took `handle` (`0x1402B08A0(…, 4)` zeroes the time each time).
        pub fn take(handle: u32) -> Held {
            Held { handle, frames: 0.0 }
        }

        /// One frame of `step` frames: false when it ends ([`HOLD`] reached, or L2 / R2).
        pub fn tick(&mut self, step: f32, l2_r2: bool) -> bool {
            self.frames += step;
            self.frames < HOLD && !l2_r2
        }
    }
}

/// KH1's lock list (`0x142DB3520`, its cursor and count): the targets L2 has stepped through
/// since the lock began, in order, and the one locked now.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct History {
    list: Vec<u32>,
    at: usize,
}

impl History {
    pub const EMPTY: History = History { list: Vec::new(), at: 0 };

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn entries(&self) -> &[u32] {
        &self.list
    }

    pub fn current(&self) -> Option<u32> {
        self.list.get(self.at).copied()
    }

    /// KH1's unlock (`0x1402B0970`): the list empties.
    pub fn clear(&mut self) {
        self.list.clear();
        self.at = 0;
    }

    /// After Re:CoM's lock-on ran: `locked` is its cursor's locked target (None: not locked, or
    /// locked on nothing). A new lock starts the list with its target (KH1's lock `0x1402B079F`
    /// with an empty list); a target Re:CoM changed by other means (a sleight's lock) starts it
    /// again; no lock empties it.
    pub fn sync(&mut self, locked: Option<u32>) {
        match locked {
            None => self.clear(),
            Some(t) if self.current() != Some(t) => {
                self.list.clear();
                self.list.push(t);
                self.at = 0;
            }
            Some(_) => {}
        }
    }

    /// KH1's per-frame drop (`0x1402B11B0`, state 3): entries `keep` refuses go, the cursor stays
    /// on its entry. The current entry always stays (Re:CoM still holds it; KH1 would unlock).
    pub fn prune(&mut self, keep: impl Fn(u32) -> bool) {
        let cur = self.current();
        let mut out = Vec::with_capacity(self.list.len());
        let mut at = 0;
        for (i, &h) in self.list.iter().enumerate() {
            if i == self.at {
                at = out.len();
                out.push(h);
            } else if keep(h) {
                out.push(h);
            }
        }
        self.list = out;
        self.at = if cur.is_some() { at } else { 0 };
    }

    /// KH1's L2 while locked (`0x1402B0610`, state 3, `0x800`): the next entry if the cursor is not
    /// at the end; else `pick` (KH1's pick of the targets not in the list), appended; else back to
    /// the first entry; else nothing.
    pub fn step(&mut self, pick: impl FnOnce(&[u32]) -> Option<u32>) -> Option<u32> {
        if self.list.is_empty() {
            return None;
        }
        if self.at + 1 < self.list.len() {
            self.at += 1;
            return self.current();
        }
        if let Some(t) = pick(&self.list) {
            if self.list.len() < kh1::LIST_ROOM {
                self.list.push(t);
                self.at = self.list.len() - 1;
            }
            return Some(t);
        }
        if self.at != 0 {
            self.at = 0;
            return self.current();
        }
        None
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn kh1_soft_target_pick_and_hold() {
        use super::soft::{holds, pick, Held, HOLD};
        let c = |h: u32, x: f32, y: f32, z: f32, r: f32| Candidate { handle: h, point: [x, y, z], radius: r };
        let o = [0.0; 3];
        let fwd = [0.0, 1.0];
        // 3 m flat less the radius; ±3.5 m up.
        assert_eq!(pick(o, fwd, &[c(1, 0.0, 0.0, 3.4, 50.0)]), Some(1));
        assert_eq!(pick(o, fwd, &[c(1, 0.0, 0.0, 3.6, 50.0)]), None);
        assert_eq!(pick(o, fwd, &[c(1, 0.0, 3.6, 1.0, 0.0)]), None);
        // In front beats behind at the same gap (behind weighs ×1.5).
        assert_eq!(pick(o, fwd, &[c(1, 0.0, 0.0, -2.0, 0.0), c(2, 0.0, 0.0, 2.5, 0.0)]), Some(2));
        assert_eq!(pick(o, fwd, &[c(1, 0.0, 0.0, -1.5, 0.0), c(2, 0.0, 0.0, 2.5, 0.0)]), Some(1), "1.5 m × 1.5 < 2.5 m");
        // Height weighs 0.2.
        assert_eq!(pick(o, fwd, &[c(1, 0.0, 2.0, 1.0, 0.0), c(2, 0.0, 0.0, 1.3, 0.0)]), Some(2));
        // The hold: 15 m across, 10 m up; 90 frames since the last command; L2 / R2.
        assert!(holds(o, &c(1, 0.0, 0.0, 14.0, 0.0)) && !holds(o, &c(1, 0.0, 0.0, 16.0, 0.0)) && !holds(o, &c(1, 0.0, 10.5, 0.0, 0.0)));
        let mut h = Held::take(7);
        assert!(h.tick(HOLD - 1.0, false));
        assert!(!h.tick(1.0, false));
        assert!(!Held::take(7).tick(1.0, true));
    }

    use super::*;

    #[test]
    fn r1_only_on_the_pad_at_combat_kh1() {
        assert_eq!(button(true, false), bits::R1);
        assert_eq!(button(true, true), bits::R2, "keyboard and mouse: Re:CoM's binding");
        assert_eq!(button(false, false), bits::R2, "Combat = Re:CoM / Riku: Re:CoM's");
        assert_eq!(button(false, true), bits::R2);
        // Re:CoM's own lock-on button is the one its pad update writes.
        assert_eq!(bits::R2, 2);
        assert_eq!(bits::R1 & (bits::L1 | bits::L2 | bits::R2), 0, "R1 is its own bit");
        // KH1's rules go with R1.
        for (k, m) in [(true, false), (true, true), (false, false), (false, true)] {
            assert_eq!(kh1_rules(k, m), button(k, m) == bits::R1);
        }
    }

    fn c(handle: u32, x: f32, radius: f32) -> Candidate {
        Candidate { handle, point: [x, 0.0, 0.0], radius }
    }

    #[test]
    fn kh1_pick_is_the_nearest_surface_first_on_a_tie_within_re_com_s_reach() {
        let o = [0.0; 3];
        let t = [c(1, 5.0, 0.0), c(2, 3.0, 0.0), c(3, 4.0, 0.0)];
        assert_eq!(nearest(o, &t, 14.0, &[]), Some(2));
        assert_eq!(nearest(o, &t, 14.0, &[2]), Some(3));
        assert_eq!(nearest(o, &t, 14.0, &[2, 3]), Some(1));
        assert_eq!(nearest(o, &t, 14.0, &[1, 2, 3]), None);
        // A big lock sphere counts from its surface (KH1 units: 5 m with radius 250 is 250).
        let big = [c(1, 3.0, 0.0), c(2, 5.0, 250.0)];
        assert_eq!(nearest(o, &big, 14.0, &[]), Some(2));
        // Whole units, the first on a tie (KH1's strict `<`).
        let tie = [c(7, 3.004, 0.0), c(8, 3.0, 0.0)];
        assert_eq!(score(o, &tie[0]), 300);
        assert_eq!(nearest(o, &tie, 14.0, &[]), Some(7));
        // Inside its own sphere: 0.
        assert_eq!(score(o, &c(9, 1.0, 150.0)), 0);
        // Re:CoM's take reach: farther targets never offered (its cursor would unlock).
        assert_eq!(nearest(o, &[c(1, 14.5, 0.0)], take_reach(true), &[]), None);
        assert_eq!(nearest(o, &[c(1, 14.0, 0.0)], take_reach(true), &[]), Some(1));
        assert_eq!(nearest(o, &[c(1, 3.5, 0.0)], take_reach(false), &[]), None);
        // Handle 0 is no target.
        assert_eq!(nearest(o, &[c(0, 1.0, 0.0)], 14.0, &[]), None);
    }

    /// KH1's L2: nearest unvisited first, then back to the first and round the list again.
    #[test]
    fn l2_steps_as_kh1_s_lock_list() {
        let o = [0.0; 3];
        // Locked on 2 (Re:CoM's pick); 1 at 5 m, 3 at 4 m, 4 at 6 m.
        let t = [c(1, 5.0, 0.0), c(2, 3.0, 0.0), c(3, 4.0, 0.0), c(4, 6.0, 0.0)];
        let mut h = History::default();
        h.sync(Some(2));
        let mut steps = vec![];
        for _ in 0..7 {
            let s = h.step(|l| nearest(o, &t, 14.0, l));
            h.sync(s);
            steps.push(s.unwrap());
        }
        assert_eq!(steps, [3, 1, 4, 2, 3, 1, 4]);
        // After the wrap a nearer newcomer waits until the end of the list (KH1 steps the list first).
        let t2 = [c(1, 5.0, 0.0), c(2, 3.0, 0.0), c(3, 4.0, 0.0), c(4, 6.0, 0.0), c(5, 1.0, 0.0)];
        let mut steps = vec![];
        for _ in 0..3 {
            let s = h.step(|l| nearest(o, &t2, 14.0, l));
            h.sync(s);
            steps.push(s.unwrap());
        }
        assert_eq!(steps, [5, 2, 3]);
    }

    #[test]
    fn l2_with_one_target_or_none_does_nothing() {
        let mut h = History::default();
        assert_eq!(h.step(|_| Some(9)), None, "not locked: L2 does nothing");
        h.sync(Some(1));
        assert_eq!(h.step(|_| None), None);
        assert_eq!(h.current(), Some(1));
        h.sync(None);
        assert!(h.is_empty(), "an unlock empties the list");
    }

    #[test]
    fn the_list_drops_what_is_gone_and_keeps_its_place() {
        let mut h = History::default();
        h.sync(Some(1));
        for t in [2, 3, 4] {
            let s = h.step(|_| Some(t));
            h.sync(s);
        }
        // Wrap to the first, step once: on 2.
        h.step(|_| None);
        h.step(|_| None);
        assert_eq!(h.current(), Some(2));
        // 1 and 3 gone: the list is [2, 4], still on 2; then 4; then the pick.
        h.prune(|t| t != 1 && t != 3);
        assert_eq!(h.entries(), &[2, 4]);
        assert_eq!(h.current(), Some(2));
        assert_eq!(h.step(|_| None), Some(4));
        // The current entry stays even when refused (Re:CoM still holds it).
        h.prune(|_| false);
        assert_eq!(h.entries(), &[4]);
        assert_eq!(h.current(), Some(4));
        // A target Re:CoM changed by other means starts the list again.
        h.sync(Some(7));
        assert_eq!(h.entries(), &[7]);
    }

    #[test]
    fn the_list_s_room_is_kh1_s() {
        assert_eq!(kh1::LIST_ROOM, 97);
        let mut h = History::default();
        h.sync(Some(1));
        for t in 2..200 {
            assert_eq!(h.step(|_| Some(t)), Some(t));
        }
        assert_eq!(h.entries().len(), kh1::LIST_ROOM);
    }
}
