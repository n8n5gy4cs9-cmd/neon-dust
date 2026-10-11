//! Enemies (Glitcher drones, Sentries, Pursuit cars, boss Warden), director/spawner, pickups.
use crate::camera::lerp_angle;
use crate::fx::FxAssets;
use crate::game::*;
use crate::meshgen::*;
use crate::noise::Rng;
use crate::player::Walker;
use crate::terrain;
use crate::vehicle::*;
use crate::weapons::{hurt_player, spawn_bullet};
use crate::world::{Focus, WorldMats};
use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EnemyKind {
    Drone,
    Sentry,
    Pursuit,
    Boss,
}

#[derive(Component)]
pub struct Enemy {
    pub kind: EnemyKind,
    pub hp: f32,
    pub max: f32,
    pub radius: f32,
    pub cd: f32,
    pub burst: u32,
    pub flash: f32,
    pub phase: f32,
    pub vel: Vec3,
    pub aux: f32,
}

#[derive(Component)]
pub struct Pickup {
    pub kind: u8, // 0 health 1 ammo 2 boost 3 weapon
    pub t: f32,
}

/// Where the player is (car or foot) and how fast.
#[derive(Resource, Default)]
pub struct Tgt {
    pub pos: Vec3,
    pub vel: Vec3,
}

#[derive(Resource)]
pub struct Director {
    pub active: bool,
    pub max_drones: usize,
    pub max_cars: usize,
    pub interval: f32,
    pub timer: f32,
    pub boss_support: bool,
}
impl Default for Director {
    fn default() -> Self {
        Director { active: false, max_drones: 0, max_cars: 0, interval: 4.0, timer: 2.0, boss_support: false }
    }
}

fn drone_mesh(core: [f32; 3], eye: [f32; 3]) -> Mesh {
    let mut m = MB::new();
    let body = col(core[0], core[1], core[2], 0.15);
    let g = col(eye[0], eye[1], eye[2], 1.0);
    m.cone(Vec3::ZERO, 0.75, 0.7, 6, body, false);
    m.with_xf(Mat4::from_rotation_x(std::f32::consts::PI), |m| m.cone(Vec3::ZERO, 0.75, 0.6, 6, body, false));
    m.cuboid(Vec3::new(0.0, 0.0, -0.7), Vec3::new(0.22, 0.12, 0.06), g);
    for (x, z) in [(-1.0f32, 0.0f32), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
        m.cuboid(Vec3::new(x * 0.9, 0.05, z * 0.9), Vec3::new(0.06 + x.abs() * 0.5, 0.04, 0.06 + z.abs() * 0.5), col(0.05, 0.03, 0.1, 0.0));
        m.frustum(Vec3::new(x * 1.45, 0.08, z * 1.45), 0.5, 0.5, 0.04, 10, g, true);
    }
    m.build()
}

fn sentry_mesh() -> Mesh {
    let mut m = MB::new();
    let dark = col(0.06, 0.03, 0.12, 0.0);
    let g = col(1.0, 0.1, 0.2, 1.0);
    m.frustum(Vec3::ZERO, 1.2, 0.8, 1.3, 8, dark, true);
    m.frustum(Vec3::new(0.0, 0.3, 0.0), 1.05, 1.05, 0.08, 8, g, false);
    m.cuboid(Vec3::new(0.0, 1.8, 0.0), Vec3::new(0.55, 0.4, 0.6), dark);
    m.cuboid(Vec3::new(-0.25, 1.8, -1.0), Vec3::new(0.07, 0.07, 0.6), col(0.1, 0.1, 0.15, 0.0));
    m.cuboid(Vec3::new(0.25, 1.8, -1.0), Vec3::new(0.07, 0.07, 0.6), col(0.1, 0.1, 0.15, 0.0));
    m.cuboid(Vec3::new(0.0, 1.95, -0.62), Vec3::new(0.3, 0.07, 0.03), g);
    m.cone(Vec3::new(0.0, 2.2, 0.0), 0.35, 0.5, 4, g, false);
    m.build()
}

