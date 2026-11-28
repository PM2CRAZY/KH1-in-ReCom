//! The first-launch build of the game-made files (owner, 2026-11-05: "DLL builds on first
//! launch"; `docs/openkh-github-install.md`). The rules here are plain Rust (tested on Linux);
//! the Windows side (the helper process, the title's hold and its progress lines) is `win::assets`.
//!
//! - **Where the files are used:** Panacea's live folder `<mod_path>/recom` (the folder
//!   Panacea's `OnInit` names). Panacea lists that folder once, when the game sets its replace
//!   path, after every DLL's `OnInit` (Checked (source): `Panacea.cpp` `FakePackageFile`,
//!   `LoadDLLs` from `DllMain`). So files placed there during `OnInit` are used in the same run.
//! - **Where they are kept:** OpenKH's Build deletes the live folder (`ModsService.RunPatcherAsync`,
//!   Checked (source)), so the build goes to a cache Build never touches: `generated/` in the
//!   mod's own folder ([`find_mod_folder`]; ignored by git, so OpenKH's update keeps it), else
//!   `kh1-recom-generated/` in the game folder. Each start copies whatever a Build removed back
//!   into the live folder ([`kh1_recom_assets::stamp::place`]).
//! - **When it builds:** when the cache has no stamp for this step and these inputs
//!   ([`decide`]): first launch, a mod update that changed the step, a game update.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use kh1_recom_assets::stamp::{self, Input, Stamp, FAILED_FILE, STAMP_FILE};
use kh1_recom_assets::step;

/// The cache folder's name in the mod's own folder (in the repo's `.gitignore`).
pub const CACHE_DIR: &str = "generated";
/// The cache folder's name in the game folder, when the mod's own folder is not found.
pub const GAME_CACHE_DIR: &str = "kh1-recom-generated";
/// The build's own log, in the game folder (next to `kh1-recom.log`).
pub const BUILD_LOG: &str = "kh1-recom-build.log";
/// The job the DLL hands its helper process, in the game folder.
pub const JOB_FILE: &str = "kh1-recom-build.job";
/// The GitHub repository OpenKH installs (`<mods>/Recom/<owner>/<repo>`).
pub const REPO: (&str, &str) = ("PM2CRAZY", "KH1-in-ReCom");
/// The DLL as the repository carries it (the CI bot's build), and as the owner's local mod does.
pub const DLL_SOURCES: [&str; 2] = ["openkh/kh1-recom.dll", "kh1-recom.dll"];

/// A top-level `key: value` of a YAML file (OpenKH's `mods-manager.yml`), quotes removed.
pub fn yaml_value(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let l = l.trim_end_matches('\r');
        let (k, v) = l.split_once(':')?;
        if k != key {
            return None;
        }
        let v = v.trim();
        let v = if v.len() >= 2 && ((v.starts_with('\'') && v.ends_with('\'')) || (v.starts_with('"') && v.ends_with('"'))) { &v[1..v.len() - 1] } else { v };
        let v = v.replace("''", "'");
        (!v.is_empty() && v != "~" && v != "null").then_some(v)
    })
}

/// The folders OpenKH keeps Re:CoM's installed mods in, given OpenKH's own folder: as the Mods
/// Manager's `InstalledModsPath` computes it from `mods-manager.yml` (`installedModsPath`, unless
/// it equals `compiledModPath`), then the default `<openkh>/mods/Recom`. Checked (source:
/// `ConfigurationService.InstalledModsPath`, `LaunchGame` "Recom").
pub fn installed_mods_dirs(openkh: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(t) = std::fs::read_to_string(openkh.join("mods-manager.yml")) {
        let inst = yaml_value(&t, "installedModsPath");
        let comp = yaml_value(&t, "compiledModPath");
        match inst {
            Some(i) if comp.as_deref() != Some(i.as_str()) => out.push(PathBuf::from(i).join("Recom")),
            Some(i) => out.push(PathBuf::from(i).join("mods").join("Recom")),
            None => {}
        }
    }
    let d = openkh.join("mods").join("Recom");
    if !out.contains(&d) {
        out.push(d);
    }
    out
}

