# Equipping Keyblades: the camp menu's 8th entry (decision 25)

At `combat = kh1`, in Sora's story, Re:CoM's camp (pause) menu gets an 8th top-level entry,
"Keyblades", below Config. Its page lists the Keyblades Sora has earned, marks the equipped one,
shows the highlighted one's Strength, Magic and reach, and equips on confirm. Re:CoM's own camp
code draws and runs all of it. At `combat = recom`, or in Riku's story, there is no entry and the
camp behaves as unmodded.

Code: `crates/kh1-recom/src/keyblades.rs` (rules, tested), `src/win/keyblades.rs` (wrappers and
page), `src/win/menu.rs` (`cfg_input` dispatch), `src/win/stats.rs` (the equipped Keyblade).
Research: the notes `recom-camp-menu.md` and `recom-cards-save.md` (outside the repo).

Steam addresses (image base 0x140000000). Tags: **Checked (decomp)**, **Checked (exe)** = disasm
or bytes read from the Steam exe, **Assumption**.

## The data

| what | where | tag |
|---|---|---|
| Equipped Keyblade | one byte at `*(u64*)0x14087B110 + 0x2674` (= card collection `save + 0x1A74` + 0xC00). 0 = not set (Kingdom Key), else the card kind 1..0x18. Re:CoM saves, checksums, loads, snapshots and zeroes it itself. | Checked (decomp + exe, `0x140158F20`, `0x14012BAE0`, `0x1401B1300`) |
| Its validation | 1..=0x18, not Soul Eater (0x12), with numbers in `kh1-stats.json`; else the Kingdom Key. The Kingdom Key is written as 0, so a save that never leaves it keeps the spare range all zero. | (ours, tested) |
| Readers | the KH1 damage (`kh1_hit`) and the blade test (`win/attack.rs`) read it through `stats::equipped_weapon()` (one pointer read per hit). | — |
| Spare range check | at the first camp open per session the log says whether `save + 0x246B ..= 0x2817` is all zero apart from our byte. | — |
| Unlocked | `0x14015B3F0(kind)` or `0x14015B3F0(kind | 0x100)` ("obtained at least once", discovery state 2 or 3), called only when the collection pointer `0x14087A7E8` is set. The Kingdom Key is always listed; Soul Eater never. Only Keyblades with numbers in the stats file are listed (kinds 1..0x11 from KH1's rows, and from step 3 0x13..0x18 from KH2's, `recom-keyblade-models.md` § 12). | Checked (decomp + exe) |
| Names | `0x140164CA0(&kind)`: message `[0x140528560 + kind × 4]` through `0x140240160(*(u64*)0x14087CD78, id)`; it reads only byte 0 for kinds below 0xDB. Re:CoM's own localized card names. The stats file's name is used if the message is empty. | Checked (decomp + exe) |
| Riku's story | `0x1401AAAE0() != 0`, and the camp's own copy `camp+0x21BC`. | Checked (exe) |

## The entry: four wrappers

Re:CoM's camp has a hard limit of 7 entries (fixed arrays `camp+0x1B8[7]` buttons and
`+0x1F0[7]` labels, the `cmp [camp+0xB8],6` confirm gate at `0x1400DAABD`, a 7-byte state table
in state 0x11, a 7-entry jump table). Checked (decomp + disasm). Our entry is a third instance of
Re:CoM's entry button, made by the same calls, held outside those arrays; four wrappers make the
camp's own code treat index 7 as one more entry. Installed all or nothing, after the Config hooks
(the page runs through the Config input hook), never when `config_menu = off`.

