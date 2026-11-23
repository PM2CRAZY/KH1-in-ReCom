//! The "Damage" setting at runtime (`crate::damage`; `docs/recom-kh1-attack.md` § Re:CoM damage
//! setting). Read once at start from `kh1-recom.ini` (`damage = recom | kh1`); ini only, no Config
//! row (owner, 2026-11-01).
//!
//! At Damage = Re:CoM (the default), in battle, the AP of a KH1 attack is what Re:CoM's own code
//! writes for the card / sleight it stands for, in the same player fields (`+0x660` AP, `+0x66C`
//! attribute; the caller's `+0x668` kind), followed by Re:CoM's own enemy-card rewrite
//! `0x1401E66A0`, as its row pick, `initMagic`, `initStockSkill` and `exeStockSkill` do. Re:CoM's
//! weapon (`onAttack`), its shots and its hit path then carry that AP into its damage path
//! unchanged. Nothing runs per frame: once per swing / cast / special row.
//!
//! The tables are Re:CoM's live ones (`*0x14087A808` `BattleCardList`, `*0x14087A810`
//! `StockSkillList`, set by the `CCardBattle` ctor with each battle). In the field Re:CoM's own
//! swing is cardless (`0x1401F0F70` writes card 0, AP = the attack stat), so KH1's field swings
//! and casts keep that.

use super::movement::{func, p, rd, wr};
use super::{base, log, va};
use crate::damage::{self as dm, RecomAp, Rule, Table};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};

/// Damage = KH1 (decisions 12 / 21); false = Re:CoM (the default).
static KH1: AtomicBool = AtomicBool::new(false);
/// `sites::DAMAGE_CHECKED` matched the exe.
static READY: AtomicBool = AtomicBool::new(false);
static LOG_LEFT: AtomicU32 = AtomicU32::new(48);

/// Re:CoM's enemy-card AP rewrite (`sites::DAMAGE_CHECKED`).
const REWRITE: u32 = 0x001E_66A0;
type RewriteFn = unsafe extern "win64" fn(usize, usize, usize, u32, u32);

/// `CPlayer` fields (Checked (decomp `0x1401F7520`, `0x1401F0D90`)).
mod f {
    pub const FLAGS_420: usize = 0x420;
    pub const ATTACK_STAT: usize = 0x43C;
    pub const AP: usize = 0x660;
    pub const BONUS: usize = 0x664;
    pub const KIND: usize = 0x668;
    pub const ATTR: usize = 0x66C;
}

/// Rows the live tables hold (`SY0002.BIN`: 0x7F0 / 16, 0x960 / 16; Checked (files)).
const CARD_ROWS: u16 = 0x7F;
const SLEIGHT_ROWS: u16 = 0x96;

pub(super) fn note(line: impl FnOnce() -> String) {
    if LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("damage: {}", line()));
    }
}

pub fn set_mode(kh1: bool) {
    KH1.store(kh1, Ordering::Release);
}

/// Damage = KH1: KH1's formula and the KH1 twins' / proxies' HP (decisions 12 / 21).
pub fn kh1() -> bool {
    KH1.load(Ordering::Acquire)
}

/// Checks the Re:CoM code the setting calls or repeats. False: KH1 attacks keep Re:CoM's cardless
/// AP (the attack stat) at Damage = Re:CoM.
pub unsafe fn install(b: usize) -> bool {
    for c in crate::sites::DAMAGE_CHECKED {
        if std::slice::from_raw_parts((b + c.rva as usize) as *const u8, c.expect.len()) != c.expect {
            log(&format!("damage: {} is not as expected: Re:CoM's card AP is off (cardless AP)", c.name));
            return false;
        }
    }
    READY.store(true, Ordering::Release);
    true
}

/// Damage = Re:CoM with the checked code: KH1 attacks carry Re:CoM's card / sleight AP.
fn recom_ready() -> bool {
    !kh1() && READY.load(Ordering::Acquire)
}

