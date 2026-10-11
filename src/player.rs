//! On-foot character "XZ": third-person controller, enter/exit car, death handling.
use crate::camera::{lerp_angle, CamRig};
use crate::game::*;
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

pub fn spawn_player(commands: &mut Commands, meshes: &mut Assets<Mesh>, mats: &WorldMats, pos: Vec3, yaw: f32) -> Entity {
    let body = meshes.add(body_mesh());
    let leg = meshes.add(leg_mesh());
    commands
        .spawn((
            Player,
            Walker { vel: Vec3::ZERO, yaw, grounded: true, anim: 0.0, moving: 0.0, last_safe: pos, shooting: 0.0 },
            Transform::from_translation(pos).with_rotation(Quat::from_rotation_y(yaw)),
            Visibility::default(),
        ))
        .with_children(|p| {
            p.spawn((Mesh3d(body), MeshMaterial3d(mats.solid.clone()), Transform::IDENTITY));
            for s in [-1.0f32, 1.0] {
                p.spawn((Mesh3d(leg.clone()), MeshMaterial3d(mats.solid.clone()), Transform::from_xyz(s * 0.14, 0.82, 0.0), Leg(s)));
            }
            p.spawn((PointLight { color: Color::srgb(0.1, 0.8, 1.0), intensity: 60_000.0, range: 6.0, ..default() }, Transform::from_xyz(0.0, 1.0, 0.0)));
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
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    ui: Res<UiBlock>,
    ctl: Res<Control>,
    rig: Res<CamRig>,
    chunks: Res<Chunks>,
    cheats: Res<Cheats>,
    mut sfx: ResMut<SfxQueue>,
    mut fx: ResMut<FxQueue>,
    mut stats: ResMut<Stats>,
    mut q: Query<(&mut Transform, &mut Walker), With<Player>>,
) {
    if ctl.in_car {
        return;
    }
    let Ok((mut tr, mut w)) = q.single_mut() else { return };
    let dt = time.delta_secs().min(0.05);
    let blocked = ui.0;
    let k = |c: KeyCode| !blocked && keys.pressed(c);
    let mut input = Vec2::ZERO;
    if k(KeyCode::KeyW) || k(KeyCode::ArrowUp) {
        input.y += 1.0;
    }
    if k(KeyCode::KeyS) || k(KeyCode::ArrowDown) {
        input.y -= 1.0;
    }
    if k(KeyCode::KeyD) || k(KeyCode::ArrowRight) {
        input.x += 1.0;
    }
    if k(KeyCode::KeyA) || k(KeyCode::ArrowLeft) {
        input.x -= 1.0;
    }
    let sprint = k(KeyCode::ShiftLeft);
    let cam_fwd = Vec3::new(-rig.yaw.sin(), 0.0, -rig.yaw.cos());
    let cam_right = Vec3::new(rig.yaw.cos(), 0.0, -rig.yaw.sin());
    let wish = (cam_fwd * input.y + cam_right * input.x).normalize_or_zero();
    let p = tr.translation;
    let sm = terrain::sample(p.x, p.z);
    let in_water = sm.m > 0.05 && sm.top < sm.rim - terrain::WATER_DEPTH && p.y < sm.rim - 1.0;
    let mut speed = if sprint { 12.5 } else { 7.0 } * cheats.speed_mul;
    if in_water {
        speed *= 0.55;
    }
    if rig.aiming {
        speed *= 0.6;
    }
    if cheats.noclip {
        let mut v = wish * speed * 3.0;
        if k(KeyCode::Space) {
            v.y += speed * 2.5;
        }
        if k(KeyCode::ControlLeft) {
            v.y -= speed * 2.5;
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
        if w.grounded && k(KeyCode::Space) {
            w.vel.y = 9.5;
            w.grounded = false;
            sfx.play(Sfx::Jump);
        }
        let g = if cheats.lowgrav { 8.0 } else { 26.0 };
        w.vel.y -= g * dt;
        tr.translation += w.vel * dt;
        // ground
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
            if !cheats.god {
                stats.hp -= 12.0;
                stats.hurt = 0.5;
            }
        }
    }
    // facing
    w.shooting = (w.shooting - dt).max(0.0);
    if mouse.pressed(MouseButton::Left) && !blocked {
        w.shooting = 0.4;
    }
    let face_cam = rig.aiming || w.shooting > 0.0;
    let moving = Vec3::new(w.vel.x, 0.0, w.vel.z);
    if face_cam {
        w.yaw = lerp_angle(w.yaw, rig.yaw, 1.0 - (-18.0 * dt).exp());
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

fn walker_anim(w: Query<&Walker, With<Player>>, mut legs: Query<(&mut Transform, &Leg)>) {
    let Ok(w) = w.single() else { return };
    let amp = (w.moving / 8.0).clamp(0.0, 1.2) * 0.9;
    for (mut t, l) in &mut legs {
        let a = if w.grounded { (w.anim * 2.4).sin() * amp * l.0 } else { 0.5 * l.0 };
        t.rotation = Quat::from_rotation_x(a);
    }
}

#[allow(clippy::too_many_arguments)]
fn enter_exit(
    keys: Res<ButtonInput<KeyCode>>,
    ui: Res<UiBlock>,
    mut ctl: ResMut<Control>,
    mut sfx: ResMut<SfxQueue>,
    mut toasts: ResMut<Toasts>,
    mut rig: ResMut<CamRig>,
    mut car: Query<(&Transform, &mut Vehicle), (With<PlayerCar>, Without<Player>)>,
    mut pl: Query<(&mut Transform, &mut Walker, &mut Visibility), With<Player>>,
) {
    if ui.0 || !keys.just_pressed(KeyCode::KeyF) {
        return;
    }
    let (Ok((ct, mut v)), Ok((mut pt, mut w, mut vis))) = (car.single_mut(), pl.single_mut()) else { return };
    if ctl.in_car {
        // bail out
        let right = v.right();
        pt.translation = ct.translation + right * -2.4 + Vec3::Y * 0.5;
        w.vel = Vec3::new(v.vel.x, 0.0, v.vel.z) * 0.4 + Vec3::Y * 4.0;
        w.yaw = v.yaw;
        w.grounded = false;
        *vis = Visibility::Inherited;
        ctl.in_car = false;
        v.throttle = 0.0;
        rig.yaw = v.yaw;
        rig.pitch = 0.2;
        sfx.play(Sfx::Door);
        toasts.say("ON FOOT - press F near the car to get back in");
    } else if pt.translation.distance(ct.translation) < 6.0 {
        ctl.in_car = true;
        *vis = Visibility::Hidden;
        rig.yaw = v.yaw;
        sfx.play(Sfx::Door);
    } else {
        sfx.play(Sfx::Deny);
        toasts.say("Too far from the car");
    }
}

fn death_check(
    mut stats: ResMut<Stats>,
    cheats: Res<Cheats>,
    mut next: ResMut<NextState<AppState>>,
    mut fx: ResMut<FxQueue>,
    mut sfx: ResMut<SfxQueue>,
    focus: Res<crate::world::Focus>,
) {
    if cheats.god {
        stats.hp = stats.hp.max(1.0);
    }
    stats.hp = stats.hp.min(stats.max_hp);
    stats.hurt = (stats.hurt - 0.016).max(0.0);
    if stats.hp <= 0.0 {
        fx.push(FxCmd::Explosion { pos: focus.0, size: 1.6 });
        sfx.play(Sfx::Explosion);
        next.set(AppState::Dead);
    }
}