| function | first bytes (Steam) | what the wrapper does |
|---|---|---|
| camp update `0x1400D9C80` (vtable slot `0x140509018`) | `48 8B C4 55 41 54 41 55 41 56 41 57` | Re:CoM's update always runs. States 7 / 0xD: when button 0's animation had ended, ours gets animation 5 as Re:CoM's loop gives its own (not in 0xD when the cursor is on ours: Re:CoM's 0xD then re-highlights it with animation 3). State 0xF: a held "down" on entry 6 (Re:CoM stops at its last entry) moves to ours with the cursor set, sound 1 and `0x1400DE100`; confirm on ours runs Re:CoM's confirm sequence (below); Re:CoM's confirm of one of its own entries gives ours animation 6. State 0x11 with the cursor on ours: Re:CoM's table would give 0x2E (a deck-editor state, from the stack's 8th byte); after the original we write 0x4A, so our page opens in Re:CoM's Config states. |
| entry build `0x1400E3DE0(camp, mode, level)` | `48 8B C4 55 56 57 41 54 41 55 41 56` | After the original, only at level 0 (`camp+0xB4`). Mode 0: create ours when wanted and absent, delete it when not wanted, else animation 4. Mode 1 / 2: animation 5 / 6. Created as `0x1400E3DE0` creates Re:CoM's: `C2DSeqCtrl` (`0x140247260`), `+0x16E = 0xC`, `+0x138 = 1`, vtable `+8` sequence 4, position `0x140248660` at (86.0, 88.0 + 26.0 × 7 = 270.0), a `C2DFnt` (`0x140243F10`, colour 0x80404040, font 0xF, 0x40) with "Keyblades", attached with vtable `+0x60` at (0.0, 6.0). Checked (disasm `0x1400E3E80..0x1400E3F8A`). "Wanted" = Combat = KH1, Sora's story, not the direct-map camp (`+0x20A4`), Keyblade numbers loaded. Switching Combat in Config adds / removes it when Re:CoM rebuilds the list on leaving Config (state 0x4C). |
| cursor set `0x1400DEA10(camp, index, show)` | `48 8B C4 48 89 58 18 48 89 70 20 55 57 41 56` | With ours, at level 0: wraps over 8 (−1 → 7, 8 → 0). Moves between Re:CoM's entries go to the original. A move from or to ours does the original's work with Re:CoM's calls: old entry animation 5, label 0x80404040 (`fnt+0x34`), priority `0x140247AA0(btn, 0xC)`, help cleared with `*(char**)0x1407C12E8`; new entry animation 3, label 0x80C0C0C0, priority 0xD, highlight bar `0x1400E07D0`, glove `0x1400E3D30`, `camp+0xB8`, help: ours, or message `0x500400 + index` for Re:CoM's (its table for Sora's top level). |
| teardown `0x1400D7C20` (both dtors, state 0x50) | `48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18` | Deletes our page's objects and our entry, then the original. |
| Config input `0x1400D3EB0` (hooked since 0.4.0) | `48 89 5C 24 08 57 48 83 EC 20` | Before anything else: when our entry exists for this camp and `camp+0xB8 == 7`, our page runs in its place (same contract: ≥ 0 running, < 0 closed). |

### The confirm sequence on ours

Re:CoM's state 0xF for an entry with a page (disasm `0x1400DAB0A..0x1400DACAC`), run after the
original's frame (which, for index 7, only eased the glove with `0x1400DE100`; nothing below
changes its target):

1. `0x1402B57A0(0, 0)`; 2. `0x1400D3A70(camp)`; 3. sound `0x1402AA360(2, −1)`;
4. the cursor set's work for (7, show 0);
5. every Re:CoM button and ours: vtable `+0x20(btn, 0, set, 6, 0.0, 0)`;
6. header item 0: `0x1400E4220` mode 1's work (header present) with our title where Re:CoM puts
   `0x1400D3890`'s ("" for index 7): animation 7, old title `C2DFnt` deleted, a new one made with
   "Keyblades" (colour `camp+0x98`, font 0xF, 0x40), vtable `+0x68(hdr, 1, set, fnt)`,
   `0x140247DC0(hdr, 1)`, `camp+0xC4 = 2`; then `0x140247DC0(camp+0x228, 1)`;
7. `camp+0x140` sequence 0xC; 8. `0x1400E6700(camp, 2)`; 9. `0x140247E50(camp+0x130, 2, 0)`;
10. state 0x11; 11. `camp+0xBC = 7`, `0x1400E46D0(camp, 4)`.

