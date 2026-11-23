//! The "KH1" camp entry's settings pages (owner, 2026-11-03): **Combat** (Combat, Guard reactions,
//! Enemy pace, Enemy AI, Revenge Value) and **Movement** (Run speed, Jump height, Air speed, Dodge
//! roll); M13 (owner, 2026-11-04: "The Effects / Sounds (later Voices) toggles go on a new Settings
//! page"): **Effects & Sound** (Effects, Sounds; Voices once voices exist). The testable part.
//!
//! Each page is Re:CoM's own Config screen machinery (`CCamp`'s Config draw `0x1400DF1B0`, its
//! window panel, its `CButton` label and value items at its `camp.lad` positions, its cursor sprite,
//! its help line, its sounds), opened from our submenu (`win::keyblades`) with our rows in place of
//! Re:CoM's five: rows 0..4 sit exactly where Re:CoM's rows 0..4 sit, so the panel needs no
//! resizing. A two-value row is drawn as Re:CoM's rows 0..3 (both options, the chosen one
//! highlighted); the three-value Dodge roll row as Re:CoM's row 4 "Game Mode" (one value item
//! showing the current option). Left / right change the row under the cursor as Re:CoM's rows do.
//!
//! Re:CoM's own Config screen (camp menu > Config) is back to exactly Re:CoM's five rows: the rows
//! we used to add there (Combat, Guard reactions, Enemy pace) moved here.
//! See `docs/recom-config-menu.md`.

use crate::settings::{Camera, Mode, RollMode, Settings};

/// Item slots in `camp+0xB38` (21 pointers, cleared and deleted by Re:CoM's own code in draw
/// modes 6 and 9 and in the camp's teardown `0x1400D7C20`). Checked (decomp: every reference to
/// `camp+0xB38..0xBE0`). Our pages fill them from slot 0, as Re:CoM's mode 9 does.
pub const SLOT_BASE: usize = 0xB38;
pub const SLOT_COUNT: usize = 21;

/// Re:CoM's Config screen has 5 rows (0..4); a page of ours never has more, so every row has
/// Re:CoM's own position. Checked (decomp `0x1400DF1B0` mode 9).
pub const MAX_ROWS: usize = 5;

/// The help box shows about 69 characters of one line and cuts the rest (owner's screenshots,
/// 2026-10-31: the Combat help showed up to "combat." at 68 characters, the Guard reactions one
/// up to "KH1 com" at 69). Checked (screenshot, at 1275 px wide); kept a few under.
pub const HELP_MAX: usize = 66;

/// How a row shows its values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// Both options side by side, the chosen one highlighted (Re:CoM's rows 0..3; 3 items).
    Pair,
    /// One wide value showing the current option (Re:CoM's row 4, "Game Mode"; 2 items).
    Single,
}

/// A settings page of the KH1 camp entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Combat,
    Movement,
    /// M13: KH1's or Re:CoM's effects and sounds on Sora's KH1 moves (ini `effects` / `sounds`).
    EffectsSound,
    /// M12 camera: Re:CoM's, KH1's or KH2's player camera (ini `camera`; owner, 2026-11-04: "a
    /// new Settings > Camera page", independent of Combat).
    Camera,
    /// Fork (0.154.1, owner 2026-11-22: "uma nova opção no menu chamada Costumes"): Riku's look,
    /// Default Attire or Dark Mode (`crate::riku_skin`), from Riku's top list.
    Costumes,
}

/// A row of a settings page: one `kh1-recom.ini` key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    /// `combat`
    Combat,
    /// `guard_reactions`
    GuardReactions,
    /// `enemy_pace`
    EnemyPace,
    /// `enemy_ai`
    EnemyAi,
    /// `boss_revenge` (owner, 2026-11-03: the row is named "Revenge Value").
    RevengeValue,
    /// `run_speed`
    RunSpeed,
    /// `jump_height`
    JumpHeight,
    /// `air_speed`
    AirSpeed,
    /// `dodge_roll`
    DodgeRoll,
    /// `effects` (M13)
    Effects,
    /// `sounds` (M13)
    Sounds,
    /// `camera` (M12 camera)
    Camera,
    /// `auto_camera` (M12 camera stage 2)
    AutoCamera,
    /// `riku_dark_skin` (fork): Default Attire / Dark Mode.
    Costume,
}

