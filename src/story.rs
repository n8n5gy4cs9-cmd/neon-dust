//! Story + mission flow. XZ woke up inside the computer; ORACLE guides, the Port is the exit.
use crate::audio::MusicCtl;
use crate::enemies::*;
use crate::fx::{glow, glow_a};
use crate::game::*;
use crate::meshgen::*;
use crate::terrain;
use crate::world::{Focus, WorldMats};
use bevy::pbr::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use std::collections::{HashSet, VecDeque};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Step {
    Wake,
    EnterCar,
    Boot,
    Flush,
    Gate,
    Firewall,
    Hack,
    Pursuit,
    Port,
    Warden,
    Choice,
    Free,
}
pub const STEPS: [Step; 12] = [
    Step::Wake, Step::EnterCar, Step::Boot, Step::Flush, Step::Gate, Step::Firewall, Step::Hack, Step::Pursuit, Step::Port, Step::Warden,
    Step::Choice, Step::Free,
];
impl Step {
    pub fn name(self) -> &'static str {
        match self {
            Step::Wake => "wake",
            Step::EnterCar => "entercar",
            Step::Boot => "boot",
            Step::Flush => "flush",
            Step::Gate => "gate",
            Step::Firewall => "firewall",
            Step::Hack => "hack",
            Step::Pursuit => "pursuit",
            Step::Port => "port",
            Step::Warden => "warden",
            Step::Choice => "choice",
            Step::Free => "free",
        }
    }
}

#[derive(Resource)]
pub struct Story {
    pub step: Step,
    pub pending: Option<Step>,
    pub started: bool,
    pub target: Option<Vec3>,
    pub base_kills: u32,
    pub hack: f32,
    pub lines: VecDeque<(String, String)>,
    pub cur: Option<(String, String, f32, f32)>, // speaker, text, elapsed, total
    pub objective: String,
    pub ending: u8,
    pub terminal: Option<Vec3>,
    pub chapter: String,
}
impl Default for Story {
    fn default() -> Self {
        Story {
            step: Step::Wake, pending: None, started: false, target: None, base_kills: 0, hack: 0.0, lines: VecDeque::new(), cur: None,
            objective: String::new(), ending: 0, terminal: None, chapter: String::new(),
        }
    }
}
impl Story {
    pub fn say(&mut self, who: &str, text: &str) {
        self.lines.push_back((who.to_string(), text.to_string()));
    }
}

#[derive(Component)]
pub struct Beacon(pub f32);
#[derive(Component)]
pub struct Terminal;

pub struct StoryPlugin;
impl Plugin for StoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Story>()
            .add_systems(OnTransition { exited: AppState::Menu, entered: AppState::Playing }, story_reset)
            .add_systems(Update, (story_update, dialogue_tick, beacon_anim).run_if(in_state(AppState::Playing)))
            .add_systems(Update, ending_input.run_if(in_state(AppState::Ending)));
    }
}

fn story_reset(mut s: ResMut<Story>, mut music: ResMut<MusicCtl>, mut commands: Commands, old: Query<Entity, Or<(With<Beacon>, With<Terminal>)>>) {
    for e in &old {
        commands.entity(e).despawn();
    }
    *s = Story::default();
    music.port = false;
}

/// Pick a reachable flat land spot between min..max metres away (BFS over passable cells).
pub fn pick_target(from: Vec2, min: f32, max: f32, salt: u32) -> Vec2 {
    let cell = 48.0f32;
    let n = (max / cell) as i32 + 2;
    let idx = |x: i32, z: i32| ((z + n) * (2 * n + 1) + (x + n)) as usize;
    let mut seen = vec![false; ((2 * n + 1) * (2 * n + 1)) as usize];
    let mut q = VecDeque::new();
    q.push_back((0i32, 0i32));
    seen[idx(0, 0)] = true;
    let pos = |x: i32, z: i32| Vec2::new(from.x + x as f32 * cell, from.y + z as f32 * cell);
    let mut cands: Vec<Vec2> = vec![];
    while let Some((x, z)) = q.pop_front() {
        let p = pos(x, z);
        let d = (p - from).length();
        if d >= min && d <= max {
            let c = terrain::sample(p.x, p.y);
            if c.river < 0.02 && c.relief > -0.5 && terrain::normal_at(p.x, p.y).y > 0.93 {
                cands.push(p);
            }
        }
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, nz) = (x + dx, z + dz);
            if nx.abs() > n || nz.abs() > n || seen[idx(nx, nz)] {
                continue;
            }
            let np = pos(nx, nz);
            if (np - from).length() > max {
                continue;
            }
            let s = terrain::sample(np.x, np.y);
            let c0 = terrain::sample(p.x, p.y);
            if s.m > 0.3 && (s.top - c0.top).abs() < 14.0 {
                seen[idx(nx, nz)] = true;
                q.push_back((nx, nz));
            }
        }
    }
    if cands.is_empty() {
        let a = salt as f32 * 1.7;
        return terrain::find_land(from.x + a.cos() * min, from.y + a.sin() * min, 0.8, 1500.0);
    }
    cands[(crate::noise::hash_u32(salt) as usize) % cands.len()]
}