unsafe fn row(ptr: u64, id: u16, rows: u16) -> Option<[u8; 16]> {
    let t: usize = rd(va(base(), ptr));
    if t == 0 || id >= rows {
        return None;
    }
    Some(rd::<[u8; 16]>(t + id as usize * 16))
}

/// A card's `BattleCardList` row (the live table; None before Re:CoM's first card battle).
unsafe fn card_row(card: u16) -> Option<[u8; 16]> {
    row(dm::BATTLE_CARD_LIST, card, CARD_ROWS)
}

/// A sleight's `StockSkillList` row.
unsafe fn sleight_row(id: u16) -> Option<[u8; 16]> {
    row(dm::STOCK_SKILL_LIST, id, SLEIGHT_ROWS)
}

fn table_row(t: Table, id: u16) -> Option<[u8; 16]> {
    // SAFETY: Re:CoM's resident tables, through the pointers its ctor sets.
    unsafe {
        match t {
            Table::Card => card_row(id),
            Table::Sleight => sleight_row(id),
        }
    }
}

/// Re:CoM's writes after it computed an AP: `+0x660` the AP, `+0x66C` the attribute, then its
/// enemy-card rewrite `0x1401E66A0(player, &AP, &kind, attribute, status)` (which only changes them
/// while an enemy card of Sora's side is active, `0x14015D5F0`). The kind `+0x668` is the caller's.
/// The AP after the rewrite.
unsafe fn apply(pl: usize, a: &RecomAp) -> i32 {
    wr(pl + f::AP, a.ap);
    wr(pl + f::ATTR, a.attr);
    func::<RewriteFn>(REWRITE)(pl, pl + f::AP, pl + f::KIND, a.attr, a.status as u32);
    rd(pl + f::AP)
}

unsafe fn in_battle(pl: usize) -> bool {
    rd::<i32>(pl + p::BATTLE) != 0
}

/// A KH1 swing's AP (`win::attack::apply_row`, after its kind is written): at Damage = Re:CoM, in
/// battle, the equipped Keyblade's card at the row's column, as the row pick computes it for a card
/// (with `initAttack`'s bonus first). False: the caller's cardless AP stands.
pub unsafe fn swing(pl: usize, kh1_row: usize) -> bool {
    // Riku (`crate::riku`): Re:CoM's card AP at either Damage setting (no KH1 numbers for him), his
    // Soul Eater card's.
    let riku = super::movement::is_main_riku(pl);
    if !(recom_ready() || riku && READY.load(Ordering::Acquire)) || !in_battle(pl) {
        return false;
    }
    let Some(col) = dm::swing_column(kh1_row, super::attack::moveset()) else { return false };
    let card = if riku { crate::keyblades::SOUL_EATER } else { super::stats::equipped_card() };
    let Some(r) = card_row(card) else {
        note(|| format!("swing row {kh1_row}: Re:CoM's BattleCardList isn't loaded: cardless AP"));
        return false;
    };
    let flags: u32 = rd(pl + f::FLAGS_420);
    let bonus = dm::attack_bonus(flags, rd(pl + f::BONUS));
    wr(pl + f::BONUS, bonus);
    let stat: i32 = rd(pl + f::ATTACK_STAT);
    let a = dm::card_ap(&r, col, stat, bonus, flags & dm::HALF_UP_FLAG != 0);
    let after = apply(pl, &a);
    note(|| format!("swing row {kh1_row}: Re:CoM card {card:#x} column {col}: power {} x attack {stat} / 100 = AP {}{} (attribute {}, status {:#x})", a.power, a.ap, if after != a.ap { format!(" -> {after} (Re:CoM's enemy-card rewrite)") } else { String::new() }, a.attr, a.status));
    true
}

