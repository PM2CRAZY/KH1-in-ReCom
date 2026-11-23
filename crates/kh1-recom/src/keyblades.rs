//! Equipping Keyblades at the KH1 setting (decision 25): the testable part.
//!
//! Re:CoM's camp (pause) menu gets an 8th top-level entry, "KH1" (M7 stage 7.13; "Keyblades"
//! before it), below Config, drawn and run by Re:CoM's own camp code through four wrappers
//! (`win::keyblades`); since 2026-11-03 it is there whatever the Combat setting, in both stories
//! (Riku's since 2026-11-03, owner: "only Combat and Movement"). It opens a submenu of Re:CoM's
//! own entry buttons ([`sub`]): at Combat = KH1 in Sora's story Items (`crate::items_page`, whose
//! Sora list starts with his Keyblade slot), Customize (`crate::customize`) and Abilities; always
//! the Combat and Movement settings pages (`crate::menu`). The Keyblade slot opens the Keyblade list: the Keyblades Sora has earned,
//! the equipped one marked, the highlighted one's numbers, equip on confirm. The choice is one byte
//! of Re:CoM's own save block.
//! See `docs/recom-keyblade-equip.md`.
//!
//! Everything here is plain logic: the cursor rules (Re:CoM's, over 8 entries), the page's list
//! rules (Re:CoM's entry-4 list `0x1400EB7E0`), unlock filtering, the save byte and the stats
//! panel's strings.

use crate::attack::Keyblade;

/// Re:CoM's Keyblade card kinds (card-info table `0x1405256D0`, type 1). Checked (exe).
pub const KINGDOM_KEY: u16 = 1;
pub const SOUL_EATER: u16 = 0x12;
pub const LAST_KEYBLADE: u16 = 0x18;
/// A premium card's id is its kind | 0x100. Checked (decomp `0x140158FC0`, `0x14015C1F0`).
pub const PREMIUM: u16 = 0x100;

/// `*(u64*)0x14087B110`: Re:CoM's main save block (0x2AD4 bytes, saved, checksummed, loaded,
/// snapshotted and zeroed at new game by Re:CoM). Checked (decomp `0x14012BAE0`, `0x1401B1300`).
pub const SAVE_PTR: u64 = 0x1_4087_B110;
/// Our byte: `save + 0x2674` = card collection (`save + 0x1A74`, exe `0x140158F20`) + 0xC00, in
/// the collection's tail that nothing in the exe reads or writes. Checked (decomp + exe scan,
/// `recom-cards-save.md` § 4); that nothing reaches it through a pointer from elsewhere is an
/// Assumption, logged at the first camp open (`spare_range_report`).
pub const EQUIP_OFFSET: usize = 0x2674;
/// The spare range `save + 0x246B ..= save + 0x2817` (the collection tail after the discovery
/// bits). Checked (exe: the save's format string at `0x1405B8128`).
pub const SPARE_FIRST: usize = 0x246B;
pub const SPARE_LAST: usize = 0x2817;

/// The equipped Keyblade from the save byte: 0 (fresh or vanilla save) means the Kingdom Key;
/// anything outside 1..=0x18, Soul Eater, or a Keyblade the stats file has no numbers for also
/// gives the Kingdom Key.
pub fn equipped_card(byte: u8, has_numbers: impl Fn(u16) -> bool) -> u16 {
    let k = byte as u16;
    if (KINGDOM_KEY..=LAST_KEYBLADE).contains(&k) && k != SOUL_EATER && has_numbers(k) {
        k
    } else {
        KINGDOM_KEY
    }
}

/// The byte written when `card` is equipped: 0 for the Kingdom Key, so a save that never leaves
/// it keeps the spare range all zero, as in vanilla.
pub fn save_byte_for(card: u16) -> u8 {
    if card == KINGDOM_KEY || !(KINGDOM_KEY..=LAST_KEYBLADE).contains(&card) || card == SOUL_EATER {
        0
    } else {
        card as u8
    }
}

/// The offsets (from the save block) in the spare range that are not zero, our bytes excepted
/// (the Keyblade byte, M7's MP block `crate::mp::BLOCK_FIRST ..= BLOCK_LAST`, and M8's items
/// extension `crate::items::EXT_FIRST ..= EXT_LAST`, M9's ability block
/// `crate::abilities::BLOCK_FIRST ..= BLOCK_LAST`, M8b's summon block
/// `crate::summons::BLOCK_FIRST ..= BLOCK_LAST`, M12's ability words' extension
/// `crate::abilities::EXT_FIRST ..= EXT_LAST`, the Enemy Skills' extension
/// `crate::abilities::SKILL_EXT_FIRST ..= SKILL_EXT_LAST`). `range` is `save[SPARE_FIRST..=SPARE_LAST]`.
pub fn spare_range_nonzero(range: &[u8]) -> Vec<usize> {
    let ours = |o: usize| o == EQUIP_OFFSET || (crate::mp::BLOCK_FIRST..=crate::mp::BLOCK_LAST).contains(&o) || (crate::items::EXT_FIRST..=crate::items::EXT_LAST).contains(&o)
        || (crate::abilities::BLOCK_FIRST..=crate::abilities::BLOCK_LAST).contains(&o)
        || (crate::summons::BLOCK_FIRST..=crate::summons::BLOCK_LAST).contains(&o)
        || (crate::abilities::EXT_FIRST..=crate::abilities::EXT_LAST).contains(&o)
        || (crate::abilities::SKILL_EXT_FIRST..=crate::abilities::SKILL_EXT_LAST).contains(&o);
    range
        .iter()
        .enumerate()
        .filter(|(i, b)| **b != 0 && !ours(SPARE_FIRST + i))
        .map(|(i, _)| SPARE_FIRST + i)
        .collect()
}

/// The Keyblades the page lists, in card order: those with numbers in the stats file
/// (`cards`), never Soul Eater, the Kingdom Key always, the others when Re:CoM's discovery state
/// says the card (plain or premium) was ever obtained (`earned`, `0x14015B3F0`).
pub fn unlocked(cards: impl IntoIterator<Item = u16>, earned: impl Fn(u16) -> bool) -> Vec<u16> {
    let mut v: Vec<u16> = cards
        .into_iter()
        .filter(|&k| (KINGDOM_KEY..=LAST_KEYBLADE).contains(&k) && k != SOUL_EATER)
        .filter(|&k| k == KINGDOM_KEY || earned(k) || earned(k | PREMIUM))
        .collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// The story the camp belongs to: Re:CoM's camp copies `0x1401AAAE0()` into `camp+0x21BC` at its
/// ctor (`0x1400D2730`) and indexes its per-story tables with it (labels `0x1407C1000`, help ids
/// in the cursor set `0x1400DEA10`, header titles `0x1400D3890`). Checked (decomp + disasm).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Story {
    Sora,
    Riku,
}

impl Story {
    /// From `camp+0x21BC` (0 Sora's story, else Riku's; `0x1401AAAE0` returns a bool).
    pub fn from_camp(v: i32) -> Story {
        if v == 0 {
            Story::Sora
        } else {
            Story::Riku
        }
    }
}

/// The camp's top list (owner, 2026-11-04, "Camp menu at KH1" in
/// `docs/design-philosophy-instructions.md`).
///
/// - **KH1 list** (Combat = KH1, Sora's story, the list camp): nine entries, top to bottom Items,
///   Abilities, Customize, World Map, Map Cards, World Cards, Status, Journal, Settings. Items is
///   Re:CoM's entry 0 (Review Decks) relabelled; Abilities and Customize are two more instances of
///   Re:CoM's entry button at camp indices 7 and 8; Re:CoM's entries 1..6 keep their indices (so
///   every Re:CoM read of the cursor still means what it meant: the entry 1 / 3 checks, the state
///   table, the header titles, the help ids). Only the order on screen and the positions change.
/// - **Settings list** (Combat = Re:CoM, Sora's story): Re:CoM's seven, where they are; only
///   Settings (entry 6) opens the short list ([`super::sub`]) instead of Re:CoM's page.
/// - **Re:CoM** (Riku's story, the camp that opens straight onto the map): untouched.
pub mod top {
    use super::Story;

    /// Re:CoM's entry count at the top level (camp ctor `0x1400D2730`). Checked (decomp).
    pub const RECOM_ENTRIES: i32 = 7;
    /// Re:CoM's Review Decks entry (state table byte 0x1D); at the KH1 list, our Items.
    pub const DECKS: i32 = 0;
    /// Re:CoM's Settings entry (Config, state table byte 0x4A).
    pub const SETTINGS: i32 = 6;
    /// Our two entries' camp indices (past Re:CoM's 7: Re:CoM's confirm skips them,
    /// `cmp [camp+0xB8],6; jg` at `0x1400DAABD`, Checked (disasm)).
    pub const ABILITIES: i32 = 7;
    pub const CUSTOMIZE: i32 = 8;
    /// Our extra buttons, in camp index order.
    pub const EXTRA: [i32; 2] = [ABILITIES, CUSTOMIZE];
    pub const KH1_ENTRIES: i32 = 9;
    /// The KH1 list on screen, top to bottom: the camp index in each slot.
    pub const KH1_ORDER: [i32; KH1_ENTRIES as usize] = [DECKS, ABILITIES, CUSTOMIZE, 1, 2, 3, 4, 5, SETTINGS];

