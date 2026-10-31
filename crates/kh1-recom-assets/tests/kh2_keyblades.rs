//! KH2's Keyblades (decision 25) against the owner's files. Needs (skipped when missing):
//! - `KH1_TEST_DATA/keyblades/PLAYER_0002_PL0002.MDL` (Re:CoM's Kingdom Key, the template);
//! - `KH1_TEST_DATA/kh2/` with the KH2 files as the extractor writes them: `03system.bin`,
//!   `00objentry.bin`, `msg_us_sys.bar`, and per Keyblade `obj_W_EX010*.mdlx`, its
//!   `.assets.txt` and its HD texture `obj_W_EX010*.mdlx__0.dds`;
//! - `KH1_TEST_DATA/btltbl.bin` (KH1's Kingdom Key row) for the stats rows.

use std::path::PathBuf;

use kh1_formats::egs::HdAsset;
use kh1_recom_assets::keyblade::{check_kh2_skeleton, convert_kh2, group_for_card, kh2_facing, kh2_template_distance, mod_files, JOINTS};
use kh1_recom_assets::kh2_keyblades::{blade_far, blade_length, find, reference_skeleton, KH2_KEYBLADES};
use kh1_recom_assets::mdl::{Mdl, MeshPacket};
use kh1_recom_assets::rtm::{tim2_decode8, Rtm};
use kh1_recom_assets::stats::{keyblades_json, Kh2Row};

fn data() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("KH1_TEST_DATA")?))
}

fn kh2(name: &str) -> Option<Vec<u8>> {
    let p = data()?.join("kh2").join(name.replace('/', "_"));
    match std::fs::read(&p) {
        Ok(b) => Some(b),
        Err(_) => {
            eprintln!("skipping: {} not found", p.display());
            None
        }
    }
}

/// A KH2 file and its HD assets (`<file>.assets.txt`: `name<TAB>tag<TAB>size`).
fn kh2_full(name: &str) -> Option<(Vec<u8>, Vec<HdAsset>)> {
    let data = kh2(name)?;
    let base = data_dir_kh2()?.join(name.replace('/', "_"));
    let list = std::fs::read_to_string(format!("{}.assets.txt", base.display())).ok()?;
    let mut assets = Vec::new();
    for l in list.lines() {
        let f: Vec<&str> = l.split('\t').collect();
        let tag = u32::from_str_radix(f[1].trim_start_matches("0x"), 16).unwrap() as i32;
        let b = std::fs::read(format!("{}__{}", base.display(), f[0].trim_start_matches('-'))).ok()?;
        assert_eq!(b.len(), f[2].parse::<usize>().unwrap());
        assets.push(HdAsset { name: f[0].to_string(), orig_offset: tag, data: b });
    }
    Some((data, assets))
}

fn data_dir_kh2() -> Option<PathBuf> {
    Some(data()?.join("kh2"))
}

fn template() -> Option<Mdl> {
    let b = std::fs::read(data()?.join("keyblades").join("PLAYER_0002_PL0002.MDL")).ok()?;
    Some(Mdl::parse(&b).unwrap())
}

fn found() -> Option<Vec<kh1_recom_assets::kh2_keyblades::Kh2Keyblade>> {
    Some(find(&kh2("03system.bin")?, &kh2("00objentry.bin")?, &kh2("msg/us/sys.bar")?).unwrap())
}

/// A strip-order triangle of a list: corners, Re:CoM facing flag, bone of the drawing vertex.
type Tri = ([[f32; 3]; 3], u32);

fn strip_tris(list: &[MeshPacket]) -> Vec<Tri> {
    let mut out = Vec::new();
    for p in list {
        for i in 2..p.verts.len() {
            if p.verts[i].draws() {
                out.push(([p.verts[i - 2].pos(), p.verts[i - 1].pos(), p.verts[i].pos()], p.verts[i].q0[0]));
            }
        }
    }
    out
}

/// The items, stat rows and models, from KH2's tables (offsets in `kh2_keyblades`' notes).
#[test]
fn a_the_six_and_their_kh2_numbers() {
    let Some(f) = found() else { return };
    let got: Vec<(u8, &str, u16, u16, u8, u8, &str)> = f.iter().map(|k| (k.card, k.name.as_str(), k.item, k.stat, k.strength, k.magic, k.model.as_str())).collect();
    for g in &got {
        eprintln!("{g:?}");
    }
    assert_eq!(
        got,
        vec![
            (0, "Kingdom Key", 0x29, 80, 3, 1, "W_EX010"),
            (0x13, "Star Seeker", 0x1E0, 123, 3, 1, "W_EX010_30"),
            (0x14, "Monochrome", 0x1E5, 128, 3, 2, "W_EX010_60"),
            (0x15, "Follow the Wind", 0x1E6, 129, 3, 1, "W_EX010_70"),
            (0x16, "Hidden Dragon", 0x1E1, 124, 2, 2, "W_EX010_40"),
            (0x17, "Two Become One", 0x21F, 148, 5, 4, "W_EX010_P0"),
            (0x18, "Bond of Flame", 0x1F2, 141, 4, 4, "W_EX010_K0"),
        ]
    );
    assert_eq!(f[1..].iter().map(|k| k.card).collect::<Vec<_>>(), KH2_KEYBLADES.iter().map(|k| k.0).collect::<Vec<_>>());
}

