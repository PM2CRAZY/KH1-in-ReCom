//! KH1's Fire numbers in Re:CoM's row and shot formats; the spell damage. The cast's frames are
//! tested on the movement tests' fake Re:CoM (`movement/tests.rs`, "M7 stage 7.4").

use super::*;

#[test]
fn fire_rows_keep_recoms_clip_and_effect_set_and_take_kh1s_ball() {
    let t = kh1_magic_table(&recom_rows());
    assert_eq!(t.len(), 21);
    for lv in 1..=3u8 {
        let i = table_index(spell::FIRE, lv).unwrap();
        let r = &t[i];
        let base = &RECOM_FIRE_ROWS[lv as usize - 1];
        // Re:CoM's clip (0x37), blend and Fire / Fira / Firaga effect set (3 / 4 / 5).
        assert_eq!(&r[..3], &base[..3]);
        assert_eq!(r[2], 2 + lv);
        // KH1's ball: 50 cm, 40 cm a frame, 3° a frame, 30 frames, a flinch, no range or gate.
        assert_eq!((r[4], r[8], r[9], r[0xB], r[0xC], r[0xD], r[0xE]), (50, 255, 40, 1, 255, 3, 15));
        assert_eq!(r[0xF] & FLAG_KH1, FLAG_KH1);
        assert_eq!(recom_base_row(spell::FIRE, lv), Some([0x22, 0x29, 0x2A][lv as usize - 1]));
    }
    // Every spell has its rows (7.10).
    assert!(t.iter().all(|r| r[0] != 0));
    assert_eq!(table_index(spell::AERO, 3), Some(20));
    assert_eq!(table_index(spell::FIRE, 0), None);
    assert!(castable(spell::FIRE) && castable(spell::BLIZZARD) && castable(spell::THUNDER) && castable(spell::CURE) && castable(spell::GRAVITY) && castable(spell::STOP) && castable(spell::AERO));
}

#[test]
fn shot_numbers_are_kh1s_in_recoms_units() {
    let row = fire_row_bytes(&RECOM_FIRE_ROWS[0]);
    let n = shot_numbers(&row);
    // 40 cm a frame = 0.40 m per 60 Hz frame (Re:CoM's mover takes `+0x80` per frame).
    assert!((n.speed - 0.40).abs() < 1e-6);
    // 30 frames × 50 ticks.
    assert_eq!(n.life, 1500);
    assert_eq!(n.fade, 1000);
    assert!((n.radius - 0.5).abs() < 1e-6);
    assert_eq!((n.turn, n.gate, n.range), (3.0, 255.0, 255.0));
    // Re:CoM's Fire flags without the 0.5 m homing delay, with the life ending the ball.
    assert_eq!(n.flags, 0x8104);
    assert_eq!(n.flags & 0x1000, 0);
    assert_eq!(n.flags & 0x100, 0x100, "a hit or wall still ends it, as Re:CoM's Fire");
    // The ball covers 12 m in its life (KH1: 40 × 30 cm, the command's range 1200).
    assert!((n.speed * n.life as f32 / 50.0 - 12.0).abs() < 1e-4);
}

#[test]
fn fire_attack_data_is_kh1s_per_level() {
    for (lv, p) in [(1, 20), (2, 28), (3, 36)] {
        let a = fire_attack(lv);
        assert_eq!((a.power, a.stat_source, a.element, a.kind, a.mp_charge, a.radius), (p, 1, 2, 3, 0, 50.0));
    }
    assert_eq!(fire::COST, 30);
    assert_eq!(fire::LIFE * fire::SPEED, 1200.0);
}

#[test]
fn spell_damage_is_power_plus_max_mp_through_the_curve_and_fire_resistance() {
    let mut rng = Kh1Rand::new(1);
    let mut def = DefenderStats { defence: 5, ..Default::default() };
    // Fire (20) + max MP 3 = 23 − Defence 5 = curve[18] = 48.
    assert_eq!(spell_damage(&fire_attack(1), 3, &def, &mut rng), 48);
    // Firaga (36) + 8 − 5 = 39: the curve's cap of 300.
    assert_eq!(spell_damage(&fire_attack(3), 8, &def, &mut rng), 300);
    // KH1's fire resistance (`resist[2]`), and the record's cap.
    def.resist[2] = 0.5;
    assert_eq!(spell_damage(&fire_attack(1), 3, &def, &mut rng), 24);
    def.resist[2] = 1.0;
    def.max_damage = 20;
    assert_eq!(spell_damage(&fire_attack(1), 3, &def, &mut rng), 20);
    // Physical resistance doesn't touch Fire.
    def.max_damage = 0;
    def.resist[1] = 0.0;
    assert_eq!(spell_damage(&fire_attack(1), 3, &def, &mut rng), 48);
}

#[test]
fn cast_motions_are_kh1s_frame_counts_and_windows() {
    let mut m = MotionSet::default();
    add_motions(&mut m);
    let len = |r| m.info(r).unwrap().length;
    assert_eq!((len(0x36), len(0x3D), len(0x83), len(0x8A)), (28.0, 60.0, 28.0, 52.0));
    let free = |r| m.info(r).unwrap().first(FlagKind::Free);
    assert_eq!((free(0x36), free(0x3D), free(0x8A)), (None, Some(46.0), Some(46.0)));
    // KH1's root helper (entry 57): back 1.5 cm a frame to −15 cm at frame 10, then 0.1 cm a frame
    // forward to −10 cm at its end (entry 82: back to −42.41 cm at 10, then still). Checked (files).
    let z = |r, t| m.info(r).unwrap().root_at(t).z;
    for (t, want) in [(0.0, 0.0), (5.0, -7.5), (10.0, -15.0), (20.0, -14.0), (46.0, -11.4), (60.0, -10.0)] {
        assert!((z(0x3D, t) - want).abs() < 1e-4, "0x3D at {t}: {}", z(0x3D, t));
    }
    for (t, want) in [(5.0, -21.2048), (10.0, -42.4096), (52.0, -42.4096)] {
        assert!((z(0x8A, t) - want).abs() < 1e-3, "0x8A at {t}: {}", z(0x8A, t));
    }
    assert!(is_cast_request(0x36) && is_cast_request(0x8A) && !is_cast_request(0x3E) && !is_cast_request(200));
}

