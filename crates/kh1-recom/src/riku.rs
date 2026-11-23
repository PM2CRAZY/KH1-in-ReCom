//! Riku in Reverse/Rebirth with KH1's movement (testing, fork branch `Re-Re-Reverse/Rebirth`;
//! ini `riku_kh1_movement`, default off). `docs/recom-riku-kh1.md`.
//!
//! What lets KH1's movement run on Riku natively:
//! - Re:CoM's per-character action tables (`0x1407C6300[player+0x398]`, 16-byte rows
//!   `{name, index}`): characters 3 / 4 (Riku) point at `0x1407C6370`, whose rows 0x00..0x38 mean
//!   what Sora's (`0x1407C5E80`) mean: idle, walk, run, jump, fall, land, roll, hit reactions,
//!   swings. Checked (exe: both tables read row by row).
//! - Riku's battle exe (`CPlayerRiku` vtable `0x1405A8868` `+0xB8` = `0x140216B10`) is
//!   `jmp 0x1401F9840`, Re:CoM's shared Battle exe; his Field exe is the shared `0x1401FA0F0`.
//!   So his actions run through the same switch and the same move / jump handlers as Sora's.
//!   Checked (exe: vtables and disasm).
//! - His own handlers: the roll exe (`+0x1C8` = `0x140212350`, Sora's is `0x140217310`); no glide
//!   (`+0x1A8` / `+0x1B0` are `CPlayer`'s defaults). Checked (exe).
//! - His clips (`PLAYER/0101/PL0101.ANN`, 51) sit on KH1's Riku skeleton: `PL0101.MDL` bone 1 is
//!   `ex_1010_sk` (KH1 `xa_ex_1010`), and 9 clips are exact bakes of KH1 Riku motions (all hit
//!   reactions, idle-turns). Checked (files).

/// Riku's action-table rows with no clip of his: their name is `p002_000`, which his pack
/// (`PL0101.ANN`, first slot `p002_001`) does not hold. Rows 0x1B..0x1E are Sora's glide rows,
/// 0x39..0x46 Sora's casts, item, and KH1-move bakes. Checked (exe: `0x1407C6370`; files).
pub const NO_CLIP: &[u32] = &[0x00, 0x1B, 0x1C, 0x1D, 0x1E];

/// The first row past Riku's own: 0x39..0x46 are `p002_000`, and our appended KH1 rows (0x47..)
/// are only in Sora's repointed table, not Riku's.
pub const ROW_END: u32 = 0x39;

/// Whether Riku's action table plays a clip of his on `row`. Anything else would start a motion
/// with no clip (a T-pose), so the KH1 frame keeps the current motion instead.
pub fn row_ok(row: u32) -> bool {
    row < ROW_END && !NO_CLIP.contains(&row)
}

/// Dark Riku's action-table rows with no clip (`0x1407C67F0`: `p003_000`; his pack `PL0201.ANN`
/// starts at `p003_001`): 0, 8..=0xD, 0x17..=0x1A, 0x1C..=0x1E, 0x39.. Row 0x1B (Sora's glide row)
/// has a clip of his, never played (no glide). Checked (exe).
pub const DARK_NO_CLIP: &[u32] = &[0x00, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E];

/// [`row_ok`] for Riku (`dark` false) or Dark Riku (0.152.0).
pub fn row_ok_for(dark: bool, row: u32) -> bool {
    if dark {
        row < ROW_END && !DARK_NO_CLIP.contains(&row)
    } else {
        row_ok(row)
    }
}

/// Riku's ability word bit for Re:CoM's higher jump (`+0x41C & 0x200000`: the jump init
/// `0x1401F29F0` reads it; `crate::abilities::recom`).
pub const HIGH_JUMP_BIT: u32 = 0x20_0000;

