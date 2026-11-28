//! KH1's magic inside Re:CoM's own magic action (M7 stage 7.4, decision 34): the cardless cast
//! and Fire. The testable part; the Re:CoM side is `win/magic.rs`, the notes
//! `docs/recom-m7-magic.md`.
//!
//! **The cast is KH1's frame on Re:CoM's magic action `0x13`**, as a swing is KH1's frame on
//! Re:CoM's attack action `0xA` (M5):
//!
//! - KH1's command (`0x1402A7510` type 2 → `0x1402A3600` → `0x1402A3DB0` / its air branch, and
//!   the Fire fast path) runs in [`Kh1Sora::frame`]: KH1's commit gate, its cast motions and
//!   their windows (the KH1 motion record, as for every KH1 move), the air hop, the facing.
//! - Re:CoM runs the action: [`Host::start_cast`] makes Sora's action Re:CoM's magic action with
//!   a **KH1 row** in Re:CoM's attack-row table format ([`fire_row_bytes`]: Re:CoM's own Fire /
//!   Fira / Firaga row with KH1's numbers), Re:CoM's clips play the cast motions, and at KH1's
//!   release frame [`Host::release_cast`] has Re:CoM's own shot runtime (`CHomingBullet`) fire the
//!   ball from that row: Re:CoM steps, homes, draws, collides and ends it, with Re:CoM's Fire
//!   effects and sounds (decision 34).
//! - **KH1's script decides the release** ([`Kh1Sora::cast_step`], `xm_fire01.mag` decoded): on
//!   the wind-up path, the frame the wind-up request (`0x36` / `0x83`) is no longer playing; on
//!   the fast path, 8 frames after the command. At that frame the release request (`0x3D` /
//!   `0x8A`) must be playing, else nothing is cast and **no MP is paid**; otherwise the ball is
//!   launched and the MP paid (KH1 syscall 47 after the launch).
//!
//! Units as [`crate::movement`]: KH1 frames (60 Hz) and cm; Re:CoM rows hold metres × 100 and
//! ticks / 100 (50 ticks per 60 Hz frame, `docs/recom-timing.md`).

use crate::movement::{Action, Host, Kh1Sora};
use kh1_core::controller::state;
use kh1_core::damage::{compute_damage, AttackerStats, LiveAttack};
use kh1_core::motion::{req, request, FlagKind, MotionInfo, MotionSet};
use kh1_core::{AttackData, DamageCurve, DefenderStats, Kh1Rand, TargetPart, Vec3, WeaponData};

/// KH1's spells (`0x142D22B80` order; the command `level + (spell + 2) × 3`).
pub mod spell {
    pub const FIRE: u8 = 0;
    pub const BLIZZARD: u8 = 1;
    pub const THUNDER: u8 = 2;
    pub const CURE: u8 = 3;
    pub const GRAVITY: u8 = 4;
    pub const STOP: u8 = 5;
    pub const AERO: u8 = 6;
    pub const NAMES: [&str; 7] = ["fire", "blizzard", "thunder", "cure", "gravity", "stop", "aero"];
}

/// Spells cast so far (Fire 7.4, Blizzard 7.5, Thunder 7.6, Cure 7.7, Gravity 7.8, Stop 7.9, Aero 7.10;
/// each its own rows and script numbers).
pub fn castable(s: u8) -> bool {
    matches!(s, spell::FIRE | spell::BLIZZARD | spell::THUNDER | spell::CURE | spell::GRAVITY | spell::STOP | spell::AERO)
}

/// A spell's MP cost (its data's `+0x30`, the same at every level): charge units below 100,
/// whole points × 100 from 100 on (`0x1402A4C10` / `0x1402A5B60`). Checked (files).
pub fn cost(s: u8) -> i32 {
    use crate::mp::cost::*;
    [FIRE, BLIZZARD, THUNDER, CURE, GRAVITY, STOP, AERO].get(s as usize).copied().unwrap_or(i32::MAX)
}

/// KH1's cast motion requests (`0x1402A3DB0` / `0x1402A3600` / the fast path in `0x1402A7510`).
/// Checked (decomp).
pub mod cast_req {
    /// Fire's wind-up and release, ground and air.
    pub const FIRE_WINDUP: u16 = 0x36;
    pub const FIRE_RELEASE: u16 = 0x3D;
    pub const FIRE_AIR_WINDUP: u16 = 0x83;
    pub const FIRE_AIR_RELEASE: u16 = 0x8A;
    /// Blizzard's one cast motion, ground and air (`0x36 + 1`, `0x83 + 1`; entries 55 / 80).
    pub const BLIZZARD: u16 = 0x37;
    pub const BLIZZARD_AIR: u16 = 0x84;
    /// Thunder's one cast motion (`0x36 + 2`, `0x83 + 2`; entries 56 / 81).
    pub const THUNDER: u16 = 0x38;
    pub const THUNDER_AIR: u16 = 0x85;
    /// Cure's one cast motion (`0x36 + 3`, `0x83 + 3`; entries 56 / 81, Thunder's). Checked (files).
    pub const CURE: u16 = 0x39;
    pub const CURE_AIR: u16 = 0x86;
    /// Gravity's one cast motion (`0x36 + 4`, `0x83 + 4`; entries 55 / 80, Blizzard's). Checked
    /// (files).
    pub const GRAVITY: u16 = 0x3A;
    pub const GRAVITY_AIR: u16 = 0x87;
    /// Stop's one cast motion (`0x36 + 5`, `0x83 + 5`; entries 56 / 81, Thunder's). Checked (files).
    pub const STOP: u16 = 0x3B;
    pub const STOP_AIR: u16 = 0x88;
    /// Aero's one cast motion (`0x36 + 6`, `0x83 + 6`; entries 56 / 81, Thunder's; the script reads
    /// requests 60 / 137). Checked (files).
    pub const AERO: u16 = 0x3C;
    pub const AERO_AIR: u16 = 0x89;

    /// The first cast motion a spell's normal cast plays: `0x1402A3DB0` plays `0x36 + spell`
    /// (Fire: `0x36` then `0x3D`), the air branch of `0x1402A3600` `0x83 + spell` (Fire: `0x83`
    /// then `0x8A`). Checked (decomp + disasm `0x1402A3E92..0x1402A3ED7`).
    pub fn first(spell: u8, air: bool) -> u16 {
        if air {
            FIRE_AIR_WINDUP + spell as u16
        } else {
            FIRE_WINDUP + spell as u16
        }
    }

    /// The motion the spell's script requires at its release check (`xm_fire01`: `0x3D` / `0x8A`;
    /// `xm_blizzard01`: 55 / 132 = `0x37` / `0x84`). Checked (files).
    pub fn release(spell: u8, air: bool) -> u16 {
        match (spell, air) {
            (super::spell::FIRE, false) => FIRE_RELEASE,
            (super::spell::FIRE, true) => FIRE_AIR_RELEASE,
            _ => first(spell, air),
        }
    }
}

/// Re:CoM's magic action and its attack ids (`initMagic` `0x1402203D0` sets `+0x638` and
/// `+0x414` to them; Fire `0x22`). Checked (decomp).
pub const RECOM_MAGIC_ACTION: u32 = 0x13;
/// `player+0x604` as `initMagic` sets it.
pub const MAGIC_PHASE: u32 = 3;

/// Re:CoM's attack id (`+0x638` / `+0x414`) for a spell: Fire 0x22 … Aero 0x28.
pub fn recom_attack_id(s: u8) -> u32 {
    0x22 + s as u32
}

/// Re:CoM's base row (`0x1407C6C70 + id × 16`) whose effect set the KH1 spell level plays:
/// Fire `0x22`, Fira `0x29` (set 4), Firaga `0x2A` (set 5). Checked (exe: the sets hold
/// `fire_all` entries; `initStockSkill`'s switch maps the -ra / -ga sleights to 0x29 / 0x2A).
pub fn recom_base_row(s: u8, level: u8) -> Option<usize> {
    match (s, level) {
        (spell::FIRE, 1) => Some(0x22),
        (spell::FIRE, 2) => Some(0x29),
        (spell::FIRE, 3) => Some(0x2A),
        // Blizzard 0x23 (set 6), Blizzara 0x2B (set 8), Blizzaga 0x2C (set 10): `initStockSkill`'s
        // -ra / -ga sleights 0x0E / 0x15. Checked (exe: the sets hold `bliz_all`).
        (spell::BLIZZARD, 1) => Some(0x23),
        (spell::BLIZZARD, 2) => Some(0x2B),
        (spell::BLIZZARD, 3) => Some(0x2C),
        // Thunder 0x24 (set 12), Thundara 0x2E (13), Thundaga 0x2F (14): `initStockSkill`'s
        // sleights 0x0F / 0x16. Checked (exe: the sets hold `_thunder_all`).
        (spell::THUNDER, 1) => Some(0x24),
        (spell::THUNDER, 2) => Some(0x2E),
        (spell::THUNDER, 3) => Some(0x2F),
        // Cure 0x25 (set 15), Cura 0x36 (16), Curaga 0x37 (17): `initStockSkill`'s sleights 0x10 /
        // 0x17. Checked (exe: the rows' clip 0x39, sets 15–17 of `cure_all`).
        (spell::CURE, 1) => Some(0x25),
        (spell::CURE, 2) => Some(0x36),
        (spell::CURE, 3) => Some(0x37),
        // Gravity 0x26 (set 18), Gravira 0x38 (19), Graviga 0x39 (20): `initStockSkill`'s sleights
        // 0x11 / 0x18. Checked (exe: clip 0x3A, sets 18–20 of `grav_all`).
        (spell::GRAVITY, 1) => Some(0x26),
        (spell::GRAVITY, 2) => Some(0x38),
        (spell::GRAVITY, 3) => Some(0x39),
        // Stop 0x27 (set 21), Stopra 0x3A (22), Stopga 0x3B (23): `initStockSkill`'s sleights 0x12 /
        // 0x19. Checked (exe: clip 0x39, sets 21–23 of `stop_all`, kind 8).
        (spell::STOP, 1) => Some(0x27),
        (spell::STOP, 2) => Some(0x3A),
        (spell::STOP, 3) => Some(0x3B),
        // Aero 0x28 (set 24), Aerora 0x3C (25), Aeroga 0x3D (26): `initStockSkill`'s sleights 0x13 /
        // 0x1A. Checked (exe: clip 0x39, sets 24–26 of `airo_all`).
        (spell::AERO, 1) => Some(0x28),
        (spell::AERO, 2) => Some(0x3C),
        (spell::AERO, 3) => Some(0x3D),
        _ => None,
    }
}

/// Re:CoM's rows 0x22 / 0x29 / 0x2A as in the Steam exe (the KH1 rows' templates; the runtime
/// refuses to build them from other bytes). Checked (exe).
pub const RECOM_FIRE_ROWS: [[u8; 16]; 3] = [
    [0x37, 0x01, 0x03, 0x00, 0x14, 0x4B, 0x00, 0x00, 0x0A, 0x0F, 0x00, 0x01, 0x0E, 0x14, 0x14, 0x00],
    [0x37, 0x01, 0x04, 0x00, 0x1E, 0x64, 0x00, 0x00, 0x0B, 0x0F, 0x00, 0x02, 0x0E, 0x14, 0x19, 0x00],
    [0x37, 0x01, 0x05, 0x00, 0x28, 0x7D, 0x00, 0x00, 0x0C, 0x0F, 0x00, 0x03, 0x0E, 0x14, 0x1E, 0x00],
];

/// KH1's Fire ball (`xm_fire01..03.mag`, the same script at every level; Checked (files: decoded
/// with `magdis.py`; the syscalls in the KH1 decomp)).
pub mod fire {
    /// The ball's speed, cm per 60 Hz frame (`ind+0x34`, sys 18 `pos += dir × speed × step`).
    pub const SPEED: f32 = 40.0;
    /// Its life in frames (the thread's `ind+0x30` = 30, −frame step per frame).
    pub const LIFE: f32 = 30.0;
    /// Its homing turn, degrees per frame (`ind+0x38`, sys 24).
    pub const TURN_DEG: f32 = 3.0;
    /// It starts fading this many frames before its end (`ind+0x3C`).
    pub const FADE: f32 = 20.0;
    /// A ball cast with no target picks the best enemy within this range of the ball each frame
    /// (`ind+0x40`, sys 78; [`super::retarget`]).
    pub const RETARGET: f32 = 500.0;
    /// The fast path's wait: the release 8 frames after the command (`L006fa(8.0)`).
    pub const FAST_WAIT: f32 = 8.0;
    /// MP cost (spell data `+0x30`, all levels): 30 charge units.
    pub const COST: i32 = crate::mp::cost::FIRE;
    /// The release motion takes a new Fire from this frame (`0x140293480`). Checked (decomp).
    pub const RECAST_FROM: f32 = 14.0;
}

/// KH1's Fire re-target (M7 stage 7.4b, decision 36). The ball thread's loop (`xm_fire01.mag`
/// `L000c2`, the same at every level; Checked (files: decoded with `magdis.py`)) reads the cast's
/// target from the thread's arguments every frame:
///
/// - **no target** (the command had none): it picks one with sys 78 (`0x1402BA440` →
///   `0x1402B0E90(caster, ball position, direction with w = 2, 500)`) and homes on it this frame;
///   the pick is not kept, so the next frame picks again;
/// - **a target that is no longer alive** (sys 120 `0x1402BAB10` → `0x140291FF0`: the actor's
///   `+0x374` bit 0 set and bit 1 clear): no homing this frame and **no new pick** (it flies
///   straight);
/// - a live target: homes on its part (sys 119).
///
/// The pick `0x1402B0E90` (Checked (decomp)): every actor of the actor array (`0x140291AB0`:
/// live, not flagged `+0 & 0x200`, on a side `+0x374 >> 5 & 3` that is not 0 and not the
/// caster's), `v` = its position (`+0x10`) − the ball's, `d` = |v| (3D, `0x1402A9240`); taken
/// only when `d < 500`; score `d × ((1 − cos) × w + 1)` with `cos` = the clamped cosine between
/// `v` and the ball's direction (`0x1402A8C70`, 0 for a zero vector) and `w` = 2; the first
/// candidate in range is taken, later ones only with a smaller score. Then (in the same frame) the
/// ball turns ≤ 3° toward the pick's part and moves.
pub mod retarget {
    /// The range, Re:CoM metres (KH1's 500 cm).
    pub const RANGE_M: f32 = super::fire::RETARGET / 100.0;
    /// The off-axis weight (the script writes 2.0 into the direction's `w`).
    pub const WEIGHT: f32 = 2.0;

    fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
    }
    fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }

    /// `0x1402A8C70`: the cosine between two vectors, clamped to −1..1, 0 when either is zero.
    pub fn cos(a: [f32; 3], b: [f32; 3]) -> f32 {
        let (la, lb) = (dot(a, a).abs().sqrt(), dot(b, b).abs().sqrt());
        if la == 0.0 || lb == 0.0 {
            return 0.0;
        }
        (dot(a, b) / (lb * la)).clamp(-1.0, 1.0)
    }

    /// One candidate's distance and score, or None when it is out of range. Positions in Re:CoM
    /// metres: KH1's (x, y, z) cm is Re:CoM's (x, −y, −z) / 100, a rotation and a uniform scale,
    /// so distances scale by 1/100 and the cosine is the same.
    pub fn score(ball: [f32; 3], dir: [f32; 3], at: [f32; 3]) -> Option<(f32, f32)> {
        let v = sub(at, ball);
        let d = dot(v, v).abs().sqrt();
        (d < RANGE_M).then(|| (d, d * ((1.0 - cos(v, dir)) * WEIGHT + 1.0)))
    }

    /// `0x1402B0E90`: the best candidate `(id, position)` for a ball at `ball` flying along `dir`,
    /// with its distance. The first one in range is taken, later ones with a smaller score
    /// (KH1's `lVar2 == 0 || score < best`).
    pub fn pick(ball: [f32; 3], dir: [f32; 3], cands: impl IntoIterator<Item = (u32, [f32; 3])>) -> Option<(u32, f32)> {
        let mut best: Option<(u32, f32, f32)> = None;
        for (id, at) in cands {
            if let Some((d, s)) = score(ball, dir, at) {
                if best.is_none_or(|b| s < b.2) {
                    best = Some((id, d, s));
                }
            }
        }
        best.map(|b| (b.0, b.1))
    }
}

/// KH1's Blizzard (`xm_blizzard01..03.mag`, decoded with `magdis.py`: one script, the level's shard
/// speed passed to it; Checked (files), the syscalls in the KH1 decomp).
pub mod blizzard {
    /// The script waits this long after the command (`L0086e(28)`), then needs its cast motion
    /// (`0x37` / `0x84`) still playing, else ends: no spell, no MP.
    pub const WAIT: f32 = 28.0;
    /// Shard speed by level, cm per frame (`L006dc(args, 20 / 30 / 40)`).
    pub const SPEED: [f32; 3] = [20.0, 30.0, 40.0];
    /// Shard life, frames (`L001dc`: `ind+0x0` = 26).
    pub const LIFE: f32 = 26.0;
    /// The height each shard keeps above the floor it finds below it (`ind+0x8` = 50).
    pub const HEIGHT: f32 = 50.0;
    /// The first cone direction (local: +z forward, x sideways, y vertical), the script's own
    /// values (`*(sp+0x30)` = (0.383, 0, 0.924): 22.5° off the axis); each next shard turns it
    /// 60° about the axis (sys 31). Six cone shards, then one straight ahead (0, 0, 1).
    pub const CONE: [f32; 3] = [0.383, 0.0, 0.924];
    pub const RING_DEG: f32 = 60.0;
    pub const CONE_SHARDS: usize = 6;
    /// A cone shard starts its direction's sideways part × 50 cm off the axis (sys 93 then sys
    /// 10 / 7: 0.383 × 50 = 19.15 cm); the straight one at the Keyblade point.
    pub const OFFSET: f32 = 50.0;
    /// Attack radius by level (data `+0x5C`: 70 / 90 / 120 cm).
    pub const RADIUS: [f32; 3] = [70.0, 90.0, 120.0];
    /// MP cost (data `+0x30`): 30 charge units.
    pub const COST: i32 = crate::mp::cost::BLIZZARD;
}

/// The shards of one Blizzard in Re:CoM's frame (metres, y up), for Sora facing `yaw` (Re:CoM's
/// body yaw: forward = (sin yaw, 0, cos yaw)): `(start offset from the Keyblade point, unit
/// direction)`, the six cone shards first, then the straight one, as the script makes them.
///
/// KH1 rotates its local vectors by the actor's yaw (sys 10) in its own axes (y down); the six
/// cone shards sit every 60° about the forward axis, a set that is the same under left / right and
/// up / down mirroring, so it maps onto Re:CoM's axes without regard to which local axis is which.
pub fn blizzard_shards(yaw: f32) -> [([f32; 3], [f32; 3]); 7] {
    use blizzard::*;
    let f = [yaw.sin(), 0.0, yaw.cos()];
    let r = [yaw.cos(), 0.0, -yaw.sin()];
    let u = [0.0, 1.0, 0.0];
    let comb = |a: f32, b: f32, c: f32| [r[0] * a + u[0] * b + f[0] * c, r[1] * a + u[1] * b + f[1] * c, r[2] * a + u[2] * b + f[2] * c];
    let mut out = [([0.0; 3], f); 7];
    for (i, o) in out.iter_mut().take(CONE_SHARDS).enumerate() {
        let phi = (RING_DEG * i as f32).to_radians();
        // sys 31: the cone vector turned about the forward (local z) axis.
        let (lx, ly) = (CONE[0] * phi.cos() - CONE[1] * phi.sin(), CONE[0] * phi.sin() + CONE[1] * phi.cos());
        let off = comb(lx * OFFSET / 100.0, ly * OFFSET / 100.0, 0.0);
        *o = (off, comb(lx, ly, CONE[2]));
    }
    out
}

/// KH1's shard height step (`xm_blizzard01.mag` `L0029e`, Re:CoM metres, y up): the shard has
/// moved sideways this frame; a ray from its height straight down by `HEIGHT − dy` (sys 77 →
/// `0x140299530`, any surface) finds `floor` (the hit's height) → the shard sits `HEIGHT` above it;
/// no hit → it climbs or sinks by its own `dy` (its direction's vertical part × speed).
pub fn blizzard_glide_y(y: f32, dy: f32, floor: Option<f32>) -> f32 {
    match floor {
        Some(h) => h + blizzard::HEIGHT / 100.0,
        None => y + dy,
    }
}

/// The glide ray's length below the shard, metres: `HEIGHT + dy_down` in KH1's y-down terms.
pub fn blizzard_ray(dy: f32) -> f32 {
    blizzard::HEIGHT / 100.0 - dy
}

/// KH1's Blizzard attack data (btltbl `0x6078` / `0x60E8` / `0x6158`, Checked (files)): power
/// 22 / 27 / 34 + max MP, ice (element 3), flinch, radius 70 / 90 / 120, no MP charge, no re-hit.
pub fn blizzard_attack(level: u8) -> AttackData {
    let i = level.clamp(1, 3) as usize - 1;
    let power = [22, 27, 34][i];
    AttackData {
        stat_source: 1,
        element: 3,
        kind_party: 3,
        kind: 3,
        bone: -1,
        power_party: power,
        mp_charge: 0,
        clash: 5,
        priority: 1,
        recoil_request: 110,
        power,
        radius: blizzard::RADIUS[i],
        ..Default::default()
    }
}

/// KH1's attack data for a spell level (the hit's damage).
pub fn spell_attack(s: u8, level: u8) -> Option<AttackData> {
    match s {
        spell::FIRE => Some(fire_attack(level)),
        spell::BLIZZARD => Some(blizzard_attack(level)),
        spell::THUNDER => Some(thunder_attack(level)),
        spell::GRAVITY => Some(gravity_attack(level)),
        spell::STOP => Some(stop_attack(level)),
        // Aero's hits are its wind's (Aerora / Aeroga); the barrier itself is a status on Sora.
        spell::AERO => Some(aero_wind_attack(level)),
        // Cure heals (Re:CoM's amounts, decision 34): no hit, no damage.
        _ => None,
    }
}

/// Re:CoM's Blizzard rows 0x23 / 0x2B / 0x2C as in the Steam exe (clip 0x3A, effect sets 6 / 8 /
/// 10 of `bliz_all`; the KH1 rows' templates). Checked (exe).
pub const RECOM_BLIZZARD_ROWS: [[u8; 16]; 3] = [
    [0x3A, 0x01, 0x06, 0x1E, 0x0A, 0x32, 0x00, 0x00, 0x00, 0x14, 0x00, 0x01, 0x0A, 0x00, 0x0A, 0x00],
    [0x3A, 0x01, 0x08, 0x1E, 0x0F, 0x4B, 0x00, 0x00, 0x00, 0x14, 0x00, 0x02, 0x0A, 0x00, 0x0F, 0x00],
    [0x3A, 0x01, 0x0A, 0x1E, 0x14, 0x64, 0x00, 0x00, 0x00, 0x14, 0x00, 0x03, 0x0A, 0x00, 0x14, 0x00],
];

/// The KH1 Blizzard row: Re:CoM's row of that level (clip, blend, effect set: decision 34) with
/// KH1's shard, in the fields Re:CoM's Blizzard case (`0x140222170` case `0x23`, Checked (decomp))
/// reads: `+4` radius × 0.01 m (70 / 90 / 120), `+9` speed × 0.01 m a frame (20 / 30 / 40), `+0xB`
/// kind 1 (flinch; KH1 kind 3), `+0xC` range 255 m (none: the life ends it), `+0xE` life × 100
/// ticks (13 = 26 frames), `+0xF` = `0x80` (a KH1 row); `+5` (the case's `desc+0x1C`) kept.
pub fn blizzard_row_bytes(recom: &[u8; 16], level: u8) -> [u8; 16] {
    let i = level.clamp(1, 3) as usize - 1;
    let mut b = *recom;
    b[3] = 0;
    b[4] = blizzard::RADIUS[i] as u8;
    b[9] = blizzard::SPEED[i] as u8;
    b[0xB] = 1;
    b[0xC] = 255;
    b[0xD] = 0;
    b[0xE] = (blizzard::LIFE * 50.0 / 100.0) as u8;
    b[0xF] |= FLAG_KH1;
    b
}

/// The shard's descriptor flags: Re:CoM's Blizzard `0x4000000` (fade, not vanish, when Re:CoM kills
/// a shot) with KH1's ends: `0x100` (a hit or a wall ends it, as KH1's shard ends on its attack's
/// first hit) and `4` (the life ends it). No homing. Checked (decomp `0x140222170` case `0x23`,
/// `0x140156E20`, `0x1401569B0`).
pub const KH1_BLIZZARD_FLAGS: u64 = 0x400_0000 | 0x100 | 4;

/// KH1's Thunder (`xm_thunder01..03.mag`, decoded with `magdis.py`: the same script at every level;
/// Checked (files), the syscalls in the KH1 decomp).
///
/// At the command (`L004b6`): the area's centre is the command target's part if it is alive, else
/// Sora's position (the area marker effect alone is drawn 2 m ahead along the camera); the
/// distances are measured from the target (alive), else Sora. Then a screen tint to 50 % over 20
/// frames (`L00064`, sys 52), **wait 30**, the release check (`0x38` / `0x85`; else the tint goes
/// back and nothing is cast, no MP), the strike thread, **the MP** (sys 47, 100 = one point),
/// then 40 frames and the tint back over 20.
///
/// The strike thread (`0x17E`): the centre moves to the target's part if it is still alive; every
/// enemy (sys 65, up to 30 slots) with its distance from the target / Sora goes into a list sorted
/// nearest first (`L0069c`: selection sort, a later entry replaces the minimum only when nearer);
/// then down the list: an enemy whose hurt bodies overlap the area (sys 90 → `0x1402B6AF0`: a
/// vertical cylinder, type 2, radius 600, half-height 500, around the centre) **is struck
/// directly** (sys 160 → `0x1402BBB70`: the attack applied onto it, no collision) and the thread
/// waits 8 frames (`L00c8e(8)`); one outside it is passed over at once. Each enemy once.
pub mod thunder {
    pub const WAIT: f32 = 30.0;
    /// The area: a vertical cylinder, cm (`L004b6`: 600, height 1000 → ±500).
    pub const RADIUS: f32 = 600.0;
    pub const HALF_HEIGHT: f32 = 500.0;
    /// The list's slots.
    pub const MAX: usize = 30;
    /// The wait after each strike, frames.
    pub const INTERVAL: f32 = 8.0;
    /// MP cost (data `+0x30` = 100): one whole point.
    pub const COST: i32 = crate::mp::cost::THUNDER;
    /// The screen tint (not built: Re:CoM has no world-only tint we found; open question): to
    /// this level over `DIM_FRAMES`, held until 40 frames after the release, back over 20.
    pub const DIM: f32 = 0.5;
    pub const DIM_FRAMES: f32 = 20.0;
}

/// KH1's Thunder attack data (btltbl `0x61C8` / `0x6238` / `0x62A8`, Checked (files)): power 16 / 20
/// / 26 + **max MP × 2** (stat source 3), thunder (element 4), flinch, no radius (struck directly),
/// no MP charge, no re-hit.
pub fn thunder_attack(level: u8) -> AttackData {
    let power = [16, 20, 26][level.clamp(1, 3) as usize - 1];
    AttackData {
        stat_source: 3,
        element: 4,
        kind_party: 3,
        kind: 3,
        bone: -1,
        power_party: power,
        mp_charge: 0,
        clash: 6,
        priority: 0,
        recoil_request: 110,
        power,
        radius: 0.0,
        ..Default::default()
    }
}

/// Re:CoM's Thunder rows 0x24 / 0x2E / 0x2F as in the Steam exe (clip 0x39, effect sets 12 / 13 / 14
/// of `_thunder_all`, Re:CoM's bolt: fall speed `+9`, radius `+4` × 0.1 m, range `+0xC`). Checked
/// (exe).
pub const RECOM_THUNDER_ROWS: [[u8; 16]; 3] = [
    [0x39, 0x00, 0x0C, 0x1E, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x28, 0x00, 0x1C, 0x08, 0x00, 0x00, 0x00],
    [0x39, 0x00, 0x0D, 0x1E, 0x14, 0x00, 0x00, 0x00, 0x00, 0x28, 0x00, 0x1D, 0x08, 0x00, 0x00, 0x00],
    [0x39, 0x00, 0x0E, 0x1E, 0x28, 0x00, 0x00, 0x00, 0x00, 0x28, 0x00, 0x1E, 0x08, 0x00, 0x00, 0x00],
];

/// The KH1 Thunder row: Re:CoM's row (clip, blend, effect set, its bolt's speed / radius / range:
/// decision 34, Re:CoM's visual) with `+3` = 0 (KH1's script decides the release), `+0xB` = 1
/// (flinch; KH1 kind 3 — Re:CoM's Thunder kinds 0x1C–0x1E are not used) and `+0xF` = 0x80.
pub fn thunder_row_bytes(recom: &[u8; 16]) -> [u8; 16] {
    let mut b = *recom;
    b[3] = 0;
    b[0xB] = 1;
    b[0xF] |= FLAG_KH1;
    b
}

/// KH1's Cure (`xm_cure01..03.mag`, decoded with `magdis.py`: the same script at every level;
/// Checked (files), the syscalls in the KH1 decomp) with **Re:CoM's heal amounts** (decision 34):
///
/// - the command: the charge effect on the Keyblade, **wait 36** (`L004e8(36)`), the release check
///   (the request must still be `0x39` / `0x86`, else it ends: no heal, no MP);
/// - the heal thread (`L00056`): the target is the command's party member, else the caster
///   (`sp+0x1c`); it is skipped (no heal, **no MP**) when the target is not alive (sys 120), has
///   flag `0x200` or lacks `0x8000` (`L00276`; Assumption: defeated / healable); else the data's
///   attack is struck onto it (sys 23 + sys 60: KH1's HP += 15 / 27 / 36 + max MP), then **the MP**
///   (sys 47, 100 = one point), then the heal effect follows the target.
///
/// At the KH1 setting the heal is Re:CoM's: its Cure case (`0x140222170` case `0x25`) heals
/// `0x1401E59A0(player, AP, 0, 1)` with AP = `BattleCardList[0x1D]` 500 / 10 = **50**, Cura
/// (`StockSkillList[0x10]` 1500 / 10) **150**, Curaga (`[0x17]` 3000 / 10) **300** (`SY0002.BIN`,
/// research `m7-magic.md` § 3.9 / 4.1.4; Checked (files + decomp)). Sora is the only target (no
/// party members yet).
pub mod cure {
    pub const WAIT: f32 = 36.0;
    /// Re:CoM's heal per level (decision 34).
    pub const HEAL: [i32; 3] = [50, 150, 300];
    /// MP cost (data `+0x30` = 100): one whole point.
    pub const COST: i32 = crate::mp::cost::CURE;

