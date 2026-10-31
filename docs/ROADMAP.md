# Roadmap: KH1 combat in Re:Chain of Memories

The goal: with every setting on KH1, Sora in Re:CoM moves, fights, casts and summons exactly as
in KH1 FM, and KH1-equivalent enemies act 1:1. Re:CoM itself runs all of it (native), and with
the settings on Re:CoM, the game is untouched. Decisions so far are in
`design-philosophy-instructions.md`. Each milestone is a version or a run of versions. A milestone
is done when its "done when" check passes.

The KH1 Rust rewrite (`kh1-rewrite.md`) is the oracle: each Re:CoM feature is checked against it,
and it is checked against KH1 traces.

## Done

- **M0, groundwork:**
  - Steam decomps of Re:CoM (made here) and KH1.
  - System maps for both games: states, card path, motion, hits, damage, battle entry, timing.
    Both games tick in 60 Hz frames, and a KH1 frame is ×50 in Re:CoM.
- **M0.5, oracle + demo:**
  - The KH1 Rust rewrite of movement and Attack, with the playground.
  - KH1's posing reproduces Square Enix's own Re:CoM bakes to ≤0.02°.
- **M0.6, mod skeleton:** a DLL that Panacea loads, which checks the Steam build and future hook
  sites. Build bot in place.

## Where we are (2026-10-30, version 0.7.0)

