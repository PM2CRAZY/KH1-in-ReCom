//! The whole step: Re:CoM's `PL0001.ANN` + KH1's motions → the KH1-entry → slot map
//! (manifest) and, for the KH1 motions Re:CoM lacks, the extended pack.
//!
//! - Owner decision (2026-10-30): "just use RECOM's default animations, and KH1 animations for
//!   anything that isnt included in ReCOM". Every KH1 entry the request map references (all
//!   118 of Sora's) is looked for among Re:CoM's clips ([`crate::matching`]). Found: the entry
//!   plays Re:CoM's clip at Re:CoM's slot. Not found: it gets one new slot, in entry order,
//!   after Re:CoM's last slot: slot `base + i` (i = rank among the KH1-only entries), named
//!   `p001_NNN` (the name must agree with the slot for `FUN_1401A47E0`'s fast path).
//! - No KH1-only entry: the pack is Re:CoM's own ([`Built::changed`] false; write nothing).
//! - The Keyblade (`PL0002.ANN`, when given): a clip at the same slot for every new Sora slot
//!   (`w001_NNN`; the weapon plays Sora's index, `FUN_1401F7180`), Re:CoM's 158 slots kept,
//!   158..160 left empty. Entries on Re:CoM's slots use Re:CoM's Keyblade clip there (or none,
//!   as in Re:CoM).
//! - Root: KH1 request-map bit 15 of the requests that play the entry (Sora has no entry
//!   played both with and without it; a conflict is an error).
//! - Loop: KH1 has no per-motion loop flag (the request call decides: `0x10000` = hold;
//!   `kh1-movement-motion.md` §5). A clip loops when KH1's pose at `t = N` returns to the pose
//!   at `t = 0` (every mapped bone within [`LOOP_TOLERANCE_M`], root travel included).
//!   Assumption; it agrees with all of Square Enix's own Re:CoM bakes of KH1 motions (test
//!   `validate`). [`BuildOptions::loop_overrides`] forces either way (new clips only; Re:CoM's
//!   clips keep their own flag).

use kh1_formats::mset::MotionSet;

use crate::ann::{name16, Ann, Slot};
use crate::convert::{ClipSpec, Converter};
use crate::matching::{decode_pack, kh1_weapon_locals, match_all, score, DecodedClip, EntryMatch, MatchKind, Query};
use crate::math::dist;
use crate::mdl::RecomSkeleton;
use crate::{bad, Result};

/// Max bone-position difference (metres) between `t = 0` and `t = N` for a looping motion.
pub const LOOP_TOLERANCE_M: f64 = 0.01;

/// Re:CoM's own slot count for Sora (`PL0001.ANN`, Checked (files)).
pub const RECOM_SORA_SLOTS: usize = 161;

/// Re:CoM's own slot count for the Keyblade (`PL0002.ANN`, Checked (files)).
pub const RECOM_KEYBLADE_SLOTS: usize = 158;

/// The Sora bone the Keyblade's bone 0 hangs from (`FUN_1401FA6E0` → `FUN_140224BA0`).
pub const KEYBLADE_ATTACH: &str = "WeaponR";

/// The four input files.
#[derive(Clone, Debug)]
pub struct Inputs {
    /// KH1 `xa_ex_0010.mset`.
    pub mset: Vec<u8>,
    /// KH1 `xa_ex_0010.mdls`.
    pub mdls: Vec<u8>,
    /// Re:CoM `PLAYER/0001/PL0001.ANN` (the original, from `Recom.pkg`).
    pub ann: Vec<u8>,
    /// Re:CoM `PLAYER/0001/PL0001.MDL`.
    pub mdl: Vec<u8>,
    /// Re:CoM `PLAYER/0002/PL0002.ANN` and `.MDL` (the Keyblade), when present.
    pub weapon: Option<(Vec<u8>, Vec<u8>)>,
}

