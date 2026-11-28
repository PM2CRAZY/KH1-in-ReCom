//! KH1's Attack (no cards) inside Re:CoM's own attack action (roadmap M5,
//! `docs/recom-kh1-attack.md`, design `docs/recom-attack-design.md`).
//!
//! Re:CoM owns the swing: its field-swing action `0xA` (init `0x1401F0F70`, exe `0x1401EAEB0`, now
//! also in battle), its row pointer `player+0x3C8` and attack id `player+0x638`, its motion system
//! (Re:CoM's own swing clips, decision 10), its hit window test and `CPlayerWeapon::onAttack`, its
//! collision pass and its damage path. What KH1 adds:
//!
//! - **KH1 rows** in Re:CoM's 16-byte row layout ([`kh1_row_bytes`]): a copy of the Re:CoM row that
//!   plays the same clip, with KH1's hit window, KH1's combo window in the unused bytes `+1` / `+3`,
//!   no homing, no push, and flag `0x80` ("KH1 row"). `+0x3C8` points at them; `+0x638` stays the
//!   Re:CoM id of the clip, so the hit effect, sounds and trails stay Re:CoM's own.
//! - **KH1's command and picker** ([`Kh1Sora::command_attack`], kh1-core's `combo.rs`), run in the
//!   handler frame with KH1's combo counter, window and last-swing-missed rule.
//! - **KH1's Keyblade volume** ([`blade_shapes`]): the sphere chain on the blade plus the swept chain,
//!   tested with Re:CoM's own sphere / column primitives in its pair dispatch.
//!
//! Units: the logic is in KH1 units (cm, y down), like [`crate::movement`]; the blade is computed in
//! Re:CoM metres from Re:CoM's own weapon bones (the line is the same at any scale).

use crate::movement::{action, Host, Kh1Sora, SwingStart};
use kh1_core::combo::{pick_air, pick_ground, AirCtx, GroundCtx, TargetPart};
use kh1_core::controller::{state, Event, Refused};
use kh1_core::damage::{AttackerStats, LiveAttack};
use kh1_core::data::{row, AttackData, Moveset, WeaponData};
use kh1_core::motion::{flag, req, request, FlagKind};
use kh1_core::shape::sphere_chain;
use kh1_core::Vec3;

/// Re:CoM's player fields of the attack (`docs/recom-player.md`, `0x1401F7520`). Checked (decomp).
pub mod field {
    /// Pointer to the current 16-byte attack row.
    pub const ROW_PTR: usize = 0x3C8;
    /// The weapon (`CPlayerWeapon`).
    pub const WEAPON: usize = 0x3D0;
    /// Attack stat (the cardless AP).
    pub const ATTACK_STAT: usize = 0x43C;
    pub const SPEED_4E0: usize = 0x4E0;
    pub const VSPEED: usize = 0x5D4;
    pub const SITUATION: usize = 0x630;
    pub const PREV_SITUATION: usize = 0x634;
    pub const ATTACK_ID: usize = 0x638;
    pub const COUNTER: usize = 0x63C;
    pub const LENGTH: usize = 0x640;
    /// Set by `procAttackCollision` when the swing hit (weapon `+0x2C8` 1 or 2).
    pub const HIT: usize = 0x644;
    pub const CHAINABLE: usize = 0x648;
    pub const QUEUED: usize = 0x650;
    /// The post-swing lockout the field input tests (`0x1401F8150`).
    pub const LOCKOUT: usize = 0x654;
    pub const AP: usize = 0x660;
    pub const KIND: usize = 0x668;
    pub const ATTR: usize = 0x66C;
    pub const CARD_7B4: usize = 0x7B4;
    pub const HOMING_TARGET: usize = 0x854;
}

/// Re:CoM's attack-row table (188 rows of 16 bytes). Checked (exe).
pub const RECOM_ATTACK_ROWS: u64 = 0x1_407C_6C70;

/// Re:CoM's rows 1–7 (Sora's cardless and card swings), read from the Steam exe. The runtime reads
/// the live table and refuses to build KH1 rows unless it holds exactly these bytes.
pub const RECOM_ROWS_1_7: [[u8; 16]; 7] = [
    [0x30, 0x00, 0x01, 0x00, 0x0C, 0x1A, 0x08, 0x1E, 0x14, 0x08, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01],
    [0x31, 0x00, 0x01, 0x00, 0x0C, 0x20, 0x0C, 0x1E, 0x14, 0x08, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01],
    [0x32, 0x00, 0x01, 0x00, 0x0C, 0x2E, 0x0F, 0x2D, 0x14, 0x08, 0x01, 0x01, 0x00, 0x00, 0x00, 0x01],
    [0x33, 0x00, 0x03, 0x00, 0x16, 0x34, 0x0A, 0x1E, 0x14, 0x08, 0x02, 0x02, 0x00, 0x00, 0x00, 0x00],
    [0x34, 0x00, 0x01, 0x00, 0x08, 0x1A, 0x00, 0x12, 0x14, 0x08, 0x01, 0x01, 0x02, 0x18, 0x00, 0x19],
    [0x35, 0x00, 0x01, 0x00, 0x04, 0x10, 0x00, 0x0C, 0x14, 0x08, 0x00, 0x01, 0x08, 0x0A, 0x00, 0x19],
    [0x36, 0x00, 0x03, 0x00, 0x0A, 0x2A, 0x0A, 0x0E, 0x14, 0x08, 0x02, 0x02, 0x08, 0x10, 0x00, 0x18],
];

/// Riku's own swing rows 0x97..=0x9D (`RECOM_ATTACK_ROWS + id × 16`), the twins of Sora's rows 1–7:
/// the same action rows (clips `p002_044..050`), his own hit windows `+4` / `+5`. Read from the
/// Steam exe (Checked (exe), 2026-11-27); the runtime refuses Riku's table unless the live rows match.
pub const RIKU_ROWS_1_7: [[u8; 16]; 7] = [
    [0x30, 0x00, 0x12, 0x00, 0x0C, 0x1A, 0x08, 0x1E, 0x14, 0x08, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01],
    [0x31, 0x00, 0x12, 0x00, 0x0C, 0x20, 0x08, 0x28, 0x14, 0x08, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01],
    [0x32, 0x00, 0x12, 0x00, 0x14, 0x26, 0x0A, 0x28, 0x14, 0x08, 0x01, 0x01, 0x00, 0x00, 0x00, 0x01],
    [0x33, 0x00, 0x14, 0x00, 0x12, 0x30, 0x06, 0x1C, 0x14, 0x08, 0x02, 0x02, 0x00, 0x00, 0x00, 0x00],
    [0x34, 0x00, 0x12, 0x00, 0x08, 0x1A, 0x00, 0x14, 0x14, 0x08, 0x01, 0x01, 0x02, 0x18, 0x00, 0x19],
    [0x35, 0x00, 0x12, 0x00, 0x04, 0x10, 0x00, 0x0E, 0x14, 0x08, 0x00, 0x01, 0x08, 0x0A, 0x00, 0x19],
    [0x36, 0x00, 0x14, 0x00, 0x0A, 0x26, 0x0A, 0x10, 0x14, 0x08, 0x02, 0x02, 0x08, 0x10, 0x00, 0x18],
];

/// Dark Riku's swing rows 0x9E..=0xA4 (clips `p003_*` on the same action rows 0x30..=0x36), his own
/// hit windows. Checked (exe), 2026-11-27.
pub const DARK_ROWS_1_7: [[u8; 16]; 7] = [
    [0x30, 0x00, 0x15, 0x00, 0x04, 0x16, 0x04, 0x12, 0x14, 0x08, 0x03, 0x01, 0x00, 0x00, 0x00, 0x01],
    [0x31, 0x00, 0x15, 0x00, 0x04, 0x2A, 0x0A, 0x10, 0x14, 0x08, 0x03, 0x01, 0x00, 0x00, 0x00, 0x01],
    [0x32, 0x00, 0x15, 0x00, 0x08, 0x1A, 0x08, 0x18, 0x14, 0x08, 0x03, 0x01, 0x00, 0x00, 0x00, 0x01],
    [0x33, 0x00, 0x16, 0x00, 0x12, 0x28, 0x08, 0x1A, 0x14, 0x08, 0x03, 0x02, 0x00, 0x00, 0x00, 0x00],
    [0x34, 0x00, 0x15, 0x00, 0x08, 0x1A, 0x00, 0x14, 0x14, 0x08, 0x03, 0x01, 0x02, 0x18, 0x00, 0x19],
    [0x35, 0x00, 0x16, 0x00, 0x08, 0x1E, 0x00, 0x10, 0x14, 0x08, 0x03, 0x02, 0x08, 0x0A, 0x00, 0x10],
    [0x36, 0x00, 0x16, 0x00, 0x04, 0x32, 0x0A, 0x12, 0x14, 0x08, 0x03, 0x02, 0x08, 0x10, 0x00, 0x19],
];

