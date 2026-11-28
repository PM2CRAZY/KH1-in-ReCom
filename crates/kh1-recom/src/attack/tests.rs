use super::*;

#[test]
fn kh1_rows_keep_recoms_clip_and_take_kh1s_windows() {
    let m = Moveset::kh1_defaults();
    let t = kh1_table(&RECOM_ROWS_1_7, &m, &|r| (r == 213).then_some(0x48));
    // Row 13 (motion 200): Re:CoM row 1's action 0x30 and effect 1; KH1 hit 14–24 held one frame
    // earlier for Re:CoM's exe-time test; combo window 22–40; kind 1 (flinch); only flag 0x80.
    let r = t[13];
    assert_eq!(r[0], 0x30);
    assert_eq!(r[2], 1);
    assert_eq!((r[4], r[5]), (13, 23));
    assert_eq!((r[1], r[3]), (22, 40));
    assert_eq!(&r[6..10], &[0, 0, 0, 0], "no homing");
    assert_eq!(r[0xB], 1);
    assert_eq!((r[0xC], r[0xD]), (0, 0));
    assert_eq!(r[0xF], FLAG_KH1);
    // The finisher (row 8, motion 203): hit 26–38, no window, kind 2 (blow-away), clip 0x33.
    let f = t[8];
    assert_eq!((f[0], f[4], f[5], f[1], f[3], f[0xB]), (0x33, 25, 37, 0, 0, 2));
    assert_eq!(row_window(&f), None);
    assert_eq!(row_window(&t[11]), Some((26.0, 42.0)));
    // Air rows: 18 → Re:CoM's id 6 clip (0x35), 17 → id 5 (0x34), 16 → id 7 (0x36).
    assert_eq!((t[18][0], t[17][0], t[16][0]), (0x35, 0x34, 0x36));
    assert_eq!((t[18][4], t[18][5]), (5, 19));
    // Rows Re:CoM can't play stay empty.
    assert_eq!(t[3], [0u8; 16]);
    // Counterattack (row 0x3E, motion 213): the appended clip row, hit 8–24 a frame earlier,
    // window 18–56, flinch; none without the clip.
    let c = t[COUNTERATTACK_ROW];
    assert_eq!((c[0], c[4], c[5], c[1], c[3], c[0xB]), (0x48, 7, 23, 18, 56, 1));
    assert_eq!(kh1_table(&RECOM_ROWS_1_7, &m, &|_| None)[COUNTERATTACK_ROW], [0u8; 16]);
    // Re:CoM's own rows never carry flag 0x80 (the gate's KH1 test leaves them alone).
    assert!(RECOM_ROWS_1_7.iter().all(|r| r[0xF] & FLAG_KH1 == 0 && row_window(r).is_none()));
}

#[test]
fn row_map_follows_the_motions() {
    let m = Moveset::kh1_defaults();
    for &(r, id) in ROW_MAP.iter() {
        let req = m.row_request(r);
        let clip = 0x30 + id - 1;
        assert_eq!(RECOM_ROWS_1_7[id as usize - 1][0] as u32, clip);
        // Re:CoM ids 1..7 play KH1 motions 200, 201, 202, 203, 205, 204, 206.
        let expect = [200, 201, 202, 203, 205, 204, 206][id as usize - 1];
        assert_eq!(req, expect, "row {r}");
    }
    assert_eq!(recom_id(13), Some(1));
    assert_eq!(recom_id(3), Some(4), "Stun Impact takes the finisher's id (M9 9a)");
    assert_eq!(recom_id(12), None, "row 12 is never picked");
    assert!(swing_root(&m, 200) && swing_root(&m, 203));
    assert!(!swing_root(&m, 204) && !swing_root(&m, 206));
    assert!(is_swing_request(206) && !is_swing_request(212) && !is_swing_request(213));
    assert!(is_swing_request(208) && is_swing_request(218) && is_swing_request(214));
    assert_eq!((recom_kind(3), recom_kind(5)), (1, 2));
}

