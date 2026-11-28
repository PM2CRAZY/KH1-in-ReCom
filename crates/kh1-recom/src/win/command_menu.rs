//! KH1's command menu (M7 stage 7.3; `crate::command_menu`, `docs/recom-m7-command-menu.md`),
//! drawn by Re:CoM.
//!
//! - **Objects:** one `D2::C2DSeqCtrl` per visible row (Re:CoM's `operator new` and ctor), with
//!   one layer of `Cockpit:00` (Re:CoM's text plate, or the same plate in the highlight colour:
//!   the two groups the asset step added after the MP bar's) and a `C2DFnt` with KH1's word,
//!   attached as Re:CoM's `CStockFolder` attaches its labels to these plates (`0x1401658F0`: font
//!   0xE, flags 0x80, vtable `+0x60` at (0, 0) with the screen offset − 4). Placed with Re:CoM's
//!   layout position (`0x140248660`, the 512 × 384 space) at KH1's coordinates, shifted by the
//!   player gauge's x offset as every HUD layer (`+0xA0`). Re:CoM's 2D manager animates and draws
//!   them; the fade is Re:CoM's control colour (`0x140248590`, `+0x138` = 1) and the text colour
//!   (`C2DFnt +0x34`).
//! - **Context tab** (FM, `0x140278B40`): never a row. KH1's look: three controls of the
//!   `kh1hud` set (the tab's four pieces, the Triangle, the word) at the tab's origin, moved by
//!   its slide and faded by its own opacity; Re:CoM's look: the highlighted plate tinted green
//!   with a `C2DFnt`, above the rows. Made after the rows (later controls draw over earlier ones
//!   of the same priority: Assumption, as the MP bar's pieces), so a row rebuild remakes it.
//! - **When:** made with the player gauge (the MP bar's wrapper on `CPlayerGauge` init), hidden
//!   and shown with Re:CoM's HUD (its second-bar state wrapper), deleted before the gauge's own
//!   controls (its dtor wrapper). Each frame of Sora's at the KH1 setting the request-bits wrapper
//!   runs KH1's menu step ([`frame`]): pad and build, then the rows are rebuilt only when what
//!   they show changed (a cursor move, a list opened, a word changed), and only colours are
//!   written while a fade or a brightness change runs.
//! - Only at Combat = KH1 in Sora's story (`super::mp::active`), with the asset step's
//!   `kh1-command-menu.json` and the plate groups in the loaded `Cockpit:00`.
//! - **Shortcuts** (stage 7.12): while L1 is held (Re:CoM's held pad bit 4, `0x14029C1D0`) and a
//!   slot is set, KH1's shortcut list takes the root's place: KH1's panel (`kh1hud` groups, in
//!   both menu looks) at KH1's place, Triangle / Square / confirm cast slot 1 / 2 / 3 through
//!   `super::magic::issue`, and the request-bits wrapper drops Attack, Triangle and Square. The
//!   save and warp points read Triangle from the pad themselves (`0x14029B550`): their two calls
//!   are retargeted to [`point_triangle`], which says "not pressed" while L1 holds the list (KH1:
//!   Triangle is shortcut 1 there, never the save point's command).
//! - **Items** (M8 stage 8a.1): the Items row opens KH1's Items list of Sora's filled slots
//!   (`super::items::slots`); KH1's look draws it with KH1's `item` panels (`kh1-hud.json`'s
//!   `items_list`; the Magic list's if an older asset step made none), Re:CoM's look with the rows'
//!   plates; the words are the menu's. A chosen item goes to `super::items::issue`.

use super::movement::{self as mv, body, func, p, rd, rd_vec, wr};
use super::{base, log};
use crate::command_menu::{self as cm, Event, Kind, Menu, Pad, RowView, Scene, ShortResult, SlotView, TabView, Words, World};
use crate::kh1_hud as kh;
use crate::sites::COMMAND_MENU_PTRS;
use crate::units::to_kh1;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

static INSTALLED: AtomicBool = AtomicBool::new(false);
/// A menu is built on some live gauge (the per-frame step's only test otherwise).
static LIVE: AtomicBool = AtomicBool::new(false);
static WORDS: OnceLock<Words> = OnceLock::new();
static ALL_SPELLS: AtomicBool = AtomicBool::new(false);
static LOG_LEFT: AtomicU32 = AtomicU32::new(60);
/// Row rebuilds so far (each deletes the shown rows and makes the new ones): the 2D census is
/// logged every 100th.
static REBUILDS: AtomicU32 = AtomicU32::new(0);
/// KH1's MP as last read or written (`super::mp`), packed; bit 63 = known.
static MP_VIEW: AtomicU64 = AtomicU64::new(0);
static EVENTS: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// Stage 7.12: the menu runs, shows, and its shortcut list is available (a slot set, a spell
/// learned): L1 held then means the shortcut list, for [`point_triangle`]. Written each frame of
/// the menu step.
static SHORT_READY: AtomicBool = AtomicBool::new(false);
/// "No shortcut panel art" logged once.
static SHORT_ART_LOGGED: AtomicBool = AtomicBool::new(false);
static POINT_LOGGED: AtomicBool = AtomicBool::new(false);
static STATE: Mutex<State> = Mutex::new(State { menu: None, uis: Vec::new() });

// Re:CoM functions (RVAs; first bytes checked in `sites`).
const NEW: u32 = 0x004A_CE54;
const SEQ_CTOR: u32 = 0x0024_7260;
const LAYOUT_POS: u32 = 0x0024_8660;
const SCALE: u32 = 0x0024_86D0;
const COLOUR: u32 = 0x0024_8590;
const FNT_CTOR: u32 = 0x0024_3F10;
const SOUND: u32 = 0x002A_A360;
const PAD_FRESH: u32 = 0x0029_B2F0;
const PAD_CANCEL: u32 = 0x0029_B410;
const PAD_CONFIRM: u32 = 0x0029_B490;
/// Re:CoM's held-buttons test (`0x14029C1D0(mask, port)`: all of `mask` held; `sites`, the
/// movement's), and L1's bit (`0x1401FEFE0`'s card input reads it; PS2 order byte-swapped).
const BUTTONS_HELD: u32 = 0x0029_C1D0;
const L1: u32 = 4;
/// The save / warp points' fresh-Triangle read (`0x14029B550(port)`: pad bit 0x10 fresh, 0 / 1 in
/// eax), called through [`point_triangle`] (stage 7.12).
const POINT_TRIANGLE: u32 = 0x0029_B550;
/// Re:CoM's confirm button (u16, PS2 order; `0x140346BB0` returns it, `sites::COMMAND_MENU_CHECKED`):
/// the shortcut panel's third glyph.
const CONFIRM_KEY: u32 = 0x0083_F3F0;
/// Re:CoM's pad type (u16; its controller config `0x14083F574` (`0x14034E930`) `+0x1C`): the
/// button glyph set `0x140349120` shows (1 Xbox, 2 PlayStation, 3 a generic pad;
/// `sites::COMMAND_MENU_CHECKED`). The shortcut panel's and the context tab's glyphs follow it.
const PAD_TYPE: u32 = 0x0083_F590;

/// Re:CoM's pad type now ([`PAD_TYPE`]).
unsafe fn pad_type() -> u16 {
    rd::<u16>(base() + PAD_TYPE as usize)
}
/// CGimmickDoor's type getter (its vtable `+0x28`); the others are `mov eax, imm32; ret`.
const DOOR_TYPE: u32 = 0x001B_8A10;
const SEQ_CTRL_VTABLE: u32 = 0x0060_7340;

/// `CPlayerGauge` fields (as `win/mp_bar.rs`): the x offset, the display state.
const G_XOFF: usize = 0xA0;
const G_STATE: usize = 0xAA;
const STATE_SHOW: u32 = 1;
const STATE_HIDDEN: u32 = 7;
/// `C2DSeqCtrl`: draw priority, "use the colour" (`0x14024BB10` multiplies by `+0x120` when set).
const C_PRIORITY: usize = 0x16E;
const C_COLOUR_ON: usize = 0x138;
/// The root rows at the HUD's priority (the HP gauge's 2), the Magic list over them.
const PRIORITY_ROOT: u8 = 2;
const PRIORITY_SUB: u8 = 3;
/// `C2DFnt`: its colour (`0x140258BC0`), as `CStockFolder` makes it (`0x1401658F0`: font 0xE,
/// flags 0x80, white 0x80808080) and attaches it ((0, 0), screen offset −4 − the HUD's).
const F_COLOUR: usize = 0x34;
const FONT: i32 = 0xE;
const FONT_FLAGS: i32 = 0x80;
const TEXT_DX: f32 = -4.0;
/// Player fields: the action-target handles the field action reads (`0x1401E80C0`), the
/// treasure box's opened bit (`0x1401E80C0`: `+0x2C1 & 1`).
const ACTION_TARGET: usize = 0x5FC;
const BOX_OPENED: usize = 0x2C1;
/// The save / warp points' updates (`0x1401C5180`, `0x1401CC280`): the lock-on handle they read
/// first (`player+0x850`, written by `0x1401E8DA0`), the save point's "Sora in range" byte
/// (`+0x2C0`), and the flag that stops the warp point (`0x14087B84C & 1`, `0x1402345D0`).
/// Checked (decomp + exe: `sites::COMMAND_MENU_CHECKED`).
const LOCK_ON: usize = 0x850;
const SAVE_IN_RANGE: usize = 0x2C0;
const WARP_BLOCK: u32 = 0x0087_B84C;

type NewFn = unsafe extern "win64" fn(usize) -> usize;
type CtorFn = unsafe extern "win64" fn(usize) -> usize;
type VecFn = unsafe extern "win64" fn(usize, *const f32);
type FntCtorFn = unsafe extern "win64" fn(usize, i32, i32, *const u32, i32, *const u8, i32) -> usize;
type SoundFn = unsafe extern "win64" fn(u32, i32) -> u64;
type PadFn = unsafe extern "win64" fn(u32, u32) -> u32;
type PlayFn = unsafe extern "win64" fn(usize, u32, f32);
/// H4: play a layer from a time (`sites::KH1_HUD_CALLEES`), units of 1/50 frame.
const PLAY_FROM: u32 = 0x0024_8550;
const UNITS: f32 = 50.0;
type PadBtnFn = unsafe extern "win64" fn(u32) -> u32;
type HeldFn = unsafe extern "win64" fn(u32, u32) -> u32;

#[repr(C, align(16))]
struct V4([f32; 4]);

struct Row {
    ctrl: usize,
    fnt: usize,
    view: RowView,
    /// H4 (KH1's look): the word's control (KH1's menu font, a group of the `kh1hud` set).
    text: usize,
    /// 0.179.1 (KH1's look): a DP sleight's mark right of its word (0: none).
    icon: usize,
}

/// 0.179.1: the gap (units) between a DP sleight's name and its mark.
const DP_ICON_GAP: f32 = 3.0;

/// Word `name` is a DP sleight's (`crate::dark_points::DP_SLEIGHTS`; a sleight's word is
/// `crate::sleights::WORD_BASE` + its id).
fn dp_word(name: u16) -> bool {
    name & 0xFF00 == crate::sleights::WORD_BASE && crate::dark_points::pays_dp(name - crate::sleights::WORD_BASE)
}

