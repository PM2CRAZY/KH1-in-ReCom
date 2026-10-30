//! Finding the KINGDOM HEARTS HD 1.5+2.5 ReMIX folder (KH1 FM's PC install).
//!
//! A game folder is one that holds `Image/dt/kh1_first.hed` and `KINGDOM HEARTS FINAL MIX.exe`
//! (the exe carries the archive key table). Search order: the `--game-dir` argument, the
//! `KH1_GAME_DIR` environment variable, this program's folder and its parents, then common
//! Steam / Epic install paths (Steam library folders listed in `libraryfolders.vdf` included).

use std::fmt;
use std::path::{Path, PathBuf};

use kh1_formats::egs::{Install, KH1_ARCHIVES, KH1_EXE};

/// The command-line flag naming the game folder.
pub const GAME_DIR_ARG: &str = "--game-dir";
/// The environment variable naming the game folder.
pub const GAME_DIR_ENV: &str = "KH1_GAME_DIR";
/// The owner's install (tried first among the common paths).
pub const OWNER_STEAM_DIR: &str = r"D:\Steam\steamapps\common\KINGDOM HEARTS -HD 1.5+2.5 ReMIX-";

/// The Steam folder name of the collection.
const STEAM_NAME: &str = "KINGDOM HEARTS -HD 1.5+2.5 ReMIX-";
/// The Epic folder name of the collection.
const EPIC_NAME: &str = "KH_1.5_2.5";

/// A KH 1.5+2.5 folder with KH1 FM's archives and executable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameDir {
    /// The folder.
    pub root: PathBuf,
}

/// No game folder was found; lists every place tried and why it was not one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotFound {
    /// `(path, where it came from, why it is not a game folder)`.
    pub tried: Vec<(PathBuf, &'static str, String)>,
}

impl fmt::Display for NotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "KINGDOM HEARTS FINAL MIX (PC) was not found. A game folder holds Image/dt/kh1_first.hed and {KH1_EXE}.")?;
        if !self.tried.is_empty() {
            writeln!(f, "Tried:")?;
            for (p, from, why) in self.tried.iter().filter(|t| t.1 != "common path" || t.2 != MISSING) {
                writeln!(f, "  {} ({from}): {why}", p.display())?;
            }
            let quiet = self.tried.iter().filter(|t| t.1 == "common path" && t.2 == MISSING).count();
            if quiet > 0 {
                writeln!(f, "  ... and {quiet} common Steam / Epic paths that do not exist")?;
            }
        }
        writeln!(f, "Pass the folder with {GAME_DIR_ARG} (or set {GAME_DIR_ENV}), for example:")?;
        write!(f, "  kh1-playground {GAME_DIR_ARG} \"{OWNER_STEAM_DIR}\"")
    }
}

impl std::error::Error for NotFound {}

const MISSING: &str = "does not exist";

/// Cleans a pasted path: surrounding spaces and quotes; text glued on after `.exe`.
fn repair(p: &Path) -> PathBuf {
    let raw = p.to_string_lossy();
    let mut s = raw.trim().trim_matches(|c| c == '"' || c == '\'').trim().to_string();
    if !Path::new(&s).exists() {
        if let Some(i) = s.to_ascii_lowercase().find(".exe") {
            s.truncate(i + 4);
        }
    }
    PathBuf::from(s)
}