/// When a change takes effect (from what reads the setting; `docs/recom-config-menu.md`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Read live (the next frame after the camp closes).
    Now,
    /// Read when a battle starts (the battle manager's construction / each enemy's).
    NextBattle,
    /// Read when the player is built (the next room or battle).
    NextRoom,
}

impl Page {
    pub const ALL: [Page; 4] = [Page::Combat, Page::Movement, Page::EffectsSound, Page::Camera];

    /// Its rows, top to bottom (owner, 2026-11-03).
    pub fn items(self) -> &'static [Item] {
        match self {
            Page::Combat => &[Item::Combat, Item::GuardReactions, Item::EnemyPace, Item::EnemyAi, Item::RevengeValue],
            Page::Movement => &[Item::RunSpeed, Item::JumpHeight, Item::AirSpeed, Item::DodgeRoll],
            Page::EffectsSound => &[Item::Effects, Item::Sounds],
            Page::Camera => &[Item::Camera, Item::AutoCamera],
            Page::Costumes => &[Item::Costume],
        }
    }

    /// The submenu row's label (also the page's breadcrumb title).
    pub fn label(self) -> &'static [u8] {
        match self {
            Page::Combat => b"Combat\0",
            Page::Movement => b"Movement\0",
            // The owner's name. Re:CoM's escape expander copies bytes under 0x81 as they are
            // (`0x1403480C0`, Checked (decomp)); that its font has a `&` glyph is an Assumption.
            Page::EffectsSound => b"Effects & Sound\0",
            Page::Camera => b"Camera\0",
            Page::Costumes => b"Costumes\0",
        }
    }

    /// The submenu row's help line.
    pub fn help(self) -> &'static [u8] {
        match self {
            Page::Combat => b"Combat, Guard reactions, enemy pace and AI, Revenge Value.\0",
            Page::Movement => b"Run speed, jump height, air speed and Dodge roll.\0",
            Page::EffectsSound => b"KH1's or Re:CoM's effects and sounds on Sora's KH1 moves.\0",
            Page::Camera => b"Re:CoM's, KH1's or KH2's camera, and their Auto camera.\0",
            Page::Costumes => b"Change Riku's attire.\0",
        }
    }
}

const RECOM_KH1: &[&[u8]] = &[b"Re:CoM\0", b"KH1\0"];
const OFF_ON: &[&[u8]] = &[b"Off\0", b"On\0"];
const ROLL: &[&[u8]] = &[b"Re:CoM\0", b"KH1\0", b"Off\0"];
const CAMERAS: &[&[u8]] = &[b"Re:CoM\0", b"KH1\0", b"KH2\0"];
const COSTUMES: &[&[u8]] = &[b"Default Attire\0", b"Dark Mode\0"];

fn mode_index(m: Mode) -> usize {
    match m {
        Mode::Recom => 0,
        Mode::Kh1 => 1,
    }
}

fn mode_of(i: usize) -> Mode {
    if i == 1 {
        Mode::Kh1
    } else {
        Mode::Recom
    }
}

