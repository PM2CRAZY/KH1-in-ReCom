//! M8d: KH1's items in Re:CoM's own Moogle shop screens (`crate::shop`, `docs/recom-m8-items.md`
//! § M8d).
//!
//! Re:CoM's shop runs as Re:CoM's: its two-Moogle menu (mode 3) unpatched. Two wrappers, both
//! acting only at Combat = KH1 in Sora's story (KH1's items live: `super::mp::active`):
//!
//! - **The pack screen** (mode 4's update `0x1403A6930`): an Items tab drawn first, before
//!   Re:CoM's four pack tabs, which the screen opens on. Entering it, Re:CoM's packs are released
//!   as its own state 0x65 releases them, Re:CoM's own pack constructor makes one pack per item
//!   (its price plate, never sold out), each pack's art layer is hidden and Re:CoM's own card
//!   picture (`C2DCard`) of the item's card is attached to it. On the Items tab Re:CoM's input state
//!   is replayed for its cells (`crate::shop::items_step`), a buy pays through Re:CoM's own payment
//!   and gives the item by KH1's pickup rule. Leaving for Re:CoM's tab, Re:CoM's packs are made
//!   again and Re:CoM's own refresh runs. With the asset step's five-tab art (`kh1-shop-tabs.json`)
//!   Re:CoM's tab tables are pointed at the art's places for the screen (`0x14065BA40`,
//!   `0x140690420`: written at each screen start, Re:CoM's own values at Combat = Re:CoM) and the
//!   icons' sequence plays the art's icons.
//! - **The card trade** (mode 5's update `0x1403A73E0`): its Square goes to the pack screen also
//!   without room for 5 cards (Re:CoM's needs room), so items can be bought with a full deck; its
//!   Square button hint is made then too, as Re:CoM makes it. And it gets an **Items tab** (the
//!   sell screen, M8d step 3: `crate::shop::sell`), first in its strip, which it opens on: Re:CoM's
//!   own stock window fill draws Sora's items from a card list of ours (Re:CoM's item card
//!   pictures, its big card, its small card box), Re:CoM's cursors and sounds, a count step of
//!   ours on the charge point window, Re:CoM's own yes / no (its states 3 / 4) with our line, and
//!   the sale through Re:CoM's own Moogle Point add. With the art the strip table (`0x1406904D0`)
//!   is pointed at the six-cell strips and Re:CoM's ALL finger constant (`0x14045B9D7`: 62.0) at
//!   Re:CoM's own 50.0 for the screen (Re:CoM's values at Combat = Re:CoM).
//!
//! Nothing of ours runs outside those two screens.

use super::movement::{func, orig, rd, wr};
use super::{base, log};
use crate::shop::{self as sh, Buy, Note, Pad, Step};
use crate::text_fit::recom as tr;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

pub static ORIG_PACK_SCREEN: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_TRADE: AtomicUsize = AtomicUsize::new(0);

static INSTALLED: AtomicBool = AtomicBool::new(false);
static LOG_LEFT: AtomicU32 = AtomicU32::new(80);
/// The five-tab art's first group in `m_shop:01` (0: no art).
static ART: AtomicU32 = AtomicU32::new(0);
/// The Items tab, while it is up (one shop at a time).
static TAB: Mutex<Option<ItemsTab>> = Mutex::new(None);
/// The card trade's Items tab, while it is up.
static SELL: Mutex<Option<SellTab>> = Mutex::new(None);

/// Re:CoM's `CMShopChargeCardData` as the stock window reads it (`0x14045DA80`, `0x14045BBF0`):
/// `+8` the records (298 of 16 bytes: u16 card kind, u16 count (0: none), the category byte, the
/// count of each value 0..9), `+0x10 + tab × 4` the cards per tab (tab 4 = all), `+0x24..+0x30`
/// the cards per category (the window's "cards in" counts). Checked (decomp `0x14045D510`).
const DATA_HEAD: usize = 0x50;
const RECORDS: usize = 298;
const RECORD: usize = 16;
const DATA_RECORDS: usize = 8;
const DATA_PER_TAB: usize = 0x10;
const DATA_PER_CATEGORY: usize = 0x24;
/// Re:CoM's fields copied into ours: `+0x24..+0x4C` (the per-category counts, their sum, the deck
/// counts; the last field Re:CoM writes is `+0x48`).
const DATA_COPY: usize = 0x28;
/// Re:CoM's item-card category (records' `+4`).
const ITEM_CATEGORY: u8 = 2;

struct SellTab {
    shop: usize,
    /// Sora's items and counts (`crate::shop::sell_list`).
    items: Vec<(u8, u32)>,
    in_grid: bool,
    cell: i32,
    qty: u32,
    /// Our card list for Re:CoM's stock window (its head points at the records).
    head: Box<[u8; DATA_HEAD]>,
    records: Box<[u8; RECORDS * RECORD]>,
    /// The yes / no line (ours, `\n` where we prefer a new line), set once the window is up.
    line: String,
    /// The line as laid out in the bubble, which Re:CoM's text window points at now, and the ones
    /// before (Re:CoM's text set keeps the pointer: `0x140259000`).
    shown: Vec<u8>,
    old: Vec<Vec<u8>>,
    asked: bool,
}

struct ItemsTab {
    shop: usize,
    /// Re:CoM's pack count, given back when the tab goes.
    count: i32,
    /// The items in the cells.
    items: Vec<u8>,
    /// Our card pictures (deleted before their packs are released).
    cards: [usize; sh::CELLS],
    note: Option<Note>,
    /// Our line shown now (`\n` where we prefer a new line).
    help: String,
    /// The line as laid out in the bubble, which Re:CoM's text window points at now, and the ones
    /// before (Re:CoM's text set keeps the pointer: `0x140259000`).
    shown: Vec<u8>,
    old: Vec<Vec<u8>>,
    /// The art's window was played for the Items tab.
    window: bool,
}

// Re:CoM functions (RVAs; first bytes checked: `sites::SHOP_CALLEES`, and `CALLEES` etc. for the
// shared ones).
const NEW: u32 = 0x004A_CE54;
const SOUND: u32 = 0x002A_A360;
const PAD_REPEAT: u32 = 0x0029_B2E0;
const PAD_FRESH: u32 = 0x0029_B2F0;
const PAD_CANCEL: u32 = 0x0029_B410;
const PAD_CONFIRM: u32 = 0x0029_B490;
const FLOOR: u32 = 0x0023_37C0;
const REFRESH: u32 = 0x003A_92F0;
const PACK_CTOR: u32 = 0x0045_A9E0;
const PACK_PRICE: u32 = 0x0045_D120;
const PACK_SOLD_OUT: u32 = 0x0045_D3F0;
const MESSAGE_PLACE: u32 = 0x0045_9AF0;
const CURSOR_PLACE: u32 = 0x0045_9A00;
const CURSOR_VISIBLE: u32 = 0x0045_97D0;
const POINT_WINDOW: u32 = 0x0045_98A0;
const SHOW_STATE: u32 = 0x0045_98D0;
const BUTTON_CTOR: u32 = 0x0045_A940;
const TEXTWND_TEXT: u32 = 0x002B_0EF0;
const POINTS_SET: u32 = 0x0023_A1D0;
const VAR_GET: u32 = 0x001B_08C0;
const VAR_SET: u32 = 0x001B_16C0;
const TROPHY: u32 = 0x002B_A6F0;
const CARD_CTOR: u32 = 0x0024_32A0;
const CARD_SET: u32 = 0x0024_3560;
const CARD_PRIORITY: u32 = 0x0024_3380;
const CARD_ATTACH: u32 = 0x0024_3DD0;
const CARD_VISIBLE: u32 = 0x0024_3A60;
const SLOT_VISIBLE: u32 = 0x0024_7E50;
const SPRITE_POS: u32 = 0x0024_85B0;
const STOCK_FILL: u32 = 0x0045_DA80;
const RANK_SET: u32 = 0x0045_D9C0;
const INFO_SET: u32 = 0x0045_A260;
const CHARGE_SET: u32 = 0x0045_D2C0;
const TAB_FINGER: u32 = 0x0045_B950;
const CELL_FINGER: u32 = 0x0045_B8B0;
const RANK_SPIN: u32 = 0x0045_B810;
const CURSOR_MOVE: u32 = 0x0045_9960;
const CURSOR_LOOK: u32 = 0x0045_9D60;
const TEXT_MESSAGE: u32 = 0x0045_95A0;
const TEXT_SET: u32 = 0x0025_8DE0;
const HELP_BAR: u32 = 0x0045_9CC0;
const POINTS_ADD: u32 = 0x0023_9FA0;
/// `C2DCard`'s vtable (`0x1402432A0` writes it); its slot 0 is the deleting dtor `0x1402073D0`.
/// Checked (disasm `0x1402432AE`; exe).
const CARD_VTABLE: usize = 0x0060_6BF8;
const CARD_DTOR: usize = 0x0020_73D0;

