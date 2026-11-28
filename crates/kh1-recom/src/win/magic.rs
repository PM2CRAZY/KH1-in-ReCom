//! KH1's cardless cast and Fire inside Re:CoM's own magic action (M7 stage 7.4; `crate::magic`,
//! `docs/recom-m7-magic.md`).
//!
//! - **Command:** the command menu's spell (and, in 7.12, a shortcut) calls [`issue`]; the next
//!   KH1 frame of Sora's (the same frame: the request-bits wrapper runs before the state exe)
//!   takes it as KH1's magic command.
//! - **Start** ([`start_cast`]): `initMagic`'s (`0x1402203D0`) own writes for a cast, cardless:
//!   Re:CoM's action `0x13`, its fields, `+0x3C8` → the KH1 row (Re:CoM's Fire / Fira / Firaga row
//!   with KH1's ball, `crate::magic::fire_row_bytes`). `initMagic` itself reads the played card
//!   (`+0x7F4` into `BattleCardList`, loaded only with a card battle, `0x1401607A0`), so it
//!   isn't called; the rest of what it does is done here with Re:CoM's own functions
//!   (`0x140209800`, `0x1401F68A0`), as M5's row pick does for swings.
//! - **Each frame** of the cast: Re:CoM's Battle exe calls `exeMagic` through Sora's vtable, the
//!   Field exe through a new case (`crate::magic::field_case`); our hook on `exeMagic` runs KH1's
//!   frame instead (as the swing exe's does): the cast motions, the windows, the script.
//! - **Release** ([`release`]): Re:CoM's spawn's Fire case (`0x140222170` case `0x22`) as a new
//!   KH1 case: the same Re:CoM calls (hand point `0x1401F0B30`, descriptor `0x140156790`, angles
//!   `0x1401A4A00`, factory `0x1401A8A80`, shot manager `0x140158DD0`, mover set-up
//!   `0x140157DA0`) with the KH1 row's numbers and KH1's life; then KH1's MP is paid
//!   (`super::mp::pay`, 30 units). Re:CoM's `CHomingBullet` then steps, homes, draws, collides
//!   and ends the ball.
//! - **Re-target** (7.4b, decision 36): a ball cast with no target gets KH1's pick each frame
//!   ([`seek_on`], [`kh1_ball_exe`]): its own flight state's exe (Re:CoM's `CStateTemplate<CBullet>`
//!   at `bullet+0x2B0`, exe `+0x2D0` = the vcall thunk `0x1401560B8` → `CHomingBullet`'s homing
//!   step `0x1401575D0`) points at our wrapper, which writes KH1's pick (`crate::magic::retarget`)
//!   into Re:CoM's homing target `+0x2F4` and runs Re:CoM's step. Re:CoM's own shots never pass
//!   through it.
//! - **Hits:** the enemy-hit wrapper ([`take_hit`]) recognises a hit part whose attacker is one of
//!   our balls and gives it KH1's spell damage (`super::stats::kh1_spell_hit`).
//! - **End** ([`end_cast`]): Re:CoM's `0x1401F6660`, as `exeMagic` ends its action.

use super::movement::{self as mv, func, orig, p, rd, rd_vec, wr};
use super::{base, log, va};
use crate::magic::{self as mg, CastEvent, CastStart, MagicCommand};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

pub static ORIG_EXE_MAGIC: AtomicUsize = AtomicUsize::new(0);

static INSTALLED: AtomicBool = AtomicBool::new(false);
/// The KH1 magic rows (`crate::magic::kh1_magic_table`). Never freed: `+0x3C8` points into it.
static TABLE: OnceLock<&'static [[u8; 16]]> = OnceLock::new();
/// 0.184.2: Riku's copy of [`TABLE`], his spells' looks (`crate::magic::riku_look`): the same KH1 rows but the effect
/// set (`+2`) of his.
static RIKU_TABLE: OnceLock<&'static [[u8; 16]]> = OnceLock::new();
/// The command issued this frame (the menu's), for the KH1 frame.
static PENDING: Mutex<Option<MagicCommand>> = Mutex::new(None);
/// Our shots: (Re:CoM shot id, spell, level, the cast's shared hit serial or 0), the last 32 (a
/// Blizzard makes 7).
static SHOTS: Mutex<Vec<(u32, u8, u8, u32)>> = Mutex::new(Vec::new());
/// Our Blizzard shards (7.5): (shot id, its vertical step dy, m per frame), the last 32.
static SHARDS: Mutex<Vec<(u32, f32)>> = Mutex::new(Vec::new());
/// Thunder (7.6): what the command saw (the target's handle if it was an alive enemy, and the area
/// centre then), for the release.
static THUNDER_CMD: Mutex<Option<(u32, [f32; 3])>> = Mutex::new(None);
/// Thunder's strike threads while they run (KH1 runs each cast's thread to its end, so a second
/// Thunder's runs beside the first's; at most 8), and a flag the per-frame check reads.
static STRIKES: Mutex<Vec<ThunderRun>> = Mutex::new(Vec::new());
/// Any spell thread is running (Thunder's strikes, Gravity's falls): the per-frame check reads it.
static STRIKING: AtomicBool = AtomicBool::new(false);
/// Gravity (7.8): what the command saw (the target's handle if it was an alive enemy, and the
/// offset along Sora's facing used without one), for the release.
static GRAVITY_CMD: Mutex<Option<(u32, [f32; 3])>> = Mutex::new(None);
/// Gravity's falling threads while they run (at most 8).
static FALLS: Mutex<Vec<GravityRun>> = Mutex::new(Vec::new());

/// A Gravity cast's sphere in its fall.
struct GravityRun {
    fall: mg::Fall,
    level: u8,
    /// Re:CoM's sphere (its handle / shot id), and the impact's centre (the marker point + 0.1 m).
    shot: u32,
    centre: [f32; 3],
    kind: u32,
    attr: u32,
    ap: i32,
}

/// A Thunder (7.6) or Stop (7.9) cast's strikes in progress.
struct ThunderRun {
    s: mg::Strikes,
    /// Thunder or Stop, and the area its bodies test (KH1's sys 90).
    spell: u8,
    area: mg::Area,
    level: u8,
    centre: [f32; 3],
    /// Re:CoM's kind / attribute / AP of the cast (the magic action's, at the release).
    kind: u32,
    attr: u32,
    ap: i32,
}
/// KH1's re-target (7.4b) is set up: the homing slot and Re:CoM's lock-on list are as expected.
static RETARGET: AtomicBool = AtomicBool::new(false);
/// Our balls cast with no target: (shot id, the last pick's handle, 0 for none), the last 16.
static SEEKERS: Mutex<Vec<(u32, u32)>> = Mutex::new(Vec::new());
static RETARGET_LINES: AtomicU32 = AtomicU32::new(40);
static BOLT_LINES: AtomicU32 = AtomicU32::new(60);
static LOG_LEFT: AtomicU32 = AtomicU32::new(80);
static HIT_LINES: AtomicU32 = AtomicU32::new(40);
static EVENTS: Mutex<Vec<String>> = Mutex::new(Vec::new());

// Re:CoM functions and data (RVAs; first bytes checked in `sites::MAGIC_*`).
pub(super) const DESC_INIT: u32 = 0x0015_6790;
const HAND_POINT: u32 = 0x001F_0B30;
const ANGLES: u32 = 0x001A_4A00;
pub(super) const FACTORY: u32 = 0x001A_8A80;
pub(super) const SHOT_MANAGER: u32 = 0x0015_8DD0;
pub(super) const SHOT_MOVER: u32 = 0x0015_7DA0;
const HOMING_TARGET: u32 = 0x001F_68A0;
const CURSOR_BUSY: u32 = 0x0020_9800;
const END_ACTION: u32 = 0x001F_6660;
const GROUND_TEST: u32 = 0x001F_3C00;
/// Re:CoM's base gravity (`initMagic` writes it to `+0x5D8`). Checked (decomp).
const BASE_GRAVITY: u64 = 0x1_4087_B398;
/// The effect sets (`0x1405A1730`, 0x60 bytes each; Fire 3, Fira 4, Firaga 5 hold `fire_all`).
pub(super) const EFFECT_SETS: u64 = 0x1_405A_1730;
/// Re:CoM's attack rows.
const ROWS: u64 = crate::attack::RECOM_ATTACK_ROWS;
/// The vcall thunk `mov rax,[rcx]; jmp [rax+0xA8]` that the bullet init (`0x140157E60`, at
/// `0x1401585D5` / `0x140158609`) stores as every bullet's flight-state exe; for a
/// `CHomingBullet` it reaches the homing step `0x1401575D0`. Checked (disasm).
pub(super) const HOMING_THUNK: u32 = 0x0015_60B8;
/// `0x140156770(bullet)`: its attack's hit serial (`coll+0x70`), which Re:CoM's Blizzard case passes
/// to its other shards (`desc+0x74`). The apply `0x14016F1C0` skips a body whose ring holds the
/// serial (`0x14016F960`) and records it on the body it hits (`0x14016FE20`); the enemy's hit
/// handler (`0x140154EB0`) then records it on all the enemy's bodies and takes one hit body per
/// run. So with one serial, a cast hits an enemy once, as KH1's shared hit bit. Checked (decomp
/// + exe).
pub(super) const SHOT_SERIAL: u32 = 0x0015_6770;
/// Re:CoM's Blizzard side-shard effect sets (`0x1405A1790` = the row's set + 1). Checked (decomp
/// `0x140222170` case `0x23`).
const EFFECT_SETS_NEXT: u64 = 0x1_405A_1790;
/// `0x1401A6B20(actor, effect name, id, &position, &angles, 0.0)`: Re:CoM's Thunder case plays its
/// `_thunder_all` effects with it (`0x1402227A2`, `0x14022282C`). Checked (disasm).
const EFFECT_AT: u32 = 0x001A_6B20;
/// Re:CoM's Thunder bolt (`0x140222170` case `0x24`, Checked (disasm `0x140222748..0x14022293C`)):
/// the bolt effect `set − 10` at the target's x / z and Sora's height (`+0x504`) − 6 m, then a
/// `CBullet` (flags `0x47`, life 800 ticks) from Sora's height there, pitched straight down
/// (π/2), its radius `row[4]` × 0.1.
const THUNDER_DROP: f32 = 6.0;
const THUNDER_BOLT_FLAGS: u64 = 0x47;
const THUNDER_BOLT_LIFE: i32 = 800;
/// Re:CoM's own "collision held off" for a shot (flag `0x200000` with `desc+0x8C` ticks: the init
/// `0x140157E60` switches its attack off, `0x1401573C0` back on when the count runs out): our bolts
/// keep it off for good; their one hit is the direct strike. Checked (decomp).
const COLLISION_HELD: u64 = 0x20_0000;
const HELD_TICKS: i32 = 1_000_000;
/// The player's position vector (`+0x500`, its y read by the Thunder case). Checked (disasm).
const PLAYER_POS: usize = 0x500;
/// `CCollAttack`'s "off" (vtable +8: `+0x10` = 0) and apply (vtable +0x38). Checked (exe + decomp).
const COLL_OFF: u32 = 0x0014_DE80;
const COLL_APPLY: u32 = 0x0016_F1C0;
/// Re:CoM's `LockOnManager` (`0x14087B1E0`, made on first use by its users; its list `+0x88`:
/// nodes `{u32 handle, …, +0x10 next}`, the last one a sentinel with no next). Checked (decomp
/// + disasm `0x1401CDB90`, Re:CoM's own nearest-target pick for homing shots).
const LOCK_ON_MANAGER: u64 = 0x1_4087_B1E0;

/// Bullet and lock-on fields the re-target reads or writes. Checked (decomp `0x140155EB0`,
/// `0x140157E60`, `0x1401575D0`, `0x140156740`, `0x1401CDB90`).
pub(super) mod rt {
    /// The flight state's exe member-function pointer (`CStateTemplate<CBullet>` at `+0x2B0`,
    /// its exe `+0x20` / this-adjust `+0x28`, run by `0x140156740` each frame).
    pub const STATE_EXE: usize = 0x2D0;
    pub const STATE_EXE_ADJUST: usize = 0x2D8;
    /// The homing step returns at once while this is set.
    pub const HOLD: usize = 0x2F0;
    /// The life (ticks), counted down by Re:CoM.
    pub const LIFE: usize = 0x160;
    pub const LIST: usize = 0x88;
    pub const NODE_NEXT: usize = 0x10;
    /// The lock-on flag every entry of Re:CoM's own pick must have (`+0x154 & 0x10000`).
    pub const ACTOR_FLAGS: usize = 0x154;
    pub const LOCKABLE: u32 = 0x1_0000;
    /// Actor position (its mover's `+0x30`).
    pub const MOVER: usize = 0xF0;
    pub const POSITION: usize = 0x30;
    /// A mover's collision object and its test parameter (`win::movement::body`).
    pub const MOVER_COLL: usize = 0x158;
    pub const MOVER_COLL_PARAM: usize = 0x134;
}

pub(super) type PlFn = unsafe extern "win64" fn(usize);
type PlIntFn = unsafe extern "win64" fn(usize, i32);
type PlOutFn = unsafe extern "win64" fn(usize, *mut [f32; 4]);
type AnglesFn = unsafe extern "win64" fn(usize) -> *const [f32; 4];
pub(super) type DescInitFn = unsafe extern "win64" fn(*mut u8, i32, i32);
pub(super) type FactoryFn = unsafe extern "win64" fn() -> usize;
pub(super) type ManagerFn = unsafe extern "win64" fn(usize, *const u8, usize) -> usize;
pub(super) type MoverFn = unsafe extern "win64" fn(usize, f32, f32, i32, i32);
type GroundFn = unsafe extern "win64" fn(usize) -> u32;
pub(super) type SerialFn = unsafe extern "win64" fn(usize) -> u32;

/// The shot descriptor (`0x140156790` fills 0xE0 bytes; the bullet init reads up to `+0xD8`).
#[repr(C, align(16))]
pub(super) struct Desc(pub(super) [u8; 0x100]);

/// `CPlayer` fields the cast writes (`initMagic` `0x1402203D0`; `docs/recom-player.md`).
/// Checked (decomp).
mod f {
    pub const ROW_PTR: usize = 0x3C8;
    pub const CURSOR: usize = 0x3F8;
    pub const PREV_ACTION: usize = 0x408;
    pub const ACTION: usize = 0x40C;
    pub const ACTION_TIME: usize = 0x410;
    pub const SUB: usize = 0x414;
    pub const FLAGS_418: usize = 0x418;
    pub const REQUEST: usize = 0x424;
    pub const CARD: usize = 0x428;
    pub const ATTACK_STAT: usize = 0x43C;
    pub const CLIP_OFFSET: usize = 0x478;
    pub const SPEED_4DC: usize = 0x4DC;
    pub const VSPEED: usize = 0x5D4;
    pub const GRAVITY: usize = 0x5D8;
    pub const PHASE: usize = 0x604;
    pub const SITUATION: usize = 0x630;
    pub const PREV_SITUATION: usize = 0x634;
    pub const ATTACK_ID: usize = 0x638;
    pub const COUNTER: usize = 0x63C;
    pub const HIT: usize = 0x644;
    pub const CHAIN: usize = 0x64C;
    pub const RECOVERY: usize = 0x65C;
    pub const AP: usize = 0x660;
    pub const KIND: usize = 0x668;
    pub const ATTR: usize = 0x66C;
    pub const SLEIGHT_670: usize = 0x670;
    pub const CARD_7B4: usize = 0x7B4;
    pub const TARGET: usize = 0x84C;
}

/// Re:CoM's attributes of the spells (the magic cards' BattleCardList `+0xB`: Fire 2, Blizzard 3,
/// Thunder 4, Cure 6, Gravity 5, Stop 5, Aero 1; research `m7-magic.md` § 3.9, Checked (files)).
/// Re:CoM's own resistance reads them for enemies without KH1 numbers.
const RECOM_ATTR: [u32; 7] = [2, 3, 4, 6, 5, 5, 1];

fn note(line: impl FnOnce() -> String) {
    if LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("magic: {}", line()));
    }
}

fn event(e: String) {
    if let Ok(mut g) = EVENTS.lock() {
        if g.len() < 16 {
            g.push(e);
        }
    }
}

/// Builds the KH1 rows from Re:CoM's live Fire rows (which must be the Steam exe's) and patches
/// the Field exe's case for action `0x13`. The exeMagic hook is installed by the caller after.
pub unsafe fn install(b: usize) -> bool {
    let read = |ids: [usize; 3]| {
        let mut live = [[0u8; 16]; 3];
        for (r, id) in live.iter_mut().zip(ids) {
            *r = rd(va(b, ROWS) + 16 * id);
        }
        live
    };
    // Each built spell's Re:CoM rows (its three levels), which must be the Steam exe's.
    let want = mg::recom_rows();
    let mut live = Vec::with_capacity(want.len());
    for (s, rows) in want.iter().enumerate() {
        let ids = [1u8, 2, 3].map(|lv| mg::recom_base_row(s as u8, lv).unwrap_or(0));
        let got = read(ids);
        if got != *rows {
            log(&format!("magic: Re:CoM's {} rows {:#x} / {:#x} / {:#x} are not the Steam exe's; no KH1 cast", mg::spell::NAMES[s], ids[0], ids[1], ids[2]));
            return false;
        }
        live.push(got);
    }
    let t: &'static [[u8; 16]] = Box::leak(mg::kh1_magic_table(&live).into_boxed_slice());
    let rt: &'static [[u8; 16]] = Box::leak(mg::riku_look(t).into_boxed_slice());
    let _ = RIKU_TABLE.set(rt);
    if TABLE.set(t).is_err() {
        return false;
    }
    use crate::magic::field_case as fc;
    let cave = b + fc::CAVE_RVA as usize;
    let entry = b + fc::TABLE_ENTRY_RVA as usize;
    let old_entry = fc::DEFAULT_RVA.to_le_bytes();
    let have = |at: usize, n: usize| std::slice::from_raw_parts(at as *const u8, n).to_vec();
    if have(cave, fc::STUB_LEN) != [0xCC; fc::STUB_LEN] || have(entry, 4) != old_entry {
        log("magic: the Field exe's case 0x13 or its cave is not as expected; no KH1 cast");
        return false;
    }
    if !super::hook::patch(cave, &[0xCC; fc::STUB_LEN], &fc::stub(), "magic: Field case stub") {
        return false;
    }
    if !super::hook::patch(entry, &old_entry, &fc::entry(), "magic: Field jump table 0x13") {
        return false;
    }
    // 7.4b: KH1's re-target needs CHomingBullet's homing slot to be Re:CoM's homing step (the
    // thunk our wrapper calls goes through it). Without it the balls home as Re:CoM's.
    let slot_ok = crate::sites::MAGIC_RETARGET_PTRS.iter().all(|p| rd::<usize>(b + p.rva as usize) == b + p.target as usize);
    RETARGET.store(slot_ok, Ordering::Release);
    if !slot_ok {
        log("magic: CHomingBullet's homing slot is not 0x1401575D0; Fire balls cast with no target fly as Re:CoM's (no KH1 re-target)");
    }
    true
}

