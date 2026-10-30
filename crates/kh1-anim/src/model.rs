//! The `.mdls` model info block that the posing code reads besides the bone table.
//!
//! Layout (`kh1-posing.md` §1a; Checked (decomp + files)): `MOBJ` +0x20 → skeleton header;
//! header +0x08 u32 → info block (relative to the header; Sora: file 0x180).
//! - info +0x00..+0x0C: f32 foot heights (17.4335 each for Sora);
//! - info +0x10 + 4·id: the **attach table**, i32 per attach id. `FUN_1401D62E0` returns the
//!   entry and `FUN_14029FAB0` / `FUN_1401D6320` use it directly as a bone index (Checked
//!   (decomp)). Id 0x30 (the weapon, attack data +0x10) is 283 for Sora;
//! - id 4 (+0x20), else id 0 (+0x10): the look-at bone, 1-based (148 → bone 147);
//! - ids 0x20..0x23 (+0x90..+0x9C): the foot-IK bones, 1-based (44, 18 → bones 43, 17).

use kh1_formats::{Error, Result};

/// How many attach-table words are kept (Sora's table holds bone indices up to id 0x37, then
/// other data; ids past the bone range are rejected by [`ModelInfo::attach_bone`]).
const ATTACH_WORDS: usize = 0x40;

/// The model info the pose code reads.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelInfo {
    /// info +0x00..+0x0C: foot heights (the foot ground-IK probe; not used by this crate yet).
    pub foot_heights: [f32; 4],
    /// The head look-at bone (0-based). The bone pass clears its "not rebuilt" bit (25).
    pub look_at_bone: Option<usize>,
    /// The blend IK list candidates (0-based, ascending, unique): Sora 17 and 43.
    pub foot_ik_bones: Vec<usize>,
    /// Raw attach table (info +0x10, one i32 per id).
    pub attach: Vec<i32>,
}

fn u32_at(b: &[u8], o: usize) -> Result<u32> {
    b.get(o..o + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or_else(|| Error::Format(format!("mdls: read past the end at {o:#x}")))
}

impl ModelInfo {
    /// Reads the info block of a whole `.mdls` file.
    pub fn parse(file: &[u8]) -> Result<ModelInfo> {
        let mobj = u32_at(file, 4)? as usize;
        if file.get(mobj..mobj + 4) != Some(b"MOBJ".as_slice()) {
            return Err(Error::Format("mdls: section 0 is not MOBJ".into()));
        }
        let hdr = mobj + u32_at(file, mobj + 0x20)? as usize;
        let info = hdr + u32_at(file, hdr + 8)? as usize;
        let mut foot_heights = [0.0f32; 4];
        for (k, h) in foot_heights.iter_mut().enumerate() {
            *h = f32::from_bits(u32_at(file, info + 4 * k)?);
        }
        let mut attach = Vec::with_capacity(ATTACH_WORDS);
        for id in 0..ATTACH_WORDS {
            match u32_at(file, info + 0x10 + 4 * id) {
                Ok(v) => attach.push(v as i32),
                Err(_) => break,
            }
        }
        if attach.len() < 0x24 {
            return Err(Error::Format("mdls: info block too short".into()));
        }
        let one_based = |v: i32| (v > 0).then(|| v as usize - 1);
        let look_at_bone = if attach[4] != -1 { one_based(attach[4]) } else { one_based(attach[0]) };
        let mut foot_ik_bones: Vec<usize> = (0x20..0x24).filter_map(|id| one_based(attach[id])).collect();
        foot_ik_bones.sort_unstable();
        foot_ik_bones.dedup();
        Ok(ModelInfo { foot_heights, look_at_bone, foot_ik_bones, attach })
    }

    /// `FUN_14029FAB0`: the bone an attach id names. Bit 15 = not a bone (special points,
    /// handled elsewhere in the exe); bit 14 or bit 12 = the low 12 bits are the bone; otherwise
    /// the attach table. `None` when the result is not a bone of `bone_count` (the exe then uses
    /// the actor position).
    pub fn attach_bone(&self, id: u16, bone_count: usize) -> Option<usize> {
        let v: i64 = if id & 0x4000 != 0 {
            (id & 0xFFF) as i64
        } else if id & 0x8000 != 0 {
            return None;
        } else if id & 0x1000 != 0 {
            (id & 0xFFF) as i64
        } else {
            *self.attach.get(id as usize)? as i64
        };
        (v > 0 && (v as usize) < bone_count).then_some(v as usize)
    }
}
