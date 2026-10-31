//! M6 stage 2, no deck for Sora in battle at the KH1 setting (decision 29;
//! `docs/recom-m6-battles.md` § Stage 2): the testable part. The runtime is `win/deck.rs`.
//!
//! At Combat = KH1, in Sora's story, outside minigame rooms:
//!
//! - **The battle revolver is empty.** Re:CoM's revolver build (`0x1401F43D0`) is wrapped: it
//!   first builds the revolver with Re:CoM's own constructor (`0x140164D40`) from a list that
//!   holds only Re:CoM's end-of-deck entry, exactly what Re:CoM builds for a saved deck of 0
//!   cards. The loader then gives it its Reload card, as always. The build itself then sees a
//!   built revolver and only runs its tail.
//! - **No Friend-card drops** (`0x14017E190`, whose roll gives only Friend cards, which only ever
//!   go into the battle revolver): it returns as Re:CoM's own failed roll does.
//! - **The deck HUD is hidden through Re:CoM's own event-hide mask** (system variable (1, 0xE2)):
//!   the cockpit's and Sora's cards' reads of it go through a stub that adds bit 2 (cockpit) or
//!   0x20 (side-1 cards) for this battle's empty revolver only. Re:CoM's own hide branches run.
//!
//! The collection (drops of normal cards, rewards, shops, chests) is never touched.

/// Re:CoM's `CPlayer` fields. Checked (decomp `0x1401F43D0`, `0x1401EA1A0`).
pub mod player {
    /// The battle revolver (`CCardRevolver`), battle only.
    pub const REVOLVER: usize = 0x3F0;
    /// Nonzero in minigame rooms (1200..1255): Re:CoM's minigame deck is kept there.
    pub const MINIGAME: usize = 0x394;
}

/// `CCardRevolver` fields. Checked (decomp `0x140164D40`, `0x14016C970`).
pub mod revolver {
    /// 1 = the player's side.
    pub const SIDE: usize = 0x5C;
    /// Slot count of page 0 (cards + the Reload card).
    pub const SLOTS: usize = 0x6C;
    /// Cards on page 0, not counting the Reload card (the hand count `0x1401EA1A0` reads).
    pub const CARDS: usize = 0x70;
    /// Flags: bit 0 = the Reload card is in; 0x80 Riku's story; 0x800 minigame.
    pub const FLAGS: usize = 0x98;
    /// Its size, as Re:CoM allocates it (`0x1404ACE54(0x280)`).
    pub const SIZE: usize = 0x280;
}

/// `CBattleCard`'s side byte (`0x14014F8C0` stores its 5th argument there; 1 = the player's).
pub const CARD_SIDE: usize = 0x26C;
pub const SIDE_PLAYER: u8 = 1;

/// Re:CoM's end-of-deck entry: id byte 0xFF (the loader stops at 0 or >= 0xCB). `0x1401F43D0`
/// appends exactly this (its high nibble comes from the byte at `0x1405A02D0`, 0). Checked (exe).
pub const END_OF_DECK: u32 = 0x0000_00FF;
/// The list the wrapper hands the constructor: the end entry (and a zero after it, unread).
pub const EMPTY_LIST: [u32; 2] = [END_OF_DECK, 0];
/// "No card selected" (what Re:CoM passes for its fixed decks).
pub const SELECTED_NONE: u32 = 0xFFFF_FFFF;

/// Re:CoM's HUD-hide mask, system variable (1, 0xE2), and the bits its readers test. Checked
/// (decomp `0x1401689F0`, `0x140150340`).
pub mod mask {
    pub const TABLE: u32 = 1;
    pub const INDEX: u32 = 0xE2;
    /// The player's cockpit (frame and count digits): `test al,2`.
    pub const COCKPIT: u32 = 2;
    /// Every side-1 `CBattleCard` (the Reload card and its gauge included).
    pub const PLAYER_CARDS: u32 = 0x20;
}

/// Event flags (table 1) the deck tutorials set, logged at each battle start. Checked (exe +
/// decomp, setter sites): 0x11 hand empty (`CAtelTutorial`), 0x12 reload / page switch
/// (`0x140167910`), 0x13 deal done (`0x140169710`), 0x14 / 0x16 a player card done, 0x15 stock
/// (`0x14016DB60`), 0x17 revolver scroll (`0x14016C580`), 0x18 the hood held in Sora's target
/// (`CTutorialHood` `0x1403A97E0`), 0x1B Leon's reaction (`CTutorialLeon` `0x1403A9F40`).
pub const FLAG_TABLE: u32 = 1;
pub const TUTORIAL_FLAGS: [u32; 9] = [0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x1B];
/// The tutorial-deck flag: with it Re:CoM deals its fixed tutorial deck (`0x1407C2450`).
pub const FLAG_TUTORIAL_DECK: u32 = 0x2C;

