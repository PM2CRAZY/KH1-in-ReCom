//! Our row in Re:CoM's Config screen, run by Re:CoM's own menu code.
//!
//! - `cfg_input` wraps the screen's input handler `0x1400D3EB0`. Re:CoM's rows go to the
//!   original unchanged; at the two edges and on our rows it does what the original does for a
//!   row (cursor sound, value flip, redraw) with the same game functions.
//! - `cfg_draw` wraps the screen's draw `0x1400DF1B0`. After Re:CoM's mode 9 (rebuild the rows)
//!   it adds our label and value items in the camp's free item slots, built by the same
//!   `CButton` code with positions from the same layout, and moves Re:CoM's cursor sprite and
//!   help line when the cursor is on our row. Re:CoM deletes those items itself (modes 6 / 9 and
//!   the camp teardown clear all 21 slots). After the modes that give the window panel a new
//!   animation (0, 1, 4) it sets the bottom edge offset of the panel's 9-slice sprites, so
//!   Re:CoM's own sprite code draws the panel one row taller (`menu::panel`).
//!
//! Offsets and calls: `docs/recom-config-menu.md`. Checked (decomp + disasm) unless noted.

use super::{base, log, va};
use crate::menu::{self, layout, Input, Step};
use crate::sites;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub static ORIG_INPUT: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_DRAW: AtomicUsize = AtomicUsize::new(0);
static LAYOUT_WARNED: AtomicBool = AtomicBool::new(false);
static PANEL_LOGS: AtomicUsize = AtomicUsize::new(0);

// CCamp fields used by the Config screen.
const STATE: usize = 0xA8;
const NO_SCALE: usize = 0x9C;
const LAYOUT: usize = 0x78;
const SET: usize = 0x80;
const HELP: usize = 0x9D0;
const PANEL: usize = 0xB30;
const CURSOR_SPRITE: usize = 0xBE0;
const CURSOR: usize = 0xBE8;

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
type SeqByHandleFn = unsafe extern "win64" fn(u32) -> *mut u8;

const DIR_FN: u32 = 0x0011_1A70;
const CONFIRM_FN: u32 = 0x0011_1B90;
const SOUND_FN: u32 = 0x002A_A360;
const NEW_FN: u32 = 0x004A_CE54;
const CTOR_FN: u32 = 0x0011_2A90;
const SETUP_FN: u32 = 0x0011_2F10;
const LAYOUT_FN: u32 = 0x0024_6100;
const SET_TEXT_FN: u32 = 0x0025_9000;
const SET_POS_FN: u32 = 0x0024_85B0;
const DRAW_RVA: u32 = 0x000D_F1B0;
const SEQ_BY_HANDLE_FN: u32 = 0x0024_6210;

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

/// Settings value our row `k` shows (option 1 = KH1).
fn row_value(_k: usize) -> bool {
    super::settings().combat == crate::settings::Mode::Kh1
}

/// Hook on `0x1400D3EB0` (the Config screen's input/state function).
pub unsafe extern "win64" fn cfg_input(camp: *mut u8) -> u64 {
    let orig: InputFn = std::mem::transmute(ORIG_INPUT.load(Ordering::Acquire));
    if camp.is_null() || rd::<i32>(camp, STATE) != 2 {
        return orig(camp);
    }
    let confirm: ConfirmFn = f(CONFIRM_FN);
    if confirm(camp, 0) != 0 {
        // Closing the screen: Re:CoM's own path (it never indexes by the cursor there).
        return orig(camp);
    }
    let dir: DirFn = f(DIR_FN);
    let i = Input { up: dir(camp, UP, 0) != 0, down: dir(camp, DOWN, 0) != 0, left: dir(camp, LEFT, 0) != 0, right: dir(camp, RIGHT, 0) != 0 };
    let cursor = rd::<i32>(camp, CURSOR);
    let step = menu::step(cursor, i);
    if step == Step::Original {
        return orig(camp);
    }
    // What the original does every frame in this state before reading the pad: animate.
    let draw: DrawFn = f(DRAW_RVA);
    draw(camp, 3);
    let sound: SoundFn = f(SOUND_FN);
    match step {
        Step::Move(c) => {
            *(camp.add(CURSOR) as *mut i32) = c;
        }
        Step::Toggle(k) => super::toggle(k),
        Step::Idle | Step::Original => return 0,
    }
    // As the original after any change: cursor sound (1, -1) and a redraw.
    sound(1, -1);
    draw(camp, 9);
    0
}