pub fn spawn_enemy(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &WorldMats,
    kind: EnemyKind,
    pos: Vec3,
) -> Entity {
    match kind {
        EnemyKind::Drone => {
            let mesh = meshes.add(drone_mesh([0.25, 0.04, 0.3], [1.0, 0.15, 0.25]));
            commands
                .spawn((
                    Enemy { kind, hp: 30.0, max: 30.0, radius: 1.5, cd: 1.0, burst: 0, flash: 0.0, phase: pos.x, vel: Vec3::ZERO, aux: 0.0 },
                    Transform::from_translation(pos),
                    Visibility::default(),
                    Mesh3d(mesh),
                    MeshMaterial3d(mats.solid.clone()),
                ))
                .id()
        }
        EnemyKind::Boss => {
            let mesh = meshes.add(drone_mesh([0.35, 0.02, 0.05], [1.0, 0.6, 0.1]));
            commands
                .spawn((
                    Enemy { kind, hp: 1500.0, max: 1500.0, radius: 7.0, cd: 3.0, burst: 0, flash: 0.0, phase: 0.0, vel: Vec3::ZERO, aux: 0.0 },
                    Transform::from_translation(pos).with_scale(Vec3::splat(5.0)),
                    Visibility::default(),
                    Mesh3d(mesh),
                    MeshMaterial3d(mats.solid.clone()),
                    PointLight { color: Color::srgb(1.0, 0.3, 0.1), intensity: 4_000_000.0, range: 60.0, ..default() },
                ))
                .id()
        }
        EnemyKind::Sentry => {
            let mesh = meshes.add(sentry_mesh());
            commands
                .spawn((
                    Enemy { kind, hp: 110.0, max: 110.0, radius: 1.8, cd: 1.0, burst: 0, flash: 0.0, phase: 0.0, vel: Vec3::ZERO, aux: 0.0 },
                    Transform::from_translation(pos),
                    Visibility::default(),
                    Mesh3d(mesh),
                    MeshMaterial3d(mats.solid.clone()),
                ))
                .id()
        }
        EnemyKind::Pursuit => {
            let e = spawn_car(commands, meshes, mats, pos, 0.0, CarKind::Pursuit);
            commands.entity(e).insert(Enemy { kind, hp: 130.0, max: 130.0, radius: 2.4, cd: 0.0, burst: 0, flash: 0.0, phase: 0.0, vel: Vec3::ZERO, aux: 0.0 });
            e
        }
    }
}

pub fn spawn_pickup(commands: &mut Commands, meshes: &mut Assets<Mesh>, mats: &WorldMats, pos: Vec3, kind: u8) {
    let c = match kind {
        0 => [0.1, 1.0, 0.3],
        1 => [1.0, 0.85, 0.1],
        2 => [0.1, 0.8, 1.0],
        _ => [1.0, 0.2, 0.9],
    };
    let mut m = MB::new();
    let g = col(c[0], c[1], c[2], 1.0);
    m.cone(Vec3::ZERO, 0.5, 0.6, 4, g, false);
    m.with_xf(Mat4::from_rotation_x(std::f32::consts::PI), |m| m.cone(Vec3::ZERO, 0.5, 0.6, 4, g, false));
    if kind == 3 {
        m.cuboid(Vec3::ZERO, Vec3::new(0.35, 0.35, 0.35), g);
    }
    commands.spawn((
        Pickup { kind, t: pos.x },
        Mesh3d(meshes.add(m.build())),
        MeshMaterial3d(mats.solid.clone()),
        Transform::from_translation(pos + Vec3::Y * 1.2),
        PointLight { color: Color::srgb(c[0], c[1], c[2]), intensity: 150_000.0, range: 10.0, ..default() },
    ));
}

pub struct EnemiesPlugin;
impl Plugin for EnemiesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Tgt>().init_resource::<Director>().add_systems(
            Update,
            (track_player, director, enemy_ai, pursuit_ai, enemy_death, pickups)
                .chain()
                .run_if(in_state(AppState::Playing))
                .after(VehicleSet),
        );
    }
}

fn track_player(
    ctl: Res<Control>,
    mut tgt: ResMut<Tgt>,
    car: Query<(&Transform, &Vehicle), With<PlayerCar>>,
    pl: Query<(&Transform, &Walker), (With<Player>, Without<PlayerCar>)>,
) {
    if ctl.in_car {
        if let Ok((t, v)) = car.single() {
            tgt.pos = t.translation + Vec3::Y * 0.8;
            tgt.vel = v.vel;
        }
    } else if let Ok((t, w)) = pl.single() {
        tgt.pos = t.translation + Vec3::Y * 1.0;
        tgt.vel = w.vel;
    }
}

