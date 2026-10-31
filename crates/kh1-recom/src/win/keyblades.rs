//! The camp menu's 8th entry, "Keyblades", and its page (decision 25), run by Re:CoM's own camp
//! code. Pure rules: `crate::keyblades`. Notes: `docs/recom-keyblade-equip.md`.
//!
//! - Entry: a `C2DSeqCtrl` + `C2DFnt` made with the calls Re:CoM's entry build `0x1400E3DE0`
//!   makes for its own 7, at index 7. Four wrappers make the camp treat index 7 as one more entry:
//!   the camp update `0x1400D9C80` (states 7 / 0xD / 0xF / 0x11), the entry build (create, delete,
//!   animations 4 / 5 / 6), the cursor set `0x1400DEA10` (highlight, help line, 8-entry wrap) and
//!   the teardown `0x1400D7C20` (delete ours).
//! - Page: run inside Re:CoM's Config states 0x4A..0x4C by the Config input hook
//!   (`menu::cfg_input` → [`page`]) when the cursor is on our entry. It follows the entry-4 page
//!   (`0x1400EB7E0` / draw `0x1400E6DF0`) with Re:CoM's classes: two panels, `CButton` rows, a
//!   `CButtonCursor`, a `CScrollBar`, label / value rows and the help line. Its state is ours
//!   ([`PAGE`]); it never writes camp fields Re:CoM's pages use.
//!
//! Nothing here runs in field or battle frames: the camp exists only while the pause menu is
//! open, and without our entry (Re:CoM setting, Riku's story) every wrapper is one load and the
//! original.
//!
//! Every offset and call below is Checked (decomp + disasm) unless tagged otherwise.

use super::movement::{func, orig, rd, wr};
use super::{base, log, va};
use crate::keyblades::{self as kb, list, text, top};
use crate::sites::data;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

pub static ORIG_UPDATE: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_BUILD: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_CURSOR: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_TEARDOWN: AtomicUsize = AtomicUsize::new(0);

/// The camp our entry belongs to, and our entry's `C2DSeqCtrl`.
static ENTRY_CAMP: AtomicUsize = AtomicUsize::new(0);
static ENTRY_BTN: AtomicUsize = AtomicUsize::new(0);
static SPARE_LOGGED: AtomicBool = AtomicBool::new(false);
static ENTRY_LOGGED: AtomicBool = AtomicBool::new(false);
static NO_NUMBERS_LOGGED: AtomicBool = AtomicBool::new(false);
static ODD_STATE_LOGGED: AtomicBool = AtomicBool::new(false);
static PAGE: Mutex<Option<Page>> = Mutex::new(None);

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
    /// Menu level: 0 top list, 1 Deck submenu, 2 in a page.
    pub const LEVEL: usize = 0xB4;
    pub const CURSOR: usize = 0xB8;
    /// The top index saved on confirm.
    pub const SAVED: usize = 0xBC;
    /// Header item 0's mode (the update's trailer runs `0x1400E4220(camp, [+0xC4], 0)`).
    pub const HEADER_MODE: usize = 0xC4;
    pub const MAIN_PANEL: usize = 0x130;
    pub const PANEL_140: usize = 0x140;
    /// Entry buttons (`C2DSeqCtrl`, 7) and their labels (`C2DFnt`, 7).
    pub const BUTTONS: usize = 0x1B8;
    pub const LABELS: usize = 0x1F0;
    /// Header item 0 (`C2DSeqCtrl`) and its title (`C2DFnt`).
    pub const HEADER: usize = 0x228;
    pub const HEADER_TITLE: usize = 0x238;
    pub const HELP: usize = 0x9D0;
    /// Non-zero: the camp opens straight into the map page and never shows the list.
    pub const DIRECT_MAP: usize = 0x20A4;
    /// `0x1401AAAE0()` at ctor: Riku's story.
    pub const RIKU: usize = 0x21BC;
    pub const COUNT: usize = 0x21C4;
    /// Copied each update from a frame counter (`% 7000`); the list rows' marker sprites get it.
    pub const BLINK: usize = 0x21D8;
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
const RIKU_STORY: u32 = 0x001A_AAE0;

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
/// Entry button position: (86.0, 88.0 + 26.0 × index) (`0x140509430`, `0x140509270`).
const ENTRY_X: f32 = 86.0;
const ENTRY_Y0: f32 = 88.0;
const ENTRY_STEP: f32 = 26.0;
/// Label colours: normal, highlighted (`0x1400E3DE0`, `0x1400DEA10`).
const LABEL_NORMAL: u32 = 0x8040_4040;
const LABEL_HIGHLIGHT: u32 = 0x80C0_C0C0;
/// The help ids of Sora's top entries 0..6: `0x500400 + index` (`0x1400DEA10`'s table).
const TOP_HELP_BASE: u32 = 0x0050_0400;
/// The status rows' value colour (`0x1400E6DF0`, rows 0..4).
const VALUE_COLOUR: u32 = 0x8004_6060;
/// The list cursor's second sprite: + (284, 168) from the row (`0x140509530`, `0x140509520`).
const CURSOR_DX: f32 = 284.0;
const CURSOR_DY: f32 = 168.0;
/// The list rows' marker sprite (`0x1400E6DF0` mode 9: Re:CoM's "seen" 0x1D / "new" 0x1E mark,
/// at (170, −13)). We mark the equipped Keyblade's row with 0x1D. Assumption: it reads as a mark
/// on this list (to check in game; 0x1E is the other choice).
const EQUIPPED_MARK: u32 = 0x1D;
const MARK_X: f32 = 170.0;
const MARK_Y: f32 = -13.0;

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
/// `+0x40` (`0x140247A50`): add a sequence at an offset; returns the new sequence object.
type MarkFn = unsafe extern "win64" fn(*mut u8, *const u8, u32, f32, f32, f32, u32) -> *mut u8;
/// Sequence object vtable `+0x30`, as `0x1400E6DF0` calls it with `camp+0x21D8`.
type ObjIntFn = unsafe extern "win64" fn(*mut u8, u32) -> u64;
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
type RikuFn = unsafe extern "win64" fn() -> u32;

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

