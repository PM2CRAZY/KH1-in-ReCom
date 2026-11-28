//! Riku's KH1 spell unlocks in his story (fork, 0.198.0; fork owner, 2026-11-26: the floors and the bosses combined, as Sora's
//! scheme; "dps de vencer malevola", "deixa o al unlock quieto", old saves "recalcula", the count rule "ss"). The testable part;
//! the runtime is `win::unlocks::riku_sync`. Game state: the workspace's `knowledge/ReCoM/recom-riku-story-progress.md`.
//!
//! - **Fire (level 1)**: Maleficent beaten (her enemy card `0x6D` obtained, B12).
//! - **The other six (level 1)**: one per floor reached, in KH1's order: Blizzard at B11, Thunder B10, Cure B9, Gravity B8, Stop
//!   B7, Aero B6. The floor is Re:CoM's (0, 0x35), the highest floor label reached (−12 = B12 … −1 = B1), never lowered by the
//!   game. Checked (decomp + exe).
//! - **-ra / -ga**: +1 per counted fight won ([`COUNTED`], 14), the k-th win giving [`UPGRADES`]`[k]` whatever boss it was (the
//!   worlds of B11-B8 and B7-B4 are in the player's order, so a per-boss table could give Cura before Cure). An upgrade of a spell
//!   not learned yet waits for it.
//! - Every signal is in Re:CoM's save, so an older save is recalculated from its progress; the levels are kept in the mod's save
//!   block (Riku's save's spell bytes, free there) and never lowered.

use crate::magic::spell::{AERO, BLIZZARD, CURE, FIRE, GRAVITY, STOP, THUNDER};

/// Spells (KH1's order).
pub const SPELLS: usize = 7;

/// Maleficent's enemy card (Riku's B12 boss): gives Fire. Checked (decomp + exe + files).
pub const FIRE_CARD: u16 = 0x6D;

/// A win's signal in the save.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Won {
    /// The boss's enemy card obtained (Re:CoM's discovery state ≥ 2, `0x14015B3F0`).
    Card(u16),
    /// A bit of Re:CoM's story int (0, 0x2A), set by the hall script after the battle room.
    Bit(u32),
}

/// Riku's hairs earned in his story (fork owner, 2026-11-27, verbatim: "modularidade e senso de rencompensa após esforço são
/// os pontos mais importântes para diversão, então por isso as nossas features de costméticos precisam ser desbloqueádas a
/// partir da narrativa ou de ações específicas nos jogos"; "um defeito MUITO comum dos jogos é liberar os cosméticos APÓS o
/// endgame ... precisa estar contida no loop principal"; to "BBS at Vexen (B10), KH2 at Riku Replica I (B8)": "perfeito,
/// sim!"). `hair`: the `riku_hair` setting (0 Re:CoM's, 1 KH2, 2 BBS); `bits`: Re:CoM's story int (0, 0x2A), whose 0x80 is
/// Vexen's hall win and 0x400 Riku Replica I's (Checked (decomp + exe + files), [`COUNTED`]).
pub fn hair_unlocked(hair: u8, bits: u32) -> bool {
    match hair {
        0 => true,
        1 => bits & HAIR_KH2_WIN != 0,
        _ => bits & HAIR_BBS_WIN != 0,
    }
}

/// Riku Replica I's hall bit: the KH2 hair. Vexen's: the BBS hair.
pub const HAIR_KH2_WIN: u32 = 0x400;
pub const HAIR_BBS_WIN: u32 = 0x80;

/// The 14 counted fights (names for the log). Maleficent (Fire), the Ansem tutorial (done for Riku at Combat = KH1, Stage 374)
/// and the final Ansem are not counted. Checked (decomp + exe + files: the reward table `0x14052D760`, the hall scripts' bits),
/// except Zexion's Dark Mode end (Assumption: it takes the same path).
pub const COUNTED: [(&str, Won); 14] = [
    ("Guard Armor", Won::Card(0x63)),
    ("Genie Jafar", Won::Card(0x69)),
    ("Parasite Cage", Won::Card(0x64)),
    ("Captain Hook", Won::Card(0x6C)),
    ("Hades", Won::Card(0x68)),
    ("Trickmaster", Won::Card(0x65)),
    ("Oogie Boogie", Won::Card(0x6A)),
    ("Ursula", Won::Card(0x6B)),
    ("Vexen", Won::Bit(0x80)),
    ("Riku Replica I", Won::Bit(0x400)),
    ("Lexaeus", Won::Card(0x73)),
    ("Darkside", Won::Card(0x66)),
    ("Zexion", Won::Card(0x75)),
    ("Riku Replica II", Won::Bit(0x8000_0000)),
];

/// The upgrade the k-th counted win gives (k = 1..14 at index 0..13): each spell twice, its -ra before its -ga, and in normal play
/// never before the spell's base (the floors' order: the spec's table).
pub const UPGRADES: [u8; 14] = [FIRE, BLIZZARD, THUNDER, CURE, GRAVITY, FIRE, STOP, AERO, CURE, STOP, GRAVITY, BLIZZARD, THUNDER, AERO];