/// A special's row (`win::specials`): Re:CoM's sleight of the same name at the column of Re:CoM's
/// row for it (Sonic Blade 0xE start, 0xF dash, 0x10 finisher; M12 step 7, Strike Raid 0xD), as
/// `initStockSkill` / `exeStockSkill` compute it (`dm::special_source`).
pub unsafe fn special(pl: usize, recom_id: u32) -> bool {
    if !recom_ready() || !in_battle(pl) {
        return false;
    }
    let Some((sleight, col)) = dm::special_source(recom_id) else { return false };
    let Some(r) = sleight_row(sleight) else {
        note(|| format!("special {recom_id:#x}: Re:CoM's StockSkillList isn't loaded: cardless AP"));
        return false;
    };
    let stat: i32 = rd(pl + f::ATTACK_STAT);
    let a = dm::row_ap(Table::Sleight, &r, col, stat, Rule::Stat);
    let after = apply(pl, &a);
    note(|| format!("special row {recom_id:#x}: Re:CoM sleight {sleight:#x} column {col}: power {} x attack {stat} / 100 = AP {}{}", a.power, a.ap, if after != a.ap { format!(" -> {after} (rewrite)") } else { String::new() }));
    true
}

/// A KH1 cast's AP (`win::magic::start_cast`, after its kind is written): Fire / Blizzard /
/// Thunder / Gravity / Stop / Aero take their Re:CoM magic card's (level 1) or sleight's (2, 3) AP
/// and attribute, as `initMagic` / `initStockSkill` compute them; Gravity and Stop also Re:CoM's
/// kind of that card / sleight (0xD, 8: `dm::recom_kind`), which `initMagic` writes from the row's
/// `+0xB` before its rewrite. Each release reads the player's fields into its shots.
pub unsafe fn spell(pl: usize, s: u8, level: u8) -> bool {
    let done = spell_ap(pl, s, level);
    // Gravity / Stop: whether this cast's hits are Re:CoM's (its AP and kind were written); Aero:
    // whether level 1 gets the wind (`dm::aero_wind`).
    if dm::tracks_recom_cast(s) {
        let bit = 1u8 << (s & 7);
        if done {
            RECOM_CAST.fetch_or(bit, Ordering::AcqRel);
        } else {
            RECOM_CAST.fetch_and(!bit, Ordering::AcqRel);
        }
    }
    done
}

/// The status bits of the Re:CoM card / sleight a KH1 spell stands for (`+9` / `+0xB` nibbles,
/// `dm::status_bits`), from the live table (Enemy Skills route 2: the rewrite's category tests).
/// None outside a card battle's tables.
pub unsafe fn spell_status(s: u8, level: u8) -> Option<u16> {
    let src = dm::spell_source(s, level)?;
    let r = table_row(src.table, src.id)?;
    Some(dm::status_bits(r[src.table.status_at()]))
}

/// The spells (bit = spell id) whose last cast took Re:CoM's AP (and kind: Gravity, Stop; Aero).
static RECOM_CAST: AtomicU8 = AtomicU8::new(0);

unsafe fn spell_ap(pl: usize, s: u8, level: u8) -> bool {
    if !recom_ready() || !dm::recom_spell(s) || !in_battle(pl) {
        return false;
    }
    let Some(src) = dm::spell_source(s, level) else { return false };
    let Some(r) = table_row(src.table, src.id) else {
        note(|| format!("spell {s} level {level}: Re:CoM's table isn't loaded: cardless AP"));
        return false;
    };
    let kind = dm::recom_kind(s, level);
    if let Some(k) = kind {
        wr(pl + f::KIND, k);
    }
    let stat: i32 = rd(pl + f::ATTACK_STAT);
    let a = dm::row_ap(src.table, &r, src.col, stat, src.rule);
    let after = apply(pl, &a);
    let rule = match src.rule {
        Rule::Stat => format!("x attack {stat} / 100"),
        Rule::Tenth => "x 10 / 100".into(),
        Rule::Raw => "as is".into(),
    };
    note(|| {
        format!(
            "{}{level}: Re:CoM {} {:#x} (row {:#x}) column {}: power {} {rule} = AP {}{} (attribute {}{})",
            crate::magic::spell::NAMES.get(s as usize).copied().unwrap_or("?"),
            src.table.name(),
            src.id,
            src.recom_row,
            src.col,
            a.power,
            a.ap,
            if after != a.ap { format!(" -> {after} (rewrite)") } else { String::new() },
            a.attr,
            kind.map_or(String::new(), |k| format!(", Re:CoM's kind {k:#x}"))
        )
    });
    true
}

