//! M8b: KH1's summons on Re:CoM's own summon sleights (`crate::summons`, `docs/recom-summons.md`).
//!
//! - **The command**: KH1's Summon list (`crate::command_menu`) issues a summon; the menu's step
//!   (from the request-bits wrapper, where Re:CoM's own `0x1401F8150` starts a played sleight)
//!   calls [`start`] the same frame, as KH1's command step runs `0x140289210` the frame it is
//!   issued.
//! - **The start** ([`start`]): battle, or (owner, 2026-11-04, "Sleights outside battle") a field
//!   room where its pack loads, free there (`win::field_sleights::start_room`); a live swing's weapon volume ends (as the specials
//!   do); `player+0x82C` = the summon's LV3 sleight id; then **Re:CoM's own sleight start**, Sora's
//!   vtable `+0x130` (`0x1402247A0`: its use counter, `initStockSkill`, `+0x838` = 10). Re:CoM then
//!   runs everything: its tint and "summon" effect, its resource wait (motion 0x2E), the summon
//!   actor (`CSummon*`), its hits and damage, and its end (`0x1401F6660`). KH1's MP (the owner's
//!   CP-based cost) is paid once Re:CoM's action is `0x16`.
//! - **While it runs**: our `exeStockSkill` wrapper (when the specials are in) passes Re:CoM's
//!   sleights through; Re:CoM's Battle exe steps it. The summon counts as out ([`out`]: from our
//!   start while Sora's action is `0x16` in the battle room): the menu's row 3 greyed and locked (a
//!   second sleight can't start mid-sleight), and **Sora is invincible** (owner, 2026-11-02): the
//!   damage-check wrapper (`attack::damage_check`, on Re:CoM's `0x1401E6DE0`) holds Re:CoM's own
//!   protection bit `player+0x418 & 1` for each check (its exit: the hit's value zeroed, no damage,
//!   no reaction; the roll's and Sonic Blade's path). The Battle exe runs the check every frame of
//!   action `0x16` (`0x1401F9840`, after the request bits), so it covers the start frame, the
//!   attached summons (Simba, Genie, Dumbo, Cloud) and the standing ones (Bambi, Tinker Bell,
//!   Mushu), and ends with the action: Re:CoM's end action `0x1401F6660`, its win action at a
//!   battle's end, or the room's end (the field: `player+0x390` = 0).
//! - **After it**: Re:CoM's end action leaves Sora in its own move action; our move-exe wrapper sees
//!   that its last KH1 frame no longer matches and takes Sora back fresh (`movement::continues`),
//!   as after Re:CoM's hit reactions and its win action.
//! - **No once-per-battle rule** (owner, 2026-11-02): usable again whenever the MP pays.
//! - **The learn order**: our save block (`crate::summons::BLOCK_FIRST`), synced from Re:CoM's
//!   discovery state by the unlocks wrapper on `0x14015C7F0` and at the first read of a save.
//! - **Preload**: when the Summon list's cursor lands on a summon in battle, Re:CoM's sleight
//!   resource load `0x140211790(player+0x3B8, sleight, 0)`, as Re:CoM loads a sleight's resources
//!   when its cards form it in the stock (`0x1401669F0`).
//!
//! - **M12 step 5, the Sleights list** ([`start_sleight`]): the same start ([`begin`]) for an
//!   equipped sleight ability's Re:CoM id, then the owner's MP by tier (all of it for Idyll Romp /
//!   Cross-slash+). Re:CoM's air-start sleights (`crate::sleights::AIR_START`) also start mid-air:
//!   Re:CoM's own start then takes their air version (owner, "Sleights menu follow-ups"). No invincibility of ours: Re:CoM's `exeStockSkill` sets its own protection bits
//!   where its sleight has them. While any sleight runs ([`sleight_running`]) both rows lock. The
//!   command menu's page ([`page`], 0.111.0; M12 step 5's bottom row before) is the summon block's
//!   byte `save + 0x26B6`.
//! - **0.111.0, the Party list** ([`start_party`]): the same start for a friend sleight, battle only.
//!
//! Nothing of ours runs per frame besides the menu's build (a save-block read and a few loads).

use super::movement::{func, p, rd, wr};
use super::{base, log, va};
use crate::summons::{self as sm, BLOCK_FIRST, BLOCK_LEN};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, AtomicUsize, Ordering};

