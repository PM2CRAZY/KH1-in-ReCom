//! A smooth stand-in figure for Sora, built on his KH1 skeleton (no game meshes).
//!
//! Every part follows bones of KH1's Sora (`xa_ex_0010.mdls`, 293 bones). The bone indices were
//! identified by matching KH1's skeleton to Re:CoM's named one (`docs/motion-conversion.md`):
//! e.g. 17 / 18 / 20 right femur / knee / ankle, 93 / 94 / 97 / 100 right shoulder / elbow /
//! wrist / hand, 151 the head centre, 152 + 3k the 14 hair spikes. Limbs are tapered capsules
//! (two spheres joined by a cone, smooth normals) between bone positions; the pelvis, chest, head
//! and shoes are ellipsoids oriented by the bones around them.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};

use crate::{to_bevy, Game, SCALE};

/// KH1 bone indices of Sora's body parts.
mod bones {
    pub const HIP: usize = 2;
    pub const WAIST: usize = 69;
    pub const CHEST2: usize = 70;
    pub const CHEST1: usize = 77;
    pub const NECK3: usize = 145;
    pub const HEAD: usize = 151;
    /// The base of the skull (where the neck ends).
    pub const HEAD_BASE: usize = 149;
    /// femur, knee, ankle, toes
    pub const LEG_R: [usize; 4] = [17, 18, 20, 23];
    pub const LEG_L: [usize; 4] = [43, 44, 46, 49];
    /// collar, shoulder, elbow, wrist, hand
    pub const ARM_R: [usize; 5] = [90, 93, 94, 97, 100];
    pub const ARM_L: [usize; 5] = [195, 198, 199, 202, 205];
    pub const FINGERS: [usize; 10] = [103, 108, 113, 118, 123, 207, 212, 217, 222, 227];
    pub const HAIR_FIRST: usize = 152;
    pub const HAIR_COUNT: usize = 14;
    pub const JACKET: [usize; 9] = [72, 248, 252, 256, 260, 264, 268, 272, 276];
    pub const HOOD: usize = 79;
    pub const CUFFS: [usize; 8] = [26, 30, 34, 38, 52, 56, 60, 64];
    pub const SLEEVES: [usize; 8] = [128, 132, 136, 140, 232, 236, 240, 244];
    pub const HAIR_L1: usize = 185;
    pub const HAIR_R1: usize = 170;
}

/// Where a bone is, in Bevy space. KH1 doesn't pose the end bones of chains (hair tips, jacket
/// tails…: flags 0x6000000), so those are placed as KH1's data defines them: the parent's world
/// matrix applied to the end bone's rest translation.
pub fn bone_point(sim: &crate::sim::Sim, b: usize) -> Option<Vec3> {
    let rig = &sim.rig;
    if b >= rig.bone_count() {
        return None;
    }
    if rig.is_posed(b) {
        return Some(to_bevy(sim.bone_world(b)));
    }
    let parent = rig.parent(b)?;
    if !rig.is_posed(parent) {
        return None;
    }
    let (_, _, t, _) = rig.locals(b)?;
    let p = rig.current().world[parent].point(t);
    Some(to_bevy(sim.to_world(p)))
}

/// A tapered capsule between two bones.
#[derive(Component)]
pub struct Limb {
    a: usize,
    b: usize,
    rest: f32,
}

/// An ellipsoid placed by a frame built from several bones.
#[derive(Component)]
pub struct Blob(BlobKind);

#[derive(Clone, Copy)]
enum BlobKind {
    Pelvis,
    Chest,
    Head,
    Shoe(usize),
}

#[derive(Component)]
pub struct BladeMesh;

