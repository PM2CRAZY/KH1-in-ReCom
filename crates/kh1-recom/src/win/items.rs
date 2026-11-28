//! KH1's items inside Re:CoM's own item action `0xE` (M8 stage 8a.1; `crate::items`,
//! `docs/recom-m8-items.md`).
//!
//! - **Store:** Sora's slots and stock in our save block's items extension (`save + 0x2685 ..=
//!   0x269E`, `crate::items::EXT_FIRST`), read and written only when something happens (the menu
//!   reads it each frame it runs). Re:CoM saves, checksums, loads, snapshots and zeroes it with the
//!   rest of its save block.
//! - **Command:** the command menu's Items list calls [`issue`]; Sora's KH1 frame of the same frame
//!   takes it (`crate::items::Kh1Sora::command_item`), else it is dropped at the next frame.
//! - **Start** ([`start_item`]): KH1's `0x1402C2760`: the item leaves its slot and the use task
//!   starts; then `initItem`'s (`0x1401F28F0`) own writes, cardless (`initItem` reads the played
//!   card's `BattleCardList` row, loaded only for a card battle): Re:CoM's action `0xE`.
//! - **Each frame** of the use motion: Re:CoM's Battle exe calls `exeItem` (`0x1401F9A13`), the
//!   Field exe through a new case (`crate::items::field_case`); our hook on `exeItem` runs KH1's
//!   frame for a KH1 item (Re:CoM plays its item clip, row 0x2D) and Re:CoM's `exeItem` for
//!   anything else. Re:CoM's card effect (`0x14016C910`, the reload) never runs for a KH1 item.
//! - **The use task** ([`task_frame`], from the request-bits wrapper: KH1 runs it after the actors,
//!   priority 139999): KH1's waits (34, then 30), then the heal: Re:CoM's own item effect (`exeItem`'s
//!   switch) on Sora and Re:CoM's heal `0x1401E59A0`; MP through `super::mp::add_points`. Sora dead
//!   before the heal: the item goes back (`0x140284E60`).

use super::movement::{self as mv, func, orig, p, rd, wr};
use super::{base, log, va};
use crate::items::{self as it, EffectSet, Inventory, ItemCommand, ItemEvent, Picked, TaskStep, UseTask};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

pub static ORIG_EXE_ITEM: AtomicUsize = AtomicUsize::new(0);

static INSTALLED: AtomicBool = AtomicBool::new(false);
/// The player whose item action is a KH1 use (set by [`start_item`], cleared by [`end_item`]).
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
/// The command issued this frame (the menu's), for the KH1 frame.
static PENDING: Mutex<Option<ItemCommand>> = Mutex::new(None);
/// KH1's use tasks running (KH1 can run more than one: a use cut by a hit frees Sora before its
/// heal), and a flag the per-frame check reads.
static TASKS: Mutex<Vec<Run>> = Mutex::new(Vec::new());
static TASKS_ON: AtomicBool = AtomicBool::new(false);
/// Re:CoM's effect set names (`always`, `itemplus`) were found where `exeItem` reads them.
static EFFECT_NAMES_OK: AtomicBool = AtomicBool::new(false);
/// `kh1_test_items` (debug).
static TEST_ITEMS: AtomicBool = AtomicBool::new(false);
static LOG_LEFT: AtomicU32 = AtomicU32::new(80);
static EVENTS: Mutex<Vec<String>> = Mutex::new(Vec::new());

struct Run {
    player: usize,
    task: UseTask,
}

