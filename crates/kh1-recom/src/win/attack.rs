//! KH1's Attack in Re:CoM's own attack action (`crate::attack`, `docs/recom-kh1-attack.md`).
//!
//! Hooks (`sites::ATTACK_HOOKS`), each a pass-through unless `combat = kh1` and the player is the
//! main player's `CPlayerSora`:
//!
//! - request bits `0x1401F8150`: in battle, Re:CoM's field branch builds the request (confirm →
//!   Attack `+0x424 & 4`; no card scroll, reload, stock or sleight input); a jump pressed on the same
//!   frame wins (KH1's order).
//! - use-card button `0x1401E8FF0`: skipped (no card plays at KH1).
//! - initAttack `0x1401F0D90`: the card attack becomes the field swing.
//! - field swing init `0x1401F0F70`: Re:CoM's command dispatch reached it, so KH1's frame runs with
//!   the Attack press (its command picks the row), and Re:CoM's init then starts that row.
//! - field swing exe `0x1401EAEB0` (also reached from the Battle exe through its jump-table entry for
//!   action `0xA`): KH1's frame (combo window, chain, facing, air physics), then Re:CoM's exe (hit
//!   window, `onAttack`, end), then our velocity and facing again.
//! - Sora's vtable slot `+0x128` (the row pick): takes the KH1 row KH1's frame chose.
//! - attack pair dispatch `0x14016EAD0`: for Sora's Keyblade on a KH1 row, KH1's sphere chain
//!   tested with Re:CoM's own sphere / column primitives.
//! - enemy hit handler `0x14017F7E0`: a KH1 hit lands during the enemy's card play (decision 14),
//!   through Re:CoM's own interruptible-card path.
//! - player damage check `0x1401E6DE0`: KH1's no-flinch frames (damage, no reaction).
//! - root motion step `0x14024DCA0`: Sora's root travel one update late (decision 17).

use super::movement::{self as mv, func, orig, p, rd, rd_vec, wr};
use super::{base, log, va};
use crate::attack::{self as atk, field};
use crate::movement::SwingStart;
use crate::sites::data;
use crate::units::to_kh1;
use kh1_core::{Moveset, TargetPart, Vec3, WeaponData};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

pub static ORIG_REQUEST_BITS: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_CARD_BUTTON: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_INIT_ATTACK: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_FIELD_INIT: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_SWING_EXE: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_PAIR: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_ENEMY_HIT: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_DAMAGE_CHECK: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_ROOT_STEP: AtomicUsize = AtomicUsize::new(0);

/// The KH1 rows (by KH1 row id), built once from Re:CoM's rows 1–7. Never freed: `+0x3C8` points
/// into it.
static TABLE: OnceLock<&'static [[u8; 16]]> = OnceLock::new();
static MOVESET: OnceLock<Moveset> = OnceLock::new();
/// The main player whose KH1 no-flinch is on.
static NO_FLINCH: AtomicUsize = AtomicUsize::new(0);
/// Sora's frames (counted by the state exes), for the root delay.
static TICK: AtomicU64 = AtomicU64::new(0);
static ROOT: Mutex<crate::movement::RootDelay> = Mutex::new(crate::movement::RootDelay::new());
static BLADE: Mutex<atk::BladeTrack> = Mutex::new(atk::BladeTrack { serial: 0, cur_a: None, prev_a: None });
/// Enemy body parts a KH1 swing hit, until their owner's hit handler takes the hit.
static KH1_HITS: Mutex<Vec<usize>> = Mutex::new(Vec::new());
/// `debug_attack = on`: one-line logs of the attack's decisions, at most [`DEBUG_LINES`].
static DEBUG: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static DEBUG_LEFT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(DEBUG_LINES);
const DEBUG_LINES: u32 = 200;

pub fn set_debug(on: bool) {
    DEBUG.store(on, Ordering::Relaxed);
}