/// Why a battle keeps Re:CoM's deck.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keep {
    /// Combat = Re:CoM.
    Setting,
    /// Riku's story (decision 6).
    Riku,
    /// Not the main player's `CPlayerSora`.
    NotSora,
    /// A minigame room: Re:CoM's minigame deck.
    Minigame,
}

impl Keep {
    pub fn as_str(self) -> &'static str {
        match self {
            Keep::Setting => "Combat = Re:CoM",
            Keep::Riku => "Riku's story",
            Keep::NotSora => "not the main CPlayerSora",
            Keep::Minigame => "minigame room",
        }
    }
}

/// What the revolver build does in this battle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Build {
    /// Build the empty revolver first.
    Empty,
    /// Re:CoM's own build.
    Recom(Keep),
}

/// What the decision reads at the revolver build (battle start).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Battle {
    pub kh1: bool,
    pub riku_story: bool,
    pub main_sora: bool,
    pub minigame: bool,
}

pub fn decide(b: &Battle) -> Build {
    if !b.kh1 {
        Build::Recom(Keep::Setting)
    } else if b.riku_story {
        Build::Recom(Keep::Riku)
    } else if !b.main_sora {
        Build::Recom(Keep::NotSora)
    } else if b.minigame {
        Build::Recom(Keep::Minigame)
    } else {
        Build::Empty
    }
}

/// Friend-card drops are skipped at KH1 in Sora's story outside minigame rooms (the same
/// battles that get the empty revolver; decided from the setting at the drop, so it doesn't
/// depend on whether the battle setup's drops come before the player's revolver build).
pub fn drops_off(kh1: bool, riku_story: bool, minigame: bool) -> bool {
    kh1 && !riku_story && !minigame
}

/// The bits the cockpit's mask read adds: bit 2, only for this battle's empty revolver.
pub fn cockpit_bits(rev: usize, empty: usize) -> u32 {
    if rev != 0 && rev == empty {
        mask::COCKPIT
    } else {
        0
    }
}

/// The bits a card's mask read adds: 0x20, only for a side-1 card while the main player holds
/// this battle's empty revolver (side-1 cards are only made by Sora's revolver and CBTL's side 1,
/// both battle only; enemy cards are never touched).
pub fn card_bits(side: u8, player_rev: usize, empty: usize) -> u32 {
    if side == SIDE_PLAYER && empty != 0 && player_rev == empty {
        mask::PLAYER_CARDS
    } else {
        0
    }
}

/// A deck reader logged once per battle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reader {
    /// `0x1401EA1A0`, from `CAtelTutorial`: "hand empty" sets (1, 0x11).
    HandCheck,
    /// `0x1401EA1E0`, from the event script VM: scrolls the revolver.
    ScriptScroll,
    /// `0x140167CF0`, from the enemy card value `0x140182500` and Oogie's `0x1403D91C0`.
    ValueSearch,
}

pub const READERS: [Reader; 3] = [Reader::HandCheck, Reader::ScriptScroll, Reader::ValueSearch];

impl Reader {
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn describe(self) -> &'static str {
        match self {
            Reader::HandCheck => "the tutorial's hand-empty check (0x1401EA1A0, CAtelTutorial): 'empty', so it sets event flag (1, 0x11)",
            Reader::ScriptScroll => "an event script's revolver scroll (0x1401EA1E0): scrolls the Reload card only",
            Reader::ValueSearch => "a revolver value search (0x140167CF0, enemy card value / Oogie)",
        }
    }
}

/// Whether a reader logs now: its last logged battle differs from this one, and lines are left.
pub fn log_reader(last_battle: u32, battle: u32, lines: u32) -> bool {
    battle != 0 && last_battle != battle && lines < READER_LOG_LINES
}

/// The set flags of `flags` (index, set), as `0x11 0x13`, or `none`.
pub fn flags_str(flags: &[(u32, bool)]) -> String {
    let set: Vec<String> = flags.iter().filter(|f| f.1).map(|f| format!("{:#x}", f.0)).collect();
    if set.is_empty() {
        "none".into()
    } else {
        set.join(" ")
    }
}

/// The battle-start line for a battle without a deck.
pub struct EmptyLine<'a> {
    pub rev: usize,
    pub cards: i16,
    pub slots: i16,
    pub rev_flags: u32,
    pub tutorial_deck: bool,
    pub hud: &'a str,
    pub flags: &'a str,
}

pub fn empty_line(l: &EmptyLine) -> String {
    format!(
        "battle without a deck: empty revolver {:#x} built by Re:CoM's ctor 0x140164D40 from its end-of-deck entry ({} cards, {} slot(s), flags {:#x}: {}); {}; deck HUD mask reads (cockpit 4, cards 3): {}; Friend-card drops off; tutorial flags set: {}",
        l.rev,
        l.cards,
        l.slots,
        l.rev_flags,
        if l.rev_flags & 1 != 0 { "Reload card in" } else { "NO Reload card" },
        if l.tutorial_deck { "tutorial-deck flag (1, 0x2C) set: Re:CoM would have dealt its tutorial deck" } else { "tutorial-deck flag (1, 0x2C) clear" },
        l.hud,
        l.flags
    )
}

