//! Windows side: start-up (from Panacea's `OnInit`, or a fallback thread), hooks, and the glue
//! between the KH1 camp entry's settings pages and `kh1-recom.ini`.

mod abilities;
mod assets;
mod attack;
mod battle;
mod camera;
mod command_menu;
mod crash;
mod d2;
mod damage;
mod damage_cap;
mod dark_points;
mod deck;
mod drops;
mod effects;
mod enemy_skills_cmp;
mod enemy_skills_deck;
mod enemy_skills;
mod fx;
mod field_sleights;
mod gimmick;
mod guard;
mod held;
mod hook;
mod items;
mod keyblades;
mod kh1_hud;
mod level_up;
mod lock_on;
mod magic;
mod magic_fx;
mod menu;
mod menu_sounds;
mod models;
mod movement;
mod mp;
mod mp_bar;
mod notices;
mod revenge;
mod riku_skin;
mod sora_costume;
mod kh1_enemy;
mod kh1_thrown;
mod kh1_cure;
mod kh1_flame;
mod kh1_peel;
mod kh1_support;
mod shop;
mod sounds;
mod specials;
mod stats;
mod story_skip;
mod party;
mod summons;
mod sys;
mod text_fit;
mod trace;
mod unlocks;

use crate::settings::{self, Settings};
use std::ffi::c_void;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, Once};
use sys::*;

static BASE: AtomicUsize = AtomicUsize::new(0);
static MODULE: AtomicUsize = AtomicUsize::new(0);
static START: Once = Once::new();
static LOG_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
static INI_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
static SETTINGS: Mutex<Option<Settings>> = Mutex::new(None);

/// The exe's image base.
pub(crate) fn base() -> usize {
    BASE.load(Ordering::Relaxed)
}

/// A Steam virtual address (image base 0x140000000) in the running image.
pub(crate) fn va(base: usize, addr: u64) -> usize {
    base + (addr - 0x1_4000_0000) as usize
}

/// Appends a line to `kh1-recom.log`.
pub(crate) fn log(msg: &str) {
    let Ok(g) = LOG_PATH.lock() else { return };
    if let Some(p) = g.as_ref() {
        if let Ok(mut f) = std::fs::OpenOptions::new().append(true).create(true).open(p) {
            let _ = writeln!(f, "{msg}");
        }
    }
}

pub(crate) fn settings() -> Settings {
    SETTINGS.lock().ok().and_then(|g| g.clone()).unwrap_or_default()
}

/// The setting changed and `kh1-recom.ini` rewritten, nothing else told (None: unchanged).
pub(crate) fn store_item(item: crate::menu::Item, v: usize) -> Option<Settings> {
    let mut s = settings();
    let was = s.clone();
    item.set(&mut s, v);
    if s == was {
        return None;
    }
    if let Ok(mut g) = SETTINGS.lock() {
        *g = Some(s.clone());
    }
    if let Some(p) = INI_PATH.lock().ok().and_then(|g| g.clone()) {
        let text = std::fs::read_to_string(&p).unwrap_or_else(|_| settings::DEFAULT_FILE.to_string());
        if let Err(e) = std::fs::write(&p, settings::store(&text, &s)) {
            log(&format!("kh1 entry: could not write {}: {e}", p.display()));
        }
    }
    Some(s)
}

/// A row of the KH1 entry's settings pages (`crate::menu`) set to option `v`: the setting changes,
/// `kh1-recom.ini` is rewritten (other lines and comments kept), and whatever reads the setting
/// live is told now. When each change takes effect: `menu::Item::effect` and
/// `docs/recom-config-menu.md`.
pub(crate) fn set_item(item: crate::menu::Item, v: usize) {
    use crate::menu::Item;
    let Some(s) = store_item(item, v) else { return };
    let shown = String::from_utf8_lossy(item.options()[v.min(item.options().len() - 1)]).trim_end_matches('\0').to_string();
    let when = match item.effect() {
        crate::menu::Effect::Now => "now",
        crate::menu::Effect::NextBattle => "from the next battle",
        crate::menu::Effect::NextRoom => "from the next room",
    };
    log(&format!("kh1 entry: {} = {shown} ({when})", item.key()));
    match item {
        // Read at each battle start (`battle::manager_ctor`, each enemy's ctor).
        Item::EnemyPace | Item::EnemyAi => {}
        // M13: read at each of Sora's KH1 motions (`fx::variants`: the next swing; KH1's banks are
        // made sure of there when Sounds turns KH1).
        Item::Effects | Item::Sounds => {}
        // M12 camera: the dispatcher switches the live camera at its next frame.
        Item::Camera => camera::set_choice(s.camera),
        Item::AutoCamera => camera::set_auto(s.auto_camera),
        // Read by the lock-on wrapper each frame (`lock_on`).
        Item::AutoLock | Item::TargetLock => {}
        Item::GuardReactions => attack::set_guard_reactions(s.guard_reactions),
        Item::RevengeValue => revenge::set_enabled(s.boss_revenge),
        // Fork: Riku's Costume: Re:CoM's char -> group entries now; the next player build shows it.
        // SAFETY: two checked writes of Re:CoM's static tables (the camp: no player is being built).
        // Fork (0.170.0): Sora's costume, from the next room or battle.
        // SAFETY: checked writes of Re:CoM's static tables (the camp: no player is being built).
        Item::SoraCostume => unsafe { sora_costume::apply(base(), s.sora_halloween && sora_costume::unlocked()) },
        Item::Costume | Item::Blade | Item::Hair => unsafe {
            riku_skin::apply(base(), riku_skin::chosen(&s));
            riku_skin::costume_chosen(base());
        },
        // Read by each KH1 frame (`movement::run_frame`, `stick_class`).
        Item::RunSpeed | Item::JumpHeight | Item::AirSpeed => movement::set_speeds(crate::movement::speeds::Speeds { run: s.run_speed, jump: s.jump_height, air: s.air_speed }),
        Item::DodgeRoll => {
            movement::set_roll_mode(s.dodge_roll);
            movement::set_abilities(s.abilities);
        }
        Item::Combat => {
            movement::set_mode(s.combat == settings::Mode::Kh1);
            // The MP bar follows the setting (made or removed on the live player gauges).
            mp_bar::refresh();
            // The weapon's group follows Combat: its files start loading now; the camp close shows it.
            models::prefetch();
            // KH1 casts' effects: Re:CoM's battle set starts loading now in the field (`magic_fx`).
            magic_fx::setting_changed();
            // Enemy Skills: applied at KH1, taken off at Re:CoM, on the player now.
            // SAFETY: Re:CoM's main player's fields, written as Re:CoM's own apply does.
            unsafe { enemy_skills::refresh("Combat switched") };
        }
    }
}

/// The KH1-only clips the asset step added (Guard): the manifest
/// `kh1-motions.json` and the `PL0001.ANN` header Panacea will serve, looked for under the mod
/// folder Panacea passed, its `recom` folder, the DLL's folders and the game folder.
fn recom_bases(dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut bases: Vec<PathBuf> = Vec::new();
    for d in dirs {
        bases.push(d.clone());
        if let Ok(rd) = std::fs::read_dir(d) {
            for e in rd.flatten() {
                if e.file_name().to_string_lossy().eq_ignore_ascii_case("recom") && e.path().is_dir() {
                    bases.push(e.path());
                }
            }
        }
    }
    bases
}

