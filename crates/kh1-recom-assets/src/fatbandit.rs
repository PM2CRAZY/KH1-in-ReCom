//! M11: KH1's Fat Bandit in Re:CoM's battles (`docs/recom-m11-enemies.md` § The Fat Bandit): where its numbers sit
//! in the player's `ex_2060.bd` (`xa_ex_2060.mdls` part 5) and its two children (part 3: the fireball `ex_2060_00.bd`,
//! the flame `ex_2060_01.bd`), and which Re:CoM files it extends (`ENEMY/0010`). The step itself is `crate::enemy`.
//!
//! The offsets are the scripts' own (Checked (files): the player's scripts disassembled with
//! `tools/kh1-ai-script.py`, the parent's code from `0x4C0`, the children's from `0x18`; the routine each sits in is
//! named in `kh1_core::fatbandit`).

use crate::enemy::{AttackSpec, RowSpec, Spec};

/// Where each constant is pushed in `ex_2060.bd` (float or integer); a name with several offsets must read the same
/// at each.
pub const CONSTS: &[(&str, &[usize])] = &[
    // Init `0x27A0` / `0x280C`: the stats block's walk / run (`+4` / `+8`), the turn rate (`sys 125(wp0, 10)`; the
    // hit handler `0x29BC` sets it again).
    ("walk_speed", &[0x27EC]),
    ("run_speed", &[0x27FE]),
    ("turn_deg", &[0x281C, 0x29C4, 0x37C2]),
    // `0x3810`: the head look's cone (`+0x1C` 45° to start looking, `+0x20` 60° once looking), its pitch limits
    // (`+0x2C` 20° up, `+0x30` 15° down) and yaw limits (`+0x34` / `+0x38` 90°): `sys 143` gives the fireball the
    // head's direction (`0x1402BAE10` → `+0x450`, turned by `0x1402A2510` / `0x1402A2980`).
    ("look_cone_in", &[0x381A]),
    ("look_cone_out", &[0x382E]),
    ("look_up", &[0x3842]),
    ("look_down", &[0x3856]),
    ("look_left", &[0x386A]),
    ("look_right", &[0x387E]),
    // The supervisor `0x24E8`: a room-spawned Fat Bandit (spawn type ≠ 0) finds Sora within 1500 while idle.
    ("idle_search", &[0x26B6]),
    // The lost state `0x83E`: the search reaches 10000.
    ("lost_search", &[0x954]),
    // The wander `0xEDA`: points [200, 400) and [400, 500) about where it stands, each walked to (walk 1 at 2 a frame,
    // the way taken every 4 frames, 300 frames, within 60); 70 %: done (40 % of all with an idle of [120, 180)); else
    // again.
    ("wander_inner_min", &[0xEDE]),
    ("wander_inner_max", &[0xEE4]),
    ("wander_outer_min", &[0xEFE]),
    ("wander_outer_max", &[0xF04]),
    ("wander_speed", &[0xF24, 0xF4A]),
    ("wander_time", &[0xF2A, 0xF50]),
    ("wander_reaim", &[0xF30, 0xF56]),
    ("wander_stop", &[0xF36, 0xF5C]),
    ("wander_done_chance", &[0xF70]),
    ("wander_idle_chance", &[0xF82]),
    ("idle_blend", &[0xF9A, 0xA9A, 0x16E2, 0x17D6, 0x1600]),
    ("wander_idle_min", &[0xFA2]),
    ("wander_idle_max", &[0xFA8]),
    // The engaged round `0xA56`: close (< 300) `0xC5C`, mid (< 600) `0xC08`, far `0xBC8`.
    ("close_range", &[0xAB2]),
    ("mid_range", &[0xAD8]),
    // Close: 70 % the belly, else the breath; then 40 % two fireballs and a flame.
    ("close_belly_chance", &[0xC66]),
    ("close_fire_chance", &[0xC94]),
    // Mid: 30 % the volley then the approach (180 frames, to 300), else the approach, the breath, the belly.
    ("mid_volley_chance", &[0xC12]),
    ("mid_approach_time", &[0xC24, 0xC38]),
    ("mid_approach_dist", &[0xC2A, 0xC3E]),
    // Far: 80 % the approach (180 frames, to 600) and the breath, else the volley.
    ("far_breath_chance", &[0xBD2]),
    ("far_approach_time", &[0xBE0]),
    ("far_approach_dist", &[0xBE6]),
    // The approach `0x35E0` → `0x36BE`: at the target, run 2 at 6 a frame, the way taken every 6 frames (Agrabah's
    // rooms but 1, 2, 5: walk 1 at 2, every 4); the turn rate 5° a frame while it goes.
    ("approach_speed", &[0x3674]),
    ("approach_reaim", &[0x367E]),
    ("approach_walk_speed", &[0x36A0]),
    ("approach_walk_reaim", &[0x36AA]),
    ("route_turn_deg", &[0x36DA]),
    // The belly `0x1750`: its rows, 200 (blend 14, a plain request) to its end, the idle (8), 75 frames.
    ("belly_blend", &[0x17BA]),
    ("belly_wait", &[0x17E4]),
    // The breath `0x1308`: 205 (blend 8), 68 frames, the front row removed, 90 frames, 207 (blend 8), 8 + 60
    // frames, the flame.
    ("windup_blend", &[0x1336]),
    ("windup_open", &[0x1344]),
    ("windup_hold", &[0x1352]),
    ("breath_blend", &[0x1366]),
    ("breath_wait", &[0x1374]),
    ("breath_aim_wait", &[0x13C8]),
    // The flame `0x14B8`: at most 140 frames while 207 plays; then to 207's end and the idle (8).
    ("flame_time", &[0x14B8]),
    // Two fireballs and a flame `0x129A`: 18 frames, 207 from its frame 40 (blend 18), 38 frames turning, the
    // flame. The quick flame `0x12DC`: 207 from 40 (blend 18), 18 frames turning, the flame.
    ("combo_gap", &[0x12AA]),
    ("combo_blend", &[0x12BE]),
    ("combo_from", &[0x12C4]),
    ("combo_wait", &[0x12CC]),
    ("quick_blend", &[0x12EA]),
    ("quick_from", &[0x12F0]),
    ("quick_wait", &[0x12F8]),
    // One fireball `0x16FC`: 201 (blend 8, from 0), 64 frames turning, the fireball (`sys 94`, child 0).
    ("fire_blend", &[0x1718]),
    ("fire_from", &[0x171E]),
    ("fire_wait", &[0x1726]),
    // The volley `0x1610`: one fireball, 6 frames; twice while 201 plays: 201 from 12 (blend 4), 48 frames turning,
    // the fireball, 6 frames; to 201's end, the idle (8), 60 frames.
    ("volley_count", &[0x1610]),
    ("volley_gap", &[0x1632, 0x16A6]),
    ("volley_blend", &[0x1670]),
    ("volley_from", &[0x1676]),
    ("volley_wait", &[0x167E]),
    ("volley_rest", &[0x16F0]),
    // The appearance `0x20F8`: 69 (blend 0), the 30-frame fade-in, 30 frames, the idle queued (0), paused to 69's end.
    ("appear_blend", &[0x2106]),
    ("appear_fade", &[0x2118]),
    ("appear_wait", &[0x213E]),
    ("appear_then_blend", &[0x2164]),
    // The hit handler `0x29BC`: a non-flinch hit on its front (cos > cos 80°) while its front row is down (the breath)
    // staggers it; the counted hits (`wp+0x70`: back to 0 past 3, +1 a flinch / +2 another kind on a run's first hit,
    // +1 on its second).
    ("stagger_front_deg", &[0x2DE0, 0x31CC]),
    ("count_reset", &[0x2F34]),
    ("count_flinch", &[0x2F6E]),
    ("count_other", &[0x2F9A]),
    ("count_second", &[0x2FCA]),
    // The hit state `0x4F2`: 25 % back to Sora as the target; staggered: 208, the EXP bonus max(EXP / 5, 1) 140 above
    // it (`sys 195`); else at most 2 counted hits: 80 % the quick flame, else two fireballs and a flame; more: the
    // count back to 0 and the volley.
    ("hit_sora_chance", &[0x536]),
    ("stagger_lift", &[0x6EC]),
    ("stagger_exp_div", &[0x702, 0x738]),
    ("stagger_exp_min", &[0x70A, 0x71C]),
    ("hit_count_max", &[0x7AA]),
    ("hit_flame_chance", &[0x7C2]),
    // The blend the host gives a reaction's queued motion (the engine's `attack_blend`; KH1's reactions queue with 8,
    // `0x1402B7260`): the attacks' own 8 (the fireball, the wind-up, the breath).
    ("attack_blend", &[0x1718, 0x1336, 0x1366]),
];