// Re:CoM data (RVAs). Checked (exe: the rip operands in `sites::SHOP_CHECKED`).
/// `*0x14087CBF8` = the save's stats; Moogle Points at `+0x20`.
const STATS_PTR: usize = 0x0087_CBF8;
const POINTS: usize = 0x20;
/// `*0x14087A7E8 + 6` (u16): the card limit of the room rule.
const CARD_LIMIT_PTR: usize = 0x0087_A7E8;
const CARD_LIMIT: usize = 6;
const TROPHY_FLAGS: usize = 0x00AF_7389;
const TROPHY_OBJ: usize = 0x00AF_7468;
/// Card class rows (`0x14068E0B0 + kind × 16`, byte 0).
const CARD_CLASS: usize = 0x0068_E0B0;
/// The tab x table (4 floats) and the window groups by tab (4 i16) then their closes (4 i16), the
/// only readers being the pack screen (`0x1403A6B3E`, `0x1403A9323` / `9434` / `9485`) and its
/// window (`0x14045D282`, `0x14045CCD9` / `CCFD`). Checked (exe scan).
const TAB_X_TABLE: usize = 0x0065_BA40;
const WINDOW_TABLE: usize = 0x0069_0420;
/// The pointer to `"m_shop.lad"` the shop's sequences play from (`0x14045D250`). Checked (decomp).
const SHOP_LAD: usize = 0x0085_9C48;
/// The card trade's strip groups by tab (5 i16, `0x14045DB92` its only reader). Checked (exe scan).
const STRIP_TABLE: usize = 0x0069_04D0;
/// Re:CoM's ALL finger: `subss xmm1, [rip → 0x1405092A8 (62.0)]` at `0x14045B9D7`; ours points the
/// same instruction at `0x140509298` (50.0, Re:CoM's float pool). Checked (disasm + exe).
const ALL_FINGER_AT: usize = 0x0045_B9D7;
const ALL_FINGER_RECOM: [u8; 8] = [0xF3, 0x0F, 0x5C, 0x0D, 0xC9, 0xD8, 0x0A, 0x00];
const ALL_FINGER_OURS: [u8; 8] = [0xF3, 0x0F, 0x5C, 0x0D, 0xB9, 0xD8, 0x0A, 0x00];
/// The yes / no finger's y (2 floats, 214 / 237) and its x (232.0) (`0x1403A73E0` case 2).
const YES_NO_Y: usize = 0x0065_BA38;
const YES_NO_X: f32 = 232.0;

/// `CMoogleShopCore` fields (Checked (decomp + disasm: `0x1403A4220`, `0x1403A4A90`,
/// `0x1403A6930`, `0x1403A73E0`, `0x1403A92F0`, `0x1403A5B10`)).
mod f {
    pub const NEXT_MODE: usize = 0x25C;
    pub const STATE: usize = 0x260;
    /// The shop's task parent (the constructors' second argument).
    pub const TASKS: usize = 0x268;
    pub const LOCK: usize = 0x274;
    pub const TAB: usize = 0x280;
    pub const CELL: usize = 0x284;
    pub const PRICES: usize = 0x2C8;
    pub const SOLD_OUT: usize = 0x308;
    pub const COUNT: usize = 0x348;
    pub const IN_GRID: usize = 0x34C;
    /// `CMShopChargeCardData`; `+0x44` Sora's card count for the room rule.
    pub const CARD_DATA: usize = 0x350;
    pub const CARD_COUNT: usize = 0x44;
    pub const POINT_WINDOW: usize = 0x3C0;
    pub const CURSOR: usize = 0x3D0;
    pub const TAB_MARK: usize = 0x3D8;
    /// `CMShopTextWindow` (the Moogle's text).
    pub const TEXT: usize = 0x3E8;
    /// The pack screen's icons (`CMShopGSeq`).
    pub const ICONS: usize = 0x408;
    pub const TAB_MESSAGE: usize = 0x410;
    /// `CMShopCardPackWindow`.
    pub const WINDOW: usize = 0x418;
    pub const CELL_MESSAGE: usize = 0x420;
    pub const PACKS: usize = 0x428;
    /// The card trade's Square button hint (`CMShopButtonMessage`, made only with room).
    pub const TRADE_BUTTON: usize = 0x448;
    /// The card trade (`0x1403A4D90`, `0x1403A73E0`): the yes / no choice, the grid flag, the tab,
    /// the scroll; the help bar, the grid finger, the value finger, its
    /// windows (the charge points, the 0–9 box, the big card, the stock window).
    pub const YES_NO: usize = 0x270;
    pub const T_IN_GRID: usize = 0x358;
    pub const T_TAB: usize = 0x35C;
    pub const T_SCROLL: usize = 0x360;
    pub const HELP_BAR: usize = 0x3C8;
    pub const GRID_FINGER: usize = 0x3D8;
    pub const VALUE_FINGER: usize = 0x3E0;
    pub const CHARGE: usize = 0x450;
    pub const RANK: usize = 0x458;
    pub const INFO: usize = 0x460;
    pub const STOCK: usize = 0x468;
}
/// `CMShopStockCardWnd +0x80`: the strip's control. A shop cursor's `+0x60`: place at once next.
const STOCK_STRIP: usize = 0x80;
const SNAP: usize = 0x60;
/// The help bar's text (`0x140459CC0`).
const BAR_TEXT: usize = 0x78;
/// The card trade's help bar id (its init `0x1403A4D90`).
const BAR_TRADE: i32 = 5;
/// Cursor looks (`0x140459D60`): the finger, the grid finger, the grid finger on a chosen card.
const LOOK_FINGER: i32 = 0;
const LOOK_GRID: i32 = 3;
const LOOK_CHOSEN: i32 = 4;
const LOOK_YES_NO: i32 = 6;
/// The text window's yes / no message (`0x1404595A0`).
const MESSAGE_YES_NO: i32 = 7;
/// The text window closed (`+0x58`, the sale waits for it).
const TW_CLOSED: i32 = -1;
/// `CMShopButtonMessage +0x60` / any shop window's show state (`0x1404598D0`).
const SHOW: usize = 0x60;
/// `CMShopCardPack`: its picture control (layer 1 the pack's art), its release flag (`+0x6C`).
const PACK_PICTURE: usize = 0x78;
const PACK_RELEASE: usize = 0x6C;
const PACK_SIZE: usize = 0xB8;
const PICTURE_ART_LAYER: i32 = 1;
/// `C2DCard`: its size (`0x14045BF50` allocates 400), the shop's card priority 0x12, "use the
/// colour" `+0x138`.
const CARD_SIZE: usize = 400;
const CARD_PRIORITY_SHOP: u32 = 0x12;
const C_COLOUR_ON: usize = 0x138;
/// `CMShopCardPackWindow` (`0x14045AD20`, `0x14045D250`, `0x14045CC60`): `+0x58` its phase (1 =
/// shown), `+0x5C` the tab it shows, `+0x60` close, `+0x68` its sequence control.
const WIN_PHASE: usize = 0x58;
const WIN_TAB: usize = 0x5C;
const WIN_CLOSE: usize = 0x60;
const WIN_CTRL: usize = 0x68;
/// `CMShopGSeq` (`0x140455F20`, `0x140457DF0`, `0x1404585B0`): `+0x5C` the group playing (−1 not
/// yet), `+0x68` its control, `+0x70` its sequence set, `+0x72` / `+0x74` / `+0x76` appear /
/// loop / close.
const GSEQ_PLAYING: usize = 0x5C;
const GSEQ_CTRL: usize = 0x68;
const GSEQ_SET: usize = 0x70;
const GSEQ_GROUPS: usize = 0x72;
/// `CMShopTextWindow`: `+0x58` 1 open; `+0x5C` its help id (`0x1404573A0`); `+0x60` its `CTextWnd`.
const TW_STATE: usize = 0x58;
const TW_HELP: usize = 0x5C;
const TW_MAIN: usize = 0x60;
/// `CTextWnd` state `+0x88` (5 shown).
const TEXTWND_STATE: usize = 0x88;
const SHOWN: i32 = 5;
/// The card trade's button hint: 0x88 bytes, kind 1 (`0x1403A73E0` case 5).
const BUTTON_SIZE: usize = 0x88;
const BUTTON_TRADE: u32 = 1;

/// The pack screen's states (`0x1403A6930`) and the card trade's (`0x1403A73E0`).
const STATE_FIRST: i32 = 0;
const STATE_INPUT: i32 = 2;
const STATE_CLOSE: i32 = 100;
const STATE_RELEASE: i32 = 0x65;
const TRADE_TABS: i32 = 1;
const TRADE_GRID: i32 = 2;
const TRADE_ASK: i32 = 3;
const TRADE_SALE: i32 = 5;

type ScreenFn = unsafe extern "win64" fn(usize);
type NoArgFn = unsafe extern "win64" fn() -> u32;
type NewFn = unsafe extern "win64" fn(usize) -> usize;
type SoundFn = unsafe extern "win64" fn(u32, i32) -> u64;
type PadMaskFn = unsafe extern "win64" fn(u32, u32) -> u32;
type PadBtnFn = unsafe extern "win64" fn(u32) -> u32;
type PackCtorFn = unsafe extern "win64" fn(usize, usize, i32, i32, i32) -> usize;
type PackIntFn = unsafe extern "win64" fn(usize, i32) -> u64;
type PackSoldFn = unsafe extern "win64" fn(usize, i32, i32) -> u64;
type PlaceFn = unsafe extern "win64" fn(usize, f32, f32);
type CursorPlaceFn = unsafe extern "win64" fn(usize, f32, f32, i32);
type VisibleFn = unsafe extern "win64" fn(usize, u32);
type ButtonCtorFn = unsafe extern "win64" fn(usize, usize, u32) -> usize;
type SetUpFn = unsafe extern "win64" fn(usize);
type PlayFn = unsafe extern "win64" fn(usize, u32, usize, u32, i32, f32, u32);
type TextFn = unsafe extern "win64" fn(usize, *const u8, usize);
type PointsSetFn = unsafe extern "win64" fn(i32) -> u64;
type VarGetFn = unsafe extern "win64" fn(u32, u32) -> u64;
type VarSetFn = unsafe extern "win64" fn(u32, u32, i32);
type TrophyFn = unsafe extern "win64" fn(usize, u32);
type CtorFn = unsafe extern "win64" fn(usize) -> usize;
type CardSetFn = unsafe extern "win64" fn(usize, u32);
type AttachFn = unsafe extern "win64" fn(usize, usize);
type SlotVisibleFn = unsafe extern "win64" fn(usize, i32, u32);
type PosFn = unsafe extern "win64" fn(usize, *const f32);
type DeleteFn = unsafe extern "win64" fn(usize, u32) -> usize;
type FillFn = unsafe extern "win64" fn(usize, usize, i32, i32, i32);
type RankFn = unsafe extern "win64" fn(usize, usize, i32);
type InfoFn = unsafe extern "win64" fn(usize, u64);
type FingerFn = unsafe extern "win64" fn(usize, *mut f32, i32);
type MoveFn = unsafe extern "win64" fn(usize, *const f32, i32);
type LookFn = unsafe extern "win64" fn(usize, i32);
type ObjFn = unsafe extern "win64" fn(usize);
type SetTextFn = unsafe extern "win64" fn(usize, *const u8);