fn kh1_clips(dirs: &[PathBuf]) -> (crate::rows::Extension, Vec<(u32, String)>) {
    use crate::{motions, rows};
    let bases = recom_bases(dirs);
    let Some(manifest) = bases.iter().map(|b| b.join("kh1-motions.json")).find(|p| p.is_file()) else {
        log("movement: no kh1-motions.json found (asset step not run?): Guard has no clip");
        return (rows::Extension::default(), Vec::new());
    };
    let text = std::fs::read_to_string(&manifest).map_err(|e| e.to_string());
    let m = match text.clone().and_then(|t| motions::parse_manifest(&t)) {
        Ok(m) => m,
        Err(e) => {
            log(&format!("movement: {}: {e}", manifest.display()));
            return (rows::Extension::default(), Vec::new());
        }
    };
    // Stage 9c.1: KH1's specials' motions (`xl_limit.dat`), keyed by xl entry.
    let xl = match text.clone().and_then(|t| motions::parse_xl(&t)) {
        Ok(x) => x,
        Err(e) => {
            log(&format!("specials: {}: {e}; no special clips", manifest.display()));
            Vec::new()
        }
    };
    let ann = bases.iter().map(|b| b.join("PLAYER").join("0001").join("PL0001.ANN")).find(|p| p.is_file()).and_then(|p| {
        use std::io::Read;
        let mut f = std::fs::File::open(&p).ok()?;
        let mut head = [0u8; 4];
        f.read_exact(&mut head).ok()?;
        let n = u32::from_le_bytes(head).min(4096);
        let mut rest = vec![0u8; motions::ann_header_len(n) - 4];
        f.read_exact(&mut rest).ok()?;
        let mut all = head.to_vec();
        all.extend_from_slice(&rest);
        motions::ann_header(&all).ok()
    });
    let Some(ann) = ann else {
        log("movement: no extended PL0001.ANN found next to the manifest: KH1-only clips not used");
        return (rows::Extension::default(), Vec::new());
    };
    // Entries Re:CoM already has (M9 9a: the modifiers it baked) play its own action row of the
    // clip; the row's name is read from Sora's action table in the running exe.
    let table = recom_row_names();
    // M11 (the Neoshadow's grab): KH1's held clip, for Sora's held reaction (`held`).
    let held = match text.and_then(|t| motions::parse_held(&t)) {
        Ok(h) => h.filter(|h| match motions::held_in_pack(h, &ann) {
            Ok(_) => true,
            Err(e) => {
                log(&format!("held: {e}; no held reaction"));
                false
            }
        }),
        Err(e) => {
            log(&format!("held: {}: {e}; no held reaction", manifest.display()));
            None
        }
    };
    let ext = rows::Extension::plan(|e| motions::clip_for_entry(&m, &ann, e).is_ok())
        .with_recom(|e| motions::recom_clip_for_entry(&m, e).and_then(|c| motions::row_of_clip(&table, &c.name)))
        .with_xl(|e| motions::clip_for_entry(&xl, &ann, e as u32).is_ok())
        .with_held(held.is_some());
    let mut names = rows::appended_rows(&ext, |e| motions::clip_for_entry(&m, &ann, e).ok().map(|c| c.name.clone()), |e| motions::clip_for_entry(&xl, &ann, e as u32).ok().map(|c| c.name.clone()));
    if let Some(r) = rows::held_row(&ext, held.as_ref().map(|h| h.clip.name.clone())) {
        names.push(r);
    }
    if let Some(h) = &held {
        let _ = held::PACK_BLEND.set(h.blend);
    }
    let recom: Vec<(u16, u32)> = rows::KH1_ONLY.iter().zip(ext.recom.iter()).filter_map(|(&(req, _), r)| Some((req, (*r)?))).collect();
    log(&format!("movement: manifest {}; KH1-only clips {:?}; Re:CoM's rows for KH1 requests {:x?}; xl_limit clips (specials) {:?}; held clip {:?}", manifest.display(), names, recom, ext.xl, ext.held));
    (ext, names)
}

/// The clip names of Re:CoM's own rows of Sora's action table (`0x1407C5E80`: 16-byte rows
/// `{const char* name; int index; int pad}`, 0x47 rows), read from the running exe.
fn recom_row_names() -> Vec<Option<String>> {
    let b = base();
    let table = va(b, crate::sites::data::SORA_TABLE);
    (0..crate::rows::RECOM_ROWS as usize)
        .map(|i| {
            // SAFETY: rows of the image's own table; each name a NUL-terminated string in the image,
            // read a byte at a time up to its NUL (at most 16: Re:CoM's clip names are shorter).
            unsafe {
                let p: usize = std::ptr::read_unaligned((table + i * 16) as *const usize);
                if p == 0 {
                    return None;
                }
                let mut name = Vec::new();
                for k in 0..16 {
                    match *((p + k) as *const u8) {
                        0 => return String::from_utf8(name).ok(),
                        c => name.push(c),
                    }
                }
                None
            }
        })
        .collect()
}

/// The KH1 movement handlers (`movement`) and the trace's sampling points (`trace`).
fn install_movement(base: usize, s: &Settings, dirs: &[PathBuf], game_dir: &Path) {
    let (ext, names) = kh1_clips(dirs);
    // SAFETY: data patch with checked old values, before the first room load.
    let ext = if unsafe { movement::extend_action_table(&ext, &names) } { ext } else { crate::rows::Extension::default() };
    let _ = movement::EXT.set(ext);
    let site = |name: &str| crate::sites::MOVE_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [
        hook::Hook { site: site("move exe (actions 2/3/4)"), ours: movement::move_exe as *const () as usize, original: &movement::ORIG_MOVE_EXE },
        hook::Hook { site: site("stick class (0x1401E9630)"), ours: movement::stick_class as *const () as usize, original: &movement::ORIG_STICK_CLASS },
        hook::Hook { site: site("move step (0x1401EF3C0)"), ours: movement::move_step as *const () as usize, original: &movement::ORIG_MOVE_STEP },
        hook::Hook { site: site("jump init (0x1401F29F0)"), ours: movement::jump_init as *const () as usize, original: &movement::ORIG_JUMP_INIT },
        hook::Hook { site: site("avoid init (0x1401F10C0)"), ours: movement::avoid_init as *const () as usize, original: &movement::ORIG_AVOID_INIT },
        hook::Hook { site: site("jump exe (action 7)"), ours: movement::jump_exe as *const () as usize, original: &movement::ORIG_JUMP_EXE },
        hook::Hook { site: site("CPlayerSora avoid exe (action 6)"), ours: movement::avoid_exe as *const () as usize, original: &movement::ORIG_AVOID_EXE },
        hook::Hook { site: site("CPlayerRiku avoid exe (action 6)"), ours: movement::riku_avoid_exe as *const () as usize, original: &movement::ORIG_RIKU_AVOID_EXE },
        hook::Hook { site: site("CPlayerDRiku avoid exe (action 6)"), ours: movement::driku_avoid_exe as *const () as usize, original: &movement::ORIG_DRIKU_AVOID_EXE },
        hook::Hook { site: site("CPlayerSora glide exe (action 8)"), ours: movement::glide_exe as *const () as usize, original: &movement::ORIG_GLIDE_EXE },
    ];
    // SAFETY: the targets' bytes are checked again inside, before anything is written.
    if unsafe { hook::install_all(base, &hooks) } {
        movement::mark_installed();
        movement::set_roll_mode(s.dodge_roll);
        movement::set_speeds(crate::movement::speeds::Speeds { run: s.run_speed, jump: s.jump_height, air: s.air_speed });
        movement::set_abilities(s.abilities);
        abilities::set_ini(&s.abilities);
        movement::set_mode(s.combat == settings::Mode::Kh1);
        movement::set_riku(s.riku_kh1_movement);
        if s.riku_kh1_movement {
            log("riku: riku_kh1_movement = on: at combat = kh1, Riku in Reverse/Rebirth moves with KH1's movement (testing)");
        }
        log(&format!(
            "movement: KH1 handlers in Re:CoM's actions 2/3/4, 6, 7, 8 ({}); abilities {:?}; dodge_roll = {} ({}); run_speed = {}, jump_height = {}, air_speed = {}",
            if s.combat == settings::Mode::Kh1 { "on: combat = kh1" } else { "pass-through: combat = recom" },
            s.abilities,
            s.dodge_roll.as_str(),
            match s.dodge_roll {
                settings::RollMode::Recom => "KH1's Square rule starts Re:CoM's own roll: its avoid init 0x1401F10C0 and exe 0x140217310 whole",
                settings::RollMode::Kh1 => "KH1's roll on Re:CoM's action 6, KH1's 12 no-flinch frames",
                settings::RollMode::Off => "no Dodge Roll: Square guards",
            },
            s.run_speed.as_str(),
            s.jump_height.as_str(),
            s.air_speed.as_str()
        ));
    } else {
        log("movement: not hooked; Sora moves as in Re:CoM");
    }
    let stats_file = recom_bases(dirs).into_iter().map(|b| b.join("kh1-stats.json")).find(|p| p.is_file());
    let bar_file = recom_bases(dirs).into_iter().map(|b| b.join("kh1-mp-bar.json")).find(|p| p.is_file());
    let menu_file = recom_bases(dirs).into_iter().map(|b| b.join("kh1-command-menu.json")).find(|p| p.is_file());
    let hud_file = recom_bases(dirs).into_iter().map(|b| b.join("kh1-hud.json")).find(|p| p.is_file());
    let sound_file = recom_bases(dirs).into_iter().map(|b| b.join(crate::menu_sounds::MANIFEST)).find(|p| p.is_file());
    // M13 stage 1: the KH1 effect sets the asset step added (checked at the first battle).
    effects::install(base, recom_bases(dirs).into_iter().map(|b| b.join(crate::effects::MANIFEST)).find(|p| p.is_file()).as_deref());
    // M13 stage 4: the fx manifest first (the Keyblade model table names each group's own EET / EFF).
    if movement::installed() {
        fx::preload(recom_bases(dirs).into_iter().map(|b| b.join(crate::fx::MANIFEST)).find(|p| p.is_file()).as_deref());
    }
    // Riku's Dark Form skin (fork): its groups' rows go in the same table.
    riku_skin::preload(recom_bases(dirs).into_iter().map(|b| b.join(kh1_recom_assets::riku_skin::MANIFEST)).find(|p| p.is_file()).as_deref());
    install_attack(base, s, stats_file.as_deref(), bar_file.as_deref(), menu_file.as_deref(), hud_file.as_deref(), sound_file.as_deref());
    // M13 stage 2: KH1's system bank also carries the KH1 effects' sounds, so it is read even
    // without the KH1 command menu look; then KH1's own SCD banks.
    if !menu_sounds::installed() && sound_file.is_some() {
        menu_sounds::install(base, sound_file.as_deref());
    }
    sounds::install(base, recom_bases(dirs).into_iter().map(|b| b.join(crate::sounds::MANIFEST)).find(|p| p.is_file()).as_deref());
    // M13 stage 3: KH1's effect / sound events of Sora's moves (variants) and KH1's trail end.
    if movement::installed() {
        fx::install(base);
    }
    install_battles(base);
    install_enemies(base, &recom_bases(dirs));
    install_revenge(base, s);
    install_story_skip(base);
    let th = &crate::sites::TRACE_HOOKS;
    let hooks = [
        hook::Hook { site: &th[0], ours: trace::field_exe as *const () as usize, original: &trace::ORIG_FIELD },
        hook::Hook { site: &th[1], ours: trace::battle_exe as *const () as usize, original: &trace::ORIG_BATTLE },
    ];
    // SAFETY: as above.
    if unsafe { hook::install_all(base, &hooks) } {
        trace::start(game_dir.to_path_buf());
        log("trace: press F9 in game to start / stop recording kh1-recom-trace-<n>.csv next to the game");
    } else {
        log("trace: not hooked");
    }
}

