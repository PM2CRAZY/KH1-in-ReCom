//! M11: KH1's Powerwild in Re:CoM's battles (`docs/recom-m11-enemies.md` § The Powerwild): where its
//! numbers sit in the player's `ex_2030.bd` (`xa_ex_2030.mdls` part 5) and which Re:CoM files it
//! extends (`ENEMY/0007`). The step itself is `crate::enemy`.
//!
//! The offsets are the script's own (Checked (files): the player's script disassembled; the
//! routine each sits in is named in `kh1_core::powerwild`).

use crate::enemy::{AttackSpec, RowSpec, Spec};

/// Where each constant is pushed in `ex_2030.bd` (`push`: float or integer). A name with several
/// offsets must read the same at each.
pub const CONSTS: &[(&str, &[usize])] = &[
    // Init `0x2346`: the stats block's walk / run / hop.
    ("walk_speed", &[0x2360]),
    ("run_speed", &[0x2372]),
    ("hop_height", &[0x2384]),
    // Supervisor `0x2262`: the idle search (`0x1536(&wp30, 0, 1500, 2)`).
    ("idle_search", &[0x229A]),
    // Engaged `0x5A2`.
    ("close_range", &[0x5D8]),
    ("mid_range", &[0x5F2]),
    // Hit `0x4B0`.
    ("hit_range", &[0x4EC]),
    ("hit_flurry_chance", &[0x504]),
    ("hit_hop_side_chance", &[0x52E]),
    // The hit handler `0x2436`: its own launch (`sys 38(hit, 270 / 280, 80, 25, 0)`) above 40 %.
    ("own_launch_hp", &[0x24F0]),
    ("own_launch_hop", &[0x253E]),
    ("own_launch_speed", &[0x2544]),
    ("own_launch_down", &[0x254A]),
    // Close `0x6FA`.
    ("close_side_chance", &[0x736]),
    // Mid `0x6A0`.
    ("mid_pause_chance", &[0x6AE]),
    ("mid_hop_strafe_chance", &[0x6D6]),
    // Far `0x658` / `0xCCE`.
    ("far_idle_chance", &[0xCFA]),
    ("far_idle_min", &[0xD1A]),
    ("far_idle_max", &[0xD20]),
    ("far_swipe_chance", &[0x666]),
    ("far_roar_chance", &[0x680]),
    ("approach_timeout", &[0xCD8]),
    ("approach_retarget", &[0xCDE]),
    ("approach_dist", &[0xCE4]),
    // Strafe `0xC80` → `0x13AA(1, 7, rand(60, 120), 2, 300)`.
    ("strafe_speed", &[0xC96]),
    ("strafe_min", &[0xC9C]),
    ("strafe_max", &[0xCA2]),
    ("strafe_turn", &[0xCAC]),
    ("strafe_stop", &[0xCB2]),
    ("strafe_side_chance", &[0x13D4]),
    ("strafe_angle", &[0x13F6]),
    ("strafe_turn_factor", &[0x141C]),
    // Pause `0x9E6`.
    ("pause_range", &[0x9EA]),
    ("pause_min", &[0x9F8]),
    ("pause_max", &[0x9FE]),
    ("pause_stop", &[0xA44]),
    // Roar `0x8D8`.
    ("roar_stop", &[0x914]),
    // Hop-strafe `0xA8E`.
    ("hop_strafe_turn", &[0xA8E]),
    ("hop_strafe_side_chance", &[0xAAE]),
    ("hop_strafe_angle", &[0xADA]),
    ("hop_strafe_min", &[0xB02]),
    ("hop_strafe_max", &[0xB08]),
    ("hop_strafe_speed", &[0xB6C]),
    ("hop_strafe_stop", &[0xB86]),
    // Hops `0xBEC` → `0x1152(angle, 20, 50, 30, r)`, `sys 82`'s drag.
    ("hop_back_angle", &[0xC02]),
    ("hop_left_angle", &[0xC22]),
    ("hop_right_angle", &[0xC42]),
    ("hop_speed", &[0xC64]),
    ("hop_jump_height", &[0xC6A]),
    ("hop_time", &[0xC70]),
    ("hop_drag", &[0x11E4]),
    // Pounce `0xD34`.
    ("pounce_offset", &[0xD6E]),
    ("pounce_divisor", &[0xD76]),
    ("pounce_max_speed", &[0xD88, 0xD9C]),
    ("pounce_height", &[0xDAC]),
    ("pounce_steer", &[0xE0C]),
    ("pounce_attack_after", &[0xE26]),
    // Wander `0x78C` → `0x1750(350, 450, &home)`.
    ("wander_inner", &[0x78C]),
    ("wander_outer", &[0x792]),
    ("wander_band", &[0x1764, 0x1788]),
    ("wander_timeout", &[0x17B2, 0x17CC, 0x184C]),
    ("wander_idle_chance", &[0x17E2]),
    ("wander_idle_min", &[0x1802]),
    ("wander_idle_max", &[0x1808]),
    ("wander_home_max", &[0x1830]),
    // Blends: the idle (`sys 12(0, 8)`), the plain motions (227 / 228 / 224 and their idles, the
    // strafe motions), the swipe's 2, the actions' 8.
    ("idle_blend", &[0x17FA, 0xA28, 0xD12]),
    ("motion_blend", &[0x8E6, 0x95A, 0x976, 0x9BA, 0x9D6, 0xB30]),
    ("swipe_blend", &[0xEB4]),
    ("attack_blend", &[0xEBA, 0xF16, 0xF1C, 0x990, 0x996]),
    // Appearance `0x802`: the fade-in, action 69's blend.
    ("appear_fade", &[0x812]),
    ("appear_blend", &[0x836]),
];

