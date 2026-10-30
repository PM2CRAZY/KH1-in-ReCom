//! The whole step: Re:CoM's `PL0001.ANN` + KH1's motions → the extended pack and its manifest.
//!
//! - The Keyblade (`PL0002.ANN`, when given): a clip at the same slot for every new Sora slot
//!   (`w001_NNN`; the weapon plays Sora's index, `FUN_1401F7180`), Re:CoM's 158 slots kept,
//!   158..160 left empty.
//! - Every KH1 entry the request map references (all 118 of Sora's) gets one new slot, in
//!   entry order, after Re:CoM's last slot: slot `base + i`, named `p001_NNN` (the name must
//!   agree with the slot for `FUN_1401A47E0`'s fast path; names past 999 still resolve by its
//!   linear search).
//! - Root: KH1 request-map bit 15 of the requests that play the entry (Sora has no entry
//!   played both with and without it; a conflict is an error).
//! - Loop: KH1 has no per-motion loop flag (the request call decides: `0x10000` = hold;
//!   `kh1-movement-motion.md` §5). A clip loops when KH1's pose at `t = N` returns to the pose
//!   at `t = 0` (every mapped bone within [`LOOP_TOLERANCE_M`], root travel included).
//!   Assumption; it agrees with all of Square Enix's own Re:CoM bakes of KH1 motions (test
//!   `validate`). [`BuildOptions::loop_overrides`] forces either way.

use kh1_formats::mset::MotionSet;

use crate::ann::{name16, Ann, Slot};
use crate::convert::{ClipSpec, Converter};
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
    /// Only these entries (tests); `None` = every referenced entry. Slots stay `base + entry
    /// rank among all referenced entries`, so a subset build has the same slot numbers.
    pub only: Option<Vec<usize>>,
    /// Require the base pack to have exactly [`RECOM_SORA_SLOTS`] slots.
    pub strict_base: bool,
}

impl Default for BuildOptions {
    fn default() -> Self {
        BuildOptions { prefix: "p001_".into(), weapon_prefix: "w001_".into(), blend: 8, loop_overrides: Vec::new(), only: None, strict_base: true }
    }
}

/// One new slot.
#[derive(Clone, Debug, PartialEq)]
pub struct ManifestEntry {
    /// Slot index in the extended pack.
    pub slot: usize,
    /// Clip name.
    pub name: String,
    /// KH1 mset entry.
    pub kh1_entry: usize,
    /// Request ids whose map word (or its type-3/4 variants) plays this entry.
    pub requests: Vec<u16>,
    /// The main request: the lowest id whose map word names this entry as its base entry, else
    /// the lowest id that reaches it as a type-3/4 variant.
    pub kh1_request: Option<u16>,
    /// KH1 length N (60 Hz frames).
    pub kh1_frames: u32,
    /// Clip frames (N + 1 play-once, N loop).
    pub frames: usize,
    /// Looping clip (loop start 1).
    pub looped: bool,
    /// Whether the loop flag came from an override.
    pub loop_overridden: bool,
    /// Pose difference t = 0 vs t = N, metres (the loop rule's measure).
    pub cycle_error_m: f64,
    /// Root travel in `Root` (request-map bit 15).
    pub root_motion: bool,
    /// Clip bytes.
    pub bytes: usize,
    /// Name of the Keyblade clip at the same slot of `PL0002.ANN`, when built.
    pub weapon_name: Option<String>,
    /// Keys stored (all animated channels).
    pub keys: usize,
}

/// The result.
#[derive(Clone, Debug)]
pub struct Built {
    /// The extended pack.
    pub ann: Ann,
    /// Its bytes.
    pub bytes: Vec<u8>,
    /// Slots of the base pack.
    pub base_slots: usize,
    /// One entry per new slot.
    pub entries: Vec<ManifestEntry>,
    /// The extended Keyblade pack, when the inputs had one.
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

/// Builds the extended pack.
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
    let mut entries = Vec::new();
    for (rank, (e, requests, root)) in refs.into_iter().enumerate() {
        let slot = base + rank;
        if opt.only.as_ref().is_some_and(|o| !o.contains(&e)) {
            continue;
        }
        let m = &ms.motions[e];
        let cycle = cycle_error(&conv, m);
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
            slot,
            name: name.clone(),
            kh1_entry: e,
            kh1_request: main_request(&ms, e, &requests),
            requests,
            kh1_frames: m.frames,
            frames: clip.frames as usize,
            looped,
            loop_overridden: ov.is_some(),
            cycle_error_m: cycle,
            root_motion: root,
            bytes,
            weapon_name,
            keys,
        });
        while ann.slots.len() < slot {
            ann.slots.push(Slot { name: [0; 16], clip: None });
        }
        ann.slots.push(Slot { name: name16(&name), clip: Some(clip) });
    }
    ann.pad_tail_like_recom();
    let bytes = ann.to_bytes();
    let weapon = weapon.map(|(mut wa, _, wbase)| {
        wa.pad_tail_like_recom();
        let bytes = wa.to_bytes();
        WeaponPack { ann: wa, bytes, base_slots: wbase }
    });
    Ok(Built { ann, bytes, base_slots: base, entries, weapon })
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
    /// The manifest (`kh1-motions.json`).
    pub fn manifest_json(&self, sources: &[(&str, String)]) -> String {
        let mut s = String::new();
        s.push_str("{\n  \"format\": \"kh1-recom-assets motions 1\",\n");
        s.push_str("  \"pack\": \"PLAYER/0001/PL0001.ANN\",\n");
        s.push_str(&format!("  \"base_slots\": {},\n  \"slot_count\": {},\n  \"fps\": 60,\n", self.base_slots, self.ann.slots.len()));
        if let Some(w) = &self.weapon {
            s.push_str(&format!("  \"weapon_pack\": \"PLAYER/0002/PL0002.ANN\",\n  \"weapon_base_slots\": {},\n  \"weapon_slot_count\": {},\n", w.base_slots, w.ann.slots.len()));
        }
        s.push_str("  \"sources\": {");
        for (i, (k, v)) in sources.iter().enumerate() {
            s.push_str(&format!("{}\n    \"{}\": \"{}\"", if i > 0 { "," } else { "" }, esc(k), esc(v)));
        }
        s.push_str("\n  },\n  \"slots\": [\n");
        for (i, e) in self.entries.iter().enumerate() {
            let req: Vec<String> = e.requests.iter().map(|r| r.to_string()).collect();
            s.push_str(&format!(
                "    {{\"slot\": {}, \"name\": \"{}\", \"kh1_entry\": {}, \"kh1_request\": {}, \"requests\": [{}], \"kh1_frames\": {}, \"frames\": {}, \"loop\": {}, \"loop_overridden\": {}, \"cycle_error_m\": {:.6}, \"root_motion\": {}, \"bytes\": {}, \"keys\": {}{}}}{}\n",
                e.slot,
                esc(&e.name),
                e.kh1_entry,
                e.kh1_request.map_or(-1, |r| r as i64),
                req.join(", "),
                e.kh1_frames,
                e.frames,
                e.looped,
                e.loop_overridden,
                e.cycle_error_m,
                e.root_motion,
                e.bytes,
                e.keys,
                e.weapon_name.as_ref().map_or(String::new(), |w| format!(", \"weapon_name\": \"{}\"", esc(w))),
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
