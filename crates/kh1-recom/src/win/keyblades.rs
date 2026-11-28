//! The camp's top list at Combat = KH1 (owner, 2026-11-04: Items, Abilities, Customize, World Map,
//! Map Cards, World Cards, Status, Journal, Settings), the Settings list (Game Settings, Combat,
//! Movement) and our pages, run by Re:CoM's own camp code. Pure rules: `crate::keyblades` (`top`,
//! `sub`), `crate::customize`. Notes: `docs/recom-keyblade-equip.md` (§ The KH1 camp list).
//!
//! - KH1 list: Re:CoM's 7 entry buttons keep their camp indices (0..6) and get their places on
//!   screen in KH1's order; entry 0 (Review Decks) is relabelled "Items"; Abilities and Customize
//!   are two more `C2DSeqCtrl` + `C2DFnt` made with the calls Re:CoM's entry build `0x1400E3DE0`
//!   makes for its own, at camp indices 7 and 8. Four wrappers make the camp treat them as nine:
//!   the camp update `0x1400D9C80` (states 7 / 0xD / 0xF / 0x11), the entry build (create, delete,
//!   relabel, places, animations 4 / 5 / 6), the cursor set `0x1400DEA10` (highlight, bar, glove,
//!   help, order on screen) and the teardown `0x1400D7C20` (delete ours).
//! - Pages: Items (M8 stage 8a.2; Sora's list starts with his Keyblade slot, which shows the
//!   Keyblade list on the same page), Abilities and Customize follow the entry-4 page (`0x1400EB7E0`
//!   / draw `0x1400E6DF0`) with Re:CoM's classes, inside Re:CoM's Config states 0x4A..0x4C (the
//!   Config input hook, `menu::cfg_input` → [`page`]). Their state is ours ([`OPEN`]); they never
//!   write camp fields Re:CoM's pages use.
//! - Settings list (Sora's story, both Combat settings): Re:CoM's own 7 entry buttons, relabelled
//!   with our rows as Re:CoM's Deck submenu relabels them (level 1), with the Deck submenu's input,
//!   cursor set, sounds, breadcrumb (header item 1) and fades, in the Config states. Game Settings
//!   runs Re:CoM's Config page itself (`menu::recom_config`). Leaving it, Re:CoM's state 0x4C
//!   rebuilds the top list (level 1 → 0), and our entry-build wrapper puts the list for the Combat
//!   setting now in place.
//!
//! Nothing here runs in field or battle frames: the camp exists only while the pause menu is
//! open. In Riku's story and the camp that opens straight onto the map every wrapper is a load and
//! the original.
//!
//! Every offset and call below is Checked (decomp + disasm) unless tagged otherwise.

use super::movement::{func, orig, rd, wr};
use super::{base, log, va};
use crate::command_menu as cm;
use crate::customize;
use crate::items_page as ip;
use crate::abilities::page as ap;
use crate::keyblades::{self as kb, list, sub, text, top};
use crate::sites::data;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU16, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

pub static ORIG_UPDATE: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_BUILD: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_CURSOR: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_TEARDOWN: AtomicUsize = AtomicUsize::new(0);

/// The asset step's item row (`crate::camp_rows`): our list table, Re:CoM's 7 entries of
/// `0x1407C11F0` (read from the image) and the new group of the loaded language's copy (set before
/// each set-up from the loaded set's group count); and every copy's group from
/// `kh1-camp-rows.json`. Set once at install; Re:CoM's row set-up keeps the table's pointer, so it
/// lives as long as the DLL (`AtomicU16` has `u16`'s layout).
static ITEM_TABLE: OnceLock<[AtomicU16; crate::camp_rows::RECOM_ENTRIES + 1]> = OnceLock::new();
static ITEM_ROWS: OnceLock<Vec<u16>> = OnceLock::new();
static ITEM_SET_LOGGED: AtomicBool = AtomicBool::new(false);
static SPARE_LOGGED: AtomicBool = AtomicBool::new(false);
static NO_NUMBERS_LOGGED: AtomicBool = AtomicBool::new(false);
static ODD_STATE_LOGGED: AtomicBool = AtomicBool::new(false);
/// The "Equipped" word's first draw logged (`Page::equipped_word`).
static WORD_LOGGED: AtomicBool = AtomicBool::new(false);

/// `CCamp` fields (Checked, decomp `0x1400D9C80` and the functions named).
mod camp {
    /// Frame time (int; the pages animate their cursor by `dt × 0.02`, `0x1400E6DF0` mode 3).
    pub const DT: usize = 0x14;
    /// Update state.
    pub const STATE: usize = 0x6C;
    /// Layout (`camp.lad`) and sprite set name (`"camp:00"`).
    pub const LAYOUT: usize = 0x78;
    pub const SET: usize = 0x80;
    /// Fresh-press flag left by the camp's direction read `0x140111A70`.
    pub const FRESH: usize = 0x94;
    /// Text colour of list rows and the header title.
    pub const TEXT_COLOUR: usize = 0x98;
    /// Non-zero: no text offsets (`0x1400E6DF0` picks 0.0 instead of 1.0 / 2.0).
    pub const NO_SCALE: usize = 0x9C;
    /// A page's sub-state (Re:CoM's pages start at 0; state 0x4A writes it).
    pub const PAGE_STATE: usize = 0xA8;
    /// Menu level: 0 top list, 1 Deck submenu, 2 in a page.
    pub const LEVEL: usize = 0xB4;
    pub const CURSOR: usize = 0xB8;
    /// The top index saved on confirm.
    pub const SAVED: usize = 0xBC;
    /// Header item 0's mode (the update's trailer runs `0x1400E4220(camp, [+0xC4], 0)`).
    pub const HEADER_MODE: usize = 0xC4;
    /// 8 bytes: gliding (`u32` 1) and the glide's time (`f32`), `0x1400E3D30` / `0x1400DE100`.
    pub const GLIDING: usize = 0xCC;
    pub const MAIN_PANEL: usize = 0x130;
    pub const PANEL_140: usize = 0x140;
    /// Entry buttons (`C2DSeqCtrl`, 7) and their labels (`C2DFnt`, 7).
    pub const BUTTONS: usize = 0x1B8;
    pub const LABELS: usize = 0x1F0;
    /// Header items 0 and 1 (`C2DSeqCtrl`, the breadcrumb) and their titles (`C2DFnt`), 8 apart.
    pub const HEADER: usize = 0x228;
    pub const HEADER_TITLE: usize = 0x238;
    /// The top list's highlight bar (sequence 0x12) and glove (sequence 0x14), made in state 8.
    pub const BAR: usize = 0x258;
    pub const GLOVE: usize = 0x260;
    pub const HELP: usize = 0x9D0;
    /// The glove now, its glide target and the glide's start (`[f32; 4]` each).
    pub const GLIDE_NOW: usize = 0xF40;
    pub const GLIDE_TARGET: usize = 0xF50;
    pub const GLIDE_FROM: usize = 0xF60;
    /// Non-zero: the camp opens straight into the map page and never shows the list.
    pub const DIRECT_MAP: usize = 0x20A4;
    /// `0x1401AAAE0()` at ctor: Riku's story (Re:CoM's index for its per-story tables).
    pub const RIKU: usize = 0x21BC;
    pub const COUNT: usize = 0x21C4;
    /// Non-zero: the camp's pad helpers (`0x140111A70` / `AE0` / `B90` / `C30`) answer 0.
    pub const INPUT_LOCK: usize = 0x8C;
    /// The Sleights List's sub-state (`0x1400EBE30`): 0 open .. 4 closing, −1 idle (the ctor's).
    pub const SLEIGHTS_LIST: usize = 0xAC;
    /// The prompt bar of slot 0 (`C2DSeqCtrl`, `0x1400DEC60`; slot 1 at +8).
    pub const PROMPT_BAR: usize = 0x4E0;
}

// Re:CoM functions (RVAs; first bytes checked in `sites`).
const SOUND: u32 = 0x002A_A360;
const NEW: u32 = 0x004A_CE54;
const CBUTTON_CTOR: u32 = 0x0011_2A90;
const CBUTTON_SETUP: u32 = 0x0011_2F10;
const LAYOUT: u32 = 0x0024_6100;
const TEXT_SET: u32 = 0x0025_9000;
const SPRITE_POS: u32 = 0x0024_85B0;
const SEQ_ENDED: u32 = 0x0024_80C0;
const PAD_REPEAT: u32 = 0x0029_B2E0;
const PAD_FRESH: u32 = 0x0029_B2F0;
const PAD_CANCEL: u32 = 0x0029_B410;
const PAD_CONFIRM: u32 = 0x0029_B490;
const GUIDE: u32 = 0x002B_57A0;
const CONFIRM_FLASH: u32 = 0x000D_3A70;
const PRIORITY: u32 = 0x0024_7AA0;
const VISIBLE: u32 = 0x0024_7DC0;
const HIGHLIGHT_BAR: u32 = 0x000E_07D0;
const GLIDE_TARGET: u32 = 0x000E_3D30;
const MSG: u32 = 0x0024_0160;
const HEADER: u32 = 0x000E_4220;
const FNT_CTOR: u32 = 0x0024_3F10;
const SEQ_CTOR: u32 = 0x0024_7260;
const LAYOUT_POS: u32 = 0x0024_8660;
const CAMP_PANEL: u32 = 0x000E_6700;
const SLOT_VISIBLE: u32 = 0x0024_7E50;
const SIDE_PANEL: u32 = 0x000E_46D0;
const GLIDE_STEP: u32 = 0x000D_E100;
const CAMP_DIR: u32 = 0x0011_1A70;
const CAMP_CANCEL: u32 = 0x0011_1B90;
const CAMP_DECIDE: u32 = 0x0011_1C30;
const CURSOR_CTOR: u32 = 0x000D_25F0;
const CURSOR_GLIDE: u32 = 0x000D_E010;
const CURSOR_HIDE: u32 = 0x000E_5CF0;
const BAR_CTOR: u32 = 0x0011_2AB0;
const BAR_SETUP: u32 = 0x0011_30C0;
const BAR_UPDATE: u32 = 0x0011_2CB0;
const TEXT_COPY: u32 = 0x0025_8DE0;
const EVER_OBTAINED: u32 = 0x0015_B3F0;
const CARD_NAME: u32 = 0x0016_4CA0;
/// Re:CoM's Sleights List update (`0x1400EBE30`, the workspace's `knowledge/ReCoM/recom-riku-sleights.md` § 3).
const SLEIGHTS_LIST: u32 = 0x000E_BE30;
/// The camp's prompt bars (`0x1400DEC60`: (camp, mode 1 show / 4 close, slot, variant)).
const PROMPT_BAR: u32 = 0x000D_EC60;
/// The camp's fresh press (`0x140111AE0`: (camp, pad bits, 0), 0 while `camp+0x8C` is set).
const CAMP_PRESS: u32 = 0x0011_1AE0;
/// Square, as the deck page's state 0x30 reads it (`0x1400DBCE6`).
const SQUARE: u32 = 0x80;
/// 0.198.18: Re:CoM's Sleights page's tab keys (`0x1400EB7E0` state 4, `0x140111A70(camp, mask, 0)`, Left index − 1,
/// Right + 1), as the list's Up `0x101000` / Down `0x204000`. Checked (decomp). The low bits are the PS2-order D-pad
/// (0x8000 Left, 0x2000 Right); Assumption: the high bits are the stick, as in Up / Down.
const LEFT: u32 = 0x40_8000;
const RIGHT: u32 = 0x80_2000;
/// Riku's D Mode card (Re:CoM's learned test answers it once `WORLD00/2102` gives it; the workspace's
/// `recom-riku-sleights.md` § 1-2).
const D_MODE: u16 = 0x6D;

// Re:CoM data (Steam addresses).
/// The message tables (`CCtdList` owner) passed to `0x140240160`.
const MSG_TABLES: u64 = 0x1_4087_CD78;
/// The empty text the cursor set clears the help line with (`mov rdx,[0x1407C12E8]`).
const HELP_CLEAR: u64 = 0x1_407C_12E8;
/// The help message id the entry-4 page restores when it closes (`0x1400E6DF0` mode 4).
const CLOSE_HELP_ID: u64 = 0x1_407C_106C;
/// Sora's card collection (`save + 0x1A74`), read by `0x14015B3F0`.
const COLLECTION: u64 = 0x1_4087_A7E8;
const STATUS_SEQ: u64 = 0x1_407C_11D0;
const LIST_SEQ: u64 = 0x1_407C_11F0;
const BAR_SEQ: u64 = 0x1_407C_1228;

// Values Re:CoM's own code uses (Checked, exe / disasm).
/// Label colours: normal, highlighted (`0x1400E3DE0`, `0x1400DEA10`).
const LABEL_NORMAL: u32 = 0x8040_4040;
const LABEL_HIGHLIGHT: u32 = 0x80C0_C0C0;
/// The status rows' value colour (`0x1400E6DF0`, rows 0..4).
const VALUE_COLOUR: u32 = 0x8004_6060;
/// The list cursor's second sprite: + (284, 168) from the row (`0x140509530`, `0x140509520`).
const CURSOR_DX: f32 = 284.0;
const CURSOR_DY: f32 = 168.0;
// A marked row's "Equipped" word (owner, 2026-11-05; before 0.140.0 Re:CoM's "NEW!" badge, its
// sequence 0x1D at (170, −13)): `crate::keyblades::badge`, drawn by [`Page::equipped_word`].

type VoidFn = unsafe extern "win64" fn(*mut u8);
type BuildFn = unsafe extern "win64" fn(*mut u8, i32, u64) -> u64;
type CursorSetFn = unsafe extern "win64" fn(*mut u8, i32, i32) -> u64;
type CampIntFn = unsafe extern "win64" fn(*mut u8, i32, i32) -> u64;
type HeaderFn = unsafe extern "win64" fn(*mut u8, u32, u32) -> u64;
type SoundFn = unsafe extern "win64" fn(i32, i32) -> u64;
type NewFn = unsafe extern "win64" fn(usize) -> *mut u8;
type CtorFn = unsafe extern "win64" fn(*mut u8) -> *mut u8;
type DeleteFn = unsafe extern "win64" fn(*mut u8, u32) -> *mut u8;
/// `C2DSeqCtrl` vtable `+0x8`: (ctrl, set, sequence, 0.0, 0).
type SeqInitFn = unsafe extern "win64" fn(*mut u8, *const u8, u32, f32, u32) -> u64;
/// `+0x20`: (ctrl, slot, set, sequence, 0.0, 0).
type SeqAnimFn = unsafe extern "win64" fn(*mut u8, u32, *const u8, u32, f32, u32) -> u64;
/// `+0x60` (`0x1402476F0`): add a text item with an offset (x, y) and a third float.
type AttachFn = unsafe extern "win64" fn(*mut u8, *const u8, *mut u8, f32, f32, f32) -> u64;
/// `+0x68` (`0x1402481A0`): replace slot `n`'s item with a new text item.
type ReplaceFn = unsafe extern "win64" fn(*mut u8, u32, *const u8, *mut u8) -> u64;
type FntCtorFn = unsafe extern "win64" fn(*mut u8, i32, i32, *const u32, i32, *const u8, i32) -> *mut u8;
#[allow(clippy::type_complexity)]
type SetupFn = unsafe extern "win64" fn(*mut u8, *const u8, u32, *const u16, *const f32, *mut u8, *const u8, i32, f32, f32, i32, *const u32, i32);
type LayoutFn = unsafe extern "win64" fn(*const u8, u32, u64, *mut f32, i32) -> u64;
type PosFn = unsafe extern "win64" fn(*mut u8, *const f32);
type SprIntFn = unsafe extern "win64" fn(*mut u8, u32);
type SlotVisFn = unsafe extern "win64" fn(*mut u8, u32, u32);
type EndedFn = unsafe extern "win64" fn(*mut u8, u32) -> u64;
type PadFn = unsafe extern "win64" fn(u32, u32) -> u32;
type PadBtnFn = unsafe extern "win64" fn(u32) -> u32;
type GuideFn = unsafe extern "win64" fn(u32, u32);
type MsgFn = unsafe extern "win64" fn(usize, u32) -> *const u8;
type CampDirFn = unsafe extern "win64" fn(*mut u8, u32, u32) -> u32;
type CampBtnFn = unsafe extern "win64" fn(*mut u8, u32) -> u32;
type CursorCtorFn = unsafe extern "win64" fn(*mut u8, u32, *mut u8, u32) -> *mut u8;
type GlideFn = unsafe extern "win64" fn(*mut u8, f32);
type BarSetupFn = unsafe extern "win64" fn(*mut u8, *const u8, u32, *const u16, *mut f32, f32, *mut u8);
type BarUpdateFn = unsafe extern "win64" fn(*mut u8, f32, f32);
type TextFn = unsafe extern "win64" fn(*mut u8, *const u8);
type EverFn = unsafe extern "win64" fn(u32) -> u32;
type NameFn = unsafe extern "win64" fn(*const u8) -> *const u8;
type ListFn = unsafe extern "win64" fn(*mut u8) -> u64;
type PromptFn = unsafe extern "win64" fn(*mut u8, u32, i32, i32) -> u64;

