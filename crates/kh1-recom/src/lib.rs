//! KH1 combat in Kingdom Hearts Re:Chain of Memories (PC, Steam).
//!
//! OpenKH's Panacea loads this DLL into `KINGDOM HEARTS Re_Chain of Memories.exe` (from
//! `mod_path\Recom\dll`) and calls its `OnInit(mod folder)`. At start it checks that it runs on
//! the Steam build it targets and the first bytes of every function it hooks or calls, reads
//! `kh1-recom.ini`, and writes `kh1-recom.log` next to the game. Then, only when every check
//! passed:
//!
//! - adds two rows to Re:CoM's Config menu ("Combat", "KH1 motions test"), drawn and driven by
//!   Re:CoM's own menu code (`menu`, `docs/recom-config-menu.md`);
//! - with "KH1 motions test" on, re-points Sora's action-table rows to the KH1 clips the asset
//!   build step appended to `PL0001.ANN` (`motions`, `docs/recom-motion-switch.md`).

pub mod hookplan;
pub mod json;
pub mod menu;
pub mod motions;
pub mod settings;
pub mod sites;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Builds the startup report from what was read out of the running game.
/// `read_site` returns the bytes at an RVA of the running exe.
pub fn startup_report(
    exe_path: &str,
    pe: Option<sites::PeInfo>,
    read_site: &dyn Fn(u32, usize) -> Vec<u8>,
    ini: Result<String, String>,
) -> (String, bool) {
    use std::fmt::Write;
    let mut r = String::new();
    let _ = writeln!(r, "kh1-recom {VERSION}");
    let _ = writeln!(r, "exe: {exe_path}");
    let name_ok = exe_path.rsplit(['\\', '/']).next().is_some_and(|n| n.eq_ignore_ascii_case(sites::EXE_NAME));
    let build_ok = match &pe {
        Some(i) => {
            let ok = sites::is_steam_build(i);
            let _ = writeln!(
                r,
                "build: time stamp {:#010x}, size of image {:#x} -> {}",
                i.timestamp,
                i.size_of_image,
                if ok { "Steam Re:CoM (supported)" } else { "NOT the Steam build this mod supports" }
            );
            ok
        }
        None => {
            let _ = writeln!(r, "build: PE headers unreadable");
            false
        }
    };
    if !name_ok {
        let _ = writeln!(r, "exe name is not `{}`: not Re:CoM?", sites::EXE_NAME);
    }
    let mut all_ok = name_ok && build_ok;
    if build_ok {
        for s in sites::all_sites() {
            let found = read_site(s.rva, s.expect.len());
            match sites::check_site(s, &found) {
                sites::SiteCheck::Ok => {
                    let _ = writeln!(r, "site ok      {:#011x}  {}", 0x1_4000_0000u64 + s.rva as u64, s.name);
                }
                sites::SiteCheck::Differs(b) => {
                    all_ok = false;
                    let hex: Vec<String> = b.iter().map(|x| format!("{x:02x}")).collect();
                    let _ = writeln!(
                        r,
                        "site CHANGED {:#011x}  {}  (found {})",
                        0x1_4000_0000u64 + s.rva as u64,
                        s.name,
                        hex.join(" ")
                    );
                }
            }
        }
    }
    match ini {
        Ok(text) => {
            let (s, warnings) = settings::parse(&text);
            let _ = writeln!(
                r,
                "settings: combat = {}, kh1_motions = {}, config_menu = {}",
                s.combat.as_str(),
                settings::switch_str(s.kh1_motions),
                settings::switch_str(s.config_menu)
            );
            for w in warnings {
                let _ = writeln!(r, "settings warning: {w}");
            }
        }
        Err(e) => {
            let _ = writeln!(r, "settings: {e}; using defaults");
        }
    }
    let _ = writeln!(
        r,
        "{}",
        if all_ok { "result: all checks passed." } else { "result: checks FAILED. The mod changes nothing on this build." }
    );
    (r, all_ok)
}

#[cfg(windows)]
mod win;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_passes_on_matching_build() {
        let pe = Some(sites::PeInfo { timestamp: sites::STEAM_TIMESTAMP, size_of_image: sites::STEAM_SIZE_OF_IMAGE });
        let read = |rva: u32, n: usize| sites::all_sites().find(|s| s.rva == rva).map(|s| s.expect[..n].to_vec()).unwrap_or_default();
        let path = format!("D:\\Steam\\x\\{}", sites::EXE_NAME);
        let (r, ok) = startup_report(&path, pe, &read, Ok(settings::DEFAULT_FILE.into()));
        assert!(ok, "{r}");
        assert!(r.contains("combat = kh1, kh1_motions = off, config_menu = on"), "{r}");
        assert!(r.contains("Config screen draw"));
    }

    #[test]
    fn report_fails_on_changed_site_or_build() {
        let pe = Some(sites::PeInfo { timestamp: sites::STEAM_TIMESTAMP, size_of_image: sites::STEAM_SIZE_OF_IMAGE });
        let read = |_: u32, n: usize| vec![0xCCu8; n];
        let path = sites::EXE_NAME.to_string();
        let (r, ok) = startup_report(&path, pe, &read, Err("missing".into()));
        assert!(!ok);
        assert!(r.contains("site CHANGED"));
        let other = Some(sites::PeInfo { timestamp: 1, size_of_image: 2 });
        let (_, ok) = startup_report(&path, other, &read, Err("missing".into()));
        assert!(!ok);
    }
}