/// 0.184.0 (fork owner, 2026-11-23: Sora's KH1 spells for Riku): Re:CoM's Riku and Dark Riku have no magic
/// action: their `initMagic` / `exeMagic` slots (`+0x148` / `+0x150`) are the base class's empty `ret`s
/// (`0x1401F2F20` / `0x1401EEAA0`; Checked (exe: both vtables, the bytes)). [`start_cast`] writes `initMagic`'s
/// fields itself, so only the exe slot is pointed at [`exe_magic`], which runs the KH1 frame of a KH1 cast for
/// them and nothing otherwise (the stub's behaviour). Checked old values; not fatal.
pub unsafe fn riku_slots(b: usize) {
    let stub = (va(b, RIKU_EXE_MAGIC_STUB) as u64).to_le_bytes();
    let ours = (exe_magic as *const () as usize as u64).to_le_bytes();
    for (vt, who) in [(crate::sites::data::RIKU_VTABLE, "Riku"), (crate::sites::data::DRIKU_VTABLE, "Dark Riku")] {
        let slot = va(b, vt) + 0x150;
        if std::slice::from_raw_parts(slot as *const u8, 8) == stub {
            super::hook::patch(slot, &stub, &ours, "magic: Riku's exeMagic slot");
        } else {
            log(&format!("magic: {who}'s exeMagic slot is not Re:CoM's empty 0x1401EEAA0; no KH1 cast for him"));
        }
    }
}

/// Re:CoM's empty member function in Riku's and Dark Riku's `exeMagic` slot.
const RIKU_EXE_MAGIC_STUB: u64 = 0x1_401E_EAA0;

/// 0.184.0: Riku's spells cost nothing (`riku_free_magic`, the owner's test setting) for the main player.
pub unsafe fn riku_free() -> bool {
    super::settings().riku_free_magic && mv::is_main_riku(rd(va(base(), crate::sites::data::MAIN_PLAYER)))
}

/// A spell's MP at its release ([`super::mp::pay`]), nothing for Riku with `riku_free_magic`.
fn pay_spell(cost: i32, why: &str) {
    // SAFETY: reads of the image's main-player pointer and its vtable word.
    if unsafe { riku_free() } {
        note(|| format!("{why}: free (riku_free_magic)"));
        return;
    }
    super::mp::pay(cost, why);
}

pub fn mark_installed() {
    INSTALLED.store(true, Ordering::Release);
}

/// 0.184.5: the level of the cast started last (Riku's cast rows play their effects at it).
static CAST_LEVEL: AtomicU32 = AtomicU32::new(1);

/// The level of the cast started last, 1..=3.
pub fn cast_level() -> u32 {
    CAST_LEVEL.load(Ordering::Relaxed).clamp(1, 3)
}

/// The KH1 cast is installed (the menu lists KH1 spells only then).
pub fn installed() -> bool {
    INSTALLED.load(Ordering::Acquire)
}

/// A KH1 magic command from the command menu (and the shortcuts, stage 7.12): taken by Sora's
/// KH1 frame this frame, if it runs (else dropped at the next frame's start, as KH1 drops a
/// command Sora can't take).
pub fn issue(spell: u8, level: u8) {
    if !INSTALLED.load(Ordering::Acquire) {
        note(|| format!("{}{level}: no KH1 cast installed", mg::spell::NAMES.get(spell as usize).copied().unwrap_or("?")));
        return;
    }
    if let Ok(mut g) = PENDING.lock() {
        *g = Some(MagicCommand { spell, level });
    }
}

/// The start of Sora's frame (the request-bits wrapper, before the menu): a command no KH1 frame
/// took is gone.
pub fn new_frame() {
    if let Ok(mut g) = PENDING.try_lock() {
        if let Some(m) = g.take() {
            note(|| format!("{}{}: dropped (no KH1 frame ran: Re:CoM's action)", mg::spell::NAMES.get(m.spell as usize).copied().unwrap_or("?"), m.level));
        }
    }
}

/// The command for this KH1 frame (`win::movement::run_frame`).
pub fn take_command() -> Option<MagicCommand> {
    PENDING.try_lock().ok().and_then(|mut g| g.take())
}

/// The ground Fire release's clock (decision 38, `crate::magic::release_root`), and whether it has
/// seen the release clip since it was last reset (one atomic load otherwise).
static RELEASE_CLOCK: Mutex<mg::release_root::Clock> = Mutex::new(mg::release_root::Clock::new());
static RELEASE_SEEN: AtomicBool = AtomicBool::new(false);
static RELEASE_LINES: AtomicU32 = AtomicU32::new(20);
/// The motion controller's time (`ctrl+0x3C`, ticks: 50 a 60 Hz frame; `0x14024CCC0` reads it,
/// `win::movement`'s motion frame too). Checked (decomp).
const CTRL_TIME: usize = 0x3C;

/// Decision 38: in the root-step wrapper, for an update of Sora's KH1 cast that plays the ground
/// release clip (row 0x38, `p001_063`): Re:CoM's root position for the update (the anchor + its
/// clip's travel) becomes the anchor + **KH1's** travel between the clip's last time and its time
/// now (`crate::magic::release_root`), on the line of Re:CoM's own travel. Any other row of a cast:
/// Re:CoM's travel stands (Checked (files): the other cast clips carry KH1's travel or none).
pub unsafe fn release_root(pl: usize, ctrl: usize, anchor: kh1_core::Vec3, root: kh1_core::Vec3) -> kh1_core::Vec3 {
    if rd::<u32>(pl + p::ROW) != mg::release_root::RECOM_ROW {
        release_root_off();
        return root;
    }
    let Ok(mut c) = RELEASE_CLOCK.try_lock() else { return root };
    RELEASE_SEEN.store(true, Ordering::Relaxed);
    let (t0, t1) = c.span(ctrl, crate::units::units_to_frames(rd(ctrl + CTRL_TIME)));
    drop(c);
    let body: usize = rd(pl + p::BODY);
    let yaw: f32 = if body != 0 { rd(body + mv::body::YAW) } else { 0.0 };
    let travel = mg::release_root::travel(root - anchor, kh1_core::Vec3::from_yaw(yaw), t0, t1);
    if RELEASE_LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        let r = root - anchor;
        log(&format!(
            "magic: fire release root: KH1 frames {t0:.1} -> {t1:.1}: Re:CoM's clip ({:.4},{:.4},{:.4}) -> KH1's ({:.4},{:.4},{:.4})",
            r.x, r.y, r.z, travel.x, travel.y, travel.z
        ));
    }
    anchor + travel
}

/// Not the release clip this update (the root-step wrapper's other paths): its clock starts again.
pub fn release_root_off() {
    if RELEASE_SEEN.swap(false, Ordering::Relaxed) {
        if let Ok(mut c) = RELEASE_CLOCK.try_lock() {
            c.reset();
        }
    }
}

/// The KH1 row `+0x3C8` points at, if it points into our magic table: a KH1 cast.
pub unsafe fn kh1_cast(pl: usize) -> bool {
    let ptr: usize = rd(pl + f::ROW_PTR);
    [TABLE.get(), RIKU_TABLE.get()].into_iter().flatten().any(|t| {
        let start = t.as_ptr() as usize;
        ptr >= start && ptr < start + t.len() * 16
    })
}

/// M12 step 3 (Leaf Bracer): Sora's KH1 Cure is casting: Re:CoM's magic action (`initMagic`'s
/// `+0x40C` = 0x13, which [`start_cast`] writes and the KH1 frame holds while the cast motion plays)
/// with a KH1 Cure row of our table (`+0x3C8`).
pub unsafe fn kh1_cure_casting(pl: usize) -> bool {
    if rd::<u32>(pl + f::ACTION) != mg::RECOM_MAGIC_ACTION {
        return false;
    }
    let ptr: usize = rd(pl + f::ROW_PTR);
    (1..=3).any(|lv| row(mg::spell::CURE, lv).is_some_and(|r| r.as_ptr() as usize == ptr))
}

fn row(s: u8, level: u8) -> Option<&'static [u8; 16]> {
    // SAFETY: reads of the image's main-player pointer and its vtable word.
    let riku = unsafe { mv::is_main_riku(rd(va(base(), crate::sites::data::MAIN_PLAYER))) };
    let t = if riku { RIKU_TABLE.get().or(TABLE.get())? } else { TABLE.get()? };
    t.get(mg::table_index(s, level)?).filter(|r| r[0] != 0)
}

/// KH1's usable test (`0x14028EFD0`): the spell is learned (the menu offers only learned ones),
/// and MP pays its cost (`0x1402A4C10`).
pub fn can_cast(s: u8, level: u8) -> bool {
    // SAFETY: reads of the image's main-player pointer and its vtable word.
    row(s, level).is_some() && mg::castable(s) && (unsafe { riku_free() } || super::mp::can_pay(mg::cost(s)))
}

/// `initMagic`'s writes for a KH1 cast, cardless (Checked (decomp `0x1402203D0`), in its order):
/// the action, the attack fields, the target cursor's busy bit, the homing target, the KH1 row
/// and its AP / kind / attribute. Not written: the air hover (`+0x418 |= 0x2800000`, gravity ×
/// 0.16: KH1's air cast hops instead) and the card modifier (no card). The clip is KH1's motion
/// request, played by the KH1 frame.
pub unsafe fn start_cast(pl: usize, c: &CastStart) -> bool {
    let Some(r) = row(c.spell, c.level) else { return false };
    CAST_LEVEL.store(c.level as u32, Ordering::Relaxed);
    let cur: u32 = rd(pl + f::ACTION);
    wr(pl + f::PREV_ACTION, cur);
    wr(pl + f::ACTION_TIME, 0u64);
    wr(pl + f::ACTION, mg::RECOM_MAGIC_ACTION);
    wr(pl + f::CARD, rd::<u32>(pl + f::CARD) & 0xFFF7_FEF7);
    wr(pl + f::REQUEST, rd::<u32>(pl + f::REQUEST) & !0x20);
    wr(pl + f::HIT, 0u64);
    wr(pl + f::CHAIN, 0u64);
    wr(pl + f::CARD_7B4, 0u32);
    wr(pl + f::COUNTER, 0u32);
    // `0x1401A38D0`: the body's velocity to zero (the KH1 frame writes its own after).
    let body: usize = rd(pl + p::BODY);
    if body != 0 {
        wr(body + 0x80, [0f32, 0.0, 0.0, 1.0]);
    }
    wr(pl + f::SITUATION, 0u32);
    wr(pl + f::FLAGS_418, rd::<u32>(pl + f::FLAGS_418) & 0xFD7F_FFFF);
    wr(pl + f::GRAVITY, rd::<f32>(va(base(), BASE_GRAVITY)));
    wr(pl + f::PHASE, mg::MAGIC_PHASE);
    wr(pl + f::SPEED_4DC, 0u64);
    wr(pl + f::VSPEED, 0f32);
    let cursor: usize = rd(pl + f::CURSOR);
    if cursor != 0 {
        func::<PlIntFn>(CURSOR_BUSY)(cursor, 8);
    }
    func::<PlFn>(HOMING_TARGET)(pl);
    if c.spell == mg::spell::THUNDER {
        // `initMagic` case 0x1C's charge (M7 f4): the set's effect id 0 on the Keyblade's bone 2 at
        // (−0.8, 0, 0), kept on it (`+0x4C |= 4`). Checked (disasm `0x14022062B..0x140220697`).
        effect_on_bone(pl, rd(pl + WEAPON), 2, set_name(r, 0), 0, &CHARGE_OFFSET, true);
        thunder_command(pl);
    }
    // 0.184.8: Riku's Cure has no `cure_all` charge (his heal sparkle at the release instead).
    if (c.spell == mg::spell::CURE && !mv::is_main_riku(pl)) || c.spell == mg::spell::AERO {
        // `initMagic` case 0x1D / 0x20's charge: the set's effect id 0 on the Keyblade's bone 2.
        charge_on_weapon(pl, r, 0);
    }
    if c.spell == mg::spell::STOP {
        // `initMagic` case 0x1F's charge: the set's effect id 0 on Sora's bone 0 at (0, 0.85, 0),
        // kept on him; and the command's target for KH1's centre.
        effect_on_bone(pl, pl, 0, set_name(r, 0), 0, &BODY_OFFSET, true);
        if let Ok(mut g) = STOP_CMD.lock() {
            *g = Some(rd(pl + f::TARGET));
        }
    }
    if c.spell == mg::spell::GRAVITY {
        // `initMagic` case 0x1E's charge (the set's `+0x30` entry) and KH1's marker point.
        charge_on_weapon(pl, r, 0x18);
        gravity_command(pl, r);
    }
    wr(pl + f::RECOVERY, 0u32);
    // `+0x478` picks the air clips (+4) for Re:CoM's own readers; KH1's motion requests choose
    // the clip here.
    let air = func::<GroundFn>(GROUND_TEST)(pl) != 0;
    wr(pl + f::CLIP_OFFSET, if air { 4u32 } else { 0 });
    let id = mg::recom_attack_id(c.spell);
    wr(pl + f::ATTACK_ID, id);
    wr(pl + f::ROW_PTR, r.as_ptr() as usize);
    // Re:CoM's AP for a target without KH1 numbers: the cardless AP (the attack stat), as M5's
    // swings (KH1's damage replaces it on twins and proxies, `take_hit`).
    let ap = match c.spell {
        // Cure: Re:CoM's heal (decision 34), its AP rule for the card / sleight (power / 10).
        mg::spell::CURE => mg::cure::heal(c.level),
        _ => rd::<i32>(pl + f::ATTACK_STAT),
    };
    wr(pl + f::AP, ap);
    // Enemy Skills route 2: Cure's heal through Re:CoM's hit rewrite, as `initMagic` runs it for
    // Re:CoM's Cure (Green Requiem, Wizard; battle only).
    if c.spell == mg::spell::CURE {
        wr(pl + f::AP, super::enemy_skills::spell_rewrite(pl, c.spell, c.level, ap));
    }
    wr(pl + f::KIND, r[0xB] as u32);
    wr(pl + f::ATTR, RECOM_ATTR[c.spell as usize % 7]);
    // Damage = Re:CoM (owner, 2026-11-01), in battle: Fire / Blizzard / Thunder take their Re:CoM
    // magic card's (level 1) or sleight's (2, 3) AP and attribute, as `initMagic` /
    // `initStockSkill` compute them, and Re:CoM's enemy-card rewrite. The shots carry them.
    super::damage::spell(pl, c.spell, c.level);
    wr(pl + f::PREV_SITUATION, 0u32);
    wr(pl + f::SUB, id);
    wr(pl + f::SLEIGHT_670, 0u64);
    wr(pl + f::FLAGS_418, rd::<u32>(pl + f::FLAGS_418) | 0x10_0000);
    wr(pl + f::ACTION_TIME, 0u32);
    true
}

/// KH1's release: Re:CoM's spawn's Fire case (`0x140222170` case `0x22`, Checked (disasm
/// `0x14022220B..0x14022234D`)) with the KH1 row and KH1's life, then KH1's MP payment. The
/// shot's id, if Re:CoM made one.
pub unsafe fn release(pl: usize, c: &CastStart) -> Option<u32> {
    match c.spell {
        mg::spell::FIRE => release_fire(pl, c),
        mg::spell::BLIZZARD => release_blizzard(pl, c),
        mg::spell::THUNDER => release_thunder(pl, c),
        mg::spell::CURE => release_cure(pl, c),
        mg::spell::GRAVITY => release_gravity(pl, c),
        mg::spell::STOP => release_stop(pl, c),
        mg::spell::AERO => release_aero(pl, c),
        _ => None,
    }
}

/// Re:CoM's Aero shot flags (`mov qword [rbp−0x50],7`) and its pitch (−π/2 at `0x140520910`).
/// Checked (disasm `0x140222DFE`, exe).
const AERO_FLAGS: u64 = 7;
/// `0x14016F7D0(attack)`: Re:CoM's new hit serial for an attack (its multi-hit attacks call it to
/// hit again; the apply's hit-once ring `0x14016F960` then lets the attack through once more).
/// Checked (decomp; 78 call sites).
pub(super) const NEW_SERIAL: u32 = 0x0016_F7D0;
/// `0x1401A49B0(actor)`: its position (the Aero case's point). Checked (decomp).
const ACTOR_POSITION: u32 = 0x001A_49B0;

/// KH1's barrier on Sora (7.10): its countdown, and Re:CoM's Aero shots that carry its look and the
/// Aerora / Aeroga wind, following Sora while it lasts.
struct BarrierState {
    b: mg::aero::Barrier,
    /// (shot id, has the wind), at most 4 (one per cast while the barrier lasts, as KH1 starts a
    /// barrier thread per cast).
    shots: Vec<(u32, bool)>,
}
static BARRIER: Mutex<Option<BarrierState>> = Mutex::new(None);
/// The barrier is on (the player damage check reads it once per call).
static BARRIER_ON: AtomicBool = AtomicBool::new(false);
static BARRIER_LINES: AtomicU32 = AtomicU32::new(40);

fn barrier_line(line: impl FnOnce() -> String) {
    if BARRIER_LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("magic: {}", line()));
    }
}

/// Whether KH1's Aero barrier is on Sora (the damage check halves his hits, `win::attack`).
pub fn barrier_on() -> bool {
    BARRIER_ON.load(Ordering::Acquire)
}