    /// The heal of a level.
    pub fn heal(level: u8) -> i32 {
        HEAL[level.clamp(1, 3) as usize - 1]
    }

    /// The thread's target test (`L00056`): the target alive, not flagged `0x200`, healable
    /// (`0x8000`). For Sora in Re:CoM: HP left and not in Re:CoM's death path (`+0x7B0`).
    pub fn target_ok(hp: i32, dead: bool) -> bool {
        hp > 0 && !dead
    }
}

/// Re:CoM's Cure rows 0x25 / 0x36 / 0x37 as in the Steam exe (clip 0x39, effect sets 15 / 16 / 17
/// of `cure_all`, kind 0x19, release `+3` 30). Checked (exe).
pub const RECOM_CURE_ROWS: [[u8; 16]; 3] = [
    [0x39, 0x00, 0x0F, 0x1E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x19, 0x00, 0x00, 0x00, 0x00],
    [0x39, 0x00, 0x10, 0x1E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x19, 0x00, 0x00, 0x00, 0x00],
    [0x39, 0x00, 0x11, 0x1E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x19, 0x00, 0x00, 0x00, 0x00],
];

/// The KH1 Cure row: Re:CoM's row (clip, blend, `cure_all` set: decision 34) with `+3` = 0 (KH1's
/// script decides the release) and `+0xF` = 0x80. Its kind is not read (no hit).
pub fn cure_row_bytes(recom: &[u8; 16]) -> [u8; 16] {
    let mut b = *recom;
    b[3] = 0;
    b[0xF] |= FLAG_KH1;
    b
}

/// KH1's Gravity (`xm_gravity01..03.mag`, decoded with `magdis.py`: one script, the level passes
/// (final scale, fall frames) = (1.0, 60) / (1.5, 70) / (1.6, 60); Checked (files), the syscalls in
/// the KH1 decomp):
///
/// - the command (`L008cc`): the charge effect on the Keyblade; the marker thread (`0x25A`, 28
///   frames): the point is the target's part if it is alive, else the caster's position + **200 cm
///   along his facing** (taken at the command); the sphere's start is that point **240 cm up**
///   (`ind+0x4` = −230, then −10); **wait 28** (`L009e2(ind+0x1c = 28)`); the release check
///   (`0x3A` / `0x87`, else end: no spell, no MP); the falling thread (`0x21`); **the MP** (sys 47,
///   100); the hand effect;
/// - the fall (`0x44..0x16C`): the sphere starts at scale 0.01 and falls **230 cm** over N = 60 /
///   70 / 60 frames, its scale growing linearly to S = 1.0 / 1.5 / 1.6; each frame the enemies whose
///   bodies touch a sphere of **170 × the current scale** are slowed (`L0055c`: `actor+0x88 |= 4`);
/// - the impact (`L0044c`, the run after the N-th): **one direct strike** (sys 23 + 60) on every
///   enemy whose bodies touch a sphere of **170 × S** cm (170 / 255 / 272) at the sphere's centre
///   (sys 90 type 1); then the sphere grows × 1.02 a frame while it fades for 20 frames.
///
/// Damage (`0x1402BFA80`, element 8): `current HP × (power + 5 × max MP)% × resist[8]`, raised to
/// the record's floor (at least 1); no Defence, no cap. Power 40 / 55 / 70 (btltbl `0x6468` /
/// `0x64D8` / `0x6548`, stat source 5). Checked (files + decomp).
pub mod gravity {
    pub const WAIT: f32 = 28.0;
    /// The hit sphere's radius at scale 1, cm.
    pub const RADIUS: f32 = 170.0;
    /// The fall, cm, and the start above the marker point (230 + 10).
    pub const FALL: f32 = 230.0;
    pub const START_UP: f32 = 240.0;
    /// With no target: the marker point this far along the caster's facing, cm.
    pub const AHEAD: f32 = 200.0;
    /// The start scale and, per level, the final scale and the fall's frames.
    pub const START_SCALE: f32 = 0.01;
    pub const SCALE: [f32; 3] = [1.0, 1.5, 1.6];
    pub const FRAMES: [f32; 3] = [60.0, 70.0, 60.0];
    /// The fade after the impact: frames, growth a frame.
    pub const FADE: f32 = 20.0;
    pub const FADE_GROWTH: f32 = 1.02;
    /// MP cost (data `+0x30` = 100): one whole point.
    pub const COST: i32 = crate::mp::cost::GRAVITY;

    /// A level's (final scale, fall frames).
    pub fn of(level: u8) -> (f32, f32) {
        let i = level.clamp(1, 3) as usize - 1;
        (SCALE[i], FRAMES[i])
    }

    /// The impact's hit radius, Re:CoM metres (170 × S cm).
    pub fn impact_radius_m(level: u8) -> f32 {
        RADIUS * of(level).0 / 100.0
    }

    /// The sphere's scale `t` frames into the fall (linear, `sp+0x24` a frame).
    pub fn scale_at(level: u8, t: f32) -> f32 {
        let (s, n) = of(level);
        START_SCALE + (s - START_SCALE) * (t / n).clamp(0.0, 1.0)
    }

    /// Re:CoM's bullet flag (descriptor `+0x60` bit 7, bullet `+0x230`) that makes its sphere grow
    /// by itself: `CHomingBullet`'s init `0x140158660` sets the collision radius to `desc+0x1C`
    /// and the effect's scale `+0x10` to `g = desc+0x1C / desc+0x78` (not `desc+0x90`), then runs
    /// one growth step `0x140156350`; every later update in flight (`0x140156E20`, until the fade
    /// `+0x230 & 0x800000000000`) runs it again: radius += `desc+0x1C`, effect scale += `g`, not
    /// scaled by the frame step. So in Re:CoM the drawn scale is always **the sphere's radius ÷
    /// `desc+0x78`**. Checked (decomp).
    pub const RECOM_SELF_GROWTH: u64 = 0x80;

    /// The KH1 sphere's flags: Re:CoM's Gravity case's as they are (owner, 2026-11-04: "make it
    /// the size of re:com gravity"), its own growth (bit 7) included: Re:CoM grows and draws the
    /// sphere by its own rule, with the step [`growth_step_m`] (0.115.0 .. 0.120.0 dropped bit 7 and
    /// wrote KH1's scale each frame).
    pub fn sphere_flags(recom: u64) -> u64 {
        recom | RECOM_SELF_GROWTH
    }

    /// Re:CoM's sphere radius at effect scale 1: its Gravity case's `desc+0x78` = the row's `+4` ×
    /// 0.01, 1.5 m on all three rows 0x26 / 0x38 / 0x39 (`0x140222B1F`). Checked (exe + disasm).
    pub const RECOM_DRAW_RADIUS: f32 = 1.5;

    /// Re:CoM's own Gravity growth step per update (its case's `desc+0x1C` = the row's `+5` × 0.01,
    /// 0.04 / 0.06 / 0.08 m; effect scale + 0.027 / 0.04 / 0.053 an update). Checked (exe +
    /// disasm `0x140222B33`). Kept for the notes and tests: how long Re:CoM's own sphere grows (its
    /// flight ends at its mover's floor contact or after 8 m) is not settled (`recom-m7-magic.md`
    /// § 7.8, The sphere's size (0.121.0)).
    pub const RECOM_STEP: [f32; 3] = [0.04, 0.06, 0.08];

    /// Re:CoM's updates of growth the sphere has at the impact: its init's 2 steps (the init sets
    /// `g`, then one growth) plus one per update for the N fall frames. **Assumption**: one bullet
    /// update per 60 Hz frame, the first on the frame after the release (±1 step).
    pub fn steps_at_impact(level: u8) -> f32 {
        of(level).1 + 2.0
    }

    /// The KH1 sphere's growth step (its descriptor's `+0x1C`, metres an update): KH1's linear
    /// growth (`xm_gravity01.mag` `0x44..`: 0.01 → S over the N fall frames) on Re:CoM's linear step,
    /// ending at **KH1's hit radius** (1.7 × S m) at the impact. Re:CoM's rule then draws it at
    /// that radius ÷ 1.5 m.
    pub fn growth_step_m(level: u8) -> f32 {
        impact_radius_m(level) / steps_at_impact(level)
    }

    /// The scale Re:CoM draws the KH1 sphere at after `updates` updates of its growth (`updates`
    /// counts the init's two): `updates` × the step ÷ 1.5 m.
    pub fn recom_effect_scale(level: u8, updates: f32) -> f32 {
        updates * growth_step_m(level) / RECOM_DRAW_RADIUS
    }

    /// The drawn sphere against KH1's own scale (Re:CoM's rule on KH1's radius): 1.7 m / 1.5 m =
    /// 1.133 on every level (the `grav_all` sphere programs 3 / 4 / 5 carry KH1's sphere programs:
    /// Checked (files), so at equal scale both draw the same sphere).
    pub fn draw_factor() -> f32 {
        RADIUS / 100.0 / RECOM_DRAW_RADIUS
    }

    /// The effect scale at the impact: KH1's final scale S × [`draw_factor`] (1.133 / 1.7 / 1.813).
    pub fn effect_scale_at_impact(level: u8) -> f32 {
        recom_effect_scale(level, steps_at_impact(level))
    }

    /// What 0.103.0 .. 0.112.0 drew (the owner's "really small"): Re:CoM's self-growth step
    /// (`desc+0x1C` = the row's `+5` × 0.01 over `desc+0x78` = KH1's impact radius) taken as the
    /// spawn scale, × KH1's scale, plus one more Re:CoM step after our write. `row5` is Re:CoM's
    /// row byte `+5` (4 / 6 / 8).
    pub fn old_effect_scale(level: u8, row5: u8, t: f32) -> f32 {
        let g = row5 as f32 * 0.01 / impact_radius_m(level);
        g * scale_at(level, t) + g
    }
}

/// What KH1 does to an enemy under the **falling** Gravity sphere (decision 38's "slow"; research
/// for M7 f2, Checked (files + decomp)):
///
/// - the falling thread (`xm_gravity01.mag` `L000ee`, each fall frame after moving the sphere down
///   and growing it) calls `L0055c`: every enemy (sys 65) whose bodies touch a **sphere of 170 cm ×
///   the current scale** at the sphere's centre (sys 90 type 1) gets `actor+0x88 |= 4`;
/// - the actor's update `0x140297FE0` (its normal branch, only while not stopped: `+0x88 & 1`
///   clear, and not in the flattened motions 0x66..0x68, and its param block's byte 0 bit 0 clear)
///   then eases **`+0x480`** and clears the bit: with the bit, `s = (s − 0.8) × 0.95 + 0.8 −
///   sin(φ) × 0.05` and `φ += π/2` (wrapped, `0x1402A8280`); without it, `s += 0.05` up to 1 and
///   `φ = 0`. It starts at 1 (`0x140292660`: `+0x480 = 1.0`);
/// - **`+0x480` is no time scale.** Its only reader is the actor's matrix build `0x1402A0A40`:
///   the actor's scale (`+0x40`) with **y × `+0x480`**, applied to the world matrix
///   (`0x1402A9090`). The bit 4 has no other reader (`0x140297FE0` only). So KH1's "slow" is a
///   **vertical squash** of the model to about 0.8 with a 4-frame wobble, springing back at 0.05 a
///   frame once out from under the sphere. Nothing of the enemy's motion, think or speed changes.
///   (The research note's "time scale eased to 0.8" was an Assumption; this corrects it.)
pub mod squash {
    /// The squashed height, the ease (kept 0.95 a frame), the wobble and its step, the spring back.
    pub const LOW: f32 = 0.8;
    pub const KEEP: f32 = 0.95;
    pub const WOBBLE: f32 = 0.05;
    pub const PHASE_STEP: f32 = std::f32::consts::FRAC_PI_2;
    pub const BACK: f32 = 0.05;

    /// One actor's `+0x480` (height scale) and `+0x484` (wobble phase).
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Squash {
        pub y: f32,
        pub phase: f32,
    }

    impl Default for Squash {
        fn default() -> Squash {
            Squash { y: 1.0, phase: 0.0 }
        }
    }

    impl Squash {
        /// One actor update (`0x140297FE0`), with or without the sphere's bit from the last run.
        /// The height scale now.
        pub fn step(&mut self, under: bool) -> f32 {
            if under {
                self.y = ((self.y - LOW) * KEEP + LOW) - self.phase.sin() * WOBBLE;
                self.phase = kh1_core::math::wrap_angle(self.phase + PHASE_STEP);
            } else {
                if self.y < 1.0 {
                    self.y += BACK;
                    if self.y > 1.0 {
                        self.y = 1.0;
                    }
                }
                self.phase = 0.0;
            }
            self.y
        }

        /// Back at full height (nothing left to do).
        pub fn done(&self) -> bool {
            self.y >= 1.0
        }
    }

    /// The test sphere's centre `fallen` frames into the fall (Re:CoM metres, y up): KH1's sphere
    /// falls straight down 230 cm over its N frames to the impact centre.
    pub fn centre_at(impact: [f32; 3], level: u8, fallen: f32) -> [f32; 3] {
        let (_, n) = super::gravity::of(level);
        let left = (1.0 - fallen / n).clamp(0.0, 1.0);
        [impact[0], impact[1] + super::gravity::FALL / 100.0 * left, impact[2]]
    }

    /// The test sphere's radius then (170 cm × the scale; Re:CoM metres).
    pub fn radius_at(level: u8, fallen: f32) -> f32 {
        super::gravity::RADIUS * super::gravity::scale_at(level, fallen) / 100.0
    }
}

