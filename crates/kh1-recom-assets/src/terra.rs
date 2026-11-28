//! BBS FM's Terra, read from the player's own 1.5+2.5 install, as the source of Riku's cast and Block motions
//! (0.189.0; fork owner, 2026-11-24, verbatim: "o block do Riku poderia usar a animação do block do Terra.. sobre a
//! magia poderia usar as animações do Terra também"). BBS FM is in the 1.5+2.5 collection, so this needs no other game.
//!
//! Formats (the workspace's `knowledge/BBS/bbs-terra-casts-guard-for-riku.md`, Checked (files) on the owner's install
//! against `OpenKh@41ac7b5` `OpenKh.Bbs/Arc.cs`, `Pam.cs`, `Pmo.cs`):
//! - the archives `Image/dt/bbs_first..fourth.hed / .pkg` are the 1.5+2.5 layout and cipher (`kh1_formats::egs`, the
//!   keys the step already has), the same as KH1's;
//! - **ARC** (`"ARC\0"`, `u16` version, `u16` entry count, 8 bytes unused; entries of 0x20 bytes from 0x10: `u32`
//!   directory pointer (non-zero = a link, no data), `u32` offset, `u32` length, `u32` unused, `char[16]` name);
//! - **PAM** version 1 ([`crate::pam::parse_pam`] reads both versions): `u32` offset + `char[12]` name inline per entry;
//! - the **PMO** v1 skeleton (`p03ex00.pmo`, 51 bones, the same `BON\0` block of 160-byte bones as other PMOs':
//!   [`crate::pam::parse_skeleton`]).
//!
//! Nothing of the game is in this file: the step reads the player's files.

use crate::pam::PamClip;
use crate::mdl::RecomSkeleton;
use crate::{bad, Result};
use std::path::Path;

/// BBS's archives.
pub const ARCHIVES: [&str; 4] = ["bbs_first", "bbs_second", "bbs_third", "bbs_fourth"];
/// Terra's model (his skeleton).
pub const MODEL: &str = "chara/pc/p03ex/00/p03ex00.pmo";
/// The packs the mod reads: the ground / air thrust cast (`315`, `316`: Fire and Blizzard), the ground / air self cast
/// (`317`, `318`: Thunder, Cure, Gravity, Magnet, Aero, Stop), and the always-loaded base pack inside `p03base.arc`
/// (`p03ex_001.pam`: Guard's `354`, PAT_TERR slot 63). Checked (decomp + files; the notes' sections 7 and 8).
pub const PACKS: [&str; 3] = ["arc/pc/p03ex_151.arc", "arc/pc/p03ex_152.arc", "arc/pc/p03base.arc"];
/// Terra's Guard (command kind 265) clip: `FUN_14025e8f0` plays PAT_TERR slot 63 once, with no separate hold, release or
/// hit clip found. Checked (decomp, files). What the pose does after its last frame: Assumption.
pub const GUARD: &str = "354";
/// Riku's hip height over Terra's (0.974 m / 1.154 m, Checked (files)): Terra's body travel scaled onto Riku's.
pub const TRAVEL_SCALE: f64 = 0.844;

/// Reads `name` from whichever BBS archive holds it.
pub fn read(game: &Path, keys: &kh1_formats::egs::KeyTable, name: &str) -> Result<Vec<u8>> {
    for a in ARCHIVES {
        let hed = game.join("Image").join("dt").join(format!("{a}.hed"));
        if !hed.is_file() {
            continue;
        }
        let arc = kh1_formats::egs::Archive::open(&hed)?;
        if arc.find(name).is_some() {
            return Ok(arc.read(keys, name)?);
        }
    }
    bad(format!("BBS: {name} in none of the bbs archives"))
}

/// The data of the first non-link entry of an ARC whose name ends in `.pam` (`p03ex_151.arc` holds `p03ex_151.pam`,
/// `p03base.arc` holds `p03ex_001.pam` among other files).
pub fn arc_pam(b: &[u8]) -> Result<&[u8]> {
    if b.get(..4) != Some(b"ARC\0") {
        return bad("ARC: no magic");
    }
    let u32_at = |o: usize| -> Result<usize> {
        Ok(u32::from_le_bytes(b.get(o..o + 4).ok_or_else(|| crate::Error::Format("ARC: outside the file".into()))?.try_into().unwrap()) as usize)
    };
    let count = u16::from_le_bytes([*b.get(6).ok_or_else(|| crate::Error::Format("ARC: short".into()))?, *b.get(7).ok_or_else(|| crate::Error::Format("ARC: short".into()))?]) as usize;
    for i in 0..count {
        let e = 0x10 + i * 0x20;
        let name = b.get(e + 16..e + 32).ok_or_else(|| crate::Error::Format("ARC: entry outside".into()))?;
        let name = &name[..name.iter().position(|&c| c == 0).unwrap_or(16)];
        if u32_at(e)? != 0 || !name.ends_with(b".pam") {
            continue;
        }
        let (off, len) = (u32_at(e + 4)?, u32_at(e + 8)?);
        return b.get(off..off + len).ok_or_else(|| crate::Error::Format("ARC: data outside".into()));
    }
    bad("ARC: no .pam entry")
}

