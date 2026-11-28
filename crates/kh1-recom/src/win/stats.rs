//! KH1 damage and HP on Re:CoM's KH1-equivalent enemies (decisions 12, 13, 18, 21;
//! `docs/recom-kh1-attack.md` § Damage and reactions).
//!
//! - The twin: the enemy's class (its vtable, checked against its RTTI name at install) and
//!   Re:CoM's current world (`0x1402339C0(0)`) pick a record from `kh1-stats.json`.
//! - HP: Re:CoM's enemy stats init `0x14017FF50` sets `+0x274` from its per-world row; at KH1
//!   our wrapper then sets the twin's KH1 HP, and the max-HP getter `0x14017F5A0` returns it for
//!   that enemy. Both are Re:CoM's own fields and functions; HP bars, deaths and boss HP checks
//!   read them as usual.
//! - Damage: a KH1 hit's value on the hit part (`part+0x92`, which Re:CoM's hit handler reads
//!   through `0x14016FDB0`) becomes KH1's damage for Sora's swing row, his KH1 Strength and the
//!   twin's Defence / floor / resistance. Re:CoM's own path then applies its extras (weak part,
//!   resistance %, boss falloff; decision 13); the twin's per-hit cap goes on at Re:CoM's damage
//!   join after them (Sora's damage cap, `super::damage_cap`).
//! - Damage = Re:CoM (owner, 2026-11-01; `super::damage`): no KH1 HP is written and no KH1 damage
//!   replaces Re:CoM's; the twins' / proxies' records still give the non-damage numbers.
//! - Proxies (`docs/recom-kh1-attack.md` § KH1 numbers for enemies without a twin): an enemy
//!   without an ordinary twin (Re:CoM-only enemies and every boss) takes, in the same wrapper,
//!   its own Re:CoM row HP × the world's ratio (from the twins, or the world's boss twin) and the
//!   reference's Defence / floor / cap / level / resistances; a boss with a KH1 equivalent takes
//!   its own KH1 record. Its hits then take KH1's damage on that record, the same path as a twin.

use super::movement::{orig, rd, wr};
use super::{base, log, va};
use crate::attack::{enp_row, EnemyStats, Kh1Stats};
use kh1_core::WeaponData;
use std::sync::atomic::AtomicUsize;
use std::sync::{Mutex, OnceLock};

pub static ORIG_STAT_INIT: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_MAX_HP: AtomicUsize = AtomicUsize::new(0);

