//! Re:CoM's Config screen code, run for the Settings list's settings pages (Combat, Movement;
//! owner, 2026-11-03 / 2026-11-04).
//!
//! - `cfg_input` wraps the Config screen's input/state function `0x1400D3EB0`. The camp runs it in
//!   its states 0x4A..0x4C; in Sora's story Settings opens our short list (Game Settings, Combat,
//!   Movement), and the KH1 list's Items / Abilities / Customize pages run there too
//!   (`keyblades::page`). Re:CoM's own Config screen (the Game Settings row, and Settings in Riku's
//!   story) always goes to the original, untouched: its five rows, layout, panel and input are
//!   exactly Re:CoM's.
//! - [`SettingsPage`]: a settings page is Re:CoM's Config screen run with our rows. Re:CoM's own
//!   Config draw `0x1400DF1B0` (called, not hooked) makes the window panel and the cursor sprite
//!   (mode 0), opens them (mode 1), glides the cursor (mode 3), closes (modes 4 / 5) and deletes
//!   everything (mode 6); our rows replace its mode 9 only: the same `CButton` items built by the
//!   same calls with the same arguments, at Re:CoM's rows 0..4 positions in `camp.lad`, in the same
//!   21 item slots its modes 6 / 9 and the camp teardown delete. Input is Re:CoM's Config input
//!   (its pad reads, its order, its sound 1 / 4). Nothing of ours runs outside the camp menu.
//!
//! The Config fields used (`camp+0xB30` panel, `+0xB38` slots, `+0xBE0` cursor sprite) are
//! free while one of our settings pages is open: Re:CoM's Config screen runs only from its own
//! top entry 6 (Riku's story) or the Settings list's Game Settings row, never at the same time.
//! Our page keeps its cursor itself; Re:CoM's cursor `+0xBE8` and value bytes `+0xBED..0xBF1` are
//! never touched.
//!
//! Offsets and calls: `docs/recom-config-menu.md`. Checked (decomp + disasm) unless noted.

use super::{base, log, va};
use crate::menu::{self, layout, Input, Item, Page, Step, Style};
use crate::sites;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub static ORIG_INPUT: AtomicUsize = AtomicUsize::new(0);
static LAYOUT_WARNED: AtomicBool = AtomicBool::new(false);

// CCamp fields used by the Config screen.
const NO_SCALE: usize = 0x9C;
const LAYOUT: usize = 0x78;
const SET: usize = 0x80;
const HELP: usize = 0x9D0;
const PANEL: usize = 0xB30;
const CURSOR_SPRITE: usize = 0xBE0;

// Pad masks the original uses.
const UP: u32 = 0x101000;
const DOWN: u32 = 0x204000;
const LEFT: u32 = 0x408000;
const RIGHT: u32 = 0x802000;

type InputFn = unsafe extern "win64" fn(*mut u8) -> u64;
type DrawFn = unsafe extern "win64" fn(*mut u8, u32) -> u64;
type DirFn = unsafe extern "win64" fn(*mut u8, u32, u32) -> u32;
type ConfirmFn = unsafe extern "win64" fn(*mut u8, u32) -> u32;
type SoundFn = unsafe extern "win64" fn(i32, i32) -> u64;
type NewFn = unsafe extern "win64" fn(usize) -> *mut u8;
type CtorFn = unsafe extern "win64" fn(*mut u8) -> *mut u8;
type DeleteFn = unsafe extern "win64" fn(*mut u8, u32) -> *mut u8;
#[allow(clippy::type_complexity)]
type SetupFn = unsafe extern "win64" fn(
    *mut u8,     // CButton
    *const u8,   // sprite set name (camp+0x80)
    u32,         // kind (u8 0x10)
    *const u16,  // sequence ids
    *const f32,  // position
    *mut u8,     // parent sprite (camp+0xB30)
    *const u8,   // text
    i32,         // font (0xD; 0xC for labels in two languages)
    f32,         // 0.0
    f32,         // 2.0, or 0.0 when camp+0x9C != 0
    i32,         // 0x42
    *const u32,  // colour (null = default)
    i32,         // selected: -1 label, 0/1 value (a short)
);
type LayoutFn = unsafe extern "win64" fn(*const u8, u32, u64, *mut f32, i32) -> u64;
type SetTextFn = unsafe extern "win64" fn(*mut u8, *const u8);
type SetPosFn = unsafe extern "win64" fn(*mut u8, *const f32);
type AnimFn = unsafe extern "win64" fn(*mut u8, u32, *const u8, u32, f32, u32) -> u64;