unsafe fn set_help(camp: *mut u8, s: *const u8) {
    let help: *mut u8 = at(camp, camp::HELP);
    if !help.is_null() && !s.is_null() {
        func::<TextFn>(TEXT_SET)(help, s);
    }
}

/// Our entry's button when `camp` is the camp it was made for.
fn entry_of(camp: *mut u8) -> *mut u8 {
    if camp.is_null() || ENTRY_CAMP.load(Ordering::Relaxed) != camp as usize {
        return std::ptr::null_mut();
    }
    ENTRY_BTN.load(Ordering::Relaxed) as *mut u8
}

/// True when the Config input hook must run our page: our entry exists for this camp and the
/// cursor is on it (the camp is in the Config states 0x4A..0x4C only after our confirm).
pub fn page_active(camp: *mut u8) -> bool {
    // SAFETY: a live camp (Re:CoM just passed it to its own Config input function).
    !entry_of(camp).is_null() && unsafe { at::<i32>(camp, camp::CURSOR) } == top::OURS
}

/// Whether the camp should show our entry now: Combat = KH1, Sora's story, the normal top list,
/// and Keyblade numbers to list.
unsafe fn wanted(camp: *mut u8) -> bool {
    if super::settings().combat != crate::settings::Mode::Kh1 {
        return false;
    }
    if at::<i32>(camp, camp::RIKU) != 0 || func::<RikuFn>(RIKU_STORY)() & 0xFF != 0 || at::<i32>(camp, camp::DIRECT_MAP) != 0 {
        return false;
    }
    if super::stats::keyblade_cards().is_empty() {
        if !NO_NUMBERS_LOGGED.swap(true, Ordering::Relaxed) {
            log("keyblades: kh1-stats.json has no Keyblades (format 3, kh1-recom-build); no camp entry");
        }
        return false;
    }
    true
}

/// Logs, once per session, whether `save + 0x246B ..= 0x2817` is zero apart from our byte (the
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
        log(&format!("keyblades: save+{:#x}..={:#x} is all zero apart from our byte (save+{:#x} = {ours:#x})", kb::SPARE_FIRST, kb::SPARE_LAST, kb::EQUIP_OFFSET));
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

