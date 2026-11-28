//! M11: KH1's Air Soldier in Re:CoM's battles (`docs/recom-m11-enemies.md` § The Air Soldier): where
//! its numbers sit in the player's `ex_2160.bd` (`xa_ex_2160.mdls` part 5) and which Re:CoM files it
//! extends (`ENEMY/0015`). The step itself is `crate::enemy`.
//!
//! The offsets are the script's own (Checked (files): the player's script disassembled; the routine
//! each sits in is named in `kh1_core::airsoldier`). `run_speed` is the stats block's default (the
//! script never writes it; the flier never runs).

use crate::enemy::{AttackSpec, RowSpec, Spec};

/// Where each constant is pushed in `ex_2160.bd` (float or integer); a name with several offsets
/// must read the same at each.
pub const CONSTS: &[(&str, &[usize])] = &[
    ("turn_deg", &[0x38F0]),
    ("walk_speed", &[0x3902]),
    ("run_speed", &[]),
    ("idle_search", &[0x3812]),
    ("wander_min", &[0x1544]),
    ("wander_max", &[0x154A]),
    ("wander_probe", &[0x1626, 0x167C]),
    ("pit_depth", &[0x1638]),
    ("pit_lift", &[0x1650]),
    ("wander_high", &[0x16B8]),
    ("wander_near_lo", &[0x16E4]),
    ("wander_near_hi", &[0x16EA]),
    ("wander_far_lo", &[0x1706]),
    ("wander_far_hi", &[0x170C]),
    ("fly_up", &[0x1788]),
    ("fly_speed", &[0x1728]),
    ("fly_arrive", &[0x1732]),
    ("fly_time_min", &[0x173C]),
    ("fly_time_max", &[0x1742]),
    ("fly_low", &[0x1834, 0x189C, 0x4824, 0x48AC, 0x20A8]),
    ("rest_min", &[0x193C]),
    ("rest_max", &[0x1942]),
    ("idle_blend", &[0x17B2, 0x5008, 0x50A6, 0x426E, 0x447C, 0x46CE, 0x24D4]),
    ("climb_dy", &[0x112A]),
    ("dive_dy", &[0x115A]),
    ("close_range", &[0x11A0]),
    ("mid_range", &[0x11C4]),
    ("mid_kick_chance", &[0x126E]),
    ("mid_swoop_chance", &[0x12C4]),
    ("close_approach_chance", &[0x1382]),
    ("hover_min", &[0x124A, 0x12A8, 0x12E2, 0x130A, 0x13A0, 0x13F6]),
    ("hover_max", &[0x1250, 0x12AE, 0x12E8, 0x1310, 0x13A6, 0x13FC]),
    ("hover_short_min", &[0x134A]),
    ("hover_short_max", &[0x1350]),
    ("hover_near", &[0x505C]),
    ("back_kick_min", &[0x1288]),
    ("back_kick_max", &[0x128E]),
    ("hit_back_min", &[0x8E6]),
    ("hit_back_max", &[0x8EC]),
    ("back_up", &[0x422E]),
    ("back_speed", &[0x4250]),
    ("back_time", &[0x4276]),
    ("back_hover_min", &[0x432C]),
    ("back_hover_max", &[0x4332]),
    ("approach_speed", &[0x13C4]),
    ("approach_arrive", &[0x13CA]),
    ("approach_time", &[0x13D0]),
    ("approach_accel", &[0x48FE]),
    ("approach_up", &[0x4758]),
    ("strafe_speed", &[0x4354]),
    ("strafe_turn", &[0x435E]),
    ("strafe_turn_factor", &[0x448C]),
    ("strafe_up", &[0x43EC]),
    ("strafe_above_min", &[0x4538]),
    ("strafe_above_max", &[0x453E]),
    ("strafe_down", &[0x4578]),
    ("strafe_rise", &[0x45C8]),
    ("strafe_left", &[0x4390]),
    ("strafe_right", &[0x43AA]),
    ("strafe_side_chance", &[0x437E]),
    ("strafe_close_min", &[0x1364]),
    ("strafe_close_max", &[0x136A]),
    ("strafe_mid_min", &[0x1322]),
    ("strafe_mid_max", &[0x1328]),
    ("strafe_hit_min", &[0xD14]),
    ("strafe_hit_max", &[0xD1A]),
    ("kick_near", &[0x2894]),
    ("kick_up", &[0x28D2]),
    ("kick_speed", &[0x290E]),
    ("slash_near", &[0x2310]),
    ("slash_up", &[0x2372]),
    ("slash_speed", &[0x23AE]),
    ("swoop_up", &[0x2500]),
    ("swoop_aim_up", &[0x257A]),
    ("swoop_rise_speed", &[0x24A2]),
    ("swoop_rise_time", &[0x250A]),
    ("swoop_wait_a", &[0x2644]),
    ("swoop_wait_b", &[0x264E]),
    ("swoop_low", &[0x267C]),
    ("swoop_down_speed", &[0x2686]),
    ("swoop_down_time", &[0x2690]),
    ("spin_off_min", &[0x1D92]),
    ("spin_off_max", &[0x1D98]),
    ("spin_off_up", &[0x1DBC]),
    ("spin_tuck_speed", &[0x1DDE]),
    ("spin_time", &[0x1B46]),
    ("spin_up", &[0x1B9E, 0x1BB2]),
    ("spin_speed", &[0x1BDE]),
    ("spin_near", &[0x1C88]),
    ("spin_close_speed", &[0x1D02]),
    ("charge_wait", &[0x1EE6]),
    ("charge_speed", &[0x1F16]),
    ("charge_arrive", &[0x1F20]),
    ("charge_time_min", &[0x1F2A]),
    ("charge_time_max", &[0x1F30]),
    ("charge_miss_add", &[0x21DA]),
    ("retarget_chance", &[0x65C]),
    ("hit_close", &[0x816]),
    ("hit_level", &[0x850]),
    ("hit_swoop_dy", &[0x86A]),
    ("hit_swoop_height", &[0x850]),
    ("anger_full", &[0x94A]),
    ("anger_power_chance", &[0x962]),
    ("anger_hp_div", &[0x98A]),
    ("power_speed", &[0x9DC]),
    ("power_arrive", &[0x9E2]),
    ("power_time", &[0x9E8]),
    ("power_reach", &[0xA2A]),
    ("power_tail", &[0xA5E, 0xB04]),
    ("power_close", &[0xAC0]),
    ("power_level", &[0xAD6]),
    ("anger_launch", &[0x3CD0]),
    ("anger_knock", &[0x3D1A]),
    ("anger_flinch", &[0x3D56]),
    ("climb_over", &[0x4D78]),
    ("dive_over", &[0x49E6]),
    ("vert_time", &[0x4D82, 0x49F0]),
    ("vert_fast_time", &[0x4DD4, 0x4A38]),
    ("vert_fast_speed", &[0x4E26, 0x4AB6]),
    ("vert_slow_time", &[0x4E68, 0x4B00]),
    ("vert_slow_speed", &[0x4EBA, 0x4B7E]),
    ("vert_speed", &[0x4D0E, 0x497C]),
    ("dive_stop_fast", &[0x4A5A]),
    ("dive_stop_slow", &[0x4B22, 0x4C06]),
    ("vert_tail", &[0x4F10, 0x4BDC]),
    ("vert_wait", &[0x4DCA]),
    ("attack_blend", &[0x286A, 0x22E6, 0x1B2A, 0x1ED8, 0x263C]),
    ("motion_tail", &[0x4452, 0x4680, 0x5014, 0x50B2]),
    ("steer_cap", &[0x18F0, 0x42B2, 0x4622, 0x4912, 0x259A, 0x2708, 0x1E22]),
    ("appear_fade", &[0x32A2]),
    ("appear_wait", &[0x32C8]),
];

