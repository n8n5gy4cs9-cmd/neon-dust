//! Game session flow: new game, respawn, pause, cursor, menu start.
use crate::camera::CamRig;
use crate::enemies::{Director, Enemy, Pickup};
use crate::game::*;
use crate::player::{spawn_player, Walker};
use crate::terrain;
use crate::vehicle::*;
use crate::weapons::Bullet;
use crate::world::{start_pos, WorldMats};
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
            .insert_resource(Control { in_car: false })
            .add_systems(OnTransition { exited: AppState::Menu, entered: AppState::Playing }, new_game)
            .add_systems(OnTransition { exited: AppState::Dead, entered: AppState::Playing }, respawn)
            .add_systems(Update, (menu_input, pause_input, cursor_grab, sync_block));
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

#[allow(clippy::too_many_arguments)]
fn new_game(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mats: Res<WorldMats>,
    mut stats: ResMut<Stats>,
    mut ctl: ResMut<Control>,
    mut rig: ResMut<CamRig>,
    mut cp: ResMut<Checkpoint>,
    mut dir: ResMut<Director>,
    mut intro: ResMut<Intro>,
    old: Query<Entity, Or<(With<Player>, With<PlayerCar>, With<Enemy>, With<Pickup>, With<Bullet>)>>,
) {
    for e in &old {
        commands.entity(e).despawn();
    }
    *stats = Stats::default();
    *dir = Director::default();
    intro.active = true;
    intro.t = 0.0;
    ctl.in_car = false;
    let sp = start_pos();
    let g = terrain::ground_or_rim(sp.x, sp.y);
    let yaw = 0.0f32;
    let car_pos = Vec3::new(sp.x, g + 0.45, sp.y);
    let ce = spawn_car(&mut commands, &mut meshes, &mats, car_pos, yaw, CarKind::Player);
    commands.entity(ce).insert(PlayerCar);
    let pp = car_pos + Vec3::new(-4.0, 0.0, 2.0);
    let ph = terrain::ground_or_rim(pp.x, pp.z);
    spawn_player(&mut commands, &mut meshes, &mats, Vec3::new(pp.x, ph, pp.z), yaw);
    rig.yaw = yaw;
    rig.pitch = 0.25;
    rig.pos = car_pos + Vec3::new(0.0, 5.0, 10.0);
    cp.pos = car_pos;
    cp.yaw = yaw;
}

fn respawn(
    mut commands: Commands,
    mut stats: ResMut<Stats>,
    mut ctl: ResMut<Control>,
    cp: Res<Checkpoint>,
    mut car: Query<(&mut Transform, &mut Vehicle), With<PlayerCar>>,
    mut pl: Query<(&mut Transform, &mut Walker, &mut Visibility), (With<Player>, Without<PlayerCar>)>,
    foes: Query<Entity, Or<(With<Enemy>, With<Bullet>)>>,
) {
    for e in &foes {
        commands.entity(e).despawn();
    }
    stats.hp = stats.max_hp;
    stats.hurt = 0.0;
    ctl.in_car = true;
    if let Ok((mut t, mut v)) = car.single_mut() {
        let sm = terrain::ground_or_rim(cp.pos.x, cp.pos.z);
        t.translation = Vec3::new(cp.pos.x, sm + 1.0, cp.pos.z);
        v.vel = Vec3::ZERO;
        v.yaw = cp.yaw;
        v.yaw_rate = 0.0;
        v.boost_fuel = 1.0;
        v.last_safe = t.translation;
    }
    if let Ok((_, _, mut vis)) = pl.single_mut() {
        *vis = Visibility::Hidden;
    }
}
