//! M11: KH1's Search Ghost in Re:CoM's battles (`docs/recom-m11-enemies.md` § The Search Ghost): where its
//! numbers sit in the player's `ex_2230.bd` (`xa_ex_2230.mdls` part 5) and its child `ex_2230_00.bd` (part 3,
//! the cone), and which Re:CoM files it extends (`ENEMY/0014`). The step itself is `crate::enemy`.
//!
//! The offsets are the scripts' own (Checked (files): the player's scripts disassembled; the routine each sits
//! in is named in `kh1_core::searchghost`). `walk_speed` / `run_speed` are the stats block's defaults (the
//! script never writes them; the flier never walks).

use crate::enemy::{AttackSpec, RowSpec, Spec};

/// Where each constant is pushed in `ex_2230.bd` (float or integer); a name with several offsets must read the
/// same at each.
pub const CONSTS: &[(&str, &[usize])] = &[
    ("walk_speed", &[]),
    ("run_speed", &[]),
    // Supervisor `0x253A` (the hold), start `0x10E6`, the search `0x1B8C`.
    ("hold_time", &[0x2678]),
    ("start_search", &[0x1188]),
    ("world11_search", &[0x1BBE]),
    // Blends (every `sys 12` / `sys 13` of 8) and the rates the routines put back.
    ("motion_blend", &[0x894, 0x924, 0xB44, 0xDF8, 0x125A, 0x1346, 0x144E, 0x1782, 0x2F3E, 0x3216, 0x3384]),
    ("attack_blend", &[0x3384]),
    ("rest_blend", &[0x3512]),
    ("turn_default", &[0x684, 0xCAA, 0xE2E, 0x1088, 0x2EB0]),
    ("ease_default", &[0xCBC, 0x2868]),
    // Appearance `0x2130`.
    ("appear_blend", &[0x213E]),
    ("appear_fade", &[0x2150]),
    ("appear_wait", &[0x2176]),
    ("appear_then_blend", &[0x219C]),
    // Hit `0x628`, engaged `0x786`.
    ("drain_hp", &[0x63A, 0x7A8]),
    ("hit_warp_chance", &[0x668, 0x6E0]),
    ("lunge_speed", &[0x694]),
    ("lunge_near", &[0x69A]),
    ("warp_lo", &[0x6F2, 0xC4C]),
    ("warp_hi", &[0x6F8, 0xC52]),
    ("warp_speed", &[0x712, 0x810, 0xC6C]),
    ("hit_hover_speed", &[0x72C]),
    ("hover_near", &[0x732, 0xA1C]),
    ("hit_hover_time", &[0x738]),
    // Roam `0x7EC`.
    ("roam_lo", &[0x7F0]),
    ("roam_hi", &[0x7F6]),
    ("roam_search", &[0x82A]),
    ("roam_close", &[0x872]),
    ("roam_wait_lo", &[0x89C]),
    ("roam_wait_hi", &[0x8A2]),
    ("roam_near", &[0x8D6]),
    ("roam_lost_lo", &[0x92C]),
    ("roam_lost_hi", &[0x932]),
    ("roam_lost_near", &[0x966]),
    ("roam_rounds", &[0x9C0]),
    ("roam_rest_lo", &[0x9DE]),
    ("roam_rest_hi", &[0x9E4]),
    // The hold's hover `0xA0E`, the drain's approach `0xA34`.
    ("hold_hover_speed", &[0xA16]),
    ("hold_hover_time", &[0xA22]),
    ("drain_speed", &[0xAEA]),
    ("drain_near", &[0xAF0]),
    // The attack round `0xB62`.
    ("approach_speed", &[0xB6A]),
    ("approach_near", &[0xB70]),
    ("away_lo", &[0xB98]),
    ("away_hi", &[0xB9E]),
    ("away_speed", &[0xBCE]),
    ("away_arrive", &[0xBD4]),
    ("away_time", &[0xBDA]),
    ("watch_ease", &[0xBF2]),
    ("watch_turn_rate", &[0xBFE]),
    ("watch_time", &[0xC06]),
    ("watch_near", &[0xC3A]),
    // Idle `0xCDE`, its flight `0xDCE`, its search `0x1044`.
    ("idle_turn_rate", &[0xCEC]),
    ("idle_far", &[0xD28]),
    ("idle_near_lo", &[0xD3A]),
    ("idle_near_hi", &[0xD40]),
    ("idle_lo", &[0xD5E]),
    ("idle_hi", &[0xD64]),
    ("wander_speed", &[0xD82]),
    ("wander_arrive", &[0xD88]),
    ("search_lo", &[0xD92]),
    ("search_hi", &[0xD98]),
    ("wander_time", &[0xDDA]),
    ("low_height", &[0xEF0, 0x2E04, 0x31C0]),
    ("home_below", &[0xF12]),
    ("sora_above", &[0xF3A]),
    ("rise_y", &[0xF54, 0x2E1A]),
    ("sora_below", &[0xF74]),
    ("sink_y", &[0xF8C]),
    ("wander_check", &[0xFBA]),
    ("stuck_factor", &[0xFDC, 0x3274, 0x3472]),
    ("wander_new_lo", &[0xFF4]),
    ("wander_new_hi", &[0xFFA]),
    ("search_blend", &[0x1052]),
    // Notice `0x1210`, burst `0x128E`.
    ("notice_then_blend", &[0x1272]),
    ("burst_turn_until", &[0x1368]),
    ("burst_end_blend", &[0x13AC]),
    // The drain `0x13BE` and its orb.
    ("drain_steer_until", &[0x1482]),
    ("drain_back_after", &[0x14B8]),
    ("drain_close", &[0x14CA]),
    ("drain_back_dir", &[0x14EE]),
    ("drain_back_speed", &[0x150C]),
    ("drain_creep_speed", &[0x1550]),
    ("orb_up", &[0x15A8, 0x1628]),
    ("orb_keep", &[0x15E8]),
    ("orb_near", &[0x1652]),
    ("orb_done", &[0x1666]),
    ("orb_last", &[0x167A]),
    ("orb_drop", &[0x1704]),
    ("orb_fade", &[0x172A]),
    // The hover `0x2A50`.
    ("hover_turn_rate", &[0x2A86, 0x2C28]),
    ("hover_rest", &[0x2AEE]),
    ("hover_level", &[0x2BC8, 0x2BEE]),
    ("hover_attack_turn_rate", &[0x2C18]),
    ("hover_bursts", &[0x2C46]),
    ("hover_back_chance", &[0x2CAA, 0x2D66]),
    ("hover_back", &[0x2CBC]),
    ("hover_back_neg", &[0x2CCC]),
    ("hover_far", &[0x2CE2, 0x2DB2]),
    ("hover_turn_lo", &[0x2D12]),
    ("hover_turn_hi", &[0x2D18]),
    ("hover_side_lo", &[0x2D78]),
    ("hover_side_hi", &[0x2D7E]),
    ("hover_side2_lo", &[0x2D92]),
    ("hover_side2_hi", &[0x2D98]),
    ("hover_check", &[0x2E56]),
    ("hover_stuck", &[0x2E78]),
    ("hover_stuck_timer", &[0x2E96]),
    // The warp `0x2EBA`: vanish `0x2FF2`, reappear `0x2EEC`.
    ("vanish_blend", &[0x2EC2]),
    ("warp_arrive", &[0x2ED4]),
    ("warp_time", &[0x2EDA]),
    ("vanish_fade", &[0x3082, 0x308C]),
    ("reappear_wait", &[0x2F02, 0x2F98]),
    ("reappear_fade", &[0x2F88]),
    // The approach `0x30C4`, the flight to a point `0x335A`.
    ("approach_above", &[0x30F4]),
    ("approach_check", &[0x3252]),
    ("approach_retry", &[0x32BC, 0x32D8]),
    ("approach_detour_lo", &[0x3306]),
    ("approach_detour_hi", &[0x330C]),
    ("approach_detour_time", &[0x3332]),
    ("fly_check", &[0x336E, 0x34D4]),
    ("fly_detour_lo", &[0x34B0]),
    ("fly_detour_hi", &[0x34B6]),
    // The hit handler's reward `sys 194(EXP / 10 + 1)`: while it holds a drain (`0x28D8` / `0x28E0`) and on a
    // Gravity hit (`0x29DC` / `0x29E4`, not carried). Its prizes: `crate::enemy::REWARDS` (the table `0x580`).
    ("reward_exp_div", &[0x28D8, 0x29DC]),
    ("reward_exp_add", &[0x28E0, 0x29E4]),
];