static STATS: OnceLock<Kh1Stats> = OnceLock::new();
/// (vtable address, class) for the classes the stats file names, RTTI-checked.
static CLASSES: OnceLock<Vec<(usize, &'static str)>> = OnceLock::new();
/// Enemies whose HP is KH1's (set by the stats init at KH1), with that HP.
static KH1_HP: Mutex<Vec<(usize, i32)>> = Mutex::new(Vec::new());
static RNG: Mutex<Option<kh1_core::Kh1Rand>> = Mutex::new(None);
static LOGGED: Mutex<Vec<(&'static str, i32)>> = Mutex::new(Vec::new());
/// Enemies without an ordinary twin that took KH1 numbers at their stats init (proxies and boss
/// twins), with the record their hits use and what it is ("proxy", "proxy boss", "boss twin").
static PROXIED: Mutex<Vec<(usize, EnemyStats, &'static str)>> = Mutex::new(Vec::new());
/// (class, world, floor) already logged for proxies and the twins' Re:CoM HP check.
static LOGGED_PROXY: Mutex<Vec<(String, i32, i16)>> = Mutex::new(Vec::new());

/// Re:CoM's current world (`0x1402339C0(0)`: the world chosen for the current floor, 0 none,
/// 1 Traverse Town … 13 Castle Oblivion, the order of the name table `0x140506AA0`). Checked
/// (decomp + exe). Not `0x14087B844`, which is the floor (the enemy's per-floor row).
const WORLD_FN: u32 = 0x0023_39C0;
/// Riku's story (`0x1401AAAE0`: byte variable 0x60 ≠ 0). Checked (exe). Riku's story stays
/// Re:CoM's (decision 6).
const RIKU_FN: u32 = 0x001A_AAE0;
/// Re:CoM's floor id (i16; `0x1402337D0` is `movzx eax, word [rip+0x64806D]` = this, Checked
/// (exe)); the stats init reads the enemy's `@ENP` row from it (`crate::attack::enp_row`).
const FLOOR: u64 = 0x1_4087_B844;
/// Re:CoM's enemy category (`0x1401A8900`: 0xC builds through the enemy factory `0x140188F40`;
/// the stats init's second argument). Checked (decomp).
const ENEMY_KIND: i32 = 0xC;

/// The save's player stats (`*0x14087CBF8` = save + 0x2818; level at `+0x1C`, the level the menu
/// shows). Checked (exe `0x140239F70`, decomp `0x14010EF10`).
const STATS_PTR: u64 = 0x1_4087_CBF8;

type StatInitFn = unsafe extern "win64" fn(usize, i32, i32, i32) -> u64;
type MaxHpFn = unsafe extern "win64" fn(usize) -> i32;
type IntFn = unsafe extern "win64" fn(i32) -> i32;
type BoolFn = unsafe extern "win64" fn() -> u32;

/// Re:CoM's enemy classes with a KH1 twin and their vtables (Checked (exe): each class's RTTI
/// complete-object locator, offset 0). Checked again at install against the RTTI name.
pub const CLASS_VTABLES: &[(&str, u32)] = &[
    ("CEnemyShadow", 0x0054_D728),
    ("CEnemySoldier", 0x0054_EA68),
    ("CEnemyLargebody", 0x0054_2AD0),
    ("CEnemyRednocturne", 0x0054_8A58),
    ("CEnemyBluerhapsody", 0x0053_7CA8),
    ("CEnemyYellowopera", 0x0055_70C8),
    ("CEnemyGreenrequiem", 0x0054_04C8),
    ("CEnemyPowerwild", 0x0054_7798),
    ("CEnemyBouncywild", 0x0053_8FB8),
    ("CEnemyAirsoldier", 0x0053_1CC8),
    ("CEnemyBandit", 0x0053_4368),
    ("CEnemyFatbandit", 0x0053_DDF8),
    ("CEnemyBarrelspider", 0x0053_5688),
    ("CEnemySearchghost", 0x0054_C408),
    ("CEnemyPirate", 0x0054_6458),
    ("CEnemyAirpirate", 0x0053_09E8),
    ("CEnemyWightnight", 0x0055_3748),
    ("CEnemyGargoyle", 0x0053_F198),
    ("CEnemyDarkball", 0x0053_B7A8),
    ("CEnemyDefender", 0x0053_CA98),
    ("CEnemyWizard", 0x0055_4A58),
    ("CEnemySeaneon", 0x0054_B0E8),
    ("CEnemyAquatank", 0x0053_2FE8),
    ("CEnemyScrewdiver", 0x0054_9E18),
    ("CEnemyWyvern", 0x0055_5DC8),
    ("CEnemyWhitemushroom", 0x0055_23F8),
    ("CEnemyBlackfungus", 0x0053_69C8),
    ("CEnemyCreapplant", 0x0053_A348),
    ("CEnemyNeoshadow", 0x0054_5128),
    ("CEnemySpadesoldier", 0x0054_FD90),
    ("CEnemyHeartsoldier", 0x0054_17D0),
];

/// The RTTI name of a vtable (`.?AV<class>@@`), through its complete-object locator (`vt − 8`:
/// signature 1, type-descriptor RVA at `+0xC`, name at descriptor `+0x10`).
pub(super) unsafe fn rtti_name(b: usize, vt: usize) -> Option<String> {
    let col: usize = rd(vt - 8);
    if col < b || rd::<u32>(col) != 1 {
        return None;
    }
    let td = b + rd::<u32>(col + 0xC) as usize;
    let name = std::slice::from_raw_parts((td + 0x10) as *const u8, 64);
    let end = name.iter().position(|&c| c == 0)?;
    Some(String::from_utf8_lossy(&name[..end]).into_owned())
}

/// An object's class name for the log (its vtable's RTTI, `.?AV` / `@@` trimmed): "none" for a null
/// object, `vt 0x…` when the vtable has no RTTI. Shared by `battle` and `guard` (0.178.4; each had a
/// copy).
pub(super) unsafe fn object_class(obj: usize) -> String {
    if obj == 0 {
        return "none".into();
    }
    let vt: usize = rd(obj);
    rtti_name(base(), vt).map(|n| n.trim_start_matches(".?AV").trim_end_matches("@@").to_string()).unwrap_or_else(|| format!("vt {vt:#x}"))
}

/// Loads `kh1-stats.json` and checks the class vtables. False when either is missing.
pub fn load(path: Option<&std::path::Path>) -> bool {
    let Some(p) = path else {
        log("stats: no kh1-stats.json (run kh1-recom-build): KH1 damage and HP are off, Re:CoM's numbers stand");
        return false;
    };
    let s = match std::fs::read_to_string(p).map_err(|e| e.to_string()).and_then(|t| crate::attack::parse_stats(&t)) {
        Ok(s) => s,
        Err(e) => {
            log(&format!("stats: {}: {e}; KH1 damage and HP are off", p.display()));
            return false;
        }
    };
    let b = base();
    let mut classes = Vec::new();
    for (class, rva) in CLASS_VTABLES {
        let vt = b + *rva as usize;
        // SAFETY: inside the mapped image (the build was checked at start).
        let name = unsafe { rtti_name(b, vt) };
        if name.as_deref() == Some(&format!(".?AV{class}@@")) {
            classes.push((vt, *class));
        } else {
            log(&format!("stats: {class}'s vtable at {vt:#x} has RTTI {name:?}; that class keeps Re:CoM's numbers"));
        }
    }
    log(&format!("stats: {} KH1 twins from {}, {} classes checked; Sora's KH1 Strength {}..{}", s.enemies.len(), p.display(), classes.len(), s.sora_strength[0], s.sora_strength[99]));
    let _ = CLASSES.set(classes);
    STATS.set(s).is_ok()
}

fn class_of(enemy: usize) -> Option<&'static str> {
    // SAFETY: an enemy object's first field is its vtable.
    // M10 / M11: a KH1 enemy's vtable copy is its class's.
    let vt: usize = super::kh1_enemy::canonical_vtable(unsafe { rd(enemy) });
    CLASSES.get()?.iter().find(|(v, _)| *v == vt).map(|(_, c)| *c)
}

fn world() -> i32 {
    // SAFETY: Re:CoM's own getter (bytes checked at start); it reads the save block.
    unsafe { super::movement::func::<IntFn>(WORLD_FN)(0) & 0xFF }
}

/// Re:CoM's current world (`0x1402339C0(0)`, 1..13 in its world table's order; 0 in Castle
/// Oblivion's halls): the KH1 enemies' world rules (M11, `kh1_enemy::kh1_world`).
pub(super) fn recom_world() -> i32 {
    world()
}

pub(super) fn riku_story() -> bool {
    // SAFETY: as above.
    unsafe { super::movement::func::<BoolFn>(RIKU_FN)() & 0xFF != 0 }
}

/// Sora's CP (`stats + 0xC`; Re:CoM's level-up CP choice adds 25), or None without the stats.
pub fn sora_cp() -> Option<i32> {
    // SAFETY: a static pointer of the image, then the stat block it points at.
    unsafe {
        let p: usize = rd(va(base(), STATS_PTR));
        (p != 0).then(|| rd::<i32>(p + 0xC))
    }
}

/// Writes Sora's CP (M12: the AP Up pick at Combat = KH1 keeps Re:CoM's CP as it was).
pub fn set_sora_cp(cp: i32) {
    // SAFETY: as above.
    unsafe {
        let p: usize = rd(va(base(), STATS_PTR));
        if p != 0 {
            wr(p + 0xC, cp);
        }
    }
}

/// Riku's DP stat (his story's stats `+0x14`: the level-up's DP row adds 2, cap 99;
/// `0x14010F70F`, Checked (disasm)), or None without the stats.
pub fn riku_dp() -> Option<i32> {
    // SAFETY: as [`sora_cp`].
    unsafe {
        let p: usize = rd(va(base(), STATS_PTR));
        (p != 0).then(|| rd::<i32>(p + 0x14))
    }
}


/// Sora's Re:CoM level (the menu's), 1 when the save isn't there.
pub fn sora_level() -> u32 {
    // SAFETY: a static pointer of the image, then the save block it points at.
    unsafe {
        let p: usize = rd(va(base(), STATS_PTR));
        if p == 0 {
            return 1;
        }
        rd::<i32>(p + 0x1C).clamp(1, 100) as u32
    }
}

/// Sora's enemy rules apply: Sora's story, or Riku's with his KH1 combat (0.198.31, fork owner,
/// 2026-11-27: his enemies' KH1 HP, Defence and records as Sora's, at Damage = KH1).
fn sora_rules() -> bool {
    !riku_story() || super::movement::riku_on()
}

fn twin(enemy: usize) -> Option<(&'static str, &'static EnemyStats)> {
    if !sora_rules() {
        return None;
    }
    twin_any(enemy)
}

/// Riku's swing at Damage = KH1 (0.198.31): KH1's damage of `kh1_row` on this enemy's twin / proxy with his
/// KH1 Strength (`crate::riku::kh1_strength` from his attack stat; Dark Riku's column) and the equipped
/// weapon's KH1 numbers with no Strength of their own (the Strength already holds his weapon). None:
/// Damage = Re:CoM, or no record (Re:CoM's number stands).
pub fn riku_kh1_hit(enemy: usize, kh1_row: usize, ap_stat: i32, dark: bool, hp_now: i32, max_mp: i32) -> Option<(i32, CapLater, i32)> {
    if !super::damage::kh1() || !riku_story() {
        return None;
    }
    let t = match twin(enemy) {
        Some((_, t)) => t.clone(),
        None => proxied(enemy)?,
    };
    let mut w = equipped_weapon();
    w.strength = 0;
    let strength = crate::riku::kh1_strength(ap_stat, dark);
    let mut g = RNG.lock().ok()?;
    let rng = g.get_or_insert_with(|| kh1_core::Kh1Rand::new(1));
    let m = super::attack::moveset();
    let (d, cap) = crate::attack::kh1_damage_cap_later(m, kh1_row, crate::attack::Sora { strength, max_mp }, 0, &w, &t, hp_now, rng)?;
    let resist = m.row(kh1_row).and_then(|r| m.attack(r.attack)).map_or(1.0, |a| resist_of(&t, a));
    Some((d.damage, cap.map(|c| (c, resist)), strength))
}

/// [`twin`] in either story (0.169.0, Riku's KH1 combat: the twin's lock radius and level for his
/// lunge and his MP charge, as Sora's; his enemies' HP and damage stay Re:CoM's through [`twin`]).
fn twin_any(enemy: usize) -> Option<(&'static str, &'static EnemyStats)> {
    if riku_story() && !super::movement::riku_on() {
        return None;
    }
    let class = class_of(enemy)?;
    let w = world();
    let t = STATS.get()?.twin(class, w)?;
    if let Ok(mut g) = LOGGED.lock() {
        if g.len() < 64 && !g.iter().any(|(c, x)| *c == class && *x == w) {
            g.push((class, w));
            let r = STATS.get().and_then(|s| s.lock_radius(class));
            log(&format!("stats: {class} in world {w}: KH1 HP {}, Defence {}, floor {} cap {}, level {}, lock radius {r:?}", t.hp, t.defence, t.min_damage, t.max_damage, t.level));
        }
    }
    Some((class, t))
}

/// The KH1 twin's defeat drops for this enemy (M8 stage 8a.4): its class's table for Re:CoM's
/// current world (format 6), with the class for the log. None: no twin, no table. 0.163.0 (fork
/// owner, 2026-11-23: Sora's features for Riku): in Riku's story too (only the drops: his enemies
/// keep Re:CoM's HP and damage).
pub fn drops(enemy: usize) -> Option<(&'static str, i32, &'static [crate::drops::Drop])> {
    let class = class_of(enemy)?;
    if riku_story() && !super::movement::riku_on() {
        return None;
    }
    let w = world();
    Some((class, w, STATS.get()?.drops(class, w)?))
}

/// Twin classes with drop tables in the stats file (for the start line).
pub fn drop_classes() -> usize {
    STATS.get().map_or(0, |s| s.drops.len())
}

/// The KH1 twin's lock-point radius (KH1 units) for this enemy, where its twin applies (KH1's
/// ground picker subtracts it from the distance, `0x1402A7320`). Assumption: the twin's actor
/// scale (KH1 actor `+0x40`) is 1.0.
pub fn lock_radius(enemy: usize) -> Option<f32> {
    let (class, _) = twin_any(enemy)?;
    STATS.get()?.lock_radius(class)
}

/// The KH1 level (record r5, KH1 stats `+0xDC`) for this enemy: its twin's (decision 21), else
/// its proxy's / boss twin's record set at its stats init. The level KH1's MP charge reads
/// (`0x1402BF330`, `super::mp`).
pub fn twin_level(enemy: usize) -> Option<i32> {
    twin_any(enemy).map(|(_, t)| t.level).or_else(|| proxied(enemy).map(|t| t.level))
}

/// Re:CoM's enemy stats init (`0x14017FF50(enemy, category, id, …)`; category 0xC for enemies,
/// id = Re:CoM's enemy id, `EN<id>`): at KH1, a twin's HP, else (Sora's story) a proxy's or a
/// boss twin's.
pub unsafe extern "win64" fn stat_init(enemy: usize, kind: i32, variant: i32, x: i32) -> u64 {
    let o: StatInitFn = orig(&ORIG_STAT_INIT);
    let r = o(enemy, kind, variant, x);
    // Boss revenge: a boss body's counter tail is set up with its stats.
    super::revenge::on_init(enemy);
    let mut g = match KH1_HP.lock() {
        Ok(g) => g,
        Err(_) => return r,
    };
    g.retain(|(e, _)| *e != enemy);
    if let Ok(mut p) = PROXIED.lock() {
        p.retain(|(e, _, _)| *e != enemy);
    }
    if r & 0xFF != 0 && super::movement::kh1_on() {
        let hp = match twin(enemy) {
            Some((class, t)) => {
                check_twin_row(enemy, class);
                Some(t.hp)
            }
            None if kind == ENEMY_KIND => proxy(enemy, variant),
            None => None,
        };
        // Damage = Re:CoM (owner, 2026-11-01): Re:CoM's own HP stands; the twin's / proxy's record
        // still gives the non-damage numbers (KH1 level for the MP charge, Stop's and Gravity's
        // resistances, drops, lock radius).
        if let Some(hp) = hp.filter(|_| super::damage::kh1()) {
            wr(enemy + 0x274, hp);
            g.push((enemy, hp));
            if g.len() > 256 {
                g.remove(0);
            }
        }
    }
    // 0.198.38: Riku's first Ansem fight (`super::story_skip::riku_fight_armed`): Ansem at 240 / 320 of Re:CoM's HP, its max
    // too (the gauge reads `max_hp`), at either Damage setting (the owner's number for this fight).
    if r & 0xFF != 0 && kind == ENEMY_KIND && variant == crate::story_skip::RIKU_ANSEM_ID && super::story_skip::riku_fight_armed() {
        let recom: i32 = rd(enemy + 0x274);
        let hp = crate::story_skip::riku_ansem_hp(recom);
        wr(enemy + 0x274, hp);
        g.retain(|(e, _)| *e != enemy);
        g.push((enemy, hp));
        super::story_skip::riku_ansem_made(enemy, recom, hp);
    }
    r
}

/// The enemy's live `@ENP` row HP (`*(enemy+0x310)`, set by the original init; before
/// Re:CoM's difficulty %) and Re:CoM's floor.
unsafe fn recom_row(enemy: usize) -> Option<(i32, i16)> {
    let row: usize = rd(enemy + 0x310);
    if row == 0 {
        return None;
    }
    Some((rd::<i32>(row), rd::<i16>(va(base(), FLOOR))))
}

/// Whether (class, world, floor) is new for the proxy log (bounded).
fn first_log(class: &str, world: i32, floor: i16) -> bool {
    let Ok(mut g) = LOGGED_PROXY.lock() else { return false };
    if g.len() >= 128 || g.iter().any(|(c, w, f)| c == class && *w == world && *f == floor) {
        return false;
    }
    g.push((class.to_string(), world, floor));
    true
}

/// Once per twin class and floor: Re:CoM's live row HP against the asset step's `@ENP` reading,
/// the check that the proxies' ratios rest on the right numbers.
unsafe fn check_twin_row(enemy: usize, class: &'static str) {
    let Some((live, floor)) = recom_row(enemy) else { return };
    let Some(file) = enp_row(floor).and_then(|r| STATS.get()?.twin_recom_hp(class, r)) else { return };
    if first_log(class, -1, floor) {
        log(&format!("stats: check {class} floor {floor}: Re:CoM row HP {live}, kh1-stats.json {file}{}", if live == file { " (ok)" } else { " (MISMATCH: the proxies' ratios are off; tell the assistant)" }));
    }
}

/// The class name of an enemy (its RTTI name without `.?AV` / `@@`).
unsafe fn class_name(enemy: usize) -> String {
    let n = rtti_name(base(), rd(enemy)).unwrap_or_default();
    n.trim_start_matches(".?AV").trim_end_matches("@@").to_string()
}

/// An enemy without an ordinary twin, in Sora's story: its KH1 HP as a proxy or boss twin
/// (`Kh1Stats::proxy`), with the record kept for its hits. None: Re:CoM's numbers stand.
unsafe fn proxy(enemy: usize, id: i32) -> Option<i32> {
    if !sora_rules() || !(0..=0xFFFF).contains(&id) {
        return None;
    }
    let stats = STATS.get()?;
    let (recom_hp, floor) = recom_row(enemy)?;
    let w = world();
    let class = class_name(enemy);
    let p = stats.proxy(&class, id as u16, w, floor, recom_hp)?;
    if first_log(&class, w, floor) {
        let t = &p.stats;
        let what = if p.own { "boss twin" } else if p.boss { "proxy boss" } else { "proxy" };
        log(&format!(
            "stats: {what} {class} (id {id}) world {w} floor {floor}: Re:CoM HP {recom_hp} -> KH1 HP {}, Defence {}, floor {} cap {}, level {}, gravity {} stop {}, ratio {:.3} (from {}){}",
            t.hp,
            t.defence,
            t.min_damage,
            t.max_damage,
            t.level,
            t.resist[8],
            t.resist[9],
            p.ratio,
            p.from,
            if super::damage::kh1() { "" } else { " [damage = recom: Re:CoM's HP stands; level and resistances used]" }
        ));
    }
    let hp = p.stats.hp;
    let what = if p.own { "boss twin" } else if p.boss { "proxy boss" } else { "proxy" };
    if let Ok(mut g) = PROXIED.lock() {
        g.push((enemy, p.stats, what));
        if g.len() > 256 {
            g.remove(0);
        }
    }
    Some(hp)
}

/// The record a proxied enemy's hits use.
fn proxied(enemy: usize) -> Option<EnemyStats> {
    PROXIED.lock().ok()?.iter().find(|(e, _, _)| *e == enemy).map(|(_, s, _)| s.clone())
}

/// Where an enemy's KH1-scale HP came from ([`kh1_scale_hps`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HpFrom {
    /// Its ordinary KH1 twin (`kh1-stats.json`).
    Twin,
    /// A proxy / proxy boss / boss twin (`Kh1Stats::proxy`; the label says which).
    Proxy(&'static str),
}

impl HpFrom {
    pub fn label(self) -> &'static str {
        match self {
            HpFrom::Twin => "KH1 twin",
            HpFrom::Proxy(what) => what,
        }
    }
}

/// M12 follow-ups (owner, "Sleights menu follow-ups": at Damage = KH1 "sleight damage scaled by the
/// enemy's KH1 max HP / its Re:CoM max HP"; then "KH1 damage scaling": "sleights also scale on
/// enemies without a KH1 twin (bosses, Re:CoM-only enemies) by their KH1-scale HP / Re:CoM HP"):
/// (its KH1-scale max HP, its Re:CoM max HP, where the first came from) for an enemy whose HP the
/// stats init set at Damage = KH1: an ordinary twin's KH1 HP, or the HP the mod already cuts a
/// Re:CoM-only enemy or boss to (a proxy's: its Re:CoM row HP × the world's ratio; a boss twin's:
/// its own KH1 record). The Re:CoM one is Re:CoM's own getter `0x14017F5A0` (its `@ENP` row HP × the
/// difficulty %), the original under our wrapper. None: the init set no KH1 HP (Damage = Re:CoM,
/// Riku's story, an enemy with no twin or proxy record, not an enemy-category object).
pub fn kh1_scale_hps(enemy: usize) -> Option<(i32, i32, HpFrom)> {
    let kh1 = KH1_HP.lock().ok()?.iter().find(|(e, _)| *e == enemy).map(|(_, hp)| *hp)?;
    let from = match twin(enemy) {
        Some(_) => HpFrom::Twin,
        None => HpFrom::Proxy(PROXIED.lock().ok()?.iter().find(|(e, _, _)| *e == enemy).map_or("proxy", |(_, _, w)| *w)),
    };
    if ORIG_MAX_HP.load(std::sync::atomic::Ordering::Acquire) == 0 {
        return None;
    }
    // SAFETY: Re:CoM's own getter (its hook installed), on a live enemy.
    let recom = unsafe { orig::<MaxHpFn>(&ORIG_MAX_HP)(enemy) };
    Some((kh1, recom, from))
}

/// An enemy's class for a log line (its RTTI name).
pub(super) fn class_label(enemy: usize) -> String {
    // SAFETY: an enemy object's first field is its vtable.
    unsafe { class_name(enemy) }
}

/// Re:CoM's max-HP getter: the KH1 HP of an enemy whose init set it.
pub unsafe extern "win64" fn max_hp(enemy: usize) -> i32 {
    if let Ok(g) = KH1_HP.lock() {
        if let Some((_, hp)) = g.iter().find(|(e, _)| *e == enemy) {
            return *hp;
        }
    }
    orig::<MaxHpFn>(&ORIG_MAX_HP)(enemy)
}

/// The loaded stats file (the Keyblade model groups read it, `models`).
pub fn all() -> Option<&'static Kh1Stats> {
    STATS.get()
}