/// A Gravity-flattened enemy stays hittable at Combat = KH1 (owner, 2026-11-04: "when the enemy is
/// flattened from gravity, sora is unable to attack them"; `win::magic::flatten_begin` / `flatten_end`).
///
/// - **Re:CoM** (Checked (decomp + exe)): Re:CoM's Gravity (kind 0xD: our KH1 Gravity at Damage =
///   Re:CoM, Re:CoM's Gravity sleights) sends the enemy to its flatten state `+0xAC0` (the reaction
///   picker `0x14017D260` case 0xD). Its begin `0x14017C350` turns **every hurt body off**
///   (`0x1401A2D70(enemy, −1)`) and every attack body off; its exe `0x140183BA0` squashes the height
///   to the timer ÷ 250 (≥ 0.1), holds 9000 ticks (3 s), then sets 1.0; its end `0x14017EB80` turns
///   the bodies back on. So in Re:CoM nothing can hit a flattened enemy for about 3 s.
/// - **KH1** (Checked (decomp)): its hit pass `0x1402B7260` runs the flatten (movement states 0x16 /
///   0x17, request 0x66: `+0x480` held at 0.1, eased back over 15 frames, then request 0x67) and then
///   its can-be-hit test `0x140292070` (actor flags `& 0x8048 == 0x8000` and an unmasked body; the
///   mask `+0x3A8` is set only by the AI scripts' syscalls `0x1402BAA00` / `0x1402BAA40`): nothing in
///   the flatten makes the actor unhittable. A flinch on a flattened actor (`0x1402B8E10` kind 3:
///   `0x1402B1B10` is true for movement 0x16) plays `0x1402B8B90`'s knock (request 0x4E / 0x4F,
///   movement 6), out of the flatten, and the actor's update `0x140297FE0` springs `+0x480` back
///   **+0.05 a frame** (outside requests 0x66..0x68).
/// - **Ours**: after Re:CoM's begin, Re:CoM's own hurt-bodies-on [`HURT_ON_RVA`] (the attack bodies
///   stay off: Re:CoM's); a hit then goes through Re:CoM's hit handler as on any enemy (damage, its
///   reaction ends the flatten). When the flatten ends before its hold (a hit's reaction took over),
///   Re:CoM leaves the height squashed: KH1's spring-back from there ([`spring_back`]). At Combat =
///   Re:CoM, or outside a KH1 battle, Re:CoM's rule.
pub mod flatten {
    /// `0x1401A2DB0(enemy, −1)`: every hurt body on (Re:CoM's flatten end calls it). Checked (exe).
    pub const HURT_ON_RVA: u32 = 0x001A_2DB0;
    /// Re:CoM's height after its own flatten (`0x140183BA0` phase 1: y = 1.0). Checked (decomp).
    pub const RECOM_HEIGHT: f32 = 1.0;

    /// The squash to run when Re:CoM's flatten ends with the height at `y` (Re:CoM's scale y): KH1's
    /// spring-back (+0.05 a frame, `super::squash::Squash::step(false)`) from `y`, or none when
    /// Re:CoM's own end already stood it up.
    pub fn spring_back(y: f32) -> Option<super::squash::Squash> {
        (y.is_finite() && y > 0.0 && y < RECOM_HEIGHT - 1e-3).then_some(super::squash::Squash { y: y / RECOM_HEIGHT, phase: 0.0 })
    }

    /// KH1's own hit flatten at Damage = KH1 (owner, 2026-11-04: "2 yes"; `win::magic::kh1_flatten_hit`
    /// / `flatten_exe`). KH1 (Checked (decomp)):
    ///
    /// - a hit of reaction kind 8 (KH1 Gravity's, btltbl kind 8) runs `0x1402B7DE0` from the reaction
    ///   pass `0x1402B8E10`: on the ground (`0x1402B1AE0` and `0x1402B1B10` false) request **0x66**
    ///   and timer `+0x404` = **68**; in the air or a special movement, movement **0x16** (ground) /
    ///   **0x17** (air) (`0x1402B1960`), request **0x68**, timer **75**;
    /// - each frame the hit pass `0x1402B7260` counts the timer down by the frame step and sets the
    ///   height `+0x480`: above 60 the squash, **0.1 + 0.9 × (t − 60) / 8** (request 0x66: `× 0.125`) or
    ///   **/ 15** (movement 0x16 / 0x17); at or under 60 **0.1**; at 0 request **0x67** (the get-up)
    ///   (and movement 0 / 4); while 0x67 plays, **h += (1 − h) × 0.3** a frame; after it, the actor
    ///   update `0x140297FE0` springs h **+0.05** a frame to 1 ([`spring_back`]);
    /// - nothing makes it unhittable; a flinch knocks it out (0.117.0).
    ///
    /// Re:CoM: its own flatten state `+0xAC0` (the reaction picker's case 0xD) owns the enemy: begin
    /// `0x14017C350` (bodies off, then 0.117.0's hurt bodies on), exe `0x140183BA0` (phase `+0x270`:
    /// 0 squash, 1 hold until its timer `+0x160` runs out, then height 1.0 and its get-up motion, 2
    /// until that motion ends), end `0x14017EB80`. Ours: KH1's numbers in its phases ([`Step`]).
    pub mod kh1 {
        /// Re:CoM's kind for the picker: its Gravity flatten (`0x14017D260` case 0xD → `+0xAC0`).
        pub const PICK_KIND: i32 = 0xD;
        /// Re:CoM's flatten state's offset in the enemy (state 24: (24 + 0x13) × 0x40).
        pub const STATE: usize = 0xAC0;
        /// KH1's held height, the hold's end (frames left), the get-up ease.
        pub const LOW: f32 = 0.1;
        pub const HOLD: f32 = 60.0;
        pub const GETUP: f32 = 0.3;
        /// Re:CoM ticks a 60 Hz frame (`recom-timing.md`, Checked (decomp)); KH1's frame step is 1.0.
        pub const TICKS: f32 = 50.0;

        /// Which of KH1's two flattens.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Variant {
            /// Request 0x66: timer 68, squash over 8 frames.
            Ground,
            /// Movement 0x16 / 0x17, request 0x68: timer 75, squash over 15 frames.
            Air,
        }

        impl Variant {
            /// KH1's start timer (frames).
            pub fn start(self) -> f32 {
                match self {
                    Variant::Ground => 68.0,
                    Variant::Air => 75.0,
                }
            }
            /// The squash's length (frames above the hold).
            pub fn squash(self) -> f32 {
                match self {
                    Variant::Ground => 8.0,
                    Variant::Air => 15.0,
                }
            }
            /// KH1's variant: on Re:CoM's floor (its mover's `+0x120 & 5`) and not a flier (`+0x260`
            /// bit 8) is the ground one. **Assumption**: Re:CoM's "off the floor" stands for KH1's
            /// air and special movements (`0x1402B1AE0` / `0x1402B1B10`).
            pub fn of(on_floor: bool, flier: bool) -> Variant {
                if on_floor && !flier {
                    Variant::Ground
                } else {
                    Variant::Air
                }
            }
        }

        /// Re:CoM's state timer at its begin: KH1's start (KH1's hit frame sets 68 at full height; the
        /// next frame's pass reads 67). Re:CoM's actor update `0x1401A59A0` counts the timer, runs the
        /// hit handler (its picker sets the next state), then the state step `0x140242F50`, which
        /// switches (end, begin) and runs the new state's exe in the same call: begin and the first exe
        /// on the hit's frame, the next frame's count first. Checked (decomp); the handler's place in
        /// the update: as `win::magic::held_hit_pace` (Checked (decomp + disasm)).
        pub fn begin_ticks(v: Variant) -> i32 {
            (v.start() * TICKS) as i32
        }

        /// The height `t` frames left (KH1's `+0x404` after this frame's count), while above the hold.
        pub fn squash_height(v: Variant, t: f32) -> Option<f32> {
            (t > HOLD).then(|| LOW + (1.0 - LOW) * ((t - HOLD) / v.squash()))
        }

        /// One frame of the get-up (request 0x67).
        pub fn getup(h: f32) -> f32 {
            h + (1.0 - h) * GETUP
        }

        /// What the KH1 flatten does in one of Re:CoM's flatten-exe calls.
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum Step {
            /// Phase 0, above the hold: this height; Re:CoM's phase-0 code not run.
            Squash(f32),
            /// Phase 0, the hold reached: height 0.1 and Re:CoM's phase 1 (its timer runs on: KH1's).
            Hold,
            /// Phase 1: Re:CoM's (it waits out the timer, then pops to 1.0 and plays its get-up); if it
            /// went to phase 2, height back to 0.1 (KH1's request 0x67 is only queued that frame).
            Wait,
            /// Phase 2 (Re:CoM's get-up motion): Re:CoM's, then the height one get-up frame on.
            GetUp,
        }

        /// The step for Re:CoM's phase and timer (ticks).
        pub fn step(v: Variant, phase: i32, timer: i32) -> Step {
            match phase {
                0 => match squash_height(v, timer as f32 / TICKS) {
                    Some(h) => Step::Squash(h),
                    None => Step::Hold,
                },
                1 => Step::Wait,
                _ => Step::GetUp,
            }
        }
    }
}

/// The falling thread's clock (`xm_gravity01.mag` `0x44..`): the release's run counts the first
/// frame; each later run counts one more until the N-th; the run after it is the impact.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fall {
    pub left: f32,
    pub total: f32,
}

impl Fall {
    /// The release frame's run (the first of the fall).
    pub fn new(level: u8) -> Fall {
        let n = gravity::of(level).1;
        Fall { left: n - 1.0, total: n }
    }

    /// One later run: true when it is the impact (the fall's frames are over).
    pub fn run(&mut self, step: f32) -> bool {
        if self.left <= 0.0 {
            return true;
        }
        self.left -= step;
        false
    }

    /// Frames fallen so far.
    pub fn fallen(&self) -> f32 {
        self.total - self.left.max(0.0)
    }
}

/// KH1's Gravity attack data (btltbl `0x6468` / `0x64D8` / `0x6548`, Checked (files)): power 40 / 55
/// / 70 + **max MP × 5** (stat source 5), element 8 (the % of HP path), kind 8, struck directly.
pub fn gravity_attack(level: u8) -> AttackData {
    let power = [40, 55, 70][level.clamp(1, 3) as usize - 1];
    AttackData {
        stat_source: 5,
        element: 8,
        kind_party: 8,
        kind: 8,
        bone: -1,
        power_party: power,
        mp_charge: 0,
        clash: 6,
        priority: 0,
        recoil_request: 110,
        power,
        radius: 0.0,
        ..Default::default()
    }
}

/// Re:CoM's Gravity rows 0x26 / 0x38 / 0x39 as in the Steam exe (clip 0x3A, effect sets 18 / 19 /
/// 20 of `grav_all`, release `+3` 32, its sphere's radius 1.5 m, speed 0.05, kind 0xD). Checked
/// (exe).
pub const RECOM_GRAVITY_ROWS: [[u8; 16]; 3] = [
    [0x3A, 0x00, 0x12, 0x20, 0x96, 0x04, 0x00, 0x00, 0x00, 0x05, 0x00, 0x0D, 0x08, 0x0A, 0x32, 0x00],
    [0x3A, 0x00, 0x13, 0x20, 0x96, 0x06, 0x00, 0x00, 0x00, 0x05, 0x00, 0x0D, 0x08, 0x14, 0x32, 0x00],
    [0x3A, 0x00, 0x14, 0x20, 0x96, 0x08, 0x00, 0x00, 0x00, 0x05, 0x00, 0x0D, 0x08, 0x1E, 0x32, 0x00],
];

/// The KH1 Gravity row: Re:CoM's row (clip, blend, `grav_all` set, its sphere's `+5` / `+0xD`:
/// decision 34) with `+3` = 0, **`+0xB` = 1** (Re:CoM's flinch: Re:CoM's kind 0xD is its own %-of-HP
/// damage with an integer % and a 3-second squash, `0x14017DC50` / state `+0xAC0`; KH1's number
/// is written as the hit's value instead) and `+0xF` = 0x80. At Damage = Re:CoM in a battle room
/// the cast writes Re:CoM's kind 0xD into `+0x668` instead (`crate::damage::recom_kind`), so the
/// sphere's hit is Re:CoM's own Gravity.
pub fn gravity_row_bytes(recom: &[u8; 16]) -> [u8; 16] {
    let mut b = *recom;
    b[3] = 0;
    b[0xB] = 1;
    b[0xF] |= FLAG_KH1;
    b
}

/// KH1's Gravity on an enemy (`0x1402BFA80` element 8): current HP × (power + 5 × max MP)% ×
/// resist[8], at least the floor (1 when the floor is under 2).
pub fn gravity_damage(level: u8, max_mp: i32, hp: i32, resist8: f32, floor: u16) -> i32 {
    let def = DefenderStats { hp, max_hp: hp, min_damage: floor, hp_percent_scale: resist8, ..Default::default() };
    spell_damage(&gravity_attack(level), max_mp, &def, &mut Kh1Rand::new(1))
}

/// KH1's Stop (`xm_stop01..03.mag`, decoded with `magdis.py`: one script, the level passes the
/// radius 250 / 350 / 500; Checked (files), the syscalls in the KH1 decomp):
///
/// - the command (`L00890`): **wait 4**, the charge effect, then **each frame for 24 frames** the
///   release check (`0x3B` / `0x88`; else the effect fades and it ends: no spell, no MP); so the
///   release is at **28**; the thread; **the MP** (sys 47, 200 = two points);
/// - the thread (`0x44..0x206`): the centre is the target's part if alive, else the caster's
///   position; every enemy (up to 30) sorted nearest first from the centre (`L0040e`, the same
///   selection sort as Thunder's); down the list, an enemy whose bodies touch a vertical cylinder of
///   **250 / 350 / 500 cm, ±300 cm** (sys 90 type 2) gets the Stop effect and **one direct strike**
///   (sys 60), then **4 frames** (`L009a2(4)`); one outside is passed over at once.
///
/// The strike (`0x1402BEEE0` element 9): if the enemy's `resist[9]` > 0, it is stopped for
/// **(2 + max MP) × 60 × resist[9] frames** (power 2 + max MP, stat source 1; the same at every
/// level); `resist[9]` = 0: immune. While stopped KH1 queues the hits it takes and lands them when
/// Stop ends, and the stopped actor's own attacks are skipped (`0x140297FE0`, `0x1402B47A0`).
/// Checked (files + decomp).
pub mod stop {
    pub const WAIT: f32 = 28.0;
    /// The area: radius by level, half-height, cm.
    pub const RADIUS: [f32; 3] = [250.0, 350.0, 500.0];
    pub const HALF_HEIGHT: f32 = 300.0;
    /// The wait after each strike, frames.
    pub const INTERVAL: f32 = 4.0;
    /// The power (data `+0x48`, all levels) and the frames per point.
    pub const POWER: i32 = 2;
    pub const FRAMES_PER_POINT: f32 = 60.0;
    /// MP cost (data `+0x30` = 200): two whole points.
    pub const COST: i32 = crate::mp::cost::STOP;

    /// The area of a level (Re:CoM metres).
    pub fn area(level: u8) -> super::Area {
        super::Area::Cylinder { radius: RADIUS[level.clamp(1, 3) as usize - 1] / 100.0, half: HALF_HEIGHT / 100.0 }
    }

    /// KH1's Stop duration in frames for a `resist[9]` (0 = immune: None).
    pub fn frames(max_mp: i32, resist9: f32) -> Option<f32> {
        (resist9 > 0.0).then(|| (POWER + max_mp) as f32 * FRAMES_PER_POINT * resist9)
    }

    /// The duration as Re:CoM's stop timer (`enemy+0x2D8` → the stop state's `+0x160`, both i32
    /// ticks: 50 a frame; Checked (disasm `0x14017DCC6`, `0x14017C7F4`)).
    pub fn ticks(max_mp: i32, resist9: f32) -> Option<i32> {
        frames(max_mp, resist9).map(|f| (f * crate::units::UNITS_PER_FRAME).round().max(1.0) as i32)
    }

    /// The hit's value that carries the duration through Re:CoM's hit (the part's `+0x92` is an
    /// i16, read by `0x14016FDB0`): at most 32767 ticks (655 frames). Longer durations (max MP 10
    /// and up: (2 + 10) × 60 = 720 frames) are written whole into `+0x2D8` once Re:CoM's handler
    /// has sent the enemy to its stop state ([`timer_after_entry`]).
    pub fn carrier(ticks: i32) -> i16 {
        ticks.clamp(1, i16::MAX as i32) as i16
    }

