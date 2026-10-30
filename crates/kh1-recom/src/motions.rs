//! The "KH1 motions test" switch (milestone M2, runtime half).
//!
//! Re:CoM plays Sora's motions through his action table (`0x1407C5E80`, reached through
//! `0x1407C6300[charIndex]`): 16-byte rows `{const char* name; int index; int pad}`. The player's
//! `stateInitial` (`0x1401FA6E0`) resolves every row's name to a motion-pack slot with
//! `0x1402A42B0` (an exact-name scan of the loaded `PL0001.ANN`, no slot limit), and the motion
//! request `0x1401F7180` plays `row.index`. So the switch only re-points the name of the rows
//! for idle, walk, run and the ground combo to the KH1 clips the asset build step appended to
//! `PL0001.ANN`; Re:CoM resolves and plays them itself. Checked (decomp + exe), see
//! `docs/recom-motion-switch.md`.
//!
//! This module is the testable part: the manifest, the `.ANN` header and the row plan.

use crate::json::{self, Value};

/// Address of Sora's action table (Steam). `0x1407C6300[1]` and `[2]` point here. Checked (exe).
pub const SORA_TABLE: u64 = 0x1_407C_5E80;
/// The per-character table pointer array. Checked (exe + decomp).
pub const CHAR_TABLES: u64 = 0x1_407C_6300;
/// Rows in Sora's table before the terminating null name (0x47). Checked (exe).
pub const SORA_ROWS: usize = 0x47;
/// Re:CoM's own slots in `PL0001.ANN`. Checked (files).
pub const RECOM_SLOTS: u32 = 161;

/// One row we re-point, and the KH1 motion it gets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Swap {
    /// Row (= action id) in Sora's table.
    pub row: u8,
    /// Re:CoM's own clip name in that row (checked against the exe before patching).
    pub recom_name: &'static str,
    /// What it is, for the log.
    pub what: &'static str,
    /// KH1 mset entries to look for, in order of preference.
    pub kh1_entries: &'static [u16],
    /// The KH1 request, used only if no entry matches and exactly one manifest slot has it.
    pub kh1_request: u16,
}

/// The rows the switch covers. Row → motion: Checked (decomp: `0x1401F9380` picks 1/2/0x12/3/0x13
/// in the field and 0x2A/0x2B/0x2C in battle stance; attack rows 1–4 of `0x1407C6C70` play
/// actions 0x30–0x33). KH1 entries: field 0/2/5 and battle 1/3/6 for idle/walk/run, swings
/// 98–101 (`docs/kh1-sora-data.md`, `motion-conversion.md`). Which KH1 stance variant matches
/// Re:CoM's field and battle rows is an Assumption; a field row falls back to the battle entry.
pub const SWAPS: &[Swap] = &[
    Swap { row: 0x01, recom_name: "p001_001", what: "field idle", kh1_entries: &[0, 1], kh1_request: 0 },
    Swap { row: 0x02, recom_name: "p001_002", what: "field walk", kh1_entries: &[2, 3], kh1_request: 1 },
    Swap { row: 0x12, recom_name: "p001_002", what: "field walk (alt row)", kh1_entries: &[2, 3], kh1_request: 1 },
    Swap { row: 0x03, recom_name: "p001_003", what: "field run", kh1_entries: &[5, 6], kh1_request: 2 },
    Swap { row: 0x13, recom_name: "p001_023", what: "field run (alt row)", kh1_entries: &[5, 6], kh1_request: 2 },
    Swap { row: 0x2A, recom_name: "p001_031", what: "battle idle", kh1_entries: &[1], kh1_request: 0 },
    Swap { row: 0x2B, recom_name: "p001_020", what: "battle walk", kh1_entries: &[3], kh1_request: 1 },
    Swap { row: 0x2C, recom_name: "p001_024", what: "battle run", kh1_entries: &[6], kh1_request: 2 },
    Swap { row: 0x30, recom_name: "p001_055", what: "ground swing 1", kh1_entries: &[98], kh1_request: 200 },
    Swap { row: 0x31, recom_name: "p001_056", what: "ground swing 2", kh1_entries: &[99], kh1_request: 201 },
    Swap { row: 0x32, recom_name: "p001_057", what: "ground lunge", kh1_entries: &[100], kh1_request: 202 },
    Swap { row: 0x33, recom_name: "p001_058", what: "ground finisher", kh1_entries: &[101], kh1_request: 203 },
];