/// The Keyblades the stats file has numbers for, by Re:CoM card kind (format 3; empty before).
pub fn keyblade_cards() -> Vec<u16> {
    STATS.get().map(|s| s.keyblades.iter().map(|k| k.card).collect()).unwrap_or_default()
}

/// A Keyblade's numbers by Re:CoM card kind.
pub fn keyblade(card: u16) -> Option<&'static crate::attack::Keyblade> {
    STATS.get()?.keyblade(card)
}

/// The equipped Keyblade at the KH1 setting (decision 25), by Re:CoM's card kind: the byte the
/// camp menu's Keyblades page writes in Re:CoM's own save block (`save + 0x2674`,
/// `keyblades::EQUIP_OFFSET`), validated (`keyblades::equipped_card`): the Kingdom Key when it is
/// 0, out of range, Soul Eater, or a Keyblade without numbers. One pointer read; Re:CoM saves,
/// loads, snapshots and zeroes the byte itself.
pub fn equipped_card() -> u16 {
    // SAFETY: a static pointer of the image, then a byte inside the 0x2AD4-byte block it points at.
    let byte = unsafe {
        let save: usize = rd(va(base(), crate::keyblades::SAVE_PTR));
        if save == 0 {
            return crate::keyblades::KINGDOM_KEY;
        }
        rd::<u8>(save + crate::keyblades::EQUIP_OFFSET)
    };
    crate::keyblades::equipped_card(byte, |c| keyblade(c).is_some())
}

