//! The KH1 HUD option (decision 39; `crate::kh1_hud`, `docs/recom-kh1-hud.md`).
//!
//! Stage H1: the `kh1hud` sprite set. The asset step adds `kh1hud.lad` / `kh1hud.imz` (and their
//! PC copies) to Re:CoM's `FO0002.SPR` / `.RTM`, which Re:CoM loads with the player HUD; Re:CoM
//! registers `kh1hud:00` itself. Here: the manifest, and at each player gauge's init (the MP bar's
//! wrapper) a check that Re:CoM's 2D registry holds `kh1hud:00` with the manifest's group count,
//! logged once.
//!
//! Stage H2: Sora's gauge cluster at `hud = kh1` in Sora's story. Three `D2::C2DSeqCtrl`s made
//! with Re:CoM's `operator new` and ctor, as the command menu's rows (no parent; layout position
//! `0x140248660` = KH1's size-class offset; each layer at KH1's right margin): the back (priority
//! 2: frame, HP back, MP back, MP), the front (3: HP, charge, max-MP ticks) and the face (3).
//! Frames are KH1's values (`crate::kh1_hud::frames`), set only on events: Re:CoM's HP setters
//! (`0x14020D6A0`, `0x14020D8E0`, wrapped here), KH1's MP reads and writes (`win/mp.rs`), the
//! gauge's init / HUD state / dtor (the MP bar's wrappers). KH1's fades, hurt face and danger flash
//! are time-played groups of the set on small controls of their own, made on those events; Re:CoM
//! plays them. The 7.2 MP bar is not made at `hud = kh1`.
//!
//! Stage H2b: KH1's cluster motions (the hit shake, the size-class slide, the gauge's 10-frame
//! fade) are played groups of the set, one per *unit* (a piece set of the held groups) and
//! program; on a hit / class change / HUD hide or show the held controls hide and the units of
//! what they show play the program from its start (`0x140248550`), rebuilt at the same time
//! (`0x140248000`) when the view changes meanwhile. `docs/recom-kh1-hud.md` § H2b.
//!
//! Stage H3: Re:CoM's HUD pieces are hidden while the KH1 HUD is up, their logic untouched:
//! - the HP gauge (`+0x78`), the FRIENDS row (`CFriendGauge`: its 3 controls `+0x78`; its cards
//!   take their parent's colour) and the prize gauge (`CPrizeGauge`: EXP / NEXT LV / Moogle
//!   points; controls `+0x70` ×6, `+0x88` ×6, `+0xA0` ×2, card `+0xA8`) get colour alpha 0
//!   (`0x140248590`, `+0x138` = 1, as Re:CoM's camp fades its labels): they keep running (the
//!   battle end waits on the prize gauge; the HP gauge plays Re:CoM's low-HP sound);
//! - the enemy gauges and the minimap run their own hide paths: their reads of the HUD-hide mask
//!   (1, 0xE2) go through our getter, which adds their bit (0x10, 8) while the KH1 HUD is up.
//! Kept: the level-up choice, the "Guard" / "Recover" / "Resist" / "LEVEL UP!" popups
//! (`CCommonGauge`), the command menu.
//!
//! Re:CoM's HP gauge at KH1's place (owner, 2026-11-01, `hud_recom_hp_kh1_place`, with
//! `hud_kh1_gauge` off): at the gauge's init its control `+0x78` gets the asset step's turned ring
//! groups on layers 1..3 (Re:CoM's own layer replace) and KH1's centre as its layout position; the
//! disc and the face stay Re:CoM's, upright. Re:CoM runs it from then on. KH1's MP ring around it
//! (`hud_kh1_mp_ring`) is the cluster with only its MP parts (`Cluster::mp_only`).
//! `docs/recom-kh1-hud.md` § Re:CoM's HP gauge at KH1's place.
//!
//! The MP ring built as Re:CoM's HP ring (owner, 2026-11-01): its groups are the asset step's
//! `mp_ring` groups (KH1's quarter pictures whole, cut by scissor parts and turned, as Re:CoM's
//! `gauge:01` 7..9) when the manifest has them, and its controls hang from Re:CoM's HP control
//! `+0x78` (`0x140248650`, as the 7.2 MP bar and Re:CoM's Dark gauge), so whatever moves that
//! control (the hit shake) moves the MP ring with it. Re:CoM's room plate (`CNaviMap` `+0x128`)
//! goes to the top left at `hud_room_plate_top_left`. `docs/recom-kh1-hud.md` § The MP ring
//! built as Re:CoM's HP ring.

use super::movement::{func, orig, rd, wr};
use super::{base, log};
use crate::kh1_hud::{self as kh, group, mp_ring, program, recom_hp, Manifest, View};
use crate::settings::Mode;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();
/// The registry check logged for this story (1 Sora's, 2 Riku's; each logged once: they load
/// different HUD files, `FO0002` / `FO0071`).
static CHECKED: AtomicU32 = AtomicU32::new(0);
/// The C2DSeqCtrl vtable's replace slot is Re:CoM Steam's (`sites::KH1_HUD_PLACE_PTRS`).
static PLACE_OK: AtomicBool = AtomicBool::new(false);
/// "gauge:01 is not ours" logged once.
static PLACE_LOGGED: AtomicBool = AtomicBool::new(false);
static HOOKED: AtomicBool = AtomicBool::new(false);
static LOG_LEFT: AtomicU32 = AtomicU32::new(80);
pub static ORIG_HP: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_MAX_HP: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_DONALD: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_GOOFY: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_EXIT: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_DTOR: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_MARKER: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_POPUP: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_PLATE: AtomicUsize = AtomicUsize::new(0);
/// H6: the friend objects Re:CoM has out now (Donald, Goofy), 0 when not.
static FRIEND: [AtomicUsize; 2] = [AtomicUsize::new(0), AtomicUsize::new(0)];
static STATE: Mutex<State> = Mutex::new(State { gauges: Vec::new(), clusters: Vec::new() });

// Re:CoM functions (RVAs; first bytes checked in `sites`).
const NEW: u32 = 0x004A_CE54;
const SEQ_CTOR: u32 = 0x0024_7260;
const FRAME: u32 = 0x0024_7C20;
const ALL_VISIBLE: u32 = 0x0024_7EC0;
const LAYOUT_POS: u32 = 0x0024_8660;
const COLOUR: u32 = 0x0024_8590;
/// `C2DSeqCtrl` parent (`0x140248650`: `+0x130` = the parent's transform node; checked in
/// `sites::MP_BAR_CALLEES`).
const PARENT: u32 = 0x0024_8650;
const VAR_GET: u32 = 0x001B_08C0;
const SEQ_CTRL_VTABLE: u32 = 0x0060_7340;
/// H2b (`sites::KH1_HUD_CALLEES`): play a layer from a time (units of 1/50 frame), a layer's time.
const PLAY_FROM: u32 = 0x0024_8550;
const LAYER_TIME: u32 = 0x0024_8000;
/// Re:CoM's 2D time units a frame (`0x140245870`: frame × 50).
const UNITS: f32 = 50.0;
/// `C2DSeqCtrl` layer count.
const C_LAYERS: usize = 0x16D;

/// `CPlayerGauge` fields (Checked (decomp `0x14020D6A0`, `0x14020D8E0`, `0x140208360`)): HP and
/// max HP (u16, capped at 560 by Re:CoM), the x offset, the display state.
const G_HP: usize = 0xA4;
const G_MAX_HP: usize = 0xA8;
const G_XOFF: usize = 0xA0;
const G_STATE: usize = 0xAA;
const STATE_SHOW: u32 = 1;
const STATE_HIDDEN: u32 = 7;
/// `C2DSeqCtrl`: draw priority (a clear resets it to 0xFF).
const C_PRIORITY: usize = 0x16E;
/// `C2DSeqCtrl`: its position (x − 256, y − 192 as `0x140248660` writes it; Checked (decomp)).
const C_POS: usize = 0xF0;
/// The back at the HUD's priority (Re:CoM's HP gauge's 2), the HP ring and face over it.
const PRIORITY_BACK: u8 = 2;
const PRIORITY_FRONT: u8 = 3;

type NewFn = unsafe extern "win64" fn(usize) -> usize;
type CtorFn = unsafe extern "win64" fn(usize) -> usize;
type FrameFn = unsafe extern "win64" fn(usize, u32, i32);
type VisFn = unsafe extern "win64" fn(usize, i32);
type VecFn = unsafe extern "win64" fn(usize, *const f32);
type ParentFn = unsafe extern "win64" fn(usize, usize);
type HpFn = unsafe extern "win64" fn(usize, u32, i32);
type MaxFn = unsafe extern "win64" fn(usize, u32);
type GetFn = unsafe extern "win64" fn(u32, u32) -> u32;
type PlayFn = unsafe extern "win64" fn(usize, u32, f32);
type TimeFn = unsafe extern "win64" fn(usize, u32) -> u64;

/// `CPlayerGauge` children (Checked (decomp `0x140208360`)): CFriendGauge, CPrizeGauge; the HP
/// control `+0x78`.
const G_FRIEND: usize = 0x60;
const G_PRIZE: usize = 0x70;
const G_HP_CTRL: usize = 0x78;
/// `C2DSeqCtrl` "use the colour" (`0x14024BB10`, with the parent's).
const C_COLOUR_ON: usize = 0x138;
/// The controls the colour hides: (child field of the gauge, its array field, count). Counts as
/// Re:CoM makes them (`_eh_vector_constructor_iterator_` in `0x140207F30` and `0x1402087C0`,
/// checked against the count word before each array); 0 = one object (not an array).
const HIDDEN_ARRAYS: [(usize, usize, usize); 5] = [(G_FRIEND, 0x78, 3), (G_PRIZE, 0x70, 6), (G_PRIZE, 0x88, 6), (G_PRIZE, 0xA0, 2), (G_PRIZE, 0xA8, 0)];
/// The mask bits the enemy gauge (`0x1402099B0`) and the minimap (`0x1401E4700`, `0x1401E43E0`)
/// hide on. Checked (decomp).
const MASK_ENEMY_GAUGE: u32 = 0x10;
/// KH1's lock-on visuals (H5 LOCK ON! + target gauge, H7 cursor) instead of Re:CoM's cursor and
/// enemy HP gauges. Off: the owner's choice, 2026-11-01 ("lets just use Re:CoM's lock on
/// graphics", "lets have Re:CoM's health bar", and the LOCK ON! plate removed too).
const LOCKON_KH1: bool = false;
/// KH1's LEVEL UP!! box (H8) instead of Re:CoM's popup. Off: "I want the ReCoM level up message
/// instead of the KH1 level up hud" (owner, 2026-11-01).
const LEVEL_UP_KH1: bool = false;
const MASK_MINIMAP: u32 = 8;
const C_SIZE: usize = 0x170;

