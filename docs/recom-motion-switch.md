# The "KH1 motions test" switch (M2, runtime half)

Steam addresses. Tags: Checked (decomp / disasm / exe / files) or Assumption. Built in
`crates/kh1-recom` (`src/motions.rs`, `src/win/motion.rs`).

## How Re:CoM names and plays Sora's motions

- Per-character action tables: `0x1407C6300[char]` (`[1]` and `[2]` → Sora's `0x1407C5E80`,
  `[3]/[4]` → `0x1407C6370`, `[5]/[6]` → `0x1407C67F0`). Rows are 16 bytes
  `{const char* name; int index; int pad}`; Sora has 0x47 rows, then a null name. All indexes
  are -1 in the exe. The table is in `.data`. Checked (exe).
- **Resolve:** the player's `stateInitial` `0x1401FA6E0` (and `0x1401F4240` for a given
  character) walks the rows and sets `index = 0x1402A42B0(resource name, row name)`. That is an
  exact name compare over every clip of the loaded pack (count = the `.ANN` header's, no fixed
  limit), so slots ≥ 161 resolve. Clip names are the header's 16-char names
  (`strncpy(…, 16)` in `0x14027ABC0`). Checked (decomp).
  - `0x1401A47E0` (reads the 3 trailing digits as the slot, then compares names) is the
    by-name lookup other actors use; it also copes with 161+ as long as the slot is below the
    pack's count. Not used by the action table. Checked (decomp).
  - The `.ANN` loader `0x14029F3A0` sizes its tables from the header count. Checked (decomp).
- **Play:** `0x1401F7180(player, row, blend, root, …)` reads `row.index` and calls the actor's
  vtable `0x70` (`0x1401A3470`). With Sora's weapon flag it plays the **same index on the
  Keyblade actor** too. An index ≥ the pack's count gives a null clip (`0x14024C760`), and the
  motion update skips a null clip (`0x14024E470`). The effect and sound tracks look the index
  up and find nothing for new slots (`0x14017A240`, `0x140239260`). Checked (decomp).

## Which rows

| row | Re:CoM clip | what | KH1 entry (request) |
|---|---|---|---|
| `0x01` | p001_001 | field idle | 0, else 1 (req 0) |
| `0x02`, `0x12` | p001_002 | field walk | 2, else 3 (req 1) |
| `0x03` | p001_003 | field run | 5, else 6 (req 2) |
| `0x13` | p001_023 | field run, other row | 5, else 6 (req 2) |
| `0x2A` | p001_031 | battle-stance idle | 1 (req 0) |
| `0x2B` | p001_020 | battle-stance walk | 3 (req 1) |
| `0x2C` | p001_024 | battle-stance run | 6 (req 2) |
| `0x30` | p001_055 | ground swing 1 | 98 (req 200) |
| `0x31` | p001_056 | ground swing 2 | 99 (req 201) |
| `0x32` | p001_057 | ground lunge | 100 (req 202) |
| `0x33` | p001_058 | ground finisher | 101 (req 203) |

- Row choice: `0x1401F9380` picks 1 / 2 or 0x12 / 3 or 0x13 by action 2/3/4 (stand/walk/run;
  0x12/0x13 when `player+0x418` bit 1 is clear), and 0x2A/0x2B/0x2C in battle stance
  (`player+0x428` bit 0 or `0x1401E6AC0`). The idle exe `0x1401EF170` plays 1 (6 when
  airborne). Attack rows 1–4 of `0x1407C6C70` play actions 0x30–0x33 (situations 1 swing,
  2 side swing, 3 lunge, 4 finisher; `0x1401F7520`). Checked (decomp + exe).
- KH1 entries: field 0/2/5 and battle 1/3/6 (`kh1-data`, `kh1-core`), swings 98–101
  (`motion-conversion.md`). Which KH1 stance variant fits Re:CoM's field rows is an
  **Assumption**; a field row takes the battle entry when the manifest has no field one.
- The lunge (`0x32`) is included because it is one of the ground combo's swings in Re:CoM.

## What the switch does

- At start (Panacea `OnInit`), when every site check passed:
  1. Checks `0x1407C6300[1] == 0x1407C5E80` and that each row above still names its Re:CoM
     clip.
  2. Reads the manifest `kh1-motions.json` (first found: `<mod>\kh1-motions.json`,
     `<mod>\PLAYER\0001\kh1-motions.json`, `<game>\kh1-motions.json`) and the header of the
     `PL0001.ANN` Panacea will serve (`<mod>\PLAYER\0001\` or `<mod>\raw\PLAYER\0001\`).
     `<mod>` is the folder Panacea passes to `OnInit` (else the DLL's folder's parent).
  3. Plans every row (all or nothing): a manifest clip per row, at slot ≥ 161, whose name and
     non-empty slot are in the `.ANN` header.
- On (ini `kh1_motions = on`, or the Config row): each row's name pointer is re-pointed to a
  static string (`p001_161`…) in our DLL, with a verified old value. Off puts Re:CoM's pointer
  back. Re:CoM resolves the names at the next player set-up, so the switch **applies from the
  next room load**. Assumption: the player is set up again on every room load.
- The index field is not touched: it is -1 until the first set-up and rewritten from the names
  at every set-up, so a reset adds nothing, and live it would make a playing Sora request slot
  -1.
- No per-frame code.

### Manifest shape the DLL reads

Either a top-level array or `{"slots": [...]}` (also `motions` / `clips`). Per clip: `slot`
or `name` (`p001_161`), and `kh1_entry` (or `entry`) and/or `kh1_request` (or `request`);
numbers or `"0xC8"` strings. Matching is by entry; by request only when exactly one clip has
it.

## Differences that remain (not 1:1 yet)

- Blend and root flags stay Re:CoM's: locomotion is requested with blend -1 (the clip's own
  default blend, clip header `+0x18` × 50) and no root motion; swings with blend 0 and root
  motion. KH1 uses blend 8 (idle 10) for locomotion. If the converter writes KH1's blend into
  the clip's default-blend field, Re:CoM's -1 picks it up. Assumption until compared.
- Hit windows, lunge distances and combo timing stay Re:CoM's attack rows (`0x1407C6C70`),
  made for Re:CoM's clips. That is M5.
- The Keyblade actor gets the same slot. If its pack has no such slot it gets a null clip and
  stops animating for those motions. Its pack's slot count was not checked. Assumption.
- Character table `[2]` shares Sora's table, so the same rows change for that character
  (which one `[2]` is was not traced).

## Needs in-game testing

- With the assets built and the switch on: after a room load, Sora's idle, walk, run and ground
  swings are KH1's; a pose capture matches the converter (the M2 "done when").
- Turning it off in Config: Re:CoM's motions come back after the next room load.
- The Keyblade during KH1 clips; foot sliding (root motion and walk/run speeds are Re:CoM's).
