//! KH1 `btltbl.bin` (71,232 bytes): the damage curve, the battle globals, Sora's action rows,
//! attack records, the item table and the weapon table.
//!
//! The exe loads the file whole at `0x142D22D40` (`0x14028F9D0`), so exe address = that +
//! file offset. Offsets here are file offsets. Values are Checked (files) and their readers
//! Checked (decomp) in `kh1-data-values.md` unless a field's doc says Assumption.

use crate::reader::Bytes;
use crate::{bad, Result};

/// File offset of the damage curve (u16 count, then u16 values).
pub const CURVE: usize = 0x00;
/// File offset of the globals block (`DAT_142D22D30`).
pub const GLOBALS: usize = 0x58;
/// Size of the globals block read here.
pub const GLOBALS_SIZE: usize = 0x100;
/// File offset of the item table (0x14-byte items; item id `i` at `ITEMS + (i - 1) * 0x14`).
pub const ITEMS: usize = 0x1A58;
/// Size of one item row.
pub const ITEM_SIZE: usize = 0x14;
/// File offset of the weapon table (0x58-byte rows; row `r` (1-based) at `WEAPONS + (r - 1) *
/// 0x58`).
pub const WEAPONS: usize = 0x94F8;
/// Size of one weapon row.
pub const WEAPON_SIZE: usize = 0x58;
/// File offset of Sora's action rows (`DAT_142D5EC40 = btltbl + 0xA9F0`, set by `0x1402A6180`).
pub const SORA_ROWS: usize = 0xA9F0;
/// Size of one action row.
pub const ROW_SIZE: usize = 0x14;
/// Rows in Sora's table (0x00..=0x41; from row 0x42 on the rows use other attack data and no
/// trail: Assumption that they belong to another character).
pub const SORA_ROW_COUNT: usize = 0x42;
/// Size of one attack record.
pub const ATTACK_SIZE: usize = 0x70;

/// Sora's action-row indices used by the combo pickers (`kh1-data-values.md` §1).
pub mod rows {
    /// Aerial Sweep (jump-up).
    pub const AERIAL_SWEEP: usize = 0;
    /// Jump-up attack.
    pub const JUMP_UP: usize = 1;
    /// Obstacle hop attack.
    pub const HOP: usize = 2;
    /// Stun Impact.
    pub const STUN_IMPACT: usize = 3;
    /// Gravity Break.
    pub const GRAVITY_BREAK: usize = 4;
    /// Zantetsuken.
    pub const ZANTETSUKEN: usize = 5;
    /// Blitz.
    pub const BLITZ: usize = 6;
    /// Ripple Drive.
    pub const RIPPLE_DRIVE: usize = 7;
    /// Ground finisher.
    pub const GROUND_FINISHER: usize = 8;
    /// Sliding Dash.
    pub const SLIDING_DASH: usize = 9;
    /// Vortex.
    pub const VORTEX: usize = 10;
    /// Ground swing 2 / side swing.
    pub const GROUND_2: usize = 11;
    /// Ground swing 1 (basic).
    pub const GROUND_1: usize = 13;
    /// Lunge.
    pub const LUNGE: usize = 14;
    /// Hurricane Blast.
    pub const HURRICANE_BLAST: usize = 15;
    /// Air finisher.
    pub const AIR_FINISHER: usize = 16;
    /// Air swing, target below.
    pub const AIR_2: usize = 17;
    /// Air swing 1 (basic).
    pub const AIR_1: usize = 18;
    /// Guard.
    pub const GUARD: usize = 0x3D;
    /// Counterattack.
    pub const COUNTER: usize = 0x3E;
}

/// The damage curve: damage = `values[clamp(power - defence, 0, len - 1)]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DamageCurve {
    /// The values (Sora's file: 42).
    pub values: Vec<u16>,
}

impl DamageCurve {
    /// The value for `power - defence`, clamped to the table as `0x1402BFA80` does.
    pub fn at(&self, diff: i32) -> u16 {
        let n = self.values.len() as i32;
        if n == 0 {
            return 0;
        }
        self.values[diff.clamp(0, n - 1) as usize]
    }
}

