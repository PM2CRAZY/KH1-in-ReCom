//! Riku's Dark Form skin (fork owner, 2026-11-22: "ele vai desbloquear a skin do Dark Mode e vai
//! poder jogar com ela no modo sem estar com darkmode ativado"; "pode colocar a skin mas os
//! movimentos ser o normal mesmo"). Pure rules; the game side is `win::riku_skin`.
//!
//! - Files (asset step `kh1_recom_assets::riku_skin`): player groups 121 / 131 hold Dark Riku's
//!   model and textures (`PL0201` / `PL0211`) with Riku's own motions retargeted onto it and
//!   Riku's sound / effect events (`PL0101` / `PL0111`), bones remapped by name.
//! - Rows: each skin group's rows copy its Riku group's rows in Re:CoM's PLAYER table (same
//!   types and order); the files the step wrote are named by the skin group's paths, the rest
//!   (camera data `.BIN`, sound bank `.SND`, effect links) stay Riku's.
//! - Use: Re:CoM picks a player's group from `0x1405A6910[char]` (`.rdata`: 0, 1, 11, 101, 111,
//!   201, 211; Checked (exe)) in the player ctor (`0x1401E4CDA`, stored at `+0x39C`), the group
//!   load (`0x140211A5B`, `0x140211AE4`) and unload (`0x1402115EE`, `0x14021161F`). Chars 3 / 4 →
//!   the skin groups while the skin is on. Riku's weapon takes no group (`0x140224C58`: chars
//!   3..6 attach to a body bone). Riku's bones are looked up by name (`0x1401A4450`), and the
//!   skin's bone 1 carries Riku's name (`ex_1010_sk`).

use crate::keyblade_models::{kind, resource_path, PlannedRow, RowName, END};

/// Re:CoM's char → action table pointers (`0x1407C6300[char]`, 16-byte rows: name pointer, clip
/// index `+8`). Checked (exe: `0x1401FA7FE`).
pub const ACTION_TABLES_PTR: u64 = 0x1_407C_6300;

/// Re:CoM's char → group table and the entries of chars 3 and 4. Checked (exe).
pub const GROUP_TABLE: u64 = 0x1_405A_6910;
/// Re:CoM's char → motion pack name table (`0x1407C6338[char]`: "PL0001" … "PL0211"): the player's
/// action rows' clips are looked up in the `.ANN` of that resource name (`0x1401FA7F6`,
/// `0x1401F425C` → `0x1402A42B0`). Only these two readers and one pointer per name (Checked
/// (exe)). 0.154.0 changed the group but not this name: no clip was found (the owner's T-pose).
pub const ANN_NAME_TABLE: u64 = 0x1_407C_6338;
/// Riku's two names ("PL0101", "PL0111") the table points at. Checked (exe).
pub const RIKU_ANN_NAMES: [u64; 2] = [0x1_405A_145C, 0x1_405A_1464];

/// The menu's own model (`CMenuModel`, ctor `0x140111F10`, resource id = group + 10000) takes its
/// group from bytes in two instructions, not from the char table (Checked (exe)): the camp's
/// `mov dword [rsp+0x30], 0x6F650B01` at `0x1400E4701` (groups 1, 11, 101, 111 by story × 2 +
/// Halloween Town, read at `0x1400E47EF`) and the level-up screen's `mov dword [rbp-0x15],
/// 0x006F0065` at `0x14010EF68` (model / weapon pairs: 101, 0, 111, 0; read at `0x14010F084`,
/// `0x14010F0E9`). No branch lands inside them (Checked (exe)). Each entry: the byte's address
/// and Riku's group (101 or 111).
pub const MENU_BYTES: [(u64, u16); 4] = [(0x1_400E_4707, 101), (0x1_400E_4708, 111), (0x1_4010_EF6B, 101), (0x1_4010_EF6D, 111)];

/// The motion pack name of a group (`PL0121`).
pub fn ann_name(group: u16) -> String {
    format!("PL{group:04}")
}

/// Riku's two groups (chars 3, 4), as the table holds them.
pub const RIKU_GROUPS: [u16; 2] = [101, 111];

/// Row types the skin's own files fill (Texture anim `.TXA` 0x13 and `.SES` 0x30, Checked (exe:
/// the table's rows)).
const OWN_KINDS: [(u16, &str); 6] = [(kind::RTM, "RTM"), (0x13, "TXA"), (kind::MDL, "MDL"), (kind::ANN, "ANN"), (kind::EET, "EET"), (0x30, "SES")];

/// One skin: Riku's group and the group that stands for it, with its look: the Dark Mode attire
/// and the blade (0.160.0: 0 the Soul Eater, 1 the Keyblade of People's Hearts). Older manifests
/// (no `dark` / `blade`): Dark Mode with the Soul Eater.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Skin {
    pub riku: u16,
    pub skin: u16,
    pub dark: bool,
    pub blade: u8,
    /// 0: Re:CoM's hair, 1: KH2 Riku's, 2: BBS kid Riku's (only the default attire has them; 0.186.0's 1 was DDD's, removed by Stage 363).
    pub hair: u8,
}

