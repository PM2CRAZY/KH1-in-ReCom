//! M11: KH1's Red Nocturne in Re:CoM's battles (`docs/recom-m11-enemies.md` § The Red Nocturne): where its numbers sit
//! in the player's `ex_2110.bd` (`xa_ex_2110.mdls` part 5) and its two children (part 3: the Fire `ex_2110_00.bd`, the
//! big Fire `ex_2110_01.bd`), and which Re:CoM files it extends (`ENEMY/0003`). The step itself is `crate::enemy`.
//!
//! The offsets are the scripts' own (Checked (files): the player's scripts disassembled with `tools/kh1-ai-script.py`,
//! the parent's code from `0x308`, the children's from `0x18`; the routine each sits in is named in
//! `kh1_core::rednocturne`). `walk_speed` / `run_speed` are the stats block's defaults (the flier never walks).
//!
//! Re:CoM has no `EN0003.SES` (Checked (files + exe): `Recom.hed` holds no `ENEMY/0003/EN0003.SES`, and the exe's
//! resource names for enemy 3 list `.RTM` / `.ANN` / `.EET` / `.ASD` / `.SND` but no `.SES`, so Re:CoM never loads one
//! for it): its clips play no motion sounds, and the step writes none ([`crate::enemy::NO_SES`]). Its `.ASD` holds no
//! record (its attacks are code shots) and the KH1 Red Nocturne makes no attack row of its own (its rows are its
//! children's): the `.ASD` is not rewritten.

use crate::enemy::{AttackSpec, Spec};