#[repr(C, align(16))]
#[derive(Clone, Copy, Default)]
struct Vec4([f32; 4]);

unsafe fn at<T: Copy>(p: *mut u8, off: usize) -> T {
    rd::<T>(p as usize + off)
}

unsafe fn put<T: Copy>(p: *mut u8, off: usize, v: T) {
    wr::<T>(p as usize + off, v)
}

unsafe fn vfn<T: Copy>(obj: *mut u8, slot: usize) -> T {
    let vt: usize = rd(obj as usize);
    let f: usize = rd(vt + slot);
    std::mem::transmute_copy(&f)
}

unsafe fn set_of(camp: *mut u8) -> *const u8 {
    at(camp, camp::SET)
}

/// Reads `kh1-camp-rows.json` (the asset step's item row, `crate::camp_rows`) and builds our list
/// table. Without it the item rows keep Re:CoM's green row 0x131.
pub fn load_item_row(file: Option<&std::path::Path>) {
    let Some(path) = file else {
        log("camp rows: no kh1-camp-rows.json: item rows keep Re:CoM's green row 0x131 (Mickey symbol)");
        return;
    };
    let rows = match std::fs::read_to_string(path).map_err(|e| e.to_string()).and_then(|t| crate::camp_rows::parse(&t)) {
        Ok(r) => r,
        Err(e) => {
            log(&format!("camp rows: {}: {e}: item rows keep Re:CoM's green row 0x131", path.display()));
            return;
        }
    };
    // SAFETY: Re:CoM's camp list table in the image (its bytes checked at start-up, `sites`).
    let recom: [u16; crate::camp_rows::RECOM_ENTRIES] = std::array::from_fn(|i| unsafe { rd::<u16>(va(base(), LIST_SEQ) + 2 * i) });
    let t = crate::camp_rows::list_table(recom, rows[0]);
    let _ = ITEM_TABLE.set(t.map(AtomicU16::new));
    log(&format!(
        "camp rows: {} item rows = camp:00 groups {rows:x?} (base file, language copies; variant {}, our table {t:x?}); used when the loaded camp:00 ends with one",
        path.display(),
        kb::row_look::ITEM
    ));
    let _ = ITEM_ROWS.set(rows);
}

/// The item row of the loaded camp set (`set`, Re:CoM's NUL-terminated name): its last group when
/// that is one of the manifest's, its group count read from Re:CoM's 2D registry. Logged once
/// when none (and once with the row found).
unsafe fn item_row_loaded(set: *const u8, rows: &[u16]) -> Option<u16> {
    if set.is_null() {
        return None;
    }
    let mut name = Vec::with_capacity(16);
    for k in 0..32 {
        let c = *set.add(k);
        name.push(c);
        if c == 0 {
            break;
        }
    }
    if name.last() != Some(&0) {
        return None;
    }
    let n = super::mp_bar::set_groups(&name);
    let row = crate::camp_rows::loaded_row(n, rows);
    if !ITEM_SET_LOGGED.swap(true, Ordering::Relaxed) {
        let set = String::from_utf8_lossy(&name[..name.len() - 1]);
        let groups = n.map_or("no".to_string(), |n| n.to_string());
        log(&match row {
            Some(r) => format!("camp rows: {set} has {groups} groups: item row = group {r:#x}"),
            None => format!(
                "camp rows: {set} has {groups} groups, none of {rows:?} + 1 (Panacea serves Re:CoM's own FORM/0100/FO0100.SPR, or this language's copy was not patched): item rows keep Re:CoM's green row 0x131"
            ),
        });
    }
    row
}

/// `vt+0x20(obj, 0, set, sequence, 0.0, 0)`, as Re:CoM gives its menu sprites a sequence.
unsafe fn anim(obj: *mut u8, set: *const u8, seq: u32) {
    if !obj.is_null() {
        vfn::<SeqAnimFn>(obj, 0x20)(obj, 0, set, seq, 0.0, 0);
    }
}

unsafe fn delete(obj: *mut u8) {
    if !obj.is_null() {
        vfn::<DeleteFn>(obj, 0)(obj, 1);
    }
}

unsafe fn sound(id: i32) {
    func::<SoundFn>(SOUND)(id, -1);
}

unsafe fn ended(obj: *mut u8) -> bool {
    !obj.is_null() && func::<EndedFn>(SEQ_ENDED)(obj, 0) as u32 != 0
}

unsafe fn message(id: u32) -> *const u8 {
    let tables: usize = rd(va(base(), MSG_TABLES));
    func::<MsgFn>(MSG)(tables, id)
}

/// Re:CoM's fonts (`0x14087CFC0`, 0x158 bytes each, `0x14087CFB8` of them; a `C2DFnt` draws with
/// font `+0x40` bit 12, `0x14025AACE`) and the flag its set-up gives the "sysfont" (`+0x150` = 1,
/// `0x14025C400`): only that font has the button sheet the glyph picker draws escapes from
/// (`0x14025A0B4`: glyphs 0x55C..0x610). Checked (decomp + exe).
const FONTS: u64 = 0x1_4087_CFC0;
const FONT_COUNT: u64 = 0x1_4087_CFB8;
const FONT_SIZE: usize = 0x158;
const FONT_BUTTONS: usize = 0x150;
const FNT_FLAGS: usize = 0x40;
/// The list and status rows' font: `CBUTTON_SETUP`'s flags 0x40 (bit 12 clear).
const ROW_FONT: u32 = 0;

/// Re:CoM's font `idx` draws button escapes (its button sheet is loaded).
fn button_glyphs(idx: u32) -> bool {
    // SAFETY: static data of the image (Re:CoM's font table).
    let ok = unsafe {
        let n: u32 = rd(va(base(), FONT_COUNT));
        idx < n.min(2) && rd::<u32>(va(base(), FONTS) + idx as usize * FONT_SIZE + FONT_BUTTONS) == 1
    };
    if !GLYPHS_LOGGED.swap(true, Ordering::Relaxed) {
        log(&format!(
            "customize: slot labels {} (Re:CoM's font {idx} {} a button sheet)",
            if ok { "as Re:CoM's button glyphs" } else { "as button names" },
            if ok { "has" } else { "has no" }
        ));
    }
    ok
}
static GLYPHS_LOGGED: AtomicBool = AtomicBool::new(false);

/// The camp's help line draws button escapes (its `C2DFnt`'s font has the button sheet).
fn help_glyphs(camp: *mut u8) -> bool {
    // SAFETY: the camp's help item, read as `set_help` writes it.
    unsafe {
        let help: *mut u8 = at(camp, camp::HELP);
        !help.is_null() && button_glyphs((at::<u32>(help, FNT_FLAGS) >> 12) & 1)
    }
}

unsafe fn set_help(camp: *mut u8, s: *const u8) {
    let help: *mut u8 = at(camp, camp::HELP);
    if !help.is_null() && !s.is_null() {
        func::<TextFn>(TEXT_SET)(help, s);
    }
}

// ---------------------------------------------------------------------------------------------
// The top list's state (one camp at a time: Re:CoM has one camp, `DAT_14087A300`).

/// The camp the state below belongs to.
static LIST_CAMP: AtomicUsize = AtomicUsize::new(0);
/// Its list (`top::Layout`, as [`layout_code`]), decided at each build of the top list.
static LIST: AtomicU8 = AtomicU8::new(0);
/// Our two entry buttons (`C2DSeqCtrl`, camp indices 7 / 8) and their labels (`C2DFnt`).
static EXTRA_BTN: [AtomicUsize; top::MAX_EXTRA] = [AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0)];
static EXTRA_FNT: [AtomicUsize; top::MAX_EXTRA] = [AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0)];
/// Re:CoM's button 0 carries our "Items" label (Re:CoM's relabels replace it).
static ITEMS_ON: AtomicBool = AtomicBool::new(false);
/// Re:CoM's 7 buttons stand at the KH1 list's places.
static MOVED: AtomicBool = AtomicBool::new(false);
/// While Re:CoM's state 0xF runs at the KH1 list: the entry its move goes to (−1: none). Its
/// cursor set call takes this index instead of Re:CoM's ±1.
static REDIRECT: AtomicI32 = AtomicI32::new(-1);
/// The last top entry of ours confirmed (camp index; −1 none): its page runs in the Config states.
static CONFIRMED: AtomicI32 = AtomicI32::new(-1);
static LAYOUT_LOGGED: AtomicU8 = AtomicU8::new(0xFF);

fn layout_code(l: top::Layout) -> u8 {
    match l {
        top::Layout::Recom => 0,
        top::Layout::Settings => 1,
        top::Layout::Kh1 => 2,
        top::Layout::Riku => 3,
        top::Layout::RikuDecks => 4,
        top::Layout::RikuPlain => 5,
    }
}

/// The list last built (any camp).
fn list_now() -> top::Layout {
    match LIST.load(Ordering::Relaxed) {
        5 => top::Layout::RikuPlain,
        4 => top::Layout::RikuDecks,
        3 => top::Layout::Riku,
        2 => top::Layout::Kh1,
        1 => top::Layout::Settings,
        _ => top::Layout::Recom,
    }
}

/// The list `camp` shows (Re:CoM's until its first build).
fn layout_of(camp: *mut u8) -> top::Layout {
    if camp.is_null() || LIST_CAMP.load(Ordering::Relaxed) != camp as usize {
        return top::Layout::Recom;
    }
    list_now()
}

/// The list the camp should show now (`top::layout`): read at each build of the top list, so
/// leaving the Settings list with Combat changed switches it.
unsafe fn layout_now(camp: *mut u8) -> top::Layout {
    let story = story(camp);
    let direct = at::<i32>(camp, camp::DIRECT_MAP) != 0;
    let combat = super::settings().combat == crate::settings::Mode::Kh1;
    top::layout(story, direct, combat, combat && story == kb::Story::Sora && kh1_pages_listed(), super::settings().riku_kh1_movement)
}

/// True when the Config input hook must run our code: the Settings list (Re:CoM's Settings entry
/// confirmed in Sora's story, `camp+0xBC` = 6, which Re:CoM's state 0xF writes on every confirm)
/// or the page of one of our KH1 list entries (our confirm). The cursor `camp+0xB8` can't tell:
/// inside the list it is the list's row, as in Re:CoM's Deck submenu.
pub fn page_active(camp: *mut u8) -> bool {
    let l = layout_of(camp);
    if l == top::Layout::Recom {
        return false;
    }
    // SAFETY: a live camp (Re:CoM just passed it to its own Config input function).
    let saved = unsafe { at::<i32>(camp, camp::SAVED) };
    ours_confirmed(saved) || (saved == top::SETTINGS && top::settings_list(l))
}

/// `saved` is the entry of ours our confirm opened.
fn ours_confirmed(saved: i32) -> bool {
    CONFIRMED.load(Ordering::Relaxed) == saved && top::entry(list_now(), saved).and_then(|e| e.page()).is_some()
}

/// The camp's story, as Re:CoM's camp code reads it for its own tables.
unsafe fn story(camp: *mut u8) -> kb::Story {
    kb::Story::from_camp(at::<i32>(camp, camp::RIKU))
}

/// The stats file lists Keyblades (the KH1 pages' condition since 0.8.0; logged once when not).
fn kh1_pages_listed() -> bool {
    let listed = !super::stats::keyblade_cards().is_empty();
    if !listed && !NO_NUMBERS_LOGGED.swap(true, Ordering::Relaxed) {
        log("camp: kh1-stats.json has no Keyblades (format 3, kh1-recom-build): Re:CoM's seven entries at Combat = KH1 too (Settings opens Game Settings, Combat, Movement, Effects & Sound, Camera)");
    }
    listed
}

/// Logs, once per session, whether `save + 0x246B ..= 0x2817` is zero apart from our bytes (the
/// check that the spare range really is unused, `recom-cards-save.md` § 4).
unsafe fn spare_check() {
    if SPARE_LOGGED.swap(true, Ordering::Relaxed) {
        return;
    }
    let save: usize = rd(va(base(), kb::SAVE_PTR));
    if save == 0 {
        log("keyblades: save block not there at the first camp open; spare range not checked");
        return;
    }
    let range = std::slice::from_raw_parts((save + kb::SPARE_FIRST) as *const u8, kb::SPARE_LAST - kb::SPARE_FIRST + 1);
    let ours: u8 = rd(save + kb::EQUIP_OFFSET);
    let nz = kb::spare_range_nonzero(range);
    if nz.is_empty() {
        log(&format!(
            "keyblades: save+{:#x}..={:#x} is all zero apart from our bytes (save+{:#x} = {ours:#x}; MP block save+{:#x}..={:#x} v{}; items save+{:#x}..={:#x} v{})",
            kb::SPARE_FIRST,
            kb::SPARE_LAST,
            kb::EQUIP_OFFSET,
            crate::mp::BLOCK_FIRST,
            crate::mp::BLOCK_LAST,
            rd::<u8>(save + crate::mp::BLOCK_FIRST),
            crate::items::EXT_FIRST,
            crate::items::EXT_LAST,
            rd::<u8>(save + crate::items::EXT_FIRST)
        ));
    } else {
        let first: Vec<String> = nz.iter().take(16).map(|o| format!("+{o:#x}={:#x}", rd::<u8>(save + o))).collect();
        log(&format!(
            "keyblades: WARNING: save+{:#x}..={:#x} has {} non-zero bytes besides ours (save+{:#x} = {ours:#x}): {}",
            kb::SPARE_FIRST,
            kb::SPARE_LAST,
            nz.len(),
            kb::EQUIP_OFFSET,
            first.join(" ")
        ));
    }
}

/// One of our entry buttons, made as `0x1400E3DE0` makes Re:CoM's (mode 0, first time:
/// `C2DSeqCtrl`, kind 0xC, `+0x138 = 1`, sequence 4, its place, a `C2DFnt` label colour 0x80404040
/// font 0xF flags 0x40 attached at (0, 6)). Checked (disasm `0x1400E3E80..0x1400E3F8A`).
unsafe fn create_extra(camp: *mut u8, l: top::Layout, k: usize) {
    let Some(&idx) = top::extras(l).get(k) else { return };
    let Some(row) = top::entry(l, idx).and_then(|e| e.page()) else { return };
    let set = set_of(camp);
    let btn = func::<NewFn>(NEW)(0x170);
    if btn.is_null() {
        return;
    }
    func::<CtorFn>(SEQ_CTOR)(btn);
    put::<u8>(btn, 0x16E, 0xC);
    put::<u8>(btn, 0x138, 1);
    vfn::<SeqInitFn>(btn, 0x8)(btn, set, 4, 0.0, 0);
    place(btn, l, idx);
    let mut fnt = func::<NewFn>(NEW)(0xB0);
    if !fnt.is_null() {
        let colour = LABEL_NORMAL;
        fnt = func::<FntCtorFn>(FNT_CTOR)(fnt, 0, 0, &colour, 0xF, row.label().as_ptr(), 0x40);
    }
    vfn::<AttachFn>(btn, 0x60)(btn, set, fnt, 0.0, 6.0, 0.0);
    EXTRA_BTN[k].store(btn as usize, Ordering::Relaxed);
    EXTRA_FNT[k].store(fnt as usize, Ordering::Relaxed);
}