/// This mod's own folder among the installed Re:CoM mods (`<dir>/<owner>/<repo>` with a
/// `mod.yml`): the one whose DLL source has the running DLL's size (Build copies it unchanged),
/// else OpenKH's GitHub name for it ([`REPO`]). Only folders whose `mod.yml` names
/// `kh1-recom.dll` count.
pub fn find_mod_folder(mods_dirs: &[PathBuf], dll_size: Option<u64>) -> Option<PathBuf> {
    let mut best: Option<(u8, PathBuf)> = None;
    for root in mods_dirs {
        let Ok(owners) = std::fs::read_dir(root) else { continue };
        for o in owners.flatten() {
            let Ok(repos) = std::fs::read_dir(o.path()) else { continue };
            for r in repos.flatten() {
                let dir = r.path();
                let Ok(yml) = std::fs::read_to_string(dir.join("mod.yml")) else { continue };
                if !yml.contains("kh1-recom.dll") {
                    continue;
                }
                let named = o.file_name().to_string_lossy().eq_ignore_ascii_case(REPO.0) && r.file_name().to_string_lossy().eq_ignore_ascii_case(REPO.1);
                let same_dll = dll_size.is_some_and(|n| DLL_SOURCES.iter().any(|s| std::fs::metadata(dir.join(s)).is_ok_and(|m| m.len() == n)));
                let score = (same_dll as u8) * 2 + named as u8;
                if score > 0 && best.as_ref().is_none_or(|(b, _)| score > *b) {
                    best = Some((score, dir));
                }
            }
        }
    }
    best.map(|b| b.1)
}

/// The cache folder: `generated/` in the mod's own folder, else `kh1-recom-generated/` in the
/// game folder.
pub fn cache_dir(mod_folder: Option<&Path>, game: &Path) -> PathBuf {
    match mod_folder {
        Some(m) => m.join(CACHE_DIR),
        None => game.join(GAME_CACHE_DIR),
    }
}

/// What to do at start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    /// `asset_build = off`.
    Off,
    /// The cache holds this build: copy what the live folder lacks.
    Place(Stamp),
    /// Build now (why, for the log and the window).
    Build(String),
    /// This very build failed before (its reason): not tried again until the step or the inputs
    /// change, or the marker is deleted.
    Failed(String),
}

/// The decision for a cache, the step's hash and the game's inputs.
pub fn decide(enabled: bool, cache: &Path, step_hash: &str, inputs: &[Input]) -> Decision {
    if !enabled {
        return Decision::Off;
    }
    let ih = stamp::inputs_hash(inputs);
    let old = std::fs::read_to_string(cache.join(STAMP_FILE)).ok().and_then(|t| Stamp::parse(&t));
    if let Some(st) = &old {
        if st.is_for(step_hash, &ih) {
            return match stamp::cache_complete(cache, st) {
                Ok(()) => Decision::Place(st.clone()),
                Err(e) => Decision::Build(format!("the built files are incomplete ({e})")),
            };
        }
    }
    if let Ok(t) = std::fs::read_to_string(cache.join(FAILED_FILE)) {
        if Stamp::parse(&t).is_some_and(|s| s.is_for(step_hash, &ih)) {
            return Decision::Failed(stamp::failed_reason(&t));
        }
    }
    Decision::Build(match old {
        None => "the KH1 files have not been built yet".into(),
        Some(st) if st.step != step_hash => "the mod's asset step changed (a mod update)".into(),
        Some(_) => "the game's files changed (a game update)".into(),
    })
}

/// The helper process's job: what to build, from where, to where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Job {
    /// The KH 1.5+2.5 folder.
    pub game: PathBuf,
    /// The cache folder the files go to.
    pub cache: PathBuf,
    /// The build's log.
    pub log: PathBuf,
    /// [`stamp::inputs_hash`] of the inputs the decision saw.
    pub inputs: String,
    /// Why it builds.
    pub reason: String,
    /// Panacea's live folder the built files are copied into after the build (empty: none).
    pub live: PathBuf,
    /// The progress file the game reads ([`Shown`]; empty: none).
    pub progress: PathBuf,
}

impl Job {
    /// The job file's text (`key=value` lines).
    pub fn to_text(&self) -> String {
        format!(
            "game={}\ncache={}\nlog={}\ninputs={}\nreason={}\nlive={}\nprogress={}\n",
            self.game.display(),
            self.cache.display(),
            self.log.display(),
            self.inputs,
            self.reason.replace('\n', " "),
            self.live.display(),
            self.progress.display()
        )
    }

