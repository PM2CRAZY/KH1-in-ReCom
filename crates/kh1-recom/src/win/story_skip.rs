//! Story skips at Combat = KH1 (`crate::story_skip`, `docs/recom-story-skips.md`).
//!
//! - **The wrapper** on Re:CoM's battle-entry request `0x140236690(room)`: when an event room's
//!   script asks for one of the two tutorial battle rooms (`crate::story_skip::SKIPS`) at Combat =
//!   KH1 in Sora's story, in Traverse Town, it does that battle script's lesson exit instead: its
//!   writes through Re:CoM's own setters (`0x1401B1650`, `0x1401B16C0`, `0x1401B08C0`), then its
//!   room change through Re:CoM's own `0x1402360F0`. Every other request goes to Re:CoM unchanged.
//! - If the room change can't find its floor or refuses, Re:CoM's battle entry runs as asked, so
//!   the story never stalls.
//! - Runs once per battle request (a script syscall or a field encounter), nothing per frame. Each
//!   skip is logged.
//! - Fork (0.196.0): Riku's Ansem tutorial (`WORLD00` 2102 → battle 1100) can't be skipped without loading it (its exit must
//!   return to 2102 through Re:CoM's battle exit); the request arms it and [`riku_frame`] does the lesson for it in the room.

use super::movement::{self as mv, func, orig, rd};
use super::{base, log, va};
use crate::keyblades::SAVE_PTR;
use crate::story_skip::{self as ss, Door, Write};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

pub static ORIG_BATTLE_ENTRY: AtomicUsize = AtomicUsize::new(0);
static LOG_LEFT: AtomicU32 = AtomicU32::new(40);

/// Re:CoM's functions (RVAs; first bytes checked: `sites::STORY_SKIP_CALLEES` and the system
/// variable getter / setter of earlier stages).
const ROOM_CHANGE: u32 = 0x0023_60F0;
const SAVED_DOOR: u32 = 0x0023_3AE0;
const FLAG_SET: u32 = 0x001B_1650;
const VAR_GET: u32 = 0x001B_08C0;
const VAR_SET: u32 = 0x001B_16C0;
const BYTE_SET: u32 = 0x001B_15D0;
const CARD_GIVE: u32 = 0x0023_AA60;

/// The save block's map world code and room (`0x1402336C0`, `0x1402339B0`). Checked (decomp).
const SAVE_WORLD: usize = 0x1878;
const SAVE_ROOM: usize = 0x187A;

type EntryFn = unsafe extern "win64" fn(u64) -> u64;
type RoomChangeFn = unsafe extern "win64" fn(u32, u32, u32) -> u32;
type DoorFn = unsafe extern "win64" fn() -> u32;
type FlagSetFn = unsafe extern "win64" fn(u32, u32, u32);
type VarGetFn = unsafe extern "win64" fn(u32, u32) -> i32;
type VarSetFn = unsafe extern "win64" fn(u32, u32, i32);
type ByteSetFn = unsafe extern "win64" fn(u32, u32, u32);
type CardGiveFn = unsafe extern "win64" fn(u32) -> u64;

/// Riku's first Ansem fight (fork, 0.196.0 skip; 0.198.38 fought): 0 idle, 1 armed (2102 asked for 1100: the fight), 2 Ansem
/// beaten (the lesson's won-path writes done, the script's exit asked).
static RIKU_STATE: AtomicU32 = AtomicU32::new(0);
/// Ansem in room 1100 (set by `super::stats::stat_init`), 0 before he is made or after the win.
static RIKU_ANSEM: AtomicUsize = AtomicUsize::new(0);
/// Frames Ansem's HP has sat at exactly 1.
static RIKU_HELD: AtomicU32 = AtomicU32::new(0);

/// The fight is armed (Ansem's HP is scaled at his stats init).
pub fn riku_fight_armed() -> bool {
    RIKU_STATE.load(Ordering::Relaxed) == 1
}

