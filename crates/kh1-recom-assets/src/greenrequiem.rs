//! M11: KH1's Green Requiem in Re:CoM's battles (`docs/recom-m11-enemies.md` § The Green Requiem): where its numbers sit
//! in the player's `ex_2140.bd` (`xa_ex_2140.mdls` part 5) and its child (part 3: the Cure `ex_2140_00.bd`), and which
//! Re:CoM files it extends (`ENEMY/0006`). The step itself is `crate::enemy`.
//!
//! The offsets are the scripts' own (Checked (files): the player's scripts disassembled with `tools/kh1-ai-script.py`,
//! the parent's code from `0x1B8`, the child's from `0x18`; the routine each sits in is named in
//! `kh1_core::greenrequiem`). `walk_speed` / `run_speed` are the stats block's defaults (the flier never walks); the
//! script never sets a turn rate (`sys 125`): KH1's default π/16 a frame.
//!
//! Re:CoM has no `EN0006.SES` (Checked (files): the session's extraction holds none, as for enemy 3; the family note in
//! the doc): the step writes none ([`crate::enemy::NO_SES`]). Its `.ASD` holds no record (its one attack, state 2, is a
//! code heal) and the KH1 Green Requiem makes no attack row that hits Sora (its one row, `0xEC`, is its Cure's, applied
//! by its child straight to an ally): the `.ASD` is not rewritten.
//!
//! Its cast's two requests are played from frame 8 (`0x37E` `sys 13(actor, 212, 8, 8)`, `0x396` `sys 14(actor, 213, 8,
//! 8)`: the queue entry's start, added when the motion switches, `0x14029FC40`; Checked (decomp)), and only there: their
//! clips start at frame 8 ([`crate::enemy::STARTS`]).

use crate::enemy::{AttackSpec, Spec};

/// Where each constant is pushed in `ex_2140.bd` (float or integer); a name with several offsets must read the same
/// at each.
pub const CONSTS: &[(&str, &[usize])] = &[
    ("walk_speed", &[]),
    ("run_speed", &[]),
    // The round `0x280`: 80 % a Cure, then 30 % a move, then a Cure and a move.
    ("cure_chance", &[0x28A]),
    ("move_chance", &[0x2AA]),
    // The Cure `0x2CA`: a point [500, 600) away at a random angle (`0x1566`, two draws; KH1 never reads it), then the
    // pick (`0x1440(&wp30, 1, 5000, 1)`: `sys 102` with the search table's entry 1, within 5000, HP ratio ≤ 1).
    ("cure_point_min", &[0x2CE]),
    ("cure_point_max", &[0x2D4]),
    ("pick_entry", &[0x326]),
    ("pick_range", &[0x32C]),
    ("pick_ratio", &[0x332]),
    // `sys 94`'s child index (`0x364`), the cast 212 at blend 8 from frame 8, 213 queued at blend 8 from frame 8.
    ("cure_child", &[0x364]),
    ("cast_blend", &[0x384, 0x39C]),
    ("cast_start", &[0x38A, 0x3A2]),
    // The blend the host gives the motion a reaction queues (`Engine::attack_blend`): the scripts' 8.
    ("attack_blend", &[0x384]),
    // The cast's flight `0x3EA(999, target, 6, 10, 180, 180)`: to 180 above the floor under the target at 6 a frame; within
    // 10 a new point [600, 800) from the target at its own height.
    ("cast_speed", &[0x3BA]),
    ("cast_stop", &[0x3C0]),
    ("cast_up", &[0x3C6]),
    ("cast_high", &[0x3CC]),
    ("cast_point_min", &[0x47A]),
    ("cast_point_max", &[0x480]),
    // To its height `0x584`: 180 above the floor under the target (`0x11BC`, probed 20 above it), `0x812(2, point, 6,
    // 100, 0, 180, 300)`.
    ("rise_up", &[0x58C]),
    ("floor_probe", &[0x11DA]),
    ("rise_speed", &[0x5A6]),
    ("rise_stop", &[0x5AC]),
    ("rise_aim", &[0x5B2]),
    ("rise_high", &[0x5B8]),
    ("rise_time", &[0x5BE]),
    // The move `0x5CA`: a point [500, 600) from itself on the floor, `0x812(2, point, [6, 7.2), 10, 180, 180, 240)`.
    ("move_min", &[0x5CE]),
    ("move_max", &[0x5D4]),
    ("move_speed_min", &[0x604]),
    ("move_speed_max", &[0x60A]),
    ("move_stop", &[0x614]),
    ("move_up", &[0x61A]),
    ("move_high", &[0x620]),
    ("move_time", &[0x626]),
    // The idle leg `0x694`: 5° a frame, 0.3 down (up below 150 over the floor), [150, 180) frames, 219 at blend 8, 8 a
    // frame (the Red Nocturne's `0x93E` with its numbers in locals; its own bytes).
    ("wander_turn", &[0x6A0]),
    ("wander_sink", &[0x6AA]),
    ("wander_low", &[0x6BA]),
    ("flip", &[0x6D0]),
    ("wander_min", &[0x6DE]),
    ("wander_max", &[0x6E4]),
    ("wander_blend", &[0x70E]),
    ("wander_speed", &[0x722]),
    // `0x7A8`: the idle search (`0x107E(&wp30, 0, 1000, 2)`).
    ("idle_search", &[0x7C6]),
    // `0x812`'s own (the family's fly-to, the actor at `wp+0`): "no motion" 999, the blend 8, the stuck factor 0.3;
    // `0xD10`'s 0.5.
    ("fly_no_motion", &[0x89E, 0x3AA]),
    ("fly_blend", &[0x8B4]),
    ("fly_stuck", &[0x992]),
    ("stuck_half", &[0xD36]),
    // `0x9EA`: the stats block's `+0x14` (the flight's easing).
    ("air_accel_set", &[0xA00]),
    // The notice `0xA70` (a room-placed one): not in KH1's world 11; 214 at blend 8, then 220 at blend 0.
    ("notice_world", &[0xA74]),
    ("notice_blend", &[0xA9C]),
    // `0x131C`: the request that also ends a wait (`0x12E8`).
    ("wait_motion", &[0x132C]),
    // The appearance `0xBEA`: the 30-frame fade-in, 69 at blend 0, 219 queued at blend 8.
    ("appear_fade", &[0xBF8]),
    ("appear_blend", &[0xC34]),
    ("appear_queue_blend", &[0xC52]),
    // The hit handler `0x1CAA`: an attribute-8 hit's EXP bonus `EXP / 10 + 2` (`0xB88`; attributes: not ported).
    ("exp_div", &[0xBD2]),
    ("exp_add", &[0xBDA]),
];