fn debug(line: impl FnOnce() -> String) {
    if !DEBUG.load(Ordering::Relaxed) {
        return;
    }
    if DEBUG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("debug_attack: {}", line()));
    }
}

/// The trace's hit log for the frame.
pub static HIT_LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// The swing KH1's frame chose, for Re:CoM's row pick inside the init it calls (set and taken
/// separately: never held across the call).
static PENDING: Mutex<Option<SwingStart>> = Mutex::new(None);

fn set_pending(s: Option<SwingStart>) {
    if let Ok(mut g) = PENDING.lock() {
        *g = s;
    }
}

fn take_pending() -> Option<SwingStart> {
    PENDING.lock().ok().and_then(|mut g| g.take())
}

// Re:CoM functions.
const END_ATTACK: u32 = 0x001F_6660;
const WEAPON_END: u32 = 0x0022_4FB0;
const ACTOR_BONE: u32 = 0x002A_BB20;
const SPHERE_SPHERE: u32 = 0x0017_77E0;
const SPHERE_COLUMN: u32 = 0x0017_6FA0;
const RECOM_PICKER: u64 = 0x1_401F_7520;
/// The collision context the pair dispatch passes to the primitives (`DAT_14087A870`).
const COLL_CONTEXT: u64 = 0x1_4087_A870;
/// `CPlayerSora` vtable slot `+0x128` (the row pick). Checked (exe: holds `0x1401F7520`).
const SORA_PICKER_SLOT: u64 = 0x1_405A_9938 + 0x128;

type PlFn = unsafe extern "win64" fn(usize);
type PlRetFn = unsafe extern "win64" fn(usize) -> u64;
type PlIntFn = unsafe extern "win64" fn(usize, i32) -> u64;
type PairFn = unsafe extern "win64" fn(usize, usize) -> i32;
type PrimFn = unsafe extern "win64" fn(usize, usize, usize, i32) -> u8;
type BoneFn = unsafe extern "win64" fn(usize, i32) -> usize;
type ApplyFn = unsafe extern "win64" fn(usize, usize) -> i32;
type RootFn = unsafe extern "win64" fn(usize, *mut [f32; 4], *mut f32);
type VecFn = unsafe extern "win64" fn(usize, *mut [f32; 4]) -> u64;

pub(super) fn moveset() -> &'static Moveset {
    MOVESET.get_or_init(Moveset::kh1_defaults)
}

/// The KH1 row id `+0x3C8` points at, if it points into our table.
unsafe fn kh1_row_of(pl: usize) -> Option<usize> {
    let t = TABLE.get()?;
    let ptr: usize = rd(pl + field::ROW_PTR);
    let start = t.as_ptr() as usize;
    (ptr >= start && ptr < start + t.len() * 16 && (ptr - start).is_multiple_of(16)).then(|| (ptr - start) / 16)
}

/// Sora's body (`player+0x690`) guard fields (Checked (decomp `0x14016F1C0`, body ctor)):
/// `+0x9C` flags (1 guard, 0x20 guarded), `+0x8C` guard element mask (all by default), `+0xB0`
/// the guard's half-angle (π by default).
const BODY_GUARD: usize = 0x690 + 0x9C;
const BODY_GUARD_ANGLE: usize = 0x690 + 0xB0;
/// KH1's guard arc: cosine −0.1736 (record `0x4DA8` +0x64) = ±100°.
const KH1_GUARD_ANGLE: f32 = 1.745_329_3;
static GUARD_ON: AtomicUsize = AtomicUsize::new(0);
static GUARD_HIT: AtomicUsize = AtomicUsize::new(0);

