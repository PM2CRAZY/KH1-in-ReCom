//! `kh1-recom-build`: the asset build step. Reads the player's own KH 1.5+2.5 folder (KH1's
//! and Re:CoM's archives) and writes `<mod>/recom/kh1-motions.json` (the KH1 entry → Re:CoM
//! slot map) and, when some KH1 motion is not in Re:CoM, `<mod>/recom/PLAYER/0001/PL0001.ANN`
//! and `<mod>/recom/PLAYER/0002/PL0002.ANN` (both extended) into Panacea's Re:CoM mod folder.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use kh1_recom_assets::ann::Ann;
use kh1_recom_assets::build::{build, BuildOptions, Inputs, RECOM_SORA_SLOTS};
use kh1_recom_assets::install;

const USAGE: &str = "usage: kh1-recom-build [--game <KH 1.5+2.5 folder>] [--mod-dir <Panacea mod_path>] [--out <file.ANN>]
                       [--loop E,E..] [--once E,E..] [--dry-run]

  --game     the game folder (holds Image/dt/Recom.hed and kh1_*.hed and the exes); default: .
  --mod-dir  Panacea's mod_path; default: mod_path= from <game>/panacea_settings.txt
  --out      write Sora's pack here (PL0002.ANN and the manifest next to it) instead of
             <mod>/recom/PLAYER/000{1,2}/PL000{1,2}.ANN and <mod>/recom/kh1-motions.json
  --loop / --once   force new KH1 clips to loop / play once (default: KH1 pose at N = pose at 0)
  --dry-run  convert and report, write nothing";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("kh1-recom-build: {e}");
            ExitCode::FAILURE
        }
    }
}

fn entries(s: &str) -> Result<Vec<usize>, String> {
    s.split(',').filter(|x| !x.is_empty()).map(|x| x.trim().parse::<usize>().map_err(|_| format!("bad entry '{x}'"))).collect()
}

fn run() -> Result<(), String> {
    let mut game = PathBuf::from(".");
    let mut mod_dir: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut dry = false;
    let mut opt = BuildOptions::default();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut val = || args.next().ok_or_else(|| format!("{a} needs a value\n{USAGE}"));
        match a.as_str() {
            "--game" => game = PathBuf::from(val()?),
            "--mod-dir" => mod_dir = Some(PathBuf::from(val()?)),
            "--out" => out = Some(PathBuf::from(val()?)),
            "--loop" => opt.loop_overrides.extend(entries(&val()?)?.into_iter().map(|e| (e, true))),
            "--once" => opt.loop_overrides.extend(entries(&val()?)?.into_iter().map(|e| (e, false))),
            "--dry-run" => dry = true,
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(());
            }
            _ => return Err(format!("unknown argument '{a}'\n{USAGE}")),
        }
    }
    let t0 = Instant::now();
    let keys = install::keys(&game).map_err(|e| format!("key table: {e}"))?;
    let arc = install::recom_archive(&game).map_err(|e| e.to_string())?;
    let ann = install::read_recom(&arc, &keys, install::SORA_ANN).map_err(|e| format!("{}: {e}", install::SORA_ANN))?;
    let mdl = install::read_recom(&arc, &keys, install::SORA_MDL).map_err(|e| format!("{}: {e}", install::SORA_MDL))?;
    let (mset, mdls) = install::read_kh1(&game, keys.clone()).map_err(|e| format!("KH1 files: {e}"))?;
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
    let t_read = t0.elapsed();
    let mut sources = vec![
        ("PL0001.ANN sha256", kh1_formats::egs::sha256_hex(&ann)),
        ("PL0001.MDL sha256", kh1_formats::egs::sha256_hex(&mdl)),
        ("xa_ex_0010.mset sha256", kh1_formats::egs::sha256_hex(&mset)),
        ("xa_ex_0010.mdls sha256", kh1_formats::egs::sha256_hex(&mdls)),
    ];
    if let Some((a, m)) = &weapon {
        sources.push(("PL0002.ANN sha256", kh1_formats::egs::sha256_hex(a)));
        sources.push(("PL0002.MDL sha256", kh1_formats::egs::sha256_hex(m)));
    }
    let t1 = Instant::now();
    let built = build(&Inputs { mset, mdls, ann, mdl, weapon }, &opt).map_err(|e| e.to_string())?;
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
    for e in built.entries.iter().filter(|e| !e.ambiguous.is_empty()) {
        println!("note: KH1 entry {} also matches Re:CoM slots {:?} (chose {})", e.kh1_entry, e.ambiguous, e.slot);
    }
    if let Some(w) = &built.weapon {
        println!("Keyblade: {} new clips; pack {} bytes ({} slots)", n_new, w.bytes.len(), w.ann.slots.len());
    }
    if dry {
        return Ok(());
    }
    // The runtime (crates/kh1-recom, win/motion.rs) looks for the manifest at the recom root.
    let (ann_path, wep_path, man) = match out {
        Some(p) => {
            let m = p.with_file_name(install::MANIFEST);
            let w = p.with_file_name("PL0002.ANN");
            (p, w, m)
        }
        None => {
            let m = match mod_dir {
                Some(m) => m,
                None => install::mod_path(&game).map_err(|e| e.to_string())?,
            };
            (install::mod_file(&m, install::SORA_ANN), install::mod_file(&m, install::KEYBLADE_ANN), install::recom_dir(&m).join(install::MANIFEST))
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
    } else {
        // Nothing new: Panacea serves Re:CoM's own files. Remove packs an earlier run left.
        for p in [&ann_path, &wep_path] {
            remove_stale(p)?;
        }
    }
    if let Some(dir) = man.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let json = built.manifest_json(&sources.iter().map(|(k, v)| (*k, v.clone())).collect::<Vec<_>>());
    std::fs::write(&man, &json).map_err(|e| format!("{}: {e}", man.display()))?;
    println!("wrote {} ({} bytes)", man.display(), json.len());
    Ok(())
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("wrote {} ({} bytes)", path.display(), bytes.len());
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
