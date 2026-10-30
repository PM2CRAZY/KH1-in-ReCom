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

## Build

- Tests: `cargo test`
- Windows DLL from Linux: `tools/build-win.sh` (MinGW; writes `openkh/kh1-recom.dll`).

This repository contains no game files, no extracted data and no decompiled code.
