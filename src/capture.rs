//! Headless-friendly screenshot helper: `--shot <path> [secs]` saves a frame then quits.
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

#[derive(Resource)]
pub struct ShotArgs {
    pub path: String,
    pub at: f32,
    pub taken: bool,
}

pub struct CapturePlugin;
impl Plugin for CapturePlugin {
    fn build(&self, app: &mut App) {
        let args: Vec<String> = std::env::args().collect();
        if let Some(i) = args.iter().position(|a| a == "--shot") {
            let path = args.get(i + 1).cloned().unwrap_or("shot.png".into());
            let at = args.get(i + 2).and_then(|s| s.parse().ok()).unwrap_or(6.0);
            app.insert_resource(ShotArgs { path, at, taken: false }).add_systems(Update, shoot);
        }
    }
}

fn shoot(mut commands: Commands, time: Res<Time<Real>>, mut s: ResMut<ShotArgs>, mut exit: EventWriter<AppExit>) {
    if !s.taken && time.elapsed_secs() > s.at {
        s.taken = true;
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(s.path.clone()));
    }
    if s.taken && time.elapsed_secs() > s.at + 3.0 {
        exit.write(AppExit::Success);
    }
}
