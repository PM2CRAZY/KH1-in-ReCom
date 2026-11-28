# M7 stage 7.3: KH1's command menu (decision 34)

Owner (2026-10-31): "Command menu: Re:CoM's own menu pieces (buttons, cursor, text), laid out like
KH1's"; "shown in the field and in battle, as in KH1"; "the highlighted row uses another colour of
Re:CoM's command plate; row spacing KH1's 22 units"; "menu words: KH1's own text from the player's
KH1 message files".

At Combat = KH1, in Sora's story, field and battle: KH1's root list (Attack / Magic / Items / the
4th row: Summon, or KH1's blank dash until summons) bottom-left at KH1's place, the Magic list,
and KH1 FM's context tab (the green Triangle prompt above the list; 0.35.0, below). Re:CoM draws it: each row is a
`C2DSeqCtrl` playing one of two groups the asset step added to Re:CoM's `Cockpit:00` (Re:CoM's
text plate, and the same plate in another colour for the highlighted row) with a `C2DFnt`
carrying KH1's word, attached as Re:CoM attaches its own labels to these plates. KH1's rules run
the menu: its cursor, its greying, its fades, its context tab. Confirm runs the highlighted row:
on Attack it is today's Attack (unchanged); Magic opens the Magic list; choosing a spell casts
it (stage 7.4, `recom-m7-magic.md`: KH1's commit gate, then KH1's command on Re:CoM's magic
action). Items opens KH1's Items list of Sora's filled item slots (M8 stage 8a.1,
`docs/recom-m8-items.md`; greyed and locked with no item). At Combat = Re:CoM, or in Riku's story,
nothing is made and nothing runs.

M8b (2026-11-02): the Summon entry (row 3) and KH1's Summon list are built: `docs/recom-summons.md`.

Code: `crates/kh1-recom/src/command_menu.rs` (KH1's rules, tested), `src/win/command_menu.rs`
(Re:CoM's objects), `crates/kh1-recom-assets/src/command_menu.rs` (words and plates, tested).
No hook of its own: it runs from the MP bar's three `CPlayerGauge` wrappers and the request-bits
wrapper. Research: `m7-menus-mp.md` § 1, § 2, § 3, § 7 and `m7-magic.md` § 2.1, § 2.2 (kept with
the decomp tools, not in the repo).

Steam addresses (image base 0x140000000), both exes. Tags: **Checked (decomp)**, **Checked
(exe)** = bytes / disassembly of the Steam exe, **Checked (files)** = the owner's files (KH1
`btltbl.bin` and its PC assets, Re:CoM `FO0002.SPR` from the OpenKH extract), **Assumption**.

## KH1's menu

| what | KH1 (Steam) | tag |
|---|---|---|
| Lists: 0x38-byte records at `0x140528900 + type × 0x38`: `+4` grey, `+8` lock, `+0xC` cursor, `+0x10` count, `+0x14` command ids, `+0x24` brightness, `+0x34` the row a submenu was opened from. Root (type 0) has 4 rows in the exe image, Magic (1) is filled per frame. | `0x140528900` | Checked (exe + decomp) |
| Root build, every frame: row 0 Attack (1); Counterattack (0x63) instead when the ability is on, the guard recoil 0x6E plays and the target is an enemy. Grey = lock = `!(state flags >> 4) \| !1` (Attack needs the movement state's flag 0x10); rows 1..3 the blank command 0x42. | `0x140294220` | Checked (decomp) |
| Magic: the learned spells in KH1's order, command `level + (spell + 2) × 3`. A spell is usable with the state's flag 0x20, MP for its cost (`0x1402A4C10`), and, for command flag 8 (Thunder, Gravity, Stop), the target within the command's range `+0xE` (1100). Grey = lock = the unusable ones. With spells: row 1 = Magic, not locked, greyed when none is usable. With none: row 1 stays the blank command, greyed and locked. | `0x1402944C0` | Checked (decomp + files) |
| Items: row 2 = Items, greyed and locked with no item slots. | `0x140294220` | Checked (decomp) |
| Context: entry 3 cleared each frame, set (and ungreyed) by the nearby object's command (`0x140293000`). NPCs (kind 1) put Talk in row 0 instead. FM never draws entry 3 as a row: it shows in the context tab (below); the 4th row shows Summon (the Magic list's last entry, appended once summons are learned) or the blank command. | `0x1402948F0`, `0x140278B40` | Checked (decomp) |
| Movement-state flags: 0 ground 0xFC, 1 take-off 0x34, 2 air 0x3D, 3 0x05, 4/5 0x02, 7 0x01, 8 flight 0x76, 9–11 0x101, 0xD–0xF / 0x11 0x01, 0x15 glide 0x05, 0x1E 0x18. | `0x142DB3840` | Checked (decomp, `kh1-movement-motion.md`) |
| Cursor, on the fresh-press word compared for equality (two presses in a frame move nothing): Up / Down wrap; Right opens the row's submenu (sound 6), a locked row sounds 4, a row without one does nothing; Left or cancel closes a submenu (sound 3; cancel also puts the root cursor on Attack); Left at the root puts the cursor on Attack (sound 1; **since M12 step 5 ours switches the 4th row Summon ↔ Sleights instead**, owner, `recom-m12-sleights.md` § 8.5); Right on the root's row 3 goes to the run (sound 4 unless Summon). No auto-repeat. In the Magic list the Summon entry is skipped (count − 1). | `0x1402931B0` | Checked (decomp) |
| Step order: at the root a fresh jump is the jump and the menu takes no input that frame; Triangle runs the context command when its tab is fully in; Square without Triangle is the roll / guard and no cursor that frame; the cursor; then confirm runs the row (`0x140293410` → `0x140293480`): locked → sound 4; a submenu opens; a command is issued, the stack unwinds and the root cursor returns to Attack. Confirm on the root's row 0 when it holds Talk is sound 4; on row 3 (or Right there) it opens Summon through the Magic list's last entry when that is Summon, else sound 4. The cursor never runs the context command. | `0x1402937F0` | Checked (decomp) |
| Buttons (logical, `0x14028B0B0` / `0x140528290`): Up 0x20, Down 0x40, Left 0x80, Right 0x100, Cross 0x210001, Circle 6 (cancel 2 + jump 4), Triangle 0x4080008, Square 0x900010, L1 0x40000. | | Checked (decomp + exe) |
| Draw: root at (16, 260), rows 22 apart; Magic at x + 11, at the Magic row's y with fewer than 3 rows, else at 348 − 22 n. Text colour (alpha × 128, b, b, b) (`0x14027A920`). | `0x14027B870` | Checked (decomp) |
| Brightness: the top list's rows ease 32 a frame between 0x73 (grey) and 0xE6; when the top list changes they are set at once and the lists under it go to 0x73. | `0x14027B870` | Checked (decomp) |
| Fades: in / out 0.1 a frame (10 frames, `0x140278AC0` 1 / 0) when the step starts / stops running; events hide and show at once (2 / 3, `0x140293750`), and the stack goes back to the root on Attack. | | Checked (decomp) |
| Context tab: 0 hidden, 1 sliding in (opacity + 1/7, x −10 → 0 by 10/7 per step), 2 in, 3 fading out (opacity − 1/7), restarted when its command changes. Its step is half a frame (14 frames). 7.3 read the "scale" `0x142866D14` as an X scale: it only multiplies alphas (`0x142866A40` / `A44`), an opacity. Drawn by `0x140278B40` as its own green plate with the Triangle and the word, over the header (`docs/recom-kh1-hud.md` § FM context tab). | `0x140277CF0`, `0x140278B40` | Checked (decomp); the half step: as read in the research (`0x14233FBE0`) |
| Sounds: 1 move, 3 back, 4 error, 6 open (calls: `0x1402931B0` Up / Down / Left-at-root 1, close 3, locked Right 4; `0x1402948B0` and `0x140293480` open 6; `0x140293480` locked confirm 4 (fresh press only: the held-confirm path clears `0x142D53A40 & 0x2000000`, `0x1402937F0`); `0x1402937F0` row 0 Talk / row 3 without Summon 4). No sound when a command or spell is issued (`0x140293480` returns 1 without one). | `0x14017A040` | Checked (decomp) |

## Sounds (KH1's own, in Re:CoM's sound banks; 2026-11-01)

Owner: "going through the command menu plays the wrong sounds when confirming (opening the magic
sub menu for example)"; chose "Build a sound bank from your KH1 files at the asset step and play
KH1's exact menu sounds"; then sent a video of KH1's menu ("lets try to get this 1:1 in ReCoM").

