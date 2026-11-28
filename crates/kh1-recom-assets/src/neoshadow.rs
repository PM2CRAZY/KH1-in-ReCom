//! M11: KH1's Neoshadow in Re:CoM's battles (`docs/recom-m11-enemies.md` § The Neoshadow): where its numbers
//! sit in the player's `ex_2420.bd` (`xa_ex_2420.mdls` part 5) and which Re:CoM files it extends
//! (`ENEMY/0027`). The step itself is `crate::enemy`.
//!
//! The offsets are the script's own (Checked (files): the player's script disassembled with
//! `tools/kh1-ai-script.py`, code from `0x36A`; the routine each sits in is named in `kh1_core::neoshadow`).

use crate::enemy::{AttackSpec, RowSpec, Spec};

/// Where each constant is pushed in `ex_2420.bd` (`push`: float or integer). A name with several offsets must
/// read the same at each; `walk_speed` has none (the stats block's default: the script never sets `+4`).
pub const CONSTS: &[(&str, &[usize])] = &[
    ("walk_speed", &[]),
    ("run_speed", &[0x470C]),
    ("crowd", &[0x72A, 0x4E0, 0xBD2, 0xC18]),
    ("group_min", &[0x746, 0x4FC, 0x41A]),
    ("pair", &[0x7A2, 0x524, 0x476, 0xDA0, 0xE82, 0x20E6, 0x217E]),
    ("gather_max", &[0x8D6, 0x57C, 0x430, 0x555A, 0x55FC, 0x3208, 0xF6E]),
    ("lone", &[0xEBC, 0xDCC, 0xCFC, 0xB84, 0x1B5C]),
    ("search_range", &[0xF3E, 0x54C, 0x3258]),
    ("counter_rise_chance", &[0x460]),
    ("counter_away_min", &[0x494]),
    ("counter_away_max", &[0x49A]),
    ("rounds", &[0xC26, 0x8B4]),
    ("lone_rounds", &[0xB74]),
    ("near_range", &[0xCAC]),
    ("near_claw_chance", &[0xCC4]),
    ("near_away_min", &[0xCE0]),
    ("near_away_max", &[0xCE6]),
    ("regroup_min", &[0xD0E]),
    ("regroup_max", &[0xD14]),
    ("regroup_time", &[0xD38]),
    ("near_rise_chance", &[0xD52]),
    ("mid_range", &[0xD86]),
    ("mid_chance", &[0xDB4]),
    ("mid_rise_chance", &[0xDE6]),
    ("strafe_speed", &[0xE10]),
    ("strafe_near", &[0xE16]),
    ("strafe_time", &[0xE1C]),
    ("approach_timeout", &[0xE30]),
    ("approach_retarget", &[0xE36]),
    ("approach_dist", &[0xE3C]),
    ("far_run_chance", &[0xE6C]),
    ("run_time", &[0xE92]),
    ("run_near", &[0xE98]),
    ("run_steer_speed", &[0xE9E]),
    ("far_near_min", &[0xEDC]),
    ("far_near_max", &[0xEE2]),
    ("claw_blend", &[0x18C4, 0x18DC]),
    ("claw_extra", &[0x18F6]),
    ("claw_face", &[0x1918]),
    ("rise_wait", &[0x15DE]),
    ("rise_near_max", &[0x1642]),
    ("shield_end", &[0x178E, 0x232C]),
    ("idle_blend", &[0x1866, 0x241E, 0x59B4]),
    ("under_time", &[0x55A8, 0x562A]),
    ("under_speed", &[0x55AE]),
    ("under_fast_speed", &[0x5630]),
    ("under_near", &[0x55B4, 0x5636]),
    ("weave_turn", &[0x5686]),
    ("weave_flip", &[0x5690]),
    ("weave_flip_chance", &[0x56A4, 0x581C]),
    ("weave_flip_low", &[0x582A]),
    ("weave_flip_high", &[0x5838]),
    ("stuck_after", &[0x586A]),
    ("stuck_factor", &[0x588C]),
    ("stuck_flip", &[0x58A0]),
    ("stuck_angle_min", &[0x58DC]),
    ("stuck_angle_max", &[0x58E2]),
    ("stuck_dist_min", &[0x58F8]),
    ("stuck_dist_max", &[0x58FE]),
    ("under_blend", &[0x56F8, 0x5960]),
    ("sink_blend", &[0x5AB2]),
    ("sink_extra", &[0x5AE2]),
    ("sink_hide", &[0x5B2A]),
    ("sink_ramp", &[0x5B9A]),
    ("come_up_trim", &[0x59DC]),
    ("run_blend", &[0x5CC6, 0x5D9C]),
    ("run_stuck_after", &[0x5D42]),
    ("run_stuck", &[0x5D60]),
    ("run_end_wait", &[0x5DA4]),
    ("strafe_stuck_after", &[0x536E]),
    ("strafe_side_chance", &[0x53F8]),
    ("strafe_angle", &[0x540A]),
    ("strafe_blend", &[0x541C, 0x543E, 0x5520]),
    ("strafe_stuck_factor", &[0x54E2]),
    ("crouch_blend", &[0x196A, 0x3040]),
    ("crouch_extra", &[0x199C, 0x3072]),
    ("lift_window", &[0x19BE, 0x3094]),
    ("lift_height", &[0x1A50, 0x3126]),
    ("lift_speed", &[0x1A94, 0x316A]),
    ("launch_wait", &[0x329C]),
    ("dive_blend", &[0x3348]),
    ("dive_row_after", &[0x3356]),
    ("dive_cycles", &[0x33AA]),
    ("dive_reaim", &[0x33E0]),
    ("dive_near", &[0x3418]),
    ("dive_level", &[0x3462]),
    ("dive_speed", &[0x348C]),
    ("dive_stuck_after", &[0x34B6]),
    ("dive_stuck", &[0x34D4]),
    ("dive_accel_after", &[0x3566]),
    ("land_trim", &[0x35E8]),
    ("aim_above", &[0x369A, 0x36B4]),
    ("aim_factor", &[0x36E0]),
    ("aim_min", &[0x36EE]),
    ("aim_far", &[0x36FC]),
    ("home_height", &[0x3736]),
    ("home_time", &[0x3752]),
    ("form_wait", &[0x20C0]),
    ("form_warp_wait", &[0x21A6]),
    ("form_rise_after", &[0x222C]),
    ("layout_low", &[0x2464]),
    ("layout_high", &[0x248C]),
    ("slot_draw_min", &[0x2890]),
    ("slot_draw_max", &[0x2896]),
    ("special_wait_min", &[0xBA2]),
    ("special_wait_max", &[0xBA8]),
    ("grab_follow", &[0x1BCC]),
    // The grab's hold: len(208) − 2 (`0x1DAE`), let go with 30 left (`0x1DF6`).
    ("grab_trim", &[0x1DAE]),
    ("grab_let_go", &[0x1DF6]),
    ("grab_range", &[0x1B70]),
    ("grab_wait", &[0x1B86]),
    ("miss_depth", &[0x1F92]),
    ("special_x", &[0xA32]),
    ("special_y", &[0xA38]),
    ("special_z", &[0xA3E]),
    ("special_rise_wait", &[0xA5A]),
    ("lone_x", &[0xB0A]),
    ("lone_y", &[0xB10]),
    ("lone_z", &[0xB16]),
    ("call_blend", &[0x12B4]),
    ("call_extra", &[0x12E6]),
    ("call_window", &[0x1308]),
    ("called_wait", &[0xFC8]),
    ("called_hold", &[0x1120]),
    ("vanish_fade", &[0x64E, 0x658]),
    ("stuck_x", &[0x58AE]),
    ("stuck_y", &[0x58B4]),
    ("stuck_z", &[0x58BA]),
    ("slot_0_0_x", &[0x24DC]),
    ("slot_0_0_y", &[0x24E2]),
    ("slot_0_0_z", &[0x24E8]),
    ("slot_0_1_x", &[0x24FE]),
    ("slot_0_1_y", &[0x2504]),
    ("slot_0_1_z", &[0x250A]),
    ("slot_0_2_x", &[0x2520]),
    ("slot_0_2_y", &[0x2526]),
    ("slot_0_2_z", &[0x252C]),
    ("slot_0_3_x", &[0x2542]),
    ("slot_0_3_y", &[0x2548]),
    ("slot_0_3_z", &[0x254E]),
    ("slot_0_4_x", &[0x2564]),
    ("slot_0_4_y", &[0x256A]),
    ("slot_0_4_z", &[0x2570]),
    ("slot_0_5_x", &[0x2586]),
    ("slot_0_5_y", &[0x258C]),
    ("slot_0_5_z", &[0x2592]),
    ("slot_0_6_x", &[0x25A8]),
    ("slot_0_6_y", &[0x25AE]),
    ("slot_0_6_z", &[0x25B4]),
    ("slot_0_7_x", &[0x25CA]),
    ("slot_0_7_y", &[0x25D0]),
    ("slot_0_7_z", &[0x25D6]),
    ("slot_0_8_x", &[0x25EC]),
    ("slot_0_8_y", &[0x25F2]),
    ("slot_0_8_z", &[0x25F8]),
    ("slot_1_0_x", &[0x2628]),
    ("slot_1_0_y", &[0x262E]),
    ("slot_1_0_z", &[0x2634]),
    ("slot_1_1_x", &[0x264A]),
    ("slot_1_1_y", &[0x2650]),
    ("slot_1_1_z", &[0x2656]),
    ("slot_1_2_x", &[0x266C]),
    ("slot_1_2_y", &[0x2672]),
    ("slot_1_2_z", &[0x2678]),
    ("slot_1_3_x", &[0x268E]),
    ("slot_1_3_y", &[0x2694]),
    ("slot_1_3_z", &[0x269A]),
    ("slot_1_4_x", &[0x26B0]),
    ("slot_1_4_y", &[0x26B6]),
    ("slot_1_4_z", &[0x26BC]),
    ("slot_1_5_x", &[0x26D2]),
    ("slot_1_5_y", &[0x26D8]),
    ("slot_1_5_z", &[0x26DE]),
    ("slot_1_6_x", &[0x26F4]),
    ("slot_1_6_y", &[0x26FA]),
    ("slot_1_6_z", &[0x2700]),
    ("slot_1_7_x", &[0x2716]),
    ("slot_1_7_y", &[0x271C]),
    ("slot_1_7_z", &[0x2722]),
    ("slot_1_8_x", &[0x2738]),
    ("slot_1_8_y", &[0x273E]),
    ("slot_1_8_z", &[0x2744]),
    ("slot_2_0_x", &[0x275E]),
    ("slot_2_0_y", &[0x2764]),
    ("slot_2_0_z", &[0x276A]),
    ("slot_2_1_x", &[0x2780]),
    ("slot_2_1_y", &[0x2786]),
    ("slot_2_1_z", &[0x278C]),
    ("slot_2_2_x", &[0x27A2]),
    ("slot_2_2_y", &[0x27A8]),
    ("slot_2_2_z", &[0x27AE]),
    ("slot_2_3_x", &[0x27C4]),
    ("slot_2_3_y", &[0x27CA]),
    ("slot_2_3_z", &[0x27D0]),
    ("slot_2_4_x", &[0x27E6]),
    ("slot_2_4_y", &[0x27EC]),
    ("slot_2_4_z", &[0x27F2]),
    ("slot_2_5_x", &[0x2808]),
    ("slot_2_5_y", &[0x280E]),
    ("slot_2_5_z", &[0x2814]),
    ("slot_2_6_x", &[0x282A]),
    ("slot_2_6_y", &[0x2830]),
    ("slot_2_6_z", &[0x2836]),
    ("slot_2_7_x", &[0x284C]),
    ("slot_2_7_y", &[0x2852]),
    ("slot_2_7_z", &[0x2858]),
    ("slot_2_8_x", &[0x286E]),
    ("slot_2_8_y", &[0x2874]),
    ("slot_2_8_z", &[0x287A]),
    // The actions' idle queue blend the host gives after a reaction (every port's).
    ("attack_blend", &[0x1866, 0x241E]),
];

