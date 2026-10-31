//! # kh1-recom-assets
//!
//! The asset build step (roadmap M2): converts KH1 FM Sora's motions (the player's own
//! `xa_ex_0010.mset` / `.mdls`) into Re:Chain of Memories' own motion pack, so Re:CoM's engine
//! plays them natively. Re:CoM's 161 slots of `PLAYER/0001/PL0001.ANN` stay byte-identical;
//! one 60 fps clip per KH1 motion is appended after them (`p001_161` …), and a manifest says
//! which slot is which KH1 entry. Notes: `docs/kh1-recom-assets.md`.
//!
//! - [`ann`]: the `.ANN` pack and clip format (reader, writer, sampler);
//! - [`mdl`]: Re:CoM's `.MDL` (skeleton for the motions; the whole file read and written);
//! - [`rtm`]: Re:CoM's texture packs (`.RTM`) and their 8-bit TIM2s;
//! - [`texture`]: DDS decoding, downscale and 256-colour quantising;
//! - [`keyblade`]: KH1 Keyblades (`.wpn` + HD `.cvbl` / `.dds`) and KH2 ones (`.mdlx` + HD `.dds`)
//!   → Re:CoM weapon models;
//! - [`kh2_keyblades`]: the six KH2 Keyblades' items, stats, models and reach (decision 25);
//! - [`convert`]: KH1 pose (via `kh1-anim`) → Re:CoM bones → clip;
//! - [`build`]: the whole pack + manifest from the four input files;
//! - [`install`]: finding the files in the player's install and the Panacea mod folder;
//! - [`ses`]: Re:CoM's per-motion sound events for the KH1-only clips (`PL0001.SES`);
//! - [`stats`]: KH1 enemy twins' records and Sora's KH1 Strength (`kh1-stats.json`);
//! - [`flip`]: the "Re:CoM's content in KH1" model-writer test (`kh1-flip-test`).
//!
//! No game data is embedded; everything is read from the user's files at run time.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod ann;
pub mod build;
pub mod convert;
pub mod flip;
pub mod install;
pub mod keyblade;
pub mod kh2_keyblades;
pub mod matching;
pub mod math;
pub mod mdl;
pub mod rtm;
pub mod ses;
pub mod stats;
pub mod texture;

/// Errors.
#[derive(Debug)]
pub enum Error {
    /// I/O.
    Io(std::io::Error),
    /// Bad data or arguments.
    Format(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "i/o error: {e}"),
            Error::Format(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<kh1_formats::Error> for Error {
    fn from(e: kh1_formats::Error) -> Self {
        match e {
            kh1_formats::Error::Io(e) => Error::Io(e),
            other => Error::Format(other.to_string()),
        }
    }
}

/// Result alias.
pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn bad<T>(msg: impl Into<String>) -> Result<T> {
    Err(Error::Format(msg.into()))
}