/// KH1's Aero release (7.10, `xm_air01.mag`): the target is Sora (no party members yet), cast only if
/// he is alive; Re:CoM's Aero case's look (`0x140222170` case `0x28`, Checked (disasm
/// `0x140222CB0..0x140222E7A`): its hand effect id 1, on the ground its effect set − 0x16, and set −
/// 0x11, at Sora's position); **KH1's barrier** ((18 + max MP) × 60 frames; a re-cast rewrites it),
/// read by Re:CoM's player damage check through our wrapper; and Re:CoM's Aero shot (`CBullet`, flags
/// 7, the row's `airo_all` set and radius) **following Sora for the whole barrier**: held off for
/// Aero (its look only), for Aerora / Aeroga KH1's wind (radius 0.8 m, re-hit every 30 frames
/// through Re:CoM's own new-serial function, `aero_frame`). Then the MP (two points).
unsafe fn release_aero(pl: usize, c: &CastStart) -> Option<u32> {
    let r = row(c.spell, c.level)?;
    let hp: i32 = rd(pl + HP);
    if !mg::cure::target_ok(hp, rd::<i32>(pl + DEAD) != 0) {
        note(|| format!("release aero{}: Sora can't take the barrier (HP {hp}): nothing, no MP", c.level));
        return None;
    }
    let max_mp = super::mp::current().map_or(0, |v| v.1);
    // Re:CoM's Aero case's effects: id 1 at the hand, the ground wind (set − 0x16) when on the
    // ground, then set − 0x11, at Sora's position.
    let body: usize = rd(pl + p::BODY);
    let mut hand = [0f32; 4];
    if body != 0 {
        let b = rd_vec(body + 0x30);
        hand = [b.x, b.y, b.z, 1.0];
    }
    func::<PlOutFn>(HAND_POINT)(pl, &mut hand);
    let name = set_name(r, 0);
    effect_at(pl, name, 1, hand);
    type PosFn = unsafe extern "win64" fn(usize) -> *const [f32; 4];
    let at = func::<PosFn>(ACTOR_POSITION)(pl);
    let at: [f32; 4] = if at.is_null() { hand } else { std::ptr::read_unaligned(at) };
    if func::<GroundFn>(GROUND_TEST)(pl) == 0 {
        effect_at(pl, name, (r[2] as u32).wrapping_sub(0x16), at);
    }
    effect_at(pl, name, (r[2] as u32).wrapping_sub(0x11), at);
    // The barrier (a re-cast rewrites the timer), and this cast's shot.
    // Aerora / Aeroga: KH1's wind; Aero (level 1): Aerora's wind with the Aero card's value at
    // Damage = Re:CoM when the cast took it (owner, "1 (b)"; `crate::damage::aero_wind`).
    let wind = crate::damage::aero_wind(c.level, super::damage::recom_cast(mg::spell::AERO));
    let frames = mg::aero::frames(max_mp);
    let c3 = sora_centre(pl);
    let shot = aero_shot(pl, r, c.level, wind, frames, [c3[0], c3[1], c3[2], 1.0]);
    if let Ok(mut g) = BARRIER.lock() {
        let st = g.get_or_insert_with(|| BarrierState { b: mg::aero::Barrier::new(max_mp), shots: Vec::new() });
        st.b.left = frames;
        if let Some(id) = shot {
            if st.shots.len() >= 4 {
                st.shots.remove(0);
            }
            st.shots.push((id, wind));
        }
        BARRIER_ON.store(true, Ordering::Release);
        STRIKING.store(true, Ordering::Release);
    }
    pay_spell(mg::aero::COST, "Aero release");
    note(|| format!("release aero{}: KH1's barrier {frames} f (max MP {max_mp}), damage taken halved; Re:CoM's Aero shot {shot:?} follows Sora{}", c.level, if !wind { " (look only)" } else if c.level == 1 { ", Aerora's KH1 wind 0.8 m re-hitting every 30 f with the Aero card's AP (Damage = Re:CoM)" } else { ", KH1's wind 0.8 m re-hitting every 30 f" }));
    event(format!("barrier {frames:.0}{}", if wind { " wind" } else { "" }));
    shot
}

/// Re:CoM's Aero shot (the case's `CBullet`: flags 7, pitch −π/2, the row's set) at Sora, alive for the
/// barrier; for Aerora / Aeroga KH1's wind (radius 80 cm × Sora's size, Assumption: 1.0; the KH1
/// row's kind 1 and the cast's AP / attribute), else its attack held off.
unsafe fn aero_shot(pl: usize, r: &[u8; 16], level: u8, wind: bool, frames: f32, at: [f32; 4]) -> Option<u32> {
    let ang = func::<AnglesFn>(ANGLES)(pl);
    let ang: [f32; 4] = if ang.is_null() { [0.0; 4] } else { std::ptr::read_unaligned(ang) };
    let mut d = Desc([0; 0x100]);
    let dp = d.0.as_mut_ptr();
    func::<DescInitFn>(DESC_INIT)(dp, 1, 3);
    let w = |o: usize| dp.add(o);
    // The cast's attack fields (`start_cast`): the KH1 row's kind, and the AP / attribute it wrote
    // (the cardless AP and Re:CoM's aero attribute 1; at Damage = Re:CoM in battle, Re:CoM's Aero
    // card's / Aerora's / Aeroga's AP and attribute, `super::damage::spell`), carried by every
    // re-hit of the wind.
    std::ptr::write_unaligned(w(0x68) as *mut u32, rd::<u32>(pl + f::KIND));
    std::ptr::write_unaligned(w(0x6C) as *mut u32, rd::<u32>(pl + f::ATTR));
    std::ptr::write_unaligned(w(0x70) as *mut i32, rd::<i32>(pl + f::AP));
    std::ptr::write_unaligned(w(0x0C) as *mut u32, 0x8000_0000);
    let flags = if wind { AERO_FLAGS | 4 } else { AERO_FLAGS | 4 | COLLISION_HELD };
    std::ptr::write_unaligned(w(0x60) as *mut u64, flags);
    if !wind {
        std::ptr::write_unaligned(w(0x8C) as *mut i32, HELD_TICKS);
    }
    std::ptr::write_unaligned(w(0x30) as *mut f32, 0.0);
    std::ptr::write_unaligned(w(0x34) as *mut f32, 255.0);
    std::ptr::write_unaligned(w(0x3C) as *mut i32, (frames * 50.0) as i32);
    std::ptr::write_unaligned(w(0x10) as *mut [f32; 4], [at[0], at[1], at[2], 1.0]);
    std::ptr::write_unaligned(w(0x20) as *mut [f32; 4], [-std::f32::consts::FRAC_PI_2, ang[1], ang[2], ang[3]]);
    std::ptr::write_unaligned(w(0xA8) as *mut usize, va(base(), EFFECT_SETS) + r[2] as usize * 0x60);
    let radius = if wind { mg::aero::WIND_RADIUS / 100.0 } else { r[4] as f32 * 0.1 };
    std::ptr::write_unaligned(w(0x78) as *mut f32, radius);
    let factory = func::<FactoryFn>(FACTORY)();
    if factory == 0 {
        return None;
    }
    let bullet = func::<ManagerFn>(SHOT_MANAGER)(factory, dp, pl);
    if bullet == 0 {
        barrier_line(|| "aero: Re:CoM made no shot (the barrier still halves damage)".into());
        return None;
    }
    func::<MoverFn>(SHOT_MOVER)(bullet, 0.1, 0.0, 0, -1);
    if !wind {
        hold_off(bullet);
    }
    let id: u32 = rd(bullet + 0x20);
    remember_shot(id, mg::spell::AERO, level, 0);
    Some(id)
}

/// Sora's body centre for the barrier's shot: his position (`0x1401A49B0`) + 0.85 m up, the body
/// offset Re:CoM's Cure and Stop cases put their effects on him with (KH1: bone 32771, the body
/// centre; Assumption: the same point).
unsafe fn sora_centre(pl: usize) -> [f32; 3] {
    type PosFn = unsafe extern "win64" fn(usize) -> *const [f32; 4];
    let at = func::<PosFn>(ACTOR_POSITION)(pl);
    let p: [f32; 4] = if at.is_null() { [0.0; 4] } else { std::ptr::read_unaligned(at) };
    [p[0] + BODY_OFFSET[0], p[1] + BODY_OFFSET[1], p[2] + BODY_OFFSET[2]]
}

/// One frame of the barrier (Sora's frame start): its countdown; each of its shots at Sora's aim
/// point (KH1: the body centre) with its life set to what is left; every 30 frames the wind's attack
/// gets Re:CoM's new hit serial (KH1's re-hit `+0x50`); at the end the shots' lives run out (Re:CoM
/// ends and fades them) and the halving stops.
unsafe fn aero_frame(pl: usize, step: f32) {
    let Ok(mut g) = BARRIER.lock() else { return };
    let Some(st) = g.as_mut() else { return };
    let (on, again) = st.b.step(step);
    let centre = sora_centre(pl);
    let left_ticks = if on { (st.b.left * 50.0) as i32 } else { 0 };
    st.shots.retain(|&(id, wind)| {
        let Some(o) = shot_object(id) else { return false };
        let mover: usize = rd(o + rt::MOVER);
        if mover != 0 {
            let p: [f32; 4] = rd(mover + rt::POSITION);
            wr(mover + rt::POSITION, [centre[0], centre[1], centre[2], p[3]]);
        }
        wr(o + rt::LIFE, left_ticks);
        if wind && again && on {
            let c: usize = rd(o + 0x2A8);
            if c != 0 {
                func::<PlFn>(NEW_SERIAL)(c);
            }
        }
        on
    });
    if !on {
        *g = None;
        BARRIER_ON.store(false, Ordering::Release);
        barrier_line(|| "aero: the barrier is over".into());
        event("barrier off".into());
    }
}

/// Re:CoM's Stop shot flags with a target (`mov qword [rbp−0x50],0x25`). Checked (disasm
/// `0x140222BEC`).
const STOP_FLAGS: u64 = 0x25;
/// Stop (7.9): the command's target handle, for the release.
static STOP_CMD: Mutex<Option<u32>> = Mutex::new(None);

/// KH1's Stop release (7.9, `xm_stop01.mag`): the centre (the target's aim point if it is alive,
/// else Sora's position), Re:CoM's lock-on enemies sorted nearest first from it, the strike thread
/// (one enemy inside the 2.5 / 3.5 / 5 m, ±3 m cylinder every 4 frames, [`stop_strike`]), the
/// first run now, then the MP (two points). The first strike's shot id.
unsafe fn release_stop(pl: usize, c: &CastStart) -> Option<u32> {
    let h = STOP_CMD.lock().ok().and_then(|mut g| g.take()).unwrap_or(0);
    let centre = match target_enemy(h) {
        Some(e) => aim_point(e),
        None => sora_pos_dir(pl).0,
    };
    let enemies: Vec<(u32, [f32; 3])> = lock_on_targets(0).into_iter().filter_map(|(h, _)| Some((h, position(target_enemy(h)?)?))).collect();
    let order = mg::thunder_order(centre, enemies);
    let n = order.len();
    let run = ThunderRun { s: mg::Strikes::every(order, mg::stop::INTERVAL), spell: mg::spell::STOP, area: mg::stop::area(c.level), level: c.level, centre, kind: rd(pl + f::KIND), attr: rd(pl + f::ATTR), ap: rd(pl + f::AP) };
    let first = match STRIKES.lock() {
        Ok(mut g) => {
            if g.len() >= 8 {
                g.remove(0);
            }
            g.push(run);
            STRIKING.store(true, Ordering::Release);
            let i = g.len() - 1;
            drop(g);
            strike_step(pl, 0.0, Some(i)).into_iter().next()
        }
        Err(_) => None,
    };
    // KH1 pays after the thread starts (sys 20, then sys 47).
    pay_spell(mg::stop::COST, "Stop release");
    note(|| format!("release stop{}: centre ({:.2},{:.2},{:.2}) r {} m ±3 m, {n} enemies listed, first {first:?}", c.level, centre[0], centre[1], centre[2], mg::stop::RADIUS[c.level.clamp(1, 3) as usize - 1] / 100.0));
    first
}

/// One Stop strike (KH1's sys 60 with the Stop data): **Re:CoM's Stop case for this enemy**
/// (`0x140222170` case `0x27` with a target, Checked (disasm `0x140222BBA..0x140222CB0`)): its
/// `CBullet` (flags `0x25`, speed 0, the row's range / life / radius × 0.1, its `stop_all` set) at the
/// enemy's aim point, its attack held off; then Re:CoM's contact and apply onto the enemy's first
/// live hurt body (7.6's mechanism, [`strike_body`]). Re:CoM's hit handler then runs its kind 8 (the stop state) with KH1's
/// duration (`spell_hit`). The shot's id.
#[allow(clippy::too_many_arguments)]
unsafe fn stop_strike(pl: usize, enemy: usize, h: u32, level: u8, kind: u32, attr: u32, ap: i32) -> Option<u32> {
    let r = row(mg::spell::STOP, level)?;
    let aim = aim_point(enemy);
    let ang = func::<AnglesFn>(ANGLES)(pl);
    let ang: [f32; 4] = if ang.is_null() { [0.0; 4] } else { std::ptr::read_unaligned(ang) };
    let mut d = Desc([0; 0x100]);
    let dp = d.0.as_mut_ptr();
    func::<DescInitFn>(DESC_INIT)(dp, 1, 3);
    let w = |o: usize| dp.add(o);
    std::ptr::write_unaligned(w(0x68) as *mut u32, kind);
    std::ptr::write_unaligned(w(0x6C) as *mut u32, attr);
    std::ptr::write_unaligned(w(0x70) as *mut i32, ap);
    std::ptr::write_unaligned(w(0x0C) as *mut u32, 0x8000_0000);
    std::ptr::write_unaligned(w(0x60) as *mut u64, STOP_FLAGS | COLLISION_HELD);
    std::ptr::write_unaligned(w(0x8C) as *mut i32, HELD_TICKS);
    std::ptr::write_unaligned(w(0x30) as *mut f32, 0.0);
    std::ptr::write_unaligned(w(0x34) as *mut f32, r[0xC] as f32);
    std::ptr::write_unaligned(w(0x3C) as *mut i32, r[0xE] as i32 * 100);
    std::ptr::write_unaligned(w(0x10) as *mut [f32; 4], [aim[0], aim[1], aim[2], 1.0]);
    std::ptr::write_unaligned(w(0x20) as *mut [f32; 4], ang);
    std::ptr::write_unaligned(w(0xA8) as *mut usize, va(base(), EFFECT_SETS) + r[2] as usize * 0x60);
    std::ptr::write_unaligned(w(0x78) as *mut f32, r[4] as f32 * 0.1);
    let factory = func::<FactoryFn>(FACTORY)();
    if factory == 0 {
        return None;
    }
    let bullet = func::<ManagerFn>(SHOT_MANAGER)(factory, dp, pl);
    if bullet == 0 {
        bolt_line(|| format!("stop on {h:#x}: Re:CoM made no shot"));
        return None;
    }
    func::<MoverFn>(SHOT_MOVER)(bullet, 0.1, 0.0, 0, -1);
    hold_off(bullet);
    let id: u32 = rd(bullet + 0x20);
    remember_shot(id, mg::spell::STOP, level, 0);
    let res = apply_to(bullet, enemy);
    bolt_line(|| format!("stop on {h:#x}: shot {id}, Re:CoM's contact and apply -> {res:?}"));
    Some(id)
}

/// Re:CoM's enemy fields Stop reads or writes (Checked (decomp `0x14017DC50`, `0x14017D260`,
/// `0x14017C740`, `0x1401843A0`)): the param block (`+0x308`; its `+0x2C` is Re:CoM's Stop gate),
/// the next state (`+0x68`), the stop state (`+0x940`), the state flags (`+0x260`, bit 19 while
/// stopped).
pub mod enemy_stop {
    pub const PARAM: usize = 0x308;
    pub const STOP_GATE: usize = 0x2C;
    pub const NEXT_STATE: usize = 0x68;
    pub const STOP_STATE: usize = 0x940;
    pub const FLAGS: usize = 0x260;
    pub const STOPPED: u64 = 0x8_0000;
    /// The duration kind 8 stores (`mov [rbx+0x2D8],eax`, `0x14017DCCA`), copied into the stop
    /// state's timer `+0x160` by its begin (`0x14017C7F4`); both i32. Checked (disasm).
    pub const DURATION: usize = 0x2D8;
}

/// Re:CoM's Gravity flatten state (`enemy+0xAC0`, entered by the reaction picker's case 0xD,
/// `0x14017D626`; its exe `0x140183BA0` squashes the height to the timer ÷ 250 (≥ 0.1), holds it
/// 9000 ticks, then sets it back to 1.0). Checked (decomp + disasm).
const FLATTEN_STATE: usize = 0xAC0;

/// Whether the enemy is in (or entering) Re:CoM's Gravity flatten: its height is Re:CoM's then
/// (KH1's squash under a falling sphere leaves it, as KH1's own squash skips its flattened
/// actors, requests 0x66..0x68 in `0x140297FE0`).
pub unsafe fn flattening(enemy: usize) -> bool {
    use enemy_stop::NEXT_STATE;
    let st = enemy + FLATTEN_STATE;
    rd::<usize>(enemy + held::CUR_STATE) == st || rd::<usize>(enemy + NEXT_STATE) == st
}

/// Whether Re:CoM's hit handler has just sent the enemy to its stop state (`+0x68` = `+0x940`,
/// the reaction picker's case 8, `0x14017D28A`); its begin runs at the next state switch.
pub unsafe fn stop_entering(enemy: usize) -> bool {
    use enemy_stop::*;
    rd::<usize>(enemy + NEXT_STATE) == enemy + STOP_STATE
}

/// KH1's whole Stop duration (ticks) into Re:CoM's `+0x2D8`, the field its kind 8 just wrote,
/// before the stop state's begin copies it into its timer. Only after [`stop_entering`].
pub unsafe fn stop_timer(enemy: usize, ticks: i32) {
    wr(enemy + enemy_stop::DURATION, ticks);
}

/// `0x1401A2D90(enemy, -1)`: every attack body of the enemy off (`+0x94` = 0), as Re:CoM's own
/// reaction states do (`0x14017C350`). Checked (decomp).
const ATTACK_BODIES_OFF: u32 = 0x001A_2D90;

/// The stopped enemy's own attack bodies off (KH1 skips the attacks a stopped actor owns,
/// `0x1402B47A0`): once, when Re:CoM's hit handler has just sent it to its stop state.
pub unsafe fn stop_attacks_off(enemy: usize) -> bool {
    if !stop_entering(enemy) {
        return false;
    }
    func::<PlIntFn>(ATTACK_BODIES_OFF)(enemy, -1);
    true
}

