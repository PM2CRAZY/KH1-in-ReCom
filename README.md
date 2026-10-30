# KH1 Combat in Re:Chain of Memories

A mod that brings Kingdom Hearts Final Mix's combat to Sora in Kingdom Hearts Re:Chain of Memories
(PC, Steam, KH 1.5+2.5), built into Re:CoM's own systems. Work happens on the `testing` branch;
the design rules are in `docs/`.

**Status: version 0.1.0.** The mod loads, checks the game and writes a log. It doesn't change
anything in play yet.

## Install (OpenKH + Panacea)

1. You need OpenKH's Mods Manager with Panacea set up for KH 1.5+2.5 (Steam).
2. Make a mod folder holding `openkh/mod.yml` and `openkh/kh1-recom.dll`. The build bot commits
   the DLL on `testing`. Install it for **Kingdom Hearts Re:Chain of Memories** in the Mods
   Manager, then tick it and build.
3. Start Re:CoM. Next to the game exe you'll find:
   - `kh1-recom.log`: what the mod found. The last line should be
     `result: all checks passed.`
   - `kh1-recom.ini`: settings, written with the defaults on first launch.

## KH1 playground (demo)

`kh1-playground` is a 1:1 Rust rewrite of KH1 FM's movement and Attack, with a mannequin Sora
posed by his real skeleton and motions and four training dummies. Everything is read from your own
KH 1.5+2.5 install. It finds the install by itself, or you can pass `--game-dir "<folder>"`.

- Keyboard: WASD move, Shift walk, Space jump/glide, J or Enter Attack, K Dodge Roll/Guard, L lock
  on, arrows/mouse camera, 1–8 abilities, Tab hitboxes, R reset, F1 help.
- Gamepad (KH1 default layout), PS5 DualSense or Xbox: ✕ / A Attack, ○ / B jump, □ / X
  roll/guard, R1 / RB lock on. The HUD's `pad:` line shows the pad it sees and the buttons held.
- `--trace out.csv` logs every frame for 1:1 comparison with KH1.

## Build

- Tests: `cargo test`
- Windows DLL from Linux: `tools/build-win.sh` (MinGW; writes `openkh/kh1-recom.dll`).
- Playground: `cargo run --release -p kh1-playground`.

This repository contains no game files, no extracted data and no decompiled code.
