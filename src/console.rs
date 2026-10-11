//! In-game developer console (` key): UI, input capture, and applying parsed actions.
use crate::audio::MusicCtl;
use crate::commands::*;
use crate::enemies::{spawn_enemy, Director, Enemy, EnemyKind, Tgt};
use crate::game::*;
use crate::player::Walker;
use crate::settings::Cvars;
use crate::story::{Story, STEPS};
use crate::terrain;
use crate::vehicle::Vehicle;
use crate::world::{RegenWorld, WorldMats};
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

#[derive(Resource, Default)]
pub struct ConsoleState {
    pub input: String,
    pub log: Vec<(LineKind, String)>,
    pub history: Vec<String>,
    pub hist_pos: Option<usize>,
    pub pending: Vec<String>,
    pub script: Vec<String>,
    pub script_delay: f32,
    pub tab: Vec<String>,
    pub tab_idx: usize,
    pub caret: f32,
    pub shots: u32,
}

#[derive(Resource, Default)]
struct ActionQueue(Vec<Action>);

#[derive(Component)]
struct ConRoot;
#[derive(Component)]
struct ConLine(usize);
#[derive(Component)]
struct ConInput;

const LINES: usize = 22;

pub struct ConsolePlugin;
impl Plugin for ConsolePlugin {
    fn build(&self, app: &mut App) {
        let mut st = ConsoleState::default();
        st.log.push((LineKind::Info, "NEON DUST dev console - type `help`. Cheats: `cheats on`.".into()));
        // --cmd "line" (repeatable): queued, run once the game is playing
        let args: Vec<String> = std::env::args().collect();
        for (i, a) in args.iter().enumerate() {
            if a == "--cmd" {
                if let Some(l) = args.get(i + 1) {
                    st.script.push(l.clone());
                }
            }
        }
        if args.iter().any(|a| a == "--docs") {
            let _ = std::fs::write("commands.txt", render_commands_txt());
            println!("wrote commands.txt");
            std::process::exit(0);
        }
        app.insert_resource(st)
            .init_resource::<ActionQueue>()
            .add_systems(Startup, setup_console)
            .add_systems(PostStartup, |mut o: ResMut<ConsoleOpen>| {
                if std::env::args().any(|a| a == "--console") {
                    o.0 = true;
                }
            })
            .add_systems(Update, (console_input, run_script, console_exec, apply_actions, console_view).chain());
    }
}

fn setup_console(mut commands: Commands) {
    commands
        .spawn((
            Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(52.0), flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(10.0)), border: UiRect::bottom(Val::Px(2.0)), display: Display::None, ..default() },
            BackgroundColor(Color::srgba(0.02, 0.0, 0.08, 0.88)),
            BorderColor(Color::srgb(1.0, 0.15, 0.8)),
            GlobalZIndex(900),
            ConRoot,
        ))
        .with_children(|p| {
            for i in 0..LINES {
                p.spawn((Text::new(""), TextFont { font: crate::fonts::mono(), font_size: 15.0, ..default() }, TextColor(Color::WHITE), ConLine(i)));
            }
            p.spawn((Text::new(""), TextFont { font: crate::fonts::mono(), font_size: 18.0, ..default() }, TextColor(Color::srgb(0.1, 0.95, 1.0)), Node { margin: UiRect::top(Val::Auto), ..default() }, ConInput));
        });
}

