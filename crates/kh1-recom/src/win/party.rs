//! 0.111.0: the command menu's Party list, Re:CoM's side (`crate::party`,
//! `docs/recom-command-pages-party.md`).
//!
//! - **Taught** ([`learned`]): Re:CoM's own learned test `0x14023AB00(id)` for each Party sleight
//!   (the abilities' checked callee): the Customize picker (camp) and the menu.
//! - **Available in this battle** ([`available`]): taught, and every Friend card it needs can drop
//!   here as Re:CoM's drop roll `0x1401602A0` decides it (`crate::party::can_drop`) from Re:CoM's own
//!   getters: the world `0x1402339C0(0)`, the room type `0x140233860`, Riku's story `0x1401AAAE0`,
//!   the minigame room `0x140235360`, the friends met (int var (0, 0x22), `0x1401B08C0`). Read once
//!   per battle (owner: "Whole battle, if possible") at the menu's first build in a battle room;
//!   none of these change inside one. Outside battle: none (friend sleights stay battle-only:
//!   `exeStockSkill`'s friend branch reads the card battle's active card without a null test, and
//!   Re:CoM's sleight packs only finish loading in battle rooms; `docs/recom-field-sleights.md`).
//! - **The start**: Re:CoM's own sleight start (`super::summons::start_party`, the Sleights list's
//!   path), which plays the friend: its pack (`initStockSkill`'s `0x140211790`), the actor
//!   (`0x14020E950`), its attack, its exit.
//!
//! Nothing per frame but one atomic load (and, while a battle's set isn't read yet, the reads).

use super::log;
use super::movement::{func, p, rd};
use crate::party::{self as pt, Place, Set};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

static INSTALLED: AtomicBool = AtomicBool::new(false);
/// This battle's available set (bit 31: read), or 0 (not read / not in battle).
static AVAIL: AtomicU32 = AtomicU32::new(0);
const READ: u32 = 1 << 31;
/// The battle player [`AVAIL`] was read for (each battle room builds its own player).
static AVAIL_PLAYER: AtomicUsize = AtomicUsize::new(0);

const WORLD: u32 = 0x0023_39C0;
const ROOM_TYPE: u32 = 0x0023_3860;
const RIKU: u32 = 0x001A_AAE0;
const MINIGAME: u32 = 0x0023_5360;
const VAR_GET: u32 = 0x001B_08C0;
const LEARNED: u32 = 0x0023_AB00;
/// The friends-met bits (int var set 0, id 0x22; `0x1401602A0`). Checked (decomp).
const MET_VAR: (u32, u32) = (0, 0x22);

type IntFn = unsafe extern "win64" fn(i32) -> i32;
type VoidFn = unsafe extern "win64" fn() -> u32;
type VarGetFn = unsafe extern "win64" fn(u32, u32) -> u64;
type LearnedFn = unsafe extern "win64" fn(u16) -> u64;

/// Checks the Re:CoM code and data the list reads (`sites::PARTY_CHECKED` and the earlier stages'
/// callees). No hook, no patch.
pub unsafe fn install(b: usize) -> bool {
    let have = |at: usize, n: usize| std::slice::from_raw_parts(at as *const u8, n).to_vec();
    let wanted = [WORLD, ROOM_TYPE, RIKU, MINIGAME, VAR_GET, LEARNED, 0x007C_1F10];
    for rva in wanted {
        let Some(site) = crate::sites::all_sites().find(|s| s.rva == rva) else {
            log(&format!("party: no checked site for {:#x}; no Party list", 0x1_4000_0000u64 + rva as u64));
            return false;
        };
        if have(b + rva as usize, site.expect.len()) != site.expect {
            log(&format!("party: {} is not as expected; no Party list", site.name));
            return false;
        }
    }
    INSTALLED.store(true, Ordering::Release);
    install_pluto(b);
    log(&format!(
        "party: the command menu's Party list (page 2): {}; each in a battle where its Friend card can drop (Re:CoM's table 0x1407C1F10) once Re:CoM taught it, through Re:CoM's sleight start",
        pt::ALL.iter().map(|f| format!("{} {:#x} ({} MP)", f.name, f.sleight, f.mp)).collect::<Vec<_>>().join(", ")
    ));
    true
}

