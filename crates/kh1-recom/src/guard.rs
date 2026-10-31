//! Enemy reactions to Sora's KH1 Guard (decision 27; `docs/recom-kh1-attack.md` § Enemy
//! reactions to Guard): the testable part. The runtime is `win/guard.rs`.
//!
//! At Combat = KH1 with the Config row "Guard reactions" on, when Re:CoM's own hit apply
//! (`0x14016F1C0`) returns 2 (guarded) on Sora's body:
//!
//! - a **melee** attacker (the collision is one of its `CAttackMotion` entries) plays Re:CoM's own
//!   flinch: Re:CoM's reaction picker `0x14017D260(enemy, 1)` from Re:CoM's enemy hit handler's
//!   place, and its card ends through Re:CoM's own `0x14017E1D0` when the picker chose the flinch;
//! - a **shot** (a `CBullet` / `CHomingBullet`'s own collision) is turned back: Re:CoM's own
//!   list setter (`CCollAttack` vtable `+0x20`, `0x14014E5A0(coll, 0)`) moves its collision to
//!   the enemies' list, its velocity and aim angles are reversed, homing shots aim at their
//!   shooter, and that one test answers 0 so the bullet's step doesn't end it.
//!
//! Every offset below is Steam, read in the Re:CoM decomp (and the exe where noted).

/// Re:CoM's collision lists (globals holding a list pointer). `0x14016F110` (`CCollAttack`
/// vtable `+0x30`) tests the attack against the enemies' list when its shape flags
/// (`shape+100`) `& 0xE == 0`, else against the player side's (where Sora's body is); with
/// `& 0xE == 4` against both. Checked (decomp).
pub const PLAYER_LIST: u64 = 0x1_4087_A838;
pub const ENEMY_LIST: u64 = 0x1_4087_A840;

/// `CCollAttack` (vtable `0x140523E28`, RTTI). Checked (decomp `0x14016F1C0`, `0x140157E60`).
pub mod coll {
    pub const VTABLE: u64 = 0x1_4052_3E28;
    /// Vtable `+0x20` = `0x14014E5A0(coll, flags)`: `shape+100 = flags | 1` when the shape
    /// exists. Checked (exe: the slot holds it, relocated; decomp).
    pub const SET_LIST_SLOT: u64 = 0x1_4052_3E48;
    pub const SHAPE: usize = 0x18;
    /// The attack's owner id (the actor, or the bullet itself for a shot).
    pub const OWNER: usize = 0x20;
    pub const KIND: usize = 0x44;
    pub const AP: usize = 0x48;
    pub const ELEMENT: usize = 0x50;
    /// The owner id of the last body the apply met (`0x14016F1C0`'s first write).
    pub const LAST_BODY_OWNER: usize = 0x54;
    /// The serial each body records once hit / guarded (hit-once ring, `0x14016FE20`).
    pub const SERIAL: usize = 0x70;
    /// `shape+100`: the target-list flags `0x14016F110` reads.
    pub const SHAPE_LIST_FLAGS: usize = 100;
}

/// A body (`CCollBody`) as the apply writes it. Sora's is `player+0x690`. Checked (decomp
/// `0x14016F1C0`).
pub mod body {
    pub const SORA: usize = 0x690;
    /// Owner id of the last attack applied to this body.
    pub const ATTACKER: usize = 0x98;
    /// Flags: 1 guard on, 0x20 guarded this frame.
    pub const FLAGS: usize = 0x9C;
    pub const GUARDED: u32 = 0x20;
}

/// `CAttackMotion` (vtable `0x140523E70`, RTTI) at `actor+0x100`: the melee attack collisions.
/// `0x14014E6A0(am)` walks `count` entries of 0x40 bytes; an entry is live when its record
/// (`+0x18`) is set and its collision is the object at `+0x20`. Checked (decomp + disasm
/// `0x14014E6A0`, `0x1401A4E70`).
pub mod attack_motion {
    pub const VTABLE: u64 = 0x1_4052_3E70;
    pub const IN_ACTOR: usize = 0x100;
    pub const COUNT: usize = 0x14;
    pub const ENTRIES: usize = 0x18;
    pub const STRIDE: usize = 0x40;
    pub const RECORD: usize = 0x18;
    pub const COLL: usize = 0x20;
}