/// Writes the equipped Keyblade into the save block (the Keyblades page's confirm).
pub fn set_equipped(card: u16) {
    // SAFETY: as above.
    unsafe {
        let save: usize = rd(va(base(), crate::keyblades::SAVE_PTR));
        if save != 0 {
            wr::<u8>(save + crate::keyblades::EQUIP_OFFSET, crate::keyblades::save_byte_for(card));
        }
    }
}

/// The equipped Keyblade's KH1 weapon numbers; the Kingdom Key's when the stats file has no
/// Keyblade rows (format 1–2).
pub fn equipped_weapon() -> WeaponData {
    keyblade(equipped_card()).map(|k| k.weapon.clone()).unwrap_or_else(WeaponData::kingdom_key)
}

/// A KH1 number's cap left for Re:CoM's join (Sora's damage cap, `crate::damage_cap`): (the cap in the
/// number's units, KH1's resistance for its element, which the cap carries). None: not capped.
pub type CapLater = Option<(f32, f32)>;

/// KH1's resistance (`+0x78 + element × 4`, the record's %) for an attack's element.
fn resist_of(t: &EnemyStats, data: &kh1_core::AttackData) -> f32 {
    t.resist.get((data.element & 0xF) as usize).copied().unwrap_or(100) as f32 / 100.0
}

