//! Player weapons (on foot + car cannons), projectiles with tracers, hit detection, explosions.
use crate::camera::AimInfo;
use crate::enemies::Enemy;
use crate::fx::FxAssets;
use crate::game::*;
use crate::noise::Rng;
use crate::player::Walker;
use crate::terrain;
use crate::vehicle::Vehicle;
use crate::world::Chunks;
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::pbr::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;

#[derive(bevy::ecs::system::SystemParam)]
struct Out<'w> {
    fx: ResMut<'w, FxQueue>,
    sfx: ResMut<'w, SfxQueue>,
    shake: ResMut<'w, Shake>,
    toasts: ResMut<'w, Toasts>,
}

#[derive(Component)]
pub struct Bullet {
    pub vel: Vec3,
    pub life: f32,
    pub dmg: f32,
    pub friendly: bool,
    pub splash: f32,
    pub rocket: bool,
    pub tick: f32,
}

pub fn hurt_player(stats: &mut Stats, cheats: &Cheats, amount: f32, in_car: bool, sfx: &mut SfxQueue) {
    if cheats.god || amount <= 0.0 {
        return;
    }
    let a = if in_car { amount * 0.6 } else { amount };
    stats.hp -= a;
    if stats.hurt <= 0.05 {
        sfx.play(Sfx::Hurt);
    }
    stats.hurt = 0.45;
    stats.combat = 8.0;
}

pub struct WeaponsPlugin;
impl Plugin for WeaponsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (fire_input, bullets).chain().run_if(in_state(AppState::Playing)).after(crate::vehicle::VehicleSet));
    }
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_bullet(
    commands: &mut Commands,
    fx: &FxAssets,
    pos: Vec3,
    dir: Vec3,
    speed: f32,
    dmg: f32,
    friendly: bool,
    kind: u8,
    splash: f32,
    life: f32,
) {
    let (mat, len) = match kind {
        0 => (fx.tr_cyan.clone(), 3.2),
        1 => (fx.tr_mag.clone(), 2.4),
        2 => (fx.tr_yellow.clone(), 1.8),
        3 => (fx.tr_orange.clone(), 1.3),
        4 => (fx.tr_red.clone(), 2.6),
        _ => (fx.tr_cyan.clone(), 4.0),
    };
    let rocket = kind == 3;
    let scale = if rocket { Vec3::new(3.5, 3.5, len) } else { Vec3::new(1.0, 1.0, len) };
    commands.spawn((
        Mesh3d(fx.tracer.clone()),
        MeshMaterial3d(mat),
        Transform::from_translation(pos).looking_to(dir, Vec3::Y).with_scale(scale),
        Bullet { vel: dir * speed, life, dmg, friendly, splash, rocket, tick: 0.0 },
        NotShadowCaster,
        NotShadowReceiver,
    ));
}