/// Our entry, made as `0x1400E3DE0` makes Re:CoM's (mode 0, first time) at index 7.
unsafe fn create_entry(camp: *mut u8) {
    let set = set_of(camp);
    let btn = func::<NewFn>(NEW)(0x170);
    if btn.is_null() {
        return;
    }
    func::<CtorFn>(SEQ_CTOR)(btn);
    put::<u8>(btn, 0x16E, 0xC);
    put::<u8>(btn, 0x138, 1);
    vfn::<SeqInitFn>(btn, 0x8)(btn, set, 4, 0.0, 0);
    let pos = Vec4([ENTRY_X, ENTRY_Y0 + ENTRY_STEP * top::OURS as f32, 0.0, 1.0]);
    func::<PosFn>(LAYOUT_POS)(btn, pos.0.as_ptr());
    let mut fnt = func::<NewFn>(NEW)(0xB0);
    if !fnt.is_null() {
        let colour = LABEL_NORMAL;
        fnt = func::<FntCtorFn>(FNT_CTOR)(fnt, 0, 0, &colour, 0xF, text::ENTRY_LABEL.as_ptr(), 0x40);
    }
    vfn::<AttachFn>(btn, 0x60)(btn, set, fnt, 0.0, 6.0, 0.0);
    ENTRY_CAMP.store(camp as usize, Ordering::Relaxed);
    ENTRY_BTN.store(btn as usize, Ordering::Relaxed);
    ENTRY_FNT.store(fnt as usize, Ordering::Relaxed);
    if !ENTRY_LOGGED.swap(true, Ordering::Relaxed) {
        log(&format!("keyblades: camp entry 8 \"Keyblades\" added (equipped: card {:#x})", super::stats::equipped_card()));
    }
}

/// Our entry's label (the button's text item), for the highlight colour.
static ENTRY_FNT: AtomicUsize = AtomicUsize::new(0);

/// Deletes our entry. Re:CoM's teardown deletes only its buttons; their text items go with the
/// buttons' sequences (Re:CoM's relabel path clears the old sequence's item pointer before
/// releasing it, `0x140246E90`). Assumption (inferred, not traced): the same holds for ours.
unsafe fn delete_entry() {
    let btn = ENTRY_BTN.swap(0, Ordering::Relaxed) as *mut u8;
    ENTRY_FNT.store(0, Ordering::Relaxed);
    ENTRY_CAMP.store(0, Ordering::Relaxed);
    delete(btn);
}

/// Hook on `0x1400E3DE0(camp, mode, level)` (the entry build): after Re:CoM's own work, ours.
pub unsafe extern "win64" fn entry_build(camp: *mut u8, mode: i32, level: u64) -> u64 {
    let o: BuildFn = orig(&ORIG_BUILD);
    let r = o(camp, mode, level);
    if camp.is_null() {
        return r;
    }
    spare_check();
    if at::<i32>(camp, camp::LEVEL) != 0 {
        // Deck submenu: ours stays as Re:CoM leaves its unused 7th button (faded by the confirm).
        return r;
    }
    let ours = entry_of(camp);
    match mode {
        0 => {
            let want = wanted(camp);
            if want && ours.is_null() {
                if ENTRY_CAMP.load(Ordering::Relaxed) != 0 {
                    // A camp we never saw torn down: forget its entry (not ours to delete).
                    ENTRY_CAMP.store(0, Ordering::Relaxed);
                    ENTRY_BTN.store(0, Ordering::Relaxed);
                }
                create_entry(camp);
            } else if !want && !ours.is_null() {
                delete_entry();
                log("keyblades: camp entry removed (Combat = Re:CoM)");
            } else if !ours.is_null() {
                anim(ours, set_of(camp), 4);
            }
        }
        1 if !ours.is_null() => anim(ours, set_of(camp), 5),
        2 if !ours.is_null() => anim(ours, set_of(camp), 6),
        _ => {}
    }
    r
}

