//! M11: KH1's Large Body in Re:CoM's battles (`docs/recom-m11-enemies.md` § The Large Body): where its numbers
//! sit in the player's `ex_2050.bd` (`xa_ex_2050.mdls` part 5) and its child `ex_2050_00.bd` (part 3, the stomp's
//! shockwave ring), and which Re:CoM files it extends (`ENEMY/0009`). The step itself is `crate::enemy`.
//!
//! The offsets are the scripts' own (Checked (files): the player's scripts disassembled with
//! `tools/kh1-ai-script.py`, code from `0x458`; the routine each sits in is named in `kh1_core::largebody`).

use crate::enemy::{AttackSpec, RowSpec, Spec};

/// Where each constant is pushed in `ex_2050.bd` (float or integer); a name with several offsets must read the
/// same at each.
pub const CONSTS: &[(&str, &[usize])] = &[
    // Init `0x20DC`: the stats block's walk / run (`+4` / `+8`), the turn rate (`sys 125(wp0, 8)`).
    ("walk_speed", &[0x20F6]),
    ("run_speed", &[0x2108]),
    ("turn_deg", &[0x2126]),
    // Supervisor `0x1FF8`: the idle search (`0x11B8(&wp30, 0, 1500, 1)`); its range in KH1's world 11.
    ("idle_search", &[0x2030]),
    ("world11_search", &[0x11EA]),
    // Engaged `0x53A` → `0x5C8`: close (< 250) the punch, mid (< 750) the slam, far the stomp and the slam.
    ("close_range", &[0x5D4]),
    ("mid_range", &[0x5F2]),
    // Idle `0x636` → the shared wander `0x1422(300, 400, &home)`.
    ("wander_inner", &[0x652]),
    ("wander_outer", &[0x658]),
    ("wander_band", &[0x1436, 0x145A]),
    ("wander_timeout", &[0x1484, 0x149E, 0x151E]),
    ("wander_idle_chance", &[0x14B4]),
    ("idle_blend", &[0x14CC]),
    ("wander_idle_min", &[0x14D4]),
    ("wander_idle_max", &[0x14DA]),
    ("wander_home_max", &[0x1502]),
    // The charge `0x6D8`: the wind-up 206, then 207 gliding at 12 for at most 180 frames at Sora, on past him to
    // a point 300–500 away once within 100, until within 100 of it or stuck (moved under 3.6 × step × 0.5);
    // then 208 (it hit) and 45–120 frames, else 209 and 30–150 frames.
    ("windup_blend", &[0x6F0]),
    ("charge_time", &[0x714]),
    ("charge_near", &[0x78E]),
    ("charge_past_min", &[0x7AA]),
    ("charge_past_max", &[0x7B0]),
    ("charge_arrive", &[0x7E0]),
    ("charge_stuck", &[0x7F0]),
    ("stuck_factor", &[0xE70]),
    ("charge_blend", &[0x816]),
    ("charge_speed", &[0x828]),
    ("landed_blend", &[0x8A6]),
    ("landed_then", &[0x8AC]),
    ("landed_min", &[0x8B6]),
    ("landed_max", &[0x8BC]),
    ("missed_blend", &[0x8D4]),
    ("missed_then", &[0x8DA]),
    ("missed_min", &[0x8E4]),
    ("missed_max", &[0x8EA]),
    // The stomp `0xA08`: 230 (blend 0, a plain request), 88 frames, the ring (the child), to 230's end.
    ("stomp_request", &[0xA20]),
    ("stomp_blend", &[0xA26]),
    ("ring_at", &[0xA34]),
    // The slam `0xA74`: action 231 (blend 8, the idle queued at 8), then 60 frames.
    ("slam_blend", &[0xA98]),
    ("slam_then", &[0xA9E]),
    ("slam_wait", &[0xAA8]),
    // The punch `0xAB4`: action 200 (blend 8, the idle queued at 8), then 75 frames.
    ("punch_blend", &[0xAF0]),
    ("punch_then", &[0xAF6]),
    ("punch_wait", &[0xB00]),
    // The positioning `0xB0C`: within 400 a point 400–600 away (120–150 frames); farther than 450 the approach
    // (`0x1CAC`: 240–300 frames, the target re-read every 5, to 350).
    ("near_range", &[0xB50]),
    ("away_min", &[0xB62]),
    ("away_max", &[0xB68]),
    ("away_time_min", &[0xB8A]),
    ("away_time_max", &[0xB90]),
    ("far_range", &[0xBA8]),
    ("approach_min", &[0xBBE]),
    ("approach_max", &[0xBC4]),
    ("approach_retarget", &[0xBCE]),
    ("approach_dist", &[0xBD4]),
    // The first look `0xC30` (214, blend 0; then the idle at 8) and the rage `0xCC2` (217 at 8; the idle at 8).
    ("look_blend", &[0xC96]),
    ("look_end_blend", &[0xCB2]),
    ("enrage_blend", &[0xCDC]),
    ("enrage_end_blend", &[0xCF8]),
    // The appearance `0x1B30`: 69 (blend 0), the 30-frame fade-in, 30 frames, paused with the idle queued (0).
    ("appear_blend", &[0x1B3E]),
    ("appear_fade", &[0x1B50]),
    ("appear_wait", &[0x1B76]),
    ("appear_then_blend", &[0x1B9C]),
    // The hit handler `0x21BE`: the first damaging hit that leaves it under 0.4 of its HP enrages it.
    ("enrage_hp", &[0x22D8, 0x2362]),
    // The hit state `0x47C`: fewer than 2 counted hits the punch, else the stomp (and the count back to 0).
    ("hit_stomp_count", &[0x4A4]),
    // The actions' idle queue (`0x15B2`'s third word: the punch's, the slam's).
    ("attack_blend", &[0xAF6, 0xA9E]),
];

