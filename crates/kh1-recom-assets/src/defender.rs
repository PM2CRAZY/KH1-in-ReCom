//! M11: KH1's Defender in Re:CoM's battles (`docs/recom-m11-enemies.md` § The Defender): where its numbers sit in
//! the player's `ex_2340.bd` (`xa_ex_2340.mdls` part 5) and its two children (part 3: the flame `ex_2340_00.bd`, the
//! fireball `ex_2340_01.bd`), and which Re:CoM files it extends (`ENEMY/0026`). The step itself is `crate::enemy`.
//!
//! The offsets are the scripts' own (Checked (files): the player's scripts disassembled with
//! `tools/kh1-ai-script.py`, the parent's code from `0x4C8`, the children's from `0x18`; the routine each sits in is
//! named in `kh1_core::defender`).

use crate::enemy::{AttackSpec, RowSpec, Spec};

/// Where each constant is pushed in `ex_2340.bd` (float or integer); a name with several offsets must read the same
/// at each.
pub const CONSTS: &[(&str, &[usize])] = &[
    // Init `0x27EC`: the stats block's walk (`+4` = 2.5); the run keeps the stats block's default (no run motion).
    ("walk_speed", &[0x27FA]),
    ("run_speed", &[]),
    // The turn rate (`sys 125`): 12° a frame (init `0x2A16`, the supervisor's `0x1C3C`), 2° while the walk (1) or the
    // fireball's wind-up (210) plays (`0x1C4A`).
    ("turn_deg", &[0x2A10, 0x1C40]),
    ("turn_slow_deg", &[0x1C4E]),
    // The engaged round `0x6EE`: close (< 470) `0x8E0`, far (> 600) `0x84C`, else mid `0x896`.
    ("close_range", &[0x72E]),
    ("far_range", &[0x748]),
    // Close `0x8E0`: rand < 0.3 the flame; > 0.4 the strike, then within 400 the spin and (rand > 0.3) the flame else a
    // fireball, beyond 600 a fireball, else the approach; otherwise (0.3..0.4) the spin.
    ("close_flame_chance", &[0x8EA]),
    ("close_strike_above", &[0x904]),
    ("close_follow_range", &[0x934]),
    ("close_follow_flame_above", &[0x950]),
    ("close_follow_far", &[0x97C]),
    // Far `0x84C` / mid `0x896`: rand < 0.3 the approach, > 0.5 a fireball, else the guard.
    ("far_approach_chance", &[0x856]),
    ("far_fire_above", &[0x870]),
    ("mid_approach_chance", &[0x8A0]),
    ("mid_fire_above", &[0x8BA]),
    // The approach `0xD38`: within 500 a walk of [100, 300) frames, else [300, 500); the walk (blend 8), 8 frames,
    // then each frame along the facing at 2.5 turning to the target; within 250 the idle and the strike; stuck (moved
    // under 1.25 × step) it stops.
    ("approach_near", &[0xD7A]),
    ("approach_near_min", &[0xD88]),
    ("approach_near_max", &[0xD8E]),
    ("approach_far_min", &[0xDA0]),
    ("approach_far_max", &[0xDA6]),
    ("walk_blend", &[0xDC2]),
    ("walk_wait", &[0xDCA]),
    ("approach_speed", &[0xE2E]),
    ("approach_stop", &[0xE50]),
    ("stuck_speed", &[0xEAC]),
    // `0x3344`: the idle (blend 8), 6 frames.
    ("idle_blend", &[0x334E]),
    ("idle_wait", &[0x3356]),
    // The guard `0xEF6`: only with Sora within 40° of its front; 203 (action) then 204 held for [60, 120) frames;
    // 50 % it may break out once fewer than [30, 60) are left; Sora off its front (40°): a fireball; within 30° and
    // the break-out: within 350 the strike, beyond 600 a fireball, else the flame; within 200: 50 % the strike else the
    // flame; at the end 205 (action, blend 8).
    ("guard_break_min", &[0xF1C]),
    ("guard_break_max", &[0xF22]),
    ("guard_time_min", &[0xF30]),
    ("guard_time_max", &[0xF36]),
    ("guard_break_chance", &[0xF4E]),
    ("guard_cone", &[0xF70, 0x1052]),
    ("guard_hold_wait", &[0xFD0]),
    ("guard_aim_cone", &[0x1094]),
    ("guard_strike_range", &[0x1100]),
    ("guard_far", &[0x1120]),
    ("guard_close", &[0x1164]),
    ("guard_close_strike", &[0x118A]),
    ("guard_drop_blend", &[0x11FE]),
    // The shield recoil `0x1230`: 206 (action, blend 2).
    ("recoil_blend", &[0x1256]),
    // The flame `0x1302`: the turn first; 212 for its length − 2 frames, the child 0; from frame 180 with Sora off its
    // front (100°): within 500 50 % the turn-around else the spin, beyond a fireball.
    ("flame_len_less", &[0x1342]),
    ("flame_turn_cone", &[0x1442]),
    ("flame_turn_after", &[0x1478]),
    ("flame_turn_near", &[0x149A]),
    ("flame_turn_chance", &[0x14B2]),
    // The fireballs `0x150C`: 210, 211 queued; the shot `0x15F6` (211 for its length − 4 frames, the child 1).
    ("fire_len_less", &[0x153C]),
    ("shot_len_less", &[0x161E]),
    // The strike `0x16F8` / `0x1D64`: Sora off its front (30°): it turns to him; 209 (action).
    ("strike_cone", &[0x174A, 0x1D86]),
    // The turn `0x17C0`: Sora within 30° of its front: nothing; within 50° of its back: the turn-around (201); else the
    // side steps (200 / 202 by the side: within 80° of its left (−90°) 202) until he is within 35° of its front or
    // 50° of its back, at most 3, then the strike.
    ("face_cone", &[0x1800]),
    ("back_deg", &[0x181C, 0x195E]),
    ("back_cone", &[0x1822, 0x1964]),
    ("side_deg", &[0x1846]),
    ("side_cone", &[0x184C]),
    ("step_max", &[0x18E4]),
    ("done_cone", &[0x1934]),
    // The steps: their length − 2 frames turning 0.45° a frame (202 by −0.45); the turn-around 201 by 2.36.
    ("step_len_less", &[0x1AA8, 0x19C2, 0x1B86]),
    ("step_deg", &[0x1B04, 0x1A1E]),
    ("turnaround_deg", &[0x1BE2]),
    // The spin `0x1C58`: within 400 its rows and 208 turning −3° a frame to its end; else the strike.
    ("spin_range", &[0x1C92]),
    ("spin_deg", &[0x1CF2]),
    // The appearance `0x9F8`: in the air it falls (its speed + 0.25 a frame to 20); 213; unhittable to its end.
    ("fall_accel_a", &[0xAE4]),
    ("fall_accel_b", &[0xAEE]),
    ("fall_max", &[0xAFE, 0xB0C]),
    // The hit state `0x514` (no partner): beyond 400 70 % a fireball else the approach; within: < 0.3 the strike, > 0.7
    // the spin, else a fireball.
    ("hit_far", &[0x60E]),
    ("hit_far_fire_above", &[0x626]),
    ("hit_near_strike", &[0x658]),
    ("hit_near_spin_above", &[0x678]),
    // The hit handler `0x2BC0`: the 5th repel record (count `wp+0x58`) in its fireballs, its flame or a quiet motion:
    // the EXP bonus EXP / 10 + 1 (`sys 194`) and the shield bash 207.
    ("repel_count", &[0x2C90]),
    ("exp_div", &[0x2CE0, 0x2DE0, 0x2E7A, 0x2F48]),
    ("exp_add", &[0x2CE8, 0x2DE8, 0x2E82, 0x2F50]),
    // Its own rows' serials (`sys 188`, row `+0x6C`): a hit from one of them (its batted fireball) on its shield from the
    // front is the big hit.
    ("own_row_first", &[0x2B22, 0x2DAA]),
    ("own_row_last", &[0x2B38, 0x2DC0]),
    // `sys 94`'s child indices: the flame (0, `0x1382`), the fireball (1, `0x164A`).
    ("flame_child", &[0x1382]),
    ("fire_child", &[0x164A]),
    // The blend the host gives a reaction's queued motion (the engine's `attack_blend`; KH1's reactions queue with 8,
    // `0x1402B7260`): the scripts' own 8 (the walk, the idle).
    ("attack_blend", &[0xDC2, 0x334E]),
];