/// The context tab's controls: KH1's look (plate, Triangle, word), or Re:CoM's (one plate with its
/// `C2DFnt`).
struct TabCtl {
    /// KH1's look: plate, Triangle, word (0: not made); Re:CoM's: the plate.
    ctrls: Vec<usize>,
    /// Re:CoM's look: the plate's font.
    fnt: usize,
    view: TabView,
}

/// Re:CoM's look's tab: the highlighted plate (Re:CoM's magic-plate blue, `0x80FFA200`) times
/// this colour reads green (KH1's tab is (110, 210, 100)); a multiply can't add the red. Assumption
/// (Re:CoM's control colour is a multiply, factors ≤ 1 used: `docs/recom-m7-command-menu.md`).
const RECOM_TAB_TINT: [f32; 3] = [1.0, 1.0, 0.39];
/// Re:CoM's look's tab: its plate's top-left, as KH1's green plate sits over its first row
/// (KH1: 28 units above the first row's top, 8 left of the list; Re:CoM's look has no header,
/// its first row at the list's y).
const RECOM_TAB_AT: (f32, f32) = (cm::ROOT_X - 8.0, cm::ROOT_Y - 28.0);

/// H4: KH1's look of the menu (`hud = kh1`): the list headers and which look is shown.
struct Look {
    battle: bool,
    /// The root's header control, and the Magic list's when it is open.
    headers: Vec<usize>,
    /// The root header dims while a submenu is open (`0x140278B40`: × 0.5).
    dim: bool,
}

struct Ui {
    gauge: usize,
    /// Re:CoM's draw list the gauge was made on: the rows' layers are added and released on it
    /// (`super::d2`).
    list: i32,
    xoff: f32,
    hidden: bool,
    rows: Vec<Row>,
    shown: Option<Scene>,
    look: Option<Look>,
    tab: Option<TabCtl>,
    /// Stage 7.12: the shortcut panel's controls while the shortcut list is on top.
    short: Option<ShortCtl>,
    /// 0.111.0: the pointing arrow's control (0: none).
    arrow: usize,
}

/// The shortcut panel (stage 7.12): the frame + title, the three glyphs, the three words (0: not
/// made), and what they show.
struct ShortCtl {
    panel: usize,
    glyphs: [usize; 3],
    words: [usize; 3],
    /// 0.179.1: a DP sleight's mark right of its word (0: none).
    icons: [usize; 3],
    view: [SlotView; 3],
    alpha: f32,
}

struct State {
    menu: Option<Menu>,
    uis: Vec<Ui>,
}

fn note(line: impl FnOnce() -> String) {
    if LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("command menu: {}", line()));
    }
}

/// Squeeze to fit: a squeezed word logged once per word and box (when it is first drawn).
fn squeezed(line: impl FnOnce() -> String) {
    static SEEN: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let l = line();
    if let Ok(mut g) = SEEN.lock() {
        if g.len() < 64 && !g.contains(&l) {
            log(&format!("command menu: squeezed to fit: {l}"));
            g.push(l);
        }
    }
}

fn event(e: String) {
    if let Ok(mut g) = EVENTS.lock() {
        if g.len() < 16 {
            g.push(e);
        }
    }
}

/// Reads the words, checks the vtable slot. The menu needs the MP bar's wrappers and its plates.
pub fn install(b: usize, manifest: Option<&std::path::Path>, s: &crate::settings::Settings) -> bool {
    let Some(path) = manifest else {
        log("command menu: no kh1-command-menu.json (asset step not run?): no command menu");
        return false;
    };
    let words = match std::fs::read_to_string(path).map_err(|e| e.to_string()).and_then(|t| cm::parse_words(&t, &s.menu_language)) {
        Ok(w) => w,
        Err(e) => {
            log(&format!("command menu: {}: {e}; no command menu", path.display()));
            return false;
        }
    };
    // SAFETY: reads of the mapped image (the build was checked at start).
    unsafe {
        for p in COMMAND_MENU_PTRS {
            let have: usize = rd(b + p.rva as usize);
            if have != b + p.target as usize {
                log(&format!("command menu: {} holds {have:#x}, not {:#x}; no command menu", p.name, b + p.target as usize));
                return false;
            }
        }
    }
    // 0.17.x checked `menu_groups()` here, which is None until a bar is built (later, on the
    // first player gauge): the menu was never installed (owner, 2026-10-31: "The command menu
    // isnt there").
    if !super::mp_bar::has_menu_groups() {
        log("command menu: kh1-mp-bar.json has no plate groups (an older asset step): no command menu");
        return false;
    }
    ALL_SPELLS.store(s.kh1_all_spells, Ordering::Relaxed);
    let folded: String = words.folded.iter().collect();
    // SAFETY: a static pointer of the image.
    let lang_index = unsafe {
        let g: usize = rd(super::va(base(), crate::sites::data::FONT_GLOBAL));
        if g == 0 {
            -1
        } else {
            rd::<i32>(g + 0x274)
        }
    };
    log(&format!(
        "command menu: KH1's command menu at combat = kh1 in Sora's story (M7 stage 7.3): words {} ({} entries{}) from {}; Re:CoM language index {lang_index}; kh1_all_spells = {}",
        words.lang,
        words.by_id.len(),
        if folded.is_empty() { String::new() } else { format!(", written without accents: {folded}") },
        path.display(),
        crate::settings::switch_str(s.kh1_all_spells)
    ));
    let _ = WORDS.set(words);
    INSTALLED.store(true, Ordering::Release);
    // Stage 7.12: Triangle with L1 held is shortcut 1 at a save / warp point too.
    // SAFETY: executable memory of our own near the exe; the calls' bytes are checked before the
    // write.
    let [sp, wp] = unsafe { install_point_calls(b) };
    log(&format!(
        "command menu: KH1's shortcut list on L1 (M7 stage 7.12): slots at save+{:#x}; the save point's Triangle read 0x1401C530E {}, the warp point's 0x1401CC31B {}",
        crate::mp::BLOCK_FIRST + crate::mp::off::SHORTCUTS,
        if sp { "retargeted" } else { "NOT retargeted (Triangle there still saves with L1 held)" },
        if wp { "retargeted" } else { "NOT retargeted (Triangle there still warps with L1 held)" }
    ));
    true
}

/// The menu is in (KH1's specials are offered only in it).
pub fn installed() -> bool {
    INSTALLED.load(Ordering::Acquire)
}

/// KH1's MP as just read or written (`super::mp`): kept for the menu's greying, no lock.
pub fn mp_seen(view: (i32, i32, u16)) {
    let packed = 1u64 << 63 | (view.0 as u8 as u64) << 24 | (view.1 as u8 as u64) << 16 | view.2 as u64;
    MP_VIEW.store(packed, Ordering::Relaxed);
}

fn mp_view() -> Option<(i32, i32, u16)> {
    let v = MP_VIEW.load(Ordering::Relaxed);
    (v >> 63 != 0).then(|| ((v >> 24) as u8 as i32, (v >> 16) as u8 as i32, v as u16))
}

/// A new player gauge (its init and the MP bar have run, on draw list `list`): the menu is made
/// for it, fading in.
pub unsafe fn build(gauge: usize, list: i32) {
    if !INSTALLED.load(Ordering::Acquire) || !super::mp::live() || super::mp_bar::menu_groups().is_none() {
        return;
    }
    if mp_view().is_none() {
        let _ = super::mp::current();
    }
    let hidden = rd::<u16>(gauge + G_STATE) as u32 == STATE_HIDDEN;
    let ui = Ui { gauge, list, xoff: -rd::<f32>(gauge + G_XOFF), hidden, rows: Vec::new(), shown: None, look: None, tab: None, short: None, arrow: 0 };
    // M8 stage 8a.1, debug (`kh1_test_items`): a room or battle starts with a test inventory when
    // Sora's slots are all empty.
    super::items::hud_built();
    if let Ok(mut s) = STATE.lock() {
        for mut old in s.uis.extract_if(.., |u| u.gauge == gauge) {
            old.clear();
        }
        let m = s.menu.get_or_insert_with(Menu::new);
        // A new HUD (a room or a battle): back to the root and Attack, fading in (KH1's
        // `0x140278AC0(1)` when its step starts running again).
        m.hide();
        if !hidden {
            m.fade_in();
        }
        s.uis.push(ui);
        LIVE.store(true, Ordering::Release);
    }
    note(|| format!("built on the player gauge {gauge:#x}{}", if hidden { " (HUD hidden)" } else { "" }));
}

/// Re:CoM's HUD hides (7) or shows (1) the player gauge: KH1's instant hide / show.
pub unsafe fn hud(gauge: usize, state: u32) {
    if state != STATE_HIDDEN && state != STATE_SHOW {
        return;
    }
    let Ok(mut s) = STATE.lock() else { return };
    let State { menu, uis } = &mut *s;
    let was_all_hidden = uis.iter().all(|u| u.hidden);
    let Some(ui) = uis.iter_mut().find(|u| u.gauge == gauge) else { return };
    if state == STATE_HIDDEN && !ui.hidden {
        ui.hidden = true;
        ui.clear();
    } else if state == STATE_SHOW && ui.hidden {
        ui.hidden = false;
    }
    // The menu (shared by every live gauge: a field one may linger while a battle one shows)
    // hides when no gauge shows, and shows when one shows again.
    let all_hidden = uis.iter().all(|u| u.hidden);
    if let Some(m) = menu {
        if all_hidden && !was_all_hidden {
            m.hide();
        } else if !all_hidden && was_all_hidden {
            m.show();
        }
    }
}

/// The gauge goes (its dtor): our controls first.
pub unsafe fn destroy(gauge: usize) {
    if let Ok(mut s) = STATE.lock() {
        for mut u in s.uis.extract_if(.., |u| u.gauge == gauge) {
            u.clear();
        }
        LIVE.store(!s.uis.is_empty(), Ordering::Release);
        if s.uis.is_empty() {
            SHORT_READY.store(false, Ordering::Relaxed);
        }
    }
}

/// After the Config or the camp: the menu follows the setting; spells may have changed. `gauges`:
/// the live player gauges and their draw lists.
pub fn refresh(gauges: &[(usize, i32)]) {
    if !INSTALLED.load(Ordering::Acquire) {
        return;
    }
    if super::mp::live() {
        for &(g, list) in gauges {
            let have = STATE.lock().map(|s| s.uis.iter().any(|u| u.gauge == g)).unwrap_or(true);
            if !have {
                // SAFETY: a live gauge (the MP bar's list; its dtor removes it).
                unsafe { build(g, list) };
            }
        }
    } else if let Ok(mut s) = STATE.lock() {
        for mut u in s.uis.drain(..) {
            // SAFETY: live controls (removed in the gauge's dtor before Re:CoM frees anything).
            unsafe { u.clear() };
        }
        LIVE.store(false, Ordering::Release);
        SHORT_READY.store(false, Ordering::Relaxed);
        note(|| "combat is not KH1 here: menu removed".to_string());
    }
}