/// Floors cleared from Re:CoM's floor label (−12 = B12 → 0, … −1 = B1 → 11); anything else (a fresh game's −666) → 0.
pub fn floors_cleared(label: i32) -> usize {
    if (-12..=-1).contains(&label) { (label + 12) as usize } else { 0 }
}

/// The levels from the progress: floor label, Maleficent beaten, counted wins.
pub fn levels(label: i32, maleficent: bool, wins: usize) -> [u8; SPELLS] {
    let a = floors_cleared(label);
    let mut out = [0u8; SPELLS];
    for s in 0..SPELLS {
        let base = if s == FIRE as usize { maleficent } else { a >= s };
        if base {
            let ups = UPGRADES.iter().take(wins.min(UPGRADES.len())).filter(|&&u| u as usize == s).count();
            out[s] = (1 + ups).min(3) as u8;
        }
    }
    out
}

/// The levels from the save's state: `label` (0, 0x35), `obtained(card)`, `bits` (0, 0x2A).
pub fn derive(label: i32, obtained: impl Fn(u16) -> bool, bits: u32) -> ([u8; SPELLS], usize) {
    let wins = COUNTED
        .iter()
        .filter(|(_, w)| match *w {
            Won::Card(c) => obtained(c),
            Won::Bit(b) => bits & b != 0,
        })
        .count();
    (levels(label, obtained(FIRE_CARD), wins), wins)
}

/// Whether a card's first pickup matters here.
pub fn tracked(card: u16) -> bool {
    card == FIRE_CARD || COUNTED.iter().any(|(_, w)| *w == Won::Card(card))
}

#[cfg(test)]
mod tests {
    #[test]
    fn hairs_are_earned_at_vexen_and_the_replica() {
        use super::hair_unlocked;
        // New game: only Re:CoM's hair.
        assert!(hair_unlocked(0, 0) && !hair_unlocked(1, 0) && !hair_unlocked(2, 0));
        // Vexen (0x80): BBS; Riku Replica I (0x400): KH2; the other's bit alone does not open it.
        assert!(hair_unlocked(2, 0x80) && !hair_unlocked(1, 0x80));
        assert!(hair_unlocked(1, 0x400) && !hair_unlocked(2, 0x400));
        assert!(hair_unlocked(1, 0x480) && hair_unlocked(2, 0x480));
    }

    use super::*;

    #[test]
    fn the_spec_table() {
        assert_eq!(levels(-666, false, 0), [0; 7], "a fresh game: nothing");
        assert_eq!(levels(-12, false, 0), [0; 7], "B12 before Maleficent");
        assert_eq!(levels(-12, true, 0), [1, 0, 0, 0, 0, 0, 0], "Fire after Maleficent");
        assert_eq!(levels(-11, true, 0), [1, 1, 0, 0, 0, 0, 0], "Blizzard at B11");
        assert_eq!(levels(-8, true, 6), [3, 2, 2, 2, 2, 0, 0], "B8, Replica I beaten: Firaga");
        assert_eq!(levels(-1, true, 14), [3; 7], "all at B1");
        assert_eq!(levels(-6, true, 0), [1; 7], "Aero at B6");
    }

    #[test]
    fn each_spell_twice_ra_before_ga() {
        for s in 0..SPELLS as u8 {
            assert_eq!(UPGRADES.iter().filter(|&&u| u == s).count(), 2, "spell {s}");
        }
        // in normal play no upgrade waits: the floor at which the k-th win lands (the spec's table) already has its spell
        let floor_of_win = [-11, -10, -10, -9, -8, -8, -7, -6, -5, -4, -4, -3, -3, -2];
        for (k, (&s, &label)) in UPGRADES.iter().zip(&floor_of_win).enumerate() {
            assert!(levels(label, true, k)[s as usize] >= 1, "win {} ({s}) at floor {label}", k + 1);
        }
    }

    #[test]
    fn waits_for_the_base_and_never_passes_3() {
        // 14 wins with Riku still on B12: only Fire (and its two upgrades)
        assert_eq!(levels(-12, true, 14), [3, 0, 0, 0, 0, 0, 0]);
        // Blizzard's upgrades wait until B11, then apply at once
        assert_eq!(levels(-11, true, 14)[BLIZZARD as usize], 3);
        assert_eq!(levels(-1, true, 99), [3; 7]);
    }

    #[test]
    fn derive_counts_cards_and_bits() {
        let cards = [FIRE_CARD, 0x63, 0x69, 0x73];
        let (l, wins) = derive(-8, |c| cards.contains(&c), 0x80 | 0x400);
        assert_eq!(wins, 5, "three cards (Guard Armor, Jafar, Lexaeus) and two bits (Vexen, Replica I)");
        assert_eq!(l, levels(-8, true, 5));
        let (_, w) = derive(-1, |_| false, 0x8000_0000);
        assert_eq!(w, 1, "Replica II's bit");
        assert!(tracked(0x6D) && tracked(0x75) && !tracked(0x71), "Vexen's card is not his signal");
    }
}
