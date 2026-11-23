//! The MP bar (M7 stage 7.2; `crate::mp_bar`, `docs/recom-m7-mp-bar.md`), drawn by Re:CoM.
//!
//! - **Object:** a `D2::C2DSeqCtrl` made with Re:CoM's own `operator new` and ctor, as
//!   `CPlayerGauge`'s init (`0x140208360`) makes its HP control `+0x78`, with three layers of
//!   `Cockpit:00` (the groups the asset step added: track, MP, charge), parented to the HP control
//!   with its colour inherited (`+0x138` = 1), exactly as Re:CoM builds Riku's Dark gauge under
//!   the HP bar (`0x14020C190`). Re:CoM's 2D manager animates, draws and clips it.
//! - **Frames** (`0x140247C20`, frame × 50 units): set only when KH1's MP, max MP or charge
//!   change — from the MP API's reads and writes (`win/mp.rs` calls [`sync`]), never per frame.
//! - **Fades:** KH1's 10-frame change fades are time-played groups of `Cockpit:00` on small
//!   controls of their own (one per slot touched, placed with `0x1402485B0` and X-scaled with
//!   `0x1402486D0`), made when a change happens; Re:CoM plays them.
//! - **Wrappers:** the gauge's init (build), its second-bar state `0x14020C190` (Re:CoM's HUD
//!   hide 7 / show 1, as the Dark gauge follows it) and its deleting dtor (our controls go first,
//!   before Re:CoM deletes the HP control they hang from).
//! - Only at Combat = KH1 in Sora's story (`super::mp::active`), and only when the loaded
//!   `Cockpit:00` has exactly Re:CoM's groups plus ours (checked in Re:CoM's 2D registry at each
//!   build). Riku's story and the Re:CoM setting: nothing is made; the Dark gauge is Re:CoM's.
//! - Not made while KH1's MP ring shows (`kh1_hud::ring_on`); mirrored under Re:CoM's HP gauge
//!   when that gauge is at KH1's place (`kh1_hud::place_on`).

use super::movement::{func, orig, rd, wr};
use super::{base, log};
use crate::mp_bar::{self as mb, Layout, Piece, View};
use crate::sites::MP_BAR_PTRS;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

pub static ORIG_INIT: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_STATE: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_DTOR: AtomicUsize = AtomicUsize::new(0);
static INSTALLED: AtomicBool = AtomicBool::new(false);
static LAYOUT: OnceLock<(Layout, Vec<u8>)> = OnceLock::new();
static LOG_LEFT: AtomicU32 = AtomicU32::new(60);
static SET_LOGGED: AtomicBool = AtomicBool::new(false);
static STATE: Mutex<State> = Mutex::new(State { gauges: Vec::new(), bars: Vec::new() });

/// Our first group in the loaded language copy of the set (`Layout::first_for`).
static FIRST: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

// Re:CoM functions (RVAs; first bytes checked in `sites`).
const NEW: u32 = 0x004A_CE54;
const SEQ_CTOR: u32 = 0x0024_7260;
const FRAME: u32 = 0x0024_7C20;
const ALL_VISIBLE: u32 = 0x0024_7EC0;
const PARENT: u32 = 0x0024_8650;
const POSITION: u32 = 0x0024_85B0;
/// The bar's drop below its data position (the Dark gauge's place, under the HP bar), in 2D
/// units: Re:CoM's "FRIENDS" row (label and friend cards) sits there in Sora's story, so the bar
/// goes under the cards (owner's video, 2026-10-31: the bar "clips through the friends bar";
/// cards' bottom ≈ 84 units, the bar's top 50 → +40). Assumption until a screenshot.
const BAR_DY: f32 = 40.0;
/// With Re:CoM's HP gauge at KH1's place, turned (`hud_recom_hp_kh1_place`, MP ring off): the bar
/// is mirrored (X scale −1: it runs left from the ring's centre, as KH1's straight segments) and
/// drops under the turned HP bar (its back reaches 44 units under the ring's centre: the bar's
/// border (−12 … +4 from here) starts 2 units below it). Assumption until a screenshot.
const BAR_DY_PLACED: f32 = 58.0;
const SCALE: u32 = 0x0024_86D0;
// Re:CoM data (RVAs).
const SEQ_CTRL_VTABLE: u32 = 0x0060_7340;
const SEQ_VTABLE: u32 = 0x0060_7260;
/// The 2D manager (non-zero when it exists) and its registry list (`0x140245A20`).
const MANAGER: u32 = 0x0087_CE38;
const REGISTRY: u32 = 0x0087_CE48;