/// Re:CoM's effect set names `exeItem` passes (pointers to `"always"` / `"itemplus"`). Checked
/// (disasm `0x1401ED85F`, `0x1401ED8ED`; exe: the strings at `0x14052A3E8` / `0x1405A0128`).
const ALWAYS_NAME_PTR: u64 = 0x1_407C_5DD8;
const ITEMPLUS_NAME_PTR: u64 = 0x1_407C_5DE0;
/// Re:CoM's heal (`0x1401E59A0(player, amount, 0, 1, 0)`, as the Cure case calls it) and end action
/// (`0x1401F6660`), Re:CoM's ground test. Checked by earlier stages.
const HEAL: u32 = 0x001E_59A0;
const END_ACTION: u32 = 0x001F_6660;
const GROUND_TEST: u32 = 0x001F_3C00;
/// Re:CoM's base gravity (`initItem` writes it to `+0x5D8`). Checked (disasm `0x1401F2988`).
const BASE_GRAVITY: u64 = 0x1_4087_B398;
/// Re:CoM's current player (`[0x14087B390]`, set by the `CPlayer` constructor; the heal
/// `0x1401E59A0` and the stats sync `0x1401F0120` read it). Checked (decomp + exe: the sync's
/// `mov rax,[rip+0x68B265]` at `0x1401F0124`).
const PLAYER: u64 = 0x1_4087_B390;
/// Re:CoM's player → save stats copy (HP, max HP, AP, EXP, Moogle points), which the camp runs as
/// it opens (`0x1400D9C80` state 2). Checked (decomp; `sites::ITEM_PAGE_CALLEES`).
const STATS_SYNC: u32 = 0x001F_0120;
/// Sora's HP (`+0x42C`, max `+0x430`) and Re:CoM's death flag (`+0x7B0`). Checked (decomp).
const HP: usize = 0x42C;
const HP_MAX: usize = 0x430;
const DEAD: usize = 0x7B0;

/// `CPlayer` fields `initItem` writes (Checked (disasm `0x1401F28F0..0x1401F29D8`)).
mod f {
    pub const PREV_ACTION: usize = 0x408;
    pub const ACTION: usize = 0x40C;
    pub const ACTION_TIME: usize = 0x410;
    pub const SUB: usize = 0x414;
    pub const FLAGS_418: usize = 0x418;
    pub const REQUEST: usize = 0x424;
    pub const CARD: usize = 0x428;
    pub const SPEED_4E0: usize = 0x4E0;
    pub const GRAVITY: usize = 0x5D8;
    pub const PHASE: usize = 0x604;
    pub const PREV_SITUATION: usize = 0x634;
    pub const COUNTER: usize = 0x63C;
    pub const HIT: usize = 0x644;
    pub const CHAIN: usize = 0x64C;
    pub const RECOVERY: usize = 0x65C;
    pub const CARD_7B4: usize = 0x7B4;
}

type PlFn = unsafe extern "win64" fn(usize);
type PlIntFn = unsafe extern "win64" fn(usize, i32);
type GroundFn = unsafe extern "win64" fn(usize) -> u32;
type HealFn = unsafe extern "win64" fn(usize, i32, i32, i32, i32);
type SyncFn = unsafe extern "win64" fn() -> u64;

fn note(line: impl FnOnce() -> String) {
    if LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("items: {}", line()));
    }
}

fn event(e: String) {
    if let Ok(mut g) = EVENTS.lock() {
        if g.len() < 16 {
            g.push(e);
        }
    }
}

/// Re:CoM's save block, when it is there.
fn save() -> Option<usize> {
    // SAFETY: a static pointer of the image.
    let s: usize = unsafe { rd(va(base(), crate::keyblades::SAVE_PTR)) };
    (s != 0).then_some(s)
}

/// Reads Sora's inventory from our extension, runs `f`, writes it back when it changed. None when
/// KH1's MP (and so the KH1 setting in Sora's story with a save) isn't live.
fn with_inventory<R>(f: impl FnOnce(&mut Inventory) -> R) -> Option<R> {
    // 0.163.0: Riku's too (his story's save holds his own block, as his MP).
    if !super::mp::live() {
        return None;
    }
    let at = save()? + it::EXT_FIRST;
    // SAFETY: inside the 0x2AD4-byte save block (EXT_LAST < 0x2818).
    let mut block: [u8; it::EXT_LEN] = unsafe { rd(at) };
    // M8c stage 3 / M8d: a save's removed items become items in play, once (a v1 / v2 block).
    if let Some(c) = it::convert_removed(&mut block) {
        // SAFETY: as above.
        unsafe { wr(at, block) };
        log(&format!(
            "items: save block v{} -> v3: removed items converted at Combat = KH1 (Mega-Potion -> Hi-Potion, Mega-Ether -> Ether, Megalixir -> Elixir, Tent -> Elixir, Camping Set -> Elixir): {} slot(s), {} from the stock, {} past 99 lost",
            c.from, c.slots, c.stock, c.lost
        ));
    }
    let before = it::read(&block);
    let mut inv = before;
    let r = f(&mut inv);
    if inv != before {
        it::write(&inv, &mut block);
        // SAFETY: as above.
        unsafe { wr(at, block) };
    }
    Some(r)
}