#[repr(C, align(16))]
struct V4([f32; 4]);

/// The held layers: (control 0 back / 1 front, layer index) of each of `kh::frames`' groups.
const LAYERS: [(usize, u32); 7] = [(0, 0), (0, 1), (0, 2), (0, 3), (1, 0), (1, 1), (1, 2)];

/// A transient piece (a played group on a pooled control).
#[derive(Clone, Copy)]
struct Piece {
    ctrl: usize,
}

/// A running motion (H2b: KH1's slot `+0x110` motion, or KH1's gauge fade): the held layers are
/// hidden and the units of what they show play the program's groups instead.
struct Running {
    prog: usize,
    /// The class the units are placed at (a slide moves from the old class).
    class: usize,
    /// Controls holding the unit layers (10 a control at most). The face is a unit too for slides
    /// and fades; a shake moves the hurt face, its own group.
    ctrls: Vec<usize>,
}

struct Cluster {
    gauge: usize,
    list: i32,
    xoff: f32,
    class: usize,
    ctrls: [usize; 4],
    view: View,
    danger: bool,
    hidden: bool,
    mp: Vec<Piece>,
    charge: Vec<Piece>,
    red: Vec<Piece>,
    pool: Vec<usize>,
    /// The held layers' frames as set (FRAME … TICKS, the `kh::frames` order).
    shown: [i32; 7],
    motion: Option<Running>,
    /// H6: Donald's and Goofy's gauge controls (0 without the data).
    party: [usize; 2],
    /// Only KH1's MP ring (`hud_kh1_mp_ring`, around Re:CoM's HP ring): controls 0 (MP back, MP)
    /// and 1 (charge, ticks), no HP, face, flash or motions (`crate::kh1_hud::mp_ring`).
    mp_only: bool,
    /// The MP ring's groups built as Re:CoM's HP ring (the manifest's `mp_ring` first group).
    ring: Option<u32>,
    /// Re:CoM's HP control our controls hang from (0: none) and its position when they were
    /// placed (`+0xF0`), which their own positions are relative to.
    parent: usize,
    rel: (f32, f32),
}

struct State {
    /// Live player gauges (init seen, dtor not yet) and their draw lists.
    gauges: Vec<(usize, i32)>,
    clusters: Vec<Cluster>,
}

fn note(line: impl FnOnce() -> String) {
    if LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("kh1 hud: {}", line()));
    }
}

/// The manifest, once installed.
pub fn manifest() -> Option<&'static Manifest> {
    MANIFEST.get()
}

/// The setting: `hud = kh1` (and a manifest).
pub fn wanted() -> bool {
    manifest().is_some() && super::settings().hud == Mode::Kh1
}

/// KH1's HUD is drawn here: `hud = kh1`, the set registered, Sora's story, **Combat = KH1** (owner,
/// 2026-11-04: "with combat set to RE:CoM the hud should also go back to the normal positioning": at
/// Combat = Re:CoM every piece, its placements and hides included, is Re:CoM's; a switch takes
/// effect where each piece is set up: the next room or battle for the gauges and the room plate).
/// Riku's story too with `riku_kh1_movement` (0.149.0, fork owner 2026-11-22: "a HUD ta com o mesmo
/// problema que está com a do Sora … o fix pode ser o mesmo"): the asset step gives his `FO0071`
/// the same set and turned ring; every piece checks Re:CoM's registry before drawing, so a served
/// `FO0071` without them keeps Re:CoM's HUD.
pub fn active() -> bool {
    wanted() && HOOKED.load(Ordering::Relaxed) && (!super::stats::riku_story() || super::movement::riku_on()) && super::movement::kh1_on()
}

/// The pieces chosen in the settings (`hud_kh1_*`, `hud_hide_*`).
fn parts() -> crate::settings::HudParts {
    super::settings().hud_parts
}

/// KH1's gauge cluster is drawn (`hud_kh1_gauge`): the MP bar and Re:CoM's HP gauge follow it.
pub fn gauge_on() -> bool {
    active() && parts().gauge
}

/// Re:CoM's HP gauge goes to KH1's place, turned (`hud_recom_hp_kh1_place`, with the KH1 gauge
/// off): the setting, the asset step's groups and the vtable slot. [`place_on`] also checks the
/// loaded set.
pub fn place_wanted() -> bool {
    let p = parts();
    active() && p.recom_hp_place && !p.gauge && PLACE_OK.load(Ordering::Relaxed) && manifest().is_some_and(|m| m.recom_hp.is_some())
}

/// [`place_wanted`], and Re:CoM registered `gauge:01` with the turned groups (the served
/// `FO0002.SPR` is this version's asset step's).
pub unsafe fn place_on() -> bool {
    place_wanted() && manifest().and_then(|m| m.recom_hp).is_some_and(|f| super::mp_bar::set_groups(recom_hp::SET) == Some(f + recom_hp::TURNED.len() as u32))
}

/// KH1's MP ring around Re:CoM's HP ring is drawn (`hud_kh1_mp_ring`, with the KH1 gauge off and
/// Re:CoM's gauge at KH1's place: a ring around nothing would make no sense): the 7.2 MP bar is not
/// made then.
pub fn ring_on() -> bool {
    parts().mp_ring && place_wanted()
}

/// KH1's command menu look is drawn (`hud_kh1_menu`).
pub fn menu_on() -> bool {
    active() && parts().menu
}

/// Re:CoM's pieces hidden now: bit 1 the HP gauge, 2 the FRIENDS row, 4 the EXP / Moogle
/// pop-ups (the prize gauge), 8 the minimap.
fn hidden_bits() -> u32 {
    if !active() {
        return 0;
    }
    let p = parts();
    (p.hide_hp as u32) | (p.hide_friends as u32) << 1 | (p.hide_prizes as u32) << 2 | (p.hide_minimap as u32) << 3
}

/// The minimap's hide bit is added (`hud_hide_minimap`), as last seen on an event.
static MINIMAP_UP: AtomicBool = AtomicBool::new(false);

/// Reads `kh1-hud.json` and hooks Re:CoM's HP setters (needs the MP bar's gauge wrappers,
/// installed before).
pub fn install(b: usize, manifest: Option<&std::path::Path>) -> bool {
    let Some(path) = manifest else {
        log("kh1 hud: no kh1-hud.json (asset step not run?): Re:CoM's HUD only");
        return false;
    };
    let m = match std::fs::read_to_string(path).map_err(|e| e.to_string()).and_then(|t| kh::parse_manifest(&t)) {
        Ok(m) => m,
        Err(e) => {
            log(&format!("kh1 hud: {}: {e}; Re:CoM's HUD only", path.display()));
            return false;
        }
    };
    log(&format!(
        "kh1 hud: kh1hud:00 ({} groups, PC copies {:?}) from {}; hud = {} in kh1-recom.ini",
        m.groups,
        m.languages,
        path.display(),
        super::settings().hud.as_str()
    ));
    let _ = MANIFEST.set(m);
    let site = |n: &str| crate::sites::KH1_HUD_HOOKS.iter().find(|h| h.name == n).expect("site");
    let hooks = [
        super::hook::Hook { site: site("CPlayerGauge HP (0x14020D6A0)"), ours: gauge_hp as *const () as usize, original: &ORIG_HP },
        super::hook::Hook { site: site("CPlayerGauge max HP (0x14020D8E0)"), ours: gauge_max_hp as *const () as usize, original: &ORIG_MAX_HP },
    ];
    // SAFETY: the targets' bytes are checked again inside, before anything is written.
    if unsafe { super::hook::install_all(b, &hooks) } {
        HOOKED.store(true, Ordering::Relaxed);
        log("kh1 hud: Sora's KH1 gauge at hud = kh1 (H2): Re:CoM's HP setters 0x14020D6A0 / 0x14020D8E0 wrapped; frames on HP / MP events only");
        // SAFETY: executable memory of our own near the exe; the call sites' bytes were checked
        // at start and are checked again before the write.
        unsafe { install_masks(b) };
        // Re:CoM's HP gauge at KH1's place calls Re:CoM's layer replace through the vtable.
        // SAFETY: reads of the mapped image (the build was checked at start).
        let slots_ok = crate::sites::KH1_HUD_PLACE_PTRS.iter().all(|p| unsafe { rd::<usize>(b + p.rva as usize) } == b + p.target as usize);
        PLACE_OK.store(slots_ok, Ordering::Relaxed);
        if !slots_ok {
            log("kh1 hud: C2DSeqCtrl's replace slot is not Re:CoM Steam's: Re:CoM's HP gauge stays top left");
        } else if MANIFEST.get().is_some_and(|m| m.recom_hp.is_none()) {
            log("kh1 hud: kh1-hud.json has no recom_hp (an older asset step): Re:CoM's HP gauge stays top left; run this version's asset step for hud_recom_hp_kh1_place");
        }
        // H7 (KH1's lock-on cursor) is off: the owner chose Re:CoM's lock-on visuals (cursor,
        // enemy HP gauges, no LOCK ON! plate), 2026-11-01. The wrapper stays in the code.
        if LOCKON_KH1 && MANIFEST.get().is_some_and(|m| m.cursor.is_some()) {
            let site = crate::sites::KH1_HUD_CURSOR_HOOKS.iter().find(|h| h.name == "CLockonMarker update (0x140209D30)").expect("site");
            let h = [super::hook::Hook { site, ours: marker_update as *const () as usize, original: &ORIG_MARKER }];
            // SAFETY: as above.
            if unsafe { super::hook::install_all(b, &h) } {
                log("kh1 hud: KH1's lock-on cursor at hud = kh1 (H7): CLockonMarker update 0x140209D30 wrapped");
            } else {
                log("kh1 hud: lock-on marker not hooked; Re:CoM's cursor stays");
            }
        }
        // Re:CoM's room plate top left (`hud_room_plate_top_left`).
        let h = [super::hook::Hook { site: &crate::sites::KH1_HUD_PLATE_HOOKS[0], ours: room_plate as *const () as usize, original: &ORIG_PLATE }];
        // SAFETY: as above.
        if unsafe { super::hook::install_all(b, &h) } {
            log("kh1 hud: CNaviMap room plate 0x1401E44A0 wrapped (the plate top left at hud_room_plate_top_left)");
        } else {
            log("kh1 hud: room plate not hooked; Re:CoM's plate stays bottom right");
        }
        // H8 (KH1's LEVEL UP!!) is off: the owner chose Re:CoM's level-up message, 2026-11-01.
        if LEVEL_UP_KH1 && MANIFEST.get().is_some_and(|m| m.level_up.is_some()) {
            let site = &crate::sites::KH1_HUD_POPUP_HOOKS[0];
            let h = [super::hook::Hook { site, ours: popup as *const () as usize, original: &ORIG_POPUP }];
            // SAFETY: as above.
            if unsafe { super::hook::install_all(b, &h) } {
                log("kh1 hud: KH1's LEVEL UP!! at hud = kh1 (H8): CCommonGauge popup 0x14020C9A0 wrapped");
            } else {
                log("kh1 hud: popups not hooked; Re:CoM's LEVEL UP! stays");
            }
        }
        if MANIFEST.get().is_some_and(|m| m.party.is_some()) {
            let ps = |n: &str| crate::sites::KH1_HUD_PARTY_HOOKS.iter().find(|h| h.name == n).expect("site");
            let party = [
                super::hook::Hook { site: ps("CFriendDonald appear (0x1401DDC20)"), ours: donald_appear as *const () as usize, original: &ORIG_DONALD },
                super::hook::Hook { site: ps("CFriendGoofy appear (0x1401DF9C0)"), ours: goofy_appear as *const () as usize, original: &ORIG_GOOFY },
                super::hook::Hook { site: ps("CFriend exit (0x1401D94D0)"), ours: friend_exit as *const () as usize, original: &ORIG_EXIT },
                super::hook::Hook { site: ps("CFriend dtor (0x1401D90F0)"), ours: friend_dtor as *const () as usize, original: &ORIG_DTOR },
            ];
            // SAFETY: as above.
            if unsafe { super::hook::install_all(b, &party) } {
                log("kh1 hud: Donald's and Goofy's KH1 gauges while Re:CoM has them out (H6): CFriendDonald / CFriendGoofy appear, CFriend exit and dtor wrapped");
            } else {
                log("kh1 hud: friend functions not hooked; no party gauges");
            }
        }
    } else {
        log("kh1 hud: HP setters not hooked; no KH1 gauge");
    }
    true
}

