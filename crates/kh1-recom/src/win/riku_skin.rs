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

use super::{log, va};
use crate::riku_skin::{self as rs, Skin};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

static SKINS: OnceLock<Vec<Skin>> = OnceLock::new();
/// The PLAYER table copy holds the skin groups' rows.
static ROWS: AtomicBool = AtomicBool::new(false);
/// The tables point at the skin groups now.
static ON: AtomicBool = AtomicBool::new(false);
/// The menu's model bytes point at the skin groups now.
static MENU_ON: AtomicBool = AtomicBool::new(false);
/// Our motion pack names, per Riku group (kept for the process: Re:CoM reads the pointers).
static NAMES: OnceLock<Vec<(u16, usize)>> = OnceLock::new();

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
    let mut ok = Vec::new();
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

/// The skins whose rows go in the PLAYER table.
pub fn skins() -> Vec<Skin> {
    SKINS.get().cloned().unwrap_or_default()
}

/// The PLAYER table copy with the skins' rows is in.
pub fn rows_in() {
    ROWS.store(true, Ordering::Release);
}

/// The motion pack name pointers for chars 3 / 4 (Riku's or the skins').
unsafe fn name_ptrs(b: usize, sk: &[Skin], on: bool) -> [u64; 2] {
    let names = NAMES.get_or_init(|| {
        sk.iter()
            .map(|s| {
                let c = std::ffi::CString::new(rs::ann_name(s.skin)).unwrap_or_default();
                (s.riku, Box::leak(c.into_boxed_c_str()).as_ptr() as usize)
            })
            .collect()
    });
    let mut out = [0u64; 2];
    for (k, g) in rs::RIKU_GROUPS.iter().enumerate() {
        let recom = va(b, rs::RIKU_ANN_NAMES[k]) as u64;
        out[k] = if on { names.iter().find(|n| n.0 == *g).map_or(recom, |n| n.1 as u64) } else { recom };
    }
    out
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
    if MENU_ON.load(Ordering::Relaxed) == on || !ROWS.load(Ordering::Acquire) {
        return;
    }
    let sk = skins();
    let skin_of = |g: u16| sk.iter().find(|s| s.riku == g).map_or(g, |s| s.skin);
    for &(at, riku) in &rs::MENU_BYTES {
        let (from, to) = if on { (riku, skin_of(riku)) } else { (skin_of(riku), riku) };
        if !super::hook::patch(va(b, at), &[from as u8], &[to as u8], "menu model group (Riku)") {
            log("riku skin: the menu model's group bytes are not Re:CoM's 101 / 111; the menu keeps Riku's model");
            return;
        }
    }
    MENU_ON.store(on, Ordering::Relaxed);
    log(&format!("riku skin: the menu's model (camp, level-up) shows {}", if on { "Dark Mode" } else { "Default Attire" }));
}

/// Points chars 3 / 4 at the skin groups (`on`, Dark Mode) or back at Riku's own.
///
/// # Safety
/// `b` is the running image's base; called on start-up or from the camp (no player being built).
pub unsafe fn apply(b: usize, on: bool) {
    let sk = skins();
    if sk.is_empty() || !ROWS.load(Ordering::Acquire) {
        if on {
            log("riku skin: Dark Mode asked, but the skin's files or rows are not in; Riku keeps his model");
        }
        return;
    }
    let now = ON.load(Ordering::Relaxed);
    if now == on {
        return;
    }
    let groups = |o: bool| -> Vec<u8> { rs::entries(&sk, o).iter().flat_map(|x| x.to_le_bytes()).collect() };
    let names = |o: bool| -> Vec<u8> { name_ptrs(b, &sk, o).iter().flat_map(|x| x.to_le_bytes()).collect() };
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
    ON.store(on, Ordering::Relaxed);
    log(&format!(
        "riku skin: {} (chars 3 / 4 -> groups {:?}, motion packs {:?}); shown from the next room or battle",
        if on { "Dark Mode (Dark Riku's model, Riku's own moves)" } else { "Default Attire" },
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
    Some(rs::entries(&sk, ON.load(Ordering::Relaxed))[k] as u16)
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
    // 0.155.2: Riku's own transformation first; the model changes at its end (`transition`).
    if TRANS_PL.load(Ordering::Relaxed) == 0 {
        func::<super::movement::MotionFn>(super::movement::MOTION_REQUEST)(e, TRANSFORM_ROW, -1, 0, 0, 0, 0);
        TRANS_PL.store(e, Ordering::Relaxed);
        TRANS_FRAMES.store(0, Ordering::Relaxed);
        TRANS_PHASE.store(0, Ordering::Relaxed);
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
    let to_dark = skins().iter().any(|s| s.skin == g);
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
        // Riku's way back effect.
        let m = func::<MtxFn>(0x002A_BB20)(e, TRANSFORM_FX_BONE);
        let h = func::<FxFn>(0x001A_2D50)(e, rd::<usize>(va(b, TRANSFORM_FX_NAME)), 2, m, 0, 0, 0.0);
        let fx = func::<FxObjFn>(0x0025_7EF0)(h);
        if fx != 0 {
            wr::<u32>(fx + 0x4C, rd::<u32>(fx + 0x4C) | 4);
        }
    }
    finish(e)
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
    let on = ON.load(Ordering::Relaxed);
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