/// The Cure child's numbers (`ex_2140_00.bd`, entry `0x2EE`): its wait in 213 (186 frames). Checked (files).
pub const CURE_CONSTS: &[(&str, &[usize])] = &[("cure_wait", &[0x35A])];

/// The Cure child's integer pushes: the charge effect 2052 (`0x2F8`), the parent's 212 / 213 (`0x30A`, `0x326`, `0x37E`),
/// the cast effects 2053 / 2054 (`0x340`, `0x350`), the thread's work word 7 (`0x3D6`), its effect object (`sys 15(…, 7,
/// 20000)`: `0x5E`, `0x64`) and the target's bone code 8 (`0xA0`) it follows.
pub const CURE_REQUESTS: &[(u32, usize)] = &[(2052, 0x2F8), (212, 0x30A), (213, 0x326), (2053, 0x340), (2054, 0x350), (213, 0x37E), (7, 0x3D6), (7, 0x5E), (20000, 0x64), (8, 0xA0)];

/// The request words the script pushes (bit 24 an action, bit 30 a queue): the behaviour plays exactly these. The
/// leash's 70 is checked but not played.
pub const REQUESTS: &[(u32, usize)] = &[
    (212, 0x37E), // the Cure: 212, 213 queued
    (213, 0x396),
    (2, 0x59C),   // to its height
    (2, 0x5FA),   // the move
    (219, 0x708), // the idle leg
    (214, 0xA96), // the notice, then 220
    (220, 0xAB2),
    (219, 0xBEA), // the appearance: 69 (an action), 219 queued
    (0x0100_0045, 0xC2E),
    (0x4000_0000, 0xC4A),
    (6, 0x132C),  // the request that ends a wait
    (0x4000_0046, 0x1858), // the leash's vanish (not ported)
];

/// The search table `sys 102` / `sys 48` read (`0x9EA` → `0x14B4(&wp30, actor, 0x190)`: 20-byte entries of percent
/// weights; Checked (files)): entry 1 (`0x1A4`), the pick's, is (60, 30, 10, 0, 0).
pub const SEARCH_TABLE: usize = 0x190;

/// No attack of its own: its one row (`0xEC`, the Cure, attribute 11) is its child's, applied to an ally (`sys 60`).
pub const ATTACKS: &[AttackSpec] = &[];