static INSTALLED: AtomicBool = AtomicBool::new(false);
/// The player whose Re:CoM summon sleight is out (KH1's in-progress flag), and which summon.
static OUT: AtomicUsize = AtomicUsize::new(0);
static OUT_SUMMON: AtomicU8 = AtomicU8::new(0);
/// The hits skipped during the summon out (for the end's log line).
static SKIPPED: AtomicU32 = AtomicU32::new(0);
/// The last sleight preloaded (its Re:CoM id + 1; 0 none): a summon's LV3 or (M12 step 5) a
/// Sleights list entry.
static PRELOADED: AtomicU32 = AtomicU32::new(0);
/// M12 step 5: the player whose Sleights-list sleight is out (Re:CoM's action 0x16 from our
/// start; no invincibility of ours), and which sleight; for the end's log line.
static SLEIGHT_OUT: AtomicUsize = AtomicUsize::new(0);
static SLEIGHT_ID: AtomicU32 = AtomicU32::new(0);
/// 0.111.0: the command menu's page with no save to keep it in (`crate::command_menu::Page` byte).
static PAGE_SESSION: AtomicU8 = AtomicU8::new(0);
/// M12 follow-ups, Damage = KH1: the Sleights-list sleight or (owner, "KH1 damage scaling") the
/// summon whose hits are scaled ([`scale_sleight_hit`]; its Re:CoM sleight id + 1, 0 none): from its
/// start until the next one starts or the battle ends, so its shots still flying after Sora's
/// action ended count too.
static SLEIGHT_HITS: AtomicU32 = AtomicU32::new(0);
/// The summon actors' vtables matched (`crate::sites::SUMMON_ACTOR_PTRS`): their hits can be told.
static ACTORS_OK: AtomicBool = AtomicBool::new(false);
static SCALE_LOG_LEFT: AtomicU32 = AtomicU32::new(48);
static LOG_LEFT: AtomicU32 = AtomicU32::new(80);

/// Re:CoM's sleight start (Sora's vtable `+0x130`), the sleight resource load, the weapon's attack
/// end (`sites::SUMMON_CHECKED`).
const SLEIGHT_START: u32 = 0x0022_47A0;
/// `CPlayerRiku`'s sleight start (his vtable `+0x130`, `0x140216AB0`: ids 0x62..=0x64, 0x66..=0x6B;
/// the same writes as Sora's after its id test: `initStockSkill` through `+0x138`, `+0x838` = 10).
/// Checked (exe: disasm). `crate::riku`.
const RIKU_SLEIGHT_START: u32 = 0x0021_6AB0;
/// Dark Riku's sleight start (`CPlayerDRiku +0x130`). Checked (exe).
const DRIKU_SLEIGHT_START: u32 = 0x0020_6830;
const SLEIGHT_LOAD: u32 = 0x0021_1790;
const WEAPON_END: u32 = 0x0022_4FB0;
/// Re:CoM's sleight action (`initStockSkill` `0x1401F8D70(player, 0x16)`).
const STOCK_ACTION: u32 = 0x16;

mod f {
    /// The vtable slot of the sleight start.
    pub const VT_START: usize = 0x130;
    /// The played card record's sleight id (`+0x824` short, copied to `+0x82C` by `0x1401F5580`;
    /// `initStockSkill` and `exeStockSkill` read `+0x82C`).
    pub const SLEIGHT: usize = 0x82C;
    /// The sleight resources object (`initStockSkill`'s `0x140211790(+0x3B8, …)`).
    pub const SLEIGHTS: usize = 0x3B8;
    pub const WEAPON: usize = 0x3D0;
    /// The weapon's attack is live.
    pub const WEAPON_LIVE: usize = 0x23C;
    /// The card battle (`CCardBattle`, the friend branch's `+0x3E8 → +0x1E8`).
    pub const CARD_BATTLE: usize = 0x3E8;
    /// Sora's state bits (`+0x418`; [`super::AIR_VERSION`]).
    pub const FLAGS_418: usize = 0x418;
}

/// `initStockSkill`'s air version of a sleight started in the air (`+0x418 |= 0x800000`, only for
/// `crate::sleights::AIR_START` and the -ra / -ga tiers). Checked (decomp `0x140220DF0`).
const AIR_VERSION: u32 = 0x80_0000;

type StartFn = unsafe extern "win64" fn(usize) -> u64;
type LoadFn = unsafe extern "win64" fn(usize, u32, u32);
type PlFn = unsafe extern "win64" fn(usize);

fn note(line: impl FnOnce() -> String) {
    if LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("summons: {}", line()));
    }
}

/// Checks the Re:CoM code the summons call (no hook, no patch).
pub unsafe fn install(b: usize) -> bool {
    let have = |at: usize, n: usize| std::slice::from_raw_parts(at as *const u8, n).to_vec();
    for c in crate::sites::SUMMON_CHECKED {
        if have(b + c.rva as usize, c.expect.len()) != c.expect {
            log(&format!("summons: {} is not as expected; no KH1 summons", c.name));
            return false;
        }
    }
    for c in crate::sites::all_sites().filter(|c| crate::sites::SUMMON_ALSO.contains(&c.rva)) {
        if have(b + c.rva as usize, c.expect.len()) != c.expect {
            log(&format!("summons: {} is not as expected; no KH1 summons", c.name));
            return false;
        }
    }
    for p in crate::sites::SUMMON_PTRS {
        if rd::<usize>(b + p.rva as usize) != b + p.target as usize {
            log(&format!("summons: {} does not point at {:#x}; no KH1 summons", p.name, 0x1_4000_0000u64 + p.target as u64));
            return false;
        }
    }
    // M12 follow-ups (Damage = KH1): the summon actors' vtables, for the summons' hit scaling only.
    let bad = crate::sites::SUMMON_ACTOR_PTRS.iter().find(|p| rd::<usize>(b + p.rva as usize) != b + p.target as usize);
    match bad {
        Some(p) => log(&format!("summons: {} does not point at {:#x}; summons keep Re:CoM's damage at Damage = KH1", p.name, 0x1_4000_0000u64 + p.target as u64)),
        None => ACTORS_OK.store(true, Ordering::Release),
    }
    INSTALLED.store(true, Ordering::Release);
    true
}

