//! Particles: explosions, fire, smoke, sparks, tracers' assets, dust, boost flames.
use crate::game::*;
use crate::noise::Rng;
use bevy::pbr::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;

pub fn glow(r: f32, g: f32, b: f32) -> StandardMaterial {
    StandardMaterial { base_color: Color::linear_rgb(r, g, b), unlit: true, fog_enabled: true, ..default() }
}
pub fn glow_a(r: f32, g: f32, b: f32, a: f32) -> StandardMaterial {
    StandardMaterial { base_color: Color::linear_rgba(r, g, b, a), unlit: true, alpha_mode: AlphaMode::Blend, ..default() }
}

#[derive(Resource)]
pub struct FxAssets {
    pub sphere: Handle<Mesh>,
    pub cube: Handle<Mesh>,
    pub torus: Handle<Mesh>,
    pub tracer: Handle<Mesh>,
    pub fire: [Handle<StandardMaterial>; 5],
    pub smoke: Handle<StandardMaterial>,
    pub dust: Handle<StandardMaterial>,
    pub spark: Handle<StandardMaterial>,
    pub boost: Handle<StandardMaterial>,
    pub ring: Handle<StandardMaterial>,
    pub debris: Handle<StandardMaterial>,
    pub water: Handle<StandardMaterial>,
    pub pick: Handle<StandardMaterial>,
    pub tr_cyan: Handle<StandardMaterial>,
    pub tr_mag: Handle<StandardMaterial>,
    pub tr_yellow: Handle<StandardMaterial>,
    pub tr_orange: Handle<StandardMaterial>,
    pub tr_red: Handle<StandardMaterial>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PKind {
    Fire,
    Smoke,
    Spark,
    Dust,
    Boost,
    Ring,
    Debris,
    Water,
    Pick,
    Muzzle,
}

#[derive(Component)]
pub struct Particle {
    pub vel: Vec3,
    pub life: f32,
    pub max: f32,
    pub s0: f32,
    pub s1: f32,
    pub grav: f32,
    pub drag: f32,
    pub kind: PKind,
    pub spin: Vec3,
}

#[derive(Component)]
pub struct Flash {
    pub life: f32,
    pub max: f32,
    pub peak: f32,
}

pub struct FxPlugin;
impl Plugin for FxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FxQueue>()
            .init_resource::<Shake>()
            .add_systems(Startup, setup_fx)
            .add_systems(Update, (spawn_fx, update_particles, update_flashes));
    }
}

fn setup_fx(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    let mut m = |s: StandardMaterial| mats.add(s);
    let fx = FxAssets {
        sphere: meshes.add(Sphere::new(0.5).mesh().ico(1).unwrap()),
        cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        torus: meshes.add(Torus::new(0.46, 0.5)),
        tracer: meshes.add(Cuboid::new(0.07, 0.07, 1.0)),
        fire: [m(glow(6.0, 5.0, 3.5)), m(glow(5.0, 3.0, 0.4)), m(glow(3.5, 1.0, 0.1)), m(glow(1.6, 0.2, 0.05)), m(glow_a(0.1, 0.03, 0.12, 0.7))],
        smoke: m(glow_a(0.09, 0.04, 0.14, 0.55)),
        dust: m(glow_a(0.55, 0.22, 0.45, 0.28)),
        spark: m(glow(8.0, 5.0, 1.5)),
        boost: m(glow(1.5, 4.0, 6.0)),
        ring: m(glow_a(3.0, 1.4, 0.4, 0.5)),
        debris: m(glow(0.12, 0.05, 0.2)),
        water: m(glow_a(0.3, 0.8, 1.2, 0.5)),
        pick: m(glow(2.0, 5.0, 3.0)),
        tr_cyan: m(glow(0.5, 5.0, 8.0)),
        tr_mag: m(glow(8.0, 0.8, 5.0)),
        tr_yellow: m(glow(9.0, 8.0, 2.0)),
        tr_orange: m(glow(9.0, 3.0, 0.5)),
        tr_red: m(glow(9.0, 0.4, 0.3)),
    };
    commands.insert_resource(fx);
}

