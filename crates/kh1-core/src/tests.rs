//! Controller and combat tests against the documented KH1 numbers.

use crate::*;

fn sora(ab: Abilities) -> Controller {
    Controller::new(Params::kh1_defaults(), ab, Moveset::kh1_defaults(), Vec3::ZERO)
}

fn step(c: &mut Controller, i: Input, parts: &[TargetPart]) -> Output {
    c.step(&i, parts, &mut FlatFloor { floor_y: 0.0 }, 1.0)
}

fn held(prev: bool, now: bool) -> Button {
    Button::from_held(prev, now)
}

fn press_attack() -> Input {
    Input { attack: held(false, true), ..Default::default() }
}

fn started(o: &Output) -> Option<usize> {
    o.events.iter().find_map(|e| if let Event::AttackStarted { row, .. } = e { Some(*row) } else { None })
}

/// Steps idle until the combo window is open, then a few frames more (still inside it).
fn into_window(c: &mut Controller, parts: &[TargetPart], extra: u32) {
    for _ in 0..200 {
        if step(c, Input::default(), parts).combo_window {
            break;
        }
    }
    for _ in 0..extra {
        step(c, Input::default(), parts);
    }
    assert!(c.in_window());
}

#[test]
fn jump_apex_190_at_frame_4_plus_28() {
    let mut c = sora(Abilities::default());
    let mut h = Vec::new();
    for k in 0..40 {
        let i = Input { jump: held(k > 0, true), ..Default::default() };
        h.push(-step(&mut c, i, &[]).pos.y);
    }
    assert_eq!(h[3], 0.0, "4 crouch frames (0..=3)");
    assert!(h[4] > 0.0, "the arc starts on the 5th frame");
    assert!((h[31] - 190.0).abs() < 1e-3, "apex on the 32nd frame = 4 + 28: {}", h[31]);
    assert!(h[30] < 190.0 && h[32] < 190.0);
    // Rise on frame 5 (t = 1): 190·sin(π/2·sin(π/2·1/28)).
    let t1 = 190.0 * ((std::f32::consts::FRAC_PI_2 * (std::f32::consts::FRAC_PI_2 / 28.0).sin()).sin());
    assert!((h[4] - t1).abs() < 1e-3);
}

#[test]
fn released_jump_is_cut_after_t6() {
    let mut c = sora(Abilities::default());
    let mut top: f32 = 0.0;
    let mut cut_at = None;
    for k in 0..60 {
        let i = Input { jump: held(k > 0 && k < 1, k == 0), ..Default::default() };
        let o = step(&mut c, i, &[]);
        if o.events.contains(&Event::JumpCut) {
            cut_at = Some(k);
        }
        top = top.max(-o.pos.y);
    }
    assert_eq!(cut_at, Some(10), "take-off on frame 4, t = 6 at frame 10's think");
    let at6 = -Arc::hop(190.0, 0.0, GRAVITY).y_at(6.0, GRAVITY);
    let expect = at6 + GRAVITY * 25.0 / 2.0;
    assert!((top - expect).abs() < 1e-2, "{top} vs {expect}");
}

#[test]
fn fall_is_capped_at_20() {
    let mut c = Controller::new(Params::kh1_defaults(), Abilities::default(), Moveset::kh1_defaults(), Vec3::new(0.0, -3000.0, 0.0));
    let mut first = None;
    for k in 0..120 {
        let o = step(&mut c, Input::default(), &[]);
        if k == 0 {
            assert!(o.events.contains(&Event::WalkedOff));
        }
        assert!(o.displacement.y <= 20.0 + 1e-4);
        if first.is_none() && (o.displacement.y - 20.0).abs() < 1e-4 {
            first = Some(k);
        }
    }
    // g·(t − 0.5) reaches 20 after 41.8 frames of falling (t counted from frame 1).
    assert_eq!(first, Some(42));
}

