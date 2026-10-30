# AGENTS.md: KH1 combat in Re:Chain of Memories

Read first, every session:

1. `docs/design-philosophy-instructions.md`
2. `docs/engineering-rules.md`
3. `docs/tester-requests.md` (its rules win if they differ)
4. The top ~15 stages below.

Work only on branch `testing`. Stages are newest first.

---

## Stage 7: Config window border fits the Combat row, version 0.4.2 (2026-10-30)

> Owner's 0.4.1 screenshots: the fill reached below "Combat" but the border stopped at "Game
> Mode". The help line on Combat was right.

- Re:CoM's 9-slice (`0x14024C270` / `0x14024AD40`) only handles sprites with exactly 9 parts. The
  panel's border and highlight are 8-part sprites, so the earlier bottom offset never reached
  them. Checked (disasm).
- Fix: at the same open/close points, nothing per frame:
  - each border part below the sprite's middle gets its own copy of its shared sprite-set rect,
    moved down 26 units;
  - this moves the bottom corners and edge and stretches the sides;
  - Re:CoM re-reads the rect each update (`0x14024B380`), so it then draws the frame itself.
  - The shared data isn't changed.
- In game (owner's 0.4.1 screenshot): the Combat row works: cursor, highlight, and the help text
  fits on one line. The border fix hasn't been seen yet.

---

## Stage 6: owner's animation rule, Config panel fit, version 0.4.1 (2026-10-30)

> "i wouldn't bother having an option to switch between KH1 and RECOM animations because for all
> intents and purposes they're the same, just use RECOM's default animations, and KH1 animations
> for anything that isnt included in ReCOM"

- Asset step (`kh1-recom-assets`, new `matching.rs`): each KH1 entry is matched against Re:CoM's
  clips at their key frames on all 135 bones.
  - 39 are bakes: every bone ≤0.9° and ≤1 mm; worst 0.74°, the four known pairs <0.02°.
  - 10 are Re:CoM edits: ≤20 bones differ, no other owner.
  - The closest wrong pair is 17 bones; the closest clip for a KH1-only entry is 26 bones.
  - Matches all 46 of `clip_origin`'s pairs, plus 0→p001_001, 65→p001_043, 78→p001_157.
    Checked (files, tests).
  - Only the 69 KH1-only entries get new slots (161–229). `PL0001.ANN` is 10.0 MB and `PL0002.ANN`
    0.84 MB. `kh1-motions.json` maps all 118 KH1 entries to the Re:CoM slot that plays them.
- DLL:
  - The "KH1 motions test" row, its ini key and the action-table row swap are removed. An old ini
    line is ignored.
  - Config panel: in the existing draw hook, Re:CoM's own 9-slice (`C2DSprObj`, vtable
    `0x1406077D8`, corner offsets `+0xA0..+0xAC`, flag 8 at `+0x144`, applied in `0x14024AD40`) is
    given one extra row of bottom offset. Checked (decomp); that the panel art is a 9-part sprite is
    an Assumption, and the log reports it.
