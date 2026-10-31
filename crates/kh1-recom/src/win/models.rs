//! The equipped Keyblade's model at the KH1 setting (decision 25, "Models"). Pure rules:
//! `crate::keyblade_models`. Notes: `docs/recom-keyblade-models.md` § Runtime.
//!
//! - **Rows**: an extended copy of Re:CoM's PLAYER resource table (its 552 rows, then our groups,
//!   `19 + card` from `kh1-stats.json`: KH1's Keyblades 21..=36, KH2's 38..=43, each laid out as
//!   its Kingdom Key group 2), installed with one checked pointer write at
//!   `0x1407EBF28`. Re:CoM's own loader (`0x14023BF70`) loads our groups from it.
//! - **Load**: Re:CoM's "queue the player's group" (`0x140211AA0`, room and battle builders)
//!   gets a wrapper that also queues the equipped Keyblade's group with Re:CoM's own call
//!   (`0x14023CCE0`), before the builder waits for the queue (`0x14023CA80(0, 0)`). Only the
//!   equipped Keyblade's group is loaded; Re:CoM's group 2 stays as Re:CoM loads it (its 100 Acre
//!   Wood Keyblade `CMini100AcreVegPanicKeyBrade` builds from group 2 too, `0x140438450`).
//! - **Pick**: CPlayerWeapon's vtable slot `+0x30` (`0x1405AADE0`, the model init `0x1401A4E70`
//!   that `0x140224BA0` calls with group 2 or 12) points at [`weapon_model`], which passes the
//!   equipped Keyblade's group at the KH1 setting in Sora's story.
//! - **Rebuild**: the field's camp open / close (`0x1401A9590`) with open = 0 runs after the camp
//!   task is gone and the field's tasks are resumed. There, once, when the weapon's group is not
//!   the wanted one, the weapon is rebuilt as Re:CoM's `stateInitial` builds it (`0x140224BA0`,
//!   then the player's put-away `+0x100`) and the old one goes through its own virtual dtor (the
//!   path Re:CoM's CTask dtor `0x1402AB950` uses for children).
//!
//! Nothing here runs per frame: the wrappers run per room / battle build, per weapon build and
//! per camp close.

use super::movement::{func, is_main_sora, orig, rd, wr};
use super::{base, log, va};
use crate::keyblade_models::{self as km, kind, Close, Models, PlannedRow, RawRow, RowName};
use crate::sites::{data, MODEL_PTRS};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

pub static ORIG_GROUP_LOAD: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_CAMP_FIELD: AtomicUsize = AtomicUsize::new(0);

static INSTALLED: AtomicBool = AtomicBool::new(false);
static MODELS: OnceLock<Models> = OnceLock::new();
static STATE: Mutex<State> = Mutex::new(State { built: None, ours_loaded: Vec::new(), logged: None });
/// One-time log lines (by a short key).
static ONCE: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

struct State {
    /// The weapon our wrapper last built for Sora, and its group.
    built: Option<(usize, u16)>,
    /// Our groups we queued and have not unloaded.
    ours_loaded: Vec<u16>,
    /// The last group the build log named.
    logged: Option<u16>,
}

// Re:CoM functions (RVAs; first bytes checked in `sites::MODEL_HOOKS` / `MODEL_CALLEES`).
const QUEUE_GROUP: u32 = 0x0023_CCE0;
const QUEUED: u32 = 0x0023_CA80;
const FIND: u32 = 0x002A_3E20;
const UNLOAD: u32 = 0x002A_5B00;
const WEAPON_CTOR: u32 = 0x0022_4BA0;
const MODEL_INIT: u32 = 0x001A_4E70;
const NEW: u32 = 0x004A_CE54;

// Re:CoM data (Steam addresses).
/// The PLAYER table (category 1) and the pointer to it in `PTR 0x1407EBF20[]`. Checked (exe).
const PLAYER_TABLE: u64 = 0x1_4080_55D0;
const TABLE_PTR: u64 = 0x1_407E_BF28;
/// CPlayerWeapon's vtable, and the slot of its model init. Checked (exe + decomp `0x140224BA0`).
const WEAPON_VTABLE: u64 = 0x1_405A_ADB0;
const MODEL_SLOT: u64 = 0x1_405A_ADE0;
/// The char `0x140211AA0` just loaded the group of (`DAT_1407C7AE0`). Checked (decomp).
const CURRENT_CHAR: u64 = 0x1_407C_7AE0;

