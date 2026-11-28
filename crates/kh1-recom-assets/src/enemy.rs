//! M11: the shared asset step for KH1's enemies in Re:CoM's battles (`docs/recom-m11-enemies.md`).
//! From the player's own files, for one enemy ([`Spec`]), it makes Re:CoM's `ENEMY/NNNN` files able
//! to play the KH1 twin natively:
//!
//! - `ENNNNN.ANN`: KH1's motions (`.mset` on the `.mdls` skeleton) as new 60 fps clips at slot
//!   100 + KH1 entry (`eNNN_1EE`; Re:CoM's name lookup `0x1401A47E0` takes the slot from the
//!   name's digits); Re:CoM's slots stay byte-identical; an entry Re:CoM has as an exact bake
//!   plays Re:CoM's clip (decision M10).
//! - `ENNNNN.ASD`: Re:CoM's attack records (`@COP` v3, 0xA0 each; loader `0x140179340`, bound by
//!   motion name at load `0x14014DBE0`, ticked by `0x14014E830`) with one new record per KH1
//!   attack row: the row's hit slot window (row `+0x34` picks the `.mset` HIT track's sub:
//!   `0x1402B2DC0` writes it to the object's bits 4..5, the hit test `0x1402B0000.c` reads
//!   `0x10000 << sub`; Checked (decomp)), its points (bone code → bone, `t` along it) in Re:CoM's
//!   frame of the bone's parent, its radius (× the actor scale 1: Assumption), the Re:CoM attack id
//!   it takes its power from (`@ENP`, decision M10).
//! - `ENNNNN.SES`: Re:CoM's entries kept; each new clip KH1's matcher pairs with a Re:CoM clip
//!   that has sounds gets those (same frames); the other attack clips get their template record's
//!   sounds at the same place relative to the hit window (a frame-0 sound stays at 0). Assumption:
//!   KH1's own enemy sounds are not mapped (decision 22: Re:CoM's sounds).
//! - `kh1-<enemy>.json` (`kh1-enemy 1`): every number of the behaviour (read from the player's
//!   script at checked offsets, `btltbl.bin`'s globals, the stat records, KH1's exe numbers), the
//!   attacks (request, clash recoil), the hurt parts' Re:CoM bones, the request → slot map.
//!
//! The Shadow (M10) keeps its own step (`crate::shadow`, unchanged output) on the helpers here.

use kh1_formats::mset::MotionSet;

use crate::ann::{name16, Ann, Slot};
use crate::build::{cycle_error, LOOP_TOLERANCE_M};
use crate::convert::{ClipSpec, Converter};
use crate::matching::{decode_pack, match_all, MatchKind, Query};
use crate::mdl::RecomSkeleton;
use crate::shadow::{attack_row, bone_codes, AttackRow};
use crate::{bad, Result};

/// New clips sit at this + KH1 entry.
pub const SLOT_BASE: usize = 100;
/// KH1's filler requests (0x141..) map entry 0 without root motion; no enemy script requests them
/// (Checked (files): the Shadow's and the Soldier's scripts).
pub const LAST_REQUEST: usize = 0x140;
/// The manifest's format; the DLL refuses others.
pub const FORMAT: &str = "kh1-enemy 1";
const REC: usize = 0xA0;

/// One attack row of a KH1 attack.
#[derive(Clone, Copy, Debug)]
pub struct RowSpec {
    /// The row's script offset (0x70 bytes, `sys 23`).
    pub at: usize,
    /// The Re:CoM attack id (`@ENP` attack entry `+0x18`) it takes its power from; its record is
    /// the template (`+0x64`).
    pub recom_id: u32,
}

/// One KH1 attack (an action request made with attack rows).
#[derive(Clone, Copy, Debug)]
pub struct AttackSpec {
    /// For the manifest and the log.
    pub name: &'static str,
    /// Its KH1 motion request.
    pub request: u16,
    /// The rows the script makes with it.
    pub rows: &'static [RowSpec],
}

/// One enemy's step.
#[derive(Clone, Copy, Debug)]
pub struct Spec {
    /// The manifest's `enemy` (the DLL's class name).
    pub name: &'static str,
    /// Re:CoM's enemy id (`ENEMY/NNNN`).
    pub recom_id: u16,
    /// KH1's model (`.mdls`, `.mset`) without the extension.
    pub kh1: &'static str,
    /// The script's name in its header (`+4`).
    pub script: &'static [u8],
    /// Re:CoM's clip name prefix (`e002_`) and slot count (Checked (files)).
    pub prefix: &'static str,
    /// Re:CoM's slot count in the `.ANN`.
    pub recom_slots: usize,
    /// Re:CoM's `.ASD`: record count, first record, trailing section (Checked (files)).
    pub asd: (usize, usize, usize),
    /// Where each constant is pushed (a name with several offsets must read the same at each).
    pub consts: &'static [(&'static str, &'static [usize])],
    /// Request words the script must push at these offsets (the behaviour plays them).
    pub requests: &'static [(u32, usize)],
    /// The stat records (their `+0x18`, the knock-back weight, must agree).
    pub records: &'static [u32],
    /// Its attacks, in the behaviour's order.
    pub attacks: &'static [AttackSpec],
    /// The manifest file (recom root).
    pub manifest: &'static str,
}

impl Spec {
    /// `ENEMY/NNNN`.
    pub fn dir(&self) -> String {
        format!("ENEMY/{:04}", self.recom_id)
    }
    /// `ENEMY/NNNN/ENNNNN.<ext>` (`Recom.hed`'s name).
    pub fn path(&self, ext: &str) -> String {
        format!("ENEMY/{:04}/EN{:04}.{ext}", self.recom_id, self.recom_id)
    }
    /// The test / scratch name of an extracted Re:CoM file (`ENEMY__0002__EN0002.ANN`).
    pub fn flat(&self, ext: &str) -> String {
        self.path(ext).replace('/', "__")
    }
}

/// Inputs: KH1's model, Re:CoM's files, KH1's `btltbl.bin`.
#[derive(Clone, Debug)]
pub struct Inputs {
    /// KH1's `.mset`.
    pub mset: Vec<u8>,
    /// KH1's `.mdls` (skeleton, bone codes, collision block, AI script in part 5).
    pub mdls: Vec<u8>,
    /// Re:CoM's `.ANN`.
    pub ann: Vec<u8>,
    /// Re:CoM's `.MDL` (skeleton).
    pub mdl: Vec<u8>,
    /// Re:CoM's `.ASD`.
    pub asd: Vec<u8>,
    /// Re:CoM's `.SES`.
    pub ses: Vec<u8>,
    /// KH1's `btltbl.bin`.
    pub btltbl: Vec<u8>,
}

/// One request's clip.
pub use crate::shadow::RequestClip;

/// The step's output.
#[derive(Clone, Debug)]
pub struct Built {
    /// The extended `.ANN`.
    pub ann: Vec<u8>,
    /// The extended `.ASD` (Re:CoM's own bytes when the enemy makes no record: the step then doesn't write it).
    pub asd: Vec<u8>,
    /// The extended `.SES` (empty for [`NO_SES`]: none written).
    pub ses: Vec<u8>,
    /// The manifest (`kh1-enemy 1`).
    pub manifest: String,
    /// Every request's clip.
    pub clips: Vec<RequestClip>,
    /// The behaviour's numbers.
    pub consts: Vec<(String, f32)>,
    /// Records added to the ASD (log lines).
    pub records: Vec<String>,
    /// The hurt parts (KH1 bone, Re:CoM bone).
    pub parts: Vec<(usize, usize)>,
    /// Per attack (the spec's order), its rows' attack-meets-attack numbers as the manifest lists them.
    pub clash: Vec<Vec<ClashOut>>,
}

/// KH1's exe numbers the shared code reads (`kh1_core::enemy::EXE_NUMBERS`, the same values;
/// kept here so this crate needs no kh1-core). Checked (decomp + exe).
pub fn exe_consts() -> Vec<(&'static str, f32)> {
    crate::shadow::exe_consts()
}

/// KH1's stats-block defaults (`0x1402A5D70`: `+4` walk 2, `+8` run 8, `+0x10` hop 190; Checked
/// (decomp)): a constant listed with no offsets takes its value here (a script that never writes
/// it keeps the default; M11 fliers).
pub const STATS_DEFAULTS: &[(&str, f32)] = &[("walk_speed", 2.0), ("run_speed", 8.0), ("hop_height", 190.0)];

/// Reads the constants at `table` (`push`: an opcode word whose low byte is 0x12 / 0x52 (a float)
/// or 0x02 / 0x42 (an integer, as a number), then the value). A name with no offsets is one of
/// [`STATS_DEFAULTS`].
pub fn script_consts(script: &[u8], name: &[u8], table: &[(&str, &[usize])]) -> Result<Vec<(String, f32)>> {
    let sname = String::from_utf8_lossy(name);
    if script.get(4..4 + name.len()) != Some(name) {
        return bad(format!("the model's part 5 is not {sname}"));
    }
    let mut out = Vec::new();
    for (n, offs) in table {
        if offs.is_empty() {
            let d = STATS_DEFAULTS.iter().find(|d| d.0 == *n).ok_or_else(|| crate::Error::Format(format!("{sname}: {n} has no offset and no stats-block default")))?;
            out.push((n.to_string(), d.1));
            continue;
        }
        let mut v: Option<f32> = None;
        for &o in offs.iter() {
            let w = script.get(o..o + 6).ok_or_else(|| crate::Error::Format(format!("{sname}: short at {o:#x}")))?;
            // A push's mode bits (6..7) don't change its immediate (`0x1402CC620` op 2 sub 0 / 1):
            // the child scripts push with mode 2 (`0x92`, M11 the Bandit's blade).
            let x = match w[0] & 0x3F {
                0x12 => f32::from_le_bytes(w[2..6].try_into().unwrap()),
                0x02 => i32::from_le_bytes(w[2..6].try_into().unwrap()) as f32,
                _ => return bad(format!("{sname} {o:#x} ({n}): not a push (opcode {:02x}{:02x}); not the script this step knows", w[0], w[1])),
            };
            if !x.is_finite() {
                return bad(format!("{sname} {o:#x} ({n}): {x}"));
            }
            match v {
                Some(p) if p != x => return bad(format!("{sname}: {n} is {p} and {x} at {offs:x?}")),
                _ => v = Some(x),
            }
        }
        out.push((n.to_string(), v.unwrap()));
    }
    Ok(out)
}

/// Checks the request words the script pushes (integer pushes) at their offsets.
pub fn check_requests(script: &[u8], name: &[u8], table: &[(u32, usize)]) -> Result<()> {
    let sname = String::from_utf8_lossy(name);
    for &(want, o) in table {
        let w = script.get(o..o + 6).ok_or_else(|| crate::Error::Format(format!("{sname}: short at {o:#x}")))?;
        let got = u32::from_le_bytes(w[2..6].try_into().unwrap());
        // Any push mode (bits 6..7 don't change the immediate, `0x1402CC620` op 2 sub 0; the Gargoyle's ball pushes its
        // bone codes with mode 2, M11).
        if w[0] & 0x3F != 0x02 || got != want {
            return bad(format!("{sname} {o:#x}: request {got:#x}, not {want:#x}: not the script this step knows"));
        }
    }
    Ok(())
}

/// The knock-back weight (stat record `+0x18`) from every record; they must agree.
pub fn weight(script: &[u8], records: &[u32]) -> Result<f32> {
    let mut w: Option<f32> = None;
    for &at in records {
        let o = at as usize + 0x18;
        let x = script.get(o..o + 4).map(|b| f32::from_le_bytes(b.try_into().unwrap())).ok_or_else(|| crate::Error::Format(format!("no record at {at:#x}")))?;
        match w {
            Some(p) if p != x => return bad(format!("knock-back weight {p} and {x} in different records")),
            _ => w = Some(x),
        }
    }
    w.ok_or_else(|| crate::Error::Format("no stat records".into()))
}

/// The btltbl globals the shared reactions read, as the behaviour's numbers.
pub fn btltbl_consts(btltbl: &[u8]) -> Result<Vec<(String, f32)>> {
    let g = kh1_formats::btltbl::Btltbl::parse(btltbl.to_vec())?.globals()?;
    Ok(vec![
        ("launch_min".into(), g.launch_speed_min as f32),
        ("launch_max".into(), g.launch_speed_max as f32),
        ("launch_hop".into(), g.launch_height_factor),
        ("flinch_push".into(), g.reaction_pushes[4]),
        ("air_flinch_speed".into(), g.air_knockback[0]),
        ("air_knock_speed".into(), g.air_knockback[1]),
    ])
}

/// KH1's hurt parts (the collision block's group 2, `.mdls` part 4 `CLS\x03`, 0x14-byte entries;
/// byte 2 a bone code) on the code's bone's parent, as Re:CoM bones (the nearest mapped
/// ancestor). Checked (files): for the Shadow this gives KH1 bones 30 / 2 (M10's checked parts).
pub fn hurt_parts(mdls: &[u8], conv: &Converter) -> Result<Vec<(usize, usize)>> {
    let c = crate::stats::mdls_part(mdls, 4).ok_or_else(|| crate::Error::Format("no collision block (part 4)".into()))?;
    if c.get(0..4) != Some(b"CLS\x03".as_slice()) {
        return bad("collision block is not CLS\\x03");
    }
    let n: Vec<usize> = (0..4).map(|i| c[4 + i] as usize).collect();
    let first = 0x20 + (n[0] + n[1]) * 0x14;
    let codes = bone_codes(mdls)?;
    let rig = conv.rig();
    let mut out = Vec::new();
    for i in 0..n[2] {
        let e = c.get(first + i * 0x14..first + (i + 1) * 0x14).ok_or_else(|| crate::Error::Format("hurt parts past the end".into()))?;
        let code = e[2] as usize;
        let bone = *codes.get(code).filter(|b| **b >= 0).ok_or_else(|| crate::Error::Format(format!("hurt part {i}: bone code {code} names no bone")))? as usize;
        let mut k = rig.parent(bone).unwrap_or(bone);
        let recom = loop {
            if let Some(r) = conv.map().iter().position(|m| *m == Some(k)) {
                break r;
            }
            match rig.parent(k) {
                Some(p) => k = p,
                None => return bad(format!("hurt part {i}: KH1 bone {bone} has no mapped ancestor")),
            }
        };
        out.push((k, recom));
    }
    if out.is_empty() {
        return bad("no hurt parts");
    }
    Ok(out)
}