/// The draw list a live player gauge was made on (Riku's costume face, `win::riku_skin`).
pub fn gauge_list(gauge: usize) -> Option<i32> {
    STATE.lock().ok()?.gauges.iter().find(|g| g.0 == gauge).map(|g| g.1)
}

/// A player gauge was made (its init has run; Re:CoM has loaded FO0002): checks once that
/// Re:CoM registered our set with the manifest's group count.
pub unsafe fn gauge_made(gauge: usize, list: i32) {
    let Some(m) = manifest() else { return };
    if let Ok(mut s) = STATE.lock() {
        s.gauges.retain(|g| g.0 != gauge);
        s.gauges.push((gauge, list));
    }
    // A fresh init is Re:CoM's layout (a gauge at a reused address is not still "placed").
    if let Ok(mut p) = PLACED.lock() {
        p.retain(|e| e.0 != gauge);
    }
    place_recom_hp(gauge, list);
    build(gauge, list);
    UP.store(active(), Ordering::Relaxed);
    let bits = hidden_bits();
    MINIMAP_UP.store(bits & 8 != 0, Ordering::Relaxed);
    recom_hud(gauge, bits);
    let (bit, file) = if super::stats::riku_story() { (2, "FORM/0071/FO0071") } else { (1, "FORM/0002/FO0002") };
    if CHECKED.fetch_or(bit, Ordering::Relaxed) & bit != 0 {
        return;
    }
    match super::mp_bar::set_groups(kh::SET) {
        Some(n) if n == m.groups => log(&format!(
            "kh1 hud: Re:CoM registered kh1hud:00 with {n} groups ({file}'s kh1hud.lad / .imz are served); hud = {}",
            if wanted() { "kh1" } else { "recom" }
        )),
        Some(n) => log(&format!("kh1 hud: kh1hud:00 has {n} groups, not {} (an older {file}.SPR is served?)", m.groups)),
        None => log(&format!("kh1 hud: kh1hud:00 is not in Re:CoM's 2D registry (Panacea serves Re:CoM's own {file}.SPR / .RTM: run this version's asset step)")),
    }
}

// -------------------------------------------------------------------------------------------------
// H2: the cluster.

/// Our set is registered with the manifest's group count (checked at each build: Panacea may
/// serve another file after a mod change).
unsafe fn set_ready() -> bool {
    manifest().is_some_and(|m| super::mp_bar::set_groups(kh::SET) == Some(m.groups))
}

/// [`set_ready`], for the command menu's shortcut panel (stage 7.12), which draws `kh1hud` groups
/// at either HUD setting. A lookup in Re:CoM's 2D registry: called when the panel is made.
pub fn set_registered() -> bool {
    // SAFETY: a read of Re:CoM's registry, as at each gauge build.
    unsafe { set_ready() }
}

unsafe fn new_ctrl() -> Option<usize> {
    let c = func::<NewFn>(NEW)(0x170);
    if c == 0 {
        return None;
    }
    func::<CtorFn>(SEQ_CTOR)(c);
    if rd::<usize>(c) != base() + SEQ_CTRL_VTABLE as usize {
        return None;
    }
    super::d2::made();
    Some(c)
}

unsafe fn place(c: usize, class: usize, dx: f32) {
    let (x, y) = kh::kh1::CLASS_OFFSET[class];
    func::<VecFn>(LAYOUT_POS)(c, V4([x + dx, y, 0.0, 1.0]).0.as_ptr());
}

unsafe fn add(c: usize, list: i32, prio: u8, g: u32, xoff: f32, flag: u32) {
    wr::<u8>(c + C_PRIORITY, prio);
    super::d2::add(c, list, kh::SET.as_ptr(), g, xoff, flag);
}

fn class_of(v: &View) -> usize {
    kh::size_class(v.max_hp, v.mp.map_or(0, |m| m.1))
}

impl Cluster {
    /// The group that shows what the cluster's group `g` shows: the MP ring's own when built.
    fn grp(&self, g: u32) -> u32 {
        match self.ring {
            Some(first) if self.mp_only => mp_ring::ring_group(first, g),
            _ => g,
        }
    }

    /// Places control `c` at size class `class` + `dx` (KH1's absolute places): under Re:CoM's
    /// HP control when the cluster hangs from it (its position relative to that control's), else
    /// on its own.
    unsafe fn put(&self, c: usize, class: usize, dx: f32) {
        if self.parent == 0 {
            place(c, class, dx);
            return;
        }
        func::<ParentFn>(PARENT)(c, self.parent);
        let (x, y) = kh::kh1::CLASS_OFFSET[class];
        // `0x140248660` stores (x − 256, y − 192); the parent's own position is added when drawn.
        func::<VecFn>(LAYOUT_POS)(c, V4([x + dx - self.rel.0, y - self.rel.1, 0.0, 1.0]).0.as_ptr());
    }

    /// The control and layer of `kh::frames`' entry `k` (None: not drawn by this cluster).
    fn layer_of(&self, k: usize) -> Option<(usize, u32)> {
        if self.mp_only {
            mp_ring::layer_of(k)
        } else {
            Some(LAYERS[k])
        }
    }

    /// The MP fades' priority (with the MP layer).
    fn prio_mp(&self) -> u8 {
        if self.mp_only {
            mp_ring::PRIORITY_MP
        } else {
            PRIORITY_BACK
        }
    }

    /// The charge fades' priority (with the charge layer).
    fn prio_charge(&self) -> u8 {
        if self.mp_only {
            mp_ring::PRIORITY_CHARGE
        } else {
            PRIORITY_FRONT
        }
    }

    unsafe fn release(&mut self, which: u8) {
        let v = match which {
            0 => &mut self.mp,
            1 => &mut self.charge,
            _ => &mut self.red,
        };
        for p in v.drain(..) {
            super::d2::clear(p.ctrl, self.list);
            self.pool.push(p.ctrl);
        }
    }

    /// A played group on a pooled control, at the class offset + `dx`.
    unsafe fn piece(&mut self, g: u32, prio: u8, dx: f32) -> Option<Piece> {
        let c = match self.pool.pop() {
            Some(c) => c,
            None => new_ctrl()?,
        };
        self.put(c, self.class, dx);
        add(c, self.list, prio, g, self.xoff, 0);
        Some(Piece { ctrl: c })
    }

    unsafe fn red_on(&mut self) {
        self.release(2);
        for (g, dx) in kh::red_pieces(self.view.max_hp) {
            if let Some(p) = self.piece(g, PRIORITY_BACK, dx) {
                self.red.push(p);
            }
        }
    }

    unsafe fn face(&mut self, hurt: Option<usize>) {
        let c = self.ctrls[2];
        super::d2::clear(c, self.list);
        match hurt {
            Some(s) => {
                // Played from frame 1: the shake baked in it starts at KH1's key time 0.5.
                add(c, self.list, PRIORITY_FRONT, kh::hurt_group(s, self.danger), self.xoff, 0);
                func::<PlayFn>(PLAY_FROM)(c, 0, UNITS);
            }
            None => {
                add(c, self.list, PRIORITY_FRONT, group::FACE, self.xoff, 1);
                func::<FrameFn>(FRAME)(c, 0, kh::face_frame(self.danger));
            }
        }
    }

