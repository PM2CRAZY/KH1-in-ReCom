//! M11: KH1's Pirate in Re:CoM's battles (`docs/recom-m11-enemies.md` § The Pirate): where its numbers
//! sit in the player's `ex_2100.bd` (`xa_ex_2100.mdls` part 5) and which Re:CoM files it extends
//! (`ENEMY/0021`). The step itself is `crate::enemy`.
//!
//! The offsets are the script's own (Checked (files): the player's script disassembled with
//! `tools/kh1-ai-script.py`; the routine each sits in is named in `kh1_core::pirate`).

use crate::enemy::{AttackSpec, RowSpec, Spec};

/// Where each constant is pushed in `ex_2100.bd` (`push`: float or integer). A name with several
/// offsets must read the same at each.
pub const CONSTS: &[(&str, &[usize])] = &[
    // Init `0x2960`: the stats block's walk / run (`+4` / `+8`).
    ("walk_speed", &[0x297A]),
    ("run_speed", &[0x298C]),
    // Supervisor `0x2824`: the idle search (`0x182E(&wp30, 0, 1000, 1)`) and the height window
    // (`dy` in [−100, 100]); `0x182E`'s range in KH1's world 11.
    ("idle_search", &[0x285C]),
    ("engage_dy_max", &[0x28A4]),
    ("engage_dy_min", &[0x28BC]),
    ("world11_search", &[0x1860]),
    // Engaged `0x6B4`.
    ("close_range", &[0x6C8]),
    ("mid_range", &[0x6E2]),
    // `0x870`: Sora on the ground above it: `dy` in [−300, −50] the leap, `dy` ≤ −400 back hop, run
    // in and the leap.
    ("above_near", &[0x8D6]),
    ("above_far", &[0x8EE]),
    ("above_high", &[0x90E]),
    // Close `0x828`, mid `0x7DA`, far `0x756`.
    ("close_slash_chance", &[0x832]),
    ("close_jump_chance", &[0x850]),
    ("mid_hop_chance", &[0x7E4]),
    ("mid_slash_chance", &[0x804]),
    ("far_leap_chance", &[0x760]),
    ("far_taunt_range", &[0x77A]),
    ("far_taunt_chance", &[0x792]),
    ("far_laugh_chance", &[0x7BA]),
    // Idle `0x946` → wander `0x1A48(100, 300, &home)`, then 60 % the taunt.
    ("wander_inner", &[0x946]),
    ("wander_outer", &[0x94C]),
    ("wander_band", &[0x1A5C, 0x1A80]),
    ("wander_timeout", &[0x1AAA, 0x1AC4, 0x1B44]),
    ("wander_idle_chance", &[0x1ADA]),
    ("wander_idle_min", &[0x1AFA]),
    ("wander_idle_max", &[0x1B00]),
    ("wander_home_max", &[0x1B28]),
    ("idle_blend", &[0x1AF2]),
    ("idle_taunt_chance", &[0x964]),
    // Hit `0x5CC`: within 300 the stance (30 %, while counted at most once) or the counter slash.
    ("hit_range", &[0x600]),
    ("hit_stance_max", &[0x612]),
    ("hit_stance_chance", &[0x62A]),
    // The counter slash `0x9C0` (action 216).
    ("counter_blend", &[0x9E4]),
    // The stance `0x9F6` / `0xA8C`: 208 at blend 6, raised again on a hit, for 120 frames, then the idle
    // at blend 12.
    ("stance_time", &[0xA08]),
    ("stance_end_blend", &[0xA7A]),
    ("stance_blend", &[0xAA6]),
    // The get-up slash `0xAD6` (235 / 236).
    ("getup_blend", &[0xB42]),
    // The leap `0xB54`: `0x20AE(facing × 12 with y 200, 0.9, 45, 200)`, the slam 218, then 90 % 213 or
    // the stuck blade (205 → 206 → 207).
    ("leap_speed", &[0xB78]),
    ("leap_height", &[0xB86]),
    ("leap_drag", &[0xB96]),
    ("leap_time", &[0xB9C]),
    ("slam_blend", &[0xBD4]),
    ("recover_chance", &[0xBF4]),
    ("recover_blend", &[0xC36]),
    ("stuck_blend", &[0xC66]),
    ("pull_blend", &[0xC96]),
    // The jump slash `0xCBE`: request 3 for its length, 4 with movement state 3 (`sys 82`: facing × 18
    // with y 12, 0.9, 60), the slash 210 from frame 8 with 6 queued; then the forward hop.
    ("jslash_speed", &[0xD14]),
    ("jslash_height", &[0xD22]),
    ("jslash_drag", &[0xD72]),
    ("jslash_time", &[0xD78]),
    ("jslash_blend", &[0xDF0]),
    // `sys 13`'s fourth word: the frame 210 starts from (`enemy::STARTS` makes its clip from there).
    ("jslash_start", &[0xDF6]),
    // The slash `0xE6C` (214), then the idle.
    ("slash_blend", &[0xEA4]),
    ("slash_end_blend", &[0xEC0]),
    // The spin `0xED0`: the wind-up 222 (from frame 8), the spin 217 gliding at 3.6 for 180 frames (a
    // random point 400–600 away when within 30), 15 frames more after a hit, then 219.
    ("spin_after_hit", &[0xED0]),
    ("windup_blend", &[0xEF2]),
    // The frame the wind-up 222 starts from (`enemy::STARTS`).
    ("windup_start", &[0xEF8]),
    ("spin_blend", &[0xF16, 0x108A]),
    ("spin_time", &[0xF28]),
    ("spin_speed", &[0x1018]),
    ("spin_near", &[0x104E]),
    ("spin_away_min", &[0x1060]),
    ("spin_away_max", &[0x1066]),
    ("dizzy_blend", &[0x10E8]),
    // The taunt `0x11A0` (220) and the laugh `0x11BC` (212).
    ("taunt_blend", &[0x11AA]),
    ("laugh_blend", &[0x11D4]),
    ("laugh_end_blend", &[0x11F0]),
    // The hops `0x157C(angle, speed, height, time, 3)`: forward `0x1200`, sideways `0x1238` (±15° at
    // random), back `0x1296` (then the run in, `0x23B0(1, 300, 20, 350)`); the drag of `sys 82`.
    ("fwd_hop_angle", &[0x1214]),
    ("fwd_hop_speed", &[0x121A]),
    ("fwd_hop_height", &[0x1220]),
    ("fwd_hop_time", &[0x1226]),
    ("side_hop_angle", &[0x1238]),
    ("side_hop_chance", &[0x124C]),
    ("side_hop_speed", &[0x1278]),
    ("side_hop_height", &[0x127E]),
    ("side_hop_time", &[0x1284]),
    ("back_hop_angle", &[0x12AA]),
    ("back_hop_speed", &[0x12B0]),
    ("back_hop_height", &[0x12B6]),
    ("back_hop_time", &[0x12BC]),
    ("approach_timeout", &[0x12DA]),
    ("approach_retarget", &[0x12E0]),
    ("approach_dist", &[0x12E6]),
    ("hop_drag", &[0x160E]),
    // The crouch's and the rise's blend in `0x157C`, `0x20AE` and the jump slash.
    ("hop_blend", &[0x15C4, 0x15F8, 0x20CA, 0x20FE, 0xD34, 0xD5C]),
    // Appearance `0x1B58`: the 30-frame fade-in, action 69 (bit 23) at blend 8.
    ("appear_fade", &[0x1B5C]),
    ("appear_blend", &[0x1B8A]),
    // The hit handler's own launch `0x25CA` (`sys 38(hit, 240 / 250, 100, 30, 0)`).
    ("own_launch_hop", &[0x2652]),
    ("own_launch_speed", &[0x2658]),
    ("own_launch_down", &[0x265E]),
    // The actions' idle queue (`0x1C9E`'s third word: the counter's, the get-up's, the taunt's, the
    // recovery's, the pull's; the blend the host queues after a reaction, as every port's).
    ("attack_blend", &[0x9EA, 0xB48, 0x11B0, 0xC3C, 0xC9C]),
];