/// Every window a HIT track is on (KH1's attack objects hit while the slot's bit is set,
/// `0x1402B47A0`): one Re:CoM record each, as Re:CoM's own models have one record per window (its
/// Powerwild's `e007_050`: two records, Checked (files)). The Powerwild's flurry turns its right
/// hand's slot on three times (M11).
pub fn hit_windows(m: &kh1_formats::mset::Motion, sub: u16) -> Result<Vec<(f32, f32)>> {
    let t = m.tracks.iter().find(|t| t.kind == kh1_formats::mset::track::HIT && t.sub == sub).ok_or_else(|| crate::Error::Format(format!("attack motion without a HIT track {sub}")))?;
    let mut out = Vec::new();
    let mut on: Option<f32> = None;
    for k in &t.keys {
        match (on, k.value() != 0) {
            (None, true) => on = Some(k.time),
            (Some(a), false) if k.time > a => {
                out.push((a, k.time));
                on = None;
            }
            _ => {}
        }
    }
    if let Some(a) = on {
        out.push((a, m.frames as f32));
    }
    if out.is_empty() {
        return bad("HIT track never on");
    }
    Ok(out)
}

/// The HIT track's first window of hit slot `sub` (key ≠ 0 on, then 0 off; `kh1-data`'s rule): the
/// sound events' anchor.
pub fn hit_window(m: &kh1_formats::mset::Motion, sub: u16) -> Result<(f32, f32)> {
    let t = m.tracks.iter().find(|t| t.kind == kh1_formats::mset::track::HIT && t.sub == sub).ok_or_else(|| crate::Error::Format(format!("attack motion without a HIT track {sub}")))?;
    let on = t.keys.iter().find(|k| k.value() != 0).map(|k| k.time).ok_or_else(|| crate::Error::Format("HIT track never on".into()))?;
    let off = t.keys.iter().find(|k| k.value() == 0 && k.time > on).map(|k| k.time).unwrap_or(m.frames as f32);
    Ok((on, off))
}

/// The ANN extended with KH1's motions: every entry a request (≤ [`LAST_REQUEST`]) names, at
/// slot [`SLOT_BASE`] + entry unless Re:CoM has it as an exact bake. Returns the ANN, every
/// request's clip and, per new clip, the Re:CoM clip the matcher pairs it with (bake or edit).
pub fn extend_ann(ann_in: &[u8], ms: &MotionSet, conv: &Converter, prefix: &str, recom_slots: usize) -> Result<(Vec<u8>, Vec<RequestClip>, Vec<(String, String)>)> {
    extend_ann_own(ann_in, ms, conv, prefix, recom_slots, &[], &[], &[], &[])
}

/// [`extend_ann`], with `own`: requests whose entry gets its own new clip even when Re:CoM has it as
/// an exact bake ([`OWN_CLIPS`]); the new clip is paired with the bake (its sounds, `extend_ses`).
/// `loops`: requests whose new clip loops although its last pose isn't its first ([`LOOPS`]); `holds`: requests whose new
/// clip holds at its end although its last pose is its first ([`HELD_CLIPS`]); `skip`: requests that get no clip
/// ([`SKIPPED`]).
#[allow(clippy::too_many_arguments)]
pub fn extend_ann_own(ann_in: &[u8], ms: &MotionSet, conv: &Converter, prefix: &str, recom_slots: usize, own: &[u16], loops: &[u16], holds: &[u16], skip: &[u16]) -> Result<(Vec<u8>, Vec<RequestClip>, Vec<(String, String)>)> {
    let mut ann = Ann::parse(ann_in)?;
    if ann.slots.len() != recom_slots {
        return bad(format!("{} slots, Re:CoM Steam's has {recom_slots} (already extended?)", ann.slots.len()));
    }
    let mut by_entry: Vec<(usize, Vec<u16>, bool)> = Vec::new();
    for (rid, w) in ms.requests.iter().enumerate().take(LAST_REQUEST + 1) {
        if skip.contains(&(rid as u16)) {
            continue;
        }
        for e in w.variants() {
            let e = e as usize;
            if e >= ms.motions.len() {
                return bad(format!("request {rid:#x} names entry {e}, past the {} motions", ms.motions.len()));
            }
            match by_entry.iter_mut().find(|x| x.0 == e) {
                Some(x) => {
                    if x.2 != w.root_motion() {
                        return bad(format!("entry {e} played with and without root motion (request {rid:#x})"));
                    }
                    x.1.push(rid as u16);
                }
                None => by_entry.push((e, vec![rid as u16], w.root_motion())),
            }
        }
    }
    by_entry.sort();
    if let Some(e) = by_entry.iter().find(|x| SLOT_BASE + x.0 > 999) {
        return bad(format!("entry {} past slot 999", e.0));
    }
    let decoded = decode_pack(&ann);
    let qs: Vec<Query> = by_entry.iter().map(|(e, _, root)| Query { entry: *e, motion: &ms.motions[*e], root_motion: *root }).collect();
    let found = match_all(conv, &qs, &decoded);
    let mut clips = Vec::new();
    let mut pairs = Vec::new();
    for ((e, reqs, root), fm) in by_entry.iter().zip(found) {
        let m = &ms.motions[*e];
        let (slot, name, looped, source) = match fm.found.as_ref() {
            Some((sc, MatchKind::Bake)) if !reqs.iter().any(|r| own.contains(r)) => {
                let s = &ann.slots[sc.slot];
                let looped = s.clip.as_ref().is_some_and(|c| c.looped());
                (sc.slot, s.name_str(), looped, "recom")
            }
            other => {
                let slot = SLOT_BASE + e;
                let name = format!("{prefix}{slot:03}");
                if let Some((sc, _)) = other {
                    pairs.push((name.clone(), ann.slots[sc.slot].name_str()));
                }
                let looped = (cycle_error(conv, m) <= LOOP_TOLERANCE_M || reqs.iter().any(|r| loops.contains(r))) && !reqs.iter().any(|r| holds.contains(r));
                let clip = conv.convert(m, &ClipSpec { name: name.clone(), looped, root_motion: *root, blend: 8 })?;
                while ann.slots.len() < slot {
                    ann.slots.push(Slot { name: [0; 16], clip: None });
                }
                if ann.slots.len() > slot {
                    return bad(format!("slot {slot} taken"));
                }
                ann.slots.push(Slot { name: name16(&name), clip: Some(clip) });
                (slot, name, looped, "kh1")
            }
        };
        for &r in reqs {
            clips.push(RequestClip { request: r, entry: *e, slot, name: name.clone(), frames: m.frames, root: *root, looped, source });
        }
    }
    clips.sort_by_key(|c| c.request);
    ann.pad_tail_like_recom();
    Ok((ann.to_bytes(), clips, pairs))
}

fn put_name(r: &mut [u8], at: usize, s: &str) {
    r[at..at + 16].copy_from_slice(&name16(s));
}
fn put_u32(r: &mut [u8], at: usize, v: u32) {
    r[at..at + 4].copy_from_slice(&v.to_le_bytes());
}
fn put_f32(r: &mut [u8], at: usize, v: f32) {
    r[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

/// One new ASD record: a row of an attack.
pub struct NewRecord<'a> {
    /// The attack's request (the clip it binds to).
    pub request: u16,
    /// The KH1 row.
    pub row: &'a AttackRow,
    /// The row's hit slot (`+0x34`).
    pub sub: u16,
    /// The Re:CoM attack id (power, template record).
    pub recom_id: u32,
}

/// The ASD with the new records added after Re:CoM's (Re:CoM's header, body and records
/// byte-identical, the trailing section moved after them, its offset `+0x18` updated; Re:CoM's
/// loader `0x140179340` / binder `0x14014DBE0` read only the counts and offsets `+8..+0x14`). The
/// template of each is Re:CoM's record with its attack id. Returns the ASD, log lines and, per new record
/// asked for, the indices of the records it made in the ASD (one per hit window; Re:CoM's records come first:
/// the index Re:CoM's attack set gives each live entry, `0x14014E010` / `0x14014DF10`: entry `+0x18` = the
/// record, records `+0x28` 0xA0 apart; M11, attack meets attack).
pub fn extend_asd(asd: &[u8], want: (usize, usize, usize), conv: &Converter, ms: &MotionSet, recs: &[NewRecord], clips: &[RequestClip]) -> Result<(Vec<u8>, Vec<String>, Vec<Vec<u16>>)> {
    let u = |o: usize| -> Result<u32> { asd.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).ok_or_else(|| crate::Error::Format("ASD: short".into())) };
    if asd.get(0..4) != Some(b"@COP".as_slice()) || u(4)? != 3 {
        return bad("ASD: not @COP version 3");
    }
    let (n, off, tail) = (u(0x10)? as usize, u(0x14)? as usize, u(0x18)? as usize);
    if (n, off, tail) != want || off + REC * n != tail || tail > asd.len() {
        return bad(format!("ASD: {n} attack records at {off:#x}, tail {tail:#x}; Re:CoM Steam's has {} at {:#x}, tail {:#x}", want.0, want.1, want.2));
    }
    let rec = |i: usize| -> [u8; REC] { asd[off + REC * i..off + REC * (i + 1)].try_into().unwrap() };
    let skel = conv.skeleton();
    let mut out_recs = Vec::new();
    let mut notes = Vec::new();
    let mut made: Vec<Vec<u16>> = Vec::with_capacity(recs.len());
    for nr in recs {
        made.push(Vec::new());
        let tmpl = (0..n).map(rec).find(|r| u32::from_le_bytes(r[0x64..0x68].try_into().unwrap()) == nr.recom_id).ok_or_else(|| crate::Error::Format(format!("ASD: no Re:CoM record with attack id {}", nr.recom_id)))?;
        let row = nr.row;
        let c = clips.iter().find(|c| c.request == nr.request).ok_or_else(|| crate::Error::Format(format!("request {} has no clip", nr.request)))?;
        let m = &ms.motions[c.entry];
        for (on, off_) in hit_windows(m, nr.sub)? {
        let mut rig = conv.rig();
        let parent = rig.parent(row.bone).ok_or_else(|| crate::Error::Format(format!("KH1 bone {} has no parent", row.bone)))?;
        let hand = conv.map().iter().position(|k| *k == Some(parent)).ok_or_else(|| crate::Error::Format(format!("KH1 bone {parent} has no Re:CoM bone")))?;
        rig.start_motion(m);
        let p0 = conv.kh1_point_in(&mut rig, m, on, hand, row.bone, row.t[0] as f64);
        let p1 = conv.kh1_point_in(&mut rig, m, on, hand, row.bone, row.t[1] as f64);
        let mut r = tmpl;
        let hname = skel.bones[hand].name.clone();
        put_name(&mut r, 0x00, &hname);
        put_u32(&mut r, 0x10, hand as u32);
        for (k, v) in p0.iter().enumerate() {
            put_f32(&mut r, 0x14 + 4 * k, *v as f32);
        }
        let line = (row.t[0] - row.t[1]).abs() > 1e-6;
        if line {
            // A line between the two points: Re:CoM's shape index 2 (type 7, `0x1407C1C98`).
            put_name(&mut r, 0x20, &hname);
            put_u32(&mut r, 0x30, hand as u32);
            for (k, v) in p1.iter().enumerate() {
                put_f32(&mut r, 0x34 + 4 * k, *v as f32);
            }
            put_u32(&mut r, 0x54, 2);
        } else {
            put_name(&mut r, 0x20, "Root");
            put_u32(&mut r, 0x30, 0);
            for k in 0..3 {
                put_f32(&mut r, 0x34 + 4 * k, 0.0);
            }
            put_u32(&mut r, 0x54, 1);
        }
        put_name(&mut r, 0x40, &c.name);
        put_u32(&mut r, 0x50, u32::MAX);
        put_f32(&mut r, 0x58, row.radius * 0.01);
        put_u32(&mut r, 0x5C, (on * 50.0).round() as u32);
        put_u32(&mut r, 0x60, (off_ * 50.0).round() as u32);
        put_u32(&mut r, 0x64, nr.recom_id);
        put_u32(&mut r, 0x68, 0);
        r[0x98..REC].fill(0);
        notes.push(format!(
            "{} (request {}) on {hname} at ({:.4}, {:.4}, {:.4}){} r {:.2} m, hit slot {} frames {on}..{off_}, attack id {}",
            c.name,
            nr.request,
            p0[0],
            p0[1],
            p0[2],
            if line { format!(" → ({:.4}, {:.4}, {:.4})", p1[0], p1[1], p1[2]) } else { String::new() },
            row.radius * 0.01,
            nr.sub,
            nr.recom_id
        ));
        made.last_mut().expect("pushed above").push(u16::try_from(n + out_recs.len()).map_err(|_| crate::Error::Format("ASD: too many records".into()))?);
        out_recs.push(r);
        }
    }
    let mut d = asd[..tail].to_vec();
    for r in &out_recs {
        d.extend_from_slice(r);
    }
    let new_tail = d.len();
    d.extend_from_slice(&asd[tail..]);
    d[0x10..0x14].copy_from_slice(&((n + out_recs.len()) as u32).to_le_bytes());
    d[0x18..0x1C].copy_from_slice(&(new_tail as u32).to_le_bytes());
    Ok((d, notes, made))
}

/// The record of an ASD whose attack id is `id`: (its motion name, its window start in frames).
fn template_of(asd: &[u8], id: u32) -> Option<(String, f32)> {
    let u = |o: usize| asd.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    let (n, off) = (u(0x10)? as usize, u(0x14)? as usize);
    (0..n).map(|i| &asd[off + REC * i..off + REC * (i + 1)]).find(|r| u32::from_le_bytes(r[0x64..0x68].try_into().unwrap()) == id).map(|r| {
        let e = r[0x40..0x50].iter().position(|&c| c == 0).unwrap_or(16);
        (String::from_utf8_lossy(&r[0x40..0x40 + e]).into_owned(), u32::from_le_bytes(r[0x5C..0x60].try_into().unwrap()) as f32 / 50.0)
    })
}

