//! M11: KH1's Darkball in Re:CoM's battles (`docs/recom-m11-enemies.md` § The Darkball): where its
//! numbers sit in the player's `ex_2280.bd` (`xa_ex_2280.mdls` part 5) and which Re:CoM files it
//! extends (`ENEMY/0023`). The step itself is `crate::enemy`.
//!
//! The offsets are the script's own (Checked (files): the player's script disassembled; the routine
//! each sits in is named in `kh1_core::darkball`). `walk_speed` / `run_speed` are the stats block's
//! defaults (the script never writes them; the flier never walks). The core point (`core_x..z`) is
//! `crate::enemy::BONE_POINTS`'.

use crate::enemy::{AttackSpec, RowSpec, Spec};

/// Where each constant is pushed in `ex_2280.bd` (float or integer); a name with several offsets
/// must read the same at each.
pub const CONSTS: &[(&str, &[usize])] = &[
    ("walk_speed", &[]),
    ("run_speed", &[]),
    ("turn_deg", &[0x1A68]),
    ("search_range", &[0x1A8E, 0x1B7E, 0x606]),
    // Appearance `0x9A0`.
    ("appear_up", &[0xA4C]),
    ("appear_cut", &[0xA7E]),
    ("appear_drift_delay", &[0xB14]),
    ("appear_drift", &[0xB38]),
    ("appear_drift_speed", &[0xB58]),
    ("appear_wait", &[0xBC2]),
    // Blends.
    ("motion_blend", &[0x9D4, 0xBF2, 0xC0A, 0x3C2, 0x3DA, 0xCDC, 0xD44, 0xE6C, 0xE84, 0x2878, 0x2890, 0x2C8E, 0x2CA6, 0x2AB2]),
    ("attack_blend", &[0xC9A, 0xCB2]),
    ("rest_blend", &[0x354C]),
    ("reappear_queue_blend", &[0x2ACA]),
    // Engaged `0x682`, notice `0xBCE`, close `0x7B6`, mid `0x806`.
    ("notice_tail", &[0xC24, 0x3F4]),
    ("miss_limit", &[0x6A0]),
    ("far_range", &[0x6E8]),
    ("far_chance", &[0x700]),
    ("misses_close", &[0x7FA]),
    ("misses_mid", &[0x89C, 0x8B4]),
    ("side_speed", &[0x810]),
    ("side_time", &[0x816]),
    ("side_near", &[0x81C]),
    ("dash_count", &[0x834]),
    // Lost `0x538`.
    ("lost_speed", &[0x538]),
    ("lost_time", &[0x53E]),
    ("lost_near", &[0x544]),
    ("lost_back_range", &[0x566]),
    ("lost_back", &[0x5AA]),
    ("lost_up_lo", &[0x5C8]),
    ("lost_up_hi", &[0x5CE]),
    ("lost_fly_speed", &[0x5E0]),
    ("lost_fly_arrive", &[0x5E6]),
    ("lost_fly_time", &[0x5EC]),
    // Idle `0x8DA`.
    ("wander_min", &[0x8E8]),
    ("wander_max", &[0x8EE]),
    ("wander_low", &[0x8FE]),
    ("wander_up_lo", &[0x916]),
    ("wander_up_hi", &[0x91C]),
    ("wander_speed", &[0x946]),
    ("wander_arrive", &[0x94C]),
    ("wander_time", &[0x952]),
    ("wander_rest", &[0x95C]),
    // Fly to a point `0x331C`.
    ("fly_check", &[0x3330]),
    ("fly_low", &[0x33BC]),
    ("fly_rise", &[0x33D2]),
    ("fly_high", &[0x33E4]),
    ("fly_sink", &[0x341A]),
    ("fly_stuck", &[0x3448]),
    ("fly_new_min", &[0x345E]),
    ("fly_new_max", &[0x3464]),
    // Turn to `0x34A0`.
    ("turn_time", &[0x34A4]),
    ("turn_cos", &[0x3506]),
    // Approach at the side `0x3060`.
    ("side_check", &[0x307A]),
    ("side_bob_start", &[0x3086]),
    ("side_range", &[0x3112]),
    ("side_chance", &[0x3124]),
    ("side_left", &[0x3136]),
    ("side_right", &[0x3146]),
    ("side_far_factor", &[0x3162]),
    ("side_low", &[0x3190]),
    ("bob_min", &[0x31A2]),
    ("bob_max", &[0x31A8]),
    ("side_above_lo", &[0x31E2]),
    ("side_rise", &[0x31FA]),
    ("side_above_hi", &[0x321A]),
    ("side_sink", &[0x3232]),
    ("bob_chance", &[0x3242]),
    ("bob_up", &[0x3258]),
    ("bob_down", &[0x326C]),
    ("side_low_rise", &[0x3284]),
    ("side_stuck", &[0x32C4]),
    ("side_stuck_wait", &[0x32E2]),
    // Dash `0xCC4`.
    ("dash_wait", &[0xCEA]),
    ("dash_time", &[0xD68]),
    ("dash_speed", &[0xDBE]),
    ("dash_low", &[0xDCC]),
    ("dash_above", &[0xDF2]),
    ("dash_sink", &[0xE0A]),
    ("dash_rise", &[0xE22]),
    // Hit `0x35C`, handler `0x1E20`.
    ("bite_mark", &[0x360, 0x2022]),
    ("gravity_mark", &[0x3A0, 0x1F4E]),
    ("hit_teleport", &[0x48A]),
    ("hit_close", &[0x49C]),
    ("hit_mid", &[0x4B6]),
    ("hit_low", &[0x418]),
    ("hit_rise", &[0x42E]),
    ("hit_rise_speed", &[0x43E]),
    ("light_limit", &[0x2014]),
    ("armor_limit", &[0x2034]),
    // Warp `0x2304`.
    ("warp_pause", &[0x231E]),
    ("warp_near_lo", &[0x233E, 0x23BC]),
    ("warp_near_hi", &[0x2344, 0x23C2]),
    ("warp_above", &[0x2368, 0x23E6]),
    ("warp_far", &[0x23AA]),
    ("warp_hop_lo", &[0x23F8]),
    ("warp_hop_hi", &[0x23FE]),
    ("warp_low", &[0x241A]),
    ("warp_low_up", &[0x2432]),
    ("warp_accel", &[0x245C]),
    ("glide_speed", &[0x2468]),
    ("glide_arrive", &[0x246E]),
    ("glide_time", &[0x2474]),
    ("accel_after", &[0x2488, 0x259E]),
    ("warp_reach", &[0x24AA]),
    ("warp_retry_wait", &[0x24C0]),
    ("warp_tries", &[0x24E0, 0x2504]),
    ("warp_give_up_accel", &[0x2578]),
    // Glide `0x25C0`.
    ("glide_check_a", &[0x25D4]),
    ("glide_check_b", &[0x25E4]),
    ("glide_near", &[0x263A]),
    ("glide_stop_accel", &[0x2680]),
    ("glide_arrive_accel", &[0x26F4, 0x27CE]),
    ("glide_low", &[0x274C]),
    ("glide_rise", &[0x2762]),
    ("glide_stuck", &[0x27B0]),
    // Vanish `0x2C7A`.
    ("vanish_cut", &[0x2D32]),
    ("vanish_low", &[0x2DCC]),
    ("vanish_rise", &[0x2DF0]),
    ("vanish_rise_speed", &[0x2E10]),
    // Reappearance `0x2A78`.
    ("reappear_rest", &[0x2A8E]),
    ("reappear_cut", &[0x2B00]),
    ("reappear_drift_delay", &[0x2BC0]),
    ("reappear_drift", &[0x2BE4]),
    ("reappear_drift_speed", &[0x2C04]),
    // Drop `0x27F8`.
    ("strike_rest", &[0x280E]),
    ("strike_wait", &[0x289E]),
    ("strike_fade", &[0x28D6]),
    ("strike_grow_time", &[0x28EE]),
    ("strike_grow_div", &[0x2914, 0x292A, 0x2940]),
    ("strike_hold", &[0x2950]),
    ("strike_cut", &[0x2966]),
    ("strike_peak", &[0x29A0, 0x29BC, 0x29D8]),
    ("strike_tail", &[0x2A6C]),
];

