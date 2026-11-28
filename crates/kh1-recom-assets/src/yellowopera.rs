//! M11: KH1's Yellow Opera in Re:CoM's battles (`docs/recom-m11-enemies.md` § The Yellow Opera): where its numbers sit
//! in the player's `ex_2130.bd` (`xa_ex_2130.mdls` part 5) and its child (part 3: the Thunder `ex_2130_00.bd`), and
//! which Re:CoM files it extends (`ENEMY/0005`). The step itself is `crate::enemy`.
//!
//! The offsets are the scripts' own (Checked (files): the player's scripts disassembled with `tools/kh1-ai-script.py`,
//! the parent's code from `0x308`, the child's from `0x18`; the routine each sits in is named in
//! `kh1_core::yellowopera`). `walk_speed` / `run_speed` are the stats block's defaults (the flier never walks); the
//! script never sets a turn rate (`sys 125`): KH1's default π/16 a frame.
//!
//! Re:CoM has no `EN0005.SES` (as the Red Nocturne's enemy 3: `Recom.hed` holds no `ENEMY/0005/EN0005.SES`; Re:CoM never
//! loads one): the step writes none ([`crate::enemy::NO_SES`]). Re:CoM's `EN0005.ASD` holds one record (`e005_054`, its
//! rush, attack id 1, `ex_2110_sk` (the root bone) r 0.57: Checked (files)); KH1's charge 209 makes two rows (`0xEC` its
//! hit, `0x15C` a type-2 body row that never hurts): two new records on 209's clip, on Re:CoM's id 1 (its only body
//! attack). Its Thunder (the child) is one of Re:CoM's bullets on Re:CoM's own Yellow Opera Thunder's power (id 0, kind
//! 1: its state 2 `0x140196A80`).

use crate::enemy::{AttackSpec, RowSpec, Spec};