/// The KH1 movement abilities Riku has: Dodge Roll (always Re:CoM's own roll, his roll exe), and
/// High Jump when Re:CoM gave him its higher jump. No Glide (Riku has none in Re:CoM), no
/// Counterattack (no clip), no combo abilities (KH1's plain combo on his own swings).
///
/// Guard (0.152.0; fork owner, 2026-11-22: "falta a defesa"): KH1's Guard rule (Square below run
/// speed, the guard object live on KH1's frames 10..36, the kind-6 recoil) on his own block-recoil
/// clip, action row 0x24 ([`GUARD_ROW`]). KH1's Riku (`xa_ex_1010.mset`) has no Guard request (212):
/// his request 110 / 111 (entry 13) is the clash recoil Re:CoM baked as row 0x24 for him too.
/// Checked (files: both motion sets' request tables).
pub fn abilities(ability_word: u32) -> kh1_core::Abilities {
    kh1_core::Abilities { high_jump: ability_word & HIGH_JUMP_BIT != 0, dodge_roll: true, guard: true, ..Default::default() }
}

/// Re:CoM's body-guard angle rule (`0x14016F1C0`'s guard branch; Checked (exe: disasm)): the
/// attack point minus the guarder's bone 0 (`0x14023E6E0`), `atan2f(d.x, d.z)` minus the guarder's
/// yaw (its mover `+0x54`, `0x1401A4A00`), wrapped to ±π (`0x14016EAA0`), within `half` (KH1's
/// Guard: ±100°, `KH1_GUARD_ANGLE`). Used for Riku's Guard when Re:CoM's own block didn't fire
/// (0.153.0).
pub fn guard_faces(guarder: [f32; 3], yaw: f32, point: [f32; 3], half: f32) -> bool {
    let (dx, dz) = (point[0] - guarder[0], point[2] - guarder[2]);
    if dx == 0.0 && dz == 0.0 {
        return true;
    }
    let pi = std::f32::consts::PI;
    let mut a = dx.atan2(dz) - yaw;
    while a > pi {
        a -= 2.0 * pi;
    }
    while a < -pi {
        a += 2.0 * pi;
    }
    a.abs() <= half
}

/// Riku's / Dark Riku's row for KH1's Guard request: their recoil row (`p002_043` / `p003_033`).
pub const GUARD_ROW: u32 = 0x24;

/// Re:CoM's attack row id for Riku's own swing that plays the same clip as Sora's row `sora_id`.
/// Sora's rows 1..7 (`0x1407C6C70`, action rows 0x30..0x36) have Riku's twins at 0x97..0x9D: the same
/// action rows and card columns, but their hit effect (row `+2`, by id: `CPlayerWeapon::onAttack`
/// `0x1402250B0` reads `0x1407C6C72[weapon+0x23C]` into the effect table `0x1405A1480`) is Riku's
/// `weapon` set (12; 0 on the finishers 4 / 7, in his `EFFECT/1500/EF1500.EFF`) instead of Sora's
/// `ex_5010` (in `EFFECT/1000`, not loaded in Riku's story: no spark on his hits). Other ids as they
/// are. Checked (exe: both row sets and the effect table; files: the two containers).
pub fn attack_id(sora_id: u32) -> u32 {
    if (1..=7).contains(&sora_id) {
        RIKU_ROW_1 - 1 + sora_id
    } else {
        sora_id
    }
}

/// Riku's twin of Sora's attack row 1 (`0x1407C6C70` row 0x97). Checked (exe).
pub const RIKU_ROW_1: u32 = 0x97;
/// Dark Riku's (rows 0x9E..=0xA4: action rows 0x30..=0x36, his effects 0x15 / 0x16). Checked (exe).
pub const DARK_ROW_1: u32 = 0x9E;

/// [`attack_id`] for Riku or Dark Riku.
pub fn attack_id_for(dark: bool, sora_id: u32) -> u32 {
    if dark && (1..=7).contains(&sora_id) {
        DARK_ROW_1 - 1 + sora_id
    } else {
        attack_id(sora_id)
    }
}

