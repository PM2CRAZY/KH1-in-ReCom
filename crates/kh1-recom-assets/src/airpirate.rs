//! M11: KH1's Air Pirate in Re:CoM's battles (`docs/recom-m11-enemies.md` § The Air Pirate): where its
//! numbers sit in the player's `ex_2210.bd` (`xa_ex_2210.mdls` part 5) and which Re:CoM files it
//! extends (`ENEMY/0022`). The step itself is `crate::enemy`.
//!
//! The offsets are the script's own (Checked (files): the player's script disassembled; the routine
//! each sits in is named in `kh1_core::airpirate`). `walk_speed` / `run_speed` are the stats block's
//! defaults (the script never writes them; the flier never walks).

use crate::enemy::{AttackSpec, RowSpec, Spec};

/// Where each constant is pushed in `ex_2210.bd` (float or integer); a name with several offsets
/// must read the same at each.
pub const CONSTS: &[(&str, &[usize])] = &[
    ("walk_speed", &[]),
    ("run_speed", &[]),
    ("search_range", &[0x574, 0x8E6, 0x1D52]),
    // Blends.
    ("motion_blend", &[0x5EE, 0xBDE, 0xC44, 0xD9E, 0xEB2, 0x1120, 0x21D8, 0x2280, 0x22EC, 0x23B2, 0x2548, 0x25B4, 0x26D4, 0x2764, 0x2812, 0x29C2, 0x2B9C, 0x32C4, 0x3404, 0x3416, 0x3542, 0x3580, 0x363E, 0x369E, 0x377A, 0x37C6, 0x37F6]),
    ("attack_blend", &[0x36DC]),
    ("appear_blend", &[0x1968]),
    ("rest_blend", &[0x94E]),
    ("dive_hit_blend", &[0x29A]),
    ("close_swoop_blend", &[0x100C]),
    ("bank_blend", &[0x2C5E, 0x2CB2, 0x2CDA]),
    // Appearance `0x195A`.
    ("appear_fade", &[0x197A]),
    ("appear_wait", &[0x19A0]),
    // Hit `0x274`.
    ("hit_wait", &[0x2D8]),
    ("hit_close", &[0x2FE]),
    ("hit_mid", &[0x3C8]),
    ("hit_high", &[0x33A]),
    ("hits_lo", &[0x310]),
    ("hits_mid", &[0x324, 0x364, 0x3DA]),
    ("hits_hi", &[0x378, 0x3EE, 0x426]),
    // Engaged `0x4A2`.
    ("engage_far", &[0x4BA]),
    ("engage_mid", &[0x4DC]),
    ("engage_close", &[0x4FE]),
    ("high_above", &[0x524]),
    // Mid `0x5DA`, close `0x69C`.
    ("approach_speed", &[0x5FE]),
    ("approach_near", &[0x604]),
    ("back_speed", &[0x390, 0x406, 0x686, 0x714, 0x872, 0x3116]),
    ("back_far", &[0x396, 0x40C, 0x311C]),
    ("back_near", &[0x648, 0x6D6, 0x834]),
    ("back_add_near", &[0x65A, 0x6E8, 0x846]),
    ("back_add_far", &[0x66E, 0x6FC, 0x85A]),
    ("close_chance", &[0x6B0]),
    ("rise_floor", &[0x732]),
    ("rise_above", &[0x75A]),
    ("close_rise", &[0x778]),
    ("rise_speed", &[0x784, 0x9EA, 0x2E2A]),
    ("roam_min", &[0x7C0]),
    ("roam_max", &[0x7C6]),
    ("roam_speed", &[0x7E0]),
    ("roam_arrive", &[0x7E6]),
    ("roam_time", &[0x7EC]),
    ("round_limit", &[0x896]),
    // Idle `0x8D2`.
    ("circle_time_lo", &[0x920, 0x9FE]),
    ("circle_time_hi", &[0x926, 0xA04]),
    ("circle_turn_lo", &[0x930, 0xA0E]),
    ("circle_turn_hi", &[0x936, 0xA14]),
    ("rest_lo", &[0x956]),
    ("rest_hi", &[0x95C]),
    ("idle_above", &[0x9C2]),
    ("idle_rise", &[0x9DE]),
    // Slash `0xBC6`, swoop `0xC4E` / `0xEBC`.
    ("slash_tail", &[0xC00]),
    ("swoop_low", &[0xC76, 0xEE4]),
    ("swoop_settle", &[0xC92, 0xF00]),
    ("swoop_settle_speed", &[0xC9E, 0xF0C]),
    ("swoop_high", &[0xCCA, 0xD10, 0xF38, 0xF7E]),
    ("swoop_sink", &[0xD36, 0xFA4]),
    ("swoop_sink_speed", &[0xD56, 0xFC4]),
    ("swoop_tail", &[0xDCE, 0x103C]),
    ("swoop_near", &[0xE04, 0x1072]),
    ("swoop_speed", &[0xE5C, 0x10CA]),
    // Far `0x2100`.
    ("below_dir", &[0x2172]),
    ("above_dir", &[0x2466]),
    ("below_cos", &[0x21B6]),
    ("level_cos", &[0x2208]),
    ("above_cos", &[0x24AA]),
    ("above_level_cos", &[0x24D4]),
    ("far_windup", &[0x2240, 0x2508]),
    ("windup_above", &[0x2264, 0x252C]),
    ("charge_drop", &[0x2340]),
    ("charge_speed", &[0x2352, 0x25EA]),
    ("charge_arrive", &[0x2358, 0x2400, 0x25F0]),
    ("charge_time", &[0x235E, 0x2406, 0x25F6]),
    ("level_speed", &[0x23FA]),
    // Climb `0x2642`.
    ("climb_under", &[0x265C]),
    ("climb_near", &[0x267A]),
    ("climb_up", &[0x2732]),
    ("climb_speed", &[0x2752]),
    ("climb_stuck", &[0x2784]),
    // Dive `0x27CA`.
    ("dive_above", &[0x27E4]),
    ("dive_near", &[0x2850]),
    ("dive_floor", &[0x289E, 0x2932]),
    ("dive_sink", &[0x28D8]),
    ("dive_sink_speed", &[0x28E8]),
    ("dive_down", &[0x2990]),
    ("dive_speed", &[0x29B0]),
    ("dive_stuck", &[0x29DC]),
    // Charge `0x2A22`.
    ("charge_near", &[0x2AB6]),
    ("charge_low", &[0x2AE8]),
    ("bank_turn", &[0x2C00]),
    ("bank_cos", &[0x2C2E, 0x2C82]),
    ("bank_hold", &[0x2CC2]),
    ("bank_face", &[0x2CE6]),
    ("charge_floor", &[0x2D68]),
    ("charge_check", &[0x2DDC]),
    ("charge_stuck", &[0x2DFE]),
    ("charge_rise", &[0x2E1E]),
    // Approach `0x2E7E`.
    ("approach_top", &[0x2ECE]),
    ("approach_over", &[0x2EEC]),
    ("approach_floor", &[0x2FB2]),
    ("approach_check", &[0x3038]),
    ("approach_stuck", &[0x305A]),
    ("approach_rise", &[0x307A]),
    ("approach_rise_speed", &[0x3086]),
    // Fly to a point `0x30B6`.
    ("fly_check", &[0x30CA]),
    ("fly_near", &[0x3100]),
    ("fly_floor", &[0x31D0]),
    ("fly_stuck", &[0x3264]),
    ("fly_stuck_wait", &[0x3288]),
    // Nudge `0x32B4`.
    ("nudge_range", &[0x32FC]),
    ("nudge_y", &[0x3334]),
    ("nudge_z", &[0x3344]),
    ("nudge_speed", &[0x3354]),
    // Back off `0x3368`.
    ("back_chance", &[0x33B6]),
    ("back_turn", &[0x33C8]),
    ("back_turn_other", &[0x33D8]),
    ("back_wait", &[0x33EE]),
    ("back_check", &[0x3492]),
    ("back_stuck", &[0x34B4]),
    ("back_close", &[0x34D8]),
    ("back_close_speed", &[0x34F0]),
    ("back_rise", &[0x34FE]),
    // Circle `0x358A`.
    ("circle_half", &[0x35AC]),
    ("circle_chance", &[0x35BA]),
    ("circle_left", &[0x35CC]),
    ("circle_right", &[0x35F4]),
    ("circle_flip", &[0x3604]),
    ("circle_speed", &[0x368E]),
    // Rise `0x3706`.
    ("rise_up", &[0x374C]),
    ("rise_stuck", &[0x3798]),
];