/// KH1's Attack (`attack`, `docs/recom-kh1-attack.md`): the KH1 rows, the data patches, then the
/// hooks, all or nothing. Needs the movement handlers.
fn install_attack(base: usize, s: &Settings, stats_file: Option<&Path>, bar_file: Option<&Path>, menu_file: Option<&Path>, hud_file: Option<&Path>, sound_file: Option<&Path>) {
    if !movement::installed() {
        log("attack: movement not hooked; KH1 Attack not installed");
        return;
    }
    attack::set_debug(s.debug_attack);
    attack::set_guard_reactions(s.guard_reactions);
    // The Damage setting (owner, 2026-11-01), read once at start.
    damage::set_mode(s.damage == settings::Mode::Kh1);
    // SAFETY: reads of the mapped exe.
    if s.damage == settings::Mode::Kh1 {
        log("damage: damage = kh1: KH1's formula and the KH1 twins' / proxies' HP (decisions 12, 21)");
    } else if unsafe { damage::install(base) } {
        log("damage: damage = recom: Re:CoM's enemy HP; in battle KH1 swings carry the equipped Keyblade card's AP (normal swings column 0, finishers column 2), Fire / Blizzard / Thunder their Re:CoM card's / sleight's, Sonic Blade its sleight's; Re:CoM's damage path as is");
    }
    abilities::set_all(s.kh1_all_abilities);
    if s.debug_attack {
        log("attack: debug_attack = on (first 200 lines)");
    }
    // SAFETY: reads of the mapped exe; data patches with checked old values.
    if !unsafe { attack::build_table(base) && attack::patch_data(base) } {
        return;
    }
    let site = |name: &str| crate::sites::ATTACK_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [
        hook::Hook { site: site("request bits (0x1401F8150)"), ours: attack::request_bits as *const () as usize, original: &attack::ORIG_REQUEST_BITS },
        hook::Hook { site: site("use-card button (0x1401E8FF0)"), ours: attack::card_button as *const () as usize, original: &attack::ORIG_CARD_BUTTON },
        hook::Hook { site: site("initAttack (0x1401F0D90)"), ours: attack::init_attack as *const () as usize, original: &attack::ORIG_INIT_ATTACK },
        hook::Hook { site: site("field swing init (0x1401F0F70)"), ours: attack::field_init as *const () as usize, original: &attack::ORIG_FIELD_INIT },
        hook::Hook { site: site("field swing exe (0x1401EAEB0)"), ours: attack::swing_exe as *const () as usize, original: &attack::ORIG_SWING_EXE },
        hook::Hook { site: site("attack pair dispatch (0x14016EAD0)"), ours: attack::pair_dispatch as *const () as usize, original: &attack::ORIG_PAIR },
        // Boss revenge wraps the KH1 enemy-hit wrapper (`revenge::enemy_hit` calls `attack::enemy_hit`).
        hook::Hook { site: site("enemy hit handler (0x14017F7E0)"), ours: revenge::enemy_hit as *const () as usize, original: &attack::ORIG_ENEMY_HIT },
        hook::Hook { site: site("player damage check (0x1401E6DE0)"), ours: attack::damage_check as *const () as usize, original: &attack::ORIG_DAMAGE_CHECK },
        hook::Hook { site: site("root motion step (0x14024DCA0)"), ours: attack::root_step as *const () as usize, original: &attack::ORIG_ROOT_STEP },
        hook::Hook { site: site("enemy attack hurt off (0x14017DA80)"), ours: attack::hurt_off as *const () as usize, original: &attack::ORIG_HURT_OFF },
    ];
    // SAFETY: the targets' bytes are checked again inside, before anything is written.
    if unsafe { hook::install_all(base, &hooks) } {
        let a = s.abilities;
        log(&format!(
            "attack: KH1 Attack in Re:CoM's attack action 0xA, field and battle (combo_plus {}, air_combo_plus {}, combo_master {}); KH1 Keyblade chain in the pair dispatch; hits land during enemy cards; no-flinch roll; root motion one update late",
            a.combo_plus, a.air_combo_plus, a.combo_master
        ));
        if guard::install(base) {
            log(&format!(
                "guard reactions: in the pair dispatch and enemy hit wrappers (decision 27; {} at combat = kh1)",
                settings::switch_str(s.guard_reactions)
            ));
        }
        install_stats(base, stats_file);
        log(&format!(
            "mp: KH1 MP at combat = kh1 in Sora's story (M7 stage 7.1): save+{:#x}..={:#x} layout v{}; charge from KH1 hits in the enemy-hit wrapper, refill one point per frame in the request-bits wrapper; no hook of its own",
            crate::mp::BLOCK_FIRST,
            crate::mp::BLOCK_LAST,
            crate::mp::LAYOUT_V1
        ));
        install_mp_refills(base);
        // The MP bar (M7 stage 7.2): Re:CoM's player gauge draws it; needs the asset step's file.
        // The command menu (7.3) runs from its wrappers and needs its plate groups.
        if mp_bar::install(base, bar_file) {
            // KH1's own menu sounds (the KH1 look's), in Re:CoM's sound banks.
            if command_menu::install(base, menu_file, s) {
                menu_sounds::install(base, sound_file);
            }
            // The KH1 HUD (decision 39) hangs from the same gauge wrappers.
            kh1_hud::install(base, hud_file);
        }
        install_magic(base);
        install_items(base, s);
        // Lock-on on R1 at combat = kh1 on a controller (`crate::lock_on`).
        let lk = [hook::Hook { site: &crate::sites::LOCK_ON_HOOK[0], ours: lock_on::lock_on as *const () as usize, original: &lock_on::ORIG_LOCK_ON }];
        let checked = crate::sites::LOCK_ON_CHECKED.iter().all(|s| {
            // SAFETY: a read of the mapped exe's code.
            unsafe { std::slice::from_raw_parts((base + s.rva as usize) as *const u8, s.expect.len()) == s.expect }
        });
        // SAFETY: the target's bytes are checked again inside, before anything is written.
        if checked && unsafe { hook::install_all(base, &lk) } {
            lock_on::mark_installed();
            // KH1's lock rules (0.137.0): the lock-on's input calls get KH1's answers.
            // SAFETY: reads of the mapped exe; each call's bytes are checked again before it is written.
            unsafe { lock_on::install_rules(base) };
        } else {
            log("lock-on: Re:CoM's lock-on not hooked; the lock-on stays on Re:CoM's button (R2)");
        }
        install_specials(base);
        install_drops(base);
        install_gimmick(base);
        install_unlocks(base);
        install_summons(base);
        // Enemy Skills stage 2: the play-time setters (`enemy_skills`).
        // SAFETY: reads of the mapped exe; the hook's target bytes are checked again inside.
        if !unsafe { enemy_skills::install(base) } {
            log("enemy skills: route 1 not installed: the skills it builds do nothing");
        }
        install_field_sleights(base);
        // M11 (the Neoshadow's grab): Sora's held reaction, in the damage-check wrapper above.
        // SAFETY: reads of the mapped exe; the data patch's old bytes are checked inside.
        unsafe { held::install(base) };
        // M12 step 3: Second Chance's rule in Re:CoM's damage check (armed by its wrapper above).
        // SAFETY: the patch site's and its context's bytes are checked inside, before anything is written.
        unsafe { kh1_support::install_second_chance(base) };
        // Sora's damage cap (owner, 2026-11-05): KH1's per-hit cap at Re:CoM's damage join, armed per hit by
        // the enemy-hit wrapper above.
        // SAFETY: the patch site's and its context's bytes are checked inside, before anything is written.
        unsafe { damage_cap::install(base) };
        // Enemy Skills stage 4: Re:CoM's direct compares (Quick Recovery, Dash, Auto-Life, Invincible,
        // Sleightblind's resistance), after Second Chance (Invincible's first site is in its checked context).
        // SAFETY: every site's and its context's bytes are checked inside, before anything is written.
        unsafe { enemy_skills_cmp::install(base) };
    } else {
        log("attack: not hooked; Sora attacks as in Re:CoM (the Battle case and row-pick slot stay patched but are not reached)");
    }
}

