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

/// One skin: Riku's group and the group that stands for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Skin {
    pub riku: u16,
    pub skin: u16,
}

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
        if !RIKU_GROUPS.contains(&riku) || out.iter().any(|o: &Skin| o.riku == riku || o.skin == skin) {
            return Err(format!("kh1-riku-skin.json: skin {skin} for group {riku}"));
        }
        out.push(Skin { riku, skin });
    }
    Ok(out)
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

/// The table's entries for chars 3 and 4 with or without the skin.
pub fn entries(skins: &[Skin], on: bool) -> [u32; 2] {
    RIKU_GROUPS.map(|g| if on { skins.iter().find(|s| s.riku == g).map_or(g, |s| s.skin) as u32 } else { g as u32 })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(g: u16, k: u16, n: &str) -> PlannedRow {
        PlannedRow { group: g, kind: k, name: RowName::Own(n.into()) }
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
        assert_eq!(entries(&skins, true), [121, 111]);
        assert_eq!(entries(&skins, false), [101, 111]);
        assert_eq!(ann_name(121), "PL0121");
        assert!(files(121).contains(&"raw/PLAYER/0121/PL0121.RTM".to_string()));
        assert!(parse(r#"{"format": 1, "skins": [{"riku": 3, "skin": 121}]}"#).is_err());
    }
}