/// Where each constant is pushed in `ex_2110.bd` (float or integer); a name with several offsets must read the same
/// at each.
pub const CONSTS: &[(&str, &[usize])] = &[
    ("walk_speed", &[]),
    ("run_speed", &[]),
    // Engaged `0x384`: within 500 `0x46A`, within 700 `0x444` (the same), farther `0x41E`; the hit state `0x32C`.
    ("close_range", &[0x3D2]),
    ("mid_range", &[0x3EC]),
    ("near_fire_chance", &[0x44E, 0x474]),
    ("far_bigfire_chance", &[0x428]),
    ("hit_close_range", &[0x330]),
    // `0xA52`: the idle search (`0x1328(&wp30, 0, 1000, 2)`).
    ("idle_search", &[0xA70]),
    // The casts' requests (`sys 13(actor, r, blend, start)`): 200 / 202 at blend 8 (202 from frame 8), the casts queued
    // at blend 0; the idle 220 after them.
    ("cast_blend", &[0x57E, 0x4B8]),
    // The blend the host gives the motion a reaction queues (`Engine::attack_blend`): the scripts' 8.
    ("attack_blend", &[0x57E, 0x4B8]),
    ("cast_start", &[0x584]),
    ("bigfire_start", &[0x4BE]),
    ("cast_queue_blend", &[0x596, 0x4D0]),
    ("idle_blend", &[0x5DA, 0x4EC]),
    ("idle_start", &[0x5E0, 0x4F2]),
    // The −1 that turns a turn or a height around.
    ("flip", &[0x548, 0x6D8, 0x882, 0x97A]),
    // The strafe and Fire `0x4FC`: 5° a frame, the side by the last hover's draw (< 0.5: −90°), 8.4 a frame.
    ("strafe_turn", &[0x4FC]),
    ("strafe_side_chance", &[0x516]),
    ("strafe_left", &[0x528]),
    ("strafe_right", &[0x538]),
    ("strafe_speed", &[0x5C0]),
    // The flee `0x5EA`: [500, 600) away, `0xABC(217, point, 14, 100, 0, 300, [120, 180))`.
    ("flee_min", &[0x5EE]),
    ("flee_max", &[0x5F4]),
    ("flee_speed", &[0x614]),
    ("flee_stop", &[0x61A]),
    ("flee_up", &[0x620]),
    ("flee_high", &[0x626]),
    ("flee_time_min", &[0x62C]),
    ("flee_time_max", &[0x632]),
    // The approach `0x652`: `0xABC(2, target, 8.4, 500, 100, 300, 240)`.
    ("approach_speed", &[0x662]),
    ("approach_stop", &[0x668]),
    ("approach_up", &[0x66E]),
    ("approach_high", &[0x674]),
    ("approach_time", &[0x67A]),
    // The circling hover `0x686`: 2° a frame, 80 % −90°, 90 over the floor its mid height, [90, 120) frames, request 2 at
    // blend 8, 3.64 a frame, 0.2 up / down, the floor probed 20 above the target, [15, 30) its low height, stuck 0.84.
    ("hover_turn", &[0x686]),
    ("hover_side_chance", &[0x6A6]),
    ("hover_left", &[0x6B8]),
    ("hover_right", &[0x6C8]),
    ("hover_mid", &[0x6EE, 0x856]),
    ("hover_time_min", &[0x74E]),
    ("hover_time_max", &[0x754]),
    ("hover_blend", &[0x77E]),
    ("hover_speed", &[0x796]),
    ("hover_sink", &[0x7A0]),
    ("floor_probe", &[0x7C4, 0x1484]),
    ("hover_low_min", &[0x7FA]),
    ("hover_low_max", &[0x800]),
    ("hover_stuck", &[0x8C0]),
    // The idle leg `0x93E`: 5° a frame, 0.3 down (up below 150 over the floor), [150, 180) frames, 219 at blend 8, 8 a
    // frame.
    ("wander_turn", &[0x94A]),
    ("wander_sink", &[0x954]),
    ("wander_low", &[0x964]),
    ("wander_min", &[0x988]),
    ("wander_max", &[0x98E]),
    ("wander_blend", &[0x9B8]),
    ("wander_speed", &[0x9CC]),
    // `0xABC`'s own: the blend 8, "no motion" 999, the stuck factor 0.3; `0xFBA`'s 0.5.
    ("fly_blend", &[0xB5E]),
    ("fly_no_motion", &[0xB48]),
    ("fly_stuck", &[0xC3C]),
    ("stuck_half", &[0xFE0]),
    // `0xC94`: the stats block's `+0x14` (the flight's easing).
    ("air_accel_set", &[0xCAA]),
    // The notice `0xD1A` (a room-placed one): not in KH1's world 11; 214 at blend 8.
    ("notice_world", &[0xD1E]),
    ("notice_blend", &[0xD46]),
    // `0x15C6`: the request that also ends a cast's wait.
    ("wait_motion", &[0x15D6]),
    // The appearance `0xE94`: the 30-frame fade-in, 69 at blend 0.
    ("appear_fade", &[0xEA2]),
    ("appear_blend", &[0xEDE]),
    // The hit handler `0x1F1E`: its own rows' serials (`0x2078`), the EXP bonus `EXP / 10 + 2`, the dizzy's 1800.
    ("own_row_first", &[0x208C]),
    ("own_row_last", &[0x20A2]),
    ("exp_div", &[0x1F8C]),
    ("exp_add", &[0x1F94]),
    ("dizzy_time", &[0x1FA2, 0x2056]),
    // `sys 94`'s child indices: the Fire (`0x4FC`) and the big Fire (`0x490`).
    ("fire_child", &[0x55A]),
    ("bigfire_child", &[0x494]),
];

/// The Fire child's numbers (`ex_2110_00.bd`, entry `0x6B6`): its wait in 201 (40), its start `sys 19(parent, 0x8001,
/// −0.5)` then 80 up, `0x434(…, turn 1, speed 12)`, `0xE8(…, model 7, life 90, fade 20)`. Checked (files).
pub const FIRE_CONSTS: &[(&str, &[usize])] = &[
    ("fire_wait", &[0x712]),
    ("fire_t", &[0x76A]),
    ("fire_up", &[0x780]),
    ("fire_turn", &[0x796]),
    ("fire_speed", &[0x79C]),
    ("fire_life", &[0x7B0]),
    ("fire_fade", &[0x7B6]),
];

/// The Fire child's integer pushes: the charge effect 2052 (`0x6C0`), the parent's 200 / 201 (`0x6D2`, `0x6EE`,
/// `0x736`), the cast effect 2054 (`0x708`), the start's code `0x8001` (`0x764`), its model 7 (`0x7AA`).
pub const FIRE_REQUESTS: &[(u32, usize)] = &[(2052, 0x6C0), (200, 0x6D2), (201, 0x6EE), (2054, 0x708), (201, 0x736), (0x8001, 0x764), (7, 0x7AA)];