pub fn installed() -> bool {
    INSTALLED.load(Ordering::Acquire)
}

/// Lucky Bounty at Combat = KH1 (`crate::party::lucky_bounty`): Re:CoM's Pluto find
/// (`sites::PLUTO_HOOKS`), wrapped.
pub static ORIG_PLUTO_FIND: AtomicUsize = AtomicUsize::new(0);
static PLUTO_LINES: AtomicU32 = AtomicU32::new(40);
/// The find's effect name (`mov rdx,[0x1405A2F90]`: a pointer to "pluto"). Checked (exe).
const PLUTO_NAME_PTR: u32 = 0x005A_2F90;
const EFFECT_POINT: u32 = 0x001D_9DB0;
const EFFECT_AT: u32 = 0x001A_6B20;
const BONE: u32 = 0x002A_BB20;
type FindFn = unsafe extern "win64" fn(usize);

unsafe fn install_pluto(b: usize) {
    for s in crate::sites::PLUTO_CHECKED {
        if std::slice::from_raw_parts((b + s.rva as usize) as *const u8, s.expect.len()) != s.expect {
            log(&format!("party: {} is not as expected; Lucky Bounty keeps Re:CoM's find at KH1", s.name));
            return;
        }
    }
    let hooks = [super::hook::Hook { site: &crate::sites::PLUTO_HOOKS[0], ours: pluto_find as *const () as usize, original: &ORIG_PLUTO_FIND }];
    if super::hook::install_all(b, &hooks) {
        log("party: Lucky Bounty at KH1: Pluto's find 0x1401E2E60 wrapped; his card find (outcome 2, empty deck) brings 1-2 KH1 items of the floor's tier (two at 50 %) (Potion, Hi-Potion, Ether, Elixir) as a KH1 item prize; his other finds Re:CoM's");
    }
}

#[repr(C, align(16))]
struct V4([f32; 4]);

/// Re:CoM's Pluto find; then, in a KH1 battle, his card find (the empty deck dealt nothing) brings 1–2
/// KH1 items: Re:CoM's "pluto" find effect at his point and one KH1 item prize each from where Re:CoM
/// throws his prizes (his bone 0, 0.5 m up), picked up by KH1's rule.
pub unsafe extern "win64" fn pluto_find(pluto: usize) {
    super::movement::orig::<FindFn>(&ORIG_PLUTO_FIND)(pluto);
    if pluto == 0 || !super::battle::kh1_battle() {
        return;
    }
    let outcome: u16 = rd(pluto + 0x472);
    if outcome != pt::lucky_bounty::CARDS_OUTCOME {
        return;
    }
    let tier = super::shop::tier();
    // 1–2 items (owner, 2026-11-04): the count, then each item, from Re:CoM's random.
    let items = pt::lucky_bounty::find(tier, super::drops::random(), || super::drops::random());
    if items.is_empty() {
        return;
    }
    // Re:CoM's find effect (as its outcomes 1 / 2 play it), when the name is still "pluto".
    let name: usize = rd(super::base() + PLUTO_NAME_PTR as usize);
    if name != 0 && std::ffi::CStr::from_ptr(name as *const std::ffi::c_char).to_bytes() == b"pluto" {
        type PointFn = unsafe extern "win64" fn(usize, *mut V4) -> usize;
        type EffectFn = unsafe extern "win64" fn(usize, usize, u32, *const V4, u64, f32) -> i32;
        let mut at = V4([0.0; 4]);
        func::<PointFn>(EFFECT_POINT)(pluto, &mut at);
        func::<EffectFn>(EFFECT_AT)(pluto, name, 1, &at, 0, 0.0);
    }
    type BoneFn = unsafe extern "win64" fn(usize, u32) -> usize;
    let bone = func::<BoneFn>(BONE)(pluto, 0);
    if bone == 0 {
        return;
    }
    let v: [f32; 4] = rd(bone + 0xD0);
    let pos = [v[0], v[1] + 0.5, v[2], v[3]];
    // Each a KH1 item prize of its own, thrown from his prize point (Re:CoM's own scatter for each).
    let mut how = Vec::with_capacity(items.len());
    for &item in &items {
        how.push(match super::drops::spawn_item_prize(pos, item) {
            Some(prize) => format!("{}: a KH1 item prize {prize:#x}", crate::items::name(item)),
            None => {
                let r = super::items::pick_up(item, "found by Pluto (Lucky Bounty)");
                format!("{}: picked up at once (no KH1 item prizes): {r:?}", crate::items::name(item))
            }
        });
    }
    if PLUTO_LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("party: Lucky Bounty at KH1: Pluto's card find (empty deck) brings {} item(s) (floor tier {tier:?}): {}", items.len(), how.join("; ")));
    }
}