/// Where each constant is pushed in `ex_2130.bd` (float or integer); a name with several offsets must read the same
/// at each.
pub const CONSTS: &[(&str, &[usize])] = &[
    ("walk_speed", &[]),
    ("run_speed", &[]),
    // Engaged `0x36E`: under 250 and under 500 the same round `0x438` (a reposition, then 50 % the charge, else another
    // reposition); farther `0x408` (60 % the Thunder, then two repositions).
    ("close_range", &[0x3BC]),
    ("mid_range", &[0x3D6]),
    ("thunder_chance", &[0x412]),
    ("charge_chance", &[0x446]),
    // The Thunder `0x466`: 205 at blend 8 from frame 8, 206 queued at blend 0 from frame 2, 207 queued at blend 0 from
    // frame 3; after it the idle 220 at blend 8 from frame 8.
    ("cast_blend", &[0x49C]),
    ("cast_start", &[0x4A2]),
    ("hold_blend", &[0x4BA]),
    ("hold_start", &[0x4C0]),
    ("strike_blend", &[0x4D8]),
    ("strike_start", &[0x4DE]),
    ("rest_blend", &[0x4F4, 0x782]),
    ("rest_start", &[0x4FA, 0x788]),
    // The blend the host gives the motion a reaction queues (`Engine::attack_blend`): the scripts' 8.
    ("attack_blend", &[0x4F4]),
    ("thunder_child", &[0x478]),
    // The charge `0x508`: 208 at blend 8 from frame 8 (waited to its end), then 209 at blend 0 with its two rows, up to
    // 150 frames at 25 a frame toward where Sora stood (its height kept unless it is below him; straight down above 500
    // over the floor), done within 30 (flat) or stuck (12.5); then 210 at blend 0 from frame 2 (waited) and 220.
    ("charge_blend", &[0x524]),
    ("charge_start", &[0x52A]),
    ("rush_blend", &[0x546, 0x70E]),
    ("rush_time", &[0x5C0]),
    ("rush_stop", &[0x634]),
    ("rush_high", &[0x6A8]),
    ("rush_speed", &[0x6E8]),
    ("rush_stuck", &[0x726]),
    ("recover_blend", &[0x762]),
    ("recover_start", &[0x768]),
    // The flee (the hit state `0x32C` → `0x7AA`): three legs to points 200..300 from Sora on the other side of his head
    // height (250), the last 50 higher, at 40 a frame (stop 10, 20 frames); then to 100 above the floor under it (probed
    // 20 above; only when that floor is above KH1's world height −400) at 40 (stop 30, 20 frames).
    ("flee_legs", &[0x7AA]),
    ("flee_min", &[0x7C6]),
    ("flee_vert", &[0x7CC]),
    ("flee_last_up", &[0x7F8]),
    ("flee_speed", &[0x810, 0x8AE]),
    ("flee_stop", &[0x816]),
    ("flee_up", &[0x81C, 0x8BA]),
    ("flee_high", &[0x822, 0x8C0]),
    ("flee_time", &[0x828, 0x8C6]),
    ("drop_probe", &[0x860]),
    ("drop_floor_min", &[0x882]),
    ("drop_up", &[0x89A]),
    ("drop_stop", &[0x8B4]),
    // The reposition `0x8D6`: a point 200..300 from Sora, the other side of his head height by 150, flown to at 15 (stop
    // 10, 30 frames), then turning to him, the idle 220 at blend 8 for 25 frames.
    ("move_min", &[0x8D6]),
    ("move_vert", &[0x8DC]),
    ("move_speed", &[0x8F6]),
    ("move_stop", &[0x8FC]),
    ("move_up", &[0x902]),
    ("move_high", &[0x908]),
    ("move_time", &[0x90E]),
    ("move_pause", &[0x928]),
    // `0x934`: the point's band (min + 100) and its head height (50 above Sora's place).
    ("point_band", &[0x94C]),
    ("point_up", &[0x972]),
    ("pause_blend", &[0x9F0]),
    // The idle leg `0xA64` (the Red Nocturne's `0x93E`, the same bytes): 5° a frame, 0.3 down (up below 150 over the
    // floor), [150, 180) frames, 219 at blend 8, 8 a frame.
    ("wander_turn", &[0xA70]),
    ("wander_sink", &[0xA7A]),
    ("wander_low", &[0xA8A]),
    ("flip", &[0xAA0]),
    ("wander_min", &[0xAAE]),
    ("wander_max", &[0xAB4]),
    ("wander_blend", &[0xADE]),
    ("wander_speed", &[0xAF2]),
    // `0xB78`: the idle search (`0x144E(&wp30, 0, 1000, 2)`).
    ("idle_search", &[0xB96]),
    // `0xBE2`'s own: "no motion" 999, the blend 8, the stuck factor 0.3; `0x10E0`'s 0.5.
    ("fly_no_motion", &[0xC6E]),
    ("fly_blend", &[0xC84]),
    ("fly_stuck", &[0xD62]),
    ("stuck_half", &[0x1106]),
    // `0xDBA`: the stats block's `+0x14` (the flight's easing).
    ("air_accel_set", &[0xDD0]),
    // The notice `0xE40` (a room-placed one): not in KH1's world 11; 214 at blend 8.
    ("notice_world", &[0xE44]),
    ("notice_blend", &[0xE6C]),
    // `0x169C`: the request that also ends a wait (`0x1668`).
    ("wait_motion", &[0x16AC]),
    // The appearance `0xFBA`: the 30-frame fade-in, 69 at blend 0, 219 queued at blend 8.
    ("appear_fade", &[0xFC8, 0xFD6]),
    ("appear_blend", &[0x1004]),
    ("appear_queue_blend", &[0x1022]),
    // The hit handler `0x2000`: an attribute-8 hit's EXP bonus `EXP / 10 + 2` (`0xF58`; attributes: not ported).
    ("exp_div", &[0xFA2]),
    ("exp_add", &[0xFAA]),
];

/// The Thunder child's numbers (`ex_2130_00.bd`, entry `0x22A`): it waits while its parent plays 205, needs 206 next and
/// waits while it plays, needs 207 next, then 80 frames; still 207: Sora's place then (`sys 120` of the target) on the
/// floor under it is the mark (KH1's cloud effect 14 200 above it); 30 frames later the strike (effect 11): a cylinder
/// of radius 50 and half-height 128 centred 128 above the mark (`sys 90` on each opponent, `sys 60` the row's hit on each
/// one inside: no clash pass). Checked (files).
pub const THUNDER_CONSTS: &[(&str, &[usize])] = &[
    ("thunder_wait", &[0x2F0]),
    ("thunder_cloud", &[0x352, 0x382]),
    ("thunder_delay", &[0x38C]),
    ("thunder_radius", &[0x3FE]),
    ("thunder_half", &[0x40C]),
    ("thunder_mid", &[0x41E]),
];

/// The Thunder child's integer pushes: its charge effect 2056 (`0x244`), the parent's 205 (`0x256`), 206 (`0x272`,
/// `0x2A0`), 207 (`0x2BC`, `0x302`), the effects 9 / 2058 / 14 / 11 / 12 (`0x28C`, `0x2E6`, `0x364`, `0x39E`, `0x494`),
/// the row's serial 0 (`0x3B8`) and flag 64 (`0x3D4`), the area's type 2 (a cylinder, `0x3F0`).
pub const THUNDER_REQUESTS: &[(u32, usize)] = &[(2056, 0x244), (205, 0x256), (206, 0x272), (9, 0x28C), (206, 0x2A0), (207, 0x2BC), (2058, 0x2E6), (207, 0x302), (14, 0x364), (11, 0x39E), (0, 0x3B8), (64, 0x3D4), (2, 0x3F0), (12, 0x494)];