#[test]
fn blade_is_two_spheres_on_the_kingdom_key_plus_the_sweep() {
    let m = Moveset::kh1_defaults();
    let w = WeaponData::kingdom_key();
    let live = live_attack(&m, 13, &w).unwrap();
    // Radius 0 + 1.0 × 40 = 40 units (0.40 m), fractions 1.0 / 0.0 × 0.8.
    assert_eq!(live.radius, 40.0);
    // Re:CoM metres: wep0 at the origin, k9 0.2111 m along −x (any direction works).
    let w2 = Vec3::ZERO;
    let w3 = Vec3::new(-0.2111, 0.0, 0.0);
    let (a, b) = blade_points(w2, w3, &live, &w);
    assert!((a.x - 0.2111 * 2.96).abs() < 1e-5, "A at t = −2.96: {a:?}");
    assert_eq!(b, w2);
    let r = live.radius * crate::units::METRES_PER_UNIT;
    // Without a sweep: spheres at A and A + 0.4 toward B (0.625 m of blade).
    let s = blade_shapes(a, b, r, false, None, Vec3::ZERO);
    assert_eq!(s.len(), 2);
    assert!((s[1].0.x - (a.x - 0.4)).abs() < 1e-5);
    // The first tested frame sweeps from the owner's centre; the next from the last A.
    let owner = Vec3::new(0.0, 0.0, -1.0);
    let first = blade_shapes(a, b, r, true, None, owner);
    assert!(first.len() > 2);
    let next = blade_shapes(a, b, r, true, Some(a), owner);
    assert_eq!(next.len(), 3, "a still blade sweeps one sphere at A");
}

#[test]
fn blade_track_moves_on_once_per_pose() {
    let mut t = BladeTrack::default();
    let a1 = Vec3::new(1.0, 0.0, 0.0);
    let a2 = Vec3::new(1.0, 0.1, 0.0);
    assert_eq!(t.frame(5, a1), None, "a new swing seeds from the owner");
    assert_eq!(t.frame(5, a1), None, "the same frame tested twice");
    assert_eq!(t.frame(5, a2), Some(a1));
    assert_eq!(t.frame(5, a2), Some(a1));
    assert_eq!(t.frame(6, a2), None, "the next swing seeds again");
}

fn twin(defence: i32) -> EnemyStats {
    EnemyStats { hp: 12, strength: 8, defence, min_damage: 0, max_damage: 0, level: 1, exp: 1, resist: [100; 16] }
}

#[test]
fn kh1_damage_uses_the_twins_defence_curve() {
    let m = Moveset::kh1_defaults();
    let mut rng = kh1_core::Kh1Rand::new(1);
    // Defence 8, Strength 10 swing → curve[2] = 2.
    assert_eq!(kh1_damage(&m, 13, 10, &WeaponData::kingdom_key(), &twin(8), &mut rng).unwrap().damage, 2);
    // The finisher adds 4 power → curve[6] = 6 (or a critical 10 + 4 → curve[10] = 12).
    for _ in 0..20 {
        let d = kh1_damage(&m, 8, 10, &WeaponData::kingdom_key(), &twin(8), &mut rng).unwrap();
        assert_eq!(d.damage, if d.critical { 12 } else { 6 });
    }
    // A cap of 1 per hit (KH1's record +0x10) holds.
    let mut capped = twin(0);
    capped.max_damage = 1;
    assert_eq!(kh1_damage(&m, 8, 40, &WeaponData::kingdom_key(), &capped, &mut rng).unwrap().damage, 1);
}

