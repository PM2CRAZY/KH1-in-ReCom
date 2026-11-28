//! Riku's Dark Points at Combat = KH1 (fork branch Re-Re-Reverse/Rebirth, 0.151.0; fork owner,
//! 2026-11-22: the gain / loss tables, "go"). Re:CoM's own DP come from card breaks, which an empty
//! revolver never gives; these rules replace them in Riku's story with `riku_kh1_movement`. The
//! testable part; the game side is `win/dark_points.rs`, notes `docs/recom-riku-kh1.md` § 0.151.0.
//!
//! - **Outside Dark Mode** (fills to [`MAX`]): a normal swing that lands +1, a finisher +3, a perfect
//!   parry +5, an enemy defeated +3, a hit taken +2; a missed parry −3, a critical hit taken −5
//!   (instead of the +2); no attack for [`IDLE_FIRST`] frames, then −1 every [`IDLE_EVERY_OUT`].
//! - At [`MAX`] Riku enters Dark Mode (Re:CoM's own transformation).
//! - **In Dark Mode** (the owner's second table): a normal hit taken −3, a heavy one −6, a special
//!   one −8; a hit taken mid-combo −2 more; a hit within [`DODGE_WINDOW`] frames after a roll −2
//!   more; no attack for [`IDLE_EVERY_IN`] frames −1 (each time); a finisher +2, a perfect parry +3,
//!   an enemy defeated +3 (a boss +5), [`STREAK`] landed swings without being hit +1. At 0 it ends.
//! - One hit taken counts once (one enemy attack, not each of its hits: the caller passes one
//!   [`Event::Hurt`] per damage check that took HP). DP reset at every battle start.
//!
//! The kinds of a hit (ours, the owner's terms on Re:CoM's data): **heavy / critical** = Re:CoM's
//! knock-away reaction (pending kind 2; flinch 1 is normal); **special** = a hit from a boss
//! (`CBoss…`).

/// The DP that start Dark Mode (and the most Riku holds).
pub const MAX: i32 = 30;

/// 0.179.0 (fork owner, 2026-11-23, verbatim: "o MP continua, mas os Sleights que só o Dark Riku
/// Executa, consomem DP"; answers 1a / 2a): the sleights Re:CoM's Riku can't start and Dark Riku can
/// (Riku's start `0x140216AB0` refuses 0x5F..=0x61 and 0x65, Dark Riku's `0x140206830` takes them;
/// Checked (exe + decomp)) pay Dark Mode's DP instead of MP, as a share (%) of Dark Mode's max (his DP
/// stat, [`Dp::cap`]): Dark Break and Dark Firaga 20 %, Dark Aura and Inverse Burst 30 %; at least
/// [`SLEIGHT_MIN`]. Every other sleight of his keeps its MP.
pub const DP_SLEIGHTS: [(u16, i32); 4] = [(0x5F, 20), (0x60, 20), (0x61, 30), (0x65, 30)];
/// The least DP a DP sleight costs (a low DP stat: 8 at a new game).
pub const SLEIGHT_MIN: i32 = 2;

/// Sleight `id` pays DP ([`DP_SLEIGHTS`]).
pub fn pays_dp(id: u16) -> bool {
    DP_SLEIGHTS.iter().any(|&(i, _)| i == id)
}

/// The DP sleight `id` costs with Dark Mode's max `cap` (rounded to the nearest point).
pub fn sleight_cost(id: u16, cap: i32) -> Option<i32> {
    let pct = DP_SLEIGHTS.iter().find(|&&(i, _)| i == id)?.1;
    Some(((cap.max(1) * pct + 50) / 100).max(SLEIGHT_MIN))
}

/// A DP sleight may start with `dp` held: at least 1 DP must be left after it (fork owner's answer
/// 3a: a sleight never ends Dark Mode).
pub fn can_pay(dp: i32, cost: i32) -> bool {
    dp - cost >= 1
}
/// Frames without an attack before the first idle loss (outside Dark Mode): 5 s.
pub const IDLE_FIRST: u32 = 300;
/// Then one loss every 3 s (outside Dark Mode).
pub const IDLE_EVERY_OUT: u32 = 180;
/// In Dark Mode: one loss every 5 s without an attack.
pub const IDLE_EVERY_IN: u32 = 300;
/// A hit this many frames after a roll ended is a missed dodge (0.5 s).
pub const DODGE_WINDOW: u32 = 30;
/// Landed swings in a row without being hit for the streak bonus.
pub const STREAK: u32 = 5;