/// Deletes our entry buttons. Re:CoM's teardown deletes only its buttons; their text items go with
/// the buttons' sequences (Re:CoM's relabel path clears the old sequence's item pointer before
/// releasing it, `0x140246E90`). Assumption (inferred, not traced; the 8th entry did the same
/// since 0.8.0 without a report): the same holds for ours.
unsafe fn delete_extras() {
    for k in 0..top::MAX_EXTRA {
        let btn = EXTRA_BTN[k].swap(0, Ordering::Relaxed) as *mut u8;
        EXTRA_FNT[k].store(0, Ordering::Relaxed);
        delete(btn);
    }
}

fn extra(idx: i32) -> (*mut u8, *mut u8) {
    match top::extras(list_now()).iter().position(|&i| i == idx) {
        Some(k) => (EXTRA_BTN[k].load(Ordering::Relaxed) as *mut u8, EXTRA_FNT[k].load(Ordering::Relaxed) as *mut u8),
        None => (std::ptr::null_mut(), std::ptr::null_mut()),
    }
}

/// An entry button at camp index `idx`'s place in list `l` (`0x140248660`, as the entry build
/// places Re:CoM's).
unsafe fn place(btn: *mut u8, l: top::Layout, idx: i32) {
    let Some(s) = top::slot(l, idx) else { return };
    if btn.is_null() {
        return;
    }
    let (x, y) = top::entry_pos(l, s);
    let pos = Vec4([x, y, 0.0, 1.0]);
    func::<PosFn>(LAYOUT_POS)(btn, pos.0.as_ptr());
}

/// Re:CoM's 7 buttons at list `l`'s places.
unsafe fn place_recoms(camp: *mut u8, l: top::Layout) {
    for i in 0..top::RECOM_ENTRIES {
        place(at(camp, camp::BUTTONS + 8 * i as usize), l, i);
    }
    MOVED.store(top::order(l).is_some(), Ordering::Relaxed);
}

/// Re:CoM's relabel of one entry button (`0x1400E3DE0` mode 0 with a new level,
/// `0x1400E3FF3..0x1400E40C4`): old label deleted, a new `C2DFnt` (colour 0x80404040, font 0xF,
/// 0x40) stored in `camp+0x1F0[i]` and put in the button's slot 1 with its vtable
/// `+0x68(btn, 1, set, fnt)`. Checked (disasm).
unsafe fn relabel_one(camp: *mut u8, i: usize, text: *const u8) {
    let btn: *mut u8 = at(camp, camp::BUTTONS + 8 * i);
    if btn.is_null() || text.is_null() {
        return;
    }
    delete(at(camp, camp::LABELS + 8 * i));
    put::<usize>(camp, camp::LABELS + 8 * i, 0);
    let mut fnt = func::<NewFn>(NEW)(0xB0);
    if !fnt.is_null() {
        let colour = LABEL_NORMAL;
        fnt = func::<FntCtorFn>(FNT_CTOR)(fnt, 0, 0, &colour, 0xF, text, 0x40);
    }
    put::<usize>(camp, camp::LABELS + 8 * i, fnt as usize);
    vfn::<ReplaceFn>(btn, 0x68)(btn, 1, set_of(camp), fnt);
}

/// Puts the top list `l` in place after Re:CoM's entry build (mode 0, level 0): our two buttons
/// made or deleted, entry 0's label ("Items" at KH1's, Re:CoM's own otherwise), every button at its
/// place. `relabelled`: Re:CoM's build just made or relabelled its 7 labels.
unsafe fn apply_layout(camp: *mut u8, l: top::Layout, relabelled: bool) {
    if LIST_CAMP.load(Ordering::Relaxed) != camp as usize {
        // A camp we never saw torn down: forget its buttons (not ours to delete).
        for k in 0..top::MAX_EXTRA {
            EXTRA_BTN[k].store(0, Ordering::Relaxed);
            EXTRA_FNT[k].store(0, Ordering::Relaxed);
        }
        MOVED.store(false, Ordering::Relaxed);
        CONFIRMED.store(-1, Ordering::Relaxed);
        LIST_CAMP.store(camp as usize, Ordering::Relaxed);
        LAYOUT_LOGGED.store(0xFF, Ordering::Relaxed);
    }
    if relabelled {
        ITEMS_ON.store(false, Ordering::Relaxed);
    }
    let prev_list = LIST.swap(layout_code(l), Ordering::Relaxed);
    if top::order(l).is_some() {
        // The KH1 list (Sora) or Riku's (fork: Customize in entry 0's place, Costumes before
        // Settings): our buttons made (or shown again), entry 0 relabelled, every button placed.
        let n = top::extras(l).len();
        if prev_list != layout_code(l) || (0..top::MAX_EXTRA).any(|k| (k >= n) != EXTRA_BTN[k].load(Ordering::Relaxed).eq(&0)) {
            // Another reordered list's buttons (or a missing one): made again for this one.
            delete_extras();
            ITEMS_ON.store(false, Ordering::Relaxed);
        }
        for k in 0..n {
            let btn = EXTRA_BTN[k].load(Ordering::Relaxed) as *mut u8;
            if btn.is_null() {
                create_extra(camp, l, k);
            } else {
                // Re:CoM's mode 0 on a built list: animation 4 (appear) for every button.
                anim(btn, set_of(camp), 4);
            }
        }
        match l {
            // Riku's at Combat = Re:CoM and Sora's (0.170.0, Costumes before Settings): Review Decks
            // stays Re:CoM's (its own label back if ours).
            top::Layout::RikuDecks | top::Layout::RikuPlain | top::Layout::Settings => {
                if ITEMS_ON.swap(false, Ordering::Relaxed) {
                    relabel_one(camp, top::DECKS as usize, message(top::RECOM_LABELS[0]));
                }
            }
            _ => {
                let first = sub::Row::Items;
                if !ITEMS_ON.swap(true, Ordering::Relaxed) {
                    relabel_one(camp, top::DECKS as usize, first.label().as_ptr());
                }
            }
        }
        place_recoms(camp, l);
    } else {
        delete_extras();
        if ITEMS_ON.swap(false, Ordering::Relaxed) {
            // Re:CoM's own label for its entry 0 (Sora's story: `0x1407C1000[0]`).
            relabel_one(camp, top::DECKS as usize, message(top::RECOM_LABELS[0]));
        }
        if MOVED.load(Ordering::Relaxed) {
            place_recoms(camp, l);
        }
        // Our indices exist only at KH1's list.
        if at::<i32>(camp, camp::CURSOR) >= top::RECOM_ENTRIES {
            put::<i32>(camp, camp::CURSOR, top::SETTINGS);
        }
    }
    let code = layout_code(l);
    if LAYOUT_LOGGED.swap(code, Ordering::Relaxed) != code {
        log(match l {
            top::Layout::Kh1 => "camp: KH1's list: Items, Abilities, Customize, Re:CoM's World Map .. Journal, Costumes, Settings (10 entries, y 88 + 21.3 each); Settings opens Game Settings, Combat, Movement, Effects & Sound, Camera",
            top::Layout::Settings => "camp: Sora's list at Combat = Re:CoM: Re:CoM's seven and Costumes before Settings (8 entries); Settings opens Game Settings, Combat, Movement, Effects & Sound, Camera",
            top::Layout::Recom => "camp: Re:CoM's list, untouched (Riku's story or the map-only camp)",
            top::Layout::RikuDecks => "camp: Riku's list at Combat = Re:CoM: Re:CoM's Review Decks .. D Report, Costumes, Settings (8 entries); Settings opens Game Settings, Combat, Movement, Camera",
            top::Layout::RikuPlain => "camp: Riku's list (riku_kh1_movement off): Re:CoM's seven and Costumes before Settings (8 entries); Settings is Re:CoM's page",
            top::Layout::Riku => "camp: Riku's list at Combat = KH1: Items in Review Decks' place, Abilities, Customize, Re:CoM's World Map .. D Report, Costumes, Settings (10 entries, y 88 + 21.3 each); Settings opens Game Settings, Combat, Movement, Camera",
        });
    }
}

/// Hook on `0x1400E3DE0(camp, mode, level)` (the entry build): after Re:CoM's own work, ours.
pub unsafe extern "win64" fn entry_build(camp: *mut u8, mode: i32, level: u64) -> u64 {
    let o: BuildFn = orig(&ORIG_BUILD);
    if camp.is_null() {
        return o(camp, mode, level);
    }
    // Re:CoM (re)makes its 7 labels in mode 0 when it makes its buttons or the level changes.
    let relabels = mode == 0 && (at::<usize>(camp, camp::BUTTONS) == 0 || at::<i32>(camp, camp::LEVEL) != level as i32);
    let r = o(camp, mode, level);
    spare_check();
    if at::<i32>(camp, camp::LEVEL) != 0 {
        // Deck submenu or our Settings list: ours stay as Re:CoM leaves its unused buttons (faded
        // by the confirm).
        return r;
    }
    match mode {
        0 => apply_layout(camp, layout_now(camp), relabels),
        1 | 2 if top::order(layout_of(camp)).is_some() => {
            for k in 0..top::MAX_EXTRA {
                anim(EXTRA_BTN[k].load(Ordering::Relaxed) as *mut u8, set_of(camp), if mode == 1 { 5 } else { 6 });
            }
        }
        _ => {}
    }
    r
}

/// The button and label of camp index `i` (ours at 7 / 8).
unsafe fn entry_parts(camp: *mut u8, i: i32) -> (*mut u8, *mut u8) {
    if top::extras(list_now()).contains(&i) {
        return extra(i);
    }
    if !(0..top::RECOM_ENTRIES).contains(&i) {
        return (std::ptr::null_mut(), std::ptr::null_mut());
    }
    (at(camp, camp::BUTTONS + 8 * i as usize), at(camp, camp::LABELS + 8 * i as usize))
}

/// The help line of camp index `i` at the KH1 list: ours for our entries, Re:CoM's message for its.
unsafe fn kh1_help(camp: *mut u8, i: i32) -> *const u8 {
    match top::entry(list_now(), i) {
        Some(top::Entry::Recom(r)) => top::recom_help(story(camp), r).map_or(std::ptr::null(), |id| message(id)),
        Some(e) => e.page().and_then(|r| r.help()).map_or(std::ptr::null(), |t| t.as_ptr()),
        None => std::ptr::null(),
    }
}

/// `0x1400DEA10`'s work at the KH1 list, for every entry: the old entry back to animation 5, label
/// 0x80404040, priority 0xC, help cleared; the new one (when `show`) animation 3, label 0x80C0C0C0,
/// priority 0xD, the highlight bar (`0x1400E07D0`'s work) and the glove's target (`0x1400E3D30`'s)
/// at its slot's place, the cursor index and its help (Re:CoM's message for its entries).
unsafe fn cursor_set_kh1(camp: *mut u8, old: i32, new: i32, show: bool) -> u64 {
    let set = set_of(camp);
    let (b, l) = entry_parts(camp, old);
    if !b.is_null() {
        anim(b, set, 5);
        if !l.is_null() {
            put::<u32>(l, 0x34, LABEL_NORMAL);
        }
        func::<SprIntFn>(PRIORITY)(b, 0xC);
    }
    set_help(camp, rd::<*const u8>(va(base(), HELP_CLEAR)));
    if show {
        let (b, l) = entry_parts(camp, new);
        if !b.is_null() {
            anim(b, set, 3);
            if !l.is_null() {
                put::<u32>(l, 0x34, LABEL_HIGHLIGHT);
            }
            func::<SprIntFn>(PRIORITY)(b, 0xD);
        }
        let s = top::slot(list_now(), new).unwrap_or(0);
        highlight_bar(camp, s);
        glove_target(camp, new, s);
        put::<i32>(camp, camp::CURSOR, new);
        set_help(camp, kh1_help(camp, new));
    }
    1
}

/// `0x1400E07D0(camp, i ≥ 0)` with slot `s`'s place: the bar (`camp+0x258`) to (48, y) and shown.
/// Checked (disasm `0x1400E07D0..0x1400E0866`).
unsafe fn highlight_bar(camp: *mut u8, s: i32) {
    let bar: *mut u8 = at(camp, camp::BAR);
    if bar.is_null() {
        return;
    }
    let (x, y) = top::bar_pos(list_now(), s);
    let pos = Vec4([x, y, 0.0, 1.0]);
    func::<PosFn>(LAYOUT_POS)(bar, pos.0.as_ptr());
    func::<SprIntFn>(VISIBLE)(bar, 1);
}

/// `0x1400E3D30(camp, i ≥ 0)` with slot `s`'s place: the glide target `+0xF50` = (150, y, 0, 1);
/// when `i` is already the cursor the glove is there (`+0xF40`); the glide's start `+0xF60` =
/// `+0xF40`; `+0xCC` (8 bytes) = 1 (gliding, time 0); the glove (`camp+0x260`) shown. Checked
/// (disasm `0x1400E3D30..0x1400E3DC5`).
unsafe fn glove_target(camp: *mut u8, i: i32, s: i32) {
    let (x, y) = top::glove_pos(list_now(), s);
    let t = [x, y, 0.0, 1.0];
    put::<[f32; 4]>(camp, camp::GLIDE_TARGET, t);
    if i == at::<i32>(camp, camp::CURSOR) {
        put::<[f32; 4]>(camp, camp::GLIDE_NOW, t);
    }
    put::<[f32; 4]>(camp, camp::GLIDE_FROM, at::<[f32; 4]>(camp, camp::GLIDE_NOW));
    put::<u64>(camp, camp::GLIDING, 1);
    let glove: *mut u8 = at(camp, camp::GLOVE);
    if !glove.is_null() {
        func::<SprIntFn>(VISIBLE)(glove, 1);
    }
}

/// Hook on `0x1400DEA10(camp, index, show)` (the camp's cursor set). At the KH1 list's top level it
/// does Re:CoM's work for nine entries at their places; elsewhere it is Re:CoM's.
pub unsafe extern "win64" fn cursor_set(camp: *mut u8, idx: i32, show: i32) -> u64 {
    let o: CursorSetFn = orig(&ORIG_CURSOR);
    let l = layout_of(camp);
    if top::order(l).is_none() || at::<i32>(camp, camp::LEVEL) != 0 {
        return o(camp, idx, show);
    }
    let redirect = REDIRECT.load(Ordering::Relaxed);
    let mut new = if show != 0 && redirect >= 0 { redirect } else { idx };
    if top::slot(l, new).is_none() {
        // Re:CoM's wrap over the list (below 0: the last on screen; past the end: the first).
        new = top::at_slot(l, if new < 0 { top::count(l) - 1 } else { 0 }).unwrap_or(0);
    }
    let old = at::<i32>(camp, camp::CURSOR);
    cursor_set_kh1(camp, old, new, show != 0)
}

/// The top list's pad for this frame, read as state 0xF reads it (all pure reads).
unsafe fn top_pad() -> top::Pad {
    let rep = func::<PadFn>(PAD_REPEAT);
    let fresh = func::<PadFn>(PAD_FRESH);
    top::Pad {
        start: fresh(0x800, 0) != 0,
        cancel: func::<PadBtnFn>(PAD_CANCEL)(0) & 0xFF != 0,
        confirm: func::<PadBtnFn>(PAD_CONFIRM)(0) & 0xFF != 0,
        up: rep(0x10_1000, 0) != 0,
        up_fresh: fresh(0x10_1000, 0) != 0,
        down: rep(0x20_4000, 0) != 0,
        down_fresh: fresh(0x20_4000, 0) != 0,
    }
}