/// The request words the script pushes (bit 24 an action, bit 23 held, bit 30 a queue, bit 16 a
/// motion that holds): the behaviour plays exactly these.
pub const REQUESTS: &[(u32, usize)] = &[
    (0x0180_0045, 0x1B84), // appearance 69 (action, held)
    (0x4000_0006, 0x1B9C), // request 6 queued after it
    (216, 0x9DE),          // the counter slash (`0x1C9E`: an action)
    (208, 0xAA0),          // the stance
    (235, 0xAE8),          // the get-up slashes
    (236, 0xB08),
    (200, 0xBA2),         // the leap's crouch (`0x20AE`)
    (0x0001_0000, 0x20F6), // `(r + 1) | 0x10000`
    (218, 0xBCE),         // the slam
    (213, 0xC30),         // the recovery
    (0x0100_00CD, 0xC60), // the blade stuck (action), the pull queued, the pull out
    (0x4000_00CE, 0xC78),
    (207, 0xC90),
    (3, 0xD2E),           // the jump slash's crouch, rise, `sys 83` base
    (0x0001_0004, 0xD56),
    (3, 0xD84),
    (0x0100_00D2, 0xDEA), // the jump slash (action), 6 queued
    (0x4000_0006, 0xE02),
    (214, 0xE9E),         // the slash
    (222, 0xEEC),         // the spin's wind-up, the spin, the dizzy end
    (217, 0xF10),
    (217, 0x1084),
    (219, 0x10E2),
    (220, 0x11A4),        // the taunt
    (212, 0x11CE),        // the laugh
    (3, 0x122C),          // the hops' request
    (3, 0x128A),
    (3, 0x12C2),
    (0x0001_0000, 0x15F0), // the hops' `(r + 1) | 0x10000`
    (240, 0x262A),        // its own launch: ground / air bases
    (250, 0x2638),
    // `0x267E`: the motions a damaging hit doesn't interrupt (`sys 42`).
    (216, 0x268E),
    (214, 0x26A4),
    (235, 0x26BC),
    (236, 0x26D4),
    (218, 0x26EC),
    (210, 0x2704),
    // `0x1C9E`: bit 24 and the idle queued.
    (0x0100_0000, 0x1CB2),
    (0x4000_0000, 0x1CCA),
];

