# Installing from GitHub through OpenKH, and the first-launch build

Owner, 2026-11-05, verbatim: "I also want people to be able to install the mod from openKH by using
the GitHub". Answers (`design-philosophy-instructions.md`, last entry): players install with OpenKH
Mods Manager's GitHub install (`PM2CRAZY/KH1-in-ReCom`) from the default branch `testing`; the DLL
builds the game-made files on first launch from the player's own KH1 and Re:CoM files, and again
after an update changes the asset step.

## 1–3. OpenKH's GitHub install, Build, and how Panacea finds mod files

How OpenKH and Panacea work in general (not this mod's) is in the KH modding workspace:
`knowledge/OpenKH/openkh-mods-manager-panacea.md` (its §§ 1–4 are this note's old §§ 1–3 and 8.1's
first two points). What applies to this mod:

- **Which branch** (§ 1 there: `main/mod.yml` first, then the default branch):
  - `PM2CRAZY/KH1-in-ReCom`: public, default branch `testing`, the only branch; `main/mod.yml` and
    (before this change) `testing/mod.yml` both answered 404. Checked (GitHub API, 2026-11-05).
    So the install takes `testing`. Typing `PM2CRAZY/KH1-in-ReCom/testing` pins it even if a
    `main` branch ever appears.
- **When the files must be there** (§ 3 there):
  - **Conclusion (0.141.0).** The files must be in `<mod_path>/recom` **before `OnInit` returns**. A worker
    thread can't do that: it would race the listing, and waiting for a thread under the loader lock
    deadlocks (the thread can't start until the lock is free). Written there during `OnInit`, they
    are used in the **same run**: no restart, no Build.
  - **Corrected (0.142.0, § 8):** only the **names** must be there at the listing. Panacea reads a
    listed file's content and size from disk at each open, so empty files made in `OnInit` and filled
    in later work, as long as the game opens none of them before. The build now runs after `OnInit`.

## 4. The DLL and its folders

- Panacea passes `OnInit(<mod_path>/recom)`. That is the live folder Build makes. The DLL is
  `<mod_path>/recom/dll/kh1-recom.dll`, and the game folder is the exe's folder. Checked (source + code).
- The mod's own folder (the clone) is not passed. The DLL finds it (`asset_build::find_mod_folder`):
  OpenKH's folder = `mod_path`'s parent (the default layout), with `mods-manager.yml`'s
  `installedModsPath` / `compiledModPath` read as `ConfigurationService.InstalledModsPath` reads
  them, else `<openkh>/mods/Recom`. In there it takes the `<owner>/<repo>` whose `mod.yml` names
  `kh1-recom.dll` and whose DLL (`openkh/kh1-recom.dll`, or `kh1-recom.dll` in a hand-made folder) has
  the running DLL's size (Build copies it unchanged), else the one named `PM2CRAZY/KH1-in-ReCom`.
  Checked (code, tests). When none is found (a custom `compiledModPath`), the cache goes to the game
  folder instead.

## 5. The flow that was built (0.141.0)

**Repository root** (Checked: test `asset_build::tests::the_repo_root_is_an_openkh_mod`):

- `mod.yml`: `title`, `originalAuthor`, `game: Recom`, `description`, and one asset:
  `dll/kh1-recom.dll` copied from `openkh/kh1-recom.dll`. That is the build bot's file;
  `.github/workflows/build.yml` is unchanged. The old `openkh/mod.yml` (0.1.0 text) is removed.
- **The game-made files are not listed in `mod.yml`.** Listing them would be safe (a missing
  source is skipped), but:
  - the first Build, before the first launch, would print one "File not found … check your mod
    install" warning per file (about 180);
  - the list would have to follow the step's outputs by hand;
  - the DLL has to place them anyway after every Build (below), so the list would add nothing.
- `.gitignore`: `/generated/` (the cache and its stamp), plus the asset step's outputs as a
  hand-made mod folder holds them (`/PLAYER/`, `/ENEMY/`, `/raw/`, `/kh1-*.json`, `/kh1-*.bin`,
  `/kh1-menu-sounds-preview/`, `/kh1-recom.dll`). So a git update never meets them.
- No `icon.png` / `preview.png` yet (none drawn; game art must not go in the repo).

**The DLL at start** (`win::assets::ensure`, from `win::run` right after the exe check, before
anything reads the files):

1. **Decide** (`asset_build::decide`):
   - `asset_build = off` in `kh1-recom.ini` → nothing.
   - The cache holds a stamp for this step and these inputs, with every file at its size →
     **place**.
   - A failed marker for this step and these inputs → log only, no new try.
   - Else → **build**: no build yet, the step changed (a mod update), or the game files changed.
2. **The stamp** (`kh1_recom_assets::stamp`):
   - `step`: a hash of the step's own sources (`kh1-recom-assets`, `kh1-formats`, `kh1-anim`,
     `kh2-formats-lite` `src/`, `\r` dropped), made by the crate's build script `step_hash.rs`;
   - `inputs`: a hash of the names, sizes and times of `Image/dt/Recom.*`, `kh1_*`, `kh2_*`
     `.hed` / `.pkg` and the two exes;
   - one line per file written, with its size.

   It is written last (`.tmp`, then renamed). Any change to the step's code builds again after an
   update; no version number has to be bumped by hand.
3. **Build** (`win::assets::build`): the DLL writes a job file (`kh1-recom-build.job` in the game
   folder) and starts `rundll32.exe "<this DLL>",BuildAssets <job>`. `rundll32` calls the DLL's
   export `BuildAssetsW`: the same code as the game's DLL, so no exe is shipped. The game waits
   for it (30 minutes at most). A process wait needs no lock the helper takes. The helper:
   - opens a **console window** titled "KH1 Combat in Re:CoM: building the KH1 files", with the
     reason, "the game starts by itself when it is done", "please don't close this window", and
     every line of the step, also written to `kh1-recom-build.log` in the game folder;
   - checks the inputs (`Recom.hed`, `kh1_*.hed`, an exe), empties the cache (only its own
     `generated/` folder), and runs the whole step into it (`step::Dest::Root`, without the WAV
     previews);
   - on success writes the stamp, shows "Done: N files in S s. The game starts now." for 2 s, and
     exits 0;
   - on failure writes the failed marker (with the reason) and shows a **message box**. Without KH1
     it says "KINGDOM HEARTS FINAL MIX's files were not found in …\Image\dt. The mod needs KH1 Final
     Mix installed in the same … folder", then "The game starts now without the KH1 files … It
     tries again when the mod or the game is updated". It exits 1.

   The game's log (`kh1-recom.log`) gets the outcome:
   - exit 0: built;
   - exit 1: failed;
   - `0xC000013A`: the window was closed, so it runs again at the next start;
   - any other code (a crash of the step ends only the helper): the failed marker, so it is not
     repeated.

   If `rundll32` can't start (blocked), the step runs inside the game with a console, guarded by a
   failed marker written first and removed on success. A crash there ends the game once, and the
   next start skips the build.
4. **Place** (`stamp::place`): each stamped file is copied from the cache into the live folder
   when it is missing there or differs in size or time. The copy gets the cache file's time. After
   a Build (which wiped them) this restores them at the next start; otherwise nothing is copied.
   This runs before Panacea's listing, so the files are used in that same run.
5. The KH1 features that read the files (`manifest_dirs`, unchanged) then find them in the live
   folder as before. With no files (no KH1, a failed build, `off`) they keep their existing
   fallbacks (Re:CoM's own models, motions, sounds).

**What the player sees** (0.141.0; since 0.142.0 the build shows on the title instead, § 8):
- First launch: a console window for a minute or two, then the game. No restart, no Build.
- After a Build: nothing (a quick copy).
- After an update that changed the step, or a game update: the console window again.
- Without KH1 FM: one message box, then the game without the KH1 files, and no box at later starts.

**Size.** Linking the step costs about **2.1 MB** of DLL. Same toolchain (`-Zbuild-std`): 0.139.4's
code 4,447,217 bytes; 0.141.0, on 0.140.0, 6,583,300 bytes. The bot's 0.139.4 build is 4,659,027 bytes.
Checked (builds here).

**Not checked:**
- in game (no Windows here);
- `rundll32` and `CreateProcess` from inside Panacea's `DllMain`;
- the step end to end (this session's game folder is a sparse copy without `PL0001.SES`, so the
  step stops at its first read, as the command line does);
- how long a full build takes on a player's PC;
- how much the live copy costs after a Build. The step writes several hundred MB on the owner's
  files (Assumption, not measured).

## 6. Steps for players (also in `README.md`)

1. Steam: KINGDOM HEARTS HD 1.5+2.5 ReMIX with KH1 Final Mix and Re:CoM installed.
2. OpenKH Mods Manager, set up for the PC version with **Panacea** installed (its setup wizard).
3. Select **Kingdom Hearts Re:Chain of Memories** in the Mods Manager's game list.
4. Mods → "Install a new mod" → `PM2CRAZY/KH1-in-ReCom` → Install.
5. Tick the mod, **Build** (or "Build and Run").
6. Start Re:CoM. On the first start the title shows "KH1 in Re:CoM: building files from your
   games... N %" and waits (a minute or two), then says "done" and goes on by itself.
7. Updates: the Mods Manager's update for the mod, then Build. The next start rebuilds only when the
   update changed the asset step.

## 7. The owner's own install

- The owner's working folder `mods/Recom/PM2CRAZY/KH1-in-ReCom/` (hand-made, 183 assets) keeps
  working. With 0.141.0's DLL:
  - the first start finds that folder (its `mod.yml` names `kh1-recom.dll`; its `kh1-recom.dll`
    has the running DLL's size);
  - it builds once into its `generated/`;
  - from then on it copies the built files over whatever Build copied, where they differ.
- To keep the manual asset step in charge (e.g. testing a step that isn't pushed yet), set
  `asset_build = off` in `kh1-recom.ini`.
- A fresh GitHub install to test: move that folder out of `mods/Recom/PM2CRAZY/` (or rename it), then
  follow § 6.

## 8. The build's progress in the game window (0.142.0)

Owner, 2026-11-05, verbatim: "I definitely want a progress screen in the game window while
everything loads". Tags: Checked (source / decomp / exe / files / code) or Assumption. Nothing was
run in game (no Windows here).

### 8.1 What Panacea needs, and when

- Panacea reads content and size at each open, and Re:CoM sizes a file through the hooked
  `GetFileSize`, so a name listed with size 0 loads with its real size: see the workspace note's § 4.
- **The new names** (those `Recom.hed` lacks): only the Keyblade groups' files,
  `PLAYER/00GG/PL00GG.MDL` + `raw/…/PL00GG.RTM` for GG = 21..36 and 38..43, and `PL00GG.EET` +
  `raw/…/PL00GG.EFF` for 21..36: 76 names (`keyblade::new_resource_names`). Every other output
  replaces a file `Recom.hed` has. Checked (files: test `new_names_are_not_in_recom_hed` on the
  owner's `Recom.hed`).
- **None of our outputs opens before the title ends.** Of the resource groups (`0x1407EBF20`
  tables, category × 10000 + group), FORM 2 (`FO0002`, the HUD) is linked only from the world groups
  (category 14, groups 100..113); EFFECT 1000 from the player groups 1 / 11 and effect groups 1106 /
  1108; FORM 100 / 109 (camp, shop) and the PLAYER / ENEMY groups are asked for by the camp, the shop,
  rooms and battles. The title asks for (0, 0xD / 0xE) only (`0x1403A1940`), which link FORM 505 /
  507 / 111. Checked (exe: every link row of every table; decomp: the request sites
  `0x14023CD10(…)`). Assumption: no computed request (room / event data) runs before the title.

### 8.2 Where the game waits: Re:CoM's title

- `CTitle` (ctor `0x14039F770`, made by `0x14012DB80`) is a `CFSM` of 14 states. State 0's update
  `0x1403A3770` waits for the loader's queue to empty (`0x14023CA80(0, 0)`), plays `title.lad`
  and goes to state 1. State 1's update `0x1403A3820`: sub-state `+0xAC` 0 plays the title's
  sequence and `Title.VSV`, 10 waits for the sequence's end or a confirm / cancel / Start press
  (`0x14029B490`, `0x14029B410`, `0x14029C240`), 0x14 fades, then state 2 (the menu). Checked
  (decomp + disasm). The title makes its own text as `C2DFnt`s (size 0xF, 0x10 with the language
  global `[0x140AF73B0]+0x274`, centred flags 2, default colour) on `aleart.lad` set 0
  (`0x1403A23D0`; `C2DSeqCtrl` `+0x30` names it `aleart:00` with `%s:%02d` at `0x140606EC8`).
  Checked (decomp + disasm).
- **The hold**: a wrapper on state 1's update (`sites::TITLE_HOLD_HOOK`, 6 bytes; its only
  reference is the ctor's `lea` `0x14039F858`; no branch into the moved bytes: exe scan;
  `TITLE_HOLD_CHECKED` the sub-states). While a build runs, in sub-state 10 Re:CoM's update is not
  run: the title stays on screen, its music plays, and no press is taken (Start / New Game /
  Continue are never reached). Sub-state 0 runs as Re:CoM's, so the title is up first.
- **The lines**: one `C2DSeqCtrl` (priority 0xC, at (256, 330) of the 512 × 384 2D space) with two
  `C2DFnt` text layers made as the title's alert text (attached to `aleart:00`, else `title:00`;
  20 units apart; squeezed to 480 units with the true squeeze). Re:CoM's 2D manager draws them.
  Updated only when a line changes; deleted before the title goes on.
  - Running: "KH1 in Re:CoM: building files from your games... N %" / "Step k of 10: <stage>. The
    title waits until it is done."
  - Done (2.5 s): "KH1 in Re:CoM: done. The KH1 files are ready." / "N files in S s."
  - No KH1: "KH1 FM not found: playing without KH1 files" / "It tries again when the mod or the
    game is updated." (7 s). Other failures: "KH1 in Re:CoM: the build failed: playing without KH1
    files" / "Details: kh1-recom-build.log in the game folder." (7 s).
  - Assumptions: `aleart` (FORM 111, linked from the title's groups) is loaded in sub-state 10;
    priority 0xC draws over the title; the place reads well (no screenshot yet); Re:CoM's text
    draws `%` as is (its expander handles bytes from 0x81).
- **Progress** is counted in the step's stages (`step::STAGES`: reading, motions, Keyblades, the
  HUD and menus, sounds, effects, enemies (one part each), writing, enemy stats) and the copy into
  Panacea's folder (one part per file): `asset_build::percent`, not time.

### 8.3 The flow

1. **`OnInit`** (`win::assets::ensure`): the decision as before (§ 5). `Off` / `Failed` / `Place`
   as 0.141.0, and the rest of the start runs at once. On `Build`:
   - every new name the build may write (§ 8.1, plus the last stamp's names `Recom.hed` lacks) that
     is not in Panacea's folder is made as an empty file;
   - the title's wrapper goes in, and the rest of the DLL's start (everything that reads the built
     files: the camp, movement, attack, HUD, camera …) waits (`win::install_deferred`);
   - a thread starts the build once `OnInit` has returned: no process is made under the loader lock,
     nothing is waited for.
2. **The build**: `rundll32.exe "<dll>",BuildAssets <job>` (no window), in a job object that ends it
   with the game (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`): the step into `generated/`, the stamp, then
   the copy into Panacea's folder; its progress whole into `kh1-recom-build.progress` (a temporary
   file, then a rename); its lines into `kh1-recom-build.log`. A crash ends only the helper (the
   failed marker, so not repeated). If the helper cannot start, the step runs on that thread inside
   the game, guarded by the failed marker.
3. **The title** reads the progress file every 6 frames. At the build's end, on the game's thread:
   the log line, the check that the cache is complete (and a copy of anything missing), the empty
   names the build did not fill removed, the rest of the DLL's start, then the end line, then the
   title goes on.
4. If the title cannot be hooked, the build runs before the game starts, as 0.141.0 (without the
   console; nothing shows until it is done).

Kept from 0.141.0: the stamp, `generated/`, the copy back after a Build, `asset_build = auto | off`,
the failed marker and the start without the KH1 files when they are missing.

### 8.4 Not checked / risks

- In game: all of it (the hold, the lines' look and place, the job object, the helper without a
  console). The log says each step (`assets:` lines).
- Assumption: the rest of the DLL's start is as safe at the title as before the game ran: it
  hooks and patches gameplay code (field, battle, camp, HUD, camera), none of which runs on the
  title, on the game's own thread. Another of Re:CoM's threads (the loader's, whose queue the title
  emptied; sound) running one of those functions at that moment would meet a half-written jump.
- A stale file of an older build in Panacea's folder is never opened before the title ends (§ 8.1),
  so a rebuild after an update is safe as well; an output a future step writes under a new name
  outside `new_resource_names` would not be in the listing until the next start.

## 9. The release branch (`main`, 2026-11-05)

- `main` holds only the release files (`mod.yml`, `openkh/kh1-recom.dll`, `README.md`, `.gitignore`, `VERSION`), with no history shared with `testing`. OpenKH's GitHub install looks for `main/mod.yml` first (§ 1), so `PM2CRAZY/KH1-in-ReCom` installs the release; `PM2CRAZY/KH1-in-ReCom/testing` installs the test builds.
- An install cloned from `testing` keeps updating from `testing` (OpenKH fetches and resets the branch it cloned: Assumption, from § 1's source reading).
- `main` is written only when the owner says so (`design-philosophy-instructions.md`, "Release build"): a new commit on top (never a force-push) replacing the five files with the chosen `testing` version's. The build bot does not run on `main`.
- Releases: 0.142.0 (from `testing` 1d82190); 0.143.0 (from `testing` f209db2, owner: "Push that into the release repo"); 0.143.1 (from `testing` 3da26a4, the KH1 HUD default).
- The release also lives in its own repository, `PM2CRAZY/KH1-Combat-in-ReCoM` (`main`), with the same five files; `PM2CRAZY/KH1-Combat-in-ReCoM` is the name players install. The DLL finds its mod folder by its own size first (`asset_build::find_mod_folder`), so the other folder name works.
