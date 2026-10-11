//! Third-person rig: chase camera for the car, over-the-shoulder on foot, aim ray, shake, FOV.
use crate::game::*;
use crate::settings::Cvars;
use crate::terrain;
use crate::vehicle::Vehicle;
use crate::world::{Focus, MainCam};
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;

#[derive(Resource)]
pub struct CamRig {
    pub yaw: f32,
    pub pitch: f32,
    pub idle: f32,
    pub pos: Vec3,
    pub fov: f32,
    pub aiming: bool,
    pub t: f32,
}
impl Default for CamRig {
    fn default() -> Self {
        CamRig { yaw: 0.0, pitch: 0.25, idle: 10.0, pos: Vec3::ZERO, fov: 70.0, aiming: false, t: 0.0 }
    }
}

#[derive(Resource, Default)]
pub struct AimInfo {
    pub point: Vec3,
    pub origin: Vec3,
    pub dir: Vec3,
}

pub fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    let mut d = (b - a) % std::f32::consts::TAU;
    if d > std::f32::consts::PI {
        d -= std::f32::consts::TAU;
    } else if d < -std::f32::consts::PI {
        d += std::f32::consts::TAU;
    }
    a + d * t.clamp(0.0, 1.0)
}

pub struct CameraPlugin;
impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CamRig>()
            .init_resource::<AimInfo>()
            .add_systems(Update, camera_update.run_if(in_state(AppState::Playing)).after(crate::vehicle::VehicleSet))
            .add_systems(Update, menu_cam.run_if(not(in_state(AppState::Playing)).and(not(in_state(AppState::Paused)))));
    }
}

fn menu_cam(
    time: Res<Time>,
    mut cam: Query<&mut Transform, With<MainCam>>,
    mut focus: ResMut<Focus>,
    state: Res<State<AppState>>,
    car: Query<&Transform, (With<PlayerCar>, Without<MainCam>)>,
) {
    let t = time.elapsed_secs() * 0.08;
    let sp = crate::world::start_pos();
    let mut center = Vec3::new(sp.x, terrain::ground_or_rim(sp.x, sp.y), sp.y);
    if matches!(state.get(), AppState::Dead | AppState::Ending) {
        if let Ok(c) = car.single() {
            center = c.translation;
        }
    }
    let c = center + Vec3::new(t.cos() * 55.0, 22.0 + (t * 2.0).sin() * 8.0, t.sin() * 55.0);
    let gh = terrain::ground_or_rim(c.x, c.z) + 6.0;
    for mut tr in &mut cam {
        tr.translation = Vec3::new(c.x, c.y.max(gh), c.z);
        tr.look_at(center + Vec3::Y * 3.0, Vec3::Y);
    }
    focus.0 = center;
}