impl Ui {
    /// Every control of ours on this gauge.
    unsafe fn clear(&mut self) {
        self.clear_lists();
        self.clear_short();
    }

    /// The lists' controls (rows, headers, the tab).
    unsafe fn clear_lists(&mut self) {
        // The ctrl's dtor releases both layers; the text layer's object deletes its `C2DFnt`
        // (`crate::d2`), as `CStockFolder`'s labels go: the font is not deleted again here.
        for r in self.rows.drain(..) {
            super::d2::delete(r.ctrl, self.list);
            if r.text != 0 {
                super::d2::delete(r.text, self.list);
            }
            if r.icon != 0 {
                super::d2::delete(r.icon, self.list);
            }
        }
        if let Some(l) = self.look.take() {
            for h in l.headers {
                super::d2::delete(h, self.list);
            }
        }
        self.clear_tab();
        if self.arrow != 0 {
            super::d2::delete(self.arrow, self.list);
            self.arrow = 0;
        }
        self.shown = None;
    }

    /// 0.111.0: the pointing arrow (`cm::arrow`) beside the root's cursor row: Re:CoM's own arrow
    /// group in `Cockpit:00` (`super::mp_bar::arrow_group`), made with the rows (so Re:CoM plays
    /// its swing-in from frame 0 at each cursor move, then loops its sway), faded with the menu.
    /// Without the group (an older asset step) nothing is drawn, logged once.
    unsafe fn show_arrow(&mut self, a: Option<cm::ArrowView>, alpha: f32, kh1_look: bool) {
        if self.arrow != 0 {
            super::d2::delete(self.arrow, self.list);
            self.arrow = 0;
        }
        let Some(a) = a else { return };
        let Some((set, group)) = super::mp_bar::arrow_group() else {
            static LOGGED: AtomicBool = AtomicBool::new(false);
            if !LOGGED.swap(true, Ordering::Relaxed) {
                log("command menu: kh1-mp-bar.json has no pointing-arrow group (run this version's asset step): no arrow drawn");
            }
            return;
        };
        let c = func::<NewFn>(NEW)(0x170);
        if c == 0 {
            return;
        }
        func::<CtorFn>(SEQ_CTOR)(c);
        if rd::<usize>(c) != base() + SEQ_CTRL_VTABLE as usize {
            return;
        }
        super::d2::made();
        wr::<u8>(c + C_PRIORITY, PRIORITY_ROOT);
        wr::<u8>(c + C_COLOUR_ON, 1);
        let (dx, dy) = cm::arrow::at(kh1_look);
        func::<VecFn>(LAYOUT_POS)(c, V4([a.x + dx, a.y + dy, 0.0, 1.0]).0.as_ptr());
        let xoff = if kh1_look { -crate::kh1_hud::right_margin(-self.xoff) } else { self.xoff };
        super::d2::add(c, self.list, set, group, xoff, 1);
        func::<VecFn>(COLOUR)(c, V4([1.0, 1.0, 1.0, alpha]).0.as_ptr());
        self.arrow = c;
    }

    unsafe fn clear_short(&mut self) {
        if let Some(p) = self.short.take() {
            for c in [p.panel].into_iter().chain(p.glyphs).chain(p.words).chain(p.icons).filter(|&c| c != 0) {
                super::d2::delete(c, self.list);
            }
        }
    }

    /// Stage 7.12: KH1's shortcut panel (`0x14027B870` case 5) in both menu looks: the `kh1hud`
    /// set's frame + title at (left margin + 16, 282), the button glyphs at `cut_ico_a / b / c`'s
    /// places (Triangle, Square, then Re:CoM's confirm button: Cross or Circle), the slots' words in
    /// KH1's menu font at (+ 16, + 22 + 22 k), 0x73 greyed / 0xE6; every alpha the menu's fade.
    /// Made when the list comes on top (or a word changes), else only colours are written when the
    /// fade or a slot's greying changes. Without the panel's groups (an older asset step) nothing
    /// is drawn; the shortcuts still cast.
    unsafe fn show_short(&mut self, slots: &[SlotView; 3], alpha: f32) {
        if self.hidden {
            return;
        }
        if let Some(p) = self.short.as_mut() {
            if p.view.map(|v| v.name) == slots.map(|v| v.name) {
                if p.alpha != alpha {
                    for c in [p.panel].into_iter().chain(p.glyphs).filter(|&c| c != 0) {
                        func::<VecFn>(COLOUR)(c, V4([1.0, 1.0, 1.0, alpha]).0.as_ptr());
                    }
                }
                for k in 0..3 {
                    if p.words[k] != 0 && (p.alpha != alpha || p.view[k].bright != slots[k].bright) {
                        let b = kh1_word_level(slots[k].bright);
                        for c in [p.words[k], p.icons[k]].into_iter().filter(|&c| c != 0) {
                            func::<VecFn>(COLOUR)(c, V4([b, b, b, alpha]).0.as_ptr());
                        }
                    }
                }
                p.view = *slots;
                p.alpha = alpha;
                return;
            }
        }
        self.clear_short();
        let Some(m) = super::kh1_hud::manifest() else { return };
        let (Some(sg), Some(mg)) = (m.shortcut, m.menu.as_ref()) else {
            if !SHORT_ART_LOGGED.swap(true, Ordering::Relaxed) {
                log("command menu: kh1-hud.json has no shortcut panel (run this version's asset step): shortcuts cast, no panel drawn");
            }
            return;
        };
        if !super::kh1_hud::set_registered() {
            if !SHORT_ART_LOGGED.swap(true, Ordering::Relaxed) {
                log("command menu: kh1hud:00 is not registered (FO0002.SPR not served?): shortcuts cast, no panel drawn");
            }
            return;
        }
        use crate::kh1_hud::shortcut as g;
        use cm::shortcut as at;
        let lang = super::settings().menu_language.clone();
        let o = at::ORIGIN;
        let one = [1.0, 1.0, 1.0, alpha];
        let colour = |c: usize, col: [f32; 4]| {
            if c != 0 {
                func::<VecFn>(COLOUR)(c, V4(col).0.as_ptr());
            }
        };
        let panel = self.kh1_ctrl(PRIORITY_ROOT, o.0, o.1, 1.0, &[(sg.first + g::PANEL, None)]).unwrap_or(0);
        colour(panel, one);
        let confirm = if cm::shortcut::confirm_is_circle(rd::<u16>(base() + CONFIRM_KEY as usize)) { g::CIRCLE } else { g::CROSS };
        let mut glyphs = [0; 3];
        if sg.icons {
            let pad = pad_type();
            for (k, grp) in [g::TRIANGLE, g::SQUARE, confirm].into_iter().enumerate() {
                let (x, y) = at::GLYPH_AT[k];
                glyphs[k] = self.kh1_ctrl(PRIORITY_ROOT, o.0 + x, o.1 + y + cm::tab::GLYPH_DY, 1.0, &[(sg.glyph(grp, pad), None)]).unwrap_or(0);
                colour(glyphs[k], one);
            }
        }
        let mut words = [0; 3];
        let mut icons = [0; 3];
        for (k, v) in slots.iter().enumerate() {
            let Some(wg) = mg.word(&lang, v.name) else { continue };
            // Squeeze to fit (owner, 2026-11-04): a name wider than its black box drawn narrower.
            let sx = kh::fit::scale(mg.width(&lang, v.name), kh::fit::SHORTCUT_BOX);
            if sx < 1.0 {
                squeezed(|| format!("shortcut {}: word {:#x} {:.1} units wide, box {}: x scale {sx:.3}", k + 1, v.name, mg.width(&lang, v.name).unwrap_or(0.0), kh::fit::SHORTCUT_BOX));
            }
            words[k] = self.kh1_ctrl(PRIORITY_ROOT, o.0 + at::WORD_AT.0, o.1 + at::WORD_AT.1 + at::PITCH * k as f32, sx, &[(wg, None)]).unwrap_or(0);
            let b = kh1_word_level(v.bright);
            colour(words[k], [b, b, b, alpha]);
            // 0.179.1: a DP sleight's mark right of its (squeezed) name.
            if let Some(ig) = mg.dp_icon.filter(|_| dp_word(v.name)) {
                let x = o.0 + at::WORD_AT.0 + mg.width(&lang, v.name).unwrap_or(0.0) * sx + DP_ICON_GAP;
                icons[k] = self.kh1_ctrl(PRIORITY_ROOT, x, o.1 + at::WORD_AT.1 + at::PITCH * k as f32, 1.0, &[(ig, None)]).unwrap_or(0);
                colour(icons[k], [b, b, b, alpha]);
            }
        }
        self.short = Some(ShortCtl { panel, glyphs, words, icons, view: *slots, alpha });
    }

    unsafe fn clear_tab(&mut self) {
        if let Some(t) = self.tab.take() {
            for c in t.ctrls.into_iter().filter(|&c| c != 0) {
                super::d2::delete(c, self.list);
            }
        }
    }

    /// KH1's look's tab (`0x140278B40`): made when it appears or its word changes (and after a
    /// row rebuild), else moved and coloured while it slides or fades.
    unsafe fn show_tab_kh1(&mut self, t: Option<TabView>, g: &kh::MenuGroups, lang: &str) {
        let Some(t) = t else {
            self.clear_tab();
            return;
        };
        let at = tab_kh1_at(&t, lang);
        let f = if t.dim { 0.5 } else { 1.0 };
        let b = kh1_word_level(t.bright);
        let cols = [[f, f, f, t.alpha], [1.0, 1.0, 1.0, t.alpha], [b, b, b, t.alpha]];
        if let Some(old) = self.tab.as_mut().filter(|o| o.view.name == t.name) {
            if old.view != t {
                for (k, &c) in old.ctrls.iter().enumerate().filter(|x| *x.1 != 0) {
                    if old.view.x != t.x {
                        func::<VecFn>(LAYOUT_POS)(c, V4([at[k].0, at[k].1, 0.0, 1.0]).0.as_ptr());
                    }
                    func::<VecFn>(COLOUR)(c, V4(cols[k]).0.as_ptr());
                }
                old.view = t;
            }
            return;
        }
        self.clear_tab();
        let mut ctrls = Vec::new();
        // The Triangle of the pad in use (the Xbox Y, ...): the shortcut panel's glyph groups.
        let tri = g.tab_icon.then(|| match super::kh1_hud::manifest().and_then(|m| m.shortcut).filter(|sg| sg.icons && sg.pads.is_some()) {
            Some(sg) if pad_type() != 2 => sg.glyph(kh::shortcut::TRIANGLE, pad_type()),
            _ => g.first + kh::menu::TRIANGLE,
        });
        let word = g.word(lang, t.name);
        for (k, grp) in [Some(g.first + kh::menu::TAB), tri, word].into_iter().enumerate() {
            let c = grp.and_then(|grp| self.kh1_ctrl(PRIORITY_ROOT, at[k].0, at[k].1, 1.0, &[(grp, None)])).unwrap_or(0);
            if c != 0 {
                func::<VecFn>(COLOUR)(c, V4(cols[k]).0.as_ptr());
            }
            ctrls.push(c);
        }
        self.tab = Some(TabCtl { ctrls, fnt: 0, view: t });
    }

