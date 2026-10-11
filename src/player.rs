//! On-foot character "XZ": third-person controller, enter/exit car, death handling.
use crate::camera::lerp_angle;
use crate::game::*;
use crate::input::Inputs;
use crate::players::*;
use crate::meshgen::*;
use crate::terrain;
use crate::vehicle::Vehicle;
use crate::world::{Chunks, NeonMat, WorldMats};
use bevy::prelude::*;

#[derive(Component)]
pub struct Walker {
    pub vel: Vec3,
    pub yaw: f32,
    pub grounded: bool,
    pub anim: f32,
    pub moving: f32,
    pub last_safe: Vec3,
    pub shooting: f32,
}
#[derive(Component)]
pub struct Leg(pub f32);

fn body_mesh() -> Mesh {
    let mut m = MB::new();
    let suit = col(0.05, 0.03, 0.12, 0.0);
    let cy = col(0.0, 0.95, 1.0, 1.0);
    let mg = col(1.0, 0.1, 0.8, 1.0);
    m.cuboid(Vec3::new(0.0, 1.15, 0.0), Vec3::new(0.27, 0.38, 0.16), suit);
    m.cuboid(Vec3::new(0.0, 1.22, -0.165), Vec3::new(0.2, 0.05, 0.01), cy); // chest "X"
    m.cuboid(Vec3::new(0.0, 1.08, -0.165), Vec3::new(0.2, 0.02, 0.01), mg);
    m.cuboid(Vec3::new(0.0, 1.72, 0.0), Vec3::new(0.17, 0.17, 0.17), suit);
    m.cuboid(Vec3::new(0.0, 1.74, -0.175), Vec3::new(0.15, 0.05, 0.01), cy); // visor
    m.cuboid(Vec3::new(0.0, 1.2, 0.22), Vec3::new(0.2, 0.3, 0.07), col(0.12, 0.03, 0.2, 0.0)); // backpack
    m.cuboid(Vec3::new(-0.1, 1.2, 0.29), Vec3::new(0.02, 0.26, 0.01), mg);
    m.cuboid(Vec3::new(0.1, 1.2, 0.29), Vec3::new(0.02, 0.26, 0.01), mg);
    // arms + gun
    m.cuboid(Vec3::new(-0.36, 1.2, 0.0), Vec3::new(0.08, 0.3, 0.09), suit);
    m.cuboid(Vec3::new(0.36, 1.25, -0.2), Vec3::new(0.08, 0.08, 0.3), suit);
    m.cuboid(Vec3::new(0.36, 1.3, -0.55), Vec3::new(0.05, 0.07, 0.3), col(0.1, 0.1, 0.16, 0.0));
    m.cuboid(Vec3::new(0.36, 1.37, -0.55), Vec3::new(0.012, 0.012, 0.3), cy);
    m.build()
}
fn leg_mesh() -> Mesh {
    let mut m = MB::new();
    m.cuboid(Vec3::new(0.0, -0.4, 0.0), Vec3::new(0.1, 0.42, 0.1), col(0.06, 0.04, 0.14, 0.0));
    m.cuboid(Vec3::new(0.0, -0.4, -0.101), Vec3::new(0.02, 0.35, 0.005), col(0.0, 0.95, 1.0, 1.0));
    m.cuboid(Vec3::new(0.0, -0.8, -0.05), Vec3::new(0.11, 0.05, 0.16), col(0.03, 0.02, 0.08, 0.0));
    m.build()
}

pub fn spawn_player(commands: &mut Commands, meshes: &mut Assets<Mesh>, mats: &WorldMats, pos: Vec3, yaw: f32, slot: usize) -> Entity {
    let body = meshes.add(body_mesh());
    let leg = meshes.add(leg_mesh());
    commands
        .spawn((
            Player(slot),
            Walker { vel: Vec3::ZERO, yaw, grounded: true, anim: 0.0, moving: 0.0, last_safe: pos, shooting: 0.0 },
            Transform::from_translation(pos).with_rotation(Quat::from_rotation_y(yaw)),
            Visibility::default(),
        ))
        .with_children(|p| {
            p.spawn((Mesh3d(body), MeshMaterial3d(mats.solid.clone()), Transform::IDENTITY));
            for s in [-1.0f32, 1.0] {
                p.spawn((Mesh3d(leg.clone()), MeshMaterial3d(mats.solid.clone()), Transform::from_xyz(s * 0.14, 0.82, 0.0), Leg(s)));
            }
            p.spawn((PointLight { color: if slot == 0 { Color::srgb(0.1, 0.8, 1.0) } else { Color::srgb(1.0, 0.5, 0.1) }, intensity: 60_000.0, range: 6.0, ..default() }, Transform::from_xyz(0.0, 1.0, 0.0)));
        })
        .id()
}

pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (walker_move, walker_anim, enter_exit, death_check).chain().run_if(in_state(AppState::Playing)).after(crate::vehicle::VehicleSet),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn walker_move(
    time: Res<Time>,
    inputs: Res<Inputs>,
    mut pl: ResMut<Pl>,
    chunks: Res<Chunks>,
    cheats: Res<Cheats>,
    mut sfx: ResMut<SfxQueue>,
    mut fx: ResMut<FxQueue>,
    mut q: Query<(&mut Transform, &mut Walker, &Player)>,
) {
    let dt = time.delta_secs().min(0.05);
    for (mut tr, mut w, who) in &mut q {
        let i = who.0;
        if i >= MAXP || pl.s[i].in_car || !pl.s[i].active {
            continue;
        }
        let inp = inputs.0[i].clone();
        let aiming = pl.s[i].rig.aiming;
        let yaw = pl.s[i].rig.yaw;
        let sprint = inp.sprint;
        let cam_fwd = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
        let cam_right = Vec3::new(yaw.cos(), 0.0, -yaw.sin());
        let wish = (cam_fwd * inp.mv.y + cam_right * inp.mv.x).clamp_length_max(1.0);
        let p = tr.translation;
        let sm = terrain::sample(p.x, p.z);
        let in_water = sm.m > 0.05 && sm.top < sm.rim - terrain::WATER_DEPTH && p.y < sm.rim - 1.0;
        let mut speed = if sprint { 12.5 } else { 7.0 } * cheats.speed_mul;
        if in_water {
            speed *= 0.55;
        }
        if aiming {
            speed *= 0.6;
        }
        if cheats.noclip && i == 0 {
            let mut v = wish * speed * 3.0;
            if inp.jump {
                v.y += speed * 2.5;
            }
            tr.translation += v * dt;
            w.vel = Vec3::ZERO;
        } else {
            let acc = if w.grounded { 60.0 } else { 12.0 };
            let target = wish * speed;
            let cur = Vec3::new(w.vel.x, 0.0, w.vel.z);
            let nv = cur + (target - cur).clamp_length_max(acc * dt);
            w.vel.x = nv.x;
            w.vel.z = nv.z;
            if w.grounded && inp.jump {
                w.vel.y = 9.5;
                w.grounded = false;
                sfx.play(Sfx::Jump);
            }
            let g = if cheats.lowgrav { 8.0 } else { 26.0 };
            w.vel.y -= g * dt;
            tr.translation += w.vel * dt;
            let p = tr.translation;
            let s2 = terrain::sample(p.x, p.z);
            if s2.m > 0.015 && p.y <= s2.top {
                if !w.grounded && w.vel.y < -14.0 {
                    sfx.at(Sfx::Land, p, 0.6);
                    for _ in 0..4 {
                        fx.push(FxCmd::Dust { pos: p, vel: Vec3::new((p.x * 3.0).sin() * 2.0, 1.5, (p.z * 3.0).cos() * 2.0) });
                    }
                }
                tr.translation.y = s2.top;
                w.vel.y = 0.0;
                w.grounded = true;
                if s2.m > 0.4 {
                    w.last_safe = tr.translation;
                }
            } else {
                w.grounded = false;
            }
            if let Some((n, pen)) = chunks.collide(Vec2::new(p.x, p.z), 0.45) {
                tr.translation.x += n.x * pen;
                tr.translation.z += n.y * pen;
            }
            if tr.translation.y < s2.rim - 450.0 {
                tr.translation = w.last_safe + Vec3::Y * 1.0;
                w.vel = Vec3::ZERO;
                crate::players::hurt(&mut pl.s[i], cheats.god, 12.0, &mut sfx);
            }
        }
        // facing
        w.shooting = (w.shooting - dt).max(0.0);
        if inp.fire {
            w.shooting = 0.4;
        }
        let face_cam = aiming || w.shooting > 0.0;
        let moving = Vec3::new(w.vel.x, 0.0, w.vel.z);
        if face_cam {
            w.yaw = lerp_angle(w.yaw, yaw, 1.0 - (-18.0 * dt).exp());
        } else if moving.length() > 0.5 {
            let ty = (-moving.x).atan2(-moving.z);
            w.yaw = lerp_angle(w.yaw, ty, 1.0 - (-14.0 * dt).exp());
        }
        w.moving = moving.length();
        tr.rotation = Quat::from_rotation_y(w.yaw);
        w.anim += w.moving * dt * 1.1;
        if w.moving > 6.0 && w.grounded && ((time.elapsed_secs() * 10.0) as i32) % 3 == 0 {
            fx.push(FxCmd::Dust { pos: tr.translation + Vec3::Y * 0.1, vel: -moving * 0.1 + Vec3::Y * 0.5 });
        }
    }
}

