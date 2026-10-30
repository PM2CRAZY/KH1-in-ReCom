//! The "Re:CoM's content in KH1" feasibility test, step 1 (`docs/kh1-model-writer.md`): can we
//! write KH1 models that KH1 loads and draws natively?
//!
//! - [`roundtrip`]: every `xa_*.mdls` in the player's KH1 archives, and every `.cvbl` HD mesh
//!   in them, is parsed and written again by `kh1-formats` ([`MdlsFile`], [`Cvbl`]) and must
//!   come back byte for byte. The rebuilt PKG entry ([`build_plain_entry`]) must decode back to
//!   the same file and assets.
//! - [`build_test_entry`]: a model's PKG entry rebuilt by our writers, for Panacea to serve
//!   from `<mod_path>/kh1/raw/<file>` (`Panacea.cpp` `GetRawFile` / `LoadFile` /
//!   `GetRemasteredAsset`; Checked (source)). [`Mode::Rebuild`] keeps the content;
//!   [`Mode::Scale`] scales every vertex position (PS2 models and HD meshes) so the result is
//!   visibly ours.
//!
//! Nothing here runs in the game: these are files Panacea hands to KH1's own loader.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use kh1_formats::cvbl::Cvbl;
use kh1_formats::egs::{build_plain_entry, decode_entry_full, name_hash, HdAsset, Install, KeyTable};
use kh1_formats::mobj::MdlsFile;

use crate::{bad, Result};

/// The Shadow's model, the test's default (KH1's most common enemy; Re:CoM's first enemy).
pub const SHADOW: &str = "xa_ex_2020.mdls";

/// Every `xa_<2 letters>_<4 digits>.mdls` name in the install (found by hashing every
/// candidate against the archives' indexes; KH1 PC has no name list).
pub fn mdls_names(inst: &Install) -> Vec<String> {
    let have: HashSet<[u8; 16]> = inst.archives.iter().flat_map(|a| a.entries.iter().map(|e| e.md5)).collect();
    let mut out = Vec::new();
    for a in b'a'..=b'z' {
        for b in b'a'..=b'z' {
            for n in 0..10000 {
                let name = format!("xa_{}{}_{n:04}.mdls", a as char, b as char);
                if have.contains(&name_hash(&name)) {
                    out.push(name);
                }
            }
        }
    }
    out
}

/// Result of [`roundtrip`].
#[derive(Debug, Default)]
pub struct RoundTrip {
    /// `.mdls` files checked.
    pub mdls: usize,
    /// `.cvbl` assets checked.
    pub cvbl: usize,
    /// Other HD assets carried through the rebuilt entry.
    pub other_assets: usize,
    /// Failures: `(file, what)`.
    pub failures: Vec<(String, String)>,
}

/// Parses and rewrites every model and HD mesh in the install; see the module notes.
pub fn roundtrip(inst: &Install, names: &[String]) -> RoundTrip {
    let mut r = RoundTrip::default();
    for name in names {
        let res = (|| -> Result<(usize, usize)> {
            let (data, assets, date) = inst.read_full(name)?;
            let f = MdlsFile::parse(&data)?;
            if f.write()? != data {
                return bad("mdls rewrite differs");
            }
            let mut cv = 0;
            for a in &assets {
                if a.name.ends_with(".cvbl") {
                    if Cvbl::parse(&a.data)?.write()? != a.data {
                        return bad(format!("{} rewrite differs", a.name));
                    }
                    cv += 1;
                }
            }
            let entry = build_plain_entry(&data, &assets, date)?;
            let (d2, a2) = decode_entry_full(&inst.keys, &entry)?;
            if d2 != data || a2 != assets {
                return bad("rebuilt PKG entry does not decode back");
            }
            Ok((cv, assets.len() - cv))
        })();
        match res {
            Ok((cv, other)) => {
                r.mdls += 1;
                r.cvbl += cv;
                r.other_assets += other;
            }
            Err(e) => r.failures.push((name.clone(), e.to_string())),
        }
    }
    r
}

/// What the test entry holds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    /// The model rebuilt by our writers, content unchanged.
    Rebuild,
    /// Every vertex position (PS2 models and HD meshes) multiplied by this factor.
    Scale(f32),
}

/// What [`build_test_entry`] changed.
#[derive(Debug, Default, PartialEq)]
pub struct TestEntry {
    /// The plain PKG entry for `<mod_path>/kh1/raw/<file>`.
    pub entry: Vec<u8>,
    /// HD meshes rewritten by our writer.
    pub cvbl_written: usize,
    /// PS2 vertex positions scaled.
    pub ps2_positions: usize,
    /// HD vertex positions scaled.
    pub hd_positions: usize,
}

/// Builds the test entry for `name` from the install.
pub fn build_test_entry(inst: &Install, name: &str, mode: Mode) -> Result<TestEntry> {
    let (data, assets, date) = inst.read_full(name)?;
    let mut f = MdlsFile::parse(&data)?;
    let mut t = TestEntry::default();
    let k = match mode {
        Mode::Rebuild => 1.0,
        Mode::Scale(k) => k,
    };
    if mode != Mode::Rebuild {
        let mut n = 0;
        let mut scale4 = |p: &mut [f32; 4]| {
            p[0] *= k;
            p[1] *= k;
            p[2] *= k;
            n += 1;
        };
        f.mobj.model.for_each_position(&mut scale4);
        if let Some(s) = &mut f.mobj.second {
            s.for_each_position(&mut scale4);
        }
        t.ps2_positions = n;
    }
    let file = f.write()?;
    let mut out_assets = Vec::with_capacity(assets.len());
    for a in assets {
        if a.name.ends_with(".cvbl") {
            let mut c = Cvbl::parse(&a.data)?;
            if mode != Mode::Rebuild {
                c.for_each_position(|p| {
                    p.iter_mut().for_each(|x| *x *= k);
                    t.hd_positions += 1;
                });
            }
            t.cvbl_written += 1;
            out_assets.push(HdAsset { data: c.write()?, ..a });
        } else {
            out_assets.push(a);
        }
    }
    t.entry = build_plain_entry(&file, &out_assets, date)?;
    Ok(t)
}

/// `<mod>/kh1` (an existing folder of that name in any case is reused).
pub fn kh1_dir(modp: &Path) -> PathBuf {
    if let Ok(rd) = std::fs::read_dir(modp) {
        for e in rd.flatten() {
            if e.file_name().to_string_lossy().eq_ignore_ascii_case("kh1") && e.path().is_dir() {
                return e.path();
            }
        }
    }
    modp.join("kh1")
}

/// Where Panacea looks for a raw PKG entry of a KH1 file: `<mod>/kh1/raw/<file>`.
pub fn raw_path(modp: &Path, name: &str) -> PathBuf {
    kh1_dir(modp).join("raw").join(name)
}

/// The key table from the KH1 exe.
pub fn kh1_keys(game: &Path) -> Result<KeyTable> {
    Ok(KeyTable::from_executable(game.join(kh1_formats::egs::KH1_EXE))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_path_layout() {
        let p = raw_path(Path::new("/nonexistent/mod"), SHADOW);
        assert!(p.ends_with("kh1/raw/xa_ex_2020.mdls"));
    }
}
