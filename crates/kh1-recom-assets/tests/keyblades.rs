//! Keyblade models against the owner's files. Needs `KH1_TEST_DATA/keyblades/` holding the files
//! as the extractor writes them (skipped when missing):
//! - `PLAYER_0002_PL0002.{MDL,RTM}` and `PLAYER_0002_PL0002.RTM__w_ex010_01_tm2.dds` (Re:CoM), and
//!   optionally `PLAYER_0011_PL0011.{MDL,RTM}`, `PLAYER_0101_PL0101.{MDL,RTM}`,
//!   `PLAYER_0001_PL0001.RTM`, `PLAYER_0012_PL0012.RTM` for the round trips;
//! - per KH1 Keyblade `xw_ex_5NN0.wpn`, `xw_ex_5NN0.wpn.assets.txt` (`name<TAB>tag<TAB>size` per
//!   HD asset) and each asset as `xw_ex_5NN0.wpn__<name without the leading '->`.

use std::path::PathBuf;

use kh1_formats::egs::HdAsset;
use kh1_recom_assets::keyblade::{convert, triangle_min_alpha, JOINTS};
use kh1_recom_assets::mdl::Mdl;
use kh1_recom_assets::rtm::{tim2_decode8, Rtm};
use kh1_recom_assets::stats::KH1_KEYBLADES;
use kh1_recom_assets::texture::{decode_dds, Image};

fn dir() -> Option<PathBuf> {
    let d = PathBuf::from(std::env::var_os("KH1_TEST_DATA")?).join("keyblades");
    if d.is_dir() {
        Some(d)
    } else {
        eprintln!("skipping: {} not found", d.display());
        None
    }
}

fn read(name: &str) -> Option<Vec<u8>> {
    std::fs::read(dir()?.join(name)).ok()
}

/// A file and its HD assets.
fn read_full(name: &str) -> Option<(Vec<u8>, Vec<HdAsset>)> {
    let d = dir()?;
    let data = std::fs::read(d.join(name)).ok()?;
    let list = std::fs::read_to_string(d.join(format!("{name}.assets.txt"))).ok()?;
    let mut assets = Vec::new();
    for l in list.lines() {
        let f: Vec<&str> = l.split('\t').collect();
        let tag = u32::from_str_radix(f[1].trim_start_matches("0x"), 16).unwrap() as i32;
        // Only the assets the converter can need are staged; others are skipped.
        if let Ok(b) = std::fs::read(d.join(format!("{name}__{}", f[0].trim_start_matches('-')))) {
            assert_eq!(b.len(), f[2].parse::<usize>().unwrap(), "{name} {}", f[0]);
            assets.push(HdAsset { name: f[0].to_string(), orig_offset: tag, data: b });
        }
    }
    Some((data, assets))
}

/// KH1 model names of the 17 Keyblades (btltbl weapon rows of `KH1_KEYBLADES`' items, read
/// from `KH1_TEST_DATA/btltbl.bin`).
fn models() -> Option<Vec<(u8, String)>> {
    let b = std::fs::read(PathBuf::from(std::env::var_os("KH1_TEST_DATA")?).join("btltbl.bin")).ok()?;
    let t = kh1_formats::btltbl::Btltbl::parse(b).unwrap();
    Some(KH1_KEYBLADES.iter().map(|(card, _, item)| (*card, t.weapon_for_item(*item).unwrap().model)).collect())
}

fn template() -> Option<Mdl> {
    Some(Mdl::parse(&read("PLAYER_0002_PL0002.MDL")?).unwrap())
}

#[test]
fn a_mdl_and_rtm_round_trip_byte_for_byte() {
    let Some(d) = dir() else { return };
    let mut n = 0;
    for f in ["PLAYER_0002_PL0002.MDL", "PLAYER_0011_PL0011.MDL", "PLAYER_0101_PL0101.MDL"] {
        let Ok(b) = std::fs::read(d.join(f)) else { continue };
        let m = Mdl::parse(&b).unwrap_or_else(|e| panic!("{f}: {e}"));
        assert!(m.write().unwrap() == b, "{f}");
        n += 1;
    }
    for f in ["PLAYER_0002_PL0002.RTM", "PLAYER_0001_PL0001.RTM", "PLAYER_0011_PL0011.RTM", "PLAYER_0012_PL0012.RTM", "PLAYER_0101_PL0101.RTM"] {
        let Ok(b) = std::fs::read(d.join(f)) else { continue };
        let r = Rtm::parse(&b).unwrap_or_else(|e| panic!("{f}: {e}"));
        assert!(r.write() == b, "{f}");
        for e in &r.entries {
            tim2_decode8(&e.data).unwrap_or_else(|x| panic!("{f} {}: {x}", e.name_str()));
        }
        n += 1;
    }
    assert!(n >= 2, "PL0002.MDL / .RTM missing");
    // PL0002: what the notes say.
    let m = template().unwrap();
    assert_eq!(m.parts.len(), 2);
    assert_eq!(m.parts[0].texture_names(), ["w_ex010_01"]);
    assert_eq!(m.parts[0].list_a.len() + m.parts[0].list_b.as_ref().unwrap().len(), 10);
    assert_eq!(m.parts[1].list_a.len(), 7);
}

