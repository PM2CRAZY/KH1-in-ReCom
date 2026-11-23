//! The command menu's Party list (owner, 2026-11-04, `design-philosophy-instructions.md` § Command
//! menu pages and Party): Re:CoM's friend sleights, the LV3 ones where Re:CoM has levels, under the
//! owner's names, each shown only in a battle where its friend's card can drop (for the whole
//! battle) and once Re:CoM has taught it. The testable part; Re:CoM's side is `win/party.rs`, the
//! start Re:CoM's own sleight start (`win/summons.rs`); notes `docs/recom-command-pages-party.md`.
//!
//! - **The sleights** ([`ALL`], Checked (files: `SY0002.CTD`'s sleight names in id order, id 4 =
//!   Sliding Dash) + decomp): Magic LV3 0x45, Stardust Blitz 0x46, Goofy Tornado LV3 0x48, Goofy
//!   Smash 0x49, Wild Crush 0x4A, Lucky Bounty LV3 0x56, Sandstorm LV3 0x4C, Surprise! LV3 0x4E,
//!   Spiral Wave LV3 0x50, Hummingbird LV3 0x52, Furious Volley LV3 0x54. The friend each one calls
//!   is Re:CoM's `0x14020F4D0(plm, id)` (1 Donald, 2 Goofy, 3 Aladdin, 4 Ariel, 5 Jack, 6 Peter Pan,
//!   7 Beast, 9 Pluto, 10 Goofy + Donald; RTTI of the ctors `0x14020E950` makes).
//! - **Which cards can drop in a battle** ([`can_drop`]): Re:CoM's Friend-card drop roll
//!   `0x1401602A0` (Checked (decomp + exe)): a row of its table `0x1407C1F10` ([`DROP_TABLE`], 26 ×
//!   8 bytes: chance ‰, three % weights, three card kinds) picked by the world (`0x1402339C0(0)`,
//!   at most 13; room type `0x140233860` 0 or 99, Castle Oblivion's halls: row 13; Riku's story
//!   + 13); a card with a weight drops only once its friend is met (int var (0, 0x22), bit by card:
//!   Donald 1, Goofy 2, Aladdin 4, Ariel 8, Jack 0x10, Peter Pan 0x20, Beast 0x40, the King 0x80,
//!   Pluto 0x100). Pluto (0x39) has his own roll before the table, in every world (20 %, met bit
//!   0x100, Sora's story, not a minigame room `0x140235360`, fewer than 10 cards in the revolver,
//!   fewer than 3 Plutos this battle).
//! - **The extra unlock**: Re:CoM's own learned bit (`0x14023AB00(id)`): Re:CoM never forms an
//!   unlearned sleight (`docs/recom-m12-sleights.md` § 1.3).
//! - **Cost**: whole MP points ([`points`]; owner: "Same MP as sleights").

/// Our command for a Party sleight: the Sleights list's `0x80 + id` (`crate::sleights::command`):
/// 0xC5..=0xD6, free (Checked (code): KH1's commands end below 0x80, the 27 sleights' are
/// 0x9B..=0xC3, Stagger 0xF0).
pub const COMMAND_BASE: u8 = crate::sleights::COMMAND_BASE;

/// One Party entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Friend {
    /// The owner's name (the list, the shortcut panel, the Customize page).
    pub name: &'static str,
    /// Re:CoM's sleight id (its LV3 where it has levels).
    pub sleight: u16,
    /// Re:CoM's Friend cards whose drop it needs (Wild Crush: Donald's and Goofy's).
    pub cards: &'static [u8],
    /// Whole MP points it costs ([`points`]).
    pub mp: i32,
}

/// Re:CoM's Friend cards (card kinds, `0x14059F2E0` / `0x140526510` type 5). Checked (exe).
pub mod card {
    pub const DONALD: u8 = 0x31;
    pub const GOOFY: u8 = 0x32;
    pub const ALADDIN: u8 = 0x33;
    pub const ARIEL: u8 = 0x34;
    pub const JACK: u8 = 0x35;
    pub const PETER_PAN: u8 = 0x36;
    pub const BEAST: u8 = 0x37;
    pub const KING: u8 = 0x38;
    pub const PLUTO: u8 = 0x39;
}