/// Sora's inventory (a read), when KH1's items are live (M8 stage 8a.2, the camp's Items page).
pub fn inventory() -> Option<Inventory> {
    with_inventory(|inv| *inv)
}

/// Runs `f` on Sora's inventory and writes it back when it changed (the camp's Items page).
pub fn edit<R>(f: impl FnOnce(&mut Inventory) -> R) -> Option<R> {
    with_inventory(f)
}

/// What KH1's pause-menu use rule (`0x1402CD3A0`) reads of Sora: HP below max (Re:CoM's player;
/// none: false, nothing to heal), MP below max (KH1's MP).
pub fn menu_ctx() -> crate::items_page::Ctx {
    // SAFETY: a static pointer of the image; the player's fields while it is set.
    let pl: usize = unsafe { rd(va(base(), PLAYER)) };
    let hp_low = pl != 0 && unsafe { rd::<i32>(pl + HP) < rd::<i32>(pl + HP_MAX) };
    let mp_low = super::mp::current().is_some_and(|(mp, max, _)| mp < max);
    crate::items_page::Ctx { hp_low, mp_low }
}

/// KH1's pause-menu use (`0x14030F160` → `0x1402CD590`): the item's effect on Sora at once, no
/// motion. HP through Re:CoM's heal `0x1401E59A0(player, amount, 0, 0, 0)` (no effect, no popup;
/// only below max, KH1's `0x1402CD5E0`), then Re:CoM's own player → save stats copy (the camp
/// copied them as it opened); MP through KH1's MP. The item already left the inventory. Returns a
/// line for the log.
pub fn use_in_menu(item: u8) -> String {
    let Some(e) = it::effect(item) else { return "no effect".into() };
    let mut what = Vec::new();
    // SAFETY: a static pointer of the image; Re:CoM's own heal and sync on its current player.
    unsafe {
        let pl: usize = rd(va(base(), PLAYER));
        if let Some(h) = e.hp {
            if pl == 0 {
                what.push("no player: HP not healed".to_string());
            } else {
                let hp: i32 = rd(pl + HP);
                let max: i32 = rd(pl + HP_MAX);
                match it::hp_to_add(h, hp, max) {
                    Some(a) => {
                        func::<HealFn>(HEAL)(pl, a, 0, 0, 0);
                        func::<SyncFn>(STATS_SYNC)();
                        what.push(format!("HP {hp} -> {} (+{a}, max {max})", rd::<i32>(pl + HP)));
                    }
                    None => what.push(format!("HP full ({hp}): no heal")),
                }
            }
        }
    }
    if let Some(n) = e.mp {
        let rose = super::mp::add_points(n, it::name(item));
        what.push(if rose { "MP up".to_string() } else { "MP full or not live".to_string() });
    }
    what.join(", ")
}

/// Sora's slots and slot count for the command menu (a read; all empty when not live).
pub fn slots() -> ([u8; it::MAX_SLOTS], u8) {
    with_inventory(|inv| (inv.slots, inv.slot_count)).unwrap_or(([0; it::MAX_SLOTS], 0))
}

/// KH1's pickup (`0x140284E60`): the first empty slot for a battle item, else the stock (below 99).
/// For the shop (8a.3) and drops (8a.4); the use task returns an unused item through it.
pub fn pick_up(item: u8, why: &str) -> Option<Picked> {
    let r = with_inventory(|inv| inv.pick_up(item)).flatten();
    note(|| format!("{} {why}: {}", it::name(item), match r {
        Some(Picked::Slot(k)) => format!("slot {}", k + 1),
        Some(Picked::Stock(n)) => format!("stock ({n})"),
        None => "refused (stock full or not live)".into(),
    }));
    r
}