/// One of Riku's sleights in his command menu's Sleights list (fork owner, 2026-11-22: option (b),
/// "a list of his sleights paid with KH1 MP, and shortcuts").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sleight {
    /// Re:CoM's sleight id (`player+0x82C`).
    pub id: u16,
    /// `SY0002.CTD`'s name (its row id + 1; Checked (files)).
    pub name: &'static str,
    /// Whole MP points (ours: KH1 has no Riku; the Sleights list's tiers, the Dark ones a point more).
    pub mp: i32,
}

/// The sleights Riku's own sleight start takes alone (`CPlayerRiku` vtable `+0x130` = `0x140216AB0`:
/// ids 0x66..=0x6B start; 0x62..=0x64 are King Mickey's, his friend card; anything else refused).
/// Checked (exe: disasm). Dark Break / Dark Firaga / Dark Aura (0x5F..=0x61) are not among them.
pub const SLEIGHTS: [Sleight; 6] = [
    Sleight { id: 0x66, name: "Impulse", mp: 2 },
    Sleight { id: 0x67, name: "Maelstrom", mp: 2 },
    Sleight { id: 0x68, name: "Barrage", mp: 3 },
    Sleight { id: 0x69, name: "Dark Impulse", mp: 3 },
    Sleight { id: 0x6A, name: "Dark Maelstrom", mp: 3 },
    Sleight { id: 0x6B, name: "Dark Barrage", mp: 4 },
];

/// Dark Riku's Sleights list (0.152.0): his own sleight start (`CPlayerDRiku +0x130` =
/// `0x140206830`) takes 0x5F..=0x63 and 0x65..=0x6B (its jump table at `0x140206894`; 0x64 refused).
/// Checked (exe). The Dark ones, with Dark Break / Dark Firaga / Dark Aura (`SY0002.CTD`'s names);
/// MP ours, as Riku's tiers.
pub const DARK_SLEIGHTS: [Sleight; 6] = [
    Sleight { id: 0x5F, name: "Dark Break", mp: 3 },
    Sleight { id: 0x60, name: "Dark Firaga", mp: 3 },
    Sleight { id: 0x61, name: "Dark Aura", mp: 4 },
    Sleight { id: 0x69, name: "Dark Impulse", mp: 3 },
    Sleight { id: 0x6A, name: "Dark Maelstrom", mp: 3 },
    Sleight { id: 0x6B, name: "Dark Barrage", mp: 4 },
];

/// Riku's or Dark Riku's sleight `id`.
pub fn sleight(id: u16) -> Option<&'static Sleight> {
    SLEIGHTS.iter().chain(DARK_SLEIGHTS.iter()).find(|s| s.id == id)
}

/// The list of one of them.
pub fn list(dark: bool) -> &'static [Sleight; 6] {
    if dark {
        &DARK_SLEIGHTS
    } else {
        &SLEIGHTS
    }
}

/// Their words in the command menu (`crate::sleights::WORD_BASE + id`), each id once.
pub fn words() -> impl Iterator<Item = (u16, &'static str)> {
    SLEIGHTS.iter().chain(DARK_SLEIGHTS.iter().filter(|d| !SLEIGHTS.iter().any(|s| s.id == d.id))).map(|s| (crate::sleights::WORD_BASE + s.id, s.name))
}