    /// Shows `v`: the held frames that changed, and KH1's fades, hurt face and danger flash for the
    /// change from what was shown (`first`: no fades).
    unsafe fn show(&mut self, v: View, first: bool) {
        let old = self.view;
        let class = if self.mp_only { mp_ring::CLASS } else { class_of(&v) };
        let old_class = self.class;
        let class_changed = class != self.class && !first;
        if class != self.class {
            self.class = class;
            for &c in self.ctrls.iter().filter(|&&c| c != 0) {
                self.put(c, class, 0.0);
            }
            for p in self.mp.iter().chain(&self.charge) {
                self.put(p.ctrl, class, 0.0);
            }
            note(|| format!("size class {class} (max HP {}, max MP {})", v.max_hp, v.mp.map_or(0, |m| m.1)));
        }
        let (fo, fv) = (kh::frames(&old), kh::frames(&v));
        // The charge's main frame follows the fade plan.
        let (oc, nc) = (old.mp.map_or(0, |m| m.2 as i32), v.mp.map_or(0, |m| m.2 as i32));
        let (charge_main, charge_pieces) = if first || old.mp.is_none() || v.mp.is_none() || self.hidden { (fv[5].1, Vec::new()) } else { kh::charge_plan(oc, nc) };
        for (k, ((g, f), (_, f0))) in fv.iter().zip(fo.iter()).enumerate() {
            let f = if *g == group::CHARGE { charge_main } else { *f };
            if first || f != *f0 || (*g == group::CHARGE && oc != nc) {
                let Some((ci, layer)) = self.layer_of(k) else { continue };
                func::<FrameFn>(FRAME)(self.ctrls[ci], layer, f);
                self.shown[k] = f;
            }
        }
        if !first && !self.hidden {
            if let (Some(a), Some(b)) = (old.mp, v.mp) {
                if a.0 != b.0 {
                    self.release(0);
                    for g in kh::mp_fade(a.0, b.0) {
                        if let Some(p) = self.piece(self.grp(g), self.prio_mp(), 0.0) {
                            self.mp.push(p);
                        }
                    }
                }
            }
            if oc != nc {
                self.release(1);
                for g in charge_pieces {
                    if let Some(p) = self.piece(self.grp(g), self.prio_charge(), 0.0) {
                        self.charge.push(p);
                    }
                }
            }
        }
        if self.mp_only {
            // KH1's MP ring only: Re:CoM's own gauge shows HP, the face and low HP.
            self.view = v;
            return;
        }
        let d = kh::danger(v.hp, v.max_hp);
        let hit = if first { None } else { kh::hurt_strength(old.hp, v.hp) };
        let danger_changed = d != self.danger;
        self.danger = d;
        self.view = v;
        if !self.hidden {
            if d && (danger_changed || old.max_hp != v.max_hp || first) {
                self.red_on();
            } else if !d && danger_changed {
                self.release(2);
            }
        }
        // KH1's motion pointer (slot `+0x110`, `0x14026D570` / `0x140268D60`): a size-class change
        // starts its slide (over a running shake); a hit starts its shake, and the hurt face, only
        // while no motion runs.
        let mut hurt = None;
        let mut started = false;
        if !self.hidden && class_changed {
            started = self.start(kh::slide_program(class), old_class);
            if !started {
                note(|| format!("no slide units for this view (class {old_class} -> {class}): moved at once"));
            }
        } else if let Some(s) = hit {
            if !self.hidden && self.motion_idle() {
                started = self.start(program::VIBE + s.min(2), class);
                hurt = Some(s);
            }
        }
        if first || hurt.is_some() || danger_changed {
            self.face(if self.hidden { None } else { hurt });
        }
        if !started {
            self.follow();
        }
        if danger_changed && !first {
            note(|| format!("danger {} (HP {}/{})", if d { "on: max-HP flash" } else { "off" }, v.hp, v.max_hp));
        }
    }

    /// The motion's elapsed frames (its first unit layer's time), None without one.
    unsafe fn elapsed(&self) -> Option<i32> {
        let r = self.motion.as_ref()?;
        let p = manifest()?.motion.programs.get(r.prog)?;
        let c = *r.ctrls.first()?;
        let t = func::<TimeFn>(LAYER_TIME)(c, 0) as u32 as i32;
        Some(t / UNITS as i32 - p.start)
    }

    /// No motion runs (none, or it is over).
    unsafe fn motion_idle(&self) -> bool {
        match (self.motion.as_ref(), self.elapsed()) {
            (None, _) | (_, None) => true,
            (Some(r), Some(e)) => manifest().and_then(|m| m.motion.programs.get(r.prog)).is_none_or(|p| e >= p.frames),
        }
    }

    /// The held controls a program replaces: back and front, and the face for slides and fades.
    unsafe fn held_visible(&self, on: bool, face: bool) {
        for &c in self.ctrls[..if face { 3 } else { 2 }].iter().filter(|&&c| c != 0) {
            func::<VisFn>(ALL_VISIBLE)(c, on as i32);
        }
    }

    /// Releases the motion's unit layers (their controls go back to the pool).
    unsafe fn drop_units(&mut self) -> Option<Running> {
        let r = self.motion.take()?;
        for &c in &r.ctrls {
            super::d2::clear(c, self.list);
            self.pool.push(c);
        }
        Some(r)
    }

    /// Makes the units of what the held layers show for program `prog` at `class`, played from
    /// the program's start + `at` frames. False when a unit lacks the program (the slides' bound).
    unsafe fn units(&mut self, prog: usize, class: usize, at: i32) -> bool {
        let Some(m) = manifest() else { return false };
        let Some(p) = m.motion.programs.get(prog).copied() else { return false };
        let face = prog >= program::FADE_OUT;
        // Held groups in the held layers' order, with their frames, and their priorities.
        let mut want: Vec<(u32, u8)> = Vec::new();
        for (k, (g, _)) in kh::frames(&self.view).iter().enumerate() {
            let prio = if LAYERS[k].0 == 0 { PRIORITY_BACK } else { PRIORITY_FRONT };
            for u in m.motion.units_at(*g, self.shown[k]) {
                match m.motion.group(u, prog) {
                    Some(gr) => want.push((gr, prio)),
                    None => return false,
                }
            }
        }
        if face {
            for u in m.motion.units_at(group::FACE, kh::face_frame(self.danger)) {
                match m.motion.group(u, prog) {
                    Some(gr) => want.push((gr, PRIORITY_FRONT)),
                    None => return false,
                }
            }
        }
        let mut r = Running { prog, class, ctrls: Vec::new() };
        let time = (p.start + at) as f32 * UNITS;
        for chunk in want.chunks(10) {
            let c = match self.pool.pop() {
                Some(c) => c,
                None => match new_ctrl() {
                    Some(c) => c,
                    None => break,
                },
            };
            place(c, class, 0.0);
            for &(gr, prio) in chunk {
                add(c, self.list, prio, gr, self.xoff, 0);
                let n = rd::<u8>(c + C_LAYERS) as u32;
                if n > 0 {
                    func::<PlayFn>(PLAY_FROM)(c, n - 1, time);
                }
            }
            r.ctrls.push(c);
        }
        self.motion = Some(r);
        true
    }

    /// Starts program `prog` with its units at `class`: the held layers hide while it runs.
    unsafe fn start(&mut self, prog: usize, class: usize) -> bool {
        self.drop_units();
        let face = prog >= program::FADE_OUT;
        if !self.units(prog, class, 0) {
            self.drop_units();
            if !self.hidden {
                self.held_visible(true, true);
            }
            return false;
        }
        self.held_visible(false, face);
        if !face {
            func::<VisFn>(ALL_VISIBLE)(self.ctrls[2], (!self.hidden) as i32);
        }
        true
    }

    /// After a view change: a running motion's units follow the new view at the same time; an
    /// ended one gives the screen back to the held layers.
    unsafe fn follow(&mut self) {
        let Some((prog, class)) = self.motion.as_ref().map(|r| (r.prog, r.class)) else { return };
        let Some(e) = self.elapsed() else {
            self.finish();
            return;
        };
        let len = manifest().and_then(|m| m.motion.programs.get(prog)).map_or(0, |p| p.frames);
        if e >= len {
            self.finish();
        } else {
            self.drop_units();
            if !self.units(prog, class, e) {
                self.finish();
            }
        }
    }

    /// The motion is over: the held layers show again (unless the HUD is hidden).
    unsafe fn finish(&mut self) {
        self.drop_units();
        if !self.hidden {
            self.held_visible(true, true);
        }
    }

    /// H6: Donald's and Goofy's gauges (hidden until Re:CoM has them out).
    unsafe fn party_build(&mut self) {
        if !parts().party {
            return;
        }
        let Some(groups) = manifest().and_then(|m| m.party) else { return };
        for k in 0..2 {
            let Some(c) = new_ctrl() else { return };
            let (dx, dy) = kh::PARTY_OFFSET[k];
            func::<VecFn>(LAYOUT_POS)(c, V4([dx, dy, 0.0, 1.0]).0.as_ptr());
            add(c, self.list, PRIORITY_BACK, groups[k], self.xoff, 1);
            self.party[k] = c;
        }
        self.party_show();
    }

    /// Shows a party gauge while its friend is out and the HUD is up.
    unsafe fn party_show(&self) {
        for k in 0..2 {
            if self.party[k] != 0 {
                let on = !self.hidden && FRIEND[k].load(Ordering::Relaxed) != 0;
                func::<VisFn>(ALL_VISIBLE)(self.party[k], on as i32);
            }
        }
    }

    /// Re:CoM's HUD hides (state 7) or shows (1) the gauge: KH1's 10-frame gauge fade
    /// (`0x140268E30`: alpha ±0.1 a frame) on the units; at once when `now` (the build) or without
    /// motion data.
    unsafe fn hide(&mut self, hidden: bool, now: bool) {
        if hidden {
            self.release(0);
            self.release(1);
            self.release(2);
        }
        self.hidden = hidden;
        // The MP ring hides and shows at once, with Re:CoM's HP gauge (state 7 / 1).
        let faded = !now && !self.mp_only && self.start(if hidden { program::FADE_OUT } else { program::FADE_IN }, self.class);
        if !faded {
            self.drop_units();
            for &c in self.ctrls.iter().filter(|&&c| c != 0) {
                func::<VisFn>(ALL_VISIBLE)(c, (!hidden) as i32);
            }
        }
        if !hidden && self.danger {
            self.red_on();
        }
        self.party_show();
    }

    unsafe fn destroy(&mut self) {
        let units: Vec<usize> = self.motion.take().map(|r| r.ctrls).unwrap_or_default();
        for c in units {
            super::d2::delete(c, self.list);
        }
        for p in self.mp.drain(..).chain(self.charge.drain(..)).chain(self.red.drain(..)) {
            super::d2::delete(p.ctrl, self.list);
        }
        for c in self.pool.drain(..).chain(self.ctrls).chain(self.party.into_iter().filter(|&c| c != 0)) {
            super::d2::delete(c, self.list);
        }
    }
}

/// The gauge's view: Re:CoM's HP and max HP, KH1's MP (read first: `mp::current` ends in
/// [`mp_seen`], which takes the lock).
unsafe fn view_of(gauge: usize) -> View {
    let mp = super::mp::current();
    View { hp: kh::hp_units(rd::<u16>(gauge + G_HP) as i32), max_hp: kh::hp_units(rd::<u16>(gauge + G_MAX_HP) as i32), mp }
}

