# Story skips at Combat = KH1: the intro tutorial and Leon's training battle

Ask (owner, 2026-11-01): "If a player has KH1 Combat enabled, have the player skip the leon training
battle. Also the intro tutorial."

Status: **built (option (a), owner 2026-11-01), not seen in game.** Both tutorials are battles that
event scripts start by battle-room number, and neither script has an "already done" branch (§ 2).
The owner chose option (a) of § 4: skip the battle room and run its own exit. See § 6 for what was
built.

Tags: Checked (exe / decomp / files) or Assumption. Steam addresses. "Files" means the owner's
`WORLD00` / `WORLD01` `.ABC` / `.CTD` / `.ESD` files, read in the session only. Nothing from them is in
the repo.

## 1. Event scripts (exe and decomp)

- A room's event script is an `.ABC` file. Its container is the archive layout `SY0001.BIN` uses
  (`{u32 offset, name[0x18], u32 size | 0x80000000}`), holding one `/abc` program. The loader
  `0x14014B8A0` (first bytes `48 89 5C 24 08 57 48 83 EC 20 81 3A 2F 61 62 63`) checks the magic
  `0x6362612F`. Program layout: entry count, 64 entry ints, instruction count, 8-byte instructions,
  data size and bytes, thread count, 0x1C0-byte thread blocks. Checked (decomp + files).
- `CAtel` (`0x1401AD7B0`) loads the room's script: `0x1402A46F0` → `0x140135010` → VM `0x140141830` →
  `0x14014B8A0`. It is stepped by `0x140143290`. Checked (decomp).
- Instruction: `{u16 op, i16 frame-relative flag, i32 operand}`. `0x140143320` dispatches:
  - below 0x3E to `0x14014A2F0`: stack and flow. 0x1C push, 0x06 add immediate, 0x1A / 0x22 load /
    store, 0x20 pop, 0x29 jmp, 0x2C / 0x2D jz / jnz, 0x30 / 0x31 call / ret, 0x39 end;
  - 0x3E..0x63 to `0x1401436F0`. Its variable ops are 0x53..0x58 (set 0 int, byte and bit get / set)
    and 0x5F..0x63 (set 1 int / byte set and get, bit get);
  - 0x6E..0xA7 to `0x140144840`. 0x6E is the **set-1 bit write** (`0x1401B1650(1, id, v)`);
  - 0xA8..0x112 to the syscall switch `0x140145C50`.

  Checked (decomp; ops decoded against the files).
- Syscalls used here (`0x140145C50`), Checked (decomp):
  - **0xEE(room)**: battle entry to battle room `room` in the current world (`0x140236690(room)`).
    It also sets (1, 5) when the current room is below 2000.
  - **0xF7(a, b, room)**: room change `0x1402360F0(a, b, room)`; b = −1 → `0x140233AE0()`.
  - 0xFE(x, op, y): bit maths (or / and / xor / shifts / and-not).
  - 0xAD(sub, a, b): sub 5 gives a card, 7 shows a card-get screen, 0x10 makes `CAtelTutorial`,
    0x1E `0x14023C260(a, b)`; sub 0xC has no case (does nothing).
- Only save-variable set 0 is saved; set 1 is session-only (research note `recom-cards-save.md` §4,
  kept with the decomp tools). Checked (decomp).

## 2. Where the two tutorials are (files)

