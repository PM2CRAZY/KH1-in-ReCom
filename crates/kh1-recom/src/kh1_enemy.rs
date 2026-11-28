//! M11, KH1's enemies on Re:CoM's own enemy states (`docs/recom-m11-enemies.md`): the testable
//! part of the shared framework the Shadow (M10, `crate::shadow`) and the Soldier run on. The
//! runtime is `win/kh1_enemy.rs`; each enemy's behaviour is its `kh1_core` module, driven through
//! `kh1_core::enemy::Behaviour`.
//!
//! One table ([`CLASSES`]) says, per Re:CoM class: its vtable and allocation size (our 0x20-byte
//! tail after the body), its ctor and factory-size sites (`crate::sites`), the Re:CoM attack state
//! each KH1 attack runs in, how its battle appearance runs, and its manifest. At Combat = KH1 in
//! Sora's story with `enemy_ai = kh1`, each battle enemy of a listed class gets a copy of its own
//! vtable whose state slots run its KH1 behaviour (decision M10 (a): native states):
//!
//! - state 0 (appearance): Re:CoM's own begin and its battle-start rules; KH1's appearance;
//! - state 1: KH1's AI and body each frame (the behaviour polled once per frame, paused while an
//!   action motion plays, as KH1's script is);
//! - the class's attack states: Re:CoM attack states (entered as P1 enters one), KH1's motions with
//!   the new attack records of its `.ASD` on them;
//! - states 11 / 12: KH1's reactions for every hit Re:CoM's hit handler sends there, and KH1's
//!   clash recoil for a Guard;
//! - state 30: Re:CoM's defeat (prizes, EXP, removal) with the killing hit's KH1 reaction motion;
//! - the think slot `+0x3D0`: nothing.
//!
//! Status states (Stop, freeze, Gravity's flatten, first-strike stun) stay Re:CoM's.

use crate::json;
use crate::sites::{PtrSite, Site};
use kh1_core::enemy::{self as ke, Cmd, Table};

/// The Re:CoM enemy states every class's copy runs (`enemy + 0x4C0 + k × 0x40`; begin / step /
/// end at vtable `+0xE8 + 0x18 k` / `+8` / `+0x10`). Checked (decomp: `0x14017AF00`).
pub mod state {
    pub const APPEAR: i32 = 0;
    pub const THINK: i32 = 1;
    pub const FLINCH: i32 = 11;
    pub const BLOW: i32 = 12;
    pub const DEFEAT: i32 = 30;
}

pub use crate::shadow::{begin_slot, end_slot, step_slot, THINK_SLOT};

/// How a class's battle appearance runs in state 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Appear {
    /// `CEnemyShadow`'s own state 0 (`0x140192850` / `0x140192B00`): its begin's writes without
    /// its motion, both body sets off, KH1's rise, then its battle-start handshake (M10).
    Shadow,
    /// `CBattleEnemy`'s state 0 (`0x14017B590` / `0x1401829E0`, a class that keeps it): Re:CoM's
    /// begin as it is (its writes, the battle message `0x96`), then KH1's appearance in place of
    /// its motion, then Re:CoM's default state (`0x14017D6F0`), as its step does when its motion ends.
    Base,
}

/// The 0x20-byte tail after a converted enemy's body (the factory's `mov ecx, size` raised, as the
/// revenge bosses' sizes): magic, battle number, the runtime's index, KH1's flag 0x40.
pub mod tail {
    pub const SIZE: u32 = 0x20;
    pub const OFF_MAGIC: usize = 0;
    pub const OFF_BATTLE: usize = 4;
    pub const OFF_INDEX: usize = 8;
    /// 1 while the runtime says it can't be hit (KH1's actor flag 0x40, `Cmd::Hittable(false)`: a sunk Shadow, a warping
    /// Darkball or Search Ghost), else 0: a copy of the runtime's flag that other enemies' code can read without the
    /// runtime table's lock (the Green Requiem's Cure, `win::kh1_cure`). Written with the flag (never per frame).
    pub const OFF_UNHITTABLE: usize = 0xC;
}

/// One Re:CoM class with a KH1 behaviour.
pub struct Class {
    /// For the log and the trace (`shadow`, `soldier`): the log lines' prefix.
    pub name: &'static str,
    /// For messages (`Shadow`, `Soldier`).
    pub title: &'static str,
    /// Re:CoM's RTTI name of the class.
    pub rtti: &'static str,
    /// Re:CoM's enemy id (the factory `0x140188F40`'s case).
    pub recom_id: u16,
    /// The class's vtable and its slot count (Checked (exe): RTTI COL at `vt − 8`, the next
    /// class's COL after the slots).
    pub vtable: u32,
    pub slots: usize,
    /// Re:CoM's allocation size (our tail sits there).
    pub alloc: u32,
    pub magic: u32,
    /// The ctor hook, the factory's size, the vtable slots checked before the copy.
    pub ctor: &'static Site,
    pub alloc_site: &'static Site,
    pub ptrs: &'static [PtrSite],
    /// The Re:CoM attack state each KH1 attack (by index) runs in.
    pub attack_states: &'static [i32],
    pub attack_names: &'static [&'static str],
    pub appear: Appear,
    /// Its script reads its HP (the Soldier's counter `0x1DBC`): the runtime passes Re:CoM's HP
    /// ratio at each hit.
    pub reads_hp: bool,
    /// The manifest the asset step writes (recom root) and its format.
    pub manifest: &'static str,
    pub format: &'static str,
    /// Its manifest's parser and its behaviour's constructor (`seed`, KH1 `yaw`).
    pub parse: fn(&str) -> Result<Manifest, String>,
    pub make: fn(&Manifest, u32, f32) -> Box<dyn ke::Behaviour>,
}

/// The Soldier's attack names (`kh1_core::soldier::Attack::ALL`'s order).
const SOLDIER_ATTACKS: &[&str] = &["kick", "double kick", "jumping kick", "spin kick", "air kick", "rush", "rush leap"];

/// Every class with a KH1 behaviour. Index = the runtime's class number. M11's enemies each keep their
/// row in their own module (`crate::m11`); one line here per enemy.
pub static CLASSES: [&Class; 21] = [&SHADOW, &SOLDIER, &crate::fliers::airsoldier::CLASS, &crate::ground::powerwild::CLASS, &crate::fliers::darkball::CLASS, &crate::ground::bandit::CLASS, &crate::fliers::airpirate::CLASS, &crate::fliers::wyvern::CLASS, &crate::fliers::searchghost::CLASS, &crate::ground::pirate::CLASS, &crate::ground::largebody::CLASS, &crate::ground::neoshadow::CLASS, &crate::ground::wightknight::CLASS, &crate::ground::fatbandit::CLASS, &crate::fliers::gargoyle::CLASS, &crate::ground::defender::CLASS, &crate::ground::bouncywild::CLASS, &crate::fliers::rednocturne::CLASS, &crate::fliers::bluerhapsody::CLASS, &crate::fliers::greenrequiem::CLASS, &crate::fliers::yellowopera::CLASS];

/// M10's Shadow (class 0).
pub static SHADOW: Class = Class {
    name: "shadow",
    title: "Shadow",
    rtti: ".?AVCEnemyShadow@@",
    recom_id: 1,
    vtable: crate::shadow::VTABLE,
    slots: crate::shadow::VTABLE_SLOTS,
    alloc: crate::shadow::ALLOC,
    magic: crate::shadow::tail::MAGIC,
    ctor: &crate::sites::SHADOW_HOOKS[0],
    alloc_site: &crate::sites::SHADOW_ALLOC[0],
    ptrs: crate::sites::SHADOW_PTRS,
    // Claw 2, swipe 4, leap claw 3 (`crate::shadow::attack_state`).
    attack_states: &[2, 4, 3],
    attack_names: &["claw", "swipe", "leap claw"],
    appear: Appear::Shadow,
    reads_hp: false,
    manifest: crate::shadow::MANIFEST,
    format: crate::shadow::FORMAT,
    parse: from_shadow,
    make: make_shadow,
};

/// M11's Soldier (class 1).
pub static SOLDIER: Class = Class {
    name: "soldier",
    title: "Soldier",
    rtti: ".?AVCEnemySoldier@@",
    recom_id: 2,
    vtable: 0x0054_EA68,
    slots: 139,
    alloc: 0xCD0,
    magic: 0x4F53_314B, // "K1SO"
    ctor: &crate::sites::SOLDIER_HOOKS[0],
    alloc_site: &crate::sites::SOLDIER_ALLOC[0],
    ptrs: crate::sites::SOLDIER_PTRS,
    // Re:CoM's Soldier has two attack states (Checked (exe): `CEnemySoldier` overrides states
    // 2 (its kick `e002_050`) and 3 (its spin kick `e002_051`)): the kicks on the ground in
    // state 2, the attacks that leave the ground in state 3.
    attack_states: &[2, 2, 3, 2, 3, 2, 3],
    attack_names: SOLDIER_ATTACKS,
    appear: Appear::Base,
    reads_hp: true,
    manifest: SOLDIER_MANIFEST,
    format: FORMAT,
    parse: parse_soldier,
    make: make_soldier,
};

/// The generic manifest's format (the Shadow keeps `kh1-shadow 1`).
pub const FORMAT: &str = "kh1-enemy 1";
pub const SOLDIER_MANIFEST: &str = "kh1-soldier.json";

/// The states a class's copy replaces (begin and step): appearance, think, its attack states, the
/// two reaction states, the defeat.
pub fn replaced_states(c: &Class) -> Vec<i32> {
    let mut v = vec![state::APPEAR, state::THINK];
    for &k in c.attack_states {
        if !v.contains(&k) {
            v.push(k);
        }
    }
    v.extend([state::FLINCH, state::BLOW, state::DEFEAT]);
    v
}