/// Decision 38: the ground release's travel in game is KH1's curve, not Re:CoM's clip's 40.5 cm.
#[test]
fn fire_release_root_is_kh1s_curve_on_recoms_line() {
    use kh1_core::Vec3;
    use release_root::{kh1_z, travel, Clock};
    // The record (the tests' fake Re:CoM) and the wrapper share KH1's keys.
    let mut m = MotionSet::default();
    add_motions(&mut m);
    for t in [0.0, 3.0, 10.0, 33.5, 60.0] {
        assert!((m.info(0x3D).unwrap().root_at(t).z - kh1_z(t)).abs() < 1e-4, "{t}");
    }
    assert_eq!((kh1_z(-1.0), kh1_z(10.0), kh1_z(60.0), kh1_z(70.0)), (0.0, -15.0, -10.0, -10.0));
    // Sora faces +x in Re:CoM (yaw π/2). Re:CoM's clip steps him back 4.047 cm an update over its
    // first KH1 frames (−x); KH1's step is 1.5 cm, on the same line.
    let fwd = Vec3::from_yaw(std::f32::consts::FRAC_PI_2);
    let back = travel(Vec3::new(-0.04047, 0.0, 0.0), fwd, 2.0, 3.0);
    assert!((back.x + 0.015).abs() < 1e-6 && back.y == 0.0 && back.z.abs() < 1e-6, "{back:?}");
    // After frame 10 both drift forward 0.1 cm a frame (KH1's travel, Re:CoM's line).
    let fwd_step = travel(Vec3::new(0.001, 0.0, 0.0), fwd, 20.0, 21.0);
    assert!((fwd_step.x - 0.001).abs() < 1e-6, "{fwd_step:?}");
    // Re:CoM's clip holds still from its frame 29 (KH1 58); KH1 still drifts 0.2 cm to 60: along the
    // facing.
    let tail = travel(Vec3::ZERO, fwd, 58.0, 60.0);
    assert!((tail.x - 0.002).abs() < 1e-6 && tail.z.abs() < 1e-6, "{tail:?}");
    // The update that spans the key at 10: −1.5 back, +0.1 forward.
    let across = travel(Vec3::new(-0.02, 0.0, 0.0), fwd, 9.0, 11.0);
    assert!((across.x + 0.014).abs() < 1e-6, "{across:?}");
    // The whole release is KH1's −10 cm (Re:CoM's clip: −35.7 cm), however the updates fall.
    let mut c = Clock::new();
    let mut total = 0.0;
    let mut t = 0.0;
    while t < 60.0 {
        t += 1.0;
        let (a, b) = c.span(1, t);
        total += travel(Vec3::new(-0.01, 0.0, 0.0), fwd, a, b).x;
    }
    assert!((total + 0.10).abs() < 1e-5, "{total}");
    // A clip Re:CoM starts one step on still hands out frame 0 → 1; a restarted clip (the Fire
    // chain) starts again from 0; another controller too; past the end it holds at 60.
    let mut c = Clock::new();
    assert_eq!(c.span(1, 1.0), (0.0, 1.0));
    assert_eq!(c.span(1, 2.0), (1.0, 2.0));
    assert_eq!(c.span(1, 15.0), (2.0, 15.0));
    assert_eq!(c.span(1, 0.0), (0.0, 0.0));
    assert_eq!(c.span(2, 1.0), (0.0, 1.0));
    assert_eq!(c.span(2, 75.0), (1.0, 60.0));
    c.reset();
    assert_eq!(c.span(2, 3.0), (0.0, 3.0));
}

#[test]
fn trace_tokens() {
    assert_eq!(trace_token(None), "cast -");
    let c = Cast { start: CastStart { spell: 0, level: 2, air: true, fast: true }, wait: 3.0, age: 5.0, released: false, cancelled: false, shot: None, fresh: false };
    assert_eq!(trace_token(Some(&c)), "cast fire2 f5 fast w3 air");
    assert_eq!(CastEvent::Released { spell: 0, level: 1, frame: 28.0, shot: Some(7) }.token(), "release(fire1 f28 shot 7)");
}

#[test]
fn field_case_stub_calls_exe_magic_on_rbx_and_goes_on_at_the_default() {
    let b = field_case::stub();
    assert_eq!(&b[..4], &[0x48, 0x8B, 0xCB, 0xE8]);
    let call = i32::from_le_bytes(b[4..8].try_into().unwrap()) as i64;
    assert_eq!(field_case::CAVE_RVA as i64 + 8 + call, 0x0021_7EE0);
    assert_eq!(b[8], 0xE9);
    let jmp = i32::from_le_bytes(b[9..13].try_into().unwrap()) as i64;
    assert_eq!(field_case::CAVE_RVA as i64 + 13 + jmp, 0x001F_A599);
    assert_eq!(field_case::entry(), [0xE1, 0xB0, 0x1F, 0x00]);
}

#[test]
fn fire_retarget_is_kh1s_pick() {
    use retarget::*;
    // 500 cm = 5 m; the weight is the script's 2.0.
    assert_eq!((RANGE_M, WEIGHT), (5.0, 2.0));
    let ball = [0.0, 1.0, 0.0];
    let fwd = [0.0, 0.0, 1.0];
    // Straight ahead: score = distance. Straight behind: × (1 + 2 × 2) = 5. Beside: × 3.
    assert_eq!(score(ball, fwd, [0.0, 1.0, 2.0]), Some((2.0, 2.0)));
    assert_eq!(score(ball, fwd, [0.0, 1.0, -2.0]), Some((2.0, 10.0)));
    assert_eq!(score(ball, fwd, [2.0, 1.0, 0.0]), Some((2.0, 6.0)));
    // The range is strict and 3D (KH1's `d < 500`, `0x1402A9240`).
    assert_eq!(score(ball, fwd, [0.0, 1.0, 5.0]), None);
    assert!(score(ball, fwd, [0.0, 4.0, 4.0]).is_none());
    assert!(score(ball, fwd, [0.0, 1.0, 4.99]).is_some());
    // On the ball's own point: distance 0, cosine 0 (`0x1402A8C70`), score 0.
    assert_eq!(score(ball, fwd, ball), Some((0.0, 0.0)));
    // A nearer enemy behind loses to a farther one ahead: 1.5 m behind scores 7.5, 4 m ahead 4.
    let c = [(1, [0.0, 1.0, -1.5]), (2, [0.0, 1.0, 5.0]), (3, [0.0, 1.0, 4.0])];
    assert_eq!(pick(ball, fwd, c), Some((3, 4.0)));
    // Nothing in range: no pick (the ball flies straight this frame).
    assert_eq!(pick(ball, fwd, [(1, [0.0, 1.0, 6.0])]), None);
    // Ties keep the first (only a smaller score replaces it).
    assert_eq!(pick(ball, fwd, [(7, [1.0, 1.0, 0.0]), (8, [-1.0, 1.0, 0.0])]), Some((7, 1.0)));
    // The pick follows the ball's direction (Re:CoM's aim angles, `guard::forward`).
    let left = crate::guard::forward(0.0, -std::f32::consts::FRAC_PI_2);
    assert_eq!(pick(ball, left, [(1, [-3.0, 1.0, 0.0]), (2, [0.0, 1.0, 2.0])]).map(|p| p.0), Some(1));
    // The cosine is clamped and the KH1 → Re:CoM axis flip leaves it unchanged.
    assert_eq!(cos([1.0, 0.0, 0.0], [2.0, 0.0, 0.0]), 1.0);
    let (a, b) = ([1.0, 2.0, 3.0], [-2.0, 0.5, 1.0]);
    assert!((cos(a, b) - cos([a[0], -a[1], -a[2]], [b[0], -b[1], -b[2]])).abs() < 1e-6);
}

#[test]
fn blizzard_rows_and_numbers_are_kh1s() {
    let t = kh1_magic_table(&recom_rows());
    for lv in 1..=3u8 {
        let r = &t[table_index(spell::BLIZZARD, lv).unwrap()];
        let base = &RECOM_BLIZZARD_ROWS[lv as usize - 1];
        // Re:CoM's clip 0x3A and its Blizzard / Blizzara / Blizzaga sets 6 / 8 / 10.
        assert_eq!(&r[..3], &base[..3]);
        assert_eq!(r[2], [6, 8, 10][lv as usize - 1]);
        assert_eq!(r[5], base[5]);
        // KH1: radius 70 / 90 / 120 cm, 20 / 30 / 40 cm a frame, 26 frames (13 × 100 ticks).
        let i = lv as usize - 1;
        assert_eq!((r[4], r[9], r[0xB], r[0xC], r[0xD], r[0xE]), ([70, 90, 120][i], [20, 30, 40][i], 1, 255, 0, 13));
        assert_eq!(r[0xF] & FLAG_KH1, FLAG_KH1);
        assert_eq!(recom_base_row(spell::BLIZZARD, lv), Some([0x23, 0x2B, 0x2C][i]));
        let a = blizzard_attack(lv);
        assert_eq!((a.power, a.stat_source, a.element, a.kind, a.mp_charge, a.radius), ([22, 27, 34][i], 1, 3, 3, 0, [70.0, 90.0, 120.0][i]));
    }
    assert_eq!((cost(spell::FIRE), cost(spell::BLIZZARD), cost(spell::THUNDER)), (30, 30, 100));
    assert_eq!(release_wait(spell::BLIZZARD), 28.0);
    assert_eq!(KH1_BLIZZARD_FLAGS & 0x104, 0x104, "a hit, a wall or the life ends a shard");
    assert_eq!(KH1_BLIZZARD_FLAGS & 0x8000_1000, 0, "no homing");
    assert_eq!((cast_req::first(spell::BLIZZARD, false), cast_req::first(spell::BLIZZARD, true)), (0x37, 0x84));
    assert_eq!((cast_req::release(spell::BLIZZARD, false), cast_req::release(spell::FIRE, true)), (0x37, 0x8A));
    assert!(is_cast_request(0x37) && is_cast_request(0x84) && !is_cast_request(0x3E));
    assert_eq!(spell_attack(spell::BLIZZARD, 2).map(|a| a.power), Some(27));
}

