//! Game session flow: new game, respawn, pause, cursor, menu start.
use crate::enemies::{Director, Enemy, Pickup};
use crate::game::*;
use crate::input::Inputs;
use crate::players::*;
use crate::player::spawn_player;
use crate::terrain;
use crate::vehicle::*;
use crate::weapons::Bullet;
use crate::world::{spawn_game_camera, start_pos, CamSlot, WorldMats};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};

pub struct SessionPlugin;
impl Plugin for SessionPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .init_resource::<Stats>()
            .init_resource::<Cheats>()
            .init_resource::<UiBlock>()
            .init_resource::<Toasts>()
            .init_resource::<SfxQueue>()
            .init_resource::<Checkpoint>()
            .init_resource::<ConsoleOpen>()
            .init_resource::<Loading>()
            .init_resource::<Intro>()
            .init_resource::<Pl>()
            .add_systems(OnTransition { exited: AppState::Menu, entered: AppState::Playing }, new_game)
            .add_systems(OnTransition { exited: AppState::Dead, entered: AppState::Playing }, respawn)
            .add_systems(Update, (menu_input, pause_input, cursor_grab, sync_block))
            .add_systems(Update, (join_leave, sync_players).chain().run_if(in_state(AppState::Playing)));
    }
}

fn sync_block(console: Res<ConsoleOpen>, intro: Res<Intro>, mut ui: ResMut<UiBlock>) {
    ui.0 = console.0 || intro.active;
}

fn cursor_grab(state: Res<State<AppState>>, console: Res<ConsoleOpen>, mut win: Query<&mut Window, With<PrimaryWindow>>) {
    let Ok(mut w) = win.single_mut() else { return };
    let lock = *state.get() == AppState::Playing && !console.0;
    let (mode, vis) = if lock { (CursorGrabMode::Locked, false) } else { (CursorGrabMode::None, true) };
    if w.cursor_options.grab_mode != mode {
        w.cursor_options.grab_mode = mode;
        w.cursor_options.visible = vis;
    }
}

fn menu_input(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
    console: Res<ConsoleOpen>,
    loading: Res<Loading>,
    story: Res<crate::story::Story>,
) {
    if console.0 || !loading.done {
        return;
    }
    let go = keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space);
    match state.get() {
        AppState::Menu | AppState::Dead if go => next.set(AppState::Playing),
        AppState::Ending if go && story.ending > 0 => next.set(AppState::Playing),
        AppState::Paused if go => next.set(AppState::Playing),
        _ => {}
    }
}

fn pause_input(keys: Res<ButtonInput<KeyCode>>, state: Res<State<AppState>>, mut next: ResMut<NextState<AppState>>, console: Res<ConsoleOpen>) {
    if console.0 {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        match state.get() {
            AppState::Playing => next.set(AppState::Paused),
            AppState::Paused => next.set(AppState::Playing),
            _ => {}
        }
    }
}

/// Spawn a slot's car + walker (+ camera for slots > 0). Returns the car position used.
fn spawn_slot(commands: &mut Commands, meshes: &mut Assets<Mesh>, mats: &WorldMats, slot: usize, car_pos: Vec3, yaw: f32, on_foot: bool) {
    let ce = spawn_car(commands, meshes, mats, car_pos, yaw, CarKind::Player);
    commands.entity(ce).insert(PlayerCar(slot));
    let pp = car_pos + Vec3::new(-4.0, 0.0, 2.0);
    let ph = terrain::ground_or_rim(pp.x, pp.z);
    let pe = spawn_player(commands, meshes, mats, Vec3::new(pp.x, ph, pp.z), yaw, slot);
    if !on_foot {
        commands.entity(pe).insert(Visibility::Hidden);
    }
    if slot > 0 {
        let cam = spawn_game_camera(commands, slot);
        crate::ui::spawn_hud(commands, slot, Some(cam));
    }
}