/// One clip of the manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clip {
    pub slot: u32,
    pub name: String,
    pub entry: Option<u32>,
    pub request: Option<u32>,
}

/// Reads `kh1-motions.json`. Accepted shapes: a top-level array of clip objects, or an object
/// whose `slots` / `motions` / `clips` member is that array. Each clip needs `slot` or `name`
/// (`p001_161`), and may give `kh1_entry` / `entry` and `kh1_request` / `request`.
pub fn parse_manifest(text: &str) -> Result<Vec<Clip>, String> {
    let v = json::parse(text)?;
    let arr = match &v {
        Value::Arr(a) => a,
        Value::Obj(_) => match ["slots", "motions", "clips"].iter().find_map(|k| v.get(k)) {
            Some(Value::Arr(a)) => a,
            _ => return Err("no `slots` array".into()),
        },
        _ => return Err("not an object or array".into()),
    };
    let mut out = Vec::new();
    for (i, c) in arr.iter().enumerate() {
        let name = c.get("name").and_then(Value::as_str).map(str::to_string);
        let slot = c.get("slot").and_then(Value::as_u32).or_else(|| name.as_deref().and_then(slot_of_name));
        let Some(slot) = slot else {
            return Err(format!("clip {i}: no slot or name"));
        };
        let name = name.unwrap_or_else(|| format!("p001_{slot:03}"));
        if name.is_empty() || name.len() > 15 || !name.is_ascii() {
            return Err(format!("clip {i}: name `{name}` must be 1-15 ASCII characters"));
        }
        let pick = |a: &str, b: &str| c.get(a).or_else(|| c.get(b)).and_then(Value::as_u32);
        out.push(Clip { slot, name, entry: pick("kh1_entry", "entry"), request: pick("kh1_request", "request") });
    }
    Ok(out)
}

/// `p001_161` → 161 (the three trailing digits, as Re:CoM's `0x1401A47E0` reads them).
pub fn slot_of_name(name: &str) -> Option<u32> {
    let d = name.get(name.len().checked_sub(3)?..)?;
    if d.bytes().all(|b| b.is_ascii_digit()) {
        d.parse().ok()
    } else {
        None
    }
}

/// The `.ANN` header: `u32 count`, then `count × {u32 size; char name[16]}`. Checked (decomp
/// `0x14029F3A0` + files). Returns (size, name) per slot.
pub fn ann_header(bytes: &[u8]) -> Result<Vec<(u32, String)>, String> {
    let count = bytes.get(0..4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).ok_or("file too short")? as usize;
    if count > 4096 {
        return Err(format!("slot count {count} is not plausible"));
    }
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let o = 4 + i * 20;
        let e = bytes.get(o..o + 20).ok_or_else(|| format!("header cut at slot {i}"))?;
        let size = u32::from_le_bytes([e[0], e[1], e[2], e[3]]);
        let n = &e[4..20];
        let n = &n[..n.iter().position(|&b| b == 0).unwrap_or(16)];
        out.push((size, String::from_utf8_lossy(n).into_owned()));
    }
    Ok(out)
}

/// Bytes of the header for `count` slots (to read only that much of the file).
pub fn ann_header_len(count: u32) -> usize {
    4 + count as usize * 20
}

/// One planned row change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowPatch {
    pub row: u8,
    pub what: &'static str,
    pub recom_name: &'static str,
    pub kh1_name: String,
    pub slot: u32,
    pub entry: Option<u32>,
}

