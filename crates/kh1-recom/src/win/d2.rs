//! Our HUD controls' 2D instances in Re:CoM's 2D manager (`crate::d2`, which has the decomp
//! notes): every layer we add or release goes through here, on the draw list its gauge was made
//! on, and is counted for the log.
//!
//! A release finds its handle only in Re:CoM's current draw list, and a new layer goes into the
//! current list. Our objects belong to the player gauge's list (read at the gauge's init). When
//! another list is current (the KH1 entry's pages run inside the camp, list 1),
//! the call runs with Re:CoM's own list setter `0x140249800` on our list and sets the current one
//! back right after (nothing else runs between: one game thread). Without this, a Config flip in
//! the camp made or released the MP bar and the menu's layers on the wrong list: each release
//! there left its pool entry taken and its `C2DSeqObj` in place (pointing at a freed control).

use super::movement::{func, rd};
use super::{base, log};
use crate::d2 as d;
use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};

// Re:CoM functions (RVAs; first bytes checked in `sites`).
const SET_LIST: u32 = 0x0024_9800;
const SEQ_DELETE: u32 = 0x0015_D230;
const CLEAR: u32 = 0x0024_7C90;
const ADD: u32 = 0x0024_7850;
// Re:CoM data (RVAs), Checked (exe: the rip operands of `0x140249800`, `0x140245710`,
// `0x140245A20`).
const MANAGER: usize = 0x0087_CE38;
const POOL: usize = 0x0087_CE58;
const LIST_INDEX: usize = 0x0082_0F94;
/// `D2::C2DOSLCManager`: its pool at `+8`, the pool's size `+0x14` and free count `+0x24`.
const POOL_SIZE: usize = 8 + 0x14;
const POOL_FREE: usize = 8 + 0x24;
/// `C2DSeqCtrl`: its layer count.
const C_LAYERS: usize = 0x16D;

type SetListFn = unsafe extern "win64" fn(u32);
type DeleteFn = unsafe extern "win64" fn(usize, u32) -> usize;
type ClearFn = unsafe extern "win64" fn(usize);
type AddFn = unsafe extern "win64" fn(usize, *const u8, u32, f32, u32) -> usize;
/// `C2DSeqCtrl` vtable `+0x60` (`0x1402476F0`): (ctrl, set, item, x, y, screen x offset).
pub type AttachFn = unsafe extern "win64" fn(usize, *const u8, usize, f32, f32, f32) -> usize;

/// Layers (= pool entries) and controls we hold now.
static LAYERS: AtomicI32 = AtomicI32::new(0);
static CTRLS: AtomicI32 = AtomicI32::new(0);
/// Census calls so far, and lines left (the first 40 calls, then every 10th, 120 lines at most).
static CENSUS_CALLS: AtomicU32 = AtomicU32::new(0);
static LOG_LEFT: AtomicU32 = AtomicU32::new(120);
static SWITCH_LOGS: AtomicU32 = AtomicU32::new(8);

/// Re:CoM's current draw list (-1 before the 2D manager made its lists).
pub fn current_list() -> i32 {
    // SAFETY: statics of the image.
    unsafe {
        if rd::<u64>(base() + MANAGER) == 0 {
            return -1;
        }
        rd(base() + LIST_INDEX)
    }
}

/// Runs `f` (our layer adds or releases) with `list` current, as it was when our objects were made.
unsafe fn on_list<R>(list: i32, what: &str, f: impl FnOnce() -> R) -> R {
    let Some((want, back)) = d::switch(current_list(), list) else { return f() };
    if SWITCH_LOGS.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("2d: {what} on draw list {want} while list {back} is current (the camp?): run on list {want}, then {back} again"));
    }
    let set = func::<SetListFn>(SET_LIST);
    set(want);
    let r = f();
    set(back);
    r
}

pub(super) unsafe fn layers(c: usize) -> i32 {
    rd::<u8>(c + C_LAYERS) as i32
}

/// A control we made (Re:CoM's `operator new` and ctor).
pub fn made() {
    CTRLS.fetch_add(1, Ordering::Relaxed);
}