fn rnd_dir(r: &mut Rng) -> Vec3 {
    loop {
        let v = Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0));
        let l = v.length();
        if l > 0.1 && l <= 1.0 {
            return v / l;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_fx(
    mut commands: Commands,
    mut q: ResMut<FxQueue>,
    fx: Res<FxAssets>,
    time: Res<Time>,
    parts: Query<(), With<Particle>>,
    flashes: Query<(), With<Flash>>,
    mut shake: ResMut<Shake>,
    mut seed: Local<u64>,
) {
    *seed = seed.wrapping_add(1);
    let mut r = Rng::new(*seed ^ (time.elapsed_secs_f64().to_bits()));
    let mut count = parts.iter().count();
    let mut nflash = flashes.iter().count();
    let cmds: Vec<FxCmd> = q.0.drain(..).collect();
    let mut mk = |commands: &mut Commands, pos: Vec3, p: Particle, mat: Handle<StandardMaterial>, mesh: Handle<Mesh>| {
        if count > 2500 {
            return;
        }
        count += 1;
        let s = p.s0;
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform::from_translation(pos).with_scale(Vec3::splat(s)),
            p,
            NotShadowCaster,
            NotShadowReceiver,
        ));
    };
    let base = |kind, vel, life, s0, s1, grav, drag| Particle { vel, life, max: life, s0, s1, grav, drag, kind, spin: Vec3::ZERO };
    for c in cmds {
        match c {
            FxCmd::Explosion { pos, size } => {
                shake.0 = (shake.0 + 0.5 * size).min(1.0);
                if nflash < 6 {
                    nflash += 1;
                    commands.spawn((
                        PointLight { color: Color::srgb(1.0, 0.55, 0.2), intensity: 9_000_000.0 * size, range: 60.0 * size, shadows_enabled: false, ..default() },
                        Transform::from_translation(pos + Vec3::Y),
                        Flash { life: 0.45, max: 0.45, peak: 9_000_000.0 * size },
                    ));
                }
                for _ in 0..(22.0 * size) as usize {
                    let d = rnd_dir(&mut r);
                    let sp = r.range(3.0, 13.0) * size;
                    mk(&mut commands, pos + d * 0.5, base(PKind::Fire, d * sp + Vec3::Y * 2.0, r.range(0.5, 1.1), r.range(1.6, 3.4) * size, 0.2, -1.5, 1.5), fx.fire[0].clone(), fx.sphere.clone());
                }
                for _ in 0..(26.0 * size) as usize {
                    let d = rnd_dir(&mut r);
                    let sp = r.range(14.0, 40.0) * size;
                    mk(&mut commands, pos, base(PKind::Spark, d * sp + Vec3::Y * 6.0, r.range(0.4, 1.0), 0.22, 0.05, 28.0, 0.4), fx.spark.clone(), fx.sphere.clone());
                }
                for _ in 0..(10.0 * size) as usize {
                    let d = rnd_dir(&mut r);
                    mk(&mut commands, pos + d, base(PKind::Smoke, d * r.range(1.0, 5.0) + Vec3::Y * 4.0, r.range(1.6, 3.2), r.range(1.5, 3.0) * size, r.range(4.0, 7.0) * size, -2.5, 0.7), fx.smoke.clone(), fx.sphere.clone());
                }
                for _ in 0..(8.0 * size) as usize {
                    let d = rnd_dir(&mut r);
                    let mut p = base(PKind::Debris, d * r.range(8.0, 22.0) * size + Vec3::Y * 8.0, r.range(1.2, 2.2), r.range(0.2, 0.55), 0.1, 26.0, 0.1);
                    p.spin = rnd_dir(&mut r) * 9.0;
                    mk(&mut commands, pos, p, fx.debris.clone(), fx.cube.clone());
                }
                mk(&mut commands, pos + Vec3::Y * 0.3, base(PKind::Ring, Vec3::ZERO, 0.45, 1.0, 22.0 * size, 0.0, 0.0), fx.ring.clone(), fx.torus.clone());
            }
            FxCmd::Spark { pos, dir, n } => {
                for _ in 0..n {
                    let d = (dir + rnd_dir(&mut r) * 0.7).normalize_or_zero();
                    mk(&mut commands, pos, base(PKind::Spark, d * r.range(6.0, 18.0), r.range(0.15, 0.5), 0.14, 0.03, 24.0, 0.3), fx.spark.clone(), fx.sphere.clone());
                }
            }
            FxCmd::Dust { pos, vel } => {
                mk(&mut commands, pos, base(PKind::Dust, vel + rnd_dir(&mut r) * 1.5, r.range(0.6, 1.1), 0.7, r.range(2.5, 4.0), -0.6, 1.0), fx.dust.clone(), fx.sphere.clone());
            }
            FxCmd::Muzzle { pos, dir, col } => {
                let m = match col {
                    0 => fx.tr_cyan.clone(),
                    1 => fx.tr_mag.clone(),
                    2 => fx.tr_yellow.clone(),
                    3 => fx.tr_orange.clone(),
                    _ => fx.tr_red.clone(),
                };
                mk(&mut commands, pos, base(PKind::Muzzle, dir * 3.0, 0.07, 0.55, 0.1, 0.0, 0.0), m, fx.sphere.clone());
            }
            FxCmd::Fire { pos } => {
                mk(&mut commands, pos + rnd_dir(&mut r) * 0.4, base(PKind::Fire, Vec3::new(r.range(-1.0, 1.0), r.range(2.5, 5.0), r.range(-1.0, 1.0)), r.range(0.5, 1.0), r.range(0.8, 1.5), 0.1, -1.0, 0.5), fx.fire[0].clone(), fx.sphere.clone());
                if r.f() < 0.4 {
                    mk(&mut commands, pos + Vec3::Y * 1.0, base(PKind::Smoke, Vec3::new(r.range(-1.0, 1.0), r.range(2.5, 4.5), r.range(-1.0, 1.0)), r.range(1.4, 2.4), 1.0, r.range(3.0, 4.5), -1.0, 0.4), fx.smoke.clone(), fx.sphere.clone());
                }
            }
            FxCmd::Smoke { pos } => {
                mk(&mut commands, pos, base(PKind::Smoke, Vec3::new(0.0, 2.0, 0.0), 1.6, 0.6, 2.8, -1.0, 0.4), fx.smoke.clone(), fx.sphere.clone());
            }
            FxCmd::Splash { pos } => {
                mk(&mut commands, pos, base(PKind::Water, Vec3::new(r.range(-3.0, 3.0), r.range(3.0, 7.0), r.range(-3.0, 3.0)), r.range(0.4, 0.8), 0.45, 0.1, 22.0, 0.2), fx.water.clone(), fx.sphere.clone());
            }
            FxCmd::Boost { pos, dir } => {
                mk(&mut commands, pos, base(PKind::Boost, dir * r.range(6.0, 14.0) + rnd_dir(&mut r) * 1.0, 0.28, 0.7, 0.1, 0.0, 0.5), fx.boost.clone(), fx.sphere.clone());
            }
            FxCmd::Pickup { pos } => {
                for _ in 0..14 {
                    let d = rnd_dir(&mut r);
                    mk(&mut commands, pos, base(PKind::Pick, d * r.range(4.0, 9.0), 0.5, 0.25, 0.02, 0.0, 2.0), fx.pick.clone(), fx.sphere.clone());
                }
            }
        }
    }
}

