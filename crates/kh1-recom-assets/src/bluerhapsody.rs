//! M11: KH1's Blue Rhapsody in Re:CoM's battles (`docs/recom-m11-enemies.md` § The Blue Rhapsody): where its numbers sit
//! in the player's `ex_2120.bd` (`xa_ex_2120.mdls` part 5) and its child (part 3: the Blizzard `ex_2120_00.bd`), and
//! which Re:CoM files it extends (`ENEMY/0004`). The step itself is `crate::enemy`.
//!
//! The offsets are the scripts' own (Checked (files): the player's scripts disassembled with `tools/kh1-ai-script.py`,
//! the parent's code from `0x298`, the child's from `0x18`; the routine each sits in is named in
//! `kh1_core::bluerhapsody`). `walk_speed` / `run_speed` are the stats block's defaults (the flier never walks); the
//! script never sets a turn rate (`sys 125`): KH1's default π/16 a frame.
//!
//! Re:CoM's `EN0004.ASD` holds one record (`e004_051`, attack id 1, the antenna `ante2`, frames 75..120 of its state 3:
//! Checked (files)); KH1's melee 215 makes two rows (`0xEC` its hit, `0xEC`'s partner `0x15C` a type-2 body row that
//! never hurts): two new records on the melee's clip, on Re:CoM's id 1 (its only body attack). Its Blizzard (the child)
//! is one of Re:CoM's bullets on Re:CoM's own Blizzard shot's power (id 0, kind 1: its state 3 `0x14018C2D0`).

use crate::enemy::{AttackSpec, RowSpec, Spec};