    /// Re:CoM's look's tab: the highlighted plate tinted green, KH1's word on it as 7.3's rows.
    unsafe fn show_tab_recom(&mut self, t: Option<TabView>, set: *const u8, first: u32, words: &Words) {
        let Some(t) = t else {
            self.clear_tab();
            return;
        };
        let f = if t.dim { 0.5 } else { 1.0 };
        let [r, g, b] = RECOM_TAB_TINT;
        let colour = [r * f, g * f, b * f, t.alpha];
        let at = (RECOM_TAB_AT.0 + cm::PLATE_ANCHOR.0 + t.x, RECOM_TAB_AT.1 + cm::PLATE_ANCHOR.1);
        if let Some(old) = self.tab.as_mut().filter(|o| o.view.name == t.name) {
            if old.view != t {
                let c = old.ctrls[0];
                if old.view.x != t.x {
                    func::<VecFn>(LAYOUT_POS)(c, V4([at.0, at.1, 0.0, 1.0]).0.as_ptr());
                }
                func::<VecFn>(COLOUR)(c, V4(colour).0.as_ptr());
                if old.fnt != 0 {
                    wr::<u32>(old.fnt + F_COLOUR, cm::text_colour(t.bright, t.alpha));
                }
                old.view = t;
            }
            return;
        }
        self.clear_tab();
        let c = func::<NewFn>(NEW)(0x170);
        if c == 0 {
            return;
        }
        func::<CtorFn>(SEQ_CTOR)(c);
        if rd::<usize>(c) != base() + SEQ_CTRL_VTABLE as usize {
            return;
        }
        super::d2::made();
        wr::<u8>(c + C_PRIORITY, PRIORITY_ROOT);
        wr::<u8>(c + C_COLOUR_ON, 1);
        func::<VecFn>(LAYOUT_POS)(c, V4([at.0, at.1, 0.0, 1.0]).0.as_ptr());
        super::d2::add(c, self.list, set, first + 1, self.xoff, 1);
        let mut fnt = 0;
        if let Some(text) = words.get(t.name) {
            let fp = func::<NewFn>(NEW)(0xB0);
            if fp != 0 {
                let tc = cm::text_colour(t.bright, t.alpha);
                fnt = func::<FntCtorFn>(FNT_CTOR)(fp, 0, 0, &tc, FONT, text.as_ptr(), FONT_FLAGS);
                super::text_fit::fit(fnt, crate::text_fit::boxes::COMMAND_ROW, || format!("command menu (Re:CoM's look) context tab word {:#x}", t.name));
                super::d2::attach(c, self.list, set, fnt, 0.0, 0.0, TEXT_DX + self.xoff);
            }
        }
        func::<VecFn>(COLOUR)(c, V4(colour).0.as_ptr());
        self.tab = Some(TabCtl { ctrls: vec![c], fnt, view: t });
    }

    /// H4: a control for KH1's look at KH1's (x, y), its layer x offset KH1's left margin.
    unsafe fn kh1_ctrl(&self, prio: u8, x: f32, y: f32, sx: f32, groups: &[(u32, Option<f32>)]) -> Option<usize> {
        let c = func::<NewFn>(NEW)(0x170);
        if c == 0 {
            return None;
        }
        func::<CtorFn>(SEQ_CTOR)(c);
        if rd::<usize>(c) != base() + SEQ_CTRL_VTABLE as usize {
            return None;
        }
        super::d2::made();
        wr::<u8>(c + C_PRIORITY, prio);
        wr::<u8>(c + C_COLOUR_ON, 1);
        func::<VecFn>(LAYOUT_POS)(c, V4([x, y, 0.0, 1.0]).0.as_ptr());
        func::<VecFn>(SCALE)(c, V4([sx, 1.0, 1.0, 1.0]).0.as_ptr());
        let xoff = -crate::kh1_hud::right_margin(-self.xoff);
        for &(g, play) in groups {
            wr::<u8>(c + C_PRIORITY, prio);
            super::d2::add(c, self.list, crate::kh1_hud::SET.as_ptr(), g, xoff, 1);
            if let Some(t) = play {
                let n = rd::<u8>(c + 0x16D) as u32;
                if n > 0 {
                    func::<PlayFn>(PLAY_FROM)(c, n - 1, t);
                }
            }
        }
        Some(c)
    }

    /// H4: shows `sc` in KH1's look (`docs/recom-kh1-hud.md` § H4): KH1's panels, selection,
    /// icons and title from the `kh1hud` set, the words in KH1's menu font; the field look, or
    /// the battle look in Re:CoM's battle rooms (the title turns red as KH1's when the mode
    /// changes). The menu's rules are 7.3's.
    unsafe fn show_kh1(&mut self, sc: &Scene, g: &kh::MenuGroups, battle: bool) {
        if self.hidden {
            return;
        }
        let lang = super::settings().menu_language.clone();
        let dim = sc.rows.iter().any(|r| r.list != Kind::Root);
        let same = self.shown.as_ref().is_some_and(|s| s.same_rows(sc)) && self.look.as_ref().is_some_and(|l| l.battle == battle && l.dim == dim);
        if same {
            let old = self.shown.as_ref().expect("same");
            let alpha_changed = old.alpha != sc.alpha;
            for (r, v) in self.rows.iter_mut().zip(&sc.rows) {
                if alpha_changed {
                    func::<VecFn>(COLOUR)(r.ctrl, V4([1.0, 1.0, 1.0, sc.alpha]).0.as_ptr());
                }
                if r.text != 0 && (alpha_changed || r.view.bright != v.bright) {
                    let b = kh1_word_level(v.bright);
                    for c in [r.text, r.icon].into_iter().filter(|&c| c != 0) {
                        func::<VecFn>(COLOUR)(c, V4([b, b, b, sc.alpha]).0.as_ptr());
                    }
                }
                r.view = *v;
            }
            if alpha_changed {
                if let Some(l) = &self.look {
                    for (i, &h) in l.headers.iter().enumerate() {
                        let d = if i == 0 && l.dim { 0.5 } else { 1.0 };
                        func::<VecFn>(COLOUR)(h, V4([d, d, d, sc.alpha]).0.as_ptr());
                    }
                }
                if self.arrow != 0 {
                    func::<VecFn>(COLOUR)(self.arrow, V4([1.0, 1.0, 1.0, sc.alpha]).0.as_ptr());
                }
            }
            self.shown = Some(sc.clone());
            self.show_tab_kh1(sc.tab, g, &lang);
            return;
        }
        let was_battle = self.look.as_ref().map(|l| l.battle);
        self.clear();
        let look = g.first + if battle { kh::menu::LOOK } else { 0 };
        // Headers: the root's at KH1's root origin; the Magic list's at its origin.
        let mut headers = Vec::new();
        // KH1 restarts the title's colour keys when the mode changes (`0x14027B870`): into battle
        // it turns red over 20 frames (played from frame 1).
        let title = if battle && was_battle == Some(false) { (g.first + kh::menu::TITLE_TO_BATTLE, Some(UNITS)) } else { (look + kh::menu::TITLE, None) };
        if let Some(h) = self.kh1_ctrl(PRIORITY_ROOT, cm::ROOT_X, cm::ROOT_Y, 1.0, &[(look + kh::menu::HEADER, None), title]) {
            let d = if dim { 0.5 } else { 1.0 };
            func::<VecFn>(COLOUR)(h, V4([d, d, d, sc.alpha]).0.as_ptr());
            headers.push(h);
        }
        if let Some(m0) = sc.rows.iter().find(|r| r.list != Kind::Root && r.index == 0) {
            let base_y = m0.y;
            let sub = sub_groups(g, m0.list);
            // M8b: the Magic list's panels stand in for a missing Summon list: no "MAGIC" title.
            // M12 step 5: the Sleights list on the Summon list's panels (or the Magic list's), without
            // a title (KH1 has no "SLEIGHTS" art).
            let stand_in = (m0.list == Kind::Summon && g.summon_list.is_none()) || m0.list == Kind::Sleight || m0.list == Kind::Party;
            let layers: &[(u32, Option<f32>)] = if stand_in { &[(sub + kh::menu::HEADER, None)] } else { &[(sub + kh::menu::HEADER, None), (sub + kh::menu::TITLE, None)] };
            if let Some(h) = self.kh1_ctrl(PRIORITY_SUB, m0.x, base_y, 1.0, layers) {
                func::<VecFn>(COLOUR)(h, V4([1.0, 1.0, 1.0, sc.alpha]).0.as_ptr());
                headers.push(h);
            }
        }
        self.look = Some(Look { battle, headers, dim });
        for v in &sc.rows {
            let count = sc.rows.iter().filter(|r| r.list == v.list).count();
            let last = v.index as usize + 1 == count;
            let (base_g, prio) = if v.list != Kind::Root { (sub_groups(g, v.list), PRIORITY_SUB) } else { (look, PRIORITY_ROOT) };
            let plate = if v.highlight {
                kh::menu::SEL
            } else if last {
                kh::menu::ROW_LAST
            } else {
                kh::menu::ROW
            };
            let mut layers = vec![(base_g + plate, None)];
            if v.highlight {
                // The command's icon (`com_sel_ico_a / b / c`: Keyblade, hat, potion; the Magic
                // list's hat). Which row takes which: Assumption (by their pictures).
                let icon = match (v.list, v.name) {
                    // The Magic list's hat; the Items list's potion (`ite_sel_ico_d`).
                    (Kind::Magic | Kind::Items | Kind::Summon | Kind::Sleight | Kind::Party, _) => Some(base_g + 5),
                    (_, 1) | (_, 343) => Some(look + kh::menu::ICON),
                    (_, 2) => Some(look + kh::menu::ICON + 1),
                    (_, 3) => Some(look + kh::menu::ICON + 2),
                    _ => None,
                };
                if let Some(i) = icon {
                    layers.push((i, None));
                }
            }
            let at = kh1_at(v, false);
            let Some(c) = self.kh1_ctrl(prio, at.0, at.1, 1.0, &layers) else { continue };
            func::<VecFn>(COLOUR)(c, V4([1.0, 1.0, 1.0, sc.alpha]).0.as_ptr());
            // The word (FM's 4th row: the blank dash until summons, M8; `cm::ROW3_CMD`).
            let mut text = 0;
            let mut icon = 0;
            if let Some(wg) = (v.name != 0).then(|| g.word(&lang, v.name)).flatten() {
                let at = kh1_at(v, true);
                // Squeeze to fit (owner, 2026-11-04): in the Magic / Summon / Sleights lists a sleight
                // name wider than its row's black box (the cursor row: up to the hat icon) is drawn
                // narrower (`kh::fit`).
                let bx = kh::fit::list_box(v.name, v.highlight).filter(|_| matches!(v.list, Kind::Magic | Kind::Summon | Kind::Sleight | Kind::Party));
                let sx = bx.map_or(1.0, |b| kh::fit::scale(g.width(&lang, v.name), b));
                if sx < 1.0 {
                    squeezed(|| format!("{:?} list{}: word {:#x} {:.1} units wide, box {}: x scale {sx:.3}", v.list, if v.highlight { " (cursor row)" } else { "" }, v.name, g.width(&lang, v.name).unwrap_or(0.0), bx.unwrap_or(0.0)));
                }
                if let Some(t) = self.kh1_ctrl(prio, at.0, at.1, sx, &[(wg, None)]) {
                    let b = kh1_word_level(v.bright);
                    func::<VecFn>(COLOUR)(t, V4([b, b, b, sc.alpha]).0.as_ptr());
                    text = t;
                    // 0.179.1: a DP sleight's mark right of its (squeezed) name, at its brightness.
                    if let Some(ig) = g.dp_icon.filter(|_| dp_word(v.name)) {
                        let x = at.0 + g.width(&lang, v.name).unwrap_or(0.0) * sx + DP_ICON_GAP;
                        if let Some(c) = self.kh1_ctrl(prio, x, at.1, 1.0, &[(ig, None)]) {
                            func::<VecFn>(COLOUR)(c, V4([b, b, b, sc.alpha]).0.as_ptr());
                            icon = c;
                        }
                    }
                }
            } else if v.name == crate::sleights::ROW_WORD || v.name == cm::cmd::PARTY_WORD || v.name & 0xFF00 == crate::sleights::WORD_BASE {
                // M12 step 5: an asset step from before the Sleights words: the row has no word.
                static LOGGED: AtomicBool = AtomicBool::new(false);
                if !LOGGED.swap(true, Ordering::Relaxed) {
                    log(&format!("command menu: kh1-hud.json has no KH1-font word {:#x} (\"Sleights\" / a sleight's name): run this version's asset step; drawn without it", v.name));
                }
            }
            self.rows.push(Row { ctrl: c, fnt: 0, view: *v, text, icon });
        }
        self.shown = Some(sc.clone());
        // 0.111.0: the pointing arrow beside the root's cursor row.
        self.show_arrow(sc.arrow, sc.alpha, true);
        // The tab after the rows: drawn over the header (`0x140278B40` draws it after them).
        self.show_tab_kh1(sc.tab, g, &lang);
        let n = REBUILDS.fetch_add(1, Ordering::Relaxed) + 1;
        if n.is_multiple_of(100) {
            super::d2::census(|| format!("command menu: {n} row rebuilds (KH1 look)"));
        }
    }

