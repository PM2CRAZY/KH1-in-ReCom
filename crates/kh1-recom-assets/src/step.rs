//! The whole asset step (`kh1-recom-build`, and the DLL's first-launch build: `docs/openkh-github-install.md`). Reads the player's own KH 1.5+2.5 folder (KH1's
//! and Re:CoM's archives) and writes `<mod>/recom/kh1-motions.json` (the KH1 entry → Re:CoM
//! slot map) and, when some KH1 motion is not in Re:CoM, `<mod>/recom/PLAYER/0001/PL0001.ANN`
//! and `<mod>/recom/PLAYER/0002/PL0002.ANN` (both extended) into Panacea's Re:CoM mod folder.
//! It also converts KH1's Keyblades, and the six KH2 ones Re:CoM has beyond KH1's (from the
//! player's KH2 FM archives), into Re:CoM weapon models (`PLAYER/00GG/PL00GG.MDL` and
//! `raw/PLAYER/00GG/PL00GG.RTM`, `docs/recom-keyblade-models.md`) and writes `kh1-stats.json`.
//! For the MP bar (M7 stage 7.2) it writes `raw/FORM/0002/FO0002.SPR` (Re:CoM's HUD sprite file
//! with KH1's MP gauge colours as new groups) and `kh1-mp-bar.json` (`docs/recom-m7-mp-bar.md`).
//! For the command menu (M7 stage 7.3) the same `FO0002.SPR` also gets Re:CoM's row plates, and
//! `kh1-command-menu.json` holds the menu's words from KH1's own `Word.bin` in every language
//! (`docs/recom-m7-command-menu.md`). For the KH1 HUD (decision 39, H1) it writes `kh1hud.lad` into
//! the same `FO0002.SPR`, `raw/FORM/0002/FO0002.RTM` with `kh1hud.imz`, and `kh1-hud.json`
//! (`docs/recom-kh1-hud.md`). For the command menu's sounds it writes `kh1-menu-sounds.bin` (KH1's
//! system sound bank `se000.win32.scd` from `amusic/se000.dat`, unchanged) and
//! `kh1-menu-sounds.json`, plus the menu entries as WAVs in `kh1-menu-sounds-preview/` for
//! listening (`docs/kh1-recom-assets.md`). For the camp's item rows it writes
//! `raw/FORM/0100/FO0100.SPR` (Re:CoM's camp sprite file with its green row carrying the item
//! symbol as a new group of `camp:00`) and `kh1-camp-rows.json` (`docs/recom-m8-items.md`).
//! For KH1's Shadow (M10) it writes `ENEMY/0001/EN0001.ANN`, `.ASD` and `.SES` (Re:CoM's Shadow
//! files with KH1's Shadow motions, attack records and their sounds added) and `kh1-shadow.json`
//! (`docs/recom-m10-shadow.md`). For each M11 enemy (`kh1_recom_assets::enemy::SPECS`, the Soldier
//! first) it writes `ENEMY/NNNN/ENNNNN.ANN`, `.ASD`, `.SES` and `kh1-<enemy>.json`
//! (`docs/recom-m11-enemies.md`). For M13's KH1 effects it writes `raw/EFFECT/1000/EF1000.EFF`
//! (Re:CoM's effect container Sora's group loads, with KH1's Sora effect set added under a `kh1_`
//! name and its PC textures, and its `.ese` sound rows) and `kh1-effects.json`; for M13's KH1
//! sounds `kh1-sounds.bin` (KH1's own SCDs, unchanged) and `kh1-sounds.json` (the KH1 → Re:CoM
//! id map) (`docs/recom-m13-plan.md`).

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use crate as kh1_recom_assets;
use kh1_recom_assets::ann::Ann;
use kh1_recom_assets::build::{build_with_held, BuildOptions, HeldInput, Inputs, RECOM_SORA_SLOTS};
use kh1_recom_assets::install;
use kh1_recom_assets::keyblade::{convert, convert_kh2, group_for_card, kh2_template_distance, Converted, KeybladeModel, KH2_TEMPLATE_MATCH};
use kh1_recom_assets::kh2_keyblades;
use kh1_recom_assets::mdl::Mdl;
use kh1_recom_assets::stats::{Kh2Row, KH1_KEYBLADES};

/// The step's report lines go through [`say`] (stdout, or the sink [`set_sink`] gave).
macro_rules! println {
    ($($t:tt)*) => {
        $crate::step::say(&format!($($t)*))
    };
}

/// Where the step writes.
#[derive(Clone, Debug)]
pub enum Dest {
    /// Panacea's `mod_path` (`None`: `mod_path=` of `<game>/panacea_settings.txt`); the files go
    /// under its `recom` folder.
    ModDir(Option<PathBuf>),
    /// `--out <file.ANN>`: Sora's pack at this path, everything else next to it.
    Out(PathBuf),
    /// A Re:CoM root itself (what `<mod_path>/recom` is): the DLL's first-launch build.
    Root(PathBuf),
}

/// What to run.
#[derive(Clone, Debug)]
pub struct Options {
    /// The KH 1.5+2.5 folder (holds `Image/dt` and the exes).
    pub game: PathBuf,
    /// Where the files go.
    pub dest: Dest,
    /// Convert and report, write nothing.
    pub dry_run: bool,
    /// Also write the menu sounds as WAVs (`kh1-menu-sounds-preview/`, for listening only).
    pub previews: bool,
    /// The motion converter's options (`--loop` / `--once`).
    pub build: BuildOptions,
}

impl Options {
    /// The defaults for a game folder and a destination (previews on, as the command line).
    pub fn new(game: PathBuf, dest: Dest) -> Self {
        Options { game, dest, dry_run: false, previews: true, build: BuildOptions::default() }
    }
}

/// What a run did.
#[derive(Clone, Debug, Default)]
pub struct Report {
    /// Every file written, in order (a file written twice is listed once).
    pub written: Vec<PathBuf>,
}

type Sink = Box<dyn Fn(&str) + Send + Sync>;
static SINK: Mutex<Option<Sink>> = Mutex::new(None);

/// The step's stages in the order it runs them, as the DLL's first-launch build shows them in the
/// game window (`docs/openkh-github-install.md` § 8). The progress is counted in these stages (and
/// the enemies one by one), not in time.
pub const STAGES: &[&str] = &["reading your game files", "motions", "Keyblades", "the HUD and menus", "sounds", "effects", "enemies", "writing the files", "enemy stats"];

/// Indices into [`STAGES`].
pub mod stage {
    /// Reading the archives and the exe's key table.
    pub const READ: usize = 0;
    /// Matching and converting KH1's motions.
    pub const MOTIONS: usize = 1;
    /// KH1's and KH2's Keyblade models.
    pub const KEYBLADES: usize = 2;
    /// The MP bar, the command menu, the KH1 HUD, the menu sounds, the camp row, the shop tabs.
    pub const HUD: usize = 3;
    /// KH1's sound banks.
    pub const SOUNDS: usize = 4;
    /// KH1's effect sets.
    pub const EFFECTS: usize = 5;
    /// KH1's Shadow and the M11 enemies (one part each).
    pub const ENEMIES: usize = 6;
    /// The files written.
    pub const WRITE: usize = 7;
    /// `kh1-stats.json`.
    pub const STATS: usize = 8;
}

/// One progress report: stage `stage` of [`STAGES`] began, or part `k` of `n` of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress {
    /// Index into [`STAGES`].
    pub stage: usize,
    /// `(k, n)`: part `k` (from 0) of `n` of the stage.
    pub part: Option<(usize, usize)>,
}

type ProgressSink = Box<dyn Fn(Progress)>;

thread_local! {
    static WRITTEN: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) };
    static PROGRESS: RefCell<Option<ProgressSink>> = const { RefCell::new(None) };
}

/// Sends this thread's [`run`]'s progress reports to `sink` (`None`: no reports).
pub fn set_progress(sink: Option<ProgressSink>) {
    PROGRESS.with(|p| *p.borrow_mut() = sink);
}

fn progress(stage: usize, part: Option<(usize, usize)>) {
    PROGRESS.with(|p| {
        if let Some(f) = p.borrow().as_ref() {
            f(Progress { stage, part });
        }
    });
}

/// Sends the step's report lines to `sink` instead of stdout (`None`: stdout again).
pub fn set_sink(sink: Option<Sink>) {
    if let Ok(mut g) = SINK.lock() {
        *g = sink;
    }
}

/// One report line.
pub fn say(line: &str) {
    if let Ok(g) = SINK.lock() {
        if let Some(f) = g.as_ref() {
            f(line);
            return;
        }
    }
    use std::io::Write;
    let _ = writeln!(std::io::stdout(), "{line}");
}

/// Runs the whole step.
pub fn run(opt: &Options) -> Result<Report, String> {
    WRITTEN.with(|w| w.borrow_mut().clear());
    let r = run_inner(opt);
    let written = WRITTEN.with(|w| std::mem::take(&mut *w.borrow_mut()));
    r.map(|()| Report { written })
}