    /// What our hit writes into Re:CoM's `+0x2D8` after its handler: KH1's whole duration, only
    /// when the handler sent the enemy to its stop state (`+0x68` = `+0x940`) and the carrier
    /// could not hold it. None: Re:CoM's own write stands.
    pub fn timer_after_entry(entered: bool, ticks: i32) -> Option<i32> {
        (entered && ticks > i16::MAX as i32).then_some(ticks)
    }

    /// Decision 14 for Stop: KH1's Stop lands on an attacking actor (`0x1402BEEE0` element 9 sets
    /// `+0x88 |= 1` whatever the actor does), so the strike carries Re:CoM's card-interrupt bit
    /// (`+0x260 |= 0x2000000`) when the enemy is in an attack (`+0x260 & 1`) and the stop isn't
    /// refused (an immune enemy's attack goes on, as KH1's). Re:CoM's hit handler `0x14017F7E0`
    /// otherwise returns at once for an enemy with bit 0 (`0x14017F8CB`), before its kind 8.
    pub fn interrupts(flags: u64, ticks: Option<i32>) -> bool {
        ticks.is_some() && flags & 1 != 0 && flags & 0x200_0000 == 0
    }

    /// KH1's pace for the hits held during Stop (decision 38; M7 f3). Checked (decomp):
    ///
    /// - a hit on a stopped actor makes a record from the third pool of the record builder
    ///   `0x1402BF630` (`+0x88 & 1`); **when the actor's queue already holds a record**, the new one
    ///   gets a delay in its `+0xC` bits 12–15 of `6 − (int)(r × −6)` frames, `r` = `0x14031BA30()`
    ///   (24 random bits / 2²⁴), so **6 to 11 frames, each equally likely**; the first record of an
    ///   empty queue has none;
    /// - when Stop ends, the hit pass `0x1402B8E10` pops the queue every frame (`0x1402BF160`): a
    ///   head with a delay loses one and stops the pop for that frame; a head with none is applied
    ///   (damage and reaction) and the next one is looked at in the same frame.
    ///
    /// So the first held hit lands on the first frame after Stop, and each next one **its own 6–11
    /// frames after the one before**. No cap (the pool has 128 records for everyone). Re:CoM holds at
    /// most 8 (kept: decision 38) and spaces them by half its reaction motion (or 750 ticks).
    pub fn held_gap(r: f32) -> f32 {
        6.0 - (r * -6.0).trunc()
    }

    /// The frames between two landings that Re:CoM's order adds to the per-landing timer: the actor
    /// update (`0x1401A59A0`) counts `+0x160` down by its dt **before** the hit handler (our wrapper
    /// sets the timer there, the frame after a landing) and the state exe; the stop exe's phase 2
    /// (`0x14018499D`) goes back to phase 1 when the timer is ≤ 0, and phase 1 lands the next hit on
    /// the frame after. Checked (decomp + disasm).
    pub const LANDING_OVERHEAD: f32 = 2.0;

    /// The per-landing timer (ticks) our wrapper writes the frame after a landing so that Re:CoM's
    /// stop exe lands the next held hit `gap` KH1 frames (60 Hz, 50 ticks each) after it, with the
    /// enemy's dt (ticks a game frame; the overhead is in game frames).
    pub fn landing_timer(gap: f32, dt: i32) -> i32 {
        (gap * crate::units::UNITS_PER_FRAME - LANDING_OVERHEAD * dt.max(1) as f32).max(0.0).round() as i32
    }
}

/// KH1's Stop attack data (btltbl `0x65B8` / `0x6628` / `0x6698`, Checked (files)): power 2 + max MP
/// (stat source 1), element 9 (the damage returns the power: the duration's points), kind 0.
pub fn stop_attack(_level: u8) -> AttackData {
    AttackData {
        stat_source: 1,
        element: 9,
        kind_party: 0,
        kind: 0,
        bone: -1,
        power_party: stop::POWER,
        mp_charge: 0,
        clash: 6,
        priority: 0,
        recoil_request: 110,
        power: stop::POWER,
        radius: 0.0,
        ..Default::default()
    }
}

/// Re:CoM's Stop rows 0x27 / 0x3A / 0x3B as in the Steam exe (clip 0x39, effect sets 21 / 22 / 23 of
/// `stop_all`, kind 8, its shot's radius / range / life). Checked (exe).
pub const RECOM_STOP_ROWS: [[u8; 16]; 3] = [
    [0x39, 0x00, 0x15, 0x1E, 0x05, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x08, 0x08, 0x00, 0x08, 0x00],
    [0x39, 0x00, 0x16, 0x1E, 0x14, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x08, 0x08, 0x00, 0x08, 0x00],
    [0x39, 0x00, 0x17, 0x1E, 0x28, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x08, 0x08, 0x00, 0x08, 0x00],
];

/// The KH1 Stop row: Re:CoM's row (clip, `stop_all` set, its shot: decision 34) with `+3` = 0 and
/// `+0xF` = 0x80. **`+0xB` stays 8**: Re:CoM's own stop kind, which puts the enemy in Re:CoM's stop
/// state (`0x14017D260` case 8 → `+0x940`) for the hit's value in ticks (`0x14017DC50` case 8).
pub fn stop_row_bytes(recom: &[u8; 16]) -> [u8; 16] {
    let mut b = *recom;
    b[3] = 0;
    b[0xF] |= FLAG_KH1;
    b
}

/// Re:CoM's stop kind (`0x14017DC50` case 8 / `0x14017D260` case 8). Checked (decomp).
pub const RECOM_STOP_KIND: u32 = 8;

/// Re:CoM's field Heartless (`CMapEnemy`, vtable `0x14055ACB0`): its type getter (`+0x28`,
/// `0x14019D050`) returns 0x1002; a battle enemy's is [`crate::battle::ENEMY_TYPE`] 0x1004. Map
/// enemies exist only in field rooms (the field's objects are destroyed before the battle room
/// loads: `docs/recom-battle-entry.md`). Checked (exe + decomp).
pub const MAP_ENEMY_TYPE: u32 = 0x1002;

/// The kind a KH1 Thunder / Stop strike carries onto a **field Heartless** (owner, 2026-11-04,
/// "Sleights outside battle": "KH1 Thunder and Stop in the field get Re:CoM's paralyse / freeze too,
/// instead of starting the battle at once"): its hit handler `0x14019E020` picks its reaction by the
/// hit's kind (`body+0x80`): Re:CoM's Thunder rows' kinds 0x1C / 0x1D / 0x1E (by level: row 0x24,
/// 0x2E, 0x2F `+0xB`) → state 5 (paralysed, no battle; a later contact: Re:CoM's halved HP); Stop's 8
/// → state 6 (frozen, no battle; a later contact: the enemy starts the battle stopped 4 s). Any
/// other spell keeps its KH1 row's kind (1: Re:CoM's field strike, the battle with first strike).
/// Battle enemies never get this (their kinds stay the KH1 rows'). Checked (decomp + exe rows).
pub fn field_kind(s: u8, level: u8, kh1_kind: u32) -> u32 {
    match s {
        spell::THUNDER => RECOM_THUNDER_ROWS[level.clamp(1, 3) as usize - 1][0xB] as u32,
        spell::STOP => RECOM_STOP_KIND,
        _ => kh1_kind,
    }
}

/// The kinds Re:CoM's field Heartless hit handler `0x14019E020` turns into something other than its
/// plain field strike (its switch on the hit's `+0x80`): 7 / 0x2A → state `+0x440`, 8 → `+0x400`
/// (frozen), 0x1C..0x1E / 0x24..0x26 → `+0x3C0` (paralysed), 0x29 → nothing. Every other kind →
/// `+0x380`, the struck state (the battle starts with first strike). Checked (decomp).
pub const FIELD_SPECIAL_KINDS: [u32; 10] = [7, 0x2A, 8, 0x1C, 0x1D, 0x1E, 0x24, 0x25, 0x26, 0x29];

/// Whether a kind is Re:CoM's plain field strike on a field Heartless.
pub fn field_plain_strike(kind: u32) -> bool {
    !FIELD_SPECIAL_KINDS.contains(&kind)
}

/// KH1's Aero (`xm_air01.mag`, one file for every level; the levels differ in their data: decoded
/// with `magdis.py`, Checked (files), the syscalls in the KH1 decomp):
///
/// - the command (`0x4C4..0x674`): the charge effect; **up to 30 frames while the request is 60 /
///   137** (`0x3C` / `0x89`): **if the motion changes, it casts at once** (no cancel); the cast aura;
///   if the target (the command's member, else the caster) and the caster are alive and healable
///   (`0x8000`): the barrier data struck onto the target (sys 60), the barrier thread, **the MP**
///   (sys 47, 200), and (if it had no barrier) the lasting barrier effect after 24 frames;
/// - the strike (`0x1402BEEE0` element 12): `+0x88 |= 8` and the party block's timer = **(18 + max
///   MP) × 60 frames** (power 18 + max MP, stat source 1, the same at every level), counted down by
///   the frame step (`0x1402A43A0`); **while it is on, damage taken is halved** (`0x1402BFA80`: the
///   value after the resistance, ÷ 2, then at least 1);
/// - the barrier thread (`0x44..0x1DE`): its effect on the target; for **Aerora / Aeroga** (data 2,
///   btltbl `0x5CF8` / `0x5D68`) an attack owned by the target, **radius 80 × the target's size**,
///   always active, **re-hit every 30 frames** (`+0x50`), following the body centre (bone 32771)
///   **while the barrier lasts**; power 8 + max MP × 2 (stat source 3), element 0, kind 3.
pub mod aero {
    /// The longest wait before the cast (if the motion keeps playing).
    pub const WAIT: f32 = 30.0;
    /// The barrier: power 18 + max MP, × 60 frames.
    pub const POWER: i32 = 18;
    pub const FRAMES_PER_POINT: f32 = 60.0;
    /// The wind (Aerora / Aeroga): radius at size 1, cm; its re-hit interval, frames.
    pub const WIND_RADIUS: f32 = 80.0;
    pub const REHIT: f32 = 30.0;
    /// MP cost (data `+0x30` = 200): two whole points.
    pub const COST: i32 = crate::mp::cost::AERO;

    /// The barrier's frames.
    pub fn frames(max_mp: i32) -> f32 {
        (POWER + max_mp) as f32 * FRAMES_PER_POINT
    }

    /// Whether a level has the wind (data 2: Aerora and Aeroga).
    pub fn has_wind(level: u8) -> bool {
        level >= 2
    }

    /// KH1's halving under the barrier (`0x1402BFA80`: `v / 2`, then at least 1) on Re:CoM's hit
    /// value for Sora; 0 stays 0.
    pub fn halve(v: i16) -> i16 {
        if v <= 0 {
            v
        } else {
            (v / 2).max(1)
        }
    }

    /// The barrier's countdown (`0x1402A43A0`: − the frame step; off at ≤ 0); a re-cast rewrites it.
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub struct Barrier {
        pub left: f32,
        /// Frames since the last re-hit of the wind.
        pub rehit: f32,
    }

    impl Barrier {
        pub fn new(max_mp: i32) -> Barrier {
            Barrier { left: frames(max_mp), rehit: 0.0 }
        }

        /// One frame: (still on, the wind may hit again now).
        pub fn step(&mut self, step: f32) -> (bool, bool) {
            self.left -= step;
            self.rehit += step;
            let again = self.rehit >= REHIT;
            if again {
                self.rehit -= REHIT;
            }
            (self.left > 0.0, again)
        }
    }
}

/// KH1's Aerora / Aeroga wind data (btltbl `0x5CF8` / `0x5D68`, the same numbers; Checked (files)):
/// power 8 + **max MP × 2** (stat source 3), element 0, kind 3, radius 80, re-hit 30 frames.
pub fn aero_wind_attack(_level: u8) -> AttackData {
    AttackData {
        stat_source: 3,
        element: 0,
        kind_party: 3,
        kind: 3,
        bone: -1,
        power_party: 8,
        mp_charge: 0,
        clash: 0x1_0005,
        priority: 99,
        recoil_request: 110,
        power: 8,
        radius: aero::WIND_RADIUS,
        ..Default::default()
    }
}

/// Re:CoM's Aero rows 0x28 / 0x3C / 0x3D as in the Steam exe (clip 0x39, effect sets 24 / 25 / 26 of
/// `airo_all`, its wind shot: radius `+4` × 0.1, life `+0xE`). Checked (exe).
pub const RECOM_AERO_ROWS: [[u8; 16]; 3] = [
    [0x39, 0x00, 0x18, 0x1E, 0x0F, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0x08, 0x00, 0x0A, 0x00],
    [0x39, 0x00, 0x19, 0x1E, 0x14, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x02, 0x08, 0x00, 0x0A, 0x00],
    [0x39, 0x00, 0x1A, 0x1E, 0x1E, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x03, 0x08, 0x00, 0x0A, 0x00],
];

/// The KH1 Aero row: Re:CoM's row (clip, `airo_all` set: decision 34) with `+3` = 0, `+0xB` = 1 (the
/// wind's KH1 kind 3, flinch) and `+0xF` = 0x80.
pub fn aero_row_bytes(recom: &[u8; 16]) -> [u8; 16] {
    let mut b = *recom;
    b[3] = 0;
    b[0xB] = 1;
    b[0xF] |= FLAG_KH1;
    b
}

/// An area KH1's sys 90 tests a body against (Re:CoM metres): a vertical cylinder (type 2: radius,
/// half-height) or a sphere (type 1: radius).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Area {
    Cylinder { radius: f32, half: f32 },
    Sphere { radius: f32 },
}

/// The distance from a point to an area around `centre` (0 inside).
fn area_distance(p: [f32; 3], centre: [f32; 3], a: Area) -> f32 {
    match a {
        Area::Cylinder { radius, half } => cylinder_distance(p, centre, radius, half),
        Area::Sphere { radius } => {
            let d = [p[0] - centre[0], p[1] - centre[1], p[2] - centre[2]];
            ((d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt() - radius).max(0.0)
        }
    }
}

/// Whether a hurt body touches an area (its shape's nearest point within its radius).
pub fn body_touches(centre: [f32; 3], area: Area, body: &BodyShape) -> bool {
    match *body {
        BodyShape::Sphere { c, r } => area_distance(c, centre, area) <= r,
        BodyShape::Column { a, b, r } => {
            let at = |t: f32| area_distance([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t], centre, area);
            let (mut lo, mut hi) = (0.0f32, 1.0f32);
            for _ in 0..40 {
                let (m1, m2) = (lo + (hi - lo) / 3.0, hi - (hi - lo) / 3.0);
                if at(m1) <= at(m2) {
                    hi = m2;
                } else {
                    lo = m1;
                }
            }
            at(0.0).min(at(1.0)).min(at((lo + hi) * 0.5)) <= r
        }
    }
}

/// A Re:CoM hurt body's shape as the area test sees it (metres): a sphere (kind 2: centre `+0x20`,
/// its y + `+0x74`, radius `+0x70`) or a column (kinds 4 / 5: from `+0x10` to `+0x20`, each y +
/// `+0x74`, radius `+0x70`), as Re:CoM's own primitives `0x1401777E0` / `0x140176FA0` read them.
/// Checked (decomp).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BodyShape {
    Sphere { c: [f32; 3], r: f32 },
    Column { a: [f32; 3], b: [f32; 3], r: f32 },
}

