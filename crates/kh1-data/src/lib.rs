//! # kh1-data
//!
//! The bridge between the player's own KINGDOM HEARTS FINAL MIX (PC, Steam) install and the
//! engine-agnostic crates of the KH1 rewrite (`docs/kh1-rewrite.md`):
//!
//! - [`GameDir`] finds the KH 1.5+2.5 folder (it holds `Image/dt/kh1_first.hed` and
//!   `KINGDOM HEARTS FINAL MIX.exe`);
//! - [`SoraKit`] reads `xa_ex_0010.mset`, `xa_ex_0010.mdls` and `btltbl.bin` from it and builds
//!   `kh1-core`'s [`Params`](kh1_core::Params), [`Moveset`](kh1_core::Moveset),
//!   [`WeaponData`](kh1_core::WeaponData), [`DamageCurve`](kh1_core::DamageCurve) and
//!   [`CombatParams`](kh1_core::CombatParams), plus the skeleton ([`kh1_anim::Rig`]) and the
//!   motion set `kh1-anim` poses.
//!
//! Nothing from the games is stored here: every number comes from the player's files at runtime.
//! Tests that need game files read them from `KH1_TEST_DATA` and skip when it is unset.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod game_dir;
mod sora;

pub use game_dir::{GameDir, NotFound, GAME_DIR_ARG, GAME_DIR_ENV, OWNER_STEAM_DIR};
pub use kh1_formats::{Error, Result};
pub use sora::{attack_from_file, battle_entry, build_moveset, fake_install, flag_windows, root_curve, row_from_file, SoraKit, BTLTBL, MDLS, MSET};

/// Test helpers: the game-file directory from `KH1_TEST_DATA` (tests skip when it is unset).
#[cfg(test)]
pub(crate) mod testdata {
    use std::path::PathBuf;

    /// The test-data directory, or `None` (the test then passes without checking).
    pub fn dir() -> Option<PathBuf> {
        let p = PathBuf::from(std::env::var_os("KH1_TEST_DATA")?);
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
