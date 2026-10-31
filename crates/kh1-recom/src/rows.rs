//! Which Re:CoM action row (Sora's action table `0x1407C5E80`, played by `0x1401F7180`) plays a
//! KH1 motion request, by the owner's rule (decision 11, 2026-10-30): where Re:CoM has the same
//! kind of move, Re:CoM's own animation; KH1's converted clips (the asset step's new `PL0001.ANN`
//! slots) only for moves Re:CoM lacks. `docs/recom-kh1-movement.md` § Motion rows.
//!
//! Re:CoM's rows and clips (Checked (exe): the table's names): 1 `p001_001` field idle, 2 / 0x12
//! `p001_002` walk, 3 `p001_003` / 0x13 `p001_023` run, 4 `p001_004` jump (KH1 entry 8's bake),
//! 5 `p001_005` jump top, 6 `p001_006` fall, 7 `p001_007` land (entry 11's bake), 0x16 `p001_070`
//! Dodge Roll (entry 117's bake), 0x1B–0x1E `p001_154`–`p001_157` glide, glide fast, glide turns
//! (entries 75–78), 0x2A / 0x2B / 0x2C `p001_031` / `p001_020` / `p001_024` battle-stance idle,
//! walk, run. Re:CoM's own picker `0x1401F9380` chooses among the idle / walk / run rows; [`row_for`]
//! does the same.
//!
//! High Jump (decision 17, 0.6.0): Re:CoM's own higher-jump rows 0x14 / 0x15 (`p001_028` /
//! `p001_029`, played by Re:CoM's jump init when `player+0x41C & 0x200000`, and for its springs)
//! play KH1's High Jump take-off (request 9) and top (request 0xA). Its fall and land share the
//! normal jump's rows (KH1 entries 10 / 11).
//!
//! KH1-only (new rows appended to the table, each naming a converted clip from the manifest):
//! Guard (entry 110, request 212) and Counterattack (entry 111, request 213). The guard-success
//! recoil (request 0x6E, entry 72) is Re:CoM's own `p001_081` (row 0x24, its bake).

use kh1_core::motion::request;

/// Rows in Re:CoM's table before its null terminator. Checked (exe).
pub const RECOM_ROWS: u32 = 0x47;

pub mod row {
    pub const IDLE: u32 = 1;
    pub const WALK: u32 = 2;
    pub const RUN: u32 = 3;
    pub const JUMP: u32 = 4;
    pub const JUMP_TOP: u32 = 5;
    pub const FALL: u32 = 6;
    pub const LAND: u32 = 7;
    pub const WALK_B: u32 = 0x12;
    pub const RUN_B: u32 = 0x13;
    pub const ROLL: u32 = 0x16;
    pub const GLIDE: u32 = 0x1B;
    pub const GLIDE_FAST: u32 = 0x1C;
    pub const GLIDE_TURN_A: u32 = 0x1D;
    pub const GLIDE_TURN_B: u32 = 0x1E;
    pub const STANCE_IDLE: u32 = 0x2A;
    pub const STANCE_WALK: u32 = 0x2B;
    pub const STANCE_RUN: u32 = 0x2C;
    /// Re:CoM's higher jump take-off and top (`p001_028` / `p001_029`).
    pub const HIGH_JUMP: u32 = 0x14;
    pub const HIGH_JUMP_TOP: u32 = 0x15;
    /// Re:CoM's `p001_081` (KH1 entry 72's bake, Checked (files): the recoil 0x6E).
    pub const RECOIL: u32 = 0x24;
}

/// KH1 requests for the KH1-only rows, in the order they are appended.
pub const KH1_ONLY: [(u16, u32); 2] = [(212, 110), (213, 111)];

/// KH1 Guard's request.
pub const GUARD: u16 = 212;
/// KH1 Counterattack's request (row 0x3E).
pub const COUNTERATTACK: u16 = 213;
/// KH1's guard-success recoil (kind 6, `0x14029D720`).
pub const RECOIL: u16 = 0x6E;

/// The appended rows: `ext[i]` is the row id of `KH1_ONLY[i]`, when its clip exists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Extension {
    pub rows: [Option<u32>; 2],
}

impl Extension {
    /// Row ids for the KH1 entries the pack holds, in `KH1_ONLY` order, from `RECOM_ROWS` on.
    pub fn plan(have: impl Fn(u32) -> bool) -> Extension {
        let mut e = Extension::default();
        let mut next = RECOM_ROWS;
        for (i, &(_, entry)) in KH1_ONLY.iter().enumerate() {
            if have(entry) {
                e.rows[i] = Some(next);
                next += 1;
            }
        }
        e
    }

