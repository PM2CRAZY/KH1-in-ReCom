//! [`SoraKit`]: everything about Sora that the controller, the combat code and the posing code
//! need, built from the player's `xa_ex_0010.mset`, `xa_ex_0010.mdls` and `btltbl.bin`.
//!
//! Where each value is in the files: `docs/kh1-sora-data.md` (all Checked (files)). How the exe
//! uses them: `docs/kh1-sora.md`.

use std::collections::HashMap;

use kh1_anim::{Rig, WeaponLine};
use kh1_core::motion::FlagKind;
use kh1_core::{ActionRow, AttackData, CombatParams, DamageCurve, MotionInfo, MotionSet, Moveset, Params, Vec3, WeaponData};
use kh1_formats::btltbl::{self, Btltbl, Globals};
use kh1_formats::mdls::Skeleton;
use kh1_formats::mset::{self, track, MapEntry};
use kh1_formats::Result;

use crate::GameDir;

/// Sora's motion set in `kh1_first.pkg`.
pub const MSET: &str = "xa_ex_0010.mset";
/// Sora's model (only the skeleton and its info block are read).
pub const MDLS: &str = "xa_ex_0010.mdls";
/// The battle table.
pub const BTLTBL: &str = "btltbl.bin";

/// Sora, built from the player's files.
#[derive(Clone, Debug)]
pub struct SoraKit {
    /// Movement, jump, glide and combo numbers: `kh1_defaults` (exe constants) with the file's
    /// values put in (jump heights, combo cones and ranges).
    pub params: Params,
    /// Motions (one [`MotionInfo`] per mset entry, entry indices kept), the request map, the
    /// action rows and the attack records they point at.
    pub moveset: Moveset,
    /// The Kingdom Key's weapon row (item 0x51).
    pub weapon: WeaponData,
    /// The Keyblade hit line for `kh1-anim` (same row).
    pub weapon_line: WeaponLine,
    /// The damage curve, MP charge and reaction numbers.
    pub combat: CombatParams,
    /// The btltbl globals block (file 0x58), for anything not taken into `params` yet.
    pub globals: Globals,
    /// Sora's skeleton (`xa_ex_0010.mdls`).
    pub skeleton: Skeleton,
    /// A rig of that skeleton, not posed yet (clone it per instance).
    pub rig: Rig,
    /// The motion set `kh1-anim` poses: `motions.motions[entry]` for a [`Moveset`] entry.
    pub motions: mset::MotionSet,
}

impl SoraKit {
    /// Reads the three files from the install and builds the kit.
    pub fn load(dir: &GameDir) -> Result<SoraKit> {
        let inst = dir.open()?;
        SoraKit::from_files(&inst.read(MSET)?, &inst.read(MDLS)?, inst.read(BTLTBL)?)
    }