/// `CPlayerGauge` fields (Checked (decomp `0x140208360`, `0x14020A0E0`)): the HP control, the
/// x offset (85.0 at 16:9, 0 at 4:3; every layer is added at its negative), the display state.
const G_HP: usize = 0x78;
const G_XOFF: usize = 0xA0;
const G_STATE: usize = 0xAA;
const STATE_SHOW: u32 = 1;
const STATE_HIDDEN: u32 = 7;
/// `C2DSeqCtrl` fields: draw priority (2 for the HP gauge and the Dark gauge; a clear resets it
/// to 0xFF), "take the parent's colour" (the Dark gauge sets it), layer count.
const C_PRIORITY: usize = 0x16E;
const C_INHERIT: usize = 0x138;
const PRIORITY_HUD: u8 = 2;
/// The C2DSeq's group count (`0x140246DFE`).
const SEQ_GROUPS: usize = 0xF4;
const SEQ_NAME: usize = 0x14;

type GaugeFn = unsafe extern "win64" fn(usize);
type StateFn = unsafe extern "win64" fn(usize, u32);
type DtorFn = unsafe extern "win64" fn(usize, u32) -> usize;
type NewFn = unsafe extern "win64" fn(usize) -> usize;
type CtorFn = unsafe extern "win64" fn(usize) -> usize;
type FrameFn = unsafe extern "win64" fn(usize, u32, i32);
type VisFn = unsafe extern "win64" fn(usize, i32);
type ParentFn = unsafe extern "win64" fn(usize, usize);
type VecFn = unsafe extern "win64" fn(usize, *const f32);

#[repr(C, align(16))]
struct V4([f32; 4]);

struct Bar {
    gauge: usize,
    /// Mirrored under Re:CoM's HP gauge at KH1's place ([`BAR_DY_PLACED`]).
    mirror: bool,
    /// Re:CoM's draw list the gauge was made on: our layers are added and released on it
    /// (`super::d2`).
    list: i32,
    hp: usize,
    ctrl: usize,
    xoff: f32,
    shown: Option<View>,
    hidden: bool,
    mp: Vec<usize>,
    charge: Vec<usize>,
    pool: Vec<usize>,
}

struct State {
    /// Live player gauges (init seen, dtor not yet) and the draw list each was made on, for a bar
    /// made later (Combat switched on).
    gauges: Vec<(usize, i32)>,
    bars: Vec<Bar>,
}

fn note(line: impl FnOnce() -> String) {
    if LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("mp bar: {}", line()));
    }
}

fn layout() -> Option<&'static (Layout, Vec<u8>)> {
    INSTALLED.load(Ordering::Acquire).then(|| LAYOUT.get()).flatten()
}