/// The button and label of top entry `i` (ours at 7).
unsafe fn entry_parts(camp: *mut u8, i: i32) -> (*mut u8, *mut u8) {
    if i == top::OURS {
        return (ENTRY_BTN.load(Ordering::Relaxed) as *mut u8, ENTRY_FNT.load(Ordering::Relaxed) as *mut u8);
    }
    if !(0..top::RECOM_ENTRIES).contains(&i) {
        return (std::ptr::null_mut(), std::ptr::null_mut());
    }
    (at(camp, camp::BUTTONS + 8 * i as usize), at(camp, camp::LABELS + 8 * i as usize))
}

/// What `0x1400DEA10` does, for a move from or to our entry: the old entry back to animation 5,
/// label 0x80404040, priority 0xC, help cleared; the new one (when `show`) animation 3, label
/// 0x80C0C0C0, priority 0xD, highlight bar and glove at its row, the cursor index and its help.
unsafe fn cursor_set_ours(camp: *mut u8, old: i32, new: i32, show: bool) -> u64 {
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
        func::<CampIntFn>(HIGHLIGHT_BAR)(camp, new, 0);
        func::<CampIntFn>(GLIDE_TARGET)(camp, new, 0);
        put::<i32>(camp, camp::CURSOR, new);
        if new == top::OURS {
            set_help(camp, text::ENTRY_HELP.as_ptr());
        } else {
            set_help(camp, message(TOP_HELP_BASE + new as u32));
        }
    }
    1
}

