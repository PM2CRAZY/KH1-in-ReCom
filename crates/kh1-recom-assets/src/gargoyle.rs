//! M11: KH1's Gargoyle in Re:CoM's battles (`docs/recom-m11-enemies.md` § The Gargoyle): where its numbers sit in the
//! player's `ex_2220.bd` (`xa_ex_2220.mdls` part 5) and its child (part 3: the ball `ex_2220_00.bd`), and which Re:CoM
//! files it extends (`ENEMY/0013`). The step itself is `crate::enemy`.
//!
//! The offsets are the scripts' own (Checked (files): the player's scripts disassembled with `tools/kh1-ai-script.py`,
//! the parent's code from `0x538`, the child's from `0x18`; the routine each sits in is named in `kh1_core::gargoyle`).
//! `walk_speed` / `run_speed` are the stats block's defaults (the script never writes them; the flier never walks); the
//! script never sets a turn rate (`sys 125`): KH1's default π/16 a frame.

use crate::enemy::{AttackSpec, RowSpec, Spec};

/// Where each constant is pushed in `ex_2220.bd` (float or integer); a name with several offsets must read the same
/// at each.
pub const CONSTS: &[(&str, &[usize])] = &[
    ("walk_speed", &[]),
    ("run_speed", &[]),
    // Engaged `0x5E0`: the first round (`0x608`) and each round's split (`0x6C8`), close (< 250) `0x892`, far `0x7E6`.
    ("close_range", &[0x634, 0x6E4, 0x932, 0x978]),
    ("first_dive_chance", &[0x646]),
    ("first_approach_chance", &[0x66C]),
    ("first_swoop_chance", &[0x68C]),
    ("far_ball_chance", &[0x7F0]),
    ("far_swoop_chance", &[0x86C]),
    ("close_dive_chance", &[0x8A4]),
    ("close_again_chance", &[0x91E]),
    ("close_strafe2_chance", &[0x964]),
    // The hit state `0x584` → `0x9A2`.
    ("hit_close_range", &[0x9BA]),
    ("hit_swipe_chance", &[0x9D2]),
    ("hit_dive_chance", &[0x9EC]),
    // The appearance `0xA30` (a spawn type other than 0 and 5: `0xC5C`): 205 rising 1 a frame for its length, the
    // 30-frame fade-in.
    ("appear_fade", &[0xC92]),
    ("appear_rise", &[0xD6C]),
    ("appear_rise_speed", &[0xD7C]),
    // Every `sys 43(...) − 2` (the motion's length less 2 frames) and the swipe's second part's − 8.
    ("len_cut", &[0xEA6, 0xFBE, 0x11CA, 0x12A4, 0x1480, 0x1AFC, 0x1BAA, 0x1C98, 0x1E5E]),
    ("swipe2_cut", &[0x1D80]),
    // The blends (8) and the waits after a motion starts.
    ("motion_blend", &[0xCC0, 0xCD8, 0xEE6, 0x1096, 0x1204, 0x12DE, 0x1494, 0x189E, 0x1B30, 0x1C1E, 0x1D18, 0x1DE0, 0x1E74]),
    ("attack_blend", &[0x1C1E, 0x1D18]),
    ("settle_wait", &[0x149C, 0x18A6]),
    // The swoop `0xDAA`: near (< 300) 50 % the hover else the back-off; far mode beyond 700; the vanish (204 / 206)
    // with its fade 20 frames before its end; the approach; the reappearance (205 / 207: mode 1 rises 2 a frame facing
    // away, turning to the target after 40 frames); hittable again halfway; 90 % the dive.
    ("swoop_near", &[0xDC2]),
    ("swoop_hover_chance", &[0xDDA]),
    ("swoop_far", &[0xE14]),
    ("swoop_turn_after", &[0xE64]),
    ("swoop_extra", &[0xEAE]),
    ("swoop_vanish", &[0xF28]),
    ("swoop_fade", &[0xF5E, 0x1084]),
    ("away_deg", &[0x1016, 0x1660]),
    ("swoop_rise", &[0x103E, 0x110C]),
    ("swoop_rise_speed", &[0x104E, 0x111C]),
    ("swoop_reveal", &[0x1154]),
    ("swoop_dive_chance", &[0x119E]),
    // The back-off `0x11BE`: 208 (unhittable, fading out from 30 frames before its middle), then 209 (back).
    ("back_fade", &[0x124A, 0x127E, 0x1324, 0x134E]),
    // The strafe `0x13E2`: 202 (turned −90°) or 217 (+90°), circling at 4° a frame at 16 a frame, height kept 50..80.
    ("strafe_turn", &[0x1404]),
    ("strafe_side_chance", &[0x1422]),
    ("strafe_left", &[0x1434]),
    ("strafe_right", &[0x144E]),
    ("strafe_low", &[0x1532]),
    ("strafe_rise", &[0x1548]),
    ("strafe_high", &[0x156E]),
    ("strafe_sink", &[0x1584]),
    ("strafe_speed", &[0x15AC]),
    // The hover `0x15BC`: within 400, 201 for up to 300 frames flying away from the target at 4, height 180..210, until
    // 600 away or stuck.
    ("hover_time", &[0x15BC]),
    ("hover_range", &[0x15DE]),
    ("hover_blend", &[0x15F6]),
    ("hover_wait", &[0x15FE]),
    ("hover_low", &[0x16A0]),
    ("hover_rise", &[0x16B6]),
    ("hover_high", &[0x16DC]),
    ("hover_sink", &[0x16F2]),
    ("hover_speed", &[0x171A]),
    ("hover_far", &[0x173C]),
    ("hover_stuck", &[0x1762]),
    // The approach `0x1786(mode)`: 200 for up to 300 frames along its facing, height kept by mode; mode 1 ends in the
    // swipe within 300; modes 2 / 3 (after the swoop's vanish) end within 400 / 50 at the height window.
    ("approach_time", &[0x178A]),
    ("a1_speed", &[0x17B0]),
    ("a1_low", &[0x17BA]),
    ("a1_high", &[0x17C4]),
    ("a1_sink", &[0x17CE]),
    ("a1_rise", &[0x17D8]),
    ("a1_stop", &[0x17E2]),
    ("a2_speed", &[0x1802]),
    ("a2_low", &[0x180C]),
    ("a2_high", &[0x1816]),
    ("a2_sink", &[0x1820]),
    ("a2_rise", &[0x182A]),
    ("a2_stop", &[0x1834]),
    ("a3_speed", &[0x1854]),
    ("a3_low", &[0x185E]),
    ("a3_high", &[0x1868]),
    ("a3_sink", &[0x1872]),
    ("a3_rise", &[0x187C]),
    ("a3_stop", &[0x1886]),
    ("approach_stuck", &[0x1ACE]),
    // The height kept during its attacks `0x1FAE(mode)`: the target 50..80 (attacks) or 180..210 (the ball) below it;
    // up or down at 0.5, speed 1.
    ("hold_low0", &[0x1FC4]),
    ("hold_high0", &[0x1FCE]),
    ("hold_low1", &[0x1FEE]),
    ("hold_high1", &[0x1FF8]),
    ("hold_rise", &[0x2034]),
    ("hold_sink", &[0x208C]),
    ("hold_speed", &[0x20BE]),
];

