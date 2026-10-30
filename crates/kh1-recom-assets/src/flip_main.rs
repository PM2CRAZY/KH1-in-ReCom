//! `kh1-flip-test`: step 1 of the "Re:CoM's content in KH1" feasibility test
//! (`docs/kh1-model-writer.md`). Reads the player's own KH1 archives; writes only into
//! Panacea's mod folder (`install`) or nothing (`roundtrip`).

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use kh1_formats::egs::Install;
use kh1_recom_assets::flip::{self, Mode, SHADOW};
use kh1_recom_assets::install;

const USAGE: &str = "usage: kh1-flip-test roundtrip [--game <folder>]
       kh1-flip-test install (rebuild | big | remove) [--game <folder>] [--mod-dir <mod_path>]
                     [--model <xa_*.mdls>] [--scale <k>]

  roundtrip  parse and rewrite every KH1 model (.mdls) and HD mesh (.cvbl); report any byte
             that differs. Writes nothing.
  install    write <mod_path>/kh1/raw/<model> (default xa_ex_2020.mdls, the Shadow):
               rebuild  the model and its HD meshes written by our writers, content unchanged
               big      the same, every vertex position times --scale (default 1.5)
               remove   delete that file again
  --game     the KH 1.5+2.5 folder (Image/dt/kh1_*.hed and the KH1 exe); default: .
  --mod-dir  Panacea's mod_path; default: mod_path= from <game>/panacea_settings.txt";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("kh1-flip-test: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let cmd = args.next().ok_or_else(|| USAGE.to_string())?;
    let mut sub = None;
    if cmd == "install" {
        sub = Some(args.next().ok_or_else(|| USAGE.to_string())?);
    }
    let mut game = PathBuf::from(".");
    let mut mod_dir: Option<PathBuf> = None;
    let mut model = SHADOW.to_string();
    let mut scale = 1.5f32;
    while let Some(a) = args.next() {
        let mut val = || args.next().ok_or_else(|| format!("{a} needs a value\n{USAGE}"));
        match a.as_str() {
            "--game" => game = PathBuf::from(val()?),
            "--mod-dir" => mod_dir = Some(PathBuf::from(val()?)),
            "--model" => model = val()?,
            "--scale" => scale = val()?.parse().map_err(|_| "bad --scale".to_string())?,
            _ => return Err(format!("unknown argument '{a}'\n{USAGE}")),
        }
    }
    let t0 = Instant::now();
    match (cmd.as_str(), sub.as_deref()) {
        ("roundtrip", None) => {
            let inst = Install::with_keys(&game, flip::kh1_keys(&game).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            let names = flip::mdls_names(&inst);
            println!("{} KH1 models found", names.len());
            let r = flip::roundtrip(&inst, &names);
            println!(
                "byte-exact: {} of {} .mdls, {} .cvbl HD meshes; {} other HD assets carried through the rebuilt entries ({:.1} s)",
                r.mdls,
                names.len(),
                r.cvbl,
                r.other_assets,
                t0.elapsed().as_secs_f32()
            );
            for (f, e) in &r.failures {
                println!("FAILED {f}: {e}");
            }
            if r.failures.is_empty() {
                Ok(())
            } else {
                Err(format!("{} files did not round-trip", r.failures.len()))
            }
        }
        ("install", Some(s)) => {
            let modp = match mod_dir {
                Some(m) => m,
                None => install::mod_path(&game).map_err(|e| e.to_string())?,
            };
            let out = flip::raw_path(&modp, &model);
            if s == "remove" {
                match std::fs::remove_file(&out) {
                    Ok(()) => println!("removed {}", out.display()),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => println!("nothing to remove at {}", out.display()),
                    Err(e) => return Err(format!("{}: {e}", out.display())),
                }
                return Ok(());
            }
            let mode = match s {
                "rebuild" => Mode::Rebuild,
                "big" if scale.is_finite() && scale > 0.0 => Mode::Scale(scale),
                "big" => return Err("--scale must be positive".into()),
                _ => return Err(USAGE.to_string()),
            };
            let inst = Install::with_keys(&game, flip::kh1_keys(&game).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            let t = flip::build_test_entry(&inst, &model, mode).map_err(|e| e.to_string())?;
            if let Some(dir) = out.parent() {
                std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            }
            std::fs::write(&out, &t.entry).map_err(|e| format!("{}: {e}", out.display()))?;
            println!(
                "wrote {} ({} bytes): {model} {}, {} HD meshes written by our writer, {} PS2 / {} HD positions scaled",
                out.display(),
                t.entry.len(),
                match mode {
                    Mode::Rebuild => "rebuilt".to_string(),
                    Mode::Scale(k) => format!("scaled x{k}"),
                },
                t.cvbl_written,
                t.ps2_positions,
                t.hd_positions
            );
            Ok(())
        }
        _ => Err(USAGE.to_string()),
    }
}