/// The request words the script pushes (bit 24 an action, bit 30 a queue): the behaviour plays exactly these.
pub const REQUESTS: &[(u32, usize)] = &[
    (72, 0x5318),          // its death (`0x5304`)
    (200, 0x5416),         // the strafe's sides (`0x5354`)
    (201, 0x5438),
    (202, 0x18BE),         // the claw (`0x1878`)
    (203, 0x1964),         // the crouch and the air (`0x1950`, `0x3036`)
    (204, 0x197C),
    (203, 0x303A),
    (204, 0x3052),
    (205, 0x3342),         // the dive (`0x3326`), its landing, 6 queued
    (206, 0x35B0),
    (6, 0x35C8),
    (207, 0x1720),         // the rising slash (`0x1568`, `0x1FB0`)
    (207, 0x22F2),
    (0, 0x1860),           // the idle after it
    (0x0100_00C8, 0x1CD4), // KH1's grab from below (`0x1AF6`): Sora's request 200 of the player set (the held clip)
    (0x4000_0000, 0x1CEC), // and the lock's release queued
    (208, 0x1D54),         // the grab and its miss
    (209, 0x1EA4),
    (210, 0x12AE),         // the call (`0x11FA`)
    (211, 0x10DE),         // a called one's rise (`0xFC8`): 211, 212, the action 213 with the idle queued
    (212, 0x10FE),
    (0x0100_00D5, 0x11B0),
    (0x4000_0000, 0x11C8),
    (216, 0x5AAC),         // sinking (`0x5A98`), the underground motion, coming up (`0x596A`)
    (217, 0x5AC4),
    (217, 0x55BA),
    (2, 0x563C),           // `0x55D2`'s travel motion (run)
    (2, 0xEA4),            // the run at Sora (`0x5BE8(…, 2)`)
    (218, 0x5996),
];