/// KH1's guard object live: Re:CoM's own body guard on Sora (all elements, ±100°), or off.
pub(super) unsafe fn set_guard(pl: usize, on: bool) {
    let f: u32 = rd(pl + BODY_GUARD);
    if on {
        wr(pl + BODY_GUARD, f | 1);
        wr(pl + BODY_GUARD_ANGLE, KH1_GUARD_ANGLE);
        GUARD_ON.store(pl, Ordering::Relaxed);
    } else {
        wr(pl + BODY_GUARD, f & !1);
        wr(pl + BODY_GUARD_ANGLE, std::f32::consts::PI);
        GUARD_ON.store(0, Ordering::Relaxed);
    }
    GUARD_HIT.store(0, Ordering::Relaxed);
}

/// A guarded hit since the last KH1 frame.
pub(super) unsafe fn take_guard_hit(pl: usize) -> bool {
    GUARD_HIT.swap(0, Ordering::Relaxed) == pl || rd::<u32>(pl + BODY_GUARD) & 0x20 != 0
}

/// Whether our attack table holds this KH1 row.
pub(super) fn row_ready(r: usize) -> bool {
    TABLE.get().is_some_and(|t| t.get(r).is_some_and(|b| b[0] != 0))
}

pub(super) fn set_no_flinch(pl: usize, on: bool) {
    NO_FLINCH.store(if on { pl } else { 0 }, Ordering::Relaxed);
}

/// A state exe ran for the main Sora (one per frame).
pub(super) fn tick() {
    TICK.fetch_add(1, Ordering::Relaxed);
    TICK_LIVE.store(true, Ordering::Relaxed);
}

/// The state exes count Sora's frames (the trace hooks are in): without that count the delay
/// can't tell consecutive updates, and Re:CoM's own same-frame root motion stays.
static TICK_LIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Re:CoM's attack target (`player+0x84C`) as a KH1 body part: its aim point (vtable `+0x50`, as
/// Re:CoM's own row pick reads it). Assumption: radius 0 (Re:CoM's distances are centre to centre).
pub(super) unsafe fn attack_target(pl: usize) -> Option<TargetPart> {
    let h: i32 = rd(pl + p::LOCK_TARGET);
    if h <= 0 {
        return None;
    }
    let obj = func::<mv::HandleFn>(mv::HANDLE_OBJ)(h as u32);
    if obj == 0 {
        return None;
    }
    let vt: usize = rd(obj);
    let f: VecFn = std::mem::transmute_copy(&rd::<usize>(vt + 0x50));
    let mut out = [0f32; 4];
    f(obj, &mut out);
    Some(TargetPart { owner: h as u32, centre: to_kh1(Vec3::new(out[0], out[1], out[2])), radius: 0.0 })
}

// ---- starting and ending swings (from `RecomHost`) ----

/// KH1's frame chose a swing: Re:CoM's field-swing init starts it, and its row pick (our vtable
/// slot) takes the row from `PENDING`.
pub(super) unsafe fn start_swing(pl: usize, s: &SwingStart) -> bool {
    let Some(t) = TABLE.get() else { return false };
    if t.get(s.kh1_row).is_none_or(|r| r[0] == 0) {
        return false;
    }
    // A chain: end the last swing's volume first, as Re:CoM's own chain does (`0x1401EAFC2`).
    let w: usize = rd(pl + field::WEAPON);
    if w != 0 && rd::<u32>(pl + p::ACTION) == crate::movement::action::ATTACK {
        func::<PlFn>(WEAPON_END)(w);
    }
    set_pending(Some(*s));
    debug(|| {
        let b: usize = rd(pl + p::BODY);
        let pos = if b != 0 { to_kh1(rd_vec(b + 0x30)) } else { Vec3::ZERO };
        let t = attack_target(pl);
        format!(
            "start row={} id={} req={} root={} n={}/{} fin={} action={:#x} pos=({:.1},{:.1},{:.1}) target={:?} lock={}",
            s.kh1_row, s.recom_id, s.request, s.root, s.counter, s.length, s.finisher, rd::<u32>(pl + p::ACTION), pos.x, pos.y, pos.z,
            t.map(|t| (t.centre.x.round(), t.centre.y.round(), t.centre.z.round())),
            rd::<i32>(pl + p::BATTLE) != 0 && rd::<usize>(pl + p::TARGET_CURSOR) != 0 && rd::<i32>(rd::<usize>(pl + p::TARGET_CURSOR) + 0xE0) != 0
        )
    });
    let o: PlIntFn = orig(&ORIG_FIELD_INIT);
    o(pl, s.counter.saturating_sub(1) as i32);
    // The row pick took it, unless something replaced Sora's vtable slot.
    take_pending().is_none()
}