    pub fn get(&self, req: u16) -> Option<u32> {
        KH1_ONLY.iter().position(|&(r, _)| r == req).and_then(|i| self.rows[i])
    }

    pub fn count(&self) -> usize {
        self.rows.iter().flatten().count()
    }

    /// Whether `row` is one of our appended KH1-only rows (Guard, Counterattack).
    pub fn has_row(&self, row: i32) -> bool {
        self.rows.iter().flatten().any(|&r| r as i32 == row)
    }
}

/// What the row picker reads from the player.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stance {
    /// `player+0x428 & 1`: battle stance.
    pub battle: bool,
    /// `player+0x418 & 2`: picks rows 2 / 3 over 0x12 / 0x13 (`0x1401F9380`).
    pub flag_418_2: bool,
    /// `player+0x470` / `+0x474`: the previous and current row (`0x1401F7180` keeps them).
    pub prev_row: i32,
    pub row: i32,
}

impl Stance {
    /// `0x1401E6AC0`: the idle row is the battle-stance one in battle stance, or while the
    /// previous or current row is a swing / stance row. Checked (decomp).
    pub fn idle_battle(&self) -> bool {
        let swingish = |r: i32| (0x24..=0x25).contains(&r) || (0x30..=0x33).contains(&r);
        self.battle || swingish(self.prev_row) || swingish(self.row) || self.row == 0x2A
    }
}

/// The Re:CoM row for a KH1 request, and whether it is played with root motion. `None`: no row
/// (a KH1-only clip the pack lacks): the caller keeps the current motion.
pub fn row_for(req: u16, s: &Stance, ext: &Extension) -> Option<(u32, bool)> {
    let r = match req {
        request::IDLE => {
            // Our KH1-only rows (Guard, Counterattack) end in the battle stance like Re:CoM's swing
            // rows, so `0x1401E6AC0`'s rule counts them as swing rows: the stance idle 0x2A after
            // them, as after a swing (owner, 2026-10-31: the field Guard snapped to the field idle).
            if s.idle_battle() || ext.has_row(s.row) || ext.has_row(s.prev_row) {
                row::STANCE_IDLE
            } else {
                row::IDLE
            }
        }
        // `0x1401F9380`: walk 2 (or 0x12), run 3 (or 0x13), 0x2B / 0x2C in battle stance.
        request::WALK => {
            if s.battle {
                row::STANCE_WALK
            } else if s.flag_418_2 {
                row::WALK
            } else {
                row::WALK_B
            }
        }
        request::RUN => {
            if s.battle {
                row::STANCE_RUN
            } else if s.flag_418_2 {
                row::RUN
            } else {
                row::RUN_B
            }
        }
        request::JUMP => row::JUMP,
        request::APEX => row::JUMP_TOP,
        // FALL and the High Jump's fall share KH1 entry 10.
        request::FALL | 0x0B => row::FALL,
        // LAND and the High Jump's land share KH1 entry 11.
        request::LAND | request::HIGH_LAND => row::LAND,
        // Decision 17: Re:CoM's own higher jump.
        request::HIGH_JUMP => row::HIGH_JUMP,
        0x0A => row::HIGH_JUMP_TOP,
        request::GLIDE => row::GLIDE,
        request::GLIDE_FAST => row::GLIDE_FAST,
        request::GLIDE_TURN_A => row::GLIDE_TURN_A,
        request::GLIDE_TURN_B => row::GLIDE_TURN_B,
        request::DODGE_ROLL => return Some((row::ROLL, true)),
        // KH1's Guard motion has the root bit (no travel).
        GUARD => return ext.get(req).map(|r| (r, true)),
        // KH1's recoil has no travel; Re:CoM's bake of it is row 0x24.
        RECOIL => row::RECOIL,
        COUNTERATTACK => return ext.get(req).map(|r| (r, true)),
        _ => return None,
    };
    Some((r, false))
}

