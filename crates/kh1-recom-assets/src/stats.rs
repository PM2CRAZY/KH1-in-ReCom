//! KH1 stats for the runtime (decisions 12, 18, 21): each Re:CoM enemy's KH1 twin record per
//! Re:CoM world, and Sora's KH1 Strength by level. Written as `kh1-stats.json` next to the motion
//! manifest. Everything is read from the player's own KH1 files; this module only knows where
//! to look (file names, script offsets, table offsets). Notes: `docs/recom-kh1-attack.md`.
//!
//! Where the numbers live (Checked (decomp + files), research notes `kh1-enemy-stats.md`):
//! - An ordinary Heartless's AI is part 5 of its own `.mdls` (`0x140290D60`, binder
//!   `0x140287E40`). At init it hands a 0x6C-byte record from its own script data to AI syscall
//!   45 (`0x1402BBDF0` → `0x1402A58E0`), chosen by the LV2 story bit (syscall 198) and, for the
//!   Shadow, by world (syscall 105). The script offsets below were found by evaluating each
//!   script for every world and both LV values.
//! - Record: r0 HP, r1 / r2 Strength / Defence index into btltbl `0xC8A8` (21 rows of
//!   {Strength, Defence}), r3 / r4 damage floor / cap (u16), r5 level, r7 EXP, r10..r25
//!   resistances (%).
//! - Sora: new-game template btltbl `0x39D8` (Strength at +3), Dive choice deltas
//!   `0x128 + 4 × choice` (keep Rod, give up Shield = choice 5), level bonuses `0x3AC0` (bonus 4 =
//!   Strength +2), Kingdom Key Strength from its weapon row (+0x30).

use crate::{Error, Result};

/// KH1 world codes (`0x1404DCCA0`) for Re:CoM world ids 1..13, in the order of Re:CoM's world name
/// table (`0x140506AA0`): Traverse Town, Agrabah, Olympus Coliseum, Wonderland, Monstro,
/// Halloween Town, Atlantica, Neverland, Hollow Bastion, 100 Acre Wood, Twilight Town, Destiny
/// Islands, Castle Oblivion. `None`: KH1 has no such world, so the LV2 record is used (owner,
/// decision 21). Assumption: Re:CoM's world id `0x14087B844` is that order + 1 (to check in game).
pub const RECOM_WORLDS: [(&str, Option<&str>); 13] = [
    ("Traverse Town", Some("tw")),
    ("Agrabah", Some("al")),
    ("Olympus Coliseum", Some("he")),
    ("Wonderland", Some("aw")),
    ("Monstro", Some("pi")),
    ("Halloween Town", Some("nm")),
    ("Atlantica", Some("lm")),
    ("Neverland", Some("pp")),
    ("Hollow Bastion", Some("pc")),
    ("100 Acre Wood", Some("po")),
    ("Twilight Town", None),
    ("Destiny Islands", Some("di")),
    ("Castle Oblivion", None),
];

/// How a script picks its first-visit (LV1) record.
#[derive(Clone, Copy, Debug)]
pub enum Lv1 {
    /// One offset everywhere.
    Fixed(u32),
    /// By KH1 world code; worlds not listed use the fallback.
    ByWorld(&'static [(&'static str, u32)], u32),
}

/// A Re:CoM enemy class and its KH1 twin.
#[derive(Clone, Copy, Debug)]
pub struct Twin {
    /// Re:CoM's class (its RTTI name; the runtime knows each class's vtable).
    pub class: &'static str,
    /// KH1's model file (its part 5 is the AI script).
    pub model: &'static str,
    /// The LV1 record.
    pub lv1: Lv1,
    /// The LV2 record (the same as LV1 for single-record scripts).
    pub lv2: u32,
    /// The pairing comes from internal names only (Assumption).
    pub assumed: bool,
}