pub(super) unsafe fn end_swing(pl: usize, airborne: bool) {
    func::<PlIntFn>(END_ATTACK)(pl, airborne as i32);
}

/// Re:CoM's row pick for a KH1 row (`0x1401F7520`'s writes, cardless).
unsafe fn apply_row(pl: usize, s: &SwingStart) -> bool {
    let Some(t) = TABLE.get() else { return false };
    let Some(row) = t.get(s.kh1_row).filter(|r| r[0] != 0) else { return false };
    wr(pl + field::HOMING_TARGET, 0i32);
    wr(pl + field::VSPEED, 0f32);
    wr(pl + field::SITUATION, s.recom_id);
    wr(pl + field::ATTACK_ID, s.recom_id);
    wr(pl + field::ROW_PTR, row.as_ptr() as usize);
    func::<mv::MotionFn>(mv::MOTION_REQUEST)(pl, row[0] as i32, 0, s.root as u64, 0, 0, 0);
    wr(pl + field::CARD_7B4, 0i32);
    // Re:CoM's cardless AP (the attack stat) until the KH1 twins' stats exist (fallback, logged).
    wr(pl + field::AP, rd::<i32>(pl + field::ATTACK_STAT));
    wr(pl + field::KIND, row[0xB] as u32);
    wr(pl + field::ATTR, 0u32);
    wr(pl + field::CHAINABLE, 0u32);
    wr(pl + field::PREV_SITUATION, s.recom_id);
    wr(pl + field::COUNTER, s.counter as i32);
    wr(pl + field::LENGTH, s.length as i32);
    // KH1 has no post-swing lockout, and a swing starts from standing (root travel only).
    wr(pl + field::LOCKOUT, 0i32);
    wr(pl + field::SPEED_4E0, 0f32);
    if let Ok(mut g) = KH1_HITS.lock() {
        g.clear();
    }
    true
}

// ---- hooks ----

unsafe fn battle_guards_clear(pl: usize, req: u32) -> bool {
    // The Battle branch's own early returns (`0x1401F8150`): keep them Re:CoM's.
    rd::<u32>(pl + p::FLAGS_418) & 0x4_0000 == 0 && rd::<i32>(pl + 0x7B0) == 0 && rd::<u32>(pl + p::CARD) & 4 == 0 && rd::<u32>(pl + p::FLAGS_420) & 0x200_0000 == 0 && req & 0x10_0000 == 0
}

pub unsafe extern "win64" fn request_bits(pl: usize) {
    let o: PlFn = orig(&ORIG_REQUEST_BITS);
    if !mv::kh1_sora(pl) {
        return o(pl);
    }
    let battle: i32 = rd(pl + p::BATTLE);
    if battle != 0 && battle_guards_clear(pl, rd(pl + p::REQUEST)) {
        // Re:CoM's own field branch: the confirm press is the Attack, no card input.
        wr(pl + p::BATTLE, 0i32);
        o(pl);
        wr(pl + p::BATTLE, battle);
    } else {
        o(pl);
    }
    // KH1: a jump on the same frame wins over the Attack (`0x1402A6220`).
    let r: u32 = rd(pl + p::REQUEST);
    if r & 1 != 0 && r & 4 != 0 {
        wr(pl + p::REQUEST, r & !4);
    }
}