/// Re:CoM's Stop gate (param `+0x2C`) for one hit: KH1's `resist[9]` decides instead (open when it
/// is above 0). The old value, to put back.
pub unsafe fn stop_gate(enemy: usize, open: bool) -> Option<(usize, i32)> {
    use enemy_stop::*;
    let param: usize = rd(enemy + PARAM);
    if param == 0 {
        return None;
    }
    let old: i32 = rd(param + STOP_GATE);
    wr(param + STOP_GATE, open as i32);
    Some((param, old))
}

/// Puts Re:CoM's Stop gate back.
pub unsafe fn stop_gate_back(g: Option<(usize, i32)>) {
    if let Some((param, old)) = g {
        wr(param + enemy_stop::STOP_GATE, old);
    }
}

/// Whether Re:CoM's stop state holds this enemy (its hits are held, `0x1401843A0`).
pub unsafe fn stopped(enemy: usize) -> bool {
    rd::<u64>(enemy + enemy_stop::FLAGS) & enemy_stop::STOPPED != 0
}

/// Re:CoM's stop exe's landing fields (M7 f3; Checked (decomp `0x1401843A0` + disasm `0x140184910`,
/// `0x14018499D`)): the enemy's current state (`+0x60`, the hit handler's own stop test), the exe's
/// phase (`+0x270`: 1 lands the held hit `+0x33E`, 2 waits for the timer), the timer (`+0x160`,
/// counted down by the actor update `0x1401A59A0` by the enemy's dt `+0x14` before its hit handler),
/// the held count (`+0x33C`, at most 8) and the index (`+0x33E`).
mod held {
    pub const CUR_STATE: usize = 0x60;
    pub const PHASE: usize = 0x270;
    pub const WAITING: i32 = 2;
    pub const TIMER: usize = 0x160;
    pub const COUNT: usize = 0x33C;
    pub const INDEX: usize = 0x33E;
    pub const DT: usize = 0x14;
}

/// The enemies whose held hits are landing: (enemy, the landing already paced), at most 32; and
/// whether there are any (one load per enemy update otherwise).
static PACED: Mutex<Vec<(usize, i16)>> = Mutex::new(Vec::new());
static PACED_ANY: AtomicBool = AtomicBool::new(false);
/// KH1's `rand()` (`0x14031BA30` draws three of it for each held record's delay), seeded once.
static PACE_RNG: Mutex<Option<kh1_core::Kh1Rand>> = Mutex::new(None);
static PACE_LINES: AtomicU32 = AtomicU32::new(40);

/// Decision 38, M7 f3: KH1's pace for the hits held during Stop. From the enemy-hit wrapper (every
/// enemy update at Combat = KH1, after Re:CoM counted the timer down, before the state exe): in
/// Re:CoM's stop state (Re:CoM keeps its 8-hit cap), **the frame after each landing** (phase 2 with a
/// held index not yet paced), the per-landing timer is set so that the next held hit lands **KH1's
/// 6–11 frames** after this one (`crate::magic::stop::held_gap` with KH1's random, then
/// `landing_timer`). Everything else is Re:CoM's: the landing, its damage, effect and reaction.
pub unsafe fn held_hit_pace(enemy: usize) {
    use enemy_stop::STOP_STATE;
    let in_stop = rd::<usize>(enemy + held::CUR_STATE) == enemy + STOP_STATE;
    if !in_stop {
        if PACED_ANY.load(Ordering::Relaxed) {
            if let Ok(mut g) = PACED.try_lock() {
                g.retain(|e| e.0 != enemy);
                PACED_ANY.store(!g.is_empty(), Ordering::Relaxed);
            }
        }
        return;
    }
    if rd::<i32>(enemy + held::PHASE) != held::WAITING {
        return;
    }
    let index: i16 = rd(enemy + held::INDEX);
    // After the last held hit there is no next one to pace: Re:CoM's own wait before its recovery.
    if index as i32 + 1 >= rd::<i16>(enemy + held::COUNT) as i32 {
        return;
    }
    let Ok(mut g) = PACED.try_lock() else { return };
    match g.iter_mut().find(|e| e.0 == enemy) {
        Some(e) if e.1 == index => return,
        Some(e) => e.1 = index,
        None => {
            if g.len() >= 32 {
                g.remove(0);
            }
            g.push((enemy, index));
        }
    }
    PACED_ANY.store(true, Ordering::Relaxed);
    drop(g);
    let r = match PACE_RNG.lock() {
        Ok(mut rng) => rng
            .get_or_insert_with(|| {
                let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.subsec_nanos() ^ d.as_secs() as u32);
                kh1_core::Kh1Rand::new(seed)
            })
            .next_f32(),
        Err(_) => 0.0,
    };
    let gap = mg::stop::held_gap(r);
    let dt: i32 = rd(enemy + held::DT);
    let dt = if dt > 0 { dt } else { 50 };
    let old: i32 = rd(enemy + held::TIMER);
    let timer = mg::stop::landing_timer(gap, dt);
    wr(enemy + held::TIMER, timer);
    let count: i16 = rd(enemy + held::COUNT);
    if PACE_LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!(
            "magic: stop held hits on {enemy:#x}: hit {} of {count} landed; the next in KH1's {gap:.0} frames (Re:CoM's timer {old} -> {timer} ticks)",
            index as i32 + 1
        ));
    }
    event(format!("held {}/{count} next {gap:.0}f", index as i32 + 1));
}

/// Re:CoM's Gravity case's descriptor `+0xB8` (`lea rax,[0x1407C78D0]`). Checked (disasm
/// `0x140222B09`).
const GRAVITY_DESC_B8: u64 = 0x1_407C_78D0;
/// Re:CoM's Gravity sphere flags (`mov qword [rbp−0x50],0x41E1`). Checked (disasm `0x140222AB7`).
const GRAVITY_FLAGS: u64 = 0x41E1;

/// Sora's position (body `+0x30`) and facing direction (Re:CoM's yaw: (sin, 0, cos)).
unsafe fn sora_pos_dir(pl: usize) -> ([f32; 3], [f32; 3]) {
    let b: usize = rd(pl + p::BODY);
    let v = if b != 0 { rd_vec(b + 0x30) } else { kh1_core::Vec3::ZERO };
    let ang = func::<AnglesFn>(ANGLES)(pl);
    let yaw = if ang.is_null() { 0.0 } else { std::ptr::read_unaligned(ang)[1] };
    ([v.x, v.y, v.z], [yaw.sin(), 0.0, yaw.cos()])
}

/// Gravity's command (`L0025a`): the target (Re:CoM's `+0x84C`) if it is an alive enemy, else the
/// offset 2 m along Sora's facing (taken now, KH1's `sp+0x10`); Re:CoM's marker (its `initMagic`
/// case 0x1E's effect id 2 of the set's `+0x30` entry) at KH1's sphere start.
unsafe fn gravity_command(pl: usize, r: &[u8; 16]) {
    let h: u32 = rd(pl + f::TARGET);
    let (pos, dir) = sora_pos_dir(pl);
    let ahead = mg::gravity::AHEAD / 100.0;
    let off = [dir[0] * ahead, 0.0, dir[2] * ahead];
    // A battle enemy, or in a field room a field Heartless (owner, 2026-11-04: KH1 Gravity in the
    // field strikes field Heartless, as 0.110.0's Thunder / Stop).
    let (h, point) = match target_enemy(h) {
        Some(e) => (h, aim_point(e)),
        None => (0, [pos[0] + off[0], pos[1], pos[2] + off[2]]),
    };
    if let Ok(mut g) = GRAVITY_CMD.lock() {
        *g = Some((h, off));
    }
    let up = mg::gravity::START_UP / 100.0;
    effect_at(pl, set_name(r, 0x18), 2, [point[0], point[1] + up, point[2], 1.0]);
}

/// Re:CoM's `0x1401A6B20(actor, name, id, &position, &angles, 0.0)` with Sora's angles (the
/// Gravity case's hand effect, `initMagic`'s marker).
unsafe fn effect_at(pl: usize, name: usize, id: u32, at: [f32; 4]) {
    type EffectFn = unsafe extern "win64" fn(usize, usize, u32, *const [f32; 4], u64, f32) -> i32;
    let ang = func::<AnglesFn>(ANGLES)(pl);
    if name == 0 || ang.is_null() {
        return;
    }
    let ang: [f32; 4] = std::ptr::read_unaligned(ang);
    func::<EffectFn>(EFFECT_AT)(pl, name, id, &at, &ang as *const [f32; 4] as u64, 0.0);
}

/// KH1's Gravity release (7.8, `xm_gravity01.mag`): the marker point now (the target's aim point
/// if it is still alive, else Sora + the command's offset), **Re:CoM's Gravity case**
/// (`0x140222170` case `0x26`, Checked (disasm `0x1402229E0..0x140222B90`)) with KH1's numbers: its
/// hand effect (id 1), its sphere (`CHomingBullet`, flags `0x41E1`, `+0xB8` = `0x1407C78D0`, the
/// row's set / `+5` / `+0xD`) **2.4 m above the point, pitched straight down, falling 2.3 m over
/// N frames** (speed 2.3 / N m a frame, no target: no homing), its attack held off (as 7.6's
/// bolts: KH1 strikes at the impact, not on contact), life N + 2 frames; the falling thread
/// ([`gravity_frame`]); then the MP (one point). The sphere's id.
unsafe fn release_gravity(pl: usize, c: &CastStart) -> Option<u32> {
    let r = row(c.spell, c.level)?;
    let (h, off) = GRAVITY_CMD.lock().ok().and_then(|mut g| g.take()).unwrap_or((0, [0.0; 3]));
    let point = match target_enemy(h) {
        Some(e) => aim_point(e),
        None => {
            let (pos, _) = sora_pos_dir(pl);
            [pos[0] + off[0], pos[1], pos[2] + off[2]]
        }
    };
    let up = mg::gravity::START_UP / 100.0;
    let fall = mg::gravity::FALL / 100.0;
    let start = [point[0], point[1] + up, point[2], 1.0];
    let centre = [point[0], point[1] + up - fall, point[2]];
    let (_, n) = mg::gravity::of(c.level);
    // The hand effect (id 1 of the set's `+0x30` entry at the hand point).
    let body: usize = rd(pl + p::BODY);
    let mut hand = [0f32; 4];
    if body != 0 {
        let b = rd_vec(body + 0x30);
        hand = [b.x, b.y, b.z, 1.0];
    }
    func::<PlOutFn>(HAND_POINT)(pl, &mut hand);
    effect_at(pl, set_name(r, 0x18), 1, hand);
    let ang = func::<AnglesFn>(ANGLES)(pl);
    let ang: [f32; 4] = if ang.is_null() { [0.0; 4] } else { std::ptr::read_unaligned(ang) };
    let mut d = Desc([0; 0x100]);
    let dp = d.0.as_mut_ptr();
    func::<DescInitFn>(DESC_INIT)(dp, 2, 3);
    let w = |o: usize| dp.add(o);
    let (kind, attr, ap): (u32, u32, i32) = (rd(pl + f::KIND), rd(pl + f::ATTR), rd(pl + f::AP));
    std::ptr::write_unaligned(w(0x68) as *mut u32, kind);
    std::ptr::write_unaligned(w(0x6C) as *mut u32, attr);
    std::ptr::write_unaligned(w(0x70) as *mut i32, ap);
    std::ptr::write_unaligned(w(0x0C) as *mut u32, 0x8000_0000);
    // Re:CoM's own growth (flag 0x80, owner 2026-11-04: "make it the size of re:com gravity"):
    // Re:CoM grows the sphere `+0x1C` an update and draws it at its radius ÷ `+0x78` (its 1.5 m);
    // the step ends at KH1's hit radius on KH1's frames (`magic::gravity::growth_step_m`).
    std::ptr::write_unaligned(w(0x60) as *mut u64, mg::gravity::sphere_flags(GRAVITY_FLAGS) | 4 | COLLISION_HELD);
    std::ptr::write_unaligned(w(0x8C) as *mut i32, HELD_TICKS);
    std::ptr::write_unaligned(w(0x30) as *mut f32, fall / n);
    std::ptr::write_unaligned(w(0x34) as *mut f32, r[0xC] as f32);
    std::ptr::write_unaligned(w(0x3C) as *mut i32, ((n + 2.0) * 50.0) as i32);
    std::ptr::write_unaligned(w(0x10) as *mut [f32; 4], start);
    std::ptr::write_unaligned(w(0x1C) as *mut f32, mg::gravity::growth_step_m(c.level));
    std::ptr::write_unaligned(w(0x20) as *mut [f32; 4], [std::f32::consts::FRAC_PI_2, ang[1], ang[2], ang[3]]);
    std::ptr::write_unaligned(w(0xA8) as *mut usize, va(base(), EFFECT_SETS) + r[2] as usize * 0x60);
    std::ptr::write_unaligned(w(0xB8) as *mut usize, va(base(), GRAVITY_DESC_B8));
    // Re:CoM's row `+4` × 0.01 (1.5 m), as its Gravity case: the divisor of its drawn scale.
    std::ptr::write_unaligned(w(0x78) as *mut f32, r[4] as f32 * 0.01);
    let factory = func::<FactoryFn>(FACTORY)();
    if factory == 0 {
        return None;
    }
    let bullet = func::<ManagerFn>(SHOT_MANAGER)(factory, dp, pl);
    if bullet == 0 {
        note(|| format!("release gravity{}: Re:CoM's shot manager made no sphere (no MP paid)", c.level));
        return None;
    }
    func::<MoverFn>(SHOT_MOVER)(bullet, r[0xD] as f32 * 0.01, 0.0, 0, -1);
    hold_off(bullet);
    let id: u32 = rd(bullet + 0x20);
    remember_shot(id, c.spell, c.level, 0);
    let run = GravityRun { fall: mg::Fall::new(c.level), level: c.level, shot: id, centre, kind, attr, ap };
    if let Ok(mut g) = FALLS.lock() {
        if g.len() >= 8 {
            g.remove(0);
        }
        g.push(run);
        STRIKING.store(true, Ordering::Release);
    }
    // KH1 pays after the falling thread starts (sys 20, then sys 47).
    pay_spell(mg::gravity::COST, "Gravity release");
    note(|| {
        format!(
            "release gravity{}: sphere {id} from ({:.2},{:.2},{:.2}) falls {fall} m over {n} f to ({:.2},{:.2},{:.2}), impact radius {:.2} m, Re:CoM's growth {:.4} m an update, drawn at scale {:.3} at the impact (Re:CoM's rule: radius / 1.5 m), target {h:#x}",
            c.level,
            start[0],
            start[1],
            start[2],
            centre[0],
            centre[1],
            centre[2],
            mg::gravity::impact_radius_m(c.level),
            mg::gravity::growth_step_m(c.level),
            mg::gravity::effect_scale_at_impact(c.level)
        )
    });
    Some(id)
}

/// A shot's attack switched off for good (Re:CoM's own "off", vtable +8, as 7.6's bolts: the bullet
/// init tests the held flag before it fills the count).
pub(super) unsafe fn hold_off(bullet: usize) {
    let c: usize = rd(bullet + 0x2A8);
    if c == 0 {
        return;
    }
    let vt: usize = rd(c);
    if rd::<usize>(vt + 8) == base() + COLL_OFF as usize {
        func::<PlFn>(COLL_OFF)(c);
    }
}

/// A shot's attack struck on an enemy (7.8's impact, 7.9's Stop): [`strike_body`] on the shot's
/// `CCollAttack` (`bullet+0x2A8`).
unsafe fn apply_to(bullet: usize, enemy: usize) -> Result<u32, &'static str> {
    strike_body(rd(bullet + 0x2A8), enemy)
}

/// Re:CoM's collision context: its pass gives it to every overlap test (`mov rcx,[0x14087A870]`
/// before each test call, `0x14016EBB3` / `0x14016EBEC`; `sites::MAGIC_CHECKED`).
const COLL_CONTEXT: u64 = 0x1_4087_A870;
/// Re:CoM's overlap tests its pass runs for a sphere attack (`crate::magic::strike_contact`):
/// sphere body `0x1401777E0(ctx, body shape, attack shape)`, capsule body `0x140176FA0(ctx, body
/// shape, attack shape, 0)`; the KH1 blade test calls the same two (`win::attack`). Byte-checked
/// (`sites::ATTACK_CALLEES`).
const TEST_SPHERE: u32 = 0x0017_77E0;
const TEST_CAPSULE: u32 = 0x0017_6FA0;

/// Collision shape fields (`0x1401720E0`, the tests above). Checked (decomp).
mod shape {
    /// The sweep: from (`+0x10`) to (`+0x20`); the contact the tests write (`+0x50`).
    pub const FROM: usize = 0x10;
    pub const TO: usize = 0x20;
    pub const CONTACT: usize = 0x50;
    pub const KIND: usize = 0x60;
    pub const FLAGS: usize = 0x64;
    pub const RADIUS: usize = 0x70;
    pub const Y_OFFSET: usize = 0x74;
}

/// A hurt body's shape as the tests read it (each y + the shape's `+0x74`); kinds 2, 4, 5.
unsafe fn body_shape(s: usize) -> Option<mg::BodyShape> {
    let pt = |o: usize| {
        let v: [f32; 4] = rd(s + o);
        [v[0], v[1] + rd::<f32>(s + shape::Y_OFFSET), v[2]]
    };
    let r: f32 = rd(s + shape::RADIUS);
    match rd::<u32>(s + shape::KIND) {
        2 => Some(mg::BodyShape::Sphere { c: pt(shape::TO), r }),
        4 | 5 => Some(mg::BodyShape::Column { a: pt(shape::FROM), b: pt(shape::TO), r }),
        _ => None,
    }
}