/// The rows (script offsets) and the Re:CoM attack ids they take their power from (decision M10: Re:CoM's attack
/// power per attack; owner 2026-11-02: "the Re:CoM attack of the same kind"). Re:CoM's Neoshadow has two
/// (Checked (files): `EN0027.ASD`, 3 records): id 0 on `e027_050` (its claw, state 2 `0x140190330`: both hands,
/// 0.35 m) and id 1 on `e027_052` (its state 3 `0x140190380`: it sinks (`e027_051`), comes up 2 m from Sora at a
/// random angle and strikes with its body, 1.0 m on `m_ex420_sk`). The matcher pairs none of KH1's attacks with
/// them (Re:CoM's clips are not edits of KH1's 202 / 207 / 205). By the kind of attack: KH1's claw 202 (both
/// hands, kind 1, power 4) takes Re:CoM's claw (id 0); its rising slash 207 out of the floor under Sora (kind 1,
/// power 8) Re:CoM's coming-up strike (id 1); its dive 205 (kind 4, power 6), which Re:CoM's Neoshadow lacks, the
/// stronger of the two (id 1; Assumption, owner question).
const CLAW_L: RowSpec = RowSpec { at: 0x1D0, recom_id: 0 };
const CLAW_R: RowSpec = RowSpec { at: 0x240, recom_id: 0 };
const RISE: RowSpec = RowSpec { at: 0x160, recom_id: 1 };
const DIVE: RowSpec = RowSpec { at: 0xF0, recom_id: 1 };

