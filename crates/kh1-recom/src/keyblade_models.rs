//! The equipped Keyblade's model at the KH1 setting (decision 25, "Models"): the testable part.
//! Windows side: `win::models`. Notes: `docs/recom-keyblade-models.md` § Runtime.
//!
//! - **Rows**: the converted Keyblades are new resource groups (`19 + card`: KH1's 21..=36, KH2's
//!   38..=43, [`our_group_for_card`]) of Re:CoM's own PLAYER resource
//!   table, each laid out as Re:CoM's Kingdom Key group 2 (`PL0002.RTM, .MDL, .ANN, .EET`, LINK
//!   `"30509"`) with its own RTM and MDL. [`plan_table`] builds the extended copy from Re:CoM's
//!   rows; the DLL installs it with one pointer write (`0x1407EBF28`).
//! - **Pick**: Re:CoM builds Sora's weapon with group 2 (Halloween Town: 12). At the KH1 setting
//!   in Sora's story the weapon gets the equipped Keyblade's group instead ([`Models::wanted`]).
//! - **Camp close**: the weapon is rebuilt once when the group it was built with is not the one
//!   wanted now ([`camp_close`]); our groups no longer used are unloaded ([`stale`]).

use crate::attack::Kh1Stats;
use crate::keyblades::KINGDOM_KEY;

/// Re:CoM's resource category of the player files (`PTR 0x1407EBF20[1]`). Checked (exe).
pub const CATEGORY: u16 = 1;
/// Resource id = category × 10000 + group (`0x1401A4E70`, `0x14023B9D0`). Checked (decomp).
pub fn resource_id(group: u16) -> u32 {
    CATEGORY as u32 * 10000 + group as u32
}

/// Re:CoM's Kingdom Key group (PL0002) and Halloween Town's Keyblade group (PL0012): what
/// `0x140224BA0` asks for, `0x1405A6910[char] + 1` for chars 1 and 2. Checked (decomp + exe).
pub const RECOM_KEYBLADE: u16 = 2;
pub const RECOM_HALLOWEEN: u16 = 12;

/// Resource types in the table (`docs/recom-keyblade-models.md` § 3). Checked (exe).
pub mod kind {
    pub const MDL: u16 = 0xB;
    pub const RTM: u16 = 0xC;
    pub const EET: u16 = 0x1A;
    pub const ANN: u16 = 0x1D;
    pub const LINK: u16 = 0x27;
}

/// The group that ends Re:CoM's tables. Checked (exe: row 552 of the PLAYER table).
pub const END: u16 = 0xFFFF;

/// The group the asset step gives a Keyblade card (kh1-recom-assets `keyblade::group_for_card`):
/// `19 + card` for every Keyblade Sora can hold but the Kingdom Key: KH1's cards 2..=0x11 →
/// 21..=36, KH2's 0x13..=0x18 → 38..=43; Soul Eater (0x12) has none. Re:CoM's PLAYER table
/// uses 1, 2, 11–14, 101, 111, 201, 211, 301–307, 401–409, 451, 452, 458, then 1001 and up,
/// and `Recom.hed` has no `PLAYER/00GG` file for 15..=100. Checked (exe + files). `plan_table`
/// refuses a group Re:CoM uses all the same.
pub fn our_group_for_card(card: u16) -> Option<u16> {
    match card {
        2..=0x11 | 0x13..=crate::keyblades::LAST_KEYBLADE => Some(19 + card),
        _ => None,
    }
}

/// The Re:CoM group the weapon asks for, by player char (`0x1405A6910` = {0, 1, 11, …}, + 1).
/// Chars 3..6 (Riku) draw their weapon from their own model. Checked (decomp + exe).
pub fn recom_group_for_char(char: i32) -> Option<u16> {
    match char {
        1 => Some(RECOM_KEYBLADE),
        2 => Some(RECOM_HALLOWEEN),
        _ => None,
    }
}

/// The Keyblades' model groups from `kh1-stats.json` (format 4).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Models {
    /// (Re:CoM card kind, PLAYER group) for every Keyblade with a converted model.
    pub groups: Vec<(u16, u16)>,
}

