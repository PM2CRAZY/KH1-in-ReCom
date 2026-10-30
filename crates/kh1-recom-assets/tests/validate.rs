//! Validation against Re:CoM's own files. Needs `KH1_TEST_DATA` with `xa_ex_0010.mdls`,
//! `xa_ex_0010.mset`, `PL0001.ANN`, `PL0001.MDL` (the user's own); skips when unset.
//!
//! (a) decode → encode of Re:CoM's `PL0001.ANN` is byte-identical (file and every clip);
//! (b) KH1 entries 98 / 117 / 4 / 8 converted at 60 fps, sampled with the decoder at even
//!     frames, match Square Enix's 30 fps bakes p001_055 / 070 / 020 / 004 at their key frames
//!     on every bone (Root and the 134 mapped bones); and the new clip reproduces KH1's pose
//!     at every 60 Hz frame through Re:CoM's composition rule;
//! (c) the extended pack parses, has 161 + 118 slots, Re:CoM's clips are untouched;
//! (d) the loop rule agrees with every Re:CoM bake of a KH1 motion.

#![allow(clippy::needless_range_loop)]

use std::path::PathBuf;

use kh1_formats::mset::MotionSet;
use kh1_recom_assets::ann::{Ann, Clip};
use kh1_recom_assets::build::{build, cycle_error, BuildOptions, Inputs, LOOP_TOLERANCE_M, RECOM_SORA_SLOTS};
use kh1_recom_assets::convert::{ClipSpec, Converter, Target};
use kh1_recom_assets::math::{angle_deg, dist, m3, rot_xyz, tr, vm, I3};
use kh1_recom_assets::mdl::RecomSkeleton;

struct Data {
    mdls: Vec<u8>,
    mset: Vec<u8>,
    ann: Vec<u8>,
    mdl: Vec<u8>,
    /// PL0002.ANN / .MDL (the Keyblade), if present.
    wep: Option<(Vec<u8>, Vec<u8>)>,
}

fn data() -> Option<Data> {
    let dir = PathBuf::from(std::env::var_os("KH1_TEST_DATA")?);
    let rd = |n: &str| match std::fs::read(dir.join(n)) {
        Ok(b) => Some(b),
        Err(_) => {
            eprintln!("skipping: {} not found", dir.join(n).display());
            None
        }
    };
    let wep = std::fs::read(dir.join("PL0002.ANN")).ok().zip(std::fs::read(dir.join("PL0002.MDL")).ok());
    Some(Data { mdls: rd("xa_ex_0010.mdls")?, mset: rd("xa_ex_0010.mset")?, ann: rd("PL0001.ANN")?, mdl: rd("PL0001.MDL")?, wep })
}

#[test]
fn a_round_trip_is_byte_identical() {
    let Some(d) = data() else { return };
    let ann = Ann::parse(&d.ann).unwrap();
    assert_eq!(ann.slots.len(), RECOM_SORA_SLOTS);
    let clips = ann.slots.iter().filter(|s| s.clip.is_some()).count();
    assert_eq!(clips, 69);
    assert_eq!(ann.to_bytes(), d.ann, "whole file");
    // Every clip on its own, and the slot name agrees with the clip name.
    let n = ann.slots.len();
    let mut p = 4 + 20 * n;
    for (i, s) in ann.slots.iter().enumerate() {
        let size = u32::from_le_bytes(d.ann[4 + 20 * i..8 + 20 * i].try_into().unwrap()) as usize;
        if let Some(c) = &s.clip {
            assert_eq!(c.to_bytes(), &d.ann[p..p + size], "slot {i}");
            assert_eq!(c.name_str(), s.name_str());
            assert_eq!(ann.find(&c.name_str()), Some(i));
            p += size;
        }
    }
    eprintln!("(a) {clips} clips in {n} slots: decode → encode byte-identical ({} bytes)", d.ann.len());
}

