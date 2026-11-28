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

/// 0.185.5: an entry `name` (a Guard clip of Riku's or Dark Riku's) with a whoosh and a grunt at frame 0, as Sora's
/// Guard has ([`add_kh1`]): the whoosh (group 1) copied from `whoosh_clip`'s events, the grunt (group 3, random from
/// its weighted table, flag 4) from `voice_clip`'s, both of the same actor's own SES (Checked (files): `PL0101.SES`
/// `p002_044` 1333 and the grunts 3011 / 3014 / 3013; `PL0201.SES` `p003_056` 1332, `p003_053` 3005 / 3006).
pub fn guard_entry(entries: &[Entry], name: &str, whoosh_clip: &str, voice_clip: &str) -> Result<Entry> {
    let whoosh = event_of(entries, whoosh_clip, |e| group(e) == 1)?;
    let voice = event_of(entries, voice_clip, |e| group(e) == 3 && flags(e) & 4 != 0)?;
    let mut head = entries.iter().find(|e| e.name() == voice_clip).map(|e| e.head).ok_or_else(|| Error::Format(format!("SES: no {voice_clip}")))?;
    head[..0x20].fill(0);
    head[..name.len().min(15)].copy_from_slice(&name.as_bytes()[..name.len().min(15)]);
    Ok(Entry { head, events: vec![with(whoosh, None, 0), with(voice, None, 0)] })
}

// ---- M9 stage 9a: the KH1-only combo modifier clips ----

/// The combo modifiers' KH1 mset entries, and whether their row is a finisher (row voice group 5 /
/// 6) or a swing (4) (btltbl rows 0..15, Checked (files)): Sliding Dash 106, Hurricane Blast 107,
/// Blitz 108, Vortex 109, Aerial Sweep 112, Ripple Drive 113, Stun Impact 114, Gravity Break 115,
/// Zantetsuken 116. Re:CoM already has 106 / 114 / 116 (its own clips keep its own sounds); the
/// rest are KH1-only clips and get entries here.
pub const MODIFIER_ENTRIES: [(usize, bool); 9] = [(106, false), (107, true), (108, true), (109, false), (112, false), (113, true), (114, true), (115, true), (116, true)];

/// One sound cue of a KH1 motion: its sound track (type 6, sub 0) keys, `0x140295420`'s ops.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    /// Op 0x19: the weapon's swing sound, the argument within the weapon's set.
    Whoosh(u16),
    /// Ops 0..=7: a footstep.
    Step,
}

/// The cues of a KH1 motion, frame (rounded down, 60 Hz as SES frames) and kind. Voice ops
/// (0x1A..0x1D) are left to the row's voice (below).
pub fn kh1_cues(m: &kh1_formats::mset::Motion) -> Vec<(i32, Cue)> {
    let mut out = Vec::new();
    let Some(t) = m.tracks.iter().find(|t| t.kind == kh1_formats::mset::track::SOUND && t.sub == 0) else { return out };
    for k in &t.keys {
        let op = u16::from_le_bytes([k.payload[0], k.payload[1]]);
        let arg = u16::from_le_bytes([k.payload[2], k.payload[3]]);
        match op {
            0x19 => out.push((k.time as i32, Cue::Whoosh(arg))),
            0..=7 => out.push((k.time as i32, Cue::Step)),
            _ => {}
        }
    }
    out
}

/// Re:CoM's whoosh (sound group 1, index) for KH1's swing sound `arg`: the pairs Re:CoM's own swing
/// clips give (KH1 entries 98–104 → `p001_055`–`p001_061`: args 7, 8, 9, 10, 11, 12, 17 → 232,
/// 233, 234, 235, 237, 236, 238; Checked (files)). Re:CoM has no whoosh for KH1's other args
/// (Assumption: none of its swing clips plays one): a finisher row takes the ground finisher's
/// (235), a swing row swing 1's (232).
pub fn whoosh_index(arg: u16, finisher: bool) -> u32 {
    match arg {
        7 => 232,
        8 => 233,
        9 => 234,
        10 => 235,
        11 => 237,
        12 => 236,
        17 => 238,
        _ if finisher => 235,
        _ => 232,
    }
}

/// One KH1-only clip to give sounds: its name in the extended pack, its KH1 cues, and whether its
/// row is a finisher.
#[derive(Clone, Debug, PartialEq)]
pub struct ClipSounds {
    /// The clip's name in the extended pack (`p001_NNN`).
    pub name: String,
    /// KH1's cues ([`kh1_cues`]).
    pub cues: Vec<(i32, Cue)>,
    /// The row is a finisher (voice group 5 / 6).
    pub finisher: bool,
}