/// The ball's numbers (`crate::enemy::CHILD_CONSTS`): `ex_2220_00.bd` (entry `0xB22`: `0x58A(…, turn 1, speed 10)`,
/// `0x18(…, model 7, life 600, fade 20)`). `0x18`: 70 frames with nothing; the model at bone 39's parent 15 up,
/// growing 0.005 a frame for 160 frames; 14 frames easing 40 % a frame toward bone code 28 (t 1); then the attack
/// (row `0x3FC`) and `0x2BA`: along the parent's facing tilted up (y −0.8, normalised), at 10 a frame, falling 0.5 a
/// frame faster each frame; each floor contact (`+4 & 0x100`) re-aims it at the target's lock point (speed 10 + 10 ×
/// distance / 1200, tilt −0.4 − 1.6 × height gap / 500, both clamped), the third ends it; a hit ends it; a clash
/// (`& 0x30`) sends it back (`0x748`: row `0x46C`, owned by the swinger, at its target) for one more flight.
/// Checked (files).
pub const BALL_CONSTS: &[(&str, &[usize])] = &[
    ("ball_wait", &[0x28]),
    ("ball_charge", &[0xD8]),
    ("ball_grow", &[0xBA, 0xC0, 0xC6]),
    ("ball_lift", &[0x116]),
    ("ball_ease", &[0x184]),
    ("ball_ease_t", &[0x1AC]),
    ("ball_ease_k", &[0x1C6]),
    ("ball_toss", &[0x2CA]),
    ("ball_gravity", &[0x334]),
    ("ball_dist_max", &[0x444, 0x482]),
    ("ball_rise_max", &[0x45C, 0x4AA]),
    ("ball_speed_base", &[0x46A]),
    ("ball_speed_add", &[0x470]),
    ("ball_tilt_base", &[0x492]),
    ("ball_tilt_add", &[0x498]),
    ("ball_bounces", &[0x502]),
    ("ball_turn", &[0xB54]),
    ("ball_speed", &[0xB5A]),
    ("ball_life", &[0xB6E]),
    ("ball_fade", &[0xB74]),
];