/// The globals block at file 0x58 (`G`). Field docs give `G` offsets.
#[derive(Clone, Debug, PartialEq)]
pub struct Globals {
    /// +0x00: narrow target cone, degrees (45: cos(22.5 deg)).
    pub narrow_cone_deg: u32,
    /// +0x04: wide target cone, degrees (240: cos(120 deg)).
    pub wide_cone_deg: u32,
    /// +0x08: close range; a target nearer than this gets the normal swing (180).
    pub close_range: u32,
    /// +0x0C: lunge range; between close and this, the lunge row (300).
    pub lunge_range: u32,
    /// +0x10: party-side hit kind threshold, % of max HP below which a hit flinches (20).
    pub party_flinch_pct: u8,
    /// +0x11: party-side threshold at or above which a hit knocks back far (30).
    pub party_knock_pct: u8,
    /// +0x14: enemy launch speed minimum (20).
    pub launch_speed_min: u16,
    /// +0x16: enemy launch speed maximum (35).
    pub launch_speed_max: u16,
    /// +0x18: launch height = speed x this (1.35).
    pub launch_height_factor: f32,
    /// +0x1C..+0x34: Sora-side hit-reaction pushes (2.5, 750, 150, 4, 80, 40, 8; not decoded).
    pub reaction_pushes: [f32; 7],
    /// +0x38 / +0x3C: enemy air knock-back speeds (4, 16; meaning Assumption (notes)).
    pub air_knockback: [f32; 2],
    /// +0x54 + c*4: per character (u16 jump height, u16 with High Jump). Sora is c = 0
    /// (190 / 290); the order of the others is an Assumption.
    pub jump_heights: [(u16, u16); 10],
    /// +0x88 / +0x8C: EXP / level scaling (0.5 / 0.5; not traced).
    pub exp_scale: [f32; 2],
    /// +0x90: timer step multiplier in `0x14029CA60` (8; not traced).
    pub timer_step: f32,
    /// +0x94: MP charge on hit, % per enemy level above Sora's (20).
    pub mp_charge_level_pct: u8,
    /// +0x95: MP charge with an ability bit set, % (150; which ability: unknown).
    pub mp_charge_ability_pct: u8,
    /// +0x96: MP charge cap, % of the record's charge (250).
    pub mp_charge_cap_pct: u16,
    /// +0xE8: hop for an air command (30).
    pub air_command_hop: f32,
    /// +0xF0: air velocity scale on the air cast / air item (0.5).
    pub air_cast_velocity_scale: f32,
    /// +0xF4 + 2n: per-member values (200, 300, 200, 300, 300; not traced).
    pub member_values: [u16; 5],
    /// +0xFE: Strength bonus with an ability flag (4).
    pub strength_ability_bonus: u8,
    /// The whole block's raw bytes (0x100), for fields not decoded.
    pub raw: Vec<u8>,
}

impl Globals {
    /// Sora's normal jump height (190).
    pub fn sora_jump(&self) -> u16 {
        self.jump_heights[0].0
    }
    /// Sora's jump height with High Jump (290).
    pub fn sora_high_jump(&self) -> u16 {
        self.jump_heights[0].1
    }
}

/// One action row (0x14 bytes; readers `0x1402A3980`, `0x1402A68F0`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActionRow {
    /// +0x0: motion request id (through the mset request map), played with the action lock.
    pub motion: i16,
    /// +0x2: weapon trail id (`0x1402C2310`; 5 on every combo row).
    pub trail: i8,
    /// +0x3: weapon effect (`0x1402ADCF0`; -1 = none).
    pub weapon_effect: i8,
    /// +0x4: attack-record offset into btltbl.
    pub attack: u32,
    /// +0x8: hop height (> 0: a hop with rise time sqrt(2h/0.48469388); rows 19-21 use it as a
    /// speed).
    pub hop: f32,
    /// +0xC: body-hit sound index (sound = weapon sound base + this).
    pub hit_sound: u8,
    /// +0xD: wall / terrain hit sound index.
    pub wall_sound: u8,
    /// +0xE: critical-hit sound index. (The three names: Assumption from the branches.)
    pub crit_sound: u8,
    /// +0xF: voice group (`0x1402901F0`).
    pub voice: i8,
    /// +0x10: 0 in every row; no reader found.
    pub tail: u32,
}