#[allow(clippy::too_many_arguments)]
fn director(
    mut commands: Commands,
    time: Res<Time>,
    mut dir: ResMut<Director>,
    tgt: Res<Tgt>,
    cheats: Res<Cheats>,
    mut meshes: ResMut<Assets<Mesh>>,
    mats: Res<WorldMats>,
    enemies: Query<&Enemy>,
    mut seed: Local<u64>,
) {
    if !dir.active || cheats.peaceful {
        return;
    }
    dir.timer -= time.delta_secs();
    if dir.timer > 0.0 {
        return;
    }
    dir.timer = dir.interval;
    *seed = seed.wrapping_add(0x1234_5677);
    let mut rng = Rng::new(*seed ^ time.elapsed_secs_f64().to_bits());
    let drones = enemies.iter().filter(|e| e.kind == EnemyKind::Drone).count();
    let cars = enemies.iter().filter(|e| e.kind == EnemyKind::Pursuit).count();
    if drones < dir.max_drones {
        let a = rng.range(0.0, 6.283);
        let d = rng.range(80.0, 130.0);
        let (x, z) = (tgt.pos.x + a.cos() * d, tgt.pos.z + a.sin() * d);
        let y = terrain::ground_or_rim(x, z).max(tgt.pos.y - 10.0) + rng.range(8.0, 20.0);
        spawn_enemy(&mut commands, &mut meshes, &mats, EnemyKind::Drone, Vec3::new(x, y, z));
    }
    if cars < dir.max_cars {
        for _ in 0..14 {
            let a = rng.range(0.0, 6.283);
            let d = rng.range(140.0, 230.0);
            let (x, z) = (tgt.pos.x + a.cos() * d, tgt.pos.z + a.sin() * d);
            let s = terrain::sample(x, z);
            if s.m > 0.5 && s.river < 0.05 && s.relief > -1.0 {
                spawn_enemy(&mut commands, &mut meshes, &mats, EnemyKind::Pursuit, Vec3::new(x, s.top + 1.0, z));
                break;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn enemy_ai(
    mut commands: Commands,
    time: Res<Time>,
    tgt: Res<Tgt>,
    cheats: Res<Cheats>,
    fxa: Res<FxAssets>,
    mut q: Query<(&mut Transform, &mut Enemy), Without<Vehicle>>,
    mut seed: Local<u64>,
) {
    let dt = time.delta_secs().min(0.05);
    let t = time.elapsed_secs();
    *seed = seed.wrapping_add(0xABCD);
    let mut rng = Rng::new(*seed ^ time.elapsed_secs_f64().to_bits());
    for (mut tr, mut en) in &mut q {
        en.flash = (en.flash - dt).max(0.0);
        let to = tgt.pos - tr.translation;
        let dist = to.length();
        let yaw_to = (-to.x).atan2(-to.z);
        match en.kind {
            EnemyKind::Drone | EnemyKind::Boss => {
                let boss = en.kind == EnemyKind::Boss;
                en.phase += dt * if boss { 0.35 } else { 0.7 };
                let r = if boss { 55.0 } else { 28.0 };
                let want_xz = Vec3::new(tgt.pos.x + en.phase.cos() * r, 0.0, tgt.pos.z + en.phase.sin() * r);
                let h = terrain::ground_or_rim(want_xz.x, want_xz.z) + if boss { 28.0 } else { 11.0 } + (t * 2.0 + en.aux).sin() * 2.0;
                let want = Vec3::new(want_xz.x, h.max(tgt.pos.y + 4.0), want_xz.z) + tgt.vel * 0.4;
                let acc = (want - tr.translation).clamp_length_max(1.0) * if boss { 22.0 } else { 40.0 };
                en.vel += acc * dt;
                en.vel *= 1.0 - 1.3 * dt;
                let maxs = if boss { 22.0 } else { 30.0 };
                en.vel = en.vel.clamp_length_max(maxs);
                tr.translation += en.vel * dt;
                let base_scale = if boss { 5.0 } else { 1.0 };
                tr.rotation = Quat::from_rotation_y(yaw_to) * Quat::from_rotation_z((t * 3.0 + en.aux).sin() * 0.1);
                tr.scale = Vec3::splat(base_scale);
                if !cheats.peaceful && dist < if boss { 220.0 } else { 100.0 } {
                    en.cd -= dt;
                    if en.cd <= 0.0 {
                        let lead = tgt.pos + tgt.vel * (dist / 95.0) * 0.5;
                        let spread = if boss { 0.06 } else { 0.035 };
                        let mut d = (lead - tr.translation).normalize_or_zero();
                        d += Vec3::new(rng.range(-spread, spread), rng.range(-spread, spread), rng.range(-spread, spread));
                        let d = d.normalize();
                        if boss {
                            for k in -2..=2 {
                                let dd = (d + Vec3::new(k as f32 * 0.05, 0.0, k as f32 * 0.05)).normalize();
                                spawn_bullet(&mut commands, &fxa, tr.translation + dd * 6.0, dd, 85.0, 8.0, false, 4, 0.0, 4.0);
                            }
                            en.aux += 1.0;
                            if en.aux as i32 % 3 == 0 {
                                spawn_bullet(&mut commands, &fxa, tr.translation + d * 6.0, d, 70.0, 30.0, false, 3, 8.0, 5.0);
                            }
                            en.cd = 1.6;
                        } else {
                            spawn_bullet(&mut commands, &fxa, tr.translation + d * 1.5, d, 95.0, 4.5, false, 4, 0.0, 2.5);
                            if en.burst == 0 {
                                en.burst = 2;
                                en.cd = 0.14;
                            } else {
                                en.burst -= 1;
                                en.cd = if en.burst == 0 { 2.2 } else { 0.14 };
                            }
                        }
                    }
                }
            }
            EnemyKind::Sentry => {
                tr.rotation = Quat::from_rotation_y(lerp_angle(tr.rotation.to_euler(EulerRot::YXZ).0, yaw_to, 1.0 - (-6.0 * dt).exp()));
                if !cheats.peaceful && dist < 120.0 {
                    en.cd -= dt;
                    if en.cd <= 0.0 {
                        let o = tr.translation + Vec3::Y * 1.8;
                        let lead = tgt.pos + tgt.vel * (dist / 105.0) * 0.5;
                        let mut d = (lead - o).normalize_or_zero();
                        d += Vec3::new(rng.range(-0.03, 0.03), rng.range(-0.03, 0.03), rng.range(-0.03, 0.03));
                        let d = d.normalize();
                        spawn_bullet(&mut commands, &fxa, o + d * 1.5, d, 105.0, 6.0, false, 4, 0.0, 2.5);
                        if en.burst == 0 {
                            en.burst = 3;
                            en.cd = 0.12;
                        } else {
                            en.burst -= 1;
                            en.cd = if en.burst == 0 { 1.6 } else { 0.12 };
                        }
                    }
                }
            }
            EnemyKind::Pursuit => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn pursuit_ai(
    time: Res<Time>,
    tgt: Res<Tgt>,
    cheats: Res<Cheats>,
    ctl: Res<Control>,
    mut stats: ResMut<Stats>,
    mut sfx: ResMut<SfxQueue>,
    mut fx: ResMut<FxQueue>,
    mut shake: ResMut<Shake>,
    mut q: Query<(&mut Transform, &mut Vehicle, &mut Enemy), Without<PlayerCar>>,
    mut pcar: Query<(&mut Transform, &mut Vehicle), With<PlayerCar>>,
) {
    let dt = time.delta_secs().min(0.05);
    for (mut tr, mut v, mut en) in &mut q {
        en.flash = (en.flash - dt).max(0.0);
        let to = Vec3::new(tgt.pos.x - tr.translation.x, 0.0, tgt.pos.z - tr.translation.z);
        let dist = to.length();
        let want_yaw = (-to.x).atan2(-to.z);
        let mut diff = want_yaw - v.yaw;
        diff = (diff + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        // avoid voids ahead
        let ahead = tr.translation + v.fwd() * 28.0;
        let void_ahead = terrain::ground(ahead.x, ahead.z).is_none();
        let mut steer = (-diff * 1.6).clamp(-1.0, 1.0);
        if void_ahead {
            steer = if diff > 0.0 { -1.0 } else { 1.0 };
        }
        v.steer += (steer - v.steer) * (8.0 * dt).min(1.0);
        v.throttle = if void_ahead { 0.2 } else if cheats.peaceful { 0.0 } else { 1.0 };
        v.boost = dist > 140.0 && !void_ahead;
        v.handbrake = diff.abs() > 1.7 && v.speed > 15.0 || cheats.peaceful;
        // ram player's car
        if let Ok((pt, mut pv)) = pcar.single_mut() {
            let d = pt.translation - tr.translation;
            if d.length() < 3.6 && ctl.in_car || (d.length() < 3.2 && !ctl.in_car) {
                let n = Vec3::new(d.x, 0.0, d.z).normalize_or_zero();
                let rel = (v.vel - pv.vel).dot(n);
                if rel > 3.0 {
                    let dmg = rel * 0.6;
                    en.hp -= dmg * 1.4;
                    en.flash = 0.2;
                    pv.vel += n * rel * 0.5;
                    v.vel -= n * rel * 0.5;
                    if ctl.in_car {
                        hurt_player(&mut stats, &cheats, dmg, true, &mut sfx);
                    } else {
                        pv.vel += n * rel * 0.2;
                    }
                    sfx.at(Sfx::Crash, tr.translation, 1.0);
                    shake.0 = (shake.0 + 0.3).min(1.0);
                    fx.push(FxCmd::Spark { pos: tr.translation + n * 1.8, dir: n, n: 12 });
                }
            }
        }
        // on foot: ram damage
        if !ctl.in_car && dist < 3.0 && v.speed > 8.0 && !cheats.peaceful {
            hurt_player(&mut stats, &cheats, 40.0 * dt * 3.0, false, &mut sfx);
        }
        let _ = &mut tr;
    }
}

#[allow(clippy::too_many_arguments)]
fn enemy_death(
    mut commands: Commands,
    time: Res<Time>,
    tgt: Res<Tgt>,
    mut stats: ResMut<Stats>,
    mut fx: ResMut<FxQueue>,
    mut sfx: ResMut<SfxQueue>,
    mut toasts: ResMut<Toasts>,
    mut meshes: ResMut<Assets<Mesh>>,
    mats: Res<WorldMats>,
    q: Query<(Entity, &Transform, &Enemy)>,
    mut seed: Local<u64>,
) {
    *seed = seed.wrapping_add(0x77);
    let mut rng = Rng::new(*seed ^ time.elapsed_secs_f64().to_bits());
    for (e, tr, en) in &q {
        let p = tr.translation;
        if en.hp <= 0.0 {
            let (size, score, drop) = match en.kind {
                EnemyKind::Drone => (0.9, 100, 0.28),
                EnemyKind::Sentry => (1.2, 250, 0.6),
                EnemyKind::Pursuit => (1.5, 300, 0.7),
                EnemyKind::Boss => (3.5, 5000, 1.0),
            };
            fx.push(FxCmd::Explosion { pos: p + Vec3::Y, size });
            sfx.at(Sfx::Explosion, p, 1.0);
            stats.score += score;
            match en.kind {
                EnemyKind::Drone => stats.kills += 1,
                EnemyKind::Sentry => {
                    stats.kills += 1;
                    stats.sentries += 1
                }
                EnemyKind::Pursuit => stats.cars_destroyed += 1,
                EnemyKind::Boss => {
                    stats.boss_dead = true;
                    toasts.say("WARDEN DESTROYED");
                }
            }
            if rng.f() < drop {
                let k = if rng.f() < 0.5 { 0 } else { 1 };
                spawn_pickup(&mut commands, &mut meshes, &mats, p.with_y(terrain::ground_or_rim(p.x, p.z)), k);
            }
            commands.entity(e).despawn();
            continue;
        }
        if en.hp < en.max * 0.35 && en.kind != EnemyKind::Sentry {
            fx.push(FxCmd::Fire { pos: p + Vec3::Y * 0.5 });
        }
        if en.kind != EnemyKind::Sentry && en.kind != EnemyKind::Boss && p.distance(tgt.pos) > 700.0 {
            commands.entity(e).despawn();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn pickups(
    mut commands: Commands,
    time: Res<Time>,
    tgt: Res<Tgt>,
    mut stats: ResMut<Stats>,
    mut fx: ResMut<FxQueue>,
    mut sfx: ResMut<SfxQueue>,
    mut toasts: ResMut<Toasts>,
    mut car: Query<&mut Vehicle, With<PlayerCar>>,
    mut q: Query<(Entity, &mut Transform, &Pickup)>,
) {
    let t = time.elapsed_secs();
    for (e, mut tr, p) in &mut q {
        tr.rotation = Quat::from_rotation_y(t * 2.0 + p.t);
        tr.translation.y += (t * 3.0 + p.t).sin() * 0.003;
        if tr.translation.distance(tgt.pos) < 4.5 {
            match p.kind {
                0 => {
                    stats.hp = (stats.hp + 35.0).min(stats.max_hp);
                    toasts.say("+35 INTEGRITY");
                }
                1 => {
                    stats.ammo[1] += 60;
                    if stats.owned[2] {
                        stats.ammo[2] += 6;
                    }
                    if stats.owned[3] {
                        stats.ammo[3] += 3;
                    }
                    stats.ammo[3] += 1;
                    toasts.say("AMMO CACHE");
                }
                2 => {
                    if let Ok(mut v) = car.single_mut() {
                        v.boost_fuel = 1.0;
                    }
                    toasts.say("BOOST REFILL");
                }
                _ => {
                    let next = (0..NUM_WEAPONS).find(|i| !stats.owned[*i]);
                    if let Some(i) = next {
                        stats.owned[i] = true;
                        stats.cur = i;
                        toasts.say(format!("NEW WEAPON: {}", WEAPON_NAMES[i]));
                    } else {
                        stats.ammo[1] += 80;
                    }
                }
            }
            fx.push(FxCmd::Pickup { pos: tr.translation });
            sfx.play(Sfx::Pickup);
            commands.entity(e).despawn();
        }
    }
}