#[test]
fn speed_snaps_at_tilt_0_8_and_direction_is_instant() {
    let fwd = Vec3::new(0.0, 0.0, 1.0);
    let mut c = sora(Abilities::default());
    let o = step(&mut c, Input { move_dir: fwd, tilt: 0.8, ..Default::default() }, &[]);
    assert_eq!(o.displacement, Vec3::new(0.0, 0.0, 2.0));
    let o = step(&mut c, Input { move_dir: fwd, tilt: 0.81, ..Default::default() }, &[]);
    assert_eq!(o.displacement, Vec3::new(0.0, 0.0, 8.0));
    let o = step(&mut c, Input { move_dir: fwd, tilt: 0.0, ..Default::default() }, &[]);
    assert_eq!(o.displacement, Vec3::ZERO, "no deceleration");
    // Turn: facing 0 → stick behind: the body turns 11.25° per frame, the movement goes at once.
    let back = Vec3::new(0.0, 0.0, -1.0);
    let o = step(&mut c, Input { move_dir: back, tilt: 1.0, ..Default::default() }, &[]);
    assert!((o.displacement.z + 8.0).abs() < 1e-5);
    assert!((o.facing.abs() - 11.25f32.to_radians()).abs() < 1e-6);
    let o = step(&mut c, Input { move_dir: back, tilt: 1.0, ..Default::default() }, &[]);
    assert!((o.facing.abs() - 22.5f32.to_radians()).abs() < 1e-5);
}

#[test]
fn ground_combo_13_11_8_with_combo_master() {
    let mut c = sora(Abilities { combo_master: true, ..Default::default() });
    let mut rows = Vec::new();
    rows.extend(started(&step(&mut c, press_attack(), &[])));
    into_window(&mut c, &[], 4);
    rows.extend(started(&step(&mut c, press_attack(), &[])));
    into_window(&mut c, &[], 4);
    let o = step(&mut c, press_attack(), &[]);
    rows.extend(started(&o));
    assert_eq!(rows, vec![13, 11, 8]);
    assert!(o.events.iter().any(|e| matches!(e, Event::AttackStarted { finisher: true, counter: 3, .. })));
    // The finisher has no window: presses are refused until it ends.
    for _ in 0..40 {
        let o = step(&mut c, press_attack(), &[]);
        assert_eq!(started(&o), None);
    }
}

#[test]
fn press_after_the_motion_starts_a_fresh_combo_and_dead_span_refuses() {
    let mut c = sora(Abilities { combo_master: true, ..Default::default() });
    step(&mut c, press_attack(), &[]);
    // Motion 200: window 22–40, no free track, 48 frames. Frames 40..47 refuse.
    for k in 1..48 {
        let o = step(&mut c, if k == 44 { press_attack() } else { Input::default() }, &[]);
        if k == 44 {
            assert!(o.events.contains(&Event::AttackRefused(Refused::NotAccepted)));
        }
    }
    let o = step(&mut c, Input::default(), &[]);
    assert!(!o.action_lock, "the queued idle released the lock at frame 48");
    let o = step(&mut c, press_attack(), &[]);
    assert_eq!(started(&o), Some(13));
    assert_eq!(o.combo_counter, 1);
}

#[test]
fn missed_swing_drops_the_press() {
    let mut c = sora(Abilities::default());
    step(&mut c, press_attack(), &[]);
    into_window(&mut c, &[], 2);
    let o = step(&mut c, press_attack(), &[]);
    assert!(o.events.contains(&Event::AttackRefused(Refused::LastSwingMissed)));
    assert_eq!(o.request, 200, "swing 1 goes on");
    assert_eq!(o.combo_counter, 1);
}

#[test]
fn lunge_at_gap_250() {
    let t = [TargetPart { owner: 7, centre: Vec3::new(0.0, -50.0, 280.0), radius: 30.0 }];
    let mut c = sora(Abilities::default());
    let o = step(&mut c, Input { lock: Some(0), ..press_attack() }, &t);
    assert_eq!(started(&o), Some(14));
    // The lunge's travel comes from its root motion: 134.94 forward.
    let mut z = 0.0;
    for _ in 0..70 {
        z = step(&mut c, Input::default(), &t).pos.z;
    }
    assert!((z - 134.94).abs() < 0.05, "{z}");
}