#[repr(C, align(16))]
struct V4([f32; 4]);

fn note(line: impl FnOnce() -> String) {
    if LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("shop: {}", line()));
    }
}

pub fn mark_installed() {
    INSTALLED.store(true, Ordering::Release);
}

/// Reads `kh1-shop-tabs.json` (the asset step's five-tab art and six-cell trade strips,
/// `crate::shop::tabs`). Without it the pack screen keeps Re:CoM's four-tab window and the card
/// trade Re:CoM's strip, and the Items tabs' cursors sit right of the tabs.
pub fn load_tabs(file: Option<&std::path::Path>) {
    let Some(path) = file else {
        log("shop: no kh1-shop-tabs.json: the pack screen and the card trade keep Re:CoM's tab art (the Items tabs' cursors right of Re:CoM's tabs)");
        return;
    };
    match std::fs::read_to_string(path).map_err(|e| e.to_string()).and_then(|t| sh::tabs::parse(&t)) {
        Ok(first) => {
            ART.store(first, Ordering::Release);
            log(&format!("shop: tab art from {}: m_shop:01 groups {first}..{} (the pack screen's five tabs, the card trade's six-cell strips)", path.display(), first + sh::tabs::COUNT - 1));
        }
        Err(e) => log(&format!("shop: {}: {e}: the pack screen and the card trade keep Re:CoM's tab art", path.display())),
    }
}

fn art() -> Option<u32> {
    match ART.load(Ordering::Acquire) {
        0 => None,
        f => Some(f),
    }
}

/// KH1's items are live (Combat = KH1, Sora's story, a save) and the wrappers are in.
/// 0.163.0 (fork owner, 2026-11-23: Sora's features for Riku): in Riku's story too (his items).
fn live() -> bool {
    INSTALLED.load(Ordering::Acquire) && super::items::installed() && super::mp::live()
}

fn lock() -> std::sync::MutexGuard<'static, Option<ItemsTab>> {
    TAB.lock().unwrap_or_else(|p| p.into_inner())
}

fn tab_up(shop: usize) -> bool {
    lock().as_ref().is_some_and(|t| t.shop == shop)
}

unsafe fn sound(id: u32) {
    func::<SoundFn>(SOUND)(id, -1);
}

unsafe fn points() -> i32 {
    let s: usize = rd(base() + STATS_PTR);
    if s == 0 {
        return 0;
    }
    rd(s + POINTS)
}

/// Re:CoM's room rule (`0x1403A752A`). No card data or limit: no room.
unsafe fn room(shop: usize) -> bool {
    let limit: usize = rd(base() + CARD_LIMIT_PTR);
    let data: usize = rd(shop + f::CARD_DATA);
    if limit == 0 || data == 0 {
        return false;
    }
    sh::room_for_pack(rd::<u16>(limit + CARD_LIMIT), rd::<i32>(data + f::CARD_COUNT))
}

/// Re:CoM's floor tier now (`0x1402337C0`): the shop's stock, and Lucky Bounty at KH1.
pub(super) unsafe fn tier() -> sh::Tier {
    let byte = func::<NoArgFn>(FLOOR)() as u8;
    sh::tier(sh::floor_of(byte))
}

unsafe fn read_pad() -> Pad {
    let dir = |mask: u32| func::<PadMaskFn>(PAD_REPEAT)(mask, 0) != 0;
    Pad {
        other: func::<PadMaskFn>(PAD_FRESH)(0x80, 0) != 0,
        cancel: func::<PadBtnFn>(PAD_CANCEL)(0) != 0,
        confirm: func::<PadBtnFn>(PAD_CONFIRM)(0) != 0,
        down: dir(0x0020_4000),
        right: dir(0x0080_2000),
        up: dir(0x0010_1000),
        left: dir(0x0040_8000),
    }
}

// ---- The five-tab art ----

fn table_bytes(x: [f32; 4], w: [i16; 4], c: [i16; 4]) -> ([u8; 16], [u8; 16]) {
    let mut a = [0u8; 16];
    let mut b = [0u8; 16];
    for i in 0..4 {
        a[4 * i..4 * i + 4].copy_from_slice(&x[i].to_le_bytes());
        b[2 * i..2 * i + 2].copy_from_slice(&w[i].to_le_bytes());
        b[8 + 2 * i..10 + 2 * i].copy_from_slice(&c[i].to_le_bytes());
    }
    (a, b)
}

/// Re:CoM's tab tables as Re:CoM has them, or at the art's places (`crate::shop::tabs`). Written
/// only over one of the two (anything else: left alone, logged).
unsafe fn set_tables(kh1: Option<u32>) {
    let recom = table_bytes(sh::RECOM_TAB_X, sh::tabs::RECOM_WINDOW, sh::tabs::RECOM_CLOSE);
    let ours = kh1.map(|first| {
        let (x, w, c) = sh::tabs::kh1_tables(first);
        table_bytes(x, w, c)
    });
    let want = ours.unwrap_or(recom);
    for (at, recom, ours, want, what) in [
        (TAB_X_TABLE, recom.0, ours.map(|o| o.0), want.0, "shop tab x table"),
        (WINDOW_TABLE, recom.1, ours.map(|o| o.1), want.1, "shop window groups"),
    ] {
        let at = base() + at;
        let have: [u8; 16] = rd(at);
        if have == want {
            continue;
        }
        if have != recom && Some(have) != ours && !(at == base() + WINDOW_TABLE && closes_only_differ(&have, &want)) {
            note(|| format!("{what} at {at:#x} holds {have:02x?}: left alone"));
            continue;
        }
        super::hook::patch(at, &have, &want, what);
    }
}

/// The window table with only its close entries changed by us (the Items tab's close, below).
fn closes_only_differ(have: &[u8; 16], want: &[u8; 16]) -> bool {
    have[..8] == want[..8]
}

/// The window's close for the Items tab: Re:CoM's close plays `closes[window tab]`
/// (`0x14045CC60`), so the window is given tab 0 and tab 0's close entry the art's place-0 close
/// (until the next screen start writes the tables again).
unsafe fn items_close(shop: usize, first: u32) {
    let w: usize = rd(shop + f::WINDOW);
    if w == 0 {
        return;
    }
    wr::<i32>(w + WIN_TAB, 0);
    let at = base() + WINDOW_TABLE + 8;
    let have: [u8; 2] = rd(at);
    let want = ((first + sh::tabs::CLOSE) as i16).to_le_bytes();
    if have != want {
        super::hook::patch(at, &have, &want, "shop window close for the Items tab");
    }
}

/// Plays the art's window with place 0 raised for the Items tab, once the window is shown
/// (`0x14045D250`'s rule), and marks it as showing no Re:CoM tab (any Re:CoM tab plays its own
/// again).
unsafe fn items_window(shop: usize, t: &mut ItemsTab, first: u32) {
    let w: usize = rd(shop + f::WINDOW);
    if t.window || w == 0 || rd::<i32>(w + WIN_PHASE) != 1 || rd::<i32>(w + WIN_CLOSE) != 0 {
        return;
    }
    let ctrl: usize = rd(w + WIN_CTRL);
    let lad: usize = rd(base() + SHOP_LAD);
    if ctrl == 0 || lad == 0 {
        return;
    }
    let play: PlayFn = std::mem::transmute::<usize, PlayFn>(rd::<usize>(rd::<usize>(ctrl) + 0x18));
    play(ctrl, 0, lad, 1, (first + sh::tabs::WINDOW) as i32, 0.0, 0);
    wr::<i32>(w + WIN_TAB, sh::ITEMS_TAB);
    t.window = true;
}

/// The icons' sequence plays the art's icons (Re:CoM's mode-4 init gave it 0x38 / 0x39 / 0x3A).
unsafe fn icons(shop: usize, first: u32) {
    let g: usize = rd(shop + f::ICONS);
    if g == 0 {
        return;
    }
    let have: [i16; 3] = rd(g + GSEQ_GROUPS);
    if have != sh::tabs::RECOM_ICONS {
        return;
    }
    let ours = [sh::tabs::ICONS_IN, sh::tabs::ICONS, sh::tabs::ICONS_OUT].map(|o| (first + o) as i16);
    wr::<[i16; 3]>(g + GSEQ_GROUPS, ours);
    if rd::<i32>(g + GSEQ_PLAYING) != -1 {
        // Already started on Re:CoM's: our appear from the start.
        let ctrl: usize = rd(g + GSEQ_CTRL);
        let lad: usize = rd(base() + SHOP_LAD);
        if ctrl != 0 && lad != 0 {
            let play: PlayFn = std::mem::transmute::<usize, PlayFn>(rd::<usize>(rd::<usize>(ctrl) + 0x18));
            play(ctrl, 0, lad, rd::<i16>(g + GSEQ_SET) as u32, ours[0] as i32, 0.0, 0);
        }
    }
}

// ---- The packs and their cards ----

unsafe fn pack(shop: usize, i: usize) -> usize {
    rd(shop + f::PACKS + i * 8)
}

/// Releases the packs in the array (as state 0x65: `+0x6C` = 1, the entry cleared).
unsafe fn release_packs(shop: usize, n: usize) {
    for i in 0..n.min(sh::CELLS) {
        let p = pack(shop, i);
        if p != 0 {
            wr::<u32>(p + PACK_RELEASE, 1);
            wr::<usize>(shop + f::PACKS + i * 8, 0);
        }
    }
}