#[test]
fn stats_json_reads_the_asset_steps_format() {
    let rec = r#"{"hp": 12, "strength": 8, "defence": 8, "floor": 2, "cap": 300, "level": 6, "exp": 1, "resist": [100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100]}"#;
    let worlds = vec![rec; 13].join(", ");
    let strength = (1..=100).map(|l| (6 + l).to_string()).collect::<Vec<_>>().join(", ");
    let t = format!(r#"{{"format": "kh1-recom-assets stats 2", "sora_strength": [{strength}], "enemies": [{{"class": "CEnemyShadow", "lock_radius": 30, "lock_radii": [30, 30], "worlds": [{worlds}]}}]}}"#);
    let s = parse_stats(&t).unwrap();
    assert_eq!(s.lock_radius("CEnemyShadow"), Some(30.0));
    assert_eq!(s.lock_radius("CEnemySoldier"), None);
    // Format 1 (no lock radius) still reads; the radius is then unknown.
    let old = parse_stats(&t.replace(r#""lock_radius": 30, "#, "")).unwrap();
    assert_eq!(old.lock_radius("CEnemyShadow"), None);
    // Format 3: base Strength plus the equipped Keyblade's, and the Keyblade rows.
    assert_eq!(s.strength_with(1, &WeaponData::kingdom_key()), Some(7), "format 2: the Kingdom Key's table");
    let base = (1..=100).map(|l| (3 + l).to_string()).collect::<Vec<_>>().join(", ");
    let kb = r#"{"card": 15, "name": "Ultima Weapon", "source": "kh1", "kh1_item": 100, "model": "xw_ex_5160", "radius": 50, "blade_far": -5.6, "blade_near": 0, "frac_scale": 0.8, "strength": 14, "crit": 40, "crit_bonus": 2, "priority": 90, "magic": 2}"#;
    let v3 = parse_stats(&t.replace(r#""sora_strength""#, &format!(r#""sora_base_strength": [{base}], "keyblades": [{kb}], "sora_strength""#))).unwrap();
    let u = v3.keyblade(15).unwrap();
    assert_eq!((u.name.as_str(), u.weapon.radius, u.weapon.strength, u.weapon.priority, u.magic), ("Ultima Weapon", 50.0, 14, 90, 2));
    assert_eq!(v3.strength_with(1, &u.weapon), Some(4 + 14));
    assert_eq!(v3.strength_with(1, &WeaponData::kingdom_key()), Some(4 + 3));
    assert!(v3.keyblade(4).is_none());
    // Format 3 has no model groups: no model swap.
    assert_eq!((u.group, u.textures.len()), (None, 0));
    assert_eq!(crate::keyblade_models::Models::from_stats(&v3).unwrap(), None);
    // Format 4: each converted Keyblade names its PLAYER group and textures.
    let kb4 = kb.replace(r#""magic": 2}"#, r#""magic": 2, "group": 34, "textures": ["xw_ex_5160_0", "xw_ex_5160_1"]}"#);
    let kk = kb.replace(r#""card": 15, "name": "Ultima Weapon""#, r#""card": 1, "name": "Kingdom Key""#);
    let v4 = parse_stats(&t.replace(r#""sora_strength""#, &format!(r#""sora_base_strength": [{base}], "keyblades": [{kk}, {kb4}], "sora_strength""#))).unwrap();
    let u4 = v4.keyblade(15).unwrap();
    assert_eq!((u4.group, u4.textures.clone()), (Some(34), vec!["xw_ex_5160_0".to_string(), "xw_ex_5160_1".to_string()]));
    let m = crate::keyblade_models::Models::from_stats(&v4).unwrap().unwrap();
    assert_eq!((m.group_of(15), m.group_of(1)), (Some(34), Some(2)));
    assert!(parse_stats(&t.replace(r#""sora_strength""#, &format!(r#""keyblades": [{}], "sora_strength""#, kb.replace(r#""magic": 2}"#, r#""magic": 2, "group": -3}"#)))).is_err());
    assert_eq!(s.strength(1), Some(7));
    assert_eq!(s.strength(150), Some(106));
    assert_eq!(s.twin("CEnemyShadow", 1).unwrap().defence, 8);
    assert!(s.twin("CEnemyShadow", 0).is_none() && s.twin("CEnemyShadow", 14).is_none());
    assert!(s.twin("CEnemySoldier", 1).is_none());
    assert!(parse_stats("{}").is_err());
}

#[test]
fn enp_rows_and_proxy_worlds_follow_recoms_stats_init() {
    // `0x14017FF50`: floor − 1; 0xE → 0, 0xF → 2, 100 → 0, 0 → 0; Riku's floors refused.
    assert_eq!((enp_row(1), enp_row(13), enp_row(0), enp_row(0xE), enp_row(0xF), enp_row(100)), (Some(0), Some(12), Some(0), Some(0), Some(2), Some(0)));
    assert_eq!((enp_row(-1), enp_row(-12), enp_row(16)), (None, None, None));
    assert_eq!((proxy_world(0), proxy_world(5), proxy_world(13), proxy_world(14), proxy_world(-1)), (Some(13), Some(5), Some(13), None, None));
    assert!(is_boss_id(102) && is_boss_id(201) && !is_boss_id(29) && !is_boss_id(99));
    assert_eq!((proxy_hp(225, 0.5), proxy_hp(1, 0.1), proxy_hp(33, 12.0 / 33.0)), (113, 1, 12));
}

/// A format-5 file in the asset step's shape (`kh1-recom-assets` `stats::build_json`).
fn stats_v5() -> String {
    let rec = |hp: i32, def: i32, cap: i32, grav: i32, stop: i32| {
        format!(r#"{{"hp": {hp}, "strength": {def}, "defence": {def}, "floor": 2, "cap": {cap}, "level": 30, "exp": 1, "resist": [50, 100, 49, 50, 100, 100, 100, 100, {grav}, {stop}, 50, 100, 100, 100, 100, 100]}}"#)
    };
    let worlds = |r: String| vec![r; 13].join(", ");
    let strength = (1..=100).map(|l| (6 + l).to_string()).collect::<Vec<_>>().join(", ");
    let hp = (0..13).map(|i| (33 + 3 * i).to_string()).collect::<Vec<_>>().join(", ");
    let cell = |ratio: f64, from: &str, r: String| format!(r#"{{"ratio": {ratio:.6}, "from": "{from}", "recom_hp": 50, "kh1_hp": 20, "record": {r}}}"#);
    // Ordinary: ratio 0.4 from the Soldier (Defence 8, cap 300); boss: ratio 2 from Guard Armor
    // with the final boss's Gravity 2 / Stop 0; world 4 row 0 has no cell.
    let orow = |w: usize| (0..13).map(|f| if w == 3 && f == 0 { "null".to_string() } else { cell(0.4, "CEnemySoldier", rec(20, 8, 300, 100, 100)) }).collect::<Vec<_>>().join(", ");
    let brow = (0..13).map(|_| cell(2.0, "CBossGuardarmor", rec(700, 8, 30, 2, 0))).collect::<Vec<_>>().join(", ");
    let ordinary = (0..13).map(|w| format!("[{}]", orow(w))).collect::<Vec<_>>().join(", ");
    let boss = vec![format!("[{brow}]"); 13].join(", ");
    format!(
        r#"{{"format": "kh1-recom-assets stats 5", "sora_strength": [{strength}], "recom_rows": 13, "enemies": [{{"class": "CEnemyShadow", "lock_radius": 30, "recom_id": 1, "recom_hp": [{hp}], "worlds": [{}]}}], "final_boss": {{"kh1_model": "xa_ew_3020.mdls", "gravity": 2, "stop": 0}}, "bosses": [{{"recom_id": 102, "class": "CBossGuardarmor", "name": "Guard Armor", "kh1_model": "xa_tw_3000.mdls", "home_world": 1, "pool": true, "assumed": false, "recom_hp": [{hp}], "worlds": [{}]}}], "proxies": {{"ordinary": [{ordinary}], "boss": [{boss}]}}}}"#,
        worlds(rec(12, 8, 300, 100, 100)),
        worlds(rec(700, 8, 30, 4, 100))
    )
}

#[test]
fn stats_v5_reads_the_boss_twins_and_proxy_tables() {
    let s = parse_stats(&stats_v5()).unwrap();
    assert_eq!(s.twin_recom_hp("CEnemyShadow", 0), Some(33));
    assert_eq!(s.twin_recom_hp("CEnemyShadow", 12), Some(69));
    assert_eq!(s.twin_recom_hp("CEnemySoldier", 0), None);
    assert_eq!((s.bosses.len(), s.bosses[0].id, s.bosses[0].worlds[0].hp), (1, 102, 700));
    let p = s.proxies.as_ref().unwrap();
    assert_eq!((p.ordinary.len(), p.ordinary[0].len(), p.boss.len()), (13, 13, 13));
    assert!(p.ordinary[3][0].is_none());
    // Older files (no proxies) still read: proxies off.
    let old = parse_stats(&stats_v5().replace(r#""proxies":"#, r#""unused":"#)).unwrap();
    assert!(old.proxies.is_none());
    // A table with a world missing is refused.
    let t = stats_v5();
    let at = t.find(r#""ordinary": ["#).unwrap() + r#""ordinary": ["#.len();
    let next = at + t[at..].find("], [").unwrap() + 3;
    let short = format!("{}{}", &t[..at], &t[next..]);
    assert!(parse_stats(&short).unwrap_err().contains("12 worlds"));
    let bad = stats_v5().replace(r#""ratio": 0.400000"#, r#""ratio": -1"#);
    assert!(parse_stats(&bad).is_err(), "a ratio must be positive");
}

#[test]
fn proxies_scale_recoms_hp_and_boss_twins_take_their_own_record() {
    let s = parse_stats(&stats_v5()).unwrap();
    // An ordinary Re:CoM-only enemy (Tornado Step, id 29) with 225 Re:CoM HP on 12F (floor 12).
    let p = s.proxy("CEnemyTornadostep", 29, 12, 12, 225).unwrap();
    assert_eq!((p.stats.hp, p.stats.defence, p.stats.max_damage, p.boss, p.own), (90, 8, 300, false, false));
    assert_eq!(p.stats.resist[8], 100);
    assert_eq!(p.from, "CEnemySoldier");
    // A Re:CoM-only boss (Larxene, id 117): × the boss ratio, Guard Armor's cap, KH1's
    // final-boss Gravity 2 % and Stop immunity (decision 34).
    let l = s.proxy("CBossLarxene", 117, 13, 7, 1000).unwrap();
    assert_eq!((l.stats.hp, l.stats.max_damage, l.stats.resist[8], l.stats.resist[9], l.boss, l.own), (2000, 30, 2, 0, true, false));
    // The boss twin itself: its own KH1 record (Gravity 4, Stop 100), whatever Re:CoM's HP.
    let g = s.proxy("CBossGuardarmor", 102, 1, 1, 350).unwrap();
    assert_eq!((g.stats.hp, g.stats.resist[8], g.stats.resist[9], g.own), (700, 4, 100, true));
    assert!((g.ratio - 2.0).abs() < 1e-9);
    // Another class with the boss's id (a boss part): the boss cell, not the twin's record.
    let part = s.proxy("CBossJafarLamp", 102, 1, 1, 350).unwrap();
    assert_eq!((part.stats.hp, part.own), (700, false));
    // Floor halls (world 0) scale as Castle Oblivion.
    assert_eq!(s.proxy("CBossRiku", 116, 0, 4, 400).unwrap().stats.hp, 800);
    // No proxy: 100 Acre Wood's minigame, no Re:CoM HP, Riku's floors, an empty cell, no tables.
    assert!(s.proxy("CEnemy100AcreBB_Bee", 38, 10, 100, 50).is_none());
    assert!(s.proxy("CEnemyTornadostep", 29, 1, 1, 0).is_none());
    assert!(s.proxy("CEnemyTornadostep", 29, 1, -3, 50).is_none());
    assert!(s.proxy("CEnemyTornadostep", 29, 4, 1, 50).is_none(), "world 4 floor 1 has no cell");
    assert!(s.proxy("CEnemyTornadostep", 29, 14, 1, 50).is_none());
    let old = parse_stats(&stats_v5().replace(r#""proxies":"#, r#""unused":"#)).unwrap();
    assert!(old.proxy("CEnemyTornadostep", 29, 1, 1, 50).is_none());
    assert_eq!(old.proxy("CBossGuardarmor", 102, 1, 1, 350).unwrap().stats.hp, 700, "boss twins need no table");
    // A proxy's hits take KH1's damage on its record: the Soldier's Defence 8, cap 300.
    let m = Moveset::kh1_defaults();
    let mut rng = kh1_core::Kh1Rand::new(1);
    assert_eq!(kh1_damage(&m, 13, 10, &WeaponData::kingdom_key(), &p.stats, &mut rng).unwrap().damage, 2);
    // Guard Armor's cap of 30 holds against a max-level Strength.
    for _ in 0..20 {
        assert!(kh1_damage(&m, 8, 51, &WeaponData::kingdom_key(), &l.stats, &mut rng).unwrap().damage <= 30);
    }
}

/// Re:CoM's per-frame order (Battle exe `0x1401F9840`): the damage check (immune while
/// `+0x7BC > 0`), the action step, then `+0x7BC -= dt`. The hold leaves KH1's 30 immune frames
/// after the last reaction frame, then the 31st frame takes hits.
#[test]
fn post_hit_hold_leaves_thirty_frames() {
    for dt in [50, 100] {
        let frames_per_tick = dt / RECOM_UNITS_PER_FRAME;
        let mut timer = 0;
        // Three reaction frames.
        for _ in 0..3 {
            timer = post_hit_hold(timer, dt);
            timer = (timer - dt).max(0);
        }
        let mut immune = 0;
        while timer > 0 {
            immune += frames_per_tick;
            timer = (timer - dt).max(0);
        }
        assert_eq!(immune, KH1_HIT_UNTOUCHABLE_FRAMES);
    }
    // Re:CoM's revive timer is never shortened.
    assert_eq!(post_hit_hold(3000, 50), 3000);
}

#[test]
fn stats_v6_reads_the_twins_drops() {
    // The asset step's shape (`kh1-recom-assets` `stats::drops_json`): 13 worlds of three pairs;
    // Destiny Islands (12) differs, as the Shadow's does.
    let world = |c: &str| format!("[[{c}, 1], [0, 0], [0.03, 233]]");
    let drops = (1..=13).map(|w| world(if w == 12 { "0.05" } else { "0.01" })).collect::<Vec<_>>().join(", ");
    let t = stats_v5().replace("stats 5", "stats 6").replace(r#""recom_id": 1, "#, &format!(r#""recom_id": 1, "drops": [{drops}], "#));
    let s = parse_stats(&t).unwrap();
    let tt = s.drops("CEnemyShadow", 1).unwrap();
    assert_eq!(tt.len(), 3);
    assert_eq!((tt[0].chance, tt[0].item, tt[2].chance, tt[2].item), (0.01, 1, 0.03, 233));
    assert_eq!(s.drops("CEnemyShadow", 12).unwrap()[0].chance, 0.05);
    // No twin, no world, or an older file: no drops.
    assert!(s.drops("CEnemySoldier", 1).is_none());
    assert!(s.drops("CEnemyShadow", 0).is_none() && s.drops("CEnemyShadow", 14).is_none());
    assert!(parse_stats(&stats_v5()).unwrap().drops("CEnemyShadow", 1).is_none());
    // Refused: 12 worlds, a chance above 1, a fractional item.
    let short = (1..=12).map(|_| world("0.01")).collect::<Vec<_>>().join(", ");
    assert!(parse_stats(&stats_v5().replace(r#""recom_id": 1, "#, &format!(r#""recom_id": 1, "drops": [{short}], "#))).unwrap_err().contains("12 worlds"));
    assert!(parse_stats(&t.replace("[0.05, 1]", "[1.5, 1]")).is_err());
    assert!(parse_stats(&t.replace("[0.05, 1]", "[0.05, 1.5]")).is_err());
}

/// M9 9a: the combo modifiers' rows: each its own clip (Re:CoM's bake or an appended row), KH1's
/// hit window one frame earlier, KH1's combo window, the Re:CoM kind of its record.
#[test]
fn modifier_rows_take_their_clips_and_kh1s_windows() {
    let m = Moveset::kh1_defaults();
    // The owner's manifest: Sliding Dash / Stun Impact / Zantetsuken on Re:CoM's rows 0x45 / 0x44
    // / 0x46, the rest appended after Guard and Counterattack.
    let clip = |r: u16| match r {
        208 => Some(0x45),
        216 => Some(0x44),
        218 => Some(0x46),
        209 => Some(0x49),
        210 => Some(0x4A),
        211 => Some(0x4B),
        214 => Some(0x4C),
        215 => Some(0x4D),
        217 => Some(0x4E),
        213 => Some(0x48),
        _ => None,
    };
    let t = kh1_table(&RECOM_ROWS_1_7, &m, &clip);
    // (row, clip, hit +4/+5, window +1/+3, Re:CoM kind)
    type Want = (usize, u8, (u8, u8), (u8, u8), u8);
    let want: [Want; 9] = [
        (0, 0x4C, (5, 27), (20, 44), 1),
        (3, 0x44, (33, 49), (0, 0), RECOM_STUN_KIND),
        (4, 0x4E, (31, 37), (0, 0), 1),
        (5, 0x46, (35, 49), (0, 0), 1),
        (6, 0x4A, (29, 37), (0, 0), 2),
        (7, 0x4D, (33, 45), (0, 0), 2),
        (9, 0x45, (25, 57), (28, 86), 1),
        (10, 0x4B, (17, 41), (38, 66), 1),
        (15, 0x49, (3, 53), (0, 0), 2),
    ];
    for (r, c, hit, win, kind) in want {
        let b = t[r];
        assert_eq!((b[0], (b[4], b[5]), (b[1], b[3]), b[0xB], b[0xF]), (c, hit, win, kind, FLAG_KH1), "row {r}");
        assert_eq!(&b[6..0xB], &[0; 5], "row {r}: no homing, no card column");
    }
    // Layouts: the finishers copy Re:CoM's finisher row (its `+2` = 3), the swings swing 1's.
    assert_eq!((t[3][2], t[9][2], t[15][2]), (RECOM_ROWS_1_7[3][2], RECOM_ROWS_1_7[0][2], RECOM_ROWS_1_7[6][2]));
    assert_eq!(modifier_rows_ready(&t), crate::abilities::MODIFIER_BITS);
    // Without a clip the row stays empty and the pickers don't get that modifier.
    let none = kh1_table(&RECOM_ROWS_1_7, &m, &|r| (r != 217).then(|| clip(r)).flatten());
    assert_eq!(none[4], [0u8; 16]);
    assert_eq!(modifier_rows_ready(&none), crate::abilities::MODIFIER_BITS & !(1 << crate::abilities::ix::GRAVITY_BREAK));
    // Root motion: KH1's request map has it for all but Hurricane Blast (an air row).
    for r in [208u16, 210, 211, 214, 215, 216, 217, 218] {
        assert!(swing_root(&m, r), "{r}");
    }
    assert!(!swing_root(&m, 209));
}

/// M9 9a: the modifiers' damage through KH1's formula: Gravity Break and Zantetsuken are % of the
/// current HP (× resist[8]), Ripple Drive reads Magic × 3, Stun Impact flinches (stun) for power 4.
#[test]
fn modifier_damage() {
    let m = Moveset::kh1_defaults();
    let w = WeaponData::kingdom_key();
    let mut rng = kh1_core::Kh1Rand::new(1);
    let t = twin(8);
    let sora = Sora { strength: 10, max_mp: 5 };
    // Gravity Break: (40 + Strength) % of the current HP (its stat source is Strength; crit chance
    // 200 × 0.05 × the Kingdom Key's 20 × 0.05 = 10: always a crit, bonus 0): 50 % of 50.
    let gb = kh1_damage_at(&m, 4, sora, &w, &t, 50, &mut rng).unwrap();
    assert_eq!((gb.damage, gb.critical), (25, true));
    // Zantetsuken: 250 % of the current HP: the rest of it.
    let zan = kh1_damage_at(&m, 5, sora, &w, &t, 37, &mut rng).unwrap();
    assert_eq!(zan.damage, 92);
    // A % hit is at least the floor (twin's floor < 2: 1).
    assert_eq!(kh1_damage_at(&m, 4, sora, &w, &t, 1, &mut rng).unwrap().damage, 1);
    // resist[8] scales it (a boss at 2 %: 50 % × 2 % of 1000 = 10).
    let mut boss = twin(8);
    boss.resist[8] = 2;
    assert_eq!(kh1_damage_at(&m, 4, sora, &w, &boss, 1000, &mut rng).unwrap().damage, 10);
    // Ripple Drive: power 10 + max MP × 3 (not Strength): curve[10 + 15 − 8].
    let rd1 = kh1_damage_at(&m, 7, sora, &w, &t, t.hp, &mut rng).unwrap().damage;
    let rd2 = kh1_damage_at(&m, 7, Sora { strength: 90, max_mp: 5 }, &w, &t, t.hp, &mut rng).unwrap().damage;
    assert_eq!(rd1, rd2, "Strength doesn't count");
    assert_eq!(rd1, kh1_core::DamageCurve::kh1_defaults().0[17] as i32);
    // Stun Impact: Strength + 4.
    assert_eq!(kh1_damage_at(&m, 3, sora, &w, &t, t.hp, &mut rng).unwrap().damage, kh1_core::DamageCurve::kh1_defaults().0[6] as i32);
}

/// M9 9a: Stun Impact's record sits on Sora's bone 8 (Re:CoM's `belt1`), not the blade.
#[test]
fn stun_impact_is_a_body_sphere() {
    let m = Moveset::kh1_defaults();
    let live = live_attack(&m, 3, &WeaponData::kingdom_key()).unwrap();
    assert_eq!((live.data.bone, live.radius), (8, 250.0), "no Keyblade radius on a body bone");
    assert_eq!(recom_body_bone(live.data.bone), Some(4));
    assert_eq!(recom_body_bone(KH1_WEAPON_BONE), None);
    // Ripple Drive: the blade's t = 0 (the hand), 200 + the Keyblade's 40.
    let rip = live_attack(&m, 7, &WeaponData::kingdom_key()).unwrap();
    assert_eq!((rip.data.bone, rip.radius, rip.blade_t(&WeaponData::kingdom_key())), (KH1_WEAPON_BONE, 240.0, (0.0, 0.0)));
}

/// Damage = Re:CoM (owner, 2026-11-01, follow-ups): Gravity Break and Zantetsuken take KH1's
/// percentage of the enemy's Re:CoM current HP (KH1 `0x1402BFA80` element 8: truncated, × the
/// record's resist[8], at least its floor); every other row: None (the card's column stands).
#[test]
fn percent_rows_on_recoms_hp() {
    let m = Moveset::kh1_defaults();
    let w = WeaponData::kingdom_key();
    let mut rng = kh1_core::Kh1Rand::new(1);
    let t = twin(8);
    let sora = Sora { strength: 10, max_mp: 5 };
    // Gravity Break: (40 + 10) % of a Re:CoM HP of 1203 = 601 (601.5 truncated).
    assert_eq!(kh1_percent_of_hp(&m, 4, sora, &w, Some(&t), 1203, &mut rng), Some(601));
    // Strength 51 (max level): 91 %.
    assert_eq!(kh1_percent_of_hp(&m, 4, Sora { strength: 51, max_mp: 5 }, &w, Some(&t), 1000, &mut rng), Some(910));
    // Zantetsuken: 250 % of it (no stat term): more than the HP left.
    assert_eq!(kh1_percent_of_hp(&m, 5, sora, &w, Some(&t), 1203, &mut rng), Some(3007));
    // A boss record's resist[8] (2 %) and floor (KH1's boss rule): 250 % × 2 % of 4000 = 200;
    // 50 % × 2 % of 50 = 0 → the floor 2.
    let mut boss = twin(8);
    boss.resist[8] = 2;
    boss.min_damage = 2;
    assert_eq!(kh1_percent_of_hp(&m, 5, sora, &w, Some(&boss), 4000, &mut rng), Some(200));
    assert_eq!(kh1_percent_of_hp(&m, 4, sora, &w, Some(&boss), 50, &mut rng), Some(2));
    // No KH1 record: 100 %, floor 1.
    assert_eq!(kh1_percent_of_hp(&m, 4, sora, &w, None, 1000, &mut rng), Some(500));
    assert_eq!(kh1_percent_of_hp(&m, 4, sora, &w, None, 1, &mut rng), Some(1));
    // Not a % row.
    for r in [13, 11, 8, 3, 7, 0x3E] {
        assert_eq!(kh1_percent_of_hp(&m, r, sora, &w, Some(&t), 1000, &mut rng), None);
    }
}

#[test]
fn riku_tables_take_his_own_hit_windows() {
    let m = Moveset::kh1_defaults();
    let t = kh1_table(&RECOM_ROWS_1_7, &m, &|_| None);
    let r = riku_table(&t, &RIKU_ROWS_1_7);
    let d = riku_table(&t, &DARK_ROWS_1_7);
    // Riku's rows 0x97..0x9D (Checked (exe)): 12–26, 12–32, 20–38, 18–48, 8–26, 4–16, 10–38.
    let win = |t: &[[u8; 16]], row: usize| (t[row][4], t[row][5]);
    assert_eq!([13, 11, 14, 8, 17, 18, 16].map(|row| win(&r, row)), [(12, 26), (12, 32), (20, 38), (18, 48), (8, 26), (4, 16), (10, 38)]);
    // Dark Riku's 0x9E..0xA4: 4–22, 4–42, 8–26, 18–40, 8–26, 8–30, 4–50.
    assert_eq!([13, 11, 14, 8, 17, 18, 16].map(|row| win(&d, row)), [(4, 22), (4, 42), (8, 26), (18, 40), (8, 26), (8, 30), (4, 50)]);
    // The jump-up / hop rows take their clip's row (ids 5 / 6).
    assert_eq!((win(&r, 1), win(&r, 2)), ((8, 26), (4, 16)));
    // Everything else is the KH1 row's: clip, combo window, kind, flag.
    // His homing: row 13 = 0x97's +6..+9 (8, 30, 20, 8); the KH1 row had none.
    assert_eq!((&t[13][6..10], &r[13][6..10]), (&[0u8, 0, 0, 0][..], &[8u8, 30, 20, 8][..]));
    assert_eq!(&d[8][6..10], &[8u8, 26, 20, 8][..]);
    for (i, (a, b)) in t.iter().zip(r.iter()).enumerate() {
        assert_eq!((a[0..4].to_vec(), a[10..].to_vec()), (b[0..4].to_vec(), b[10..].to_vec()), "row {i}");
        if !ROW_MAP.iter().any(|&(row, _)| row == i) {
            assert_eq!(a, b, "row {i} not a swing row");
        }
    }
    // The constants are Re:CoM's Riku rows: Sora's action rows, Riku's own effects.
    for k in 0..7 {
        assert_eq!((RIKU_ROWS_1_7[k][0], DARK_ROWS_1_7[k][0]), (RECOM_ROWS_1_7[k][0], RECOM_ROWS_1_7[k][0]));
    }
}