/// Header item `idx` (0 the entry, 1 the Settings list's row: the breadcrumb) with our title:
/// `0x1400E4220` mode 1 (header present), with our text where Re:CoM puts `0x1400D3890`'s.
unsafe fn header_title(camp: *mut u8, idx: usize, title: &[u8]) {
    let set = set_of(camp);
    let hdr: *mut u8 = at(camp, camp::HEADER + 8 * idx);
    let title_at = camp::HEADER_TITLE + 8 * idx;
    if hdr.is_null() {
        // Re:CoM makes both items at camp open (state 3); not seen missing. Its own call, then our
        // text on the new item. Assumption: the item reads its text pointer when drawn.
        func::<HeaderFn>(HEADER)(camp, 1, idx as u32);
        let t: *mut u8 = at(camp, title_at);
        if !t.is_null() {
            func::<TextFn>(TEXT_SET)(t, title.as_ptr());
        }
        log("camp: header was missing at confirm; made by Re:CoM, title set afterwards");
        return;
    }
    anim(hdr, set, 7);
    delete(at(camp, title_at));
    put::<usize>(camp, title_at, 0);
    let mut fnt = func::<NewFn>(NEW)(0xB0);
    if !fnt.is_null() {
        let colour: u32 = at(camp, camp::TEXT_COLOUR);
        fnt = func::<FntCtorFn>(FNT_CTOR)(fnt, 0, 0, &colour, 0xF, title.as_ptr(), 0x40);
    }
    put::<usize>(camp, title_at, fnt as usize);
    vfn::<ReplaceFn>(hdr, 0x68)(hdr, 1, set, fnt);
    func::<SprIntFn>(VISIBLE)(hdr, 1);
    put::<i32>(camp, camp::HEADER_MODE + 4 * idx, 2);
}

/// Re:CoM's confirm sequence for an entry with a page (state 0xF, `0x1400DAB0A..0x1400DACAC`, as
/// for its entries 2 / 4 / 5 / 6), for one of ours at camp index `idx`, with our header title.
/// Re:CoM's own update ran this frame (its confirm skipped: index > 6) and already eased the glove
/// (`0x1400DE100`, which no step here changes the target of).
unsafe fn confirm_page(camp: *mut u8, idx: i32, row: sub::Row) {
    let set = set_of(camp);
    func::<GuideFn>(GUIDE)(0, 0);
    func::<VoidFn>(CONFIRM_FLASH)(camp);
    sound(2);
    cursor_set_kh1(camp, idx, idx, false);
    let n = at::<i32>(camp, camp::COUNT).clamp(0, top::RECOM_ENTRIES);
    for i in 0..n {
        anim(at(camp, camp::BUTTONS + 8 * i as usize), set, 6);
    }
    for k in 0..top::MAX_EXTRA {
        anim(EXTRA_BTN[k].load(Ordering::Relaxed) as *mut u8, set, 6);
    }
    header_title(camp, 0, row.label());
    let hdr: *mut u8 = at(camp, camp::HEADER);
    if !hdr.is_null() {
        func::<SprIntFn>(VISIBLE)(hdr, 1);
    }
    anim(at(camp, camp::PANEL_140), set, 0xC);
    func::<CampIntFn>(CAMP_PANEL)(camp, 2, 0);
    // Step 9: the main panel's layer 2 hidden, as for Re:CoM's page entries; state 0x4C shows it.
    main_layer2(camp, false);
    put::<i32>(camp, camp::STATE, 0x11);
    put::<i32>(camp, camp::SAVED, idx);
    CONFIRMED.store(idx, Ordering::Relaxed);
    func::<CampIntFn>(SIDE_PANEL)(camp, 4, 0);
}

/// Hook on `0x1400D9C80` (the camp update). Re:CoM's update always runs; at the KH1 list it adds
/// what Re:CoM's 7-entry code can't do for nine, and at Sora's lists the Settings list keeps the
/// main panel's layer 2 (as Re:CoM's Deck submenu does).
pub unsafe extern "win64" fn camp_update(camp: *mut u8) {
    let o: VoidFn = orig(&ORIG_UPDATE);
    let l = layout_of(camp);
    if l == top::Layout::Recom {
        return o(camp);
    }
    let state = at::<i32>(camp, camp::STATE);
    let cursor = at::<i32>(camp, camp::CURSOR);
    // A list with an order of its own: Sora's KH1 list, Riku's (fork).
    let kh1 = top::order(l).is_some();
    match state {
        // Re:CoM gives every entry animation 5 once button 0's animation has ended (then, in 0xD,
        // re-highlights the cursor's entry).
        7 | 0xD if kh1 => {
            let b0: *mut u8 = at(camp, camp::BUTTONS);
            let was_ended = ended(b0);
            o(camp);
            if was_ended {
                for (k, &i) in top::extras(l).iter().enumerate() {
                    if !(state == 0xD && at::<i32>(camp, camp::CURSOR) == i) {
                        anim(EXTRA_BTN[k].load(Ordering::Relaxed) as *mut u8, set_of(camp), 5);
                    }
                }
            }
        }
        0xF => {
            let pad = top_pad();
            let fix = if kh1 { top::list_fix(l, cursor, pad) } else { top::Fix::None };
            if let top::Fix::Confirm(e) = fix {
                // Re:CoM's confirm must not run for ours (entry 0 would open its deck submenu):
                // an index past 6 makes it skip (`0x1400DAABD`), the frame's other work stays.
                if cursor == top::DECKS {
                    put::<i32>(camp, camp::CURSOR, top::ABILITIES);
                }
                o(camp);
                if cursor == top::DECKS && at::<i32>(camp, camp::CURSOR) == top::ABILITIES {
                    put::<i32>(camp, camp::CURSOR, top::DECKS);
                }
                if at::<i32>(camp, camp::STATE) == 0xF && at::<i32>(camp, camp::CURSOR) == cursor {
                    if let Some(row) = e.page() {
                        confirm_page(camp, cursor, row);
                    }
                }
                return;
            }
            if kh1 {
                REDIRECT.store(top::list_move(l, cursor, pad).unwrap_or(-1), Ordering::Relaxed);
            }
            o(camp);
            REDIRECT.store(-1, Ordering::Relaxed);
            let now = at::<i32>(camp, camp::STATE);
            if now == 0xF && at::<i32>(camp, camp::CURSOR) == cursor {
                if let top::Fix::Move(t) = fix {
                    cursor_set_kh1(camp, cursor, t, true);
                    sound(1);
                    func::<VoidFn>(GLIDE_STEP)(camp);
                }
            } else if now == 0x10 || now == 0x11 {
                // Re:CoM confirmed one of its entries: step 5 (every button animation 6) for ours.
                CONFIRMED.store(-1, Ordering::Relaxed);
                if kh1 {
                    for k in 0..top::MAX_EXTRA {
                        anim(EXTRA_BTN[k].load(Ordering::Relaxed) as *mut u8, set_of(camp), 6);
                    }
                }
                if cursor == top::SETTINGS && top::settings_list(l) {
                    // Settings opens a list now: its step 9 undone the same frame (the Deck
                    // submenu keeps the layer; our list hides it when a page opens).
                    main_layer2(camp, true);
                }
            }
        }
        // State 0x11 reads a 7-byte table (`1D 16 41 1A 44 47 4A`): index 0 gives the deck
        // submenu 0x1D, 7 / 8 read past it. Our pages open through the Config states 0x4A..0x4C.
        0x11 if kh1 && ours_confirmed(cursor) => {
            o(camp);
            let now = at::<i32>(camp, camp::STATE);
            if now != 0x11 {
                if !ODD_STATE_LOGGED.swap(true, Ordering::Relaxed) {
                    log(&format!("camp: state 0x11 left to {now:#x} for our entry {cursor}; its page opens in the Config states 0x4A..0x4C"));
                }
                put::<i32>(camp, camp::STATE, 0x4A);
            }
        }
        _ => o(camp),
    }
}

/// Hook on `0x1400D7C20` (the camp teardown, both dtors and state 0x50): ours go with Re:CoM's.
pub unsafe extern "win64" fn teardown(camp: *mut u8) {
    let o: VoidFn = orig(&ORIG_TEARDOWN);
    if !camp.is_null() {
        if let Ok(mut g) = OPEN.lock() {
            if g.as_ref().is_some_and(|s| s.camp() == camp as usize) {
                if let Some(s) = g.take() {
                    s.delete_all();
                }
            }
        }
        if LIST_CAMP.load(Ordering::Relaxed) == camp as usize {
            delete_extras();
            LIST_CAMP.store(0, Ordering::Relaxed);
            LIST.store(0, Ordering::Relaxed);
            ITEMS_ON.store(false, Ordering::Relaxed);
            MOVED.store(false, Ordering::Relaxed);
            CONFIRMED.store(-1, Ordering::Relaxed);
        }
    }
    o(camp)
}

// ---------------------------------------------------------------------------------------------
// The Settings list (owner, 2026-11-04)

/// Where the list is (our state; never in camp fields Re:CoM's pages use).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SubState {
    /// Re:CoM's state 0x1D: the buttons play animation 4 (appear); when button 0's ends, idle
    /// (entry build mode 1) and the cursor set on the row.
    Opening,
    /// Re:CoM's state 0x22.
    Input,
    /// After a confirm: the buttons play animation 6 (fade); when button 0's ends, the page opens.
    ToPage,
    InPage,
    /// After a cancel: the buttons fade; when button 0's ends, back to the top list (−1).
    Closing,
}

/// The page open in the list: one of our settings pages (Re:CoM's Config screen with our rows,
/// `super::menu::SettingsPage`) or Re:CoM's own Config page.
enum Open {
    Settings(super::menu::SettingsPage),
    /// Re:CoM's Config input `0x1400D3EB0` (the original, past our hook), run as its states 0x4A /
    /// 0x4B run it.
    Recom,
}

impl Open {
    unsafe fn step(&mut self, camp: *mut u8) -> i32 {
        match self {
            Open::Settings(p) => p.step(),
            Open::Recom => super::menu::recom_config(camp) as i32,
        }
    }
}

/// The Settings list and the page open in it.
struct Sub {
    camp: usize,
    /// The list's rows (`sub::settings_rows`: Riku's story has fewer).
    rows: &'static [sub::Row],
    state: SubState,
    row: i32,
    page: Option<Open>,
}

/// Re:CoM's relabel of its entry buttons (`0x1400E3DE0` mode 0 with a new level), with our texts.
unsafe fn relabel(camp: *mut u8, labels: &[&'static [u8]]) {
    for (i, text) in labels.iter().enumerate().take(sub::MAX_ROWS) {
        relabel_one(camp, i, text.as_ptr());
    }
    ITEMS_ON.store(false, Ordering::Relaxed);
}

/// Re:CoM's entry build (the original, past our wrapper): mode 0 at the current level only plays
/// animation 4 on the `count` buttons; mode 1 / 2 animation 5 / 6.
unsafe fn build(camp: *mut u8, mode: i32, level: u64) {
    let o: BuildFn = orig(&ORIG_BUILD);
    o(camp, mode, level);
}

/// Re:CoM's cursor set (the original: at level 1 our wrapper passes it through anyway).
unsafe fn recom_cursor(camp: *mut u8, idx: i32, show: bool) {
    let o: CursorSetFn = orig(&ORIG_CURSOR);
    o(camp, idx, show as i32);
}

unsafe fn button0_ended(camp: *mut u8) -> bool {
    ended(at(camp, camp::BUTTONS))
}

/// Main panel layer 2: Re:CoM hides it when a page opens (state 0xF step 9 for entries 2 / 4..6)
/// and shows it when the page is done (state 0x4C); its Deck submenu keeps it.
unsafe fn main_layer2(camp: *mut u8, show: bool) {
    let main: *mut u8 = at(camp, camp::MAIN_PANEL);
    if !main.is_null() {
        func::<SlotVisFn>(SLOT_VISIBLE)(main, 2, show as u32);
    }
}


impl Sub {
    fn p(&self) -> *mut u8 {
        self.camp as *mut u8
    }

    /// The list's first call (state 0x4A, after Re:CoM's state 0x11 saw the top buttons fade):
    /// what Re:CoM's state 0x10 does for the Deck at level 1, with our rows, on Re:CoM's places
    /// (the KH1 list's go back when the top list is rebuilt, state 0x4C).
    unsafe fn open(camp: *mut u8) -> Sub {
        if MOVED.load(Ordering::Relaxed) {
            place_recoms(camp, top::Layout::Settings);
        }
        let rows = sub::settings_rows(story(camp));
        let labels: Vec<&'static [u8]> = rows.iter().map(|r| r.label()).collect();
        relabel(camp, &labels);
        put::<i32>(camp, camp::COUNT, rows.len() as i32);
        // Re:CoM's Deck submenu level; the cursor set and entry build read it (`< 2`: help).
        put::<i32>(camp, camp::LEVEL, 1);
        // Re:CoM's cursor set unhighlights the button at `camp+0xB8` first, which must be one of
        // the list's.
        put::<i32>(camp, camp::CURSOR, 0);
        build(camp, 0, 1);
        log(&format!("camp: Settings list opened ({rows:?})"));
        Sub { camp: camp as usize, rows, state: SubState::Opening, row: 0, page: None }
    }

    unsafe fn help(&self) {
        if let Some(r) = self.rows.get(self.row as usize) {
            match (r.help(), r.recom_help()) {
                (Some(t), _) => set_help(self.p(), t.as_ptr()),
                (None, Some(id)) => set_help(self.p(), message(id)),
                (None, None) => {}
            }
        }
    }

    /// One frame (states 0x4A / 0x4B). ≥ 0 running, < 0 back to the top list.
    unsafe fn step(&mut self) -> i32 {
        let camp = self.p();
        match self.state {
            SubState::Opening => {
                if button0_ended(camp) {
                    build(camp, 1, 0);
                    put::<i32>(camp, camp::CURSOR, self.row);
                    recom_cursor(camp, self.row, true);
                    self.help();
                    self.state = SubState::Input;
                }
            }
            SubState::Input => {
                match sub::step(self.row, self.rows.len(), top_pad()) {
                    sub::Act::Close => {
                        recom_cursor(camp, self.row, false);
                        build(camp, 2, 0);
                        sound(4);
                        self.state = SubState::Closing;
                    }
                    sub::Act::Open(i) => {
                        // State 0x22's confirm: sound 2, header item 1 (the breadcrumb) with the
                        // row's title, buttons fade, highlight bar and glove hidden; Re:CoM's page
                        // entries' confirm also hides the main panel's layer 2.
                        sound(2);
                        header_title(camp, 1, self.rows[i].label());
                        build(camp, 2, 0);
                        func::<CampIntFn>(HIGHLIGHT_BAR)(camp, -1, 0);
                        func::<CampIntFn>(GLIDE_TARGET)(camp, -1, 0);
                        main_layer2(camp, false);
                        self.state = SubState::ToPage;
                    }
                    sub::Act::Move(i) => {
                        recom_cursor(camp, i, true);
                        sound(1);
                        self.row = i;
                        self.help();
                    }
                    sub::Act::None => {}
                }
                func::<VoidFn>(GLIDE_STEP)(camp);
            }
            SubState::ToPage => {
                if button0_ended(camp) {
                    let row = self.rows.get(self.row as usize).copied().unwrap_or(sub::Row::GameSettings);
                    self.page = Some(match row.settings_page() {
                        Some(sp) => Open::Settings(super::menu::SettingsPage::open(camp, sp)),
                        None => {
                            // Re:CoM's state 0x4A: its page's sub-state 0 (open), then its input
                            // function every frame until it returns < 0.
                            put::<i32>(camp, camp::PAGE_STATE, 0);
                            log("camp: Game Settings: Re:CoM's Config page");
                            Open::Recom
                        }
                    });
                    if let Some(p @ Open::Recom) = self.page.as_mut() {
                        // Re:CoM's state 0x4A calls it once (open) on this frame.
                        if p.step(camp) < 0 {
                            self.page = None;
                        }
                    }
                    self.state = SubState::InPage;
                }
            }
            SubState::InPage => {
                let r = self.page.as_mut().map_or(-1, |p| p.step(camp));
                if r < 0 {
                    // Re:CoM's return from a Deck page (states 0x34 / 0x35): header item 1
                    // closes, the buttons appear again (entry build mode 0 at the same level),
                    // then idle and the cursor on the row (`Opening`).
                    self.page = None;
                    func::<HeaderFn>(HEADER)(camp, 4, 1);
                    build(camp, 0, 1);
                    main_layer2(camp, true);
                    self.state = SubState::Opening;
                }
            }
            SubState::Closing => {
                if button0_ended(camp) {
                    // Back to the top list: Re:CoM's state 0x4B closes header item 0, 0x4C
                    // rebuilds the list (level 1 → 0, `0x1400E3DE0`: our wrapper puts the list
                    // for the Combat setting now in place) and 0xD highlights Settings.
                    put::<i32>(camp, camp::CURSOR, top::SETTINGS);
                    log("camp: Settings list closed");
                    return -1;
                }
            }
        }
        0
    }

    /// At the camp's teardown: a settings page's objects are the Config screen's, which Re:CoM's
    /// teardown `0x1400D7C20` deletes itself (`camp+0xB30`, the 21 slots, `+0xBE0`), as it does
    /// for its own Config page. Checked (decomp).
    unsafe fn delete_all(&self) {}
}

/// What runs in the Config states for our entries: the Settings list, or one of our KH1 list
/// entries' pages.
enum Session {
    List(Sub),
    Page(Page),
    /// A top entry that opens one of our settings pages (Riku's Costumes, fork).
    Settings(super::menu::SettingsPage),
}

impl Session {
    fn camp(&self) -> usize {
        match self {
            Session::List(s) => s.camp,
            Session::Page(p) => p.camp,
            Session::Settings(p) => p.camp(),
        }
    }

    unsafe fn delete_all(&self) {
        match self {
            Session::List(s) => s.delete_all(),
            Session::Page(p) => p.delete_all(),
            // Its items are in Re:CoM's Config slots: Re:CoM's teardown deletes them.
            Session::Settings(_) => {}
        }
    }
}

/// The open session of the camp.
static OPEN: Mutex<Option<Session>> = Mutex::new(None);

// ---------------------------------------------------------------------------------------------
// The pages

/// 0.198.18: Riku's Dark Mode shortcuts on his Customize page (`crate::riku::dark_block`): its slots, Dark Riku's
/// learned sleights and its picker (his spells, those sleights, the King's two Dark Riku can start).
struct DarkSet {
    slots: [u8; 3],
    sleights: Vec<u16>,
    picks: Vec<customize::Pick>,
}

/// What a page lists.
enum Content {
    /// The Keyblade list, shown from the Items page's Keyblade slot (that page's content waits in
    /// [`Page::under`]).
    Keyblades {
        cards: Vec<u16>,
        equipped: u16,
    },
    /// KH1's Customize (`crate::customize`): the slot list, or (`picking` = the slot) the picker.
    /// `sleights`: the equipped sleight abilities when the page opened (M12 step 6).
    Customize {
        levels: [u8; 7],
        slots: [u8; 3],
        sleights: Vec<u16>,
        picks: Vec<customize::Pick>,
        picking: Option<usize>,
        /// 0.198.18: Riku's Dark Mode set (slot indices 3..5), once Re:CoM has taught him D Mode (card 0x6D).
        dark: Option<Box<DarkSet>>,
        /// 0.198.18: the list shows the Dark Mode set instead of Riku's own (Left / Right, as Re:CoM's Sleights page
        /// switches its tabs, `0x1400EB7E0` state 4). The list keeps KH1's three rows either way.
        dark_shown: bool,
    },
    /// M8 stage 8a.2: KH1's pause-menu Items screen (`crate::items_page`) on Sora's inventory in
    /// our save block (`win::items`). `inv`, `ctx` and `rows` are read again after every change.
    Items {
        screen: ip::Screen,
        /// KH1's message after a refusal, on the help line until the next move.
        msg: Option<ip::Msg>,
        inv: crate::items::Inventory,
        ctx: ip::Ctx,
        rows: Vec<ip::Row>,
    },
    /// M9 stage 9b: KH1's ability screen (`crate::abilities::page`) on the ability state in our
    /// save block (`win::abilities`). `state`, `level` and `rows` are read again after every change.
    Abilities {
        screen: ap::Screen,
        state: crate::abilities::State,
        level: u32,
        rows: Vec<ap::Row>,
        /// KH1's message after a refusal (not enough AP), on the help line until the next move.
        msg: Option<&'static [u8]>,
        /// The ability state is live (the KH1 setting, Sora's story, a save).
        live: bool,
    },
}

/// KH1's one picker cursor for every slot (`0x142E9A040`), kept for the session.
static LAST_PICK: AtomicUsize = AtomicUsize::new(0);

/// A page's objects and state (never in camp fields).
struct Page {
    camp: usize,
    sub: u8,
    stats_panel: usize,
    list_panel: usize,
    cursor: usize,
    bar: usize,
    rows: [usize; 8],
    stat_rows: [usize; 3],
    list: list::List,
    /// Each open window's scroll (owner, 2026-11-05: a confirm keeps the list where it was).
    scroll: list::Scroll,
    content: Content,
    /// The Items screen (and its list as shown) while its Keyblade list is shown; cancel there
    /// brings it back.
    under: Option<(Content, list::List)>,
    /// Riku's Customize (0.198.17): Re:CoM's Sleights List on Square; its run (`SleightsList`).
    riku_list: bool,
    sleights: SleightsList,
}

/// Re:CoM's Sleights List over our page: idle, running (`0x1400EBE30` each frame until it answers 1),
/// draining (one more call, which lowers the camp's input lock, as Re:CoM's page keeps calling it).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SleightsList {
    Idle,
    Running,
    Draining,
}

