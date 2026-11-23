//! `kh1-recom.ini`, next to the game. Written with the defaults when missing, read at start.
//!
//! Every setting switches between Re:CoM's behaviour and KH1's. The camp menu's "KH1" entry has two
//! settings pages, Combat and Movement (`crate::menu`), whose rows change these values and rewrite
//! this file at once, so the file stays the backup (and the only store: Re:CoM's own config bytes
//! live in its save data, see `docs/recom-config-menu.md`).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Recom,
    Kh1,
}

impl Mode {
    fn parse(v: &str) -> Option<Mode> {
        match v.trim().to_ascii_lowercase().as_str() {
            "recom" => Some(Mode::Recom),
            "kh1" => Some(Mode::Kh1),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Recom => "recom",
            Mode::Kh1 => "kh1",
        }
    }
}

/// Sora's Dodge Roll at `combat = kh1` (`dodge_roll` in the ini; owner, 2026-11-02: "make dodge
/// roll work how it does in ReCoM, not KH1", "The whole roll"). KH1's input rule decides when the
/// roll is asked for in every case (`crate::movement`); what runs then:
/// - `Recom` (the default): Re:CoM's own roll, its action 6 from its avoid init `0x1401F10C0` to
///   its exe's end: motion, travel, timing, cancel and invulnerability all Re:CoM's;
/// - `Kh1`: KH1's roll as before (KH1's frames on Re:CoM's action 6, KH1's 12 no-flinch frames);
/// - `Off`: no Dodge Roll (Square always guards, KH1's rule without the ability).
///
/// The old values still read: `on` is `Recom`, `off` is `Off`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RollMode {
    Recom,
    Kh1,
    Off,
}

impl RollMode {
    pub fn parse(v: &str) -> Option<RollMode> {
        match v.trim().to_ascii_lowercase().as_str() {
            "recom" | "on" | "1" | "true" | "yes" => Some(RollMode::Recom),
            "kh1" => Some(RollMode::Kh1),
            "off" | "0" | "false" | "no" => Some(RollMode::Off),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            RollMode::Recom => "recom",
            RollMode::Kh1 => "kh1",
            RollMode::Off => "off",
        }
    }
    /// Sora has a Dodge Roll at all.
    pub fn on(self) -> bool {
        self != RollMode::Off
    }
}

/// The camera (M12 camera, owner, 2026-11-04: "Lets also have an option in the config to switch
/// between Re:CoM camera, KH1 Camera, And KH2 camera"; answers: a Settings > Camera page,
/// independent of Combat, default Re:CoM). `crate::camera`, `docs/recom-m12-camera-plan.md`.
/// - `Recom` (the default): Re:CoM's player camera, untouched;
/// - `Kh1`: KH1's follow camera as Re:CoM camera modes (`crate::camera::kh1`);
/// - `Kh2`: KH2's camera as Re:CoM camera modes (`crate::camera::kh2`), its lengths × 0.89.
///
/// Riku's story and boss fights with Re:CoM's boss camera keep Re:CoM's camera at any value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Camera {
    Recom,
    Kh1,
    Kh2,
}

impl Camera {
    pub fn parse(v: &str) -> Option<Camera> {
        match v.trim().to_ascii_lowercase().as_str() {
            "recom" => Some(Camera::Recom),
            "kh1" => Some(Camera::Kh1),
            "kh2" => Some(Camera::Kh2),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Camera::Recom => "recom",
            Camera::Kh1 => "kh1",
            Camera::Kh2 => "kh2",
        }
    }
}

fn parse_switch(v: &str) -> Option<bool> {
    match v.trim().to_ascii_lowercase().as_str() {
        "on" | "1" | "true" | "yes" => Some(true),
        "off" | "0" | "false" | "no" => Some(false),
        _ => None,
    }
}