pub fn installed() -> bool {
    INSTALLED.load(Ordering::Acquire)
}

pub fn start_line() -> String {
    let t: Vec<String> = sm::ALL.iter().map(|s| format!("{} -> {} (sleight {:#x}, {} MP)", s.name, s.sleight_name, s.sleight, sm::mp_from_cp(s.cp))).collect();
    format!(
        "summons: KH1's Summon list at combat = kh1 in Sora's story (M8b), in battle and (free) in field rooms with sleight packs, each Re:CoM's LV3 sleight through Sora's sleight start 0x1402247A0: {}; learn order at save+{:#x}..={:#x} (layout v{}); KH1's party rule skipped until the party exists",
        t.join(", "),
        BLOCK_FIRST,
        sm::BLOCK_LAST,
        sm::BLOCK_V1
    )
}

// ---- the save block ----

unsafe fn block_at() -> Option<usize> {
    if super::stats::riku_story() {
        return None;
    }
    let save: usize = rd(va(base(), crate::keyblades::SAVE_PTR));
    (save != 0).then_some(save + BLOCK_FIRST)
}

/// Brings the learn order up to Re:CoM's discovery state (`0x14015B3F0`), logging each summon
/// learned. Called by the unlocks wrapper when a summon card is first obtained, and by [`learned`]
/// for a save whose block was never written.
pub unsafe fn sync(why: &str) {
    if !installed() {
        return;
    }
    let Some(at) = block_at() else { return };
    // SAFETY: inside the 0x2AD4-byte save block (BLOCK_LAST < 0x2818).
    let old: [u8; BLOCK_LEN] = rd(at);
    let first = old[sm::field::VERSION] == 0;
    let (new, added) = sm::sync(old, |id| super::keyblades::ever_obtained(id));
    for i in &added {
        let s = &sm::ALL[*i];
        note(|| format!("learned {} ({}, card {:#x}){}: {why}", s.name, s.sleight_name, s.card, if first { ", an older save: KH1's index order, Cloud last" } else { "" }));
    }
    if new != old {
        wr(at, new);
    }
}

/// The learned summons in learn order, and their count (the menu's build).
pub unsafe fn learned() -> ([u8; sm::COUNT], u8) {
    let mut out = [0u8; sm::COUNT];
    if !installed() {
        return (out, 0);
    }
    let Some(at) = block_at() else { return (out, 0) };
    let mut b: [u8; BLOCK_LEN] = rd(at);
    if b[sm::field::VERSION] == 0 {
        sync("the first read of this save's summons at the KH1 setting (Re:CoM's discovery state)");
        b = rd(at);
    }
    let order = sm::order(&b);
    for (k, &i) in order.iter().enumerate() {
        out[k] = i as u8;
    }
    (out, order.len() as u8)
}

// ---- the battle ----

/// KH1's usable test's inputs from Re:CoM's side, every menu build: (battle mode, a summon out).
pub unsafe fn gate_state(pl: usize) -> (bool, bool) {
    let battle = rd::<i32>(pl + p::BATTLE) != 0;
    if !battle {
        SLEIGHT_HITS.store(0, Ordering::Relaxed);
    }
    // The preload's "already asked" mark is the room's (its PLM, new each room build): a field room
    // preloads too now (owner, 2026-11-04, "Sleights outside battle").
    let plm: usize = if pl != 0 { rd(pl + f::SLEIGHTS) } else { 0 };
    if PRELOAD_ROOM.swap(plm, Ordering::Relaxed) != plm {
        PRELOADED.store(0, Ordering::Relaxed);
    }
    (battle, out(pl))
}

/// The PLM [`PRELOADED`] belongs to.
static PRELOAD_ROOM: AtomicUsize = AtomicUsize::new(0);

/// `pl`'s Re:CoM summon sleight (ours) is out: from [`start`] while Sora's action is `0x16` (the
/// battle room, or since 2026-11-04 a field room). The first call that sees otherwise (Re:CoM's end
/// action `0x1401F6660`, its win action, another player: a new room's) ends it. Two loads unless
/// one is out.
pub unsafe fn out(pl: usize) -> bool {
    let o = OUT.load(Ordering::Relaxed);
    if o == 0 || pl == 0 {
        return false;
    }
    if o == pl && rd::<u32>(pl + p::ACTION) == STOCK_ACTION {
        return true;
    }
    if OUT.compare_exchange(o, 0, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
        let s = &sm::ALL[OUT_SUMMON.load(Ordering::Relaxed) as usize % sm::COUNT];
        let skipped = SKIPPED.swap(0, Ordering::Relaxed);
        if o == pl {
            note(|| format!("{}: Re:CoM's {} ended (action {:#x}{}); invincibility off ({skipped} hits skipped); the KH1 frame takes Sora back", s.name, s.sleight_name, rd::<u32>(pl + p::ACTION), if rd::<i32>(pl + p::BATTLE) == 0 { ", in a field room" } else { "" }));
        } else {
            note(|| format!("{}: its player is gone (another player {pl:#x}); invincibility off ({skipped} hits skipped)", s.name));
        }
    }
    false
}