/// A pack as state 0 makes it (`0x1403A6AD0`): 0xB8 bytes, (task parent, cell, price, sold out).
unsafe fn make_pack(shop: usize, cell: i32, price: i32, sold_out: i32) -> usize {
    let mem = func::<NewFn>(NEW)(PACK_SIZE);
    if mem == 0 {
        return 0;
    }
    func::<PackCtorFn>(PACK_CTOR)(mem, rd(shop + f::TASKS), cell, price, sold_out)
}

/// Re:CoM's card picture of an item card on `pack`'s picture control, as the shop's own card in
/// `0x14045BF50` (priority 0x12, colour on), as the "cards got" screen makes its cards.
unsafe fn make_card(pack: usize, kind: u8) -> usize {
    let ctrl: usize = rd(pack + PACK_PICTURE);
    if ctrl == 0 {
        return 0;
    }
    let mem = func::<NewFn>(NEW)(CARD_SIZE);
    if mem == 0 {
        return 0;
    }
    let card = func::<CtorFn>(CARD_CTOR)(mem);
    if card == 0 || rd::<usize>(card) != base() + CARD_VTABLE {
        note(|| "a card picture's vtable is not C2DCard's; no picture".into());
        return 0;
    }
    let class: u8 = rd(base() + CARD_CLASS + kind as usize * 16);
    func::<CardSetFn>(CARD_SET)(card, sh::card_value(kind, class));
    func::<CardSetFn>(CARD_PRIORITY)(card, CARD_PRIORITY_SHOP);
    func::<AttachFn>(CARD_ATTACH)(card, ctrl);
    func::<PosFn>(SPRITE_POS)(card, V4([0.0, 0.0, 0.0, 1.0]).0.as_ptr());
    wr::<u8>(card + C_COLOUR_ON, 1);
    func::<VisibleFn>(CARD_VISIBLE)(card, 1);
    card
}

/// Deletes a card picture (its own deleting dtor, `C2DCard` vtable slot 0, flag 1).
unsafe fn delete_card(card: usize) {
    if card == 0 {
        return;
    }
    let dtor: usize = rd(rd::<usize>(card));
    if dtor != base() + CARD_DTOR {
        note(|| format!("a card picture's dtor is {dtor:#x}, not 0x1402073D0; left in place"));
        return;
    }
    let delete: DeleteFn = std::mem::transmute::<usize, DeleteFn>(dtor);
    delete(card, 1);
}

/// Into the Items tab from Re:CoM's tab `from` (its packs, `released` of them, already made).
unsafe fn enter(shop: usize, from: i32, count: i32, released: usize) {
    release_packs(shop, released);
    let items = sh::buy_list(tier());
    let mut cards = [0usize; sh::CELLS];
    let mut made = 0;
    for (i, &item) in items.iter().take(sh::CELLS).enumerate() {
        let p = make_pack(shop, i as i32, sh::buy_price(item).unwrap_or(0), 0);
        wr::<usize>(shop + f::PACKS + i * 8, p);
        if p == 0 {
            break;
        }
        made += 1;
        func::<SlotVisibleFn>(SLOT_VISIBLE)(rd(p + PACK_PICTURE), PICTURE_ART_LAYER, 0);
        if let Some(k) = sh::card_kind(item) {
            cards[i] = make_card(p, k);
        }
    }
    for i in made..sh::CELLS {
        wr::<usize>(shop + f::PACKS + i * 8, 0);
    }
    let n = items.len().min(sh::CELLS);
    *lock() = Some(ItemsTab { shop, count, items: items.clone(), cards, note: None, help: String::new(), shown: Vec::new(), old: Vec::new(), window: false });
    if made < n {
        note(|| format!("Items tab: only {made} of {n} packs made (no memory); back to tab {from}"));
        wr::<i32>(shop + f::COUNT, made as i32);
        leave_to(shop, from.clamp(0, 3));
        return;
    }
    wr::<i32>(shop + f::COUNT, n as i32);
    wr::<i32>(shop + f::TAB, sh::ITEMS_TAB);
    wr::<i32>(shop + f::CELL, rd::<i32>(shop + f::CELL).clamp(0, n as i32 - 1));
    refresh(shop);
    note(|| format!("Items tab from tab {from}: Re:CoM's {count} packs released, {:?}; cards {cards:#x?}; {} Moogle Points", items.iter().map(|&i| crate::items::name(i)).collect::<Vec<_>>(), points()));
}

/// Takes the Items tab's state, deletes its cards; None when it isn't up for `shop`.
unsafe fn take_tab(shop: usize) -> Option<ItemsTab> {
    let t = lock().take_if(|t| t.shop == shop)?;
    for c in t.cards {
        delete_card(c);
    }
    Some(t)
}

/// From the Items tab to Re:CoM's tab `to`: Re:CoM's packs for it as state 0 makes them, then
/// Re:CoM's refresh. Re:CoM's text window line is Re:CoM's again (its help id forgotten).
unsafe fn leave_to(shop: usize, to: i32) {
    let Some(t) = take_tab(shop) else { return };
    let ours: i32 = rd(shop + f::COUNT);
    release_packs(shop, ours.clamp(0, sh::CELLS as i32) as usize);
    let n = t.count.clamp(0, 4);
    for i in 0..n {
        let k = (to * 4 + i) as usize * 4;
        let p = make_pack(shop, i, rd(shop + f::PRICES + k), rd(shop + f::SOLD_OUT + k));
        wr::<usize>(shop + f::PACKS + i as usize * 8, p);
    }
    wr::<i32>(shop + f::COUNT, t.count);
    wr::<i32>(shop + f::TAB, to);
    wr::<i32>(shop + f::CELL, rd::<i32>(shop + f::CELL).clamp(0, (t.count - 1).max(0)));
    forget_help(shop);
    func::<ScreenFn>(REFRESH)(shop);
    note(|| format!("Items tab -> tab {to}: Re:CoM's {} packs made again", t.count));
}

/// Re:CoM's help line call sets its message only when the id changes (`0x1404573A0`): forgetting
/// it makes Re:CoM's next call set Re:CoM's line again.
unsafe fn forget_help(shop: usize) {
    let tw: usize = rd(shop + f::TEXT);
    if tw != 0 {
        wr::<i32>(tw + TW_HELP, -1);
        // Re:CoM's own line goes on the same text: never squeezed.
        super::text_fit::unsqueeze(bubble_fnt(rd(tw + TW_MAIN)));
    }
}

/// Re:CoM's refresh (`0x1403A92F0`) for the Items tab: the button messages at the tab and the
/// cell, the packs' prices (never sold out), the cursors, the window.
unsafe fn refresh(shop: usize) {
    let five = art();
    let x = sh::tab_x(sh::ITEMS_TAB, five.is_some());
    let n = rd::<i32>(shop + f::COUNT).clamp(0, sh::CELLS as i32);
    let cell = rd::<i32>(shop + f::CELL).clamp(0, (n - 1).max(0)) as usize;
    let place = func::<PlaceFn>(MESSAGE_PLACE);
    place(rd(shop + f::TAB_MESSAGE), x, sh::TAB_MESSAGE_Y);
    place(rd(shop + f::CELL_MESSAGE), sh::CELL_X[cell], sh::CELL_Y[cell]);
    let items = lock().as_ref().filter(|t| t.shop == shop).map(|t| t.items.clone()).unwrap_or_default();
    for (i, &item) in items.iter().take(n as usize).enumerate() {
        let p = pack(shop, i);
        if p != 0 {
            func::<PackIntFn>(PACK_PRICE)(p, sh::buy_price(item).unwrap_or(0));
            func::<PackSoldFn>(PACK_SOLD_OUT)(p, 0, 0);
        }
    }
    let cursor: usize = rd(shop + f::CURSOR);
    let mark: usize = rd(shop + f::TAB_MARK);
    let show = func::<VisibleFn>(CURSOR_VISIBLE);
    let put = func::<CursorPlaceFn>(CURSOR_PLACE);
    if rd::<i32>(shop + f::IN_GRID) != 0 {
        show(mark, 1);
        put(mark, x - sh::CURSOR_DX, sh::TAB_CURSOR_Y, 0);
        show(cursor, 1);
        put(cursor, sh::CELL_X[cell] - sh::CURSOR_DX, sh::CELL_Y[cell] - sh::CURSOR_DY, 0);
    } else {
        show(cursor, 1);
        put(cursor, x - sh::CURSOR_DX, sh::TAB_CURSOR_Y, 0);
        show(mark, 0);
    }
    if let Some(first) = five {
        let mut g = lock();
        if let Some(t) = g.as_mut().filter(|t| t.shop == shop) {
            items_window(shop, t, first);
        }
    }
    help(shop);
}

/// The Items tab's line in the Moogle's text window (Re:CoM's text window, its own text call).
unsafe fn help(shop: usize) {
    let in_grid = rd::<i32>(shop + f::IN_GRID) != 0;
    let cell = rd::<i32>(shop + f::CELL).max(0) as usize;
    let inv = super::items::inventory().unwrap_or_default();
    let mut g = lock();
    let Some(t) = g.as_mut().filter(|t| t.shop == shop) else { return };
    let line = sh::help_text(in_grid, t.items.get(cell).copied(), t.note, &inv);
    if line == t.help && !t.shown.is_empty() {
        return;
    }
    let tw: usize = rd(shop + f::TEXT);
    if tw == 0 || rd::<i32>(tw + TW_STATE) != 1 {
        return;
    }
    let main: usize = rd(tw + TW_MAIN);
    if main == 0 || rd::<i32>(main + TEXTWND_STATE) != SHOWN {
        return;
    }
    let shown = bubble_line(main, &line, "pack screen, Items tab");
    func::<TextFn>(TEXTWND_TEXT)(main, shown.as_ptr(), 0);
    wr::<i32>(tw + TW_HELP, -1);
    t.help = line;
    let old = std::mem::replace(&mut t.shown, shown);
    if !old.is_empty() {
        t.old.push(old);
    }
    while t.old.len() > 2 {
        t.old.remove(0);
    }
}