    /// Reads a job file's text.
    pub fn parse(text: &str) -> Option<Job> {
        let get = |k: &str| text.lines().map(|l| l.trim_end_matches('\r')).find_map(|l| l.strip_prefix(k)?.strip_prefix('=')).map(str::to_string);
        Some(Job {
            game: get("game")?.into(),
            cache: get("cache")?.into(),
            log: get("log")?.into(),
            inputs: get("inputs")?,
            reason: get("reason").unwrap_or_default(),
            live: get("live").unwrap_or_default().into(),
            progress: get("progress").unwrap_or_default().into(),
        })
    }
}

/// Re:CoM's title, where the game waits for the build (`win::assets`; `sites::TITLE_HOLD_HOOK`).
/// Checked (decomp + disasm): `CTitle` (ctor `0x14039F770`) is a `CFSM` of 14 states; state 0's
/// update `0x1403A3770` waits for the loader's queue to empty (`0x14023CA80(0, 0)`), then state 1's
/// `0x1403A3820` plays the title (sub-state `+0xAC` 0), waits for its sequence's end or a confirm /
/// cancel / Start press (10), fades (0x14) and goes to the menu (state 2, `+0x168`).
pub mod title {
    /// `CTitle`'s state-1 update (RVA).
    pub const STATE1_UPDATE: u32 = 0x003A_3820;
    /// Its sub-state (`i32`).
    pub const SUB: usize = 0xAC;
    /// The sub-state that waits for a press (the title shown).
    pub const WAIT: i32 = 10;
}

/// The progress file the build writes for the game window (in the game folder, next to the job).
pub const PROGRESS_FILE: &str = "kh1-recom-build.progress";
const PROGRESS_MAGIC: &str = "kh1-recom-build progress 1";

/// The build's stages as the game window counts them: the step's ([`step::STAGES`]), then the copy
/// into Panacea's folder.
pub fn stage_count() -> usize {
    step::STAGES.len() + 1
}

/// The last stage's label (the copy into Panacea's folder).
pub const PLACE_STAGE: &str = "copying the files into the mod folder";

/// A stage's label (`stage_count() - 1` is [`PLACE_STAGE`]).
pub fn stage_label(stage: usize) -> &'static str {
    step::STAGES.get(stage).copied().unwrap_or(PLACE_STAGE)
}

/// The percentage shown for stage `stage` of `of` begun (part `k` of `n` of it done): stages count
/// alike, parts split their stage; under 100 until the build is done.
pub fn percent(stage: usize, of: usize, part: Option<(usize, usize)>) -> u32 {
    if of == 0 {
        return 0;
    }
    let frac = part.filter(|&(_, n)| n > 0).map_or(0.0, |(k, n)| k.min(n) as f64 / n as f64);
    (((stage.min(of) as f64 + frac) / of as f64 * 100.0).floor() as u32).min(99)
}

/// What the game window shows of the build.
#[derive(Clone, Debug, PartialEq)]
pub enum Shown {
    /// Building: stage `stage` (from 0) of `of`, the label, the percentage.
    Running { stage: usize, of: usize, label: String, percent: u32 },
    /// Built and copied: the files, the seconds.
    Done { files: usize, secs: u32 },
    /// Not built: the reason (the failed marker's).
    Failed { reason: String },
}

impl Shown {
    /// The state at stage `stage` (part `k` of `n`).
    pub fn at(stage: usize, part: Option<(usize, usize)>) -> Shown {
        let of = stage_count();
        Shown::Running { stage, of, label: stage_label(stage).to_string(), percent: percent(stage, of, part) }
    }

    /// Before the first report.
    pub fn starting() -> Shown {
        Shown::Running { stage: 0, of: stage_count(), label: "starting".into(), percent: 0 }
    }

    /// The progress file's text.
    pub fn to_text(&self) -> String {
        let one = |s: &str| s.replace(['\r', '\n'], " ");
        match self {
            Shown::Running { stage, of, label, percent } => format!("{PROGRESS_MAGIC}\nrunning {stage} {of} {percent} {}\n", one(label)),
            Shown::Done { files, secs } => format!("{PROGRESS_MAGIC}\ndone {files} {secs}\n"),
            Shown::Failed { reason } => format!("{PROGRESS_MAGIC}\nfailed {}\n", one(reason)),
        }
    }

