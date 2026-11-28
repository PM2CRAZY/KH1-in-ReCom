//! Riku's Dark Form skin in the game (rules and addresses: `crate::riku_skin`).
//!
//! - [`preload`] (before the Keyblade model table): reads `kh1-riku-skin.json` and keeps the skins
//!   whose files are all on disk next to it.
//! - The model table (`win::models::install`) adds their rows ([`skins`]) and says so
//!   ([`rows_in`]).
//! - [`apply`] (at start, and from the Costumes page): Re:CoM's char → group entries of chars 3 / 4
//!   (`0x1405A6910 + 12`) and their motion pack names (`0x1407C6338 + 24`) point at the skin
//!   groups (Dark Mode) or back at Riku's own (Default Attire), each write checked against what is
//!   there. Re:CoM reads them when it builds a player or loads its group: the next room or battle.
//! - 0.198.30: with a hair that has a group for the battle's Dark Riku (the manifest's `dark_mode`), the entries of chars 5 / 6
//!   (`0x1405A6910 + 20`, `0x1407C6338 + 40`) the same way (Re:CoM's Dark Mode: `0x140211960` preloads `[5 / 6]` for chars 3 / 4,
//!   the factory `0x14020EC70` builds `CPlayerDRiku` on it, its rows bind in the pack named `[5 / 6]`; Checked (decomp)).

use super::{log, va};
use crate::riku_skin::{self as rs, Look, Skin};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

static SKINS: OnceLock<Vec<Skin>> = OnceLock::new();
/// 0.197.4: (Re:CoM's event pack group, its 109-bone copy) whose copy file the asset step wrote (`rs::event_packs`).
static EVENT_PACKS: OnceLock<Vec<(u16, u16)>> = OnceLock::new();
/// 0.198.32: (Re:CoM's pack the battle's Dark Riku plays, its 139-bone copy for KH2's hair) whose copy file the asset step wrote
/// (`rs::dark_packs`).
static DARK_PACKS: OnceLock<Vec<(u16, u16)>> = OnceLock::new();
/// Riku's packs carry Sora's KH1 Guard (manifest + files), and his action tables name it.
static GUARD: AtomicBool = AtomicBool::new(false);
static GUARD_FILES: AtomicBool = AtomicBool::new(false);
/// 0.183.2: Dark Riku's own packs (`PL0201` / `PL0211.ANN`) carry the guard too, and his action table names it.
static DARK_GUARD: AtomicBool = AtomicBool::new(false);
static DARK_GUARD_FILES: AtomicBool = AtomicBool::new(false);
/// 0.184.1: the packs carry the cast clips, and the tables name them (Riku's / Dark Riku's).
static CAST_FILES: AtomicBool = AtomicBool::new(false);
static CAST: AtomicBool = AtomicBool::new(false);
static DARK_CAST: AtomicBool = AtomicBool::new(false);
/// The PLAYER table copy holds the skin groups' rows.
static ROWS: AtomicBool = AtomicBool::new(false);
/// The look the tables point at now (`Look::bits`; 0 Riku's own).
static ON: AtomicU8 = AtomicU8::new(0);
/// The look the menu's model bytes point at now.
static MENU_ON: AtomicU8 = AtomicU8::new(0);
/// Our motion pack names, per skin group (kept for the process: Re:CoM reads the pointers).
static NAMES: OnceLock<Vec<(u16, usize)>> = OnceLock::new();
/// 0.198.30: the look (`Look::bits`; 0.198.36: the hair and the blade) the Dark Mode entries (chars 5 / 6) point at now (0: Dark
/// Riku's own groups).
static DARK_ON: AtomicU8 = AtomicU8::new(0);
/// Re:CoM's own name pointers of chars 5 / 6 ("PL0201" / "PL0211"), read before our first write there; None when they are not those.
static DARK_NAMES: OnceLock<Option<[u64; 2]>> = OnceLock::new();

fn look() -> Look {
    Look::from_bits(ON.load(Ordering::Relaxed))
}

/// The look the settings ask for, each part only once unlocked.
///
/// # Safety
/// Reads the save (main thread).
pub unsafe fn chosen(s: &crate::settings::Settings) -> Look {
    Look {
        dark: s.riku_costume == 1 && unlocked(),
        blade: (s.riku_heart_blade && blade_unlocked()) as u8,
        halloween: s.riku_costume == 2 && halloween_unlocked(),
        hair: if hair_unlocked(s.riku_hair.min(2)) { s.riku_hair.min(2) } else { 0 },
    }
}

/// A hair is earned in this save (0.198.33, `crate::riku_unlocks::hair_unlocked`: BBS at Vexen's win, KH2 at Riku Replica I's),
/// or the debug `all_unlock` (development builds only). Re:CoM's own hair always.
///
/// # Safety
/// Reads Re:CoM's story variable (main thread).
pub unsafe fn hair_unlocked(hair: u8) -> bool {
    hair == 0 || super::settings().all_unlock || super::unlocks::riku_hall_bits().is_some_and(|b| crate::riku_unlocks::hair_unlocked(hair, b))
}

/// The save block's spare bit for "Riku reached Halloween Town" (0.178.0; Riku's own save: bits 1 /
/// 2 / 4 taken, Sora's 0x10 in his).
pub const HALLOWEEN_SEEN: u8 = 0x20;

/// The Halloween Town costume is unlocked: Riku has been there (char 4) in this save, or the debug
/// `all_unlock`.
pub unsafe fn halloween_unlocked() -> bool {
    if super::settings().all_unlock {
        return true;
    }
    let save: usize = rd(va(super::base(), crate::keyblades::SAVE_PTR));
    save != 0 && super::stats::riku_story() && rd::<u8>(save + crate::mp::BLOCK_FIRST + crate::mp::off::SPARE) & HALLOWEEN_SEEN != 0
}

/// A costume of the Costume row can be chosen now.
pub unsafe fn costume_open(v: usize) -> bool {
    match v {
        1 => unlocked(),
        2 => halloween_unlocked(),
        _ => true,
    }
}

/// Riku as char 4 (Halloween Town): kept in his save.
unsafe fn mark_halloween(e: usize) {
    let save: usize = rd(va(super::base(), crate::keyblades::SAVE_PTR));
    if save == 0 || !super::stats::riku_story() || rd::<u32>(e + CHAR) != 4 {
        return;
    }
    let at = save + crate::mp::BLOCK_FIRST + crate::mp::off::SPARE;
    let b: u8 = rd(at);
    if b & HALLOWEEN_SEEN == 0 {
        wr(at, b | HALLOWEEN_SEEN);
        log("riku skin: Halloween Town reached: kept in Riku's save (the Halloween Town costume unlocks)");
    }
}

/// The Keyblade of Heart is unlocked with the Dark Mode costume (0.167.0; fork owner, 2026-11-23: "a
/// keyblade cosmética dele pode liberar junto com o Dark Mode também"): Riku's first Dark Mode in
/// this save, or the debug keys.
pub unsafe fn blade_unlocked() -> bool {
    super::settings().all_unlock || unlocked()
}

