//! Sora's Halloween Town costume in every world (fork, 0.170.0; fork owner, 2026-11-23: "para o Sora
//! quero que ele também tenha trajes desbloqueáveis de acordo como a história, então se o sora for
//! para a halloween town por exemplo, o jogador pode livremente alterar para este traje a qualquer
//! momento"; of KH1's world costumes, "Só Halloween").
//!
//! - Re:CoM (Checked (exe)): Sora is char 1 (group 1, pack `PL0001`) and, in Halloween Town, char 2
//!   (group 11, pack `PL0011`): the char → group table `0x1405A6910` (0, 1, 11, …) and the char →
//!   pack name table `0x1407C6338`. Both chars share one action table (`0x1407C5E80`), and the two
//!   packs hold the same 69 clip names (Checked (files)): char 1 on group 11 / `PL0011` plays every
//!   row as in Halloween Town. The camp's menu model takes its group from the byte `0x1400E4705`
//!   (1; Halloween Town's 11 is `0x1400E4706`).
//! - The change is made when Re:CoM queues the player's group (each room / battle build) and from
//!   the camp; it shows from the next room or battle.
//! - Unlock: the save block's spare bit [`HALLOWEEN_SEEN`], set at any frame of Sora as char 2.

use super::movement::{rd, wr};
use super::{log, va};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// The spare byte's bit for "Sora reached Halloween Town" (bits 1 / 2 / 4 are taken).
pub const HALLOWEEN_SEEN: u8 = 0x10;
const GROUP_TABLE: u64 = 0x1_405A_6910;
const PACK_NAMES: u64 = 0x1_407C_6338;
const MENU_BYTE: u64 = 0x1_400E_4705;
const CHAR: usize = 0x398;
const SORA: u32 = 1;
const HALLOWEEN: u32 = 11;

/// The tables point Sora at the Halloween model now.
static ON: AtomicBool = AtomicBool::new(false);
/// Re:CoM's own pack name pointer for char 1 ("PL0001"), read before our first write.
static OWN_NAME: AtomicU64 = AtomicU64::new(0);

/// Sora has reached Halloween Town in this save, or the debug key.
pub unsafe fn unlocked() -> bool {
    if super::settings().all_unlock {
        return true;
    }
    let save: usize = rd(va(super::base(), crate::keyblades::SAVE_PTR));
    save != 0 && !super::stats::riku_story() && rd::<u8>(save + crate::mp::BLOCK_FIRST + crate::mp::off::SPARE) & HALLOWEEN_SEEN != 0
}

/// One frame of Sora (his request bits): Halloween Town reached (char 2) kept in the save.
pub unsafe fn frame(pl: usize) {
    if rd::<u32>(pl + CHAR) != 2 {
        return;
    }
    let save: usize = rd(va(super::base(), crate::keyblades::SAVE_PTR));
    if save == 0 || super::stats::riku_story() {
        return;
    }
    let at = save + crate::mp::BLOCK_FIRST + crate::mp::off::SPARE;
    let b: u8 = rd(at);
    if b & HALLOWEEN_SEEN == 0 {
        wr(at, b | HALLOWEEN_SEEN);
        log("sora costume: Halloween Town reached: kept in Sora's save (the Halloween Town costume unlocks)");
    }
}

/// Points Sora (char 1) at the Halloween model (`on`) or back at his own: the group, the motion pack
/// name and the menu model's byte, each written only from the value it should have.
pub unsafe fn apply(b: usize, on: bool) {
    if ON.load(Ordering::Relaxed) == on {
        return;
    }
    let g_at = va(b, GROUP_TABLE) + 4;
    let n_at = va(b, PACK_NAMES) + 8;
    let sora_name: u64 = rd(va(b, PACK_NAMES) + 8);
    let hw_name: u64 = rd(va(b, PACK_NAMES) + 16);
    if OWN_NAME.load(Ordering::Relaxed) == 0 {
        if sora_name == hw_name {
            log("sora costume: char 1's pack name is already Halloween's before ours: Sora keeps his model");
            return;
        }
        OWN_NAME.store(sora_name, Ordering::Relaxed);
    }
    let own_name = OWN_NAME.load(Ordering::Relaxed);
    let (from_g, to_g) = if on { (SORA, HALLOWEEN) } else { (HALLOWEEN, SORA) };
    let (from_n, to_n) = if on { (own_name, hw_name) } else { (hw_name, own_name) };
    if sora_name != from_n {
        log(&format!("sora costume: Sora's motion pack name pointer is {sora_name:#x}, not {from_n:#x}: Sora keeps his model"));
        return;
    }
    if !super::hook::patch(n_at, &from_n.to_le_bytes(), &to_n.to_le_bytes(), "char -> motion pack name (Sora)") {
        return;
    }
    if !super::hook::patch(g_at, &from_g.to_le_bytes(), &to_g.to_le_bytes(), "char -> group table (Sora)") {
        super::hook::patch(n_at, &to_n.to_le_bytes(), &from_n.to_le_bytes(), "char -> motion pack name (Sora, back)");
        return;
    }
    if !super::hook::patch(va(b, MENU_BYTE), &[from_g as u8], &[to_g as u8], "menu model group (Sora)") {
        log("sora costume: the menu model's byte is not Re:CoM's 1 there: the menu keeps Sora's model");
    }
    ON.store(on, Ordering::Relaxed);
    log(&format!("sora costume: {} (char 1 -> group {to_g}); shown from the next room or battle", if on { "Halloween Town" } else { "Default Attire" }));
}

/// The costume as it should be now (when Re:CoM queues the player's group, before its queue reads
/// the table): the setting, if unlocked (else Default Attire, the ini rewritten).
pub unsafe fn refresh(b: usize) {
    if super::stats::riku_story() {
        apply(b, false);
        return;
    }
    let s = super::settings();
    if s.sora_halloween && !unlocked() {
        log("sora costume: Halloween Town chosen, but this save hasn't reached it yet: Default Attire");
        let _ = super::store_item(crate::menu::Item::SoraCostume, 0);
        apply(b, false);
        return;
    }
    apply(b, s.sora_halloween);
}