/// Re:CoM world (U, S, P) per bone of a decoded clip at frame t, by `FUN_14023EB00`'s rule;
/// `ext` is bone 0's external parent (the weapon's `WeaponR`).
fn recom_worlds(sk: &RecomSkeleton, c: &Clip, t: f64, sample_root: bool, ext: Option<&Target>) -> Vec<Target> {
    let mut w: Vec<Target> = Vec::with_capacity(sk.bones.len());
    for (b, bone) in sk.bones.iter().enumerate() {
        let (rl, tl, s) = if b == 0 && !sample_root {
            (I3, [0.0; 3], [1.0; 3])
        } else {
            let x = c.sample(b, t);
            (rot_xyz(x.r[0], x.r[1], x.r[2]), x.t, x.s)
        };
        let parent = if bone.parent < 0 { ext } else { Some(&w[bone.parent as usize]) };
        let tg = match parent {
            None => Target { u: rl, s, p: tl },
            Some(p) => {
            let mut pw = p.u;
            for k in 0..3 {
                for v in pw[k].iter_mut() {
                    *v *= p.s[k];
                }
            }
            let off = vm(tl, &pw);
            Target { u: m3(&rl, &p.u), s, p: [p.p[0] + off[0], p.p[1] + off[1], p.p[2] + off[2]] }
            }
        };
        w.push(tg);
    }
    w
}

#[derive(Default, Debug)]
struct Stat {
    rot_n: usize,
    rot_max: f64,
    pos_n: usize,
    pos_max_mm: f64,
    scl_n: usize,
    scl_max: f64,
}

#[test]
fn b_matches_recom_bakes() {
    let Some(d) = data() else { return };
    let ms = MotionSet::parse(&d.mset).unwrap();
    let ann = Ann::parse(&d.ann).unwrap();
    let sk = RecomSkeleton::parse(&d.mdl).unwrap();
    let conv = Converter::new(&d.mdls, sk.clone()).unwrap();
    assert_eq!(conv.map().iter().filter(|m| m.is_some()).count(), 134);
    for (entry, bake, looped, root) in [(98usize, "p001_055", false, true), (117, "p001_070", false, true), (4, "p001_020", true, false), (8, "p001_004", false, false)] {
        let m = &ms.motions[entry];
        let ours = conv.convert(m, &ClipSpec { name: "p001_999".into(), looped, root_motion: root, blend: 8 }).unwrap();
        let ours = Clip::parse(&ours.to_bytes()).unwrap();
        assert_eq!(ours.fps(), 60);
        assert_eq!(ours.frames as u32, if looped { m.frames } else { m.frames + 1 });
        let theirs = ann.slots[ann.find(bake).unwrap()].clip.as_ref().unwrap();
        assert_eq!(theirs.fps(), 30);
        assert_eq!(theirs.looped(), looped, "{bake} loop flag");
        let mut st = Stat::default();
        for f in 0..theirs.frames as usize {
            let t = 2.0 * f as f64;
            for b in 0..sk.bones.len() {
                let a = ours.sample(b, t);
                let e = theirs.sample(b, f as f64);
                if (3..6).all(|c| theirs.keyed(b, c, f)) {
                    let q = m3(&rot_xyz(a.r[0], a.r[1], a.r[2]), &tr(&rot_xyz(e.r[0], e.r[1], e.r[2])));
                    st.rot_n += 1;
                    st.rot_max = st.rot_max.max(angle_deg(&q));
                }
                for c in 0..3 {
                    if theirs.keyed(b, c, f) {
                        st.pos_n += 1;
                        st.pos_max_mm = st.pos_max_mm.max((a.t[c] - e.t[c]).abs() * 1000.0);
                    }
                    if theirs.chans() == 9 && theirs.keyed(b, 6 + c, f) {
                        st.scl_n += 1;
                        st.scl_max = st.scl_max.max((a.s[c] - e.s[c]).abs());
                    }
                }
            }
        }
        // Self-check at every 60 Hz frame: the decoded clip through Re:CoM's composition vs
        // KH1's pose (world position and rotation of every bone, Root included).
        let mut rig = conv.rig();
        rig.start_motion(m);
        let (mut wp, mut wr) = (0f64, 0f64);
        for f in 0..ours.frames as usize {
            let want = conv.targets(&mut rig, m, f as f32, root);
            let got = recom_worlds(&sk, &ours, f as f64, true, None);
            for b in 0..want.len() {
                wp = wp.max(dist(want[b].p, got[b].p) * 1000.0);
                wr = wr.max(angle_deg(&m3(&want[b].u, &tr(&got[b].u))));
            }
        }
        eprintln!(
            "(b) KH1 {entry} → 60 fps vs {bake} @30: rot n {} max {:.4}°, trans n {} max {:.4} mm, scale n {} max {:.2e} | every 60 Hz frame vs KH1 pose: world pos max {:.4} mm, rot max {:.4}°",
            st.rot_n, st.rot_max, st.pos_n, st.pos_max_mm, st.scl_n, st.scl_max, wp, wr
        );
        assert!(st.rot_n > 500 && st.pos_n > 3000, "{st:?}");
        assert!(st.rot_max < 0.03, "{entry}: {st:?}");
        assert!(st.pos_max_mm < 0.1, "{entry}: {st:?}");
        assert!(st.scl_max < 1e-4, "{entry}: {st:?}");
        assert!(wp < 0.1 && wr < 0.02, "{entry}: self-check {wp} mm {wr} deg");
    }
}

