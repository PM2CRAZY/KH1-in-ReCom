# KH1 battle system: the Rust rewrite (reference + demo)

Owner's decision (2026-10-30): a full 1:1 Rust rewrite of KH1 FM's battle system, like the KH2
rewrite in KH2-Combat-in-KH1, as a **reference and demo**. It is the 1:1 oracle that Re:CoM's
native implementation is tested against, and it feeds the asset build step. It never drives Sora
inside Re:CoM; Re:CoM still runs KH1 natively (`design-philosophy-instructions.md`).

First demo (owner): **movement + Attack**: run, jump, glide, Dodge Roll, Guard, ground and air
combos and finishers, hitting training dummies with KH1's damage and reactions. Magic, items,
abilities, enemies and the party come in later versions.

## Crates

| crate | what | depends on |
|---|---|---|
| `kh1-formats` | readers: PC HED/PKG archives, KH1 `.mset` motions, `.mdls` skeleton, `btltbl.bin` | — |
| `kh1-anim` | poses Sora's skeleton from a motion like KH1 does: curves, starting poses, constraints, IK | formats |
| `kh1-core` | engine-agnostic 60 Hz KH1 controller and combat: states, movement, jump, glide, roll, guard, combo picker, Keyblade hit shape, damage, reactions | — (plain data) |
| `kh1-data` | finds the player's install, reads the files, builds `kh1-core`'s data | formats, core, anim |
| `kh1-playground` | the demo: a window, a mannequin Sora with his real motions, dummies, hitbox display | all |

## Rules

- Every number comes from the player's own install at runtime (`kh1_first.pkg` holds
  `xa_ex_0010.mset`, `xa_ex_0010.mdls` and `btltbl.bin` under those plain names; Checked (files)),
  or from the KH1 Steam exe as documented in `kh1-sora.md` / `kh1-sora-data.md`. No game files or
  decompiled code in the repo.
- Units: KH1 units (≈1 cm, y down) and 60 Hz frames, with a frame step like KH1's (`0x14233FBDC`).
- Tests that need game files read them from the path in `KH1_TEST_DATA` and skip when it is unset.