/// KH2's Kingdom Key is Re:CoM's PL0002: same skeleton below the grip, same bone-space mesh
/// (× 0.01), same UVs; its translucent group is PL0002's list B.
#[test]
fn b_kh2_kingdom_key_is_pl0002() {
    let (Some(t), Some((kk, assets))) = (template(), kh2_full("obj/W_EX010.mdlx")) else { return };
    let reference = reference_skeleton(&kk).unwrap();
    check_kh2_skeleton(&reference, &reference, &t).unwrap();
    let d = kh2_template_distance(&t, &kk).unwrap();
    eprintln!("W_EX010 vs PL0002: worst {:.4} mm", d * 1000.0);
    assert!(d < 0.0001, "{d}");
    let c = convert_kh2(&t, "W_EX010", &kk, &assets, &reference).unwrap();
    eprintln!("{}", c.report.line());
    let ours = Mdl::parse(&c.mdl).unwrap();
    let (oa, ob) = (strip_tris(&ours.parts[0].list_a), strip_tris(ours.parts[0].list_b.as_deref().unwrap()));
    let (ra, rb) = (strip_tris(&t.parts[0].list_a), strip_tris(t.parts[0].list_b.as_deref().unwrap()));
    assert_eq!((oa.len(), ob.len()), (ra.len(), rb.len()));
    assert_eq!((oa.len(), ob.len()), (322, 70));
    // Corner for corner (any rotation, either order), and whether the facing agrees.
    let close = |p: &[f32; 3], q: &[f32; 3]| (0..3).all(|k| (p[k] - q[k]).abs() < 2e-4);
    let matches = |o: &[Tri], r: &[Tri]| {
        let (mut same, mut agree) = (0, 0);
        for (c1, f1) in o {
            for (c2, f2) in r {
                let cyc = (0..3).find(|&s| (0..3).all(|i| close(&c1[i], &c2[(i + s) % 3]))).map(|_| true);
                let rev = || (0..3).find(|&s| (0..3).all(|i| close(&c1[i], &c2[(s + 3 - i) % 3]))).map(|_| false);
                if let Some(same_order) = cyc.or_else(rev) {
                    same += 1;
                    agree += (same_order == ((f1 & 0x20) == (f2 & 0x20))) as usize;
                    break;
                }
            }
        }
        (same, agree)
    };
    let (sa, aa) = matches(&oa, &ra);
    let (sb, ab) = matches(&ob, &rb);
    eprintln!("list A: {sa} of 322 triangles corner for corner in PL0002's list A, facing agrees on {aa}; list B: {sb} of 70, {ab}");
    assert_eq!(sb, 70, "list B is PL0002's list B");
    assert!(sa >= 200 && aa * 10 >= sa * 8, "{sa} {aa}");
    // Every list-A corner is a PL0002 list-A vertex (the rest of the triangles split their quads
    // along the other diagonal).
    let rv: Vec<[f32; 3]> = ra.iter().flat_map(|t| t.0).collect();
    assert!(oa.iter().flat_map(|t| t.0).all(|p| rv.iter().any(|q| close(&p, q))));
    // The reach reference: KH2's and Re:CoM's Kingdom Keys reach as far.
    let len = blade_length(&kk).unwrap();
    assert!((len - 0.7790).abs() < 1e-3, "{len}");
}

