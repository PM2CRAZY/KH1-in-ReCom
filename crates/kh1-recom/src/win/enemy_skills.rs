//! Enemy Skills' effects through Re:CoM's own enemy-card code (`crate::skill_effects`,
//! `docs/recom-enemy-skills.md` § Stage 2 (built)). Combat = KH1 in Sora's story only.
//!
//! **Stage 2, route 1 (the play-time setters).** Re:CoM's player-side writes of `0x140163C00` for
//! each equipped skill (its `+0x420` bits, Draw's radius, the Vanish / Aero Guard setters) and its
//! Sora-side CBTL flags of `0x140163170` (Bio, Regen); never the bosses' affinity call `0x1401F8810`.
//! Applied:
//! - at the first KH1 frame of each player object (a battle's Sora, the field's: [`kh1_frame`], one
//!   atomic compare a frame), after every Re:CoM battle start ([`battle_start`] marks it);
//! - after every Sora-side clear `0x140161E10` (our wrapper [`clear`]; not at the battle's end);
//! - when the ability set changes or Combat switches ([`refresh`], which also takes off what is no
//!   longer equipped).
//! In a field room only Draw (the prize radius); everything else in battle. Re:CoM's own readers do
//! the rest. Where the owner's number differs, Re:CoM's constant is switched at each battle start
//! (`sites::ENEMY_SKILL_PATCHES`, the Roxas stub `sites::ENEMY_SKILL_STUB_SITES`), or the setter's
//! parameter is ours (Card Soldier's motion speed), or the KH1 side's (Float's gravity, Bandit's
//! combo length, Guard Armor's KH1 sphere radii by Re:CoM's own scale).

use super::movement::{self as mv, func, orig, p, rd, wr};
use super::{base, log, va};
use crate::skill_effects::{self as se, cbtl, draw, flag420, numbers, Play};
use crate::sites::{self, data};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

pub static ORIG_CLEAR: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_REWRITE: AtomicUsize = AtomicUsize::new(0);
/// Stage 3: the equipped route-2 skills (`skill_effects::rewrite_mask`), the stub block (cells then
/// stubs) and its stubs' offsets, and whether the rewrite's wrapper is in.
static RW_MASK: AtomicU32 = AtomicU32::new(0);
static RW_BLOCK: AtomicUsize = AtomicUsize::new(0);
static RW_OFFS: std::sync::OnceLock<[usize; 5]> = std::sync::OnceLock::new();
static RW_INSTALLED: AtomicBool = AtomicBool::new(false);
/// Lexaeus' last warp: the kind it replaced + 1 (0: none), for "never bosses".
static LEX_FROM: AtomicU32 = AtomicU32::new(0);
static INSTALLED: AtomicBool = AtomicBool::new(false);
/// The equipped skills' effects as last read (`Play` packed, [`pack`]); 0 when nothing applies.
static PLAY: AtomicU32 = AtomicU32::new(0);
/// The player the effects were last applied to, and the generation then ([`GEN`]).
static APPLIED_PL: AtomicUsize = AtomicUsize::new(0);
static APPLIED_GEN: AtomicU32 = AtomicU32::new(u32::MAX);
/// Bumped by each battle start, battle end, ability change and Combat switch: the next KH1 frame
/// applies again.
static GEN: AtomicU32 = AtomicU32::new(0);
/// The battle the switched sites were last set for: a KH1 battle (Combat = KH1, Sora's story).
static KH1_BATTLE: AtomicBool = AtomicBool::new(false);
/// The Roxas stub (0: none).
static STUB: AtomicUsize = AtomicUsize::new(0);
static LINES: AtomicU32 = AtomicU32::new(40);

const CBTL_PTR: u64 = 0x1_4087_A818;
const VANISH: u32 = 0x001F_7EC0;
const AERO: u32 = 0x001F_6FB0;
const MOTION_SPEED: u32 = 0x001F_74D0;
const BARRIER_TEST: u32 = 0x0016_EED0;
const BARRIER_SET: u32 = 0x001F_8FF0;
type SetFn = unsafe extern "win64" fn(usize, i32);
type SpeedFn = unsafe extern "win64" fn(usize, f32);
type TestFn = unsafe extern "win64" fn(usize, usize) -> u64;
type ClearFn = unsafe extern "win64" fn(usize, i32);
type RewriteFn = unsafe extern "win64" fn(usize, usize) -> u64;

