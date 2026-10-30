//! Re:CoM's per-motion sound events (`PLAYER/0001/PL0001.SES`) for the KH1-only clips.
//!
//! Format (Checked (files + decomp: loader `0x1402387E0`, start `0x140239260`)): `"@SES"`, u32
//! version 1, u32 entry count, u32 entry table offset (0x10); 0x30-byte entries (+0 motion name,
//! +0x20 i32 motion index = −1 in the file and bound at load by name to the actor's motion pack,
//! +0x24 event count, +0x28 event offset); 0x50-byte events (+0 sound group, +4 index (id = group
//! × 1000 + index), +0x18 start frame (60 Hz, as KH1's), +0x1C stop (−1), +0x24 fade, +0x28 flags:
//! 1 footstep, 4 random from the +0x40 table). Re:CoM plays a clip's events itself when the clip
//! starts through `0x1401F7180` (Sora's vtable +0x70 `0x1401A3470`), so the new clips get sounds
//! natively by name.
//!
//! The added events copy Re:CoM's own events from its swing clips and change only the id and the
//! frame (owner, 2026-10-30: Re:CoM's sounds for now):
//! - Counterattack (`KH1 entry 111`): KH1's whoosh weapon + 0xB = Re:CoM 1237 at 0 (as `p001_060`),
//!   the finisher grunts {10, 11, 12} at 0 (as `p001_058`), steps at 46 and 80 (KH1's codes 0 / 5).
//! - Guard (`KH1 entry 110`): Re:CoM has no whoosh for KH1's weapon + 0xD nor KH1's guard voice; the
//!   nearest whoosh 1236 (weapon + 0xC) at 0 and the swing grunts of `p001_055` at 0.

use crate::{Error, Result};

const ENTRY: usize = 0x30;
const EVENT: usize = 0x50;

/// One entry: its 0x30-byte head and its events.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// The 0x30-byte entry (name, motion index, event count and offset; the last three are
    /// rewritten by [`write`]).
    pub head: [u8; ENTRY],
    /// Its 0x50-byte events.
    pub events: Vec<[u8; EVENT]>,
}

impl Entry {
    /// The motion name (`p001_055`).
    pub fn name(&self) -> String {
        let end = self.head[..16].iter().position(|&c| c == 0).unwrap_or(16);
        String::from_utf8_lossy(&self.head[..end]).into_owned()
    }
}

fn u32_at(d: &[u8], o: usize) -> Result<u32> {
    d.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).ok_or_else(|| Error::Format(format!("SES: short at {o:#x}")))
}

/// Reads a `.SES` file.
pub fn parse(d: &[u8]) -> Result<Vec<Entry>> {
    if d.get(0..4) != Some(b"@SES") || u32_at(d, 4)? != 1 {
        return Err(Error::Format("SES: not an @SES version 1 file".into()));
    }
    let n = u32_at(d, 8)? as usize;
    let off = u32_at(d, 12)? as usize;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let at = off + ENTRY * i;
        let head: [u8; ENTRY] = d.get(at..at + ENTRY).ok_or_else(|| Error::Format("SES: entry outside the file".into()))?.try_into().unwrap();
        let count = u32_at(&head, 0x24)? as usize;
        let eo = u32_at(&head, 0x28)? as usize;
        let mut events = Vec::with_capacity(count);
        for k in 0..count {
            let e = eo + EVENT * k;
            events.push(d.get(e..e + EVENT).ok_or_else(|| Error::Format("SES: event outside the file".into()))?.try_into().unwrap());
        }
        out.push(Entry { head, events });
    }
    Ok(out)
}

/// Writes entries back: header, entries, then every entry's events in order (Re:CoM's layout).
pub fn write(entries: &[Entry]) -> Vec<u8> {
    let mut d = Vec::new();
    d.extend_from_slice(b"@SES");
    d.extend_from_slice(&1u32.to_le_bytes());
    d.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    d.extend_from_slice(&0x10u32.to_le_bytes());
    let mut ev = 0x10 + ENTRY * entries.len();
    for e in entries {
        let mut h = e.head;
        h[0x20..0x24].copy_from_slice(&(-1i32).to_le_bytes());
        h[0x24..0x28].copy_from_slice(&(e.events.len() as u32).to_le_bytes());
        h[0x28..0x2C].copy_from_slice(&(if e.events.is_empty() { 0 } else { ev as u32 }).to_le_bytes());
        d.extend_from_slice(&h);
        ev += EVENT * e.events.len();
    }
    for e in entries {
        for x in &e.events {
            d.extend_from_slice(x);
        }
    }
    d
}

fn event_of(entries: &[Entry], clip: &str, pick: impl Fn(&[u8; EVENT]) -> bool) -> Result<[u8; EVENT]> {
    entries
        .iter()
        .find(|e| e.name() == clip)
        .and_then(|e| e.events.iter().find(|x| pick(x)).copied())
        .ok_or_else(|| Error::Format(format!("SES: {clip} lacks the template event")))
}

fn group(e: &[u8; EVENT]) -> u32 {
    u32::from_le_bytes(e[0..4].try_into().unwrap())
}
fn flags(e: &[u8; EVENT]) -> u32 {
    u32::from_le_bytes(e[0x28..0x2C].try_into().unwrap())
}

