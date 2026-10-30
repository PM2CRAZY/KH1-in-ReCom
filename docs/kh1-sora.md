# KH1 Sora (Steam): movement, motion and the Attack combo

All addresses are **Steam** KH1 FM (`KINGDOM HEARTS FINAL MIX.exe`, SHA-256 `d7907462…`). Units
are KH1 world units (about 1 cm, y down) and 60 Hz frames (frame step `0x14233FBDC`, see
`recom-timing.md`).

Tags: Checked (decomp) = read in the KH1 Steam decomp. Checked (notes) = from KH2-Combat-in-KH1's
notes, with the Epic address converted and the same code found at the Steam address. Assumption =
inferred. Epic → Steam conversions use KH2-Combat-in-KH1's `steam_map.in`.

## The big constraint: most numbers are in KH1's data files

The exe has the logic. The per-move numbers live in two data files:

- **`btltbl.bin`** (loaded at `0x142D22D40`): action rows, attack data (power, radius, bone),
  combo ranges and angles, jump heights.
- **Sora's motion set `xa_ex_0010.mset`**: each motion's length, root travel, and the flag tracks
  that hold hit-active frames, the combo window, the "free to act" frame and cancel windows.

So 1:1 needs the player's extracted KH1 files, read by the asset build step. The decomp shows where
each value sits and how it is used. Checked (decomp).

## States: table-driven

- State id at `actor+0x70` (previous at `+0x378`). The table at `0x142DB3840` holds 59 records of
  0x40 bytes: flags, enter, exit, input, move, next-state, stick handler, jump handler. Registered
  by `0x1402B1B80`. State change `0x1402B1960`. Checked (decomp).
- Sora's main states: 0 ground (idle, walk, run, guard and roll are all state 0 with different
  motions), 1 jump take-off, 2 air (rise and fall), 8 flight, `0x15` glide, plus ladders, hangs,
  swim and push. Checked (decomp).
- Per frame: think `0x1402A3300` (pad, state input, facing), physics `0x14029A3C0` (state move,
  root motion, mover, next state), motion update `0x14029F0A0`. Checked (decomp).

## Ground movement

- Stick: per-axis dead zone 60 of 127.5, rescaled by 67.5, length clamped to 1. Checked (decomp).
- **Speed snaps, no acceleration:** tilt 0 → 0; up to 0.8 → walk; over 0.8 → run.
  `0x1402A5F10`. Checked (decomp).
- Stats block (`actor+0x6C`, pool `0x142D5CC10`): **walk 2.0, run 8.0 units/frame**, air accel
  0.1, air drag 0.95 per 2 frames, **turn 11.25°/frame** (π/16). Checked (decomp; defaults in
  `0x1402A5D70`).
- Movement direction follows the stick at once; only the body's facing turns at the turn rate.
  Checked (decomp).
- Motions: idle / walk / run requests 0 / 1 / 2, blend 8 (idle 10). Checked (decomp).

## Jump and gravity

- Press → `0x1402AF9F0`: request `0x10004` (High Jump `0x10009`). 4-frame crouch, then the arc.
  Checked (decomp).
- **Gravity 0.48469388 units/frame².** Rise time `T = sqrt(2h/g)`, a double-eased sine rise,
  then a parabolic fall, **capped at 20 units/frame**. Checked (decomp).
- Jump height h comes from `btltbl` (`+0xAC` normal, `+0xAE` High Jump, for Sora). Assumption:
  190 normal, which matches g exactly. The value is in the data file.
- Releasing jump after frame 6 cuts the rise: 5 more frames, 6.06 units. Checked (decomp).
- Air control: 0.1 × speed per frame toward the stick, |v| capped at max(previous, 8); drag 0.95
  per 2 frames with no stick. Checked (decomp).
- Landing: the land motion plays as an action, then idle at blend 8. Checked (decomp).

## Glide, Dodge Roll, Guard

