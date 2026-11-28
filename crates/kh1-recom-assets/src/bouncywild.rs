//! M11: KH1's Bouncywild in Re:CoM's battles (`docs/recom-m11-enemies.md` § The Bouncywild): where its numbers sit in
//! the player's `ex_2040.bd` (`xa_ex_2040.mdls` part 5) and its three children (part 3: the thrown banana
//! `ex_2040_00.bd`, the tossed peel `ex_2040_01.bd`, the dropped peel `ex_2040_02.bd`), and which Re:CoM files it
//! extends (`ENEMY/0008`). The step itself is `crate::enemy`.
//!
//! The offsets are the scripts' own (Checked (files): the player's scripts disassembled with
//! `tools/kh1-ai-script.py`, the parent's code from `0x300`, the children's from `0x18`; the routine each sits in is
//! named in `kh1_core::bouncywild`).

use crate::enemy::{AttackSpec, Spec};

/// Where each constant is pushed in `ex_2040.bd` (float or integer); a name with several offsets must read the same
/// at each.
pub const CONSTS: &[(&str, &[usize])] = &[
    // Init `0x1EBC`: the stats block's walk (`+4` = 5), run (`+8` = 13) and hop (`+0x10` = 500, unused).
    ("walk_speed", &[0x1ED6]),
    ("run_speed", &[0x1EE8]),
    ("hop_height", &[0x1EFA]),
    // The supervisor `0x1DD8`: while idle, Sora within 1500 (`0x1050(&wp30, 0, 1500, 4)`) → engaged.
    ("idle_search", &[0x1E10]),
    // The appearance `0x1A2A`: 69 (`sys 13`, blend 0), the fade-in 30, flag 0x80 for 30 frames, 30 frames, then flag 4
    // (the script waits until 69 ends) with the idle queued (blend 0).
    ("appear_blend", &[0x1A38]),
    ("appear_fade", &[0x1A4A]),
    ("appear_wait", &[0x1A70]),
    ("appear_then_blend", &[0x1A96]),
    // The idle `0x50A`: the wander `0x126A(350, 450, &home)`; the idle (`sys 12`, blend 12) for [120, 180) frames; the
    // stretch 216 (`0x13FA`: an action, blend 8, the idle queued at 8).
    ("wander_inner", &[0x50A]),
    ("wander_outer", &[0x510]),
    ("rest_blend", &[0x528]),
    ("rest_min", &[0x530]),
    ("rest_max", &[0x536]),
    ("stretch_blend", &[0x826]),
    // The wander `0x126A`: points [inner − 100, inner) and [outer − 100, outer) about where it stands, each walked to
    // (mode 0) for at most 300 frames; 40 %: the idle (blend 8) for [90, 180) frames; farther than 700 from home: home.
    ("wander_band", &[0x127E, 0x12A2]),
    ("wander_timeout", &[0x12CC, 0x12E6, 0x1366]),
    ("wander_idle_chance", &[0x12FC]),
    ("wander_idle_blend", &[0x1314]),
    ("wander_idle_min", &[0x131C]),
    ("wander_idle_max", &[0x1322]),
    ("wander_home_max", &[0x134A]),
    // Engaged `0x3C0`: five rounds (`wp+0x44`); close (< 400) the leap and the run-away; mid (< 800) 70 % the peel toss
    // else the taunt; far 80 % the banana else the call and the run-away; then the look-about 215.
    ("rounds", &[0x404]),
    ("close_range", &[0x430]),
    ("mid_range", &[0x44A]),
    ("far_banana_chance", &[0x48C]),
    ("mid_peel_chance", &[0x4BA]),
    // The look-about `0x59E`: 215 (`sys 13`, blend 8) to its end; Sora within 300: the leap and the run-away.
    ("ready_blend", &[0x5A8]),
    ("ready_close", &[0x5D6]),
    // The call `0x600`: 206 (action, blend 8); the room's helpers (`sys 37(0xB0000 | 2)`, none in Re:CoM's battles):
    // 207 (action, blend 12).
    ("call_blend", &[0x620]),
    ("call_end_blend", &[0x720]),
    // The taunt `0x736`: 213 (blend 8) to its end, cut after 90 frames when Sora is within 800; then `0x7BE`: 217
    // (blend 8) to its end, the idle (blend 8), the escape.
    ("taunt_blend", &[0x740]),
    ("taunt_min", &[0x74E]),
    ("taunt_range", &[0x778]),
    ("taunt_end_blend", &[0x7CC]),
    // The recovery after its own launch `0x838`: 212 (action, blend 8).
    ("recover_blend", &[0x83E]),
    // The turn after a near hit `0x850`: 205 (blend 8).
    ("turn_blend", &[0x86E]),
    // The first look `0x89A` (a placed one only): 209 (blend 8).
    ("notice_blend", &[0x8A8]),
    // The banana `0x952`: 211 (blend 8), the idle queued at 8.
    ("throw_blend", &[0x98C]),
    // The peel toss `0x9B8`: one peel (`+24` = 1), 200 (`sys 13`, blend 0).
    ("peel_count", &[0x9FC]),
    ("toss_blend", &[0xA1C]),
    // The bounce-about `0xA30`: 202 (blend 8) to its end or Sora within 300; eight legs, each to a point [250, 300)
    // away at a random angle, 203 (blend 8) gliding at 13 (stats `+0xC`, `sys 40` mode 2) for at most 180 frames, until
    // stuck or within 100; then 204 (action, blend 0, the idle queued at 8) and 8 frames.
    ("dance_blend", &[0xA48]),
    ("dance_close", &[0xA76]),
    ("dance_legs", &[0xA96]),
    ("dance_min", &[0xAB6]),
    ("dance_max", &[0xABC]),
    ("dance_leg_time", &[0xB14]),
    ("dance_fx_every", &[0xB26]),
    ("bounce_blend", &[0xB50]),
    ("bounce_speed", &[0xB62]),
    ("stuck_speed", &[0xB9C, 0xCBC]),
    ("arrive_dist", &[0xBB0, 0xCD0]),
    ("pose_wait", &[0xBFA, 0xD0A]),
    // The run-away `0xC24`: a point 800 straight away from Sora; run there for 10 frames, drop a peel, run on for at most
    // 180 frames until stuck or within 100, drop another, 204 and 8 frames.
    ("flee_dist", &[0xC4A]),
    ("flee_first", &[0xC6C]),
    ("flee_time", &[0xC8A]),
    ("drop_count", &[0xD40]),
    // The leap `0xD4A`: facing Sora now, `0x180A(actor, facing × 33 with height 130, 0.9, 50, 3)`: request 3 for its
    // length, then 4 and movement state 3 (`sys 82`: speed 33, height 130, drag 0.9, 50 frames), `sys 83` base 3.
    ("leap_speed", &[0xD6A]),
    ("leap_height", &[0xD78]),
    ("leap_drag", &[0xD88]),
    ("leap_time", &[0xD8E]),
    // The stuck test `0xE4C`: moved under arg × step × 0.5.
    ("stuck_factor", &[0xE72]),
    // The hit state `0x324`: within 400 the turn and the run-away, else the look-about.
    ("hit_range", &[0x360]),
    // The hit handler `0x1FAC`: a living hit of kind 4 / 5: its own launch (`sys 38(actor, hit, 0, 350, 35, 0)`: base 0
    // = KH1's launch 0x52 / air 0x5C, hop 350, speed 35, no down time).
    ("own_launch_hop", &[0x2074]),
    ("own_launch_speed", &[0x207A]),
    ("own_launch_down", &[0x2080]),
    // `sys 94`'s child indices: the banana (0, `0x95A`), the tossed peel (1, `0x9D8`), the dropped peel (2, `0xD1C`).
    ("fire_child", &[0x95A]),
    ("peel_child", &[0x9D8]),
    ("drop_child", &[0xD1C]),
    // The blend the host gives a reaction's queued motion and an action's queued idle (KH1's 8: `0x13FA`'s queue).
    ("attack_blend", &[0x82C, 0x844, 0xBF0, 0xD00]),
];