    /// Builds the kit from the three files' bytes.
    pub fn from_files(mset_file: &[u8], mdls_file: &[u8], btltbl_file: Vec<u8>) -> Result<SoraKit> {
        let motions = mset::MotionSet::parse(mset_file)?;
        let skeleton = Skeleton::parse(mdls_file)?;
        let rig = Rig::from_mdls(mdls_file)?;
        let table = Btltbl::parse(btltbl_file)?;
        let globals = table.globals()?;

        let mut params = Params::kh1_defaults();
        params.jump_height = globals.sora_jump() as f32;
        params.high_jump_height = globals.sora_high_jump() as f32;
        params.combo.narrow_cone_deg = globals.narrow_cone_deg;
        params.combo.wide_cone_deg = globals.wide_cone_deg;
        params.combo.close_range = globals.close_range as f32;
        params.combo.lunge_range = globals.lunge_range as f32;

        let mut combat = CombatParams::kh1_defaults();
        combat.curve = DamageCurve(table.curve()?.values);
        combat.mp.per_level = globals.mp_charge_level_pct;
        combat.mp.bonus = globals.mp_charge_ability_pct;
        combat.mp.cap_percent = globals.mp_charge_cap_pct;
        combat.reaction.launch_speed_min = globals.launch_speed_min as f32;
        combat.reaction.launch_speed_max = globals.launch_speed_max as f32;
        combat.reaction.launch_height = globals.launch_height_factor;
        // G+0x3C: the air flinch's push speed (`0x1402B77D0`).
        combat.reaction.air_flinch_speed = globals.air_knockback[1];

        let w = table.weapon_for_item(btltbl::KINGDOM_KEY)?;
        let weapon = WeaponData {
            radius: w.radius,
            blade_far: w.blade_far,
            blade_near: w.blade_near,
            frac_scale: w.frac_scale,
            strength: w.strength as i32,
            crit: w.crit,
            crit_bonus: w.crit_bonus,
            priority: w.priority,
        };
        let weapon_line = WeaponLine { t0: w.blade_near, t1: w.blade_far, frac_scale: w.frac_scale, radius: w.radius };

        let moveset = build_moveset(&motions, &table)?;
        Ok(SoraKit { params, moveset, weapon, weapon_line, combat, globals, skeleton, rig, motions })
    }

    /// The mset motion of a moveset entry (what `kh1-anim` poses).
    pub fn motion(&self, entry: usize) -> Option<&mset::Motion> {
        self.motions.motions.get(entry)
    }

    /// The mset motion a request plays (through the moveset's request map).
    pub fn motion_for_request(&self, request: u16) -> Option<&mset::Motion> {
        self.motion(self.moveset.motions.entry(request)?)
    }
}

/// The entry a request plays. Types 3 / 4 (field / battle / no-weapon variants) take entry + 1,
/// the battle variant. Assumption, carried from `kh1-core`'s `Moveset::kh1_defaults` (idle 1,
/// walk 3, run 6): the stance pick of `0x1402A01E0` is not modelled.
pub fn battle_entry(m: MapEntry) -> Option<u16> {
    let e = m.entry()?;
    Some(match m.kind() {
        3 | 4 => e + 1,
        _ => e,
    })
}

/// A flag track kind as `kh1-core` names it (hit slots, combo window, free, trail); others are
/// triggers, not flags.
fn flag_kind(kind: u16, sub: u16) -> Option<FlagKind> {
    match (kind, sub) {
        (track::HIT, s) if s < 4 => Some(FlagKind::Hit(s as u8)),
        (track::COMBO, 0) => Some(FlagKind::ComboWindow),
        (track::FREE, 0) => Some(FlagKind::Free),
        (track::TRAIL, 0) => Some(FlagKind::Trail),
        _ => None,
    }
}

/// One mset motion's flag tracks as `kh1-core` windows. The exe's lookup (`0x140394770`) takes
/// the first track of a type and sub, and the last key with time ≤ t decides (non-zero = on), so
/// each "on" key opens a window that runs to the next key, or forever after the last one.
pub fn flag_windows(m: &mset::Motion) -> Vec<kh1_core::FlagWindow> {
    let mut out = Vec::new();
    let mut seen: Vec<(u16, u16)> = Vec::new();
    for tr in &m.tracks {
        let Some(kind) = flag_kind(tr.kind, tr.sub) else { continue };
        if seen.contains(&(tr.kind, tr.sub)) {
            continue;
        }
        seen.push((tr.kind, tr.sub));
        for (i, k) in tr.keys.iter().enumerate() {
            if k.value() == 0 {
                continue;
            }
            let end = tr.keys.get(i + 1).map_or(f32::INFINITY, |n| n.time);
            if end > k.time {
                out.push(kh1_core::FlagWindow::new(kind, k.time, end));
            }
        }
    }
    out
}