/// The flame `ex_2340_00.bd` (entry `0x22A`): it waits up to 110 frames while the parent plays 212 (`0x72E`; it ends
/// if 212 stops first); one attack object from the parent's row (`0x38C`) in mode 3 (`sys 213(…, 3)`), shape 2 (a
/// chain of spheres, `0x1D0`), its hit-once mask cleared every 10 frames (`+0x50`); each frame for 140 frames: from
/// bone 105 (code `0x1069`: bit 12 names the bone) at t 3, 25 lower, to the same bone at t 15, 25 lower, radii 20 to
/// 40, the positions of 7 frames before (an 8-slot ring, `0x7A6`); for its first 30 frames (life above 110) the line
/// and the radii grow by (140 − life) / 30; once the parent stops 212, 10 frames more. Checked (files).
pub const FLAME_CONSTS: &[(&str, &[usize])] = &[
    ("flame_request", &[0x23A, 0x69A]),
    ("flame_delay", &[0x22E]),
    ("flame_rehit", &[0x284]),
    ("flame_t0", &[0x2A8, 0x2E6, 0x424]),
    ("flame_lift0", &[0x30C, 0x456]),
    ("flame_t1", &[0x322, 0x46C]),
    ("flame_lift1", &[0x34C, 0x4A2]),
    ("flame_ring", &[0x36E, 0x406, 0x4B2]),
    ("flame_life", &[0x3E4, 0x568]),
    ("flame_r0", &[0x4F6]),
    ("flame_r1", &[0x500]),
    ("flame_grow_until", &[0x556]),
    ("flame_grow_div", &[0x574]),
    ("flame_tail", &[0x6A8, 0x6C8]),
];