fn with(mut e: [u8; EVENT], index: Option<u32>, frame: i32) -> [u8; EVENT] {
    if let Some(i) = index {
        e[4..8].copy_from_slice(&i.to_le_bytes());
    }
    e[0x18..0x1C].copy_from_slice(&frame.to_le_bytes());
    e
}

/// Adds entries for the Guard and Counterattack clips (`guard` / `counter`: their clip names in
/// the extended pack). Re:CoM's own entries stay byte-identical; an existing entry of the same
/// name (an earlier build) is replaced.
pub fn add_kh1(entries: &mut Vec<Entry>, guard: Option<&str>, counter: Option<&str>) -> Result<usize> {
    let whoosh = event_of(entries, "p001_060", |e| group(e) == 1)?;
    let swing_voice = event_of(entries, "p001_055", |e| group(e) == 2 && flags(e) & 4 != 0)?;
    let finisher_voice = event_of(entries, "p001_058", |e| group(e) == 2 && flags(e) & 4 != 0)?;
    let step1 = event_of(entries, "p001_055", |e| flags(e) & 1 != 0 && u32::from_le_bytes(e[4..8].try_into().unwrap()) == 1)?;
    let step5 = event_of(entries, "p001_055", |e| flags(e) & 1 != 0 && u32::from_le_bytes(e[4..8].try_into().unwrap()) == 5)?;
    let head = entries.iter().find(|e| e.name() == "p001_055").map(|e| e.head).ok_or_else(|| Error::Format("SES: no p001_055".into()))?;
    let mut added = 0;
    let mut put = |name: &str, events: Vec<[u8; EVENT]>| {
        entries.retain(|e| e.name() != name);
        let mut h = head;
        h[..16].fill(0);
        h[..name.len().min(15)].copy_from_slice(&name.as_bytes()[..name.len().min(15)]);
        entries.push(Entry { head: h, events });
        added += 1;
    };
    if let Some(n) = counter {
        put(n, vec![with(whoosh, Some(237), 0), with(finisher_voice, None, 0), with(step1, None, 46), with(step5, None, 80)]);
    }
    if let Some(n) = guard {
        put(n, vec![with(whoosh, Some(236), 0), with(swing_voice, None, 0)]);
    }
    Ok(added)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(group: u32, index: u32, frame: i32, flags: u32) -> [u8; EVENT] {
        let mut e = [0u8; EVENT];
        e[0..4].copy_from_slice(&group.to_le_bytes());
        e[4..8].copy_from_slice(&index.to_le_bytes());
        e[0x18..0x1C].copy_from_slice(&frame.to_le_bytes());
        e[0x1C..0x20].copy_from_slice(&(-1i32).to_le_bytes());
        e[0x28..0x2C].copy_from_slice(&flags.to_le_bytes());
        e
    }

    fn entry(name: &str, events: Vec<[u8; EVENT]>) -> Entry {
        let mut head = [0u8; ENTRY];
        head[..name.len()].copy_from_slice(name.as_bytes());
        head[0x13] = 0x51;
        Entry { head, events }
    }

    fn recom() -> Vec<Entry> {
        vec![
            entry("p001_055", vec![ev(601, 1, 19, 1), ev(601, 5, 44, 1), ev(1, 232, 1, 0), ev(2, 0, 5, 4)]),
            entry("p001_058", vec![ev(1, 235, 10, 0), ev(2, 0, 4, 4)]),
            entry("p001_060", vec![ev(1, 237, 2, 0), ev(2, 0, 0, 4)]),
            entry("p001_081", vec![ev(601, 1, 8, 1)]),
        ]
    }

    #[test]
    fn round_trip_is_byte_identical() {
        let d = write(&recom());
        let back = parse(&d).unwrap();
        assert_eq!(write(&back), d);
        assert_eq!(back[3].name(), "p001_081");
    }

    #[test]
    fn kh1_entries_copy_recoms_events_and_take_kh1s_frames() {
        let mut e = recom();
        assert_eq!(add_kh1(&mut e, Some("p001_225"), Some("p001_226")).unwrap(), 2);
        let d = write(&e);
        let back = parse(&d).unwrap();
        let c = back.iter().find(|x| x.name() == "p001_226").unwrap();
        let frames: Vec<(u32, u32, i32)> = c.events.iter().map(|x| (group(x), u32::from_le_bytes(x[4..8].try_into().unwrap()), i32::from_le_bytes(x[0x18..0x1C].try_into().unwrap()))).collect();
        assert_eq!(frames, vec![(1, 237, 0), (2, 0, 0), (601, 1, 46), (601, 5, 80)]);
        let g = back.iter().find(|x| x.name() == "p001_225").unwrap();
        assert_eq!((group(&g.events[0]), u32::from_le_bytes(g.events[0][4..8].try_into().unwrap())), (1, 236));
        assert_eq!(g.head[0x13], 0x51, "the rest of the head is Re:CoM's");
        // Re:CoM's entries are unchanged, and a second run replaces rather than duplicates.
        for (a, b) in back[..4].iter().zip(recom().iter()) {
            assert_eq!((a.name(), &a.events, &a.head[..0x20]), (b.name(), &b.events, &b.head[..0x20]));
        }
        add_kh1(&mut e, Some("p001_225"), Some("p001_226")).unwrap();
        assert_eq!(e.len(), 6);
    }
}