/// The request words the script pushes (bit 30: a request that clears the action flag when it
/// starts): the behaviour plays exactly these.
pub const REQUESTS: &[(u32, usize)] = &[
    (69, 0x328A),          // the appearance
    (1, 0x17AC),           // the flight
    (200, 0x2864),         // the swoop kick
    (201, 0x22E0),         // the slash (from frame 20)
    (202, 0x2636),         // the swoop
    (203, 0x1C6A),         // the spin
    (210, 0x1ED2),         // the charge's wind-up
    (211, 0x1F98),         // the charge
    (212, 0x21BA),         // the charge's end (hit)
    (213, 0x21F2),         // the charge's end (miss)
    (220, 0x4268),         // the back-off flight
    (221, 0x4398),         // the strafe left
    (222, 0x43C4),         // the strafe right
    (0x4000_00E1, 0x9B0),  // the power-up 225
    (226, 0x9CC),          // its approach
    (227, 0xA3C),          // its end
    (0x4000_00E6, 0x4D38), // the climb 230
    (231, 0x4FBC),         // its loop
    (232, 0x4DBC),         // its end
    (0x4000_00EB, 0x49A6), // the dive's tuck 235
    (235, 0x1B24),         // the spin's tuck
    (236, 0x4CB4),         // the tuck's loop
    (236, 0x1B54),
    (237, 0x4A2A),         // the dive's end
];