fn run_inner(opt: &Options) -> Result<(), String> {
    let game = opt.game.clone();
    let dry = opt.dry_run;
    let opt_build = &opt.build;
    let t0 = Instant::now();
    progress(stage::READ, None);
    let keys = install::keys(&game).map_err(|e| format!("key table: {e}"))?;
    let arc = install::recom_archive(&game).map_err(|e| e.to_string())?;
    let ann = install::read_recom(&arc, &keys, install::SORA_ANN).map_err(|e| format!("{}: {e}", install::SORA_ANN))?;
    let mdl = install::read_recom(&arc, &keys, install::SORA_MDL).map_err(|e| format!("{}: {e}", install::SORA_MDL))?;
    let ses = install::read_recom(&arc, &keys, install::SORA_SES).map_err(|e| format!("{}: {e}", install::SORA_SES))?;
    // Sora's Halloween Town pack (group 11), extended with the same clips (`extend_costume_pack`).
    let halloween = match arc.find(install::HALLOWEEN_ANN) {
        Some(_) => Some(install::read_recom(&arc, &keys, install::HALLOWEEN_ANN).map_err(|e| format!("{}: {e}", install::HALLOWEEN_ANN))?),
        None => None,
    };
    let halloween_ses = match arc.find(install::HALLOWEEN_SES) {
        Some(_) => Some(install::read_recom(&arc, &keys, install::HALLOWEEN_SES).map_err(|e| format!("{}: {e}", install::HALLOWEEN_SES))?),
        None => None,
    };
    let (mset, mdls) = install::read_kh1(&game, keys.clone()).map_err(|e| format!("KH1 files: {e}"))?;
    // M9 stage 9c.1: KH1's specials' motions (Sonic Blade), as new clips; without the file the
    // runtime offers no special.
    let xl_file = match install::read_kh1_xl(&game, keys.clone()) {
        Ok(f) => Some(f),
        Err(e) => {
            println!("note: {}: {e}; Sonic Blade's motions are not built (no KH1 specials in game)", install::XL_LIMIT);
            None
        }
    };
    let xl = match &xl_file {
        Some(f) => Some(kh1_recom_assets::build::xl_motion_set(f).map_err(|e| e.to_string())?.to_vec()),
        None => None,
    };
    // M11 (the Neoshadow's grab): KH1's held clip from the Neoshadow's files; without them the Neoshadow misses its
    // grab (KH1's other branch).
    let held_in = match install::kh1(&game, keys.clone()).map_err(|e| e.to_string()).and_then(|i| Ok(HeldInput { mset: i.read("xa_ex_2420.mset").map_err(|e| e.to_string())?, mdls: i.read("xa_ex_2420.mdls").map_err(|e| e.to_string())? })) {
        Ok(h) => Some(h),
        Err(e) => {
            println!("note: xa_ex_2420.mset / .mdls: {e}; KH1's held clip is not built (the Neoshadow misses its grab)");
            None
        }
    };
    // The Keyblade (CPlayerWeapon, resource group 2): converted when both files are there.
    let weapon = match (arc.find(install::KEYBLADE_ANN), arc.find(install::KEYBLADE_MDL)) {
        (Some(_), Some(_)) => Some((
            install::read_recom(&arc, &keys, install::KEYBLADE_ANN).map_err(|e| format!("{}: {e}", install::KEYBLADE_ANN))?,
            install::read_recom(&arc, &keys, install::KEYBLADE_MDL).map_err(|e| format!("{}: {e}", install::KEYBLADE_MDL))?,
        )),
        _ => {
            println!("note: {} / .MDL not in Recom.hed; the Keyblade pack is not built", install::KEYBLADE_ANN);
            None
        }
    };
    let pl0002_mdl = weapon.as_ref().map(|w| w.1.clone());
    let t_read = t0.elapsed();
    let mut sources = vec![
        ("PL0001.ANN sha256", kh1_formats::egs::sha256_hex(&ann)),
        ("PL0001.MDL sha256", kh1_formats::egs::sha256_hex(&mdl)),
        ("xa_ex_0010.mset sha256", kh1_formats::egs::sha256_hex(&mset)),
        ("xa_ex_0010.mdls sha256", kh1_formats::egs::sha256_hex(&mdls)),
    ];
    if let Some(f) = &xl_file {
        sources.push(("xl_limit.dat sha256", kh1_formats::egs::sha256_hex(f)));
    }
    if let Some(h) = &held_in {
        sources.push(("xa_ex_2420.mset sha256", kh1_formats::egs::sha256_hex(&h.mset)));
        sources.push(("xa_ex_2420.mdls sha256", kh1_formats::egs::sha256_hex(&h.mdls)));
    }
    if let Some((a, m)) = &weapon {
        sources.push(("PL0002.ANN sha256", kh1_formats::egs::sha256_hex(a)));
        sources.push(("PL0002.MDL sha256", kh1_formats::egs::sha256_hex(m)));
    }
    let t1 = Instant::now();
    progress(stage::MOTIONS, None);
    // M9 9a: the KH1 sound cues of the combo modifiers' motions (their SES entries, below).
    let kh1_set = kh1_formats::mset::MotionSet::parse(&mset).map_err(|e| format!("xa_ex_0010.mset: {e}"))?;
    let built = build_with_held(&Inputs { mset, mdls, ann, mdl, weapon, xl }, held_in.as_ref(), opt_build).map_err(|e| e.to_string())?;
    let t_conv = t1.elapsed();
    let n_recom = built.recom_entries();
    let n_new = built.new_slots();
    let n_edit = built.entries.iter().filter(|e| e.match_kind == Some(kh1_recom_assets::matching::MatchKind::Edited)).count();
    println!(
        "{} KH1 motions: {} play Re:CoM's own clips ({} bakes, {} with Re:CoM edits), {} get new slots {}; read {:.2}s, match + convert {:.2}s",
        built.entries.len(),
        n_recom,
        n_recom - n_edit,
        n_edit,
        n_new,
        if built.changed { format!("{}..{} (pack {} bytes, {} slots)", built.base_slots, built.ann.slots.len() - 1, built.bytes.len(), built.ann.slots.len()) } else { "(none: Re:CoM's packs are left as they are)".into() },
        t_read.as_secs_f64(),
        t_conv.as_secs_f64()
    );
    if !built.xl.is_empty() {
        println!("KH1's specials (xl_limit.dat): Sonic Blade's wind-up, dash, finisher, Strike Raid's wind-up, throws, catches and wait, Ars Arcanum's wind-up, hits and finisher on new slots {:?}", built.xl.iter().map(|e| (e.slot, e.name.as_str(), e.frames)).collect::<Vec<_>>());
    }
    if let Some((e, blend)) = &built.held {
        println!("held (M11, the Neoshadow's grab): KH1's held clip (xa_ex_2420.mset player set, request {}) on new slot {} {}, {} frames, blend {blend}", e.kh1_request.unwrap_or(0), e.slot, e.name, e.frames);
    }
    for e in built.entries.iter().filter(|e| !e.ambiguous.is_empty()) {
        println!("note: KH1 entry {} also matches Re:CoM slots {:?} (chose {})", e.kh1_entry, e.ambiguous, e.slot);
    }
    if let Some(w) = &built.weapon {
        println!("Keyblade: {} new clips; pack {} bytes ({} slots)", n_new, w.bytes.len(), w.ann.slots.len());
    }
    // KH1's Keyblades as Re:CoM weapon models (decision 25).
    let t2 = Instant::now();
    progress(stage::KEYBLADES, None);
    let mut keyblades = convert_keyblades(&game, keys.clone(), pl0002_mdl.as_deref())?;
    // The six from KH2 (decision 25): KH2's models, Strength and Magic, reach from the blade.
    let (kh2_rows, kh2_models) = kh2_keyblades_step(&game, keys.clone(), pl0002_mdl.as_deref())?;
    for r in &kh2_rows {
        let k = &r.keyblade;
        println!(
            "KH2 Keyblade card {:#x} {}: item {:#x}, stat row {} (Strength +{}, Magic +{}), obj/{}.mdlx, blade {:.4} m (Kingdom Key {:.4} m)",
            k.card, k.name, k.item, k.stat, k.strength, k.magic, k.model, r.blade_length, r.kk_blade_length
        );
    }
    keyblades.extend(kh2_models);
    for (m, c) in &keyblades {
        match c {
            Some(c) => println!("Keyblade card {} → group {}: {} (MDL {} bytes, RTM {} bytes)", m.card, m.group, c.report.line(), c.mdl.len(), c.rtm.len()),
            None => println!("Keyblade card {} → group {}: Re:CoM's own PL0002 ({:?})", m.card, m.group, m.textures),
        }
    }
    println!("Keyblades: {} converted in {:.2}s", keyblades.iter().filter(|k| k.1.is_some()).count(), t2.elapsed().as_secs_f64());
    // The MP bar (M7 stage 7.2): built here, written with the other files below.
    progress(stage::HUD, None);
    let mp_bar = match mp_bar_build(&game, keys.clone(), &arc) {
        Ok(b) => Some(b),
        Err(e) => {
            println!("note: MP bar not built: {e}");
            None
        }
    };
    // The command menu's words (M7 stage 7.3): KH1's own, from the player's btltbl.bin assets.
    let menu = match (&mp_bar, command_menu_build(&game, keys.clone(), &arc)) {
        (Some(_), Ok(m)) => Some(m),
        (None, _) => {
            println!("note: command menu not built: it needs the MP bar's FO0002.SPR (its plates are added there)");
            None
        }
        (_, Err(e)) => {
            println!("note: command menu not built: {e}");
            None
        }
    };
    // The KH1 HUD set (decision 39, stage H1): kh1hud.lad into the bar's FO0002.SPR, kh1hud.imz
    // into FO0002.RTM.
    let mut mp_bar = mp_bar;
    let hud = match mp_bar.as_mut() {
        Some(b) => match kh1_hud_build(&game, keys.clone(), &arc, b) {
            Ok(h) => Some(h),
            Err(e) => {
                println!("note: KH1 HUD not built: {e}");
                None
            }
        },
        None => {
            println!("note: KH1 HUD not built: it needs the MP bar's FO0002.SPR");
            None
        }
    };
    // KH1's command menu sounds (owner, 2026-11-01): KH1's system sound bank for Re:CoM's banks.
    let menu_sounds = match menu_sounds_build(&game, keys.clone()) {
        Ok(m) => Some(m),
        Err(e) => {
            println!("note: KH1 menu sounds not taken: {e} (the menu keeps Re:CoM's sounds)");
            None
        }
    };
    // The camp list's green row with the item symbol (owner, 2026-11-01): a group of camp:00.
    let camp_rows = match camp_rows_build(keys.clone(), &arc) {
        Ok(c) => Some(c),
        Err(e) => {
            println!("note: camp item row not built: {e} (the item rows keep Re:CoM's green row with the Mickey symbol)");
            None
        }
    };
    // M8d: the Moogle shop's pack screen with a fifth tab (the Items tab): 13 groups of m_shop:01.
    let shop_tabs = match shop_tabs_build(keys.clone(), &arc) {
        Ok(c) => Some(c),
        Err(e) => {
            println!("note: shop tabs not built: {e} (the shop keeps Re:CoM's four-tab window; the Items tab's cursor sits right of them)");
            None
        }
    };
    // M13 stage 2: KH1's own sound banks (Sora's SCDs, the Kingdom Key's from the world packs).
    progress(stage::SOUNDS, None);
    let sounds = match sounds_build(&game, keys.clone()) {
        Ok(s) => Some(s),
        Err(e) => {
            println!("note: KH1 sound banks not built: {e} (Re:CoM's sounds stay)");
            None
        }
    };
    // KH1's system bank (ids under 1000: the effect programs' sounds) as the menu sounds give it.
    let system_sounds = menu_sounds.as_ref().and_then(|(bin, _, _)| kh1_recom_assets::menu_sounds::scd_info(bin).ok()).map(|i| i.sounds as u32);
    // M13 stage 1: KH1's Sora effect set in the effect container Sora's group loads.
    progress(stage::EFFECTS, None);
    let effects = match effects_build(&game, keys.clone(), &arc, system_sounds) {
        Ok(e) => Some(e),
        Err(e) => {
            println!("note: KH1 effects not added: {e} (Re:CoM's effect container is left as it is)");
            None
        }
    };
    // KH1's Shadow (M10): its motions, attacks and the behaviour's numbers in Re:CoM's Shadow.
    let enemy_parts = kh1_recom_assets::enemy::SPECS.len() + 1;
    progress(stage::ENEMIES, Some((0, enemy_parts)));
    let shadow = match shadow_build(&game, keys.clone(), &arc) {
        Ok(b) => {
            println!(
                "Shadow (M10): {} requests on {} clips ({} Re:CoM bakes), {} attack records: {}",
                b.clips.len(),
                {
                    let mut s: Vec<usize> = b.clips.iter().map(|c| c.slot).collect();
                    s.sort_unstable();
                    s.dedup();
                    s.len()
                },
                b.clips.iter().filter(|c| c.source == "recom").count(),
                b.records.len(),
                b.records.join("; ")
            );
            Some(b)
        }
        Err(e) => {
            println!("note: KH1's Shadow not built: {e} (Shadows keep Re:CoM's behaviour)");
            None
        }
    };
    // KH1's other enemies (M11): each spec's files and manifest.
    let enemies: Vec<(&kh1_recom_assets::enemy::Spec, Option<kh1_recom_assets::enemy::Built>)> = kh1_recom_assets::enemy::SPECS
        .iter()
        .enumerate()
        .map(|(i, spec)| {
            progress(stage::ENEMIES, Some((i + 1, enemy_parts)));
            spec
        })
        .map(|spec| match enemy_build(spec, keys.clone(), &game, &arc) {
            Ok(b) => {
                let slots = {
                    let mut s: Vec<usize> = b.clips.iter().map(|c| c.slot).collect();
                    s.sort_unstable();
                    s.dedup();
                    s.len()
                };
                println!("{} (M11): {} requests on {} clips ({} Re:CoM bakes), {} attack records, hurt parts {:?}: {}", spec.name, b.clips.len(), slots, b.clips.iter().filter(|c| c.source == "recom").count(), b.records.len(), b.parts, b.records.join("; "));
                (*spec, Some(b))
            }
            Err(e) => {
                println!("note: KH1's {} not built: {e} (it keeps Re:CoM's behaviour)", spec.name);
                (*spec, None)
            }
        })
        .collect();
    if dry {
        return Ok(());
    }
    progress(stage::WRITE, None);
    // The runtime (crates/kh1-recom, win/motion.rs) looks for the manifest at the recom root.
    let (ann_path, wep_path, man, hw_dir) = match &opt.dest {
        Dest::Out(p) => {
            let m = p.with_file_name(install::MANIFEST);
            let w = p.with_file_name("PL0002.ANN");
            let h = p.with_file_name("PL0011.ANN");
            (p.clone(), w, m, h)
        }
        Dest::ModDir(mod_dir) => {
            let m = match mod_dir {
                Some(m) => m.clone(),
                None => install::mod_path(&game).map_err(|e| e.to_string())?,
            };
            (install::mod_file(&m, install::SORA_ANN), install::mod_file(&m, install::KEYBLADE_ANN), install::recom_dir(&m).join(install::MANIFEST), install::mod_file(&m, install::HALLOWEEN_ANN))
        }
        Dest::Root(r) => {
            let at = |p: &str| p.split('/').fold(r.clone(), |acc, s| acc.join(s));
            (at(install::SORA_ANN), at(install::KEYBLADE_ANN), r.join(install::MANIFEST), at(install::HALLOWEEN_ANN))
        }
    };
    if wep_path == ann_path {
        return Err("--out must not be named PL0002.ANN".into());
    }
    if built.changed {
        if let Some(w) = &built.weapon {
            write(&wep_path, &w.bytes)?;
        }
        write(&ann_path, &built.bytes)?;
        match &halloween {
            Some(h) => write(&hw_dir, &kh1_recom_assets::build::extend_costume_pack(&built, h).map_err(|e| format!("{}: {e}", install::HALLOWEEN_ANN))?)?,
            None => println!("note: {} not in Recom.hed; Halloween Town keeps Re:CoM's pack", install::HALLOWEEN_ANN),
        }
    } else {
        // Nothing new: Panacea serves Re:CoM's own files. Remove packs an earlier run left.
        for p in [&ann_path, &wep_path] {
            remove_stale(p)?;
        }
    }
    let json = built.manifest_json(&sources.iter().map(|(k, v)| (*k, v.clone())).collect::<Vec<_>>());
    write(&man, json.as_bytes())?;
    // Sounds for the KH1-only clips (Guard, Counterattack) in Re:CoM's own `PL0001.SES`.
    let clip = |entry: usize| built.entries.iter().find(|e| e.kh1_entry == entry && e.source.as_str() == "kh1").map(|e| e.name.clone());
    let (guard, counter) = (clip(110), clip(111));
    let mut sound = kh1_recom_assets::ses::parse(&ses).map_err(|e| format!("{}: {e}", install::SORA_SES))?;
    let n = kh1_recom_assets::ses::add_kh1(&mut sound, guard.as_deref(), counter.as_deref()).map_err(|e| e.to_string())?;
    // M9 9a: the KH1-only combo modifier clips, with Re:CoM's whooshes at KH1's frames.
    let modifier_clips: Vec<kh1_recom_assets::ses::ClipSounds> = kh1_recom_assets::ses::MODIFIER_ENTRIES
        .iter()
        .filter_map(|&(entry, finisher)| {
            let name = clip(entry)?;
            let m = kh1_set.motions.get(entry)?;
            Some(kh1_recom_assets::ses::ClipSounds { name, cues: kh1_recom_assets::ses::kh1_cues(m), finisher })
        })
        .collect();
    let nm = kh1_recom_assets::ses::add_clips(&mut sound, &modifier_clips).map_err(|e| e.to_string())?;
    // M9 stage 9c.1: Sonic Blade's clips get no entry here: base Re:CoM's sleight motion events
    // play on them (owner: its voices and effects; `win::specials::motion_events` in the DLL). An
    // older build's entries for them go with this rewrite (the file is Re:CoM's plus ours).
    // M13 stage 3: KH1's sounds for Sora's combos and finishers at the KH1 variant.
    let fx_moves = fx_moves(&game, keys.clone(), &kh1_set, &built);
    let mut fx_added = kh1_recom_assets::kh1_fx::Added::default();
    let sound_map = |id: u32| -> Option<u32> {
        if id < 1000 {
            return system_sounds.filter(|&n| id < n).map(|_| kh1_recom_assets::sounds::SYSTEM_GROUP * 1000 + id);
        }
        sounds.as_ref().and_then(|s| s.map.iter().find(|m| m.0 == id).map(|m| m.1))
    };
    let weapon_base = sounds.as_ref().map_or(0, |s| s.weapon_base);
    // M13 stage 4: each Keyblade's sounds at its own variant (the Kingdom Key's at stage 3's 1301).
    let weapon_bases: Vec<(u16, u32)> = match sounds.as_ref() {
        Some(s) if !s.weapon_bases.is_empty() => s.weapon_bases.clone(),
        _ if weapon_base != 0 => vec![(1, weapon_base)],
        _ => Vec::new(),
    };
    if let Some(mv) = &fx_moves {
        for &(card, base) in &weapon_bases {
            kh1_recom_assets::kh1_fx::sora_ses(&mut sound, mv, base, kh1_recom_assets::kh1_fx::sound_variant(card), sound_map, &mut fx_added).map_err(|e| e.to_string())?;
        }
    }
    let ses_path = ann_path.with_file_name("PL0001.SES");
    write(&ses_path, &kh1_recom_assets::ses::write(&sound))?;
    println!(
        "wrote {} ({} KH1 clip entries: guard {:?}, counterattack {:?}; {} combo modifier clips: {:?}; the specials' clips: none, Re:CoM's own events play on them)",
        ses_path.display(),
        n,
        guard,
        counter,
        nm,
        modifier_clips.iter().map(|c| c.name.as_str()).collect::<Vec<_>>()
    );
    // Halloween Town's sound events: Re:CoM's `PL0011.SES` with the same KH1 entries.
    if let Some(hs) = &halloween_ses {
        let mut hsound = kh1_recom_assets::ses::parse(hs).map_err(|e| format!("{}: {e}", install::HALLOWEEN_SES))?;
        kh1_recom_assets::ses::add_kh1(&mut hsound, guard.as_deref(), counter.as_deref()).map_err(|e| e.to_string())?;
        kh1_recom_assets::ses::add_clips(&mut hsound, &modifier_clips).map_err(|e| e.to_string())?;
        if let Some(mv) = &fx_moves {
            for &(card, base) in &weapon_bases {
                kh1_recom_assets::kh1_fx::sora_ses(&mut hsound, mv, base, kh1_recom_assets::kh1_fx::sound_variant(card), sound_map, &mut kh1_recom_assets::kh1_fx::Added::default()).map_err(|e| e.to_string())?;
            }
        }
        write(&hw_dir.with_file_name("PL0011.SES"), &kh1_recom_assets::ses::write(&hsound))?;
    }
    // The Keyblade models, under the same root as the manifest (`<mod>/recom` or --out's folder).
    let root = man.parent().map(Path::to_path_buf).unwrap_or_default();
    for (m, c) in &keyblades {
        if let Some(c) = c {
            for (rel, bytes) in kh1_recom_assets::keyblade::mod_files(m.group, c).map_err(|e| e.to_string())? {
                write(&rel.split('/').fold(root.clone(), |p, s| p.join(s)), &bytes)?;
            }
        }
    }
    // M13 stage 3: KH1's trails and finisher effects as Re:CoM's EET events at the KH1 variant
    // (Sora's PL0001.EET, the Keyblade's PL0002.EET) and kh1-fx.json, the moves the DLL switches.
    let fx_manifest_path = root.join(kh1_recom_assets::kh1_fx::MANIFEST);
    let kh1_groups: Vec<(u16, u16, String)> = keyblades.iter().filter_map(|(m, c)| Some((m.card as u16, m.group, c.as_ref()?.report.model.clone()))).filter(|k| k.0 <= kh1_recom_assets::keyblade::LAST_KH1_CARD as u16).collect();
    match fx_eets(&arc, &keys, fx_moves.as_deref(), &mut fx_added, &game, &kh1_groups, &sound_map) {
        Ok(Some((files, group_files))) => {
            for (name, bytes) in &files {
                // Next to the pack of the same group (as the step writes PL0001 / PL0002 / PL0011).
                let at = if name.starts_with("PL0002") { wep_path.with_file_name(name) } else if name.starts_with("PL0001") { ann_path.with_file_name(name) } else { hw_dir.with_file_name(name) };
                write(&at, bytes)?;
            }
            // M13 stage 4: each KH1 Keyblade group's own EET (loose) and EFF (raw, with its PC textures).
            for (rel, bytes) in &group_files {
                write(&rel.split('/').fold(root.clone(), |p, s| p.join(s)), bytes)?;
            }
            let src = [("PL0001.SES sha256", kh1_formats::egs::sha256_hex(&ses))];
            write(&fx_manifest_path, kh1_recom_assets::kh1_fx::manifest_json(&fx_added, &src).as_bytes())?;
            println!(
                "KH1 fx (M13 stage 3, variant {}): trails on {} Keyblade clips {:?}; effects {:?}{}; KH1 sounds on {} clips{}",
                kh1_recom_assets::kh1_fx::VARIANT,
                fx_added.trails.len(),
                fx_added.trails.iter().map(|t| (t.0, t.2, t.3)).collect::<Vec<_>>(),
                fx_added.effects.iter().map(|e| (e.0, e.2.clone())).collect::<Vec<_>>(),
                if fx_added.effects_out.is_empty() { String::new() } else { format!("; NOT placed: {:?}", fx_added.effects_out.iter().map(|e| (e.0, e.1, e.2)).collect::<Vec<_>>()) },
                fx_added.sounds.iter().filter(|s| s.4 == kh1_recom_assets::kh1_fx::VARIANT).count(),
                {
                    let miss: Vec<u32> = fx_added.sounds.iter().flat_map(|s| s.3.clone()).collect();
                    if miss.is_empty() { String::new() } else { format!(" (Re:CoM's whoosh kept for KH1 ids not in a bank: {miss:?})") }
                }
            );
        }
        Ok(None) => {
            let mut stale = vec![fx_manifest_path.clone(), ann_path.with_file_name("PL0001.EET"), wep_path.with_file_name("PL0002.EET"), hw_dir.with_file_name("PL0011.EET"), hw_dir.with_file_name("PL0012.EET")];
            for (_, g, _) in &kh1_groups {
                for rel in [kh1_recom_assets::keyblade::resource_path(*g, "EET"), format!("raw/{}", kh1_recom_assets::keyblade::resource_path(*g, "EFF"))] {
                    stale.push(rel.split('/').fold(root.clone(), |p, s| p.join(s)));
                }
            }
            for p in &stale {
                if p.is_file() {
                    std::fs::remove_file(p).map_err(|e| format!("{}: {e}", p.display()))?;
                    println!("removed {} (no KH1 fx this run)", p.display());
                }
            }
        }
        Err(e) => return Err(e),
    }
    // The MP bar: Re:CoM's FO0002.SPR with the bar's groups (a raw PKG entry, every language's
    // copy inside) and the manifest the DLL reads. Without it, an older manifest is removed so the
    // DLL never adds groups the served file may not have.
    let bar_manifest = root.join(kh1_recom_assets::mp_bar::MANIFEST);
    match &mp_bar {
        Some(b) => {
            let entry = kh1_formats::egs::build_plain_entry(&b.spr, &b.assets, b.date).map_err(|e| e.to_string())?;
            write(&root.join("raw").join(kh1_recom_assets::mp_bar::SPR_PATH.replace('/', std::path::MAIN_SEPARATOR_STR)), &entry)?;
            write(&bar_manifest, b.manifest.as_bytes())?;
        }
        None => {
            if bar_manifest.is_file() {
                std::fs::remove_file(&bar_manifest).map_err(|e| format!("{}: {e}", bar_manifest.display()))?;
                println!("removed {} (no MP bar this run)", bar_manifest.display());
            }
        }
    }
    // The camp's item row: Re:CoM's FO0100.SPR with the new group (a raw PKG entry, every
    // language's copy inside) and its manifest; without them an older manifest is removed (the DLL
    // then keeps Re:CoM's green row 0x131).
    let camp_manifest = root.join(kh1_recom_assets::camp_rows::MANIFEST);
    match &camp_rows {
        Some(c) => {
            write(&root.join("raw").join(kh1_recom_assets::camp_rows::SPR_PATH.replace('/', std::path::MAIN_SEPARATOR_STR)), &c.entry)?;
            write(&camp_manifest, c.manifest.as_bytes())?;
        }
        None => {
            if camp_manifest.is_file() {
                std::fs::remove_file(&camp_manifest).map_err(|e| format!("{}: {e}", camp_manifest.display()))?;
                println!("removed {} (no camp item row this run)", camp_manifest.display());
            }
        }
    }
    // M8d: Re:CoM's FO0109.SPR with the five-tab groups (a raw PKG entry, every language's copy
    // inside) and its manifest; without them an older manifest is removed (the DLL then keeps
    // Re:CoM's four-tab window).
    let shop_manifest = root.join(kh1_recom_assets::shop_tabs::MANIFEST);
    match &shop_tabs {
        Some(c) => {
            write(&root.join("raw").join(kh1_recom_assets::shop_tabs::SPR_PATH.replace('/', std::path::MAIN_SEPARATOR_STR)), &c.entry)?;
            write(&shop_manifest, c.manifest.as_bytes())?;
        }
        None => {
            if shop_manifest.is_file() {
                std::fs::remove_file(&shop_manifest).map_err(|e| format!("{}: {e}", shop_manifest.display()))?;
                println!("removed {} (no shop tabs this run)", shop_manifest.display());
            }
        }
    }
    // M13: Re:CoM's EF1000.EFF with KH1's sets (a raw PKG entry) and its manifest; without them
    // an older manifest and container are removed (Re:CoM's own container is served again).
    let fx_manifest = root.join(kh1_recom_assets::effects::MANIFEST);
    let fx_entry = root.join("raw").join(kh1_recom_assets::effects::EFF_PATH.replace('/', std::path::MAIN_SEPARATOR_STR));
    match &effects {
        Some(e) => {
            write(&fx_entry, &e.entry)?;
            write(&fx_manifest, e.manifest.as_bytes())?;
        }
        None => {
            for f in [&fx_manifest, &fx_entry] {
                if f.is_file() {
                    std::fs::remove_file(f).map_err(|e| format!("{}: {e}", f.display()))?;
                    println!("removed {} (no KH1 effects this run)", f.display());
                }
            }
        }
    }
    // M13 stage 2: KH1's sound banks (the SCD pack and its manifest); without them older ones
    // are removed (Re:CoM's sounds stay).
    let sd_manifest = root.join(kh1_recom_assets::sounds::MANIFEST);
    let sd_pack = root.join(kh1_recom_assets::sounds::PACK);
    match &sounds {
        Some(s) => {
            write(&sd_pack, &s.pack)?;
            write(&sd_manifest, s.manifest.as_bytes())?;
        }
        None => {
            for p in [&sd_manifest, &sd_pack] {
                if p.is_file() {
                    std::fs::remove_file(p).map_err(|e| format!("{}: {e}", p.display()))?;
                    println!("removed {} (no KH1 sound banks this run)", p.display());
                }
            }
        }
    }
    // The KH1 HUD: the raw FO0002.RTM entry and kh1-hud.json; without them an older manifest is
    // removed (the DLL then keeps Re:CoM's HUD).
    let hud_manifest = root.join(kh1_recom_assets::kh1_hud::MANIFEST);
    match &hud {
        Some(h) => {
            write(&root.join("raw").join(kh1_recom_assets::mp_bar::RTM_PATH.replace('/', std::path::MAIN_SEPARATOR_STR)), &h.rtm_entry)?;
            write(&hud_manifest, h.manifest.as_bytes())?;
        }
        None => {
            if hud_manifest.is_file() {
                std::fs::remove_file(&hud_manifest).map_err(|e| format!("{}: {e}", hud_manifest.display()))?;
                println!("removed {} (no KH1 HUD this run)", hud_manifest.display());
            }
        }
    }
    // The command menu's manifest; without it (or without the bar) an older one is removed.
    let menu_manifest = root.join(kh1_recom_assets::command_menu::MANIFEST);
    match &menu {
        Some(m) => write(&menu_manifest, m.as_bytes())?,
        None => {
            if menu_manifest.is_file() {
                std::fs::remove_file(&menu_manifest).map_err(|e| format!("{}: {e}", menu_manifest.display()))?;
                println!("removed {} (no command menu this run)", menu_manifest.display());
            }
        }
    }
    // KH1's menu sounds: the pack and its manifest; without them older ones are removed (the DLL
    // then plays Re:CoM's sounds).
    let snd_manifest = root.join(kh1_recom_assets::menu_sounds::MANIFEST);
    let snd_pack = root.join(kh1_recom_assets::menu_sounds::PACK);
    match &menu_sounds {
        Some((bin, json, previews)) => {
            write(&snd_pack, bin)?;
            write(&snd_manifest, json.as_bytes())?;
            // For listening only (not part of the mod): each menu entry as a WAV.
            for (name, wav) in previews.iter().filter(|_| opt.previews) {
                write(&root.join(kh1_recom_assets::menu_sounds::PREVIEW_DIR).join(name), wav)?;
            }
        }
        None => {
            for p in [&snd_manifest, &snd_pack] {
                if p.is_file() {
                    std::fs::remove_file(p).map_err(|e| format!("{}: {e}", p.display()))?;
                    println!("removed {} (no KH1 menu sounds this run)", p.display());
                }
            }
        }
    }
    // KH1's Shadow (M10): Re:CoM's ENEMY/0001 files extended and the manifest; without them the
    // older files are removed (the DLL then keeps Re:CoM's Shadow).
    {
        use kh1_recom_assets::shadow as sh;
        let man = root.join(sh::MANIFEST);
        let files = [(sh::ANN_PATH, 0), (sh::ASD_PATH, 1), (sh::SES_PATH, 2)];
        let at = |p: &str| p.split('/').fold(root.clone(), |acc, s| acc.join(s));
        match &shadow {
            Some(b) => {
                for (p, i) in files {
                    write(&at(p), [&b.ann, &b.asd, &b.ses][i])?;
                }
                write(&man, b.manifest.as_bytes())?;
            }
            None => {
                // Only when an earlier run of this step wrote them (its manifest is there).
                if man.is_file() {
                    for p in [man.clone(), at(sh::ANN_PATH), at(sh::ASD_PATH), at(sh::SES_PATH)] {
                        if p.is_file() {
                            std::fs::remove_file(&p).map_err(|e| format!("{}: {e}", p.display()))?;
                            println!("removed {} (no KH1 Shadow this run)", p.display());
                        }
                    }
                }
            }
        }
    }
    // KH1's other enemies (M11): the same, per spec.
    for (spec, b) in &enemies {
        let man = root.join(spec.manifest);
        let files = [(spec.path("ANN"), 0), (spec.path("ASD"), 1), (spec.path("SES"), 2)];
        let at = |p: &str| p.split('/').fold(root.clone(), |acc, s| acc.join(s));
        match b {
            Some(b) => {
                // An enemy with no new attack record keeps Re:CoM's `.ASD` (the Red Nocturne: its rows are its
                // children's); one Re:CoM keeps no `.SES` for gets none (`enemy::NO_SES`).
                let records = b.asd.get(0x10..0x14).map_or(0, |w| u32::from_le_bytes(w.try_into().unwrap()) as usize);
                for (p, i) in &files {
                    let skip = match *i {
                        1 => records == spec.asd.0,
                        2 => b.ses.is_empty(),
                        _ => false,
                    };
                    if skip {
                        println!("{}: {p} not written (Re:CoM's own stays)", spec.name);
                        continue;
                    }
                    write(&at(p), [&b.ann, &b.asd, &b.ses][*i])?;
                }
                write(&man, b.manifest.as_bytes())?;
            }
            None => {
                if man.is_file() {
                    for p in [man.clone(), at(&files[0].0), at(&files[1].0), at(&files[2].0)] {
                        if p.is_file() {
                            std::fs::remove_file(&p).map_err(|e| format!("{}: {e}", p.display()))?;
                            println!("removed {} (no KH1 {} this run)", p.display(), spec.name);
                        }
                    }
                }
            }
        }
    }
    // KH1 stats (decisions 12, 18, 21, 25): the enemy twins' records, Sora's Strength, the
    // Keyblades (with their model groups).
    let models: Vec<KeybladeModel> = keyblades.iter().map(|k| k.0.clone()).collect();
    progress(stage::STATS, None);
    let stats = kh1_stats(&game, keys, &arc, &models, &kh2_rows)?;
    for n in &stats.notes {
        println!("{n}");
    }
    let sp = man.with_file_name(install::STATS);
    write_quiet(&sp, stats.json.as_bytes())?;
    println!("wrote {} ({} KH1 enemy twins, {} boss twins, {} bytes)", sp.display(), stats.twins, stats.bosses, stats.json.len());
    Ok(())
}

