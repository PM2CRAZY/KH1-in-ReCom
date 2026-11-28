//! M11: KH1's Soldier in Re:CoM's battles (`docs/recom-m11-enemies.md` § The Soldier): where its
//! numbers sit in the player's `ex_2010.bd` (`xa_ex_2010.mdls` part 5) and which Re:CoM files it
//! extends (`ENEMY/0002`). The step itself is `crate::enemy`.
//!
//! The offsets are the script's own (Checked (files): the player's script disassembled; the
//! routine each sits in is named in `kh1_core::soldier`).

use crate::enemy::{AttackSpec, RowSpec, Spec};

/// Where each constant is pushed in `ex_2010.bd` (`push`: float or integer). A name with several
/// offsets must read the same at each.
pub const CONSTS: &[(&str, &[usize])] = &[
    // Init `0x2298`: the stats block's walk / run / hop.
    ("walk_speed", &[0x22B2]),
    ("run_speed", &[0x22C4]),
    ("hop_height", &[0x22D6]),
    // Supervisor `0x2152`: the idle search (`0x13A0(&wp30, 0, 1000, 2)`).
    ("idle_search", &[0x218A]),
    // Engaged `0x4F6`.
    ("close_range", &[0x532]),
    ("mid_range", &[0x54C]),
    // Close `0x684`.
    ("close_hits", &[0x68C]),
    ("close_retreat_chance", &[0x6A4]),
    // `0xB88`.
    ("kick_range", &[0xB8C]),
    // Mid `0x5D8`.
    ("mid_jump_kick_chance", &[0x5E2]),
    ("mid_spin_chance", &[0x5FC]),
    ("mid_approach_timeout", &[0x612, 0x666]),
    ("mid_approach_dist", &[0x618, 0x66C]),
    ("mid_again_chance", &[0x638]),
    ("mid_again_jump_kick_chance", &[0x650]),
    // Far `0x5B6`.
    ("far_approach_timeout", &[0x5B6]),
    ("far_approach_dist", &[0x5BC]),
    // Approach and pause `0xC5E`.
    ("approach_retarget", &[0xC74]),
    ("pause_range", &[0xC88]),
    ("pause_chance", &[0xCA0]),
    ("pause_min", &[0xCC0]),
    ("pause_max", &[0xCC6]),
    ("idle_blend", &[0xCB8, 0x1664]),
    // Strafe `0xCDE` / `0x1214`.
    ("strafe_short_chance", &[0xCE8]),
    ("strafe_short_min", &[0xCF6]),
    ("strafe_short_max", &[0xCFC]),
    ("strafe_long_min", &[0xD0E]),
    ("strafe_long_max", &[0xD14]),
    ("strafe_near", &[0xD3E]),
    ("strafe_turn_near", &[0xD4C]),
    ("strafe_turn_far", &[0xD5A]),
    ("strafe_speed", &[0xD6E]),
    ("strafe_side_chance", &[0x123E]),
    ("strafe_angle", &[0x1260]),
    ("strafe_turn_factor", &[0x1286]),
    // Back off `0xC42` → `0x11A8`.
    ("back_off_min", &[0xC42]),
    ("back_off_max", &[0xC48]),
    ("back_off_timeout", &[0xC52]),
    // Notice `0x758`.
    ("notice_wait_max", &[0x780]),
    ("notice_blend", &[0x798, 0x7B4]),
    // Spin kick `0x9B6`, double kick `0xBAC`: the waits after.
    ("spin_wait", &[0xA06]),
    ("double_kick_wait", &[0xBFC]),
    // Hit `0x40C`.
    ("hit_range", &[0x410]),
    ("hit_count", &[0x422]),
    ("hit_hp_ratio", &[0x442]),
    ("hit_kick_chance", &[0x46E]),
    ("hit_rush_chance", &[0x49E]),
    // The jump at a target above `0x7E0` / `0xFB2`.
    ("jump_height", &[0x814]),
    ("jump_max_speed", &[0x81A]),
    ("jump_offset", &[0x820]),
    ("jump_divisor", &[0x826]),
    ("jump_steer", &[0x882]),
    ("jump_kick_after", &[0x89C]),
    ("jump_kick_window", &[0x8DA]),
    // Wander `0x6EA` → `0x15BA`.
    ("wander_inner", &[0x6EA]),
    ("wander_outer", &[0x6F0]),
    ("wander_band", &[0x15CE, 0x15F2]),
    ("wander_timeout", &[0x161C, 0x1636, 0x16B6]),
    ("wander_idle_chance", &[0x164C]),
    ("wander_idle_min", &[0x166C]),
    ("wander_idle_max", &[0x1672]),
    ("wander_home_max", &[0x169A]),
    // Appearance `0x16CA`: the fade-in.
    ("appear_fade", &[0x16CE, 0x16DC]),
    // The actions' blend (`0x17EC`'s callers).
    ("attack_blend", &[0xBEC, 0xC30, 0xA3E]),
];

