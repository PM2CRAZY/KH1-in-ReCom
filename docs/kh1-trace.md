# KH1 trace tool (roadmap M1)

A small logger DLL for real KH1 FM (Steam) plus a replay and a diff, to check the KH1 Rust
rewrite (`kh1-core`, `kh1-playground`) against KH1 frame for frame. Crate: `crates/kh1-trace`.

- `kh1_trace.dll`: OpenKH Panacea loads it into `KINGDOM HEARTS FINAL MIX.exe`. It logs Sora's
  state and the inputs, one CSV row per frame, while recording. It changes nothing the game reads.
- `kh1-playground --replay <trace.csv>`: feeds the recorded inputs into the rewrite, headless,
  writes its own trace and prints the comparison.
- `kh1-tracediff <a.csv> <b.csv>`: compares any two traces.

Tags: Checked (exe / decomp / files / trace) or Assumption. All addresses are **Steam** KH1
(`KINGDOM HEARTS FINAL MIX.exe`, SHA-256 `d7907462…8bac`, image base `0x140000000`).

## Install

1. Build: `RUSTC_BOOTSTRAP=1 CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc cargo
   build --release --target x86_64-pc-windows-gnu -p kh1-trace -Zbuild-std=std,panic_abort`.
   Output: `target/x86_64-pc-windows-gnu/release/kh1_trace.dll` (and `kh1-tracediff.exe`).
