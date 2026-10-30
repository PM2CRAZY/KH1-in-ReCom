//! KH1's battle data as plain structs: action rows, attack records, the weapon row, the damage
//! curve, and Sora's moveset. `kh1-data` fills these from `btltbl.bin` and `xa_ex_0010.mset`;
//! [`Moveset::kh1_defaults`] holds the values read from the owner's files
//! (`docs/kh1-sora-data.md`), so tests and the demo run without them.

use std::collections::HashMap;

use crate::motion::{FlagKind, MotionInfo, MotionSet};

/// One action row (btltbl `0xA9F0 + row × 0x14` for Sora; read by `0x1402A3980` and
/// `0x1402A68F0`). Checked (decomp) for the layout.
#[derive(Clone, Debug, PartialEq)]
pub struct ActionRow {
    /// +0x0: motion request id (played `| 0x1000000` at blend 0). < 0 = none.
    pub request: i16,
    /// +0x2: weapon trail id (−1 none).
    pub trail: i8,
    /// +0x3: weapon effect (−1 none).
    pub weapon_effect: i8,
    /// +0x4: attack record offset into btltbl.
    pub attack: u32,
    /// +0x8: hop height (> 0 starts an arc). Rows 0 / 1 use the picker's height instead.
    pub hop: f32,
    /// +0xC / +0xD / +0xE: body / wall / critical hit sound index (names: Assumption).
    pub sounds: [u8; 3],
    /// +0xF: voice group (−1 none).
    pub voice: i8,
}

impl ActionRow {
    pub fn none() -> ActionRow {
        ActionRow { request: -1, trail: -1, weapon_effect: -1, attack: 0, hop: 0.0, sounds: [0; 3], voice: -1 }
    }

    fn new(request: i16, attack: u32, hop: f32, sounds: [u8; 3], voice: i8) -> ActionRow {
        ActionRow { request, trail: 5, weapon_effect: -1, attack, hop, sounds, voice }
    }
}

/// One 0x70-byte attack record (constructor `0x1402B2DC0`, damage `0x1402BFA80`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AttackData {
    /// +0x00: stat source: 0 Strength, 1 Magic (stats +0x48), 2 none, 3/4/5 Magic ×2/×3/×5,
    /// 6 the leader's Strength.
    pub stat_source: u8,
    /// +0x04 & 0xF: element (8 = % of HP, 9/11/12 = power as damage).
    pub element: u8,
    /// +0x08 & 0x1F: reaction kind against the party.
    pub kind_party: u8,
    /// +0x0C: reaction kind against enemies (3 flinch, 4 knock-back, 5 launch).
    pub kind: u8,
    /// +0x10: bone (0x30 = the weapon; < 0 = free-standing).
    pub bone: i32,
    /// +0x14 / +0x20 / +0x24: power, min, max against the party side.
    pub power_party: i32,
    pub min_party: i32,
    pub max_party: i32,
    /// +0x18: crit chance in 5 % units (× 0.05 × crit factor × weapon crit × 0.05).
    pub crit: i32,
    /// +0x1C: crit power bonus.
    pub crit_bonus: i32,
    /// +0x2C: MP charge units per hit (30 = 1 MP).
    pub mp_charge: i32,
    /// +0x34 & 3: hit slot (flag track 0x10000 << slot).
    pub hit_slot: u8,
    /// +0x38: bit 16 = spent-on-birth flag; bits 0..2 = clash type (2 = pure guard).
    pub clash: u32,
    /// +0x3C: priority.
    pub priority: i32,
    /// +0x40: clash-recoil motion request (110). Assumption (notes) for the meaning.
    pub recoil_request: i32,
    /// +0x48 / +0x4C / +0x50: power, min, max against enemies.
    pub power: i32,
    pub min: i32,
    pub max: i32,
    /// +0x54 / +0x58: blade fractions A / B (× weapon +0x2C).
    pub frac_a: f32,
    pub frac_b: f32,
    /// +0x5C: radius (an f32; + actor scale × weapon radius).
    pub radius: f32,
}

impl AttackData {
    pub fn clash_type(&self) -> u32 {
        self.clash & 7
    }

    /// Clash types 1, 2 and 4 build no swept chain (`0x1402B3800`); type 2 is the pure guard.
    pub fn sweeps(&self) -> bool {
        !matches!(self.clash_type(), 1 | 2 | 4)
    }
}