// CPlayer / CPlayerWeapon fields. Checked (decomp `0x1401FA6E0`, `0x140224BA0`).
/// The weapon (`stateInitial`'s `param_1 + 0x378`, `param_1` = player + 0x58).
const PL_WEAPON: usize = 0x3D0;
/// The char (1 Sora, 2 Halloween Town Sora) and the WeaponR bone index.
const PL_CHAR: usize = 0x398;
const PL_WEAPON_BONE: usize = 0x3A0;
/// The action; 0xA is Re:CoM's attack action (`docs/recom-kh1-attack.md`).
const PL_ACTION: usize = 0x40C;
const ATTACK_ACTION: u32 = 0xA;
/// The player's vtable slot that puts the Keyblade away (`0x1401F5480`), as `stateInitial`
/// calls it right after building the weapon.
const VT_PUT_AWAY: usize = 0x100;
/// The weapon's owner (`param_1[0x46]`, set before the model init).
const WEAPON_OWNER: usize = 0x230;
/// CPlayerWeapon's size (`operator new(0x300)` in `stateInitial`).
const WEAPON_SIZE: usize = 0x300;

type GroupLoadFn = unsafe extern "win64" fn(u32);
type CampFieldFn = unsafe extern "win64" fn(usize, i32, usize, usize);
type ModelInitFn = unsafe extern "win64" fn(usize, i32, i32, u32) -> u64;
type QueueFn = unsafe extern "win64" fn(u32);
type QueuedFn = unsafe extern "win64" fn(i32, i32) -> u64;
type FindFn = unsafe extern "win64" fn(u32, u32, i32) -> usize;
type UnloadFn = unsafe extern "win64" fn(u32);
type NewFn = unsafe extern "win64" fn(usize) -> usize;
type CtorFn = unsafe extern "win64" fn(usize, usize, i32, u32) -> usize;
type DtorFn = unsafe extern "win64" fn(usize, u32) -> usize;
type PlayerFn = unsafe extern "win64" fn(usize);

