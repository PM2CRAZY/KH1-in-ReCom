use super::*;

#[test]
fn kh1_rows_keep_recoms_clip_and_take_kh1s_windows() {
    let m = Moveset::kh1_defaults();
    let t = kh1_table(&RECOM_ROWS_1_7, &m);
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
    assert_eq!(recom_id(3), None);
    assert!(swing_root(&m, 200) && swing_root(&m, 203));
    assert!(!swing_root(&m, 204) && !swing_root(&m, 206));
    assert!(is_swing_request(206) && !is_swing_request(212));
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

#[test]
fn kh1_damage_uses_the_twins_defence_curve() {
    let m = Moveset::kh1_defaults();
    let mut rng = kh1_core::Kh1Rand::new(1);
    // A twin at defence level 3 (Defence 8): Strength 10 swing → curve[2] = 2.
    let twin = EnemyStats { name: "Shadow".into(), hp: 12, strength_level: 3, defence_level: 3, min_damage: 0, max_damage: 0, level: 1 };
    assert_eq!(kh1_damage(&m, 13, 10, &twin, &mut rng).unwrap().damage, 2);
    // The finisher adds 4 power → curve[6] = 6 (or a critical 10 + 4 → curve[10] = 12).
    for _ in 0..20 {
        let d = kh1_damage(&m, 8, 10, &twin, &mut rng).unwrap();
        assert_eq!(d.damage, if d.critical { 12 } else { 6 });
    }
    assert_eq!(ENEMY_STR_DEF.len(), 21);
    assert_eq!(ENEMY_STR_DEF[3], (8, 8));
}

#[test]
fn enemies_json_reads_the_documented_format() {
    let t = r#"{"enemies": [{"recom_id": 10010, "name": "Shadow", "hp": 12, "strength_level": 3, "defence_level": 3, "level": 1}]}"#;
    let v = parse_enemies(t).unwrap();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].0, 10010);
    assert_eq!(v[0].1.hp, 12);
    assert_eq!(v[0].1.max_damage, 0);
    assert!(parse_enemies("{}").is_err());
    assert!(parse_enemies(r#"{"enemies": [{"name": "x"}]}"#).is_err());
}

#[test]
fn battle_case_stub_calls_the_swing_exe_then_the_default() {
    let b = battle_case::stub();
    assert_eq!(&b[..4], &[0x48, 0x8B, 0xCF, 0xE8]);
    let call = i32::from_le_bytes(b[4..8].try_into().unwrap()) as i64;
    assert_eq!(0x0001_401F_A6C1_i64 + 8 + call, 0x0001_401E_AEB0);
    assert_eq!(b[8], 0xE9);
    let jmp = i32::from_le_bytes(b[9..13].try_into().unwrap()) as i64;
    assert_eq!(0x0001_401F_A6C1_i64 + 13 + jmp, 0x0001_401F_9CC9);
    assert_eq!(battle_case::entry(), [0xC1, 0xA6, 0x1F, 0x00]);
}