/// The six convert on PL0002's skeleton: bones 2..11 only, files read back, unique texture names,
/// UVs inside the texture, part 1 holds every triangle, the raw RTM entry reads back.
#[test]
fn c_all_six_convert() {
    let (Some(t), Some(f), Some((kk, _))) = (template(), found(), kh2_full("obj/W_EX010.mdlx")) else { return };
    let reference = reference_skeleton(&kk).unwrap();
    let kk_len = blade_length(&kk).unwrap();
    let mut names = std::collections::BTreeSet::new();
    let mut n = 0;
    for k in &f[1..] {
        let Some((mdlx, assets)) = kh2_full(&k.file()) else { continue };
        let c = convert_kh2(&t, &k.model, &mdlx, &assets, &reference).unwrap_or_else(|e| panic!("{}: {e}", k.name));
        let r = &c.report;
        let len = blade_length(&mdlx).unwrap();
        eprintln!("{:#x} {}: {} | blade {len:.4} m, blade_far {:.4}", k.card, k.name, r.line(), blade_far(-3.7, len, kk_len));
        assert!(r.joints.iter().all(|j| JOINTS.contains(j)));
        let m = Mdl::parse(&c.mdl).unwrap();
        assert_eq!(m.write().unwrap(), c.mdl);
        for (p, tp) in m.parts.iter().zip(&t.parts) {
            assert_eq!((&p.bones, &p.rest), (&tp.bones, &tp.rest), "{}: skeleton", k.name);
        }
        assert_eq!(m.parts[0].texture_names(), c.textures);
        assert_eq!(c.textures, vec![format!("{}_0", k.model.to_ascii_lowercase())]);
        assert_eq!(c.hd[0].0, format!("{}_0_tm2.dds", k.model.to_ascii_lowercase()));
        assert_eq!(c.hd[0].1, assets.iter().find(|a| a.name == "-0.dds").unwrap().data, "HD texture copied as it is");
        let rtm = Rtm::parse(&c.rtm).unwrap();
        let (w, h, _) = tim2_decode8(&rtm.entries[0].data).unwrap();
        assert_eq!((w, h), (256, 128));
        let drawn = |l: &[MeshPacket]| l.iter().flat_map(|p| p.verts.iter()).filter(|v| v.draws()).count();
        assert_eq!(drawn(&m.parts[0].list_a) + drawn(m.parts[0].list_b.as_deref().unwrap_or(&[])), r.triangles);
        assert_eq!(drawn(&m.parts[1].list_a), r.triangles);
        // KH2's own UVs, kept as they are: Two Become One's reach 1.00024 (one step of KH2's
        // 1/4096 fixed point past the edge).
        let edge = 1.0 + 1.0 / 2048.0;
        assert!(r.uv_range.0 >= 0.0 && r.uv_range.1 <= edge && r.uv_range.2 >= 0.0 && r.uv_range.3 <= edge, "uv {:?}", r.uv_range);
        for name in &c.textures {
            assert!(names.insert(name.clone()), "texture name {name} twice");
        }
        let group = group_for_card(k.card).unwrap();
        assert_eq!(group, 19 + k.card as u16);
        let files = mod_files(group, &c).unwrap();
        assert_eq!(files[0].0, format!("PLAYER/{group:04}/PL{group:04}.MDL"));
        assert_eq!(files[1].0, format!("raw/PLAYER/{group:04}/PL{group:04}.RTM"));
        let e = &files[1].1;
        let wd = |o: usize| i32::from_le_bytes(e[o..o + 4].try_into().unwrap());
        assert_eq!((wd(0) as usize, wd(4), wd(8)), (c.rtm.len(), 1, -2));
        assert_eq!(&e[16 + 0x30..16 + 0x30 + c.rtm.len()], &c.rtm[..]);
        n += 1;
    }
    assert_eq!(n, 6, "all six staged");
    // The facing rule.
    assert_eq!((kh2_facing(0x20), kh2_facing(0x30)), (0x20, 0));
}

/// The stats rows: KH2's Strength and Magic, the reach scaled from KH1's Kingdom Key, the rest the
/// Kingdom Key's KH1 row, the groups 38..43.
#[test]
fn d_stats_rows() {
    let (Some(f), Some((kk, _))) = (found(), kh2_full("obj/W_EX010.mdlx")) else { return };
    let Some(btl) = data().and_then(|d| std::fs::read(d.join("btltbl.bin")).ok()) else { return };
    let t = kh1_formats::btltbl::Btltbl::parse(btl).unwrap();
    let kk_len = blade_length(&kk).unwrap();
    let mut rows = Vec::new();
    let mut models = Vec::new();
    for k in &f[1..] {
        let Some(mdlx) = kh2(&k.file()) else { return };
        rows.push(Kh2Row { keyblade: k.clone(), blade_length: blade_length(&mdlx).unwrap(), kk_blade_length: kk_len });
        let model = k.model.to_ascii_lowercase();
        models.push(kh1_recom_assets::keyblade::KeybladeModel { card: k.card, group: group_for_card(k.card).unwrap(), textures: vec![format!("{model}_0")] });
    }
    let j = keyblades_json(&t, &models, &rows).unwrap();
    eprintln!("{j}");
    assert_eq!(j.matches("\"source\": \"kh1\"").count(), 17);
    assert_eq!(j.matches("\"source\": \"kh2\"").count(), 6);
    assert!(j.contains("\"card\": 19, \"name\": \"Star Seeker\", \"source\": \"kh2\", \"kh2_item\": 480, \"kh2_stat\": 123, \"model\": \"W_EX010_30\""));
    assert!(j.contains("\"radius\": 40, \"blade_far\": -4.91"));
    assert!(j.contains("\"blade_near\": 0, \"frac_scale\": 0.79999995, \"strength\": 4, \"crit\": 20, \"crit_bonus\": 0, \"priority\": 30, \"magic\": 4, \"group\": 43, \"textures\": [\"w_ex010_k0_0\"]}"));
}