#[allow(clippy::too_many_arguments)]
fn camera_update(
    time: Res<Time>,
    motion: Res<AccumulatedMouseMotion>,
    mouse: Res<ButtonInput<MouseButton>>,
    ui: Res<UiBlock>,
    ctl: Res<Control>,
    cv: Res<Cvars>,
    mut rig: ResMut<CamRig>,
    mut aim: ResMut<AimInfo>,
    mut focus: ResMut<Focus>,
    mut shake: ResMut<Shake>,
    car: Query<(&Transform, &Vehicle), (With<PlayerCar>, Without<MainCam>)>,
    player: Query<&Transform, (With<Player>, Without<MainCam>, Without<PlayerCar>)>,
    mut cam: Query<(&mut Transform, &mut Projection), With<MainCam>>,
) {
    let Ok((mut ct, mut proj)) = cam.single_mut() else { return };
    let dt = time.delta_secs().min(0.05);
    rig.t += dt;
    let d = if ui.0 { Vec2::ZERO } else { motion.delta };
    if d.length_squared() > 0.0 {
        rig.yaw -= d.x * cv.sens;
        rig.pitch = (rig.pitch + d.y * cv.sens).clamp(-0.35, 1.25);
        rig.idle = 0.0;
    } else {
        rig.idle += dt;
    }
    rig.aiming = !ctl.in_car && !ui.0 && mouse.pressed(MouseButton::Right);

    let (target, dist, fov_t, lookat);
    if ctl.in_car {
        let Ok((t, v)) = car.single() else { return };
        let sp = v.speed;
        if rig.idle > 1.0 {
            let vy = if sp > 6.0 { (-v.vel.x).atan2(-v.vel.z) } else { v.yaw };
            let ty = lerp_angle(v.yaw, vy, 0.5);
            rig.yaw = lerp_angle(rig.yaw, ty, 1.0 - (-3.0 * dt).exp());
            rig.pitch += (0.2 - rig.pitch) * (1.0 - (-2.0 * dt).exp());
        }
        target = t.translation + Vec3::Y * 1.3;
        dist = 8.0 + sp * 0.035 + if v.boost && v.boost_fuel > 0.02 { 2.5 } else { 0.0 };
        fov_t = cv.fov + (sp * 0.2).min(22.0) + if v.boost && v.boost_fuel > 0.02 { 12.0 } else { 0.0 };
        lookat = target + v.fwd() * 5.0;
    } else {
        let Ok(t) = player.single() else { return };
        target = t.translation + Vec3::Y * 1.55;
        dist = if rig.aiming { 2.6 } else { 4.4 };
        fov_t = if rig.aiming { cv.fov - 18.0 } else { cv.fov };
        lookat = target;
    }
    rig.fov += (fov_t - rig.fov) * (1.0 - (-5.0 * dt).exp());
    let rot = Quat::from_rotation_y(rig.yaw) * Quat::from_rotation_x(-rig.pitch);
    let fwd = rot * Vec3::NEG_Z;
    let right = rot * Vec3::X;
    let shoulder = if ctl.in_car { Vec3::ZERO } else { right * if rig.aiming { 0.8 } else { 0.55 } };
    let desired = target - fwd * dist + shoulder;
    let k = if ctl.in_car { 1.0 - (-14.0 * dt).exp() } else { 1.0 - (-22.0 * dt).exp() };
    rig.pos = if rig.pos.distance(desired) > 60.0 { desired } else { rig.pos.lerp(desired, k) };
    let mut p = rig.pos;
    if let Some(g) = terrain::ground(p.x, p.z) {
        p.y = p.y.max(g + 1.0);
    }
    ct.translation = p;
    let look = if ctl.in_car { lookat } else { target + fwd * 30.0 };
    ct.look_at(look, Vec3::Y);
    if ctl.in_car {
        // slight chase-cam look-at smoothing already via rig.pos lag
    }
    // shake
    shake.0 = (shake.0 - dt * 1.4).max(0.0);
    if shake.0 > 0.0 {
        let s = shake.0 * shake.0;
        let t = rig.t * 40.0;
        ct.translation += Vec3::new((t * 1.3).sin(), (t * 1.7).cos(), (t * 0.9).sin()) * 0.35 * s;
        ct.rotate_local_z((t * 1.1).sin() * 0.03 * s);
    }
    if let Projection::Perspective(pp) = &mut *proj {
        pp.fov = rig.fov.to_radians();
    }
    focus.0 = target;

    // aim ray ---------------------------------------------------------
    let f = ct.forward().as_vec3();
    aim.origin = ct.translation;
    aim.dir = f;
    let start = dist.max(3.0);
    let mut hit = ct.translation + f * 300.0;
    let mut d = start;
    while d < 300.0 {
        let q = ct.translation + f * d;
        if let Some(g) = terrain::ground(q.x, q.z) {
            if q.y < g {
                hit = ct.translation + f * (d - 2.0);
                break;
            }
        }
        d += 4.0;
    }
    aim.point = hit;
}