/// The request words the script pushes (bit 24: an action; bit 30: a request that clears the action
/// flag when it starts): the behaviour plays exactly these.
pub const REQUESTS: &[(u32, usize)] = &[
    (69, 0x9CE),           // the appearance
    (69, 0xA76),           // its length
    (205, 0xBEC),          // the notice
    (205, 0x3BC),          // the stagger
    (0x0100_00CC, 0xC94),  // the bite 204, an action
    (0x4000_0000, 0xCAC),  // its queued idle
    (201, 0xCD6),          // the dash's wind-up
    (201, 0xCF8),          // its length
    (202, 0xD3E),          // the dash
    (202, 0xD88),          // its loop
    (203, 0xE56),          // the dash's end (length)
    (203, 0xE66),          // the dash's end
    (200, 0x2C88),         // the vanish
    (200, 0x2D2A),         // its length
    (200, 0x2AAC),         // the reappearance
    (200, 0x2AF8),         // its length
    (200, 0x2872),         // the drop
    (200, 0x295E),         // its length
];

/// The rows (script offsets) and the Re:CoM attack ids they take their power from (decision M10:
/// Re:CoM's attack power per attack). Re:CoM's Darkball has two (Checked (files): `EN0023.ASD`): id 0
/// on its dash `e023_051` (a 0.6 m sphere on `ex_2280_sk`, the whole clip), id 1 on its bite
/// `e023_053` (1.3 m, frames 22..110). KH1's matcher finds both clips exact bakes of KH1's dash 202 and
/// bite 204 (Checked (files)), so the same attacks take the same power: the dash's row `0x158` (bone
/// code 8, a line t 0 → −1, radius 100, KH1 power 40 %) id 0, the bite's `0x238` (code 8, radius 160,
/// 100 %) id 1. The drop's row `0x1C8` (code 0, the core, radius 230, 100 %) has no Re:CoM twin: id 1
/// (Assumption: the nearest, Re:CoM's whole-body 1.3 m record, the same KH1 power as the bite; owner
/// question).
const DASH: RowSpec = RowSpec { at: 0x158, recom_id: 0 };
const BITE: RowSpec = RowSpec { at: 0x238, recom_id: 1 };
const DROP: RowSpec = RowSpec { at: 0x1C8, recom_id: 1 };