/// Sora's body (`player+0x690`): the hit Re:CoM's apply recorded (`0x14016F1C0`: `+0x30` the
/// attack's point, `+0x84` its attribute / barrier class, `+0x92` the value) and its barrier fields
/// (`+0x88` class mask = `player+0x718`, `+0x9C` = `player+0x72C`, `+0xAC` half-angle =
/// `player+0x73C`). Checked (decomp).
const BODY: usize = 0x690;
const HIT_POINT: usize = BODY + 0x30;
const HIT_CLASS: usize = BODY + 0x84;
const HIT_PENDING: usize = 0x720;
const HIT_VALUE: usize = BODY + 0x92;
const BARRIER_MASK: usize = 0x718;
const BARRIER_FLAGS: usize = 0x72C;
const BARRIER_ANGLE: usize = 0x73C;
/// The weapon (`player+0x3D0`): `+0x2CC` its bits (1 wide, 0x10 back), its collision at `+0x240`
/// (power `+0x288` = collision `+0x48`, back bonus `+0x28C` = `+0x4C`). Checked (decomp
/// `0x1402250B0`, `0x1402257C0`, `0x140225320`, `0x14016F1C0`).
const WEAPON: usize = 0x3D0;
const W_BITS: usize = 0x2CC;
const W_POWER: usize = 0x288;
const W_BACK: usize = 0x28C;
/// `initAttack`'s bonus (`+0x664`).
const BONUS: usize = 0x664;

fn line(f: impl FnOnce() -> String) {
    if LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("enemy skills: {}", f()));
    }
}

fn pack(p: &Play) -> u32 {
    [p.draw, p.large_body, p.combo_finish, p.back_attack, p.float, p.vanish, p.bio, p.wide, p.haste, p.regen, p.quick_barrier, p.aero_guard, p.combo_boost, p.double_strike]
        .iter()
        .enumerate()
        .fold(1 << 31, |a, (i, &b)| a | (b as u32) << i)
}

fn unpack(v: u32) -> Play {
    let b = |i: u32| v & (1 << i) != 0;
    Play { draw: b(0), large_body: b(1), combo_finish: b(2), back_attack: b(3), float: b(4), vanish: b(5), bio: b(6), wide: b(7), haste: b(8), regen: b(9), quick_barrier: b(10), aero_guard: b(11), combo_boost: b(12), double_strike: b(13) }
}

/// The effects now (the last applied read).
fn now() -> Play {
    unpack(PLAY.load(Ordering::Relaxed))
}

/// Checks the callees and sites, builds the Roxas stub and wraps the clear. False: no route-1
/// effect (the skills stay as Re:CoM leaves them; logged).
pub unsafe fn install(b: usize) -> bool {
    let have = |rva: u32, n: usize| std::slice::from_raw_parts((b + rva as usize) as *const u8, n).to_vec();
    for s in sites::ENEMY_SKILL_CALLEES.iter().chain(sites::ENEMY_SKILL_CHECKED).chain(sites::ENEMY_SKILL_STUB_SITES).chain(sites::ENEMY_SKILL_PATCHES.iter().map(|p| &p.site)) {
        if have(s.rva, s.expect.len()) != s.expect {
            log(&format!("enemy skills: {} is not as expected; their effects are not installed", s.name));
            return false;
        }
    }
    let site = &sites::ENEMY_SKILL_STUB_SITES[0];
    let at = b + site.rva as usize;
    let back = at + site.expect.len();
    // Placed with its return left 0, then the return written for where it landed.
    let Some(stub) = super::hook::place_near(at, &se::double_strike_stub(back as u64 + 20, back as u64)) else {
        log("enemy skills: no memory for Roxas' stub; their effects are not installed");
        return false;
    };
    let code = se::double_strike_stub(stub as u64, back as u64);
    let placed = std::slice::from_raw_parts((stub + 16) as *const u8, 4).to_vec();
    if !super::hook::patch(stub + 16, &placed, &code[16..20], "Roxas stub's return") {
        return false;
    }
    STUB.store(stub, Ordering::Release);
    let hooks = [super::hook::Hook { site: &sites::ENEMY_SKILL_HOOKS[0], ours: clear as *const () as usize, original: &ORIG_CLEAR }];
    if !super::hook::install_all(b, &hooks) {
        log("enemy skills: the clear 0x140161E10 isn't hooked; their effects are not installed");
        return false;
    }
    INSTALLED.store(true, Ordering::Release);
    install_rewrite(b);
    log(&format!(
        "enemy skills: route 1 (play-time setters) at Combat = KH1 in Sora's story: {}; Re:CoM's numbers switched per battle: {}",
        se::BUILT.iter().map(|(c, _)| crate::enemy_skills::of_card(*c).map_or("?", |s| s.name)).collect::<Vec<_>>().join(", "),
        sites::ENEMY_SKILL_PATCHES.iter().map(|p| p.site.name).chain(sites::ENEMY_SKILL_STUB_SITES.iter().map(|s| s.name)).collect::<Vec<_>>().join(", ")
    ));
    true
}