/// The children's numbers (`crate::enemy::CHILD_CONSTS`). The fireball `ex_2060_00.bd` (entry `0x5E6`): effect 18 on
/// the parent, its start `sys 19(parent, 4, 2.5)` (bone code 4 at t 2.5), `0x364(…, 0, 45)`: no turn (0) at 45 a
/// frame (`func_418(45, 45)`), `0x18(…, 5, 240, 20)`: its model 5, a 240-frame life, a 20-frame fade before it ends;
/// along the head's direction (`sys 143`). Checked (files).
pub const FIRE_CONSTS: &[(&str, &[usize])] = &[("fire_t", &[0x606]), ("fire_turn", &[0x61E]), ("fire_speed", &[0x624, 0x62A]), ("fire_life", &[0x650]), ("fire_fade", &[0x656])];

/// The fireball's integer pushes the port relies on: its effect 18 (`0x5F0`), its bone code 4 (`0x600`), its model 5
/// (`0x64A`).
pub const FIRE_REQUESTS: &[(u32, usize)] = &[(18, 0x5F0), (4, 0x600), (5, 0x64A)];

/// The flame `ex_2060_01.bd` (entry `0x22A`): an attack object from the parent's row (`0x15C`) in mode 3 (`sys 213(…,
/// 3)`, tested every frame), shape 2 (`func_1D0(…, 2)`: a chain of spheres from `+0x10` (radius `+0xC`) to `+0x40`
/// (radius `+0x3C`), `0x1402B3D50`), its hit-once mask cleared every 60 frames (`+0x50`, `0x1402B3380`); each frame
/// for at most 140 frames while 207 plays: from bone code 0 at t 3, 25 lower, to bone code 0 at t 15, 75 lower, radii
/// 23 to 60; for its first 60 frames the line and the radii grow by t / 60. Checked (files + decomp).
pub const FLAME_CONSTS: &[(&str, &[usize])] = &[
    ("flame_rehit", &[0x264]),
    ("flame_t0", &[0x288, 0x2E2]),
    ("flame_life", &[0x2C8]),
    ("flame_lift0", &[0x2F8]),
    ("flame_t1", &[0x30E]),
    ("flame_lift1", &[0x324]),
    ("flame_r0", &[0x32E]),
    ("flame_r1", &[0x338]),
    ("flame_grow", &[0x392]),
    ("flame_grow_div", &[0x3A4]),
];