/// The request words the script pushes (bit 24 an action, bit 16 a held request, bit 30 a queue): the behaviour plays
/// exactly these. The Thunder's 206 / 207 and the charge's 209 / 210 are pushed as 205 / 208 plus 1 and 2. The leash's 70
/// is checked but not played.
pub const REQUESTS: &[(u32, usize)] = &[
    (205, 0x466), // the Thunder: 205, 205 + 1 and 205 + 2 queued, then 220
    (1, 0x4B2),
    (2, 0x4D0),
    (220, 0x4EE),
    (208, 0x50E), // the charge: 208, 208 + 1 (twice), 208 + 2, then 220
    (1, 0x53E),
    (1, 0x706),
    (2, 0x75A),
    (220, 0x77C),
    (2, 0x564), // its rows' mode 2 (`sys 213`), their hit callback `0x79A` (wp+0 = 2)
    (973, 0x572),
    (2, 0x590),
    (973, 0x59E),
    (2, 0x79E),
    (220, 0x806), // the flee's legs and its drop
    (220, 0x8A4),
    (220, 0x8EC), // the reposition, its pause
    (220, 0x9EA),
    (219, 0xAD8), // the idle leg
    (214, 0xE66), // the notice, then 220
    (220, 0xE82),
    (219, 0xFBA), // the appearance: 69 (an action), 219 queued
    (0x0100_0045, 0xFFE),
    (0x4000_0000, 0x101A),
    (6, 0x16AC),           // the request that ends a wait
    (0x4000_0046, 0x1BB8), // the leash's vanish (not ported)
];

/// KH1's charge 209 (`0x508`: rows `0xEC` (the hit, type 0) and `0x15C` (a type-2 body row that never hurts), one hit
/// serial, mode 2) on Re:CoM's only body attack, id 1 (`e005_054`'s record, its rush: decision M10's "power of the Re:CoM
/// attack of the same kind"); its body row on the same.
pub const CHARGE: RowSpec = RowSpec { at: 0xEC, recom_id: 1 };
/// The charge's body row (`0x15C`, type 2: it only clashes).
pub const CHARGE_BODY: RowSpec = RowSpec { at: 0x15C, recom_id: 1 };

/// Its one attack, in the behaviour's order (`kh1_core::yellowopera::Attack`).
pub const ATTACKS: &[AttackSpec] = &[AttackSpec { name: "charge", request: 209, rows: &[CHARGE, CHARGE_BODY] }];