/// Where each constant is pushed in `ex_2120.bd` (float or integer); a name with several offsets must read the same
/// at each.
pub const CONSTS: &[(&str, &[usize])] = &[
    ("walk_speed", &[]),
    ("run_speed", &[]),
    // Engaged `0x314`: under 250 the back-off and Blizzard `0x41A`; under 500 the hops and melee `0x3BC`; farther the
    // approach `0x3B6` → `0x666`. The hit state `0x2BC`: under 500 the back-off and Blizzard.
    ("close_range", &[0x366]),
    ("mid_range", &[0x380]),
    ("hit_close_range", &[0x2C0]),
    // `0x3BC`: the hops' side (`wp+0` = 0 under 0.5, else 1: the hop turned −20°), 60 % two hops (else one), the melee.
    ("hop_side_chance", &[0x3C6]),
    ("double_hop_chance", &[0x3FA]),
    // `0x4DE`: the hop: its facing turned ±20°, × 30, 60 high; the crouch 3 for its length, 4 (bit 16) and `sys 82` with
    // drag 0.9 over 12 frames, `sys 83(3)`; facing the target while in the air.
    ("hop_turn", &[0x500]),
    ("flip", &[0x524, 0x89A]),
    ("hop_speed", &[0x540]),
    ("hop_height", &[0x54E]),
    ("hop_drag", &[0x59E]),
    ("hop_time", &[0x5A4]),
    // `0x600`: the back-off: its facing turned 180°, × 35, 250 high, drag 0.9 over 40 frames (`0x18E2`), then to its height.
    ("back_turn", &[0x620]),
    ("back_speed", &[0x62C]),
    ("back_height", &[0x63A]),
    ("back_drag", &[0x64A]),
    ("back_time", &[0x650]),
    // `0x666`: the approach: up to 120 hops (20 ahead, 120 high, 0.9 over 25 frames) until within 750; a wall ahead
    // (`sys 92` = 1) a climb (5 ahead, 500 high, 1.0 over 60 frames).
    ("approach_count", &[0x66A]),
    ("approach_speed", &[0x6A6]),
    ("approach_height", &[0x6B4]),
    ("approach_drag", &[0x6C4]),
    ("approach_time", &[0x6CA]),
    ("climb_speed", &[0x708, 0x71C]),
    ("climb_height", &[0x72C]),
    ("climb_drag", &[0x73C]),
    ("climb_time", &[0x742]),
    ("approach_stop", &[0x75E]),
    // `0x78E`: to 35 above the floor under the target (`0x1386`: probed 20 above it), straight up or down:
    // `0x9DC(220, point, 6, 20, 0, 500, 300)`.
    ("rise_up", &[0x79A]),
    ("floor_probe", &[0x13A4]),
    ("rise_speed", &[0x7D4]),
    ("rise_stop", &[0x7DA]),
    ("rise_aim", &[0x7E0]),
    ("rise_high", &[0x7E6]),
    ("rise_time", &[0x7EC]),
    // The idle leg `0x85E`: 5° a frame, 0.3 down (up below 150 over the floor), [150, 180) frames, 219 at blend 8, 8 a
    // frame (the Red Nocturne's `0x93E`, the same bytes).
    ("wander_turn", &[0x86A]),
    ("wander_sink", &[0x874]),
    ("wander_low", &[0x884]),
    ("wander_min", &[0x8A8]),
    ("wander_max", &[0x8AE]),
    ("wander_blend", &[0x8D8]),
    ("wander_speed", &[0x8EC]),
    // `0x972`: the idle search (`0x1248(&wp30, 0, 1000, 2)`).
    ("idle_search", &[0x990]),
    // `0x9DC`'s own: "no motion" 999, the blend 8, the stuck factor 0.3; `0xEDA`'s 0.5.
    ("fly_no_motion", &[0xA68]),
    ("fly_blend", &[0xA7E]),
    ("fly_stuck", &[0xB5C]),
    ("stuck_half", &[0xF00]),
    // `0xBB4`: the stats block's `+0x14` (the flight's easing).
    ("air_accel_set", &[0xBCA]),
    // The notice `0xC3A` (a room-placed one): not in KH1's world 11; 214 at blend 8.
    ("notice_world", &[0xC3E]),
    ("notice_blend", &[0xC66]),
    // `0x14E6`: the request that also ends a wait (`0x14B2`).
    ("wait_motion", &[0x14F6]),
    // The appearance `0xDB4`: the 30-frame fade-in, 69 at blend 0, 219 queued at blend 8.
    ("appear_fade", &[0xDC2]),
    ("appear_blend", &[0xDFE]),
    ("appear_queue_blend", &[0xE1C]),
    // The Blizzard `0x424`: 55 at blend 0 from frame 0; the melee `0x47E`: 215 at blend 8 from frame 8; after either the
    // idle 220 at blend 0 from frame 2.
    ("cast_blend", &[0x44E]),
    ("cast_start", &[0x454]),
    ("melee_blend", &[0x4AE]),
    ("melee_start", &[0x4B4]),
    // The blend the host gives the motion a reaction queues (`Engine::attack_blend`): the melee's 8.
    ("attack_blend", &[0x4AE]),
    ("rest_blend", &[0x46A, 0x4CA]),
    ("rest_start", &[0x470, 0x4D0]),
    // The hit handler `0x1EFA`: a Fire hit's EXP bonus `EXP / 10 + 2` (`0xD52`) and dizzy 1800 (element: not ported).
    ("exp_div", &[0xD9C]),
    ("exp_add", &[0xDA4]),
    ("dizzy_time", &[0x1FB0]),
    // `sys 94`'s child index: the Blizzard (`0x424`).
    ("blizzard_child", &[0x43A]),
];

/// The Blizzard child's numbers (`ex_2120_00.bd`, entry `0x1C2`): its wait in 55 (84 frames, counted while the parent is
/// not stopped), its start `sys 19(parent, 0, 0)` (bone code 0 at t 0), its flight (`sys 18`: 25 a frame along the
/// parent's facing) and life (60). Checked (files).
pub const BLIZZARD_CONSTS: &[(&str, &[usize])] = &[("blizzard_wait", &[0x1E6]), ("blizzard_t", &[0x252]), ("blizzard_life", &[0x2EA]), ("blizzard_speed", &[0x30E])];

/// The Blizzard child's integer pushes: the cast effects 2054 / 2055 (`0x1CC`, `0x1DC`), the parent's 55 (`0x20C`), the
/// start's bone code 0 (`0x24C`), its effect object (`sys 15(…, 8, 20000)`: `0x2C2`, `0x2C8`), its row's mode 1 (`sys 213`,
/// `0x332`: on for its whole flight).
pub const BLIZZARD_REQUESTS: &[(u32, usize)] = &[(2054, 0x1CC), (2055, 0x1DC), (55, 0x20C), (0, 0x24C), (8, 0x2C2), (20000, 0x2C8), (1, 0x332)];