/// `kh1_core::neoshadow::Attack::ALL`'s order.
pub const ATTACKS: &[AttackSpec] = &[
    AttackSpec { name: "claw", request: 202, rows: &[CLAW_L, CLAW_R] },
    AttackSpec { name: "rise slash", request: 207, rows: &[RISE] },
    AttackSpec { name: "dive", request: 205, rows: &[DIVE] },
    // KH1's grab from below 208: no KH1 row; the held record (`crate::enemy::HOLDS`).
    AttackSpec { name: "grab", request: 208, rows: &[] },
];

/// The Neoshadow.
pub const SPEC: Spec = Spec {
    name: "neoshadow",
    recom_id: 27,
    kh1: "xa_ex_2420",
    script: b"ex_2420.bd",
    prefix: "e027_",
    recom_slots: 73,
    asd: (3, 0x6C, 0x24C),
    consts: CONSTS,
    requests: REQUESTS,
    // Its one stat record (`0x4700` `sys 45(wp, 0x14)`).
    records: &[0x14],
    attacks: ATTACKS,
    manifest: "kh1-neoshadow.json",
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
        assert_eq!((get("walk_speed"), get("run_speed"), get("weight")), (2.0, 12.0, 1.0));
        assert_eq!((get("crowd"), get("group_min"), get("pair"), get("gather_max"), get("lone")), (5.0, 3.0, 2.0, 4.0, 1.0));
        assert_eq!((get("near_range"), get("mid_range"), get("far_run_chance"), get("run_time"), get("run_steer_speed")), (500.0, 900.0, 0.4, 240.0, 12.0));
        assert_eq!((get("lift_window"), get("lift_height"), get("lift_speed"), get("dive_cycles"), get("dive_speed"), get("dive_near")), (14.0, 120.0, 20.0, 9.0, 80.0, 250.0));
        assert_eq!((get("aim_above"), get("aim_factor"), get("aim_min"), get("aim_far"), get("dive_accel_after")), (80.0, 1.4, 500.0, 800.0, 0.4));
        assert_eq!((get("sink_hide"), get("sink_extra"), get("sink_ramp"), get("come_up_trim"), get("shield_end")), (40.0, 6.0, 20.0, 2.0, 70.0));
        assert_eq!((get("under_time"), get("under_speed"), get("under_fast_speed"), get("under_near"), get("weave_turn")), (180.0, 12.0, 30.0, 50.0, 4.0));
        assert_eq!((get("special_x"), get("special_y"), get("special_z"), get("lone_x"), get("lone_y"), get("lone_z")), (9000.0, 5300.0, 100.0, 8900.0, 5300.0, 100.0));
        assert_eq!((get("stuck_x"), get("stuck_y"), get("stuck_z"), get("home_height"), get("layout_low"), get("layout_high")), (8700.0, 5300.0, 100.0, 5380.0, 8250.0, 9200.0));
        assert_eq!((get("slot_0_0_x"), get("slot_1_8_x"), get("slot_2_0_x"), get("slot_2_0_y"), get("slot_2_8_z")), (8100.0, 10000.0, 8800.0, 5480.0, -350.0));
        // The clips: all KH1's but one (67, an exact bake of Re:CoM's `e027_071`); no launch motions.
        let c = |r: u16| b.clips.iter().find(|c| c.request == r).unwrap().clone();
        assert_eq!((c(202).frames, c(205).frames, c(207).frames, c(216).frames, c(218).frames, c(210).frames), (74, 40, 120, 60, 70, 240));
        assert!(c(205).looped && c(217).looped && c(204).looped && !c(207).looped && c(202).root);
        for r in [0, 1, 2, 6, 7, 72, 73, 74, 75, 76, 77, 78, 79, 200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 215, 216, 217, 218] {
            assert!(b.clips.iter().any(|x| x.request == r), "request {r}");
        }
        assert!(!b.clips.iter().any(|x| x.request == 0x52 || x.request == 110), "no launch, no recoil 110");
        assert_eq!(c(67).source, "recom");
        // Re:CoM's 73 slots unchanged.
        let a0 = crate::ann::Ann::parse(&inp.ann).unwrap();
        let a1 = crate::ann::Ann::parse(&b.ann).unwrap();
        for i in 0..SPEC.recom_slots {
            assert_eq!(a0.slots[i], a1.slots[i], "slot {i}");
        }
        // ASD: Re:CoM's 3 records kept, one per row's hit window (the claw's two hands, the rising slash, the
        // dive), the grab's held record, the tail moved.
        assert_eq!(&b.asd[0x6C..0x24C], &inp.asd[0x6C..0x24C]);
        assert_eq!(u32::from_le_bytes(b.asd[0x10..0x14].try_into().unwrap()), 3 + 5);
        assert_eq!(&b.asd[0x24C + 5 * 0xA0..], &inp.asd[0x24C..]);
        let rec = |name: &str| -> Vec<(u32, u32, u32)> {
            (3..8)
                .map(|i| &b.asd[0x6C + i * 0xA0..0x6C + (i + 1) * 0xA0])
                .filter(|r| r[0x40..0x50].starts_with(name.as_bytes()))
                .map(|r| (u32::from_le_bytes(r[0x5C..0x60].try_into().unwrap()), u32::from_le_bytes(r[0x60..0x64].try_into().unwrap()), u32::from_le_bytes(r[0x64..0x68].try_into().unwrap())))
                .collect()
        };
        assert_eq!(rec(&c(202).name), vec![(40 * 50, 54 * 50, 0), (52 * 50, 68 * 50, 0)], "the claw's hands on id 0");
        assert_eq!(rec(&c(207).name), vec![(26 * 50, 50 * 50, 1)], "the rising slash on id 1");
        assert_eq!(rec(&c(205).name), vec![(0, 40 * 50, 1)], "the dive on id 1");
        // The grab's held record: Re:CoM's come-up strike record (id 1, `m_ex420_sk`, 1.0 m) on 208's clip, frames
        // 0..2, the held kind's index 21, no hit effect.
        assert_eq!(rec(&c(208).name), vec![(0, 2 * 50, 1)], "the grab on id 1's record");
        let g = &b.asd[0x6C + 7 * 0xA0..0x6C + 8 * 0xA0];
        let tmpl = &inp.asd[0x6C + 2 * 0xA0..0x6C + 3 * 0xA0];
        assert_eq!(&g[..0x40], &tmpl[..0x40], "the template's bone, place and shape");
        assert_eq!((&g[0x50..0x5C], u32::from_le_bytes(g[0x68..0x6C].try_into().unwrap())), (&tmpl[0x50..0x5C], 21));
        assert_eq!((u32::from_le_bytes(g[0x6C..0x70].try_into().unwrap()), &g[0x70..0x80]), (0, &[0u8; 16][..]));
        assert_eq!((get("grab_trim"), get("grab_let_go"), get("grab_record_frames"), get("grab_kind_index")), (2.0, 30.0, 2.0, 21.0));
        assert!(b.manifest.contains(r#"{"name": "grab", "request": 208, "recoil": 110, "rows": [], "clash": []}"#), "{}", b.manifest);
        // SES: Re:CoM's entries kept, sounds on the attack clips.
        let s0 = crate::ses::parse(&inp.ses).unwrap();
        let s1 = crate::ses::parse(&b.ses).unwrap();
        for (a, b) in s0.iter().zip(&s1) {
            assert_eq!((a.name(), &a.events), (b.name(), &b.events), "Re:CoM's entry kept");
        }
        // The shield: row 0x80, type 4, priority 100, all round (cos −1), held (mode 3): no windows.
        assert!(b.manifest.contains(r#"{"name": "shield", "row": "0x80", "type": 4, "priority": 100, "recoil": 110, "slot": 0, "turn": 0.0, "cos": -1.0, "windows": []}"#), "{}", b.manifest);
        assert!(b.manifest.contains("\"format\": \"kh1-enemy 1\"") && b.manifest.contains("\"enemy\": \"neoshadow\""));
        assert!(b.manifest.contains(r#"{"name": "claw", "request": 202, "recoil": 215"#) && b.manifest.contains(r#"{"name": "dive", "request": 205, "recoil": 110"#));
        assert!(!b.parts.is_empty());
        std::fs::write(std::env::temp_dir().join("kh1-neoshadow-test.json"), &b.manifest).unwrap();
        println!("parts {:?}", b.parts);
        println!("{}", b.records.join("\n"));
    }
}