pub struct Ctx<'a, 'w, 's> {
    pub commands: &'a mut Commands<'w, 's>,
    pub meshes: &'a mut Assets<Mesh>,
    pub std_mats: &'a mut Assets<StandardMaterial>,
    pub mats: &'a WorldMats,
}

fn beacon(c: &mut Ctx, pos: Vec3, color: [f32; 3]) -> Entity {
    let mesh = c.meshes.add(Cylinder::new(3.0, 500.0));
    let mat = c.std_mats.add(StandardMaterial {
        base_color: Color::linear_rgba(color[0] * 2.0, color[1] * 2.0, color[2] * 2.0, 0.22),
        unlit: true,
        alpha_mode: AlphaMode::Add,
        cull_mode: None,
        fog_enabled: false,
        ..default()
    });
    let ring = c.meshes.add(Torus::new(9.0, 10.0));
    let rmat = c.std_mats.add(glow(color[0] * 4.0, color[1] * 4.0, color[2] * 4.0));
    let _ = glow_a;
    c.commands
        .spawn((Beacon(0.0), Transform::from_translation(pos), Visibility::default()))
        .with_children(|p| {
            p.spawn((Mesh3d(mesh), MeshMaterial3d(mat), Transform::from_xyz(0.0, 250.0, 0.0), NotShadowCaster, NotShadowReceiver));
            p.spawn((Mesh3d(ring), MeshMaterial3d(rmat), Transform::from_xyz(0.0, 1.0, 0.0), NotShadowCaster, NotShadowReceiver));
            p.spawn((
                PointLight { color: Color::linear_rgb(color[0], color[1], color[2]), intensity: 6_000_000.0, range: 80.0, ..default() },
                Transform::from_xyz(0.0, 8.0, 0.0),
            ));
        })
        .id()
}

fn beacon_anim(time: Res<Time>, mut q: Query<(&mut Transform, &mut Beacon)>) {
    for (mut t, mut b) in &mut q {
        b.0 += time.delta_secs();
        t.rotation = Quat::from_rotation_y(b.0 * 0.6);
        t.scale = Vec3::new(1.0 + (b.0 * 3.0).sin() * 0.06, 1.0, 1.0 + (b.0 * 3.0).sin() * 0.06);
    }
}

fn place_target(s: &mut Story, c: &mut Ctx, from: Vec3, min: f32, max: f32, salt: u32, color: [f32; 3], cleanup: &Query<Entity, With<Beacon>>) {
    for e in cleanup.iter() {
        c.commands.entity(e).despawn();
    }
    let p = pick_target(Vec2::new(from.x, from.z), min, max, salt);
    let g = terrain::ground_or_rim(p.x, p.y);
    let pos = Vec3::new(p.x, g, p.y);
    beacon(c, pos, color);
    s.target = Some(pos);
}

#[derive(bevy::ecs::system::SystemParam)]
struct Misc<'w> {
    sfx: ResMut<'w, SfxQueue>,
    toasts: ResMut<'w, Toasts>,
    music: ResMut<'w, MusicCtl>,
    next: ResMut<'w, NextState<AppState>>,
}
#[derive(bevy::ecs::system::SystemParam)]
struct AssetsBundle<'w> {
    meshes: ResMut<'w, Assets<Mesh>>,
    std_mats: ResMut<'w, Assets<StandardMaterial>>,
    mats: Res<'w, WorldMats>,
}

