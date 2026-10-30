//! KH1 `.mdls` model files: the skeleton only (Sora's `xa_ex_0010.mdls` has 293 bones).
//! Meshes and textures are not read.
//!
//! Layout (Checked (files); the same layout as the KH2 project's KH1 reader):
//! - file +0x00 u32 section count, then u32 section offsets; section 0 is `"MOBJ"`.
//! - `MOBJ` +0x20 u32: offset (from `MOBJ`) of the skeleton header.
//! - skeleton header +0x00 u32 bone count, +0x04 u32 offset (from the header) of the bone table.
//! - bone, 0x30 bytes: `scale xyz f32 | index u32 | rot xyz f32 (radians) | f32 (unread) |
//!   trans xyz f32 | links u32` where links = parent (bits 0-9, 0x3FF = none), first sibling
//!   (10-19), first child (20-29), 2 flag bits (30-31).
//!
//! Local matrix = S · Rx · Ry · Rz · T with row vectors (proven by the exact rest-pose match
//! with Re:CoM's copy of the skeleton, `motion-conversion.md` §4a). Units are about 1 cm. The
//! file holds no bone names.

use crate::reader::Bytes;
use crate::{bad, Result};

/// Size of one bone record.
pub const BONE_SIZE: usize = 0x30;
/// The "no bone" link value.
pub const NO_LINK: u16 = 0x3FF;
/// Sora's bone count in `xa_ex_0010.mdls`.
pub const SORA_BONES: usize = 293;

/// One skeleton bone: rest local transform and links.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bone {
    /// The record's own index field (equals its position in Sora's file).
    pub index: u32,
    /// Parent bone, or `None` for a root.
    pub parent: Option<u16>,
    /// First sibling link (raw; [`NO_LINK`] = none).
    pub sibling: u16,
    /// First child link (raw; [`NO_LINK`] = none).
    pub child: u16,
    /// Links bits 30-31 (meaning not traced).
    pub link_flags: u8,
    /// Rest local scale.
    pub scale: [f32; 3],
    /// Rest local rotation, Euler radians, applied X then Y then Z (row vectors).
    pub rot: [f32; 3],
    /// The f32 after the rotation (+0x1C). Not read by the posing code seen; kept raw.
    pub rot_extra: f32,
    /// Rest local translation (about 1 cm).
    pub trans: [f32; 3],
}

/// A parsed skeleton.
#[derive(Clone, Debug, PartialEq)]
pub struct Skeleton {
    /// The bones, in file order (parents always come first in Sora's file).
    pub bones: Vec<Bone>,
}

impl Skeleton {
    /// Parses the skeleton out of a whole `.mdls` file.
    pub fn parse(file: &[u8]) -> Result<Skeleton> {
        let b = Bytes(file);
        let mobj = b.u32(4)? as usize;
        if b.slice(mobj, 4)? != b"MOBJ" {
            return bad("mdls: section 0 is not MOBJ");
        }
        let hdr = b.rel(mobj, b.u32(mobj + 0x20)?)?;
        let count = b.u32(hdr)? as usize;
        if count == 0 || count > NO_LINK as usize {
            return bad(format!("mdls: implausible bone count {count}"));
        }
        let table = b.rel(hdr, b.u32(hdr + 4)?)?;
        let bones = parse_bones(file, table, count)?;
        Ok(Skeleton { bones })
    }

    /// Number of bones.
    pub fn len(&self) -> usize {
        self.bones.len()
    }

    /// True when there are no bones.
    pub fn is_empty(&self) -> bool {
        self.bones.is_empty()
    }

    /// Parent index per bone (-1 for a root).
    pub fn parents(&self) -> Vec<i16> {
        self.bones.iter().map(|b| b.parent.map_or(-1, |p| p as i16)).collect()
    }
}

/// Parses `count` 0x30-byte bone records starting at `at` (also usable on a bone table copied
/// out of game memory).
pub fn parse_bones(data: &[u8], at: usize, count: usize) -> Result<Vec<Bone>> {
    let b = Bytes(data);
    let mut bones = Vec::with_capacity(count);
    for i in 0..count {
        let a = at + i * BONE_SIZE;
        let links = b.u32(a + 0x2C)?;
        let parent = (links & 0x3FF) as u16;
        let bone = Bone {
            index: b.u32(a + 0x0C)?,
            parent: (parent != NO_LINK).then_some(parent),
            sibling: ((links >> 10) & 0x3FF) as u16,
            child: ((links >> 20) & 0x3FF) as u16,
            link_flags: (links >> 30) as u8,
            scale: b.f32x3(a)?,
            rot: b.f32x3(a + 0x10)?,
            rot_extra: b.f32(a + 0x1C)?,
            trans: b.f32x3(a + 0x20)?,
        };
        if let Some(p) = bone.parent {
            if p as usize >= count {
                return bad(format!("mdls: bone {i} has parent {p} of {count}"));
            }
        }
        bones.push(bone);
    }
    Ok(bones)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testdata;

    #[test]
    fn sora_skeleton() {
        let Some(file) = testdata::read("xa_ex_0010.mdls") else { return };
        let s = Skeleton::parse(&file).unwrap();
        assert_eq!(s.len(), SORA_BONES);
        let b0 = s.bones[0];
        assert_eq!(b0.parent, None);
        assert!((b0.trans[1] - 92.664).abs() < 1e-3 && (b0.trans[2] - 1.178).abs() < 1e-3);
        assert!((b0.rot[1] + std::f32::consts::FRAC_PI_2).abs() < 1e-4);
        assert_eq!(s.bones[1].parent, Some(0));
        assert_eq!(s.bones[4].parent, Some(3));
        assert!((s.bones[3].trans[0] - 13.852).abs() < 1e-3);
        for (i, b) in s.bones.iter().enumerate() {
            assert_eq!(b.index as usize, i);
            if let Some(p) = b.parent {
                assert!((p as usize) < i, "bone {i} parent {p} comes later");
            }
        }
        assert_eq!(s.parents()[0], -1);
    }

    #[test]
    fn rejects_garbage() {
        assert!(Skeleton::parse(&[0u8; 64]).is_err());
        assert!(Skeleton::parse(&[]).is_err());
    }
}