#[test]
fn air_combo_18_17_16() {
    let mut c = sora(Abilities { combo_master: true, ..Default::default() });
    for k in 0..8 {
        step(&mut c, Input { jump: held(k > 0, true), ..Default::default() }, &[]);
    }
    assert_eq!(c.state, state::AIR);
    let mut rows = Vec::new();
    let o = step(&mut c, press_attack(), &[]);
    rows.extend(started(&o));
    assert!((o.air_velocity.horizontal_length() - 8.0 * 0.95f32.sqrt()).abs() < 1e-4, "hit 1 pushes at 8, then drag");
    into_window(&mut c, &[], 2);
    let o = step(&mut c, press_attack(), &[]);
    rows.extend(started(&o));
    assert!((o.air_velocity.horizontal_length() - 4.0 * 0.95f32.sqrt()).abs() < 1e-4, "middle hits at 4");
    into_window(&mut c, &[], 2);
    let o = step(&mut c, press_attack(), &[]);
    rows.extend(started(&o));
    assert!((o.air_velocity.horizontal_length() - 12.0 * 0.95f32.sqrt()).abs() < 1e-4, "the finisher at 12");
    assert_eq!(rows, vec![18, 17, 16]);
    assert_eq!(c.state, state::AIR);
}

#[test]
fn dodge_roll_no_flinch_timer_and_travel() {
    let mut c = sora(Abilities { dodge_roll: true, ..Default::default() });
    let o = step(&mut c, Input { square: held(false, true), ..Default::default() }, &[]);
    assert!(o.events.contains(&Event::Rolled));
    assert!(o.flinch_immune);
    assert_eq!(c.no_flinch, 11.0, "the 12-frame timer counts in the roll's own frame");
    let mut on = vec![o.flinch_immune];
    let mut z = 0.0;
    for _ in 1..60 {
        let o = step(&mut c, Input::default(), &[]);
        on.push(o.flinch_immune);
        z = o.pos.z;
    }
    // The flag is set in frames 0–11 (cleared in frame 11's task, after its damage step).
    assert_eq!(on.iter().take_while(|x| **x).count(), 11);
    assert!(!on[11]);
    assert!((z - 350.0).abs() < 1e-3, "the roll's root travel: {z}");
}

#[test]
fn guard_beats_roll_below_run_tilt() {
    let ab = Abilities { dodge_roll: true, guard: true, ..Default::default() };
    let mut c = sora(ab.clone());
    let o = step(&mut c, Input { square: held(false, true), move_dir: Vec3::new(0.0, 0.0, 1.0), tilt: 0.8, ..Default::default() }, &[]);
    assert!(o.events.contains(&Event::Guarded));
    assert_eq!(o.request, 212);
    let mut c = sora(ab);
    let o = step(&mut c, Input { square: held(false, true), move_dir: Vec3::new(0.0, 0.0, 1.0), tilt: 0.9, ..Default::default() }, &[]);
    assert!(o.events.contains(&Event::Rolled));
}

#[test]
fn landing_locks_for_12_frames() {
    let mut c = sora(Abilities::default());
    let mut k = 0;
    let mut landed = false;
    while !landed && k < 200 {
        let o = step(&mut c, Input { jump: held(k > 0, k < 3), ..Default::default() }, &[]);
        landed = o.grounded && c.state == state::GROUND && k > 5;
        k += 1;
    }
    assert!(landed);
    let mut locked = 0;
    let mut first_free = None;
    for n in 0..30 {
        // Assumption-dependent: a press on the touchdown frame itself is accepted before the
        // pre-step locks (command menu task before Sora's), so the presses start one frame later.
        let o = step(&mut c, if n == 0 { Input::default() } else { press_attack() }, &[]);
        if n == 0 {
            assert!(o.events.contains(&Event::Landed));
        }
        if started(&o).is_some() {
            first_free = Some(n);
            break;
        }
        if o.action_lock {
            locked += 1;
        }
    }
    assert_eq!(locked, 12, "land motion frames 0–11");
    assert_eq!(first_free, Some(13), "the press after the free frame (12) attacks");
}

