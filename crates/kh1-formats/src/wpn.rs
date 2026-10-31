//! KH1 weapon files (`xw_ex_5010.wpn` …): the container, its `MENV` model section, and the
//! names of the PC's HD assets that belong to the model.
//!
//! Layout, Checked (files: the 17 Keyblades KH1 has Re:CoM cards for, plus Jungle King 5020):
//! - `u32 2 | u32 0x80 | u32 MENV offset | u32` (the 4th word is not the file size), zeros to
//!   0x80. Section 0 at 0x80 is the weapon data (records, trail texture; OpenKH `OpenKh.Kh1/Wpn.cs`
//!   "MainFile"), not read here.
//! - Section 1 starts with `"MENV"` and has the `MOBJ` header layout: `+8` texture info offset /
//!   size, `+0x10` texture data, `+0x18` CLUT, `+0x20` model offset / size (from the MENV
//!   start; the model is at `+0x40` in all 18 files). [`crate::mobj::Model::parse`] reads it
//!   (no skeleton: the joint and pair tables are `0xFFFFFFFF`).
//! - HD assets (PC): the model's HD mesh is `-<model file offset, hex>.cvbl`, and the texture of
//!   cvbl slot `s` is `-<model file offset + 0x100 + s, hex>.dds` (DXT5). Checked (files, 18 of 18;
//!   the rule itself is read off the names: Assumption for files not checked).

use crate::mobj::Model;
use crate::reader::Bytes;
use crate::{bad, Result};

/// A parsed `.wpn` (borrowing the file).
#[derive(Clone, Copy, Debug)]
pub struct Wpn<'a> {
    file: &'a [u8],
    /// Offset of the `MENV` section in the file.
    pub menv: usize,
    /// Offset of the model in the file (`MENV + model offset`).
    pub model_offset: usize,
    /// Size of the model in bytes.
    pub model_size: usize,
}

impl<'a> Wpn<'a> {
    /// Parses the container and the `MENV` header.
    pub fn parse(file: &'a [u8]) -> Result<Wpn<'a>> {
        let b = Bytes(file);
        if b.u32(0)? != 2 || b.u32(4)? != 0x80 {
            return bad("wpn: not a 2-section weapon file");
        }
        let menv = b.u32(8)? as usize;
        if b.slice(menv, 4)? != b"MENV" {
            return bad(format!("wpn: no MENV at {menv:#x}"));
        }
        let mo = b.u32(menv + 0x20)? as usize;
        let ms = b.u32(menv + 0x24)? as usize;
        let model_offset = menv.checked_add(mo).ok_or_else(|| crate::Error::Format("wpn: model offset".into()))?;
        b.slice(model_offset, ms)?;
        Ok(Wpn { file, menv, model_offset, model_size: ms })
    }

    /// The model bytes.
    pub fn model_bytes(&self) -> &'a [u8] {
        &self.file[self.model_offset..self.model_offset + self.model_size]
    }

    /// The PS2 model (meshes and packets).
    pub fn model(&self) -> Result<Model> {
        Model::parse(self.model_bytes(), false)
    }

    /// The HD asset name of the model's `.cvbl` mesh, e.g. `-9a40.cvbl`.
    pub fn cvbl_name(&self) -> String {
        format!("-{:x}.cvbl", self.model_offset)
    }

    /// The HD asset name of the texture in cvbl slot `slot`, e.g. `-9b40.dds`.
    pub fn texture_name(&self, slot: u32) -> String {
        format!("-{:x}.dds", self.model_offset + 0x100 + slot as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_other_files() {
        assert!(Wpn::parse(&[0u8; 0x100]).is_err());
        let mut f = vec![0u8; 0x200];
        f[0] = 2;
        f[4] = 0x80;
        f[8] = 0x80;
        assert!(Wpn::parse(&f).is_err());
        f[0x80..0x84].copy_from_slice(b"MENV");
        f[0xA0] = 0x40;
        f[0xA4] = 0x10;
        let w = Wpn::parse(&f).unwrap();
        assert_eq!(w.model_offset, 0xC0);
        assert_eq!(w.cvbl_name(), "-c0.cvbl");
        assert_eq!(w.texture_name(1), "-1c1.dds");
    }

    /// Every `.wpn` in `KH1_TEST_DATA/keyblades/` has its model's `.cvbl` among its HD assets
    /// (`<file>.assets.txt`, as the extractor writes it).
    #[test]
    fn hd_asset_names() {
        let Some(dir) = crate::testdata::dir().map(|d| d.join("keyblades")).filter(|d| d.is_dir()) else { return };
        let mut n = 0;
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if !name.ends_with(".wpn") {
                continue;
            }
            let f = std::fs::read(e.path()).unwrap();
            let w = Wpn::parse(&f).unwrap();
            let list = std::fs::read_to_string(dir.join(format!("{name}.assets.txt"))).unwrap();
            assert!(list.lines().any(|l| l.split('\t').next() == Some(&w.cvbl_name())), "{name}: {}", w.cvbl_name());
            assert!(list.lines().any(|l| l.split('\t').next() == Some(&w.texture_name(0))), "{name}");
            assert!(w.model().is_ok(), "{name}");
            n += 1;
        }
        assert!(n >= 17, "{n} .wpn files");
    }
}
