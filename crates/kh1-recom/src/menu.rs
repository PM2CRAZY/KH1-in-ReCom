//! Our rows in Re:CoM's Config menu (camp menu → Config): the testable part.
//!
//! Re:CoM's Config screen is `CCamp`'s sub-screen run by `0x1400D3EB0` (input) and drawn by
//! `0x1400DF1B0` (mode 9 rebuilds the rows). It has 5 rows (cursor `camp+0xBE8`, 0..4): rows 0-3
//! are two-choice toggles kept in bytes `camp+0xBED + row`, row 4 shows a 4-value setting it
//! cannot change. Checked (decomp + disasm). We add row 5 below row 4, drawn by the same game
//! code (its `CButton` items, its layout positions, its cursor sprite and help line), and have
//! Re:CoM's own 9-slice code draw its window panel one row taller (`panel`).
//! See `docs/recom-config-menu.md`.

/// Re:CoM's last row.
pub const LAST_RECOM_ROW: i32 = 4;
/// Our first row.
pub const FIRST_ROW: i32 = 5;
/// How many rows we add.
pub const ROWS: usize = 1;
/// The last row the cursor can reach.
pub const LAST_ROW: i32 = FIRST_ROW + ROWS as i32 - 1;

/// Item slots in `camp+0xB38` (21 pointers, cleared and deleted by Re:CoM's own code in draw
/// modes 6 and 9 and in the camp's teardown `0x1400D7C20`). Re:CoM fills 0..13 (5 labels, 9
/// values); 14..20 are never written. Checked (decomp: every reference to `camp+0xB38..0xBE0`).
pub const SLOT_BASE: usize = 0xB38;
pub const SLOT_COUNT: usize = 21;
pub const FIRST_FREE_SLOT: usize = 14;

/// Item slots our row `k` uses: label, option 0, option 1.
pub fn slots(k: usize) -> [usize; 3] {
    let s = FIRST_FREE_SLOT + 3 * k;
    [s, s + 1, s + 2]
}

/// What a frame of input does with the cursor on the Config screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Re:CoM's own code handles this frame.
    Original,
    /// Move the cursor to this row (with Re:CoM's cursor sound and a redraw).
    Move(i32),
    /// Flip our row `k`.
    Toggle(usize),
    /// Our row has the cursor and nothing was pressed.
    Idle,
}

/// The input of one frame, as Re:CoM's own `0x140111A70` reports it (same masks as the
/// original: up `0x101000`, down `0x204000`, left `0x408000`, right `0x802000`). Confirm/cancel
/// is checked before this and always goes to Re:CoM's code (it closes the screen).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

/// The original checks up, then down, then left, then right (`0x1400D3F1E..0x1400D4040`); this
/// keeps that order. Re:CoM's rows keep their behaviour except at the two edges that now lead to
/// our rows: up on row 0 (wrapped to 4, now to our last row) and down on row 4 (wrapped to 0, now
/// to our first row).
pub fn step(cursor: i32, i: Input) -> Step {
    if cursor <= LAST_RECOM_ROW {
        if i.up {
            return if cursor == 0 { Step::Move(LAST_ROW) } else { Step::Original };
        }
        if i.down && cursor == LAST_RECOM_ROW {
            return Step::Move(FIRST_ROW);
        }
        return Step::Original;
    }
    if i.up {
        return Step::Move(cursor - 1);
    }
    if i.down {
        return Step::Move(if cursor >= LAST_ROW { 0 } else { cursor + 1 });
    }
    if i.left || i.right {
        return Step::Toggle((cursor - FIRST_ROW).clamp(0, ROWS as i32 - 1) as usize);
    }
    Step::Idle
}