/// Builds the cluster on `gauge` (its init has run, on draw list `list`): KH1's whole gauge
/// (`hud_kh1_gauge`), else KH1's MP ring around Re:CoM's HP ring (`hud_kh1_mp_ring`).
unsafe fn build(gauge: usize, list: i32) {
    if !gauge_on() {
        if ring_on() && set_ready() {
            build_ring(gauge, list);
        }
        return;
    }
    if !set_ready() {
        return;
    }
    let v = view_of(gauge);
    let xoff = kh::right_margin(rd::<f32>(gauge + G_XOFF));
    let class = class_of(&v);
    // KH1's whole gauge: back, front, face (the fourth control is the MP ring's).
    let mut ctrls = [0usize; 4];
    for c in ctrls.iter_mut().take(3) {
        match new_ctrl() {
            Some(n) => *c = n,
            None => {
                for &m in ctrls.iter().filter(|&&m| m != 0) {
                    super::d2::delete(m, list);
                }
                return;
            }
        }
    }
    for &c in &ctrls[..3] {
        place(c, class, 0.0);
    }
    for (g, f) in kh::frames(&v).iter().take(4) {
        let _ = f;
        add(ctrls[0], list, PRIORITY_BACK, *g, xoff, 1);
    }
    for (g, _) in kh::frames(&v).iter().skip(4) {
        add(ctrls[1], list, PRIORITY_FRONT, *g, xoff, 1);
    }
    let mut cl = Cluster { gauge, list, xoff, class, ctrls, view: v, danger: false, hidden: false, mp: Vec::new(), charge: Vec::new(), red: Vec::new(), pool: Vec::new(), shown: [0; 7], motion: None, party: [0; 2], mp_only: false, ring: None, parent: 0, rel: (0.0, 0.0) };
    cl.party_build();
    cl.show(v, true);
    let hidden = rd::<u16>(gauge + G_STATE) as u32 == STATE_HIDDEN;
    if hidden {
        cl.hide(true, true);
    }
    note(|| format!("built on the player gauge {gauge:#x}: HP {}/{}, MP {:?}, class {class}, margin {xoff}{}; Re:CoM's HP gauge, FRIENDS row, prize gauge hidden (colour), enemy gauges and minimap by their mask bits", v.hp, v.max_hp, v.mp, if hidden { " (HUD hidden)" } else { "" }));
    if let Ok(mut s) = STATE.lock() {
        for mut old in s.clusters.extract_if(.., |c| c.gauge == gauge) {
            old.destroy();
        }
        s.clusters.push(cl);
    } else {
        cl.destroy();
    }
}

/// Builds KH1's MP ring on `gauge`: the cluster's MP back, MP, charge and tick groups (KH1's
/// absolute places, size class 2: centred on [`recom_hp::CENTRE`], where Re:CoM's HP ring goes),
/// at KH1's right margin, under Re:CoM's HP gauge (priorities 0 / 1 < 2). Frames, fades and the
/// HUD's hide / show are the cluster's (`Cluster::show`, `hide`), on KH1's MP reads and writes.
///
/// The MP back, MP, charge and their fades are the asset step's `mp_ring` groups when the
/// manifest has them (KH1's quarter pictures whole, cut by scissor parts and turned, as Re:CoM
/// builds its HP ring: the same sharpness; owner, 2026-11-01), else the cluster's baked ones. The
/// controls hang from Re:CoM's HP control `+0x78` (`0x140248650`; their colour stays their own,
/// `+0x138` = 0: nothing of Re:CoM's colours that control, H3's colour hide aside): Re:CoM moves
/// that control's transform when Sora is hit (the ring, the disc and the face move together in
/// the owner's video; none of their groups has a translation key: Checked (files)), and the MP
/// ring now moves with it. Assumption: the movement is the control's transform (the code that
/// writes it was not found in the decomp).
unsafe fn build_ring(gauge: usize, list: i32) {
    let v = view_of(gauge);
    let xoff = kh::right_margin(rd::<f32>(gauge + G_XOFF));
    let ring = manifest().and_then(|m| m.mp_ring);
    let hp: usize = rd(gauge + G_HP_CTRL);
    let parent = if hp != 0 && rd::<usize>(hp) == base() + SEQ_CTRL_VTABLE as usize { hp } else { 0 };
    let rel = if parent != 0 { (rd::<f32>(parent + C_POS), rd::<f32>(parent + C_POS + 4)) } else { (0.0, 0.0) };
    let mut cl = Cluster { gauge, list, xoff, class: mp_ring::CLASS, ctrls: [0; 4], view: v, danger: false, hidden: false, mp: Vec::new(), charge: Vec::new(), red: Vec::new(), pool: Vec::new(), shown: [0; 7], motion: None, party: [0; 2], mp_only: true, ring, parent, rel };
    for k in 0..mp_ring::CONTROLS {
        match new_ctrl() {
            Some(n) => cl.ctrls[k] = n,
            None => {
                for &m in cl.ctrls.iter().filter(|&&m| m != 0) {
                    super::d2::delete(m, list);
                }
                return;
            }
        }
    }
    for k in 0..mp_ring::CONTROLS {
        cl.put(cl.ctrls[k], mp_ring::CLASS, 0.0);
    }
    let frames = kh::frames(&v);
    for (k, (g, _)) in frames.iter().enumerate() {
        if let Some((ci, _)) = mp_ring::layer_of(k) {
            let prio = mp_ring::priority(ci);
            add(cl.ctrls[ci], list, prio, cl.grp(*g), xoff, 1);
        }
    }
    cl.show(v, true);
    let hidden = rd::<u16>(gauge + G_STATE) as u32 == STATE_HIDDEN;
    if hidden {
        cl.hide(true, true);
    }
    note(|| {
        format!(
            "KH1's MP ring around Re:CoM's HP ring on the player gauge {gauge:#x}: MP {:?}, margin {xoff}{}; {}; {}",
            v.mp,
            if hidden { " (HUD hidden)" } else { "" },
            match ring {
                Some(f) => format!("built as Re:CoM's HP ring (groups {f}..{})", f + mp_ring::group::COUNT - 1),
                None => "the baked groups (kh1-hud.json has no mp_ring: run this version's asset step)".to_string(),
            },
            if parent != 0 { format!("hung from Re:CoM's HP control {parent:#x} at ({}, {})", rel.0, rel.1) } else { "not hung (no HP control)".to_string() }
        )
    });
    if let Ok(mut s) = STATE.lock() {
        for mut old in s.clusters.extract_if(.., |c| c.gauge == gauge) {
            old.destroy();
        }
        s.clusters.push(cl);
    } else {
        cl.destroy();
    }
}

/// Re:CoM's HP gauge at KH1's place, turned 180° (`hud_recom_hp_kh1_place`): right after Re:CoM's
/// init, its control `+0x78`'s layers 1..3 (`gauge:01` 7, 8, 9) are replaced with the asset step's
/// turned copies through Re:CoM's own replace (the same place in the draw list, the same
/// visibility; the frames Re:CoM's init gave them set again: trunc(max HP / 560 × 470)), and the
/// control's layout position (Re:CoM's `(60, 60)`) becomes KH1's centre at KH1's right margin.
/// Layer 0 (the disc) and 4 (the face) stay Re:CoM's, upright. From then on Re:CoM's own code sets
/// the frames (HP setters, the drain in its update), swaps the disc and face (hurt, low HP), hides
/// and shows the layers, and draws them; nothing of ours runs for it.
/// Gauges [`place_recom_hp`] turned, with Re:CoM's layout position to give back.
static PLACED: Mutex<Vec<(usize, f32, f32)>> = Mutex::new(Vec::new());
/// A control's layout position (`0x140248660`: x − 256 at `+0xF0`, y − 192 at `+0xF4`). Checked (decomp).
const C_LAYOUT_X: usize = 0xF0;

/// Owner, 2026-11-04 ("when switching combat from recom to KH1, everything changes except the HP bar, and vice
/// versa"): a turned gauge back to Re:CoM's: layers 1..3 Re:CoM's own groups 7, 8, 9 again through the same
/// replace, their frames as now (max HP; HP for the drain and HP layers), and Re:CoM's layout position.
unsafe fn unplace_recom_hp(gauge: usize, list: i32) {
    let Some(at) = PLACED.lock().ok().and_then(|p| p.iter().position(|e| e.0 == gauge)) else { return };
    let (_, x, y) = PLACED.lock().map(|mut p| p.remove(at)).unwrap_or((gauge, 60.0, 60.0));
    let hp: usize = rd(gauge + G_HP_CTRL);
    if hp == 0 || rd::<usize>(hp) != base() + SEQ_CTRL_VTABLE as usize {
        return;
    }
    let gx = rd::<f32>(gauge + G_XOFF);
    let fmax = recom_hp::frame(rd::<u16>(gauge + G_MAX_HP) as u32);
    let fh = recom_hp::frame(rd::<u16>(gauge + G_HP) as u32);
    for (k, &g) in recom_hp::TURNED.iter().enumerate() {
        let layer = recom_hp::FIRST_LAYER + k as u32;
        if !super::d2::replace(hp, list, layer, recom_hp::SET.as_ptr(), g, -gx, 1) {
            note(|| format!("Re:CoM's HP gauge {gauge:#x}: group {g} not made back; left as it is"));
            return;
        }
        func::<FrameFn>(FRAME)(hp, layer, if k == 0 { fmax } else { fh });
    }
    func::<VecFn>(LAYOUT_POS)(hp, V4([x, y, 0.0, 1.0]).0.as_ptr());
    note(|| format!("Re:CoM's HP gauge {gauge:#x} back at Re:CoM's place ({x}, {y}), upright"));
}

unsafe fn place_recom_hp(gauge: usize, list: i32) {
    place_recom_hp_at(gauge, list, false)
}