/// Re:CoM's payment for `price` (state 4, `0x1403A7041`): the spent tally and its trophy, the
/// points, the Moogle Point window.
unsafe fn pay(shop: usize, price: i32, after: i32) {
    let flags = base() + TROPHY_FLAGS;
    if rd::<u8>(flags) & 0x10 == 0 {
        let (t, v) = sh::SPENT_VAR;
        let before = func::<VarGetFn>(VAR_GET)(t, v) as i32;
        let (spent, trophy) = sh::spent(before, price);
        func::<VarSetFn>(VAR_SET)(t, v, spent);
        if trophy {
            let o: usize = rd(base() + TROPHY_OBJ);
            if o != 0 {
                func::<TrophyFn>(TROPHY)(o, sh::SPENT_TROPHY);
            }
            wr::<u8>(flags, rd::<u8>(flags) | 0x10);
        }
    }
    func::<PointsSetFn>(POINTS_SET)(after);
    let w: usize = rd(shop + f::POINT_WINDOW);
    if w != 0 {
        func::<PackIntFn>(POINT_WINDOW)(w, points());
    }
}

unsafe fn buy(shop: usize, cell: i32) {
    let item = lock().as_ref().filter(|t| t.shop == shop).and_then(|t| t.items.get(cell.max(0) as usize).copied());
    let Some(item) = item else {
        sound(sh::sound::ERROR);
        return;
    };
    let have = points();
    let r = super::items::edit(|inv| sh::buy(item, have, inv));
    let n = match r {
        Some(Buy::Bought { price, points: after, to, .. }) => {
            sound(sh::sound::BUY);
            sound(sh::sound::BUY_TOO);
            pay(shop, price, after);
            note(|| format!("bought {} for {price}: {have} -> {} Moogle Points, {to:?}", crate::items::name(item), points()));
            None
        }
        Some(Buy::Poor { .. }) => Some(Note::Poor),
        Some(Buy::NoRoom { .. }) => Some(Note::NoRoom),
        _ => None,
    };
    if !matches!(r, Some(Buy::Bought { .. })) {
        sound(sh::sound::ERROR);
        note(|| format!("refused {} on cell {cell} with {have} Moogle Points: {r:?}", crate::items::name(item)));
    }
    if let Some(t) = lock().as_mut().filter(|t| t.shop == shop) {
        t.note = n;
    }
}

fn clear_note(shop: usize) {
    if let Some(t) = lock().as_mut().filter(|t| t.shop == shop) {
        t.note = None;
    }
}

/// One frame of the Items tab's input state.
unsafe fn items_frame(shop: usize) {
    let in_grid = rd::<i32>(shop + f::IN_GRID) != 0;
    let cell: i32 = rd(shop + f::CELL);
    let count: i32 = rd(shop + f::COUNT);
    let locked = rd::<i32>(shop + f::LOCK) > 0;
    let pad = if locked { Pad::default() } else { read_pad() };
    match sh::items_step(in_grid, cell, count, &pad, locked) {
        Step::Leave { sound: s, mode } => {
            sound(s);
            wr::<i32>(shop + f::NEXT_MODE, mode);
            wr::<i32>(shop + f::STATE, STATE_CLOSE);
            return;
        }
        Step::ToTab(t) => {
            sound(sh::sound::MOVE);
            leave_to(shop, t);
            return;
        }
        Step::IntoGrid => {
            sound(sh::sound::MOVE);
            wr::<i32>(shop + f::IN_GRID, 1);
            wr::<i32>(shop + f::CELL, 0);
            message_show(shop, 0);
            clear_note(shop);
        }
        Step::OutOfGrid => {
            sound(sh::sound::MOVE);
            wr::<i32>(shop + f::IN_GRID, 0);
            message_show(shop, -1);
            clear_note(shop);
        }
        Step::Move(c) => {
            sound(sh::sound::MOVE);
            wr::<i32>(shop + f::CELL, c);
            clear_note(shop);
        }
        Step::Confirm(c) => buy(shop, c),
        Step::Nothing => {}
    }
    refresh(shop);
}

/// The cell's button message's show state (`0x1404598D0`: one field write).
unsafe fn message_show(shop: usize, v: i32) {
    let m: usize = rd(shop + f::CELL_MESSAGE);
    if m != 0 {
        wr::<i32>(m + SHOW, v);
    }
}

/// The wrapper on the pack screen's update (`0x1403A6930`).
///
/// # Safety
/// Called by Re:CoM with its `CMoogleShopCore`.
pub unsafe extern "win64" fn pack_screen(shop: usize) {
    let run = orig::<ScreenFn>(&ORIG_PACK_SCREEN);
    let state: i32 = rd(shop + f::STATE);
    let up = tab_up(shop);
    match state {
        STATE_FIRST => {
            if up {
                // A new screen on the same object with our tab still recorded: it never passed
                // state 0x65 (not seen in Re:CoM's code). Its cards went with the old screen.
                *lock() = None;
                note(|| "a new pack screen with the Items tab still recorded: forgotten".into());
            }
            let kh1 = live();
            set_tables(art().filter(|_| kh1));
            if !kh1 {
                return run(shop);
            }
            if let Some(first) = art() {
                icons(shop, first);
            }
            // The screen opens on the Items tab: Re:CoM's state 0 makes no packs (count 0), ours
            // are made as it moves on.
            let count: i32 = rd(shop + f::COUNT);
            wr::<i32>(shop + f::COUNT, 0);
            run(shop);
            wr::<i32>(shop + f::COUNT, count);
            if rd::<i32>(shop + f::STATE) != STATE_FIRST {
                enter(shop, 0, count, 0);
            }
        }
        STATE_INPUT if up => items_frame(shop),
        STATE_INPUT if live() => {
            let tab: i32 = rd(shop + f::TAB);
            let in_grid = rd::<i32>(shop + f::IN_GRID) != 0;
            let locked = rd::<i32>(shop + f::LOCK) > 0;
            if (0..sh::RECOM_TABS).contains(&tab) && !in_grid && !locked && sh::enters_items(tab, in_grid, &read_pad(), locked) {
                sound(sh::sound::MOVE);
                let count: i32 = rd(shop + f::COUNT);
                enter(shop, tab, count, count.clamp(0, 4) as usize);
            } else {
                run(shop)
            }
        }
        STATE_CLOSE if up => {
            // Our cards go before Re:CoM starts closing the packs they hang from; the window
            // closes as the Items tab's.
            if let Some(t) = lock().as_mut().filter(|t| t.shop == shop) {
                for c in std::mem::take(&mut t.cards) {
                    delete_card(c);
                }
            }
            if let Some(first) = art() {
                items_close(shop, first);
            }
            run(shop)
        }
        STATE_RELEASE if up => {
            run(shop);
            if rd::<i32>(shop + f::STATE) != STATE_RELEASE {
                // Re:CoM released our packs (the count was ours): its count back.
                if let Some(t) = take_tab(shop) {
                    wr::<i32>(shop + f::COUNT, t.count);
                    note(|| format!("pack screen left from the Items tab: Re:CoM's count {} back", t.count));
                }
            }
        }
        _ => run(shop),
    }
}

/// The card trade's Square button hint, made as Re:CoM's case 5 makes it (`0x1403A7F8E`), when
/// Re:CoM made none (no room for 5 cards).
unsafe fn trade_button(shop: usize) {
    if rd::<usize>(shop + f::TRADE_BUTTON) != 0 {
        return;
    }
    let mem = func::<NewFn>(NEW)(BUTTON_SIZE);
    if mem == 0 {
        return;
    }
    let b = func::<ButtonCtorFn>(BUTTON_CTOR)(mem, rd(shop + f::TASKS), BUTTON_TRADE);
    if b == 0 {
        return;
    }
    wr::<usize>(shop + f::TRADE_BUTTON, b);
    let set_up: SetUpFn = std::mem::transmute::<usize, SetUpFn>(rd::<usize>(rd::<usize>(b) + 0x38));
    set_up(b);
    func::<PackIntFn>(SHOW_STATE)(b, 0);
}

// ---- The card trade's Items tab (the sell screen) ----

fn sell_lock() -> std::sync::MutexGuard<'static, Option<SellTab>> {
    SELL.lock().unwrap_or_else(|p| p.into_inner())
}

fn sell_up(shop: usize) -> bool {
    sell_lock().as_ref().is_some_and(|t| t.shop == shop)
}

/// The trade's strip table and Re:CoM's ALL finger constant as Re:CoM has them, or for the art
/// (written over one of the two only); with the table changed, Re:CoM's stock window drawn again
/// with it (its init drew it before this first update).
unsafe fn trade_tables(shop: usize, kh1: Option<u32>) {
    let mut recom = [0u8; 10];
    for (i, g) in sh::tabs::RECOM_STRIP.iter().enumerate() {
        recom[2 * i..2 * i + 2].copy_from_slice(&g.to_le_bytes());
    }
    let ours = kh1.map(|first| {
        let mut o = [0u8; 10];
        for (i, g) in sh::tabs::kh1_strip(first).iter().enumerate() {
            o[2 * i..2 * i + 2].copy_from_slice(&g.to_le_bytes());
        }
        o
    });
    let want = ours.unwrap_or(recom);
    let at = base() + STRIP_TABLE;
    let have: [u8; 10] = rd(at);
    if have != want {
        if have == recom || Some(have) == ours {
            if super::hook::patch(at, &have, &want, "card trade strip table") {
                let stock: usize = rd(shop + f::STOCK);
                let data: usize = rd(shop + f::CARD_DATA);
                if stock != 0 && data != 0 {
                    func::<FillFn>(STOCK_FILL)(stock, data, rd(shop + f::T_TAB), rd(shop + f::T_SCROLL), 0);
                }
            }
        } else {
            note(|| format!("card trade strip table holds {have:02x?}: left alone"));
        }
    }
    let at = base() + ALL_FINGER_AT;
    let want = if kh1.is_some() { ALL_FINGER_OURS } else { ALL_FINGER_RECOM };
    let have: [u8; 8] = rd(at);
    if have != want {
        if have == ALL_FINGER_RECOM || have == ALL_FINGER_OURS {
            super::hook::patch(at, &have, &want, "card trade ALL finger");
        } else {
            note(|| format!("card trade ALL finger code holds {have:02x?}: left alone"));
        }
    }
}