/// Riku's (or Dark Riku's) copy of the KH1 table: the swing rows of [`ROW_MAP`] take the hit window
/// (`+4` / `+5`) and the homing (`+6..=+9`, 0.198.26) of his own Re:CoM row of the same clip, in
/// Re:CoM's own frame terms (no [`HIT_LEAD`]: Re:CoM's exe reads these bytes itself). Everything else
/// stays the KH1 row's (combo window, kind, flag 0x80). Fork owner, 2026-11-27: KH1's system adapted to Riku's own moves
/// (`docs/design-philosophy-instructions.md` § Riku's combat); before this his clips swung with KH1
/// Sora's hit frames and the blade passed through enemies while its volume was off.
pub fn riku_table(kh1: &[[u8; 16]], own_rows_1_7: &[[u8; 16]; 7]) -> Vec<[u8; 16]> {
    let mut t = kh1.to_vec();
    for &(r, id) in ROW_MAP.iter() {
        let Some(row) = t.get_mut(r).filter(|b| b[0] != 0) else { continue };
        let own = &own_rows_1_7[id as usize - 1];
        row[4] = own[4];
        row[5] = own[5];
        // 0.198.26: his approach too (Re:CoM's homing: turn to the target until `+6`, then close in
        // at `+9` / 60 until within `+8` × 0.1 m before `+7`, while nothing is hit;
        // `knowledge/ReCoM/recom-riku-combo.md` § 5). KH1 Sora's rows carry none.
        row[6..10].copy_from_slice(&own[6..10]);
    }
    t
}

/// Row flag `0x80` (unused by all 188 Re:CoM rows, Checked (exe)): a KH1 row, whose `+1` / `+3`
/// hold KH1's combo window.
pub const FLAG_KH1: u8 = 0x80;
/// Re:CoM's attack action and the phase its init sets (`+0x604 = 3`).
pub const ATTACK_PHASE: u32 = 3;

/// KH1 row → the Re:CoM attack id whose clip is the same KH1 motion (Checked (exe action table +
/// `kh1-motions.json`, `recom-attack-design.md` §1)): 13/200 → 1, 11/201 → 2, 14/202 → 3,
/// 8/203 → 4, 17/205 → 5, 18/204 → 6, 16/206 → 7; the jump-up row 1 plays 205 (→ 5) and the hop
/// row 2 plays 204 (→ 6).
pub const ROW_MAP: [(usize, u32); 9] = [(13, 1), (11, 2), (14, 3), (8, 4), (17, 5), (18, 6), (16, 7), (1, 5), (2, 6)];

/// The combo modifiers' rows (M9 stage 9a, `docs/recom-m9-abilities.md`): KH1 row → the Re:CoM
/// row (1–7) whose layout the KH1 row copies and whose id `+0x638` / `+0x630` take, by the move's
/// kind: the jump-up Aerial Sweep as the jump-up row 1 (5), the ground finishers as the finisher
/// (4), Sliding Dash as swing 1 (1), Vortex as the side swing (2), Hurricane Blast as the air
/// finisher (7). The clip (`+0`) is the modifier's own (`kh1_table`'s `clip_row`). Re:CoM's sleight
/// ids are not used: Re:CoM reads `+0x638` in many places and some test sleight ids (Strike Raid's
/// 0xD among them), so the ids stay Re:CoM's plain swings'. Checked (decomp: the readers).
pub const MODIFIER_ROWS: [(usize, u32); 9] = [
    (row::AERIAL_SWEEP, 5),
    (row::STUN_IMPACT, 4),
    (row::GRAVITY_BREAK, 4),
    (row::ZANTETSUKEN, 4),
    (row::BLITZ, 4),
    (row::RIPPLE_DRIVE, 4),
    (row::SLIDING_DASH, 1),
    (row::VORTEX, 2),
    (row::HURRICANE_BLAST, 7),
];

pub fn recom_id(kh1_row: usize) -> Option<u32> {
    ROW_MAP.iter().chain(MODIFIER_ROWS.iter()).find(|&&(r, _)| r == kh1_row).map(|&(_, id)| id)
}

/// Re:CoM tests a row's hit window in its exe against the clock of the **last** motion update
/// (`frame = time / 50` before this frame's update), and the weapon's collision runs later in the
/// same frame on this frame's pose. KH1 tests its hit flags after this frame's update. So the
/// Re:CoM row holds KH1's window one frame earlier. Assumption: the weapon's task runs after
/// Sora's motion update in the frame (checked by the trace's `hit_active` frames in game).
pub const HIT_LEAD: u8 = 1;

/// KH1 kind → Re:CoM kind (`docs/recom-attack-design.md` §5): flinch 3 → 1, launch 5 → 2. Gravity
/// Break's reaction 8 is a flinch carrying KH1's % damage, as the Gravity spell's hit (M7 7.8:
/// Re:CoM's Gravity kind 0xD computes its own % and flattens for 3 s).
pub fn recom_kind(kh1_kind: u8) -> u8 {
    match kh1_kind {
        5 => 2,
        _ => 1,
    }
}

/// KH1's stun element (10, `0x1402BEEE0` → `0x1402B8730`).
pub const KH1_STUN_ELEMENT: u8 = 10;
/// Re:CoM's stun kind: its reaction picker `0x14017D260` sends kind 7 to the enemy's stun state
/// (`+0x900`) when the enemy's param `+0x28` allows it, else to the flinch; its damage
/// `0x14017DC50` takes kind 7 as a plain hit; Re:CoM's own Stun Impact row (attack row 0xB) carries
/// kind 7. Checked (decomp + exe).
pub const RECOM_STUN_KIND: u8 = 7;

/// The Re:CoM kind for a KH1 record: the stun element → Re:CoM's stun; else by KH1's reaction.
pub fn recom_kind_of(data: &AttackData) -> u8 {
    if data.element == KH1_STUN_ELEMENT {
        RECOM_STUN_KIND
    } else {
        recom_kind(data.kind)
    }
}

/// The KH1 row in Re:CoM's layout. `recom` is the Re:CoM row of the same clip.
pub fn kh1_row_bytes(recom: &[u8; 16], kh1_row: usize, m: &Moveset) -> Option<[u8; 16]> {
    let rw = m.row(kh1_row)?;
    let info = m.motions.info(rw.request as u16)?;
    row_bytes_from(recom, info, m.attack(rw.attack)?)
}

/// [`kh1_row_bytes`] for a motion and a record given.
pub fn row_bytes_from(recom: &[u8; 16], info: &kh1_core::MotionInfo, data: &AttackData) -> Option<[u8; 16]> {
    let slot = data.hit_slot & 3;
    let hit = info.flags.iter().find(|w| w.kind == FlagKind::Hit(slot))?;
    let mut b = *recom;
    let lead = |v: f32| (v as i32 - HIT_LEAD as i32).clamp(0, 255) as u8;
    b[4] = lead(hit.start);
    b[5] = lead(hit.end);
    match info.flags.iter().find(|w| w.kind == FlagKind::ComboWindow) {
        Some(w) => {
            b[1] = w.start as u8;
            b[3] = if w.end.is_finite() { w.end as u8 } else { 255 };
        }
        None => {
            b[1] = 0;
            b[3] = 0;
        }
    }
    // No homing (KH1 sets the facing once), no stop distance, no homing speed.
    b[6] = 0;
    b[7] = 0;
    b[8] = 0;
    b[9] = 0;
    b[0xA] = 0;
    b[0xB] = recom_kind_of(data);
    b[0xC] = 0;
    b[0xD] = 0;
    // Not chainable by Re:CoM's own gate, no early cancel, no push: KH1's window decides.
    b[0xF] = FLAG_KH1;
    Some(b)
}