/// The big Fire child's numbers (`ex_2110_01.bd`, entry `0x6B6`): its wait in 203 (90), its start `sys 19(parent,
/// 0x8001, −0.5)` then 70 up, `0x434(…, turn 2, speed 22)`, `0xE8(…, model 11, life 90, fade 20)`. Checked (files).
pub const BIGFIRE_CONSTS: &[(&str, &[usize])] = &[
    ("bigfire_wait", &[0x712]),
    ("bigfire_t", &[0x77A]),
    ("bigfire_up", &[0x790]),
    ("bigfire_turn", &[0x7A6]),
    ("bigfire_speed", &[0x7AC]),
    ("bigfire_life", &[0x7C0]),
    ("bigfire_fade", &[0x7C6]),
];

/// The big Fire child's integer pushes: 2053 (`0x6C0`), 202 / 203 (`0x6D2`, `0x6EE`, `0x744`), 2058 (`0x708`), `0x8001`
/// (`0x774`), model 11 (`0x7BA`).
pub const BIGFIRE_REQUESTS: &[(u32, usize)] = &[(2053, 0x6C0), (202, 0x6D2), (203, 0x6EE), (2058, 0x708), (203, 0x744), (0x8001, 0x774), (11, 0x7BA)];

/// The request words the script pushes (bit 24 an action, bit 30 a queue): the behaviour plays exactly these. The
/// leash's 70 is checked but not played.
pub const REQUESTS: &[(u32, usize)] = &[
    (202, 0x4B2), // the big Fire: 202, 203 queued, then the idle 220
    (203, 0x4CA),
    (220, 0x4E6),
    (200, 0x578), // the Fire: 200, 201 queued, then 220
    (201, 0x590),
    (220, 0x5D4),
    (217, 0x60A), // the flee
    (2, 0x652),   // the approach
    (2, 0x778),   // the circling hover
    (219, 0x9B2), // the idle leg
    (214, 0xD40), // the notice, then 220
    (220, 0xD5C),
    (219, 0xE94), // the appearance: 69 (an action), 219 queued
    (0x0100_0045, 0xED8),
    (0x4000_0000, 0xEF4),
    (0x4000_0046, 0x1AE2), // the leash's vanish (not ported)
];

/// No attack of its own: its rows are its children's (`crate::enemy::SHOTS`).
pub const ATTACKS: &[AttackSpec] = &[];

