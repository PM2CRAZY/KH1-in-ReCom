//! The six Keyblades Re:CoM has and KH1 lacks come from KH2 FM (decision 25): KH2's model,
//! KH2's Strength and Magic, a reach measured from the KH2 model's blade, and the Kingdom Key's
//! KH1 radius, critical rate / bonus and priority. Everything is read from the player's own KH2
//! files (`kh2_*.hed` / `.pkg`). Notes: `docs/recom-keyblade-models.md` § 12.
//!
//! Where the numbers live (`kh2-formats-lite`, ported from KH2-Combat-in-KH1):
//! - `03system.bin` → `item`: 24-byte rows from `+8`, found by id (`0x1403E39A0` walks them
//!   comparing the u16 at `+0`). A Keyblade is type 2 (`+2`); `+4` is its weapon number; for
//!   types 2..0xD the u16 at `+6` is its stat id (`0x1403E3EF0`). The stat rows follow the items
//!   (`0x1403E39E0`: `items + 8 + count × 0x18`, `u32 count` at `+4`, 16-byte rows from `+8`),
//!   found by bsearch on their u16 id (`0x14041E8F0`). A stat row's bytes `+4 / +5 / +6 / +7`
//!   are what the equipment screen's compares (`0x1402FCC80`, `0x1402FD050`, `0x1402FCD30`,
//!   `0x1402FCBD0`) set against a character's `+0x188 / +0x18A / +0x18C / +0x18E`: Strength,
//!   Magic, Defence, AP in OpenKH's names. Checked (decomp, KH2 Steam).
//! - The item's name: `+8`, a message id in `msg/us/sys.bar`.
//! - Its model: base Sora's weapon objects in `03system.bin` → `went` (word 1 = the list's
//!   offset in words, then one object id per weapon number), and `00objentry.bin` (the object's
//!   model name). From KH2-Combat-in-KH1's `kh1-hook` `model.rs`; Checked (files: the Kingdom
//!   Key, Oathkeeper, Oblivion and Ultima Weapon resolve to `W_EX010`, `_10`, `_20`, `_N0`); the
//!   exe's use of `went` is not traced (Assumption beyond that).

use kh2_formats_lite::{bar::Bar, msg::Messages, objentry, skeleton::Bone, system};

use crate::{bad, Result};

/// Re:CoM's Keyblade card kinds that come from KH2, with Re:CoM's (and KH2's) English names.
/// The kinds are the order of Re:CoM's Keyblade names at `0x1405060F0` (Soul Eater 0x12 is
/// not for Sora). Checked (exe).
pub const KH2_KEYBLADES: [(u8, &str); 6] = [
    (0x13, "Star Seeker"),
    (0x14, "Monochrome"),
    (0x15, "Follow the Wind"),
    (0x16, "Hidden Dragon"),
    (0x17, "Photon Debugger"),
    (0x18, "Bond of Flame"),
];

/// The KH2 message file holding the item names (US English; the names are matched in it).
/// Checked (files: in the owner's Steam `kh2_*` archives; `msg/jp/sys.bar` is not).
pub const SYS_BAR: &str = "msg/us/sys.bar";

/// KH2's Kingdom Key, the reach reference and the skeleton every KH2 Keyblade must share.
pub const KINGDOM_KEY: &str = "Kingdom Key";

/// KH2's item type of a Keyblade (`item` `+2`). Checked (files, decomp `0x1403E3EF0`).
pub const ITEM_KEYBLADE: u8 = 2;

/// One KH2 Keyblade as the files give it.
#[derive(Clone, Debug, PartialEq)]
pub struct Kh2Keyblade {
    /// Re:CoM card kind (0 for the Kingdom Key reference).
    pub card: u8,
    /// Its English name (KH2's `sys.bar`).
    pub name: String,
    /// KH2 item id.
    pub item: u16,
    /// Its stat row id (the item's u16 at `+6`).
    pub stat: u16,
    /// Stat row `+4` / `+5`.
    pub strength: u8,
    /// See `strength`.
    pub magic: u8,
    /// Its model (`W_EX010_30`); the file is `obj/<model>.mdlx`.
    pub model: String,
}

impl Kh2Keyblade {
    /// The archive path of its model.
    pub fn file(&self) -> String {
        format!("obj/{}.mdlx", self.model)
    }
}