fn tris(list: &[kh1_recom_assets::mdl::MeshPacket]) -> Vec<[[f32; 2]; 3]> {
    let mut out = Vec::new();
    for p in list {
        for i in 0..p.verts.len() {
            if p.verts[i].draws() {
                out.push([p.verts[i - 2].uv().unwrap(), p.verts[i - 1].uv().unwrap(), p.verts[i].uv().unwrap()]);
            }
        }
    }
    out
}

/// The list A / B rule against Square's own split, and PL0002's UV convention against its HD
/// texture (t = 0 at the top row).
#[test]
fn b_list_rule_reproduces_pl0002() {
    let (Some(m), Some(dds)) = (template(), read("PLAYER_0002_PL0002.RTM__w_ex010_01_tm2.dds")) else { return };
    let img = decode_dds(&dds).unwrap();
    assert_eq!((img.w, img.h), (1024, 512));
    let p = &m.parts[0];
    let a = tris(&p.list_a);
    let b = tris(p.list_b.as_ref().unwrap());
    let a_ok = a.iter().filter(|t| triangle_min_alpha(&img, **t) == 255).count();
    let b_ok = b.iter().filter(|t| triangle_min_alpha(&img, **t) < 255).count();
    eprintln!("PL0002: list A {a_ok}/{} opaque, list B {b_ok}/{} touch alpha < 255", a.len(), b.len());
    assert_eq!((a_ok, a.len()), (322, 322));
    assert_eq!((b_ok, b.len()), (67, 70));
}

fn verts(m: &Mdl, part: usize) -> Vec<(usize, [f32; 3])> {
    let p = &m.parts[part];
    p.list_a.iter().chain(p.list_b.iter().flatten()).flat_map(|k| k.verts.iter().map(|v| (v.bone(), v.pos()))).collect()
}

fn nearest(set: &[(usize, [f32; 3])], bone: usize, p: [f32; 3]) -> f32 {
    set.iter().filter(|(b, _)| *b == bone).map(|(_, q)| ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt()).fold(f32::MAX, f32::min)
}