/// The request words the script pushes (bit 24 an action, bit 30 a queue): the behaviour plays
/// exactly these.
pub const REQUESTS: &[(u32, usize)] = &[
    (0x0100_0045, 0x830), // appearance 69 (action)
    (0x4000_0000, 0x848), // the idle queued after it
    (215, 0x516),         // the counter's back hop
    (207, 0x53C),         // the counter's side hops
    (202, 0x54A),
    (215, 0x71E), // close: the back hop
    (207, 0x744), // close: the side hops
    (202, 0x752),
    (228, 0x8E0),         // the roar
    (224, 0x954),         // the shake-off
    (223, 0x98A),         // the stretch (`0x18E0`: an action)
    (227, 0x9B4),         // the first look
    (201, 0xAC8),         // the hop-strafe's motions
    (200, 0xAF4),
    (215, 0xBF4),         // `0xBEC`'s hop angles by request
    (202, 0xC14),
    (207, 0xC34),
    (0x0100_00DE, 0xE5E), // the pounce's strike (action)
    (212, 0xEAE),         // the swipe (`0x18E0`)
    (214, 0xF10),         // the flurry (`0x18E0`)
    (270, 0x2516),        // its own launch: ground / air bases
    (280, 0x2524),
    (0x0100_0003, 0x1D00), // the jump start (`0x1CF0`)
    (0x4001_0004, 0x1D18), // the fall queued after it
];

/// The rows (script offsets) and the Re:CoM attack ids they take their power from (decision M10:
/// Re:CoM's attack power per attack). Re:CoM's Powerwild has two: id 0 its swipe `e007_051` (31
/// Re:CoM frames, one hit: KH1's 212, 62 frames, one hit) and id 1 its flurry `e007_050` (49
/// frames, two hits: KH1's 214, 98 frames) (Checked (files): the lengths and hit windows; its
/// state 2 plays `e007_051`, state 3 `e007_050`, Checked (exe)). The pounce 222 has no Re:CoM
/// counterpart: its rows' `+0x3C` (10, the flurry's; the swipe's is 50) gives id 1 (Assumption:
/// the nearest Re:CoM attack, the Soldier's rule).
const SWIPE: RowSpec = RowSpec { at: 0x23C, recom_id: 0 };
const FLURRY_R: RowSpec = RowSpec { at: 0xEC, recom_id: 1 };
const FLURRY_L: RowSpec = RowSpec { at: 0x15C, recom_id: 1 };
const FLURRY_BODY: RowSpec = RowSpec { at: 0x1CC, recom_id: 1 };
const POUNCE_R: RowSpec = RowSpec { at: 0x2AC, recom_id: 1 };
const POUNCE_L: RowSpec = RowSpec { at: 0x31C, recom_id: 1 };