/// Finds KH2's Kingdom Key and the six Keyblades of [`KH2_KEYBLADES`] by name in the player's
/// `03system.bin`, `00objentry.bin` and `msg/us/sys.bar`. Returns the Kingdom Key first. Each
/// must be a Keyblade item with a stat row and a `W_EX010*` model; else an error.
pub fn find(system_bin: &[u8], objentry_bin: &[u8], sys_bar: &[u8]) -> Result<Vec<Kh2Keyblade>> {
    let e = |w: &str| crate::Error::Format(format!("KH2: {w}"));
    let items = system::parse_items(system_bin).map_err(|x| e(&x.to_string()))?;
    let objects = objentry::parse_objentry(objentry_bin).map_err(|x| e(&x.to_string()))?;
    let msg = Messages::parse(sys_bar).map_err(|x| e(&x.to_string()))?;
    let bar = Bar::parse(system_bin).map_err(|x| e(&x.to_string()))?;
    let went = bar.get("went").ok_or_else(|| e("03system.bin has no went"))?.data;
    let item_entry = bar.get("item").ok_or_else(|| e("03system.bin has no item"))?.data;
    let word = |i: usize| went.get(i * 4..i * 4 + 4).map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")));
    let list = word(1).ok_or_else(|| e("went too short"))? as usize;
    let mut out = Vec::new();
    for (card, name) in std::iter::once((0u8, KINGDOM_KEY)).chain(KH2_KEYBLADES) {
        let mut found: Vec<Kh2Keyblade> = Vec::new();
        for (i, it) in items.items.iter().enumerate() {
            if it.kind != ITEM_KEYBLADE || !msg.text(it.name as u32).is_some_and(|n| n.eq_ignore_ascii_case(name)) {
                continue;
            }
            // The game reads the stat id as the u16 at `+6` (stat byte, AP byte).
            let at = 8 + i * 24 + 6;
            let stat = item_entry.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]])).ok_or_else(|| e("item row cut short"))?;
            let Some(row) = items.stats.iter().find(|s| s.id == stat) else { continue };
            let Some(obj) = word(list + it.number as usize).filter(|o| *o != 0) else { continue };
            let Some(model) = objects.iter().find(|o| o.id == obj).map(|o| o.model.clone()).filter(|m| m.starts_with("W_EX010")) else { continue };
            found.push(Kh2Keyblade { card, name: name.to_string(), item: it.id, stat, strength: row.strength, magic: row.magic, model });
        }
        // An item a second time (KH2 has a few Keyblade rows twice, e.g. a nameless Kingdom
        // Key 0x51) must agree with the first.
        let Some(first) = found.first().cloned() else {
            return bad(format!("KH2: no Keyblade item named {name:?} with a stat row and a W_EX010 model"));
        };
        if let Some(other) = found.iter().find(|k| (k.strength, k.magic, &k.model) != (first.strength, first.magic, &first.model)) {
            return bad(format!("KH2: {name:?} twice with different numbers: {first:?} / {other:?}"));
        }
        out.push(first);
    }
    Ok(out)
}

/// The blade's length (metres): how far the model reaches from its blade bone (bone 2, Re:CoM's
/// `wep0`) along the blade, i.e. `−min x` of its bone-2 vertices × 0.01. The keychain hangs on
/// bones 3..11 along `+x` (bone 3 sits at `+x` 21.1), the blade runs along `−x`. On KH2's
/// Kingdom Key it is 0.7790 m, Re:CoM's PL0002 the same. Checked (files).
pub fn blade_length(mdlx: &[u8]) -> Result<f32> {
    let m = kh2_formats_lite::mdlx::parse(mdlx).map_err(|x| crate::Error::Format(format!("KH2 model: {x}")))?;
    let min_x = m.groups.iter().flat_map(|g| g.triangles.iter().flatten()).filter(|v| v.bone == 2).map(|v| v.position[0]).fold(f32::INFINITY, f32::min);
    if !min_x.is_finite() || min_x >= 0.0 {
        return bad("KH2 model: no blade on bone 2 along -x");
    }
    Ok(-min_x * 0.01)
}

/// A KH2 Keyblade's KH1 blade parameter (weapon row `+0x24`, `blade_far`): the Kingdom Key's
/// KH1 value scaled by the two KH2 blade lengths (decision 25: reach from the KH2 model's
/// blade). Why it holds: KH1's own rows follow their models' blades the same way (each KH1
/// Keyblade's `blade_far` is within 0.13 of its tip's distance over `|wep0 → k9|` but Olympia's,
/// e.g. Kingdom Key 3.69 vs 3.7, Ultima Weapon 5.52 vs 5.6), and KH2's Kingdom Key reaches as
/// far as KH1's (3.691 × `|wep0 → k9|`). Checked (files).
pub fn blade_far(kk_blade_far: f32, length: f32, kk_length: f32) -> f32 {
    kk_blade_far * length / kk_length
}

/// The skeleton every KH2 Keyblade must have: KH2's Kingdom Key's.
pub fn reference_skeleton(kingdom_key_mdlx: &[u8]) -> Result<Vec<Bone>> {
    kh2_formats_lite::skeleton::parse_skeleton(kingdom_key_mdlx).map_err(|x| crate::Error::Format(format!("KH2 Kingdom Key: {x}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reach_scales_the_kingdom_keys() {
        assert_eq!(blade_far(-3.7, 0.7790, 0.7790), -3.7);
        assert!((blade_far(-3.7, 1.0345, 0.7790) - -4.9135).abs() < 1e-3);
        assert_eq!(KH2_KEYBLADES.iter().map(|k| k.0).collect::<Vec<_>>(), (0x13..=0x18).collect::<Vec<_>>());
    }
}