pub unsafe extern "win64" fn card_button(pl: usize) -> u64 {
    if mv::kh1_sora(pl) {
        return 0;
    }
    orig::<PlRetFn>(&ORIG_CARD_BUTTON)(pl)
}

pub unsafe extern "win64" fn init_attack(pl: usize, a: i32) -> u64 {
    if mv::kh1_sora(pl) {
        field_init(pl, 0);
        return 0;
    }
    orig::<PlIntFn>(&ORIG_INIT_ATTACK)(pl, a)
}

pub unsafe extern "win64" fn field_init(pl: usize, counter: i32) -> u64 {
    let o: PlIntFn = orig(&ORIG_FIELD_INIT);
    if !mv::kh1_sora(pl) || TABLE.get().is_none() {
        return o(pl, counter);
    }
    // Re:CoM's dispatch saw the Attack: KH1's frame decides (its command, its picker) and starts
    // the swing through `start_swing`.
    let fresh = mv::dispatch_fresh(pl);
    debug(|| format!("dispatch action={:#x} fresh={fresh} req={:#x}", rd::<u32>(pl + p::ACTION), rd::<u32>(pl + p::REQUEST)));
    mv::run_frame(pl, fresh);
    0
}

pub unsafe extern "win64" fn swing_exe(pl: usize) {
    let o: PlFn = orig(&ORIG_SWING_EXE);
    if !mv::kh1_sora(pl) || kh1_row_of(pl).is_none() {
        return o(pl);
    }
    if !mv::continues(pl) {
        // Re:CoM ran something else in between: its exe finishes this swing.
        return o(pl);
    }
    mv::run_frame(pl, false);
    debug(|| {
        let c: usize = rd(pl + p::CTRL);
        let b: usize = rd(pl + p::BODY);
        let v = if b != 0 { rd_vec(b + 0x80) } else { Vec3::ZERO };
        format!(
            "exe action={:#x} frame={:.1} vel=({:.3},{:.3},{:.3}) contact={:#x} hit={}",
            rd::<u32>(pl + p::ACTION),
            if c != 0 { rd::<i32>(c + 0x3C) as f32 / 50.0 } else { -1.0 },
            v.x,
            v.y,
            v.z,
            if b != 0 { rd::<u32>(b + 0x120) } else { 0 },
            rd::<i32>(pl + field::HIT)
        )
    });
    if rd::<u32>(pl + p::ACTION) == crate::movement::action::ATTACK {
        o(pl);
        if rd::<u32>(pl + p::ACTION) == crate::movement::action::ATTACK {
            mv::reapply(pl);
        }
    }
}

/// Sora's vtable slot `+0x128`.
pub unsafe extern "win64" fn picker(pl: usize) {
    if mv::kh1_sora(pl) {
        if let Some(s) = take_pending() {
            if apply_row(pl, &s) {
                return;
            }
        }
    }
    let f: PlFn = std::mem::transmute_copy(&va(base(), RECOM_PICKER));
    f(pl)
}

/// A fake Re:CoM sphere shape (`CSphereCollision` fields the primitives read: `+0x10` / `+0x20`
/// the swept centres, `+0x70` radius, `+0x74` height offset; they write `+0x30..+0x60`).
#[repr(C, align(16))]
struct FakeSphere([u8; 0x90]);