const SHADOW_WORLDS: &[(&str, u32)] = &[
    ("dh", 0x14),
    ("di", 0xEC),
    ("tw", 0x158),
    ("aw", 0x1C4),
    ("he", 0x230),
    ("tz", 0x29C),
    ("al", 0x3E0),
    ("pi", 0x44C),
    ("nm", 0x590),
    ("pp", 0x5FC),
    ("pc", 0x6D4),
    ("dc", 0x740),
    ("po", 0x740),
    ("lm", 0x740),
];

const fn lv(class: &'static str, model: &'static str, assumed: bool) -> Twin {
    Twin { class, model, lv1: Lv1::Fixed(0x14), lv2: 0x80, assumed }
}
const fn one(class: &'static str, model: &'static str, assumed: bool) -> Twin {
    Twin { class, model, lv1: Lv1::Fixed(0x14), lv2: 0x14, assumed }
}

/// Re:CoM's KH1-equivalent enemies. Re:CoM-only ones (and KH1 ones not identified: Tornado
/// Step) keep Re:CoM's numbers.
pub const TWINS: &[Twin] = &[
    Twin { class: "CEnemyShadow", model: "xa_ex_2020.mdls", lv1: Lv1::ByWorld(SHADOW_WORLDS, 0x740), lv2: 0x740, assumed: false },
    lv("CEnemySoldier", "xa_ex_2010.mdls", false),
    lv("CEnemyLargebody", "xa_ex_2050.mdls", false),
    lv("CEnemyRednocturne", "xa_ex_2110.mdls", false),
    lv("CEnemyBluerhapsody", "xa_ex_2120.mdls", false),
    lv("CEnemyYellowopera", "xa_ex_2130.mdls", false),
    lv("CEnemyGreenrequiem", "xa_ex_2140.mdls", false),
    lv("CEnemyPowerwild", "xa_ex_2030.mdls", false),
    lv("CEnemyBouncywild", "xa_ex_2040.mdls", false),
    lv("CEnemyAirsoldier", "xa_ex_2160.mdls", false),
    lv("CEnemyBandit", "xa_ex_2090.mdls", false),
    lv("CEnemyFatbandit", "xa_ex_2060.mdls", false),
    lv("CEnemyBarrelspider", "xa_ex_2170.mdls", true),
    lv("CEnemySearchghost", "xa_ex_2230.mdls", false),
    lv("CEnemyPirate", "xa_ex_2100.mdls", false),
    lv("CEnemyAirpirate", "xa_ex_2210.mdls", false),
    lv("CEnemyWightnight", "xa_ex_2200.mdls", false),
    lv("CEnemyGargoyle", "xa_ex_2220.mdls", false),
    lv("CEnemyDarkball", "xa_ex_2280.mdls", false),
    lv("CEnemyDefender", "xa_ex_2340.mdls", false),
    lv("CEnemyWizard", "xa_ex_2150.mdls", false),
    lv("CEnemySeaneon", "xa_ex_2070.mdls", true),
    lv("CEnemyAquatank", "xa_ex_2080.mdls", true),
    lv("CEnemyScrewdiver", "xa_ex_2240.mdls", true),
    lv("CEnemyWyvern", "xa_ex_2320.mdls", true),
    one("CEnemyWhitemushroom", "xa_ex_2350.mdls", false),
    one("CEnemyBlackfungus", "xa_ex_2380.mdls", false),
    one("CEnemyCreapplant", "xa_ex_2290.mdls", true),
    one("CEnemyNeoshadow", "xa_ex_2420.mdls", true),
    one("CEnemySpadesoldier", "xa_aw_1030.mdls", false),
    one("CEnemyHeartsoldier", "xa_aw_1060.mdls", false),
];

/// One record, resolved (Strength / Defence through btltbl's index table).
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    /// r0.
    pub hp: i32,
    /// btltbl `0xC8A8[r1].0`.
    pub strength: i32,
    /// btltbl `0xC8A8[r2].1`.
    pub defence: i32,
    /// r3, r4 (u16).
    pub floor: u16,
    /// Damage cap (0 = none).
    pub cap: u16,
    /// r5.
    pub level: i32,
    /// r7.
    pub exp: i32,
    /// r10..r25, %.
    pub resist: [i32; 16],
}

