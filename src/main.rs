mod audio;
mod camera;
mod capture;
mod enemies;
mod fx;
mod game;
mod meshgen;
mod noise;
mod player;
mod session;
mod story;
mod settings;
mod terrain;
mod ui;
mod vehicle;
mod weapons;
mod world;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window { title: "Neon Dust".into(), ..default() }),
            ..default()
        }))
        .init_resource::<settings::Cvars>()
        .add_plugins((
            world::WorldPlugin,
            capture::CapturePlugin,
            session::SessionPlugin,
            fx::FxPlugin,
            vehicle::VehiclePlugin,
            camera::CameraPlugin,
            player::PlayerPlugin,
            weapons::WeaponsPlugin,
            enemies::EnemiesPlugin,
            audio::GameAudioPlugin,
            story::StoryPlugin,
            ui::UiPlugin,
        ))
        .run();
}