impl GameDir {
    /// Checks one folder (or the KH1 exe inside it, or its `Image/dt` folder). `Err` says why it
    /// is not a game folder.
    pub fn at(path: impl AsRef<Path>) -> Result<GameDir, String> {
        let p = repair(path.as_ref());
        if !p.exists() {
            return Err(MISSING.to_string());
        }
        let mut root = if p.is_file() { p.parent().unwrap_or(Path::new(".")).to_path_buf() } else { p.clone() };
        // Accept `.../Image/dt` and `.../Image` too.
        for _ in 0..2 {
            if !root.join("Image").join("dt").join(format!("{}.hed", KH1_ARCHIVES[0])).is_file() {
                if let Some(up) = root.parent().filter(|_| root.ends_with("dt") || root.ends_with("Image")) {
                    root = up.to_path_buf();
                }
            }
        }
        let hed = root.join("Image").join("dt").join(format!("{}.hed", KH1_ARCHIVES[0]));
        if !hed.is_file() {
            return Err(format!("no Image/dt/{}.hed", KH1_ARCHIVES[0]));
        }
        if !root.join(KH1_EXE).is_file() {
            return Err(format!("no {KH1_EXE}"));
        }
        Ok(GameDir { root })
    }

    /// Finds the game folder from this process's arguments ([`GAME_DIR_ARG`]), then
    /// [`GAME_DIR_ENV`], the program's folder and parents, and the common install paths.
    pub fn find() -> Result<GameDir, NotFound> {
        let args: Vec<String> = std::env::args().collect();
        Self::find_from(arg_value(&args, GAME_DIR_ARG).map(PathBuf::from))
    }

    /// [`GameDir::find`] with the argument given by the caller.
    pub fn find_from(arg: Option<PathBuf>) -> Result<GameDir, NotFound> {
        let mut tried = Vec::new();
        for (p, from) in Self::candidates(arg) {
            match GameDir::at(&p) {
                Ok(g) => return Ok(g),
                Err(why) => tried.push((p, from, why)),
            }
        }
        Err(NotFound { tried })
    }

