//! # kh1-formats
//!
//! Readers for KINGDOM HEARTS FINAL MIX (PC, Steam) files, used by the 1:1 Rust rewrite of
//! KH1's battle system (see `docs/kh1-rewrite.md`).
//!
//! - [`egs`]: the `kh1_*.hed` / `kh1_*.pkg` archive pairs, and [`egs::Install`] over all five
//! - [`mset`]: Sora's motion set (`xa_ex_0010.mset`): request map, motions, curves, flag tracks
//! - [`mdls`]: Sora's skeleton (`xa_ex_0010.mdls`): bones, parents, rest transforms
//! - [`btltbl`]: `btltbl.bin`: action rows, attack records, damage curve, globals, weapons
//!
//! No game data is embedded here. The one piece of the archive cipher that comes from the game
//! (a 256-byte substitution table) is read from the user's own executable at runtime; this crate
//! only stores its SHA-256 to recognise it. The cipher is a port of OpenKH's `OpenKh.Egs`
//! (Apache-2.0, Copyright OpenKH Team and Contributors).
//!
//! Units are KH1's: about 1 cm, 60 Hz frames, radians.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod btltbl;
pub mod egs;
pub mod mdls;
pub mod mset;
mod reader;

/// Errors from the readers.
#[derive(Debug)]
pub enum Error {
    /// An I/O error while reading a file.
    Io(std::io::Error),
    /// The data does not have the expected layout.
    Format(String),
    /// The named file is in none of the archives.
    NotFound(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "i/o error: {e}"),
            Error::Format(s) => f.write_str(s),
            Error::NotFound(s) => write!(f, "file not found in archive: {s}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn bad<T>(msg: impl Into<String>) -> Result<T> {
    Err(Error::Format(msg.into()))
}

/// Test helpers: the game-file directory from `KH1_TEST_DATA` (tests skip when it is unset).
#[cfg(test)]
pub(crate) mod testdata {
    use std::path::PathBuf;

    /// The test-data directory, or `None` (the test then passes without checking).
    pub fn dir() -> Option<PathBuf> {
        let d = std::env::var_os("KH1_TEST_DATA")?;
        let p = PathBuf::from(d);
        p.is_dir().then_some(p)
    }

    /// Reads `name` from the test-data directory, if present.
    pub fn read(name: &str) -> Option<Vec<u8>> {
        let p = dir()?.join(name);
        match std::fs::read(&p) {
            Ok(b) => Some(b),
            Err(_) => {
                eprintln!("skipping: {} not found", p.display());
                None
            }
        }
    }
}