/// Reads the manifest, checks the vtable slots, and hooks the gauge's init, state and dtor.
pub fn install(b: usize, manifest: Option<&std::path::Path>) -> bool {
    let Some(path) = manifest else {
        log("mp bar: no kh1-mp-bar.json (asset step not run?): no MP bar");
        return false;
    };
    let l = match std::fs::read_to_string(path).map_err(|e| e.to_string()).and_then(|t| mb::parse_manifest(&t)) {
        Ok(l) => l,
        Err(e) => {
            log(&format!("mp bar: {}: {e}; no MP bar", path.display()));
            return false;
        }
    };
    // SAFETY: reads of the mapped image (the build was checked at start).
    unsafe {
        for p in MP_BAR_PTRS {
            let have: usize = rd(b + p.rva as usize);
            if have != b + p.target as usize {
                log(&format!("mp bar: {} holds {have:#x}, not {:#x}; no MP bar", p.name, b + p.target as usize));
                return false;
            }
        }
    }
    let mut name = l.set.clone().into_bytes();
    name.push(0);
    let (first, set) = (l.first_group, l.set.clone());
    let _ = LAYOUT.set((l, name));
    let site = |n: &str| crate::sites::MP_BAR_HOOKS.iter().find(|h| h.name == n).expect("site");
    let hooks = [
        super::hook::Hook { site: site("CPlayerGauge init (0x140208360)"), ours: gauge_init as *const () as usize, original: &ORIG_INIT },
        super::hook::Hook { site: site("CPlayerGauge second-bar state (0x14020C190)"), ours: gauge_state as *const () as usize, original: &ORIG_STATE },
        super::hook::Hook { site: site("CPlayerGauge deleting dtor (0x140207770)"), ours: gauge_dtor as *const () as usize, original: &ORIG_DTOR },
    ];
    // SAFETY: the targets' bytes are checked again inside, before anything is written.
    if !unsafe { super::hook::install_all(b, &hooks) } {
        log("mp bar: gauge wrappers not hooked; no MP bar");
        return false;
    }
    INSTALLED.store(true, Ordering::Release);
    log(&format!(
        "mp bar: KH1 MP bar on Re:CoM's player gauge at combat = kh1 in Sora's story (M7 stage 7.2): {set} groups {first}..{} from {}; frames set on MP changes only",
        first + mb::group::COUNT - 1,
        path.display()
    ));
    true
}

/// The group count of the loaded sprite set `name` (NUL-terminated), from Re:CoM's 2D registry
/// (the walk `0x140245A20` does: list `[0x14087CE48] + 8`, next `+8`, name `+0x14`; a C2DSeq
/// keeps its group count at `+0xF4`).
pub(super) unsafe fn set_groups(name: &[u8]) -> Option<u32> {
    let b = base();
    if rd::<u64>(b + MANAGER as usize) == 0 {
        return None;
    }
    let list: usize = rd(b + REGISTRY as usize);
    if list == 0 {
        return None;
    }
    let vt = b + SEQ_VTABLE as usize;
    let mut node: usize = rd(list + 8);
    for _ in 0..8192 {
        if node == 0 {
            break;
        }
        if rd::<usize>(node) == vt && std::slice::from_raw_parts((node + SEQ_NAME) as *const u8, name.len()) == name {
            return Some(rd(node + SEQ_GROUPS));
        }
        node = rd(node + 8);
    }
    None
}

/// The bar's X scale sign and drop.
fn dir(mirror: bool) -> (f32, f32) {
    if mirror {
        (-1.0, BAR_DY_PLACED)
    } else {
        (1.0, BAR_DY)
    }
}

/// A new `C2DSeqCtrl` (Re:CoM's allocator and ctor), parented to `hp`.
unsafe fn new_ctrl(hp: usize, mirror: bool) -> Option<usize> {
    let c = func::<NewFn>(NEW)(0x170);
    if c == 0 {
        return None;
    }
    func::<CtorFn>(SEQ_CTOR)(c);
    if rd::<usize>(c) != base() + SEQ_CTRL_VTABLE as usize {
        return None;
    }
    super::d2::made();
    func::<ParentFn>(PARENT)(c, hp);
    wr::<u8>(c + C_INHERIT, 1);
    // The control's own transform starts at rest: a rebuilt bar (Combat switched back to KH1)
    // otherwise drew wherever the reused memory left it (owner, 2026-10-31: "randomly placed
    // somewhere else each time").
    let (sx, dy) = dir(mirror);
    func::<VecFn>(POSITION)(c, V4([0.0, dy, 0.0, 1.0]).0.as_ptr());
    func::<VecFn>(SCALE)(c, V4([sx, 1.0, 1.0, 1.0]).0.as_ptr());
    Some(c)
}