    unsafe fn make_row(&self, set: *const u8, first: u32, v: &RowView, sc: &Scene, words: &Words) -> Option<Row> {
        let c = func::<NewFn>(NEW)(0x170);
        if c == 0 {
            return None;
        }
        func::<CtorFn>(SEQ_CTOR)(c);
        if rd::<usize>(c) != base() + SEQ_CTRL_VTABLE as usize {
            return None;
        }
        super::d2::made();
        wr::<u8>(c + C_PRIORITY, if v.list != Kind::Root { PRIORITY_SUB } else { PRIORITY_ROOT });
        wr::<u8>(c + C_COLOUR_ON, 1);
        place(c, v);
        let g = first + if v.highlight { 1 } else { 0 };
        super::d2::add(c, self.list, set, g, self.xoff, 1);
        let mut fnt = 0;
        if let Some(text) = (v.name != 0).then(|| words.get(v.name)).flatten() {
            let f = func::<NewFn>(NEW)(0xB0);
            if f != 0 {
                let colour = cm::text_colour(v.bright, sc.alpha);
                fnt = func::<FntCtorFn>(FNT_CTOR)(f, 0, 0, &colour, FONT, text.as_ptr(), FONT_FLAGS);
                // 0.113.0, true squeeze (owner, 2026-11-04): a word wider than Re:CoM's plate is
                // drawn narrower; the highlighted plate is the same plate, so one width on every row.
                super::text_fit::fit(fnt, crate::text_fit::boxes::COMMAND_ROW, || format!("command menu (Re:CoM's look) {:?} row {} word {:#x}", v.list, v.index, v.name));
                super::d2::attach(c, self.list, set, fnt, 0.0, 0.0, TEXT_DX + self.xoff);
            }
        }
        func::<VecFn>(COLOUR)(c, V4([1.0, 1.0, 1.0, sc.alpha]).0.as_ptr());
        Some(Row { ctrl: c, fnt, view: *v, text: 0, icon: 0 })
    }

    /// Shows `sc`: rebuilds the rows when what they show changed, else writes colours and the
    /// tab's place.
    unsafe fn show(&mut self, sc: &Scene, set: *const u8, first: u32, words: &Words) {
        if self.hidden {
            return;
        }
        let same = self.shown.as_ref().is_some_and(|s| s.same_rows(sc));
        if !same {
            self.clear();
            for v in &sc.rows {
                if let Some(r) = self.make_row(set, first, v, sc, words) {
                    self.rows.push(r);
                }
            }
            self.shown = Some(sc.clone());
            self.show_arrow(sc.arrow, sc.alpha, false);
            self.show_tab_recom(sc.tab, set, first, words);
            let n = REBUILDS.fetch_add(1, Ordering::Relaxed) + 1;
            if n.is_multiple_of(100) {
                super::d2::census(|| format!("command menu: {n} row rebuilds"));
            }
            return;
        }
        let old = self.shown.as_ref().expect("same");
        let alpha_changed = old.alpha != sc.alpha;
        if alpha_changed && self.arrow != 0 {
            func::<VecFn>(COLOUR)(self.arrow, V4([1.0, 1.0, 1.0, sc.alpha]).0.as_ptr());
        }
        for (r, v) in self.rows.iter_mut().zip(&sc.rows) {
            if alpha_changed {
                func::<VecFn>(COLOUR)(r.ctrl, V4([1.0, 1.0, 1.0, sc.alpha]).0.as_ptr());
            }
            if r.fnt != 0 && (alpha_changed || r.view.bright != v.bright) {
                wr::<u32>(r.fnt + F_COLOUR, cm::text_colour(v.bright, sc.alpha));
            }
            r.view = *v;
        }
        self.shown = Some(sc.clone());
        self.show_tab_recom(sc.tab, set, first, words);
    }
}

/// The colour level of a KH1-look word for KH1's brightness `bright` (0x73 grey … 0xE6). The
/// baked glyphs are KH1's grey texels doubled (0x80 → 0xFF, as KH1's ×2 vertex colour shows
/// them); KH1 draws texel × bright / 0x80 = bright (0xE6: 230, off-white), so the level is
/// bright / 255. It was bright / 128, which clipped every bright word to pure white (owner's
/// side-by-side with KH1, 2026-11-01).
fn kh1_word_level(bright: u8) -> f32 {
    bright as f32 / 255.0
}

/// The first of a submenu's six groups in KH1's look: the Magic list's, or (M8 stage 8a.1) the
/// Items list's (`items_list`); an older asset step built none: the Magic list's, logged once.
fn sub_groups(g: &kh::MenuGroups, list: Kind) -> u32 {
    static LOGGED: AtomicBool = AtomicBool::new(false);
    static LOGGED_SUMMON: AtomicBool = AtomicBool::new(false);
    if list == Kind::Summon || list == Kind::Sleight || list == Kind::Party {
        return match g.summon_list {
            Some(first) => first,
            None => {
                if !LOGGED_SUMMON.swap(true, Ordering::Relaxed) {
                    log("command menu: kh1-hud.json has no KH1 Summon list (run this version's asset step): drawn with the Magic list's panels, without its title");
                }
                g.first + kh::menu::MAGIC
            }
        };
    }
    match (list, g.items_list) {
        (Kind::Items, Some(first)) => first,
        (Kind::Items, None) => {
            if !LOGGED.swap(true, Ordering::Relaxed) {
                log("command menu: kh1-hud.json has no KH1 Items list (run this version's asset step): drawn with the Magic list's panels");
            }
            g.first + kh::menu::MAGIC
        }
        _ => g.first + kh::menu::MAGIC,
    }
}

/// H4: the KH1 look's groups when the KH1 HUD is up and its menu look was built.
fn kh1_look() -> Option<&'static kh::MenuGroups> {
    if !super::kh1_hud::menu_on() {
        return None;
    }
    // Riku's story (0.149.0): his FO0071 has the set only from this version's asset step; without
    // it Re:CoM's look (a registry lookup, Riku's story only).
    if super::stats::riku_story() && !super::kh1_hud::set_registered() {
        return None;
    }
    super::kh1_hud::manifest()?.menu.as_ref()
}

/// KH1's place of a row's plate (or its word): the frame at the list origin + (0, 20 + 22 i), the
/// selection at + (11, 18 + 22 c); the words at + (16, 22 + 22 i) / + (29, 25 + 22 c)
/// (`0x140278B40`, `0x14027A920`). `v.y` is the list origin + 22 i.
fn kh1_at(v: &RowView, word: bool) -> (f32, f32) {
    use kh::menu as m;
    let (dx, dy) = match (v.highlight, word) {
        (false, false) => (0.0, m::HEADER_H),
        (true, false) => m::SEL_AT,
        (false, true) => m::WORD_AT,
        (true, true) => m::SEL_WORD_AT,
    };
    (v.x + dx, v.y + dy)
}

/// KH1's place of the tab's plate, Triangle and word (`cm::tab`): the root's origin + (9, −10)
/// + the slide; the Triangle by the menu language, dropped as KH1's button glyphs.
fn tab_kh1_at(t: &TabView, lang: &str) -> [(f32, f32); 3] {
    use cm::tab;
    let o = (cm::ROOT_X + tab::AT.0 + t.x, cm::ROOT_Y + tab::AT.1);
    [o, (o.0 + tab::triangle_x(lang), o.1 + tab::TRIANGLE_Y + tab::GLYPH_DY), (o.0 + tab::WORD_AT.0, o.1 + tab::WORD_AT.1)]
}

/// Re:CoM's layout position of a row: KH1's top-left + the plate's anchor.
unsafe fn place(c: usize, v: &RowView) {
    func::<VecFn>(LAYOUT_POS)(c, V4([v.x + cm::PLATE_ANCHOR.0, v.y + cm::PLATE_ANCHOR.1, 0.0, 1.0]).0.as_ptr());
    func::<VecFn>(SCALE)(c, V4([1.0, 1.0, 1.0, 1.0]).0.as_ptr());
}

/// Re:CoM's pad, this frame's fresh presses (pure reads), when Sora takes input; L1 held always
/// (KH1 swaps the shortcut list in before its "Sora takes input" test, `0x1402937F0`).
unsafe fn read_pad(pl: usize) -> Pad {
    let l1 = func::<HeldFn>(BUTTONS_HELD)(L1, 0) != 0;
    if rd::<u32>(pl + p::PAD) & 1 == 0 {
        return Pad { l1, ..Pad::default() };
    }
    let fresh = func::<PadFn>(PAD_FRESH);
    let btn = |a: u32| func::<PadBtnFn>(a)(0) & 0xFF != 0;
    Pad {
        up: fresh(0x1000, 0) != 0,
        down: fresh(0x4000, 0) != 0,
        left: fresh(0x8000, 0) != 0,
        right: fresh(0x2000, 0) != 0,
        confirm: btn(PAD_CONFIRM),
        cancel: btn(PAD_CANCEL),
        triangle: fresh(0x10, 0) != 0,
        square: fresh(0x80, 0) != 0,
        // L2, R2, L1, R1, Select.
        other: fresh(0x10F, 0) != 0,
        l1,
    }
}