#[allow(clippy::too_many_arguments)]
fn new_game(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mats: Res<WorldMats>,
    mut stats: ResMut<Stats>,
    mut pl: ResMut<Pl>,
    mut cp: ResMut<Checkpoint>,
    mut dir: ResMut<Director>,
    mut intro: ResMut<Intro>,
    old: Query<Entity, Or<(With<Player>, With<PlayerCar>, With<Enemy>, With<Pickup>, With<Bullet>)>>,
    cams2: Query<(Entity, &CamSlot)>,
    huds: Query<(Entity, &crate::ui::HudRoot)>,
) {
    for (e, h) in &huds {
        if h.0 > 0 {
            commands.entity(e).despawn();
        }
    }
    for e in &old {
        commands.entity(e).despawn();
    }
    for (e, c) in &cams2 {
        if c.0 > 0 {
            commands.entity(e).despawn();
        }
    }
    *stats = Stats::default();
    *dir = Director::default();
    intro.active = true;
    intro.t = 0.0;
    let sp = start_pos();
    let g = terrain::ground_or_rim(sp.x, sp.y);
    let yaw = 0.0f32;
    let car_pos = Vec3::new(sp.x, g + 0.45, sp.y);
    for i in 0..MAXP {
        pl.s[i].reset_stats();
        pl.s[i].in_car = i > 0;
        if pl.s[i].active {
            let cp_i = car_pos + Vec3::new(8.0 * i as f32, 0.0, 0.0);
            let cp_i = Vec3::new(cp_i.x, terrain::ground_or_rim(cp_i.x, cp_i.z) + 0.45, cp_i.z);
            spawn_slot(&mut commands, &mut meshes, &mats, i, cp_i, yaw, i == 0);
        }
        pl.s[i].rig.yaw = yaw;
        pl.s[i].rig.pitch = 0.25;
        pl.s[i].rig.pos = car_pos + Vec3::new(0.0, 5.0, 10.0);
    }
    cp.pos = car_pos;
    cp.yaw = yaw;
}

/// Player 2 joins with the gamepad Start button.
fn join_leave(inputs: Res<Inputs>, mut pl: ResMut<Pl>, mut toasts: ResMut<Toasts>) {
    if inputs.0[1].start && !pl.s[1].active {
        pl.s[1] = Slot::new(true);
        pl.s[1].in_car = true;
        pl.version += 1;
        toasts.say("PLAYER 2 JOINED (gamepad)");
    }
}

/// Keep entities in step with the active-slot flags (spawn P2 on join, remove on leave).
#[allow(clippy::too_many_arguments)]
fn sync_players(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mats: Res<WorldMats>,
    mut pl: ResMut<Pl>,
    cars: Query<(Entity, &PlayerCar)>,
    walkers: Query<(Entity, &Player)>,
    cams: Query<(Entity, &CamSlot)>,
    huds: Query<(Entity, &crate::ui::HudRoot)>,
) {
    for i in 1..MAXP {
        let has = cars.iter().any(|(_, p)| p.0 == i);
        if pl.s[i].active && !has {
            let base = pl.s[0].car_pos;
            let p = base + Vec3::new(8.0, 0.0, 4.0);
            let car_pos = Vec3::new(p.x, terrain::ground_or_rim(p.x, p.z) + 0.45, p.z);
            spawn_slot(&mut commands, &mut meshes, &mats, i, car_pos, pl.s[0].rig.yaw, false);
            pl.s[i].in_car = true;
            pl.s[i].rig.pos = car_pos + Vec3::new(0.0, 5.0, 10.0);
            pl.s[i].rig.yaw = pl.s[0].rig.yaw;
            // hide the walker while in the car
            for (e, pp) in &walkers {
                if pp.0 == i {
                    commands.entity(e).insert(Visibility::Hidden);
                }
            }
        } else if !pl.s[i].active && has {
            for (e, p) in &cars {
                if p.0 == i {
                    commands.entity(e).despawn();
                }
            }
            for (e, p) in &walkers {
                if p.0 == i {
                    commands.entity(e).despawn();
                }
            }
            for (e, c) in &cams {
                if c.0 == i {
                    commands.entity(e).despawn();
                }
            }
            for (e, h) in &huds {
                if h.0 == i {
                    commands.entity(e).despawn();
                }
            }
        }
    }
}

fn respawn(
    mut commands: Commands,
    mut pl: ResMut<Pl>,
    cp: Res<Checkpoint>,
    mut car: Query<(&mut Transform, &mut Vehicle, &PlayerCar)>,
    mut walkers: Query<(&mut Visibility, &Player)>,
    foes: Query<Entity, Or<(With<Enemy>, With<Bullet>)>>,
) {
    for e in &foes {
        commands.entity(e).despawn();
    }
    for i in 0..MAXP {
        let s = &mut pl.s[i];
        s.hp = s.max_hp;
        s.hurt = 0.0;
        s.dead_t = 0.0;
        s.in_car = true;
    }
    for (mut t, mut v, pc) in &mut car {
        let base = cp.pos + Vec3::new(8.0 * pc.0 as f32, 0.0, 0.0);
        t.translation = Vec3::new(base.x, terrain::ground_or_rim(base.x, base.z) + 1.0, base.z);
        v.vel = Vec3::ZERO;
        v.yaw = cp.yaw;
        v.yaw_rate = 0.0;
        v.boost_fuel = 1.0;
        v.last_safe = t.translation;
    }
    for (mut vis, _) in &mut walkers {
        *vis = Visibility::Hidden;
    }
}