fn once(key: &str, msg: &str) {
    if let Ok(mut g) = ONCE.lock() {
        if g.len() < 256 && g.insert(key.to_string()) {
            log(msg);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Install

/// Reads Re:CoM's PLAYER table (up to and including its end row).
unsafe fn read_recom_table(b: usize) -> Option<Vec<RawRow>> {
    let t = va(b, PLAYER_TABLE);
    let mut rows = Vec::new();
    for i in 0..4096 {
        let bytes: [u8; 16] = rd(t + 16 * i);
        let r = RawRow::from_bytes(&bytes);
        rows.push(r);
        if r.group == km::END {
            return Some(rows);
        }
    }
    None
}

/// A name pointer of the table, read when it points inside the image.
unsafe fn name_at(b: usize, p: u64) -> Option<String> {
    let p = p as usize;
    if p < b || p >= b + crate::sites::STEAM_SIZE_OF_IMAGE as usize - 64 {
        return None;
    }
    let s = std::slice::from_raw_parts(p as *const u8, 64);
    let end = s.iter().position(|&c| c == 0)?;
    Some(String::from_utf8_lossy(&s[..end]).into_owned())
}

/// Our copy of the table: 16-byte rows (8-aligned), and our path strings, both kept for the
/// life of the process (Re:CoM's loader thread reads them).
fn build_rows(planned: &[PlannedRow]) -> Option<usize> {
    let mut words: Vec<u64> = Vec::with_capacity(planned.len() * 2);
    for r in planned {
        let name = match &r.name {
            RowName::Recom(p) => *p,
            RowName::Own(s) => {
                let c = std::ffi::CString::new(s.as_str()).ok()?;
                Box::leak(c.into_boxed_c_str()).as_ptr() as u64
            }
        };
        let b = RawRow { group: r.group, kind: r.kind, zero: 0, name }.to_bytes();
        words.push(u64::from_le_bytes(b[0..8].try_into().ok()?));
        words.push(u64::from_le_bytes(b[8..16].try_into().ok()?));
    }
    Some(Box::leak(words.into_boxed_slice()).as_ptr() as usize)
}

/// Installs the model swap: the stats file's groups, the pointer checks, the table copy, the
/// two wrappers, then the table pointer and the vtable slot. Nothing is changed unless every
/// check passed; the wrappers do nothing until the last write is in.
pub fn install(b: usize) {
    let Some(stats) = super::stats::all() else { return };
    let m = match Models::from_stats(stats) {
        Ok(Some(m)) => m,
        Ok(None) => {
            log("keyblades: kh1-stats.json has no model groups (format 3 or older, or no PL0002.MDL at build time); Sora's weapon stays Re:CoM's (no model swap)");
            return;
        }
        Err(e) => {
            log(&format!("keyblades: kh1-stats.json model groups: {e}; no model swap"));
            return;
        }
    };
    // SAFETY: reads of the mapped image (the build was checked at start).
    unsafe {
        for p in MODEL_PTRS {
            let have: usize = rd(b + p.rva as usize);
            if have != b + p.target as usize {
                log(&format!("keyblades: {} holds {have:#x}, not {:#x}; no model swap", p.name, b + p.target as usize));
                return;
            }
        }
        let Some(recom) = read_recom_table(b) else {
            log("keyblades: Re:CoM's PLAYER table has no end row; no model swap");
            return;
        };
        let groups = m.our_groups();
        let planned = match km::plan_table(&recom, |p| name_at(b, p), &groups) {
            Ok(t) => t,
            Err(e) => {
                log(&format!("keyblades: PLAYER table: {e}; no model swap"));
                return;
            }
        };
        let Some(table) = build_rows(&planned) else {
            log("keyblades: PLAYER table copy failed; no model swap");
            return;
        };
        let site = |name: &str| crate::sites::MODEL_HOOKS.iter().find(|h| h.name == name).expect("site");
        let hooks = [
            super::hook::Hook { site: site("player group load (0x140211AA0)"), ours: group_load as *const () as usize, original: &ORIG_GROUP_LOAD },
            super::hook::Hook { site: site("field camp open/close (0x1401A9590)"), ours: camp_field as *const () as usize, original: &ORIG_CAMP_FIELD },
        ];
        if !super::hook::install_all(b, &hooks) {
            log("keyblades: model wrappers not hooked; no model swap");
            return;
        }
        let old_table = (va(b, PLAYER_TABLE) as u64).to_le_bytes();
        if !super::hook::patch(va(b, TABLE_PTR), &old_table, &(table as u64).to_le_bytes(), "PLAYER table pointer") {
            log("keyblades: PLAYER table pointer not written; no model swap");
            return;
        }
        let old_slot = ((b + MODEL_INIT as usize) as u64).to_le_bytes();
        if !super::hook::patch(va(b, MODEL_SLOT), &old_slot, &(weapon_model as *const () as usize as u64).to_le_bytes(), "CPlayerWeapon +0x30") {
            // The extra rows alone change nothing: no row of Re:CoM's names our groups.
            log("keyblades: CPlayerWeapon slot +0x30 not written; no model swap (the table copy stays, unused)");
            return;
        }
        let _ = MODELS.set(m);
        INSTALLED.store(true, Ordering::Release);
        log(&format!(
            "keyblades: models in: PLAYER table {} rows ({} Re:CoM's + {} for groups {:?}) at {table:#x}; weapon group picked at CPlayerWeapon +0x30; rebuilt at camp close",
            planned.len(),
            recom.len(),
            planned.len() - recom.len(),
            groups
        ));
    }
}

// ---------------------------------------------------------------------------------------------
// Resources

unsafe fn queued(g: u16) -> bool {
    func::<QueuedFn>(QUEUED)(km::CATEGORY as i32, g as i32) as u32 == 0
}

unsafe fn queue_idle() -> bool {
    func::<QueuedFn>(QUEUED)(0, 0) as u32 != 0
}

unsafe fn found(k: u16, g: u16) -> bool {
    func::<FindFn>(FIND)(k as u32, km::resource_id(g), 1) != 0
}

/// A group's files are in: not queued, and its RTM, MDL and ANN are found by its id.
unsafe fn loaded(g: u16) -> bool {
    !queued(g) && found(kind::RTM, g) && found(kind::MDL, g) && found(kind::ANN, g)
}

/// Queues group `g` with Re:CoM's own call when its RTM is not loaded and it is not queued (as
/// `0x140211AA0` does for the player's group).
unsafe fn queue_if_missing(g: u16) -> bool {
    if found(kind::RTM, g) || queued(g) {
        return false;
    }
    func::<QueueFn>(QUEUE_GROUP)(g as u32);
    true
}

/// Gets group `g` loading. Our groups share Re:CoM's `PL0002.ANN` / `.EET` by name; a shared
/// file keeps the id of the group that loaded it first (`0x1402A1BC0`), and Re:CoM's own lookups
/// of group 2 use that id. So group 2 is queued first when it is not loaded (FIFO queue).
unsafe fn request(g: u16) {
    let Some(m) = MODELS.get() else { return };
    if m.is_ours(g) {
        if queue_if_missing(km::RECOM_KEYBLADE) {
            log("keyblades: Re:CoM's group 2 queued first (its PL0002.ANN / .EET are shared by our groups)");
        }
        if let Ok(mut s) = STATE.lock() {
            if !s.ours_loaded.contains(&g) {
                s.ours_loaded.push(g);
            }
        }
    }
    if queue_if_missing(g) {
        log(&format!("keyblades: group {g} queued (card {:#x})", m.card_of(g).unwrap_or(0)));
    }
}

/// The group wanted when Re:CoM asks for `requested`, or None for Re:CoM's own.
fn wanted(requested: u16) -> Option<u16> {
    let m = MODELS.get()?;
    let kh1 = super::settings().combat == crate::settings::Mode::Kh1;
    let card = super::stats::equipped_card();
    let w = m.wanted(requested, kh1, !super::stats::riku_story(), card);
    if w.is_none() && kh1 && m.group_of(card).is_none() {
        once(&format!("nomodel{card}"), &format!("keyblades: card {card:#x} has no converted model; Re:CoM's weapon shows"));
    }
    w
}

/// Starts loading the equipped Keyblade's group (after an equip or a Combat switch), so it is in
/// when the camp closes.
pub fn prefetch() {
    if !INSTALLED.load(Ordering::Acquire) {
        return;
    }
    // SAFETY: static pointers of the image; Re:CoM's own queue calls (main thread: the camp).
    unsafe {
        let b = base();
        let pl: usize = rd(va(b, data::MAIN_PLAYER));
        let ch: i32 = if is_main_sora(pl) { rd(pl + PL_CHAR) } else { rd(va(b, CURRENT_CHAR)) };
        let Some(r) = km::recom_group_for_char(ch) else { return };
        if let Some(w) = wanted(r).filter(|w| *w != r) {
            request(w);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Wrappers

/// Re:CoM's "queue the player's group if missing" (`0x140211AA0(char)`): then the equipped
/// Keyblade's group, before the builder waits for the queue.
pub unsafe extern "win64" fn group_load(ch: u32) {
    orig::<GroupLoadFn>(&ORIG_GROUP_LOAD)(ch);
    if !INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let now: i32 = rd(va(base(), CURRENT_CHAR));
    let Some(r) = km::recom_group_for_char(now) else { return };
    if let Some(w) = wanted(r).filter(|w| *w != r) {
        request(w);
    }
}

/// CPlayerWeapon's model init (vtable `+0x30`, Re:CoM's `0x1401A4E70(weapon, category, group,
/// flags)`): Sora's Keyblade group 2 / 12 becomes the equipped Keyblade's at the KH1 setting.
pub unsafe extern "win64" fn weapon_model(weapon: usize, category: i32, group: i32, flags: u32) -> u64 {
    let o: ModelInitFn = func(MODEL_INIT);
    let b = base();
    let owner: usize = if weapon != 0 { rd(weapon + WEAPON_OWNER) } else { 0 };
    let sora = owner != 0 && rd::<usize>(owner) == va(b, data::SORA_VTABLE);
    if !INSTALLED.load(Ordering::Acquire) || category != km::CATEGORY as i32 || !sora || !(0..=0xFFFF).contains(&group) {
        return o(weapon, category, group, flags);
    }
    let requested = group as u16;
    let mut g = requested;
    if let Some(w) = wanted(requested).filter(|w| *w != requested) {
        if loaded(w) {
            g = w;
        } else {
            request(w);
            once(&format!("fallback{w}"), &format!("keyblades: group {w} not loaded when the weapon was built; Re:CoM's group {requested} this time"));
        }
    }
    let r = o(weapon, category, g as i32, flags);
    if r as u8 == 0 && g != requested {
        log(&format!("keyblades: weapon model init failed for group {g}"));
    }
    let card = MODELS.get().and_then(|m| m.card_of(g));
    if let Ok(mut s) = STATE.lock() {
        s.built = Some((weapon, g));
        if s.logged != Some(g) {
            s.logged = Some(g);
            match card {
                Some(c) => log(&format!("keyblades: weapon group {g} built for card {c:#x}")),
                None => log(&format!("keyblades: weapon group {g} built (Re:CoM's)")),
            }
        }
    }
    r
}

/// The field's camp open / close (`0x1401A9590(field, open)`): after Re:CoM's close work, the
/// weapon is rebuilt once if it does not show the wanted group.
pub unsafe extern "win64" fn camp_field(field: usize, open: i32, a3: usize, a4: usize) {
    orig::<CampFieldFn>(&ORIG_CAMP_FIELD)(field, open, a3, a4);
    if open == 0 && INSTALLED.load(Ordering::Acquire) {
        camp_closed();
    }
}

unsafe fn camp_closed() {
    let b = base();
    let pl: usize = rd(va(b, data::MAIN_PLAYER));
    if !is_main_sora(pl) {
        return;
    }
    let ch: i32 = rd(pl + PL_CHAR);
    let Some(r) = km::recom_group_for_char(ch) else { return };
    let weapon: usize = rd(pl + PL_WEAPON);
    if weapon == 0 || rd::<usize>(weapon) != va(b, WEAPON_VTABLE) {
        return;
    }
    let want = wanted(r).unwrap_or(r);
    let built = STATE.lock().ok().and_then(|s| s.built).filter(|(w, _)| *w == weapon).map(|(_, g)| g);
    let attacking = rd::<u32>(pl + PL_ACTION) == ATTACK_ACTION;
    let mut now = built;
    match km::camp_close(built, want, loaded(want), attacking) {
        Close::Keep => {}
        Close::Rebuild(g) => {
            if rebuild(pl, weapon, ch, g) {
                now = Some(g);
            }
        }
        Close::Wait(g) => {
            request(g);
            log(&format!("keyblades: camp closed; group {g} still loading: it shows from Re:CoM's next weapon build (next room or battle) or the next camp close"));
        }
        Close::Skip if built.is_none() => {
            once("unseen", "keyblades: camp closed, but Sora's weapon was not built through our slot; no rebuild");
        }
        Close::Skip => log(&format!("keyblades: camp closed mid-attack; group {want} shows from the next weapon build")),
    }
    // Our groups nobody uses any more: unloaded with Re:CoM's own call, only when the weapon's
    // group is known and the loader is idle (the unload walks the lists the loader thread fills).
    let Some(cur) = now else { return };
    if !queue_idle() {
        return;
    }
    let ours = STATE.lock().map(|s| s.ours_loaded.clone()).unwrap_or_default();
    for g in km::stale(&ours, &[cur, want]) {
        func::<UnloadFn>(UNLOAD)(km::resource_id(g));
        if let Ok(mut s) = STATE.lock() {
            s.ours_loaded.retain(|x| *x != g);
        }
        log(&format!("keyblades: group {g} unloaded (not equipped)"));
    }
}

/// Sora's weapon rebuilt as `CPlayer::stateInitial` (`0x1401FA6E0`) builds it: `operator
/// new(0x300)`, `0x140224BA0(weapon, player, char, WeaponR)` (our slot picks the group), the
/// player's `+0x3D0`, then its put-away (vtable `+0x100`). The old weapon goes through its own
/// virtual scalar-deleting dtor (`0x140224EC0`), as Re:CoM's CTask dtor deletes its children:
/// its CTask part leaves the player's child list, its CCollAttack leaves the collision lists
/// (`0x140172740`), its model and parts go with its CEntity part (`0x1401A2440`). The new weapon
/// is built first, so Sora is never without one.
unsafe fn rebuild(pl: usize, old: usize, ch: i32, g: u16) -> bool {
    let mem = func::<NewFn>(NEW)(WEAPON_SIZE);
    if mem == 0 {
        log("keyblades: no memory for the new weapon; the old one stays");
        return false;
    }
    let bone: u32 = rd(pl + PL_WEAPON_BONE);
    let w = func::<CtorFn>(WEAPON_CTOR)(mem, pl, ch, bone);
    let got = STATE.lock().ok().and_then(|s| s.built).filter(|(x, _)| *x == w).map(|(_, x)| x);
    if w == 0 || got != Some(g) {
        log(&format!("keyblades: rebuilt weapon has group {got:?}, not {g}"));
    }
    if w == 0 {
        return false;
    }
    let dtor: DtorFn = std::mem::transmute_copy(&rd::<usize>(rd::<usize>(old)));
    dtor(old, 1);
    wr(pl + PL_WEAPON, w);
    let put_away: PlayerFn = std::mem::transmute_copy(&rd::<usize>(rd::<usize>(pl) + VT_PUT_AWAY));
    put_away(pl);
    log(&format!("keyblades: Sora's weapon rebuilt at camp close with group {g}"));
    true
}