/// The SES with Re:CoM's sounds on the new clips (Re:CoM's entries kept): a new clip paired with a
/// Re:CoM clip that has sounds gets them (same frames); every other attack clip gets its first
/// row's template record's sounds, moved by (KH1's hit start − Re:CoM's window start), a sound at
/// frame 0 kept at 0 (the Shadow's rule, M10).
pub fn extend_ses(ses: &[u8], asd: &[u8], ms: &MotionSet, clips: &[RequestClip], pairs: &[(String, String)], attacks: &[(u16, u32, u16)]) -> Result<Vec<u8>> {
    let mut entries = crate::ses::parse(ses)?;
    let find = |es: &[crate::ses::Entry], n: &str| es.iter().find(|e| e.name() == n).cloned();
    let set_frame = |mut e: [u8; 0x50], f: i32| {
        e[0x18..0x1C].copy_from_slice(&f.to_le_bytes());
        e
    };
    let frame_of = |e: &[u8; 0x50]| i32::from_le_bytes(e[0x18..0x1C].try_into().unwrap());
    let mut add: Vec<(String, [u8; 0x30], Vec<[u8; 0x50]>)> = Vec::new();
    for (new, old) in pairs {
        if let Some(src) = find(&entries, old) {
            add.push((new.clone(), src.head, src.events.clone()));
        }
    }
    for &(req, id, sub) in attacks {
        let c = clips.iter().find(|c| c.request == req).ok_or_else(|| crate::Error::Format(format!("request {req} has no clip")))?;
        if c.source == "recom" || add.iter().any(|a| a.0 == c.name) {
            continue;
        }
        let Some((tm, t_on)) = template_of(asd, id) else { continue };
        let Some(src) = find(&entries, &tm) else { continue };
        let (on, _) = hit_window(&ms.motions[c.entry], sub)?;
        let ev = src.events.iter().map(|e| if frame_of(e) == 0 { *e } else { set_frame(*e, (frame_of(e) as f32 - t_on + on).round().max(0.0) as i32) }).collect();
        add.push((c.name.clone(), src.head, ev));
    }
    for (name, head, events) in add {
        entries.retain(|e| e.name() != name);
        let mut h = head;
        h[..16].copy_from_slice(&name16(&name));
        entries.push(crate::ses::Entry { head: h, events });
    }
    Ok(crate::ses::write(&entries))
}

/// A ring's clip ([`RINGS`]) with its template record's sounds, moved by (the ring's start − Re:CoM's window
/// start), a frame-0 sound kept at 0; nothing when the matcher already gave the clip its Re:CoM twin's sounds.
fn ring_sounds(ses: &[u8], asd: &[u8], clips: &[RequestClip], pairs: &[(String, String)], request: u16, rec: [u8; REC]) -> Result<Vec<u8>> {
    let mut entries = crate::ses::parse(ses)?;
    let c = clips.iter().find(|c| c.request == request).ok_or_else(|| crate::Error::Format(format!("request {request} has no clip")))?;
    if c.source == "recom" || pairs.iter().any(|p| p.0 == c.name) || entries.iter().any(|e| e.name() == c.name) {
        return Ok(ses.to_vec());
    }
    let id = u32::from_le_bytes(rec[0x64..0x68].try_into().unwrap());
    let Some((tm, t_on)) = template_of(asd, id) else { return Ok(ses.to_vec()) };
    let Some(src) = entries.iter().find(|e| e.name() == tm).cloned() else { return Ok(ses.to_vec()) };
    let on = u32::from_le_bytes(rec[0x5C..0x60].try_into().unwrap()) as f32 / 50.0;
    let events = src
        .events
        .iter()
        .map(|e| {
            let f = i32::from_le_bytes(e[0x18..0x1C].try_into().unwrap());
            let mut e = *e;
            if f != 0 {
                e[0x18..0x1C].copy_from_slice(&((f as f32 - t_on + on).round().max(0.0) as i32).to_le_bytes());
            }
            e
        })
        .collect();
    let mut h = src.head;
    h[..16].copy_from_slice(&name16(&c.name));
    entries.push(crate::ses::Entry { head: h, events });
    Ok(crate::ses::write(&entries))
}

fn fmt_f(v: f32) -> String {
    let s = format!("{v}");
    if s.contains('.') || s.contains('e') || s.contains("inf") || s.contains("NaN") {
        s
    } else {
        format!("{s}.0")
    }
}

/// Requests that get no clip (M11): KH1 status motions the port never plays (Re:CoM's own Stop / freeze / stun states
/// stay, decision M10) whose entry another request plays with the other root-motion rule, which one clip can't hold. The
/// Red Nocturne's 0x66 (entry 7 without root motion; `0x1402B7260` plays it for KH1's status of movement states 0x16 /
/// 0x17, Checked (decomp)) shares entry 7 with its dizzy loop 0x43 (with root motion; the dizzy is not built either,
/// owner question): 0x43 keeps the clip. The Blue Rhapsody's 0x66 the same (entry 12, shared with its dizzy loop 0x43;
/// its dizzy is element-only, left out), and its 0x69 (entry 7 without root motion, another of those status motions;
/// entry 7 keeps request 7, the landing, with root motion). Checked (files: the player's `xa_ex_2120.mset` map). The Green
/// Requiem's 0x66 the same as the Red Nocturne's (entry 7, shared with its dizzy loop 0x43; Checked (files: `xa_ex_2140.mset`)).
pub const SKIPPED: &[(&str, u16)] = &[("greenrequiem", 0x66), ("rednocturne", 0x66), ("bluerhapsody", 0x66), ("bluerhapsody", 0x69), ("yellowopera", 0x66)];

/// Enemies Re:CoM keeps no `.SES` for (M11, the Red Nocturne: `Recom.hed` holds no `ENEMY/0003/EN0003.SES` and the
/// exe's resource names for enemy 3 list none, so Re:CoM never loads one; Checked (files + exe)): their clips play no
/// motion sounds in Re:CoM, the step reads none and writes none (`Built::ses` empty). The Green Requiem (enemy 6) the same
/// (Checked (files): no `EN0006.SES` in the session's extraction). The Yellow Opera's enemy 5 the same (Checked (files + exe), M11); its 0x66 as the Red Nocturne's
/// (Checked (files: the player's `xa_ex_2130.mset` map)).
pub const NO_SES: &[&str] = &["greenrequiem", "rednocturne", "yellowopera"];

/// Builds everything for one enemy.
pub fn build(spec: &Spec, inp: &Inputs) -> Result<Built> {
    let ms = MotionSet::parse(&inp.mset)?;
    let skel = RecomSkeleton::parse(&inp.mdl)?;
    let conv = Converter::new(&inp.mdls, skel)?;
    let script = crate::stats::mdls_part(&inp.mdls, 5).ok_or_else(|| crate::Error::Format(format!("{}.mdls: no AI script (part 5)", spec.kh1)))?;
    let mut consts = script_consts(script, spec.script, spec.consts)?;
    check_requests(script, spec.script, spec.requests)?;
    for (n, v) in exe_consts() {
        consts.push((n.to_string(), v));
    }
    consts.extend(btltbl_consts(&inp.btltbl)?);
    consts.push(("weight".into(), weight(script, spec.records)?));
    let own: Vec<u16> = OWN_CLIPS.iter().filter(|o| o.0 == spec.name).map(|o| o.1).collect();
    let loops: Vec<u16> = LOOPS.iter().filter(|o| o.0 == spec.name).map(|o| o.1).collect();
    let holds: Vec<u16> = HELD_CLIPS.iter().filter(|o| o.0 == spec.name).map(|o| o.1).collect();
    let skip: Vec<u16> = SKIPPED.iter().filter(|o| o.0 == spec.name).map(|o| o.1).collect();
    let (ann, clips, pairs) = extend_ann_own(&inp.ann, &ms, &conv, spec.prefix, spec.recom_slots, &own, &loops, &holds, &skip)?;
    consts.extend(bone_points(spec.name, &inp.mdls, &ms, &conv, &clips)?);
    consts.extend(child_consts(spec.name, &inp.mdls)?);
    consts.extend(t_points(spec.name, &inp.mdls, &ms, &conv, &clips, &consts)?);
    consts.extend(shot_consts(spec.name, script)?);
    consts.extend(strike_consts(spec.name, script)?);
    consts.extend(sight_bones(spec.name, &inp.mdls, &conv)?);
    consts.extend(search_consts(spec.name, script)?);
    consts.extend(reward_consts(spec.name, script)?);
    // The attack rows → ASD records.
    let codes = bone_codes(&inp.mdls)?;
    let mut rows: Vec<(usize, AttackRow, u16)> = Vec::new();
    for a in spec.attacks {
        for r in a.rows {
            if rows.iter().any(|x| x.0 == r.at) {
                continue;
            }
            let row = attack_row(script, r.at, &codes)?;
            let sub = script.get(r.at + 0x34..r.at + 0x38).map(|b| u32::from_le_bytes(b.try_into().unwrap())).filter(|&s| s < 4).ok_or_else(|| crate::Error::Format(format!("attack row {:#x}: hit slot outside 0..3", r.at)))? as u16;
            rows.push((r.at, row, sub));
        }
    }
    let row_of = |at: usize| rows.iter().find(|x| x.0 == at).expect("read above");
    let mut new = Vec::new();
    let mut sound_rows = Vec::new();
    // Per new record: its attack and its row (the manifest's clash rows).
    let mut new_of = Vec::new();
    for (ai, a) in spec.attacks.iter().enumerate() {
        for (i, r) in a.rows.iter().enumerate() {
            let (_, row, sub) = row_of(r.at);
            new.push(NewRecord { request: a.request, row, sub: *sub, recom_id: r.recom_id });
            new_of.push((ai, i));
            if i == 0 {
                sound_rows.push((a.request, r.recom_id, *sub));
            }
        }
    }
    let (mut asd, mut records, made) = extend_asd(&inp.asd, spec.asd, &conv, &ms, &new, &clips)?;
    // M11 (the Large Body's stomp): a child ring on Re:CoM's own shockwave record.
    let rings = ring_records(spec.name, script, &inp.asd, &clips, &consts)?;
    add_records(&mut asd, &rings.iter().map(|r| r.1).collect::<Vec<_>>())?;
    records.extend(rings.iter().map(|r| r.4.clone()));
    // M11 (the Neoshadow's grab): the held record on its grab's clip.
    let holds = hold_records(spec.name, &inp.asd, &clips)?;
    add_records(&mut asd, &holds.iter().map(|r| r.1).collect::<Vec<_>>())?;
    records.extend(holds.iter().map(|r| r.2.clone()));
    for h in HOLDS.iter().filter(|h| h.enemy == spec.name) {
        consts.push(("grab_record_frames".into(), h.frames));
        consts.push(("grab_kind_index".into(), HELD_KIND_INDEX as f32));
    }
    let no_ses = NO_SES.contains(&spec.name);
    if no_ses && !inp.ses.is_empty() {
        return bad(format!("{}: Re:CoM has no {}, yet one was given", spec.name, spec.path("SES")));
    }
    let mut ses = if no_ses { Vec::new() } else { extend_ses(&inp.ses, &inp.asd, &ms, &clips, &pairs, &sound_rows)? };
    for r in rings.iter().filter(|_| !no_ses) {
        // The ring's clip takes its Re:CoM template's sounds at the ring's start (no HIT track on its motion).
        ses = ring_sounds(&ses, &inp.asd, &clips, &pairs, r.0, r.1)?;
    }
    let (mut ann, mut clips) = (ann, clips);
    apply_starts(spec.name, &ms, &conv, &mut ann, &mut clips, &mut asd, spec.asd.0, &mut ses)?;
    let parts = hurt_parts(&inp.mdls, &conv)?;
    // M11, the shared enemy-guard step: the enemy's guard-type rows and their HIT windows.
    let guards = guard_rows(spec.name, script, &ms, &clips)?;
    // M11: its thrown object's numbers (the child script, part 3), when it has one.
    consts.extend(thrown_consts(spec.name, &inp.mdls, script, &inp.asd, spec.asd, &conv)?);
    // The attacks and their clash recoils (row `+0x40`, request in the low 16 bits; the first row's,
    // as KH1's clash reads the record that met the Guard: Assumption that an attack's rows agree,
    // checked below).
    let mut attacks = Vec::new();
    for (ai, a) in spec.attacks.iter().enumerate() {
        // M11, attack meets attack: each row's clash numbers and the ASD records it made.
        let clash: Vec<ClashOut> = a
            .rows
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let row = &row_of(r.at).1;
                let records = new_of.iter().zip(&made).filter(|(o, _)| **o == (ai, i)).flat_map(|(_, m)| m.iter().copied()).collect();
                ClashOut { row: r.at, clash_type: row.clash_type, priority: row.priority, turn: row.turn, cos: row.cos, records }
            })
            .collect();
        if a.rows.is_empty() && holds.iter().any(|h| h.0 == a.request) {
            // A held record's attack ([`HOLDS`]): no KH1 row, so no clash (the grab is no attack in KH1) and no recoil
            // (110: KH1's "none", Re:CoM's flinch when Sora's Guard blocks it).
            if !clips.iter().any(|c| c.request == a.request) {
                return bad(format!("{}: request {} has no motion", a.name, a.request));
            }
            attacks.push((a.name, a.request, NO_RECOIL, Vec::new(), clash));
            continue;
        }
        if a.rows.is_empty() {
            // A ring's attack ([`RINGS`]): its row's recoil (KH1's clash on the child's row goes to its owner).
            let Some(r) = rings.iter().find(|r| r.0 == a.request) else { return bad(format!("{}: no rows and no ring", a.name)) };
            if !clips.iter().any(|c| c.request == a.request) {
                return bad(format!("{}: request {} has no motion", a.name, a.request));
            }
            attacks.push((a.name, a.request, r.2, vec![r.3], clash));
            continue;
        }
        let rs: Vec<u32> = a.rows.iter().map(|r| row_of(r.at).1.recoil & 0xFFFF).collect();
        if rs.iter().any(|&x| x != rs[0]) {
            return bad(format!("{}: its rows' clash recoils differ ({rs:?})", a.name));
        }
        if !clips.iter().any(|c| c.request == a.request) {
            return bad(format!("{}: request {} has no motion", a.name, a.request));
        }
        attacks.push((a.name, a.request, rs[0], a.rows.iter().map(|r| r.at).collect::<Vec<_>>(), clash));
    }
    let manifest = manifest_json(spec, &consts, &clips, &attacks, &parts, &guards);
    let clash = attacks.iter().map(|a| a.4.clone()).collect();
    Ok(Built { ann, asd, ses, manifest, clips, consts, records, parts, clash })
}