/// The shortcut slots Riku gets when all three are empty (and were never set up: the caller's
/// test): his first learned sleights. Slots already set are kept; his Customize page (0.149.0) sets
/// them after.
pub fn fill_shortcuts(slots: [u8; 3], learned: &[u16]) -> Option<[u8; 3]> {
    if slots != [0; 3] || learned.is_empty() {
        return None;
    }
    let mut out = [0u8; 3];
    for (k, &id) in learned.iter().take(3).enumerate() {
        out[k] = id as u8;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rows::row;

    #[test]
    fn rikus_rows_cover_kh1_movement() {
        for r in [row::IDLE, row::WALK, row::RUN, row::JUMP, row::JUMP_TOP, row::FALL, row::LAND, row::WALK_B, row::RUN_B, row::ROLL, row::STANCE_IDLE, row::STANCE_WALK, row::STANCE_RUN, row::HIGH_JUMP, row::HIGH_JUMP_TOP] {
            assert!(row_ok(r), "row {r:#x}");
        }
        for r in [row::GLIDE, row::GLIDE_FAST, row::GLIDE_TURN_A, row::GLIDE_TURN_B, row::THUNDER, row::BLIZZARD, row::FIRE_AIR_WINDUP, 0x46, 0x47, 0x60] {
            assert!(!row_ok(r), "row {r:#x}");
        }
    }

    #[test]
    fn rikus_attack_rows() {
        assert_eq!((1..=7).map(attack_id).collect::<Vec<_>>(), (0x97..=0x9D).collect::<Vec<_>>());
        assert_eq!(attack_id(0), 0);
        assert_eq!(attack_id(0x44), 0x44);
    }

    #[test]
    fn rikus_sleights() {
        assert!(SLEIGHTS.iter().all(|s| (0x66..=0x6B).contains(&s.id)));
        assert!(DARK_SLEIGHTS.iter().all(|s| (0x5F..=0x63).contains(&s.id) || (0x65..=0x6B).contains(&s.id)));
        assert_eq!(words().count(), 9);
        assert_eq!(attack_id_for(true, 4), 0xA1);
        assert_eq!(attack_id_for(false, 4), 0x9A);
        assert!(row_ok_for(true, 0x30) && !row_ok_for(true, 0x08) && row_ok_for(false, 0x08));
        assert_eq!(sleight(0x67).map(|s| s.name), Some("Maelstrom"));
        assert_eq!(sleight(0x5F).map(|s| s.name), Some("Dark Break"));
        assert!(sleight(0x64).is_none(), "Holy Burst is the King's");
        assert_eq!(fill_shortcuts([0; 3], &[0x66, 0x67]), Some([0x66, 0x67, 0]));
        assert_eq!(fill_shortcuts([0x66, 0, 0], &[0x67]), None);
        assert_eq!(fill_shortcuts([0; 3], &[]), None);
        // Their commands don't reach Stagger's 0xF0.
        assert!(SLEIGHTS.iter().all(|s| crate::sleights::command(s.id) < 0xF0));
        assert!(SLEIGHTS.iter().all(|s| crate::sleights::of_command(crate::sleights::command(s.id)) == Some(s.id)));
        assert_eq!(crate::sleights::pay_points(0x68, 3), Some(3));
        assert_eq!(crate::sleights::pay_points(0x68, 2), None);
        assert_eq!(crate::sleights::name(0x69), "Dark Impulse");
        assert_eq!(crate::sleights::list(&[0x67, 0x66]), vec![0x66, 0x67]);
        assert_eq!(crate::command_menu::shortcut::slot_sleight(0x66), Some(0x66));
    }

    #[test]
    fn rikus_guard_faces_the_front() {
        let half = 1.745_329_3;
        // Yaw 0 faces +z (atan2(x, z) = 0).
        assert!(guard_faces([0.0; 3], 0.0, [0.0, 0.0, 2.0], half));
        assert!(guard_faces([0.0; 3], 0.0, [2.0, 0.0, 0.1], half));
        assert!(!guard_faces([0.0; 3], 0.0, [0.0, 0.0, -2.0], half));
        assert!(!guard_faces([0.0; 3], 0.0, [0.5, 0.0, -2.0], half));
        // Facing −z (yaw π): behind is +z.
        assert!(guard_faces([1.0, 0.0, 1.0], std::f32::consts::PI, [1.0, 0.0, -3.0], half));
        assert!(!guard_faces([1.0, 0.0, 1.0], std::f32::consts::PI, [1.0, 0.0, 4.0], half));
    }

    #[test]
    fn rikus_abilities() {
        let a = abilities(0);
        assert!(a.dodge_roll && !a.high_jump && !a.glide && !a.superglide && a.guard && !a.counterattack && a.combo_plus == 0);
        assert!(abilities(HIGH_JUMP_BIT).high_jump);
    }
}