    /// Every folder [`GameDir::find_from`] tries, in order, with where it came from.
    pub fn candidates(arg: Option<PathBuf>) -> Vec<(PathBuf, &'static str)> {
        let mut out: Vec<(PathBuf, &'static str)> = Vec::new();
        if let Some(a) = arg {
            out.push((a, "--game-dir"));
        }
        if let Some(e) = std::env::var_os(GAME_DIR_ENV).filter(|e| !e.is_empty()) {
            out.push((PathBuf::from(e), GAME_DIR_ENV));
        }
        if let Ok(exe) = std::env::current_exe() {
            let mut d = exe.parent().map(Path::to_path_buf);
            while let Some(dir) = d {
                out.push((dir.clone(), "program folder"));
                d = dir.parent().map(Path::to_path_buf);
            }
        }
        for p in common_paths() {
            if !out.iter().any(|(q, _)| q == &p) {
                out.push((p, "common path"));
            }
        }
        out
    }

    /// Opens the five KH1 archives with the exe's key table.
    pub fn open(&self) -> kh1_formats::Result<Install> {
        Install::open(&self.root)
    }

    /// The KH1 executable.
    pub fn exe(&self) -> PathBuf {
        self.root.join(KH1_EXE)
    }
}

/// The value after `name` in `args` (`--game-dir X`), or of `name=X`.
pub(crate) fn arg_value(args: &[String], name: &str) -> Option<String> {
    let eq = format!("{name}=");
    args.iter().enumerate().find_map(|(i, a)| if a == name { args.get(i + 1).cloned() } else { a.strip_prefix(&eq).map(str::to_string) })
}

/// Steam library roots listed in a `libraryfolders.vdf` (`"path" "D:\\SteamLibrary"` lines).
fn vdf_paths(text: &str) -> Vec<PathBuf> {
    text.lines()
        .filter_map(|l| {
            let l = l.trim();
            let rest = l.strip_prefix("\"path\"")?.trim();
            let v = rest.trim_matches('"');
            Some(PathBuf::from(v.replace("\\\\", "\\")))
        })
        .collect()
}

/// Common Steam and Epic folders (Windows drives C–H, Linux / Proton Steam roots).
fn common_paths() -> Vec<PathBuf> {
    let mut out = vec![PathBuf::from(OWNER_STEAM_DIR)];
    let mut steam_roots: Vec<PathBuf> = Vec::new();
    if cfg!(windows) {
        for d in ['C', 'D', 'E', 'F', 'G', 'H'] {
            for s in [r"Program Files (x86)\Steam", r"Program Files\Steam", "Steam", "SteamLibrary", r"Games\Steam", r"Games\SteamLibrary"] {
                steam_roots.push(PathBuf::from(format!(r"{d}:\{s}")));
            }
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        let h = PathBuf::from(home);
        steam_roots.push(h.join(".steam/steam"));
        steam_roots.push(h.join(".local/share/Steam"));
        steam_roots.push(h.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"));
    }
    // Libraries named in each Steam root's libraryfolders.vdf.
    let mut libs = Vec::new();
    for r in &steam_roots {
        if let Ok(t) = std::fs::read_to_string(r.join("steamapps").join("libraryfolders.vdf")) {
            libs.extend(vdf_paths(&t));
        }
    }
    steam_roots.extend(libs);
    for r in steam_roots {
        out.push(r.join("steamapps").join("common").join(STEAM_NAME));
    }
    if cfg!(windows) {
        for d in ['C', 'D', 'E', 'F', 'G', 'H'] {
            for s in [r"Program Files\Epic Games", "Epic Games", "epic", "Games"] {
                out.push(PathBuf::from(format!(r"{d}:\{s}\{EPIC_NAME}")));
            }
        }
    }
    let mut seen = Vec::new();
    out.retain(|p| {
        let keep = !seen.contains(p);
        seen.push(p.clone());
        keep
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kh1-data-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_game_folder_needs_the_hed_and_the_exe() {
        let root = temp("gamedir");
        assert!(GameDir::at(root.join("nope")).unwrap_err().contains("exist"));
        assert!(GameDir::at(&root).unwrap_err().contains("kh1_first.hed"));
        let dt = root.join("Image").join("dt");
        std::fs::create_dir_all(&dt).unwrap();
        std::fs::write(dt.join("kh1_first.hed"), []).unwrap();
        assert!(GameDir::at(&root).unwrap_err().contains(KH1_EXE));
        std::fs::write(root.join(KH1_EXE), []).unwrap();
        let g = GameDir::at(&root).unwrap();
        assert_eq!(g.root, root);
        // The exe, Image/dt and a quoted path all lead to the same folder.
        assert_eq!(GameDir::at(root.join(KH1_EXE)).unwrap().root, root);
        assert_eq!(GameDir::at(&dt).unwrap().root, root);
        let quoted = format!("  \"{}\" ", root.display());
        assert_eq!(GameDir::at(quoted).unwrap().root, root);
        // The argument wins over everything else.
        assert_eq!(GameDir::find_from(Some(root.clone())).unwrap().root, root);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn candidates_order_and_messages() {
        let c = GameDir::candidates(Some(PathBuf::from("/x/arg")));
        assert_eq!(c[0], (PathBuf::from("/x/arg"), "--game-dir"));
        assert!(c.iter().any(|(p, f)| *f == "common path" && p == Path::new(OWNER_STEAM_DIR)));
        let nf = NotFound { tried: vec![(PathBuf::from("/x/arg"), "--game-dir", MISSING.into())] };
        let s = nf.to_string();
        assert!(s.contains("--game-dir") && s.contains("/x/arg"));
        let args: Vec<String> = ["prog", "--trace", "a.csv", "--game-dir=/g"].iter().map(|s| s.to_string()).collect();
        assert_eq!(arg_value(&args, "--game-dir").as_deref(), Some("/g"));
        assert_eq!(arg_value(&args, "--trace").as_deref(), Some("a.csv"));
        assert_eq!(arg_value(&args, "--nope"), None);
    }

    #[test]
    fn steam_library_list() {
        let v = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"D:\\\\SteamLibrary\"\n\t}\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"/mnt/games\"\n\t}\n}";
        assert_eq!(vdf_paths(v), vec![PathBuf::from(r"D:\SteamLibrary"), PathBuf::from("/mnt/games")]);
    }
}
