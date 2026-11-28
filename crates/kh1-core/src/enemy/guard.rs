//! KH1's guard-type attack rows (M11, the shared enemy-guard step; `docs/recom-m11-enemies.md`
//! § Enemy guard): an enemy row whose clash type is 1..4 never meets a hurt body
//! (`0x1402B3BE0`: an attack against a body entry is tested only when its type is 0, 3 or ≥ 5) and
//! only meets the attack types [`MEETS`] lists; when two attacks meet, KH1 resolves them by
//! priority (`0x1402B4A20` → `0x1402B4BC0`, both orders). Checked (decomp + exe), Steam addresses.
//!
//! What a guard row is to the host: while it lives (its request plays and that motion's HIT slot is
//! on, `0x1402B47A0` mode 2), an attack of a type it meets coming from in front of the enemy (cos of
//! the angle between the enemy's facing turned by the row's `+0x60` and the attacker, at least the
//! row's `+0x64`: `0x1402B4640`) is stopped; both sides get KH1's clash (kind 6, or 7 when 40 or more
//! under, [`resolve`]); the enemy plays the row's recoil (`+0x40`) and its script's hit handler hears
//! the clash.
//!
//! Every number here is KH1's exe; the rows' own numbers come from the player's files (the asset
//! step). Nothing of the game is in this file.

/// `0x140528CB8`: which attack types an attack of each type meets (`table[type b] & (1 << type a)`,
/// `0x1402B3BE0`). Checked (exe: the 8 bytes `1F 09 09 3F 29 18 00 00`). Symmetric.
pub const MEETS: [u8; 8] = [0x1F, 0x09, 0x09, 0x3F, 0x29, 0x18, 0x00, 0x00];

/// An attack of type `a` and one of type `b` are tested against each other (`0x1402B3BE0`).
pub fn meets(a: u32, b: u32) -> bool {
    MEETS.get(b as usize & 7).is_some_and(|m| m & (1 << (a & 7)) != 0)
}

/// Types 1..4 (`((t − 1) & ~6) == 0` on the object's bits 17..19): no hurt-body test, the front cone
/// of [`GuardRow::front`] (`0x1402B4640`). Type 3 still tests hurt bodies (`0x1402B3BE0`: `t == 3`
/// passes); 1, 2 and 4 never hurt.
pub fn is_guard_type(t: u32) -> bool {
    (1..=4).contains(&t)
}

/// An attack row of clash type `t` is tested against hurt bodies (`0x1402B3BE0`: `((t << 17) − 0x20000) &
/// 0xFFF9FFFF != 0 || t == 3` on the object's bits 17..19): every type but 1, 2 and 4. A row of those types only
/// meets other attacks: it never hurts anyone. Checked (decomp).
pub fn hurts_bodies(t: u32) -> bool {
    let bits = (t & 7) << 17;
    (bits.wrapping_sub(0x20000) & 0xFFF9_FFFF) != 0 || bits == 0x60000
}

/// The request a guard row made with KH1's mode 3 is "made with" (`sys 213(row, 3)`, `0x1402BBC10` writes the
/// mode to the object's bits 2..3; `0x1402B47A0` tests a mode-3 row every frame without looking at the owner's
/// request or HIT slots, until `sys 213(row, 0)` removes it; Checked (decomp)). M11, the Large Body's front row.
/// Such a row's manifest entry has no windows.
pub const ALWAYS: u16 = u16::MAX;

/// A side of 100 is never moved (`0x1402B4BC0`: `a.priority == 100` → 0x10, no record).
pub const UNMOVED: i32 = 100;