/// The rows (script offsets) and the Re:CoM attack ids they take their power from (decision M10: Re:CoM's
/// attack power per attack; owner 2026-11-02: "the Re:CoM attack of the same kind"). Re:CoM's Pirate has
/// two (Checked (files): `EN0021.ASD`): id 0 on `e021_050` (its state 2, `0x140191000`; record kind index
/// 0, Re:CoM kind 1, the flinch) and id 1 on `e021_051` (its state 3, `0x140191040`; kind index 8, Re:CoM
/// kind 7). The matcher pairs `e021_050` with KH1's spin 217 and `e021_051` with KH1's slash 214 (Re:CoM's
/// edits of them at 30 fps: every bone but the root, hips and spine agrees; Checked (files)): those take
/// them. The others have no Re:CoM twin; by the kind of hit (their rows' `+8`, KH1's reaction on Sora, and
/// `+0x14`, KH1's power): the jump slash 210 and the get-up slashes 235 / 236 (kind 1, power 2, as the
/// spin's) take id 0; the counter 216 and the slam 218 (kind 4, power 4; the slash 214's kind 4, power 6)
/// take id 1 (Assumption: the nearest kind).
const SLASH: RowSpec = RowSpec { at: 0x23C, recom_id: 1 };
const COUNTER: RowSpec = RowSpec { at: 0x4DC, recom_id: 1 };
const JSLASH: RowSpec = RowSpec { at: 0x31C, recom_id: 0 };
const SLAM: RowSpec = RowSpec { at: 0x38C, recom_id: 1 };
const SPIN_A: RowSpec = RowSpec { at: 0xEC, recom_id: 0 };
const SPIN_B: RowSpec = RowSpec { at: 0x15C, recom_id: 0 };
const GETUP: RowSpec = RowSpec { at: 0x3FC, recom_id: 0 };