/// KH1's damage for a spell (M7 stage 7.4: `crate::magic::spell_damage`, power + max MP) on this
/// enemy's twin or proxy, or None (Re:CoM's number stands). `hp`: the enemy's current HP (KH1's
/// for twins and proxies, `+0x274`), which Gravity's % reads (7.8; `resist[8]` is its scale).
/// Sora's damage cap: the number is made **without** the enemy's per-hit cap, which comes back
/// with it for Re:CoM's join (after every bonus, KH1's order).
pub fn kh1_spell_hit(enemy: usize, data: &kh1_core::AttackData, max_mp: i32, hp: i32) -> Option<(i32, CapLater)> {
    let t = match twin(enemy) {
        Some((_, t)) => t.clone(),
        None if sora_rules() => proxied(enemy)?,
        None => return None,
    };
    let mut resist = [1.0f32; 16];
    for (r, v) in resist.iter_mut().zip(t.resist.iter()) {
        *r = *v as f32 / 100.0;
    }
    let def = kh1_core::DefenderStats { hp: hp.max(0), max_hp: t.hp, defence: t.defence, level: t.level, min_damage: t.min_damage, max_damage: t.max_damage, resist, hp_percent_scale: resist[8], ..Default::default() };
    let mut g = RNG.lock().ok()?;
    let rng = g.get_or_insert_with(|| kh1_core::Kh1Rand::new(1));
    let (d, cap) = crate::magic::spell_damage_cap_later(data, max_mp, &def, rng);
    Some((d, cap.map(|c| (c, resist_of(&t, data)))))
}