fn console_input(
    mut ev: EventReader<KeyboardInput>,
    mut c: ResMut<ConsoleState>,
    mut open: ResMut<ConsoleOpen>,
    time: Res<Time<Real>>,
) {
    c.caret += time.delta_secs();
    for e in ev.read() {
        if e.state != ButtonState::Pressed {
            continue;
        }
        if e.key_code == KeyCode::Backquote || e.key_code == KeyCode::IntlBackslash {
            open.0 = !open.0;
            c.input.clear();
            continue;
        }
        if !open.0 {
            continue;
        }
        match &e.logical_key {
            Key::Escape => open.0 = false,
            Key::Enter => {
                let line = std::mem::take(&mut c.input);
                if !line.trim().is_empty() {
                    c.history.push(line.clone());
                    c.pending.push(line);
                }
                c.hist_pos = None;
            }
            Key::Backspace => {
                c.input.pop();
                c.tab.clear();
            }
            Key::ArrowUp => {
                if !c.history.is_empty() {
                    let p = c.hist_pos.map(|p| p.saturating_sub(1)).unwrap_or(c.history.len() - 1);
                    c.hist_pos = Some(p);
                    c.input = c.history[p].clone();
                }
            }
            Key::ArrowDown => {
                if let Some(p) = c.hist_pos {
                    if p + 1 < c.history.len() {
                        c.hist_pos = Some(p + 1);
                        c.input = c.history[p + 1].clone();
                    } else {
                        c.hist_pos = None;
                        c.input.clear();
                    }
                }
            }
            Key::Tab => {
                if c.tab.is_empty() {
                    c.tab = complete(&c.input);
                    c.tab_idx = 0;
                } else {
                    c.tab_idx = (c.tab_idx + 1) % c.tab.len();
                }
                if let Some(w) = c.tab.get(c.tab_idx).cloned() {
                    let keep = match c.input.rfind(' ') {
                        Some(i) => c.input[..=i].to_string(),
                        None => String::new(),
                    };
                    c.input = format!("{keep}{w} ");
                }
                if c.tab.len() > 1 && c.tab_idx == 0 {
                    let l = c.tab.join("  ");
                    c.log.push((LineKind::Info, l));
                }
            }
            Key::Space => {
                c.input.push(' ');
                c.tab.clear();
            }
            Key::Character(s) => {
                if c.input.len() < 120 {
                    for ch in s.chars() {
                        if !ch.is_control() && ch != '`' {
                            c.input.push(ch);
                        }
                    }
                }
                c.tab.clear();
            }
            _ => {}
        }
    }
}

fn run_script(time: Res<Time<Real>>, state: Res<State<AppState>>, mut c: ResMut<ConsoleState>) {
    if c.script.is_empty() || *state.get() != AppState::Playing {
        return;
    }
    c.script_delay += time.delta_secs();
    if c.script_delay > 1.5 {
        c.script_delay = 1.0;
        let l = c.script.remove(0);
        c.pending.push(l);
    }
}

#[allow(clippy::too_many_arguments)]
fn console_exec(
    mut c: ResMut<ConsoleState>,
    mut q: ResMut<ActionQueue>,
    cheats: Res<Cheats>,
    stats: Res<Stats>,
    story: Res<Story>,
    ctl: Res<Control>,
    cv: Res<Cvars>,
    tgt: Res<Tgt>,
    enemies: Query<&Enemy>,
    car: Query<&Vehicle, With<PlayerCar>>,
) {
    if c.pending.is_empty() {
        return;
    }
    let lines = std::mem::take(&mut c.pending);
    let mut cheats_now = cheats.enabled;
    for line in lines {
        let snap = Snapshot {
            cheats_on: cheats_now,
            pos: tgt.pos.to_array(),
            hp: stats.hp,
            kills: stats.kills,
            score: stats.score,
            seed: terrain::seed(),
            step: story.step.name().to_string(),
            in_car: ctl.in_car,
            enemies: enemies.iter().count(),
            speed_kmh: car.single().map(|v| v.speed * 3.6).unwrap_or(0.0),
            history: c.history.clone(),
            cvars: format!("fov {} bloom {:.2} fog {} viewdist {} music {:.2} sfx {:.2}", cv.fov, cv.bloom, cv.fog_vis, cv.view_radius, cv.music_vol, cv.sfx_vol),
            version: env!("CARGO_PKG_VERSION"),
        };
        c.log.push((LineKind::Cmd, format!("> {line}")));
        let out = execute(&line, &snap);
        for a in &out.actions {
            if let Action::CheatsOn(b) = a {
                cheats_now = *b;
            }
        }
        c.log.extend(out.lines);
        q.0.extend(out.actions);
        let n = c.log.len();
        if n > 300 {
            c.log.drain(..n - 300);
        }
    }
}