pub fn switch_str(on: bool) -> &'static str {
    if on {
        "on"
    } else {
        "off"
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    /// Sora's combat: Re:CoM's, or KH1's (from M4: KH1's movement).
    pub combat: Mode,
    /// Enemy reactions to Sora's KH1 Guard (decision 27; only at `combat = kh1`): a blocked melee
    /// attacker plays Re:CoM's own flinch, a blocked shot flies back and can hurt enemies.
    pub guard_reactions: bool,
    /// How often enemies attack in a crowd at `combat = kh1` (decision 32). `Recom`: Re:CoM's
    /// crowd sharing stays (each enemy's think rate is multiplied by the number of enemies,
    /// `0x14017F380`); `Kh1`: each enemy attacks as if alone (decision 30, patch P4).
    pub enemy_pace: Mode,
    /// The AI of Re:CoM's enemies that have a KH1 twin built natively (M10: the Shadow) at
    /// `combat = kh1` (owner, 2026-11-02): `Kh1` (the default): KH1's AI, attacks, reactions and
    /// motions (stats stay Re:CoM's); `Recom`: Re:CoM's own. Decided at each battle start, like
    /// `enemy_pace` (which doesn't apply to KH1-AI enemies). ini only.
    pub enemy_ai: Mode,
    /// Damage and enemy HP at `combat = kh1` (owner, 2026-11-01; `crate::damage`). `Recom` (the
    /// default): Re:CoM's enemy HP, and Sora's KH1 swings, spells and specials carry Re:CoM's AP
    /// for the equipped Keyblade's card / the spell's card or sleight / Sonic Blade's sleight.
    /// `Kh1`: KH1's formula on the KH1 twins' / proxies' records and HP (decisions 12, 21).
    pub damage: Mode,
    /// Boss revenge at `combat = kh1` (owner, 2026-11-01; `crate::revenge`): KH2's revenge value on
    /// Re:CoM's humanoid bosses. The KH1 entry's Combat page row "Revenge Value".
    pub boss_revenge: bool,
    /// M13 (owner, 2026-11-03: Effects / Sounds / Voices, each KH1 / Re:CoM): at `combat = kh1`,
    /// `Kh1` (the default) plays KH1's trail and finisher effects for Sora's KH1 moves (the asset
    /// step's events, `crate::fx`); `Recom` plays Re:CoM's. Read at each motion start.
    pub effects: Mode,
    /// M13: at `combat = kh1`, `Kh1` (the default) plays KH1's swing and finisher sounds for Sora's
    /// KH1 moves; `Recom` Re:CoM's. Read at each motion start.
    pub sounds: Mode,
    /// M12 camera: the player camera ([`Camera`]); the camp's Settings > Camera page. Read every
    /// frame by the camera's dispatcher (`crate::camera`), at any Combat setting.
    pub camera: Camera,
    /// M12 camera stage 2: the KH1 camera's Auto pieces (KH1's Config "field camera" Auto: the
    /// swing behind Sora, the wall-avoid, the eye walk, the visibility re-place, the auto recenter).
    /// The Camera page's Auto camera row; default on (KH1's default, Assumption).
    pub auto_camera: bool,
    /// The camp menu's "KH1" entry (and the Config input wrapper it runs through). `off` leaves
    /// Re:CoM's camp menu untouched (a safety switch; the settings then come from this file only).
    /// The key keeps its old name (it once added rows to Re:CoM's Config menu).
    pub config_menu: bool,
    /// The first-launch build of the game-made files (`asset_build = auto | off`;
    /// `docs/openkh-github-install.md`): `auto` builds them from the player's KH1 and Re:CoM files
    /// when they are missing or out of date, and copies them into Panacea's folder; `off` leaves
    /// that folder as Build made it (the owner's manual asset step). Default auto.
    pub asset_build: bool,
    /// KH1 abilities from the ini (M9 stage 9b, `crate::abilities`): the story abilities' unlock
    /// (High Jump, Glide, Superglide, Dodge Roll, Combo Master), the movement abilities a save
    /// keeps equipped at its migration to the ability state (Guard, Counterattack, Combo Plus, Air
    /// Combo Plus), and all of them while there is no save block.
    pub abilities: Kh1Abilities,
    /// Which Dodge Roll Sora has at `combat = kh1` ([`RollMode`]); `abilities.dodge_roll` follows
    /// it (on unless `Off`). The KH1 entry's Movement page.
    pub dodge_roll: RollMode,
    /// Sora's movement speeds at `combat = kh1` (owner, 2026-11-02; `crate::movement::speeds`), each
    /// Re:CoM's or KH1's. The KH1 entry's Movement page. `run_speed`: the full
    /// run (default Re:CoM's 6.2 m/s × the stick; KH1's 4.8 m/s); the walk stays KH1's.
    pub run_speed: Mode,
    /// `jump_height`: the jump's take-off speed, gravity and release cut (default KH1's 190 / 290
    /// units; Re:CoM's 28 / 32.5 speed).
    pub jump_height: Mode,
    /// `air_speed`: horizontal speed and control in the air (default KH1's).
    pub air_speed: Mode,
    /// One-line logs of the Attack's decisions in `kh1-recom.log` (first 200 lines).
    pub debug_attack: bool,
    /// Testing (fork branch Re-Re-Reverse/Rebirth): at `combat = kh1`, Riku in Reverse/Rebirth
    /// moves with KH1's movement rules on his own Re:CoM animations (`crate::riku`). Default off:
    /// Riku's story stays Re:CoM's.
    pub riku_kh1_movement: bool,
    /// Debug (M7, until stage 7.11 earns spells): Sora has all 7 spells at level 3.
    pub kh1_all_spells: bool,
    /// Debug (M9): every ability of the equip screen is equipped, AP ignored (not written to the
    /// save).
    pub kh1_all_abilities: bool,
    /// Debug (M8 stage 8a.1, until the shop and drops give items): a test inventory: whenever
    /// Sora's slots are all empty as a room or battle starts, they get Potion, Hi-Potion, Ether,
    /// and the stock at least 9 of each battle item (written to the save).
    pub kh1_test_items: bool,
    /// The language of the command menu's KH1 words (`kh1-command-menu.json`): us, uk, fr, gr,
    /// it or sp.
    pub menu_language: String,
    /// The HUD (decision 39): Re:CoM's, or KH1's elements drawn by Re:CoM from the `kh1hud` set
    /// (`kh1-hud.json`). Interim switch until the KH1 camp entry has its row (stage 7.13).
    pub hud: Mode,
    /// Which KH1 HUD pieces show at `hud = kh1`, and which of Re:CoM's are hidden then (owner,
    /// 2026-11-01: "I want to be able to pick and choose which elements i want").
    pub hud_parts: HudParts,
}

/// The KH1 HUD's pieces (each only at `hud = kh1`). Defaults: the owner's picks, 2026-11-01 (the
/// baked KH1 gauge dropped: "Lets just keep the KH1 command menu and the triangle prompts. For the
/// health bar ... get the Re:CoM health bar, place it in the same place as the KH1 health bar";
/// MP "Around the ring").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudParts {
    /// KH1's gauge cluster (HP / MP rings, face, hurt face, low-HP flash, shake); off: Re:CoM's
    /// HP gauge (placed by [`HudParts::recom_hp_place`]) and the MP ring or the 7.x MP bar.
    pub gauge: bool,
    /// With the KH1 gauge off: Re:CoM's HP gauge at KH1's place (bottom right), its ring and bar
    /// turned 180° as KH1's (the face upright); off: Re:CoM's place (top left).
    pub recom_hp_place: bool,
    /// With the KH1 gauge off and Re:CoM's gauge at KH1's place: KH1's MP ring (MP, max MP,
    /// charge, KH1's fades and straight segments) around Re:CoM's HP ring; off: the 7.2 MP bar.
    pub mp_ring: bool,
    /// KH1's command menu look (panels, font, icons, the green Triangle tab); off: 7.3's look.
    pub menu: bool,
    /// Donald's and Goofy's KH1 gauges (drawn with the gauge cluster).
    pub party: bool,
    /// Re:CoM's HP gauge hidden.
    pub hide_hp: bool,
    /// Re:CoM's minimap hidden.
    pub hide_minimap: bool,
    /// Re:CoM's FRIENDS row hidden.
    pub hide_friends: bool,
    /// Re:CoM's EXP / NEXT LV / Moogle point pop-ups hidden.
    pub hide_prizes: bool,
    /// With Re:CoM's HP gauge at KH1's place (bottom right): Re:CoM's room plate (the room card
    /// and its name, bottom right) moved to the top left (owner, 2026-11-01: "the room indicator
    /// should be in the top left of the screen"; 0.40.x: top right, under the minimap); off:
    /// Re:CoM's place. Key `hud_room_plate_top_left`; 0.40.x's `hud_room_plate_top_right` is read
    /// as the same switch.
    pub room_plate_top: bool,
}

impl Default for HudParts {
    fn default() -> Self {
        HudParts { gauge: false, recom_hp_place: true, mp_ring: true, menu: true, party: true, hide_hp: false, hide_minimap: false, hide_friends: true, hide_prizes: false, room_plate_top: true }
    }
}

