//! # kh2-formats-lite
//!
//! Readers for the few KINGDOM HEARTS II FINAL MIX (PC) files the asset step needs to bring
//! KH2's Keyblades into Re:Chain of Memories (decision 25, `docs/recom-keyblade-models.md` § 12):
//!
//! - [`bar`]: BAR containers (most KH2 files);
//! - [`mdlx`]: MDLX models: the mesh (bone-local vertices, UVs, strip windings, groups) and the
//!   PS2 textures;
//! - [`skeleton`]: an MDLX's bones and their bind pose;
//! - [`system`]: `03system.bin` `item` (items and the equipment stat rows);
//! - [`objentry`]: `00objentry.bin` (object → model name);
//! - [`msg`]: `msg/us/sys.bar` (item names).
//!
//! The PC archives themselves (`kh2_*.hed` / `.pkg`, same format and cipher as KH1's and
//! Re:CoM's) are read with `kh1-formats`' `egs`.
//!
//! **Origin and licence.** Ported from the owner's other project, KH2-Combat-in-KH1
//! (`crates/kh2-formats`: `bar.rs`, `reader.rs`, `mdlx.rs`, `anim.rs`'s skeleton, `system.rs`,
//! `objentry.rs`, `msg.rs`), Apache-2.0. Those files' layouts follow the OpenKH project's
//! documentation and C# readers (<https://github.com/OpenKH/OpenKh>, Apache-2.0, Copyright
//! 2019-2022 OpenKH Team and Contributors; `OpenKh.Kh2/Bar.cs`, `Objentry.cs`,
//! `SystemData/Item.cs`, `docs/kh2/file/type/mdlx.md`). The PS2 GS memory layout used to read
//! the textures is the hardware's (as documented by the PS2 emulation community). Changes in
//! this port: `thiserror` dropped, the skeleton's bind pose computed here (no `glam`),
//! `system` reduced to items. No game data is embedded here.

#![forbid(unsafe_code)]

pub mod bar;
pub mod mdlx;
pub mod msg;
pub mod objentry;
mod reader;
pub mod skeleton;
pub mod system;

/// Errors from the readers.
#[derive(Debug)]
pub enum Error {
    /// The data does not have the expected layout.
    Format(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Format(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for Error {}

/// Result alias.
pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn bad<T>(msg: impl Into<String>) -> Result<T> {
    Err(Error::Format(msg.into()))
}
