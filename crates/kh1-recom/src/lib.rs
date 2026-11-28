//! KH1 combat in Kingdom Hearts Re:Chain of Memories (PC, Steam).
//!
//! OpenKH's Panacea loads this DLL into `KINGDOM HEARTS Re_Chain of Memories.exe` (from
//! `mod_path\Recom\dll`) and calls its `OnInit(mod folder)`. At start it checks that it runs on
//! the Steam build it targets and the first bytes of every function it hooks or calls, reads
//! `kh1-recom.ini`, and writes `kh1-recom.log` next to the game. Then, only when every check
//! passed:
//!
//! - adds "Combat" and "Guard reactions" rows to Re:CoM's Config menu, drawn and driven by
//!   Re:CoM's own menu code, with Re:CoM's window panel drawn taller (`menu`,
//!   `docs/recom-config-menu.md`);
//! - at `combat = kh1`, runs KH1 Sora's movement as the handlers of Re:CoM's own movement actions
//!   (`movement`, `rows`, `units`; `docs/recom-kh1-movement.md`);
//! - at `combat = kh1`, runs KH1's Attack (no cards) through Re:CoM's own attack action, rows,
//!   collision and damage path (`attack`; `docs/recom-kh1-attack.md`);
//! - at `combat = kh1`, in Sora's story, adds an 8th camp menu entry, "Keyblades", whose page
//!   equips the Keyblades Sora has earned, run by Re:CoM's own camp code (`keyblades`;
//!   `docs/recom-keyblade-equip.md`);
//! - at `combat = kh1`, in Sora's story, Sora's weapon shows the equipped Keyblade's model: new
//!   groups in Re:CoM's PLAYER resource table, picked when Re:CoM builds the weapon, and the
//!   weapon rebuilt with Re:CoM's own constructor when the camp closes (`keyblade_models`;
//!   `docs/recom-keyblade-models.md`);
//! - at `combat = kh1`, in Sora's story, Heartless (bosses included) attack without cards and
//!   several at once: the card play does message 2's work, and Re:CoM's three other
//!   serialisers are patched per battle (`battle`; `docs/recom-m6-battles.md`);
//! - at `combat = kh1`, in Sora's story, Sora fights without a deck: Re:CoM's own constructor
//!   builds an empty battle revolver, Friend-card drops stop, and the deck HUD is hidden through
//!   Re:CoM's own event-hide mask (`deck`; `docs/recom-m6-battles.md` § Stage 2);
//! - at `combat = kh1`, in Sora's story, Sora has KH1's MP: max MP by level and Keyblade, MP and
//!   charge kept in a block of Re:CoM's own save, charge from KH1 hits, KH1's refill and payment
//!   rules (`mp`; `docs/recom-m7-mp.md`);
//! - at `combat = kh1`, in Sora's story, a KH1 MP bar under Re:CoM's HP bar: new groups the asset
//!   step adds to Re:CoM's `Cockpit:00`, played on a `C2DSeqCtrl` of Re:CoM's player gauge, set
//!   only when MP changes (`mp_bar`; `docs/recom-m7-mp-bar.md`);
//! - at `combat = kh1`, in Sora's story, KH1's command menu (Attack / Magic / Items / context,
//!   the Magic list) in the field and in battle: Re:CoM's own text plates (new groups of
//!   `Cockpit:00`) and `C2DFnt` text with KH1's own words, KH1's cursor rules, fades and greying;
//!   confirm runs the highlighted row (`command_menu`; `docs/recom-m7-command-menu.md`);
//! - at `combat = kh1`, in Sora's story, KH1's cardless cast and Fire: the command menu's spell
//!   runs KH1's command on Re:CoM's own magic action `0x13` with KH1 rows in Re:CoM's attack-row
//!   format, Re:CoM's cast clips and its shot runtime fire the ball at KH1's release frame, KH1's
//!   MP is paid then, and the ball's hits take KH1's damage (`magic`; `docs/recom-m7-magic.md`);
//! - in Sora's story, KH1's spells are learned from Re:CoM's own card collection: a spell's base
//!   level with its magic card's first pickup, -ra / -ga with Re:CoM's bosses (their enemy cards'
//!   first pickup), through a wrapper on Re:CoM's "first obtained" `0x14015C7F0`, kept in our save
//!   block (`unlocks`; `docs/recom-m7-unlocks.md`);
//! - at `combat = kh1`, in Sora's story, KH1's items: the command menu's Items list of Sora's
//!   filled item slots (kept with the stock in our save block), a use plays KH1's motion on
//!   Re:CoM's own item action `0xE` and clip, and KH1's use task heals through Re:CoM's heal with
//!   Re:CoM's item effects (`items`; `docs/recom-m8-items.md`);
//! - at `combat = kh1`, in Sora's story, a fifth "Items" tab in Re:CoM's Moogle shop pack screen
//!   sells Potion, Hi-Potion and Ether for Moogle Points: Re:CoM's own pack objects, cursors,
//!   sounds, help lines and payment, Re:CoM's item card pictures, KH1's pickup rule (`shop`;
//!   `docs/recom-m8-items.md` § Stage 8a.3);
//! - at `combat = kh1`, in Sora's story, KH1's combo modifiers (Sliding Dash, Blitz, Stun Impact,
//!   Zantetsuken, Vortex, Aerial Sweep, Hurricane Blast, Ripple Drive, Gravity Break): unlocked by
//!   Re:CoM's own learned sleights or KH1's learn levels on Re:CoM's level, kept in our save block,
//!   passed to KH1's pickers, played as KH1 rows on Re:CoM's attack action (`abilities`;
//!   `docs/recom-m9-abilities.md`); and (stage 9b) KH1's ability screen with AP in the KH1 camp
//!   entry (Abilities): the modifiers and the movement abilities (Guard, Counterattack, Dodge Roll,
//!   High Jump, Glide, Superglide, Combo Plus, Air Combo Plus, Combo Master), equipped with KH1's
//!   costs within KH1's max AP on Re:CoM's level;
//! - F9 records a Re:CoM trace in kh1-trace's CSV schema (`trace`).
//!
//! `motions` (with `json`) reads the asset build step's manifest and `PL0001.ANN` header, for the
//! KH1-only clips the movement plays (High Jump, Guard).