    /// Reads a progress file's text.
    pub fn parse(text: &str) -> Option<Shown> {
        let mut lines = text.lines().map(|l| l.trim_end_matches('\r'));
        if lines.next()? != PROGRESS_MAGIC {
            return None;
        }
        let l = lines.next()?;
        let (kind, rest) = l.split_once(' ')?;
        match kind {
            "running" => {
                let mut f = rest.splitn(4, ' ');
                let stage = f.next()?.parse().ok()?;
                let of = f.next()?.parse().ok()?;
                let percent = f.next()?.parse().ok()?;
                Some(Shown::Running { stage, of, label: f.next().unwrap_or_default().to_string(), percent })
            }
            "done" => {
                let (a, b) = rest.split_once(' ')?;
                Some(Shown::Done { files: a.parse().ok()?, secs: b.parse().ok()? })
            }
            "failed" => Some(Shown::Failed { reason: rest.to_string() }),
            _ => None,
        }
    }

    /// The two lines the game window shows (ASCII: Re:CoM's text expands bytes from 0x81 as its
    /// own escapes, `0x1403480C0`).
    pub fn lines(&self) -> (String, String) {
        let ascii = |s: String| s.chars().map(|c| if c.is_ascii() && !c.is_ascii_control() { c } else { '?' }).collect::<String>();
        let (a, b) = match self {
            Shown::Running { stage, of, label, percent } => {
                (format!("KH1 in Re:CoM: building files from your games... {percent} %"), format!("Step {} of {of}: {label}. The title waits until it is done.", (stage + 1).min(*of)))
            }
            Shown::Done { files, secs } => ("KH1 in Re:CoM: done. The KH1 files are ready.".to_string(), format!("{files} files in {secs} s.")),
            Shown::Failed { reason } if lacks_kh1(reason) => ("KH1 FM not found: playing without KH1 files".to_string(), "It tries again when the mod or the game is updated.".to_string()),
            Shown::Failed { .. } => ("KH1 in Re:CoM: the build failed: playing without KH1 files".to_string(), "Details: kh1-recom-build.log in the game folder.".to_string()),
        };
        (ascii(a), ascii(b))
    }
}

/// Whether a failed build's reason is that KH1 Final Mix's files are missing.
pub fn lacks_kh1(reason: &str) -> bool {
    reason.contains("KH1 Final Mix")
}

/// A stamp path under Panacea's folder as the game names it (`raw/` dropped): Re:CoM's archive
/// path, or None for our own files at the root (`kh1-*.json`, `kh1-*.bin`: the DLL reads them,
/// the game never opens them) and anything else that is not an upper-case archive folder.
pub fn game_name(rel: &str) -> Option<&str> {
    let n = rel.strip_prefix("raw/").unwrap_or(rel);
    let (top, rest) = n.split_once('/')?;
    (!rest.is_empty() && !top.is_empty() && top.bytes().all(|b| b.is_ascii_uppercase())).then_some(n)
}

/// The empty files to make in Panacea's folder before it is listed: every name the build may
/// write (`expected`: the step's new names; `previous`: the last build's stamp) that is a game
/// file `Recom.hed` lacks and is not there yet. Panacea reads a file's content at each open
/// (`docs/openkh-github-install.md` § 8), so the build fills them in later.
pub fn placeholders(expected: &[String], previous: &[String], in_recom: impl Fn(&str) -> bool, exists: impl Fn(&str) -> bool) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for rel in expected.iter().chain(previous) {
        let Some(name) = game_name(rel) else { continue };
        if in_recom(name) || exists(rel) || out.iter().any(|o| o.eq_ignore_ascii_case(rel)) {
            continue;
        }
        out.push(rel.clone());
    }
    out.sort();
    out
}

/// The line sink: the build log (`win::assets`).
pub type Say = Arc<dyn Fn(&str) + Send + Sync>;

/// Runs a job: checks the inputs, empties the cache, runs the whole asset step into it and
/// writes its stamp (or, on failure, the failed marker with the reason). `say` gets every line.
/// `guard`: a failed marker written before the step runs and removed when it succeeds, for a run
/// inside the game, where a crash of the step would end the game (the next start then skips it).
pub fn run_job(job: &Job, say: &Say, guard: Option<&str>) -> Result<Stamp, String> {
    run_job_with(job, say, guard, Box::new(|_| {}))
}