impl Item {
    /// The row's label.
    pub fn label(self) -> &'static [u8] {
        match self {
            Item::Combat => b"Combat\0",
            Item::GuardReactions => b"Guard reactions\0",
            Item::EnemyPace => b"Enemy pace\0",
            Item::EnemyAi => b"Enemy AI\0",
            Item::RevengeValue => b"Revenge Value\0",
            Item::RunSpeed => b"Run speed\0",
            Item::JumpHeight => b"Jump height\0",
            Item::AirSpeed => b"Air speed\0",
            Item::DodgeRoll => b"Dodge roll\0",
            Item::Effects => b"Effects\0",
            Item::Sounds => b"Sounds\0",
            Item::Camera => b"Camera\0",
            Item::AutoCamera => b"Auto camera\0",
            Item::Costume => b"Costume\0",
        }
    }

    /// Its options, in the order left / right cycle through them.
    pub fn options(self) -> &'static [&'static [u8]] {
        match self {
            Item::GuardReactions | Item::RevengeValue | Item::AutoCamera => OFF_ON,
            Item::DodgeRoll => ROLL,
            Item::Camera => CAMERAS,
            Item::Costume => COSTUMES,
            _ => RECOM_KH1,
        }
    }

    /// Two options: Re:CoM's pair row; more: Re:CoM's single-value row.
    pub fn style(self) -> Style {
        // "Default Attire" is too wide for a pair row's value: one value showing the current one.
        if self.options().len() == 2 && self != Item::Costume {
            Style::Pair
        } else {
            Style::Single
        }
    }

    /// The help line with the cursor on the row (at most [`HELP_MAX`] characters, no `%`).
    pub fn help(self) -> &'static [u8] {
        match self {
            Item::Combat => concat!("Re:CoM: card battles. KH1: Kingdom Hearts combat. (mod v", env!("CARGO_PKG_VERSION"), ")\0").as_bytes(),
            Item::GuardReactions => b"On: blocked attackers flinch, blocked shots fly back. KH1 only.\0",
            Item::EnemyPace => b"KH1: each enemy attacks as if alone. KH1 only, from next battle.\0",
            Item::EnemyAi => b"KH1: KH1 enemies' own AI. KH1 only, from the next battle.\0",
            Item::RevengeValue => b"On: humanoid bosses use KH2's revenge value. KH1 only.\0",
            Item::RunSpeed => b"Re:CoM: 6.2 m/s run. KH1: 4.8 m/s. KH1 only.\0",
            Item::JumpHeight => b"Re:CoM: Re:CoM's jump (2.0 m). KH1: KH1's jump (1.9 m). KH1 only.\0",
            Item::AirSpeed => b"Speed and steering in the air: Re:CoM's or KH1's. KH1 only.\0",
            Item::DodgeRoll => b"Re:CoM's roll, KH1's roll, or Off (Square guards). KH1 only.\0",
            Item::Effects => b"KH1: KH1's Keyblade trails and finisher effects. KH1 only.\0",
            Item::Sounds => b"KH1: KH1's swing and finisher sounds. KH1 only.\0",
            Item::Camera => b"Re:CoM's camera, KH1's, or KH2's (its distances x 0.89).\0",
            Item::AutoCamera => b"On: the KH1 / KH2 camera swings behind Sora as he runs.\0",
            Item::Costume => b"Dark Mode: Dark Riku's look, Riku's own moves. From the next room.\0",
        }
    }

    /// When a change takes effect.
    pub fn effect(self) -> Effect {
        match self {
            Item::EnemyPace | Item::EnemyAi => Effect::NextBattle,
            Item::Costume => Effect::NextRoom,
            _ => Effect::Now,
        }
    }

    /// The `kh1-recom.ini` key.
    pub fn key(self) -> &'static str {
        match self {
            Item::Combat => "combat",
            Item::GuardReactions => "guard_reactions",
            Item::EnemyPace => "enemy_pace",
            Item::EnemyAi => "enemy_ai",
            Item::RevengeValue => "boss_revenge",
            Item::RunSpeed => "run_speed",
            Item::JumpHeight => "jump_height",
            Item::AirSpeed => "air_speed",
            Item::DodgeRoll => "dodge_roll",
            Item::Effects => "effects",
            Item::Sounds => "sounds",
            Item::Camera => "camera",
            Item::AutoCamera => "auto_camera",
            Item::Costume => "riku_dark_skin",
        }
    }

    /// The option index the settings hold.
    pub fn value(self, s: &Settings) -> usize {
        match self {
            Item::Combat => mode_index(s.combat),
            Item::GuardReactions => s.guard_reactions as usize,
            Item::EnemyPace => mode_index(s.enemy_pace),
            Item::EnemyAi => mode_index(s.enemy_ai),
            Item::RevengeValue => s.boss_revenge as usize,
            Item::RunSpeed => mode_index(s.run_speed),
            Item::JumpHeight => mode_index(s.jump_height),
            Item::AirSpeed => mode_index(s.air_speed),
            Item::DodgeRoll => match s.dodge_roll {
                RollMode::Recom => 0,
                RollMode::Kh1 => 1,
                RollMode::Off => 2,
            },
            Item::Effects => mode_index(s.effects),
            Item::Sounds => mode_index(s.sounds),
            Item::Camera => match s.camera {
                Camera::Recom => 0,
                Camera::Kh1 => 1,
                Camera::Kh2 => 2,
            },
            Item::AutoCamera => s.auto_camera as usize,
            Item::Costume => s.riku_dark_skin as usize,
        }
    }

    /// The settings with this row at option `v` (everything else kept; Dodge roll also keeps
    /// the ini's ability switch in step, as the ini's parser does).
    pub fn set(self, s: &mut Settings, v: usize) {
        match self {
            Item::Combat => s.combat = mode_of(v),
            Item::GuardReactions => s.guard_reactions = v == 1,
            Item::EnemyPace => s.enemy_pace = mode_of(v),
            Item::EnemyAi => s.enemy_ai = mode_of(v),
            Item::RevengeValue => s.boss_revenge = v == 1,
            Item::RunSpeed => s.run_speed = mode_of(v),
            Item::JumpHeight => s.jump_height = mode_of(v),
            Item::AirSpeed => s.air_speed = mode_of(v),
            Item::DodgeRoll => {
                s.dodge_roll = match v {
                    1 => RollMode::Kh1,
                    2 => RollMode::Off,
                    _ => RollMode::Recom,
                };
                s.abilities.dodge_roll = s.dodge_roll.on();
            }
            Item::Effects => s.effects = mode_of(v),
            Item::Sounds => s.sounds = mode_of(v),
            Item::Camera => {
                s.camera = match v {
                    1 => Camera::Kh1,
                    2 => Camera::Kh2,
                    _ => Camera::Recom,
                }
            }
            Item::AutoCamera => s.auto_camera = v == 1,
            Item::Costume => s.riku_dark_skin = v == 1,
        }
    }
}