/// One strike of ours on an enemy, as Re:CoM's collision pass (`0x14016EAD0`) strikes a body:
/// the enemy's first live hurt body the pass would test against this attack, **Re:CoM's own
/// overlap test** for the pair (the attack's sweep set straight down onto the body,
/// `crate::magic::strike_contact::sweep`), then, on its contact, the attack's apply (`CCollAttack`
/// vtable `+0x38` = `0x14016F1C0`). The test is what gives the body's shape its contact point
/// (`+0x50`), the point Re:CoM's defeat (`0x14017BBC0`) makes the death effect and the enemy's
/// prizes at: without it a strike that killed left the prizes at whatever the shape held (never
/// written: allocation garbage), which can be non-finite (the battle-end soft-lock,
/// `docs/recom-m7-magic.md` § Strikes and Re:CoM's contact). The attack stays off for Re:CoM's
/// own pass (Re:CoM's "off", vtable `+8`). Only through Re:CoM's own slots and functions.
unsafe fn strike_body(coll: usize, enemy: usize) -> Result<u32, &'static str> {
    type ApplyFn = unsafe extern "win64" fn(usize, usize) -> u32;
    type TestFn = unsafe extern "win64" fn(usize, usize, usize, i32) -> u8;
    use mg::strike_contact as sc;
    if coll == 0 {
        return Err("no attack");
    }
    let vt: usize = rd(coll);
    if rd::<usize>(vt + 8) != base() + COLL_OFF as usize || rd::<usize>(vt + 0x38) != base() + COLL_APPLY as usize {
        return Err("the shot's attack is not a CCollAttack");
    }
    let attack: usize = rd(coll + 0x18);
    let ctx: usize = rd(va(base(), COLL_CONTEXT));
    if attack == 0 || ctx == 0 {
        return Err("no attack shape or collision context");
    }
    let kind: u32 = rd(attack + shape::KIND);
    let info: usize = rd(enemy + 0xF8);
    if info == 0 {
        return Err("no hurt bodies");
    }
    let parts: usize = rd(info + 0x40);
    let n: i32 = rd(info + 0x38);
    let found = (0..n.clamp(0, 32) as usize).find_map(|i| {
        let part = parts + i * 0x120;
        let s: usize = rd(part + 0x18);
        if rd::<i32>(part + 0x10) == 0 || s == 0 {
            return None;
        }
        let test = sc::test_for(rd(s + shape::KIND), kind, rd(s + shape::FLAGS))?;
        Some((part, s, test, body_shape(s)?))
    });
    let Some((part, s, test, body)) = found else {
        return Err("no hurt body Re:CoM's pass hits with this attack");
    };
    let (from, to) = sc::sweep(test, &body, rd(attack + shape::RADIUS), rd(attack + shape::Y_OFFSET));
    wr(attack + shape::FROM, from);
    wr(attack + shape::TO, to);
    let contact = match test {
        sc::Test::Sphere => func::<TestFn>(TEST_SPHERE)(ctx, s, attack, 0),
        sc::Test::Capsule => func::<TestFn>(TEST_CAPSULE)(ctx, s, attack, 0),
    };
    func::<PlFn>(COLL_OFF)(coll);
    if contact == 0 {
        return Err("Re:CoM's overlap test found no contact (no hit, as in its pass)");
    }
    let p: [f32; 3] = rd(s + shape::CONTACT);
    if !sc::finite(p) {
        return Err("Re:CoM's overlap test left a non-finite contact; no hit");
    }
    Ok(func::<ApplyFn>(COLL_APPLY)(coll, part))
}

/// Our shot by its id (Re:CoM's entity handle, `+0x20`, resolved by `0x1402AD350` as the apply
/// resolves bodies' owners), if it is still one of Re:CoM's bullets.
unsafe fn shot_object(id: u32) -> Option<usize> {
    if id == 0 {
        return None;
    }
    let o = func::<mv::HandleFn>(mv::HANDLE_OBJ)(id);
    if o == 0 {
        return None;
    }
    let vt: usize = rd(o);
    let b = base();
    (vt == va(b, crate::guard::bullet::HOMING_VTABLE) || vt == va(b, crate::guard::bullet::VTABLE)).then_some(o)
}

/// An enemy KH1's falling Gravity sphere squashed (M7 f2, decision 38; `crate::magic::squash`):
/// its handle, Re:CoM's height scale before (the mover's scale y), KH1's `+0x480` / `+0x484`, and
/// whether the sphere touched it at the last run (KH1's `+0x88 & 4`, read by its next update).
struct Squashed {
    h: u32,
    base: f32,
    s: mg::squash::Squash,
    under: bool,
}

/// The enemies being squashed or springing back (at most 32). Empty: nothing of this runs.
static SQUASHED: Mutex<Vec<Squashed>> = Mutex::new(Vec::new());
static SQUASH_LINES: AtomicU32 = AtomicU32::new(20);
/// `0x1401A4A20(actor)`: its scale vector (the mover's `+0x70`); `0x1401A7050(actor, &scale)`: sets
/// it (an aligned 16-byte copy). Re:CoM's own Gravity squash (state `0x140183BA0`, stop exe
/// `0x1401843A0` case 5) writes the height this way. Checked (decomp + exe).
const SCALE_GET: u32 = 0x001A_4A20;
const SCALE_SET: u32 = 0x001A_7050;

#[repr(C, align(16))]
struct Scale([f32; 4]);

/// KH1's squash for this frame (`0x140297FE0` on each actor, then the falling threads' `L0055c`):
/// first every squashed enemy's update with the bit the last run left (its height = Re:CoM's base ×
/// KH1's `+0x480`, through Re:CoM's own scale setter; a stopped enemy's update doesn't run, as
/// KH1's), then each falling sphere's test: every alive enemy whose bodies touch the sphere of 170 cm
/// × its scale at its centre (KH1's sys 90 type 1) is under it for the next update.
unsafe fn squash_frame(falling: &[(u8, [f32; 3], f32)]) {
    type GetFn = unsafe extern "win64" fn(usize) -> *const [f32; 4];
    type SetFn = unsafe extern "win64" fn(usize, *const Scale);
    let Ok(mut g) = SQUASHED.lock() else { return };
    if g.is_empty() && falling.is_empty() {
        return;
    }
    let height = |obj: usize| -> Option<[f32; 4]> {
        let v = func::<GetFn>(SCALE_GET)(obj);
        (!v.is_null()).then(|| std::ptr::read_unaligned(v))
    };
    let set_height = |obj: usize, y: f32| {
        if let Some(mut v) = height(obj) {
            v[1] = y;
            let s = Scale(v);
            func::<SetFn>(SCALE_SET)(obj, &s);
        }
    };
    // The actors' updates.
    g.retain_mut(|e| {
        let obj = func::<mv::HandleFn>(mv::HANDLE_OBJ)(e.h);
        if obj == 0 {
            return false;
        }
        if alive_enemy(e.h).is_some() && stopped(obj) {
            return true;
        }
        // Re:CoM's Gravity flatten (Damage = Re:CoM: the impact's kind 0xD) owns the height now.
        if flattening(obj) {
            return false;
        }
        let y = e.s.step(e.under);
        e.under = false;
        set_height(obj, e.base * y);
        if e.s.done() {
            set_height(obj, e.base);
            return false;
        }
        true
    });
    // The falling threads' test.
    for &(level, impact, fallen) in falling {
        let centre = mg::squash::centre_at(impact, level, fallen);
        let area = mg::Area::Sphere { radius: mg::squash::radius_at(level, fallen) };
        for (h, _) in lock_on_list(0, false) {
            let Some(e) = alive_enemy(h) else { continue };
            if flattening(e) || !bodies(e).iter().any(|(_, b)| mg::body_touches(centre, area, b)) {
                continue;
            }
            if let Some(s) = g.iter_mut().find(|s| s.h == h) {
                s.under = true;
            } else if g.len() < 32 {
                let base = height(e).map_or(1.0, |v| v[1]);
                let base = if base > 0.01 && base < 100.0 { base } else { 1.0 };
                g.push(Squashed { h, base, s: mg::squash::Squash::default(), under: true });
                if SQUASH_LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
                    log(&format!(
                        "magic: gravity{level} squash on {h:#x}: under the falling sphere (centre ({:.2},{:.2},{:.2}), r {:.2} m), Re:CoM's height scale {base:.3} x KH1's +0x480 from the next update",
                        centre[0], centre[1], centre[2], mg::squash::radius_at(level, fallen)
                    ));
                }
            }
        }
    }
    if !g.is_empty() {
        let under = g.iter().filter(|s| s.under).count();
        event(format!("squash n{} u{under}", g.len()));
    }
}

/// Re:CoM's flatten state's begin / end (`sites::FLATTEN_HOOKS`; `crate::magic::flatten`).
pub static ORIG_FLATTEN_BEGIN: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_FLATTEN_END: AtomicUsize = AtomicUsize::new(0);
static FLATTEN_LINES: AtomicU32 = AtomicU32::new(20);
type StateFn = unsafe extern "win64" fn(usize);

fn flatten_note(line: impl FnOnce() -> String) {
    if FLATTEN_LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("magic: {}", line()));
    }
}

/// Re:CoM's flatten begin (hurt and attack bodies off, its squash), then in a KH1 battle the hurt
/// bodies back on with Re:CoM's own switch: a flattened enemy stays hittable, as in KH1.
pub unsafe extern "win64" fn flatten_begin(enemy: usize) {
    orig::<StateFn>(&ORIG_FLATTEN_BEGIN)(enemy);
    if !super::battle::kh1_battle() {
        return;
    }
    type BodiesFn = unsafe extern "win64" fn(usize, i32);
    func::<BodiesFn>(mg::flatten::HURT_ON_RVA)(enemy, -1);
    // KH1's own hit flatten (Damage = KH1): KH1's timer in Re:CoM's (its phase 0 from KH1's squash).
    if let Some(v) = kh1_flatten_begins(enemy) {
        wr(enemy + FLATTEN_TIMER, mg::flatten::kh1::begin_ticks(v));
        flatten_note(|| format!("flatten on {enemy:#x}: KH1's hit flatten ({v:?}: timer {} frames, squash {} frames, hold 60, then the get-up), hurt bodies kept on", v.start(), v.squash()));
        return;
    }
    flatten_note(|| format!("flatten on {enemy:#x}: Re:CoM's Gravity flatten, its hurt bodies kept on (KH1: a flattened enemy stays hittable)"));
}

/// Re:CoM's flatten state fields its exe reads (`0x140183BA0`): phase `+0x270`, timer `+0x160`
/// (ticks, counted down by the actor update `0x1401A59A0` before the state step). Checked (decomp).
const FLATTEN_PHASE: usize = 0x270;
const FLATTEN_TIMER: usize = 0x160;

/// An enemy KH1's hit flatten owns (Damage = KH1; `crate::magic::flatten::kh1`): its handle, the
/// variant, and whether the flatten was picked and its begin hasn't run yet.
struct Kh1Flatten {
    h: u32,
    v: mg::flatten::kh1::Variant,
    pending: bool,
}

/// At most 32; empty: the exe wrapper is one atomic load per flattened enemy update.
static KH1_FLATTENS: Mutex<Vec<Kh1Flatten>> = Mutex::new(Vec::new());
static KH1_FLATTEN_ANY: AtomicBool = AtomicBool::new(false);
/// The flatten exe's wrapper is in (`win::install`): only then does a KH1 Gravity hit flatten.
static KH1_FLATTEN_ON: AtomicBool = AtomicBool::new(false);

pub fn kh1_flatten_on() {
    KH1_FLATTEN_ON.store(true, Ordering::Release);
}

/// KH1's Gravity hit at Damage = KH1 (`win::attack::spell_hit`, right after Re:CoM's hit handler):
/// when Re:CoM gave the hit its flinch (its picker's kind-1 answer: the enemy alive, not stopped, no
/// super armour; its next state `+0x780`), the reaction becomes Re:CoM's own Gravity flatten:
/// Re:CoM's picker again with its kind 0xD (`0x14017D260`, the handler's last call, case 0xD →
/// `+0xAC0`), and the enemy is marked for KH1's numbers. What it did, for the hit line.
pub unsafe fn kh1_flatten_hit(enemy: usize) -> Option<mg::flatten::kh1::Variant> {
    use crate::guard::{enemy as ge, state_offset};
    use mg::flatten::kh1;
    if !KH1_FLATTEN_ON.load(Ordering::Acquire) || !super::battle::kh1_battle() || rd::<i32>(enemy + ge::HP) <= 0 {
        return None;
    }
    if rd::<usize>(enemy + ge::NEXT_STATE) != enemy + state_offset(ge::FLINCH_STATE) {
        return None;
    }
    type PickerFn = unsafe extern "win64" fn(usize, i32);
    func::<PickerFn>(crate::guard::func::PICKER)(enemy, kh1::PICK_KIND);
    if rd::<usize>(enemy + ge::NEXT_STATE) != enemy + kh1::STATE {
        return None;
    }
    let mover: usize = rd(enemy + rt::MOVER);
    let on_floor = mover != 0 && rd::<u32>(mover + MOVER_CONTACT) & FLOOR_CONTACT != 0;
    let flier = rd::<u64>(enemy + ge::FLAGS) & FLIER != 0;
    let v = kh1::Variant::of(on_floor, flier);
    let h: u32 = rd(enemy + 0x20);
    let Ok(mut g) = KH1_FLATTENS.lock() else { return None };
    if let Some(e) = g.iter_mut().find(|e| e.h == h) {
        e.v = v;
        e.pending = true;
    } else {
        if g.len() >= 32 {
            g.remove(0);
        }
        g.push(Kh1Flatten { h, v, pending: true });
    }
    KH1_FLATTEN_ANY.store(true, Ordering::Release);
    Some(v)
}

/// A mover's contact flags (`+0x120`: bit 0 a floor normal, bit 2 the floor test's; `0x1401D58F0`,
/// `0x1401D65B0`) and an enemy's flier flag (`+0x260` bit 8: no gravity). Checked (decomp).
const MOVER_CONTACT: usize = 0x120;
const FLOOR_CONTACT: u32 = 5;
const FLIER: u64 = 0x100;

/// At the flatten's begin: the variant when this flatten is KH1's (picked by `kh1_flatten_hit`);
/// an entry left from an older flatten (Re:CoM's own now) is dropped.
unsafe fn kh1_flatten_begins(enemy: usize) -> Option<mg::flatten::kh1::Variant> {
    if !KH1_FLATTEN_ANY.load(Ordering::Acquire) {
        return None;
    }
    let h: u32 = rd(enemy + 0x20);
    let mut g = KH1_FLATTENS.lock().ok()?;
    let k = g.iter().position(|e| e.h == h)?;
    if g[k].pending {
        g[k].pending = false;
        return Some(g[k].v);
    }
    g.swap_remove(k);
    KH1_FLATTEN_ANY.store(!g.is_empty(), Ordering::Release);
    None
}

/// The KH1 flatten running on this enemy, if any (not one still waiting for its begin).
unsafe fn kh1_flatten_of(enemy: usize) -> Option<mg::flatten::kh1::Variant> {
    if !KH1_FLATTEN_ANY.load(Ordering::Acquire) {
        return None;
    }
    let h: u32 = rd(enemy + 0x20);
    let g = KH1_FLATTENS.lock().ok()?;
    g.iter().find(|e| e.h == h && !e.pending).map(|e| e.v)
}

/// At the flatten's end: a KH1 flatten over (one picked again stays, for its next begin).
unsafe fn kh1_flatten_ends(enemy: usize) {
    if !KH1_FLATTEN_ANY.load(Ordering::Acquire) {
        return;
    }
    let h: u32 = rd(enemy + 0x20);
    if let Ok(mut g) = KH1_FLATTENS.lock() {
        g.retain(|e| e.h != h || e.pending);
        KH1_FLATTEN_ANY.store(!g.is_empty(), Ordering::Release);
    }
}

/// Re:CoM's flatten state's exe (`sites::FLATTEN_EXE_HOOK`), wrapped: for KH1's hit flatten, KH1's
/// numbers in Re:CoM's phases (`crate::magic::flatten::kh1::step`): phase 0 KH1's squash (Re:CoM's
/// own squash not run), then 0.1 and Re:CoM's phase 1 (its timer, KH1's, runs out: KH1's 60-frame
/// hold); Re:CoM's phase 1 end (its 1.0, its get-up motion) with the height kept at 0.1; Re:CoM's
/// phase 2 with KH1's get-up ease. Every other flatten: Re:CoM's.
pub static ORIG_FLATTEN_EXE: AtomicUsize = AtomicUsize::new(0);
pub unsafe extern "win64" fn flatten_exe(enemy: usize) {
    use mg::flatten::kh1::{self, Step};
    let Some(v) = kh1_flatten_of(enemy) else {
        return orig::<StateFn>(&ORIG_FLATTEN_EXE)(enemy);
    };
    let phase: i32 = rd(enemy + FLATTEN_PHASE);
    match kh1::step(v, phase, rd(enemy + FLATTEN_TIMER)) {
        Step::Squash(y) => set_height(enemy, y),
        Step::Hold => {
            set_height(enemy, kh1::LOW);
            wr(enemy + FLATTEN_PHASE, 1i32);
        }
        Step::Wait => {
            orig::<StateFn>(&ORIG_FLATTEN_EXE)(enemy);
            if rd::<i32>(enemy + FLATTEN_PHASE) != 1 {
                set_height(enemy, kh1::LOW);
                flatten_note(|| format!("flatten on {enemy:#x}: KH1's hold over: Re:CoM's get-up motion, KH1's get-up ease (+30 % a frame)"));
            }
        }
        Step::GetUp => {
            let y = height(enemy).unwrap_or(kh1::LOW);
            orig::<StateFn>(&ORIG_FLATTEN_EXE)(enemy);
            set_height(enemy, kh1::getup(y));
        }
    }
}

/// The enemy's height scale (Re:CoM's scale y, `0x1401A4A20`).
unsafe fn height(obj: usize) -> Option<f32> {
    type GetFn = unsafe extern "win64" fn(usize) -> *const [f32; 4];
    let v = func::<GetFn>(SCALE_GET)(obj);
    (!v.is_null()).then(|| std::ptr::read_unaligned(v)[1])
}

/// Its height scale set through Re:CoM's own setter (`0x1401A7050`), as Re:CoM's flatten writes it.
unsafe fn set_height(obj: usize, y: f32) {
    type GetFn = unsafe extern "win64" fn(usize) -> *const [f32; 4];
    type SetFn = unsafe extern "win64" fn(usize, *const Scale);
    let v = func::<GetFn>(SCALE_GET)(obj);
    if v.is_null() {
        return;
    }
    let mut v = std::ptr::read_unaligned(v);
    v[1] = y;
    let s = Scale(v);
    func::<SetFn>(SCALE_SET)(obj, &s);
}