fn installed() -> bool {
    INSTALLED.load(Ordering::Acquire)
}

/// Stage 3: the stub block near the rewrite and the rewrite's wrapper. A failure leaves route 2 off
/// (logged); route 1 stays.
unsafe fn install_rewrite(b: usize) {
    for s in sites::ENEMY_SKILL_REWRITE_SITES.iter().chain(sites::ENEMY_SKILL_REWRITE_CHECKED).chain(sites::ENEMY_SKILL_REWRITE_HOOK) {
        if std::slice::from_raw_parts((b + s.rva as usize) as *const u8, s.expect.len()) != s.expect {
            log(&format!("enemy skills: {} is not as expected; route 2 (the hit rewrite) is off", s.name));
            return;
        }
    }
    let near = b + sites::ENEMY_SKILL_REWRITE_HOOK[0].rva as usize;
    // The block's code depends on where it lands: placed once with a guess, then rebuilt in place.
    // The block's bytes are built in the Steam image's address space (`se::rewrite` holds Steam
    // VAs; nothing in the block is an absolute pointer), so a rel32 between the block and Re:CoM's
    // code is the same wherever ASLR put the exe (0.139.1: built from the running addresses, the
    // jumps back into the rewrite were truncated and landed outside the exe).
    let img = |a: usize| (a as i64 - b as i64 + 0x1_4000_0000) as u64;
    let (guess, _) = se::rewrite_block(img(near));
    let Some(at) = super::hook::place_data_near(near, &guess) else {
        log("enemy skills: no memory for the rewrite's stubs; route 2 is off");
        return;
    };
    let (code, offs) = se::rewrite_block(img(at));
    let placed = std::slice::from_raw_parts(at as *const u8, code.len()).to_vec();
    if !super::hook::patch(at, &placed, &code, "the rewrite's stubs") {
        return;
    }
    let _ = RW_OFFS.set(offs);
    RW_BLOCK.store(at, Ordering::Release);
    let hooks = [super::hook::Hook { site: &sites::ENEMY_SKILL_REWRITE_HOOK[0], ours: rewrite as *const () as usize, original: &ORIG_REWRITE }];
    if !super::hook::install_all(b, &hooks) {
        log("enemy skills: the rewrite 0x14015D5F0 isn't hooked; route 2 is off");
        return;
    }
    RW_INSTALLED.store(true, Ordering::Release);
    log(&format!(
        "enemy skills: route 2 (the hit rewrite 0x14015D5F0, once per equipped skill with its id in Sora's slot): {}; the owner's numbers in our stubs' cells while it runs (Zexion {} %, Lexaeus {} %, Demyx x{:.2}, Defender / Ursula x{:.2})",
        se::BUILT_3.iter().map(|(c, _)| crate::enemy_skills::of_card(*c).map_or("?", |s| s.name)).collect::<Vec<_>>().join(", "),
        se::rewrite::OWNER_CELLS[0],
        se::rewrite::OWNER_CELLS[1],
        se::rewrite::OWNER_CELLS[2] as f32 / 100.0,
        se::rewrite::OWNER_CELLS[3] as f32 / 100.0
    ));
}

