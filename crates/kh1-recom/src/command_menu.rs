//! KH1's command menu (M7 stage 7.3, decision 34): the testable part.
//!
//! KH1 (Steam, Checked (decomp) unless tagged; research `m7-menus-mp.md` § 1, kept with the
//! decomp tools):
//! - **Lists** are 0x38-byte records (`0x140528900` + type × 0x38): `+4` grey mask (drawn grey),
//!   `+8` lock mask (can't run), `+0xC` cursor, `+0x10` count, `+0x14` command ids, `+0x24`
//!   per-row brightness, `+0x34` the row a submenu was opened from. Root (type 0, 4 entries) and
//!   Magic (type 1) are this stage's. The root's 4th entry holds the context command, but KH1 FM
//!   never draws it as a row: the 4th row shows Summon (or the blank dash) and the context
//!   command shows in its own green tab with the Triangle (`0x140278B40`; see **Draw**).
//! - **Build, every frame** (`0x140294220`, `0x1402944C0`, `0x1402948F0`): row 0 Attack (1), or
//!   Counterattack (0x63) while the guard recoil 0x6E plays against a target with the ability;
//!   grey = lock = `!(state flags >> 4) | !1` (Attack needs the movement state's flag 0x10), rows
//!   1..3 the blank command 0x42. With spells learned: row 1 Magic (2), not locked, greyed when no
//!   spell is usable (usable: the state's flag 0x20, MP `0x1402A4C10`, and for the command flag
//!   8 spells the target within the command's range `+0xE`); the Magic list holds the learned
//!   spells (`level + (spell + 2) × 3`), grey = lock for the unusable ones. Row 2 Items (3),
//!   greyed and locked with no item slots. Entry 3 the context command (0 = none), ungreyed when
//!   one is set (`0x140293000`). Summon is the Magic list's extra last entry (command 6, or 0x4F)
//!   once summons are learned (`0x1402944C0`); FM's cursor skips it in the Magic list
//!   (`0x1402931B0`: count − 1) and the draw shows it as the root's 4th row.
//! - **Cursor** (`0x1402931B0`), on the fresh-press word compared for equality: Up / Down move and
//!   wrap; Right opens the row's submenu (sound 6), a locked row sounds 4; Left or cancel closes a
//!   submenu (sound 3; cancel also puts the root cursor on Attack); Left at the root puts the
//!   cursor on Attack (sound 1). No auto-repeat. **0.111.0** (owner, 2026-11-04): Left at the root,
//!   on any row, switches the menu's two pages instead (Re:CoM's cursor sound, the cursor stays on
//!   its row; see **Pages**).
//! - **Step** (`0x1402937F0`): at the root a fresh jump is the jump (no menu input that frame);
//!   Triangle runs the context command when its tab is fully in (`0x140277CF0` state 2); Square
//!   (without Triangle) is the roll / guard (no cursor that frame); then the cursor; then confirm
//!   (and Right on the root's row 3) runs the row (`0x140293410` → `0x140293480`): a locked row
//!   sounds 4, a row with a submenu opens it, a command is issued and the root cursor returns to
//!   Attack. A spell passes KH1's commit gate first (free or in a combo window; Fire also from
//!   frame 14 of its release), else nothing happens and the list stays (M7 stage 7.4). Confirm
//!   (or Right) on the root's row 3 opens the Summon list through the Magic list's last entry
//!   when it is Summon, else it is the error sound (`0x1402937F0`); the cursor never runs the
//!   context command.
//! - **Draw** (`0x14027B870`): root rows at (16, 260), 22 apart; the Magic list at x + 11, at the
//!   Magic row's y when it has fewer than 3 rows, else bottom-anchored at 348 − 22 n. The top
//!   list's rows ease ±32 brightness a frame between 0x73 (grey) and 0xE6; when the top list
//!   changes, every row is set at once (the lists under it to 0x73). The menu fades in / out at
//!   0.1 a frame (`0x140278AC0`) and hides / shows at once for events (`0x140293750`). FM's list
//!   draw (`0x140278B40`) draws the root's rows 0..2, then the context tab, then the 4th row
//!   (the last-row frame) with Summon (the Magic list's last entry, 0x73 / 0xE6 by its grey bit)
//!   or the blank command 0x42 (0x73, 0xE6 under the cursor); the brightness there is not eased.
//! - **Context tab** (`0x140277CF0`, drawn by `0x140278B40`): its own green plate above the
//!   header with the Triangle and the command's word; it slides in over 14 frames (x −10 → 0 by
//!   10/7 and opacity 0 → 1 by 1/7 per half-frame step) and fades out by the opacity. The
//!   "scale" `0x142866D14` only multiplies the alpha of the tab's vertices (`0x142866A44`) and
//!   word (`0x142866A40`): it is an opacity, not an X scale (7.3 read it as one).
//! - **Shortcuts** (stage 7.12, `0x1402937F0`, `0x1402944C0`, `0x14027B870` case 5): while L1 is
//!   held and a slot is set (and Sora has a spell), the shortcut list takes the root's place (any
//!   submenu closes; released, the root returns on Attack); a fresh Triangle / Square / Cross casts
//!   slot 1 / 2 / 3 through the same commit path as the menu (see [`shortcut`]). **M12 step 6**
//!   (owner): a slot may also hold an equipped sleight, run as the Sleights list runs it
//!   (`docs/recom-m12-sleights.md` § 8.6).
//! - **Summons** (M8b, `crate::summons`, `docs/recom-summons.md`): KH1 FM shows Summon (command 6,
//!   the Magic list's extra last entry, `0x1402944C0`) as the root's 4th row; 0.111.0 moves it to
//!   page 2's row 1 (see **Pages**), which opens KH1's Summon list (type 3, `0x1405289A8`):
//!   Cloud first (owner), then the learned summons in learn order (`0x2F + summon`), grey = lock
//!   = not usable (`0x140288F30`, without its once-per-battle test: owner). The row is greyed when
//!   none is usable, greyed and locked while one is out (KH1's in-progress flag; owner: no
//!   Dismiss). The list at x + 11, beside its row (`0x14027B870` case 3's rule). Confirm on an
//!   entry passes KH1's commit gate (free or a combo window, `0x140293480`) and issues it (no menu
//!   sound); the stack unwinds to Attack.
//! - **Items** (M8 stage 8a.1, `0x140294220`, `0x140293480`, `0x14027B870` case 2): row 2 Items
//!   opens KH1's Items list (type 2, `0x140528970`): Sora's filled slots in slot order (command
//!   `item + 0x1E`), at x + 11, at the Items row's y with fewer than 2 entries, else
//!   bottom-anchored at 348 − 22 n. No filled slot: row 2 greyed and locked. A state without the
//!   item flag 0x40 (the air): row 2 greyed but openable, every entry greyed and locked. KH1 greys
//!   no entry for full HP. Confirm on an entry passes KH1's commit gate (free or a combo window)
//!   and issues the item on Sora (no party list: as Cure; `crate::items`).
//!
//! - **Sleights** (M12 step 5, `crate::sleights`, `docs/recom-m12-sleights.md` § 8.5): the 4th row
//!   says "Sleights" on both pages (greyed when none is usable, greyed and locked with none
//!   equipped or while a sleight runs); confirm or Right opens the Sleights list (the equipped
//!   sleight abilities in Re:CoM's order, grey = lock by the summons' gate with the sleight's MP);
//!   confirm on an entry passes KH1's commit gate and issues it (Re:CoM's own sleight start,
//!   `win/summons.rs`).
//! - **Pages** (0.111.0, owner 2026-11-04: "Page 1 (the default) should be Attack, Magic, Items,
//!   Sleights. Page 2 should be Attack, Summon, Party, Sleights"; [`Page`],
//!   `docs/recom-command-pages-party.md`): D-pad Left at the root switches them (Re:CoM's cursor
//!   sound, the cursor stays on its row), kept per save in the summon block's byte (2 = page 2;
//!   older saves' 0 / 1 read page 1). Page 2's Summon row opens the Summon list (greyed and
//!   locked until a summon is learned, greyed when none is usable, greyed and locked while one is
//!   out); its Party row opens the Party list (`crate::party`: the friend sleights whose friend's
//!   card can drop in this battle; greyed when none is usable, greyed and locked when none is
//!   available or a sleight runs). Every list opens beside its own row, bottom-anchored at 348
//!   when it wouldn't fit (KH1's rule for each list, `0x14027B870`).
//! - **The pointing arrow** ([`Scene::arrow`]; owner: "it should have the pointing arrow that
//!   follows your selection on the command menu so people know that you can press left on the D
//!   Pad to switch between the two pages"): beside the root's cursor row while the root is on top;
//!   Re:CoM's own arrow art and motion in `Cockpit:00` ([`arrow`]).
//!
//! Re:CoM side: `win/command_menu.rs`; notes `docs/recom-m7-command-menu.md`.

use crate::json;

/// The words manifest's format line (`kh1-recom-assets` `command_menu::manifest_json`).
pub const FORMAT: &str = "kh1-recom-assets command-menu 1";

/// KH1 command ids (the command table, `btltbl.bin` `+0xA58`; Checked (files)).
pub mod cmd {
    /// No command (no context command).
    pub const NONE: u8 = 0;
    pub const ATTACK: u8 = 1;
    pub const MAGIC: u8 = 2;
    pub const ITEMS: u8 = 3;
    /// Summon (message 6): the Magic list's last entry once summons are learned (M8b).
    pub const SUMMON: u8 = 6;
    pub const TALK: u8 = 4;
    /// KH1's generic Examine (message 5): our word for Re:CoM's warp point. KH1 has no warp or
    /// teleport command (Checked (files): no such word in `Word.bin`); the word is an Assumption,
    /// an owner choice.
    pub const EXAMINE: u8 = 5;
    pub const LIFT: u8 = 0x1C;
    /// KH1's save point command (message 293): the save point's handler `0x1402AD0B0` puts it in
    /// the context entry (`0x140293000(3, 0x36)` at `0x1402AD11F`) and opens the save menu when it
    /// runs. Checked (decomp + exe + files).
    pub const SAVE: u8 = 0x36;
    pub const OPEN: u8 = 0x37;
    /// The blank command rows 1..3 hold until filled (message 305, a dash).
    pub const BLANK: u8 = 0x42;
    /// The blank command's message (305).
    pub const BLANK_NAME: u16 = 305;
    pub const JUMP_ON: u8 = 0x45;
    pub const COUNTER: u8 = 0x63;
    /// Treated as "no context command" by the tab (`0x140277CF0`).
    pub const NOT_CONTEXT: u8 = 0x7C;
    /// 0.111.0: page 2's Party row (ours: KH1's command table has none; free, Checked (code)).
    pub const PARTY: u8 = 0xF1;
    /// Its word: "Party" (ours, the owner's word; [`super::OUR_WORDS`]).
    pub const PARTY_WORD: u16 = 903;

    /// A spell's command: `level + (spell + 2) × 3` (spell 0..6, level 1..3).
    pub const fn spell(spell: u8, level: u8) -> u8 {
        6 + 3 * spell + level
    }

    /// (spell, level) of a spell command.
    pub fn spell_of(c: u8) -> Option<(u8, u8)> {
        (7..=27).contains(&c).then(|| ((c - 7) / 3, (c - 7) % 3 + 1))
    }

    /// The command's name (its record's `+4` message id).
    pub fn name(c: u8) -> u16 {
        match c {
            LIFT => 28,
            SAVE => 293,
            OPEN => 294,
            BLANK => 305,
            JUMP_ON => 313,
            COUNTER => 343,
            PARTY => PARTY_WORD,
            // A held gimmick card's command (ours; `crate::gimmick`).
            crate::gimmick::STAGGER => crate::gimmick::STAGGER_WORD,
            // M8b: the summons (`0x2F + summon`: KH1's words 286..291, Cloud ours).
            c if crate::summons::of_command(c).is_some() => crate::summons::ALL[crate::summons::of_command(c).unwrap_or(0)].word,
            // M12 step 5: the sleights (`0x80 + id`, our words `0x8100 + id`).
            c if crate::sleights::of_command(c).is_some() => crate::sleights::word(crate::sleights::of_command(c).unwrap_or(0)),
            // M9 stage 9c.1: the specials and Sonic Blade's follow-ups (`crate::specials::cmd`).
            c => crate::specials::cmd::name(c).unwrap_or(c as u16),
        }
    }

    /// The list a command opens (record `+8`): Magic 1, Items 2, Summon 3 (Cure / Aero open the
    /// party list 4: with the casts, stage 7.4 / 7.7); ours: the Sleights list 4 (row 3), the
    /// Party list 5 (0.111.0).
    pub fn submenu(c: u8) -> u8 {
        match c {
            MAGIC => 1,
            ITEMS => 2,
            SUMMON => 3,
            PARTY => 5,
            _ => 0,
        }
    }

    /// M8b: the summon of a Summon list command.
    pub fn summon(c: u8) -> Option<usize> {
        crate::summons::of_command(c)
    }

    /// The commands KH1 puts in row 0 instead of Attack (`0x62C0183` over 0x46.., and Talk).
    pub fn row0_special(c: u8) -> bool {
        c == TALK || (c.wrapping_sub(0x46) < 0x1B && (0x062C_0183u32 >> (c - 0x46)) & 1 != 0)
    }
}

/// Re:CoM's field objects as KH1's context commands (`win::command_menu::context` reads them).
pub mod recom {
    use super::cmd;

    /// Kinds (the objects' type getters, vtable `+0x28`). Checked (decomp + exe: RTTI, getters).
    pub mod kind {
        /// `CGimmickDoor` (its getter `0x1401B8A10`: 0x1006, or 0x1005).
        pub const DOOR: u32 = 0x1006;
        /// `CGimmickTreasureBox`, `…Pooh`.
        pub const BOX: u32 = 0x2000;
        /// `CGimmickJumpStand`.
        pub const JUMP_STAND: u32 = 0x2002;
        /// `CGimmickCarry`, `…Tu`.
        pub const CARRY: u32 = 0x2003;
        /// `CGimmickSavePoint` (vtable `0x1405846C8`, getter `0x1401C50B0`).
        pub const SAVE_POINT: u32 = 0x2004;
        /// `CGimmickWarpPoint` (vtable `0x14058F7E0`, getter `0x1401CC1E0`).
        pub const WARP_POINT: u32 = 0x2005;
        /// `ATEL::CAvatar` (NPCs, the 100 Acre minigames).
        pub const NPC: u32 = 0x4000;
    }

    /// The object Re:CoM's field action (`0x1401E80C0`, on Triangle's request bit) acts on:
    /// a box not yet opened → Open, a door → Open, a jump stand → Jump On, a carry → Lift, an NPC
    /// → Talk. The words for Re:CoM's objects: Assumption.
    pub fn field_action(kind: u32, box_opened: bool) -> u8 {
        match kind {
            kind::BOX if !box_opened => cmd::OPEN,
            kind::DOOR => cmd::OPEN,
            kind::JUMP_STAND => cmd::JUMP_ON,
            kind::CARRY => cmd::LIFT,
            kind::NPC => cmd::TALK,
            _ => cmd::NONE,
        }
    }

    /// The save and warp points read Triangle themselves (`0x14029B550`, the fresh pad bit 0x10)
    /// in their updates `0x1401C5180` / `0x1401CC280`, on the object of the lock-on handle
    /// (`player+0x850`, unless 0 or −1, else `+0x84C`), with Re:CoM's action (`player+0x40C`)
    /// 1..4. The save point also needs Sora within its range (its `+0x2C0`, `in_range`: set at a
    /// distance < 0.8 and < 0.2 above it, cleared when he leaves); the warp point does nothing
    /// while `0x14087B84C & 1` (`blocked`, `0x1402345D0`). Checked (decomp + exe).
    pub fn point(kind: u32, action: u32, in_range: bool, blocked: bool) -> u8 {
        if action.wrapping_sub(1) > 3 {
            return cmd::NONE;
        }
        match kind {
            kind::SAVE_POINT if in_range => cmd::SAVE,
            kind::WARP_POINT if !blocked => cmd::EXAMINE,
            _ => cmd::NONE,
        }
    }
}

/// The spells, in KH1's order: Fire, Blizzard, Thunder, Cure, Gravity, Stop, Aero.
pub mod spell {
    pub const COUNT: usize = 7;
    /// MP costs (spell data `+0x30`; `crate::mp::cost`). Checked (files).
    pub const COSTS: [i32; COUNT] = [crate::mp::cost::FIRE, crate::mp::cost::BLIZZARD, crate::mp::cost::THUNDER, crate::mp::cost::CURE, crate::mp::cost::GRAVITY, crate::mp::cost::STOP, crate::mp::cost::AERO];
    /// Command flag 8 (greyed when the target is beyond the range): Thunder, Gravity, Stop.
    /// Checked (files: flags 0xA / 0xE / 0xA).
    pub const RANGED: [bool; COUNT] = [false, false, true, false, true, true, false];

    /// The command's range `+0xE` (KH1 cm). Checked (files).
    pub fn range(spell: u8, level: u8) -> f32 {
        match (spell, level) {
            (1, 1) => 700.0,
            (1, 2) => 900.0,
            (0 | 1, _) => 1200.0,
            (2 | 4 | 5, _) => 1100.0,
            _ => 0.0,
        }
    }
}

/// KH1's movement-state flags (`0x142DB3840 + state × 0x40`, registrations read; research
/// `kh1-movement-motion.md`): 0x10 Attack, 0x20 magic, 0x40 items. Checked (decomp). States not
/// in that table: 0.
pub fn state_flags(state: u32) -> u32 {
    match state {
        0 => 0xFC,
        1 => 0x34,
        2 => 0x3D,
        3 => 0x05,
        4 | 5 => 0x02,
        7 => 0x01,
        8 => 0x76,
        9..=11 => 0x101,
        0xD | 0xE | 0xF | 0x11 => 0x01,
        0x15 => 0x05,
        0x1E => 0x18,
        _ => 0,
    }
}

/// What KH1's build reads, from Re:CoM's side.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct World {
    /// Sora's KH1 movement state.
    pub state: u32,
    /// The Counterattack is offered (row 0 becomes Counter).
    pub counter: bool,
    /// Spell levels, 0 = not learned.
    pub spells: [u8; spell::COUNT],
    /// KH1's MP (MP, max, charge), None when MP isn't live.
    pub mp: Option<(i32, i32, u16)>,
    /// Distance to the target (KH1 cm), when there is one.
    pub target_dist: Option<f32>,
    /// The context command (entry 3, shown in the tab; 0: none).
    pub context: u8,
    /// KH1's commit gate for a spell (`0x140293480`, M7 stage 7.4): Sora is free or in a combo
    /// window (`0x1402A2EC0`) and not taking off, and Re:CoM's action is one KH1's frame runs.
    pub gate_any: bool,
    /// Fire alone is also taken from frame 14 of its release motion (`0x3D` / `0x8A`).
    pub gate_fire: bool,
    /// Stage 7.12: the three shortcut slots as our save block keeps them (KH1 save
    /// `+0x844..0x846`): 0 = none, 1..=7 spell + 1 ([`shortcut::slot_spell`]), or (M12 step 6) a
    /// sleight ability's Re:CoM id ([`shortcut::slot_sleight`]).
    pub shortcuts: [u8; 3],
    /// M8 stage 8a.1: Sora's item slots (KH1 party record `+0x22..`: item ids, 0 = empty) and their
    /// count (`+0x21`).
    pub slots: [u8; crate::items::MAX_SLOTS],
    pub slot_count: u8,
    /// M9 stage 9c.1: KH1's special offer (`0x14028D0F0`, `crate::specials::offer`) for the context
    /// entry when no object took it: its command (0: none) and whether it is greyed (MP short).
    pub special: u8,
    pub special_grey: bool,
    /// A held Re:CoM gimmick card (`crate::gimmick`): [`crate::gimmick::STAGGER`] when Re:CoM's
    /// own play would take it now, else 0. Takes the context entry after an object's command and
    /// before the special offer (`crate::gimmick::context_entry`).
    pub gimmick: u8,
    /// M9 stage 9c.1: a running special's follow-up in the Attack row (`0x14039ED70`): Rave /
    /// Blast (0: none) and whether it is greyed.
    pub follow: u8,
    pub follow_grey: bool,
    /// M8b: the learned summons in learn order (`crate::summons` indexes; KH1's save `+0x7D0`; the
    /// list puts Cloud first, `crate::summons::list_order`) and
    /// their count.
    pub summons: [u8; crate::summons::COUNT],
    pub summon_count: u8,
    /// M8b: KH1's battle mode (Re:CoM's battle room) and a summon out (Re:CoM's sleight running).
    pub battle: bool,
    pub summon_out: bool,
    /// Sleights outside battle (owner, 2026-11-04): a field room where Re:CoM's sleight code runs
    /// (`crate::summons::Gate::field`) and whether its packs load (`Gate::packs`).
    pub field: bool,
    pub packs: bool,
    /// 0.111.0: the page shown (the save's choice, [`Page`]).
    pub page: Page,
    /// M12 step 5: the equipped sleight abilities' Re:CoM ids (`crate::sleights::list`, Re:CoM's
    /// order) and their count, for the Sleights list and (M12 step 6) the shortcut slots that hold
    /// a sleight.
    pub sleights: [u8; crate::sleights::MAX],
    pub sleight_count: u8,
    /// M12 step 7: the listed KH1 specials (`crate::sleights::SPECIALS`) their prompt would offer
    /// now, by `crate::specials` index bit (`0x14028D0F0`'s score 2).
    pub sleight_specials: u8,
    /// 0.111.0: the Party sleights Re:CoM has taught (`crate::party::Set`) and those available in
    /// this battle (taught, and every card they need can drop here; 0 outside battle).
    pub party_learned: crate::party::Set,
    pub party_avail: crate::party::Set,
}

/// The command menu's page (owner, 2026-11-04): page 1 (the default) Attack, Magic, Items,
/// Sleights; page 2 Attack, Summon, Party, Sleights. D-pad Left switches them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    One,
    Two,
}

