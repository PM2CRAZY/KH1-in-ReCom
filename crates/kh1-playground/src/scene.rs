//! What is drawn: the arena, the mannequin Sora (our own capsules between his posed bones), the
//! Keyblade line, the dummies and the hit-shape gizmos. Nothing here is game art.

use std::f32::consts::FRAC_PI_2;

use bevy::math::Isometry3d;
use bevy::prelude::*;
use kh1_core::{DummyState, Shape, ShapeKind};

use crate::{to_bevy, Game, View, SCALE};

/// A capsule between two bones (parent `a`, child `b`); `rest` is its length when spawned.
#[derive(Component)]
pub struct DummyBody(pub usize);

/// Bone roles the mannequin uses beyond the plain bone segments. Assumption (from the
/// hierarchy, the file has no names): bone 0 the hips, 147 the look-at bone (the model info's,
/// the neck), 151 the bone the face and hair bones hang from (the head); 17 / 43 start the legs,
/// 21 / 47 the feet, 101 / 206 the hands.
pub fn spawn_world(mut commands: Commands, game: Res<Game>, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    commands.spawn((Camera3d::default(), Msaa::Sample4, Projection::Perspective(PerspectiveProjection { fov: 0.9, near: 0.05, ..default() }), Transform::from_xyz(0.0, 3.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y)));
    commands.spawn((
        DirectionalLight { illuminance: 8000.0, shadows_enabled: true, ..default() },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, 0.5, 0.0)),
    ));
    // A soft fill light from the other side, without shadows.
    commands.spawn((
        DirectionalLight { illuminance: 2500.0, shadows_enabled: false, color: Color::srgb(0.85, 0.9, 1.0), ..default() },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.6, -2.4, 0.0)),
    ));
    let size = game.sim.arena.half_size * 2.0 * SCALE;
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(size, size))),
        MeshMaterial3d(mats.add(StandardMaterial { base_color: Color::srgb(0.18, 0.2, 0.24), perceptual_roughness: 0.9, ..default() })),
        Transform::IDENTITY,
    ));
    let p = game.sim.arena.platform;
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(p.half_x * 2.0 * SCALE, p.height * SCALE, p.half_z * 2.0 * SCALE))),
        MeshMaterial3d(mats.add(StandardMaterial { base_color: Color::srgb(0.32, 0.3, 0.42), perceptual_roughness: 0.8, ..default() })),
        Transform::from_translation(to_bevy(kh1_core::Vec3::new(p.x, -p.height * 0.5, p.z))),
    ));
}

pub fn spawn_dummies(mut commands: Commands, game: Res<Game>, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    for (i, d) in game.sim.dummies.iter().enumerate() {
        let p = d.parts[0];
        let mesh = meshes.add(Capsule3d::new(p.radius * SCALE, (p.half_height * 2.0 - p.radius * 2.0).max(1.0) * SCALE).mesh().longitudes(48).latitudes(24));
        commands.spawn((Mesh3d(mesh), MeshMaterial3d(mats.add(StandardMaterial { base_color: Color::srgb(0.6, 0.6, 0.62), ..default() })), Transform::IDENTITY, DummyBody(i)));
    }
}

pub fn sync_dummies(game: Res<Game>, mut q: Query<(&DummyBody, &mut Transform, &MeshMaterial3d<StandardMaterial>)>, mut mats: ResMut<Assets<StandardMaterial>>) {
    for (DummyBody(i), mut t, mat) in &mut q {
        let Some(d) = game.sim.dummies.get(*i) else { continue };
        let p = d.parts[0];
        let feet = to_bevy(d.pos);
        let facing = Quat::from_rotation_y(-d.facing);
        let (lean, color) = match d.state {
            _ if d.stats.hp <= 0 => (FRAC_PI_2, Color::srgb(0.2, 0.2, 0.22)),
            DummyState::Idle => (0.0, Color::srgb(0.6, 0.6, 0.62)),
            DummyState::Flinch { .. } => (0.25, Color::srgb(0.95, 0.6, 0.2)),
            DummyState::Air { .. } => (0.6, Color::srgb(0.95, 0.25, 0.2)),
            DummyState::Down { .. } => (FRAC_PI_2, Color::srgb(0.55, 0.3, 0.8)),
        };
        // Leaning back about the feet (our display only; the hit shape stays upright).
        let rot = facing * Quat::from_rotation_x(lean);
        t.rotation = rot;
        t.translation = feet + rot * Vec3::Y * (-p.offset.y * SCALE);
        if let Some(m) = mats.get_mut(&mat.0) {
            m.base_color = color;
        }
    }
}