Sora's opening in Traverse Town (world 1) is a chain of event rooms (2000+) and event battle rooms
(1050..1198, which the battle manager `0x1401871D0` doesn't end on its own). Checked (files + decomp):

| Step | Room (world 1) | Script does |
|---|---|---|
| Opening scene (text `CTtr00`) | event 2000 | sets friend bits (0, 0x22) ∣= 1 ∣ 2 (Donald, Goofy), then **`0xEE(1100)`** at the end of the scene and at the end of a second path (Assumption: the scene-skip path) |
| **Intro tutorial** (hooded figure, text `CTtr01`) | battle 1100 | see below |
| Scene after it (text `CTtr02`) | event 2001 | |
| Scene with Leon (text `CTtr02`) | event 2002 | **`0xEE(1101)`** at the end of the scene and at the end of a second path (Assumption: the scene-skip path) |
| **Leon's training battle** (text `CTtr03`) | battle 1101 | see below |
| Scene after it (text `CTtr04`) | event 2003 | gives the floor's cards and the card-get screens (0xAD 5 / 7) |

- The intro tutorial's battle room 1100 (`tr_1100.abc`):
  - At its start it sets the tutorial session bits (1, 0x2C) (tutorial deck), (1, 0xE1) = 0 and
    (1, 0xE2) = −1, makes `CAtelTutorial`, and clears friend bits (0, 0x22) &= ~1, ~2.
  - Then it runs the lesson.
  - **Lesson exit**: session bits cleared, friend bits set again (∣= 1 ∣ 2), **`0xF7(0, 0, 2001)`**.
  - **Won exit** (bit (1, 2), set by the battle manager's `0x140182240`): session bits cleared,
    `0xF7(0, 0, 2001)`. This exit does not set the friend bits again.
- Leon's battle room 1101:
  - At its start it sets (1, 0x2C), (1, 0x2F), (1, 0xA) and (1, 0xE2) = −1.
  - Then it runs the lesson: card values, card break, stocking.
  - **Lesson exit**: session bits cleared, (1, 0x32) = 1 (`0x1401AF9D0` and `0x1401ADD20` then keep
    the music playing into the next room), **`0xF7(1, −1, 2003)`**.
  - **Won exit** ((1, 2)): the same without (1, 0x32).
- Neither battle script gives a card, item or reward (no 0xAD 5 / 7 / 0x13 / 0x15), and neither
  writes a set-0 variable other than the friend bits (0, 0x22). Checked (files).
- (0, 0x22) is the friends-met bitmask that Re:CoM's Friend-card drop roll `0x1401602A0` reads
  (bit 1 Donald, 2 Goofy, 4..0x40 others). Checked (decomp + exe scan).
- **No "already done" branch:** event 2000 and event 2002 always end in `0xEE`, with no test before
  it. The battle scripts always run their lesson. The chain moves on only by the battle script's own
  `0xF7`. Checked (files).
- Which exit a normal play takes (lesson or won): Assumption, the lesson exit. Whether winning
  either battle by KO gives EXP: Assumption, no (not read).
- Riku's story has its own tutorial battle 1100 in world 0, started by `WORLD00` event 2102 (his first Ansem fight, a
  Rapid Break and Duel lesson). Fork branch, 0.196.0 (`AGENTS.md` Stage 374): at Combat = KH1 with Riku's KH1 combat the
  battle room loads and the mod does its lesson's won-path writes (saved Rapid Break / Duel bits, the 9 world cards) and
  sets its script's "done" bit (1, 2) so the script leaves through its own exit back to 2102. Research:
  the workspace's `knowledge/ReCoM/recom-riku-ansem-tutorial.md`.
- Kept as they are (not battle tutorials): Traverse Town's field help screens (`CTtr11` field
  actions in room 1, `CTtr12` doors, `CTtr13` save point, `CTtr14` Moogle shop). Also kept: every
  cutscene, with Re:CoM's own scene skip.

## 3. What a played tutorial leaves (for saves)

- Set 0 (saved): friend bits (0, 0x22) end the same as after event 2000 (1 ∣ 2 set) on the lesson
  exits. Nothing else. Checked (files).
- Set 1 (session): all cleared on exit. The only exception is (1, 0x32) = 1 after Leon (music), which
  is cleared by the next room's load. Assumption: cleared at the next room start (its reader
  `0x1401AF9D0`, not traced).
- The deck is untouched: the tutorial deck is a fixed table (`0x1407C2450`, used while (1, 0x2C) is
  set), and since Stage 29 the revolver is empty at KH1 anyway. Checked (decomp).
- So a save made after a skip that ends in the same room with (0, 0x22) ∣= 1 ∣ 2 is the same as a
  save after playing the tutorials. Switching Combat back to Re:CoM later is safe: the floor's cards
  come from event 2003, which still plays.