/// Hook on `0x1400DF1B0` (the Config screen's draw).
pub unsafe extern "win64" fn cfg_draw(camp: *mut u8, mode: u32) -> u64 {
    let orig: DrawFn = std::mem::transmute(ORIG_DRAW.load(Ordering::Acquire));
    let r = orig(camp, mode);
    if !camp.is_null() {
        if mode == 9 {
            build_rows(camp);
        } else if menu::panel::restretch_after(mode) {
            stretch_panel(camp);
        }
    }
    r
}

/// Re:CoM's Config window panel, one row taller (`menu::panel`). On each sprite of the panel's
/// current sequence: a fresh 9-slice gets its bottom edge offset and flag 8; a frame sprite (the
/// 3x3 without its centre) gets, for each part that moves, its own copy of the part's rect
/// moved down. Re:CoM's sprite code does all the drawing; nothing runs per frame once the
/// screen is open (mode 3 is not touched).
unsafe fn stretch_panel(camp: *mut u8) {
    use menu::panel;
    let ctrl: *mut u8 = rd(camp, PANEL);
    if ctrl.is_null() || rd::<u8>(ctrl, panel::CTRL_COUNT) == 0 {
        return;
    }
    let seq_by_handle: SeqByHandleFn = f(SEQ_BY_HANDLE_FN);
    let seq = seq_by_handle(rd::<u32>(ctrl, panel::CTRL_HANDLES));
    if seq.is_null() {
        return;
    }
    let list: *mut u8 = rd(seq, panel::SEQ_LIST);
    if list.is_null() {
        return;
    }
    let spr_vt = va(base(), sites::data::SPR_OBJ_VTABLE);
    let mut grow: Option<Option<f32>> = None;
    let mut grow_now = || *grow.get_or_insert_with(|| row_step(camp).map(|(y3, y4)| panel::grow(y3, y4)));
    let mut report: Vec<String> = Vec::new();
    let mut changed = false;
    let mut obj: *mut u8 = rd(list, 8);
    // The list is Re:CoM's; a bound guards against a broken one.
    for _ in 0..64 {
        if obj.is_null() {
            break;
        }
        let (what, did) = stretch_sprite(obj, spr_vt, &mut grow_now);
        report.push(what);
        changed |= did;
        obj = rd(obj, 8);
    }
    // Logged when something changed (each animation's new sprites), a few times per session.
    if changed && PANEL_LOGS.fetch_add(1, Ordering::Relaxed) < 6 {
        let g = grow.flatten().map(|g| format!("{g:.1}")).unwrap_or_else(|| "?".into());
        log(&format!("config menu: window panel, one row ({g} units) taller; sprites: {}", report.join("; ")));
    }
}

/// One sprite of the panel; returns what was done (for the log) and whether it changed it.
unsafe fn stretch_sprite(obj: *mut u8, spr_vt: usize, grow: &mut dyn FnMut() -> Option<f32>) -> (String, bool) {
    let what = stretch_sprite_inner(obj, spr_vt, grow);
    let changed = what.starts_with("9 parts (9-slice)") || what.contains("moved/stretched");
    (what, changed)
}