    /// Which top list the camp shows.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Layout {
        /// Re:CoM's own, untouched (Riku's story, the direct-map camp).
        Recom,
        /// Re:CoM's seven where they are; Settings opens the short list.
        Settings,
        /// The owner's nine.
        Kh1,
        /// Riku's story at Combat = KH1 with `riku_kh1_movement` (fork branch, 0.150.0; fork owner
        /// 2026-11-22: "poderia substituir o Review Decks quando a gameplay do KH1 estiver ativa,
        /// IGUAL a do Sora"): Re:CoM's seven where they are, entry 0 (Review Decks) relabelled
        /// Customize (his L1 shortcuts), Settings opening the short list.
        Riku,
    }

    /// The list for this camp. `direct_map` = `camp+0x20A4` (the camp never shows the list);
    /// `kh1_pages` = Combat = KH1 and the stats file lists Keyblades (the KH1 pages' condition
    /// since 0.8.0: without the asset step's numbers they have nothing to show).
    ///
    /// `riku_settings` (fork testing, `riku_kh1_movement` on): Riku's story gets the Settings list
    /// too ([`super::sub::RIKU_SETTINGS_ROWS`]), so its Combat and Movement pages reach him.
    pub fn layout(story: Story, direct_map: bool, combat_kh1: bool, kh1_pages: bool, riku_settings: bool) -> Layout {
        if direct_map {
            Layout::Recom
        } else if story == Story::Riku {
            if riku_settings && combat_kh1 {
                Layout::Riku
            } else if riku_settings {
                Layout::Settings
            } else {
                Layout::Recom
            }
        } else if combat_kh1 && kh1_pages {
            Layout::Kh1
        } else {
            Layout::Settings
        }
    }

    /// What a top entry is.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Entry {
        /// Our pages (`super::sub::Row` names the page).
        Items,
        Abilities,
        Customize,
        /// Re:CoM's entry `i` (0..6), run by Re:CoM (Settings: see [`settings_list`]).
        Recom(i32),
    }

    impl Entry {
        /// The page our entry opens.
        pub fn page(self) -> Option<super::sub::Row> {
            match self {
                Entry::Items => Some(super::sub::Row::Items),
                Entry::Abilities => Some(super::sub::Row::Abilities),
                Entry::Customize => Some(super::sub::Row::Customize),
                Entry::Recom(_) => None,
            }
        }
    }

    /// The entry at camp index `idx` in this list.
    pub fn entry(l: Layout, idx: i32) -> Option<Entry> {
        match (l, idx) {
            (Layout::Kh1, DECKS) => Some(Entry::Items),
            (Layout::Kh1, ABILITIES) => Some(Entry::Abilities),
            (Layout::Kh1, CUSTOMIZE) => Some(Entry::Customize),
            (Layout::Riku, DECKS) => Some(Entry::Customize),
            (_, i) if (0..RECOM_ENTRIES).contains(&i) => Some(Entry::Recom(i)),
            _ => None,
        }
    }

    /// Settings opens the short list (Game Settings, Combat, Movement) in Sora's story at both
    /// Combat settings; Re:CoM's page directly otherwise.
    pub fn settings_list(l: Layout) -> bool {
        l != Layout::Recom
    }

    /// Entries in the list.
    pub fn count(l: Layout) -> i32 {
        if l == Layout::Kh1 {
            KH1_ENTRIES
        } else {
            RECOM_ENTRIES
        }
    }

    /// The slot on screen (0 = top) of camp index `idx`.
    pub fn slot(l: Layout, idx: i32) -> Option<i32> {
        if l == Layout::Kh1 {
            KH1_ORDER.iter().position(|&i| i == idx).map(|s| s as i32)
        } else {
            (0..RECOM_ENTRIES).contains(&idx).then_some(idx)
        }
    }

    /// The camp index in slot `s`.
    pub fn at_slot(l: Layout, s: i32) -> Option<i32> {
        if l == Layout::Kh1 {
            usize::try_from(s).ok().and_then(|s| KH1_ORDER.get(s)).copied()
        } else {
            (0..RECOM_ENTRIES).contains(&s).then_some(s)
        }
    }

    // Positions (Re:CoM's 2D units, 512 × 384 with (0, 0) top left; `0x140248660` subtracts
    // (256, 192)). Re:CoM's: entry build `0x1400E3DE0` places button i at (86, 88 + 26 i)
    // (`0x140509430`, step `0x140509270`); the highlight bar `0x1400E07D0` at (48, 80 + 26 i)
    // (`0x140509294`, `0x1405092C0`); the glove `0x1400E3D30` glides to (150, 82 + 26 i)
    // (`0x1405092F4`, `0x1405092C4`). Checked (decomp + exe).
    pub const ENTRY_X: f32 = 86.0;
    pub const RECOM_Y0: f32 = 88.0;
    pub const RECOM_STEP: f32 = 26.0;
    pub const BAR_X: f32 = 48.0;
    pub const RECOM_BAR_Y0: f32 = 80.0;
    pub const GLOVE_X: f32 = 150.0;
    pub const RECOM_GLOVE_Y0: f32 = 82.0;
    /// The KH1 list: nine entries at y 78 .. 270, in the room Re:CoM's seven and the 8th entry
    /// ("KH1", at y 270, seen in game since 0.8.0) took. Measured (Checked (files: `camp.lad` of the
    /// owner's `FO0100.SPR`)): an entry's plate (sprite group 18) is 26 high (−14 .. +12 around
    /// the button), its highlight glow (17) 30 (−16 .. +14); Re:CoM's step 26 makes the plates
    /// touch. Step 24 overlaps neighbouring plates by 2. The first entry's plate top (64) stays
    /// below the header (layout group 0 elements 11 / 12 at y 51, plate 37 .. 63), which Re:CoM
    /// shows on a confirm while the entries fade; the last's bottom (282) where the 8th entry's was.
    ///
    /// **Moved down 10 (owner, 2026-11-04, screenshot: "items is kind of clipping through the top, there's enough
    /// space at the bottom to bring everything down a little bit"):** the header frame's lower border covers
    /// screen y 0..≈ 69 (720p screenshot: its bar at ≈ 130 px of 720 = 69.3 units), so the first plate's top 64 was
    /// under it; the bottom info panel starts at ≈ 297 (558 px). Now y 88 .. 280 (Re:CoM's own first y): plates
    /// 74 .. 292, glows 72 .. 294. Checked (owner's screenshot; plate / glow sizes as above).
    pub const KH1_Y0: f32 = 88.0;
    pub const KH1_STEP: f32 = 24.0;

    fn y0_step(l: Layout) -> (f32, f32) {
        if l == Layout::Kh1 {
            (KH1_Y0, KH1_STEP)
        } else {
            (RECOM_Y0, RECOM_STEP)
        }
    }

    /// Entry button position of slot `s`.
    pub fn entry_pos(l: Layout, s: i32) -> (f32, f32) {
        let (y0, step) = y0_step(l);
        (ENTRY_X, y0 + step * s as f32)
    }

    /// The highlight bar's position for slot `s` (Re:CoM's: 8 above the entry).
    pub fn bar_pos(l: Layout, s: i32) -> (f32, f32) {
        (BAR_X, entry_pos(l, s).1 - (RECOM_Y0 - RECOM_BAR_Y0))
    }

    /// The glove's glide target for slot `s` (Re:CoM's: 6 above the entry).
    pub fn glove_pos(l: Layout, s: i32) -> (f32, f32) {
        (GLOVE_X, entry_pos(l, s).1 - (RECOM_Y0 - RECOM_GLOVE_Y0))
    }

    /// One frame of the pad as the top list's input (update `0x1400D9C80`, state 0xF) reads it.
    /// Checked (disasm `0x1400DAA8B..0x1400DAD84`): Start (mask 0x800, fresh) or cancel first,
    /// then confirm, then up (repeat, mask 0x101000), then down (repeat, mask 0x204000). At the
    /// ends of the list a held repeat stops; only a fresh press wraps.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct Pad {
        pub start: bool,
        pub cancel: bool,
        pub confirm: bool,
        pub up: bool,
        pub up_fresh: bool,
        pub down: bool,
        pub down_fresh: bool,
    }

    /// What Re:CoM's own state 0xF does with `count` entries: the index it passes to the cursor
    /// set `0x1400DEA10` (before its wrap), or None when nothing moves.
    pub fn recom_move(cursor: i32, count: i32, p: Pad) -> Option<i32> {
        if p.start || p.cancel || p.confirm {
            return None;
        }
        if p.up {
            if cursor - 1 < 0 && !p.up_fresh {
                return None;
            }
            return Some(cursor - 1);
        }
        if p.down {
            if count <= cursor + 1 && !p.down_fresh {
                return None;
            }
            return Some(cursor + 1);
        }
        None
    }

    /// The KH1 list's move: Re:CoM's rule over the nine slots on screen (held stops at the ends,
    /// fresh wraps, as the cursor set wraps), as a camp index.
    pub fn kh1_move(cursor: i32, p: Pad) -> Option<i32> {
        let s = slot(Layout::Kh1, cursor)?;
        let t = recom_move(s, KH1_ENTRIES, p)?;
        at_slot(Layout::Kh1, t.rem_euclid(KH1_ENTRIES))
    }

    /// What our update wrapper does around Re:CoM's state 0xF at the KH1 list.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Fix {
        /// Re:CoM's own code does it: its close / its confirm of entries 1..6 / no input / a move
        /// it makes itself (the cursor set then puts the cursor on `kh1_move`'s entry).
        None,
        /// A move Re:CoM's 7-entry rule refuses (a held "down" on index 7 or 8, which it takes
        /// for its last entries): ours, with Re:CoM's cursor set, sound 1 and glide.
        Move(i32),
        /// Confirm on one of our entries: Re:CoM must not run its own (entry 0 would open the
        /// deck submenu; 7 / 8 it skips); we run its confirm sequence for a page entry.
        Confirm(Entry),
    }

    pub fn kh1_fix(cursor: i32, p: Pad) -> Fix {
        if p.start || p.cancel {
            return Fix::None;
        }
        if p.confirm {
            return match entry(Layout::Kh1, cursor) {
                Some(e @ (Entry::Items | Entry::Abilities | Entry::Customize)) => Fix::Confirm(e),
                _ => Fix::None,
            };
        }
        match (kh1_move(cursor, p), recom_move(cursor, RECOM_ENTRIES, p)) {
            (Some(t), None) if t != cursor => Fix::Move(t),
            _ => Fix::None,
        }
    }

    /// Riku's list ([`Layout::Riku`]): Re:CoM's own moves; a confirm on entry 0 is ours (Re:CoM's
    /// would open its deck submenu).
    pub fn riku_fix(cursor: i32, p: Pad) -> Fix {
        if p.confirm && !p.start && !p.cancel && cursor == DECKS {
            Fix::Confirm(Entry::Customize)
        } else {
            Fix::None
        }
    }

    /// The whole KH1-list rule as the game runs it: Re:CoM's state 0xF with its 7-entry count,
    /// the cursor set redirected to `kh1_move`'s entry when Re:CoM moves, and our fix when it
    /// doesn't. Used by the tests.
    pub fn kh1_step(cursor: i32, p: Pad) -> Option<i32> {
        if recom_move(cursor, RECOM_ENTRIES, p).is_some() {
            return kh1_move(cursor, p);
        }
        match kh1_fix(cursor, p) {
            Fix::Move(t) => Some(t),
            _ => None,
        }
    }

    /// The help message ids of Re:CoM's top entries 0..6, per story: the cursor set `0x1400DEA10`
    /// builds them on its stack (`mov dword [rbp+d], id`) and reads index
    /// `(level + story × 2) × 7 + entry`. Sora's at `0x1400DEA45`, Riku's at `0x1400DEAA9`
    /// (`crate::sites::KEYBLADE_DATA`, the test reads these from the checked bytes). Riku's entries
    /// 0 and 5 have their own. Checked (exe disasm).
    pub const RECOM_HELP: [[u32; RECOM_ENTRIES as usize]; 2] = [
        [0x50_0400, 0x50_0401, 0x50_0402, 0x50_0403, 0x50_0404, 0x50_0405, 0x50_0406],
        [0x50_042A, 0x50_0401, 0x50_0402, 0x50_0403, 0x50_0404, 0x50_042C, 0x50_0406],
    ];

    /// The help id Re:CoM shows with the cursor on its top entry `i` in this story.
    pub fn recom_help(story: Story, i: i32) -> Option<u32> {
        let row = &RECOM_HELP[(story == Story::Riku) as usize];
        usize::try_from(i).ok().and_then(|i| row.get(i)).copied()
    }

    /// The label message ids of Re:CoM's top entries in Sora's story (`0x1407C1000`, read by the
    /// entry build `0x1400E3DE0` at `((level + story × 2) × 7 + i) × 4`; Checked (exe)): Review
    /// Decks is `0xA80001`. Used to give entry 0 its own label back when the list leaves KH1's.
    pub const RECOM_LABELS: [u32; RECOM_ENTRIES as usize] = [0xA8_0001, 0xA8_0002, 0xA8_0003, 0xA8_0004, 0xA8_0005, 0xA8_0006, 0xA8_0007];
}

