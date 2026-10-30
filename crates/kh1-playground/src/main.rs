//! KH1 FM battle-system playground (`docs/kh1-rewrite.md`): movement and the Attack combo.
//!
//! A mannequin Sora (our own capsules on his real skeleton, posed by his real motions) runs on
//! kh1-core's 60 Hz controller, swings at three training dummies and one on a raised platform,
//! with KH1's damage and reactions. Every number and motion comes from the player's own install
//! through kh1-data; no game meshes, textures or sounds are loaded.
//!
//! Run: `kh1-playground --game-dir "D:\Steam\steamapps\common\KINGDOM HEARTS -HD 1.5+2.5 ReMIX-"`
//! (or set `KH1_GAME_DIR`). F1 shows the controls. `--trace out.csv` logs every frame.

mod arena;
mod hud;
mod scene;
mod script;
mod sim;
mod trace;

use std::path::PathBuf;
use std::sync::Arc;

use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::window::WindowResolution;
use kh1_data::{GameDir, SoraKit};

use script::Script;
use sim::{Key, Options, Pad, Sim};
use trace::Trace;

/// KH1 units (about 1 cm) to Bevy metres.
pub const SCALE: f32 = 0.01;

/// A kh1-core world point (y down) in Bevy (y up, metres): a half turn about x, so KH1's +z
/// (yaw 0 forward) is Bevy's −z (its forward) and +x stays right.
pub fn to_bevy(p: kh1_core::Vec3) -> Vec3 {
    Vec3::new(p.x, -p.y, -p.z) * SCALE
}

fn arg(args: &[String], name: &str) -> Option<String> {
    let eq = format!("{name}=");
    args.iter().enumerate().find_map(|(i, a)| if a == name { args.get(i + 1).cloned() } else { a.strip_prefix(&eq).map(str::to_string) })
}

fn usage() -> String {
    format!(
        "kh1-playground [--game-dir <KH 1.5+2.5 folder>] [--trace out.csv] [--script {}] [--frames N]\n\
         \x20              [--screenshot out.png [--screenshot-frame N]] [--strength N] [--max-mp N]\n\
         \x20              [--dummy-hp N] [--dummy-defence N] [--no-push]\n\
         The game folder holds Image/dt/kh1_first.hed and KINGDOM HEARTS FINAL MIX.exe; it is also\n\
         looked for in KH1_GAME_DIR, next to this program and in common Steam / Epic folders.",
        Script::NAMES.replace(", ", "|")
    )
}

/// Everything the app runs on.
#[derive(Resource)]
pub struct Game {
    pub sim: Sim,
    pub source: String,
    trace: Option<Trace>,
    script: Option<Script>,
    exit_after: Option<u64>,
    shot: Option<(PathBuf, u64)>,
    shot_taken: Option<u32>,
}

/// Input latched between simulation frames.
#[derive(Resource, Default)]
pub struct Latch {
    /// jump, attack, square.
    held: [bool; 3],
    pressed: [bool; 3],
    lock: bool,
    stick: [f32; 2],
}

/// The orbit camera (our own, not KH1's camera).
#[derive(Resource)]
pub struct OrbitCam {
    /// Bevy yaw of the camera around Sora (0 = behind him at the start) and pitch, radians.
    pub yaw: f32,
    pub pitch: f32,
    /// Metres.
    pub dist: f32,
}