/// KH1's cardless cast and Fire (M7 stage 7.4, `magic`): the KH1 rows, the Field exe's case for
/// action 0x13, then the exeMagic hook. Needs the Attack's hooks (the request-bits wrapper and the
/// enemy-hit wrapper carry its command and hits).
fn install_magic(base: usize) {
    // SAFETY: the vtable slot is a read of the mapped image.
    let slot_ok = unsafe {
        let p = &crate::sites::MAGIC_PTRS[0];
        movement::rd::<usize>(base + p.rva as usize) == base + p.target as usize
    };
    if !slot_ok {
        log("magic: CPlayerSora's exeMagic slot is not 0x140217EE0; no KH1 cast");
        return;
    }
    // SAFETY: reads of the mapped exe; data patches with checked old values.
    if !unsafe { magic::install(base) } {
        return;
    }
    let site = |name: &str| crate::sites::MAGIC_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [hook::Hook { site: site("exeMagic (0x140217EE0)"), ours: magic::exe_magic as *const () as usize, original: &magic::ORIG_EXE_MAGIC }];
    // SAFETY: the target's bytes are checked again inside, before anything is written.
    if unsafe { hook::install_all(base, &hooks) } {
        magic::mark_installed();
        log(&magic::start_line());
        // 0.184.0: Riku's / Dark Riku's empty exeMagic slots run the KH1 cast too.
        // SAFETY: data patches of the mapped image with checked old values.
        unsafe { magic::riku_slots(base) };
        // A Gravity-flattened enemy stays hittable in KH1 battles (`crate::magic::flatten`).
        let fl = crate::sites::FLATTEN_HOOKS;
        let flatten = [
            hook::Hook { site: &fl[0], ours: magic::flatten_begin as *const () as usize, original: &magic::ORIG_FLATTEN_BEGIN },
            hook::Hook { site: &fl[1], ours: magic::flatten_end as *const () as usize, original: &magic::ORIG_FLATTEN_END },
        ];
        // SAFETY: the targets' bytes are checked again inside, before anything is written.
        if unsafe { hook::install_all(base, &flatten) } {
            log("magic: Re:CoM's Gravity flatten (0x14017C350 / 0x14017EB80) keeps the enemy's hurt bodies on in KH1 battles; a flatten cut short by a hit springs back at KH1's 0.05 a frame");
        } else {
            log("magic: the flatten state's begin / end not hooked; a flattened enemy can't be hit (Re:CoM's rule)");
        }
        // KH1's own hit flatten at Damage = KH1 (`crate::magic::flatten::kh1`): its numbers in Re:CoM's
        // flatten state's exe. Only with the begin / end wrappers (they arm and drop it).
        let fe = [hook::Hook { site: &crate::sites::FLATTEN_EXE_HOOK[0], ours: magic::flatten_exe as *const () as usize, original: &magic::ORIG_FLATTEN_EXE }];
        let checked = crate::sites::FLATTEN_EXE_CHECKED.iter().all(|s| {
            // SAFETY: a read of the mapped exe's code.
            unsafe { std::slice::from_raw_parts((base + s.rva as usize) as *const u8, s.expect.len()) == s.expect }
        });
        // SAFETY: the target's bytes are checked again inside, before anything is written.
        if magic::ORIG_FLATTEN_END.load(std::sync::atomic::Ordering::Acquire) != 0 && checked && unsafe { hook::install_all(base, &fe) } {
            magic::kh1_flatten_on();
            log("magic: KH1's hit flatten at damage = kh1: KH1 Gravity's flinch becomes Re:CoM's flatten (picker 0xD) with KH1's numbers in its exe 0x140183BA0 (squash 8 / 15 frames, hold 60, get-up +30 % a frame)");
        } else {
            log("magic: the flatten state's exe not hooked; at damage = kh1 KH1 Gravity flinches (no flatten)");
        }
        // SAFETY: branch patches with checked old bytes (code of the mapped exe).
        if unsafe { magic_fx::install(base) } {
            log("magic fx: Re:CoM's battle set (spell effects) in the field at combat = kh1 in Sora's story: field state 1 0x1401AF933 -> 0x140211AA0 then 0x140211960; camp unloads 0x1401A95BD / 0x1401A99F9 keep it");
        }
        // 0.184.2: Riku's battles load Sora's spell effects too (his KH1 spells).
        // SAFETY: a branch patch with checked old bytes.
        unsafe { magic_fx::install_riku(base) };
    } else {
        log("magic: exeMagic not hooked; no KH1 cast (the Field case stays patched but is not reached: Re:CoM never casts in the field)");
    }
}

