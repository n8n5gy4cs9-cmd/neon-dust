mod audio;
mod camera;
mod commands;
mod console;
mod capture;
mod enemies;
mod fonts;
mod fx;
mod game;
mod input;
mod meshgen;
mod noise;
mod player;
mod players;
mod session;
mod story;
mod settings;
mod split;
mod terrain;
mod ui;
mod vehicle;
mod weapons;
mod world;

use bevy::prelude::*;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window { title: "Neon Dust".into(), ..default() }),
        ..default()
    }));
    fonts::init(&mut app);
    app
        .init_resource::<settings::Cvars>()
        .add_plugins((
            world::WorldPlugin,
            capture::CapturePlugin,
            session::SessionPlugin,
            input::InputPlugin,
            fx::FxPlugin,
            vehicle::VehiclePlugin,
            camera::CameraPlugin,
            player::PlayerPlugin,
            weapons::WeaponsPlugin,
            enemies::EnemiesPlugin,
            audio::GameAudioPlugin,
            story::StoryPlugin,
            ui::UiPlugin,
            console::ConsolePlugin,
        ))
        .run();
}