/// Patches the Field exe's case for action `0xE` and checks the effect names. The exeItem hook is
/// installed by the caller after.
pub unsafe fn install(b: usize, s: &crate::settings::Settings) -> bool {
    use crate::items::field_case as fc;
    let cave = b + fc::CAVE_RVA as usize;
    let entry = b + fc::TABLE_ENTRY_RVA as usize;
    let old_entry = fc::DEFAULT_RVA.to_le_bytes();
    let have = |at: usize, n: usize| std::slice::from_raw_parts(at as *const u8, n).to_vec();
    if have(cave, fc::STUB_LEN) != [0xCC; fc::STUB_LEN] || have(entry, 4) != old_entry {
        log("items: the Field exe's case 0xE or its cave is not as expected; no KH1 items");
        return false;
    }
    if !super::hook::patch(cave, &[0xCC; fc::STUB_LEN], &fc::stub(), "items: Field case stub") {
        return false;
    }
    if !super::hook::patch(entry, &old_entry, &fc::entry(), "items: Field jump table 0xE") {
        return false;
    }
    let name_is = |ptr: u64, want: &[u8]| {
        let s: usize = rd(va(b, ptr));
        s != 0 && have(s, want.len()) == want
    };
    let ok = name_is(ALWAYS_NAME_PTR, b"always\0") && name_is(ITEMPLUS_NAME_PTR, b"itemplus\0");
    EFFECT_NAMES_OK.store(ok, Ordering::Release);
    if !ok {
        log("items: Re:CoM's effect names at 0x1407C5DD8 / 0x1407C5DE0 are not always / itemplus: uses heal without Re:CoM's item effects");
    }
    TEST_ITEMS.store(s.kh1_test_items, Ordering::Relaxed);
    true
}

pub fn mark_installed() {
    INSTALLED.store(true, Ordering::Release);
}

/// The items are in (the command menu shows Sora's slots only then).
pub fn installed() -> bool {
    INSTALLED.load(Ordering::Acquire)
}

/// An item from the command menu's Items list: taken by Sora's KH1 frame this frame, if it runs
/// (else dropped at the next frame's start).
pub fn issue(item: u8, slot: u8) {
    if !INSTALLED.load(Ordering::Acquire) {
        note(|| format!("{}: no KH1 items installed", it::name(item)));
        return;
    }
    if let Ok(mut g) = PENDING.lock() {
        *g = Some(ItemCommand { item, slot });
    }
}

/// The start of Sora's frame (the request-bits wrapper, before the menu): a command no KH1 frame
/// took is gone.
pub fn new_frame() {
    if let Ok(mut g) = PENDING.try_lock() {
        if let Some(c) = g.take() {
            note(|| format!("{}: dropped (no KH1 frame ran: Re:CoM's action)", it::name(c.item)));
        }
    }
}

/// The command for this KH1 frame (`win::movement::run_frame`).
pub fn take_command() -> Option<ItemCommand> {
    PENDING.try_lock().ok().and_then(|mut g| g.take())
}

/// KH1's `0x1402C2760` (the item leaves its slot, the use task starts), then `initItem`'s writes
/// for the item action, cardless (Checked (disasm `0x1401F28F0`), in its order): the action `0xE`,
/// sub 2 (`exeItem`'s "the clip plays": the KH1 frame plays it), `+0x63C` = 0 (its argument),
/// `+0x65C` = 0 (`0x1407C7830[BattleCardList[card] + 8]`: no card; index 0 is 0). False when the
/// slot doesn't hold the item: nothing changes.
pub unsafe fn start_item(pl: usize, c: &ItemCommand) -> bool {
    if with_inventory(|inv| inv.take(c.slot, c.item).is_some()) != Some(true) {
        note(|| format!("{} slot {}: the slot doesn't hold it (or items aren't live): not used", it::name(c.item), c.slot + 1));
        return false;
    }
    let cur: u32 = rd(pl + f::ACTION);
    wr(pl + f::PREV_ACTION, cur);
    wr(pl + f::ACTION, it::RECOM_ITEM_ACTION);
    wr(pl + f::ACTION_TIME, 0u32);
    wr(pl + f::SUB, 0u32);
    // `0x1401A38D0`: the body's velocity to zero (the KH1 frame writes its own after).
    let body: usize = rd(pl + p::BODY);
    if body != 0 {
        wr(body + 0x80, [0f32, 0.0, 0.0, 1.0]);
    }
    wr(pl + f::CARD, rd::<u32>(pl + f::CARD) & 0xFFF7_FF77);
    wr(pl + f::REQUEST, rd::<u32>(pl + f::REQUEST) & 0xFFFF_FFF7);
    wr(pl + f::FLAGS_418, rd::<u32>(pl + f::FLAGS_418) & 0xFD7F_FFFF);
    wr(pl + f::COUNTER, 0u32);
    wr(pl + f::PREV_SITUATION, 0u32);
    wr(pl + f::HIT, 0u64);
    wr(pl + f::CHAIN, 0u64);
    wr(pl + f::CARD_7B4, 0u32);
    wr(pl + f::SPEED_4E0, 0u32);
    wr(pl + f::GRAVITY, rd::<f32>(va(base(), BASE_GRAVITY)));
    wr(pl + f::PHASE, it::ITEM_PHASE);
    wr(pl + f::SUB, it::ITEM_SUB);
    wr(pl + f::RECOVERY, 0u32);
    ACTIVE.store(pl, Ordering::Release);
    if let Ok(mut g) = TASKS.lock() {
        if g.len() >= 4 {
            g.remove(0);
        }
        g.push(Run { player: pl, task: UseTask::new(c.item, c.slot) });
        TASKS_ON.store(true, Ordering::Release);
    }
    true
}