/// KH1's damage for Sora's swing row on this enemy's twin, or None (Re:CoM's number stands).
/// `hp_now`: the enemy's current HP (KH1's for twins and proxies, `+0x274`), which the % of HP
/// rows read (Gravity Break, Zantetsuken); `max_mp`: Sora's KH1 max MP (Ripple Drive's Magic × 3).
/// `bonus`: Strength added to Strength-source records (M12 step 3, Berserk's 4, else 0;
/// `crate::attack::kh1_damage_with`). Sora's damage cap: made without the enemy's per-hit cap,
/// which comes back with it for Re:CoM's join ([`kh1_spell_hit`]).
pub fn kh1_hit_with(enemy: usize, kh1_row: usize, sora_level: u32, hp_now: i32, max_mp: i32, bonus: i32) -> Option<(i32, CapLater)> {
    // Damage = Re:CoM: Re:CoM's AP stands (`super::damage`).
    if !super::damage::kh1() {
        return None;
    }
    // A twin, else a proxy / boss twin set up at its stats init (Sora's story only).
    let t = match twin(enemy) {
        Some((_, t)) => t.clone(),
        None if sora_rules() => proxied(enemy)?,
        None => return None,
    };
    let w = equipped_weapon();
    let strength = STATS.get()?.strength_with(sora_level, &w)?;
    let mut g = RNG.lock().ok()?;
    let rng = g.get_or_insert_with(|| kh1_core::Kh1Rand::new(1));
    let m = super::attack::moveset();
    let (d, cap) = crate::attack::kh1_damage_cap_later(m, kh1_row, crate::attack::Sora { strength, max_mp }, bonus, &w, &t, hp_now, rng)?;
    let resist = m.row(kh1_row).and_then(|r| m.attack(r.attack)).map_or(1.0, |a| resist_of(&t, a));
    Some((d.damage, cap.map(|c| (c, resist))))
}