## 4. Fallback options (owner chose (a), 2026-11-01)

Each acts only at Combat = KH1, in Sora's story, in world 1, for battle rooms 1100 and 1101.
Re:CoM's setting and Riku's story are unchanged.

- **(a) Skip the battle room entirely.** A wrapper on Re:CoM's battle-entry request `0x140236690`
  (first bytes `48 89 5C 24 08 57 48 83 EC 20 0F B7 F9`; called with a room by script 0xEE; the
  other callers pass 0 or 1030 (`0x1401CA750`, a gimmick); Checked (decomp)). For room 1100 it
  calls instead Re:CoM's own `0x1402360F0(0, 0, 2001)` and sets the friend bits as the lesson exit
  does. For 1101 it calls `0x1402360F0(1, 0x140233AE0(), 2003)` with
  (1, 0x32) = 1.
  - The arena never loads: the opening scene fades straight into the next scene.
  - Runs once per battle request, nothing per frame.
  - Risk: event-to-event moves by `0xF7` aren't used anywhere in vanilla (only battle → event and
    event → field). `0x1402360F0`'s own entry test allows it from an event room (Checked (decomp)),
    but it isn't seen in game. If it misbehaves, (b) is the fallback.
- **(b) Let the battle room load and take its own exit at once.** After Re:CoM's battle manager
  constructor (`0x1401859B0`, already wrapped by M6), in rooms 1100 / 1101 Re:CoM's own "battle won"
  `0x140182240` sets (1, 2). The battle script's won exit then runs its own `0xF7`.
  - For 1100 we also set the friend bits that the won exit doesn't restore.
  - Only vanilla transitions are used.
  - Cost: a battle load, a fade and maybe a frame of the first tutorial line.
- **(c) Patch the two event scripts' bytecode at load** (`0x14014B8A0`): replace the `0xEE`
  sequence with the exit's `0xF7` sequence. It acts like (a), but as data. Each patch would be
  checked against the expected instructions (an unknown file version stays unpatched). More
  fragile (instruction offsets, jump targets). Not recommended.

Recommendation: (a). It is cleanest for the player and native (Re:CoM's own room change, with the
battle script's own arguments). Fall back to (b) if the in-game test shows a problem.

## 5. In-game checks

A new game at Combat = KH1:
1. After the opening scene, the hooded-figure battle does not start, and the next scene plays.
2. After the Leon scene, his battle does not start, and the next scene plays and gives the cards.
3. Donald and Goofy are back afterwards: their Friend cards drop again at Combat = Re:CoM.
4. Save, then load at Combat = Re:CoM: the game plays on.
5. Skipping either scene with Re:CoM's own scene skip gives the same result (the skip path asks for
   the same battle room).
6. The log has the start line, then one `story skip: … skipped at Combat = KH1 … -> 1` line for each
   tutorial.

Also check at Combat = Re:CoM that both tutorials play as before, and that the log says
`… Re:CoM's battle (Combat = Re:CoM, Sora's story)`. If a scene freezes instead of moving on, send
the log: it names the room change's answer.

## 6. Built (option (a))

Code: `crates/kh1-recom/src/story_skip.rs` (pure: which rooms, which writes, which room change;
unit tests) and `crates/kh1-recom/src/win/story_skip.rs` (the wrapper). Sites:
`sites::STORY_SKIP_HOOKS`, `sites::STORY_SKIP_CALLEES`.

- **Hook:** Re:CoM's battle-entry request `0x140236690`. The wrapper moves its first 10 bytes
  `48 89 5C 24 08 57 48 83 EC 20` (`mov [rsp+8],rbx; push rdi; sub rsp,0x20`). The next instruction,
  `movzx edi,cx`, stays. Checked (exe): no branch lands inside the moved bytes; the only other
  reference is the `.pdata` entry. Callers (Checked (exe)): script syscall 0xEE at `0x1401490D6`
  (the room), case 10 `0x1401460D6`, the gimmicks `0x1401B75C8` / `0x1401B763F` / `0x1401B7787` /
  `0x1401B7B02` / `0x1401B7BD7` and the tail jump at `0x140237020` (0), and `0x1401CA9D4` (1030).