#[test]
fn blizzard_shards_are_kh1s_cone() {
    for yaw in [0.0f32, 1.0, -2.5] {
        let s = blizzard_shards(yaw);
        let f = [yaw.sin(), 0.0, yaw.cos()];
        let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        // The straight one last, from the Keyblade point.
        assert_eq!(s[6].0, [0.0; 3]);
        assert!((dot(s[6].1, f) - 1.0).abs() < 1e-6);
        let mut ups = 0;
        for (off, dir) in &s[..6] {
            // 22.5° off the facing (the script's 0.924), a unit vector (0.383² + 0.924² ≈ 1).
            assert!((dot(*dir, f) - 0.924).abs() < 1e-4, "{dir:?}");
            assert!((dot(*dir, *dir) - 1.0).abs() < 1e-3);
            // Starting 19.15 cm off the axis, square to it, on the side the shard flies to.
            assert!(dot(*off, f).abs() < 1e-6);
            assert!((dot(*off, *off).sqrt() - 0.1915).abs() < 1e-4);
            assert!(dot(*off, *dir) > 0.0);
            if dir[1] > 0.01 {
                ups += 1;
            }
        }
        // Every 60° about the axis: two level, two up, two down.
        assert_eq!(ups, 2);
        assert_eq!(s[..6].iter().filter(|x| x.1[1].abs() < 1e-6).count(), 2);
        for i in 0..6 {
            let (a, b) = (s[i].1, s[(i + 1) % 6].1);
            let (ra, rb) = ([a[0] - 0.924 * f[0], a[1], a[2] - 0.924 * f[2]], [b[0] - 0.924 * f[0], b[1], b[2] - 0.924 * f[2]]);
            assert!((cos_between(ra, rb) - 0.5).abs() < 1e-3, "60° apart");
        }
    }
    fn cos_between(a: [f32; 3], b: [f32; 3]) -> f32 {
        retarget::cos(a, b)
    }
}

#[test]
fn blizzard_glide_is_kh1s() {
    // A level shard (dy 0) with the floor within 50 cm below sits 50 cm above it; the ray is 50 cm.
    assert!((blizzard_ray(0.0) - 0.5).abs() < 1e-6);
    assert!((blizzard_glide_y(1.0, 0.0, Some(0.62)) - 1.12).abs() < 1e-6);
    // No floor within reach: it keeps its own slope (a sinking shard 15 cm a frame).
    assert!((blizzard_glide_y(1.0, -0.15, None) - 0.85).abs() < 1e-6);
    // A sinking shard reaches further (KH1: 50 + its fall this frame), a rising one less.
    assert!((blizzard_ray(-0.15) - 0.65).abs() < 1e-6);
    assert!((blizzard_ray(0.15) - 0.35).abs() < 1e-6);
}

#[test]
fn thunder_rows_and_numbers_are_kh1s() {
    let t = kh1_magic_table(&recom_rows());
    for lv in 1..=3u8 {
        let i = lv as usize - 1;
        let r = &t[table_index(spell::THUNDER, lv).unwrap()];
        let base = &RECOM_THUNDER_ROWS[i];
        // Re:CoM's clip 0x39, sets 12 / 13 / 14, its bolt's speed, radius and range.
        assert_eq!((&r[..3], r[4], r[9], r[0xC]), (&base[..3], base[4], base[9], base[0xC]));
        assert_eq!(r[2], 12 + lv - 1);
        assert_eq!((r[3], r[0xB], r[0xF] & FLAG_KH1), (0, 1, FLAG_KH1));
        assert_eq!(recom_base_row(spell::THUNDER, lv), Some([0x24, 0x2E, 0x2F][i]));
        let a = thunder_attack(lv);
        assert_eq!((a.power, a.stat_source, a.element, a.kind, a.mp_charge), ([16, 20, 26][i], 3, 4, 3, 0));
    }
    assert_eq!((cost(spell::THUNDER), release_wait(spell::THUNDER)), (100, 30.0));
    assert_eq!((cast_req::first(spell::THUNDER, false), cast_req::first(spell::THUNDER, true)), (0x38, 0x85));
    assert!(is_cast_request(0x38) && is_cast_request(0x85));
    // Thunder + max MP × 2: 16 + 6 = 22 − Defence 5 = curve[17]; × KH1's thunder resistance.
    let mut rng = Kh1Rand::new(1);
    let mut def = DefenderStats { defence: 5, ..Default::default() };
    let full = spell_damage(&thunder_attack(1), 3, &def, &mut rng);
    let fire = spell_damage(&fire_attack(1), 3, &def, &mut rng);
    assert!(full < fire, "Thunder 16 + 2 × 3 = 22 < Fire 20 + 3 = 23");
    def.resist[4] = 0.5;
    assert_eq!(spell_damage(&thunder_attack(1), 3, &def, &mut rng), full / 2);
    def.resist[2] = 0.0;
    assert_eq!(spell_damage(&thunder_attack(1), 3, &def, &mut rng), full / 2, "fire resistance doesn't touch Thunder");
}

#[test]
fn thunder_area_and_order_are_kh1s() {
    let c = [0.0, 0.0, 0.0];
    // The cylinder: 6 m around, ±5 m; a body counts when any of it is inside.
    let s = |x: f32, y: f32, r: f32| BodyShape::Sphere { c: [x, y, 0.0], r };
    assert!(body_in_area(c, &s(5.9, 0.0, 0.0)));
    assert!(!body_in_area(c, &s(6.1, 0.0, 0.0)));
    assert!(body_in_area(c, &s(6.4, 0.0, 0.5)), "its edge inside");
    assert!(body_in_area(c, &s(0.0, 4.9, 0.0)) && !body_in_area(c, &s(0.0, 5.2, 0.1)) && body_in_area(c, &s(0.0, -5.3, 0.4)));
    // The rim: 0.3 m out and 0.3 m up from the edge is 0.42 m away.
    assert!(!body_in_area(c, &s(6.3, 5.3, 0.4)) && body_in_area(c, &s(6.3, 5.3, 0.45)));
    // A column crossing the area's top from far above: its lowest point counts.
    let col = BodyShape::Column { a: [3.0, 9.0, 0.0], b: [3.0, 4.5, 0.0], r: 0.1 };
    assert!(body_in_area(c, &col));
    let col = BodyShape::Column { a: [7.0, 0.0, -3.0], b: [7.0, 0.0, 3.0], r: 0.95 };
    assert!(!body_in_area(c, &col));
    let col = BodyShape::Column { a: [7.0, 0.0, -3.0], b: [7.0, 0.0, 3.0], r: 1.05 };
    assert!(body_in_area(c, &col));
    // Nearest first from the target, the first 30 only; ties keep the earlier.
    let e = |id: u32, x: f32| (id, [x, 0.0, 0.0]);
    assert_eq!(thunder_order(c, [e(1, 5.0), e(2, -1.0), e(3, 3.0), e(4, 1.0)]), vec![2, 4, 3, 1]);
    let many: Vec<(u32, [f32; 3])> = (0..40).map(|i| (i, [40.0 - i as f32, 0.0, 0.0])).collect();
    let o = thunder_order(c, many);
    assert_eq!(o.len(), 30);
    assert_eq!((o[0], o[29]), (29, 0));
}