/// Re:CoM's flatten end; in a KH1 battle, a flatten cut short (a hit's reaction took over before
/// Re:CoM stood it up) springs back KH1's way from where it is.
pub unsafe extern "win64" fn flatten_end(enemy: usize) {
    orig::<StateFn>(&ORIG_FLATTEN_END)(enemy);
    kh1_flatten_ends(enemy);
    if !super::battle::kh1_battle() {
        return;
    }
    type GetFn = unsafe extern "win64" fn(usize) -> *const [f32; 4];
    let v = func::<GetFn>(SCALE_GET)(enemy);
    if v.is_null() {
        return;
    }
    let y = std::ptr::read_unaligned(v)[1];
    let Some(s) = mg::flatten::spring_back(y) else { return };
    let h: u32 = rd(enemy + 0x20);
    let Ok(mut g) = SQUASHED.lock() else { return };
    if let Some(e) = g.iter_mut().find(|e| e.h == h) {
        *e = Squashed { h, base: mg::flatten::RECOM_HEIGHT, s, under: false };
    } else if g.len() < 32 {
        g.push(Squashed { h, base: mg::flatten::RECOM_HEIGHT, s, under: false });
    } else {
        return;
    }
    STRIKING.store(true, Ordering::Release);
    flatten_note(|| format!("flatten on {enemy:#x} cut short at height {y:.2}: KH1's spring-back (+0.05 a frame)"));
}

/// Enemies being squashed (the per-frame spell check runs while any are).
fn squashing() -> bool {
    SQUASHED.lock().map(|g| !g.is_empty()).unwrap_or(false)
}

/// One run of Gravity's falling threads (Sora's frame start): KH1's clock (Re:CoM grows the sphere
/// itself, flag 0x80), and at the impact KH1's strikes (`L0044c`): every alive enemy whose bodies touch the 1.7 × S m sphere at the
/// centre, struck once with the sphere's attack through Re:CoM's contact and apply
/// ([`strike_body`]). If the sphere is gone (Re:CoM ended it on a wall), a held bolt like 7.6's
/// carries the strikes. Each fall frame before
/// the impact also squashes the enemies under the sphere (`L0055c`, M7 f2: [`squash_frame`]).
unsafe fn gravity_frame(pl: usize, step: f32) {
    let mut due = Vec::new();
    let mut falling = Vec::new();
    {
        let Ok(mut g) = FALLS.lock() else { return };
        for run in g.iter_mut() {
            // The sphere's growth is Re:CoM's own (flag 0x80, `release_gravity`).
            let impact = run.fall.run(step);
            if impact {
                due.push((run.shot, run.level, run.centre, run.kind, run.attr, run.ap));
            } else {
                falling.push((run.level, run.centre, run.fall.fallen()));
            }
        }
        g.retain(|r| !(r.fall.left <= 0.0 && due.iter().any(|d| d.0 == r.shot)));
    }
    squash_frame(&falling);
    for (shot, level, centre, kind, attr, ap) in due {
        let area = mg::Area::Sphere { radius: mg::gravity::impact_radius_m(level) };
        let carrier = shot_object(shot).or_else(|| carrier_at(pl, level, centre, kind, attr, ap));
        let mut n = 0;
        // Battle enemies, and in a field room the field Heartless (owner, 2026-11-04): the sphere's
        // kind is the KH1 row's (1; no Re:CoM kind outside battle rooms), which Re:CoM's field
        // Heartless handler `0x14019E020` takes as its plain field strike (state `+0x380`: the
        // battle starts with first strike); `magic::field_kind` keeps it.
        for (h, _) in lock_on_targets(0) {
            let Some(e) = target_enemy(h) else { continue };
            if !bodies(e).iter().any(|(_, b)| mg::body_touches(centre, area, b)) {
                continue;
            }
            let res = carrier.ok_or("no carrier").and_then(|c| apply_to(c, e));
            n += 1;
            bolt_line(|| format!("gravity{level} impact on {h:#x}: Re:CoM's contact and apply -> {res:?}"));
        }
        event(format!("impact gravity{level} shot {shot} n{n}"));
    }
}

/// A held strike carrier at the impact (when Re:CoM already ended the sphere): a `CBullet` of the
/// row's set like 7.6's bolts, its attack off.
unsafe fn carrier_at(pl: usize, level: u8, centre: [f32; 3], kind: u32, attr: u32, ap: i32) -> Option<usize> {
    let r = row(mg::spell::GRAVITY, level)?;
    let mut d = Desc([0; 0x100]);
    let dp = d.0.as_mut_ptr();
    func::<DescInitFn>(DESC_INIT)(dp, 1, 3);
    let w = |o: usize| dp.add(o);
    std::ptr::write_unaligned(w(0x68) as *mut u32, kind);
    std::ptr::write_unaligned(w(0x6C) as *mut u32, attr);
    std::ptr::write_unaligned(w(0x70) as *mut i32, ap);
    std::ptr::write_unaligned(w(0x0C) as *mut u32, 0x8000_0000);
    std::ptr::write_unaligned(w(0x60) as *mut u64, THUNDER_BOLT_FLAGS | COLLISION_HELD);
    std::ptr::write_unaligned(w(0x8C) as *mut i32, HELD_TICKS);
    std::ptr::write_unaligned(w(0x3C) as *mut i32, 100);
    std::ptr::write_unaligned(w(0x10) as *mut [f32; 4], [centre[0], centre[1], centre[2], 1.0]);
    std::ptr::write_unaligned(w(0xA8) as *mut usize, va(base(), EFFECT_SETS) + r[2] as usize * 0x60);
    std::ptr::write_unaligned(w(0x78) as *mut f32, mg::gravity::impact_radius_m(level));
    let factory = func::<FactoryFn>(FACTORY)();
    if factory == 0 {
        return None;
    }
    let bullet = func::<ManagerFn>(SHOT_MANAGER)(factory, dp, pl);
    if bullet == 0 {
        return None;
    }
    func::<MoverFn>(SHOT_MOVER)(bullet, 0.1, 0.0, 0, -1);
    hold_off(bullet);
    remember_shot(rd(bullet + 0x20), mg::spell::GRAVITY, level, 0);
    Some(bullet)
}

/// `0x1402ABB20(object, bone)`: a bone of an actor (or the weapon) for effects. Checked by M5.
const BONE: u32 = 0x002A_BB20;
/// `0x1401A2D50(actor, effect name, id, bone, &offset, 0, 0.0)`: Re:CoM's effect on a bone (a
/// thunk to `0x1402577D0` with the actor's `+0x150`); `0x140257EF0(handle)` its object (`+0x4C |=
/// 4` as the Cure / Stop cases do). Checked (disasm `0x140222946..0x1402229B3`).
const EFFECT_ON_BONE: u32 = 0x001A_2D50;
const EFFECT_OBJECT: u32 = 0x0025_7EF0;
/// `0x1401E59A0(player, amount, 0, 1, 0)`: Re:CoM's heal (HP += amount, clamped to the max; the
/// HUD and the heal voice). Checked (decomp).
const HEAL: u32 = 0x001E_59A0;
/// `initMagic`'s charge offset (`0x1407C7E50`: (−0.8, 0, 0, 1)) and the Cure / Stop cases' body
/// offset (`0x1405A0910`: (0, 0.85, 0, 1)). Checked (exe).
const CHARGE_OFFSET: [f32; 4] = [-0.8, 0.0, 0.0, 1.0];
pub(super) const BODY_OFFSET: [f32; 4] = [0.0, 0.85, 0.0, 1.0];
/// The player's weapon (`+0x3D0`, `initMagic`'s charge goes on its bone 2). Checked (disasm).
const WEAPON: usize = 0x3D0;

/// An effect of the row's set (name at the set's `+0x18 + extra`) on a bone, as Re:CoM's cases
/// start them. The effect's handle.
pub(super) unsafe fn effect_on_bone(pl: usize, bone_of: usize, bone: i32, set_name: usize, id: u32, offset: &[f32; 4], sticky: bool) -> u32 {
    type BoneFn = unsafe extern "win64" fn(usize, i32) -> usize;
    type EffFn = unsafe extern "win64" fn(usize, usize, u32, usize, *const [f32; 4], u64, f32) -> u32;
    type ObjFn = unsafe extern "win64" fn(u32) -> usize;
    if bone_of == 0 || set_name == 0 {
        return 0;
    }
    let b = func::<BoneFn>(BONE)(bone_of, bone);
    let h = func::<EffFn>(EFFECT_ON_BONE)(pl, set_name, id, b, offset, 0, 0.0);
    if sticky {
        let o = func::<ObjFn>(EFFECT_OBJECT)(h);
        if o != 0 {
            wr(o + 0x4C, rd::<u32>(o + 0x4C) | 4);
        }
    }
    h
}

/// The name of an effect set's entry (`0x1405A1730 + set × 0x60 + 0x18 + extra`; Gravity's cases
/// read `+0x30`).
unsafe fn set_name(r: &[u8; 16], extra: usize) -> usize {
    rd(va(base(), EFFECT_SETS) + r[2] as usize * 0x60 + 0x18 + extra)
}

/// `initMagic`'s charge for Cure / Aero (cases 0x1D / 0x20; Gravity's reads the set's `+0x30`):
/// effect id 0 of the row's set on the weapon's bone 2, at (−0.8, 0, 0). Checked (disasm
/// `0x1402206F0..0x140220752`).
unsafe fn charge_on_weapon(pl: usize, r: &[u8; 16], extra: usize) {
    let weapon: usize = rd(pl + WEAPON);
    effect_on_bone(pl, weapon, 2, set_name(r, extra), 0, &CHARGE_OFFSET, false);
}

/// KH1's Cure release (7.7, `xm_cure01.mag`): the heal thread's target test (Sora: alive, not in
/// Re:CoM's death path), then **Re:CoM's Cure case** (`0x140222170` case `0x25`, Checked (disasm
/// `0x140222946..0x1402229CA`)): its `cure_all` effect (id set − 14 on Sora's bone 0 at (0, 0.85,
/// 0), kept on Sora) and its heal `0x1401E59A0(player, AP, 0, 1, 0)` with AP = 50 / 150 / 300; then
/// KH1's MP (sys 47 after the strike). An invalid target heals nothing and pays nothing.
unsafe fn release_cure(pl: usize, c: &CastStart) -> Option<u32> {
    type HealFn = unsafe extern "win64" fn(usize, i32, i32, i32, i32);
    let r = row(c.spell, c.level)?;
    let hp: i32 = rd(pl + HP);
    let dead = rd::<i32>(pl + DEAD) != 0;
    if !mg::cure::target_ok(hp, dead) {
        note(|| format!("release cure{}: Sora can't be healed (HP {hp}, dying {dead}): no heal, no MP", c.level));
        return None;
    }
    if mv::is_main_riku(pl) {
        // 0.184.8 (fork owner, 2026-11-23: "o efeito de cura com aqueles sininhos ficaram muito comicos era bom achar outro
        // efeito melhor"): Riku's own heal sparkle, the one his item use plays (`always` 1, his `p002_034` at frame 37;
        // the workspace's `knowledge/ReCoM/recom-m13-recom-fx.md` § 8, Checked (files)), on his bone 0 at Re:CoM's Cure
        // offset, once per level (Cura twice, Curaga three times: brighter), instead of Re:CoM's `cure_all`.
        let name: usize = rd(base() + EFFECT_ALWAYS as usize);
        if name != 0 && std::slice::from_raw_parts(name as *const u8, 7) == b"always\0" {
            for _ in 0..c.level.clamp(1, 3) {
                effect_on_bone(pl, pl, 0, name, RIKU_HEAL_SPARKLE, &BODY_OFFSET, true);
            }
        }
    } else {
        effect_on_bone(pl, pl, 0, set_name(r, 0), (r[2] as u32).wrapping_sub(0xE), &BODY_OFFSET, true);
    }
    let ap: i32 = rd(pl + f::AP);
    func::<HealFn>(HEAL)(pl, ap, 0, 1, 0);
    let after: i32 = rd(pl + HP);
    pay_spell(mg::cure::COST, "Cure release");
    note(|| format!("release cure{}: Re:CoM's heal {ap} -> HP {hp} -> {after} (max {})", c.level, rd::<i32>(pl + HP + 4)));
    event(format!("heal {ap} hp {after}"));
    // No shot: the trace's release token shows `no-shot`, a heal is its own event.
    None
}

/// Riku's item heal sparkle: `always.dpx` id 1 (Checked (files): his item use `p002_034`'s EET).
const RIKU_HEAL_SPARKLE: u32 = 1;
/// Re:CoM's `"always"` effect name pointer (`0x1407C2C40`, as `win::attack` reads it).
const EFFECT_ALWAYS: u32 = 0x007C_2C40;

/// Sora's HP (`+0x42C`, max `+0x430`) and Re:CoM's death flag (`+0x7B0`). Checked (decomp
/// `0x1401E59A0`, `0x1401E6DE0`).
const HP: usize = 0x42C;
const DEAD: usize = 0x7B0;

fn remember_shot(id: u32, s: u8, level: u8, serial: u32) {
    if let Ok(mut g) = SHOTS.lock() {
        if g.len() >= 32 {
            g.remove(0);
        }
        g.push((id, s, level, serial));
    }
}

unsafe fn release_fire(pl: usize, c: &CastStart) -> Option<u32> {
    let r = row(c.spell, c.level)?;
    let n = mg::shot_numbers(r);
    let body: usize = rd(pl + p::BODY);
    let mut hand = [0f32; 4];
    if body != 0 {
        let b = rd_vec(body + 0x30);
        hand = [b.x, b.y, b.z, 1.0];
    }
    func::<PlOutFn>(HAND_POINT)(pl, &mut hand);
    let mut d = Desc([0; 0x100]);
    let dp = d.0.as_mut_ptr();
    // Class 2 = CHomingBullet, 3 = the descriptor kind Re:CoM's spells pass.
    func::<DescInitFn>(DESC_INIT)(dp, 2, 3);
    let at = |o: usize| dp.add(o);
    std::ptr::write_unaligned(at(0x68) as *mut u32, rd::<u32>(pl + f::KIND));
    std::ptr::write_unaligned(at(0x6C) as *mut u32, rd::<u32>(pl + f::ATTR));
    std::ptr::write_unaligned(at(0x70) as *mut i32, rd::<i32>(pl + f::AP));
    std::ptr::write_unaligned(at(0x0C) as *mut u32, 0x8000_0000);
    std::ptr::write_unaligned(at(0x60) as *mut u64, n.flags);
    std::ptr::write_unaligned(at(0x30) as *mut f32, n.speed);
    std::ptr::write_unaligned(at(0x3C) as *mut i32, n.life);
    std::ptr::write_unaligned(at(0x34) as *mut f32, n.range);
    std::ptr::write_unaligned(at(0x44) as *mut i32, rd::<i32>(pl + f::TARGET));
    std::ptr::write_unaligned(at(0x48) as *mut f32, n.gate);
    std::ptr::write_unaligned(at(0x4C) as *mut f32, n.turn);
    std::ptr::write_unaligned(at(0x10) as *mut [f32; 4], [hand[0], hand[1], hand[2], 0.5]);
    let ang = func::<AnglesFn>(ANGLES)(pl);
    if !ang.is_null() {
        std::ptr::write_unaligned(at(0x20) as *mut [f32; 4], std::ptr::read_unaligned(ang));
    }
    std::ptr::write_unaligned(at(0xA8) as *mut usize, va(base(), EFFECT_SETS) + r[2] as usize * 0x60);
    std::ptr::write_unaligned(at(0x78) as *mut f32, n.radius);
    std::ptr::write_unaligned(at(0x94) as *mut i32, n.fade);
    let factory = func::<FactoryFn>(FACTORY)();
    if factory == 0 {
        note(|| "release: no entity factory: no shot".into());
        return None;
    }
    let bullet = func::<ManagerFn>(SHOT_MANAGER)(factory, dp, pl);
    if bullet == 0 {
        note(|| "release: Re:CoM's shot manager made no shot (no MP paid)".into());
        return None;
    }
    func::<MoverFn>(SHOT_MOVER)(bullet, 0.1, 0.0, 0, -1);
    let id: u32 = rd(bullet + 0x20);
    // KH1's ball with no target picks one each frame (7.4b); with a target, Re:CoM's homing is
    // KH1's (it homes while the target exists, then flies straight, never re-picking).
    if rd::<i32>(pl + f::TARGET) <= 0 {
        seek_on(bullet, id);
    }
    remember_shot(id, c.spell, c.level, 0);
    // KH1 pays after the launch (`xm_fire01.mag`: sys 20, then sys 47).
    pay_spell(mg::fire::COST, "Fire release");
    note(|| {
        format!(
            "release {}{}: shot id {id} at ({:.2},{:.2},{:.2}), speed {:.2} m/f, life {} ticks, turn {}°/f, radius {:.2} m, target {}",
            mg::spell::NAMES[c.spell as usize],
            c.level,
            hand[0],
            hand[1],
            hand[2],
            n.speed,
            n.life,
            n.turn,
            n.radius,
            rd::<i32>(pl + f::TARGET)
        )
    });
    Some(id)
}