/// KH1's Shadow (M10, `kh1_recom_assets::shadow`): KH1's `xa_ex_2020.mset` / `.mdls` and
/// `btltbl.bin`, Re:CoM's `ENEMY/0001/EN0001.{ANN,MDL,ASD,SES}`.
fn shadow_build(game: &Path, keys: kh1_formats::egs::KeyTable, arc: &kh1_formats::egs::Archive) -> Result<kh1_recom_assets::shadow::Built, String> {
    use kh1_recom_assets::shadow as sh;
    let inst = install::kh1(game, keys.clone()).map_err(|e| format!("KH1 files: {e}"))?;
    let k = |n: &str| inst.read(n).map_err(|e| format!("{n}: {e}"));
    let r = |n: &str| install::read_recom(arc, &keys, n).map_err(|e| format!("{n}: {e}"));
    let inp = sh::Inputs { mset: k(sh::KH1_MSET)?, mdls: k(sh::KH1_MDLS)?, ann: r(sh::ANN_PATH)?, mdl: r(sh::MDL_PATH)?, asd: r(sh::ASD_PATH)?, ses: r(sh::SES_PATH)?, btltbl: k("btltbl.bin")? };
    sh::build(&inp).map_err(|e| e.to_string())
}

/// An M11 enemy (`kh1_recom_assets::enemy`): KH1's `.mset` / `.mdls` and `btltbl.bin`, Re:CoM's
/// `ENEMY/NNNN/ENNNNN.{ANN,MDL,ASD,SES}`.
fn enemy_build(spec: &kh1_recom_assets::enemy::Spec, keys: kh1_formats::egs::KeyTable, game: &Path, arc: &kh1_formats::egs::Archive) -> Result<kh1_recom_assets::enemy::Built, String> {
    let inst = install::kh1(game, keys.clone()).map_err(|e| format!("KH1 files: {e}"))?;
    let k = |n: &str| inst.read(n).map_err(|e| format!("{n}: {e}"));
    let r = |n: &str| install::read_recom(arc, &keys, n).map_err(|e| format!("{n}: {e}"));
    let inp = kh1_recom_assets::enemy::Inputs {
        mset: k(&format!("{}.mset", spec.kh1))?,
        mdls: k(&format!("{}.mdls", spec.kh1))?,
        ann: r(&spec.path("ANN"))?,
        mdl: r(&spec.path("MDL"))?,
        asd: r(&spec.path("ASD"))?,
        // Re:CoM keeps none for some enemies (`enemy::NO_SES`, the Red Nocturne): none read.
        ses: if kh1_recom_assets::enemy::NO_SES.contains(&spec.name) { Vec::new() } else { r(&spec.path("SES"))? },
        btltbl: k("btltbl.bin")?,
    };
    kh1_recom_assets::enemy::build(spec, &inp).map_err(|e| e.to_string())
}