- **Glide** (state `0x15`): needs more than 50 units above the floor; forward speed rises 0.4 per
  frame to 8 (Superglide 16); turn 2° (Superglide 1°) per frame; 24 frames before the next glide.
  Checked (decomp).
- **Dodge Roll** (`0x1402C9E60`): motion `0xDC` with root motion; **12 frames of no flinch**; the
  untouchable frames are events in the motion. With Guard also equipped, the roll needs tilt over
  0.8. Checked (decomp); the frame numbers are motion data.
- **Guard**: action row `0x3D` of the action table, facing the lock-on target. Duration and repel
  window are motion and attack data. Checked (decomp) for the path.

## Motion requests

- `0x14029F7C0(actor, request, blend)`: locomotion; does nothing if already playing; keeps the
  walk/run phase. `0x14029F980`: replaces the queue and starts now (actions). `0x14029FA30`:
  appends. Blend is in 60 Hz frames. Checked (decomp).
- Request bits: `0x10000` hold at end, `0x1000000` action lock, `0x40000000` release the lock.
  The request → motion entry map is in the mset. Checked (decomp).
- Frames advance by step × rate (rate 1.0). Root motion goes into the physics step. After each
  update, `+0x1CC` = the motion's per-frame flags. Checked (decomp).

## The Attack combo

- **Press to swing:** `0x140293480` stores the Attack command → command step `0x1402A7510` →
  row exec `0x1402A68F0` → row reader `0x1402A3980`, which plays the motion and builds the attack
  object. Allowed when Sora can act, or inside the combo window (`+0x1CC & 0x400000`).
  Checked (decomp).
- **Combo counter:** byte `+0x41` of the combo record (`actor+0x14C`). It resets to 1 outside the
  window and goes up by 1 inside it. A press is dropped if the last swing hit nothing, unless Combo
  Master is equipped. Checked (decomp).
- **Length:** 3 + Combo Plus copies (ground), 3 + Air Combo Plus copies (air) (`0x1402A5130`).
  The finisher is the press where the counter reaches the max. Checked (decomp).
- **Ground rows:** basic swings 13 / 11 (alternating), lunge 14 (by target distance), jump-up 1
  (target 160+ above), hop 2 (path blocked), finisher 8 (or Stun Impact, Gravity Break, Zantetsuken,
  Blitz, Ripple Drive by ability), Sliding Dash 9, Vortex 10. Checked (decomp); the ability names
  are an Assumption.
- **Air rows:** swings 18 / 17, finisher 16, Hurricane Blast 15. Horizontal speed toward the target:
  8 on hit 1, 4 on middle hits, 12 on the finisher. Checked (decomp).
- **Thresholds** (close range, lunge range, angle cosines) come from `btltbl` + 0x58. Checked
  (decomp); the values are data.
- **Timing comes from the motion's flag tracks**, copied to `+0x1CC` each frame: hit active, combo
  window, free to act, Keyblade trail. Checked (decomp).
- **Row layout** (`0x142D5CAB0[char] + row × 0x14`): motion, trail, weapon effect, attack-data
  offset into `btltbl`, hop height, effect/sound, voice. Checked (decomp).
- **Attack data** (0x70 bytes): power (+ Strength), min/max, bone, blade span, radius. Finisher
  knockback `0x1402B8530`. Checked (notes) for the damage path.
- **Keyblade hit shape:** a chain of spheres along the weapon bone between two blade fractions,
  plus a chain swept from the last frame. `0x1402B3D50`. Checked (decomp); the radius is data.

## What this means for the native build

- KH1's hit and combo timing is **motion-data-driven** (flag tracks in the mset). Re:CoM's is
  **table-driven** (hit window bytes in the attack row, `recom-combat.md`). Matching KH1 1:1 means
  carrying KH1's per-motion windows into Re:CoM's own data, as new attack rows, motion events or
  both. That design is open and needs the data first.
- Movement constants (speeds, gravity, turn, jump arc) are in the exe and fully known, except the
  jump heights.