/// One attack record (0x70 bytes; constructor `0x1402B2DC0`, damage `0x1402BFA80`). "Party"
/// fields apply against party-side targets, the others against enemies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttackData {
    /// File offset of the record.
    pub offset: usize,
    /// +0x00: stat source: 0 Strength, 1 ST+0x48, 2 none, 3/4/5 ST+0x48 x2/x3/x5, 6 leader's
    /// Strength.
    pub stat_source: i32,
    /// +0x04: element (& 0xF; 8 = % of HP, 9 = fixed, 10 = stun).
    pub element: i32,
    /// +0x08: reaction kind vs party (& 0x1F).
    pub kind_party: i32,
    /// +0x0C: reaction kind vs enemies (3 flinch, 4 knock-back, 5 launch, 0 none).
    pub kind_enemy: i32,
    /// +0x10: bone (0x30 = the weapon; < 0 free-standing).
    pub bone: i32,
    /// +0x14: power vs party.
    pub power_party: i32,
    /// +0x18: crit chance (x 0.05 x ST+0xD8).
    pub crit_chance: i32,
    /// +0x1C: crit bonus.
    pub crit_bonus: i32,
    /// +0x20: damage minimum vs party (0 = no clamp).
    pub min_party: i32,
    /// +0x24: damage maximum vs party (0 = no clamp).
    pub max_party: i32,
    /// +0x28: effect bit.
    pub effect_bit: i32,
    /// +0x2C: MP charge per landed hit (30 units = 1 MP).
    pub mp_charge: i32,
    /// +0x30: MP cost.
    pub mp_cost: i32,
    /// +0x34: flags; low 2 bits = the hit slot.
    pub flags_34: u32,
    /// +0x38: clash word: bits 0-2 clash type, bit 16 "spent" (meaning Assumption (notes)).
    pub clash: u32,
    /// +0x3C: priority (the weapon's is added).
    pub priority: i32,
    /// +0x40: clash-reaction motion request (110 = Sora's recoil; Assumption (notes)).
    pub clash_motion: i32,
    /// +0x44: damage to object actors (Assumption (notes)).
    pub object_damage: i32,
    /// +0x48: power vs enemies (+ the stat source).
    pub power: i32,
    /// +0x4C: damage minimum vs enemies (0 = no clamp).
    pub min: i32,
    /// +0x50: damage maximum vs enemies (0 = no clamp).
    pub max: i32,
    /// +0x54: blade fraction A (x the weapon's fraction scale).
    pub frac_a: f32,
    /// +0x58: blade fraction B.
    pub frac_b: f32,
    /// +0x5C: sphere radius (f32; the weapon's radius x actor scale is added).
    pub radius: f32,
    /// +0x60: directional arc turn (Assumption (notes)).
    pub arc_turn: f32,
    /// +0x64: directional arc cosine (1.0 = all round; Guard: cos 100 deg).
    pub arc_cos: f32,
    /// +0x68: x ST+0x68 into attack object +0xB0 (meaning not traced).
    pub f68: f32,
    /// +0x6C: raw (small numbers; looks like a record id).
    pub id_6c: u32,
}

impl AttackData {
    /// The flag-track slot whose hit bit drives this attack (`0x10000 << slot`).
    pub fn hit_slot(&self) -> u8 {
        (self.flags_34 & 3) as u8
    }
    /// Clash type (+0x38 bits 0-2; 2 = Guard's).
    pub fn clash_type(&self) -> u8 {
        (self.clash & 7) as u8
    }
    /// +0x38 bit 16 (attack object flag 0x100000).
    pub fn spent(&self) -> bool {
        self.clash & 0x10000 != 0
    }
}