/// Part `k` of a `.mdls` (an i32 count, then count + 1 offsets).
pub fn mdls_part(d: &[u8], k: usize) -> Option<&[u8]> {
    let n = i32::from_le_bytes(d.get(0..4)?.try_into().ok()?) as usize;
    if !(k + 1..=64).contains(&n) {
        return None;
    }
    let off = |i: usize| -> Option<usize> { Some(i32::from_le_bytes(d.get(4 + 4 * i..8 + 4 * i)?.try_into().ok()?) as usize) };
    let (a, b) = (off(k)?, off(k + 1)?);
    d.get(a..b)
}

/// The radii (KH1 units) of the model's kind-0 lock points, in file order: what KH1's ground
/// picker subtracts from the distance to each point (`0x1402A7320` → `0x1402B0110` iterates the
/// kind-0 points, `0x1402B04C0` returns point `+0xC` × actor `+0x40`). They are the collision
/// block's (`.mdls` part 4, `CLS\x03`, counts at `+4..+7`, 0x14-byte entries from `+0x20`) last
/// group; an entry's kind is byte 0 & 0xF. Checked (decomp + files).
pub fn lock_radii(mdls: &[u8]) -> Result<Vec<f32>> {
    let bad = |w: &str| Error::Format(format!("collision block: {w}"));
    let c = mdls_part(mdls, 4).ok_or_else(|| bad("no part 4"))?;
    if c.get(0..4) != Some(b"CLS\x03".as_slice()) {
        return Err(bad("not CLS\\x03"));
    }
    let n = |i: usize| c[4 + i] as usize;
    let first = 0x20 + (n(0) + n(1) + n(2)) * 0x14;
    let mut out = Vec::new();
    for i in 0..n(3) {
        let e = c.get(first + i * 0x14..first + (i + 1) * 0x14).ok_or_else(|| bad("lock points past the end"))?;
        if e[0] & 0xF == 0 {
            out.push(f32::from_le_bytes(e[0xC..0x10].try_into().expect("4 bytes")));
        }
    }
    if out.is_empty() {
        return Err(bad("no kind-0 lock point"));
    }
    Ok(out)
}

/// Reads one record from a script.
pub fn record(script: &[u8], at: u32, btltbl: &[u8]) -> Result<Record> {
    let at = at as usize;
    let r = |i: usize| -> Option<i32> { Some(i32::from_le_bytes(script.get(at + 4 * i..at + 4 * i + 4)?.try_into().ok()?)) };
    let bad = || Error::Format(format!("no stats record at script offset {at:#x}"));
    let idx = |i: usize| -> Result<(u8, u8)> {
        let v = r(i).ok_or_else(bad)?;
        if !(0..=20).contains(&v) {
            return Err(Error::Format(format!("record at {at:#x}: level index {v} outside 0..20")));
        }
        let o = 0xC8A8 + 2 * v as usize;
        Ok((*btltbl.get(o).ok_or_else(bad)?, *btltbl.get(o + 1).ok_or_else(bad)?))
    };
    let hp = r(0).ok_or_else(bad)?;
    if hp <= 0 {
        return Err(Error::Format(format!("record at {at:#x}: HP {hp}")));
    }
    let mut resist = [0; 16];
    for (k, x) in resist.iter_mut().enumerate() {
        *x = r(10 + k).ok_or_else(bad)?;
    }
    Ok(Record {
        hp,
        strength: idx(1)?.0 as i32,
        defence: idx(2)?.1 as i32,
        floor: (r(3).ok_or_else(bad)? & 0xFFFF) as u16,
        cap: (r(4).ok_or_else(bad)? & 0xFFFF) as u16,
        level: r(5).ok_or_else(bad)?,
        exp: r(7).ok_or_else(bad)?,
        resist,
    })
}