/// Converts the 17 KH1 Keyblades (`KH1_KEYBLADES`): their model names from KH1's weapon table,
/// each `.wpn` with its HD assets from KH1's archives, laid out on Re:CoM's own `PL0002.MDL`.
/// The Kingdom Key keeps Re:CoM's PL0002 (group 2). Without PL0002.MDL nothing is converted.
fn convert_keyblades(game: &Path, keys: kh1_formats::egs::KeyTable, pl0002: Option<&[u8]>) -> Result<Vec<(KeybladeModel, Option<Converted>)>, String> {
    let Some(pl0002) = pl0002 else {
        println!("note: no PL0002.MDL; the Keyblade models are not built");
        return Ok(Vec::new());
    };
    let template = Mdl::parse(pl0002).map_err(|e| format!("{}: {e}", install::KEYBLADE_MDL))?;
    let inst = install::kh1(game, keys).map_err(|e| format!("KH1 files: {e}"))?;
    let btl = inst.read("btltbl.bin").map_err(|e| format!("btltbl.bin: {e}"))?;
    let table = kh1_formats::btltbl::Btltbl::parse(btl).map_err(|e| format!("btltbl.bin: {e}"))?;
    let mut out = Vec::new();
    for (card, name, item) in KH1_KEYBLADES {
        let group = group_for_card(*card).ok_or_else(|| format!("{name}: no group for card {card}"))?;
        if *card == 1 {
            out.push((KeybladeModel { card: *card, group, textures: template.parts[0].texture_names() }, None));
            continue;
        }
        let model = table.weapon_for_item(*item).map_err(|e| format!("{name}: {e}"))?.model;
        let file = format!("{model}.wpn");
        let (wpn, assets, _) = inst.read_full(&file).map_err(|e| format!("{name}: {file}: {e}"))?;
        let c = convert(&template, &model, &wpn, &assets).map_err(|e| format!("{name} ({file}): {e}"))?;
        out.push((KeybladeModel { card: *card, group, textures: c.textures.clone() }, Some(c)));
    }
    Ok(out)
}

/// A Keyblade's model entry and its conversion (None: Re:CoM's own PL0002).
type ModelOut = (KeybladeModel, Option<Converted>);

/// The six KH2 Keyblades (`kh2_keyblades::KH2_KEYBLADES`): their items, stat rows and models from
/// the player's KH2 files, each model's blade length, and (with Re:CoM's PL0002.MDL) the models
/// converted on it. KH2's Kingdom Key must match PL0002 first (`kh2_template_distance`), and
/// every KH2 Keyblade must have its skeleton. Without KH2's archives there are no KH2 rows.
fn kh2_keyblades_step(game: &Path, keys: kh1_formats::egs::KeyTable, pl0002: Option<&[u8]>) -> Result<(Vec<Kh2Row>, Vec<ModelOut>), String> {
    let inst = match install::kh2(game, keys) {
        Ok(i) => i,
        Err(e) => {
            println!("note: KH2's archives not read ({e}); the KH2 Keyblades are not built");
            return Ok((Vec::new(), Vec::new()));
        }
    };
    let read = |n: &str| inst.read(n).map_err(|e| format!("KH2 {n}: {e}"));
    let found = kh2_keyblades::find(&read("03system.bin")?, &read("00objentry.bin")?, &read(kh2_keyblades::SYS_BAR)?).map_err(|e| e.to_string())?;
    let kk = &found[0];
    let kk_mdlx = read(&kk.file())?;
    let reference = kh2_keyblades::reference_skeleton(&kk_mdlx).map_err(|e| e.to_string())?;
    let kk_len = kh2_keyblades::blade_length(&kk_mdlx).map_err(|e| e.to_string())?;
    let template = match pl0002 {
        Some(b) => {
            let t = Mdl::parse(b).map_err(|e| format!("{}: {e}", install::KEYBLADE_MDL))?;
            let d = kh2_template_distance(&t, &kk_mdlx).map_err(|e| e.to_string())?;
            println!("KH2's Kingdom Key ({}) on Re:CoM's PL0002: every vertex within {:.3} mm of PL0002's on the same bone", kk.model, d * 1000.0);
            if d > KH2_TEMPLATE_MATCH {
                return Err(format!("KH2's Kingdom Key is {:.3} mm from Re:CoM's PL0002 (more than {:.1} mm): the KH2 bone mapping does not hold for these files", d * 1000.0, KH2_TEMPLATE_MATCH * 1000.0));
            }
            Some(t)
        }
        None => None,
    };
    let mut rows = Vec::new();
    let mut models = Vec::new();
    for k in &found[1..] {
        let (mdlx, assets, _) = inst.read_full(&k.file()).map_err(|e| format!("{} ({}): {e}", k.name, k.file()))?;
        let len = kh2_keyblades::blade_length(&mdlx).map_err(|e| format!("{}: {e}", k.name))?;
        rows.push(Kh2Row { keyblade: k.clone(), blade_length: len, kk_blade_length: kk_len });
        if let Some(t) = &template {
            let group = group_for_card(k.card).ok_or_else(|| format!("{}: no group for card {:#x}", k.name, k.card))?;
            let c = convert_kh2(t, &k.model, &mdlx, &assets, &reference).map_err(|e| format!("{} ({}): {e}", k.name, k.file()))?;
            models.push((KeybladeModel { card: k.card, group, textures: c.textures.clone() }, Some(c)));
        }
    }
    Ok((rows, models))
}