/// The Yellow Opera.
pub const SPEC: Spec = Spec {
    name: "yellowopera",
    recom_id: 5,
    kh1: "xa_ex_2130",
    script: b"ex_2130.bd",
    prefix: "e005_",
    recom_slots: 79,
    asd: (1, 0x6C, 0x10C),
    consts: CONSTS,
    requests: REQUESTS,
    // The LV1 / LV2 stat records (`sys 45` with `0x14` / `0x80`).
    records: &[0x14, 0x80],
    attacks: ATTACKS,
    manifest: "kh1-yellowopera.json",
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enemy::{build, Inputs};

    /// The player's files: `KH1_M11_DIR` (KH1's `xa_*.mdls` / `.mset` and Re:CoM's `ENEMY__NNNN__ENNNNN.*`; no `.SES`
    /// for enemy 5) and `KH1_TEST_DATA` (`btltbl.bin`).
    fn inputs() -> Option<Inputs> {
        crate::enemy::test_inputs(&SPEC, false)
    }

    #[test]
    fn builds_from_the_players_files() {
        let Some(inp) = inputs() else { return };
        assert!(inp.ses.is_empty(), "Re:CoM has no EN0005.SES");
        let b = build(&SPEC, &inp).unwrap();
        let get = |n: &str| b.consts.iter().find(|c| c.0 == n).map(|c| c.1).unwrap_or_else(|| panic!("{n}"));
        println!("clips: {:?}", b.clips.iter().map(|c| (c.request, c.frames, c.root, c.looped, c.source)).collect::<Vec<_>>());
        println!("records: {:?}", b.records);
        // The script's numbers (the disassembly's).
        assert_eq!((get("close_range"), get("mid_range"), get("thunder_chance"), get("charge_chance"), get("idle_search")), (250.0, 500.0, 0.6, 0.5, 1000.0));
        assert_eq!((get("cast_blend"), get("cast_start"), get("hold_blend"), get("hold_start"), get("strike_blend"), get("strike_start"), get("rest_blend"), get("rest_start")), (8.0, 8.0, 0.0, 2.0, 0.0, 3.0, 8.0, 8.0));
        assert_eq!((get("charge_blend"), get("charge_start"), get("rush_blend"), get("rush_time"), get("rush_stop"), get("rush_high"), get("rush_speed"), get("rush_stuck"), get("recover_blend"), get("recover_start")), (8.0, 8.0, 0.0, 150.0, 30.0, 500.0, 25.0, 12.5, 0.0, 2.0));
        assert_eq!((get("flee_legs"), get("flee_min"), get("flee_vert"), get("flee_last_up"), get("flee_speed"), get("flee_stop"), get("flee_up"), get("flee_high"), get("flee_time")), (3.0, 200.0, 250.0, 50.0, 40.0, 10.0, 0.0, 500.0, 20.0));
        assert_eq!((get("drop_probe"), get("drop_floor_min"), get("drop_up"), get("drop_stop")), (20.0, -400.0, 100.0, 30.0));
        assert_eq!((get("move_min"), get("move_vert"), get("move_speed"), get("move_stop"), get("move_up"), get("move_high"), get("move_time"), get("move_pause")), (200.0, 150.0, 15.0, 10.0, 0.0, 500.0, 30.0, 25.0));
        assert_eq!((get("point_band"), get("point_up"), get("pause_blend")), (100.0, 50.0, 8.0));
        assert_eq!((get("wander_turn"), get("wander_sink"), get("wander_low"), get("flip"), get("wander_min"), get("wander_max"), get("wander_blend"), get("wander_speed")), (5.0, 0.3, 150.0, -1.0, 150.0, 180.0, 8.0, 8.0));
        assert_eq!((get("fly_no_motion"), get("fly_blend"), get("fly_stuck"), get("stuck_half"), get("air_accel_set"), get("wait_motion")), (999.0, 8.0, 0.3, 0.5, 0.2, 6.0));
        assert_eq!((get("appear_fade"), get("appear_blend"), get("appear_queue_blend"), get("notice_world"), get("notice_blend")), (30.0, 0.0, 8.0, 11.0, 8.0));
        assert_eq!((get("exp_div"), get("exp_add"), get("thunder_child")), (10.0, 2.0, 0.0));
        assert_eq!((get("walk_speed"), get("turn_rate"), get("weight")), (2.0, std::f32::consts::PI / 16.0, 1.0));
        // The child's: 80 frames in 207, the cloud 200 above the mark, the strike 30 frames later: radius 50, half-height
        // 128, centred 128 above the mark; Re:CoM's own Yellow Opera Thunder's power (id 0, kind 1).
        assert_eq!((get("thunder_wait"), get("thunder_cloud"), get("thunder_delay"), get("thunder_radius"), get("thunder_half"), get("thunder_mid")), (80.0, 200.0, 30.0, 50.0, 128.0, 128.0));
        assert_eq!((get("thunder_recom_id"), get("thunder_kind")), (0.0, 1.0));
        // Every request the behaviour plays has a clip; the reactions' too.
        for r in [0u16, 69, 0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D, 0x4E, 0x50, 0x51, 205, 206, 207, 208, 209, 210, 214, 219, 220] {
            assert!(b.clips.iter().any(|x| x.request == r), "request {r}");
        }
        let a0 = crate::ann::Ann::parse(&inp.ann).unwrap();
        let a1 = crate::ann::Ann::parse(&b.ann).unwrap();
        for i in 0..SPEC.recom_slots {
            assert_eq!(a0.slots[i], a1.slots[i], "slot {i}");
        }
        // The .ASD: Re:CoM's record kept, the charge's two new ones on 209's clip (the body row's never hurts: type 2).
        assert_eq!(&b.asd[..0x6C + 0xA0][0x6C..], &inp.asd[0x6C..0x6C + 0xA0]);
        assert_eq!(u32::from_le_bytes(b.asd[0x10..0x14].try_into().unwrap()), 3);
        assert_eq!(b.clash.len(), 1);
        assert_eq!(b.clash[0].iter().map(|c| (c.clash_type, c.priority)).collect::<Vec<_>>(), vec![(0, 30), (2, 30)]);
        assert!(b.ses.is_empty(), "no .SES (Re:CoM has none)");
        assert!(b.manifest.contains("\"format\": \"kh1-enemy 1\"") && b.manifest.contains("\"enemy\": \"yellowopera\""));
        std::fs::write(std::env::temp_dir().join("kh1-yellowopera-test.json"), &b.manifest).unwrap();
        println!("parts {:?}", b.parts);
    }
}