/// The contact of a strike that Re:CoM's collision doesn't find itself (7.6's bolts, 7.8's impact,
/// 7.9's Stop: their attack is applied to one body directly), found by **Re:CoM's own overlap
/// test** as its collision pass `0x14016EAD0` finds it before every apply (battle-end soft-lock,
/// `docs/recom-m7-magic.md` § Strikes and Re:CoM's contact).
///
/// Re:CoM's pass picks the test by `body kind | attack kind << 4` (Checked (decomp + disasm
/// `0x14016EB9D..0x14016EC17`)); our strikes' attacks are spheres (kind 2, the shot descriptor's
/// `+0x80`, `0x140156790`), so 0x22 is the sphere test `0x1401777E0` and 0x24 / 0x25 the capsule
/// test `0x140176FA0(…, 0)`; other body kinds are never hit by Re:CoM's pass. A body whose shape
/// flags (`+0x64`) & 0xE are 6 is skipped by the pass. The test sweeps the attack sphere from its
/// `+0x10` to its `+0x20` and, on contact, writes the contact point to **both** shapes' `+0x50`
/// and the normal to `+0x40`. Re:CoM's defeat `0x14017BBC0` makes the death effect and every prize
/// of the enemy at the hit body's shape `+0x50`, which a shape is not given at its creation
/// (`0x1401720E0` fills `+0x10..+0x3C` only). Checked (decomp).
pub mod strike_contact {
    use super::BodyShape;

    /// The attack shape kind of our strikes' shots (a sphere).
    pub const ATTACK_SPHERE: u32 = 2;
    /// How far above the body's top the sweep starts (beyond both radii), metres.
    pub const MARGIN_M: f32 = 0.5;

    /// Re:CoM's overlap test for a body kind against a sphere attack.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Test {
        /// Key 0x22: `0x1401777E0` (the attack's points as they are).
        Sphere,
        /// Keys 0x24 / 0x25: `0x140176FA0` with 0 (each attack point's y + the attack's `+0x74`).
        Capsule,
    }

    /// The test Re:CoM's pass runs for this pair, or None (Re:CoM's pass doesn't hit this body).
    pub fn test_for(body_kind: u32, attack_kind: u32, body_flags: u32) -> Option<Test> {
        if body_flags & 0xE == 6 {
            return None;
        }
        match body_kind | attack_kind << 4 {
            0x22 => Some(Test::Sphere),
            0x24 | 0x25 => Some(Test::Capsule),
            _ => None,
        }
    }

    /// The attack's sweep for the strike, as the raw `+0x10` / `+0x20` to write: straight down from
    /// above the body's top (beyond both radii) to the body's centre, as a bolt falling onto it. The
    /// end lies inside the body, so Re:CoM's test always finds the contact (where the sweep enters
    /// the body). `body`: the shape as [`BodyShape`] (each y already + the body's `+0x74`).
    pub fn sweep(test: Test, body: &BodyShape, attack_radius: f32, attack_y_offset: f32) -> ([f32; 3], [f32; 3]) {
        let (centre, top) = match *body {
            BodyShape::Sphere { c, r } => (c, c[1] + r),
            BodyShape::Column { a, b, r } => ([(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5, (a[2] + b[2]) * 0.5], a[1].max(b[1]) + r),
        };
        let start = [centre[0], top + attack_radius.max(0.0) + MARGIN_M, centre[2]];
        let dy = match test {
            Test::Sphere => 0.0,
            Test::Capsule => attack_y_offset,
        };
        ([start[0], start[1] - dy, start[2]], [centre[0], centre[1] - dy, centre[2]])
    }

    /// Whether a point is a usable position (a non-finite one is what a never-written shape `+0x50`
    /// can hold).
    pub fn finite(p: [f32; 3]) -> bool {
        p.iter().all(|v| v.is_finite())
    }
}

/// The distance from a point to KH1's area cylinder (0 inside).
fn cylinder_distance(p: [f32; 3], centre: [f32; 3], radius: f32, half: f32) -> f32 {
    let (dx, dz) = (p[0] - centre[0], p[2] - centre[2]);
    let h = ((dx * dx + dz * dz).sqrt() - radius).max(0.0);
    let v = ((p[1] - centre[1]).abs() - half).max(0.0);
    (h * h + v * v).sqrt()
}

/// Whether a hurt body overlaps KH1's Thunder area (the vertical cylinder, Re:CoM metres): its
/// shape's nearest point to the cylinder within its radius. The distance from a segment's points
/// to a convex solid is convex along it, so a ternary search finds the column's nearest point.
pub fn body_in_area(centre: [f32; 3], body: &BodyShape) -> bool {
    body_touches(centre, Area::Cylinder { radius: thunder::RADIUS / 100.0, half: thunder::HALF_HEIGHT / 100.0 }, body)
}

/// KH1's strike list (Thunder's `L0069c`, Stop's `L0040e`): the first 30 enemies with their
/// distance from `from`, sorted nearest first by KH1's selection sort (a later entry replaces the
/// minimum only when strictly nearer, then swaps).
pub fn thunder_order(from: [f32; 3], enemies: impl IntoIterator<Item = (u32, [f32; 3])>) -> Vec<u32> {
    let mut l: Vec<(u32, f32)> = enemies
        .into_iter()
        .take(thunder::MAX)
        .map(|(id, p)| {
            let d = [p[0] - from[0], p[1] - from[1], p[2] - from[2]];
            (id, (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt())
        })
        .collect();
    for i in 0..l.len() {
        let mut m = i;
        for j in i + 1..l.len() {
            if l[j].1 - l[m].1 < 0.0 {
                m = j;
            }
        }
        if m != i {
            l.swap(i, m);
        }
    }
    l.into_iter().map(|e| e.0).collect()
}

/// The strike thread's state (KH1's script; Re:CoM has no place for it).
#[derive(Clone, Debug, PartialEq)]
pub struct Strikes {
    pub order: Vec<u32>,
    pub next: usize,
    /// `L00c8e(8)`'s countdown (Stop: `L009a2(4)`).
    pub wait: f32,
    pub struck: u32,
    /// The wait after each strike (Thunder 8, Stop 4).
    pub interval: f32,
}

impl Strikes {
    pub fn new(order: Vec<u32>) -> Strikes {
        Strikes::every(order, thunder::INTERVAL)
    }

    /// A strike list with its own wait after each strike.
    pub fn every(order: Vec<u32>, interval: f32) -> Strikes {
        Strikes { order, next: 0, wait: 0.0, struck: 0, interval }
    }

    /// One run of the thread: a run inside the wait counts it down; otherwise the next listed
    /// enemy `inside` the area is struck (returned) and the wait starts, counting this run (KH1's
    /// `L00c8e` decrements once before it yields); enemies outside are passed over in the same
    /// run. So strikes come 8 runs apart.
    pub fn run(&mut self, step: f32, mut inside: impl FnMut(u32) -> bool) -> Option<u32> {
        if self.wait > 0.0 {
            self.wait -= step;
            return None;
        }
        while let Some(&id) = self.order.get(self.next) {
            self.next += 1;
            if inside(id) {
                self.wait = self.interval - step;
                self.struck += 1;
                return Some(id);
            }
        }
        None
    }

    /// The list is done (the thread's last wait over).
    pub fn done(&self) -> bool {
        self.next >= self.order.len() && self.wait <= 0.0
    }
}

/// KH1's air cast (`0x1402A3600` state 2): a new jump arc of G`+0xE8` = 30 cm and the air
/// velocity × G`+0xF0` = 0.5 (the fast path only scales). Checked (decomp + files).
pub const AIR_HOP: f32 = 30.0;
pub const AIR_SPEED_SCALE: f32 = 0.5;
/// The fast path plays the release motion with an 8-frame blend (`movss xmm2,[0x14042F94C]`
/// = 8.0); the air cast's queued fall too. Checked (exe).
pub const FAST_BLEND: f32 = 8.0;
pub const AIR_FALL_BLEND: f32 = 8.0;

/// KH1's Fire attack data (btltbl `0x5F28` / `0x5F98` / `0x6008`, read through `kh1-formats`
/// from the player's `btltbl.bin`). Checked (files).
pub fn fire_attack(level: u8) -> AttackData {
    let power = match level {
        2 => 28,
        3 => 36,
        _ => 20,
    };
    AttackData {
        stat_source: 1,
        element: 2,
        kind_party: 3,
        kind: 3,
        bone: -1,
        power_party: power,
        mp_charge: 0,
        clash: 5,
        priority: 1,
        recoil_request: 110,
        power,
        radius: 50.0,
        ..Default::default()
    }
}

/// Row flag `0x80` (unused by Re:CoM's 188 rows; `crate::attack::FLAG_KH1`): a KH1 row.
pub const FLAG_KH1: u8 = crate::attack::FLAG_KH1;

/// The KH1 Fire row in Re:CoM's 16-byte attack-row layout: Re:CoM's row of that level (its clip,
/// blend and effect set: Re:CoM's visuals and sounds, decision 34) with KH1's ball. The fields as
/// Re:CoM's spawn reads them (`0x140222170` case `0x22`, Checked (disasm `0x14022220B..`)):
///
/// | byte | Re:CoM reads | KH1 value |
/// |---|---|---|
/// | `+4` | radius × 0.01 m | 50 (KH1's 50 cm) |
/// | `+8` | the homing distance gate (m; homes while the target is nearer) | 255 (KH1 homes at any distance) |
/// | `+9` | speed × 0.01 m per frame | 40 (KH1's 40 cm a frame) |
/// | `+0xB` | Re:CoM kind | 1, flinch (KH1 kind 3) |
/// | `+0xC` | range from the spawn point (m) | 255 (KH1's ball has no range: its life ends it) |
/// | `+0xD` | homing turn, degrees per frame (`0x1401D7F00`: × dt × 0.02) | 3 |
/// | `+0xE` | life × 100 ticks | 15 = 1500 ticks = 30 frames (read by our spawn case; Re:CoM's Fire case uses 12000) |
/// | `+0xF` | flags | `0x80`: a KH1 row |
pub fn fire_row_bytes(recom: &[u8; 16]) -> [u8; 16] {
    let mut b = *recom;
    b[3] = 0;
    b[4] = 50;
    b[8] = 255;
    b[9] = fire::SPEED as u8;
    b[0xB] = 1;
    b[0xC] = 255;
    b[0xD] = fire::TURN_DEG as u8;
    b[0xE] = (fire::LIFE * 50.0 / 100.0) as u8;
    b[0xF] |= FLAG_KH1;
    b
}

/// Re:CoM's rows of the spells built so far, by spell (`[spell][level − 1]`), as in the Steam exe:
/// the templates of the KH1 rows.
pub fn recom_rows() -> Vec<[[u8; 16]; 3]> {
    vec![RECOM_FIRE_ROWS, RECOM_BLIZZARD_ROWS, RECOM_THUNDER_ROWS, RECOM_CURE_ROWS, RECOM_GRAVITY_ROWS, RECOM_STOP_ROWS, RECOM_AERO_ROWS]
}

/// The KH1 magic rows, `[spell × 3 + level − 1]` (21; the spells built so far), from Re:CoM's rows
/// by spell (which must be [`recom_rows`]).
pub fn kh1_magic_table(recom: &[[[u8; 16]; 3]]) -> Vec<[u8; 16]> {
    let mut t = vec![[0u8; 16]; 21];
    for (s, rows) in recom.iter().enumerate().take(7) {
        for (i, r) in rows.iter().enumerate() {
            let lv = i as u8 + 1;
            t[s * 3 + i] = match s as u8 {
                spell::FIRE => fire_row_bytes(r),
                spell::BLIZZARD => blizzard_row_bytes(r, lv),
                spell::THUNDER => thunder_row_bytes(r),
                spell::CURE => cure_row_bytes(r),
                spell::GRAVITY => gravity_row_bytes(r),
                spell::STOP => stop_row_bytes(r),
                spell::AERO => aero_row_bytes(r),
                _ => [0; 16],
            };
        }
    }
    t
}

/// 0.184.2 (fork owner, 2026-11-23: Riku's spells' look, "gostei das sugestões"): Re:CoM's effect set of Dark Riku's
/// Dark Firaga (`0x1405A1730` set 48: `dark_faiga` in `EFFECT/1500/EF1500.EFF`, Riku's own effect file; Checked (exe +
/// files), the workspace's `knowledge/ReCoM/recom-m13-recom-fx.md` § 8).
pub const DARK_FIRAGA_SET: u8 = 48;

/// Riku's copy of the KH1 rows ([`kh1_magic_table`]): every number Sora's (the owner: only the look changes), Fire's
/// three levels' effect set (`+2`, which Re:CoM's Fire ball reads) Dark Firaga's. The other spells keep Re:CoM's own
/// sets (their effects load with Sora's battle set in his battles too, `win::magic_fx::battle_load`).
pub fn riku_look(t: &[[u8; 16]]) -> Vec<[u8; 16]> {
    let mut out = t.to_vec();
    for lv in 1..=3 {
        if let Some(r) = table_index(spell::FIRE, lv).and_then(|i| out.get_mut(i)).filter(|r| r[0] != 0) {
            r[2] = DARK_FIRAGA_SET;
        }
    }
    out
}

/// The index of a spell level in [`kh1_magic_table`].
pub fn table_index(s: u8, level: u8) -> Option<usize> {
    (s < 7 && (1..=3).contains(&level)).then(|| s as usize * 3 + level as usize - 1)
}

/// The shot descriptor's numbers (`0x140156790`'s 0xE0-byte block, filled as Re:CoM's Fire case
/// fills it), from a KH1 row. Re:CoM ticks: 50 per 60 Hz frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShotNumbers {
    /// `+0x60` (u64): the bullet's flags.
    pub flags: u64,
    /// `+0x30`: speed, m per frame.
    pub speed: f32,
    /// `+0x34`: range from the spawn point, m.
    pub range: f32,
    /// `+0x3C`: life, ticks (ends the ball with flag 4).
    pub life: i32,
    /// `+0x48`: the homing distance gate, m.
    pub gate: f32,
    /// `+0x4C`: the homing turn, degrees per frame.
    pub turn: f32,
    /// `+0x78`: radius, m.
    pub radius: f32,
    /// `+0x94`: the fade after the end, ticks (Re:CoM's default 1500).
    pub fade: i32,
}

/// Re:CoM's Fire shot flags `0x9100` (Checked (disasm `0x140222254`)): `0x8000` steers by angles
/// (`0x1401D7F00`), `0x1000` homes only beyond `+0x1C` (0.5 m) from the spawn point, `0x100`
/// stops and fades on a hit, a wall, the range or the life. KH1: homing from its first frame (no
/// `0x1000`), and the life ends it (`4`, `0x140156E20`). Checked (decomp `0x1401575D0`,
/// `0x140156E20`).
pub const RECOM_FIRE_FLAGS: u64 = 0x9100;
pub const KH1_FIRE_FLAGS: u64 = (RECOM_FIRE_FLAGS & !0x1000) | 4;