/// Our card list: Sora's items as Re:CoM's item cards (category 2, values unknown: no value
/// counts), the per-category counts Re:CoM's own (the window's "cards in" numbers unchanged).
unsafe fn card_list(shop: usize, items: &[(u8, u32)], head: &mut [u8; DATA_HEAD], records: &mut [u8; RECORDS * RECORD]) {
    head.fill(0);
    records.fill(0);
    let real: usize = rd(shop + f::CARD_DATA);
    if real != 0 {
        let same: [u8; DATA_COPY] = rd(real + DATA_PER_CATEGORY);
        head[DATA_PER_CATEGORY..DATA_PER_CATEGORY + DATA_COPY].copy_from_slice(&same);
    }
    let mut n = 0u32;
    for &(item, count) in items.iter().take(sh::sell::CELLS as usize) {
        let Some(kind) = sh::card_kind(item) else { continue };
        let r = &mut records[n as usize * RECORD..(n as usize + 1) * RECORD];
        r[0..2].copy_from_slice(&(kind as u16).to_le_bytes());
        r[2..4].copy_from_slice(&(count.clamp(1, 0xFFFF) as u16).to_le_bytes());
        r[4] = ITEM_CATEGORY;
        n += 1;
    }
    let ptr = records.as_ptr() as usize;
    head[DATA_RECORDS..DATA_RECORDS + 8].copy_from_slice(&ptr.to_le_bytes());
    let all = DATA_PER_TAB + 4 * sh::sell::ALL as usize;
    head[all..all + 4].copy_from_slice(&n.to_le_bytes());
}

/// Sora's items to sell now, with the card list rebuilt.
unsafe fn refresh_list(shop: usize, t: &mut SellTab) {
    let inv = super::items::inventory().unwrap_or_default();
    t.items = sh::sell_list(&inv).into_iter().filter(|s| sh::card_kind(s.0).is_some()).take(sh::sell::CELLS as usize).collect();
    card_list(shop, &t.items, &mut t.head, &mut t.records);
}

/// Re:CoM's stock window from our list (its own fill, as ALL: every record), then the art's
/// Items-lit strip on its strip control.
unsafe fn fill(shop: usize, head: usize) {
    let stock: usize = rd(shop + f::STOCK);
    if stock == 0 {
        return;
    }
    func::<FillFn>(STOCK_FILL)(stock, head, sh::sell::ALL, 0, 0);
    if let Some(first) = art() {
        let ctrl: usize = rd(stock + STOCK_STRIP);
        let lad: usize = rd(base() + SHOP_LAD);
        if ctrl != 0 && lad != 0 {
            let play: PlayFn = std::mem::transmute::<usize, PlayFn>(rd::<usize>(rd::<usize>(ctrl) + 0x18));
            play(ctrl, 0, lad, 1, (first + sh::tabs::STRIP_ITEMS) as i32, 0.0, 0);
        }
    }
}

/// The 0–9 box and the big card for the item at `cell` (None: the item category's blank box, no
/// card), as Re:CoM's `0x14045D9C0` / `0x14045A260` show a card.
unsafe fn show_card(shop: usize, records: usize, cell: Option<(i32, u8)>) {
    let rank: usize = rd(shop + f::RANK);
    let info: usize = rd(shop + f::INFO);
    match cell {
        Some((c, kind)) => {
            if rank != 0 {
                func::<RankFn>(RANK_SET)(rank, records + c as usize * RECORD, ITEM_CATEGORY as i32);
            }
            if info != 0 {
                func::<InfoFn>(INFO_SET)(info, kind as u64);
            }
        }
        None => {
            if rank != 0 {
                func::<RankFn>(RANK_SET)(rank, 0, ITEM_CATEGORY as i32);
            }
            if info != 0 {
                func::<InfoFn>(INFO_SET)(info, 0);
            }
        }
    }
}

unsafe fn cell_card(t: &SellTab, c: i32) -> Option<(i32, u8)> {
    let (item, _) = *t.items.get(usize::try_from(c).ok()?)?;
    Some((c, sh::card_kind(item)?))
}

unsafe fn charge(shop: usize, v: i32) {
    let w: usize = rd(shop + f::CHARGE);
    if w != 0 {
        func::<PackIntFn>(CHARGE_SET)(w, v);
    }
}

/// The help bar's line: ours on the Items tab, Re:CoM's (id 5) back.
unsafe fn help_bar(shop: usize, ours: bool) {
    let bar: usize = rd(shop + f::HELP_BAR);
    if bar == 0 {
        return;
    }
    if ours {
        let obj: usize = rd(bar + BAR_TEXT);
        if obj != 0 {
            // Re:CoM's text set copies the line.
            let line = sh::c_text(sh::sell::HELP);
            func::<SetTextFn>(TEXT_SET)(obj, line.as_ptr());
        }
    } else {
        func::<LookFn>(HELP_BAR)(bar, BAR_TRADE);
    }
}

unsafe fn look(cursor: usize, l: i32) {
    if cursor != 0 {
        func::<LookFn>(CURSOR_LOOK)(cursor, l);
    }
}

unsafe fn snap(cursor: usize) {
    if cursor != 0 {
        wr::<i32>(cursor + SNAP, 1);
    }
}

unsafe fn show(cursor: usize, on: bool) {
    if cursor != 0 {
        func::<VisibleFn>(CURSOR_VISIBLE)(cursor, on as u32);
    }
}

/// The tab row's finger on the Items tab (Re:CoM's state 1 tail): with the art at its cell (Re:CoM's
/// ALL place, now at the art's ALL, less 20); without, one tab step right of tab 3 (**Assumption**:
/// no art to place it by).
unsafe fn tab_finger(shop: usize, items: bool, tab: i32) {
    let stock: usize = rd(shop + f::STOCK);
    let finger: usize = rd(shop + f::CURSOR);
    if stock == 0 || finger == 0 {
        return;
    }
    let mut pos = V4([0.0, 0.0, 0.0, 1.0]);
    let place = func::<FingerFn>(TAB_FINGER);
    if items {
        match art() {
            Some(_) => {
                place(stock, pos.0.as_mut_ptr(), sh::sell::ALL);
                pos.0[0] += (sh::tabs::ITEMS_FINGER - sh::tabs::ALL_FINGER) as f32;
            }
            None => {
                place(stock, pos.0.as_mut_ptr(), sh::sell::ALL - 1);
                pos.0[0] += sh::tabs::FINGER_STEP;
            }
        }
    } else {
        place(stock, pos.0.as_mut_ptr(), tab);
    }
    func::<MoveFn>(CURSOR_MOVE)(finger, pos.0.as_ptr(), 1);
    look(finger, LOOK_FINGER);
    show(finger, true);
}

/// The grid finger at `cell` (Re:CoM's state 1 grid tail), in look `l`.
unsafe fn grid_finger(shop: usize, cell: i32, l: i32) {
    let stock: usize = rd(shop + f::STOCK);
    let finger: usize = rd(shop + f::GRID_FINGER);
    if stock == 0 || finger == 0 {
        return;
    }
    let rank: usize = rd(shop + f::RANK);
    if rank != 0 {
        func::<ObjFn>(RANK_SPIN)(rank);
    }
    let mut pos = V4([0.0, 0.0, 0.0, 1.0]);
    func::<FingerFn>(CELL_FINGER)(stock, pos.0.as_mut_ptr(), cell);
    func::<MoveFn>(CURSOR_MOVE)(finger, pos.0.as_ptr(), 0);
    self::look(finger, l);
    show(finger, true);
}

/// Onto the Items tab: our list, Re:CoM's windows showing it, the help bar's line. `open`: the
/// screen's start (Re:CoM's init put it in the grid when ALL has cards: ours in the grid when
/// Sora has items).
unsafe fn enter_sell(shop: usize, open: bool) {
    let mut t = SellTab {
        shop,
        items: Vec::new(),
        in_grid: false,
        cell: 0,
        qty: 1,
        head: Box::new([0; DATA_HEAD]),
        records: Box::new([0; RECORDS * RECORD]),
        line: String::new(),
        shown: Vec::new(),
        old: Vec::new(),
        asked: false,
    };
    refresh_list(shop, &mut t);
    t.in_grid = open && !t.items.is_empty();
    let (head, records) = (t.head.as_ptr() as usize, t.records.as_ptr() as usize);
    let card = if t.in_grid { cell_card(&t, 0) } else { None };
    let names: Vec<String> = t.items.iter().map(|&(i, n)| format!("{} x{n}", crate::items::name(i))).collect();
    let in_grid = t.in_grid;
    *sell_lock() = Some(t);
    fill(shop, head);
    show_card(shop, records, card);
    charge(shop, -1);
    help_bar(shop, true);
    note(|| format!("card trade: Items tab ({}), {names:?}, in the grid {in_grid}; {} Moogle Points", if open { "opened on" } else { "from the tab row" }, points()));
}