/// The record offset for a Re:CoM world (1-based index into [`RECOM_WORLDS`]).
pub fn offset_for(t: &Twin, recom_world: usize) -> u32 {
    match RECOM_WORLDS.get(recom_world.wrapping_sub(1)).and_then(|w| w.1) {
        None => t.lv2,
        Some(code) => match t.lv1 {
            Lv1::Fixed(o) => o,
            Lv1::ByWorld(list, fallback) => list.iter().find(|(c, _)| *c == code).map_or(fallback, |x| x.1),
        },
    }
}

/// Sora's KH1 Strength for levels 1..=100 (index 0 = level 1), keep Rod / give up Shield
/// (decision 18): template + choice delta + Kingdom Key + 2 per bonus-4 level.
pub fn sora_strength(btltbl: &[u8], keyblade_strength: i32) -> Result<Vec<i32>> {
    let b = |o: usize| btltbl.get(o).copied().ok_or_else(|| Error::Format(format!("btltbl too short for {o:#x}")));
    const TEMPLATE: usize = 0x39D8; // Sora's row: HP, MP, AP, Strength, Defence, …
    const CHOICE: usize = 0x128 + 4 * 5; // keep Rod, give up Shield: (Str, Def, MP, AP) deltas
    const BONUS: usize = 0x3AC0; // bonus for reaching level L at [L − 2]
    let mut s = b(TEMPLATE + 3)? as i32 + b(CHOICE)? as i8 as i32 + keyblade_strength;
    let mut out = vec![s];
    for l in 2..=100usize {
        if b(BONUS + l - 2)? == 4 {
            s += 2;
        }
        out.push(s);
    }
    Ok(out)
}

fn rec_json(r: &Record) -> String {
    let res: Vec<String> = r.resist.iter().map(|x| x.to_string()).collect();
    format!(
        "{{\"hp\": {}, \"strength\": {}, \"defence\": {}, \"floor\": {}, \"cap\": {}, \"level\": {}, \"exp\": {}, \"resist\": [{}]}}",
        r.hp,
        r.strength,
        r.defence,
        r.floor,
        r.cap,
        r.level,
        r.exp,
        res.join(", ")
    )
}