/// The request words the script pushes (bit 24 an action, bit 23 KH1's flag 0x1000, bit 30 a
/// queue): the behaviour plays exactly these.
pub const REQUESTS: &[(u32, usize)] = &[
    (0x0180_0045, 0x16F6), // appearance 69 (action, held)
    (208, 0x792),          // the look (plain)
    (216, 0x7C8),          // the crouch (`0x17EC`)
    (203, 0x9F0),          // the spin kick (`0x17EC`)
    (201, 0xBE6),          // the double kick
    (200, 0xC2A),          // the kick
    (0x0180_00CA, 0xA38),  // the jumping kick (action, held)
    (0x0180_00CC, 0x94E),  // the air kick (action, held)
    (0x0100_00CF, 0xADE),  // the rush (action)
    (0x4000_00D1, 0xAF6),  // the leap queued after it
    (0x0180_00D1, 0xB20),  // the rush leap (action, held)
    (0x4000_0006, 0xA50),  // request 6 queued after a held action
    (0x0100_0003, 0x1C0C), // the jump start (`0x1BFC`)
    (0x4001_0004, 0x1C24), // the fall queued after it
];

/// The rows (script offsets) and the Re:CoM attack ids they take their power from (decision M10:
/// Re:CoM's attack power per attack). Re:CoM's Soldier has two: id 0 its kick `e002_050` (KH1's
/// matcher: an edit of KH1's kick, entry 37 = request 200) and id 1 its spin kick `e002_051` (an
/// edit of KH1's jumping kick, entry 39 = request 202) (Checked (files)). Rows of KH1's other
/// attacks take the id of the Re:CoM attack whose row they share (the double kick and the rush:
/// the kick's rows; the rush leap: the jumping kick's row); the spin kick's two rows and the air
/// kick's go by the row's `+0x3C` (20 like the kick's rows → id 0, 100 like the jumping kick's →
/// id 1): Assumption (nearest Re:CoM attack; owner question).
const KICK_R: RowSpec = RowSpec { at: 0xEC, recom_id: 0 };
const KICK_L: RowSpec = RowSpec { at: 0x15C, recom_id: 0 };
const BODY: RowSpec = RowSpec { at: 0x1CC, recom_id: 1 };
const SPIN_FOOT: RowSpec = RowSpec { at: 0x23C, recom_id: 1 };
const SPIN_BODY: RowSpec = RowSpec { at: 0x2AC, recom_id: 1 };
const AIR: RowSpec = RowSpec { at: 0x31C, recom_id: 0 };

/// `kh1_core::soldier::Attack::ALL`'s order.
pub const ATTACKS: &[AttackSpec] = &[
    AttackSpec { name: "kick", request: 200, rows: &[KICK_R] },
    AttackSpec { name: "double kick", request: 201, rows: &[KICK_R, KICK_L] },
    AttackSpec { name: "jumping kick", request: 202, rows: &[BODY] },
    AttackSpec { name: "spin kick", request: 203, rows: &[SPIN_FOOT, SPIN_BODY] },
    AttackSpec { name: "air kick", request: 204, rows: &[AIR] },
    AttackSpec { name: "rush", request: 207, rows: &[KICK_R, KICK_L] },
    AttackSpec { name: "rush leap", request: 209, rows: &[BODY] },
];