/// Off the Items tab to Re:CoM's tab `to` (Re:CoM's tab move: its stock window, its blank box, its
/// finger), Re:CoM's help bar line back.
unsafe fn leave_sell(shop: usize, to: i32) {
    if sell_lock().take_if(|t| t.shop == shop).is_none() {
        return;
    }
    wr::<i32>(shop + f::T_TAB, to);
    wr::<i32>(shop + f::T_IN_GRID, 0);
    let stock: usize = rd(shop + f::STOCK);
    let data: usize = rd(shop + f::CARD_DATA);
    if stock != 0 && data != 0 {
        func::<FillFn>(STOCK_FILL)(stock, data, to, 0, 0);
    }
    let rank: usize = rd(shop + f::RANK);
    if rank != 0 {
        func::<RankFn>(RANK_SET)(rank, 0, to);
    }
    help_bar(shop, false);
    tab_finger(shop, false, to);
    note(|| format!("card trade: Items tab -> Re:CoM's tab {to}"));
}

unsafe fn read_keys() -> sh::sell::Keys {
    let fresh = |mask: u32| func::<PadMaskFn>(PAD_FRESH)(mask, 0) != 0;
    sh::sell::Keys { pad: read_pad(), up_fresh: fresh(0x0010_1000), down_fresh: fresh(0x0020_4000), left_fresh: fresh(0x0040_8000), right_fresh: fresh(0x0080_2000) }
}

/// Re:CoM's lock step at the update's head (`+0x274`: −1 while > 0); locked after it.
unsafe fn lock_step(shop: usize) -> bool {
    let l: i32 = rd(shop + f::LOCK);
    if l > 0 {
        wr::<i32>(shop + f::LOCK, l - 1);
    }
    l - 1 > 0
}

unsafe fn leave_trade(shop: usize, sound_id: u32, mode: i32) {
    sound(sound_id);
    wr::<i32>(shop + f::NEXT_MODE, mode);
    wr::<i32>(shop + f::STATE, STATE_CLOSE);
}

/// One frame of the Items tab in Re:CoM's state 1 (the tab row or the grid).
unsafe fn sell_frame(shop: usize) {
    use sh::sell::Step;
    let locked = lock_step(shop);
    let keys = if locked { sh::sell::Keys::default() } else { read_keys() };
    let (in_grid, cell, n) = match sell_lock().as_ref().filter(|t| t.shop == shop) {
        Some(t) => (t.in_grid, t.cell, t.items.len() as i32),
        None => return,
    };
    let step = if in_grid { sh::sell::grid(&keys, n, cell, locked) } else { sh::sell::tab_row(&keys, n, locked) };
    if let Step::Leave { sound: s, mode } = step {
        return leave_trade(shop, s, mode);
    }
    charge(shop, -1);
    let records = |t: &SellTab| t.records.as_ptr() as usize;
    match step {
        Step::ToTab(to) => {
            sound(sh::sound::MOVE);
            return leave_sell(shop, to);
        }
        Step::Refused => sound(sh::sound::ERROR),
        Step::IntoGrid(c) => {
            sound(sh::sell::CARD_MOVE);
            let (r, card) = {
                let mut g = sell_lock();
                let Some(t) = g.as_mut().filter(|t| t.shop == shop) else { return };
                t.in_grid = true;
                t.cell = c;
                (records(t), cell_card(t, c))
            };
            show_card(shop, r, card);
            show(rd(shop + f::CURSOR), false);
            snap(rd(shop + f::GRID_FINGER));
            wr::<i32>(shop + f::LOCK, 2);
            return;
        }
        Step::OutOfGrid => {
            sound(sh::sound::MOVE);
            let r = {
                let mut g = sell_lock();
                let Some(t) = g.as_mut().filter(|t| t.shop == shop) else { return };
                t.in_grid = false;
                records(t)
            };
            show_card(shop, r, None);
            show(rd(shop + f::GRID_FINGER), false);
            snap(rd(shop + f::CURSOR));
            wr::<i32>(shop + f::LOCK, 2);
            return;
        }
        Step::Move(c) => {
            sound(sh::sell::CARD_MOVE);
            let (r, card) = {
                let mut g = sell_lock();
                let Some(t) = g.as_mut().filter(|t| t.shop == shop) else { return };
                t.cell = c;
                (records(t), cell_card(t, c))
            };
            show_card(shop, r, card);
        }
        Step::Confirm(c) => {
            sound(sh::sound::BUY);
            let price = {
                let mut g = sell_lock();
                let Some(t) = g.as_mut().filter(|t| t.shop == shop) else { return };
                t.qty = 1;
                t.items.get(c.max(0) as usize).map(|&(i, _)| sh::sell::total(i, 1)).unwrap_or(0)
            };
            look(rd(shop + f::GRID_FINGER), LOOK_CHOSEN);
            snap(rd(shop + f::VALUE_FINGER));
            wr::<i32>(shop + f::LOCK, 2);
            wr::<i32>(shop + f::STATE, TRADE_GRID);
            charge(shop, price);
            return;
        }
        _ => {}
    }
    let (in_grid, cell) = match sell_lock().as_ref().filter(|t| t.shop == shop) {
        Some(t) => (t.in_grid, t.cell),
        None => return,
    };
    if in_grid {
        grid_finger(shop, cell, LOOK_GRID);
    } else {
        tab_finger(shop, true, 0);
    }
}

/// One frame of the count step (in Re:CoM's state 2).
unsafe fn count_frame(shop: usize) {
    use sh::sell::Count;
    let locked = lock_step(shop);
    let keys = if locked { sh::sell::Keys::default() } else { read_keys() };
    let Some((item, owned, qty)) = sell_lock().as_ref().filter(|t| t.shop == shop).and_then(|t| t.items.get(t.cell.max(0) as usize).map(|&(i, n)| (i, n, t.qty))) else {
        return;
    };
    match sh::sell::count(&keys, qty, owned, locked) {
        Count::Back => {
            sound(sh::sound::BACK);
            show(rd(shop + f::VALUE_FINGER), false);
            wr::<i32>(shop + f::STATE, TRADE_TABS);
            wr::<i32>(shop + f::LOCK, 2);
            return;
        }
        Count::Leave => return leave_trade(shop, sh::sound::OTHER, sh::mode::PACKS),
        Count::Ask => {
            sound(sh::sound::BUY);
            if let Some(t) = sell_lock().as_mut().filter(|t| t.shop == shop) {
                t.line = sh::sell::ask_text(item, qty, sh::sell::total(item, qty));
                t.asked = false;
            }
            wr::<i32>(shop + f::YES_NO, 0);
            let tw: usize = rd(shop + f::TEXT);
            if tw != 0 {
                func::<LookFn>(TEXT_MESSAGE)(tw, MESSAGE_YES_NO);
            }
            look(rd(shop + f::VALUE_FINGER), LOOK_YES_NO);
            let y: f32 = rd(base() + YES_NO_Y);
            let finger: usize = rd(shop + f::CURSOR);
            if finger != 0 {
                func::<CursorPlaceFn>(CURSOR_PLACE)(finger, YES_NO_X, y, 0);
            }
            show(finger, false);
            wr::<i32>(shop + f::STATE, TRADE_ASK);
            note(|| format!("card trade: sell {} x{qty} for {}? ({} Moogle Points)", crate::items::name(item), sh::sell::total(item, qty), points()));
            return;
        }
        Count::Set(q) => {
            sound(sh::sell::CARD_MOVE);
            if let Some(t) = sell_lock().as_mut().filter(|t| t.shop == shop) {
                t.qty = q;
            }
        }
        Count::Nothing => {}
    }
    let qty = sell_lock().as_ref().filter(|t| t.shop == shop).map(|t| t.qty).unwrap_or(1);
    charge(shop, sh::sell::total(item, qty));
}

/// Our line on Re:CoM's yes / no, once its text window is up (Re:CoM's state 3 runs as Re:CoM's).
unsafe fn ask_line(shop: usize) {
    let mut g = sell_lock();
    let Some(t) = g.as_mut().filter(|t| t.shop == shop && !t.asked && !t.line.is_empty()) else { return };
    let tw: usize = rd(shop + f::TEXT);
    if tw == 0 || rd::<i32>(tw + TW_STATE) != 1 {
        return;
    }
    let main: usize = rd(tw + TW_MAIN);
    if main == 0 || rd::<i32>(main + TEXTWND_STATE) != SHOWN {
        return;
    }
    let shown = bubble_line(main, &t.line, "card trade, yes / no");
    func::<TextFn>(TEXTWND_TEXT)(main, shown.as_ptr(), 0);
    let old = std::mem::replace(&mut t.shown, shown);
    if !old.is_empty() {
        t.old.push(old);
    }
    while t.old.len() > 2 {
        t.old.remove(0);
    }
    t.asked = true;
}

/// A shown `CTextWnd`'s text (`+0xD0` its `CText`, `+0x38` its `CStrPrim`), or 0.
unsafe fn bubble_fnt(main: usize) -> usize {
    if main == 0 {
        return 0;
    }
    let ctext: usize = rd(main + tr::WND_TEXT);
    if ctext == 0 {
        return 0;
    }
    rd(ctext + tr::CTEXT_FNT)
}

/// The bubble last written to and the lines Re:CoM's own text had in it (its `CText +0x54`, read
/// before our first line there), and the box lines already logged.
static BUBBLE: Mutex<(usize, usize, Vec<(i32, i32, i32, i32, i32, i32)>)> = Mutex::new((0, 0, Vec::new()));

