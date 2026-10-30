//! # kh1-core
//!
//! An engine-agnostic, 1:1 re-implementation of KINGDOM HEARTS FINAL MIX (PC, Steam) Sora's
//! movement and Attack combat at 60 Hz with KH1's frame step. It is the oracle the Re:CoM port is
//! tested against and the logic of the demo (`docs/kh1-rewrite.md`).
//!
//! Every game number comes in as plain data ([`Params`], [`Moveset`], [`AttackData`],
//! [`WeaponData`], [`DamageCurve`]); `kh1-data` fills them from the player's own files. The
//! `kh1_defaults()` constructors hold the values read from the Steam exe and the owner's files.
//!
//! Units are KH1's: about 1 cm, **y down**, 60 Hz frames. Code comments cite the Steam addresses
//! each rule was read from; anything inferred is marked "Assumption:".
//!
//! Per frame: [`Controller::step`] (the host's collision through [`World`]), pose the skeleton
//! for [`Output::request`] / [`Output::motion_frame`], then [`combat::resolve_hits`] with the
//! blade points, then [`Dummy::step`].

pub mod arc;
pub mod combat;
pub mod combo;
pub mod controller;
pub mod damage;
pub mod data;
pub mod math;
pub mod motion;
pub mod params;
pub mod reaction;
pub mod rng;
pub mod shape;

pub use arc::Arc;
pub use combat::{blade_params, resolve_hits, Blade, CombatParams, HitReport};
pub use combo::TargetPart;
pub use controller::{state, Button, ComboRecord, Controller, Event, FlatFloor, Input, MoveResult, Output, Refused, Swing, World};
pub use damage::{AttackerStats, DamageResult, DefenderStats, LiveAttack, MpChargeParams};
pub use data::{ActionRow, AttackData, DamageCurve, Moveset, WeaponData};
pub use math::Vec3;
pub use motion::{FlagKind, FlagWindow, MotionInfo, MotionPlayer, MotionSet};
pub use params::{stick_from_raw, Abilities, Params, GRAVITY};
pub use reaction::{Dummy, DummyPart, DummyState, Reaction, ReactionParams};
pub use rng::Kh1Rand;
pub use shape::{BladeVolume, Shape, ShapeKind};

#[cfg(test)]
mod tests;