/// The request words the script pushes (bit 24: an action; bit 30: a request that clears the action
/// flag when it starts): the behaviour plays exactly these.
pub const REQUESTS: &[(u32, usize)] = &[
    (69, 0x1962),          // the appearance
    (0x4000_0000, 0x19C0), // its queued idle
    (211, 0x294),          // hit in the dive
    (211, 0x2AC),          // its length
    (204, 0x39C),          // the back-offs' rise
    (204, 0x412),
    (204, 0x690),
    (204, 0x71E),
    (204, 0x77E),
    (204, 0x87C),
    (204, 0x9E4),
    (204, 0x275E),
    (204, 0x2E24),
    (204, 0x3122),
    (212, 0xBD8),          // the slash
    (212, 0xBF0),          // its length
    (200, 0xD98),          // the swoop
    (200, 0xDBE),          // its length
    (200, 0x1006),         // the swoop (blend 0)
    (200, 0x102C),
    (205, 0x21D2),         // before the dive
    (205, 0x21EA),
    (210, 0x227A),         // the wind-up
    (210, 0x2292),
    (210, 0x2542),
    (210, 0x255A),
    (201, 0x22E6),         // the take-off
    (201, 0x22FE),
    (201, 0x23AC),
    (201, 0x23C4),
    (201, 0x25AE),
    (201, 0x25C6),
    (201, 0x33FE),
    (201, 0x3638),
    (207, 0x280C),         // the dive's last stretch
    (207, 0x281E),
    (206, 0x29BC),         // the dive
    (206, 0x1F78),         // the hit handler's test
    (202, 0x2B96),         // flying
    (202, 0x2CD4),
    (202, 0x3410),
    (209, 0x2C58),         // banking
    (208, 0x2CAC),
    (208, 0x35E2),
    (209, 0x361C),
    (203, 0x353C),         // the halt
    (203, 0x354E),
    (0x0100_00CB, 0x36D6), // the circle's halt, an action
    (0x4000_0000, 0x36EE), // its queued idle
];