/// The cells our rewrite stubs read: the owner's while our wrapper runs Sora's skills, Re:CoM's
/// otherwise.
unsafe fn cells(owner: bool) {
    let at = RW_BLOCK.load(Ordering::Acquire);
    if at == 0 {
        return;
    }
    let v = if owner { se::rewrite::OWNER_CELLS } else { se::rewrite::RECOM_CELLS };
    for (k, x) in v.iter().enumerate() {
        wr(at + k * 4, *x);
    }
}

/// Re:CoM's battle manager ctor (once per battle, `win::battle::manager_ctor`): the switched sites
/// ours in a KH1 battle, Re:CoM's otherwise; the next KH1 frame applies the skills.
pub unsafe fn battle_start(kh1: bool) {
    GEN.fetch_add(1, Ordering::Relaxed);
    if !installed() {
        return;
    }
    let b = base();
    let mut written = 0;
    for pt in sites::ENEMY_SKILL_PATCHES {
        let at = b + pt.site.rva as usize;
        let (from, to) = if kh1 { (pt.site.expect, pt.new) } else { (pt.new, pt.site.expect) };
        let now = std::slice::from_raw_parts(at as *const u8, to.len());
        if now == to {
            continue;
        }
        if super::hook::patch(at, from, to, pt.site.name) {
            written += 1;
        }
    }
    let stub = STUB.load(Ordering::Acquire);
    if stub != 0 {
        let s = &sites::ENEMY_SKILL_STUB_SITES[0];
        let at = b + s.rva as usize;
        let ours = se::double_strike_jump(at as u64, stub as u64);
        let (from, to): (&[u8], &[u8]) = if kh1 { (s.expect, &ours) } else { (&ours, s.expect) };
        let now = std::slice::from_raw_parts(at as *const u8, to.len());
        if now != to && super::hook::patch(at, from, to, s.name) {
            written += 1;
        }
    }
    let at = RW_BLOCK.load(Ordering::Acquire);
    if let (true, Some(offs)) = (at != 0 && RW_INSTALLED.load(Ordering::Acquire), RW_OFFS.get()) {
        for (s, off) in sites::ENEMY_SKILL_REWRITE_SITES.iter().zip(offs) {
            let site = b + s.rva as usize;
            let ours = se::jump_to(site as u64, (at + off) as u64, s.expect.len());
            let (from, to): (&[u8], &[u8]) = if kh1 { (s.expect, &ours) } else { (&ours, s.expect) };
            let now = std::slice::from_raw_parts(site as *const u8, to.len());
            if now != to && super::hook::patch(site, from, to, s.name) {
                written += 1;
            }
        }
        cells(false);
    }
    if KH1_BATTLE.swap(kh1, Ordering::Relaxed) != kh1 || written != 0 {
        line(|| format!("battle start: Re:CoM's enemy-card numbers {} ({written} site(s) written)", if kh1 { "the owner's for Sora's skills (KH1 battle)" } else { "Re:CoM's" }));
    }
}

/// Whether the skills act for this player now: installed, Combat = KH1, Sora's story, the main Sora.
unsafe fn live(pl: usize) -> bool {
    // 0.164.0: Riku's too (his story's skills, his save).
    installed() && mv::kh1_on() && ((mv::is_main_sora(pl) && !super::stats::riku_story()) || mv::kh1_riku(pl))
}

/// A KH1 frame of Sora (`win::movement::run_frame`): the first one of a player object or after a
/// change applies the skills. One atomic compare otherwise.
pub unsafe fn kh1_frame(pl: usize) {
    let gen = GEN.load(Ordering::Relaxed);
    if APPLIED_PL.load(Ordering::Relaxed) == pl && APPLIED_GEN.load(Ordering::Relaxed) == gen {
        return;
    }
    APPLIED_PL.store(pl, Ordering::Relaxed);
    APPLIED_GEN.store(gen, Ordering::Relaxed);
    apply(pl, false, "first KH1 frame");
}