#[test]
fn thunder_strikes_one_every_8_frames_nearest_first_once_each() {
    let mut s = Strikes::new(vec![10, 11, 12, 13]);
    let outside = 11;
    let mut at = Vec::new();
    for frame in 0..60 {
        if let Some(id) = s.run(1.0, |id| id != outside) {
            at.push((frame, id));
        }
    }
    // The release's run strikes the nearest; one outside the area is passed over at once.
    assert_eq!(at, vec![(0, 10), (8, 12), (16, 13)]);
    assert!(s.done());
    assert_eq!(s.struck, 3);
    // Nothing inside: done in the first run, no bolt.
    let mut s = Strikes::new(vec![1, 2]);
    assert_eq!(s.run(1.0, |_| false), None);
    assert!(s.done());
}

#[test]
fn cure_rows_and_numbers_are_kh1s_cast_with_recoms_heal() {
    let t = kh1_magic_table(&recom_rows());
    for lv in 1..=3u8 {
        let i = lv as usize - 1;
        let r = &t[table_index(spell::CURE, lv).unwrap()];
        let base = &RECOM_CURE_ROWS[i];
        // Re:CoM's clip 0x39 (Thunder's p001_064) and its Cure / Cura / Curaga sets 15 / 16 / 17.
        assert_eq!(&r[..3], &base[..3]);
        assert_eq!(r[2], 15 + lv - 1);
        assert_eq!((r[3], r[0xF] & FLAG_KH1), (0, FLAG_KH1));
        assert_eq!(recom_base_row(spell::CURE, lv), Some([0x25, 0x36, 0x37][i]));
        // Re:CoM's heal (decision 34): 500 / 1500 / 3000 ÷ 10.
        assert_eq!(cure::heal(lv), [50, 150, 300][i]);
    }
    // KH1: one point of MP, the release 36 frames after the command, no damage record.
    assert_eq!((cost(spell::CURE), release_wait(spell::CURE)), (100, 36.0));
    assert_eq!(spell_attack(spell::CURE, 1), None);
    assert_eq!((cast_req::first(spell::CURE, false), cast_req::first(spell::CURE, true)), (0x39, 0x86));
    assert_eq!((cast_req::release(spell::CURE, false), cast_req::release(spell::CURE, true)), (0x39, 0x86));
    assert!(is_cast_request(0x39) && is_cast_request(0x86));
    let mut m = MotionSet::default();
    add_motions(&mut m);
    // Entries 56 / 81: 68 frames, no free key (locked to the end).
    for r in [0x39, 0x86] {
        assert_eq!(m.info(r).unwrap().length, 68.0);
        assert_eq!(m.info(r).unwrap().first(FlagKind::Free), None);
    }
    // The thread heals only a live target (Sora with HP left, not dying).
    assert!(cure::target_ok(1, false) && !cure::target_ok(0, false) && !cure::target_ok(40, true));
}

#[test]
fn gravity_rows_numbers_and_fall_are_kh1s() {
    let t = kh1_magic_table(&recom_rows());
    for lv in 1..=3u8 {
        let i = lv as usize - 1;
        let r = &t[table_index(spell::GRAVITY, lv).unwrap()];
        let base = &RECOM_GRAVITY_ROWS[i];
        // Re:CoM's clip 0x3A (Blizzard's p001_065) and its Gravity sets 18 / 19 / 20.
        assert_eq!((&r[..3], r[5], r[0xD]), (&base[..3], base[5], base[0xD]));
        assert_eq!(r[2], 18 + lv - 1);
        // Re:CoM's flinch, not its kind 0xD.
        assert_eq!((r[3], r[0xB], r[0xF] & FLAG_KH1), (0, 1, FLAG_KH1));
        assert_eq!(recom_base_row(spell::GRAVITY, lv), Some([0x26, 0x38, 0x39][i]));
        let a = gravity_attack(lv);
        assert_eq!((a.power, a.stat_source, a.element, a.kind), ([40, 55, 70][i], 5, 8, 8));
    }
    assert_eq!((cost(spell::GRAVITY), release_wait(spell::GRAVITY)), (100, 28.0));
    assert_eq!((cast_req::first(spell::GRAVITY, false), cast_req::first(spell::GRAVITY, true)), (0x3A, 0x87));
    let mut m = MotionSet::default();
    add_motions(&mut m);
    assert_eq!((m.info(0x3A).unwrap().length, m.info(0x87).unwrap().length), (86.0, 78.0));
    assert_eq!((m.info(0x3A).unwrap().first(FlagKind::Free), m.info(0x87).unwrap().first(FlagKind::Free)), (Some(68.0), Some(68.0)));
    // The impact radius 170 × S cm and the scale's growth.
    assert_eq!([1u8, 2, 3].map(gravity::impact_radius_m), [1.7, 2.55, 2.72]);
    assert!((gravity::scale_at(2, 35.0) - (0.01 + 1.49 * 0.5)).abs() < 1e-6);
    assert_eq!(gravity::scale_at(3, 99.0), 1.6);
}

/// KH1 Gravity in the field (owner, 2026-11-04): its kind on a field Heartless is Re:CoM's plain field
/// strike (the battle with first strike), whether the KH1 row's (1) or Re:CoM's Gravity (0xD);
/// Thunder / Stop keep their paralyse / freeze.
#[test]
fn gravity_is_a_plain_field_strike() {
    for lv in 1..=3u8 {
        let row = gravity_row_bytes(&RECOM_GRAVITY_ROWS[lv as usize - 1]);
        let k = field_kind(spell::GRAVITY, lv, row[0xB] as u32);
        assert_eq!(k, 1);
        assert!(field_plain_strike(k));
        assert!(field_plain_strike(RECOM_GRAVITY_ROWS[lv as usize - 1][0xB] as u32), "0xD");
        assert!(!field_plain_strike(field_kind(spell::THUNDER, lv, 1)), "paralyse");
    }
    assert!(!field_plain_strike(field_kind(spell::STOP, 1, 1)), "freeze");
}

/// A flatten a hit cut short springs back KH1's way (+0.05 a frame): from Re:CoM's 0.1, 18 frames.
#[test]
fn a_cut_flatten_springs_back_at_kh1s_rate() {
    assert!(flatten::spring_back(1.0).is_none(), "Re:CoM's own end stood it up");
    assert!(flatten::spring_back(0.9995).is_none());
    assert!(flatten::spring_back(f32::NAN).is_none() && flatten::spring_back(0.0).is_none());
    let mut s = flatten::spring_back(0.1).unwrap();
    let mut frames = 0;
    while !s.done() {
        s.step(false);
        frames += 1;
    }
    assert_eq!(frames, 18);
    assert_eq!(s.y, 1.0);
}