/// Riku's look: the attire and the blade.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Look {
    pub dark: bool,
    pub blade: u8,
    /// 0.178.0: Riku's Halloween Town look in every world (char 3 on his Halloween group 111 and
    /// its skins, as char 4 is there).
    pub halloween: bool,
    /// 0 Re:CoM's hair, 1 KH2 Riku's, 2 BBS kid Riku's.
    pub hair: u8,
}

impl Look {
    /// As one byte (bit 0 Dark Mode, bits 1..2 the blade, bit 3 Halloween Town, bits 4..5 the hair).
    pub fn bits(self) -> u8 {
        self.dark as u8 | (self.blade & 3) << 1 | (self.halloween as u8) << 3 | (self.hair & 3) << 4
    }
    pub fn from_bits(b: u8) -> Look {
        Look { dark: b & 1 != 0, blade: (b >> 1) & 3, halloween: b & 8 != 0, hair: (b >> 4) & 3 }
    }
}

/// The Keyblades Riku can hold (the Costumes page's names).
pub const BLADES: [&str; 2] = ["Soul Eater", "Keyblade of Heart"];

/// Parses `kh1-riku-skin.json` (`{"format": 1, "skins": [{"riku": 101, "skin": 121}, …]}`).
pub fn parse(text: &str) -> Result<Vec<Skin>, String> {
    let v = crate::json::parse(text).map_err(|e| format!("kh1-riku-skin.json: {e}"))?;
    if v.get("format").and_then(|f| f.as_u32()) != Some(1) {
        return Err("kh1-riku-skin.json: not format 1".into());
    }
    let mut out = Vec::new();
    for s in v.get("skins").and_then(|s| s.as_array()).ok_or("kh1-riku-skin.json: no skins")? {
        let g = |k: &str| s.get(k).and_then(|x| x.as_u32()).filter(|&x| x > 0 && x < END as u32).map(|x| x as u16);
        let (Some(riku), Some(skin)) = (g("riku"), g("skin")) else { return Err("kh1-riku-skin.json: a skin without riku / skin".into()) };
        let dark = s.get("dark").map_or(true, |d| matches!(d, crate::json::Value::Bool(true)));
        let blade = s.get("blade").and_then(|b| b.as_u32()).unwrap_or(0);
        if blade as usize >= BLADES.len() {
            return Err(format!("kh1-riku-skin.json: skin {skin}: blade {blade}"));
        }
        let blade = blade as u8;
        let hair = s.get("hair").and_then(|b| b.as_u32()).unwrap_or(0).min(2) as u8;
        if !RIKU_GROUPS.contains(&riku) || (!dark && blade == 0 && hair == 0) || out.iter().any(|o: &Skin| (o.riku == riku && o.dark == dark && o.blade == blade && o.hair == hair) || o.skin == skin) {
            return Err(format!("kh1-riku-skin.json: skin {skin} for group {riku}"));
        }
        out.push(Skin { riku, skin, dark, blade, hair });
    }
    Ok(out)
}

/// The manifest says Riku's own packs carry Sora's KH1 Guard (`"riku_guard": true`, 0.157.0).
pub fn riku_guard(text: &str) -> bool {
    crate::json::parse(text).ok().and_then(|v| v.get("riku_guard").map(|g| matches!(g, crate::json::Value::Bool(true)))).unwrap_or(false)
}

/// 0.184.1: the manifest says Riku's and Dark Riku's own packs carry the cast clips (`"riku_cast": true`).
pub fn riku_cast(text: &str) -> bool {
    crate::json::parse(text).ok().and_then(|v| v.get("riku_cast").map(|g| matches!(g, crate::json::Value::Bool(true)))).unwrap_or(false)
}

/// 0.197.4 (fork, item 1 of Stage 391): the manifest's `event_packs`, (Re:CoM's event pack group, its 109-bone copy written by the
/// asset step for the KH2 hair). Only groups 4000..4999 → a different group of the same range; anything else is left out.
pub fn event_packs(text: &str) -> Vec<(u16, u16)> {
    let Ok(v) = crate::json::parse(text) else { return vec![] };
    let Some(list) = v.get("event_packs").and_then(|l| l.as_array()) else { return vec![] };
    let mut out: Vec<(u16, u16)> = vec![];
    for p in list {
        let Some(pair) = p.as_array() else { continue };
        let g = |i: usize| pair.get(i).and_then(|x| x.as_u32()).filter(|x| (4000..5000).contains(x)).map(|x| x as u16);
        if let (Some(a), Some(b)) = (g(0), g(1)) {
            if a != b && !out.iter().any(|o| o.0 == a || o.1 == b) {
                out.push((a, b));
            }
        }
    }
    out
}

/// The table rows of the event pack copies: one `.ANN` row each, its own file (no file shared with Re:CoM's group, so no id of
/// Re:CoM's is taken by ours), before the end row.
pub fn add_event_rows(mut planned: Vec<PlannedRow>, pairs: &[(u16, u16)]) -> Result<Vec<PlannedRow>, String> {
    let end = planned.iter().position(|r| r.group == END).ok_or("no end row")?;
    if end + 1 != planned.len() {
        return Err("rows after the end row".into());
    }
    let mut add = Vec::new();
    for &(_, alt) in pairs {
        if planned.iter().any(|r| r.group == alt) {
            return Err(format!("group {alt} is already in the table"));
        }
        add.push(PlannedRow { group: alt, kind: kind::ANN, name: RowName::Own(resource_path(alt, "ANN")) });
    }
    planned.splice(end..end, add);
    Ok(planned)
}