/// Shots: `CBullet` (vtable `0x140525430`, `0x2F0` bytes) and `CHomingBullet` (vtable
/// `0x140525530`, `0x340` bytes), made by the bullet manager `0x140158DD0` (type id 0x1007,
/// `0x140156780`). Checked (decomp + exe RTTI).
pub mod bullet {
    pub const VTABLE: u64 = 0x1_4052_5430;
    pub const HOMING_VTABLE: u64 = 0x1_4052_5530;
    /// The mover (`CMovement`): velocity `+0x80` (its length is the speed, `0x1401D69F0`), aim
    /// angles pitch `+0x50`, yaw `+0x54`, roll `+0x58`. The homing turns (`0x1401D6A10`,
    /// `0x1401D7F00`) move the angles toward `0x1401A2F10`'s aim and `0x1401D5740` rebuilds the
    /// velocity from them (forward `0x1407C5860` = (0, 0, 1)). Checked (decomp + exe).
    pub const MOVER: usize = 0xF0;
    pub const VELOCITY: usize = 0x80;
    pub const PITCH: usize = 0x50;
    pub const YAW: usize = 0x54;
    pub const ROLL: usize = 0x58;
    /// Flags (u64). Checked (decomp `0x140156A30`, `0x1401573C0`, `0x1401575D0`).
    pub const FLAGS: usize = 0x230;
    /// The shooter's id: written by the manager `0x140158DD0` from the spawning actor's `+0x20`
    /// (only for actor types 0x1000.., 0x2000.., 0x4000; else 0). Checked (decomp).
    pub const SHOOTER: usize = 0x238;
    /// Its `CCollAttack` (owner = the bullet's own id). Checked (decomp `0x140157E60`).
    pub const COLL: usize = 0x2A8;
    /// `CBullet` homing (vtable `+0xA8` = `0x1401573C0`): with flag `0x800` and time `+0x25C` > 0,
    /// it turns toward the actor `+0x258`. Checked (decomp).
    pub const PLAIN_HOMING_FLAG: u64 = 0x800;
    pub const PLAIN_TARGET: usize = 0x258;
    /// `CHomingBullet` homing (vtable `+0xA8` = `0x1401575D0`, and `0x1401563F0` with flag
    /// `0x2000`): it turns toward the actor `+0x2F4` while that is > 0, or toward the point
    /// `+0x300` with flag `0x80000`. Checked (decomp).
    pub const HOMING_TARGET: usize = 0x2F4;
    pub const HOMING_ORBIT_FLAG: u64 = 0x2000;
    pub const HOMING_POINT_FLAG: u64 = 0x80000;
    pub const HOMING_POINT: usize = 0x300;
}

/// Re:CoM's enemy (`CBattleEnemy`) fields the reaction reads or Re:CoM's own functions write.
/// Checked (decomp `0x14017F7E0`, `0x14017D260`, `0x14017D800`).
pub mod enemy {
    pub const ID: usize = 0x20;
    /// The actor's mover; its `+0x30` is the position (`0x1401A49B0`).
    pub const MOVER: usize = 0xF0;
    pub const MOVER_POSITION: usize = 0x30;
    /// The hurt-body info: `+8` flags (4 hit this frame, set by `0x140154EB0` inside the hit
    /// handler), `+0x50` the body that took it (its `+0x98` is the attacker's id).
    pub const INFO: usize = 0xF8;
    pub const INFO_FLAGS: usize = 8;
    pub const INFO_HIT: u32 = 4;
    pub const INFO_BODY: usize = 0x50;
    /// Current state pointer; state k lives at `enemy + (k + 0x13) * 0x40`.
    pub const STATE: usize = 0x60;
    /// Next state pointer (what the picker writes).
    pub const NEXT_STATE: usize = 0x68;
    pub const FLAGS: usize = 0x260;
    pub const HP: usize = 0x274;
    /// Bit 0: a card is playing (the hit handler takes no hit then, unless bit 25 too).
    pub const CARD_PLAYING: u64 = 1;
    /// Bit 13: the hit handler only plays the guard spark and returns (no reaction).
    pub const NO_REACTION: u64 = 1 << 13;
    /// Bits 19 and 21: the picker plays only a damage voice for kind 1 (super armour).
    pub const VOICE_ONLY: u64 = 1 << 19;
    pub const SUPER_ARMOUR: u64 = 1 << 21;
    /// State 11, the flinch (`0x14017D800`: `+0x68 = enemy + 0x780`).
    pub const FLINCH_STATE: usize = 11;
}