/// Reads the manifest and checks the skin groups' files.
pub fn preload(manifest: Option<&Path>) {
    let Some(path) = manifest else {
        log("riku skin: no kh1-riku-skin.json (asset step not run?); Riku keeps his model");
        return;
    };
    let skins = match std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display())).and_then(|t| rs::parse(&t)) {
        Ok(s) => s,
        Err(e) => {
            log(&format!("riku skin: {e}; Riku keeps his model"));
            return;
        }
    };
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let guard_files = rs::riku_guard(&text) && ["PLAYER/0101/PL0101.ANN", "PLAYER/0111/PL0111.ANN"].iter().all(|f| f.split('/').fold(dir.clone(), |p, x| p.join(x)).is_file());
    GUARD_FILES.store(guard_files, Ordering::Relaxed);
    // 0.183.2: the asset step writes Dark Riku's own packs with the guard (`kh1_recom_assets::riku_skin::dark_guard_pack`).
    let dark_files = guard_files && ["PLAYER/0201/PL0201.ANN", "PLAYER/0211/PL0211.ANN"].iter().all(|f| f.split('/').fold(dir.clone(), |p, x| p.join(x)).is_file());
    DARK_GUARD_FILES.store(dark_files, Ordering::Relaxed);
    let all_packs = ["PLAYER/0101/PL0101.ANN", "PLAYER/0111/PL0111.ANN", "PLAYER/0201/PL0201.ANN", "PLAYER/0211/PL0211.ANN"].iter().all(|f| f.split('/').fold(dir.clone(), |p, x| p.join(x)).is_file());
    CAST_FILES.store(rs::riku_cast(&text) && all_packs, Ordering::Relaxed);
    if !CAST_FILES.load(Ordering::Relaxed) {
        log("riku cast: no cast clips from the asset step (older than 0.184.1?): Riku's spells keep his current motion; run the game once for the asset step");
    }
    if guard_files && !dark_files {
        log("riku guard: no PL0201 / PL0211.ANN of the asset step (older than 0.183.2?): Dark Riku's Guard keeps his clash recoil; run Build again");
    }
    // 0.197.4: the event pack copies the step wrote
    let packs: Vec<(u16, u16)> = rs::event_packs(&text).into_iter().filter(|p| crate::keyblade_models::resource_path(p.1, "ANN").split('/').fold(dir.clone(), |d, x| d.join(x)).is_file()).collect();
    if !packs.is_empty() {
        log(&format!("riku skin: event pack copies for the KH2 hair {packs:?}"));
    }
    let _ = EVENT_PACKS.set(packs);
    // 0.198.32: the battle Dark Riku's pack copies for KH2's hair; without all eleven his KH2 group is left out (`rs::dark_mode_usable`)
    let dpacks: Vec<(u16, u16)> = rs::dark_packs(&text).into_iter().filter(|p| crate::keyblade_models::resource_path(p.1, "ANN").split('/').fold(dir.clone(), |d, x| d.join(x)).is_file()).collect();
    if !dpacks.is_empty() {
        log(&format!("riku skin: the battle Dark Riku's pack copies for the KH2 hair {dpacks:?}"));
    }
    let dm = rs::dark_mode(&text);
    let usable = rs::dark_mode_usable(dm.clone(), &dpacks);
    if usable.len() != dm.len() {
        log(&format!("riku skin: only {} of the battle Dark Riku's {} pack copies on disk: his KH2 hair group is left out (Re:CoM's hair with it); run Build again", dpacks.len(), rs::DARK_PACK_SOURCES.len()));
    }
    let _ = DARK_PACKS.set(dpacks);
    let mut ok = Vec::new();
    // 0.198.30: the battle Dark Riku's hair groups (`rs::dark_mode`), checked as the skins are
    let skins: Vec<Skin> = skins.into_iter().chain(usable).collect();
    for s in skins {
        let missing: Vec<String> = rs::files(s.skin).into_iter().filter(|f| !f.split('/').fold(dir.clone(), |p, x| p.join(x)).is_file()).collect();
        if missing.is_empty() {
            ok.push(s);
        } else {
            log(&format!("riku skin: group {} files not found {missing:?}; group {} keeps Riku's model", s.skin, s.riku));
        }
    }
    let _ = SKINS.set(ok);
}

/// Riku's Guard row: his row 0x39 (Sora's KH1 Guard on his skeleton) once [`guard_rows`] named it,
/// else his clash recoil 0x24; Dark Riku's (the real Dark Mode) his row 0x39 (`p003_225`, 0.183.2) the same
/// way, else his own 0x24.
pub fn guard_row(dark: bool) -> u32 {
    if (if dark { &DARK_GUARD } else { &GUARD }).load(Ordering::Relaxed) {
        crate::riku::KH1_GUARD_ROW
    } else {
        crate::riku::GUARD_ROW
    }
}