pub fn alloc_bytes(size: u32) -> [u8; 5] {
    crate::shadow::alloc_bytes(size)
}

/// One request's clip in the class's `.ANN`.
pub use crate::shadow::Clip;

/// A class's manifest, in the form the runtime uses.
#[derive(Clone, Debug)]
pub struct Manifest {
    pub consts: Table,
    pub clips: Vec<Clip>,
    /// The clash recoil request of each attack (by index).
    pub recoil: Vec<u16>,
    /// Re:CoM's bones of KH1's hurt parts (part 0 first).
    pub parts: Vec<i32>,
    /// The enemy's guard-type rows (the shared enemy-guard step, `kh1_core::enemy::guard`; its
    /// manifest's `guards`, in the behaviour's order), none for an enemy without any.
    pub guards: Vec<ke::guard::GuardRow>,
    /// A KH1 child ring on Re:CoM's own shockwave record (M11, the Large Body's stomp; the asset step's
    /// `enemy::RINGS`), from the manifest's `ring_*` / `stomp_request` numbers; none for the others.
    pub ring: Option<Ring>,
    /// M11, attack meets attack: every attack row's KH1 clash numbers and the `.ASD` records made from it (the
    /// manifest's per-attack `clash`, the Shadow's top-level `clash`); empty for a manifest from an older asset
    /// step (then no clashes: logged at start).
    pub clash: Vec<ClashRow>,
}

/// One KH1 attack row as Re:CoM's attack records carry it (M11, attack meets attack; `docs/recom-m11-enemies.md`
/// § Attack meets attack): the attack it belongs to (the class's index), KH1's clash type (`+0x38 & 7`), priority
/// (`+0x3C`) and front cone (`+0x60` / `+0x64`), and the indices of the `.ASD` records the asset step made from it
/// (Re:CoM's attack set gives each live entry its record: entry `+0x18`, records at set `+0x28`, 0xA0 apart).
#[derive(Clone, Debug, PartialEq)]
pub struct ClashRow {
    /// The class's attack index.
    pub attack: u8,
    /// `+0x38 & 7`.
    pub clash_type: u32,
    /// `+0x3C`.
    pub priority: i32,
    /// `+0x60` (radians) and `+0x64`: only guard types (1..4) test the cone (`0x1402B4640`).
    pub turn: f32,
    pub cos: f32,
    /// The `.ASD` records made from it.
    pub records: Vec<u16>,
}

/// The size of a Re:CoM `.ASD` attack record (`0x14014E010` steps its records by 0xA0). Checked (decomp).
pub const ASD_RECORD: usize = 0xA0;

/// The index of a live attack entry's record (entry `+0x18`) in the set's records (`+0x28`, `count` of them), or
/// None when it isn't one of them.
pub fn record_index(record: usize, records: usize, count: u32) -> Option<u16> {
    let d = record.checked_sub(records)?;
    (d.is_multiple_of(ASD_RECORD) && d / ASD_RECORD < count as usize).then(|| u16::try_from(d / ASD_RECORD).ok()).flatten()
}

/// One live KH1 enemy attack row a KH1 swing's frame looks at (M11, attack meets attack; `win::kh1_enemy`
/// fills it from Re:CoM's attack set): its enemy (any key), its Re:CoM collision, its KH1 numbers, whether the
/// swing's spheres touch its Re:CoM shape, whether the swing comes from inside its front cone (`0x1402B4640`, only
/// guard types test it), whether this swing already met it (KH1's met masks `+0x9C`, `0x1402B3BE0`: two attack
/// objects meet once while both live).
#[derive(Clone, Copy, Debug)]
pub struct LiveRow<'a> {
    pub enemy: usize,
    pub coll: usize,
    pub row: &'a ClashRow,
    pub touches: bool,
    pub front: bool,
    pub met_before: bool,
}

/// What a meeting gives the enemy (KH1's record on it, `0x1402B4BC0`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyGets {
    /// A clash record (kind 6 / 7): its row's recoil, its script hears the clash.
    Clash { attack: u8, kind: u8, diff: i32 },
    /// The swing's record flagged 0x20 (its type 1 / 3 / 4 row repelled it): no damage, no reaction.
    Repelled { attack: u8 },
    /// No record.
    Nothing,
}

/// One meeting of the swing with an enemy row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Met {
    pub enemy: usize,
    pub coll: usize,
    pub gets: EnemyGets,
    /// The row is marked for the rest of the frame: it meets no hurt body (here: Re:CoM's test of that
    /// collision against Sora answers 0 until the enemy's hit handler has run).
    pub stops: bool,
}

/// One KH1 swing frame against the live enemy rows (KH1's attack pass `0x1402B47A0` → `0x1402B3BE0` → `0x1402B4A20`,
/// before any hurt body is tested: `0x1402B4460`). The swing: KH1's clash type and priority (row + Keyblade). Returns
/// Sora's record (his first: once marked he gets no other this frame), whether the swing is marked (it meets no hurt
/// body this frame), and each meeting. Rows the swing doesn't touch, whose types don't meet (`0x140528CB8`), outside
/// their cone or met before are skipped. Checked (decomp).
pub fn sweep<'a>(swing: (u32, i32), rows: impl IntoIterator<Item = LiveRow<'a>>) -> (Option<(u8, i32)>, bool, Vec<Met>) {
    let (mut sora, mut marked, mut out) = (None, false, Vec::new());
    for r in rows {
        if !r.touches || r.met_before || !r.front || !ke::guard::meets(swing.0, r.row.clash_type) {
            continue;
        }
        let m = ke::guard::meeting((swing.0, swing.1, marked), (r.row.clash_type, r.row.priority, false));
        if let ke::guard::Met::Clash { kind, diff } = m.a {
            sora.get_or_insert((kind, diff));
        }
        marked = m.a_stops;
        let gets = match m.b {
            ke::guard::Met::Clash { kind, diff } => EnemyGets::Clash { attack: r.row.attack, kind, diff },
            ke::guard::Met::Repels => EnemyGets::Repelled { attack: r.row.attack },
            _ => EnemyGets::Nothing,
        };
        // 0.198.11: a row that gave Sora a clash record does him no harm this frame (`Meeting::a_unharmed`: his kind 6 / 7 comes
        // first and its flag 0x80 makes the row's hit 0 HP), even when the row itself is not stopped (priority 98-100)
        out.push(Met { enemy: r.enemy, coll: r.coll, gets, stops: m.a_unharmed() });
    }
    (sora, marked, out)
}

/// KH1's reaction step takes the strongest clash of a frame (`0x1402B8E10`: a record plays only when its kind is
/// above the last played, `uVar6 < uVar5`): the pending clash after `new` arrives.
pub fn stronger(old: Option<EnemyGets>, new: EnemyGets) -> Option<EnemyGets> {
    match (old, new) {
        (_, EnemyGets::Nothing) => old,
        (Some(EnemyGets::Clash { kind: k0, .. }), EnemyGets::Clash { kind: k1, .. }) if k1 <= k0 => old,
        (Some(EnemyGets::Clash { .. }), EnemyGets::Repelled { .. }) => old,
        _ => Some(new),
    }
}

/// The manifest's clash rows (`"clash"`): per row `type`, `priority`, `turn`, `cos`, `records`, and `attack` when
/// `attack` is None (the Shadow's list).
fn parse_clash(list: &json::Value, attack: Option<u8>, attacks: usize) -> Result<Vec<ClashRow>, String> {
    let list = list.as_array().ok_or("\"clash\" is not a list")?;
    let mut out = Vec::with_capacity(list.len());
    for c in list {
        let n = |k: &str| c.get(k).and_then(|x| x.as_f64()).filter(|x| x.is_finite()).ok_or_else(|| format!("clash row without \"{k}\""));
        let a = match attack {
            Some(a) => a,
            None => n("attack")? as u8,
        };
        if a as usize >= attacks {
            return Err(format!("clash row of attack {a}: the class has {attacks}"));
        }
        let (t, p, turn, cos) = (n("type")?, n("priority")?, n("turn")? as f32, n("cos")? as f32);
        if !(0.0..=7.0).contains(&t) || !(0.0..=100.0).contains(&p) || !(-1.0..=1.0).contains(&cos) {
            return Err(format!("clash row: type {t} priority {p} cos {cos}"));
        }
        let records = c
            .get("records")
            .and_then(|r| r.as_array())
            .ok_or("clash row without \"records\"")?
            .iter()
            .map(|r| r.as_u32().filter(|&r| r <= 0xFFFF).map(|r| r as u16).ok_or_else(|| "bad clash record".to_string()))
            .collect::<Result<Vec<_>, _>>()?;
        out.push(ClashRow { attack: a, clash_type: t as u32, priority: p as i32, turn, cos, records });
    }
    Ok(out)
}

/// A KH1 child object that grows on the floor (`ex_2050_00.bd`: made at frame `start` of `request`, its radius
/// `growth` × the frames since, for `life` frames; KH1 units). The asset step put Re:CoM's circle record on the
/// request's clip for those frames; the host sets its radius each KH1 frame (`win::kh1_enemy::ring_step`) and its
/// band, so Re:CoM's circle test runs KH1's ring rule on it ([`ring_hits`], `win::kh1_enemy::circle_test`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ring {
    pub request: u16,
    pub start: f32,
    pub life: f32,
    pub growth: f32,
    /// The band's width (`ring_band`, the child's 100 at `0x3CC`) and the height window (`ring_height`, its 50 at
    /// `0x3D2`), KH1 units.
    pub band: f32,
    pub height: f32,
}