/// The Green Requiem.
pub const SPEC: Spec = Spec {
    name: "greenrequiem",
    recom_id: 6,
    kh1: "xa_ex_2140",
    script: b"ex_2140.bd",
    prefix: "e006_",
    recom_slots: 76,
    asd: (0, 0x6C, 0x6C),
    consts: CONSTS,
    requests: REQUESTS,
    // The LV1 / LV2 stat records (`sys 45` with `0x14` / `0x80`).
    records: &[0x14, 0x80],
    attacks: ATTACKS,
    manifest: "kh1-greenrequiem.json",
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enemy::{build, Inputs};

    /// The player's files: `KH1_M11_DIR` (KH1's `xa_*.mdls` / `.mset` and Re:CoM's `ENEMY__NNNN__ENNNNN.*`; no `.SES`
    /// for enemy 6) and `KH1_TEST_DATA` (`btltbl.bin`).
    fn inputs() -> Option<Inputs> {
        crate::enemy::test_inputs(&SPEC, false)
    }

    #[test]
    fn builds_from_the_players_files() {
        let Some(inp) = inputs() else { return };
        assert!(inp.ses.is_empty(), "Re:CoM has no EN0006.SES");
        let b = build(&SPEC, &inp).unwrap();
        let get = |n: &str| b.consts.iter().find(|c| c.0 == n).map(|c| c.1).unwrap_or_else(|| panic!("{n}"));
        println!("clips: {:?}", b.clips.iter().map(|c| (c.request, c.frames, c.root, c.looped, c.source)).collect::<Vec<_>>());
        // The script's numbers (the disassembly's).
        assert_eq!((get("cure_chance"), get("move_chance"), get("cure_point_min"), get("cure_point_max"), get("idle_search")), (0.8, 0.3, 500.0, 600.0, 1000.0));
        assert_eq!((get("pick_entry"), get("pick_range"), get("pick_ratio"), get("cure_child")), (1.0, 5000.0, 1.0, 0.0));
        assert_eq!((get("pick_weight_0"), get("pick_weight_1"), get("pick_weight_2"), get("pick_weight_3")), (60.0, 30.0, 10.0, 0.0));
        assert_eq!((get("cast_blend"), get("cast_start"), get("cast_speed"), get("cast_stop"), get("cast_up"), get("cast_high"), get("cast_point_min"), get("cast_point_max")), (8.0, 8.0, 6.0, 10.0, 180.0, 180.0, 600.0, 800.0));
        assert_eq!((get("rise_up"), get("floor_probe"), get("rise_speed"), get("rise_stop"), get("rise_aim"), get("rise_high"), get("rise_time")), (180.0, 20.0, 6.0, 100.0, 0.0, 180.0, 300.0));
        assert_eq!((get("move_min"), get("move_max"), get("move_speed_min"), get("move_speed_max"), get("move_stop"), get("move_up"), get("move_high"), get("move_time")), (500.0, 600.0, 6.0, 7.2, 10.0, 180.0, 180.0, 240.0));
        assert_eq!((get("wander_turn"), get("wander_sink"), get("wander_low"), get("flip"), get("wander_min"), get("wander_max"), get("wander_blend"), get("wander_speed")), (5.0, 0.3, 150.0, -1.0, 150.0, 180.0, 8.0, 8.0));
        assert_eq!((get("fly_no_motion"), get("fly_blend"), get("fly_stuck"), get("stuck_half"), get("air_accel_set"), get("wait_motion")), (999.0, 8.0, 0.3, 0.5, 0.2, 6.0));
        assert_eq!((get("appear_fade"), get("appear_blend"), get("appear_queue_blend"), get("notice_world"), get("notice_blend")), (30.0, 0.0, 8.0, 11.0, 8.0));
        assert_eq!((get("exp_div"), get("exp_add"), get("cure_wait")), (10.0, 2.0, 186.0));
        assert_eq!((get("walk_speed"), get("turn_rate"), get("weight")), (2.0, std::f32::consts::PI / 16.0, 1.0));
        // Its one row (`0xEC`, the Cure): power kind 0, attribute 11 (KH1's heal: `0x1402BEEE0` adds the record's number),
        // reaction kind 0, free-standing (bone −1).
        let script = crate::stats::mdls_part(&inp.mdls, 5).unwrap();
        let w = |o: usize| i32::from_le_bytes(script[0xEC + o..0xEC + o + 4].try_into().unwrap());
        assert_eq!((w(0), w(4), w(8), w(0x10)), (0, 11, 0, -1));
        // Every request the behaviour plays has a clip; the reactions' too; Re:CoM's 76 slots unchanged.
        for r in [0u16, 2, 69, 0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D, 0x4E, 0x50, 0x51, 212, 213, 214, 219, 220] {
            assert!(b.clips.iter().any(|x| x.request == r), "request {r}");
        }
        let clip = |r: u16| b.clips.iter().find(|x| x.request == r).unwrap();
        // The cast's two clips start at frame 8 (their manifest lengths the rest); the Cure acts 186 frames into 213.
        assert!(clip(213).frames as f32 > get("cure_wait"), "213 outlasts the Cure's wait");
        let a0 = crate::ann::Ann::parse(&inp.ann).unwrap();
        let a1 = crate::ann::Ann::parse(&b.ann).unwrap();
        for i in 0..SPEC.recom_slots {
            assert_eq!(a0.slots[i], a1.slots[i], "slot {i}");
        }
        // No new record: the .ASD stays Re:CoM's; no .SES.
        assert_eq!(b.asd, inp.asd);
        assert!(b.ses.is_empty());
        assert!(b.manifest.contains("\"format\": \"kh1-enemy 1\"") && b.manifest.contains("\"enemy\": \"greenrequiem\""));
        assert!(b.manifest.replace([' ', '\n'], "").contains("\"attacks\":[]"), "no attack of its own");
        std::fs::write(std::env::temp_dir().join("kh1-greenrequiem-test.json"), &b.manifest).unwrap();
        println!("parts {:?}", b.parts);
    }
}