pub unsafe extern "win64" fn pair_dispatch(coll: usize, list: usize) -> i32 {
    let o: PairFn = orig(&ORIG_PAIR);
    if !mv::kh1_on() {
        return o(coll, list);
    }
    let b = base();
    let pl: usize = rd(va(b, data::MAIN_PLAYER));
    if !mv::is_main_sora(pl) {
        return o(coll, list);
    }
    let weapon: usize = rd(pl + field::WEAPON);
    let shape: usize = rd(coll + 0x18);
    let Some(kh1_row) = kh1_row_of(pl) else { return o(coll, list) };
    if weapon == 0 || coll != weapon + 0x240 || shape == 0 {
        return o(coll, list);
    }
    let Some(live) = atk::live_attack(moveset(), kh1_row, &WeaponData::kingdom_key()) else { return o(coll, list) };
    let bone = func::<BoneFn>(ACTOR_BONE);
    let (b2, b3) = (bone(weapon, 2), bone(weapon, 3));
    if b2 == 0 || b3 == 0 {
        return o(coll, list);
    }
    let (w2, w3) = (rd_vec(b2 + 0xD0), rd_vec(b3 + 0xD0));
    let (a, bp) = atk::blade_points(w2, w3, &live, &WeaponData::kingdom_key());
    let serial: i32 = rd(coll + 0x70);
    let prev = BLADE.lock().map(|mut t| t.frame(serial, a)).unwrap_or(None);
    // Assumption: Sora's centre for the first frame's sweep is `player+0x4B0` (the attack origin
    // `onAttack` copies).
    let owner = rd_vec(pl + 0x4B0);
    let spheres = atk::blade_shapes(a, bp, live.radius * crate::units::METRES_PER_UNIT, live.data.sweeps(), prev, owner);
    let ctx: usize = rd(va(b, COLL_CONTEXT));
    let owner_id: i32 = rd(coll + 0x20);
    let vt: usize = rd(coll);
    let apply: ApplyFn = std::mem::transmute_copy(&rd::<usize>(vt + 0x38));
    let mut fake = FakeSphere([0; 0x90]);
    let fp = fake.0.as_mut_ptr() as usize;
    wr(fp + 0x60, 2u32);
    let mut result = 0;
    let mut node: usize = rd(list + 8);
    while node != 0 {
        let next: usize = rd(node + 8);
        if rd::<i32>(node + 0x10) != 0 && rd::<i32>(node + 0x20) != owner_id {
            let bshape: usize = rd(node + 0x18);
            let flags = if bshape == 0 { 0xFFu8 } else { rd::<u32>(bshape + 100) as u8 };
            if flags & 0xE != 6 && bshape != 0 {
                let kind: u32 = rd(bshape + 0x60);
                let mut hit = false;
                for &(c, r) in &spheres {
                    for o4 in [0x10, 0x20] {
                        wr(fp + o4, [c.x, c.y, c.z, 1.0f32]);
                    }
                    wr(fp + 0x70, r);
                    wr(fp + 0x74, 0f32);
                    hit = match kind {
                        2 => func::<PrimFn>(SPHERE_SPHERE)(ctx, bshape, fp, 0) != 0,
                        4 | 5 => func::<PrimFn>(SPHERE_COLUMN)(ctx, bshape, fp, 0) != 0,
                        _ => false,
                    };
                    if hit {
                        break;
                    }
                }
                if hit {
                    // The contact the apply reads from the attack's shape (`+0x30..+0x60`).
                    std::ptr::copy_nonoverlapping((fp + 0x30) as *const u8, (shape + 0x30) as *mut u8, 0x30);
                    let r = apply(coll, node);
                    if r != 0 {
                        if let Ok(mut g) = KH1_HITS.lock() {
                            if g.len() < 64 {
                                g.push(node);
                            }
                        }
                    }
                    if result == 0 {
                        result = r;
                    }
                }
            }
        }
        node = next;
    }
    result
}

/// Whether one of the enemy's body parts took a KH1 swing's hit (consumed).
unsafe fn take_kh1_hit(enemy: usize) -> Option<usize> {
    let info: usize = rd(enemy + 0xF8);
    if info == 0 {
        return None;
    }
    let parts: usize = rd(info + 0x40);
    let n: i32 = rd(info + 0x38);
    let Ok(mut g) = KH1_HITS.lock() else { return None };
    for i in 0..n.clamp(0, 32) as usize {
        let part = parts + i * 0x120;
        if let Some(k) = g.iter().position(|&x| x == part) {
            if rd::<i16>(part + 0x90) != 0 {
                g.swap_remove(k);
                return Some(part);
            }
        }
    }
    None
}