impl Ring {
    /// The ring's radius in metres at KH1 frame `t` of its request's motion, or none outside its life. The child
    /// tests with `r` then adds `growth` × the step (`0x384`..`0x43A`): `r` = `growth` × (t − `start`).
    pub fn radius(&self, t: f32) -> Option<f32> {
        (t >= self.start && t < self.start + self.life).then(|| self.growth * (t - self.start) * crate::units::METRES_PER_UNIT)
    }

    /// The band's width in metres (KH1's 100 → 1.0 m).
    pub fn band_m(&self) -> f32 {
        self.band * crate::units::METRES_PER_UNIT
    }

    /// The height window in metres (KH1's 50 → 0.5 m).
    pub fn height_m(&self) -> f32 {
        self.height * crate::units::METRES_PER_UNIT
    }

    fn from(c: &Table) -> Option<Ring> {
        let (r, start, life, growth) = (c.get("stomp_request"), c.get("ring_at"), c.get("ring_life"), c.get("ring_growth"));
        let (band, height) = (c.get("ring_band"), c.get("ring_height"));
        if !(r.is_finite() && start.is_finite() && life > 0.0 && growth.is_finite()) || !(0.0..=65535.0).contains(&r) {
            return None;
        }
        if !(band.is_finite() && band > 0.0 && height.is_finite() && height >= 0.0) {
            return None;
        }
        Some(Ring { request: r as u16, start, life, growth, band, height })
    }
}

/// KH1's ring test (`ex_2050_00.bd` routine `0x18`, Checked (files: the child disassembled; decomp: `sys 8` / `sys 6`))
/// on Re:CoM's shapes, metres: `centre` the ring's centre (Re:CoM's circle `+0x10`), `r` its radius (`+0x70`), `band`
/// its width (our `+0x74`), `height` the window, `foot` the target's position point (its body's bottom: KH1 reads
/// the actor's `+0x50`). A miss when the heights differ by more than `height`; else a hit when the flat distance d
/// is strictly inside (r − band, r). No body radius is added (KH1 adds none).
pub fn ring_hits(centre: [f32; 3], r: f32, band: f32, height: f32, foot: [f32; 3]) -> bool {
    if (foot[1] - centre[1]).abs() > height {
        return false;
    }
    let (dx, dz) = (foot[0] - centre[0], foot[2] - centre[2]);
    let d = (dx * dx + dz * dz).sqrt();
    r - band < d && d < r
}

/// The position point [`ring_hits`] tests of a Re:CoM body shape (the body in the circle test `0x140177190`): a
/// column (types 4 / 5: ends `+0x10` / `+0x20`, Sora's bottom at his toes' mean height, top at his head + 0.2, both at
/// his hips' x / z; `0x1401F4000`) its lower end; a sphere (type 2: centre `+0x20` raised by `+0x74`, radius `+0x70`,
/// as Re:CoM's test places it) its lowest point. Checked (decomp); that a column's lower end and a sphere's lowest
/// point stand for KH1's position point: Assumption.
pub fn body_foot(kind: u32, p10: [f32; 3], p20: [f32; 3], radius: f32, lift: f32) -> Option<[f32; 3]> {
    match kind {
        4 | 5 => Some(if p20[1] < p10[1] { p20 } else { p10 }),
        2 => Some([p20[0], p20[1] + lift - radius, p20[2]]),
        _ => None,
    }
}

impl Manifest {
    pub fn clip(&self, r: u16) -> Option<&Clip> {
        self.clips.iter().find(|c| c.request == r)
    }

    /// `sys 43`'s motion lengths for the behaviour.
    pub fn frames(&self) -> Vec<(u16, f32)> {
        self.clips.iter().map(|c| (c.request, c.frames as f32)).collect()
    }

    /// The clash recoil of an attack; None when the model has no motion for it (KH1's request then
    /// names no entry: `0x1402A01E0` flags it 0x200000).
    pub fn recoil_of(&self, attack: u8) -> Option<u16> {
        self.recoil.get(attack as usize).copied().filter(|&r| self.clip(r).is_some())
    }

    /// The attack row a live `.ASD` record (by index) was made from (M11, attack meets attack).
    pub fn clash_of(&self, record: u16) -> Option<&ClashRow> {
        self.clash.iter().find(|c| c.records.contains(&record))
    }

    /// A live `.ASD` record (by index) was made from a row KH1 never tests against a hurt body (clash type 1, 2
    /// or 4, `0x1402B3BE0`): it only meets Sora's swings, it never hurts (owner 2026-11-02, "Clash choices":
    /// block only). `win::attack::pair_dispatch` answers 0 for its collision against the player side.
    pub fn never_hurts(&self, record: u16) -> bool {
        self.clash_of(record).is_some_and(|c| !ke::guard::hurts_bodies(c.clash_type))
    }

    /// The manifest has a row that never hurts ([`Manifest::never_hurts`]): the Soldier's and the Powerwild's.
    pub fn has_never_hurting(&self) -> bool {
        self.clash.iter().any(|c| !ke::guard::hurts_bodies(c.clash_type))
    }
}

impl ke::Motions for Manifest {
    fn has(&self, request: u16) -> bool {
        self.clip(request).is_some()
    }
}

/// The Shadow's `kh1-shadow 1` manifest as the framework's.
pub fn from_shadow(text: &str) -> Result<Manifest, String> {
    let m = crate::shadow::parse(text)?;
    let v = json::parse(text)?;
    let parts = parse_parts(&v, 2)?;
    let consts = Table::new(m.consts.pairs().to_vec(), kh1_core::shadow::NAMES)?;
    let clash = match v.get("clash") {
        Some(c) => parse_clash(c, None, 3)?,
        None => Vec::new(),
    };
    Ok(Manifest { consts, clips: m.clips, recoil: vec![m.recoil[0], m.recoil[1], m.recoil[2]], parts, guards: Vec::new(), ring: None, clash })
}

fn parse_parts(v: &json::Value, min: usize) -> Result<Vec<i32>, String> {
    let a = v.get("parts").and_then(|a| a.as_array()).ok_or("no \"parts\"")?;
    if a.len() < min {
        return Err(format!("\"parts\" needs at least {min} bones"));
    }
    a.iter().map(|b| b.as_u32().filter(|&b| b < 256).ok_or_else(|| "bad part bone".to_string()).map(|b| b as i32)).collect()
}

/// A `kh1-enemy 1` manifest (the asset step's `kh1_recom_assets::enemy`): `enemy` (the class's
/// name), `consts`, `attacks` (per attack: its request and clash recoil), `parts`, `requests`.
/// `names`: the behaviour's constants; `needed`: the requests it plays.
pub fn parse(text: &str, enemy: &str, names: &[&str], needed: &[u16]) -> Result<Manifest, String> {
    let v = json::parse(text)?;
    if v.get("format").and_then(|f| f.as_str()) != Some(FORMAT) {
        return Err(format!("not a \"{FORMAT}\" manifest (an older asset step? run it again)"));
    }
    if v.get("enemy").and_then(|f| f.as_str()) != Some(enemy) {
        return Err(format!("not the {enemy}'s manifest"));
    }
    let json::Value::Obj(c) = v.get("consts").ok_or("no \"consts\"")? else { return Err("\"consts\" is not an object".into()) };
    let mut pairs = Vec::new();
    for (k, x) in c {
        let f = x.as_f64().ok_or_else(|| format!("const {k} is not a number"))? as f32;
        if !f.is_finite() {
            return Err(format!("const {k} = {f}"));
        }
        pairs.push((k.clone(), f));
    }
    let consts = Table::new(pairs, names)?;
    let list = v.get("requests").and_then(|a| a.as_array()).ok_or("no \"requests\"")?;
    let mut clips = Vec::with_capacity(list.len());
    for r in list {
        let n = |k: &str| r.get(k).and_then(|x| x.as_u32()).ok_or_else(|| format!("request entry without \"{k}\""));
        let b = |k: &str| match r.get(k) {
            Some(json::Value::Bool(b)) => Ok(*b),
            _ => Err(format!("request entry without \"{k}\"")),
        };
        let request = n("request")?;
        let slot = n("slot")?;
        if request > 0xFFFF || slot >= 1000 {
            return Err(format!("request {request} slot {slot}"));
        }
        clips.push(Clip { request: request as u16, slot, frames: n("frames")?, root: b("root")?, looped: b("loop")? });
    }
    let atk = v.get("attacks").and_then(|a| a.as_array()).ok_or("no \"attacks\"")?;
    let mut recoil = Vec::new();
    let mut clash = Vec::new();
    for (i, a) in atk.iter().enumerate() {
        if let Some(c) = a.get("clash") {
            clash.extend(parse_clash(c, Some(i as u8), atk.len())?);
        }
        let r = a.get("recoil").and_then(|x| x.as_u32()).filter(|&r| r <= 0xFFFF).ok_or("attack without \"recoil\"")?;
        let q = a.get("request").and_then(|x| x.as_u32()).filter(|&r| r <= 0xFFFF).ok_or("attack without \"request\"")?;
        if !clips.iter().any(|c| c.request as u32 == q) {
            return Err(format!("no clip for the attack request {q}"));
        }
        recoil.push(r as u16);
    }
    let parts = parse_parts(&v, 1)?;
    let guards = parse_guards(&v, &clips)?;
    let ring = Ring::from(&consts);
    if let Some(r) = ring {
        if !clips.iter().any(|c| c.request == r.request && (r.start as u32) < c.frames) {
            return Err(format!("the ring's request {} has no clip of more than {} frames", r.request, r.start));
        }
    }
    let m = Manifest { consts, clips, recoil, parts, guards, ring, clash };
    for &r in needed {
        if m.clip(r).is_none() {
            return Err(format!("no clip for KH1 request {r}"));
        }
    }
    Ok(m)
}