pub mod abilities;
pub mod asset_build;
pub mod attack;
pub mod battle;
pub mod boss;
pub mod camera;
pub mod camp_rows;
pub mod command_menu;
pub mod customize;
pub mod d2;
pub mod dim;
pub mod damage;
pub mod damage_cap;
pub mod dark_points;
pub mod effects;
pub mod fx;
pub mod enemy_skills;
pub mod enemy_skills_cmp;
pub mod enemy_skills_deck;
pub mod skill_effects;
pub mod deck;
pub mod drops;
pub mod field_sleights;
pub mod fliers;
pub mod ground;
pub mod gimmick;
pub mod guard;
pub mod held;
pub mod hookplan;
pub mod items;
pub mod kh1_support;
pub mod items_page;
pub mod json;
pub mod keyblade_models;
pub mod keyblades;
pub mod kh1_hud;
pub mod level_up;
pub mod lock_on;
pub mod magic;
pub mod menu_sounds;
pub mod sounds;
pub mod menu;
pub mod motions;
pub mod movement;
pub mod mp;
pub mod mp_bar;
pub mod notices;
pub mod revenge;
pub mod riku;
pub mod riku_skin;
pub mod rows;
pub mod settings;
pub mod kh1_enemy;
pub mod shadow;
pub mod shop;
pub mod sleights;
pub mod specials;
pub mod sites;
pub mod story_skip;
pub mod party;
pub mod summons;
pub mod text_fit;
pub mod trace;
pub mod units;
pub mod unlocks;
pub mod riku_unlocks;

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
                "settings: combat = {}, config_menu = {}",
                s.combat.as_str(),
                settings::switch_str(s.config_menu)
            );
            let _ = writeln!(r, "settings: guard_reactions = {} (only at combat = kh1)", settings::switch_str(s.guard_reactions));
            let _ = writeln!(r, "settings: enemy_pace = {} (only at combat = kh1)", s.enemy_pace.as_str());
            let _ = writeln!(r, "settings: damage = {} (only at combat = kh1; recom: Re:CoM's enemy HP and card / sleight damage)", s.damage.as_str());
            let a = s.abilities;
            let _ = writeln!(
                r,
                "settings: KH1 abilities: high_jump = {}, glide = {}, superglide = {}, dodge_roll = {} (recom: Re:CoM's own roll; kh1: KH1's; off: none), guard = {}",
                settings::switch_str(a.high_jump),
                settings::switch_str(a.glide),
                settings::switch_str(a.superglide),
                s.dodge_roll.as_str(),
                settings::switch_str(a.guard)
            );
            let _ = writeln!(
                r,
                "settings: movement (only at combat = kh1): run_speed = {} (recom: Re:CoM's 6.2 m/s x stick; kh1: 4.8 m/s), jump_height = {} (recom: Re:CoM's 28 / 32.5 take-off, its gravity and cut; kh1: 190 / 290), air_speed = {} (recom: Re:CoM's air control; kh1: KH1's)",
                s.run_speed.as_str(),
                s.jump_height.as_str(),
                s.air_speed.as_str()
            );
            let _ = writeln!(r, "settings: KH1 combo: combo_plus = {}, air_combo_plus = {}, combo_master = {}", a.combo_plus, a.air_combo_plus, settings::switch_str(a.combo_master));
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
        assert!(r.contains("combat = kh1, config_menu = on"), "{r}");
        assert!(r.contains("guard_reactions = on"), "{r}");
        assert!(r.contains("run_speed = recom ") && r.contains("jump_height = kh1 ") && r.contains("air_speed = kh1 "), "{r}");
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