/// The MP bar's files (M7 stage 7.2): the raw `FO0002.SPR` entry and `kh1-mp-bar.json`.
struct MpBar {
    /// The patched `FO0002.SPR`, its PC assets and the entry's date word (the entry is built at
    /// write time, after the KH1 HUD's files are added).
    spr: Vec<u8>,
    assets: Vec<kh1_formats::egs::HdAsset>,
    date: i32,
    manifest: String,
}

/// Reads KH1's gauge look (`command2/uitex.bin`) and Re:CoM's neutral texel (`FO0002.RTM`), and adds
/// the bar's groups to `Cockpit:00` in Re:CoM's `FO0002.SPR` and in each of its PC assets that is
/// a `Cockpit.lad` or a whole container (the language copies), as a plain PKG entry with the same
/// asset names, tags and date (`docs/recom-m7-mp-bar.md`).
fn mp_bar_build(game: &Path, keys: kh1_formats::egs::KeyTable, arc: &kh1_formats::egs::Archive) -> Result<MpBar, String> {
    use kh1_recom_assets::mp_bar::{self as mb, Patched};
    let inst = install::kh1(game, keys.clone()).map_err(|e| format!("KH1 files: {e}"))?;
    let (uitex, hd, _) = inst.read_full(mb::KH1_UITEX).map_err(|e| format!("{}: {e}", mb::KH1_UITEX))?;
    if !hd.is_empty() {
        let names: Vec<&str> = hd.iter().map(|a| a.name.as_str()).collect();
        println!("note: {} has {} PC assets {:?}; the MP bar's colours are read from its PS2 pictures", mb::KH1_UITEX, hd.len(), names);
    }
    let look = mb::kh1_look(&uitex).map_err(|e| format!("{}: {e}", mb::KH1_UITEX))?;
    let rtm = install::read_recom(arc, &keys, mb::RTM_PATH).map_err(|e| format!("{}: {e}", mb::RTM_PATH))?;
    let neutral = mb::neutral_texel(&rtm).map_err(|e| format!("{}: {e}", mb::RTM_PATH))?;
    let e = arc.find(mb::SPR_PATH).ok_or_else(|| format!("{} not in Recom.hed", mb::SPR_PATH))?;
    let raw = arc.read_raw_entry(e).map_err(|e| format!("{}: {e}", mb::SPR_PATH))?;
    let date = i32::from_le_bytes(raw.get(12..16).ok_or("short entry")?.try_into().unwrap());
    let (spr, assets) = kh1_formats::egs::decode_entry_full(&keys, &raw).map_err(|e| format!("{}: {e}", mb::SPR_PATH))?;
    let (base, first) = mb::patch_spr(&spr, &look, neutral).map_err(|e| format!("{}: {e}", mb::SPR_PATH))?;
    let mut files = vec![(mb::SPR_PATH.to_string(), Patched::Bar(first))];
    let mut out_assets = Vec::with_capacity(assets.len());
    for a in &assets {
        let (data, how) = mb::patch_asset(&a.name, &a.data, &look, neutral).map_err(|e| format!("{} asset {}: {e}", mb::SPR_PATH, a.name))?;
        if let Patched::Bar(f) = how {
            if f != first {
                println!("note: {} asset {}: its own Cockpit:00 has {f} groups (the base file {first}); the bar's groups start at {f} there", mb::SPR_PATH, a.name);
            }
        }
        files.push((a.name.clone(), how));
        out_assets.push(kh1_formats::egs::HdAsset { name: a.name.clone(), orig_offset: a.orig_offset, data });
    }
    let sources = [
        ("uitex.bin sha256", kh1_formats::egs::sha256_hex(&uitex)),
        ("FO0002.RTM sha256", kh1_formats::egs::sha256_hex(&rtm)),
        ("FO0002.SPR sha256", kh1_formats::egs::sha256_hex(&spr)),
    ];
    let mut firsts: Vec<usize> = files.iter().filter_map(|f| if let Patched::Bar(g) = f.1 { Some(g) } else { None }).collect();
    firsts.sort_unstable();
    firsts.dedup();
    let manifest = mb::manifest_json(first, &firsts, &look, neutral, &files, &sources);
    let patched: Vec<&str> = files.iter().filter(|f| matches!(f.1, Patched::Bar(_))).map(|f| f.0.as_str()).collect();
    let kept: Vec<&str> = files.iter().filter(|f| f.1 == Patched::Kept).map(|f| f.0.as_str()).collect();
    println!(
        "MP bar: KH1 MP {:?}→{:?}, charge {:?}→{:?}, background {:?} / {:?}, segment {}×{} (charge {} high); Re:CoM neutral texel {:?}; groups {}..{} of {} added to {:?}{}",
        look.mp_top,
        look.mp_bottom,
        look.charge_top,
        look.charge_bottom,
        look.back,
        look.border,
        look.slot_w,
        look.mp_h,
        look.charge_h,
        neutral,
        first,
        first + mb::group::COUNT - 1,
        mb::SET,
        patched,
        if kept.is_empty() { String::new() } else { format!("; kept as they are: {kept:?}") }
    );
    Ok(MpBar { spr: base, assets: out_assets, date, manifest })
}

/// M13 stage 1's files: the raw `EF1000.EFF` entry and `kh1-effects.json`.
struct Effects {
    entry: Vec<u8>,
    manifest: String,
}

/// Adds KH1's Sora effect set (`xa_ex_0010.mdls` section 1, with its PC textures) to Re:CoM's
/// `EF1000.EFF` after Re:CoM's own rows, as a plain PKG entry with Re:CoM's assets, the set's
/// textures and Re:CoM's date (`kh1_recom_assets::effects`).
fn effects_build(game: &Path, keys: kh1_formats::egs::KeyTable, arc: &kh1_formats::egs::Archive, system_sounds: Option<u32>) -> Result<Effects, String> {
    use kh1_recom_assets::effects as fx;
    let inst = install::kh1(game, keys.clone()).map_err(|e| format!("KH1 files: {e}"))?;
    let (mdls, mdls_hd, _) = inst.read_full(fx::SORA_MDLS).map_err(|e| format!("{}: {e}", fx::SORA_MDLS))?;
    let mut sora = fx::kh1_set(fx::SORA_SET, fx::SORA_MDLS, &mdls, fx::SORA_SECTION, &mdls_hd).map_err(|e| format!("{}: {e}", fx::SORA_MDLS))?;
    // M13 stage 2: the set's own sounds (KH1's program `+0xFC`, ids under 1000 = system bank
    // entries) as Re:CoM's `.ese` rows, on bank 9000 (the menu sounds' pack holds the bank).
    let map = |id: u32| match system_sounds {
        Some(n) if id < n => Some(kh1_recom_assets::sounds::SYSTEM_GROUP * 1000 + id),
        _ => None,
    };
    if system_sounds.is_none() {
        println!("note: KH1 effects: no KH1 system bank (amusic/se000.dat not taken): {} has no .ese (its effects play without sounds)", fx::SORA_SET);
    } else {
        let (rows, skipped) = fx::kh1_ese_rows(&sora.data, map).map_err(|e| format!("{}: {e}", fx::SORA_SET))?;
        for (id, snd, why) in &skipped {
            println!("note: KH1 effects: {} effect {id}'s sound {snd} not in its .ese: {why}", fx::SORA_SET);
        }
        if !rows.is_empty() {
            println!("KH1 effects: {} with {} rows {:?} (effect id, Re:CoM id; bank 9000 = KH1's system bank)", sora.ese_name(), rows.len(), rows.iter().map(|r| (r.id, r.group as u32 * 1000 + r.number as u32)).collect::<Vec<_>>());
            sora.ese = Some(fx::write_ese(&rows));
        }
    }
    let e = arc.find(fx::EFF_PATH).ok_or_else(|| format!("{} not in Recom.hed", fx::EFF_PATH))?;
    let raw = arc.read_raw_entry(e).map_err(|e| format!("{}: {e}", fx::EFF_PATH))?;
    let date = i32::from_le_bytes(raw.get(12..16).ok_or("short entry")?.try_into().unwrap());
    let (eff, assets) = kh1_formats::egs::decode_entry_full(&keys, &raw).map_err(|e| format!("{}: {e}", fx::EFF_PATH))?;
    let own: Vec<String> = fx::parse_eff(&eff).map_err(|e| format!("{}: {e}", fx::EFF_PATH))?.into_iter().map(|r| r.name).collect();
    let (out, all) = fx::add_sets(&eff, &assets, std::slice::from_ref(&sora)).map_err(|e| e.to_string())?;
    let texture_names: Vec<&str> = sora.hd.iter().map(|a| a.name.as_str()).collect();
    println!(
        "KH1 effects: {} ({}, {} bytes, effect ids {:?}, PC textures {:?}) added to {} after Re:CoM's {:?} ({} PC assets kept)",
        sora.name,
        sora.source,
        sora.data.len(),
        sora.ids,
        texture_names,
        fx::EFF_PATH,
        own,
        assets.len()
    );
    let entry = kh1_formats::egs::build_plain_entry(&out, &all, date).map_err(|e| e.to_string())?;
    let sources = [("EF1000.EFF sha256", kh1_formats::egs::sha256_hex(&eff)), ("xa_ex_0010.mdls sha256", kh1_formats::egs::sha256_hex(&mdls))];
    Ok(Effects { entry, manifest: fx::manifest_json(std::slice::from_ref(&sora), &sources) })
}

/// M13 stage 3: the moves of Sora's KH1 rows that have a trail (`kh1_fx::moves`), with the clips the
/// motion build gave their requests. None without the files (no KH1 fx).
fn fx_moves(game: &Path, keys: kh1_formats::egs::KeyTable, set: &kh1_formats::mset::MotionSet, built: &kh1_recom_assets::build::Built) -> Option<Vec<kh1_recom_assets::kh1_fx::Move>> {
    let inst = install::kh1(game, keys).ok()?;
    let btl = inst.read("btltbl.bin").ok()?;
    let rows = kh1_formats::btltbl::Btltbl::parse(btl).ok()?.sora_rows().ok()?;
    let clip_of = |r: u16| built.entries.iter().find(|e| e.requests.contains(&r)).map(|e| (e.name.clone(), e.weapon_name.clone()));
    Some(kh1_recom_assets::kh1_fx::moves(set, &rows, clip_of))
}