/// The ball's integer pushes the port relies on: the request it lives with (219, `0x56` / `0x156` / `0x1F4`), the hold
/// point's bone (`0x1027`: bit 12, bone 39 itself, `0xFA`), the launch point's bone code 28 (`0x1A6`), its model 7
/// (`0xB68`), its end effect 9 (`0x51C` / `0x554`), the attack's flag 128 (`0x240`).
pub const BALL_REQUESTS: &[(u32, usize)] = &[(219, 0x56), (219, 0x156), (219, 0x1F4), (0x1027, 0xFA), (28, 0x1A6), (7, 0xB68), (9, 0x51C), (9, 0x554), (128, 0x240)];

/// The request words the script pushes (bit 24 an action, bit 30 a queue): the behaviour plays exactly these. The
/// perched appearance (spawn type 5: 210, then 211 / 209, 213) and the leash's 70 are checked but not played.
pub const REQUESTS: &[(u32, usize)] = &[
    (210, 0xA9E),
    (0x0100_00D3, 0xB36),
    (0x4000_00D1, 0xB4E),
    (0x0100_00D5, 0xB68),
    (0x4000_0000, 0xB80),
    (205, 0xC74), // the appearance: 205, the idle queued
    (205, 0xCBA),
    (0, 0xCD2),
    (204, 0xE50), // the swoop: 204 / 205 (near) or 206 / 207 (far)
    (205, 0xE5A),
    (206, 0xE84),
    (207, 0xE8E),
    (208, 0x11C2), // the back-off
    (208, 0x11FE),
    (209, 0x129C),
    (209, 0x12D8),
    (202, 0x143C), // the strafe
    (217, 0x1468),
    (201, 0x15F0), // the hover
    (201, 0x161E),
    (200, 0x1898), // the approach
    (200, 0x18C6),
    (219, 0x1AF4), // the ball
    (219, 0x1B2A),
    (0, 0x1B42),
    (203, 0x1BA2), // the dive
    (203, 0x1C18),
    (0, 0x1C30),
    (212, 0x1C90), // the swipe, 218 queued
    (212, 0x1D12),
    (218, 0x1D2A),
    (218, 0x1D78),
    (218, 0x1DDA),
    (0, 0x1DF2),
    (213, 0x1E56), // the roar
    (213, 0x1E6E),
    (0, 0x1E86),
    (0x4000_0046, 0x2436), // the leash's vanish (not ported)
];