#[derive(bevy::ecs::system::SystemParam)]
struct Env<'w, 's> {
    commands: Commands<'w, 's>,
    meshes: ResMut<'w, Assets<Mesh>>,
    mats: Res<'w, WorldMats>,
    cheats: ResMut<'w, Cheats>,
    stats: ResMut<'w, Stats>,
    story: ResMut<'w, Story>,
    ctl: ResMut<'w, Control>,
    cv: ResMut<'w, Cvars>,
    tgt: Res<'w, Tgt>,
    regen: ResMut<'w, RegenWorld>,
    music: ResMut<'w, MusicCtl>,
    vtime: ResMut<'w, Time<Virtual>>,
    fx: ResMut<'w, FxQueue>,
    dir: ResMut<'w, Director>,
    aim: Res<'w, crate::camera::AimInfo>,
    rig: Res<'w, crate::camera::CamRig>,
}

#[allow(clippy::too_many_arguments)]
fn apply_actions(
    mut q: ResMut<ActionQueue>,
    mut env: Env,
    mut con: ResMut<ConsoleState>,
    mut exit: EventWriter<AppExit>,
    mut car: Query<(&mut Transform, &mut Vehicle), With<PlayerCar>>,
    mut pl: Query<(&mut Transform, &mut Walker), (With<Player>, Without<PlayerCar>)>,
    mut foes: Query<&mut Enemy>,
    mut vis: Query<&mut Visibility, With<Player>>,
) {
    let acts: Vec<Action> = q.0.drain(..).collect();
    for a in acts {
        match a {
            Action::CheatsOn(b) => env.cheats.enabled = b,
            Action::Toggle(f, v) => {
                let c = &mut *env.cheats;
                let r = match f {
                    Flag::God => &mut c.god,
                    Flag::Noclip => &mut c.noclip,
                    Flag::InfAmmo => &mut c.infammo,
                    Flag::OneHit => &mut c.onehit,
                    Flag::Peaceful => &mut c.peaceful,
                    Flag::InfBoost => &mut c.infboost,
                    Flag::LowGrav => &mut c.lowgrav,
                };
                *r = v.unwrap_or(!*r);
            }
            Action::Speed(v) => env.cheats.speed_mul = v,
            Action::Heal => {
                env.stats.hp = env.stats.max_hp;
                if let Ok((_, mut v)) = car.single_mut() {
                    v.boost_fuel = 1.0;
                }
            }
            Action::Kill => env.stats.hp = 0.0,
            Action::KillAll => {
                for mut e in &mut foes {
                    e.hp = -1.0;
                }
            }
            Action::GiveAll => {
                env.stats.owned = [true; NUM_WEAPONS];
                env.stats.ammo = [0, 400, 60, 30];
            }
            Action::GiveAmmo => {
                env.stats.ammo[1] += 200;
                env.stats.ammo[2] += 30;
                env.stats.ammo[3] += 15;
            }
            Action::GiveBoost => {
                if let Ok((_, mut v)) = car.single_mut() {
                    v.boost_fuel = 1.0;
                }
            }
            Action::GiveWeapon(i) => {
                env.stats.owned[i] = true;
                env.stats.cur = i;
                env.stats.ammo[i] += 40;
            }
            Action::Tp(x, z) => {
                let y = terrain::ground_or_rim(x, z);
                teleport(&mut env.ctl, &mut car, &mut pl, Vec3::new(x, y + 1.0, z));
            }
            Action::GotoStart => {
                let s = crate::world::start_pos();
                let y = terrain::ground_or_rim(s.x, s.y);
                teleport(&mut env.ctl, &mut car, &mut pl, Vec3::new(s.x, y + 1.0, s.y));
            }
            Action::GotoTarget => {
                if let Some(t) = env.story.target {
                    teleport(&mut env.ctl, &mut car, &mut pl, t + Vec3::new(0.0, 1.0, 40.0));
                } else {
                    con.log.push((LineKind::Err, "no active target".into()));
                }
            }
            Action::GotoCar => {
                if let (Ok((ct, _)), Ok((mut pt, _))) = (car.single(), pl.single_mut()) {
                    pt.translation = ct.translation + Vec3::new(3.0, 1.0, 0.0);
                }
            }
            Action::CarHere => {
                let fwd = Vec3::new(-env.rig.yaw.sin(), 0.0, -env.rig.yaw.cos());
                let p = env.tgt.pos + fwd * 6.0;
                if let Ok((mut ct, mut v)) = car.single_mut() {
                    ct.translation = Vec3::new(p.x, terrain::ground_or_rim(p.x, p.z) + 1.0, p.z);
                    v.vel = Vec3::ZERO;
                    v.yaw = env.rig.yaw;
                    v.yaw_rate = 0.0;
                }
            }
            Action::Drive => {
                env.ctl.in_car = true;
                for mut v in &mut vis {
                    *v = Visibility::Hidden;
                }
            }
            Action::Flip => {
                if let Ok((mut ct, mut v)) = car.single_mut() {
                    v.up = Vec3::Y;
                    v.roll = 0.0;
                    ct.translation.y += 1.0;
                }
            }
            Action::Spawn(kind, n) => {
                let k = match kind.as_str() {
                    "drone" => EnemyKind::Drone,
                    "sentry" => EnemyKind::Sentry,
                    "car" => EnemyKind::Pursuit,
                    _ => EnemyKind::Boss,
                };
                let fwd = Vec3::new(-env.rig.yaw.sin(), 0.0, -env.rig.yaw.cos());
                for i in 0..n {
                    let side = Vec3::new(env.rig.yaw.cos(), 0.0, -env.rig.yaw.sin());
                    let p = env.tgt.pos + fwd * 45.0 + side * ((i as f32 - n as f32 * 0.5) * 8.0);
                    let g = terrain::ground_or_rim(p.x, p.z);
                    let y = match k {
                        EnemyKind::Drone => g + 12.0,
                        EnemyKind::Boss => g + 45.0,
                        _ => g + 0.5,
                    };
                    spawn_enemy(&mut env.commands, &mut env.meshes, &env.mats, k, Vec3::new(p.x, y, p.z));
                }
            }
            Action::Wave => {
                let fwd = Vec3::new(-env.rig.yaw.sin(), 0.0, -env.rig.yaw.cos());
                for i in 0..6 {
                    let a = i as f32 * 1.05;
                    let p = env.tgt.pos + fwd * 50.0 + Vec3::new(a.cos() * 25.0, 0.0, a.sin() * 25.0);
                    let g = terrain::ground_or_rim(p.x, p.z);
                    spawn_enemy(&mut env.commands, &mut env.meshes, &env.mats, EnemyKind::Drone, Vec3::new(p.x, g + 12.0, p.z));
                }
                let p = env.tgt.pos + fwd * 90.0;
                let g = terrain::ground_or_rim(p.x, p.z);
                spawn_enemy(&mut env.commands, &mut env.meshes, &env.mats, EnemyKind::Pursuit, Vec3::new(p.x, g + 1.0, p.z));
            }
            Action::Explode => {
                let p = env.aim.point;
                env.fx.push(FxCmd::Explosion { pos: p, size: 1.5 });
            }
            Action::Score(n) => env.stats.score += n,
            Action::Mission(m) => {
                if let Some(s) = STEPS.iter().find(|s| s.name() == m) {
                    env.story.pending = Some(*s);
                    env.story.lines.clear();
                    env.story.cur = None;
                }
            }
            Action::NextMission => {
                let i = STEPS.iter().position(|s| *s == env.story.step).unwrap_or(0);
                let n = STEPS[(i + 1).min(STEPS.len() - 1)];
                env.story.pending = Some(n);
            }
            Action::Regen(s) => {
                if let Some(s) = s {
                    terrain::SEED.store(s, std::sync::atomic::Ordering::Relaxed);
                }
                env.regen.0 = true;
                let sp = crate::world::start_pos();
                let y = terrain::ground_or_rim(sp.x, sp.y);
                teleport(&mut env.ctl, &mut car, &mut pl, Vec3::new(sp.x, y + 1.0, sp.y));
            }
            Action::Cvar(n, v) => match n.as_str() {
                "fov" => env.cv.fov = v,
                "bloom" => env.cv.bloom = v,
                "fog" => env.cv.fog_vis = v,
                "viewdist" => env.cv.view_radius = v as i32,
                "musicvol" => env.cv.music_vol = v,
                "sfxvol" => env.cv.sfx_vol = v,
                "sens" => env.cv.sens = v,
                _ => {}
            },
            Action::ShowFps(v) => env.cv.show_fps = v.unwrap_or(!env.cv.show_fps),
            Action::Timescale(v) => env.vtime.set_relative_speed(v),
            Action::Track(t) => env.music.force_track = t,
            Action::Screenshot => {
                con.shots += 1;
                let path = format!("screenshot_{}.png", con.shots);
                env.commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
            }
            Action::Quit => {
                exit.write(AppExit::Success);
            }
            Action::Clear => con.log.clear(),
        }
    }
}

