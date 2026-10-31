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

use crate::movement::{action, FrameOut, Host, Kh1Sora, SwingStart};
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

pub fn recom_id(kh1_row: usize) -> Option<u32> {
    ROW_MAP.iter().find(|&&(r, _)| r == kh1_row).map(|&(_, id)| id)
}

/// Re:CoM tests a row's hit window in its exe against the clock of the **last** motion update
/// (`frame = time / 50` before this frame's update), and the weapon's collision runs later in the
/// same frame on this frame's pose. KH1 tests its hit flags after this frame's update. So the
/// Re:CoM row holds KH1's window one frame earlier. Assumption: the weapon's task runs after
/// Sora's motion update in the frame (checked by the trace's `hit_active` frames in game).
pub const HIT_LEAD: u8 = 1;

/// KH1 kind → Re:CoM kind (`docs/recom-attack-design.md` §5): flinch 3 → 1, launch 5 → 2.
pub fn recom_kind(kh1_kind: u8) -> u8 {
    match kh1_kind {
        5 => 2,
        _ => 1,
    }
}

/// The KH1 row in Re:CoM's layout. `recom` is the Re:CoM row of the same clip.
pub fn kh1_row_bytes(recom: &[u8; 16], kh1_row: usize, m: &Moveset) -> Option<[u8; 16]> {
    let rw = m.row(kh1_row)?;
    let info = m.motions.info(rw.request as u16)?;
    let data = m.attack(rw.attack)?;
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
    b[0xB] = recom_kind(data.kind);
    b[0xC] = 0;
    b[0xD] = 0;
    // Not chainable by Re:CoM's own gate, no early cancel, no push: KH1's window decides.
    b[0xF] = FLAG_KH1;
    Some(b)
}

/// KH1's Counterattack row (motion 213, record `0x4E18`).
pub const COUNTERATTACK_ROW: usize = 0x3E;

/// The KH1 rows by KH1 row id (0..=0x3E; unused rows are zero), from Re:CoM's rows 1–7. The
/// Counterattack (row 0x3E) plays the appended action row `counter_clip` (its KH1-only clip),
/// in the layout of Re:CoM's row 1; without that row it stays empty (not offered).
pub fn kh1_table(recom_rows_1_7: &[[u8; 16]; 7], m: &Moveset, counter_clip: Option<u32>) -> Vec<[u8; 16]> {
    let mut t = vec![[0u8; 16]; COUNTERATTACK_ROW + 1];
    for &(r, id) in ROW_MAP.iter() {
        if let Some(b) = kh1_row_bytes(&recom_rows_1_7[id as usize - 1], r, m) {
            t[r] = b;
        }
    }
    if let Some(clip) = counter_clip.filter(|c| *c <= 0xFF) {
        if let Some(mut b) = kh1_row_bytes(&recom_rows_1_7[0], COUNTERATTACK_ROW, m) {
            b[0] = clip as u8;
            t[COUNTERATTACK_ROW] = b;
        }
    }
    t
}

/// A row's KH1 combo window (`+1` / `+3` with flag `0x80`), in frames.
pub fn row_window(b: &[u8; 16]) -> Option<(f32, f32)> {
    (b[0xF] & FLAG_KH1 != 0 && b[3] > b[1]).then(|| (b[1] as f32, if b[3] == 255 { f32::INFINITY } else { b[3] as f32 }))
}

/// Whether a KH1 request is one of the swings the rows play (Re:CoM owns its motion).
pub fn is_swing_request(r: u16) -> bool {
    (200..=206).contains(&r)
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
        if self.swing.take().is_some() {
            h.end_swing(self.state != state::GROUND);
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

/// What a frame did with the attack, for the trace.
pub fn attack_events(out: &FrameOut) -> Vec<String> {
    out.events
        .iter()
        .filter_map(|e| match e {
            Event::AttackStarted { row, request, counter, finisher } => Some(format!("attack(row={row} req={request} n={counter}{})", if *finisher { " fin" } else { "" })),
            Event::AttackRefused(r) => Some(format!("refused({r:?})")),
            _ => None,
        })
        .collect()
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
/// Re:CoM's extras (back bonus, weak part, boss falloff) which its own damage path applies.
pub fn kh1_damage(m: &Moveset, kh1_row: usize, strength: i32, weapon: &WeaponData, twin: &EnemyStats, rng: &mut kh1_core::Kh1Rand) -> Option<kh1_core::DamageResult> {
    let rw = m.row(kh1_row)?;
    let data = m.attack(rw.attack)?;
    let attacker = AttackerStats { strength, ..Default::default() };
    let live = LiveAttack::new(data, weapon, &attacker, 1.0);
    let mut resist = [1.0f32; 16];
    for (r, v) in resist.iter_mut().zip(twin.resist.iter()) {
        *r = *v as f32 / 100.0;
    }
    let stats = kh1_core::DefenderStats { hp: twin.hp, max_hp: twin.hp, defence: twin.defence, level: twin.level, min_damage: twin.min_damage, max_damage: twin.max_damage, resist, ..Default::default() };
    Some(kh1_core::damage::compute_damage(&live, &kh1_core::DamageCurve::kh1_defaults(), &stats, rng))
}

/// One Keyblade Sora can equip at the KH1 setting (decision 25), keyed by Re:CoM's Keyblade card
/// kind (1 Kingdom Key … 0x18 Bond of Flame; table `0x1405256D0`).
#[derive(Clone, Debug, PartialEq)]
pub struct Keyblade {
    pub card: u16,
    pub name: String,
    /// Where its numbers and model come from: `kh1` (KH1's weapon row), later `kh2`.
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
    for e in v.get("enemies").and_then(|e| e.as_array()).ok_or("no `enemies` array")? {
        let class = e.get("class").and_then(|c| c.as_str()).ok_or("enemy without `class`")?.to_string();
        let mut worlds = Vec::new();
        for w in e.get("worlds").and_then(|w| w.as_array()).ok_or_else(|| format!("{class}: no `worlds`"))? {
            let num = |k: &str| w.get(k).and_then(|x| x.as_f64()).ok_or_else(|| format!("{class}: record without `{k}`"));
            let res = ints(w.get("resist").ok_or_else(|| format!("{class}: no `resist`"))?, "resist")?;
            let mut resist = [100; 16];
            for (r, x) in resist.iter_mut().zip(res) {
                *r = x;
            }
            worlds.push(EnemyStats {
                hp: num("hp")? as i32,
                strength: num("strength")? as i32,
                defence: num("defence")? as i32,
                min_damage: num("floor")? as u16,
                max_damage: num("cap")? as u16,
                level: num("level")? as i32,
                exp: num("exp").unwrap_or(0.0) as i32,
                resist,
            });
        }
        if worlds.len() != 13 {
            return Err(format!("{class}: {} worlds, not 13", worlds.len()));
        }
        if let Some(r) = e.get("lock_radius").and_then(|r| r.as_f64()) {
            lock_radius.push((class.clone(), r as f32));
        }
        enemies.push((class, worlds));
    }
    Ok(Kh1Stats { sora_strength, base_strength, keyblades, enemies, lock_radius })
}

#[cfg(test)]
mod tests;