impl Page {
    /// The save byte (`crate::summons::field::PAGE`): 2 = page 2. Everything else reads page 1, so
    /// a save from before 0.111.0 (0 Summon, 1 Sleights: M12 step 5's 4th-row choice) opens on page 1.
    pub fn of_byte(b: u8) -> Page {
        if b == 2 {
            Page::Two
        } else {
            Page::One
        }
    }
    pub fn byte(self) -> u8 {
        match self {
            Page::One => 0,
            Page::Two => 2,
        }
    }
    pub fn other(self) -> Page {
        match self {
            Page::One => Page::Two,
            Page::Two => Page::One,
        }
    }
}

/// The fresh presses of one frame (Re:CoM's pad, mapped to KH1's buttons).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pad {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    /// Re:CoM's confirm button (KH1's Cross: run the row).
    pub confirm: bool,
    /// Re:CoM's cancel button (KH1's Circle: back, and the jump).
    pub cancel: bool,
    pub triangle: bool,
    pub square: bool,
    /// Any other button (L1, R1, …): makes the word differ from a lone direction.
    pub other: bool,
    /// L1 **held** (not a fresh press; KH1's held word `+4` & 0x40000): the shortcut list while
    /// held (stage 7.12). Not part of [`Pad::word`].
    pub l1: bool,
}

impl Pad {
    /// KH1's logical fresh word (`0x14028B0B0`'s table `0x140528290`; Checked (decomp + exe)).
    pub fn word(&self) -> u32 {
        let mut w = 0;
        for (on, bits) in [(self.up, 0x20), (self.down, 0x40), (self.left, 0x80), (self.right, 0x100), (self.confirm, 0x21_0001), (self.cancel, 6), (self.triangle, 0x408_0008), (self.square, 0x90_0010), (self.other, 0x4_0000)] {
            if on {
                w |= bits;
            }
        }
        w
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Root,
    Magic,
    /// Stage 7.12: KH1's shortcut list (type 5, `0x140528A18`), in the root's place while L1 is
    /// held.
    Shortcut,
    /// M8 stage 8a.1: KH1's Items list (type 2, `0x140528970`).
    Items,
    /// M8b: KH1's Summon list (type 3, `0x1405289A8`).
    Summon,
    /// M12 step 5: the Sleights list (ours).
    Sleight,
    /// 0.111.0: the Party list (ours, `crate::party`).
    Party,
}

/// One KH1 list.
#[derive(Clone, Debug, PartialEq)]
pub struct List {
    pub kind: Kind,
    pub cmds: Vec<u8>,
    pub grey: u32,
    pub lock: u32,
    pub cursor: i32,
    /// Per-row text brightness (0x73 grey … 0xE6), eased by the draw.
    pub bright: Vec<f32>,
}

impl List {
    fn new(kind: Kind) -> List {
        List { kind, cmds: Vec::new(), grey: !0, lock: !0, cursor: 0, bright: Vec::new() }
    }
    fn count(&self) -> i32 {
        self.cmds.len() as i32
    }
    fn locked(&self, row: i32) -> bool {
        (0..32).contains(&row) && self.lock >> row & 1 != 0
    }
    fn grey_row(&self, row: usize) -> bool {
        row < 32 && self.grey >> row & 1 != 0
    }
}

/// KH1's menu sounds (`0x14017A040`): 1 move, 3 back, 4 error, 6 open. KH1's menu plays no sound
/// when a command (or a spell) is issued: `0x140293480` returns 1 without one (Checked (decomp);
/// the owner's KH1 video: no menu sound on the cast, only the spell's own).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    Move,
    Back,
    Error,
    Open,
    /// 0.111.0: D-pad Left switched the page: Re:CoM's cursor sound in either look (the caller
    /// plays [`Sound::recom_id`] for it).
    Page,
}

impl Sound {
    /// KH1's own id (`0x14017A040`), whose sound the asset step takes from the player's KH1
    /// files (`menu_sounds`). Checked (decomp).
    pub fn kh1_id(self) -> u32 {
        match self {
            Sound::Move | Sound::Page => 1,
            Sound::Back => 3,
            Sound::Error => 4,
            Sound::Open => 6,
        }
    }

    /// Re:CoM's menu sound for it (`0x1402AA360(id, -1)`): 1 cursor, 4 cancel, 5 refused, 2
    /// decide. Assumption (ids from how Re:CoM's camp uses them: 1 on a move, 2 on a confirm, 4
    /// on a close, 5 when a confirm is refused). The fallback when KH1's own sounds are not
    /// there (`menu_sounds`) or the menu has Re:CoM's look.
    pub fn recom_id(self) -> u32 {
        match self {
            Sound::Move | Sound::Page => 1,
            Sound::Back => 4,
            Sound::Error => 5,
            Sound::Open => 2,
        }
    }
}

/// What happened, for the log and the trace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Opened(Kind),
    Closed,
    /// A greyed (locked) spell row was confirmed: the error sound. (command, locked)
    Spell(u8, bool),
    /// A spell was issued (M7 stage 7.4): the cast command, the stack unwinds to Attack.
    Cast(u8),
    /// KH1's commit gate refused the spell this frame (Sora busy): no sound, the list stays.
    NotNow(u8),
    /// A locked row, or a row with nothing to run.
    Refused(u8),
    /// The context command went to Re:CoM's Triangle action.
    Context(u8),
    /// Stage 7.12: the shortcut list came on top (L1 held with a slot set) or went (the root
    /// back on Attack).
    ShortcutList(bool),
    /// Stage 7.12: a shortcut button: (slot 0..2, its command, what happened).
    Shortcut(u8, u8, ShortResult),
    /// M8 stage 8a.1: an Items list entry confirmed: (its command, its slot, what happened:
    /// issued / greyed (the error sound) / KH1's gate refused it).
    Item(u8, u8, ShortResult),
    /// M8b: a Summon list entry confirmed: (its command, what happened).
    Summon(u8, ShortResult),
    /// M12 step 5: a Sleights list entry confirmed: (its command, what happened).
    Sleight(u8, ShortResult),
    /// 0.111.0: a Party list entry confirmed: (its command, what happened).
    Party(u8, ShortResult),
    /// 0.111.0: D-pad Left at the root switched the page (to this).
    Page(Page),
}

/// What a shortcut press did (`0x140293480` on the shortcut list).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortResult {
    /// Issued: the spell is cast this frame (no menu sound).
    Cast,
    /// A greyed slot (empty, not usable, MP short, out of range, a state that can't cast): the
    /// error sound.
    Greyed,
    /// KH1's commit gate refused it (Sora busy): dropped, no sound, not buffered.
    NotNow,
}

/// Re:CoM's request bits after the menu: which of the pad's actions stay.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Outcome {
    /// Re:CoM's Attack request (`+0x424 & 4`) may stay: the confirm was the Attack row's.
    pub attack: bool,
    /// The jump (`& 1`) may stay: KH1 jumps only from the root list.
    pub jump: bool,
    /// Re:CoM's Triangle action (`& 0x200`) may stay. KH1 runs the context command with Triangle
    /// only from the root list and once its tab is in; with no context command shown, Re:CoM's own
    /// action is left to decide (an object this menu doesn't recognise keeps its Triangle).
    pub triangle: bool,
    /// Re:CoM's Square request (`& 2`, KH1's roll / guard / Superglide start) may stay. Only the
    /// shortcut list takes it (stage 7.12: KH1 sets no roll / guard request while the shortcut
    /// list is on top, `0x1402937F0`).
    pub square: bool,
    pub sounds: Vec<Sound>,
    pub events: Vec<Event>,
    /// The spell command issued this frame (M7 stage 7.4), for the cast.
    pub cast: Option<u8>,
    /// The item issued this frame (M8 stage 8a.1): the item and its slot.
    pub item: Option<crate::items::ItemCommand>,
    /// M8b: the summon issued this frame (`crate::summons` index).
    pub summon: Option<usize>,
    /// M12 step 5: the sleight issued this frame (its Re:CoM id; 0.111.0: a Party sleight's too,
    /// `crate::party::index`).
    pub sleight: Option<u16>,
    /// 0.111.0: the page switched this frame (to keep in the save).
    pub page: Option<Page>,
    /// M9 stage 9c.1: a special command run by Triangle this frame (Sonic Blade, Rave, Blast):
    /// ours to run, not Re:CoM's Triangle action.
    pub special: Option<u8>,
    /// M9 stage 9c.1: a confirm KH1's commit refused, or a submenu it opened (`0x140293410`
    /// returns 0 for both): a running special's attacking bit goes.
    pub confirm_refused: bool,
    /// Triangle on "Stagger" this frame: the held gimmick card is played (`win::gimmick`), not
    /// Re:CoM's Triangle action.
    pub gimmick: bool,
}

/// The context tab (`0x140277CF0`): 0 hidden, 1 sliding in, 2 in, 3 fading out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tab {
    pub state: u8,
    /// x offset (`0x140527CA8`, −10 → 0) and opacity (`0x142866D14`, 0 → 1).
    pub x: f32,
    pub alpha: f32,
    /// The command it shows (`0x142866D10`).
    pub last: u8,
}

impl Default for Tab {
    fn default() -> Self {
        Tab { state: 0, x: -10.0, alpha: 0.0, last: 0 }
    }
}

/// KH1's numbers for the brightness, the fade and the tab. Checked (decomp).
pub const BRIGHT_GREY: f32 = 115.0; // 0x73
pub const BRIGHT_ON: f32 = 230.0; // 0xE6
pub const BRIGHT_STEP: f32 = 32.0;
pub const FADE_STEP: f32 = 0.1;
/// The tab's step is half a frame step (`0x14233FBE0`; research § 1.2). Checked (decomp, as read
/// in the research); that it is half the frame step on PC: Assumption.
pub const TAB_STEP: f32 = 0.5;

/// The menu.
#[derive(Clone, Debug, PartialEq)]
pub struct Menu {
    pub root: List,
    pub magic: List,
    /// A submenu is open (the stack is root → [`Menu::sub`]).
    pub open: bool,
    /// Which submenu `open` means: the Magic list or (M8 stage 8a.1) the Items list.
    pub sub: Kind,
    /// M8 stage 8a.1: the Items list and the slot of each entry (`0x142D53A90`).
    pub items: List,
    pub item_slots: Vec<u8>,
    /// M8b: the Summon list, and the Summon entry (root row 3): shown, greyed, locked.
    pub summon: List,
    pub summon_on: bool,
    pub summon_grey: bool,
    pub summon_lock: bool,
    /// 0.111.0: the page shown.
    pub page: Page,
    /// M12 step 5: the Sleights list (Re:CoM ids in `0x80 + id` commands) and the Sleights row's
    /// grey / lock (row 3 on both pages).
    pub sleight: List,
    pub sleight_grey: bool,
    pub sleight_lock: bool,
    /// 0.111.0: the Party list (`crate::party` commands) and page 2's Party row's grey / lock.
    pub party: List,
    pub party_grey: bool,
    pub party_lock: bool,
    /// The fade (`0x14285280C`) and its speed.
    pub alpha: f32,
    pub alpha_speed: f32,
    /// The list type last drawn on top (`0x142852790`).
    top: Option<Kind>,
    pub tab: Tab,
    /// The name the tab last showed (it keeps it while sliding out).
    pub tab_cmd: u8,
    /// Stage 7.12: the shortcut list (type 5): its three commands, grey = lock.
    pub shortcut: List,
    /// The shortcut list is on top (in the root's place).
    pub short_on: bool,
    /// KH1's "a shortcut can show" (`0x142D53A5C`, set by the build): a slot is set and Sora has a
    /// spell (magic is never sealed in Re:CoM).
    pub short_ok: bool,
}

impl Default for Menu {
    fn default() -> Self {
        Menu::new()
    }
}

impl Menu {
    pub fn new() -> Menu {
        let mut root = List::new(Kind::Root);
        root.cmds = vec![cmd::ATTACK, cmd::BLANK, cmd::BLANK, cmd::NONE];
        let mut shortcut = List::new(Kind::Shortcut);
        // The exe's defaults (`0x140528A18`): cursor −1 (no cursor), three entries.
        shortcut.cursor = -1;
        shortcut.cmds = vec![cmd::BLANK; 3];
        Menu {
            root,
            magic: List::new(Kind::Magic),
            open: false,
            sub: Kind::Magic,
            items: List::new(Kind::Items),
            item_slots: Vec::new(),
            summon: List::new(Kind::Summon),
            summon_on: false,
            summon_grey: true,
            summon_lock: true,
            page: Page::One,
            sleight: List::new(Kind::Sleight),
            sleight_grey: true,
            sleight_lock: true,
            party: List::new(Kind::Party),
            party_grey: true,
            party_lock: true,
            alpha: 0.0,
            alpha_speed: 0.0,
            top: None,
            tab: Tab::default(),
            tab_cmd: 0,
            shortcut,
            short_on: false,
            short_ok: false,
        }
    }

    /// The menu appears (the player's HUD is built): KH1's fade-in (`0x140278AC0(1)`).
    pub fn fade_in(&mut self) {
        self.alpha = 0.0;
        self.alpha_speed = FADE_STEP;
    }

    /// Re:CoM's HUD hides (events): KH1's instant hide, and the stack back to the root with the
    /// cursor on Attack when it shows again (`0x1402937F0` with `0x142D53A4C & 8`).
    pub fn hide(&mut self) {
        self.alpha = 0.0;
        self.alpha_speed = 0.0;
        self.open = false;
        self.root.cursor = 0;
        self.tab = Tab::default();
        // The stack's base goes back to the root (`0x1402937F0`): the shortcut list too.
        self.short_on = false;
    }

    /// Re:CoM's HUD shows again: KH1's instant show (`0x140278AC0(2)`).
    pub fn show(&mut self) {
        self.alpha = 1.0;
        self.alpha_speed = 0.0;
    }

    /// KH1's per-frame build of the root and Magic lists.
    pub fn build(&mut self, w: &World) {
        let flags = state_flags(w.state);
        let r = &mut self.root;
        r.cmds = vec![if w.counter { cmd::COUNTER } else { cmd::ATTACK }, cmd::BLANK, cmd::BLANK, cmd::BLANK];
        r.grey = !(flags >> 4) | 0xFFFF_FFFE;
        r.lock = r.grey;
        // M9 stage 9c.1: a running special's follow-up takes row 0 (`0x14039ED70`, the menu callback
        // `0x1402948F0` runs first: `0x140293000(0, cmd)` ungreys it, `0x140292FB0(0)` greys it).
        if w.follow != cmd::NONE {
            r.cmds[0] = w.follow;
            if w.follow_grey {
                r.grey |= 1;
            } else {
                r.grey &= !1;
            }
            r.lock = r.grey;
        }
        // Magic (`0x1402944C0`).
        let learned: Vec<(u8, u8)> = (0..spell::COUNT as u8).filter_map(|s| Some((s, w.spells[s as usize])).filter(|x| (1..=3).contains(&x.1))).collect();
        let mut usable = !0u32;
        // Per spell (KH1's `local_98[spell]`), for the shortcut slots.
        let mut usable_spell = [false; spell::COUNT];
        if flags & 0x20 != 0 {
            for (i, &(s, lv)) in learned.iter().enumerate() {
                let pays = w.mp.is_some_and(|(mp, max, c)| crate::mp::Mp::new(mp, max, c).can_pay(spell::COSTS[s as usize]));
                let in_range = !spell::RANGED[s as usize] || w.target_dist.is_none_or(|d| d <= spell::range(s, lv));
                if pays && in_range {
                    usable &= !(1 << i);
                    usable_spell[s as usize] = true;
                }
            }
        }
        // The summons' gate (M8b), shared by the Summon list, the Sleights list and (M12 step 6) the
        // sleight shortcuts.
        let gate = crate::summons::Gate { battle: w.battle, ground: flags & 0x80 != 0, air: flags & 0x20 != 0 && flags & 0x80 == 0, mp: w.mp.map_or(0, |m| m.0), out: w.summon_out, field: w.field, packs: w.packs };
        let n_sl = (w.sleight_count as usize).min(crate::sleights::MAX);
        let equipped_sl = &w.sleights[..n_sl];
        // The shortcut list (stage 7.12, the same build `0x1402944C0`, only with a spell learned):
        // slot k shows its spell at the spell's current level (`level + (spell + 2) × 3`) or the
        // blank command 0x42; grey = lock = all but the slots whose spell is usable. A slot set
        // makes the list available (`0x142D53A5C` = 1). Re:CoM never seals magic (KH1's save
        // `+0x874` bits 0 / 3): not modelled.
        // M12 step 6: a slot holding a sleight (ours; KH1's slots hold spells only) shows it as the
        // Sleights list does (its command `0x80 + id`), greyed and locked unless the Sleights list's
        // test passes (`crate::sleights::usable`: battle room, ground or its air rule, nothing
        // running, its MP). A sleight no longer equipped shows the blank command, greyed (as a
        // slot whose spell isn't learned); the byte stays, so equipping it again brings it back.
        // A sleight slot makes the list available with or without a spell learned.
        let mut short_grey = !0u32;
        let mut short_cmds = vec![cmd::BLANK; 3];
        let mut short_ok = false;
        for (k, &b) in w.shortcuts.iter().enumerate() {
            match shortcut::slot_of(b) {
                Some(shortcut::Slot::Spell(s)) if !learned.is_empty() => {
                    short_ok = true;
                    let lv = w.spells[s as usize];
                    // A slot whose spell isn't learned (not reachable: slots only get learned
                    // spells) shows the blank command, greyed.
                    if (1..=3).contains(&lv) {
                        short_cmds[k] = cmd::spell(s, lv);
                    }
                    if usable_spell[s as usize] {
                        short_grey &= !(1 << k);
                    }
                }
                // 0.111.0: a Party sleight slot shows its name once Re:CoM taught it, usable as the
                // Party list's entry where it is available in this battle (else greyed).
                Some(shortcut::Slot::Sleight(id)) if crate::party::index(id).is_some() => {
                    short_ok = true;
                    let i = crate::party::index(id).unwrap_or(0);
                    if w.party_learned >> i & 1 != 0 {
                        short_cmds[k] = crate::party::command(i);
                        if w.party_avail >> i & 1 != 0 && crate::party::usable(&gate, i) {
                            short_grey &= !(1 << k);
                        }
                    }
                }
                Some(shortcut::Slot::Sleight(id)) => {
                    short_ok = true;
                    if equipped_sl.iter().any(|&e| e as u16 == id) {
                        short_cmds[k] = crate::sleights::command(id);
                        if crate::sleights::usable(&gate, id) {
                            short_grey &= !(1 << k);
                        }
                    }
                }
                _ => {}
            }
        }
        self.shortcut.cmds = short_cmds;
        self.shortcut.grey = short_grey;
        self.shortcut.lock = short_grey;
        self.shortcut.bright.resize(3, BRIGHT_GREY);
        self.short_ok = short_ok;
        self.magic.cmds = learned.iter().map(|&(s, lv)| cmd::spell(s, lv)).collect();
        self.magic.grey = usable;
        self.magic.lock = usable;
        if !learned.is_empty() {
            r.grey &= !2;
            r.lock &= !2;
            r.cmds[1] = cmd::MAGIC;
            if usable == !0 {
                r.grey |= 2;
            }
        }
        // M8b, `0x1402944C0`: with a spell learned, the summons learned make the Summon list (each
        // greyed and locked when `0x140288F30` says no) and the Magic list's last entry, Summon,
        // shown as the root's 4th row: greyed when none is usable, greyed and locked while one is
        // out (KH1's in-progress flag `0x142867364 & 0x2000`; owner: no Dismiss).
        let n = (w.summon_count as usize).min(crate::summons::COUNT);
        // Owner (2026-11-02): Cloud first, then the others in learn order.
        let learned_summons: Vec<usize> = w.summons[..n].iter().map(|&i| i as usize).filter(|&i| i < crate::summons::COUNT).collect();
        let list = crate::summons::list_order(&learned_summons);
        let mut sgrey = !0u32;
        for (k, &i) in list.iter().enumerate() {
            if crate::summons::usable(&gate, i) {
                sgrey &= !(1 << k);
            }
        }
        self.summon.cmds = list.iter().map(|&i| crate::summons::command(i)).collect();
        self.summon.grey = sgrey;
        self.summon.lock = sgrey;
        let slen = self.summon.cmds.len() as i32;
        if self.summon.cursor >= slen {
            self.summon.cursor = (slen - 1).max(0);
        }
        // 0.111.0 (owner, 2026-11-04): Summon is page 2's row 1, no longer the Magic list's last entry
        // (so no spell is needed): greyed and locked until a summon is learned.
        self.page = w.page;
        let page2 = w.page == Page::Two;
        self.summon_on = page2 && !list.is_empty();
        self.summon_lock = !self.summon_on || w.summon_out;
        self.summon_grey = self.summon_lock || sgrey == !0;
        // M12 step 5: the Sleights list (the equipped sleight abilities), each greyed and locked
        // when the summons' gate with its MP says no; row 3 (both pages) greyed when none is usable,
        // greyed and locked with none equipped or while a sleight runs (as the Summon row's).
        let sl = crate::sleights::list(&equipped_sl.iter().map(|&b| b as u16).collect::<Vec<_>>());
        let mut lgrey = !0u32;
        for (k, &id) in sl.iter().enumerate() {
            let ok = match crate::sleights::special_index(id) {
                // M12 step 7: a KH1 special as its prompt (`crate::sleights::special_usable`).
                Some(_) => crate::sleights::special_usable(&gate, id, w.sleight_specials),
                None => crate::sleights::usable(&gate, id),
            };
            if ok {
                lgrey &= !(1 << k);
            }
        }
        self.sleight.cmds = sl.iter().map(|&id| crate::sleights::command(id)).collect();
        self.sleight.grey = lgrey;
        self.sleight.lock = lgrey;
        let llen = self.sleight.cmds.len() as i32;
        if self.sleight.cursor >= llen {
            self.sleight.cursor = (llen - 1).max(0);
        }
        self.sleight_lock = sl.is_empty() || w.summon_out;
        self.sleight_grey = self.sleight_lock || lgrey == !0;
        // 0.111.0: the Party list (the friend sleights available in this battle), each greyed and
        // locked when the Sleights list's gate with its MP says no; page 2's Party row greyed when
        // none is usable, greyed and locked when none is available or while a sleight runs.
        let pl = crate::party::list(w.party_avail & w.party_learned);
        let mut pgrey = !0u32;
        for (k, &i) in pl.iter().enumerate() {
            if crate::party::usable(&gate, i) {
                pgrey &= !(1 << k);
            }
        }
        self.party.cmds = pl.iter().map(|&i| crate::party::command(i)).collect();
        self.party.grey = pgrey;
        self.party.lock = pgrey;
        let plen = self.party.cmds.len() as i32;
        if self.party.cursor >= plen {
            self.party.cursor = (plen - 1).max(0);
        }
        self.party_lock = !page2 || pl.is_empty() || w.summon_out;
        self.party_grey = self.party_lock || pgrey == !0;
        // Items (M8 stage 8a.1, `0x140294220`; KH1's save `+0x874` bits 0x10 / 2, items disabled /
        // sealed, are never set in Re:CoM): row 2 Items, unlocked; the list holds the filled slots
        // (command `item + 0x1E`, `0x142D53A90[k]` = the slot), each entry's bit cleared. With
        // entries: the cursor clamped; without the state's item flag 0x40, row 2 greyed (not
        // locked) and every entry greyed and locked. Without entries: row 2 greyed and locked.
        r.cmds[2] = cmd::ITEMS;
        r.grey &= !4;
        r.lock &= !4;
        let filled: Vec<(u8, u8)> = (0..(w.slot_count as usize).min(crate::items::MAX_SLOTS)).filter(|&k| w.slots[k] != 0).map(|k| (k as u8, w.slots[k])).collect();
        let mut igrey = !0u32;
        for i in 0..filled.len() {
            igrey &= !(1 << i);
        }
        self.items.cmds = filled.iter().map(|&(_, it)| crate::items::command(it)).collect();
        self.item_slots = filled.iter().map(|&(k, _)| k).collect();
        let ilen = self.items.cmds.len() as i32;
        if ilen > 0 {
            if self.items.cursor >= ilen {
                self.items.cursor = ilen - 1;
            }
            if flags & 0x40 == 0 {
                r.grey |= 4;
                igrey = !0;
            }
        } else {
            r.grey |= 4;
            r.lock |= 4;
        }
        self.items.grey = igrey;
        self.items.lock = igrey;
        // 0.111.0: page 2's rows 1 and 2 (the Magic and Items lists stay built for the shortcuts and
        // the page switch back).
        if page2 {
            let row = |r: &mut List, i: usize, c: u8, grey: bool, lock: bool| {
                r.cmds[i] = c;
                let bit = 1u32 << i;
                r.grey = if grey { r.grey | bit } else { r.grey & !bit };
                r.lock = if lock { r.lock | bit } else { r.lock & !bit };
            };
            row(r, 1, cmd::SUMMON, self.summon_grey, self.summon_lock);
            row(r, 2, cmd::PARTY, self.party_grey, self.party_lock);
        }
        // The context command, entry 3 (`0x1402948F0` clears it, `0x140293000` sets it).
        r.cmds[3] = cmd::NONE;
        // An object's command, then a held gimmick card's "Stagger", then (M9 stage 9c.1) the
        // special offer, which KH1 runs last (`0x14028D0F0`) and only into an entry nothing took;
        // greyed when it scored 1 (MP short).
        let (c, special) = crate::gimmick::context_entry(w.context, w.gimmick, w.special);
        if c != cmd::NONE {
            r.cmds[3] = c;
            if special && w.special_grey {
                r.grey |= 8;
                r.lock |= 8;
            } else {
                r.grey &= !8;
                r.lock &= !8;
            }
        }
        for l in [&mut self.root, &mut self.magic, &mut self.items, &mut self.summon, &mut self.sleight, &mut self.party] {
            l.bright.resize(l.cmds.len(), BRIGHT_GREY);
        }
    }