/// Re:CoM's enemy hit handler. KH1's rule (decision 14): Sora's hits land during the enemy's card
/// play. Re:CoM's own path for an interruptible card (`+0x260 & 0x2000001`: end the card, then
/// take the hit) runs for a KH1 hit.
pub unsafe extern "win64" fn enemy_hit(enemy: usize) -> u64 {
    let o: PlRetFn = orig(&ORIG_ENEMY_HIT);
    if !mv::kh1_on() {
        return o(enemy);
    }
    let Some(part) = take_kh1_hit(enemy) else { return o(enemy) };
    // KH1's damage (decision 12) as the hit part's value (`+0x92`, read by `0x14016FDB0`); Re:CoM's
    // own path then applies its weak part, resistance % and boss falloff (decision 13).
    let pl: usize = rd(va(base(), data::MAIN_PLAYER));
    let row = if pl != 0 { kh1_row_of(pl) } else { None };
    let kh1 = row.and_then(|r| super::stats::kh1_hit(enemy, r, super::stats::sora_level()));
    if let Some(d) = kh1 {
        wr(part + 0x92, d.clamp(0, i16::MAX as i32) as i16);
    }
    let flags: u64 = rd(enemy + 0x260);
    let force = flags & 1 != 0 && flags & 0x200_0000 == 0;
    if force {
        wr(enemy + 0x260, flags | 0x200_0000);
    }
    let hp: i32 = rd(enemy + 0x274);
    let r = o(enemy);
    if force {
        let now: u64 = rd(enemy + 0x260);
        wr(enemy + 0x260, now & !0x200_0000);
    }
    let after: i32 = rd(enemy + 0x274);
    if let Ok(mut g) = HIT_LOG.lock() {
        if g.len() < 16 {
            g.push(format!("e{:x}:dmg {}:hp {}{}{}", enemy & 0x00FF_FFFF, hp - after, after, if force { ":card" } else { "" }, kh1.map_or(String::new(), |d| format!(":kh1 {d}"))));
        }
    }
    r
}

/// Re:CoM's player damage check: KH1's no-flinch (`0x140297F30`: damage, no hit reaction) uses
/// the check's own no-reaction exit (`+0x420 & 2`, read once into a local at its entry; its only
/// other readers are the room-card regen ticks, which don't run inside this call).
pub unsafe extern "win64" fn damage_check(pl: usize) -> u64 {
    let o: PlRetFn = orig(&ORIG_DAMAGE_CHECK);
    // A hit guarded by Sora's body guard (`body+0x9C & 0x20`, `player+0x72C`): kept for the next
    // KH1 frame (Re:CoM clears the bit each frame, `0x1401F4000` / `0x1401FABC0`).
    if GUARD_ON.load(Ordering::Relaxed) == pl && rd::<u32>(pl + BODY_GUARD) & 0x20 != 0 {
        GUARD_HIT.store(pl, Ordering::Relaxed);
    }
    if NO_FLINCH.load(Ordering::Relaxed) != pl || !mv::kh1_sora(pl) {
        return o(pl);
    }
    let f: u32 = rd(pl + p::FLAGS_420);
    wr(pl + p::FLAGS_420, f | 2);
    let r = o(pl);
    if f & 2 == 0 {
        wr(pl + p::FLAGS_420, rd::<u32>(pl + p::FLAGS_420) & !2);
    }
    r
}