/// The request words the script pushes (bit 24 an action, bit 30 a queue): the behaviour plays exactly these.
pub const REQUESTS: &[(u32, usize)] = &[
    (69, 0x1A32),          // the appearance
    (0x4000_0000, 0x1A90), // its idle queued
    (0x4000_0046, 0x19DC), // the escape's 70 (not ported: owner question)
    (215, 0x5A2),          // the look-about
    (206, 0x61A),          // the call (`0x13FA`: | 0x1000000 at `0x140E`)
    (207, 0x71A),
    (213, 0x73A),          // the taunt
    (217, 0x7C6),
    (208, 0x808),          // the slip on a peel (message 1001: not ported)
    (216, 0x820),          // the stretch
    (212, 0x838),          // the recovery
    (205, 0x868),          // the turn
    (209, 0x8A2),          // the first look
    (211, 0x986),          // the banana
    (200, 0xA16),          // the peel toss
    (202, 0xA42),          // the bounce-about
    (203, 0xB4A),
    (204, 0xBE4),          // the pose
    (204, 0xCF4),
    (3, 0xD94),            // the leap's base (`0x180A`: 3, then 4 | 0x10000)
    (0x0100_0000, 0x140E), // `0x13FA`'s action bit
    (0x4000_0000, 0x1426), // and its queued idle
];