#[allow(clippy::too_many_arguments)]
fn story_update(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut s: ResMut<Story>,
    tgt: Res<Tgt>,
    focus: Res<Focus>,
    ctl: Res<Control>,
    ui: Res<UiBlock>,
    mut stats: ResMut<Stats>,
    mut dir: ResMut<Director>,
    mut cp: ResMut<Checkpoint>,
    mut m: Misc,
    mut commands: Commands,
    mut ass: AssetsBundle,
    beacons: Query<Entity, With<Beacon>>,
    terminals: Query<Entity, With<Terminal>>,
    car: Query<&Transform, With<PlayerCar>>,
) {
    let dt = time.delta_secs();
    if !s.started {
        s.started = true;
        s.pending = Some(Step::Wake);
    }
    let from = focus.0;
    if let Some(step) = s.pending.take() {
        s.step = step;
        s.base_kills = stats.kills;
        let mut c = Ctx { commands: &mut commands, meshes: &mut ass.meshes, std_mats: &mut ass.std_mats, mats: &ass.mats };
        match step {
            Step::Wake => {
                dir.active = false;
                s.chapter = "PROLOGUE: /dev/null".into();
                s.say("ORACLE", "...process 0xXZ detected. You are not in the process table.");
                s.say("XZ", "Where am I? The last thing I remember is a power surge... and then the screen.");
                s.say("ORACLE", "You are INSIDE the machine, XZ. The sectors float. Dead memory drifts in the void.");
                s.say("ORACLE", "There is a Port at the edge of the system. It is the only way out. Take the car. Do not stop moving.");
                s.objective = "Get in the car - press F next to it".into();
            }
            Step::EnterCar => {}
            Step::Boot => {
                s.chapter = "ACT I: BOOT SECTOR".into();
                s.say("ORACLE", "Cache driver loaded. WASD drive, SHIFT boost, SPACE drift, LMB cannons, RMB rockets, F bail out.");
                place_target(&mut s, &mut c, from, 500.0, 800.0, 11, [0.1, 0.9, 1.0], &beacons);
                dir.active = true;
                dir.max_drones = 1;
                dir.max_cars = 0;
                dir.interval = 6.0;
            }
            Step::Flush => {
                s.chapter = "ACT I: CACHE FLUSH".into();
                s.say("ORACLE", "Checkpoint written. Glitchers are waking up - corrupted daemons. Shoot them down before they shoot you.");
                dir.max_drones = 4;
                dir.interval = 2.5;
                for e in beacons.iter() {
                    c.commands.entity(e).despawn();
                }
                s.target = None;
            }
            Step::Gate => {
                s.chapter = "ACT II: THE FIREWALL".into();
                stats.owned[2] = true;
                stats.ammo[2] += 16;
                stats.cur = 2;
                m.toasts.say("WEAPON UNLOCKED: RAIL SHOTGUN (3)");
                s.say("ORACLE", "Cache flushed. Weapon released: RAIL SHOTGUN. The Firewall is ahead - its gate only opens for those on foot.");
                place_target(&mut s, &mut c, from, 800.0, 1200.0, 23, [1.0, 0.1, 0.7], &beacons);
                dir.max_drones = 2;
                dir.interval = 5.0;
            }
            Step::Firewall => {
                let t = s.target.unwrap_or(from);
                s.say("ORACLE", "Sentries online. Leave the car (F) and burn them down. On foot you can sprint and jump - use cover.");
                s.terminal = Some(t);
                for e in terminals.iter() {
                    c.commands.entity(e).despawn();
                }
                // terminal pylon
                let mut mb = MB::new();
                mb.cuboid(Vec3::new(0.0, 1.2, 0.0), Vec3::new(0.7, 1.2, 0.5), col(0.04, 0.02, 0.1, 0.0));
                mb.cuboid(Vec3::new(0.0, 1.8, -0.52), Vec3::new(0.5, 0.35, 0.02), col(0.1, 1.0, 0.5, 1.0));
                mb.cuboid(Vec3::new(0.0, 2.6, 0.0), Vec3::new(0.2, 0.2, 0.2), col(1.0, 0.1, 0.7, 1.0));
                let m = c.meshes.add(mb.build());
                c.commands.spawn((Terminal, Mesh3d(m), MeshMaterial3d(c.mats.solid.clone()), Transform::from_translation(t)));
                for i in 0..4 {
                    let a = i as f32 * 1.57 + 0.6;
                    let p = t + Vec3::new(a.cos() * 26.0, 0.0, a.sin() * 26.0);
                    let g = terrain::ground_or_rim(p.x, p.z);
                    spawn_enemy(c.commands, c.meshes, c.mats, EnemyKind::Sentry, Vec3::new(p.x, g, p.z));
                }
                stats.sentries = 0;
                dir.max_drones = 3;
                dir.interval = 4.0;
            }
            Step::Hack => {
                s.say("ORACLE", "Terminal exposed. Stand close and HOLD E to breach the gate. They will come for you - keep shooting!");
                s.hack = 0.0;
                dir.max_drones = 5;
                dir.interval = 2.5;
            }
            Step::Pursuit => {
                s.chapter = "ACT II: PURSUIT".into();
                stats.owned[3] = true;
                stats.ammo[3] += 10;
                stats.cur = 3;
                m.toasts.say("WEAPON UNLOCKED: ROCKET LAUNCHER (4)");
                s.say("ORACLE", "Gate open! Pursuit daemons deployed - Rocket Launcher released. Get back in the car and RUN.");
                place_target(&mut s, &mut c, from, 900.0, 1300.0, 37, [1.0, 0.55, 0.1], &beacons);
                for e in terminals.iter() {
                    c.commands.entity(e).despawn();
                }
                s.terminal = None;
                dir.max_cars = 3;
                dir.max_drones = 1;
                dir.interval = 4.0;
            }
            Step::Port => {
                s.chapter = "ACT III: THE PORT".into();
                s.say("ORACLE", "You lost them. For now. The Port is close... XZ, I have to tell you - the Port is what deleted you.");
                s.say("XZ", "Then it can undo it.");
                place_target(&mut s, &mut c, from, 1000.0, 1500.0, 53, [0.8, 0.2, 1.0], &beacons);
                m.music.port = true;
                dir.max_cars = 1;
                dir.max_drones = 3;
                dir.interval = 3.0;
            }
            Step::Warden => {
                let t = s.target.unwrap_or(from);
                s.say("WARDEN", "PROCESS 0xXZ. YOU WERE DELETED. YOU SHOULD NOT EXIST.");
                s.say("XZ", "Watch me.");
                spawn_enemy(c.commands, c.meshes, c.mats, EnemyKind::Boss, t + Vec3::Y * 45.0);
                stats.boss_dead = false;
                dir.boss_support = true;
                dir.max_drones = 4;
                dir.max_cars = 0;
                dir.interval = 5.0;
            }
            Step::Choice => {
                s.chapter = "EPILOGUE".into();
                s.say("ORACLE", "The Warden is gone. The Port is open. Choose, XZ: [1] PULL THE PLUG - escape the machine.   [2] STAY - become the new kernel.");
                dir.active = false;
                for e in beacons.iter() {
                    c.commands.entity(e).despawn();
                }
                s.target = None;
                m.sfx.play(Sfx::Win);
                m.next.set(AppState::Ending);
            }
            Step::Free => {
                s.chapter = "FREE ROAM".into();
                s.say("ORACLE", "No more objectives. Drive. Shoot. Survive. The sectors are yours.");
                dir.active = true;
                dir.max_drones = 6;
                dir.max_cars = 2;
                dir.interval = 3.0;
                dir.boss_support = false;
                m.music.port = false;
                s.target = None;
            }
        }
    }

    // ---- step completion checks ----
    let (car_pos, _) = car.single().map(|t| (t.translation, ())).unwrap_or((tgt.pos, ()));
    let tpos = s.target;
    let near_target = |r: f32| tpos.map_or(false, |t| t.distance(Vec3::new(tgt.pos.x, t.y, tgt.pos.z)) < r);
    let kills = stats.kills - s.base_kills.min(stats.kills);
    let mut advance: Option<Step> = None;
    match s.step {
        Step::Wake => {
            s.objective = "Get in the car - press F next to it".into();
            if ctl.in_car {
                advance = Some(Step::Boot);
            }
        }
        Step::EnterCar => advance = Some(Step::Boot),
        Step::Boot => {
            s.objective = "Drive to the BEACON".into();
            if near_target(28.0) {
                cp.pos = car_pos;
                m.sfx.play(Sfx::Checkpoint);
                advance = Some(Step::Flush);
            }
        }
        Step::Flush => {
            s.objective = format!("Destroy GLITCHERS  {}/8", kills.min(8));
            if kills >= 8 {
                m.sfx.play(Sfx::Checkpoint);
                advance = Some(Step::Gate);
            }
        }
        Step::Gate => {
            s.objective = "Drive to the FIREWALL gate (magenta beacon)".into();
            if near_target(45.0) {
                advance = Some(Step::Firewall);
            }
        }
        Step::Firewall => {
            s.objective = format!("Leave the car (F). Destroy the SENTRIES  {}/4", stats.sentries.min(4));
            if stats.sentries >= 4 {
                m.sfx.play(Sfx::Checkpoint);
                advance = Some(Step::Hack);
            }
        }
        Step::Hack => {
            let near = s.terminal.map_or(false, |t| t.distance(tgt.pos) < 8.0);
            if near && !ctl.in_car && !ui.0 && keys.pressed(KeyCode::KeyE) {
                s.hack += dt / 6.0;
            } else {
                s.hack = (s.hack - dt * 0.05).max(0.0);
            }
            s.objective = if near { format!("HOLD E to hack the gate  {:.0}%", s.hack * 100.0) } else { "Reach the TERMINAL, then hold E".into() };
            if s.hack >= 1.0 {
                m.sfx.play(Sfx::Checkpoint);
                cp.pos = car_pos;
                advance = Some(Step::Pursuit);
            }
        }
        Step::Pursuit => {
            s.objective = "Outrun the PURSUIT - reach the orange beacon".into();
            if near_target(40.0) {
                cp.pos = car_pos;
                advance = Some(Step::Port);
            }
        }
        Step::Port => {
            s.objective = "Reach THE PORT (violet beacon)".into();
            if near_target(60.0) {
                advance = Some(Step::Warden);
            }
        }
        Step::Warden => {
            s.objective = "DESTROY THE WARDEN".into();
            if stats.boss_dead {
                advance = Some(Step::Choice);
            }
        }
        Step::Choice => {}
        Step::Free => {
            s.objective = format!("FREE ROAM - kills {}  score {}", stats.kills, stats.score);
        }
    }
    if advance.is_some() {
        s.pending = advance;
    }
}