/// The Keyblade's weapon row (btltbl `0x94F8`, 0x58 bytes), the fields the hit code reads
/// (`0x1402B3270`, `0x14029EB20`).
#[derive(Clone, Debug, PartialEq)]
pub struct WeaponData {
    /// +0x20: hit radius (× actor scale, added to the record's radius).
    pub radius: f32,
    /// +0x24 / +0x28: blade span: bone parameter t = near + (far − near) × fraction.
    pub blade_far: f32,
    pub blade_near: f32,
    /// +0x2C: fraction scale.
    pub frac_scale: f32,
    /// +0x30: Strength bonus (already in the refreshed Strength stat).
    pub strength: i32,
    /// +0x31: crit factor (× 0.05).
    pub crit: u8,
    /// +0x32: crit bonus.
    pub crit_bonus: i8,
    /// +0x33: priority bonus.
    pub priority: u8,
}

impl WeaponData {
    /// Kingdom Key (item 0x51). Checked (files).
    pub fn kingdom_key() -> WeaponData {
        WeaponData { radius: 40.0, blade_far: -3.7, blade_near: 0.0, frac_scale: 0.8, strength: 3, crit: 20, crit_bonus: 0, priority: 30 }
    }

    /// `0x14029EB20` for bone 0x30: the bone parameter for a blade fraction (already scaled).
    pub fn blade_t(&self, frac: f32) -> f32 {
        (self.blade_far - self.blade_near) * frac + self.blade_near
    }
}

/// The Strength − Defence curve at btltbl file 0x00 (u16 count, then values). Checked (files).
#[derive(Clone, Debug, PartialEq)]
pub struct DamageCurve(pub Vec<u16>);

impl DamageCurve {
    pub fn kh1_defaults() -> DamageCurve {
        let mut v = vec![1, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 17, 20, 24, 28, 34, 40, 48, 57, 68, 80, 95, 113, 135, 160, 190, 227, 270];
        v.extend(std::iter::repeat_n(300, 13));
        DamageCurve(v)
    }
}

/// Rows the controller names.
pub mod row {
    pub const AERIAL_SWEEP: usize = 0;
    pub const JUMP_UP: usize = 1;
    pub const HOP: usize = 2;
    pub const STUN_IMPACT: usize = 3;
    pub const GRAVITY_BREAK: usize = 4;
    pub const ZANTETSUKEN: usize = 5;
    pub const BLITZ: usize = 6;
    pub const RIPPLE_DRIVE: usize = 7;
    pub const GROUND_FINISHER: usize = 8;
    pub const SLIDING_DASH: usize = 9;
    pub const VORTEX: usize = 10;
    pub const SWING_2: usize = 11;
    pub const SWING_1: usize = 13;
    pub const LUNGE: usize = 14;
    pub const HURRICANE_BLAST: usize = 15;
    pub const AIR_FINISHER: usize = 16;
    pub const AIR_SWING_2: usize = 17;
    pub const AIR_SWING_1: usize = 18;
    pub const GUARD: usize = 0x3D;
    pub const COUNTERATTACK: usize = 0x3E;
    /// Rows the ground picker walks (0..=0x41).
    pub const GROUND_COUNT: usize = 0x42;
}

/// Everything the controller needs about Sora's moves.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Moveset {
    pub motions: MotionSet,
    /// Sora's action rows, indexed by row id.
    pub rows: Vec<ActionRow>,
    /// Attack records by btltbl offset.
    pub attacks: HashMap<u32, AttackData>,
}

impl Moveset {
    pub fn row(&self, r: usize) -> Option<&ActionRow> {
        self.rows.get(r)
    }

    /// The row's motion request, or −1.
    pub fn row_request(&self, r: usize) -> i32 {
        self.rows.get(r).map_or(-1, |x| x.request as i32)
    }

    pub fn attack(&self, off: u32) -> Option<&AttackData> {
        self.attacks.get(&off)
    }