/// One weapon row (0x58 bytes; readers `0x1402B3270`, `0x14029EB20`).
#[derive(Clone, Debug, PartialEq)]
pub struct Weapon {
    /// File offset of the row.
    pub offset: usize,
    /// +0x00: model name (e.g. `xw_ex_5010`, the Kingdom Key).
    pub model: String,
    /// +0x20: hit radius (x actor scale; Kingdom Key 40).
    pub radius: f32,
    /// +0x24: blade parameter at fraction 1 (lerp(+0x28, +0x24, f) along the weapon bone).
    pub blade_far: f32,
    /// +0x28: blade parameter at fraction 0.
    pub blade_near: f32,
    /// +0x2C: scale applied to the record's fractions.
    pub frac_scale: f32,
    /// +0x30: Strength bonus.
    pub strength: i8,
    /// +0x31: crit factor (x 0.05).
    pub crit: u8,
    /// +0x32: crit bonus.
    pub crit_bonus: i8,
    /// +0x33: priority bonus.
    pub priority: u8,
    /// +0x34: hit-sound id base.
    pub sound_base: u32,
    /// +0x38: Magic bonus.
    pub magic: i8,
}

/// One item row (0x14 bytes), the two fields used here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Item {
    /// Item id.
    pub id: u16,
    /// +0x2: kind (2 = Sora's weapons).
    pub kind: u8,
    /// +0x6: weapon row (1-based) for weapons.
    pub weapon_row: i16,
}

/// A `btltbl.bin` file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Btltbl {
    /// The file bytes.
    pub data: Vec<u8>,
}

impl Btltbl {
    /// Takes the file's bytes (checks only that the fixed tables fit).
    pub fn parse(data: Vec<u8>) -> Result<Btltbl> {
        if data.len() < SORA_ROWS + SORA_ROW_COUNT * ROW_SIZE {
            return bad(format!("btltbl: {} bytes is too short", data.len()));
        }
        Ok(Btltbl { data })
    }