/// The request words `ex_2230.bd` pushes (bit 24: an action; bit 30: a request that clears the action flag when
/// it starts): the behaviour plays exactly these.
pub const REQUESTS: &[(u32, usize)] = &[
    (69, 0x2138),          // the appearance
    (0x4000_0000, 0x2196), // its queued idle
    (200, 0x6A0),          // the invisible lunge out of a hit
    (200, 0xAF6),          // the drain's approach
    (200, 0xB3E),          // again after a miss
    (0, 0xB76),            // the attack round's approach plays the idle
    (202, 0x88E),          // the roam's searches
    (202, 0x91E),
    (202, 0x104C),         // the idle's search
    (0, 0xDF2),            // the idle's flight
    (0x0100_00CE, 0x1254), // the notice, an action
    (0x4000_0000, 0x126C), // its queued idle
    (204, 0x1340),         // the burst
    (204, 0x1358),
    (0, 0x13A6),           // the idle after it
    (203, 0x1448),         // the drain
    (203, 0x1460),
    (0, 0x177C),           // the idle after it
    (201, 0x3004),         // the vanish
    (0, 0x301A),           // its queued idle
    (201, 0x2F20),         // the reappearance's 201, cancelled in its frame
    (0, 0x2F38),           // its queued idle (cancelled too)
    (0, 0x2EFC),           // the reappearance's idles (`func_3502`'s request)
    (0, 0x2F92),
    (0, 0x337E),           // the flight to a point plays the idle
];

