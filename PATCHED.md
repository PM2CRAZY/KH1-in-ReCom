# KH1 Combat in Re:CoM: patched build

This fork's `main` is the author's
[`PM2CRAZY/KH1-in-ReCom`](https://github.com/PM2CRAZY/KH1-in-ReCom) `testing` plus fixes. Each
fix is also offered to the author as a pull request from its own `fix/...` branch:

- **KH1 HUD with KH1 texture mods**
  ([PM2CRAZY/KH1-in-ReCom#2](https://github.com/PM2CRAZY/KH1-in-ReCom/pull/2), branch
  `fix/hud-texture-mod-scale`). With a texture mod in KH1's archives (found with TieuLink's
  [HD Textures](https://www.nexusmods.com/kingdomheartsrechainofmemories/mods/2)), the KH1 HUD was
  skipped, or Re:CoM crashed after the title. Now each HUD picture is fitted to KH1 FM PC's own scale. Stock
  installs give the same files as the original.
- **Button glyphs follow the controller type**
  ([PM2CRAZY/KH1-in-ReCom#3](https://github.com/PM2CRAZY/KH1-in-ReCom/pull/3), branch
  `fix/shortcut-pad-glyphs`). The shortcut panel and the context tab now show Xbox, PlayStation or
  generic-pad glyphs, as Re:CoM's controller setting says.

All credit for the mod goes to its author (PM2CRAZY). Everything else (requirements, settings,
first-launch build) is as in [README.md](README.md).

## Install (OpenKH)

1. In the Mods Manager, select **Kingdom Hearts Re:Chain of Memories**.
2. Mods → **Install a new mod** → `JoaoVitorSCS1999/KH1-in-ReCom`.
3. Tick it, and untick the original `PM2CRAZY/...` if you have it. Press **Build**.
4. Start Re:CoM. The title builds the KH1 files by itself.

Already installed: Mods → check for updates → update the mod, then **Build**.

Or from a release's **KH1-in-ReCom.zip** (`mod.yml` and the DLL): Mods → **Install a new mod** →
**Select and install Mod Archive** → the zip; tick it, **Build**. Update by installing the newer zip.

## Releases

`.github/workflows/patched-release.yml` runs on every push to `main`. It:

1. runs `cargo test --workspace` and builds the Windows DLL (`tools/build-win.sh`);
2. commits the DLL to `main`, so OpenKH's "Check for updates" sees it;
3. publishes a release `v<VERSION>-patched-<commit>` with the OpenKH install steps and
   `KH1-in-ReCom.zip` (`mod.yml`, the DLL, README, PATCHED) for OpenKH's zip install.

If the tests or the build fail, nothing is published, and GitHub emails the fork's owner.

**Taking a new version of the author's:** Actions → patched-release → Run workflow, with *Merge
the author's latest testing* ticked. The run merges it into `main` (keeping this build's DLL), then
tests, builds and releases. If the author's changes conflict with a fix, the run stops without
publishing.

## Contributing a fix

1. Branch `fix/<name>` from the author's `testing`, and open a pull request against
   `PM2CRAZY/KH1-in-ReCom` `testing`.
2. Merge the branch into this fork's `main`: the push builds and releases it.