/// Options.
#[derive(Clone, Debug)]
pub struct BuildOptions {
    /// Name prefix of the new clips.
    pub prefix: String,
    /// Name prefix of the new Keyblade clips.
    pub weapon_prefix: String,
    /// Default blend written in each new clip (60 Hz frames). KH1 has no per-motion default
    /// blend (no field in the motion header or the request-map word; the code passes the blend
    /// with each request call, `kh1-movement-motion.md` §5), so this is Re:CoM's own default, 8.
    /// The runtime passes KH1's blend per request; this is only used when it passes a negative
    /// blend (`FUN_1401A3470`).
    pub blend: u16,
    /// `(entry, looped)` overrides of the loop rule.
    pub loop_overrides: Vec<(usize, bool)>,
    /// Only these entries (tests); `None` = every referenced entry. The matching still runs on
    /// all entries, so a subset build has the same slot numbers.
    pub only: Option<Vec<usize>>,
    /// Require the base pack to have exactly [`RECOM_SORA_SLOTS`] slots.
    pub strict_base: bool,
    /// Play Re:CoM's own clip for every KH1 entry Re:CoM already has ([`crate::matching`];
    /// owner decision 2026-10-30). `false` converts every entry (tests).
    pub reuse_recom: bool,
}

impl Default for BuildOptions {
    fn default() -> Self {
        BuildOptions { prefix: "p001_".into(), weapon_prefix: "w001_".into(), blend: 8, loop_overrides: Vec::new(), only: None, strict_base: true, reuse_recom: true }
    }
}

/// Where an entry's clip comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// Re:CoM's own clip (a bake of the KH1 motion, possibly with Re:CoM's edits).
    Recom,
    /// A new 60 fps clip converted from KH1.
    Kh1,
}

impl Source {
    /// Manifest text.
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Recom => "recom",
            Source::Kh1 => "kh1",
        }
    }
}

/// One KH1 entry and the slot that plays it.
#[derive(Clone, Debug, PartialEq)]
pub struct ManifestEntry {
    /// KH1 mset entry.
    pub kh1_entry: usize,
    /// Request ids whose map word (or its type-3/4 variants) plays this entry.
    pub requests: Vec<u16>,
    /// The main request: the lowest id whose map word names this entry as its base entry, else
    /// the lowest id that reaches it as a type-3/4 variant.
    pub kh1_request: Option<u16>,
    /// KH1 length N (60 Hz frames).
    pub kh1_frames: u32,
    /// Re:CoM slot to play: Re:CoM's own clip when it has the motion, else the new slot.
    pub slot: usize,
    /// Clip name at that slot.
    pub name: String,
    /// Where the clip comes from.
    pub source: Source,
    /// For [`Source::Recom`]: bake or edited ([`crate::matching`]).
    pub match_kind: Option<MatchKind>,
    /// For [`Source::Recom`]: worst agreeing-bone rotation error (degrees).
    pub match_error: Option<f64>,
    /// For [`Source::Recom`]: bones Re:CoM edited (names).
    pub edited_bones: Vec<String>,
    /// For [`Source::Recom`]: how differently Re:CoM splits the travel between `Root` and
    /// `ex_0010_sk` (mm; 0 when it splits it as KH1's request-map root flag says).
    pub root_split_mm: Option<f64>,
    /// For [`Source::Recom`]: other clips that also pass the rule (should be empty).
    pub ambiguous: Vec<usize>,
    /// Clip frames.
    pub frames: usize,
    /// Clip frame rate (Re:CoM's clips 30, new clips 60).
    pub fps: u32,
    /// Looping clip.
    pub looped: bool,
    /// Whether the loop flag came from an override (new clips only).
    pub loop_overridden: bool,
    /// Pose difference t = 0 vs t = N, metres (the loop rule's measure).
    pub cycle_error_m: f64,
    /// KH1 request-map bit 15 (root travel in `Root`).
    pub root_motion: bool,
    /// New clips: clip bytes and keys stored.
    pub bytes: usize,
    /// New clips: keys stored (all animated channels).
    pub keys: usize,
    /// Name of the Keyblade clip at the same slot of `PL0002.ANN` (none: Re:CoM has no
    /// Keyblade clip there, the Keyblade gets a null clip as in Re:CoM).
    pub weapon_name: Option<String>,
    /// For [`Source::Recom`] with a Keyblade clip: its worst bone rotation error against KH1's
    /// blade (degrees).
    pub weapon_match_error: Option<f64>,
}