/// The banana `ex_2040_00.bd` (entry `0x5E6`; its routines `0x18..0x5E6` byte for byte the Fat Bandit's and the
/// Defender's fireball's, Checked (files)): effects 2062 / 2060; 82 frames; ends unless the parent plays 211; its start
/// `sys 19(parent, 28, 1)` (bone code 28 at t 1); `0x18(…, model 13, life 75, fade 10)`, `0x364(…, turn 1.25, speed
/// 25)`: along the parent's facing, turning at most 1.25° a frame toward its target's lock point, 75 frames; a hit ends
/// it; a repel re-aims it (`0x1B8`) with the row `0x15C`. Checked (files).
pub const FIRE_CONSTS: &[(&str, &[usize])] = &[("fire_wait", &[0x60A]), ("fire_t", &[0x63C]), ("fire_turn", &[0x654]), ("fire_speed", &[0x65A]), ("fire_life", &[0x66E]), ("fire_fade", &[0x674])];

/// The banana's integer pushes: the parent's request it checks (211, `0x61C`), its bone code 28 (`0x636`), its model 13
/// (`0x668`).
pub const FIRE_REQUESTS: &[(u32, usize)] = &[(211, 0x61C), (28, 0x636), (13, 0x668)];

/// The tossed peel `ex_2040_01.bd` (entry `0x502`): effect 2057; 80 frames; ends unless the parent plays 200; from
/// `sys 19(parent, 28, 1)` along the parent's facing, `+24` peels (1), each turned by the running angle (0, then ±5 ×
/// rand [1.5, 2.5) alternating), at rand [8, 10) a frame with rand [10, 15) up. Each peel (`0x18` → `0x96`, the same
/// bytes as the dropped one's): falls (its speed's y + 0.8 × step a frame) through a radius-30 sweep (`sys 140`) until
/// it touches the ground; then lies (effect 11) for 120 frames, testing each frame each party member that can be hit
/// (`sys 141`), stands on the ground (`+0x70` = 0) and has no flag 0x400: within 50 of the peel, the slip (`0x29A`: Sora
/// takes request 200 of the Bouncywild's player set as an action, the idle queued at 8, flag 0x80 for 30 frames, loses
/// the munny of the parent's table `0x2A4` and drops it as prizes there; an enemy gets message 1001) and the peel ends.
/// Checked (files).
pub const PEEL_CONSTS: &[(&str, &[usize])] = &[
    ("peel_sweep", &[0xB4]),
    ("peel_gravity", &[0xF4]),
    ("peel_lie", &[0x130]),
    ("peel_reach", &[0x234]),
    ("peel_protect", &[0x2F2]),
    ("peel_wait", &[0x51E]),
    ("peel_t", &[0x550]),
    ("peel_spread", &[0x574]),
    ("peel_speed_min", &[0x5B4]),
    ("peel_speed_max", &[0x5BA]),
    ("peel_up_min", &[0x5CC]),
    ("peel_up_max", &[0x5D2]),
    ("peel_spread_min", &[0x604]),
    ("peel_spread_max", &[0x60A]),
];

/// The tossed peel's integer pushes: the parent's request (200, `0x530`), its bone code 28 (`0x54A`), Sora's slip
/// request (`0x10000C8`, `0x2C2`), the flag test 0x400 (`0x200`).
pub const PEEL_REQUESTS: &[(u32, usize)] = &[(200, 0x530), (28, 0x54A), (0x0100_00C8, 0x2C2), (0x400, 0x200)];

/// The dropped peel `ex_2040_02.bd` (entry `0x502`, its peel the tossed one's bytes): from `sys 19(parent, 28, 1)` at
/// once, toward the parent's target (the way from the hand to its position, normalised), `+24` peels (1), at rand
/// [3, 10) a frame with rand [5, 10) up. Checked (files).
pub const DROP_CONSTS: &[(&str, &[usize])] = &[("drop_t", &[0x51A]), ("drop_speed_min", &[0x590]), ("drop_speed_max", &[0x596]), ("drop_up_min", &[0x5A8]), ("drop_up_max", &[0x5AE])];

/// The dropped peel's integer pushes: its bone code 28 (`0x514`).
pub const DROP_REQUESTS: &[(u32, usize)] = &[(28, 0x514)];

/// No attack of its own makes a row on its body: KH1's Bouncywild has no HIT track (Checked (files)); its banana is its
/// child's (a Re:CoM bullet, the host's `fire_*` numbers), its peels hit nothing (the slip). The banana's row `0xEC` the
/// parent also makes with 211 (`sys 23` with a hit bit) and the peel toss's `0x1CC` (type 6, radius 0) with 200 have no
/// bone and no window: nothing to build (Assumption: they only hold the hit bit).
pub const ATTACKS: &[AttackSpec] = &[];