/// The owner's "make it the size of re:com gravity" (2026-11-04, over KH1's size of 0.115.0): the KH1
/// sphere keeps Re:CoM's own growth (flag 0x80) and Re:CoM's draw radius (1.5 m), with a step that
/// ends at KH1's hit radius on KH1's frames: drawn at the radius ÷ 1.5 m, 1.133 × KH1's scale.
#[test]
fn gravity_sphere_draws_at_recoms_rule_on_kh1s_radius() {
    // Re:CoM's Gravity case's flags (`0x140222AB7`) carry the self-growth bit; ours keep it.
    assert_eq!(gravity::sphere_flags(0x41E1), 0x41E1);
    assert_eq!(gravity::sphere_flags(0x4161), 0x41E1);
    // Re:CoM's own numbers from its rows: draw radius `+4` × 0.01, step `+5` × 0.01.
    for (i, r) in RECOM_GRAVITY_ROWS.iter().enumerate() {
        assert_eq!(r[4] as f32 * 0.01, gravity::RECOM_DRAW_RADIUS, "row {i}");
        assert!((r[5] as f32 * 0.01 - gravity::RECOM_STEP[i]).abs() < 1e-6, "row {i}");
    }
    let f = gravity::draw_factor();
    assert!((f - 1.7 / 1.5).abs() < 1e-6, "{f}");
    for lv in 1u8..=3 {
        let (s, n) = gravity::of(lv);
        // The step reaches KH1's hit radius at the impact (the init's 2 steps + N updates).
        assert_eq!(gravity::steps_at_impact(lv), n + 2.0);
        assert!((gravity::growth_step_m(lv) * (n + 2.0) - gravity::impact_radius_m(lv)).abs() < 1e-5);
        let at = gravity::effect_scale_at_impact(lv);
        assert!((at - s * f).abs() < 1e-5, "LV{lv}: {at} vs {}", s * f);
        // Linear on KH1's frames: half way through the fall, about half of it.
        let half = gravity::recom_effect_scale(lv, n / 2.0 + 2.0);
        assert!((half / at - (n / 2.0 + 2.0) / (n + 2.0)).abs() < 1e-5);
        // The spawn (Re:CoM's 2 init steps) is near KH1's 0.01 × the factor: under 0.06.
        assert!(gravity::recom_effect_scale(lv, 2.0) < 0.06, "LV{lv}");
        // Against what 0.103.0 .. 0.112.0 drew ("really small"): over 20 times bigger.
        let old = gravity::old_effect_scale(lv, RECOM_GRAVITY_ROWS[lv as usize - 1][5], n);
        assert!(at / old > 20.0, "LV{lv}: old {old}");
    }
    let at: Vec<f32> = (1u8..=3).map(gravity::effect_scale_at_impact).collect();
    assert!((at[0] - 1.1333).abs() < 1e-3 && (at[1] - 1.7).abs() < 1e-3 && (at[2] - 1.8133).abs() < 1e-3, "{at:?}");
    // The hit radius stays KH1's.
    for (lv, r) in [(1u8, 1.7f32), (2, 2.55), (3, 2.72)] {
        assert!((gravity::impact_radius_m(lv) - r).abs() < 1e-5, "LV{lv}");
    }
}