fn teleport(
    ctl: &mut Control,
    car: &mut Query<(&mut Transform, &mut Vehicle), With<PlayerCar>>,
    pl: &mut Query<(&mut Transform, &mut Walker), (With<Player>, Without<PlayerCar>)>,
    p: Vec3,
) {
    if ctl.in_car {
        if let Ok((mut t, mut v)) = car.single_mut() {
            t.translation = p;
            v.vel = Vec3::ZERO;
            v.grounded = false;
            v.last_safe = p;
        }
    }
    if let Ok((mut t, mut w)) = pl.single_mut() {
        t.translation = p + if ctl.in_car { Vec3::ZERO } else { Vec3::ZERO };
        w.vel = Vec3::ZERO;
        w.last_safe = p;
    }
}

fn console_view(
    open: Res<ConsoleOpen>,
    c: Res<ConsoleState>,
    mut root: Query<&mut Node, With<ConRoot>>,
    mut lines: Query<(&mut Text, &mut TextColor, &ConLine), Without<ConInput>>,
    mut input: Query<&mut Text, With<ConInput>>,
) {
    if let Ok(mut n) = root.single_mut() {
        n.display = if open.0 { Display::Flex } else { Display::None };
    }
    if !open.0 {
        return;
    }
    let start = c.log.len().saturating_sub(LINES);
    let visible = &c.log[start..];
    for (mut t, mut col, l) in &mut lines {
        match visible.get(l.0) {
            Some((k, s)) => {
                t.0 = s.clone();
                col.0 = match k {
                    LineKind::Info => Color::srgb(0.85, 0.8, 1.0),
                    LineKind::Ok => Color::srgb(0.3, 1.0, 0.55),
                    LineKind::Err => Color::srgb(1.0, 0.3, 0.35),
                    LineKind::Cmd => Color::srgb(0.1, 0.95, 1.0),
                };
            }
            None => t.0.clear(),
        }
    }
    if let Ok(mut t) = input.single_mut() {
        let caret = if (c.caret * 2.0) as i32 % 2 == 0 { "_" } else { " " };
        t.0 = format!("> {}{}", c.input, caret);
    }
}