const DIR_FN: u32 = 0x0011_1A70;
const CONFIRM_FN: u32 = 0x0011_1B90;
const SOUND_FN: u32 = 0x002A_A360;
const NEW_FN: u32 = 0x004A_CE54;
const CTOR_FN: u32 = 0x0011_2A90;
const SETUP_FN: u32 = 0x0011_2F10;
const LAYOUT_FN: u32 = 0x0024_6100;
const SET_TEXT_FN: u32 = 0x0025_9000;
const SET_POS_FN: u32 = 0x0024_85B0;
/// Re:CoM's Config draw (`sites::CALLEES`, called only).
const DRAW_FN: u32 = 0x000D_F1B0;

unsafe fn f<T: Copy>(rva: u32) -> T {
    let p = base() + rva as usize;
    std::mem::transmute_copy(&p)
}

unsafe fn rd<T: Copy>(p: *mut u8, off: usize) -> T {
    std::ptr::read_unaligned(p.add(off) as *const T)
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Default)]
struct Vec4([f32; 4]);

/// Hook on `0x1400D3EB0` (the Config screen's input/state function).
pub unsafe extern "win64" fn cfg_input(camp: *mut u8) -> u64 {
    // The camp's Config states 0x4A..0x4C run the Settings list (Settings confirmed in Sora's
    // story) or one of our KH1 list entries' pages (`keyblades`); Re:CoM's Config screen runs from
    // the list's Game Settings row (`recom_config`), and directly in Riku's story.
    if super::keyblades::page_active(camp) {
        return super::keyblades::page(camp);
    }
    recom_config(camp)
}

/// Re:CoM's own Config input function (the original, past our hook), as its states 0x4A / 0x4B
/// call it: `camp+0xA8` 0 opens it, then ≥ 0 each frame while it runs, < 0 when it has closed.
pub unsafe fn recom_config(camp: *mut u8) -> u64 {
    let orig: InputFn = std::mem::transmute(ORIG_INPUT.load(Ordering::Acquire));
    orig(camp)
}

unsafe fn draw(camp: *mut u8, mode: u32) -> u64 {
    f::<DrawFn>(DRAW_FN)(camp, mode)
}

unsafe fn sound(id: i32) {
    f::<SoundFn>(SOUND_FN)(id, -1);
}

/// A settings page open in the Settings list: Re:CoM's Config screen with our rows.
pub struct SettingsPage {
    camp: usize,
    page: Page,
    cursor: usize,
    /// 1 opening (Re:CoM's Config state 1), 2 input (state 2), 3 closing (state 3).
    state: u8,
}

impl SettingsPage {
    /// The camp it runs in.
    pub fn camp(&self) -> usize {
        self.camp
    }

    fn p(&self) -> *mut u8 {
        self.camp as *mut u8
    }

    /// Re:CoM's Config state 0: draw modes 0 (panel and cursor sprite) and our rows for its 9.
    pub unsafe fn open(camp: *mut u8, page: Page) -> SettingsPage {
        draw(camp, 0);
        let mut p = SettingsPage { camp: camp as usize, page, cursor: 0, state: 1 };
        p.rows();
        let s = super::settings();
        let now: Vec<String> = page.items().iter().map(|it| format!("{} = {}", it.key(), String::from_utf8_lossy(&it.options()[it.value(&s)][..it.options()[it.value(&s)].len() - 1]))).collect();
        log(&format!("kh1 entry: {page:?} page opened ({})", now.join(", ")));
        p
    }