/// Cumulative root travel at integer frames `0..=length`, from the last rig helper (motion
/// space: +z forward, x sideways, y down), relative to frame 0.
pub fn root_curve(m: &mset::Motion) -> Vec<Vec3> {
    let p0 = m.root_position(0.0);
    (0..=m.frames)
        .map(|f| {
            let p = m.root_position(f as f32);
            Vec3::new(p[0] - p0[0], p[1] - p0[1], p[2] - p0[2])
        })
        .collect()
}

/// A `kh1-core` action row from the file's.
pub fn row_from_file(r: &btltbl::ActionRow) -> ActionRow {
    ActionRow {
        request: r.motion,
        trail: r.trail,
        weapon_effect: r.weapon_effect,
        attack: r.attack,
        hop: r.hop,
        sounds: [r.hit_sound, r.wall_sound, r.crit_sound],
        voice: r.voice,
    }
}

/// A `kh1-core` attack record from the file's (the fields the hit and damage code read).
pub fn attack_from_file(a: &btltbl::AttackData) -> AttackData {
    AttackData {
        stat_source: a.stat_source as u8,
        element: (a.element & 0xF) as u8,
        kind_party: (a.kind_party & 0x1F) as u8,
        kind: a.kind_enemy as u8,
        bone: a.bone,
        power_party: a.power_party,
        min_party: a.min_party,
        max_party: a.max_party,
        crit: a.crit_chance,
        crit_bonus: a.crit_bonus,
        mp_charge: a.mp_charge,
        hit_slot: a.hit_slot(),
        clash: a.clash,
        priority: a.priority,
        recoil_request: a.clash_motion,
        power: a.power,
        min: a.min,
        max: a.max,
        frac_a: a.frac_a,
        frac_b: a.frac_b,
        radius: a.radius,
    }
}

/// The whole moveset: one entry per mset motion (same indices, so the host can pose
/// `motions[entry]`), every request mapped, the root curve on entries a root-motion request
/// plays, Sora's 0x42 action rows and every attack record they point at.
pub fn build_moveset(ms: &mset::MotionSet, table: &Btltbl) -> Result<Moveset> {
    let mut set = MotionSet::default();
    for m in &ms.motions {
        let mut info = MotionInfo::new(m.frames as f32);
        info.flags = flag_windows(m);
        set.entries.push(info);
    }
    for (id, word) in ms.requests.iter().enumerate() {
        let Some(e) = battle_entry(*word).map(usize::from).filter(|&e| e < set.entries.len()) else { continue };
        set.map_request(id as u16, e);
        if word.root_motion() {
            set.entries[e].root_motion = true;
        }
    }
    for (e, info) in set.entries.iter_mut().enumerate() {
        if info.root_motion {
            info.root = root_curve(&ms.motions[e]);
        }
    }
    let rows: Vec<ActionRow> = table.sora_rows()?.iter().map(row_from_file).collect();
    let mut attacks = HashMap::new();
    for r in &rows {
        if r.request < 0 || attacks.contains_key(&r.attack) {
            continue;
        }
        if let Ok(a) = table.attack(r.attack as usize) {
            attacks.insert(r.attack, attack_from_file(&a));
        }
    }
    Ok(Moveset { motions: set, rows, attacks })
}

