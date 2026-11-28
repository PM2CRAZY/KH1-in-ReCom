//! M11: KH1's Bandit in Re:CoM's battles (`docs/recom-m11-enemies.md` § The Bandit): where its
//! numbers sit in the player's `ex_2090.bd` (`xa_ex_2090.mdls` part 5) and which Re:CoM files it
//! extends (`ENEMY/0011`). The step itself is `crate::enemy`.
//!
//! The offsets are the script's own (Checked (files): the player's script disassembled with
//! `tools/kh1-ai-script.py`; the routine each sits in is named in `kh1_core::bandit`).

use crate::enemy::{AttackSpec, RowSpec, Spec};

/// Where each constant is pushed in `ex_2090.bd` (`push`: float or integer). A name with several
/// offsets must read the same at each.
pub const CONSTS: &[(&str, &[usize])] = &[
    // Init `0x34B0`: the stats block's walk / run (`+4` / `+8`).
    ("walk_speed", &[0x34E6]),
    ("run_speed", &[0x34F8]),
    // Entry `0x317C`'s idle search for a spawned Bandit (`0x2950(&wp40, 0, 1500, 100)`).
    ("idle_search", &[0x343A]),
    // Appearance `0x2C8A`: request 69 at blend 0, the 30-frame fade-in, the 30-frame wait.
    ("appear_blend", &[0x2C98]),
    ("appear_fade", &[0x2CAA]),
    ("appear_wait", &[0x2CD0]),
    // Wander `0x1346`: points [200, 400) and [400, 500) about where it stands, walked to (timeout
    // 300); 70 % it stops: 40 % of all an idle of [238, 308) frames; Agrabah: home beyond 500.
    ("wander_a_min", &[0x1396]),
    ("wander_a_max", &[0x139C]),
    ("wander_b_min", &[0x13DC]),
    ("wander_b_max", &[0x13E2]),
    ("wander_timeout", &[0x142C, 0x1446]),
    ("wander_stop_chance", &[0x145C]),
    ("wander_idle_chance", &[0x146E]),
    ("wander_idle_min", &[0x148E, 0x1616]),
    ("wander_idle_max", &[0x1494, 0x161C]),
    ("home_max", &[0x15DC]),
    ("home_timeout", &[0x15F8]),
    // The idle blends (`sys 12(0, 8)` / `sys 13(0, 8)`): the wander's, after the slash, the
    // throw, the spin; the pause's motion.
    ("idle_blend", &[0x1486, 0x160E, 0x2428, 0x239A, 0x227A, 0x44CE]),
    // Engaged `0xB38`.
    ("close_range", &[0xC78]),
    ("mid_range", &[0xC9E]),
    // Close `0x107A`.
    ("close_hop_chance", &[0x1084]),
    // Mid `0xFA8`.
    ("mid_strafe_turn", &[0xFC0]),
    ("mid_pause_min", &[0xFE0]),
    ("mid_pause_max", &[0xFE6]),
    ("mid_slash_chance", &[0xFFE]),
    ("mid_approach_time", &[0x100C]),
    ("mid_approach_stop", &[0x1012]),
    // Far `0xD8E` (outside Agrabah: past 2000 the leash `0x3B12`, not ported; else close to 400
    // and a pause of [30, 60)); Agrabah: 30 % the throw then close to 600, else 80 % the leap,
    // else close to 600.
    ("far_leash", &[0xDBE]),
    ("far_approach_time", &[0xDDA, 0xE38, 0xE66]),
    ("far_approach_stop", &[0xDE0]),
    ("far_pause_min", &[0xE00]),
    ("far_pause_max", &[0xE06]),
    ("agrabah_throw_chance", &[0xE26]),
    ("agrabah_far_stop", &[0xE3E, 0xE6C]),
    ("agrabah_leap_chance", &[0xE50]),
    // The closing pause after each round (`0x44AE(0, rand(60, 75))`).
    ("pause_min", &[0xE90, 0x1064, 0x10F8, 0x1150]),
    ("pause_max", &[0xE96, 0x106A, 0x10FE, 0x1156]),
    // The strafe's time pick `0x4606(0.5, 0.8)` and turn (`0x4334(target, time, 4)`).
    ("strafe_short_chance", &[0xFB0, 0x1028, 0x10A2, 0x10C8, 0x111C]),
    ("strafe_mid_chance", &[0xFB6, 0x102E, 0x10A8, 0x10CE, 0x1122]),
    ("strafe_turn", &[0x1038, 0x10B2, 0x10D8, 0x112C]),
    ("strafe_short_min", &[0x462E]),
    ("strafe_short_max", &[0x4634]),
    ("strafe_mid_min", &[0x4656]),
    ("strafe_mid_max", &[0x465C]),
    ("strafe_long_min", &[0x466E]),
    ("strafe_long_max", &[0x4674]),
    // Strafe `0x4334`: 60 % to one side (−90°), else the other (+90°, the turn negated); run at 8
    // for time + 10 frames, turning by turn × 0.5° a frame; stops within 200 or off the ground.
    ("strafe_side_chance", &[0x437E]),
    ("strafe_angle_first", &[0x4390]),
    ("strafe_angle", &[0x43A0]),
    ("strafe_speed", &[0x43C2]),
    ("strafe_extra", &[0x43DE]),
    ("strafe_stop", &[0x4432]),
    ("run_blend", &[0x4450]),
    ("strafe_turn_factor", &[0x4460]),
    // Pause `0x44AE(motion, n)`: n + 8 frames, until within 400.
    ("pause_extra", &[0x44BA]),
    ("pause_stop", &[0x4554]),
    // Approach `0x42DC(t, d)` → `0x2DEA(actor, 1, t + 8, 20, d)`.
    ("approach_extra", &[0x431A]),
    ("approach_retarget", &[0x4322]),
    // Hit `0x4FA`.
    ("hit_far", &[0x71C]),
    ("hit_throw_chance", &[0x734]),
    ("hit_stance_chance", &[0x772]),
    ("hit_slash_chance", &[0x78C]),
    ("hit_leap_chance", &[0x7C6]),
    ("hit_close", &[0x828]),
    ("hit_mid", &[0x872]),
    // Back hop `0x41B8` → `0x41DC(221, 25, 60, 30, 180)`, `sys 82`'s drag.
    ("hop_angle", &[0x41B8]),
    ("hop_speed", &[0x41BE]),
    ("hop_jump_height", &[0x41C4]),
    ("hop_time", &[0x41CA]),
    ("hop_drag", &[0x427E]),
    // The blend the host gives a reaction's queued motion (the engine's `attack_blend`; KH1's
    // reactions queue with 8, `0x1402B7260`): the attacks' own 8 (the slash, the throw, the leap).
    ("attack_blend", &[0x23E6, 0x22E8, 0x1BCC]),
    // Slash `0x23AA`: 200 at blend 8, waits 8 and 22 (its effect), then to the motion's end.
    ("slash_blend", &[0x23E6]),
    ("slash_wait", &[0x23F4]),
    ("slash_effect_wait", &[0x23FE]),
    // Throw `0x22DA`: 201 at blend 8, 8 frames, its effects, 82 frames facing, the blade (`sys 94`),
    // 290 frames facing, the blade back, to the motion's end.
    ("throw_blend", &[0x22E8]),
    ("throw_wait", &[0x22F6]),
    ("throw_release", &[0x231C]),
    ("throw_hold", &[0x2362]),
    // Spin `0x1EC6`: 210 at blend 4, 34 frames facing, to its end; then up to 4 rounds of 211 (12
    // frames and len − 2 − 12 more, gliding at the stats block's `+0xC` = 4), ended early once a
    // round's attack landed; then 212 (its effect's 12 / 22 / 16 frames), to its end.
    ("spin_blend", &[0x1EE6]),
    ("spin_windup", &[0x1EF4]),
    ("spin_speed", &[0x1F48]),
    ("spin_loops", &[0x1F8E]),
    ("spin_loop_blend", &[0x1FB4, 0x21B2]),
    ("spin_step", &[0x1FBC, 0x207E]),
    ("spin_trim", &[0x2076]),
    ("spin_end_wait_a", &[0x21C0]),
    ("spin_end_wait_b", &[0x21F6]),
    ("spin_end_wait_c", &[0x2232]),
    // Leap `0x1B36`: 216 at blend 8, to its end; movement state 7 toward a point 50 above Sora
    // (`0x1D42(217, &p, 70, 100, 500)`); 218 (bit 16) 24 frames, to its end; state 0, a drop.
    ("leap_blend", &[0x1BCC]),
    ("leap_lift", &[0x1BF8]),
    ("leap_speed", &[0x1C9A]),
    ("leap_arrive", &[0x1CA0]),
    ("leap_timeout", &[0x1CA6]),
    ("leap_air_blend", &[0x1D70, 0x1CBA]),
    ("leap_drop_height", &[0x1E40]),
    ("leap_stuck", &[0x1EA8]),
    // `0x24FE(v)`: stuck when the last frame's travel (`sys 118`) is under v × step × 0.5.
    ("leap_stuck_half", &[0x2524]),
    ("leap_end_wait", &[0x1CC8]),
    // The block stance `0x400A`: 230 at blend 4, to its end; 231 at blend 0 for [60, 90) frames,
    // ended once Sora is more than 120° off its facing; 233 at blend 0, to its end.
    ("stance_blend", &[0x409A]),
    ("stance_hold_blend", &[0x40D8, 0x4192]),
    ("stance_min", &[0x40F0]),
    ("stance_max", &[0x40F6]),
    ("stance_angle", &[0x414E]),
];