/// The global resource id (`10000 + group`, PLAYER) of the copy for Re:CoM's pack id `id`, if `pairs` has one.
pub fn event_alt_id(pairs: &[(u16, u16)], id: u32) -> Option<u32> {
    pairs.iter().find(|p| 10000 + p.0 as u32 == id).map(|p| 10000 + p.1 as u32)
}

/// The files of a skin group the DLL checks are on disk (relative to the manifest's folder).
pub fn files(skin: u16) -> Vec<String> {
    OWN_KINDS
        .iter()
        .map(|&(k, ext)| if k == kind::RTM || ext == "TXA" { format!("raw/{}", resource_path(skin, ext)) } else { resource_path(skin, ext) })
        .collect()
}

/// `planned` (Re:CoM's rows, ours, then the end row) with each skin's rows before the end row:
/// its Riku group's rows, contiguous, retyped to the skin's group.
pub fn add_rows(mut planned: Vec<PlannedRow>, skins: &[Skin]) -> Result<Vec<PlannedRow>, String> {
    let end = planned.iter().position(|r| r.group == END).ok_or("no end row")?;
    if end + 1 != planned.len() {
        return Err("rows after the end row".into());
    }
    let mut add = Vec::new();
    for s in skins {
        if planned.iter().any(|r| r.group == s.skin) {
            return Err(format!("group {} is already in the table", s.skin));
        }
        let first = planned.iter().position(|r| r.group == s.riku).ok_or_else(|| format!("no rows of group {}", s.riku))?;
        let rows: Vec<&PlannedRow> = planned[first..].iter().take_while(|r| r.group == s.riku).collect();
        if planned.iter().filter(|r| r.group == s.riku).count() != rows.len() {
            return Err(format!("group {}'s rows are not contiguous", s.riku));
        }
        for &(k, _) in &OWN_KINDS {
            if !rows.iter().any(|r| r.kind == k) {
                return Err(format!("group {} has no row of type {k:#x}", s.riku));
            }
        }
        for r in rows {
            let name = match OWN_KINDS.iter().find(|o| o.0 == r.kind) {
                Some(&(_, ext)) => RowName::Own(resource_path(s.skin, ext)),
                None => r.name.clone(),
            };
            add.push(PlannedRow { group: s.skin, kind: r.kind, name });
        }
    }
    planned.splice(end..end, add);
    Ok(planned)
}

/// The table's entries for chars 3 and 4 for a look (Riku's own group where no skin has it).
pub fn entries(skins: &[Skin], look: Look) -> [u32; 2] {
    RIKU_GROUPS.map(|g| group_for_char(skins, g, look) as u32)
}

/// The group for Riku's char whose own group is `riku` (101 / 111): with the Halloween Town look,
/// char 3 takes Halloween Town's (111) and its skins.
pub fn group_for_char(skins: &[Skin], riku: u16, look: Look) -> u16 {
    group_for(skins, if look.halloween { RIKU_GROUPS[1] } else { riku }, look)
}

/// Riku's group `riku` in a look.
pub fn group_for(skins: &[Skin], riku: u16, look: Look) -> u16 {
    // 0.186.0: a look with a hair that has no hair group (Dark Mode, Halloween Town…) shows Re:CoM's.
    let find = |hair: u8| skins.iter().find(|s| s.riku == riku && s.dark == look.dark && s.blade == look.blade && s.hair == hair);
    find(look.hair).or_else(|| find(0)).map_or(riku, |s| s.skin)
}

/// Dark Riku's two groups (chars 5 / 6: the battle's Dark Mode), as the table holds them. Checked (exe: `0x1405A6910[5 / 6]`).
pub const DARK_GROUPS: [u16; 2] = [201, 211];
/// Their motion pack names (`0x1407C6338[5 / 6]`). Checked (exe).
pub const DARK_ANN_NAMES: [&[u8; 7]; 2] = [b"PL0201\0", b"PL0211\0"];

/// 0.198.30 (fork owner, 2026-11-27: "reparei que quando o personagem se transforma no DarkMode o cabelo é resetado para o Default"): the
/// manifest's `dark_mode` groups (`[{"dark": 201, "skin": 205, "hair": 2, "blade": 0}, …]`: Dark Riku's group with a hair and a blade),
/// as skins whose `riku` is Dark Riku's group (their rows copy his, [`add_rows`]; [`group_for`] never picks them for chars 3 / 4). A bad
/// row is left out alone; older manifests: none.
/// 0.198.36 (fork owner, 2026-11-27: "no darkmode a keyblade tbm nn troca, já coloca essas regrs para o futuro"): `blade` (0 the Soul
/// Eater, 1 the Keyblade of People's Hearts; none: 0), and Re:CoM's hair (0) with the Keyblade; one group per (Dark Riku, hair, blade),
/// never his own look (Re:CoM's hair with the Soul Eater: his own group).
pub fn dark_mode(text: &str) -> Vec<Skin> {
    let Ok(v) = crate::json::parse(text) else { return vec![] };
    let Some(list) = v.get("dark_mode").and_then(|l| l.as_array()) else { return vec![] };
    let mut out: Vec<Skin> = vec![];
    for s in list {
        let g = |k: &str| s.get(k).and_then(|x| x.as_u32());
        let (Some(dark), Some(skin), Some(hair)) = (g("dark"), g("skin"), g("hair")) else { continue };
        let (Ok(dark), Ok(skin), Ok(hair), Ok(blade)) = (u16::try_from(dark), u16::try_from(skin), u8::try_from(hair), u8::try_from(g("blade").unwrap_or(0))) else { continue };
        let ours = skin > 0 && skin < END && !DARK_GROUPS.contains(&skin) && !RIKU_GROUPS.contains(&skin);
        let look = hair <= 2 && (blade as usize) < BLADES.len() && (hair, blade) != (0, 0);
        if DARK_GROUPS.contains(&dark) && ours && look && !out.iter().any(|o| (o.riku == dark && o.hair == hair && o.blade == blade) || o.skin == skin) {
            out.push(Skin { riku: dark, skin, dark: true, blade, hair });
        }
    }
    out
}