/// The Keyblade page's list: Re:CoM's entry-4 list rules (`0x1400EB7E0` sub-state 2 and draw
/// mode 9 of `0x1400E6DF0`). Checked (decomp).
pub mod list {
    /// Rows on screen (`camp+0xC58[8]`).
    pub const VISIBLE: i32 = 8;

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct List {
        pub cursor: i32,
        pub top: i32,
        pub count: i32,
    }

    /// One frame of input. `up` / `down` are `0x140111A70`'s repeat result; `*_fresh` is the
    /// fresh-press flag it leaves in `camp+0x94` for that direction. `page_up` / `page_down` are
    /// the raw repeat reads of pad bits 4 / 8 (`0x14029B2E0`), `*_fresh` the fresh reads
    /// (`0x14029B2F0`).
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct Pad {
        pub up: bool,
        pub up_fresh: bool,
        pub down: bool,
        pub down_fresh: bool,
        pub page_up: bool,
        pub page_up_fresh: bool,
        pub page_down: bool,
        pub page_down_fresh: bool,
    }

    /// The list after one frame, and whether the cursor moved (Re:CoM then plays sound 1 and
    /// redraws the rows). Re:CoM's up past the first row goes to its tab bar; this page has no
    /// tabs, so it wraps to the last row as "down" wraps to the first.
    pub fn step(l: List, p: Pad) -> (List, bool) {
        let mut n = l;
        let old = l.cursor;
        let c = l.cursor;
        if l.count <= 0 {
            return (l, false);
        }
        if p.up {
            if c - 1 < 0 {
                if !p.up_fresh {
                    return (l, false);
                }
                n.cursor = l.count - 1;
            } else {
                n.cursor = c - 1;
            }
        } else if p.down {
            if c + 1 < l.count {
                n.cursor = c + 1;
            } else if p.down_fresh {
                n.cursor = 0;
            } else {
                return (l, false);
            }
        } else if p.page_up {
            let t = c - 8;
            if t >= 0 {
                n.cursor = t;
                n.top = (l.top - 8).max(0);
            } else if t < -7 {
                if !p.page_up_fresh {
                    return (l, false);
                }
                n.cursor = l.count - 1;
            } else {
                n.cursor = 0;
            }
        } else if p.page_down {
            let t = c + 8;
            if t < l.count {
                n.cursor = t;
                n.top = l.top + 8;
                if l.count - 8 <= n.top {
                    n.top = l.count - 8;
                }
            } else if t < l.count + 7 {
                n.cursor = l.count - 1;
            } else {
                if !p.page_down_fresh {
                    return (l, false);
                }
                n.cursor = 0;
            }
        } else {
            return (l, false);
        }
        (n, n.cursor != old)
    }

    /// The top row a redraw uses (draw mode 9): the cursor is kept inside the 8 visible rows.
    pub fn top_for(l: List) -> i32 {
        if l.cursor < l.top {
            l.cursor
        } else if l.top + (VISIBLE - 1) < l.cursor {
            l.cursor - (VISIBLE - 1)
        } else {
            l.top
        }
    }

    /// The scroll bar's (position, size) fractions (`0x140112CB0` as draw mode 9 calls it:
    /// `min(1, top / n)`, `min(1, 8 / n)`, n = count or 1). Checked (disasm `0x1400E7B6F`).
    pub fn bar(l: List) -> (f32, f32) {
        let n = if l.count == 0 { 1.0 } else { l.count as f32 };
        ((l.top as f32 / n).min(1.0), (8.0 / n).min(1.0))
    }

    /// The list opened on the equipped Keyblade.
    pub fn open_on(count: i32, cursor: i32) -> List {
        let l = List { cursor: cursor.clamp(0, (count - 1).max(0)), top: 0, count };
        List { top: top_for(l), ..l }
    }

    /// The list re-read with the window scrolled where it was: `top` kept (clamped so no blank rows
    /// show past the end, as Re:CoM's page down clamps it), then moved only as far as the cursor
    /// needs (draw mode 9's rule, [`top_for`]).
    pub fn keep(count: i32, cursor: i32, top: i32) -> List {
        let l = List { cursor: cursor.clamp(0, (count - 1).max(0)), top: top.clamp(0, (count - VISIBLE).max(0)), count };
        List { top: top_for(l), ..l }
    }

    /// Each open window's top row, by its depth in the page's window stack (Abilities: 0 the owner
    /// list, 1 an owner's abilities; Items: KH1's window stack). Owner, 2026-11-05: "When
    /// activating an ability it automatically puts it all the way at the bottom of the list": the
    /// order never changed, but every re-read opened the list afresh at top 0, so the cursor's row
    /// was drawn on the last visible line. A re-read of the same window keeps its scroll; going
    /// back to a window restores the scroll it had; a newly opened window starts afresh.
    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct Scroll {
        tops: Vec<i32>,
        depth: usize,
    }

    impl Scroll {
        /// The list for the window at `depth` (`count` rows, cursor `cursor`), `shown` being the
        /// list on screen before the re-read (the window at the last depth read).
        pub fn reread(&mut self, shown: List, depth: usize, count: i32, cursor: i32) -> List {
            let from = self.depth;
            if self.tops.len() <= from {
                self.tops.resize(from + 1, 0);
            }
            self.tops[from] = shown.top;
            self.depth = depth;
            if depth > from {
                self.tops.truncate(from + 1);
                return open_on(count, cursor);
            }
            self.tops.truncate(depth + 1);
            keep(count, cursor, self.tops[depth])
        }
    }
}