/// Ansem was made in the fight (`super::stats::stat_init`).
pub fn riku_ansem_made(enemy: usize, recom_hp: i32, hp: i32) {
    RIKU_ANSEM.store(enemy, Ordering::Relaxed);
    RIKU_HELD.store(0, Ordering::Relaxed);
    note(|| format!("Riku's Ansem fight: Ansem (EN0122) at {hp} HP (Re:CoM's {recom_hp} x {} / {})", ss::RIKU_ANSEM_HP, ss::RIKU_ANSEM_RECOM_HP));
}

/// The lesson's won-path writes and cards, once, when Ansem is beaten.
unsafe fn riku_won(how: &str) {
    for w in ss::RIKU_TUTORIAL_SAVED {
        apply(*w);
    }
    let (id, v) = ss::RIKU_TUTORIAL_BYTE;
    func::<ByteSetFn>(BYTE_SET)(1, id, v as u32);
    let mut given = vec![];
    for c in ss::RIKU_TUTORIAL_CARDS {
        given.push(format!("{c}:{}", func::<CardGiveFn>(CARD_GIVE)(c as u32) & 0xFF));
    }
    let bits = func::<VarGetFn>(VAR_GET)(0, 0x24);
    RIKU_STATE.store(2, Ordering::Relaxed);
    RIKU_ANSEM.store(0, Ordering::Relaxed);
    RIKU_FRAMES.store(0, Ordering::Relaxed);
    note(|| format!("Riku's Ansem fight won ({how}): the lesson's saved writes ((0, 0x24) = {bits:#x}, byte (1, 4) = 1), world cards given (card:result) {}; its \"done\" bit (1, 2) set so its script leaves through its own exit", given.join(" ")));
}
/// Frames seen in the battle room while armed.
static RIKU_FRAMES: AtomicU32 = AtomicU32::new(0);

fn note(line: impl FnOnce() -> String) {
    if LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("story skip: {}", line()));
    }
}

pub fn start_line() -> String {
    let rooms: Vec<String> = ss::SKIPS.iter().map(|s| format!("{} ({} from {} -> {})", s.name, s.battle_room, s.from_room, s.exit.room)).collect();
    format!(
        "story skip: wrapper on Re:CoM's battle-entry request 0x140236690; at Combat = KH1 in Sora's story, Traverse Town's tutorial battle rooms are skipped through their own lesson exit: {}",
        rooms.join(", ")
    )
}

/// What the request is asked in, from the save block (None before there is one).
unsafe fn ask(battle_room: i16) -> Option<ss::Ask> {
    let save: usize = rd(va(base(), SAVE_PTR));
    if save == 0 {
        return None;
    }
    Some(ss::Ask {
        kh1: mv::kh1_on(),
        riku_story: super::stats::riku_story(),
        world: rd(save + SAVE_WORLD),
        room: rd(save + SAVE_ROOM),
        battle_room,
    })
}

unsafe fn apply(w: Write) {
    match w {
        Write::Bit1(id, v) => func::<FlagSetFn>(FLAG_SET)(1, id, v as u32),
        Write::Int1(id, v) => func::<VarSetFn>(VAR_SET)(1, id, v),
        Write::Int0Or(id, mask) => {
            let v = func::<VarGetFn>(VAR_GET)(0, id) | mask as i32;
            func::<VarSetFn>(VAR_SET)(0, id, v);
        }
    }
}