/// An item's name: KH1's word as the command menu's Items list shows it (`kh1-command-menu.json`),
/// else our English name; 0 is KH1's empty slot.
fn item_word(item: u8) -> Vec<u8> {
    if item == 0 {
        return ip::text::EMPTY.to_vec();
    }
    crate::items::data(item).and_then(|d| super::command_menu::word(d.name)).unwrap_or_else(|| ip::text::english(item).to_vec())
}

/// The words of an Items row.
fn item_label(l: ip::Label) -> Vec<u8> {
    use ip::{Command as C, Label as L, Owner as O};
    let t: &[u8] = match l {
        // SAFETY: Re:CoM's card-name call (reads its message tables), as the Keyblade list's rows.
        L::Keyblade if riku_page() => return unsafe { blade_name(riku_blade()) },
        L::Keyblade => return unsafe { card_name(super::stats::equipped_card()) },
        L::Owner(O::Sora) if riku_page() => b"Riku\0",
        L::Owner(O::Sora) => ip::text::SORA,
        L::Owner(O::Stock) => ip::text::STOCK,
        L::Slot(i) => return item_word(i),
        L::Stocked(i, n) => return ip::stocked_text(&item_word(i), n),
        L::Command(C::Use, _) => ip::text::USE,
        L::Command(C::Move, _) => ip::text::MOVE,
        L::Command(C::EquipRemove, true) => ip::text::REMOVE,
        L::Command(C::EquipRemove, false) => ip::text::EQUIP,
        L::Command(C::Discard, _) => ip::text::DISCARD,
        L::Yes => ip::text::YES,
        L::No => ip::text::NO,
    };
    t.to_vec()
}

/// A spell's name at its level: KH1's word as the command menu shows it, else our English name.
fn spell_word(spell: u8, level: u8) -> Vec<u8> {
    super::command_menu::word(customize::name_id(spell, level)).unwrap_or_else(|| kb::c_text(crate::unlocks::level_name(spell, level.clamp(1, 3))))
}

/// A shortcut pick's name: a spell at its level, (M12 step 6) a sleight's name as the Sleights list
/// shows it (the command menu's word, else the Abilities page's English name), or "None".
fn pick_word(p: customize::Pick, levels: &[u8; 7]) -> Vec<u8> {
    match p {
        Some(cm::shortcut::Slot::Spell(s)) => spell_word(s, levels.get(s as usize).copied().unwrap_or(1)),
        Some(cm::shortcut::Slot::Sleight(id)) => super::command_menu::word(crate::sleights::word(id)).unwrap_or_else(|| kb::c_text(crate::sleights::name(id))),
        None => customize::text::NONE.to_vec(),
    }
}

impl Page {
    fn p(&self) -> *mut u8 {
        self.camp as *mut u8
    }

    /// Draw mode 6 of `0x1400E6DF0`: panels, rows, cursor, scroll bar (in Re:CoM's order).
    unsafe fn delete_all(&self) {
        delete(self.stats_panel as *mut u8);
        delete(self.list_panel as *mut u8);
        for r in self.stat_rows.iter().chain(self.rows.iter()) {
            delete(*r as *mut u8);
        }
        delete(self.cursor as *mut u8);
        delete(self.bar as *mut u8);
    }

    unsafe fn layout(&self, group: u32, elem: u64, flag: i32) -> Option<Vec4> {
        let lay: *const u8 = at(self.p(), camp::LAYOUT);
        if lay.is_null() {
            return None;
        }
        let mut v = Vec4::default();
        (func::<LayoutFn>(LAYOUT)(lay, group, elem, v.0.as_mut_ptr(), flag) != 0).then_some(v)
    }

    /// A `C2DSeqCtrl` panel at a layout position (draw mode 0).
    unsafe fn panel(&self, kind: u8, elem: u64, seq: u32) -> usize {
        let c = func::<NewFn>(NEW)(0x170);
        if c.is_null() {
            return 0;
        }
        func::<CtorFn>(SEQ_CTOR)(c);
        put::<u8>(c, 0x16E, kind);
        put::<u8>(c, 0x138, 1);
        let pos = self.layout(0x11, elem, 0).unwrap_or_default();
        func::<PosFn>(LAYOUT_POS)(c, pos.0.as_ptr());
        vfn::<SeqInitFn>(c, 0x8)(c, set_of(self.p()), seq, 0.0, 0);
        c as usize
    }

    /// Draw mode 0 for the page: the two panels, the cursor, the rows.
    unsafe fn open(camp: *mut u8, row: sub::Row) -> Page {
        let (content, list) = match row {
            sub::Row::Customize if story(camp) == kb::Story::Riku => {
                // Riku's (0.149.0): his sleights; his slots (the same save bytes). 0.187.0: and his unlocked spells.
                let levels = super::command_menu::riku_spell_levels();
                let slots = super::mp::riku_shortcuts().unwrap_or([0; 3]);
                // His six and (0.152.0) Dark Riku's, each once (a slot starts what the one in play takes).
                // 0.198.15: only those Re:CoM has taught (`riku_learned_sleights`).
                // 0.198.18: his own list only. Dark Riku's own sleights (0x5F..0x61, 0x65) are refused by Riku's own sleight
                // start (`0x140216AB0`, `crate::riku::SLEIGHTS`): they are offered in the Dark Mode set alone. A slot that
                // already holds one keeps its byte and shows greyed (`customize::unequipped`).
                let mut sleights: Vec<u16> = super::command_menu::riku_learned_sleights(false);
                // 0.150.0: the King's Party sleights too (0.198.16: once Re:CoM has taught them, B11).
                sleights.extend(super::party::learned_ids().into_iter().filter(|&id| crate::party::index(id).is_some_and(|i| crate::party::ALL[i].cards.contains(&crate::party::card::KING))));
                let picks = customize::picks(levels, &sleights);
                // 0.198.18: Dark Mode's own set once Re:CoM has taught him D Mode: its own learned test (`0x14023AB00`) on
                // card 0x6D, which Re:CoM gives in `WORLD00/2102` step 170 (the workspace's `recom-riku-sleights.md` § 1-2).
                // Development builds only (feature `dev`, `tools/dev-install.sh`): `all_unlock = on` shows it from
                // the start; a released DLL never does (fork owner: "não vai chegar de jeito nenhum isso para o pessoal").
                let d_mode = super::abilities::sleight_learned_fn()(D_MODE) & 0xFF != 0;
                let dark = (d_mode || (cfg!(feature = "dev") && super::settings().all_unlock)).then(|| {
                    let mut sleights = super::command_menu::riku_learned_sleights(true);
                    let slots = super::command_menu::riku_dark_shortcuts(&sleights);
                    sleights.extend(super::party::learned_ids().into_iter().filter(|id| crate::riku::DARK_KING.contains(id)));
                    let picks = customize::picks(levels, &sleights);
                    Box::new(DarkSet { slots, sleights, picks })
                });
                (Content::Customize { levels, slots, sleights, picks, picking: None, dark, dark_shown: false }, list::open_on(customize::SLOTS as i32, 0))
            }
            sub::Row::Customize => {
                let (levels, slots) = super::command_menu::spell_view();
                // 0.111.0: the Party sleights Re:CoM has taught can be shortcuts too.
                let mut sleights = super::abilities::equipped_sleights();
                sleights.extend(super::party::learned_ids());
                let picks = customize::picks(levels, &sleights);
                (Content::Customize { levels, slots, sleights, picks, picking: None, dark: None, dark_shown: false }, list::open_on(customize::SLOTS as i32, 0))
            }
            sub::Row::Items => {
                let content = Content::Items { screen: ip::Screen::new(), msg: None, inv: Default::default(), ctx: Default::default(), rows: Vec::new() };
                (content, list::open_on(2, 0))
            }
            sub::Row::Abilities | sub::Row::GameSettings | sub::Row::Combat | sub::Row::Movement | sub::Row::EffectsSound | sub::Row::Camera | sub::Row::Costumes | sub::Row::SoraCostumes => {
                // The settings rows open a settings page (`Open`), never a list page.
                let content = Content::Abilities { screen: ap::Screen::default(), state: Default::default(), level: 1, rows: Vec::new(), msg: None, live: false };
                (content, list::open_on(2, 0))
            }
        };
        let mut p = Page { camp: camp as usize, sub: 1, stats_panel: 0, list_panel: 0, cursor: 0, bar: 0, rows: [0; 8], stat_rows: [0; 3], list, scroll: list::Scroll::default(), content, under: None, riku_list: false, sleights: SleightsList::Idle };
        // 0.198.17: Riku's Customize shows Re:CoM's "Sleights" prompt bar (slot 0, his variant 9, as
        // the deck page's state 0x22 makes it) and opens Re:CoM's Sleights List on Square.
        if matches!(row, sub::Row::Customize) && story(camp) == kb::Story::Riku {
            p.riku_list = true;
            func::<PromptFn>(PROMPT_BAR)(camp, 1, 0, 9);
        }
        p.items_read();
        p.abilities_read();
        p.stats_panel = p.panel(0x10, 3, 0x125);
        p.list_panel = p.panel(0x11, 2, 0x14C);
        let c = func::<NewFn>(NEW)(0x1C0);
        if !c.is_null() {
            p.cursor = func::<CursorCtorFn>(CURSOR_CTOR)(c, 0x12, p.list_panel as *mut u8, 0x12D) as usize;
        }
        p.redraw();
        match &p.content {
            Content::Keyblades { .. } => {}
            Content::Customize { levels, slots, sleights, dark, .. } => log(&format!(
                "customize: page opened: spell levels {levels:?}, shortcut slots {slots:x?} (0 none, 1..=7 spell + 1, else a sleight's Re:CoM id), equipped sleights {sleights:x?}{}",
                dark.as_ref().map_or(String::new(), |d| format!("; Dark Mode slots {:x?}, Dark Riku's sleights {:x?}", d.slots, d.sleights))
            )),
            Content::Items { inv, ctx, .. } => log(&format!(
                "items page: opened: slots {:?} (of {}), stock {:?}, HP below max {}, MP below max {}{}",
                &inv.slots[..(inv.slot_count as usize).min(crate::items::MAX_SLOTS)],
                inv.slot_count,
                ip::stock_list(inv).iter().map(|&i| (i, inv.stock_of(i))).collect::<Vec<_>>(),
                ctx.hp_low,
                ctx.mp_low,
                if super::items::inventory().is_none() { " (KH1's items not live: nothing can change)" } else { "" }
            )),
            Content::Abilities { state, level, live, .. } => log(&format!(
                "abilities page: opened at Re:CoM level {level}: AP {}/{} left; unlocked {:?}; equipped {:?}{}",
                crate::abilities::ap_left(state.equipped, state.max_ap()),
                state.max_ap(),
                crate::abilities::names(state.unlocked),
                crate::abilities::names(state.equipped),
                if *live { "" } else { " (the ability state isn't live: nothing can change)" }
            )),
        }
        p
    }