/// The "Equipped" word on a camp list row, where Re:CoM's "NEW!" badge was (owner, 2026-11-05:
/// "instead of equipped items/abilities saying "new" when they're equipped, can they just say
/// "equipped""). Notes: `docs/recom-keyblade-equip.md` § "Equipped" instead of "NEW".
///
/// Re:CoM's badge (Checked (decomp + disasm + files)): its Sleights page (`0x1400E6DF0` mode 9) adds
/// sequence `0x1D + (byte > 0)` of `camp:00` to a row's sprite through the sprite's `+0x40` at
/// (170, −13) (`0x140509308`, `0x140509340`): 0x1E (never highlighted) pulses alpha 0x40 ↔ 0x80,
/// 0x1D (seen) is static at alpha 0x20. Both draw sprite 31, the "NEW!" picture (39 × 12 units),
/// at translate (−23, 13): with the row's anchor (22, 14, the group's first piece) the box is x
/// 149..188, y 8..20, the plate's lower right (plate 0..197 × −5..21). Our pages drew 0x1D on every
/// equipped row (the faint "NEW!" in the owner's recording: glyphs at x 152.5..186.7, y 9.7..19.0).
///
/// Ours: Re:CoM's own text, a `C2DFnt` attached to the row's sprite with its `+0x60` as Re:CoM's
/// status values are (`0x1400E6DF0` rows 0..4), right-aligned at the badge's right edge, in the
/// status panel's font 0xE (cap height 34 px against the badge's 33 px and the row names' 37 px at
/// 2560 × 1440, measured in the recording). No asset, no new Re:CoM call. None of our pages draws
/// Re:CoM's badge any more: on them it never meant "new" (only "equipped" / "set").
pub mod badge {
    /// The word (NUL-terminated ASCII, no `%`).
    pub const TEXT: &[u8] = b"Equipped\0";
    /// Re:CoM's status-panel font (the badge's height).
    pub const FONT: i32 = 0xE;
    /// Right-aligned at its x (Re:CoM's status values, `0x1400E6DF0`).
    pub const FLAGS: i32 = 0x41;
    /// The row's text anchor (the group's first piece, `crate::text_fit::boxes::CAMP_TEXT_LEFT` − 1).
    /// Checked (recording): the name at offset 1 starts 23.2 units from the plate's left.
    pub const ANCHOR_X: f32 = 22.0;
    /// The right edge, in the plate's units: the badge picture's right (188) less one (its glyphs end
    /// at 186.7).
    pub const RIGHT: f32 = 187.0;
    /// The offset given to `+0x60` (x relative to the anchor).
    pub const X: f32 = RIGHT - ANCHOR_X;
    /// The vertical offset: the word's glyphs on the badge's (y 9.7..19.0), where the name at
    /// offset 2 has its capitals at 2.9..13.4 (recording). Assumption: font 0xE sits on its offset
    /// as font 0xF does (top-aligned that gives 8.9, bottom-aligned 7.4; Re:CoM itself puts both
    /// fonts' rows at (1, 2)); 8 is within a unit of either.
    pub const Y: f32 = 8.0;
    /// The widest the word may be drawn (squeezed to fit beyond, `crate::text_fit`): from the plate's
    /// x 120 (past the longest names' usual end) to [`RIGHT`].
    pub const MAX_WIDTH: f32 = RIGHT - 120.0;
    /// Room kept between a name and the word.
    pub const GAP: f32 = 4.0;
    /// The text colour (ABGR, 0x80 = ×1), a lit version of the badge's yellow-green: the picture is
    /// about (247, 255, 120) drawn at alpha 0x20 (seen as (64, 108, 45) on the plate (3, 57, 20)).
    /// Shown as about (128, 180, 64): readable, the badge's hue, calmer than a white name.
    /// Assumption: Re:CoM's text colour maps as about 13 + 2.49 × value per channel (fitted to the
    /// status value 0x80046060 drawn (252, 255, 23)).
    pub const COLOUR: u32 = 0x8014_432E;

    /// A marked row's name box (`crate::text_fit` units): from the name's left to the word's left
    /// less [`GAP`], the word `word` units wide as drawn; never more than an unmarked row's.
    pub fn name_box(word: f32) -> f32 {
        let left = RIGHT - word.clamp(0.0, MAX_WIDTH) - GAP;
        (left - crate::text_fit::boxes::CAMP_TEXT_LEFT).min(crate::text_fit::boxes::CAMP_ROW)
    }
}

/// The look of a camp list row: the variant Re:CoM's `CButton` set-up `0x140112F10` takes (its
/// last argument) to pick the row's sequence from the list's table `0x1407C11F0`
/// (`seqs[variant]`, Checked (decomp)). Re:CoM's Sleights page passes each row's category
/// (`0x1400E6DF0` mode 9 reads it from the list entry's second byte, filled in mode 10 from the
/// sleight's kind: Sora's 0..3, Riku's 4..7; Checked (decomp)). What each sequence draws, from
/// `camp.lad` in `FO0100.SPR` (Checked (files)): a plate (sprites 70 / 71) tinted by its
/// animation's colour, and an icon sprite.
pub mod row_look {
    /// Sequence 0x12E: red plate (colour 0x8000003C), icon sprite 167 untinted (Re:CoM's
    /// physical sleights; the rows we drew before took this one).
    pub const RED: i32 = 0;
    /// Sequence 0x133: blue plate (0x803C1E00), icon sprite 168 (Re:CoM's magic sleights).
    pub const BLUE: i32 = 1;
    /// Sequence 0x131: green plate (0x80143200), icon sprite 278 tinted green (0x801EDC1E): the
    /// green **Mickey** symbol (owner, in game, 2026-11-01), not the item symbol. Re:CoM's item
    /// symbol is sprite 166 (sprite group 83; the Deck page's item-card tab, Checked (decomp +
    /// files)); Re:CoM has no row with it, the asset step adds one ([`ITEM`]):
    /// `docs/recom-m8-items.md` § Row looks.
    pub const GREEN: i32 = 2;
    /// The asset step's group (`crate::camp_rows`): Re:CoM's green row 0x131 with the item symbol
    /// (sprite 166). Only in the DLL's own 8-entry copy of the list table, never in Re:CoM's.
    pub const ITEM: i32 = crate::camp_rows::ITEM_VARIANT;
}

/// Texts: NUL-terminated ASCII handed to Re:CoM's text items (as the Config row's are; seen in
/// game there). No `%`: the help line's formatter `0x1402402B0` treats it as a code.
pub mod text {
    /// The Keyblade list's help (opened from the Items page's Keyblade slot).
    pub const PAGE_HELP: &[u8] = b"Choose the Keyblade Sora fights with.\0";
    pub const STAT_LABELS: [&[u8]; 3] = [b"Strength\0", b"Magic\0", b"Reach\0"];
    /// The Customize entry (KH1's pause-menu word) and its help.
    pub const CUSTOMIZE_LABEL: &[u8] = b"Customize\0";
    pub const CUSTOMIZE_HELP: &[u8] = b"Set the spells and sleights of the three shortcuts.\0";
    /// The Settings list's first row: Re:CoM's own Config page (its help line is Re:CoM's
    /// message for its Settings entry, `top::recom_help(Sora, 6)`).
    pub const GAME_SETTINGS_LABEL: &[u8] = b"Game Settings\0";
}

/// The Settings list (owner, 2026-11-04: "Settings opens a short list"): Re:CoM's own entry
/// buttons relabelled with these rows, as Re:CoM's Deck submenu relabels them (`0x1400E3DE0` mode 0
/// at level 1), run with Re:CoM's Deck-submenu input rule (update state 0x22, `0x1400D9C80`).
/// Checked (decomp). In Sora's story at both Combat settings; Riku's story keeps Re:CoM's page.
///
/// [`Row`] also names the KH1 pages (Items, Customize, Abilities) the KH1 list's own entries open.
pub mod sub {
    use super::text;
    use super::top::Pad;

