//! M11: KH1's Wyvern in Re:CoM's battles (`docs/recom-m11-enemies.md` § The Wyvern): where its numbers sit
//! in the player's `ex_2320.bd` (`xa_ex_2320.mdls` part 5) and which Re:CoM files it extends
//! (`ENEMY/0024`). The step itself is `crate::enemy`.
//!
//! The offsets are the script's own (Checked (files): the player's script disassembled; the routine each
//! sits in is named in `kh1_core::wyvern`). `walk_speed` / `run_speed` are the stats block's defaults (the
//! script never writes them; the flier never walks).

use crate::enemy::{AttackSpec, RowSpec, Spec};

/// Where each constant is pushed in `ex_2320.bd` (float or integer); a name with several offsets must read
/// the same at each.
pub const CONSTS: &[(&str, &[usize])] = &[
    ("walk_speed", &[]),
    ("run_speed", &[]),
    // Supervisor `0x21CC`, search `0x1930`.
    ("search_range", &[0x229C]),
    ("world11_search", &[0x1962]),
    // Blends (every `sys 12` / `sys 13` / `sys 14` of 8) and the rates the routines put back.
    ("motion_blend", &[0x484, 0x50A, 0x76E, 0xA16, 0xCEC, 0xF08, 0x10F4, 0x116C, 0x123E, 0x1324, 0x13CE, 0x1454, 0x281E, 0x28E0, 0x291A, 0x2A2A, 0x2B12, 0x3250]),
    ("attack_blend", &[0x3250]),
    ("turn_default", &[0x7F0, 0xD64, 0xE00, 0xFEC, 0x1582, 0x2548, 0x289A, 0x2F40, 0x3220, 0x32E8]),
    ("ease_default", &[0x288E, 0x29A2, 0x2AB2, 0x2F52]),
    // Appearance `0xC6E`.
    ("appear_fade", &[0xC72]),
    ("appear_blend", &[0xC96]),
    ("appear_then_blend", &[0xCAE]),
    // Hit `0x3C4`.
    ("dive_hit_blend", &[0x3EA]),
    ("hit_close", &[0x424]),
    ("hit_mid", &[0x4AA]),
    ("hits_lo", &[0x436, 0x4BC]),
    ("hits_hi", &[0x44A, 0x462, 0x4D0, 0x4E8]),
    ("idle_pause", &[0x48C, 0x512, 0x776]),
    // The low test and the rise `0x30C2`.
    ("low_height", &[0x52A, 0x90A]),
    ("low_above", &[0x552, 0x622, 0x702, 0x932, 0xB6C]),
    ("rise_up", &[0x570, 0x63E, 0x71E, 0x950, 0xB88]),
    ("rise_speed", &[0x576, 0x644, 0x724, 0x956, 0xB8E]),
    ("rise_time", &[0x30D2]),
    ("rise_check", &[0x30E0]),
    ("rise_turn_rate", &[0x3126]),
    ("rise_dir", &[0x3164]),
    ("hover_blend", &[0x2F86, 0x3102]),
    // Lost `0x5D2`, idle `0xB4E`, circle `0x26EA`.
    ("lost_search", &[0x5E6]),
    ("lost_rest_blend", &[0x68A]),
    ("idle_rest_blend", &[0xBD8]),
    ("idle_turn_rate", &[0xBE4]),
    ("rest_lo", &[0x692, 0xBEC]),
    ("rest_hi", &[0x698, 0xBF2]),
    ("circle_time_lo", &[0x65C, 0x740, 0xBA6]),
    ("circle_time_hi", &[0x662, 0x746, 0xBAC]),
    ("circle_turn_lo", &[0x66C, 0x750, 0xBB6]),
    ("circle_turn_hi", &[0x672, 0x756, 0xBBC]),
    ("circle_turn_rate", &[0x2700]),
    ("circle_chance", &[0x271E]),
    ("circle_left", &[0x2730]),
    ("circle_right", &[0x2754]),
    ("circle_flip", &[0x2764]),
    ("circle_blend", &[0x2798]),
    ("circle_wait", &[0x27A6]),
    ("circle_half", &[0x27CA]),
    ("circle_speed", &[0x27F8]),
    ("halt_ease", &[0x2846]),
    ("halt_tail", &[0x2852, 0x294E, 0x2A5E]),
    // Engaged `0x7BE`, notice `0xCCC`, roar `0xAF2`.
    ("round_limit", &[0x7FC]),
    ("engage_far", &[0x844]),
    ("far_chance", &[0x85C]),
    ("close_rounds", &[0x886]),
    ("notice_turn_rate", &[0xCDA]),
    ("notice_then_blend", &[0xD04]),
    ("notice_extra", &[0xD1E]),
    ("roar_blend", &[0xB0C]),
    ("roar_then_blend", &[0xB24]),
    // The far attack `0x904`, the dive attack `0xA8C`.
    ("approach_face_rate", &[0x97A]),
    ("chase_speed", &[0x9A0, 0xABC]),
    ("chase_arrive_far", &[0x9A6]),
    ("chase_arrive_dive", &[0xAC2]),
    ("chase_time", &[0x9AC, 0xAC8]),
    ("flyby_ahead", &[0x9E4]),
    ("flyby_up", &[0xA02]),
    ("flyby_speed", &[0xA28]),
    ("flyby_arrive", &[0xA2E]),
    ("flyby_time", &[0xA34]),
    ("flyby_chance", &[0xA5A]),
    ("dive_above", &[0xAA6]),
    // The charges `0xD70` / `0xF52`.
    ("hit_charge_face_rate", &[0xD80]),
    ("charge_face_rate", &[0xF62]),
    ("hit_windup_blend", &[0xD94]),
    ("windup_blend", &[0xF76]),
    ("windup_wait", &[0xF84]),
    ("windup_short", &[0xDAE, 0xF9A]),
    ("windup_turn_rate", &[0xDD0, 0xFBC]),
    ("charge_blend", &[0xE12, 0xE5E, 0xFFE, 0x104A]),
    ("charge_time", &[0xE38, 0x1024]),
    ("charge_sink", &[0xEAE, 0x109A]),
    ("charge_speed", &[0xEC2, 0x10AE]),
    ("charge_then_blend", &[0xF20, 0x110C]),
    ("charge_extra", &[0xF3A, 0x1126]),
    // The dive `0x113E`.
    ("bomb_above", &[0x1158]),
    ("bomb_queue_blend", &[0x117E]),
    ("bomb_flat", &[0x11EA]),
    ("bomb_creep", &[0x1214]),
    ("bomb_floor", &[0x128A]),
    ("bomb_down", &[0x12C2]),
    ("bomb_speed", &[0x12E2]),
    ("bomb_stuck", &[0x12FC]),
    ("land_floor", &[0x1362]),
    ("land_down", &[0x1390]),
    ("land_speed", &[0x13A0]),
    // The bite `0x13D8`.
    ("bite_turn_rate", &[0x13F2]),
    ("bite_blend_flying", &[0x1418]),
    ("bite_wait", &[0x1482]),
    ("bite_tail", &[0x14E0]),
    ("bite_tail_speed", &[0x14F6]),
    ("bite_speed", &[0x150A]),
    ("bite_above", &[0x152A]),
    ("bite_sink", &[0x1542]),
    ("bite_end_blend", &[0x1570]),
    // Fly to `0x28A4`, the chase `0x2ADA`.
    ("stuck_after", &[0x28B8, 0x2AFC]),
    ("stuck_factor", &[0x2A0C, 0x2EFA, 0x31CA]),
    ("fly_halt_ease", &[0x2942, 0x2A52]),
    ("fly_idle_blend", &[0x298A, 0x2A9A]),
    ("fly_level", &[0x29CE]),
    ("chase_queue_blend", &[0x2B24]),
    ("chase_near", &[0x2BA8]),
    ("chase_stop_ease", &[0x2C56]),
    ("bank_turn", &[0x2CD2]),
    ("chase_turn_rate", &[0x2CEC]),
    ("bank_cos", &[0x2D1C, 0x2D70]),
    ("bank_blend", &[0x2D4C, 0x2DA0, 0x2DC8]),
    ("bank_hold", &[0x2DB0]),
    ("bank_face", &[0x2DD4]),
    ("chase_floor", &[0x2E66]),
    // Descend `0x2F5C`.
    ("descend_face_rate", &[0x2F72]),
    ("descend_idle_blend", &[0x2FC8, 0x3016]),
    ("descend_idle_start", &[0x2FCE, 0x301C]),
    ("descend_wait", &[0x2FD6, 0x3024]),
    ("descend_down", &[0x304C]),
    ("descend_speed", &[0x306C]),
    ("descend_stuck", &[0x3092]),
    // Turn `0x322A`.
    ("turn_time", &[0x3232]),
    ("turn_cos", &[0x32B0]),
];