/// The flame's integer pushes: shape 2 (`0x248`), mode 3 (`0x256`), bone code 0 (`0x282`, `0x2DC`, `0x308`), the
/// effect 4 (`0x2A4`), the request it lives with (207, `0x50C`).
pub const FLAME_REQUESTS: &[(u32, usize)] = &[(2, 0x248), (3, 0x256), (0, 0x282), (0, 0x2DC), (0, 0x308), (4, 0x2A4), (207, 0x50C)];

/// The request words the script pushes (bit 30 a queue): the behaviour plays exactly these.
pub const REQUESTS: &[(u32, usize)] = &[
    (0x4000_00D0, 0x6B8), // the stagger 208 (the hit state; the defeat 0x31E4)
    (0x4000_00D0, 0x31E4),
    (0, 0xA94), // the engaged round's idle
    (0, 0xF94), // the wander's idle
    (1, 0xF1E), // the wander's walk
    (1, 0xF44),
    (207, 0x12B8), // two fireballs and a flame: 207 from 40
    (207, 0x12E4), // the quick flame
    (205, 0x1330), // the breath's wind-up
    (207, 0x1360), // the breath
    (207, 0x150A), // the flame's test
    (207, 0x15E0),
    (0, 0x15FA),   // the idle after the flame
    (201, 0x1656), // the volley's test, its shots from 12
    (201, 0x166A),
    (0, 0x16DC),
    (201, 0x1712), // one fireball
    (200, 0x17B4), // the belly
    (0, 0x17D0),
    (2, 0x366E), // the approach: run (2), Agrabah's narrow rooms walk (1)
    (1, 0x369A),
    (69, 0x2100), // the appearance, the idle queued
    (0x4000_0000, 0x215E),
    (6, 0x1C8C), // the waits stop when 6 (the fall) plays
];