/// Adds group `group` of `set` as a layer (`C2DSeqCtrl` `+8`).
pub unsafe fn add(c: usize, list: i32, set: *const u8, group: u32, xoff: f32, flag: u32) {
    let n = layers(c);
    on_list(list, "a layer", || func::<AddFn>(ADD)(c, set, group, xoff, flag));
    LAYERS.fetch_add(layers(c) - n, Ordering::Relaxed);
}

/// `C2DSeqCtrl` vtable `+0x20` (`0x1402482F0`): replace layer `layer` with group `group` of `set`
/// (ctrl, layer, set, group, screen x offset, flag): a new instance takes the old one's place in
/// the draw list and its visibility (`0x140247000` → `0x140249EB0`, the old object's `+0x45`), the
/// old one's entry goes back to the pool. Checked (decomp). The frame is not carried over.
type ReplaceFn = unsafe extern "win64" fn(usize, u32, *const u8, u32, f32, u32) -> usize;

/// Replaces layer `layer` of `c` (Re:CoM's control or ours; called through its own vtable, as
/// Re:CoM's HUD code does). False when Re:CoM made no instance (no such set or group).
pub unsafe fn replace(c: usize, list: i32, layer: u32, set: *const u8, group: u32, xoff: f32, flag: u32) -> bool {
    let f: ReplaceFn = std::mem::transmute_copy(&rd::<usize>(rd::<usize>(c) + 0x20));
    on_list(list, "a layer replace", || f(c, layer, set, group, xoff, flag)) != 0
}

/// Attaches the text item `fnt` (`C2DSeqCtrl` `+0x60`); the layer then owns the font.
pub unsafe fn attach(c: usize, list: i32, set: *const u8, fnt: usize, x: f32, y: f32, dx: f32) -> usize {
    let n = layers(c);
    let attach: AttachFn = std::mem::transmute_copy(&rd::<usize>(rd::<usize>(c) + 0x60));
    let r = on_list(list, "a text layer", || attach(c, set, fnt, x, y, dx));
    LAYERS.fetch_add(layers(c) - n, Ordering::Relaxed);
    r
}

/// Releases every layer of `c` (`0x140247C90`); the control stays ours.
pub unsafe fn clear(c: usize, list: i32) {
    if c == 0 {
        return;
    }
    let n = layers(c);
    on_list(list, "a clear", || func::<ClearFn>(CLEAR)(c));
    LAYERS.fetch_sub(n, Ordering::Relaxed);
}

/// Deletes `c` (its deleting dtor, flag 1): its layers' entries, objects and attached fonts go.
pub unsafe fn delete(c: usize, list: i32) {
    if c == 0 {
        return;
    }
    let n = layers(c);
    on_list(list, "a delete", || func::<DeleteFn>(SEQ_DELETE)(c, 1));
    LAYERS.fetch_sub(n, Ordering::Relaxed);
    CTRLS.fetch_sub(1, Ordering::Relaxed);
}

/// Re:CoM's pool: (free entries, size), None without a 2D manager.
pub fn pool() -> Option<(i32, i32)> {
    // SAFETY: statics of the image, then the pool object they point at.
    unsafe {
        if rd::<u64>(base() + MANAGER) == 0 {
            return None;
        }
        let p: usize = rd(base() + POOL);
        (p != 0).then(|| (rd::<i32>(p + POOL_FREE), rd::<i32>(p + POOL_SIZE)))
    }
}

/// Our count and Re:CoM's pool, as text (the battle-end line, the census).
pub fn census_text() -> String {
    d::census_line(LAYERS.load(Ordering::Relaxed), CTRLS.load(Ordering::Relaxed), pool(), current_list())
}

/// Logs the count after `why` (the first 40 times, then every 10th; 120 lines in all): it stays
/// flat across rooms, battles and menu use when nothing leaks.
pub fn census(why: impl FnOnce() -> String) {
    let n = CENSUS_CALLS.fetch_add(1, Ordering::Relaxed);
    if n >= 40 && n % 10 != 0 {
        return;
    }
    if LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("2d: {}: {}", why(), census_text()));
    }
}