impl HudParts {
    /// (ini key, value) for each piece, in the file's order.
    pub fn keys(self) -> [(&'static str, bool); 10] {
        [
            ("hud_kh1_gauge", self.gauge),
            ("hud_recom_hp_kh1_place", self.recom_hp_place),
            ("hud_kh1_mp_ring", self.mp_ring),
            ("hud_kh1_menu", self.menu),
            ("hud_kh1_party", self.party),
            ("hud_hide_recom_hp", self.hide_hp),
            ("hud_hide_minimap", self.hide_minimap),
            ("hud_hide_friends", self.hide_friends),
            ("hud_hide_prizes", self.hide_prizes),
            ("hud_room_plate_top_left", self.room_plate_top),
        ]
    }

    fn set(&mut self, key: &str, v: bool) -> bool {
        match key {
            "hud_kh1_gauge" => self.gauge = v,
            "hud_recom_hp_kh1_place" => self.recom_hp_place = v,
            "hud_kh1_mp_ring" => self.mp_ring = v,
            "hud_kh1_menu" => self.menu = v,
            "hud_kh1_party" => self.party = v,
            "hud_hide_recom_hp" => self.hide_hp = v,
            "hud_hide_minimap" => self.hide_minimap = v,
            "hud_hide_friends" => self.hide_friends = v,
            "hud_hide_prizes" => self.hide_prizes = v,
            "hud_room_plate_top_left" | "hud_room_plate_top_right" => self.room_plate_top = v,
            _ => return false,
        }
        true
    }
}

/// The languages the asset step writes KH1's menu words in.
pub const MENU_LANGUAGES: &[&str] = &["us", "uk", "fr", "gr", "it", "sp"];

/// The movement abilities the KH1 handlers read (`kh1_core::Abilities`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kh1Abilities {
    pub high_jump: bool,
    pub glide: bool,
    pub superglide: bool,
    pub dodge_roll: bool,
    pub guard: bool,
    pub counterattack: bool,
    /// Copies of Combo Plus / Air Combo Plus (each adds a swing to the combo), and Combo Master.
    pub combo_plus: u8,
    pub air_combo_plus: u8,
    pub combo_master: bool,
}

impl Default for Kh1Abilities {
    fn default() -> Self {
        Kh1Abilities { high_jump: false, glide: false, superglide: false, dodge_roll: true, guard: true, counterattack: true, combo_plus: 0, air_combo_plus: 0, combo_master: false }
    }
}

impl Kh1Abilities {
    pub fn to_core(self) -> kh1_core::Abilities {
        kh1_core::Abilities {
            high_jump: self.high_jump,
            glide: self.glide,
            superglide: self.superglide,
            dodge_roll: self.dodge_roll,
            guard: self.guard,
            counterattack: self.counterattack,
            combo_plus: self.combo_plus,
            air_combo_plus: self.air_combo_plus,
            combo_master: self.combo_master,
            ..Default::default()
        }
    }

    /// KH1's ability bits (`0x142D5EC10`), as the trace writes them.
    pub fn bits(self) -> u32 {
        (self.high_jump as u32) << 9 | (self.glide as u32) << 7 | (self.superglide as u32) << 8 | (self.guard as u32) << 4 | (self.dodge_roll as u32) << 5 | (self.counterattack as u32) << 2
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings { combat: Mode::Kh1, guard_reactions: true, enemy_pace: Mode::Recom, enemy_ai: Mode::Kh1, damage: Mode::Recom, boss_revenge: true, effects: Mode::Kh1, sounds: Mode::Kh1, camera: Camera::Recom, auto_camera: true, config_menu: true, asset_build: true, abilities: Kh1Abilities::default(), dodge_roll: RollMode::Recom, run_speed: Mode::Recom, jump_height: Mode::Kh1, air_speed: Mode::Kh1, debug_attack: false, riku_kh1_movement: false, kh1_all_spells: false, kh1_all_abilities: false, kh1_test_items: false, menu_language: "us".into(), hud: Mode::Kh1, hud_parts: HudParts::default() }
    }
}