/// The belly's rows (both hands: bone codes 25 / 24, t 1.3 → 0, radius 60, hit slots 0 / 1) and the Re:CoM attack id
/// they take their power from (decision M10: Re:CoM's attack power per attack; owner 2026-11-02: "the Re:CoM attack of
/// the same kind"). Re:CoM's Fat Bandit has two attacks (Checked (files + decomp)): id 0, its belly `e010_050` (state 2,
/// `0x14018DEE0`; `EN0010.ASD`: both hands, `r_hand` [45, 72) and `l_hand` [75, 115)), and id 1, its fireball (state 3,
/// `0x14018DF20`: a `CBullet` with `0x14017F0E0(enemy, 1, 1, 0)`'s AP; no `.ASD` record). The belly 200 takes id 0.
const BELLY_R: RowSpec = RowSpec { at: 0x31C, recom_id: 0 };
const BELLY_L: RowSpec = RowSpec { at: 0x38C, recom_id: 0 };

/// `kh1_core::fatbandit::Attack::ALL`'s order. The fireballs and the flame are its children's (Re:CoM bullets, the
/// host's `fire_*` / `flame_*` numbers).
pub const ATTACKS: &[AttackSpec] = &[AttackSpec { name: "belly", request: 200, rows: &[BELLY_R, BELLY_L] }];

