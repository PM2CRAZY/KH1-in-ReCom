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

## The window panel

- `camp+0xB30` is a `D2::C2DSeqCtrl` (ctor `0x140247260`, base vtable `0x140607340`), made in
  draw mode 0 with `vtable+8` (`0x140247850`) playing animation **`0xF7`** (open) from the
  camp's sprite set `camp+0x80`. Mode 1 switches it to **`0xF8`** (idle) once `0xF7` ends
  (`0x1402480C0`), mode 4 to **`0xF9`** (close), both with `vtable+0x20` (`0x1402482F0`,
  slot 0); mode 6 deletes it. Every Config item and the cursor are its children (`CButton`
  setup `0x140112F10` parents the item's sprite with `0x140248650` and gives it the raw layout
  position with `0x1402485B0`; the panel itself sits at (-256, -h/2), `0x140248660`, h =
  `DAT_140821ADC`, 384 in the exe image). So layout units are the panel's local units.
  Checked (decomp + disasm).
- Each animation change makes a new `D2::C2DSeqObj` whose list (`+0x20`; first at list `+8`,
  next at object `+8`) holds `D2::C2DSprObj`s (vtable **`0x1406077D8`**, ctor `0x140249FC0`),
  one per layer, each with `+0x142` parts. The ctrl keeps a handle per sequence at `+0x144`
  (count `+0x16D`); **`0x140246210(handle)`** returns the `C2DSeqObj`. Checked (decomp + exe).
- **Re:CoM has a native 9-slice.** A `C2DSprObj` with exactly 9 parts and flag 8 in `+0x144`
  is drawn by its update `0x14024BB10` (called from its vtable slots `0x14024A3F0` /
  `0x14024A7E0`) through **`0x14024AD40`**, which moves the 3×3 parts' corners by four edge
  offsets: left `+0xA0`, top `+0xA4`, right `+0xA8`, bottom `+0xAC` (top row moves by top,
  bottom row by bottom, the middle row stretches between). Checked (decomp + disasm).
  - Re:CoM's own resize, `C2DSprObj` vtable `+0x38` = **`0x14024C270(obj, w, h)`**, sets
    those four offsets so the centre part spans (-w/2, -h/2)..(w/2, h/2) and sets flag 8;
    `0x1402493B0(seqObj, w, h)` calls it on every sprite of a sequence. Its only user is the
    message window `0x1402B3040` (`message.sqd`, after each animation change). Checked
    (disasm: w, h in xmm1, xmm2).
  - The fresh object has flag 8 clear (`+0x142` is set to `00 00 00 FF` as a dword by the
    ctor). Checked (disasm `0x14024A00C`).
- The 9-slice path is **hard-wired to exactly 9 parts** (`cmp byte [obj+0x142], 9` in
  `0x14024C270`; the same check in `0x14024AD40`). Other sprites are drawn from their parts'
  rects as they are. Checked (disasm).
- A part (`0x98` bytes, array at `C2DSprObj+0x120`) has at `+0` a pointer to a 0x14-byte entry
  `{i32 x0, y0, x1, y1; u32 uv index}` in the loaded sprite set (set by `0x14024B910` from
  `0x140249150`), so shared with every other user of the set. Each update `0x14024BB10` reads
  it with `0x14024B380` (corners TL, TR, BL, BR), transforms them into the part's vertices
  `+0x40..+0x5F`, and the draw `0x14024A3F0` uses those. Nothing else reads `+0`. Checked
  (decomp + disasm).
- A 1-part sprite whose UVs are all zero gets a `CScissoring` (`obj+0x128`) instead of a quad:
  its corners become a clip rect (`0x14024A3F0`, `0x140265F00`). Checked (decomp).