    /// The Items page's Keyblade slot confirmed: the list shows the Keyblade list (the Keyblades
    /// page of before: the earned Keyblades, the equipped one marked, its numbers, the scroll bar),
    /// on the equipped one; the Items screen waits in `under`.
    unsafe fn open_keyblades(&mut self) {
        // 0.163.0: Riku's Keyblades (cosmetic: the Soul Eater, the Keyblade of Heart).
        let (cards, equipped) = if riku_page() { (vec![SOUL_EATER, RIKU_HEART], riku_blade()) } else { (kb::unlocked(super::stats::keyblade_cards(), |k| ever_obtained(k)), super::stats::equipped_card()) };
        let at_eq = cards.iter().position(|&c| c == equipped).unwrap_or(0) as i32;
        let items_list = std::mem::replace(&mut self.list, list::open_on(cards.len() as i32, at_eq));
        log(&format!("keyblades: list opened from the Items page: {} Keyblades listed {:x?}, equipped {:#x}", cards.len(), cards, equipped));
        self.under = Some((std::mem::replace(&mut self.content, Content::Keyblades { cards, equipped }), items_list));
        // The entry-4 page's scroll bar (`0x140112AB0` / `0x1401130C0`) on the list panel, as the
        // Keyblades page made it.
        let b = func::<NewFn>(NEW)(0x28);
        if !b.is_null() {
            let bar = func::<CtorFn>(BAR_CTOR)(b);
            let mut pos = Vec4([133.0, -56.0, 0.0, 1.0]);
            func::<BarSetupFn>(BAR_SETUP)(bar, set_of(self.p()), 0x14, va(base(), BAR_SEQ) as *const u16, pos.0.as_mut_ptr(), 157.0, self.list_panel as *mut u8);
            self.bar = bar as usize;
        }
        self.redraw();
        self.help();
    }

    /// Cancel in the Keyblade list: back to the Items screen (Sora's list, on the Keyblade slot,
    /// now naming the equipped one); the scroll bar deleted as the page's close deletes it.
    unsafe fn back_from_keyblades(&mut self) {
        let Some((under, items_list)) = self.under.take() else { return };
        self.content = under;
        // Sora's list as it was shown, so its re-read keeps its scroll.
        self.list = items_list;
        delete(self.bar as *mut u8);
        self.bar = 0;
        self.items_read();
        self.redraw();
        self.help();
    }