- In game (owner's screenshot of 0.4.0): our rows render with Re:CoM's own buttons, font and value
  highlights. The panel was one frame short, which is what this stage fixes. Not yet seen in game.

---

## Stage 5: Phase 1 in Re:CoM: KH1 motions asset step, motion switch, Config rows, version 0.4.0 (2026-10-30)

> "lets finish phase 1" (ROADMAP M2 + M3)

- **Asset step** (`crates/kh1-recom-assets`, CLI `kh1-recom-build`; `docs/kh1-recom-assets.md`):
  - Converts all 118 KH1 Sora motions (via `kh1-anim`) into new 60 fps slots 161–278 of Re:CoM's
    `PLAYER/0001/PL0001.ANN`, and the matching Keyblade clips into `PLAYER/0002/PL0002.ANN`.
    Re:CoM's own slots stay byte-identical.
  - Writes `<mod_path>\Recom\…` plus `kh1-motions.json`. Panacea serves files from
    mod_path + "/recom" by archive path. Checked (OpenKH Panacea source).
  - Validation: Re:CoM pack decode → encode is byte-identical; converted clips match KH1's pose
    ≤0.045 mm / 0.005° per frame. Against Square Enix's own bakes they are ≤0.019° (body) and
    ≤0.007° (Keyblade, entry 98). Checked (files, tests).
  - Known: the Keyblade tip `k9` is off by 1.58° in entry 4 (KH1 bone 283's copy-orientation
    constraint in `kh1-anim`), and by 0.18° in entry 74.
  - Loader: no 161-slot limit; sizes come from the file (`0x14029F3A0`), and the name lookup
    (`0x1401A47E0` / `0x1402A42B0`) handles slots ≥161. Checked (decomp).
- **DLL** (`crates/kh1-recom`; `docs/recom-config-menu.md`, `docs/recom-motion-switch.md`):
  - Hook infrastructure: byte-checked, all-or-nothing. Starts from Panacea's `OnInit`.
  - Config: two rows, "Combat: Re:CoM / KH1" and "KH1 motions test: Off / On", added to the camp
    menu's Config screen using Re:CoM's own item, layout, cursor and help code. Hooks: Config input
    `0x1400D3EB0` (`48 89 5C 24 08 57 48 83 EC 20`) and draw `0x1400DF1B0`
    (`48 8B C4 55 41 54 41 55 41 56`). Bytes re-checked by the lead against the exe.
  - The values are saved in `kh1-recom.ini`: the Config save has no byte that can be proven free.
  - `config_menu = off` is a safety switch.
  - The KH1 motions test repoints 12 name pointers in Sora's action table (`0x1407C5E80`): idle,
    walk, run, swings 1–2, lunge, finisher. Re:CoM resolves and plays them itself from the next
    room load. Data only, nothing per frame.
- Not checked in game: the rows' look and behaviour, the clips playing, the Keyblade following.
  Tester thread `testers/recom-phase1/thread.md`.

## Stage 4: KH1 trace logger, replay and diff (M1), version 0.3.0 (2026-10-30)

> "lets finish phase 1" (ROADMAP M1)

- `crates/kh1-trace` (`docs/kh1-trace.md`):
  - `kh1-trace.dll` for KH1 FM Steam. Build check: PE time stamp `0x669E37CE`, size
    `0x2F91000`.
  - One hook on the per-actor task `0x140297FE0` (`48 8B C4 57 48 83 EC 60`). F9 records
    `kh1-trace-<n>.csv` with about 70 columns: state, motion, position, velocities, combo, and the
    inputs.
  - `kh1-tracediff` compares two traces.
- Playground: `--replay <csv>` feeds the recorded inputs to the controller and diffs the result.
  Replay is deterministic byte for byte (tests).
- Finding: KH1's "ground contact" is +0x2D0 bit 0, not bit 1. Checked (decomp).
- Assumptions: the camera yaw formula (checked against KH1's own move vector on real traces); Sora
  is party slot 0; Panacea calls `OnInit` before frames run.
- Not run in KH1 yet. Tester thread `testers/kh1-traces/thread.md`.

---

## Stage 3: smoother Sora in the playground, PS5 controllers, version 0.2.1 (2026-10-30)

> "lets make the actual character model looking smoother, and make sure ps5 controllers work with
> it" (owner chose a smoother mannequin, not the real mesh)

- `kh1-playground/src/body.rs` replaces the old one-capsule-per-bone figure:
  - smooth tapered limbs (two spheres joined by a cone, with smooth normals) and rounded joints;
  - the pelvis, chest, head and shoes are ellipsoids oriented by the surrounding bones;
  - Sora's 14 real hair spikes, jacket tails, hood and fingers follow their own bones;
  - MSAA ×4, plus a fill light.
- The KH1 bone roles come from matching KH1's skeleton to Re:CoM's named one, e.g. 17/18/20/23 for
  the right leg and 93/94/97/100 for the right arm. Checked (files).
- **Finding:** KH1 doesn't pose the end bones of chains (flags `0x6000000`: hair tips, jacket
  tails, hood). The figure places them from the parent's world matrix and the end bone's rest
  translation. Checked (files: idle pose dump).
- Gamepads:
  - Bevy's gilrs uses Windows.Gaming.Input with SDL's controller database, which includes the
    DualSense `054C:0CE6` and the Edge `0DF2` over USB and Bluetooth (gilrs builds USB-style
    GUIDs). Checked (crate source; the mapping string is in the built exe).
  - The HUD shows each pad's name, ids, stick and held buttons, with PS labels.
  - Not tested with a real pad.
- No change to the rewrite crates or the Re:CoM DLL, apart from the version number.

---

## Stage 2: KH1 battle system Rust rewrite + playground demo, version 0.2.0 (2026-10-30)

> "Lets try to do a full 1:1 Rust Rewrite of the KH1 Battle System, similar to how we did the KH2
> battle system rewrite, with a demo i can test" (owner: reference + demo; first demo movement +
> Attack)

- New crates (`docs/kh1-rewrite.md`):
  - `kh1-formats`: reads PC HED/PKG and KH1's `.mset`, `.mdls` and `btltbl.bin`.
  - `kh1-anim`: KH1's posing pass (curves, start pose, constraints, two-bone IK, frozen-snapshot
    blends).
  - `kh1-core`: the 60 Hz controller and combat.
  - `kh1-data`: install discovery and `SoraKit`.
  - `kh1-playground`: Bevy demo.