/// Re:CoM ends its item action (`0x1401F6660(player, airborne)`, as `exeItem` does at its end).
/// True when it entered its win action instead.
pub unsafe fn end_item(pl: usize, airborne: bool) -> bool {
    ACTIVE.store(0, Ordering::Release);
    func::<PlIntFn>(END_ACTION)(pl, airborne as i32);
    mv::in_recom_win(pl)
}

/// `exeItem` (`0x1401ED6E0`, action `0xE`; the Battle exe at `0x1401F9A13`, the Field exe through
/// our case): KH1's frame for a KH1 item use, Re:CoM's exe otherwise (its item cards, untouched).
pub unsafe extern "win64" fn exe_item(pl: usize) {
    let o: PlFn = orig(&ORIG_EXE_ITEM);
    if !mv::kh1_fighter(pl) || ACTIVE.load(Ordering::Acquire) != pl {
        return o(pl);
    }
    if !mv::continues(pl) {
        // Re:CoM ran something else in between: end its action and take over from Re:CoM's state
        // (the use task goes on: KH1's doesn't watch the motion). Unless that end entered Re:CoM's
        // win action: Re:CoM's battle end owns Sora.
        if end_item(pl, func::<GroundFn>(GROUND_TEST)(pl) != 0) {
            return;
        }
        return mv::run_frame(pl, true);
    }
    mv::run_frame(pl, false);
}

/// KH1's use tasks' runs for the last frame (`0x1402C2880`; the request-bits wrapper, each frame of
/// Sora's at the KH1 setting): one atomic load unless a use is going.
pub unsafe fn task_frame(pl: usize) {
    if !TASKS_ON.load(Ordering::Acquire) {
        return;
    }
    let step = (rd::<i32>(pl + p::DT).max(0) as f32) / 50.0;
    let hp: i32 = rd(pl + HP);
    // KH1's `0x140291FF0` on Sora: alive; Re:CoM: HP left and not in its death path.
    let alive = hp > 0 && rd::<i32>(pl + DEAD) == 0;
    let row: i32 = rd(pl + p::ROW);
    let mut out = Vec::new();
    if let Ok(mut g) = TASKS.try_lock() {
        for r in g.iter_mut() {
            // Another player object (a room or battle change before the heal): the item goes back,
            // as KH1 returns it when an event takes Sora (Assumption: KH1's map change ends its
            // tasks this way).
            let s = if r.player != pl { TaskStep::Return } else { r.task.run(step, alive, false, row) };
            if r.player != pl {
                r.task.done = true;
            }
            if s != TaskStep::Wait {
                out.push((r.task, s));
            }
        }
        g.retain(|r| !r.task.done);
        TASKS_ON.store(!g.is_empty(), Ordering::Release);
    }
    for (t, s) in out {
        match s {
            TaskStep::Hand { same_motion } => {
                // KH1's hand effect (`always.dpx` 1 at bone 0x1C): Re:CoM has no item effect at
                // this moment (its own plays at the use); not drawn (docs, not 1:1).
                event(format!("hand {}{}", it::name(t.item), if same_motion { "" } else { " (motion cut)" }));
                note(|| format!("use {}: KH1's hand moment at f{:.0}{}", it::name(t.item), t.age, if same_motion { "" } else { " (the motion was cut: KH1 shows no hand effect)" }));
            }
            TaskStep::Heal => heal(pl, t.item, t.age),
            TaskStep::Return => {
                pick_up(t.item, "returned (Sora not alive before the heal)");
                event(format!("return {}", it::name(t.item)));
            }
            TaskStep::Wait => {}
        }
    }
}