    /// The open submenu's list.
    pub fn sub_list(&self) -> &List {
        match self.sub {
            Kind::Items => &self.items,
            Kind::Summon => &self.summon,
            Kind::Sleight => &self.sleight,
            Kind::Party => &self.party,
            _ => &self.magic,
        }
    }

    fn sub_list_mut(&mut self) -> &mut List {
        match self.sub {
            Kind::Items => &mut self.items,
            Kind::Summon => &mut self.summon,
            Kind::Sleight => &mut self.sleight,
            Kind::Party => &mut self.party,
            _ => &mut self.magic,
        }
    }

    fn row3_valid(&self) -> bool {
        let c = self.root.cmds[3];
        c != cmd::NONE && c != cmd::BLANK && c != cmd::NOT_CONTEXT && !self.root.grey_row(3)
    }

    fn row0_offers(&self) -> bool {
        cmd::row0_special(self.root.cmds[0]) && !self.root.grey_row(0)
    }

    /// One frame of KH1's menu step for this pad: the build, then the input.
    pub fn step(&mut self, w: &World, pad: Pad) -> Outcome {
        self.build(w);
        // The stack (`0x1402931B0`'s walk): a submenu stays open while its row can open it.
        let row_can_open = match self.sub {
            Kind::Items => self.root.cmds[2] == cmd::ITEMS && !self.root.locked(2) && !self.items.cmds.is_empty(),
            Kind::Summon => self.summon_on && !self.summon_lock && !self.summon.cmds.is_empty(),
            Kind::Sleight => !self.sleight_lock && !self.sleight.cmds.is_empty(),
            Kind::Party => self.root.cmds[2] == cmd::PARTY && !self.party_lock && !self.party.cmds.is_empty(),
            _ => !self.root.locked(1) && self.root.cmds[1] == cmd::MAGIC && !self.magic.cmds.is_empty(),
        };
        if self.open && !row_can_open {
            self.open = false;
        }
        if self.open {
            let l = self.sub_list_mut();
            if l.cursor >= l.count() {
                l.cursor = l.count() - 1;
            }
        }
        // Stage 7.12, the stack's base (`0x1402937F0`, before its input checks): L1 held with the
        // list available swaps the shortcut list in for the root and drops any submenu; L1 released
        // (or the list no longer available) brings the root back with its cursor on Attack. The
        // same frame, no sound.
        let mut pre = Vec::new();
        let want = pad.l1 && self.short_ok;
        if want && !self.short_on {
            self.short_on = true;
            self.open = false;
            pre.push(Event::ShortcutList(true));
        } else if !want && self.short_on {
            self.short_on = false;
            self.open = false;
            self.root.cursor = 0;
            pre.push(Event::ShortcutList(false));
        }
        if self.short_on {
            return self.step_shortcut(w, pad, pre);
        }
        let was_open = self.open;
        let on_attack = !was_open && self.root.cursor == 0;
        let offered = self.row3_valid() || self.row0_offers();
        let mut out = Outcome { attack: on_attack, jump: !was_open, triangle: !was_open && (!offered || self.tab.state == 2), square: true, events: pre, ..Default::default() };
        let word = pad.word();
        // At the root a fresh jump is the jump: no menu input this frame.
        if !was_open && pad.cancel {
            return out;
        }
        // Triangle: the context command, when its tab is in.
        if !was_open && pad.triangle && self.tab.state == 2 && offered {
            let c = if self.row3_valid() { self.root.cmds[3] } else { self.root.cmds[0] };
            out.events.push(Event::Context(c));
            self.root.cursor = 0;
            // M9 stage 9c.1: a special (its start or a follow-up) is ours to run, not Re:CoM's
            // Triangle action.
            if crate::specials::cmd::is_ours(c) {
                out.special = Some(c);
                out.triangle = false;
            }
            // A held gimmick card: ours to play, not Re:CoM's Triangle action.
            if c == crate::gimmick::STAGGER {
                out.gimmick = true;
                out.triangle = false;
            }
        }
        // Square without Triangle: the roll / guard, no cursor this frame.
        if word & 0x400_0010 == 0x10 {
            return out;
        }
        // The cursor.
        let consumed = self.cursor(word, &mut out);
        if consumed {
            out.attack = false;
            return out;
        }
        // Run the row: confirm, or Right on the root's row 3.
        let at3 = !self.open && self.root.cursor == 3;
        let run = word & (0x1_0000 | if at3 { 0x100 } else { 0 }) != 0;
        if !run {
            return out;
        }
        if !self.open {
            match self.root.cursor {
                0 => {
                    if cmd::row0_special(self.root.cmds[0]) {
                        out.sounds.push(Sound::Error);
                        out.attack = false;
                    }
                    // Attack / Counter: Re:CoM's own Attack request, as before the menu.
                }
                3 => {
                    // FM (`0x1402937F0`): row 3 opens its list through `0x140293480` (locked → sound
                    // 4, else the list opens, sound 6); 0.111.0: row 3 is Sleights on both pages.
                    // The context command in entry 3 is Triangle's only.
                    out.attack = false;
                    self.run_row3(&mut out);
                }
                row => {
                    out.attack = false;
                    self.run_root(row, &mut out);
                }
            }
        } else if self.sub == Kind::Items {
            out.attack = false;
            self.run_item(w, &mut out);
        } else if self.sub == Kind::Summon {
            out.attack = false;
            self.run_summon(w, &mut out);
        } else if self.sub == Kind::Sleight {
            out.attack = false;
            self.run_sleight(w, &mut out);
        } else if self.sub == Kind::Party {
            out.attack = false;
            self.run_party(w, &mut out);
        } else {
            out.attack = false;
            let row = self.magic.cursor;
            let c = self.magic.cmds.get(row as usize).copied().unwrap_or(0);
            let locked = self.magic.locked(row);
            // `0x140293480`: a locked row sounds 4; KH1's gate may refuse it (no sound, the list
            // stays); else the command is issued and the stack unwinds to Attack.
            let fire = cmd::spell_of(c).is_some_and(|(s, _)| s == 0);
            if locked || c == 0 {
                out.sounds.push(Sound::Error);
                out.events.push(Event::Spell(c, true));
                out.confirm_refused = true;
            } else if !(w.gate_any || (fire && w.gate_fire)) {
                out.events.push(Event::NotNow(c));
                out.confirm_refused = true;
            } else {
                out.cast = Some(c);
                out.events.push(Event::Cast(c));
                self.open = false;
                self.root.cursor = 0;
            }
        }
        out
    }

    /// Stage 7.12: a frame with the shortcut list on top (`0x1402937F0` with type 5). KH1 skips the
    /// root's Triangle (context) and Square (roll / guard) block and has no cursor here (−1:
    /// `0x1402931B0` returns at once); a fresh jump still jumps and takes the frame (the base
    /// list's jump check); then a fresh Triangle, else Square, else Cross runs slot 1, 2 or 3
    /// (only the first: bits 0x80000 / 0x100000 / 0x200000 tested in that order) through
    /// `0x140293480`: a locked (greyed) slot plays the error sound, KH1's commit gate may refuse it
    /// (dropped silently), else the spell is issued with no menu sound. Confirm is not Attack,
    /// Triangle runs no context command, Square is no roll / guard.
    fn step_shortcut(&mut self, w: &World, pad: Pad, pre: Vec<Event>) -> Outcome {
        let mut out = Outcome { attack: false, jump: true, triangle: false, square: false, events: pre, ..Default::default() };
        if pad.cancel {
            return out;
        }
        let slot = if pad.triangle {
            0
        } else if pad.square {
            1
        } else if pad.confirm {
            2
        } else {
            return out;
        };
        let c = self.shortcut.cmds[slot];
        // M12 step 6: a sleight slot runs as the Sleights list's entry (`run_sleight`): a locked
        // (greyed) one plays the error sound, KH1's commit gate may refuse it (dropped), else
        // Re:CoM's own sleight start this frame (`Outcome::sleight`).
        if let Some(id) = crate::sleights::of_command(c) {
            let result = if self.shortcut.locked(slot as i32) {
                out.sounds.push(Sound::Error);
                ShortResult::Greyed
            } else if !w.gate_any {
                ShortResult::NotNow
            } else {
                out.sleight = Some(id);
                ShortResult::Cast
            };
            out.events.push(Event::Shortcut(slot as u8, c, result));
            return out;
        }
        let fire = cmd::spell_of(c).is_some_and(|(s, _)| s == 0);
        let result = if self.shortcut.locked(slot as i32) || cmd::spell_of(c).is_none() {
            out.sounds.push(Sound::Error);
            ShortResult::Greyed
        } else if !(w.gate_any || (fire && w.gate_fire)) {
            ShortResult::NotNow
        } else {
            out.cast = Some(c);
            ShortResult::Cast
        };
        out.events.push(Event::Shortcut(slot as u8, c, result));
        out
    }

    /// M8 stage 8a.1, `0x140293480` on an Items entry: locked (greyed) → the error sound; KH1's
    /// item command opens the party list (record `+8` = 4, or 6 for the Mega items); with no party
    /// yet it is used on Sora at once (the owner's choice for Cure / Aero, applied to items); KH1's
    /// commit gate (free or a combo window; no Fire-like exception) may refuse it silently; else it
    /// is issued (`0x142D539E4` = the entry's slot) and the stack unwinds to Attack.
    fn run_item(&mut self, w: &World, out: &mut Outcome) {
        let row = self.items.cursor;
        let c = self.items.cmds.get(row as usize).copied().unwrap_or(0);
        let slot = self.item_slots.get(row as usize).copied().unwrap_or(0);
        let item = crate::items::item_of_command(c);
        let result = match item {
            Some(_) if !self.items.locked(row) && !w.gate_any => {
                out.confirm_refused = true;
                ShortResult::NotNow
            }
            Some(it) if !self.items.locked(row) => {
                out.item = Some(crate::items::ItemCommand { item: it, slot });
                self.open = false;
                self.root.cursor = 0;
                ShortResult::Cast
            }
            _ => {
                out.sounds.push(Sound::Error);
                out.confirm_refused = true;
                ShortResult::Greyed
            }
        };
        out.events.push(Event::Item(c, slot, result));
    }

    /// Confirm (or Right) on the root's row 3 (`0x1402937F0` → `0x140293480`): 0.111.0, the Sleights
    /// row on both pages (locked: the error sound).
    fn run_row3(&mut self, out: &mut Outcome) {
        out.confirm_refused = true;
        if self.sleight_lock {
            out.sounds.push(Sound::Error);
            out.events.push(Event::Refused(ROW3_CMD));
        } else {
            self.open_sub(4, out);
        }
    }

    /// 0.111.0: confirm on a Party list entry, as on a Sleights list entry: locked (greyed) → the
    /// error sound; KH1's commit gate (free or a combo window) may refuse it silently; else it is
    /// issued (Re:CoM's own sleight start, no menu sound) and the stack unwinds to Attack.
    fn run_party(&mut self, w: &World, out: &mut Outcome) {
        let row = self.party.cursor;
        let c = self.party.cmds.get(row as usize).copied().unwrap_or(0);
        let result = match crate::party::of_command(c) {
            Some(_) if !self.party.locked(row) && !w.gate_any => {
                out.confirm_refused = true;
                ShortResult::NotNow
            }
            Some(i) if !self.party.locked(row) => {
                out.sleight = Some(crate::party::ALL[i].sleight);
                self.open = false;
                self.root.cursor = 0;
                ShortResult::Cast
            }
            _ => {
                out.sounds.push(Sound::Error);
                out.confirm_refused = true;
                ShortResult::Greyed
            }
        };
        out.events.push(Event::Party(c, result));
    }

    /// M8b, `0x140293480` on a Summon list entry: locked (greyed) → the error sound; KH1's commit
    /// gate (free or a combo window; no Fire-like exception) may refuse it silently; else it is
    /// issued (no menu sound) and the stack unwinds to Attack.
    fn run_summon(&mut self, w: &World, out: &mut Outcome) {
        let row = self.summon.cursor;
        let c = self.summon.cmds.get(row as usize).copied().unwrap_or(0);
        let result = match crate::summons::of_command(c) {
            Some(_) if !self.summon.locked(row) && !w.gate_any => {
                out.confirm_refused = true;
                ShortResult::NotNow
            }
            Some(i) if !self.summon.locked(row) => {
                out.summon = Some(i);
                self.open = false;
                self.root.cursor = 0;
                ShortResult::Cast
            }
            _ => {
                out.sounds.push(Sound::Error);
                out.confirm_refused = true;
                ShortResult::Greyed
            }
        };
        out.events.push(Event::Summon(c, result));
    }

    /// M12 step 5: confirm on a Sleights list entry, as on a Summon list entry: locked (greyed) →
    /// the error sound; KH1's commit gate (free or a combo window) may refuse it silently; else it
    /// is issued (no menu sound) and the stack unwinds to Attack.
    fn run_sleight(&mut self, w: &World, out: &mut Outcome) {
        let row = self.sleight.cursor;
        let c = self.sleight.cmds.get(row as usize).copied().unwrap_or(0);
        let result = match crate::sleights::of_command(c) {
            Some(_) if !self.sleight.locked(row) && !w.gate_any => {
                out.confirm_refused = true;
                ShortResult::NotNow
            }
            Some(id) if !self.sleight.locked(row) => {
                // M12 step 7: a listed KH1 special runs as its prompt's Triangle does (its start
                // command, `crate::specials`), not as Re:CoM's sleight.
                match crate::sleights::special_index(id) {
                    Some(i) => out.special = Some(crate::specials::TABLE[i].1),
                    None => out.sleight = Some(id),
                }
                self.open = false;
                self.root.cursor = 0;
                ShortResult::Cast
            }
            _ => {
                out.sounds.push(Sound::Error);
                out.confirm_refused = true;
                ShortResult::Greyed
            }
        };
        out.events.push(Event::Sleight(c, result));
    }

    fn run_root(&mut self, row: i32, out: &mut Outcome) {
        let c = self.root.cmds[row as usize];
        // `0x140293480` returns 0 for each of these (a locked row, a submenu opened, a command it
        // doesn't run here): `0x140293410` clears the attacking bit (M9 stage 9c.1).
        out.confirm_refused = true;
        if self.root.locked(row) {
            out.sounds.push(Sound::Error);
            out.events.push(Event::Refused(c));
        } else if cmd::submenu(c) != 0 {
            self.open_sub(cmd::submenu(c), out);
        } else {
            out.events.push(Event::Refused(c));
        }
    }

    /// Opens the Magic list (1), the Items list (2), the Summon list (3), (M12 step 5) the
    /// Sleights list (4) or (0.111.0) the Party list (5) (`0x140293480` / `0x1402931B0`: sound 6).
    fn open_sub(&mut self, which: u8, out: &mut Outcome) {
        self.sub = match which {
            2 => Kind::Items,
            3 => Kind::Summon,
            4 => Kind::Sleight,
            5 => Kind::Party,
            _ => Kind::Magic,
        };
        self.open = true;
        out.sounds.push(Sound::Open);
        out.events.push(Event::Opened(self.sub));
        let l = self.sub_list_mut();
        if l.cursor >= l.count() || l.cursor < 0 {
            l.cursor = 0;
        }
    }

    /// `0x1402931B0`: true when the word was the cursor's.
    fn cursor(&mut self, word: u32, out: &mut Outcome) -> bool {
        let open = self.open;
        let (count, cur) = if open { (self.sub_list().count(), self.sub_list().cursor) } else { (4, self.root.cursor) };
        if word == 0x100 {
            if open || cur != 3 {
                if open {
                    // Spells and items open no list (KH1's party list: Cure / Aero / items are
                    // used on Sora; no party yet).
                    if self.sub_list().locked(cur) {
                        out.sounds.push(Sound::Error);
                    }
                } else if self.root.locked(cur) {
                    out.sounds.push(Sound::Error);
                } else if cmd::submenu(self.root.cmds[cur as usize]) != 0 {
                    self.open_sub(cmd::submenu(self.root.cmds[cur as usize]), out);
                }
                return true;
            }
            return false;
        }
        if word == 0x80 || word & 2 != 0 {
            if open {
                self.open = false;
                if word & 2 != 0 && self.root.cursor >= 0 {
                    self.root.cursor = 0;
                }
                out.sounds.push(Sound::Back);
                out.events.push(Event::Closed);
                return true;
            }
            if word & 0x80 != 0 {
                // 0.111.0 (owner, 2026-11-04: "you can press left on the D Pad to switch between the
                // two pages"; M12 step 5 dropped KH1's Left → Attack): Left at the root, on any row,
                // switches the page; the cursor stays on its row (KH1's `0x1402931B0` put it on
                // Attack), Re:CoM's cursor sound.
                self.page = self.page.other();
                out.page = Some(self.page);
                out.sounds.push(Sound::Page);
                out.events.push(Event::Page(self.page));
                return true;
            }
            return false;
        }
        let next = match word {
            0x20 => Some(if cur < 1 { count - 1 } else { cur - 1 }),
            0x40 => Some(if cur >= count - 1 { 0 } else { cur + 1 }),
            _ => None,
        };
        let Some(n) = next else { return false };
        if open {
            self.sub_list_mut().cursor = n;
        } else {
            self.root.cursor = n;
        }
        out.sounds.push(Sound::Move);
        true
    }