2. Copy it as `kh1-trace.dll` to `<mod_path>\kh1\dll\` (Panacea loads every `*.dll` there and
   calls its `OnInit`; `OpenKh.Research.Panacea/Panacea.cpp`, `LoadDLLs`).
3. **Take `kh2style.dll` out of that folder** (or disable that mod) while recording. It is the
   KH2-combat mod and takes Sora over, so the trace would not be real KH1.
4. Start the game. `kh1-trace.log` next to the exe says whether the build was recognised and the
   hook went in. On any other build it hooks nothing.

## Recording

- **F9** starts and stops recording (only while the game window has focus). Each start writes a
  new `kh1-trace-<n>.csv` next to the exe. One short move per file keeps them small and easy to
  read.
- Before each move, stand still for about a second on flat, open ground, then press F9, wait half a
  second, do the move, wait until Sora is idle again, press F9.
- Keep the command menu on **Attack** (the replay reads the command button as Attack).
- For the combo tests, **lock on** to the enemy before pressing F9 and keep the lock. The replay
  puts its dummy where the lock target stood.
- The abilities equipped are recorded each frame, and the replay uses them. With Guard and Dodge
  Roll both equipped, the roll needs full tilt (KH1's rule).

What to record (one file each):

1. Walk a slow circle (half tilt), then run a circle (full tilt), then stop.
2. Jump, holding the button (full height). Then a jump released early (tap).
3. High Jump equipped: the same two jumps.
4. Glide: jump, then hold jump in the air until landing. Superglide too, if equipped.
5. Dodge Roll from a run.
6. Guard from standing.
7. 3-hit ground combo on an enemy at close range (touching distance), locked on.
8. The same combo starting about 250 units away (a few steps; the lunge range).
9. Air combo: jump next to the enemy, three Attack presses in the air, locked on.

Then send the files (and `kh1-trace.log`).

## Replay and diff

```
kh1-playground --game-dir "<KH 1.5+2.5 folder>" --replay kh1-trace-3.csv [--trace out.csv]
kh1-tracediff kh1-trace-3.csv out.csv
```

- The replay starts Sora at position 0 with the recorded facing and abilities. Each row's stick,
  camera yaw, buttons, lock and frame step go into the controller (`kh1-playground/src/replay.rs`).
- `--replay-layout kh1` (default): no platform; one dummy where the KH1 lock target stood relative
  to Sora's start, or none if there was no lock. `--replay-layout playground` keeps the
  playground's own arena (for replaying a playground trace).
- The diff lines the traces up at their first input (stick tilted or a button), so idle lead-ins do
  not matter. It compares state, motion request (low 16 bits), entry, motion frame, position and
  facing relative to Sora's start (in his start facing's frame), combo counter and row (when both
  have one). It prints the first divergence and each field's mismatch count and largest error.
  Default tolerances: position 0.01, facing 0.001 rad, motion frame 0.001. Exit code 0 match, 1
  divergence.
- It warns when the two traces' inputs differ (then the comparison is not like for like), and
  checks the camera formula below against KH1's own move vector.

## The hook

- **`0x140297FE0`**, the per-actor task. `0x140297980` registers it for every actor (task record
  +0x18 = the actor), so it runs once per frame per actor. Sora's think `0x1402A3300` and physics
  `0x14029A3C0` run inside it. His motion update `0x14029F0A0` runs later in the frame, from the
  motion task `0x140298730` (registered at priorities −1000, 5000, 8000; Sora's task is at 2000).
  Checked (decomp).
- First bytes `48 8B C4 57 48 83 EC 60` (`mov rax, rsp` / `push rdi` / `sub rsp, 0x60`): three
  whole instructions, nothing RIP-relative. The DLL checks all 8 before patching, and the function
  is 16-aligned, so the 8-byte patch (`jmp rel32` + 3 `nop`) is one atomic store.
  Checked (exe, disassembled).
- The hook calls the original for every actor. Only when recording and the actor is Sora does it
  read anything. Row n = frame n's inputs, read as Sora's task starts (before his think consumes
  the command word) plus the move vector right after his task, and the state after frame n, read as
  his task starts on frame n+1 (after the motion update and the rest of the frame).
- Why not hook the think or the motion update: the think is skipped while actor +0x88 bit 0 is set,
  and the command word is already cleared when the motion update runs. Checked (decomp).
- Build check before hooking: exe name, PE time stamp `0x669E37CE` and size of image `0x2F91000`.
  Checked (exe).

## Columns and addresses

Actor fields are Sora's (`0x142D37280`, party slot 0). Units: KH1 units, y down, radians, 60 Hz
frames.

| column | KH1 source | tag |
|---|---|---|
| frame | rows since F9 | — |
| step | float `0x14233FBDC` (set by `0x14018D1F0`) | Checked (decomp) |
| freeze | actor +0x88 bit 0 (think skipped; set by `0x1402BEEE0` for damage type 9, a Stop-like status) | Checked (decomp) |
| state / prev_state | +0x70 / +0x378 | Checked (decomp) |
| request / entry / motion_frame | +0x164 / +0x168 / +0x16C | Checked (decomp) |
| blend / blend_clock | +0x170 / +0x174 | Checked (decomp) |
| motion_flags | +0x1CC | Checked (decomp) |
| flags, action_lock | +0x00, bit 4 | Checked (decomp) |
| hit_active, combo_window | +0x1CC & 0xF0000, & 0x400000 (kh1-core's reading of the flag bits) | Checked (decomp) for 0x400000; Assumption (kh1-core) for 0xF0000 |
| grounded | not (+0x2D0 bit 0 and no support handle +0x398): the physics step's airborne test in `0x14029A3C0` | Checked (decomp) |
| height | +0x3D4 | Checked (decomp) |
| pos_x/y/z | +0x10 | Checked (decomp) |
| move_x/z, speed | +0x20 / +0x28, +0x2C | Checked (decomp) |
| air_vx/vy/vz | +0x3C0 | Checked (decomp) |
| facing / target_yaw | +0x3C (written by `0x140297E10`) / +0x150 | Checked (decomp) |
| air_t / rise_t | +0x3B0 / +0x3B8 | Checked (decomp) |
| no_flinch | +0x0C | Checked (decomp) |
| combo_counter, row, cb_flags, swing_hits | combo record (handle +0x14C): +0x41, +0x40, +0x42 (u16), +0x50 (int) | Checked (decomp) |
| in_stick_x/y, in_tilt | logical pad `0x14298CA90`: unit vector +0x60 / +0x64 (y down) × tilt +0x4C, turned to kh1-core's (x right, y forward) | Checked (decomp: `0x14028C7E0` reads them; `0x14028B0B0` copies pad 0 there) |
| in_raw_x/y | device record `*(0x1421AAF18)` +0x18 +4 / +5, when its +0x37 is 1 | Checked (disassembly of `0x14028B0B0`'s call to `0x14028B4F0`) |
| in_cam_raw / raw2 / mode | `0x14299BB04` / `0x14299C194` / `0x14299BB08` | Checked (decomp) |
| in_cam_yaw | π + raw + raw2 (see below) | Assumption |
| in_move_x/z/w | `0x142D5EC30` (x, z, w = tilt), read after Sora's task | Checked (decomp) |
| in_can_act | `0x140297870`'s test (flag 4, +0x88 bit 0x20, +0x444 bit 1) as the frame starts | Checked (decomp) |
| in_held / in_pressed | logical pad +4 / +8 (`0x14028B430` fills them) | Checked (decomp) |
| in_command | `0x142D53A40` as Sora's task starts; the control step `0x1402A6220` clears it | Checked (decomp) |
| in_jump / in_attack / in_square | held/pressed bits 4, 0x4000000, 0x10 (the dispatcher `0x1402937F0` turns them into command bits 0x1000000, the command, 0x4000000) | Checked (decomp) for the bits; the button names are Assumption |
| in_lock, lock_state | `0x142DB3510` ≠ 0 | Checked (decomp) |
| target_x/y/z, target_part | lock pair `0x142DB3518`: actor handle → actor +0x10; part `0x142DB351C` | Checked (decomp, `0x1402A3B00`) |
| abilities | `0x142D5EC10` (built by `0x1402A6380`) | Checked (decomp) |
| combo_max / air_combo_max | stats (handle +0x6C) +0xD4 / +0xD5 | Checked (decomp, `0x1402A5130`) |
| combo_master | `*(0x142868BA0)` +0x44, 48 bytes: entry 0x41 with bit 7 clear | Checked (decomp, `0x1402A7510`) |
| disp_*, mp_gauge, events, hits | playground only | — |

Handles resolve as `0x14038AF40` does: table `0x142EE3980`, pointer = table[(h & 0x7FFFFFFF) >> 25]
| (h & 0x1FFFFFF). Checked (decomp). The DLL checks each resolved pointer is readable before
reading it.

### Camera yaw for the replay

`0x14028C7E0` builds the move vector from the pad vector v = (x, 0, y-down): turn by −yaw
(`0x14299BB04`, wrapped), negate x, turn by the second yaw (`0x14299C194`). Its rotation matrix
(`0x1400E6C80`) has rows (c, 0, −s), (0, 1, 0), (s, 0, c), applied as a row-vector product
(`0x1400E5AD0`). Checked (decomp). Worked through, that equals kh1-core's
`Input::move_from_stick(stick, π + yaw + yaw2)` (unit test `camera_formula_matches_kh1_steps`).
Assumption: the base matrix `0x1428672D0` is the identity (it is in .bss, so not readable from the
file). `kh1-tracediff` checks the formula against KH1's own move vector on every tilted frame where
Sora could act, and prints how many agree. Camera modes 2 and 0xB take other paths in
`0x14028C7E0`; the replay does not model them.

## Notes and corrections

- Pad blocks: `0x14298C9B0` (+ index × 0x70, index `0x14298CB10`) is the per-device block.
  The controller reads the **logical** pad `0x14298CA90`, which `0x14028B0B0` fills from it
  (button remap table `0x140528290`, stick vectors copied). The raw stick bytes are not in either
  block; they are in the device record above. Checked (decomp).
- `kh1-movement-motion.md` calls +0x2D0 bit 1 "ground contact". In `0x14029A3C0` the test is
  bit 0, and a **set** bit with no support handle means airborne. Checked (decomp).
- The playground's `--trace` now writes this schema too (a superset of its old columns, same names
  for those).

## Assumptions and limits

- The pad update and the command dispatcher run before Sora's task in a frame, so the pad bits
  read at his task's start are the frame's. Assumption (the command word, by contrast, is
  certainly the one his think consumes).
- `in_cam_yaw`: the identity base matrix (above). Assumption, checked by the diff on real traces.
- `hit_active` = motion flags & 0xF0000 is kh1-core's reading. Assumption.
- Party slot 0 is Sora. Assumption (strong: `0x140290D60` fills slot 0 for party member 0, and
  KH2-Combat-in-KH1 uses it as Sora's actor).
- The KH1 enemy is replaced by one dummy at its position, with the dummy's body (one column,
  radius 40), not the enemy's parts. Combo rows chosen by distance can differ near the range
  thresholds. Assumption: the enemy's actor +0x10 is the point the dummy's feet should stand on.
- The replay has a flat floor and no walls: slopes, steps or walls in the KH1 area make positions
  diverge. Record on flat, open ground.
- Hooking happens in `OnInit` (before the game runs frames). If no loader calls `OnInit`, the DLL
  hooks from a thread 2 s after loading; that patch could race the game thread. Assumption: Panacea
  always calls `OnInit`.
- Untested in the running game (no Windows / KH1 here). The DLL builds for Windows, its addresses
  and first bytes were checked against the Steam exe and decomp, and the replay, schema and diff
  are unit-tested. The first real trace is the real test.
