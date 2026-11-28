//! M7 stage 7.13, Customize (decision 34: "Customize: one "KH1" camp entry … with a submenu:
//! Keyblades, Customize"): KH1's shortcut page, the testable part. The page itself is Re:CoM's
//! camp list (`win::keyblades`), opened from the "KH1" entry's submenu (`crate::keyblades::sub`).
//!
//! KH1 (Steam, Checked (decomp + exe); research `m7-menus-mp.md` § 1.4, kept with the decomp
//! tools):
//! - **Slot list** `0x140302950`: three rows, labels sysmsg `0xCE / 0xCF / 0xD0` (table
//!   `0x140548060`), each with its slot's spell name (`0x14028F290`: the command name
//!   `level + (spell + 2) × 3` at the spell's current level) or "none" (sysmsg `0xD4`) when the
//!   slot is empty. Confirm opens the picker for that slot; cancel leaves the page.
//! - **Picker** `0x140302740`, list filled by `0x1403041A0`: the learned spells in KH1's order
//!   (Fire … Aero, `0x140285230(0, spell)`), then "none" as the last row (at most 8 rows). It opens
//!   on the row its cursor was last on (`0x142E9A040`, one value for every slot). Confirm writes the
//!   slot (`0x1402857C0(slot, spell or −1)`, no duplicate check) and goes back to the slot list;
//!   cancel goes back without writing.
//!
//! Ours: the slot bytes of our save block (`crate::mp::off::SHORTCUTS`, 0 = none, else spell + 1,
//! stage 7.12), which the command menu reads every frame, so a change shows in the L1 list at once.
//! The slot rows show the button each slot casts with, as Re:CoM's own button glyph (its font's
//! escapes, [`text::glyph`]; owner, 2026-11-01), or the button's name when Re:CoM's font has no
//! button sheet. KH1's sysmsg file is not in the asset step: "None" and the help lines are our
//! English words (Assumption: KH1's own words for 0xCE..0xD0 / 0xD4 not read); the spell names
//! are KH1's words from `kh1-command-menu.json`, as the command menu shows them.
//!
//! **M12 step 6** (owner, 2026-11-03: "I also would like sleights to be able to be added to the
//! customize menu, for shortcuts"; `docs/recom-m12-sleights.md` § 8.6): the picker lists the
//! learned spells, then the **equipped** sleight abilities (Re:CoM's order, as the Sleights list),
//! then "none". A sleight is saved in the same slot byte as its Re:CoM id
//! (`crate::command_menu::shortcut::slot_sleight`). A slot whose sleight is no longer equipped keeps
//! its byte: the page shows it greyed with a help line saying so ([`unequipped`]), and the L1 list
//! shows the blank command, greyed (as a slot whose spell isn't learned).
//!
//! **0.107.0** (owner, 2026-11-04: "when something is bound to a shortcut, they should also be
//! listed by the button prompt"; `docs/recom-m12-sleights.md` § 8.7): each slot row reads its button
//! glyph and the bound name ([`text::slot_row`]); an empty slot shows the glyph alone.
//! 0.198.18: an empty slot reads the glyph and "None", as KH1's slot list (`0x140302950`: sysmsg `0xD4` on an empty
//! slot).

use crate::command_menu::{cmd, shortcut as sc};

pub const SLOTS: usize = 3;
pub const SPELLS: usize = 7;

/// A picker row: a spell (0..6) or (M12 step 6) a sleight, or None for KH1's last row, "none".
pub type Pick = Option<sc::Slot>;

/// KH1's picker list (`0x1403041A0`): the learned spells (level 1..3) in KH1's order, then (M12
/// step 6) the equipped sleight abilities in Re:CoM's order (`crate::sleights::list`), then "none".
/// Only the 27 sleight abilities (a slot byte holds one of them, `sc::sleight_byte`); the KH1
/// specials the Sleights list also holds (`crate::sleights::SPECIALS`) are never shortcuts.
///
/// 0.111.0 (owner, 2026-11-04: "friend sleights should be able to be put in the shortcuts"): then
/// the Party sleights Re:CoM has taught (`equipped_sleights` holds their ids too: the caller adds
/// them), in the Party list's order (`crate::party::ALL`), before "none".
pub fn picks(levels: [u8; SPELLS], equipped_sleights: &[u16]) -> Vec<Pick> {
    let mut v: Vec<Pick> = (0..SPELLS as u8).filter(|&s| (1..=3).contains(&levels[s as usize])).map(|s| Some(sc::Slot::Spell(s))).collect();
    v.extend(crate::sleights::list(equipped_sleights).into_iter().filter(|&id| sc::sleight_byte(id).is_some()).map(|id| Some(sc::Slot::Sleight(id))));
    v.extend(crate::party::ALL.iter().map(|f| f.sleight).filter(|id| equipped_sleights.contains(id)).map(|id| Some(sc::Slot::Sleight(id))));
    v.push(None);
    v
}