impl Models {
    /// The model groups the stats file names; None when it has none (formats 1..3, or a build
    /// without Re:CoM's PL0002.MDL): then Re:CoM's own weapon stays everywhere.
    pub fn from_stats(s: &Kh1Stats) -> Result<Option<Models>, String> {
        let mut groups: Vec<(u16, u16)> = Vec::new();
        for k in &s.keyblades {
            let Some(g) = k.group else { continue };
            if k.card == KINGDOM_KEY {
                if g != RECOM_KEYBLADE {
                    return Err(format!("{}: group {g}, but the Kingdom Key is Re:CoM's group {RECOM_KEYBLADE}", k.name));
                }
                continue;
            }
            if our_group_for_card(k.card) != Some(g) {
                return Err(format!("{}: card {:#x} has group {g}, not {:?}", k.name, k.card, our_group_for_card(k.card)));
            }
            if groups.iter().any(|(c, x)| *x == g || *c == k.card) {
                return Err(format!("{}: card {:#x} / group {g} given twice", k.name, k.card));
            }
            groups.push((k.card, g));
        }
        let any = s.keyblades.iter().any(|k| k.group.is_some());
        Ok(any.then_some(Models { groups }))
    }

    /// The group of a card's model: the Kingdom Key is Re:CoM's group 2; None for a Keyblade
    /// without a converted model.
    pub fn group_of(&self, card: u16) -> Option<u16> {
        if card == KINGDOM_KEY {
            return Some(RECOM_KEYBLADE);
        }
        self.groups.iter().find(|(c, _)| *c == card).map(|(_, g)| *g)
    }

    /// The card a group shows (for the log).
    pub fn card_of(&self, group: u16) -> Option<u16> {
        if group == RECOM_KEYBLADE {
            return Some(KINGDOM_KEY);
        }
        self.groups.iter().find(|(_, g)| *g == group).map(|(c, _)| *c)
    }

    /// One of our groups (not Re:CoM's): the only ones we load and unload ourselves.
    pub fn is_ours(&self, group: u16) -> bool {
        self.groups.iter().any(|(_, g)| *g == group)
    }

    /// Our groups, ascending.
    pub fn our_groups(&self) -> Vec<u16> {
        let mut v: Vec<u16> = self.groups.iter().map(|(_, g)| *g).collect();
        v.sort_unstable();
        v
    }

    /// The group Sora's weapon should be built with when Re:CoM asks for `requested` (2 or 12),
    /// or None for Re:CoM's own choice: at the Re:CoM setting, in Riku's story, for a group that
    /// is not Sora's Keyblade, and for a card without a model. At the KH1 setting Halloween Town
    /// shows the equipped Keyblade too (the Kingdom Key there is group 2, not 12).
    pub fn wanted(&self, requested: u16, kh1: bool, sora_story: bool, card: u16) -> Option<u16> {
        if !kh1 || !sora_story || (requested != RECOM_KEYBLADE && requested != RECOM_HALLOWEEN) {
            return None;
        }
        self.group_of(card)
    }
}

/// One row of Re:CoM's resource table as read from memory: `u16 group | u16 type | u32 0 |
/// char *name` (16 bytes). Checked (exe).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawRow {
    pub group: u16,
    pub kind: u16,
    pub zero: u32,
    pub name: u64,
}

impl RawRow {
    pub fn to_bytes(self) -> [u8; 16] {
        let mut b = [0u8; 16];
        b[0..2].copy_from_slice(&self.group.to_le_bytes());
        b[2..4].copy_from_slice(&self.kind.to_le_bytes());
        b[4..8].copy_from_slice(&self.zero.to_le_bytes());
        b[8..16].copy_from_slice(&self.name.to_le_bytes());
        b
    }

    pub fn from_bytes(b: &[u8; 16]) -> RawRow {
        RawRow {
            group: u16::from_le_bytes([b[0], b[1]]),
            kind: u16::from_le_bytes([b[2], b[3]]),
            zero: u32::from_le_bytes([b[4], b[5], b[6], b[7]]),
            name: u64::from_le_bytes(b[8..16].try_into().unwrap_or([0; 8])),
        }
    }
}