/// KH1's Blizzard release (7.5, `xm_blizzard01.mag` `L006dc` → the spawner thread): seven shards,
/// each Re:CoM's Blizzard shot (`0x140222170` case `0x23`'s calls: descriptor class 2, the hand
/// point, Re:CoM's `bliz_all` set for the straight shard and the next set for the cone ones, the
/// first shard's hit serial passed to the others) with KH1's numbers: the cone and start offsets
/// (`crate::magic::blizzard_shards`), speed, life, radius, a hit or wall ends it; then each shard
/// glides as KH1's ([`kh1_shard_exe`]); then the MP (sys 47 after the thread). The first shard's id.
unsafe fn release_blizzard(pl: usize, c: &CastStart) -> Option<u32> {
    let r = row(c.spell, c.level)?;
    let body: usize = rd(pl + p::BODY);
    let mut hand = [0f32; 4];
    if body != 0 {
        let b = rd_vec(body + 0x30);
        hand = [b.x, b.y, b.z, 1.0];
    }
    func::<PlOutFn>(HAND_POINT)(pl, &mut hand);
    let ang = func::<AnglesFn>(ANGLES)(pl);
    if ang.is_null() {
        return None;
    }
    let ang: [f32; 4] = std::ptr::read_unaligned(ang);
    let speed = r[9] as f32 * 0.01;
    let life = r[0xE] as i32 * 100;
    let mut serial = 0u32;
    let mut first = None;
    let mut made = 0;
    for (k, (off, dir)) in mg::blizzard_shards(ang[1]).iter().enumerate() {
        let Some((pitch, yaw)) = crate::guard::aim_angles(*dir) else { continue };
        let mut d = Desc([0; 0x100]);
        let dp = d.0.as_mut_ptr();
        func::<DescInitFn>(DESC_INIT)(dp, 2, 3);
        let at = |o: usize| dp.add(o);
        std::ptr::write_unaligned(at(0x68) as *mut u32, rd::<u32>(pl + f::KIND));
        std::ptr::write_unaligned(at(0x6C) as *mut u32, rd::<u32>(pl + f::ATTR));
        std::ptr::write_unaligned(at(0x70) as *mut i32, rd::<i32>(pl + f::AP));
        std::ptr::write_unaligned(at(0x0C) as *mut u32, 0x8000_0000);
        std::ptr::write_unaligned(at(0x60) as *mut u64, mg::KH1_BLIZZARD_FLAGS);
        std::ptr::write_unaligned(at(0x30) as *mut f32, speed);
        std::ptr::write_unaligned(at(0x34) as *mut f32, r[0xC] as f32);
        std::ptr::write_unaligned(at(0x38) as *mut i32, life);
        std::ptr::write_unaligned(at(0x3C) as *mut i32, life);
        std::ptr::write_unaligned(at(0x10) as *mut [f32; 4], [hand[0] + off[0], hand[1] + off[1], hand[2] + off[2], r[5] as f32 * 0.01]);
        std::ptr::write_unaligned(at(0x20) as *mut [f32; 4], [pitch, yaw, ang[2], ang[3]]);
        let set = if k < mg::blizzard::CONE_SHARDS { EFFECT_SETS_NEXT } else { EFFECT_SETS };
        std::ptr::write_unaligned(at(0xA8) as *mut usize, va(base(), set) + r[2] as usize * 0x60);
        std::ptr::write_unaligned(at(0x78) as *mut f32, r[4] as f32 * 0.01);
        std::ptr::write_unaligned(at(0x74) as *mut u32, serial);
        let factory = func::<FactoryFn>(FACTORY)();
        if factory == 0 {
            break;
        }
        let bullet = func::<ManagerFn>(SHOT_MANAGER)(factory, dp, pl);
        if bullet == 0 {
            continue;
        }
        // A small mover (Re:CoM's Fire set-up): the shard glides 50 cm above the floor, its attack
        // keeps KH1's radius.
        func::<MoverFn>(SHOT_MOVER)(bullet, 0.1, 0.0, 0, -1);
        if serial == 0 {
            serial = func::<SerialFn>(SHOT_SERIAL)(bullet);
        }
        let id: u32 = rd(bullet + 0x20);
        first.get_or_insert(id);
        made += 1;
        remember_shot(id, c.spell, c.level, serial);
        shard_on(bullet, id, dir[1] * speed);
    }
    let first = first?;
    // KH1 pays after the spawner thread starts (sys 20, then sys 47).
    pay_spell(mg::blizzard::COST, "Blizzard release");
    note(|| format!("release {}{}: {made} shards from ({:.2},{:.2},{:.2}) yaw {:.3}, speed {speed:.2} m/f, life {life} ticks, radius {:.2} m, serial {serial}", mg::spell::NAMES[c.spell as usize], c.level, hand[0], hand[1], hand[2], ang[1], r[4] as f32 * 0.01));
    Some(first)
}

/// An alive enemy for a handle: Re:CoM's enemy (type 0x1004) with HP left. KH1's sys 120 / its
/// actor walk count live actors (`+0x374` bit 0 set, bit 1 clear) not flagged defeated.
unsafe fn alive_enemy(h: u32) -> Option<usize> {
    type TypeFn = unsafe extern "win64" fn(usize) -> u32;
    if h == 0 || h as i32 <= 0 {
        return None;
    }
    let obj = func::<mv::HandleFn>(mv::HANDLE_OBJ)(h);
    if obj == 0 {
        return None;
    }
    let vt: usize = rd(obj);
    let ty: TypeFn = std::mem::transmute_copy(&rd::<usize>(vt + crate::battle::enemy::VT_TYPE));
    (ty(obj) == crate::battle::ENEMY_TYPE && rd::<i32>(obj + crate::battle::enemy::HP) > 0).then_some(obj)
}

/// A field Heartless (Re:CoM's `CMapEnemy`, type [`mg::MAP_ENEMY_TYPE`]) Re:CoM's own field lock-on
/// would take (`0x1401CD810`: type 0x1002 and `+0x154 & 0x10000`). Only field rooms have them.
unsafe fn is_map_enemy(obj: usize) -> bool {
    type TypeFn = unsafe extern "win64" fn(usize) -> u32;
    if obj == 0 {
        return false;
    }
    let vt: usize = rd(obj);
    let ty: TypeFn = std::mem::transmute_copy(&rd::<usize>(vt + crate::battle::enemy::VT_TYPE));
    ty(obj) == mg::MAP_ENEMY_TYPE && rd::<u32>(obj + rt::ACTOR_FLAGS) & rt::LOCKABLE != 0
}

/// A strike target for a handle (owner, 2026-11-04, "Sleights outside battle"): an alive battle
/// enemy ([`alive_enemy`]), or in a field room a field Heartless ([`is_map_enemy`]): KH1's Thunder,
/// Stop and specials there strike Re:CoM's field Heartless as its own field attacks do (its hit
/// handler `0x14019E020` decides by the kind). The two never meet: map enemies are field-only.
unsafe fn target_enemy(h: u32) -> Option<usize> {
    if let Some(e) = alive_enemy(h) {
        return Some(e);
    }
    if h == 0 || h as i32 <= 0 {
        return None;
    }
    let obj = func::<mv::HandleFn>(mv::HANDLE_OBJ)(h);
    is_map_enemy(obj).then_some(obj)
}

/// [`target_enemy`] as a test (the specials' targets).
pub(super) unsafe fn is_target_enemy(h: u32) -> bool {
    target_enemy(h).is_some()
}

/// An actor's aim point (vtable `+0x50`, what Re:CoM homes and its Thunder strikes at; KH1's target
/// part, Assumption 5) and its position (mover `+0x30`, KH1's actor `+0x10`).
pub(super) unsafe fn aim_point(obj: usize) -> [f32; 3] {
    type AimFn = unsafe extern "win64" fn(usize, *mut [f32; 4]);
    let vt: usize = rd(obj);
    let f: AimFn = std::mem::transmute_copy(&rd::<usize>(vt + 0x50));
    let mut p = [0f32; 4];
    f(obj, &mut p);
    [p[0], p[1], p[2]]
}

unsafe fn position(obj: usize) -> Option<[f32; 3]> {
    let mover: usize = rd(obj + rt::MOVER);
    (mover != 0).then(|| {
        let p = rd_vec(mover + rt::POSITION);
        [p.x, p.y, p.z]
    })
}

/// Thunder's command (`L004b6`): the target (Re:CoM's `+0x84C`, the command's) if it is an alive
/// enemy, and the area centre: its aim point, else Sora's position.
unsafe fn thunder_command(pl: usize) {
    let h: u32 = rd(pl + f::TARGET);
    let cmd = match target_enemy(h) {
        Some(e) => (h, aim_point(e)),
        None => {
            let b: usize = rd(pl + p::BODY);
            let v = if b != 0 { rd_vec(b + 0x30) } else { kh1_core::Vec3::ZERO };
            (0, [v.x, v.y, v.z])
        }
    };
    if let Ok(mut g) = THUNDER_CMD.lock() {
        *g = Some(cmd);
    }
}

/// The hurt bodies of an enemy as Re:CoM's primitives read their shapes (the live ones: body
/// `+0x10` on, shape kind 2 / 4 / 5).
unsafe fn bodies(enemy: usize) -> Vec<(usize, mg::BodyShape)> {
    let mut out = Vec::new();
    let info: usize = rd(enemy + 0xF8);
    if info == 0 {
        return out;
    }
    let parts: usize = rd(info + 0x40);
    let n: i32 = rd(info + 0x38);
    for i in 0..n.clamp(0, 32) as usize {
        let part = parts + i * 0x120;
        let s: usize = rd(part + 0x18);
        if rd::<i32>(part + 0x10) == 0 || s == 0 {
            continue;
        }
        if let Some(b) = body_shape(s) {
            out.push((part, b));
        }
    }
    out
}

/// KH1's Thunder release (7.6, `xm_thunder01.mag`): the centre moves to the target if it is still
/// alive; Re:CoM's lock-on enemies, sorted nearest first from the target / Sora
/// (`crate::magic::thunder_order`); the first strike now; the MP (one point); the rest every 8
/// frames from Sora's frame start ([`strikes_frame`]). The first bolt's id, if one struck.
unsafe fn release_thunder(pl: usize, c: &CastStart) -> Option<u32> {
    let (h, mut centre) = THUNDER_CMD.lock().ok().and_then(|mut g| g.take()).unwrap_or((0, [0.0; 3]));
    let target = target_enemy(h);
    if let Some(t) = target {
        centre = aim_point(t);
    }
    let from = match target {
        Some(t) => position(t),
        None => {
            let b: usize = rd(pl + p::BODY);
            (b != 0).then(|| {
                let v = rd_vec(b + 0x30);
                [v.x, v.y, v.z]
            })
        }
    }
    .unwrap_or(centre);
    let enemies: Vec<(u32, [f32; 3])> = lock_on_targets(0).into_iter().filter_map(|(h, _)| Some((h, position(target_enemy(h)?)?))).collect();
    let order = mg::thunder_order(from, enemies);
    let n = order.len();
    // Re:CoM's Thunder case's first effect: id 1 at the hand.
    if let Some(r) = row(c.spell, c.level) {
        let mut hand = rd::<[f32; 4]>(pl + PLAYER_POS);
        func::<PlOutFn>(HAND_POINT)(pl, &mut hand);
        thunder_effect(pl, r, 1, hand);
    }
    let area = mg::Area::Cylinder { radius: mg::thunder::RADIUS / 100.0, half: mg::thunder::HALF_HEIGHT / 100.0 };
    let run = ThunderRun { s: mg::Strikes::new(order), spell: mg::spell::THUNDER, area, level: c.level, centre, kind: rd(pl + f::KIND), attr: rd(pl + f::ATTR), ap: rd(pl + f::AP) };
    let first = match STRIKES.lock() {
        Ok(mut g) => {
            if g.len() >= 8 {
                g.remove(0);
            }
            g.push(run);
            STRIKING.store(true, Ordering::Release);
            let i = g.len() - 1;
            drop(g);
            strike_step(pl, 0.0, Some(i)).into_iter().next()
        }
        Err(_) => None,
    };
    // KH1 pays after the strike thread starts (sys 20, then sys 47).
    pay_spell(mg::thunder::COST, "Thunder release");
    note(|| format!("release thunder{}: area centre ({:.2},{:.2},{:.2}) r {} m ±{} m, {n} enemies listed from ({:.2},{:.2},{:.2}), first bolt {first:?}", c.level, centre[0], centre[1], centre[2], mg::thunder::RADIUS / 100.0, mg::thunder::HALF_HEIGHT / 100.0, from[0], from[1], from[2]));
    first
}

/// Re:CoM's effect call as its Thunder case makes it (Sora's angles as the rotation).
unsafe fn thunder_effect(pl: usize, r: &[u8; 16], id: u32, at: [f32; 4]) {
    type EffectFn = unsafe extern "win64" fn(usize, usize, u32, *const [f32; 4], u64, f32) -> i32;
    let name: usize = rd(va(base(), EFFECT_SETS) + r[2] as usize * 0x60 + 0x18);
    let ang = func::<AnglesFn>(ANGLES)(pl);
    if name == 0 || ang.is_null() {
        return;
    }
    let ang: [f32; 4] = std::ptr::read_unaligned(ang);
    func::<EffectFn>(EFFECT_AT)(pl, name, id, &at, &ang as *const [f32; 4] as u64, 0.0);
}

/// Spell threads still running (Thunder / Stop strikes, Gravity falls), for the battle-end log.
pub fn threads_running() -> (usize, usize) {
    (STRIKES.lock().map(|g| g.len()).unwrap_or(0), FALLS.lock().map(|g| g.len()).unwrap_or(0))
}

/// Sora's frame start (the request-bits wrapper, every frame at Combat = KH1): one run of a
/// Thunder strike thread, if one is going (one atomic load otherwise).
pub unsafe fn strikes_frame(pl: usize) {
    if !STRIKING.load(Ordering::Acquire) {
        return;
    }
    let step: f32 = rd(pl + p::STEP);
    strike_step(pl, step, None);
    gravity_frame(pl, step);
    aero_frame(pl, step);
    let idle = STRIKES.lock().map(|g| g.is_empty()).unwrap_or(true) && FALLS.lock().map(|g| g.is_empty()).unwrap_or(true) && !barrier_on() && !squashing();
    if idle {
        STRIKING.store(false, Ordering::Release);
    }
}

/// One run of the strike threads (`only`: the release's own run of the new thread, step 0: its
/// wait then counts that run). The bolts' ids.
unsafe fn strike_step(pl: usize, step: f32, only: Option<usize>) -> Vec<u32> {
    let mut due = Vec::new();
    {
        let Ok(mut g) = STRIKES.lock() else { return Vec::new() };
        for (i, run) in g.iter_mut().enumerate() {
            if only.is_some_and(|o| o != i) {
                continue;
            }
            let (centre, area, spell, level) = (run.centre, run.area, run.spell, run.level);
            let hit = run.s.run(if step > 0.0 { step } else { 1.0 }, |h| {
                let Some(e) = target_enemy(h) else {
                    bolt_line(|| format!("{}{level}: {h:#x} passed over (not alive)", mg::spell::NAMES.get(spell as usize).copied().unwrap_or("?")));
                    return false;
                };
                let inside = bodies(e).iter().any(|(_, b)| mg::body_touches(centre, area, b));
                if !inside {
                    bolt_line(|| format!("{}{level}: {h:#x} passed over (no hurt body in the area, as KH1)", mg::spell::NAMES.get(spell as usize).copied().unwrap_or("?")));
                }
                inside
            });
            if let Some(h) = hit {
                due.push((h, run.spell, run.level, run.kind, run.attr, run.ap, run.s.struck));
            }
        }
        g.retain(|r| !r.s.done());
    }
    let mut ids = Vec::new();
    for (h, sp, level, kind, attr, ap, n) in due {
        let Some(e) = target_enemy(h) else { continue };
        // Owner (2026-11-04, "Sleights outside battle"): a field Heartless takes Re:CoM's own Thunder /
        // Stop kind, its handler's paralyse / freeze (`crate::magic::field_kind`); battle enemies the
        // KH1 row's.
        let kind = if is_map_enemy(e) { mg::field_kind(sp, level, kind) } else { kind };
        let (id, what) = if sp == mg::spell::STOP { (stop_strike(pl, e, h, level, kind, attr, ap), "stop") } else { (strike(pl, e, h, level, kind, attr, ap), "bolt") };
        event(format!("{what} {n} e{h:x}{}", id.map_or(" -".into(), |i| format!(" shot {i}"))));
        ids.extend(id);
    }
    ids
}

/// One bolt (KH1's sys 160): Re:CoM's Thunder case for this enemy (its bolt effect, its `CBullet`
/// with the cast's kind / attribute / AP, the attack held off), then **Re:CoM's own hit path
/// directly**: as Re:CoM's collision pass (`0x14016EAD0`) strikes a body, Re:CoM's own overlap
/// test for the bolt and the enemy's first live hurt body (the bolt falling onto it), then the
/// attack's apply (`CCollAttack` vtable `+0x38` = `0x14016F1C0`) ([`strike_body`]). Re:CoM then
/// runs the enemy's hit (its handler, our KH1 damage on it, the reaction, the hit effect) and, if
/// it dies, its death effect and prizes at the test's contact. The bolt's id.
#[allow(clippy::too_many_arguments)]
unsafe fn strike(pl: usize, enemy: usize, h: u32, level: u8, kind: u32, attr: u32, ap: i32) -> Option<u32> {
    let r = row(mg::spell::THUNDER, level)?;
    let aim = aim_point(enemy);
    let sora: [f32; 4] = rd(pl + PLAYER_POS);
    let mut at = [aim[0], sora[1] - THUNDER_DROP, aim[2], sora[3]];
    thunder_effect(pl, r, (r[2] as u32).wrapping_sub(10), at);
    at[1] += THUNDER_DROP;
    let ang = func::<AnglesFn>(ANGLES)(pl);
    let ang: [f32; 4] = if ang.is_null() { [0.0; 4] } else { std::ptr::read_unaligned(ang) };
    let mut d = Desc([0; 0x100]);
    let dp = d.0.as_mut_ptr();
    func::<DescInitFn>(DESC_INIT)(dp, 1, 3);
    let w = |o: usize| dp.add(o);
    std::ptr::write_unaligned(w(0x68) as *mut u32, kind);
    std::ptr::write_unaligned(w(0x6C) as *mut u32, attr);
    std::ptr::write_unaligned(w(0x70) as *mut i32, ap);
    std::ptr::write_unaligned(w(0x0C) as *mut u32, 0x8000_0000);
    std::ptr::write_unaligned(w(0x60) as *mut u64, THUNDER_BOLT_FLAGS | COLLISION_HELD);
    std::ptr::write_unaligned(w(0x8C) as *mut i32, HELD_TICKS);
    std::ptr::write_unaligned(w(0x30) as *mut f32, r[9] as f32 * 0.01);
    std::ptr::write_unaligned(w(0x3C) as *mut i32, THUNDER_BOLT_LIFE);
    std::ptr::write_unaligned(w(0x34) as *mut f32, r[0xC] as f32);
    std::ptr::write_unaligned(w(0x20) as *mut [f32; 4], [std::f32::consts::FRAC_PI_2, ang[1], ang[2], ang[3]]);
    std::ptr::write_unaligned(w(0x10) as *mut [f32; 4], at);
    std::ptr::write_unaligned(w(0xA8) as *mut usize, va(base(), EFFECT_SETS) + r[2] as usize * 0x60);
    std::ptr::write_unaligned(w(0x78) as *mut f32, r[4] as f32 * 0.1);
    let factory = func::<FactoryFn>(FACTORY)();
    if factory == 0 {
        return None;
    }
    let bullet = func::<ManagerFn>(SHOT_MANAGER)(factory, dp, pl);
    if bullet == 0 {
        bolt_line(|| format!("bolt on {h:#x}: Re:CoM made no shot"));
        return None;
    }
    func::<MoverFn>(SHOT_MOVER)(bullet, 0.1, 0.0, 0, -1);
    let id: u32 = rd(bullet + 0x20);
    remember_shot(id, mg::spell::THUNDER, level, 0);
    // The bolt's own collision stays off (Re:CoM's own "off", the slot its init calls for a held
    // collision): its one hit is this strike, with Re:CoM's own contact ([`strike_body`]).
    let res = strike_body(rd(bullet + 0x2A8), enemy);
    bolt_line(|| format!("bolt on {h:#x}: shot {id}, Re:CoM's contact and apply -> {res:?} (1 hit, 2 / 3 guarded, 0 already hit)"));
    Some(id)
}