/// How hard a hit taken was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    /// Re:CoM's flinch (or no reaction).
    Normal,
    /// Re:CoM's knock-away: the owner's "golpe crítico" / "ataque pesado".
    Heavy,
    /// A boss's attack: the owner's "ataque especial" (outside Dark Mode: a critical one when heavy,
    /// else a normal one).
    Special { heavy: bool },
}

/// Something that moves the DP.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// A swing of Riku's that landed (one per swing).
    Swing { finisher: bool },
    PerfectParry,
    ParryMiss,
    Defeat { boss: bool },
    /// A hit that took HP: its kind, whether Riku was swinging (his combo cut), whether a roll had
    /// ended within [`DODGE_WINDOW`] frames.
    Hurt { hit: Hit, in_combo: bool, after_roll: bool },
}

/// What the game must do after an event or a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    None,
    /// DP reached [`MAX`] outside Dark Mode: start Re:CoM's transformation.
    Enter,
    /// DP reached 0 in Dark Mode: end it.
    Exit,
}

/// One battle's DP.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dp {
    pub dp: i32,
    /// Riku is in Dark Mode (the game's state, set by the caller from Re:CoM's main player).
    pub dark: bool,
    /// Frames since his last attack.
    pub idle: u32,
    /// Landed swings since he was last hit.
    pub streak: u32,
    /// Dark Mode's DP (0.166.0; fork owner, 2026-11-23: "o DP enchendo e esvaziando tbm liga com o
    /// DP maximo oq aconteceu com ele"): Re:CoM's Dark Riku starts with Riku's DP stat (`+0x450`,
    /// his save's `stats+0x14`: 8 at a new game, + 2 a DP pick, at most 99) and holds no more
    /// (`0x1402048BD..0x1402048CE`, the gauge's max there: `0x1401E72F1`, `0x1401F5BB3`; Checked
    /// (disasm)). Outside Dark Mode the gauge fills to Re:CoM's fixed 30 ([`MAX`]). 0: [`MAX`].
    pub dark_max: i32,
}

impl Dp {
    /// The most DP held now: [`MAX`] outside Dark Mode, his DP stat in it.
    pub fn cap(&self) -> i32 {
        if self.dark && self.dark_max > 0 {
            self.dark_max
        } else {
            MAX
        }
    }

    /// The DP sleight `id` costs now: None outside Dark Mode or for an MP sleight.
    pub fn sleight_cost(&self, id: u16) -> Option<i32> {
        self.dark.then(|| sleight_cost(id, self.cap())).flatten()
    }

    /// Pays DP sleight `id` ([`can_pay`]); the points paid, or None (nothing changed).
    pub fn pay_sleight(&mut self, id: u16) -> Option<i32> {
        let c = self.sleight_cost(id)?;
        can_pay(self.dp, c).then(|| {
            self.dp -= c;
            c
        })
    }

    /// Dark Mode begins (Re:CoM's Dark Riku switched in): DP = his DP stat, as Re:CoM's switch-in.
    pub fn enter_dark(&mut self, dp_stat: i32) {
        self.dark = true;
        self.dark_max = dp_stat.clamp(1, 99);
        self.dp = self.dark_max;
    }

    fn add(&mut self, n: i32) {
        self.dp = (self.dp + n).clamp(0, self.cap());
    }

    fn change(&self) -> Change {
        if !self.dark && self.dp >= MAX {
            Change::Enter
        } else if self.dark && self.dp <= 0 {
            Change::Exit
        } else {
            Change::None
        }
    }

    /// The DP an event gives or takes, applied; what to do next.
    pub fn event(&mut self, e: Event) -> Change {
        let n = delta(e, self.dark, &mut self.streak);
        if matches!(e, Event::Swing { .. }) {
            self.idle = 0;
        }
        self.add(n);
        self.change()
    }