/// Adds group `first + k` as a layer (flag 1: held at the frame we set; 0: played over time) on
/// draw list `list`.
unsafe fn add(c: usize, list: i32, l: &(Layout, Vec<u8>), k: u32, xoff: f32, flag: u32) {
    wr::<u8>(c + C_PRIORITY, PRIORITY_HUD);
    super::d2::add(c, list, l.1.as_ptr(), FIRST.load(Ordering::Relaxed) + k, xoff, flag);
}

impl Bar {
    unsafe fn release_pieces(&mut self, which: u32) {
        let list = if which == 0 { &mut self.mp } else { &mut self.charge };
        for c in list.drain(..) {
            super::d2::clear(c, self.list);
            self.pool.push(c);
        }
    }

    unsafe fn place(&mut self, l: &(Layout, Vec<u8>), p: &Piece) -> Option<usize> {
        let c = match self.pool.pop() {
            Some(c) => c,
            None => new_ctrl(self.hp, self.mirror)?,
        };
        let (sx, dy) = dir(self.mirror);
        func::<VecFn>(POSITION)(c, V4([sx * p.x, dy, 0.0, 1.0]).0.as_ptr());
        let s = V4([sx * p.scale, 1.0, 1.0, 1.0]);
        func::<VecFn>(SCALE)(c, s.0.as_ptr());
        add(c, self.list, l, p.group, self.xoff, 0);
        Some(c)
    }

    /// Shows `view`: the layers' frames and KH1's fades for the change from what was shown.
    unsafe fn show(&mut self, l: &(Layout, Vec<u8>), view: View) {
        let plan = mb::plan(self.shown, view, &l.0);
        for (i, f) in plan.frames.iter().enumerate() {
            if let Some(f) = f {
                func::<FrameFn>(FRAME)(self.ctrl, i as u32, *f);
            }
        }
        for (which, pieces) in [(0u32, &plan.mp), (1, &plan.charge)] {
            let Some(pieces) = pieces else { continue };
            self.release_pieces(which);
            let made: Vec<usize> = pieces.iter().filter_map(|p| self.place(l, p)).collect();
            if which == 0 {
                self.mp = made;
            } else {
                self.charge = made;
            }
        }
        if let Some(old) = self.shown {
            if !plan.mp.as_ref().is_none_or(|p| p.is_empty()) || !plan.charge.as_ref().is_none_or(|p| p.is_empty()) {
                note(|| format!("{}/{} c{} -> {}/{} c{}: frames {:?}, fade pieces MP {}, charge {}", old.0, old.1, old.2, view.0, view.1, view.2, plan.frames, self.mp.len(), self.charge.len()));
            }
        }
        self.shown = Some(view);
    }

    unsafe fn hide(&mut self) {
        func::<VisFn>(ALL_VISIBLE)(self.ctrl, 0);
        self.release_pieces(0);
        self.release_pieces(1);
        self.hidden = true;
    }

    unsafe fn destroy(&mut self) {
        for c in self.mp.drain(..).chain(self.charge.drain(..)).chain(self.pool.drain(..)) {
            super::d2::delete(c, self.list);
        }
        super::d2::delete(self.ctrl, self.list);
        self.ctrl = 0;
    }
}