#[test]
fn c_extended_pack() {
    let Some(d) = data() else { return };
    let t0 = std::time::Instant::now();
    let built = build(&Inputs { mset: d.mset.clone(), mdls: d.mdls.clone(), ann: d.ann.clone(), mdl: d.mdl.clone(), weapon: d.wep.clone() }, &BuildOptions::default()).unwrap();
    let el = t0.elapsed();
    let orig = Ann::parse(&d.ann).unwrap();
    let ext = Ann::parse(&built.bytes).unwrap();
    assert_eq!(ext.slots.len(), RECOM_SORA_SLOTS + 118);
    assert_eq!(built.entries.len(), 118);
    assert_eq!(built.bytes.len() % 16, 0);
    for i in 0..RECOM_SORA_SLOTS {
        assert_eq!(ext.slots[i], orig.slots[i], "slot {i}");
        if let Some(c) = &ext.slots[i].clip {
            assert_eq!(c.to_bytes(), orig.slots[i].clip.as_ref().unwrap().to_bytes());
        }
    }
    for e in &built.entries {
        let c = ext.slots[e.slot].clip.as_ref().unwrap();
        assert_eq!(c.name_str(), e.name);
        assert_eq!(ext.find(&e.name), Some(e.slot));
        assert_eq!(e.slot, RECOM_SORA_SLOTS + e.kh1_entry);
        assert_eq!((c.fps(), c.bones, c.looped()), (60, 135, e.looped));
        assert_eq!(c.frames as usize, e.frames);
    }
    if let (Some((wann, _)), Some(w)) = (&d.wep, &built.weapon) {
        let orig = Ann::parse(wann).unwrap();
        let ext = Ann::parse(&w.bytes).unwrap();
        assert_eq!((orig.slots.len(), ext.slots.len()), (158, 279));
        assert_eq!(&ext.slots[..158], &orig.slots[..]);
        assert!(ext.slots[158..161].iter().all(|s| s.clip.is_none()));
        for e in &built.entries {
            let c = ext.slots[e.slot].clip.as_ref().unwrap();
            assert_eq!((c.name_str(), c.bones, c.frames as usize, c.looped(), c.fps()), (format!("w001_{}", e.slot), 12, e.frames, e.looped, 60));
            assert_eq!(e.weapon_name.as_deref(), Some(c.name_str().as_str()));
        }
        let json = built.manifest_json(&[]);
        assert!(json.contains("\"weapon_name\": \"w001_259\"") && json.contains("\"weapon_base_slots\": 158"));
        eprintln!("(c) Keyblade pack {} bytes ({} → {} slots, Re:CoM's 158 identical, 158..160 empty)", w.bytes.len(), orig.slots.len(), ext.slots.len());
    }
    let big = built.entries.iter().filter(|e| e.frames >= 256).count();
    let loops = built.entries.iter().filter(|e| e.looped).count();
    let roots = built.entries.iter().filter(|e| e.root_motion).count();
    let json = built.manifest_json(&[("test", "x".into())]);
    assert!(json.contains("\"slot\": 161") && json.contains("\"slot\": 278"));
    // The runtime's shape (crates/kh1-recom motions.rs): a `slots` array of {slot, name,
    // kh1_entry, kh1_request}; the swaps it needs.
    assert!(json.contains("\"slots\": [") && !json.contains("\"slots\": 2"));
    for (entry, req) in [(0usize, Some(0u16)), (2, Some(1)), (5, Some(2)), (98, Some(200)), (99, Some(201)), (100, Some(202)), (101, Some(203))] {
        let e = built.entries.iter().find(|e| e.kh1_entry == entry).unwrap();
        assert_eq!(e.kh1_request, req, "entry {entry}: requests {:?}", e.requests);
    }
    for entry in [0usize, 1, 2, 3, 5, 6, 98, 99, 100, 101] {
        assert!(json.contains(&format!("\"name\": \"p001_{}\", \"kh1_entry\": {entry},", 161 + entry)), "entry {entry}");
    }
    eprintln!(
        "(c) extended pack {} bytes ({} → {} slots), {} new clips ({} bytes, {} u16-key clips, {} loops, {} root), built in {:.2}s",
        built.bytes.len(),
        RECOM_SORA_SLOTS,
        ext.slots.len(),
        built.entries.len(),
        built.entries.iter().map(|e| e.bytes).sum::<usize>(),
        big,
        loops,
        roots,
        el.as_secs_f64()
    );
}