/// One attack row's KH1 attack-meets-attack numbers in the manifest (M11, `docs/recom-m11-enemies.md` § Attack
/// meets attack): the row, its clash type, priority, front cone, and the ASD records made from it.
#[derive(Clone, Debug, PartialEq)]
pub struct ClashOut {
    /// The row's script offset.
    pub row: usize,
    /// `+0x38 & 7`.
    pub clash_type: u32,
    /// `+0x3C`.
    pub priority: i32,
    /// `+0x60` (radians).
    pub turn: f32,
    /// `+0x64`.
    pub cos: f32,
    /// The ASD records made from it (indices; Re:CoM's records first).
    pub records: Vec<u16>,
}

impl ClashOut {
    /// The manifest's entry: `{"row", "type", "priority", "turn", "cos", "records"}`.
    pub fn json(&self) -> String {
        format!(
            "{{\"row\": \"{:#x}\", \"type\": {}, \"priority\": {}, \"turn\": {}, \"cos\": {}, \"records\": [{}]}}",
            self.row,
            self.clash_type,
            self.priority,
            fmt_f(self.turn),
            fmt_f(self.cos),
            self.records.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(", ")
        )
    }
}

/// One attack in the manifest: name, request, clash recoil, rows, the rows' clash numbers.
type AttackOut<'a> = (&'a str, u16, u32, Vec<usize>, Vec<ClashOut>);

fn manifest_json(spec: &Spec, consts: &[(String, f32)], clips: &[RequestClip], attacks: &[AttackOut], parts: &[(usize, usize)], guards: &[Guard]) -> String {
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(&format!("  \"format\": \"{FORMAT}\",\n  \"enemy\": \"{}\",\n  \"recom_id\": {},\n  \"kh1\": \"{}\",\n", spec.name, spec.recom_id, spec.kh1));
    s.push_str("  \"consts\": {\n");
    let c: Vec<String> = consts.iter().map(|(k, v)| format!("    \"{k}\": {}", fmt_f(*v))).collect();
    s.push_str(&c.join(",\n"));
    s.push_str("\n  },\n  \"attacks\": [\n");
    let a: Vec<String> = attacks
        .iter()
        .map(|(n, r, rc, rows, clash)| {
            format!(
                "    {{\"name\": \"{n}\", \"request\": {r}, \"recoil\": {rc}, \"rows\": [{}], \"clash\": [{}]}}",
                rows.iter().map(|x| format!("\"{x:#x}\"")).collect::<Vec<_>>().join(", "),
                clash.iter().map(|c| c.json()).collect::<Vec<_>>().join(", ")
            )
        })
        .collect();
    s.push_str(&a.join(",\n"));
    s.push_str("\n  ],\n");
    if !guards.is_empty() {
        s.push_str("  \"guards\": [\n");
        let g: Vec<String> = guards
            .iter()
            .map(|g| {
                let w: Vec<String> = g.windows.iter().map(|(r, on)| format!("{{\"request\": {r}, \"on\": [{}]}}", on.iter().map(|(a, b)| format!("[{}, {}]", fmt_f(*a), fmt_f(*b))).collect::<Vec<_>>().join(", "))).collect();
                // Rows acting with it (the Search Ghost's burst): listed with their slots; none for the Bandit's.
                let also = if g.also.is_empty() {
                    String::new()
                } else {
                    format!("\"also\": [{}], \"slots\": [{}], ", g.also.iter().map(|a| format!("\"{a:#x}\"")).collect::<Vec<_>>().join(", "), g.slots.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
                };
                format!(
                    "    {{\"name\": \"{}\", \"row\": \"{:#x}\", {also}\"type\": {}, \"priority\": {}, \"recoil\": {}, \"slot\": {}, \"turn\": {}, \"cos\": {}, \"windows\": [{}]}}",
                    g.name,
                    g.row,
                    g.clash_type,
                    g.priority,
                    g.recoil,
                    g.slot,
                    fmt_f(g.turn),
                    fmt_f(g.cos),
                    w.join(", ")
                )
            })
            .collect();
        s.push_str(&g.join(",\n"));
        s.push_str("\n  ],\n");
    }
    s.push_str(&format!("  \"parts\": [{}],\n", parts.iter().map(|p| p.1.to_string()).collect::<Vec<_>>().join(", ")));
    s.push_str(&format!("  \"parts_kh1\": [{}],\n", parts.iter().map(|p| p.0.to_string()).collect::<Vec<_>>().join(", ")));
    s.push_str("  \"requests\": [\n");
    let r: Vec<String> = clips
        .iter()
        .map(|c| format!("    {{\"request\": {}, \"entry\": {}, \"slot\": {}, \"name\": \"{}\", \"frames\": {}, \"root\": {}, \"loop\": {}, \"source\": \"{}\"}}", c.request, c.entry, c.slot, c.name, c.frames, c.root, c.looped, c.source))
        .collect();
    s.push_str(&r.join(",\n"));
    s.push_str("\n  ]\n}\n");
    s
}

/// One guard-type row of an enemy (M11, the shared enemy-guard step; `kh1_core::enemy::guard`): its
/// name (the behaviour's), script offset and the requests the script makes it with.
pub struct GuardSpec {
    /// The enemy (the spec's `name`).
    pub enemy: &'static str,
    /// The row's name (the behaviour's `Guard::name`).
    pub name: &'static str,
    /// The row's script offset.
    pub row: usize,
    /// Rows made with it that differ only in their sphere (bone, radius) and HIT slot (the Search Ghost's
    /// burst: both hands at both strikes). Re:CoM's body guard is the whole body, so they act as one row,
    /// on while any of their slots is on: their clash numbers must equal `row`'s.
    pub also: &'static [usize],
    /// The requests it is made with; none for a row KH1 makes with mode 3 (`sys 213(row, 3)`: `0x1402B47A0`
    /// tests it every frame, whatever plays, until it is removed; the Large Body's front, M11).
    pub requests: &'static [u16],
}

/// A KH1 child object that is an expanding ring on the floor (M11, the Large Body's stomp `ex_2050_00.bd`: made
/// by `sys 94` at the stomp's frame `ring_at` with the parent's row `row`; each frame for `ring_life` frames it hits
/// every party member standing `[r − ring_band, r)` from where it was made and within `ring_height` of its height,
/// then grows `r` by `ring_growth`; Checked (files: the child disassembled; decomp: `sys 23` / `sys 60`)). Built on
/// **Re:CoM's own shockwave**: a copy of the Re:CoM record with attack id `recom_id` (its Large Body's
/// `e009_053` circle, Re:CoM's type 11 `CCircleCollision` at its root, the record Re:CoM's state 3 grows each frame
/// through `0x14014E5C0`) bound to the KH1 request's clip, on for the ring's frames; the DLL grows its radius by
/// KH1's numbers (`win::kh1_enemy::ring_step`).
pub struct RingSpec {
    /// The enemy (the spec's `name`).
    pub enemy: &'static str,
    /// The request the ring is made during (the attack's, [`AttackSpec::request`]).
    pub request: u16,
    /// The parent's row the child's attack is made with (`sys 94`'s fourth word).
    pub row: usize,
    /// The Re:CoM attack id whose record (shape, place) and power it takes.
    pub recom_id: u32,
}

/// The rings (checked against the scripts: `ex_2050.bd` `0xA68` passes `0x31C` to `sys 94`; its child's
/// `0x330` makes it). The Large Body's ring takes Re:CoM's stomp shockwave (id 1).
pub const RINGS: &[RingSpec] = &[RingSpec { enemy: "largebody", request: 230, row: 0x31C, recom_id: 1 }];

/// Re:CoM's shape index of a type 11 `CCircleCollision` (`0x1407C1C98`: indices 0..4 → types 1, 2, 7, 8, 11;
/// Checked (exe)).
const CIRCLE_SHAPE: u32 = 4;

/// An `.ASD` record's offset from its bone: x / y / z at `+0x14` / `+0x18` / `+0x1C` (`0x14014D740` reads them as
/// the point the bone's matrix places; Checked (decomp)). The ring's y.
pub const RING_OFFSET_Y: usize = 0x18;

/// [`RINGS`]' records of an enemy: (the record, its recoil, its row, a log line). The record is the Re:CoM
/// template's (its shape must be the circle), on the request's clip, radius 0, its centre on the bone (offset y 0:
/// KH1's height is the stomp's spot), on for frames `[ring_at, ring_at + ring_life)`.
fn ring_records(name: &str, script: &[u8], asd: &[u8], clips: &[RequestClip], consts: &[(String, f32)]) -> Result<Vec<(u16, [u8; REC], u32, usize, String)>> {
    let get = |n: &str| consts.iter().find(|c| c.0 == n).map(|c| c.1).ok_or_else(|| crate::Error::Format(format!("{name}: no {n} for its ring")));
    let u = |o: usize| asd.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).ok_or_else(|| crate::Error::Format("ASD: short".into()));
    let mut out = Vec::new();
    for r in RINGS.iter().filter(|r| r.enemy == name) {
        let row = script.get(r.row..r.row + 0x70).ok_or_else(|| crate::Error::Format(format!("{name}: no ring row {:#x}", r.row)))?;
        if !crate::stats::pushes_address(script, r.row as u32) || i32::from_le_bytes(row[0x10..0x14].try_into().unwrap()) != -1 {
            return bad(format!("{name}: row {:#x} is not the ring's (pushed, bone −1)", r.row));
        }
        let recoil = u32::from_le_bytes(row[0x40..0x44].try_into().unwrap()) & 0xFFFF;
        let (n, off) = (u(0x10)? as usize, u(0x14)? as usize);
        let tmpl = (0..n).map(|i| &asd[off + REC * i..off + REC * (i + 1)]).find(|x| u32::from_le_bytes(x[0x64..0x68].try_into().unwrap()) == r.recom_id).ok_or_else(|| crate::Error::Format(format!("{name}: no Re:CoM record with attack id {}", r.recom_id)))?;
        let shape = u32::from_le_bytes(tmpl[0x54..0x58].try_into().unwrap());
        if shape != CIRCLE_SHAPE {
            return bad(format!("{name}: Re:CoM's record {} has shape {shape}, not the circle {CIRCLE_SHAPE}: not the record this step knows", r.recom_id));
        }
        let c = clips.iter().find(|c| c.request == r.request).ok_or_else(|| crate::Error::Format(format!("{name}: request {} has no clip", r.request)))?;
        let (at, life) = (get("ring_at")?, get("ring_life")?);
        if !(at >= 0.0 && life > 0.0 && at < c.frames as f32) {
            return bad(format!("{name}: ring frames {at} + {life} on a {}-frame motion", c.frames));
        }
        let mut rec: [u8; REC] = tmpl.try_into().unwrap();
        put_name(&mut rec, 0x40, &c.name);
        put_u32(&mut rec, 0x50, u32::MAX);
        put_f32(&mut rec, 0x58, 0.0);
        put_u32(&mut rec, 0x5C, (at * 50.0).round() as u32);
        put_u32(&mut rec, 0x60, ((at + life) * 50.0).round() as u32);
        // KH1 measures the ring's height from the stomp's own spot (the child at the parent's position, its band test
        // `0x18` within `ring_height` of it): the centre at the bone itself (Re:CoM's template sits 0.3 m up its root).
        put_f32(&mut rec, RING_OFFSET_Y, 0.0);
        let line = format!("{} (request {}) Re:CoM's circle record {} (its root, offset 0), radius grown by KH1's ring from 0, frames {at}..{}", c.name, r.request, r.recom_id, at + life);
        out.push((r.request, rec, recoil, r.row, line));
    }
    Ok(out)
}

/// M11 (the Neoshadow's grab from below, owner 2026-11-02: a new Re:CoM player reaction, "held"): a record that
/// carries the held reaction kind to Sora. KH1's grab is no attack row (`ex_2420.bd` `0x1AF6` requests Sora's held
/// motion itself, `sys 113` / `sys 13`); Re:CoM reaches Sora only through a hit, so the grab's clip gets one record:
/// Re:CoM's template (its shape, bone, place, radius and power id), kind index [`HELD_KIND_INDEX`] (the DLL's
/// extended kind table: the held kind; `kh1-recom`'s `held`), live from the clip's first frame for `frames`, no hit
/// effect (as Re:CoM's records with an empty effect name). The DLL's damage check takes no damage for the kind.
pub struct HoldSpec {
    /// The enemy (the spec's `name`).
    pub enemy: &'static str,
    /// The grab's request (the attack's, [`AttackSpec::request`]).
    pub request: u16,
    /// The Re:CoM attack id whose record it takes its shape and place from.
    pub recom_id: u32,
    /// Frames from the clip's start the record lives (`kh1_core::neoshadow` waits as long for the hold).
    pub frames: f32,
}

/// The held kind's index in the DLL's kind table (`kh1-recom`'s `held::HELD_INDEX`: the first after Re:CoM's 21,
/// `0x14052A220`; the manifest's `grab_kind_index`, which the DLL checks). Checked (exe): Re:CoM's table holds 21.
pub const HELD_KIND_INDEX: u32 = 21;

/// KH1's "no recoil" request (`110`: no such motion; the host plays Re:CoM's flinch).
const NO_RECOIL: u32 = 110;

/// The held records. The Neoshadow's grab 208 takes Re:CoM's come-up strike (id 1: `m_ex420_sk`, its root, 1.0 m;
/// the Re:CoM strike that comes out of the floor under Sora; Assumption: covers him standing on its spot, where KH1
/// puts it), live for 2 frames (Assumption: KH1's grab is decided in one frame; a second covers Re:CoM's
/// collision tick).
pub const HOLDS: &[HoldSpec] = &[HoldSpec { enemy: "neoshadow", request: 208, recom_id: 1, frames: 2.0 }];