/// 0.198.32 (fork owner, 2026-11-27, asked to choose between 139-bone copies of his packs with a second hook and fitting KH2's hair on
/// his own hair bones: "a"): the 128-bone packs the battle's Dark Riku plays besides his own (sleights 1087, 1088, 1089, 1093, his dash
/// 1310, 1311..1313; event packs 4003, 4043, 4083). Checked (files, exe: `kh1_recom_assets::riku_skin::DARK_PACKS`). KH2's hair makes
/// him 139 bones, so none of them plays on him without its copy.
pub const DARK_PACK_SOURCES: [u16; 11] = [1087, 1088, 1089, 1093, 1310, 1311, 1312, 1313, 4003, 4043, 4083];

/// The manifest's `dark_packs` ((Re:CoM's pack, its 139-bone copy)): only [`DARK_PACK_SOURCES`], each once, onto a PLAYER group of
/// 1000..4999 that is no source; anything else is left out. Older manifests: none.
pub fn dark_packs(text: &str) -> Vec<(u16, u16)> {
    let Ok(v) = crate::json::parse(text) else { return vec![] };
    let Some(list) = v.get("dark_packs").and_then(|l| l.as_array()) else { return vec![] };
    let mut out: Vec<(u16, u16)> = vec![];
    for p in list {
        let Some(pair) = p.as_array() else { continue };
        let g = |i: usize| pair.get(i).and_then(|x| x.as_u32()).filter(|x| (1000..5000).contains(x)).map(|x| x as u16);
        if let (Some(a), Some(b)) = (g(0), g(1)) {
            if DARK_PACK_SOURCES.contains(&a) && !DARK_PACK_SOURCES.contains(&b) && !out.iter().any(|o| o.0 == a || o.1 == b) {
                out.push((a, b));
            }
        }
    }
    out
}

/// The battle Dark Riku's hair groups that can be used: KH2's (hair 1, 139 bones) only when every pack of [`DARK_PACK_SOURCES`] has
/// its copy (else his sleights would freeze him); BBS's (128 bones) always.
pub fn dark_mode_usable(dm: Vec<Skin>, packs: &[(u16, u16)]) -> Vec<Skin> {
    let all = DARK_PACK_SOURCES.iter().all(|g| packs.iter().any(|p| p.0 == *g));
    dm.into_iter().filter(|s| s.hair != 1 || all).collect()
}

/// The table's entries for chars 5 / 6 (the battle's Dark Riku) in Riku's look: the hair and the blade carry over, the attire is his
/// (0.198.36). The same rule as Riku's ([`group_for`]): his group with that (blade, hair); a missing one keeps the blade on Re:CoM's
/// hair; else his own group.
pub fn dark_entries(skins: &[Skin], look: Look) -> [u32; 2] {
    DARK_GROUPS.map(|d| group_for(skins, d, dark_look(look)) as u32)
}

/// Riku's look as the battle's Dark Riku wears it: his Dark Mode attire, the chosen blade and hair (Halloween Town is his group 211).
pub fn dark_look(look: Look) -> Look {
    Look { dark: true, halloween: false, ..look }
}

/// 0.198.36: Dark Riku's groups (201 / 211) with no group of their own for `look`'s (blade, hair), for the log (Re:CoM's hair with the
/// Soul Eater is his own group: never missing).
pub fn dark_missing(skins: &[Skin], look: Look) -> Vec<u16> {
    let l = dark_look(look);
    if (l.blade, l.hair) == (0, 0) {
        return vec![];
    }
    DARK_GROUPS.into_iter().filter(|&d| !skins.iter().any(|s| s.riku == d && s.dark && s.blade == l.blade && s.hair == l.hair)).collect()
}