/// The damage check met a hit (`+0x41C & 0x200`) on Sora while his summon is out (the check's
/// protection exit skips it): counted and logged.
pub fn hit_skipped(value: i16) {
    let n = SKIPPED.fetch_add(1, Ordering::Relaxed) + 1;
    if n <= 8 {
        note(|| format!("a hit on Sora during the summon (value {value}): skipped (Re:CoM's +0x418 bit 0: no damage, no reaction)"));
    }
}

/// Summon `i` (the menu's command, KH1's usable test passed): Re:CoM's LV3 sleight through Sora's
/// own sleight start, then KH1's MP. False (nothing changed) when it can't start.
pub unsafe fn start(pl: usize, i: usize) -> bool {
    let Some(s) = sm::ALL.get(i) else { return false };
    if !installed() || pl == 0 {
        return false;
    }
    // Owner (2026-11-04, "Sleights outside battle"): a field room too (its pack loads there), free.
    let Some(field) = super::field_sleights::start_room(pl, s.sleight, s.name) else { return false };
    let cost = if field { 0 } else { sm::mp_cost(i) };
    if !field && !super::mp::can_pay(cost * 100) {
        note(|| format!("{}: MP short ({cost})", s.name));
        return false;
    }
    let Some(before) = begin(pl, s.sleight, s.name) else { return false };
    // Owner (2026-11-03, "KH1 damage scaling": "summons get the same rule"): at Damage = KH1 its hits
    // (the summon actor's, its shots, Sora's own while it plays) scale as a sleight's.
    SLEIGHT_HITS.store(s.sleight as u32 + 1, Ordering::Relaxed);
    // KH1 pays at once, in whole points (`0x140289210` → `0x1402A4A00(actor, −cost)`); free in the
    // field.
    if !field {
        super::mp::pay(cost * 100, s.name);
    }
    OUT_SUMMON.store(i as u8, Ordering::Relaxed);
    SKIPPED.store(0, Ordering::Relaxed);
    OUT.store(pl, Ordering::Relaxed);
    note(|| format!("{} -> Re:CoM's {} (sleight {:#x}, attack {:#x}, {} from PLAYER/{:04}), {cost} MP{}, from action {before:#x}; Sora invincible until it ends", s.name, s.sleight_name, s.sleight, s.attack_id, s.class, s.pack, if field { " (in the field: free)" } else { "" }));
    true
}

/// Re:CoM's own sleight start for `sleight` (the summons' and the Sleights list's): Sora's vtable
/// `+0x130` checked to be `0x1402247A0`, a live swing's volume ended, `player+0x82C` = the id, the
/// start called as Re:CoM calls it (`0x1401F8150`), and Re:CoM's action `0x16` checked after it.
/// Some(the action before) when it started; None (logged) when it refused.
unsafe fn begin(pl: usize, sleight: u16, name: &str) -> Option<u32> {
    // Sora's own vtable slot, as Re:CoM calls it (`0x1401F8150`).
    let vt: usize = rd(pl);
    let slot: usize = if vt != 0 { rd(vt + f::VT_START) } else { 0 };
    // Riku's own start for every sleight of his (his six, and the King's in his Party list).
    let riku = super::movement::is_main_riku(pl);
    let want = if super::movement::is_main_dark_riku(pl) {
        DRIKU_SLEIGHT_START
    } else if riku {
        RIKU_SLEIGHT_START
    } else {
        SLEIGHT_START
    };
    if slot != base() + want as usize {
        note(|| format!("{name}: the player's vtable +0x130 is {slot:#x}, not Re:CoM's sleight start"));
        return None;
    }
    // A live swing's volume ends first (the specials' rule; Re:CoM's own card chain ends the
    // weapon's attack before the next card, `0x1401F9520`).
    let w: usize = rd(pl + f::WEAPON);
    if w != 0 && rd::<i32>(w + f::WEAPON_LIVE) != 0 {
        func::<PlFn>(WEAPON_END)(w);
    }
    let before: u32 = rd(pl + p::ACTION);
    wr(pl + f::SLEIGHT, sleight as u32);
    let r = std::mem::transmute::<usize, StartFn>(slot)(pl);
    let now: u32 = rd(pl + p::ACTION);
    if r & 0xFF == 0 || now != STOCK_ACTION {
        note(|| format!("{name}: Re:CoM's sleight start refused sleight {sleight:#x} (returned {r}, action {now:#x})"));
        return None;
    }
    Some(before)
}

/// The Summon list's cursor rests on a summon (its LV3 sleight) or (M12 step 5) a Sleights list
/// entry: Re:CoM's sleight resource load `0x140211790(player+0x3B8, id, 0)` for `id` once per landing
/// (`None`: nothing under the cursor), in battle, as Re:CoM loads a sleight's resources when its
/// cards form it in the stock (`0x1401669F0`). The summons' and (M12 step 5) the Sleights list's.
pub unsafe fn preload_sleight(pl: usize, id: Option<u16>) {
    let tag = id.map_or(0, |id| id as u32 + 1);
    if PRELOADED.swap(tag, Ordering::Relaxed) == tag || !installed() {
        return;
    }
    let Some(id) = id else { return };
    if !preload_room(pl) {
        return;
    }
    let res: usize = rd(pl + f::SLEIGHTS);
    if res != 0 {
        func::<LoadFn>(SLEIGHT_LOAD)(res, id as u32, 0);
    }
}