pub const DEFAULT_FILE: &str = "\
; KH1 combat in Re:Chain of Memories -- settings
; ini_version: the file's layout (the mod updates older files once; leave it as it is).
ini_version = 2
; The camp menu's KH1 entry changes these and rewrites this file: its Combat page (combat,
; guard_reactions, enemy_pace, enemy_ai, boss_revenge) and its Movement page (run_speed,
; jump_height, air_speed, dodge_roll). The rest are set here only.
;
; combat = kh1 | recom
;   kh1:   Sora fights as in KH1 (no cards in battle).
;   recom: Sora fights with Re:CoM's cards, as in the original game.
;   kh1 moves Sora with KH1's movement (walk, run, jump, glide, Dodge Roll, Guard).
combat = kh1
;
; guard_reactions = on | off (only at combat = kh1)
;   on:  when Sora's Guard blocks an enemy, the attacker flinches (Re:CoM's own flinch), and a
;        blocked shot flies back and can hurt enemies (Re:CoM's damage).
;   off: a blocked enemy carries on; a blocked shot just vanishes.
guard_reactions = on
;
; enemy_pace = recom | kh1 (only at combat = kh1)
;   recom: a crowd of enemies shares one attack budget, as in Re:CoM (four enemies each attack
;          about four times less often than one alone).
;   kh1:   each enemy attacks as often as when it is alone.
enemy_pace = recom
;
; enemy_ai = kh1 | recom (only at combat = kh1, in Sora's story; read at each battle start)
;   kh1:   enemies with a KH1 version built so far (the Shadow) think, attack, react and move as
;          in KH1; their HP, attack, defence, drops and EXP stay Re:CoM's.
;   recom: Re:CoM's enemy behaviour.
enemy_ai = kh1
;
; damage = recom | kh1 (only at combat = kh1; read at start)
;   recom: Re:CoM's enemy HP and Re:CoM's damage: each KH1 swing does what Re:CoM's card of the
;          equipped Keyblade does (normal swings its normal power, finishers its finisher power),
;          Fire / Blizzard / Thunder their Re:CoM card's or sleight's, Sonic Blade Re:CoM's.
;   kh1:   KH1's damage formula and KH1's HP for the enemies (each Re:CoM enemy's KH1 twin).
damage = recom
;
; boss_revenge = on | off (only at combat = kh1, in Sora's story; the row Revenge Value)
;   on:  humanoid bosses count Sora's hits as KH2's revenge value (Axel 92, Vexen 150, Hades 80
;        then 50, the rest 100) and, past it, break out of their flinch with an attack of their own.
;   off: bosses keep Re:CoM's rules.
boss_revenge = on
;
; effects = kh1 | recom (only at combat = kh1; read at each of Sora's motions; also the camp's
;   Settings > Effects & Sound page)
;   kh1:   KH1's Keyblade trail (KH1's frames) and finisher effects on Sora's KH1 moves.
;   recom: Re:CoM's.
effects = kh1
;
; sounds = kh1 | recom (only at combat = kh1; read at each of Sora's motions; also the camp's
;   Settings > Effects & Sound page)
;   kh1:   KH1's swing and finisher sounds on Sora's KH1 moves (Re:CoM's steps and voices stay).
;   recom: Re:CoM's.
sounds = kh1
;
; camera = recom | kh1 | kh2 (any combat setting, Sora's story; also the camp's Settings > Camera
;   page)
;   recom: Re:CoM's camera.
;   kh1:   KH1's camera: its follow (look point, distance 4 m, pitch 10 degrees, FOV 53), its
;          right-stick turning and R3 recenter. Sleights, summons and boss cameras stay Re:CoM's.
;   kh2:   KH2's camera (its distances x 0.89 for KH1's Sora): distance 3.7 m on its height
;          curve, right stick turns (left / right) and zooms (up / down), R3 resets behind Sora.
camera = recom
;
; auto_camera = on | off (camera = kh1 or kh2; the Camera page's Auto camera row)
;   on:  KH1's Auto camera: it swings behind Sora while he runs, turns along walls, keeps him in
;        sight and recenters when squeezed. KH2's: it swings behind Sora while he runs.
;   off: KH1's / KH2's Manual camera (the stick and R3 only).
auto_camera = on
;
; config_menu = on | off
;   off: no KH1 entry in the camp menu: Re:CoM's camp menu untouched (use this file instead).
config_menu = on
;
; asset_build = auto | off
;   auto: at start, when the KH1 files the mod makes from your own KH1 and Re:CoM files are
;         missing or out of date (first launch, an update of the mod, a game update), the mod
;         builds them and puts them into OpenKH's mod folder; Re:CoM's title shows the
;         progress and waits until it is done.
;   off:  never: the files are the ones OpenKH's Build put there.
asset_build = auto
;
; KH1 abilities at combat = kh1 (on | off). Since M9 9b they live in the save and are equipped
; in the camp's KH1 > Abilities page with KH1's AP. Sora learns them in the game: Guard and
; Dodge Roll (equipped) from the start; High Jump, Glide and Superglide when Re:CoM gives Sora
; its own; Combo Master at level 50; Counterattack 57, Air Combo Plus 24 and 45, Combo Plus 87,
; 93, 99 (KH1's levels).
; - high_jump, glide, superglide, combo_master: no longer unlock anything; only
;   used while no save is loaded. kh1_all_abilities (below) is the testing switch.
; - guard, counterattack, combo_plus, air_combo_plus: only read once per save, when a save from
;   before 9b first meets the ability page: what they had on stays equipped if learned.
high_jump = off
glide = off
superglide = off
;
; dodge_roll = recom | kh1 | off (only at combat = kh1)
;   Square while running rolls (KH1's rule: slower, or standing, guards).
;   recom: Re:CoM's own roll (its motion, speed, distance, timing and invulnerable frames).
;   kh1:   KH1's roll (KH1's frames; damage still taken, no flinch for 12 frames).
;   off:   no Dodge Roll (Square always guards).
;   (on, from older files, reads as recom.)
dodge_roll = recom
;
; Sora's movement speeds at combat = kh1 (recom | kh1 each):
; run_speed:   recom: the full run at Re:CoM's speed (6.2 m/s, scaled by the stick as in Re:CoM).
;              kh1:   KH1's run (4.8 m/s). The walk is KH1's either way.
run_speed = recom
; jump_height: recom: Re:CoM's jump (its take-off speed, gravity and early release; about 2.0 m,
;                     2.7 m with High Jump).
;              kh1:   KH1's jump (1.9 m, 2.9 m with High Jump).
jump_height = kh1
; air_speed:   recom: Re:CoM's speed and steering in the air (0.8 of the ground speed, eased).
;              kh1:   KH1's.
air_speed = kh1
guard = on
counterattack = on
; combo_plus / air_combo_plus: how many were equipped (0-9); combo_master = on | off
combo_plus = 0
air_combo_plus = 0
combo_master = off
;
; debug_attack = on | off: log the Attack's decisions (first 200 lines) in kh1-recom.log
debug_attack = off
;
; riku_kh1_movement = on | off (testing; only at combat = kh1): Riku in Reverse/Rebirth moves
;   with KH1's movement (run, jump, air control, landing) on his own Re:CoM animations. His
;   cards, Dark Mode and roll stay Re:CoM's. Off: Riku's story is Re:CoM's.
riku_kh1_movement = off
;
; kh1_all_spells = on | off (debug, until spells are earned in the game): Sora has all 7 KH1
;   spells at level 3 in the command menu's Magic list.
kh1_all_spells = off
;
; kh1_all_abilities = on | off (debug): Sora has every KH1 ability of the Abilities page
;   equipped (the combo modifiers, Guard, Counterattack, Dodge Roll, High Jump, Glide,
;   Superglide, Combo Master, 3 Combo Plus, 2 Air Combo Plus), AP ignored. Not written to the save.
kh1_all_abilities = off
;
; kh1_test_items = on | off (debug, until items come from the shop and drops): whenever Sora's
;   item slots are all empty as a room or battle starts, they are filled with Potion, Hi-Potion
;   and Ether, and the stock keeps at least 9 of each item. Written to the save.
kh1_test_items = off
;
; menu_language = us | uk | fr | gr | it | sp: the language of the command menu's KH1 words.
menu_language = us
;
; hud = recom | kh1 (needs the asset step's kh1-hud.json)
;   recom: Re:CoM's HUD.
;   kh1:   KH1's HUD elements, drawn by Re:CoM (stage by stage; the rest stays Re:CoM's), only at
;          combat = kh1 (combat = recom always shows Re:CoM's HUD).
hud = kh1
;
; At hud = kh1, each piece on or off:
;   hud_kh1_gauge: KH1's own HP / MP rings and face (off: Re:CoM's HP gauge, see the next two)
;   hud_recom_hp_kh1_place: with hud_kh1_gauge off, Re:CoM's HP gauge bottom right where KH1's
;     is, its ring and bar turned 180 degrees as KH1's, Sora's face upright (off: top left)
;   hud_kh1_mp_ring: with hud_kh1_gauge off, hud_recom_hp_kh1_place on and combat = kh1, KH1's
;     MP ring around Re:CoM's HP ring (KH1's MP, charge and fades; past 9 MP KH1's straight
;     segments along the bottom) (off: the Re:CoM-style MP bar under the HP bar)
;   hud_kh1_menu:  KH1's command menu look and green Triangle tab (off: the Re:CoM-styled menu)
;   hud_kh1_party: Donald's and Goofy's KH1 gauges (shown with hud_kh1_gauge)
;   hud_hide_recom_hp / hud_hide_minimap / hud_hide_friends / hud_hide_prizes: hide Re:CoM's
;     HP gauge / minimap / FRIENDS row / EXP, NEXT LV and Moogle point pop-ups
;   hud_room_plate_top_left: with hud_recom_hp_kh1_place on, Re:CoM's room plate (the room card
;     and its name) top left, off the gauge (off: bottom right)
hud_kh1_gauge = off
hud_recom_hp_kh1_place = on
hud_kh1_mp_ring = on
hud_kh1_menu = on
hud_kh1_party = on
hud_hide_recom_hp = off
hud_hide_minimap = off
hud_hide_friends = on
hud_hide_prizes = off
hud_room_plate_top_left = on
";

/// Parses the file. Unknown keys and bad values are reported and the default kept.
pub fn parse(text: &str) -> (Settings, Vec<String>) {
    let mut s = Settings::default();
    let mut warnings = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.split(';').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            warnings.push(format!("line {}: expected key = value", n + 1));
            continue;
        };
        let bad = |what: &str| format!("line {}: {what}", n + 1);
        match k.trim().to_ascii_lowercase().as_str() {
            "combat" => match Mode::parse(v) {
                Some(m) => s.combat = m,
                None => warnings.push(bad("combat must be kh1 or recom")),
            },
            // The "KH1 motions test" switch was removed (owner, 2026-10-30): an old line is
            // ignored without a warning.
            "kh1_motions" => {}
            "guard_reactions" => match parse_switch(v) {
                Some(b) => s.guard_reactions = b,
                None => warnings.push(bad("guard_reactions must be on or off")),
            },
            "enemy_pace" => match Mode::parse(v) {
                Some(m) => s.enemy_pace = m,
                None => warnings.push(bad("enemy_pace must be kh1 or recom")),
            },
            "damage" => match Mode::parse(v) {
                Some(m) => s.damage = m,
                None => warnings.push(bad("damage must be recom or kh1")),
            },
            "enemy_ai" => match Mode::parse(v) {
                Some(m) => s.enemy_ai = m,
                None => warnings.push(bad("enemy_ai must be kh1 or recom")),
            },
            "boss_revenge" => match parse_switch(v) {
                Some(b) => s.boss_revenge = b,
                None => warnings.push(bad("boss_revenge must be on or off")),
            },
            "effects" => match Mode::parse(v) {
                Some(m) => s.effects = m,
                None => warnings.push(bad("effects must be kh1 or recom")),
            },
            "camera" => match Camera::parse(v) {
                Some(c) => s.camera = c,
                None => warnings.push(bad("camera must be recom, kh1 or kh2")),
            },
            "auto_camera" => match parse_switch(v) {
                Some(b) => s.auto_camera = b,
                None => warnings.push(bad("auto_camera must be on or off")),
            },
            "sounds" => match Mode::parse(v) {
                Some(m) => s.sounds = m,
                None => warnings.push(bad("sounds must be kh1 or recom")),
            },
            "ini_version" => {}
            "asset_build" => match v.trim().to_ascii_lowercase().as_str() {
                "auto" => s.asset_build = true,
                x => match parse_switch(x) {
                    Some(b) => s.asset_build = b,
                    None => warnings.push(bad("asset_build must be auto or off")),
                },
            },
            "config_menu" => match parse_switch(v) {
                Some(b) => s.config_menu = b,
                None => warnings.push(bad("config_menu must be on or off")),
            },
            "dodge_roll" => match RollMode::parse(v) {
                Some(m) => {
                    s.dodge_roll = m;
                    s.abilities.dodge_roll = m.on();
                }
                None => warnings.push(bad("dodge_roll must be recom, kh1 or off (on = recom)")),
            },
            k @ ("run_speed" | "jump_height" | "air_speed") => match Mode::parse(v) {
                Some(m) => {
                    *match k {
                        "run_speed" => &mut s.run_speed,
                        "jump_height" => &mut s.jump_height,
                        _ => &mut s.air_speed,
                    } = m
                }
                None => warnings.push(bad(&format!("{k} must be recom or kh1"))),
            },
            k @ ("high_jump" | "glide" | "superglide" | "guard" | "counterattack") => match parse_switch(v) {
                Some(b) => {
                    let a = &mut s.abilities;
                    *match k {
                        "high_jump" => &mut a.high_jump,
                        "glide" => &mut a.glide,
                        "superglide" => &mut a.superglide,
                        "counterattack" => &mut a.counterattack,
                        _ => &mut a.guard,
                    } = b;
                }
                None => warnings.push(bad(&format!("{k} must be on or off"))),
            },
            k @ ("combo_plus" | "air_combo_plus") => match v.trim().parse::<u8>() {
                Ok(n) if n <= 9 => {
                    if k == "combo_plus" {
                        s.abilities.combo_plus = n;
                    } else {
                        s.abilities.air_combo_plus = n;
                    }
                }
                _ => warnings.push(bad(&format!("{k} must be a number from 0 to 9"))),
            },
            "riku_kh1_movement" => match parse_switch(v) {
                Some(b) => s.riku_kh1_movement = b,
                None => warnings.push(bad("riku_kh1_movement must be on or off")),
            },
            "debug_attack" => match parse_switch(v) {
                Some(b) => s.debug_attack = b,
                None => warnings.push(bad("debug_attack must be on or off")),
            },
            "kh1_all_spells" => match parse_switch(v) {
                Some(b) => s.kh1_all_spells = b,
                None => warnings.push(bad("kh1_all_spells must be on or off")),
            },
            "kh1_all_abilities" => match parse_switch(v) {
                Some(b) => s.kh1_all_abilities = b,
                None => warnings.push(bad("kh1_all_abilities must be on or off")),
            },
            "kh1_test_items" => match parse_switch(v) {
                Some(b) => s.kh1_test_items = b,
                None => warnings.push(bad("kh1_test_items must be on or off")),
            },
            "menu_language" => {
                let l = v.trim().to_ascii_lowercase();
                if MENU_LANGUAGES.contains(&l.as_str()) {
                    s.menu_language = l;
                } else {
                    warnings.push(bad("menu_language must be us, uk, fr, gr, it or sp"));
                }
            }
            "hud" => match Mode::parse(v) {
                Some(m) => s.hud = m,
                None => warnings.push(bad("hud must be recom or kh1")),
            },
            k if k.starts_with("hud_") => match parse_switch(v) {
                Some(b) if s.hud_parts.set(k, b) => {}
                Some(_) => warnings.push(bad(&format!("unknown key `{k}`"))),
                None => warnings.push(bad(&format!("{k} must be on or off"))),
            },
            "combo_master" => match parse_switch(v) {
                Some(b) => s.abilities.combo_master = b,
                None => warnings.push(bad("combo_master must be on or off")),
            },
            other => warnings.push(bad(&format!("unknown key `{other}`"))),
        }
    }
    (s, warnings)
}