/// Re:CoM's change of a row's value (`0x1400D3EB0` state 2): left −1, right +1, wrapping over the
/// option count (its rows have 2: left from 0 gives 1, right from 1 gives 0). Checked (decomp).
pub fn cycle(v: usize, n: usize, right: bool) -> usize {
    if n == 0 {
        return 0;
    }
    if right {
        (v + 1) % n
    } else {
        (v + n - 1) % n
    }
}

/// The input of one frame, as Re:CoM's own `0x140111A70` reports it (masks: up `0x101000`, down
/// `0x204000`, left `0x408000`, right `0x802000`), and `0x140111B90` (confirm or cancel: closes the
/// screen).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    pub close: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

/// What a frame of input does on a settings page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Close the page (Re:CoM: draw mode 4, sound 4).
    Close,
    /// The cursor to this row (sound 1, redraw).
    Move(usize),
    /// The row under the cursor changes: right (true) or left (sound 1, redraw).
    Change(bool),
    Idle,
}

/// Re:CoM's Config input (`0x1400D3EB0` state 2), in its order: confirm / cancel closes; else up
/// (from row 0 to the last), else down (from the last to row 0); else left, else right, on the row
/// under the cursor (every row of ours changes; Re:CoM's own row 4 is display-only). Checked
/// (decomp).
pub fn step(cursor: usize, rows: usize, i: Input) -> Step {
    if i.close {
        return Step::Close;
    }
    if rows == 0 {
        return Step::Idle;
    }
    if i.up {
        return Step::Move(if cursor == 0 || cursor >= rows { rows - 1 } else { cursor - 1 });
    }
    if i.down {
        return Step::Move(if cursor + 1 >= rows { 0 } else { cursor + 1 });
    }
    if i.left {
        return Step::Change(false);
    }
    if i.right {
        return Step::Change(true);
    }
    Step::Idle
}