    /// The draw's per-frame part: the fade, the rows' brightness, the tab. `step` = frames (1.0
    /// at 60 fps).
    pub fn tick(&mut self, step: f32) {
        if self.alpha_speed != 0.0 {
            self.alpha = (self.alpha + step * self.alpha_speed).clamp(0.0, 1.0);
        }
        // The stack's lists: the shortcut list replaces the root at its base (stage 7.12).
        let top = if self.short_on {
            Kind::Shortcut
        } else if self.open {
            self.sub
        } else {
            Kind::Root
        };
        if self.top != Some(top) {
            self.top = Some(top);
            let (t, under) = match top {
                Kind::Shortcut => (&mut self.shortcut, None),
                Kind::Magic => (&mut self.magic, Some(&mut self.root)),
                Kind::Items => (&mut self.items, Some(&mut self.root)),
                Kind::Summon => (&mut self.summon, Some(&mut self.root)),
                Kind::Sleight => (&mut self.sleight, Some(&mut self.root)),
                Kind::Party => (&mut self.party, Some(&mut self.root)),
                Kind::Root => (&mut self.root, None),
            };
            for (i, b) in t.bright.iter_mut().enumerate() {
                *b = if i < 32 && t.grey >> i & 1 != 0 { BRIGHT_GREY } else { BRIGHT_ON };
            }
            if let Some(u) = under {
                u.bright.iter_mut().for_each(|b| *b = BRIGHT_GREY);
            }
        } else {
            let t = match top {
                Kind::Shortcut => &mut self.shortcut,
                Kind::Magic => &mut self.magic,
                Kind::Items => &mut self.items,
                Kind::Summon => &mut self.summon,
                Kind::Sleight => &mut self.sleight,
                Kind::Party => &mut self.party,
                Kind::Root => &mut self.root,
            };
            for (i, b) in t.bright.iter_mut().enumerate() {
                *b = if i < 32 && t.grey >> i & 1 != 0 { (*b - step * BRIGHT_STEP).trunc().max(BRIGHT_GREY) } else { (*b + step * BRIGHT_STEP).trunc().min(BRIGHT_ON) };
            }
        }
        // The tab steps only inside the root's draw (`0x140277CF0` from `0x140278B40`): it keeps
        // its state while the shortcut list is drawn instead (stage 7.12).
        if !self.short_on {
            self.tab_step(step * TAB_STEP);
        }
    }

    fn tab_restart(&mut self, c: u8) {
        self.tab = Tab { state: 1, x: -10.0, alpha: 0.0, last: c };
        self.tab_cmd = c;
    }

    /// `0x140277CF0`.
    fn tab_step(&mut self, half: f32) {
        if self.alpha == 0.0 {
            self.tab = Tab::default();
            return;
        }
        let r3 = self.row3_valid();
        let gone = !r3 && !self.row0_offers();
        let c = if r3 { self.root.cmds[3] } else { self.root.cmds[0] };
        match self.tab.state {
            0 => {
                if !gone {
                    self.tab_restart(c);
                }
            }
            1 => {
                if gone {
                    self.tab.last = 0;
                    self.tab.state = 3;
                    return;
                }
                if c != self.tab.last && self.tab.last != 0 {
                    self.tab_restart(c);
                }
                if self.tab.alpha < 1.0 {
                    self.tab.alpha = (self.tab.alpha + half * 0.142_857_15).min(1.0);
                }
                if self.tab.x > 0.0 {
                    return;
                }
                self.tab.x += half * 1.428_571_5;
                if self.tab.x <= 0.0 {
                    return;
                }
                self.tab = Tab { state: 2, x: 0.0, alpha: 1.0, last: c };
                self.tab_cmd = c;
            }
            2 => {
                if gone {
                    self.tab.state = 3;
                    self.tab.last = 0;
                } else if c != self.tab.last && self.tab.last != 0 {
                    self.tab_restart(c);
                }
            }
            _ => {
                if gone {
                    self.tab.alpha -= half * 0.142_857_15;
                    if self.tab.alpha <= 0.0 {
                        self.tab = Tab::default();
                    }
                } else {
                    self.tab.last = 0;
                    self.tab.state = 1;
                }
            }
        }
    }

    /// What to draw.
    pub fn scene(&self) -> Scene {
        // Stage 7.12 (`0x14027B870` case 5): only the shortcut panel (the root, its header and the
        // context tab are not drawn); each slot's word at 0x73 when greyed, else 0xE6, set from
        // the grey bits (not the eased bytes).
        if self.short_on {
            let slots = std::array::from_fn(|k| {
                let c = self.shortcut.cmds.get(k).copied().unwrap_or(cmd::BLANK);
                SlotView { name: cmd::name(c), bright: if self.shortcut.grey_row(k) { BRIGHT_GREY as u8 } else { BRIGHT_ON as u8 } }
            });
            return Scene { rows: Vec::new(), alpha: self.alpha, tab: None, shortcut: Some(slots), arrow: None };
        }
        let mut rows = Vec::new();
        let top_root = !self.open;
        for i in 0..4 {
            let cursor_here = top_root && self.root.cursor == i as i32;
            // FM's 4th row (`0x140278B40`), never the context command (that is the tab's); its
            // brightness is set, not eased. 0.111.0: "Sleights" on both pages (0x73 / 0xE6 by its
            // grey bit, as FM's Summon there).
            let (name, bright) = if i == 3 {
                (crate::sleights::ROW_WORD, if self.sleight_grey { BRIGHT_GREY } else { BRIGHT_ON })
            } else {
                (cmd::name(self.root.cmds[i]), self.root.bright.get(i).copied().unwrap_or(BRIGHT_GREY))
            };
            rows.push(RowView { list: Kind::Root, index: i as u8, x: ROOT_X, y: ROOT_Y + PITCH * i as f32, name, highlight: cursor_here, bright: bright as u8 });
        }
        if self.open {
            let l = self.sub_list();
            let n = l.cmds.len();
            // `0x14027B870`: each list at its own row's y unless its rows would end below 348, then
            // bottom-anchored at 348 − 22 n: the Magic list (case 1) at row 1 with fewer than 3
            // entries, the Items list (case 2) at row 2 with fewer than 2, the Summon list (case 3)
            // beside row 3 when y + 20 + (3 + n) × 22 < 369 (one entry): all `min(row y, 348 −
            // 22 n)`. 0.111.0: the Summon list beside page 2's row 1 (it is no longer the Magic list's
            // last entry, so the Magic list's test no longer counts it), the Party list beside row
            // 2, the Sleights list beside row 3.
            let y0 = (ROOT_Y + PITCH * list_row(self.sub) as f32).min(BOTTOM - PITCH * n as f32);
            for (i, &c) in l.cmds.iter().enumerate() {
                rows.push(RowView {
                    list: self.sub,
                    index: i as u8,
                    x: ROOT_X + SUB_DX,
                    y: y0 + PITCH * i as f32,
                    name: cmd::name(c),
                    highlight: l.cursor == i as i32,
                    bright: l.bright.get(i).copied().unwrap_or(BRIGHT_GREY) as u8,
                });
            }
        }
        // The tab: entry 3's word at its eased brightness, all × 0.5 while a submenu is on top
        // (`0x140278B40`'s `fVar25`); its opacity is the tab's own, not the menu's fade.
        let tab = (self.tab.state != 0 && self.tab_cmd != cmd::NONE).then(|| {
            let dim = self.open;
            let b = self.root.bright.get(3).copied().unwrap_or(BRIGHT_GREY);
            TabView { name: cmd::name(self.tab_cmd), x: self.tab.x, alpha: self.tab.alpha, bright: (if dim { b * 0.5 } else { b }) as u8, dim }
        });
        // The pointing arrow: beside the root's cursor row while the root is on top (KH2 FM's; it
        // tells the player D-pad Left switches the pages).
        let arrow = (!self.open && (0..4).contains(&self.root.cursor)).then(|| ArrowView { row: self.root.cursor as u8, x: ROOT_X, y: ROOT_Y + PITCH * self.root.cursor as f32 });
        Scene { rows, alpha: self.alpha, tab, shortcut: None, arrow }
    }

    /// The F9 trace token.
    pub fn trace_token(&self, events: &[String]) -> String {
        if self.short_on {
            let mut t = format!("menu sc g{:x}", self.shortcut.grey & 7);
            for e in events {
                t.push(' ');
                t.push_str(e);
            }
            return t;
        }
        let (tag, gtag) = match self.sub {
            Kind::Items => ("i", "ig"),
            Kind::Summon => ("s", "sg"),
            Kind::Sleight => ("l", "lg"),
            Kind::Party => ("p", "pg"),
            _ => ("m", "mg"),
        };
        let mut t = if self.open { format!("menu {tag}:{}", self.sub_list().cursor) } else { format!("menu r:{}", self.root.cursor) };
        t.push_str(&format!(" g{:x}", self.root.grey & 0xF));
        if self.open {
            let l = self.sub_list();
            t.push_str(&format!(" {gtag}{:x}", l.grey & ((1u64 << l.cmds.len()) - 1) as u32));
        }
        if self.tab.state != 0 {
            t.push_str(&format!(" tab{}:{:#x}", self.tab.state, self.tab_cmd));
        }
        if self.root.cmds[0] == cmd::COUNTER {
            t.push_str(" counter");
        }
        if self.page == Page::Two {
            t.push_str(" p2");
            t.push_str(if self.summon_lock { " sum:lock" } else if self.summon_grey { " sum:grey" } else { " sum" });
            t.push_str(if self.party_lock { " pty:lock" } else if self.party_grey { " pty:grey" } else { " pty" });
        }
        t.push_str(if self.sleight_lock { " sl:lock" } else if self.sleight_grey { " sl:grey" } else { " sl" });
        for e in events {
            t.push(' ');
            t.push_str(e);
        }
        t
    }
}

/// The root row a list opens from (its place, `Menu::scene`): Magic 1, Items 2, Summon (page 2) 1,
/// Party 2, Sleights 3.
pub fn list_row(k: Kind) -> u8 {
    match k {
        Kind::Magic | Kind::Summon => 1,
        Kind::Items | Kind::Party => 2,
        Kind::Sleight => 3,
        Kind::Root | Kind::Shortcut => 0,
    }
}

/// KH1's "Summon" (command 6, message 6, `btltbl.bin`'s Word.bin; Checked (files)).
pub const SUMMON_NAME: u16 = 6;
/// What FM's 4th root row shows without summons (`0x140278B40`): KH1's blank command 0x42
/// (message 305, the dash), greyed; with summons learned (and a spell), Summon ([`SUMMON_NAME`],
/// M8b). Checked (decomp).
pub const ROW3_CMD: u8 = cmd::BLANK;
/// KH1's layout (`0x14027B870`), in the 512 × 384 2D space. Checked (decomp).
pub const ROOT_X: f32 = 16.0;
pub const ROOT_Y: f32 = 260.0;
pub const PITCH: f32 = 22.0;
pub const SUB_DX: f32 = 11.0;
pub const BOTTOM: f32 = 348.0;
/// Re:CoM's plate is drawn around its cap's centre: its left edge 13 units left, its top 13
/// up (parts (−13, −13)…(165, 12)). KH1's (x, y) is taken as the row's top-left. Assumption.
pub const PLATE_ANCHOR: (f32, f32) = (13.0, 13.0);

/// One row to draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowView {
    pub list: Kind,
    pub index: u8,
    /// KH1's row position (top-left, 512 × 384).
    pub x: f32,
    pub y: f32,
    /// The KH1 message id of its text (0: none).
    pub name: u16,
    pub highlight: bool,
    pub bright: u8,
}

impl RowView {
    /// What needs the row rebuilt when it changes (not the brightness).
    pub fn key(&self) -> (Kind, u8, i32, i32, u16, bool) {
        (self.list, self.index, self.x as i32, self.y as i32, self.name, self.highlight)
    }
}

/// FM's context tab (`0x140278B40`, the block after the root's rows), KH1's 2D units. Checked
/// (decomp + files) unless tagged:
/// - its origin is the root list's origin + (9, −10) + (the tab's x offset, 0);
/// - four pieces from picture 2's PC-only rows (UVs set in the code, the rects from `com`'s
///   selection objects 4..7 widened): a shadow, the green plate, a band and a stripe
///   (`kh1_recom_assets::kh1_hud::tab`), drawn over the header;
/// - the word at the origin + (2, 5) + `0x14027A920`'s (16, 2);
/// - the Triangle (`0x140279FC0`, glyph 0x46 of the font's button icons) at the root's x + the
///   tab's x offset − 0x53 (Spanish − 0x57) + the language's offset + `uibin_cut` object 2's x
///   (91), and y − 0x30 + 22 + 23 (`CLanguageControlToUiCommand`, `0x14027C8E0` & al.).
pub mod tab {
    /// The tab's origin from the root list's origin.
    pub const AT: (f32, f32) = (9.0, -10.0);
    /// The word from the tab's origin.
    pub const WORD_AT: (f32, f32) = (18.0, 7.0);
    /// The Triangle's y from the tab's origin, before the glyph's own drop.
    pub const TRIANGLE_Y: f32 = 7.0;
    /// The button glyph's drop (`0x1402D2050`, icons ≥ 100: −48 × 14 / 18 in 1/16 units, −37).
    pub const GLYPH_DY: f32 = -37.0 / 16.0;
    /// The glyph's quad (`0x1402D2050`: 320 × 384 sixteenths × 14 / 18): 15.5 × 18.625 units.
    pub const GLYPH_SIZE: (f32, f32) = (15.5, 18.625);

    /// The Triangle's x from the tab's origin, by the menu language (us, uk, fr, gr, it, sp):
    /// −0x53 (sp −0x57) + the language's `+0x10` (English −2, French / German / Italian −8,
    /// Spanish −13: `0x14027C8E0`, `0x14027C400`, `0x14027C500`, `0x14027C610`, `0x14027C7F0`)
    /// + 91 (`cut_ico_a`) − 9.
    pub fn triangle_x(lang: &str) -> f32 {
        let (base, off) = match lang {
            "fr" | "gr" | "it" => (-0x53, -8),
            "sp" => (-0x57, -13),
            _ => (-0x53, -2),
        };
        (base + off + 91 - 9) as f32
    }
}

/// The context tab to draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TabView {
    /// The KH1 message id of its word.
    pub name: u16,
    /// Its x offset (−10 → 0).
    pub x: f32,
    /// Its opacity (0 → 1): the plate's, the word's and the Triangle's alpha.
    pub alpha: f32,
    /// The word's brightness (× 0.5 while a submenu is on top).
    pub bright: u8,
    /// A submenu is on top: the plate's RGB × 0.5 (the Triangle keeps ×1).
    pub dim: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    pub rows: Vec<RowView>,
    pub alpha: f32,
    /// The context tab, while it shows.
    pub tab: Option<TabView>,
    /// Stage 7.12: the shortcut panel's three slots, while the shortcut list is on top (then
    /// `rows` is empty and `tab` None).
    pub shortcut: Option<[SlotView; 3]>,
    /// 0.111.0: the pointing arrow, while the root is on top ([`arrow`]).
    pub arrow: Option<ArrowView>,
}

impl Scene {
    pub fn same_rows(&self, o: &Scene) -> bool {
        self.rows.len() == o.rows.len() && self.rows.iter().zip(&o.rows).all(|(a, b)| a.key() == b.key()) && self.shortcut.map(|s| s.map(|v| v.name)) == o.shortcut.map(|s| s.map(|v| v.name)) && self.arrow == o.arrow
    }
}

/// The pointing arrow beside the root's cursor row: the row and its KH1 top-left (as its
/// [`RowView`]). The look's offset is [`arrow::at`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArrowView {
    pub row: u8,
    pub x: f32,
    pub y: f32,
}

/// The pointing arrow (owner, 2026-11-04, with a KH2 FM recording: "it should have the pointing arrow
/// that follows your selection on the command menu"; "Re:CoM's own arrow").
///
/// - **Art**: Re:CoM's own white triangle of `Cockpit:00` (sprite UV (16, 49)–(32, 64), one of its
///   lock-on cursor's three pointers, animation group 98; Checked (files: the owner's
///   `FO0002.SPR` / `.RTM`)), turned to point left, in a group the asset step adds after the
///   command menu's plates (`kh1_recom_assets::command_menu::group::ARROW`); Re:CoM loads, plays
///   and draws it with the HUD's set, in the field and in battle.
/// - **Motion** (KH2's, measured on the owner's recording `Recording_2026-11-04_125817.mp4`, 448 ×
///   300 at 30 fps, every frame, the arrow's luminance centroid; one row = 37.7 px = KH1's 22 units,
///   so 1 px = 0.584 unit): **it never blinks** (drawn in all 335 frames); it **sways** left and right
///   about its rest place: ±1.5 units, a period of 31.5 frames (60 Hz) (troughs at 87 / 120 / 151 and
///   peaks at 103 / 134 / 166 frames after a cursor move); after each cursor move it **swings in**
///   from 6 units to the right of its rest, overshoots 4.8 left (frame 13), 2.7 right (30), 2.7 left
///   (51), then sways. Baked into the group (integer translations, Re:CoM's format): keys
///   [`INTRO`], then [`SWAY`] looped from [`LOOP_START`]. Measured (recording; ±1 recorded frame,
///   ±0.5 unit); KH2's own numbers not read (no KH2 decomp in the session).
/// - **Place** ([`at`]): its centre left of the selected row's plate, at the plate's height. KH2's
///   gap (the arrow's right edge 4 units left of the plate) from the recording; the plates' places
///   are ours (KH1's look: the selection plate at the row + (11, 18), the word's centre + 30;
///   Re:CoM's look: the plate's left edge at the row's x, its centre 12 below). Assumption (to judge
///   in game).
pub mod arrow {
    /// The arrow's width and height after the turn (units): KH2's arrow is 10.5 × 16.4 units on the
    /// recording; Re:CoM's 16 × 15 triangle is drawn 16 × 10 before its turn.
    pub const SIZE: (f32, f32) = (10.0, 16.0);
    /// The gap from the arrow's right edge to the plate (recording: 7 px).
    pub const GAP: f32 = 4.0;
    /// The arrow's centre from a row's KH1 top-left: KH1's look (`hud = kh1`), Re:CoM's look.
    pub fn at(kh1_look: bool) -> (f32, f32) {
        if kh1_look {
            (crate::kh1_hud::menu::SEL_AT.0 - GAP - SIZE.0 / 2.0, 30.0)
        } else {
            (-GAP - SIZE.0 / 2.0, 12.0)
        }
    }
    /// The swing-in after a cursor move: (frame, x offset in units from the sway's base), linear
    /// between keys. The base is the rest place − 0.5 (the sway runs −1 ↔ +2 about it: integers).
    pub const INTRO: [(i32, i32); 6] = [(0, 6), (3, 7), (13, -4), (30, 3), (51, -2), (LOOP_START, 2)];
    /// The sway's first frame (Re:CoM's animation group loop start).
    pub const LOOP_START: i32 = 67;
    /// One sway period from [`LOOP_START`] (32 frames: the measured 31.5 rounded), keyed where
    /// a sine through −1 … +2 crosses each integer, so the linear segments follow it.
    pub const SWAY: [(i32, i32); 7] = [(0, 2), (6, 1), (10, 0), (16, -1), (22, 0), (26, 1), (32, 2)];
}

/// One shortcut slot to draw: the KH1 message id of its word (the spell at its level, or the
/// blank command's dash) and its brightness (0x73 greyed, 0xE6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotView {
    pub name: u16,
    pub bright: u8,
}

/// KH1's shortcut list (stage 7.12; `docs/recom-m7-command-menu.md` § 7.12). Checked (decomp +
/// files) unless tagged:
/// - slots: KH1's save `+0x844..0x846`, spell 0..6 or 0xFF (`0x140285090` get, `0x1402857C0` set);
///   ours `save + 0x2681..0x2683`, 0 = none, else spell + 1 (`crate::mp::off::SHORTCUTS`);
/// - drawn by `0x14027B870` case 5 at (left margin + 16, 0x11A): `uibin_cut` objects 0 (the
///   frame, `cut_fra_a`, `uitex.bin` picture 0 through the sprite def `0x1427F1460`) and 1 (the
///   title, `cut_shortcut`, picture 1 through `0x1427F1430`; `0x140266580` builds both defs), the
///   button glyphs 0x46 / 0x45 / 0x41 (`0x140279FC0`, the font's button escape, at objects 2..4's
///   places), then the three words (`0x14027A920`, which adds (16, 2)) at the frame's place + (0,
///   20 + 22 k). The objects are drawn as the file stores them: nothing starts their colour keys
///   (the file's 2-frame alpha-in is never played); every alpha is the menu's fade.
pub mod shortcut {
    /// The panel's origin (KH1's 512 × 384 space, before the left margin).
    pub const ORIGIN: (f32, f32) = (16.0, 282.0);
    /// A slot's word from the origin: (16, 22 + 22 k).
    pub const WORD_AT: (f32, f32) = (16.0, 22.0);
    /// Row pitch.
    pub const PITCH: f32 = 22.0;
    /// The button glyphs' places from the origin (`cut_ico_a / b / c`), before the glyph's own
    /// drop (`super::tab::GLYPH_DY`).
    pub const GLYPH_AT: [(f32, f32); 3] = [(91.0, 23.0), (91.0, 45.0), (91.0, 67.0)];

    /// The spell of a slot byte (0 = none, else spell + 1; anything else: none).
    pub fn slot_spell(b: u8) -> Option<u8> {
        (1..=7).contains(&b).then(|| b - 1)
    }

    /// The slot byte of a spell.
    pub fn slot_byte(spell: u8) -> u8 {
        spell + 1
    }