fn flat_circle(gizmos: &mut Gizmos, c: Vec3, r: f32, color: Color) {
    gizmos.circle(Isometry3d::new(c, Quat::from_rotation_x(FRAC_PI_2)), r, color).resolution(32);
}

fn draw_shape(gizmos: &mut Gizmos, s: &Shape, color: Color) {
    let c = to_bevy(s.centre);
    let (r, h) = (s.radius * SCALE, s.half_height * SCALE);
    match s.kind {
        ShapeKind::Ellipsoid if (s.radius - s.half_height).abs() < 1e-3 => {
            gizmos.sphere(Isometry3d::from_translation(c), r, color).resolution(12);
        }
        ShapeKind::Ellipsoid | ShapeKind::Column => {
            flat_circle(gizmos, c + Vec3::Y * h, r, color);
            flat_circle(gizmos, c - Vec3::Y * h, r, color);
            for k in 0..4 {
                let a = k as f32 * FRAC_PI_2;
                let o = Vec3::new(a.cos(), 0.0, a.sin()) * r;
                gizmos.line(c + o + Vec3::Y * h, c + o - Vec3::Y * h, color);
            }
        }
        ShapeKind::Box => {
            gizmos.cuboid(Transform::from_translation(c).with_scale(Vec3::new(r * 2.0, h * 2.0, r * 2.0)), color);
        }
    }
}

pub fn draw_gizmos(game: Res<Game>, view: Res<View>, mut gizmos: Gizmos) {
    let sim = &game.sim;
    // The floor grid: 1 m (100 units) cells.
    let half = sim.arena.half_size * SCALE;
    let grid = Color::srgba(0.5, 0.55, 0.65, 0.35);
    let mut x = -half;
    while x <= half + 1e-3 {
        gizmos.line(Vec3::new(x, 0.002, -half), Vec3::new(x, 0.002, half), grid);
        gizmos.line(Vec3::new(-half, 0.002, x), Vec3::new(half, 0.002, x), grid);
        x += 1.0;
    }
    // The Keyblade line along its blade points (grip to the tip of the weapon line).
    if let Some((grip, tip)) = sim.blade_line {
        gizmos.line(to_bevy(grip), to_bevy(tip), Color::srgb(1.0, 0.85, 0.3));
    }
    if !view.gizmos {
        return;
    }
    // The live hit spheres (this frame's chain along the blade plus the swept chain).
    for s in &sim.hit_shapes {
        draw_shape(&mut gizmos, s, Color::srgb(1.0, 0.45, 0.1));
    }
    for d in &sim.dummies {
        let col = if Some(d.id) == sim.lock { Color::srgb(1.0, 0.3, 0.3) } else { Color::srgb(0.3, 0.85, 1.0) };
        for s in d.shapes() {
            draw_shape(&mut gizmos, &s, col);
        }
    }
    // The combo ranges around Sora (gap to a part's surface): close 180, lunge 300.
    let feet = to_bevy(sim.ctrl.pos) + Vec3::Y * 0.01;
    let c = &sim.ctrl.params.combo;
    flat_circle(&mut gizmos, feet, c.close_range * SCALE, Color::srgba(0.3, 1.0, 0.4, 0.5));
    flat_circle(&mut gizmos, feet, c.lunge_range * SCALE, Color::srgba(1.0, 1.0, 0.3, 0.4));
    // Facing.
    let fwd = to_bevy(sim.ctrl.pos + kh1_core::Vec3::from_yaw(sim.ctrl.facing) * 60.0) + Vec3::Y * 0.01;
    gizmos.arrow(feet, fwd, Color::srgb(0.9, 0.9, 0.9));
    if sim.ctrl.guard_live() {
        gizmos.sphere(Isometry3d::from_translation(to_bevy(sim.bone_world(0))), 0.6, Color::srgb(0.3, 0.6, 1.0));
    }
}
