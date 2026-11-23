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
//! - [`step`]: the whole step (the command line's and the DLL's first-launch build);
//! - [`stamp`]: the build stamp (the step's source hash, the inputs' sizes and times, the files
//!   written);
//! - [`install`]: finding the files in the player's install and the Panacea mod folder;
//! - [`ses`]: Re:CoM's per-motion sound events for the KH1-only clips (`PL0001.SES`);
//! - [`stats`]: KH1 enemy twins' records and Sora's KH1 Strength (`kh1-stats.json`);
//! - [`mp_bar`]: the MP bar (M7 stage 7.2): KH1's gauge colours as new groups in Re:CoM's
//!   `Cockpit:00` (`FORM/0002/FO0002.SPR`) and `kh1-mp-bar.json`;
//! - [`command_menu`]: KH1's command menu (M7 stage 7.3): its words from the player's KH1
//!   `Word.bin` (`kh1-command-menu.json`) and Re:CoM's row plates as groups of `Cockpit:00`;
//! - [`menu_sounds`]: KH1's command menu sounds (its own SCDs) for Re:CoM's sound banks;
//! - [`recom_hp`]: Re:CoM's HP ring and bar turned 180° (`gauge:01`) for KH1's place;
//! - [`camp_rows`]: the camp list's green row with Re:CoM's item symbol (`camp:00` in
//!   `FORM/0100/FO0100.SPR`) and `kh1-camp-rows.json`;
//! - [`shop_tabs`]: the Moogle shop's pack screen with a fifth tab (`m_shop:01` in
//!   `FORM/0109/FO0109.SPR`; M8d).
//! - [`flip`]: the "Re:CoM's content in KH1" model-writer test (`kh1-flip-test`);
//! - [`shadow`]: M10, KH1's Shadow: its motions, attack records and sounds in Re:CoM's
//!   `ENEMY/0001` files and the behaviour's numbers (`kh1-shadow.json`).
//! - [`enemy`]: M11, the same step for any KH1 enemy twin ([`enemy::Spec`]); [`soldier`] is the
//!   Soldier's spec (`ENEMY/0002`, `kh1-soldier.json`).
//!
//! No game data is embedded; everything is read from the user's files at run time.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod ann;
pub mod build;
pub mod camp_rows;
pub mod shop_tabs;
pub mod command_menu;
pub mod convert;
pub mod eet;
pub mod effects;
pub mod sounds;
pub mod enemy;
pub mod flip;
pub mod install;
pub mod keyblade;
pub mod kh2_keyblades;
pub mod matching;
pub mod math;
pub mod mdl;
pub mod kh1_fx;
pub mod kh1_hud;
pub mod mp_bar;
pub mod recom_hp;
pub mod riku_skin;
pub mod rtm;
pub mod menu_sounds;
pub mod ses;
pub mod shadow;
pub mod soldier;
pub mod airsoldier;
pub mod powerwild;
pub mod darkball;
pub mod bandit;
pub mod pirate;
pub mod neoshadow;
pub mod airpirate;
pub mod wyvern;
pub mod searchghost;
pub mod largebody;
pub mod wightknight;
pub mod fatbandit;
pub mod gargoyle;
pub mod defender;
pub mod bouncywild;
pub mod greenrequiem;
pub mod rednocturne;
pub mod bluerhapsody;
pub mod yellowopera;
pub mod stats;
pub mod stamp;
pub mod step;
pub mod texture;
pub mod uibin;

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