- Data is read at runtime from the player's `kh1_first.pkg` (`xa_ex_0010.mset/.mdls`,
  `btltbl.bin`). Checked (files: decoding gives the extracted files' SHA-256).
- `kh1-anim` matches Square Enix's own Re:CoM bakes of the same KH1 motions to ≤0.019° local
  rotation and ≤0.051 mm world position on swing / roll / run / jump. Checked (files, tests).
- `kh1-core` corrections to the notes (cone 22°, air row 17 above, combo 13→14→8 with a close
  target, …) are in `docs/kh1-sora.md`. Checked (decomp).
- Assumptions are marked in the code and the HUD: root travel subtraction, MSVC `rand`, task order,
  dummy stats (Strength 5, Defence 2, 300 HP), and the camera (an orbit camera, not KH1's).
- Headless check: a scripted combo runs rows 13 → 14 → 8 and hits the close dummy for 3, 3 and 7.
- The `kh1-recom` DLL is unchanged apart from its version (0.2.0); it still only checks the game.
- 70 tests (all pass with `KH1_TEST_DATA`); clippy clean.
- Not checked against a KH1 trace yet. Gamepad untested.

---

## Stage 1: the mod's skeleton DLL, version 0.1.0 (2026-10-30)

> "B, and use whichever is least effort and is most native to RECom" (owner: Panacea loads the
> mod; at KH1 setting Sora plays KH1's motions re-baked at 60 fps)

- New crate `crates/kh1-recom`: a cdylib that OpenKH's Panacea loads (`openkh/mod.yml`, copied as
  `dll/kh1-recom.dll`). On load it starts one thread and then nothing runs per frame:
  - checks the exe is the Steam build: PE time stamp `0x669E38DF`, size of image `0xD4A000`.
    Checked (exe).
  - checks the first 16 bytes of each future hook site. Checked (exe bytes = decomp function
    starts):
    - initAttack `0x1401F0D90`
    - field swing init `0x1401F0F70`
    - player motion request `0x1401F7180`
    - attack row pick `0x1401F7520`
    - Battle exe `0x1401F9840`
    - Field exe `0x1401FA0F0`
    - use-card button `0x1401E8FF0`
    - `.ANN` loader `0x14029F3A0`
  - reads `kh1-recom.ini` (`combat = kh1 | recom`, default `kh1`), writing it if missing. Nothing
    reads the setting yet. The Config-menu toggle is still to do.
  - writes `kh1-recom.log` next to the game.
- **No hooks installed and nothing patched.** The game plays exactly as Re:CoM.
- Panacea knows Re:CoM: its `DBGHELP.dll` names the Re:CoM exe, uses a `/recom` mod subfolder, and
  hooks Re:CoM's `Axa::CFileMan`. Checked (files: strings in the owner's Panacea DLL). Whether it
  loads `dll/*.dll` for Re:CoM as it does for KH1: Assumption, to check with the first launch.
- Build: MinGW cross-compile (`tools/build-win.sh`); build bot `.github/workflows/build.yml`
  commits `openkh/kh1-recom.dll` on `testing`. 7 unit tests.
- Not checked in game yet: that Panacea loads the DLL, and the log on the owner's machine.

---

## Stage 0: groundwork (2026-10-29)

> "Where I'd start: a cardless Attack, built into Re:CoM's own battle code. Before that comes a
> short groundwork step (step 0)."

No code. `VERSION` not created yet; the first change under `crates/` starts it.

- Repo seeded: these rules files, `docs/sources.md`.
- Re:CoM Steam decomp made (see `docs/sources.md`). Checked (exe): SHA-256 `acf42e96…6bbf`.
- Finding: Re:CoM's exe is mixed-mode C++/CLI; the .NET part is only the shell. Checked (exe).
- Finding: the KH1 notes in KH2-Combat-in-KH1 use Epic addresses; convert to Steam before use.
  Checked (files).

Step 0 mapping (2026-10-29, notes only):

- `docs/recom-timing.md`: both games step by a variable delta in 60 Hz frames. Re:CoM uses 50
  units per frame, so KH1 frames × 50 = Re:CoM time. Checked (decomp + exe, both games).
- `docs/recom-player.md`: player state machine (table-driven, 4 states: Field / Battle / Wait /
  Hide), the action switch, and the card path from button to `initAttack`. Checked (decomp).
- `docs/recom-combat.md`: motion request and advance, Sora's attack-row table `0x1407C6C70`, the
  hit path and the damage formula. Checked (decomp; rows read from the exe).
- `docs/recom-battle-entry.md`: field-to-battle handover (separate battle room) and the native
  cardless field swing (action `0xA`, card id 0) that shares battle collision. Checked (decomp).
- Assumption, well supported: Re:CoM's motion system is not KH2's mset system.

Step 0 still to do:

1. Plan the trace tool with the tester: the same fields from both games (tick, state, motion,
   frame, position, velocity, hit events).
2. KH1's side mapped in `docs/kh1-sora.md` (states, movement, jump, glide, roll, guard, motion
   requests, the Attack combo). Still needed: the numbers in KH1's data files (`btltbl.bin`,
   `xa_ex_0010.mset`) and Re:CoM's (`stockskilllist.bin`, `PL0001.ANN`), from the player's
   extracted files.
3. Open, not read yet: the damage veto `0x140180920`, the AP rewrite `0x14015D5F0`, the
   double-hit spawn `0x140158DD0`, and which hit kind the field swing produces on map enemies.