/// Layout elements (group `0xB` of `camp.lad`, read with `0x140246100`) Re:CoM's rows use.
/// Labels take x from the first and y from the second; values take x from the option's element
/// and y from the row's. Checked (disasm `0x1400DF5B7..0x1400DF878`).
pub mod layout {
    pub const GROUP: u32 = 0xB;
    /// (x element, y element) of the labels of rows 0..4.
    pub const LABEL: [(u64, u64); 5] = [(2, 2), (3, 3), (5, 4), (7, 5), (8, 6)];
    /// x elements of the 9 value items (rows 0-3 × 2, then row 4).
    pub const VALUE_X: [u64; 9] = [9, 10, 11, 12, 15, 16, 20, 21, 22];
    /// y elements of the value rows 0..4.
    pub const VALUE_Y: [u64; 5] = [9, 0xB, 0xD, 0xF, 0x11];
}

/// Cursor animation on a pair row's selected value (Re:CoM's rows 0..3). Checked (disasm
/// `0x1400DF93B`).
pub const CURSOR_ANIM: u32 = 0xFA;
/// Cursor animation on a single-value row (Re:CoM's row 4). Checked (decomp `0x1400DF1B0`).
pub const CURSOR_ANIM_SINGLE: u32 = 0x100;
/// Re:CoM's single-value item: index 8 (sequence `0x1407C1240[8]`, x element `0x16`).
pub const SINGLE_VALUE_ITEM: usize = 8;

/// x offset Re:CoM adds to the cursor sprite on value item `i` (floats at `rbp+0x64`: 0.0 for
/// item 0, 150.0 for items 1..8). Checked (exe + decomp: the table starts with two 0.0 and the
/// pointer starts 4 bytes in).
pub fn cursor_x_offset(item: usize) -> f32 {
    if item == 0 {
        0.0
    } else {
        150.0
    }
}

/// Where a value item of ours goes: Re:CoM's value item whose x element, sequence ids and cursor
/// offset it takes (`item`, 0..8), at row `row`'s y.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValueAt {
    pub item: usize,
    pub x_elem: u64,
}

/// The value items of row `row` (0..4) in `style`: a pair row takes Re:CoM's row's own two items
/// (row 4, a single row in Re:CoM, takes row 3's two, as our former Config rows did); a single row
/// takes Re:CoM's item 8.
pub fn values_at(row: usize, style: Style) -> Vec<ValueAt> {
    match style {
        Style::Single => vec![ValueAt { item: SINGLE_VALUE_ITEM, x_elem: layout::VALUE_X[SINGLE_VALUE_ITEM] }],
        Style::Pair => {
            let r = row.min(3);
            (0..2).map(|o| ValueAt { item: 2 * r + o, x_elem: layout::VALUE_X[2 * r + o] }).collect()
        }
    }
}

/// Item slots a page uses (label + values per row), from slot 0.
pub fn slots_used(page: Page) -> usize {
    page.items().iter().map(|it| 1 + if it.style() == Style::Pair { 2 } else { 1 }).sum()
}

