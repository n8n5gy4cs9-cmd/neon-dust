//! Car physics: DustRacing2D-style 2D rigid body (engine/brake/drag/rolling resistance +
//! slip-angle tyre forces with sliding friction) extended into 3D (x,y -> x,z plane + gravity,
//! heightfield ground following, jumps, pitch/roll from terrain normal).
use crate::game::*;
use crate::input::Inputs;
use crate::players::*;
use crate::meshgen::*;
use crate::terrain;
use crate::world::{Chunks, NeonMat, WorldMats};
use bevy::prelude::*;

pub const GRAVITY: f32 = 24.0;
const MASS: f32 = 1100.0;
const WHEELBASE: f32 = 2.8;
const A_CG: f32 = 1.3; // CG -> front axle
const B_CG: f32 = 1.5; // CG -> rear axle
const INERTIA: f32 = 2400.0;
const RIDE: f32 = 0.45;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CarKind {
    Player,
    Pursuit,
}

#[derive(Component)]
pub struct Vehicle {
    pub kind: CarKind,
    pub vel: Vec3,
    pub yaw: f32,
    pub yaw_rate: f32,
    pub up: Vec3,
    pub throttle: f32,
    pub steer: f32,
    pub brake: f32,
    pub handbrake: bool,
    pub boost: bool,
    pub boost_fuel: f32,
    pub grounded: bool,
    pub slip: f32,
    pub roll: f32,
    pub in_water: bool,
    pub air_time: f32,
    pub power: f32,
    pub last_safe: Vec3,
    pub fall_timer: f32,
    pub speed: f32,
}

impl Vehicle {
    pub fn new(kind: CarKind, yaw: f32) -> Self {
        Vehicle {
            kind,
            vel: Vec3::ZERO,
            yaw,
            yaw_rate: 0.0,
            up: Vec3::Y,
            throttle: 0.0,
            steer: 0.0,
            brake: 0.0,
            handbrake: false,
            boost: false,
            boost_fuel: 1.0,
            grounded: true,
            slip: 0.0,
            roll: 0.0,
            in_water: false,
            air_time: 0.0,
            power: if kind == CarKind::Player { 520_000.0 } else { 400_000.0 },
            last_safe: Vec3::ZERO,
            fall_timer: 0.0,
            speed: 0.0,
        }
    }
    pub fn fwd(&self) -> Vec3 {
        Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos())
    }
    pub fn right(&self) -> Vec3 {
        Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin())
    }
}