/// Where the child script `ex_2230_00.bd` (the cone, `kh1_core::searchghost`'s `cone`) pushes its numbers
/// (`crate::enemy::CHILD_CONSTS`).
pub const CHILD_CONSTS: &[(&str, &[usize])] = &[("cone_range", &[0x312]), ("cone_angle", &[0x332])];

/// The child's integer pushes the port relies on: the two motions it looks during (the idle 0 at `0x248`, the
/// search 202 at `0x260`), its bone (`sys 115`'s 45 at `0x27A`, `sys 19`'s 4141 = bone 45 | 0x1000 at `0x28C`:
/// `crate::enemy::SIGHTS`) and its three party members (`0x2AC`).
pub const CHILD_REQUESTS: &[(u32, usize)] = &[(0, 0x248), (202, 0x260), (45, 0x27A), (4141, 0x28C), (3, 0x2AC)];

/// The rows (script offsets) and the Re:CoM attack ids they take their power from (decision M10: Re:CoM's
/// attack power per attack). Re:CoM's Search Ghost has two attacks (Checked (files): `EN0014.ASD`, 2 records):
/// id 0, `e014_050` (a 0.5 m sphere on `l_fing3b`, frames 108..118, its state 2), and id 1, its drain
/// `e014_051` (0.5 m on `l_fing3b`, frames 30..60, its state 3, whose step heals it by 1/8 of its max HP when
/// it lands: `0x140192630` → `0x140181CE0`, Checked (decomp)). KH1's matcher finds `e014_051` an edit (≤ 20
/// bones differ) of KH1's drain 203 (Checked (files)): the drain's two hand rows take id 1. `e014_050` matches no
/// KH1 motion within the edit rule, but its window is exactly KH1's burst 204's second strike (108..118, Checked
/// (files)) and it is Re:CoM's other attack: the burst's damaging rows take id 0.
///
/// The burst's four 0.9 m rows (`0x38C` / `0x3FC` / `0x46C` / `0x4DC`, the hands at both strikes) are attack
/// type 1 (row `+0x38` low bits), which never meets a hurt body (`0x1402B3BE0`: types 1 / 2 / 4 skip hurt bodies)
/// and only Sora's attacks (the table `0x140528CB8`): a repel layer for his swings, not damage (Checked
/// (decomp + exe)). They are not Re:CoM attack records: they are the shared enemy-guard step's row "burst"
/// (`crate::enemy::GUARDS`, owner 2026-11-02).
const DRAIN_R: RowSpec = RowSpec { at: 0xEC, recom_id: 1 };
const DRAIN_L: RowSpec = RowSpec { at: 0x15C, recom_id: 1 };
const BURST_R1: RowSpec = RowSpec { at: 0x1CC, recom_id: 0 };
const BURST_L1: RowSpec = RowSpec { at: 0x23C, recom_id: 0 };
const BURST_R2: RowSpec = RowSpec { at: 0x2AC, recom_id: 0 };
const BURST_L2: RowSpec = RowSpec { at: 0x31C, recom_id: 0 };