/// M12 step 6: the L1 shortcut list came on top: Re:CoM's sleight resource load for each sleight
/// its slots hold (at most three, once per time the list comes up), in battle, as
/// [`preload_sleight`] does for the Sleights list's cursor. Re:CoM's load returns at once for a
/// pack already in its 16-entry table (`0x140211790`'s first loop). Checked (decomp).
pub unsafe fn preload_sleights(pl: usize, ids: &[u16]) {
    if !installed() || ids.is_empty() || !preload_room(pl) {
        return;
    }
    let res: usize = rd(pl + f::SLEIGHTS);
    if res == 0 {
        return;
    }
    for &id in ids.iter().take(3) {
        func::<LoadFn>(SLEIGHT_LOAD)(res, id as u32, 0);
    }
}

/// A room where Re:CoM loads sleight packs for us: battle, or (owner, 2026-11-04, "Sleights outside
/// battle") a field room whose PLM loads them (`win::field_sleights::packs_on`). In any other field
/// room a load would queue a file Re:CoM's step never finishes: none is asked.
unsafe fn preload_room(pl: usize) -> bool {
    pl != 0 && (rd::<i32>(pl + p::BATTLE) != 0 || (super::field_sleights::field_on(pl) && super::field_sleights::packs_on(pl)))
}

/// A battle starts (its manager's ctor, before its Sora's first frame): nothing of ours can still
/// be running from the field (owner, 2026-11-04: a field sleight its strike cut); the next check
/// would see a new player anyway.
pub fn forget_running() {
    OUT.store(0, Ordering::Relaxed);
    SLEIGHT_OUT.store(0, Ordering::Relaxed);
}

// ---- M12 step 5: the Sleights row and list; 0.111.0: the pages ----

/// The command menu's page (`crate::command_menu::Page`): the save's choice (the summon block's
/// byte `save + 0x26B6`, which held M12 step 5's bottom row: those saves read page 1), or this
/// session's without a save. A one-byte read.
pub unsafe fn page() -> crate::command_menu::Page {
    let b = match block_at() {
        Some(at) => rd::<u8>(at + sm::field::PAGE),
        None => PAGE_SESSION.load(Ordering::Relaxed),
    };
    crate::command_menu::Page::of_byte(b)
}

/// D-pad Left switched the page: kept in the save (read by every later frame; written only on the
/// press), else for the session.
pub unsafe fn set_page(pg: crate::command_menu::Page) {
    PAGE_SESSION.store(pg.byte(), Ordering::Relaxed);
    let saved = match block_at() {
        Some(at) => {
            // SAFETY: inside the 0x2AD4-byte save block (BLOCK_LAST < 0x2818).
            wr::<u8>(at + sm::field::PAGE, pg.byte());
            true
        }
        None => false,
    };
    note(|| format!("page -> {pg:?} ({})", if saved { "kept in the save at save+0x26B6" } else { "no save: this session only" }));
}

/// Any Re:CoM sleight is running for `pl` (its action `0x16`, any room: a summon, a
/// Sleights-list sleight, or Sonic Blade's): the Summon and Sleights rows lock (one at a time). The
/// first call that sees ours end logs it. Two loads.
pub unsafe fn sleight_running(pl: usize) -> bool {
    // Any room (owner, 2026-11-04: sleights in the field too; Re:CoM's action 0x16 is busy there as
    // in battle).
    let running = pl != 0 && rd::<u32>(pl + p::ACTION) == STOCK_ACTION;
    let o = SLEIGHT_OUT.load(Ordering::Relaxed);
    if o != 0 && !(o == pl && running) && SLEIGHT_OUT.compare_exchange(o, 0, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
        let id = SLEIGHT_ID.load(Ordering::Relaxed) as u16;
        note(|| format!("sleight {} ({id:#x}): Re:CoM's sleight ended (action {:#x}); the KH1 frame takes Sora back", crate::sleights::name(id), if pl != 0 { rd::<u32>(pl + p::ACTION) } else { 0 }));
        // Enemy Skills stage 5: Marluxia's second play waits for the next frame.
        super::enemy_skills_deck::sleight_ended(o, id);
    }
    running
}

/// Owner, 2026-11-03 (`crate::sleights::dim_shield`): Sora's sleight has the room dimmed. Three
/// loads, only when a hit arrives (the damage check's wrapper).
pub unsafe fn sleight_dimmed(pl: usize) -> bool {
    pl != 0 && crate::sleights::dim_shield(rd(pl + f::FLAGS_418), rd(pl + p::ACTION), super::specials::kh1_special(pl), rd::<i32>(pl + p::BATTLE) != 0)
}

/// A hit on Sora skipped by [`sleight_dimmed`]: one log line each, at most 40 a session.
pub fn dim_hit_skipped(value: i16, pl: usize) {
    static LEFT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(40);
    if LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        // SAFETY: the player the damage check runs for.
        let action = unsafe { rd::<u32>(pl + p::ACTION) };
        log(&format!("sleight dim: a hit of {value} on Sora skipped (his sleight has the room dimmed; action {action:#x})"));
    }
}

