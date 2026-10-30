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
    /// Our rows in Re:CoM's Config menu. `off` leaves the menu untouched (a safety switch while
    /// the rows are untested in game; the settings then come from this file only).
    pub config_menu: bool,
    /// KH1 abilities Sora has at `combat = kh1`, until abilities are built into Re:CoM's
    /// progression (roadmap M9).
    pub abilities: Kh1Abilities,
}

/// The movement abilities the KH1 handlers read (`kh1_core::Abilities`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kh1Abilities {
    pub high_jump: bool,
    pub glide: bool,
    pub superglide: bool,
    pub dodge_roll: bool,
    pub guard: bool,
}

impl Default for Kh1Abilities {
    fn default() -> Self {
        Kh1Abilities { high_jump: false, glide: false, superglide: false, dodge_roll: true, guard: true }
    }
}

impl Kh1Abilities {
    pub fn to_core(self) -> kh1_core::Abilities {
        kh1_core::Abilities { high_jump: self.high_jump, glide: self.glide, superglide: self.superglide, dodge_roll: self.dodge_roll, guard: self.guard, ..Default::default() }
    }

    /// KH1's ability bits (`0x142D5EC10`), as the trace writes them.
    pub fn bits(self) -> u32 {
        (self.high_jump as u32) << 9 | (self.glide as u32) << 7 | (self.superglide as u32) << 8 | (self.guard as u32) << 4 | (self.dodge_roll as u32) << 5
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings { combat: Mode::Kh1, config_menu: true, abilities: Kh1Abilities::default() }
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
            "config_menu" => match parse_switch(v) {
                Some(b) => s.config_menu = b,
                None => warnings.push(bad("config_menu must be on or off")),
            },
            k @ ("high_jump" | "glide" | "superglide" | "dodge_roll" | "guard") => match parse_switch(v) {
                Some(b) => {
                    let a = &mut s.abilities;
                    *match k {
                        "high_jump" => &mut a.high_jump,
                        "glide" => &mut a.glide,
                        "superglide" => &mut a.superglide,
                        "dodge_roll" => &mut a.dodge_roll,
                        _ => &mut a.guard,
                    } = b;
                }
                None => warnings.push(bad(&format!("{k} must be on or off"))),
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
    let mut t = set_key(&t, "config_menu", switch_str(s.config_menu));
    let a = s.abilities;
    for (k, v) in [("high_jump", a.high_jump), ("glide", a.glide), ("superglide", a.superglide), ("dodge_roll", a.dodge_roll), ("guard", a.guard)] {
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
        assert_eq!(s.abilities, Kh1Abilities { high_jump: true, glide: true, superglide: true, dodge_roll: false, guard: false });
        assert_eq!(s.abilities.bits(), 0x200 | 0x80 | 0x100);
        assert_eq!(Kh1Abilities::default().bits(), 0x10 | 0x20);
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
        let s = Settings { combat: Mode::Recom, config_menu: false, abilities: Kh1Abilities { high_jump: true, glide: true, ..Default::default() } };
        let t = store(DEFAULT_FILE, &s);
        let (back, w) = parse(&t);
        assert_eq!(back, s);
        assert!(w.is_empty(), "{w:?}");
        assert!(t.contains("; combat = kh1 | recom"), "comments survive");
        // An empty file gets every key.
        assert_eq!(parse(&store("", &s)).0, s);
    }
}
