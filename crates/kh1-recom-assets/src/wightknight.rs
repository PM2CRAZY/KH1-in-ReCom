//! M11: KH1's Wight Knight in Re:CoM's battles (`docs/recom-m11-enemies.md` § The Wight Knight): where its
//! numbers sit in the player's `ex_2200.bd` (`xa_ex_2200.mdls` part 5) and which Re:CoM files it extends
//! (`ENEMY/0012`). The step itself is `crate::enemy`.
//!
//! The offsets are the script's own (Checked (files): the player's script disassembled with
//! `tools/kh1-ai-script.py`, code from `0x618`; the routine each sits in is named in `kh1_core::wightknight`).

use crate::enemy::{AttackSpec, RowSpec, Spec};

/// Where each constant is pushed in `ex_2200.bd` (float or integer); a name with several offsets must read the
/// same at each.
pub const CONSTS: &[(&str, &[usize])] = &[
    // Init `0x23FE`: the stats block's walk (`+4` = 5); the run is never written (the stats-block default; the
    // script never runs).
    ("walk_speed", &[0x2456]),
    ("run_speed", &[]),
    // Engaged `0x7E2`: the first round (20 % the combo, 50 % the leap slash, else a forward leap and the leap
    // slash); then close (< 300) `0x95A` or far `0x910`.
    ("first_combo_chance", &[0x808]),
    ("first_leap_chance", &[0x822]),
    ("close_range", &[0x870]),
    // Far `0x910`: 20 % the forward leap and `0xD34`, else the circling walk `0xD96(0.6)` and the dance 212.
    ("far_hop_chance", &[0x91A]),
    ("far_circle_rate", &[0x940]),
    // Close `0x95A`: 50 % the back leap, then beyond 400 (30 % the leap slash, above 50 % the sway 218, else the
    // circling walk `0xD96(0.5)` and the forward leap), within (40 % the leap slash, above 80 % the sway, else the
    // combo); else the combo and the back leap.
    ("close_back_chance", &[0x964]),
    ("close_far", &[0x99A]),
    ("close_far_leap", &[0x9B2]),
    ("close_far_sway", &[0x9CC]),
    ("close_far_circle_rate", &[0x9E8]),
    ("close_near_leap", &[0xA14]),
    ("close_near_sway", &[0xA2E]),
    // The hit state `0x664`: beyond 400, 80 % the leap slash, else the forward leap; within, above 70 % the leap
    // slash, else a leap (above one half forward, else back) and, beyond 300, the leap slash, else the combo.
    ("hit_far", &[0x6B4]),
    ("hit_far_leap", &[0x6CC]),
    ("hit_leap_chance", &[0x704]),
    ("hit_fwd_chance", &[0x724]),
    ("hit_combo_range", &[0x772]),
    // `0xD34` (after the far forward leap): beyond 250, above one half the leap slash, else the dance; within,
    // the combo.
    ("after_hop_range", &[0xD4C]),
    ("after_hop_leap", &[0xD64]),
    // The appearance `0xC46`: 69 (a plain request, blend 0, the idle queued at 0) for its length − 4 frames,
    // turning to Sora; the 30-frame fade-in; then `0x2870`.
    ("appear_cut", &[0xC52]),
    ("appear_fade", &[0xC62]),
    ("appear_blend", &[0xCAE]),
    ("appear_queue_blend", &[0xCC6]),
    // `0x2870`: the idle at blend 8, then 6 frames.
    ("idle_blend", &[0x287A]),
    ("idle_wait", &[0x2882]),
    // The circling walk `0xD96(rate)`: [100, 200) frames walking (request 1, blend 16) at 5 along Sora's way
    // turned −90° (60 %, the rate as given) or +90° (the rate negated), the way turned by the rate each frame;
    // stuck (moved under 2.5 a frame): the stuck hop `0x266C`; within 100 of Sora after 50 frames: the back leap.
    ("circle_min", &[0xDB6]),
    ("circle_max", &[0xDBC]),
    ("circle_side_chance", &[0xE0A]),
    ("circle_right", &[0xE1C]),
    ("circle_left", &[0xE2C]),
    ("circle_blend", &[0xE66]),
    ("circle_speed", &[0xEA4]),
    ("circle_stuck", &[0xEDA]),
    ("circle_near", &[0xF4C]),
    ("circle_min_time", &[0xF60]),
    // The dance `0x103C(1)` (212 for 200 frames) and the sway `0x103C(0)` (218 for 150 frames, turning to Sora);
    // blend 16; within 200 of Sora after 80 frames: the back leap facing Sora.
    ("dance_time", &[0x1082]),
    ("sway_time", &[0x10B2]),
    ("dance_blend", &[0x1096, 0x10C6]),
    ("dance_near", &[0x120C]),
    ("dance_min_time", &[0x1220]),
    // The leaps `0x1284`: 205 (forward) / 206 (back) at blend 4 with 6 queued; movement state 7 with no velocity
    // (`sys 169`) from frame 20; then movement state 0 (`sys 88`). The far leap before the leap slash `0x1BAA`:
    // 205 the same way, from frame 22.
    ("hop_blend", &[0x130A, 0x135E, 0x1C16]),
    ("hop_queue_blend", &[0x1322, 0x1376, 0x1C2E]),
    ("hop_lift_at", &[0x13BC]),
    ("far_hop_lift_at", &[0x1C70]),
    // The combo `0x146E`: action 200 (blend 4, 201 queued); then swipes 201 / 202 (actions, blend 0) by turns
    // until a row of the last one landed (`wp+0x54` = 4) or Sora is beyond 350; then 203 / 204; beyond 350, the
    // leap slash.
    ("combo_blend", &[0x1896]),
    ("swipe_blend", &[0x167E, 0x17B6]),
    ("combo_end_blend", &[0x154A, 0x159A]),
    ("combo_far", &[0x16DE, 0x1816]),
    ("combo_max", &[0x14C8, 0x16B8, 0x17F0]),
    // The leap slash `0x18CA`: beyond 600 the far leap first; the crouch 207 (an action, blend 0, 208 queued);
    // 208 (bit 16) and movement state 3 (`sys 82`: Sora's way × distance × 0.035, height 500, drag 0.9, 30
    // frames; `sys 83(207)`), facing Sora now each frame in the air; on landing its rows and 216 (blend 0, the
    // idle queued) for its length − 2 − 8 frames.
    ("leap_far", &[0x18FE]),
    ("leap_height", &[0x198E]),
    ("leap_time", &[0x1998]),
    ("leap_speed_k", &[0x19A6]),
    ("leap_drag", &[0x1A52]),
    ("leap_crouch_blend", &[0x1A00]),
    ("leap_rise_blend", &[0x1A3C]),
    ("leap_cut_a", &[0x1ABC]),
    ("leap_cut_b", &[0x1AC4]),
    ("leap_slash_blend", &[0x1B3A]),
    // The stuck hop `0x266C`: the jump start 3 (an action, the idle queued), 4 (bit 16) and movement state 3
    // (`sys 82`: the facing × 20, height 400, drag 0.9, 50 frames; `sys 83(3)`).
    ("stuck_hop_blend", &[0x26E4]),
    ("stuck_hop_height", &[0x26A8]),
    ("stuck_hop_speed", &[0x26C6]),
    ("stuck_hop_drag", &[0x272C]),
    ("stuck_hop_time", &[0x2732]),
    // The queues' blend after the actions (203 / 204 and the jump start: `0x40000000` with blend 0; the blend the
    // host queues after a reaction, as every port's).
    ("attack_blend", &[0x1562, 0x15B2, 0x26FC]),
];