/// Builds `kh1-stats.json`. `read` gives a KH1 file by name.
pub fn build_json(read: &mut dyn FnMut(&str) -> Result<Vec<u8>>, btltbl: &[u8], keyblade_strength: i32) -> Result<(String, usize)> {
    let strength = sora_strength(btltbl, keyblade_strength)?;
    let mut s = String::from("{\n  \"format\": \"kh1-recom-assets stats 2\",\n");
    s.push_str(&format!("  \"sora_strength\": [{}],\n", strength.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", ")));
    s.push_str(&format!(
        "  \"worlds\": [{}],\n  \"enemies\": [\n",
        RECOM_WORLDS.iter().map(|w| format!("\"{}\"", w.0)).collect::<Vec<_>>().join(", ")
    ));
    let mut n = 0;
    for (i, t) in TWINS.iter().enumerate() {
        let mdls = read(t.model)?;
        let script = mdls_part(&mdls, 5).ok_or_else(|| Error::Format(format!("{}: no AI script (part 5)", t.model)))?;
        let mut worlds = Vec::new();
        for w in 1..=RECOM_WORLDS.len() {
            let r = record(script, offset_for(t, w), btltbl).map_err(|e| Error::Format(format!("{}: {e}", t.model)))?;
            worlds.push(format!("      {}", rec_json(&r)));
        }
        let radii = lock_radii(&mdls).map_err(|e| Error::Format(format!("{}: {e}", t.model)))?;
        s.push_str(&format!(
            "    {{\"class\": \"{}\", \"kh1_model\": \"{}\", \"assumed\": {}, \"lock_radius\": {}, \"lock_radii\": [{}], \"worlds\": [\n{}\n    ]}}{}\n",
            t.class,
            t.model,
            t.assumed,
            radii[0],
            radii.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(", "),
            worlds.join(",\n"),
            if i + 1 < TWINS.len() { "," } else { "" }
        ));
        n += 1;
    }
    s.push_str("  ]\n}\n");
    Ok((s, n))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn btl() -> Vec<u8> {
        let mut b = vec![0u8; 0xD000];
        // Index rows: row i = (i + 2, i + 1).
        for i in 0..21 {
            b[0xC8A8 + 2 * i] = (i + 2) as u8;
            b[0xC8A8 + 2 * i + 1] = (i + 1) as u8;
        }
        b[0x39D8 + 3] = 4;
        b[0x128 + 20] = 0; // keep Rod, give up Shield: Strength +0
        b[0x3AC0] = 4; // level 2: Strength +2
        b[0x3AC0 + 1] = 5;
        b[0x3AC0 + 2] = 4; // level 4
        b
    }

    #[test]
    fn lock_radii_are_the_kind_0_points_of_part_4() {
        // Part table: 6 parts, part 4 at 0x40.
        let mut d = vec![0u8; 0x40];
        d[0..4].copy_from_slice(&6i32.to_le_bytes());
        for i in 0..=6 {
            let at = if i <= 4 { 0x40 } else { 0x40 + 0x20 + 5 * 0x14 };
            d[4 + 4 * i..8 + 4 * i].copy_from_slice(&(at as i32).to_le_bytes());
        }
        let mut c = vec![0u8; 0x20 + 5 * 0x14];
        c[0..4].copy_from_slice(b"CLS\x03");
        c[4..8].copy_from_slice(&[1, 1, 0, 3]);
        let lock = 0x20 + 2 * 0x14;
        for (i, (kind, r)) in [(0u8, 30f32), (2, 10.0), (0, 40.0)].iter().enumerate() {
            c[lock + i * 0x14] = *kind;
            c[lock + i * 0x14 + 0xC..lock + i * 0x14 + 0x10].copy_from_slice(&r.to_le_bytes());
        }
        d.extend_from_slice(&c);
        assert_eq!(lock_radii(&d).unwrap(), vec![30.0, 40.0]);
        d[0x40] = b'X';
        assert!(lock_radii(&d).is_err());
    }

    #[test]
    fn strength_by_level() {
        let s = sora_strength(&btl(), 3).unwrap();
        assert_eq!(&s[..4], &[7, 9, 9, 11]);
        assert_eq!(s.len(), 100);
    }

    #[test]
    fn shadow_record_per_world_and_lv2_for_unknown_worlds() {
        let shadow = &TWINS[0];
        assert_eq!(offset_for(shadow, 1), 0x158); // Traverse Town
        assert_eq!(offset_for(shadow, 2), 0x3E0); // Agrabah
        assert_eq!(offset_for(shadow, 10), 0x740); // 100 Acre Wood (po)
        assert_eq!(offset_for(shadow, 11), 0x740); // Twilight Town: LV2
        let soldier = &TWINS[1];
        assert_eq!(offset_for(soldier, 1), 0x14);
        assert_eq!(offset_for(soldier, 13), 0x80); // Castle Oblivion: LV2
    }

    #[test]
    fn reads_a_record_from_part_5() {
        let mut script = vec![0u8; 0x100];
        let rec: [i32; 27] = [24, 3, 3, 3, 300, 8, 0x3F80_0000, 5, 5021, 0x3F80_0000, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 0];
        for (i, v) in rec.iter().enumerate() {
            script[0x14 + 4 * i..0x18 + 4 * i].copy_from_slice(&v.to_le_bytes());
        }
        let mut mdls = Vec::new();
        mdls.extend_from_slice(&6i32.to_le_bytes());
        let base = 4 + 7 * 4;
        for i in 0..7 {
            let o: i32 = if i <= 5 { base } else { base + script.len() as i32 };
            mdls.extend_from_slice(&o.to_le_bytes());
        }
        mdls.extend_from_slice(&script);
        let p = mdls_part(&mdls, 5).unwrap();
        let r = record(p, 0x14, &btl()).unwrap();
        assert_eq!((r.hp, r.strength, r.defence, r.floor, r.cap, r.level, r.exp), (24, 5, 4, 3, 300, 8, 5));
        assert!(record(p, 0x1C, &btl()).is_err(), "a misaligned record fails the index check");
    }
}