/// The Party sleights Re:CoM has taught (Re:CoM's own test, a pure read of its save).
pub fn learned() -> Set {
    if !installed() {
        return 0;
    }
    // SAFETY: Re:CoM's own test (bytes checked at install), a pure read.
    let f = unsafe { func::<LearnedFn>(LEARNED) };
    let mut s = 0;
    // 0.198.16: Riku's story too follows Re:CoM's test: the King's sleights (0x62..0x64) are taught by
    // B11's hall script `WORLD00/2103` (0.150.0-0.198.15 counted them as taught from the start).
    for (i, fr) in pt::ALL.iter().enumerate() {
        // SAFETY: as above.
        if unsafe { f(fr.sleight) } & 0xFF != 0 {
            s |= 1 << i;
        }
    }
    s
}

/// The Party sleights' ids Re:CoM has taught (the Customize picker's).
pub fn learned_ids() -> Vec<u16> {
    let s = learned();
    pt::ALL.iter().enumerate().filter(|(i, _)| s >> i & 1 != 0).map(|(_, f)| f.sleight).collect()
}

/// Where this battle is, from Re:CoM's getters.
unsafe fn place() -> Place {
    Place {
        world: func::<IntFn>(WORLD)(0) & 0xFF,
        room_type: func::<VoidFn>(ROOM_TYPE)(),
        riku: func::<VoidFn>(RIKU)() & 0xFF != 0,
        minigame: func::<VoidFn>(MINIGAME)() & 0xFF != 0,
        met: func::<VarGetFn>(VAR_GET)(MET_VAR.0, MET_VAR.1) as u32,
    }
}

/// (taught, available in this battle) for player `pl`'s menu build. Read at the first build in a
/// battle room and kept for that battle's player (bit 31 read, the taught set in bits 16.., the
/// available set in bits 0..); outside battle: (taught, none), Re:CoM's test each call (11 calls of
/// a save read, only while page 2 shows or a slot holds a Party sleight; not measured).
pub unsafe fn sets(pl: usize) -> (Set, Set) {
    if !installed() || pl == 0 {
        return (0, 0);
    }
    if rd::<i32>(pl + p::BATTLE) == 0 {
        AVAIL.store(0, Ordering::Relaxed);
        return (learned(), 0);
    }
    let a = AVAIL.load(Ordering::Relaxed);
    if a & READ != 0 && AVAIL_PLAYER.load(Ordering::Relaxed) == pl {
        return (((a >> 16) & 0x7FFF) as Set, (a & 0x7FFF) as Set);
    }
    let taught = learned();
    let at = place();
    let s = pt::available(taught, &at);
    AVAIL.store(READ | (taught as u32) << 16 | s as u32, Ordering::Relaxed);
    AVAIL_PLAYER.store(pl, Ordering::Relaxed);
    log(&format!(
        "party: this battle (world {}, room type {}, friends met {:#x}, drop row {}; taught {:#x}): {}",
        at.world,
        at.room_type as i32,
        at.met,
        pt::drop_row(&at),
        taught,
        if s == 0 { "no Party sleight available".to_string() } else { pt::list(s).iter().map(|&i| pt::ALL[i].name).collect::<Vec<_>>().join(", ") }
    ));
    (taught, s)
}