/// [`run_job`], with the step's progress ([`Shown::at`]) to `on` (from this thread).
pub fn run_job_with(job: &Job, say: &Say, guard: Option<&str>, on: Box<dyn Fn(Shown)>) -> Result<Stamp, String> {
    step::set_progress(Some(Box::new(move |p: step::Progress| on(Shown::at(p.stage, p.part)))));
    let inputs = stamp::inputs(&job.game);
    let r = run_job_inner(job, &inputs, say, guard);
    step::set_progress(None);
    if let Err(e) = &r {
        let _ = std::fs::create_dir_all(&job.cache);
        let _ = std::fs::write(job.cache.join(FAILED_FILE), stamp::failed_text(stamp::STEP_HASH, &job.inputs, e));
    }
    r
}

fn run_job_inner(job: &Job, inputs: &[Input], say: &Say, guard: Option<&str>) -> Result<Stamp, String> {
    stamp::inputs_usable(inputs).map_err(|e| format!("{e} in {}", job.game.display()))?;
    if stamp::inputs_hash(inputs) != job.inputs {
        say("note: the game's files changed since the start; building from them as they are now");
    }
    // Only our own folder (its fixed name) is emptied: files of an older build go with it.
    if job.cache.is_dir() {
        std::fs::remove_dir_all(&job.cache).map_err(|e| format!("{}: {e}", job.cache.display()))?;
    }
    std::fs::create_dir_all(&job.cache).map_err(|e| format!("{}: {e}", job.cache.display()))?;
    if let Some(g) = guard {
        let _ = std::fs::write(job.cache.join(FAILED_FILE), stamp::failed_text(stamp::STEP_HASH, &job.inputs, g));
    }
    let mut opt = step::Options::new(job.game.clone(), step::Dest::Root(job.cache.clone()));
    opt.previews = false;
    let to = say.clone();
    step::set_sink(Some(Box::new(move |l: &str| to(l))));
    let r = step::run(&opt);
    step::set_sink(None);
    let report = r?;
    let st = Stamp::of_build(stamp::STEP_HASH, &stamp::inputs_hash(inputs), &job.cache, &report.written);
    let tmp = job.cache.join(format!("{STAMP_FILE}.tmp"));
    std::fs::write(&tmp, st.to_text()).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, job.cache.join(STAMP_FILE)).map_err(|e| format!("{}: {e}", job.cache.display()))?;
    let _ = std::fs::remove_file(job.cache.join(FAILED_FILE));
    Ok(st)
}

/// Copies a finished build's files into the job's live folder (none when it has no live folder),
/// with the copy's progress to `on` (the last stage, one part per file).
pub fn place_job(job: &Job, st: &Stamp, on: &dyn Fn(Shown)) -> Result<stamp::Placed, String> {
    if job.live.as_os_str().is_empty() {
        return Ok(stamp::Placed::default());
    }
    let last = stage_count() - 1;
    let mut shown = u32::MAX;
    stamp::place_with(&job.cache, &job.live, st, |k, n| {
        let s = Shown::at(last, Some((k, n)));
        if let Shown::Running { percent, .. } = &s {
            if *percent != shown {
                shown = *percent;
                on(s);
            }
        }
    })
    .map_err(|e| format!("could not copy the built files into {}: {e}", job.live.display()))
}