/// The rows (script offsets) and the Re:CoM attack ids they take their power from (decision M10:
/// Re:CoM's attack power per attack). Re:CoM's Air Soldier has two (Checked (files): `EN0015.ASD`):
/// id 0 its kick `e015_050` on `r_toes`, id 1 its dash punch `e015_051` on `r_hand` (state 3 moves it,
/// `0x14018A8D0`). KH1's matcher pairs neither with a KH1 motion. By the limb of KH1's row (the row's
/// bone code's bone, Checked (files)): the swoop kick's `0x2AC` (l. hand) and the slash's `0x38C`
/// (r. hand) the punch's power, the swoop's `0x3FC` (r. toes) and the spin's `0x4DC` (l. toes) the
/// kick's, the charge's `0x46C` (l. collar, the body's dash) the dash punch's: Assumption (nearest
/// Re:CoM attack; owner question). Each attack's other row (`0xEC`, `0x15C`, `0x1CC`, `0x23C`: on the
/// chest, reaction kind 0, mode bits 2: a body block, no damage reaction on Sora) has no Re:CoM
/// counterpart and is not made (Assumption; owner question).
const KICK: RowSpec = RowSpec { at: 0x2AC, recom_id: 1 };
const SLASH: RowSpec = RowSpec { at: 0x38C, recom_id: 1 };
const SWOOP: RowSpec = RowSpec { at: 0x3FC, recom_id: 0 };
const SPIN: RowSpec = RowSpec { at: 0x4DC, recom_id: 0 };
const CHARGE: RowSpec = RowSpec { at: 0x46C, recom_id: 1 };

/// `kh1_core::airsoldier::Attack::ALL`'s order.
pub const ATTACKS: &[AttackSpec] = &[
    AttackSpec { name: "swoop kick", request: 200, rows: &[KICK] },
    AttackSpec { name: "slash", request: 201, rows: &[SLASH] },
    AttackSpec { name: "swoop", request: 202, rows: &[SWOOP] },
    AttackSpec { name: "spin", request: 203, rows: &[SPIN] },
    AttackSpec { name: "charge", request: 211, rows: &[CHARGE] },
];

