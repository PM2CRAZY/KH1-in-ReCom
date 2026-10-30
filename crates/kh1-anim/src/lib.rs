//! # kh1-anim
//!
//! Poses KINGDOM HEARTS FINAL MIX (PC, Steam) Sora's skeleton from a motion exactly like KH1
//! does every frame: curves sampled at `int(t·256)`, the motion-change reset with the motion's
//! starting-pose table, the helper chain, the one-pass bone constraints (FK, aim at a helper,
//! copy a helper's orientation, helper position, one-bone aim, two-bone IK), KH1's
//! non-shearing scale rule, the model matrix, and KH1's frozen-snapshot blends.
//!
//! Spec: `kh1-posing.md` (research notes, Steam addresses). The crate tests reproduce
//! Re:CoM's PS2 bakes of the same motions at Re:CoM key frames (see `tests/recom_bake.rs`).
//!
//! Units: KH1's (≈ 1 cm, Y down in world space; the model matrix is Y up), 60 Hz frames,
//! radians. Matrices are row-vector affines ([`Affine`]); [`Affine::to_yup_metres`] converts a
//! world matrix to the model's Y-up space in metres (Re:CoM's space for this skeleton).
//!
//! ```no_run
//! # fn main() -> kh1_formats::Result<()> {
//! use kh1_anim::{Rig, WeaponLine};
//! let mdls = std::fs::read("xa_ex_0010.mdls")?;
//! let mset = kh1_formats::mset::MotionSet::parse(&std::fs::read("xa_ex_0010.mset")?)?;
//! let mut rig = Rig::from_mdls(&mdls)?;
//! let swing = &mset.motions[98];
//! let pose = rig.pose(swing, 14.0);
//! let hips = pose.world[0].to_yup_metres();
//! let key = WeaponLine { t0: 0.0, t1: -3.7, frac_scale: 0.8, radius: 40.0 };
//! let (tip, grip) = rig.blade_segment(&key, 1.0, 0.0).unwrap();
//! # let _ = (hips, tip, grip); Ok(()) }
//! ```
//!
//! Not implemented (open in the spec, ask before adding): the head look-at (`FUN_140391860`,
//! instance flag bits 2/3) and the foot ground-IK (bits 0/1), both off unless gameplay turns
//! them on; and motions with no motion pointer (`FUN_140391550`).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod blend;
mod math;
mod model;
mod rig;
mod weapon;

pub use math::{point_to_yup_metres, Affine, Vec3};
pub use model::ModelInfo;
pub use rig::{flag, Pose, Rig};
pub use weapon::{WeaponLine, WEAPON_ATTACH};