/// Re:CoM's functions the reactions call.
pub mod func {
    /// `0x14017D260(enemy, kind)`: Re:CoM's reaction picker (its only caller is the hit handler).
    pub const PICKER: u32 = 0x0017_D260;
    /// `0x14017E1D0(enemy)`: "card done" for this enemy if it is the side-2 card now playing
    /// (`CBTL::resetNowPlaying`, no break).
    pub const END_CARD: u32 = 0x0017_E1D0;
    /// `0x14014E5A0(coll, flags)`: `CCollAttack`'s list setter.
    pub const SET_LIST: u32 = 0x0014_E5A0;
}

/// The kind the picker is given: 1, the kind of Sora's basic swing (Re:CoM row 1 `+0xB`), which
/// takes the picker's default branch to the flinch. Checked (exe + decomp).
pub const PICKER_KIND: i32 = 1;

/// The offset of state `k`'s object in the enemy (`(k + 0x13) * 0x40`).
pub const fn state_offset(k: usize) -> usize {
    (k + 0x13) * 0x40
}

/// The state index a state pointer stands for, if it points into the enemy's states.
pub fn state_index(enemy: usize, ptr: usize) -> Option<usize> {
    let off = ptr.checked_sub(enemy)?;
    (off % 0x40 == 0 && (0x13 * 0x40..0x13 * 0x40 + 64 * 0x40).contains(&off)).then(|| off / 0x40 - 0x13)
}

/// Whether Re:CoM's picker, given kind 1, puts this enemy in its flinch (else it plays only a
/// damage voice). Mirrors `0x14017D260` (bit 19) then `0x14017D800` (bit 21) for kind 1.
pub fn picker_flinches(flags: u64) -> bool {
    flags & (enemy::VOICE_ONLY | enemy::SUPER_ARMOUR) == 0
}

/// Which shot class a vtable is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shot {
    Plain,
    Homing,
}

/// The homing field to point at the shooter, if the shot homes (or may home) on an actor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Retarget {
    /// Not a homing shot (or no shooter): fly straight back.
    None,
    /// Write the shooter's id here.
    Actor(usize),
    /// Point-homing: write the shooter's position here.
    Point(usize),
}

/// What to retarget for a shot with these flags and current homing target.
pub fn retarget(shot: Shot, flags: u64, homing_target: i32, shooter: i32) -> Retarget {
    if shooter <= 0 {
        return Retarget::None;
    }
    match shot {
        Shot::Plain if flags & bullet::PLAIN_HOMING_FLAG != 0 => Retarget::Actor(bullet::PLAIN_TARGET),
        Shot::Plain => Retarget::None,
        Shot::Homing if flags & bullet::HOMING_POINT_FLAG != 0 => Retarget::Point(bullet::HOMING_POINT),
        Shot::Homing if homing_target > 0 || flags & bullet::HOMING_ORBIT_FLAG != 0 => Retarget::Actor(bullet::HOMING_TARGET),
        Shot::Homing => Retarget::None,
    }
}

/// The velocity flown the other way (`w` kept).
pub fn reversed(v: [f32; 4]) -> [f32; 4] {
    [-v[0], -v[1], -v[2], v[3]]
}