#[test]
fn jump_out_of_the_landing_lock() {
    let mut c = sora(Abilities::default());
    let mut k = 0;
    while !(c.state == state::GROUND && k > 5) {
        step(&mut c, Input { jump: held(k > 0, k < 3), ..Default::default() }, &[]);
        k += 1;
    }
    step(&mut c, Input::default(), &[]);
    assert!(c.lock);
    let o = step(&mut c, Input { jump: held(false, true), ..Default::default() }, &[]);
    assert!(o.events.iter().any(|e| matches!(e, Event::Jumped { .. })));
    assert_eq!(c.arc.t, 3.0, "restarts the crouch at t = 2, then this frame's +1");
}

#[test]
fn glide_starts_above_50_and_ends_on_release() {
    let ab = Abilities { glide: true, ..Default::default() };
    let mut c = sora(ab);
    let mut k = 0;
    // Jump to above 50, then press jump again.
    loop {
        let o = step(&mut c, Input { jump: held(k > 0, true), ..Default::default() }, &[]);
        k += 1;
        if -o.pos.y > 60.0 {
            break;
        }
    }
    let o = step(&mut c, Input { jump: held(false, true), ..Default::default() }, &[]);
    assert!(o.events.contains(&Event::GlideStarted { superglide: false }));
    assert_eq!(c.state, state::GLIDE);
    // Forward speed builds 0.4 per frame to 8; held, it glides.
    let fwd = Vec3::new(0.0, 0.0, 1.0);
    let mut last = Vec3::ZERO;
    for _ in 0..30 {
        last = step(&mut c, Input { jump: held(true, true), move_dir: fwd, tilt: 1.0, ..Default::default() }, &[]).displacement;
    }
    assert_eq!(c.state, state::GLIDE);
    assert!(last.y > 0.0 && last.y <= 1.6, "slow descent: {}", last.y);
    let o = step(&mut c, Input { jump: held(true, false), ..Default::default() }, &[]);
    assert!(o.events.contains(&Event::GlideEnded));
    assert_eq!(c.state, state::AIR);
    assert_eq!(c.glide_timer, 24.0);
}

#[test]
fn keyblade_hits_once_per_swing_and_finisher_launches() {
    let weapon = WeaponData::kingdom_key();
    let cp = CombatParams::kh1_defaults();
    let mut atk = AttackerStats { strength: 8, ..Default::default() };
    let mut dummies = vec![Dummy::standing(7, Vec3::new(0.0, 0.0, 120.0), DefenderStats { hp: 500, max_hp: 500, defence: 2, level: 1, ..Default::default() })];
    dummies[0].facing = std::f32::consts::PI; // facing Sora
    let mut c = sora(Abilities::default());
    let mut rows = Vec::new();
    let mut hits = Vec::new();
    for k in 0..400 {
        let parts: Vec<TargetPart> = dummies.iter().flat_map(|d| d.target_parts()).collect();
        let press = c.accepts_attack() && (c.in_window() || !c.lock) && rows.len() < 3 && (k == 0 || c.in_window());
        let o = step(&mut c, Input { lock: Some(0), ..if press { press_attack() } else { Input::default() } }, &parts);
        rows.extend(started(&o));
        // A blade held out in front at chest height.
        let f = Vec3::from_yaw(o.facing);
        let a = o.pos + Vec3::new(0.0, -80.0, 0.0) + f * 40.0;
        let b = a + f * 50.0;
        hits.extend(resolve_hits(&mut c, Blade { a, b }, o.pos + Vec3::new(0.0, -60.0, 0.0), &weapon, &mut atk, 1.0, &mut dummies, &cp));
        for d in &mut dummies {
            d.step(&cp.reaction, 0.0, 1.0);
        }
    }
    assert_eq!(rows, vec![13, 14, 8], "a close foe in front flags rows 13 and 14: the 2nd swing is the lunge");
    assert_eq!(hits.len(), 3, "one hit per swing");
    assert_eq!(hits[0].damage, 6, "curve[8 − 2]");
    assert!(matches!(hits[0].reaction, Reaction::Flinch { request: 0x4C }));
    assert_eq!(hits[0].mp_units, 3, "equal levels: the record charge, 3 units");
    assert!(matches!(hits[2].reaction, Reaction::Launch { .. }));
    assert!(hits[2].damage == 12 || hits[2].damage == 24);
}