/// Re:CoM's root step (`0x14024DCA0(ctrl, out travel, out yaw)`): for Sora's KH1 actions (the roll
/// and the swings), the last update's travel (decision 17).
pub unsafe extern "win64" fn root_step(ctrl: usize, out: *mut [f32; 4], yaw: *mut f32) {
    let o: RootFn = orig(&ORIG_ROOT_STEP);
    o(ctrl, out, yaw);
    if !mv::kh1_on() || out.is_null() || !TICK_LIVE.load(Ordering::Relaxed) {
        return;
    }
    let pl: usize = rd(va(base(), data::MAIN_PLAYER));
    if !mv::is_main_sora(pl) || rd::<usize>(pl + p::CTRL) != ctrl {
        return;
    }
    let a: u32 = rd(pl + p::ACTION);
    if a != crate::movement::action::AVOID && a != crate::movement::action::ATTACK {
        return;
    }
    // `out` is an absolute position (the model's position `ctrl+0xF0` + this update's travel,
    // `0x14024C960`), not a travel: delay the travel, keep the anchor.
    let v = &mut *out;
    let root = Vec3::new(v[0], v[1], v[2]);
    let anchor = rd_vec(ctrl + 0xF0);
    if let Ok(mut d) = ROOT.lock() {
        let t = d.position(ctrl, TICK.load(Ordering::Relaxed), anchor, root, crate::movement::ROOT_DELAY_MAX_GAP);
        debug(|| format!("root a={a} id={id} anchor=({:.3},{:.3},{:.3}) root=({:.3},{:.3},{:.3}) out=({:.3},{:.3},{:.3})", anchor.x, anchor.y, anchor.z, root.x, root.y, root.z, t.x, t.y, t.z, id = rd::<u32>(pl + field::ATTACK_ID)));
        v[0] = t.x;
        v[1] = t.y;
        v[2] = t.z;
    }
}

// ---- install ----

/// Builds the KH1 rows from the live Re:CoM rows 1–7 (which must be exactly the Steam exe's).
pub unsafe fn build_table(b: usize) -> bool {
    let rows = va(b, atk::RECOM_ATTACK_ROWS) + 16;
    let mut live = [[0u8; 16]; 7];
    for (i, r) in live.iter_mut().enumerate() {
        *r = rd(rows + 16 * i);
    }
    if live != atk::RECOM_ROWS_1_7 {
        log("attack: Re:CoM's attack rows 1-7 are not the Steam exe's; KH1 Attack not installed");
        return false;
    }
    let t: &'static [[u8; 16]] = Box::leak(atk::kh1_table(&live, moveset(), mv::EXT.get().and_then(|e| e.get(crate::rows::COUNTERATTACK))).into_boxed_slice());
    TABLE.set(t).is_ok()
}

/// Sora's vtable slot `+0x128` → our row pick (checked old value), and the Battle exe's case for
/// action `0xA` (jump-table entry → the cave stub). Data patches, nothing per frame.
pub unsafe fn patch_data(b: usize) -> bool {
    use super::hook::patch;
    let slot = va(b, SORA_PICKER_SLOT);
    let old = (va(b, RECOM_PICKER) as u64).to_le_bytes();
    let new = (picker as *const () as usize as u64).to_le_bytes();
    let cave = va(b, 0x1_4000_0000 + atk::battle_case::CAVE_RVA as u64);
    let entry = va(b, 0x1_4000_0000 + atk::battle_case::TABLE_ENTRY_RVA as u64);
    let old_entry = atk::battle_case::DEFAULT_RVA.to_le_bytes();
    let stub = atk::battle_case::stub();
    // All three or none: check every old value first.
    let have = |at: usize, n: usize| std::slice::from_raw_parts(at as *const u8, n).to_vec();
    if have(cave, 13) != [0xCC; 13] || have(entry, 4) != old_entry || have(slot, 8) != old {
        log("attack: the Battle case, its cave or Sora's row-pick slot is not as expected; KH1 Attack not installed");
        return false;
    }
    if !patch(cave, &[0xCC; 13], &stub, "attack: Battle case stub") {
        return false;
    }
    if !patch(entry, &old_entry, &atk::battle_case::entry(), "attack: Battle jump table 0xA") {
        return false;
    }
    patch(slot, &old, &new, "attack: Sora's row pick slot")
}