/// The trace's stage-2 token: Sora's revolver (none in the field) and the tutorial flags.
pub fn trace_token(player_rev: usize, empty: usize, flags: &str) -> String {
    let deck = match (player_rev, empty) {
        (0, _) => "none",
        (r, e) if r == e => "empty",
        _ => "recom",
    };
    format!("m6deck deck={deck} tut=[{flags}]")
}

/// Log caps.
pub const BATTLE_LOG_LINES: u32 = 100;
pub const DROP_LOG_LINES: u32 = 20;
pub const READER_LOG_LINES: u32 = 60;

#[cfg(test)]
mod tests {
    use super::*;

    fn kh1() -> Battle {
        Battle { kh1: true, riku_story: false, main_sora: true, minigame: false }
    }

    #[test]
    fn only_kh1_sora_story_battles_lose_the_deck() {
        assert_eq!(decide(&kh1()), Build::Empty);
        assert_eq!(decide(&Battle { kh1: false, ..kh1() }), Build::Recom(Keep::Setting));
        assert_eq!(decide(&Battle { riku_story: true, ..kh1() }), Build::Recom(Keep::Riku));
        assert_eq!(decide(&Battle { main_sora: false, ..kh1() }), Build::Recom(Keep::NotSora));
        assert_eq!(decide(&Battle { minigame: true, ..kh1() }), Build::Recom(Keep::Minigame));
        // The setting wins over everything else (Re:CoM stays byte for byte).
        assert_eq!(decide(&Battle { kh1: false, riku_story: true, main_sora: false, minigame: true }), Build::Recom(Keep::Setting));
        assert!(drops_off(true, false, false));
        assert!(!drops_off(false, false, false));
        assert!(!drops_off(true, true, false));
        assert!(!drops_off(true, false, true));
    }

    #[test]
    fn the_list_is_re_coms_end_of_deck_entry() {
        // The loader stops at an id byte of 0 or >= 0xCB: the first entry already ends it.
        let id = (EMPTY_LIST[0] & 0xFF) as u8;
        assert!(id == 0 || id >= 0xCB);
        assert_eq!(EMPTY_LIST[0], 0xFF);
        assert_eq!(revolver::SIZE, 0x280);
    }

    #[test]
    fn hud_bits_only_for_this_battles_empty_revolver() {
        let e = 0x5000;
        assert_eq!(cockpit_bits(e, e), mask::COCKPIT);
        assert_eq!(cockpit_bits(0x6000, e), 0);
        assert_eq!(cockpit_bits(0, 0), 0);
        assert_eq!(card_bits(1, e, e), mask::PLAYER_CARDS);
        // Enemy cards (side 2), another revolver, or no empty revolver: Re:CoM's mask as is.
        assert_eq!(card_bits(2, e, e), 0);
        assert_eq!(card_bits(1, 0x6000, e), 0);
        assert_eq!(card_bits(1, 0, 0), 0);
        // The card read at 0x14015037C tests 0x20 for side 1 and 0x40 for side 2: our bit never
        // reaches an enemy card.
        assert_eq!(mask::PLAYER_CARDS & 0x40, 0);
        assert_eq!(mask::COCKPIT & (0x20 | 0x40 | 4), 0);
    }

    #[test]
    fn readers_log_once_per_battle() {
        assert!(log_reader(0, 1, 0));
        assert!(!log_reader(1, 1, 0));
        assert!(log_reader(1, 2, 5));
        assert!(!log_reader(1, 2, READER_LOG_LINES));
        assert!(!log_reader(0, 0, 0));
        assert_eq!(READERS.iter().map(|r| r.index()).collect::<Vec<_>>(), vec![0, 1, 2]);
    }

    #[test]
    fn lines() {
        assert_eq!(flags_str(&[(0x11, true), (0x12, false), (0x13, true)]), "0x11 0x13");
        assert_eq!(flags_str(&[(0x11, false)]), "none");
        let l = empty_line(&EmptyLine { rev: 0x1234, cards: 0, slots: 1, rev_flags: 1, tutorial_deck: true, hud: "ours (7 written)", flags: "0x13" });
        assert!(l.starts_with("battle without a deck: empty revolver 0x1234"));
        assert!(l.contains("0 cards, 1 slot(s), flags 0x1: Reload card in"));
        assert!(l.contains("(1, 0x2C) set"));
        assert!(l.contains("ours (7 written)"));
        assert_eq!(trace_token(0, 0, "none"), "m6deck deck=none tut=[none]");
        assert_eq!(trace_token(0x10, 0x10, "0x11"), "m6deck deck=empty tut=[0x11]");
        assert_eq!(trace_token(0x10, 0x20, "none"), "m6deck deck=recom tut=[none]");
    }
}