/// Builds the bar on `gauge` (its init has run, on draw list `list`). The MP view is read first:
/// `mp::current` calls [`sync`], which takes the lock.
unsafe fn build(gauge: usize, list: i32) {
    let Some(l) = layout() else { return };
    // Sora's, or Riku's (`crate::riku`: under his HP gauge, as Re:CoM's own Dark gauge).
    if !super::mp::live() {
        return;
    }
    let hp: usize = rd(gauge + G_HP);
    if hp == 0 {
        return;
    }
    match set_groups(&l.1).and_then(|n| l.0.first_for(n).map(|f| (n, f))) {
        Some((_, f)) => FIRST.store(f, Ordering::Relaxed),
        None => {
            let other = set_groups(&l.1);
            if !SET_LOGGED.swap(true, Ordering::Relaxed) {
                log(&format!(
                    "mp bar: {} has {} groups, not {} (Panacea serves Re:CoM's own FORM/0002/FO0002.SPR, or this language's copy was not patched): no MP bar",
                    l.0.set,
                    other.map_or("no".to_string(), |n| n.to_string()),
                    l.0.total_groups()
                ));
            }
            return;
        }
    }
    // The KH1 HUD draws KH1's own MP ring instead (decision 39, H2; or around Re:CoM's HP ring,
    // `hud_kh1_mp_ring`).
    if super::kh1_hud::gauge_on() || super::kh1_hud::ring_on() {
        return;
    }
    let Some(view) = super::mp::current() else { return };
    // Re:CoM's HP gauge moves to KH1's place right after this (`kh1_hud::gauge_made`).
    let mirror = super::kh1_hud::place_on();
    let Some(ctrl) = new_ctrl(hp, mirror) else { return };
    let xoff = -rd::<f32>(gauge + G_XOFF);
    for k in [mb::group::TRACK, mb::group::MP, mb::group::CHARGE] {
        add(ctrl, list, l, k, xoff, 1);
    }
    let mut bar = Bar { gauge, mirror, list, hp, ctrl, xoff, shown: None, hidden: false, mp: Vec::new(), charge: Vec::new(), pool: Vec::new() };
    bar.show(l, view);
    let hidden = rd::<u16>(gauge + G_STATE) as u32 == STATE_HIDDEN;
    if hidden {
        bar.hide();
    }
    note(|| format!("built on the player gauge {gauge:#x}: MP {}/{} c{}, x offset {xoff}{}{}", view.0, view.1, view.2, if mirror { ", mirrored under Re:CoM's HP gauge at KH1's place" } else { "" }, if hidden { " (HUD hidden)" } else { "" }));
    if let Ok(mut s) = STATE.lock() {
        for mut old in s.bars.extract_if(.., |b| b.gauge == gauge) {
            old.destroy();
        }
        s.bars.push(bar);
    } else {
        bar.destroy();
    }
}

/// `CPlayerGauge` init (`0x140208360`): Re:CoM's, then our bar, then the command menu (M7
/// stage 7.3, `super::command_menu`), which uses the plate groups after the bar's.
pub unsafe extern "win64" fn gauge_init(gauge: usize) {
    orig::<GaugeFn>(&ORIG_INIT)(gauge);
    // The list Re:CoM's init just made the gauge's own controls on.
    let list = super::d2::current_list();
    if let Ok(mut s) = STATE.lock() {
        s.gauges.retain(|g| g.0 != gauge);
        s.gauges.push((gauge, list));
    }
    build(gauge, list);
    super::command_menu::build(gauge, list);
    super::kh1_hud::gauge_made(gauge, list);
    super::d2::census(|| format!("player gauge {gauge:#x} made (draw list {list})"));
}

/// Whether `kh1-mp-bar.json` carries the command menu's plate groups (checked at install; the
/// group numbers themselves are known only once a bar is built, [`menu_groups`]).
pub fn has_menu_groups() -> bool {
    layout().is_some_and(|l| l.0.extra_groups >= 2)
}

/// The command menu's two plate groups (`kh1-mp-bar.json` `extra_groups`, after the bar's in the
/// loaded copy of the set) and the set's NUL-terminated name. None before a bar was built on a
/// set that has them.
pub fn menu_groups() -> Option<(*const u8, u32)> {
    let l = layout()?;
    let f = FIRST.load(Ordering::Relaxed);
    (l.0.extra_groups >= 2 && f != 0).then(|| (l.1.as_ptr(), f + mb::group::COUNT))
}

/// 0.111.0: the command menu's pointing arrow (`crate::command_menu::arrow`): the third group the
/// asset step adds after the bar's (`kh1-mp-bar.json` `extra_groups` ≥ 3; an older asset step
/// built two: no arrow), and the set's NUL-terminated name.
pub fn arrow_group() -> Option<(*const u8, u32)> {
    let l = layout()?;
    let f = FIRST.load(Ordering::Relaxed);
    (l.0.extra_groups >= 3 && f != 0).then(|| (l.1.as_ptr(), f + mb::group::COUNT + ARROW_GROUP))
}

