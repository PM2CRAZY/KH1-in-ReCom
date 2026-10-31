//! `kh1-recom.ini`, next to the game. Written with the defaults when missing, read at start.
//!
//! Every setting switches between Re:CoM's behaviour and KH1's. The rows we add to Re:CoM's
//! Config menu change these values and rewrite this file at once, so the file stays the backup
//! (and the only store: Re:CoM's own config bytes live in its save data, see
//! `docs/recom-config-menu.md`).

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
    /// Our rows in Re:CoM's Config menu. `off` leaves the menu untouched (a safety switch while
    /// the rows are untested in game; the settings then come from this file only).
    pub config_menu: bool,
    /// KH1 abilities Sora has at `combat = kh1`, until abilities are built into Re:CoM's
    /// progression (roadmap M9).
    pub abilities: Kh1Abilities,
    /// One-line logs of the Attack's decisions in `kh1-recom.log` (first 200 lines).
    pub debug_attack: bool,
}

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
        Settings { combat: Mode::Kh1, guard_reactions: true, config_menu: true, abilities: Kh1Abilities::default(), debug_attack: false }
    }
}

pub const DEFAULT_FILE: &str = "\
; KH1 combat in Re:Chain of Memories -- settings
; The Config menu (camp menu > Config) changes these and rewrites this file.
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
; config_menu = on | off
;   off: don't add our rows to Re:CoM's Config menu (use this file instead).
config_menu = on
;
; KH1 abilities at combat = kh1 (on | off), until abilities are built into the game:
high_jump = off
glide = off
superglide = off
dodge_roll = on
guard = on
counterattack = on
; Combo Plus / Air Combo Plus: how many are equipped (0-9); combo_master = on | off
combo_plus = 0
air_combo_plus = 0
combo_master = off
;
; debug_attack = on | off: log the Attack's decisions (first 200 lines) in kh1-recom.log
debug_attack = off
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
            "config_menu" => match parse_switch(v) {
                Some(b) => s.config_menu = b,
                None => warnings.push(bad("config_menu must be on or off")),
            },
            k @ ("high_jump" | "glide" | "superglide" | "dodge_roll" | "guard" | "counterattack") => match parse_switch(v) {
                Some(b) => {
                    let a = &mut s.abilities;
                    *match k {
                        "high_jump" => &mut a.high_jump,
                        "glide" => &mut a.glide,
                        "superglide" => &mut a.superglide,
                        "dodge_roll" => &mut a.dodge_roll,
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
            "debug_attack" => match parse_switch(v) {
                Some(b) => s.debug_attack = b,
                None => warnings.push(bad("debug_attack must be on or off")),
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
    let mut t = set_key(&t, "config_menu", switch_str(s.config_menu));
    let a = s.abilities;
    for (k, v) in [("high_jump", a.high_jump), ("glide", a.glide), ("superglide", a.superglide), ("dodge_roll", a.dodge_roll), ("guard", a.guard), ("counterattack", a.counterattack)] {
        t = set_key(&t, k, switch_str(v));
    }
    let t = set_key(&t, "combo_plus", &a.combo_plus.to_string());
    let t = set_key(&t, "air_combo_plus", &a.air_combo_plus.to_string());
    let t = set_key(&t, "combo_master", switch_str(a.combo_master));
    set_key(&t, "debug_attack", switch_str(s.debug_attack))
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
        let s = Settings { combat: Mode::Recom, guard_reactions: false, config_menu: false, abilities: Kh1Abilities { high_jump: true, glide: true, ..Default::default() }, debug_attack: true };
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
}