/// Plans the row changes. `table_names` is Sora's table as read from the game (row → name);
/// `ann` the header of the `PL0001.ANN` Panacea will serve. Every swap must be found, or nothing
/// is planned (all or nothing).
pub fn plan(manifest: &[Clip], table_names: &[Option<String>], ann: &[(u32, String)]) -> Result<Vec<RowPatch>, String> {
    let mut out = Vec::new();
    for s in SWAPS {
        let have = table_names.get(s.row as usize).cloned().flatten();
        if have.as_deref() != Some(s.recom_name) {
            return Err(format!("row {:#04x} ({}) is `{}`, expected `{}`", s.row, s.what, have.unwrap_or_default(), s.recom_name));
        }
        let by_entry = s.kh1_entries.iter().find_map(|&e| manifest.iter().find(|c| c.entry == Some(e as u32)));
        let clip = match by_entry {
            Some(c) => c,
            None => {
                let m: Vec<&Clip> = manifest.iter().filter(|c| c.request == Some(s.kh1_request as u32)).collect();
                match m.as_slice() {
                    &[c] => c,
                    [] => return Err(format!("manifest has no KH1 entry {:?} / request {} for {}", s.kh1_entries, s.kh1_request, s.what)),
                    _ => return Err(format!("manifest has several clips for KH1 request {} ({}) and no entry", s.kh1_request, s.what)),
                }
            }
        };
        if clip.slot < RECOM_SLOTS {
            return Err(format!("{}: slot {} would be one of Re:CoM's own", s.what, clip.slot));
        }
        match ann.get(clip.slot as usize) {
            Some((size, name)) if *size > 0 && *name == clip.name => {}
            Some((size, name)) => {
                return Err(format!("PL0001.ANN slot {} is `{name}` ({size} bytes), manifest says `{}`", clip.slot, clip.name))
            }
            None => return Err(format!("PL0001.ANN has {} slots, no slot {} for {}", ann.len(), clip.slot, s.what)),
        }
        out.push(RowPatch { row: s.row, what: s.what, recom_name: s.recom_name, kh1_name: clip.name.clone(), slot: clip.slot, entry: clip.entry });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> Vec<Option<String>> {
        let mut t = vec![Some("p001_xxx".to_string()); SORA_ROWS];
        for s in SWAPS {
            t[s.row as usize] = Some(s.recom_name.to_string());
        }
        t
    }

    fn ann(n: u32) -> Vec<(u32, String)> {
        (0..n).map(|i| (100, format!("p001_{i:03}"))).collect()
    }

    const MANIFEST: &str = r#"{"slots": [
        {"slot": 161, "name": "p001_161", "kh1_entry": 0, "kh1_request": 0},
        {"slot": 162, "kh1_entry": 1},
        {"slot": 163, "kh1_entry": 2, "kh1_request": 1},
        {"slot": 164, "kh1_entry": 3},
        {"slot": 165, "kh1_entry": 5, "kh1_request": 2},
        {"slot": 166, "kh1_entry": 6},
        {"name": "p001_167", "entry": 98, "request": 200},
        {"slot": 168, "kh1_entry": 99, "kh1_request": "0xC9"},
        {"slot": 169, "kh1_entry": 100},
        {"slot": 170, "kh1_entry": 101}
    ]}"#;

    #[test]
    fn manifest_shapes() {
        let m = parse_manifest(MANIFEST).unwrap();
        assert_eq!(m.len(), 10);
        assert_eq!(m[1], Clip { slot: 162, name: "p001_162".into(), entry: Some(1), request: None });
        assert_eq!(m[6], Clip { slot: 167, name: "p001_167".into(), entry: Some(98), request: Some(200) });
        assert_eq!(m[7].request, Some(201));
        let arr = parse_manifest(r#"[{"slot": 170}]"#).unwrap();
        assert_eq!(arr[0].name, "p001_170");
        assert!(parse_manifest(r#"{"x": 1}"#).is_err());
        assert!(parse_manifest(r#"[{"kh1_entry": 1}]"#).is_err());
        assert!(parse_manifest(r#"[{"name": "a_name_that_is_too_long"}]"#).is_err());
    }

    #[test]
    fn slot_names() {
        assert_eq!(slot_of_name("p001_161"), Some(161));
        assert_eq!(slot_of_name("p001_05x"), None);
        assert_eq!(slot_of_name("ab"), None);
    }

    #[test]
    fn ann_header_reads_names() {
        let mut b = 3u32.to_le_bytes().to_vec();
        for (s, n) in [(10u32, "p001_000"), (0, ""), (5, "p001_161_long16c")] {
            b.extend_from_slice(&s.to_le_bytes());
            let mut name = [0u8; 16];
            name[..n.len()].copy_from_slice(n.as_bytes());
            b.extend_from_slice(&name);
        }
        let h = ann_header(&b).unwrap();
        assert_eq!(h, vec![(10, "p001_000".into()), (0, String::new()), (5, "p001_161_long16c".into())]);
        assert_eq!(ann_header_len(3), b.len());
        assert!(ann_header(&b[..30]).is_err());
    }

    #[test]
    fn plans_every_row() {
        let m = parse_manifest(MANIFEST).unwrap();
        let p = plan(&m, &table(), &ann(171)).unwrap();
        assert_eq!(p.len(), SWAPS.len());
        let row = |r: u8| p.iter().find(|x| x.row == r).unwrap();
        assert_eq!(row(0x01).kh1_name, "p001_161");
        assert_eq!(row(0x2A).kh1_name, "p001_162");
        assert_eq!(row(0x12).kh1_name, "p001_163");
        assert_eq!(row(0x13).kh1_name, "p001_165");
        assert_eq!(row(0x30).kh1_name, "p001_167");
        assert_eq!(row(0x33).slot, 170);
    }

    #[test]
    fn field_rows_fall_back_to_battle_entries() {
        let m: Vec<Clip> = parse_manifest(MANIFEST).unwrap().into_iter().filter(|c| ![Some(0), Some(2), Some(5)].contains(&c.entry)).collect();
        let p = plan(&m, &table(), &ann(171)).unwrap();
        assert_eq!(p.iter().find(|x| x.row == 0x01).unwrap().kh1_name, "p001_162");
    }

    #[test]
    fn refuses_incomplete_or_mismatched_data() {
        let m = parse_manifest(MANIFEST).unwrap();
        // ANN too short (assets not built or not delivered).
        assert!(plan(&m, &table(), &ann(161)).is_err());
        // ANN slot named differently.
        let mut a = ann(171);
        a[167].1 = "other".into();
        assert!(plan(&m, &table(), &a).is_err());
        // Empty slot.
        let mut a = ann(171);
        a[170].0 = 0;
        assert!(plan(&m, &table(), &a).is_err());
        // A table row that isn't Re:CoM's (another mod, or another build).
        let mut t = table();
        t[0x30] = Some("p001_999".into());
        assert!(plan(&m, &t, &ann(171)).is_err());
        // Missing swing.
        let m2: Vec<Clip> = m.iter().filter(|c| c.entry != Some(101)).cloned().collect();
        assert!(plan(&m2, &table(), &ann(171)).is_err());
        // A manifest slot inside Re:CoM's range.
        let mut m3 = m.clone();
        m3[9].slot = 58;
        m3[9].name = "p001_058".into();
        assert!(plan(&m3, &table(), &ann(171)).is_err());
    }

    #[test]
    fn swaps_are_unique_rows_inside_the_table() {
        let mut rows: Vec<u8> = SWAPS.iter().map(|s| s.row).collect();
        rows.sort();
        rows.dedup();
        assert_eq!(rows.len(), SWAPS.len());
        assert!(rows.iter().all(|&r| (r as usize) < SORA_ROWS));
    }
}