/// The owner's list, in the owner's order.
///
/// MP (owner: "Same MP as sleights"; Assumption, ours: friend sleights have no AP tier, so each
/// takes the tier a sleight of its kind would): 2 MP (the 3- / 4-AP tier) for the attacks, 3 MP
/// (the 5-AP tier) for Wild Crush (two friends, three cards in Re:CoM), 1 MP (the 2-AP tier) for
/// Lucky Bounty (a prize finder, no damage). An owner question.
pub const ALL: [Friend; 13] = [
    Friend { name: "Donald Magic", sleight: 0x45, cards: &[card::DONALD], mp: 2 },
    Friend { name: "Stardust Blitz", sleight: 0x46, cards: &[card::DONALD], mp: 2 },
    Friend { name: "Goofy Tornado", sleight: 0x48, cards: &[card::GOOFY], mp: 2 },
    Friend { name: "Goofy Smash", sleight: 0x49, cards: &[card::GOOFY], mp: 2 },
    Friend { name: "Wild Crush", sleight: 0x4A, cards: &[card::DONALD, card::GOOFY], mp: 3 },
    Friend { name: "Lucky Bounty", sleight: 0x56, cards: &[card::PLUTO], mp: 1 },
    Friend { name: "Sandstorm", sleight: 0x4C, cards: &[card::ALADDIN], mp: 2 },
    Friend { name: "Surprise!", sleight: 0x4E, cards: &[card::JACK], mp: 2 },
    Friend { name: "Spiral Wave", sleight: 0x50, cards: &[card::ARIEL], mp: 2 },
    Friend { name: "Hummingbird", sleight: 0x52, cards: &[card::PETER_PAN], mp: 2 },
    Friend { name: "Furious Volley", sleight: 0x54, cards: &[card::BEAST], mp: 2 },
    // Riku's story (fork branch Re-Re-Reverse/Rebirth, 0.150.0; fork owner, 2026-11-22: "minha
    // sugestão pra cura, igual no caso do donald e do goofy são tratados na gameplay do Sora"): the
    // King's friend sleights, which Riku's own sleight start takes (`0x140216AB0`: 0x62..=0x64 with
    // `0x14020E590`, Checked (exe)); the King's card is the only Friend card of Riku's drop rows
    // (14..25, [`DROP_TABLE`]), never in Sora's. MM Miracle's LV3 (`SY0002.CTD` "MM Miracle LV3",
    // its heal: Assumption, not read); MP ours (the heal one point dearer than the attacks).
    Friend { name: "Mickey Miracle", sleight: 0x63, cards: &[card::KING], mp: 3 },
    Friend { name: "Holy Burst", sleight: 0x64, cards: &[card::KING], mp: 2 },
];
pub const COUNT: usize = ALL.len();
const _: () = assert!(COUNT <= 15, "a Set and `win::party`'s per-battle word hold 15 entries");

/// A set of [`ALL`] entries (bit i = `ALL[i]`).
pub type Set = u16;

/// The entry of Re:CoM sleight `id`.
pub fn index(id: u16) -> Option<usize> {
    ALL.iter().position(|f| f.sleight == id)
}

/// The command of entry `i`.
pub fn command(i: usize) -> u8 {
    crate::sleights::command(ALL[i].sleight)
}

/// The entry of a command.
pub fn of_command(c: u8) -> Option<usize> {
    index(c.checked_sub(COMMAND_BASE)? as u16)
}

/// Entry `i`'s name.
pub fn name(id: u16) -> Option<&'static str> {
    index(id).map(|i| ALL[i].name)
}

/// Our words (`crate::sleights::word`: `0x8100 + id`) and names, for the menu's word list (the
/// asset step's `command_menu::PARTY_WORDS` holds the same).
pub fn words() -> impl Iterator<Item = (u16, &'static str)> {
    ALL.iter().map(|f| (crate::sleights::WORD_BASE + f.sleight, f.name))
}

/// The whole MP points entry `id` takes from `mp` (None: it can't pay).
pub fn points(id: u16, mp: i32) -> Option<i32> {
    let n = ALL[index(id)?].mp;
    (mp >= n).then_some(n)
}