/// The manifest's `guards` (absent: none): per guard-type row its `type` (clash type 1..4),
/// `priority`, `recoil` (a request with a clip, or one the model lacks: then Re:CoM's flinch, as for
/// a guarded attack), the front cone `turn` / `cos`, and its `windows` (per request that makes it,
/// the frames `on` its HIT slot is on). Checked here: the type is a guard type, each window lies in
/// its request's clip.
fn parse_guards(v: &json::Value, clips: &[Clip]) -> Result<Vec<ke::guard::GuardRow>, String> {
    let Some(list) = v.get("guards") else { return Ok(Vec::new()) };
    let list = list.as_array().ok_or("\"guards\" is not a list")?;
    let mut out = Vec::new();
    for g in list {
        let name = g.get("name").and_then(|x| x.as_str()).ok_or("guard without \"name\"")?.to_string();
        let n = |k: &str| g.get(k).and_then(|x| x.as_f64()).filter(|x| x.is_finite()).ok_or_else(|| format!("guard {name} without \"{k}\""));
        let clash_type = n("type")? as u32;
        if !ke::guard::is_guard_type(clash_type) {
            return Err(format!("guard {name}: type {clash_type} is not a guard type"));
        }
        let priority = n("priority")?;
        let recoil = n("recoil")?;
        if !(0.0..=100.0).contains(&priority) || !(0.0..=65535.0).contains(&recoil) {
            return Err(format!("guard {name}: priority {priority} recoil {recoil}"));
        }
        let mut windows = Vec::new();
        for w in g.get("windows").and_then(|x| x.as_array()).ok_or_else(|| format!("guard {name} without \"windows\""))? {
            let r = w.get("request").and_then(|x| x.as_u32()).filter(|&r| r <= 0xFFFF).ok_or_else(|| format!("guard {name}: window without \"request\""))? as u16;
            let clip = clips.iter().find(|c| c.request == r).ok_or_else(|| format!("guard {name}: no clip for its request {r}"))?;
            let mut on = Vec::new();
            for p in w.get("on").and_then(|x| x.as_array()).ok_or_else(|| format!("guard {name}: request {r} without \"on\""))? {
                let a = p.as_array().filter(|a| a.len() == 2).ok_or_else(|| format!("guard {name}: request {r}: a window is not [on, off]"))?;
                let (x, y) = (a[0].as_f64().unwrap_or(f64::NAN) as f32, a[1].as_f64().unwrap_or(f64::NAN) as f32);
                if !(0.0 <= x && x < y && y <= clip.frames as f32) {
                    return Err(format!("guard {name}: request {r}: window [{x}, {y}) outside its {} frames", clip.frames));
                }
                on.push((x, y));
            }
            windows.push((r, on));
        }
        let (turn, cos) = (n("turn")? as f32, n("cos")? as f32);
        if !(-1.0..=1.0).contains(&cos) {
            return Err(format!("guard {name}: cos {cos}"));
        }
        // Rows acting with it (the asset step's merged rows, `"also"`): KH1 row instances it stands for.
        let also = g.get("also").and_then(|x| x.as_array()).map_or(0, |a| a.len());
        let rows = u8::try_from(1 + also).map_err(|_| format!("guard {name}: {also} rows with it"))?;
        out.push(ke::guard::GuardRow { name, clash_type, priority: priority as i32, recoil: recoil as u16, turn, cos, windows, rows });
    }
    Ok(out)
}

/// The requests the Soldier's behaviour plays (its manifest must map them).
pub const SOLDIER_REQUESTS: &[u16] = {
    use kh1_core::soldier::req::*;
    &[IDLE, WALK, RUN, JUMP, FALL, AIR, LAND, FLINCH, APPEAR, NOTICE, KICK, DOUBLE_KICK, JUMP_KICK, SPIN_KICK, AIR_KICK, RUSH, RUSH_LEAP, CROUCH]
};

/// A behaviour command as a log token (attack names from the class).
pub fn cmd_token(c: &Cmd<u8>, names: &[&str], state_name: &dyn Fn(u8) -> &'static str) -> String {
    match c {
        Cmd::Motion { request, blend } => format!("motion {request} b{blend}"),
        Cmd::Queue { request, blend } => format!("queue {request} b{blend}"),
        Cmd::Action { request, attack, then, .. } => format!("action {request}{} then {then}", attack.map_or(String::new(), |a| format!(" ({})", names.get(a as usize).copied().unwrap_or("?")))),
        Cmd::Yaw(y) => format!("yaw {:.2}", y),
        Cmd::Jump { height, velocity } => format!("jump h{height} v({:.1},{:.1})", velocity.x, velocity.z),
        Cmd::Underground(u) => format!("underground {u}"),
        Cmd::Hittable(h) => format!("hittable {h}"),
        Cmd::Height(h) => format!("height {h:.2}"),
        Cmd::FadeIn(f) => format!("fade in {f}"),
        Cmd::Target(t) => format!("target {t}"),
        Cmd::State(s) => format!("state {}", state_name(*s)),
        Cmd::Held => "held (root motion carries it)".into(),
        Cmd::Attack { request, attack, .. } => format!("attack {request} ({}), steered", names.get(*attack as usize).copied().unwrap_or("?")),
        Cmd::AttackAt { request, attack, start, .. } => format!("attack {request} ({}) from {start}, steered", names.get(*attack as usize).copied().unwrap_or("?")),
        Cmd::Scale(k) => format!("scale {k:.2}"),
        Cmd::Visible(v) => format!("visible {v}"),
        Cmd::MotionAt { request, blend, start } => format!("motion {request} b{blend} from {start}"),
        Cmd::Guard(Some((g, r))) if *r == ke::guard::HELD => format!("guard row {g} held (KH1's mode 3)"),
        Cmd::Guard(Some((g, r))) => format!("guard row {g} with {r}"),
        Cmd::Guard(None) => "guard rows end".into(),
        Cmd::Throw(n) => format!("throws its object {n}"),
        Cmd::Note(n) => format!("note: {n}"),
        Cmd::HealFull => "heals to full HP (KH1's HP = max HP)".into(),
        Cmd::Support { child, draw } => format!("its spell {child}'s target among its allies (KH1's sys 102, draw {draw:.4})"),
        Cmd::Reward(r) => format!("reward: EXP bonus EXP / {} + {}{}, KH1 prizes {:?}", r.exp_div, r.exp_add, if r.exp_min > 0 { format!(" (at least {})", r.exp_min) } else { String::new() }, r.prizes),
        Cmd::QueueAt { request, blend, start } => format!("queue {request} b{blend} from {start}"),
        Cmd::Strike { child, at } => format!("strike {child} at ({:.0},{:.0},{:.0})", at.x, at.y, at.z),
        Cmd::Warn { child, at } => format!("mark {child} at ({:.0},{:.0},{:.0}): the warning until its strike", at.x, at.y, at.z),
        Cmd::Restart { request, blend, start } => format!("motion {request} b{blend} again from {start}"),
    }
}

/// Whether a command is worth a log line (state changes, actions, jumps, underground; not the
/// per-frame walk / yaw / height).
pub fn logged(c: &Cmd<u8>) -> bool {
    matches!(c, Cmd::Action { .. } | Cmd::Attack { .. } | Cmd::AttackAt { .. } | Cmd::Jump { .. } | Cmd::Underground(_) | Cmd::Hittable(_) | Cmd::State(_) | Cmd::Target(_) | Cmd::FadeIn(_) | Cmd::Held | Cmd::Visible(_) | Cmd::Guard(_) | Cmd::MotionAt { .. } | Cmd::Restart { .. } | Cmd::Throw(_) | Cmd::Strike { .. } | Cmd::Warn { .. } | Cmd::Note(_) | Cmd::HealFull | Cmd::Support { .. } | Cmd::Reward(_)) || matches!(c, Cmd::Motion { request, .. } if *request != ke::react::req::WALK && *request != ke::react::req::RUN)
}

/// The end of the motion playing, read from Re:CoM's motion clock (`0x1401A47C0`, units).
///
/// A clip the asset step marks as looping (KH1's last pose equals its first: the Soldier's kick
/// 200, its air kick 204) loops in Re:CoM too: its clock wraps back to 0 at the clip's end and
/// never reaches KH1's length (owner 2026-11-02, "this soldier just decides to not attack": every
/// kick 200 in the 0.68.0 log waited forever). A wrap is the end of the motion's first cycle, so
/// it ends there as the KH1 length does. Assumption: KH1 ends an action at its motion's length
/// whether or not the motion loops (KH1's Soldier kicks once and goes on).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ClipWatch {
    /// The clock at the last read.
    pub last: i32,
    /// KH1 frames since the motion was played.
    pub age: f32,
    /// The clock wrapped since it was played.
    pub wrapped: bool,
}

impl ClipWatch {
    /// A motion was just played.
    pub fn restart(&mut self) {
        *self = ClipWatch::default();
    }

    /// One KH1 frame passed.
    pub fn tick(&mut self, step: f32) {
        self.age += step;
    }

    /// Whether the motion (KH1 length `frames`) has ended at clock `now`. A clock that went back
    /// counts as a wrap only once half the motion has played (the first reads after a play may
    /// still show the previous motion's clock: Assumption, the play's reset timing is not read).
    pub fn ended(&mut self, now: i32, frames: f32) -> bool {
        if now < self.last && self.age >= frames * 0.5 {
            self.wrapped = true;
        }
        self.last = now;
        self.wrapped || now >= crate::units::frames_to_units(frames)
    }
}