/// M8 stage 8a.1 (`items`): KH1's items on Re:CoM's item action `0xE`: the Field exe's case for
/// it, then the exeItem hook. Needs the Attack's hooks (the request-bits wrapper carries the command
/// and the use task) and the command menu (its Items list issues them).
fn install_items(base: usize, s: &Settings) {
    // SAFETY: reads of the mapped exe; data patches with checked old values.
    if !unsafe { items::install(base, s) } {
        return;
    }
    let site = |name: &str| crate::sites::ITEM_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [hook::Hook { site: site("exeItem (0x1401ED6E0)"), ours: items::exe_item as *const () as usize, original: &items::ORIG_EXE_ITEM }];
    // SAFETY: the target's bytes are checked again inside, before anything is written.
    if unsafe { hook::install_all(base, &hooks) } {
        items::mark_installed();
        log(&items::start_line());
        install_shop(base);
    } else {
        log("items: exeItem not hooked; no KH1 items (the Field case stays patched but is not reached: Re:CoM never uses items in the field)");
    }
}

/// M9 stage 9c.1 (`specials`): KH1's Sonic Blade on Re:CoM's sleight action `0x16`, in battle
/// (the Battle exe's own case): the exeStockSkill hook. Needs the Attack's hooks (the request-bits
/// wrapper, the pair dispatch and the enemy-hit wrapper carry its command, spheres and damage),
/// the command menu (its context tab offers it) and the asset step's xl_limit clips.
fn install_specials(base: usize) {
    if !command_menu::installed() {
        log("specials: no KH1 command menu; no KH1 specials");
        return;
    }
    // SAFETY: the vtable slot is a read of the mapped image.
    let slot_ok = unsafe {
        let p = &crate::sites::SPECIAL_PTRS[0];
        movement::rd::<usize>(base + p.rva as usize) == base + p.target as usize
    };
    if !slot_ok {
        log("specials: CPlayerSora's exeStockSkill slot is not 0x140218950; no KH1 specials");
        return;
    }
    // SAFETY: reads of the mapped exe.
    if !unsafe { specials::install(base) } {
        return;
    }
    let site = |name: &str| crate::sites::SPECIAL_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [hook::Hook { site: site("exeStockSkill (0x140218950)"), ours: specials::exe_stock as *const () as usize, original: &specials::ORIG_EXE_STOCK }];
    // SAFETY: the target's bytes are checked again inside, before anything is written.
    if unsafe { hook::install_all(base, &hooks) } {
        specials::mark_installed();
        log(&specials::start_line());
    } else {
        log("specials: exeStockSkill not hooked; no KH1 specials");
    }
}

/// M8d (`shop`): KH1's items in Re:CoM's own Moogle shop screens: wrappers on the pack screen's
/// update (its Items tab) and the card trade's (its Square without room for 5 cards), both or
/// none. Needs KH1's items (their store and pickup rule).
fn install_shop(base: usize) {
    let site = |name: &str| crate::sites::SHOP_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [
        hook::Hook { site: site("pack screen update (0x1403A6930)"), ours: shop::pack_screen as *const () as usize, original: &shop::ORIG_PACK_SCREEN },
        hook::Hook { site: site("card trade update (0x1403A73E0)"), ours: shop::trade as *const () as usize, original: &shop::ORIG_TRADE },
    ];
    // SAFETY: the targets' bytes are checked again inside, before anything is written.
    if unsafe { hook::install_all(base, &hooks) } {
        shop::mark_installed();
        log(&shop::start_line());
    } else {
        log("shop: the pack screen / card trade not hooked; the Moogle shop is Re:CoM's (no Items tab)");
    }
}

/// M8 stage 8a.4 (`drops`): KH1's item drops from enemies as a new Re:CoM prize kind. Needs the
/// items (the pickup puts the item in Sora's slots / stock) and the stats file's drop tables
/// (format 6). The look row first (harmless alone: Re:CoM's kinds read the same rows), then the
/// two wrappers, all or nothing.
fn install_drops(base: usize) {
    if !items::installed() {
        log("drops: KH1 items not installed; no KH1 drops");
        return;
    }
    if stats::drop_classes() == 0 {
        log("drops: kh1-stats.json has no drop tables (format 6: re-run kh1-recom-build); no KH1 drops");
        return;
    }
    // SAFETY: reads of the mapped exe; data patches with checked old values.
    if !unsafe { drops::install(base) } {
        return;
    }
    let site = |name: &str| crate::sites::DROP_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [
        hook::Hook { site: site("defeat card prize (0x140180D20)"), ours: drops::defeat_prize as *const () as usize, original: &drops::ORIG_DEFEAT_PRIZE },
        hook::Hook { site: site("prize add (0x1401F4AB0)"), ours: drops::prize_add as *const () as usize, original: &drops::ORIG_PRIZE_ADD },
    ];
    // SAFETY: the targets' bytes are checked again inside, before anything is written.
    if unsafe { hook::install_all(base, &hooks) } {
        drops::mark_installed();
        log(&drops::start_line());
    } else {
        log("drops: not hooked; no KH1 drops (the look table copy stays: Re:CoM's kinds read the same rows)");
    }
}

/// Re:CoM's gimmick cards at Combat = KH1 (`gimmick`, `docs/recom-gimmick-cards.md`): the drop log on
/// Re:CoM's prize factory, the pickup in the prize add wrapper (`drops::prize_add`: installed here
/// when the KH1 item drops didn't install it), "Stagger" in the command menu's context tab.
fn install_gimmick(base: usize) {
    if !command_menu::installed() {
        log("gimmick: no KH1 command menu; gimmick cards stay Re:CoM's at combat = kh1 (held in the hidden revolver, no Stagger)");
        return;
    }
    // SAFETY: reads of the mapped exe.
    if !unsafe { gimmick::install(base) } {
        return;
    }
    let mut hooks = vec![hook::Hook { site: &crate::sites::GIMMICK_HOOKS[0], ours: gimmick::prize_spawn as *const () as usize, original: &gimmick::ORIG_PRIZE_SPAWN }];
    if !drops::installed() {
        let site = crate::sites::DROP_HOOKS.iter().find(|h| h.name == "prize add (0x1401F4AB0)").expect("site");
        hooks.push(hook::Hook { site, ours: drops::prize_add as *const () as usize, original: &drops::ORIG_PRIZE_ADD });
    }
    // SAFETY: the targets' bytes are checked again inside, before anything is written.
    if unsafe { hook::install_all(base, &hooks) } {
        gimmick::mark_installed();
        log(&gimmick::start_line());
    } else {
        log("gimmick: not hooked; gimmick cards stay Re:CoM's at combat = kh1 (held in the hidden revolver, no Stagger)");
    }
}

/// M7 stage 7.11 (`unlocks`, decision 34): KH1's spells learned from Re:CoM's card collection, through
/// a wrapper on Re:CoM's "first obtained" `0x14015C7F0`. Without it, spells come only from the debug
/// switch `kh1_all_spells`.
fn install_unlocks(base: usize) {
    let site = |name: &str| crate::sites::UNLOCK_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [hook::Hook { site: site("card first obtained (0x14015C7F0)"), ours: unlocks::first_obtained as *const () as usize, original: &unlocks::ORIG_FIRST_OBTAINED }];
    // SAFETY: the target's bytes are checked again inside, before anything is written.
    if unsafe { hook::install_all(base, &hooks) } {
        unlocks::mark_installed();
        log(&unlocks::start_line());
    } else {
        log("unlocks: 0x14015C7F0 not hooked; no spell is learned (kh1_all_spells still gives all)");
    }
}