/// The arrow's group after the plates' first (the asset step's `command_menu::group::ARROW`).
const ARROW_GROUP: u32 = 2;

/// `CPlayerGauge` second-bar state (`0x14020C190(gauge, state)`): Re:CoM's (the Dark gauge),
/// then ours follows the HUD: 7 hides, 1 shows and rescrubs.
pub unsafe extern "win64" fn gauge_state(gauge: usize, state: u32) {
    orig::<StateFn>(&ORIG_STATE)(gauge, state);
    if state != STATE_HIDDEN && state != STATE_SHOW {
        return;
    }
    let mut show = false;
    if let Ok(mut s) = STATE.lock() {
        if let Some(bar) = s.bars.iter_mut().find(|b| b.gauge == gauge) {
            if state == STATE_HIDDEN && !bar.hidden {
                bar.hide();
            } else if state == STATE_SHOW && bar.hidden {
                func::<VisFn>(ALL_VISIBLE)(bar.ctrl, 1);
                bar.hidden = false;
                bar.shown = None;
                show = true;
            }
        }
    }
    if show {
        refresh();
    }
    super::command_menu::hud(gauge, state);
    super::kh1_hud::hud(gauge, state);
}

/// `CPlayerGauge`'s deleting dtor (`0x140207770`): our controls first (they hang from its HP
/// control), then Re:CoM's.
pub unsafe extern "win64" fn gauge_dtor(gauge: usize, flags: u32) -> usize {
    super::command_menu::destroy(gauge);
    super::kh1_hud::destroy(gauge);
    if let Ok(mut s) = STATE.lock() {
        s.gauges.retain(|g| g.0 != gauge);
        for mut b in s.bars.extract_if(.., |b| b.gauge == gauge) {
            b.destroy();
        }
    }
    let r = orig::<DtorFn>(&ORIG_DTOR)(gauge, flags);
    super::d2::census(|| format!("player gauge {gauge:#x} deleted"));
    r
}

/// KH1's MP as just read or written (`win/mp.rs`): rescrub the bars whose shown value differs.
/// One lock and a compare when nothing changed.
pub fn sync(view: View) {
    let Some(l) = layout() else { return };
    let Ok(mut s) = STATE.lock() else { return };
    for b in s.bars.iter_mut().filter(|b| !b.hidden && b.shown != Some(view)) {
        // SAFETY: a live bar (removed in the gauge's dtor before Re:CoM frees anything).
        unsafe { b.show(l, view) };
    }
}

/// After the Config or the camp: the bars follow the setting. Combat = KH1 in Sora's story: a
/// bar on every live gauge, showing the current MP; else none shows.
pub fn refresh() {
    if layout().is_none() {
        return;
    }
    super::kh1_hud::refresh();
    if super::kh1_hud::gauge_on() || super::kh1_hud::ring_on() {
        // The KH1 HUD's MP ring replaces the bar.
        if let Ok(mut s) = STATE.lock() {
            for mut b in s.bars.drain(..) {
                // SAFETY: as in `sync`.
                unsafe { b.destroy() };
            }
        }
    }
    if super::mp::live() {
        let missing: Vec<(usize, i32)> = STATE.lock().map(|s| s.gauges.iter().copied().filter(|g| !s.bars.iter().any(|b| b.gauge == g.0)).collect()).unwrap_or_default();
        for (g, list) in missing {
            // SAFETY: a live gauge (its dtor removes it from the list).
            unsafe { build(g, list) };
        }
        let _ = super::mp::current(); // calls `sync`
        let gauges: Vec<(usize, i32)> = STATE.lock().map(|s| s.gauges.clone()).unwrap_or_default();
        super::command_menu::refresh(&gauges);
    } else if let Ok(mut s) = STATE.lock() {
        for mut b in s.bars.drain(..) {
            // SAFETY: as in `sync`.
            unsafe { b.destroy() };
        }
        note(|| "combat is not KH1 here: bars removed".to_string());
        drop(s);
        super::command_menu::refresh(&[]);
    }
    super::d2::census(|| "after the Config or the camp".to_string());
}