/// M12 step 5: a Sleights list entry (the menu's command, the usable test passed): Re:CoM's own
/// sleight start for `id` (as a summon's, [`begin`]), then the owner's MP (KH1's whole-point
/// payment `0x1402A5B60`: the tier's points, or all of Sora's whole MP). Re:CoM runs the rest: its
/// motion, effects, hits and damage, its own protection bits (`+0x418`, set by `exeStockSkill`
/// where its sleights have them; nothing of ours), its end. False (nothing changed) when it can't.
pub unsafe fn start_sleight(pl: usize, id: u16) -> bool {
    let name = crate::sleights::name(id);
    if !installed() || pl == 0 {
        return false;
    }
    // Owner (2026-11-04, "Sleights outside battle"): a field room too, free.
    let Some(field) = super::field_sleights::start_room(pl, id, name) else { return false };
    let mp = super::mp::current().map_or(0, |m| m.0);
    let points = if field {
        0
    } else {
        let Some(points) = crate::sleights::pay_points(id, mp) else {
            note(|| format!("sleight {name}: MP short ({:?}, MP {mp})", crate::sleights::cost(id)));
            return false;
        };
        points
    };
    let Some(before) = begin(pl, id, name) else { return false };
    if !field {
        super::mp::pay(points * 100, name);
    }
    SLEIGHT_ID.store(id as u32, Ordering::Relaxed);
    SLEIGHT_OUT.store(pl, Ordering::Relaxed);
    SLEIGHT_HITS.store(id as u32 + 1, Ordering::Relaxed);
    // M12 follow-ups (owner, "Sleights menu follow-ups"): an air-start sleight started mid-air takes
    // Re:CoM's air version (`initStockSkill` sets `+0x418 & 0x800000`; `crate::sleights::AIR_START`).
    let air = rd::<u32>(pl + f::FLAGS_418) & AIR_VERSION != 0;
    note(|| format!("sleight {name} -> Re:CoM's sleight {id:#x} through Sora's sleight start{}{}, {points} MP ({:?}), from action {before:#x}; Re:CoM's own protection only", if air { " (in the air: Re:CoM's air version)" } else { "" }, if field { " in the field (free; a hit on a field Heartless is Re:CoM's field strike)" } else { "" }, crate::sleights::cost(id).unwrap_or(crate::sleights::Cost::Points(0))));
    true
}

/// Enemy Skills stage 5 (Marluxia's Double Sleight): Re:CoM's own sleight start for `id` again, free
/// (Re:CoM's Double Sleight plays the second without a second payment), with the Sleights list's
/// marks. False when Re:CoM's start refused it.
pub unsafe fn replay_sleight(pl: usize, id: u16) -> bool {
    let name = crate::sleights::name(id);
    if !installed() || pl == 0 {
        return false;
    }
    let Some(before) = begin(pl, id, name) else { return false };
    SLEIGHT_ID.store(id as u32, Ordering::Relaxed);
    SLEIGHT_OUT.store(pl, Ordering::Relaxed);
    SLEIGHT_HITS.store(id as u32 + 1, Ordering::Relaxed);
    note(|| format!("sleight {name} -> Re:CoM's sleight {id:#x} again (Marluxia's Double Sleight, free), from action {before:#x}"));
    true
}

/// 0.111.0: a Party list entry (or a shortcut holding one): Re:CoM's own sleight start for its
/// friend sleight, as [`start_sleight`] (its MP: `crate::sleights::pay_points` gives the Party
/// tier's), **in a battle room only** whatever the Sleights list allows: Re:CoM's friend branch of
/// `exeStockSkill` reads the card battle's active card (`player+0x3E8 → +0x1E8`) without a null
/// test, and the friend's pack only finishes loading in battle (`docs/recom-field-sleights.md`).
pub unsafe fn start_party(pl: usize, id: u16) -> bool {
    if crate::party::index(id).is_none() || pl == 0 || rd::<i32>(pl + p::BATTLE) == 0 {
        note(|| format!("party {}: not in battle (friend sleights are battle-only)", crate::sleights::name(id)));
        return false;
    }
    if rd::<usize>(pl + f::CARD_BATTLE) == 0 {
        note(|| format!("party {}: no card battle object (player+0x3E8): not started", crate::sleights::name(id)));
        return false;
    }
    start_sleight(pl, id)
}

/// 0.111.0: a friend actor of the PLM (`player+0x3B8`; `0x14020E950` keeps the friend it made for
/// the sleight at `+0x70`, Wild Crush's second at `+0x78`). Checked (decomp).
unsafe fn is_our_friend(pl: usize, obj: usize) -> bool {
    let plm: usize = rd(pl + f::SLEIGHTS);
    obj != 0 && plm != 0 && (rd::<usize>(plm + PLM_FRIEND) == obj || rd::<usize>(plm + PLM_FRIEND + 8) == obj)
}

/// The PLM's friend actor slots (`0x14020E950`). Checked (decomp).
const PLM_FRIEND: usize = 0x70;

// ---- M12 follow-ups: a sleight's damage at Damage = KH1 ----

