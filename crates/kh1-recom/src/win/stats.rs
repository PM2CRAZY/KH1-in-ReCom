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
//!   twin's Defence / floor / cap / resistance. Re:CoM's own path then applies its extras (weak
//!   part, resistance %, boss falloff; decision 13).

use super::movement::{orig, rd, wr};
use super::{base, log, va};
use crate::attack::{EnemyStats, Kh1Stats};
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

/// Re:CoM's current world (`0x1402339C0(0)`: the world chosen for the current floor, 0 none,
/// 1 Traverse Town … 13 Castle Oblivion, the order of the name table `0x140506AA0`). Checked
/// (decomp + exe). Not `0x14087B844`, which is the floor (the enemy's per-floor row).
const WORLD_FN: u32 = 0x0023_39C0;
/// Riku's story (`0x1401AAAE0`: byte variable 0x60 ≠ 0). Checked (exe). Riku's story stays
/// Re:CoM's (decision 6).
const RIKU_FN: u32 = 0x001A_AAE0;
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
unsafe fn rtti_name(b: usize, vt: usize) -> Option<String> {
    let col: usize = rd(vt - 8);
    if col < b || rd::<u32>(col) != 1 {
        return None;
    }
    let td = b + rd::<u32>(col + 0xC) as usize;
    let name = std::slice::from_raw_parts((td + 0x10) as *const u8, 64);
    let end = name.iter().position(|&c| c == 0)?;
    Some(String::from_utf8_lossy(&name[..end]).into_owned())
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
    let vt: usize = unsafe { rd(enemy) };
    CLASSES.get()?.iter().find(|(v, _)| *v == vt).map(|(_, c)| *c)
}

fn world() -> i32 {
    // SAFETY: Re:CoM's own getter (bytes checked at start); it reads the save block.
    unsafe { super::movement::func::<IntFn>(WORLD_FN)(0) & 0xFF }
}

fn riku_story() -> bool {
    // SAFETY: as above.
    unsafe { super::movement::func::<BoolFn>(RIKU_FN)() & 0xFF != 0 }
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

fn twin(enemy: usize) -> Option<(&'static str, &'static EnemyStats)> {
    let class = class_of(enemy)?;
    if riku_story() {
        return None;
    }
    let w = world();
    let t = STATS.get()?.twin(class, w)?;
    if let Ok(mut g) = LOGGED.lock() {
        if g.len() < 64 && !g.iter().any(|(c, x)| *c == class && *x == w) {
            g.push((class, w));
            log(&format!("stats: {class} in world {w}: KH1 HP {}, Defence {}, floor {} cap {}, level {}", t.hp, t.defence, t.min_damage, t.max_damage, t.level));
        }
    }
    Some((class, t))
}

/// Re:CoM's enemy stats init (`0x14017FF50(enemy, type, variant, …)`): at KH1, a twin's HP.
pub unsafe extern "win64" fn stat_init(enemy: usize, kind: i32, variant: i32, x: i32) -> u64 {
    let o: StatInitFn = orig(&ORIG_STAT_INIT);
    let r = o(enemy, kind, variant, x);
    let mut g = match KH1_HP.lock() {
        Ok(g) => g,
        Err(_) => return r,
    };
    g.retain(|(e, _)| *e != enemy);
    if r & 0xFF != 0 && super::movement::kh1_on() {
        if let Some((_, t)) = twin(enemy) {
            wr(enemy + 0x274, t.hp);
            g.push((enemy, t.hp));
            if g.len() > 256 {
                g.remove(0);
            }
        }
    }
    r
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

/// KH1's damage for Sora's swing row on this enemy's twin, or None (Re:CoM's number stands).
pub fn kh1_hit(enemy: usize, kh1_row: usize, sora_level: u32) -> Option<i32> {
    let (_, t) = twin(enemy)?;
    let strength = STATS.get()?.strength(sora_level)?;
    let mut g = RNG.lock().ok()?;
    let rng = g.get_or_insert_with(|| kh1_core::Kh1Rand::new(1));
    let d = crate::attack::kh1_damage(super::attack::moveset(), kh1_row, strength, t, rng)?;
    Some(d.damage)
}