/// The request words the script pushes (bit 24 an action, bit 30 clears the action flag when the
/// motion starts, bit 16 holds its last pose: `0x14029FC40`, `0x14029F0A0`): the behaviour plays
/// exactly these.
pub const REQUESTS: &[(u32, usize)] = &[
    (69, 0x2C92),          // the appearance (plain)
    (0x4000_0000, 0x2CF0), // the idle queued after it (`sys 14`)
    (0, 0x1480),           // the wander's idle
    (0, 0x2422),           // the idle after the slash
    (200, 0x23E0),         // the slash
    (201, 0x22E2),         // the throw
    (210, 0x1EE0),         // the spin's wind-up
    (211, 0x1FAE),         // the spin
    (211, 0x206E),         // its length (`sys 43`)
    (0x4000_00D4, 0x21AC), // the spin's end 212
    (216, 0x1BC6),         // the leap's crouch
    (217, 0x1C90),         // the leap
    (0x0001_00DA, 0x1CB4), // its landing slash 218 (bit 16)
    (221, 0x41D0),         // the back hop's crouch
    (0x0001_0000, 0x4260), // the hop's rise (r + 1) | bit 16
    (0x4000_00E6, 0x4094), // the block stance 230
    (0x4000_00E7, 0x40D2), // 231
    (0x4000_00E9, 0x418C), // 233
    (2, 0x43CC),           // the strafe's run
];