/// The ini layout this DLL writes (`ini_version`).
pub const INI_VERSION: u32 = 2;

/// Brings an older settings file up to [`INI_VERSION`], once: the new text and what changed, or
/// `None` when it is current. Version 2 (0.143.1, owner 2026-11-05: testers saw Re:CoM's HUD at
/// combat = kh1): `hud` defaulted to `recom` before and no menu shows it, so an older file's
/// `hud = recom` (or a missing `hud`) becomes `kh1`, the HUD the mod is meant to show.
pub fn migrate(text: &str) -> Option<(String, Vec<String>)> {
    let version = text
        .lines()
        .filter_map(|l| l.split(';').next()?.split_once('='))
        .find(|(k, _)| k.trim().eq_ignore_ascii_case("ini_version"))
        .and_then(|(_, v)| v.trim().parse::<u32>().ok())
        .unwrap_or(1);
    if version >= INI_VERSION {
        return None;
    }
    let mut notes = Vec::new();
    let mut t = text.to_string();
    if parse(text).0.hud != Mode::Kh1 {
        t = set_key(&t, "hud", "kh1");
        notes.push("hud = recom -> kh1 (the KH1 HUD at combat = kh1)".to_string());
    }
    t = set_key(&t, "ini_version", &INI_VERSION.to_string());
    notes.push(format!("ini_version {version} -> {INI_VERSION}"));
    Some((t, notes))
}