/// The attacks' rows and the Re:CoM attack id they take their power from (decision M10: Re:CoM's attack power per
/// attack; owner 2026-11-02: "the Re:CoM attack of the same kind"). Re:CoM's Gargoyle has two attacks (Checked (files
/// + decomp)): id 0, its wing strike `e013_050` (state 2, `0x14018E310`; `EN0013.ASD`: both wings, `l_wing2c` /
/// `r_wing2c` [25, 40)), and id 1, its shot (state 3, `0x14018E350`: a `CBullet` from `chin` with
/// `0x14017F0E0(enemy, 1, 1, 0)`'s AP; no `.ASD` record). Every body attack (wings, claws) takes id 0; the ball id 1
/// (`crate::enemy::SHOTS`).
const DIVE_L: RowSpec = RowSpec { at: 0xEC, recom_id: 0 };
const DIVE_R: RowSpec = RowSpec { at: 0x15C, recom_id: 0 };
const SWIPE_A: RowSpec = RowSpec { at: 0x1CC, recom_id: 0 };
const SWIPE_B: RowSpec = RowSpec { at: 0x23C, recom_id: 0 };
/// The swipe's body row (type 2, priority 30, front half): it only meets Sora's swings (§ Rows that only clash).
const SWIPE_BODY: RowSpec = RowSpec { at: 0x2AC, recom_id: 0 };
const SWIPE2_A: RowSpec = RowSpec { at: 0x31C, recom_id: 0 };
const SWIPE2_BODY: RowSpec = RowSpec { at: 0x38C, recom_id: 0 };

/// `kh1_core::gargoyle::Attack::ALL`'s order. The ball is its child's (a Re:CoM bullet, the host's `ball_*` numbers).
pub const ATTACKS: &[AttackSpec] = &[
    AttackSpec { name: "dive", request: 203, rows: &[DIVE_L, DIVE_R] },
    AttackSpec { name: "swipe", request: 212, rows: &[SWIPE_A, SWIPE_B, SWIPE_BODY] },
    AttackSpec { name: "swipe end", request: 218, rows: &[SWIPE2_A, SWIPE2_BODY] },
];