/// Whether a pair row's option `opt` shows as selected (Re:CoM: value byte == option).
pub fn selected(value: usize, opt: usize) -> bool {
    value == opt
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: Input = Input { close: false, up: false, down: false, left: false, right: false };

    #[test]
    fn pages_hold_the_owners_rows() {
        let labels = |p: Page| p.items().iter().map(|i| std::str::from_utf8(i.label()).unwrap().trim_end_matches('\0').to_string()).collect::<Vec<_>>();
        assert_eq!(labels(Page::Combat), ["Combat", "Guard reactions", "Enemy pace", "Enemy AI", "Revenge Value"]);
        assert_eq!(labels(Page::Movement), ["Run speed", "Jump height", "Air speed", "Dodge roll"]);
        assert_eq!(labels(Page::EffectsSound), ["Effects", "Sounds"], "M13; Voices once voices exist");
        assert_eq!((Page::Combat.label(), Page::Movement.label(), Page::EffectsSound.label()), (&b"Combat\0"[..], &b"Movement\0"[..], &b"Effects & Sound\0"[..]));
        let keys: Vec<&str> = Page::ALL.iter().flat_map(|p| p.items().iter().map(|i| i.key())).collect();
        assert_eq!(keys, ["combat", "guard_reactions", "enemy_pace", "enemy_ai", "boss_revenge", "run_speed", "jump_height", "air_speed", "dodge_roll", "effects", "sounds", "camera", "auto_camera"]);
        assert_eq!(labels(Page::Camera), ["Camera", "Auto camera"], "M12 camera stages 1 and 2");
        // Damage stays ini-only (owner).
        assert!(!keys.contains(&"damage"));
    }

    #[test]
    fn pages_fit_recoms_config_screen() {
        for p in Page::ALL {
            assert!(p.items().len() <= MAX_ROWS);
            assert!(slots_used(p) <= SLOT_COUNT, "{p:?}");
        }
        assert_eq!(slots_used(Page::Combat), 15);
        assert_eq!(slots_used(Page::Movement), 11);
        assert_eq!(slots_used(Page::EffectsSound), 6);
        assert_eq!(slots_used(Page::Camera), 5);
        assert_eq!(Item::AutoCamera.options(), OFF_ON);
        assert_eq!(Item::Camera.options(), CAMERAS, "M12 camera stage 4: Re:CoM / KH1 / KH2");
        assert_eq!(Item::Camera.style(), Style::Single, "a single row like Dodge roll");
        assert!(Page::EffectsSound.items().iter().all(|i| i.style() == Style::Pair && i.options() == RECOM_KH1));
        assert_eq!(Item::DodgeRoll.style(), Style::Single);
        assert!(Page::Combat.items().iter().all(|i| i.style() == Style::Pair));
    }

    #[test]
    fn rows_take_recoms_positions() {
        assert_eq!(values_at(0, Style::Pair), vec![ValueAt { item: 0, x_elem: 9 }, ValueAt { item: 1, x_elem: 10 }]);
        assert_eq!(values_at(2, Style::Pair), vec![ValueAt { item: 4, x_elem: 15 }, ValueAt { item: 5, x_elem: 16 }]);
        assert_eq!(values_at(4, Style::Pair), vec![ValueAt { item: 6, x_elem: 20 }, ValueAt { item: 7, x_elem: 21 }]);
        assert_eq!(values_at(3, Style::Single), vec![ValueAt { item: 8, x_elem: 0x16 }]);
        assert_eq!((cursor_x_offset(0), cursor_x_offset(1), cursor_x_offset(8)), (0.0, 150.0, 150.0));
    }

    #[test]
    fn input_follows_recoms_config_input() {
        assert_eq!(step(2, 5, Input { close: true, up: true, ..NONE }), Step::Close);
        assert_eq!(step(0, 5, Input { up: true, ..NONE }), Step::Move(4));
        assert_eq!(step(3, 5, Input { up: true, ..NONE }), Step::Move(2));
        assert_eq!(step(4, 5, Input { down: true, ..NONE }), Step::Move(0));
        assert_eq!(step(3, 4, Input { down: true, ..NONE }), Step::Move(0));
        assert_eq!(step(1, 4, Input { down: true, ..NONE }), Step::Move(2));
        assert_eq!(step(1, 4, Input { up: true, down: true, ..NONE }), Step::Move(0), "up first");
        assert_eq!(step(1, 4, Input { down: true, left: true, ..NONE }), Step::Move(2), "moves before changes");
        assert_eq!(step(1, 4, Input { left: true, right: true, ..NONE }), Step::Change(false), "left first");
        assert_eq!(step(1, 4, Input { right: true, ..NONE }), Step::Change(true));
        assert_eq!(step(1, 4, NONE), Step::Idle);
    }

    #[test]
    fn values_cycle_with_wrap() {
        // Re:CoM's two-value rows: left and right both flip.
        assert_eq!((cycle(0, 2, false), cycle(1, 2, false), cycle(0, 2, true), cycle(1, 2, true)), (1, 0, 1, 0));
        // Dodge roll: Re:CoM -> KH1 -> Off -> Re:CoM.
        assert_eq!((cycle(0, 3, true), cycle(1, 3, true), cycle(2, 3, true)), (1, 2, 0));
        assert_eq!((cycle(0, 3, false), cycle(2, 3, false)), (2, 1));
        let mut s = Settings::default();
        for it in Page::ALL.iter().flat_map(|p| p.items()) {
            let n = it.options().len();
            let start = it.value(&s);
            assert!(start < n, "{it:?}");
            // Right through every option comes back; each step is a different value.
            let mut seen = vec![start];
            for _ in 0..n {
                let v = cycle(it.value(&s), n, true);
                it.set(&mut s, v);
                assert_eq!(it.value(&s), v, "{it:?}");
                seen.push(v);
            }
            assert_eq!(seen.first(), seen.last());
            seen.pop();
            seen.sort();
            seen.dedup();
            assert_eq!(seen.len(), n, "{it:?}");
        }
        assert_eq!(s, Settings::default(), "a full turn of every row changes nothing");
    }

    #[test]
    fn rows_change_only_their_setting_and_round_trip_the_ini() {
        use crate::settings::{parse, store, DEFAULT_FILE};
        for it in Page::ALL.iter().flat_map(|p| p.items()) {
            for v in 0..it.options().len() {
                let mut s = Settings::default();
                it.set(&mut s, v);
                // Only this key differs from the defaults (Dodge roll carries its ability switch).
                let text = store(DEFAULT_FILE, &s);
                let (back, w) = parse(&text);
                assert!(w.is_empty(), "{w:?}");
                assert_eq!(back, s, "{it:?} = {v}");
                assert_eq!(it.value(&back), v);
                let changed: Vec<&str> = text.lines().zip(DEFAULT_FILE.lines()).filter(|(a, b)| a != b).map(|(a, _)| a).collect();
                if v != it.value(&Settings::default()) {
                    assert_eq!(changed.len(), 1, "{it:?}: {changed:?}");
                    assert!(changed[0].starts_with(it.key()), "{it:?}: {changed:?}");
                } else {
                    assert!(changed.is_empty(), "{it:?}: {changed:?}");
                }
            }
        }
        let mut s = Settings::default();
        Item::DodgeRoll.set(&mut s, 2);
        assert_eq!((s.dodge_roll, s.abilities.dodge_roll), (RollMode::Off, false));
        Item::DodgeRoll.set(&mut s, 1);
        assert_eq!((s.dodge_roll, s.abilities.dodge_roll), (RollMode::Kh1, true));
        Item::RevengeValue.set(&mut s, 0);
        assert!(!s.boss_revenge);
    }

    #[test]
    fn when_changes_take_effect() {
        for it in Page::ALL.iter().flat_map(|p| p.items()) {
            let want = if matches!(it, Item::EnemyPace | Item::EnemyAi) { Effect::NextBattle } else { Effect::Now };
            assert_eq!(it.effect(), want, "{it:?}");
        }
    }

    #[test]
    fn texts_are_terminated_ascii_and_fit_the_help_box() {
        let mut all: Vec<&[u8]> = Vec::new();
        for p in Page::ALL {
            all.extend([p.label(), p.help()]);
            for it in p.items() {
                all.extend([it.label(), it.help()]);
                all.extend(it.options().iter().copied());
            }
        }
        for s in all {
            assert_eq!(s.last(), Some(&0));
            assert!(s.len() - 1 <= HELP_MAX, "{}", std::str::from_utf8(s).unwrap());
            assert!(s[..s.len() - 1].iter().all(|&b| (0x20..0x7F).contains(&b)));
            assert!(!s[..s.len() - 1].contains(&b'%'), "the help line formatter treats % as a code");
        }
        assert!(selected(1, 1) && !selected(1, 0) && selected(0, 0));
    }

    #[test]
    fn combat_help_shows_the_mod_version() {
        let help = std::str::from_utf8(Item::Combat.help()).unwrap();
        assert!(help.ends_with(&format!("(mod v{})\0", crate::VERSION)), "{help}");
    }
}