    /// What a shortcut slot holds (M12 step 6, owner 2026-11-03: "I also would like sleights to be
    /// able to be added to the customize menu, for shortcuts"): a spell (KH1's) or one of the 27
    /// sleight abilities (`crate::sleights`).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Slot {
        Spell(u8),
        Sleight(u16),
    }

    /// The sleight of a slot byte: the byte **is** the Re:CoM sleight id (0x1B..=0x33, 0x3A, 0x43,
    /// the sleight abilities'; 0.111.0: the Party sleights' 0x45..=0x56, `crate::party`; all ≥ 8,
    /// so never a spell byte). Any other byte: none. Bytes 8.. were
    /// "none" before step 6 (`slot_spell`), so an older build shows such a slot empty and keeps the
    /// byte unless the player sets that slot. Checked (code).
    pub fn slot_sleight(b: u8) -> Option<u16> {
        let id = b as u16;
        (b >= 8 && (crate::abilities::of_sleight(id).is_some() || crate::party::index(id).is_some() || crate::riku::sleight(id).is_some())).then_some(id)
    }

    /// The slot byte of a sleight ability's Re:CoM id (None: not one of the 27).
    pub fn sleight_byte(id: u16) -> Option<u8> {
        u8::try_from(id).ok().filter(|&b| slot_sleight(b) == Some(id))
    }

    /// What slot byte `b` holds (None: an empty slot).
    pub fn slot_of(b: u8) -> Option<Slot> {
        match slot_spell(b) {
            Some(s) => Some(Slot::Spell(s)),
            None => slot_sleight(b).map(Slot::Sleight),
        }
    }

    /// The slot byte of `s` (0 for a sleight that isn't one of the 27).
    pub fn byte_of(s: Slot) -> u8 {
        match s {
            Slot::Spell(sp) => slot_byte(sp),
            Slot::Sleight(id) => sleight_byte(id).unwrap_or(0),
        }
    }

    /// Whether slot byte `b` is set (a spell or a sleight): KH1's fill-on-learn skips it.
    pub fn is_set(b: u8) -> bool {
        slot_of(b).is_some()
    }

    /// KH1's 0x41 glyph is the confirm button (`0x1402D4A50`: Cross unless the confirm swap is
    /// set, then Circle). Re:CoM's confirm button is its key word `0x14083F3F0` (PS2 order, read by
    /// `0x140346BB0`, byte-swapped by `0x14029B690`): Circle when it is 0x2000 (Re:CoM's pad bit
    /// 0x20). Checked (decomp); bit 0x20 = Circle: Assumption (bit order).
    pub fn confirm_is_circle(key: u16) -> bool {
        key == 0x2000
    }
}

/// A text colour for Re:CoM's `C2DFnt` (`+0x34`, 0xAABBGGRR, 0x80 = ×1): KH1's
/// `0x14027A920` passes (alpha × 128, b, b, b).
pub fn text_colour(bright: u8, alpha: f32) -> u32 {
    let a = (alpha.clamp(0.0, 1.0) * 128.0) as u32;
    let b = bright as u32;
    a << 24 | b << 16 | b << 8 | b
}

/// Our own menu words (message ids above KH1's `Word.bin`), English in every language: the
/// asset step's `command_menu::OUR_WORDS` holds the same.
/// 0.111.0: 903 "Party", page 2's row 2 (the owner's word).
pub const OUR_WORDS: &[(u16, &str)] = &[(crate::gimmick::STAGGER_WORD, crate::gimmick::STAGGER_TEXT), (crate::sleights::ROW_WORD, crate::sleights::ROW_TEXT), (cmd::PARTY_WORD, PARTY_TEXT)];
/// Page 2's row 2.
pub const PARTY_TEXT: &str = "Party";

/// The words (`kh1-command-menu.json`) of one language, as Re:CoM's font bytes (NUL-terminated),
/// by message id.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Words {
    pub lang: String,
    pub by_id: Vec<(u16, Vec<u8>)>,
    /// Characters Re:CoM's font bytes don't carry, written as their base letters.
    pub folded: Vec<char>,
}

impl Words {
    pub fn get(&self, id: u16) -> Option<&[u8]> {
        self.by_id.iter().find(|x| x.0 == id).map(|x| x.1.as_slice())
    }
}

/// Text as Re:CoM's `C2DFnt` bytes: ASCII as it is (as our Config rows, seen in game); Re:CoM's
/// codes for accented letters are not known (its text bytes ≥ 0x80 start a 2-byte code,
/// `0x140258EB0`): they are written as their base letters. Returns the bytes and the folded
/// characters.
pub fn font_bytes(s: &str) -> (Vec<u8>, Vec<char>) {
    let mut b = Vec::with_capacity(s.len() + 1);
    let mut folded = Vec::new();
    for c in s.chars() {
        if c.is_ascii() && !c.is_ascii_control() {
            b.push(c as u8);
            continue;
        }
        // German ß as "ss" (M9 stage 9c.1: "Sonicstoß", "Todesstoß").
        if c == 'ß' {
            folded.push(c);
            b.extend_from_slice(b"ss");
            continue;
        }
        let base = match c {
            'á' | 'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'Ö' => 'O',
            'Ä' => 'A',
            'É' | 'È' => 'E',
            'Ü' => 'U',
            'ñ' => 'n',
            'ç' => 'c',
            _ => '?',
        };
        folded.push(c);
        b.push(base as u8);
    }
    b.push(0);
    (b, folded)
}