    /// A page: a Settings-list row, or a KH1 list entry's page.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Row {
        /// Re:CoM's Config page, unchanged (its input `0x1400D3EB0` and draw `0x1400DF1B0`).
        GameSettings,
        /// The Combat settings page (`crate::menu::Page::Combat`).
        Combat,
        /// The Movement settings page (`crate::menu::Page::Movement`).
        Movement,
        /// M13: the Effects & Sound settings page (`crate::menu::Page::EffectsSound`).
        EffectsSound,
        /// M12 camera: the Camera settings page (`crate::menu::Page::Camera`), the fifth entry.
        Camera,
        /// M8 stage 8a.2: KH1's pause-menu Items screen (`crate::items_page`); Sora's list holds
        /// his Keyblade slot, then his item slots.
        Items,
        Customize,
        /// M9 stage 9b: KH1's ability screen with AP (`crate::abilities::page`).
        Abilities,
    }

    /// The Settings list's rows, top to bottom.
    pub const SETTINGS_ROWS: &[Row] = &[Row::GameSettings, Row::Combat, Row::Movement, Row::EffectsSound, Row::Camera];

    /// Riku's story's Settings list (fork testing, `riku_kh1_movement` on): only the pages that
    /// act on Riku (Combat for the switch, Movement for his KH1 movement, Camera: the KH1 / KH2
    /// cameras follow him too). Effects & Sound stays Sora's (his effects are Re:CoM's). 0.149.0
    /// (fork owner, 2026-11-22: "falta poder configurar os shortcuts igual da com o Sora"): Customize
    /// on the top list (0.150.0, [`super::top::Layout::Riku`]: in Review Decks' place, as Sora's).
    pub const RIKU_SETTINGS_ROWS: &[Row] = &[Row::GameSettings, Row::Combat, Row::Movement, Row::Camera];

    /// The Settings list's rows in this story.
    pub fn settings_rows(story: super::Story) -> &'static [Row] {
        if story == super::Story::Riku {
            RIKU_SETTINGS_ROWS
        } else {
            SETTINGS_ROWS
        }
    }

    /// Re:CoM's camp has 7 entry buttons (`camp+0x1B8[7]`); the list reuses them. Checked
    /// (decomp).
    pub const MAX_ROWS: usize = 7;
    const _: () = assert!(SETTINGS_ROWS.len() <= MAX_ROWS);

    impl Row {
        /// The button's label, also the page's header title (header item 1 in the Settings
        /// list, item 0 for a KH1 list entry).
        pub fn label(self) -> &'static [u8] {
            match self {
                Row::GameSettings => text::GAME_SETTINGS_LABEL,
                Row::Items => crate::items_page::text::LABEL,
                Row::Customize => text::CUSTOMIZE_LABEL,
                Row::Abilities => crate::abilities::page::text::LABEL,
                Row::Combat => crate::menu::Page::Combat.label(),
                Row::Movement => crate::menu::Page::Movement.label(),
                Row::EffectsSound => crate::menu::Page::EffectsSound.label(),
                Row::Camera => crate::menu::Page::Camera.label(),
            }
        }

        /// The help line while the row (or entry) is highlighted; None: Re:CoM's message
        /// ([`Row::recom_help`]).
        pub fn help(self) -> Option<&'static [u8]> {
            match self {
                Row::GameSettings => None,
                Row::Items => Some(crate::items_page::text::HELP),
                Row::Customize => Some(text::CUSTOMIZE_HELP),
                Row::Abilities => Some(crate::abilities::page::text::HELP),
                Row::Combat => Some(crate::menu::Page::Combat.help()),
                Row::Movement => Some(crate::menu::Page::Movement.help()),
                Row::EffectsSound => Some(crate::menu::Page::EffectsSound.help()),
                Row::Camera => Some(crate::menu::Page::Camera.help()),
            }
        }

        /// Re:CoM's help message for the row (Game Settings: its Settings entry's).
        pub fn recom_help(self) -> Option<u32> {
            (self == Row::GameSettings).then(|| super::top::recom_help(super::Story::Sora, super::top::SETTINGS)).flatten()
        }

        /// The settings page the row opens, if it is one of ours.
        pub fn settings_page(self) -> Option<crate::menu::Page> {
            match self {
                Row::Combat => Some(crate::menu::Page::Combat),
                Row::Movement => Some(crate::menu::Page::Movement),
                Row::EffectsSound => Some(crate::menu::Page::EffectsSound),
                Row::Camera => Some(crate::menu::Page::Camera),
                _ => None,
            }
        }
    }

    /// What one frame of input does in the list.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Act {
        None,
        /// Back to the top list (sound 4).
        Close,
        /// Open the row's page (sound 2).
        Open(usize),
        /// The cursor to this row (sound 1).
        Move(i32),
    }

    /// Re:CoM's Deck submenu (state 0x22, `0x1400DB3F5..`) over `n` rows: cancel first
    /// (`0x14029B410`; Start does nothing there), then confirm (`0x14029B490`), then up / down
    /// (repeat masks 0x101000 / 0x204000): at the ends a held repeat stops and only a fresh press
    /// goes on, and the cursor set `0x1400DEA10` wraps over the count (`camp+0x21C4`). Checked
    /// (decomp).
    pub fn step(cursor: i32, n: usize, p: Pad) -> Act {
        let n = n as i32;
        if p.cancel {
            return Act::Close;
        }
        if p.confirm {
            return if (0..n).contains(&cursor) { Act::Open(cursor as usize) } else { Act::None };
        }
        let mv = super::top::recom_move(cursor, n, Pad { start: false, cancel: false, confirm: false, ..p });
        match mv {
            Some(i) if i < 0 => Act::Move(n - 1),
            Some(i) if i >= n => Act::Move(0),
            Some(i) => Act::Move(i),
            None => Act::None,
        }
    }
}

/// The blade span a Keyblade's hit spheres cover, in the weapon's bone parameter (KH1 weapon row
/// `+0x24 / +0x28 / +0x2C`, `kh1_core::WeaponData::blade_t`): `|far − near| × fraction scale`.
fn span(w: &kh1_core::WeaponData) -> f32 {
    (w.blade_far - w.blade_near).abs() * w.frac_scale
}

/// The stats panel's values for a Keyblade: KH1's Strength and Magic bonuses ("+3"), and its
/// reach as its blade span over the Kingdom Key's ("x1.00"). The reach is a display figure
/// only (nothing in either game shows it).
pub fn stat_values(k: &Keyblade) -> [String; 3] {
    let kk = span(&kh1_core::WeaponData::kingdom_key());
    let r = if kk > 0.0 { span(&k.weapon) / kk } else { 1.0 };
    [format!("{:+}", k.weapon.strength), format!("{:+}", k.magic), format!("x{r:.2}")]
}