    /// One frame (≥ 0 running, −1 closed and deleted).
    pub unsafe fn step(&mut self) -> i32 {
        let camp = self.p();
        match self.state {
            1 => {
                // State 1: mode 1 until the panel's opening ends (it also glides the cursor).
                if draw(camp, 1) as u32 != 0 {
                    self.state = 2;
                }
                0
            }
            2 => {
                // State 2: mode 3 (animate), then the pad in Re:CoM's order.
                draw(camp, 3);
                let dir: DirFn = f(DIR_FN);
                let close = f::<ConfirmFn>(CONFIRM_FN)(camp, 0) != 0;
                let i = if close {
                    Input { close, ..Default::default() }
                } else {
                    Input { close, up: dir(camp, UP, 0) != 0, down: dir(camp, DOWN, 0) != 0, left: dir(camp, LEFT, 0) != 0, right: dir(camp, RIGHT, 0) != 0 }
                };
                let items = self.page.items();
                match menu::step(self.cursor, items.len(), i) {
                    Step::Close => {
                        draw(camp, 4);
                        sound(4);
                        self.state = 3;
                        return 0;
                    }
                    Step::Move(c) => self.cursor = c,
                    Step::Change(right) => {
                        let it = items[self.cursor.min(items.len() - 1)];
                        let v = menu::cycle(it.value(&super::settings()), it.options().len(), right);
                        super::set_item(it, v);
                    }
                    Step::Idle => return 0,
                }
                sound(1);
                self.rows();
                0
            }
            _ => {
                // State 3: mode 5 until the close ends, then mode 6 deletes the panel, the items
                // and the cursor sprite.
                if draw(camp, 5) as u32 != 0 {
                    draw(camp, 6);
                    log(&format!("kh1 entry: {:?} page closed", self.page));
                    return -1;
                }
                0
            }
        }
    }