/// The request a guard row made in KH1's mode 3 stands for (`Cmd::Guard(Some((row, HELD)))`, M11, the
/// Neoshadow's body row `0x80`): `sys 213(row, 3)` (`0x1402BBC10`: the object's bits 2..3 = 3) makes the
/// attack object live every frame, whatever its owner plays and whatever its HIT slots say (`0x1402B47A0`:
/// mode 3 goes straight to the hit tests, `0x1402B3380`), until `sys 213(row, 0)` ends it (mode 0: the
/// object is freed, `| 0x200000`). Checked (decomp). Its manifest row has no windows.
pub const HELD: u16 = ALWAYS;
/// Clashes happen only while one side is under 98 (`0x1402B4BC0`'s first test).
pub const CLASH_BELOW: i32 = 98;
/// 40 or more under the other side: kind 7 (`diff < −0x28`), else kind 6.
pub const STRONG: i32 = 40;

/// What `0x1402B4BC0(a, b)` does to attack `a`'s owner when `a` meets `b`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// Nothing (no clash: `b`'s priority is 0, both 98 or more, `a` already clashed, `a` at 100).
    None,
    /// `a` wins (higher priority and type 1, 3 or 4): `b`'s owner gets a repel record
    /// (`0x1402BF630(b, 0, owner a)`, flag 0x20); `a` goes on. The Large Body's and the Defender's
    /// shields (not built: documented for their ports).
    Repels,
    /// `a`'s owner gets a clash record: KH1's reaction kind 6 or 7, `diff` = a − b (record `+8`;
    /// the player's kind-6 recoil pushes him back by `−diff` when it is negative, `0x14029D720`).
    Clash { kind: u8, diff: i32 },
}

/// `0x1402B4BC0(a, b)` for two attacks that meet (types and front cone already tested): `a`'s clash
/// type and priority, `b`'s priority. Not modelled: a projectile `a` (`+0x30 & 0x400`) is turned
/// to `b`'s side instead of its owner getting a record; the player-in-a-special-state test
/// (`0x140291FF0`, `+0x88 & 1`). Checked (decomp).
pub fn resolve(a_type: u32, a_priority: i32, b_priority: i32) -> Side {
    if b_priority == 0 || (a_priority >= CLASH_BELOW && b_priority >= CLASH_BELOW) {
        return Side::None;
    }
    if a_priority == UNMOVED {
        return Side::None;
    }
    let diff = a_priority - b_priority;
    if diff > 0 && matches!(a_type, 1 | 3 | 4) {
        return Side::Repels;
    }
    if a_priority < CLASH_BELOW {
        return Side::Clash { kind: if diff < -STRONG { 7 } else { 6 }, diff };
    }
    Side::None
}

/// What `0x1402B4BC0(a, b)` returns for side `a` and does (M11, attack meets attack). Checked (decomp):
/// the return is 0, 0x10 or 0x20; `0x1402B4A20` reads it (both orders) to mark the sides and pick the sparks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Met {
    /// 0: nothing (`a` already clashed this frame, `b`'s priority is 0, both 98 or more; not modelled: the
    /// owner in KH1's special state, `+0x88 & 1`).
    Nothing,
    /// 0x10 with no record and `a` not marked: `a` at 100, or at 98 / 99 against a side under 98. Its attack
    /// goes on and still meets bodies this frame.
    Unmoved,
    /// 0x10: `a` (type 1, 3 or 4, higher priority) repels; `b`'s attack record flagged 0x20 goes to `a`'s owner
    /// (no damage, no reaction; its script's hit handler hears it); `a` is marked.
    Repels,
    /// 0x10 / 0x20: `a`'s owner gets a clash record (kind 6, or 7 when 40 or more under, `diff` = a − b); `a`
    /// is marked.
    Clash { kind: u8, diff: i32 },
}

impl Met {
    /// The return value is not 0.
    pub fn returned(self) -> bool {
        self != Met::Nothing
    }

    /// `a`'s own `+4 |= 0x10 / 0x20` (a repel or a clash).
    fn marks(self) -> bool {
        matches!(self, Met::Repels | Met::Clash { .. })
    }

    /// The same as [`resolve`]'s answer.
    pub fn side(self) -> Side {
        match self {
            Met::Nothing | Met::Unmoved => Side::None,
            Met::Repels => Side::Repels,
            Met::Clash { kind, diff } => Side::Clash { kind, diff },
        }
    }
}