/// The rows (script offsets) and the Re:CoM attack ids they take their power from (decision M10:
/// Re:CoM's attack power per attack). Re:CoM's Bandit has two (Checked (files): `EN0011.ASD`):
/// id 0 its slash `e011_050` (blade `wep0`, window 28–40: KH1's slash 200 is the same swing, its
/// window 26–38) and id 1 its whirl `e011_052` (a 0.5 m blade sphere through the whole loop of its
/// state 3 charge `e011_051` → `052` → `053`, Checked (exe)). So the slash 200 takes id 0, KH1's
/// spin (211, 212) id 1 (the same kind of attack: a whirling charge), and the leap 217 id 1
/// (Assumption: a moving lunge, nearest Re:CoM's charge; its rows' `+0x3C` are the slash's 30).
const SLASH: RowSpec = RowSpec { at: 0x23C, recom_id: 0 };
const SPIN: RowSpec = RowSpec { at: 0x2AC, recom_id: 1 };
const SPIN_END: RowSpec = RowSpec { at: 0x31C, recom_id: 1 };
const LEAP_BLADE: RowSpec = RowSpec { at: 0x3FC, recom_id: 1 };
const LEAP_BODY: RowSpec = RowSpec { at: 0x38C, recom_id: 1 };

/// `kh1_core::bandit::Attack::ALL`'s order. Not `.ASD` records (they never hurt Sora: reaction kind
/// 0, attack type 2, which meets only other attacks, `0x1402B3BE0` / `0x140528CB8`): the spin's body
/// row `0xEC` and the block stance's blade row `0x15C` are guard rows (`crate::enemy::GUARDS`); the
/// thrown blade's row `0x1CC` is its child object's (`crate::enemy::THROWN`: one of Re:CoM's own
/// bullets, its power Re:CoM's id 1).
pub const ATTACKS: &[AttackSpec] = &[
    AttackSpec { name: "slash", request: 200, rows: &[SLASH] },
    AttackSpec { name: "spin", request: 211, rows: &[SPIN] },
    AttackSpec { name: "spin end", request: 212, rows: &[SPIN_END] },
    AttackSpec { name: "leap", request: 217, rows: &[LEAP_BLADE, LEAP_BODY] },
];