    /// Sora's moveset from the owner's files (`docs/kh1-sora-data.md`). Motions not listed there
    /// (walk/run lengths aside) are absent. All Checked (files) unless marked.
    pub fn kh1_defaults() -> Moveset {
        use FlagKind::*;
        let mut m = MotionSet::default();
        let inf = f32::INFINITY;
        // Entry indices follow the mset (so requests that share an entry share it here).
        let mut put = |entry: usize, reqs: &[u16], info: MotionInfo| {
            if m.entries.len() <= entry {
                m.entries.resize(entry + 1, MotionInfo::new(0.0));
            }
            m.entries[entry] = info;
            for &r in reqs {
                m.map_request(r, entry);
            }
        };
        // Assumption: the battle variants (entry 1 idle, 3 walk, 6 run) are the ones the build
        // step maps (pose.rs: field 0/2/5, battle 1/3/6).
        put(1, &[0], MotionInfo::new(170.0));
        put(3, &[1], MotionInfo::new(60.0));
        put(6, &[2], MotionInfo::new(36.0));
        put(8, &[4], MotionInfo::new(26.0));
        put(9, &[5], MotionInfo::new(14.0));
        put(10, &[6, 11], MotionInfo::new(40.0));
        put(11, &[7, 12], MotionInfo::new(28.0).with_flag(Free, 12.0, inf));
        put(12, &[9], MotionInfo::new(34.0));
        put(13, &[10], MotionInfo::new(26.0));
        put(75, &[0x71], MotionInfo::new(120.0));
        put(76, &[0x72], MotionInfo::new(60.0));
        put(77, &[0x73], MotionInfo::new(60.0));
        put(78, &[0x74], MotionInfo::new(60.0));
        put(98, &[200], MotionInfo::new(48.0).with_flag(Hit(0), 14.0, 24.0).with_flag(ComboWindow, 22.0, 40.0).with_flag(Trail, 4.0, 24.0).with_forward_keys(&[(0.0, 0.0), (8.0, 10.4), (14.0, 48.33), (34.0, 100.0), (48.0, 100.0)]));
        put(99, &[201], MotionInfo::new(48.0).with_flag(Hit(0), 18.0, 32.0).with_flag(ComboWindow, 26.0, 42.0).with_flag(Trail, 12.0, 32.0).with_forward_keys(&[(0.0, 0.0), (12.0, 0.0), (24.0, 57.93), (48.0, 120.0)]));
        put(100, &[202], MotionInfo::new(64.0).with_flag(Hit(0), 18.0, 30.0).with_flag(ComboWindow, 26.0, 54.0).with_flag(Trail, 18.0, 30.0).with_forward_keys(&[(0.0, 0.0), (18.0, 0.01), (20.0, 10.84), (24.0, 81.86), (28.0, 118.98), (32.0, 129.64), (40.0, 134.5), (64.0, 134.94)]));
        put(101, &[203], MotionInfo::new(80.0).with_flag(Hit(0), 26.0, 38.0).with_flag(Trail, 8.0, 38.0).with_forward_keys(&[(0.0, 0.0), (20.0, 44.33), (26.0, 97.62), (32.0, 119.43), (56.0, 136.11), (68.0, 146.33), (80.0, 190.0)]));
        put(102, &[204], MotionInfo::new(40.0).with_flag(Hit(0), 6.0, 20.0).with_flag(ComboWindow, 18.0, 34.0).with_flag(Trail, 2.0, 20.0));
        put(103, &[205], MotionInfo::new(42.0).with_flag(Hit(0), 10.0, 20.0).with_flag(ComboWindow, 18.0, 32.0).with_flag(Trail, 4.0, 20.0));
        put(104, &[206], MotionInfo::new(56.0).with_flag(Hit(0), 12.0, 38.0).with_flag(Trail, 6.0, 38.0));
        let mut guard = MotionInfo::new(54.0).with_flag(Hit(0), 10.0, 36.0).with_flag(ComboWindow, 22.0, 54.0);
        guard.root_motion = true; // flag set, no travel
        put(110, &[212], guard);
        // Entry 72, request 0x6E: the guard-success / repel recoil (60 frames, no travel). Checked
        // (files: free 28–60, combo window 28–60).
        put(72, &[0x6E], MotionInfo::new(60.0).with_flag(Free, 28.0, inf).with_flag(ComboWindow, 28.0, 60.0));
        put(111, &[213], MotionInfo::new(86.0).with_flag(Hit(0), 8.0, 24.0).with_flag(ComboWindow, 18.0, 56.0).with_flag(Trail, 4.0, 24.0).with_forward_keys(&[(0.0, 0.0), (4.0, 12.86), (6.0, 44.08), (14.0, 163.77), (24.0, 237.5), (36.0, 267.05), (86.0, 267.05)]));
        put(117, &[0xDC], MotionInfo::new(38.0).with_flag(Free, 30.0, inf).with_forward_keys(&[(0.0, 0.0), (12.0, 213.82), (24.0, 350.0), (38.0, 350.0)]));

        let mut rows = vec![ActionRow::none(); 0x42];
        rows[0] = ActionRow::new(214, 0x4CC8, 0.0, [3, 5, 6], 4);
        rows[1] = ActionRow::new(205, 0x4BE8, 0.0, [4, 5, 6], 4);
        rows[2] = ActionRow::new(204, 0x4B78, 60.0, [0, 5, 6], 4);
        rows[3] = ActionRow::new(216, 0x4EF8, 0.0, [27, 5, 27], 6);
        rows[4] = ActionRow::new(217, 0x4F68, 0.0, [23, 5, 23], 6);
        rows[5] = ActionRow::new(218, 0x4FD8, 0.0, [24, 5, 24], 6);
        rows[6] = ActionRow::new(210, 0x4D38, 0.0, [27, 5, 6], 6);
        rows[7] = ActionRow::new(215, 0x4E88, 0.0, [23, 5, 23], 5);
        rows[8] = ActionRow::new(203, 0x4C58, 0.0, [3, 5, 6], 5);
        rows[9] = ActionRow::new(208, 0x50B8, 0.0, [2, 5, 6], 4);
        rows[10] = ActionRow::new(211, 0x4CC8, 0.0, [1, 5, 6], 4);
        rows[11] = ActionRow::new(201, 0x4BE8, 0.0, [1, 5, 6], 4);
        rows[12] = ActionRow::new(207, 0x5128, 0.0, [0, 5, 6], 4);
        rows[13] = ActionRow::new(200, 0x4B78, 0.0, [0, 5, 6], 4);
        rows[14] = ActionRow::new(202, 0x4B78, 0.0, [2, 5, 6], 4);
        rows[15] = ActionRow::new(209, 0x5048, 40.0, [27, 5, 6], 5);
        rows[16] = ActionRow::new(206, 0x4C58, 40.0, [3, 5, 6], 5);
        rows[17] = ActionRow::new(205, 0x4BE8, 20.0, [4, 5, 6], 4);
        rows[18] = ActionRow::new(204, 0x4B78, 20.0, [0, 5, 6], 4);
        rows[19] = ActionRow::new(206, 0x4C58, 8.0, [3, 5, 6], 5);
        rows[20] = ActionRow::new(205, 0x4B78, 8.0, [4, 5, 6], 4);
        rows[21] = ActionRow::new(204, 0x4B78, 8.0, [0, 5, 6], 4);
        rows[0x3D] = ActionRow::new(212, 0x4DA8, 0.0, [0, 5, 6], 19);
        rows[0x3E] = ActionRow::new(213, 0x4E18, 0.0, [3, 5, 6], 5);

        let rec = |kind: u8, power: i32, crit: i32, crit_bonus: i32, mp: i32, clash: u32, frac_b: f32, radius: f32| AttackData {
            stat_source: 0,
            element: 1,
            kind_party: kind,
            kind,
            bone: 0x30,
            power_party: power,
            crit,
            crit_bonus,
            mp_charge: mp,
            clash,
            recoil_request: 110,
            power,
            frac_a: 1.0,
            frac_b,
            radius,
            ..Default::default()
        };
        let mut attacks = HashMap::new();
        attacks.insert(0x4B78, rec(3, 0, 0, 0, 3, 0, 0.0, 0.0));
        attacks.insert(0x4BE8, rec(3, 0, 0, 0, 3, 0x10000, 0.0, 0.0));
        attacks.insert(0x4CC8, rec(3, 2, 0, 0, 3, 0x10000, 0.0, 0.0));
        attacks.insert(0x4C58, rec(5, 4, 4, 4, 6, 0x10000, 0.0, 0.0));
        attacks.insert(0x4D38, rec(5, 6, 4, 4, 6, 0, 0.0, 10.0));
        attacks.insert(0x5048, rec(5, 6, 4, 4, 6, 0x10000, -0.4, 20.0));
        attacks.insert(0x50B8, rec(3, 0, 0, 0, 3, 0x10000, 0.0, 30.0));
        attacks.insert(0x5128, rec(3, 0, 4, 4, 3, 0, 0.0, 10.0));
        attacks.insert(0x4DA8, rec(0, 0, 0, 0, 0, 0x10002, 0.0, 0.0));
        attacks.insert(0x4E18, rec(3, 0, 200, 4, 24, 0, 0.0, 10.0));
        Moveset { motions: m, rows, attacks }
    }
}