/// What Re:CoM's pick reads about the target, from KH1's target part (KH1 units, y down) in metres.
/// The part's centre is the target's vtable `+0x50` point (`win::attack::attack_target`), the one
/// Re:CoM's pick reads; `above` = its height over Riku's position minus 1.0 m (`+0x864 − +0x4B4 −
/// 1.0`), then the flat distance and the angle off his facing. Assumption: Riku's position stands for
/// `+0x4B4`, and `0x1401A3050`'s distance is flat (its body not read); the angle is wrapped to ±π.
pub fn riku_combo_target(t: TargetPart, pos: Vec3, facing: f32) -> crate::riku::ComboTarget {
    let m = crate::units::METRES_PER_UNIT;
    let d = t.centre - pos;
    let flat = d.flat();
    let angle = if flat.is_zero_xz() { 0.0 } else { kh1_core::math::wrap_angle(flat.yaw() - facing).abs() };
    crate::riku::ComboTarget { above: -d.y * m - 1.0, distance: flat.length() * m, angle }
}

/// KH1's Counterattack row (motion 213, record `0x4E18`).
pub const COUNTERATTACK_ROW: usize = 0x3E;

/// The KH1 rows by KH1 row id (0..=0x3E; unused rows are zero), from Re:CoM's rows 1–7. Rows that
/// play a clip of their own (the Counterattack, row 0x3E, and the combo modifiers,
/// [`MODIFIER_ROWS`]) take its action row from `clip_row(KH1 request)`: an appended row for a
/// KH1-only clip, or Re:CoM's own row of its bake (`rows::Extension::get`); without one the row
/// stays empty (not offered).
pub fn kh1_table(recom_rows_1_7: &[[u8; 16]; 7], m: &Moveset, clip_row: &dyn Fn(u16) -> Option<u32>) -> Vec<[u8; 16]> {
    let mut t = vec![[0u8; 16]; COUNTERATTACK_ROW + 1];
    for &(r, id) in ROW_MAP.iter() {
        if let Some(b) = kh1_row_bytes(&recom_rows_1_7[id as usize - 1], r, m) {
            t[r] = b;
        }
    }
    let own = MODIFIER_ROWS.iter().copied().chain(std::iter::once((COUNTERATTACK_ROW, 1)));
    for (r, id) in own {
        let Ok(req) = u16::try_from(m.row_request(r)) else { continue };
        if let Some(clip) = clip_row(req).filter(|c| *c <= 0xFF) {
            if let Some(mut b) = kh1_row_bytes(&recom_rows_1_7[id as usize - 1], r, m) {
                b[0] = clip as u8;
                t[r] = b;
            }
        }
    }
    t
}

/// M9 stage 9c.1: the specials' rows into the table (Sonic Blade `0x1B..=0x21`; M12 step 7, Strike
/// Raid `0x2F..=0x33`): their motions are the xl set's (`kh1_core::data::xl`, not Sora's own
/// requests 0xC9 / 0xCA / 0xE7 / 0xE8), their clip rows `xl_row(xl entry)` (the asset step's
/// converted clips); the layout a copy of Re:CoM's swing 1 (dashes, throws) or finisher (row 4,
/// each special's last row) as the modifiers'. Re:CoM's attack id is written by the special's row
/// start (`specials::recom`), not taken from the row. Without the clip the rows stay empty.
pub fn add_special_rows(t: &mut [[u8; 16]], recom_rows_1_7: &[[u8; 16]; 7], m: &Moveset, xl_row: &dyn Fn(usize) -> Option<u32>) {
    for rows in crate::specials::ROWS_OF {
        for r in rows.clone() {
            let Some(rw) = m.row(r).filter(|_| r < t.len()) else { continue };
            let Some(e) = kh1_core::data::xl::entry_of_kh1_request(rw.request) else { continue };
            let Some(clip) = xl_row(e).filter(|c| *c <= 0xFF) else { continue };
            let template = &recom_rows_1_7[if r == *rows.end() { 3 } else { 0 }];
            if let Some(mut b) = m.attack(rw.attack).and_then(|d| row_bytes_from(template, &kh1_core::data::xl::motion(e)?, d)) {
                b[0] = clip as u8;
                t[r] = b;
            }
        }
    }
}

/// Whether the table holds every row of special `index` (`crate::specials::ROWS_OF`).
pub fn special_rows_ready(t: &[[u8; 16]], index: usize) -> bool {
    crate::specials::ROWS_OF.get(index).is_some_and(|rows| !rows.is_empty() && rows.clone().all(|r| t.get(r).is_some_and(|b| b[0] != 0)))
}

/// The modifiers whose rows the table holds, as `abilities::ix` bits (the pickers only get those).
pub fn modifier_rows_ready(t: &[[u8; 16]]) -> crate::abilities::Bits {
    use crate::abilities::ix;
    let ready = |r: usize| t.get(r).is_some_and(|b| b[0] != 0);
    [
        (ix::SLIDING_DASH, row::SLIDING_DASH),
        (ix::BLITZ, row::BLITZ),
        (ix::STUN_IMPACT, row::STUN_IMPACT),
        (ix::ZANTETSUKEN, row::ZANTETSUKEN),
        (ix::VORTEX, row::VORTEX),
        (ix::AERIAL_SWEEP, row::AERIAL_SWEEP),
        (ix::HURRICANE_BLAST, row::HURRICANE_BLAST),
        (ix::RIPPLE_DRIVE, row::RIPPLE_DRIVE),
        (ix::GRAVITY_BREAK, row::GRAVITY_BREAK),
    ]
    .iter()
    .filter(|&&(_, r)| ready(r))
    .fold(0, |b, &(i, _)| b | 1 << i)
}

/// A row's KH1 combo window (`+1` / `+3` with flag `0x80`), in frames.
pub fn row_window(b: &[u8; 16]) -> Option<(f32, f32)> {
    (b[0xF] & FLAG_KH1 != 0 && b[3] > b[1]).then(|| (b[1] as f32, if b[3] == 255 { f32::INFINITY } else { b[3] as f32 }))
}

/// Whether a KH1 request is one of the swings the rows play (Re:CoM owns its motion).
pub fn is_swing_request(r: u16) -> bool {
    (200..=211).contains(&r) || (214..=218).contains(&r)
}

/// Whether the swing's clip moves Sora by root motion (KH1's request map bit; the air swings have
/// none).
pub fn swing_root(m: &Moveset, r: u16) -> bool {
    m.motions.info(r).is_some_and(|i| i.root_motion)
}

// ---- the Keyblade volume ----

/// The swing's attack object numbers for Sora with the Kingdom Key (`0x1402B2DC0` +
/// `0x1402B3270`; actor scale 1.0).
pub fn live_attack(m: &Moveset, kh1_row: usize, weapon: &WeaponData) -> Option<LiveAttack> {
    let rw = m.row(kh1_row)?;
    let data: &AttackData = m.attack(rw.attack)?;
    Some(LiveAttack::new(data, weapon, &AttackerStats::default(), 1.0))
}

/// KH1's blade points A and B on Re:CoM's Keyblade. KH1's weapon line is the segment from bone 282
/// (t = 0) to bone 283 (t = 1); Re:CoM's Keyblade bones `wep0` (2) and `k9` (3) are KH1's 282 / 283
/// × 0.01 (`docs/kh1-recom-assets.md`, Checked (files) ≤ 0.012 mm). `w2` / `w3` are their world
/// origins. Kingdom Key: A at t = −2.96, B at t = 0 (`kh1-anim` `weapon.rs`).
pub fn blade_points(w2: Vec3, w3: Vec3, live: &LiveAttack, weapon: &WeaponData) -> (Vec3, Vec3) {
    let (ta, tb) = live.blade_t(weapon);
    (w2.lerp(w3, ta), w2.lerp(w3, tb))
}

/// KH1's weapon bone id in attack records (`0x30`): the blade line ([`blade_points`]).
pub const KH1_WEAPON_BONE: i32 = 0x30;

/// The Re:CoM Sora bone (`PL0001.MDL` / `PL0011.MDL` index) standing for a KH1 Sora bone a
/// non-blade record is centred on (`0x1402B2DC0` reads the record's bone): Stun Impact's bone 8 →
/// Re:CoM's `belt1` (4), the bone at the same rest place and orientation (Checked (files:
/// `skel_map.py`, KH1 bones 7 and 8 coincide at rest)). That the posed bone 8 stays on Re:CoM's
/// `belt1` in the clip is an Assumption (KH1's bone 8 is a flags-1 bone with no offset from 7; a
/// 2.5 m sphere hides any difference). Other bones: none known (the swing keeps Re:CoM's test).
pub fn recom_body_bone(kh1_bone: i32) -> Option<i32> {
    match kh1_bone {
        8 => Some(4),
        _ => None,
    }
}