/// The request words the script pushes (bit 24: an action; bit 30: a request that clears the action flag
/// when it starts): the behaviour plays exactly these.
pub const REQUESTS: &[(u32, usize)] = &[
    (0x0100_0045, 0xC90), // the appearance, an action
    (0x4000_0000, 0xCA8), // its queued idle
    (214, 0x3E4),         // hit in the dive
    (214, 0x3FC),         // its length
    (202, 0x2556),        // the hit handler's test: the dive
    (0x0100_00D5, 0xB06), // the roar, an action
    (0x4000_0000, 0xB1E), // its queued idle
    (215, 0xCE6),         // the notice
    (215, 0xD16),         // its length
    (204, 0xD8E),         // the wind-up (out of a hit)
    (204, 0xDA6),
    (204, 0xF70),         // the wind-up (engaged)
    (204, 0xF92),
    (205, 0xE0C),         // the charge
    (205, 0xE58),
    (205, 0xFF8),
    (205, 0x1044),
    (206, 0xF02),         // its end
    (206, 0xF32),
    (206, 0x10EE),
    (206, 0x111E),
    (201, 0x1166),        // before the dive
    (201, 0x1190),
    (202, 0x1178),        // the dive (queued, then with its row)
    (202, 0x1238),
    (203, 0x131E),        // the landing
    (203, 0x133E),
    (208, 0x1400),        // the bite's test: flying
    (200, 0x1412),        // the bite
    (200, 0x144E),
    (200, 0x14A4),
    (208, 0x156A),        // flying after it
    (208, 0xA10),         // the fly-by
    (208, 0x28DA),
    (209, 0x2742),        // the circle's banks
    (210, 0x2778),
    (211, 0x2818),        // the halts
    (211, 0x2830),
    (211, 0x2914),
    (211, 0x292C),
    (211, 0x2A24),
    (211, 0x2A3C),
    (207, 0x2B0C),        // the chase
    (208, 0x2B1E),
    (210, 0x2D46),        // its banks
    (209, 0x2D9A),
    (208, 0x2DC2),
    (212, 0x2F80),        // hovering down
    (212, 0x30FC),        // hovering up
];