Checked (decomp + disasm). The only difference from Re:CoM's order: the update's per-frame trailer
(`0x1400E4220` ×2, `0x1400DEC60` ×2, `0x1400E8760(9)`) ran before these steps this frame instead
of after. Its header call is mode 999 (idle) before and mode 2 after; mode 2 first waits for the
header's new animation 7 to end, so it does nothing on this frame either way. Checked (decomp).

## The page

Runs inside Re:CoM's Config states: 0x4A (`camp+0xB4 = 2`, first call), 0x4B (each frame until
it returns < 0, then Re:CoM closes the header), 0x4C (Re:CoM rebuilds the list and returns to
state 0xD, which re-highlights our entry through the cursor set). Its state is ours (`PAGE`); it
writes no camp field Re:CoM's pages use. It follows the entry-4 page (`0x1400EB7E0` / draw
`0x1400E6DF0`), with Re:CoM's classes and calls. Checked (decomp + disasm):

| part | made as Re:CoM makes it |
|---|---|
| status panel | `C2DSeqCtrl`, `+0x16E = 0x10`, `+0x138 = 1`, layout group 0x11 element 3, sequences 0x125 open / 0x126 idle / 0x127 close |
| list panel | `C2DSeqCtrl`, kind 0x11, layout group 0x11 element 2, sequences 0x14C / 0x14D / 0x14E |
| cursor | `CButtonCursor` (0x1C0 bytes, ctor `0x1400D25F0(obj, 0x12, list panel, 0x12D)`), shown when the opening ends, glides by `0x1400DE010(cursor, dt × 0.02)`, hidden on close by `0x1400E5CF0` |
| scroll bar | `CScrollBar` (0x28, ctor `0x140112AB0`, setup `0x1401130C0(bar, set, 0x14, seqs 0x1407C1228, (133, −56), 157.0, list panel)`, update `0x140112CB0(bar, min(1, top/n), min(1, 8/n))`) |
| list rows | 8 visible `CButton`s (`0x140112A90`, setup `0x140112F10`, kind 0x11, seqs `0x1407C11F0`, layout group 0x12 element 2+i, font 0xF (0xE for languages 3..5 or `camp+0x9C`), offsets 1.0 / 2.0, colour `camp+0x98`, variant 0), rebuilt on each move (draw mode 9). Top row: the cursor stays in the 8 visible rows. |
| equipped mark | the row sprite's marker, vtable `+0x40(sprite, set, 0x1D, 170.0, −13.0, 0.0, 0)` then the new object's `+0x30(obj, camp+0x21D8)`, as Re:CoM marks its rows ("seen" 0x1D / "new" 0x1E). **Assumption**: 0x1D reads as a mark here. |
| status rows | 3 `CButton`s, kind 0x10, seqs `0x1407C11D0` (variant 0..2 = sequence 0x12C), layout group 0x11 elements 4..6, y + 8.0, font 0xE; a value `C2DFnt` (colour 0x80046060, font 0xE, 0x41), text copied with `0x140258DE0`, attached at (220.0, 1.0). Labels "Strength", "Magic", "Reach". Rebuilt for the highlighted Keyblade on each move (Re:CoM builds its status rows once). |
| help line | ours while the page is open; on close the message `*(u32*)0x1407C106C`, as the entry-4 page restores. |

Input (sub-state 2), in `0x1400EB7E0`'s order: cancel `0x140111B90` closes (both panels' close
sequences, cursor hidden, sound 4) and the page returns −1 when the status panel's close ends,
deleting its objects in Re:CoM's order (draw mode 6); confirm `0x140111C30` equips the
highlighted Keyblade (save byte, sound 2, rows redrawn so the mark moves); up / down through
`0x140111A70` (repeat; the fresh flag it leaves in `camp+0x94` decides the wrap at the ends);
page up / down on pad bits 4 / 8 (`0x14029B2E0` repeat, `0x14029B2F0` fresh), with Re:CoM's
±8 and top-row rules. Any move plays sound 1.