/// The result.
#[derive(Clone, Debug)]
pub struct Built {
    /// The pack (extended, or Re:CoM's own when no entry needs a new slot).
    pub ann: Ann,
    /// Its bytes.
    pub bytes: Vec<u8>,
    /// Slots of the base pack.
    pub base_slots: usize,
    /// Whether the pack differs from Re:CoM's (some KH1 entry got a new slot). When false,
    /// nothing should be written: Panacea then serves Re:CoM's own files.
    pub changed: bool,
    /// One entry per KH1 entry, in entry order.
    pub entries: Vec<ManifestEntry>,
    /// The extended Keyblade pack, when the inputs had one and the pack changed.
    pub weapon: Option<WeaponPack>,
}

/// The extended `PL0002.ANN`.
#[derive(Clone, Debug)]
pub struct WeaponPack {
    /// The pack.
    pub ann: Ann,
    /// Its bytes.
    pub bytes: Vec<u8>,
    /// Slots of the base pack.
    pub base_slots: usize,
}

/// The main request of an entry (see [`ManifestEntry::kh1_request`]).
pub fn main_request(ms: &MotionSet, entry: usize, requests: &[u16]) -> Option<u16> {
    requests.iter().copied().find(|&r| ms.requests[r as usize].entry() == Some(entry as u16)).or_else(|| requests.first().copied())
}

/// Per entry (ascending): request ids (ascending) and root flag.
pub fn referenced_entries(ms: &MotionSet) -> Result<Vec<(usize, Vec<u16>, bool)>> {
    let mut by: Vec<Option<(Vec<u16>, bool)>> = vec![None; ms.motions.len()];
    for (rid, w) in ms.requests.iter().enumerate() {
        for e in w.variants() {
            let e = e as usize;
            if e >= by.len() {
                return bad(format!("request {rid:#x} names entry {e}, past the {} motions", by.len()));
            }
            match &mut by[e] {
                Some((ids, root)) => {
                    if *root != w.root_motion() {
                        return bad(format!("entry {e} is played with and without root motion (request {rid:#x})"));
                    }
                    ids.push(rid as u16);
                }
                slot @ None => *slot = Some((vec![rid as u16], w.root_motion())),
            }
        }
    }
    Ok(by.into_iter().enumerate().filter_map(|(e, v)| v.map(|(ids, root)| (e, ids, root))).collect())
}