/// A row of the extended table: Re:CoM's name pointer, or a path of ours (to be allocated).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowName {
    Recom(u64),
    Own(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedRow {
    pub group: u16,
    pub kind: u16,
    pub name: RowName,
}

/// `PLAYER/<group>/PL<group>.<ext>`, as Re:CoM's rows name its files (`PLAYER/0002/PL0002.MDL`)
/// and as the asset step writes them.
pub fn resource_path(group: u16, ext: &str) -> String {
    format!("PLAYER/{group:04}/PL{group:04}.{ext}")
}

/// The shape Re:CoM's group 2 must have for our groups to copy it. Checked (exe: rows
/// 0x140805690..0x1408056D0).
const GROUP2: [(u16, &str); 5] = [
    (kind::RTM, "PLAYER/0002/PL0002.RTM"),
    (kind::MDL, "PLAYER/0002/PL0002.MDL"),
    (kind::ANN, "PLAYER/0002/PL0002.ANN"),
    (kind::EET, "PLAYER/0002/PL0002.EET"),
    (kind::LINK, "30509"),
];

/// The extended PLAYER table: Re:CoM's rows as they are, then one group per entry of `groups`
/// laid out as Re:CoM's group 2 (same row order and types; its own RTM and MDL paths; group 2's
/// ANN, EET and LINK `"30509"` name pointers), then Re:CoM's end row.
///
/// The loader (`0x14023BF70`) finds a group's first row by a linear scan and reads its rows while
/// the group stays the same, so a group's rows must be contiguous and the end row last. Checked
/// (decomp). `recom` is the table up to and including its end row; `name` reads a name pointer.
pub fn plan_table(recom: &[RawRow], name: impl Fn(u64) -> Option<String>, groups: &[u16]) -> Result<Vec<PlannedRow>, String> {
    let end = recom.iter().position(|r| r.group == END).ok_or("Re:CoM's table has no end row")?;
    if end + 1 != recom.len() {
        return Err("rows after Re:CoM's end row".into());
    }
    let body = &recom[..end];
    // Re:CoM's group 2: contiguous, five rows, as checked.
    let first = body.iter().position(|r| r.group == RECOM_KEYBLADE).ok_or("Re:CoM's table has no group 2")?;
    let g2: Vec<RawRow> = body[first..].iter().take_while(|r| r.group == RECOM_KEYBLADE).copied().collect();
    if body.iter().filter(|r| r.group == RECOM_KEYBLADE).count() != g2.len() || g2.len() != GROUP2.len() {
        return Err(format!("Re:CoM's group 2 has {} rows, not the {} expected", g2.len(), GROUP2.len()));
    }
    for (r, (k, n)) in g2.iter().zip(GROUP2.iter()) {
        let have = name(r.name).unwrap_or_default();
        if r.kind != *k || have != *n || r.zero != 0 {
            return Err(format!("Re:CoM's group 2 row: type {:#x} {have:?}, expected {k:#x} {n:?}", r.kind));
        }
    }
    let mut seen: Vec<u16> = Vec::new();
    for &g in groups {
        if g == END || g == 0 || seen.contains(&g) {
            return Err(format!("group {g} given twice or reserved"));
        }
        if body.iter().any(|r| r.group == g) {
            return Err(format!("group {g} is already one of Re:CoM's"));
        }
        seen.push(g);
    }
    let mut out: Vec<PlannedRow> = body.iter().map(|r| PlannedRow { group: r.group, kind: r.kind, name: RowName::Recom(r.name) }).collect();
    for &g in groups {
        for r in &g2 {
            let n = match r.kind {
                kind::RTM => RowName::Own(resource_path(g, "RTM")),
                kind::MDL => RowName::Own(resource_path(g, "MDL")),
                _ => RowName::Recom(r.name),
            };
            out.push(PlannedRow { group: g, kind: r.kind, name: n });
        }
    }
    out.push(PlannedRow { group: END, kind: recom[end].kind, name: RowName::Recom(recom[end].name) });
    Ok(out)
}

/// What the camp-close point does with the weapon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Close {
    /// The weapon already shows the wanted group.
    Keep,
    /// Rebuild the weapon now with this group (its files are loaded).
    Rebuild(u16),
    /// Its files are not loaded yet: queue them; Re:CoM's next weapon build (next room or
    /// battle) or the next camp close picks it.
    Wait(u16),
    /// Sora is mid-attack (or the weapon is not one we saw built): leave it to the next build.
    Skip,
}

/// The camp-close decision: `built` the group the current weapon was built with (None if we did
/// not see it built), `want` the group wanted now, `loaded` whether its files are in, `attacking`
/// whether Sora is in Re:CoM's attack action.
pub fn camp_close(built: Option<u16>, want: u16, loaded: bool, attacking: bool) -> Close {
    match built {
        None => Close::Skip,
        Some(b) if b == want => Close::Keep,
        Some(_) if attacking => Close::Skip,
        Some(_) if loaded => Close::Rebuild(want),
        Some(_) => Close::Wait(want),
    }
}