/// Re:CoM slot → KH1 entry for every Re:CoM clip that is a bake of a KH1 motion
/// (`docs/motion-conversion.md`, table from `clip_origin.py`; 155-157 bake 76/77).
const BAKES: [(usize, usize); 46] = [
    (4, 8), (7, 11), (8, 18), (10, 20), (11, 21), (12, 22), (13, 23), (14, 26), (15, 14), (16, 15), (17, 16), (20, 4), (31, 1), (32, 58),
    (40, 61), (41, 64), (42, 67), (44, 66), (45, 74), (55, 98), (56, 99), (57, 100), (58, 101), (59, 103), (60, 102), (61, 104), (62, 54),
    (63, 57), (64, 56), (65, 55), (66, 79), (67, 82), (68, 81), (69, 80), (70, 117), (81, 72), (101, 114), (103, 106), (104, 116), (150, 84),
    (151, 85), (152, 88), (153, 90), (154, 75), (155, 76), (156, 77),
];

#[test]
fn d_loop_rule_agrees_with_recom_bakes() {
    let Some(d) = data() else { return };
    let ms = MotionSet::parse(&d.mset).unwrap();
    let ann = Ann::parse(&d.ann).unwrap();
    let conv = Converter::new(&d.mdls, RecomSkeleton::parse(&d.mdl).unwrap()).unwrap();
    let mut worst_loop = 0f64;
    let mut best_once = f64::MAX;
    let mut plus_one = Vec::new();
    for (slot, entry) in BAKES {
        let c = ann.slots[slot].clip.as_ref().unwrap();
        let err = cycle_error(&conv, &ms.motions[entry]);
        assert_eq!(err <= LOOP_TOLERANCE_M, c.looped(), "slot {slot} / entry {entry}: cycle error {err} m, Re:CoM loop {}", c.looped());
        if c.looped() {
            worst_loop = worst_loop.max(err);
        } else {
            best_once = best_once.min(err);
        }
        // Frame-count convention of the bakes: N/2 frames (one more for some play-once).
        let half = ms.motions[entry].frames / 2;
        assert!(c.frames as u32 == half || (!c.looped() && c.frames as u32 == half + 1), "slot {slot}: {} frames, N {}", c.frames, half * 2);
        if c.frames as u32 != half {
            plus_one.push(slot);
        }
    }
    eprintln!("(d) loop rule matches all {} bakes: loops' cycle error ≤ {:.4} m, play-once ≥ {:.4} m; bakes with N/2 + 1 frames: {plus_one:?}", BAKES.len(), worst_loop, best_once);
}