/// Text of our rows. Re:CoM's own rows take theirs from its message table (`0x140240160`, ids
/// `0x5001xx`); ours are NUL-terminated ASCII handed to the same text items. That plain ASCII
/// renders as itself is an Assumption (the message formatter `0x1402402B0` treats `a`-`z` as
/// letters; not seen in game yet).
pub struct RowText {
    pub label: &'static [u8],
    pub options: [&'static [u8]; 2],
    pub help: &'static [u8],
}

pub const TEXT: [RowText; ROWS] = [
    RowText {
        label: b"Combat\0",
        options: [b"Re:CoM\0", b"KH1\0"],
        help: b"Re:CoM: card battles as in the original. KH1: Kingdom Hearts combat.\0",
    },
];

/// Layout elements (group `0xB` of `camp.lad`, read with `0x140246100`) Re:CoM's rows use.
/// Labels take x from the first and y from the second; values take x from the option's element
/// and y from the row's. Checked (disasm `0x1400DF5B7..0x1400DF878`).
pub mod layout {
    pub const GROUP: u32 = 0xB;
    /// (x element, y element) of the labels of rows 0..4.
    pub const LABEL: [(u64, u64); 5] = [(2, 2), (3, 3), (5, 4), (7, 5), (8, 6)];
    /// x elements of the 9 value items (rows 0-3 × 2, then row 4).
    pub const VALUE_X: [u64; 9] = [9, 10, 11, 12, 15, 16, 20, 21, 22];
    /// y elements of the value rows 0..4.
    pub const VALUE_Y: [u64; 5] = [9, 0xB, 0xD, 0xF, 0x11];
}

/// The y of our row `k`: one row step below the last, as many times as needed. `y3` and `y4`
/// are the y of Re:CoM's rows 3 and 4. Assumption: the rows are evenly spaced (they are in the
/// layout's elements as read at runtime).
pub fn row_y(y3: f32, y4: f32, k: usize) -> f32 {
    y4 + (y4 - y3) * (k as f32 + 1.0)
}

/// The value index a row shows as selected (Re:CoM's rows: byte == option).
pub fn selected(value: bool, option: usize) -> bool {
    (option == 1) == value
}

/// x offset Re:CoM adds to the cursor sprite on a value item: 0.0 for item 0, 150.0 for items
/// 1..8 (floats at `rbp+0x64`, from `0x1405094F0` / `0x140509500`). Checked (exe + disasm).
pub const CURSOR_X_OFFSET: f32 = 150.0;
/// Cursor animation for a toggle row (row 4 uses 0x100). Checked (disasm `0x1400DF93B`).
pub const CURSOR_ANIM: u32 = 0xFA;

/// The Config window panel (`camp+0xB30`, a `C2DSeqCtrl`), drawn taller by Re:CoM's own 9-slice
/// code. Checked (decomp + disasm), see `docs/recom-config-menu.md`:
///
/// - The draw `0x1400DF1B0` gives the panel animation `0xF7` (open) in mode 0, `0xF8` (idle)
///   in mode 1 once `0xF7` ends, and `0xF9` (close) in mode 4. Each change makes a new
///   `C2DSeqObj` with new `C2DSprObj` children (vtable `0x1406077D8`), found through the
///   panel's handle `+0x144` with `0x140246210` and the object's list `+0x20` (next at `+8`).
/// - A `C2DSprObj` with exactly 9 parts (`+0x142`) is a 9-slice: with flag 8 in `+0x144` its
///   update `0x14024BB10` calls `0x14024AD40`, which moves the parts' corners by the edge
///   offsets left `+0xA0`, top `+0xA4`, right `+0xA8`, bottom `+0xAC` (top row by top, middle
///   row stretched, bottom row by bottom). Re:CoM's own resize `0x14024C270` (vtable `+0x38`,
///   used by the message window `0x1402B3040`) sets the same fields, but centres the window on
///   its origin; we set only the bottom offset so the panel's top stays where it is.
pub mod panel {
    /// Draw modes after which the panel has a new animation (0 open, 1 opening tick that may
    /// switch to idle, 4 close). Mode 3 (every frame while open) and 9 (rows) don't touch it.
    pub fn restretch_after(mode: u32) -> bool {
        matches!(mode, 0 | 1 | 4)
    }

    /// How much lower the bottom edge goes: one row step per row we add. Units are the
    /// layout's, which are the panel's local units (its items are its children, positioned
    /// with raw layout values). Assumption: the panel's animation has scale 1 at rest.
    pub fn grow(y3: f32, y4: f32) -> f32 {
        (y4 - y3) * super::ROWS as f32
    }

    /// Parts of a 9-slice sprite (`C2DSprObj+0x142`).
    pub const NINE: u8 = 9;
    /// `C2DSprObj` fields.
    pub const PARTS: usize = 0x120;
    pub const PART_COUNT: usize = 0x142;
    pub const FLAGS: usize = 0x144;
    pub const FLAG_NINE_SLICE: u8 = 8;
    pub const OFFSETS: usize = 0xA0;
    /// `C2DSeqObj+0x20`: its sprite list; first object at list `+8`, next at object `+8`.
    pub const SEQ_LIST: usize = 0x20;
    /// `C2DSeqCtrl`: handles of its sequences at `+0x144`, count at `+0x16D`.
    pub const CTRL_HANDLES: usize = 0x144;
    pub const CTRL_COUNT: usize = 0x16D;

    /// What to do with one sprite of the panel's sequence.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum Sprite {
        /// Not a 9-slice, or not a `C2DSprObj`: leave it.
        Skip,
        /// Already has flag 8 (ours from an earlier call, or Re:CoM's own resize): leave it.
        Done,
        /// Write these edge offsets (left, top, right, bottom) and set flag 8.
        Stretch([f32; 4]),
    }