/// The request words the script pushes (bit 24 an action, bit 16 a held request, bit 30 a queue): the behaviour plays
/// exactly these. The leash's 70 is checked but not played.
pub const REQUESTS: &[(u32, usize)] = &[
    (55, 0x448),  // the Blizzard: 55, then the idle 220
    (220, 0x464),
    (215, 0x4A8), // the melee: 215, then 220
    (220, 0x4C4),
    (3, 0x55A),   // the hop: the crouch 3 (its length, `sys 83`'s base), then 4 with bit 16
    (3, 0x572),
    (0x10004, 0x582),
    (3, 0x5B0),
    (3, 0x656),   // the back-off, the approach's hop and climb (`0x18E2`'s motion 3)
    (3, 0x6D0),
    (3, 0x748),
    (220, 0x7CA), // to its height
    (219, 0x8D2), // the idle leg
    (214, 0xC60), // the notice, then 220
    (220, 0xC7C),
    (219, 0xDB4), // the appearance: 69 (an action), 219 queued
    (0x0100_0045, 0xDF8),
    (0x4000_0000, 0xE14),
    (6, 0x14F6),  // the request that ends a wait
    (0x4000_0046, 0x1AB4), // the leash's vanish (not ported)
];

/// KH1's melee 215 (`0x47E`: rows `0xEC` (the hit, type 0) and `0x15C` (a type-2 body row that never hurts), one hit
/// serial) on Re:CoM's only body attack, id 1 (`e004_051`'s record: decision M10's "power of the Re:CoM attack of the same
/// kind"); its body row on the same.
pub const MELEE: RowSpec = RowSpec { at: 0xEC, recom_id: 1 };
/// Melee 215's body row (`0x15C`), on the same Re:CoM attack.
pub const MELEE_BODY: RowSpec = RowSpec { at: 0x15C, recom_id: 1 };

/// Its one attack, in the behaviour's order (`kh1_core::bluerhapsody::Attack`).
pub const ATTACKS: &[AttackSpec] = &[AttackSpec { name: "melee", request: 215, rows: &[MELEE, MELEE_BODY] }];

