//! Re:CoM's spell effects in the field for KH1 casts (`crate::magic::field_fx`,
//! `docs/recom-m7-magic.md` § Spell effects in the field).
//!
//! Re:CoM's spell effects come with Sora's battle set (PLAYER group 1001 and the EFFECT groups it
//! links), which Re:CoM loads only for its battle scene, or for the whole session under its system
//! byte (0, 0x60). At Combat = KH1 in Sora's story the field does what Re:CoM does under that
//! byte, with Re:CoM's own calls:
//!
//! - **Room load:** `CGameSubField` state 1's tail jump to `0x140211AA0` (the player's group) goes
//!   to [`field_load`], which makes that call and then `0x140211960(0)`, the call
//!   `CGameSubBattle`'s state 1 makes. The room's next state waits for the loader queue as for
//!   the player's group.
//! - **Camp:** the camp openings' `0x140211560(0)` (the battle set's unload) go to [`camp_unload`],
//!   which keeps the set (as Re:CoM does under the byte) and otherwise calls Re:CoM's.
//! - **Combat switched to KH1 in the camp:** [`setting_changed`] queues the set at once.
//!
//! Nothing here runs per frame: once per field room load, camp opening and Combat switch.

use super::movement::{func, rd};
use super::{base, log, va};
use crate::magic::field_fx as fx;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

static INSTALLED: AtomicBool = AtomicBool::new(false);
static LINES: AtomicU32 = AtomicU32::new(40);

type CharFn = unsafe extern "win64" fn(u32);

fn note(line: impl FnOnce() -> String) {
    if LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("magic fx: {}", line()));
    }
}

fn wanted() -> bool {
    fx::wanted(super::settings().combat == crate::settings::Mode::Kh1, super::stats::riku_story())
}

/// Re:CoM's battle-set load (`0x140211960(0)`: the char from the world, its set when not loaded).
unsafe fn load_set(why: &str) {
    func::<CharFn>(fx::LOAD_RVA)(0);
    let set: u32 = rd(va(base(), fx::SET_GLOBAL));
    note(|| format!("{why}: Re:CoM's battle set (PLAYER group {set}) loaded or queued with 0x140211960 for KH1 casts' effects"));
}

/// `CGameSubField` state 1's tail call: Re:CoM's `0x140211AA0(ch)`, then the battle set.
pub unsafe extern "win64" fn field_load(ch: u32) {
    func::<CharFn>(fx::PLAYER_GROUP_RVA)(ch);
    if wanted() {
        load_set("field room");
    }
    // 0.184.3 (fork owner, 2026-11-23: "fora do combate só a firaga solta projetil"): in Riku's story the field never
    // loads the spell effects (Dark Firaga's are his own `EF1500.EFF`): Sora's 1001 for his KH1 spells, which stays
    // resident once loaded (`battle_load`).
    riku_sora_set("field room");
}

/// 0.184.2: `0x1402A53E0(1, group)`: a PLAYER group is loaded; `0x14023CCE0(group)`: queue it (Checked (decomp): the
/// battle-set loader's own pair).
const GROUP_LOADED_RVA: u32 = 0x002A_53E0;
const GROUP_LOAD_RVA: u32 = 0x0023_CCE0;
/// Riku's battle set (`0x140211960`: chars 3 / 4, `0x1407C7AE4` = 0x44D; Checked (decomp)).
const RIKU_SET: u32 = 1101;
static RIKU_LINES: AtomicU32 = AtomicU32::new(20);

/// 0.184.2 (fork owner, 2026-11-23, on Riku's spells: "não tem efeito, som projetil cadê o resto?"): Re:CoM's spell
/// effects (`fire_all` … `airo_all`) and the sounds riding them come with Sora's battle set, PLAYER 1001, whose LINK
/// rows alone load EFFECT 2000..2063 (`docs/recom-m7-magic.md` § Spell effects in the field; Checked (exe)); Riku's
/// set 1101 doesn't link them, so his KH1 spells drew nothing. `CGameSubBattle` state 1's tail call: Re:CoM's
/// `0x140211960(ch)`, then, when it chose Riku's set and his KH1 spells are on, Sora's 1001 too (unless loaded). The
/// room waits on the loader queue as for Re:CoM's set. It is not unloaded (Re:CoM's unloader takes the set the
/// world chose): it stays resident, as Re:CoM keeps 1001 under its system byte 0x60; the next battle finds it loaded.
pub unsafe extern "win64" fn battle_load(ch: u32) {
    func::<CharFn>(fx::LOAD_RVA)(ch);
    if rd::<u32>(va(base(), fx::SET_GLOBAL)) == RIKU_SET {
        riku_sora_set("Riku's battle");
    }
}

/// Riku's KH1 spells are on (his story, `riku_kh1_movement`, Combat = KH1, the cast installed).
fn riku_wanted() -> bool {
    // SAFETY: atomics and settings reads.
    super::stats::riku_story() && unsafe { super::movement::riku_on() && super::movement::kh1_on() } && super::magic::installed()
}