/// KH1's write (`0x1402857C0`): the slot gets the pick, whatever the other slots hold (KH1 has no
/// duplicate check). Our byte: 0 = none, else spell + 1, or a sleight's Re:CoM id.
pub fn set(slots: [u8; SLOTS], slot: usize, pick: Pick) -> [u8; SLOTS] {
    let mut s = slots;
    if slot < SLOTS {
        s[slot] = pick.map_or(0, sc::byte_of);
    }
    s
}

/// A slot holding a sleight that is no longer equipped (the page greys it; the L1 list shows the
/// blank command, greyed). Its Re:CoM id.
pub fn unequipped(slot_byte: u8, equipped_sleights: &[u16]) -> Option<u16> {
    sc::slot_sleight(slot_byte).filter(|id| !equipped_sleights.contains(id))
}

/// The row the picker opens on: KH1 keeps one picker cursor for every slot (`0x142E9A040`) and
/// opens the picker on it. Clamped to the list (the list only grows in KH1: Assumption that a
/// shorter list would clamp).
pub fn picker_cursor(last: i32, count: i32) -> i32 {
    last.clamp(0, (count - 1).max(0))
}

/// The picker row that holds the slot's spell (the None row for an empty slot), for the row mark.
pub fn row_of(picks: &[Pick], slot_byte: u8) -> Option<usize> {
    let want = sc::slot_of(slot_byte);
    picks.iter().position(|&p| p == want)
}

/// A row's look on Re:CoM's camp list (`crate::keyblades::row_look`; owner, 2026-11-01: "in the
/// customize menu, make these slots be blue with the magic symbol"): the three shortcut slots
/// (`picking` false) and the picker's spells are Re:CoM's blue magic row (list entry 0x133 through
/// its row set-up `0x140112F10`); the picker's "None" row is no spell and keeps the red row. The
/// sleights (M12 step 6: Re:CoM's magic-category sleights, `StockSkillList +9` = 2, and the two
/// summon-type ones) take the same blue row as the spells.
pub fn row_look(picking: bool, pick: Pick) -> i32 {
    use crate::keyblades::row_look::{BLUE, RED};
    if !picking || pick.is_some() {
        BLUE
    } else {
        RED
    }
}

/// 0.198.18: the look of slot row `slot` on the slot list (0..2 Riku's or Sora's set, 3..5 Riku's Dark Mode set).
/// Fork owner, 2026-11-27, verbatim: "ficou bom, só que era bom colocar talvez algum ícone que ajudasse o usuário a
/// ver se nn ele só vai ter a escrita pra se guiar, não? pode ser o ícone de heartless que já era pra estar
/// funcionando nos sleights que só o Riku em DarkMode consegue usar". The Dark Mode set's rows take Re:CoM's own
/// row with Dark Riku's wing ([`crate::keyblades::row_look::DARK`], the mark Re:CoM's Sleights List puts on Dark
/// Riku's sleights); the other slots keep the blue magic row ([`row_look`]).
pub fn slot_look(slot: usize) -> i32 {
    if slot >= SLOTS {
        crate::keyblades::row_look::DARK
    } else {
        row_look(false, None)
    }
}

/// The message id of a spell's name at its level (KH1's `0x14028F290`: the command
/// `level + (spell + 2) × 3`, named by its own id). A level outside 1..=3 reads as level 1.
pub fn name_id(spell: u8, level: u8) -> u16 {
    cmd::name(cmd::spell(spell, level.clamp(1, 3)))
}