    pub fn plan(is_spr_obj: bool, parts: u8, has_parts: bool, flags: u8, grow: f32) -> Sprite {
        if !is_spr_obj || parts != NINE || !has_parts {
            Sprite::Skip
        } else if flags & FLAG_NINE_SLICE != 0 {
            Sprite::Done
        } else {
            Sprite::Stretch([0.0, 0.0, 0.0, grow])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: Input = Input { up: false, down: false, left: false, right: false };
    const UP: Input = Input { up: true, ..NONE };
    const DOWN: Input = Input { down: true, ..NONE };
    const LEFT: Input = Input { left: true, ..NONE };
    const RIGHT: Input = Input { right: true, ..NONE };

    #[test]
    fn recom_rows_keep_their_behaviour() {
        for c in 0..=4 {
            assert_eq!(step(c, NONE), Step::Original);
            assert_eq!(step(c, LEFT), Step::Original);
            assert_eq!(step(c, RIGHT), Step::Original);
        }
        for c in 1..=4 {
            assert_eq!(step(c, UP), Step::Original);
        }
        for c in 0..=3 {
            assert_eq!(step(c, DOWN), Step::Original);
        }
        // Up wins over down, as in the original.
        assert_eq!(step(4, Input { up: true, down: true, ..NONE }), Step::Original);
    }

    #[test]
    fn edges_lead_to_our_row() {
        assert_eq!(LAST_ROW, 5);
        assert_eq!(step(4, DOWN), Step::Move(5));
        assert_eq!(step(0, UP), Step::Move(5));
        assert_eq!(step(5, UP), Step::Move(4));
        assert_eq!(step(5, DOWN), Step::Move(0));
    }

    #[test]
    fn our_row_toggles() {
        assert_eq!(step(5, LEFT), Step::Toggle(0));
        assert_eq!(step(5, RIGHT), Step::Toggle(0));
        assert_eq!(step(5, NONE), Step::Idle);
        assert_eq!(step(5, Input { up: true, left: true, ..NONE }), Step::Move(4));
    }

    #[test]
    fn slots_fit_in_the_free_ones() {
        let all: Vec<usize> = (0..ROWS).flat_map(slots).collect();
        assert_eq!(all, vec![14, 15, 16]);
        assert!(all.iter().all(|&s| (FIRST_FREE_SLOT..SLOT_COUNT).contains(&s)));
    }

    #[test]
    fn rows_are_spaced_like_the_last_two() {
        assert_eq!(row_y(300.0, 340.0, 0), 380.0);
        assert_eq!(row_y(300.0, 340.0, 1), 420.0);
    }

    #[test]
    fn texts_are_terminated_ascii() {
        for t in &TEXT {
            for s in [t.label, t.options[0], t.options[1], t.help] {
                assert_eq!(s.last(), Some(&0));
                assert!(s[..s.len() - 1].iter().all(|&b| (0x20..0x7F).contains(&b)));
                assert!(!s[..s.len() - 1].contains(&b'%'), "the help line formatter treats % as a code");
            }
        }
        assert!(selected(true, 1) && !selected(true, 0) && selected(false, 0));
    }

    #[test]
    fn panel_restretches_only_when_its_animation_changes() {
        for m in [0, 1, 4] {
            assert!(panel::restretch_after(m));
        }
        for m in [2, 3, 5, 6, 7, 8, 9] {
            assert!(!panel::restretch_after(m));
        }
    }

    #[test]
    fn panel_grows_by_one_row_step_per_row() {
        assert_eq!(panel::grow(300.0, 319.5), 19.5 * ROWS as f32);
        assert_eq!(ROWS, 1);
    }

    #[test]
    fn panel_plan_stretches_only_fresh_nine_slices_downward() {
        use panel::{plan, Sprite};
        assert_eq!(plan(true, 9, true, 0, 20.0), Sprite::Stretch([0.0, 0.0, 0.0, 20.0]));
        // Other flags kept apart from bit 3.
        assert_eq!(plan(true, 9, true, 0x06, 20.0), Sprite::Stretch([0.0, 0.0, 0.0, 20.0]));
        assert_eq!(plan(true, 9, true, 0x08, 20.0), Sprite::Done);
        assert_eq!(plan(true, 1, true, 0, 20.0), Sprite::Skip);
        assert_eq!(plan(true, 9, false, 0, 20.0), Sprite::Skip);
        assert_eq!(plan(false, 9, true, 0, 20.0), Sprite::Skip);
    }
}