    /// The page's name (logs).
    fn kind(&self) -> &'static str {
        match &self.content {
            Content::Keyblades { .. } => "Keyblades",
            Content::Customize { picking: Some(_), .. } => "Customize picker",
            Content::Customize { .. } => "Customize",
            Content::Items { .. } => "Items",
            Content::Abilities { .. } => "Abilities",
        }
    }

    /// Customize: the slot index of list row `row` (0..2): 3..5 while Riku's Dark Mode set is shown.
    fn cz_index(&self, row: usize) -> usize {
        match &self.content {
            Content::Customize { dark: Some(_), dark_shown: true, .. } => row % customize::SLOTS + customize::SLOTS,
            _ => row,
        }
    }

    /// Customize: slot `idx`'s byte and the sleights its picker offers (3..5: Riku's Dark Mode set).
    fn cz_slot(&self, idx: usize) -> Option<(u8, &[u16])> {
        let Content::Customize { slots, sleights, dark, .. } = &self.content else { return None };
        match idx.checked_sub(customize::SLOTS) {
            None => slots.get(idx).map(|&b| (b, sleights.as_slice())),
            Some(j) => dark.as_ref().and_then(|d| d.slots.get(j).map(|&b| (b, d.sleights.as_slice()))),
        }
    }

    /// Customize: the picker of slot row `k`.
    fn cz_picks(&self, k: usize) -> &[customize::Pick] {
        match &self.content {
            Content::Customize { dark: Some(d), .. } if k >= customize::SLOTS => &d.picks,
            Content::Customize { picks, .. } => picks,
            _ => &[],
        }
    }

    /// 0.198.18: Left / Right on Riku's slot list with the Dark Mode set: the other set, its rows rebuilt, the cursor on
    /// the same row, sound 1; wrapping (two sets), as Re:CoM's Sleights page switches its tabs (`0x1400EB7E0` state 4:
    /// `0x140111A70(camp, 0x408000 / 0x802000)`, index ∓1 with wrap, sound 1, draw mode 10). Returns whether it switched.
    unsafe fn switch_set(&mut self, left: bool) -> bool {
        let Content::Customize { dark: Some(_), dark_shown, picking: None, .. } = &mut self.content else { return false };
        *dark_shown = !*dark_shown;
        let shown = *dark_shown;
        sound(1);
        self.redraw();
        self.help();
        log(&format!(
            "customize: Riku: {} set shown ({}); rows: Re:CoM's {}",
            if shown { "Dark Mode" } else { "Riku's" },
            if left { "Left" } else { "Right" },
            if shown { "row 0x12F (red plate, Dark Riku's wing)" } else { "blue magic row 0x133" }
        ));
        true
    }

    /// The text of list row `idx`.
    fn row_text(&self, idx: usize) -> Vec<u8> {
        match &self.content {
            // SAFETY: Re:CoM's card-name call (reads its message tables).
            Content::Keyblades { cards, .. } => unsafe { blade_name(cards[idx]) },
            Content::Customize { levels, picking: Some(k), .. } => pick_word(self.cz_picks(*k).get(idx).copied().flatten(), levels),
            // Owner, 2026-11-04: the bound spell's or sleight's name beside the button prompt
            // (the status panel's words); an empty slot shows the button alone.
            Content::Customize { levels, picking: None, .. } => {
                let glyphs = button_glyphs(ROW_FONT);
                let circle = super::command_menu::confirm_is_circle();
                // 0.198.18: Riku's Dark Mode set's rows say so ("Dark" + the button); an empty slot reads "None"
                // (`customize::text::slot_row`, KH1's `0x140302950`), on Sora's page too (this same code).
                let idx = self.cz_index(idx);
                let label = if idx >= customize::SLOTS { customize::text::dark_slot_label(idx - customize::SLOTS, glyphs, circle) } else { customize::text::slot_label(idx, glyphs, circle).to_vec() };
                let name = self.cz_slot(idx).and_then(|(b, _)| cm::shortcut::slot_of(b)).map(|p| pick_word(Some(p), levels));
                customize::text::slot_row(&label, name.as_deref(), glyphs)
            }
            Content::Items { rows, .. } => rows.get(idx).map_or_else(|| ip::text::NONE.to_vec(), |r| item_label(r.label)),
            Content::Abilities { rows, .. } => match rows.get(idx).map(|r| r.label) {
                Some(ap::Label::Owner(ap::Owner::Sora)) if riku_page() => b"Riku\0".to_vec(),
                Some(ap::Label::Owner(ap::Owner::Sora)) => ap::text::SORA.to_vec(),
                Some(ap::Label::Owner(ap::Owner::Shared)) => ap::text::SHARED.to_vec(),
                Some(ap::Label::Owner(ap::Owner::EnemySkills)) => ap::text::ENEMY_SKILLS.to_vec(),
                Some(ap::Label::Owner(ap::Owner::Sleights)) => ap::text::SLEIGHTS.to_vec(),
                // Enemy Skills: the card's own name as Re:CoM shows it (its card-name call, localized).
                Some(ap::Label::Ability(i)) => match crate::abilities::ALL[i].source {
                    // SAFETY: Re:CoM's card-name call (reads its message tables), as the Keyblade list's rows.
                    crate::abilities::Source::Card(c) => unsafe { enemy_card_name(c, crate::abilities::ALL[i].name) },
                    _ => kb::c_text(crate::abilities::ALL[i].name),
                },
                None => ip::text::NONE.to_vec(),
            },
        }
    }

    /// A greyed row (Items, Abilities: KH1's greyed rows).
    fn row_grey(&self, idx: usize) -> bool {
        match &self.content {
            // 0.163.0 / 0.167.0: the Keyblade of Heart before Riku's first Dark Mode.
            // SAFETY: reads the save (the camp, main thread).
            Content::Keyblades { cards, .. } => cards.get(idx) == Some(&RIKU_HEART) && unsafe { !super::riku_skin::blade_unlocked() },
            Content::Items { rows, .. } => rows.get(idx).is_some_and(|r| r.grey),
            Content::Abilities { rows, .. } => rows.get(idx).is_some_and(|r| r.grey),
            // M12 step 6: a slot whose sleight is no longer equipped.
            Content::Customize { picking: None, .. } => self.cz_slot(self.cz_index(idx)).is_some_and(|(b, sl)| customize::unequipped(b, sl).is_some()),
            _ => false,
        }
    }

    /// The row's look on Re:CoM's list (`CBUTTON_SETUP`'s variant, `crate::keyblades::row_look`):
    /// the Items page's Stock row and item slots and the Abilities page's rows are Re:CoM's green
    /// row (green Mickey symbol); the Customize page's shortcut slots and spells are Re:CoM's blue
    /// magic row (owner, 2026-11-01); everything else keeps the red row.
    fn row_look(&self, idx: usize, item_row: bool) -> i32 {
        match &self.content {
            Content::Items { rows, .. } => rows.get(idx).map_or(kb::row_look::RED, |r| ip::row_look(r.label, item_row)),
            Content::Abilities { .. } => ap::ROW_LOOK,
            // 0.198.18: the slot list's rows by set (Riku's Dark Mode set: Re:CoM's row with Dark Riku's wing).
            Content::Customize { picking: None, .. } => customize::slot_look(self.cz_index(idx)),
            Content::Customize { picking, .. } => customize::row_look(picking.is_some(), picking.and_then(|k| self.cz_picks(k).get(idx).copied().flatten())),
            Content::Keyblades { .. } => kb::row_look::RED,
        }
    }

    /// Abilities: reads the ability state again (a sync: new unlocks from Re:CoM), the
    /// window's rows, and the list from the screen's cursor, scrolled where the window was
    /// ([`list::Scroll`]). Nothing for the other pages.
    fn abilities_read(&mut self) {
        let Content::Abilities { screen, state, level, rows, live, .. } = &mut self.content else { return };
        match super::abilities::page_state() {
            Some((s, l)) => {
                (*state, *level, *live) = (s, l, true);
            }
            None => *live = false,
        }
        *rows = screen.rows(state);
        let n = rows.len() as i32;
        let c = screen.cursor().clamp(0, (n - 1).max(0));
        screen.set_cursor(c);
        self.list = self.scroll.reread(self.list, screen.depth(), n, c);
    }

    /// Items: reads the inventory and Sora's state again, the window's rows, and the list from the
    /// screen's cursor, scrolled where the window was ([`list::Scroll`]). Nothing for the other
    /// pages.
    fn items_read(&mut self) {
        let Content::Items { screen, inv, ctx, rows, .. } = &mut self.content else { return };
        *inv = super::items::inventory().unwrap_or_default();
        *ctx = super::items::menu_ctx();
        *rows = screen.rows(inv, *ctx);
        let c = screen.cursor(inv, *ctx);
        screen.set_cursor(c);
        let n = rows.len() as i32;
        self.list = self.scroll.reread(self.list, screen.depth(), n, c);
    }

    /// Whether row `idx` has Re:CoM's row mark: the equipped Keyblade, the picker's row holding the
    /// slot's spell, or an equipped ability.
    fn marked(&self, idx: usize) -> bool {
        match &self.content {
            Content::Keyblades { cards, equipped } => cards.iter().position(|c| c == equipped) == Some(idx),
            Content::Customize { picking: Some(k), .. } => self.cz_slot(*k).is_some_and(|(b, _)| customize::row_of(self.cz_picks(*k), b) == Some(idx)),
            Content::Customize { picking: None, .. } => false,
            Content::Items { .. } => false,
            Content::Abilities { rows, .. } => rows.get(idx).is_some_and(|r| r.marked),
        }
    }

    /// The status panel's three rows: labels and values.
    fn status(&self) -> ([&'static [u8]; 3], [Vec<u8>; 3]) {
        match &self.content {
            Content::Keyblades { .. } if riku_page() => (text::STAT_LABELS, [0u8; 3].map(|_| kb::c_text("-"))),
            Content::Items { screen, inv, ctx, .. } if riku_page() && screen.slot(inv, *ctx) == Some(ip::Slot::Keyblade) => (text::STAT_LABELS, [0; 3].map(|_| kb::c_text("-"))),
            Content::Keyblades { cards, .. } => {
                let values = cards
                    .get(self.list.cursor as usize)
                    .and_then(|&c| super::stats::keyblade(c))
                    .map(kb::stat_values)
                    .unwrap_or_else(|| [String::new(), String::new(), String::new()]);
                (text::STAT_LABELS, values.map(|v| kb::c_text(&v)))
            }
            Content::Customize { levels, picking, .. } => {
                // 0.198.18: the set of the row (or the picker's slot) the cursor is on.
                let at = picking.unwrap_or(self.cz_index(self.list.cursor.max(0) as usize));
                let first = if at >= customize::SLOTS { customize::SLOTS } else { 0 };
                let v = |k: usize| pick_word(self.cz_slot(first + k).and_then(|(b, _)| cm::shortcut::slot_of(b)), levels);
                (customize::text::slot_labels(button_glyphs(ROW_FONT), super::command_menu::confirm_is_circle()), [v(0), v(1), v(2)])
            }
            // Sora's Keyblade slot: the equipped Keyblade's numbers, as the Keyblade list shows them.
            Content::Items { screen, inv, ctx, .. } if screen.slot(inv, *ctx) == Some(ip::Slot::Keyblade) => {
                let values = super::stats::keyblade(super::stats::equipped_card()).map(kb::stat_values).unwrap_or_else(|| [String::new(), String::new(), String::new()]);
                (text::STAT_LABELS, values.map(|v| kb::c_text(&v)))
            }
            Content::Items { screen, inv, ctx, .. } => {
                let (owner, item) = screen.status(inv, *ctx);
                let o = item_label(ip::Label::Owner(owner));
                let (name, count) = if item == 0 { (ip::text::NONE.to_vec(), ip::text::NONE.to_vec()) } else { (item_word(item), kb::c_text(&inv.stock_of(item).to_string())) };
                (ip::text::STATUS, [o, name, count])
            }
            Content::Abilities { screen, state, .. } => (ap::text::STATUS, screen.status(state).map(|v| kb::c_text(&v))),
        }
    }

    /// The help line for the page's current mode and row.
    unsafe fn help(&self) {
        let t: &[u8] = match &self.content {
            Content::Keyblades { .. } => text::PAGE_HELP,
            // 0.198.18: the Dark Mode set's picker names it.
            Content::Customize { picking: Some(k), .. } if *k >= customize::SLOTS => customize::text::dark_pick_help(*k - customize::SLOTS, help_glyphs(self.p()), super::command_menu::confirm_is_circle()),
            Content::Customize { picking: Some(k), .. } => customize::text::pick_help(*k % customize::SLOTS, help_glyphs(self.p()), super::command_menu::confirm_is_circle()),
            Content::Customize { picking: None, .. } if self.cz_slot(self.cz_index(self.list.cursor.max(0) as usize)).is_some_and(|(b, sl)| customize::unequipped(b, sl).is_some()) => {
                self.cz_slot(self.cz_index(self.list.cursor.max(0) as usize)).and_then(|(b, sl)| customize::unequipped(b, sl)).map_or(customize::text::UNEQUIPPED_HELP, customize::text::unequipped_help)
            }
            // 0.198.18: the Dark Mode set (shown with Left / Right) names it and the way back; Riku's own, the way there.
            Content::Customize { picking: None, .. } if self.cz_index(self.list.cursor.max(0) as usize) >= customize::SLOTS => {
                customize::text::dark_slot_help(self.list.cursor.max(0) as usize % customize::SLOTS, help_glyphs(self.p()), super::command_menu::confirm_is_circle())
            }
            Content::Customize { picking: None, dark: Some(_), .. } => customize::text::switch_slot_help(self.list.cursor.max(0) as usize, help_glyphs(self.p()), super::command_menu::confirm_is_circle()),
            Content::Customize { picking: None, .. } => customize::text::slot_help(self.list.cursor.max(0) as usize, help_glyphs(self.p()), super::command_menu::confirm_is_circle()),
            Content::Items { screen, rows, msg, .. } => screen.help(rows, self.list.cursor, *msg),
            Content::Abilities { msg: Some(m), .. } => m,
            Content::Abilities { rows, .. } => match rows.get(self.list.cursor.max(0) as usize) {
                Some(ap::Row { label: ap::Label::Owner(_), grey: true, .. }) | None => ap::text::EMPTY_HELP,
                Some(ap::Row { label: ap::Label::Owner(ap::Owner::Sora), .. }) => ap::text::OWNER_HELP[0],
                Some(ap::Row { label: ap::Label::Owner(ap::Owner::Shared), .. }) => ap::text::OWNER_HELP[1],
                Some(ap::Row { label: ap::Label::Owner(ap::Owner::EnemySkills), .. }) => ap::text::OWNER_HELP[2],
                Some(ap::Row { label: ap::Label::Owner(ap::Owner::Sleights), .. }) => ap::text::OWNER_HELP[3],
                Some(ap::Row { label: ap::Label::Ability(i), .. }) => ap::text::help(*i),
            },
        };
        set_help(self.p(), t.as_ptr());
    }

    /// Draw mode 9 (rows, cursor, scroll bar), plus the status rows.
    unsafe fn redraw(&mut self) {
        let camp = self.p();
        let set = set_of(camp);
        self.list.top = list::top_for(self.list);
        let no_scale = at::<i32>(camp, camp::NO_SCALE) != 0;
        let font_global: *const u8 = rd(va(base(), data::FONT_GLOBAL));
        let lang = if font_global.is_null() { 0 } else { rd::<i32>(font_global as usize + 0x274) };
        let (font, f9, f10) = if no_scale {
            (0xE, 0.0, 0.0)
        } else if (lang - 3) as u32 <= 2 {
            (0xE, 1.0, 2.0)
        } else {
            (0xF, 1.0, 2.0)
        };
        let colour: u32 = at(camp, camp::TEXT_COLOUR);
        let setup = func::<SetupFn>(CBUTTON_SETUP);
        // The item row needs our table (Re:CoM's + the new group) and a loaded set that has it.
        let item_table = ITEM_TABLE.get().zip(ITEM_ROWS.get()).and_then(|(t, rows)| {
            let r = item_row_loaded(set, rows)?;
            t[crate::camp_rows::RECOM_ENTRIES].store(r, Ordering::Relaxed);
            Some(t)
        });
        let seqs = item_table.map_or(va(base(), LIST_SEQ) as *const u16, |t| t.as_ptr() as *const u16);
        let item_row = item_table.is_some();
        for i in 0..list::VISIBLE as usize {
            delete(self.rows[i] as *mut u8);
            self.rows[i] = 0;
            let idx = self.list.top + i as i32;
            if idx >= self.list.count {
                continue;
            }
            let pos = self.layout(0x12, 2 + i as u64, 0).unwrap_or_default();
            let btn = func::<NewFn>(NEW)(0x30);
            if btn.is_null() {
                continue;
            }
            func::<CtorFn>(CBUTTON_CTOR)(btn);
            self.rows[i] = btn as usize;
            let name = self.row_text(idx as usize);
            let c = if self.row_grey(idx as usize) { ip::grey_colour(colour) } else { colour };
            setup(btn, set, 0x11, seqs, pos.0.as_ptr(), self.list_panel as *mut u8, name.as_ptr(), font, f9, f10, 0x40, &c, self.row_look(idx as usize, item_row));
            // 0.140.0 (owner, 2026-11-05): an equipped row says "Equipped" where Re:CoM's "NEW!"
            // badge was.
            let spr: *mut u8 = at(btn, 0x18);
            let word = if self.marked(idx as usize) && !spr.is_null() { self.equipped_word(spr, set, idx) } else { None };
            // 0.113.0, true squeeze (owner, 2026-11-04): a name wider than the row's plate is drawn
            // narrower (`CButton +0x20` = the row's `C2DFnt`, `0x140112F10`). One box on every row
            // (Re:CoM's cursor frame hugs the plate), so a name keeps one width; a marked row's name
            // ends before its word (0.140.0).
            let name_box = word.map_or(crate::text_fit::boxes::CAMP_ROW, kb::badge::name_box);
            super::text_fit::fit(at::<usize>(btn, crate::text_fit::recom::BUTTON_TEXT), name_box, || format!("{} list row {idx}", self.kind()));
        }
        self.place_cursor();
        if self.bar != 0 {
            let (t, s) = list::bar(self.list);
            func::<BarUpdateFn>(BAR_UPDATE)(self.bar as *mut u8, t, s);
        }
        self.stats();
    }

    /// The "Equipped" word on a marked row's sprite (`crate::keyblades::badge`): Re:CoM's `C2DFnt`
    /// (`0x140243F10`, the status panel's font 0xE, right-aligned 0x41, the text copied in with
    /// `0x140258DE0`) attached with the sprite's `+0x60` as the status values are; the sprite owns
    /// it (deleted with the row, as the values are). Squeezed past [`kb::badge::MAX_WIDTH`]. Returns
    /// its drawn width (units), `None` when nothing was made.
    unsafe fn equipped_word(&self, spr: *mut u8, set: *const u8, idx: i32) -> Option<f32> {
        let mut w = func::<NewFn>(NEW)(0xB0);
        if w.is_null() {
            return None;
        }
        let colour = kb::badge::COLOUR;
        w = func::<FntCtorFn>(FNT_CTOR)(w, 0, 0, &colour, kb::badge::FONT, std::ptr::null(), kb::badge::FLAGS);
        if w.is_null() {
            return None;
        }
        func::<TextFn>(TEXT_COPY)(w, kb::badge::TEXT.as_ptr());
        let level = super::text_fit::fit(w as usize, kb::badge::MAX_WIDTH, || format!("{} list row {idx} \"Equipped\"", self.kind()));
        let width = super::text_fit::width(w as usize).map(|w16| crate::text_fit::scaled(w16 as i64, level) as f32 / 16.0);
        vfn::<AttachFn>(spr, 0x60)(spr, set, w, kb::badge::X, kb::badge::Y, 0.0);
        if !WORD_LOGGED.swap(true, Ordering::Relaxed) {
            log(&format!(
                "{}: equipped rows say \"Equipped\" (Re:CoM's text, font {:#x}, right edge x {} y {} of the row, colour {:#x}); drawn {} units wide (squeeze {:.3})",
                self.kind(),
                kb::badge::FONT,
                kb::badge::RIGHT,
                kb::badge::Y,
                kb::badge::COLOUR,
                width.map_or_else(|| "?".to_string(), |w| format!("{w:.1}")),
                crate::text_fit::factor(level)
            ));
        }
        Some(width.unwrap_or(kb::badge::MAX_WIDTH))
    }

    /// The cursor onto the highlighted row (draw mode 9's tail, `0x1400E7ABF..0x1400E7B6F`).
    unsafe fn place_cursor(&self) {
        let cur = self.cursor as *mut u8;
        let r = (self.list.cursor - self.list.top) as usize;
        if cur.is_null() || r >= self.rows.len() || self.rows[r] == 0 {
            return;
        }
        let spr: *mut u8 = at(self.rows[r] as *mut u8, 0x18);
        if spr.is_null() {
            return;
        }
        func::<PosFn>(SPRITE_POS)(cur, spr.add(0xF0) as *const f32);
        if at::<usize>(cur, 0x170) == 0 {
            return;
        }
        let v: Vec4 = at(spr, 0xF0);
        let t = Vec4([v.0[0] + CURSOR_DX, v.0[1] + CURSOR_DY, v.0[2], v.0[3]]);
        put::<u64>(cur, 0x1B0, 1);
        put::<Vec4>(cur, 0x190, t);
        let src: Vec4 = if at::<i32>(cur, 0x1B8) != 0 { at(cur, 0x180) } else { t };
        put::<Vec4>(cur, 0x1A0, src);
    }

    /// The status rows (draw mode 0's label / value rows, layout group 0x11 elements 4.., y + 8),
    /// rebuilt on each change as the list rows are.
    unsafe fn stats(&mut self) {
        let camp = self.p();
        let set = set_of(camp);
        let no_scale = at::<i32>(camp, camp::NO_SCALE) != 0;
        let y = if no_scale { 0.0 } else { 1.0 };
        let (labels, values) = self.status();
        let setup = func::<SetupFn>(CBUTTON_SETUP);
        let seqs = va(base(), STATUS_SEQ) as *const u16;
        for (i, value) in values.iter().enumerate() {
            delete(self.stat_rows[i] as *mut u8);
            self.stat_rows[i] = 0;
            let mut pos = self.layout(0x11, 4 + i as u64, 1).unwrap_or_default();
            pos.0[1] += 8.0;
            let btn = func::<NewFn>(NEW)(0x30);
            if btn.is_null() {
                continue;
            }
            func::<CtorFn>(CBUTTON_CTOR)(btn);
            self.stat_rows[i] = btn as usize;
            setup(btn, set, 0x10, seqs, pos.0.as_ptr(), self.stats_panel as *mut u8, labels[i].as_ptr(), 0xE, 0.0, y, 0x40, std::ptr::null(), i as i32);
            let mut v = func::<NewFn>(NEW)(0xB0);
            if !v.is_null() {
                let colour = VALUE_COLOUR;
                v = func::<FntCtorFn>(FNT_CTOR)(v, 0, 0, &colour, 0xE, std::ptr::null(), 0x41);
                func::<TextFn>(TEXT_COPY)(v, value.as_ptr());
                // 0.113.0, true squeeze: the value (right-aligned at 220) ends where it ends and
                // starts no further left than its label's end + a gap (both measured by Re:CoM).
                let label = super::text_fit::width(at::<usize>(btn, crate::text_fit::recom::BUTTON_TEXT)).map_or(0.0, |w| w as f32 / 16.0);
                super::text_fit::fit(v as usize, crate::text_fit::boxes::status(label), || format!("{} status value {i} (label {label:.1} units)", self.kind()));
            }
            let spr: *mut u8 = at(btn, 0x18);
            if !spr.is_null() {
                vfn::<AttachFn>(spr, 0x60)(spr, set, v, 220.0, y, 0.0);
            }
        }
    }

    /// Draw mode 3: the cursor glides (`0x1400DE010(cursor, dt × 0.02)`).
    unsafe fn glide(&self) {
        if self.cursor != 0 {
            let dt = at::<i32>(self.p(), camp::DT) as f32 * 0.020_000_001;
            func::<GlideFn>(CURSOR_GLIDE)(self.cursor as *mut u8, dt);
        }
    }

    /// Sub-state 1 (draw mode 1): when the status panel's opening ends, both panels idle, the
    /// cursor shows, the help line is ours.
    unsafe fn opening(&mut self) -> bool {
        let done = ended(self.stats_panel as *mut u8);
        if done {
            let set = set_of(self.p());
            anim(self.stats_panel as *mut u8, set, 0x126);
            anim(self.list_panel as *mut u8, set, 0x14D);
            let cur = self.cursor as *mut u8;
            if !cur.is_null() {
                func::<SprIntFn>(VISIBLE)(cur, 1);
                let child: *mut u8 = at(cur, 0x170);
                if !child.is_null() {
                    func::<SprIntFn>(VISIBLE)(child, 1);
                }
            }
            self.help();
        }
        self.glide();
        done
    }

    /// Square on Riku's Customize: Re:CoM's Sleights List opens over the page, as the deck page's
    /// state 0x30 opens it (`0x1400DBCF7`: `camp+0xAC = 0`, `camp+0x8C = 1`).
    unsafe fn sleights_list_open(&mut self) -> bool {
        let camp = self.p();
        if !self.riku_list || self.sleights != SleightsList::Idle || at::<i32>(camp, camp::SLEIGHTS_LIST) != -1 {
            return false;
        }
        if func::<CampDirFn>(CAMP_PRESS)(camp, SQUARE, 0) == 0 {
            return false;
        }
        // Its open plays slot 0's close without a null check (`0x1400EBE30` state 0).
        if at::<usize>(camp, camp::PROMPT_BAR) == 0 {
            func::<PromptFn>(PROMPT_BAR)(camp, 1, 0, 9);
        }
        put::<i32>(camp, camp::SLEIGHTS_LIST, 0);
        put::<i32>(camp, camp::INPUT_LOCK, 1);
        self.sleights = SleightsList::Running;
        log("customize: Riku: Re:CoM's Sleights List opened (Square)");
        true
    }

    /// One frame of Re:CoM's Sleights List; our page's input waits (the list reads the pad itself).
    unsafe fn sleights_list_step(&mut self) {
        let camp = self.p();
        let done = func::<ListFn>(SLEIGHTS_LIST)(camp) as u32 == 1;
        match self.sleights {
            SleightsList::Running if done => self.sleights = SleightsList::Draining,
            SleightsList::Draining => {
                self.sleights = SleightsList::Idle;
                log(&format!("customize: Riku: Re:CoM's Sleights List closed (input lock {})", at::<i32>(camp, camp::INPUT_LOCK)));
                self.help();
            }
            _ => {}
        }
    }

    /// Draw mode 4, sound 4, sub-state 3: the page closes.
    unsafe fn close(&mut self) {
        let camp = self.p();
        if self.riku_list && at::<usize>(camp, camp::PROMPT_BAR) != 0 {
            // As the deck page's close (`0x1400DBD30`); the camp update's trailer deletes it.
            func::<PromptFn>(PROMPT_BAR)(camp, 4, 0, 0);
        }
        let set = set_of(camp);
        anim(self.stats_panel as *mut u8, set, 0x127);
        anim(self.list_panel as *mut u8, set, 0x14E);
        if self.cursor != 0 {
            func::<VoidFn>(CURSOR_HIDE)(self.cursor as *mut u8);
        }
        set_help(camp, message(rd::<u32>(va(base(), CLOSE_HELP_ID))));
        sound(4);
        self.sub = 3;
    }

    /// Customize: the list shows the slots (`slot` highlighted) or the picker for a slot.
    unsafe fn customize_mode(&mut self, picking: Option<usize>, cursor: i32) {
        let count = match picking {
            Some(k) => self.cz_picks(k).len() as i32,
            None => customize::SLOTS as i32,
        };
        let Content::Customize { picking: p, .. } = &mut self.content else { return };
        *p = picking;
        self.list = list::open_on(count, cursor);
        self.redraw();
        self.help();
    }

    /// Confirm on the highlighted row.
    unsafe fn confirm(&mut self) {
        let cursor = self.list.cursor;
        match &self.content {
            Content::Keyblades { cards, .. } if riku_page() => {
                let Some(card) = cards.get(cursor as usize).copied() else { return };
                if card == RIKU_HEART && !super::riku_skin::blade_unlocked() {
                    sound(5);
                    set_help(self.p(), crate::menu::BLADE_LOCKED.as_ptr());
                    return;
                }
                super::set_item(crate::menu::Item::Blade, (card == RIKU_HEART) as usize);
                if let Content::Keyblades { equipped, .. } = &mut self.content {
                    *equipped = card;
                }
                sound(2);
                self.redraw();
                log(&format!("keyblades: Riku holds the {}", if card == RIKU_HEART { "Keyblade of Heart" } else { "Soul Eater" }));
            }
            Content::Keyblades { cards, .. } => {
                let Some(card) = cards.get(cursor as usize).copied() else { return };
                super::stats::set_equipped(card);
                let now = super::stats::equipped_card();
                if let Content::Keyblades { equipped, .. } = &mut self.content {
                    *equipped = now;
                }
                // Its model starts loading now; the weapon is rebuilt when the camp closes.
                super::models::prefetch();
                sound(2);
                self.redraw();
                log(&format!("keyblades: equipped card {card:#x} (save byte {:#x})", kb::save_byte_for(card)));
            }
            Content::Customize { picking: None, .. } => {
                // KH1's slot list → its picker, on the picker's last row (`0x142E9A040`). 0.198.18: the set on screen.
                let k = self.cz_index(cursor.max(0) as usize);
                let at = customize::picker_cursor(LAST_PICK.load(Ordering::Relaxed) as i32, self.cz_picks(k).len() as i32);
                sound(2);
                self.customize_mode(Some(k), at);
            }
            Content::Customize { picking: Some(k), dark: Some(d), levels, .. } if *k >= customize::SLOTS => {
                // 0.198.18: Riku's Dark Mode set.
                let (j, old, levels) = (*k - customize::SLOTS, d.slots, *levels);
                let pick = d.picks.get(cursor as usize).copied().flatten();
                let new = customize::set(old, j, pick);
                if super::mp::set_riku_dark_shortcuts(new) {
                    if let Content::Customize { dark: Some(d), .. } = &mut self.content {
                        d.slots = new;
                    }
                    sound(2);
                    let name = |p: customize::Pick| match p {
                        None => "None".to_string(),
                        Some(cm::shortcut::Slot::Spell(s)) => crate::unlocks::level_name(s, levels[s as usize].clamp(1, 3)).to_string(),
                        Some(cm::shortcut::Slot::Sleight(id)) => format!("sleight {} ({id:#x})", crate::sleights::name(id)),
                    };
                    log(&format!("customize: Riku's Dark Mode shortcut {} = {} (was {}); slots {new:?}", j + 1, name(pick), name(cm::shortcut::slot_of(old[j]))));
                } else {
                    sound(5);
                    log("customize: the save block isn't there (MP not live); Dark Mode shortcut not set");
                }
                self.customize_mode(None, j as i32);
            }
            Content::Customize { picking: Some(k), picks, slots, levels, .. } => {
                let (k, slots, levels) = (*k, *slots, *levels);
                let pick = picks.get(cursor as usize).copied().flatten();
                let new = customize::set(slots, k, pick);
                let name = |p: customize::Pick| match p {
                    None => "None".to_string(),
                    Some(cm::shortcut::Slot::Spell(s)) => crate::unlocks::level_name(s, levels[s as usize].clamp(1, 3)).to_string(),
                    Some(cm::shortcut::Slot::Sleight(id)) => format!("sleight {} ({id:#x})", crate::sleights::name(id)),
                };
                let written = if story(self.p()) == kb::Story::Riku { super::mp::set_riku_shortcuts(new) } else { super::mp::set_shortcuts(new) };
                if written {
                    if let Content::Customize { slots: s, .. } = &mut self.content {
                        *s = new;
                    }
                    sound(2);
                    log(&format!("customize: shortcut {} = {} (was {}); slots {new:?}", k + 1, name(pick), name(cm::shortcut::slot_of(slots[k]))));
                } else {
                    sound(5);
                    log("customize: the save block isn't there (MP not live); shortcut not set");
                }
                self.customize_mode(None, k as i32);
            }
            Content::Items { .. } => self.items_confirm(),
            Content::Abilities { .. } => self.abilities_confirm(),
        }
    }

    /// Abilities: confirm on the highlighted row (`crate::abilities::page::Screen::confirm`, KH1's
    /// rule), written to our save block, with Re:CoM's camp sounds (2 opened / equipped / removed,
    /// 5 refused or nothing to open).
    unsafe fn abilities_confirm(&mut self) {
        let cursor = self.list.cursor;
        let Content::Abilities { screen, state, msg, live, .. } = &mut self.content else { return };
        screen.set_cursor(cursor);
        *msg = None;
        let mut s = *state;
        let out = screen.confirm(&mut s);
        match out {
            ap::Outcome::Opened => sound(2),
            ap::Outcome::Equipped(i) | ap::Outcome::Removed(i) => {
                if !*live || !super::abilities::set_equipped(s.equipped) {
                    sound(5);
                    log("abilities page: the ability state isn't live (no save block, or not Combat = KH1 in Sora's story); nothing changed");
                } else {
                    sound(2);
                    log(&format!(
                        "abilities page: {} {} ({} AP); AP {}/{} left; equipped {:?}",
                        if matches!(out, ap::Outcome::Equipped(_)) { "equipped" } else { "removed" },
                        crate::abilities::ALL[i].name,
                        crate::abilities::ALL[i].ap,
                        crate::abilities::ap_left(s.equipped, s.max_ap()),
                        s.max_ap(),
                        crate::abilities::names(s.equipped)
                    ));
                }
            }
            ap::Outcome::Refused(i) => {
                *msg = Some(ap::text::NO_AP);
                sound(5);
                log(&format!("abilities page: refused {} ({} AP): AP {} left (KH1's rule)", crate::abilities::ALL[i].name, crate::abilities::ALL[i].ap, crate::abilities::ap_left(s.equipped, s.max_ap())));
            }
            ap::Outcome::NotBuilt(i) => {
                *msg = Some(ap::text::NOT_BUILT);
                sound(5);
                log(&format!("abilities page: refused {} ({} AP): its effect isn't built yet", crate::abilities::ALL[i].name, crate::abilities::ALL[i].ap));
            }
            ap::Outcome::Empty => {
                *msg = Some(ap::text::EMPTY_HELP);
                sound(5);
            }
            ap::Outcome::Nothing | ap::Outcome::Back | ap::Outcome::Close => {}
        }
        self.abilities_read();
        self.redraw();
        self.help();
    }

    /// Items: confirm on the highlighted row (`crate::items_page::Screen::confirm` on our save
    /// block's inventory), with Re:CoM's camp sounds (2 opened / done, 5 refused).
    unsafe fn items_confirm(&mut self) {
        let cursor = self.list.cursor;
        let Content::Items { screen, msg, ctx, inv: shown, .. } = &mut self.content else { return };
        screen.set_cursor(cursor);
        let ctx = *ctx;
        // The Keyblade slot needs no item store: tried on copies when the items aren't live.
        let out = match super::items::edit(|inv| screen.confirm(inv, ctx)) {
            Some(out) => out,
            None if screen.clone().confirm(&mut shown.clone(), ctx) == ip::Outcome::Keyblade => ip::Outcome::Keyblade,
            None => {
                sound(5);
                log("items page: KH1's items aren't live (no save block, or not Combat = KH1 in Sora's story); nothing changed");
                return;
            }
        };
        *msg = None;
        if out == ip::Outcome::Keyblade {
            sound(2);
            self.open_keyblades();
            return;
        }
        match out {
            ip::Outcome::Nothing => {}
            ip::Outcome::Refused => sound(5),
            ip::Outcome::Error(m) => {
                *msg = Some(m);
                sound(5);
                log(&format!("items page: refused: {m:?}"));
            }
            ip::Outcome::Opened | ip::Outcome::Keyblade => sound(2),
            ip::Outcome::Back => sound(4),
            ip::Outcome::Close => {}
            ip::Outcome::Done(e) => {
                sound(2);
                let extra = match e {
                    ip::Effect::Used { item, .. } => format!(": {}", super::items::use_in_menu(item)),
                    _ => String::new(),
                };
                let inv = super::items::inventory().unwrap_or_default();
                log(&format!(
                    "items page: {e:?}{extra}; slots {:?}, stock {:?}",
                    &inv.slots[..(inv.slot_count as usize).min(crate::items::MAX_SLOTS)],
                    ip::stock_list(&inv).iter().map(|&i| (i, inv.stock_of(i))).collect::<Vec<_>>()
                ));
            }
        }
        if out == ip::Outcome::Close {
            self.close();
            return;
        }
        self.items_read();
        self.redraw();
        self.help();
    }

    /// Cancel: the picker goes back to the slot list (no write), the Keyblade list to the Items
    /// screen; the Items screen steps back a window; anything else closes the page.
    unsafe fn cancel(&mut self) {
        if matches!(self.content, Content::Keyblades { .. }) && self.under.is_some() {
            sound(4);
            self.back_from_keyblades();
            return;
        }
        if let Content::Customize { picking: Some(k), .. } = self.content {
            sound(4);
            self.customize_mode(None, (k % customize::SLOTS) as i32);
            return;
        }
        if let Content::Abilities { screen, msg, .. } = &mut self.content {
            *msg = None;
            if screen.cancel() == ap::Outcome::Back {
                sound(4);
                self.abilities_read();
                self.redraw();
                self.help();
                return;
            }
        }
        if let Content::Items { screen, msg, .. } = &mut self.content {
            *msg = None;
            if screen.cancel() == ip::Outcome::Back {
                sound(4);
                self.items_read();
                self.redraw();
                self.help();
                return;
            }
        }
        self.close();
    }

    /// Sub-state 2: the list's input, in `0x1400EB7E0`'s order. Returns the page's result.
    unsafe fn input(&mut self) -> i32 {
        let camp = self.p();
        self.glide();
        if func::<CampBtnFn>(CAMP_CANCEL)(camp, 0) & 0xFF != 0 {
            self.cancel();
            return 1;
        }
        if func::<CampBtnFn>(CAMP_DECIDE)(camp, 0) & 0xFF != 0 {
            self.confirm();
            return 0;
        }
        let dir = func::<CampDirFn>(CAMP_DIR);
        // 0.198.18: Riku's two sets (Re:CoM's Sleights page's tab keys, `0x1400EB7E0` state 4).
        if matches!(self.content, Content::Customize { dark: Some(_), picking: None, .. }) {
            if dir(camp, LEFT, 0) != 0 && self.switch_set(true) {
                return 0;
            }
            if dir(camp, RIGHT, 0) != 0 && self.switch_set(false) {
                return 0;
            }
        }
        let mut pad = list::Pad::default();
        if dir(camp, 0x10_1000, 0) != 0 {
            pad.up = true;
            pad.up_fresh = at::<i32>(camp, camp::FRESH) != 0;
        } else if dir(camp, 0x20_4000, 0) != 0 {
            pad.down = true;
            pad.down_fresh = at::<i32>(camp, camp::FRESH) != 0;
        } else {
            let rep = func::<PadFn>(PAD_REPEAT);
            let fresh = func::<PadFn>(PAD_FRESH);
            if rep(4, 0) != 0 {
                pad.page_up = true;
                pad.page_up_fresh = fresh(4, 0) != 0;
            } else if rep(8, 0) != 0 {
                pad.page_down = true;
                pad.page_down_fresh = fresh(8, 0) != 0;
            }
        }
        let (n, moved) = list::step(self.list, pad);
        self.list = n;
        if let Content::Items { screen, msg, .. } = &mut self.content {
            screen.set_cursor(n.cursor);
            if moved {
                *msg = None;
            }
        }
        if let Content::Abilities { screen, msg, .. } = &mut self.content {
            screen.set_cursor(n.cursor);
            if moved {
                *msg = None;
            }
        }
        if moved {
            sound(1);
            self.redraw();
            match self.content {
                Content::Customize { picking: Some(_), .. } => LAST_PICK.store(n.cursor.max(0) as usize, Ordering::Relaxed),
                Content::Customize { picking: None, .. } | Content::Items { .. } | Content::Abilities { .. } => self.help(),
                Content::Keyblades { .. } => {}
            }
        }
        0
    }

    /// One frame: sub-state 1 opening, 2 input, 3 closing (−1 when the status panel's close
    /// ends, its objects deleted).
    unsafe fn step(&mut self) -> i32 {
        match self.sub {
            1 => {
                if self.opening() {
                    self.sub = 2;
                }
                0
            }
            2 if self.sleights != SleightsList::Idle => {
                self.sleights_list_step();
                0
            }
            2 => {
                if self.sleights_list_open() {
                    return 0;
                }
                self.input().min(0)
            }
            _ => {
                if ended(self.stats_panel as *mut u8) {
                    self.delete_all();
                    self.stats_panel = 0;
                    self.list_panel = 0;
                    self.cursor = 0;
                    self.bar = 0;
                    self.rows = [0; 8];
                    self.stat_rows = [0; 3];
                    -1
                } else {
                    0
                }
            }
        }
    }
}