fn update_particles(
    mut commands: Commands,
    time: Res<Time>,
    fx: Res<FxAssets>,
    mut q: Query<(Entity, &mut Transform, &mut Particle, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    let dt = time.delta_secs();
    for (e, mut tr, mut p, mut mat) in &mut q {
        p.life -= dt;
        if p.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        let t = 1.0 - p.life / p.max; // 0 -> 1
        let drag = (1.0 - p.drag * dt).max(0.0);
        p.vel *= drag;
        p.vel.y -= p.grav * dt;
        tr.translation += p.vel * dt;
        let mut s = p.s0 + (p.s1 - p.s0) * t;
        match p.kind {
            PKind::Fire => {
                let idx = ((t * 4.2) as usize).min(4);
                if mat.0 != fx.fire[idx] {
                    mat.0 = fx.fire[idx].clone();
                }
                s *= 1.0 - t * 0.4;
            }
            PKind::Smoke | PKind::Dust => {
                if t > 0.7 {
                    s *= 1.0 - (t - 0.7) / 0.3;
                }
            }
            PKind::Debris => {
                let sp = p.spin;
                tr.rotate(Quat::from_euler(EulerRot::XYZ, sp.x * dt, sp.y * dt, sp.z * dt));
                if tr.translation.y < -1000.0 {
                    p.life = 0.0;
                }
            }
            PKind::Ring => {
                tr.scale = Vec3::new(s, s * 0.25, s);
                continue;
            }
            _ => {}
        }
        tr.scale = Vec3::splat(s.max(0.001));
    }
}

fn update_flashes(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut Flash, &mut PointLight)>) {
    for (e, mut f, mut l) in &mut q {
        f.life -= time.delta_secs();
        if f.life <= 0.0 {
            commands.entity(e).despawn();
        } else {
            l.intensity = f.peak * (f.life / f.max).powi(2);
        }
    }
}