/// Texts: NUL-terminated for Re:CoM's text items (`C2DFnt`): ASCII and Re:CoM's two-byte button
/// escapes ([`text::glyph`]); no `%` (the help line's formatter treats it as a code).
pub mod text {
    /// Re:CoM's button escapes (owner, 2026-11-01: "instead of saying 'shortcut 1, shortcut 2 etc'
    /// it should be the corresponding button that it would be mapped to"). Two bytes `0xF5 n` in a
    /// `C2DFnt` text: its text copy (`0x1403480C0`) has `0x140345B40` replace them with the glyph of
    /// the button that Re:CoM action is on now (Checked (decomp + exe: the jump table
    /// `0x140345F18`)): the pad type's icon (PS / Xbox / the third type, confirm and cancel
    /// swapped as Re:CoM's setting has them, `0x140349120`) or, with the keyboard, the bound key's
    /// glyph (`0x1403488E0`). The actions are the shortcut buttons' (Checked (decomp + exe): Re:CoM's
    /// key bindings `0x14034B7D0` / `0x14034BC90` tie each glyph's key to these pad bits, PS2
    /// order, defaults at `0x14083F3F0..`).
    pub mod glyph {
        /// `0xF564` → `0x14083F510`: the confirm button (the pad bit at `0x14083F3F0`, Cross or
        /// Circle: what slot 3 casts with).
        pub const CONFIRM: [u8; 2] = [0xF5, 0x64];
        /// `0xF566` → `0x14083F518`: the key bound with pad bit `0x8000` (Square).
        pub const SQUARE: [u8; 2] = [0xF5, 0x66];
        /// `0xF567` → `0x14083F51A`: pad bit `0x1000` (Triangle).
        pub const TRIANGLE: [u8; 2] = [0xF5, 0x67];
        /// `0xF568` → `0x14083F51C`: pad bit `0x400` (L1).
        pub const L1: [u8; 2] = [0xF5, 0x68];
    }

    /// The slot labels with Re:CoM's glyphs: the button each slot casts with (L1 held): Triangle,
    /// Square, confirm (KH1's `0x1402937F0`, stage 7.12).
    pub const SLOT_GLYPHS: [&[u8]; 3] = [b"\xF5\x67\0", b"\xF5\x66\0", b"\xF5\x64\0"];
    /// The same as words, when Re:CoM's font has no button sheet (its "sysfont" has one).
    pub const SLOT_WORDS: [&[u8]; 3] = [b"Triangle\0", b"Square\0", b"Cross\0"];
    pub const SLOT3_WORD_CIRCLE: &[u8] = b"Circle\0";
    /// KH1's "none" (sysmsg 0xD4, not read; our word).
    pub const NONE: &[u8] = b"None\0";
    /// The slot list's help: which buttons cast the slot (stage 7.12: L1 + Triangle / Square /
    /// confirm, KH1's `0x1402937F0`).
    pub const SLOT_HELP_GLYPHS: [&[u8]; 3] = [
        b"Hold \xF5\x68 and press \xF5\x67 to use this shortcut.\0",
        b"Hold \xF5\x68 and press \xF5\x66 to use this shortcut.\0",
        b"Hold \xF5\x68 and press \xF5\x64 to use this shortcut.\0",
    ];
    pub const SLOT_HELP: [&[u8]; 3] = [b"Hold L1 and press Triangle to use this shortcut.\0", b"Hold L1 and press Square to use this shortcut.\0", b"Hold L1 and press Cross to use this shortcut.\0"];
    /// Slot 3's help when Re:CoM's confirm button is Circle.
    pub const SLOT3_HELP_CIRCLE: &[u8] = b"Hold L1 and press Circle to use this shortcut.\0";
    /// M12 step 6: the slot list's help on a slot whose sleight is no longer equipped.
    pub const UNEQUIPPED_HELP: &[u8] = b"Not equipped: equip this sleight on the Abilities page.\0";
    /// 0.111.0: the same for a Party sleight Re:CoM hasn't taught (yet).
    pub const UNLEARNED_HELP: &[u8] = b"Not learned yet: Re:CoM teaches this friend sleight.\0";

    /// 0.198.18: Riku's set holding one of Dark Riku's own sleights (Dark Break, Dark Firaga, Dark Aura, Inverse Burst).
    pub const DARK_ONLY_HELP: &[u8] = b"Dark Mode only: set it in the Dark Mode set (Left/Right).\0";