/// `kh1_core::darkball::Attack::ALL`'s order.
pub const ATTACKS: &[AttackSpec] = &[
    AttackSpec { name: "bite", request: 204, rows: &[BITE] },
    AttackSpec { name: "dash", request: 202, rows: &[DASH] },
    AttackSpec { name: "drop", request: 200, rows: &[DROP] },
];

/// The Darkball.
pub const SPEC: Spec = Spec {
    name: "darkball",
    recom_id: 23,
    kh1: "xa_ex_2280",
    script: b"ex_2280.bd",
    prefix: "e023_",
    recom_slots: 86,
    asd: (2, 0x6C, 0x1AC),
    consts: CONSTS,
    requests: REQUESTS,
    // The stat records (`0x1BD0`: `sys 45` with `0x14` (LV1), `0x80` (LV2) or `0xEC` (LV2, world 16
    // from room 37), `crate::stats::TWINS`).
    records: &[0x14, 0x80, 0xEC],
    attacks: ATTACKS,
    manifest: "kh1-darkball.json",
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
        assert_eq!((get("turn_deg"), get("search_range"), get("far_range"), get("glide_speed")), (5.0, 1000.0, 600.0, 24.0));
        assert_eq!((get("light_limit"), get("armor_limit"), get("bite_mark"), get("hit_teleport")), (5.0, 6.0, 100.0, 5.0));
        assert_eq!((get("side_left"), get("side_right"), get("lost_back"), get("strike_peak")), (80.0, -80.0, -800.0, 2.0));
        assert_eq!((get("accel_after"), get("glide_stop_accel"), get("weight"), get("walk_speed")), (0.2, 0.05, 1.0, 2.0));
        // Bone code 0 (bone 3) in the idle: about 96 above the actor.
        assert!((get("core_y") + 96.0).abs() < 5.0 && get("core_x").abs() < 10.0, "{} {}", get("core_x"), get("core_y"));
        let c = |r: u16| b.clips.iter().find(|c| c.request == r).unwrap().clone();
        // Re:CoM's bakes: the dash's wind-up and end, the idle; the dash and the bite get their own
        // clips (Re:CoM's records stay on Re:CoM's); the drop, the appearance, the notice are new.
        assert_eq!((c(201).name.as_str(), c(203).name.as_str(), c(0).name.as_str()), ("e023_050", "e023_052", "e023_001"));
        assert_eq!((c(202).slot, c(202).source, c(202).looped), (112, "kh1", true));
        assert_eq!((c(204).slot, c(204).root, c(200).slot, c(69).slot, c(205).slot), (114, true, 110, 108, 115));
        assert_eq!((c(200).frames, c(204).frames, c(202).frames), (100, 152, 60));
        // Re:CoM's 86 slots unchanged.
        let a0 = crate::ann::Ann::parse(&inp.ann).unwrap();
        let a1 = crate::ann::Ann::parse(&b.ann).unwrap();
        for i in 0..SPEC.recom_slots {
            assert_eq!(a0.slots[i], a1.slots[i], "slot {i}");
        }
        // ASD: Re:CoM's bytes kept, one record per attack (3), the tail moved.
        assert_eq!(&b.asd[..0x10], &inp.asd[..0x10]);
        assert_eq!(&b.asd[0x6C..0x1AC], &inp.asd[0x6C..0x1AC]);
        assert_eq!(u32::from_le_bytes(b.asd[0x10..0x14].try_into().unwrap()), 5);
        assert_eq!(&b.asd[0x1AC + 3 * 0xA0..], &inp.asd[0x1AC..]);
        let rec = |i: usize| &b.asd[0x1AC + i * 0xA0..0x1AC + (i + 1) * 0xA0];
        let win = |i: usize| (u32::from_le_bytes(rec(i)[0x5C..0x60].try_into().unwrap()), u32::from_le_bytes(rec(i)[0x60..0x64].try_into().unwrap()));
        let id = |i: usize| u32::from_le_bytes(rec(i)[0x64..0x68].try_into().unwrap());
        assert_eq!((win(0), id(0)), ((22 * 50, 74 * 50), 1), "the bite [22, 74)");
        assert_eq!((win(1), id(1)), ((0, 60 * 50), 0), "the dash all along");
        assert_eq!((win(2), id(2)), ((12 * 50, 28 * 50), 1), "the drop [12, 28)");
        assert_eq!((&rec(0)[0x40..0x48], &rec(1)[0x40..0x48], &rec(2)[0x40..0x48]), (&b"e023_114"[..], &b"e023_112"[..], &b"e023_110"[..]));
        // SES: Re:CoM's entries kept; the own clips get the bakes' sounds.
        let s0 = crate::ses::parse(&inp.ses).unwrap();
        let s1 = crate::ses::parse(&b.ses).unwrap();
        for e in &s0 {
            assert!(s1.iter().any(|x| x.name() == e.name() && x.events == e.events), "{}", e.name());
        }
        let ev = |n: &str| s1.iter().find(|e| e.name() == n).map(|e| e.events.clone());
        assert_eq!(ev("e023_112"), ev("e023_051"));
        assert_eq!(ev("e023_114"), ev("e023_053"));
        assert!(b.manifest.contains("\"enemy\": \"darkball\""));
        std::fs::write(std::env::temp_dir().join("kh1-darkball-test.json"), &b.manifest).unwrap();
        println!("{}\nparts {:?}", b.records.join("\n"), b.parts);
    }
}
