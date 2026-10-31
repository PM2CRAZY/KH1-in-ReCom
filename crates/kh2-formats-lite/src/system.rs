//! `03system.bin` → `item`: the item rows (24 bytes) and, after them, the
//! equipment stat rows (16 bytes). Layouts follow OpenKH's docs and its
//! `OpenKh.Kh2.SystemData` readers (Apache-2.0); ported from
//! KH2-Combat-in-KH1's `kh2-formats/src/system.rs` (Apache-2.0), commands
//! left out. The byte offsets each field is read from are listed on
//! [`parse_items`].

use crate::bar::Bar;
use crate::reader::Reader;
use crate::{bad, Result};

/// Item types (OpenKH's names).
pub mod item_type {
    pub const CONSUMABLE: u8 = 0;
    pub const KEYBLADE: u8 = 2;
    pub const ARMOR: u8 = 14;
    pub const ACCESSORY: u8 = 15;
    pub const MAGIC: u8 = 18;
    pub const ABILITY: u8 = 19;
    pub const SUMMON: u8 = 20;
    pub const FORM: u8 = 21;
}

/// One `item` row (24 bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub id: u16,
    pub kind: u8,
    /// For abilities: the ability number the game's tables use (PTYA rows
    /// name their required ability by it; Guard = 1, Upper Slash = 18).
    pub number: u8,
    /// Index into the stat table (equipment).
    pub stat: u16,
    /// AP cost (abilities).
    pub ap: u8,
    /// Message ids (sys.bar) of the name and description.
    pub name: u16,
    pub description: u16,
    pub command: u16,
}

/// One equipment stat row (16 bytes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ItemStat {
    pub id: u16,
    /// Ability item id the equipment grants (0 = none).
    pub ability: u16,
    pub strength: u8,
    pub magic: u8,
    pub defense: u8,
    pub ap: u8,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ItemTable {
    pub items: Vec<Item>,
    pub stats: Vec<ItemStat>,
}

impl ItemTable {
    pub fn get(&self, id: u16) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }
}

fn entry<'a>(system: &'a [u8], name: &str) -> Result<&'a [u8]> {
    let bar = Bar::parse(system)?;
    match bar.entries.iter().find(|e| e.name == name) {
        Some(e) => Ok(e.data),
        None => bad(format!("03system.bin has no {name}")),
    }
}

/// Reads `item`. In the BAR entry: `u32 version, u32 count`, then `count` item rows of 24
/// bytes from `+8`: `+0 id u16, +2 type, +3 flag, +4 number (Keyblades: the weapon number),
/// +5 rank, +6 stat row index, +7 AP, +8 name message u16, +0xA description, +0xC buy, +0xE
/// sell, +0x10 command`. Then at `8 + count × 24`: `u32 version, u32 count` and the stat rows,
/// 16 bytes from `+8`: `+0 id u16, +2 ability u16, +4 Strength, +5 Magic, +6 Defence, +7 AP`.
pub fn parse_items(system: &[u8]) -> Result<ItemTable> {
    let d = entry(system, "item")?;
    let mut r = Reader::new(d);
    let _version = r.u32()?;
    let count = r.u32()? as usize;
    let mut items = Vec::with_capacity(count);
    for i in 0..count {
        let mut e = Reader::at(d, 8 + i * 24);
        let id = e.u16()?;
        let kind = e.u8()?;
        let _flag0 = e.u8()?;
        let number = e.u8()?;
        let _rank = e.u8()?;
        let stat = e.u8()? as u16;
        let ap = e.u8()?;
        let name = e.u16()?;
        let description = e.u16()?;
        let _buy = e.u16()?;
        let _sell = e.u16()?;
        let command = e.u16()?;
        items.push(Item { id, kind, number, stat, ap, name, description, command });
    }
    let so = 8 + count * 24;
    let mut stats = Vec::new();
    if so + 8 <= d.len() {
        let mut r = Reader::at(d, so);
        let _version = r.u32()?;
        let n = r.u32()? as usize;
        for i in 0..n {
            let mut e = Reader::at(d, so + 8 + i * 16);
            stats.push(ItemStat {
                id: e.u16()?,
                ability: e.u16()?,
                strength: e.u8()?,
                magic: e.u8()?,
                defense: e.u8()?,
                ap: e.u8()?,
            });
        }
    }
    Ok(ItemTable { items, stats })
}
