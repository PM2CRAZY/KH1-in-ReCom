# Re:CoM's Config menu, and our rows in it

Steam addresses. Tags: Checked (decomp / disasm / exe) means read; Assumption means inferred.
Built in `crates/kh1-recom` (`src/menu.rs`, `src/win/menu.rs`).

## Where it is

- Re:CoM has two option menus:
  - the PC port's launcher settings (`SettingMenu` namespace: display, sound, pad, keyboard;
    `config28.dat` / `config1525.dat`). Not the game's; not touched. Checked (RTTI, strings).
  - the game's own camp menu (`CCamp`, vtable `0x140509010`, constructor `0x1400D2730`, layout
    `camp.lad`, sprite set `camp:00`, update `0x1400D9C80`). Checked (RTTI + decomp).
- Camp menu items 0..6 go to states `1D 16 41 1A 44 47 4A` (`0x1400D9C80`, case `0x11`).
  Item 6 → states `0x4A..0x4C` → the **Config screen** `0x1400D3EB0`. Checked (decomp).

## The Config screen

- **Input / state** `0x1400D3EB0(camp)`, state at `camp+0xA8` (0 open, 1 opening, 2 active,
  3 closing). Checked (decomp + disasm).
  - State 2, each frame: draw mode 3 (animate), then confirm/cancel (`0x140111B90`) closes,
    else up `0x101000` / down `0x204000` move the cursor `camp+0xBE8` over **5 rows (0..4,
    wrapping)**, else for rows 0..3 left `0x408000` / right `0x802000` flip the row's byte
    `camp+0xBED + row` (0/1, wrapping). After any change: sound `0x1402AA360(1, -1)` and draw
    mode 9. Pad reads (`0x140111A70`) are pure reads of the pad state (`0x14029C220/240`).
    Checked.
- **Draw** `0x1400DF1B0(camp, mode)`. Mode 9 rebuilds the rows:
  - 5 labels (message ids `0x500100 / 101 / 103 / 105 / 106`) and 9 values (rows 0..3: two each,
    ids `0x500107..0x500113`; row 4: one, `0x500114 + byte 0xBF1`, display only). Text comes
    from the message table `0x140240160(DAT_14087CD78, id)`; it lives in message files, not in
    the exe. Checked.
  - Each item is a `CButton` (`operator new 0x1404ACE54(0x30)`, ctor `0x140112A90`, set up by
    `0x140112F10`, 13 arguments, see `src/win/menu.rs`), stored in **21 slots at
    `camp+0xB38`**. Re:CoM fills slots 0..13; **14..20 are never written** (only zeroed in the
    constructor and deleted in draw modes 6/9 and the teardown `0x1400D7C20`). Checked (every
    reference to `camp+0xB38..0xBE0`).
  - Positions come from `camp.lad` group `0xB` through `0x140246100(layout, 0xB, element, out,
    0)`: labels (x, y) elements (2,2) (3,3) (5,4) (7,5) (8,6); value x elements
    9,10,11,12,15,16,20,21,22, value y elements 9, B, D, F, 11 per row. Checked (disasm
    `0x1400DF5B7..0x1400DF878`).
  - The selected value of the cursor row gets the cursor sprite `camp+0xBE0` (animation `0xFA`,
    `0x100` for row 4), moved to the item's sprite position + 150.0 in x, and the help line
    `camp+0x9D0` gets help id `0x50041A + item` (`0x140259000`). Checked (disasm
    `0x1400DF931..0x1400DF9EF`).
- **Values and their store.** On opening the camp, bytes `0xBED..0xBF1` are read from
  `0x1401B0800(0, id)` with ids `65 66 63 67 61` (`0x140507C2C`); the camp's destructor writes
  them back with `0x1401B15D0`. That is a byte array in **save data** (set 0 of `0x14087B0E8`),
  shared with story flags: 73 functions read it with many ids, and event scripts may too, so a
  free byte can't be proven unused. Checked (decomp). **Our values therefore stay in
  `kh1-recom.ini`.**
- Row 4 (id `0x61`, 4 values, never changed here) is probably the difficulty. Assumption. The
  meaning of rows 0..3 isn't known without the message files. Not needed.

## Our rows (native)

Two rows after row 4, drawn and run by the code above:

| row | label | options | ini key |
|---|---|---|---|
| 5 | Combat | Re:CoM / KH1 | `combat` (default kh1) |
| 6 | KH1 motions test | Off / On | `kh1_motions` (default off) |

- **Hook `0x1400D3EB0`** (bytes `48 89 5C 24 08 57 48 83 EC 20`). Re:CoM's rows go to the
  original. Changed only: up on row 0 → row 6 (was 4), down on row 4 → row 5 (was 0). On our
  rows: up/down move, left/right flip, each followed by Re:CoM's cursor sound and a mode-9
  redraw, as the original does. Confirm/cancel always goes to the original.
- **Hook `0x1400DF1B0`** (bytes `48 8B C4 55 41 54 41 55 41 56`). After mode 9, builds our 2
  labels and 4 values in slots 14..19 with the same calls and arguments the original uses for
  row 3 (same fonts, sequence tables `0x1407C1230` / `0x1407C1240[6..7]`), at row 3's x and one
  row step (row 4 − row 3) further down per row. When the cursor is on our row, moves the cursor
  sprite and sets our help line exactly as the original does for its rows. Re:CoM deletes our
  items with its own.
- A flip updates the setting, rewrites `kh1-recom.ini` at once (other lines and comments kept),
  and for "KH1 motions test" re-points the action rows (`recom-motion-switch.md`).
- Text: our labels, options and help are NUL-terminated ASCII handed to the same text items
  (the message table is bypassed, not changed). **Assumption:** ASCII renders as itself in the
  menu font. Not seen in game.
- **Assumption:** the window art has room for two more rows below row 4. The positions are
  computed at runtime from the layout; whether the panel graphic covers them is only visible in
  game.
- `config_menu = off` in the ini leaves the menu untouched (a safety switch while untested).
- Every hooked and called function's first bytes are checked against the Steam exe before
  anything is written; the two hooks go in together or not at all.

## Needs in-game testing

- The rows show, are readable, sit inside the window, and the cursor and help line follow.
- Up/down wrap through rows 0..6; left/right flip; the ini changes on each flip.
- Opening and closing the camp and the Config screen repeatedly (item ownership and deletion).