unsafe fn place_recom_hp_at(gauge: usize, list: i32, live: bool) {
    if !place_wanted() {
        return;
    }
    let Some(first) = manifest().and_then(|m| m.recom_hp) else { return };
    let want = first + recom_hp::TURNED.len() as u32;
    let have = super::mp_bar::set_groups(recom_hp::SET);
    if have != Some(want) {
        if !PLACE_LOGGED.swap(true, Ordering::Relaxed) {
            log(&format!("kh1 hud: gauge:01 has {} groups, not {want} (Panacea serves another FORM/0002/FO0002.SPR: run this version's asset step): Re:CoM's HP gauge stays top left", have.map_or("no".to_string(), |n| n.to_string())));
        }
        return;
    }
    let hp: usize = rd(gauge + G_HP_CTRL);
    if hp == 0 || rd::<usize>(hp) != base() + SEQ_CTRL_VTABLE as usize || (rd::<u8>(hp + C_LAYERS) as u32) < recom_hp::FIRST_LAYER + recom_hp::TURNED.len() as u32 {
        return;
    }
    if PLACED.lock().is_ok_and(|p| p.iter().any(|e| e.0 == gauge)) {
        return;
    }
    // Re:CoM's layout position (`0x140248660` keeps it at `+0xF0 / +0xF4` as x − 256, y − 192), for
    // [`unplace_recom_hp`].
    let back = (rd::<f32>(hp + C_LAYOUT_X) + 256.0, rd::<f32>(hp + C_LAYOUT_X + 4) + 192.0);
    let gx = rd::<f32>(gauge + G_XOFF);
    let f = recom_hp::frame(rd::<u16>(gauge + G_MAX_HP) as u32);
    for k in 0..recom_hp::TURNED.len() as u32 {
        let layer = recom_hp::FIRST_LAYER + k;
        if !super::d2::replace(hp, list, layer, recom_hp::SET.as_ptr(), first + k, -gx, 1) {
            // Put back what was turned: Re:CoM's gauge as it was.
            for j in 0..k {
                super::d2::replace(hp, list, recom_hp::FIRST_LAYER + j, recom_hp::SET.as_ptr(), recom_hp::TURNED[j as usize], -gx, 1);
                func::<FrameFn>(FRAME)(hp, recom_hp::FIRST_LAYER + j, f);
            }
            note(|| format!("gauge:01 group {} not made: Re:CoM's HP gauge stays top left", first + k));
            return;
        }
        func::<FrameFn>(FRAME)(hp, layer, f);
    }
    // At a live switch (Combat or the camp, [`refresh`]) the HP and drain layers show the HP now,
    // as Re:CoM's setters left them; at the init they are all the max (what Re:CoM's init just set).
    if live {
        let fh = recom_hp::frame(rd::<u16>(gauge + G_HP) as u32);
        for layer in recom_hp::FIRST_LAYER + 1..recom_hp::FIRST_LAYER + recom_hp::TURNED.len() as u32 {
            func::<FrameFn>(FRAME)(hp, layer, fh);
        }
    }
    let (x, y) = recom_hp::origin(gx);
    func::<VecFn>(LAYOUT_POS)(hp, V4([x, y, 0.0, 1.0]).0.as_ptr());
    if let Ok(mut p) = PLACED.lock() {
        p.push((gauge, back.0, back.1));
    }
    note(|| format!("Re:CoM's HP gauge {gauge:#x} at KH1's place, turned 180° (gauge:01 groups {first}..{} on layers 1..3, frame {f}), origin ({x}, {y}), Re:CoM's x offset {gx}", want - 1));
}

/// Re:CoM's gauge HP and max HP changed: show them.
unsafe fn hp_event(gauge: usize) {
    if !HOOKED.load(Ordering::Relaxed) {
        return;
    }
    let mp = super::mp::current();
    let Ok(mut s) = STATE.lock() else { return };
    if let Some(c) = s.clusters.iter_mut().find(|c| c.gauge == gauge) {
        let v = View { hp: kh::hp_units(rd::<u16>(gauge + G_HP) as i32), max_hp: kh::hp_units(rd::<u16>(gauge + G_MAX_HP) as i32), mp };
        if v != c.view {
            c.show(v, false);
        }
    }
}

/// `CPlayerGauge` HP setter (`0x14020D6A0(gauge, hp, reset)`): Re:CoM's, then ours.
pub unsafe extern "win64" fn gauge_hp(gauge: usize, hp: u32, reset: i32) {
    orig::<HpFn>(&ORIG_HP)(gauge, hp, reset);
    hp_event(gauge);
}

/// `CPlayerGauge` max-HP setter (`0x14020D8E0(gauge, max)`): Re:CoM's, then ours.
pub unsafe extern "win64" fn gauge_max_hp(gauge: usize, max: u32) {
    orig::<MaxFn>(&ORIG_MAX_HP)(gauge, max);
    hp_event(gauge);
}

/// KH1's MP as just read or written (`win/mp.rs`): one lock and a compare when unchanged.
pub fn mp_seen(view: (i32, i32, u16)) {
    let Ok(mut s) = STATE.lock() else { return };
    for c in s.clusters.iter_mut().filter(|c| c.view.mp != Some(view)) {
        let v = View { mp: Some(view), ..c.view };
        // SAFETY: a live cluster (removed in the gauge's dtor before Re:CoM frees anything).
        unsafe { c.show(v, false) };
    }
}