/// The object of handle `h` and its kind (its type getter, vtable `+0x28`: `mov eax, imm32;
/// ret`, the door's recognised by its address). Pure reads but Re:CoM's handle lookup.
unsafe fn object(h: i32) -> Option<(usize, u32)> {
    let obj = func::<mv::HandleFn>(mv::HANDLE_OBJ)(h as u32);
    if obj == 0 {
        return None;
    }
    let getter: usize = rd(rd::<usize>(obj) + 0x28);
    let kind = if getter == base() + DOOR_TYPE as usize {
        cm::recom::kind::DOOR
    } else if rd::<u8>(getter) == 0xB8 && rd::<u8>(getter + 5) == 0xC3 {
        rd::<u32>(getter + 1)
    } else {
        0
    };
    Some((obj, kind))
}

/// KH1's context command for what Re:CoM's Triangle would act on in the field:
/// - Re:CoM's field action's object (`0x1401E80C0`: handle `+0x5FC` when > 0, else `+0x84C`):
///   a treasure box not yet opened → Open, a carry gimmick → Lift, an NPC (`ATEL::CAvatar`) →
///   Talk, a door → Open, a jump stand → Jump On;
/// - else the save / warp point their own updates would act on (`0x1401C5180` / `0x1401CC280`:
///   handle `+0x850` unless 0 or −1, else `+0x84C`; Re:CoM's action 1..4): a save point Sora
///   stands on → Save, a warp point → Examine (`cm::recom::point`).
///
/// The words for Re:CoM's objects: Assumption.
unsafe fn context(pl: usize) -> u8 {
    if rd::<i32>(pl + p::BATTLE) != 0 {
        return cm::cmd::NONE;
    }
    let mut h: i32 = rd(pl + ACTION_TARGET);
    if h <= 0 {
        h = rd(pl + p::LOCK_TARGET);
    }
    let mut seen = None;
    if h > 0 {
        seen = object(h).map(|o| (h, o));
        if let Some((_, (obj, kind))) = seen {
            let c = cm::recom::field_action(kind, rd::<u8>(obj + BOX_OPENED) & 1 != 0);
            if c != cm::cmd::NONE {
                return c;
            }
        }
    }
    let mut hp: i32 = rd(pl + LOCK_ON);
    if (hp as u32).wrapping_add(1) < 2 {
        hp = rd(pl + p::LOCK_TARGET);
    }
    if hp == 0 {
        return cm::cmd::NONE;
    }
    // Most frames this is the object just read (no lock-on, no `+0x5FC`).
    let Some((obj, kind)) = (match seen {
        Some((sh, o)) if sh == hp => Some(o),
        _ => object(hp),
    }) else {
        return cm::cmd::NONE;
    };
    let action: u32 = rd(pl + p::ACTION);
    match kind {
        cm::recom::kind::SAVE_POINT => cm::recom::point(kind, action, rd::<u8>(obj + SAVE_IN_RANGE) == 1, false),
        cm::recom::kind::WARP_POINT => cm::recom::point(kind, action, false, rd::<u32>(base() + WARP_BLOCK as usize) & 1 != 0),
        _ => cm::cmd::NONE,
    }
}

/// The spells Sora has and his shortcut slots: the save block's (stages 7.11, 7.12), or with the
/// debug switch every spell at level 3 and, while the save has no slot set, the slots KH1's learn
/// would give them all (Fire, Blizzard, Thunder; not written to the save).
fn spells() -> ([u8; 7], [u8; 3]) {
    let (levels, slots) = super::mp::spell_state();
    if !ALL_SPELLS.load(Ordering::Relaxed) {
        return (levels, slots);
    }
    let none = slots.iter().all(|&b| !cm::shortcut::is_set(b));
    ([3; 7], if none { [1, 2, 3] } else { slots })
}

/// The spells and shortcut slots the menu uses (stage 7.13's Customize page shows and edits the
/// same ones).
pub fn spell_view() -> ([u8; 7], [u8; 3]) {
    spells()
}

/// A KH1 word of the menu's language (`kh1-command-menu.json`, Re:CoM font bytes, NUL-terminated)
/// by message id, when the command menu's words are loaded.
/// 0.181.0: the highlighted command has KH1's flag 4 (`Menu::highlighted_targets`), so KH1 would
/// pick a soft target. No menu, or the menu busy this instant: KH1's default entry 0x43 (flag 4).
pub fn highlighted_targets() -> bool {
    STATE.try_lock().ok().and_then(|s| s.menu.as_ref().map(|m| m.highlighted_targets())).unwrap_or(true)
}

pub fn word(id: u16) -> Option<Vec<u8>> {
    WORDS.get().and_then(|w| w.get(id)).map(|b| b.to_vec())
}

/// Re:CoM's confirm button is Circle (its key word `0x14083F3F0`), for the shortcut help lines.
pub fn confirm_is_circle() -> bool {
    // SAFETY: a static of the image.
    cm::shortcut::confirm_is_circle(unsafe { rd::<u16>(base() + CONFIRM_KEY as usize) })
}

/// Riku's menu (`crate::riku`, `riku_kh1_movement`): Attack, and his Sleights list
/// (`crate::riku::SLEIGHTS`, started by his own sleight start) on row 3; no spells, items, summons,
/// Party or specials. His shortcut slots are filled with his first sleights once, while all three
/// are empty and never set up; his Customize page (0.149.0, his Settings list) sets them after.
/// Riku's spell levels (0.187.0): Sora's seven at level 3 while the magic is in (the owner's "deixa desbloqueado no
/// menu"); the command menu and his Customize picker read this one answer, so a spell unlocks in both together.
/// 0.198.0 (fork owner, 2026-11-26): learned through his story (`crate::riku_unlocks`, `super::unlocks::riku_sync`), not all at 3.
pub fn riku_spell_levels() -> [u8; crate::command_menu::spell::COUNT] {
    if !super::magic::installed() {
        return [0; crate::command_menu::spell::COUNT];
    }
    // 0.198.18, development builds only (`settings::DEV_KEYS`; fork owner, 2026-11-27: "quero testar as magias habilita"):
    // `kh1_all_spells = on` gives Riku all seven at level 3 too, as it does Sora (not written to the save).
    if cfg!(feature = "dev") && ALL_SPELLS.load(Ordering::Relaxed) {
        return [3; crate::command_menu::spell::COUNT];
    }
    // SAFETY: a read inside Re:CoM's save block.
    let l = unsafe { super::unlocks::riku_levels() };
    let mut out = [0u8; crate::command_menu::spell::COUNT];
    for (o, v) in out.iter_mut().zip(l) {
        *o = v;
    }
    out
}

unsafe fn riku_world(pl: usize) -> World {
    // 0.198.15: only what Re:CoM has taught him (none before the Ansem tutorial's end), as Re:CoM's Sleights List.
    let dark = mv::is_main_dark_riku(pl);
    let ids = riku_learned_sleights(dark);
    let mut shortcuts = super::mp::riku_shortcuts().unwrap_or([0; 3]);
    // 0.198.18: Dark Mode has its own three slots (filled once from his with the Dark twins).
    if dark {
        shortcuts = riku_dark_shortcuts(&ids);
    } else if let Some(f) = crate::riku::fill_shortcuts(shortcuts, &ids).filter(|_| !super::mp::riku_shortcuts_set()) {
        if super::mp::set_riku_shortcuts(f) {
            shortcuts = f;
            riku_learned_note();
            note(|| format!("riku: shortcut slots set to his first sleights {f:02x?}"));
        }
    }
    // 0.150.0: page 2 (Left) as Sora's, its Party list holding the King's sleights (Riku's only
    // friend; the Summon row stays greyed: he has none).
    let mut w = World { mp: mp_view(), dp: super::dark_points::sleight_view(pl), shortcuts, page: super::summons::page(), ..Default::default() };
    // 0.184.0 (fork owner, 2026-11-23: "deixa desbloqueado no menu"): Sora's seven KH1 spells, all at level 3,
    // while the magic is in (its exeMagic slots, `super::magic::riku_slots`); MP free with `riku_free_magic`.
    if super::magic::installed() {
        w.spells = riku_spell_levels();
        w.free_spells = super::magic::riku_free();
        if let Some(t) = super::attack::attack_target(pl) {
            let b: usize = rd(pl + p::BODY);
            if b != 0 {
                w.target_dist = Some((to_kh1(rd_vec(b + body::POS)) - t.centre).length());
            }
        }
    }
    // 0.163.0: his KH1 item slots (the Items row), as Sora's.
    if super::items::installed() {
        (w.slots, w.slot_count) = super::items::slots();
    }
    if let Ok(g) = mv::SORA.try_lock() {
        if let Some(s) = g.as_ref().filter(|s| s.player == pl) {
            w.state = s.sora.state;
            if mv::sync_ok(pl, s) {
                w.gate_any = s.sora.accepts_spell(crate::magic::spell::BLIZZARD);
                w.gate_fire = s.sora.accepts_spell(crate::magic::spell::FIRE);
            }
        }
    }
    w.context = context(pl);
    // 0.162.0: a held gimmick card's "Stagger", as Sora's.
    if w.context == 0 {
        w.gimmick = super::gimmick::offer(pl);
    }
    if super::summons::installed() {
        (w.battle, w.summon_out) = super::summons::gate_state(pl);
        w.summon_out |= super::summons::sleight_running(pl);
    }
    for (k, &id) in ids.iter().enumerate().take(crate::sleights::MAX) {
        w.sleights[k] = id as u8;
    }
    w.sleight_count = ids.len().min(crate::sleights::MAX) as u8;
    if super::party::installed() && (w.page == cm::Page::Two || w.shortcuts.iter().any(|&b| cm::shortcut::slot_sleight(b).is_some_and(|id| crate::party::index(id).is_some()))) {
        (w.party_learned, w.party_avail) = super::party::sets(pl);
    }
    w
}

/// 0.198.18: Dark Mode's three shortcut slots (`crate::riku::dark_block`); the first time (never set up), Riku's own
/// set through `crate::riku::dark_fill` (Impulse → Dark Impulse ..), written once. `dark_learned`: Dark Riku's learned
/// sleights ([`riku_learned_sleights`]`(true)`).
pub(super) fn riku_dark_shortcuts(dark_learned: &[u16]) -> [u8; 3] {
    let Some((slots, set)) = super::mp::riku_dark_shortcuts() else { return [0; 3] };
    // As his own set's fill (Stage 414): nothing is written (nor marked set up) while Dark Riku has learned nothing.
    if set || dark_learned.is_empty() {
        return slots;
    }
    let riku = super::mp::riku_shortcuts().unwrap_or([0; 3]);
    let f = crate::riku::dark_fill(riku, dark_learned);
    if super::mp::set_riku_dark_shortcuts(f) {
        log(&format!("riku: Dark Mode shortcut slots set up from his {riku:02x?}: {f:02x?}"));
        return f;
    }
    slots
}