/// The flame's integer pushes: shape 2 (`0x268`), mode 3 (`0x276`),
/// bone code `0x1069` (`0x2A2`, `0x2E0`, `0x31C`, `0x41E`, `0x466`), its effect 7 (`0x2C4`).
pub const FLAME_REQUESTS: &[(u32, usize)] = &[(2, 0x268), (3, 0x276), (0x1069, 0x2A2), (0x1069, 0x2E0), (0x1069, 0x31C), (0x1069, 0x41E), (0x1069, 0x466), (7, 0x2C4)];

/// The fireball `ex_2340_01.bd` (entry `0x5E6`): its start `sys 19(parent, 0x106C, 3)` (bone 108 at t 3), `0x364(…,
/// turn 1, speed rand [8, 10))`, along the parent's facing (`sys 143` with no head look); `0x18(…, model 11, life 100,
/// fade 10)`: each frame it turns at most 1° toward its target's lock point 0 while that lives (`sys 24`) and moves
/// (`sys 18`); a hit ends it; a repel re-aims it (`0x1B8`: at the swinger's target, half-way from the way to its
/// parent, or at the parent) with the row `0x31C`, its clock back to 0. Checked (files).
pub const FIRE_CONSTS: &[(&str, &[usize])] = &[("fire_t", &[0x5F6]), ("fire_turn", &[0x60E]), ("fire_speed_min", &[0x614]), ("fire_speed_max", &[0x61A]), ("fire_life", &[0x640]), ("fire_fade", &[0x646])];

/// The fireball's integer pushes: its bone code `0x106C` (`0x5F0`), its model 11 (`0x63A`).
pub const FIRE_REQUESTS: &[(u32, usize)] = &[(0x106C, 0x5F0), (11, 0x63A)];

