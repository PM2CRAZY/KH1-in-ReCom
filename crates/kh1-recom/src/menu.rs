//! Our rows in Re:CoM's Config menu (camp menu → Config): the testable part.
//!
//! Re:CoM's Config screen is `CCamp`'s sub-screen run by `0x1400D3EB0` (input) and drawn by
//! `0x1400DF1B0` (mode 9 rebuilds the rows). It has 5 rows (cursor `camp+0xBE8`, 0..4): rows 0-3
//! are two-choice toggles kept in bytes `camp+0xBED + row`, row 4 shows a 4-value setting it
//! cannot change. Checked (decomp + disasm). We add rows 5 and 6 below row 4, drawn by the same
//! game code (its `CButton` items, its layout positions, its cursor sprite and help line).
//! See `docs/recom-config-menu.md`.

/// Re:CoM's last row.
pub const LAST_RECOM_ROW: i32 = 4;
/// Our first row.
pub const FIRST_ROW: i32 = 5;
/// How many rows we add.
pub const ROWS: usize = 2;
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
    RowText {
        label: b"KH1 motions test\0",
        options: [b"Off\0", b"On\0"],
        help: b"Debug: Sora plays KH1's idle, walk, run and swings. Applies from the next room.\0",
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
/// layout's elements as read at runtime; whether the window art covers two more rows is not
/// known until seen in game).
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
    fn edges_lead_to_our_rows() {
        assert_eq!(step(4, DOWN), Step::Move(5));
        assert_eq!(step(0, UP), Step::Move(LAST_ROW));
        assert_eq!(step(5, UP), Step::Move(4));
        assert_eq!(step(5, DOWN), Step::Move(6));
        assert_eq!(step(6, UP), Step::Move(5));
        assert_eq!(step(6, DOWN), Step::Move(0));
    }

    #[test]
    fn our_rows_toggle() {
        assert_eq!(step(5, LEFT), Step::Toggle(0));
        assert_eq!(step(5, RIGHT), Step::Toggle(0));
        assert_eq!(step(6, RIGHT), Step::Toggle(1));
        assert_eq!(step(6, NONE), Step::Idle);
        assert_eq!(step(6, Input { up: true, left: true, ..NONE }), Step::Move(5));
    }

    #[test]
    fn slots_fit_in_the_free_ones() {
        let all: Vec<usize> = (0..ROWS).flat_map(slots).collect();
        assert_eq!(all, vec![14, 15, 16, 17, 18, 19]);
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
}