fn bolt_line(line: impl FnOnce() -> String) {
    if BOLT_LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("magic: {}", line()));
    }
}

/// A shard's flight state's exe becomes [`kh1_shard_exe`] (as [`seek_on`]: only on a plain
/// `CHomingBullet` whose exe is Re:CoM's thunk).
unsafe fn shard_on(bullet: usize, id: u32, dy: f32) {
    let b = base();
    let vt: usize = rd(bullet);
    let exe: usize = rd(bullet + rt::STATE_EXE);
    let adj: i32 = rd(bullet + rt::STATE_EXE_ADJUST);
    if vt != va(b, crate::guard::bullet::HOMING_VTABLE) || exe != b + HOMING_THUNK as usize || adj != 0 {
        rt_line(|| format!("blizzard: shard {id} is not a plain CHomingBullet in flight: Re:CoM's straight flight"));
        return;
    }
    if let Ok(mut g) = SHARDS.lock() {
        if g.len() >= 32 {
            g.remove(0);
        }
        g.push((id, dy));
    }
    wr(bullet + rt::STATE_EXE, kh1_shard_exe as *const () as usize);
}

/// The flight-state exe of our Blizzard shards (7.5): KH1's glide for this frame
/// (`crate::magic::blizzard_glide_y`): Re:CoM's segment test (`0x140170330` on the shard mover's
/// own collision object, as Re:CoM's movers test their moves) from the shard's height straight down
/// by `0.5 m − dy`, at where the shard is going this frame; the vertical velocity then lands it 0.5
/// m above that floor, or on its own slope. Then Re:CoM's own step (the thunk: no target, so the
/// homing step only runs its checks). Runs only for these shards, once a frame each.
pub unsafe extern "win64" fn kh1_shard_exe(bullet: usize) {
    use crate::guard::bullet as gb;
    let mover: usize = rd(bullet + gb::MOVER);
    let live = rd::<i32>(bullet + rt::HOLD) == 0 && rd::<i32>(bullet + rt::LIFE) > 0 && mover != 0;
    let id: u32 = rd(bullet + 0x20);
    let dy = SHARDS.lock().ok().and_then(|g| g.iter().find(|e| e.0 == id).map(|e| e.1));
    if let (true, Some(dy)) = (live, dy) {
        let pos = rd_vec(mover + rt::POSITION);
        let v: [f32; 4] = rd(mover + gb::VELOCITY);
        let floor = floor_below(mover, [pos.x + v[0], pos.y, pos.z + v[2]], mg::blizzard_ray(dy));
        let y = mg::blizzard_glide_y(pos.y, dy, floor);
        wr(mover + gb::VELOCITY + 4, y - pos.y);
    }
    func::<PlFn>(HOMING_THUNK)(bullet)
}

/// Re:CoM's segment test from `from` straight down by `len` m with this mover's collision object
/// (`mover+0x158`, its segment `+0x18`: ends `+0x10` / `+0x20`, hit `+0x30`; the parameter
/// `mover+0x134`), as `win::movement`'s floor probe for Sora. The hit's height, any surface (KH1's
/// sys 77 takes any).
unsafe fn floor_below(mover: usize, from: [f32; 3], len: f32) -> Option<f32> {
    if len <= 0.0 {
        return None;
    }
    let coll: usize = rd(mover + rt::MOVER_COLL);
    if coll == 0 {
        return None;
    }
    let seg: usize = rd(coll + 0x18);
    if seg == 0 {
        return None;
    }
    for (o, y) in [(0x10, from[1]), (0x20, from[1] - len)] {
        wr(seg + o, [from[0], y, from[2], 1.0f32]);
    }
    let hit = func::<mv::CollideFn>(mv::COLLIDE)(coll, rd::<i32>(mover + rt::MOVER_COLL_PARAM)) != 0;
    let seg: usize = rd(coll + 0x18);
    (hit && seg != 0).then(|| rd::<f32>(seg + 0x34))
}

/// Our ball cast with no target: its flight state's exe becomes [`kh1_ball_exe`] (only when it is
/// Re:CoM's homing thunk with no this-adjust, on a `CHomingBullet`).
unsafe fn seek_on(bullet: usize, id: u32) {
    if !RETARGET.load(Ordering::Acquire) {
        return;
    }
    let b = base();
    let vt: usize = rd(bullet);
    let exe: usize = rd(bullet + rt::STATE_EXE);
    let adj: i32 = rd(bullet + rt::STATE_EXE_ADJUST);
    if vt != va(b, crate::guard::bullet::HOMING_VTABLE) || exe != b + HOMING_THUNK as usize || adj != 0 {
        rt_line(|| format!("re-target: shot {id} is not a plain CHomingBullet in flight (vt {vt:#x}, exe {exe:#x}, adjust {adj}): Re:CoM's homing"));
        return;
    }
    if let Ok(mut g) = SEEKERS.lock() {
        if g.len() >= 16 {
            g.remove(0);
        }
        g.push((id, 0));
    }
    wr(bullet + rt::STATE_EXE, kh1_ball_exe as *const () as usize);
    rt_line(|| format!("re-target: shot {id} cast with no target: KH1's pick each frame (best enemy within {} m of the ball)", mg::retarget::RANGE_M));
}

fn rt_line(line: impl FnOnce() -> String) {
    if RETARGET_LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("magic: {}", line()));
    }
}

/// Re:CoM's enemies in its lock-on list (`0x1401CDB90`'s walk and filter: the handle resolves, the
/// object is not the shot, `+0x154 & 0x10000`), kept to enemies (type 0x1004) with HP left, with
/// their positions. At most 64.
unsafe fn lock_on_enemies(skip: usize) -> Vec<(u32, [f32; 3])> {
    lock_on_list(skip, true)
}

/// Re:CoM's lock-on list's enemies (type 0x1004, HP left), with or without its lock-on flag
/// (`+0x154 & 0x10000`: Thunder's area test reads the bodies instead, as KH1's sys 90 does).
pub(super) unsafe fn lock_on_list(skip: usize, lockable_only: bool) -> Vec<(u32, [f32; 3])> {
    lock_on_walk(skip, lockable_only, false)
}

/// [`lock_on_list`]'s enemies and (owner, 2026-11-04, "Sleights outside battle") in a field room the
/// field Heartless Re:CoM's field lock-on takes ([`is_map_enemy`]): the strike lists of KH1's Thunder
/// and Stop and Ragnarok's shots.
pub(super) unsafe fn lock_on_targets(skip: usize) -> Vec<(u32, [f32; 3])> {
    lock_on_walk(skip, false, true)
}

unsafe fn lock_on_walk(skip: usize, lockable_only: bool, map_too: bool) -> Vec<(u32, [f32; 3])> {
    type TypeFn = unsafe extern "win64" fn(usize) -> u32;
    let mut out = Vec::new();
    let m: usize = rd(va(base(), LOCK_ON_MANAGER));
    if m == 0 {
        return out;
    }
    let mut node: usize = rd(m + rt::LIST);
    let mut guard = 0;
    while node != 0 && guard < 256 && out.len() < 64 {
        let next: usize = rd(node + rt::NODE_NEXT);
        if next == 0 {
            break;
        }
        let h: u32 = rd(node);
        let obj = func::<mv::HandleFn>(mv::HANDLE_OBJ)(h);
        if obj != 0 && obj != skip && (!lockable_only || rd::<u32>(obj + rt::ACTOR_FLAGS) & rt::LOCKABLE != 0) {
            let vt: usize = rd(obj);
            let ty: TypeFn = std::mem::transmute_copy(&rd::<usize>(vt + crate::battle::enemy::VT_TYPE));
            let mover: usize = rd(obj + rt::MOVER);
            let battle_enemy = ty(obj) == crate::battle::ENEMY_TYPE && rd::<i32>(obj + crate::battle::enemy::HP) > 0;
            if (battle_enemy || (map_too && is_map_enemy(obj))) && mover != 0 {
                let p = rd_vec(mover + rt::POSITION);
                out.push((h, [p.x, p.y, p.z]));
            }
        }
        node = next;
        guard += 1;
    }
    out
}

/// The flight-state exe of our balls cast with no target (7.4b): KH1's pick for this frame
/// (`crate::magic::retarget::pick` on the ball's position and its direction before this frame's
/// turn) into Re:CoM's homing target `+0x2F4` (0: none, the ball flies straight this frame), then
/// Re:CoM's own homing step (the thunk → `0x1401575D0`: the ≤ 3° turn toward the pick's aim
/// point, the velocity). Runs only for these balls, once a frame each.
pub unsafe extern "win64" fn kh1_ball_exe(bullet: usize) {
    use crate::guard::bullet as gb;
    let mover: usize = rd(bullet + gb::MOVER);
    let live = rd::<i32>(bullet + rt::HOLD) == 0 && rd::<i32>(bullet + rt::LIFE) > 0 && mover != 0;
    let pick = if live {
        let p = rd_vec(mover + rt::POSITION);
        let dir = crate::guard::forward(rd(mover + gb::PITCH), rd(mover + gb::YAW));
        mg::retarget::pick([p.x, p.y, p.z], dir, lock_on_enemies(bullet))
    } else {
        None
    };
    wr(bullet + gb::HOMING_TARGET, pick.map_or(0, |p| p.0 as i32));
    if live {
        let id: u32 = rd(bullet + 0x20);
        let h = pick.map_or(0, |p| p.0);
        let changed = SEEKERS
            .lock()
            .ok()
            .and_then(|mut g| g.iter_mut().find(|e| e.0 == id).map(|e| std::mem::replace(&mut e.1, h) != h))
            .unwrap_or(false);
        if changed {
            let t = match pick {
                Some((h, d)) => format!("rt shot {id} -> {h} ({d:.2} m)"),
                None => format!("rt shot {id} -> none"),
            };
            event(t.clone());
            rt_line(|| format!("re-target: {t}"));
        }
    }
    func::<PlFn>(HOMING_THUNK)(bullet)
}

/// Re:CoM ends its magic action (`0x1401F6660(player, airborne)`, as `exeMagic` does at its end).
/// True when it entered its win action instead (`0x1401F6794`: the battle's win is pending).
pub unsafe fn end_cast(pl: usize, airborne: bool) -> bool {
    func::<PlIntFn>(END_ACTION)(pl, airborne as i32);
    mv::in_recom_win(pl)
}

/// The frame's cast events (from `win::movement::run_frame`): log and trace.
pub fn on_events(ev: &[CastEvent]) {
    for e in ev {
        let t = e.token();
        event(t.clone());
        note(|| t);
    }
}

/// `CPlayerSora::exeMagic` (action `0x13`; the Battle exe through Sora's vtable, the Field exe
/// through our case): KH1's frame for a KH1 cast, Re:CoM's exe otherwise.
pub unsafe extern "win64" fn exe_magic(pl: usize) {
    let o: PlFn = orig(&ORIG_EXE_MAGIC);
    // 0.184.0: Riku's and Dark Riku's own slots (`+0x150`, Re:CoM's empty `0x1401EEAA0`) point here too
    // ([`riku_slots`]), and the Field exe's case 0x13 (shared with Riku) calls Sora's exeMagic directly: only
    // Sora's object ever runs Sora's exe; anyone else runs the KH1 frame of a KH1 cast or nothing (Re:CoM's
    // own stub).
    if rd::<usize>(pl) != va(base(), crate::sites::data::SORA_VTABLE) {
        if !(mv::kh1_riku(pl) && kh1_cast(pl)) {
            return;
        }
    } else if !mv::kh1_sora(pl) || !kh1_cast(pl) {
        return o(pl);
    }
    if !mv::continues(pl) {
        // Re:CoM ran something else in between (our cast isn't what it ran): end its action and
        // take over from Re:CoM's state; the cast is dropped (no spell, no MP if not released).
        // Unless that end entered Re:CoM's win action: Re:CoM's battle end owns Sora.
        if end_cast(pl, func::<GroundFn>(GROUND_TEST)(pl) != 0) {
            return;
        }
        return mv::run_frame(pl, true);
    }
    mv::run_frame(pl, false);
}

/// The spell of one of our recent shots, by the attacker id a hit body carries (`+0x98`), without
/// taking anything (the Parasite Cage's faint, `win::battle::cage_hit_callback`).
pub fn shot_spell(id: u32) -> Option<u8> {
    SHOTS.try_lock().ok()?.iter().find(|&&(x, _, _, _)| x == id).map(|&(_, s, _, _)| s)
}

/// One of our balls hit this enemy this frame: the hit part (`*(enemy+0xF8)+0x40`, `0x120` bytes
/// each, hit flag `+0x90`, attacker id `+0x98`; Checked (decomp `0x140154EB0`, `0x14016F1C0`)),
/// the spell and level.
pub unsafe fn take_hit(enemy: usize) -> Option<(usize, u8, u8)> {
    let info: usize = rd(enemy + 0xF8);
    if info == 0 {
        return None;
    }
    let g = SHOTS.lock().ok()?;
    if g.is_empty() {
        return None;
    }
    let parts: usize = rd(info + 0x40);
    let n: i32 = rd(info + 0x38);
    for i in 0..n.clamp(0, 32) as usize {
        let part = parts + i * 0x120;
        if rd::<i16>(part + 0x90) == 0 {
            continue;
        }
        let by: u32 = rd(part + 0x98);
        if let Some(&(_, s, l, _)) = g.iter().find(|&&(id, _, _, _)| id == by) {
            return Some((part, s, l));
        }
    }
    None
}

/// The hit part's attribute (`+0x84`), which Re:CoM's hit handler reads for its own resistance
/// (`0x14017F440`: attributes 0..4 from the enemy's `@ENP`, 100 % otherwise). Checked (decomp
/// `0x14017F7E0`).
pub const PART_ATTR: usize = 0x84;
/// An attribute Re:CoM's resistance table doesn't cover (100 %): KH1's resistance is already in
/// the KH1 damage.
pub const NEUTRAL_ATTR: u32 = 0xFF;

/// A logged spell hit (capped), for the log and the trace's hits column.
pub fn hit_line(line: impl FnOnce() -> String) {
    if HIT_LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("magic: {}", line()));
    }
}

/// The F9 trace's cast token: the cast in progress and this frame's events.
pub fn trace_token() -> String {
    let ev = EVENTS.lock().map(|mut g| std::mem::take(&mut *g)).unwrap_or_default();
    let mut t = match mv::SORA.try_lock() {
        Ok(g) => mg::trace_token(g.as_ref().and_then(|s| s.sora.cast.as_ref())),
        Err(_) => "cast ?".into(),
    };
    for e in ev {
        t.push(' ');
        t.push_str(&e);
    }
    if let Ok(g) = BARRIER.try_lock() {
        if let Some(st) = g.as_ref() {
            t.push_str(&format!(" barrier {:.0}", st.b.left.max(0.0)));
        }
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
        "magic: KH1's cardless cast at combat = kh1 in Sora's story (M7 stages 7.4-7.10): Fire / Fira / Firaga on Re:CoM's magic action 0x13 with KH1 rows ({} rows), KH1's release ({} frames after the wind-up / {} on the fast path), KH1's ball (speed {} cm/f, life {} f, turn {}°/f, radius 50 cm) through Re:CoM's CHomingBullet, MP {} units paid at the release; Thunder / Thundara / Thundaga (7.6): at 30, every enemy whose bodies touch a {} m, ±{} m cylinder, nearest first, struck through Re:CoM's own hit path one per {} frames, 1 MP; Blizzard / Blizzara / Blizzaga (7.5): 7 shards on Re:CoM's Blizzard shots, KH1's cone, speed 20/30/40 cm/f, life {} f, 50 cm glide, one hit per enemy per cast; Cure / Cura / Curaga (7.7): at {}, Re:CoM's heal {:?} HP on Sora, 1 MP; Gravity / Gravira / Graviga (7.8): at {}, Re:CoM's sphere 2.4 m above the target falls 2.3 m over {:?} f growing to {:?}, then strikes every enemy within 170 x scale cm once (current HP x (power + 5 x max MP)% x resist[8]), 1 MP; Stop / Stopra / Stopga (7.9): at {}, every enemy in a {:?} cm, +-300 cm cylinder, nearest first, one every {} f, into Re:CoM's stop state for (2 + max MP) x 60 x resist[9] f (0: immune), 2 MP; Aero / Aerora / Aeroga (7.10): at {} (at once if cut), KH1's barrier (18 + max MP) x 60 f halving Sora's damage, Re:CoM's Aero shot following him, Aerora / Aeroga's wind {} cm re-hitting every {} f, 2 MP; Field exe case 0x13 -> exeMagic; KH1's re-target for balls cast with no target (best enemy within {} m of the ball, each frame): {}",
        TABLE.get().map_or(0, |t| t.iter().filter(|r| r[0] != 0).count()),
        28,
        mg::fire::FAST_WAIT,
        mg::fire::SPEED,
        mg::fire::LIFE,
        mg::fire::TURN_DEG,
        mg::fire::COST,
        mg::thunder::RADIUS / 100.0,
        mg::thunder::HALF_HEIGHT / 100.0,
        mg::thunder::INTERVAL,
        mg::blizzard::LIFE,
        mg::cure::WAIT,
        mg::cure::HEAL,
        mg::gravity::WAIT,
        mg::gravity::FRAMES,
        mg::gravity::SCALE,
        mg::stop::WAIT,
        mg::stop::RADIUS,
        mg::stop::INTERVAL,
        mg::aero::WAIT,
        mg::aero::WIND_RADIUS,
        mg::aero::REHIT,
        mg::retarget::RANGE_M,
        if RETARGET.load(Ordering::Acquire) { "on" } else { "off" }
    )
}