/// The rows (script offsets) and the Re:CoM attack ids they take their power from (decision M10: Re:CoM's
/// attack power per attack). Re:CoM's Wyvern has two attacks (Checked (files): `EN0024.ASD`, 3 records):
/// id 0, its claw `e024_050` (two 0.5 m spheres on `r_foot2` / `l_foot2`, frames 20..80, its state 2), and
/// id 1, its rush `e024_052` (a 0.55 m line `head` → `tail2` through the whole clip, its state 3 after
/// `e024_051`, then `e024_053`). KH1's matcher finds `e024_050` an edit (≤ 20 bones differ) of KH1's bite
/// 200, and `e024_051` / `e024_052` / `e024_053` edits of KH1's wind-up 204, charge 205 and its end 206
/// (Checked (files)): the bite's two foot rows take id 0, the charge's row `0x1CC` id 1 on 205 and 206. KH1's
/// dive (202 → 203, rows `0x23C` / `0x2AC`) has no Re:CoM twin: its rows are the charge row's shape (bone
/// code 0, t 0.5 → −2, radius 60, KH1 power 100 %), a body ram: id 1 (Assumption: the same kind of attack;
/// owner question).
const BITE_R: RowSpec = RowSpec { at: 0xEC, recom_id: 0 };
const BITE_L: RowSpec = RowSpec { at: 0x15C, recom_id: 0 };
const CHARGE: RowSpec = RowSpec { at: 0x1CC, recom_id: 1 };
const DIVE: RowSpec = RowSpec { at: 0x23C, recom_id: 1 };
const LAND: RowSpec = RowSpec { at: 0x2AC, recom_id: 1 };