/// The Bandit.
pub const SPEC: Spec = Spec {
    name: "bandit",
    recom_id: 11,
    kh1: "xa_ex_2090",
    script: b"ex_2090.bd",
    prefix: "e011_",
    recom_slots: 74,
    asd: (2, 0x6C, 0x1AC),
    consts: CONSTS,
    requests: REQUESTS,
    // The LV1 / LV2 stat records (`0x34B0`: `sys 45` with `0x14` / `0x80`, `crate::stats::TWINS`).
    records: &[0x14, 0x80],
    attacks: ATTACKS,
    manifest: "kh1-bandit.json",
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enemy::{build, Inputs};

    /// The player's files: `KH1_M11_DIR` (KH1's `xa_*.mdls` / `.mset` and Re:CoM's
    /// `ENEMY__NNNN__ENNNNN.*`) and `KH1_TEST_DATA` (`btltbl.bin`).
    fn inputs() -> Option<Inputs> {
        crate::enemy::test_inputs(&SPEC, true)
    }

    #[test]
    fn builds_from_the_players_files() {
        let Some(inp) = inputs() else { return };
        let b = build(&SPEC, &inp).unwrap();
        let get = |n: &str| b.consts.iter().find(|c| c.0 == n).map(|c| c.1).unwrap();
        // The script's numbers (the disassembly's).
        assert_eq!((get("walk_speed"), get("run_speed"), get("idle_search"), get("weight")), (2.0, 8.0, 1500.0, 1.0));
        assert_eq!((get("close_range"), get("mid_range"), get("hit_far"), get("far_leash")), (200.0, 400.0, 400.0, 2000.0));
        assert_eq!((get("hop_angle"), get("hop_speed"), get("hop_jump_height"), get("hop_time"), get("hop_drag")), (180.0, 30.0, 60.0, 25.0, 0.9));
        assert_eq!((get("leap_lift"), get("leap_speed"), get("leap_arrive"), get("leap_timeout"), get("leap_stuck")), (50.0, 70.0, 100.0, 500.0, 0.1));
        assert_eq!((get("spin_speed"), get("spin_loops"), get("spin_step"), get("spin_trim")), (4.0, 4.0, 12.0, 2.0));
        assert_eq!((get("strafe_angle_first"), get("strafe_angle"), get("stance_angle")), (-90.0, 90.0, 120.0));
        assert_eq!((get("throw_release"), get("throw_hold"), get("attack_blend")), (82.0, 290.0, 8.0));
        // The clips: KH1's (Re:CoM's bake of the appearance 69 / landing 7 kept), the loops KH1's
        // script relies on marked looping (211, 217, 231), the holding 218 not.
        let c = |r: u16| b.clips.iter().find(|c| c.request == r).unwrap().clone();
        assert_eq!((c(200).frames, c(201).frames, c(211).frames, c(212).frames, c(217).frames), (60, 438, 40, 94, 20));
        assert!(c(211).looped && c(217).looped && c(231).looped && !c(218).looped);
        assert_eq!((c(69).source, c(69).name.as_str()), ("recom", "e011_073"));
        for r in [221, 222, 223, 224, 230, 233, 205, 213, 232, 0x52, 0x5C, 0x4E, 0x50] {
            assert!(b.clips.iter().any(|x| x.request == r), "request {r}");
        }
        // Re:CoM's 74 slots unchanged.
        let a0 = crate::ann::Ann::parse(&inp.ann).unwrap();
        let a1 = crate::ann::Ann::parse(&b.ann).unwrap();
        for i in 0..SPEC.recom_slots {
            assert_eq!(a0.slots[i], a1.slots[i], "slot {i}");
        }
        // ASD: Re:CoM's bytes kept, one record per row's hit window (6: the slash 1, the spin 1,
        // its end 2, the leap 2), the tail moved.
        assert_eq!(&b.asd[0x6C..0x1AC], &inp.asd[0x6C..0x1AC]);
        assert_eq!(u32::from_le_bytes(b.asd[0x10..0x14].try_into().unwrap()), 2 + 6);
        assert_eq!(&b.asd[0x1AC + 6 * 0xA0..], &inp.asd[0x1AC..]);
        // The blade line (code 48 → KH1 bone 120, a chain end KH1 never poses): from the parent's
        // world × its local 52 cm (`0x1401D6320`), 2.5 × along `wep0` = 1.30 m; Re:CoM's own slash
        // record lies 1.16 m out on the same bone.
        let rec = |i: usize| &b.asd[0x1AC + i * 0xA0..0x1AC + (i + 1) * 0xA0];
        let f = |r: &[u8], o: usize| f32::from_le_bytes(r[o..o + 4].try_into().unwrap());
        let r0 = rec(0);
        assert_eq!(&r0[0..4], b"wep0");
        assert!((f(r0, 0x14) - 1.3013).abs() < 2e-3 && f(r0, 0x18).abs() < 1e-3 && f(r0, 0x1C).abs() < 1e-3, "{:?}", (f(r0, 0x14), f(r0, 0x18), f(r0, 0x1C)));
        assert_eq!((u32::from_le_bytes(r0[0x5C..0x60].try_into().unwrap()), u32::from_le_bytes(r0[0x60..0x64].try_into().unwrap()), u32::from_le_bytes(r0[0x64..0x68].try_into().unwrap())), (26 * 50, 38 * 50, 0));
        assert_eq!(b.parts.len(), 6);
        assert!(b.manifest.contains("\"format\": \"kh1-enemy 1\"") && b.manifest.contains("\"enemy\": \"bandit\""));
        // The guard rows (M11, the shared enemy-guard step): the spin's body row and the stance's blade.
        let g = crate::enemy::guard_rows("bandit", crate::stats::mdls_part(&inp.mdls, 5).unwrap(), &kh1_formats::mset::MotionSet::parse(&inp.mset).unwrap(), &b.clips).unwrap();
        assert_eq!(g.iter().map(|g| (g.name, g.clash_type, g.priority, g.recoil, g.slot, g.turn, g.cos)).collect::<Vec<_>>(), vec![("spin", 2, 90, 213, 1, 0.0, 0.0), ("stance", 2, 30, 232, 0, 0.0, 0.0)]);
        assert_eq!(g[0].windows, vec![(210, vec![(10.0, 46.0)]), (211, vec![(0.0, 40.0)]), (212, vec![(0.0, 50.0)])]);
        assert_eq!(g[1].windows, vec![(230, vec![(0.0, 12.0)]), (231, vec![(0.0, 60.0)]), (233, vec![(0.0, 10.0)])]);
        assert!(b.manifest.contains("\"guards\": ["));
        // The thrown scimitar (`ex_2090_00.bd` and row `0x1CC`; Re:CoM's whirl id 1, its record's kind
        // index 0 = Re:CoM kind 1).
        assert_eq!((get("blade_life"), get("blade_turn_back"), get("blade_speed"), get("blade_accel"), get("blade_turn"), get("blade_slide"), get("blade_slide_drag")), (300.0, 120.0, 10.0, -0.5, 3.0, 60.0, 0.9));
        assert_eq!((get("blade_radius"), get("blade_recom_id"), get("blade_kind_index"), get("blade_flag2")), (70.0, 1.0, 0.0, 0.0));
        println!("blade bone (Re:CoM) {}", get("blade_bone"));
        std::fs::write(std::env::temp_dir().join("kh1-bandit-test.json"), &b.manifest).unwrap();
        println!("{}", b.records.join("\n"));
        println!("parts {:?}", b.parts);
    }
}