/// The table rows to append after Re:CoM's 0x47: `(row id, clip name)`, from the manifest's KH1
/// entries (`motions::clip_for_entry`).
pub fn appended_rows(ext: &Extension, name_of_entry: impl Fn(u32) -> Option<String>) -> Vec<(u32, String)> {
    KH1_ONLY
        .iter()
        .zip(ext.rows.iter())
        .filter_map(|(&(_, entry), r)| Some((r.as_ref().copied()?, name_of_entry(entry)?)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locomotion_rows_follow_recoms_picker() {
        let field = Stance { flag_418_2: true, ..Default::default() };
        let ext = Extension::default();
        assert_eq!(row_for(0, &field, &ext), Some((row::IDLE, false)));
        // After the Guard or Counterattack row (ours), the stance idle, as after a swing.
        let both = Extension::plan(|_| true);
        let guard = both.get(GUARD).expect("guard row");
        let after_guard = Stance { row: guard as i32, ..field };
        assert_eq!(row_for(0, &after_guard, &both), Some((row::STANCE_IDLE, false)));
        let after_guard_prev = Stance { prev_row: guard as i32, row: row::IDLE as i32, ..field };
        assert_eq!(row_for(0, &after_guard_prev, &both), Some((row::STANCE_IDLE, false)));
        let counter = both.get(COUNTERATTACK).expect("counterattack row");
        assert_eq!(row_for(0, &Stance { row: counter as i32, ..field }, &both), Some((row::STANCE_IDLE, false)));
        assert_eq!(row_for(0, &field, &both), Some((row::IDLE, false)), "a plain field idle stays Re:CoM's");
        assert_eq!(row_for(1, &field, &ext), Some((row::WALK, false)));
        assert_eq!(row_for(2, &field, &ext), Some((row::RUN, false)));
        let b = Stance { flag_418_2: false, ..Default::default() };
        assert_eq!(row_for(1, &b, &ext).unwrap().0, row::WALK_B);
        assert_eq!(row_for(2, &b, &ext).unwrap().0, row::RUN_B);
        let battle = Stance { battle: true, ..Default::default() };
        assert_eq!(row_for(0, &battle, &ext).unwrap().0, row::STANCE_IDLE);
        assert_eq!(row_for(1, &battle, &ext).unwrap().0, row::STANCE_WALK);
        assert_eq!(row_for(2, &battle, &ext).unwrap().0, row::STANCE_RUN);
        // Right after a swing row the idle is the stance idle (0x1401E6AC0).
        let after_swing = Stance { row: 0x31, ..Default::default() };
        assert_eq!(row_for(0, &after_swing, &ext).unwrap().0, row::STANCE_IDLE);
    }

    #[test]
    fn jump_glide_roll_rows() {
        let s = Stance::default();
        let none = Extension::default();
        assert_eq!(row_for(4, &s, &none), Some((row::JUMP, false)));
        assert_eq!(row_for(5, &s, &none), Some((row::JUMP_TOP, false)));
        assert_eq!(row_for(6, &s, &none), Some((row::FALL, false)));
        assert_eq!(row_for(0xB, &s, &none), Some((row::FALL, false)));
        assert_eq!(row_for(7, &s, &none), Some((row::LAND, false)));
        assert_eq!(row_for(0xC, &s, &none), Some((row::LAND, false)));
        assert_eq!(row_for(0x71, &s, &none), Some((0x1B, false)));
        assert_eq!(row_for(0x74, &s, &none), Some((0x1E, false)));
        assert_eq!(row_for(0xDC, &s, &none), Some((row::ROLL, true)));
        // High Jump plays Re:CoM's own higher jump (decision 17); Guard needs its converted clip.
        assert_eq!(row_for(9, &s, &none), Some((row::HIGH_JUMP, false)));
        assert_eq!(row_for(0xA, &s, &none), Some((row::HIGH_JUMP_TOP, false)));
        assert_eq!(row_for(GUARD, &s, &none), None);
        assert_eq!(row_for(200, &s, &none), None);
    }

    #[test]
    fn extension_rows_are_appended_in_order() {
        let all = Extension::plan(|_| true);
        assert_eq!(all.rows, [Some(0x47), Some(0x48)]);
        let s = Stance::default();
        // The converted High Jump clips are no longer used, even when the pack has them.
        assert_eq!(row_for(9, &s, &all), Some((row::HIGH_JUMP, false)));
        assert_eq!(row_for(GUARD, &s, &all), Some((0x47, true)));
        assert_eq!(row_for(COUNTERATTACK, &s, &all), Some((0x48, true)));
        assert_eq!(row_for(RECOIL, &s, &all), Some((row::RECOIL, false)));
        let no_guard = Extension::plan(|e| e != 110);
        assert_eq!(no_guard.rows, [None, Some(0x47)], "the counterattack moves up when the guard clip is missing");
        let rows = appended_rows(&all, |e| Some(format!("p001_{}", 150 + e)));
        assert_eq!(rows, vec![(0x47, "p001_260".to_string()), (0x48, "p001_261".to_string())]);
        assert!(appended_rows(&all, |_| None).is_empty());
    }
}