/// M8b (`summons`): KH1's Summon list, each summon Re:CoM's LV3 sleight through Sora's own sleight
/// start (no hook, no patch). Needs the command menu (its Summon list issues them); the learn order
/// follows Re:CoM's card collection through the unlocks wrapper (and the first read of a save).
fn install_summons(base: usize) {
    if !command_menu::installed() {
        log("summons: no KH1 command menu; no KH1 summons");
        return;
    }
    // SAFETY: reads of the mapped exe.
    if unsafe { summons::install(base) } {
        log(&summons::start_line());
        if !unlocks::installed() {
            log("summons: the unlocks wrapper is not in: summons learned only at a save's first read");
        }
        // 0.111.0: the Party list (page 2) starts its friend sleights through the same start.
        // SAFETY: reads of the mapped exe.
        if !unsafe { party::install(base) } {
            log("party: not installed: page 2's Party row stays greyed");
        }
    }
}

/// Sleights outside battle (owner, 2026-11-04, option (a); `field_sleights`): the Field exe's case for
/// action 0x16 and the PLM init wrapper (packs in field rooms). Needs the command menu (its lists and
/// shortcuts start them) and the summons' start (Re:CoM's sleight start through Sora's vtable).
fn install_field_sleights(base: usize) {
    if !command_menu::installed() || !summons::installed() {
        log("field sleights: no KH1 command menu or summons' start; sleights stay battle-only");
        return;
    }
    // SAFETY: reads of the mapped exe; data patches with checked old values; the hook's target
    // bytes are checked again inside, before anything is written.
    if unsafe { field_sleights::install(base) } {
        log(&field_sleights::start_line());
        // The camp shut and no contact battle while a (dimming) field sleight runs.
        // SAFETY: call patches with checked old bytes and checked context.
        unsafe { field_sleights::install_field_rules(base) };
    }
}

/// M6 stages 1 and 2 (`battle`, `deck`, decisions 29 / 30): at Combat = KH1 in Sora's story,
/// Heartless attack without cards, several at once, and Sora has no deck. Needs the movement
/// handlers (they carry the setting).
/// Decision 35: MP to full where Re:CoM fills Sora's HP (two wrappers and one call retarget,
/// `win::mp`). Each part on its own; a missing one only loses that refill.
fn install_mp_refills(base: usize) {
    let site = |name: &str| crate::sites::MP_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [
        hook::Hook { site: site("HP refill state step (0x1401AC9D0)"), ours: mp::heal_state as *const () as usize, original: &mp::ORIG_HEAL_STATE },
        hook::Hook { site: site("level-up screen (0x14010EF10)"), ours: mp::level_up as *const () as usize, original: &mp::ORIG_LEVEL_UP },
        hook::Hook { site: site("save point update (0x1401C5180)"), ours: mp::save_point as *const () as usize, original: &mp::ORIG_SAVE_POINT },
    ];
    // SAFETY: the targets' bytes are checked again inside, before anything is written.
    let w = unsafe { hook::install_all(base, &hooks) };
    // SAFETY: the call's bytes and context are checked inside `patch`.
    let c = unsafe { mp::install_checkpoint(base) };
    log(&format!(
        "mp: refill to full where Re:CoM fills HP (decision 35): state step and level-up wrappers {}, checkpoint call 0x140237F3A {}; KH1's save point refill (1 MP every 3rd frame on a save point) on the save point update 0x1401C5180",
        if w { "in" } else { "NOT in" },
        if c { "retargeted" } else { "NOT retargeted" }
    ));
}

fn install_battles(base: usize) {
    if !movement::installed() {
        log("m6: movement not hooked; battles stay Re:CoM's");
        return;
    }
    if battle::install(base) {
        log("m6: wrappers in (card play 0x140198870, battle enemy manager ctor 0x1401859B0, setPlayCard log 0x140163830); P2-P4 are switched at each battle start by the setting");
    } else {
        log("m6: not hooked; battles stay Re:CoM's (cards and turn-taking)");
    }
    // Re:CoM-AI Heartless keep their own attack-end cooldown in KH1 battles (owner, 2026-11-02).
    let pace = battle::install_pace(base);
    if pace {
        log("m6: attack end 0x14017E540 wrapped: in a KH1 battle, ordinary Heartless on Re:CoM's AI keep the cooldown their class passes (6000 ticks = 120 frames) instead of Re:CoM's 250 (0x14017E5F5); bosses and KH1-AI enemies keep Re:CoM's");
    } else {
        log("m6: attack end not wrapped; Re:CoM's 250-tick cooldown stays for every enemy");
    }
    // Oogie Boogie (owner, 2026-11-03): his dice throw ending unbroken lowers the gate as a break.
    if pace && battle::install_oogie(base) {
        log("m6: Oogie Boogie (CBossBoogie 0x140670A60) checked, his state-13 begin slot +0x220 wrapped (0x1403D7A40 as is, logged): in a KH1 battle each dice throw (state 2) that ends unbroken gets Re:CoM's card break (his handler 0x140181620, message 3 -> state 13), whose begin lowers the gate one step");
    } else {
        log("m6: Oogie Boogie not as read (RTTI, his code or slots) or the attack end not wrapped: he stays Re:CoM's at KH1 (the gate lowers only from a card break or his gimmick card)");
    }
    // The Parasite Cage (owner, 2026-11-03): no flinch from Sora's hits, about 3 s between attacks.
    if battle::install_cage(base) {
        log("m6: Parasite Cage (CBossParasiteCage 0x140669220) checked: in a KH1 battle Sora's hits land on Re:CoM's super armour (+0x260 bits 20 / 21 around the hit handler 0x14017F7E0: no flinch or blow-away, its attacks go on) and it waits 9000 ticks (180 frames) after each attack (the attack-end wrapper)");
    } else {
        log("m6: Parasite Cage not as read (RTTI, its attack ends or slots): it stays Re:CoM's at KH1 (flinches, 250 ticks between attacks)");
    }
    // Stage 2: no deck for Sora (decision 29), on its own: it doesn't need stage 1's hooks.
    if deck::install(base) {
        log("m6: no-deck wrappers in (revolver build 0x1401F43D0, hit drop 0x14017E190); the 16 deck and enemy-card HUD mask reads are switched at each battle start by the setting");
    } else {
        log("m6: no-deck not hooked; Sora keeps Re:CoM's deck in battle");
    }
}

/// M10 / M11 (`kh1_enemy`, decision M10 (a)): KH1's enemies on Re:CoM's own states, one class at
/// a time from `crate::kh1_enemy::CLASSES`. Needs the battle wrappers (the battle's setting), the
/// asset step's manifest and the class's `ENEMY/<id>` files. Per class: the manifest, the vtable
/// checks and the factory size first, then the ctor hook; all or nothing for that class.
fn install_enemies(base: usize, bases: &[PathBuf]) {
    for (k, c) in crate::kh1_enemy::CLASSES.iter().enumerate() {
        if !movement::installed() {
            log(&format!("{}: movement not hooked; Re:CoM's behaviour kept", c.name));
            continue;
        }
        let manifest = bases.iter().map(|b| b.join(c.manifest)).find(|p| p.is_file());
        // SAFETY: reads of the mapped exe; the size patch's bytes are checked inside.
        if !unsafe { kh1_enemy::prepare(base, k, manifest.as_deref()) } {
            continue;
        }
        let hooks = [hook::Hook { site: c.ctor, ours: kh1_enemy::ctor_ptr(k), original: &kh1_enemy::ORIG_CTOR[k] }];
        // SAFETY: the target's bytes are checked again inside, before anything is written.
        if unsafe { hook::install_all(base, &hooks) } {
            kh1_enemy::mark_installed(k);
            log(&kh1_enemy::start_line(k));
        } else {
            // SAFETY: puts back the bytes `prepare` wrote (checked).
            unsafe { kh1_enemy::unprepare(base, k) };
            log(&format!("{}: ctor not hooked; Re:CoM's behaviour kept", c.name));
        }
    }
    // The shared enemy guard: no Re:CoM "Guard" popup over KH1 enemies (owner 2026-11-02).
    // SAFETY: the call's context and bytes are checked inside, before anything is written.
    unsafe { kh1_enemy::install_guard_popup(base) };
    // The KH1 Bouncywild's slip loses nothing (owner 2026-11-03): the damage check's slip scatter call through our stub.
    // SAFETY: the call's context and callee are checked inside, before anything is written.
    unsafe { kh1_peel::install_slip_scatter(base) };
    // The Large Body's stomp ring as KH1's ring: Re:CoM's circle test, KH1's rule for our ring only.
    // SAFETY: the target's bytes are checked again inside, before anything is written.
    unsafe { kh1_enemy::install_ring_test(base) };
}