/// `kh1_core::pirate::Attack::ALL`'s order.
pub const ATTACKS: &[AttackSpec] = &[
    AttackSpec { name: "slash", request: 214, rows: &[SLASH] },
    AttackSpec { name: "counter", request: 216, rows: &[COUNTER] },
    AttackSpec { name: "jump slash", request: 210, rows: &[JSLASH] },
    AttackSpec { name: "slam", request: 218, rows: &[SLAM] },
    AttackSpec { name: "spin", request: 217, rows: &[SPIN_A, SPIN_B] },
    AttackSpec { name: "getup front", request: 235, rows: &[GETUP] },
    AttackSpec { name: "getup back", request: 236, rows: &[GETUP] },
];

/// The Pirate.
pub const SPEC: Spec = Spec {
    name: "pirate",
    recom_id: 21,
    kh1: "xa_ex_2100",
    script: b"ex_2100.bd",
    prefix: "e021_",
    recom_slots: 75,
    asd: (2, 0x6C, 0x1AC),
    consts: CONSTS,
    requests: REQUESTS,
    // The LV1 / LV2 stat records (`0x12F4(0x14, 0x80)`).
    records: &[0x14, 0x80],
    attacks: ATTACKS,
    manifest: "kh1-pirate.json",
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
        assert_eq!((get("walk_speed"), get("run_speed"), get("idle_search"), get("world11_search")), (3.0, 6.0, 1000.0, 10000.0));
        assert_eq!((get("engage_dy_max"), get("engage_dy_min"), get("close_range"), get("mid_range")), (100.0, -100.0, 250.0, 450.0));
        assert_eq!((get("above_near"), get("above_far"), get("above_high")), (-50.0, -300.0, -400.0));
        assert_eq!((get("leap_speed"), get("leap_height"), get("leap_drag"), get("leap_time"), get("recover_chance")), (12.0, 200.0, 0.9, 45.0, 0.9));
        assert_eq!((get("jslash_speed"), get("jslash_height"), get("jslash_time"), get("jslash_blend")), (18.0, 12.0, 60.0, 8.0));
        assert_eq!((get("spin_time"), get("spin_speed"), get("spin_after_hit"), get("spin_near"), get("spin_away_min"), get("spin_away_max")), (180.0, 3.6, 15.0, 30.0, 400.0, 600.0));
        assert_eq!((get("hit_range"), get("hit_stance_max"), get("hit_stance_chance"), get("stance_time"), get("stance_blend"), get("stance_end_blend")), (300.0, 1.0, 0.3, 120.0, 6.0, 12.0));
        assert_eq!((get("own_launch_hop"), get("own_launch_speed"), get("own_launch_down")), (100.0, 30.0, 0.0));
        assert_eq!((get("fwd_hop_speed"), get("fwd_hop_height"), get("fwd_hop_time"), get("side_hop_angle"), get("back_hop_angle"), get("hop_drag")), (30.0, 150.0, 40.0, 15.0, 180.0, 0.9));
        // The two requests played from a later frame are `enemy::STARTS`' (the script's own words).
        let starts = crate::enemy::starts_of("pirate");
        assert_eq!(starts, vec![(210, get("jslash_start") as u32), (222, get("windup_start") as u32)]);
        // The clips: all KH1's (Re:CoM's Pirate is a 30 fps edit: no exact bakes), the trios, its own
        // launch's layout, the reactions; the jump slash and the wind-up from frame 8; the spin looped.
        let c = |r: u16| b.clips.iter().find(|c| c.request == r).unwrap().clone();
        assert_eq!((c(214).frames, c(216).frames, c(217).frames, c(218).frames, c(235).frames, c(236).frames), (178, 110, 48, 52, 88, 64));
        assert_eq!((c(210).frames, c(222).frames, c(217).looped), (58, 122, true));
        for r in [0, 1, 2, 3, 4, 5, 6, 7, 69, 72, 73, 74, 75, 78, 79, 82, 86, 92, 200, 201, 202, 203, 205, 206, 207, 208, 209, 211, 212, 213, 215, 219, 220, 221, 240, 241, 242, 243, 244, 245, 249, 250] {
            assert!(b.clips.iter().any(|x| x.request == r), "request {r}");
        }
        assert!(b.clips.iter().all(|x| x.source == "kh1"));
        // Re:CoM's 75 slots unchanged.
        let a0 = crate::ann::Ann::parse(&inp.ann).unwrap();
        let a1 = crate::ann::Ann::parse(&b.ann).unwrap();
        for i in 0..SPEC.recom_slots {
            assert_eq!(a0.slots[i], a1.slots[i], "slot {i}");
        }
        // ASD: Re:CoM's 2 records kept, one per row's hit window (8: the slash, the counter, the jump
        // slash, the slam, the spin's two, the get-ups'), the tail moved; the jump slash's window starts
        // 8 frames earlier (its clip starts at 8).
        assert_eq!(&b.asd[0x6C..0x1AC], &inp.asd[0x6C..0x1AC]);
        assert_eq!(u32::from_le_bytes(b.asd[0x10..0x14].try_into().unwrap()), 2 + 8);
        assert_eq!(&b.asd[0x1AC + 8 * 0xA0..], &inp.asd[0x1AC..]);
        let rec = |name: &str| -> Vec<(u32, u32, u32)> {
            (2..10)
                .map(|i| &b.asd[0x6C + i * 0xA0..0x6C + (i + 1) * 0xA0])
                .filter(|r| r[0x40..0x50].starts_with(name.as_bytes()))
                .map(|r| (u32::from_le_bytes(r[0x5C..0x60].try_into().unwrap()), u32::from_le_bytes(r[0x60..0x64].try_into().unwrap()), u32::from_le_bytes(r[0x64..0x68].try_into().unwrap())))
                .collect()
        };
        assert_eq!(rec("e021_141"), vec![(20 * 50, 26 * 50, 0)], "the jump slash from frame 8");
        assert_eq!(rec("e021_148"), vec![(12 * 50, 18 * 50, 0), (36 * 50, 42 * 50, 0)], "the spin on id 0");
        assert_eq!(rec("e021_145"), vec![(122 * 50, 134 * 50, 1)], "the slash on id 1");
        assert_eq!((rec("e021_147")[0].2, rec("e021_149")[0].2, rec("e021_154")[0].2, rec("e021_155")[0].2), (1, 1, 0, 0));
        // SES: Re:CoM's two entries kept, sounds on the attack clips.
        let s0 = crate::ses::parse(&inp.ses).unwrap();
        let s1 = crate::ses::parse(&b.ses).unwrap();
        for (a, b) in s0.iter().zip(&s1) {
            assert_eq!((a.name(), &a.events), (b.name(), &b.events), "Re:CoM's entry kept (its offset word is the writer's)");
        }
        for n in ["e021_145", "e021_148"] {
            assert!(s1.iter().any(|e| e.name() == n), "{n} has Re:CoM's sounds");
        }
        // The guard rows: the spin's body 90 / 221, the stance's blade 30 / 209, both type 2, the front
        // half, on through their motion.
        assert!(b.manifest.contains(r#"{"name": "spin", "row": "0x1cc", "type": 2, "priority": 90, "recoil": 221, "slot": 2, "turn": 0.0, "cos": 0.0, "windows": [{"request": 217, "on": [[0.0, 48.0]]}]}"#), "{}", b.manifest);
        assert!(b.manifest.contains(r#"{"name": "stance", "row": "0x46c", "type": 2, "priority": 30, "recoil": 209, "slot": 0, "turn": 0.0, "cos": 0.0, "windows": [{"request": 208, "on": [[0.0, 48.0]]}]}"#));
        assert!(b.manifest.contains("\"format\": \"kh1-enemy 1\"") && b.manifest.contains("\"enemy\": \"pirate\""));
        assert_eq!(b.parts.len(), 7);
        std::fs::write(std::env::temp_dir().join("kh1-pirate-test.json"), &b.manifest).unwrap();
        println!("{}", b.records.join("\n"));
    }
}