/// 0.198.15: Riku's (or Dark Riku's) sleights that Re:CoM's own test (`0x14023AB00`) answers as learned
/// (`crate::riku::learned_ids`). His command menu, shortcuts and Customize page list only these.
pub(super) unsafe fn riku_learned_sleights(dark: bool) -> Vec<u16> {
    let f = super::abilities::sleight_learned_fn();
    crate::riku::learned_ids(dark, |id| unsafe { f(id) } & 0xFF != 0)
}

/// Re:CoM's own "learned" answer for each of Riku's sleights (`0x14023AB00`), logged once (0.198.15: the lists
/// follow it).
unsafe fn riku_learned_note() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let f = super::abilities::sleight_learned_fn();
        let v: Vec<String> = crate::riku::SLEIGHTS.iter().map(|s| format!("{} {}", s.name, if f(s.id) & 0xFF != 0 { "learned" } else { "not learned" })).collect();
        log(&format!("riku: Re:CoM's sleight-learned test: {}", v.join(", ")));
    });
}

unsafe fn world(pl: usize) -> World {
    if mv::is_main_riku(pl) {
        return riku_world(pl);
    }
    let (spells, shortcuts) = spells();
    let mut w = World { mp: mp_view(), spells, shortcuts, ..Default::default() };
    // M8 stage 8a.1: Sora's item slots (a read of our save block), only with the items in.
    if super::items::installed() {
        (w.slots, w.slot_count) = super::items::slots();
    }
    let mut in_recoil = false;
    let mut special_free = false;
    if let Ok(g) = mv::SORA.try_lock() {
        if let Some(s) = g.as_ref().filter(|s| s.player == pl) {
            w.state = s.sora.state;
            // M9 stage 9c.1: a running special's follow-up in the Attack row (`0x14039ED70`).
            if let Some((c, grey)) = s.sora.follow_up() {
                (w.follow, w.follow_grey) = (c, grey);
            }
            special_free = s.sora.special.is_none();
            in_recoil = s.sora.abilities.counterattack && s.sora.clashed && s.sora.rec.head() == crate::rows::RECOIL;
            // KH1's commit gate for spells (`0x140293480`), on the KH1 frame as last left, and
            // only while Re:CoM's action is one KH1's frame runs (M7 stage 7.4). One update
            // stale: the frame itself checks again (`Kh1Sora::accepts_spell`).
            if mv::sync_ok(pl, s) {
                w.gate_any = s.sora.accepts_spell(crate::magic::spell::BLIZZARD);
                w.gate_fire = s.sora.accepts_spell(crate::magic::spell::FIRE);
            }
        }
    }
    let target = if in_recoil || (0..7).any(|i| cm::spell::RANGED[i] && w.spells[i] > 0) { super::attack::attack_target(pl) } else { None };
    w.counter = in_recoil && target.is_some();
    if let Some(t) = target {
        let b: usize = rd(pl + p::BODY);
        if b != 0 {
            w.target_dist = Some((to_kh1(rd_vec(b + body::POS)) - t.centre).length());
        }
    }
    w.context = context(pl);
    // A held Re:CoM gimmick card's "Stagger" (battle only: the cards exist only in battle rooms),
    // when no object took the context entry.
    if w.context == 0 {
        w.gimmick = super::gimmick::offer(pl);
    }
    // M8b: the learned summons and KH1's usable inputs (battle mode, one out).
    if super::summons::installed() {
        (w.summons, w.summon_count) = super::summons::learned();
        (w.battle, w.summon_out) = super::summons::gate_state(pl);
        // Owner (2026-11-04, "Sleights outside battle"): a field room where Re:CoM's sleights run,
        // and whether its packs load (two loads in a field room).
        (w.field, w.packs) = super::field_sleights::gate(pl);
        // 0.111.0: the page (one save byte); the equipped sleights (row 3 on both pages, and M12
        // step 6's shortcut slots); any Re:CoM sleight running locks the Summon, Party and Sleights
        // rows (one at a time).
        w.summon_out |= super::summons::sleight_running(pl);
        w.page = super::summons::page();
        // The Party list (page 2) and the shortcut slots holding a Party sleight: this battle's set
        // (read once a battle, `super::party::sets`).
        if super::party::installed() && (w.page == cm::Page::Two || w.shortcuts.iter().any(|&b| cm::shortcut::slot_sleight(b).is_some_and(|id| crate::party::index(id).is_some()))) {
            (w.party_learned, w.party_avail) = super::party::sets(pl);
        }
        {
            // M12 step 7: the equipped KH1 specials listed too (Strike Raid, Sonic Blade, Ars Arcanum, Ragnarok; owner: "also
            // show up in the sleights menu"), each usable as its prompt would offer it.
            let mut equipped = super::abilities::equipped_sleights();
            if let Some(e) = equipped_specials() {
                equipped.extend(crate::sleights::SPECIALS.iter().filter(|&&i| e[i]).filter_map(|&i| crate::sleights::special_sleight(i)));
            }
            let ids = crate::sleights::list(&equipped);
            for (k, &id) in ids.iter().enumerate() {
                w.sleights[k] = id as u8;
            }
            w.sleight_count = ids.len() as u8;
            if special_free && super::specials::installed() && (w.battle || w.field) {
                w.sleight_specials = sleight_specials(pl, w.field);
            }
        }
    }
    // M9 stage 9c.1: KH1's special offer (`0x14028D0F0`) when nothing took the context entry.
    // Battle only (owner, 2026-11-01: "Only in battle"), lifted (owner, 2026-11-04, "Sleights outside
    // battle"): a field room too, free there, Sonic Blade / Ars Arcanum / Ragnarok where their packs
    // load (`crate::field_sleights::room_ok`).
    if w.context == 0 && w.gimmick == 0 && special_free && super::specials::installed() && rd::<i32>(pl + p::BATTLE) != 0 {
        (w.special, w.special_grey) = special_offer(pl, false, true).unwrap_or((0, false));
    } else if w.context == 0 && w.gimmick == 0 && special_free && super::specials::installed() && w.field {
        (w.special, w.special_grey) = special_offer(pl, true, w.packs).unwrap_or((0, false));
    }
    w
}

/// `0x14028D0F0` on Sora's equipped specials and Re:CoM's attack target, when it is a live enemy
/// (KH1 picks its target among enemies; Assumption: Re:CoM's lock target stands for KH1's).
///
/// In a field room (`field`): free, and a special with a pack only where packs load (`packs`).
unsafe fn special_offer(pl: usize, field: bool, packs: bool) -> Option<(u8, bool)> {
    let mut equipped = equipped_specials()?;
    if field && !packs {
        for (i, e) in equipped.iter_mut().enumerate() {
            if crate::sleights::special_sleight(i).is_some_and(crate::field_sleights::needs_pack) {
                *e = false;
            }
        }
    }
    if !equipped.iter().any(|&b| b) {
        return None;
    }
    let t = special_target(pl)?;
    crate::specials::offer(&equipped, Some(t), &mut |c| field || super::mp::can_pay(c))
}

/// The built specials Sora has equipped (by `crate::specials` index), each only when it is in.
fn equipped_specials() -> Option<[bool; 5]> {
    use crate::abilities::ix;
    let e = super::abilities::equipped_live()?;
    let mut equipped = [false; 5];
    for (i, a) in [(crate::specials::SONIC_BLADE, ix::SONIC_BLADE), (crate::specials::ARS_ARCANUM, ix::ARS_ARCANUM), (crate::specials::STRIKE_RAID, ix::STRIKE_RAID), (crate::specials::RAGNAROK, ix::RAGNAROK)] {
        equipped[i] = e & 1 << a != 0 && super::specials::ready(i);
    }
    Some(equipped)
}

/// What `0x14028D0F0` sees of Re:CoM's attack target, when it is a live enemy (or, in a field room,
/// a field Heartless: `win::magic::is_target_enemy`).
unsafe fn special_target(pl: usize) -> Option<crate::specials::OfferTarget> {
    let t = super::attack::attack_target(pl).filter(|t| super::magic::is_target_enemy(t.owner))?;
    let b: usize = rd(pl + p::BODY);
    if b == 0 {
        return None;
    }
    let sora = to_kh1(rd_vec(b + body::POS));
    Some(crate::specials::OfferTarget::new(sora, &t))
}

/// M12 step 7 (owner, 2026-11-03: "also show up in the sleights menu"): the Sleights list's specials
/// Sora could start now, by `crate::specials` index bit: each one's own `0x14028D0F0` score is 2 (in
/// range of a live target, enough MP), as its prompt would offer it.
///
/// In a field room (`field`): free (owner, 2026-11-04, "Sleights outside battle"); the pack rule is
/// the list's (`crate::sleights::special_usable`).
unsafe fn sleight_specials(pl: usize, field: bool) -> u8 {
    let Some(equipped) = equipped_specials() else { return 0 };
    let Some(t) = special_target(pl) else { return 0 };
    let mut ok = 0u8;
    for i in crate::sleights::SPECIALS {
        if equipped[i] && crate::specials::score(i, &t, &mut |c| field || super::mp::can_pay(c)) == 2 {
            ok |= 1 << i;
        }
    }
    ok
}

/// Re:CoM's request bits the menu lets through this frame: (Attack, jump, Triangle, Square).
pub struct Keep {
    pub attack: bool,
    pub jump: bool,
    pub triangle: bool,
    /// Square (`& 2`): not while the shortcut list is on top (stage 7.12).
    pub square: bool,
}

/// KH1's menu step for this frame of Sora's (from the request-bits wrapper, after Re:CoM's
/// builder). None when there is no menu (nothing changes).
pub unsafe fn frame(pl: usize) -> Option<Keep> {
    let k = step_frame(pl);
    if k.is_none() {
        SHORT_READY.store(false, Ordering::Relaxed);
    }
    k
}