/// Whether an enemy's own motion pack carries its class's KH1 clips (owner 2026-11-03: "switching from recom to KH1
/// enemy ai in the same room results in the ported enemies T Posing"). A class can be built for more than one Re:CoM
/// enemy id, each with its own `ENEMY/<id>` files: the factory `0x140188F40` builds `CEnemyShadow` for ids 1 and 32
/// and `CEnemySearchghost` for 14 and 33 (also 16 / 34, 17 / 35, 31 / 36, classes not ported). The asset step extends
/// only `ENEMY/<recom_id>`, so an enemy of the other id has Re:CoM's plain pack: every KH1 request names a slot past
/// its count, Re:CoM's clip lookup gives no clip and the model stands in its bind pose (Checked (decomp + exe + log)).
/// So the class swap is undone, in the spawn's init, for an enemy whose pack lacks any clip its manifest plays: it
/// keeps Re:CoM's AI and motions, as at `enemy_ai = recom`.
pub mod pack {
    use super::Clip;

    /// Re:CoM's motion set as its clip lookup `0x14024DC60` reads it: the enemy's motion controller `+0xD8`
    /// (`0x1401A5110` makes it, `0x1401A3470` plays through it), its set `+0x20` (`0x14024E060` binds the `.ANN`
    /// found by the model id, `0x1402A4820`), the set's slot count `+0x68`, its clip pointers at `+8` → `+8`, one
    /// per slot, 0 for an empty one. Checked (decomp).
    pub const CONTROLLER: usize = 0xD8;
    pub const SET: usize = 0x20;
    pub const COUNT: usize = 0x68;
    pub const TABLE: usize = 0x8;
    pub const CLIPS: usize = 0x8;
    /// The entity spawn `0x1401A8900` calls the new enemy's vtable `+0x38` right after the factory, with (enemy,
    /// kind 0xC, Re:CoM id); every class's `+0x38` runs its init `+0x30` (`0x140198600` / `0x1401996F0` →
    /// `0x1401805A0`, which builds the model and its motion controller). A failed init (0) makes the spawn delete
    /// the enemy. Checked (decomp: `0x1401A8900`, `0x140192AE0`, `0x14018A360`).
    pub const INIT_SLOT: usize = 0x38;

    /// What an enemy's pack holds for its class's clips.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Verdict {
        /// Every clip the manifest names is there (`count`: the pack's slots).
        Carries { count: i32 },
        /// The first clip missing: past the pack's `count` slots, or an empty slot.
        Missing { count: i32, request: u16, slot: u32 },
        /// The enemy has no motion controller or set (nothing can play on it).
        NoSet,
    }

    impl Verdict {
        pub fn carries(self) -> bool {
            matches!(self, Verdict::Carries { .. })
        }
    }

    /// `count`: the set's slots (None: no set); `has(slot)`: the slot's clip pointer is not 0, asked only for a
    /// slot under `count` (as `0x14024DC60` reads it).
    pub fn check(count: Option<i32>, clips: &[Clip], has: impl Fn(u32) -> bool) -> Verdict {
        let Some(count) = count else { return Verdict::NoSet };
        for c in clips {
            if i64::from(c.slot) >= i64::from(count) || !has(c.slot) {
                return Verdict::Missing { count, request: c.request, slot: c.slot };
            }
        }
        Verdict::Carries { count }
    }

    /// The first time this class meets this Re:CoM id (the once-per-id log line).
    pub fn first_time(seen: &mut Vec<(usize, u32)>, class: usize, id: u32) -> bool {
        if seen.contains(&(class, id)) {
            return false;
        }
        seen.push((class, id));
        true
    }

    /// The once-per-id line: what the pack of `ENEMY/<id>` holds and which AI the class's enemies of that id get.
    pub fn line(name: &str, title: &str, id: u32, v: Verdict) -> String {
        match v {
            Verdict::Carries { count } => format!("{name}: Re:CoM id {id} (ENEMY/{id:04}): its motion pack carries the KH1 clips ({count} slots): its {title}s get KH1's AI"),
            Verdict::Missing { count, request, slot } => format!(
                "{name}: Re:CoM id {id} (ENEMY/{id:04}): its motion pack has no clip for KH1's request {request} (slot {slot}; the pack has {count} slots): its {title}s keep Re:CoM's AI and motions (KH1's would leave them in the bind pose; the asset step extends only the class's own id's pack)"
            ),
            Verdict::NoSet => format!("{name}: Re:CoM id {id} (ENEMY/{id:04}): no motion set on the enemy after its init: its {title}s keep Re:CoM's AI"),
        }
    }
}

/// The safety net for an action the behaviour waits on (KH1's script pauses while an action
/// motion plays): no KH1 enemy may wait forever.
pub mod watchdog {
    /// An attack's Re:CoM attack state must begin within this many KH1 frames of the request.
    pub const START_LIMIT: f32 = 10.0;
    /// An action that plays this many KH1 frames past its motion's length is released.
    pub const END_MARGIN: f32 = 30.0;
    /// A reaction (states 11 / 12) that takes this many KH1 frames is released (KH1's longest,
    /// a launch's air time, lying (at most `down_time`, 30) and the get-up, is well under).
    pub const REACTION_LIMIT: f32 = 600.0;
    /// An AI polled in state 1 that issues nothing for this many KH1 frames is restarted. KH1's
    /// longest silent wait is the Search Ghost's search (`ex_2230.bd` `0x1044`: request 202, then
    /// [60, 600) frames with no request, steering or turn, then a frame more; M11); every ported
    /// script's before it is under 180 (the wander's idle). The long-fight tests stay under it with
    /// any command counted.
    pub const QUIET_LIMIT: f32 = 900.0;

    /// Why an action was released.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum Stuck {
        /// Its motion never started (an attack: Re:CoM's attack state never began).
        NeverStarted,
        /// Its motion never reached its end.
        NeverEnded,
    }

    /// `age`: KH1 frames since the request (`playing` false) or since its motion started
    /// (`playing` true); `frames`: its motion's KH1 length.
    pub fn check(playing: bool, age: f32, frames: f32) -> Option<Stuck> {
        if !playing {
            (age >= START_LIMIT).then_some(Stuck::NeverStarted)
        } else {
            (age >= frames + END_MARGIN).then_some(Stuck::NeverEnded)
        }
    }

    /// The loud log line (`<class>: eNNN: action NNN never started/ended (…), released`).
    pub fn line(class: &str, id: u32, request: u16, why: Stuck, detail: &str) -> String {
        let w = match why {
            Stuck::NeverStarted => "never started",
            Stuck::NeverEnded => "never ended",
        };
        format!("{class}: e{id}: action {request} {w} ({detail}), released")
    }
}

fn make_shadow(m: &Manifest, seed: u32, yaw: f32) -> Box<dyn ke::Behaviour> {
    let c = kh1_core::shadow::Consts::new(m.consts.pairs().to_vec()).expect("checked at parse");
    Box::new(kh1_core::shadow::Ai::new(c, m.frames(), seed, yaw, false))
}

fn parse_soldier(text: &str) -> Result<Manifest, String> {
    parse(text, "soldier", kh1_core::soldier::NAMES, SOLDIER_REQUESTS)
}

fn make_soldier(m: &Manifest, seed: u32, yaw: f32) -> Box<dyn ke::Behaviour> {
    Box::new(kh1_core::soldier::Ai::new(m.consts.clone(), m.frames(), seed, yaw, true))
}

/// KH1's world id (`sys 105`'s `+0`, the index of KH1's world code table `0x1404DCCA0`: 1 `di`,
/// 3 `tw`, 4 `aw`, 6 `po`, 8 `al`, 9 `lm`, 10 `nm`, 11 `he`, 12 `pi`, 13 `pp`, 15 `pc`; Checked
/// (exe)) for Re:CoM's world (`0x1402339C0(0)`: 1..13 in the order of its world table
/// `0x140506AA0`), as the asset step maps the twins' records (`kh1_recom_assets::stats::RECOM_WORLDS`,
/// decision 21). None: Twilight Town, Castle Oblivion (and its halls, 0), which KH1 doesn't have.
pub fn kh1_world(recom_world: i32) -> Option<u8> {
    const MAP: [Option<u8>; 13] = [Some(3), Some(8), Some(11), Some(4), Some(12), Some(10), Some(9), Some(13), Some(15), Some(6), None, Some(1), None];
    usize::try_from(recom_world).ok().and_then(|w| w.checked_sub(1)).and_then(|i| MAP.get(i).copied().flatten())
}

/// A new behaviour for a class (`seed`: its random stream; `yaw`: its facing, KH1's).
pub fn make(class: usize, m: &Manifest, seed: u32, yaw: f32) -> Box<dyn ke::Behaviour> {
    (CLASSES[class].make)(m, seed, yaw)
}