- **In game (0.4.1, owner's screenshot and log, 2026-10-30):** the panel's open animation has
  4 sprites with **1, 8, 9, 8** parts. The 9-part one is the dark fill: with the bottom offset
  it reached one row lower, exactly the margin it had below "Game Mode" (so its scale at rest
  is 1, and the row step is **26.0** layout units). The glossy border stayed at the old height:
  the 8-part sprites are the frame (3x3 without the centre) and its highlight. Checked (in game).
  Nothing else in the screenshot ends at the old bottom, so the 1-part sprite needs nothing
  (Assumption; the log now prints its rect and whether it is a scissor).

## Our rows (native)

Rows after row 4, drawn and run by the code above (the "KH1 motions test" row was removed by
owner decision, 2026-10-30; see `recom-motion-switch.md`):

| row | label | options | ini key |
|---|---|---|---|
| 5 | Combat | Re:CoM / KH1 | `combat` (default kh1) |
| 6 | Guard reactions | Off / On | `guard_reactions` (default on; decision 27) |

- "Guard reactions" only has an effect at Combat = KH1 (Re:CoM's Sora has no Guard; at Re:CoM
  nothing reads it). What it does: `recom-kh1-attack.md` § Enemy reactions to Guard. Its flip
  takes effect at once (no restart).

- **Hook `0x1400D3EB0`** (bytes `48 89 5C 24 08 57 48 83 EC 20`). Re:CoM's rows go to the
  original. Changed only: up on row 0 → row 6, our last (was 4), down on row 4 → row 5 (was 0).
  On our rows: up → the row above (5 → 4), down → the row below (6 → 0), left/right flip that
  row, each followed by Re:CoM's cursor sound and a mode-9 redraw, as the original does.
  Confirm/cancel always goes to the original (it never indexes by the cursor there; its draw
  only compares its rows 0..4 with the cursor, `0x1400DF91C..0x1400DF925`, Checked (disasm)).
- **Hook `0x1400DF1B0`** (bytes `48 8B C4 55 41 54 41 55 41 56`).
  - After mode 9, builds each row's label and 2 values (row 5: slots 14..16, row 6: 17..19;
    Re:CoM leaves 14..20 unused) with the same calls and arguments the original uses for row 3
    (same fonts, sequence tables `0x1407C1230` / `0x1407C1240[6..7]`), at row 3's x and one row
    step (row 4 − row 3) below the row before. When the cursor is on one of our rows, moves the
    cursor sprite and sets that row's help line exactly as the original does for its rows.
    Re:CoM deletes our items with its own.
  - **Panel taller by one row step per row we add (two).** After modes 0, 1 and 4 (the ones
    that can give the panel a new animation), on each object of the panel's current sequence
    with the `C2DSprObj` vtable. The row step is layout group `0xB` element `0x11` y − element
    `0xF` y; the panel grows by 2 × that.
    - **9 parts** with flag 8 clear (the fill): edge offsets (0, 0, 0, 2 × row step) and flag
      8. Re:CoM's own 9-slice code draws its bottom two rows lower, the top where it was. Not
      Re:CoM's resize call, because that recentres the window on its origin (the screen's
      top-left here). Flag 8 set means done.
    - **Other part counts (≥ 2) shaped like a frame** (parts wholly above the sprite's middle,
      wholly below, and across it; `menu::panel::frame_rects`): each part with a y below the
      middle gets its own copy of its rect entry with those y moved down by 2 × the row step, and
      its `+0` repointed to the copy. The bottom row moves, the side edges stretch, the top row
      keeps Re:CoM's entry. It is the same move the 9-slice bottom offset makes, in the same
      part-local units, so frame and fill stay together through the open and close
      animations. The copies are made once per source entry, kept for the process, reused
      (keyed by source address, its bytes and the new rect; at most 256), and a sprite whose
      parts already point at a copy is done. The set's own data is never written.
    - **1 part:** left (logged with its rect and whether it is a scissor).
    - Mode 1 runs each frame only while the screen opens; mode 3 (every frame while open) is
      not touched, so nothing of ours runs per frame once open.
  - The log prints `window panel, 2 rows (X units) taller; sprites: …` (what each sprite
    got) whenever something changed, at most 6 times per session.
  - **Assumption:** the frame sprites have the same scale as the fill at rest (seen for the
    fill; the frame is checked by the next screenshot).
- New checked sites (bytes from the Steam exe): `0x140246210` (called:
  `48 83 EC 28 48 83 3D 1C 6C 63 00 00 8B D1`), `0x14024AD40`
  (`48 83 B9 20 01 00 00 00 0F 84 0B 06 00 00`) and `0x14024C270`
  (`40 53 48 81 EC 90 00 00 00 0F 29 B4 24 80 00 00`), checked only, since their meaning of
  the fields we write is what we rely on. The `C2DSprObj` vtable `0x1406077D8` is compared
  with each object's vtable before anything is written.
- A flip updates the setting and rewrites `kh1-recom.ini` at once (other lines and comments
  kept; an old `kh1_motions` line is left and ignored).
- Text: our label, options and help are NUL-terminated ASCII handed to the same text items
  (the message table is bypassed, not changed). Seen in game (owner's screenshot,
  2026-10-30): the rows render with Re:CoM's buttons, text and highlights. Checked (in game).
- **Help line:** with the cursor on our rows the help line shows our text:
  - Combat: "Re:CoM: card battles as in the original. KH1: Kingdom Hearts combat. (mod v0.6.2)"
    (the version is the DLL's own, built in from `Cargo.toml` since 0.6.2);
  - Guard reactions: "On: a blocked attacker flinches and a blocked shot flies back. KH1 combat
    only." (79 characters; ours are kept within 90, `menu::HELP_MAX`. Assumption: that fits the
    help box on one line, as the Combat help does in game). Mode 9 of the
  original first sets help `0x50041A` (its highlighted item index stays 0 when the cursor is
  on row 5), then our hook sets ours on the same help object. `0x140259000` keeps the pointer
  as is when `help+0x9C` is 0, else formats it with `0x1402402B0`; our text is static and has
  no `%`. Checked (decomp); not seen in game yet. On Re:CoM's rows it is Re:CoM's own help
  (the screenshots show the camera help with the cursor on row 0, which is correct).
- `config_menu = off` in the ini installs neither hook: the menu, the panel and its size stay
  exactly Re:CoM's.
- Every hooked and called function's first bytes are checked against the Steam exe before
  anything is written; the two hooks go in together or not at all.

## Needs in-game testing

- The panel's border (frame and highlight) now ends just below "Guard reactions" with its
  corners intact, lined up with the fill, through the open and close animations. Send the
  "window panel, 2 rows … taller; sprites: …" log lines (they list each sprite's parts and
  what it got, including the 1-part sprite's rect).
- The cursor and help line on both our rows; up/down wrap through rows 0..6; left/right flip
  the row under the cursor; the ini changes on each flip (`combat`, `guard_reactions`).
- Opening and closing the camp and the Config screen repeatedly (item ownership and deletion).