/// This frame's spheres (centre, radius), in the units of the points given: the chain from A to
/// B, and (when the record sweeps) the chain from A back to last frame's A, or to the owner's
/// centre on the first tested frame (`0x1402B3800` / `0x1402B3D50`, `kh1_core::shape`).
pub fn blade_shapes(a: Vec3, b: Vec3, radius: f32, sweeps: bool, prev_a: Option<Vec3>, owner_centre: Vec3) -> Vec<(Vec3, f32)> {
    let mut out = Vec::new();
    if sweeps {
        out.extend(sphere_chain(a, radius, prev_a.unwrap_or(owner_centre), radius, false).into_iter().map(|s| (s.centre, s.radius)));
    }
    out.extend(sphere_chain(a, radius, b, radius, false).into_iter().map(|s| (s.centre, s.radius)));
    out
}

/// The live blade across the frames of one swing: KH1's `prev_a` (attack object `+0x70`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BladeTrack {
    /// The swing (Re:CoM's collision serial, `coll+0x70`).
    pub serial: i32,
    /// This frame's A, and the last tested frame's.
    pub cur_a: Option<Vec3>,
    pub prev_a: Option<Vec3>,
}

impl BladeTrack {
    /// Re:CoM may test a frame's volume more than once (two body lists, `0x14016F110`): the
    /// previous A moves on only when A itself changed (a new frame's pose). A new swing (serial)
    /// seeds from the owner again.
    pub fn frame(&mut self, serial: i32, a: Vec3) -> Option<Vec3> {
        if serial != self.serial {
            *self = BladeTrack { serial, cur_a: Some(a), prev_a: None };
        } else if self.cur_a != Some(a) {
            self.prev_a = self.cur_a;
            self.cur_a = Some(a);
        }
        self.prev_a
    }
}

// ---- KH1's Attack command in the handler frame ----

impl Kh1Sora {
    /// The combo window bit of the playing motion (KH1 `+0x1CC & 0x400000`).
    pub fn in_window(&self) -> bool {
        self.rec.flags & flag::COMBO_WINDOW != 0
    }

    /// `0x140293480` for the Attack command: not in the take-off, and free or in the window.
    pub fn accepts_attack(&self) -> bool {
        self.state != state::TAKE_OFF && (!self.lock || self.in_window())
    }

    /// `0x1402A7510`, command type 1 (Attack): KH1's counter rules, its picker, then the row exec.
    /// Re:CoM starts the swing through [`Host::start_swing`].
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn command_attack(&mut self, h: &mut dyn Host, dir: &mut Vec3, w: &mut f32, target: Option<TargetPart>, locked: bool, pos: Vec3, ev: &mut Vec<Event>) {
        let lock = if locked { target } else { None };
        match lock {
            Some(t) => {
                let d = (t.centre - pos).flat().normalized();
                *dir = d;
                self.target_yaw = d.yaw();
            }
            None => {
                if dir.is_zero_xz() {
                    *dir = Vec3::from_yaw(self.facing);
                }
            }
        }
        *w = 0.0;
        if !self.in_window() {
            self.combo_counter = 1;
        } else {
            if !h.swing_hit() && !self.abilities.combo_master {
                ev.push(Event::AttackRefused(Refused::LastSwingMissed));
                return;
            }
            self.combo_counter = self.combo_counter.wrapping_add(1);
        }
        let current = self.rec.req as i32;
        let air = self.state == state::AIR;
        let length = if air { self.params.air_length(&self.abilities) } else { self.params.ground_length(&self.abilities) };
        let finisher = length <= self.combo_counter;
        // Without a lock the scan sees Re:CoM's current target (Assumption: not every enemy; M12).
        let parts: Vec<TargetPart> = target.into_iter().collect();
        if let Some(dark) = self.riku_moves {
            // Riku: Re:CoM's own pick of his next swing (`crate::riku::situation`) on KH1's counter.
            self.riku_attack(h, dir, w, target, pos, dark, finisher, length, ev);
            return;
        }
        let picked = match self.state {
            state::GROUND => {
                let ctx = GroundCtx {
                    params: &self.params,
                    abilities: &self.abilities,
                    moveset: &self.moveset,
                    counter: self.combo_counter,
                    length,
                    current_request: current,
                    pos,
                    aim: *dir,
                    target: lock,
                    parts: &parts,
                };
                let rng = &mut self.rng;
                // Assumption: no obstacle probe yet (the hop row 2 needs `0x1402C6E70`'s wall test).
                let g = pick_ground(&ctx, &mut || rng.next_f32(), &mut |_| false);
                *dir = Vec3::new(g.aim.x, g.hop, g.aim.z);
                *w = g.speed;
                g.row
            }
            state::AIR => {
                let ctx = AirCtx { params: &self.params, abilities: &self.abilities, moveset: &self.moveset, counter: self.combo_counter, length, current_request: current, in_state_2: true, pos, aim: *dir, target: lock };
                let (r, aim) = pick_air(&ctx);
                *dir = aim;
                r
            }
            _ => {
                ev.push(Event::AttackRefused(Refused::NoRows));
                return;
            }
        };
        self.row_exec(h, picked, dir, w, finisher, length, pos, ev);
    }

    /// Riku's swing (fork owner, 2026-11-27: KH1's system on Riku's own moves): Re:CoM's situation
    /// (`crate::riku::situation`, `0x1401F7520` / Dark `0x140204F80`) from his last swing and the target,
    /// on KH1's combo counter and length; KH1's row of that clip; KH1's row exec. His rising swing from
    /// the ground leaves it as KH1's jump-up (its hop toward the target). Sora's combo modifiers are not
    /// offered (they are Sora's clips).
    #[allow(clippy::too_many_arguments)]
    fn riku_attack(&mut self, h: &mut dyn Host, dir: &mut Vec3, w: &mut f32, target: Option<TargetPart>, pos: Vec3, dark: bool, finisher: bool, length: u8, ev: &mut Vec<Event>) {
        let airborne = self.state == state::AIR;
        if self.combo_counter <= 1 {
            self.riku_prev_sit = 0;
        }
        let t = target.map(|t| riku_combo_target(t, pos, self.facing));
        let sit = crate::riku::situation(self.riku_prev_sit, finisher, airborne, dark, t);
        let r = crate::riku::situation_row(sit, airborne);
        if let Some(t) = target {
            let (aim, len) = (t.centre - pos).flat().normalize_len();
            if len > 0.0 {
                *dir = aim;
                self.target_yaw = aim.yaw();
            }
        }
        *w = 0.0;
        if r == row::JUMP_UP {
            // KH1's jump-up hop toward the target (`pick_ground`'s jump-up branch numbers).
            let p = &self.params.combo;
            let (hop, speed) = match target {
                Some(t) => {
                    let d = t.centre - pos;
                    let gap = d.flat().length() - t.radius;
                    let s = ((gap - p.jump_up_speed_offset) / p.jump_up_speed_div).clamp(-p.jump_up_speed_cap, p.jump_up_speed_cap);
                    let reach = self.params.reach(&self.abilities);
                    let hh = -d.y - p.jump_up_hop_offset;
                    (hh.min(reach).max(p.jump_up_hop_min), s)
                }
                None => (p.jump_up_hop_min, 0.0),
            };
            *dir = Vec3::new(dir.x, hop, dir.z);
            *w = speed;
        } else if airborne {
            *dir = Vec3::new(dir.x, 0.0, dir.z);
        }
        self.riku_prev_sit = sit;
        self.row_exec(h, r, dir, w, finisher, length, pos, ev);
    }