/// Boss revenge at Combat = KH1 (`revenge`, owner 2026-11-01): KH2's revenge value on Re:CoM's
/// humanoid bosses. Two state-step hooks, the attack-state end hook and the bosses' allocation sizes,
/// all or nothing; the add is in the enemy-hit wrapper. `boss_revenge = off` leaves it all passing through.
fn install_revenge(base: usize, s: &Settings) {
    if !movement::installed() {
        log("revenge: movement not hooked; bosses keep Re:CoM's rules");
        return;
    }
    revenge::set_enabled(s.boss_revenge);
    // SAFETY: reads of the mapped exe; the hook targets' and patch sites' bytes are checked inside.
    if unsafe { revenge::install(base) } {
        log(&format!(
            "revenge: KH2's revenge value on {} humanoid bosses (boss_revenge = {}): counter in a 0x30-byte tail of each boss body (factory sizes raised), add in the enemy-hit wrapper, check in the flinch / blow-away steps 0x140184100 / 0x140182E50 at KH2's frame 20 (Hades 30), or at the reaction's end if sooner (state-change callback 0x1401A3B80 in the 12 classes' tables); the revenge attack untouchable by Re:CoM's own rule (hurt bodies off 0x14017DA80, its handler's bit-0 drop) until its state end 0x14017DA90 (Hades: until his wind-up ends, his state-5 exe +0x168)",
            crate::revenge::BOSSES.len(),
            settings::switch_str(s.boss_revenge)
        ));
        log(&format!(
            "dim: bosses untouchable during the moves that dim the room (any boss_revenge): Re:CoM's scene colour 0x140232170 read at each attack begin (slot 0 against the state change's snapshot; Marluxia 3's own slot 1) and in {} dimming steps (Larxene states 3 / 4, Marluxia 3 states 7 / 9); the same untouchable rule until the state end 0x14017DA90",
            crate::dim::EXES.len()
        ));
    } else {
        log("revenge: not installed; bosses keep Re:CoM's rules (no revenge, Re:CoM's falloff)");
    }
}

/// Story skips at Combat = KH1 (`story_skip`, owner 2026-11-01, option (a)): the intro tutorial and
/// Leon's training battle skipped through their own exit. One wrapper; the setting comes from the
/// movement handlers, so without them it always passes through.
fn install_story_skip(base: usize) {
    let hooks = [hook::Hook { site: &crate::sites::STORY_SKIP_HOOKS[0], ours: story_skip::battle_entry as *const () as usize, original: &story_skip::ORIG_BATTLE_ENTRY }];
    // SAFETY: the target's bytes are checked again inside, before anything is written.
    if unsafe { hook::install_all(base, &hooks) } {
        log(&story_skip::start_line());
    } else {
        log("story skip: 0x140236690 not hooked; the tutorial battles play as in Re:CoM");
    }
}

/// The camp's top list (`keyblades`, owner 2026-11-04): KH1's nine entries at Combat = KH1 in Sora's
/// story, and the Settings list (Game Settings, Combat, Movement) in Sora's story: four camp
/// wrappers, all or nothing. Needs the Config input hook (the pages run through it), so it is
/// installed only after that one. Which list shows is decided each time Re:CoM builds the list.
fn install_keyblades(base: usize, bases: &[PathBuf]) {
    let site = |name: &str| crate::sites::KEYBLADE_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [
        hook::Hook { site: site("camp update (0x1400D9C80)"), ours: keyblades::camp_update as *const () as usize, original: &keyblades::ORIG_UPDATE },
        hook::Hook { site: site("camp entry build (0x1400E3DE0)"), ours: keyblades::entry_build as *const () as usize, original: &keyblades::ORIG_BUILD },
        hook::Hook { site: site("camp cursor set (0x1400DEA10)"), ours: keyblades::cursor_set as *const () as usize, original: &keyblades::ORIG_CURSOR },
        hook::Hook { site: site("camp teardown (0x1400D7C20)"), ours: keyblades::teardown as *const () as usize, original: &keyblades::ORIG_TEARDOWN },
    ];
    // SAFETY: the targets' bytes are checked again inside, before anything is written.
    if unsafe { hook::install_all(base, &hooks) } {
        log("keyblades: camp wrappers in; Sora's story: at Combat = KH1 the top list is Items, Abilities, Customize, World Map .. Settings; Settings opens Game Settings, Combat, Movement, Effects & Sound, Camera (Riku's story: Re:CoM's)");
        keyblades::load_item_row(bases.iter().map(|b| b.join(crate::camp_rows::MANIFEST)).find(|p| p.is_file()).as_deref());
    } else {
        log("keyblades: not hooked; Re:CoM's camp list (no Items / Abilities / Customize / Combat / Movement pages; the Kingdom Key's numbers stand; shortcuts keep KH1's learn order)");
    }
}

/// KH1 damage and HP on Re:CoM's KH1-equivalent enemies (`stats`): needs `kh1-stats.json`.
fn install_stats(base: usize, stats_file: Option<&Path>) {
    if !stats::load(stats_file) {
        log("attack: damage: Re:CoM's own numbers (cardless AP = attack stat): no KH1 stats");
        return;
    }
    // The equipped Keyblade's model (decision 25): its groups are in the stats file (format 4).
    models::install(base);
    let site = |name: &str| crate::sites::STATS_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [
        hook::Hook { site: site("enemy stats init (0x14017FF50)"), ours: stats::stat_init as *const () as usize, original: &stats::ORIG_STAT_INIT },
        hook::Hook { site: site("enemy max HP (0x14017F5A0)"), ours: stats::max_hp as *const () as usize, original: &stats::ORIG_MAX_HP },
    ];
    // SAFETY: the targets' bytes are checked again inside, before anything is written.
    if unsafe { hook::install_all(base, &hooks) } {
        if settings().damage == settings::Mode::Recom {
            log("attack: damage = recom: Re:CoM's HP and AP; the KH1 records give levels, Stop / Gravity resistances, drops and lock radii");
        } else {
            log("attack: damage: KH1's (Sora's KH1 Strength, the twin's Defence, floor, cap and resistance; Re:CoM's weak part and falloff on top); KH1 HP on twins");
        }
    } else {
        log("attack: damage: Re:CoM's own numbers: the stats hooks are not in");
    }
}

fn module_path(m: *mut c_void) -> PathBuf {
    let mut buf = vec![0u16; 1024];
    // SAFETY: the buffer length is passed; a null module names the exe.
    let n = unsafe { GetModuleFileNameW(m, buf.as_mut_ptr(), buf.len() as u32) } as usize;
    PathBuf::from(String::from_utf16_lossy(&buf[..n.min(buf.len())]))
}

fn wide_to_path(p: *const u16) -> Option<PathBuf> {
    if p.is_null() {
        return None;
    }
    // SAFETY: Panacea passes a NUL-terminated wide string (its mod folder).
    let s = unsafe {
        let mut n = 0;
        while *p.add(n) != 0 && n < 4096 {
            n += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(p, n))
    };
    (!s.is_empty()).then(|| PathBuf::from(s))
}