/// The Air Soldier.
pub const SPEC: Spec = Spec {
    name: "airsoldier",
    recom_id: 15,
    kh1: "xa_ex_2160",
    script: b"ex_2160.bd",
    prefix: "e015_",
    recom_slots: 75,
    asd: (2, 0x6C, 0x1AC),
    consts: CONSTS,
    requests: REQUESTS,
    // The LV1 / LV2 stat records (`0x38C0`: `sys 45` with `0x14` / `0x80`, `crate::stats::TWINS`).
    records: &[0x14, 0x80],
    attacks: ATTACKS,
    manifest: "kh1-airsoldier.json",
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
        assert_eq!((get("turn_deg"), get("walk_speed"), get("run_speed"), get("idle_search")), (7.5, 8.0, 8.0, 1500.0));
        assert_eq!((get("close_range"), get("mid_range"), get("hover_near"), get("charge_speed")), (300.0, 600.0, 480.0, 50.0));
        assert_eq!((get("anger_launch"), get("anger_knock"), get("anger_flinch"), get("anger_full"), get("anger_hp_div")), (25.0, 10.0, 5.0, 100.0, 4.0));
        assert_eq!((get("strafe_left"), get("strafe_right"), get("strafe_rise"), get("climb_dy")), (-90.0, 90.0, -0.2, -700.0));
        assert_eq!((get("air_accel"), get("weight")), (0.1, get("weight")));
        // Every request is a KH1 clip (no Re:CoM bake: Re:CoM's Air Soldier clips are its own), the
        // slash starts at frame 20 (78 − 20).
        let c = |r: u16| b.clips.iter().find(|c| c.request == r).unwrap().clone();
        assert!(b.clips.iter().filter(|c| c.request >= 66).all(|c| c.source == "kh1"));
        assert_eq!((c(201).slot, c(201).frames, c(201).root), (114, 58, true));
        assert_eq!((c(200).frames, c(211).looped), (64, true));
        // Re:CoM's 75 slots unchanged.
        let a0 = crate::ann::Ann::parse(&inp.ann).unwrap();
        let a1 = crate::ann::Ann::parse(&b.ann).unwrap();
        for i in 0..SPEC.recom_slots {
            assert_eq!(a0.slots[i], a1.slots[i], "slot {i}");
        }
        // The slash's clip is the motion from frame 20: 59 frames (58 + KH1's held end).
        assert_eq!(a1.slots[114].clip.as_ref().unwrap().frames, 59);
        // ASD: Re:CoM's bytes kept, one record per attack (5), the tail moved.
        assert_eq!(&b.asd[..0x10], &inp.asd[..0x10]);
        assert_eq!(&b.asd[0x6C..0x1AC], &inp.asd[0x6C..0x1AC]);
        assert_eq!(u32::from_le_bytes(b.asd[0x10..0x14].try_into().unwrap()), 7);
        assert_eq!(&b.asd[0x1AC + 5 * 0xA0..], &inp.asd[0x1AC..]);
        let rec = |i: usize| &b.asd[0x1AC + i * 0xA0..0x1AC + (i + 1) * 0xA0];
        let win = |i: usize| (u32::from_le_bytes(rec(i)[0x5C..0x60].try_into().unwrap()), u32::from_le_bytes(rec(i)[0x60..0x64].try_into().unwrap()));
        assert_eq!(win(0), (28 * 50, 50 * 50), "the swoop kick's hit [28, 50)");
        assert_eq!(win(1), (2 * 50, 32 * 50), "the slash's [22, 52) from frame 20");
        assert_eq!(&rec(1)[0x40..0x48], b"e015_114");
        assert_eq!(u32::from_le_bytes(rec(4)[0x64..0x68].try_into().unwrap()), 1, "the charge: the dash punch's power");
        assert!(b.manifest.contains("\"enemy\": \"airsoldier\""));
        std::fs::write(std::env::temp_dir().join("kh1-airsoldier-test.json"), &b.manifest).unwrap();
        println!("{}\nparts {:?}", b.records.join("\n"), b.parts);
    }
}