/// [`HOLDS`]' records of an enemy: (its request, the record, a log line).
fn hold_records(name: &str, asd: &[u8], clips: &[RequestClip]) -> Result<Vec<(u16, [u8; REC], String)>> {
    let u = |o: usize| asd.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).ok_or_else(|| crate::Error::Format("ASD: short".into()));
    let mut out = Vec::new();
    for h in HOLDS.iter().filter(|h| h.enemy == name) {
        let (n, off) = (u(0x10)? as usize, u(0x14)? as usize);
        let tmpl = (0..n).map(|i| &asd[off + REC * i..off + REC * (i + 1)]).find(|x| u32::from_le_bytes(x[0x64..0x68].try_into().unwrap()) == h.recom_id).ok_or_else(|| crate::Error::Format(format!("{name}: no Re:CoM record with attack id {}", h.recom_id)))?;
        let c = clips.iter().find(|c| c.request == h.request).ok_or_else(|| crate::Error::Format(format!("{name}: request {} has no clip", h.request)))?;
        if !(h.frames > 0.0 && h.frames < c.frames as f32) {
            return bad(format!("{name}: held record frames {} on a {}-frame motion", h.frames, c.frames));
        }
        let mut rec: [u8; REC] = tmpl.try_into().unwrap();
        put_name(&mut rec, 0x40, &c.name);
        put_u32(&mut rec, 0x50, u32::MAX);
        put_u32(&mut rec, 0x5C, 0);
        put_u32(&mut rec, 0x60, (h.frames * 50.0).round() as u32);
        put_u32(&mut rec, 0x68, HELD_KIND_INDEX);
        // No hit effect: id 0, no name (Re:CoM's own records with an empty effect name; Checked (files)).
        put_u32(&mut rec, 0x6C, 0);
        rec[0x70..0x80].fill(0);
        let bone = String::from_utf8_lossy(&rec[0..16]).trim_end_matches('\0').to_string();
        let line = format!("{} (request {}) Re:CoM's record {} ({bone}, r {:.2} m), the held kind (index {HELD_KIND_INDEX}), frames 0..{}, no effect", c.name, h.request, h.recom_id, f32::from_le_bytes(rec[0x58..0x5C].try_into().unwrap()), h.frames);
        out.push((h.request, rec, line));
    }
    Ok(out)
}

/// Re:CoM's ASD with `recs` added after its last record (the trailing section moved after them, its count and
/// tail offset updated, as [`extend_asd`] does).
fn add_records(asd: &mut Vec<u8>, recs: &[[u8; REC]]) -> Result<()> {
    if recs.is_empty() {
        return Ok(());
    }
    let u = |d: &[u8], o: usize| u32::from_le_bytes(d[o..o + 4].try_into().unwrap()) as usize;
    let (n, off, tail) = (u(asd, 0x10), u(asd, 0x14), u(asd, 0x18));
    if off + REC * n != tail || tail > asd.len() {
        return bad("ASD: records and tail don't meet");
    }
    let rest = asd.split_off(tail);
    for r in recs {
        asd.extend_from_slice(r);
    }
    let new_tail = asd.len();
    asd.extend_from_slice(&rest);
    asd[0x10..0x14].copy_from_slice(&((n + recs.len()) as u32).to_le_bytes());
    asd[0x18..0x1C].copy_from_slice(&(new_tail as u32).to_le_bytes());
    Ok(())
}

/// Every built enemy's guard rows, in its behaviour's order (`kh1_core::bandit::Guard::ALL`).
/// Checked (files): the Bandit's `ex_2090.bd` makes `0xEC` at `0x1ED6` / `0x1F6E` / `0x2188` (with
/// 210, 211, 212) and `0x15C` at `0x408E` / `0x40CC` / `0x4186` (230, 231, 233).
///
/// The Search Ghost's burst (`ex_2230.bd` `0x128E`): `0x38C` / `0x3FC` with hit bit A (`0x12D4` / `0x12E4`)
/// and `0x46C` / `0x4DC` with hit bit B (`0x131A` / `0x132A`), all made with 204. Checked (files).
pub const GUARDS: &[GuardSpec] = &[
    GuardSpec { enemy: "bandit", name: "spin", row: 0xEC, also: &[], requests: &[210, 211, 212] },
    GuardSpec { enemy: "bandit", name: "stance", row: 0x15C, also: &[], requests: &[230, 231, 233] },
    GuardSpec { enemy: "searchghost", name: "burst", row: 0x38C, also: &[0x3FC, 0x46C, 0x4DC], requests: &[204] },
    // The Pirate's `ex_2100.bd` makes `0x1CC` at `0x1156` (then 217) and `0x46C` at `0xAB4` (after 208).
    GuardSpec { enemy: "pirate", name: "spin", row: 0x1CC, also: &[], requests: &[217] },
    GuardSpec { enemy: "pirate", name: "stance", row: 0x46C, also: &[], requests: &[208] },
    // The Large Body's `ex_2050.bd` makes `0x38C` at `0x9F2` (`0x9CA`, at init and after each charge) with
    // `sys 213(row, 3)`: mode 3, on every frame until `sys 213(row, 0)` removes it (`0x9B8`, at the charge's
    // start): no request, no HIT slot ([`GuardSpec::requests`] empty; `kh1_core::enemy::guard::ALWAYS`).
    GuardSpec { enemy: "largebody", name: "front", row: 0x38C, also: &[], requests: &[] },
    // The Neoshadow's `ex_2420.bd` makes `0x80` in KH1's mode 3 (`sys 213(row, 3)`: on every frame until
    // `sys 213(row, 0)`, `kh1_core::enemy::guard::HELD`) at `0x16CE` / `0x1A32` / `0x2260` / `0x3108`: no request,
    // no windows.
    GuardSpec { enemy: "neoshadow", name: "shield", row: 0x80, also: &[], requests: &[] },
    // The Wight Knight's `ex_2200.bd` (M11): its body row `0xEC` (type 1, priority 100, all round) made with the combo's
    // start 200 (`0x187A`); its body row `0x15C` (type 2, priority 20, all round) made with each swipe (`0x1632` with 202,
    // `0x176A` with 201); its body row `0x54C` (the same numbers) made with the leap slash 216 (`0x1B2E`).
    GuardSpec { enemy: "wightknight", name: "windup", row: 0xEC, also: &[], requests: &[200] },
    GuardSpec { enemy: "wightknight", name: "swipe", row: 0x15C, also: &[], requests: &[201, 202] },
    GuardSpec { enemy: "wightknight", name: "leap", row: 0x54C, also: &[], requests: &[216] },
    // The Fat Bandit's `ex_2060.bd` makes `0xEC` (type 1, priority 100, 70° front) with `sys 213(row, 3)` at its init
    // (`0x254E`), after the breath (`0x1444`) and in its hit handler (`0x2ED6`); `0x13DC` removes it for the breath.
    GuardSpec { enemy: "fatbandit", name: "front", row: 0xEC, also: &[], requests: &[] },
    // The Defender's `ex_2340.bd` makes its shield's front row `0xEC` (type 1, priority 99, 70° front) with `sys 213(row,
    // 3)` at its init (`0x2828`) and removes it only at its death (`0x3230`); its spin makes `0x23C` (type 1, priority
    // 100, all round) with 208 (`0x1CBE`).
    GuardSpec { enemy: "defender", name: "front", row: 0xEC, also: &[], requests: &[] },
    GuardSpec { enemy: "defender", name: "spin", row: 0x23C, also: &[], requests: &[208] },
];

/// A guard row as read (the manifest's `guards` entry).
#[derive(Clone, Debug, PartialEq)]
pub struct Guard {
    /// Its name and script offset.
    pub name: &'static str,
    /// Its script offset.
    pub row: usize,
    /// The rows that act with it ([`GuardSpec::also`]).
    pub also: &'static [usize],
    /// `+0x38 & 7`.
    pub clash_type: u32,
    /// `+0x3C`.
    pub priority: i32,
    /// `+0x40 & 0xFFFF`.
    pub recoil: u32,
    /// `+0x34 & 3` (`row`'s).
    pub slot: u32,
    /// Every HIT slot of `row` and `also` (their windows are joined).
    pub slots: Vec<u32>,
    /// `+0x60` (radians).
    pub turn: f32,
    /// `+0x64`.
    pub cos: f32,
    /// Per request, the HIT slot's windows (KH1 frames).
    pub windows: Vec<(u16, Vec<(f32, f32)>)>,
}

/// Reads [`GUARDS`]' rows of an enemy: `+0x34` the HIT slot, `+0x38 & 7` the clash type (a guard
/// type, 1..4), `+0x3C` the priority, `+0x40 & 0xFFFF` the recoil (a request the model has), `+0x60`
/// the cone's turn, `+0x64` its cosine; per request, the slot's windows in its motion (KH1's rows
/// live while their HIT slot is on, `0x1402B47A0`). Checked (decomp: `0x1402B2DC0`, `0x1402B4640`,
/// `0x1402B4BC0`).
pub fn guard_rows(enemy: &str, script: &[u8], ms: &MotionSet, clips: &[RequestClip]) -> Result<Vec<Guard>> {
    let mut out = Vec::new();
    for g in GUARDS.iter().filter(|g| g.enemy == enemy) {
        // Each row: (slot, type, priority, recoil, turn, cos).
        let read = |at: usize| -> Result<(u32, u32, i32, u32, f32, f32)> {
            let r = script.get(at..at + 0x70).ok_or_else(|| crate::Error::Format(format!("{enemy}: no guard row at {at:#x}")))?;
            if !crate::stats::pushes_address(script, at as u32) {
                return bad(format!("{enemy}: the script never pushes the guard row {at:#x}"));
            }
            let u = |o: usize| u32::from_le_bytes(r[o..o + 4].try_into().unwrap());
            let f = |o: usize| f32::from_le_bytes(r[o..o + 4].try_into().unwrap());
            Ok((u(0x34) & 3, u(0x38) & 7, u(0x3C) as i32, u(0x40) & 0xFFFF, f(0x60), f(0x64)))
        };
        let (slot, clash_type, priority, recoil, turn, cos) = read(g.row)?;
        if !(1..=4).contains(&clash_type) || !(0..=100).contains(&priority) {
            return bad(format!("{enemy}: row {:#x} is not a guard row (type {clash_type}, priority {priority})", g.row));
        }
        // A row at priority 98 or more never gets a clash record (`0x1402B4BC0` returns before any record for a side at
        // 100, and records only a side under 98): its recoil never plays, so the model needn't have it (the Large Body's
        // front row names 110 at 100, the Defender's 110 at 99).
        if priority < 98 && !clips.iter().any(|c| c.request as u32 == recoil) {
            return bad(format!("{enemy}: guard row {:#x}'s recoil {recoil} has no motion", g.row));
        }
        if !(turn.is_finite() && cos.is_finite() && (-1.0..=1.0).contains(&cos)) {
            return bad(format!("{enemy}: guard row {:#x}: cone {turn} / {cos}", g.row));
        }
        let mut slots = vec![slot];
        for &at in g.also {
            let (s2, t2, p2, r2, turn2, cos2) = read(at)?;
            if (t2, p2, r2) != (clash_type, priority, recoil) || turn2.to_bits() != turn.to_bits() || cos2.to_bits() != cos.to_bits() {
                return bad(format!("{enemy}: guard row {at:#x} (type {t2}, priority {p2}, recoil {r2}, cone {turn2} / {cos2}) differs from {:#x}'s", g.row));
            }
            if !slots.contains(&s2) {
                slots.push(s2);
            }
        }
        let mut windows = Vec::new();
        for &req in g.requests {
            let e = ms.requests.get(req as usize).and_then(|w| w.entry()).ok_or_else(|| crate::Error::Format(format!("{enemy}: request {req} has no motion")))? as usize;
            let m = ms.motions.get(e).ok_or_else(|| crate::Error::Format(format!("{enemy}: request {req}: entry {e}")))?;
            let mut on = Vec::new();
            for &sl in &slots {
                on.extend(hit_windows(m, sl as u16)?);
            }
            windows.push((req, merge_windows(on)));
        }
        out.push(Guard { name: g.name, row: g.row, also: g.also, clash_type, priority, recoil, slot, slots, turn, cos, windows });
    }
    Ok(out)
}

/// Windows sorted and joined where they touch or overlap.
fn merge_windows(mut on: Vec<(f32, f32)>) -> Vec<(f32, f32)> {
    on.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out: Vec<(f32, f32)> = Vec::new();
    for w in on {
        match out.last_mut() {
            Some(l) if w.0 <= l.1 => l.1 = l.1.max(w.1),
            _ => out.push(w),
        }
    }
    out
}

/// An enemy's thrown object (M11, `kh1_core::enemy::thrown`): its child script (entry 0 of the
/// `.mdls` part 3, by name), the constants pushed there, the bone code push (`sys 19`'s second
/// argument), the parent's attack row the throw passes (`sys 94`), and the Re:CoM attack id whose
/// power it takes (decision M10).
pub struct ThrownSpec {
    /// The enemy (the spec's `name`).
    pub enemy: &'static str,
    /// The constants' prefix (`blade` → `blade_life`).
    pub prefix: &'static str,
    /// The child script's name in its header.
    pub child: &'static [u8],
    /// Its constants' pushes.
    pub consts: &'static [(&'static str, &'static [usize])],
    /// The bone code's push (`sys 19`).
    pub bone_code_at: usize,
    /// The parent's attack row the throw passes.
    pub row: usize,
    /// The Re:CoM attack id whose power it takes.
    pub recom_id: u32,
}

/// The Bandit's scimitar (`ex_2090_00.bd`; Checked (files): the child disassembled): its init
/// `0x348` stores life 300 (push `0x3CE`), slide 60 (`0x3D8`), turn-back 120 (`0x3E2`), speed 10
/// (`0x3EC`), change −0.5 (`0x3F6`), turn 3° (`0x400`); the repelled slide's × 0.9 (`0x29A`); the
/// entry `0x5EC`'s `sys 19(parent, 28, 1, …)` (`0x5F6`); the throw `0x2360` passes row `0x1CC`. Its
/// power: Re:CoM's whirl (id 1; Assumption: the nearest by KH1's power, the row's `+0x14` 4 as the
/// spin's end and the leap, which take id 1; the slash's is 2).
pub const THROWN: &[ThrownSpec] = &[ThrownSpec {
    enemy: "bandit",
    prefix: "blade",
    child: b"ex_2090_00.bd",
    consts: &[("life", &[0x3CE]), ("slide", &[0x3D8]), ("turn_back", &[0x3E2]), ("speed", &[0x3EC]), ("accel", &[0x3F6]), ("turn", &[0x400]), ("slide_drag", &[0x29A])],
    bone_code_at: 0x5F6,
    row: 0x1CC,
    recom_id: 1,
}];

/// Entry `k` of a `.mdls` part holding a file list (u32 count, then the entries' `[start, end)`
/// offsets; part 3, the child scripts: `1, 0x80, 0x700, 0` for the Bandit's). Checked (files).
fn part_entry(part: &[u8], k: usize) -> Option<&[u8]> {
    let u = |o: usize| part.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize);
    let n = u(0)?;
    if k >= n || n > 64 {
        return None;
    }
    let (a, b) = (u(4 + 4 * k)?, u(8 + 4 * k)?);
    part.get(a..b.min(part.len()))
}