/// NUL-terminated copy for Re:CoM's text items (non-ASCII and NUL bytes become '?').
pub fn c_text(s: &str) -> Vec<u8> {
    let mut v: Vec<u8> = s.bytes().map(|b| if (0x20..0x7F).contains(&b) { b } else { b'?' }).collect();
    v.push(0);
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use kh1_core::WeaponData;

    #[test]
    fn save_byte_falls_back_to_the_kingdom_key() {
        let all = |_: u16| true;
        assert_eq!(equipped_card(0, all), KINGDOM_KEY);
        assert_eq!(equipped_card(1, all), 1);
        assert_eq!(equipped_card(0x11, all), 0x11);
        assert_eq!(equipped_card(0x18, all), 0x18);
        assert_eq!(equipped_card(0x12, all), KINGDOM_KEY, "Soul Eater");
        assert_eq!(equipped_card(0x19, all), KINGDOM_KEY);
        assert_eq!(equipped_card(0xFF, all), KINGDOM_KEY);
        assert_eq!(equipped_card(0x13, |k| k <= 0x11), KINGDOM_KEY, "no numbers yet");
        assert_eq!(save_byte_for(1), 0);
        assert_eq!(save_byte_for(0x0D), 0x0D);
        assert_eq!(save_byte_for(0x12), 0);
        assert_eq!(save_byte_for(0x30), 0);
        assert_eq!(EQUIP_OFFSET, 0x1A74 + 0xC00);
        assert!((SPARE_FIRST..=SPARE_LAST).contains(&EQUIP_OFFSET));
    }

    #[test]
    fn spare_range_ignores_our_byte() {
        let mut r = vec![0u8; SPARE_LAST - SPARE_FIRST + 1];
        assert!(spare_range_nonzero(&r).is_empty());
        r[EQUIP_OFFSET - SPARE_FIRST] = 5;
        assert!(spare_range_nonzero(&r).is_empty());
        // M7's MP block (`crate::mp`) is ours too.
        r[crate::mp::BLOCK_FIRST - SPARE_FIRST] = 1;
        r[crate::mp::BLOCK_LAST - SPARE_FIRST] = 1;
        assert!(spare_range_nonzero(&r).is_empty());
        // M8's items extension right after it is ours too, and M9's ability block after that; the
        // byte after that is not.
        r[crate::items::EXT_FIRST - SPARE_FIRST] = 1;
        r[crate::items::EXT_LAST - SPARE_FIRST] = 9;
        r[crate::abilities::BLOCK_FIRST - SPARE_FIRST] = 1;
        r[crate::abilities::BLOCK_LAST - SPARE_FIRST] = 4;
        assert!(spare_range_nonzero(&r).is_empty());
        // M8b's summon block after the ability block is ours; the byte after it is not.
        r[crate::summons::BLOCK_FIRST - SPARE_FIRST] = 1;
        r[crate::summons::BLOCK_LAST - SPARE_FIRST] = 7;
        assert!(spare_range_nonzero(&r).is_empty());
        // M12's ability extension after the summon block is ours; the byte after it is not.
        r[crate::abilities::EXT_FIRST - SPARE_FIRST] = 3;
        r[crate::abilities::EXT_LAST - SPARE_FIRST] = 3;
        assert!(spare_range_nonzero(&r).is_empty());
        // Enemy Skills' extension right after it is ours; the byte after it is not.
        r[crate::abilities::SKILL_EXT_FIRST - SPARE_FIRST] = 1;
        r[crate::abilities::SKILL_EXT_LAST - SPARE_FIRST] = 1;
        assert!(spare_range_nonzero(&r).is_empty());
        r[crate::abilities::SKILL_EXT_LAST + 1 - SPARE_FIRST] = 3;
        assert_eq!(spare_range_nonzero(&r), vec![crate::abilities::SKILL_EXT_LAST + 1]);
        r[crate::abilities::SKILL_EXT_LAST + 1 - SPARE_FIRST] = 0;
        r[0] = 1;
        r[SPARE_LAST - SPARE_FIRST] = 2;
        assert_eq!(spare_range_nonzero(&r), vec![SPARE_FIRST, SPARE_LAST]);
    }

    #[test]
    fn unlocked_lists_earned_keyblades_with_numbers() {
        let cards: Vec<u16> = (1..=0x11).chain([0x12]).collect();
        // Nothing earned: the Kingdom Key only.
        assert_eq!(unlocked(cards.clone(), |_| false), vec![1]);
        // Plain Oathkeeper, premium Lionheart, Soul Eater "earned" (never listed).
        let e = |id: u16| id == 0x0D || id == (0x0A | PREMIUM) || id == 0x12;
        assert_eq!(unlocked(cards.clone(), e), vec![1, 0x0A, 0x0D]);
        // A Keyblade without numbers is not listed even when earned.
        assert_eq!(unlocked([1, 2], |_| true), vec![1, 2]);
        assert_eq!(unlocked([2, 1, 2, 0x19, 0], |_| true), vec![1, 2]);
        // No Kingdom Key numbers: not listed.
        assert_eq!(unlocked([3], |_| false), Vec::<u16>::new());
    }

    use top::{Entry, Fix, Layout, Pad};
    const NONE: Pad = Pad { start: false, cancel: false, confirm: false, up: false, up_fresh: false, down: false, down_fresh: false };
    const DOWN_HELD: Pad = Pad { down: true, ..NONE };
    const DOWN_FRESH: Pad = Pad { down: true, down_fresh: true, ..NONE };
    const UP_HELD: Pad = Pad { up: true, ..NONE };
    const UP_FRESH: Pad = Pad { up: true, up_fresh: true, ..NONE };

    #[test]
    fn the_list_per_story_and_setting() {
        // Owner, 2026-11-04: nine at KH1 in Sora's story; Re:CoM's seven at Re:CoM (Settings opens
        // the short list); Riku's story and the direct-map camp untouched.
        assert_eq!(top::layout(Story::Sora, false, true, true, false), Layout::Kh1);
        assert_eq!(top::layout(Story::Sora, false, false, true, false), Layout::Settings);
        assert_eq!(top::layout(Story::Sora, false, true, false, false), Layout::Settings, "no Keyblade numbers: no KH1 pages");
        for (c, k) in [(true, true), (true, false), (false, true), (false, false)] {
            assert_eq!(top::layout(Story::Riku, false, c, k, false), Layout::Recom);
            assert_eq!(top::layout(Story::Sora, true, c, k, false), Layout::Recom, "direct map");
            assert_eq!(top::layout(Story::Riku, true, c, k, false), Layout::Recom);
            // Fork testing: `riku_kh1_movement` gives Riku's story the Settings list (not the map camp).
            assert_eq!(top::layout(Story::Riku, false, c, k, true), if c { Layout::Riku } else { Layout::Settings });
            assert_eq!(top::layout(Story::Riku, true, c, k, true), Layout::Recom);
            assert_eq!(top::layout(Story::Sora, false, true, true, true), Layout::Kh1);
        }
        assert!(top::settings_list(Layout::Kh1) && top::settings_list(Layout::Settings) && top::settings_list(Layout::Riku) && !top::settings_list(Layout::Recom));
        // Riku's list: Review Decks' place is Customize; the rest Re:CoM's, where they are.
        assert_eq!(top::entry(Layout::Riku, top::DECKS), Some(Entry::Customize));
        assert_eq!(top::entry(Layout::Riku, 3), Some(Entry::Recom(3)));
        assert_eq!(top::entry(Layout::Riku, top::CUSTOMIZE), None);
        assert_eq!((top::count(Layout::Riku), top::slot(Layout::Riku, 0), top::entry_pos(Layout::Riku, 2)), (7, Some(0), top::entry_pos(Layout::Settings, 2)));
        let confirm = top::Pad { confirm: true, ..Default::default() };
        assert_eq!(top::riku_fix(0, confirm), top::Fix::Confirm(Entry::Customize));
        assert_eq!(top::riku_fix(1, confirm), top::Fix::None);
        assert_eq!(top::riku_fix(0, top::Pad::default()), top::Fix::None);
        assert_eq!((top::count(Layout::Kh1), top::count(Layout::Settings), top::count(Layout::Recom)), (9, 7, 7));
        assert_eq!((Story::from_camp(0), Story::from_camp(1)), (Story::Sora, Story::Riku));
    }

    #[test]
    fn kh1_list_is_the_owners_nine() {
        // Items Abilities Customize World Map Map Cards World Cards Status Journal Settings.
        let on_screen: Vec<Entry> = (0..9).map(|s| top::entry(Layout::Kh1, top::at_slot(Layout::Kh1, s).unwrap()).unwrap()).collect();
        let r = Entry::Recom;
        assert_eq!(on_screen, vec![Entry::Items, Entry::Abilities, Entry::Customize, r(1), r(2), r(3), r(4), r(5), r(6)]);
        // Every camp index once; slot and index are inverses.
        let mut seen = top::KH1_ORDER.to_vec();
        seen.sort_unstable();
        assert_eq!(seen, (0..9).collect::<Vec<i32>>());
        for i in 0..9 {
            assert_eq!(top::at_slot(Layout::Kh1, top::slot(Layout::Kh1, i).unwrap()), Some(i));
        }
        assert_eq!((top::slot(Layout::Kh1, 9), top::at_slot(Layout::Kh1, 9), top::at_slot(Layout::Kh1, -1)), (None, None, None));
        // Re:CoM's lists: its seven, where they are; no entry 7 / 8.
        for l in [Layout::Settings, Layout::Recom] {
            for i in 0..7 {
                assert_eq!((top::slot(l, i), top::at_slot(l, i), top::entry(l, i)), (Some(i), Some(i), Some(r(i))));
            }
            assert_eq!((top::entry(l, 7), top::entry(l, 8), top::slot(l, 7)), (None, None, None));
        }
        // Our entries open our pages; Re:CoM's keep theirs.
        assert_eq!(Entry::Items.page(), Some(sub::Row::Items));
        assert_eq!(Entry::Abilities.page(), Some(sub::Row::Abilities));
        assert_eq!(Entry::Customize.page(), Some(sub::Row::Customize));
        assert_eq!(r(6).page(), None);
        // Settings is Re:CoM's entry 6 in both of Sora's lists (the cursor goes back to it).
        assert_eq!(top::entry(Layout::Kh1, top::SETTINGS), top::entry(Layout::Settings, top::SETTINGS));
    }

    #[test]
    fn positions_fit_the_room() {
        // Re:CoM's own, unchanged (the constants read from the exe: `sites::KEYBLADE_DATA`).
        for s in 0..7 {
            let y = 88.0 + 26.0 * s as f32;
            assert_eq!(top::entry_pos(Layout::Settings, s), (86.0, y));
            assert_eq!(top::bar_pos(Layout::Settings, s), (48.0, 80.0 + 26.0 * s as f32));
            assert_eq!(top::glove_pos(Layout::Recom, s), (150.0, 82.0 + 26.0 * s as f32));
        }
        // KH1's nine: 78 .. 270 in steps of 24; the bar and glove keep Re:CoM's offsets.
        let ys: Vec<f32> = (0..9).map(|s| top::entry_pos(Layout::Kh1, s).1).collect();
        assert_eq!(ys, vec![88.0, 112.0, 136.0, 160.0, 184.0, 208.0, 232.0, 256.0, 280.0]);
        for s in 0..9 {
            let (x, y) = top::entry_pos(Layout::Kh1, s);
            assert_eq!(x, 86.0);
            assert_eq!(top::bar_pos(Layout::Kh1, s), (48.0, y - 8.0));
            assert_eq!(top::glove_pos(Layout::Kh1, s), (150.0, y - 6.0));
        }
        // The first plate (−14) clear of the header frame's lower border (≈ 69.3 on screen), the last
        // plate (+12) and glow (+14) above the bottom panel (≈ 297), the plates (26 high) overlapping by 2.
        assert_eq!(top::entry_pos(Layout::Kh1, 0).1, top::RECOM_Y0);
        assert!(top::entry_pos(Layout::Kh1, 0).1 - 14.0 > 70.0);
        assert!(top::entry_pos(Layout::Kh1, 8).1 + 14.0 < 297.0);
        assert_eq!(26.0 - top::KH1_STEP, 2.0);
    }

    #[test]
    fn kh1_list_moves_in_screen_order() {
        let down = |c| top::kh1_step(c, DOWN_HELD);
        let up = |c| top::kh1_step(c, UP_HELD);
        // Held down from the top walks the nine in order and stops on Settings.
        let mut c = top::DECKS;
        let mut walk = vec![c];
        while let Some(n) = down(c) {
            walk.push(n);
            c = n;
        }
        assert_eq!(walk, top::KH1_ORDER.to_vec());
        // Held up walks back and stops on Items.
        let mut back = vec![c];
        while let Some(n) = up(c) {
            back.push(n);
            c = n;
        }
        back.reverse();
        assert_eq!(back, top::KH1_ORDER.to_vec());
        // A fresh press wraps at both ends.
        assert_eq!(top::kh1_step(top::SETTINGS, DOWN_FRESH), Some(top::DECKS));
        assert_eq!(top::kh1_step(top::DECKS, UP_FRESH), Some(top::SETTINGS));
        // Inside, fresh moves one like held.
        assert_eq!(top::kh1_step(top::CUSTOMIZE, DOWN_FRESH), Some(1));
        assert_eq!(top::kh1_step(1, UP_FRESH), Some(top::CUSTOMIZE));
        // Confirm, cancel and Start are not moves.
        for i in 0..9 {
            for p in [Pad { confirm: true, ..NONE }, Pad { cancel: true, down: true, ..NONE }, Pad { start: true, up: true, ..NONE }, NONE] {
                assert_eq!(top::kh1_step(i, p), None);
            }
        }
    }

    #[test]
    fn kh1_fix_only_where_recoms_rule_differs() {
        // Re:CoM moves (its cursor set then takes our target) wherever the nine move, except a
        // held down on 7 / 8 (Re:CoM's end rule over its 7); never where the nine don't.
        for i in 0..9 {
            for p in [DOWN_HELD, DOWN_FRESH, UP_HELD, UP_FRESH] {
                let recom = top::recom_move(i, top::RECOM_ENTRIES, p).is_some();
                let ours = top::kh1_move(i, p);
                if !recom {
                    let expect = if ours.is_some() { Fix::Move(ours.unwrap()) } else { Fix::None };
                    assert_eq!(top::kh1_fix(i, p), expect, "{i} {p:?}");
                } else {
                    assert!(ours.is_some(), "Re:CoM moves from {i} on {p:?} but the nine don't");
                    assert_eq!(top::kh1_fix(i, p), Fix::None);
                }
            }
        }
        assert_eq!(top::kh1_fix(top::ABILITIES, DOWN_HELD), Fix::Move(top::CUSTOMIZE));
        assert_eq!(top::kh1_fix(top::CUSTOMIZE, DOWN_HELD), Fix::Move(1));
        // Confirm: ours on Items / Abilities / Customize, Re:CoM's on its own; cancel wins.
        let ok = Pad { confirm: true, ..NONE };
        assert_eq!(top::kh1_fix(top::DECKS, ok), Fix::Confirm(Entry::Items));
        assert_eq!(top::kh1_fix(top::ABILITIES, ok), Fix::Confirm(Entry::Abilities));
        assert_eq!(top::kh1_fix(top::CUSTOMIZE, ok), Fix::Confirm(Entry::Customize));
        for i in 1..7 {
            assert_eq!(top::kh1_fix(i, ok), Fix::None);
        }
        assert_eq!(top::kh1_fix(top::DECKS, Pad { confirm: true, cancel: true, ..NONE }), Fix::None, "cancel wins");
        assert_eq!(top::kh1_fix(top::DECKS, Pad { confirm: true, start: true, ..NONE }), Fix::None, "Start wins");
    }

    #[test]
    fn recom_rule_is_recoms() {
        for c in 0..6 {
            assert_eq!(top::recom_move(c, 7, DOWN_HELD), Some(c + 1));
        }
        assert_eq!(top::recom_move(6, 7, DOWN_HELD), None);
        assert_eq!(top::recom_move(6, 7, DOWN_FRESH), Some(7), "the cursor set wraps it to 0");
        assert_eq!(top::recom_move(0, 7, UP_HELD), None);
        assert_eq!(top::recom_move(0, 7, UP_FRESH), Some(-1));
        assert_eq!(top::recom_move(3, 7, Pad { up: true, down: true, ..NONE }), Some(2), "up first");
    }

    use list::{List, Pad as LPad};

    #[test]
    fn list_moves_like_recoms_list() {
        let l = List { cursor: 0, top: 0, count: 5 };
        let down = LPad { down: true, ..Default::default() };
        let (n, moved) = list::step(l, down);
        assert!(moved);
        assert_eq!(n.cursor, 1);
        // Held at the end stops; fresh wraps.
        let end = List { cursor: 4, ..l };
        assert_eq!(list::step(end, down), (end, false));
        assert_eq!(list::step(end, LPad { down: true, down_fresh: true, ..Default::default() }).0.cursor, 0);
        let up = LPad { up: true, ..Default::default() };
        assert_eq!(list::step(l, up), (l, false));
        assert_eq!(list::step(l, LPad { up: true, up_fresh: true, ..Default::default() }).0.cursor, 4);
        // Nothing pressed.
        assert_eq!(list::step(l, LPad::default()), (l, false));
    }

    #[test]
    fn list_pages_like_recoms_list() {
        let l = List { cursor: 2, top: 0, count: 17 };
        let pd = LPad { page_down: true, ..Default::default() };
        let (n, _) = list::step(l, pd);
        assert_eq!((n.cursor, n.top), (10, 8));
        let (n, _) = list::step(n, pd);
        assert_eq!((n.cursor, n.top), (16, 8), "past the end: the last row");
        assert_eq!(list::step(n, pd), (n, false), "held at the end stops");
        assert_eq!(list::step(n, LPad { page_down: true, page_down_fresh: true, ..Default::default() }).0.cursor, 0);
        let pu = LPad { page_up: true, ..Default::default() };
        let (m, _) = list::step(List { cursor: 12, top: 8, count: 17 }, pu);
        assert_eq!((m.cursor, m.top), (4, 0));
        assert_eq!(list::step(List { cursor: 5, top: 0, count: 17 }, pu).0.cursor, 0);
        let first = List { cursor: 0, top: 0, count: 17 };
        assert_eq!(list::step(first, pu), (first, false));
        assert_eq!(list::step(first, LPad { page_up: true, page_up_fresh: true, ..Default::default() }).0.cursor, 16);
    }

    #[test]
    fn top_row_follows_the_cursor() {
        assert_eq!(list::top_for(List { cursor: 9, top: 0, count: 17 }), 2);
        assert_eq!(list::top_for(List { cursor: 3, top: 5, count: 17 }), 3);
        assert_eq!(list::top_for(List { cursor: 6, top: 2, count: 17 }), 2);
        assert_eq!(list::open_on(17, 16), List { cursor: 16, top: 9, count: 17 });
        assert_eq!(list::open_on(3, 9), List { cursor: 2, top: 0, count: 3 });
        assert_eq!(list::bar(List { cursor: 0, top: 0, count: 4 }), (0.0, 1.0));
        assert_eq!(list::bar(List { cursor: 16, top: 8, count: 16 }), (0.5, 0.5));
        assert_eq!(list::bar(List { cursor: 0, top: 0, count: 0 }), (0.0, 1.0));
    }

    /// Owner, 2026-11-05 (recording: equipping Aquatank, row 7 of 11 with rows 3..10 shown): the
    /// re-read after a confirm keeps the window where it was, so the row stays on its line.
    #[test]
    fn a_reread_keeps_the_window_where_it_was() {
        // The recording's case: Enemy Skills, 11 rows, top 3, cursor 7 (Aquatank).
        let shown = List { cursor: 7, top: 3, count: 11 };
        assert_eq!(list::open_on(11, 7).top, 0, "before: opened afresh, Aquatank drawn on the last line");
        let mut s = list::Scroll::default();
        // The owner list (depth 0) was read when the page opened, then the abilities (depth 1).
        assert_eq!(s.reread(List { cursor: 2, top: 0, count: 4 }, 0, 4, 2), List { cursor: 2, top: 0, count: 4 });
        assert_eq!(s.reread(List { cursor: 2, top: 0, count: 4 }, 1, 11, 0), List { cursor: 0, top: 0, count: 11 }, "a new window starts at its top");
        assert_eq!(s.reread(shown, 1, 11, 7), shown, "equip: same window, same scroll");
        // Fewer rows than before (an Items slot list shrinking): no blank rows past the end.
        assert_eq!(s.reread(List { cursor: 10, top: 3, count: 11 }, 1, 9, 8), List { cursor: 8, top: 1, count: 9 });
        assert_eq!(list::keep(4, 3, 6), List { cursor: 3, top: 0, count: 4 });
        // The cursor out of the kept window: moved only as far as it needs.
        assert_eq!(list::keep(17, 15, 3), List { cursor: 15, top: 8, count: 17 });
    }

    /// The word sits where Re:CoM's badge is and stays inside the plate; a marked row's name ends
    /// before it.
    #[test]
    fn the_equipped_word_takes_the_badges_place() {
        use crate::text_fit::boxes::{CAMP_PLATE_RIGHT, CAMP_ROW, CAMP_TEXT_LEFT};
        // Re:CoM's badge box: anchor (22, 14) + (170, −13) + translate (−23, 13) + part (−20..19, −6..6).
        let (bx0, bx1, by0, by1) = (22.0 + 170.0 - 23.0 - 20.0, 22.0 + 170.0 - 23.0 + 19.0, 14.0 - 13.0 + 13.0 - 6.0, 14.0 - 13.0 + 13.0 + 6.0);
        assert_eq!((bx0, bx1, by0, by1), (149.0, 188.0, 8.0, 20.0));
        assert!(badge::RIGHT <= bx1 && badge::RIGHT >= 186.7, "ends where the badge's glyphs end");
        assert!(badge::RIGHT < CAMP_PLATE_RIGHT);
        assert_eq!(badge::ANCHOR_X + badge::X, badge::RIGHT);
        // Its glyphs (9.6 units tall at font 0xE) from the offset + 0.9 (top-aligned) or ending at
        // the offset + 11.0 (bottom-aligned) both fall on the badge's 8..20.
        assert!(badge::Y + 0.9 >= by0 && badge::Y + 0.9 + 9.6 <= by1);
        assert!(badge::Y + 11.4 <= by1 && badge::Y + 11.4 - 9.6 >= by0);
        // The name's box: from 23 to the word's left − 4.
        assert_eq!(badge::name_box(58.0), 187.0 - 58.0 - 4.0 - CAMP_TEXT_LEFT);
        assert_eq!(badge::name_box(500.0), 187.0 - badge::MAX_WIDTH - 4.0 - CAMP_TEXT_LEFT, "the word is squeezed past its widest");
        assert_eq!(badge::name_box(0.0), CAMP_ROW.min(187.0 - 4.0 - CAMP_TEXT_LEFT));
        assert!(badge::name_box(badge::MAX_WIDTH) >= 90.0, "names as long as \"Search Ghost\" (95 units) keep most of their width");
        // Re:CoM's text bytes: NUL-terminated ASCII, no `%`.
        assert_eq!(badge::TEXT.last(), Some(&0));
        assert!(badge::TEXT[..badge::TEXT.len() - 1].iter().all(|&c| c.is_ascii_graphic() && c != b'%'));
        // The colour's channels (ABGR) on the fitted map: about (128, 180, 64).
        let ch = |s: u32| 13.0 + 2.49 * ((badge::COLOUR >> s) & 0xFF) as f32;
        assert!((ch(0) - 128.0).abs() < 3.0 && (ch(8) - 180.0).abs() < 3.0 && (ch(16) - 64.0).abs() < 3.0);
        assert_eq!(badge::COLOUR >> 24, 0x80);
    }

    /// Back to a window restores its scroll; a deeper window opened again starts afresh.
    #[test]
    fn going_back_restores_the_windows_scroll() {
        let mut s = list::Scroll::default();
        s.reread(List::default(), 0, 2, 0);
        s.reread(List { cursor: 0, top: 0, count: 2 }, 1, 9, 8); // Items: Sora's list (9 rows), last slot
        let sora = List { cursor: 4, top: 1, count: 9 };
        assert_eq!(s.reread(sora, 2, 4, 0), List { cursor: 0, top: 0, count: 4 }, "the commands window");
        assert_eq!(s.reread(List { cursor: 1, top: 0, count: 4 }, 1, 9, 4), sora, "done / back: Sora's list as it was");
        assert_eq!(s.reread(sora, 0, 2, 0), List { cursor: 0, top: 0, count: 2 });
        assert_eq!(s.reread(List { cursor: 0, top: 0, count: 2 }, 1, 9, 0), List { cursor: 0, top: 0, count: 9 }, "opened again: afresh");
    }

    #[test]
    fn stats_panel_strings() {
        let k = Keyblade { card: 1, name: "Kingdom Key".into(), source: "kh1".into(), model: String::new(), weapon: WeaponData::kingdom_key(), magic: 0, group: None, textures: vec![] };
        assert_eq!(stat_values(&k), ["+3".to_string(), "+0".into(), "x1.00".into()]);
        let mut long = k.clone();
        long.weapon.blade_far = -7.4;
        long.weapon.strength = -1;
        long.magic = 4;
        assert_eq!(stat_values(&long), ["-1".to_string(), "+4".into(), "x2.00".into()]);
        for s in stat_values(&long) {
            assert!(!s.contains('%'));
        }
    }

    #[test]
    fn texts_are_terminated_ascii_without_codes() {
        let mut all = vec![text::PAGE_HELP, text::STAT_LABELS[0], text::STAT_LABELS[1], text::STAT_LABELS[2], text::CUSTOMIZE_LABEL, text::CUSTOMIZE_HELP, text::GAME_SETTINGS_LABEL];
        for r in [sub::Row::GameSettings, sub::Row::Combat, sub::Row::Movement, sub::Row::EffectsSound, sub::Row::Camera, sub::Row::Items, sub::Row::Customize, sub::Row::Abilities] {
            all.push(r.label());
            all.extend(r.help());
        }
        for s in all {
            assert_eq!(s.last(), Some(&0));
            assert!(s[..s.len() - 1].iter().all(|&b| (0x20..0x7F).contains(&b) && b != b'%'));
            assert!(s.len() - 1 <= crate::menu::HELP_MAX);
        }
        assert_eq!(c_text("One-Winged Angel"), b"One-Winged Angel\0".to_vec());
        assert_eq!(c_text("a\u{e9}"), b"a??\0".to_vec());
    }

    #[test]
    fn settings_list_rows_and_moves_follow_recoms_deck_submenu() {
        use sub::{Act, Row};
        // Owner, 2026-11-04: Game Settings (Re:CoM's page), Combat, Movement.
        assert_eq!(sub::SETTINGS_ROWS, &[Row::GameSettings, Row::Combat, Row::Movement, Row::EffectsSound, Row::Camera]);
        assert_eq!(sub::settings_rows(Story::Riku), &[Row::GameSettings, Row::Combat, Row::Movement, Row::Camera]);
        assert!(sub::settings_rows(Story::Riku).len() <= sub::MAX_ROWS);
        assert_eq!(sub::settings_rows(Story::Sora), sub::SETTINGS_ROWS);
        assert_eq!((Row::Camera.label(), Row::Camera.settings_page()), (&b"Camera\0"[..], Some(crate::menu::Page::Camera)));
        assert_eq!((Row::EffectsSound.label(), Row::EffectsSound.settings_page()), (&b"Effects & Sound\0"[..], Some(crate::menu::Page::EffectsSound)));
        assert_eq!(Row::GameSettings.label(), &b"Game Settings\0"[..]);
        assert_eq!((Row::Combat.label(), Row::Movement.label()), (&b"Combat\0"[..], &b"Movement\0"[..]));
        assert_eq!((Row::Items.label(), Row::Customize.label(), Row::Abilities.label()), (&b"Items\0"[..], &b"Customize\0"[..], &b"Abilities\0"[..]));
        assert_eq!(Row::Combat.settings_page(), Some(crate::menu::Page::Combat));
        assert_eq!(Row::Movement.settings_page(), Some(crate::menu::Page::Movement));
        assert_eq!((Row::GameSettings.settings_page(), Row::Items.settings_page()), (None, None));
        // Game Settings' help is Re:CoM's own line for its Settings entry.
        assert_eq!((Row::GameSettings.help(), Row::GameSettings.recom_help()), (None, Some(0x50_0406)));
        assert_eq!(Row::Combat.recom_help(), None);
        for n in [2usize, 3, 5] {
            let last = n as i32 - 1;
            assert_eq!(sub::step(0, n, DOWN_HELD), Act::Move(1));
            assert_eq!(sub::step(last, n, UP_HELD), Act::Move(last - 1));
            assert_eq!(sub::step(last, n, DOWN_HELD), Act::None);
            assert_eq!(sub::step(last, n, DOWN_FRESH), Act::Move(0));
            assert_eq!(sub::step(0, n, UP_HELD), Act::None);
            assert_eq!(sub::step(0, n, UP_FRESH), Act::Move(last));
            assert_eq!(sub::step(n as i32, n, Pad { confirm: true, ..NONE }), Act::None);
        }
        assert_eq!(sub::step(1, 3, Pad { confirm: true, ..NONE }), Act::Open(1));
        assert_eq!(sub::step(1, 3, Pad { confirm: true, cancel: true, ..NONE }), Act::Close);
        assert_eq!(sub::step(0, 3, Pad { start: true, ..NONE }), Act::None);
        assert_eq!(sub::step(0, 3, Pad { confirm: true, down: true, ..NONE }), Act::Open(0), "confirm before moves");
    }

    /// The `mov dword [rbp+d], imm32` immediates (`C7 45 d imm32`) in a checked byte run.
    fn stack_ids(code: &[u8]) -> Vec<u32> {
        let mut v = Vec::new();
        let mut i = 0;
        while i + 7 <= code.len() {
            if code[i] == 0xC7 && code[i + 1] == 0x45 {
                v.push(u32::from_le_bytes([code[i + 3], code[i + 4], code[i + 5], code[i + 6]]));
                i += 7;
            } else {
                i += 1;
            }
        }
        v
    }

    #[test]
    fn top_help_ids_are_the_cursor_sets_own() {
        let site = |rva: u32| crate::sites::KEYBLADE_DATA.iter().find(|s| s.rva == rva).expect("site").expect;
        assert_eq!(stack_ids(site(0x000D_EA45)), top::RECOM_HELP[0].to_vec(), "Sora's story");
        assert_eq!(stack_ids(site(0x000D_EAA9)), top::RECOM_HELP[1].to_vec(), "Riku's story");
        assert_eq!(top::recom_help(Story::Sora, 0), Some(0x50_0400));
        assert_eq!(top::recom_help(Story::Riku, 0), Some(0x50_042A));
        assert_eq!(top::recom_help(Story::Riku, 5), Some(0x50_042C));
        assert_eq!(top::recom_help(Story::Riku, 6), Some(0x50_0406));
        assert_eq!(top::recom_help(Story::Riku, top::ABILITIES), None);
        assert_eq!(top::recom_help(Story::Sora, -1), None);
    }

    #[test]
    fn positions_and_labels_are_recoms_own() {
        let site = |rva: u32| crate::sites::KEYBLADE_DATA.iter().find(|s| s.rva == rva).expect("site").expect;
        let f = |b: &[u8], i: usize| f32::from_le_bytes(b[4 * i..4 * i + 4].try_into().unwrap());
        let entry = site(0x0050_9430);
        assert_eq!((f(entry, 0), f(entry, 1)), (top::ENTRY_X, top::RECOM_Y0));
        assert_eq!(f(site(0x0050_9270), 0), top::RECOM_STEP);
        assert_eq!(f(site(0x0050_9294), 0), top::BAR_X);
        let y0 = site(0x0050_92C0);
        assert_eq!((f(y0, 0), f(y0, 1)), (top::RECOM_BAR_Y0, top::RECOM_GLOVE_Y0));
        assert_eq!(f(site(0x0050_92F4), 0), top::GLOVE_X);
        let labels = site(0x007C_1000);
        let ids: Vec<u32> = labels.chunks(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect();
        assert_eq!(ids, top::RECOM_LABELS.to_vec());
    }
}