/// The child's (`ex_2050_00.bd`, the stomp's ring; `crate::enemy::CHILD_CONSTS`) numbers: its life (28 frames,
/// `0x36C`), its radius growth (25 a frame, `0x42E`), the band a target must stand in (`[r − 100, r)`, `0x3CC`)
/// and the height window (`|dy| ≤ 50`, `0x3D2`). Checked (files).
pub const CHILD_CONSTS: &[(&str, &[usize])] = &[("ring_life", &[0x36C]), ("ring_growth", &[0x42E]), ("ring_band", &[0x3CC]), ("ring_height", &[0x3D2])];

/// The child's integer pushes the port relies on: its effect 7 (`0x310`), the attack row's flag 64 (no front cone,
/// `0x33C`).
pub const CHILD_REQUESTS: &[(u32, usize)] = &[(7, 0x310), (64, 0x33C)];

/// The request words the script pushes (bit 24 an action through `0x15B2`, bit 30 a queue): the behaviour
/// plays exactly these.
pub const REQUESTS: &[(u32, usize)] = &[
    (206, 0x6EA), // the charge's wind-up
    (207, 0x810), // the charge (sys 12 each frame), its test
    (207, 0x864),
    (208, 0x8A0), // after the charge: it hit / it didn't
    (209, 0x8CE),
    (230, 0xA20), // the stomp
    (231, 0xA92), // the slam
    (200, 0xAEA), // the punch
    (214, 0xC80), // the first look
    (217, 0xCD6), // the rage
    (0, 0xCF2),   // the idle after the rage
    (69, 0x1B38), // the appearance (a plain request)
    (0x4000_0000, 0x1B96),
    (0x0100_0000, 0x15C6), // `0x15B2`: bit 24, the idle queued
    (0x4000_0000, 0x15DE),
    // The hit handler `0x21BE`: the charge, the motions a knock-back doesn't interrupt (230, 231, 217).
    (207, 0x21F6),
    (207, 0x229C),
    (207, 0x23B8),
    (230, 0x244C),
    (231, 0x2462),
    (217, 0x247A),
];

/// The rows (script offsets) and the Re:CoM attack ids they take their power from (decision M10: Re:CoM's attack
/// power per attack; owner 2026-11-02: "the Re:CoM attack of the same kind"). Re:CoM's Large Body has two
/// (Checked (files): `EN0009.ASD`): id 0 on `e009_050` (its state 2, `0x14018F150`: a 1 m body record on
/// `ex_2050_sk`, frames 92–130, while its step moves it forward at 0.35, `0x14018F220`: its belly bump) and id 1 on
/// `e009_053` (its state 3, `0x14018F1A0`: a circle record, Re:CoM's type 11 `CCircleCollision`, at its root,
/// frames 90–120, whose radius its step grows by 0.3 a frame through `0x14014E5C0`: its stomp's shockwave). The
/// punch 200 (both hands, KH1 power 6, a flinch), the slam 231 (the belly, power 8, a knock-back) and the charge 207
/// (the belly, power 8, a knock-back) are body attacks: id 0. The stomp's ring is its shockwave: id 1 (§ Power).
const PUNCH_R: RowSpec = RowSpec { at: 0xEC, recom_id: 0 };
const PUNCH_L: RowSpec = RowSpec { at: 0x15C, recom_id: 0 };
const SLAM: RowSpec = RowSpec { at: 0x1CC, recom_id: 0 };
const CHARGE: RowSpec = RowSpec { at: 0x23C, recom_id: 0 };