/// The request words the script pushes (bit 24 an action, bit 30 a queue, bit 16 a motion that holds): the
/// behaviour plays exactly these.
pub const REQUESTS: &[(u32, usize)] = &[
    (69, 0xC4A),          // the appearance (`sys 43`, then `sys 13`), the idle queued
    (69, 0xCA8),
    (0, 0xCC0),
    (1, 0xE60),           // the circling walk
    (212, 0x1090),        // the dance, the sway
    (218, 0x10C0),
    (212, 0x117A),
    (218, 0x11C2),
    (205, 0x12E4),        // the leaps, 6 queued
    (205, 0x1304),
    (6, 0x131C),
    (206, 0x1338),
    (206, 0x1358),
    (6, 0x1370),
    (0x0100_00CB, 0x1544), // the combo's ends 203 / 204, the idle queued
    (0x4000_0000, 0x155C),
    (0x0100_00CC, 0x1594),
    (0x4000_0000, 0x15AC),
    (0x0100_00CA, 0x1678), // swipe 202, 201 queued
    (0x4000_00C9, 0x1690),
    (0x0100_00C9, 0x17B0), // swipe 201, 202 queued
    (0x4000_00CA, 0x17C8),
    (0x0100_00C8, 0x1890), // the combo's start 200, 201 queued
    (0x4000_00C9, 0x18A8),
    (0x0100_00CF, 0x19FA), // the leap slash: crouch 207, 208 queued, 208 (bit 16), `sys 83(207)`, 216
    (0x4000_00D0, 0x1A12),
    (0x0001_00D0, 0x1A36),
    (207, 0x1A62),
    (216, 0x1AB4),
    (216, 0x1B34),
    (0, 0x1B4C),
    (205, 0x1BE6),        // the far leap before it
    (205, 0x1C10),
    (6, 0x1C28),
    (0x0100_0003, 0x26DE), // the stuck hop: 3 (action), the idle queued, 4 (bit 16), `sys 83(3)`
    (0x4000_0000, 0x26F6),
    (0x0001_0004, 0x2710),
    (3, 0x273E),
    (0, 0x2874),          // `0x2870`: the idle
];