/// The rows (script offsets) and the Re:CoM attack ids they take their power from (decision M10:
/// Re:CoM's attack power per attack). Re:CoM's Air Pirate has two (Checked (files): `EN0022.ASD`, both a
/// 0.5 m sphere on `r_hand`, the sword hand): id 0 on `e022_050` (frames 38..60, its state 2), id 1 on
/// `e022_051` (37..52, its state 3). KH1's matcher finds `e022_050` an edit (≤ 20 bones differ) of KH1's
/// slash 212 (Checked (files)), so the slash's row `0x15C` (bone code 29 → the sword hand, a line t −0.5
/// → 1, radius 60, KH1 power 100 %) takes id 0. The swoop's row `0xEC` (the same hand, t 0.1 → 1, radius
/// 60, KH1 power 50 %) has no Re:CoM twin: id 1, Re:CoM's other sword attack (Assumption; owner question).
const SWOOP: RowSpec = RowSpec { at: 0xEC, recom_id: 1 };
const SLASH: RowSpec = RowSpec { at: 0x15C, recom_id: 0 };

/// `kh1_core::airpirate::Attack::ALL`'s order.
pub const ATTACKS: &[AttackSpec] = &[AttackSpec { name: "swoop", request: 200, rows: &[SWOOP] }, AttackSpec { name: "slash", request: 212, rows: &[SLASH] }];