/// Re:CoM's aim angles (pitch, yaw) for a direction, as `0x1401A2F10` computes them toward a
/// point: pitch = −atan2(dy, |dxz|), yaw = atan2(dx, dz). None for a zero vector.
pub fn aim_angles(d: [f32; 3]) -> Option<(f32, f32)> {
    let h = (d[0] * d[0] + d[2] * d[2]).sqrt();
    if h == 0.0 && d[1] == 0.0 {
        return None;
    }
    Some((-d[1].atan2(h), d[0].atan2(d[2])))
}

/// The forward direction for aim angles under the same convention (the inverse of
/// [`aim_angles`]): used by the tests to check the reversal.
pub fn forward(pitch: f32, yaw: f32) -> [f32; 3] {
    let c = pitch.cos();
    [c * yaw.sin(), -pitch.sin(), c * yaw.cos()]
}

/// How many enemy hit-handler calls a remembered melee attacker waits for its own (Re:CoM runs
/// it in the attacker's update right after the attack check, so normally the very next call is
/// its own; an attacker that never runs it, a gimmick, is dropped after this many).
pub const PENDING_CALLS: u32 = 64;

/// Log caps.
pub const BLOCK_LOG_LINES: u32 = 50;
pub const HIT_LOG_LINES: u32 = 50;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flinch_state_is_0x780() {
        assert_eq!(state_offset(enemy::FLINCH_STATE), 0x780);
        assert_eq!(state_index(0x1000, 0x1000 + 0x780), Some(11));
        assert_eq!(state_index(0x1000, 0x1000 + 0x7C0), Some(12));
        assert_eq!(state_index(0x1000, 0x1000 + 0x781), None);
        assert_eq!(state_index(0x1000, 0x10), None);
    }

    #[test]
    fn picker_respects_super_armour_and_bit_19() {
        assert!(picker_flinches(0));
        assert!(picker_flinches(enemy::CARD_PLAYING | 0x40 | 0x10));
        assert!(!picker_flinches(enemy::SUPER_ARMOUR));
        assert!(!picker_flinches(enemy::VOICE_ONLY | 1));
    }

    #[test]
    fn homing_shots_aim_at_the_shooter() {
        use bullet::*;
        assert_eq!(retarget(Shot::Plain, 0, 0, 7), Retarget::None);
        assert_eq!(retarget(Shot::Plain, PLAIN_HOMING_FLAG, 0, 7), Retarget::Actor(PLAIN_TARGET));
        assert_eq!(retarget(Shot::Plain, PLAIN_HOMING_FLAG, 0, 0), Retarget::None);
        assert_eq!(retarget(Shot::Homing, 0, 12, 7), Retarget::Actor(HOMING_TARGET));
        assert_eq!(retarget(Shot::Homing, 0, 0, 7), Retarget::None);
        assert_eq!(retarget(Shot::Homing, HOMING_ORBIT_FLAG, 0, 7), Retarget::Actor(HOMING_TARGET));
        assert_eq!(retarget(Shot::Homing, HOMING_POINT_FLAG, 12, 7), Retarget::Point(HOMING_POINT));
        // The fields fit their objects (CBullet 0x2F0 bytes, CHomingBullet 0x340).
        assert!(PLAIN_TARGET + 4 <= 0x2F0 && HOMING_TARGET + 4 <= 0x340 && HOMING_POINT + 16 <= 0x340);
        assert!(COLL + 8 <= 0x2F0 && SHOOTER + 4 <= 0x2F0);
    }

    #[test]
    fn reversed_angles_point_back() {
        for v in [[1.0f32, 0.0, 0.0], [0.3, -0.2, 0.9], [-0.5, 0.4, -0.1], [0.0, 0.0, -2.0], [0.0, 1.0, 0.0]] {
            let r = reversed([v[0], v[1], v[2], 1.0]);
            assert_eq!(r[3], 1.0);
            let (p, y) = aim_angles([r[0], r[1], r[2]]).unwrap();
            let f = forward(p, y);
            let n = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
            for i in 0..3 {
                assert!((f[i] - r[i] / n).abs() < 1e-5, "{v:?}: {f:?} vs {r:?}");
            }
        }
        assert_eq!(aim_angles([0.0, 0.0, 0.0]), None);
    }
}