/// The ability set changed (the camp's Abilities page) or Combat switched: applies what is equipped
/// and takes off what isn't, on the main player now.
pub unsafe fn refresh(why: &str) {
    GEN.fetch_add(1, Ordering::Relaxed);
    if !installed() {
        return;
    }
    let pl: usize = rd(va(base(), data::MAIN_PLAYER));
    if pl == 0 || !(mv::is_main_sora(pl) || mv::kh1_riku(pl)) {
        return;
    }
    apply(pl, true, why);
}

/// Writes the equipped skills' route-1 effects on `pl` (and Sora's side of the CBTL in battle).
/// `undo`: also takes off the effects no longer equipped (only ever ours at Combat = KH1).
unsafe fn apply(pl: usize, undo: bool, why: &str) {
    let play = if live(pl) { super::abilities::equipped_live().map(se::play).unwrap_or_default() } else { Play::default() };
    let was = now();
    PLAY.store(pack(&play), Ordering::Relaxed);
    let mask = if live(pl) { super::abilities::equipped_live().map(se::rewrite_mask).unwrap_or(0) } else { 0 };
    let mask_was = RW_MASK.swap(mask, Ordering::Relaxed);
    if !installed() || pl == 0 {
        return;
    }
    // Draw (field and battle): Re:CoM's `+0x54C` / `+0x550` (`0x140163D44`); taken off only when ours.
    if play.draw {
        wr(pl + draw::FIELD_54C, draw::ON_54C);
        wr(pl + draw::RADIUS, draw::ON);
    } else if undo && rd::<f32>(pl + draw::RADIUS) == draw::ON {
        wr(pl + draw::FIELD_54C, draw::OFF);
        wr(pl + draw::RADIUS, draw::OFF);
    }
    let battle = rd::<i32>(pl + p::BATTLE) != 0;
    if battle {
        let f: u32 = rd(pl + p::FLAGS_420);
        let plain = play.flags_420();
        let f2 = if undo { (f & !flag420::PLAIN) | plain } else { f | plain };
        if f2 != f {
            wr(pl + p::FLAGS_420, f2);
        }
        // The setters, once each (Aero Guard's makes its wind object at every call).
        let set = |rva: u32, bit: u32, on: bool| {
            let has = rd::<u32>(pl + p::FLAGS_420) & bit != 0;
            if on && !has {
                func::<SetFn>(rva)(pl, 1);
            } else if undo && !on && has {
                func::<SetFn>(rva)(pl, 0);
            }
        };
        set(VANISH, flag420::VANISH, play.vanish);
        set(AERO, flag420::AERO_GUARD, play.aero_guard);
        let c: usize = rd(va(base(), CBTL_PTR));
        if c != 0 && rd::<u32>(c + cbtl::FLAGS) & cbtl::ENDING != cbtl::ENDING {
            let flags: u32 = rd(c + cbtl::FLAGS);
            let mut nf = flags;
            // Each counter starts at 0 with its flag (`0x1401632C6`, `0x1401633D3`).
            for (on, bit, count) in [(play.bio, cbtl::BIO, cbtl::BIO_COUNT), (play.regen, cbtl::REGEN, cbtl::REGEN_COUNT)] {
                if on && flags & bit == 0 {
                    nf |= bit;
                    wr(c + count, 0i32);
                } else if undo && !on && flags & bit != 0 {
                    nf &= !bit;
                }
            }
            if nf != flags {
                wr(c + cbtl::FLAGS, nf);
            }
        }
    }
    if play != was || undo || mask != mask_was {
        line(|| {
            format!(
                "{why}: {} ({}) {:?}; route 2 [{}]",
                if battle { "battle" } else { "field" },
                if live(pl) { "Combat = KH1, Sora's story" } else { "not live: nothing applied" },
                play,
                se::rewrite_ids(mask).map(|c| format!("{c:#x}")).collect::<Vec<_>>().join(" ")
            )
        });
    }
}

