# KH1 Combat in Re:Chain of Memories (JoaoVitorSCS1999's fork)

A mod for **Kingdom Hearts Re:Chain of Memories** (PC, Steam, KH 1.5+2.5) that replaces its card battles with
real-time combat in the style of **Kingdom Hearts Final Mix**, for Sora and, in Reverse/Rebirth, for Riku.

## The vision

The goal is not to rebuild KH1 inside Re:CoM. It is to **reimplement Re:CoM's combat while keeping
Re:Chain of Memories' own identity**. You get Re:CoM's story, worlds, characters, look and progression with
a cardless, real-time combat. KH1 Final Mix is the reference for how that combat moves and feels.

- **Re:CoM comes first.** Its characters keep their own resources and rules: Riku's Dark Points and Dark
  Mode, his and Sora's sleights, Re:CoM's sleight ideas and its progression. When KH1's way and Re:CoM's
  originality pull apart, Re:CoM's wins.
- **KH1 is the reference, not a copy.** Movement, combos, the command menu, magic, guard, lock-on and the
  HUD follow KH1 Final Mix, and its numbers are kept where they serve the game.
- **Native, not an overlay.** Everything is built into Re:CoM's own systems: its player state table,
  command dispatcher, attack and hit tables, motion slots, effects, sounds and HUD. It is not a separate KH1
  engine running on top.
- **Every system is a choice.** Each setting switches one system between Re:CoM and KH1. With everything on
  Re:CoM, the game plays exactly like Re:CoM.