/// The descriptor numbers a KH1 row gives.
pub fn shot_numbers(row: &[u8; 16]) -> ShotNumbers {
    ShotNumbers {
        flags: KH1_FIRE_FLAGS,
        speed: row[9] as f32 * 0.01,
        range: row[0xC] as f32,
        life: row[0xE] as i32 * 100,
        gate: row[8] as f32,
        turn: row[0xD] as f32,
        radius: row[4] as f32 * 0.01,
        // KH1 fades the ball over its last 20 frames while it flies; Re:CoM fades a stopped one
        // after its end. The fade's length is KH1's.
        fade: (fire::FADE * 50.0) as i32,
    }
}

/// KH1's spell damage on an enemy (`0x1402BFA80`, kh1-core): power + max MP (stat source 1),
/// the curve on power − Defence, the record's floor and cap, × the element's resistance (fire:
/// `resist[2]`). No crit (the record's crit is 0; the roll is still drawn, as KH1 does).
pub fn spell_damage(data: &AttackData, max_mp: i32, def: &DefenderStats, rng: &mut Kh1Rand) -> i32 {
    let attacker = AttackerStats { max_mp, ..Default::default() };
    let live = LiveAttack::new(data, &WeaponData::kingdom_key(), &attacker, 1.0);
    compute_damage(&live, &DamageCurve::kh1_defaults(), def, rng).damage
}

/// [`spell_damage`] with the enemy's per-hit cap left for Re:CoM's join (`crate::damage_cap`): the
/// number before the cap and the cap in its units (None: no cap, or an element KH1 never caps).
pub fn spell_damage_cap_later(data: &AttackData, max_mp: i32, def: &DefenderStats, rng: &mut Kh1Rand) -> (i32, Option<f32>) {
    let attacker = AttackerStats { max_mp, ..Default::default() };
    let live = LiveAttack::new(data, &WeaponData::kingdom_key(), &attacker, 1.0);
    let (d, cap) = kh1_core::damage::compute_damage_cap_later(&live, &DamageCurve::kh1_defaults(), def, rng);
    (d.damage, cap)
}

/// KH1's cast motions (Sora's mset, read with `kh1-data` from the player's
/// `xa_ex_0010.mset`; Checked (files)): Fire's wind-up `0x36` (entry 54, 28 frames, no flags),
/// release `0x3D` (entry 57, 60 frames, free from 46, root −15 cm over frames 0–10, then 5 cm
/// forward to −10 cm at frame 60: linear keys, read with `kh1_mset.py`'s root helper), air
/// wind-up `0x83` (entry 79, 28) and air release `0x8A` (entry 82, 52 frames, free from 46, root
/// −42.4 cm over 0–10). Re:CoM's clips `p001_062`, `p001_063`, `p001_066`, `p001_067` (rows 0x37,
/// 0x38, 0x3B, 0x3C) have the same frame counts at 60 Hz (14, 30, 14, 26 at 30 fps; research
/// `m7-magic.md` § 3.5, Checked (files)).
pub fn add_motions(m: &mut MotionSet) {
    let inf = f32::INFINITY;
    m.add(&[cast_req::FIRE_WINDUP], MotionInfo::new(28.0));
    m.add(&[cast_req::FIRE_RELEASE], MotionInfo::new(60.0).with_flag(FlagKind::Free, 46.0, inf).with_forward_keys(&release_root::KH1_KEYS));
    m.add(&[cast_req::FIRE_AIR_WINDUP], MotionInfo::new(28.0));
    m.add(&[cast_req::FIRE_AIR_RELEASE], MotionInfo::new(52.0).with_flag(FlagKind::Free, 46.0, inf).with_forward_keys(&[(0.0, 0.0), (10.0, -42.4096), (52.0, -42.4096)]));
    // Blizzard (7.5): entry 55, 86 frames, free from 68, no root travel; air entry 80, 78 frames,
    // free from 68. Re:CoM's `p001_065` / `p001_069` (rows 0x3A / 0x3E): 43 / 39 frames at 30 fps.
    m.add(&[cast_req::BLIZZARD], MotionInfo::new(86.0).with_flag(FlagKind::Free, 68.0, inf));
    m.add(&[cast_req::BLIZZARD_AIR], MotionInfo::new(78.0).with_flag(FlagKind::Free, 68.0, inf));
    // Thunder (7.6): entry 56, 68 frames, no free key (locked to its end), no root travel; air
    // entry 81, 68. Re:CoM's `p001_064` / `p001_068` (rows 0x39 / 0x3D): 34 frames at 30 fps.
    m.add(&[cast_req::THUNDER], MotionInfo::new(68.0));
    m.add(&[cast_req::THUNDER_AIR], MotionInfo::new(68.0));
    // Cure (7.7): Thunder's entries 56 / 81 (68 frames, locked to the end, no root travel). Checked
    // (files: requests 0x39 / 0x86 map to them).
    m.add(&[cast_req::CURE], MotionInfo::new(68.0));
    m.add(&[cast_req::CURE_AIR], MotionInfo::new(68.0));
    // Gravity (7.8): Blizzard's entries 55 / 80 (86 / 78 frames, free from 68, no root travel).
    // Checked (files: requests 0x3A / 0x87 map to them).
    m.add(&[cast_req::GRAVITY], MotionInfo::new(86.0).with_flag(FlagKind::Free, 68.0, inf));
    m.add(&[cast_req::GRAVITY_AIR], MotionInfo::new(78.0).with_flag(FlagKind::Free, 68.0, inf));
    // Stop (7.9): Thunder's entries 56 / 81 (68 frames, locked to the end). Checked (files: requests
    // 0x3B / 0x88 map to them).
    m.add(&[cast_req::STOP], MotionInfo::new(68.0));
    m.add(&[cast_req::STOP_AIR], MotionInfo::new(68.0));
    // Aero (7.10): Thunder's entries 56 / 81 (68 frames, locked to the end). Checked (files: requests
    // 0x3C / 0x89 map to them).
    m.add(&[cast_req::AERO], MotionInfo::new(68.0));
    m.add(&[cast_req::AERO_AIR], MotionInfo::new(68.0));
}

/// The ground Fire release's root travel (decision 38: "Fire's release step is KH1's 15 cm curve",
/// replacing Re:CoM's clip travel in the root-step wrapper; `docs/recom-m7-magic.md` § 7.4c).
///
/// - **KH1** (`xa_ex_0010.mset` request `0x3D` → entry 57, root helper read with `kh1_mset.py`):
///   linear keys (0, 0) (10, −15) (60, −10) cm along the actor's z (−1.5 cm a frame over frames
///   0–10, then +0.1 cm a frame to −10 at 60); x and y stay 0. Checked (files).
/// - **Re:CoM** plays clip `p001_063` (row 0x38) for it: bone 0's z goes 0 → −0.4047 m by its frame
///   5 (KH1 frame 10), then +0.002 m a clip frame to −0.3567 at 29 (KH1 58), x and y constant 0
///   (`recom_ann.py`). Its forward drift is KH1's 0.1 cm a frame; only the step back is 2.7 × deeper
///   (40.5 cm, not 15). Checked (files).
/// - **Every other cast motion matches already** (`kh1_mset.py` / `recom_ann.py`, Checked (files)):
///   the air release `0x8A` (entry 82, −42.41 cm by frame 10, then still) and `p001_067` (row 0x3C,
///   −0.4241 m by its frame 5, then still) are the same curve; the wind-ups (`0x36` / `0x83`,
///   entries 54 / 79) and Blizzard / Thunder / Cure / Gravity / Stop / Aero's motions (entries 55,
///   56, 80, 81) have no root travel in KH1, and their clips (`p001_062`, `064`, `065`, `066`, `068`,
///   `069`) none in Re:CoM (bone 0 constant 0; their rows are played without root motion,
///   `rows::row_for`).
///
/// In game: the root-step wrapper (`0x14024DCA0`, decision 17's) replaces Re:CoM's travel of each
/// update of row 0x38 in a KH1 cast with KH1's curve between the clip's last time and its time now
/// (`ctrl+0x3C`, 50 ticks a KH1 frame), along the line Re:CoM's own travel takes (the model's z);
/// the one-update delay then applies to it as to every KH1 root travel.
pub mod release_root {
    use kh1_core::Vec3;

    /// KH1's root keys of the ground release (frame, cm along the actor's z).
    pub const KH1_KEYS: [(f32, f32); 3] = [(0.0, 0.0), (10.0, -15.0), (60.0, -10.0)];
    /// Re:CoM's row of the ground release (`p001_063`).
    pub const RECOM_ROW: u32 = crate::rows::row::FIRE_RELEASE;

    /// KH1's root z (cm) at KH1 frame `t` (linear between the keys, held outside them).
    pub fn kh1_z(t: f32) -> f32 {
        let k = &KH1_KEYS;
        if t <= k[0].0 {
            return k[0].1;
        }
        for w in k.windows(2) {
            let ((t0, z0), (t1, z1)) = (w[0], w[1]);
            if t <= t1 {
                return z0 + (z1 - z0) * (t - t0) / (t1 - t0);
            }
        }
        k[k.len() - 1].1
    }

    /// The world travel (Re:CoM metres) for an update of the release from KH1 frame `t0` to `t1`:
    /// KH1's z difference along the model's z axis. The axis is the line of Re:CoM's own travel this
    /// update (`recom`: its clip moves only along z), pointed along Sora's facing `fwd` (Re:CoM's
    /// (sin yaw, 0, cos yaw)); with no travel of Re:CoM's (its clip holds still after its frame 29),
    /// the facing itself. Assumption: the model's +z is Sora's facing (the clip steps him back).
    pub fn travel(recom: Vec3, fwd: Vec3, t0: f32, t1: f32) -> Vec3 {
        let dz = (kh1_z(t1) - kh1_z(t0)) * crate::units::METRES_PER_UNIT;
        let flat = Vec3::new(recom.x, 0.0, recom.z);
        let axis = if flat.horizontal_length() > 1e-5 {
            let u = flat.normalized();
            if u.dot(fwd) < 0.0 {
                u * -1.0
            } else {
                u
            }
        } else {
            fwd.flat().normalized()
        };
        axis * dz
    }

    /// The release clip's time at the last update the wrapper saw, so each update's span is known.
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub struct Clock {
        ctrl: usize,
        t: f32,
        live: bool,
    }

    impl Clock {
        pub const fn new() -> Clock {
            Clock { ctrl: 0, t: 0.0, live: false }
        }

        /// The span (t0, t1) of this update (KH1 frames; `t1` = the clip's time now, at most 60).
        /// A new clip (none seen, another controller, or a time that went back: the Fire chain
        /// restarts the clip) starts from frame 0, so a clip Re:CoM starts one step on (a queued
        /// motion, decision 17) still hands out KH1's travel of frame 0 → 1.
        pub fn span(&mut self, ctrl: usize, t1: f32) -> (f32, f32) {
            let t1 = t1.clamp(0.0, KH1_KEYS[KH1_KEYS.len() - 1].0);
            let t0 = if self.live && self.ctrl == ctrl && self.t <= t1 { self.t } else { 0.0 };
            *self = Clock { ctrl, t: t1, live: true };
            (t0, t1)
        }

        /// Not the release clip this update: the next one starts from frame 0.
        pub fn reset(&mut self) {
            self.live = false;
        }
    }
}

/// Whether a KH1 request is one of the cast motions Re:CoM's magic action plays.
pub fn is_cast_request(r: u16) -> bool {
    (0..7u8).filter(|&s| castable(s)).any(|s| [false, true].iter().any(|&a| r == cast_req::first(s, a) || r == cast_req::release(s, a)))
}

/// Fire's wind-up (its script waits while it plays).
fn is_windup(r: u16) -> bool {
    r == cast_req::FIRE_WINDUP || r == cast_req::FIRE_AIR_WINDUP
}

/// Fire's release motion (the Fire re-cast and fast path read it).
fn is_release(r: u16) -> bool {
    r == cast_req::FIRE_RELEASE || r == cast_req::FIRE_AIR_RELEASE
}

/// The motion a spell's script requires at its release check, ground or air.
fn is_release_of(s: u8, r: u16) -> bool {
    r == cast_req::release(s, false) || r == cast_req::release(s, true)
}

/// The script's wait from the command to its release check (Fire's wind-up path waits on the
/// motion instead: 0).
pub fn release_wait(s: u8) -> f32 {
    match s {
        spell::BLIZZARD => blizzard::WAIT,
        spell::THUNDER => thunder::WAIT,
        spell::CURE => cure::WAIT,
        spell::GRAVITY => gravity::WAIT,
        // Stop: `L009a2(4)`, then the check each frame for 24: the release at 28 (a cut motion
        // before it ends the cast through `leave_cast` / the resync, as KH1's per-frame check).
        spell::STOP => stop::WAIT,
        spell::AERO => aero::WAIT,
        _ => 0.0,
    }
}

/// A KH1 magic command (the menu's, or a shortcut's in 7.12): spell 0..6, level 1..3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MagicCommand {
    pub spell: u8,
    pub level: u8,
}

/// A cast for Re:CoM to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CastStart {
    pub spell: u8,
    pub level: u8,
    /// Started in the air (Re:CoM's air clips, `+0x478` = 4).
    pub air: bool,
    /// KH1's Fire fast path (no wind-up).
    pub fast: bool,
}

/// The cast in progress: KH1's `.mag` thread state that Re:CoM has no place for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cast {
    pub start: CastStart,
    /// The fast path's countdown (`L006fa`), frames.
    pub wait: f32,
    /// Frames since the command (for the log and trace).
    pub age: f32,
    /// The ball is out (MP paid), or the script ended without one.
    pub released: bool,
    pub cancelled: bool,
    /// Re:CoM's shot id (`bullet+0x20`), once launched.
    pub shot: Option<u32>,
    /// Started in this frame (a new cast replacing an older one keeps Re:CoM's action).
    pub fresh: bool,
}