/// A capsule of radius `r0` at its bottom end and `r1` at its top, `len` between the two sphere
/// centres, along +Y and centred on the origin, with smooth normals.
pub fn rounded_cone(r0: f32, r1: f32, len: f32, sides: usize, rings: usize) -> Mesh {
    let h = len * 0.5;
    // The cone touches both spheres at the latitude where sin = (r0 - r1) / len.
    let phi = if len > 1e-6 { ((r0 - r1) / len).clamp(-0.999, 0.999).asin() } else { 0.0 };
    let mut profile: Vec<(f32, f32, f32, f32)> = Vec::new(); // (radius, y, normal x, normal y)
    let half_pi = std::f32::consts::FRAC_PI_2;
    for i in 0..=rings {
        let th = -half_pi + (phi + half_pi) * i as f32 / rings as f32;
        profile.push((r0 * th.cos(), -h + r0 * th.sin(), th.cos(), th.sin()));
    }
    for i in 0..=rings {
        let th = phi + (half_pi - phi) * i as f32 / rings as f32;
        profile.push((r1 * th.cos(), h + r1 * th.sin(), th.cos(), th.sin()));
    }
    let mut pos = Vec::new();
    let mut nor = Vec::new();
    let mut uv = Vec::new();
    for (k, &(r, y, nx, ny)) in profile.iter().enumerate() {
        for s in 0..=sides {
            let a = s as f32 / sides as f32 * std::f32::consts::TAU;
            let (sa, ca) = a.sin_cos();
            pos.push([r * ca, y, r * sa]);
            nor.push([nx * ca, ny, nx * sa]);
            uv.push([s as f32 / sides as f32, k as f32 / (profile.len() - 1) as f32]);
        }
    }
    let row = sides as u32 + 1;
    let mut idx = Vec::new();
    for k in 0..profile.len() as u32 - 1 {
        for s in 0..sides as u32 {
            let (a, b) = (k * row + s, (k + 1) * row + s);
            idx.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nor)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_indices(Indices::U32(idx))
}

fn children(rig: &kh1_anim::Rig, b: usize) -> Vec<usize> {
    (0..rig.bone_count()).filter(|&c| rig.parent(c) == Some(b)).collect()
}

/// `root` and its first-child descendants down to the end of the chain.
fn chain(rig: &kh1_anim::Rig, root: usize) -> Vec<usize> {
    let mut out = vec![root];
    let mut b = root;
    while let Some(&c) = children(rig, b).first() {
        out.push(c);
        b = c;
        if out.len() > 8 {
            break;
        }
    }
    out
}

pub fn spawn_sora(mut commands: Commands, game: Res<Game>, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    let sim = &game.sim;
    let rig = &sim.rig;
    let n = rig.bone_count();
    let mut mat = |c: Color, rough: f32| mats.add(StandardMaterial { base_color: c, perceptual_roughness: rough, reflectance: 0.35, ..default() });
    let suit = mat(Color::srgb(0.78, 0.13, 0.12), 0.55);
    let jacket = mat(Color::srgb(0.12, 0.13, 0.18), 0.45);
    let shoe = mat(Color::srgb(0.98, 0.78, 0.18), 0.35);
    let skin = mat(Color::srgb(0.95, 0.82, 0.70), 0.6);
    let hair = mat(Color::srgb(0.42, 0.26, 0.14), 0.5);
    let glove = mat(Color::srgb(0.14, 0.14, 0.16), 0.5);
    let pos = |b: usize| bone_point(sim, b).unwrap_or(Vec3::ZERO) / SCALE;
    let ok = |b: usize| bone_point(sim, b).is_some();
    let _ = n;

    let limb = |commands: &mut Commands, meshes: &mut Assets<Mesh>, a: usize, b: usize, r0: f32, r1: f32, m: &Handle<StandardMaterial>| {
        if !ok(a) || !ok(b) {
            return;
        }
        let len = (pos(b) - pos(a)).length();
        if len < 0.5 {
            return;
        }
        let mesh = meshes.add(rounded_cone(r0 * SCALE, r1 * SCALE, len * SCALE, 24, 6));
        commands.spawn((Mesh3d(mesh), MeshMaterial3d(m.clone()), Transform::IDENTITY, Limb { a, b, rest: len }));
    };
    // A chain of bones drawn as one tapering tube.
    let tube = |commands: &mut Commands, meshes: &mut Assets<Mesh>, bones: &[usize], r_root: f32, r_tip: f32, m: &Handle<StandardMaterial>| {
        // Bones that sit on top of the previous one (KH1 often stacks a rotation bone on a
        // position bone) add nothing: keep the first of each stack.
        let mut kept: Vec<usize> = Vec::new();
        for &b in bones {
            if !ok(b) {
                break;
            }
            if kept.last().is_none_or(|&l| (pos(b) - pos(l)).length() > 0.5) {
                kept.push(b);
            }
        }
        let bones = &kept[..];
        let k = bones.len().saturating_sub(1).max(1) as f32;
        for (i, w) in bones.windows(2).enumerate() {
            let r0 = r_root + (r_tip - r_root) * i as f32 / k;
            let r1 = r_root + (r_tip - r_root) * (i + 1) as f32 / k;
            limb(commands, meshes, w[0], w[1], r0, r1, m);
        }
    };

    use bones::*;
    // Legs: thigh, shin (baggy red trousers).
    for leg in [LEG_R, LEG_L] {
        limb(&mut commands, &mut meshes, leg[0], leg[1], 7.4, 5.6, &suit);
        limb(&mut commands, &mut meshes, leg[1], leg[2], 5.6, 4.6, &suit);
    }
    // Spine and neck.
    limb(&mut commands, &mut meshes, WAIST, CHEST2, 9.0, 10.0, &suit);
    limb(&mut commands, &mut meshes, CHEST2, CHEST1, 10.0, 10.5, &jacket);
    limb(&mut commands, &mut meshes, NECK3, HEAD_BASE, 3.6, 3.2, &skin);
    // Arms: shoulder pad, upper arm (jacket sleeve), forearm, wrist, hand.
    for arm in [ARM_R, ARM_L] {
        limb(&mut commands, &mut meshes, arm[0], arm[1], 3.8, 4.4, &jacket);
        limb(&mut commands, &mut meshes, arm[1], arm[2], 3.9, 3.2, &jacket);
        limb(&mut commands, &mut meshes, arm[2], arm[3], 3.1, 2.6, &skin);
        limb(&mut commands, &mut meshes, arm[3], arm[4], 2.9, 3.2, &glove);
    }
    for f in FINGERS {
        if ok(f) {
            tube(&mut commands, &mut meshes, &chain(rig, f), 1.05, 0.8, &glove);
        }
    }
    // Hair: 14 spikes from their roots to their tips.
    for k in 0..HAIR_COUNT {
        let root = HAIR_FIRST + 3 * k;
        if ok(root) {
            tube(&mut commands, &mut meshes, &chain(rig, root), 4.2, 0.4, &hair);
        }
    }
    // Jacket tails, hood, trouser cuffs, sleeve cuffs.
    for j in JACKET {
        tube(&mut commands, &mut meshes, &chain(rig, j), 2.4, 1.4, &jacket);
    }
    tube(&mut commands, &mut meshes, &chain(rig, HOOD), 3.6, 2.0, &jacket);
    // (KH1's trouser and sleeve panels, `CUFFS` / `SLEEVES`, are left out: as tubes they read as
    // spokes. The thighs and upper arms are drawn wider instead.)
    let _ = (CUFFS, SLEEVES);
    // Ellipsoids.
    let ball = meshes.add(Sphere::new(1.0).mesh().uv(48, 32));
    for (kind, m) in [(BlobKind::Pelvis, &suit), (BlobKind::Chest, &jacket), (BlobKind::Head, &skin), (BlobKind::Shoe(0), &shoe), (BlobKind::Shoe(1), &shoe)] {
        commands.spawn((Mesh3d(ball.clone()), MeshMaterial3d(m.clone()), Transform::IDENTITY, Blob(kind)));
    }
    // Round joints so the tapered parts meet smoothly.
    for (b, r, m) in [
        (LEG_R[1], 5.6, &suit),
        (LEG_L[1], 5.6, &suit),
        (ARM_R[1], 4.4, &jacket),
        (ARM_L[1], 4.4, &jacket),
        (ARM_R[2], 3.3, &jacket),
        (ARM_L[2], 3.3, &jacket),
        (ARM_R[4], 3.3, &glove),
        (ARM_L[4], 3.3, &glove),
        (CHEST2, 10.0, &suit),
    ] {
        if ok(b) {
            let mesh = meshes.add(Sphere::new(r * SCALE).mesh().uv(32, 20));
            commands.spawn((Mesh3d(mesh), MeshMaterial3d(m.clone()), Transform::IDENTITY, Limb { a: b, b, rest: 0.0 }));
        }
    }
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(1.0, 1.0).mesh().resolution(16))),
        MeshMaterial3d(mats.add(StandardMaterial { base_color: Color::srgb(0.85, 0.87, 0.92), metallic: 0.85, perceptual_roughness: 0.25, ..default() })),
        Transform::IDENTITY,
        BladeMesh,
    ));
}