**KH1** (Steam):

| what | where | tag |
|---|---|---|
| `0x14017A040(id)` = `SslSeCreate(2, id)` (`0x1400D2FD0` → `0x1400DE460`) then `SslSePlay(2, id, 0, 0x3FFF, 0x3FFF)` (`0x1400D3060` → `0x1400DECF0`). | | Checked (decomp + exe) |
| The two 0x3FFF words become floats (/ 16383.0 at `0x1403B1D04`) in `xmm3` and the fifth-argument slot, but `SslSePlay` uses only bank / no / id (Ghidra gives it three parameters; its body starts the sound with a zeroed local): each sound plays at its file's own volume. | `0x1400D3060`, `0x1400DECF0` | Checked (decomp + exe disasm of the start); that nothing deeper reads them: Assumption |
| Bank 2 is the "system" bank: `0x1404D5EB8` = 2 (`.data`, never written). For it `SslSeCreate` takes the loaded bank entry with bank `+0x10` = 2 and number `+0x14` = 0 and makes **sound `id` of that one SCD**: `CreateSound(handle, id, 0, 1, 0)` (`0x14004DBF0` at `0x1400DE50E`, `mov r8d,ebp` = id). (Other banks: the entry whose number is `id`, sound 0, `0x1400DE5AC`; not the menu's path.) 0.39.0 read the second path (one SCD per sound): wrong for bank 2. | `0x1400DE460` | Checked (decomp + exe) |
| Bank entries: every PC asset of a loaded file whose name holds `.win32.scd` and `se` / `music` (`0x1400E0A50` / `0x1400DB910` → `0x1400E0F00`); number = `atoi(name + 2)` (`0x1400DB1F0`). | | Checked (decomp) |
| At start (`0x140170630`, `0x140171410`) KH1 loads `amusic/se000.dat` (`0x1401791D0`) and registers its PC assets as bank 2 (`0x14017A080(2, …)` → `0x1400D30F0` → `0x1400DB910`). Its one PC asset is `se000.win32.scd` (owner's asset step, 0.39.0) → number `atoi("000.win32.scd")` = 0. So KH1's menu sound `n` = entry `n` of `se000.win32.scd`. | `kh1_first.hed` | Checked (decomp + the owner's step output) |
| KH1's per-sound SCDs (`xa_ex_0010.mdls`'s 21): `SEDBSSCF` v3, 1 sound, 1 stream, MS ADPCM mono ~48 kHz. `se000.win32.scd`'s layout (entry count, codecs): the step prints it; not seen here. | | Checked (files) |

**The owner's video** (KH1 with a KH2-combat mod; mono 44.1 kHz audio): 16 cursor moves (root
and Magic list) are one sound (normalised cross-correlation 0.64–0.86 with the first, through the
music; −37.9 dB RMS, ~−24 dB peak, 95–165 ms above −50 dB); opening Magic (5.93 → 6.00 s) is
another (0.37 with the move; −36.6 dB RMS, −25.6 dB peak: about as loud); confirming Firaga
(10.10 → 10.17 s, menu back on Attack) plays no menu sound, only the spell's own sounds from
10.18 s. No back / error in the video. So: move = id 1, open = id 6, no decide sound: what the
decomp says. Not compared with KH1's files (`se000.dat` is not in this session); the step now
writes the menu entries as WAVs (`kh1-menu-sounds-preview/`) for that comparison.

**Re:CoM** (Steam; the same SQEX "Ssl" sound code):

| what | where | tag |
|---|---|---|
| `0x1402AA360(id, vol)` → `0x1402AA0F0(id, (0,0,0,1), vol)`: bank = `0x1402A8560(id)` (group = `(id & 0x0FFFFFFF) / 1000`: 0 → 0, 1 → 1, 2–3 → 3, 100–199 → 2, 200–599 → itself, 600–799 → 0xC–0xF, 800–7999 → 4, else −1 = no sound); `0x1402F3080(bank, id % 1000, …)` makes sound `id % 1000` of the first entry of `0x140822370`'s list with that bank (`SslSeCreate2`); `0x1402AA3B0` → `0x1402A6DD0` → `0x1402F3B60` plays it. vol −1 → 127; `127 / 127.0` = 1.0 is the fifth argument of `0x1402F3080` (`movss [rsp+0x20]` before the call at `0x1402AA319`). | | Checked (decomp + exe disasm) |
| Banks are made by `0x1402F5D70(name, bank, data, tag)` → `0x1402F0080` (`Axa::BankEntry`, 0x50 bytes: `+8` next, `+0x10` bank, `+0x18` driver handle from `0x14004BD70(data, 0)`, `+0x20` data, `+0x1C` tag, `+0x30` name), pushed on `0x140822370`. Callers: a pack's `.seb` (`0x1402AB290` → `0x1402A79F0` → `0x1402F0780`: bank from the `.seb`'s `+8` through `0x1402A8210`, the PC SCD assets whose names hold the `.seb`'s stem) and `_add_scd_thread` (`0x1402F58C0`). Voices go to bank 0xFFFF (`0x1402F1250`). | | Checked (decomp) |
| A bank entry's release frees `+0x20` with `_aligned_free` (`0x1402F0610`); `0x1402F5BB0(bank)` releases a bank's entries before a pack loads it again. | | Checked (decomp) |
| Re:CoM's own KH1-style lookup by `+0x14` (`0x1402F32D0`, `SslSeCreate`) has no callers. | | Checked (decomp) |
| Re:CoM's create makes the sound with the same call as KH1's bank-2 path: `CreateSound(handle, id % 1000, 0, 1, 0)` (`0x14004C430` at `0x1402F314E`, `mov r8d,r14d`); `0x14004C430` and KH1's `0x14004DBF0` are the same function (SQEX `SoundController`, vtable `+0x68`). So entry `n` of a multi-entry SCD is played by index `n`. | `0x1402F3080` | Checked (decomp + exe) |
| Same SCD driver: `ScdHeader::GetScdHeaderSize` takes versions 1–4 in both (KH1 `0x1400748F0`, Re:CoM `0x140073130`). | | Checked (decomp) |
| `_aligned_malloc` import: `0x1405047A0` (`api-ms-win-crt-heap-l1-1-0`). | | Checked (exe) |

**What is built** (asset step + DLL; 0.39.0 took one SCD per sound and found none: fixed):

- The asset step (`kh1-recom-assets::menu_sounds`) reads KH1's `amusic/se000.dat` with its PC
  assets, takes the one KH1 numbers 0 (`se000.win32.scd`, KH1's bank 2), checks it is a whole
  SCD and that ids 1 / 3 / 4 / 6 are entries of it (sound count at `tables + 0`), and writes it
  **whole and unchanged** as `kh1-menu-sounds.bin`, with `kh1-menu-sounds.json` (format
  `kh1-recom-assets menu-sounds 2`: name, size, sha256, group 9000, the SCD's counts, per id
  its entry, Re:CoM id, the streams its sound plays and the first one's channels, rate, codec,
  bytes, samples, loop). It prints the SCD's counts and, per id, that stream's channels / rate /
  codec / length, and writes it as a WAV into `kh1-menu-sounds-preview/` (for listening; not
  part of the mod).
- **The sound table** (0.40.1; Checked (files), the owner's `se000.win32.scd` and 21 loose KH1
  SCDs): the table offsets after the counts are `tables + 0x08` tracks (193 command lists),
  `+ 0x0C` streams (223 headers), `+ 0x10` the 184 0x80-byte sound headers, `+ 0x18` another
  table, and `+ 0x20` (= `+ 0x24`) the sound → stream table: one entry per sound, in sound
  order, 16 bytes (byte 0 = pair count, `+0x0C` = the sound's number) then that many
  (u16 track, u16 stream) pairs. In se000 the pairs cover all 193 tracks once, and 174 of the
  193 tracks' wait (command 3, the length the command list waits before it ends: Re:CoM's
  `0x14008AE50` adds those waits) equals its stream's length to the millisecond; the other 19
  are loops or cut plays. So the menu ids play:

  | KH1 id | use | track | stream | stream | audible |
  |---|---|---|---|---|---|
  | 1 | move | 0 | 221 | stereo 48 kHz MS ADPCM, 1.284 s | ~0.1 s, then a quiet tail |
  | 3 | back | 2 | 220 | stereo, 1.626 s | ~0.15 s |
  | 4 | error | 3 | 219 | stereo, 1.843 s | ~0.25 s |
  | 6 | open | 5 | 218 | stereo, 1.774 s | ~0.1 s |

  Streams 0–3 and 5 are empty slots (codec −1); the PC port put these five sounds at the end
  (218–222). 0.39.x–0.40.0's printout took stream `n` for sound `n` (so move / back showed
  "empty", and error / open named the wrong streams); what Re:CoM plays never depended on it,
  since both drivers resolve the table themselves (same driver body, the SCD unchanged).
- **Whole asset:** the SCD's size word (15 277 472) is 1280 bytes short of the asset; stream 222
  (sound 2) ends at 15 278 750. 0.39.2 cut those bytes as padding (Assumption, wrong); 0.40.1
  keeps the asset whole, as KH1 loads it, and the DLL's head check takes a size word up to the
  pack's size.
- Whole file, not a trimmed SCD: no SCD writer to trust (the sound / track tables are not
  decoded), every entry keeps KH1's index, and ~10.9 MB once in Re:CoM's memory is small.
- The DLL (`menu_sounds`, `win/menu_sounds.rs`), when the command menu is installed and both
  files are there:
  - at start, checks the manifest against the pack (size, SCD head), reads the pack into a
    buffer from Re:CoM's own `_aligned_malloc` (checked equal to ucrtbase's export), and hooks
    `0x1402A8560` (13 bytes checked: `and ecx,0x0FFFFFFF; mov eax,0x10624DD3; mul ecx`): groups
    9000..9999 are their own bank; every other id goes to Re:CoM's map unchanged. Re:CoM gives
    those groups no bank today (−1), and never puts a bank there (its banks: 0–4, 0xC–0xF,
    200–599, 0xFFFF), so no Re:CoM sound changes. Checked (decomp).
  - With KH1's look (`hud_kh1_menu = on`), a menu sound plays `0x1402AA360(9000000 + n, −1)`
    (move 9000001, back 9000003, error 9000004, open 9000006). The first time, if bank 9000 is
    not in Re:CoM's list, it calls `0x1402F5D70("se000.win32.scd", 9000, buffer, size)`: Re:CoM's
    entry owns the buffer from there. It walks the list before each menu sound (not per frame);
    if the bank has gone (a sound-system reset), it reads the pack again and registers it again.
  - Re:CoM's ids (1 / 4 / 5 / 2) stay: with Re:CoM's look, without the asset files, for an id
    the SCD has no entry for, or when Re:CoM refuses the bank (logged once).