/// What the cast did this frame (log and trace).
#[derive(Clone, Debug, PartialEq)]
pub enum CastEvent {
    /// The command was taken: spell, level, fast path, air.
    Started { spell: u8, level: u8, fast: bool, air: bool },
    /// The command was refused (KH1's gate, the state, the MP, no Re:CoM row).
    Refused { spell: u8, level: u8, why: &'static str },
    /// The release frame: frames since the command, the shot id (None: Re:CoM made no shot).
    Released { spell: u8, level: u8, frame: f32, shot: Option<u32> },
    /// The script's release check failed, or the cast was cut before it: no spell, no MP.
    Interrupted { spell: u8, level: u8, frame: f32, by: u16 },
}

impl CastEvent {
    pub fn token(&self) -> String {
        let n = |s: u8, l: u8| format!("{}{l}", spell::NAMES.get(s as usize).copied().unwrap_or("?"));
        match self {
            CastEvent::Started { spell, level, fast, air } => format!("cast({}{}{})", n(*spell, *level), if *fast { " fast" } else { "" }, if *air { " air" } else { "" }),
            CastEvent::Refused { spell, level, why } => format!("cast-refused({} {why})", n(*spell, *level)),
            CastEvent::Released { spell, level, frame, shot } => format!("release({} f{frame:.0}{})", n(*spell, *level), shot.map_or(" no-shot".to_string(), |s| format!(" shot {s}"))),
            CastEvent::Interrupted { spell, level, frame, by } => format!("cast-cut({} f{frame:.0} req {by:#x})", n(*spell, *level)),
        }
    }
}

/// The trace token of the cast in progress: `cast -`, or `cast fire1 f<age> [fast] [w<wait>]
/// [rel] [shot <id>]`.
pub fn trace_token(c: Option<&Cast>) -> String {
    match c {
        None => "cast -".into(),
        Some(c) => {
            let s = c.start;
            let mut t = format!("cast {}{} f{:.0}", spell::NAMES.get(s.spell as usize).copied().unwrap_or("?"), s.level, c.age);
            if s.fast {
                t.push_str(&format!(" fast w{:.0}", c.wait.max(0.0)));
            } else if c.wait > 0.0 {
                t.push_str(&format!(" w{:.0}", c.wait));
            }
            if s.air {
                t.push_str(" air");
            }
            if c.released {
                t.push_str(" rel");
            }
            if c.cancelled {
                t.push_str(" cut");
            }
            if let Some(id) = c.shot {
                t.push_str(&format!(" shot {id}"));
            }
            t
        }
    }
}

// ---- KH1's magic command in the handler frame ----

impl Kh1Sora {
    /// `0x140293480` for a spell: not in the take-off; free or in a combo window; else only Fire
    /// while the release motion (`0x3D` / `0x8A`) is at frame 14 or later. Checked (decomp).
    pub fn accepts_spell(&self, s: u8) -> bool {
        if self.state == state::TAKE_OFF {
            return false;
        }
        if !self.lock || self.in_window() {
            return true;
        }
        s == spell::FIRE && is_release(self.rec.req) && fire::RECAST_FROM <= self.rec.frame
    }

    /// The Fire fast path's condition in `0x1402A7510` (Checked (decomp + disasm
    /// `0x1402A75B6`)): a Fire command while the playing request is the release motion, or swing 1
    /// (200) / the lunge (202) inside its combo window.
    pub fn fire_fast_path(&self, s: u8) -> bool {
        s == spell::FIRE && (is_release(self.rec.req) || ((self.rec.req == 200 || self.rec.req == 202) && self.in_window()))
    }

    /// Re:CoM's action for a cast in progress (its magic action, as `initMagic` sets it).
    pub(crate) fn cast_action(&self) -> Option<Action> {
        let c = self.cast.as_ref()?;
        is_cast_request(self.rec.head()).then(|| Action { id: RECOM_MAGIC_ACTION, sub: recom_attack_id(c.start.spell), phase: MAGIC_PHASE })
    }

    /// `0x1402A7510` type 2 (the Fire fast path), else `0x1402A3600` → `0x1402A3DB0` (ground) / the
    /// air branch. The command first faces its target (`0x1402A38E0`, record flag 2: all spells).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn command_magic(&mut self, h: &mut dyn Host, dir: &mut Vec3, w: &mut f32, target: Option<TargetPart>, pos: Vec3, m: MagicCommand) {
        let refuse = |me: &mut Kh1Sora, why| me.cast_log.push(CastEvent::Refused { spell: m.spell, level: m.level, why });
        if !castable(m.spell) {
            return refuse(self, "spell not in this stage");
        }
        // `0x1402A38E0`: toward the target's part, else the stick (or the facing); no speed.
        if let Some(t) = target {
            let d = (t.centre - pos).flat().normalized();
            if !d.is_zero_xz() {
                *dir = d;
                self.target_yaw = d.yaw();
            }
        } else if dir.is_zero_xz() {
            *dir = Vec3::from_yaw(self.facing);
        }
        *w = 0.0;
        let fast = self.fire_fast_path(m.spell);
        let air = self.state == state::AIR;
        if !fast {
            // `0x1402A3600`: ground (0) and air (2) cast; other states drop the command.
            if self.state != state::GROUND && !air {
                return refuse(self, "state");
            }
            // `0x14028EFD0`: KH1's usable test (learned, MP for the cost).
            if !h.can_cast(m.spell, m.level) {
                return refuse(self, "mp");
            }
        }
        let start = CastStart { spell: m.spell, level: m.level, air, fast };
        // A swing playing (the fast path from swing 1 / the lunge, or a cast from a combo
        // window): Re:CoM ends its attack action first, as the new action starts.
        self.leave_swing(h);
        if self.recom_took {
            // Re:CoM's win action took over as the swing ended (the frame then yields).
            return refuse(self, "Re:CoM's win");
        }
        if !h.start_cast(&start) {
            return refuse(self, "no Re:CoM row");
        }
        // KH1's Sora holds the Keyblade for the whole cast; Re:CoM hides it in the field and
        // draws it for its own attacks (the field swing init `0x1401F0F70` ends with its vtable
        // `+0xF8`, `0x1401F5D30(Sora, 1)`). Its `initMagic` never draws it (Re:CoM casts only in
        // battle, where it is already out), so a KH1 cast draws it here as the swing and Guard do.
        // Already out: Re:CoM's function does nothing (`weapon+0x154 & 1` clear).
        h.draw_weapon();
        use cast_req::*;
        if fast {
            if air {
                self.play(FIRE_AIR_RELEASE as u32 | req::ACTION, FAST_BLEND);
                self.append(req::RELEASE | request::FALL as u32, 0.0);
                self.air_vel = self.air_vel * AIR_SPEED_SCALE;
            } else {
                self.play(FIRE_RELEASE as u32 | req::ACTION, FAST_BLEND);
                self.append(req::RELEASE | request::IDLE as u32, 0.0);
            }
        } else if air {
            // `0x1402A3600` state 2: `0x83 + spell` (Fire: then `0x8A`), the fall (blend 8).
            self.play(cast_req::first(m.spell, true) as u32 | req::ACTION, 0.0);
            if m.spell == spell::FIRE {
                self.append(FIRE_AIR_RELEASE as u32, 0.0);
            }
            self.append(req::RELEASE | request::FALL as u32, AIR_FALL_BLEND);
            // `0x1402AF960(actor, 30)`, then `+0x3C0` × 0.5.
            self.hop(AIR_HOP, pos.y);
            self.air_vel = self.air_vel * AIR_SPEED_SCALE;
        } else {
            // `0x1402A3DB0`: `0x36 + spell` (Fire: then `0x3D`), then idle.
            self.play(cast_req::first(m.spell, false) as u32 | req::ACTION, 0.0);
            if m.spell == spell::FIRE {
                self.append(FIRE_RELEASE as u32, 0.0);
            }
            self.append(req::RELEASE | request::IDLE as u32, 0.0);
        }
        let wait = if fast { fire::FAST_WAIT } else { release_wait(m.spell) };
        self.cast = Some(Cast { start, wait, age: 0.0, released: false, cancelled: false, shot: None, fresh: true });
        self.cast_log.push(CastEvent::Started { spell: m.spell, level: m.level, fast, air });
    }

    /// One run of KH1's spell script (`xm_fire01.mag` main) for the last frame. KH1 runs the spell
    /// threads in their task (`0x14028F6B0`, priority 137000) after the actors' update, so a run
    /// sees that frame's motion update: here, right after [`Kh1Sora::frame`] has replayed it at
    /// the start of the next handler. The wind-up path waits while the wind-up plays, the fast
    /// path 8 frames (`L006fa`); then the release motion must be playing, else nothing is cast and
    /// no MP is paid; else the ball goes and the MP is paid (sys 20, then sys 47). `age` counts
    /// the runs from 0 in the command's frame, so the ball's KH1 frame is the age of its run.
    pub(crate) fn cast_step(&mut self, h: &mut dyn Host, step: f32) {
        let head = self.rec.head();
        let Some(c) = self.cast.as_mut() else { return };
        let age = c.age;
        c.age += step;
        c.fresh = false;
        if c.released || c.cancelled {
            return;
        }
        // The fast path's `L006fa(8)`, Blizzard's `L0086e(28)`: count down, a frame at a time.
        if c.wait > 0.0 {
            c.wait -= step;
            return;
        }
        if c.start.spell == spell::FIRE && !c.start.fast && is_windup(head) {
            return;
        }
        let s = c.start;
        // Aero (`0x50E..0x56C`): the motion leaving its request ends the wait, and it casts anyway.
        if !is_release_of(s.spell, head) && s.spell != spell::AERO {
            c.cancelled = true;
            self.cast_log.push(CastEvent::Interrupted { spell: s.spell, level: s.level, frame: age, by: head });
            return;
        }
        c.released = true;
        let shot = h.release_cast(&s);
        if let Some(c) = self.cast.as_mut() {
            c.shot = shot;
        }
        self.cast_log.push(CastEvent::Released { spell: s.spell, level: s.level, frame: age, shot });
    }

    /// The cast's motions are no longer KH1's playing motion: Re:CoM ends its magic action
    /// (`0x1401F6660`, as `exeMagic` ends it) unless a new swing or cast already took Re:CoM's
    /// action over. A cast cut before its release casts nothing and pays nothing.
    pub(crate) fn leave_cast(&mut self, h: &mut dyn Host) {
        let Some(c) = self.cast else { return };
        if is_cast_request(self.rec.head()) {
            return;
        }
        self.cast = None;
        if !c.released && !c.cancelled {
            if c.start.spell == spell::AERO {
                // KH1's Aero isn't cut: a motion change ends its wait and it casts now.
                let shot = h.release_cast(&c.start);
                self.cast_log.push(CastEvent::Released { spell: c.start.spell, level: c.start.level, frame: c.age, shot });
            } else {
                self.cast_log.push(CastEvent::Interrupted { spell: c.start.spell, level: c.start.level, frame: c.age, by: self.rec.head() });
            }
        }
        if self.swing.is_none() && h.end_cast(self.state != state::GROUND) {
            self.recom_took = true;
        }
    }

    /// The cast record and its events, for the frame's caller (drained).
    pub fn take_cast_log(&mut self) -> Vec<CastEvent> {
        std::mem::take(&mut self.cast_log)
    }
}

/// The Field exe (`0x1401FA0F0`) has no case for action `0x13`: Re:CoM never casts in the field
/// (its jump table `0x1401FA5F4` sends the id to the default `0x1401FA599`). KH1 casts in the
/// field, so the entry is pointed at a stub in an int3 cave that runs `exeMagic` on the player
/// (`rbx` in the Field exe's switch) and goes on at the default, as the table's other cases do
/// (`mov rcx,rbx; call exe; jmp default`). The Battle exe already calls it through Sora's vtable
/// (`0x1401F9B4D`). Checked (disasm `0x1401FA1B0`, `0x1401FA599`).
pub mod field_case {
    pub const TABLE_ENTRY_RVA: u32 = 0x001F_A63C;
    pub const CAVE_RVA: u32 = 0x001F_B0E1;
    pub const EXE_MAGIC_RVA: u32 = 0x0021_7EE0;
    pub const DEFAULT_RVA: u32 = 0x001F_A599;
    pub const STUB_LEN: usize = 13;

    /// The 13-byte stub at the cave.
    pub fn stub() -> [u8; STUB_LEN] {
        let mut b = [0u8; STUB_LEN];
        b[..3].copy_from_slice(&[0x48, 0x8B, 0xCB]);
        b[3] = 0xE8;
        let call = EXE_MAGIC_RVA as i64 - (CAVE_RVA as i64 + 8);
        b[4..8].copy_from_slice(&(call as i32).to_le_bytes());
        b[8] = 0xE9;
        let jmp = DEFAULT_RVA as i64 - (CAVE_RVA as i64 + 13);
        b[9..13].copy_from_slice(&(jmp as i32).to_le_bytes());
        b
    }

    /// The table entry's new value (an RVA, as the table holds).
    pub fn entry() -> [u8; 4] {
        CAVE_RVA.to_le_bytes()
    }
}

/// KH1 casts in the field (owner's 0.45.0 video, "13F Entrance Hall": no spell effects in the
/// field). Re:CoM's spell effects (`fire_all`, `bliz_all`, `_thunder_all`, `cure_all`, …: the
/// effect sets `0x1405A1730`) are looked up by name among the **loaded** resources (`0x1402A4920`
/// → `0x1402A43E0(5, name)`; none loaded: no effect, handle 0). Their packs come with Sora's
/// **battle set**, PLAYER group 1001 (Halloween Town Sora's too; Riku 1101 / 1201), whose LINK rows
/// load EFFECT groups 1100, 1102, 2000, 2003, 2010, 2013, 2017, 2020, 2023, 2030, 2040, 2050,
/// 2060, 2063, 2073, 2078, 3090, 3220, 501, 502, 504, 506, 556 (Checked (exe): the PLAYER table).
/// Re:CoM queues it only in its battle scene (`CGameSubBattle` state 1 `0x1401AEA20` →
/// `0x140211960(0)`) and, under its system byte (0, 0x60), from `CGameMain`'s load state
/// (`0x1401AD040`), which also keeps it through the camp. Its field scene (`CGameSubField` state 1
/// `0x1401AF920`) queues only the player's group (`0x140211AA0(0)`), and the camp's two openings
/// (`0x1401A9590`, `0x1401A9990`) unload the battle set (`0x140211560(0)`) unless that byte is set.
/// Checked (decomp + exe).
///
/// At Combat = KH1 in Sora's story the field does what Re:CoM does under that byte, with Re:CoM's
/// own calls: the field state's tail jump to `0x140211AA0` goes to a wrapper that then calls
/// `0x140211960(0)` (the battle state's call), and the camp openings' unload calls keep the set.
pub mod field_fx {
    /// `0x140211960(char)`: Re:CoM's battle-set loader (queues PLAYER group 1001 / 1101 / 1201
    /// and the char's extra group when not loaded).
    pub const LOAD_RVA: u32 = 0x0021_1960;
    /// `0x140211560(char)`: Re:CoM's battle-set unloader.
    pub const UNLOAD_RVA: u32 = 0x0021_1560;
    /// `0x140211AA0(char)`: Re:CoM's "queue the player's group" (the field state's tail call).
    pub const PLAYER_GROUP_RVA: u32 = 0x0021_1AA0;
    /// `CGameSubField` state 1's tail `jmp 0x140211AA0` (`E9 68 21 06 00`).
    pub const FIELD_JUMP_RVA: u32 = 0x001A_F933;
    /// The camp openings' `call 0x140211560` (`0x1401A9590` open path, `0x1401A9990`).
    pub const CAMP_CALL_RVAS: [u32; 2] = [0x001A_95BD, 0x001A_99F9];
    /// Sora's battle set (PLAYER group), and the global the loader leaves it in (`0x1407C7AE4`).
    pub const SORA_SET: u32 = 1001;
    pub const SET_GLOBAL: u64 = 0x1_407C_7AE4;

    /// The battle set is wanted in the field: KH1 casts (Combat = KH1) in Sora's story.
    pub fn wanted(kh1: bool, riku_story: bool) -> bool {
        kh1 && !riku_story
    }

    /// Where a `jmp rel32` / `call rel32` at `at` goes (RVAs), if `b` is one.
    pub fn branch_target(at: u32, b: &[u8]) -> Option<u32> {
        if b.len() != 5 || (b[0] != 0xE8 && b[0] != 0xE9) {
            return None;
        }
        let rel = i32::from_le_bytes([b[1], b[2], b[3], b[4]]) as i64;
        u32::try_from(at as i64 + 5 + rel).ok()
    }
}

#[cfg(test)]
mod tests;