/// KH1's PS2 mesh of a `.wpn`: `(joint, position × 0.01)` per vertex.
fn ps2_verts(wpn: &[u8]) -> Vec<(usize, [f32; 3])> {
    use kh1_formats::mobj::{Item, Vertices};
    let m = kh1_formats::wpn::Wpn::parse(wpn).unwrap().model().unwrap();
    let mut out = Vec::new();
    for mesh in &m.meshes {
        let mut slots = [usize::MAX; 16];
        for p in &mesh.packets {
            for it in &p.items {
                match it {
                    Item::Matrix { joint, slot, .. } => slots[*slot as usize] = *joint as usize,
                    Item::Strip(s) => {
                        if let Vertices::Full(vs) = &s.vertices {
                            for v in vs {
                                let i = v.influences[0];
                                out.push((slots[v.normal_w as usize], [i[0] * 0.01, i[1] * 0.01, i[2] * 0.01]));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    out
}

/// Worst nearest-vertex distance from every vertex of `a` to `b` on the same bone, per bone.
fn worst(a: &[(usize, [f32; 3])], b: &[(usize, [f32; 3])]) -> [f32; 12] {
    let mut w = [0f32; 12];
    for (bn, p) in a {
        w[*bn] = w[*bn].max(nearest(b, *bn, *p));
    }
    w
}

fn mm(w: [f32; 12]) -> [f32; 12] {
    w.map(|x| (x * 1e5).round() / 100.0)
}

/// KH1's own Kingdom Key converted, against Re:CoM's PL0002.
/// - The transform (joint = bone, × 0.01, same axes): KH1's PS2 mesh lands on PL0002's
///   vertices bone for bone (the research's 0.077 mm).
/// - The cvbl (KH1 PC's mesh, what we convert): equal to the PS2 mesh on the bones Square did
///   not remake, so there ours lands on PL0002's within the same tolerance. On the flat cut-out
///   parts `k9` (bone 3) and `k1` (bone 11) KH1 PC has its own re-tessellation: same plane,
///   same outline (bounds within 1 mm), different vertices.
#[test]
fn c_kingdom_key_matches_pl0002() {
    let (Some(t), Some((wpn, assets))) = (template(), read_full("xw_ex_5010.wpn")) else { return };
    let c = convert(&t, "xw_ex_5010", &wpn, &assets).unwrap();
    eprintln!("{}", c.report.line());
    let ours = Mdl::parse(&c.mdl).unwrap();
    assert_eq!(ours.write().unwrap(), c.mdl);
    let (rv, kv, pv) = (verts(&t, 0), verts(&ours, 0), ps2_verts(&wpn));
    let bones = |v: &[(usize, [f32; 3])]| v.iter().map(|x| x.0).collect::<std::collections::BTreeSet<_>>();
    assert_eq!(bones(&rv), bones(&kv));
    assert_eq!(bones(&rv), bones(&pv));
    let (r_p, p_r) = (worst(&rv, &pv), worst(&pv, &rv));
    let (k_p, p_k) = (worst(&kv, &pv), worst(&pv, &kv));
    let (r_k, k_r) = (worst(&rv, &kv), worst(&kv, &rv));
    eprintln!("PS2 mesh vs PL0002, worst per bone (mm): {:?} / {:?}", mm(p_r), mm(r_p));
    eprintln!("cvbl vs PS2 mesh, worst per bone (mm):  {:?} / {:?}", mm(k_p), mm(p_k));
    eprintln!("ours vs PL0002, worst per bone (mm):    {:?} / {:?}", mm(k_r), mm(r_k));
    for b in 2..12 {
        assert!(r_p[b] < 1e-4 && p_r[b] < 1e-4, "PS2 vs PL0002 bone {b}");
        let same_mesh = k_p[b] < 1e-6 && p_k[b] < 1e-6;
        if same_mesh {
            assert!(r_k[b] < 1e-4 && k_r[b] < 1e-4, "ours vs PL0002 bone {b}");
        } else {
            assert!(b == 3 || b == 11, "bone {b}: KH1 PC's mesh differs from the PS2 mesh");
            let z: Vec<f32> = kv.iter().chain(&rv).filter(|v| v.0 == b).map(|v| v.1[2]).collect();
            assert!(z.iter().all(|z| z.abs() < 1e-4), "bone {b} not flat");
            let (tb, ob) = (t.parts[0].bounds[b].map(f32::from_bits), ours.parts[0].bounds[b].map(f32::from_bits));
            assert!(tb.iter().zip(&ob).all(|(a, o)| (a - o).abs() < 0.001), "bone {b}: {tb:?} {ob:?}");
        }
    }
    // Bounds: same rule as Square's (empty bones keep ±16 700 000; bone 2 equal).
    assert_eq!(ours.parts[0].bounds[0], t.parts[0].bounds[0]);
    assert_eq!(ours.parts[1].bounds[1], t.parts[1].bounds[1]);
    let (tb, ob) = (t.parts[0].bounds[2].map(f32::from_bits), ours.parts[0].bounds[2].map(f32::from_bits));
    assert!(tb.iter().zip(&ob).all(|(a, b)| (a - b).abs() < 0.0001), "{tb:?} {ob:?}");
}

/// Share of list-A vertices of part 0 that land on an opaque texel of our own TIM2.
fn opaque_share(m: &Mdl, tims: &[Image]) -> f64 {
    let (mut ok, mut n) = (0, 0);
    for p in &m.parts[0].list_a {
        for v in &p.verts {
            let tex = v.q2.unwrap()[3] as usize;
            let [u, t] = v.uv().unwrap();
            let img = &tims[tex];
            let c = img.at((u * img.w as f32).floor() as isize, (t * img.h as f32).floor() as isize);
            ok += (c[3] >= 0x40) as usize;
            n += 1;
        }
    }
    ok as f64 / n as f64
}

/// Every Keyblade converts: joints 2–11 only, files read back, textures decode, and the UVs
/// land on our own texture's opaque texels.
#[test]
fn d_all_seventeen_convert() {
    let (Some(t), Some(models)) = (template(), models()) else { return };
    let mut names = std::collections::BTreeSet::new();
    let mut n = 0;
    for (card, model) in &models {
        let Some((wpn, assets)) = read_full(&format!("{model}.wpn")) else {
            eprintln!("skipping {model}: not staged");
            continue;
        };
        let c = convert(&t, model, &wpn, &assets).unwrap_or_else(|e| panic!("{model}: {e}"));
        let r = &c.report;
        eprintln!("card {card}: {} | PS2 joints {:?} | packets {:?} vertices {:?} | mixed-bone triangles {}", r.line(), r.ps2_joints, r.packets, r.vertices, r.mixed_bone_triangles);
        assert!(r.joints.iter().all(|j| JOINTS.contains(j)), "{model}: {:?}", r.joints);
        let m = Mdl::parse(&c.mdl).unwrap();
        assert_eq!(m.write().unwrap(), c.mdl);
        // PL0002's skeleton, bone for bone: the DLL's KH1 blade test reads the weapon's bones 2
        // (`wep0`) and 3 (`k9`) through `0x1402ABB20`, and PL0002.ANN drives the same bones.
        for (p, tp) in m.parts.iter().zip(&t.parts) {
            assert_eq!((&p.bones, &p.rest), (&tp.bones, &tp.rest), "{model}: skeleton");
        }
        let bone_name = |i: usize| String::from_utf8_lossy(&m.parts[0].bones[i].name).trim_end_matches('\0').to_string();
        assert_eq!((m.parts.len(), m.parts[0].bones.len(), bone_name(2), bone_name(3)), (2, 12, "wep0".to_string(), "k9".to_string()));
        assert_eq!(m.parts[0].texture_names(), c.textures);
        let rtm = Rtm::parse(&c.rtm).unwrap();
        assert_eq!(rtm.entries.len(), c.textures.len());
        let tims: Vec<Image> = rtm.entries.iter().map(|e| {
            let (w, h, px) = tim2_decode8(&e.data).unwrap();
            Image { w, h, px }
        }).collect();
        // Part 1 holds every triangle again.
        let drawn = |l: &[kh1_recom_assets::mdl::MeshPacket]| l.iter().flat_map(|p| p.verts.iter()).filter(|v| v.draws()).count();
        assert_eq!(drawn(&m.parts[0].list_a) + drawn(m.parts[0].list_b.as_deref().unwrap_or(&[])), r.triangles);
        assert_eq!(drawn(&m.parts[1].list_a), r.triangles);
        let share = opaque_share(&m, &tims);
        eprintln!("  list-A vertices on opaque TIM2 texels: {:.1}%", 100.0 * share);
        assert!(share > 0.95, "{model}: {share}");
        assert!(r.uv_range.0 >= 0.0 && r.uv_range.1 <= 1.0 && r.uv_range.2 >= 0.0 && r.uv_range.3 <= 1.0, "{model}: uv {:?}", r.uv_range);
        for name in &c.textures {
            assert!(names.insert(name.clone()), "texture name {name} twice");
        }
        // The files the step writes, read back as Panacea does (`LoadFile` / `GetRemasteredAsset`
        // on a raw entry: 16-byte header, 0x30 per asset, data, assets, each padded to 16).
        let group = kh1_recom_assets::keyblade::group_for_card(*card).unwrap();
        let files = kh1_recom_assets::keyblade::mod_files(group, &c).unwrap();
        assert_eq!(files[0].0, format!("PLAYER/{group:04}/PL{group:04}.MDL"));
        assert_eq!(files[0].1, c.mdl);
        assert_eq!(files[1].0, format!("raw/PLAYER/{group:04}/PL{group:04}.RTM"));
        let e = &files[1].1;
        let w = |o: usize| i32::from_le_bytes(e[o..o + 4].try_into().unwrap());
        let (len, na) = (w(0) as usize, w(4) as usize);
        assert_eq!((w(8), na), (-2, c.hd.len()));
        let mut off = 16 + 0x30 * na + ((len + 15) & !15);
        assert_eq!(&e[16 + 0x30 * na..16 + 0x30 * na + len], &c.rtm[..]);
        for (k, (name, data)) in c.hd.iter().enumerate() {
            let h = 16 + 0x30 * k;
            let nm = &e[h..h + 0x20];
            assert_eq!(&nm[..name.len()], name.as_bytes());
            assert_eq!((w(h + 0x20) as usize, w(h + 0x24), w(h + 0x28) as usize, w(h + 0x2C)), (off, 0x2F7D2, data.len(), -2));
            assert_eq!(&e[off..off + data.len()], &data[..]);
            off += (data.len() + 15) & !15;
        }
        assert_eq!(off, e.len());
        n += 1;
    }
    assert!(n >= 3, "only {n} Keyblades staged");
}