/// An actor's id (Re:CoM's `+0x20`: a collision's owner `+0x20` and a bullet's shooter `+0x238` hold
/// it; `0x1402AD350` resolves it). Checked (decomp `0x14016F1C0`, `0x140158DD0`).
const ACTOR_ID: usize = 0x20;
/// The hurt body's hit fields (written by Re:CoM's hit apply `0x14016F1C0`): its Re:CoM kind, value,
/// owner id. Checked (decomp).
const BODY_KIND: usize = 0x80;
const BODY_VALUE: usize = 0x92;
const BODY_OWNER: usize = 0x98;

/// Whose a waiting hit is, for [`scale_sleight_hit`].
enum Owner {
    /// Sora (or his weapon) while the Sleights-list sleight or our summon runs.
    Sora,
    /// A Re:CoM shot (`CBullet` / `CHomingBullet`) Sora or his summon fired (its shooter `+0x238`).
    Shot,
    /// Sora's summon actor (`crate::summons::ACTOR_VTABLES`, its player `+0x248` = Sora): a summon's
    /// own hits, and the summon-type sleights' (Idyll Romp: Bambi, Cross-slash+: Cloud).
    Summon,
    /// Anything else (an enemy's, a summon's, an effect's): Re:CoM's value stands. The object's
    /// vtable RVA for the log (0: the id doesn't resolve).
    Other(usize),
}

/// Whether `obj` is one of Re:CoM's summon actors summoned for `pl` (its vtable one of
/// `crate::summons::ACTOR_VTABLES`, checked at install; its player `+0x248`).
unsafe fn is_our_summon(pl: usize, obj: usize) -> bool {
    if obj == 0 || !ACTORS_OK.load(Ordering::Acquire) {
        return false;
    }
    let vt = rd::<usize>(obj).wrapping_sub(base());
    sm::ACTOR_VTABLES.iter().any(|&(_, v, _)| v as usize == vt) && rd::<usize>(obj + sm::ACTOR_PLAYER) == pl
}

/// Owner, 2026-11-04 ("some hits from the lethal frame also arent hitting, same goes for the summons at
/// times"; trace 21 / 22: the enemies in an attack, `+0x260` bit 0, took none of the sleight's hits):
/// the hit waiting on `enemy` is the running Sleights-list sleight's or summon's (Sora's own collision
/// while it runs, a Re:CoM shot he or his summon fired, his summon actor), as [`scale_sleight_hit`]
/// counts them. Re:CoM's hit handler `0x14017F7E0` drops a hit on an enemy in an attack unless the
/// hit carries the card-interrupt bit `0x2000000` (in Re:CoM the sleight's cards break the enemy's
/// card first; at KH1 neither side has one). The caller gives that hit the bit, as a KH1 swing's.
pub unsafe fn soras_sleight_hit(enemy: usize) -> bool {
    if SLEIGHT_OUT.load(Ordering::Relaxed) == 0 && OUT.load(Ordering::Relaxed) == 0 {
        return false;
    }
    let Some(body) = super::attack::waiting_body(enemy) else { return false };
    let pl: usize = rd(va(base(), crate::sites::data::MAIN_PLAYER));
    if pl == 0 {
        return false;
    }
    matches!(owner_of(pl, rd(body + BODY_OWNER)), Owner::Sora | Owner::Shot | Owner::Summon)
}

/// A sleight's or summon's hit given the card-interrupt bit: one log line each, at most 40 a session.
pub fn interrupt_note(enemy: usize) {
    static LEFT: AtomicU32 = AtomicU32::new(40);
    if LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        let id = SLEIGHT_ID.load(Ordering::Relaxed) as u16;
        let who = super::stats::class_label(enemy);
        let what = if OUT.load(Ordering::Relaxed) != 0 { "summon".to_string() } else { format!("sleight {}", crate::sleights::name(id)) };
        log(&format!("{what}: a hit on {who} during its attack lands (card-interrupt bit, as a KH1 swing's)"));
    }
}

unsafe fn owner_of(pl: usize, owner: u32) -> Owner {
    let b = base();
    let me: u32 = rd(pl + ACTOR_ID);
    let running = (SLEIGHT_OUT.load(Ordering::Relaxed) == pl || OUT.load(Ordering::Relaxed) == pl) && rd::<u32>(pl + p::ACTION) == STOCK_ACTION;
    if owner == me && running {
        return Owner::Sora;
    }
    let handle = super::movement::HANDLE_OBJ;
    let obj = func::<super::movement::HandleFn>(handle)(owner);
    if obj == 0 {
        return Owner::Other(0);
    }
    let weapon: usize = rd(pl + f::WEAPON);
    if (obj == pl || (weapon != 0 && obj == weapon)) && running {
        return Owner::Sora;
    }
    // 0.111.0: a Party sleight's friend (its hits are the sleight's, as a summon's).
    if is_our_summon(pl, obj) || (SLEIGHT_OUT.load(Ordering::Relaxed) == pl && crate::party::index(SLEIGHT_ID.load(Ordering::Relaxed) as u16).is_some() && is_our_friend(pl, obj)) {
        return Owner::Summon;
    }
    let vt: usize = rd(obj);
    if vt == va(b, crate::guard::bullet::VTABLE) || vt == va(b, crate::guard::bullet::HOMING_VTABLE) {
        let shooter: u32 = rd(obj + crate::guard::bullet::SHOOTER);
        // Sora's own shot, or his summon's (Re:CoM writes the summon's id: type 0x1000). A summon
        // already gone when its shot lands doesn't resolve: Re:CoM's damage (logged as other).
        if shooter == me || (shooter != 0 && is_our_summon(pl, func::<super::movement::HandleFn>(handle)(shooter))) {
            return Owner::Shot;
        }
    }
    Owner::Other(vt.wrapping_sub(b))
}