/// The rows (script offsets) and the Re:CoM attack ids they take their power from (decision M10: Re:CoM's attack
/// power per attack; owner 2026-11-02: "the Re:CoM attack of the same kind"). Re:CoM's Wight Knight has two
/// (Checked (files): `EN0012.ASD`): id 0 on `e012_052` (its state 2, `0x140194FA0`: both hands at two strikes,
/// frames [0, 16) and [20, 45)) and id 1 on `e012_054` (its state 3: the wind-up `e012_053` turning to Sora, then
/// `e012_054`, `0x140194FF0` / `0x140195320`: both hands, the whole motion). The matcher pairs neither with a KH1
/// motion. `e012_054` is KH1's swipe 201 in Re:CoM's hands: its root goes 0.94 m forward by KH1 frame 42 and comes
/// back to 0.90 m by 66 (KH1's 201: 0.94 m at 42, 0.89 m at 68), after a wind-up as KH1's 200 (Checked (files): both
/// clips' roots read): the swipes (201, 202: the same strike from the other hand, kind 3, power 4) take id 1. The leap
/// slash 216 (two strikes, kind 3, power 2) takes id 0, Re:CoM's two-strike attack (Assumption: by elimination and
/// the strikes' count; owner question).
const SWIPE_A1: RowSpec = RowSpec { at: 0x1CC, recom_id: 1 };
const SWIPE_A2: RowSpec = RowSpec { at: 0x23C, recom_id: 1 };
const SWIPE_B1: RowSpec = RowSpec { at: 0x2AC, recom_id: 1 };
const SWIPE_B2: RowSpec = RowSpec { at: 0x31C, recom_id: 1 };
const LEAP_1: RowSpec = RowSpec { at: 0x38C, recom_id: 0 };
const LEAP_2: RowSpec = RowSpec { at: 0x3FC, recom_id: 0 };
const LEAP_3: RowSpec = RowSpec { at: 0x46C, recom_id: 0 };
const LEAP_4: RowSpec = RowSpec { at: 0x4DC, recom_id: 0 };