unsafe fn step_frame(pl: usize) -> Option<Keep> {
    if !LIVE.load(Ordering::Acquire) {
        return None;
    }
    let (set, first) = super::mp_bar::menu_groups()?;
    let words = WORDS.get()?;
    if !super::mp::live() {
        return None;
    }
    let pad = read_pad(pl);
    let w = world(pl);
    let step = (rd::<i32>(pl + p::DT).max(0) as f32) / 50.0;
    let (out, token_events, summon_at, short_sleights) = {
        let mut s = STATE.lock().ok()?;
        let State { menu, uis } = &mut *s;
        let m = menu.as_mut()?;
        if uis.iter().all(|u| u.hidden) {
            return None;
        }
        let out = m.step(&w, pad);
        SHORT_READY.store(m.short_ok, Ordering::Relaxed);
        m.tick(step);
        let sc = m.scene();
        let kh1 = kh1_look();
        let battle = rd::<i32>(pl + p::BATTLE) != 0;
        for u in uis.iter_mut() {
            // Stage 7.12: the shortcut list in the root's place (KH1's panel in both looks).
            if let Some(slots) = &sc.shortcut {
                if u.shown.is_some() || u.tab.is_some() {
                    u.clear_lists();
                }
                u.show_short(slots, sc.alpha);
                continue;
            }
            if u.short.is_some() {
                u.clear_short();
            }
            match kh1 {
                Some(g) => u.show_kh1(&sc, g, battle),
                None => {
                    if u.look.is_some() {
                        u.clear();
                    }
                    u.show(&sc, set, first, words)
                }
            }
        }
        let ev: Vec<String> = out.events.iter().map(|e| describe(e, m)).collect();
        // M8b: the summon under the Summon list's cursor (the preload): its LV3 sleight. M12 step 5:
        // the sleight under the Sleights list's cursor, or its first while the root's cursor rests
        // on the Sleights row.
        let at = if m.open && m.sub == Kind::Summon {
            m.summon.cmds.get(m.summon.cursor.max(0) as usize).copied().and_then(cm::cmd::summon).and_then(|i| crate::summons::ALL.get(i)).map(|s| s.sleight)
        } else if m.open && m.sub == Kind::Sleight {
            m.sleight.cmds.get(m.sleight.cursor.max(0) as usize).copied().and_then(crate::sleights::of_command)
        } else if m.open && m.sub == Kind::Party {
            m.party.cmds.get(m.party.cursor.max(0) as usize).copied().and_then(crate::sleights::of_command)
        } else if !m.open && m.root.cursor == 3 {
            m.sleight.cmds.first().copied().and_then(crate::sleights::of_command)
        } else if !m.open && m.root.cursor == 2 && m.page == cm::Page::Two {
            m.party.cmds.first().copied().and_then(crate::sleights::of_command)
        } else {
            None
        };
        // M12 step 6: the shortcut list's sleights (their commands), for the preload.
        let short_sleights: [Option<u16>; 3] = std::array::from_fn(|k| m.shortcut.cmds.get(k).copied().and_then(crate::sleights::of_command));
        (out, ev, at, short_sleights)
    };
    // KH1's own sounds with KH1's look (`menu_sounds`), else Re:CoM's; the page switch Re:CoM's
    // cursor sound in either look (0.111.0).
    let kh1_sounds = super::kh1_hud::menu_on();
    for snd in &out.sounds {
        let id = if *snd == cm::Sound::Page { snd.recom_id() } else { super::menu_sounds::id_for(*snd, kh1_sounds) };
        func::<SoundFn>(SOUND)(id, -1);
    }
    for (e, d) in out.events.iter().zip(&token_events) {
        event(d.clone());
        match e {
            Event::Spell(c, _) => note(|| format!("{d} [{c:#x}] greyed (Re:CoM's refused sound)")),
            Event::NotNow(c) => note(|| format!("{d} [{c:#x}]: KH1's gate refused it this frame (Sora busy)")),
            Event::Cast(c) => note(|| format!("{d} [{c:#x}]: issued")),
            Event::Shortcut(_, c, _) => note(|| format!("{d} [{c:#x}]")),
            Event::Item(c, _, _) => note(|| format!("{d} [{c:#x}]")),
            Event::Summon(c, _) => note(|| format!("{d} [{c:#x}]")),
            Event::Sleight(c, _) | Event::Party(c, _) => note(|| format!("{d} [{c:#x}]")),
            _ => note(|| d.clone()),
        }
    }
    // The spell issued (M7 stage 7.4): KH1's magic command for this frame's KH1 frame.
    if let Some((s, lv)) = out.cast.and_then(cm::cmd::spell_of) {
        super::magic::issue(s, lv);
    }
    // The item chosen (M8 stage 8a.1): KH1's item command for this frame's KH1 frame.
    if let Some(c) = out.item {
        super::items::issue(c.item, c.slot);
    }
    // M8b: a summon (KH1's command step runs it the frame it is issued: Re:CoM's sleight start from
    // its request bits, where we are).
    if let Some(i) = out.summon {
        super::summons::start(pl, i);
    }
    // M12 step 5: a Sleights list entry: Re:CoM's own sleight start for it (the summons' path);
    // D-pad Left's switch kept in the save.
    // 0.111.0: a Party sleight (the Party list or a shortcut) through the same start, battle only.
    if let Some(id) = out.sleight {
        if crate::party::index(id).is_some() {
            super::summons::start_party(pl, id);
        } else {
            super::summons::start_sleight(pl, id);
        }
    }
    if let Some(pg) = out.page {
        super::summons::set_page(pg);
    }
    if super::summons::installed() {
        super::summons::preload_sleight(pl, summon_at);
        // M12 step 6: the shortcut list came on top: its sleights' packs, as the Sleights list's
        // cursor preloads its entry.
        if out.events.contains(&Event::ShortcutList(true)) {
            let ids: Vec<u16> = short_sleights.iter().copied().flatten().collect();
            super::summons::preload_sleights(pl, &ids);
        }
    }
    // M9 stage 9c.1: a special (the context tab's Triangle, or Rave / Blast in the Attack row).
    if let Some(c) = out.special {
        super::specials::issue(c);
    }
    // Triangle on "Stagger": the held gimmick card is played (Re:CoM's play writes).
    if out.gimmick {
        super::gimmick::use_card(pl);
    }
    if out.confirm_refused {
        super::specials::confirm_refused(pl);
    }
    Some(Keep { attack: out.attack, jump: out.jump, triangle: out.triangle, square: out.square })
}

fn describe(e: &Event, m: &Menu) -> String {
    let name = |c: u8| {
        WORDS.get().and_then(|w| w.get(cm::cmd::name(c))).map(|b| String::from_utf8_lossy(&b[..b.len().saturating_sub(1)]).into_owned()).unwrap_or_else(|| format!("{c:#x}"))
    };
    match *e {
        Event::Opened(Kind::Items) => format!("open items ({} filled slots {:?}, grey {:#x})", m.items.cmds.len(), m.item_slots, m.items.grey & 0xFF),
        Event::Opened(Kind::Summon) => format!("open summon ({} learned, grey {:#x})", m.summon.cmds.len(), m.summon.grey & 0x7F),
        Event::Opened(Kind::Sleight) => format!("open sleights ({} equipped, grey {:#x})", m.sleight.cmds.len(), m.sleight.grey & 0x7FF_FFFF),
        Event::Opened(Kind::Party) => format!("open party ({} available, grey {:#x})", m.party.cmds.len(), m.party.grey & 0x7FF),
        Event::Page(pg) => format!("page {}", if pg == cm::Page::Two { 2 } else { 1 }),
        Event::Party(c, r) => format!(
            "party {} {}",
            name(c),
            match r {
                ShortResult::Cast => "issued",
                ShortResult::Greyed => "greyed",
                ShortResult::NotNow => "not-now",
            }
        ),
        Event::Sleight(c, r) => format!(
            "sleight {} {}",
            name(c),
            match r {
                ShortResult::Cast => "issued",
                ShortResult::Greyed => "greyed",
                ShortResult::NotNow => "not-now",
            }
        ),
        Event::Opened(_) => format!("open magic ({} spells, grey {:#x})", m.magic.cmds.len(), m.magic.grey & 0x7F),
        Event::Closed => "close".into(),
        Event::Spell(c, _) => format!("spell {}", name(c)),
        Event::Cast(c) => format!("cast {}", name(c)),
        Event::NotNow(c) => format!("not-now {}", name(c)),
        Event::Refused(c) => format!("refused {}", if c == 0 { "-".into() } else { name(c) }),
        Event::Context(c) => format!("context {}", name(c)),
        Event::ShortcutList(on) => (if on { "shortcuts on (L1)" } else { "shortcuts off" }).into(),
        Event::Item(c, slot, r) => format!(
            "item {} slot {} {}",
            name(c),
            slot + 1,
            match r {
                ShortResult::Cast => "used",
                ShortResult::Greyed => "greyed",
                ShortResult::NotNow => "not-now",
            }
        ),
        Event::Summon(c, r) => format!(
            "summon {} {}",
            name(c),
            match r {
                ShortResult::Cast => "issued",
                ShortResult::Greyed => "greyed",
                ShortResult::NotNow => "not-now",
            }
        ),
        Event::Shortcut(k, c, r) => format!(
            "shortcut {} {} {}",
            k + 1,
            if c == cm::cmd::BLANK { "-".into() } else { name(c) },
            match r {
                ShortResult::Cast => "cast",
                ShortResult::Greyed => "greyed",
                ShortResult::NotNow => "not-now",
            }
        ),
    }
}

/// Stage 7.12: the save and warp points' fresh-Triangle read (their calls of `0x14029B550` at
/// `0x1401C530E` / `0x1401CC31B`, retargeted here): "not pressed" while L1 holds KH1's shortcut
/// list (the menu runs, shows, a slot is set), where KH1's Triangle is shortcut 1 and never the save
/// point's command; else Re:CoM's own read. Two loads and a held-pad read each call (only while
/// Sora targets the point).
pub unsafe extern "win64" fn point_triangle(port: u32) -> u32 {
    if SHORT_READY.load(Ordering::Relaxed) && super::mp::active() && func::<HeldFn>(BUTTONS_HELD)(L1, port) != 0 {
        if func::<PadBtnFn>(POINT_TRIANGLE)(port) != 0 && !POINT_LOGGED.swap(true, Ordering::Relaxed) {
            log("command menu: save / warp point: Triangle with L1 held is shortcut 1 (KH1), not the point's menu");
        }
        return 0;
    }
    func::<PadBtnFn>(POINT_TRIANGLE)(port)
}

/// Retargets the save and warp points' Triangle reads to [`point_triangle`] through one jump near
/// them (`sites::COMMAND_MENU_POINT_CALLS`; each call's bytes are checked by `patch`, their
/// context by the start-up check). Each on its own: one missing only keeps Re:CoM's read there.
unsafe fn install_point_calls(b: usize) -> [bool; 2] {
    let calls = crate::sites::COMMAND_MENU_POINT_CALLS;
    let jmp = crate::hookplan::abs_jmp(point_triangle as *const () as u64);
    let Some(thunk) = super::hook::place_near(b + calls[0].rva as usize, &jmp) else { return [false; 2] };
    let mut done = [false; 2];
    for (k, c) in calls.iter().enumerate() {
        let at = b + c.rva as usize;
        if let Some(call) = crate::hookplan::call_rel32(at as u64, thunk as u64) {
            done[k] = super::hook::patch(at, c.expect, &call, c.name);
        }
    }
    done
}

/// The F9 trace's menu token: list and cursor, greys, the tab, this frame's events.
pub fn trace_token() -> String {
    let ev = EVENTS.lock().map(|mut g| std::mem::take(&mut *g)).unwrap_or_default();
    match STATE.try_lock().ok().and_then(|s| s.menu.clone()) {
        Some(m) if LIVE.load(Ordering::Relaxed) => m.trace_token(&ev),
        _ => "menu -".to_string(),
    }
}

/// Clears the trace's events (a new recording starts).
pub fn clear_events() {
    if let Ok(mut g) = EVENTS.lock() {
        g.clear();
    }
}