/// The Bouncywild.
pub const SPEC: Spec = Spec {
    name: "bouncywild",
    recom_id: 8,
    kh1: "xa_ex_2040",
    script: b"ex_2040.bd",
    prefix: "e008_",
    recom_slots: RECOM_SLOTS,
    asd: (0, 0x6C, 0x6C),
    consts: CONSTS,
    requests: REQUESTS,
    // The LV1 / LV2 stat records (`sys 45` with `0x14` / `0x80`).
    records: &[0x14, 0x80],
    attacks: ATTACKS,
    manifest: "kh1-bouncywild.json",
};

/// Re:CoM's `EN0008.ANN` slot count (Checked (files)).
pub const RECOM_SLOTS: usize = 75;

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
        std::fs::write(std::env::temp_dir().join("kh1-bouncywild-test.json"), &b.manifest).unwrap();
        // The script's numbers (the disassembly's).
        assert_eq!((get("walk_speed"), get("run_speed"), get("idle_search"), get("rounds"), get("close_range"), get("mid_range")), (5.0, 13.0, 1500.0, 5.0, 400.0, 800.0));
        assert_eq!((get("leap_speed"), get("leap_height"), get("leap_drag"), get("leap_time"), get("flee_dist"), get("bounce_speed")), (33.0, 130.0, 0.9, 50.0, -800.0, 13.0));
        assert_eq!((get("own_launch_hop"), get("own_launch_speed"), get("own_launch_down"), get("fire_child"), get("peel_child"), get("drop_child")), (350.0, 35.0, 0.0, 0.0, 1.0, 2.0));
        // The children's.
        assert_eq!((get("fire_wait"), get("fire_t"), get("fire_turn"), get("fire_speed"), get("fire_life"), get("fire_fade"), get("fire_radius")), (82.0, 1.0, 1.25, 25.0, 75.0, 10.0, 50.0));
        assert_eq!((get("peel_sweep"), get("peel_gravity"), get("peel_lie"), get("peel_reach"), get("peel_wait"), get("peel_t"), get("drop_t")), (30.0, 0.8, 120.0, 50.0, 80.0, 1.0, 1.0));
        assert_eq!((get("peel_speed_min"), get("peel_speed_max"), get("peel_up_min"), get("peel_up_max"), get("drop_speed_min"), get("drop_speed_max"), get("drop_up_min"), get("drop_up_max")), (8.0, 10.0, 10.0, 15.0, 3.0, 10.0, 5.0, 10.0));
        assert_eq!((get("peel_count"), get("drop_count")), (1.0, 1.0));
        // Its Re:CoM shot's power: the homing banana (state 3, id 1, kind 1).
        assert_eq!((get("fire_recom_id"), get("fire_kind")), (1.0, 1.0));
        // The banana's start: bone code 28 at t 1 on Re:CoM's hand.
        let skel = crate::mdl::RecomSkeleton::parse(&inp.mdl).unwrap();
        let bone = skel.bones[get("fire_at_bone") as usize].name.clone();
        assert!(bone.contains("hand") || bone.contains("fing"), "{bone}");
        // The clips; Re:CoM's slots kept.
        let c = |r: u16| b.clips.iter().find(|c| c.request == r).unwrap_or_else(|| panic!("request {r}")).clone();
        for r in [0u16, 1, 2, 3, 4, 5, 6, 7, 69, 200, 202, 203, 204, 205, 206, 207, 209, 211, 212, 213, 215, 216, 217, 72, 73, 74, 75, 0x52, 0x5C] {
            let _ = c(r);
        }
        assert!(c(211).frames > 82 && c(200).frames > 80, "the throws leave inside their motions");
        let a0 = crate::ann::Ann::parse(&inp.ann).unwrap();
        let a1 = crate::ann::Ann::parse(&b.ann).unwrap();
        assert_eq!(a0.slots.len(), SPEC.recom_slots);
        for i in 0..SPEC.recom_slots {
            assert_eq!(a0.slots[i], a1.slots[i], "slot {i}");
        }
        // No new attack record: Re:CoM's `.ASD` as it is.
        assert_eq!(b.asd, inp.asd);
        let s0 = crate::ses::parse(&inp.ses).unwrap();
        let s1 = crate::ses::parse(&b.ses).unwrap();
        for (a, b) in s0.iter().zip(&s1) {
            assert_eq!((a.name(), &a.events), (b.name(), &b.events), "Re:CoM's entry kept");
        }
        assert!(b.manifest.contains("\"format\": \"kh1-enemy 1\"") && b.manifest.contains("\"enemy\": \"bouncywild\"") && b.manifest.contains("\"attacks\": [\n\n  ]"), "{}", &b.manifest[..400]);
    }
}