/// Parses a class's manifest.
pub fn parse_for(class: usize, text: &str) -> Result<Manifest, String> {
    (CLASSES[class].parse)(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// KH1's ring rule (`ex_2050_00.bd` `0x18`): heights first (|Δy| ≤ 0.5 m, inclusive), then the flat distance
    /// strictly inside (r − 1, r); no body radius; the inside of the ring misses.
    #[test]
    fn ring_rule() {
        let c = [10.0, 2.0, -4.0];
        let at = |d: f32, dy: f32| [10.0 + d * 0.6, 2.0 + dy, -4.0 + d * 0.8];
        let hit = |r: f32, d: f32, dy: f32| ring_hits(c, r, 1.0, 0.5, at(d, dy));
        // On the band.
        assert!(hit(3.0, 2.5, 0.0) && hit(3.0, 2.01, 0.0) && hit(3.0, 2.99, 0.0));
        // Inside the ring (it already passed): no hit, unlike Re:CoM's disk.
        assert!(!hit(3.0, 1.0, 0.0) && !hit(3.0, 0.0, 0.0));
        // Beyond the edge, and Re:CoM's body radius (0.25) isn't added.
        assert!(!hit(3.0, 3.1, 0.0) && !hit(3.0, 3.2, 0.0));
        // Both ends strict.
        assert!(!ring_hits(c, 3.0, 1.0, 0.5, [13.0, 2.0, -4.0]) && !ring_hits(c, 3.0, 1.0, 0.5, [12.0, 2.0, -4.0]));
        // Height: within 0.5 m either way hits, above (a jump) or below misses.
        assert!(hit(3.0, 2.5, 0.5) && hit(3.0, 2.5, -0.5) && hit(3.0, 2.5, 0.3));
        assert!(!hit(3.0, 2.5, 0.51) && !hit(3.0, 2.5, -0.51) && !hit(3.0, 2.5, 1.2));
        // At its start (r = 0) and its first step (r = 0.25, the band reaches inside 0): only (−0.75, 0.25).
        assert!(!hit(0.0, 0.0, 0.0));
        assert!(hit(0.25, 0.1, 0.0) && !hit(0.25, 0.3, 0.0));
        // KH1's numbers through the unit conversion.
        let ring = Ring { request: 230, start: 88.0, life: 28.0, growth: 25.0, band: 100.0, height: 50.0 };
        assert_eq!((ring.band_m(), ring.height_m()), (1.0, 0.5));
        let near = |t: f32, want: f32| (ring.radius(t).unwrap() - want).abs() < 1e-5;
        assert!(near(88.0, 0.0) && near(92.0, 1.0) && near(115.0, 6.75) && ring.radius(116.0).is_none());
    }

    /// Owner 2026-11-03 (T-posed Shadows in Halloween Town): KH1's AI only on a pack that carries every clip the
    /// manifest plays. The Shadow's own pack (`ENEMY/0001`, extended: 76 Re:CoM slots, KH1's at 100 + entry) carries
    /// them; Re:CoM's plain pack of its other id (`ENEMY/0032`, 76 slots) has none past 75; an empty slot counts as
    /// missing; no set, no KH1 AI.
    #[test]
    fn kh1_ai_only_on_a_pack_with_the_kh1_clips() {
        let clip = |request: u16, slot: u32| Clip { request, slot, frames: 30, root: false, looped: false };
        // Entry 6 on Re:CoM's bake (slot 5), the rise 69 at 143, the claw 200 at 137.
        let clips = [clip(6, 5), clip(69, 143), clip(200, 137)];
        let extended = |s: u32| s < 76 || (100..160).contains(&s);
        assert_eq!(pack::check(Some(160), &clips, extended), pack::Verdict::Carries { count: 160 });
        assert!(pack::check(Some(160), &clips, extended).carries());
        // Re:CoM's plain pack: the first KH1 clip is past its count; its own slots are never asked past it.
        let asked = std::cell::RefCell::new(Vec::new());
        let plain = |s: u32| {
            asked.borrow_mut().push(s);
            s < 76
        };
        assert_eq!(pack::check(Some(76), &clips, plain), pack::Verdict::Missing { count: 76, request: 69, slot: 143 });
        assert!(asked.borrow().iter().all(|&s| s < 76));
        // A pack as long as ours with the KH1 slot empty (an older asset step's pack).
        assert_eq!(pack::check(Some(160), &clips, |s| s != 137), pack::Verdict::Missing { count: 160, request: 200, slot: 137 });
        assert_eq!(pack::check(None, &clips, extended), pack::Verdict::NoSet);
        assert!(!pack::Verdict::NoSet.carries());
        // Logged once per class and id.
        let mut seen = Vec::new();
        assert!(pack::first_time(&mut seen, 0, 32));
        assert!(!pack::first_time(&mut seen, 0, 32));
        assert!(pack::first_time(&mut seen, 0, 1) && pack::first_time(&mut seen, 8, 32));
        let l = pack::line("shadow", "Shadow", 32, pack::Verdict::Missing { count: 76, request: 69, slot: 143 });
        assert!(l.starts_with("shadow: Re:CoM id 32 (ENEMY/0032): its motion pack has no clip for KH1's request 69 (slot 143; the pack has 76 slots)"), "{l}");
        assert!(l.contains("keep Re:CoM's AI"));
        assert!(pack::line("shadow", "Shadow", 1, pack::Verdict::Carries { count: 160 }).contains("ENEMY/0001): its motion pack carries the KH1 clips (160 slots): its Shadows get KH1's AI"));
    }

    /// Every ported class's Re:CoM id is the one its asset step extends, and the factory's shared cases (two ids, one
    /// class: Checked (decomp `0x140188F40`)) are the ones the pack check must catch for the ported classes.
    #[test]
    fn shared_factory_cases_of_ported_classes() {
        // (class's id, the factory's other id for the same ctor).
        const SHARED: [(u16, u16); 5] = [(1, 0x20), (0xE, 0x21), (0x10, 0x22), (0x11, 0x23), (0x1F, 0x24)];
        let ported: Vec<u16> = CLASSES.iter().map(|c| c.recom_id).collect();
        let caught: Vec<u16> = SHARED.iter().filter(|(a, _)| ported.contains(a)).map(|&(_, b)| b).collect();
        assert_eq!(caught, [0x20, 0x21], "the Shadow's id 32 and the Search Ghost's id 33");
        assert!(CLASSES.iter().all(|c| !SHARED.iter().any(|&(_, b)| b == c.recom_id)));
    }

    /// The body's position point: a column's lower end (whichever end it is), a sphere's lowest point.
    #[test]
    fn ring_body_foot() {
        let (lo, hi) = ([1.0, 0.1, 2.0], [1.0, 1.9, 2.0]);
        assert_eq!(body_foot(4, lo, hi, 0.25, 0.0), Some(lo));
        assert_eq!(body_foot(5, hi, lo, 0.25, 0.0), Some(lo));
        assert_eq!(body_foot(2, [9.0; 3], [1.0, 0.0, 2.0], 0.5, 0.5), Some([1.0, 0.0, 2.0]));
        assert_eq!(body_foot(7, lo, hi, 0.25, 0.0), None);
    }

    fn soldier_text() -> String {
        let consts: Vec<String> = kh1_core::soldier::NAMES.iter().chain(ke::ENGINE_NAMES).map(|n| format!("\"{n}\": 1.0")).collect();
        let reqs: Vec<u16> = SOLDIER_REQUESTS.iter().copied().chain([0x49, 0x4A, 0x4B, 218]).collect();
        let r: Vec<String> = reqs.iter().map(|q| format!("{{\"request\": {q}, \"entry\": 0, \"slot\": {}, \"name\": \"e002_100\", \"frames\": 30, \"root\": false, \"loop\": false, \"source\": \"kh1\"}}", 100 + q % 50)).collect();
        let a: Vec<String> = kh1_core::soldier::Attack::ALL.iter().map(|x| format!("{{\"name\": \"{}\", \"request\": {}, \"recoil\": {}}}", x.name(), x.request(), if x.rows()[0] == 0xEC { 218 } else { 110 })).collect();
        format!("{{\"format\": \"kh1-enemy 1\", \"enemy\": \"soldier\", \"consts\": {{{}}}, \"attacks\": [{}], \"parts\": [5, 3, 40, 41], \"requests\": [{}]}}", consts.join(", "), a.join(", "), r.join(", "))
    }

    #[test]
    fn soldier_manifest_parses_and_checks() {
        let m = parse_for(1, &soldier_text()).unwrap();
        assert_eq!(m.recoil.len(), 7);
        assert_eq!(m.recoil_of(0), Some(218));
        assert_eq!(m.recoil_of(2), None, "110 has no motion in the Soldier's model");
        assert_eq!(m.parts, vec![5, 3, 40, 41]);
        assert!(parse_for(1, &soldier_text().replace("\"soldier\"", "\"shadow\"")).is_err());
        assert!(parse_for(1, &soldier_text().replace("\"close_range\": 1.0, ", "")).is_err());
        assert!(parse_for(1, &soldier_text().replace("{\"request\": 208,", "{\"request\": 1208,")).is_err());
    }

    /// The asset step's real manifest (`KH1_SOLDIER_JSON`) parses here.
    #[test]
    fn asset_step_soldier_manifest_parses() {
        let Some(p) = std::env::var_os("KH1_SOLDIER_JSON") else { return };
        let m = parse_for(1, &std::fs::read_to_string(p).unwrap()).unwrap();
        assert_eq!(m.consts.get("close_range"), 350.0);
        assert_eq!(m.recoil_of(0), Some(218));
        let _ = make(1, &m, 1, 0.0);
    }

    /// M11, attack meets attack: the per-attack `clash` rows (the asset step's), their record lookup and
    /// the record index of a live attack entry.
    #[test]
    fn clash_rows_parse_and_look_up() {
        let consts: Vec<String> = ke::ENGINE_NAMES.iter().map(|n| format!("\"{n}\": 1.0")).collect();
        let r: Vec<String> = [0u16, 200, 207, 215, 218].iter().map(|q| format!("{{\"request\": {q}, \"entry\": 0, \"slot\": {}, \"name\": \"e009_100\", \"frames\": 30, \"root\": false, \"loop\": false, \"source\": \"kh1\"}}", 100 + q % 50)).collect();
        let punch = r#"{"name": "punch", "request": 200, "recoil": 215, "rows": ["0xec", "0x15c"], "clash": [{"row": "0xec", "type": 0, "priority": 30, "turn": 0.0, "cos": 1.0, "records": [2, 3]}, {"row": "0x15c", "type": 0, "priority": 30, "turn": 0.0, "cos": 1.0, "records": [4, 5]}]}"#;
        let charge = r#"{"name": "charge", "request": 207, "recoil": 218, "rows": ["0x23c"], "clash": [{"row": "0x23c", "type": 0, "priority": 90, "turn": 0.0, "cos": 1.0, "records": [7]}]}"#;
        let t = format!("{{\"format\": \"kh1-enemy 1\", \"enemy\": \"x\", \"consts\": {{{}}}, \"attacks\": [{punch}, {charge}], \"parts\": [5], \"requests\": [{}]}}", consts.join(", "), r.join(", "));
        let m = parse(&t, "x", &[], &[0]).unwrap();
        assert_eq!(m.clash.len(), 3);
        assert_eq!(m.clash_of(7).map(|c| (c.attack, c.clash_type, c.priority)), Some((1, 0, 90)));
        assert_eq!(m.clash_of(4).map(|c| (c.attack, c.priority)), Some((0, 30)));
        assert!(m.clash_of(0).is_none() && m.clash_of(6).is_none(), "Re:CoM's own records and others: none");
        // An older manifest (no "clash"): parses, no rows.
        let old = t.replace(r#", "clash": [{"row": "0x23c", "type": 0, "priority": 90, "turn": 0.0, "cos": 1.0, "records": [7]}]"#, "");
        assert_eq!(parse(&old, "x", &[], &[0]).unwrap().clash.len(), 2);
        assert!(parse(&t.replace("\"priority\": 90", "\"priority\": 190"), "x", &[], &[0]).is_err());
        // Entry `+0x18` against the set's records `+0x28` (0xA0 apart, `+0x24` of them).
        assert_eq!(record_index(0x1000 + 7 * 0xA0, 0x1000, 9), Some(7));
        assert_eq!(record_index(0x1000 + 9 * 0xA0, 0x1000, 9), None);
        assert_eq!(record_index(0x1000 + 0x50, 0x1000, 9), None);
        assert_eq!(record_index(0xF00, 0x1000, 9), None);
        assert!(!m.has_never_hurting() && !(0..10).any(|r| m.never_hurts(r)), "type 0 rows all hurt");
    }

    /// Owner 2026-11-02 ("Clash choices": block only): a record made from a KH1 row of clash type 1, 2 or 4 never
    /// hurts (Re:CoM's test of it against the player side answers 0); the rest of its attack's records, Re:CoM's own
    /// records and a type 3 row (the Wyvern's charge) still hurt.
    #[test]
    fn guard_type_rows_inside_attacks_never_hurt() {
        let consts: Vec<String> = ke::ENGINE_NAMES.iter().map(|n| format!("\"{n}\": 1.0")).collect();
        let r: Vec<String> = [0u16, 200, 207, 215, 218].iter().map(|q| format!("{{\"request\": {q}, \"entry\": 0, \"slot\": {}, \"name\": \"e002_100\", \"frames\": 30, \"root\": false, \"loop\": false, \"source\": \"kh1\"}}", 100 + q % 50)).collect();
        // The Soldier's spin kick: its foot (type 0, 100) and its chest `0x2AC` (type 1, 100, cone ±120°).
        let spin = r#"{"name": "spin kick", "request": 200, "recoil": 215, "rows": ["0x23c", "0x2ac"], "clash": [{"row": "0x23c", "type": 0, "priority": 100, "turn": 0.0, "cos": 1.0, "records": [2, 3]}, {"row": "0x2ac", "type": 1, "priority": 100, "turn": 0.0, "cos": -0.5, "records": [4]}]}"#;
        let charge = r#"{"name": "charge", "request": 207, "recoil": 218, "rows": ["0x1cc"], "clash": [{"row": "0x1cc", "type": 3, "priority": 100, "turn": 0.0, "cos": -1.0, "records": [5]}]}"#;
        let t = format!("{{\"format\": \"kh1-enemy 1\", \"enemy\": \"x\", \"consts\": {{{}}}, \"attacks\": [{spin}, {charge}], \"parts\": [5], \"requests\": [{}]}}", consts.join(", "), r.join(", "));
        let m = parse(&t, "x", &[], &[0]).unwrap();
        assert!(m.has_never_hurting());
        assert_eq!((0..8).filter(|&r| m.never_hurts(r)).collect::<Vec<_>>(), vec![4], "only the chest's record");
        // The Powerwild's flurry body (type 2) too; a type 4 row likewise.
        for (t2, never) in [(2, true), (4, true), (3, false), (0, false)] {
            let m2 = parse(&t.replace("\"type\": 1,", &format!("\"type\": {t2},")), "x", &[], &[0]).unwrap();
            assert_eq!(m2.never_hurts(4), never, "type {t2}");
        }
    }

    /// One swing frame against live rows (M11, attack meets attack): the Kingdom Key's 30.
    #[test]
    fn a_swing_frame_against_enemy_rows() {
        let row = |attack: u8, t: u32, p: i32| ClashRow { attack, clash_type: t, priority: p, turn: 0.0, cos: 1.0, records: vec![] };
        let (punch, charge, dive, chest, flurry_body) = (row(0, 0, 30), row(2, 0, 90), row(2, 0, 100), row(3, 1, 100), row(1, 2, 10));
        fn live(enemy: usize, r: &ClashRow) -> LiveRow<'_> {
            LiveRow { enemy, coll: enemy * 16, row: r, touches: true, front: true, met_before: false }
        }
        // A fist: both kind 6; the swing is marked (no damage this frame), so is the fist.
        let (sora, marked, m) = sweep((0, 30), [live(1, &punch)]);
        assert_eq!((sora, marked), (Some((6, 0)), true));
        assert_eq!(m, vec![Met { enemy: 1, coll: 16, gets: EnemyGets::Clash { attack: 0, kind: 6, diff: 0 }, stops: true }]);
        // A charging Large Body's belly: Sora the strong recoil, the Large Body kind 6 (its handler ends the charge).
        let (sora, _, m) = sweep((0, 30), [live(1, &charge)]);
        assert_eq!((sora, m[0].gets), (Some((7, -60)), EnemyGets::Clash { attack: 2, kind: 6, diff: 60 }));
        // A diving Neoshadow (100): Sora bounces (kind 7); the dive is not stopped but does him no harm (0.198.11: his clash
        // record's flag 0x80 comes before its hit record).
        let (sora, marked, m) = sweep((0, 30), [live(1, &dive)]);
        assert_eq!((sora, marked, m[0].gets, m[0].stops), (Some((7, -70)), true, EnemyGets::Nothing, true));
        // Two fists in one frame: Sora's first record only; the second fist still takes its clash.
        let (sora, _, m) = sweep((0, 30), [live(1, &punch), live(2, &punch)]);
        assert_eq!((sora, m.len(), m[1].gets), (Some((6, 0)), 2, EnemyGets::Clash { attack: 0, kind: 6, diff: 0 }));
        // Skipped: not touching, met before, outside a guard row's cone; Fire (type 5) meets no type-0 row.
        let mut l = live(1, &punch);
        l.touches = false;
        assert!(sweep((0, 30), [l]).2.is_empty());
        let mut l = live(1, &punch);
        l.met_before = true;
        assert!(sweep((0, 30), [l]).2.is_empty());
        let mut l = live(1, &chest);
        l.front = false;
        assert!(sweep((0, 30), [l]).2.is_empty());
        assert!(sweep((5, 30), [live(1, &punch)]).2.is_empty());
        // The Soldier's spin-kick chest (type 1, 100) from the front: the strong recoil; the Powerwild's flurry body
        // (type 2, 10): Sora kind 6 above it, the Powerwild kind 6 (a type 2 never repels).
        assert_eq!(sweep((0, 30), [live(1, &chest)]).0, Some((7, -70)));
        let (sora, _, m) = sweep((0, 30), [live(1, &flurry_body)]);
        assert_eq!((sora, m[0].gets), (Some((6, 20)), EnemyGets::Clash { attack: 1, kind: 6, diff: -20 }));
        // KH1 plays the strongest clash of the frame.
        let c6 = EnemyGets::Clash { attack: 0, kind: 6, diff: 0 };
        let c7 = EnemyGets::Clash { attack: 1, kind: 7, diff: -50 };
        assert_eq!(stronger(Some(c6), c7), Some(c7));
        assert_eq!(stronger(Some(c7), c6), Some(c7));
        assert_eq!(stronger(None, EnemyGets::Nothing), None);
        assert_eq!(stronger(Some(c6), EnemyGets::Repelled { attack: 0 }), Some(c6));
    }

    #[test]
    fn generic_manifest_parses_and_checks() {
        let consts: Vec<String> = ke::ENGINE_NAMES.iter().map(|n| format!("\"{n}\": 1.0")).collect();
        let r: Vec<String> = [0u16, 200, 218].iter().map(|q| format!("{{\"request\": {q}, \"entry\": 0, \"slot\": {}, \"name\": \"e009_100\", \"frames\": 30, \"root\": false, \"loop\": false, \"source\": \"kh1\"}}", 100 + q % 50)).collect();
        let t = format!("{{\"format\": \"kh1-enemy 1\", \"enemy\": \"x\", \"consts\": {{{}}}, \"attacks\": [{{\"name\": \"a\", \"request\": 200, \"recoil\": 218}}, {{\"name\": \"b\", \"request\": 200, \"recoil\": 110}}], \"parts\": [5, 3], \"requests\": [{}]}}", consts.join(", "), r.join(", "));
        let m = parse(&t, "x", &[], &[0, 200]).unwrap();
        assert_eq!((m.recoil_of(0), m.recoil_of(1)), (Some(218), None));
        assert!(parse(&t, "y", &[], &[]).is_err());
        assert!(parse(&t, "x", &["close_range"], &[]).is_err());
        assert!(parse(&t, "x", &[], &[7]).is_err());
    }

    /// Every class through the host's rules (`kh1_core::enemy::host::model`, what
    /// `win::kh1_enemy` decides) with its own attack-state map and the asset step's manifest
    /// (`KH1_<NAME>_JSON`, e.g. `KH1_BANDIT_JSON`; skipped when unset): in a long fight no attack
    /// is released and no steered attack is asked for again within two frames (0.73.0: every
    /// steered attack was released on its first step in its own state and asked for again, the
    /// Bandit's leap restarting every other frame).
    #[test]
    fn every_class_in_the_host_model() {
        use kh1_core::Vec3;
        for (i, c) in CLASSES.iter().enumerate() {
            let Some(p) = std::env::var_os(format!("KH1_{}_JSON", c.name.to_uppercase())) else { continue };
            let m = parse_for(i, &std::fs::read_to_string(p).unwrap()).unwrap();
            let loops: Vec<u16> = m.clips.iter().filter(|x| x.looped).map(|x| x.request).collect();
            for seed in 1..5u32 {
                let mut h = ke::host::model::Model::new(make(i, &m, seed, 0.0), m.consts.clone(), m.frames(), loops.clone(), c.attack_states.to_vec());
                for f in 0..20_000usize {
                    let a = f as f32 / 600.0;
                    let rad = 200.0 + 250.0 * (1.0 + (f as f32 / 1700.0).sin());
                    h.run(Vec3::new(a.sin() * rad, 0.0, a.cos() * rad), 1);
                }
                assert_eq!(h.released, 0, "{} seed {seed}", c.name);
                let issued: Vec<(usize, u16)> = h.log.iter().filter_map(|(f, c)| if let Cmd::Attack { request, .. } | Cmd::AttackAt { request, .. } = c { Some((*f, *request)) } else { None }).collect();
                for w in issued.windows(2) {
                    assert!(w[1].1 != w[0].1 || w[1].0 > w[0].0 + 2, "{} seed {seed}: {} asked again at {} after {}", c.name, w[0].1, w[1].0, w[0].0);
                }
                eprintln!("{} seed {seed}: {} steered attacks, {} commands", c.name, issued.len(), h.log.len());
            }
        }
    }

    #[test]
    fn classes_table() {
        for (i, c) in CLASSES.iter().enumerate() {
            assert_eq!(c.attack_states.len(), c.attack_names.len(), "{}", c.name);
            assert!(c.attack_states.iter().all(|k| (2..=10).contains(k)), "{}", c.name);
            assert_eq!(c.alloc_site.expect, &alloc_bytes(c.alloc), "{}", c.name);
            assert!(CLASSES.iter().enumerate().all(|(j, d)| j == i || (d.magic != c.magic && d.vtable != c.vtable)));
        }
        assert_eq!(replaced_states(CLASSES[0]), vec![0, 1, 2, 4, 3, 11, 12, 30]);
        assert_eq!(replaced_states(CLASSES[1]), vec![0, 1, 2, 3, 11, 12, 30]);
        assert_eq!(CLASSES[1].attack_names, kh1_core::soldier::Attack::ALL.map(|a| a.name()));
    }

    #[test]
    fn shadow_manifest_is_the_frameworks_too() {
        let Some(p) = std::env::var_os("KH1_SHADOW_JSON") else { return };
        let m = parse_for(0, &std::fs::read_to_string(p).unwrap()).unwrap();
        assert_eq!(m.recoil, vec![207, 207, 208]);
        assert_eq!(m.parts.len(), 2);
        let _ = make(0, &m, 1, 0.0);
    }

    #[test]
    fn tokens() {
        let n = |s: u8| ["idle", "engaged", "hit"][s as usize];
        assert_eq!(cmd_token(&Cmd::Action { request: 200, attack: Some(0), then: 203, blend: 8.0 }, CLASSES[0].attack_names, &n), "action 200 (claw) then 203");
        assert_eq!(cmd_token(&Cmd::State(1), CLASSES[0].attack_names, &n), "state engaged");
        assert!(logged(&Cmd::Held) && !logged(&Cmd::Yaw(0.0)));
        // M11 (the Darkball): its warps' model switch is logged, its per-frame scale isn't.
        assert_eq!(cmd_token(&Cmd::Visible(false), CLASSES[0].attack_names, &n), "visible false");
        assert_eq!(cmd_token(&Cmd::Scale(0.5), CLASSES[0].attack_names, &n), "scale 0.50");
        assert!(logged(&Cmd::Visible(true)) && !logged(&Cmd::Scale(1.0)));
        assert_eq!(cmd_token(&Cmd::MotionAt { request: 0, blend: 24.0, start: 16.0 }, CLASSES[0].attack_names, &n), "motion 0 b24 from 16");
        assert!(logged(&Cmd::MotionAt { request: 0, blend: 24.0, start: 16.0 }));
        // M11 (the Blue Rhapsody): a steered attack from a later frame.
        assert_eq!(cmd_token(&Cmd::AttackAt { request: 215, attack: 0, blend: 8.0, start: 8.0 }, CLASSES[0].attack_names, &n), "attack 215 (claw) from 8, steered");
        assert!(logged(&Cmd::AttackAt { request: 215, attack: 0, blend: 8.0, start: 8.0 }));
        // M11 (the Yellow Opera): a queued request from a later frame; a strike at a KH1 point.
        assert_eq!(cmd_token(&Cmd::QueueAt { request: 206, blend: 0.0, start: 2.0 }, CLASSES[0].attack_names, &n), "queue 206 b0 from 2");
        assert!(!logged(&Cmd::QueueAt { request: 206, blend: 0.0, start: 2.0 }));
        assert_eq!(cmd_token(&Cmd::Strike { child: 0, at: kh1_core::Vec3::new(1.0, 2.0, 3.0) }, CLASSES[0].attack_names, &n), "strike 0 at (1,2,3)");
        assert!(logged(&Cmd::Strike { child: 0, at: kh1_core::Vec3::new(1.0, 2.0, 3.0) }));
        assert_eq!(cmd_token(&Cmd::Warn { child: 0, at: kh1_core::Vec3::new(1.0, 2.0, 3.0) }, CLASSES[0].attack_names, &n), "mark 0 at (1,2,3): the warning until its strike");
        assert!(logged(&Cmd::Warn { child: 0, at: kh1_core::Vec3::new(1.0, 2.0, 3.0) }));
        // M11 (the Search Ghost): what KH1 does that isn't built is logged.
        assert_eq!(cmd_token(&Cmd::Note("x"), CLASSES[0].attack_names, &n), "note: x");
        assert!(logged(&Cmd::Note("x")));
        assert!(logged(&Cmd::HealFull) && logged(&Cmd::Reward(ke::Reward { exp_div: 10, exp_add: 1, exp_min: 0, prizes: [0, 0, 0, 0, 3, 0, 0] })));
        assert_eq!(cmd_token(&Cmd::Reward(ke::Reward { exp_div: 10, exp_add: 1, exp_min: 0, prizes: [0, 0, 0, 0, 3, 0, 0] }), CLASSES[0].attack_names, &n), "reward: EXP bonus EXP / 10 + 1, KH1 prizes [0, 0, 0, 0, 3, 0, 0]");
    }

    /// Re:CoM's worlds → KH1's world ids (decision 21's pairing): Traverse Town 3, Agrabah 8,
    /// Olympus 11, …; Twilight Town and Castle Oblivion none.
    #[test]
    fn recom_worlds_map_to_kh1_worlds() {
        assert_eq!((kh1_world(1), kh1_world(2), kh1_world(3)), (Some(3), Some(8), Some(11)));
        assert_eq!((kh1_world(10), kh1_world(11), kh1_world(12), kh1_world(13)), (Some(6), None, Some(1), None));
        assert_eq!((kh1_world(0), kh1_world(14), kh1_world(-1)), (None, None, None));
    }

    #[test]
    fn a_looping_clip_ends_at_its_wrap() {
        // The Soldier's kick 200: 110 KH1 frames (5500 units), looped in Re:CoM.
        let mut w = ClipWatch::default();
        w.restart();
        // The first read after the play still shows the last motion's clock: no wrap yet.
        assert!(!w.ended(4000, 110.0));
        w.tick(1.0);
        assert!(!w.ended(50, 110.0), "a clock going back early is the play's reset");
        let mut t = 50;
        for _ in 0..107 {
            w.tick(1.0);
            t += 50;
            assert!(!w.ended(t, 110.0));
        }
        // 5450 → the clip's end wraps to 0 before KH1's 5500.
        w.tick(1.0);
        assert!(w.ended(0, 110.0), "the wrap is the end");
        assert!(w.ended(50, 110.0), "and stays ended");
        // A clip that does not loop ends at KH1's length.
        let mut w = ClipWatch::default();
        assert!(!w.ended(5450, 110.0) && w.ended(5500, 110.0));
    }

    #[test]
    fn the_watchdog_releases_stuck_actions() {
        use watchdog::*;
        assert_eq!(check(false, 9.0, 110.0), None);
        assert_eq!(check(false, 10.0, 110.0), Some(Stuck::NeverStarted));
        assert_eq!(check(true, 139.0, 110.0), None);
        assert_eq!(check(true, 140.0, 110.0), Some(Stuck::NeverEnded));
        assert_eq!(line("soldier", 336, 200, Stuck::NeverEnded, "clock 50 of 5500"), "soldier: e336: action 200 never ended (clock 50 of 5500), released");
    }
}