/// The text the player sees when a build failed.
pub fn failure_notice(reason: &str, game: &Path, log: &Path) -> String {
    let lacks_kh1 = reason.contains("KH1 Final Mix");
    let mut s = String::from("KH1 Combat in Re:CoM could not build its KH1 files from your games.\n\n");
    if lacks_kh1 {
        s.push_str(&format!("KINGDOM HEARTS FINAL MIX's files were not found in\n{}\\Image\\dt.\nThe mod needs KH1 Final Mix installed in the same KINGDOM HEARTS HD 1.5+2.5 ReMIX folder (Steam).\n\n", game.display()));
    } else {
        s.push_str(&format!("Reason: {reason}\n\n"));
    }
    s.push_str(&format!("The game starts now without the KH1 files (Re:CoM's own models, motions and sounds stay).\nIt tries again when the mod or the game is updated.\n\nDetails: {}", log.display()));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kh1-ab-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn mods_manager_paths() {
        assert_eq!(yaml_value("gameEdition: 2\ninstalledModsPath: 'D:\\Mods'\r\ncompiledModPath: \n", "installedModsPath").as_deref(), Some("D:\\Mods"));
        assert_eq!(yaml_value("installedModsPath: \"C:\\a b\"\n", "installedModsPath").as_deref(), Some("C:\\a b"));
        assert_eq!(yaml_value("  installedModsPath: x\n", "installedModsPath"), None, "top level only");
        assert_eq!(yaml_value("compiledModPath: ~\n", "compiledModPath"), None);
        let d = tmp("yml");
        assert_eq!(installed_mods_dirs(&d), vec![d.join("mods").join("Recom")]);
        std::fs::write(d.join("mods-manager.yml"), "installedModsPath: /x/mods\ncompiledModPath: /x/mod\n").unwrap();
        assert_eq!(installed_mods_dirs(&d), vec![PathBuf::from("/x/mods/Recom"), d.join("mods").join("Recom")]);
        std::fs::write(d.join("mods-manager.yml"), "installedModsPath: /x\ncompiledModPath: /x\n").unwrap();
        assert_eq!(installed_mods_dirs(&d)[0], PathBuf::from("/x/mods/Recom"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_mod_folder_is_found_by_its_dll_then_its_name() {
        let d = tmp("find");
        let mods = d.join("mods").join("Recom");
        let mk = |owner: &str, repo: &str, yml: &str, dll: Option<(&str, usize)>| {
            let p = mods.join(owner).join(repo);
            std::fs::create_dir_all(p.join("openkh")).unwrap();
            std::fs::write(p.join("mod.yml"), yml).unwrap();
            if let Some((at, n)) = dll {
                std::fs::write(p.join(at), vec![0u8; n]).unwrap();
            }
            p
        };
        let other = mk("someone", "other-mod", "assets:\n- name: dll/other.dll\n", Some(("openkh/kh1-recom.dll", 7)));
        assert_eq!(find_mod_folder(&[mods.clone()], Some(7)), None, "another mod's mod.yml");
        let _ = other;
        let named = mk("pm2crazy", "kh1-in-recom", "- name: dll/kh1-recom.dll\n", Some(("openkh/kh1-recom.dll", 5)));
        assert_eq!(find_mod_folder(&[mods.clone()], Some(9)), Some(named.clone()), "by name (case as typed)");
        let fork = mk("fork", "KH1-in-ReCom", "- name: dll/kh1-recom.dll\n  source:\n  - name: kh1-recom.dll\n", Some(("kh1-recom.dll", 9)));
        assert_eq!(find_mod_folder(&[mods.clone()], Some(9)), Some(fork), "the running DLL's own source wins");
        assert_eq!(find_mod_folder(&[d.join("nowhere")], Some(9)), None);
        assert_eq!(cache_dir(Some(&named), &d), named.join("generated"));
        assert_eq!(cache_dir(None, &d), d.join("kh1-recom-generated"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn decide_builds_once_then_places() {
        let d = tmp("decide");
        let cache = d.join("generated");
        let ins = vec![Input { name: "Image/dt/Recom.hed".into(), size: 1, time: 2 }];
        let ih = stamp::inputs_hash(&ins);
        assert_eq!(decide(false, &cache, "s", &ins), Decision::Off);
        assert!(matches!(decide(true, &cache, "s", &ins), Decision::Build(r) if r.contains("not been built")));
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(cache.join("a.bin"), b"abc").unwrap();
        let st = Stamp { step: "s".into(), inputs: ih.clone(), files: vec![("a.bin".into(), 3)] };
        std::fs::write(cache.join(STAMP_FILE), st.to_text()).unwrap();
        assert_eq!(decide(true, &cache, "s", &ins), Decision::Place(st.clone()));
        assert!(matches!(decide(true, &cache, "t", &ins), Decision::Build(r) if r.contains("mod update")));
        let ins2 = vec![Input { name: "Image/dt/Recom.hed".into(), size: 1, time: 3 }];
        assert!(matches!(decide(true, &cache, "s", &ins2), Decision::Build(r) if r.contains("game update")));
        std::fs::write(cache.join("a.bin"), b"ab").unwrap();
        assert!(matches!(decide(true, &cache, "s", &ins), Decision::Build(r) if r.contains("incomplete")));
        // A failed build of the same step and inputs is not tried again; another one is.
        std::fs::write(cache.join(FAILED_FILE), stamp::failed_text("t", &ih, "KH1 files: none")).unwrap();
        assert_eq!(decide(true, &cache, "t", &ins), Decision::Failed("KH1 files: none".into()));
        assert!(matches!(decide(true, &cache, "u", &ins), Decision::Build(_)));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_job_reads_back_and_fails_cleanly_without_the_games() {
        let d = tmp("job");
        let job = Job { game: d.join("game"), cache: d.join("generated"), log: d.join(BUILD_LOG), inputs: "00".into(), reason: "first\nlaunch".into(), live: d.join("live"), progress: d.join(PROGRESS_FILE) };
        let back = Job::parse(&job.to_text()).unwrap();
        assert_eq!(back.reason, "first launch");
        assert_eq!(Job { reason: "first launch".into(), ..job.clone() }, back);
        std::fs::create_dir_all(job.cache.join("old")).unwrap();
        let lines = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let l2 = lines.clone();
        let say: Say = Arc::new(move |l: &str| l2.lock().unwrap().push(l.to_string()));
        let e = run_job(&job, &say, None).unwrap_err();
        assert!(e.contains("KH1 Final Mix") && e.contains("Recom.hed"), "{e}");
        assert!(job.cache.join("old").is_dir(), "nothing is deleted when the inputs are missing");
        let t = std::fs::read_to_string(job.cache.join(FAILED_FILE)).unwrap();
        assert!(Stamp::parse(&t).is_some_and(|s| s.is_for(stamp::STEP_HASH, "00")));
        assert!(failure_notice(&stamp::failed_reason(&t), &job.game, &job.log).contains("KINGDOM HEARTS FINAL MIX's files were not found"));
        // The window's failure line for it is the owner's.
        assert_eq!(Shown::Failed { reason: stamp::failed_reason(&t) }.lines().0, "KH1 FM not found: playing without KH1 files");
        // A job file of 0.141.0 (no live / progress lines) still reads.
        let old = Job::parse("game=g\ncache=c\nlog=l\ninputs=00\nreason=r\n").unwrap();
        assert!(old.live.as_os_str().is_empty() && old.progress.as_os_str().is_empty());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn progress_reads_back_and_counts_stages() {
        let of = stage_count();
        assert_eq!(of, step::STAGES.len() + 1);
        assert_eq!(stage_label(of - 1), PLACE_STAGE);
        assert_eq!(stage_label(step::stage::MOTIONS), "motions");
        assert_eq!(percent(0, of, None), 0);
        assert_eq!(percent(5, 10, None), 50);
        assert_eq!(percent(5, 10, Some((1, 2))), 55);
        assert_eq!(percent(9, 10, Some((9, 9))), 99, "under 100 until done");
        assert_eq!(percent(3, 0, None), 0);
        for s in [Shown::at(step::stage::ENEMIES, Some((3, 21))), Shown::Done { files: 184, secs: 95 }, Shown::Failed { reason: "not found: KH1 Final Mix's Image/dt/kh1_*.hed".into() }, Shown::starting()] {
            assert_eq!(Shown::parse(&s.to_text()), Some(s.clone()), "{s:?}");
            let (a, b) = s.lines();
            assert!(a.is_ascii() && b.is_ascii() && !a.is_empty());
        }
        assert_eq!(Shown::parse("something\nrunning 1 2 3 x"), None);
        assert_eq!(Shown::parse("kh1-recom-build progress 1\nfailed two\nlines"), Some(Shown::Failed { reason: "two".into() }));
        let (a, b) = Shown::at(step::stage::KEYBLADES, None).lines();
        assert_eq!(a, format!("KH1 in Re:CoM: building files from your games... {} %", percent(2, of, None)));
        assert_eq!(b, format!("Step 3 of {of}: Keyblades. The title waits until it is done."));
        assert_eq!(Shown::Failed { reason: "disk full".into() }.lines().0, "KH1 in Re:CoM: the build failed: playing without KH1 files");
        assert!(Shown::Done { files: 3, secs: 1 }.lines().0.contains("done"));
    }

    #[test]
    fn place_job_copies_with_progress() {
        let d = tmp("placejob");
        let cache = d.join("generated");
        std::fs::create_dir_all(cache.join("PLAYER/0021")).unwrap();
        std::fs::write(cache.join("PLAYER/0021/PL0021.MDL"), b"model").unwrap();
        std::fs::write(cache.join("kh1-stats.json"), b"{}").unwrap();
        let st = Stamp::of_build("s", "i", &cache, &[cache.join("PLAYER/0021/PL0021.MDL"), cache.join("kh1-stats.json")]);
        let mut job = Job { game: d.join("game"), cache: cache.clone(), log: d.join(BUILD_LOG), inputs: "i".into(), reason: String::new(), live: PathBuf::new(), progress: PathBuf::new() };
        assert_eq!(place_job(&job, &st, &|_| {}).unwrap(), stamp::Placed::default(), "no live folder: nothing");
        job.live = d.join("live");
        // An empty file made before Panacea's listing is replaced by the built one.
        std::fs::create_dir_all(job.live.join("PLAYER/0021")).unwrap();
        std::fs::write(job.live.join("PLAYER/0021/PL0021.MDL"), b"").unwrap();
        let seen = std::sync::Mutex::new(Vec::new());
        let p = place_job(&job, &st, &|s| seen.lock().unwrap().push(s)).unwrap();
        assert_eq!((p.copied, p.kept), (2, 0));
        assert_eq!(std::fs::read(job.live.join("PLAYER/0021/PL0021.MDL")).unwrap(), b"model");
        let seen = seen.into_inner().unwrap();
        assert!(matches!(&seen[0], Shown::Running { stage, label, .. } if *stage == stage_count() - 1 && label == PLACE_STAGE));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn placeholders_are_the_new_game_names_only() {
        assert_eq!(game_name("raw/PLAYER/0021/PL0021.RTM"), Some("PLAYER/0021/PL0021.RTM"));
        assert_eq!(game_name("PLAYER/0021/PL0021.MDL"), Some("PLAYER/0021/PL0021.MDL"));
        assert_eq!(game_name("kh1-stats.json"), None, "the DLL's own files");
        assert_eq!(game_name("kh1-menu-sounds-preview/a.wav"), None);
        assert_eq!(game_name("raw/kh1-x.bin"), None);
        let expected = kh1_recom_assets::keyblade::new_resource_names();
        let previous: Vec<String> = ["PLAYER/0001/PL0001.ANN", "raw/FORM/0002/FO0002.SPR", "kh1-stats.json", "PLAYER/0021/PL0021.MDL", "PLAYER/0099/PL0099.MDL"].iter().map(|s| s.to_string()).collect();
        // Re:CoM's names: what the step replaces (here: everything not in a group past 20).
        let in_recom = |n: &str| !(n.starts_with("PLAYER/") && n[7..11].parse::<u32>().is_ok_and(|g| g > 20));
        let have = |rel: &str| rel == "raw/PLAYER/0022/PL0022.RTM";
        let p = placeholders(&expected, &previous, in_recom, have);
        assert_eq!(p.len(), expected.len() - 1 + 1, "every new name but the one there, and the last build's own new one");
        assert!(p.contains(&"PLAYER/0099/PL0099.MDL".to_string()) && p.contains(&"raw/PLAYER/0036/PL0036.EFF".to_string()));
        assert!(!p.iter().any(|n| n.contains("FO0002") || n.contains("PL0001") || n.contains("json") || n == "raw/PLAYER/0022/PL0022.RTM"));
        assert_eq!(p.iter().filter(|n| n.as_str() == "PLAYER/0021/PL0021.MDL").count(), 1, "once");
    }

    #[test]
    fn the_repo_root_is_an_openkh_mod() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
        // OpenKH's GitHub install checks raw.githubusercontent.com/<repo>/<branch>/mod.yml (the root).
        let yml = std::fs::read_to_string(root.join("mod.yml")).unwrap();
        assert!(yml.contains("\ntitle: ") || yml.starts_with("title: "));
        assert!(yml.contains("\ngame: Recom\n"));
        // The DLL goes to <mod_path>/recom/dll/ (Panacea's LoadDLLs), from the build bot's file.
        assert!(yml.contains("- name: dll/kh1-recom.dll\n  method: copy\n  source:\n  - name: openkh/kh1-recom.dll\n"), "{yml}");
        assert!(root.join(DLL_SOURCES[0]).is_file());
        // No game-made file is listed (the DLL places them), and none is tracked.
        assert_eq!(yml.matches("- name:").count(), 2, "one asset, one source");
        let ignore = std::fs::read_to_string(root.join(".gitignore")).unwrap();
        for p in ["/generated/", "/PLAYER/", "/ENEMY/", "/raw/", "/kh1-*.json", "/kh1-*.bin", "/kh1-menu-sounds-preview/", "/kh1-recom.dll"] {
            assert!(ignore.lines().any(|l| l == p), "{p} not in .gitignore");
        }
        assert_eq!(CACHE_DIR, "generated");
    }
}