/// M13 stage 3: Re:CoM's `PL0001.EET` and `PL0002.EET` with KH1's events added (`kh1_fx`), and
/// Halloween Town's `PL0011.EET` (Sora) / `PL0012.EET` (its Keyblade) when Recom.hed has them (the DLL
/// passes KH1's variant there too, so they need the same events). None when there are no moves or
/// one of the first two files is not in Recom.hed.
///
/// M13 stage 4: also each KH1 Keyblade group's own files (`groups`: card, group, KH1 model), as paths under
/// the recom root: `PLAYER/00GG/PL00GG.EET` (Re:CoM's `PL0002.EET` with KH1's trail on the Keyblade's own
/// `.wpn` set at its reach, and Gravity Break's blade point at its reach) and `raw/PLAYER/00GG/PL00GG.EFF`
/// (the set and its PC textures). A Keyblade whose files don't read is noted and keeps the shared
/// `PL0002.EET` (the Kingdom Key's trail).
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn fx_eets(
    arc: &kh1_formats::egs::Archive,
    keys: &kh1_formats::egs::KeyTable,
    moves: Option<&[kh1_recom_assets::kh1_fx::Move]>,
    added: &mut kh1_recom_assets::kh1_fx::Added,
    game: &Path,
    groups: &[(u16, u16, String)],
    sound_map: &dyn Fn(u32) -> Option<u32>,
) -> Result<Option<(Vec<(String, Vec<u8>)>, Vec<(String, Vec<u8>)>)>, String> {
    use kh1_recom_assets::kh1_fx as fx;
    let Some(moves) = moves else {
        println!("note: KH1 fx not built: no KH1 rows (btltbl.bin)");
        return Ok(None);
    };
    let (Some(_), Some(_), Some(_)) = (arc.find(install::SORA_EET), arc.find(install::KEYBLADE_EET), arc.find(install::KEYBLADE_MDL)) else {
        println!("note: KH1 fx not built: {} / {} / {} not in Recom.hed", install::SORA_EET, install::KEYBLADE_EET, install::KEYBLADE_MDL);
        return Ok(None);
    };
    let rd = |n: &str| install::read_recom(arc, keys, n).map_err(|e| format!("{n}: {e}"));
    let (sora, weapon, mdl) = (rd(install::SORA_EET)?, rd(install::KEYBLADE_EET)?, rd(install::KEYBLADE_MDL)?);
    // Gravity Break's effect 2 on the blade at KH1's fraction 0.8 (code 0x30: t = W+0x28 + (W+0x24 −
    // W+0x28) × 0.8, `0x14029EB20`), as an offset from Re:CoM's `wep0` toward `k9` (KH1's 282 → 283).
    let table = install::kh1(game, keys.clone()).ok().and_then(|inst| inst.read("btltbl.bin").ok()).and_then(|b| kh1_formats::btltbl::Btltbl::parse(b).ok());
    let segment: Option<[f32; 3]> = (|| {
        let sk = kh1_recom_assets::mdl::RecomSkeleton::parse(&mdl).ok()?;
        let b = sk.bones.get(fx::BLADE_BONES.1)?;
        (b.parent as usize == fx::BLADE_BONES.0).then(|| b.local.t.map(|v| v as f32))
    })();
    let blade_at = |w: &kh1_formats::btltbl::Weapon| segment.map(|seg| seg.map(|v| v * (w.blade_near + (w.blade_far - w.blade_near) * 0.8)));
    let blade = table.as_ref().and_then(|t| t.weapon_for_item(kh1_recom_assets::sounds::KINGDOM_KEY_ITEM).ok()).and_then(|w| blade_at(&w));
    if blade.is_none() {
        println!("note: KH1 fx: Gravity Break's blade effect not placed (PL0002.MDL bone 3 is not wep0's child, or no Kingdom Key row)");
    }
    // Stun Impact's 0 and Zantetsuken's 5 (KH1 code 0x8003): Sora's own body point, from his CLS record.
    let body = install::kh1(game, keys.clone()).map_err(|e| e.to_string()).and_then(|inst| inst.read_full(kh1_recom_assets::effects::SORA_MDLS).map_err(|e| e.to_string())).and_then(|(m, _, _)| fx::sora_body_point(&m).map_err(|e| e.to_string()));
    let body = match body {
        Ok(p) => {
            println!("KH1 fx: Sora's body point (KH1 code 0x8003, his CLS record) {p:?} m above Root");
            Some(p)
        }
        Err(e) => {
            println!("note: KH1 fx: Stun Impact's effect 0 and Zantetsuken's 5 not placed: {}: {e}", kh1_recom_assets::effects::SORA_MDLS);
            None
        }
    };
    let weapon_eet = fx::weapon_eet(&weapon, moves, blade, None, added).map_err(|e| format!("{}: {e}", install::KEYBLADE_EET))?;
    let sora_eet = fx::sora_eet(&sora, moves, body, added).map_err(|e| format!("{}: {e}", install::SORA_EET))?;
    let mut out = vec![("PL0001.EET".to_string(), sora_eet), ("PL0002.EET".to_string(), weapon_eet)];
    for (path, weapon_file) in [(install::HALLOWEEN_EET, false), (install::HALLOWEEN_KEYBLADE_EET, true)] {
        if arc.find(path).is_none() {
            println!("note: KH1 fx: {path} not in Recom.hed (Halloween Town keeps none of KH1's events there)");
            continue;
        }
        let r = rd(path).and_then(|d| if weapon_file { fx::weapon_eet(&d, moves, blade, None, &mut fx::Added::default()) } else { fx::sora_eet(&d, moves, body, &mut fx::Added::default()) }.map_err(|e| format!("{path}: {e}")));
        match r {
            Ok(b) => out.push((path.rsplit('/').next().unwrap_or(path).to_string(), b)),
            Err(e) => println!("note: KH1 fx: {e} (Halloween Town keeps none of KH1's events there)"),
        }
    }
    // M13 stage 4: each KH1 Keyblade group's own trail and set.
    let mut group_files = Vec::new();
    let (Some(table), Some(seg)) = (table.as_ref(), segment) else {
        println!("note: KH1 fx: no btltbl.bin or PL0002.MDL's wep0 → k9: every Keyblade keeps the Kingdom Key's trail");
        return Ok(Some((out, group_files)));
    };
    let inst = install::kh1(game, keys.clone()).map_err(|e| format!("KH1 files: {e}"))?;
    for (card, group, model) in groups {
        let r = (|| -> Result<(fx::KeybladeOut, Vec<(String, Vec<u8>)>), String> {
            let item = kh1_recom_assets::stats::KH1_KEYBLADES.iter().find(|k| k.0 as u16 == *card).map(|k| k.2).ok_or("not a KH1 Keyblade")?;
            let w = table.weapon_for_item(item).map_err(|e| e.to_string())?;
            let file = format!("{model}.wpn");
            let (wpn, assets, _) = inst.read_full(&file).map_err(|e| format!("{file}: {e}"))?;
            let name = kh1_recom_assets::effects::keyblade_set_name(model);
            let mut set = kh1_recom_assets::effects::kh1_set(&name, &file, &wpn, kh1_recom_assets::effects::WPN_SECTION, &assets).map_err(|e| format!("{file}: {e}"))?;
            if !set.ids.contains(&(fx::TRAIL_ID as u32)) {
                return Err(format!("{file}: its set has no trail id {}", fx::TRAIL_ID));
            }
            // The set's own program sounds (none on the trail: Checked (files)), as its `.ese` when any map.
            let (rows, _) = kh1_recom_assets::effects::kh1_ese_rows(&set.data, sound_map).map_err(|e| e.to_string())?;
            if !rows.is_empty() {
                set.ese = Some(kh1_recom_assets::effects::write_ese(&rows));
            }
            let attach = kh1_recom_assets::effects::program_attach(&set.data, fx::TRAIL_ID as u32).map_err(|e| e.to_string())?.unwrap_or(0);
            if attach != 0 && attach != fx::ATTACH_BLADE {
                return Err(format!("{file}: its trail's attach code {attach:#x} is not read (only 0 and 0x30)"));
            }
            let trail = fx::KeybladeTrail { dpx: name.clone(), far: w.blade_far, near: w.blade_near, segment: seg, attach };
            let eet = fx::weapon_eet(&weapon, moves, blade_at(&w), Some(&trail), &mut fx::Added::default()).map_err(|e| e.to_string())?;
            let (eff, hd) = kh1_recom_assets::effects::group_eff(&set).map_err(|e| e.to_string())?;
            let entry = kh1_formats::egs::build_plain_entry(&eff, &hd, 0).map_err(|e| e.to_string())?;
            let eet_path = kh1_recom_assets::keyblade::resource_path(*group, "EET");
            let eff_path = kh1_recom_assets::keyblade::resource_path(*group, "EFF");
            let sv = fx::sound_variant(*card);
            let sound_variant = added.sounds.iter().any(|s| s.4 == sv).then_some(sv);
            let reach = [seg[0] * w.blade_far, seg[0] * w.blade_near];
            println!(
                "KH1 fx (M13 stage 4): card {card} → group {group}: {name} ({} bytes, ids {:?}, PC textures {:?}{}), trail attach {attach:#x}, nodes B {:.4} / C {:.4} m on wep0 (KH1 far {} / near {}), sounds {}",
                set.data.len(),
                set.ids,
                set.hd.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(),
                if set.ese.is_some() { ", with its .ese" } else { "" },
                reach[0],
                reach[1],
                w.blade_far,
                w.blade_near,
                sound_variant.map_or("the Kingdom Key's (no KH1 sound of its own found)".to_string(), |v| format!("at variant {v}"))
            );
            let out = fx::KeybladeOut { card: *card, group: *group, set: name, eet: eet_path.clone(), eff: eff_path.clone(), reach, sound_variant };
            Ok((out, vec![(eet_path, eet), (format!("raw/{eff_path}"), entry)]))
        })();
        match r {
            Ok((k, files)) => {
                added.keyblades.push(k);
                group_files.extend(files);
            }
            Err(e) => println!("note: KH1 fx: card {card} (group {group}): {e}: it keeps the Kingdom Key's trail (the shared PL0002.EET)"),
        }
    }
    Ok(Some((out, group_files)))
}

/// M13 stage 2's files: the SCD pack and `kh1-sounds.json`.
struct Sounds {
    pack: Vec<u8>,
    manifest: String,
    /// KH1 id → Re:CoM id of every SCD taken (stage 3's events use it).
    map: Vec<(u32, u32)>,
    weapon_base: u32,
    /// Each KH1 Keyblade card's weapon sound base whose `.se` was read (stage 4).
    weapon_bases: Vec<(u16, u32)>,
}

/// KH1's own sounds for Re:CoM's banks (`kh1_recom_assets::sounds`): every SCD of Sora's
/// `xa_ex_0010.mdls` and the Kingdom Key's 40 (its btltbl weapon row's `+0x34` base), looked up
/// in Sora's file, then the Keyblade's own `xw_ex_5010.se` (every SCD of it). A file that is not there
/// or does not read is noted and skipped.
fn sounds_build(game: &Path, keys: kh1_formats::egs::KeyTable) -> Result<Sounds, String> {
    use kh1_recom_assets::sounds as sd;
    let inst = install::kh1(game, keys).map_err(|e| format!("KH1 files: {e}"))?;
    let (_, sora, _) = inst.read_full(sd::SORA_MDLS).map_err(|e| format!("{}: {e}", sd::SORA_MDLS))?;
    let btl = inst.read("btltbl.bin").map_err(|e| format!("btltbl.bin: {e}"))?;
    let table = kh1_formats::btltbl::Btltbl::parse(btl.clone()).map_err(|e| format!("btltbl.bin: {e}"))?;
    let kk = table.weapon_for_item(sd::KINGDOM_KEY_ITEM).map_err(|e| format!("Kingdom Key: {e}"))?;
    let base = kk.sound_base;
    let mut wanted = sd::wanted(&sora, base);
    let mut sources = vec![sd::Source { file: sd::SORA_MDLS.into(), assets: sora, card: None }];
    let mut read = Vec::new();
    let mut weapon_bases = Vec::new();
    // Each KH1 Keyblade's own sound file (`<model>.se`, `xw_ex_5010.se`: its 40 SCDs and the few the
    // Keyblades share, 11018..; M13 stage 4: all 17, each tagged with its card). The world packs hold
    // none of them (the owner's run of 0.108.0): not read any more (about 600 MB).
    for (card, name, item) in kh1_recom_assets::stats::KH1_KEYBLADES {
        let w = match table.weapon_for_item(*item) {
            Ok(w) => w,
            Err(e) => {
                println!("note: KH1 sounds: {name}: {e} (skipped)");
                continue;
            }
        };
        let f = sd::weapon_file(&w.model);
        match inst.read_full(&f) {
            Ok((_, assets, _)) => {
                let n = assets.iter().filter(|a| sd::sound_id(&a.name).is_some()).count();
                read.push(format!("{f} ({n} SCDs)"));
                if *card == 1 {
                    let names: Vec<&str> = assets.iter().map(|a| a.name.as_str()).collect();
                    println!("KH1 sounds: {f}'s PC assets: {names:?}");
                }
                // Every SCD of the Keyblade's own file.
                wanted.extend(assets.iter().filter_map(|a| sd::sound_id(&a.name)));
                wanted.sort_unstable();
                wanted.dedup();
                weapon_bases.push((*card as u16, w.sound_base));
                sources.push(sd::Source { file: f, assets, card: Some(*card as u16) });
            }
            Err(e) => println!("note: KH1 sounds: {name}: {f}: {e} (skipped: its swings keep the Kingdom Key's KH1 sounds)"),
        }
    }
    let t = sd::take(&sources, &wanted).map_err(|e| e.to_string())?;
    for n in &t.notes {
        println!("note: {n}");
    }
    if t.sounds.is_empty() {
        return Err("no KH1 sound found".into());
    }
    let from = |file: &str| t.sounds.iter().filter(|s| s.source == file).count();
    println!(
        "KH1 sounds: {} of {} wanted as Re:CoM banks {}..{} ({} from {}, {} from the Keyblade's file {:?}); Kingdom Key base {base} ({base:#x}){}",
        t.sounds.len(),
        wanted.len(),
        sd::FIRST_BANK,
        sd::FIRST_BANK + t.sounds.len() as u32 - 1,
        from(sd::SORA_MDLS),
        sd::SORA_MDLS,
        t.sounds.len() - from(sd::SORA_MDLS),
        read,
        if t.missing.is_empty() { String::new() } else { format!("; NOT FOUND: {:?} (those play Re:CoM's sounds)", t.missing) }
    );
    let src = [("btltbl.bin sha256", kh1_formats::egs::sha256_hex(&btl))];
    let (pack, manifest) = sd::build(&t, base, &weapon_bases, &src);
    let map = t.sounds.iter().map(|s| (s.kh1_id, s.recom_id())).collect();
    Ok(Sounds { pack, manifest, map, weapon_base: base, weapon_bases })
}