/// The Blue Rhapsody.
pub const SPEC: Spec = Spec {
    name: "bluerhapsody",
    recom_id: 4,
    kh1: "xa_ex_2120",
    script: b"ex_2120.bd",
    prefix: "e004_",
    recom_slots: 75,
    asd: (1, 0x6C, 0x10C),
    consts: CONSTS,
    requests: REQUESTS,
    // The LV1 / LV2 stat records (`sys 45` with `0x14` / `0x80`).
    records: &[0x14, 0x80],
    attacks: ATTACKS,
    manifest: "kh1-bluerhapsody.json",
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
        let get = |n: &str| b.consts.iter().find(|c| c.0 == n).map(|c| c.1).unwrap_or_else(|| panic!("{n}"));
        // The script's numbers (the disassembly's).
        assert_eq!((get("close_range"), get("mid_range"), get("hit_close_range"), get("hop_side_chance"), get("double_hop_chance"), get("idle_search")), (250.0, 500.0, 500.0, 0.5, 0.6, 1000.0));
        assert_eq!((get("hop_turn"), get("flip"), get("hop_speed"), get("hop_height"), get("hop_drag"), get("hop_time")), (20.0, -1.0, 30.0, 60.0, 0.9, 12.0));
        assert_eq!((get("back_turn"), get("back_speed"), get("back_height"), get("back_drag"), get("back_time")), (180.0, 35.0, 250.0, 0.9, 40.0));
        assert_eq!((get("approach_count"), get("approach_speed"), get("approach_height"), get("approach_drag"), get("approach_time"), get("approach_stop")), (120.0, 20.0, 120.0, 0.9, 25.0, 750.0));
        assert_eq!((get("climb_speed"), get("climb_height"), get("climb_drag"), get("climb_time")), (5.0, 500.0, 1.0, 60.0));
        assert_eq!((get("rise_up"), get("floor_probe"), get("rise_speed"), get("rise_stop"), get("rise_aim"), get("rise_high"), get("rise_time")), (35.0, 20.0, 6.0, 20.0, 0.0, 500.0, 300.0));
        assert_eq!((get("wander_turn"), get("wander_sink"), get("wander_low"), get("wander_min"), get("wander_max"), get("wander_blend"), get("wander_speed")), (5.0, 0.3, 150.0, 150.0, 180.0, 8.0, 8.0));
        assert_eq!((get("fly_no_motion"), get("fly_blend"), get("fly_stuck"), get("stuck_half"), get("air_accel_set"), get("wait_motion")), (999.0, 8.0, 0.3, 0.5, 0.2, 6.0));
        assert_eq!((get("cast_blend"), get("cast_start"), get("melee_blend"), get("melee_start"), get("rest_blend"), get("rest_start")), (0.0, 0.0, 8.0, 8.0, 0.0, 2.0));
        assert_eq!((get("appear_fade"), get("appear_blend"), get("appear_queue_blend"), get("notice_world"), get("notice_blend")), (30.0, 0.0, 8.0, 11.0, 8.0));
        assert_eq!((get("exp_div"), get("exp_add"), get("dizzy_time"), get("blizzard_child")), (10.0, 2.0, 1800.0, 0.0));
        assert_eq!((get("walk_speed"), get("turn_rate"), get("weight")), (2.0, std::f32::consts::PI / 16.0, 1.0));
        // The child's: 84 frames in 55, from bone code 0's parent (t 0), 25 a frame for 60 frames; its row's radius 40,
        // Re:CoM's own Blizzard shot's power (id 0, kind 1); its start point on Re:CoM's bone, at the bone's origin.
        assert_eq!((get("blizzard_wait"), get("blizzard_t"), get("blizzard_speed"), get("blizzard_life")), (84.0, 0.0, 25.0, 60.0));
        assert_eq!((get("blizzard_radius"), get("blizzard_recom_id"), get("blizzard_kind")), (40.0, 0.0, 1.0));
        assert!(get("blizzard_at_bone") >= 0.0 && [get("blizzard_at_x"), get("blizzard_at_y"), get("blizzard_at_z")].iter().all(|v| v.abs() < 1e-4));
        // Every request the behaviour plays has a clip; the reactions' too; the status motions 0x66 / 0x69 none.
        for r in [0u16, 3, 4, 5, 6, 55, 69, 0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D, 0x4E, 0x50, 0x51, 214, 215, 216, 219, 220] {
            assert!(b.clips.iter().any(|x| x.request == r), "request {r}");
        }
        assert!(!b.clips.iter().any(|x| x.request == 0x66 || x.request == 0x69));
        let clip = |r: u16| b.clips.iter().find(|x| x.request == r).unwrap();
        assert_eq!((clip(55).frames, clip(215).frames, clip(3).frames, clip(69).frames, clip(216).frames), (148, 148, 16, 90, 52));
        let a0 = crate::ann::Ann::parse(&inp.ann).unwrap();
        let a1 = crate::ann::Ann::parse(&b.ann).unwrap();
        for i in 0..SPEC.recom_slots {
            assert_eq!(a0.slots[i], a1.slots[i], "slot {i}");
        }
        // The .ASD: Re:CoM's record kept, the melee's two new ones on its clip (the body row's never hurts: type 2).
        assert_eq!(&b.asd[..0x6C + 0xA0][0x6C..], &inp.asd[0x6C..0x6C + 0xA0]);
        assert_eq!(u32::from_le_bytes(b.asd[0x10..0x14].try_into().unwrap()), 3);
        assert_eq!(b.clash.len(), 1);
        assert_eq!(b.clash[0].iter().map(|c| (c.clash_type, c.priority)).collect::<Vec<_>>(), vec![(0, 30), (2, 30)]);
        assert!(!b.ses.is_empty(), "Re:CoM's EN0004.SES extended");
        assert!(b.manifest.contains("\"format\": \"kh1-enemy 1\"") && b.manifest.contains("\"enemy\": \"bluerhapsody\""));
        assert!(b.manifest.replace([' ', '\n'], "").contains("\"recoil\":216"), "the melee's recoil 216");
        std::fs::write(std::env::temp_dir().join("kh1-bluerhapsody-test.json"), &b.manifest).unwrap();
        std::fs::write(std::env::temp_dir().join("kh1-bluerhapsody-test.asd"), &b.asd).unwrap();
    }
}