/// The Air Pirate.
pub const SPEC: Spec = Spec {
    name: "airpirate",
    recom_id: 22,
    kh1: "xa_ex_2210",
    script: b"ex_2210.bd",
    prefix: "e022_",
    recom_slots: 74,
    asd: (2, 0x6C, 0x1AC),
    consts: CONSTS,
    requests: REQUESTS,
    // The stat records (`0x1E04`: `sys 45` with `0x14` (LV1) or `0x80` (LV2), `crate::stats::TWINS`).
    records: &[0x14, 0x80],
    attacks: ATTACKS,
    manifest: "kh1-airpirate.json",
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
        assert_eq!((get("search_range"), get("engage_mid"), get("engage_close"), get("charge_speed")), (5000.0, 500.0, 200.0, 24.0));
        assert_eq!((get("dive_above"), get("back_far"), get("rise_floor"), get("approach_top")), (320.0, 600.0, -170.0, -100.0));
        assert_eq!((get("hits_mid"), get("hits_hi"), get("round_limit"), get("bank_cos")), (5.0, 10.0, 10.0, 0.98));
        assert_eq!((get("nudge_y"), get("nudge_z"), get("circle_left"), get("circle_right")), (0.2, -1.0, -90.0, 90.0));
        assert_eq!((get("weight"), get("walk_speed"), get("motion_blend"), get("bank_blend")), (1.0, 2.0, 8.0, 15.0));
        let c = |r: u16| b.clips.iter().find(|c| c.request == r).unwrap().clone();
        // No Re:CoM bake: every request is a new clip at 100 + entry.
        assert!(b.clips.iter().all(|c| c.source == "kh1"));
        assert_eq!((c(200).slot, c(200).root, c(212).slot, c(212).root), (116, true, 123, true));
        assert_eq!((c(200).frames, c(212).frames, c(69).frames, c(203).frames), (122, 100, 78, 50));
        // Re:CoM's 74 slots unchanged.
        let a0 = crate::ann::Ann::parse(&inp.ann).unwrap();
        let a1 = crate::ann::Ann::parse(&b.ann).unwrap();
        for i in 0..SPEC.recom_slots {
            assert_eq!(a0.slots[i], a1.slots[i], "slot {i}");
        }
        // ASD: Re:CoM's bytes kept, one record per attack window (2), the tail moved.
        assert_eq!(&b.asd[..0x10], &inp.asd[..0x10]);
        assert_eq!(&b.asd[0x6C..0x1AC], &inp.asd[0x6C..0x1AC]);
        assert_eq!(u32::from_le_bytes(b.asd[0x10..0x14].try_into().unwrap()), 4);
        assert_eq!(&b.asd[0x1AC + 2 * 0xA0..], &inp.asd[0x1AC..]);
        let rec = |i: usize| &b.asd[0x1AC + i * 0xA0..0x1AC + (i + 1) * 0xA0];
        let win = |i: usize| (u32::from_le_bytes(rec(i)[0x5C..0x60].try_into().unwrap()), u32::from_le_bytes(rec(i)[0x60..0x64].try_into().unwrap()));
        let id = |i: usize| u32::from_le_bytes(rec(i)[0x64..0x68].try_into().unwrap());
        assert_eq!((win(0), id(0)), ((41 * 50, 48 * 50), 1), "the swoop [41, 48)");
        assert_eq!((win(1), id(1)), (((39.3f32 * 50.0).round() as u32, 51 * 50), 0), "the slash [39.3, 51)");
        assert_eq!((&rec(0)[0x40..0x48], &rec(1)[0x40..0x48]), (&b"e022_116"[..], &b"e022_123"[..]));
        assert_eq!((&rec(0)[0..6], &rec(1)[0..6]), (&b"r_hand"[..], &b"r_hand"[..]));
        // The clash recoils: the swoop's 213 is a motion of the model, the slash's 110 is not.
        assert!(b.manifest.contains("{\"name\": \"swoop\", \"request\": 200, \"recoil\": 213"));
        assert!(b.manifest.contains("{\"name\": \"slash\", \"request\": 212, \"recoil\": 110"));
        assert!(!b.clips.iter().any(|c| c.request == 110));
        // SES: Re:CoM's entry kept; the slash's clip gets `e022_050`'s sounds (its edit).
        let s0 = crate::ses::parse(&inp.ses).unwrap();
        let s1 = crate::ses::parse(&b.ses).unwrap();
        for e in &s0 {
            assert!(s1.iter().any(|x| x.name() == e.name() && x.events == e.events), "{}", e.name());
        }
        let ev = |n: &str| s1.iter().find(|e| e.name() == n).map(|e| e.events.clone());
        assert_eq!(ev("e022_123"), ev("e022_050"));
        assert_eq!(b.parts.len(), 7);
        assert!(b.manifest.contains("\"enemy\": \"airpirate\""));
        std::fs::write(std::env::temp_dir().join("kh1-airpirate-test.json"), &b.manifest).unwrap();
        println!("{}\nparts {:?}", b.records.join("\n"), b.parts);
    }
}