/// `kh1_core::powerwild::Attack::ALL`'s order.
pub const ATTACKS: &[AttackSpec] = &[
    AttackSpec { name: "swipe", request: 212, rows: &[SWIPE] },
    AttackSpec { name: "flurry", request: 214, rows: &[FLURRY_R, FLURRY_L, FLURRY_BODY] },
    AttackSpec { name: "pounce", request: 222, rows: &[POUNCE_R, POUNCE_L] },
];

/// The Powerwild.
pub const SPEC: Spec = Spec {
    name: "powerwild",
    recom_id: 7,
    kh1: "xa_ex_2030",
    script: b"ex_2030.bd",
    prefix: "e007_",
    recom_slots: 75,
    asd: (3, 0x6C, 0x24C),
    consts: CONSTS,
    requests: REQUESTS,
    // The LV1 / LV2 stat records (`0xF28(0x14, 0x80)`, `crate::stats::TWINS`).
    records: &[0x14, 0x80],
    attacks: ATTACKS,
    manifest: "kh1-powerwild.json",
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
        assert_eq!((get("walk_speed"), get("run_speed"), get("hop_height"), get("idle_search")), (3.0, 7.0, 500.0, 1500.0));
        assert_eq!((get("close_range"), get("mid_range"), get("hit_range"), get("own_launch_hp")), (300.0, 700.0, 700.0, 0.4));
        assert_eq!((get("hop_back_angle"), get("hop_left_angle"), get("hop_right_angle")), (180.0, 70.0, -70.0));
        assert_eq!((get("hop_speed"), get("hop_jump_height"), get("hop_time"), get("hop_drag")), (20.0, 50.0, 30.0, 0.9));
        assert_eq!((get("pounce_height"), get("pounce_steer"), get("pounce_attack_after"), get("swipe_blend")), (200.0, 7.0, 19.0, 2.0));
        assert_eq!((get("own_launch_hop"), get("own_launch_speed"), get("own_launch_down")), (80.0, 25.0, 0.0));
        // The clips: KH1's (Re:CoM's bakes where its matcher finds one), the hops' trios and its
        // own launch's layout.
        let c = |r: u16| b.clips.iter().find(|c| c.request == r).unwrap().clone();
        assert_eq!((c(212).frames, c(214).frames, c(222).frames), (62, 98, 24));
        for r in [202, 203, 204, 205, 207, 208, 209, 210, 215, 216, 217, 218, 270, 271, 272, 273, 274, 275, 276, 277, 278, 279, 280, 285] {
            assert!(b.clips.iter().any(|x| x.request == r), "request {r}");
        }
        // Re:CoM's 75 slots unchanged.
        let a0 = crate::ann::Ann::parse(&inp.ann).unwrap();
        let a1 = crate::ann::Ann::parse(&b.ann).unwrap();
        for i in 0..SPEC.recom_slots {
            assert_eq!(a0.slots[i], a1.slots[i], "slot {i}");
        }
        // ASD: Re:CoM's bytes kept, one record per row's hit window (9: the swipe 1, the flurry's
        // right hand 3, left hand 2, body 1, the pounce 2), the tail moved.
        assert_eq!(&b.asd[0x6C..0x24C], &inp.asd[0x6C..0x24C]);
        assert_eq!(u32::from_le_bytes(b.asd[0x10..0x14].try_into().unwrap()), 3 + 9);
        assert_eq!(&b.asd[0x24C + 9 * 0xA0..], &inp.asd[0x24C..]);
        assert!(b.manifest.contains("\"format\": \"kh1-enemy 1\"") && b.manifest.contains("\"enemy\": \"powerwild\""));
        std::fs::write(std::env::temp_dir().join("kh1-powerwild-test.json"), &b.manifest).unwrap();
        println!("{}", b.records.join("\n"));
        println!("parts {:?}", b.parts);
    }
}