/// Display switches.
#[derive(Resource)]
pub struct View {
    pub gizmos: bool,
    pub help: bool,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{}", usage());
        return;
    }
    let num = |n: &str| arg(&args, n).and_then(|v| v.parse::<i32>().ok());
    let mut opts = Options::default();
    if let Some(v) = num("--strength") {
        opts.attacker.strength = v;
    }
    if let Some(v) = num("--max-mp") {
        opts.attacker.max_mp = v;
    }
    if let Some(v) = num("--dummy-hp") {
        opts.dummy.hp = v;
        opts.dummy.max_hp = v;
    }
    if let Some(v) = num("--dummy-defence") {
        opts.dummy.defence = v;
    }
    opts.push = !args.iter().any(|a| a == "--no-push");
    let script = match arg(&args, "--script") {
        Some(s) => match Script::new(&s) {
            Some(sc) => Some(sc),
            None => {
                eprintln!("unknown --script {s} (one of: {})", Script::NAMES);
                std::process::exit(2);
            }
        },
        None => None,
    };
    let exit_after = arg(&args, "--frames").and_then(|v| v.parse().ok());
    let shot = arg(&args, "--screenshot").map(|p| (PathBuf::from(p), arg(&args, "--screenshot-frame").and_then(|v| v.parse().ok()).unwrap_or(120)));

    let loaded = GameDir::find_from(arg(&args, kh1_data::GAME_DIR_ARG).map(PathBuf::from))
        .map_err(|e| e.to_string())
        .and_then(|dir| SoraKit::load(&dir).map(|k| (dir, k)).map_err(|e| format!("Found the game folder but could not read Sora's files:\n  {e}")));

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window { title: "KH1 FM movement & Attack (kh1-playground)".into(), resolution: WindowResolution::new(1280.0, 720.0), ..default() }),
        ..default()
    }))
    .insert_resource(ClearColor(Color::srgb(0.07, 0.08, 0.11)));

    match loaded {
        Ok((dir, kit)) => {
            let trace = arg(&args, "--trace").map(|p| match Trace::create(std::path::Path::new(&p)) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("cannot write the trace {p}: {e}");
                    std::process::exit(2);
                }
            });
            let sim = Sim::new(Arc::new(kit), opts);
            app.insert_resource(Game { sim, source: dir.root.display().to_string(), trace, script, exit_after, shot, shot_taken: None })
                .insert_resource(Time::<Fixed>::from_hz(60.0))
                .insert_resource(AmbientLight { color: Color::WHITE, brightness: 350.0 })
                .insert_resource(Latch::default())
                .insert_resource(OrbitCam { yaw: -0.5, pitch: 0.3, dist: 4.5 })
                .insert_resource(View { gizmos: true, help: false })
                .add_systems(Startup, (scene::spawn_world, scene::spawn_sora, scene::spawn_dummies, hud::spawn))
                .add_systems(FixedUpdate, simulate)
                .add_systems(
                    Update,
                    (latch_input, keys, camera_follow, scene::sync_sora, scene::sync_dummies, scene::draw_gizmos, hud::update, hud::labels, screenshot_and_exit).chain(),
                );
        }
        Err(msg) => {
            let msg = format!("{msg}\n\n{}", usage());
            eprintln!("{msg}");
            app.insert_resource(hud::ErrorText(msg)).add_systems(Startup, hud::show_error);
        }
    }
    app.run();
}

/// KH1's stick read (`0x14028B4F0`, dead zone 60 of 127.5) from a −1..1 stick, as bytes.
fn kh1_stick(v: Vec2) -> [f32; 2] {
    let byte = |x: f32| (127.5 + x.clamp(-1.0, 1.0) * 127.5).round().clamp(0.0, 255.0) as u8;
    let (s, _) = kh1_core::stick_from_raw(byte(v.x), byte(-v.y), 60.0);
    [s[0], -s[1]]
}

fn latch_input(keys: Res<ButtonInput<KeyCode>>, pads: Query<&Gamepad>, mut latch: ResMut<Latch>) {
    let mut stick = Vec2::ZERO;
    for (k, d) in [(KeyCode::KeyW, Vec2::Y), (KeyCode::KeyS, -Vec2::Y), (KeyCode::KeyA, -Vec2::X), (KeyCode::KeyD, Vec2::X)] {
        if keys.pressed(k) {
            stick += d;
        }
    }
    // Keyboard: full tilt runs; Shift holds the tilt at 0.5 (KH1 walks at or below 0.8).
    let mut s = stick.normalize_or_zero();
    if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        s *= 0.5;
    }
    let mut out = [s.x, s.y];
    let mut held = [
        keys.pressed(KeyCode::Space),
        keys.pressed(KeyCode::KeyJ) || keys.pressed(KeyCode::Enter),
        keys.pressed(KeyCode::KeyK),
    ];
    let mut lock = keys.just_pressed(KeyCode::KeyL);
    for pad in &pads {
        let l = pad.left_stick();
        if l.length() > 0.05 {
            out = kh1_stick(l);
        }
        // KH1's default (US) layout: Cross = Attack, Circle = Jump, Square = roll / guard.
        // Xbox: A / South = Attack, B / East = Jump, X / West = Square. R1 lock-on.
        held[0] |= pad.pressed(GamepadButton::East);
        held[1] |= pad.pressed(GamepadButton::South);
        held[2] |= pad.pressed(GamepadButton::West);
        lock |= pad.just_pressed(GamepadButton::RightTrigger);
    }
    for (i, h) in held.into_iter().enumerate() {
        if h && !latch.held[i] {
            latch.pressed[i] = true;
        }
        latch.held[i] = h;
    }
    latch.lock |= lock;
    latch.stick = out;
}