    /// Our mode 9: Re:CoM's 21 item slots emptied as its mode 9 empties them, then our rows built
    /// with its calls; the cursor sprite onto the cursor row's value and the row's help line.
    unsafe fn rows(&mut self) {
        let camp = self.p();
        let slots = camp.add(menu::SLOT_BASE) as *mut *mut u8;
        for k in 0..menu::SLOT_COUNT {
            let o = *slots.add(k);
            if !o.is_null() {
                let vt: *const usize = rd(o, 0);
                let del: DeleteFn = std::mem::transmute(*vt);
                del(o, 1);
            }
            *slots.add(k) = std::ptr::null_mut();
        }
        let panel: *mut u8 = rd(camp, PANEL);
        let lay: *const u8 = rd(camp, LAYOUT);
        let set: *const u8 = rd(camp, SET);
        if panel.is_null() || lay.is_null() || set.is_null() {
            return;
        }
        let new: NewFn = f(NEW_FN);
        let ctor: CtorFn = f(CTOR_FN);
        let setup: SetupFn = f(SETUP_FN);
        let no_scale = rd::<i32>(camp, NO_SCALE) != 0;
        let f10 = if no_scale { 0.0 } else { 2.0 };
        let font_global = *(va(base(), sites::data::FONT_GLOBAL) as *const *const u8);
        let lang = if font_global.is_null() { 0 } else { std::ptr::read_unaligned(font_global.add(0x274) as *const i32) };
        let label_font = if !no_scale && (lang == 3 || lang == 6) { 0xC } else { 0xD };
        let label_seq = va(base(), sites::data::LABEL_SEQ) as *const u16;
        let value_seq = va(base(), sites::data::VALUE_SEQ) as *const *const u16;
        let s = super::settings();
        let mut slot = 0usize;
        let mut alloc = || -> *mut u8 {
            if slot >= menu::SLOT_COUNT {
                return std::ptr::null_mut();
            }
            let b = new(0x30);
            if b.is_null() {
                return b;
            }
            ctor(b);
            *slots.add(slot) = b;
            slot += 1;
            b
        };
        let mut highlighted: Option<(Item, *mut u8, u32, f32)> = None;
        for (row, &it) in self.page.items().iter().enumerate().take(menu::MAX_ROWS) {
            let (lx, ly) = layout::LABEL[row];
            let (Some(lx), Some(ly), Some(vy)) = (pos(lay, lx), pos(lay, ly), pos(lay, layout::VALUE_Y[row])) else {
                if !LAYOUT_WARNED.swap(true, Ordering::Relaxed) {
                    log("kh1 entry: a camp.lad layout element was not found; the settings rows are not drawn");
                }
                return;
            };
            let label = alloc();
            if label.is_null() {
                return;
            }
            let lp = Vec4([lx.0[0], ly.0[1], 0.0, 1.0]);
            setup(label, set, 0x10, label_seq, lp.0.as_ptr(), panel, it.label().as_ptr(), label_font, 0.0, f10, 0x42, std::ptr::null(), -1);
            let value = it.value(&s);
            let style = it.style();
            for (opt, at) in menu::values_at(row, style).into_iter().enumerate() {
                let Some(vx) = pos(lay, at.x_elem) else { return };
                let item = alloc();
                if item.is_null() {
                    return;
                }
                let vp = Vec4([vx.0[0], vy.0[1], 0.0, 1.0]);
                let seq = *value_seq.add(at.item);
                let (text, sel, anim) = match style {
                    // As Re:CoM's row 4: one value item showing the current option, selected 0.
                    Style::Single => (it.options()[value], false, menu::CURSOR_ANIM_SINGLE),
                    Style::Pair => (it.options()[opt], menu::selected(value, opt), menu::CURSOR_ANIM),
                };
                setup(item, set, 0x10, seq, vp.0.as_ptr(), panel, text.as_ptr(), 0xD, 0.0, f10, 0x42, std::ptr::null(), sel as i32);
                if row == self.cursor && (sel || style == Style::Single) {
                    highlighted = Some((it, item, anim, menu::cursor_x_offset(at.item)));
                }
            }
        }
        if let Some((it, item, anim, dx)) = highlighted {
            move_cursor(camp, set, item, anim, dx);
            let help: *mut u8 = rd(camp, HELP);
            if !help.is_null() {
                f::<SetTextFn>(SET_TEXT_FN)(help, it.help().as_ptr());
            }
        }
    }
}

unsafe fn pos(lay: *const u8, elem: u64) -> Option<Vec4> {
    let layout_pos: LayoutFn = f(LAYOUT_FN);
    let mut v = Vec4::default();
    (layout_pos(lay, layout::GROUP, elem, v.0.as_mut_ptr(), 0) != 0).then_some(v)
}

/// Re:CoM's cursor sprite onto `item`, as its mode 9 does for the selected value
/// (disasm `0x1400DF931..0x1400DF9EF`).
unsafe fn move_cursor(camp: *mut u8, set: *const u8, item: *mut u8, cursor_anim: u32, dx: f32) {
    let cur: *mut u8 = rd(camp, CURSOR_SPRITE);
    if cur.is_null() {
        return;
    }
    let vt: *const usize = rd(cur, 0);
    let anim: AnimFn = std::mem::transmute(*vt.add(0x20 / 8));
    anim(cur, 0, set, cursor_anim, 0.0, 0);
    let sprite: *mut u8 = rd(item, 0x18);
    if sprite.is_null() {
        return;
    }
    let set_pos: SetPosFn = f(SET_POS_FN);
    set_pos(cur, sprite.add(0xF0) as *const f32);
    let child: *mut c_void = rd(cur, 0x170);
    if child.is_null() {
        return;
    }
    let mut v: Vec4 = rd(sprite, 0xF0);
    v.0[0] += dx;
    std::ptr::write_unaligned(cur.add(0x190) as *mut Vec4, v);
    std::ptr::write_unaligned(cur.add(0x1B0) as *mut u64, 1);
    let src: Vec4 = if rd::<i32>(cur, 0x1B8) != 0 { rd(cur, 0x180) } else { v };
    std::ptr::write_unaligned(cur.add(0x1A0) as *mut Vec4, src);
}