- Logs: `menu sounds: KH1's system bank se000.win32.scd (… bytes) as Re:CoM bank 9000 …, entries
  [1 → id 9000001, …]` at start; `menu sounds: KH1's se000.win32.scd (… bytes) registered as
  Re:CoM bank 9000 (driver handle …)` once.

**1:1:** KH1's own bank, its entries played by the index KH1 plays them by, at their own volume
(KH1 and Re:CoM both play at 1.0: Re:CoM's vol −1 → 127 / 127), as one-shots, at the same
frames KH1's menu plays them (the menu logic's emit points are KH1's, above). Checked: decomp
(both), exe bytes (both create calls), KH1 SCD layout on 21 real KH1 SCDs; the video's move /
open / no-decide pattern; the sound → stream table (0.40.1, files). Not checked: the sounds
against the video;
that Re:CoM's SE volume setting and mixer bus treat KH1's SCD as its own (Assumption); that
Re:CoM's 2D sound position (0, 0, 0) plays them centred like KH1's (Assumption); that Re:CoM's
driver takes a 10.9 MB bank (Assumption: KH1's own driver takes it). Not seen in game.

## KH1's words

The command record's `+4` message id names it (`0x14027A920` → `0x14028F9B0`); on PC the message
table is refilled from `btltbl.bin`'s PC asset `Word.bin` of the game's language (`0x14028F9D0`).
The asset step decodes the menu's messages from `US_ UK_ FR_ GR_ IT_ SP_Word.bin` into
`kh1-command-menu.json` (`docs/kh1-recom-assets.md` § The command menu). KH1's encoding: 0x01
space, 0x21.. digits, 0x2B.. A–Z, 0x45.. a–z, 0x63 '+', 0x6E '-', 0x71 ''', and the accented
letters the words use (0xDD Ö, 0xE4 á, 0xE9 é, 0xED í). Checked (files): every word, all six
languages. 0x6F (the blank command, message 305) is read as '-' (GR / SP write that entry with
0x6E): Assumption.

The DLL takes the language from `kh1-recom.ini` `menu_language` (us, uk, fr, gr, it, sp; default
us), missing ids from us. Re:CoM's `C2DFnt` takes ASCII (our Config rows, seen in game); its bytes
≥ 0x80 start a 2-byte code (`0x140258EB0`) whose table for accented letters isn't known, so
accented letters are written as their base letters (FR "Gravite", GR "Offnen"); the log names
them.

## Re:CoM's side

- **Plates:** `Cockpit:00` group 0x19 is Re:CoM's text plate (idle): a dark cap (sprite UV
  (229, 110)–(255, 135), part (−13, −13)…(13, 12)), a strip ((0, −13)…(152, 6), X translate 13)
  and a card-type icon. `CStockFolder` (`0x1401658F0`) shows these plates with a `C2DFnt` on
  them. The plate's texels are black (alpha-shaped), so a colour can't be multiplied onto them:
  the highlighted plate draws the strip again on the atlas's neutral texel with the magic
  plate's colour (group 0x1F's icon, vertex 0x80FFA200). Both groups are data the asset step
  adds (Checked (files)).
- **Rows:** `C2DSeqCtrl` (`operator new` `0x1404ACE54`, ctor `0x140247260`), priority `+0x16E` 2
  (the HP gauge's; the Magic list 3), `+0x138` = 1, layout position `0x140248660` at KH1's (x, y)
  + (13, 13) (the plate's cap centre, so its left / top edges are KH1's), one layer (vtable
  `+8`, `0x140247850`) at the gauge's x offset −`+0xA0` with flag 1, as the MP bar's. Text:
  `C2DFnt` (0xB0, ctor `0x140243F10(fnt, 0, 0, &colour, 0xE, text, 0x80)`), attached with vtable
  `+0x60` (`0x1402476F0`) at (0, 0), screen offset −4 − `+0xA0`: what `CStockFolder` passes
  (Checked (exe): `0x140165BEC..0x140165C4C`). Checked (decomp) unless tagged.
- **Colour:** `0x140248590(ctrl, &rgba)` writes the control's colour `+0x120` (floats, 1.0 = ×1),
  which Re:CoM's sprite update multiplies in when `+0x138` is set (`0x14024BB10`, with the
  parent's). Re:CoM's camp uses it. The menu's fade is its alpha; the text's is `C2DFnt +0x34`
  (0xAABBGGRR, 0x80 = ×1), as Re:CoM's camp changes its labels' colour. KH1's brightness bytes
  go there as they are: both games use 0x80 as ×1 (Assumption for the screen result).
- **Tab (0.35.0):** not a row. KH1's look: three `kh1hud` controls (the tab's pieces, the
  Triangle, the word) at KH1's places; Re:CoM's look: the highlighted plate tinted green
  (× (1, 1, 0.39): Re:CoM's blue `0x80FFA200` can't be multiplied to KH1's (110, 210, 100), the red
  is missing) with the word's `C2DFnt`, its top-left (8, 232) (28 above the first row, as KH1's
  tab sits over its header), no Triangle (no button glyph identified in Re:CoM's `Cockpit`). Both
  move (layout position) and fade (control colour, the font's colour) only while the tab
  slides or fades (14 frames). 7.3 drew it as row 3 with an X scale: a mis-port.
- **Context:** Re:CoM's field action (`0x1401E80C0`, Triangle `+0x424 & 0x200`, field only) acts
  on the object of handle `player+0x5FC` (when > 0) else `+0x84C`, by its type getter (vtable
  `+0x28`): 0x2000 treasure box (`CGimmickTreasureBox`, `…Pooh`; not when `+0x2C1 & 1`), 0x2002
  jump stand, 0x2003 carry (`CGimmickCarry`, `…Tu`), 0x4000 NPC (`ATEL::CAvatar`, the 100 Acre
  minigames), 0x1006 door (`CGimmickDoor`, room / world select). Checked (decomp + exe: RTTI,
  getters `mov eax, imm32; ret`, the door's computed at `0x1401B8A10`). The menu reads the same
  object without calls (`0x1402AD350` for the handle, the getter's bytes) and shows KH1's word:
  chest → Open, door → Open, carry → Lift, NPC → Talk, jump stand → Jump On (the words for Re:CoM's
  objects: Assumption). Battle: no context tab (KH1's battle specials are M9).
- **Save and warp points** (owner, 2026-11-01: "standing on a save point and using the purple
  teleporter thing dont prompt the triangle to interact"): not the field action's. Their own
  per-frame updates take Triangle straight from the pad and act; the menu reads the same
  conditions and shows the tab (`cm::recom::point`). Checked (decomp + exe) unless tagged:
  - `CGimmickSavePoint` (RTTI vtable `0x1405846C8`, type getter `0x1401C50B0` = 0x2004) and
    `CGimmickWarpPoint` (`0x14058F7E0`, getter `0x1401CC1E0` = 0x2005). `0x1401E80C0` has no
    case for either.
  - Save point update `0x1401C5180` (vtable slot `+0xA0`, the exe of the gimmick state at `+0x240`
    that `0x1401B2BF0` selects; that it runs every frame of the room: Assumption): while the main
    player (`[0x14087B380]`) is at a distance < 0.8 (`0x1401A32A0`, the float at `0x1405847D8`)
    and < 0.2 above it (`0x140509234`), the first frame heals him (`0x1401E59A0` with max HP
    `+0x430`), plays `rec_svpt.dpx` and sets **`+0x2C0` = 1**; leaving clears `+0x2C0`, the
    targetable bit `+0x154 & 0x10000` and calls `0x1401E6990`. In range, it reads the object of
    **`player+0x850`** (the lock-on handle, `0x1401E8DA0`) unless 0 or −1, else `+0x84C`; if that
    object's kind is 0x2004 and **Triangle is fresh** (`0x14029B550(0)` = Re:CoM's fresh pad
    bit 0x10, the same read as the menu's) and Re:CoM's action `+0x40C` is **1..4** (stand 2,
    walk 3, run 4), it opens the save menu (`0x1401B1650(1, 5, 1)`, `0x1401AC660`).
  - Warp point update `0x1401CC280`: makes itself targetable once story flag `0xC2 & 8` is set;
    unless `0x14087B84C & 1` (`0x1402345D0`), the same handle pick, kind 0x2005, fresh
    Triangle, action 1..4 → `0x1401B1650(1, 5, 1)` and a `CWorldWarp` (`0x140130ED0`,
    `wwarp.lad`: Re:CoM's world-warp menu). It has no distance test of its own: it acts whenever
    it is the lock-on / nearest target (`0x1401CDB90` picks the nearest object with
    `+0x154 & 0x10000`, no range).
  - The menu: when the field action's object gives no word, the object of `+0x850` (unless 0 /
    −1) else `+0x84C` (the same object, read once, on most frames): a save point with `+0x2C0`
    = 1 and the action 1..4 → **Save**; a warp point with `0x14087B84C & 1` clear and the action
    1..4 → **Examine**. Pure reads besides the handle lookup. The tab shows exactly when
    Triangle would save / warp (the "one save point per room" reading: Re:CoM's save point tests
    its own range and that the target is *a* save point; we read the target's range byte:
    Assumption).
  - Triangle: the save / warp point reads the pad, not the request bits, so the request-bits
    wrapper can't swallow it: saving and warping work exactly as before. The other side of
    that: they don't wait for the tab to slide in, and they still act with the Magic list open
    (KH1 runs the context command only from the root once the tab is in). Not 1:1; open
    question 5.
  - KH1: the save point's handler `0x1402AD0B0` puts command **0x36 Save** (message 293) in the
    context entry (`0x140293000(3, 0x36)`, `0x1402AD11F`: `mov edx,0x36; lea ecx,[rdx-0x33]`)
    and opens the save menu (`0x140196CD0`) when it runs. Checked (decomp + exe + files). KH1
    has no warp / teleport command: `Word.bin` has no Warp, Enter or Teleport (Checked
    (files): the 125 command records' names and every `US_Word.bin` string). The warp point's
    word is **Examine** (command 5, message 5: KH1's generic one; FR Examiner, GR Untersuchen, IT
    Esamina, SP Observar). Assumption, an owner choice (open question 6).
- **Inputs:** Re:CoM's pad (pure reads, as the camp's): fresh d-pad Up 0x1000 / Down 0x4000 /
  Left 0x8000 / Right 0x2000, Triangle 0x10, Square 0x80 (`0x14029B2F0`), confirm / cancel
  (`0x14029B490` / `0x14029B410`, Re:CoM's configurable buttons), only while Sora's pad is on
  (`player+0x230 & 1`). Checked (decomp); button names from bit order: Assumption.

### What changes in Re:CoM's request bits (KH1 setting, Sora's story, only while the menu is shown)

After Re:CoM's builder (`0x1401F8150`, already wrapped):

| bit | stays when | KH1 |
|---|---|---|
| `+0x424 & 4` Attack | the root list is on top with the cursor on row 0 (Attack, or Counter: the KH1 frame then plays the Counterattack, as before) and the cursor didn't use the press | confirm runs the highlighted row |
| `& 1` jump | the root list is on top | the jump only from the root; cancel closes a submenu |
| `& 0x200` Triangle | the root list is on top, and the context tab is in when one is shown | Triangle runs the context command once its tab is in |

Square (roll / guard) and lock-on are untouched. With no context tab shown, Re:CoM's own
Triangle action decides as before (an object the menu doesn't know keeps its Triangle). While
Re:CoM's HUD is hidden (events) the menu changes nothing. The save and warp points don't use
these bits (they read the pad, `0x14029B550`): nothing here changes them.

## Per frame

Nothing of ours runs per frame at Combat = Re:CoM, in Riku's story or with no menu built (one
atomic load in the request-bits wrapper). With the menu: KH1's step (pad reads, the build from
values already at hand: the movement state, the MP view the MP code stores on each read / write,
the spell levels, the context object, the target distance while a range-limited spell is
learned), then the rows: rebuilt only when what they show changed; colour writes only while the
fade (10 frames) or a brightness change (4 frames) runs; the tab's place and colour only while it
slides or fades (14 frames).

## Logs and trace

- Start: `command menu: KH1's command menu at combat = kh1 in Sora's story (M7 stage 7.3): words
  us (33 entries) from …; Re:CoM language index N; kh1_all_spells = off`.
- Up to 60 lines: `command menu: built on the player gauge 0x…`, `open magic (7 spells, grey
  0x0)`, `close`, `cast Firaga [0x9]: issued` (7.4), `not-now Fire [0x7]: KH1's gate refused it
  this frame (Sora busy)`, `spell Fire [0x7] greyed (Re:CoM's refused sound)`, `refused Items`,
  `context Open` (`context Save` / `context Examine` at a save / warp point, Triangle once the
  tab is in).
- F9 trace, events column: `…; menu r:<cursor> g<root grey> [m:<cursor> mg<spell grey>]
  [tab<state>:<cmd>] [counter] [events]`, or `menu -`.

## Sites (`sites::COMMAND_MENU_*`, bytes from the Steam exe)

| address | bytes | use |
|---|---|---|
| `0x140248590` C2DSeqCtrl colour | `0F 28 02 66 0F 7F 81 20 01 00 00 C3` | called |
| `0x1402476F0` C2DSeqCtrl add text (vtable `+0x60`) | `40 53 48 83 EC 30 48 8B 01 0F 29 74 24 20` | called through the slot |
| `0x1406073A0` vtable slot | → `0x1402476F0` | compared at install |
| `0x1401E80E8` field action pick: handles | `8B B1 FC 05 00 00 85 F6 7F 11 8B B1 4C 08 00 00 85 F6 7F 07` | checked only |
| `0x1401B8A10` door type getter | `48 83 EC 28 BA 24 00 00 00 8D 4A DD` | checked only (its address is compared) |
| `0x1401C50B0` save point type getter | `B8 04 20 00 00 C3` | checked only |
| `0x1401CC1E0` warp point type getter | `B8 05 20 00 00 C3` | checked only |
| `0x1401C52CC` save point update: in range (`+0x2C0` = 1), handle pick (`+0x850` / `+0x84C`), kind 0x2004 | `C6 87 C0 02 00 00 01 48 8B 15 A6 60 6B 00 8B 8A 50 08 00 00 8D 41 01 83 F8 01 77 06 8B 8A 4C 08 00 00 E8 5D 80 0E 00 48 85 C0 0F 84 80 00 00 00 48 8B 10 48 8B C8 FF 52 28 3D 04 20 00 00` | checked only |
| `0x1401C531E` save point update: action 1..4 | `44 8B 80 0C 04 00 00 41 FF C8 41 83 F8 03 77 4E` | checked only |
| `0x1401C5356` save point update: out of range (`+0x2C0` = 0) | `80 BF C0 02 00 00 01 75 1D 81 A7 54 01 00 00 FF FF FE FF 48 8B 0D 10 60 6B 00 E8 1B 16 02 00 C6 87 C0 02 00 00 00` | checked only |
| `0x1401CC2D7` warp point update: stop flag, handle pick, kind 0x2005 | `E8 F4 82 06 00 85 C0 0F 85 9D 00 00 00 48 8B 15 95 F0 6A 00 8B 8A 50 08 00 00 8D 41 01 83 F8 01 77 06 8B 8A 4C 08 00 00 E8 4C 10 0E 00 48 85 C0 74 78 48 8B 10 48 8B C8 FF 52 28 3D 05 20 00 00` | checked only |
| `0x1401CC32B` warp point update: action 1..4 | `8B 88 0C 04 00 00 FF C9 83 F9 03 77 49` | checked only |
| `0x1402345D0` warp point stop flag (`0x14087B84C & 1`) | `8B 05 76 72 64 00 83 E0 01 C3` | checked only (the flag is read) |

Already checked by earlier stages and called: `0x1404ACE54`, `0x140247260`, `0x140247850`,
`0x140248660`, `0x1402486D0`, `0x14015D230`, `0x140243F10`, `0x1402AA360`, `0x14029B2F0`,
`0x14029B410`, `0x14029B490`, `0x1402AD350`. No new hook, no byte patch.

## Rows and Re:CoM's 2D pool

Each shown row is one `C2DSeqCtrl` with two layers (the plate, the text), so two of Re:CoM's 2047
2D instance entries. A rebuild (a cursor move, a list opened or closed, a word changed) deletes
the shown rows first (`0x14015D230(ctrl, 1)`: both entries back, and the text layer deletes its
`C2DFnt`, as Re:CoM's `CStockFolder` labels go; we never delete the font ourselves) and makes the
new ones; colour-only changes rebuild nothing. Since 0.24.x the rows are made and deleted on the
player gauge's draw list (`win/d2.rs`, `docs/recom-m7-mp-bar.md` § 2D instances and draw lists):
a release from another list (the camp's, when Combat is flipped in the Config) left the entries
taken. The log's `2d: command menu: N row rebuilds: ours L layer(s) …` (every 100 rebuilds)
should show L and the pool's free count flat during menu use. Checked (decomp); in game: not yet.

## 1:1 checks

- Checked (decomp): the lists, the build rules, the cursor rules, the step order, the layout
  numbers, the brightness steps, the fade, the tab's state machine, the sounds' meaning. Tests
  (`command_menu::tests`) replay them: wrap, equality, Right / Left / cancel, the Attack row,
  the jump and Square frames, greying by MP (Fire / Blizzard at 30 charge units) and range, the
  14-frame tab and its Triangle gate, the 4-frame greying, the 10-frame fade, KH1's positions.
- Checked (files): the words in six languages; the plates in Re:CoM's base `FO0002.SPR`.
- Checked (exe): every new site's bytes and the vtable slot.
- Not checked: nothing seen in game; no KH1 recording of the menu (a KH1 video of opening Magic
  and of walking up to a chest would confirm the timing frame by frame).
- 0.35.0: the tab's draw (`0x140278B40`) Checked (decomp + exe + files), rendered offline from
  the bake; tests: the 4th row is never the context command, the cursor's confirm / Right there
  is the error sound, Triangle runs the context from any root row, the per-language Triangle
  offsets.
- Save / warp points: Checked (decomp + exe) Re:CoM's conditions (above) and KH1's Save (the
  save point handler's `0x140293000(3, 0x36)`); Checked (files) Save (293) and Examine (5) in
  all six languages (`owner_words` with the owner's `Word.bin`s). Tests: `recom_objects_give_kh1s_context_commands`
  (range, actions 1..4, the warp's stop flag), `save_point_shows_save_in_the_tab`,
  `sites::command_menu_sites` (the updates' rip targets: the main player, the handle lookup,
  `0x14087B84C`). Not checked: in game (the tab at a save point and at a warp point, and that
  Triangle still saves / warps); a KH1 recording of the save point's tab.

## Not 1:1 by design or yet

- The look is Re:CoM's plates (decision 34), not KH1's panels; the highlighted row is a colour,
  not KH1's cursor.
- Talk next to NPCs is KH1's since 0.178.3 (fork owner, 2026-11-23, answer 8b): Talk goes in row 0
  (instead of Attack), drawn as a dimmed "Attack" whose confirm gives the error sound; the tab
  shows Talk and Triangle talks. Until 0.178.2 it was entry 3's, with a normal Attack in row 0. A
  running special's follow-up keeps row 0 (Talk then stays entry 3's): Assumption, KH1's order
  between the two not read.
- Confirm on a locked Attack row (a state without flag 0x10, e.g. the glide) gives no error sound
  here (today's Attack path is left as it was).
- The Magic list's spells cast from 7.4 (Fire first; Blizzard … Aero do nothing until their
  stages); Cure / Aero's party list comes with their casts.
- KH1's slide-in of a newly opened list (`0x14027A0B0`'s "fresh" flag) is not reproduced: lists
  appear at once.
- Text without accents (Re:CoM's codes for them unknown).
- Save / warp points take Triangle from the pad themselves: they act while the tab is still
  sliding in and with the Magic list open (KH1: only from the root, once the tab is in). Open
  question 5.

## Assumptions

1. KH1's (x, y) is the row's top-left; Re:CoM's 512 × 384 layout space is KH1's 2D space; the
   HUD's −85 x offset matches KH1's widescreen margin. To check on a screenshot.
2. The text sits on the plate where `CStockFolder` puts it ((0, 0), −4); with the icon gone it may
   need a nudge right.
3. Re:CoM's sound ids: 1 cursor, 2 decide, 4 back, 5 refused (from how the camp uses them). Now
   only the fallback: with KH1's look the menu plays KH1's own sounds (§ Sounds).
4. Writing `C2DFnt +0x34` and the control colour each frame of a fade is safe (Re:CoM's camp
   writes label colours the same way).
5. Deleting our rows in the HUD-hide wrapper is safe (they are ours, not the gauge's).
6. (Gone 0.35.0: the 4th row is always drawn, as FM's.)
7. A new player gauge (room, battle) resets the stack to the root on Attack and fades in, as
   KH1's step starting again.
8. The context words for Re:CoM's objects (Open for doors, Jump On for jump stands, Examine for
   the warp point).
9. One save point per room: Re:CoM's save point tests its own range and that the target is a
   save point; the menu reads the target's range byte `+0x2C0`.
10. The gimmicks' updates (`0x1401C5180`, `0x1401CC280`) run every frame Sora is in their room
    (the exe of the state at `+0x240`, through the actor update `0x1401A59A0`'s state machine).

## Open questions (owner)

1. **Highlight colour:** (a) the magic plate's blue (built); (b) another of Re:CoM's plate
   colours (red / grey); (c) a value you pick from a screenshot.
2. **Talk next to NPCs** (answered 2026-11-23 by the fork owner: **(b)**, built in 0.178.3): (a) in the context tab (Attack stays Attack); (b) KH1's rule:
   Talk goes in row 0, which FM draws as a dimmed "Attack" whose confirm gives the error sound;
   the tab shows Talk and Triangle talks.
3. **Menu language:** (a) the ini's `menu_language` (built, default us); (b) follow Re:CoM's own
   language automatically (needs one log line from each language: the `Re:CoM language index`).
4. **Accented letters:** (a) base letters (built); (b) find Re:CoM's codes for them (needs Re:CoM's
   message files of a European language at the asset step).
5. **Triangle at save / warp points:** (a) Re:CoM's own, untouched (built): Triangle saves /
   warps at once, even while the tab slides in or the Magic list is open; (b) KH1's gate: a
   wrapper on the Triangle read the two updates share (`0x14029B550`, also called by
   `0x140142A30`) answers "not pressed" until the tab is in and the root list is on top.
6. **The warp point's word** (KH1 has no warp command): (a) Examine (built; KH1's generic word,
   message 5); (b) Touch (354; GR "Berühren": its ü, byte 0xF8, is not decoded yet); (c) Get In (362; FR
   "Monter", IT "Sali", SP "Subir"); (d) Move (348; but FR "Pousser" = push); (e) Save (293),
   KH1's save point being where KH1 goes to the world map.

## What the first in-game test must check

1. Log: the start line (words, language index), `mp bar: … groups 151..158` with no "has N
   groups" line, then on entering a room `command menu: built on the player gauge …`.
2. **Screenshot** in the field: four rows bottom-left (Attack highlighted in blue, a dash row
   greyed, Items greyed; the 4th row a greyed dash), KH1's words, no overlap with the HP / MP
   bars. Another one with `kh1_all_spells = on` and the Magic list open (7 spells, Firaga …
   Aeroga, x + 11, bottom at 348).
3. Up / Down move the highlight (with Re:CoM's cursor sound), wrap through row 3; two d-pad
   presses at once move nothing; the stick doesn't move it.
4. Confirm on Attack swings as before (combos, air combos, Counterattack after a guard: row 0
   reads "Counter" during the recoil).
5. Cursor on Magic without spells: confirm buzzes. With `kh1_all_spells = on`: Right or confirm
   opens the list; Left closes it (cursor stays on Magic); cancel closes it (cursor back to
   Attack, Sora doesn't jump); a spell: cast (7.4; Fire only so far), the menu back on Attack.
   With MP 0 the spells grey (over 4 frames), Fire / Blizzard come back at 30 charge.
6. In the field next to a chest / NPC / door / box to carry: the green tab with the Triangle
   slides in over the header (about a quarter second) with Open / Talk / Open / Lift; Triangle
   works once it is in, not before; the 4th row stays "–". In battle: no tab. On a save point:
   the tab says Save (once standing on it; it goes when stepping off, or while rolling /
   jumping), Triangle opens Re:CoM's save menu as before. At the warp point: the tab says
   Examine wherever Triangle would open the warp menu, and Triangle opens it as before.
7. Cutscenes / events: the menu disappears with the HP gauge and comes back with it, on Attack.
8. Combat = Re:CoM in the Config: the menu goes when the camp closes; back to KH1: it comes back.
   Riku's story: no menu.
9. F9 trace: the `menu r:0 ge` token (no spells, no context: rows 1–3 grey), and its events when
   moving / opening.

## Stage 7.4: confirm on a spell casts it

`0x140293480` for a spell row (Checked (decomp)): locked (greyed) → the error sound; else KH1's
commit gate (free or in a combo window, not in the take-off; Fire also from frame 14 of its
release motion) → refused: no sound, the list stays open; taken: the command is issued
(`win::magic::issue`), the stack unwinds and the root cursor returns to Attack. The gate is read
from Sora's KH1 frame as last left (one motion update stale; the frame checks again exactly) and
only while Re:CoM's action is one KH1's frame runs. `recom-m7-magic.md`.

## Stage 7.12: KH1's shortcut menu (L1)

Owner (2026-11-01): "L1 like KH1. Take KH1's behaviour for the rest": Cure / Aero from a shortcut
cast on Sora at once (as the Magic list); slots filled by KH1's learn rule, older saves once from
the spells already learned (KH1's spell order, first three); Customize is 7.13 (built:
`recom-keyblade-equip.md` § Stage 7.13; it writes the same three bytes); at a save / warp
point with L1 held, KH1 casts only (no save menu); the panel is KH1's art in both menu looks
(`hud_kh1_menu` on or off); keyboard: whatever key Re:CoM maps to its L1 bit.

### KH1 (Steam)

| what | where | tag |
|---|---|---|
| Slots: save `+0x844..0x846`, spell 0..6 or 0xFF none; spells only, cast at the spell's current level (`level + (spell + 2) × 3`). Get `0x140285090`, set `0x1402857C0`, new game all 0xFF `0x1402852C0`. | | Checked (decomp) |
| Learn `0x140285260` (the script command `0x1401AF7C0`): sets the learned bit, then puts the spell in the first slot holding 0xFF, if any (no duplicate check). Level rises (-ra / -ga) don't go through it. | `0x140285260` | Checked (decomp) |
| Build (`0x1402944C0`, with the Magic list, only when a spell is learned and the save's `+0x874` bits 3 / 0 (no magic / sealed) are clear): entry k = the slot's spell command, or the blank 0x42; grey = lock = all but the slots whose spell is usable this frame (the Magic list's test: the state's flag 0x20, MP, range for flag-8 spells); `0x142D53A5C` = 1 when a slot is set. | `0x1402944C0` | Checked (decomp) |
| Swap (`0x1402937F0`, before its "Sora takes input" checks): the stack's base is the root and `0x142D53A5C` and L1 held (`0x14298CA94 & 0x40000`) → the base becomes the shortcut list (type 5, `0x140528A18`) and the stack above it is cleared (an open submenu closes). Base is the shortcut list and L1 released or `0x142D53A5C` 0 → the root, root cursor 0 (Attack). No sound. `0x140297950` (Sora `+0x444 & 1`) clears `0x142D53A5C` first. | `0x1402937F0` | Checked (decomp) |
| With type 5 on top: the root's Triangle (context) / Square (roll / guard: `0x142D53A40 \|= 0x4000000`) block is skipped; the base list's jump check still runs (a fresh jump outside states 8 / 0x18 / 0x19 / 0x21 is the jump and ends the step); the cursor `0x1402931B0` returns at once (the list's cursor is −1), so the d-pad does nothing; then, with no command picked, fresh `0x80000` (Triangle) else `0x100000` (Square) else `0x200000` (Cross) runs entry 0 / 1 / 2 through `0x140293480`. | `0x1402937F0`, `0x1402931B0` | Checked (decomp) |
| `0x140293480` on a slot: locked → sound 4 (the step set `0x142D53A40 = 0x2000000`); a command with a submenu opens it (Cure / Aero: the party list 4; not ours: owner, cast on Sora); KH1's commit gate (`0x1402A2EC0` free or combo window, else Fire from frame 14 of motions 0x3D / 0x8A) refuses silently (the press is gone, not buffered); else the command is written and returns 1, no sound. Works in the air (the gate decides). | `0x140293480` | Checked (decomp) |
| Superglide / roll / guard read `0x142D53A40 & 0x4000000` (`0x1402A7B30`), so none starts while the shortcut list is up; the glide's hold reads the pad itself (`0x1402B2BF0`). | | Checked (decomp) |
| Draw `0x14027B870` case 5 at (left margin + 16, 0x11A): `uibin_cut` objects 0 (`cut_fra_a`, picture 0: sprite def `0x1427F1460`, which `0x140266580` builds from `uitex.bin` offset 0) and 1 (`cut_shortcut`, picture 1: def `0x1427F1430`, offset 0x2080) through `0x14027A9F0`; glyphs 0x46, 0x45, 0x41 through `0x140279FC0` (size 14, RGB × 1, alpha the fade) at objects 2..4's places (91, 23 / 45 / 67); words through `0x14027A920` at the frame's place (0, 0) + (16, 2) + (0, 20 + 22 k), 0x73 when the grey bit is set, else 0xE6 (set from the bits, not the eased bytes). The root, its header and the context tab are not drawn. | `0x14027B870` | Checked (decomp + files) |
| The `cut` objects are copied once (`0x140278180` → `0x14027E1C0(&0x140527B58)`); nothing starts their keys (`0x14027E710` is called in the draw only for the title layouts `0x142859390` / `0x1428592D0`, Checked (exe)), so the file's 2-frame alpha-in never plays: they are drawn static, × the menu's fade. (The research note's "each piece a 2-frame alpha-in" is the file's data, not what KH1 draws.) | | Checked (decomp + exe) |
| The context tab's step `0x140277CF0` is called only from the root's draw (`0x140278B40`): while the shortcut list is up it does not step. | | Checked (decomp) |
| Glyph 0x41 = the confirm button (`0x1402D4A50`: 3 without the confirm swap, 0 with it; `0x1403EFF90`'s PlayStation column: 0x43 → 0, 0x44 → 3, 0x45 → 2, 0x46 → 1). | | Checked (decomp + exe); 0 Circle / 2 Square / 3 Cross: Assumption (1 Triangle is the tab's) |

### Re:CoM side (built)

- **Slots:** our save block `save + 0x2681..0x2683` (`crate::mp::off::SHORTCUTS`), 0 = none,
  else spell + 1; spare byte `save + 0x2684` bit 1 (`crate::unlocks::SHORTCUTS_SET`) = set up.
  The unlocks' sync (`win/unlocks.rs`, the first-obtained wrapper and the first read at the KH1
  setting) fills them: a save never set up first gets the spells it already had in KH1's order
  (first three), then every spell learned (level 0 → more) takes the first empty slot
  (`crate::unlocks::shortcuts`). `win::mp::spell_state` reads levels and slots in one read and
  runs the sync once when either spare bit is missing. `kh1_all_spells = on` (debug) shows Fire /
  Blizzard / Thunder while the save has no slot set (not written).
- **Logic** (`crate::command_menu`, tested): `World.shortcuts`, `Pad.l1` (held), the shortcut list
  built with the Magic list, the swap, `step_shortcut` (jump first; Triangle > Square > confirm;
  greyed → error sound; gate refused → nothing; else `Outcome.cast`), the brightness reset when
  the root comes back, the tab frozen while the list is up, `Scene.shortcut`.
- **Input:** L1 held = Re:CoM's `0x14029C1D0(4, 0)` (pad bit 4: L1 by bit order, Assumption; the
  card input `0x1401FEFE0` reads it; at the KH1 setting the card input doesn't run, so L1 is free),
  read even while Sora's pad is off (KH1 swaps before its input test). Triangle / Square / confirm
  are the menu's fresh reads (confirm = Re:CoM's configurable confirm). Keyboard: whatever key
  Re:CoM's key config maps to its L1 bit (Assumption: not read).
- **Request bits** (the request-bits wrapper): while the list is up Attack (`& 4`), Triangle
  (`& 0x200`) and Square (`& 2`: roll, guard, Superglide start) are dropped; the jump stays.
- **Cast:** `Outcome.cast` → `win::magic::issue(spell, level)`, as the Magic list (Cure / Aero on
  Sora at once).
- **Panel** (`win/command_menu.rs` `show_short`), in both looks: `kh1hud` controls at KH1's places
  (`crate::command_menu::shortcut`): the panel group at (16, 282) with the left margin, the glyphs
  at (107, 305 / 327 / 349) + the glyph drop (−37/16), the words (the menu's word groups) at
  (32, 304 + 22 k); colours (1, 1, 1, fade), words (b, b, b, fade) with b = brightness / 255. The
  third glyph follows Re:CoM's confirm key (`0x14083F3F0`, read by `0x140346BB0`): 0x2000 →
  Circle, else Cross. Made when the list comes up (or a slot's word changes), colours written
  only when the fade or a slot's greying changes; deleted when the list goes (the root's rows are
  remade). Needs `kh1-hud.json`'s `"shortcut"` and `kh1hud:00` registered (checked when the panel
  is made); without them the shortcuts cast with no panel (logged once).
- **Save / warp points** (owner: "KH1 casts only"): their updates read Triangle from the pad
  (`0x14029B550(0)`). The two calls (`0x1401C530E`, `0x1401CC31B`) are retargeted through one jump
  near them to `point_triangle`: when the menu ran this frame with a slot set
  (`SHORT_READY`), MP is live, and L1 is held, it returns 0 (not pressed); else Re:CoM's read.
  So with L1 held Triangle casts slot 1 and the save / world-warp menu doesn't open; without L1
  (or with no slot set) nothing changes. `0x14029B550`'s third caller (`0x140142A30`) is not
  touched.

### Sites (stage 7.12)

| address | bytes | use |
|---|---|---|
| `0x1401C530E` save point Triangle call | `E8 3D 62 0D 00` | retargeted (call → our jump → `point_triangle`) |
| `0x1401CC31B` warp point Triangle call | `E8 30 F2 0C 00` | retargeted |
| `0x1401C530A` its context | `75 70 33 C9 E8 3D 62 0D 00 85 C0 74 65` | checked only |
| `0x1401CC317` its context | `75 68 33 C9 E8 30 F2 0C 00 85 C0 74 5D` | checked only |
| `0x14029B550` the Triangle read (whole) | `48 83 EC 28 8B D1 B9 10 00 00 00 E8 E0 0C 00 00 33 C9 85 C0 0F 95 C1 8B C1 48 83 C4 28 C3` | called by `point_triangle` |
| `0x140346BB0` confirm key getter | `0F B7 05 39 88 4F 00 C3` | checked only (`0x14083F3F0` is read) |

`0x14029C1D0` (held buttons) is the movement's checked site. Every byte set checked against
`recomsteam.exe`.

### Per frame

The menu step already runs each frame; the shortcut list adds a held-pad read, the slots from the
block read the menu already makes, and one atomic store. While the list is up: no 2D work but
colour writes on a fade or greying change. `point_triangle` runs only when a save / warp point's
update reaches its Triangle read (Sora targets it).

### Logs and trace

- Start: `command menu: KH1's shortcut list on L1 (M7 stage 7.12): slots at save+0x2681; the save
  point's Triangle read 0x1401C530E retargeted, the warp point's 0x1401CC31B retargeted`.
- Unlocks: `unlocks: shortcut 1 = Fire (KH1's learn: the first empty slot)` / `(set up once from
  the spells this save had)`.
- Menu events (log and F9 trace): `shortcuts on (L1)`, `shortcuts off`, `shortcut 1 Fira cast`,
  `shortcut 3 - greyed`, `shortcut 2 Cure not-now`; trace token `menu sc g<grey bits>` while up.
- Once: `command menu: save / warp point: Triangle with L1 held is shortcut 1 (KH1), not the
  point's menu`; `… no shortcut panel (run this version's asset step) …`.

### 1:1

Checked (decomp): the swap rule and its timing (same frame, no sound, submenu dropped, root back
on Attack), the slot commands and greying, the button order and single press, the error sound,
the silent gate refusal, no sound on a cast, no roll / guard / context / Attack while up, the jump
taking the frame, the panel's pieces, places, pictures and brightness, the frozen tab, the learn
rule. Checked (files + offline render): the panel's art and layout (`builds_from_a_dump`, render
`/tmp/sc/shortcut_panel.png` on the session machine). Not checked: in game; a KH1 trace or video
of the shortcut list.

Not 1:1 / Assumptions:
- Cure / Aero cast on Sora at once (owner): KH1 opens the party list (`0x140293480`).
- KH1's jump check exempts states 8 / 0x18 / 0x19 / 0x21 (flight and others Re:CoM has no use for
  yet); ours, as the root's, always lets a fresh jump take the frame.
- `0x140297950` (Sora `+0x444 & 1`, a KH1 no-command state) is not modelled.
- L1 = Re:CoM's pad bit 4 and Circle = bit 0x20 (bit order); the keyboard key Re:CoM maps to L1;
  the PC's glyph for 0x41 / 0x45 (by `FontIcon.inf`'s PlayStation names).
- At a save / warp point: Triangle there still acts without waiting for the tab (7.3's known gap);
  with L1 held it is shortcut 1 only.
- An empty slot is drawn with the blank command's dash, greyed (KH1's 0x42).

## Gimmick cards: "Stagger" in the context tab

A held Re:CoM gimmick card (`recom-gimmick-cards.md`) puts our command `0xF0` ("Stagger", word id
901, ours) in the context entry. Order: an object's command → Stagger → KH1's special offer (Sonic
Blade), `crate::gimmick::context_entry`. Triangle on it is ours (`Outcome::gimmick`), not Re:CoM's
Triangle action; confirm on row 3 never runs it (as every context command).