/// M7 f2 (decision 38): under the falling sphere KH1 squashes the enemy's height (`+0x480`, the y of
/// its matrix scale) toward 0.8 with a wobble, and springs it back 0.05 a frame after.
#[test]
fn gravity_squash_is_kh1s_height_ease_and_wobble() {
    use squash::Squash;
    let mut s = Squash::default();
    assert_eq!((s.y, s.phase), (1.0, 0.0));
    // First update under the sphere: (1 − 0.8) × 0.95 + 0.8 − sin 0 × 0.05 = 0.99; φ = π/2.
    assert!((s.step(true) - 0.99).abs() < 1e-6);
    assert!((s.phase - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    // Second: (0.99 − 0.8) × 0.95 + 0.8 − 1 × 0.05 = 0.9305; φ = π.
    assert!((s.step(true) - 0.9305).abs() < 1e-5);
    // The phase runs π/2 a frame, wrapped into [−π, π] (`0x1402A8280`): π → −π/2 → 0 → π/2.
    s.step(true);
    assert!((s.phase + std::f32::consts::FRAC_PI_2).abs() < 1e-5, "{}", s.phase);
    // Under for a whole fall (60 frames): it hovers around 0.8 within the wobble's reach.
    for _ in 0..57 {
        let y = s.step(true);
        assert!(y > 0.68 && y < 1.0, "{y}");
    }
    let low = s.y;
    assert!((low - 0.8).abs() < 0.12, "{low}");
    // Out from under: +0.05 a frame to 1, the phase back to 0, then done.
    let y1 = s.step(false);
    assert!((y1 - (low + 0.05)).abs() < 1e-6 && s.phase == 0.0);
    let mut n = 1;
    while !s.done() {
        s.step(false);
        n += 1;
        assert!(n < 10);
    }
    assert_eq!(s.y, 1.0);
    // The test sphere: 170 cm × the scale, at the falling centre (2.3 m above the impact centre at
    // the release, at it after N frames).
    let impact = [1.0, 2.0, 3.0];
    assert_eq!(squash::centre_at(impact, 1, 0.0), [1.0, 4.3, 3.0]);
    assert!((squash::centre_at(impact, 2, 35.0)[1] - 3.15).abs() < 1e-5);
    assert_eq!(squash::centre_at(impact, 3, 60.0), impact);
    assert!((squash::radius_at(1, 0.0) - 0.017).abs() < 1e-6);
    assert!((squash::radius_at(3, 60.0) - 2.72).abs() < 1e-5);
}

#[test]
fn gravity_damage_is_kh1s_percent_of_current_hp() {
    // Gravity + 5 × max MP 3 = 55 % of the current HP × resist[8].
    assert_eq!(gravity_damage(1, 3, 200, 1.0, 0), 110);
    // An LV2 Heartless (80 %): 44 % → 88.
    assert_eq!(gravity_damage(1, 3, 200, 0.8, 0), 88);
    // A Re:CoM-only boss (decision 34, 2 %) at 1000 HP: 1000 × 0.55 × 0.02 = 11.
    assert_eq!(gravity_damage(1, 3, 1000, 0.02, 2), 11);
    // Under the floor: the floor (2), or 1 when the floor is under 2.
    assert_eq!(gravity_damage(1, 3, 50, 0.02, 2), 2);
    assert_eq!(gravity_damage(1, 3, 50, 0.02, 0), 1);
    // Graviga (70 + 40 = 110 % at max MP 8) on a twin: never more than ... the current HP × 110 %.
    assert_eq!(gravity_damage(3, 8, 100, 0.04, 2), 4);
    // Immune record (Sephiroth 0 %): still the floor.
    assert_eq!(gravity_damage(2, 3, 900, 0.0, 4), 4);
}

#[test]
fn gravity_falls_its_frames_then_strikes_once() {
    for (lv, n) in [(1u8, 60), (2, 70), (3, 60)] {
        let mut f = Fall::new(lv);
        let mut runs = 0;
        // The release run counted the first frame; the impact is the run N frames after it.
        while !f.run(1.0) {
            runs += 1;
            assert!(runs < 200);
        }
        assert_eq!(runs, n - 1, "level {lv}: N − 1 more frames fall, the next run strikes");
        assert_eq!(f.fallen(), n as f32);
    }
    // The impact sphere: a body that touches 1.7 m around the centre counts.
    let c = [0.0, 1.0, 0.0];
    let a = Area::Sphere { radius: 1.7 };
    assert!(body_touches(c, a, &BodyShape::Sphere { c: [2.0, 1.0, 0.0], r: 0.31 }));
    assert!(!body_touches(c, a, &BodyShape::Sphere { c: [2.0, 1.0, 0.0], r: 0.29 }));
    assert!(!body_touches(c, a, &BodyShape::Sphere { c: [0.0, 3.0, 0.0], r: 0.2 }), "a sphere, not a column");
    assert!(body_touches(c, a, &BodyShape::Column { a: [1.0, -5.0, 1.0], b: [1.0, 5.0, 1.0], r: 0.1 }));
}

#[test]
fn stop_rows_area_and_duration_are_kh1s() {
    let t = kh1_magic_table(&recom_rows());
    for lv in 1..=3u8 {
        let i = lv as usize - 1;
        let r = &t[table_index(spell::STOP, lv).unwrap()];
        let base = &RECOM_STOP_ROWS[i];
        // Re:CoM's clip 0x39, sets 21 / 22 / 23, and its stop kind 8 (Re:CoM's stop state).
        assert_eq!((&r[..3], r[4], r[0xC], r[0xE]), (&base[..3], base[4], base[0xC], base[0xE]));
        assert_eq!((r[2], r[0xB] as u32), (21 + lv - 1, RECOM_STOP_KIND));
        assert_eq!((r[3], r[0xF] & FLAG_KH1), (0, FLAG_KH1));
        assert_eq!(recom_base_row(spell::STOP, lv), Some([0x27, 0x3A, 0x3B][i]));
        assert_eq!(stop::area(lv), Area::Cylinder { radius: [2.5, 3.5, 5.0][i], half: 3.0 });
    }
    assert_eq!((cost(spell::STOP), release_wait(spell::STOP)), (200, 28.0));
    assert_eq!((cast_req::first(spell::STOP, false), cast_req::first(spell::STOP, true)), (0x3B, 0x88));
    // (2 + max MP) × 60 × resist[9] frames, the same at every level; 0 = immune.
    assert_eq!(stop::frames(3, 1.0), Some(300.0));
    assert_eq!(stop::frames(3, 0.3), Some(90.0));
    assert_eq!(stop::frames(8, 1.0), Some(600.0));
    assert_eq!(stop::frames(5, 0.0), None);
    assert_eq!(stop::ticks(3, 1.0), Some(15000));
    assert_eq!(stop::ticks(8, 1.0), Some(30000));
    assert_eq!(stop::ticks(3, 0.2), Some(3000));
    // The owner's 0.53.0 log: max MP 12 (840 frames) was clipped to the i16 carrier (655 frames);
    // the whole duration now goes into Re:CoM's i32 timer once its handler has entered the stop.
    assert_eq!(stop::ticks(12, 1.0), Some(42000));
    assert_eq!(stop::ticks(12, 0.6), Some(25200));
    assert_eq!(stop::carrier(42000), i16::MAX);
    assert_eq!(stop::carrier(25200), 25200);
    assert_eq!(stop::timer_after_entry(true, 42000), Some(42000));
    assert_eq!(stop::timer_after_entry(true, 25200), None, "Re:CoM's own write already holds it");
    assert_eq!(stop::timer_after_entry(false, 42000), None, "not stopped: nothing written");
    // Decision 14: the strike breaks into an enemy's attack (bit 0), as every KH1 hit; Re:CoM's
    // handler drops a hit on an attacking enemy otherwise (`0x14017F8CB`). Not for an immune enemy.
    assert!(stop::interrupts(0x8200108005, Some(25200)), "the owner's Creapplant mid-attack");
    assert!(stop::interrupts(0x80000005, Some(32767)), "the owner's Tornadostep mid-attack");
    assert!(!stop::interrupts(0x8200108810, Some(25200)), "not attacking: Re:CoM's path as it is");
    assert!(!stop::interrupts(0x8200108005, None), "immune: its attack goes on");
    assert!(!stop::interrupts(0x200_0001, Some(15000)), "already set");
    // The damage path returns the power for element 9 (the points of the duration).
    let mut rng = Kh1Rand::new(1);
    assert_eq!(spell_damage(&stop_attack(2), 3, &DefenderStats::default(), &mut rng), 5);
}

#[test]
fn stop_strikes_one_every_4_frames_nearest_first() {
    let order = thunder_order([0.0; 3], [(1, [3.0, 0.0, 0.0]), (2, [1.0, 0.0, 0.0]), (3, [2.0, 0.0, 0.0]), (4, [9.0, 0.0, 0.0])]);
    assert_eq!(order, vec![2, 3, 1, 4]);
    let mut s = Strikes::every(order, stop::INTERVAL);
    let a = stop::area(1);
    let inside = |id: u32| {
        let x = [0.0, 3.0, 1.0, 2.0, 9.0][id as usize];
        body_touches([0.0; 3], a, &BodyShape::Sphere { c: [x, 0.0, 0.0], r: 0.3 })
    };
    let mut at = Vec::new();
    for frame in 0..30 {
        if let Some(id) = s.run(1.0, inside) {
            at.push((frame, id));
        }
    }
    // 2.5 m and a 0.3 m body: 1 and 2 m inside; 3 m (0.2 m short) and 9 m passed over at once.
    assert_eq!(at, vec![(0, 2), (4, 3)]);
    assert!(s.done());
}

/// M7 f3 (decision 38): the held hits land at KH1's pace (6–11 frames apart) in Re:CoM's stop exe.
#[test]
fn stop_held_hits_land_at_kh1s_pace_in_recoms_stop_exe() {
    // KH1's delay `6 − (int)(r × −6)`: 6 .. 11 frames, a sixth each.
    assert_eq!([0.0, 0.1666, 0.17, 0.5, 0.8334, 0.999_999].map(stop::held_gap), [6.0, 6.0, 7.0, 9.0, 11.0, 11.0]);
    let mut seen = [0; 12];
    let mut rng = Kh1Rand::new(7);
    for _ in 0..6000 {
        seen[stop::held_gap(rng.next_f32()) as usize] += 1;
    }
    assert!(seen[..6].iter().all(|&n| n == 0) && seen[6..].iter().all(|&n| n > 800 && n < 1200), "{seen:?}");
    // Re:CoM's order, frame by frame (`0x1401A59A0`): the timer down by dt when above 0; the hit
    // handler (our wrapper: the frame after a landing, the timer for KH1's gap); the stop exe (phase
    // 1 lands the held hit and sets its own timer, half the reaction: here 1500; phase 2 waits while
    // the timer is above 0, then the next index, phase 1).
    for gaps in [vec![6.0, 6.0, 6.0], vec![11.0, 7.0, 9.0, 6.0, 10.0, 8.0, 11.0]] {
        let count = gaps.len() + 1;
        let (mut phase, mut timer, mut index, mut paced) = (1, 0i32, 0usize, None);
        let mut landed = Vec::new();
        for frame in 0..200 {
            if timer > 0 {
                timer -= 50;
            }
            if phase == 2 && paced != Some(index) && index + 1 < count {
                paced = Some(index);
                timer = stop::landing_timer(gaps[index], 50);
            }
            match phase {
                1 if index < count => {
                    landed.push(frame);
                    timer = 1500;
                    phase = 2;
                }
                1 => phase = 3,
                2 if timer <= 0 => {
                    index += 1;
                    phase = 1;
                }
                _ => {}
            }
        }
        let got: Vec<f32> = landed.windows(2).map(|w| (w[1] - w[0]) as f32).collect();
        assert_eq!(got, gaps, "{landed:?}");
    }
    // At 30 fps (100 ticks a game frame) the same time: 8 KH1 frames = 4 game frames = 2 of timer.
    assert_eq!((stop::landing_timer(6.0, 50), stop::landing_timer(11.0, 50), stop::landing_timer(8.0, 100)), (200, 450, 200));
}

#[test]
fn aero_rows_barrier_and_wind_are_kh1s() {
    let t = kh1_magic_table(&recom_rows());
    for lv in 1..=3u8 {
        let i = lv as usize - 1;
        let r = &t[table_index(spell::AERO, lv).unwrap()];
        let base = &RECOM_AERO_ROWS[i];
        assert_eq!((&r[..3], r[4]), (&base[..3], base[4]));
        assert_eq!((r[2], r[3], r[0xB], r[0xF] & FLAG_KH1), (24 + lv - 1, 0, 1, FLAG_KH1));
        assert_eq!(recom_base_row(spell::AERO, lv), Some([0x28, 0x3C, 0x3D][i]));
    }
    assert_eq!((cost(spell::AERO), release_wait(spell::AERO)), (200, 30.0));
    assert_eq!((cast_req::first(spell::AERO, false), cast_req::first(spell::AERO, true)), (0x3C, 0x89));
    // (18 + max MP) × 60 frames, the same at every level; the wind only from Aerora.
    assert_eq!((aero::frames(3), aero::frames(8)), (1260.0, 1560.0));
    assert!(!aero::has_wind(1) && aero::has_wind(2) && aero::has_wind(3));
    // Halved damage, at least 1 (KH1's order: after the resistance).
    assert_eq!((aero::halve(30), aero::halve(31), aero::halve(1), aero::halve(0)), (15, 15, 1, 0));
    // The wind: 8 + 2 × max MP through the curve, element 0.
    let a = aero_wind_attack(2);
    assert_eq!((a.power, a.stat_source, a.element, a.kind, a.radius), (8, 3, 0, 3, 80.0));
    let mut rng = Kh1Rand::new(1);
    let def = DefenderStats { defence: 5, ..Default::default() };
    let d = spell_damage(&a, 3, &def, &mut rng);
    assert_eq!(d, spell_damage(&thunder_attack(1), 0, &DefenderStats { defence: 5 + 2, ..Default::default() }, &mut rng), "8 + 6 − 5 = curve[9] = Thunder 16 + 0 − 7");
    // The countdown and the 30-frame re-hit.
    let mut b = aero::Barrier::new(3);
    let mut hits = 0;
    let mut frames = 0;
    loop {
        let (on, again) = b.step(1.0);
        frames += 1;
        hits += again as u32;
        if !on {
            break;
        }
    }
    assert_eq!(frames, 1260);
    assert_eq!(hits, 42);
}

#[test]
fn field_fx_wants_the_battle_set_only_for_kh1_casts_in_soras_story() {
    use field_fx::*;
    assert!(wanted(true, false));
    assert!(!wanted(false, false), "Combat = Re:CoM: Re:CoM's own field (no set)");
    assert!(!wanted(true, true), "Riku's story: Re:CoM's own field");
    assert!(!wanted(false, true));
    // A branch's target: `jmp 0x140211AA0` at 0x1401AF933 (E9 68 21 06 00), `call 0x140211560` at
    // 0x1401A95BD (E8 9E 7F 06 00); anything else is no branch.
    assert_eq!(branch_target(FIELD_JUMP_RVA, &[0xE9, 0x68, 0x21, 0x06, 0x00]), Some(PLAYER_GROUP_RVA));
    assert_eq!(branch_target(CAMP_CALL_RVAS[0], &[0xE8, 0x9E, 0x7F, 0x06, 0x00]), Some(UNLOAD_RVA));
    assert_eq!(branch_target(0x100, &[0xE8, 0xF0, 0xFF, 0xFF, 0xFF]), Some(0xF5), "backwards");
    assert_eq!(branch_target(0x100, &[0x90, 0, 0, 0, 0]), None);
    assert_eq!(branch_target(0x100, &[0xE8, 0, 0, 0]), None);
}

#[test]
fn strike_contact_takes_recoms_test_for_the_pair() {
    use strike_contact::{test_for, Test, ATTACK_SPHERE};
    // Re:CoM's key body | attack << 4 (`0x14016EAD0`): 0x22 sphere, 0x24 / 0x25 capsule.
    assert_eq!(test_for(2, ATTACK_SPHERE, 0), Some(Test::Sphere));
    assert_eq!(test_for(4, ATTACK_SPHERE, 0), Some(Test::Capsule));
    assert_eq!(test_for(5, ATTACK_SPHERE, 0x10), Some(Test::Capsule));
    // Other kinds: Re:CoM's pass never hits them (no test, no apply).
    assert_eq!(test_for(1, ATTACK_SPHERE, 0), None);
    assert_eq!(test_for(3, ATTACK_SPHERE, 0), None);
    assert_eq!(test_for(2, 1, 0), None);
    // Shape flags & 0xE == 6: the pass skips the body.
    assert_eq!(test_for(2, ATTACK_SPHERE, 6), None);
    assert_eq!(test_for(4, ATTACK_SPHERE, 0x17), None);
    assert_eq!(test_for(4, ATTACK_SPHERE, 0x4), Some(Test::Capsule));
}

#[test]
fn strike_sweep_falls_from_above_the_body_into_its_centre() {
    use strike_contact::{sweep, Test, MARGIN_M};
    // A sphere body: straight down from above its top (beyond both radii) to its centre; the
    // sphere test reads the attack's points as they are.
    let s = BodyShape::Sphere { c: [1.0, 2.8, -3.0], r: 0.6 };
    let (a, b) = sweep(Test::Sphere, &s, 0.5, 0.5);
    assert_eq!(b, [1.0, 2.8, -3.0]);
    assert_eq!((a[0], a[2]), (1.0, -3.0));
    assert!((a[1] - (2.8 + 0.6 + 0.5 + MARGIN_M)).abs() < 1e-6);
    // The start is outside the summed radius, the end inside: Re:CoM's sweep test must contact.
    assert!(a[1] - 2.8 > 0.6 + 0.5);
    // A column body: from above its top end to its middle; the capsule test adds the attack's
    // y offset to each point, so the raw points are lowered by it.
    let c = BodyShape::Column { a: [0.0, 0.2, 0.0], b: [0.0, 1.4, 0.0], r: 0.4 };
    let (a, b) = sweep(Test::Capsule, &c, 0.3, 0.5);
    assert!((b[1] + 0.5 - 0.8).abs() < 1e-6);
    assert!((a[1] + 0.5 - (1.4 + 0.4 + 0.3 + MARGIN_M)).abs() < 1e-6);
    assert_eq!((a[0], a[2], b[0], b[2]), (0.0, 0.0, 0.0, 0.0));
}

#[test]
fn strike_contact_finite_points() {
    assert!(strike_contact::finite([0.0, -1.0, 2.0]));
    assert!(!strike_contact::finite([f32::NAN, 0.0, 0.0]));
    assert!(!strike_contact::finite([0.0, f32::NEG_INFINITY, 0.0]));
}

/// Owner (2026-11-04, "Sleights outside battle"): KH1 Thunder and Stop on a field Heartless carry
/// Re:CoM's own kinds, which its handler `0x14019E020` turns into its paralyse (0x1C..0x1E → state 5)
/// and freeze (8 → state 6); other spells keep their KH1 row's kind 1 (the field strike).
#[test]
fn field_heartless_get_recoms_thunder_and_stop_kinds() {
    assert_eq!([1u8, 2, 3].map(|lv| field_kind(spell::THUNDER, lv, 1)), [0x1C, 0x1D, 0x1E]);
    assert_eq!(field_kind(spell::THUNDER, 0, 1), 0x1C, "a level outside 1..=3 clamps");
    assert_eq!([1u8, 2, 3].map(|lv| field_kind(spell::STOP, lv, 8)), [8, 8, 8]);
    for s in [spell::FIRE, spell::BLIZZARD, spell::GRAVITY, spell::AERO] {
        assert_eq!(field_kind(s, 2, 1), 1);
    }
    // The KH1 Thunder rows themselves (battle) keep kind 1.
    assert_eq!(thunder_row_bytes(&RECOM_THUNDER_ROWS[1])[0xB], 1);
    assert_eq!(MAP_ENEMY_TYPE, 0x1002);
    assert_ne!(MAP_ENEMY_TYPE, crate::battle::ENEMY_TYPE);
}

/// KH1's own hit flatten (owner, 2026-11-04, "2 yes"): Re:CoM's flatten state run with KH1's numbers
/// gives KH1's height frame by frame (`0x1402B7DE0` / `0x1402B7260`), both variants.
#[test]
fn kh1_flatten_in_recoms_state_is_kh1s_frame_by_frame() {
    use flatten::kh1::*;
    for v in [Variant::Ground, Variant::Air] {
        // KH1: the hit frame sets t = start (full height); each later frame counts 1, then the height.
        let mut kh1 = vec![1.0f32];
        let (mut t, mut h, mut getup_on) = (v.start(), 1.0f32, false);
        for _ in 1..100 {
            if getup_on {
                h = getup(h);
            } else {
                t -= 1.0;
                if t > HOLD {
                    h = LOW + 0.9 * (t - HOLD) / v.squash();
                } else if t > 0.0 {
                    h = LOW;
                } else {
                    getup_on = true; // request 0x67 queued: the ease from the next frame
                }
            }
            kh1.push(h);
        }
        // Re:CoM: on the hit's frame the begin (our timer) and the first exe; then the timer counted,
        // the exe.
        let mut recom = Vec::new();
        let (mut timer, mut phase, mut y) = (begin_ticks(v), 0, 1.0f32);
        for k in 0..100 {
            if k > 0 && timer > 0 {
                timer -= 50;
            }
            match step(v, phase, timer) {
                Step::Squash(hh) => y = hh,
                Step::Hold => {
                    y = LOW;
                    phase = 1;
                }
                Step::Wait => {
                    // Re:CoM's phase 1: at the timer's end height 1.0, its get-up, phase 2.
                    // (it writes 1.0; ours puts 0.1 back in the same call).
                    if timer <= 0 {
                        phase = 2;
                        y = LOW;
                    }
                }
                Step::GetUp => y = getup(y),
            }
            recom.push(y);
        }
        for (k, (a, b)) in kh1.iter().zip(&recom).enumerate() {
            assert!((a - b).abs() < 1e-5, "{v:?} frame {k} after the hit: KH1 {a} Re:CoM {b}");
        }
        // The numbers (index = frames after the hit): full height on the hit's frame, the first
        // squashed frame, the hold, the get-up's start.
        let n = v.squash() as usize;
        assert_eq!(kh1[0], 1.0);
        assert!((kh1[1] - (0.1 + 0.9 * (n as f32 - 1.0) / n as f32)).abs() < 1e-6);
        assert_eq!(kh1[n], LOW, "{v:?}: held from frame {n}");
        let end = v.start() as usize;
        assert_eq!(kh1[end], LOW, "{v:?}: still 0.1 on the timer's last frame");
        assert!((kh1[end + 1] - 0.37).abs() < 1e-6, "{v:?}: the get-up's first frame");
    }
    assert_eq!(begin_ticks(Variant::Ground), 3400);
    assert_eq!(begin_ticks(Variant::Air), 3750);
    assert_eq!(Variant::of(true, false), Variant::Ground);
    assert_eq!(Variant::of(false, false), Variant::Air);
    assert_eq!(Variant::of(true, true), Variant::Air);
    assert_eq!((PICK_KIND, STATE), (0xD, (24 + 0x13) * 0x40));
    // After the get-up motion the spring-back is KH1's +0.05 (0.117.0's).
    let s = flatten::spring_back(0.9).unwrap();
    assert!((s.y - 0.9).abs() < 1e-6);
}

/// 0.184.2: Riku's rows are Sora's but Fire's effect set, Dark Firaga's.
#[test]
fn rikus_rows_change_only_fires_effect_set() {
    let live: Vec<[[u8; 16]; 3]> = recom_rows();
    let t = kh1_magic_table(&live);
    let r = riku_look(&t);
    for (i, (a, b)) in t.iter().zip(&r).enumerate() {
        if i < 3 {
            assert_eq!(b[2], DARK_FIRAGA_SET, "Fire level {}", i + 1);
            let (mut a2, mut b2) = (*a, *b);
            a2[2] = 0;
            b2[2] = 0;
            assert_eq!(a2, b2, "Fire level {}: only the set", i + 1);
        } else {
            assert_eq!(a, b, "row {i}");
        }
    }
}

/// 0.184.3 (fork owner, 2026-11-23: "teste cada uma com o Neo shadow em cada nível"): each KH1 spell at each level on
/// KH1's Neoshadow, with Riku's KH1 MP (max 3, the owner's 0.184.2 log: "max MP 3") and the Neoshadow's KH1 record
/// (the owner's files, `kh1-stats.json` `CEnemyNeoshadow`, every world the same; Checked (files)): 150 HP, Defence 30,
/// floor 10 / cap 40 a hit, level 55, resist % [0] 50, [2] fire 49, [3] blizzard 50, [4] thunder 50, [8] gravity 0,
/// [9] stop 20. KH1's own formulas (`0x1402BFA80` through `kh1_core`, `gravity_damage`, `stop::frames`). Printed as a
/// table (`--nocapture`); the asserts hold KH1's rules on it.
#[test]
fn every_spell_and_level_on_kh1s_neoshadow() {
    let mut resist = [1.0f32; 16];
    for (i, v) in [50, 100, 49, 50, 50, 100, 100, 100, 0, 20, 50, 100, 100, 100, 100, 100].iter().enumerate() {
        resist[i] = *v as f32 / 100.0;
    }
    let neo = |hp: i32| DefenderStats { hp, max_hp: 150, defence: 30, level: 55, min_damage: 10, max_damage: 40, resist, hp_percent_scale: resist[8], ..Default::default() };
    let max_mp = 3;
    let mut rng = kh1_core::Kh1Rand::new(1);
    let hits_to_kill = |d: i32| if d > 0 { (150 + d - 1) / d } else { 0 };
    for lv in 1..=3u8 {
        for (s, name, data) in [(spell::FIRE, "Fire", fire_attack(lv)), (spell::BLIZZARD, "Blizzard", blizzard_attack(lv)), (spell::THUNDER, "Thunder", thunder_attack(lv))] {
            let d = spell_damage(&data, max_mp, &neo(150), &mut rng);
            eprintln!("{name} {lv}: power {} element {} -> {d} a hit (cap 40, resist {}%), {} casts to beat 150 HP", data.power, data.element, (resist[data.element as usize] * 100.0) as i32, hits_to_kill(d));
            assert!((1..=40).contains(&d), "{name} {lv}: within the Neoshadow's floor / cap ({d})");
            let _ = s;
        }
        let g = gravity_damage(lv, max_mp, 150, resist[8], 10);
        eprintln!("Gravity {lv}: power {} -> {g} on a full Neoshadow (resist[8] 0 %: KH1's % of HP times 0)", gravity_attack(lv).power);
        let w = spell_damage(&aero_wind_attack(lv), max_mp, &neo(150), &mut rng);
        eprintln!("Aero {lv}: barrier {} frames; wind {} (power 8, element 0, resist 50 %) every 30 frames: {w}", aero::frames(max_mp), if aero::has_wind(lv) { "on" } else { "off" });
    }
    let stop = stop::frames(max_mp, resist[9]);
    eprintln!("Stop (every level): {stop:?} frames on the Neoshadow ((2 + max MP) x 60 x 20 %)");
    assert_eq!(stop, Some(60.0), "(2 + 3) x 60 x 0.2");
    for lv in 1..=3u8 {
        eprintln!("Cure {lv}: Re:CoM's heal {} HP on Riku", cure::heal(lv));
    }
}
