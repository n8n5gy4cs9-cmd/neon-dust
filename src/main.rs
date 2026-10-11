mod capture;
mod meshgen;
mod noise;
mod settings;
mod terrain;
mod world;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window { title: "Neon Dust".into(), ..default() }),
            ..default()
        }))
        .init_resource::<settings::Cvars>()
        .add_plugins((world::WorldPlugin, capture::CapturePlugin))
        .add_systems(Update, demo_cam)
        .run();
}

fn demo_cam(time: Res<Time>, mut cam: Query<&mut Transform, With<world::MainCam>>, mut focus: ResMut<world::Focus>) {
    let sp = world::start_pos();
    let t = time.elapsed_secs() * 0.1;
    let c = Vec3::new(sp.x + t.cos() * 200.0, 0.0, sp.y + t.sin() * 200.0);
    let g = terrain::ground_or_rim(c.x, c.z);
    for mut tr in &mut cam {
        tr.translation = c + Vec3::Y * (g + 60.0);
        tr.look_at(Vec3::new(sp.x, g + 20.0, sp.y), Vec3::Y);
    }
    focus.0 = c;
}