/// Re:CoM's Friend-card drop table (`0x1407C1F10`, rows `0x1401602A0` reads; `sites::PARTY_CHECKED`
/// checks these bytes against the exe): chance ‰, weights % for the three slots, the three cards.
/// Row = world (0 none … 13 Castle Oblivion), + 13 in Riku's story. Checked (exe).
pub const DROP_TABLE: [[u8; 8]; 26] = [
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x96, 0x32, 0x32, 0x00, 0x31, 0x32, 0x00, 0x00],
    [0x96, 0x1E, 0x14, 0x32, 0x31, 0x32, 0x33, 0x00],
    [0x96, 0x3C, 0x28, 0x00, 0x31, 0x32, 0x00, 0x00],
    [0x96, 0x28, 0x3C, 0x00, 0x31, 0x32, 0x00, 0x00],
    [0x96, 0x32, 0x32, 0x00, 0x31, 0x32, 0x00, 0x00],
    [0x96, 0x14, 0x1E, 0x32, 0x31, 0x32, 0x35, 0x00],
    [0x96, 0x14, 0x14, 0x3C, 0x31, 0x32, 0x34, 0x00],
    [0x96, 0x14, 0x1E, 0x32, 0x31, 0x32, 0x36, 0x00],
    [0x96, 0x1E, 0x1E, 0x28, 0x31, 0x32, 0x37, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x96, 0x32, 0x32, 0x00, 0x31, 0x32, 0x00, 0x00],
    [0x96, 0x32, 0x32, 0x00, 0x31, 0x32, 0x00, 0x00],
    [0x96, 0x32, 0x32, 0x00, 0x31, 0x32, 0x00, 0x00],
    [0x7D, 0x64, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00],
    [0x7D, 0x64, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00],
    [0x7D, 0x64, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00],
    [0x7D, 0x64, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00],
    [0x7D, 0x64, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00],
    [0x7D, 0x64, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00],
    [0x7D, 0x64, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00],
    [0x7D, 0x64, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00],
    [0x7D, 0x64, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x7D, 0x64, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00],
    [0x7D, 0x64, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00],
];

/// The friends-met bit of a Friend card (int var (0, 0x22)), as `0x1401602A0` tests it.
pub fn met_bit(card: u8) -> u32 {
    match card {
        card::DONALD..=card::BEAST => 1 << (card - card::DONALD),
        card::KING => 0x80,
        card::PLUTO => 0x100,
        _ => 0,
    }
}

/// Where the battle is, as Re:CoM's drop roll reads it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Place {
    /// `0x1402339C0(0)`.
    pub world: i32,
    /// `0x140233860()`: the room-type byte (0 or 99: Castle Oblivion's halls).
    pub room_type: u32,
    pub riku: bool,
    /// `0x140235360()`.
    pub minigame: bool,
    /// Int var (0, 0x22).
    pub met: u32,
}

/// `0x1401602A0`'s row: the world (at most 13), 13 in the halls (room type 0 / 99), + 13 in Riku's
/// story.
pub fn drop_row(p: &Place) -> usize {
    let mut r = p.world.clamp(0, 13) as usize;
    if p.room_type == 0 || p.room_type == 99 {
        r = 13;
    }
    if p.riku {
        r += 13;
    }
    r
}

/// Friend card `card` can drop in a battle here (Re:CoM's roll `0x1401602A0` can give it): Pluto's
/// own roll (Sora's story, not a minigame room, met); else a slot of the place's row with a weight
/// holds the card, the row's chance is above 0 (the hit handler passes the row's), and the friend
/// is met (Donald / Goofy unmet: no drop; another friend unmet: the roll draws again).
pub fn can_drop(card: u8, p: &Place) -> bool {
    if p.met & met_bit(card) == 0 {
        return false;
    }
    if card == card::PLUTO {
        return !p.riku && !p.minigame;
    }
    let Some(row) = DROP_TABLE.get(drop_row(p)) else { return false };
    row[0] > 0 && (0..3).any(|k| row[1 + k] > 0 && row[4 + k] == card)
}

/// The entries usable in this battle's list: Re:CoM has taught them (`learned`, by entry) and
/// every card they need can drop here.
pub fn available(learned: Set, p: &Place) -> Set {
    let mut s = 0;
    for (i, f) in ALL.iter().enumerate() {
        if learned >> i & 1 != 0 && f.cards.iter().all(|&c| can_drop(c, p)) {
            s |= 1 << i;
        }
    }
    s
}