/// The Re:CoM bone a KH1 bone code stands for: its bone, or its nearest mapped ancestor.
fn recom_bone_of_code(mdls: &[u8], conv: &Converter, code: usize) -> Result<usize> {
    let codes = bone_codes(mdls)?;
    let mut k = *codes.get(code).filter(|b| **b >= 0).ok_or_else(|| crate::Error::Format(format!("bone code {code} names no bone")))? as usize;
    let rig = conv.rig();
    loop {
        if let Some(r) = conv.map().iter().position(|m| *m == Some(k)) {
            return Ok(r);
        }
        k = rig.parent(k).ok_or_else(|| crate::Error::Format(format!("bone code {code}: no mapped ancestor")))?;
    }
}

/// [`THROWN`]'s numbers of an enemy as `<prefix>_<name>` constants: the child's pushes, the row's
/// radius (`+0x5C`, a free-standing row: bone `−1`, `0x1402B2DC0` takes the radius as it is), the bone
/// (Re:CoM's index for the code), the Re:CoM attack id and its `.ASD` record's kind index (`+0x68`,
/// Re:CoM's kind table `0x14052A220`, `0x140179410`) and `+0x9C & 2` (`0x14014E320` passes it to the
/// power getter `0x14017F0E0`). Checked (decomp + files).
fn thrown_consts(enemy: &str, mdls: &[u8], script: &[u8], asd: &[u8], asd_shape: (usize, usize, usize), conv: &Converter) -> Result<Vec<(String, f32)>> {
    let mut out = Vec::new();
    for t in THROWN.iter().filter(|t| t.enemy == enemy) {
        let part = crate::stats::mdls_part(mdls, 3).ok_or_else(|| crate::Error::Format(format!("{enemy}: no child scripts (part 3)")))?;
        let child = part_entry(part, 0).ok_or_else(|| crate::Error::Format(format!("{enemy}: part 3 holds no child script")))?;
        for (n, v) in script_consts(child, t.child, t.consts)? {
            out.push((format!("{}_{n}", t.prefix), v));
        }
        let w = child.get(t.bone_code_at..t.bone_code_at + 6).ok_or_else(|| crate::Error::Format(format!("{enemy}: child short at {:#x}", t.bone_code_at)))?;
        if w[0] & 0x3F != 0x02 {
            return bad(format!("{enemy}: child {:#x} is not the bone code's push", t.bone_code_at));
        }
        let code = i32::from_le_bytes(w[2..6].try_into().unwrap());
        let bone = recom_bone_of_code(mdls, conv, usize::try_from(code).map_err(|_| crate::Error::Format(format!("bone code {code}")))?)?;
        let row = script.get(t.row..t.row + 0x70).ok_or_else(|| crate::Error::Format(format!("{enemy}: no row {:#x}", t.row)))?;
        if !crate::stats::pushes_address(script, t.row as u32) || i32::from_le_bytes(row[0x10..0x14].try_into().unwrap()) != -1 {
            return bad(format!("{enemy}: row {:#x} is not the thrown object's (pushed, bone −1)", t.row));
        }
        let radius = f32::from_le_bytes(row[0x5C..0x60].try_into().unwrap());
        if !(radius > 0.0 && radius < 1000.0) {
            return bad(format!("{enemy}: thrown row radius {radius}"));
        }
        let (n, first) = (asd_shape.0, asd_shape.1);
        let rec = (0..n).map(|i| &asd[first + i * REC..first + (i + 1) * REC]).find(|r| u32::from_le_bytes(r[0x64..0x68].try_into().unwrap()) == t.recom_id).ok_or_else(|| crate::Error::Format(format!("{enemy}: no Re:CoM record with attack id {}", t.recom_id)))?;
        let kind = u32::from_le_bytes(rec[0x68..0x6C].try_into().unwrap());
        let flag2 = u32::from_le_bytes(rec[0x9C..0xA0].try_into().unwrap()) & 2 != 0;
        if kind >= 8 {
            return bad(format!("{enemy}: record {} kind index {kind}", t.recom_id));
        }
        for (n, v) in [("radius", radius), ("bone", bone as f32), ("recom_id", t.recom_id as f32), ("kind_index", kind as f32), ("flag2", flag2 as u8 as f32)] {
            out.push((format!("{}_{n}", t.prefix), v));
        }
    }
    Ok(out)
}

/// Requests a script only ever plays from a later frame (KH1's request word `+4`: the start time of
/// the queue entry `0x14029F980` writes, added when the motion switches, `0x14029FC40`; Checked
/// (decomp); M11): (enemy, request, KH1 frame). Their clip is the motion from that frame
/// ([`Converter::convert_from`]), their attack records' windows and their sounds move by it, their
/// manifest length is the rest. The Air Soldier's slash: `0x22F2` `sys 13(wp, 201, 8, 20)`.
/// The Pirate's jump slash (`0xDEA` `sys 13(wp, 0x10000D2, 8, 8)`, an action: `Cmd::MotionAt` can't
/// carry it) and its spin's wind-up (`0xEEC` `sys 13(wp, 222, 8, 8)`), each played only there.
/// The Green Requiem's Cure: 212 from frame 8 (`0x37E` `sys 13(wp, 212, 8, 8)`) and 213 queued from frame 8 (`0x396`
/// `sys 14(wp, 213, 8, 8)`), each played only there.
pub const STARTS: &[(&str, u16, u32)] = &[("airsoldier", 201, 20), ("greenrequiem", 212, 8), ("greenrequiem", 213, 8), ("pirate", 210, 8), ("pirate", 222, 8)];

/// [`STARTS`]' requests of an enemy.
pub fn starts_of(name: &str) -> Vec<(u16, u32)> {
    STARTS.iter().filter(|s| s.0 == name).map(|s| (s.1, s.2)).collect()
}

/// Applies [`STARTS`] to a built enemy: the clip (each such request must be its entry's only one),
/// the new records of `first..` bound to it (their windows move back by the start; a window that
/// would start before 0 begins at 0), the clip's sounds (frames move back, a frame-0 sound stays),
/// the manifest length.
#[allow(clippy::too_many_arguments)]
fn apply_starts(name: &str, ms: &MotionSet, conv: &Converter, ann: &mut Vec<u8>, clips: &mut [RequestClip], asd: &mut [u8], first: usize, ses: &mut Vec<u8>) -> Result<()> {
    let starts = starts_of(name);
    if starts.is_empty() {
        return Ok(());
    }
    let mut a = Ann::parse(ann)?;
    // An enemy Re:CoM keeps no `.SES` for ([`NO_SES`]: the Green Requiem) has no sounds to move.
    let has_ses = !ses.is_empty();
    let mut sounds = if has_ses { crate::ses::parse(ses)? } else { Vec::new() };
    for (req, start) in starts {
        let Some(i) = clips.iter().position(|c| c.request == req) else { return bad(format!("{name}: request {req} has no clip")) };
        let c = clips[i].clone();
        if c.source != "kh1" || clips.iter().any(|d| d.entry == c.entry && d.request != req) {
            return bad(format!("{name}: request {req} (entry {}) shares its clip; it can't start at {start}", c.entry));
        }
        let m = &ms.motions[c.entry];
        let clip = conv.convert_from(m, &ClipSpec { name: c.name.clone(), looped: c.looped, root_motion: c.root, blend: 8 }, start)?;
        a.slots[c.slot].clip = Some(clip);
        clips[i].frames = m.frames - start;
        let u = |d: &[u8], o: usize| u32::from_le_bytes(d[o..o + 4].try_into().unwrap());
        let (n, off) = (u(asd, 0x10) as usize, u(asd, 0x14) as usize);
        for r in first..n {
            let at = off + REC * r;
            if asd[at + 0x40..at + 0x50] == name16(&c.name) {
                let shift = start * 50;
                let on = u(asd, at + 0x5C).saturating_sub(shift);
                let offw = u(asd, at + 0x60).saturating_sub(shift);
                asd[at + 0x5C..at + 0x60].copy_from_slice(&on.to_le_bytes());
                asd[at + 0x60..at + 0x64].copy_from_slice(&offw.to_le_bytes());
            }
        }
        for e in sounds.iter_mut().filter(|e| e.name() == c.name) {
            for ev in e.events.iter_mut() {
                let f = i32::from_le_bytes(ev[0x18..0x1C].try_into().unwrap());
                if f != 0 {
                    ev[0x18..0x1C].copy_from_slice(&(f - start as i32).max(0).to_le_bytes());
                }
            }
        }
    }
    a.pad_tail_like_recom();
    *ann = a.to_bytes();
    if has_ses {
        *ses = crate::ses::write(&sounds);
    }
    Ok(())
}

/// Requests whose new clip loops although its last pose isn't its first (M11): KH1 loops an enemy's motion
/// unless its request has bit 16 (`0x14029FC40` copies it to `+0x160` bit 3; `0x14029F0A0` loops back to
/// `+0x288`, 0 for enemies, or holds with it; Checked (decomp), § The Bandit), so a script that plays one past
/// its length sees it start again; Re:CoM holds a clip not marked looping. Only the requests a script plays
/// past their length: the Wyvern's wind-up 204 (108 frames) in its engaged charge `0xF52` (16 + 106 frames).
/// The Pirate's spin 217 (48 frames) in its spin `0xED0` (up to 180 frames).
pub const LOOPS: &[(&str, u16)] = &[("wyvern", 204), ("pirate", 217)];

/// Requests whose new clip holds at its end although its last pose is its first (M11): the script plays them only with
/// bit 16 (KH1 holds them, `0x14029FC40` → `+0x160` bit 3, `0x14029F0A0`), so a script that plays one again sees it held
/// at its end. The Defender's side steps 200 / 202 and turn-around 201 (`0x1AB8` / `0x19D2` / `0x1B96`: `0x100C8` /
/// `0x100CA` / `0x100C9`; Checked (files): no other request of them in its script).
pub const HELD_CLIPS: &[(&str, u16)] = &[("defender", 200), ("defender", 201), ("defender", 202)];

/// Requests that get their own clip although Re:CoM has their motion as an exact bake (M11): the
/// bake carries Re:CoM's own attack records (`.ASD`, bound by the clip's name), which would hit with
/// KH1's rows. The Darkball's dash 202 (`e023_051`, Re:CoM's record id 0) and bite 204 (`e023_053`,
/// id 1). Checked (files): Re:CoM's `EN0023.ASD`.
pub const OWN_CLIPS: &[(&str, u16)] = &[("darkball", 202), ("darkball", 204)];

/// Bone points a behaviour reads (M11): (enemy, name, KH1 bone code, request, KH1 frame): the code's
/// bone's place in that request's motion at that frame, in the actor's frame (KH1 units, y down,
/// root motion off), as the constants `<name>_x`, `_y`, `_z`. The Darkball's reappearance moves the
/// actor to bone code 0's place (`0x2A78`: `sys 19(actor, 0, 0)` → `0x14029EB20`: the code's bone's
/// position, `0x14029FAB0` / `0x1401D62E0`, Checked (decomp)) while its idle plays (blended in over 20
/// frames: frame 20, Assumption ±10 units).
pub const BONE_POINTS: &[(&str, &str, u32, u16, f32)] = &[("darkball", "core", 0, 0, 20.0)];

fn bone_points(name: &str, mdls: &[u8], ms: &MotionSet, conv: &Converter, clips: &[RequestClip]) -> Result<Vec<(String, f32)>> {
    let mut out = Vec::new();
    for &(_, point, code, request, frame) in BONE_POINTS.iter().filter(|b| b.0 == name) {
        let codes = bone_codes(mdls)?;
        let bone = *codes.get(code as usize).filter(|b| **b >= 0).ok_or_else(|| crate::Error::Format(format!("{name}: bone code {code} names no bone")))? as usize;
        let c = clips.iter().find(|c| c.request == request).ok_or_else(|| crate::Error::Format(format!("{name}: request {request} has no clip")))?;
        let m = &ms.motions[c.entry];
        let mut rig = conv.rig();
        rig.start_motion(m);
        let _ = conv.targets(&mut rig, m, frame, false);
        let t = rig.current().world[bone].t;
        for (axis, v) in ["x", "y", "z"].iter().zip([t[0], t[1], t[2]]) {
            let v = v as f32;
            if !v.is_finite() {
                return bad(format!("{name}: {point}_{axis} = {v}"));
            }
            out.push((format!("{point}_{axis}"), v));
        }
    }
    Ok(out)
}

/// A model's child script (`.mdls` part 3, its file list, [`part_entry`]: the table KH1's `sys 94` reads,
/// `actor+0x13C`, `0x1402BC030` → `0x14028F0A0`; Checked (decomp + files): the Search Ghost's one child
/// `ex_2230_00.bd` at 0x80..0x480, M11).
pub fn child_script(mdls: &[u8], idx: usize) -> Result<&[u8]> {
    let p = crate::stats::mdls_part(mdls, 3).ok_or_else(|| crate::Error::Format("no sub-object table (part 3)".into()))?;
    part_entry(p, idx).filter(|s| s.len() > 0x30).ok_or_else(|| crate::Error::Format(format!("part 3 holds no child script {idx}")))
}