Values: Strength and Magic are the KH1 weapon row's bonuses ("+3"; for the six KH2 Keyblades
KH2's stat row's); reach is the blade span `|far − near| × fraction scale` over the Kingdom Key's
("x1.00"), a display figure only.

Differences from the entry-4 page, by design: no tab bar (`camp+0xC08`, sequences 0x14F..) and no
`0x1400DEC60(camp, 1, 0)` side element (the Config page, whose states we run in, has neither); up
past the first row wraps to the last (Re:CoM's goes to its tab bar).

## Per frame

The camp exists only while the pause menu is open: nothing of this runs in field or battle
frames. With no entry (Re:CoM setting, Riku's story) each wrapper is one load and the original.
With the entry, the update reads the pad (pure reads) in state 0xF and a few fields; the page's
code runs only while it is open, like Re:CoM's pages.

## Assumptions and risks

1. **Room on screen** at y = 270 (highlight bar 262, glove 264): the owner's 720p screenshot
   shows the 8th entry would sit at screen y ≈ 485–530, with about 28 px above the bottom HUD and
   clear of Sora's status box. Not seen in game yet.
2. **Text items own nothing we delete**: Re:CoM's teardown deletes its buttons, never their
   labels; the replace path `0x140246E90` clears the old sequence object's item pointer before
   releasing it, so the items go with their sequences. We delete only buttons, panels, the cursor
   and the scroll bar, as Re:CoM does. Inferred from the decomp, not traced: watch for leaks or a
   crash on closing the page or the camp.
3. **The equipped mark** (0x1D) and the **status rows' look** (sequence 0x12C, rebuilt on each
   move): to judge in game.
4. **ASCII texts** ("Keyblades", help lines, "Strength", "+3", "x1.00") render as on the Config
   row; `+` and `x` are not seen yet. Card names are Re:CoM's localized messages.
5. **The spare range** is unused by vanilla code (decomp + exe scans); a pointer from elsewhere
   can't be ruled out statically. The log reports it at the first camp open each session.
6. **Retry snapshots** (`0x1401B03C0` / `0x1401B1410`) copy the whole save block: a Continue after
   a game over can restore the Keyblade equipped before the checkpoint, like Re:CoM's own deck
   edits. Checked (decomp); when snapshots happen is an Assumption.

## What the first in-game test must check

1. The 8th entry at y ≈ 270: clear of the bottom HUD, label "Keyblades", help "Equip a
   Keyblade.", highlight bar and glove on it.
2. Cursor rules: held down from Settings stops on Keyblades; held down there stops; a fresh
   press wraps to Review Decks; a fresh up on Review Decks wraps to Keyblades.
3. Confirm: sound, buttons fade, header title "Keyblades", the page opens (not the deck editor).
4. The page: both panels, rows with the card names, the mark on the equipped one, the cursor,
   the scroll bar (17 rows > 8), the status rows' values, the help line. Confirm moves the mark;
   cancel closes back to the list with Keyblades highlighted.
5. The log: "keyblades: camp wrappers in", the spare-range line, "page opened", "equipped".
6. Damage changes with the equipped Keyblade's Strength (KH1 setting, a twin enemy).
7. Combat = Re:CoM in Config: leaving Config removes the entry; back to KH1 adds it.
8. Riku's story: no entry.
9. Close the camp with Start while on the entry, and quit to title with the page open: no crash.

## In game, 0.8.0 (owner, 2026-10-31)

- Owner: "it all looks good". The entry, the page, equipping, removing the entry at the Re:CoM
  setting and putting it back all work. Checked (owner + log).
- Log on the owner's late-game save (Sora at level 99): `save+0x246b..=0x2817 is all zero apart
  from our byte`. The spare range was unused in a real save. Checked (log).
- The log also shows 17 Keyblades listed (cards 1..0x11, all KH1's, all ever obtained) and
  equips from 0x1 up to 0x11 saved in the byte. Checked (log).