/// Our line `raw` (`\n` where we prefer a new line) laid out for the Moogle's bubble `main` (a
/// shown `CTextWnd`), as the bytes to hand to Re:CoM's text call: wrapped onto the bubble's lines
/// as Re:CoM's own two-line lines are, measured with Re:CoM's width routine on the bubble's own
/// text (`crate::text_fit::wrap`); squeezed (the true squeeze, set on the bubble's text before
/// the call) only when no wrap fits. The box: the bubble's width less its text margins on both
/// sides and 2 units each side (`text_fit::boxes::MARGIN`); its lines: as many as its height less
/// both margins holds at the text's size and gap, or the lines Re:CoM's own text there had if more.
/// Assumption: the margins are the same on both sides (the text is placed at the window + the
/// margin, `0x1402B1D50`, and centred or left-aligned in the window's width, `0x1402B0820`).
/// Without Re:CoM's font (no width): `raw` as it is.
unsafe fn bubble_line(main: usize, raw: &str, what: &str) -> Vec<u8> {
    // 0.163.0: Riku's own name in his story's lines.
    let riku_text;
    let raw = if super::stats::riku_story() {
        riku_text = raw.replace("Sora", "Riku");
        riku_text.as_str()
    } else {
        raw
    };
    let fnt = bubble_fnt(main);
    if fnt == 0 {
        return sh::c_text(raw);
    }
    let (w, h): (i32, i32) = (rd(main + tr::WND_SIZE), rd(main + tr::WND_SIZE + 4));
    let (mx, my): (i32, i32) = (rd(main + tr::WND_MARGINS), rd(main + tr::WND_MARGINS + 4));
    let (size16, gap16): (i32, i32) = (rd(fnt + tr::SIZE), rd(fnt + tr::GAP));
    let box16 = (((w - 2 * mx) as f32 - 2.0 * crate::text_fit::boxes::MARGIN) * 16.0) as i64;
    let geometry = crate::text_fit::wrap::lines_in(((h - 2 * my) as i64) * 16, size16 as i64, gap16 as i64);
    let (recom_lines, first_box) = {
        let mut b = BUBBLE.lock().unwrap_or_else(|e| e.into_inner());
        if b.0 != main {
            let ctext: usize = rd(main + tr::WND_TEXT);
            b.0 = main;
            b.1 = rd::<i32>(ctext + tr::CTEXT_LINES).clamp(0, 8) as usize;
        }
        let key = (w, h, mx, my, size16, gap16);
        let first = !b.2.contains(&key) && b.2.len() < 32;
        if first {
            b.2.push(key);
        }
        (b.1, first)
    };
    let lines = geometry.max(recom_lines).max(1);
    if first_box {
        note(|| format!("bubble ({what}): {w} x {h}, text margins {mx} / {my}, font {} / gap {}: box {:.0} units, {lines} lines (by its height {geometry}, Re:CoM's own text there {recom_lines})", size16 / 16, gap16 / 16, box16 as f32 / 16.0));
    }
    if box16 <= 0 {
        return sh::c_text(raw);
    }
    let laid = crate::text_fit::wrap::layout(raw, box16, lines, &mut |s| super::text_fit::width_of(fnt, s.as_bytes()).map(i64::from));
    let Some(laid) = laid else {
        super::text_fit::unsqueeze(fnt);
        return sh::c_text(raw);
    };
    let bytes = sh::c_text(&laid.text);
    super::text_fit::set_level(fnt, laid.level, &bytes[..bytes.len() - 1], laid.widest, box16, || format!("Moogle's bubble ({what})"));
    note(|| format!("bubble ({what}): {:?} -> {:?}, {} line(s), widest {:.1} units of {:.0}{}", raw, laid.text, laid.lines, laid.widest as f32 / 16.0, box16 as f32 / 16.0, if laid.level < crate::text_fit::ONE { format!(", squeezed x {:.3}", crate::text_fit::factor(laid.level)) } else { String::new() }));
    bytes
}

/// The sale (in Re:CoM's state 5, once the yes / no has closed): the items from the stock first,
/// then the slots (`crate::shop::sell`), the points through Re:CoM's own add (`0x140239FA0`: its
/// cap and its trophy check) and its Moogle Point window; then the grid (or the tab row with
/// nothing left), as Re:CoM goes back after a card sale.
unsafe fn sale_frame(shop: usize) {
    let tw: usize = rd(shop + f::TEXT);
    if tw != 0 && rd::<i32>(tw + TW_STATE) != TW_CLOSED {
        return;
    }
    wr::<i32>(shop + f::LOCK, 2);
    let Some((item, qty, cell)) = sell_lock().as_ref().filter(|t| t.shop == shop).and_then(|t| t.items.get(t.cell.max(0) as usize).map(|&(i, n)| (i, t.qty.clamp(1, n), t.cell))) else {
        return;
    };
    let before = points();
    let (sold, total) = super::items::edit(|inv| {
        let (mut sold, mut total) = (0u32, 0i32);
        for _ in 0..qty {
            match sh::sell(item, 0, inv) {
                sh::Sell::Sold { price, .. } => {
                    sold += 1;
                    total += price;
                }
                sh::Sell::None => break,
            }
        }
        (sold, total)
    })
    .unwrap_or((0, 0));
    if total > 0 {
        func::<PointsSetFn>(POINTS_ADD)(total);
    }
    let w: usize = rd(shop + f::POINT_WINDOW);
    if w != 0 {
        func::<PackIntFn>(POINT_WINDOW)(w, points());
    }
    let (head, records, card, in_grid) = {
        let mut g = sell_lock();
        let Some(t) = g.as_mut().filter(|t| t.shop == shop) else { return };
        refresh_list(shop, t);
        t.qty = 1;
        match sh::sell::after_sale(cell, t.items.len() as i32) {
            Some(c) => {
                t.in_grid = true;
                t.cell = c;
            }
            None => {
                t.in_grid = false;
                t.cell = 0;
            }
        }
        (t.head.as_ptr() as usize, t.records.as_ptr() as usize, if t.in_grid { cell_card(t, t.cell) } else { None }, t.in_grid)
    };
    fill(shop, head);
    show_card(shop, records, card);
    show(rd(shop + f::VALUE_FINGER), false);
    if in_grid {
        snap(rd(shop + f::GRID_FINGER));
    } else {
        snap(rd(shop + f::CURSOR));
        show(rd(shop + f::GRID_FINGER), false);
    }
    wr::<i32>(shop + f::STATE, TRADE_TABS);
    note(|| format!("card trade: sold {} x{sold} of {qty} for {total}: {before} -> {} Moogle Points", crate::items::name(item), points()));
}

/// The card trade's Square to the pack screen without room for 5 cards, at Combat = KH1
/// (`crate::shop::trade_square_opens_packs`; Re:CoM's own with room). Re:CoM's reads before the
/// Square (`0x1403A752A`): its lock decrement runs at the function's head, so the lock is read as
/// Re:CoM's Square test reads it (≤ 0 after). True when it took the frame.
unsafe fn trade_square(shop: usize) -> bool {
    let lock_now = rd::<i32>(shop + f::LOCK);
    let cancel_first = lock_now < 2 && func::<PadBtnFn>(PAD_CANCEL)(0) != 0;
    if !cancel_first && lock_now < 2 && func::<PadMaskFn>(PAD_FRESH)(0x80, 0) != 0 {
        if lock_now > 0 {
            wr::<i32>(shop + f::LOCK, lock_now - 1);
        }
        leave_trade(shop, sh::sound::OTHER, sh::mode::PACKS);
        note(|| "card trade: Square to the pack screen without room for 5 cards (its Items tab)".into());
        return true;
    }
    false
}

/// The wrapper on the card trade's update (`0x1403A73E0`), at Combat = KH1: its Square opens the
/// pack screen also without room for 5 cards (`crate::shop::trade_square_opens_packs`), and its
/// Items tab (`crate::shop::sell`).
///
/// # Safety
/// Called by Re:CoM with its `CMoogleShopCore`.
pub unsafe extern "win64" fn trade(shop: usize) {
    let run = orig::<ScreenFn>(&ORIG_TRADE);
    let state: i32 = rd(shop + f::STATE);
    let kh1 = live();
    let up = sell_up(shop);
    if state == STATE_FIRST {
        if !up {
            trade_tables(shop, art().filter(|_| kh1));
        }
        if !kh1 {
            return run(shop);
        }
        if !room(shop) {
            trade_button(shop);
        }
        run(shop);
        if !up {
            enter_sell(shop, true);
        }
        return;
    }
    if !kh1 {
        if up {
            *sell_lock() = None;
        }
        return run(shop);
    }
    if up {
        match state {
            TRADE_TABS => sell_frame(shop),
            TRADE_GRID => count_frame(shop),
            TRADE_ASK => {
                ask_line(shop);
                run(shop)
            }
            TRADE_SALE => sale_frame(shop),
            STATE_CLOSE => {
                *sell_lock() = None;
                note(|| "card trade left from its Items tab".into());
                run(shop)
            }
            _ => run(shop),
        }
        return;
    }
    if (state == TRADE_TABS || state == TRADE_GRID) && !room(shop) && sh::trade_square_opens_packs(kh1, false) && trade_square(shop) {
        return;
    }
    if state == TRADE_TABS {
        let lock_now = rd::<i32>(shop + f::LOCK);
        let tab: i32 = rd(shop + f::T_TAB);
        let in_grid = rd::<i32>(shop + f::T_IN_GRID) != 0;
        if lock_now < 2 && !in_grid && sh::sell::enters(tab, in_grid, &read_pad(), false) {
            if lock_now > 0 {
                wr::<i32>(shop + f::LOCK, lock_now - 1);
            }
            sound(sh::sound::MOVE);
            enter_sell(shop, false);
            tab_finger(shop, true, 0);
            return;
        }
    }
    run(shop)
}

pub fn start_line() -> String {
    "shop: KH1's items in Re:CoM's own Moogle shop screens (M8d): at Combat = KH1 in Sora's story the pack screen opens on an Items tab drawn before Re:CoM's four (Re:CoM's packs with Re:CoM's item cards and price plates; 1F Potion 10 / Ether 40, 2F+ Hi-Potion 30, 7F+ Elixir 200 Moogle Points; KH1's pickup); the card trade opens on an Items tab first in its strip (Sora's items as Re:CoM's item cards, a count step, Re:CoM's yes / no; sold stock first, then slots, at the buy price / 5); its Square opens the pack screen also without room for 5 cards".into()
}