    /// `0x1402A7510`, command type 99 (Counterattack, command 0x63): CB+0x41 + 1, then row 0x3E
    /// (`0x1402A68F0`). The command aims at its target (entry byte 0 bit 2, `0x1402A38E0`), else
    /// the stick, else the facing. Checked (decomp + files).
    pub(crate) fn command_counterattack(&mut self, h: &mut dyn Host, dir: &mut Vec3, w: &mut f32, target: Option<TargetPart>, pos: Vec3, ev: &mut Vec<Event>) {
        match target {
            Some(t) => {
                let d = (t.centre - pos).flat().normalized();
                *dir = d;
                self.target_yaw = d.yaw();
            }
            None => {
                if dir.is_zero_xz() {
                    *dir = Vec3::from_yaw(self.facing);
                }
            }
        }
        *w = 0.0;
        self.combo_counter = self.combo_counter.wrapping_add(1);
        let length = self.params.ground_length(&self.abilities);
        self.row_exec(h, COUNTERATTACK_ROW, dir, w, false, length, pos, ev);
    }

    /// `0x1402A68F0` (with `0x1402A3980`, the row reader).
    #[allow(clippy::too_many_arguments)]
    fn row_exec(&mut self, h: &mut dyn Host, r: usize, dir: &mut Vec3, w: &mut f32, finisher: bool, length: u8, pos: Vec3, ev: &mut Vec<Event>) {
        let Some(rw) = self.moveset.row(r).cloned() else { return };
        // `0x1402B3270`: each row start clears CB+0x50 (the clash bit).
        self.clashed = false;
        let mut hop = rw.hop;
        let mut counter_reset = false;
        let air_row = matches!(r, row::HURRICANE_BLAST..=row::AIR_SWING_1);
        match r {
            row::AERIAL_SWEEP | row::JUMP_UP | row::HOP => {
                if r == row::HOP {
                    *w = self.params.combo.hop_speed;
                } else {
                    hop = dir.y;
                }
                counter_reset = true;
            }
            _ if air_row => {
                *dir = dir.normalized();
                let c = &self.params.combo;
                let v = if self.combo_counter == 1 {
                    c.air_speed_first
                } else if r == row::HURRICANE_BLAST || r == row::AIR_FINISHER {
                    c.air_speed_finisher
                } else {
                    c.air_speed_middle
                };
                self.air_vel = *dir * v;
                *w = 0.0;
            }
            _ => *w = 0.0,
        }
        // The row reader: the swing's motion (Re:CoM plays it) and its attack object.
        let request = rw.request as u16;
        let start = SwingStart {
            kh1_row: r,
            recom_id: recom_id(r).unwrap_or(1),
            request,
            root: swing_root(&self.moveset, request),
            counter: if counter_reset { 1 } else { self.combo_counter },
            length,
            finisher,
        };
        self.play(request as u32 | req::ACTION, 0.0);
        self.rec.issued = true;
        self.rec.issued_ok = h.start_swing(&start);
        self.swing = Some(start);
        let end = if matches!(r, row::AERIAL_SWEEP | row::JUMP_UP | row::HOP) || air_row { request::FALL } else { request::IDLE };
        self.append(req::RELEASE | end as u32, 0.0);
        if counter_reset {
            self.combo_counter = 1;
        }
        if 0.0 < hop {
            self.hop(hop, pos.y);
        }
        self.target_yaw = dir.yaw();
        ev.push(Event::AttackStarted { row: r, request: rw.request, counter: self.combo_counter, finisher });
    }

    /// The swing's motion is no longer KH1's playing motion (it ended into its queued idle / fall,
    /// a landing or a walk-off replaced it): Re:CoM ends its attack action (`0x1401F6660`), and
    /// the KH1 motion that follows is played by the caller.
    pub(crate) fn leave_swing(&mut self, h: &mut dyn Host) {
        if self.swing.take().is_some() && h.end_swing(self.state != state::GROUND) {
            self.recom_took = true;
        }
    }

    /// Re:CoM's action for a playing swing.
    pub(crate) fn swing_action(&self) -> Option<crate::movement::Action> {
        let s = self.swing.as_ref()?;
        (self.rec.head() == s.request).then_some(crate::movement::Action { id: action::ATTACK, sub: 0, phase: ATTACK_PHASE })
    }
}

/// The Battle exe has no case for action `0xA` (its jump table `0x1401FA050` sends it to the
/// default `0x1401F9CC9`). Its entry is pointed at a stub in an int3 cave that runs Re:CoM's field
/// swing exe on the player (`rdi` in the Battle exe's switch) and goes on at the default, as the
/// table's other cases do (`mov rcx,rdi; call exe; jmp default`). Checked (disasm `0x1401F9959`).
pub mod battle_case {
    pub const TABLE_ENTRY_RVA: u32 = 0x001F_A074;
    pub const CAVE_RVA: u32 = 0x001F_A6C1;
    pub const SWING_EXE_RVA: u32 = 0x001E_AEB0;
    pub const DEFAULT_RVA: u32 = 0x001F_9CC9;