/// Hook on `0x1400DEA10(camp, index, show)` (the camp's cursor set): with our entry, 8 entries.
pub unsafe extern "win64" fn cursor_set(camp: *mut u8, idx: i32, show: i32) -> u64 {
    let o: CursorSetFn = orig(&ORIG_CURSOR);
    if entry_of(camp).is_null() || at::<i32>(camp, camp::LEVEL) != 0 {
        return o(camp, idx, show);
    }
    let old = at::<i32>(camp, camp::CURSOR);
    let new = top::wrap(idx);
    if old != top::OURS && new != top::OURS {
        return o(camp, new, show);
    }
    cursor_set_ours(camp, old, new, show != 0)
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

/// Header item 0 with our title: `0x1400E4220` mode 1 (header present), with our text where
/// Re:CoM puts `0x1400D3890`'s (which is "" for index 7).
unsafe fn header_title(camp: *mut u8) {
    let set = set_of(camp);
    let hdr: *mut u8 = at(camp, camp::HEADER);
    if hdr.is_null() {
        // Re:CoM makes the header at camp open (state 3); not seen missing. Its own call, then our
        // text on the new item. Assumption: the item reads its text pointer when drawn.
        func::<HeaderFn>(HEADER)(camp, 1, 0);
        let t: *mut u8 = at(camp, camp::HEADER_TITLE);
        if !t.is_null() {
            func::<TextFn>(TEXT_SET)(t, text::TITLE.as_ptr());
        }
        log("keyblades: header was missing at confirm; made by Re:CoM, title set afterwards");
        return;
    }
    anim(hdr, set, 7);
    delete(at(camp, camp::HEADER_TITLE));
    put::<usize>(camp, camp::HEADER_TITLE, 0);
    let mut fnt = func::<NewFn>(NEW)(0xB0);
    if !fnt.is_null() {
        let colour: u32 = at(camp, camp::TEXT_COLOUR);
        fnt = func::<FntCtorFn>(FNT_CTOR)(fnt, 0, 0, &colour, 0xF, text::TITLE.as_ptr(), 0x40);
    }
    put::<usize>(camp, camp::HEADER_TITLE, fnt as usize);
    vfn::<ReplaceFn>(hdr, 0x68)(hdr, 1, set, fnt);
    func::<SprIntFn>(VISIBLE)(hdr, 1);
    put::<i32>(camp, camp::HEADER_MODE, 2);
}

/// Re:CoM's confirm sequence for an entry with a page (state 0xF, `0x1400DAB0A..0x1400DACAC`),
/// for ours. Re:CoM's own update ran this frame and already eased the glove (`0x1400DE100`,
/// which no step here changes the target of).
unsafe fn confirm_ours(camp: *mut u8) {
    let set = set_of(camp);
    func::<GuideFn>(GUIDE)(0, 0);
    func::<VoidFn>(CONFIRM_FLASH)(camp);
    sound(2);
    cursor_set_ours(camp, top::OURS, top::OURS, false);
    let n = at::<i32>(camp, camp::COUNT).clamp(0, top::RECOM_ENTRIES);
    for i in 0..n {
        anim(at(camp, camp::BUTTONS + 8 * i as usize), set, 6);
    }
    anim(ENTRY_BTN.load(Ordering::Relaxed) as *mut u8, set, 6);
    header_title(camp);
    let hdr: *mut u8 = at(camp, camp::HEADER);
    if !hdr.is_null() {
        func::<SprIntFn>(VISIBLE)(hdr, 1);
    }
    anim(at(camp, camp::PANEL_140), set, 0xC);
    func::<CampIntFn>(CAMP_PANEL)(camp, 2, 0);
    let main: *mut u8 = at(camp, camp::MAIN_PANEL);
    if !main.is_null() {
        func::<SlotVisFn>(SLOT_VISIBLE)(main, 2, 0);
    }
    put::<i32>(camp, camp::STATE, 0x11);
    put::<i32>(camp, camp::SAVED, top::OURS);
    func::<CampIntFn>(SIDE_PANEL)(camp, 4, 0);
}

/// Hook on `0x1400D9C80` (the camp update). Re:CoM's update always runs; with our entry, it
/// adds what Re:CoM's 7-entry code can't do for index 7.
pub unsafe extern "win64" fn camp_update(camp: *mut u8) {
    let o: VoidFn = orig(&ORIG_UPDATE);
    let ours = entry_of(camp);
    if ours.is_null() {
        return o(camp);
    }
    let state = at::<i32>(camp, camp::STATE);
    let cursor = at::<i32>(camp, camp::CURSOR);
    match state {
        // Re:CoM gives every entry animation 5 once button 0's animation has ended (then, in 0xD,
        // re-highlights the cursor's entry).
        7 | 0xD => {
            let b0: *mut u8 = at(camp, camp::BUTTONS);
            let was_ended = ended(b0);
            o(camp);
            if was_ended && !(state == 0xD && at::<i32>(camp, camp::CURSOR) == top::OURS) {
                anim(ours, set_of(camp), 5);
            }
        }
        0xF => {
            let fix = top::fix(cursor, top_pad());
            o(camp);
            let now = at::<i32>(camp, camp::STATE);
            if now == 0xF && at::<i32>(camp, camp::CURSOR) == cursor {
                match fix {
                    top::Fix::MoveToOurs => {
                        cursor_set_ours(camp, cursor, top::OURS, true);
                        sound(1);
                        func::<VoidFn>(GLIDE_STEP)(camp);
                    }
                    top::Fix::ConfirmOurs => confirm_ours(camp),
                    top::Fix::None => {}
                }
            } else if (now == 0x10 || now == 0x11) && cursor != top::OURS {
                // Re:CoM confirmed one of its entries: step 5 (every button animation 6) for ours.
                anim(ours, set_of(camp), 6);
            }
        }
        // State 0x11 reads a 7-byte table (`1D 16 41 1A 44 47 4A`): index 7 would read 0x2E, a
        // deck-editor state. Ours opens through the Config states 0x4A..0x4C.
        0x11 if cursor == top::OURS => {
            o(camp);
            let now = at::<i32>(camp, camp::STATE);
            if now != 0x11 {
                if now != 0x2E && !ODD_STATE_LOGGED.swap(true, Ordering::Relaxed) {
                    log(&format!("keyblades: state 0x11 left to {now:#x} for index 7 (0x2E expected); opening our page anyway"));
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
        if let Ok(mut g) = PAGE.lock() {
            if g.as_ref().is_some_and(|p| p.camp == camp as usize) {
                if let Some(p) = g.take() {
                    p.delete_all();
                }
            }
        }
        if !entry_of(camp).is_null() {
            delete_entry();
        }
    }
    o(camp)
}

// ---------------------------------------------------------------------------------------------
// The page

/// Our page's objects and state (never in camp fields).
struct Page {
    camp: usize,
    sub: u8,
    stats_panel: usize,
    list_panel: usize,
    cursor: usize,
    bar: usize,
    rows: [usize; 8],
    stat_rows: [usize; 3],
    cards: Vec<u16>,
    list: list::List,
    equipped: u16,
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

    unsafe fn open(camp: *mut u8) -> Page {
        let cards = kb::unlocked(super::stats::keyblade_cards(), |k| ever_obtained(k));
        let equipped = super::stats::equipped_card();
        let at_eq = cards.iter().position(|&c| c == equipped).unwrap_or(0) as i32;
        let mut p = Page {
            camp: camp as usize,
            sub: 1,
            stats_panel: 0,
            list_panel: 0,
            cursor: 0,
            bar: 0,
            rows: [0; 8],
            stat_rows: [0; 3],
            list: list::open_on(cards.len() as i32, at_eq),
            cards,
            equipped,
        };
        let set = set_of(camp);
        p.stats_panel = p.panel(0x10, 3, 0x125);
        p.list_panel = p.panel(0x11, 2, 0x14C);
        let c = func::<NewFn>(NEW)(0x1C0);
        if !c.is_null() {
            p.cursor = func::<CursorCtorFn>(CURSOR_CTOR)(c, 0x12, p.list_panel as *mut u8, 0x12D) as usize;
        }
        let b = func::<NewFn>(NEW)(0x28);
        if !b.is_null() {
            let bar = func::<CtorFn>(BAR_CTOR)(b);
            let mut pos = Vec4([133.0, -56.0, 0.0, 1.0]);
            func::<BarSetupFn>(BAR_SETUP)(bar, set, 0x14, va(base(), BAR_SEQ) as *const u16, pos.0.as_mut_ptr(), 157.0, p.list_panel as *mut u8);
            p.bar = bar as usize;
        }
        p.redraw();
        log(&format!("keyblades: page opened: {} Keyblades listed {:x?}, equipped {:#x}", p.cards.len(), p.cards, p.equipped));
        p
    }

    /// Draw mode 9 (rows, cursor, scroll bar), plus the status rows for the highlighted Keyblade.
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
        let blink: u32 = at(camp, camp::BLINK);
        let setup = func::<SetupFn>(CBUTTON_SETUP);
        let seqs = va(base(), LIST_SEQ) as *const u16;
        for i in 0..list::VISIBLE as usize {
            delete(self.rows[i] as *mut u8);
            self.rows[i] = 0;
            let idx = self.list.top + i as i32;
            if idx >= self.list.count {
                continue;
            }
            let card = self.cards[idx as usize];
            let pos = self.layout(0x12, 2 + i as u64, 0).unwrap_or_default();
            let btn = func::<NewFn>(NEW)(0x30);
            if btn.is_null() {
                continue;
            }
            func::<CtorFn>(CBUTTON_CTOR)(btn);
            self.rows[i] = btn as usize;
            let name = card_name(card);
            setup(btn, set, 0x11, seqs, pos.0.as_ptr(), self.list_panel as *mut u8, name.as_ptr(), font, f9, f10, 0x40, &colour, 0);
            let spr: *mut u8 = at(btn, 0x18);
            if card == self.equipped && !spr.is_null() {
                let m = vfn::<MarkFn>(spr, 0x40)(spr, set, EQUIPPED_MARK, MARK_X, MARK_Y, 0.0, 0);
                if !m.is_null() {
                    vfn::<ObjIntFn>(m, 0x30)(m, blink);
                }
            }
        }
        self.place_cursor();
        if self.bar != 0 {
            let (t, s) = list::bar(self.list);
            func::<BarUpdateFn>(BAR_UPDATE)(self.bar as *mut u8, t, s);
        }
        self.stats();
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

    /// The status rows (draw mode 0's label / value rows, layout group 0x11 elements 4.., y + 8)
    /// for the highlighted Keyblade: Strength, Magic, reach. Rebuilt on each move, as the list
    /// rows are.
    unsafe fn stats(&mut self) {
        let camp = self.p();
        let set = set_of(camp);
        let no_scale = at::<i32>(camp, camp::NO_SCALE) != 0;
        let y = if no_scale { 0.0 } else { 1.0 };
        let values = self
            .cards
            .get(self.list.cursor as usize)
            .and_then(|&c| super::stats::keyblade(c))
            .map(kb::stat_values)
            .unwrap_or_else(|| [String::new(), String::new(), String::new()]);
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
            setup(btn, set, 0x10, seqs, pos.0.as_ptr(), self.stats_panel as *mut u8, text::STAT_LABELS[i].as_ptr(), 0xE, 0.0, y, 0x40, std::ptr::null(), i as i32);
            let mut v = func::<NewFn>(NEW)(0xB0);
            if !v.is_null() {
                let colour = VALUE_COLOUR;
                v = func::<FntCtorFn>(FNT_CTOR)(v, 0, 0, &colour, 0xE, std::ptr::null(), 0x41);
                func::<TextFn>(TEXT_COPY)(v, kb::c_text(value).as_ptr());
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
            set_help(self.p(), text::PAGE_HELP.as_ptr());
        }
        self.glide();
        done
    }

    /// Sub-state 2: the list's input, in `0x1400EB7E0`'s order. Returns the page's result.
    unsafe fn input(&mut self) -> i32 {
        let camp = self.p();
        self.glide();
        if func::<CampBtnFn>(CAMP_CANCEL)(camp, 0) & 0xFF != 0 {
            // Draw mode 4, sound 4, sub-state 3.
            let set = set_of(camp);
            anim(self.stats_panel as *mut u8, set, 0x127);
            anim(self.list_panel as *mut u8, set, 0x14E);
            if self.cursor != 0 {
                func::<VoidFn>(CURSOR_HIDE)(self.cursor as *mut u8);
            }
            set_help(camp, message(rd::<u32>(va(base(), CLOSE_HELP_ID))));
            sound(4);
            self.sub = 3;
            return 1;
        }
        if func::<CampBtnFn>(CAMP_DECIDE)(camp, 0) & 0xFF != 0 {
            if let Some(&card) = self.cards.get(self.list.cursor as usize) {
                super::stats::set_equipped(card);
                self.equipped = super::stats::equipped_card();
                sound(2);
                self.redraw();
                log(&format!("keyblades: equipped card {card:#x} (save byte {:#x})", kb::save_byte_for(card)));
            }
            return 0;
        }
        let dir = func::<CampDirFn>(CAMP_DIR);
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
        if moved {
            sound(1);
            self.redraw();
        }
        0
    }
}

/// The Keyblade's localized card name (`0x140164CA0`, message `[0x140528560 + kind × 4]`), or the
/// stats file's name when that message isn't loaded.
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

/// Re:CoM's "obtained at least once" (`0x14015B3F0`), when the collection is there.
unsafe fn ever_obtained(id: u16) -> bool {
    let coll: usize = rd(va(base(), COLLECTION));
    coll != 0 && func::<EverFn>(EVER_OBTAINED)(id as u32) & 0xFF != 0
}

/// Our page, called by the Config input hook in place of `0x1400D3EB0` when [`page_active`]:
/// the same contract (≥ 0 running, < 0 closed). Re:CoM's states 0x4A..0x4C around it.
pub unsafe fn page(camp: *mut u8) -> u64 {
    let Ok(mut g) = PAGE.lock() else { return (-1i32) as u32 as u64 };
    if g.as_ref().is_some_and(|p| p.camp != camp as usize) {
        if let Some(stale) = g.take() {
            stale.delete_all();
        }
    }
    let r: i32 = match g.as_mut() {
        None => {
            // State 0x4A's call (sub-state 0): build, list, rows.
            *g = Some(Page::open(camp));
            0
        }
        Some(p) => match p.sub {
            1 => {
                if p.opening() {
                    p.sub = 2;
                    1
                } else {
                    0
                }
            }
            2 => p.input(),
            _ => {
                // Sub-state 3: when the status panel's closing ends, delete everything.
                if ended(p.stats_panel as *mut u8) {
                    if let Some(p) = g.take() {
                        p.delete_all();
                    }
                    -1
                } else {
                    0
                }
            }
        },
    };
    r as u32 as u64
}