/// Development builds only: the saved story bit (0, 0x24) 0x08 that `WORLD00/2102` sets after Riku's first battle and his
/// player build (`0x1401F0C50`) turns into Dark Mode (`+0x41C & 0x4000000`, Re:CoM then makes Dark Riku); set whenever
/// missing (any Riku save loaded), logged. Takes effect from the next battle's player. One story-int read a frame (dev only).
#[cfg(feature = "dev")]
unsafe fn dev_dark_mode() {
    if rd::<usize>(va(base(), SAVE_PTR)) == 0 || !super::stats::riku_story() {
        return;
    }
    let before = func::<VarGetFn>(VAR_GET)(0, 0x24);
    // The D Mode card too, as `WORLD00/2102` and Re:CoM's own debug helpers (`0x1400D1A00` / `0x1400D1FC0`) give both (the
    // workspace's `recom-riku-sleights.md` § 4): the Customize page's Dark Mode set reads the card.
    let has_card = super::abilities::sleight_learned_fn()(0x6D) & 0xFF != 0;
    if before & 0x08 == 0 || !has_card {
        apply(ss::Write::Int0Or(0x24, 0x08));
        let card = if has_card { 1 } else { func::<CardGiveFn>(CARD_GIVE)(0x6D) & 0xFF };
        let after = func::<VarGetFn>(VAR_GET)(0, 0x24);
        log(&format!("DEV riku_dark_mode: story bit (0, 0x24) 0x08 ({before:#x} -> {after:#x}), D Mode card 0x6D ({}): Riku has Dark Mode from the next battle", if has_card { "already learned".to_string() } else { format!("given: {card}") }));
    }
}

/// Once per frame of Riku's at Combat = KH1 (the request-bits wrapper): while the tutorial skip is armed, in battle room 1100
/// the lesson's won-path writes and its "done" bit (`crate::story_skip::RIKU_TUTORIAL_*`); back in 2102, the duel block cleared
/// once more and the skip disarmed. One atomic load otherwise.
pub unsafe fn riku_frame() {
    // 0.198.18, development builds only (`settings::DEV_KEYS`): `riku_dark_mode = on` sets the story bit that gives Riku
    // Dark Mode; compiled out of a released DLL.
    #[cfg(feature = "dev")]
    if super::settings().riku_dark_mode {
        dev_dark_mode();
    }
    let st = RIKU_STATE.load(Ordering::Relaxed);
    if st == 0 {
        return;
    }
    let save: usize = rd(va(base(), SAVE_PTR));
    if save == 0 {
        return;
    }
    let (world, room): (i16, i16) = (rd(save + SAVE_WORLD), rd(save + SAVE_ROOM));
    if world == ss::WORLD_RIKU_TUTORIAL && room == ss::RIKU_TUTORIAL_BATTLE {
        if st == 1 {
            // 0.198.38: the fight. The card lesson stays off and the script not done until Ansem is beaten.
            for w in ss::RIKU_FIGHT_SESSION {
                apply(*w);
            }
            let ansem = RIKU_ANSEM.load(Ordering::Relaxed);
            if ansem != 0 {
                let hp: i32 = rd(ansem + 0x274);
                let held = if hp == 1 {
                    RIKU_HELD.fetch_add(1, Ordering::Relaxed) + 1
                } else {
                    RIKU_HELD.store(0, Ordering::Relaxed);
                    0
                };
                if hp <= 0 {
                    riku_won("Ansem's HP 0");
                } else if held >= ss::RIKU_ANSEM_HELD_AT_1 {
                    riku_won("Ansem held at 1 HP");
                }
            }
            return;
        }
        let n = RIKU_FRAMES.fetch_add(1, Ordering::Relaxed);
        if n < ss::RIKU_TUTORIAL_GIVE_UP {
            for w in ss::RIKU_TUTORIAL_SESSION {
                apply(*w);
            }
        } else if n == ss::RIKU_TUTORIAL_GIVE_UP {
            note(|| format!("Riku's Ansem tutorial: still in battle room 1100 after {n} frames; the writes stop (the lesson would need Re:CoM's combat)"));
        }
        return;
    }
    if st == 1 {
        // left the battle room without beating Ansem (a game over, a reload): disarmed; 2102 asks for the fight again.
        RIKU_STATE.store(0, Ordering::Relaxed);
        RIKU_ANSEM.store(0, Ordering::Relaxed);
        note(|| format!("Riku's Ansem fight left without a win: now world {world} room {room}"));
        return;
    }
    if st == 2 {
        // left the battle room: the duel block stays cleared (only scripts write it), the fight is done
        apply(ss::Write::Bit1(0x2A, false));
        let n = RIKU_FRAMES.load(Ordering::Relaxed);
        RIKU_STATE.store(0, Ordering::Relaxed);
        note(|| format!("Riku's Ansem tutorial left after {n} frame(s) in battle room 1100: now world {world} room {room}"));
    }
}