/// The Soldier.
pub const SPEC: Spec = Spec {
    name: "soldier",
    recom_id: 2,
    kh1: "xa_ex_2010",
    script: b"ex_2010.bd",
    prefix: "e002_",
    recom_slots: 75,
    asd: (2, 0x6C, 0x1AC),
    consts: CONSTS,
    requests: REQUESTS,
    // The LV1 / LV2 stat records (`0xD88(0x14, 0x80)`, `crate::stats::TWINS`).
    records: &[0x14, 0x80],
    attacks: ATTACKS,
    manifest: "kh1-soldier.json",
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
        assert_eq!((get("walk_speed"), get("run_speed"), get("hop_height")), (3.0, 7.0, 300.0));
        assert_eq!((get("idle_search"), get("close_range"), get("mid_range"), get("kick_range")), (1000.0, 350.0, 450.0, 280.0));
        assert_eq!((get("close_hits"), get("hit_count"), get("strafe_speed"), get("jump_kick_after")), (3.0, 3.0, 7.0, 37.0));
        assert_eq!((get("flinch_push"), get("weight")), (80.0, 1.0));
        // Entry 32 is Re:CoM's only bake (`e002_070`); the attacks are new clips.
        let c = |r: u16| b.clips.iter().find(|c| c.request == r).unwrap().clone();
        assert_eq!((c(66).slot, c(66).source), (70, "recom"));
        assert_eq!((c(200).slot, c(200).name.as_str(), c(200).frames, c(200).root), (137, "e002_137", 110, true));
        assert_eq!((c(69).frames, c(209).frames), (86, 92));
        // Re:CoM's 75 slots unchanged.
        let a0 = crate::ann::Ann::parse(&inp.ann).unwrap();
        let a1 = crate::ann::Ann::parse(&b.ann).unwrap();
        for i in 0..SPEC.recom_slots {
            assert_eq!(a0.slots[i], a1.slots[i], "slot {i}");
        }
        // ASD: Re:CoM's bytes kept, one record per row (10), the tail moved.
        assert_eq!(&b.asd[..0x10], &inp.asd[..0x10]);
        assert_eq!(&b.asd[0x6C..0x1AC], &inp.asd[0x6C..0x1AC]);
        assert_eq!(u32::from_le_bytes(b.asd[0x10..0x14].try_into().unwrap()), 12);
        assert_eq!(&b.asd[0x1AC + 10 * 0xA0..], &inp.asd[0x1AC..]);
        let rec = &b.asd[0x1AC..0x1AC + 0xA0];
        assert_eq!(&rec[0x40..0x48], b"e002_137");
        assert_eq!(u32::from_le_bytes(rec[0x5C..0x60].try_into().unwrap()), 3000, "the kick's hit [60, 70)");
        // The double kick's second row uses hit slot 1 ([92, 120)).
        let r2 = &b.asd[0x1AC + 2 * 0xA0..0x1AC + 3 * 0xA0];
        assert_eq!((u32::from_le_bytes(r2[0x5C..0x60].try_into().unwrap()), u32::from_le_bytes(r2[0x60..0x64].try_into().unwrap())), (4600, 6000));
        // Hurt parts: KH1's four (head, hips, forearms).
        assert_eq!(b.parts.len(), 4, "{:?}", b.parts);
        assert_eq!(b.parts[1].1, 2, "the hips");
        assert!(b.manifest.contains("\"format\": \"kh1-enemy 1\"") && b.manifest.contains("\"enemy\": \"soldier\""));
        std::fs::write(std::env::temp_dir().join("kh1-soldier-test.json"), &b.manifest).unwrap();
        println!("{}", b.records.join("\n"));
    }

    #[test]
    fn the_shadow_hurt_parts_match_m10s() {
        // The generic rule (group 2's codes → the code bone's parent) gives M10's checked bones.
        let Some(d) = std::env::var_os("KH1_SHADOW_DIR") else { return };
        let d = std::path::PathBuf::from(d);
        let mdls = std::fs::read(d.join("xa_ex_2020.mdls")).unwrap();
        let mdl = std::fs::read(d.join("ENEMY__0001__EN0001.MDL")).unwrap();
        let conv = crate::convert::Converter::new(&mdls, crate::mdl::RecomSkeleton::parse(&mdl).unwrap()).unwrap();
        let p = crate::enemy::hurt_parts(&mdls, &conv).unwrap();
        assert_eq!(p.iter().map(|x| x.0).collect::<Vec<_>>(), crate::shadow::HURT_PART_BONES.to_vec());
    }
}