#[test]
fn e_keyblade_matches_recom_bakes() {
    let Some(d) = data() else { return };
    let Some((wann, wmdl)) = &d.wep else {
        eprintln!("skipping: no PL0002.ANN/.MDL");
        return;
    };
    let ms = MotionSet::parse(&d.mset).unwrap();
    let ann = Ann::parse(wann).unwrap();
    assert_eq!(ann.slots.len(), 158);
    assert_eq!(ann.to_bytes(), *wann, "PL0002.ANN round trip");
    let sk = RecomSkeleton::parse(&d.mdl).unwrap();
    let conv = Converter::new(&d.mdls, sk).unwrap();
    let wsk = RecomSkeleton::parse(wmdl).unwrap();
    let w = conv.weapon(wsk.clone(), "WeaponR").unwrap();
    let want: Vec<Option<usize>> = [101usize, 280, 282].into_iter().chain(283..=291).map(Some).collect();
    assert_eq!(w.map, want, "weapon bone map");
    let pl1 = Ann::parse(&d.ann).unwrap();
    let mut others = Vec::new();
    for (slot, entry) in BAKES {
        let Some(theirs) = &ann.slots.get(slot).and_then(|s| s.clip.clone()) else { continue };
        let bake = theirs.name_str();
        let looped = pl1.slots[slot].clip.as_ref().unwrap().looped();
        let m = &ms.motions[entry];
        let ours = conv.convert_weapon(&w, m, &ClipSpec { name: "w001_999".into(), looped, root_motion: false, blend: 8 }).unwrap();
        let ours = Clip::parse(&ours.to_bytes()).unwrap();
        let mut st = Stat::default();
        for f in 0..theirs.frames as usize {
            for b in 0..wsk.bones.len() {
                let a = ours.sample(b, 2.0 * f as f64);
                let e = theirs.sample(b, f as f64);
                if (3..6).all(|c| theirs.keyed(b, c, f)) {
                    let q = m3(&rot_xyz(a.r[0], a.r[1], a.r[2]), &tr(&rot_xyz(e.r[0], e.r[1], e.r[2])));
                    st.rot_n += 1;
                    if std::env::var_os("KEYBLADE_DEBUG").is_some() && angle_deg(&q) > 0.05 {
                        eprintln!("  {bake} f{f} bone {b}: {:.3}° ours {:?} theirs {:?}", angle_deg(&q), a.r.map(f64::to_degrees), e.r.map(f64::to_degrees));
                    }
                    st.rot_max = st.rot_max.max(angle_deg(&q));
                }
                for c in 0..3 {
                    if theirs.keyed(b, c, f) {
                        st.pos_n += 1;
                        st.pos_max_mm = st.pos_max_mm.max((a.t[c] - e.t[c]).abs() * 1000.0);
                    }
                }
            }
        }
        // Every 60 Hz frame: the decoded weapon clip hung from KH1's hand vs KH1's blade bones.
        let mut rig = conv.rig();
        rig.start_motion(m);
        let (mut wp, mut wr) = (0f64, 0f64);
        for f in 0..ours.frames as usize {
            rig.pose(m, f as f32);
            let tg = conv.weapon_targets(&w, &rig);
            let got = recom_worlds(&wsk, &ours, f as f64, false, Some(&tg[0]));
            for b in 0..tg.len() {
                wp = wp.max(dist(tg[b].p, got[b].p) * 1000.0);
                wr = wr.max(angle_deg(&m3(&tg[b].u, &tr(&got[b].u))));
            }
        }
        assert!(wp < 0.1 && wr < 0.02, "{entry}: self-check {wp} mm {wr} deg");
        assert!(st.pos_max_mm < 0.1, "{entry}: {st:?}");
        if [98, 117, 8].contains(&entry) {
            eprintln!(
                "(e) Keyblade KH1 {entry} vs {bake}: rot n {} max {:.4}°, trans n {} max {:.4} mm | every 60 Hz frame: world pos {:.4} mm, rot {:.4}°",
                st.rot_n, st.rot_max, st.pos_n, st.pos_max_mm, wp, wr
            );
            assert!(st.rot_n > 90 && st.pos_n > 400, "{st:?}");
            assert!(st.rot_max < 0.05, "{entry}: {st:?}");
        }
        others.push((entry, (st.rot_max * 1000.0).round() / 1000.0));
    }
    others.sort_by(|a, b| b.1.total_cmp(&a.1));
    let within = others.iter().filter(|o| o.1 < 0.05).count();
    eprintln!("(e) all {} Keyblade bakes of KH1 motions: {within} within 0.05°; worst (entry, °): {:?}", others.len(), &others[..others.len().min(12)]);
    // Known exceptions (kh1-anim, not the encoder): entry 4, the only motion whose blade bone
    // 283 copies a helper's orientation (mode 2, 12-helper rig); entry 74, one of the spine-aim
    // residual motions of kh1-posing.md §8.
    assert!(others.iter().all(|&(e, r)| e == 4 || r < 0.2), "{others:?}");
    assert_eq!(others.len(), 36);
}