/// Adds entries for the KH1-only modifier clips (owner, 2026-10-30: Re:CoM's sounds for now): each
/// KH1 swing sound as Re:CoM's whoosh ([`whoosh_index`]) at KH1's frame, each KH1 footstep as
/// Re:CoM's step 1 at its frame (Assumption: KH1's step codes aren't mapped), and the row's voice at
/// 0 (KH1 starts the row's voice group with the row, `0x1402901F0`): the finisher grunts of
/// `p001_058` for finishers, the swing grunts of `p001_055` for swings. Re:CoM's own entries stay
/// byte-identical; an entry of the same name is replaced.
pub fn add_clips(entries: &mut Vec<Entry>, clips: &[ClipSounds]) -> Result<usize> {
    if clips.is_empty() {
        return Ok(0);
    }
    let whoosh = event_of(entries, "p001_060", |e| group(e) == 1)?;
    let swing_voice = event_of(entries, "p001_055", |e| group(e) == 2 && flags(e) & 4 != 0)?;
    let finisher_voice = event_of(entries, "p001_058", |e| group(e) == 2 && flags(e) & 4 != 0)?;
    let step1 = event_of(entries, "p001_055", |e| flags(e) & 1 != 0 && u32::from_le_bytes(e[4..8].try_into().unwrap()) == 1)?;
    let head = entries.iter().find(|e| e.name() == "p001_055").map(|e| e.head).ok_or_else(|| Error::Format("SES: no p001_055".into()))?;
    for c in clips {
        let mut events = Vec::new();
        for &(frame, cue) in &c.cues {
            events.push(match cue {
                Cue::Whoosh(arg) => with(whoosh, Some(whoosh_index(arg, c.finisher)), frame),
                Cue::Step => with(step1, None, frame),
            });
        }
        events.push(with(if c.finisher { finisher_voice } else { swing_voice }, None, 0));
        entries.retain(|e| e.name() != c.name);
        let mut h = head;
        h[..16].fill(0);
        let n = c.name.len().min(15);
        h[..n].copy_from_slice(&c.name.as_bytes()[..n]);
        entries.push(Entry { head: h, events });
    }
    Ok(clips.len())
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
    fn modifier_clips_take_recoms_whooshes_at_kh1s_frames() {
        let mut e = recom();
        let clips = vec![
            // Blitz (entry 108): KH1 swing sound 26 at 0, footsteps at 12 and 34; a finisher row.
            ClipSounds { name: "p001_223".into(), cues: vec![(0, Cue::Whoosh(26)), (12, Cue::Step), (34, Cue::Step)], finisher: true },
            // Vortex (entry 109): swing sounds 13 at 6 and 10 at 12; a swing row.
            ClipSounds { name: "p001_224".into(), cues: vec![(6, Cue::Whoosh(13)), (12, Cue::Whoosh(10))], finisher: false },
        ];
        assert_eq!(add_clips(&mut e, &clips).unwrap(), 2);
        let back = parse(&write(&e)).unwrap();
        let ev = |n: &str| -> Vec<(u32, u32, i32)> {
            back.iter().find(|x| x.name() == n).unwrap().events.iter().map(|x| (group(x), u32::from_le_bytes(x[4..8].try_into().unwrap()), i32::from_le_bytes(x[0x18..0x1C].try_into().unwrap()))).collect()
        };
        assert_eq!(ev("p001_223"), vec![(1, 235, 0), (601, 1, 12), (601, 1, 34), (2, 0, 0)]);
        assert_eq!(ev("p001_224"), vec![(1, 232, 6), (1, 235, 12), (2, 0, 0)]);
        // Re:CoM's entries untouched; a second run replaces.
        for (a, b) in back[..4].iter().zip(recom().iter()) {
            assert_eq!((a.name(), &a.events), (b.name(), &b.events));
        }
        add_clips(&mut e, &clips).unwrap();
        assert_eq!(e.len(), 6);
        assert_eq!(add_clips(&mut e, &[]).unwrap(), 0);
    }

    /// The player's own files, when given (`KH1_TEST_DATA` holding Re:CoM's `PL0011.SES` and KH1's
    /// `xa_ex_0010.mset`): the modifier clips' entries are added to Re:CoM's real file, which
    /// round-trips with Re:CoM's entries unchanged.
    #[test]
    fn modifier_entries_on_the_players_files() {
        let Ok(dir) = std::env::var("KH1_TEST_DATA") else { return };
        let dir = std::path::PathBuf::from(dir);
        let (Ok(ses), Ok(mset)) = (std::fs::read(dir.join("PL0011.SES")), std::fs::read(dir.join("xa_ex_0010.mset"))) else { return };
        let set = kh1_formats::mset::MotionSet::parse(&mset).unwrap();
        let mut e = parse(&ses).unwrap();
        let n0 = e.len();
        let clips: Vec<ClipSounds> = MODIFIER_ENTRIES.iter().map(|&(entry, finisher)| ClipSounds { name: format!("p001_{}", 115 + entry), cues: kh1_cues(&set.motions[entry]), finisher }).collect();
        // Blitz (108): KH1's swing sound 26 at 0, steps at 12 and 34 (Checked (files)).
        assert_eq!(clips[2].cues, vec![(0, Cue::Whoosh(26)), (12, Cue::Step), (34, Cue::Step)]);
        // Ripple Drive (113): a step at 19, its swing sound 8 at 20 (→ Re:CoM 233).
        assert_eq!(clips[5].cues[..2], [(19, Cue::Step), (20, Cue::Whoosh(8))]);
        assert_eq!(add_clips(&mut e, &clips).unwrap(), 9);
        let back = parse(&write(&e)).unwrap();
        assert_eq!(back.len(), n0 + 9);
        let orig = parse(&ses).unwrap();
        for (a, b) in back[..n0].iter().zip(orig.iter()) {
            assert_eq!((a.name(), &a.events), (b.name(), &b.events));
        }
    }

    #[test]
    fn whooshes_follow_recoms_own_swing_clips() {
        assert_eq!([7, 8, 9, 10, 11, 12, 17].map(|a| whoosh_index(a, false)), [232, 233, 234, 235, 237, 236, 238]);
        assert_eq!((whoosh_index(33, true), whoosh_index(28, false)), (235, 232));
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
