# Timing: Re:CoM and KH1 ticks

Steam addresses. Image base `0x140000000`.

## Summary

Both games count time in 60 Hz frames and both step gameplay by a variable delta, not a fixed
tick. **A KH1 frame count converts to Re:CoM time as `frames × 50`** (Re:CoM units), with no
rounding. Checked (decomp, both exes).

## Re:CoM

- **Time unit is 1/3000 s.** A static initializer at `0x140002C50` computes
  `0x140AC5538 = 3000 / [0x140821B88]`. The exe stores 60 at `0x140821B88`, so one 60 Hz frame is
  **50 units**. Checked (exe bytes `b8 b8 0b 00 00 99 f7 3d …` = `mov eax,3000; cdq; idiv`;
  value 60 read from the exe).
- **Pacing.** The thread `0x1402E94A0` runs a ~60 Hz multimedia timer. Each tick calls
  `0x14028FE20`, which increments the tick counter `0x140AC4444`, presents every 1 or 2 ticks
  (`0x140AC444C` = 0 or 1) and signals the main loop. Checked (decomp).
- **One frame.** `0x140290E20` computes `dt = ticks since last frame × 50`, capped at 200 (4
  ticks); a stall of over 1 s counts as one tick. Checked (decomp).
- **Game speed.** `0x1402AD810` multiplies `dt` by the float `0x140821BD0` (1.0 in the exe).
  Checked (decomp; value read from the exe).
- **Per task.** `0x1402AC3C0` stores the task's `dt` at `task+0x14`. Gameplay reads it there:
  timers subtract it directly, movement uses `dt × 0.02` (= 60 Hz frames, at `player+0x464`).
  Some low-priority tasks may skip updates under load. Checked (decomp).
- **No fixed sub-step** was found. Assumption: none exists.
- **30 / 60 fps mode.** `0x1402919B0` forces 60 fps, `0x1402919F0` forces 30 fps (`dt` = 100 per
  update), `0x140291960` follows the config. The field forces 30 fps in rooms 2000–2999.
  Checked (decomp). What those rooms are: Assumption (event rooms).

## KH1 (Steam)

- **Frame step** is the float `0x14233FBDC`, set each frame by `0x14018D1F0`:
  `step = vsyncs since last frame (0x142340EC4) × 1.0 (0x14233FBCC) × speed scale (0x14233FBE4)`.
  It is doubled when `0x14233FBF0` is set, capped at 5.0, and set to 128.0 when
  `0x1423404EC == 1`. Checked (KH1 Steam decomp).
- So at 60 fps the step is 1.0 per frame, and KH1's frame data counts 60 Hz frames.
- Epic counterpart: `0x1423405DC` (KH2-Combat-in-KH1 `steam_map.in`: `0x23405DC → 0x233FBDC`).
  Checked (files).
- **Trap:** in the KH2-Combat-in-KH1 notes, Epic `0x142340EC4` is the *room id*. In the KH1
  Steam exe, `0x142340EC4` is the *vsync counter*. They have the same number but are different
  variables. Never reuse an Epic number as a Steam one.

## What this means for 1:1

- KH1 frame data (windows, durations, timers) goes into Re:CoM as `frames × 50` units, and runs
  at the same real-time speed at 60 fps.
- Both games tie movement to the variable step, so behaviour under frame drops should match in
  kind. Not checked against a trace yet.
- At Re:CoM's forced 30 fps (rooms 2000–2999), `dt` is 100 per update. KH1 at 30 fps uses a
  step of 2.0. These match in kind. Not checked in game.