/// `kh1_core::wightknight::Attack::ALL`'s order.
pub const ATTACKS: &[AttackSpec] = &[
    AttackSpec { name: "swipe", request: 201, rows: &[SWIPE_A1, SWIPE_A2] },
    AttackSpec { name: "back swipe", request: 202, rows: &[SWIPE_B1, SWIPE_B2] },
    AttackSpec { name: "leap slash", request: 216, rows: &[LEAP_1, LEAP_2, LEAP_3, LEAP_4] },
];

/// The Wight Knight.
pub const SPEC: Spec = Spec {
    name: "wightknight",
    recom_id: 12,
    kh1: "xa_ex_2200",
    script: b"ex_2200.bd",
    prefix: "e012_",
    recom_slots: 75,
    asd: (6, 0xBC, 0x47C),
    consts: CONSTS,
    requests: REQUESTS,
    // The LV1 / LV2 stat records (`sys 45` with `0x14` / `0x80`).
    records: &[0x14, 0x80],
    attacks: ATTACKS,
    manifest: "kh1-wightknight.json",
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
        assert_eq!((get("walk_speed"), get("run_speed"), get("close_range"), get("first_combo_chance"), get("first_leap_chance")), (5.0, 8.0, 300.0, 0.8, 0.5));
        assert_eq!((get("leap_far"), get("leap_height"), get("leap_time"), get("leap_speed_k"), get("leap_drag"), get("leap_cut_a"), get("leap_cut_b")), (600.0, 500.0, 30.0, 0.035, 0.9, 2.0, 8.0));
        assert_eq!((get("circle_min"), get("circle_max"), get("circle_side_chance"), get("circle_right"), get("circle_left"), get("circle_speed"), get("circle_stuck")), (100.0, 200.0, 0.6, -90.0, 90.0, 5.0, 2.5));
        assert_eq!((get("dance_time"), get("sway_time"), get("dance_blend"), get("dance_near"), get("dance_min_time")), (200.0, 150.0, 16.0, 200.0, 80.0));
        assert_eq!((get("hop_blend"), get("hop_lift_at"), get("far_hop_lift_at"), get("combo_blend"), get("combo_far"), get("combo_max")), (4.0, 20.0, 22.0, 4.0, 350.0, 4.0));
        assert_eq!((get("hit_far"), get("hit_far_leap"), get("hit_leap_chance"), get("hit_fwd_chance"), get("hit_combo_range")), (400.0, 0.8, 0.7, 0.5, 300.0));
        assert_eq!((get("stuck_hop_height"), get("stuck_hop_speed"), get("stuck_hop_drag"), get("stuck_hop_time"), get("appear_cut"), get("appear_fade")), (400.0, 20.0, 0.9, 50.0, 4.0, 30.0));
        assert_eq!(get("weight"), 1.0);
        // The clips: all KH1's (no exact Re:CoM bake of an attack), the reactions, the leaps' root motion.
        let c = |r: u16| b.clips.iter().find(|c| c.request == r).unwrap().clone();
        assert_eq!((c(69).frames, c(200).frames, c(201).frames, c(202).frames, c(216).frames, c(217).frames), (112, 40, 70, 70, 96, 52));
        assert_eq!((c(201).root, c(205).root, c(206).root, c(216).root, c(217).root, c(200).root), (true, true, true, true, true, false));
        for r in [0, 1, 3, 4, 5, 6, 7, 69, 72, 73, 74, 75, 78, 79, 80, 81, 82, 83, 84, 85, 86, 200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 212, 216, 217, 218] {
            assert!(b.clips.iter().any(|x| x.request == r), "request {r}");
        }
        assert!(!b.clips.iter().any(|x| x.request == 2), "no run motion");
        for r in [200, 201, 202, 216, 217] {
            assert_eq!(c(r).source, "kh1", "{r}");
        }
        // Re:CoM's 75 slots unchanged.
        let a0 = crate::ann::Ann::parse(&inp.ann).unwrap();
        let a1 = crate::ann::Ann::parse(&b.ann).unwrap();
        for i in 0..SPEC.recom_slots {
            assert_eq!(a0.slots[i], a1.slots[i], "slot {i}");
        }
        // ASD: Re:CoM's 6 records kept; one per row's hit window (8: the swipes' two hands each, the leap slash's two
        // hands at two strikes); the tail moved.
        assert_eq!(&b.asd[0xBC..0x47C], &inp.asd[0xBC..0x47C]);
        assert_eq!(u32::from_le_bytes(b.asd[0x10..0x14].try_into().unwrap()), 6 + 8);
        assert_eq!(&b.asd[0x47C + 8 * 0xA0..], &inp.asd[0x47C..]);
        let rec = |name: &str| -> Vec<(u32, u32, u32)> {
            (6..14)
                .map(|i| &b.asd[0xBC + i * 0xA0..0xBC + (i + 1) * 0xA0])
                .filter(|r| r[0x40..0x50].starts_with(name.as_bytes()))
                .map(|r| (u32::from_le_bytes(r[0x5C..0x60].try_into().unwrap()), u32::from_le_bytes(r[0x60..0x64].try_into().unwrap()), u32::from_le_bytes(r[0x64..0x68].try_into().unwrap())))
                .collect()
        };
        assert_eq!(rec("e012_128"), vec![(20 * 50, 60 * 50, 1); 2], "the swipe 201's hands on id 1");
        assert_eq!(rec("e012_129"), vec![(20 * 50, 60 * 50, 1); 2], "the swipe 202's hands on id 1");
        assert_eq!(rec("e012_142"), vec![(2 * 50, 10 * 50, 0), (2 * 50, 10 * 50, 0), (24 * 50, 34 * 50, 0), (24 * 50, 34 * 50, 0)], "the leap slash on id 0");
        // SES: Re:CoM's entries kept, sounds on the attack clips (their template's at the hit window).
        let s0 = crate::ses::parse(&inp.ses).unwrap();
        let s1 = crate::ses::parse(&b.ses).unwrap();
        for (a, b) in s0.iter().zip(&s1) {
            assert_eq!((a.name(), &a.events), (b.name(), &b.events), "Re:CoM's entry kept");
        }
        for n in ["e012_128", "e012_129", "e012_142"] {
            assert!(s1.iter().any(|e| e.name() == n), "{n} has Re:CoM's sounds");
        }
        // The body rows: the combo's start at 100 (type 1), the swipes' and the leap slash's at 20 (type 2), all round.
        assert!(b.manifest.contains(r#"{"name": "windup", "row": "0xec", "type": 1, "priority": 100, "recoil": 110, "slot": 0, "turn": 0.0, "cos": -1.0, "windows": [{"request": 200, "on": [[6.0, 40.0]]}]}"#), "{}", b.manifest);
        assert!(b.manifest.contains(r#"{"name": "swipe", "row": "0x15c", "type": 2, "priority": 20, "recoil": 217, "slot": 2, "turn": 0.0, "cos": -1.0, "windows": [{"request": 201, "on": [[0.0, 70.0]]}, {"request": 202, "on": [[0.0, 70.0]]}]}"#));
        assert!(b.manifest.contains(r#"{"name": "leap", "row": "0x54c", "type": 2, "priority": 20, "recoil": 217, "slot": 1, "turn": 0.0, "cos": -1.0, "windows": [{"request": 216, "on": [[2.0, 34.0]]}]}"#));
        assert!(b.manifest.contains("\"format\": \"kh1-enemy 1\"") && b.manifest.contains("\"enemy\": \"wightknight\""));
        assert!(b.manifest.contains(r#"{"name": "leap slash", "request": 216, "recoil": 217"#));
        assert_eq!(b.parts.len(), 5);
        std::fs::write(std::env::temp_dir().join("kh1-wightknight-test.json"), &b.manifest).unwrap();
        println!("{}", b.records.join("\n"));
    }
}