/// The gauge's HUD state (`0x14020C190`, after Re:CoM's): 7 hides ours, 1 shows it again.
pub unsafe fn hud(gauge: usize, state: u32) {
    if state != STATE_HIDDEN && state != STATE_SHOW {
        return;
    }
    let Ok(mut s) = STATE.lock() else { return };
    if let Some(c) = s.clusters.iter_mut().find(|c| c.gauge == gauge) {
        let hide = state == STATE_HIDDEN;
        if hide != c.hidden {
            c.hide(hide, false);
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Re:CoM's room plate at the top left (owner, 2026-11-01).

/// `Gauge::CNaviMap`'s room plate controls (`0x1401E44A0`: an array of two `C2DSeqCtrl` made by
/// the navi map's init `0x1401E39D0`, its count word before it; [0] the plate's art and the room
/// name, [1] the second layer some rooms add, which an extra object hangs from). Checked (decomp).
const NAVI_PLATES: usize = 0x128;
type PlateFn = unsafe extern "win64" fn(usize, usize, i32);

/// The room plate goes to the top left (`hud_room_plate_top_left`, with Re:CoM's HP gauge at KH1's
/// place: it would sit on the gauge otherwise).
pub fn plate_on() -> bool {
    parts().room_plate_top && place_wanted()
}

/// `CNaviMap` room plate set-up (`0x1401E44A0(navimap, set, group)`, when a room's plate appears
/// and settles): Re:CoM's, which puts its two controls at layout (0, 0) (the plate's place is in
/// its groups), then at `hud_room_plate_top_left` both controls at [`kh::room_plate::shift`] for the player
/// gauge's margin (`+0xA0`, Re:CoM's own left offset for its HUD). Re:CoM plays, hides and draws
/// the plate as before; once per plate set-up, nothing per frame.
pub unsafe extern "win64" fn room_plate(navi: usize, set: usize, group: i32) {
    orig::<PlateFn>(&ORIG_PLATE)(navi, set, group);
    if navi == 0 || !plate_on() {
        return;
    }
    let arr: usize = rd(navi + NAVI_PLATES);
    if arr == 0 || rd::<u64>(arr - 0x10) != 2 {
        return;
    }
    let Some(gauge) = STATE.lock().ok().and_then(|s| s.gauges.first().map(|g| g.0)) else { return };
    let m = rd::<f32>(gauge + G_XOFF);
    let (dx, dy) = kh::room_plate::shift(m);
    for k in 0..2 {
        let c = arr + k * C_SIZE;
        if rd::<usize>(c) == base() + SEQ_CTRL_VTABLE as usize {
            func::<VecFn>(LAYOUT_POS)(c, V4([dx, dy, 0.0, 1.0]).0.as_ptr());
        }
    }
    note(|| format!("room plate (group {group}) top left: moved ({dx}, {dy}) at margin {m}"));
}

// -------------------------------------------------------------------------------------------------
// H8: KH1's popups where Re:CoM has the event.

/// `CCommonGauge` (`[0x14087B6A8]`, H3's table): its seven popup controls `+0x60` (0x170 each).
const COMMON_GAUGE: usize = 0x0087_B6A8;
const CG_CTRLS: usize = 0x60;
type PopupFn = unsafe extern "win64" fn(i32, i32);

/// The LEVEL UP!! box we show (one at a time) and its list.
static LEVEL_BOX: Mutex<Option<(usize, i32)>> = Mutex::new(None);

/// `0x14020C9A0(actor, kind)`: Re:CoM's popup; at `hud = kh1` its level up (kind 4) is KH1's
/// LEVEL UP!! box instead (Re:CoM's control drawn at alpha 0, ours played from its keys' start).
pub unsafe extern "win64" fn popup(actor: i32, kind: i32) {
    orig::<PopupFn>(&ORIG_POPUP)(actor, kind);
    if kind != kh::POPUP_LEVEL_UP || !UP.load(Ordering::Relaxed) {
        return;
    }
    let Some(group) = manifest().and_then(|m| m.level_up) else { return };
    let cg: usize = rd(base() + COMMON_GAUGE);
    if cg != 0 {
        let arr: usize = rd(cg + CG_CTRLS);
        if arr != 0 {
            tint(arr + kind as usize * C_SIZE, 0.0);
        }
    }
    let host = STATE.lock().ok().and_then(|s| s.clusters.iter().find(|c| !c.hidden).map(|c| (c.list, c.xoff)));
    let Some((list, xoff)) = host else { return };
    let Ok(mut lb) = LEVEL_BOX.lock() else { return };
    let c = match lb.take() {
        Some((c, l)) => {
            super::d2::clear(c, l);
            c
        }
        None => match new_ctrl() {
            Some(c) => c,
            None => return,
        },
    };
    func::<VecFn>(LAYOUT_POS)(c, V4([kh::LEVEL_UP_AT.0, kh::LEVEL_UP_AT.1, 0.0, 1.0]).0.as_ptr());
    add(c, list, PRIORITY_FRONT, group, -xoff, 0);
    *lb = Some((c, list));
    note(|| "LEVEL UP!!: KH1's box (H8) for Re:CoM's level up".to_string());
}

// -------------------------------------------------------------------------------------------------
// H7: KH1's lock-on cursor in place of Re:CoM's.

/// `Gauge::CLockonMarker` (Checked (decomp `0x140208120`, `0x140209D30`)): its cursor control
/// `+0x60` (a `Cockpit:00` layer 0, group `+0x80`: 0x63, 0x92 or 0x62), the target handle
/// `+0xD0`, "target found" `+0xEC`, the lock point `+0x90` (16-byte aligned; the control's 3D
/// anchor, `0x140248570`), its distance scale `+0x104`.
const M_CTRL: usize = 0x60;
const M_TARGET: usize = 0xD0;
const M_FOUND: usize = 0xEC;
const M_POINT: usize = 0x90;
const LAYER_VISIBLE: u32 = 0x0024_8090;
const SLOT_VISIBLE: u32 = 0x0024_7E50;
const ANCHOR: u32 = 0x0024_8570;
const SEQ_CTOR_MODE: u32 = 0x0024_71D0;
type CtorModeFn = unsafe extern "win64" fn(usize, i32) -> usize;
type MarkerFn = unsafe extern "win64" fn(usize);
type IsVisFn = unsafe extern "win64" fn(usize, i32) -> i32;
type SlotVisFn = unsafe extern "win64" fn(usize, i32, i32);

struct Cursor {
    marker: usize,
    ctrl: usize,
    list: i32,
    target: i32,
}

static CURSOR: Mutex<Option<Cursor>> = Mutex::new(None);

/// `CLockonMarker` update (`0x140209D30(marker)`), each frame of a battle: Re:CoM's, then at
/// `hud = kh1` its cursor layer is hidden and KH1's (`lockon_c`, played from its keys' start when
/// the lock begins or moves to another target) is drawn on a control of ours at the same lock
/// point (Re:CoM projects it; KH1's cursor is not scaled with the distance as Re:CoM's is).
pub unsafe extern "win64" fn marker_update(m: usize) {
    orig::<MarkerFn>(&ORIG_MARKER)(m);
    let Some(group) = manifest().and_then(|x| x.cursor) else { return };
    let Ok(mut cur) = CURSOR.lock() else { return };
    let up = UP.load(Ordering::Relaxed) && STATE.lock().map(|s| s.clusters.iter().any(|c| !c.hidden)).unwrap_or(false);
    if cur.as_ref().is_some_and(|c| c.marker != m) || !up {
        if let Some(c) = cur.take() {
            super::d2::delete(c.ctrl, c.list);
        }
        if !up {
            return;
        }
    }
    let ctrl: usize = rd(m + M_CTRL);
    if ctrl == 0 {
        return;
    }
    // Re:CoM's cursor layer stays made (its logic runs) but is not drawn.
    if rd::<u8>(ctrl + C_LAYERS) > 0 && func::<IsVisFn>(LAYER_VISIBLE)(ctrl, 0) != 0 {
        func::<SlotVisFn>(SLOT_VISIBLE)(ctrl, 0, 0);
    }
    let target: i32 = rd(m + M_TARGET);
    let locked = target > 0 && rd::<i32>(m + M_FOUND) != 0;
    if cur.is_none() {
        // As Re:CoM makes its marker's control: `0x1402471D0(ctrl, 2)` (projected), priority 2.
        let c = func::<NewFn>(NEW)(0x170);
        if c == 0 {
            return;
        }
        func::<CtorModeFn>(SEQ_CTOR_MODE)(c, 2);
        if rd::<usize>(c) != base() + SEQ_CTRL_VTABLE as usize {
            return;
        }
        super::d2::made();
        *cur = Some(Cursor { marker: m, ctrl: c, list: super::d2::current_list(), target: 0 });
    }
    let Some(c) = cur.as_mut() else { return };
    if locked {
        func::<VecFn>(ANCHOR)(c.ctrl, (m + M_POINT) as *const f32);
        if c.target != target {
            super::d2::clear(c.ctrl, c.list);
            add(c.ctrl, c.list, PRIORITY_BACK, group, 0.0, 1);
            c.target = target;
            note(|| format!("lock-on cursor: KH1's lockon_c on target {target:#x} (H7)"));
        }
    } else if c.target != 0 {
        super::d2::clear(c.ctrl, c.list);
        c.target = 0;
    }
}

// -------------------------------------------------------------------------------------------------
// H6: Donald's and Goofy's gauges, while Re:CoM has them out as friends.

type FriendFn = unsafe extern "win64" fn(usize);
type FriendDtorFn = unsafe extern "win64" fn(usize, usize, usize, usize);

fn friend_changed(k: usize, obj: usize, out: bool) {
    let was = FRIEND[k].swap(if out { obj } else { 0 }, Ordering::Relaxed);
    if (was != 0) != out {
        note(|| format!("{} {}", ["Donald", "Goofy"][k], if out { "out: KH1 gauge shown (H6)" } else { "gone: gauge hidden" }));
        if let Ok(s) = STATE.lock() {
            for c in &s.clusters {
                // SAFETY: a live cluster.
                unsafe { c.party_show() };
            }
        }
    }
}

/// `CFriendDonald::beginStateAppear` (`0x1401DDC20(friend)`): Re:CoM's, then the gauge shows.
pub unsafe extern "win64" fn donald_appear(f: usize) {
    orig::<FriendFn>(&ORIG_DONALD)(f);
    friend_changed(0, f, true);
}

/// `CFriendGoofy::beginStateAppear` (`0x1401DF9C0(friend)`).
pub unsafe extern "win64" fn goofy_appear(f: usize) {
    orig::<FriendFn>(&ORIG_GOOFY)(f);
    friend_changed(1, f, true);
}

/// `CFriend::beginStateExit` (`0x1401D94D0(friend)`): the friend leaves; its gauge hides.
pub unsafe extern "win64" fn friend_exit(f: usize) {
    for k in 0..2 {
        if FRIEND[k].load(Ordering::Relaxed) == f {
            friend_changed(k, f, false);
        }
    }
    orig::<FriendFn>(&ORIG_EXIT)(f);
}

/// `CFriend`'s dtor (`0x1401D90F0`, run by every friend's): a friend that goes without its exit.
pub unsafe extern "win64" fn friend_dtor(f: usize, a: usize, b: usize, c: usize) {
    for k in 0..2 {
        if FRIEND[k].load(Ordering::Relaxed) == f {
            friend_changed(k, f, false);
        }
    }
    orig::<FriendDtorFn>(&ORIG_DTOR)(f, a, b, c);
}

// -------------------------------------------------------------------------------------------------
// H5: the LOCK ON! plate and the target's HP gauge (top left).

/// Re:CoM's enemy max-HP getter (`0x14017F5A0`; KH1's max for a proxied enemy through the stats
/// hook, `win/stats.rs`). First bytes checked (`sites::STATS` / the hook).
const ENEMY_MAX_HP: u32 = 0x0017_F5A0;
type MaxHpFn = unsafe extern "win64" fn(usize) -> i32;
type TypeFn = unsafe extern "win64" fn(usize) -> u32;

struct Target {
    gauge: usize,
    list: i32,
    plate: usize,
    /// Head, body, cap, bars (5 layers), marker.
    ctrls: [usize; 5],
    enemy: usize,
    hp: i32,
    max: i32,
    /// The gauge's alpha and whether it fades out; the plate's alpha (it fades out by 1/6).
    alpha: f32,
    out: bool,
    plate_alpha: f32,
}

static TARGET: Mutex<Option<Target>> = Mutex::new(None);

/// Re:CoM's locked enemy: in battle, its lock-on on (`*(player+0x3F8)+0xE0`), the target handle
/// `player+0x84C` an enemy (type 0x1004) with HP. As the movement code reads the lock
/// (`win/movement.rs` `lock_target`); Assumption: that pair is Re:CoM's lock-on.
unsafe fn locked_enemy(pl: usize) -> Option<usize> {
    use super::movement::p;
    if pl == 0 || rd::<i32>(pl + p::BATTLE) == 0 {
        return None;
    }
    let cur: usize = rd(pl + p::TARGET_CURSOR);
    if cur == 0 || rd::<i32>(cur + 0xE0) == 0 {
        return None;
    }
    let h: u32 = rd(pl + p::LOCK_TARGET);
    if h == 0 {
        return None;
    }
    let obj = func::<super::movement::HandleFn>(super::movement::HANDLE_OBJ)(h);
    if obj == 0 {
        return None;
    }
    let ty: TypeFn = std::mem::transmute_copy(&rd::<usize>(rd::<usize>(obj) + crate::battle::enemy::VT_TYPE));
    (ty(obj) == crate::battle::ENEMY_TYPE).then_some(obj)
}

impl Target {
    unsafe fn colours(&self) {
        func::<VecFn>(COLOUR)(self.plate, V4([1.0, 1.0, 1.0, self.plate_alpha]).0.as_ptr());
        for &c in &self.ctrls {
            func::<VecFn>(COLOUR)(c, V4([1.0, 1.0, 1.0, self.alpha]).0.as_ptr());
        }
    }

    /// Shows HP / max: the body's length, the cap's place, the bars, the marker.
    unsafe fn values(&mut self, hp: i32, max: i32) {
        let (ox, oy) = (kh::target::ORIGIN.0, kh::target::ORIGIN.1 + kh::target::BELOW_PLATE);
        let l = kh::target::body(max);
        func::<FrameFn>(FRAME)(self.ctrls[1], 0, l);
        func::<VecFn>(LAYOUT_POS)(self.ctrls[2], V4([ox + kh::target::BODY_X + l as f32, oy, 0.0, 1.0]).0.as_ptr());
        let bars = kh::target::bars(hp);
        for (k, len) in bars.iter().enumerate() {
            func::<FrameFn>(FRAME)(self.ctrls[3], k as u32, *len);
        }
        match kh::target::marker(&bars) {
            Some(k) => {
                func::<VecFn>(LAYOUT_POS)(self.ctrls[4], V4([ox + kh::target::BODY_X + bars[k] as f32, oy, 0.0, 1.0]).0.as_ptr());
                func::<VisFn>(ALL_VISIBLE)(self.ctrls[4], 1);
            }
            None => func::<VisFn>(ALL_VISIBLE)(self.ctrls[4], 0),
        }
        self.hp = hp;
        self.max = max;
    }

    unsafe fn destroy(&mut self) {
        super::d2::delete(self.plate, self.list);
        for c in self.ctrls {
            super::d2::delete(c, self.list);
        }
    }
}

/// Makes the plate and gauge on `cl`'s list for `enemy`.
unsafe fn target_new(gauge: usize, list: i32, xoff: f32, first: u32, enemy: usize, hp: i32, max: i32) -> Option<Target> {
    use kh::target as t;
    let xoff = -xoff;
    let mk = |x: f32, y: f32| -> Option<usize> {
        let c = new_ctrl()?;
        func::<VecFn>(LAYOUT_POS)(c, V4([x, y, 0.0, 1.0]).0.as_ptr());
        wr::<u8>(c + C_COLOUR_ON, 1);
        Some(c)
    };
    let (ox, oy) = t::ORIGIN;
    let gy = oy + t::BELOW_PLATE;
    let plate = mk(ox, oy)?;
    add(plate, list, PRIORITY_BACK, first + t::PLATE, xoff, 0);
    // KH1 restarts the plate's colour keys and plays them (drawn after the step: from frame 1).
    func::<PlayFn>(PLAY_FROM)(plate, 0, UNITS);
    let head = mk(ox, gy)?;
    add(head, list, PRIORITY_BACK, first + t::HEAD, xoff, 1);
    let body = mk(ox + t::BODY_X, gy)?;
    add(body, list, PRIORITY_BACK, first + t::BODY, xoff, 1);
    let cap = mk(ox, gy)?;
    add(cap, list, PRIORITY_BACK, first + t::CAP, xoff, 1);
    let bars = mk(ox + t::BODY_X, gy)?;
    for k in 0..5 {
        add(bars, list, PRIORITY_BACK, first + t::BAR + k, xoff, 1);
    }
    let marker = mk(ox, gy)?;
    add(marker, list, PRIORITY_BACK, first + t::MARKER, xoff, 1);
    let mut tg = Target { gauge, list, plate, ctrls: [head, body, cap, bars, marker], enemy, hp, max, alpha: 0.0, out: false, plate_alpha: 1.0 };
    tg.values(hp, max);
    tg.colours();
    Some(tg)
}

/// Each of Sora's frames at the KH1 setting (the request-bits wrapper, after the command menu):
/// KH1's lock-on HUD follows Re:CoM's lock. Reads only, unless the lock, the target's HP or a fade
/// changes something.
pub unsafe fn lock_tick(pl: usize) {
    if !LOCKON_KH1 {
        return;
    }
    let Some(first) = manifest().and_then(|m| m.target) else { return };
    let Ok(mut t) = TARGET.lock() else { return };
    let host = if active() { STATE.lock().ok().and_then(|s| s.clusters.iter().find(|c| !c.hidden).map(|c| (c.gauge, c.list, c.xoff))) } else { None };
    let enemy = if host.is_some() { locked_enemy(pl) } else { None };
    match (t.as_mut(), enemy) {
        (None, Some(e)) => {
            let Some((g, list, xoff)) = host else { return };
            let (hp, max) = (rd::<i32>(e + crate::battle::enemy::HP), func::<MaxHpFn>(ENEMY_MAX_HP)(e));
            *t = target_new(g, list, xoff, first, e, hp, max);
            note(|| format!("lock on: target {e:#x} HP {hp}/{max} (LOCK ON! + KH1's target gauge, H5)"));
        }
        (Some(tg), Some(e)) => {
            if tg.out {
                // A new lock while fading out: KH1 starts over (plate keys, gauge from 0).
                tg.out = false;
                tg.alpha = 0.0;
                tg.plate_alpha = 1.0;
                func::<PlayFn>(PLAY_FROM)(tg.plate, 0, UNITS);
                tg.colours();
            } else if tg.alpha < 1.0 {
                tg.alpha = (tg.alpha + kh::target::GAUGE_FADE).min(1.0);
                tg.colours();
            }
            let hp = rd::<i32>(e + crate::battle::enemy::HP);
            if e != tg.enemy || hp != tg.hp {
                let max = if e != tg.enemy { func::<MaxHpFn>(ENEMY_MAX_HP)(e) } else { tg.max };
                tg.enemy = e;
                tg.values(hp, max);
            }
        }
        (Some(tg), None) => {
            if host.is_none() {
                // The HUD is hidden or gone (events, Combat / HUD switched): at once.
                tg.destroy();
                *t = None;
                return;
            }
            if !tg.out {
                tg.out = true;
                tg.alpha = 1.0;
            } else {
                tg.alpha = (tg.alpha - kh::target::GAUGE_FADE).max(0.0);
            }
            tg.plate_alpha = (tg.plate_alpha - kh::target::PLATE_FADE).max(0.0);
            tg.colours();
            if tg.alpha <= 0.0 && tg.plate_alpha <= 0.0 {
                tg.destroy();
                *t = None;
            }
        }
        (None, None) => {}
    }
}

/// The gauge's deleting dtor (before Re:CoM's): our controls go.
pub unsafe fn destroy(gauge: usize) {
    if let Ok(mut lb) = LEVEL_BOX.lock() {
        if let Some((c, l)) = lb.take() {
            super::d2::delete(c, l);
        }
    }
    // The lock-on marker is made and deleted with the player's HUD (`0x1401FA6E0` makes both):
    // our cursor goes with any player gauge.
    if let Ok(mut c) = CURSOR.lock() {
        if let Some(x) = c.take() {
            super::d2::delete(x.ctrl, x.list);
        }
    }
    if let Ok(mut t) = TARGET.lock() {
        if t.as_ref().is_some_and(|x| x.gauge == gauge) {
            if let Some(mut x) = t.take() {
                x.destroy();
            }
        }
    }
    if let Ok(mut t) = TINTED.lock() {
        t.retain(|e| e.0 != gauge);
    }
    if let Ok(mut p) = PLACED.lock() {
        p.retain(|e| e.0 != gauge);
    }
    let Ok(mut s) = STATE.lock() else { return };
    s.gauges.retain(|g| g.0 != gauge);
    for mut c in s.clusters.extract_if(.., |c| c.gauge == gauge) {
        c.destroy();
    }
}

/// After the Config or the camp: clusters follow the setting.
pub fn refresh() {
    if manifest().is_none() {
        return;
    }
    UP.store(active(), Ordering::Relaxed);
    let bits = hidden_bits();
    MINIMAP_UP.store(bits & 8 != 0, Ordering::Relaxed);
    let place = place_wanted();
    for (g, list) in STATE.lock().map(|s| s.gauges.clone()).unwrap_or_default() {
        // SAFETY: a live gauge.
        unsafe {
            // Re:CoM's HP gauge follows the setting at once (Combat, `hud`, `hud_recom_hp_kh1_place`).
            if place {
                place_recom_hp_at(g, list, true);
            } else {
                unplace_recom_hp(g, list);
            }
            recom_hud(g, bits)
        };
    }
    if gauge_on() || ring_on() {
        let missing: Vec<(usize, i32)> = STATE.lock().map(|s| s.gauges.iter().copied().filter(|g| !s.clusters.iter().any(|c| c.gauge == g.0)).collect()).unwrap_or_default();
        for (g, list) in missing {
            // SAFETY: a live gauge (its dtor removes it from the list).
            unsafe { build(g, list) };
        }
        let _ = super::mp::current();
    } else if let Ok(mut s) = STATE.lock() {
        for mut c in s.clusters.drain(..) {
            // SAFETY: as in `mp_seen`.
            unsafe { c.destroy() };
        }
    }
}

// -------------------------------------------------------------------------------------------------
// H3: Re:CoM's HUD hidden.

unsafe fn tint(c: usize, alpha: f32) {
    if c == 0 || rd::<usize>(c) == 0 {
        return;
    }
    wr::<u8>(c + C_COLOUR_ON, 1);
    func::<VecFn>(COLOUR)(c, V4([1.0, 1.0, 1.0, alpha]).0.as_ptr());
}

/// Gauges whose Re:CoM pieces we tinted, with the bits tinted (only those are ever written
/// back: at `hud = recom` nothing of Re:CoM's is touched).
static TINTED: Mutex<Vec<(usize, u32)>> = Mutex::new(Vec::new());

/// Re:CoM's HP gauge (bit 1), FRIENDS row (2) and prize gauge (4) of `gauge`: colour alpha 0
/// for the bits in `hide`, 1 for the ones we hid before and no longer hide.
unsafe fn recom_hud(gauge: usize, hide: u32) {
    if !HOOKED.load(Ordering::Relaxed) {
        return;
    }
    let hide = hide & 7;
    let Ok(mut t) = TINTED.lock() else { return };
    let had = t.iter().find(|e| e.0 == gauge).map_or(0, |e| e.1);
    if hide == had {
        return;
    }
    t.retain(|e| e.0 != gauge);
    if hide != 0 {
        t.push((gauge, hide));
    }
    drop(t);
    for bit in [1u32, 2, 4] {
        if (hide ^ had) & bit == 0 {
            continue;
        }
        let a = if hide & bit != 0 { 0.0 } else { 1.0 };
        if bit == 1 {
            tint(rd(gauge + G_HP_CTRL), a);
            continue;
        }
        for (child, field, n) in HIDDEN_ARRAYS {
            if (bit == 2) != (child == G_FRIEND) {
                continue;
            }
            let o: usize = rd(gauge + child);
            if o == 0 {
                continue;
            }
            let arr: usize = rd(o + field);
            if arr == 0 {
                continue;
            }
            if n == 0 {
                tint(arr, a);
            } else if rd::<u64>(arr - 0x10) == n as u64 {
                for k in 0..n {
                    tint(arr + k * C_SIZE, a);
                }
            } else {
                note(|| format!("{child:#x}/{field:#x}: {} controls, not {n}: not hidden", rd::<u64>(arr - 0x10)));
            }
        }
    }
}

static MASKS: AtomicBool = AtomicBool::new(false);
/// [`active`] as last seen on an event (gauge init, the Config / camp refresh): what the mask
/// getters read, so they cost one load inside Re:CoM's per-frame updates.
static UP: AtomicBool = AtomicBool::new(false);

/// The enemy gauge's mask read: Re:CoM's value, plus its hide bit while the KH1 HUD is up.
pub unsafe extern "win64" fn enemy_gauge_mask(table: u32, index: u32) -> u32 {
    func::<GetFn>(VAR_GET)(table, index) | if LOCKON_KH1 && UP.load(Ordering::Relaxed) { MASK_ENEMY_GAUGE } else { 0 }
}

/// The minimap's mask reads: likewise with its bit.
pub unsafe extern "win64" fn minimap_mask(table: u32, index: u32) -> u32 {
    func::<GetFn>(VAR_GET)(table, index) | if MINIMAP_UP.load(Ordering::Relaxed) { MASK_MINIMAP } else { 0 }
}

/// Retargets the five mask reads to our getters (all or none; the getters pass Re:CoM's value
/// through unless the KH1 HUD is up).
unsafe fn install_masks(bs: usize) {
    let calls = crate::sites::KH1_HUD_MASK_CALLS;
    let mut code = Vec::with_capacity(28);
    code.extend_from_slice(&crate::hookplan::abs_jmp(enemy_gauge_mask as *const () as u64));
    code.extend_from_slice(&crate::hookplan::abs_jmp(minimap_mask as *const () as u64));
    let Some(block) = super::hook::place_near(bs + calls[0].rva as usize, &code) else {
        log("kh1 hud: no memory near the exe for the mask stubs; enemy gauges and minimap stay");
        return;
    };
    let mut writes = Vec::new();
    for (i, c) in calls.iter().enumerate() {
        let stub = if i == 0 { block } else { block + 14 };
        match crate::hookplan::call_rel32((bs + c.rva as usize) as u64, stub as u64) {
            Some(b) => writes.push((c, b)),
            None => {
                log(&format!("kh1 hud: {} cannot reach the stub; enemy gauges and minimap stay", c.name));
                return;
            }
        }
    }
    let mut done: Vec<(&crate::sites::Site, [u8; 5])> = Vec::new();
    for &(c, b) in &writes {
        if !super::hook::patch(bs + c.rva as usize, c.expect, &b, c.name) {
            for &(d, db) in done.iter().rev() {
                super::hook::patch(bs + d.rva as usize, &db, d.expect, d.name);
            }
            log("kh1 hud: a mask read could not be switched; enemy gauges and minimap stay");
            return;
        }
        done.push((c, b));
    }
    MASKS.store(true, Ordering::Relaxed);
    log(&format!("kh1 hud: enemy-gauge and minimap mask reads (5) through our getters at {block:#x} (their own hide paths while the KH1 HUD is up, H3)"));
}