    fn b(&self) -> Bytes<'_> {
        Bytes(&self.data)
    }

    /// The damage curve at file 0x00.
    pub fn curve(&self) -> Result<DamageCurve> {
        let b = self.b();
        let n = b.u16(CURVE)? as usize;
        let values = (0..n).map(|i| b.u16(CURVE + 2 + 2 * i)).collect::<Result<_>>()?;
        Ok(DamageCurve { values })
    }

    /// The globals block at file 0x58.
    pub fn globals(&self) -> Result<Globals> {
        let b = self.b();
        let g = GLOBALS;
        let f = |o: usize| b.f32(g + o);
        let mut reaction_pushes = [0.0; 7];
        for (i, v) in reaction_pushes.iter_mut().enumerate() {
            *v = f(0x1C + 4 * i)?;
        }
        let mut jump_heights = [(0, 0); 10];
        for (c, v) in jump_heights.iter_mut().enumerate() {
            *v = (b.u16(g + 0x54 + 4 * c)?, b.u16(g + 0x56 + 4 * c)?);
        }
        let mut member_values = [0; 5];
        for (n, v) in member_values.iter_mut().enumerate() {
            *v = b.u16(g + 0xF4 + 2 * n)?;
        }
        Ok(Globals {
            narrow_cone_deg: b.u32(g)?,
            wide_cone_deg: b.u32(g + 4)?,
            close_range: b.u32(g + 8)?,
            lunge_range: b.u32(g + 0xC)?,
            party_flinch_pct: b.u8(g + 0x10)?,
            party_knock_pct: b.u8(g + 0x11)?,
            launch_speed_min: b.u16(g + 0x14)?,
            launch_speed_max: b.u16(g + 0x16)?,
            launch_height_factor: f(0x18)?,
            reaction_pushes,
            air_knockback: [f(0x38)?, f(0x3C)?],
            jump_heights,
            exp_scale: [f(0x88)?, f(0x8C)?],
            timer_step: f(0x90)?,
            mp_charge_level_pct: b.u8(g + 0x94)?,
            mp_charge_ability_pct: b.u8(g + 0x95)?,
            mp_charge_cap_pct: b.u16(g + 0x96)?,
            air_command_hop: f(0xE8)?,
            air_cast_velocity_scale: f(0xF0)?,
            member_values,
            strength_ability_bonus: b.u8(g + 0xFE)?,
            raw: b.slice(g, GLOBALS_SIZE)?.to_vec(),
        })
    }

    /// The action row at a file offset.
    pub fn row_at(&self, at: usize) -> Result<ActionRow> {
        let b = self.b();
        Ok(ActionRow {
            motion: b.i16(at)?,
            trail: b.i8(at + 2)?,
            weapon_effect: b.i8(at + 3)?,
            attack: b.u32(at + 4)?,
            hop: b.f32(at + 8)?,
            hit_sound: b.u8(at + 0xC)?,
            wall_sound: b.u8(at + 0xD)?,
            crit_sound: b.u8(at + 0xE)?,
            voice: b.i8(at + 0xF)?,
            tail: b.u32(at + 0x10)?,
        })
    }

    /// Sora's action row `r`.
    pub fn row(&self, r: usize) -> Result<ActionRow> {
        self.row_at(SORA_ROWS + r * ROW_SIZE)
    }

    /// All of Sora's rows (0x00..=0x41).
    pub fn sora_rows(&self) -> Result<Vec<ActionRow>> {
        (0..SORA_ROW_COUNT).map(|r| self.row(r)).collect()
    }

    /// The attack record at a file offset (a row's [`ActionRow::attack`]).
    pub fn attack(&self, at: usize) -> Result<AttackData> {
        let b = self.b();
        b.slice(at, ATTACK_SIZE)?;
        let i = |o: usize| b.i32(at + o);
        let f = |o: usize| b.f32(at + o);
        Ok(AttackData {
            offset: at,
            stat_source: i(0x00)?,
            element: i(0x04)?,
            kind_party: i(0x08)?,
            kind_enemy: i(0x0C)?,
            bone: i(0x10)?,
            power_party: i(0x14)?,
            crit_chance: i(0x18)?,
            crit_bonus: i(0x1C)?,
            min_party: i(0x20)?,
            max_party: i(0x24)?,
            effect_bit: i(0x28)?,
            mp_charge: i(0x2C)?,
            mp_cost: i(0x30)?,
            flags_34: b.u32(at + 0x34)?,
            clash: b.u32(at + 0x38)?,
            priority: i(0x3C)?,
            clash_motion: i(0x40)?,
            object_damage: i(0x44)?,
            power: i(0x48)?,
            min: i(0x4C)?,
            max: i(0x50)?,
            frac_a: f(0x54)?,
            frac_b: f(0x58)?,
            radius: f(0x5C)?,
            arc_turn: f(0x60)?,
            arc_cos: f(0x64)?,
            f68: f(0x68)?,
            id_6c: b.u32(at + 0x6C)?,
        })
    }

    /// The attack record a row points at.
    pub fn row_attack(&self, row: &ActionRow) -> Result<AttackData> {
        self.attack(row.attack as usize)
    }

    /// The item row of item `id` (1-based).
    pub fn item(&self, id: u16) -> Result<Item> {
        if id == 0 {
            return bad("btltbl: item id 0");
        }
        let at = ITEMS + (id as usize - 1) * ITEM_SIZE;
        let b = self.b();
        Ok(Item { id, kind: b.u8(at + 2)?, weapon_row: b.i16(at + 6)? })
    }

    /// Weapon row `row` (1-based, as an item's [`Item::weapon_row`]).
    pub fn weapon_row(&self, row: usize) -> Result<Weapon> {
        if row == 0 {
            return bad("btltbl: weapon row 0");
        }
        let at = WEAPONS + (row - 1) * WEAPON_SIZE;
        let b = self.b();
        Ok(Weapon {
            offset: at,
            model: b.cstr(at, 0x20)?,
            radius: b.f32(at + 0x20)?,
            blade_far: b.f32(at + 0x24)?,
            blade_near: b.f32(at + 0x28)?,
            frac_scale: b.f32(at + 0x2C)?,
            strength: b.i8(at + 0x30)?,
            crit: b.u8(at + 0x31)?,
            crit_bonus: b.i8(at + 0x32)?,
            priority: b.u8(at + 0x33)?,
            sound_base: b.u32(at + 0x34)?,
            magic: b.i8(at + 0x38)?,
        })
    }

    /// The weapon of item `id` (e.g. 0x51, the Kingdom Key).
    pub fn weapon_for_item(&self, id: u16) -> Result<Weapon> {
        let it = self.item(id)?;
        if it.weapon_row <= 0 {
            return bad(format!("btltbl: item {id:#x} has no weapon row"));
        }
        self.weapon_row(it.weapon_row as usize)
    }
}