/// `kh1_core::searchghost::Attack::ALL`'s order.
pub const ATTACKS: &[AttackSpec] = &[
    AttackSpec { name: "burst", request: 204, rows: &[BURST_R1, BURST_L1, BURST_R2, BURST_L2] },
    AttackSpec { name: "drain", request: 203, rows: &[DRAIN_R, DRAIN_L] },
];

/// The Search Ghost.
pub const SPEC: Spec = Spec {
    name: "searchghost",
    recom_id: 14,
    kh1: "xa_ex_2230",
    script: b"ex_2230.bd",
    prefix: "e014_",
    recom_slots: 75,
    asd: (2, 0x6C, 0x1AC),
    consts: CONSTS,
    requests: REQUESTS,
    // The stat records (`0x26BC`: `sys 45` with `0x14` (LV1) or `0x80` (LV2), `crate::stats::TWINS`).
    records: &[0x14, 0x80],
    attacks: ATTACKS,
    manifest: "kh1-searchghost.json",
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enemy::{build, Inputs};

    /// The player's files: `KH1_M11_DIR` (KH1's `xa_*.mdls` / `.mset` and Re:CoM's `ENEMY__NNNN__ENNNNN.*`)
    /// and `KH1_TEST_DATA` (`btltbl.bin`).
    fn inputs() -> Option<Inputs> {
        crate::enemy::test_inputs(&SPEC, true)
    }

    #[test]
    fn builds_from_the_players_files() {
        let Some(inp) = inputs() else { return };
        let b = build(&SPEC, &inp).unwrap();
        let get = |n: &str| b.consts.iter().find(|c| c.0 == n).map(|c| c.1).unwrap();
        // The scripts' numbers (the disassembly's).
        assert_eq!((get("hold_time"), get("drain_hp"), get("hit_warp_chance"), get("turn_default")), (568.0, 0.5, 0.5, 11.25));
        assert_eq!((get("cone_range"), get("cone_angle"), get("sight_bone")), (300.0, 35.0, 32.0), "the cone: 3 m, 35°, on Re:CoM's l_eye");
        assert_eq!((get("orb_near"), get("orb_done"), get("orb_keep"), get("orb_up")), (320.0, 100.0, 0.95, 80.0));
        assert_eq!((get("drain_speed"), get("drain_near"), get("approach_near"), get("watch_near")), (20.0, 250.0, 450.0, 200.0));
        assert_eq!((get("search_lo"), get("search_hi"), get("roam_rounds"), get("hover_bursts")), (60.0, 600.0, 3.0, 3.0));
        // The hold's reward: `sys 194(EXP / 10 + 1)` and the table `0x580`: 3 prizes of KH1 kind 4 (HP +5).
        assert_eq!((get("reward_exp_div"), get("reward_exp_add")), (10.0, 1.0));
        assert_eq!((0..7).map(|k| get(&format!("reward_prize_{k}"))).collect::<Vec<_>>(), vec![0.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0]);
        // The burst's guard row: the four type-1 rows as one, priority 40, recoil 205, cone cos 1.0, on in 204's
        // two strikes.
        assert!(b.manifest.contains("\"name\": \"burst\", \"row\": \"0x38c\", \"also\": [\"0x3fc\", \"0x46c\", \"0x4dc\"], \"slots\": [0, 1], \"type\": 1, \"priority\": 40, \"recoil\": 205"), "{}", b.manifest);
        assert!(b.manifest.contains("\"cos\": 1.0, \"windows\": [{\"request\": 204, \"on\": [[36.0, 48.0], [108.0, 118.0]]}]"), "{}", b.manifest);
        assert_eq!((get("hover_side_lo"), get("hover_back_neg"), get("drain_back_dir"), get("rise_y")), (-120.0, -180.0, -3.0, -0.2));
        assert_eq!((get("weight"), get("walk_speed"), get("motion_blend"), get("air_accel")), (1.0, 2.0, 8.0, 0.1));
        let c = |r: u16| b.clips.iter().find(|c| c.request == r).unwrap().clone();
        // Re:CoM's bakes: the idle (`e014_001`) and 70 (`e014_074`); every other request a new clip at 100 + entry.
        assert_eq!((c(0).name.as_str(), c(0).source, c(70).name.as_str(), c(70).source), ("e014_001", "recom", "e014_074", "recom"));
        assert_eq!((c(200).slot, c(201).slot, c(202).slot, c(203).slot, c(204).slot, c(205).slot, c(206).slot, c(69).slot), (110, 111, 112, 113, 114, 115, 116, 108));
        assert_eq!((c(203).frames, c(204).frames, c(205).frames, c(206).frames, c(69).frames), (120, 164, 82, 108, 120));
        assert!(!c(204).looped && c(202).looped);
        // Re:CoM's 75 slots unchanged.
        let a0 = crate::ann::Ann::parse(&inp.ann).unwrap();
        let a1 = crate::ann::Ann::parse(&b.ann).unwrap();
        for i in 0..SPEC.recom_slots {
            assert_eq!(a0.slots[i], a1.slots[i], "slot {i}");
        }
        // ASD: Re:CoM's bytes kept, one record per damaging row window (4 burst, 2 drain), the tail moved.
        assert_eq!(&b.asd[..0x10], &inp.asd[..0x10]);
        assert_eq!(&b.asd[0x6C..0x1AC], &inp.asd[0x6C..0x1AC]);
        assert_eq!(u32::from_le_bytes(b.asd[0x10..0x14].try_into().unwrap()), 2 + 6);
        assert_eq!(&b.asd[0x1AC + 6 * 0xA0..], &inp.asd[0x1AC..]);
        let rec = |i: usize| &b.asd[0x1AC + i * 0xA0..0x1AC + (i + 1) * 0xA0];
        let win = |i: usize| (u32::from_le_bytes(rec(i)[0x5C..0x60].try_into().unwrap()) / 50, u32::from_le_bytes(rec(i)[0x60..0x64].try_into().unwrap()) / 50);
        let id = |i: usize| u32::from_le_bytes(rec(i)[0x64..0x68].try_into().unwrap());
        let clip = |i: usize| String::from_utf8_lossy(&rec(i)[0x40..0x48]).into_owned();
        let bone = |i: usize| String::from_utf8_lossy(&rec(i)[0..8]).trim_end_matches('\0').to_string();
        let r = |i: usize| f32::from_le_bytes(rec(i)[0x58..0x5C].try_into().unwrap());
        assert_eq!((clip(0), win(0), id(0), bone(0)), ("e014_114".into(), (36, 48), 0, "r_hand1".into()), "the burst's first strike, right hand");
        assert_eq!((clip(1), win(1), id(1), bone(1)), ("e014_114".into(), (36, 48), 0, "l_hand1".into()));
        assert_eq!((clip(2), win(2), id(2)), ("e014_114".into(), (108, 118), 0), "its second strike");
        assert_eq!((clip(3), win(3), id(3)), ("e014_114".into(), (108, 118), 0));
        assert_eq!((clip(4), win(4), id(4), bone(4)), ("e014_113".into(), (34, 54), 1, "r_hand1".into()), "the drain");
        assert_eq!((clip(5), win(5), id(5), bone(5)), ("e014_113".into(), (34, 54), 1, "l_hand1".into()));
        assert!((0..4).all(|i| (r(i) - 0.6).abs() < 1e-6) && (4..6).all(|i| (r(i) - 0.5).abs() < 1e-6));
        // The clash recoil 205 is in the model: KH1's recoil when Sora guards either attack (and the burst's guard
        // row's).
        assert_eq!(b.manifest.matches("\"recoil\": 205").count(), 3);
        // SES: Re:CoM's entries kept; the drain's clip gets `e014_051`'s sounds (its edit).
        let s0 = crate::ses::parse(&inp.ses).unwrap();
        let s1 = crate::ses::parse(&b.ses).unwrap();
        for e in &s0 {
            assert!(s1.iter().any(|x| x.name() == e.name() && x.events == e.events), "{}", e.name());
        }
        let ev = |n: &str| s1.iter().find(|e| e.name() == n).map(|e| e.events.clone());
        if ev("e014_051").is_some() {
            assert_eq!(ev("e014_113"), ev("e014_051"), "the drain: Re:CoM's drain's sounds");
        }
        assert_eq!(b.parts.len(), 5);
        assert!(b.manifest.contains("\"enemy\": \"searchghost\""));
        std::fs::write(std::env::temp_dir().join("kh1-searchghost-test.json"), &b.manifest).unwrap();
        println!("{}\nparts {:?}", b.records.join("\n"), b.parts);
    }
}