/// The list's entries (indexes into [`ALL`], the owner's order).
pub fn list(avail: Set) -> Vec<usize> {
    (0..COUNT).filter(|&i| avail >> i & 1 != 0).collect()
}

/// Entry `i` can start now: the Sleights list's gate (`crate::sleights::usable`'s ground rule:
/// none of them has an air start in Re:CoM, `initStockSkill` `0x140220DF0`), in a battle, nothing
/// running, its MP.
pub fn usable(g: &crate::summons::Gate, i: usize) -> bool {
    g.battle && g.ground && !g.out && g.mp >= ALL[i].mp
}

/// Lucky Bounty at Combat = KH1 (owner, 2026-11-04, `design-philosophy-instructions.md` § Party and
/// field follow-ups: "a random KH1 item (Potion, Hi-Potion, Ether, Elixir by the floor's tier) instead
/// of a deck card (the deck is empty at KH1)"; `win::party::pluto_find`).
///
/// Re:CoM (Checked (decomp + exe)): Pluto's find `0x1401E2E60` (called only from his two action
/// states `0x1401E3350` / `0x1401E35C0`) acts on the outcome his action's begin rolled into `+0x472`
/// (`0x1401E2B50` / `0x1401E2D90`, weights `0x1407C5960`: 20 / 60 / 20 %, or 5 / 5 / 90 % with fewer
/// than 7 cards in the revolver): 0 a "nonos" shot, 1 HP and Moogle prizes (3 / 7 / 10 each), 2 cards
/// dealt from the deck (`0x140166930`, then `0x140168650` each; 3 / 6 / 9 by level). At KH1 the deck is
/// empty: outcome 2 (90 %) brings nothing. Ours: outcome 2 brings **1–2 KH1 items** instead (owner,
/// 2026-11-04: "1-2 items"; two at 50 %, each picked on its own); 0 and 1 stay Re:CoM's.
pub mod lucky_bounty {
    use crate::items::id;
    use crate::shop::{self, Tier};

    /// Pluto's outcome (`+0x472`) that deals cards.
    pub const CARDS_OUTCOME: u16 = 2;
    /// The items he can bring (the owner's four).
    pub const ITEMS: [u8; 4] = [id::POTION, id::HI_POTION, id::ETHER, id::ELIXIR];

    /// The items on a floor tier: the four, as far as the shop sells them there (`shop::STOCK`'s
    /// tiers: 1F Potion, Ether; 2F+ Hi-Potion; 7F+ Elixir).
    pub fn items(t: Tier) -> Vec<u8> {
        shop::buy_list(t).into_iter().filter(|i| ITEMS.contains(i)).collect()
    }

    /// How many items one find brings: two when Re:CoM's random float `r` is under one half, else one
    /// (the owner's 1–2 at even odds; the Party list plays only LV3, so Re:CoM's 3 / 6 / 9 cards by
    /// level give no split of their own).
    pub const MAX: usize = 2;
    pub fn count(r: f32) -> usize {
        if r < 0.5 {
            MAX
        } else {
            1
        }
    }

    /// The items of one find: `count(r)` of them, each picked on its own from the next floats.
    pub fn find(t: Tier, r: f32, mut next: impl FnMut() -> f32) -> Vec<u8> {
        (0..count(r)).filter_map(|_| pick(t, next())).collect()
    }