/// Damage = Re:CoM (owner, 2026-11-01): Gravity Break's and Zantetsuken's KH1 percentage of the
/// enemy's Re:CoM current HP `hp_now` (`crate::attack::kh1_percent_of_hp`: the twin's / proxy's
/// `resist[8]` and floor, else 100 %), with Sora's KH1 Strength (Gravity Break's stat term). None for
/// other rows, at Damage = KH1 (`kh1_hit_with` does it there), and in Riku's story.
/// `bonus`: Strength added (M12 step 3, Berserk: Gravity Break's stat term; else 0).
pub fn recom_percent_hit_with(enemy: usize, kh1_row: usize, sora_level: u32, hp_now: i32, max_mp: i32, bonus: i32) -> Option<i32> {
    if super::damage::kh1() || riku_story() {
        return None;
    }
    let m = super::attack::moveset();
    if !crate::damage::percent_of_hp_row(kh1_row, m) {
        return None;
    }
    let record = kh1_record(enemy);
    let w = equipped_weapon();
    let strength = STATS.get()?.strength_with(sora_level, &w)?;
    let mut g = RNG.lock().ok()?;
    let rng = g.get_or_insert_with(|| kh1_core::Kh1Rand::new(1));
    crate::attack::kh1_percent_of_hp_with(m, kh1_row, crate::attack::Sora { strength, max_mp }, bonus, &w, record.as_ref(), hp_now, rng)
}