/// Re:CoM's `CBTL::resetNowEnchant` (`0x140161E10(CBTL, side)`): Re:CoM's clear, then Sora's
/// equipped skills back (side 1, a KH1 battle, not the battle's end, which marks the next player).
pub unsafe extern "win64" fn clear(c: usize, side: i32) {
    orig::<ClearFn>(&ORIG_CLEAR)(c, side);
    if side != 1 || !KH1_BATTLE.load(Ordering::Relaxed) || c == 0 {
        return;
    }
    if rd::<u32>(c + cbtl::FLAGS) & cbtl::ENDING == cbtl::ENDING {
        GEN.fetch_add(1, Ordering::Relaxed);
        return;
    }
    let pl: usize = rd(va(base(), data::MAIN_PLAYER));
    if pl != 0 && live(pl) {
        apply(pl, false, "after Re:CoM's Sora-side clear");
    }
}

/// Wight Knight's Float: the factor on Sora's gravity (one atomic load).
pub fn gravity_factor() -> f32 {
    se::gravity_factor(&now())
}

/// A KH1 swing starts (`win::attack::apply_row`, after its motion request, before the Damage =
/// Re:CoM AP): Card Soldier's haste through Re:CoM's own motion speed (`0x1401F74D0`, as its row
/// pick does at `0x1401F7A27`, with the owner's 1.25 for its 2.0); Saïx's bonus from 0 at a combo's
/// first swing.
pub unsafe fn swing_started(pl: usize, counter: u8) {
    let f: u32 = rd(pl + p::FLAGS_420);
    if f & (flag420::HASTE | flag420::COMBO_BOOST) == 0 {
        return;
    }
    if f & flag420::HASTE != 0 {
        func::<SpeedFn>(MOTION_SPEED)(pl, numbers::HASTE);
    }
    wr(pl + BONUS, se::combo_boost_before(counter, f, rd(pl + BONUS)));
}

/// Guard Armor on a KH1 swing (`win::attack::pair_dispatch`): KH1's sphere radii × Re:CoM's own
/// wide scale over its base, read from the weapon's collider (`0x140172690`'s field) while Re:CoM's
/// `setWideAttack` flag is on it.
pub unsafe fn wide_factor(weapon: usize, shape: usize) -> f32 {
    if weapon == 0 || shape == 0 || rd::<u32>(weapon + W_BITS) & 1 == 0 {
        return 1.0;
    }
    se::wide_factor(true, se::collider_scale(rd(shape + 0x60), rd(shape + 0x64), rd(shape + 0x70), rd(shape + 0x74)))
}

/// Fat Bandit at Damage = KH1 (`win::attack::kh1_enemy_hit`): KH1's number for a hit Re:CoM's apply
/// doubled from behind is doubled too.
pub unsafe fn back_attack_kh1(pl: usize, carried: i32, d: i32) -> i32 {
    let w: usize = rd(pl + WEAPON);
    if w == 0 || rd::<u32>(w + W_BITS) & 0x10 == 0 {
        return d;
    }
    if se::back_hit(carried, rd::<i16>(w + W_POWER) as i32, rd::<i16>(w + W_BACK) as i32) {
        line(|| format!("Back Attack at Damage = KH1: a hit from behind (Re:CoM's {carried}): KH1's {d} -> {}", d.saturating_mul(2)));
        d.saturating_mul(2)
    } else {
        d
    }
}