The mod was created by **PM2CRAZY** ([`PM2CRAZY/KH1-in-ReCom`](https://github.com/PM2CRAZY/KH1-in-ReCom);
release: [`PM2CRAZY/KH1-Combat-in-ReCoM`](https://github.com/PM2CRAZY/KH1-Combat-in-ReCoM)), and all credit for
it goes to them. This fork builds on their work with the vision above: Riku in Reverse/Rebirth and
Re:CoM-flavoured additions. Fixes that also apply to the original are offered to the author as pull requests
([PATCHED.md](PATCHED.md)).

## State of the mod

**In active development and testing.** Most systems are built and covered by tests, but much has not been
played through in game yet. Expect rough edges and report what you find. A full playthrough at both settings
has not been done.

### Sora

- **Movement and attacks**: KH1's movement (run, jump, glide, dodge roll, guard) and KH1's Attack, combos,
  finishers and Counterattack, with KH1's motions and Keyblades built from your own KH1 files.
- **Command menu**: KH1's command menu with MP and magic, items and quick slots, the Moogle shop and
  summons. Shortcuts and Talk / context commands work as in KH1.
- **Abilities**: KH1's abilities with an AP screen, the KH1 specials (Sonic Blade, Strike Raid, Ars Arcanum,
  Ragnarok), Re:CoM's sleights used as abilities and in the field, and Enemy Skills.
- **Battles without cards**: Re:CoM's battle rooms fought in real time. Enemies keep Re:CoM's pace, or use
  KH1's AI where a KH1 version exists, and bosses fight without the card duel.
- **Camera and lock-on**: KH1's or KH2's camera, KH1's lock-on rules, and KH1 FM's Auto Lock and Target
  Lock. With Auto Lock on, attacks aim at the nearest enemy, which the KH1 HUD marks with KH1's orange
  auto-target ring.
- **Effects, sounds and HUD**: KH1's effects and sounds, and the KH1 HUD (HP gauge and MP ring, the command
  menu, party gauges, popups), with Re:CoM's own HUD as an option.
- **Costume**: Sora's Halloween Town costume in every world, unlocked by reaching Halloween Town.

### Riku (Reverse/Rebirth)

- KH1-style movement (`riku_kh1_movement`, off while it is tested), Attack, MP, his own command menu with
  his Sleights list and shortcuts, and the KH1 HUD.
- His own Guard (KH1 Riku's): it blocks, turns shots back and allows Counterattack. He also gets Sora's
  combat rules, items, Enemy Skills and an Abilities page.
- **Dark Mode stays Re:CoM's.** His Dark Points start and end it on Re:CoM's Dark gauge, once the story gives
  it to him. Dark Riku's own sleights cost DP instead of MP and carry Re:CoM's Heartless emblem in the
  menus.
- Cosmetic costumes, changed in place with his transformation:
  - Dark Mode costume and the Keyblade of Heart, unlocked by his first Dark Mode;
  - Halloween Town costume, unlocked by reaching Halloween Town.

### Not done yet

- Marluxia's final card duel, to become a winnable duel built from his patterns in other games.
- KH1's own bosses, and some KH1 enemies (Re:CoM's versions are used).
- KH1's room dust and splashes, and KH1's EXP and munny popups.

What changed in each version is in its [release notes](https://github.com/JoaoVitorSCS1999/KH1-in-ReCom/releases).

## Requirements

- **KINGDOM HEARTS HD 1.5+2.5 ReMIX on Steam (PC)**, with **Kingdom Hearts Final Mix** and
  **Re:Chain of Memories** both installed in the same collection folder. The mod builds its KH1 files from
  your own KH1 and Re:CoM files. No game files are shipped.
- **OpenKH Mods Manager** set up for the PC version, with **Panacea** installed (the Mods Manager's setup
  wizard does both).

## Install (OpenKH, from GitHub)

1. Open the OpenKH Mods Manager and select **Kingdom Hearts Re:Chain of Memories** in its game list (the mod
   is installed for the game selected at that moment).
2. Mods → **Install a new mod** → type `JoaoVitorSCS1999/KH1-in-ReCom` → Install. If you also have the original
   (`PM2CRAZY/...`), untick it: don't enable both at once.
3. Tick the mod and press **Build** (or **Build and Run**).
4. Start Re:CoM. **On the first start the mod builds the KH1 files** from your games. The title screen shows
   "KH1 in Re:CoM: building files from your games... N %" and the step it is on, and waits a minute or two.
   When it says "done", the title goes on by itself. No second Build or restart is needed.
5. Next to the game exe you'll find `kh1-recom.log` (what the mod did), `kh1-recom-build.log` (the build's
   details) and `kh1-recom.ini` (the settings, also in the camp menu's KH1 entry).

**Or from a release's zip**: download **KH1-in-ReCom.zip** from the
[Releases](https://github.com/JoaoVitorSCS1999/KH1-in-ReCom/releases). In the Mods Manager, with Re:CoM selected:
Mods → **Install a new mod** → **Select and install Mod Archive** → the zip. Tick it and press **Build**.

What the first start does: it reads KH1 Final Mix's and Re:CoM's archives (`Image/dt`) and makes the files the
mod plays with (KH1 motions, Keyblades, enemies, effects, sounds, HUD pieces). It keeps them in the mod's
folder (`generated/`) and copies them into OpenKH's mod folder. A later Build removes that copy, and the next
start puts it back in a moment. The build comes back only when an update changes how the files are made,
or when the game itself is updated.

If KH1 Final Mix is not installed, the title says "KH1 FM not found: playing without KH1 files" and the game
goes on without them.

The games used are all in the KH 1.5+2.5 collection: Re:CoM and KH1 FM (required), KH2 FM (Riku's KH2 hair and the
Keyblade models) and Birth by Sleep FM (Riku's spell and Block motions). A missing optional game only leaves that part
with Re:CoM's or KH1's own motions and models; no other collection is needed.

**Updating:** in the Mods Manager, check for updates and update the mod, then Build.

Details for developers: `docs/openkh-github-install.md`.

## Settings

Everything is in `kh1-recom.ini` (each key is explained in the file) and in the camp menu's KH1 entry
(Settings: Combat, Movement, Camera, Effects & Sound). Some of them:

| Key | Default | What it does |
|---|---|---|
| `combat` | `kh1` | the reimplemented combat, or Re:CoM's card battles |
| `hud` | `kh1` | KH1's HUD or Re:CoM's |
| `enemy_pace`, `enemy_ai` | `recom`, `kh1` | a crowd shares Re:CoM's attack budget (or each attacks as if alone); KH1's AI for enemies that have a KH1 version |
| `damage` | `recom` | Re:CoM's or KH1's damage numbers |
| `camera` | `recom` | Re:CoM's, KH1's or KH2's camera |
| `auto_lock` | `on` | KH1's Auto Lock: with no lock, attacks aim at the nearest enemy (the orange ring with the KH1 HUD) |
| `target_lock` | `auto` | KH1's Target Lock: `auto` (R1 again unlocks) or `manual` (unlocks when R1 is let go) |
| `riku_kh1_movement` | `on` | KH1's combat and movement for Riku in Reverse/Rebirth (on out of the box) |
| `riku_costume`, `riku_keyblade`, `sora_costume` | `default`, `soul_eater`, `default` | the cosmetic costumes and Keyblade, once unlocked |
| `riku_hair` | `recom` | Riku's hair: `recom`, `kh2` (KH2 Riku's) or `bbs` (BBS kid Riku's); default attire, Halloween Town and the Dark Form skin (the battle's Dark Mode keeps Re:CoM's); needs the 1.5+2.5 collection |
| `all_unlock` | `off` | **for debug only**: unlocks every costume and cosmetic Keyblade |

## Build

- Tests: `cargo test` (or `tools/test-affected.sh` for only the crates a change touches).
- Windows DLL from Linux: `tools/build-win.sh` (MinGW). It writes `openkh/kh1-recom.dll`, which the root
  `mod.yml` installs. The build bot commits it.
- The asset step by hand: `kh1-recom-build --game <game folder>` (`docs/kh1-recom-assets.md`). Set
  `asset_build = off` in `kh1-recom.ini` so the DLL leaves its files alone.
- Releases: every push to `main` runs the tests, builds the DLL and publishes a GitHub Release whose notes list
  the changes since the previous one (`.github/workflows/patched-release.yml`).
- Design rules: `docs/design-philosophy-instructions.md`. Version history and what each change checked:
  `AGENTS.md`.

This repository contains no game files, no extracted data and no decompiled code.