/// Builds the pack and the map.
pub fn build(inp: &Inputs, opt: &BuildOptions) -> Result<Built> {
    let ms = MotionSet::parse(&inp.mset)?;
    let skel = RecomSkeleton::parse(&inp.mdl)?;
    let mut ann = Ann::parse(&inp.ann)?;
    let base = ann.slots.len();
    if opt.strict_base && base != RECOM_SORA_SLOTS {
        return bad(format!("PL0001.ANN has {base} slots, expected Re:CoM's own {RECOM_SORA_SLOTS} (already extended?)"));
    }
    for (i, s) in ann.slots.iter().enumerate() {
        if let Some(c) = &s.clip {
            if c.bones as usize != skel.bones.len() {
                return bad(format!("slot {i}: {} bones, skeleton has {}", c.bones, skel.bones.len()));
            }
        }
    }
    let names: Vec<String> = skel.bones.iter().map(|b| b.name.clone()).collect();
    let conv = Converter::new(&inp.mdls, skel)?;
    // The Keyblade plays Sora's motion index on its own set (`FUN_1401F7180`), so its new clips
    // go to the same slots; Re:CoM's own slots stay as they are.
    let mut weapon = match &inp.weapon {
        None => None,
        Some((wann, wmdl)) => {
            let wa = Ann::parse(wann)?;
            let wbase = wa.slots.len();
            if opt.strict_base && wbase != RECOM_KEYBLADE_SLOTS {
                return bad(format!("PL0002.ANN has {wbase} slots, expected Re:CoM's own {RECOM_KEYBLADE_SLOTS} (already extended?)"));
            }
            if wbase > base {
                return bad(format!("PL0002.ANN has {wbase} slots, more than PL0001.ANN's {base}"));
            }
            let rig = conv.weapon(RecomSkeleton::parse(wmdl)?, KEYBLADE_ATTACH)?;
            Some((wa, rig, wbase))
        }
    };
    let refs = referenced_entries(&ms)?;
    // Which KH1 motions Re:CoM already has (all entries, so slot numbers don't depend on
    // `only`).
    let found: Vec<Option<EntryMatch>> = if opt.reuse_recom {
        let clips = decode_pack(&ann);
        let qs: Vec<Query> = refs.iter().map(|(e, _, root)| Query { entry: *e, motion: &ms.motions[*e], root_motion: *root }).collect();
        match_all(&conv, &qs, &clips).into_iter().map(|m| m.found.is_some().then_some(m)).collect()
    } else {
        vec![None; refs.len()]
    };
    let wclips: Vec<DecodedClip> = weapon.as_ref().map_or(Vec::new(), |(wa, _, _)| decode_pack(wa));
    let mut entries = Vec::new();
    let mut new_rank = 0usize;
    for ((e, requests, root), fm) in refs.into_iter().zip(found) {
        let m = &ms.motions[e];
        let kh1_request = main_request(&ms, e, &requests);
        let cycle = cycle_error(&conv, m);
        if let Some(fm) = fm {
            if opt.only.as_ref().is_some_and(|o| !o.contains(&e)) {
                continue;
            }
            let (sc, kind) = fm.found.expect("found");
            let slot = sc.slot;
            let c = ann.slots[slot].clip.as_ref().expect("matched slot has a clip");
            let (weapon_name, weapon_match_error) = match weapon.as_ref().and_then(|(wa, rig, _)| wa.slots.get(slot).and_then(|s| s.clip.as_ref()).map(|wc| (wc, rig))) {
                Some((wc, rig)) => {
                    let dc = wclips.iter().find(|d| d.slot == slot).expect("decoded");
                    let kh1 = kh1_weapon_locals(&conv, rig, m, dc.frames);
                    (Some(wc.name_str()), Some(score(&kh1, dc).max_rot()))
                }
                None => (None, None),
            };
            entries.push(ManifestEntry {
                kh1_entry: e,
                kh1_request,
                requests,
                kh1_frames: m.frames,
                slot,
                name: ann.slots[slot].name_str(),
                source: Source::Recom,
                match_kind: Some(kind),
                match_error: Some(sc.match_error()),
                edited_bones: sc.disagreeing().into_iter().map(|b| names[b].clone()).collect(),
                root_split_mm: Some(sc.root_split_mm()),
                ambiguous: fm.ambiguous,
                frames: c.frames as usize,
                fps: c.fps(),
                looped: c.looped(),
                loop_overridden: false,
                cycle_error_m: cycle,
                root_motion: root,
                bytes: 0,
                keys: 0,
                weapon_name,
                weapon_match_error,
            });
            continue;
        }
        let slot = base + new_rank;
        new_rank += 1;
        if opt.only.as_ref().is_some_and(|o| !o.contains(&e)) {
            continue;
        }
        let ov = opt.loop_overrides.iter().find(|(x, _)| *x == e).map(|&(_, l)| l);
        let looped = ov.unwrap_or(cycle <= LOOP_TOLERANCE_M);
        let name = format!("{}{slot:03}", opt.prefix);
        let clip = conv.convert(m, &ClipSpec { name: name.clone(), looped, root_motion: root, blend: opt.blend })?;
        let mut weapon_name = None;
        if let Some((wa, rig, _)) = weapon.as_mut() {
            let wname = format!("{}{slot:03}", opt.weapon_prefix);
            let wclip = conv.convert_weapon(rig, m, &ClipSpec { name: wname.clone(), looped, root_motion: false, blend: opt.blend })?;
            while wa.slots.len() < slot {
                wa.slots.push(Slot { name: [0; 16], clip: None });
            }
            wa.slots.push(Slot { name: name16(&wname), clip: Some(wclip) });
            weapon_name = Some(wname);
        }
        let bytes = clip.to_bytes().len();
        let keys = clip.channels.iter().map(|c| if let crate::ann::Channel::Curve(cv) = c { cv.keys.len() } else { 0 }).sum();
        entries.push(ManifestEntry {
            kh1_entry: e,
            kh1_request,
            requests,
            kh1_frames: m.frames,
            slot,
            name: name.clone(),
            source: Source::Kh1,
            match_kind: None,
            match_error: None,
            edited_bones: Vec::new(),
            root_split_mm: None,
            ambiguous: Vec::new(),
            frames: clip.frames as usize,
            fps: clip.fps(),
            looped,
            loop_overridden: ov.is_some(),
            cycle_error_m: cycle,
            root_motion: root,
            bytes,
            keys,
            weapon_name,
            weapon_match_error: None,
        });
        while ann.slots.len() < slot {
            ann.slots.push(Slot { name: [0; 16], clip: None });
        }
        ann.slots.push(Slot { name: name16(&name), clip: Some(clip) });
    }
    let changed = ann.slots.len() > base;
    if !changed {
        // Nothing new: Re:CoM's own pack, untouched (Panacea serves it when nothing is written).
        return Ok(Built { ann, bytes: inp.ann.clone(), base_slots: base, changed, entries, weapon: None });
    }
    ann.pad_tail_like_recom();
    let bytes = ann.to_bytes();
    let weapon = weapon.map(|(mut wa, _, wbase)| {
        wa.pad_tail_like_recom();
        let bytes = wa.to_bytes();
        WeaponPack { ann: wa, bytes, base_slots: wbase }
    });
    Ok(Built { ann, bytes, base_slots: base, changed, entries, weapon })
}