/// Sora's damage cap (owner, 2026-11-05: "Sleights-list sleights, LV3 summons and Party (friend)
/// sleights"): the hurt body holding the hit waiting on `enemy` and the sleight's name, when that hit
/// is the current Sleights-list sleight's, Party sleight's or summon's by [`scale_sleight_hit`]'s own
/// rule (Sora's collision while it runs, his summon or friend actor, a Re:CoM shot he or his summon
/// fired), at either Damage setting. One atomic load while none was used this battle.
pub unsafe fn counted_hit(enemy: usize) -> Option<(usize, &'static str)> {
    let cur = SLEIGHT_HITS.load(Ordering::Relaxed);
    if cur == 0 {
        return None;
    }
    let body = super::attack::waiting_body(enemy)?;
    let pl: usize = rd(va(base(), crate::sites::data::MAIN_PLAYER));
    if pl == 0 {
        return None;
    }
    match owner_of(pl, rd(body + BODY_OWNER)) {
        Owner::Sora | Owner::Shot | Owner::Summon => Some((body, sm::hit_name((cur - 1) as u16))),
        Owner::Other(_) => None,
    }
}

fn scale_note(line: impl FnOnce() -> String) {
    if SCALE_LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("summons: {}", line()));
    }
}

/// M12 follow-ups (owner, 2026-11-03, "Sleights menu follow-ups": "at Damage = KH1 sleight damage
/// scaled by the enemy's KH1 max HP / its Re:CoM max HP"): in the enemy-hit wrapper, just before
/// Re:CoM's handler `0x14017F7E0` takes a hit that is neither a KH1 swing's nor a KH1 spell's: when it
/// is the current Sleights-list sleight's or (owner, "KH1 damage scaling": "summons get the same
/// rule") summon's ([`SLEIGHT_HITS`]: Sora's own collision while it runs, Sora's summon actor, or a
/// Re:CoM shot Sora or his summon fired), Re:CoM's value on the hurt body its gather takes (`+0x92`)
/// becomes Re:CoM's × (the enemy's KH1-scale max HP / its Re:CoM max HP) (`crate::sleights::kh1_scaled`;
/// damage kinds only). Re:CoM's handler then applies its resistance, weak part, frozen bonus and boss
/// falloff to it as to any hit. The KH1-scale HP (`win::stats::kh1_scale_hps`): a twin's KH1 HP, or
/// (owner, "KH1 damage scaling") the proxy's / boss twin's HP the mod already cuts a Re:CoM-only enemy
/// or boss to. An enemy whose HP the init didn't set keeps Re:CoM's value (logged). Damage = Re:CoM:
/// nothing. One atomic load while no Sleights-list sleight or summon has been used this battle.
pub unsafe fn scale_sleight_hit(enemy: usize) {
    let cur = SLEIGHT_HITS.load(Ordering::Relaxed);
    if cur == 0 || !super::damage::kh1() {
        return;
    }
    let Some(body) = super::attack::waiting_body(enemy) else { return };
    let pl: usize = rd(va(base(), crate::sites::data::MAIN_PLAYER));
    if pl == 0 {
        return;
    }
    let id = (cur - 1) as u16;
    let name = sm::hit_name(id);
    let owner: u32 = rd(body + BODY_OWNER);
    match owner_of(pl, owner) {
        Owner::Sora | Owner::Shot | Owner::Summon => {}
        Owner::Other(vt) => {
            // Only worth a line while the sleight or summon itself plays (an enemy's hits are the
            // common case).
            if (SLEIGHT_OUT.load(Ordering::Relaxed) == pl || OUT.load(Ordering::Relaxed) == pl) && vt != 0 && vt != rd::<usize>(enemy).wrapping_sub(base()) {
                scale_note(|| format!("sleight {name}: a hit on {} by an object we don't count as Sora's (vtable {:#x}, owner {owner:#x}): Re:CoM's damage", super::stats::class_label(enemy), 0x1_4000_0000u64 + vt as u64));
            }
            return;
        }
    }
    let kind: u32 = rd(body + BODY_KIND);
    let value: i16 = rd(body + BODY_VALUE);
    let Some((kh1, recom, from)) = super::stats::kh1_scale_hps(enemy) else {
        scale_note(|| format!("sleight {name}: {} has no KH1-scale HP (no twin, proxy or boss twin set it): Re:CoM's hit {value} (kind {kind:#x}) stands", super::stats::class_label(enemy)));
        return;
    };
    let Some(v) = crate::sleights::kh1_scaled(value, kind, kh1, recom) else { return };
    wr(body + BODY_VALUE, v);
    scale_note(|| format!("sleight {name} on {}: Re:CoM's hit {value} (kind {kind:#x}) -> {v} (KH1-scale max HP {kh1} ({}) / Re:CoM max HP {recom})", super::stats::class_label(enemy), from.label()));
}