/// Numbers a behaviour reads from one of its model's child scripts (M11): (enemy, sub-object, script name,
/// constants as [`Spec::consts`]). The Search Ghost's cone `ex_2230_00.bd` (`crate::searchghost`).
pub const CHILD_CONSTS: &[(&str, usize, &[u8], &[(&str, &[usize])], &[(u32, usize)])] = &[
    ("searchghost", 0, b"ex_2230_00.bd", crate::searchghost::CHILD_CONSTS, crate::searchghost::CHILD_REQUESTS),
    // The Large Body's stomp ring `ex_2050_00.bd` (`crate::largebody`, [`RINGS`]).
    ("largebody", 0, b"ex_2050_00.bd", crate::largebody::CHILD_CONSTS, crate::largebody::CHILD_REQUESTS),
    // The Fat Bandit's fireball `ex_2060_00.bd` and flame `ex_2060_01.bd` (`crate::fatbandit`, [`T_POINTS`]).
    ("fatbandit", 0, b"ex_2060_00.bd", crate::fatbandit::FIRE_CONSTS, crate::fatbandit::FIRE_REQUESTS),
    ("fatbandit", 1, b"ex_2060_01.bd", crate::fatbandit::FLAME_CONSTS, crate::fatbandit::FLAME_REQUESTS),
    // The Gargoyle's ball `ex_2220_00.bd` (`crate::gargoyle`, [`T_POINTS`], [`SHOTS`]).
    ("gargoyle", 0, b"ex_2220_00.bd", crate::gargoyle::BALL_CONSTS, crate::gargoyle::BALL_REQUESTS),
    // The Defender's flame `ex_2340_00.bd` and fireball `ex_2340_01.bd` (`crate::defender`, [`T_POINTS`]).
    ("defender", 0, b"ex_2340_00.bd", crate::defender::FLAME_CONSTS, crate::defender::FLAME_REQUESTS),
    ("defender", 1, b"ex_2340_01.bd", crate::defender::FIRE_CONSTS, crate::defender::FIRE_REQUESTS),
    // The Bouncywild's banana `ex_2040_00.bd`, tossed peel `ex_2040_01.bd` and dropped peel `ex_2040_02.bd`
    // (`crate::bouncywild`, [`T_POINTS`], [`SHOTS`]).
    ("bouncywild", 0, b"ex_2040_00.bd", crate::bouncywild::FIRE_CONSTS, crate::bouncywild::FIRE_REQUESTS),
    ("bouncywild", 1, b"ex_2040_01.bd", crate::bouncywild::PEEL_CONSTS, crate::bouncywild::PEEL_REQUESTS),
    ("bouncywild", 2, b"ex_2040_02.bd", crate::bouncywild::DROP_CONSTS, crate::bouncywild::DROP_REQUESTS),
    // The Green Requiem's Cure `ex_2140_00.bd` (`crate::greenrequiem`: its wait in 213).
    ("greenrequiem", 0, b"ex_2140_00.bd", crate::greenrequiem::CURE_CONSTS, crate::greenrequiem::CURE_REQUESTS),
    // The Red Nocturne's Fire `ex_2110_00.bd` and big Fire `ex_2110_01.bd` (`crate::rednocturne`, [`SHOTS`]).
    ("rednocturne", 0, b"ex_2110_00.bd", crate::rednocturne::FIRE_CONSTS, crate::rednocturne::FIRE_REQUESTS),
    ("rednocturne", 1, b"ex_2110_01.bd", crate::rednocturne::BIGFIRE_CONSTS, crate::rednocturne::BIGFIRE_REQUESTS),
    // The Blue Rhapsody's Blizzard `ex_2120_00.bd` (`crate::bluerhapsody`, [`T_POINTS`], [`SHOTS`]).
    ("bluerhapsody", 0, b"ex_2120_00.bd", crate::bluerhapsody::BLIZZARD_CONSTS, crate::bluerhapsody::BLIZZARD_REQUESTS),
    // The Yellow Opera's Thunder `ex_2130_00.bd` (`crate::yellowopera`, [`STRIKES`]).
    ("yellowopera", 0, b"ex_2130_00.bd", crate::yellowopera::THUNDER_CONSTS, crate::yellowopera::THUNDER_REQUESTS),
];

fn child_consts(name: &str, mdls: &[u8]) -> Result<Vec<(String, f32)>> {
    let mut out = Vec::new();
    for &(_, idx, script, table, requests) in CHILD_CONSTS.iter().filter(|c| c.0 == name) {
        let s = child_script(mdls, idx)?;
        out.extend(script_consts(s, script, table)?);
        check_requests(s, script, requests)?;
    }
    Ok(out)
}

/// Where a [`T_POINTS`] point's t comes from: a constant already read (a child's push), or a number of KH1's exe.
#[derive(Clone, Copy, Debug)]
pub enum TFrom {
    /// A constant already read, by name.
    Const(&'static str),
    /// A number of KH1's exe.
    Value(f32),
}

/// Points on a KH1 bone code's line the host reads each time it needs them (M11, the Fat Bandit): (enemy, name, KH1
/// bone code, t). KH1's `sys 19(actor, code, t)` (`0x14029EB20` → `0x1401D6320`: the code's bone's parent's world
/// place, then the lerp `0x1400E64D0` toward the bone by t; Checked (decomp)). The point is constant in the frame of
/// the bone's parent (the bone's local translation scaled by t), so it is written as `<name>_bone` (the Re:CoM bone
/// the converter maps to that KH1 parent: the same, not an ancestor; refused otherwise) and `<name>_x / _y / _z` (the
/// point in that bone's frame, metres, as the step's `.ASD` records hold theirs). The step checks it is the same in two
/// poses. The Fat Bandit's fireball start (`ex_2060_00.bd` `0x610`: code 4 at `fire_t`), the head's point its aim
/// starts from (`0x1402A2980`: `sys 19(actor, 0, 0.5)`, KH1's exe), the flame's ends (`ex_2060_01.bd` `0x2EC` /
/// `0x318`: code 0 at `flame_t0` / `flame_t1`).
pub const T_POINTS: &[(&str, &str, usize, TFrom)] = &[
    ("fatbandit", "fire_at", 4, TFrom::Const("fire_t")),
    ("fatbandit", "eye", 0, TFrom::Value(0.5)),
    ("fatbandit", "flame0", 0, TFrom::Const("flame_t0")),
    ("fatbandit", "flame1", 0, TFrom::Const("flame_t1")),
    // The Gargoyle's ball (`ex_2220_00.bd`): held at `sys 19(parent, 0x1027, 0)` (`0xFA`: bit 12, bone 39 itself; t 0:
    // its parent's place) 15 up, then eased toward `sys 19(parent, 28, 1)` (`0x1A6` / `0x1AC`: bone code 28's bone).
    ("gargoyle", "ball_hold", 0x1027, TFrom::Value(0.0)),
    ("gargoyle", "ball_at", 28, TFrom::Const("ball_ease_t")),
    // The Defender's fireball start (`ex_2340_01.bd` `0x600`: code `0x106C`, bone 108, at `fire_t`) and its flame's ends
    // (`ex_2340_00.bd` `0x2F8` / `0x336`: code `0x1069`, bone 105, at `flame_t0` / `flame_t1`).
    ("defender", "fire_at", 0x106C, TFrom::Const("fire_t")),
    ("defender", "flame0", 0x1069, TFrom::Const("flame_t0")),
    ("defender", "flame1", 0x1069, TFrom::Const("flame_t1")),
    // The Bouncywild's banana and peels all start at `sys 19(parent, 28, 1)` (`ex_2040_00.bd` `0x636`, `_01` `0x54A`,
    // `_02` `0x514`: code 28, the left hand, at t 1; the step checks the three t agree).
    ("bouncywild", "fire_at", 28, TFrom::Const("fire_t")),
    // The Blue Rhapsody's Blizzard starts at `sys 19(parent, 0, t)` (`ex_2120_00.bd` `0x246..0x25C`: bone code 0 at
    // `blizzard_t`, 0: the code's bone's parent's place).
    ("bluerhapsody", "blizzard_at", 0, TFrom::Const("blizzard_t")),
];

fn t_points(name: &str, mdls: &[u8], ms: &MotionSet, conv: &Converter, clips: &[RequestClip], consts: &[(String, f32)]) -> Result<Vec<(String, f32)>> {
    let mut out = Vec::new();
    for &(_, point, code, from) in T_POINTS.iter().filter(|p| p.0 == name) {
        let t = match from {
            TFrom::Value(v) => v,
            TFrom::Const(c) => consts.iter().find(|x| x.0 == c).map(|x| x.1).ok_or_else(|| crate::Error::Format(format!("{name}: {point}'s t {c} not read")))?,
        };
        // A code with bit 12 names a bone directly (`0x14029FAB0`: `code & 0xFFF`; Checked (decomp)).
        let k = if code & 0x1000 != 0 {
            code & 0xFFF
        } else {
            let codes = bone_codes(mdls)?;
            *codes.get(code).filter(|b| **b >= 0).ok_or_else(|| crate::Error::Format(format!("{name}: bone code {code} names no bone")))? as usize
        };
        let rig = conv.rig();
        let parent = rig.parent(k).ok_or_else(|| crate::Error::Format(format!("{name}: KH1 bone {k} has no parent")))?;
        let hand = conv.map().iter().position(|m| *m == Some(parent)).ok_or_else(|| crate::Error::Format(format!("{name}: {point}: KH1 bone {parent} (code {code}'s parent) has no Re:CoM bone")))?;
        // Two poses: the idle's first frame and another motion's middle; the point must agree (its parent's frame).
        let mut at = Vec::new();
        for (req, frame) in [(0u16, 0.0f32), (clips.iter().map(|c| c.request).filter(|&r| r >= 200).max().unwrap_or(0), 0.5)] {
            let c = clips.iter().find(|c| c.request == req).ok_or_else(|| crate::Error::Format(format!("{name}: request {req} has no clip")))?;
            let m = &ms.motions[c.entry];
            let mut rig = conv.rig();
            rig.start_motion(m);
            at.push(conv.kh1_point_in(&mut rig, m, frame * m.frames as f32, hand, k, t as f64));
        }
        if (0..3).any(|i| (at[0][i] - at[1][i]).abs() > 1e-3 || !at[0][i].is_finite()) {
            return bad(format!("{name}: {point} moves in its parent's frame ({:?} / {:?})", at[0], at[1]));
        }
        out.push((format!("{point}_bone"), hand as f32));
        for (axis, v) in ["x", "y", "z"].iter().zip(at[0]) {
            out.push((format!("{point}_{axis}"), v as f32));
        }
    }
    Ok(out)
}

/// A child object flying from its parent as one of Re:CoM's bullets (M11, the Fat Bandit's fireball): (enemy, the
/// constants' prefix, the parent's rows the script passes (`sys 94`), the Re:CoM attack id and kind its power is asked
/// with). The row (a free-standing row: bone −1) gives `<prefix>_radius` (`+0x5C`, `0x1402B2DC0` takes it as it is);
/// every row listed must be the same apart from its place. The Fat Bandit's `0x16FC` passes `0x1CC`, its volley `0x23C`
/// (Checked (files): the two rows' bytes equal); Re:CoM's own fireball takes its power with
/// `0x14017F0E0(enemy, 1, 1, 0)` (its state 3 `0x14018DF20`; Checked (decomp)).
///
/// The Gargoyle's ball: its row `0x3FC` (`ex_2220.bd` `0x1B24` passes it), Re:CoM's own Gargoyle shot's power
/// `0x14017F0E0(enemy, 1, 1, 0)` (its state 3 `0x14018E350`; Checked (decomp)); the row the ball takes when a swing
/// sends it back (`0x46C`, the child's `0x748`: owned by the swinger) gives `ball_bat_radius` (its id and kind unused:
/// the batted ball keeps its power, Guard's turn-back rule).
pub const SHOTS: &[(&str, &str, &[usize], u32, u32)] = &[
    ("fatbandit", "fire", &[0x1CC, 0x23C], 1, 1),
    ("gargoyle", "ball", &[0x3FC], 1, 1),
    ("gargoyle", "ball_bat", &[0x46C], 1, 1),
    ("defender", "fire", &[0x2AC], 1, 1),
    ("bouncywild", "fire", &[0xEC], 1, 1),
    // The Red Nocturne's Fire (`0x4FC` passes `0xEC` / `0x15C`) and big Fire (`0x490`: `0x1CC` / `0x23C`): Re:CoM's own
    // Red Nocturne shots take theirs with `0x14017F0E0(enemy, 0, 1)` (state 2 `0x140191C00`, the small shot) and
    // `(enemy, 1, 1)` (state 3 `0x140191CC0`, the big one) (Checked (decomp)); the batted rows give `*_bat_radius`.
    ("rednocturne", "fire", &[0xEC], 0, 1),
    ("rednocturne", "fire_bat", &[0x15C], 0, 1),
    ("rednocturne", "bigfire", &[0x1CC], 1, 1),
    ("rednocturne", "bigfire_bat", &[0x23C], 1, 1),
    // The Blue Rhapsody's Blizzard (`0x424` passes `0x1CC` and no second row: KH1's child ends on a clash, it is never
    // batted): Re:CoM's own Blue Rhapsody shot takes its power with `0x14017F0E0(enemy, 0, 1)` (state 3 `0x14018C2D0`;
    // Checked (decomp)).
    ("bluerhapsody", "blizzard", &[0x1CC], 0, 1),
];
///
/// The Defender's `0x15F6` passes `0x2AC` (its batted row `0x31C` is the child's second); Re:CoM's own fireball takes its
/// power with `0x14017F0E0(enemy, 1, 1, 0)` (its state 3 `0x14018DAF0`; Checked (decomp)).

fn shot_consts(name: &str, script: &[u8]) -> Result<Vec<(String, f32)>> {
    let mut out = Vec::new();
    for &(_, prefix, rows, id, kind) in SHOTS.iter().filter(|s| s.0 == name) {
        let row = |at: usize| -> Result<&[u8]> {
            let r = script.get(at..at + 0x70).ok_or_else(|| crate::Error::Format(format!("{name}: no row {at:#x}")))?;
            if !crate::stats::pushes_address(script, at as u32) || i32::from_le_bytes(r[0x10..0x14].try_into().unwrap()) != -1 {
                return bad(format!("{name}: row {at:#x} is not a shot's (pushed, bone −1)"));
            }
            Ok(r)
        };
        let first = row(rows[0])?;
        for &at in &rows[1..] {
            let r = row(at)?;
            // The rows' last word is their own serial; the rest must agree.
            if r[..0x6C] != first[..0x6C] {
                return bad(format!("{name}: rows {:#x} and {at:#x} differ", rows[0]));
            }
        }
        let radius = f32::from_le_bytes(first[0x5C..0x60].try_into().unwrap());
        if !(radius > 0.0 && radius < 1000.0) {
            return bad(format!("{name}: shot radius {radius}"));
        }
        out.push((format!("{prefix}_radius"), radius));
        out.push((format!("{prefix}_recom_id"), id as f32));
        out.push((format!("{prefix}_kind"), kind as f32));
    }
    Ok(out)
}

/// A child object that strikes an area once (M11, the Yellow Opera's Thunder `ex_2130_00.bd`): (enemy, the constants'
/// prefix, the parent's row the script passes (`sys 94`), the Re:CoM attack id and kind its power is asked with). KH1's
/// child makes the row free-standing at its mark (`sys 23`, bone −1) and applies its hit to each opponent inside its area
/// (`sys 90` → `0x1402B6AF0`, `sys 60` → `0x1402BF330`: no clash pass, so neither a swing nor a guard meets it; Checked
/// (decomp)): the row's radius is unused (0) and only its power matters. The Yellow Opera's `0x466` passes `0x23C`;
/// Re:CoM's own Yellow Opera Thunder takes its power with `0x14017F0E0(enemy, 0, 1)` (its state 2 `0x140196A80`; Checked
/// (decomp)).
pub const STRIKES: &[(&str, &str, usize, u32, u32)] = &[("yellowopera", "thunder", 0x23C, 0, 1)];

fn strike_consts(name: &str, script: &[u8]) -> Result<Vec<(String, f32)>> {
    let mut out = Vec::new();
    for &(_, prefix, at, id, kind) in STRIKES.iter().filter(|s| s.0 == name) {
        let r = script.get(at..at + 0x70).ok_or_else(|| crate::Error::Format(format!("{name}: no row {at:#x}")))?;
        if !crate::stats::pushes_address(script, at as u32) || i32::from_le_bytes(r[0x10..0x14].try_into().unwrap()) != -1 {
            return bad(format!("{name}: row {at:#x} is not a strike's (pushed, bone −1)"));
        }
        out.push((format!("{prefix}_recom_id"), id as f32));
        out.push((format!("{prefix}_kind"), kind as f32));
    }
    Ok(out)
}

/// Bones a behaviour looks from (M11, `kh1_core::enemy::Sight`): (enemy, KH1 bone code). KH1's `sys 19` (t
/// 0) and `sys 115` take the code's bone's **parent** (`0x1401D6320`'s lerp at t = 0, `0x14029EA00` →
/// `0x1401D62B0`); a code with bit 12 names a bone directly (`0x14029FAB0`: `code & 0xFFF`). Checked (decomp).
/// The constant `sight_bone` is the Re:CoM bone posed at that parent's KH1 world (the converter's map). The
/// Search Ghost's cone: `ex_2230_00.bd` `0x28C` (`sys 19(parent, 4141, 0)`) and `0x27A` (`sys 115(…, 45)`):
/// bone 45, an end bone the motions don't pose, whose parent 44 is Re:CoM's `l_eye`.
pub const SIGHTS: &[(&str, u32)] = &[("searchghost", 0x102D)];

/// Search-table entries a script's target searches read (M11, the Green Requiem's ally pick): (enemy, the constants'
/// prefix, the entry's script offset). KH1's `sys 102` (`0x1402BC0B0` → `0x1402B80A0`) and `sys 48` (`0x1402B8250`) take
/// 20-byte entries of percent weights (the first `w0` % the best, the next `w1` % the second, …; Checked (decomp)); the
/// Green Requiem's table at `0x190` (`0x9EA`: `0x14B4(&wp30, actor, 0x190)`), its Cure's entry 1 at `0x1A4` (`0x2CA`:
/// `0x1440(&wp30, 1, 5000, 1)` → `entry × 20 + table`; Checked (files)). Read as `<prefix>_0..3`.
pub const SEARCHES: &[(&str, &str, usize)] = &[("greenrequiem", "pick_weight", 0x1A4)];

fn search_consts(name: &str, script: &[u8]) -> Result<Vec<(String, f32)>> {
    let mut out = Vec::new();
    for &(_, prefix, at) in SEARCHES.iter().filter(|s| s.0 == name) {
        let w = script.get(at..at + 20).ok_or_else(|| crate::Error::Format(format!("{name}: no search entry at {at:#x}")))?;
        let v: Vec<i32> = (0..5).map(|k| i32::from_le_bytes(w[k * 4..k * 4 + 4].try_into().unwrap())).collect();
        if v.iter().any(|&x| !(0..=100).contains(&x)) || v.iter().sum::<i32>() > 100 {
            return bad(format!("{name}: search entry {at:#x} is not percent weights ({v:?})"));
        }
        out.extend(v[..4].iter().enumerate().map(|(k, &x)| (format!("{prefix}_{k}"), x as f32)));
    }
    Ok(out)
}

/// Prize tables a script's handler hands to `sys 122` as a reward (M11, `kh1_core::enemy::Reward`): (enemy,
/// the constants' prefix, the table's script offset). The Search Ghost's hold reward: `0x28EE` / `0x29C4`
/// push `0x580` for `func_1F64` (`sys 32` → `sys 122`). Checked (files).
pub const REWARDS: &[(&str, &str, u32)] = &[("searchghost", "reward_prize", 0x580)];

/// [`REWARDS`]' tables as `<prefix>_0..6` (the seven prize counts, `0x1402ABC40`'s, by KH1 prize kind). A
/// table that also rolls items is refused (a reward carries no items).
fn reward_consts(name: &str, script: &[u8]) -> Result<Vec<(String, f32)>> {
    let mut out = Vec::new();
    for &(_, prefix, at) in REWARDS.iter().filter(|r| r.0 == name) {
        let t = crate::stats::drop_table(script, at)?;
        if t.items.iter().any(|i| i.0 > 0.0) {
            return bad(format!("{name}: the reward table {at:#x} also rolls items ({:?}); not built", t.items));
        }
        out.extend(t.prizes.iter().enumerate().map(|(k, &n)| (format!("{prefix}_{k}"), n as f32)));
    }
    Ok(out)
}

fn sight_bones(name: &str, mdls: &[u8], conv: &Converter) -> Result<Vec<(String, f32)>> {
    let mut out = Vec::new();
    for &(_, code) in SIGHTS.iter().filter(|s| s.0 == name) {
        let bone = if code & 0x1000 != 0 {
            (code & 0xFFF) as usize
        } else {
            let codes = bone_codes(mdls)?;
            *codes.get(code as usize).filter(|b| **b >= 0).ok_or_else(|| crate::Error::Format(format!("{name}: bone code {code} names no bone")))? as usize
        };
        let parent = conv.rig().parent(bone).ok_or_else(|| crate::Error::Format(format!("{name}: KH1 bone {bone} has no parent")))?;
        let recom = conv.map().iter().position(|m| *m == Some(parent)).ok_or_else(|| crate::Error::Format(format!("{name}: KH1 bone {parent} has no Re:CoM bone")))?;
        out.push(("sight_bone".to_string(), recom as f32));
    }
    Ok(out)
}

/// Every enemy this step builds (M11); the Shadow (M10) is `crate::shadow`.
pub const SPECS: &[&Spec] = &[&crate::soldier::SPEC, &crate::airsoldier::SPEC, &crate::powerwild::SPEC, &crate::darkball::SPEC, &crate::bandit::SPEC, &crate::pirate::SPEC, &crate::airpirate::SPEC, &crate::wyvern::SPEC, &crate::searchghost::SPEC, &crate::largebody::SPEC, &crate::neoshadow::SPEC, &crate::wightknight::SPEC, &crate::fatbandit::SPEC, &crate::gargoyle::SPEC, &crate::defender::SPEC, &crate::bouncywild::SPEC, &crate::greenrequiem::SPEC, &crate::rednocturne::SPEC, &crate::bluerhapsody::SPEC, &crate::yellowopera::SPEC];

/// Tests only: the player's files for `spec` (`KH1_M11_DIR`: KH1's `.mdls` / `.mset`, Re:CoM's
/// `ENEMY__NNNN__ENNNNN.*`; `KH1_TEST_DATA`: `btltbl.bin`). `ses_required`: no Inputs without the
/// enemy's `.SES` (else an empty one, for the classes Re:CoM ships none for). One helper for every
/// enemy module's tests (0.178.4; each had its own copy).
#[cfg(test)]
pub(crate) fn test_inputs(spec: &Spec, ses_required: bool) -> Option<Inputs> {
    let d = std::path::PathBuf::from(std::env::var_os("KH1_M11_DIR")?);
    let t = std::path::PathBuf::from(std::env::var_os("KH1_TEST_DATA")?);
    let r = |n: &str| std::fs::read(d.join(n)).ok();
    let ses = r(&spec.flat("SES"));
    Some(Inputs {
        mset: r(&format!("{}.mset", spec.kh1))?,
        mdls: r(&format!("{}.mdls", spec.kh1))?,
        ann: r(&spec.flat("ANN"))?,
        mdl: r(&spec.flat("MDL"))?,
        asd: r(&spec.flat("ASD"))?,
        ses: if ses_required { ses? } else { ses.unwrap_or_default() },
        btltbl: std::fs::read(t.join("btltbl.bin")).ok()?,
    })
}

#[cfg(test)]
mod clash_tests {
    use super::*;