/// Re:CoM's `0x140236690(room)` (the room in `cx`; `bool` in `al`).
pub unsafe extern "win64" fn battle_entry(arg: u64) -> u64 {
    let o: EntryFn = orig(&ORIG_BATTLE_ENTRY);
    let battle_room = arg as u16 as i16;
    // Fork (0.196.0): Riku's Ansem tutorial is armed here and done on his frames in the battle room (`riku_frame`).
    if battle_room == ss::RIKU_TUTORIAL_BATTLE {
        if let Some(a) = ask(battle_room) {
            if ss::riku_tutorial(a, mv::riku_on()) {
                RIKU_STATE.store(1, Ordering::Relaxed);
                RIKU_FRAMES.store(0, Ordering::Relaxed);
                RIKU_ANSEM.store(0, Ordering::Relaxed);
                note(|| format!("Riku's Ansem fight: battle room 1100 asked from room {} (world {}) at Combat = KH1 with Riku's KH1 combat: armed (fought without the card lesson; its ending runs when Ansem is beaten)", a.room, a.world));
                return o(arg);
            }
        }
    }
    // Only the two tutorial rooms are looked at further (other callers pass 0 or 1030).
    if !ss::SKIPS.iter().any(|s| s.battle_room == battle_room) {
        return o(arg);
    }
    let Some(a) = ask(battle_room) else { return o(arg) };
    let Some(skip) = ss::plan(a) else {
        if ss::is_tutorial_ask(a) {
            note(|| format!("battle room {battle_room} asked from room {} (world {}): Re:CoM's battle (Combat = {}, {})", a.room, a.world, if a.kh1 { "KH1" } else { "Re:CoM" }, if a.riku_story { "Riku's story" } else { "Sora's story" }));
        }
        return o(arg);
    };
    let e = skip.exit;
    if e.kind < 5 {
        let slot: i16 = rd(va(base(), ss::FLOOR_SLOTS) + 2 * e.kind as usize);
        if !ss::floor_slot_ok(e.kind, slot) {
            note(|| format!("{}: floor slot {} is {slot}; Re:CoM's battle room {battle_room} runs instead", skip.name, e.kind));
            return o(arg);
        }
    }
    for w in skip.writes {
        apply(*w);
    }
    let door = match e.door {
        Door::Fixed(d) => d,
        Door::Saved => func::<DoorFn>(SAVED_DOOR)() & 0xFF,
    };
    let ok = func::<RoomChangeFn>(ROOM_CHANGE)(e.kind, door, e.room as u16 as u32);
    let friends = func::<VarGetFn>(VAR_GET)(0, ss::FRIENDS_MET);
    if ok != 0 {
        note(|| {
            format!(
                "{} skipped at Combat = KH1: battle room {battle_room} (asked from room {}) not entered; its lesson exit's {} writes done (friends-met (0, {:#x}) = {friends:#x}), Re:CoM's room change 0x1402360F0({}, {door}, {}) -> {ok}",
                skip.name, a.room, skip.writes.len(), ss::FRIENDS_MET, e.kind, e.room
            )
        });
        return 1;
    }
    note(|| format!("{}: Re:CoM's room change 0x1402360F0({}, {door}, {}) refused; Re:CoM's battle room {battle_room} runs instead", skip.name, e.kind, e.room));
    o(arg)
}