/// `kh1_core::largebody::Attack::ALL`'s order. The stomp has no attack row of its own: its ring is the child's
/// row `0x31C` (`crate::enemy::RINGS`).
pub const ATTACKS: &[AttackSpec] = &[
    AttackSpec { name: "punch", request: 200, rows: &[PUNCH_R, PUNCH_L] },
    AttackSpec { name: "slam", request: 231, rows: &[SLAM] },
    AttackSpec { name: "charge", request: 207, rows: &[CHARGE] },
    AttackSpec { name: "stomp", request: 230, rows: &[] },
];

/// The Large Body.
pub const SPEC: Spec = Spec {
    name: "largebody",
    recom_id: 9,
    kh1: "xa_ex_2050",
    script: b"ex_2050.bd",
    prefix: "e009_",
    recom_slots: 75,
    asd: (2, 0x6C, 0x1AC),
    consts: CONSTS,
    requests: REQUESTS,
    // The LV1 / LV2 stat records (`0xD9E(0x14, 0x80)`).
    records: &[0x14, 0x80],
    attacks: ATTACKS,
    manifest: "kh1-largebody.json",
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
        assert_eq!((get("walk_speed"), get("run_speed"), get("turn_deg"), get("idle_search"), get("world11_search")), (3.5, 5.0, 8.0, 1500.0, 10000.0));
        assert_eq!((get("close_range"), get("mid_range"), get("near_range"), get("far_range"), get("approach_dist")), (250.0, 750.0, 400.0, 450.0, 350.0));
        assert_eq!((get("charge_time"), get("charge_speed"), get("charge_near"), get("charge_past_min"), get("charge_past_max"), get("charge_stuck")), (180.0, 12.0, 100.0, 300.0, 500.0, 3.6));
        assert_eq!((get("punch_wait"), get("slam_wait"), get("ring_at"), get("stomp_request"), get("enrage_hp"), get("hit_stomp_count")), (75.0, 60.0, 88.0, 230.0, 0.4, 2.0));
        // The child ring's numbers.
        assert_eq!((get("ring_life"), get("ring_growth"), get("ring_band"), get("ring_height")), (28.0, 25.0, 100.0, 50.0));
        // The clips: all KH1's (no exact Re:CoM bakes); its lengths; the charge loops, the stomp doesn't; no jump.
        let c = |r: u16| b.clips.iter().find(|c| c.request == r).unwrap().clone();
        assert_eq!((c(200).frames, c(206).frames, c(207).frames, c(208).frames, c(209).frames, c(230).frames, c(231).frames, c(217).frames, c(214).frames, c(69).frames), (158, 92, 24, 116, 296, 182, 272, 128, 92, 100));
        assert!(c(207).looped && !c(230).looped && c(231).root);
        assert!(b.clips.iter().all(|x| x.source == "kh1"));
        for r in [0, 1, 2, 6, 7, 72, 73, 74, 75, 76, 77, 82, 83, 84, 85, 86, 215, 216, 218] {
            assert!(b.clips.iter().any(|x| x.request == r), "request {r}");
        }
        assert!(!b.clips.iter().any(|x| x.request == 3 || x.request == 4 || x.request == 110), "no jump, no recoil 110");
        // Re:CoM's 75 slots unchanged.
        let a0 = crate::ann::Ann::parse(&inp.ann).unwrap();
        let a1 = crate::ann::Ann::parse(&b.ann).unwrap();
        for i in 0..SPEC.recom_slots {
            assert_eq!(a0.slots[i], a1.slots[i], "slot {i}");
        }
        // ASD: Re:CoM's 2 records kept, 4 for the punch's hands (two windows each), 1 slam, 1 charge, 1 ring; the tail
        // moved.
        assert_eq!(&b.asd[0x6C..0x1AC], &inp.asd[0x6C..0x1AC]);
        assert_eq!(u32::from_le_bytes(b.asd[0x10..0x14].try_into().unwrap()), 2 + 7);
        assert_eq!(&b.asd[0x1AC + 7 * 0xA0..], &inp.asd[0x1AC..]);
        let rec = |name: &str| -> Vec<(u32, u32, u32, u32)> {
            (2..9)
                .map(|i| &b.asd[0x6C + i * 0xA0..0x6C + (i + 1) * 0xA0])
                .filter(|r| r[0x40..0x50].starts_with(name.as_bytes()))
                .map(|r| (u32::from_le_bytes(r[0x5C..0x60].try_into().unwrap()), u32::from_le_bytes(r[0x60..0x64].try_into().unwrap()), u32::from_le_bytes(r[0x64..0x68].try_into().unwrap()), u32::from_le_bytes(r[0x54..0x58].try_into().unwrap())))
                .collect()
        };
        assert_eq!(rec("e009_124").len(), 4, "the punch: both hands, two windows each");
        assert_eq!(rec("e009_139"), vec![(102 * 50, 120 * 50, 0, 2)], "the slam on id 0");
        assert_eq!(rec("e009_126"), vec![(0, 24 * 50, 0, 2)], "the charge on id 0, every cycle");
        assert_eq!(rec("e009_138"), vec![(88 * 50, 116 * 50, 1, 4)], "the ring: Re:CoM's circle (shape 4), id 1, frames 88..116");
        let ring = &b.asd[0x6C + 8 * 0xA0..0x6C + 9 * 0xA0];
        assert_eq!(f32::from_le_bytes(ring[0x58..0x5C].try_into().unwrap()), 0.0);
        // Re:CoM's own record's place (its root), the 0.3 m lift dropped: the centre at the root (KH1's stomp spot).
        assert_eq!(&ring[0..0x18], &inp.asd[0x6C..0x6C + 0x18]);
        assert_eq!(&ring[0x1C..0x40], &inp.asd[0x6C + 0x1C..0x6C + 0x40]);
        assert_eq!(&ring[0..4], b"Root");
        assert!((f32::from_le_bytes(inp.asd[0x6C + 0x18..0x6C + 0x1C].try_into().unwrap()) - 0.3).abs() < 1e-6, "Re:CoM's 0.3 m up");
        assert_eq!(f32::from_le_bytes(ring[0x18..0x1C].try_into().unwrap()), 0.0);
        assert_eq!((f32::from_le_bytes(ring[0x14..0x18].try_into().unwrap()), f32::from_le_bytes(ring[0x1C..0x20].try_into().unwrap())), (0.0, 0.0));
        // The stomp has no root motion: its Root stays at the Large Body's own place for the ring's frames.
        assert!(!c(230).root);
        // SES: Re:CoM's entries kept; the ring's clip takes its template's sounds.
        let s0 = crate::ses::parse(&inp.ses).unwrap();
        let s1 = crate::ses::parse(&b.ses).unwrap();
        for (a, b) in s0.iter().zip(&s1) {
            assert_eq!((a.name(), &a.events), (b.name(), &b.events), "Re:CoM's entry kept");
        }
        // The front row: type 1, priority 100, recoil 110 (never played), 70° front (cos 0.342), mode 3 (no windows).
        assert!(b.manifest.contains(r#"{"name": "front", "row": "0x38c", "type": 1, "priority": 100, "recoil": 110, "slot": 0, "turn": 0.0, "cos": 0.34202, "windows": []}"#), "{}", b.manifest);
        assert!(b.manifest.contains(r#"{"name": "stomp", "request": 230, "recoil": 110, "rows": ["0x31c"], "clash": []}"#));
        assert!(b.manifest.contains("\"format\": \"kh1-enemy 1\"") && b.manifest.contains("\"enemy\": \"largebody\""));
        assert_eq!(b.parts.len(), 9);
        std::fs::write(std::env::temp_dir().join("kh1-largebody-test.json"), &b.manifest).unwrap();
        println!("{}", b.records.join("\n"));
        for e in &s1 {
            println!("ses {} {:?}", e.name(), e.events.iter().map(|e| i32::from_le_bytes(e[0x18..0x1C].try_into().unwrap())).collect::<Vec<_>>());
        }
    }
}