/// The request words the script pushes (bit 24 an action, bit 30 a queue, bit 16 a hold at the end): the behaviour
/// plays exactly these.
pub const REQUESTS: &[(u32, usize)] = &[
    (213, 0xB8A),          // the appearance
    (1, 0xDBC),            // the approach's walk
    (1, 0xDEA),
    (0x0100_00CB, 0xF90),  // the guard: 203 (action), 204 queued and held
    (0x4000_00CC, 0xFA8),
    (204, 0xFC2),
    (204, 0xFF0),
    (0x0100_00CD, 0x11F8), // 205 (action), the idle queued
    (0x4000_0000, 0x1210),
    (0x0100_00CE, 0x1250), // the shield recoil 206 (action)
    (0x4000_0000, 0x1268),
    (0x0100_00CF, 0x12D2), // the shield bash 207 (action)
    (0x4000_0000, 0x12EA),
    (212, 0x133A),         // the flame 212, the idle queued
    (212, 0x1352),
    (0, 0x136A),
    (211, 0x1534),         // the fireballs: 210, 211 queued
    (210, 0x155E),
    (211, 0x1576),
    (211, 0x1616),
    (211, 0x1638),
    (0x0100_00D1, 0x1790), // the strike 209 (action)
    (0x4000_0000, 0x17A8),
    (0x0100_00D1, 0x1DC8),
    (0x4000_0000, 0x1DE0),
    (202, 0x19BA),         // the steps (bit 16: held at the end)
    (0x0001_00CA, 0x19D2),
    (202, 0x1A04),
    (200, 0x1AA0),
    (0x0001_00C8, 0x1AB8),
    (200, 0x1AEA),
    (201, 0x1B7E),         // the turn-around
    (0x0001_00C9, 0x1B96),
    (201, 0x1BC8),
    (208, 0x1CC4),         // the spin
    (0x0001_00D0, 0x1CD6),
    (0, 0x3348),           // the idle (blend 8)
];

/// Its attacks' rows and the Re:CoM attack id each takes its power from (decision M10: Re:CoM's attack power per
/// attack; owner 2026-11-02: "the Re:CoM attack of the same kind"). Re:CoM's Defender has two attacks (Checked (files +
/// decomp)): id 0 its melee `e026_050` (state 2 `0x14018DA90`; `EN0026.ASD`: one record on `lb_wep0`, 0.7 m), id 1 its
/// fireball (state 3 `0x14018DAF0`: a `CBullet` from `lb_wep0` with `0x14017F0E0(enemy, 1, 1, 0)`'s AP; no `.ASD`
/// record). Every KH1 melee row takes id 0: the strike 209 (`0x15C`), the spin (`0x1CC`), the shield bash 207
/// (`0x3FC`). The spin's other row `0x23C` is a guard row (type 1, `crate::enemy::GUARDS`).
const STRIKE: RowSpec = RowSpec { at: 0x15C, recom_id: 0 };
const SPIN: RowSpec = RowSpec { at: 0x1CC, recom_id: 0 };
const BASH: RowSpec = RowSpec { at: 0x3FC, recom_id: 0 };

/// `kh1_core::defender::Attack::ALL`'s order. The fireballs and the flame are its children's (Re:CoM bullets, the
/// host's `fire_*` / `flame_*` numbers).
pub const ATTACKS: &[AttackSpec] = &[
    AttackSpec { name: "strike", request: 209, rows: &[STRIKE] },
    AttackSpec { name: "spin", request: 208, rows: &[SPIN] },
    AttackSpec { name: "bash", request: 207, rows: &[BASH] },
];