/// The Fat Bandit.
pub const SPEC: Spec = Spec {
    name: "fatbandit",
    recom_id: 10,
    kh1: "xa_ex_2060",
    script: b"ex_2060.bd",
    prefix: "e010_",
    recom_slots: 74,
    asd: (2, 0x6C, 0x1AC),
    consts: CONSTS,
    requests: REQUESTS,
    // The LV1 / LV2 stat records (`sys 45` with `0x14` / `0x80`).
    records: &[0x14, 0x80],
    attacks: ATTACKS,
    manifest: "kh1-fatbandit.json",
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
        assert_eq!((get("walk_speed"), get("run_speed"), get("turn_deg"), get("idle_search"), get("close_range"), get("mid_range")), (2.0, 6.0, 10.0, 1500.0, 300.0, 600.0));
        assert_eq!((get("close_belly_chance"), get("close_fire_chance"), get("mid_volley_chance"), get("far_breath_chance")), (0.7, 0.4, 0.3, 0.8));
        assert_eq!((get("belly_wait"), get("windup_open"), get("windup_hold"), get("breath_wait"), get("breath_aim_wait"), get("flame_time")), (75.0, 68.0, 90.0, 8.0, 60.0, 140.0));
        assert_eq!((get("fire_wait"), get("volley_count"), get("volley_from"), get("volley_wait"), get("volley_rest"), get("stagger_front_deg")), (64.0, 2.0, 12.0, 48.0, 60.0, 80.0));
        assert_eq!((get("hit_count_max"), get("hit_flame_chance"), get("stagger_exp_div"), get("stagger_exp_min"), get("weight")), (2.0, 0.8, 5.0, 1.0, 1.0));
        // The children's.
        assert_eq!((get("fire_t"), get("fire_turn"), get("fire_speed"), get("fire_life"), get("fire_fade"), get("fire_radius")), (2.5, 0.0, 45.0, 240.0, 20.0, 30.0));
        assert_eq!((get("flame_rehit"), get("flame_t0"), get("flame_t1"), get("flame_lift0"), get("flame_lift1"), get("flame_r0"), get("flame_r1"), get("flame_life")), (60.0, 3.0, 15.0, 25.0, 75.0, 23.0, 60.0, 140.0));
        assert_eq!((get("look_cone_out"), get("look_up"), get("look_down"), get("look_left"), get("fire_recom_id"), get("fire_kind")), (60.0, 20.0, 15.0, 90.0, 1.0, 1.0));
        // The points: the fireball's start on bone 36, the head's and the flame's on bone 39, along the bone's x (t
        // scales the parent → bone translation, 0.2467 m for code 0).
        assert_eq!((get("fire_at_bone"), get("eye_bone"), get("flame0_bone"), get("flame1_bone")), (36.0, 39.0, 39.0, 39.0));
        assert!((get("flame1_x") / get("flame0_x") - 5.0).abs() < 1e-4 && (get("eye_x") / get("flame0_x") - 0.5 / 3.0).abs() < 1e-4);
        assert!(["fire_at_y", "fire_at_z", "flame1_y", "flame1_z"].iter().all(|n| get(n).abs() < 1e-5));
        // The clips: all KH1's; their lengths; 201 with root motion; no clip loops that the script plays once.
        let c = |r: u16| b.clips.iter().find(|c| c.request == r).unwrap().clone();
        assert_eq!((c(69).frames, c(200).frames, c(201).frames, c(202).frames, c(205).frames, c(207).frames, c(208).frames), (102, 158, 108, 52, 180, 264, 268));
        assert!(c(201).root && !c(200).root && !c(207).looped && !c(205).looped && !c(201).looped);
        assert!(b.clips.iter().all(|x| x.source == "kh1"));
        for r in [0, 1, 2, 6, 7, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86] {
            assert!(b.clips.iter().any(|x| x.request == r), "request {r}");
        }
        // Re:CoM's 74 slots unchanged.
        let a0 = crate::ann::Ann::parse(&inp.ann).unwrap();
        let a1 = crate::ann::Ann::parse(&b.ann).unwrap();
        for i in 0..SPEC.recom_slots {
            assert_eq!(a0.slots[i], a1.slots[i], "slot {i}");
        }
        // ASD: Re:CoM's 2 records kept, 4 for the belly's hands (two windows each) on id 0; the tail moved.
        assert_eq!(&b.asd[0x6C..0x1AC], &inp.asd[0x6C..0x1AC]);
        assert_eq!(u32::from_le_bytes(b.asd[0x10..0x14].try_into().unwrap()), 2 + 4);
        assert_eq!(&b.asd[0x1AC + 4 * 0xA0..], &inp.asd[0x1AC..]);
        let wins: Vec<(u32, u32, u32)> = (2..6)
            .map(|i| &b.asd[0x6C + i * 0xA0..0x6C + (i + 1) * 0xA0])
            .map(|r| (u32::from_le_bytes(r[0x5C..0x60].try_into().unwrap()) / 50, u32::from_le_bytes(r[0x60..0x64].try_into().unwrap()) / 50, u32::from_le_bytes(r[0x64..0x68].try_into().unwrap())))
            .collect();
        assert_eq!(wins, vec![(54, 68, 0), (84, 106, 0), (50, 72, 0), (86, 100, 0)]);
        // SES: Re:CoM's entries kept.
        let s0 = crate::ses::parse(&inp.ses).unwrap();
        let s1 = crate::ses::parse(&b.ses).unwrap();
        for (a, b) in s0.iter().zip(&s1) {
            assert_eq!((a.name(), &a.events), (b.name(), &b.events), "Re:CoM's entry kept");
        }
        // The front row: type 1, priority 100, recoil 110 (never played), 70° front, mode 3 (no windows); the belly's
        // rows priority 30, recoil 202 (in the model).
        assert!(b.manifest.contains(r#"{"name": "front", "row": "0xec", "type": 1, "priority": 100, "recoil": 110, "slot": 0, "turn": 0.0, "cos": 0.34202, "windows": []}"#), "{}", b.manifest);
        assert!(b.manifest.contains(r#"{"name": "belly", "request": 200, "recoil": 202, "rows": ["0x31c", "0x38c"]"#));
        assert!(b.manifest.contains("\"format\": \"kh1-enemy 1\"") && b.manifest.contains("\"enemy\": \"fatbandit\""));
        assert_eq!(b.parts.len(), 10);
        std::fs::write(std::env::temp_dir().join("kh1-fatbandit-test.json"), &b.manifest).unwrap();
        println!("{}", b.records.join("\n"));
        let skel = crate::mdl::RecomSkeleton::parse(&inp.mdl).unwrap();
        println!("bones: 36 {}, 39 {}; parts {:?}", skel.bones[36].name, skel.bones[39].name, b.parts.iter().map(|p| skel.bones[p.1].name.clone()).collect::<Vec<_>>());
    }
}