#[allow(clippy::too_many_arguments)]
fn fire_input(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    scroll: Res<AccumulatedMouseScroll>,
    ui: Res<UiBlock>,
    ctl: Res<Control>,
    cheats: Res<Cheats>,
    aim: Res<AimInfo>,
    fxa: Res<FxAssets>,
    mut stats: ResMut<Stats>,
    mut out: Out,
    mut seed: Local<u64>,
    mut side: Local<bool>,
    car: Query<(&Transform, &Vehicle), With<PlayerCar>>,
    pl: Query<(&Transform, &Walker), (With<Player>, Without<PlayerCar>)>,
) {
    let dt = time.delta_secs();
    stats.cooldown = (stats.cooldown - dt).max(0.0);
    stats.combat = (stats.combat - dt).max(0.0);
    stats.time += dt;
    if ui.0 {
        return;
    }
    *seed = seed.wrapping_add(0x9E37);
    let mut rng = Rng::new(*seed ^ time.elapsed_secs_f64().to_bits());
    // weapon switch
    for (i, k) in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4].iter().enumerate() {
        if keys.just_pressed(*k) {
            if stats.owned[i] {
                stats.cur = i;
                out.sfx.play(Sfx::Blip);
            } else {
                out.toasts.say(format!("{} not found yet", crate::game::WEAPON_NAMES[i]));
            }
        }
    }
    if scroll.delta.y != 0.0 {
        let dir = if scroll.delta.y > 0.0 { 1i32 } else { -1 };
        let mut c = stats.cur as i32;
        for _ in 0..NUM_WEAPONS {
            c = (c + dir).rem_euclid(NUM_WEAPONS as i32);
            if stats.owned[c as usize] {
                break;
            }
        }
        stats.cur = c as usize;
    }
    let lmb = mouse.pressed(MouseButton::Left);
    let rmb = mouse.pressed(MouseButton::Right);
    if stats.cooldown > 0.0 {
        return;
    }
    let spread_dir = |d: Vec3, s: f32, rng: &mut Rng| (d + Vec3::new(rng.range(-s, s), rng.range(-s, s), rng.range(-s, s))).normalize();
    if ctl.in_car {
        let Ok((ct, v)) = car.single() else { return };
        let aim_dir_from = |o: Vec3| {
            let d = (aim.point - o).normalize_or_zero();
            if d.dot(v.fwd()) < 0.2 || aim.point.distance(o) < 8.0 { v.fwd() } else { d }
        };
        if lmb {
            *side = !*side;
            let sx = if *side { 0.75 } else { -0.75 };
            let o = ct.translation + v.right() * sx + v.fwd() * 2.0 + Vec3::Y * 0.7;
            let d = spread_dir(aim_dir_from(o), 0.008, &mut rng);
            let vel_add = v.speed.max(0.0);
            spawn_bullet(&mut commands, &fxa, o, d, 230.0 + vel_add * 0.5, 9.0, true, 5, 0.0, 1.6);
            out.fx.push(FxCmd::Muzzle { pos: o + d, dir: d, col: 0 });
            out.sfx.play(Sfx::Pistol);
            stats.cooldown = 0.075;
            out.shake.0 = (out.shake.0 + 0.02).min(1.0);
            stats.combat = 6.0;
        } else if rmb && (stats.ammo[3] > 0 || cheats.infammo) {
            let o = ct.translation + v.fwd() * 2.0 + Vec3::Y * 0.9;
            let d = aim_dir_from(o);
            spawn_bullet(&mut commands, &fxa, o, d, 120.0 + v.speed, 85.0, true, 3, 9.0, 4.0);
            out.fx.push(FxCmd::Muzzle { pos: o + d, dir: d, col: 3 });
            out.sfx.play(Sfx::Rocket);
            if !cheats.infammo {
                stats.ammo[3] -= 1;
            }
            stats.cooldown = 0.9;
            out.shake.0 = (out.shake.0 + 0.15).min(1.0);
            stats.combat = 8.0;
        } else if rmb {
            out.sfx.play(Sfx::Deny);
            stats.cooldown = 0.4;
        }
    } else if lmb {
        let Ok((pt, w)) = pl.single() else { return };
        let facing = Quat::from_rotation_y(w.yaw);
        let o = pt.translation + Vec3::Y * 1.38 + facing * Vec3::new(0.36, 0.0, -0.9);
        let d0 = {
            let d = (aim.point - o).normalize_or_zero();
            if aim.point.distance(o) < 3.0 || d == Vec3::ZERO { aim.dir } else { d }
        };
        let cur = stats.cur;
        let has_ammo = cur == 0 || stats.ammo[cur] > 0 || cheats.infammo;
        if !has_ammo {
            out.sfx.play(Sfx::Deny);
            stats.cooldown = 0.3;
            out.toasts.say("OUT OF AMMO");
            stats.cur = 0;
            return;
        }
        let spend = |stats: &mut Stats| {
            if cur != 0 && !cheats.infammo {
                stats.ammo[cur] -= 1;
            }
        };
        match cur {
            0 => {
                let d = spread_dir(d0, 0.003, &mut rng);
                spawn_bullet(&mut commands, &fxa, o, d, 320.0, 18.0, true, 0, 0.0, 1.5);
                out.fx.push(FxCmd::Muzzle { pos: o + d * 0.4, dir: d, col: 0 });
                out.sfx.play(Sfx::Pistol);
                stats.cooldown = 0.2;
                out.shake.0 = (out.shake.0 + 0.03).min(1.0);
            }
            1 => {
                let d = spread_dir(d0, 0.02, &mut rng);
                spawn_bullet(&mut commands, &fxa, o, d, 280.0, 8.0, true, 1, 0.0, 1.3);
                out.fx.push(FxCmd::Muzzle { pos: o + d * 0.4, dir: d, col: 1 });
                out.sfx.play(Sfx::Smg);
                spend(&mut stats);
                stats.cooldown = 0.065;
                out.shake.0 = (out.shake.0 + 0.02).min(1.0);
            }
            2 => {
                for _ in 0..9 {
                    let d = spread_dir(d0, 0.06, &mut rng);
                    spawn_bullet(&mut commands, &fxa, o, d, 260.0, 11.0, true, 2, 0.0, 0.7);
                }
                out.fx.push(FxCmd::Muzzle { pos: o + d0 * 0.5, dir: d0, col: 2 });
                out.sfx.play(Sfx::Shotgun);
                spend(&mut stats);
                stats.cooldown = 0.85;
                out.shake.0 = (out.shake.0 + 0.2).min(1.0);
            }
            _ => {
                spawn_bullet(&mut commands, &fxa, o, d0, 115.0, 90.0, true, 3, 9.0, 4.0);
                out.fx.push(FxCmd::Muzzle { pos: o + d0 * 0.5, dir: d0, col: 3 });
                out.sfx.play(Sfx::Rocket);
                spend(&mut stats);
                stats.cooldown = 1.05;
                out.shake.0 = (out.shake.0 + 0.2).min(1.0);
            }
        }
        stats.combat = 8.0;
    }
}