/// Sets `key = value` in the file text, keeping every other line (and the comment after the
/// value). A missing key is appended.
pub fn set_key(text: &str, key: &str, value: &str) -> String {
    let mut out = String::with_capacity(text.len() + 32);
    let mut found = false;
    for line in text.lines() {
        let (body, comment) = match line.find(';') {
            Some(i) => (&line[..i], &line[i..]),
            None => (line, ""),
        };
        let is_key = body.split_once('=').is_some_and(|(k, _)| k.trim().eq_ignore_ascii_case(key));
        if is_key && !found {
            found = true;
            out.push_str(key);
            out.push_str(" = ");
            out.push_str(value);
            if !comment.is_empty() {
                out.push(' ');
                out.push_str(comment);
            }
        } else if is_key {
            // A repeated key would override ours on the next read: comment it out.
            out.push_str("; ");
            out.push_str(line);
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    if !found {
        out.push_str(key);
        out.push_str(" = ");
        out.push_str(value);
        out.push('\n');
    }
    out
}

/// Writes every setting into the file text.
pub fn store(text: &str, s: &Settings) -> String {
    let t = set_key(text, "combat", s.combat.as_str());
    let t = set_key(&t, "guard_reactions", switch_str(s.guard_reactions));
    let t = set_key(&t, "enemy_pace", s.enemy_pace.as_str());
    let t = set_key(&t, "enemy_ai", s.enemy_ai.as_str());
    let t = set_key(&t, "damage", s.damage.as_str());
    let t = set_key(&t, "boss_revenge", switch_str(s.boss_revenge));
    let t = set_key(&t, "effects", s.effects.as_str());
    let t = set_key(&t, "sounds", s.sounds.as_str());
    let t = set_key(&t, "camera", s.camera.as_str());
    let t = set_key(&t, "auto_camera", switch_str(s.auto_camera));
    let t = set_key(&t, "config_menu", switch_str(s.config_menu));
    let mut t = set_key(&t, "asset_build", if s.asset_build { "auto" } else { "off" });
    let a = s.abilities;
    for (k, v) in [("high_jump", a.high_jump), ("glide", a.glide), ("superglide", a.superglide), ("guard", a.guard), ("counterattack", a.counterattack)] {
        t = set_key(&t, k, switch_str(v));
    }
    let t = set_key(&t, "dodge_roll", s.dodge_roll.as_str());
    let t = set_key(&t, "run_speed", s.run_speed.as_str());
    let t = set_key(&t, "jump_height", s.jump_height.as_str());
    let t = set_key(&t, "air_speed", s.air_speed.as_str());
    let t = set_key(&t, "combo_plus", &a.combo_plus.to_string());
    let t = set_key(&t, "air_combo_plus", &a.air_combo_plus.to_string());
    let t = set_key(&t, "combo_master", switch_str(a.combo_master));
    let t = set_key(&t, "debug_attack", switch_str(s.debug_attack));
    let t = set_key(&t, "kh1_all_spells", switch_str(s.kh1_all_spells));
    let t = set_key(&t, "kh1_all_abilities", switch_str(s.kh1_all_abilities));
    let t = set_key(&t, "kh1_test_items", switch_str(s.kh1_test_items));
    let t = set_key(&t, "menu_language", &s.menu_language);
    let mut t = set_key(&t, "hud", s.hud.as_str());
    for (k, v) in s.hud_parts.keys() {
        t = set_key(&t, k, switch_str(v));
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_file_parses_to_defaults() {
        let (s, w) = parse(DEFAULT_FILE);
        assert_eq!(s, Settings::default());
        assert!(w.is_empty(), "{w:?}");
    }

    #[test]
    fn reads_values_and_reports_bad_lines() {
        let (s, w) = parse("combat = ReCoM ; comment\nfoo = 1\nnonsense\ncombat=kh2\nconfig_menu=maybe\n");
        assert_eq!(s.combat, Mode::Recom);
        assert!(s.config_menu);
        assert_eq!(w.len(), 4);
    }

    #[test]
    fn old_motions_line_is_ignored_silently() {
        let old = "combat = recom\n;\n; kh1_motions = off | on\nkh1_motions = on\nKH1_Motions = nonsense\nconfig_menu = off\n";
        let (s, w) = parse(old);
        assert_eq!(s, Settings { combat: Mode::Recom, config_menu: false, ..Default::default() });
        assert!(w.is_empty(), "{w:?}");
        // Rewriting keeps the old line (harmless) and still reads back clean.
        let t = store(old, &Settings { combat: Mode::Kh1, config_menu: false, ..Default::default() });
        assert!(t.contains("kh1_motions = on"));
        let (back, w) = parse(&t);
        assert_eq!(back.combat, Mode::Kh1);
        assert!(w.is_empty(), "{w:?}");
    }

    #[test]
    fn abilities_read_and_map_to_kh1_bits() {
        let (s, w) = parse("high_jump = on\nglide = yes\nsuperglide = 1\ndodge_roll = off\nguard = no\nglide = maybe\n");
        assert_eq!(w.len(), 1, "{w:?}");
        assert_eq!(s.abilities, Kh1Abilities { high_jump: true, glide: true, superglide: true, dodge_roll: false, guard: false, ..Default::default() });
        assert_eq!(s.abilities.bits(), 0x200 | 0x80 | 0x100 | 0x4);
        assert_eq!(Kh1Abilities::default().bits(), 0x10 | 0x20 | 0x4);
        let (off, _) = parse("counterattack = off\n");
        assert!(!off.abilities.counterattack && !off.abilities.to_core().counterattack);
        let c = s.abilities.to_core();
        assert!(c.high_jump && c.glide && c.superglide && !c.dodge_roll && !c.guard);
    }

    /// Owner (2026-11-02): Re:CoM's whole roll by default; `kh1` keeps KH1's; the old on / off read
    /// as recom / off.
    #[test]
    fn movement_speeds_read_and_round_trip() {
        let d = Settings::default();
        assert_eq!((d.run_speed, d.jump_height, d.air_speed), (Mode::Recom, Mode::Kh1, Mode::Kh1));
        let (s, w) = parse("run_speed = KH1\njump_height = recom\nair_speed = recom\n");
        assert!(w.is_empty(), "{w:?}");
        assert_eq!((s.run_speed, s.jump_height, s.air_speed), (Mode::Kh1, Mode::Recom, Mode::Recom));
        let (s, w) = parse("run_speed = fast\njump_height = 2\nair_speed = on\n");
        assert_eq!(w.len(), 3, "{w:?}");
        assert_eq!(s, Settings::default());
        // Every combination survives the file.
        for r in [Mode::Recom, Mode::Kh1] {
            for j in [Mode::Recom, Mode::Kh1] {
                for a in [Mode::Recom, Mode::Kh1] {
                    let want = Settings { run_speed: r, jump_height: j, air_speed: a, ..Default::default() };
                    let (back, w) = parse(&store(DEFAULT_FILE, &want));
                    assert!(w.is_empty(), "{w:?}");
                    assert_eq!(back, want);
                }
            }
        }
        // An older file without the keys gets them appended.
        let (back, _) = parse(&store("combat = kh1\n", &Settings { air_speed: Mode::Recom, ..Default::default() }));
        assert_eq!(back.air_speed, Mode::Recom);
    }

    #[test]
    fn dodge_roll_reads_recom_kh1_off_and_the_old_switch() {
        assert_eq!(Settings::default().dodge_roll, RollMode::Recom);
        assert!(Settings::default().abilities.dodge_roll);
        for (v, m) in [("recom", RollMode::Recom), ("ReCoM", RollMode::Recom), ("kh1", RollMode::Kh1), ("off", RollMode::Off), ("on", RollMode::Recom), ("yes", RollMode::Recom), ("no", RollMode::Off)] {
            let (s, w) = parse(&format!("dodge_roll = {v}\n"));
            assert!(w.is_empty(), "{v}: {w:?}");
            assert_eq!(s.dodge_roll, m, "{v}");
            assert_eq!(s.abilities.dodge_roll, m != RollMode::Off, "{v}");
        }
        let (s, w) = parse("dodge_roll = sideways\n");
        assert_eq!((s.dodge_roll, w.len()), (RollMode::Recom, 1));
        // Written back as the mode, and read back the same.
        for m in [RollMode::Recom, RollMode::Kh1, RollMode::Off] {
            let t = store("dodge_roll = on\n", &Settings { dodge_roll: m, abilities: Kh1Abilities { dodge_roll: m.on(), ..Default::default() }, ..Default::default() });
            assert!(t.contains(&format!("dodge_roll = {}", m.as_str())), "{t}");
            assert_eq!(parse(&t).0.dodge_roll, m);
        }
        // The default file names the new values.
        assert!(DEFAULT_FILE.contains("\ndodge_roll = recom\n"));
    }

    #[test]
    fn an_old_file_gets_the_kh1_hud_once() {
        let old = "combat = kh1\nhud = recom\n";
        let (t, notes) = migrate(old).expect("an old file is updated");
        assert_eq!(parse(&t).0.hud, Mode::Kh1);
        assert!(notes.iter().any(|n| n.contains("hud")));
        assert!(migrate(&t).is_none(), "only once");
        // A current file that chose recom keeps it.
        assert!(migrate("ini_version = 2\nhud = recom\n").is_none());
        // The default file is current and shows the KH1 HUD.
        assert!(migrate(DEFAULT_FILE).is_none());
        assert_eq!(parse(DEFAULT_FILE).0.hud, Mode::Kh1);
        assert!(parse(DEFAULT_FILE).1.is_empty());
    }

    #[test]
    fn set_key_keeps_the_rest() {
        let t = set_key("; head\ncombat = kh1 ; why\nother = 1\n", "combat", "recom");
        assert_eq!(t, "; head\ncombat = recom ; why\nother = 1\n");
        let t = set_key("combat=kh1\n", "config_menu", "on");
        assert_eq!(t, "combat=kh1\nconfig_menu = on\n");
        let t = set_key("combat = kh1\ncombat = recom\n", "combat", "kh1");
        assert_eq!(parse(&t).0.combat, Mode::Kh1);
    }

    #[test]
    fn store_round_trips() {
        let s = Settings { combat: Mode::Recom, guard_reactions: false, enemy_pace: Mode::Kh1, enemy_ai: Mode::Recom, damage: Mode::Kh1, boss_revenge: false, effects: Mode::Recom, sounds: Mode::Recom, camera: Camera::Kh1, auto_camera: false, config_menu: false, asset_build: false, abilities: Kh1Abilities { high_jump: true, glide: true, ..Default::default() }, dodge_roll: RollMode::Kh1, run_speed: Mode::Kh1, jump_height: Mode::Recom, air_speed: Mode::Recom, debug_attack: true, riku_kh1_movement: false, kh1_all_spells: true, kh1_all_abilities: true, kh1_test_items: true, menu_language: "fr".into(), hud: Mode::Kh1, hud_parts: HudParts { gauge: false, recom_hp_place: false, mp_ring: true, menu: true, party: false, hide_hp: false, hide_minimap: true, hide_friends: false, hide_prizes: true, room_plate_top: false } };
        let t = store(DEFAULT_FILE, &s);
        let (back, w) = parse(&t);
        assert_eq!(back, s);
        assert!(w.is_empty(), "{w:?}");
        assert!(t.contains("; combat = kh1 | recom"), "comments survive");
        // An empty file gets every key.
        assert_eq!(parse(&store("", &s)).0, s);
    }
}

#[cfg(test)]
mod guard_reaction_tests {
    use super::*;

    #[test]
    fn enemy_ai_default_kh1_and_round_trip() {
        assert_eq!(Settings::default().enemy_ai, Mode::Kh1);
        assert!(DEFAULT_FILE.contains("\nenemy_ai = kh1\n"));
        assert_eq!(parse(DEFAULT_FILE).0.enemy_ai, Mode::Kh1);
        let (s, w) = parse("enemy_ai = recom\nenemy_ai = maybe\n");
        assert_eq!(s.enemy_ai, Mode::Recom);
        assert_eq!(w.len(), 1);
        let t = store(DEFAULT_FILE, &Settings { enemy_ai: Mode::Recom, ..Default::default() });
        assert_eq!(t.lines().filter(|l| l.starts_with("enemy_ai")).count(), 1);
        assert_eq!(parse(&t).0.enemy_ai, Mode::Recom);
    }

    #[test]
    fn enemy_pace_default_recom_and_round_trip() {
        assert_eq!(Settings::default().enemy_pace, Mode::Recom);
        assert!(DEFAULT_FILE.contains("\nenemy_pace = recom\n"));
        assert_eq!(parse(DEFAULT_FILE).0.enemy_pace, Mode::Recom);
        // An older file without the key keeps the default.
        assert_eq!(parse("combat = kh1\n").0.enemy_pace, Mode::Recom);
        let (s, w) = parse("enemy_pace = kh1\nenemy_pace = fast\n");
        assert_eq!(s.enemy_pace, Mode::Kh1);
        assert_eq!(w.len(), 1);
        let t = store(DEFAULT_FILE, &Settings { enemy_pace: Mode::Kh1, ..Default::default() });
        assert_eq!(t.lines().filter(|l| l.starts_with("enemy_pace")).count(), 1);
        assert_eq!(parse(&t).0.enemy_pace, Mode::Kh1);
    }

    #[test]
    fn damage_default_recom_and_round_trip() {
        // Owner (2026-11-01): Re:CoM is the default.
        assert_eq!(Settings::default().damage, Mode::Recom);
        assert!(DEFAULT_FILE.contains("\ndamage = recom\n"));
        assert_eq!(parse(DEFAULT_FILE).0.damage, Mode::Recom);
        // An older file without the key keeps the default, without a warning.
        let (s, w) = parse("combat = kh1\n");
        assert_eq!(s.damage, Mode::Recom);
        assert!(w.is_empty());
        let (s, w) = parse("damage = KH1\ndamage = both\n");
        assert_eq!(s.damage, Mode::Kh1);
        assert_eq!(w.len(), 1);
        let t = store(DEFAULT_FILE, &Settings { damage: Mode::Kh1, ..Default::default() });
        assert_eq!(t.lines().filter(|l| l.starts_with("damage")).count(), 1);
        assert_eq!(parse(&t).0.damage, Mode::Kh1);
        // An old file gains the key when stored.
        let t = store("combat = kh1\n", &Settings::default());
        assert_eq!(parse(&t).0.damage, Mode::Recom);
        assert!(t.contains("damage = recom"));
    }

    #[test]
    fn guard_reactions_default_on_and_round_trip() {
        assert!(Settings::default().guard_reactions);
        assert!(DEFAULT_FILE.contains("\nguard_reactions = on\n"));
        // An old ini without the key keeps the default (on), without a warning.
        let (s, w) = parse("combat = kh1\nconfig_menu = on\n");
        assert!(s.guard_reactions && w.is_empty(), "{w:?}");
        let (s, w) = parse("guard_reactions = off\nguard_reactions = maybe\n");
        assert!(!s.guard_reactions);
        assert_eq!(w.len(), 1, "{w:?}");
        // Rewriting an old file appends the key; flipping it changes only that line.
        let t = store("combat = kh1\n", &Settings { guard_reactions: false, ..Default::default() });
        assert!(t.contains("guard_reactions = off"));
        assert!(!parse(&t).0.guard_reactions);
        let t2 = store(DEFAULT_FILE, &Settings { guard_reactions: false, ..Default::default() });
        assert_eq!(t2.lines().filter(|l| l.starts_with("guard_reactions")).count(), 1);
        assert!(t2.contains("; guard_reactions = on | off"), "comments survive");
        assert_eq!(parse(&t2).0, Settings { guard_reactions: false, ..Default::default() });
    }
}

#[cfg(test)]
mod menu_tests {
    use super::*;

    #[test]
    fn menu_switches_default_off_and_us() {
        let d = Settings::default();
        assert!(!d.kh1_all_spells && !d.kh1_test_items && d.menu_language == "us");
        let (s, w) = parse("kh1_all_spells = on\nmenu_language = GR\nmenu_language = jp\n");
        assert!(s.kh1_all_spells && s.menu_language == "gr");
        let (s, w2) = parse("kh1_test_items = on\n");
        assert!(s.kh1_test_items && w2.is_empty());
        assert!(!d.kh1_all_abilities);
        let (s, w3) = parse("kh1_all_abilities = on\n");
        assert!(s.kh1_all_abilities && w3.is_empty());
        assert_eq!(w.len(), 1, "{w:?}");
        // An older file without the keys keeps the defaults without a warning.
        let (s, w) = parse("combat = kh1\n");
        assert!(!s.kh1_all_spells && s.menu_language == "us" && w.is_empty());
    }
}

#[cfg(test)]
mod combo_tests {
    use super::*;

    #[test]
    fn combo_abilities_read_and_reach_kh1() {
        let (s, w) = parse("combo_plus = 2\nair_combo_plus = 1\ncombo_master = on\ncombo_plus = 12\n");
        assert_eq!(w.len(), 1, "{w:?}");
        assert_eq!((s.abilities.combo_plus, s.abilities.air_combo_plus, s.abilities.combo_master), (2, 1, true));
        let c = s.abilities.to_core();
        assert_eq!((c.combo_plus, c.air_combo_plus, c.combo_master), (2, 1, true));
        let back = parse(&store(DEFAULT_FILE, &s)).0;
        assert_eq!(back.abilities, s.abilities);
    }

    #[test]
    fn hud_parts_default_and_parse() {
        assert_eq!(parse(DEFAULT_FILE).0.hud_parts, HudParts::default());
        let (s, w) = parse("hud_kh1_gauge = on\nhud_hide_minimap = on\nhud_bogus = on\n");
        assert_eq!((s.hud_parts.gauge, s.hud_parts.hide_minimap, w.len()), (true, true, 1));
        // The owner's ini (2026-11-01) has neither of the two new keys: both on.
        let (s, w) = parse("hud = kh1\nhud_kh1_gauge = off\nhud_kh1_menu = on\nhud_kh1_party = off\nhud_hide_recom_hp = off\nhud_hide_friends = on\n");
        assert!(w.is_empty());
        assert!(!s.hud_parts.gauge && s.hud_parts.recom_hp_place && s.hud_parts.mp_ring && !s.hud_parts.hide_hp);
        let (s, _) = parse("hud_recom_hp_kh1_place = off\nhud_kh1_mp_ring = off\n");
        assert!(!s.hud_parts.recom_hp_place && !s.hud_parts.mp_ring);
        assert!(s.hud_parts.room_plate_top, "on unless the file says off");
        let (s, w) = parse("hud_room_plate_top_left = off\n");
        assert!(!s.hud_parts.room_plate_top && w.is_empty());
        // 0.40.x's key is the same switch.
        let (s, w) = parse("hud_room_plate_top_right = off\n");
        assert!(!s.hud_parts.room_plate_top && w.is_empty());
    }

    #[test]
    fn hud_default_kh1_and_round_trip() {
        // 0.143.1: the KH1 HUD is the default (owner, 2026-11-05).
        assert_eq!(parse(DEFAULT_FILE).0.hud, Mode::Kh1);
        let (s, w) = parse("hud = recom\n");
        assert!(w.is_empty());
        assert_eq!(s.hud, Mode::Recom);
        assert_eq!(parse(&store(DEFAULT_FILE, &s)).0.hud, Mode::Recom);
        assert_eq!(parse("hud = maybe\n").1.len(), 1);
    }

    #[test]
    fn asset_build_switch() {
        assert!(Settings::default().asset_build);
        assert!(DEFAULT_FILE.contains("\nasset_build = auto\n"));
        let (s, w) = parse("asset_build = off\n");
        assert!(!s.asset_build && w.is_empty());
        let (s, w) = parse("asset_build = Auto\n");
        assert!(s.asset_build && w.is_empty());
        let (s, w) = parse("asset_build = sometimes\n");
        assert!(s.asset_build && w.len() == 1, "{w:?}");
        let t = store("combat = kh1\n", &Settings { asset_build: false, ..Default::default() });
        assert!(t.contains("asset_build = off") && !parse(&t).0.asset_build);
    }
}