/// Terra's skeleton and the clips of [`PACKS`] (`315`..`318`, `354`, among the base pack's others).
pub fn read_terra(game: &Path, keys: &kh1_formats::egs::KeyTable) -> Result<(RecomSkeleton, Vec<PamClip>)> {
    let sk = crate::pam::parse_skeleton(&read(game, keys, MODEL)?)?;
    let mut clips = Vec::new();
    for p in PACKS {
        let arc = read(game, keys, p)?;
        clips.extend(crate::pam::parse_pam(arc_pam(&arc)?)?);
    }
    for n in ["315", "316", "317", "318", GUARD] {
        let Some(c) = clips.iter().find(|c| c.name == n) else { return bad(format!("BBS Terra: no clip {n}")) };
        if c.bones.len() != sk.bones.len() {
            return bad(format!("BBS Terra: clip {n} has {} bones, his skeleton {}", c.bones.len(), sk.bones.len()));
        }
    }
    Ok((sk, clips))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::install;

    /// A PAM v1 pack with one empty slot and one clip `name` of 2 frames and 1 bone: tx constant 0.5, ry keyed (frame 0 -> 0,
    /// frame 1 -> max).
    fn pam_v1(name: &str) -> Vec<u8> {
        let mut b = vec![0u8; 0x10];
        b[..4].copy_from_slice(b"PAM\0");
        b[4..8].copy_from_slice(&2u32.to_le_bytes());
        b[0xE] = 1;
        b.extend([0u8; 16]); // slot 0: empty
        let mut e = [0u8; 16];
        e[..4].copy_from_slice(&0x40u32.to_le_bytes());
        e[4..4 + name.len()].copy_from_slice(name.as_bytes());
        b.extend(e);
        b.resize(0x40, 0);
        // header: flag u16, fps u8, blend u8, loop u16, bones u8, pad u8, frames u16, return u16
        b.extend([0, 1, 30, 0, 0, 0, 1, 0, 2, 0, 0, 0]);
        b.extend((1u16 | 1 << 4).to_le_bytes()); // channel mask: tx, ry
        b.extend(0.5f32.to_le_bytes());
        b.extend(0.5f32.to_le_bytes());
        b.push(1); // tx: one key = a constant (max)
        b.extend(1.0f32.to_le_bytes());
        b.extend(0.0f32.to_le_bytes());
        b.push(2); // ry: key count = frames, so no frame ids
        b.extend(0u16.to_le_bytes());
        b.extend(65535u16.to_le_bytes());
        b
    }

    #[test]
    fn reads_a_pam_v1_pack() {
        let clips = crate::pam::parse_pam(&pam_v1("315")).unwrap();
        assert_eq!(clips.len(), 1);
        let c = &clips[0];
        assert_eq!((c.name.as_str(), c.fps, c.frames, c.bones.len()), ("315", 30, 2, 1));
        let (t, r, s) = c.local(0, 0.0);
        assert_eq!((t, r, s), ([0.5, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]));
        assert!((c.local(0, 1.0).1[1] - 1.0).abs() < 1e-9);
        assert!((c.local(0, 0.5).1[1] - 0.5).abs() < 1e-4);
    }

    /// An ARC with a link entry, a non-pam file and the pack: the pack's data is found.
    #[test]
    fn finds_the_pam_in_an_arc() {
        let pam = pam_v1("354");
        let mut b = vec![0u8; 0x10];
        b[..4].copy_from_slice(b"ARC\0");
        b[4] = 1;
        b[6] = 3;
        let data_at = 0x10 + 3 * 0x20;
        let entry = |link: u32, off: usize, len: usize, name: &str| {
            let mut e = [0u8; 0x20];
            e[..4].copy_from_slice(&link.to_le_bytes());
            e[4..8].copy_from_slice(&(off as u32).to_le_bytes());
            e[8..12].copy_from_slice(&(len as u32).to_le_bytes());
            e[16..16 + name.len()].copy_from_slice(name.as_bytes());
            e
        };
        b.extend(entry(0x1234, 0, 0, "linked.pam"));
        b.extend(entry(0, data_at, 4, "other.bin"));
        b.extend(entry(0, data_at + 4, pam.len(), "p03ex_001.pam"));
        b.extend(b"\x01\x02\x03\x04");
        b.extend(&pam);
        assert_eq!(arc_pam(&b).unwrap(), &pam[..]);
        assert!(arc_pam(b"nope").is_err());
        assert!(crate::pam::parse_pam(&pam[..0x30]).is_err(), "a cut pack is an error, not a panic");
    }

    /// The owner's install (`RECOM_GAME`): Terra's skeleton and clips read, with the frames the workspace's note lists.
    #[test]
    fn reads_terras_clips() {
        let Some(g) = std::env::var_os("RECOM_GAME").map(std::path::PathBuf::from) else { return };
        let keys = install::keys(&g).unwrap();
        let (sk, clips) = read_terra(&g, &keys).unwrap();
        assert_eq!(sk.bones.len(), 51);
        let frames = |n: &str| clips.iter().find(|c| c.name == n).unwrap().frames;
        assert_eq!([frames("315"), frames("316"), frames("317"), frames("318"), frames(GUARD)], [36, 36, 37, 37, 36]);
        assert!(clips.iter().all(|c| c.fps == 30));
        for n in ["Root", "center", "koshi", "hara", "mune", "R_kata", "R_te", "R_buki"] {
            assert!(sk.find(n).is_some(), "bone {n}");
        }
    }
}