/// Large Body's Guard (`win::attack::damage_check`, before Re:CoM's check, beside KH1's Aero): a
/// hit Re:CoM's own Large Body barrier would block (its setter's values for the call, its barrier
/// test `0x14016EED0` on the hit Re:CoM's apply recorded: the front 180°, its class mask) takes 10 %
/// less instead. The barrier fields are put back at once (no block).
pub unsafe fn large_body(pl: usize) {
    if !now().large_body || rd::<u16>(pl + HIT_PENDING) == 0 || rd::<i32>(pl + p::BATTLE) == 0 {
        return;
    }
    let v: i16 = rd(pl + HIT_VALUE);
    if v <= 0 {
        return;
    }
    let saved: (u32, u32, u32, f32) = (rd(pl + p::FLAGS_420), rd(pl + BARRIER_FLAGS), rd(pl + BARRIER_MASK), rd(pl + BARRIER_ANGLE));
    func::<SetFn>(BARRIER_SET)(pl, 1);
    let mut attack = [0u8; 0x80];
    let a = attack.as_mut_ptr() as usize;
    wr(a + 0x50, rd::<u32>(pl + HIT_CLASS));
    wr(a + 0x60, rd::<[f32; 4]>(pl + HIT_POINT));
    let front = func::<TestFn>(BARRIER_TEST)(a, pl + BODY) & 0xFF != 0;
    wr(pl + p::FLAGS_420, saved.0);
    wr(pl + BARRIER_FLAGS, saved.1);
    wr(pl + BARRIER_MASK, saved.2);
    wr(pl + BARRIER_ANGLE, saved.3);
    if front {
        let c = se::front_cut(v);
        wr(pl + HIT_VALUE, c);
        line(|| format!("Large Body: a hit to Sora's front (class {}) {v} -> {c}", rd::<u32>(pl + HIT_CLASS)));
    }
}

// ---- Stage 3, route 2: the hit rewrite ----

/// Re:CoM's hit rewrite (`0x14015D5F0(CBTL, record)`): for Sora's side in a KH1 battle with his slot
/// empty (Sora plays no card at KH1), Re:CoM's rewrite once per equipped skill of route 2, each with
/// its id in Sora's slot (`CBTL+0x1E8`, the global its code reads), the slot put back after; the
/// owner's numbers in our stubs' cells for those calls. Every other record: Re:CoM's, once.
pub unsafe extern "win64" fn rewrite(c: usize, rec: usize) -> u64 {
    let o: RewriteFn = orig(&ORIG_REWRITE);
    let mask = RW_MASK.load(Ordering::Relaxed);
    if mask == 0 || !KH1_BATTLE.load(Ordering::Relaxed) || rec == 0 || rd::<i32>(rec + se::rewrite::SIDE) != 1 {
        return o(c, rec);
    }
    let g: usize = rd(va(base(), CBTL_PTR));
    if g == 0 || rd::<u8>(g + se::rewrite::SLOT) >= 0x44 {
        return o(c, rec);
    }
    let slot: u8 = rd(g + se::rewrite::SLOT);
    let (ap0, kind0) = (rd::<i16>(rec + se::rewrite::AP), rd::<u32>(rec + se::rewrite::KIND));
    cells(true);
    let mut r = 0;
    for id in se::rewrite_ids(mask) {
        let before: u32 = rd(rec + se::rewrite::KIND);
        wr(g + se::rewrite::SLOT, id);
        r = o(c, rec);
        if id == 0x73 && rd::<u16>(rec + se::rewrite::MODE) == 1 {
            let after: u32 = rd(rec + se::rewrite::KIND);
            LEX_FROM.store(if after == se::rewrite::WARP && before != after { before + 1 } else { 0 }, Ordering::Relaxed);
        }
    }
    wr(g + se::rewrite::SLOT, slot);
    cells(false);
    let (ap1, kind1) = (rd::<i16>(rec + se::rewrite::AP), rd::<u32>(rec + se::rewrite::KIND));
    if ap1 != ap0 || kind1 != kind0 {
        line(|| format!("rewrite (mode {}, attack card {}, status {:#x}): AP {ap0} -> {ap1}, kind {kind0:#x} -> {kind1:#x}", rd::<u16>(rec + se::rewrite::MODE), rd::<u16>(rec + se::rewrite::ATTACK_CARD), rd::<u16>(rec + se::rewrite::STATUS)));
    }
    r
}

/// Sora's fields the record takes (`0x1401E66A0`): his handle, HP, max HP, the card battle object
/// and the situation `+0x630`.
const HANDLE: usize = 0x20;
const HP: usize = 0x42C;
const MAX_HP: usize = 0x430;
const CARD_BATTLE: usize = 0x3E8;
const SITUATION: usize = 0x630;
/// A hurt body's reaction kind (`+0x80`, the attack's `+0x44`, `0x14016F1C0`).
const PART_KIND: usize = 0x80;