    /// The 13-byte stub at the cave.
    pub fn stub() -> [u8; 13] {
        let mut b = [0u8; 13];
        b[..3].copy_from_slice(&[0x48, 0x8B, 0xCF]);
        b[3] = 0xE8;
        let call = SWING_EXE_RVA as i64 - (CAVE_RVA as i64 + 8);
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

// ---- KH1 enemy stats (decisions 12, 21) ----

/// One KH1 twin record as the asset step resolves it (`kh1-stats.json`; the 0x6C-byte record
/// KH1's AI hands to syscall 45, `0x1402BBDF0` → `0x1402A58E0`, with Strength / Defence already
/// looked up in btltbl `0xC8A8`). Checked (decomp + files).
#[derive(Clone, Debug, PartialEq)]
pub struct EnemyStats {
    pub hp: i32,
    pub strength: i32,
    pub defence: i32,
    /// Damage floor / cap per hit (stats `+0xC0` / `+0xC2`; 0 = none).
    pub min_damage: u16,
    pub max_damage: u16,
    pub level: i32,
    pub exp: i32,
    /// Per element, % (stats `+0x78..`).
    pub resist: [i32; 16],
}

/// The KH1 swing damage against a KH1 twin's stats (`0x1402BFA80` through kh1-core), before
/// Re:CoM's extras (back bonus, weak part, boss falloff) which its own damage path applies. The
/// twin at full HP, Sora at KH1's default max MP (records of swings read neither).
pub fn kh1_damage(m: &Moveset, kh1_row: usize, strength: i32, weapon: &WeaponData, twin: &EnemyStats, rng: &mut kh1_core::Kh1Rand) -> Option<kh1_core::DamageResult> {
    kh1_damage_at(m, kh1_row, Sora { strength, max_mp: AttackerStats::default().max_mp }, weapon, twin, twin.hp, rng)
}

/// Sora's numbers a KH1 record can read: Strength (stat source 0) and max MP (sources 1, 3, 4, 5:
/// Ripple Drive's Magic × 3 is source 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sora {
    pub strength: i32,
    pub max_mp: i32,
}

/// [`kh1_damage`] with Sora's max MP and the enemy's current HP (`hp_now`), which the % of HP
/// element (8: Gravity Break, Zantetsuken) reads with the record's `resist[8]` as its scale
/// (stats `+0x98`, as the Gravity spell, `win::stats::kh1_spell_hit`).
pub fn kh1_damage_at(m: &Moveset, kh1_row: usize, sora: Sora, weapon: &WeaponData, twin: &EnemyStats, hp_now: i32, rng: &mut kh1_core::Kh1Rand) -> Option<kh1_core::DamageResult> {
    kh1_damage_with(m, kh1_row, sora, 0, weapon, twin, hp_now, rng)
}

/// [`kh1_damage_at`] with a Strength bonus on Strength-source records (M12 step 3, Berserk: KH1's
/// attack object `0x1402B2DC0` case 0 adds G`+0xFE` = 4 to the stat term; kh1-core's
/// `AttackerStats::strength_bonus`). `bonus` 0 = none.
#[allow(clippy::too_many_arguments)]
pub fn kh1_damage_with(m: &Moveset, kh1_row: usize, sora: Sora, bonus: i32, weapon: &WeaponData, twin: &EnemyStats, hp_now: i32, rng: &mut kh1_core::Kh1Rand) -> Option<kh1_core::DamageResult> {
    let (live, stats) = kh1_attack_on(m, kh1_row, sora, bonus, weapon, twin, hp_now)?;
    Some(kh1_core::damage::compute_damage(&live, &kh1_core::DamageCurve::kh1_defaults(), &stats, rng))
}

/// [`kh1_damage_with`] with the enemy's per-hit cap left for Re:CoM's join (`crate::damage_cap`):
/// the number before the cap, and the cap in its units (after KH1's resistance), None for the % of
/// HP rows and an enemy without a cap (`kh1_core::damage::compute_damage_cap_later`).
#[allow(clippy::too_many_arguments)]
pub fn kh1_damage_cap_later(m: &Moveset, kh1_row: usize, sora: Sora, bonus: i32, weapon: &WeaponData, twin: &EnemyStats, hp_now: i32, rng: &mut kh1_core::Kh1Rand) -> Option<(kh1_core::DamageResult, Option<f32>)> {
    let (live, stats) = kh1_attack_on(m, kh1_row, sora, bonus, weapon, twin, hp_now)?;
    Some(kh1_core::damage::compute_damage_cap_later(&live, &kh1_core::DamageCurve::kh1_defaults(), &stats, rng))
}

/// The attack object (`0x1402B2DC0` + `0x1402B3270`) and the defender's block that
/// [`kh1_damage_with`] hands to KH1's damage (`0x1402BFA80`).
pub fn kh1_attack_on(m: &Moveset, kh1_row: usize, sora: Sora, bonus: i32, weapon: &WeaponData, twin: &EnemyStats, hp_now: i32) -> Option<(LiveAttack, kh1_core::DefenderStats)> {
    let rw = m.row(kh1_row)?;
    let data = m.attack(rw.attack)?;
    let attacker = AttackerStats { strength: sora.strength, max_mp: sora.max_mp, strength_bonus: (bonus != 0).then_some(bonus), ..Default::default() };
    let live = LiveAttack::new(data, weapon, &attacker, 1.0);
    let mut resist = [1.0f32; 16];
    for (r, v) in resist.iter_mut().zip(twin.resist.iter()) {
        *r = *v as f32 / 100.0;
    }
    let stats = kh1_core::DefenderStats {
        hp: hp_now.max(0),
        max_hp: twin.hp,
        defence: twin.defence,
        level: twin.level,
        min_damage: twin.min_damage,
        max_damage: twin.max_damage,
        resist,
        hp_percent_scale: resist[8],
        ..Default::default()
    };
    Some((live, stats))
}

/// Damage = Re:CoM (owner, 2026-11-01): Gravity Break and Zantetsuken take **KH1's percentage of
/// Re:CoM's HP** (`crate::damage::percent_of_hp_row`): [`kh1_damage_at`] with `hp_now` = the
/// enemy's Re:CoM current HP, the KH1 record's `resist[8]` and floor (its twin / proxy / boss twin:
/// KH1's boss rule), or, with no record, 100 % and no floor (1), as the Gravity spell's fallback.
/// None for any other row.
pub fn kh1_percent_of_hp(m: &Moveset, kh1_row: usize, sora: Sora, weapon: &WeaponData, record: Option<&EnemyStats>, hp_now: i32, rng: &mut kh1_core::Kh1Rand) -> Option<i32> {
    kh1_percent_of_hp_with(m, kh1_row, sora, 0, weapon, record, hp_now, rng)
}

/// [`kh1_percent_of_hp`] with a Strength bonus (Berserk, [`kh1_damage_with`]): Gravity Break's stat
/// term is Strength, so + 4 % of the HP.
#[allow(clippy::too_many_arguments)]
pub fn kh1_percent_of_hp_with(m: &Moveset, kh1_row: usize, sora: Sora, bonus: i32, weapon: &WeaponData, record: Option<&EnemyStats>, hp_now: i32, rng: &mut kh1_core::Kh1Rand) -> Option<i32> {
    if !crate::damage::percent_of_hp_row(kh1_row, m) {
        return None;
    }
    let plain = plain_record(hp_now);
    kh1_damage_with(m, kh1_row, sora, bonus, weapon, record.unwrap_or(&plain), hp_now, rng).map(|d| d.damage)
}

/// A KH1 record for an enemy without one (no twin, no proxy): no Defence, no floor or cap, 100 %
/// everywhere (the Gravity spell's fallback).
pub fn plain_record(hp: i32) -> EnemyStats {
    EnemyStats { hp, strength: 0, defence: 0, min_damage: 0, max_damage: 0, level: 1, exp: 0, resist: [100; 16] }
}

/// One Keyblade Sora can equip at the KH1 setting (decision 25), keyed by Re:CoM's Keyblade card
/// kind (1 Kingdom Key … 0x18 Bond of Flame; table `0x1405256D0`).
#[derive(Clone, Debug, PartialEq)]
pub struct Keyblade {
    pub card: u16,
    pub name: String,
    /// Where its numbers and model come from: `kh1` (KH1's weapon row and model) or `kh2` (KH2's
    /// model, Strength and Magic, reach from its blade, the rest the Kingdom Key's KH1 row).
    pub source: String,
    pub model: String,
    pub weapon: WeaponData,
    /// KH1 weapon row `+0x38`: the Magic bonus.
    pub magic: i32,
    /// Its model's Re:CoM PLAYER resource group (format 4; `keyblade_models`): None for a
    /// Keyblade without a converted model, and in older files.
    pub group: Option<u16>,
    /// Its model's texture names (format 4), for the log.
    pub textures: Vec<String>,
}

/// `kh1-stats.json` (written by the asset step, `kh1-recom-assets` `stats`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Kh1Stats {
    /// Sora's KH1 Strength at levels 1..=100 with the Kingdom Key (keep Rod, give up Shield;
    /// decision 18).
    pub sora_strength: Vec<i32>,
    /// The same without a Keyblade (format 3): the equipped one's Strength is added.
    pub base_strength: Vec<i32>,
    /// The Keyblades with KH1 numbers (format 3).
    pub keyblades: Vec<Keyblade>,
    /// Re:CoM class name → its twin's record per Re:CoM world id 1..=13.
    pub enemies: Vec<(String, Vec<EnemyStats>)>,
    /// Re:CoM class name → its twin's first kind-0 lock-point radius (KH1 units), the radius
    /// KH1's ground picker subtracts (`0x1402A7320`). Absent in older files (format 1).
    pub lock_radius: Vec<(String, f32)>,
    /// Re:CoM class name → its twin's Re:CoM HP per floor row (`@ENP`, format 5), for the log's
    /// check against the live row.
    pub recom_hp: Vec<(String, Vec<i32>)>,
    /// Re:CoM's bosses with a KH1 equivalent (format 5).
    pub bosses: Vec<BossTwin>,
    /// The proxy tables for enemies without a twin (format 5; None before).
    pub proxies: Option<Proxies>,
    /// Re:CoM class name → its twin's KH1 defeat drops per Re:CoM world id 1..=13 (format 6,
    /// M8 stage 8a.4; absent: no drops).
    pub drops: Vec<(String, Vec<Vec<crate::drops::Drop>>)>,
}

// ---- KH1 numbers for Re:CoM enemies without a KH1 twin (proxies) ----

/// A Re:CoM boss with a KH1 equivalent (`kh1-stats.json` format 5 `bosses`): it takes its own KH1
/// record per Re:CoM world, as an ordinary twin does. Matched by class and Re:CoM id (some
/// sub-objects init with their boss's id, e.g. a Jafar-family class with 108).
#[derive(Clone, Debug, PartialEq)]
pub struct BossTwin {
    pub id: u16,
    pub class: String,
    pub name: String,
    pub worlds: Vec<EnemyStats>,
}

/// One proxy cell: the reference's KH1 HP ÷ Re:CoM HP and its record (Defence, level, floor /
/// cap, resistances; the HP is the reference's).
#[derive(Clone, Debug, PartialEq)]
pub struct ProxyCell {
    pub ratio: f64,
    pub from: String,
    pub record: EnemyStats,
}

/// The proxy tables, `[world − 1][floor row]` (13 × 13).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Proxies {
    pub ordinary: Vec<Vec<Option<ProxyCell>>>,
    pub boss: Vec<Vec<Option<ProxyCell>>>,
}

/// What an enemy without an ordinary twin takes at the KH1 setting.
#[derive(Clone, Debug, PartialEq)]
pub struct Proxy {
    /// The KH1 record it fights with; `hp` is its KH1 HP.
    pub stats: EnemyStats,
    /// KH1 HP ÷ Re:CoM HP.
    pub ratio: f64,
    /// Where the numbers come from (the reference twin or boss twin).
    pub from: String,
    pub boss: bool,
    /// A boss twin (its own KH1 record), not a scaled proxy.
    pub own: bool,
}