/// Sora's Halloween Town body pack (`PLAYER/0011/PL0011.ANN`) with the same new clips at the
/// same slots as the extended `PL0001.ANN`.
///
/// - Why: in Halloween Town Re:CoM gives Sora group 11 (`PL0011.MDL` / `.ANN`), whose pack is a
///   reduced copy of `PL0001.ANN`: its clips keep their `p001_NNN` names at the same slots,
///   with empty slots between. Our action-table rows (Guard, Counterattack) name `p001_225` /
///   `p001_226`, which the Halloween pack lacked, so the Guard never started there (owner's
///   trace 12: motion time stuck at 0 in a Halloween Town battle).
/// - Checked (files): the skeletons match: 135 bones, same names, rest pose within 0.06 mm.
/// - Same slots, because Re:CoM's name lookup `0x1401A47E0` takes the index from the name's
///   digits first, and the Keyblade plays Sora's clip index on its own pack (`0x1401F7180`).
/// - Re:CoM's own slots are kept. Slots up to `PL0001`'s base are padded empty; ours are added
///   after.
pub fn extend_costume_pack(built: &Built, costume_ann: &[u8]) -> Result<Vec<u8>> {
    let mut ann = Ann::parse(costume_ann)?;
    let ours = &built.ann.slots[built.base_slots..];
    if ann.slots.len() > built.base_slots {
        return bad(format!("costume pack has {} slots, more than PL0001.ANN's {} (already extended?)", ann.slots.len(), built.base_slots));
    }
    let bones = built.ann.slots.iter().filter_map(|s| s.clip.as_ref()).map(|c| c.bones).next();
    if let Some(c) = ann.slots.iter().filter_map(|s| s.clip.as_ref()).find(|c| Some(c.bones) != bones) {
        return bad(format!("costume pack clip {} has {} bones, PL0001's have {:?}", c.name_str(), c.bones, bones));
    }
    while ann.slots.len() < built.base_slots {
        ann.slots.push(Slot { name: [0; 16], clip: None });
    }
    ann.slots.extend(ours.iter().cloned());
    ann.pad_tail_like_recom();
    Ok(ann.to_bytes())
}

/// Largest mapped-bone position difference between KH1's pose at `t = 0` and `t = N` in model
/// space, root travel included (a motion that ends in its start pose but has travelled, like
/// ground swing 3, entry 100, is not a loop) (metres).
pub fn cycle_error(conv: &Converter, m: &kh1_formats::mset::Motion) -> f64 {
    let mut rig = conv.rig();
    rig.start_motion(m);
    let a = conv.targets(&mut rig, m, 0.0, false);
    let b = conv.targets(&mut rig, m, m.frames as f32, false);
    (1..a.len()).map(|i| dist(a[i].p, b[i].p)).fold(0.0, f64::max)
}