fn walker_anim(w: Query<&Walker, With<Player>>, children: Query<&Children>, walkers: Query<(Entity, &Walker), With<Player>>, mut legs: Query<(&mut Transform, &Leg)>) {
    let _ = &w;
    for (e, w) in &walkers {
        let amp = (w.moving / 8.0).clamp(0.0, 1.2) * 0.9;
        if let Ok(ch) = children.get(e) {
            for c in ch.iter() {
                if let Ok((mut t, l)) = legs.get_mut(c) {
                    let a = if w.grounded { (w.anim * 2.4).sin() * amp * l.0 } else { 0.5 * l.0 };
                    t.rotation = Quat::from_rotation_x(a);
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn enter_exit(
    inputs: Res<Inputs>,
    mut pl: ResMut<Pl>,
    mut sfx: ResMut<SfxQueue>,
    mut toasts: ResMut<Toasts>,
    mut car: Query<(&Transform, &mut Vehicle, &PlayerCar), Without<Player>>,
    mut walkers: Query<(&mut Transform, &mut Walker, &mut Visibility, &Player)>,
) {
    for i in 0..MAXP {
        if !pl.s[i].alive() || !inputs.0[i].interact {
            continue;
        }
        let Some((ct, mut v, _)) = car.iter_mut().find(|(_, _, p)| p.0 == i) else { continue };
        let Some((mut pt, mut w, mut vis, _)) = walkers.iter_mut().find(|(_, _, _, p)| p.0 == i) else { continue };
        if pl.s[i].in_car {
            let right = v.right();
            pt.translation = ct.translation + right * -2.4 + Vec3::Y * 0.5;
            w.vel = Vec3::new(v.vel.x, 0.0, v.vel.z) * 0.4 + Vec3::Y * 4.0;
            w.yaw = v.yaw;
            w.grounded = false;
            *vis = Visibility::Inherited;
            pl.s[i].in_car = false;
            v.throttle = 0.0;
            pl.s[i].rig.yaw = v.yaw;
            pl.s[i].rig.pitch = 0.2;
            sfx.play(Sfx::Door);
            toasts.say("ON FOOT - press F / Y near the car to get back in");
        } else if pt.translation.distance(ct.translation) < 6.0 {
            pl.s[i].in_car = true;
            *vis = Visibility::Hidden;
            pl.s[i].rig.yaw = v.yaw;
            sfx.play(Sfx::Door);
        } else {
            sfx.play(Sfx::Deny);
            toasts.say("Too far from the car");
        }
    }
}

/// Health clamp, hurt decay, and death. Single player: Kernel Panic. Co-op: the player is downed and
/// respawns near their partner; both down = Kernel Panic.
#[allow(clippy::too_many_arguments)]
fn death_check(
    time: Res<Time>,
    cheats: Res<Cheats>,
    mut pl: ResMut<Pl>,
    mut next: ResMut<NextState<AppState>>,
    mut fx: ResMut<FxQueue>,
    mut sfx: ResMut<SfxQueue>,
    mut toasts: ResMut<Toasts>,
    mut walkers: Query<(&mut Transform, &mut Walker, &mut Visibility, &Player)>,
    mut cars: Query<(&mut Transform, &mut Vehicle, &PlayerCar), Without<Player>>,
) {
    let dt = time.delta_secs();
    let multi = pl.count() > 1;
    for i in 0..MAXP {
        if !pl.s[i].active {
            continue;
        }
        let s = &mut pl.s[i];
        if cheats.god && s.dead_t <= 0.0 {
            s.hp = s.hp.max(1.0);
        }
        s.hp = s.hp.min(s.max_hp);
        s.hurt = (s.hurt - dt).max(0.0);
        if s.dead_t > 0.0 {
            s.dead_t -= dt;
            if s.dead_t <= 0.0 {
                // respawn next to the partner (or at the car's last safe spot)
                s.hp = s.max_hp;
                s.hurt = 0.0;
                s.in_car = true;
                let partner = pl.s[1 - i].alive().then(|| pl.s[1 - i].pos);
                if let Some((mut t, mut v, _)) = cars.iter_mut().find(|(_, _, p)| p.0 == i).map(|(a, b, c)| (a, b, c)) {
                    let base = partner.map(|p| p + Vec3::new(6.0, 0.0, 0.0)).unwrap_or(v.last_safe);
                    t.translation = Vec3::new(base.x, terrain::ground_or_rim(base.x, base.z) + 1.0, base.z);
                    v.vel = Vec3::ZERO;
                    v.yaw_rate = 0.0;
                    v.boost_fuel = 1.0;
                }
                toasts.say(format!("PLAYER {} REBOOTED", i + 1));
                sfx.play(Sfx::Checkpoint);
            }
            continue;
        }
        if s.hp <= 0.0 {
            let pos = s.pos;
            fx.push(FxCmd::Explosion { pos, size: 1.6 });
            sfx.play(Sfx::Explosion);
            if multi {
                s.dead_t = 5.0;
                s.hp = 0.0;
                s.in_car = false;
                // hide the body; the car idles with the handbrake on (inputs are zeroed while downed)
                if let Some((_, _, mut vis, _)) = walkers.iter_mut().find(|(_, _, _, p)| p.0 == i) {
                    *vis = Visibility::Hidden;
                }
            }
        }
    }
    let all_dead = pl.s.iter().filter(|s| s.active).all(|s| s.hp <= 0.0 && (s.dead_t > 0.0 || !multi));
    if all_dead {
        next.set(AppState::Dead);
    }
}
