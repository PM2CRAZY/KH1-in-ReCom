//! The player's files: Re:CoM's and KH1's archives in the KH 1.5+2.5 folder, and Panacea's mod
//! folder.
//!
//! - Re:CoM: `Image/dt/Recom.hed` / `.pkg`, same format as KH1's (OpenKH `Egs`); files are found
//!   by MD5 of the plain path (`PLAYER/0001/PL0001.ANN`). The key table comes from the Re:CoM
//!   exe (any 1.5+2.5 exe carries it). Checked (files): PL0001.ANN/MDL decode to the sizes the
//!   index gives.
//! - KH1: `Image/dt/kh1_*.hed` / `.pkg` (`kh1_formats::egs::Install`), `xa_ex_0010.mset/.mdls`.
//! - Panacea (`OpenKh.Research.Panacea`, `OpenKH.cpp`): reads `panacea_settings.txt` from the
//!   game folder (`mod_path=` line; default `./mod`), appends `/recom` for Re:CoM, and serves
//!   `<mod_path>/recom/<archive path>` in place of the archive file (`Panacea.cpp`
//!   `TransformFilePath`). Checked (source).

use std::path::{Path, PathBuf};

use kh1_formats::egs::{Archive, Install, KeyTable, KH1_EXE};

use crate::{bad, Result};

/// The Re:CoM executable's file name (Steam).
pub const RECOM_EXE: &str = "KINGDOM HEARTS Re_Chain of Memories.exe";
/// Sora's motion pack path inside `Recom.pkg`.
pub const SORA_ANN: &str = "PLAYER/0001/PL0001.ANN";
/// Sora's model path inside `Recom.pkg`.
pub const SORA_MDL: &str = "PLAYER/0001/PL0001.MDL";
/// The Keyblade's (`CPlayerWeapon`, resource group 2) motion pack.
pub const KEYBLADE_ANN: &str = "PLAYER/0002/PL0002.ANN";
/// The Keyblade's model.
pub const KEYBLADE_MDL: &str = "PLAYER/0002/PL0002.MDL";
/// The manifest's file name (written at `<mod>/recom/`, where the runtime looks first).
pub const MANIFEST: &str = "kh1-motions.json";
/// Panacea's settings file in the game folder.
pub const PANACEA_SETTINGS: &str = "panacea_settings.txt";

/// The key table, from the Re:CoM exe, else the KH1 exe.
pub fn keys(game: &Path) -> Result<KeyTable> {
    let mut last = None;
    for exe in [RECOM_EXE, KH1_EXE] {
        match KeyTable::from_executable(game.join(exe)) {
            Ok(k) => return Ok(k),
            Err(e) => last = Some(e),
        }
    }
    Err(last.map(Into::into).unwrap_or_else(|| crate::Error::Format("no executable".into())))
}

/// `Image/dt/Recom.hed` of a game folder.
pub fn recom_archive(game: &Path) -> Result<Archive> {
    let hed = game.join("Image").join("dt").join("Recom.hed");
    if !hed.is_file() {
        return bad(format!("{} not found (not a KH 1.5+2.5 PC folder?)", hed.display()));
    }
    Ok(Archive::open(hed)?)
}

/// Reads one file from `Recom.pkg`.
pub fn read_recom(arc: &Archive, keys: &KeyTable, name: &str) -> Result<Vec<u8>> {
    Ok(arc.read(keys, name)?)
}

/// KH1's installed archives (any KH1 file by name).
pub fn kh1(game: &Path, keys: KeyTable) -> Result<Install> {
    Ok(Install::with_keys(game, keys)?)
}

/// Sora's per-motion sound events in `Recom.pkg`.
pub const SORA_SES: &str = "PLAYER/0001/PL0001.SES";

/// The KH1 stats file the runtime reads, next to [`MANIFEST`].
pub const STATS: &str = "kh1-stats.json";

/// Reads KH1's `xa_ex_0010.mset` and `.mdls` from the game folder's `kh1_*` archives.
pub fn read_kh1(game: &Path, keys: KeyTable) -> Result<(Vec<u8>, Vec<u8>)> {
    let inst = Install::with_keys(game, keys)?;
    Ok((inst.read("xa_ex_0010.mset")?, inst.read("xa_ex_0010.mdls")?))
}

/// Panacea's mod folder for a game folder: `mod_path=` of `panacea_settings.txt` (relative
/// paths are relative to the game folder, Panacea's working directory).
pub fn mod_path(game: &Path) -> Result<PathBuf> {
    let p = game.join(PANACEA_SETTINGS);
    let text = match std::fs::read_to_string(&p) {
        Ok(t) => t,
        Err(e) => return bad(format!("{}: {e} (is Panacea set up? or pass --mod-dir)", p.display())),
    };
    parse_mod_path(&text).map(|m| if m.is_absolute() { m } else { game.join(m) }).ok_or_else(|| crate::Error::Format(format!("{}: no mod_path= line", p.display())))
}

/// The `mod_path=` value of a settings text (Panacea's parser: first `=` splits; value up to
/// the line end).
pub fn parse_mod_path(text: &str) -> Option<PathBuf> {
    text.lines().find_map(|l| {
        let (k, v) = l.split_once('=')?;
        let v = v.trim_end_matches(['\r', '\n']);
        (k == "mod_path" && !v.is_empty()).then(|| PathBuf::from(v))
    })
}

/// `<mod>/recom` (an existing folder of that name in any case is reused).
pub fn recom_dir(modp: &Path) -> PathBuf {
    if let Ok(rd) = std::fs::read_dir(modp) {
        for e in rd.flatten() {
            if e.file_name().to_string_lossy().eq_ignore_ascii_case("recom") && e.path().is_dir() {
                return e.path();
            }
        }
    }
    modp.join("recom")
}

/// Where a Re:CoM archive path goes under the mod folder.
pub fn mod_file(modp: &Path, archive_path: &str) -> PathBuf {
    let mut p = recom_dir(modp);
    for part in archive_path.split('/') {
        p.push(part);
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_parse() {
        assert_eq!(parse_mod_path("show_console=true\r\nmod_path=C:\\OpenKH\\mod\r\n"), Some(PathBuf::from("C:\\OpenKH\\mod")));
        assert_eq!(parse_mod_path("mod_path=\n"), None);
        assert_eq!(parse_mod_path("dev_path=x\n"), None);
        let m = mod_file(Path::new("/nonexistent/mod"), SORA_ANN);
        assert!(m.ends_with("recom/PLAYER/0001/PL0001.ANN"));
    }
}