pub fn car_mesh(body: [f32; 3], glow: [f32; 3], light: [f32; 3]) -> Mesh {
    let mut m = MB::new();
    let dark = col(body[0], body[1], body[2], 0.0);
    let trim = col(0.02, 0.02, 0.05, 0.0);
    let g = col(glow[0], glow[1], glow[2], 1.0);
    // chassis
    m.cuboid(Vec3::new(0.0, 0.52, 0.0), Vec3::new(0.95, 0.26, 2.15), dark);
    // hood wedge + cabin
    m.quad(Vec3::new(-0.9, 0.78, -2.1), Vec3::new(-0.9, 0.78, -0.5), Vec3::new(0.9, 0.78, -0.5), Vec3::new(0.9, 0.78, -2.1), dark);
    m.cuboid(Vec3::new(0.0, 0.98, 0.35), Vec3::new(0.78, 0.22, 1.05), dark);
    // windshield glass (glowing dark teal)
    m.quad(Vec3::new(-0.74, 0.78, -0.72), Vec3::new(-0.74, 1.2, -0.7 + 0.45), Vec3::new(0.74, 1.2, -0.7 + 0.45), Vec3::new(0.74, 0.78, -0.72), col(0.0, 0.08, 0.18, 0.3));
    // rear wing
    m.cuboid(Vec3::new(0.0, 1.15, 2.0), Vec3::new(0.95, 0.04, 0.28), trim);
    m.cuboid(Vec3::new(-0.7, 0.92, 2.0), Vec3::new(0.05, 0.22, 0.2), trim);
    m.cuboid(Vec3::new(0.7, 0.92, 2.0), Vec3::new(0.05, 0.22, 0.2), trim);
    m.cuboid(Vec3::new(0.0, 1.2, 2.0), Vec3::new(0.95, 0.015, 0.29), g);
    // neon side strips + underglow
    m.cuboid(Vec3::new(-0.97, 0.5, 0.0), Vec3::new(0.02, 0.04, 1.9), g);
    m.cuboid(Vec3::new(0.97, 0.5, 0.0), Vec3::new(0.02, 0.04, 1.9), g);
    m.cuboid(Vec3::new(0.0, 0.2, 0.0), Vec3::new(0.85, 0.02, 1.9), col(glow[0], glow[1], glow[2], 1.0));
    // head + tail lights
    let hl = col(light[0], light[1], light[2], 1.0);
    m.cuboid(Vec3::new(-0.62, 0.6, -2.17), Vec3::new(0.2, 0.06, 0.02), hl);
    m.cuboid(Vec3::new(0.62, 0.6, -2.17), Vec3::new(0.2, 0.06, 0.02), hl);
    m.cuboid(Vec3::new(0.0, 0.62, 2.17), Vec3::new(0.8, 0.05, 0.02), col(1.0, 0.05, 0.1, 1.0));
    // wheels
    for (x, z) in [(-0.95, -1.35), (0.95, -1.35), (-0.95, 1.5), (0.95, 1.5)] {
        m.wheel(Vec3::new(x, 0.38, z), 0.38, 0.34, 10, trim);
        m.wheel(Vec3::new(x + x.signum() * 0.17, 0.38, z), 0.22, 0.03, 10, g);
    }
    m.build()
}

pub fn spawn_car(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &WorldMats,
    pos: Vec3,
    yaw: f32,
    kind: CarKind,
) -> Entity {
    let (body, glow, light) = match kind {
        CarKind::Player => ([0.35, 0.02, 0.4], [0.0, 0.95, 1.0], [1.0, 0.95, 0.8]),
        CarKind::Pursuit => ([0.28, 0.03, 0.02], [1.0, 0.25, 0.05], [1.0, 0.2, 0.1]),
    };
    let mesh = meshes.add(car_mesh(body, glow, light));
    let mut v = Vehicle::new(kind, yaw);
    v.last_safe = pos;
    let e = commands
        .spawn((Transform::from_translation(pos).with_rotation(Quat::from_rotation_y(yaw)), Visibility::default(), v))
        .with_children(|p| {
            p.spawn((Mesh3d(mesh), MeshMaterial3d(mats.solid.clone()), Transform::IDENTITY));
            if kind == CarKind::Player {
                p.spawn((
                    SpotLight { color: Color::srgb(1.0, 0.95, 0.85), intensity: 3_500_000.0, range: 110.0, outer_angle: 0.55, inner_angle: 0.2, ..default() },
                    Transform::from_xyz(0.0, 0.7, -2.0).looking_to(Vec3::NEG_Z, Vec3::Y),
                ));
                p.spawn((PointLight { color: Color::srgb(0.1, 0.9, 1.0), intensity: 220_000.0, range: 10.0, ..default() }, Transform::from_xyz(0.0, 0.15, 0.0)));
            } else {
                p.spawn((PointLight { color: Color::srgb(1.0, 0.2, 0.05), intensity: 120_000.0, range: 8.0, ..default() }, Transform::from_xyz(0.0, 0.3, 0.0)));
            }
        })
        .id();
    e
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct VehicleSet;

pub struct VehiclePlugin;
impl Plugin for VehiclePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (player_car_input, vehicle_physics).chain().in_set(VehicleSet).run_if(in_state(AppState::Playing)));
    }
}