/// The camp item row's files: the raw `FO0100.SPR` entry and `kh1-camp-rows.json`.
struct CampRows {
    entry: Vec<u8>,
    manifest: String,
}

/// Adds the green row with Re:CoM's item symbol to `camp:00` in Re:CoM's `FO0100.SPR` and in each
/// of its PC assets that is a `camp.lad` (any language prefix) or a whole container with one, as a
/// plain PKG entry with the same asset names, tags and date (`kh1_recom_assets::camp_rows`).
fn camp_rows_build(keys: kh1_formats::egs::KeyTable, arc: &kh1_formats::egs::Archive) -> Result<CampRows, String> {
    use kh1_recom_assets::camp_rows as cr;
    let e = arc.find(cr::SPR_PATH).ok_or_else(|| format!("{} not in Recom.hed", cr::SPR_PATH))?;
    let raw = arc.read_raw_entry(e).map_err(|e| format!("{}: {e}", cr::SPR_PATH))?;
    let date = i32::from_le_bytes(raw.get(12..16).ok_or("short entry")?.try_into().unwrap());
    let (spr, mut assets) = kh1_formats::egs::decode_entry_full(&keys, &raw).map_err(|e| format!("{}: {e}", cr::SPR_PATH))?;
    let sha = kh1_formats::egs::sha256_hex(&spr);
    let mut out = spr;
    let (group, files) = cr::patch_spr(&mut out, &mut assets).map_err(|e| e.to_string())?;
    let all: Vec<&str> = assets.iter().map(|a| a.name.as_str()).collect();
    println!(
        "camp item row: {} group {group:#x} = group {:#x} with sprite group {} (item symbol) for {} added to {files:?} (PC assets of the entry: {all:?})",
        cr::SET,
        cr::GREEN_ROW,
        cr::ITEM_SG,
        cr::MICKEY_SG
    );
    let entry = kh1_formats::egs::build_plain_entry(&out, &assets, date).map_err(|e| e.to_string())?;
    let manifest = cr::manifest_json(group, &files, &[("FO0100.SPR sha256", sha)]);
    Ok(CampRows { entry, manifest })
}

/// The shop tabs' files (M8d): the raw `FO0109.SPR` entry and `kh1-shop-tabs.json`.
struct ShopTabs {
    entry: Vec<u8>,
    manifest: String,
}

/// Adds the five-tab window and icon groups to `m_shop:01` in Re:CoM's `FO0109.SPR` and in each of
/// its PC assets that is an `m_shop.lad` (any language prefix) or a whole container with one, as a
/// plain PKG entry with the same asset names, tags and date (`kh1_recom_assets::shop_tabs`).
fn shop_tabs_build(keys: kh1_formats::egs::KeyTable, arc: &kh1_formats::egs::Archive) -> Result<ShopTabs, String> {
    use kh1_recom_assets::shop_tabs as st;
    let e = arc.find(st::SPR_PATH).ok_or_else(|| format!("{} not in Recom.hed", st::SPR_PATH))?;
    let raw = arc.read_raw_entry(e).map_err(|e| format!("{}: {e}", st::SPR_PATH))?;
    let date = i32::from_le_bytes(raw.get(12..16).ok_or("short entry")?.try_into().unwrap());
    let (spr, mut assets) = kh1_formats::egs::decode_entry_full(&keys, &raw).map_err(|e| format!("{}: {e}", st::SPR_PATH))?;
    let sha = kh1_formats::egs::sha256_hex(&spr);
    let mut out = spr;
    let (first, files) = st::patch_spr(&mut out, &mut assets).map_err(|e| e.to_string())?;
    let all: Vec<&str> = assets.iter().map(|a| a.name.as_str()).collect();
    println!("shop tabs: {} groups {first}..{} (5-tab windows, closes, icons; the card trade's 6-cell strips) added to {files:?} (PC assets of the entry: {all:?})", st::SET, first + st::group::COUNT - 1);
    let entry = kh1_formats::egs::build_plain_entry(&out, &assets, date).map_err(|e| e.to_string())?;
    let manifest = st::manifest_json(first, &files, &[("FO0109.SPR sha256", sha)]);
    Ok(ShopTabs { entry, manifest })
}

/// The KH1 HUD's files (stage H1): the raw `FO0002.RTM` entry and `kh1-hud.json`.
struct Hud {
    rtm_entry: Vec<u8>,
    manifest: String,
}

/// Builds the `kh1hud` set from KH1's `uitex.bin` (+ PC assets), `UK_uibin_g_s.bin`,
/// `UK_uibin_g_red.bin` and the party's faces, adds `kh1hud.lad` (+ `<LANG>_kh1hud.lad`) to the MP
/// bar's `FO0002.SPR` and `kh1hud.imz` (+ `<LANG>_kh1hud.imz`) to Re:CoM's `FO0002.RTM`
/// (`docs/recom-kh1-hud.md`).
fn kh1_hud_build(game: &Path, keys: kh1_formats::egs::KeyTable, arc: &kh1_formats::egs::Archive, bar: &mut MpBar) -> Result<Hud, String> {
    use kh1_recom_assets::kh1_hud as kh;
    use kh1_recom_assets::mp_bar as mb;
    let inst = install::kh1(game, keys.clone()).map_err(|e| format!("KH1 files: {e}"))?;
    let (uitex, uitex_hd, _) = inst.read_full(mb::KH1_UITEX).map_err(|e| format!("{}: {e}", mb::KH1_UITEX))?;
    let g_s = inst.read(kh::G_S).map_err(|e| format!("{}: {e}", kh::G_S))?;
    let g_red = inst.read(kh::G_RED).map_err(|e| format!("{}: {e}", kh::G_RED))?;
    let g_act = inst.read(kh::G_ACT).map_err(|e| format!("{}: {e}", kh::G_ACT))?;
    // H5: the LOCK ON! plate and the target gauge.
    let lock = inst.read(kh::LOCK).ok();
    let g_enm = inst.read(kh::G_ENM).ok();
    // H6: the party's gauges.
    let g_c = inst.read(kh::G_C).ok();
    // H8: the LEVEL UP!! box.
    let lev_s = inst.read(kh::LEV_S).ok();
    // H4: the command menu's layouts, KH1's menu font (Image/dt/Theater) and the words.
    let menu_files = kh1_menu_files(game, &keys, &inst);
    let mut faces = Vec::new();
    for n in kh::FACES {
        let (m, a, _) = inst.read_full(n).map_err(|e| format!("{n}: {e}"))?;
        faces.push(kh::face(&m, &a).map_err(|e| format!("{n}: {e}"))?);
    }
    let faces: [kh::Source; 3] = faces.try_into().map_err(|_| "faces")?;
    let langs = kh::languages(&bar.assets);
    if langs.is_empty() {
        return Err("FO0002.SPR has no <LANG>_Cockpit.lad PC layouts (no language to build for)".into());
    }
    let lrefs: Vec<&str> = langs.iter().map(String::as_str).collect();
    let menu = match &menu_files {
        Ok(m) => Some(kh::MenuSources {
            com: &m.com,
            com_battle: &m.com_battle,
            mag: &m.mag,
            font: [&m.font[0], &m.font[1]],
            words: m.words.clone(),
            icon: m.icon.as_ref().map(|[a, b]| [a.as_slice(), b.as_slice()]),
            cut: m.cut.as_deref(),
            item: m.item.as_deref(),
            sum: m.sum.as_deref(),
        }),
        Err(e) => {
            println!("note: KH1 HUD: no KH1 command menu look: {e}");
            None
        }
    };
    let src = kh::Sources { uitex: &uitex, uitex_hd: &uitex_hd, g_s: &g_s, g_red: &g_red, g_act: &g_act, menu, lock: lock.as_deref(), g_enm: g_enm.as_deref(), g_c: g_c.as_deref(), lev_s: lev_s.as_deref(), faces };
    let b = kh::build(&src, &lrefs).map_err(|e| e.to_string())?;
    bar.spr = kh::add_to_spr(&bar.spr, &mut bar.assets, &b, &langs).map_err(|e| format!("{}: {e}", mb::SPR_PATH))?;
    // Re:CoM's HP ring and bar turned 180° for KH1's place (`hud_recom_hp_kh1_place`): new groups
    // in every copy of `gauge:01`. Without them the DLL leaves Re:CoM's gauge where it is.
    let recom_hp = match kh1_recom_assets::recom_hp::patch_spr(&mut bar.spr, &mut bar.assets) {
        Ok((first, files)) => {
            println!("KH1 HUD: Re:CoM's HP ring and bar turned 180° as {} groups {first}..{} in {files:?}", kh1_recom_assets::recom_hp::SET, first + 2);
            Some(first)
        }
        Err(e) => {
            println!("note: KH1 HUD: Re:CoM's HP gauge not turned ({e}); hud_recom_hp_kh1_place will leave it top-left");
            None
        }
    };
    let e = arc.find(mb::RTM_PATH).ok_or_else(|| format!("{} not in Recom.hed", mb::RTM_PATH))?;
    let raw = arc.read_raw_entry(e).map_err(|e| format!("{}: {e}", mb::RTM_PATH))?;
    let date = i32::from_le_bytes(raw.get(12..16).ok_or("short entry")?.try_into().unwrap());
    let (rtm, mut rtm_assets) = kh1_formats::egs::decode_entry_full(&keys, &raw).map_err(|e| format!("{}: {e}", mb::RTM_PATH))?;
    let rtm_new = kh::add_to_rtm(&rtm, &mut rtm_assets, &b, &langs).map_err(|e| format!("{}: {e}", mb::RTM_PATH))?;
    let rtm_entry = kh1_formats::egs::build_plain_entry(&rtm_new, &rtm_assets, date).map_err(|e| e.to_string())?;
    let sources = [
        ("uitex.bin sha256", kh1_formats::egs::sha256_hex(&uitex)),
        ("UK_uibin_g_s.bin sha256", kh1_formats::egs::sha256_hex(&g_s)),
        ("UK_uibin_g_red.bin sha256", kh1_formats::egs::sha256_hex(&g_red)),
        ("UK_uibin_g_act.bin sha256", kh1_formats::egs::sha256_hex(&g_act)),
        ("FO0002.RTM sha256", kh1_formats::egs::sha256_hex(&rtm)),
    ];
    let manifest = kh::manifest_json(&b, &langs, &sources, recom_hp);
    if let Some(m) = &b.menu {
        println!("KH1 HUD: command menu look from group {}: {} glyphs of KH1's menu font, words for {:?}{}", m.first, m.glyphs, m.words.iter().map(|w| w.0.as_str()).collect::<Vec<_>>(), if m.missing.is_empty() { String::new() } else { format!("; no glyph for {:?} (drawn as a space)", m.missing) });
        println!("KH1 HUD: FM context tab from uitex.bin picture 2's PC rows; Triangle {}", if m.tab_icon { "from Theater's Icon.tm2 (the PlayStation glyph)" } else { "MISSING (no Icon.tm2 / FontIcon.inf): the tab shows without it" });
        match m.items_list {
            Some(g) => println!("KH1 HUD: Items list (M8 stage 8a.1) from group {g}: {}", kh::ITEM),
            None => println!("note: KH1 HUD: no Items list panels (no {}): the DLL draws the Items list with the Magic list's", kh::ITEM),
        }
        match (m.summon_list, &m.summon_note) {
            (Some(g), _) => println!("KH1 HUD: Summon list (M8b) from group {g}: {}", kh::SUM),
            (None, Some(n)) => println!("note: KH1 HUD: no Summon list panels ({n}): the DLL draws the Summon list with the Magic list's"),
            (None, None) => println!("note: KH1 HUD: no Summon list panels (no {}): the DLL draws the Summon list with the Magic list's", kh::SUM),
        }
    }
    match &b.shortcut {
        Some(sc) => println!(
            "KH1 HUD: shortcut panel (M7 stage 7.12) from group {}: {} (picture 0's frame, picture 1's title); button glyphs {}{}",
            sc.first,
            kh::CUT,
            if sc.icons { "Triangle / Square / Cross / Circle from Theater's Icon.tm2" } else { "MISSING (no Icon.tm2 / FontIcon.inf): the panel shows without them" },
            match sc.pads {
                Some(p) => format!("; the other pad types' (Xbox Y / X / A / B, the generic pad's) from group {p}"),
                None => String::new(),
            }
        ),
        None => println!("note: KH1 HUD: no shortcut panel (no {} or no command menu look): the DLL's shortcuts cast without a panel", kh::CUT),
    }
    println!(
        "KH1 HUD: {} groups ({} motion units), {} sprites, {} atlas items; PS2 atlas {}×{}, PC atlas {}×{}; kh1hud.lad + {} language copies into FO0002.SPR, kh1hud.imz + {} into FO0002.RTM ({:?}); {} PC texture rects redrawn from the PS2 pictures (repacked on PC)",
        b.groups,
        b.motion.first.len(),
        b.sprites,
        b.pieces,
        b.atlas.0,
        b.atlas.1,
        b.atlas.2,
        b.atlas.3,
        langs.len(),
        langs.len(),
        langs,
        b.refit
    );
    let (rf, rs) = b.ring;
    println!(
        "KH1 HUD: MP ring built as Re:CoM's HP ring from group {rf} ({} groups: KH1's quarter pictures whole, scissor parts, turned max-MP end, closing outline); cuts between cells: {} scissor cells with texels on both sides (largest alpha {}), charge cuts inside a cell: {} (staircases)",
        kh::ring_group::COUNT,
        rs.gap_conflicts,
        rs.gap_alpha,
        rs.cell_conflicts
    );
    Ok(Hud { rtm_entry, manifest })
}