fn simulate(mut game: ResMut<Game>, mut latch: ResMut<Latch>, cam: Res<OrbitCam>, mut exit: EventWriter<AppExit>) {
    let g = &mut *game;
    if g.exit_after.is_some_and(|n| g.sim.frame >= n) {
        return;
    }
    let pad = match &mut g.script {
        Some(s) => s.pad(&g.sim),
        None => {
            let k = |i: usize| Key { held: latch.held[i], pressed: latch.pressed[i] };
            let p = Pad { stick: latch.stick, camera_yaw: -cam.yaw, jump: k(0), attack: k(1), square: k(2), lock_toggle: latch.lock };
            latch.pressed = [false; 3];
            latch.lock = false;
            p
        }
    };
    let out = g.sim.step(&pad).clone();
    if let Some(t) = &mut g.trace {
        if let Err(e) = t.frame(&g.sim, &out) {
            eprintln!("trace write failed: {e}");
            g.trace = None;
        }
    }
    if g.sim.frame.is_multiple_of(60) {
        if let Some(t) = &mut g.trace {
            let _ = t.flush();
        }
    }
    if g.exit_after.is_some_and(|n| g.sim.frame >= n) {
        if let Some(t) = &mut g.trace {
            let _ = t.flush();
        }
        if g.shot.is_none() {
            exit.send(AppExit::Success);
        }
    }
}

fn keys(keys: Res<ButtonInput<KeyCode>>, mut game: ResMut<Game>, mut view: ResMut<View>) {
    if keys.just_pressed(KeyCode::Tab) {
        view.gizmos = !view.gizmos;
    }
    if keys.just_pressed(KeyCode::F1) {
        view.help = !view.help;
    }
    if keys.just_pressed(KeyCode::KeyR) {
        game.sim.reset();
    }
    let a = &mut game.sim.ctrl.abilities;
    let digits = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6, KeyCode::Digit7, KeyCode::Digit8];
    for (i, k) in digits.into_iter().enumerate() {
        if !keys.just_pressed(k) {
            continue;
        }
        match i {
            0 => a.guard = !a.guard,
            1 => a.dodge_roll = !a.dodge_roll,
            2 => a.high_jump = !a.high_jump,
            3 => a.glide = !a.glide,
            4 => a.superglide = !a.superglide,
            5 => a.combo_plus = (a.combo_plus + 1) % 4,
            6 => a.air_combo_plus = (a.air_combo_plus + 1) % 4,
            _ => a.combo_master = !a.combo_master,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn camera_follow(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut motion: EventReader<MouseMotion>,
    mut wheel: EventReader<MouseWheel>,
    pads: Query<&Gamepad>,
    time: Res<Time>,
    game: Res<Game>,
    mut cam: ResMut<OrbitCam>,
    mut q: Query<&mut Transform, With<Camera3d>>,
) {
    let dt = time.delta_secs();
    let mut turn = Vec2::ZERO;
    for (k, d) in [(KeyCode::ArrowLeft, -Vec2::X), (KeyCode::ArrowRight, Vec2::X), (KeyCode::ArrowUp, Vec2::Y), (KeyCode::ArrowDown, -Vec2::Y)] {
        if keys.pressed(k) {
            turn += d * 2.0 * dt;
        }
    }
    for pad in &pads {
        let r = pad.right_stick();
        if r.length() > 0.2 {
            turn += r * 2.5 * dt;
        }
    }
    let drag = mouse.pressed(MouseButton::Right) || mouse.pressed(MouseButton::Left);
    for m in motion.read() {
        if drag {
            turn += Vec2::new(m.delta.x, -m.delta.y) * 0.005;
        }
    }
    for w in wheel.read() {
        cam.dist = (cam.dist * (1.0 - w.y * 0.1)).clamp(1.5, 25.0);
    }
    cam.yaw -= turn.x;
    cam.pitch = (cam.pitch - turn.y).clamp(-0.2, 1.4);
    let Ok(mut t) = q.get_single_mut() else { return };
    let target = to_bevy(game.sim.ctrl.pos) + Vec3::Y * 0.8;
    let off = Vec3::new(cam.yaw.sin() * cam.pitch.cos(), cam.pitch.sin(), cam.yaw.cos() * cam.pitch.cos()) * cam.dist;
    *t = Transform::from_translation(target + off).looking_at(target, Vec3::Y);
}

fn screenshot_and_exit(mut commands: Commands, mut game: ResMut<Game>, mut exit: EventWriter<AppExit>) {
    let g = &mut *game;
    let Some((path, at)) = g.shot.clone() else { return };
    match g.shot_taken {
        None if g.sim.frame >= at => {
            use bevy::render::view::screenshot::{save_to_disk, Screenshot};
            commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
            g.shot_taken = Some(0);
        }
        Some(n) => {
            g.shot_taken = Some(n + 1);
            // Give the capture a few frames to reach the disk.
            if n >= 30 && g.exit_after.is_none_or(|e| g.sim.frame >= e) {
                if let Some(t) = &mut g.trace {
                    let _ = t.flush();
                }
                exit.send(AppExit::Success);
            }
        }
        None => {}
    }
}