/// M12 step 3, Berserk at Damage = Re:CoM: KH1's damage of this swing row on this enemy with and
/// without Berserk's Strength (`crate::kh1_support::berserk::share`): Sora's KH1 Strength at his
/// level with the equipped Keyblade, the enemy's KH1 record (twin / proxy / boss twin, else
/// `crate::attack::plain_record`) at its current HP. None: Berserk changes nothing for this row.
pub fn berserk_share(enemy: usize, kh1_row: usize, sora_level: u32, hp_now: i32, max_mp: i32) -> Option<(i32, i32)> {
    if riku_story() {
        return None;
    }
    let record = kh1_record(enemy).unwrap_or_else(|| crate::attack::plain_record(hp_now));
    let w = equipped_weapon();
    let strength = STATS.get()?.strength_with(sora_level, &w)?;
    crate::kh1_support::berserk::share(super::attack::moveset(), kh1_row, crate::attack::Sora { strength, max_mp }, &w, &record, hp_now)
}

/// Sora's damage cap: the enemy's KH1 record (twin, else proxy / boss twin, as [`kh1_record`]) and what
/// it is ("twin", "proxy", "proxy boss", "boss twin").
pub fn record_from(enemy: usize) -> Option<(EnemyStats, &'static str)> {
    // 0.169.0: the twin in Riku's story too (his hits' damage cap, as Sora's).
    match twin_any(enemy) {
        Some((_, t)) => Some((t.clone(), "twin")),
        None if sora_rules() => PROXIED.lock().ok()?.iter().find(|(e, _, _)| *e == enemy).map(|(_, s, w)| (s.clone(), *w)),
        None => None,
    }
}

/// Re:CoM's own max HP of an enemy (its getter `0x14017F5A0` under our wrapper: its `@ENP` row HP × the
/// difficulty %), None before the hook is in.
pub fn recom_max_hp(enemy: usize) -> Option<i32> {
    if ORIG_MAX_HP.load(std::sync::atomic::Ordering::Acquire) == 0 {
        return None;
    }
    // SAFETY: Re:CoM's own getter (its hook installed), on a live enemy.
    Some(unsafe { orig::<MaxHpFn>(&ORIG_MAX_HP)(enemy) })
}

/// The KH1 record (twin, else proxy / boss twin) of an enemy at the KH1 setting in Sora's story,
/// for the spells' status gates (7.9: Stop's `resist[9]`).
pub fn kh1_record(enemy: usize) -> Option<EnemyStats> {
    match twin(enemy) {
        Some((_, t)) => Some(t.clone()),
        None if sora_rules() => proxied(enemy),
        None => None,
    }
}