/// A route-2 record for Sora's hit, run through the wrapper above: (AP, kind) after.
unsafe fn run_record(pl: usize, kind: u32, status: u16, ap: i32, situation: u32) -> Option<(i32, u32)> {
    if RW_MASK.load(Ordering::Relaxed) == 0 || !KH1_BATTLE.load(Ordering::Relaxed) || !RW_INSTALLED.load(Ordering::Acquire) || pl == 0 {
        return None;
    }
    let cb: usize = rd(pl + CARD_BATTLE);
    if cb == 0 || rd::<usize>(va(base(), CBTL_PTR)) == 0 {
        return None;
    }
    let mut rec = se::hit_record(rd(pl + HANDLE), kind, 0, status, ap, rd(pl + HP), rd(pl + MAX_HP), situation);
    let r = rec.as_mut_ptr() as usize;
    rewrite(cb, r);
    Some((rd::<i16>(r + se::rewrite::AP) as i32, rd(r + se::rewrite::KIND)))
}

/// Damage = KH1, a KH1 swing's hit (`win::attack::kh1_enemy_hit`): KH1's number through Re:CoM's
/// rewrite as Re:CoM's own swing AP goes (`0x1401E66A0`'s record: the swing's situation makes it an
/// attack card, a finisher's a "sleight" too); the reaction kind it writes (Drain, Confu-Strike, Warp
/// Break) on the hit, never a warp on a boss.
pub unsafe fn kh1_swing_hit(pl: usize, enemy: usize, part: usize, d: i32) -> i32 {
    let kind: u32 = rd(part + PART_KIND);
    let Some((ap, k)) = run_record(pl, kind, 0, d, rd(pl + SITUATION)) else { return d };
    let k = se::warp_kind(k, (k != kind).then_some(kind), &super::stats::rtti_name(base(), rd(enemy)).unwrap_or_default());
    if k != kind {
        wr(part + PART_KIND, k);
    }
    if ap != d || k != kind {
        line(|| format!("Damage = KH1 swing hit on {}: KH1's {d} -> {ap}, kind {kind:#x} -> {k:#x}", super::stats::class_label(enemy)));
    }
    ap
}

/// A KH1 spell's number through Re:CoM's rewrite as Re:CoM's own card / sleight goes (`initMagic`'s
/// record: situation 0, the card's or sleight's status bits `+9` / `+0xB`): Cure's heal at its cast
/// (both Damage settings: Re:CoM's Cure takes it in `initMagic`), and at Damage = KH1 each spell hit.
/// Battle only (the record needs Re:CoM's card battle).
pub unsafe fn spell_rewrite(pl: usize, spell: u8, level: u8, ap: i32) -> i32 {
    if RW_MASK.load(Ordering::Relaxed) == 0 {
        return ap;
    }
    let Some(status) = super::damage::spell_status(spell, level) else { return ap };
    match run_record(pl, 0, status, ap, 0) {
        Some((v, _)) => {
            if v != ap {
                line(|| format!("{}{level}: {ap} -> {v} (status {status:#x})", crate::magic::spell::NAMES.get(spell as usize).copied().unwrap_or("?")));
            }
            v
        }
        None => ap,
    }
}

/// Lexaeus "never bosses" at Damage = Re:CoM (`win::attack::enemy_hit`): a hit of Sora's waiting on a
/// boss with the kind Warp Break gave his swing takes back the kind it replaced.
pub unsafe fn warp_guard(enemy: usize) {
    let from = LEX_FROM.load(Ordering::Relaxed);
    if from == 0 {
        return;
    }
    let Some(body) = super::attack::waiting_body(enemy) else { return };
    // Only Sora's side makes a hurt body of an enemy take Re:CoM's warp kind in a KH1 battle.
    if rd::<u32>(body + PART_KIND) != se::rewrite::WARP {
        return;
    }
    let kind: u32 = rd(body + PART_KIND);
    let k = se::warp_kind(kind, Some(from - 1), &super::stats::rtti_name(base(), rd(enemy)).unwrap_or_default());
    if k != kind {
        wr(body + PART_KIND, k);
        line(|| format!("Warp Break: never on a boss ({}): kind {kind:#x} -> {k:#x}", super::stats::class_label(enemy)));
    }
}