/// The Gargoyle.
pub const SPEC: Spec = Spec {
    name: "gargoyle",
    recom_id: 13,
    kh1: "xa_ex_2220",
    script: b"ex_2220.bd",
    prefix: "e013_",
    recom_slots: 75,
    asd: (2, 0x6C, 0x1AC),
    consts: CONSTS,
    requests: REQUESTS,
    // The LV1 / LV2 stat records (`sys 45` with `0x14` / `0x80`).
    records: &[0x14, 0x80],
    attacks: ATTACKS,
    manifest: "kh1-gargoyle.json",
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
        println!("clips: {:?}", b.clips.iter().map(|c| (c.request, c.frames, c.root, c.looped, c.source)).collect::<Vec<_>>());
        println!("{}", b.records.join("\n"));
        // The script's numbers (the disassembly's).
        assert_eq!((get("close_range"), get("first_dive_chance"), get("first_approach_chance"), get("first_swoop_chance")), (250.0, 0.5, 0.3, 0.6));
        assert_eq!((get("far_ball_chance"), get("far_swoop_chance"), get("close_dive_chance"), get("close_again_chance"), get("close_strafe2_chance")), (0.7, 0.3, 0.8, 0.3, 0.5));
        assert_eq!((get("hit_close_range"), get("hit_swipe_chance"), get("hit_dive_chance")), (400.0, 0.6, 0.8));
        assert_eq!((get("swoop_near"), get("swoop_far"), get("swoop_turn_after"), get("swoop_vanish"), get("swoop_dive_chance")), (300.0, 700.0, 40.0, 20.0, 0.1));
        assert_eq!((get("strafe_turn"), get("strafe_speed"), get("hover_time"), get("hover_far"), get("approach_time")), (4.0, 16.0, 300.0, 600.0, 300.0));
        assert_eq!((get("a1_speed"), get("a1_stop"), get("a2_speed"), get("a2_stop"), get("a3_stop")), (4.0, 300.0, 10.0, 400.0, 50.0));
        assert_eq!((get("hold_low0"), get("hold_high0"), get("hold_low1"), get("hold_high1"), get("hold_speed")), (50.0, 80.0, 180.0, 210.0, 1.0));
        assert_eq!((get("len_cut"), get("swipe2_cut"), get("motion_blend"), get("walk_speed"), get("turn_rate")), (2.0, 8.0, 8.0, 2.0, std::f32::consts::PI / 16.0));
        // The ball's.
        assert_eq!((get("ball_wait"), get("ball_charge"), get("ball_lift"), get("ball_ease"), get("ball_ease_t"), get("ball_ease_k")), (70.0, 160.0, -15.0, 14.0, 1.0, 0.4));
        assert_eq!((get("ball_toss"), get("ball_gravity"), get("ball_speed"), get("ball_life"), get("ball_fade"), get("ball_bounces")), (-0.8, 0.5, 10.0, 600.0, 20.0, 2.0));
        assert_eq!((get("ball_dist_max"), get("ball_rise_max"), get("ball_speed_base"), get("ball_speed_add"), get("ball_tilt_base"), get("ball_tilt_add")), (1200.0, 500.0, 10.0, 10.0, -0.4, -1.6));
        assert_eq!((get("ball_radius"), get("ball_recom_id"), get("ball_kind"), get("ball_bat_radius")), (30.0, 1.0, 1.0, 60.0));
        assert!(get("ball_hold_bone") >= 0.0 && get("ball_at_bone") >= 0.0);
        // Every request the behaviour plays has a clip; Re:CoM's 75 slots unchanged.
        for r in [0u16, 0x48, 0x4C, 0x50, 200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 212, 213, 217, 218, 219] {
            assert!(b.clips.iter().any(|x| x.request == r), "request {r}");
        }
        let a0 = crate::ann::Ann::parse(&inp.ann).unwrap();
        let a1 = crate::ann::Ann::parse(&b.ann).unwrap();
        for i in 0..SPEC.recom_slots {
            assert_eq!(a0.slots[i], a1.slots[i], "slot {i}");
        }
        // ASD: Re:CoM's 2 records kept, then the new ones; the tail moved.
        assert_eq!(&b.asd[0x6C..0x1AC], &inp.asd[0x6C..0x1AC]);
        let n = u32::from_le_bytes(b.asd[0x10..0x14].try_into().unwrap()) as usize;
        assert!(n > 2);
        assert_eq!(&b.asd[0x6C + n * 0xA0..], &inp.asd[0x1AC..]);
        // SES: Re:CoM's entries kept.
        let s0 = crate::ses::parse(&inp.ses).unwrap();
        let s1 = crate::ses::parse(&b.ses).unwrap();
        for (a, b) in s0.iter().zip(&s1) {
            assert_eq!((a.name(), &a.events), (b.name(), &b.events), "Re:CoM's entry kept");
        }
        // The rows: the dive's (priority 100, no recoil), the swipe's (30, recoil 215) with its body row (type 2), the
        // swipe end's (30, 216) with its body row.
        let cl = |a: usize| b.clash[a].iter().map(|c| (c.clash_type, c.priority)).collect::<Vec<_>>();
        assert_eq!((cl(0), cl(1), cl(2)), (vec![(0, 100), (0, 100)], vec![(0, 30), (0, 30), (2, 30)], vec![(0, 30), (2, 30)]));
        assert!(b.manifest.contains(r#""name": "dive", "request": 203, "recoil": 110"#), "{}", b.manifest);
        assert!(b.manifest.contains(r#""name": "swipe", "request": 212, "recoil": 215"#));
        assert!(b.manifest.contains(r#""name": "swipe end", "request": 218, "recoil": 216"#));
        assert!(b.manifest.contains("\"format\": \"kh1-enemy 1\"") && b.manifest.contains("\"enemy\": \"gargoyle\""));
        std::fs::write(std::env::temp_dir().join("kh1-gargoyle-test.json"), &b.manifest).unwrap();
        let skel = crate::mdl::RecomSkeleton::parse(&inp.mdl).unwrap();
        println!("bones: hold {} at {:?}, launch {} at {:?}; parts {:?}", skel.bones[get("ball_hold_bone") as usize].name, (get("ball_hold_x"), get("ball_hold_y"), get("ball_hold_z")), skel.bones[get("ball_at_bone") as usize].name, (get("ball_at_x"), get("ball_at_y"), get("ball_at_z")), b.parts.iter().map(|p| skel.bones[p.1].name.clone()).collect::<Vec<_>>());
    }
}