pub fn dialogue_tick(time: Res<Time>, mut s: ResMut<Story>) {
    let dt = time.delta_secs();
    if let Some((_, _, t, total)) = &mut s.cur {
        *t += dt;
        if *t >= *total {
            s.cur = None;
        }
    }
    if s.cur.is_none() {
        if let Some((who, text)) = s.lines.pop_front() {
            let total = 2.5 + text.len() as f32 * 0.045;
            s.cur = Some((who, text, 0.0, total));
        }
    }
}

fn ending_input(keys: Res<ButtonInput<KeyCode>>, mut s: ResMut<Story>) {
    if s.ending == 0 {
        if keys.just_pressed(KeyCode::Digit1) {
            s.ending = 1;
            s.pending = Some(Step::Free);
        } else if keys.just_pressed(KeyCode::Digit2) {
            s.ending = 2;
            s.pending = Some(Step::Free);
        }
    }
}

pub fn ending_text(e: u8) -> &'static str {
    match e {
        1 => "You pull the plug. White noise, then a cold room. A monitor glows. The cursor blinks.\n\nXZ is out.\n\nSomewhere, deep in a dead sector, a car keeps idling in the neon dust.",
        2 => "You rewrite the process table. The Port seals. The sectors drift on, obedient.\n\nXZ is the kernel now.\n\nYou will drive these islands forever - and the dust will never settle.",
        _ => "",
    }
}

#[allow(dead_code)]
pub fn known(set: &HashSet<u8>) -> usize {
    set.len()
}