/// A KH1 spell's hits take Re:CoM's damage (no KH1 override in the enemy-hit wrapper). Gravity and
/// Stop only when their cast took Re:CoM's card AP and kind (a battle room with the live tables);
/// otherwise (the field) they keep KH1's rules, as they had no Re:CoM number to carry.
pub fn recom_spell_hits(s: u8) -> bool {
    if kh1() || !dm::recom_spell(s) {
        return false;
    }
    if dm::needs_recom_kind(s) {
        return RECOM_CAST.load(Ordering::Acquire) & (1u8 << (s & 7)) != 0;
    }
    true
}

/// Whether the spell's last cast took Re:CoM's card / sleight AP (`spell`; Gravity, Stop, Aero).
pub fn recom_cast(s: u8) -> bool {
    dm::tracks_recom_cast(s) && !kh1() && RECOM_CAST.load(Ordering::Acquire) & (1u8 << (s & 7)) != 0
}

/// Re:CoM's own gate for a spell's kind on this enemy (`dm::recom_gate_offset`: Stop `+0x2C`,
/// Gravity `+0x40` of the param block `+0x308`): Some(open), None for other spells or no param
/// block (Re:CoM's kind would then read null; it never meets one in a battle room).
pub unsafe fn recom_gate_open(enemy: usize, s: u8) -> Option<bool> {
    let off = dm::recom_gate_offset(s)?;
    let param: usize = rd(enemy + 0x308);
    if param == 0 {
        return None;
    }
    Some(rd::<i32>(param + off) != 0)
}

/// The hits trace's tag for a KH1 swing or special hit carrying `value` at Damage = Re:CoM.
pub fn swing_tag(kh1_row: usize, value: i32) -> String {
    use kh1_core::data::row;
    if let Some(col) = dm::swing_column(kh1_row, super::attack::moveset()) {
        return dm::trace_tag(value, col, Table::Card, super::stats::equipped_card());
    }
    if crate::specials::ROWS_OF[crate::specials::STRIKE_RAID].contains(&kh1_row) {
        return dm::trace_tag(value, 0, Table::Sleight, crate::specials::recom::RAID_SLEIGHT);
    }
    if crate::specials::ROWS_OF[crate::specials::RAGNAROK].contains(&kh1_row) {
        return dm::trace_tag(value, 0, Table::Sleight, crate::specials::recom::RAGNAROK_SLEIGHT);
    }
    if crate::specials::ROWS_OF[crate::specials::ARS_ARCANUM].contains(&kh1_row) {
        let id = crate::specials::recom::ID_ARCANUM_HIT + (kh1_row - kh1_core::data::row::ARS_ARCANUM_HIT) as u32;
        let (s, col) = dm::special_source(id).unwrap_or((crate::specials::recom::ARCANUM_SLEIGHT, 0));
        return dm::trace_tag(value, col, Table::Sleight, s);
    }
    let col = if kh1_row == row::SONIC_BLADE_FINISHER { dm::COL_FINISHER } else { dm::COL_NORMAL };
    dm::trace_tag(value, col, Table::Sleight, dm::sonic_blade::SLEIGHT)
}

/// The tag for a spell hit carrying `value`.
pub fn spell_tag(s: u8, level: u8, value: i32) -> String {
    match dm::spell_source(s, level) {
        Some(src) => dm::trace_tag(value, src.col, src.table, src.id),
        None => format!(":recom {value}"),
    }
}