    /// One 60 Hz frame of battle: `attacking` = Riku's attack action this frame.
    pub fn frame(&mut self, attacking: bool) -> Change {
        if attacking {
            self.idle = 0;
            return self.change();
        }
        self.idle = self.idle.saturating_add(1);
        let lose = if self.dark {
            self.idle % IDLE_EVERY_IN == 0
        } else {
            self.idle >= IDLE_FIRST && (self.idle - IDLE_FIRST) % IDLE_EVERY_OUT == 0
        };
        if lose {
            self.add(-1);
        }
        self.change()
    }
}

/// The owner's tables: the DP an event moves, outside (`dark` false) or in Dark Mode. `streak`
/// counts landed swings since the last hit taken.
pub fn delta(e: Event, dark: bool, streak: &mut u32) -> i32 {
    match e {
        Event::Swing { finisher } => {
            *streak += 1;
            let bonus = if dark && *streak % STREAK == 0 { 1 } else { 0 };
            bonus
                + match (dark, finisher) {
                    (false, false) => 1,
                    (false, true) => 3,
                    (true, false) => 0,
                    (true, true) => 2,
                }
        }
        Event::PerfectParry => {
            if dark {
                3
            } else {
                5
            }
        }
        Event::ParryMiss => {
            if dark {
                0
            } else {
                -3
            }
        }
        Event::Defeat { boss } => match (dark, boss) {
            (true, true) => 5,
            _ => 3,
        },
        Event::Hurt { hit, in_combo, after_roll } => {
            *streak = 0;
            if dark {
                let base = match hit {
                    Hit::Normal => -3,
                    Hit::Heavy => -6,
                    Hit::Special { .. } => -8,
                };
                base - if in_combo { 2 } else { 0 } - if after_roll { 2 } else { 0 }
            } else {
                match hit {
                    Hit::Heavy | Hit::Special { heavy: true } => -5,
                    _ => 2,
                }
            }
        }
    }
}

/// The kind of a hit from Re:CoM's pending reaction kind (`player+0x710`: 2 = knock away) and
/// whether its attacker is a boss.
pub fn hit_of(reaction_kind: u16, boss: bool) -> Hit {
    let heavy = reaction_kind == 2;
    match (boss, heavy) {
        (true, h) => Hit::Special { heavy: h },
        (false, true) => Hit::Heavy,
        (false, false) => Hit::Normal,
    }
}