impl Built {
    /// Entries played by Re:CoM's own clips.
    pub fn recom_entries(&self) -> usize {
        self.entries.iter().filter(|e| e.source == Source::Recom).count()
    }

    /// New slots (KH1 entries Re:CoM doesn't have).
    pub fn new_slots(&self) -> usize {
        self.entries.iter().filter(|e| e.source == Source::Kh1).count()
    }

    /// The manifest (`kh1-motions.json`): one object per KH1 entry in `slots` (the runtime looks
    /// motions up by KH1 entry through it).
    pub fn manifest_json(&self, sources: &[(&str, String)]) -> String {
        let mut s = String::new();
        s.push_str("{\n  \"format\": \"kh1-recom-assets motions 2\",\n");
        s.push_str("  \"pack\": \"PLAYER/0001/PL0001.ANN\",\n");
        s.push_str(&format!(
            "  \"base_slots\": {},\n  \"slot_count\": {},\n  \"pack_changed\": {},\n  \"fps\": 60,\n  \"kh1_entries\": {},\n  \"recom_entries\": {},\n  \"new_slots\": {},\n",
            self.base_slots,
            self.ann.slots.len(),
            self.changed,
            self.entries.len(),
            self.recom_entries(),
            self.new_slots()
        ));
        if let Some(w) = &self.weapon {
            s.push_str(&format!("  \"weapon_pack\": \"PLAYER/0002/PL0002.ANN\",\n  \"weapon_base_slots\": {},\n  \"weapon_slot_count\": {},\n", w.base_slots, w.ann.slots.len()));
        }
        s.push_str("  \"sources\": {");
        for (i, (k, v)) in sources.iter().enumerate() {
            s.push_str(&format!("{}\n    \"{}\": \"{}\"", if i > 0 { "," } else { "" }, esc(k), esc(v)));
        }
        s.push_str("\n  },\n  \"slots\": [\n");
        let num = |v: Option<f64>, p: usize| v.map_or("null".to_string(), |x| format!("{x:.p$}"));
        for (i, e) in self.entries.iter().enumerate() {
            let req: Vec<String> = e.requests.iter().map(|r| r.to_string()).collect();
            let edited: Vec<String> = e.edited_bones.iter().map(|b| format!("\"{}\"", esc(b))).collect();
            let amb: Vec<String> = e.ambiguous.iter().map(|x| x.to_string()).collect();
            s.push_str(&format!(
                "    {{\"kh1_entry\": {}, \"kh1_request\": {}, \"requests\": [{}], \"slot\": {}, \"name\": \"{}\", \"source\": \"{}\", \"match_kind\": {}, \"match_error\": {}, \"edited_bones\": [{}], \"root_split_mm\": {}, \"ambiguous\": [{}], \"kh1_frames\": {}, \"frames\": {}, \"fps\": {}, \"loop\": {}, \"loop_overridden\": {}, \"cycle_error_m\": {:.6}, \"root_motion\": {}, \"bytes\": {}, \"keys\": {}, \"weapon_name\": {}, \"weapon_match_error\": {}}}{}\n",
                e.kh1_entry,
                e.kh1_request.map_or(-1, |r| r as i64),
                req.join(", "),
                e.slot,
                esc(&e.name),
                e.source.as_str(),
                e.match_kind.map_or("null".to_string(), |k| format!("\"{}\"", k.as_str())),
                num(e.match_error, 4),
                edited.join(", "),
                num(e.root_split_mm, 1),
                amb.join(", "),
                e.kh1_frames,
                e.frames,
                e.fps,
                e.looped,
                e.loop_overridden,
                e.cycle_error_m,
                e.root_motion,
                e.bytes,
                e.keys,
                e.weapon_name.as_ref().map_or("null".to_string(), |w| format!("\"{}\"", esc(w))),
                num(e.weapon_match_error, 4),
                if i + 1 < self.entries.len() { "," } else { "" }
            ));
        }
        s.push_str("  ]\n}\n");
        s
    }
}

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}