    /// The player's files (`KH1_M11_DIR`: KH1's `.mdls` / `.mset`, Re:CoM's `ENEMY__NNNN__ENNNNN.*`;
    /// `KH1_TEST_DATA`: `btltbl.bin`) for one spec.
    fn inputs(spec: &Spec) -> Option<Inputs> {
        test_inputs(spec, false)
    }

    /// M11, attack meets attack: every ported class's manifest lists, per attack row, KH1's clash type,
    /// priority and cone and the ASD records made from it; each record is one of the step's (after Re:CoM's) on
    /// the attack's clip. The numbers the doc's table gives (the players' scripts).
    #[test]
    fn every_class_lists_its_attack_rows_clash_numbers() {
        let mut seen = 0;
        for spec in SPECS {
            let Some(inp) = inputs(spec) else { continue };
            let Ok(b) = build(spec, &inp) else { continue };
            seen += 1;
            let n: usize = u32::from_le_bytes(b.asd[0x10..0x14].try_into().unwrap()) as usize;
            let off: usize = u32::from_le_bytes(b.asd[0x14..0x18].try_into().unwrap()) as usize;
            assert_eq!(b.clash.len(), spec.attacks.len());
            for (clash, spec_a) in b.clash.iter().zip(spec.attacks) {
                assert_eq!(clash.len(), spec_a.rows.len(), "{} {}", spec.name, spec_a.name);
                let clip = b.clips.iter().find(|c| c.request == spec_a.request).unwrap();
                for c in clash {
                    assert!(!c.records.is_empty(), "{} {}: a row without records", spec.name, spec_a.name);
                    for &i in &c.records {
                        let i = i as usize;
                        assert!(i >= spec.asd.0 && i < n, "{} {}: record {i}", spec.name, spec_a.name);
                        assert_eq!(&b.asd[off + REC * i + 0x40..off + REC * i + 0x50], &name16(&clip.name), "{} {}: record {i} on another clip", spec.name, spec_a.name);
                    }
                    assert!(b.manifest.contains(&c.json()), "{} {}: {}", spec.name, spec_a.name, c.json());
                }
            }
            let row = |attack: &str, k: usize| {
                let i = spec.attacks.iter().position(|a| a.name == attack).unwrap();
                let c = &b.clash[i][k];
                (c.clash_type, c.priority, c.cos)
            };
            match spec.name {
                "largebody" => {
                    assert_eq!((row("punch", 0), row("punch", 1)), ((0, 30, 1.0), (0, 30, 1.0)));
                    assert_eq!((row("slam", 0), row("charge", 0)), ((0, 30, 1.0), (0, 90, 1.0)));
                    assert!(b.clash[3].is_empty(), "the ring is a child row");
                }
                "soldier" => {
                    assert_eq!((row("kick", 0), row("jumping kick", 0)), ((0, 20, 1.0), (0, 100, 1.0)));
                    assert_eq!(row("spin kick", 1), (1, 100, -0.5), "the spin kick's chest: a guard type");
                }
                "powerwild" => assert_eq!((row("swipe", 0), row("flurry", 2)), ((0, 50, 1.0), (2, 10, 0.0))),
                "pirate" => assert_eq!(row("slash", 0), (0, 90, 1.0)),
                "wyvern" => assert_eq!(row("charge", 0), (3, 100, -1.0)),
                "neoshadow" => assert_eq!((row("claw", 0), row("dive", 0)), ((0, 40, 1.0), (0, 100, 1.0))),
                "bandit" => assert_eq!(row("leap", 1), (0, 30, 1.0)),
                "searchghost" => assert_eq!((row("burst", 0), row("burst", 2)), ((0, 40, 1.0), (0, 70, 1.0))),
                _ => {}
            }
        }
        if seen == 0 {
            eprintln!("attack-meets-attack manifest test skipped: KH1_M11_DIR / KH1_TEST_DATA not set");
        }
    }

    /// Fix (owner, 2026-11-02, "Clash choices": block only): the attack rows KH1 never tests against a hurt
    /// body (clash types 1, 2, 4, `0x1402B3BE0`) are, of every ported class, only the Soldier's spin-kick chest,
    /// the Powerwild's flurry body and the Gargoyle's two body rows (M11). The host reads their type from the manifest (`kh1_enemy::Manifest::
    /// never_hurts`), so the manifests the step already wrote serve.
    #[test]
    fn only_two_attack_rows_never_hurt() {
        let mut seen = 0;
        let mut found = Vec::new();
        for spec in SPECS {
            let Some(inp) = inputs(spec) else { continue };
            let Ok(b) = build(spec, &inp) else { continue };
            seen += 1;
            for (clash, a) in b.clash.iter().zip(spec.attacks) {
                for (i, c) in clash.iter().enumerate() {
                    if !kh1_core::enemy::guard::hurts_bodies(c.clash_type) {
                        found.push((spec.name, a.name, i, c.row, c.clash_type, c.priority));
                    }
                }
            }
        }
        if seen == 0 {
            eprintln!("never-hurting rows test skipped: KH1_M11_DIR / KH1_TEST_DATA not set");
            return;
        }
        // M11, the Gargoyle: its swipe's and its swipe end's body rows (type 2, priority 30, front half); the Blue Rhapsody's
        // melee body row (type 2, priority 30); the Yellow Opera's charge body row (type 2, priority 30).
        let want = [("soldier", "spin kick", 1, 0x2AC, 1, 100), ("powerwild", "flurry", 2, 0x1CC, 2, 10), ("gargoyle", "swipe", 2, 0x2AC, 2, 30), ("gargoyle", "swipe end", 1, 0x38C, 2, 30), ("bluerhapsody", "melee", 1, 0x15C, 2, 30), ("yellowopera", "charge", 1, 0x15C, 2, 30)];
        for w in want {
            if seen == SPECS.len() || found.iter().any(|f| f.0 == w.0) {
                assert!(found.contains(&w), "{w:?} not among {found:?}");
            }
        }
        assert!(found.iter().all(|f| want.contains(f)), "{found:?}");
    }
}