/// The look a group shows (Riku's own: the Default Attire with the Soul Eater).
pub fn look_of(skins: &[Skin], group: u16) -> Look {
    match skins.iter().find(|s| s.skin == group) {
        Some(s) => Look { dark: s.dark, blade: s.blade, halloween: s.riku == RIKU_GROUPS[1], hair: s.hair },
        None => Look { halloween: group == RIKU_GROUPS[1], ..Look::default() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(g: u16, k: u16, n: &str) -> PlannedRow {
        PlannedRow { group: g, kind: k, name: RowName::Own(n.into()) }
    }

    /// Riku's look with a hair and the Soul Eater (the Default Attire).
    fn hl(hair: u8) -> Look {
        Look { hair, ..Look::default() }
    }

    /// 0.198.36 (fork owner, 2026-11-27: "no darkmode a keyblade tbm nn troca, já coloca essas regrs para o futuro"): the battle's Dark
    /// Riku takes Riku's chosen Keyblade with every hair, whatever Riku's attire (Default, Dark Mode, Halloween Town), by the rule of
    /// Riku's own looks; a missing (blade, hair) keeps the Keyblade on Re:CoM's hair and is reported; older manifests (no `blade`) read as
    /// the Soul Eater; his own look is never a row.
    #[test]
    fn the_battle_dark_riku_keeps_rikus_keyblade() {
        let rows: Vec<String> = [(205, 201, 2, 0), (206, 211, 2, 0), (207, 201, 1, 0), (208, 211, 1, 0), (209, 201, 0, 1), (210, 211, 0, 1), (212, 201, 2, 1), (213, 211, 2, 1), (216, 201, 1, 1), (217, 211, 1, 1)]
            .iter()
            .map(|(s, d, h, b)| format!(r#"{{"dark": {d}, "skin": {s}, "hair": {h}, "blade": {b}}}"#))
            .collect();
        let bad = r#"{"dark": 201, "skin": 218, "hair": 0, "blade": 0}, {"dark": 201, "skin": 219, "hair": 2, "blade": 1}, {"dark": 201, "skin": 220, "hair": 1, "blade": 2}, {"dark": 211, "skin": 209, "hair": 2, "blade": 1}"#;
        let text = format!(r#"{{"format": 1, "skins": [{{"riku": 101, "skin": 121, "dark": true, "blade": 0}}, {{"riku": 101, "skin": 161, "dark": true, "blade": 1}}], "dark_mode": [{}, {bad}]}}"#, rows.join(", "));
        let dm = dark_mode(&text);
        assert_eq!(dm.len(), 10, "his own look, a second (Dark Riku, hair, blade), a blade out of range and a group taken twice are left out: {dm:?}");
        let mut sk = parse(&text).unwrap();
        sk.extend(dm);
        let lk = |dark, blade, halloween, hair| Look { dark, blade, halloween, hair };
        for (dark, halloween) in [(false, false), (true, false), (false, true)] {
            assert_eq!(dark_entries(&sk, lk(dark, 0, halloween, 0)), [201, 211], "Soul Eater, Re:CoM's hair: his own");
            assert_eq!(dark_entries(&sk, lk(dark, 1, halloween, 0)), [209, 210], "Keyblade, Re:CoM's hair");
            assert_eq!(dark_entries(&sk, lk(dark, 0, halloween, 1)), [207, 208]);
            assert_eq!(dark_entries(&sk, lk(dark, 1, halloween, 1)), [216, 217], "Keyblade, KH2's hair");
            assert_eq!(dark_entries(&sk, lk(dark, 0, halloween, 2)), [205, 206]);
            assert_eq!(dark_entries(&sk, lk(dark, 1, halloween, 2)), [212, 213], "Keyblade, BBS's hair");
        }
        assert!(dark_missing(&sk, lk(true, 1, false, 2)).is_empty() && dark_missing(&sk, Look::default()).is_empty());
        // Riku's own chars 3 / 4 unchanged by his groups
        assert_eq!(entries(&sk, lk(true, 1, false, 0)), [161, 111]);
        // a missing combination keeps the Keyblade on Re:CoM's hair (Riku's rule), reported
        let no_216: Vec<Skin> = sk.iter().copied().filter(|s| s.skin != 216).collect();
        assert_eq!(dark_entries(&no_216, lk(false, 1, false, 1)), [209, 217]);
        assert_eq!(dark_missing(&no_216, lk(false, 1, false, 1)), [201]);
        let no_heart: Vec<Skin> = sk.iter().copied().filter(|s| s.riku < 200 || s.blade == 0).collect();
        assert_eq!(dark_entries(&no_heart, lk(false, 1, false, 2)), [201, 211], "no Keyblade group at all: his own");
        assert_eq!(dark_missing(&no_heart, lk(false, 1, false, 2)), [201, 211]);
        // an older manifest: no `blade`, the Soul Eater
        assert_eq!(dark_mode(r#"{"format": 1, "skins": [], "dark_mode": [{"dark": 201, "skin": 205, "hair": 2}]}"#), [Skin { riku: 201, skin: 205, dark: true, blade: 0, hair: 2 }]);
        // the KH2 rows (both blades) wait for the pack copies; the others do not
        assert_eq!(dark_mode_usable(dark_mode(&text), &[]).iter().map(|s| s.skin).collect::<Vec<_>>(), [205, 206, 209, 210, 212, 213]);
        assert_eq!(look_of(&sk, 216), lk(true, 1, false, 1));
    }

    /// 0.198.30: the battle's Dark Riku (chars 5 / 6) takes his hair group; Riku's chars 3 / 4 never do; his rows copy Dark Riku's.
    #[test]
    fn the_battle_dark_riku_takes_the_hair() {
        let text = r#"{"format": 1, "skins": [{"riku": 101, "skin": 121, "dark": true, "blade": 0}, {"riku": 101, "skin": 183, "dark": false, "blade": 0, "hair": 2}, {"riku": 101, "skin": 185, "dark": true, "blade": 0, "hair": 2}], "dark_mode": [{"dark": 201, "skin": 205, "hair": 2}, {"dark": 211, "skin": 206, "hair": 2}, {"dark": 201, "skin": 207, "hair": 2}, {"dark": 101, "skin": 208, "hair": 2}, {"dark": 211, "skin": 201, "hair": 1}, {"dark": 211, "skin": 209, "hair": 0}, {"dark": 201, "skin": 206, "hair": 1}]}"#;
        let dm = dark_mode(text);
        assert_eq!(dm, [Skin { riku: 201, skin: 205, dark: true, blade: 0, hair: 2 }, Skin { riku: 211, skin: 206, dark: true, blade: 0, hair: 2 }], "one group per Dark Riku and hair; only his groups; not onto Re:CoM's; a hair");
        assert!(dark_mode(r#"{"format": 1, "skins": []}"#).is_empty(), "older manifests: none");
        let mut all = parse(text).unwrap();
        all.extend(dm);
        assert_eq!(dark_entries(&all, hl(2)), [205, 206], "BBS kid hair");
        assert_eq!(dark_entries(&all, hl(0)), [201, 211], "Re:CoM's hair: his own groups");
        assert_eq!(dark_entries(&all, hl(1)), [201, 211], "KH2's hair: no group for him (139 bones), his own");
        // chars 3 / 4 are unchanged by his groups
        let lk = |dark, hair| Look { dark, blade: 0, halloween: false, hair };
        assert_eq!(entries(&all, lk(false, 2)), [183, 111]);
        assert_eq!(entries(&all, lk(true, 2)), [185, 111]);
        assert_eq!(entries(&all, lk(true, 0)), [121, 111]);
        assert_eq!(entries(&all, Look { halloween: true, ..lk(false, 2) }), [111, 111]);
        // his rows copy Dark Riku's (201's: no camera / effect link rows of Riku's), retyped to his group
        let mut t = Vec::new();
        for (g, n) in [(101, "PL0101"), (201, "PL0201"), (211, "PL0211")] {
            for (k, e) in [(0xC, "RTM"), (0x13, "TXA"), (0xB, "MDL"), (0x1D, "ANN"), (0x1A, "EET"), (0x30, "SES"), (0x2C, "SND")] {
                t.push(row(g, k, &format!("PLAYER/0{}/{n}.{e}", &n[3..])));
            }
        }
        t.push(row(END, 0, "end"));
        let out = add_rows(t, &all[1..].iter().filter(|s| s.skin != 185).copied().collect::<Vec<_>>()).unwrap();
        let ours: Vec<_> = out.iter().filter(|r| r.group == 205).collect();
        assert_eq!(ours.len(), 7);
        assert!(matches!(&ours.iter().find(|r| r.kind == 0xB).unwrap().name, RowName::Own(n) if n == "PLAYER/0205/PL0205.MDL"));
        assert!(matches!(&ours.iter().find(|r| r.kind == 0x2C).unwrap().name, RowName::Own(n) if n == "PLAYER/0201/PL0201.SND"), "Dark Riku's sound bank");
        assert!(matches!(&out.iter().find(|r| r.group == 206 && r.kind == 0x2C).unwrap().name, RowName::Own(n) if n == "PLAYER/0211/PL0211.SND"));
        assert_eq!(ann_name(205), "PL0205");
        assert_eq!(&DARK_ANN_NAMES[0][..6], ann_name(DARK_GROUPS[0]).as_bytes());
        assert_eq!(&DARK_ANN_NAMES[1][..6], ann_name(DARK_GROUPS[1]).as_bytes());
    }

    /// 0.198.32: the battle's Dark Riku takes KH2's hair group only with every copy of his 11 packs; the copies are looked up by id.
    #[test]
    fn the_battle_dark_riku_takes_the_kh2_hair_with_his_pack_copies() {
        let all: Vec<String> = DARK_PACK_SOURCES.iter().map(|g| format!("[{g}, {}]", g + 100)).collect();
        let text = format!(r#"{{"format": 1, "skins": [{{"riku": 101, "skin": 182, "dark": false, "blade": 0, "hair": 1}}], "dark_mode": [{{"dark": 201, "skin": 205, "hair": 2}}, {{"dark": 211, "skin": 206, "hair": 2}}, {{"dark": 201, "skin": 207, "hair": 1}}, {{"dark": 211, "skin": 208, "hair": 1}}], "dark_packs": [{}, [1092, 1192], [1087, 1999], [4083, 4003], [4002, 4102], [205, 1500]]}}"#, all.join(", "));
        let packs = dark_packs(&text);
        assert_eq!(packs, DARK_PACK_SOURCES.map(|g| (g, g + 100)), "his eleven only, each once, never onto a source or out of range");
        let dm = dark_mode_usable(dark_mode(&text), &packs);
        assert_eq!(dm.iter().map(|s| (s.riku, s.skin, s.hair)).collect::<Vec<_>>(), [(201, 205, 2), (211, 206, 2), (201, 207, 1), (211, 208, 1)]);
        let mut sk = parse(&text).unwrap();
        sk.extend(dm);
        assert_eq!(dark_entries(&sk, hl(1)), [207, 208], "KH2's hair");
        assert_eq!(dark_entries(&sk, hl(2)), [205, 206], "BBS's unchanged");
        assert_eq!(dark_entries(&sk, hl(0)), [201, 211], "Re:CoM's: his own");
        assert_eq!(entries(&sk, Look { dark: false, blade: 0, halloween: false, hair: 1 }), [182, 111], "Riku's chars 3 / 4 unchanged");
        // one copy missing: no KH2 group for him (a sleight would freeze), BBS's kept
        let some = dark_mode_usable(dark_mode(&text), &packs[1..]);
        assert_eq!(some.iter().map(|s| s.skin).collect::<Vec<_>>(), [205, 206]);
        assert!(dark_packs(r#"{"format": 1, "skins": []}"#).is_empty(), "older manifests: none");
        assert_eq!(event_alt_id(&packs, 11087), Some(11187));
        assert_eq!(event_alt_id(&packs, 14083), Some(14183));
        assert_eq!(event_alt_id(&packs, 11092), None, "Riku's own sleight: no copy");
        // their rows: one `.ANN` row each, after the event copies'
        let rows = vec![PlannedRow { group: 1087, kind: kind::ANN, name: RowName::Own("x".into()) }, PlannedRow { group: END, kind: 0, name: RowName::Own(String::new()) }];
        let t = add_event_rows(add_event_rows(rows, &[(4002, 4102)]).unwrap(), &packs).unwrap();
        assert_eq!(t.len(), 2 + 1 + 11);
        assert!(matches!(&t[2].name, RowName::Own(n) if n == "PLAYER/1187/PL1187.ANN"));
        assert!(matches!(&t[12].name, RowName::Own(n) if n == "PLAYER/4183/PL4183.ANN") && t[13].group == END);
    }

    #[test]
    fn skin_rows_copy_rikus_and_name_its_files() {
        let mut t = Vec::new();
        for (k, n) in [(0xC, "RTM"), (0x13, "TXA"), (0xB, "MDL"), (0x1D, "ANN"), (0x17, "BIN"), (0x1A, "EET"), (0x27, "31500"), (0x30, "SES"), (0x2C, "SND")] {
            t.push(row(101, k, &format!("PLAYER/0101/PL0101.{n}")));
        }
        t.push(row(201, 0xC, "x"));
        t.push(row(END, 0, "end"));
        let skins = parse(r#"{"format": 1, "skins": [{"riku": 101, "skin": 121}]}"#).unwrap();
        let out = add_rows(t.clone(), &skins).unwrap();
        assert_eq!(out.len(), t.len() + 9);
        assert_eq!(out.last().unwrap().group, END);
        let ours: Vec<_> = out.iter().filter(|r| r.group == 121).collect();
        assert_eq!(ours.len(), 9);
        let name = |k: u16| match &ours.iter().find(|r| r.kind == k).unwrap().name { RowName::Own(s) => s.clone(), RowName::Recom(_) => String::new() };
        assert_eq!(name(0xB), "PLAYER/0121/PL0121.MDL");
        assert_eq!(name(0x13), "PLAYER/0121/PL0121.TXA");
        assert_eq!(name(0x30), "PLAYER/0121/PL0121.SES");
        assert_eq!(name(0x17), "PLAYER/0101/PL0101.BIN", "Riku's camera data");
        assert_eq!(name(0x2C), "PLAYER/0101/PL0101.SND", "Riku's sound bank");
        assert!(add_rows(out, &skins).is_err(), "twice");
        assert_eq!(entries(&skins, Look { dark: true, blade: 0, halloween: false, hair: 0 }), [121, 111]);
        assert_eq!(entries(&skins, Look::default()), [101, 111]);
        assert_eq!(skins[0], Skin { riku: 101, skin: 121, dark: true, blade: 0, hair: 0 }, "an older manifest: Dark Mode, Soul Eater");
        let all = parse(r#"{"format": 1, "skins": [{"riku": 101, "skin": 121, "dark": true, "blade": 0}, {"riku": 101, "skin": 141, "dark": false, "blade": 1}, {"riku": 101, "skin": 161, "dark": true, "blade": 1}, {"riku": 111, "skin": 151, "dark": false, "blade": 1}]}"#).unwrap();
        assert_eq!(entries(&all, Look { dark: false, blade: 1, halloween: false, hair: 0 }), [141, 151]);
        assert_eq!(entries(&all, Look { dark: true, blade: 1, halloween: false, hair: 0 }), [161, 111], "no Halloween group for that look: Riku's own");
        assert_eq!(look_of(&all, 161), Look { dark: true, blade: 1, halloween: false, hair: 0 });
        // 0.178.0: the Halloween Town look puts char 3 on 111 and its skins.
        let hw = |dark, blade| Look { dark, blade, halloween: true, hair: 0 };
        assert_eq!(entries(&all, hw(false, 0)), [111, 111]);
        assert_eq!(entries(&all, hw(false, 1)), [151, 151]);
        assert_eq!(entries(&all, hw(true, 1)), [111, 111], "no Halloween Dark group in this manifest: Riku's own Halloween look");
        assert_eq!(look_of(&all, 111), hw(false, 0));
        assert_eq!(look_of(&all, 101), Look::default());
        assert_eq!(Look::from_bits(hw(true, 1).bits()), hw(true, 1));
        assert_eq!(Look::from_bits(Look { dark: true, blade: 1, halloween: false, hair: 0 }.bits()), Look { dark: true, blade: 1, halloween: false, hair: 0 });
        assert!(parse(r#"{"format": 1, "skins": [{"riku": 101, "skin": 141, "dark": false, "blade": 0}]}"#).is_err(), "Riku's own look");
        // KH2 hair groups (default attire, Soul Eater 182 / Keyblade of Heart 192); a look without one shows Re:CoM's.
        let hair = parse(r#"{"format": 1, "skins": [{"riku": 101, "skin": 121, "dark": true, "blade": 0}, {"riku": 101, "skin": 141, "dark": false, "blade": 1}, {"riku": 101, "skin": 182, "dark": false, "blade": 0, "hair": 1}, {"riku": 101, "skin": 192, "dark": false, "blade": 1, "hair": 1}]}"#).unwrap();
        let lk = |dark, blade, hair| Look { dark, blade, halloween: false, hair };
        assert_eq!(entries(&hair, lk(false, 0, 1)), [182, 111]);
        assert_eq!(entries(&hair, lk(false, 1, 1)), [192, 111]);
        assert_eq!(entries(&hair, lk(false, 0, 0)), [101, 111], "Re:CoM's hair: Riku's own group");
        assert_eq!(entries(&hair, lk(false, 1, 0)), [141, 111]);
        assert_eq!(entries(&hair, lk(true, 0, 1)), [121, 111], "Dark Mode has no KH2 hair: Re:CoM's");
        assert_eq!(look_of(&hair, 192), lk(false, 1, 1));
        assert_eq!(Look::from_bits(lk(false, 1, 1).bits()), lk(false, 1, 1));
        assert!(parse(r#"{"format": 1, "skins": [{"riku": 101, "skin": 182, "dark": false, "blade": 0, "hair": 1}, {"riku": 101, "skin": 183, "dark": false, "blade": 0, "hair": 1}]}"#).is_err(), "one group per look");
        // 0.194.0: BBS kid hair groups (183 / 193), hair value 2; KH2's (1) and Re:CoM's (0) keep their groups.
        let bbs = parse(r#"{"format": 1, "skins": [{"riku": 101, "skin": 182, "dark": false, "blade": 0, "hair": 1}, {"riku": 101, "skin": 183, "dark": false, "blade": 0, "hair": 2}, {"riku": 101, "skin": 193, "dark": false, "blade": 1, "hair": 2}]}"#).unwrap();
        assert_eq!(entries(&bbs, lk(false, 0, 2)), [183, 111]);
        assert_eq!(entries(&bbs, lk(false, 1, 2)), [193, 111]);
        assert_eq!(entries(&bbs, lk(false, 0, 1)), [182, 111]);
        assert_eq!(entries(&bbs, lk(false, 1, 1)), [101, 111], "no KH2 Keyblade group in this manifest: Riku's own");
        assert_eq!(entries(&bbs, lk(true, 0, 2)), [101, 111], "Dark Mode has no BBS hair: Riku's own group");
        assert_eq!(look_of(&bbs, 193), lk(false, 1, 2));
        assert_eq!(Look::from_bits(lk(false, 1, 2).bits()), lk(false, 1, 2));
        assert_eq!(ann_name(121), "PL0121");
        // 0.197.4: event pack copies
        let ev = event_packs(r#"{"format": 1, "skins": [], "event_packs": [[4002, 4102], [4012, 4112], [4002, 4199], [101, 4100], [4072, 4072]]}"#);
        assert_eq!(ev, [(4002, 4102), (4012, 4112)], "PLAYER event groups only, each once, not onto themselves");
        assert!(event_packs(r#"{"format": 1, "skins": []}"#).is_empty(), "older manifests: none");
        assert_eq!(event_alt_id(&ev, 14002), Some(14102));
        assert_eq!(event_alt_id(&ev, 14003), None, "Dark Riku's pack has no copy");
        assert_eq!(event_alt_id(&ev, 4002), None, "a group number is not an id");
        let rows = vec![PlannedRow { group: 4002, kind: kind::ANN, name: RowName::Own("x".into()) }, PlannedRow { group: END, kind: 0, name: RowName::Own(String::new()) }];
        let t = add_event_rows(rows.clone(), &ev).unwrap();
        assert_eq!(t.iter().map(|r| r.group).collect::<Vec<_>>(), [4002, 4102, 4112, END]);
        assert!(matches!(&t[1].name, RowName::Own(n) if n == "PLAYER/4102/PL4102.ANN") && t[1].kind == kind::ANN);
        assert!(add_event_rows(t, &ev).is_err(), "a group twice");
        assert!(files(121).contains(&"raw/PLAYER/0121/PL0121.RTM".to_string()));
        assert!(parse(r#"{"format": 1, "skins": [{"riku": 3, "skin": 121}]}"#).is_err());
        assert!(riku_guard(r#"{"format": 1, "skins": [], "riku_guard": true}"#));
        assert!(riku_cast(r#"{"format": 1, "skins": [], "riku_guard": false, "riku_cast": true}"#));
        assert!(!riku_cast(r#"{"format": 1, "skins": [], "riku_guard": true}"#), "older manifests: no cast clips");
        assert!(!riku_guard(r#"{"format": 1, "skins": [{"riku": 101, "skin": 121}]}"#), "older manifests: no guard");
    }
}
