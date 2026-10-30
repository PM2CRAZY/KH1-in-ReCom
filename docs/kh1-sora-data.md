# KH1 Sora: the numbers from KH1's data files

The logic is in `kh1-sora.md`; this note gives the values. They were read from the owner's original
KH1 files (`btltbl.bin` SHA-256 `b70db229…`; `xa_ex_0010.mset` SHA-256 `60d43ee9…`), guided by the
KH1 Steam decomp. Units: KH1 units and **60 Hz frames** (every motion in the mset has fps 60.0,
Checked (files)). In Re:CoM these become `frames × 50` (`recom-timing.md`).

The files themselves are never put in this repo. The asset build step reads them from the player's
own copy.

## Where the values are (so a build step can read them)

- `btltbl.bin` is loaded at `0x142D22D40`; file offset = address − that. Checked (decomp).
- Combo and jump block `G` = file `0x58` (`0x1402A6180` reads it). Checked (decomp + files).
- Sora's action rows: file `0xA9F0 + row × 0x14`. The exe pickers compare against
  `btltbl + 0xA9F0` (`0x1402A6180`). Checked (decomp + files).
- Attack records: 0x70 bytes at the offset in each row's `+4`. Checked (files); field meanings from
  the damage code.
- Weapon table at file `0x94F8` (Kingdom Key hit radius 40). Checked (files).
- mset: 3 sections at `0x80` / `0x252480` / `0x252A00`. The request map is one u32 per request
  (entry bits 0–11, type 12–14, root motion bit 15). Each motion has its frame count at header
  `+4`, fps at `+8`, and its flag-track list at `+0x3C` (12-byte entries, 16-byte keys; type 1 hit,
  0 free, 5 combo window, 7 trail). Checked (files).

## Combo thresholds and jump

| Value | File | Number | Tag |
|---|---|---|---|
| Narrow cone | `0x58` | 45° (cos 22.5° = 0.92388) | Checked (files + decomp) |
| Wide cone | `0x5C` | 240° (cos 120° = −0.5) | Checked (files + decomp) |
| Close range (normal swing below it) | `0x60` | 180 | Checked (files + decomp) |
| Lunge range (lunge between 180 and this) | `0x64` | 300 | Checked (files + decomp) |
| Jump height, normal | `0xAC` | **190** (matches gravity 0.48469388 exactly) | Checked (files) |
| Jump height, High Jump | `0xAE` | **290** | Checked (files) |

## The combo motions (frames from the motion's start)

| Move (row) | Motion | Length | Hit active | Combo window | Root travel | Attack record |
|---|---|---|---|---|---|---|
| Ground swing 1 (13) | 200 | 48 | 14–24 | 22–40 | 100 | `0x4B78` |
| Ground swing 2 (11) | 201 | 48 | 18–32 | 26–42 | 120 | `0x4BE8` |
| Lunge (14) | 202 | 64 | 18–30 | 26–54 | 134.94 | `0x4B78` |
| Ground finisher (8) | 203 | 80 | 26–38 | none | 190 | `0x4C58` |
| Air swing 1 (18) | 204 | 40 | 6–20 | 18–34 | 0 (hop 20) | `0x4B78` |
| Air swing 2 (17) | 205 | 42 | 10–20 | 18–32 | 0 (hop 20) | `0x4BE8` |
| Air finisher (16) | 206 | 56 | 12–38 | none | 0 (hop 40) | `0x4C58` |
| Hop attack (2) | 204 | 40 | 6–20 | 18–34 | 0 (hop 60) | `0x4B78` |
| Guard (0x3D) | 212 | 54 | 10–36 (repel) | 22–54 | 0 | `0x4DA8` |
| Counterattack (0x3E) | 213 | 86 | 8–24 | 18–56 | 267.05 | `0x4E18` |

All Checked (files). Ability finishers and Sliding Dash / Vortex rows (3–7, 9, 10, 15, 0) are in the
research notes kept with the decomp tools, to be added when those abilities are built.

What these mean in play (from the exe logic, Checked (decomp)):

- Attack is accepted again only inside the combo window. No combo motion has a "free" track, so after
  the window closes Attack is refused until the motion ends. For example, frames 40–48 of swing 1. A
  press after the motion ends starts a fresh combo.
- The basic ground combo alternates 200 ↔ 201 (rows 13 ↔ 11), with 202 as the lunge. The 3rd press
  (by default) is the finisher.
- Swings are flinch hits (kind 3); finishers are launch hits (kind 5, +4 power). Basic swings have
  power 0, so their damage comes from the Strength − Defence curve at file `0x00`.

## Other motions

| Move | Motion | Length | Free from | Root travel | Tag |
|---|---|---|---|---|---|
| Dodge Roll | `0xDC` | 38 | 30 | 350 (213.82 in frames 0–12, 136.18 in 12–24) | Checked (files) |
| Land | 7 (High Jump `0xC`) | 28 | 12 (a 12-frame lock) | 0 | Checked (files) |

- Normal jump motions: entries 8 → 9 → 10 → 11. High Jump: 12 → 13, then the same fall (10) and land
  (11). Checked (files).
- The roll's untouchable frames are not in the mset flag tracks. Where they come from is open.

## Found along the way (not built, matters for 1:1 later)

- **MP regen on hits.** Each hit charges 3 (basic swing), 6 (finisher) or 24 (counter), scaled by
  level difference; 30 units = 1 MP. Checked (files + decomp, research agent). Needed when MP is built.
- **A fast cast path.** Command type 2 (Assumption: magic) replays request `0x3D` (`0x8A` in the air)
  inside swing 1's and the lunge's windows. Not in any earlier note.

## Corrections to KH2-Combat-in-KH1's notes

- The attack radius at record `+0x5C` is a float (e.g. 10.0), not an int.
- 200 / 201 alternate as the basic swings; 202 is only the lunge.
- The "jump 12 → 13 → 10 → 11" sequence in its logs was a High Jump.
- Landing has a 12-frame lock.