/// `kh1_core::wyvern::Attack::ALL`'s order.
pub const ATTACKS: &[AttackSpec] = &[
    AttackSpec { name: "bite", request: 200, rows: &[BITE_R, BITE_L] },
    AttackSpec { name: "dive", request: 202, rows: &[DIVE] },
    AttackSpec { name: "landing", request: 203, rows: &[LAND] },
    AttackSpec { name: "charge", request: 205, rows: &[CHARGE] },
    AttackSpec { name: "charge end", request: 206, rows: &[CHARGE] },
];

/// The Wyvern.
pub const SPEC: Spec = Spec {
    name: "wyvern",
    recom_id: 24,
    kh1: "xa_ex_2320",
    script: b"ex_2320.bd",
    prefix: "e024_",
    recom_slots: 77,
    asd: (3, 0x10C, 0x2EC),
    consts: CONSTS,
    requests: REQUESTS,
    // The stat records (`0x2362`: `sys 45` with `0x14` (LV1) or `0x80` (LV2), `crate::stats::TWINS`).
    records: &[0x14, 0x80],
    attacks: ATTACKS,
    manifest: "kh1-wyvern.json",
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enemy::{build, Inputs};

    /// The player's files: `KH1_M11_DIR` (KH1's `xa_*.mdls` / `.mset` and Re:CoM's `ENEMY__NNNN__ENNNNN.*`)
    /// and `KH1_TEST_DATA` (`btltbl.bin`).
    fn inputs() -> Option<Inputs> {
        crate::enemy::test_inputs(&SPEC, true)
    }

    #[test]
    fn builds_from_the_players_files() {
        let Some(inp) = inputs() else { return };
        let b = build(&SPEC, &inp).unwrap();
        let get = |n: &str| b.consts.iter().find(|c| c.0 == n).map(|c| c.1).unwrap();
        // The script's numbers (the disassembly's).
        assert_eq!((get("search_range"), get("engage_far"), get("hit_close"), get("hit_mid")), (1500.0, 600.0, 300.0, 1000.0));
        assert_eq!((get("chase_speed"), get("dive_above"), get("bomb_above"), get("bomb_speed")), (30.0, 400.0, 250.0, 24.0));
        assert_eq!((get("hits_hi"), get("round_limit"), get("close_rounds"), get("far_chance")), (5.0, 3.0, 2.0, 0.3));
        assert_eq!((get("turn_default"), get("ease_default"), get("charge_speed"), get("charge_time")), (11.25, 0.2, 40.0, 40.0));
        assert_eq!((get("descend_idle_start"), get("rise_up"), get("low_above"), get("world11_search")), (16.0, -300.0, 170.0, 10000.0));
        assert_eq!((get("weight"), get("walk_speed"), get("motion_blend"), get("bank_blend")), (1.0, 2.0, 8.0, 16.0));
        let c = |r: u16| b.clips.iter().find(|c| c.request == r).unwrap().clone();
        // Re:CoM's bakes: the two banks (`e024_005` / `e024_006`); every other request a new clip at 100 +
        // entry.
        assert_eq!((c(209).name.as_str(), c(210).name.as_str(), c(209).source, c(210).source), ("e024_005", "e024_006", "recom", "recom"));
        assert!(b.clips.iter().filter(|c| c.request != 209 && c.request != 210).all(|c| c.source == "kh1"));
        assert_eq!((c(200).slot, c(202).slot, c(203).slot, c(205).slot, c(206).slot), (112, 114, 115, 117, 118));
        assert_eq!((c(200).frames, c(202).frames, c(203).frames, c(205).frames, c(206).frames, c(69).frames), (120, 120, 100, 120, 106, 60));
        assert_eq!((c(204).root, c(213).root, c(202).root), (true, true, false));
        // `enemy::LOOPS`: the wind-up loops as KH1's does (the engaged charge plays it 122 frames).
        assert!(c(204).looped);
        // Re:CoM's 77 slots unchanged.
        let a0 = crate::ann::Ann::parse(&inp.ann).unwrap();
        let a1 = crate::ann::Ann::parse(&b.ann).unwrap();
        for i in 0..SPEC.recom_slots {
            assert_eq!(a0.slots[i], a1.slots[i], "slot {i}");
        }
        // ASD: Re:CoM's bytes kept, one record per attack row window (6), the tail moved.
        assert_eq!(&b.asd[..0x10], &inp.asd[..0x10]);
        assert_eq!(&b.asd[0x10C..0x2EC], &inp.asd[0x10C..0x2EC]);
        assert_eq!(u32::from_le_bytes(b.asd[0x10..0x14].try_into().unwrap()), 3 + 6);
        assert_eq!(&b.asd[0x2EC + 6 * 0xA0..], &inp.asd[0x2EC..]);
        let rec = |i: usize| &b.asd[0x2EC + i * 0xA0..0x2EC + (i + 1) * 0xA0];
        let win = |i: usize| (u32::from_le_bytes(rec(i)[0x5C..0x60].try_into().unwrap()) / 50, u32::from_le_bytes(rec(i)[0x60..0x64].try_into().unwrap()) / 50);
        let id = |i: usize| u32::from_le_bytes(rec(i)[0x64..0x68].try_into().unwrap());
        let clip = |i: usize| String::from_utf8_lossy(&rec(i)[0x40..0x48]).into_owned();
        assert_eq!((clip(0), win(0), id(0)), ("e024_112".into(), (20, 84), 0), "the bite, right foot");
        assert_eq!((clip(1), win(1), id(1)), ("e024_112".into(), (20, 84), 0), "the bite, left foot");
        assert_eq!((clip(2), win(2), id(2)), ("e024_114".into(), (0, 120), 1), "the dive");
        assert_eq!((clip(3), win(3), id(3)), ("e024_115".into(), (0, 36), 1), "the landing");
        assert_eq!((clip(4), win(4), id(4)), ("e024_117".into(), (0, 120), 1), "the charge");
        assert_eq!((clip(5), win(5), id(5)), ("e024_118".into(), (0, 24), 1), "its end");
        // The clash recoils: every row's is 110, which the model doesn't have (Re:CoM's flinch).
        for a in ["bite", "dive", "landing", "charge", "charge end"] {
            assert!(b.manifest.contains(&format!("{{\"name\": \"{a}\", \"request\": ")), "{a}");
        }
        assert_eq!(b.manifest.matches("\"recoil\": 110").count(), 5);
        assert!(!b.clips.iter().any(|c| c.request == 110));
        // SES: Re:CoM's entries kept; the edits get their Re:CoM clip's sounds.
        let s0 = crate::ses::parse(&inp.ses).unwrap();
        let s1 = crate::ses::parse(&b.ses).unwrap();
        for e in &s0 {
            assert!(s1.iter().any(|x| x.name() == e.name() && x.events == e.events), "{}", e.name());
        }
        let ev = |n: &str| s1.iter().find(|e| e.name() == n).map(|e| e.events.clone());
        assert_eq!(ev("e024_112"), ev("e024_050"), "the bite: the claw's sounds");
        assert_eq!(ev("e024_117"), ev("e024_052"), "the charge: the rush's");
        assert_eq!(ev("e024_118"), ev("e024_053"), "its end");
        assert!(ev("e024_114").is_some() && ev("e024_115").is_some(), "the dive and landing: the rush's sounds at their windows");
        assert_eq!(b.parts.len(), 11);
        assert!(b.manifest.contains("\"enemy\": \"wyvern\""));
        std::fs::write(std::env::temp_dir().join("kh1-wyvern-test.json"), &b.manifest).unwrap();
        println!("{}\nparts {:?}", b.records.join("\n"), b.parts);
    }
}