| Milestone | State |
|---|---|
| M1 oracle vs KH1 | Tool built. No real-KH1 recordings yet, so "1:1" rests on the KH1 decomp and files, not traces. |
| M2 motions | Done in game: Re:CoM's clips where it has them, 69 KH1 clips added. |
| M3 Config | Done in game (row, panel, help line with the version). |
| M4 movement | Built; fixes in game (springs 0.6.0, roll/guard position 0.6.1, ledge grab 0.6.3 untested). Not yet diffed against the oracle ×50. |
| M5 Attack | Combos, windows and press rules match KH1 in the owner's traces. 0.7.0: KH1 damage and HP on 31 Re:CoM enemy classes (each world's KH1 record), Sora's KH1 Strength by level (keep Rod, give up Shield); not seen in game yet. Next: Guard's block, recoil and Counterattack, with new Re:CoM enemy reactions behind a toggle (decision 20). MP charge comes with M7. |
| M6 battles without cards | Stage 1 built, not seen in game (`recom-m6-battles.md`): at KH1, Heartless and Heartless bosses attack without cards (the card play does message 2's work), several at once (think gate, grant and crowd pace switched per battle). Next: stage 2 (no deck / deck HUD for Sora), stage 3 (bosses, P5 first). |
| M7–M14 | Not started. |

## Phase 1: foundations inside Re:CoM

Status (2026-10-30): everything for M1–M3 is built and installed (version 0.4.0). Each is waiting
on its in-game check (`testers/kh1-traces`, `testers/recom-phase1`).

**M1. Verify the oracle against KH1** (before building on it)
- A KH1 trace tool: a small KH1-side logger (state, motion and frame, position, velocity,
  combo counter, hits) in the same CSV format as the playground's `--trace`.
- A tester records a few set moves in KH1 and in the playground; we diff them and fix them.
- Also: confirm Panacea loads `kh1-recom.dll` (first launch log).
- Done when the traces match frame for frame for walk/run, jump (normal and High Jump), glide,
  roll, guard, and the ground and air combos.

**M2. The asset build step: KH1 motions in Re:CoM** (owner, 2026-10-30: Re:CoM's own clips
wherever Re:CoM has the motion; only the KH1 motions it lacks are converted; no switch)
- A converter run on the player's own files (KH1 `xa_ex_0010.mset` / `.mdls` → Re:CoM 60 fps
  clips through `kh1-anim`). It writes an extended `PL0001.ANN` (Re:CoM's 161 slots untouched,
  KH1's after) into Panacea's `Recom` folder, plus the Keyblade's motions.
- New rows in Sora's action table, so Re:CoM can play the new slots by name.
- Done when, with a debug switch, Re:CoM's Sora plays KH1's idle, run and swing in game, and a
  pose capture matches the converter.

**M3. The Config switch**
- KH1 / Re:CoM toggles in Re:CoM's own Config menu, drawn by its menu code. `kh1-recom.ini`
  remains as a backup.
- Done when the toggles show in Config, are saved, and switch behaviour on the next room load.

## Phase 2: KH1 Sora in Re:CoM

**M4. KH1 movement (field and battle)**
- New actions in Re:CoM's player action switch, with KH1's numbers from `kh1-sora-data.md`:
  - walk/run speed snap and turn rate;
  - the jump arc, short jump and fall cap;
  - High Jump, glide and Superglide;
  - Dodge Roll and Guard;
  - landing lock.
- Re:CoM's own collision and floor are used.
- Done when a Re:CoM trace matches the oracle's (×50 time) in the field and in a battle room.

**M5. The KH1 Attack, with no cards**
- A cardless Attack command in Re:CoM's battle and field states.
- KH1's combo picker: rows, ranges, cones, alternation, finishers, air combo, combo window,
  Combo Plus and Combo Master.
- New attack rows with KH1's hit windows.
- KH1's Keyblade hit shape: a chain of spheres on the blade, added as a Re:CoM collision shape.
- KH1's damage formula and reactions (flinch, launch knockback) applied through Re:CoM's damage
  path.
- The field swing triggers Re:CoM's battle entry and first-strike bonus (kept, as decided).
- Done when the combo traces match the oracle, and damage on a Re:CoM enemy matches KH1's
  formula with that enemy's stats.

**M6. Battles without cards**
- At KH1 setting, the deck and card UI are hidden in battle and there are no card breaks.
- Re:CoM-only enemies keep their behaviour without card plays (option a).
- Re:CoM's events, scripts and bosses still get every card value they read. This is the main
  risk, so each card-checking fight gets tested.
- A KH1 HP/MP gauge, drawn by Re:CoM's HUD code.
- Done when every world's normal battles and every boss can be finished at KH1 setting, with no
  soft-locks.

## Phase 3: KH1's battle systems

**M7. Command menu, MP and magic**
- KH1's command menu with free Attack, Magic and Items, built into Re:CoM's HUD.
- MP, including MP refill on hits: 3 / 6 / 24 per hit, 30 units = 1 MP.
- KH1's Fire, Blizzard, Thunder, Cure, Gravity, Stop and Aero, with their levels. Fira, Firaga
  and the rest become KH1's magic levels (decided).
- Done when MP costs, cast timings and spell damage match KH1.

**M8. Items and summons**
- KH1's items and quick slots.
- KH1's summons: Simba, Genie, Bambi, Dumbo, Tinker Bell and Mushu. Cloud stays removed for now.
- Done when the effects, durations and costs match KH1.

**M9. Abilities**
- KH1's abilities, inside Re:CoM's level-up progression for now (decided):
  - Sonic Blade, Strike Raid, Ars Arcanum and Ragnarok (from the sleights);
  - the finisher variants (Stun Impact, Gravity Break, Zantetsuken, Blitz, Ripple Drive);
  - Sliding Dash and Vortex;
  - Glide, High Jump, Dodge Roll and Guard, Combo Plus / Air Combo Plus, Combo Master.
- How the player equips them: an open design question for the owner.

## Phase 4: KH1 enemies

**M10. Shadow, the pilot**
- KH1's Shadow built as native Re:CoM enemy states and attack data:
  - KH1's AI, attacks, hit and hurt shapes, stats and reactions;
  - its motions converted by the build step (the same pipeline as M2, for enemies).
- Done when a trace of the Shadow matches KH1 frame for frame.

**M11. Every KH1-equivalent enemy**
- The same pipeline per enemy: Soldier, Large Body, the Nocturnes and Operas, Powerwild,
  Bouncywild, Air Soldier, Bandit, Fat Bandit, Barrel Spider, Search Ghost, Sea Neon, Screwdiver,
  Aquatank, Wight Knight, Gargoyle, Pirate, Air Pirate, Darkball, Defender, Wizard, Neoshadow,
  White Mushroom, Black Fungus, Creeper Plant, Tornado Step, Crescendo, Card Soldier, and the
  rest of Re:CoM's list.
- Then the KH1 bosses that Re:CoM shares: Guard Armor, Trickmaster, Darkside, Parasite Cage,
  Oogie Boogie, Ursula, Dragon Maleficent, Hades.
- Done when each enemy's trace matches KH1.

## Phase 5: feel and finish

**M12. Camera and lock-on**
- KH1's camera, lock-on and target switching. KH1's Scan HP bar is a scope item to ask about.

**M13. Effects, sounds and voices**
- KH1's swing trails, hit effects, spell effects, sounds and voices, built into Re:CoM's effect
  programs and sound banks by the build step.

**M14. Full-game pass and release**
- A full playthrough at KH1 setting and at Re:CoM setting.
- Performance check.
- An OpenKH Mods Manager package, with the build step run on install.

## Later (decided "not now")

Persistent party (Donald and Goofy with AI); KH1 stat growth, AP and abilities as progression;
Cloud; enemy cards; sleights with no KH1 equivalent; Riku's story at KH1 setting.

## Open questions to settle on the way

- How abilities are equipped while progression is Re:CoM's (M9).
- Re:CoM's battle rooms are arenas. Check that KH1 jumps, glides and Sonic Blade travel fit
  them (M4/M5).
- Bosses that read card values (M6): the full list, found by testing.