/// Re:CoM's attack ids of the finishers (`0x1407C6C70` rows with the finisher column 2: Sora's 4 /
/// 7, Riku's twins 0x9A / 0x9D, Dark Riku's 0xA1 / 0xA4). Checked (exe: the rows' bytes).
pub fn is_finisher(attack_id: u32) -> bool {
    matches!(attack_id, 4 | 7 | 0x9A | 0x9D | 0xA1 | 0xA4)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hurt(hit: Hit) -> Event {
        Event::Hurt { hit, in_combo: false, after_roll: false }
    }

    #[test]
    fn filling_to_dark_mode() {
        let mut d = Dp::default();
        for _ in 0..9 {
            assert_eq!(d.event(Event::Swing { finisher: false }), Change::None);
        }
        assert_eq!(d.dp, 9);
        assert_eq!(d.event(Event::Swing { finisher: true }), Change::None);
        assert_eq!(d.dp, 12);
        d.event(Event::PerfectParry);
        d.event(Event::Defeat { boss: false });
        d.event(hurt(Hit::Normal));
        assert_eq!(d.dp, 22);
        d.event(hurt(Hit::Heavy));
        d.event(Event::ParryMiss);
        assert_eq!(d.dp, 14);
        assert_eq!(d.event(hurt(Hit::Special { heavy: true })), Change::None);
        assert_eq!(d.dp, 9);
        for _ in 0..7 {
            d.event(Event::Swing { finisher: true });
        }
        assert_eq!(d.dp, MAX, "capped");
        let mut e = Dp::default();
        e.enter_dark(8);
        assert_eq!((e.dp, e.cap()), (8, 8), "Dark Mode starts at his DP stat (a new game: 8)");
        e.event(Event::Defeat { boss: false });
        assert_eq!(e.dp, 8, "never past it");
        e.enter_dark(150);
        assert_eq!(e.cap(), 99);
        assert_eq!(d.change(), Change::Enter);
    }

    #[test]
    fn dark_mode_drains_and_ends() {
        let mut d = Dp { dp: MAX, dark: true, ..Default::default() };
        assert_eq!(d.event(Event::Swing { finisher: false }), Change::None);
        assert_eq!(d.dp, MAX, "a normal swing gives nothing in Dark Mode");
        d.event(hurt(Hit::Normal));
        assert_eq!(d.dp, 27);
        d.event(Event::Hurt { hit: Hit::Heavy, in_combo: true, after_roll: true });
        assert_eq!(d.dp, 17);
        d.event(hurt(Hit::Special { heavy: false }));
        assert_eq!(d.dp, 9);
        d.event(Event::Defeat { boss: true });
        d.event(Event::Swing { finisher: true });
        d.event(Event::PerfectParry);
        assert_eq!(d.dp, 19);
        // Five landed swings in a row: +1 on the fifth.
        for _ in 0..5 {
            d.event(Event::Swing { finisher: false });
        }
        assert_eq!(d.dp, 20);
        d.event(hurt(Hit::Special { heavy: true }));
        d.event(hurt(Hit::Special { heavy: true }));
        assert_eq!(d.event(hurt(Hit::Normal)), Change::None);
        assert_eq!(d.dp, 1);
        assert_eq!(d.event(hurt(Hit::Normal)), Change::Exit);
        assert_eq!(d.dp, 0);
    }

    #[test]
    fn idle_losses() {
        let mut d = Dp { dp: 10, ..Default::default() };
        for _ in 0..IDLE_FIRST - 1 {
            d.frame(false);
        }
        assert_eq!(d.dp, 10);
        d.frame(false);
        assert_eq!(d.dp, 9, "after 5 s");
        for _ in 0..IDLE_EVERY_OUT {
            d.frame(false);
        }
        assert_eq!(d.dp, 8, "then every 3 s");
        d.frame(true);
        assert_eq!(d.idle, 0);
        let mut d = Dp { dp: 10, dark: true, ..Default::default() };
        for _ in 0..IDLE_EVERY_IN * 2 {
            d.frame(false);
        }
        assert_eq!(d.dp, 8, "every 5 s in Dark Mode");
    }

    #[test]
    fn kinds_and_finishers() {
        assert_eq!(hit_of(1, false), Hit::Normal);
        assert_eq!(hit_of(2, false), Hit::Heavy);
        assert_eq!(hit_of(2, true), Hit::Special { heavy: true });
        assert!(is_finisher(0x9D) && is_finisher(0xA1) && !is_finisher(0x97));
    }

    #[test]
    fn dark_rikus_own_sleights_pay_a_share_of_dark_modes_dp() {
        // 0.179.0: 20 % / 30 % of Dark Mode's max, at least 2, and 1 DP must be left.
        assert!(pays_dp(0x5F) && pays_dp(0x60) && pays_dp(0x61) && pays_dp(0x65));
        assert!(!pays_dp(0x69) && !pays_dp(0x66), "the sleights Riku can start too keep MP");
        assert_eq!([sleight_cost(0x5F, 8), sleight_cost(0x61, 8)], [Some(2), Some(2)], "a new game's DP stat");
        assert_eq!([sleight_cost(0x60, 30), sleight_cost(0x65, 30)], [Some(6), Some(9)]);
        assert_eq!([sleight_cost(0x5F, 99), sleight_cost(0x61, 99)], [Some(20), Some(30)]);
        let mut d = Dp::default();
        assert_eq!(d.pay_sleight(0x5F), None, "outside Dark Mode: none");
        d.enter_dark(30);
        assert_eq!(d.pay_sleight(0x61), Some(9));
        assert_eq!(d.dp, 21);
        d.dp = 9;
        assert_eq!(d.pay_sleight(0x61), None, "it would end Dark Mode");
        assert_eq!(d.dp, 9);
        d.dp = 10;
        assert_eq!(d.pay_sleight(0x61), Some(9));
        assert_eq!(d.dp, 1);
    }
}