/// `0x1402B4BC0(a, b)` with KH1's return value ([`Met`]): `a`'s clash type and priority, whether `a` already
/// clashed this frame (its `+4 & 0x30`, cleared each frame by `0x1402B47A0`), `b`'s priority. Types and front
/// cones are tested before (`0x1402B3BE0`, `0x1402B4640`). Not modelled: a projectile `a` (`+0x30 & 0x400`:
/// turned to `b`'s side instead of a record), either owner's special state (`+0x88 & 1`). Checked (decomp).
pub fn met(a_type: u32, a_priority: i32, a_marked: bool, b_priority: i32) -> Met {
    if a_marked || b_priority == 0 || (a_priority >= CLASH_BELOW && b_priority >= CLASH_BELOW) {
        return Met::Nothing;
    }
    if a_priority == UNMOVED {
        return Met::Unmoved;
    }
    let diff = a_priority - b_priority;
    if diff > 0 && matches!(a_type, 1 | 3 | 4) {
        return Met::Repels;
    }
    if a_priority < CLASH_BELOW {
        return Met::Clash { kind: if diff < -STRONG { 7 } else { 6 }, diff };
    }
    Met::Unmoved
}

/// Two attacks that met (`0x1402B4A20`): each side's [`Met`] and whether it is marked for the rest of the frame
/// (`+4 & 0x30`: it meets no hurt body and gets no other record this frame, `0x1402B3BE0` / `0x1402B4BC0`). A
/// side marks itself by a repel or a clash; a side that returned 0 is marked by the other side's non-zero return
/// (`0x1402B4A20`: `puVar1[1] |= uVar5` when `uVar4 == 0`). Checked (decomp).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Meeting {
    pub a: Met,
    pub b: Met,
    pub a_stops: bool,
    pub b_stops: bool,
}

/// [`Meeting`] of attack `a` (clash type, priority, marked this frame) and `b`.
pub fn meeting(a: (u32, i32, bool), b: (u32, i32, bool)) -> Meeting {
    let ma = met(a.0, a.1, a.2, b.1);
    let mb = met(b.0, b.1, b.2, a.1);
    let a_stops = a.2 || ma.marks() || (ma == Met::Nothing && mb.returned());
    let b_stops = b.2 || mb.marks() || (mb == Met::Nothing && ma.returned());
    Meeting { a: ma, b: mb, a_stops, b_stops }
}

impl Meeting {
    /// Whether `b` does no harm to the player who owns `a` this frame: `b` is stopped, or `a` got a clash record. 0.198.11
    /// (fork owner, 2026-11-26: "no KH1 a defesa funciona", against the Defender's priority-100 mace; "eu preciso de uma
    /// correção geral"): a player's records are handled in his own loop (`0x14029BBD0`), the clash record first (the clash
    /// pass `0x1402B47A0` runs before the body pass in `0x1402B4460`; records are taken from the list's head), and kind 6 /
    /// 7's handlers (`0x14029D720` / `0x14029D5F0`) set flag 0x80 (`0x140297F30`), so the unstopped attack's hit record that
    /// follows does 0 HP and no reaction (`0x1402BEEE0`: HP only when `actor+0 & 0x1000080` is clear). Checked (decomp).
    pub fn a_unharmed(&self) -> bool {
        self.b_stops || matches!(self.a, Met::Clash { .. })
    }
}

/// `0x1402B4640`'s front cone of a row of clash type `t` (`+0x60` a turn of the facing, `+0x64` the cosine
/// limit): only guard types (1..4) test it (`((t − 1) & ~3) == 0`); any other type passes. The 3D cosine
/// (`0x1402A8C70`) is 0 when either vector is zero.
pub fn front(t: u32, turn: f32, cos: f32, facing_yaw: f32, to_attacker: crate::Vec3) -> bool {
    if !is_guard_type(t) {
        return true;
    }
    let f = crate::Vec3::from_yaw(facing_yaw + turn);
    let (la, lb) = (f.length(), to_attacker.length());
    let c = if la == 0.0 || lb == 0.0 { 0.0 } else { (f.dot(to_attacker) / (la * lb)).clamp(-1.0, 1.0) };
    cos <= c
}

