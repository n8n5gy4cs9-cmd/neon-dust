//! Third-person rig per player slot: chase camera for the car, over-the-shoulder on foot,
//! aim ray, shake, FOV, and split-screen viewports.
use crate::game::*;
use crate::input::Inputs;
use crate::players::*;
use crate::settings::Cvars;
use crate::split;
use crate::terrain;
use crate::vehicle::Vehicle;
use crate::world::{CamSlot, Focus, MainCam};
use bevy::prelude::*;
use bevy::render::camera::Viewport;
use bevy::window::PrimaryWindow;

#[derive(Clone)]
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

#[derive(Clone, Default)]
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
        app.add_systems(Update, camera_update.run_if(in_state(AppState::Playing)).after(crate::vehicle::VehicleSet))
            .add_systems(Update, menu_cam.run_if(not(in_state(AppState::Playing)).and(not(in_state(AppState::Paused)))))
            .add_systems(Update, layout_cameras);
    }
}

fn menu_cam(
    time: Res<Time>,
    mut cam: Query<&mut Transform, With<MainCam>>,
    mut focus: ResMut<Focus>,
    state: Res<State<AppState>>,
    car: Query<(&Transform, &PlayerCar), Without<MainCam>>,
) {
    let t = time.elapsed_secs() * 0.08;
    let sp = crate::world::start_pos();
    let mut center = Vec3::new(sp.x, terrain::ground_or_rim(sp.x, sp.y), sp.y);
    if matches!(state.get(), AppState::Dead | AppState::Ending) {
        if let Some((c, _)) = car.iter().find(|(_, p)| p.0 == 0) {
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
    focus.1 = None;
}

/// Apply the split-screen layout (viewport + order) to the player cameras.
fn layout_cameras(pl: Res<Pl>, windows: Query<&Window, With<PrimaryWindow>>, mut cams: Query<(&CamSlot, &mut Camera)>) {
    let Ok(w) = windows.single() else { return };
    let (ww, wh) = (w.physical_width(), w.physical_height());
    if ww == 0 || wh == 0 {
        return;
    }
    let lay = split::layout(pl.mask(), pl.opts);
    let multi = pl.count() > 1;
    for (slot, mut cam) in &mut cams {
        cam.order = slot.0 as isize;
        let want = match (multi, lay[slot.0]) {
            (true, Some(r)) => {
                let (x, y, pw, ph) = r.to_px(ww, wh);
                Some(Viewport { physical_position: UVec2::new(x, y), physical_size: UVec2::new(pw, ph), ..default() })
            }
            _ => None,
        };
        let same = match (&cam.viewport, &want) {
            (None, None) => true,
            (Some(a), Some(b)) => a.physical_position == b.physical_position && a.physical_size == b.physical_size,
            _ => false,
        };
        if !same {
            cam.viewport = want;
        }
    }
}

struct CarInfo {
    pos: Vec3,
    speed: f32,
    yaw: f32,
    vel: Vec3,
    fwd: Vec3,
    boosting: bool,
}

#[allow(clippy::too_many_arguments)]
fn camera_update(
    time: Res<Time>,
    cv: Res<Cvars>,
    inputs: Res<Inputs>,
    mut pl: ResMut<Pl>,
    mut focus: ResMut<Focus>,
    mut shake: ResMut<Shake>,
    car: Query<(&Transform, &Vehicle, &PlayerCar), Without<CamSlot>>,
    player: Query<(&Transform, &Player), (Without<CamSlot>, Without<PlayerCar>)>,
    mut cams: Query<(&CamSlot, &mut Transform, &mut Projection)>,
) {
    let dt = time.delta_secs().min(0.05);
    let mut cars: [Option<CarInfo>; MAXP] = [None, None];
    for (t, v, p) in &car {
        if p.0 < MAXP {
            cars[p.0] = Some(CarInfo { pos: t.translation, speed: v.speed, yaw: v.yaw, vel: v.vel, fwd: v.fwd(), boosting: v.boost && v.boost_fuel > 0.02 });
        }
    }
    let mut feet: [Option<Vec3>; MAXP] = [None, None];
    for (t, p) in &player {
        if p.0 < MAXP {
            feet[p.0] = Some(t.translation);
        }
    }
    let multi = pl.count() > 1;
    for (slot, mut ct, mut proj) in &mut cams {
        let i = slot.0;
        if i >= MAXP || !pl.s[i].active {
            continue;
        }
        let inp = &inputs.0[i];
        let s = &mut pl.s[i];
        s.rig.t += dt;
        if inp.look.length_squared() > 0.0 {
            s.rig.yaw += inp.look.x;
            s.rig.pitch = (s.rig.pitch + inp.look.y).clamp(-0.35, 1.25);
            s.rig.idle = 0.0;
        } else {
            s.rig.idle += dt;
        }
        s.rig.aiming = !s.in_car && inp.alt;
        let (target, dist, fov_t, lookat);
        if s.in_car {
            let Some(c) = &cars[i] else { continue };
            if s.rig.idle > 1.0 {
                let vy = if c.speed > 6.0 { (-c.vel.x).atan2(-c.vel.z) } else { c.yaw };
                let ty = lerp_angle(c.yaw, vy, 0.5);
                s.rig.yaw = lerp_angle(s.rig.yaw, ty, 1.0 - (-3.0 * dt).exp());
                s.rig.pitch += (0.2 - s.rig.pitch) * (1.0 - (-2.0 * dt).exp());
            }
            target = c.pos + Vec3::Y * 1.3;
            dist = 8.0 + c.speed * 0.035 + if c.boosting { 2.5 } else { 0.0 };
            fov_t = cv.fov + (c.speed * 0.2).min(22.0) + if c.boosting { 12.0 } else { 0.0 };
            lookat = target + c.fwd * 5.0;
        } else {
            let Some(f) = feet[i] else { continue };
            target = f + Vec3::Y * 1.55;
            dist = if s.rig.aiming { 2.6 } else { 4.4 };
            fov_t = if s.rig.aiming { cv.fov - 18.0 } else { cv.fov };
            lookat = target;
        }
        let fov_t = if multi { fov_t * 0.9 } else { fov_t };
        s.rig.fov += (fov_t - s.rig.fov) * (1.0 - (-5.0 * dt).exp());
        let rot = Quat::from_rotation_y(s.rig.yaw) * Quat::from_rotation_x(-s.rig.pitch);
        let fwd = rot * Vec3::NEG_Z;
        let right = rot * Vec3::X;
        let shoulder = if s.in_car { Vec3::ZERO } else { right * if s.rig.aiming { 0.8 } else { 0.55 } };
        let desired = target - fwd * dist + shoulder;
        let k = if s.in_car { 1.0 - (-14.0 * dt).exp() } else { 1.0 - (-22.0 * dt).exp() };
        s.rig.pos = if s.rig.pos.distance(desired) > 60.0 { desired } else { s.rig.pos.lerp(desired, k) };
        let mut p = s.rig.pos;
        if let Some(g) = terrain::ground(p.x, p.z) {
            p.y = p.y.max(g + 1.0);
        }
        ct.translation = p;
        let look = if s.in_car { lookat } else { target + fwd * 30.0 };
        ct.look_at(look, Vec3::Y);
        if shake.0 > 0.0 {
            let sq = shake.0 * shake.0;
            let t = s.rig.t * 40.0;
            ct.translation += Vec3::new((t * 1.3).sin(), (t * 1.7).cos(), (t * 0.9).sin()) * 0.35 * sq;
            ct.rotate_local_z((t * 1.1).sin() * 0.03 * sq);
        }
        if let Projection::Perspective(pp) = &mut *proj {
            pp.fov = s.rig.fov.to_radians();
        }
        s.focus = target;

        // aim ray ---------------------------------------------------------
        let f = ct.forward().as_vec3();
        s.aim.origin = ct.translation;
        s.aim.dir = f;
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
        s.aim.point = hit;
    }
    shake.0 = (shake.0 - dt * 1.4).max(0.0);
    focus.0 = pl.s[0].focus;
    focus.1 = if pl.s[1].active { Some(pl.s[1].focus) } else { None };
}