fn seg_sphere(p0: Vec3, p1: Vec3, c: Vec3, r: f32) -> bool {
    let d = p1 - p0;
    let l2 = d.length_squared();
    let t = if l2 < 1e-6 { 0.0 } else { ((c - p0).dot(d) / l2).clamp(0.0, 1.0) };
    (p0 + d * t).distance(c) <= r
}

#[allow(clippy::too_many_arguments)]
fn bullets(
    mut commands: Commands,
    time: Res<Time>,
    chunks: Res<Chunks>,
    cheats: Res<Cheats>,
    ctl: Res<Control>,
    mut stats: ResMut<Stats>,
    mut fx: ResMut<FxQueue>,
    mut sfx: ResMut<SfxQueue>,
    mut bq: Query<(Entity, &mut Transform, &mut Bullet)>,
    mut enemies: Query<(&Transform, &mut Enemy), Without<Bullet>>,
    focus: Res<crate::world::Focus>,
) {
    let dt = time.delta_secs().min(0.05);
    let mut booms: Vec<(Vec3, f32, f32, bool)> = vec![];
    let target = focus.0;
    let player_r = if ctl.in_car { 2.0 } else { 1.0 };
    for (e, mut tr, mut b) in &mut bq {
        b.life -= dt;
        if b.life <= 0.0 {
            if b.rocket {
                booms.push((tr.translation, b.splash, b.dmg, b.friendly));
            }
            commands.entity(e).despawn();
            continue;
        }
        let p0 = tr.translation;
        let steps = 3;
        let mut p = p0;
        let mut hit: Option<(Vec3, Option<Entity>)> = None;
        'march: for i in 0..steps {
            let np = p0 + b.vel * dt * ((i + 1) as f32 / steps as f32);
            if b.friendly {
                for (et, en) in enemies.iter() {
                    let c = et.translation + Vec3::Y * (en.radius * 0.5);
                    if seg_sphere(p, np, c, en.radius) {
                        hit = Some((np, None));
                        break 'march;
                    }
                }
            } else if seg_sphere(p, np, target, player_r) {
                hit = Some((np, None));
                break 'march;
            }
            if let Some(g) = terrain::ground(np.x, np.z) {
                if np.y < g {
                    hit = Some((np, None));
                    break 'march;
                }
            }
            if np.y < terrain::rim_y(np.x, np.z) - 400.0 {
                b.life = 0.0;
            }
            if let Some((_, _)) = chunks.collide(Vec2::new(np.x, np.z), 0.3) {
                if let Some(g) = terrain::ground(np.x, np.z) {
                    if np.y < g + 10.0 {
                        hit = Some((np, None));
                        break 'march;
                    }
                }
            }
            p = np;
        }
        if let Some((hp, _)) = hit {
            fx.push(FxCmd::Spark { pos: hp, dir: -b.vel.normalize_or_zero(), n: 6 });
            if b.splash > 0.0 {
                booms.push((hp, b.splash, b.dmg, b.friendly));
            } else if b.friendly {
                // direct hit damage
                for (et, mut en) in enemies.iter_mut() {
                    let c = et.translation + Vec3::Y * (en.radius * 0.5);
                    if c.distance(hp) <= en.radius + 1.0 {
                        en.hp -= if cheats.onehit { 9999.0 } else { b.dmg };
                        en.flash = 0.12;
                        sfx.at(Sfx::Hit, hp, 0.5);
                        break;
                    }
                }
            } else if hp.distance(target) <= player_r + 1.0 {
                hurt_player(&mut stats, &cheats, b.dmg, ctl.in_car, &mut sfx);
            }
            commands.entity(e).despawn();
            continue;
        }
        tr.translation = p;
        if b.rocket {
            b.tick += dt;
            fx.push(FxCmd::Fire { pos: p });
            fx.push(FxCmd::Smoke { pos: p });
        }
    }
    for (pos, r, dmg, friendly) in booms {
        fx.push(FxCmd::Explosion { pos, size: (r / 9.0).max(0.7) });
        sfx.at(Sfx::Explosion, pos, 1.0);
        if friendly {
            for (et, mut en) in enemies.iter_mut() {
                let d = et.translation.distance(pos) - en.radius;
                if d < r {
                    en.hp -= if cheats.onehit { 9999.0 } else { dmg * (1.0 - (d / r).clamp(0.0, 1.0) * 0.7) };
                    en.flash = 0.15;
                }
            }
            // self damage when close
            let d = target.distance(pos);
            if d < r * 0.6 {
                hurt_player(&mut stats, &cheats, dmg * 0.2 * (1.0 - d / (r * 0.6)), ctl.in_car, &mut sfx);
            }
        } else {
            let d = target.distance(pos);
            if d < r {
                hurt_player(&mut stats, &cheats, dmg * (1.0 - d / r), ctl.in_car, &mut sfx);
            }
        }
    }
}