/// One guard row of an enemy (its manifest's `guards`): the row's clash type (`+0x38 & 7`),
/// priority (`+0x3C`), recoil request (`+0x40`, low 16 bits), the front cone (`+0x60` a turn of the
/// facing in radians, `+0x64` the cosine limit) and, per request that makes it, the frames its HIT
/// slot (`+0x34`) is on in that request's motion (KH1 frames, `[on, off)`).
#[derive(Clone, Debug, PartialEq)]
pub struct GuardRow {
    pub name: String,
    pub clash_type: u32,
    pub priority: i32,
    pub recoil: u16,
    pub turn: f32,
    pub cos: f32,
    pub windows: Vec<(u16, Vec<(f32, f32)>)>,
    /// How many KH1 rows it stands for (1; the Search Ghost's burst: 4, its hands at both strikes, made
    /// with the same request and the same clash numbers: the asset step's merged rows). KH1 marks a row
    /// that met an attack as done (`+4 |= 0x10`); the others go on.
    pub rows: u8,
}

impl GuardRow {
    /// Whether the row is on at KH1 frame `t` of request `r`'s motion (`0x1402B47A0`: the slot's
    /// bit, `+0x1CC & 0x10000 << slot`). False for a request that doesn't make it.
    pub fn live(&self, r: u16, t: f32) -> bool {
        if r == ALWAYS {
            return self.always();
        }
        self.windows.iter().find(|w| w.0 == r).is_some_and(|w| w.1.iter().any(|&(a, b)| a <= t && t < b))
    }

    /// Whether the row's request list holds `r` (a mode-3 row: [`ALWAYS`]).
    pub fn made_with(&self, r: u16) -> bool {
        if r == ALWAYS {
            return self.always();
        }
        self.windows.iter().any(|w| w.0 == r)
    }

    /// A row KH1 makes with mode 3 ([`ALWAYS`]): no request makes it, so its manifest entry has no windows.
    pub fn always(&self) -> bool {
        self.windows.is_empty()
    }

    /// The front cone as a half angle about the facing (radians), when the row's cone is centred on
    /// the facing (`+0x60` = 0): `acos(+0x64)`. A turned cone (`+0x60` ≠ 0) has none (a host whose
    /// guard test is a half angle about the facing can't hold it).
    pub fn half_angle(&self) -> Option<f32> {
        (self.turn == 0.0).then(|| self.cos.clamp(-1.0, 1.0).acos())
    }

    /// `0x1402B4640`: the attacker at `to_attacker` (from the guarding enemy) is inside the cone:
    /// cos(facing turned by `+0x60`, `to_attacker`) ≥ `+0x64` (3D cosine `0x1402A8C70`, 0 when either
    /// is zero, so a zero vector passes only a cone of 90° or more).
    pub fn front(&self, facing_yaw: f32, to_attacker: crate::Vec3) -> bool {
        // Every guard row is a guard type ([`is_guard_type`]): the cone always applies.
        front(1, self.turn, self.cos, facing_yaw, to_attacker)
    }

