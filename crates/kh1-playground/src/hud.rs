//! HUD text: the controller's live state, the dummies, MP, abilities, help, and the error screen.

use bevy::prelude::*;
use kh1_core::controller::state;
use kh1_core::DummyState;

use crate::{to_bevy, Game, View};

#[derive(Component)]
pub struct HudText;

#[derive(Component)]
pub struct HelpText;

#[derive(Component)]
pub struct Label(usize);

#[derive(Resource)]
pub struct ErrorText(pub String);

const HELP: &str = "Controls (F1 hides)\n\
Keyboard\n\
  WASD      move (full tilt = run)\n\
  Shift     walk (tilt 0.5)\n\
  Space     jump (hold for full height; again in the air = Glide)\n\
  J / Enter Attack\n\
  K         Square: Dodge Roll / Guard (Superglide in the air)\n\
  L         lock on to the nearest dummy / let go\n\
  Arrows, right/left mouse drag, wheel   camera\n\
  1-8       abilities (see the list)\n\
  Tab       hit gizmos      R  reset      F1  help\n\
Gamepad\n\
  left stick move (KH1's dead zone), right stick camera\n\
  A / Cross Attack, B / Circle jump, X / Square roll-guard, R1 lock on\n\
\n\
With Guard and Dodge Roll both on, the roll needs the stick past 0.8 (KH1).\n\
Rings: green = close range 180, yellow = lunge range 300 (gap to the target's surface).";

pub fn spawn(mut commands: Commands, game: Res<Game>) {
    commands.spawn((
        Text::new(""),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgb(0.92, 0.94, 0.98)),
        Node { position_type: PositionType::Absolute, top: Val::Px(10.0), left: Val::Px(12.0), ..default() },
        HudText,
    ));
    commands.spawn((
        Text::new(HELP),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgb(0.95, 0.9, 0.7)),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
        Node { position_type: PositionType::Absolute, top: Val::Px(10.0), right: Val::Px(12.0), padding: UiRect::all(Val::Px(8.0)), ..default() },
        Visibility::Hidden,
        HelpText,
    ));
    commands.spawn((
        Text::new("Camera: a simple orbit follow camera, not KH1's camera yet.  F1: controls"),
        TextFont { font_size: 13.0, ..default() },
        TextColor(Color::srgb(0.7, 0.75, 0.85)),
        Node { position_type: PositionType::Absolute, bottom: Val::Px(8.0), left: Val::Px(12.0), ..default() },
    ));
    for i in 0..game.sim.dummies.len() {
        commands.spawn((
            Text::new(""),
            TextFont { font_size: 13.0, ..default() },
            TextColor(Color::WHITE),
            Node { position_type: PositionType::Absolute, ..default() },
            Label(i),
        ));
    }
}

fn state_name(s: u32) -> &'static str {
    match s {
        state::GROUND => "ground",
        state::TAKE_OFF => "take-off",
        state::AIR => "air",
        state::GLIDE => "glide",
        _ => "?",
    }
}

fn on(b: bool) -> &'static str {
    if b {
        "ON "
    } else {
        "off"
    }
}