/// Our groups to unload: loaded by us and neither the one the weapon uses nor the one wanted.
pub fn stale(ours_loaded: &[u16], keep: &[u16]) -> Vec<u16> {
    ours_loaded.iter().copied().filter(|g| !keep.contains(g)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attack::Keyblade;
    use kh1_core::WeaponData;

    fn kb(card: u16, group: Option<u16>) -> Keyblade {
        Keyblade { card, name: format!("k{card}"), source: "kh1".into(), model: String::new(), weapon: WeaponData::kingdom_key(), magic: 0, group, textures: vec![] }
    }

    fn stats(k: Vec<Keyblade>) -> Kh1Stats {
        Kh1Stats { keyblades: k, ..Default::default() }
    }

    #[test]
    fn groups_come_from_format_4_only() {
        // Format 3 (no `group`): no model swap at all.
        assert_eq!(Models::from_stats(&stats(vec![kb(1, None), kb(2, None)])).unwrap(), None);
        let m = Models::from_stats(&stats(vec![kb(1, None), kb(2, Some(21)), kb(17, Some(36)), kb(3, None)])).unwrap().unwrap();
        assert_eq!(m.groups, vec![(2, 21), (17, 36)]);
        assert_eq!(m.group_of(1), Some(2));
        assert_eq!(m.group_of(2), Some(21));
        assert_eq!(m.group_of(3), None);
        assert_eq!(m.card_of(36), Some(17));
        assert_eq!(m.card_of(2), Some(1));
        assert!(m.is_ours(21) && !m.is_ours(2) && !m.is_ours(12));
        // Bad files are refused, not half used.
        assert!(Models::from_stats(&stats(vec![kb(2, Some(12))])).is_err());
        assert!(Models::from_stats(&stats(vec![kb(2, Some(21)), kb(3, Some(21))])).is_err());
        assert!(Models::from_stats(&stats(vec![kb(1, Some(21))])).is_err());
        assert!(Models::from_stats(&stats(vec![kb(1, Some(2)), kb(2, Some(21))])).is_ok());
    }

    #[test]
    fn kh2_keyblades_get_groups_38_to_43() {
        assert_eq!(our_group_for_card(2), Some(21));
        assert_eq!(our_group_for_card(0x11), Some(36));
        assert_eq!(our_group_for_card(0x12), None, "Soul Eater");
        assert_eq!(our_group_for_card(0x13), Some(38));
        assert_eq!(our_group_for_card(0x18), Some(43));
        assert_eq!(our_group_for_card(0x19), None);
        assert_eq!(our_group_for_card(1), None, "the Kingdom Key is Re:CoM's group 2");
        // All 22 converted Keyblades: KH1's 16 and KH2's 6.
        let all: Vec<Keyblade> = std::iter::once(kb(1, Some(2))).chain((2..=0x11).chain(0x13..=0x18).map(|c| kb(c, Some(19 + c)))).collect();
        let m = Models::from_stats(&stats(all)).unwrap().unwrap();
        assert_eq!(m.groups.len(), 22);
        assert_eq!(m.our_groups(), (21..=36).chain(38..=43).collect::<Vec<u16>>());
        assert_eq!(m.group_of(0x16), Some(41));
        assert_eq!(m.card_of(43), Some(0x18));
        assert_eq!(m.wanted(2, true, true, 0x13), Some(38));
        assert_eq!(m.wanted(12, true, true, 0x18), Some(43), "Halloween Town too");
        // A KH2 card with a KH1-range group, Soul Eater, or group 37: refused.
        assert!(Models::from_stats(&stats(vec![kb(0x13, Some(21))])).is_err());
        assert!(Models::from_stats(&stats(vec![kb(0x12, Some(37))])).is_err());
        assert!(Models::from_stats(&stats(vec![kb(0x13, Some(37))])).is_err());
        // The table gets them after Re:CoM's rows, each laid out as group 2.
        let (rows, names) = recom_rows();
        let name = |p: u64| names.iter().find(|(q, _)| *q == p).map(|(_, s)| s.to_string());
        let t = plan_table(&rows, name, &m.our_groups()).unwrap();
        assert_eq!(t.len(), rows.len() + 5 * 22);
        let g43: Vec<&PlannedRow> = t.iter().filter(|r| r.group == 43).collect();
        assert_eq!(g43[0].name, RowName::Own("PLAYER/0043/PL0043.RTM".into()));
        assert_eq!(g43[1].name, RowName::Own("PLAYER/0043/PL0043.MDL".into()));
    }

    /// A KH2 row as the asset step writes it (`source` `kh2`, extra keys the parser ignores).
    #[test]
    fn kh2_rows_parse() {
        let row = r#"{"card": 24, "name": "Bond of Flame", "source": "kh2", "kh2_item": 498, "kh2_stat": 141, "model": "W_EX010_K0", "blade_length": 0.9100403, "kh2_kingdom_key_blade_length": 0.77902657, "radius": 40, "blade_far": -4.322252, "blade_near": 0, "frac_scale": 0.79999995, "strength": 4, "crit": 20, "crit_bonus": 0, "priority": 30, "magic": 4, "group": 43, "textures": ["w_ex010_k0_0"]}"#;
        let text = format!("{{\"sora_strength\": [{}], \"keyblades\": [{row}], \"enemies\": []}}", vec!["7"; 100].join(", "));
        let s = crate::attack::parse_stats(&text).unwrap();
        let k = s.keyblade(0x18).unwrap();
        assert_eq!((k.source.as_str(), k.model.as_str(), k.weapon.strength, k.magic, k.group), ("kh2", "W_EX010_K0", 4, 4, Some(43)));
        assert_eq!((k.weapon.radius, k.weapon.blade_near, k.weapon.crit, k.weapon.crit_bonus, k.weapon.priority), (40.0, 0.0, 20, 0, 30));
        assert!((k.weapon.blade_far - -4.322252).abs() < 1e-6);
        assert_eq!(k.textures, vec!["w_ex010_k0_0".to_string()]);
        let m = Models::from_stats(&s).unwrap().unwrap();
        assert_eq!(m.group_of(0x18), Some(43));
        // The camp page lists it once earned, and the save byte keeps it.
        assert_eq!(crate::keyblades::unlocked([1, 0x18], |c| c == 0x18), vec![1, 0x18]);
        assert_eq!(crate::keyblades::equipped_card(0x18, |c| s.keyblade(c).is_some()), 0x18);
        assert_eq!(crate::keyblades::save_byte_for(0x18), 0x18);
    }

    #[test]
    fn the_pick_follows_the_setting_the_story_and_the_card() {
        let m = Models { groups: vec![(2, 21), (15, 34)] };
        // KH1, Sora's story: the equipped one, in every world.
        assert_eq!(m.wanted(2, true, true, 15), Some(34));
        assert_eq!(m.wanted(12, true, true, 15), Some(34));
        // Halloween Town's Kingdom Key at KH1 is group 2, not 12 (owner).
        assert_eq!(m.wanted(12, true, true, 1), Some(2));
        assert_eq!(m.wanted(2, true, true, 1), Some(2));
        // Re:CoM setting, Riku's story, a group that is not Sora's Keyblade: Re:CoM's own.
        assert_eq!(m.wanted(2, false, true, 15), None);
        assert_eq!(m.wanted(12, false, true, 15), None);
        assert_eq!(m.wanted(2, true, false, 15), None);
        assert_eq!(m.wanted(101, true, true, 15), None);
        // A Keyblade without a model: Re:CoM's own.
        assert_eq!(m.wanted(2, true, true, 5), None);
        assert_eq!(recom_group_for_char(1), Some(2));
        assert_eq!(recom_group_for_char(2), Some(12));
        assert_eq!(recom_group_for_char(3), None);
        assert_eq!(resource_id(21), 10021);
    }

    /// Re:CoM's PLAYER table around group 2, as in the exe (names by fake pointers).
    fn recom_rows() -> (Vec<RawRow>, Vec<(u64, &'static str)>) {
        let names: Vec<(u64, &str)> = vec![
            (0x100, "PLAYER/0001/PL0001.RTM"),
            (0x110, "10002"),
            (0x200, "PLAYER/0002/PL0002.RTM"),
            (0x210, "PLAYER/0002/PL0002.MDL"),
            (0x220, "PLAYER/0002/PL0002.ANN"),
            (0x230, "PLAYER/0002/PL0002.EET"),
            (0x240, "30509"),
            (0x300, "PLAYER/0011/PL0011.RTM"),
        ];
        let r = |group, kind, name| RawRow { group, kind, zero: 0, name };
        let rows = vec![
            r(1, kind::RTM, 0x100),
            r(1, kind::LINK, 0x110),
            r(2, kind::RTM, 0x200),
            r(2, kind::MDL, 0x210),
            r(2, kind::ANN, 0x220),
            r(2, kind::EET, 0x230),
            r(2, kind::LINK, 0x240),
            r(11, kind::RTM, 0x300),
            r(END, 0xFFFF, 0),
        ];
        (rows, names)
    }

    #[test]
    fn the_table_gets_our_groups_laid_out_as_group_2() {
        let (rows, names) = recom_rows();
        let name = |p: u64| names.iter().find(|(q, _)| *q == p).map(|(_, s)| s.to_string());
        let t = plan_table(&rows, name, &[21, 22]).unwrap();
        assert_eq!(t.len(), rows.len() + 10);
        // Re:CoM's rows unchanged and first.
        for (a, b) in rows[..rows.len() - 1].iter().zip(&t) {
            assert_eq!((a.group, a.kind, RowName::Recom(a.name)), (b.group, b.kind, b.name.clone()));
        }
        let g21: Vec<&PlannedRow> = t.iter().filter(|r| r.group == 21).collect();
        assert_eq!(g21.iter().map(|r| r.kind).collect::<Vec<_>>(), vec![kind::RTM, kind::MDL, kind::ANN, kind::EET, kind::LINK]);
        assert_eq!(g21[0].name, RowName::Own("PLAYER/0021/PL0021.RTM".into()));
        assert_eq!(g21[1].name, RowName::Own("PLAYER/0021/PL0021.MDL".into()));
        assert_eq!(g21[2].name, RowName::Recom(0x220));
        assert_eq!(g21[3].name, RowName::Recom(0x230));
        assert_eq!(g21[4].name, RowName::Recom(0x240));
        // Each group contiguous; the end row last.
        let first22 = t.iter().position(|r| r.group == 22).unwrap();
        assert!(t[first22..first22 + 5].iter().all(|r| r.group == 22));
        assert_eq!(t.last().unwrap().group, END);
        assert_eq!(t.iter().filter(|r| r.group == END).count(), 1);
        // Byte layout round trip.
        let b = RawRow { group: 21, kind: kind::MDL, zero: 0, name: 0x1_4080_55D0 }.to_bytes();
        assert_eq!(RawRow::from_bytes(&b), RawRow { group: 21, kind: kind::MDL, zero: 0, name: 0x1_4080_55D0 });
        assert_eq!(&b[..4], &[21, 0, 0xB, 0]);
    }

    #[test]
    fn the_table_is_refused_when_recom_differs() {
        let (rows, names) = recom_rows();
        let name = |p: u64| names.iter().find(|(q, _)| *q == p).map(|(_, s)| s.to_string());
        assert!(plan_table(&rows, &name, &[11]).is_err(), "Re:CoM's own group");
        assert!(plan_table(&rows, &name, &[21, 21]).is_err());
        assert!(plan_table(&rows[..rows.len() - 1], &name, &[21]).is_err(), "no end row");
        let mut odd = rows.clone();
        odd[4].kind = 0x17;
        assert!(plan_table(&odd, &name, &[21]).is_err());
        let wrong_name = |p: u64| if p == 0x240 { Some("30510".into()) } else { name(p) };
        assert!(plan_table(&rows, wrong_name, &[21]).is_err());
        assert_eq!(plan_table(&rows, &name, &[]).unwrap().len(), rows.len());
    }

    #[test]
    fn camp_close_rebuilds_once_and_only_when_loaded() {
        assert_eq!(camp_close(Some(2), 2, true, false), Close::Keep);
        assert_eq!(camp_close(Some(2), 21, true, false), Close::Rebuild(21));
        assert_eq!(camp_close(Some(21), 2, true, false), Close::Rebuild(2));
        assert_eq!(camp_close(Some(2), 21, false, false), Close::Wait(21));
        assert_eq!(camp_close(Some(2), 21, true, true), Close::Skip);
        assert_eq!(camp_close(None, 21, true, false), Close::Skip);
        assert_eq!(stale(&[21, 22, 23], &[22, 2]), vec![21, 23]);
        assert!(stale(&[21], &[21]).is_empty());
    }
}