/// The Red Nocturne.
pub const SPEC: Spec = Spec {
    name: "rednocturne",
    recom_id: 3,
    kh1: "xa_ex_2110",
    script: b"ex_2110.bd",
    prefix: "e003_",
    recom_slots: 75,
    asd: (0, 0x6C, 0x6C),
    consts: CONSTS,
    requests: REQUESTS,
    // The LV1 / LV2 stat records (`sys 45` with `0x14` / `0x80`).
    records: &[0x14, 0x80],
    attacks: ATTACKS,
    manifest: "kh1-rednocturne.json",
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enemy::{build, Inputs};

    /// The player's files: `KH1_M11_DIR` (KH1's `xa_*.mdls` / `.mset` and Re:CoM's `ENEMY__NNNN__ENNNNN.*`; no `.SES`
    /// for enemy 3) and `KH1_TEST_DATA` (`btltbl.bin`).
    fn inputs() -> Option<Inputs> {
        crate::enemy::test_inputs(&SPEC, false)
    }

    #[test]
    fn builds_from_the_players_files() {
        let Some(inp) = inputs() else { return };
        assert!(inp.ses.is_empty(), "Re:CoM has no EN0003.SES");
        let b = build(&SPEC, &inp).unwrap();
        let get = |n: &str| b.consts.iter().find(|c| c.0 == n).map(|c| c.1).unwrap_or_else(|| panic!("{n}"));
        println!("clips: {:?}", b.clips.iter().map(|c| (c.request, c.frames, c.root, c.looped, c.source)).collect::<Vec<_>>());
        // The script's numbers (the disassembly's).
        assert_eq!((get("close_range"), get("mid_range"), get("near_fire_chance"), get("far_bigfire_chance"), get("hit_close_range"), get("idle_search")), (500.0, 700.0, 0.4, 0.5, 500.0, 1000.0));
        assert_eq!((get("strafe_turn"), get("strafe_side_chance"), get("strafe_left"), get("strafe_right"), get("strafe_speed"), get("flip")), (5.0, 0.5, -90.0, 90.0, 8.4, -1.0));
        assert_eq!((get("flee_min"), get("flee_max"), get("flee_speed"), get("flee_stop"), get("flee_up"), get("flee_high"), get("flee_time_min"), get("flee_time_max")), (500.0, 600.0, 14.0, 100.0, 0.0, 300.0, 120.0, 180.0));
        assert_eq!((get("approach_speed"), get("approach_stop"), get("approach_up"), get("approach_high"), get("approach_time")), (8.4, 500.0, 100.0, 300.0, 240.0));
        assert_eq!((get("hover_turn"), get("hover_side_chance"), get("hover_mid"), get("hover_time_min"), get("hover_time_max"), get("hover_speed"), get("hover_sink")), (2.0, 0.8, 90.0, 90.0, 120.0, 3.64, 0.2));
        assert_eq!((get("floor_probe"), get("hover_low_min"), get("hover_low_max"), get("hover_stuck"), get("stuck_half")), (20.0, 15.0, 30.0, 0.84, 0.5));
        assert_eq!((get("wander_turn"), get("wander_sink"), get("wander_low"), get("wander_min"), get("wander_max"), get("wander_speed")), (5.0, 0.3, 150.0, 150.0, 180.0, 8.0));
        assert_eq!((get("cast_blend"), get("cast_start"), get("bigfire_start"), get("air_accel_set"), get("appear_fade"), get("wait_motion")), (8.0, 0.0, 8.0, 0.2, 30.0, 6.0));
        assert_eq!((get("own_row_first"), get("own_row_last"), get("exp_div"), get("exp_add"), get("dizzy_time"), get("fire_child"), get("bigfire_child")), (10500.0, 10504.0, 10.0, 2.0, 1800.0, 0.0, 1.0));
        // The children's.
        assert_eq!((get("fire_wait"), get("fire_t"), get("fire_up"), get("fire_turn"), get("fire_speed"), get("fire_life"), get("fire_fade")), (40.0, -0.5, -80.0, 1.0, 12.0, 90.0, 20.0));
        assert_eq!((get("bigfire_wait"), get("bigfire_t"), get("bigfire_up"), get("bigfire_turn"), get("bigfire_speed"), get("bigfire_life"), get("bigfire_fade")), (90.0, -0.5, -70.0, 2.0, 22.0, 90.0, 20.0));
        // The shots: radii 30 / 40, batted 60, Re:CoM's two shots' powers (id 0 / 1, kind 1).
        assert_eq!((get("fire_radius"), get("fire_recom_id"), get("fire_kind"), get("fire_bat_radius")), (30.0, 0.0, 1.0, 60.0));
        assert_eq!((get("bigfire_radius"), get("bigfire_recom_id"), get("bigfire_kind"), get("bigfire_bat_radius")), (40.0, 1.0, 1.0, 60.0));
        assert_eq!((get("walk_speed"), get("turn_rate")), (2.0, std::f32::consts::PI / 16.0));
        // Its own rows carry the serials its handler tests.
        let script = crate::stats::mdls_part(&inp.mdls, 5).unwrap();
        for at in [0xECusize, 0x15C, 0x1CC, 0x23C] {
            let serial = u32::from_le_bytes(script[at + 0x6C..at + 0x70].try_into().unwrap()) as f32;
            assert!(serial >= get("own_row_first") && serial <= get("own_row_last"), "{at:#x}: {serial}");
        }
        // Every request the behaviour plays has a clip; the reactions' too; Re:CoM's 75 slots unchanged.
        for r in [0u16, 2, 69, 0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D, 0x4E, 0x50, 200, 201, 202, 203, 214, 217, 219, 220] {
            assert!(b.clips.iter().any(|x| x.request == r), "request {r}");
        }
        let a0 = crate::ann::Ann::parse(&inp.ann).unwrap();
        let a1 = crate::ann::Ann::parse(&b.ann).unwrap();
        for i in 0..SPEC.recom_slots {
            assert_eq!(a0.slots[i], a1.slots[i], "slot {i}");
        }
        // No new record: the .ASD stays Re:CoM's; no .SES.
        assert_eq!(b.asd, inp.asd);
        assert!(b.ses.is_empty());
        assert!(b.manifest.contains("\"format\": \"kh1-enemy 1\"") && b.manifest.contains("\"enemy\": \"rednocturne\""));
        assert!(b.manifest.replace([' ', '\n'], "").contains("\"attacks\":[]"), "no attack of its own");
        std::fs::write(std::env::temp_dir().join("kh1-rednocturne-test.json"), &b.manifest).unwrap();
        println!("parts {:?}", b.parts);
    }
}