    /// The help line of a slot holding sleight `id` that isn't in the picker.
    pub fn unequipped_help(id: u16) -> &'static [u8] {
        // 0.198.18: one of Dark Riku's own (Riku's start `0x140216AB0` refuses them) left in Riku's set.
        if crate::riku::DARK_SLEIGHTS.iter().any(|s| s.id == id) && !crate::riku::SLEIGHTS.iter().any(|s| s.id == id) {
            return DARK_ONLY_HELP;
        }
        if crate::party::index(id).is_some() {
            UNLEARNED_HELP
        } else {
            UNEQUIPPED_HELP
        }
    }
    /// The picker's help.
    pub const PICK_HELP_GLYPHS: [&[u8]; 3] = [b"Choose the spell or sleight for \xF5\x68 + \xF5\x67.\0", b"Choose the spell or sleight for \xF5\x68 + \xF5\x66.\0", b"Choose the spell or sleight for \xF5\x68 + \xF5\x64.\0"];
    pub const PICK_HELP: [&[u8]; 3] = [b"Choose the spell or sleight for L1 + Triangle.\0", b"Choose the spell or sleight for L1 + Square.\0", b"Choose the spell or sleight for L1 + Cross.\0"];
    pub const PICK3_HELP_CIRCLE: &[u8] = b"Choose the spell or sleight for L1 + Circle.\0";

    /// Slot `k`'s label (0..2): Re:CoM's glyph, else the button's name.
    pub fn slot_label(k: usize, glyphs: bool, confirm_is_circle: bool) -> &'static [u8] {
        match k.min(2) {
            k if glyphs => SLOT_GLYPHS[k],
            2 if confirm_is_circle => SLOT3_WORD_CIRCLE,
            k => SLOT_WORDS[k],
        }
    }

    /// A slot row's text (owner, 2026-11-04: "when something is bound to a shortcut, they should
    /// also be listed by the button prompt"): the slot's label (its button glyph, or the button's
    /// name), then the bound spell's or sleight's name (`name`, NUL-terminated or not) after a space
    /// (after ": " beside a button's name). 0.198.18: an empty slot (`None`) reads [`NONE`] there, as
    /// KH1's slot list (`0x140302950`) shows sysmsg `0xD4` on an empty slot (it used to show the label alone).
    pub fn slot_row(label: &[u8], name: Option<&[u8]>, glyphs: bool) -> Vec<u8> {
        let strip = |t: &[u8]| -> Vec<u8> { t.iter().copied().take_while(|&b| b != 0).collect() };
        let mut v = strip(label);
        let n = name.map(strip).filter(|n| !n.is_empty()).unwrap_or_else(|| strip(NONE));
        v.extend_from_slice(if glyphs { b" " } else { b": " });
        v.extend(n);
        v.push(0);
        v
    }

    /// The three labels (the status panel's).
    pub fn slot_labels(glyphs: bool, confirm_is_circle: bool) -> [&'static [u8]; 3] {
        [0, 1, 2].map(|k| slot_label(k, glyphs, confirm_is_circle))
    }

    /// The slot list's help for slot `k` (0..2).
    pub fn slot_help(k: usize, glyphs: bool, confirm_is_circle: bool) -> &'static [u8] {
        match k.min(2) {
            k if glyphs => SLOT_HELP_GLYPHS[k],
            2 if confirm_is_circle => SLOT3_HELP_CIRCLE,
            k => SLOT_HELP[k],
        }
    }

    /// 0.198.18: Riku's Dark Mode set (`crate::riku::dark_block`), shown on his page in place of his own three rows
    /// (Left / Right, as Re:CoM's Sleights page switches its tabs, `0x1400EB7E0` state 4): its rows' label prefix and
    /// help line (the same buttons, used while he is in Dark Mode; Left / Right goes back to his own set).
    pub const DARK_PREFIX: &[u8] = b"Dark ";
    pub const DARK_SLOT_HELP_GLYPHS: [&[u8]; 3] = [
        b"In Dark Mode: hold \xF5\x68 and press \xF5\x67. Left/Right: Riku's set.\0",
        b"In Dark Mode: hold \xF5\x68 and press \xF5\x66. Left/Right: Riku's set.\0",
        b"In Dark Mode: hold \xF5\x68 and press \xF5\x64. Left/Right: Riku's set.\0",
    ];
    pub const DARK_SLOT_HELP: [&[u8]; 3] = [
        b"In Dark Mode: hold L1 and press Triangle. Left/Right: Riku's set.\0",
        b"In Dark Mode: hold L1 and press Square. Left/Right: Riku's set.\0",
        b"In Dark Mode: hold L1 and press Cross. Left/Right: Riku's set.\0",
    ];
    pub const DARK_SLOT3_HELP_CIRCLE: &[u8] = b"In Dark Mode: hold L1 and press Circle. Left/Right: Riku's set.\0";
    /// Riku's own set when the Dark Mode set exists: his slot help with the Left / Right hint.
    pub const SWITCH_SLOT_HELP_GLYPHS: [&[u8]; 3] = [
        b"Hold \xF5\x68 and press \xF5\x67. Left/Right: Dark Mode set.\0",
        b"Hold \xF5\x68 and press \xF5\x66. Left/Right: Dark Mode set.\0",
        b"Hold \xF5\x68 and press \xF5\x64. Left/Right: Dark Mode set.\0",
    ];
    pub const SWITCH_SLOT_HELP: [&[u8]; 3] = [
        b"Hold L1 and press Triangle. Left/Right: Dark Mode set.\0",
        b"Hold L1 and press Square. Left/Right: Dark Mode set.\0",
        b"Hold L1 and press Cross. Left/Right: Dark Mode set.\0",
    ];
    pub const SWITCH_SLOT3_HELP_CIRCLE: &[u8] = b"Hold L1 and press Circle. Left/Right: Dark Mode set.\0";
    /// The Dark Mode set's picker help.
    pub const DARK_PICK_HELP_GLYPHS: [&[u8]; 3] = [
        b"Choose the spell or sleight for Dark Mode's \xF5\x68 + \xF5\x67.\0",
        b"Choose the spell or sleight for Dark Mode's \xF5\x68 + \xF5\x66.\0",
        b"Choose the spell or sleight for Dark Mode's \xF5\x68 + \xF5\x64.\0",
    ];
    pub const DARK_PICK_HELP: [&[u8]; 3] = [
        b"Choose the spell or sleight for Dark Mode's L1 + Triangle.\0",
        b"Choose the spell or sleight for Dark Mode's L1 + Square.\0",
        b"Choose the spell or sleight for Dark Mode's L1 + Cross.\0",
    ];
    pub const DARK_PICK3_HELP_CIRCLE: &[u8] = b"Choose the spell or sleight for Dark Mode's L1 + Circle.\0";

    /// A Dark Mode row's label: [`DARK_PREFIX`] then slot `k`'s ([`slot_label`]), NUL-terminated.
    pub fn dark_slot_label(k: usize, glyphs: bool, confirm_is_circle: bool) -> Vec<u8> {
        let mut v = DARK_PREFIX.to_vec();
        v.extend(slot_label(k, glyphs, confirm_is_circle).iter().copied().take_while(|&b| b != 0));
        v.push(0);
        v
    }

    /// The help of Dark Mode's slot `k` (0..2).
    pub fn dark_slot_help(k: usize, glyphs: bool, confirm_is_circle: bool) -> &'static [u8] {
        match k.min(2) {
            k if glyphs => DARK_SLOT_HELP_GLYPHS[k],
            2 if confirm_is_circle => DARK_SLOT3_HELP_CIRCLE,
            k => DARK_SLOT_HELP[k],
        }
    }

    /// The picker's help for slot `k` (0..2).
    pub fn pick_help(k: usize, glyphs: bool, confirm_is_circle: bool) -> &'static [u8] {
        match k.min(2) {
            k if glyphs => PICK_HELP_GLYPHS[k],
            2 if confirm_is_circle => PICK3_HELP_CIRCLE,
            k => PICK_HELP[k],
        }
    }

    /// The help of Riku's own slot `k` (0..2) when his Dark Mode set exists (the Left / Right hint).
    pub fn switch_slot_help(k: usize, glyphs: bool, confirm_is_circle: bool) -> &'static [u8] {
        match k.min(2) {
            k if glyphs => SWITCH_SLOT_HELP_GLYPHS[k],
            2 if confirm_is_circle => SWITCH_SLOT3_HELP_CIRCLE,
            k => SWITCH_SLOT_HELP[k],
        }
    }

    /// The picker's help for Dark Mode's slot `k` (0..2).
    pub fn dark_pick_help(k: usize, glyphs: bool, confirm_is_circle: bool) -> &'static [u8] {
        match k.min(2) {
            k if glyphs => DARK_PICK_HELP_GLYPHS[k],
            2 if confirm_is_circle => DARK_PICK3_HELP_CIRCLE,
            k => DARK_PICK_HELP[k],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 0.198.18: a Dark Riku-only sleight left in Riku's set says where it belongs.
    #[test]
    fn dark_only_sleights_point_to_the_dark_set() {
        assert_eq!(text::unequipped_help(0x60), text::DARK_ONLY_HELP);
        assert_eq!(text::unequipped_help(0x65), text::DARK_ONLY_HELP);
        assert_ne!(text::unequipped_help(0x69), text::DARK_ONLY_HELP, "Dark Impulse: Riku starts it too");
        assert!(text::DARK_ONLY_HELP.len() - 1 <= crate::menu::HELP_MAX);
    }

    /// 0.198.18: Riku's Dark Mode rows say so, with the same buttons.
    #[test]
    fn dark_mode_rows_name_their_buttons() {
        assert_eq!(text::dark_slot_label(0, true, false), b"Dark \xF5\x67\0".to_vec());
        assert_eq!(text::dark_slot_label(2, false, true), b"Dark Circle\0".to_vec());
        assert!(text::dark_slot_help(1, false, false).starts_with(b"In Dark Mode: hold L1 and press Square"));
        assert_eq!(text::dark_slot_help(2, false, true), text::DARK_SLOT3_HELP_CIRCLE);
        for t in text::DARK_SLOT_HELP_GLYPHS.iter().chain(text::DARK_SLOT_HELP.iter()) {
            assert!(t.ends_with(&[0]) && !t.contains(&b'%'));
        }
        // The Dark set's help names it and the way back; Riku's own (when the Dark set exists) the way there.
        for k in 0..3 {
            for (glyphs, circle) in [(true, false), (false, false), (false, true)] {
                assert!(text::dark_slot_help(k, glyphs, circle).windows(10).any(|w| w == b"Left/Right"), "{k}");
                assert!(text::switch_slot_help(k, glyphs, circle).windows(13).any(|w| w == b"Dark Mode set"), "{k}");
                assert!(text::dark_pick_help(k, glyphs, circle).windows(11).any(|w| w == b"Dark Mode's"), "{k}");
            }
            assert!(text::dark_pick_help(k, true, false).windows(2).any(|w| w == &text::SLOT_GLYPHS[k][..2]), "{k}");
            assert!(text::switch_slot_help(k, true, false).windows(2).any(|w| w == &text::SLOT_GLYPHS[k][..2]), "{k}");
        }
        assert_eq!(text::switch_slot_help(2, false, true), text::SWITCH_SLOT3_HELP_CIRCLE);
        assert_eq!(text::dark_pick_help(2, false, true), text::DARK_PICK3_HELP_CIRCLE);
        assert_eq!(text::dark_pick_help(9, false, false), text::DARK_PICK_HELP[2]);
        // An empty Dark row: "Dark" + the glyph + "None" (KH1's sysmsg 0xD4 on an empty slot).
        assert_eq!(text::slot_row(&text::dark_slot_label(0, true, false), None, true), b"Dark \xF5\x67 None\0".to_vec());
    }

    use sc::Slot::{Sleight, Spell};

    #[test]
    fn the_picker_lists_learned_spells_in_kh1_order_then_none() {
        assert_eq!(picks([0; 7], &[]), vec![None]);
        assert_eq!(picks([1, 0, 3, 0, 0, 2, 0], &[]), vec![Some(Spell(0)), Some(Spell(2)), Some(Spell(5)), None]);
        assert_eq!(picks([3; 7], &[]).len(), 8, "KH1's 8 rows at most");
        // A bad level byte is not a learned spell.
        assert_eq!(picks([4, 0, 0, 0, 0, 0, 1], &[]), vec![Some(Spell(6)), None]);
    }

    /// M12 step 6 (owner: sleights "added to the customize menu, for shortcuts"): the equipped
    /// sleights after the spells, in Re:CoM's order; anything not one of the 27 left out.
    #[test]
    fn the_picker_adds_the_equipped_sleights_after_the_spells() {
        use crate::abilities::sleight as sl;
        let p = picks([1, 0, 0, 0, 0, 0, 0], &[sl::HOLY, sl::FIRE_RAID, sl::SONIC_BLADE]);
        assert_eq!(p, vec![Some(Spell(0)), Some(Sleight(sl::FIRE_RAID)), Some(Sleight(sl::HOLY)), None]);
        // The KH1 specials of the Sleights list are never shortcuts (no slot byte).
        assert_eq!(picks([0; 7], &[sl::STRIKE_RAID, sl::SONIC_BLADE, sl::ARS_ARCANUM, sl::RAGNAROK]), vec![None]);
        // Written as the sleight's Re:CoM id; read back to its picker row.
        let slots = set([1, 0, 0], 2, p[2]);
        assert_eq!(slots, [1, 0, sl::HOLY as u8]);
        assert_eq!(row_of(&p, slots[2]), Some(2));
        assert_eq!(set(slots, 2, None), [1, 0, 0]);
        // Unequipped later: not in the picker (no row mark), the slot byte kept and reported.
        let later = picks([1, 0, 0, 0, 0, 0, 0], &[sl::FIRE_RAID]);
        assert_eq!(row_of(&later, slots[2]), None);
        assert_eq!(unequipped(slots[2], &[sl::FIRE_RAID]), Some(sl::HOLY));
        assert_eq!(unequipped(slots[2], &[sl::HOLY]), None);
        assert_eq!(unequipped(slots[0], &[]), None, "a spell slot");
        assert_eq!(unequipped(0, &[]), None);
        // 0.111.0: the taught Party sleights after the sleights, in the Party list's order; one not
        // taught is reported with its own help line.
        let p = picks([0; 7], &[0x4C, sl::HOLY, 0x45, 0x47]);
        assert_eq!(p, vec![Some(Sleight(sl::HOLY)), Some(Sleight(0x45)), Some(Sleight(0x4C)), None], "Goofy Tornado LV2 is not listed");
        assert_eq!(unequipped(0x4E, &[0x45]), Some(0x4E));
        assert_eq!(text::unequipped_help(0x4E), text::UNLEARNED_HELP);
        assert_eq!(text::unequipped_help(sl::HOLY), text::UNEQUIPPED_HELP);
        // Every sleight row is the blue magic row.
        assert_eq!(row_look(true, Some(Sleight(sl::IDYLL_ROMP))), crate::keyblades::row_look::BLUE);
    }

    #[test]
    fn the_write_is_kh1s_without_a_duplicate_check() {
        assert_eq!(set([1, 2, 3], 0, Some(Spell(1))), [2, 2, 3], "Blizzard twice: allowed, as KH1");
        assert_eq!(set([1, 2, 3], 2, None), [1, 2, 0]);
        assert_eq!(set([0, 0, 0], 1, Some(Spell(6))), [0, 7, 0]);
        assert_eq!(set([1, 2, 3], 3, Some(Spell(0))), [1, 2, 3], "no fourth slot");
        assert_eq!(set([0x1B, 0, 0], 1, Some(Sleight(0x1B))), [0x1B, 0x1B, 0], "a sleight twice: allowed too");
    }

    #[test]
    fn the_picker_opens_on_its_last_row_and_marks_the_slots_spell() {
        assert_eq!(picker_cursor(0, 1), 0);
        assert_eq!(picker_cursor(3, 5), 3);
        assert_eq!(picker_cursor(7, 4), 3);
        assert_eq!(picker_cursor(-1, 4), 0);
        let p = picks([2, 0, 1, 0, 0, 0, 0], &[]);
        assert_eq!(row_of(&p, 3), Some(1), "Thunder");
        assert_eq!(row_of(&p, 0), Some(2), "empty: the None row");
        assert_eq!(row_of(&p, 2), None, "Blizzard not learned");
        assert_eq!(row_of(&p, 9), Some(2), "a bad byte reads as none");
    }

    #[test]
    fn slots_and_spells_are_recoms_blue_magic_row() {
        use crate::keyblades::row_look::{BLUE, RED};
        assert_eq!(row_look(false, None), BLUE, "every shortcut slot, empty or not");
        assert_eq!(row_look(false, Some(Spell(2))), BLUE);
        assert_eq!(row_look(true, Some(Spell(0))), BLUE, "Fire");
        assert_eq!(row_look(true, Some(Spell(6))), BLUE, "Aero");
        assert_eq!(row_look(true, None), RED, "the picker's None row is no spell");
        // BLUE is the list's entry 0x133 (the checked sequence table 0x1407C11F0).
        let seqs = crate::sites::KEYBLADE_DATA.iter().find(|s| s.rva == 0x007C_11F0).unwrap().expect;
        assert_eq!(u16::from_le_bytes([seqs[2 * BLUE as usize], seqs[2 * BLUE as usize + 1]]), 0x133);
    }

    /// 0.198.18: Riku's Dark Mode set (slots 3..5) takes Re:CoM's row 0x12F (Dark Riku's wing); slots 0..2 stay blue.
    #[test]
    fn dark_mode_set_takes_recoms_dark_wing_row() {
        use crate::keyblades::row_look::{BLUE, DARK};
        for k in 0..SLOTS {
            assert_eq!(slot_look(k), BLUE, "slot {k}");
            assert_eq!(slot_look(k + SLOTS), DARK, "Dark Mode slot {k}");
        }
        let seqs = crate::sites::KEYBLADE_DATA.iter().find(|s| s.rva == 0x007C_11F0).unwrap().expect;
        assert_eq!(u16::from_le_bytes([seqs[2 * DARK as usize], seqs[2 * DARK as usize + 1]]), 0x12F, "a checked entry of Re:CoM's table");
    }

    #[test]
    fn spell_names_are_the_command_menus_words_at_the_level() {
        assert_eq!(name_id(0, 1), 7, "Fire");
        assert_eq!(name_id(0, 2), 8, "Fira");
        assert_eq!(name_id(6, 3), 27, "Aeroga");
        assert_eq!(name_id(3, 0), 16, "Cure (level 0 reads as 1)");
    }

    /// A text Re:CoM's `C2DFnt` takes: ASCII (no `%`) and button escapes, NUL-terminated; the
    /// visible length (an escape is one glyph).
    fn shown_len(s: &[u8]) -> usize {
        assert_eq!(s.last(), Some(&0));
        let b = &s[..s.len() - 1];
        let (mut i, mut n) = (0, 0);
        while i < b.len() {
            if b[i] == 0xF5 {
                assert!((0x64..=0x84).contains(&b[i + 1]), "{s:x?}");
                i += 2;
            } else {
                assert!((0x20..0x7F).contains(&b[i]) && b[i] != b'%', "{s:x?}");
                i += 1;
            }
            n += 1;
        }
        n
    }

    #[test]
    fn texts_fit_the_help_box() {
        let all = [
            text::NONE,
            text::UNEQUIPPED_HELP,
            text::UNLEARNED_HELP,
            text::SLOT3_HELP_CIRCLE,
            text::SLOT3_WORD_CIRCLE,
            text::PICK3_HELP_CIRCLE,
            text::DARK_SLOT3_HELP_CIRCLE,
            text::SWITCH_SLOT3_HELP_CIRCLE,
            text::DARK_PICK3_HELP_CIRCLE,
        ]
        .into_iter()
        .chain(text::DARK_SLOT_HELP)
        .chain(text::DARK_SLOT_HELP_GLYPHS)
        .chain(text::SWITCH_SLOT_HELP)
        .chain(text::SWITCH_SLOT_HELP_GLYPHS)
        .chain(text::DARK_PICK_HELP)
        .chain(text::DARK_PICK_HELP_GLYPHS)
            .chain(text::SLOT_GLYPHS)
            .chain(text::SLOT_WORDS)
            .chain(text::SLOT_HELP)
            .chain(text::SLOT_HELP_GLYPHS)
            .chain(text::PICK_HELP)
            .chain(text::PICK_HELP_GLYPHS);
        for s in all {
            assert!(shown_len(s) <= crate::menu::HELP_MAX);
        }
        assert_eq!(text::slot_help(2, false, true), text::SLOT3_HELP_CIRCLE);
        assert_eq!(text::slot_help(2, false, false), text::SLOT_HELP[2]);
        assert_eq!(text::slot_help(0, false, true), text::SLOT_HELP[0]);
        assert_eq!(text::pick_help(2, false, true), text::PICK3_HELP_CIRCLE);
        assert_eq!(text::pick_help(9, false, false), text::PICK_HELP[2]);
    }

    /// Owner, 2026-11-04: the bound name beside the button prompt; an empty slot keeps the glyph
    /// alone.
    #[test]
    fn slot_rows_show_the_bound_name_beside_the_button() {
        let tri = text::slot_label(0, true, false);
        assert_eq!(text::slot_row(tri, Some(b"Curaga\0"), true), b"\xF5\x67 Curaga\0".to_vec());
        // 0.198.18: empty reads "None" beside the button, as KH1's slot list (`0x140302950`, sysmsg 0xD4).
        assert_eq!(text::slot_row(tri, None, true), b"\xF5\x67 None\0".to_vec(), "empty: the glyph and None");
        assert_eq!(text::slot_row(tri, Some(b"\0"), true), b"\xF5\x67 None\0".to_vec());
        assert_eq!(text::slot_row(text::slot_label(1, false, false), None, false), b"Square: None\0".to_vec());
        assert_eq!(text::slot_row(text::slot_label(2, false, true), Some(b"Homing Blizzara"), false), b"Circle: Homing Blizzara\0".to_vec());
        // Re:CoM's C2DFnt takes it: ASCII and the escape, NUL-terminated.
        shown_len(&text::slot_row(tri, Some(b"Homing Blizzara\0"), true));
    }

    #[test]
    fn slot_labels_are_the_buttons_they_cast_with() {
        use text::glyph::*;
        // Re:CoM's glyphs: Triangle, Square, then its confirm escape (Cross or Circle as Re:CoM's
        // setting has it; Re:CoM picks the glyph, so one text serves both).
        assert_eq!(text::slot_labels(true, false), [&[TRIANGLE[0], TRIANGLE[1], 0][..], &[SQUARE[0], SQUARE[1], 0][..], &[CONFIRM[0], CONFIRM[1], 0][..]]);
        assert_eq!(text::slot_labels(true, true), text::slot_labels(true, false));
        assert_eq!(text::slot_labels(false, false), [&b"Triangle\0"[..], b"Square\0", b"Cross\0"]);
        assert_eq!(text::slot_label(2, false, true), b"Circle\0");
        // The help names L1 and the slot's button; glyph help and word help say the same.
        for k in 0..3 {
            let g = text::slot_help(k, true, false);
            assert!(g.windows(2).any(|w| w == L1), "{k}");
            assert!(g.windows(2).any(|w| w == &text::SLOT_GLYPHS[k][..2]), "{k}");
            assert!(text::pick_help(k, true, false).windows(2).any(|w| w == &text::SLOT_GLYPHS[k][..2]), "{k}");
        }
        // Re:CoM's escapes: 0xF564 confirm, 0xF566 Square, 0xF567 Triangle, 0xF568 L1.
        assert_eq!(u16::from_be_bytes(CONFIRM), 0xF564);
        assert_eq!(u16::from_be_bytes(SQUARE), 0xF566);
        assert_eq!(u16::from_be_bytes(TRIANGLE), 0xF567);
        assert_eq!(u16::from_be_bytes(L1), 0xF568);
    }
}