fn run(mod_dir: Option<PathBuf>) {
    // SAFETY: a null name returns the exe's own (mapped) image base.
    let base_ptr = unsafe { GetModuleHandleW(std::ptr::null()) } as *const u8;
    if base_ptr.is_null() {
        return;
    }
    // SAFETY: the first 0x1000 bytes of a mapped PE image are its headers.
    let headers = unsafe { std::slice::from_raw_parts(base_ptr, 0x1000) };
    let pe = crate::sites::pe_info(headers);
    let size = pe.as_ref().map(|p| p.size_of_image).unwrap_or(0);
    let read_site = |rva: u32, n: usize| -> Vec<u8> {
        if (rva as u64 + n as u64) > size as u64 {
            return Vec::new();
        }
        // SAFETY: inside the mapped image (checked against its size); code is readable.
        unsafe { std::slice::from_raw_parts(base_ptr.add(rva as usize), n).to_vec() }
    };
    let exe = module_path(std::ptr::null_mut());
    let game_dir = exe.parent().map(Path::to_path_buf).unwrap_or_default();
    let dll = module_path(MODULE.load(Ordering::Relaxed) as *mut c_void);
    // The DLL sits in mod_path\Recom\dll: its parent's parent is the mod folder.
    let mod_dir = mod_dir.or_else(|| dll.parent().and_then(Path::parent).map(Path::to_path_buf)).unwrap_or_else(|| game_dir.clone());
    let ini_path = game_dir.join("kh1-recom.ini");
    let mut migrated: Vec<String> = Vec::new();
    let ini = match std::fs::read_to_string(&ini_path) {
        Ok(t) => match settings::migrate(&t) {
            Some((n, notes)) => {
                let _ = std::fs::write(&ini_path, &n);
                migrated = notes;
                Ok(n)
            }
            None => Ok(t),
        },
        Err(_) => {
            let _ = std::fs::write(&ini_path, settings::DEFAULT_FILE);
            Err(format!("{} was missing and has been written with the defaults", ini_path.display()))
        }
    };
    let s = match &ini {
        Ok(t) => settings::parse(t).0,
        Err(_) => Settings::default(),
    };
    let (report, ok) = crate::startup_report(&exe.to_string_lossy(), pe, &read_site, ini);
    let log_path = game_dir.join("kh1-recom.log");
    let _ = std::fs::write(&log_path, &report);
    if let Ok(mut g) = LOG_PATH.lock() {
        *g = Some(log_path);
    }
    // 0.198.18: development builds (feature `dev`, `tools/dev-install.sh`) say so; a released DLL has no such line.
    #[cfg(feature = "dev")]
    log("DEV BUILD (feature dev): development-only debug aids are on (the all_unlock key, which also shows Riku's Dark Mode shortcut rows); never released");
    if !migrated.is_empty() {
        log(&format!("settings: {} updated: {}", ini_path.display(), migrated.join("; ")));
    }
    if let Ok(mut g) = INI_PATH.lock() {
        *g = Some(ini_path);
    }
    if let Ok(mut g) = SETTINGS.lock() {
        *g = Some(s.clone());
    }
    log(&format!("dll: {}", dll.display()));
    log(&format!("mod folder: {}", mod_dir.display()));
    if !ok {
        return;
    }
    let base = base_ptr as usize;
    BASE.store(base, Ordering::Relaxed);
    crash::install(MODULE.load(Ordering::Relaxed));

    // The game-made files (owner, 2026-11-05: "DLL builds on first launch"): copied into Panacea's
    // folder before anything below reads them and before the game lists that folder; when they
    // must be built, the rest of the start waits for the build at Re:CoM's title, which shows its
    // progress (owner, 2026-11-05: "a progress screen in the game window while everything loads").
    let rest = Rest { mod_dir: mod_dir.clone(), dll: dll.clone(), game_dir: game_dir.clone(), settings: s.clone() };
    match assets::ensure(&game_dir, &mod_dir, &dll, s.asset_build) {
        assets::Ready::Now => install_rest(&rest),
        assets::Ready::AtTitle => {
            if let Ok(mut g) = REST.lock() {
                *g = Some(rest);
            }
            log("start: the rest waits for the KH1 files' build (at the title)");
        }
    }
}

/// What the start's second half needs (`install_rest`).
struct Rest {
    mod_dir: PathBuf,
    dll: PathBuf,
    game_dir: PathBuf,
    settings: Settings,
}

/// The start's second half, when it waits for the build (`assets`).
static REST: Mutex<Option<Rest>> = Mutex::new(None);

/// The start's second half after the build (at Re:CoM's title, on the game's thread; once).
pub(super) fn install_deferred() {
    let rest = REST.lock().ok().and_then(|mut g| g.take());
    if let Some(r) = rest {
        log("start: the KH1 files' build ended: the rest of the start runs now (at the title)");
        install_rest(&r);
    }
}

/// The start's second half: everything that reads the built files.
fn install_rest(r: &Rest) {
    let (mod_dir, dll, game_dir, s) = (&r.mod_dir, &r.dll, &r.game_dir, &r.settings);
    let base = BASE.load(Ordering::Relaxed);
    // The camp menu's "KH1" entry: its submenu and pages run in the camp's Config states through
    // the Config input wrapper (Re:CoM's own Config screen goes to the original, unchanged).
    if s.config_menu {
        let hooks = [hook::Hook { site: &crate::sites::HOOKS[0], ours: menu::cfg_input as *const () as usize, original: &menu::ORIG_INPUT }];
        // SAFETY: the target's bytes are checked again inside, before anything is written.
        if unsafe { hook::install_all(base, &hooks) } {
            log("camp menu: Config input wrapped for the KH1 entry's pages (Re:CoM's Config screen and its rows as unmodded)");
            install_keyblades(base, &recom_bases(&manifest_dirs(mod_dir, dll, game_dir)));
        } else {
            log("camp menu: Config input not hooked; no KH1 entry (settings from kh1-recom.ini only)");
        }
    } else {
        log("camp menu: config_menu = off in the ini; Re:CoM's camp menu untouched (no KH1 entry; settings from kh1-recom.ini only)");
    }

    let dirs = manifest_dirs(mod_dir, dll, game_dir);
    // M8d: the shop's five-tab art (read before the shop's wrappers can run).
    shop::load_tabs(recom_bases(&dirs).iter().map(|d| d.join(crate::shop::tabs::MANIFEST)).find(|p| p.is_file()).as_deref());
    install_movement(base, s, &dirs, game_dir);
    // M12 camera: independent of Combat (owner); Re:CoM's camera untouched at Camera = Re:CoM.
    camera::install(base, s);
}

/// Where the asset step's files are looked for: the mod folder, the DLL's folder and its parent,
/// the game folder (each also with its `recom` subfolder, `recom_bases`).
fn manifest_dirs(mod_dir: &Path, dll: &Path, game_dir: &Path) -> [PathBuf; 4] {
    let dll_dir = dll.parent().map(Path::to_path_buf).unwrap_or_default();
    [mod_dir.to_path_buf(), dll_dir.parent().map(Path::to_path_buf).unwrap_or_default(), dll_dir, game_dir.to_path_buf()]
}

fn start(mod_dir: Option<PathBuf>) {
    START.call_once(|| run(mod_dir));
}

/// Panacea calls this once after loading every DLL in `mod_path\Recom\dll`, with the mod folder.
///
/// # Safety
/// `folder` is null or a NUL-terminated wide string.
#[no_mangle]
pub unsafe extern "C" fn OnInit(folder: *const u16) {
    start(wide_to_path(folder));
}

/// # Safety
/// Called by the Windows loader.
#[no_mangle]
pub unsafe extern "system" fn DllMain(module: *mut c_void, reason: u32, _reserved: *mut c_void) -> i32 {
    const DLL_PROCESS_ATTACH: u32 = 1;
    if reason == DLL_PROCESS_ATTACH {
        MODULE.store(module as usize, Ordering::Relaxed);
        // The first-launch build's helper (`rundll32 … BuildAssets`): never the game's start-up.
        let host = module_path(std::ptr::null_mut());
        if host.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("rundll32.exe")) {
            return 1;
        }
        // If nothing calls OnInit (loaded some other way), start anyway after a moment.
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_secs(3));
            start(None);
        });
    }
    1
}