/// A rotation whose +Y is `up` and +X is as close to `side` as possible.
fn frame(side: Vec3, up: Vec3) -> Quat {
    let y = up.normalize_or(Vec3::Y);
    let mut x = side - y * side.dot(y);
    if x.length_squared() < 1e-8 {
        x = y.any_orthonormal_vector();
    }
    let x = x.normalize();
    let z = x.cross(y);
    Quat::from_mat3(&Mat3::from_cols(x, y, z))
}

fn place_between(t: &mut Transform, a: Vec3, b: Vec3, rest: f32) {
    let d = b - a;
    let len = d.length();
    t.translation = (a + b) * 0.5;
    t.rotation = Quat::from_rotation_arc(Vec3::Y, d.normalize_or(Vec3::Y));
    // Bones keep their length unless KH1 scales them; only then is the part stretched.
    let s = if rest > 0.0 { len / (rest * SCALE) } else { 1.0 };
    t.scale = Vec3::new(1.0, if (s - 1.0).abs() < 0.02 { 1.0 } else { s }, 1.0);
}

#[allow(clippy::type_complexity)]
pub fn sync_sora(
    game: Res<Game>,
    mut limbs: Query<(&Limb, &mut Transform, &mut Visibility), (Without<Blob>, Without<BladeMesh>)>,
    mut blobs: Query<(&Blob, &mut Transform, &mut Visibility), (Without<Limb>, Without<BladeMesh>)>,
    mut blade: Query<&mut Transform, (With<BladeMesh>, Without<Limb>, Without<Blob>)>,
) {
    let sim = &game.sim;
    let rig = &sim.rig;
    let p = |b: usize| bone_point(sim, b).unwrap_or(Vec3::ZERO);
    let shown = |b: usize| bone_point(sim, b).is_some();
    let _ = rig;
    for (l, mut t, mut vis) in &mut limbs {
        let on = shown(l.a) && shown(l.b);
        *vis = if on { Visibility::Inherited } else { Visibility::Hidden };
        if !on {
            continue;
        }
        if l.a == l.b {
            t.translation = p(l.a);
        } else {
            place_between(&mut t, p(l.a), p(l.b), l.rest);
        }
    }
    use bones::*;
    for (Blob(kind), mut t, mut vis) in &mut blobs {
        let need: &[usize] = match kind {
            BlobKind::Pelvis => &[HIP, LEG_R[0], LEG_L[0], CHEST2],
            BlobKind::Chest => &[CHEST2, NECK3, ARM_R[1], ARM_L[1]],
            BlobKind::Head => &[HEAD, HEAD_BASE, HAIR_L1, HAIR_R1],
            BlobKind::Shoe(i) => {
                let leg = if *i == 0 { &LEG_R } else { &LEG_L };
                &leg[1..4]
            }
        };
        let on = need.iter().all(|&b| shown(b));
        *vis = if on { Visibility::Inherited } else { Visibility::Hidden };
        if !on {
            continue;
        }
        let (centre, rot, radii) = match kind {
            BlobKind::Pelvis => {
                let side = p(LEG_L[0]) - p(LEG_R[0]);
                let up = p(CHEST2) - p(HIP);
                let c = (p(LEG_L[0]) + p(LEG_R[0])) * 0.5 * 0.6 + p(HIP) * 0.4;
                (c, frame(side, up), Vec3::new(12.5, 9.5, 9.0))
            }
            BlobKind::Chest => {
                let side = p(ARM_L[1]) - p(ARM_R[1]);
                let up = p(NECK3) - p(CHEST2);
                let c = p(CHEST2).lerp(p(NECK3), 0.45);
                (c, frame(side, up), Vec3::new(13.0, 12.0, 8.5))
            }
            BlobKind::Head => {
                let side = p(HAIR_L1) - p(HAIR_R1);
                let up = p(HEAD) - p(HEAD_BASE);
                // The skull sits between its base (149) and the hair centre (151).
                (p(HEAD_BASE).lerp(p(HEAD), 0.5), frame(side, up), Vec3::new(10.5, 11.0, 11.0))
            }
            BlobKind::Shoe(i) => {
                let leg = if *i == 0 { &LEG_R } else { &LEG_L };
                let (knee, ankle, toe) = (p(leg[1]), p(leg[2]), p(leg[3]));
                let fwd = toe - ankle;
                let len = fwd.length() / SCALE;
                let r = frame(fwd, knee - ankle);
                // The shoe's long axis (+X of the frame) runs from the heel past the toes.
                (ankle.lerp(toe, 0.55) - (knee - ankle).normalize_or(Vec3::Y) * 2.5 * SCALE, r, Vec3::new(len * 0.75 + 4.0, 4.6, 5.8))
            }
        };
        t.translation = centre;
        t.rotation = rot;
        t.scale = radii * SCALE;
    }
    if let (Ok(mut t), Some((grip, tip))) = (blade.get_single_mut(), sim.blade_line) {
        let (a, b) = (to_bevy(grip), to_bevy(tip));
        let len = (b - a).length();
        t.translation = (a + b) * 0.5;
        t.rotation = Quat::from_rotation_arc(Vec3::Y, (b - a).normalize_or(Vec3::Y));
        t.scale = Vec3::new(1.8 * SCALE, len.max(1e-4), 1.8 * SCALE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::render::mesh::VertexAttributeValues;

    #[test]
    fn rounded_cone_is_closed_and_smooth() {
        let m = rounded_cone(2.0, 1.0, 5.0, 16, 4);
        let Some(VertexAttributeValues::Float32x3(p)) = m.attribute(Mesh::ATTRIBUTE_POSITION) else { panic!() };
        let Some(VertexAttributeValues::Float32x3(n)) = m.attribute(Mesh::ATTRIBUTE_NORMAL) else { panic!() };
        let ys: Vec<f32> = p.iter().map(|v| v[1]).collect();
        let (lo, hi) = ys.iter().fold((f32::MAX, f32::MIN), |(a, b), &y| (a.min(y), b.max(y)));
        assert!((lo + 2.5 + 2.0).abs() < 1e-4 && (hi - 2.5 - 1.0).abs() < 1e-4);
        for v in n {
            let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            assert!((l - 1.0).abs() < 1e-4);
        }
    }

    #[test]
    fn frame_keeps_up_and_side() {
        let q = frame(Vec3::X * 3.0 + Vec3::Y, Vec3::Y * 2.0);
        assert!((q * Vec3::Y - Vec3::Y).length() < 1e-5);
        assert!((q * Vec3::X - Vec3::X).length() < 1e-5);
    }
}