/// The Keyblade's localized card name (`0x140164CA0`, message `[0x140528560 + kind × 4]`), or the
/// stats file's name when that message isn't loaded.
/// Riku's Soul Eater (Re:CoM's card 0x12) and the Keyblade of Heart (ours: no card).
const SOUL_EATER: u16 = 0x12;
const RIKU_HEART: u16 = 0x100;

/// The page belongs to Riku's story (his items, his Keyblades).
fn riku_page() -> bool {
    super::stats::riku_story()
}

/// The Keyblade Riku holds now (the setting, once unlocked).
unsafe fn riku_blade() -> u16 {
    if super::settings().riku_heart_blade && super::riku_skin::blade_unlocked() {
        RIKU_HEART
    } else {
        SOUL_EATER
    }
}

/// A Keyblade list row's name: Re:CoM's card name, or the Keyblade of Heart's.
unsafe fn blade_name(card: u16) -> Vec<u8> {
    if card == RIKU_HEART {
        return b"Keyblade of Heart\0".to_vec();
    }
    card_name(card)
}

unsafe fn card_name(card: u16) -> Vec<u8> {
    let id = [card as u8, 0, 0, 0];
    let p = func::<NameFn>(CARD_NAME)(id.as_ptr());
    if !p.is_null() && *p != 0 {
        let s = std::ffi::CStr::from_ptr(p as *const std::ffi::c_char);
        let mut v = s.to_bytes().to_vec();
        v.push(0);
        return v;
    }
    kb::c_text(super::stats::keyblade(card).map(|k| k.name.as_str()).unwrap_or("?"))
}

/// An enemy card's localized name (Re:CoM's card-name call `0x140164CA0`: message `[0x140528560 +
/// kind × 4]`, which is the kind itself for 0x44..=0x7E, Checked (exe)), or our English name when
/// that message isn't loaded.
unsafe fn enemy_card_name(card: u16, english: &str) -> Vec<u8> {
    let id = [card as u8, 0, 0, 0];
    let p = func::<NameFn>(CARD_NAME)(id.as_ptr());
    if !p.is_null() && *p != 0 {
        let s = std::ffi::CStr::from_ptr(p as *const std::ffi::c_char);
        let mut v = s.to_bytes().to_vec();
        v.push(0);
        return v;
    }
    kb::c_text(english)
}

/// Re:CoM's "obtained at least once" (`0x14015B3F0`), when the collection is there.
pub(super) unsafe fn ever_obtained(id: u16) -> bool {
    let coll: usize = rd(va(base(), COLLECTION));
    coll != 0 && func::<EverFn>(EVER_OBTAINED)(id as u32) & 0xFF != 0
}

/// The Settings list and our KH1 list entries' pages, called by the Config input hook in place of
/// `0x1400D3EB0` when [`page_active`]: the same contract (≥ 0 running, < 0 closed). Re:CoM's
/// states 0x4A..0x4C around it.
pub unsafe fn page(camp: *mut u8) -> u64 {
    let Ok(mut g) = OPEN.lock() else { return (-1i32) as u32 as u64 };
    if g.as_ref().is_some_and(|s| s.camp() != camp as usize) {
        if let Some(stale) = g.take() {
            stale.delete_all();
        }
    }
    let r: i32 = match g.as_mut() {
        // State 0x4A's call: the list's buttons, or the page.
        None => {
            let saved = at::<i32>(camp, camp::SAVED);
            let row = if ours_confirmed(saved) { top::entry(list_now(), saved).and_then(|e| e.page()) } else { None };
            *g = Some(match row {
                Some(row) if row.settings_page().is_some() => {
                    log(&format!("camp: {row:?} opened from the top list (a settings page)"));
                    Session::Settings(super::menu::SettingsPage::open(camp, row.settings_page().unwrap()))
                }
                Some(row) => {
                    log(&format!("camp: {row:?} opened from the top list"));
                    Session::Page(Page::open(camp, row))
                }
                None => Session::List(Sub::open(camp)),
            });
            0
        }
        Some(Session::List(s)) => s.step(),
        Some(Session::Page(p)) => p.step(),
        Some(Session::Settings(p)) => p.step(),
    };
    if r < 0 {
        *g = None;
        CONFIRMED.store(-1, Ordering::Relaxed);
    }
    r as u32 as u64
}