/// Builds a game folder at `root` from a test-data folder laid out like `kh1-formats`' tests
/// (`hed/kh1_*.hed`, `pkg/<name>.pkgent`, `key.exe`): every hed, a sparse `kh1_first.pkg` with
/// the three entries at their hed offsets, and `key.exe` copied as KH1's exe. `None` when the
/// test data lacks those files. For tests and the playground's headless check only.
#[doc(hidden)]
pub fn fake_install(testdata: &std::path::Path, root: &std::path::Path) -> Option<std::path::PathBuf> {
    use std::io::{Seek, SeekFrom, Write};
    use kh1_formats::egs::{Archive, KH1_ARCHIVES, KH1_EXE};
    let hed_dir = testdata.join("hed");
    let first = Archive::open(hed_dir.join("kh1_first.hed")).ok()?;
    let key = testdata.join("key.exe");
    if !key.is_file() {
        return None;
    }
    let dt = root.join("Image").join("dt");
    std::fs::create_dir_all(&dt).ok()?;
    for stem in KH1_ARCHIVES {
        let src = hed_dir.join(format!("{stem}.hed"));
        if src.is_file() {
            std::fs::copy(&src, dt.join(format!("{stem}.hed"))).ok()?;
        }
    }
    let mut pkg = std::fs::File::create(dt.join("kh1_first.pkg")).ok()?;
    for name in [MSET, MDLS, BTLTBL] {
        let raw = std::fs::read(testdata.join("pkg").join(format!("{name}.pkgent"))).ok()?;
        let e = first.find(name)?;
        pkg.seek(SeekFrom::Start(e.offset)).ok()?;
        pkg.write_all(&raw).ok()?;
    }
    std::fs::copy(&key, root.join(KH1_EXE)).ok()?;
    Some(root.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testdata;

    fn kit() -> Option<SoraKit> {
        let mset = testdata::read(MSET)?;
        let mdls = testdata::read(MDLS)?;
        let bt = testdata::read(BTLTBL)?;
        Some(SoraKit::from_files(&mset, &mdls, bt).unwrap())
    }

    /// A window list with ends at or past the motion's end written as "to the end" (both are
    /// the same flag word for every frame the motion plays before it ends).
    fn norm(info: &MotionInfo) -> Vec<(FlagKind, f32, f32)> {
        let mut v: Vec<_> = info.flags.iter().map(|w| (w.kind, w.start, if w.end >= info.length { f32::INFINITY } else { w.end })).collect();
        v.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
        v
    }

    /// Everything kh1-core's `kh1_defaults` took from the same files matches what the files give.
    #[test]
    fn matches_kh1_core_defaults() {
        let Some(k) = kit() else { return };
        assert_eq!(k.params, Params::kh1_defaults());
        // The file's f32s (-3.6999998, 0.79999995) are the defaults' decimals to f32 rounding.
        let (w, kk) = (&k.weapon, WeaponData::kingdom_key());
        assert!((w.blade_far - kk.blade_far).abs() < 1e-6 && (w.frac_scale - kk.frac_scale).abs() < 1e-6);
        assert_eq!(WeaponData { blade_far: kk.blade_far, frac_scale: kk.frac_scale, ..w.clone() }, kk);
        let mut c = k.combat.clone();
        assert!((c.reaction.launch_height - 1.35).abs() < 1e-6, "file 1.3499999");
        c.reaction.launch_height = 1.35;
        assert_eq!(c, CombatParams::kh1_defaults());
        assert_eq!(k.combat.curve, DamageCurve::kh1_defaults());

        let d = Moveset::kh1_defaults();
        let (dm, fm) = (&d.motions, &k.moveset.motions);
        for (r, e) in dm.map.iter().enumerate() {
            let Some(e) = e else { continue };
            assert_eq!(fm.entry(r as u16), Some(*e), "request {r}");
        }
        for (e, want) in dm.entries.iter().enumerate() {
            if want.length == 0.0 {
                continue; // not described in the defaults
            }
            let got = &fm.entries[e];
            assert_eq!(got.length, want.length, "entry {e}");
            assert_eq!(got.root_motion, want.root_motion, "entry {e}");
            assert_eq!(norm(got), norm(want), "entry {e}");
            if !want.root.is_empty() {
                assert!((got.root_at(want.length).z - want.root_at(want.length).z).abs() < 0.01, "entry {e}");
            }
        }
        // The defaults' root keys (frame, forward units), from the same curves.
        let keys: [(usize, &[(f32, f32)]); 6] = [
            (98, &[(8.0, 10.4), (14.0, 48.33), (34.0, 100.0), (48.0, 100.0)]),
            (99, &[(12.0, 0.0), (24.0, 57.93), (48.0, 120.0)]),
            (100, &[(18.0, 0.01), (20.0, 10.84), (24.0, 81.86), (28.0, 118.98), (32.0, 129.64), (40.0, 134.5), (64.0, 134.94)]),
            (101, &[(20.0, 44.33), (26.0, 97.62), (32.0, 119.43), (56.0, 136.11), (68.0, 146.33), (80.0, 190.0)]),
            (111, &[(4.0, 12.86), (6.0, 44.08), (14.0, 163.77), (24.0, 237.5), (36.0, 267.05), (86.0, 267.05)]),
            (117, &[(12.0, 213.82), (24.0, 350.0), (38.0, 350.0)]),
        ];
        for (e, ks) in keys {
            for &(f, z) in ks {
                let got = fm.entries[e].root_at(f);
                assert!((got.z - z).abs() < 0.01 && got.x == 0.0 && got.y == 0.0, "entry {e} frame {f}: {got:?} vs {z}");
            }
        }
        for (r, want) in d.rows.iter().enumerate() {
            if want.request >= 0 {
                assert_eq!(&k.moveset.rows[r], want, "row {r}");
            }
        }
        for (off, want) in &d.attacks {
            let got = k.moveset.attack(*off).unwrap();
            assert!((got.frac_b - want.frac_b).abs() < 1e-6, "attack {off:#x}");
            assert_eq!(&AttackData { frac_b: want.frac_b, ..got.clone() }, want, "attack {off:#x}");
        }
    }

    /// The numbers in `docs/kh1-sora-data.md`.
    #[test]
    fn documented_numbers() {
        let Some(k) = kit() else { return };
        let p = &k.params;
        assert_eq!((p.jump_height, p.high_jump_height), (190.0, 290.0));
        assert_eq!((p.combo.narrow_cone_deg, p.combo.wide_cone_deg, p.combo.close_range, p.combo.lunge_range), (45, 240, 180.0, 300.0));
        assert_eq!(k.weapon.radius, 40.0);
        let l = k.weapon_line;
        assert!(l.t0 == 0.0 && (l.t1 + 3.7).abs() < 1e-6 && (l.frac_scale - 0.8).abs() < 1e-6 && l.radius == 40.0);
        let ms = &k.moveset;
        // (row, request, length, hit, combo window, root travel, attack record)
        type Row = (usize, i16, f32, (f32, f32), Option<(f32, f32)>, f32, u32);
        let table: [Row; 10] = [
            (13, 200, 48.0, (14.0, 24.0), Some((22.0, 40.0)), 100.0, 0x4B78),
            (11, 201, 48.0, (18.0, 32.0), Some((26.0, 42.0)), 120.0, 0x4BE8),
            (14, 202, 64.0, (18.0, 30.0), Some((26.0, 54.0)), 134.94, 0x4B78),
            (8, 203, 80.0, (26.0, 38.0), None, 190.0, 0x4C58),
            (18, 204, 40.0, (6.0, 20.0), Some((18.0, 34.0)), 0.0, 0x4B78),
            (17, 205, 42.0, (10.0, 20.0), Some((18.0, 32.0)), 0.0, 0x4BE8),
            (16, 206, 56.0, (12.0, 38.0), None, 0.0, 0x4C58),
            (2, 204, 40.0, (6.0, 20.0), Some((18.0, 34.0)), 0.0, 0x4B78),
            (0x3D, 212, 54.0, (10.0, 36.0), Some((22.0, 54.0)), 0.0, 0x4DA8),
            (0x3E, 213, 86.0, (8.0, 24.0), Some((18.0, 56.0)), 267.05, 0x4E18),
        ];
        for (r, req, len, hit, combo, travel, atk) in table {
            let row = ms.row(r).unwrap();
            assert_eq!((row.request, row.attack), (req, atk), "row {r}");
            let info = ms.motions.info(req as u16).unwrap();
            assert_eq!(info.length, len, "row {r}");
            let slot = ms.attack(atk).unwrap().hit_slot;
            let w = |kind: FlagKind| info.flags.iter().filter(|w| w.kind == kind).map(|w| (w.start, w.end.min(len))).collect::<Vec<_>>();
            assert_eq!(w(FlagKind::Hit(slot)), vec![hit], "row {r}");
            assert_eq!(w(FlagKind::ComboWindow), combo.into_iter().collect::<Vec<_>>(), "row {r}");
            assert!(w(FlagKind::Free).is_empty(), "row {r}: no free track");
            let t = if info.root_motion { info.root_at(len).z } else { 0.0 };
            assert!((t - travel).abs() < 0.01, "row {r}: {t}");
        }
        assert_eq!([ms.row(18).unwrap().hop, ms.row(17).unwrap().hop, ms.row(16).unwrap().hop, ms.row(2).unwrap().hop], [20.0, 20.0, 40.0, 60.0]);
        // Swings flinch (kind 3, power 0); finishers launch (kind 5, power 4); MP charge 3 / 6 / 24.
        let a = |o: u32| ms.attack(o).unwrap();
        assert_eq!((a(0x4B78).kind, a(0x4B78).power, a(0x4B78).mp_charge), (3, 0, 3));
        assert_eq!((a(0x4C58).kind, a(0x4C58).power, a(0x4C58).mp_charge), (5, 4, 6));
        assert_eq!(a(0x4E18).mp_charge, 24);
        // Dodge Roll: 38 frames, free from 30, 213.82 in frames 0-12 and 350 in all.
        let roll = ms.motions.info(0xDC).unwrap();
        assert_eq!((roll.length, roll.first(FlagKind::Free)), (38.0, Some(30.0)));
        assert!((roll.root_at(12.0).z - 213.82).abs() < 0.01 && (roll.root_at(38.0).z - 350.0).abs() < 0.01);
        // Land 28 frames, free from 12; jump entries 8-11, High Jump 12, 13 then 10, 11.
        let land = ms.motions.info(7).unwrap();
        assert_eq!((land.length, land.first(FlagKind::Free)), (28.0, Some(12.0)));
        let e = |r: u16| ms.motions.entry(r).unwrap();
        assert_eq!([e(4), e(5), e(6), e(7)], [8, 9, 10, 11]);
        assert_eq!([e(9), e(10), e(11), e(12)], [12, 13, 10, 11]);
        // Idle / walk / run take the battle variants.
        assert_eq!([e(0), e(1), e(2)], [1, 3, 6]);
        // Skeleton and motion set for kh1-anim.
        assert_eq!(k.skeleton.len(), 293);
        assert_eq!(k.rig.bone_count(), 293);
        assert_eq!(k.motions.motions.len(), 118);
        assert_eq!(ms.motions.entries.len(), 118);
        assert_eq!(k.motion_for_request(200).unwrap().frames, 48);
        assert_eq!(k.rig.attach_bone(kh1_anim::WEAPON_ATTACH), Some(283));
    }

    /// An install folder assembled from the test data (the hed files, a sparse kh1_first.pkg
    /// with the three entries at their offsets, the key-carrying exe under KH1's exe name)
    /// loads the same kit as the loose files.
    #[test]
    fn loads_from_an_install_folder() {
        let Some(dir) = testdata::dir() else { return };
        let Some(root) = crate::sora::fake_install(&dir, &std::env::temp_dir().join(format!("kh1-data-install-{}", std::process::id()))) else {
            eprintln!("skipping: no hed/pkg test data");
            return;
        };
        let g = GameDir::at(&root).unwrap();
        let k = SoraKit::load(&g).unwrap();
        let loose = kit().unwrap();
        assert_eq!(k.params, loose.params);
        assert_eq!(k.moveset, loose.moveset);
        assert_eq!(k.combat, loose.combat);
        let _ = std::fs::remove_dir_all(&root);
    }
}