    /// One of them, equally likely, from Re:CoM's random float `r` in [0, 1).
    pub fn pick(t: Tier, r: f32) -> Option<u8> {
        let l = items(t);
        if l.is_empty() {
            return None;
        }
        let k = ((r.clamp(0.0, 1.0) * l.len() as f32) as usize).min(l.len() - 1);
        Some(l[k])
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn lucky_bounty_brings_the_floors_items() {
        use super::lucky_bounty::*;
        use crate::items::id::*;
        use crate::shop::{tier, Tier};
        assert_eq!(items(tier(1)), [POTION, ETHER]);
        assert_eq!(items(tier(2)), [POTION, HI_POTION, ETHER]);
        assert_eq!(items(tier(7)), [POTION, HI_POTION, ETHER, ELIXIR]);
        assert_eq!(items(tier(13)), [POTION, HI_POTION, ETHER, ELIXIR]);
        assert!(items(Tier::Late).iter().all(|i| ITEMS.contains(i)), "never a Tent or Camping Set");
        assert_eq!(pick(tier(1), 0.0), Some(POTION));
        assert_eq!(pick(tier(1), 0.49), Some(POTION));
        assert_eq!(pick(tier(1), 0.5), Some(ETHER));
        assert_eq!(pick(tier(1), 0.999_999), Some(ETHER));
        assert_eq!(pick(tier(1), 1.0), Some(ETHER), "clamped");
        assert_eq!(pick(tier(7), 0.75), Some(ELIXIR));
        assert_eq!(CARDS_OUTCOME, 2);
        // 1–2 items: two under one half, each its own pick.
        assert_eq!((count(0.0), count(0.499), count(0.5), count(0.999)), (2, 2, 1, 1));
        let mut rs = [0.0f32, 0.75].into_iter();
        assert_eq!(find(tier(7), 0.2, || rs.next().unwrap()), [POTION, ELIXIR]);
        let mut rs = [0.6f32].into_iter();
        assert_eq!(find(tier(1), 0.7, || rs.next().unwrap()), [ETHER]);
        let mut calls = 0;
        let one = find(tier(1), 0.9, || {
            calls += 1;
            0.0
        });
        assert_eq!((one.len(), calls), (1, 1), "one find, one more draw");
        // Over many finds: about half bring two (Re:CoM's random is uniform: Assumption).
        let n = 1000;
        let two = (0..n).filter(|i| count(*i as f32 / n as f32) == 2).count();
        assert_eq!(two, n / 2);
    }

    use super::*;

    fn at(world: i32, met: u32) -> Place {
        Place { world, room_type: 3, riku: false, minigame: false, met }
    }

    const ALL_MET: u32 = 0x1FF;

    #[test]
    fn the_sleights_and_commands() {
        assert_eq!(COUNT, 13);
        let ids: Vec<u16> = ALL.iter().map(|f| f.sleight).collect();
        assert_eq!(ids, [0x45, 0x46, 0x48, 0x49, 0x4A, 0x56, 0x4C, 0x4E, 0x50, 0x52, 0x54, 0x63, 0x64]);
        for i in 0..COUNT {
            assert_eq!(of_command(command(i)), Some(i));
            assert!((0xC5..=0xD6).contains(&command(i)) || (0xE3..=0xE4).contains(&command(i)));
            assert!(crate::riku::sleight(ALL[i].sleight).is_none(), "not one of Riku's six");
            assert_ne!(command(i), crate::gimmick::STAGGER);
            assert!(crate::abilities::of_sleight(ALL[i].sleight).is_none(), "not one of the 27");
        }
        assert_eq!(name(0x4E), Some("Surprise!"));
        assert_eq!(index(0x44), None, "Magic LV2 is not listed");
        assert_eq!(words().count(), COUNT);
    }

    #[test]
    fn the_table_is_the_checked_sites() {
        let site = crate::sites::PARTY_CHECKED.iter().find(|s| s.rva == 0x007C_1F10).expect("the table's site");
        assert_eq!(site.expect, DROP_TABLE.concat().as_slice());
    }

    #[test]
    fn mp_cost() {
        assert_eq!(points(0x45, 2), Some(2));
        assert_eq!(points(0x45, 1), None);
        assert_eq!(points(0x4A, 3), Some(3));
        assert_eq!(points(0x56, 1), Some(1));
        assert_eq!(points(0x1B, 9), None, "not a party sleight");
    }

    #[test]
    fn rows_are_recoms() {
        assert_eq!(drop_row(&at(2, 0)), 2);
        assert_eq!(drop_row(&at(20, 0)), 13, "clamped to 13");
        assert_eq!(drop_row(&Place { room_type: 0, ..at(2, 0) }), 13, "a hall");
        assert_eq!(drop_row(&Place { room_type: 99, ..at(6, 0) }), 13);
        assert_eq!(drop_row(&Place { riku: true, ..at(2, 0) }), 15);
    }

    #[test]
    fn friends_drop_in_their_worlds_once_met() {
        use card::*;
        // Traverse Town: Donald and Goofy only (owner: no Sandstorm there).
        let tt = at(1, ALL_MET);
        assert!(can_drop(DONALD, &tt) && can_drop(GOOFY, &tt));
        assert!(!can_drop(ALADDIN, &tt));
        // Agrabah's Aladdin, Halloween Town's Jack, Atlantica's Ariel, Neverland's Peter Pan, Hollow
        // Bastion's Beast.
        for (w, c) in [(2, ALADDIN), (6, JACK), (7, ARIEL), (8, PETER_PAN), (9, BEAST)] {
            assert!(can_drop(c, &at(w, ALL_MET)), "world {w}");
            assert!(!can_drop(c, &at(w, ALL_MET & !met_bit(c))), "world {w}: not met yet");
            assert!(!can_drop(c, &at(1, ALL_MET)));
        }
        // 100 Acre Wood and world 0: no row.
        assert!(!can_drop(DONALD, &at(10, ALL_MET)));
        assert!(!can_drop(DONALD, &at(0, ALL_MET)));
        // Castle Oblivion's halls.
        assert!(can_drop(GOOFY, &Place { room_type: 0, ..at(0, ALL_MET) }));
        assert!(!can_drop(ALADDIN, &Place { room_type: 99, ..at(2, ALL_MET) }), "a hall is row 13");
        // Pluto: any world, his own roll.
        assert!(can_drop(PLUTO, &at(10, ALL_MET)));
        assert!(!can_drop(PLUTO, &Place { minigame: true, ..at(1, ALL_MET) }));
        assert!(!can_drop(PLUTO, &at(1, ALL_MET & !0x100)));
        assert!(!can_drop(KING, &at(1, ALL_MET)), "Riku's story only");
        assert!(!can_drop(DONALD, &at(1, 0)));
    }

    #[test]
    fn availability_needs_the_learned_bit_and_every_card() {
        let learned: Set = (1 << COUNT) - 1;
        let tt = available(learned, &at(1, ALL_MET));
        let names: Vec<&str> = list(tt).iter().map(|&i| ALL[i].name).collect();
        assert_eq!(names, ["Donald Magic", "Stardust Blitz", "Goofy Tornado", "Goofy Smash", "Wild Crush", "Lucky Bounty"]);
        let ag = available(learned, &at(2, ALL_MET));
        assert!(ag >> 6 & 1 != 0, "Sandstorm in Agrabah");
        // Wild Crush needs both.
        assert_eq!(available(learned, &at(1, met_bit(card::DONALD))) >> 4 & 1, 0);
        // Not learned: never.
        assert_eq!(available(learned & !1, &at(1, ALL_MET)) & 1, 0);
    }

    #[test]
    fn usable_is_the_sleights_gate() {
        let g = crate::summons::Gate { battle: true, ground: true, air: false, mp: 2, out: false, field: false, packs: true };
        assert!(usable(&g, 0));
        assert!(!usable(&g, 4), "Wild Crush 3 MP");
        assert!(usable(&crate::summons::Gate { mp: 1, ..g }, 5), "Lucky Bounty 1 MP");
        assert!(!usable(&crate::summons::Gate { ground: false, air: true, ..g }, 0), "ground only");
        assert!(!usable(&crate::summons::Gate { battle: false, ..g }, 0));
        assert!(!usable(&crate::summons::Gate { out: true, ..g }, 0));
    }

    #[test]
    fn the_kings_sleights_only_in_rikus_battles() {
        let k = [index(0x63).unwrap(), index(0x64).unwrap()];
        let all: Set = (1 << COUNT) - 1;
        // Sora's story: never (no King card in rows 0..13).
        for w in 0..=13 {
            let a = available(all, &at(w, ALL_MET));
            assert!(k.iter().all(|&i| a >> i & 1 == 0), "world {w}");
        }
        // Riku's story: wherever his row gives the King's card, once met; never Sora's friends.
        let riku = |world, met| Place { world, room_type: 3, riku: true, minigame: false, met };
        let a = available(all, &riku(1, ALL_MET));
        assert_eq!(list(a), k.to_vec());
        assert_eq!(available(all, &riku(1, ALL_MET & !0x80)), 0, "the King not met");
        assert_eq!(available(all, &riku(10, ALL_MET)), 0, "row 23: no drops");
    }
}