unsafe fn stretch_sprite_inner(obj: *mut u8, spr_vt: usize, grow: &mut dyn FnMut() -> Option<f32>) -> String {
    use menu::panel::{self, Sprite};
    if rd::<usize>(obj, 0) != spr_vt {
        return "other object, left".into();
    }
    let n = rd::<u8>(obj, panel::PART_COUNT);
    let parts: *mut u8 = rd(obj, panel::PARTS);
    if parts.is_null() || n == 0 {
        return format!("{n} parts, no part array, left");
    }
    let flags = rd::<u8>(obj, panel::FLAGS);
    if n == panel::NINE {
        let fresh = flags & panel::FLAG_NINE_SLICE == 0;
        let g = if fresh { grow() } else { Some(0.0) };
        let Some(g) = g else { return "9 parts, layout unreadable, left".into() };
        return match panel::plan(true, n, true, flags, g) {
            Sprite::Stretch(off) => {
                std::ptr::write_unaligned(obj.add(panel::OFFSETS) as *mut [f32; 4], off);
                *obj.add(panel::FLAGS) |= panel::FLAG_NINE_SLICE;
                "9 parts (9-slice), bottom edge offset set".into()
            }
            Sprite::Done => "9 parts, already set".into(),
            Sprite::Skip => "9 parts, left".into(),
        };
    }
    // Part rects (and whether a part already has one of our copies).
    let mut src: Vec<*const u8> = Vec::with_capacity(n as usize);
    for i in 0..n as usize {
        let p: *const u8 = rd(parts.add(i * panel::PART_STRIDE), 0);
        if p.is_null() {
            return format!("{n} parts, a part without a rect, left");
        }
        src.push(p);
    }
    let rect = |p: *const u8| -> [i32; 4] { std::ptr::read_unaligned(p as *const [i32; 4]) };
    let scissor = !rd::<*const u8>(obj, 0x128).is_null();
    if n == 1 {
        let r = rect(src[0]);
        return format!("1 part{} rect {:?}, left", if scissor { " (scissor)" } else { "" }, r);
    }
    if src.iter().any(|&p| is_our_copy(p)) {
        return format!("{n} parts, frame already moved");
    }
    let rects: Vec<[i32; 4]> = src.iter().map(|&p| rect(p)).collect();
    let Some(g) = grow() else { return format!("{n} parts, layout unreadable, left") };
    let Some(new) = panel::frame_rects(&rects, g.round() as i32) else {
        return format!("{n} parts, not a frame, left (rects {rects:?})");
    };
    // Every copy first, so the frame is moved whole or not at all.
    let mut writes = Vec::new();
    for (i, nr) in new.iter().enumerate() {
        let Some(nr) = nr else { continue };
        let Some(copy) = rect_copy(src[i], *nr) else {
            return format!("{n} parts, frame: copy limit reached, left");
        };
        writes.push((i, copy));
    }
    for &(i, copy) in &writes {
        std::ptr::write_unaligned(parts.add(i * panel::PART_STRIDE) as *mut *const u8, copy);
    }
    format!("{n} parts (frame), {} parts moved/stretched", writes.len())
}

/// Our rect copies: (source entry, its bytes when copied, our copy). Copies live for the whole
/// process (Re:CoM's parts keep pointing at them). Bounded.
#[allow(clippy::type_complexity)]
static RECT_COPIES: std::sync::Mutex<Vec<(usize, [u8; menu::panel::RECT_SIZE], [i32; 4], usize)>> = std::sync::Mutex::new(Vec::new());
const RECT_COPY_LIMIT: usize = 256;

unsafe fn is_our_copy(p: *const u8) -> bool {
    RECT_COPIES.lock().map(|v| v.iter().any(|e| e.3 == p as usize)).unwrap_or(false)
}

/// A copy of the rect entry at `src` with the rect `r` (other bytes as in `src`), reused when the
/// same source, with the same bytes, was copied before.
unsafe fn rect_copy(src: *const u8, r: [i32; 4]) -> Option<*const u8> {
    let orig: [u8; menu::panel::RECT_SIZE] = std::ptr::read_unaligned(src as *const [u8; menu::panel::RECT_SIZE]);
    let mut v = RECT_COPIES.lock().ok()?;
    if let Some(e) = v.iter().find(|e| e.0 == src as usize && e.1 == orig && e.2 == r) {
        return Some(e.3 as *const u8);
    }
    if v.len() >= RECT_COPY_LIMIT {
        return None;
    }
    let mut b = orig;
    for (k, y) in r.iter().enumerate() {
        b[k * 4..k * 4 + 4].copy_from_slice(&y.to_le_bytes());
    }
    let copy = Box::leak(Box::new(b)).as_ptr();
    v.push((src as usize, orig, r, copy as usize));
    Some(copy)
}

/// Re:CoM's value-row step (layout y of row 4 minus row 3), as `build_rows` uses it.
unsafe fn row_step(camp: *mut u8) -> Option<(f32, f32)> {
    let lay: *const u8 = rd(camp, LAYOUT);
    if lay.is_null() {
        return None;
    }
    let y3 = pos(lay, layout::VALUE_Y[3])?;
    let y4 = pos(lay, layout::VALUE_Y[4])?;
    Some((y3.0[1], y4.0[1]))
}

unsafe fn pos(lay: *const u8, elem: u64) -> Option<Vec4> {
    let layout_pos: LayoutFn = f(LAYOUT_FN);
    let mut v = Vec4::default();
    (layout_pos(lay, layout::GROUP, elem, v.0.as_mut_ptr(), 0) != 0).then_some(v)
}