- **When it acts** (`story_skip::plan`): Combat = KH1 (the movement handlers' setting), Sora's story
  (`0x1401AAAE0`), map world code `save + 0x1878` = 1 (Traverse Town), and the pair (asking room
  `save + 0x187A`, battle room) is (2000, 1100) or (2002, 1101). Anything else goes to Re:CoM's own
  function unchanged; for any other room the wrapper only compares one number.
- **What it does** (the battle script's lesson exit, in its order; § 2):
  - Intro (1100): session bits (1, 0x0), (1, 3..6), (1, 8..0xC), (1, 0x23), (1, 0x2C) cleared and
    the HUD mask (1, 0xE2) = 0, through `0x1401B1650` / `0x1401B16C0`. The friends-met bits are set:
    `(0, 0x22) |= 1`, then `|= 2`, through `0x1401B08C0` / `0x1401B16C0`. Then
    `0x1402360F0(0, 0, 2001)`.
  - Leon (1101): (1, 0x2C), (1, 0xA), (1, 0x2F), (1, 0x30) cleared, (1, 0x32) = 1 (music), (1, 0x0)
    and (1, 3..6) cleared, the HUD mask (1, 0xE2) = 0. Then `0x1402360F0(1, 0x140233AE0(), 2003)`.
    The script's door −1 becomes `0x140233AE0()`, as syscall 0xF7 does.
  - Returns 1, as the request does when it is accepted. The script that asked has already ended
    its thread after the syscall (`end`), as it does after a real battle request. Checked (files).
- **Fallbacks:** if the floor slot `0x14087CBE2[kind]` the room change reads is negative (a null
  floor in `0x140237850`), or the room change returns 0, Re:CoM's battle entry runs as asked and the
  tutorial plays. Writes done by then are the exit's own values (cleared session bits, friend bits
  that event 2000 had already set). Checked (decomp).
- **Not copied from the script:**
  - script op 0x6E also calls `0x1402E75D0(v == 0, 0)` (a device field) when it writes bit 4. The
    skip writes the bit only. The last call before it, event 2000's own (1, 4) = 0, already passed
    the same argument. Checked (files + decomp).
  - 0xAD sub 0xC has no case in the Steam build (does nothing). Checked (decomp).
- **Per frame:** nothing. Once per battle request.
- **Log:** at start, `story skip: wrapper on Re:CoM's battle-entry request 0x140236690; …`. Each
  skip: `story skip: <name> skipped at Combat = KH1: battle room <n> (asked from room <r>) not
  entered; its lesson exit's <k> writes done (friends-met (0, 0x22) = 0x…), Re:CoM's room change
  0x1402360F0(kind, door, room) -> 1`. At Combat = Re:CoM or in Riku's story: one line saying
  Re:CoM's battle runs. A refused room change or bad floor slot is logged too.
- **Saves:** the skip writes only what the lesson exit writes. Its one saved value, (0, 0x22),
  ends as after a played tutorial. A save made after the skip is valid at Combat = Re:CoM. Checked
  (files + decomp); not seen in game.
- **Checked:** unit tests (rooms, arguments, the writes against the scripts, the floor guard, site
  bytes), `cargo test --workspace`, the Windows build (no warnings), all literal site byte lists and
  pointer sites against the Steam exe with pefile (450 + 13, 0 mismatches).
- **Assumptions:**
  - an event-room-to-event-room move through `0x1402360F0` works as the battle-room-to-event-room
    move does. Its entry test allows it (Checked (decomp)), but vanilla never does it;
  - the floor state is the same in event 2000 / 2002 as in battle 1100 / 1101 (the battle entry
    doesn't change it);
  - the lesson exit is the one a normal play takes;
  - Tutorial Leon gives no EXP.
- **Not seen in game.**