/// Row 0x39 of Riku's two action tables (`0x1407C6300[3]`, `[4]`) named `p002_225` (its name
/// pointer, checked to be Re:CoM's `"p002_000"` first); Re:CoM's rebind by name (`0x1401F4240`,
/// at every player build) binds it to the clip. Only with the asset step's guard files.
/// 0.183.2: Dark Riku's table (`[5]`, `[6]` = `0x1407C67F0`, rows 0x39 / 0x3A `"p003_000"`; Checked (exe))
/// the same with `p003_225` / `p003_226` (Assumption: his player build rebinds by name as Riku's does).
///
/// # Safety
/// `b` is the running image's base; at start, before any player is built.
pub unsafe fn guard_rows(b: usize) {
    if GUARD_FILES.load(Ordering::Relaxed) && !GUARD.load(Ordering::Relaxed) {
        let names: [(u32, &'static [u8]); 2] = [(crate::riku::KH1_GUARD_ROW, crate::riku::KH1_GUARD_CLIP), (crate::riku::KH1_RECOIL_ROW, crate::riku::KH1_RECOIL_CLIP)];
        if name_rows(b, &[3, 4], b"p002_000\0", &names, "Riku") {
            GUARD.store(true, Ordering::Relaxed);
            log("riku guard: Riku's Guard plays his guard clip (row 0x39, p002_225: Terra's Block when the asset step read BBS FM, else KH1 Riku's own guard) and his guard recoil (row 0x3A, p002_226)");
        }
    }
    if CAST_FILES.load(Ordering::Relaxed) && !CAST.load(Ordering::Relaxed) {
        let names: Vec<(u32, &'static [u8])> = crate::riku::CAST_ROWS.iter().copied().zip(crate::riku::CAST_CLIPS).collect();
        if name_rows(b, &[3, 4], b"p002_000\0", &names, "Riku") {
            CAST.store(true, Ordering::Relaxed);
            log("riku cast: Riku's spells play the Dark Firaga cast clips (rows 0x3B..0x40, p002_227..232)");
        }
        let names: Vec<(u32, &'static [u8])> = crate::riku::CAST_ROWS.iter().copied().zip(crate::riku::DARK_CAST_CLIPS).collect();
        if name_rows(b, &[5, 6], b"p003_000\0", &names, "Dark Riku") {
            DARK_CAST.store(true, Ordering::Relaxed);
            log("riku cast: Dark Riku's spells play his Dark Firaga cast clips (rows 0x3B..0x40, p003_227..232)");
        }
    }
    if DARK_GUARD_FILES.load(Ordering::Relaxed) && !DARK_GUARD.load(Ordering::Relaxed) {
        let names: [(u32, &'static [u8]); 2] = [(crate::riku::KH1_GUARD_ROW, crate::riku::DARK_GUARD_CLIP), (crate::riku::KH1_RECOIL_ROW, crate::riku::DARK_RECOIL_CLIP)];
        if name_rows(b, &[5, 6], b"p003_000\0", &names, "Dark Riku") {
            DARK_GUARD.store(true, Ordering::Relaxed);
            log("riku guard: Dark Riku's Guard plays KH1 Riku's guard on his skeleton (row 0x39, p003_225) and its recoil (row 0x3A, p003_226)");
        }
    }
}

/// The rows of the chars' action tables (each table once: chars that share one) renamed to the clips, each
/// checked to be `empty` first. True when every row of every table was named.
unsafe fn name_rows(b: usize, chars: &[usize], empty: &[u8; 9], names: &[(u32, &'static [u8])], who: &str) -> bool {
    // Chars 3 and 4 share one table (Riku's and his Halloween Town form: 0.157.0 named its row,
    // then refused it as "not p002_000" for char 4): each table once.
    let mut tables: Vec<usize> = chars.iter().map(|ch| rd::<usize>(va(b, rs::ACTION_TABLES_PTR) + 8 * ch)).filter(|&t| t != 0).collect();
    tables.dedup();
    let want = tables.len();
    let mut done = 0;
    // Every row is checked before any is renamed, so a refusal leaves the tables as Re:CoM's.
    for &table in &tables {
        for &(row, _) in names {
            let old: usize = rd(table + 16 * row as usize);
            let s = if old == 0 { &[][..] } else { std::slice::from_raw_parts(old as *const u8, 9) };
            if s != empty {
                log(&format!("riku guard: {who}'s action table (chars {chars:?}) row {row:#x} is not {}; {who}'s Guard keeps the clash recoil", String::from_utf8_lossy(&empty[..8])));
                return false;
            }
        }
    }
    for table in tables {
        for &(row, name) in names {
            let at = table + 16 * row as usize;
            let old: usize = rd(at);
            if super::hook::patch(at, &old.to_le_bytes(), &(name.as_ptr() as usize).to_le_bytes(), "Riku's guard row name") {
                done += 1;
            }
        }
    }
    if done != names.len() * want {
        log(&format!("riku guard: {who}'s rows: {done} of {} renamed (a patch failed); {who}'s Guard keeps the clash recoil", names.len() * want));
    }
    done == names.len() * want && want > 0
}

/// 0.184.1: Riku's / Dark Riku's row for a KH1 cast request once the tables name the cast clips, else None.
pub fn cast_row(dark: bool, request: u16) -> Option<u32> {
    (if dark { &DARK_CAST } else { &CAST }).load(Ordering::Relaxed).then(|| crate::riku::cast_row(request)).flatten()
}

/// Riku's row for KH1's guard recoils (0x6E / 0x6F): his KH1 recoil (0x3A) once named, else None
/// (the rows' own rule: his clash recoil 0x24).
pub fn recoil_row(dark: bool) -> Option<u32> {
    (if dark { &DARK_GUARD } else { &GUARD }).load(Ordering::Relaxed).then_some(crate::riku::KH1_RECOIL_ROW)
}

/// The skins whose rows go in the PLAYER table.
/// The event pack copies (none before the manifest is read or without them).
pub fn event_packs() -> &'static [(u16, u16)] {
    EVENT_PACKS.get().map_or(&[], |v| v.as_slice())
}

/// 0.198.32: the battle Dark Riku's pack copies (none before the manifest is read or without them).
pub fn dark_packs() -> &'static [(u16, u16)] {
    DARK_PACKS.get().map_or(&[], |v| v.as_slice())
}

/// 0.198.32: chars 5 / 6 point at the battle Dark Riku's KH2 hair groups (139 bones: his 128-bone packs need their copies).
pub fn dark_kh2_hair() -> bool {
    let sk = skins();
    // 0.198.36: any blade (216 / 217 share the copies); the group the entries point at, not the hair asked (a fallback has none)
    rs::dark_entries(&sk, Look::from_bits(DARK_ON.load(Ordering::Relaxed))).iter().any(|&g| sk.iter().any(|s| s.skin as u32 == g && s.hair == 1))
}

/// The look asks for the KH2 hair (109 bones, so Re:CoM's 102-bone event packs need their copies).
pub unsafe fn kh2_hair_look() -> bool {
    let l = chosen(&super::settings());
    l.hair == 1 && !l.dark
}

pub fn skins() -> Vec<Skin> {
    SKINS.get().cloned().unwrap_or_default()
}

/// The PLAYER table copy with the skins' rows is in.
pub fn rows_in() {
    ROWS.store(true, Ordering::Release);
}

/// The motion pack name pointers for chars 3 / 4 (Riku's or the skins').
unsafe fn name_ptrs(b: usize, sk: &[Skin], look: Look) -> [u64; 2] {
    let names = our_names(sk);
    let mut out = [0u64; 2];
    for (k, g) in rs::RIKU_GROUPS.iter().enumerate() {
        let recom = va(b, rs::RIKU_ANN_NAMES[k]) as u64;
        let want = rs::group_for_char(sk, *g, look);
        // Riku's own groups take Re:CoM's own names (char 3 on 111: "PL0111").
        out[k] = match rs::RIKU_GROUPS.iter().position(|r| *r == want) {
            Some(i) => va(b, rs::RIKU_ANN_NAMES[i]) as u64,
            None => names.iter().find(|n| n.0 == want).map_or(recom, |n| n.1 as u64),
        };
    }
    out
}

/// Our pack names (made once, for every skin group of the manifest).
fn our_names(sk: &[Skin]) -> &'static [(u16, usize)] {
    NAMES.get_or_init(|| {
        sk.iter()
            .map(|s| {
                let c = std::ffi::CString::new(rs::ann_name(s.skin)).unwrap_or_default();
                (s.skin, Box::leak(c.into_boxed_c_str()).as_ptr() as usize)
            })
            .collect()
    })
}

/// Re:CoM's name pointers of chars 5 / 6, read once (before our first write there) and checked to be "PL0201" / "PL0211".
unsafe fn dark_recom_names(b: usize) -> Option<[u64; 2]> {
    *DARK_NAMES.get_or_init(|| {
        let at = va(b, rs::ANN_NAME_TABLE) + 8 * 5;
        let p: [u64; 2] = [rd(at), rd(at + 8)];
        let ok = p.iter().zip(rs::DARK_ANN_NAMES).all(|(&q, want)| q != 0 && std::slice::from_raw_parts(q as *const u8, 7) == want);
        ok.then_some(p)
    })
}

/// 0.198.30 (fork owner, 2026-11-27: "reparei que quando o personagem se transforma no DarkMode o cabelo é resetado para o Default"):
/// chars 5 / 6 (the battle's Dark Riku) at the groups of `look`'s hair and blade ([`rs::dark_entries`]; 0.198.36 (fork owner,
/// 2026-11-27: "no darkmode a keyblade tbm nn troca, já coloca essas regrs para o futuro"): the blade too), their pack names with them;
/// each write checked against what we left there (Re:CoM's 201 / 211 and "PL0201" / "PL0211" the first time). Both or neither, as for
/// Riku's.
unsafe fn apply_dark(b: usize, sk: &[Skin], look: Look) {
    let on = rs::dark_look(look);
    let old = Look::from_bits(DARK_ON.load(Ordering::Relaxed));
    let (now, want) = (rs::dark_entries(sk, old), rs::dark_entries(sk, on));
    let missing = rs::dark_missing(sk, on);
    if !missing.is_empty() && rs::dark_look(old) != on {
        log(&format!(
            "riku skin: no group of the battle's Dark Riku for hair {} with the {} (groups {missing:?}; run Build again): {}",
            on.hair,
            rs::BLADES[on.blade as usize % rs::BLADES.len()],
            if on.blade != 0 { "the Keyblade kept on Re:CoM's hair where it has one, else his own look" } else { "his own look" }
        ));
    }
    if now == want {
        DARK_ON.store(on.bits(), Ordering::Relaxed);
        return;
    }
    let Some(recom) = dark_recom_names(b) else {
        log("riku skin: Re:CoM's motion pack names for Dark Riku are not PL0201 / PL0211 there; the battle's Dark Riku keeps his hair");
        return;
    };
    let names = our_names(sk);
    let ptrs = |e: [u32; 2]| -> Option<Vec<u8>> {
        let mut v = Vec::new();
        for (k, g) in e.iter().enumerate() {
            let p = if *g == rs::DARK_GROUPS[k] as u32 { recom[k] } else { names.iter().find(|n| n.0 as u32 == *g)?.1 as u64 };
            v.extend(p.to_le_bytes());
        }
        Some(v)
    };
    let (Some(n_now), Some(n_want)) = (ptrs(now), ptrs(want)) else { return };
    let groups = |e: [u32; 2]| -> Vec<u8> { e.iter().flat_map(|x| x.to_le_bytes()).collect() };
    let g_at = va(b, rs::GROUP_TABLE) + 4 * 5;
    let n_at = va(b, rs::ANN_NAME_TABLE) + 8 * 5;
    if !super::hook::patch(n_at, &n_now, &n_want, "char -> motion pack name (Dark Riku)") {
        log("riku skin: Re:CoM's motion pack names for Dark Riku are not what we left there; the battle's Dark Riku keeps his hair");
        return;
    }
    if !super::hook::patch(g_at, &groups(now), &groups(want), "char -> group table (Dark Riku)") {
        super::hook::patch(n_at, &n_want, &n_now, "char -> motion pack name (Dark Riku, back)");
        log("riku skin: Re:CoM's char -> group table is not 201 / 211 there; the battle's Dark Riku keeps his hair");
        return;
    }
    DARK_ON.store(on.bits(), Ordering::Relaxed);
    log(&format!("riku skin: the battle's Dark Riku (chars 5 / 6) -> groups {want:?}, motion packs {:?}; from the next battle", want.map(|g| rs::ann_name(g as u16))));
}

fn look_name(l: Look) -> String {
    format!("{}{} with the {}", if l.dark { "Dark Mode" } else { "Default Attire" }, if l.halloween { " (Halloween Town)" } else { "" }, rs::BLADES[l.blade as usize % rs::BLADES.len()])
}

/// The menu's model (camp and level-up screen) follows the group the player loads: called when
/// Re:CoM queues the player's group (room and battle builds), so the menu never asks for a group
/// that isn't loaded (a costume changed in the camp shows at the next room, in the field and in
/// the menu alike).
///
/// # Safety
/// `b` is the running image's base.
pub unsafe fn sync_menu(b: usize) {
    let on = ON.load(Ordering::Relaxed);
    let menu = MENU_ON.load(Ordering::Relaxed);
    if menu == on || !ROWS.load(Ordering::Acquire) {
        return;
    }
    let sk = skins();
    for &(at, riku) in &rs::MENU_BYTES {
        let (from, to) = (rs::group_for_char(&sk, riku, Look::from_bits(menu)), rs::group_for_char(&sk, riku, Look::from_bits(on)));
        if !super::hook::patch(va(b, at), &[from as u8], &[to as u8], "menu model group (Riku)") {
            log("riku skin: the menu model's group bytes are not what we wrote; the menu keeps its model");
            return;
        }
    }
    MENU_ON.store(on, Ordering::Relaxed);
    log(&format!("riku skin: the menu's model (camp, level-up) shows {}", look_name(Look::from_bits(on))));
}

/// The Dark Mode costume is unlocked: Riku has reached Dark Mode once in this save (the save
/// block's spare bit [`super::dark_points::DARK_SEEN`], 0.151.0; fork owner, 2026-11-22: "depois a
/// gente bota o requisito do jogo pra liberar ela").
pub unsafe fn unlocked() -> bool {
    // Debug only: `all_unlock = on` in the ini (0.178.5; was `riku_dark_skin_unlocked`).
    super::settings().all_unlock || super::dark_points::dark_seen()
}

/// Riku's mover's ground speed stopped (0.156.1; fork owner: "quando vc está andando e troca a
/// skin a animação fica tocando enquanto o personagem está se movimentando"): his move exe is
/// held, but Re:CoM's mover step keeps the last velocity (`mover+0x80`). The vertical part stays
/// Re:CoM's (gravity).
unsafe fn stop(e: usize) {
    let m: usize = rd(e + super::movement::p::BODY);
    if m != 0 {
        let v: [f32; 4] = rd(m + super::movement::body::VEL);
        wr::<[f32; 4]>(m + super::movement::body::VEL, [0.0, v[1], 0.0, v[3]]);
    }
}

/// The costume as it should be now: the setting, if unlocked. Called when Re:CoM queues the
/// player's group (each room / battle build, a save loaded by then), before its queue reads the
/// table. A Dark Mode setting on a save that hasn't reached Dark Mode goes back to Default Attire
/// (the ini rewritten, logged).
///
/// # Safety
/// `b` is the running image's base; Re:CoM's room / battle build (main thread).
pub unsafe fn refresh(b: usize) {
    if skins().is_empty() || !ROWS.load(Ordering::Acquire) || !super::stats::riku_story() {
        return;
    }
    let s = super::settings();
    if (s.riku_costume == 1 && !unlocked()) || (s.riku_costume == 2 && !halloween_unlocked()) {
        log("riku skin: a costume not unlocked in this save (Dark Mode: his first Dark Mode; Halloween Town: reaching it): Default Attire");
        // Only the setting (no live change: the old player may be gone during a room build).
        let _ = super::store_item(crate::menu::Item::Costume, 0);
    }
    if !hair_unlocked(s.riku_hair.min(2)) {
        log(&format!("riku skin: the {} hair chosen, but this save hasn't earned it yet (BBS: Vexen's win; KH2: Riku Replica I's): Re:CoM's hair", if s.riku_hair == 1 { "KH2" } else { "BBS" }));
        let _ = super::store_item(crate::menu::Item::Hair, 0);
    }
    if s.riku_heart_blade && !blade_unlocked() {
        log("riku skin: the Keyblade of Heart chosen, but this save hasn't reached Dark Mode yet: the Soul Eater");
        let _ = super::store_item(crate::menu::Item::Blade, 0);
    }
    apply(b, chosen(&super::settings()));
}

/// Points chars 3 / 4 at the groups of a look (Riku's own for the Default Attire with the Soul
/// Eater).
///
/// # Safety
/// `b` is the running image's base; called on start-up or from the camp (no player being built).
pub unsafe fn apply(b: usize, on: Look) {
    let sk = skins();
    if sk.is_empty() || !ROWS.load(Ordering::Acquire) {
        if on != Look::default() {
            log("riku skin: a costume / Keyblade asked, but the skin's files or rows are not in; Riku keeps his model");
        }
        return;
    }
    apply_dark(b, &sk, on);
    let now = look();
    if now == on {
        return;
    }
    let groups = |o: Look| -> Vec<u8> { rs::entries(&sk, o).iter().flat_map(|x| x.to_le_bytes()).collect() };
    let names = |o: Look| -> Vec<u8> { name_ptrs(b, &sk, o).iter().flat_map(|x| x.to_le_bytes()).collect() };
    let g_at = va(b, rs::GROUP_TABLE) + 12;
    let n_at = va(b, rs::ANN_NAME_TABLE) + 24;
    // Both or neither: the names first (checked), then the groups; a failed group write puts the
    // names back.
    if !super::hook::patch(n_at, &names(now), &names(on), "char -> motion pack name (Riku)") {
        log("riku skin: Re:CoM's motion pack names for Riku are not PL0101 / PL0111 there; Riku keeps his model");
        return;
    }
    if !super::hook::patch(g_at, &groups(now), &groups(on), "char -> group table (Riku)") {
        super::hook::patch(n_at, &names(on), &names(now), "char -> motion pack name (Riku, back)");
        log("riku skin: Re:CoM's char -> group table is not 101 / 111 there; Riku keeps his model");
        return;
    }
    ON.store(on.bits(), Ordering::Relaxed);
    log(&format!(
        "riku skin: {} (chars 3 / 4 -> groups {:?}, motion packs {:?}); shown from the next room or battle",
        look_name(on),
        rs::entries(&sk, on),
        rs::entries(&sk, on).map(|g| rs::ann_name(g as u16))
    ));
}

// ---------------------------------------------------------------------------------------------
// The costume changed in place (0.155.0; fork owner, 2026-11-22: "seria possível trocar a skin sem
// ter que passar por uma porta?", chose "Trocar no lugar").
//
// Re:CoM never rebuilds a live player's model; every step below is one Re:CoM itself runs when it
// builds an entity's model or a player's state, in the same order (Checked (exe: disasm)):
//
// 1. The model part is released as the entity's dtor releases it (`0x1401A2440`): the parts
//    `+0xB0` (vector-deleting dtor of element 0, or a plain free with no count), the matrices
//    `+0xB8`, the sub-models `+0xC8` / `+0xD0`, the motion controller `+0xD8`, the materials
//    `+0xE0` (array dtor `0x1402894E0`, 0x40 each), the texture animation `+0x1A0`, the EET / SES
//    players `+0x108` / `+0x118`.
// 2. Built again for the new group as the model init `0x1401A4E70` builds them: `0x1401A5110(e,
//    id, 1)` (parts, matrices, sub-models, controller; the parts bind to the entity's transform
//    node `e+8`, which takes their bone count), the texture animation (`0x1402A4D70`, `0x140178620`,
//    `0x140178F00`, vtable `+0x10`), the EET player (`0x140179420`, `0x140179A30`) and SES player
//    (`0x140238460`, `0x140238710`).
// 3. The player's state init's model steps (`0x1401FA6E0`): the controller's defaults
//    `0x1401A3990(e, −1, −1)`, its step-effect name and flag (`0x1401A6B40(e, "asiato", 0)`,
//    `0x1401A6B70(e, 1)`), the action rows' clips bound by name in the new motion pack
//    (`0x1401F4240(e, char)`), the bones by name (`0x1401A4450`: `wep0`, `ex_1010_sk`, `chest1`,
//    `head`, `l_toes`, `r_toes` at `e+0x3A0..0x3B4`).
// 4. Riku's weapon object (`e+0x3D0`, chars 3..6: attached to `wep0`'s matrix, `0x140224D23`):
//    its bone `+0x238` and its attach (`0x1402ABD20(w, 1, 0x1402ABB20(e, wep0))`, which reuses its
//    node and only re-points it).
// 5. The idle row's motion (Re:CoM's motion request `0x1401F7180`).
//
// Only in the field, with Riku standing, walking or running (actions 1..4), never in battle or in
// Dark Mode; at the camp's close, else at his next frame once the group is loaded.

use super::movement::{func, rd, wr};
use std::sync::atomic::{AtomicU32, AtomicU8, AtomicUsize};

/// 0 unknown, 1 the callees match, 2 they don't (no live change).
static CALLEES: AtomicU8 = AtomicU8::new(0);
/// A change waits for its group to load (or for Riku to stand).
static PENDING: AtomicBool = AtomicBool::new(false);

const CALLEE_BYTES: &[(u32, [u8; 8])] = &[
    (0x001A_5110, [0x44, 0x89, 0x44, 0x24, 0x18, 0x89, 0x54, 0x24]),
    (0x002A_4D70, [0x48, 0x83, 0xEC, 0x28, 0x41, 0xB8, 0x01, 0x00]),
    (0x0017_8620, [0x48, 0x89, 0x4C, 0x24, 0x08, 0x53, 0x48, 0x83]),
    (0x0017_8F00, [0x4D, 0x85, 0xC0, 0x0F, 0x84, 0xF8, 0x02, 0x00]),
    (0x0017_9420, [0x48, 0x89, 0x51, 0x08, 0x48, 0x8D, 0x05, 0xFD]),
    (0x0017_9A30, [0x48, 0x89, 0x5C, 0x24, 0x18, 0x56, 0x48, 0x83]),
    (0x0023_8460, [0x48, 0x89, 0x51, 0x08, 0x48, 0x8D, 0x05, 0xF5]),
    (0x0023_8710, [0x41, 0x56, 0x48, 0x83, 0xEC, 0x30, 0x48, 0xC7]),
    (0x001A_6B40, [0x48, 0x8B, 0x81, 0xD8, 0x00, 0x00, 0x00, 0x48]),
    (0x001A_6B70, [0x4C, 0x8B, 0x81, 0xD8, 0x00, 0x00, 0x00, 0x4D]),
    (0x001A_3990, [0x48, 0x8B, 0x89, 0xD8, 0x00, 0x00, 0x00, 0x48]),
    (0x001F_4240, [0x48, 0x83, 0xEC, 0x28, 0x85, 0xD2, 0x74, 0x5F]),
    (0x001A_4450, [0x4C, 0x8B, 0x89, 0xB0, 0x00, 0x00, 0x00, 0x4D]),
    (0x002A_BB20, [0x48, 0x8B, 0x49, 0x08, 0x48, 0x85, 0xC9, 0x74]),
    (0x002A_BD20, [0x40, 0x56, 0x57, 0x41, 0x56, 0x48, 0x83, 0xEC]),
    (0x004A_CE54, [0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B]),
    (0x004A_D470, [0xE9, 0xF7, 0x0E, 0x00, 0x00, 0xCC, 0xCC, 0xCC]),
    (0x004A_D374, [0x48, 0x8B, 0xC4, 0x4C, 0x89, 0x48, 0x20, 0x4C]),
    (0x004A_D478, [0xE9, 0xF3, 0xFF, 0xFF, 0xFF, 0xCC, 0xCC, 0xCC]),
    (0x0028_94E0, [0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B]),
    (0x001F_7180, [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C]),
    (0x0028_F800, [0x40, 0x53, 0x48, 0x83, 0xEC, 0x50, 0x48, 0x8B]),
    (0x001A_57D0, [0x48, 0x8B, 0x81, 0xD8, 0x00, 0x00, 0x00, 0x48]),
    (0x001A_2D50, [0x45, 0x8B, 0xD0, 0x4C, 0x8B, 0xDA, 0x44, 0x8B]),
    (0x0025_7EF0, [0x48, 0x83, 0xEC, 0x28, 0x8B, 0xD1, 0x48, 0x8B]),
    (0x002A_42B0, [0x40, 0x56, 0x41, 0x56, 0x41, 0x57, 0x48, 0x83]),
];
/// The deferred array deleter's vtable (the node dtor's, `0x14023DEA1`). Checked (exe).
const ARRAY_DELETER_VT: u64 = 0x1_4060_5B38;

type DeferFn = unsafe extern "win64" fn(usize);

/// A transform node's matrix array released as the node's dtor releases it (`0x14023DE20`: each
/// element's `+0x9C` cleared, the array handed to a 0x20-byte deleter (vtable `0x140605B38`, flag
/// 1) for Re:CoM's deferred free `0x14028F800`, the draw thread may still read it this frame),
/// then pointer and count 0. The next bind (`0x140270030`) or attach (`0x1402ABD20`) allocates it
/// again at the right size (`0x14023EBE0` allocates only at count 0, and never resizes: 0.155.0's
/// bug, the node kept Riku's 102 for Dark Riku's 128 bones and the other way round).
unsafe fn clear_node(n: usize) {
    if n == 0 {
        return;
    }
    let arr: usize = rd(n + 8);
    if arr != 0 {
        let count: u32 = rd(n + 0x10);
        for i in 0..count as usize {
            wr::<u32>(arr + i * 0xE0 + 0x9C, 0);
        }
        let d = func::<NewFn>(0x004A_CE54)(0x20);
        if d != 0 {
            wr::<usize>(d, va(super::base(), ARRAY_DELETER_VT));
            wr::<usize>(d + 8, 0);
            wr::<usize>(d + 0x10, arr);
            wr::<u32>(d + 0x18, 1);
            func::<DeferFn>(0x0028_F800)(d);
        }
    }
    wr::<usize>(n + 8, 0);
    wr::<u32>(n + 0x10, 0);
}
/// The step-effect name pointer the state init passes (`"asiato"`). Checked (exe).
const ASIATO_PTR: u64 = 0x1_407C_5DB0;
/// Riku's player fields (entity offsets) for his bones by name, in the state init's order.
const BONE_FIELDS: [(usize, &[u8]); 6] = [(0x3A0, b"wep0\0"), (0x3A4, b"ex_1010_sk\0"), (0x3A8, b"chest1\0"), (0x3AC, b"head\0"), (0x3B0, b"l_toes\0"), (0x3B4, b"r_toes\0")];
const WEAPON: usize = 0x3D0;
const WEAPON_BONE: usize = 0x238;
const GROUP: usize = 0x39C;
const CHAR: usize = 0x398;

type NewFn = unsafe extern "win64" fn(usize) -> usize;
type FreeFn = unsafe extern "win64" fn(usize);
type VdelFn = unsafe extern "win64" fn(usize, u32);
type LoadFn = unsafe extern "win64" fn(usize, u32, u32) -> u32;
type ResFn = unsafe extern "win64" fn(u32) -> usize;
type CtorFn = unsafe extern "win64" fn(usize) -> usize;
type Ctor2Fn = unsafe extern "win64" fn(usize, usize) -> usize;
type BindFn = unsafe extern "win64" fn(usize, u32) -> u64;
type TxaFn = unsafe extern "win64" fn(usize, usize, usize) -> u64;
type ArrDtorFn = unsafe extern "win64" fn(usize, usize, usize, usize);
type SizedFreeFn = unsafe extern "win64" fn(usize, usize);
type DefaultsFn = unsafe extern "win64" fn(usize, f32, f32) -> u64;
type NameFn = unsafe extern "win64" fn(usize, usize, u32);
type FlagFn = unsafe extern "win64" fn(usize, u32);
type RebindFn = unsafe extern "win64" fn(usize, u32);
type BoneFn = unsafe extern "win64" fn(usize, *const u8, u32) -> i32;
type MtxFn = unsafe extern "win64" fn(usize, u32) -> usize;
type AttachFn = unsafe extern "win64" fn(usize, u32, usize);

unsafe fn callees_ok(b: usize) -> bool {
    match CALLEES.load(Ordering::Relaxed) {
        1 => true,
        2 => false,
        _ => {
            let bad: Vec<String> = CALLEE_BYTES
                .iter()
                .filter(|(rva, want)| std::slice::from_raw_parts((b + *rva as usize) as *const u8, 8) != want)
                .map(|(rva, _)| format!("{:#x}", 0x1_4000_0000u64 + *rva as u64))
                .collect();
            let ok = bad.is_empty();
            CALLEES.store(if ok { 1 } else { 2 }, Ordering::Relaxed);
            if !ok {
                log(&format!("riku skin: Re:CoM's functions at {bad:?} are not the expected ones; a costume shows from the next room only"));
            }
            ok
        }
    }
}

/// `obj`'s virtual deleting dtor with `flags` (vtable `+0`), as the entity's dtor calls it.
unsafe fn vdel(obj: usize, flags: u32) {
    if obj != 0 {
        let vt: usize = rd(obj);
        let f: VdelFn = std::mem::transmute_copy(&rd::<usize>(vt));
        f(obj, flags);
    }
}

/// Step 1: the model part released as `0x1401A2440` releases it.
unsafe fn release(e: usize) {
    let parts: usize = rd(e + 0xB0);
    if parts != 0 {
        if rd::<u64>(parts - 0x10) == 0 {
            func::<FreeFn>(0x004A_D470)(parts - 0x10);
        } else {
            vdel(parts, 3);
        }
    }
    wr::<usize>(e + 0xB0, 0);
    let m: usize = rd(e + 0xB8);
    if m != 0 {
        func::<FreeFn>(0x004A_D470)(m);
    }
    wr::<usize>(e + 0xB8, 0);
    for off in [0xC8, 0xD0, 0xD8] {
        vdel(rd(e + off), 1);
        wr::<usize>(e + off, 0);
    }
    let mats: usize = rd(e + 0xE0);
    if mats != 0 {
        let head = mats - 8;
        let n: usize = rd(head);
        func::<ArrDtorFn>(0x004A_D374)(mats, 0x40, n, base_of(0x0028_94E0));
        func::<SizedFreeFn>(0x004A_D478)(head, n * 0x40 + 8);
    }
    wr::<usize>(e + 0xE0, 0);
    for off in [0x1A0, 0x108, 0x118] {
        vdel(rd(e + off), 1);
        wr::<usize>(e + off, 0);
    }
}

unsafe fn base_of(rva: u32) -> usize {
    super::base() + rva as usize
}

/// Steps 2..5 for group `g`. False when Re:CoM's model load refused (the entity then has no model
/// until the next room builds the player again).
unsafe fn rebuild(e: usize, g: u16) -> bool {
    let id = 10000 + g as u32;
    if func::<LoadFn>(0x001A_5110)(e, id, 1) == 0 {
        return false;
    }
    let res = func::<ResFn>(0x002A_4D70)(id);
    if res != 0 {
        let t = func::<NewFn>(0x004A_CE54)(0x80);
        if t != 0 {
            func::<CtorFn>(0x0017_8620)(t);
            wr::<usize>(e + 0x1A0, t);
            func::<TxaFn>(0x0017_8F00)(t, res, e);
            let vt: usize = rd(t);
            let f: FreeFn = std::mem::transmute_copy(&rd::<usize>(vt + 0x10));
            f(t);
        }
    }
    let eet = func::<NewFn>(0x004A_CE54)(0x930);
    if eet != 0 {
        func::<Ctor2Fn>(0x0017_9420)(eet, e);
        wr::<usize>(e + 0x108, eet);
        func::<BindFn>(0x0017_9A30)(eet, id);
    }
    let ses = func::<NewFn>(0x004A_CE54)(0xE8);
    if ses != 0 {
        func::<Ctor2Fn>(0x0023_8460)(ses, e);
        wr::<usize>(e + 0x118, ses);
        func::<BindFn>(0x0023_8710)(ses, id);
    }
    // Step 3.
    func::<DefaultsFn>(0x001A_3990)(e, -1.0, -1.0);
    func::<NameFn>(0x001A_6B40)(e, rd::<usize>(va(super::base(), ASIATO_PTR)), 0);
    func::<FlagFn>(0x001A_6B70)(e, 1);
    wr::<u32>(e + GROUP, g as u32);
    func::<RebindFn>(0x001F_4240)(e, rd::<u32>(e + CHAR));
    for (off, name) in BONE_FIELDS {
        let i = func::<BoneFn>(0x001A_4450)(e, name.as_ptr(), 0);
        wr::<i32>(e + off, i);
    }
    // Step 4.
    let w: usize = rd(e + WEAPON);
    let bone: i32 = rd(e + 0x3A0);
    if w != 0 && bone >= 0 {
        wr::<i32>(w + WEAPON_BONE, bone);
        // Its node's one matrix still names the old array: made again by the attach.
        clear_node(rd(w + 8));
        let m = func::<MtxFn>(0x002A_BB20)(e, bone as u32);
        func::<AttachFn>(0x002A_BD20)(w, 1, m);
    }
    // Step 5: the idle row.
    func::<super::movement::MotionFn>(super::movement::MOTION_REQUEST)(e, 1, 0, 0, 0, 0, 0);
    true
}

/// The group the costume wants for this Riku (char 3 / 4), if the skin is in.
unsafe fn wanted(e: usize) -> Option<u16> {
    let sk = skins();
    if sk.is_empty() || !ROWS.load(Ordering::Acquire) {
        return None;
    }
    let ch: u32 = rd(e + CHAR);
    let k = ch.checked_sub(3).filter(|k| *k < 2)? as usize;
    Some(rs::entries(&sk, look())[k] as u16)
}

/// The main player is (plain) Riku in the field, standing, walking or running.
unsafe fn swappable(b: usize, e: usize) -> bool {
    e != 0
        && rd::<usize>(e) == va(b, crate::sites::data::RIKU_VTABLE)
        && e == rd::<usize>(va(b, crate::sites::data::MAIN_PLAYER))
        && rd::<u32>(e + super::movement::p::BATTLE) == 0
        && (1..=4).contains(&rd::<u32>(e + super::movement::p::ACTION))
}

/// Tries the change now: true when nothing is left to do.
unsafe fn try_swap(b: usize) -> bool {
    let e: usize = rd(va(b, crate::sites::data::MAIN_PLAYER));
    if e == 0 || rd::<usize>(e) != va(b, crate::sites::data::RIKU_VTABLE) {
        return true;
    }
    let Some(g) = wanted(e) else { return true };
    let now: u32 = rd(e + GROUP);
    if now == g as u32 {
        return true;
    }
    if !super::models::group_ready(g) {
        super::models::queue_group(g);
        return false;
    }
    if !swappable(b, e) || !callees_ok(b) {
        return !callees_ok(b);
    }
    // 0.160.0: only the Keyblade changes (same attire): no transformation; the model changes now,
    // with Riku's way-back effect.
    let sk = skins();
    let (lnow, lg) = (rs::look_of(&sk, now as u16), rs::look_of(&sk, g));
    if TRANS_PL.load(Ordering::Relaxed) == 0 && lnow.dark == lg.dark && lnow.halloween == lg.halloween {
        swap_now(b, e, g);
        weapon_appear(e);
        func::<super::movement::MotionFn>(super::movement::MOTION_REQUEST)(e, 1, 0, 0, 0, 0, 0);
        return true;
    }
    // 0.155.2: Riku's own transformation first; the model changes at its end (`transition`).
    if TRANS_PL.load(Ordering::Relaxed) == 0 {
        func::<super::movement::MotionFn>(super::movement::MOTION_REQUEST)(e, TRANSFORM_ROW, -1, 0, 0, 0, 0);
        TRANS_PL.store(e, Ordering::Relaxed);
        TRANS_FRAMES.store(0, Ordering::Relaxed);
        TRANS_PHASE.store(0, Ordering::Relaxed);
        stop(e);
        log(&format!("riku skin: costume change: Riku's transformation (row {TRANSFORM_ROW:#x}) plays, the model changes at its end"));
    }
    false
}

/// The model change itself (after the transformation's motion).
unsafe fn swap_now(b: usize, e: usize, g: u16) {
    let now: u32 = rd(e + GROUP);
    let node_before = node_count(e);
    release(e);
    clear_node(rd(e + 8));
    let ok = rebuild(e, g);
    log(&format!(
        "riku skin: costume changed in place: group {now} -> {g} ({}); transform node {node_before} -> {} matrices; wep0 bone {}",
        if ok { "model rebuilt" } else { "Re:CoM's model load REFUSED: no model until the next room" },
        node_count(e),
        rd::<i32>(e + 0x3A0)
    ));
    sync_menu(b);
}

unsafe fn node_count(e: usize) -> u32 {
    let n: usize = rd(e + 8);
    if n == 0 {
        0
    } else {
        rd(n + 0x10)
    }
}

/// A costume chosen in the camp: its group starts loading now; the change happens at the camp's
/// close (or at Riku's next frame once it is loaded).
///
/// # Safety
/// The camp's update (main thread).
pub unsafe fn costume_chosen(b: usize) {
    let e: usize = rd(va(b, crate::sites::data::MAIN_PLAYER));
    if let Some(g) = (e != 0 && rd::<usize>(e) == va(b, crate::sites::data::RIKU_VTABLE)).then(|| wanted(e)).flatten() {
        super::models::queue_group(g);
        PENDING.store(true, Ordering::Relaxed);
    }
}

/// The camp closed (main thread, the field's tasks running again).
///
/// # Safety
/// `b` is the running image's base.
pub unsafe fn camp_closed(b: usize) {
    if PENDING.load(Ordering::Relaxed) && try_swap(b) {
        PENDING.store(false, Ordering::Relaxed);
    }
}

/// Riku's frame (his move exe): one atomic load unless a change waits; the HUD face follows the
/// costume (one pointer compare).
///
/// # Safety
/// The game's thread, in a player's move exe (`e` that player).
pub unsafe fn frame(e: usize) -> bool {
    let b = super::base();
    // 0.156.0: Dark Mode reached (Dark Riku the main player, any Combat setting): the costume's
    // unlock kept in Riku's save. One compare a frame.
    if e != 0 && rd::<usize>(e) == va(b, crate::sites::data::DRIKU_VTABLE) && e == rd::<usize>(va(b, crate::sites::data::MAIN_PLAYER)) {
        super::dark_points::mark_seen();
    }
    if e != 0 && rd::<usize>(e) == va(b, crate::sites::data::RIKU_VTABLE) && e == rd::<usize>(va(b, crate::sites::data::MAIN_PLAYER)) {
        mark_halloween(e);
    }
    if TRANS_PL.load(Ordering::Relaxed) != 0 {
        return transition(b, e);
    }
    if PENDING.load(Ordering::Relaxed) && try_swap(b) {
        PENDING.store(false, Ordering::Relaxed);
    }
    if TRANS_PL.load(Ordering::Relaxed) != 0 {
        return true;
    }
    if ROWS.load(Ordering::Relaxed) && e != 0 && rd::<usize>(e) == va(b, crate::sites::data::RIKU_VTABLE) && e == rd::<usize>(va(b, crate::sites::data::MAIN_PLAYER)) {
        face(b, e);
    }
    false
}

// The transformation (0.155.2; fork owner, 2026-11-22: "só que eu realmente queria a transição
// quando altera-se a skin de uma para outra, a animação e o efeito"). Riku's own Dark Mode start
// (`0x140215FB0(riku, 1)` → action 0x20, exe `0x140215380`, Checked (exe: disasm)) plays his action
// row 0x37 (`p002_057`, blend −1: `0x140215402..0x140215415`) on the ground, its effects and sounds
// being that clip's EET / SES events; at the motion's end (`0x1401A57D0`: controller `+0x50 & 3`,
// `0x140215428`) it switches to Dark Riku. Here the same row plays (the costume's own `PL01x1.EET`
// / `.SES` events, remapped to its bones), Riku's move exe is held while it plays (no KH1 frame, no
// Re:CoM move: nothing replaces the motion), and at its end the model changes in place. Riku's
// Dark Mode action itself is not entered: its end switches to the battle-only Dark Riku.

/// Riku's transformation row (`0x140215411`: `lea edx, [rdi+0x37]`). Checked (exe).
const TRANSFORM_ROW: i32 = 0x37;
/// At most this many frames for the motion (its clip is shorter; a safety end).
const TRANSFORM_MAX: u32 = 300;
/// The player playing the transformation (0: none).
static TRANS_PL: AtomicUsize = AtomicUsize::new(0);
static TRANS_FRAMES: AtomicU32 = AtomicU32::new(0);
type EndFn = unsafe extern "win64" fn(usize) -> u32;

/// Phase of the transformation: 0 Riku's crouch (row 0x37), 1 Dark Riku's flip (0.155.3).
static TRANS_PHASE: AtomicU8 = AtomicU8::new(0);
/// Re:CoM's char → action table (`0x1407C6300[char]`, 16-byte rows: name, clip index `+8`) and
/// the char → motion pack name table. Checked (exe: `0x1401FA7F6..0x1401FA81E`).
const ACTION_TABLES: u64 = 0x1_407C_6300;
const PACK_NAMES: u64 = 0x1_407C_6338;
/// The way back's effect (Riku's own: `0x1402157CD..0x140215811`): `"transform"` effect 2 on his
/// bone 0x1B, through `0x1401A2D50`, the effect flagged `+0x4C |= 4`. Checked (exe).
const TRANSFORM_FX_NAME: u64 = 0x1_407C_7C80;
const TRANSFORM_FX_BONE: u32 = 0x1B;
type FxFn = unsafe extern "win64" fn(usize, usize, u32, usize, u64, u64, f32) -> u32;
type FxObjFn = unsafe extern "win64" fn(u32) -> usize;
type ClipFn = unsafe extern "win64" fn(usize, *const u8, u32) -> i32;

/// One frame of the transformation; true while Riku's move exe is held.
unsafe fn transition(b: usize, e: usize) -> bool {
    let pl = TRANS_PL.load(Ordering::Relaxed);
    if e != pl {
        // Another player's move exe (or the room was rebuilt): only ours is held.
        if rd::<usize>(va(b, crate::sites::data::MAIN_PLAYER)) != pl {
            TRANS_PL.store(0, Ordering::Relaxed);
            TRANS_PHASE.store(0, Ordering::Relaxed);
        }
        return false;
    }
    let n = TRANS_FRAMES.fetch_add(1, Ordering::Relaxed) + 1;
    stop(e);
    // The motion's end: its first frames may still read the previous clip's end bits.
    let ended = n > 4 && func::<EndFn>(0x001A_57D0)(e) & 3 != 0;
    if !ended && n < TRANSFORM_MAX {
        return true;
    }
    let ch: u32 = rd(e + CHAR);
    let row = rd::<usize>(va(b, ACTION_TABLES) + 8 * ch as usize) + 16 * TRANSFORM_ROW as usize;
    if TRANS_PHASE.load(Ordering::Relaxed) == 1 {
        // The flip ended: row 0x37 bound back to its own clip (Re:CoM's rebind by name).
        func::<RebindFn>(0x001F_4240)(e, ch);
        return finish(e);
    }
    let Some(g) = wanted(e) else { return finish(e) };
    if rd::<u32>(e + GROUP) == g as u32 || !super::models::group_ready(g) {
        return finish(e);
    }
    swap_now(b, e, g);
    let to_dark = rs::look_of(&skins(), g).dark;
    if to_dark {
        // Dark Riku's entry flip, as his switch-in plays it (`0x140204841..0x14020485B`: his row
        // 0x37, blend −1): the skin's pack holds his own clip; row 0x37 points at it for the play.
        let pack: usize = rd(va(b, PACK_NAMES) + 8 * ch as usize);
        let clip = func::<ClipFn>(0x002A_42B0)(pack, b"p003_067\0".as_ptr(), 0);
        if clip >= 0 {
            wr::<i32>(row + 8, clip);
            func::<super::movement::MotionFn>(super::movement::MOTION_REQUEST)(e, TRANSFORM_ROW, -1, 0, 0, 0, 0);
            TRANS_PHASE.store(1, Ordering::Relaxed);
            TRANS_FRAMES.store(0, Ordering::Relaxed);
            log(&format!("riku skin: Dark Riku's entry flip (clip {clip}) plays"));
            return true;
        }
        log("riku skin: the skin's pack has no p003_067 (asset step older than 0.155.3?): no flip");
    } else {
        transform_fx(b, e);
    }
    finish(e)
}

/// 0.174.0 (fork owner, 2026-11-23: "na transição quando troca de keyblade deve ser o mesmo efeito
/// quando ele está sem e vc aperta o botão de ataque e ela aparece na mão"): Riku's own "draw the
/// weapon" (his vtable `+0xF8` = `0x1402160A0(riku, 1)`, Checked (exe: disasm)): with the weapon put
/// away (`+0x418 & 2`) it clears that, plays its appear effect (`0x1401A2D50` with the effect name
/// `[0x1407C7C98]` on his weapon bone `+0x3A0`) and fades the weapon in (`+0x154 | 0x40000000`,
/// `+0x17C..+0x190`: 2000 ticks). The bit is set first, so the new blade appears as from his hand.
unsafe fn weapon_appear(e: usize) {
    wr::<u32>(e + 0x418, rd::<u32>(e + 0x418) | 2);
    let vt: usize = rd(e);
    let draw: usize = rd(vt + 0xF8);
    if draw != 0 {
        let f: unsafe extern "win64" fn(usize, i32) = std::mem::transmute(draw);
        f(e, 1);
    }
}

/// Riku's way back effect.
unsafe fn transform_fx(b: usize, e: usize) {
    let m = func::<MtxFn>(0x002A_BB20)(e, TRANSFORM_FX_BONE);
    let h = func::<FxFn>(0x001A_2D50)(e, rd::<usize>(va(b, TRANSFORM_FX_NAME)), 2, m, 0, 0, 0.0);
    let fx = func::<FxObjFn>(0x0025_7EF0)(h);
    if fx != 0 {
        wr::<u32>(fx + 0x4C, rd::<u32>(fx + 0x4C) | 4);
    }
}

/// The transformation's end: Riku's idle, the move exe his again.
unsafe fn finish(e: usize) -> bool {
    TRANS_PL.store(0, Ordering::Relaxed);
    TRANS_PHASE.store(0, Ordering::Relaxed);
    PENDING.store(false, Ordering::Relaxed);
    func::<super::movement::MotionFn>(super::movement::MOTION_REQUEST)(e, 1, 0, 0, 0, 0, 0);
    true
}

// The HUD face (fork owner, 2026-11-22: "quando trocar a skin para a darkmode também deve alterar
// a foto no hud"). Re:CoM's own Dark Mode switch of the player gauge (`0x14020BC60`, Checked (exe:
// disasm)) points the gauge's face set `+0x88` at `"gauge:02"` (`0x1407C7AB0`; Riku's own is
// `"gauge:00"`, `0x1407C7AA0`, the gauge init's) and replaces the face layer 4 of its control
// `+0x78` (vtable `+0x20`, the layer replace) with that set's group 3, or group 0 at 25 % HP or
// less (`+0xA4` HP, `+0xA8` max, x offset −`+0xA0`, flag 1); every later face change reads `+0x88`.
// Its other changes (the frame's layer 0, the Dark gauge state `+0x90 & 8`) are Dark Mode's, not
// done. The gauge is the player's `+0x3E0` (the DP gauge setter's). Never while that state bit is
// set (Re:CoM's Dark Mode owns the gauge then).

const GAUGE: usize = 0x3E0;
/// A face replace Re:CoM refused: not tried again this session.
static FACE_FAILED: AtomicBool = AtomicBool::new(false);
const FACE_SET_RIKU: u64 = 0x1_407C_7AA0;
const FACE_SET_DARK: u64 = 0x1_407C_7AB0;

unsafe fn face(b: usize, e: usize) {
    let g: usize = rd(e + GAUGE);
    if g == 0 || FACE_FAILED.load(Ordering::Relaxed) || rd::<u32>(g + 0x90) & 8 != 0 {
        return;
    }
    let (riku, dark) = (rd::<usize>(va(b, FACE_SET_RIKU)), rd::<usize>(va(b, FACE_SET_DARK)));
    let on = look().dark;
    let now: usize = rd(g + 0x88);
    let want = if on { dark } else { riku };
    // Only Riku's own face or ours: any other set is Re:CoM's state, left alone.
    if now == want || (now != riku && now != dark) {
        return;
    }
    let ctrl: usize = rd(g + 0x78);
    let Some(list) = super::kh1_hud::gauge_list(g) else { return };
    if ctrl == 0 {
        return;
    }
    let (hp, max) = (rd::<u16>(g + 0xA4) as i32, rd::<u16>(g + 0xA8) as i32);
    let group = if hp > max * 25 / 100 { 3 } else { 0 };
    wr::<usize>(g + 0x88, want);
    let ok = super::d2::replace(ctrl, list, 4, want as *const u8, group, -rd::<f32>(g + 0xA0), 1);
    log(&format!(
        "riku skin: HUD face {} (layer 4 of {} on control {ctrl:#x}, set {:?} group {group}{})",
        if on { "Dark Mode" } else { "Default Attire" },
        super::d2::layers(ctrl),
        std::ffi::CStr::from_ptr(want as *const std::ffi::c_char),
        if ok { "" } else { "; NOT made: left as it is" }
    ));
    if !ok {
        wr::<usize>(g + 0x88, now);
        FACE_FAILED.store(true, Ordering::Relaxed);
    }
}