/// Kingdom Key's item id.
pub const KINGDOM_KEY: u16 = 0x51;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mset::{track, MotionSet};
    use crate::testdata;

    fn file() -> Option<Btltbl> {
        Some(Btltbl::parse(testdata::read("btltbl.bin")?).unwrap())
    }

    #[test]
    fn short_file_is_refused() {
        assert!(Btltbl::parse(vec![0; 100]).is_err());
    }

    #[test]
    fn curve_and_globals() {
        let Some(t) = file() else { return };
        let c = t.curve().unwrap();
        assert_eq!(c.values.len(), 42);
        assert_eq!(&c.values[..10], &[1, 1, 2, 3, 4, 5, 6, 7, 8, 10]);
        assert_eq!(c.values[29], 300);
        assert_eq!(c.at(-5), 1);
        assert_eq!(c.at(1000), 300);
        let g = t.globals().unwrap();
        assert_eq!((g.narrow_cone_deg, g.wide_cone_deg, g.close_range, g.lunge_range), (45, 240, 180, 300));
        assert_eq!((g.party_flinch_pct, g.party_knock_pct), (20, 30));
        assert_eq!((g.launch_speed_min, g.launch_speed_max), (20, 35));
        assert!((g.launch_height_factor - 1.35).abs() < 1e-6);
        assert_eq!(g.reaction_pushes, [2.5, 750.0, 150.0, 4.0, 80.0, 40.0, 8.0]);
        assert_eq!(g.air_knockback, [4.0, 16.0]);
        assert_eq!((g.sora_jump(), g.sora_high_jump()), (190, 290));
        assert_eq!(g.jump_heights[1], (250, 330));
        assert_eq!((g.mp_charge_level_pct, g.mp_charge_ability_pct, g.mp_charge_cap_pct), (20, 150, 250));
        assert_eq!((g.air_command_hop, g.air_cast_velocity_scale, g.timer_step), (30.0, 0.5, 8.0));
        assert_eq!(g.member_values, [200, 300, 200, 300, 300]);
        assert_eq!(g.strength_ability_bonus, 4);
    }

    #[test]
    fn rows_and_attacks() {
        let Some(t) = file() else { return };
        let r13 = t.row(rows::GROUND_1).unwrap();
        assert_eq!((r13.motion, r13.trail, r13.weapon_effect, r13.attack, r13.hop), (200, 5, -1, 0x4B78, 0.0));
        assert_eq!((r13.hit_sound, r13.wall_sound, r13.crit_sound, r13.voice, r13.tail), (0, 5, 6, 4, 0));
        let table: [(usize, i16, u32, f32); 12] = [
            (11, 201, 0x4BE8, 0.0),
            (14, 202, 0x4B78, 0.0),
            (8, 203, 0x4C58, 0.0),
            (18, 204, 0x4B78, 20.0),
            (17, 205, 0x4BE8, 20.0),
            (16, 206, 0x4C58, 40.0),
            (15, 209, 0x5048, 40.0),
            (2, 204, 0x4B78, 60.0),
            (0, 214, 0x4CC8, 0.0),
            (rows::GUARD, 212, 0x4DA8, 0.0),
            (rows::COUNTER, 213, 0x4E18, 0.0),
            (6, 210, 0x4D38, 0.0),
        ];
        for (r, m, a, hop) in table {
            let row = t.row(r).unwrap();
            assert_eq!((row.motion, row.attack, row.hop), (m, a, hop), "row {r}");
        }
        assert_eq!(t.row(rows::GUARD).unwrap().voice, 19);
        assert_eq!(t.sora_rows().unwrap().len(), SORA_ROW_COUNT);

        let a = t.row_attack(&r13).unwrap();
        assert_eq!((a.stat_source, a.element, a.kind_party, a.kind_enemy, a.bone), (0, 1, 3, 3, 0x30));
        assert_eq!((a.power, a.min, a.max, a.mp_charge, a.clash_motion, a.object_damage), (0, 0, 0, 3, 110, 1));
        assert_eq!((a.frac_a, a.frac_b, a.radius, a.arc_cos), (1.0, 0.0, 0.0, 1.0));
        assert_eq!((a.hit_slot(), a.spent(), a.clash_type()), (0, false, 0));
        let fin = t.attack(0x4C58).unwrap();
        assert_eq!((fin.kind_enemy, fin.power, fin.crit_chance, fin.crit_bonus, fin.mp_charge), (5, 4, 4, 4, 6));
        assert!(fin.spent());
        let guard = t.attack(0x4DA8).unwrap();
        assert_eq!((guard.kind_enemy, guard.clash_type(), guard.mp_charge), (0, 2, 0));
        assert!((guard.arc_cos - (100f32).to_radians().cos()).abs() < 1e-6);
        let ripple = t.attack(0x4E88).unwrap();
        assert_eq!((ripple.stat_source, ripple.radius, ripple.clash), (4, 200.0, 0x10006));
        assert_eq!(t.attack(0x4E18).unwrap().mp_charge, 24);
        assert!((t.attack(0x4FD8).unwrap().frac_b + 0.2).abs() < 1e-6);
    }

    #[test]
    fn weapons() {
        let Some(t) = file() else { return };
        let kk = t.weapon_for_item(KINGDOM_KEY).unwrap();
        assert_eq!(kk.offset, WEAPONS);
        assert_eq!(kk.model, "xw_ex_5010");
        assert_eq!((kk.radius, kk.blade_near, kk.strength, kk.crit, kk.crit_bonus, kk.priority, kk.sound_base, kk.magic), (40.0, 0.0, 3, 20, 0, 30, 0x2B10, 0));
        assert!((kk.blade_far + 3.7).abs() < 1e-6 && (kk.frac_scale - 0.8).abs() < 1e-6);
        assert_eq!(t.item(KINGDOM_KEY).unwrap().kind, 2);
        let ultima = t.weapon_for_item(0x64).unwrap();
        assert_eq!((ultima.radius, ultima.strength, ultima.crit, ultima.crit_bonus, ultima.priority, ultima.magic), (50.0, 14, 40, 2, 90, 2));
        for id in 0x51..=0x66u16 {
            let w = t.weapon_for_item(id).unwrap();
            assert!((35.0..=60.0).contains(&w.radius) && w.blade_near == 0.0, "item {id:#x}");
        }
    }

    /// Rows joined to the motion set (the combo table in `kh1-data-values.md` §0).
    #[test]
    fn combo_table() {
        let Some(t) = file() else { return };
        let Some(m) = testdata::read("xa_ex_0010.mset") else { return };
        let ms = MotionSet::parse(&m).unwrap();
        // (row, frames, hit, combo window, root travel)
        type Want = (usize, u32, (f32, f32), Option<(f32, f32)>, f32);
        let want: [Want; 7] = [
            (13, 48, (14.0, 24.0), Some((22.0, 40.0)), 100.0),
            (11, 48, (18.0, 32.0), Some((26.0, 42.0)), 120.0),
            (14, 64, (18.0, 30.0), Some((26.0, 54.0)), 134.94),
            (8, 80, (26.0, 38.0), None, 190.0),
            (18, 40, (6.0, 20.0), Some((18.0, 34.0)), 0.0),
            (17, 42, (10.0, 20.0), Some((18.0, 32.0)), 0.0),
            (16, 56, (12.0, 38.0), None, 0.0),
        ];
        for (r, frames, hit, combo, travel) in want {
            let row = t.row(r).unwrap();
            let (map, mo) = ms.motion_for(row.motion as u16).unwrap();
            let slot = t.row_attack(&row).unwrap().hit_slot() as u16;
            assert_eq!(mo.frames, frames, "row {r}");
            assert_eq!(mo.intervals(track::HIT, slot), vec![hit], "row {r}");
            assert_eq!(mo.intervals(track::COMBO, 0), combo.into_iter().collect::<Vec<_>>(), "row {r}");
            assert!(mo.intervals(track::FREE, 0).is_empty(), "row {r}");
            let tr = if map.root_motion() { mo.root_travel(0.0, frames as f32)[2] } else { 0.0 };
            assert!((tr - travel).abs() < 0.01, "row {r}: {tr}");
        }
    }
}