/// Reads `kh1-command-menu.json` for `lang` (us, uk, fr, gr, it, sp); ids missing there come
/// from `us`.
pub fn parse_words(text: &str, lang: &str) -> Result<Words, String> {
    let v = json::parse(text)?;
    if v.get("format").and_then(|f| f.as_str()) != Some(FORMAT) {
        return Err(format!("not a \"{FORMAT}\" manifest"));
    }
    let words = v.get("words").ok_or("no \"words\"")?;
    let us = words.get("us").ok_or("no \"us\" words")?;
    let want = words.get(lang).unwrap_or(us);
    let mut out = Words { lang: if words.get(lang).is_some() { lang.to_string() } else { "us".into() }, ..Default::default() };
    let json::Value::Obj(base) = us else { return Err("bad \"us\" words".into()) };
    for (k, _) in base {
        let Ok(id) = k.parse::<u16>() else { continue };
        let Some(t) = want.get(k).or_else(|| us.get(k)).and_then(|x| x.as_str()) else { continue };
        let (b, f) = font_bytes(t);
        out.folded.extend(f);
        out.by_id.push((id, b));
    }
    // M8b: the summons' names from a manifest written before them (re-run the asset step for
    // KH1's own words): English, Re:CoM's font bytes. Cloud is English in every language anyway.
    for s in crate::summons::ALL.iter() {
        if out.get(s.word).is_none() {
            out.by_id.push((s.word, font_bytes(s.name).0));
        }
    }
    // M12 step 7: Strike Raid's words from a manifest written before them: KH1's English ones
    // (re-run the asset step for each language's).
    for (id, text) in crate::specials::ENGLISH_WORDS {
        if out.get(*id).is_none() {
            out.by_id.push((*id, font_bytes(text).0));
        }
    }
    if [1u16, 2, 3].iter().any(|id| out.get(*id).is_none()) {
        return Err("Attack / Magic / Items missing".into());
    }
    // Our own words (not KH1's): the asset step writes them; a manifest from before them gets the
    // built-in text (the Re:CoM look draws it; the KH1 look needs the asset step's glyphs).
    for (id, text) in OUR_WORDS.iter().copied().chain(crate::sleights::words()).chain(crate::party::words()).chain(crate::riku::words()) {
        if out.get(id).is_none() {
            out.by_id.push((id, font_bytes(text).0));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> World {
        World { state: 0, mp: Some((3, 3, 0)), gate_any: true, gate_fire: true, ..Default::default() }
    }

    fn all_spells() -> [u8; 7] {
        [3; 7]
    }

    fn press(f: impl FnOnce(&mut Pad)) -> Pad {
        let mut p = Pad::default();
        f(&mut p);
        p
    }

    #[test]
    fn root_without_spells_greys_magic_as_kh1() {
        let mut m = Menu::new();
        m.build(&world());
        // Row 1 stays KH1's blank command (message 305), greyed and locked; Items greyed and
        // locked; no context.
        assert_eq!(m.root.cmds, vec![cmd::ATTACK, cmd::BLANK, cmd::ITEMS, cmd::NONE]);
        assert_eq!(m.root.grey & 0xF, 0b1110);
        assert_eq!(m.root.lock & 0xF, 0b1110);
        // Confirm on it: the error sound, no list.
        m.root.cursor = 1;
        let o = m.step(&world(), press(|p| p.confirm = true));
        assert!(!m.open && o.sounds == vec![Sound::Error] && !o.attack);
    }

    #[test]
    fn spells_by_level_and_greyed_when_mp_cant_pay() {
        let mut w = World { spells: all_spells(), mp: Some((0, 3, 29)), ..world() };
        let mut m = Menu::new();
        m.build(&w);
        assert_eq!(m.root.cmds[1], cmd::MAGIC);
        assert_eq!(m.magic.cmds, vec![9, 12, 15, 18, 21, 24, 27]);
        // 0 MP and 29 charge: nothing is payable; the Magic row greys but stays openable.
        assert_eq!(m.magic.grey & 0x7F, 0x7F);
        assert!(m.root.grey & 2 != 0 && m.root.lock & 2 == 0);
        // 30 charge: Fire and Blizzard (30 units) can pay.
        w.mp = Some((0, 3, 30));
        m.build(&w);
        assert_eq!(m.magic.grey & 0x7F, 0x7F & !0b11);
        assert!(m.root.grey & 2 == 0);
        // 2 MP: all but Stop and Aero (2 points need MP ≥ 2: yes) → all.
        w.mp = Some((2, 3, 0));
        m.build(&w);
        assert_eq!(m.magic.grey & 0x7F, 0);
        // Thunder, Gravity, Stop beyond their range (1100) grey.
        w.target_dist = Some(1150.0);
        m.build(&w);
        assert_eq!(m.magic.grey & 0x7F, 0b0110100);
        // In a state without flag 0x20 (glide 0x15): none.
        w.state = 0x15;
        m.build(&w);
        assert_eq!(m.magic.grey & 0x7F, 0x7F);
        assert!(m.root.grey & 1 != 0, "no Attack in the glide (flags 5)");
        // Names: Firaga, Thundaga; level 1 Fire.
        assert_eq!((cmd::name(9), cmd::name(15)), (9, 15));
        let mut one = [0; 7];
        one[0] = 1;
        m.build(&World { spells: one, ..world() });
        assert_eq!(m.magic.cmds, vec![7]);
        assert_eq!(cmd::spell_of(27), Some((6, 3)));
    }

    #[test]
    fn kh1_cursor_rules() {
        let w = World { spells: all_spells(), ..world() };
        let mut m = Menu::new();
        // Up from Attack wraps to row 3 (FM's Summon row); Down back to Attack.
        let o = m.step(&w, press(|p| p.up = true));
        assert_eq!((m.root.cursor, o.sounds.clone()), (3, vec![Sound::Move]));
        assert!(!o.attack);
        m.step(&w, press(|p| p.down = true));
        assert_eq!(m.root.cursor, 0);
        // Two presses in one frame: nothing (the word is compared for equality).
        m.step(&w, press(|p| {
            p.down = true;
            p.up = true
        }));
        assert_eq!(m.root.cursor, 0);
        // Down to Magic, Right opens it (sound "open"), Down moves in it, Left closes it with
        // the root cursor kept on Magic.
        m.step(&w, press(|p| p.down = true));
        let o = m.step(&w, press(|p| p.right = true));
        assert!(m.open && o.sounds == vec![Sound::Open]);
        m.step(&w, press(|p| p.down = true));
        assert_eq!(m.magic.cursor, 1);
        // Up from the first spell wraps to the last.
        m.step(&w, press(|p| p.up = true));
        m.step(&w, press(|p| p.up = true));
        assert_eq!(m.magic.cursor, 6);
        let o = m.step(&w, press(|p| p.left = true));
        assert!(!m.open && o.sounds == vec![Sound::Back] && m.root.cursor == 1);
        // Confirm opens it too; cancel closes it and puts the root cursor on Attack, and is no
        // jump.
        m.step(&w, press(|p| p.confirm = true));
        assert!(m.open);
        let o = m.step(&w, press(|p| p.cancel = true));
        assert!(!m.open && m.root.cursor == 0 && !o.jump);
        // Left at the root (0.111.0, owner: KH1's Left → Attack dropped): the page switches, the
        // cursor stays, Re:CoM's cursor sound.
        m.root.cursor = 2;
        let o = m.step(&w, press(|p| p.left = true));
        assert!(m.root.cursor == 2 && o.sounds == vec![Sound::Page]);
        assert_eq!(o.page, Some(Page::Two));
        m.page = Page::One;
        // The magic cursor is kept when the list opens again (KH1 keeps its list's cursor).
        m.magic.cursor = 4;
        m.root.cursor = 1;
        m.step(&w, press(|p| p.right = true));
        assert_eq!(m.magic.cursor, 4);
    }

    #[test]
    fn confirm_attacks_only_from_the_attack_row() {
        let w = World { spells: all_spells(), ..world() };
        let mut m = Menu::new();
        let o = m.step(&w, press(|p| p.confirm = true));
        assert!(o.attack && o.sounds.is_empty() && o.jump);
        // On Magic: no attack, the list opens.
        m.root.cursor = 1;
        let o = m.step(&w, press(|p| p.confirm = true));
        assert!(!o.attack && m.open);
        // In the list, while KH1's gate refuses (Sora busy): nothing, no sound, the list stays.
        let busy = World { gate_any: false, gate_fire: false, ..w };
        let o = m.step(&busy, press(|p| p.confirm = true));
        assert_eq!(o.events, vec![Event::NotNow(9)]);
        assert!(o.sounds.is_empty() && o.cast.is_none() && m.open);
        // Fire alone passes the release-motion gate.
        let fire_only = World { gate_any: false, gate_fire: true, ..w };
        m.magic.cursor = 1;
        let o = m.step(&fire_only, press(|p| p.confirm = true));
        assert_eq!(o.events, vec![Event::NotNow(12)], "Blizzard isn't taken from Fire's release");
        m.magic.cursor = 0;
        // A spell is issued (7.4): no attack, no jump, no sound; the stack unwinds to Attack.
        let o = m.step(&fire_only, press(|p| p.confirm = true));
        assert_eq!(o.events, vec![Event::Cast(9)]);
        assert_eq!(o.cast, Some(9));
        assert!(!o.attack && !o.jump && o.sounds.is_empty() && !m.open && m.root.cursor == 0);
        // A greyed spell: buzz, flagged locked, nothing issued.
        m.root.cursor = 1;
        m.step(&w, press(|p| p.confirm = true));
        assert!(m.open);
        let mut w2 = w;
        w2.mp = Some((0, 3, 0));
        let o = m.step(&w2, press(|p| p.confirm = true));
        assert_eq!(o.events, vec![Event::Spell(9, true)]);
        assert!(o.cast.is_none() && o.sounds == vec![Sound::Error] && m.open);
        // On Items (locked): the error sound.
        let mut m = Menu::new();
        m.root.cursor = 2;
        let o = m.step(&w, press(|p| p.confirm = true));
        assert!(!o.attack && o.sounds == vec![Sound::Error] && !m.open);
        // Confirm on row 3: the error sound (only Summon opens from there).
        m.root.cursor = 3;
        let o = m.step(&w, press(|p| p.confirm = true));
        assert!(!o.attack && o.sounds == vec![Sound::Error]);
    }

    fn with_items(slots: &[u8]) -> World {
        let mut w = world();
        w.slot_count = 3;
        for (k, &i) in slots.iter().enumerate() {
            w.slots[k] = i;
        }
        w
    }

    #[test]
    fn items_list_holds_kh1s_filled_slots() {
        use crate::items::id;
        // Slots Potion, -, Ether: the list is the two filled ones in slot order, row 2 open.
        let w = with_items(&[id::POTION, 0, id::ETHER]);
        let mut m = Menu::new();
        m.build(&w);
        assert_eq!(m.root.cmds[2], cmd::ITEMS);
        assert_eq!(m.root.grey & 4, 0);
        assert_eq!(m.root.lock & 4, 0);
        assert_eq!(m.items.cmds, vec![0x1F, 0x21]);
        assert_eq!(m.item_slots, vec![0, 2]);
        assert_eq!(m.items.grey & 3, 0);
        // No filled slot: greyed and locked, confirm buzzes.
        let mut e = Menu::new();
        e.build(&with_items(&[]));
        assert_eq!((e.root.grey & 4, e.root.lock & 4), (4, 4));
        // In the air (state 2, flags 0x3D: no 0x40): row 2 greyed but openable; every entry
        // greyed and locked.
        let air = World { state: 2, ..w };
        m.build(&air);
        assert_eq!((m.root.grey & 4, m.root.lock & 4), (4, 0));
        assert_eq!(m.items.grey & 3, 3);
        assert_eq!(m.items.lock & 3, 3);
        // KH1 greys no entry for full HP: the menu knows nothing of HP.
    }

    #[test]
    fn items_list_opens_and_uses_on_sora() {
        use crate::items::{id, ItemCommand};
        let w = with_items(&[id::POTION, 0, id::ETHER]);
        let mut m = Menu::new();
        m.root.cursor = 2;
        // Right (or confirm) opens it with KH1's open sound.
        let o = m.step(&w, press(|p| p.right = true));
        assert!(m.open && m.sub == Kind::Items && o.sounds == vec![Sound::Open]);
        assert_eq!(o.events, vec![Event::Opened(Kind::Items)]);
        // Down moves to the Ether (slot 2); KH1's gate refuses while Sora is busy: no sound.
        m.step(&w, press(|p| p.down = true));
        assert_eq!(m.items.cursor, 1);
        let busy = World { gate_any: false, gate_fire: true, ..w };
        let o = m.step(&busy, press(|p| p.confirm = true));
        assert_eq!(o.events, vec![Event::Item(0x21, 2, ShortResult::NotNow)]);
        assert!(o.sounds.is_empty() && o.item.is_none() && m.open);
        // Taken: the Ether in slot 2, no sound, the stack back on Attack.
        let o = m.step(&w, press(|p| p.confirm = true));
        assert_eq!(o.item, Some(ItemCommand { item: id::ETHER, slot: 2 }));
        assert_eq!(o.events, vec![Event::Item(0x21, 2, ShortResult::Cast)]);
        assert!(o.sounds.is_empty() && !o.attack && !o.jump && !m.open && m.root.cursor == 0);
        // The list's place: one entry at the Items row (y 304), two from 348 − 44.
        m.root.cursor = 2;
        m.step(&w, press(|p| p.confirm = true));
        let sc = m.scene();
        let ys: Vec<f32> = sc.rows.iter().filter(|r| r.list == Kind::Items).map(|r| r.y).collect();
        assert_eq!(ys, vec![304.0, 326.0]);
        assert!(sc.rows.iter().all(|r| r.list != Kind::Items || r.x == ROOT_X + SUB_DX));
        let one = with_items(&[id::POTION]);
        let mut m1 = Menu::new();
        m1.root.cursor = 2;
        m1.step(&one, press(|p| p.confirm = true));
        assert_eq!(m1.scene().rows.iter().filter(|r| r.list == Kind::Items).map(|r| r.y).collect::<Vec<_>>(), vec![304.0]);
        // A greyed entry (in the air): the error sound, nothing issued.
        let air = World { state: 2, ..w };
        let o = m.step(&air, press(|p| p.confirm = true));
        assert_eq!(o.sounds, vec![Sound::Error]);
        assert!(o.item.is_none());
        // The last item used: the list closes (its row can no longer open it).
        let none = with_items(&[]);
        m.step(&none, Pad::default());
        assert!(!m.open);
        // Trace: `menu i:<cursor> ig<grey>` while open.
        m.root.cursor = 2;
        m.step(&w, press(|p| p.confirm = true));
        // (KH1 keeps the list's cursor: still on the second entry.)
        assert!(m.trace_token(&[]).starts_with("menu i:1 g"), "{}", m.trace_token(&[]));
    }

    #[test]
    fn jump_and_square_skip_the_cursor() {
        let w = World { spells: all_spells(), ..world() };
        let mut m = Menu::new();
        // A jump at the root: no cursor move that frame.
        let o = m.step(&w, press(|p| {
            p.cancel = true;
            p.down = true
        }));
        assert!(o.jump && m.root.cursor == 0);
        // Square: the roll; no cursor.
        let o = m.step(&w, press(|p| {
            p.square = true;
            p.down = true
        }));
        assert!(m.root.cursor == 0 && o.sounds.is_empty());
    }

    #[test]
    fn the_magic_list_closes_when_its_row_locks() {
        let mut w = World { spells: all_spells(), ..world() };
        let mut m = Menu::new();
        m.root.cursor = 1;
        m.step(&w, press(|p| p.right = true));
        assert!(m.open);
        w.spells = [0; 7];
        m.step(&w, Pad::default());
        assert!(!m.open);
    }

    #[test]
    fn counterattack_replaces_attack() {
        let mut m = Menu::new();
        m.build(&World { counter: true, ..world() });
        assert_eq!(m.root.cmds[0], cmd::COUNTER);
        assert_eq!(cmd::name(cmd::COUNTER), 343);
        assert!(m.trace_token(&[]).contains("counter"));
    }

    #[test]
    fn brightness_eases_by_kh1s_steps_and_resets_on_a_new_top_list() {
        let w = World { spells: all_spells(), ..world() };
        let mut m = Menu::new();
        m.build(&w);
        m.tick(1.0);
        // First draw: set at once (Attack and Magic bright, Items grey, row 3 grey).
        assert_eq!(m.root.bright, vec![BRIGHT_ON, BRIGHT_ON, BRIGHT_GREY, BRIGHT_GREY]);
        // MP runs out: Magic greys over 4 frames (230 → 198 → 166 → 134 → 115).
        let dry = World { mp: Some((0, 3, 0)), ..w };
        let mut seen = Vec::new();
        for _ in 0..5 {
            m.build(&dry);
            m.tick(1.0);
            seen.push(m.root.bright[1]);
        }
        assert_eq!(seen, vec![198.0, 166.0, 134.0, 115.0, 115.0]);
        // Opening Magic: its rows at once, the root's all grey.
        m.build(&w);
        m.root.cursor = 1;
        m.step(&w, press(|p| p.right = true));
        m.tick(1.0);
        assert!(m.root.bright.iter().all(|&b| b == BRIGHT_GREY));
        assert!(m.magic.bright.iter().all(|&b| b == BRIGHT_ON));
    }

    #[test]
    fn fade_in_takes_ten_frames() {
        let mut m = Menu::new();
        m.fade_in();
        for _ in 0..9 {
            m.tick(1.0);
        }
        assert!(m.alpha < 1.0 && m.alpha > 0.85);
        m.tick(1.0);
        assert!((m.alpha - 1.0).abs() < 1e-5);
        m.hide();
        assert_eq!(m.alpha, 0.0);
        m.show();
        assert_eq!(m.alpha, 1.0);
        assert_eq!(text_colour(0xE6, 1.0), 0x80E6_E6E6);
        assert_eq!(text_colour(0x73, 0.5), 0x4073_7373);
    }

    #[test]
    fn context_tab_slides_in_over_14_frames_and_gates_triangle() {
        let w = World { context: cmd::OPEN, ..world() };
        let mut m = Menu::new();
        m.show();
        m.build(&w);
        let mut frames = 0;
        while m.tab.state != 2 {
            // Triangle before the tab is in: Re:CoM's action waits.
            let o = m.step(&w, press(|p| p.triangle = true));
            assert!(!o.triangle);
            m.tick(1.0);
            frames += 1;
            assert!(frames < 30);
        }
        // The frame it appears starts the tab, then 14 frames of slide (half-frame steps of 10/7).
        assert_eq!(frames, 15);
        assert_eq!((m.tab.x, m.tab.alpha, m.tab_cmd), (0.0, 1.0, cmd::OPEN));
        let o = m.step(&w, press(|p| p.triangle = true));
        assert!(o.triangle && o.events == vec![Event::Context(cmd::OPEN)]);
        // No context command: Re:CoM's own Triangle action decides (nothing to show).
        let mut plain = Menu::new();
        plain.show();
        assert!(plain.step(&world(), press(|p| p.triangle = true)).triangle);
        // In the Magic list Triangle does nothing.
        let ws = World { spells: all_spells(), ..w };
        m.root.cursor = 1;
        m.step(&ws, press(|p| p.right = true));
        assert!(!m.step(&ws, press(|p| p.triangle = true)).triangle);
        // The object goes: the tab fades out by its opacity (14 frames), keeping its name.
        let gone = World { context: 0, ..world() };
        m.step(&gone, Pad::default());
        m.tick(1.0);
        assert_eq!(m.tab.state, 3);
        let mut n = 1;
        while m.tab.state != 0 {
            m.step(&gone, Pad::default());
            m.tick(1.0);
            n += 1;
        }
        assert_eq!(n, 15, "the frame it goes, then 14 frames of fading");
        // Hidden HUD: reset.
        m.build(&w);
        m.hide();
        m.tick(1.0);
        assert_eq!(m.tab, Tab::default());
    }

    /// M9 stage 9c.1: Sonic Blade in the context entry (KH1's offer, after any object), its tab
    /// gating Triangle as every context command; Triangle runs it as ours, not Re:CoM's action; a
    /// greyed offer shows no tab (FM); an object takes the entry first.
    #[test]
    fn the_special_offer_takes_the_context_tab() {
        use crate::specials::cmd as sp;
        let w = World { special: sp::SONIC_BLADE, ..world() };
        let mut m = Menu::new();
        m.show();
        let mut frames = 0;
        while m.tab.state != 2 {
            let o = m.step(&w, press(|p| p.triangle = true));
            assert!(o.special.is_none());
            m.tick(1.0);
            frames += 1;
            assert!(frames < 30);
        }
        assert_eq!((frames, m.tab_cmd, cmd::name(m.tab_cmd)), (15, sp::SONIC_BLADE, 319));
        let o = m.step(&w, press(|p| p.triangle = true));
        assert_eq!((o.special, o.triangle), (Some(sp::SONIC_BLADE), false));
        // MP short: greyed, no tab, Triangle runs nothing.
        let grey = World { special_grey: true, ..w };
        let mut g = Menu::new();
        g.show();
        for _ in 0..20 {
            let o = g.step(&grey, press(|p| p.triangle = true));
            assert!(o.special.is_none());
            g.tick(1.0);
        }
        assert_eq!(g.tab.state, 0);
        assert!(g.root.grey & 8 != 0 && g.root.cmds[3] == sp::SONIC_BLADE);
        // An object's command wins the entry.
        let mut o2 = Menu::new();
        o2.build(&World { context: cmd::OPEN, ..w });
        assert_eq!(o2.root.cmds[3], cmd::OPEN);
    }

    /// A held gimmick card: "Stagger" in the context tab (before the special offer, after an
    /// object's command), gated by the tab as every context command; Triangle plays it as ours
    /// (not Re:CoM's Triangle action); used, the tab fades.
    #[test]
    fn a_held_gimmick_card_offers_stagger() {
        use crate::gimmick::{STAGGER, STAGGER_WORD};
        use crate::specials::cmd as sp;
        let w = World { gimmick: STAGGER, special: sp::SONIC_BLADE, ..world() };
        let mut m = Menu::new();
        m.show();
        let mut frames = 0;
        while m.tab.state != 2 {
            let o = m.step(&w, press(|p| p.triangle = true));
            assert!(!o.gimmick && o.special.is_none());
            m.tick(1.0);
            frames += 1;
            assert!(frames < 30);
        }
        assert_eq!((frames, m.tab_cmd, cmd::name(m.tab_cmd)), (15, STAGGER, STAGGER_WORD));
        assert!(m.root.grey & 8 == 0);
        let o = m.step(&w, press(|p| p.triangle = true));
        assert!(o.gimmick && !o.triangle && o.special.is_none());
        assert_eq!(o.events, vec![Event::Context(STAGGER)]);
        // Confirm on row 3 never plays it (the cursor never runs the context command).
        m.root.cursor = 3;
        let o = m.step(&w, press(|p| p.confirm = true));
        assert!(!o.gimmick && o.sounds == vec![Sound::Error]);
        // Played (no card held any more): the tab fades out.
        let gone = World { gimmick: 0, ..world() };
        m.step(&gone, Pad::default());
        m.tick(1.0);
        assert_eq!(m.tab.state, 3);
        // An object's command takes the entry first; a greyed special doesn't grey Stagger.
        let mut o2 = Menu::new();
        o2.build(&World { context: cmd::OPEN, ..w });
        assert_eq!(o2.root.cmds[3], cmd::OPEN);
        let mut o3 = Menu::new();
        o3.build(&World { special_grey: true, ..w });
        assert!(o3.root.cmds[3] == STAGGER && o3.root.grey & 8 == 0);
    }

    #[test]
    fn our_words_come_without_the_asset_step() {
        let old = r#"{"format": "kh1-recom-assets command-menu 1", "words": {"us": {"1": "Attack", "2": "Magic", "3": "Items"}}}"#;
        let w = parse_words(old, "fr").unwrap();
        assert_eq!(w.get(crate::gimmick::STAGGER_WORD), Some(&b"Stagger\0"[..]));
        let new = r#"{"format": "kh1-recom-assets command-menu 1", "words": {"us": {"1": "Attack", "2": "Magic", "3": "Items", "901": "Stagger"}}}"#;
        let w = parse_words(new, "us").unwrap();
        assert_eq!(w.by_id.iter().filter(|x| x.0 == crate::gimmick::STAGGER_WORD).count(), 1);
    }

    /// M9 stage 9c.1: Rave / Blast in the Attack row during the special (`0x14039ED70`): shown in
    /// the tab when ungreyed, run by Triangle; confirm on it is the error sound (no Attack); a
    /// refused confirm (a submenu opened) is reported for the attacking bit.
    #[test]
    fn sonic_blade_follow_ups_take_row_0() {
        use crate::specials::cmd as sp;
        let w = World { state: 0x20, follow: sp::RAVE, spells: all_spells(), ..world() };
        let mut m = Menu::new();
        m.show();
        for _ in 0..16 {
            m.step(&w, Pad::default());
            m.tick(1.0);
        }
        assert_eq!((m.root.cmds[0], m.root.grey & 1), (sp::RAVE, 0));
        assert_eq!(m.tab.state, 2);
        let o = m.step(&w, press(|p| p.confirm = true));
        assert!(!o.attack && o.sounds == vec![Sound::Error] && !o.confirm_refused, "X on Rave: the error sound only");
        let o = m.step(&w, press(|p| p.triangle = true));
        assert_eq!(o.special, Some(sp::RAVE));
        // Greyed (outside the dash's window): no tab, no run.
        let gw = World { follow_grey: true, ..w };
        for _ in 0..16 {
            m.step(&gw, Pad::default());
            m.tick(1.0);
        }
        assert!(m.root.grey & 1 != 0 && m.tab.state == 0);
        assert!(m.step(&gw, press(|p| p.triangle = true)).special.is_none());
        // Confirm on Magic opens the list: KH1's commit returned 0 (the bit goes).
        m.root.cursor = 1;
        let o = m.step(&gw, press(|p| p.confirm = true));
        assert!(m.open && o.confirm_refused);
        assert_eq!(cmd::name(sp::BLAST), 322);
    }

    #[test]
    fn scene_positions_are_kh1s() {
        let w = World { spells: all_spells(), context: cmd::TALK, ..world() };
        let mut m = Menu::new();
        m.show();
        m.step(&w, Pad::default());
        m.tick(1.0);
        m.step(&w, Pad::default());
        m.tick(1.0);
        let s = m.scene();
        // FM: four rows always (0.111.0: the 4th is Sleights on both pages); the context command is
        // not a row.
        assert_eq!(s.rows.iter().map(|r| (r.x, r.y)).collect::<Vec<_>>(), vec![(16.0, 260.0), (16.0, 282.0), (16.0, 304.0), (16.0, 326.0)]);
        assert!(s.rows[0].highlight && s.rows[3].name == crate::sleights::ROW_WORD && !s.rows[3].highlight);
        assert!(s.rows.iter().all(|r| r.name != cmd::name(cmd::TALK)));
        // The tab: Talk, sliding in (the frame it starts, then x −10 + 10/7 × ½, opacity 1/14).
        let t = s.tab.expect("the tab shows");
        assert_eq!(t.name, cmd::name(cmd::TALK));
        assert!(t.x < 0.0 && t.x > -10.0 && t.alpha > 0.0 && t.alpha < 0.1 && !t.dim);
        // Magic with 7 spells: bottom-anchored at 348 − 22 × 7 = 194, x + 11; the tab dims.
        m.root.cursor = 1;
        m.step(&w, press(|p| p.right = true));
        m.tick(1.0);
        let s = m.scene();
        let mg: Vec<_> = s.rows.iter().filter(|r| r.list == Kind::Magic).collect();
        assert_eq!((mg[0].x, mg[0].y, mg.len()), (27.0, 194.0, 7));
        assert!(mg[0].highlight && !s.rows[1].highlight, "only the top list's cursor row");
        let t = s.tab.unwrap();
        assert!(t.dim && t.bright == (BRIGHT_GREY * 0.5) as u8, "the root's rows go grey under the list, the tab × 0.5");
        // Two spells: at the Magic row's y.
        let mut two = [0; 7];
        two[0] = 2;
        two[3] = 1;
        m.step(&World { spells: two, ..w }, Pad::default());
        let s = m.scene();
        let mg: Vec<_> = s.rows.iter().filter(|r| r.list == Kind::Magic).collect();
        assert_eq!((mg[0].y, mg[1].y, mg[0].name, mg[1].name), (282.0, 304.0, 8, 16));
        // No context: no tab; the 4th row stays, grey by its grey bit (nothing equipped), under the
        // cursor too.
        let mut m = Menu::new();
        m.show();
        m.step(&world(), Pad::default());
        m.tick(1.0);
        let s = m.scene();
        assert_eq!((s.rows.len(), s.tab), (4, None));
        assert_eq!(s.rows[3].bright, BRIGHT_GREY as u8);
        m.root.cursor = 3;
        let s = m.scene();
        assert_eq!((s.rows[3].name, s.rows[3].highlight, s.rows[3].bright), (crate::sleights::ROW_WORD, true, BRIGHT_GREY as u8));
    }

    #[test]
    fn the_cursor_never_runs_the_context_command() {
        let w = World { spells: all_spells(), context: cmd::OPEN, ..world() };
        let mut m = Menu::new();
        m.show();
        for _ in 0..16 {
            m.step(&w, Pad::default());
            m.tick(1.0);
        }
        assert_eq!(m.tab.state, 2);
        // Up to row 3 (Summon's row) and confirm or Right: the error sound, no context.
        m.step(&w, press(|p| p.up = true));
        assert_eq!(m.root.cursor, 3);
        for pad in [press(|p| p.confirm = true), press(|p| p.right = true)] {
            let o = m.step(&w, pad);
            assert_eq!((o.sounds.clone(), o.events.clone()), (vec![Sound::Error], vec![Event::Refused(cmd::BLANK)]));
            assert!(!o.attack);
        }
        // Triangle runs it, from any root row.
        let o = m.step(&w, press(|p| p.triangle = true));
        assert_eq!(o.events, vec![Event::Context(cmd::OPEN)]);
        // KH1's triangle offsets by language (English −3, French / German / Italian −9, Spanish −18).
        assert_eq!([tab::triangle_x("us"), tab::triangle_x("fr"), tab::triangle_x("sp")], [-3.0, -9.0, -18.0]);
    }

    /// M8b: Simba (5), Dumbo (0), Cloud (6) learned in that order, in battle, 3 MP.
    fn summon_world() -> World {
        let mut summons = [0u8; crate::summons::COUNT];
        summons[..3].copy_from_slice(&[5, 0, 6]);
        World { spells: all_spells(), summons, summon_count: 3, battle: true, ..world() }
    }

    #[test]
    fn summon_is_page_2s_row_1() {
        // Page 1: Magic on row 1, no Summon entry anywhere.
        let mut m = Menu::new();
        m.build(&summon_world());
        assert!(!m.summon_on && m.root.cmds[1] == cmd::MAGIC);
        // Page 2: row 1 is Summon, not greyed (one usable), the list in learn order; no spell is
        // needed any more (it is not the Magic list's entry).
        let w = World { page: Page::Two, ..summon_world() };
        m.build(&w);
        assert!(m.summon_on && !m.summon_grey && !m.summon_lock && m.root.cmds[1] == cmd::SUMMON);
        m.build(&World { spells: [0; 7], ..w });
        assert!(m.summon_on, "no spell needed on page 2");
        m.build(&w);
        // Cloud first (owner), then Simba, Dumbo in learn order.
        assert_eq!(m.summon.cmds, vec![0x35, 0x34, 0x2F]);
        assert_eq!(m.summon.grey & 7, 0);
        // The context command keeps entry 3 (the tab's); row 3's word is Sleights, row 1's Summon.
        assert_eq!(m.root.cmds[3], cmd::NONE);
        m.show();
        let sc = m.scene();
        assert_eq!(sc.rows[1].name, SUMMON_NAME);
        assert_eq!(sc.rows[3].name, crate::sleights::ROW_WORD);
        // Down to row 1, confirm: the Summon list opens (sound 6); Right does the same.
        for pad in [press(|p| p.confirm = true), press(|p| p.right = true)] {
            let mut m = Menu::new();
            m.step(&w, press(|p| p.down = true));
            let o = m.step(&w, pad);
            assert!(m.open && m.sub == Kind::Summon, "{pad:?}");
            assert_eq!((o.sounds, o.events), (vec![Sound::Open], vec![Event::Opened(Kind::Summon)]));
            assert!(!o.attack);
        }
        // None learned: Summon greyed and locked (the error sound).
        let w0 = World { summon_count: 0, ..w };
        let mut m = Menu::new();
        m.step(&w0, press(|p| p.down = true));
        assert!(!m.summon_on && m.summon_grey && m.summon_lock && m.root.cmds[1] == cmd::SUMMON);
        let o = m.step(&w0, press(|p| p.confirm = true));
        assert!(!m.open && o.sounds == vec![Sound::Error]);
    }

    #[test]
    fn summon_greying_is_kh1s() {
        let p2 = |w: World| World { page: Page::Two, ..w };
        let mut m = Menu::new();
        // 2 MP: Cloud (3, first) not usable, Simba (2) and Dumbo (2) usable.
        m.build(&p2(World { mp: Some((2, 3, 0)), ..summon_world() }));
        assert_eq!(m.summon.grey & 7, 0b001);
        assert_eq!(m.summon.lock & 7, 0b001);
        // No once-per-battle rule (owner): with the MP, a summon stays usable after it was used;
        // only one out at a time (below).
        // Not in battle (the field): all greyed; the row greyed but openable.
        m.build(&p2(World { battle: false, ..summon_world() }));
        assert_eq!(m.summon.grey & 7, 0b111);
        assert!(m.summon_grey && !m.summon_lock);
        // In the air (KH1 state 2: no flag 0x80): all greyed.
        m.build(&p2(World { state: 2, ..summon_world() }));
        assert_eq!(m.summon.grey & 7, 0b111);
        // One out: the row greyed and locked (owner: no Dismiss); confirm is the error sound.
        let w = p2(World { summon_out: true, ..summon_world() });
        let mut m = Menu::new();
        m.step(&w, press(|p| p.down = true));
        assert!(m.summon_grey && m.summon_lock);
        let o = m.step(&w, press(|p| p.confirm = true));
        assert!(!m.open);
        assert_eq!((o.sounds, o.events), (vec![Sound::Error], vec![Event::Refused(cmd::SUMMON)]));
        m.show();
        m.tick(1.0);
        m.tick(1.0);
        m.tick(1.0);
        m.tick(1.0);
        assert_eq!(m.scene().rows[1].bright, BRIGHT_GREY as u8);
    }

    #[test]
    fn summon_list_issues_through_kh1s_gate() {
        let w = World { page: Page::Two, ..summon_world() };
        let mut m = Menu::new();
        m.step(&w, press(|p| p.down = true));
        m.step(&w, press(|p| p.confirm = true));
        // Down to Simba (Cloud is first); the cursor wraps over the three.
        m.step(&w, press(|p| p.down = true));
        assert_eq!(m.summon.cursor, 1);
        // KH1's commit gate refuses (Sora busy): no sound, the list stays.
        let o = m.step(&World { gate_any: false, ..w }, press(|p| p.confirm = true));
        assert!(m.open && o.sounds.is_empty() && o.summon.is_none());
        assert_eq!(o.events, vec![Event::Summon(0x34, ShortResult::NotNow)]);
        // Issued: no menu sound, the stack back to Attack.
        let o = m.step(&w, press(|p| p.confirm = true));
        assert_eq!(o.summon, Some(5));
        assert!(!m.open && m.root.cursor == 0 && o.sounds.is_empty() && !o.attack);
        // A greyed entry (Cloud with 2 MP): the error sound.
        let w2 = World { mp: Some((2, 3, 0)), ..w };
        m.step(&w2, press(|p| p.down = true));
        m.step(&w2, press(|p| p.confirm = true));
        // The list keeps its cursor (Simba, 1), as KH1's records do: Up to Cloud, Up again wraps.
        assert_eq!(m.summon.cursor, 1);
        m.step(&w2, press(|p| p.up = true));
        assert_eq!(m.summon.cursor, 0, "Cloud");
        let o = m.step(&w2, press(|p| p.confirm = true));
        assert_eq!((o.sounds, o.summon), (vec![Sound::Error], None));
        // Cancel closes it (sound 3), the root back on Attack.
        let o = m.step(&w2, press(|p| p.cancel = true));
        assert!(!m.open && m.root.cursor == 0 && o.sounds == vec![Sound::Back]);
    }

    /// M12 step 5: Fire Raid (2 MP), Holy (3 MP), Idyll Romp (all MP) equipped, in battle, the
    /// summons of [`summon_world`] learned too.
    fn sleight_world() -> World {
        use crate::abilities::sleight as s;
        let mut sleights = [0u8; crate::sleights::MAX];
        sleights[..3].copy_from_slice(&[s::HOLY as u8, s::IDYLL_ROMP as u8, s::FIRE_RAID as u8]);
        World { sleights, sleight_count: 3, ..summon_world() }
    }

    #[test]
    fn left_at_the_root_switches_the_pages() {
        // From any root row, Left switches the page (Re:CoM's cursor sound) and keeps the cursor;
        // the save keeps it (the next frame's world says page 2).
        for row in 0..4 {
            let mut w = summon_world();
            let mut m = Menu::new();
            m.root.cursor = row;
            let o = m.step(&w, press(|p| p.left = true));
            assert_eq!((o.page, o.sounds.clone(), o.events.clone(), m.root.cursor), (Some(Page::Two), vec![Sound::Page], vec![Event::Page(Page::Two)], row));
            assert!(!o.attack, "Left is no Attack");
            w.page = Page::Two;
            m.step(&w, Pad::default());
            m.show();
            let names: Vec<u16> = m.scene().rows.iter().map(|r| r.name).collect();
            assert_eq!(names, [1, SUMMON_NAME, cmd::PARTY_WORD, crate::sleights::ROW_WORD]);
            // And back.
            let o = m.step(&w, press(|p| p.left = true));
            assert_eq!(o.page, Some(Page::One));
            w.page = Page::One;
            m.step(&w, Pad::default());
            let names: Vec<u16> = m.scene().rows.iter().map(|r| r.name).collect();
            assert_eq!(names, [1, 2, 3, crate::sleights::ROW_WORD]);
        }
        // In an open list Left still closes it (KH1), with no switch.
        let w = summon_world();
        let mut m = Menu::new();
        m.root.cursor = 1;
        m.step(&w, press(|p| p.confirm = true));
        assert!(m.open);
        let o = m.step(&w, press(|p| p.left = true));
        assert!(!m.open && o.page.is_none() && o.sounds == vec![Sound::Back]);
        // The shortcut list (L1 held) has no cursor: no switch.
        let mut m = Menu::new();
        let ws = World { shortcuts: [1, 0, 0], ..w };
        let o = m.step(&ws, press(|p| {
            p.l1 = true;
            p.left = true
        }));
        assert!(m.short_on && o.page.is_none());
        // A page switch with the Magic list open on page 1 (another frame's world): it closes.
        let mut m = Menu::new();
        m.root.cursor = 1;
        m.step(&w, press(|p| p.confirm = true));
        assert!(m.open && m.sub == Kind::Magic);
        m.step(&World { page: Page::Two, ..w }, Pad::default());
        assert!(!m.open);
    }

    #[test]
    fn the_page_byte() {
        assert_eq!(Page::of_byte(0), Page::One, "a save from before 0.111.0 (its Summon choice)");
        assert_eq!(Page::of_byte(1), Page::One, "its Sleights choice");
        assert_eq!(Page::of_byte(2), Page::Two);
        assert_eq!(Page::of_byte(7), Page::One);
        assert_eq!(Page::of_byte(Page::Two.byte()), Page::Two);
        assert_eq!(Page::of_byte(Page::One.byte()), Page::One);
        assert_eq!(Page::One.other(), Page::Two);
    }

    #[test]
    fn sleights_list_holds_the_equipped_ones_greyed_by_mp() {
        use crate::abilities::sleight as s;
        let mut m = Menu::new();
        // 3 MP: all three usable, in Re:CoM's id order.
        m.build(&sleight_world());
        assert_eq!(m.sleight.cmds, vec![crate::sleights::command(s::FIRE_RAID), crate::sleights::command(s::HOLY), crate::sleights::command(s::IDYLL_ROMP)]);
        assert_eq!(m.sleight.grey & 7, 0);
        assert!(!m.sleight_grey && !m.sleight_lock);
        // Its names are the sleights' words.
        assert_eq!(cmd::name(m.sleight.cmds[1]), crate::sleights::word(s::HOLY));
        // 2 MP: Holy (3) greyed and locked.
        m.build(&World { mp: Some((2, 3, 0)), ..sleight_world() });
        assert_eq!((m.sleight.grey & 7, m.sleight.lock & 7), (0b010, 0b010));
        // 0 MP (a full charge doesn't count): all greyed, the row greyed but openable.
        m.build(&World { mp: Some((0, 3, 29)), ..sleight_world() });
        assert_eq!(m.sleight.grey & 7, 0b111);
        assert!(m.sleight_grey && !m.sleight_lock);
        // The field (no field case: Combat = Re:CoM or not installed), the air: all greyed.
        m.build(&World { battle: false, ..sleight_world() });
        assert_eq!(m.sleight.grey & 7, 0b111);
        // Owner (2026-11-04, "Sleights outside battle"): a field room where Re:CoM's sleight code
        // runs: all usable with no MP; without the packs (100 Acre Wood, rooms 2000-2999) only Fire
        // Raid (Holy and Idyll Romp need theirs).
        m.build(&World { battle: false, field: true, packs: true, mp: Some((0, 3, 0)), ..sleight_world() });
        assert_eq!(m.sleight.grey & 7, 0);
        m.build(&World { battle: false, field: true, packs: false, mp: Some((0, 3, 0)), ..sleight_world() });
        assert_eq!(m.sleight.grey & 7, 0b110);
        m.build(&World { state: 2, ..sleight_world() });
        assert_eq!(m.sleight.grey & 7, 0b111);
        // A sleight (or a summon) running: greyed and locked; the Summon row too.
        m.build(&World { summon_out: true, ..sleight_world() });
        assert!(m.sleight_grey && m.sleight_lock);
        // Nothing equipped: the row still says Sleights, greyed and locked; confirm buzzes.
        let w = World { sleight_count: 0, ..sleight_world() };
        let mut m = Menu::new();
        m.step(&w, press(|p| p.up = true));
        assert!(m.sleight.cmds.is_empty() && m.sleight_grey && m.sleight_lock);
        let o = m.step(&w, press(|p| p.confirm = true));
        assert!(!m.open && o.sounds == vec![Sound::Error] && o.sleight.is_none());
        m.show();
        let sc = m.scene();
        assert_eq!((sc.rows[3].name, sc.rows[3].bright), (crate::sleights::ROW_WORD, BRIGHT_GREY as u8), "set by its grey bit, as Summon's");
    }

    /// Owner (2026-11-03, "Sleights menu follow-ups"): in the air only Re:CoM's air-start sleights
    /// (`crate::sleights::AIR_START`) are usable; KH1's air states that take a command (take-off 1,
    /// air 2, flight 8), not one that doesn't (a reaction, state 4).
    #[test]
    fn sleights_list_in_the_air_offers_recoms_air_start_ones() {
        use crate::abilities::sleight as s;
        let mut sleights = [0u8; crate::sleights::MAX];
        // Fire Raid (ground only), Warp, Teleport (air start).
        sleights[..3].copy_from_slice(&[s::FIRE_RAID as u8, 0x2C, 0x32]);
        let w = World { sleights, sleight_count: 3, ..sleight_world() };
        let mut m = Menu::new();
        m.build(&w);
        assert_eq!(m.sleight.grey & 7, 0, "on the ground: all three");
        for state in [1, 2, 8] {
            m.build(&World { state, ..w });
            assert_eq!((m.sleight.grey & 7, m.sleight.lock & 7), (0b001, 0b001), "state {state}: Fire Raid greyed, Warp / Teleport usable");
            assert!(!m.sleight_grey && !m.sleight_lock);
        }
        m.build(&World { state: 4, ..w });
        assert_eq!(m.sleight.grey & 7, 0b111, "a KH1 state that takes no command");
        // Issued mid-air through KH1's gate.
        let wa = World { state: 2, ..w };
        let mut m = Menu::new();
        m.step(&wa, press(|p| p.up = true));
        m.step(&wa, press(|p| p.confirm = true));
        assert!(m.open && m.sub == Kind::Sleight);
        m.sleight.cursor = 1;
        let o = m.step(&wa, press(|p| p.confirm = true));
        assert_eq!(o.sleight, Some(0x2C));
        // Fire Raid in the air: the error sound.
        m.step(&wa, press(|p| p.up = true));
        m.step(&wa, press(|p| p.confirm = true));
        m.sleight.cursor = 0;
        let o = m.step(&wa, press(|p| p.confirm = true));
        assert_eq!((o.sounds, o.sleight), (vec![Sound::Error], None));
    }

    #[test]
    fn sleights_list_opens_beside_row_3_and_issues_through_kh1s_gate() {
        use crate::abilities::sleight as s;
        let w = sleight_world();
        let mut m = Menu::new();
        m.step(&w, press(|p| p.up = true));
        // Right (or confirm) on row 3 opens it (sound 6).
        let o = m.step(&w, press(|p| p.right = true));
        assert!(m.open && m.sub == Kind::Sleight);
        assert_eq!((o.sounds, o.events), (vec![Sound::Open], vec![Event::Opened(Kind::Sleight)]));
        // Beside row 3: three entries bottom-anchored at 348 − 66 = 282, x + 11.
        m.show();
        let ys: Vec<(f32, f32, u16)> = m.scene().rows.iter().filter(|r| r.list == Kind::Sleight).map(|r| (r.x, r.y, r.name)).collect();
        assert_eq!(ys, vec![(27.0, 282.0, crate::sleights::word(s::FIRE_RAID)), (27.0, 304.0, crate::sleights::word(s::HOLY)), (27.0, 326.0, crate::sleights::word(s::IDYLL_ROMP))]);
        // KH1's gate refuses (busy): no sound, the list stays.
        m.step(&w, press(|p| p.down = true));
        let o = m.step(&World { gate_any: false, ..w }, press(|p| p.confirm = true));
        assert_eq!(o.events, vec![Event::Sleight(crate::sleights::command(s::HOLY), ShortResult::NotNow)]);
        assert!(o.sounds.is_empty() && o.sleight.is_none() && m.open);
        // Taken: Holy, no sound, the stack back on Attack.
        let o = m.step(&w, press(|p| p.confirm = true));
        assert_eq!((o.sleight, o.sounds.clone()), (Some(s::HOLY), vec![]));
        assert!(!m.open && m.root.cursor == 0 && !o.attack);
        // Greyed (2 MP): the error sound, nothing issued.
        let w2 = World { mp: Some((2, 3, 0)), ..w };
        m.step(&w2, press(|p| p.up = true));
        m.step(&w2, press(|p| p.confirm = true));
        m.sleight.cursor = 1;
        let o = m.step(&w2, press(|p| p.confirm = true));
        assert_eq!((o.sounds, o.sleight), (vec![Sound::Error], None));
        // One entry: beside row 3.
        let mut one = [0u8; crate::sleights::MAX];
        one[0] = s::FREEZE as u8;
        let w1 = World { sleights: one, sleight_count: 1, ..w };
        let mut m = Menu::new();
        m.step(&w1, press(|p| p.up = true));
        m.step(&w1, press(|p| p.confirm = true));
        m.show();
        let ys: Vec<f32> = m.scene().rows.iter().filter(|r| r.list == Kind::Sleight).map(|r| r.y).collect();
        assert_eq!(ys, vec![326.0]);
        // Equipped no more while it is open (another frame's world): it closes.
        m.step(&World { sleight_count: 0, ..w1 }, Pad::default());
        assert!(!m.open);
    }

    /// M12 step 7 (owner, 2026-11-03: "also show up in the sleights menu"): an equipped Strike Raid
    /// is listed under Re:CoM's sleight 8 (first: Re:CoM's order), named "Strike" (KH1's word 338),
    /// usable exactly when its prompt would offer it (the world's bit), on the ground; confirm issues
    /// its KH1 command (0x5E), not Re:CoM's sleight.
    #[test]
    fn sleights_list_holds_strike_raid_as_its_prompt() {
        use crate::abilities::sleight as s;
        use crate::specials::{cmd as sp, STRIKE_RAID};
        let mut sleights = [0u8; crate::sleights::MAX];
        sleights[..2].copy_from_slice(&[s::FIRE_RAID as u8, s::STRIKE_RAID as u8]);
        let w = World { sleights, sleight_count: 2, sleight_specials: 1 << STRIKE_RAID, ..sleight_world() };
        let mut m = Menu::new();
        m.build(&w);
        assert_eq!(m.sleight.cmds, vec![crate::sleights::command(s::STRIKE_RAID), crate::sleights::command(s::FIRE_RAID)]);
        assert_eq!(cmd::name(m.sleight.cmds[0]), 338);
        assert_eq!(m.sleight.grey & 3, 0);
        // Its prompt wouldn't offer it (no target in range, MP short): greyed; Fire Raid unchanged.
        m.build(&World { sleight_specials: 0, ..w });
        assert_eq!(m.sleight.grey & 3, 0b01);
        // In the air: greyed (KH1's specials start from state 0).
        m.build(&World { state: 2, ..w });
        assert_eq!(m.sleight.grey & 1, 1);
        // Confirm: the special's own command, as the prompt's Triangle.
        let mut m = Menu::new();
        m.step(&w, press(|p| p.up = true));
        m.step(&w, press(|p| p.confirm = true));
        assert!(m.open && m.sub == Kind::Sleight);
        let o = m.step(&w, press(|p| p.confirm = true));
        assert_eq!((o.special, o.sleight), (Some(sp::STRIKE_RAID), None));
        assert!(!m.open);
    }

    /// Owner (2026-11-03, "KH1 specials follow-ups": "Sonic Blade also in the Sleights list"): an
    /// equipped Sonic Blade is listed under Re:CoM's sleight 9 (after Strike Raid 8, before Ars
    /// Arcanum 10), named "Sonic" (KH1's word 319), usable exactly when its prompt would offer it, on
    /// the ground; confirm issues its KH1 command 0x4B, as the prompt's Triangle does (the same
    /// `Outcome::special`), not Re:CoM's sleight.
    #[test]
    fn sleights_list_holds_sonic_blade_as_its_prompt() {
        use crate::abilities::sleight as s;
        use crate::specials::{cmd as sp, SONIC_BLADE, STRIKE_RAID};
        let mut sleights = [0u8; crate::sleights::MAX];
        sleights[..3].copy_from_slice(&[s::FIRE_RAID as u8, s::SONIC_BLADE as u8, s::STRIKE_RAID as u8]);
        let w = World { sleights, sleight_count: 3, sleight_specials: 1 << SONIC_BLADE, ..sleight_world() };
        let mut m = Menu::new();
        m.build(&w);
        assert_eq!(m.sleight.cmds, vec![crate::sleights::command(s::STRIKE_RAID), crate::sleights::command(s::SONIC_BLADE), crate::sleights::command(s::FIRE_RAID)]);
        assert_eq!(cmd::name(m.sleight.cmds[1]), 319);
        // Sonic Blade and Fire Raid usable; Strike Raid's prompt wouldn't offer it (its bit clear).
        assert_eq!(m.sleight.grey & 7, 0b001);
        m.build(&World { sleight_specials: 0, ..w });
        assert_eq!(m.sleight.grey & 7, 0b011, "its prompt wouldn't offer it: greyed");
        m.build(&World { state: 2, ..w });
        assert_eq!(m.sleight.grey & 2, 2, "in the air: greyed (KH1's specials start from state 0)");
        // Confirm on its row (the second): KH1's own Sonic Blade command, as the prompt's Triangle.
        let w = World { sleight_specials: 1 << SONIC_BLADE | 1 << STRIKE_RAID, ..w };
        let mut m = Menu::new();
        m.step(&w, press(|p| p.up = true));
        m.step(&w, press(|p| p.confirm = true));
        assert!(m.open && m.sub == Kind::Sleight);
        m.step(&w, press(|p| p.down = true));
        let o = m.step(&w, press(|p| p.confirm = true));
        assert_eq!((o.special, o.sleight), (Some(sp::SONIC_BLADE), None));
        assert!(!m.open);
    }

    #[test]
    fn sleights_words_are_in_the_word_list() {
        // A manifest from before M12 step 5: the built-in English words (Re:CoM's look draws them).
        let text = r#"{"format": "kh1-recom-assets command-menu 1", "words": {"us": {"1": "Attack", "2": "Magic", "3": "Items"}}}"#;
        let w = parse_words(text, "fr").unwrap();
        assert_eq!(w.get(crate::sleights::ROW_WORD), Some(&b"Sleights\0"[..]));
        assert_eq!(w.get(crate::sleights::word(crate::abilities::sleight::CROSS_SLASH_PLUS)), Some(&b"Cross-slash+\0"[..]));
        assert_eq!(crate::sleights::words().filter(|(id, _)| w.get(*id).is_some()).count(), crate::sleights::MAX - crate::sleights::SPECIALS.len());
    }

    /// The asset step writes the same words under the same ids (its `command_menu::SLEIGHT_WORDS`
    /// and `OUR_WORDS`; the crates don't depend on each other).
    #[test]
    fn the_asset_steps_sleight_words_match() {
        let src = include_str!("../../kh1-recom-assets/src/command_menu.rs");
        for (id, name) in crate::sleights::words() {
            let line = format!("({id:#06X}, \"{name}\")");
            assert!(src.contains(&line), "the asset step lacks {line}");
        }
        assert!(src.contains(&format!("({}, \"{}\")", crate::sleights::ROW_WORD, crate::sleights::ROW_TEXT)));
        // 0.111.0: the Party row and names, and the arrow's motion.
        assert!(src.contains(&format!("({}, \"{}\")", cmd::PARTY_WORD, PARTY_TEXT)));
        for (id, name) in crate::party::words().chain(crate::riku::words()) {
            let line = format!("({id:#06X}, \"{name}\")");
            assert!(src.contains(&line), "the asset step lacks {line}");
        }
        assert!(src.contains(&format!("pub const INTRO: [(i32, i32); 6] = {:?};", arrow::INTRO).replace("LOOP_START", "67").replace(&format!("({}, 2)", arrow::LOOP_START), "(LOOP_START, 2)")));
        assert!(src.contains(&format!("pub const SWAY: [(i32, i32); 7] = {:?};", arrow::SWAY)));
        assert!(src.contains(&format!("pub const LOOP_START: i32 = {};", arrow::LOOP_START)));
    }

    #[test]
    fn summon_list_place_is_beside_its_row() {
        let p2 = |w: World| World { page: Page::Two, ..w };
        // One summon: beside page 2's row 1 (260 + 22 = 282), x + 11.
        let mut one = p2(summon_world());
        one.summon_count = 1;
        let mut m = Menu::new();
        m.show();
        m.step(&one, press(|p| p.down = true));
        m.step(&one, press(|p| p.confirm = true));
        let sc = m.scene();
        let sl: Vec<_> = sc.rows.iter().filter(|r| r.list == Kind::Summon).collect();
        assert_eq!((sl[0].x, sl[0].y, sl.len(), sl[0].name), (27.0, 282.0, 1, 291));
        // Three: 282 (348 − 66 too); Cloud's word is ours.
        let mut m = Menu::new();
        m.show();
        let w = p2(summon_world());
        m.step(&w, press(|p| p.down = true));
        m.step(&w, press(|p| p.confirm = true));
        let sc = m.scene();
        let ys: Vec<(f32, u16)> = sc.rows.iter().filter(|r| r.list == Kind::Summon).map(|r| (r.y, r.name)).collect();
        assert_eq!(ys, vec![(282.0, crate::summons::CLOUD_NAME), (304.0, 291), (326.0, 286)]);
        // The Magic list with two spells (summons learned): at the Magic row (282): the Summon entry
        // is no longer in it (0.111.0).
        let mut two = [0; 7];
        two[0] = 2;
        two[3] = 1;
        let w = World { spells: two, ..summon_world() };
        let mut m = Menu::new();
        m.show();
        m.step(&w, press(|p| p.down = true));
        m.step(&w, press(|p| p.confirm = true));
        let ys: Vec<f32> = m.scene().rows.iter().filter(|r| r.list == Kind::Magic).map(|r| r.y).collect();
        assert_eq!(ys, vec![282.0, 304.0]);
        // KH1's rule for each list: min(its row's y, 348 − 22 n).
        assert_eq!([Kind::Magic, Kind::Items, Kind::Summon, Kind::Party, Kind::Sleight].map(list_row), [1, 2, 1, 2, 3]);
    }

    /// 0.111.0: the Party list: Donald Magic (2 MP), Wild Crush (3 MP), Lucky Bounty (1 MP)
    /// learned; Donald Magic and Lucky Bounty available here (Wild Crush not: Goofy's card can't
    /// drop, say); page 2, in battle, 2 MP.
    fn party_world() -> World {
        World { page: Page::Two, mp: Some((2, 3, 0)), party_learned: 0b11_0001, party_avail: 0b10_0001, ..summon_world() }
    }

    #[test]
    fn party_is_page_2s_row_2_with_the_available_friend_sleights() {
        let w = party_world();
        let mut m = Menu::new();
        m.build(&w);
        assert_eq!(m.root.cmds[2], cmd::PARTY);
        assert_eq!(m.party.cmds, vec![crate::party::command(0), crate::party::command(5)], "only the available ones, the owner's order");
        assert!(!m.party_grey && !m.party_lock);
        assert_eq!(cmd::name(m.party.cmds[0]), crate::sleights::WORD_BASE + 0x45);
        // 1 MP: Donald Magic (2) greyed and locked, Lucky Bounty usable.
        m.build(&World { mp: Some((1, 3, 0)), ..w });
        assert_eq!((m.party.grey & 3, m.party.lock & 3), (0b01, 0b01));
        // None available here: greyed and locked; confirm buzzes.
        let w0 = World { party_avail: 0, ..w };
        let mut m = Menu::new();
        m.step(&w0, press(|p| p.down = true));
        m.step(&w0, press(|p| p.down = true));
        assert!(m.party_grey && m.party_lock && m.party.cmds.is_empty());
        let o = m.step(&w0, press(|p| p.confirm = true));
        assert!(!m.open && o.sounds == vec![Sound::Error]);
        // A sleight running: locked.
        m.build(&World { summon_out: true, ..w });
        assert!(m.party_lock);
        // Opens beside row 2 (two entries: 304 = 348 − 44), issues through KH1's gate.
        let mut m = Menu::new();
        m.show();
        m.step(&w, press(|p| p.down = true));
        m.step(&w, press(|p| p.down = true));
        let o = m.step(&w, press(|p| p.right = true));
        assert!(m.open && m.sub == Kind::Party);
        assert_eq!((o.sounds, o.events), (vec![Sound::Open], vec![Event::Opened(Kind::Party)]));
        let ys: Vec<(f32, f32)> = m.scene().rows.iter().filter(|r| r.list == Kind::Party).map(|r| (r.x, r.y)).collect();
        assert_eq!(ys, vec![(27.0, 304.0), (27.0, 326.0)]);
        let o = m.step(&World { gate_any: false, ..w }, press(|p| p.confirm = true));
        assert_eq!(o.events, vec![Event::Party(crate::party::command(0), ShortResult::NotNow)]);
        assert!(m.open && o.sleight.is_none());
        let o = m.step(&w, press(|p| p.confirm = true));
        assert_eq!((o.sleight, o.sounds.clone()), (Some(0x45), vec![]));
        assert!(!m.open && m.root.cursor == 0 && !o.attack);
        // Back on page 1 the Party list closes (another frame's world).
        let mut m = Menu::new();
        m.step(&w, press(|p| p.down = true));
        m.step(&w, press(|p| p.down = true));
        m.step(&w, press(|p| p.confirm = true));
        assert!(m.open);
        m.step(&World { page: Page::One, ..w }, Pad::default());
        assert!(!m.open);
    }

    #[test]
    fn the_arrow_follows_the_root_cursor() {
        let w = summon_world();
        let mut m = Menu::new();
        m.show();
        m.step(&w, Pad::default());
        assert_eq!(m.scene().arrow, Some(ArrowView { row: 0, x: ROOT_X, y: ROOT_Y }));
        m.step(&w, press(|p| p.down = true));
        assert_eq!(m.scene().arrow.map(|a| (a.row, a.y)), Some((1, ROOT_Y + PITCH)));
        // Both pages.
        let o = m.step(&w, press(|p| p.left = true));
        assert_eq!(o.page, Some(Page::Two));
        assert_eq!(m.scene().arrow.map(|a| a.row), Some(1));
        // Not while a list is open (the list's cursor is the plate), nor with the shortcut list.
        m.step(&World { page: Page::Two, ..w }, press(|p| p.confirm = true));
        assert!(m.open && m.scene().arrow.is_none());
        let mut m = Menu::new();
        m.step(&World { shortcuts: [1, 0, 0], ..w }, held(|_| {}));
        assert!(m.short_on && m.scene().arrow.is_none());
        // The motion: KH2's swing-in, then its sway, in integer keys.
        assert_eq!(arrow::INTRO.last().copied(), Some((arrow::LOOP_START, arrow::SWAY[0].1)));
        assert_eq!(arrow::SWAY.first().map(|k| k.1), arrow::SWAY.last().map(|k| k.1), "the loop closes");
        assert!(arrow::SWAY.windows(2).all(|k| k[0].0 < k[1].0) && arrow::INTRO.windows(2).all(|k| k[0].0 < k[1].0));
        let (lo, hi) = arrow::SWAY.iter().fold((i32::MAX, i32::MIN), |a, k| (a.0.min(k.1), a.1.max(k.1)));
        assert_eq!(hi - lo, 3, "±1.5 units");
        // Its place: left of the plate.
        assert!(arrow::at(true).0 + arrow::SIZE.0 / 2.0 + arrow::GAP <= crate::kh1_hud::menu::SEL_AT.0);
        assert!(arrow::at(false).0 + arrow::SIZE.0 / 2.0 <= 0.0);
    }

    #[test]
    fn words_manifest_and_font_bytes() {
        let j = r#"{"format": "kh1-recom-assets command-menu 1", "plates": {}, "words": {"us": {"1": "Attack", "2": "Magic", "3": "Items", "19": "Gravity"}, "fr": {"1": "Attaquer", "19": "Gravité"}}, "sources": {}}"#;
        let fr = parse_words(j, "fr").unwrap();
        assert_eq!(fr.get(1), Some(&b"Attaquer\0"[..]));
        assert_eq!(fr.get(2), Some(&b"Magic\0"[..]), "missing ids come from us");
        assert_eq!(fr.get(19), Some(&b"Gravite\0"[..]));
        assert_eq!(fr.folded, vec!['é']);
        assert_eq!(font_bytes("Todesstoß"), (b"Todesstoss\0".to_vec(), vec!['ß']));
        let xx = parse_words(j, "xx").unwrap();
        assert_eq!((xx.lang.as_str(), xx.get(19)), ("us", Some(&b"Gravity\0"[..])));
        assert!(parse_words(&j.replace("menu 1", "menu 9"), "us").is_err());
        assert_eq!(font_bytes("Brasier +").0, b"Brasier +\0");
        assert_eq!(font_bytes("Öffnen").0, b"Offnen\0");
    }

    #[test]
    fn pad_word_is_kh1s() {
        assert_eq!(press(|p| p.up = true).word(), 0x20);
        assert_eq!(press(|p| p.right = true).word(), 0x100);
        assert_eq!(press(|p| p.cancel = true).word() & 2, 2);
        assert_eq!(
            press(|p| {
                p.up = true;
                p.confirm = true
            })
            .word(),
            0x21_0021
        );
        assert!(cmd::row0_special(cmd::TALK) && cmd::row0_special(0x58) && !cmd::row0_special(cmd::ATTACK) && !cmd::row0_special(cmd::OPEN));
        assert_eq!(state_flags(0) & 0x70, 0x70);
    }

    #[test]
    fn recom_objects_give_kh1s_context_commands() {
        use recom::{field_action, kind, point};
        // Re:CoM's field action's objects (7.3).
        assert_eq!(field_action(kind::BOX, false), cmd::OPEN);
        assert_eq!(field_action(kind::BOX, true), cmd::NONE);
        assert_eq!([field_action(kind::DOOR, false), field_action(kind::JUMP_STAND, false), field_action(kind::CARRY, false), field_action(kind::NPC, false)], [cmd::OPEN, cmd::JUMP_ON, cmd::LIFT, cmd::TALK]);
        // The save / warp points are not the field action's: their own updates take Triangle.
        assert_eq!(field_action(kind::SAVE_POINT, false), cmd::NONE);
        assert_eq!(field_action(kind::WARP_POINT, false), cmd::NONE);
        // Save: Sora within its range, Re:CoM's action 1..4 (stand 2, walk 3, run 4).
        for a in 1..=4 {
            assert_eq!(point(kind::SAVE_POINT, a, true, false), cmd::SAVE, "{a}");
            assert_eq!(point(kind::WARP_POINT, a, false, false), cmd::EXAMINE, "{a}");
        }
        assert_eq!(point(kind::SAVE_POINT, 2, false, false), cmd::NONE, "out of its range");
        for a in [0, 5, 6, 7, 8, 0xA] {
            assert_eq!(point(kind::SAVE_POINT, a, true, false), cmd::NONE, "{a}: not 1..4");
            assert_eq!(point(kind::WARP_POINT, a, false, false), cmd::NONE, "{a}: not 1..4");
        }
        assert_eq!(point(kind::WARP_POINT, 2, false, true), cmd::NONE, "stopped (0x14087B84C & 1)");
        assert_eq!(point(kind::BOX, 2, true, false), cmd::NONE);
        // KH1's words: Save (293), Examine (5); neither a spell, a submenu nor a row-0 special.
        assert_eq!((cmd::name(cmd::SAVE), cmd::name(cmd::EXAMINE)), (293, 5));
        for c in [cmd::SAVE, cmd::EXAMINE] {
            assert!(cmd::spell_of(c).is_none() && cmd::submenu(c) == 0 && !cmd::row0_special(c));
        }
    }

    /// Fire 2, Blizzard 1, Cure 1, Stop 1; slots Fire, Cure, none.
    fn short_world() -> World {
        let mut spells = [0; 7];
        spells[0] = 2;
        spells[1] = 1;
        spells[3] = 1;
        spells[5] = 1;
        World { spells, mp: Some((3, 3, 0)), shortcuts: [1, 4, 0], ..world() }
    }

    fn held(f: impl FnOnce(&mut Pad)) -> Pad {
        let mut p = Pad { l1: true, ..Pad::default() };
        f(&mut p);
        p
    }

    #[test]
    fn shortcut_list_builds_from_the_slots() {
        let w = short_world();
        let mut m = Menu::new();
        m.build(&w);
        // Fira (level 2), Cure, the blank command; the empty slot greyed.
        assert_eq!(m.shortcut.cmds, vec![cmd::spell(0, 2), cmd::spell(3, 1), cmd::BLANK]);
        assert_eq!(m.shortcut.grey & 7, 0b100);
        assert!(m.short_ok);
        // Out of MP: Fire (30 charge units) and Cure (1 MP) grey; the list still shows.
        m.build(&World { mp: Some((0, 3, 0)), ..w });
        assert_eq!((m.shortcut.grey & 7, m.short_ok), (0b111, true));
        // A state without magic (glide 0x15): all greyed.
        m.build(&World { state: 0x15, ..w });
        assert_eq!(m.shortcut.grey & 7, 0b111);
        // No slot set, or no spell learned: no list (L1 does nothing).
        m.build(&World { shortcuts: [0; 3], ..w });
        assert!(!m.short_ok);
        m.build(&World { spells: [0; 7], ..w });
        assert!(!m.short_ok);
        // A ranged spell beyond its range greys (Stop, 1100).
        m.build(&World { shortcuts: [6, 0, 0], target_dist: Some(1200.0), ..w });
        assert_eq!((m.shortcut.cmds[0], m.shortcut.grey & 1), (cmd::spell(5, 1), 1));
        // Slot bytes: 0 none, else spell + 1.
        assert_eq!((shortcut::slot_spell(0), shortcut::slot_spell(1), shortcut::slot_spell(7), shortcut::slot_spell(8), shortcut::slot_byte(6)), (None, Some(0), Some(6), None, 7));
    }

    #[test]
    fn l1_swaps_the_root_for_the_shortcut_list_and_back_on_attack() {
        let w = short_world();
        let mut m = Menu::new();
        m.show();
        // Magic open, cursor on Magic: L1 closes it and shows the list, no sound.
        m.root.cursor = 1;
        m.step(&w, press(|p| p.right = true));
        assert!(m.open);
        let o = m.step(&w, held(|_| {}));
        assert!(m.short_on && !m.open && o.sounds.is_empty());
        assert_eq!(o.events, vec![Event::ShortcutList(true)]);
        // While held: confirm is not Attack, Triangle no context, Square no roll; the jump stays.
        assert!(!o.attack && !o.triangle && !o.square && o.jump);
        // The d-pad does nothing (no cursor in the list).
        let o = m.step(&w, held(|p| p.down = true));
        assert!(o.sounds.is_empty() && o.events.is_empty() && m.root.cursor == 1);
        // Released: the root, cursor on Attack.
        let o = m.step(&w, Pad::default());
        assert!(!m.short_on && m.root.cursor == 0 && o.events == vec![Event::ShortcutList(false)]);
        assert!(o.square && o.jump);
        // No slot set: L1 changes nothing.
        let mut n = Menu::new();
        let o = n.step(&World { shortcuts: [0; 3], ..w }, held(|p| p.confirm = true));
        assert!(!n.short_on && o.attack);
        // The scene: the panel only (no rows, no tab), the empty slot's dash greyed.
        m.step(&w, held(|_| {}));
        m.tick(1.0);
        let s = m.scene();
        assert!(s.rows.is_empty() && s.tab.is_none());
        let sc = s.shortcut.unwrap();
        assert_eq!(sc.map(|v| v.name), [cmd::name(cmd::spell(0, 2)), cmd::name(cmd::spell(3, 1)), cmd::BLANK_NAME]);
        assert_eq!(sc.map(|v| v.bright), [BRIGHT_ON as u8, BRIGHT_ON as u8, BRIGHT_GREY as u8]);
        assert!(m.trace_token(&[]).starts_with("menu sc g4"));
        // Hidden HUD (events): the base goes back to the root.
        m.hide();
        assert!(!m.short_on);
    }

    #[test]
    fn shortcut_buttons_cast_the_slots_as_kh1() {
        let w = short_world();
        let mut m = Menu::new();
        m.show();
        m.step(&w, held(|_| {}));
        // Triangle: slot 1 (Fira), issued with no menu sound; the list stays.
        let o = m.step(&w, held(|p| p.triangle = true));
        assert_eq!(o.cast, Some(cmd::spell(0, 2)));
        assert!(o.sounds.is_empty() && m.short_on && !o.triangle);
        assert_eq!(o.events, vec![Event::Shortcut(0, cmd::spell(0, 2), ShortResult::Cast)]);
        // Square: slot 2 (Cure), cast at once (no party target list yet).
        let o = m.step(&w, held(|p| p.square = true));
        assert_eq!((o.cast, o.square), (Some(cmd::spell(3, 1)), false));
        // Cross (Re:CoM's confirm): slot 3, empty: the error sound, nothing cast, no Attack.
        let o = m.step(&w, held(|p| p.confirm = true));
        assert_eq!((o.cast, o.attack, o.sounds.clone()), (None, false, vec![Sound::Error]));
        assert_eq!(o.events, vec![Event::Shortcut(2, cmd::BLANK, ShortResult::Greyed)]);
        // Only the first of a frame: Triangle before Square before Cross.
        let o = m.step(&w, held(|p| {
            p.square = true;
            p.triangle = true;
            p.confirm = true
        }));
        assert_eq!(o.events, vec![Event::Shortcut(0, cmd::spell(0, 2), ShortResult::Cast)]);
        let o = m.step(&w, held(|p| {
            p.square = true;
            p.confirm = true
        }));
        assert_eq!(o.cast, Some(cmd::spell(3, 1)));
        // A fresh jump takes the frame: no shortcut.
        let o = m.step(&w, held(|p| {
            p.cancel = true;
            p.triangle = true
        }));
        assert!(o.jump && o.cast.is_none() && o.events.is_empty());
        // Short of MP: greyed, the error sound.
        let dry = World { mp: Some((0, 3, 0)), ..w };
        let o = m.step(&dry, held(|p| p.triangle = true));
        assert_eq!((o.cast, o.sounds.clone()), (None, vec![Sound::Error]));
        // Sora busy: dropped silently (not buffered); Fire alone from frame 14 of its release.
        let busy = World { gate_any: false, gate_fire: false, ..w };
        let o = m.step(&busy, held(|p| p.triangle = true));
        assert_eq!((o.cast, o.sounds.is_empty()), (None, true));
        assert_eq!(o.events, vec![Event::Shortcut(0, cmd::spell(0, 2), ShortResult::NotNow)]);
        assert_eq!(m.step(&busy, Pad::default()).cast, None, "not buffered");
        m.step(&w, held(|_| {}));
        let fire_only = World { gate_any: false, gate_fire: true, ..w };
        assert_eq!(m.step(&fire_only, held(|p| p.triangle = true)).cast, Some(cmd::spell(0, 2)));
        assert_eq!(m.step(&fire_only, held(|p| p.square = true)).cast, None, "Cure isn't taken from Fire's release");
    }

    /// M12 step 6 (owner: "I also would like sleights to be able to be added to the customize menu,
    /// for shortcuts"): slots Fire, Fire Raid, Homing Blizzara; both sleights equipped; in battle.
    fn sleight_short_world() -> World {
        let mut sleights = [0u8; crate::sleights::MAX];
        sleights[0] = 0x1B;
        sleights[1] = 0x24;
        World { shortcuts: [1, 0x1B, 0x24], battle: true, sleights, sleight_count: 2, ..short_world() }
    }

    #[test]
    fn sleight_shortcuts_run_as_the_sleights_list() {
        use crate::abilities::sleight as s;
        // The slot bytes: a sleight is its Re:CoM id (the 27 only, all ≥ 8: never a spell).
        assert_eq!(shortcut::slot_of(0x1B), Some(shortcut::Slot::Sleight(s::FIRE_RAID)));
        assert_eq!(shortcut::slot_of(3), Some(shortcut::Slot::Spell(2)));
        for b in [0u8, 8, 9, 0x1A, 0x34, 0x42, 0x44, 0xFF] {
            assert_eq!(shortcut::slot_of(b), None, "{b:#x}: not one of the 27 (Sonic Blade 9, a summon's LV3 0x42…)");
        }
        for id in crate::abilities::SLEIGHTS.filter_map(crate::abilities::sleight_of) {
            let b = shortcut::sleight_byte(id).unwrap();
            assert!(b >= 8 && shortcut::slot_spell(b).is_none(), "{id:#x}");
            assert_eq!(shortcut::slot_of(shortcut::byte_of(shortcut::Slot::Sleight(id))), Some(shortcut::Slot::Sleight(id)));
        }
        assert_eq!(shortcut::sleight_byte(s::SONIC_BLADE), None);
        assert!(shortcut::is_set(0x43) && !shortcut::is_set(0x42));

        let w = sleight_short_world();
        let mut m = Menu::new();
        m.show();
        m.build(&w);
        assert_eq!(m.shortcut.cmds, vec![cmd::spell(0, 2), crate::sleights::command(s::FIRE_RAID), crate::sleights::command(0x24)]);
        // MP 3: Fire Raid (2 MP) and Homing Blizzara (2) usable on the ground in battle.
        assert_eq!(m.shortcut.grey & 7, 0);
        // The panel's words: the sleights' (as the Sleights list names them).
        assert_eq!(cmd::name(m.shortcut.cmds[1]), crate::sleights::word(s::FIRE_RAID));
        // The Sleights list's greying: out of battle, short of MP, a sleight running, mid-air (only
        // Re:CoM's air-start ones: Homing Blizzara).
        m.build(&World { battle: false, ..w });
        assert_eq!(m.shortcut.grey & 6, 6);
        m.build(&World { mp: Some((1, 3, 0)), ..w });
        assert_eq!(m.shortcut.grey & 6, 6);
        m.build(&World { summon_out: true, ..w });
        assert_eq!(m.shortcut.grey & 6, 6);
        m.build(&World { state: 2, ..w });
        assert_eq!(m.shortcut.grey & 6, 0b010, "air: Fire Raid greyed, Homing Blizzara usable");
        // Unequipped later: the blank command, greyed; the slot byte untouched.
        let un = World { sleight_count: 1, ..w };
        m.build(&un);
        assert_eq!((m.shortcut.cmds[2], m.shortcut.grey & 4), (cmd::BLANK, 4));
        // A sleight slot shows the list even with no spell learned (the spell slot then doesn't).
        m.build(&World { spells: [0; 7], ..w });
        assert!(m.short_ok);
        assert_eq!(m.shortcut.cmds[0], cmd::BLANK);
        // On either page (the shortcut doesn't need the Sleights row).
        assert_eq!(w.page, Page::One);

        // Square: slot 2 (Fire Raid): Re:CoM's sleight start, no cast, no menu sound.
        m.step(&w, held(|_| {}));
        let o = m.step(&w, held(|p| p.square = true));
        assert_eq!((o.sleight, o.cast, o.square), (Some(s::FIRE_RAID), None, false));
        assert!(o.sounds.is_empty() && m.short_on);
        assert_eq!(o.events, vec![Event::Shortcut(1, crate::sleights::command(s::FIRE_RAID), ShortResult::Cast)]);
        // Greyed: the error sound, nothing started.
        let o = m.step(&World { mp: Some((1, 3, 0)), ..w }, held(|p| p.confirm = true));
        assert_eq!((o.sleight, o.sounds.clone()), (None, vec![Sound::Error]));
        assert_eq!(o.events, vec![Event::Shortcut(2, crate::sleights::command(0x24), ShortResult::Greyed)]);
        let o = m.step(&un, held(|p| p.confirm = true));
        assert_eq!((o.sleight, o.sounds.clone()), (None, vec![Sound::Error]), "unequipped");
        // KH1's commit gate (Sora busy): dropped silently, as the Sleights list.
        let o = m.step(&World { gate_any: false, ..w }, held(|p| p.square = true));
        assert_eq!((o.sleight, o.sounds.is_empty()), (None, true));
        assert_eq!(o.events, vec![Event::Shortcut(1, crate::sleights::command(s::FIRE_RAID), ShortResult::NotNow)]);
    }

    /// 0.111.0 (owner: "friend sleights should be able to be put in the shortcuts"): a slot holding a
    /// Party sleight (its Re:CoM id) shows it once Re:CoM taught it, usable where the Party list
    /// would offer it (greyed where its card can't drop), and starts it as the Party list does.
    #[test]
    fn party_shortcuts_follow_the_party_lists_availability() {
        assert_eq!(shortcut::slot_of(0x45), Some(shortcut::Slot::Sleight(0x45)));
        assert_eq!(shortcut::sleight_byte(0x4E), Some(0x4E));
        assert_eq!(shortcut::slot_of(0x47), None, "Goofy Tornado LV2 is not listed");
        let w = World { shortcuts: [0x45, 0x4C, 0], battle: true, party_learned: 0b100_0001, party_avail: 0b1, ..short_world() };
        let mut m = Menu::new();
        m.build(&w);
        assert!(m.short_ok);
        assert_eq!(m.shortcut.cmds[..2], [crate::party::command(0), crate::party::command(6)]);
        assert_eq!(m.shortcut.grey & 3, 0b10, "Sandstorm taught but not available here: greyed");
        // Not taught: the blank dash, greyed.
        m.build(&World { party_learned: 0b1, ..w });
        assert_eq!((m.shortcut.cmds[1], m.shortcut.grey & 2), (cmd::BLANK, 2));
        // Out of battle: greyed.
        m.build(&World { battle: false, party_avail: 0, ..w });
        assert_eq!(m.shortcut.grey & 1, 1);
        // Triangle: Donald Magic through Re:CoM's sleight start.
        m.step(&w, held(|_| {}));
        let o = m.step(&w, held(|p| p.triangle = true));
        assert_eq!((o.sleight, o.cast), (Some(0x45), None));
        let o = m.step(&w, held(|p| p.square = true));
        assert_eq!((o.sleight, o.sounds), (None, vec![Sound::Error]));
    }

    #[test]
    fn the_tab_waits_while_the_shortcut_list_is_up() {
        let w = World { context: cmd::OPEN, ..short_world() };
        let mut m = Menu::new();
        m.show();
        for _ in 0..4 {
            m.step(&w, Pad::default());
            m.tick(1.0);
        }
        let before = m.tab;
        assert_eq!(before.state, 1);
        for _ in 0..10 {
            m.step(&w, held(|_| {}));
            m.tick(1.0);
        }
        assert_eq!(m.tab, before, "the tab steps in the root's draw only");
        // Back at the root: the root's brightness is set at once.
        m.step(&w, Pad::default());
        m.tick(1.0);
        assert_eq!(m.root.bright[0], BRIGHT_ON);
        assert!(m.tab.alpha > before.alpha);
        // Re:CoM's confirm key: Circle (0x2000) or Cross.
        assert!(shortcut::confirm_is_circle(0x2000) && !shortcut::confirm_is_circle(0x4000));
    }

    #[test]
    fn save_point_shows_save_in_the_tab() {
        let w = World { context: cmd::SAVE, ..world() };
        let mut m = Menu::new();
        m.show();
        m.build(&w);
        let mut frames = 0;
        while m.tab.state != 2 {
            m.step(&w, Pad::default());
            m.tick(1.0);
            frames += 1;
            assert!(frames < 30);
        }
        let t = m.scene().tab.expect("the tab");
        assert_eq!(t.name, 293);
        // The 4th row stays FM's (not the context command).
        assert!(m.scene().rows.iter().all(|r| r.name != 293));
        // Triangle once it is in: logged; Re:CoM's own request bit stays (the save point reads
        // the pad itself).
        let o = m.step(&w, press(|p| p.triangle = true));
        assert!(o.triangle && o.events == vec![Event::Context(cmd::SAVE)]);
    }
}