/// The command menu look's KH1 inputs (H4).
struct MenuFiles {
    com: Vec<u8>,
    com_battle: Vec<u8>,
    mag: Vec<u8>,
    font: [Vec<u8>; 2],
    words: Vec<(String, Vec<(u16, String)>)>,
    /// `Icon.tm2`, `FontIcon.inf` (`Image/dt/Theater`), for the context tab's Triangle.
    icon: Option<[Vec<u8>; 2]>,
    /// `UK_uibin_cut.bin`, the shortcut panel (stage 7.12).
    cut: Option<Vec<u8>>,
    /// `UK_uibin_item.bin`, the Items list (M8 stage 8a.1).
    item: Option<Vec<u8>>,
    /// `UK_uibin_sum.bin`, the Summon list (M8b).
    sum: Option<Vec<u8>>,
}

fn kh1_menu_files(game: &Path, keys: &kh1_formats::egs::KeyTable, inst: &kh1_formats::egs::Install) -> Result<MenuFiles, String> {
    use kh1_recom_assets::command_menu as cm;
    use kh1_recom_assets::kh1_hud as kh;
    let rd = |n: &str| inst.read(n).map_err(|e| format!("{n}: {e}"));
    let (com, com_battle, mag) = (rd(kh::COM)?, rd(kh::COM_BATTLE)?, rd(kh::MAG)?);
    // The PC's button icons (the context tab's Triangle): `Image/dt/Theater`.
    let icon = (|| -> Result<[Vec<u8>; 2], String> {
        let theater = kh1_formats::egs::Archive::open(game.join("Image").join("dt").join("Theater.hed")).map_err(|e| format!("Image/dt/Theater.hed: {e}"))?;
        let [a, b] = kh::ICON_FILES.map(|n| theater.read(keys, n).map_err(|e| format!("Theater {n}: {e}")));
        Ok([a?, b?])
    })();
    let icon = match icon {
        Ok(i) => Some(i),
        Err(e) => {
            println!("note: KH1 HUD: no Triangle for the context tab: {e}");
            None
        }
    };
    let (_, sys_assets, _) = inst.read_full(kh::SYSFONT).map_err(|e| format!("{}: {e}", kh::SYSFONT))?;
    let dds = sys_assets.iter().find(|a| a.name.eq_ignore_ascii_case(kh::SYSFONT_HD)).ok_or_else(|| format!("{}: no PC asset {}", kh::SYSFONT, kh::SYSFONT_HD))?.data.clone();
    let font = [dds, rd(kh::FONT_TABLE)?];
    let (_, assets, _) = inst.read_full("btltbl.bin").map_err(|e| format!("btltbl.bin: {e}"))?;
    let mut words = Vec::new();
    for (lang, prefix) in cm::LANGUAGES {
        if let Some(a) = assets.iter().find(|a| a.name.eq_ignore_ascii_case(&format!("{prefix}Word.bin"))) {
            words.push((lang.to_string(), cm::words(&a.data).0));
        }
    }
    let cut = match rd(kh::CUT) {
        Ok(c) => Some(c),
        Err(e) => {
            println!("note: KH1 HUD: no shortcut panel: {e}");
            None
        }
    };
    let item = match rd(kh::ITEM) {
        Ok(c) => Some(c),
        Err(e) => {
            println!("note: KH1 HUD: no KH1 Items list panels (the DLL draws it with the Magic list's): {e}");
            None
        }
    };
    let sum = match rd(kh::SUM) {
        Ok(c) => Some(c),
        Err(e) => {
            println!("note: KH1 HUD: no KH1 Summon list panels (the DLL draws it with the Magic list's): {e}");
            None
        }
    };
    Ok(MenuFiles { com, com_battle, mag, font, words, icon, cut, item, sum })
}

/// KH1's command words (`btltbl.bin`'s PC assets `<LANG>_Word.bin`, decoded for the menu's
/// commands) and the plates found in Re:CoM's `Cockpit:00` (base file and language copies):
/// `kh1-command-menu.json` (M7 stage 7.3, `docs/recom-m7-command-menu.md`).
fn command_menu_build(game: &Path, keys: kh1_formats::egs::KeyTable, arc: &kh1_formats::egs::Archive) -> Result<String, String> {
    use kh1_recom_assets::command_menu as cm;
    let inst = install::kh1(game, keys.clone()).map_err(|e| format!("KH1 files: {e}"))?;
    let (btl, assets, _) = inst.read_full("btltbl.bin").map_err(|e| format!("btltbl.bin: {e}"))?;
    let mut langs = Vec::new();
    for (lang, prefix) in cm::LANGUAGES {
        let name = format!("{prefix}Word.bin");
        let Some(a) = assets.iter().find(|a| a.name.eq_ignore_ascii_case(&name)) else {
            println!("note: btltbl.bin has no PC asset {name}: no {lang} words");
            continue;
        };
        let (w, missing) = cm::words(&a.data);
        if !missing.is_empty() {
            println!("note: {name}: messages {missing:?} not decoded (the DLL falls back to US English for them)");
        }
        langs.push((*lang, w));
    }
    if !langs.iter().any(|(l, _)| *l == "us") {
        return Err("btltbl.bin has no US_Word.bin (the fallback language)".into());
    }
    // The plates: what the bar's patch added to the base file and to each language copy.
    let e = arc.find(kh1_recom_assets::mp_bar::SPR_PATH).ok_or("FO0002.SPR not in Recom.hed")?;
    let raw = arc.read_raw_entry(e).map_err(|e| e.to_string())?;
    let (spr, spr_assets) = kh1_formats::egs::decode_entry_full(&keys, &raw).map_err(|e| e.to_string())?;
    let base = cm::probe_plates(&spr).ok_or("FO0002.SPR: no Cockpit.lad")?;
    for a in spr_assets.iter().filter(|a| !a.data.starts_with(b"LAYD") || a.name.to_ascii_lowercase().ends_with("cockpit.lad")) {
        if let Some(p) = cm::probe_plates(&a.data) {
            if !p.found {
                println!("note: FO0002.SPR asset {}: Re:CoM's text plate is not where the base file has it; its menu rows have no plates", a.name);
            }
            if !p.arrow {
                println!("note: FO0002.SPR asset {}: Re:CoM's arrow triangle (UV 16, 49 - 32, 64 x the atlas scale) not found; no pointing arrow with this copy", a.name);
            }
        }
    }
    for (lang, w) in &langs {
        let show: Vec<String> = w.iter().filter(|(id, _)| [1u16, 2, 3, 7, 343].contains(id)).map(|(id, t)| format!("{id} {t:?}")).collect();
        println!("command menu: {lang} words {} ({})", w.len(), show.join(", "));
    }
    println!(
        "command menu: plates {} (highlight vertex colour {:#010x}), pointing arrow {}, groups after the MP bar's",
        if base.found { "from Re:CoM's text plate" } else { "NOT found (empty groups)" },
        base.colour,
        if base.arrow { "from Re:CoM's lock-on triangle" } else { "NOT found (an empty group)" }
    );
    let sources = [("btltbl.bin sha256", kh1_formats::egs::sha256_hex(&btl)), ("FO0002.SPR sha256", kh1_formats::egs::sha256_hex(&spr))];
    Ok(cm::manifest_json(&langs, kh1_recom_assets::mp_bar::group::COUNT, base, &sources))
}

/// KH1's command menu sounds (`menu_sounds`): KH1's system sound bank, the PC asset
/// `se000.win32.scd` of `amusic/se000.dat` (bank 2, sound `id` = entry `id`), unchanged, as
/// `kh1-menu-sounds.bin` + `.json`, and the menu entries as WAVs for listening.
#[allow(clippy::type_complexity)]
fn menu_sounds_build(game: &Path, keys: kh1_formats::egs::KeyTable) -> Result<(Vec<u8>, String, Vec<(String, Vec<u8>)>), String> {
    use kh1_recom_assets::menu_sounds as ms;
    let inst = install::kh1(game, keys).map_err(|e| format!("KH1 files: {e}"))?;
    let (data, assets, _) = inst.read_full(ms::KH1_FILE).map_err(|e| format!("{}: {e}", ms::KH1_FILE))?;
    let mut notes = Vec::new();
    let taken = ms::take(&assets, &mut notes).map_err(|e| e.to_string())?;
    for l in ms::lines(&taken) {
        println!("{l}");
    }
    for n in &notes {
        println!("note: {n}");
    }
    if ms::present(&taken).is_empty() {
        return Err(format!("{} has no entry for any menu sound", taken.name));
    }
    let sources = [("se000.dat sha256", kh1_formats::egs::sha256_hex(&data))];
    let (bin, json) = ms::build(&taken, &sources);
    Ok((bin, json, ms::previews(&taken)))
}

fn kh1_stats(game: &std::path::Path, keys: kh1_formats::egs::KeyTable, arc: &kh1_formats::egs::Archive, models: &[KeybladeModel], kh2: &[Kh2Row]) -> Result<kh1_recom_assets::stats::StatsJson, String> {
    // Re:CoM's enemy stats files (`ENEMY/<id>/EN<id>.EPD`), for the proxies' ratios.
    let mut recom = |name: &str| match arc.find(name) {
        None => None,
        Some(_) => match install::read_recom(arc, &keys, name) {
            Ok(b) => Some(b),
            Err(e) => {
                println!("note: {name}: {e}");
                None
            }
        },
    };
    let inst = install::kh1(game, keys.clone()).map_err(|e| format!("KH1 files: {e}"))?;
    let btl = inst.read("btltbl.bin").map_err(|e| format!("btltbl.bin: {e}"))?;
    let parsed = kh1_formats::btltbl::Btltbl::parse(btl.clone()).map_err(|e| format!("btltbl.bin: {e}"))?;
    // The Kingdom Key (item 0x51): its weapon row's Strength (+0x30).
    let kk = parsed.weapon_for_item(0x51).map_err(|e| format!("Kingdom Key: {e}"))?.strength as i32;
    let mut read = |name: &str| inst.read(name).map_err(|e| kh1_recom_assets::Error::Format(format!("{name}: {e}")));
    kh1_recom_assets::stats::build_json(&mut read, &mut recom, &btl, kk, models, kh2).map_err(|e| e.to_string())
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    write_quiet(path, bytes)?;
    println!("wrote {} ({} bytes)", path.display(), bytes.len());
    Ok(())
}

/// Writes a file (and its folders) and records it in the run's [`Report::written`].
fn write_quiet(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    WRITTEN.with(|w| {
        let mut w = w.borrow_mut();
        if !w.iter().any(|p| p == path) {
            w.push(path.to_path_buf());
        }
    });
    Ok(())
}

/// Deletes `path` only if it is a pack this step wrote earlier (an `.ANN` with more than
/// Re:CoM's 161 slots; Re:CoM's own have 161 and 158); anything else is left alone.
fn remove_stale(path: &Path) -> Result<(), String> {
    let Ok(b) = std::fs::read(path) else { return Ok(()) };
    let ours = Ann::parse(&b).is_ok_and(|a| a.slots.len() > RECOM_SORA_SLOTS);
    if ours {
        std::fs::remove_file(path).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("removed {} (an earlier extended pack; Re:CoM's own is used now)", path.display());
    } else {
        println!("note: {} exists and was not written by this step; left as it is", path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_starts_with_reading_and_stops_with_the_run() {
        assert_eq!(STAGES.len(), stage::STATS + 1);
        assert!(STAGES.iter().all(|s| !s.is_empty() && s.is_ascii()));
        let seen = std::rc::Rc::new(RefCell::new(Vec::new()));
        let s2 = seen.clone();
        set_progress(Some(Box::new(move |p| s2.borrow_mut().push(p))));
        let dir = std::env::temp_dir().join(format!("kh1-step-progress-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        set_sink(Some(Box::new(|_| {})));
        // No game here: the run stops at its first read, after its first report.
        assert!(run(&Options::new(dir.clone(), Dest::Root(dir.join("out")))).is_err());
        set_sink(None);
        set_progress(None);
        assert_eq!(seen.borrow().first(), Some(&Progress { stage: stage::READ, part: None }));
        assert_eq!(seen.borrow().len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
