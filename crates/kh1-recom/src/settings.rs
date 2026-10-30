//! `kh1-recom.ini`, next to the game. Written with the defaults when missing, read at start.
//!
//! Every setting switches between Re:CoM's behaviour and KH1's. The Config-menu toggles the
//! design calls for come later; until then this file is the switch. In 0.1.0 no system reads it.

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    /// Sora's combat: Re:CoM's cards, or KH1's.
    pub combat: Mode,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { combat: Mode::Kh1 }
    }
}

pub const DEFAULT_FILE: &str = "\
; KH1 combat in Re:Chain of Memories -- settings (read when the game starts)
;
; combat = kh1 | recom
;   kh1:   Sora fights as in KH1 (no cards in battle).
;   recom: Sora fights with Re:CoM's cards, as in the original game.
;   Version 0.1.0 only checks the game; nothing reads this yet.
combat = kh1
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
        match k.trim().to_ascii_lowercase().as_str() {
            "combat" => match Mode::parse(v) {
                Some(m) => s.combat = m,
                None => warnings.push(format!("line {}: combat must be kh1 or recom", n + 1)),
            },
            other => warnings.push(format!("line {}: unknown key `{other}`", n + 1)),
        }
    }
    (s, warnings)
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
    fn reads_recom_and_reports_bad_lines() {
        let (s, w) = parse("combat = ReCoM ; comment\nfoo = 1\nnonsense\ncombat=kh2\n");
        assert_eq!(s.combat, Mode::Recom);
        assert_eq!(w.len(), 3);
    }
}