/// Sora's battle set (PLAYER 1001, the spell effects) queued when Riku's KH1 spells are on and it isn't loaded.
unsafe fn riku_sora_set(why: &str) {
    if !riku_wanted() {
        return;
    }
    type LoadedFn = unsafe extern "win64" fn(i32, i32) -> u64;
    if func::<LoadedFn>(GROUP_LOADED_RVA)(1, fx::SORA_SET as i32) & 0xFF != 0 {
        return;
    }
    func::<CharFn>(GROUP_LOAD_RVA)(fx::SORA_SET);
    if RIKU_LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("magic fx: {why}: Sora's battle set (PLAYER group 1001: the spell effects) queued too, for Riku's KH1 spells"));
    }
}

/// The camp openings' unload call: Re:CoM's `0x140211560(ch)`, unless the field keeps the set.
pub unsafe extern "win64" fn camp_unload(ch: u32) {
    if wanted() {
        note(|| "camp opened: the battle set stays loaded (Re:CoM's rule under its system byte 0x60)".into());
        return;
    }
    func::<CharFn>(fx::UNLOAD_RVA)(ch);
}

/// Combat switched in the camp's Config: at KH1, the set starts loading now (Re:CoM's queue; the
/// camp is on the main thread). At Re:CoM it stays until Re:CoM's next unload (a room change or
/// camp opening), as under Re:CoM's own byte.
pub fn setting_changed() {
    if !INSTALLED.load(Ordering::Acquire) || !(wanted() || riku_wanted()) {
        return;
    }
    // SAFETY: a static pointer of the image; Re:CoM's own queue call, from the camp's update
    // (main thread), only while Sora is in a room (not from a title-screen Config).
    unsafe {
        let pl: usize = rd(va(base(), crate::sites::data::MAIN_PLAYER));
        if super::movement::is_main_sora(pl) {
            load_set("Combat = KH1");
        }
        // 0.184.3: Riku's KH1 spells.
        if super::movement::is_main_riku(pl) {
            riku_sora_set("Combat = KH1");
        }
    }
}

/// Retargets the field state's tail jump and the two camp unload calls through one jump placed
/// near them (`sites::MAGIC_FX_BRANCHES`; each branch's bytes checked by `patch`, their context
/// by the start-up check). All or nothing: the room load first, so a camp never keeps a set the
/// field doesn't load.
pub unsafe fn install(b: usize) -> bool {
    let br = crate::sites::MAGIC_FX_BRANCHES;
    let jmp_field = crate::hookplan::abs_jmp(field_load as *const () as u64);
    let jmp_camp = crate::hookplan::abs_jmp(camp_unload as *const () as u64);
    let (Some(t_field), Some(t_camp)) = (super::hook::place_near(b + br[0].rva as usize, &jmp_field), super::hook::place_near(b + br[1].rva as usize, &jmp_camp)) else {
        log("magic fx: no memory near the exe; spell effects only in battle (Re:CoM's)");
        return false;
    };
    let field_at = b + br[0].rva as usize;
    let Some(new_jmp) = crate::hookplan::rel_jmp(field_at as u64, t_field as u64) else { return false };
    let mut calls = Vec::new();
    for c in &br[1..] {
        let at = b + c.rva as usize;
        let Some(call) = crate::hookplan::call_rel32(at as u64, t_camp as u64) else { return false };
        calls.push((at, c, call));
    }
    // Every branch must still be Re:CoM's before the first write.
    let ours_ok = br.iter().all(|s| std::slice::from_raw_parts((b + s.rva as usize) as *const u8, s.expect.len()) == s.expect);
    if !ours_ok {
        log("magic fx: a branch is not Re:CoM's; spell effects only in battle (Re:CoM's)");
        return false;
    }
    if !super::hook::patch(field_at, br[0].expect, &new_jmp, br[0].name) {
        return false;
    }
    for (at, c, call) in calls {
        if !super::hook::patch(at, c.expect, &call, c.name) {
            // The room load stays: Re:CoM's camp then unloads the set and the next room reloads it.
            log("magic fx: a camp unload call not retargeted; the set reloads with the next room");
        }
    }
    INSTALLED.store(true, Ordering::Release);
    true
}

/// 0.184.2: retargets `CGameSubBattle` state 1's tail jump to [`battle_load`] (`sites::RIKU_FX_BRANCH`, its bytes
/// checked by `patch`). Not fatal: without it Riku's spells draw nothing, as before.
pub unsafe fn install_riku(b: usize) {
    let s = &crate::sites::RIKU_FX_BRANCH[0];
    let at = b + s.rva as usize;
    let Some(t) = super::hook::place_near(at, &crate::hookplan::abs_jmp(battle_load as *const () as u64)) else {
        log("magic fx: no memory near the battle state's jump; Riku's spells draw no effects");
        return;
    };
    let Some(j) = crate::hookplan::rel_jmp(at as u64, t as u64) else { return };
    if !super::hook::patch(at, s.expect, &j, s.name) {
        log("magic fx: the battle state's jump to 0x140211960 is not Re:CoM's; Riku's spells draw no effects");
    }
}