fn player_car_input(inputs: Res<Inputs>, pl: Res<Pl>, mut q: Query<(&mut Vehicle, &PlayerCar)>) {
    for (mut v, pc) in &mut q {
        let i = pc.0;
        if i >= MAXP {
            continue;
        }
        let active = pl.s[i].alive() && pl.s[i].in_car;
        let inp = &inputs.0[i];
        if !active {
            v.throttle = 0.0;
            v.steer *= 0.9;
            v.boost = false;
            v.handbrake = true;
            continue;
        }
        v.throttle = inp.throttle;
        let target = inp.steer;
        let rate = if target == 0.0 { 6.0 } else { 4.5 };
        let t = 1.0 - (-rate * 1.0 / 60.0f32).exp();
        v.steer += (target - v.steer) * t;
        v.handbrake = inp.handbrake;
        v.boost = inp.boost;
        v.brake = 0.0;
    }
}

#[allow(clippy::too_many_arguments)]
fn vehicle_physics(
    time: Res<Time>,
    chunks: Res<Chunks>,
    cheats: Res<Cheats>,
    mut fx: ResMut<FxQueue>,
    mut sfx: ResMut<SfxQueue>,
    mut shake: ResMut<Shake>,
    mut pl: ResMut<Pl>,
    mut q: Query<(&mut Transform, &mut Vehicle, Option<&PlayerCar>)>,
) {
    let dt_full = time.delta_secs().min(1.0 / 20.0);
    let steps = ((dt_full / (1.0 / 120.0)).ceil() as usize).max(1);
    let dt = dt_full / steps as f32;
    let grav = if cheats.lowgrav { GRAVITY * 0.35 } else { GRAVITY };
    for (mut tr, mut v, is_player) in &mut q {
        let mut slip_acc = 0.0;
        for _ in 0..steps {
            let p = tr.translation;
            let sm = terrain::sample(p.x, p.z);
            let on_land = sm.m > 0.015;
            let ground = if on_land { sm.top } else { f32::NEG_INFINITY };
            let fwd = v.fwd();
            let right = v.right();
            let vh = Vec3::new(v.vel.x, 0.0, v.vel.z);
            let vf = vh.dot(fwd);
            let vl = vh.dot(right);
            let speed = vh.length();
            v.speed = speed;
            let pm = if is_player.is_some() { cheats.speed_mul } else { 1.0 };

            // boost
            let boosting = v.boost && (v.boost_fuel > 0.02 || cheats.infboost) && v.throttle >= 0.0;
            if boosting {
                if !cheats.infboost {
                    v.boost_fuel = (v.boost_fuel - dt * 0.35).max(0.0);
                }
            } else {
                v.boost_fuel = (v.boost_fuel + dt * 0.07).min(1.0);
            }
            let bm = if boosting { 2.1 } else { 1.0 };

            if v.grounded {
                // steering angle shrinks with speed
                let max_d = 0.52 / (1.0 + speed / 22.0);
                let delta = v.steer * max_d;
                let sgn = if vf >= 0.0 { 1.0 } else { -1.0 };
                let den = vf.abs().max(2.5) * sgn;
                let af = ((vl - A_CG * v.yaw_rate) / den).atan() - delta;
                let ar = ((vl + B_CG * v.yaw_rate) / den).atan();
                let nf = MASS * 9.81 * B_CG / WHEELBASE + speed * speed * 1.2;
                let nr = MASS * 9.81 * A_CG / WHEELBASE + speed * speed * 1.2;
                let surf = if sm.river > 0.3 { 0.5 } else { 1.0 };
                let mu_f = 1.25 * surf;
                let mu_r = (if v.handbrake { 0.5 } else { 1.2 }) * surf;
                let cf = 11.0 * nf;
                let cr = 11.0 * nr;
                let mut ff = -cf * af;
                let mut fr = -cr * ar;
                let mut sliding = false;
                if ff.abs() > mu_f * nf {
                    ff = ff.signum() * mu_f * nf * 0.9;
                    sliding = true;
                }
                if fr.abs() > mu_r * nr {
                    fr = fr.signum() * mu_r * nr * 0.88;
                    sliding = true;
                }
                // longitudinal
                let mut fx_ = 0.0;
                let pw = v.power * bm * pm;
                let traction = mu_r * nr * if boosting { 1.5 } else { 1.0 };
                if v.throttle > 0.0 {
                    fx_ += (v.throttle * pw / vf.max(7.0)).min(traction);
                } else if v.throttle < 0.0 {
                    if vf > 1.0 {
                        fx_ -= (-v.throttle * 13_000.0).min(traction * 1.2); // brake
                    } else {
                        fx_ -= (-v.throttle * 5000.0).min(traction) * if vf > -18.0 { 1.0 } else { 0.0 };
                    }
                }
                if v.handbrake {
                    fx_ -= vf.signum() * (vf.abs().min(1.0)) * 6000.0;
                }
                // drag + rolling resistance (DustRacing2D: linear rolling + quadratic air)
                let drag_k = if boosting { 0.7 } else { 0.9 };
                fx_ -= vf.signum() * (drag_k * vf * vf + 170.0 * vf.abs().min(1.0));
                if sm.river > 0.3 {
                    fx_ -= vf * 900.0;
                    v.in_water = true;
                } else {
                    v.in_water = false;
                }
                fx_ -= ff * delta.sin();
                let lat = ff * delta.cos() + fr;
                let acc = (fwd * fx_ + right * lat) / MASS;
                // gravity along slope
                let gslope = terrain::normal_at(p.x, p.z);
                let slope_acc = Vec3::new(gslope.x, 0.0, gslope.z) * 9.81;
                v.vel.x += (acc.x + slope_acc.x) * dt;
                v.vel.z += (acc.z + slope_acc.z) * dt;
                let torque = -A_CG * ff * delta.cos() + B_CG * fr;
                v.yaw_rate += (torque / INERTIA) * dt;
                v.yaw_rate -= v.yaw_rate * 1.6 * dt;
                // keep rotation from exploding at standstill
                if speed < 0.5 && v.throttle == 0.0 {
                    v.yaw_rate *= 0.9;
                }
                slip_acc += if sliding { (vl.abs() / (speed + 1.0)).min(1.0) } else { 0.0 };
                if boosting {
                    v.vel += fwd * (pw / MASS * 0.0) * dt;
                }
            } else {
                // airborne: minimal air control, no tyre forces
                v.vel.x *= 1.0 - 0.05 * dt;
                v.vel.z *= 1.0 - 0.05 * dt;
                v.yaw_rate *= 1.0 - 1.0 * dt;
                v.yaw_rate += v.steer * -1.2 * dt;
            }
            v.yaw += v.yaw_rate * dt;

            // vertical -----------------------------------------------------
            let mut pos = tr.translation;
            if v.grounded {
                let e = 0.8;
                let ground_vy = {
                    let h1 = terrain::ground_or_rim(pos.x + v.vel.x * e * dt, pos.z + v.vel.z * e * dt);
                    (h1 - terrain::ground_or_rim(pos.x, pos.z)) / (e * dt).max(1e-4)
                };
                v.vel.y = ground_vy.clamp(-60.0, 80.0);
            } else {
                v.vel.y -= grav * dt;
            }
            pos += v.vel * dt;
            let (nx, nz) = (pos.x, pos.z);
            let sm2 = terrain::sample(nx, nz);
            let land2 = sm2.m > 0.015;
            let gh = if land2 { sm2.top + RIDE } else { f32::NEG_INFINITY };
            if pos.y <= gh {
                if !v.grounded && v.vel.y < -9.0 {
                    let imp = -v.vel.y;
                    shake.0 = (shake.0 + imp * 0.02).min(1.0);
                    sfx.at(Sfx::Land, pos, (imp / 25.0).min(1.0));
                    if let Some(pc) = is_player {
                        if imp > 28.0 && !cheats.god {
                            pl.s[pc.0].hp -= (imp - 28.0) * 1.2;
                            pl.s[pc.0].hurt = 0.5;
                        }
                    }
                    for _ in 0..6 {
                        fx.push(FxCmd::Dust { pos: pos - Vec3::Y * 0.4, vel: Vec3::new(((nx * 13.0).sin()) * 4.0, 3.0, ((nz * 7.0).cos()) * 4.0) });
                    }
                }
                pos.y = gh;
                v.grounded = true;
                v.air_time = 0.0;
            } else if v.grounded && pos.y > gh + 0.12 {
                v.grounded = false;
            } else if !v.grounded {
                v.air_time += dt;
            }
            tr.translation = pos;

            // prop collisions ---------------------------------------------
            if let Some((n, pen)) = chunks.collide(Vec2::new(pos.x, pos.z), 1.4) {
                tr.translation.x += n.x * pen;
                tr.translation.z += n.y * pen;
                let nv = Vec3::new(n.x, 0.0, n.y);
                let vn = v.vel.dot(nv);
                if vn < 0.0 {
                    v.vel -= nv * vn * 1.4;
                    v.vel *= 0.92;
                    if vn < -6.0 {
                        shake.0 = (shake.0 + 0.2).min(1.0);
                        sfx.at(Sfx::Crash, pos, (-vn / 20.0).min(1.0));
                        fx.push(FxCmd::Spark { pos: pos + nv * -1.0, dir: nv, n: 8 });
                        if let Some(pc) = is_player {
                            if !cheats.god {
                                pl.s[pc.0].hp -= (-vn - 6.0) * 0.5;
                                pl.s[pc.0].hurt = 0.3;
                            }
                        }
                    }
                }
            }
        }
        v.slip = slip_acc / steps as f32;

        // dust / skid fx
        let p = tr.translation;
        if v.grounded && (v.slip > 0.15 || (v.speed > 8.0 && v.throttle > 0.5)) && (time.elapsed_secs() * 60.0) as u32 % 2 == 0 {
            let back = -v.fwd();
            let side = v.right();
            for s in [-0.9f32, 0.9] {
                fx.push(FxCmd::Dust { pos: p + back * 1.6 + side * s - Vec3::Y * 0.2, vel: -v.vel * 0.1 + Vec3::Y * 1.2 });
            }
        }
        if v.in_water && v.speed > 5.0 {
            fx.push(FxCmd::Splash { pos: p });
        }
        if v.boost && v.boost_fuel > 0.02 && v.throttle >= 0.0 {
            fx.push(FxCmd::Boost { pos: p + v.fwd() * 2.2 + Vec3::Y * 0.3, dir: -v.fwd() });
        }

        // fall handling / safe pos
        let sm = terrain::sample(p.x, p.z);
        if v.grounded && sm.m > 0.4 {
            v.last_safe = p;
        }
        if p.y < sm.rim - 160.0 && !sm_land(&sm) {
            v.fall_timer += time.delta_secs();
        }
        if p.y < sm.rim - 500.0 || p.y < -1500.0 {
            tr.translation = v.last_safe + Vec3::Y * 2.0;
            v.vel = Vec3::ZERO;
            v.yaw_rate = 0.0;
            v.grounded = false;
            if let Some(pc) = is_player {
                if !cheats.god {
                    pl.s[pc.0].hp -= 15.0;
                    pl.s[pc.0].hurt = 0.6;
                }
            }
        }

        // orientation ---------------------------------------------------
        let target_up = if v.grounded { terrain::normal_at(p.x, p.z) } else { Vec3::Y };
        let k = if v.grounded { 10.0 } else { 1.2 };
        v.up = v.up.lerp(target_up, (k * dt_full).min(1.0)).normalize();
        let lat_acc = v.yaw_rate * v.speed;
        v.roll += (-(lat_acc * 0.012).clamp(-0.18, 0.18) - v.roll) * (6.0 * dt_full).min(1.0);
        tr.rotation = Quat::from_rotation_arc(Vec3::Y, v.up) * Quat::from_rotation_y(v.yaw) * Quat::from_rotation_z(v.roll);
    }
}

fn sm_land(s: &terrain::Sample) -> bool {
    s.m > 0.015
}