pub fn update(game: Res<Game>, view: Res<View>, mut hud: Query<&mut Text, (With<HudText>, Without<HelpText>)>, mut help: Query<&mut Visibility, With<HelpText>>) {
    if let Ok(mut v) = help.get_single_mut() {
        *v = if view.help { Visibility::Inherited } else { Visibility::Hidden };
    }
    let Ok(mut text) = hud.get_single_mut() else { return };
    let sim = &game.sim;
    let c = &sim.ctrl;
    let mut s = String::new();
    use std::fmt::Write;
    let _ = writeln!(s, "KH1 FM Sora (kh1-playground)   data: {}", game.source);
    let _ = writeln!(s, "frame {}   60 Hz, step 1.0{}", sim.frame, if sim.opts.push { "" } else { "   (no dummy push)" });
    let _ = writeln!(s, "state {:#x} {}   grounded {}   height {:.1}", c.state, state_name(c.state), c.grounded, c.height_above_floor);
    let len = c.motion.info(&c.moveset.motions).map_or(0.0, |m| m.length);
    let blend = if c.motion.blend > 0.0 { format!("   blend {:.0}/{:.0}", c.motion.blend_clock, c.motion.blend) } else { String::new() };
    let _ = writeln!(s, "motion request {:#x} ({}) entry {:?}   frame {:.1} / {:.0}{}", c.motion.request, c.motion.request, c.motion.entry, c.motion.frame, len, blend);
    let f = c.motion.flags;
    let _ = writeln!(
        s,
        "windows  hit {}  combo {}  free {}  trail {}   action lock {}   no-flinch {}",
        on(c.hit_live()),
        on(c.in_window()),
        on(f & kh1_core::motion::flag::FREE != 0),
        on(f & kh1_core::motion::flag::TRAIL != 0),
        on(c.lock),
        on(c.flinch_immune())
    );
    let (gl, al) = (c.params.ground_length(&c.abilities), c.params.air_length(&c.abilities));
    let _ = writeln!(s, "combo counter {} / {} (air {})   row {:?}   hits this swing {:#x}", c.combo.counter, gl, al, c.combo.row, c.combo.hits);
    let _ = writeln!(s, "pos ({:.1}, {:.1}, {:.1})   facing {:.1} deg   air vel ({:.2}, {:.2}, {:.2})", c.pos.x, c.pos.y, c.pos.z, c.facing.to_degrees(), c.air_vel.x, c.air_vel.y, c.air_vel.z);
    let (mp, max) = sim.mp();
    let bar_n = 20;
    let filled = if max > 0 { ((mp / max as f32) * bar_n as f32).round() as usize } else { 0 };
    let _ = writeln!(s, "MP {:.2} / {}  [{}{}]  (charge gauge {} units, 30 = 1 MP)", mp, max, "#".repeat(filled.min(bar_n)), ".".repeat(bar_n - filled.min(bar_n)), sim.attacker.mp_gauge);
    let lock = sim.lock.map_or("none".to_string(), |id| sim.info[id as usize - 1].name.to_string());
    let _ = writeln!(s, "lock-on {lock}   Strength {}   Tab gizmos {}", sim.attacker.strength, on(view.gizmos));
    let a = &c.abilities;
    let _ = writeln!(
        s,
        "abilities  [1] Guard {}  [2] Dodge Roll {}  [3] High Jump {}  [4] Glide {}  [5] Superglide {}\n           [6] Combo Plus x{}  [7] Air Combo Plus x{}  [8] Combo Master {}",
        on(a.guard),
        on(a.dodge_roll),
        on(a.high_jump),
        on(a.glide),
        on(a.superglide),
        a.combo_plus,
        a.air_combo_plus,
        on(a.combo_master)
    );
    let _ = writeln!(s, "\ndummies");
    for (d, info) in sim.dummies.iter().zip(&sim.info) {
        let st = match d.state {
            _ if d.stats.hp <= 0 => format!("beaten (back in {})", 120u32.saturating_sub(info.respawn)),
            DummyState::Idle => "idle".into(),
            DummyState::Flinch { left } => format!("flinch {left:.0}"),
            DummyState::Air { .. } => "launched".into(),
            DummyState::Down { left } => format!("down {left:.0}"),
        };
        let last = info.last_hit.map_or(String::new(), |(dmg, crit, at)| format!("   last -{dmg}{} @{at}", if crit { " crit" } else { "" }));
        let dist = (d.pos - c.pos).flat().length();
        let _ = writeln!(s, "  {:<5} HP {:>3}/{:<3} {:<14} dist {:>5.0}{}   total {}", info.name, d.stats.hp, d.stats.max_hp, st, dist, last, info.total);
    }
    let _ = writeln!(s, "\nevents");
    for (at, e) in sim.log.iter().rev().take(8) {
        let _ = writeln!(s, "  {at:>6}  {e}");
    }
    text.0 = s;
}

/// HP and the last damage over each dummy.
pub fn labels(game: Res<Game>, cams: Query<(&Camera, &GlobalTransform)>, mut q: Query<(&Label, &mut Text, &mut Node, &mut Visibility)>) {
    let Ok((cam, ct)) = cams.get_single() else { return };
    for (Label(i), mut text, mut node, mut vis) in &mut q {
        let (Some(d), Some(info)) = (game.sim.dummies.get(*i), game.sim.info.get(*i)) else { continue };
        let head = to_bevy(d.pos + kh1_core::Vec3::new(0.0, -185.0, 0.0));
        match cam.world_to_viewport(ct, head) {
            Ok(p) => {
                *vis = Visibility::Inherited;
                node.left = Val::Px(p.x - 40.0);
                node.top = Val::Px(p.y - 30.0);
                let recent = info.last_hit.filter(|h| game.sim.frame.saturating_sub(h.2) < 60).map_or(String::new(), |(dmg, crit, _)| format!("\n-{dmg}{}", if crit { "!" } else { "" }));
                text.0 = format!("{} {}/{}{}", info.name, d.stats.hp, d.stats.max_hp, recent);
            }
            Err(_) => *vis = Visibility::Hidden,
        }
    }
}

pub fn show_error(mut commands: Commands, e: Res<ErrorText>) {
    commands.spawn(Camera2d);
    commands.spawn((
        Text::new(e.0.clone()),
        TextFont { font_size: 18.0, ..default() },
        TextColor(Color::srgb(1.0, 0.65, 0.6)),
        Node { position_type: PositionType::Absolute, top: Val::Px(24.0), left: Val::Px(24.0), right: Val::Px(24.0), ..default() },
    ));
}