unsafe fn build_rows(camp: *mut u8) {
    let panel: *mut u8 = rd(camp, PANEL);
    let lay: *const u8 = rd(camp, LAYOUT);
    let set: *const u8 = rd(camp, SET);
    if panel.is_null() || lay.is_null() || set.is_null() {
        return;
    }
    // Positions: below Re:CoM's last row, one row step at a time.
    let (lx, ly3, ly4) = (pos(lay, layout::LABEL[3].0), pos(lay, layout::LABEL[3].1), pos(lay, layout::LABEL[4].1));
    let (vx0, vx1) = (pos(lay, layout::VALUE_X[6]), pos(lay, layout::VALUE_X[7]));
    let (vy3, vy4) = (pos(lay, layout::VALUE_Y[3]), pos(lay, layout::VALUE_Y[4]));
    let (Some(lx), Some(ly3), Some(ly4), Some(vx0), Some(vx1), Some(vy3), Some(vy4)) = (lx, ly3, ly4, vx0, vx1, vy3, vy4) else {
        if !LAYOUT_WARNED.swap(true, Ordering::Relaxed) {
            log("config menu: a camp.lad layout element was not found; our rows are not drawn");
        }
        return;
    };
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
    // Row 3's value items (6 and 7) are the two-choice ones we copy.
    let seq = [*value_seq.add(6), *value_seq.add(7)];
    let cursor = rd::<i32>(camp, CURSOR);
    let slots = camp.add(menu::SLOT_BASE) as *mut *mut u8;
    let mut highlighted: Option<(usize, *mut u8)> = None;
    for k in 0..menu::ROWS {
        let [ls, v0, v1] = menu::slots(k);
        let text = &menu::TEXT[k];
        let mut items = [std::ptr::null_mut::<u8>(); 3];
        for (n, &slot) in [ls, v0, v1].iter().enumerate() {
            if !(*slots.add(slot)).is_null() {
                // Not ours to replace (Re:CoM never fills these; another mod might).
                log("config menu: item slot already in use; our rows are not drawn");
                return;
            }
            let b = new(0x30);
            if b.is_null() {
                return;
            }
            ctor(b);
            *slots.add(slot) = b;
            items[n] = b;
        }
        let row = menu::FIRST_ROW + k as i32;
        let lp = Vec4([lx.0[0], menu::row_y(ly3.0[1], ly4.0[1], k), 0.0, 1.0]);
        setup(items[0], set, 0x10, label_seq, lp.0.as_ptr(), panel, text.label.as_ptr(), label_font, 0.0, f10, 0x42, std::ptr::null(), -1);
        let vy = menu::row_y(vy3.0[1], vy4.0[1], k);
        let value = row_value(k);
        for opt in 0..2 {
            let x = if opt == 0 { vx0.0[0] } else { vx1.0[0] };
            let vp = Vec4([x, vy, 0.0, 1.0]);
            let sel = menu::selected(value, opt);
            setup(items[1 + opt], set, 0x10, seq[opt], vp.0.as_ptr(), panel, text.options[opt].as_ptr(), 0xD, 0.0, f10, 0x42, std::ptr::null(), sel as i32);
            if sel && row == cursor {
                highlighted = Some((k, items[1 + opt]));
            }
        }
    }
    if let Some((k, item)) = highlighted {
        move_cursor(camp, set, item);
        let help: *mut u8 = rd(camp, HELP);
        if !help.is_null() {
            let set_text: SetTextFn = f(SET_TEXT_FN);
            set_text(help, menu::TEXT[k].help.as_ptr());
        }
    }
}

/// Re:CoM's cursor sprite onto `item`, as the original does for a selected value
/// (disasm `0x1400DF931..0x1400DF9EF`).
unsafe fn move_cursor(camp: *mut u8, set: *const u8, item: *mut u8) {
    let cur: *mut u8 = rd(camp, CURSOR_SPRITE);
    if cur.is_null() {
        return;
    }
    let vt: *const usize = rd(cur, 0);
    let anim: AnimFn = std::mem::transmute(*vt.add(0x20 / 8));
    anim(cur, 0, set, menu::CURSOR_ANIM, 0.0, 0);
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
    v.0[0] += menu::CURSOR_X_OFFSET;
    std::ptr::write_unaligned(cur.add(0x190) as *mut Vec4, v);
    std::ptr::write_unaligned(cur.add(0x1B0) as *mut u64, 1);
    let src: Vec4 = if rd::<i32>(cur, 0x1B8) != 0 { rd(cur, 0x180) } else { v };
    std::ptr::write_unaligned(cur.add(0x1A0) as *mut Vec4, src);
}