/// `0x1402C2B50` on Sora: Re:CoM's own effect for the item (`exeItem`'s, on his bone 0 at (0,
/// 0.85, 0), kept on him) and the item's effect: HP through Re:CoM's heal (only below max, as KH1's
/// `0x1402CD5E0`), MP through KH1's MP (`0x1402CD6B0`). The Mega items heal the party: Sora alone
/// for now.
unsafe fn heal(pl: usize, item: u8, age: f32) {
    let Some(e) = it::effect(item) else { return };
    if EFFECT_NAMES_OK.load(Ordering::Acquire) {
        if let Some((set, id)) = it::recom_effect(item) {
            let ptr = match set {
                EffectSet::Always => ALWAYS_NAME_PTR,
                EffectSet::ItemPlus => ITEMPLUS_NAME_PTR,
            };
            let name: usize = rd(va(base(), ptr));
            super::magic::effect_on_bone(pl, pl, 0, name, id, &super::magic::BODY_OFFSET, true);
        }
    }
    let hp: i32 = rd(pl + HP);
    let max: i32 = rd(pl + HP_MAX);
    let mut what = Vec::new();
    if let Some(h) = e.hp {
        match it::hp_to_add(h, hp, max) {
            Some(a) => {
                func::<HealFn>(HEAL)(pl, a, 0, 1, 0);
                what.push(format!("HP {hp} -> {} (+{a}, max {max})", rd::<i32>(pl + HP)));
            }
            None => what.push(format!("HP full ({hp}): no heal")),
        }
    }
    if let Some(n) = e.mp {
        let rose = super::mp::add_points(n, it::name(item));
        what.push(if rose { "MP up".to_string() } else { "MP full or not live".to_string() });
    }
    event(format!("heal {} {}", it::name(item), rd::<i32>(pl + HP)));
    note(|| format!("use {}: heal at f{age:.0}: {}", it::name(item), what.join(", ")));
}

/// Debug (`kh1_test_items`): a new player HUD (a room or battle): the test inventory when every
/// slot is empty (`crate::items::test_inventory`, written to the save).
pub fn hud_built() {
    if !TEST_ITEMS.load(Ordering::Relaxed) {
        return;
    }
    if with_inventory(it::test_inventory) == Some(true) {
        let (slots, n) = slots();
        log(&format!("items: kh1_test_items: slots {:?} (of {n}), stock 9 of each item in play (Potion, Hi-Potion, Ether, Elixir; written to the save)", &slots[..n as usize]));
    }
}

/// The frame's item events (from `win::movement::run_frame`): log and trace.
pub fn on_events(ev: &[ItemEvent]) {
    for e in ev {
        let t = e.token();
        event(t.clone());
        note(|| t);
    }
}

/// The F9 trace's item token: the first use task running and this frame's events.
pub fn trace_token() -> String {
    let ev = EVENTS.lock().map(|mut g| std::mem::take(&mut *g)).unwrap_or_default();
    let mut t = match TASKS.try_lock() {
        Ok(g) => it::trace_token(g.first().map(|r| &r.task)),
        Err(_) => "item ?".into(),
    };
    for e in ev {
        t.push(' ');
        t.push_str(&e);
    }
    t
}

/// Clears the trace's events (a new recording starts).
pub fn clear_events() {
    if let Ok(mut g) = EVENTS.lock() {
        g.clear();
    }
}

/// The start log line.
pub fn start_line() -> String {
    format!(
        "items: KH1's items at combat = kh1 in Sora's story (M8 stage 8a.1): slots and stock at save+{:#x}..={:#x} (layout v1 saves read as KH1's new game: {} empty slots); the command menu's Items list; a use plays KH1's motion 0x3E (Re:CoM's item clip row 0x2D) on Re:CoM's item action 0xE, KH1's use task: the hand moment {} frames after the command, the heal {} frames after that, with Re:CoM's heal (HP: KH1's amount x 560/120, provisional) and Re:CoM's item effects{}; Field exe case 0xE -> exeItem; kh1_test_items = {}",
        it::EXT_FIRST,
        it::EXT_LAST,
        it::START_SLOTS,
        it::SELF_DELAY,
        it::HEAL_DELAY + 1.0,
        if EFFECT_NAMES_OK.load(Ordering::Acquire) { "" } else { " (effects OFF: names not found)" },
        crate::settings::switch_str(TEST_ITEMS.load(Ordering::Relaxed))
    )
}