/// The Defender.
pub const SPEC: Spec = Spec {
    name: "defender",
    recom_id: 26,
    kh1: "xa_ex_2340",
    script: b"ex_2340.bd",
    prefix: "e026_",
    recom_slots: 75,
    asd: (1, 0xBC, 0x15C),
    consts: CONSTS,
    requests: REQUESTS,
    // The LV1 / LV2 stat records (`sys 45` with `0x14` / `0x80`).
    records: &[0x14, 0x80],
    attacks: ATTACKS,
    manifest: "kh1-defender.json",
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enemy::{build, Inputs};

    /// The player's files: `KH1_M11_DIR` (KH1's `xa_*.mdls` / `.mset` and Re:CoM's `ENEMY__NNNN__ENNNNN.*`) and
    /// `KH1_TEST_DATA` (`btltbl.bin`).
    fn inputs() -> Option<Inputs> {
        crate::enemy::test_inputs(&SPEC, true)
    }

    #[test]
    fn builds_from_the_players_files() {
        let Some(inp) = inputs() else { return };
        let b = build(&SPEC, &inp).unwrap();
        let get = |n: &str| b.consts.iter().find(|c| c.0 == n).map(|c| c.1).unwrap();
        // The script's numbers (the disassembly's).
        assert_eq!((get("walk_speed"), get("turn_deg"), get("turn_slow_deg"), get("close_range"), get("far_range"), get("weight")), (2.5, 12.0, 2.0, 470.0, 600.0, 1.0));
        assert_eq!((get("step_deg"), get("turnaround_deg"), get("spin_deg"), get("side_deg"), get("step_max"), get("face_cone")), (0.45, 2.36, -3.0, -90.0, 3.0, 30.0));
        assert_eq!((get("guard_time_min"), get("guard_time_max"), get("guard_cone"), get("approach_speed"), get("approach_stop"), get("spin_range")), (60.0, 120.0, 40.0, 2.5, 250.0, 400.0));
        assert_eq!((get("repel_count"), get("exp_div"), get("exp_add"), get("own_row_first"), get("own_row_last"), get("flame_child"), get("fire_child")), (5.0, 10.0, 1.0, 12900.0, 12908.0, 0.0, 1.0));
        // The children's.
        assert_eq!((get("fire_t"), get("fire_turn"), get("fire_speed_min"), get("fire_speed_max"), get("fire_life"), get("fire_fade"), get("fire_radius")), (3.0, 1.0, 8.0, 10.0, 100.0, 10.0, 60.0));
        assert_eq!((get("flame_request"), get("flame_delay"), get("flame_rehit"), get("flame_t0"), get("flame_t1"), get("flame_lift0"), get("flame_lift1")), (212.0, 110.0, 10.0, 3.0, 15.0, 25.0, 25.0));
        assert_eq!((get("flame_r0"), get("flame_r1"), get("flame_life"), get("flame_ring"), get("flame_grow_until"), get("flame_grow_div"), get("flame_tail")), (20.0, 40.0, 140.0, 8.0, 110.0, 30.0, 10.0));
        assert_eq!((get("fire_recom_id"), get("fire_kind")), (1.0, 1.0));
        // Its rows' serials: the handler's range holds every row of its own (`+0x6C`, 12900 .. 12907).
        let script = crate::stats::mdls_part(&inp.mdls, 5).unwrap();
        for at in [0xEC, 0x15C, 0x1CC, 0x23C, 0x2AC, 0x31C, 0x38C, 0x3FC] {
            let serial = u32::from_le_bytes(script[at + 0x6C..at + 0x70].try_into().unwrap()) as f32;
            assert!(serial >= get("own_row_first") && serial <= get("own_row_last"), "{at:#x}: {serial}");
        }
        // The points: the fireball's start on Re:CoM's `chin` (49, KH1 bone 108's parent), the flame's ends on `lb_wep0`
        // (48, bone 105's parent), along the bone's x (t scales the parent → bone translation: the flame's 15 / 3 = 5).
        let skel = crate::mdl::RecomSkeleton::parse(&inp.mdl).unwrap();
        assert_eq!((skel.bones[get("fire_at_bone") as usize].name.as_str(), skel.bones[get("flame0_bone") as usize].name.as_str(), get("flame1_bone")), ("chin", "lb_wep0", get("flame0_bone")));
        assert!((get("flame1_x") / get("flame0_x") - 5.0).abs() < 1e-4);
        assert!(["fire_at_y", "fire_at_z", "flame0_y", "flame0_z", "flame1_y", "flame1_z"].iter().all(|n| get(n).abs() < 1e-5));
        // The clips: all KH1's; their lengths; the strike, the bash and the fireball with root motion.
        let c = |r: u16| b.clips.iter().find(|c| c.request == r).unwrap().clone();
        assert_eq!((c(200).frames, c(201).frames, c(204).frames, c(207).frames, c(208).frames, c(209).frames, c(211).frames, c(212).frames, c(213).frames), (60, 76, 140, 208, 120, 128, 100, 280, 66));
        assert!(c(207).root && c(209).root && c(211).root && !c(208).root && !c(208).looped && !c(209).looped);
        // The steps and the turn-around hold at their end (`enemy::HELD_CLIPS`: KH1's bit 16); the guard held and the flame loop.
        assert!(!c(200).looped && !c(201).looped && !c(202).looped && c(204).looped && c(212).looped);
        assert!(b.clips.iter().all(|x| x.source == "kh1"));
        for r in [0, 1, 6, 7, 72, 73, 74, 75, 76, 77, 78, 79] {
            assert!(b.clips.iter().any(|x| x.request == r), "request {r}");
        }
        // Re:CoM's 75 slots unchanged.
        let a0 = crate::ann::Ann::parse(&inp.ann).unwrap();
        let a1 = crate::ann::Ann::parse(&b.ann).unwrap();
        for i in 0..SPEC.recom_slots {
            assert_eq!(a0.slots[i], a1.slots[i], "slot {i}");
        }
        // ASD: Re:CoM's record kept, one per KH1 row (strike, spin, bash) on id 0, on `lf_wep0`; the tail moved.
        assert_eq!(&b.asd[0xBC..0x15C], &inp.asd[0xBC..0x15C]);
        assert_eq!(u32::from_le_bytes(b.asd[0x10..0x14].try_into().unwrap()), 1 + 3);
        assert_eq!(&b.asd[0x15C + 3 * 0xA0..], &inp.asd[0x15C..]);
        let wins: Vec<(u32, u32, u32)> = (1..4)
            .map(|i| &b.asd[0xBC + i * 0xA0..0xBC + (i + 1) * 0xA0])
            .map(|r| (u32::from_le_bytes(r[0x5C..0x60].try_into().unwrap()) / 50, u32::from_le_bytes(r[0x60..0x64].try_into().unwrap()) / 50, u32::from_le_bytes(r[0x64..0x68].try_into().unwrap())))
            .collect();
        assert_eq!(wins, vec![(10, 24, 0), (44, 80, 0), (120, 140, 0)]);
        assert!(b.records.iter().all(|r| r.contains("lf_wep0") && r.contains("r 1.00 m")), "{:?}", b.records);
        // SES: Re:CoM's entries kept.
        let s0 = crate::ses::parse(&inp.ses).unwrap();
        let s1 = crate::ses::parse(&b.ses).unwrap();
        for (a, b) in s0.iter().zip(&s1) {
            assert_eq!((a.name(), &a.events), (b.name(), &b.events), "Re:CoM's entry kept");
        }
        // The guard rows: the shield's front (type 1, priority 99, recoil 110 never played, 70°, mode 3) and the spin's
        // (type 1, priority 100, all round, on 208's slot 2 [0, 116)); the attacks' rows priority 100, recoil 110.
        assert!(b.manifest.contains(r#"{"name": "front", "row": "0xec", "type": 1, "priority": 99, "recoil": 110, "slot": 0, "turn": 0.0, "cos": 0.34202, "windows": []}"#), "{}", b.manifest);
        assert!(b.manifest.contains(r#"{"name": "spin", "row": "0x23c", "type": 1, "priority": 100, "recoil": 110, "slot": 2, "turn": 0.0, "cos": -1.0, "windows": [{"request": 208, "on": [[0.0, 116.0]]}]}"#));
        assert!(b.manifest.contains(r#"{"name": "strike", "request": 209, "recoil": 110, "rows": ["0x15c"]"#));
        assert!(b.manifest.contains("\"format\": \"kh1-enemy 1\"") && b.manifest.contains("\"enemy\": \"defender\""));
        assert_eq!(b.parts.len(), 14);
        std::fs::write(std::env::temp_dir().join("kh1-defender-test.json"), &b.manifest).unwrap();
    }
}