/// Rows in the proxy tables (Sora's floors 1..13).
pub const PROXY_ROWS: usize = 13;

/// Re:CoM's floor id (`0x14087B844`) → the `@ENP` row its stats init reads (`0x14017FF50`): floor
/// − 1, 0xE → 0, 0xF → 2, 100 (100 Acre Wood) → 0, below 1 → 0. Riku's floors (< 0) → None (his
/// story keeps Re:CoM's numbers). Checked (decomp).
pub fn enp_row(floor: i16) -> Option<usize> {
    let row = match floor {
        f if f < 0 => return None,
        0xE | 100 => 0,
        0xF => 2,
        f => (f - 1).max(0) as usize,
    };
    (row < PROXY_ROWS).then_some(row)
}

/// The Re:CoM world a proxy scales for: 1..13 as is; 0 (no world: Castle Oblivion's floor halls,
/// where Larxene, Riku, Vexen and Axel are fought) → 13, Castle Oblivion. Assumption: the getter
/// answers 0 in the halls (its fallback); the log shows the world each proxy used.
pub fn proxy_world(world: i32) -> Option<usize> {
    match world {
        0 => Some(13),
        1..=13 => Some(world as usize),
        _ => None,
    }
}

/// Re:CoM's boss rule (`0x1401805A0`: id / 100 ≥ 1; the boss falloff in `0x14017F7E0` tests id
/// 100..199). Checked (decomp).
pub fn is_boss_id(id: u16) -> bool {
    id >= 100
}

/// Ids that never take a proxy: 100 Acre Wood's minigame Beehive (37) and Bee (38) (factory
/// `0x140188F40`; Re:CoM's minigame stays as it is).
pub const NO_PROXY_IDS: [u16; 2] = [37, 38];

/// A proxy's KH1 HP: its Re:CoM row HP (before Re:CoM's difficulty %, as the twins' KH1 HP) ×
/// the ratio, rounded, at least 1.
pub fn proxy_hp(recom_hp: i32, ratio: f64) -> i32 {
    ((recom_hp as f64 * ratio).round() as i32).max(1)
}

impl Kh1Stats {
    /// The KH1 numbers for a Re:CoM enemy without an ordinary twin: its class (RTTI name) and id,
    /// Re:CoM's world and floor, and its Re:CoM row HP (`*(enemy+0x310)`). A boss twin takes its
    /// own record; other bosses the world's boss cell; ordinary enemies the ordinary cell.
    pub fn proxy(&self, class: &str, id: u16, world: i32, floor: i16, recom_hp: i32) -> Option<Proxy> {
        if NO_PROXY_IDS.contains(&id) {
            return None;
        }
        let w = proxy_world(world)?;
        // Riku's floors (< 0) never reach here (his story keeps Re:CoM's numbers); refuse them anyway.
        let row = enp_row(floor)?;
        let boss = is_boss_id(id);
        if boss {
            if let Some(b) = self.bosses.iter().find(|b| b.id == id && b.class == class) {
                let stats = b.worlds.get(w - 1)?.clone();
                let ratio = if recom_hp > 0 { stats.hp as f64 / recom_hp as f64 } else { 0.0 };
                return Some(Proxy { stats, ratio, from: b.name.clone(), boss, own: true });
            }
        }
        if recom_hp <= 0 {
            return None;
        }
        let p = self.proxies.as_ref()?;
        let cell = (if boss { &p.boss } else { &p.ordinary }).get(w - 1)?.get(row)?.as_ref()?;
        let mut stats = cell.record.clone();
        stats.hp = proxy_hp(recom_hp, cell.ratio);
        Some(Proxy { stats, ratio: cell.ratio, from: cell.from.clone(), boss, own: false })
    }

    /// A twin's Re:CoM HP on a floor row, from the asset step (format 5).
    pub fn twin_recom_hp(&self, class: &str, row: usize) -> Option<i32> {
        self.recom_hp.iter().find(|(c, _)| c == class)?.1.get(row).copied().filter(|h| *h > 0)
    }
}

impl Kh1Stats {
    /// Sora's KH1 Strength at a Re:CoM level (clamped to 1..=100).
    pub fn strength(&self, level: u32) -> Option<i32> {
        self.sora_strength.get(level.clamp(1, 100) as usize - 1).copied()
    }
    /// Sora's KH1 Strength at a Re:CoM level with this Keyblade's Strength bonus. Older files
    /// (no base Strength) only know the Kingdom Key's.
    pub fn strength_with(&self, level: u32, weapon: &WeaponData) -> Option<i32> {
        if self.base_strength.len() == 100 {
            return self.base_strength.get(level.clamp(1, 100) as usize - 1).map(|b| b + weapon.strength);
        }
        self.strength(level)
    }
    /// A Keyblade by Re:CoM's card kind.
    pub fn keyblade(&self, card: u16) -> Option<&Keyblade> {
        self.keyblades.iter().find(|k| k.card == card)
    }
    /// The twin record for a Re:CoM class in a Re:CoM world (1-based). Worlds outside 1..=13
    /// (none, special floors) give nothing: Re:CoM's own numbers stand.
    pub fn twin(&self, class: &str, world: i32) -> Option<&EnemyStats> {
        if !(1..=13).contains(&world) {
            return None;
        }
        self.enemies.iter().find(|(c, _)| c == class)?.1.get(world as usize - 1)
    }
    /// The twin's lock-point radius (KH1 units) for a Re:CoM class.
    pub fn lock_radius(&self, class: &str) -> Option<f32> {
        self.lock_radius.iter().find(|(c, _)| c == class).map(|(_, r)| *r)
    }
    /// The twin's KH1 defeat drops for a Re:CoM class in a Re:CoM world (1-based), as [`twin`]
    /// picks its record. None: no drops (no twin, no table, or a world outside 1..=13).
    ///
    /// [`twin`]: Kh1Stats::twin
    pub fn drops(&self, class: &str, world: i32) -> Option<&[crate::drops::Drop]> {
        if !(1..=13).contains(&world) {
            return None;
        }
        self.drops.iter().find(|(c, _)| c == class)?.1.get(world as usize - 1).map(|v| v.as_slice())
    }
}