    /// The clash an attack of type `kind` and priority `priority` gets from this row and the one the
    /// row's enemy gets, or None when they don't meet ([`meets`]).
    pub fn clash(&self, kind: u32, priority: i32) -> Option<(Side, Side)> {
        meets(kind, self.clash_type).then(|| (resolve(kind, priority, self.priority), resolve(self.clash_type, self.priority, priority)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Vec3;

    fn stance() -> GuardRow {
        // The Bandit's block stance row `0x15C` (the player's `ex_2090.bd`): type 2, priority 30,
        // recoil 232, cone 0 / 0 (the front half), slot 0 on 230 [0, 12), 231 [0, 60), 233 [0, 10).
        GuardRow { name: "stance".into(), clash_type: 2, priority: 30, recoil: 232, turn: 0.0, cos: 0.0, windows: vec![(230, vec![(0.0, 12.0)]), (231, vec![(0.0, 60.0)]), (233, vec![(0.0, 10.0)])], rows: 1 }
    }

    #[test]
    fn types_1_2_and_4_never_hurt_a_body() {
        // `0x1402B3BE0`'s body test: type 3 (the Wyvern's charge) still hurts; Sora's Guard (type 2) meets
        // neither a type 1 nor a type 2 row (`MEETS`), so such rows pass him without a block too.
        let hurts: Vec<u32> = (0..8).filter(|&t| hurts_bodies(t)).collect();
        assert_eq!(hurts, vec![0, 3, 5, 6, 7]);
        assert!(!meets(2, 1) && !meets(2, 2) && !meets(2, 4));
    }

    #[test]
    fn the_type_table_is_symmetric_and_guard_types_meet_swings() {
        for a in 0..8 {
            for b in 0..8 {
                assert_eq!(meets(a, b), meets(b, a), "{a} {b}");
            }
        }
        // A swing (type 0) meets every guard type; Fire (5) / Blizzard and Sonic Blade (6) meet no type 2.
        assert!((1..=4).all(|t| meets(0, t)));
        assert!(!meets(5, 2) && !meets(6, 2) && !meets(2, 2));
        assert!(is_guard_type(2) && !is_guard_type(0) && !is_guard_type(5));
    }

    /// Sora's swing (type 0, the Kingdom Key's 30) on the stance (30): both get kind 6, diff 0.
    /// On the spin's body row (90): Sora 7 (60 under), the Bandit 6 (a type 2 never repels).
    #[test]
    fn priorities_pick_the_kinds() {
        let s = stance();
        assert_eq!(s.clash(0, 30), Some((Side::Clash { kind: 6, diff: 0 }, Side::Clash { kind: 6, diff: 0 })));
        let spin = GuardRow { priority: 90, ..stance() };
        assert_eq!(spin.clash(0, 30), Some((Side::Clash { kind: 7, diff: -60 }, Side::Clash { kind: 6, diff: 60 })));
        // A 1-bonus Keyblade under the stance: kind 6 with a push (diff −29).
        assert_eq!(s.clash(0, 1).map(|c| c.0), Some(Side::Clash { kind: 6, diff: -29 }));
        // A type 1 / 3 / 4 row above the attacker repels it; type 2 never does.
        assert_eq!(resolve(1, 60, 30), Side::Repels);
        assert_eq!(resolve(2, 60, 30), Side::Clash { kind: 6, diff: 30 });
        // No clash against a priority 0 attack (the thrown blade's row), none at 100.
        assert_eq!(resolve(2, 30, 0), Side::None);
        assert_eq!(resolve(2, 100, 30), Side::None);
        assert_eq!(s.clash(5, 30), None);
    }

    /// The Bandit's thrown blade (its row `0x1CC`: type 0, priority 0) met by Sora's swing (type 0,
    /// the Kingdom Key's 30): Sora's side gets nothing (`b`'s priority 0), the blade's side the clash
    /// (bit 0x10; a projectile turns instead of taking a record), which its child script hears as a
    /// repel (`ex_2090_00.bd` `0x1A0`: `+4 & 0x30`).
    #[test]
    fn a_swing_knocks_the_thrown_blade_away() {
        assert!(meets(0, 0));
        assert_eq!(resolve(0, 30, 0), Side::None);
        assert_eq!(resolve(0, 0, 30), Side::Clash { kind: 6, diff: -30 });
    }

    /// The Search Ghost's burst rows (the player's `ex_2230.bd`: type 1, priority 40, cone 0 / 1.0): above the
    /// Kingdom Key's 30 they repel (Sora kind 6, 10 under); their cos 1.0 passes only an attacker exactly along
    /// the facing at the same height: never for a Ghost above or beside Sora.
    #[test]
    fn the_search_ghost_burst_rows() {
        let b = GuardRow { name: "burst".into(), clash_type: 1, priority: 40, recoil: 205, turn: 0.0, cos: 1.0, windows: vec![(204, vec![(36.0, 48.0), (108.0, 118.0)])], rows: 4 };
        assert_eq!(b.clash(0, 30), Some((Side::Clash { kind: 6, diff: -10 }, Side::Repels)));
        assert_eq!(b.clash(0, 40), Some((Side::Clash { kind: 6, diff: 0 }, Side::Clash { kind: 6, diff: 0 })));
        assert_eq!(b.clash(0, 60).map(|c| c.1), Some(Side::Clash { kind: 6, diff: -20 }));
        assert!(!b.front(0.0, Vec3::new(1.0, 0.0, 100.0)) && !b.front(0.0, Vec3::new(0.0, -50.0, 400.0)) && !b.front(0.0, Vec3::new(0.0, 0.0, -100.0)));
        assert!(b.front(0.0, Vec3::new(0.0, 0.0, 100.0)), "exactly ahead, level");
        assert!(b.live(204, 36.0) && !b.live(204, 48.0) && b.live(204, 117.0) && !b.live(204, 60.0));
        assert_eq!(b.half_angle(), Some(0.0));
    }

    /// The Large Body's front row (the player's `ex_2050.bd` `0x38C`: type 1, priority 100, recoil 110, cone turn 0 /
    /// cos 0.34202 = 70°, mode 3): on whatever plays; against the Kingdom Key's 30 Sora gets kind 7 (70 under) and
    /// the Large Body nothing (a side at 100 is never moved); a 98+ swing meets no clash at all.
    #[test]
    fn the_large_body_front_row() {
        let f = GuardRow { name: "front".into(), clash_type: 1, priority: 100, recoil: 110, turn: 0.0, cos: 0.342_02, windows: vec![], rows: 1 };
        assert!(f.always() && f.live(ALWAYS, 0.0) && f.made_with(ALWAYS) && !f.live(200, 10.0) && !f.made_with(200));
        assert_eq!(f.clash(0, 30), Some((Side::Clash { kind: 7, diff: -70 }, Side::None)));
        assert_eq!(f.clash(0, 98), Some((Side::None, Side::None)));
        assert_eq!(f.clash(5, 30), None, "magic (type 5) never meets a type-1 row");
        let half = f.half_angle().unwrap();
        assert!((half.to_degrees() - 70.0).abs() < 0.01, "{}", half.to_degrees());
        // Facing +z: ahead and 60° off pass; 80° off and behind don't; a Sora 2 m ahead and 2 m above is 45° up.
        let at = |deg: f32| Vec3::new(deg.to_radians().sin(), 0.0, deg.to_radians().cos()) * 200.0;
        assert!(f.front(0.0, at(0.0)) && f.front(0.0, at(60.0)) && !f.front(0.0, at(80.0)) && !f.front(0.0, at(180.0)));
        assert!(f.front(0.0, Vec3::new(0.0, -200.0, 200.0)) && !f.front(0.0, Vec3::new(0.0, -600.0, 200.0)));
    }

    /// Attack meets attack (M11, `0x1402B4A20` / `0x1402B4BC0`): Sora's swing (type 0, the Kingdom Key's 30)
    /// against enemy attack rows (type 0) with the priorities the players' scripts give them.
    #[test]
    fn attack_meets_attack() {
        let sora = |p: i32| (0u32, p, false);
        // The Large Body's punch (30): both kind 6, no push; both stop (no damage either way this frame).
        let m = meeting(sora(30), (0, 30, false));
        assert_eq!((m.a, m.b), (Met::Clash { kind: 6, diff: 0 }, Met::Clash { kind: 6, diff: 0 }));
        assert!(m.a_stops && m.b_stops);
        // Its charge's belly (90): Sora the strong recoil (60 under), the Large Body kind 6 (a type 0 never repels).
        let m = meeting(sora(30), (0, 90, false));
        assert_eq!((m.a, m.b), (Met::Clash { kind: 7, diff: -60 }, Met::Clash { kind: 6, diff: 60 }));
        // The Soldier's kick (20): Sora kind 6 above it (no push), the Soldier kind 6 with its push.
        let m = meeting(sora(30), (0, 20, false));
        assert_eq!((m.a, m.b), (Met::Clash { kind: 6, diff: 10 }, Met::Clash { kind: 6, diff: -10 }));
        // A row at 100 (the Neoshadow's dive, the Wyvern's bite): Sora bounces; it is never moved and still meets
        // bodies this frame.
        let m = meeting(sora(30), (0, 100, false));
        assert_eq!((m.a, m.b), (Met::Clash { kind: 7, diff: -70 }, Met::Unmoved));
        assert!(m.a_stops && !m.b_stops);
        // A 98+ swing against a 30 row: the swing goes on (no record, not marked), the row gets kind 7 and stops.
        let m = meeting(sora(99), (0, 30, false));
        assert_eq!((m.a, m.b), (Met::Unmoved, Met::Clash { kind: 7, diff: -69 }));
        assert!(!m.a_stops && m.b_stops);
        // Both 98 or more: nothing at all.
        let m = meeting(sora(99), (0, 100, false));
        assert_eq!((m.a, m.b, m.a_stops, m.b_stops), (Met::Nothing, Met::Nothing, false, false));
        // A swing that already clashed this frame gets nothing more; the row it meets still takes its clash, and
        // the swing stays marked.
        let m = meeting((0, 30, true), (0, 30, false));
        assert_eq!((m.a, m.b, m.a_stops, m.b_stops), (Met::Nothing, Met::Clash { kind: 6, diff: 0 }, true, true));
        // A row of priority 0 (the thrown blade's): the swing returns 0 but is marked by the row's clash.
        let m = meeting(sora(30), (0, 0, false));
        assert_eq!((m.a, m.b, m.a_stops), (Met::Nothing, Met::Clash { kind: 6, diff: -30 }, true));
        // A type 1 / 3 / 4 row above the swing (under 100) repels it.
        let m = meeting(sora(30), (3, 60, false));
        assert_eq!((m.a, m.b), (Met::Clash { kind: 6, diff: -30 }, Met::Repels));
        // `resolve` is `met`'s side.
        for (a, b) in [(30, 30), (30, 90), (30, 100), (99, 30), (99, 100), (30, 0)] {
            assert_eq!(resolve(0, a, b), met(0, a, false, b).side(), "{a} {b}");
        }
        // Only guard types test the cone.
        assert!(front(0, 0.0, 1.0, 0.0, Vec3::new(0.0, 0.0, -100.0)));
        assert!(!front(2, 0.0, 0.0, 0.0, Vec3::new(0.0, 0.0, -100.0)) && front(2, 0.0, 0.0, 0.0, Vec3::new(0.0, 0.0, 100.0)));
        // The Soldier's spin-kick chest row (type 1, cos −0.5: 120° each side).
        assert!(front(1, 0.0, -0.5, 0.0, Vec3::new(100.0, 0.0, -50.0)) && !front(1, 0.0, -0.5, 0.0, Vec3::new(10.0, 0.0, -100.0)));
    }

    #[test]
    fn windows_and_cone() {
        let s = stance();
        assert!(s.live(231, 59.9) && !s.live(233, 10.0) && s.live(230, 0.0) && !s.live(200, 1.0));
        assert!(s.made_with(233) && !s.made_with(232));
        assert_eq!(s.half_angle(), Some(std::f32::consts::FRAC_PI_2));
        assert_eq!(GuardRow { turn: 0.5, ..stance() }.half_angle(), None);
        // Facing +z (yaw 0): in front, at the side (90°: cos 0 passes), behind.
        assert!(s.front(0.0, Vec3::new(0.0, 0.0, 100.0)) && s.front(0.0, Vec3::new(100.0, 0.0, 0.0)) && !s.front(0.0, Vec3::new(0.0, 0.0, -100.0)));
    }
}