/// Reads `kh1-stats.json`.
pub fn parse_stats(text: &str) -> Result<Kh1Stats, String> {
    let v = crate::json::parse(text)?;
    let ints = |x: &crate::json::Value, what: &str| -> Result<Vec<i32>, String> {
        x.as_array().ok_or_else(|| format!("`{what}` is not an array"))?.iter().map(|n| n.as_f64().map(|f| f as i32).ok_or_else(|| format!("`{what}`: not a number"))).collect()
    };
    let sora_strength = ints(v.get("sora_strength").ok_or("no `sora_strength`")?, "sora_strength")?;
    if sora_strength.len() != 100 {
        return Err(format!("`sora_strength` has {} levels, not 100", sora_strength.len()));
    }
    let base_strength = match v.get("sora_base_strength") {
        Some(b) => ints(b, "sora_base_strength")?,
        None => Vec::new(),
    };
    if !base_strength.is_empty() && base_strength.len() != 100 {
        return Err(format!("`sora_base_strength` has {} levels, not 100", base_strength.len()));
    }
    let mut keyblades = Vec::new();
    for k in v.get("keyblades").and_then(|k| k.as_array()).unwrap_or(&[]) {
        let name = k.get("name").and_then(|n| n.as_str()).ok_or("keyblade without `name`")?.to_string();
        let num = |f: &str| k.get(f).and_then(|x| x.as_f64()).ok_or_else(|| format!("{name}: no `{f}`"));
        keyblades.push(Keyblade {
            card: num("card")? as u16,
            source: k.get("source").and_then(|x| x.as_str()).unwrap_or("kh1").to_string(),
            model: k.get("model").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            weapon: WeaponData {
                radius: num("radius")? as f32,
                blade_far: num("blade_far")? as f32,
                blade_near: num("blade_near")? as f32,
                frac_scale: num("frac_scale")? as f32,
                strength: num("strength")? as i32,
                crit: num("crit")? as u8,
                crit_bonus: num("crit_bonus")? as i8,
                priority: num("priority")? as u8,
            },
            magic: num("magic")? as i32,
            group: match k.get("group") {
                None => None,
                Some(g) => Some(g.as_f64().filter(|x| (1.0..65535.0).contains(x) && x.fract() == 0.0).ok_or_else(|| format!("{name}: bad `group`"))? as u16),
            },
            textures: k.get("textures").and_then(|t| t.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default(),
            name,
        });
    }
    let mut enemies = Vec::new();
    let mut lock_radius = Vec::new();
    let mut recom_hp = Vec::new();
    let mut drops = Vec::new();
    for e in v.get("enemies").and_then(|e| e.as_array()).ok_or("no `enemies` array")? {
        let class = e.get("class").and_then(|c| c.as_str()).ok_or("enemy without `class`")?.to_string();
        let worlds = world_records(e, &class)?;
        if let Some(r) = e.get("lock_radius").and_then(|r| r.as_f64()) {
            lock_radius.push((class.clone(), r as f32));
        }
        if let Some(h) = e.get("recom_hp") {
            recom_hp.push((class.clone(), ints(h, "recom_hp")?));
        }
        if let Some(d) = e.get("drops") {
            drops.push((class.clone(), drop_tables(d, &class)?));
        }
        enemies.push((class, worlds));
    }
    // Format 5: the boss twins and the proxy tables.
    let mut bosses = Vec::new();
    for b in v.get("bosses").and_then(|b| b.as_array()).unwrap_or(&[]) {
        let class = b.get("class").and_then(|c| c.as_str()).ok_or("boss without `class`")?.to_string();
        let id = b.get("recom_id").and_then(|x| x.as_f64()).filter(|x| (0.0..65536.0).contains(x)).ok_or_else(|| format!("{class}: no `recom_id`"))? as u16;
        let name = b.get("name").and_then(|n| n.as_str()).unwrap_or(&class).to_string();
        bosses.push(BossTwin { id, worlds: world_records(b, &class)?, class, name });
    }
    let proxies = match v.get("proxies") {
        None => None,
        Some(p) => {
            let table = |k: &str| -> Result<Vec<Vec<Option<ProxyCell>>>, String> {
                let worlds = p.get(k).and_then(|t| t.as_array()).ok_or_else(|| format!("proxies: no `{k}`"))?;
                if worlds.len() != 13 {
                    return Err(format!("proxies.{k}: {} worlds, not 13", worlds.len()));
                }
                let mut out = Vec::new();
                for (w, row) in worlds.iter().enumerate() {
                    let cells = row.as_array().ok_or_else(|| format!("proxies.{k}[{w}]: not an array"))?;
                    if cells.len() != PROXY_ROWS {
                        return Err(format!("proxies.{k}[{w}]: {} rows, not {PROXY_ROWS}", cells.len()));
                    }
                    let mut r = Vec::new();
                    for c in cells {
                        if matches!(c, crate::json::Value::Null) {
                            r.push(None);
                            continue;
                        }
                        let who = format!("proxies.{k}[{w}]");
                        let ratio = c.get("ratio").and_then(|x| x.as_f64()).filter(|x| x.is_finite() && *x > 0.0).ok_or_else(|| format!("{who}: bad `ratio`"))?;
                        let from = c.get("from").and_then(|x| x.as_str()).unwrap_or("?").to_string();
                        let record = record_of(c.get("record").ok_or_else(|| format!("{who}: no `record`"))?, &who)?;
                        r.push(Some(ProxyCell { ratio, from, record }));
                    }
                    out.push(r);
                }
                Ok(out)
            };
            Some(Proxies { ordinary: table("ordinary")?, boss: table("boss")? })
        }
    };
    Ok(Kh1Stats { sora_strength, base_strength, keyblades, enemies, lock_radius, recom_hp, bosses, proxies, drops })
}

/// A twin's `drops` (format 6): 13 worlds, each a list of `[chance, item]` pairs (KH1's three, in
/// roll order). Chances must be in 0..=1, items 0..=255.
fn drop_tables(d: &crate::json::Value, class: &str) -> Result<Vec<Vec<crate::drops::Drop>>, String> {
    let worlds = d.as_array().ok_or_else(|| format!("{class}: `drops` is not an array"))?;
    if worlds.len() != 13 {
        return Err(format!("{class}: `drops` has {} worlds, not 13", worlds.len()));
    }
    let mut out = Vec::new();
    for (w, pairs) in worlds.iter().enumerate() {
        let mut t = Vec::new();
        for p in pairs.as_array().ok_or_else(|| format!("{class}: drops[{w}] is not an array"))? {
            let pair = p.as_array().filter(|a| a.len() == 2).ok_or_else(|| format!("{class}: drops[{w}]: not a [chance, item] pair"))?;
            let chance = pair[0].as_f64().filter(|c| c.is_finite() && (0.0..=1.0).contains(c)).ok_or_else(|| format!("{class}: drops[{w}]: bad chance"))?;
            let item = pair[1].as_f64().filter(|i| (0.0..=255.0).contains(i) && i.fract() == 0.0).ok_or_else(|| format!("{class}: drops[{w}]: bad item"))?;
            t.push(crate::drops::Drop { chance: chance as f32, item: item as u8 });
        }
        out.push(t);
    }
    Ok(out)
}

/// One KH1 record of `kh1-stats.json` (`hp`, `strength`, `defence`, `floor`, `cap`, `level`,
/// `exp`, `resist`).
fn record_of(w: &crate::json::Value, who: &str) -> Result<EnemyStats, String> {
    let num = |k: &str| w.get(k).and_then(|x| x.as_f64()).ok_or_else(|| format!("{who}: record without `{k}`"));
    let res = w.get("resist").and_then(|r| r.as_array()).ok_or_else(|| format!("{who}: no `resist`"))?;
    let mut resist = [100; 16];
    for (r, x) in resist.iter_mut().zip(res) {
        *r = x.as_f64().ok_or_else(|| format!("{who}: `resist`: not a number"))? as i32;
    }
    Ok(EnemyStats {
        hp: num("hp")? as i32,
        strength: num("strength")? as i32,
        defence: num("defence")? as i32,
        min_damage: num("floor")? as u16,
        max_damage: num("cap")? as u16,
        level: num("level")? as i32,
        exp: num("exp").unwrap_or(0.0) as i32,
        resist,
    })
}

/// An entry's 13 world records (`worlds`).
fn world_records(e: &crate::json::Value, class: &str) -> Result<Vec<EnemyStats>, String> {
    let worlds = e.get("worlds").and_then(|w| w.as_array()).ok_or_else(|| format!("{class}: no `worlds`"))?.iter().map(|w| record_of(w, class)).collect::<Result<Vec<_>, _>>()?;
    if worlds.len() != 13 {
        return Err(format!("{class}: {} worlds, not 13", worlds.len()));
    }
    Ok(worlds)
}

/// Re:CoM's hit reaction kinds on Sora (the damage check's switch, `0x1401E6DE0`): 1 the plain
/// flinch, `0x21` the card duel (action `0x24`, `0x140220C40`). Checked (decomp).
pub const RECOM_FLINCH_REACTION: u32 = 1;
pub const RECOM_DUEL_REACTION: u32 = 0x21;

/// KH1's post-hit invulnerability: every damage reaction (`0x14029CC40` … `0x14029D450`) calls
/// `0x140297F30(Sora, 30.0)`: flag `0x80` (no damage, no reaction) with a 30-frame timer that
/// `0x140297FE0` counts down by the frame step (1 at 60 fps) only once the reaction's lock is
/// released. Checked (decomp).
pub const KH1_HIT_UNTOUCHABLE_FRAMES: i32 = 30;

/// Re:CoM's time unit per 60 fps frame (`player+0x460` = dt; the roll's 600 units are 12 frames,
/// `+0x464` = dt × 0.02). Checked (decomp).
pub const RECOM_UNITS_PER_FRAME: i32 = 50;

/// The value for Re:CoM's hit-immunity timer `player+0x7BC` on a frame of the hit reaction
/// (action `0xC`). The damage check `0x1401E6DE0` skips hits while it is above 0, and the
/// Battle / Field exes subtract dt after the action step. Held at 30 frames + this frame's dt
/// through the reaction, it leaves exactly KH1's 30 frames once the reaction ends. Never lowers
/// a longer timer (Re:CoM's revive sets 3000).
pub fn post_hit_hold(current: i32, dt: i32) -> i32 {
    current.max(KH1_HIT_UNTOUCHABLE_FRAMES * RECOM_UNITS_PER_FRAME + dt.max(0))
}

#[cfg(test)]
mod tests;
