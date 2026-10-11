use bevy::prelude::*;

#[derive(Resource, Clone)]
pub struct Cvars {
    pub view_radius: i32,
    pub fog_vis: f32,
    pub bloom: f32,
    pub fov: f32,
    pub music_vol: f32,
    pub sfx_vol: f32,
    pub sens: f32,
    pub show_fps: bool,
    pub timescale: f32,
}
impl Default for Cvars {
    fn default() -> Self {
        Cvars { view_radius: 6, fog_vis: 900.0, bloom: 0.28, fov: 70.0, music_vol: 0.55, sfx_vol: 0.8, sens: 0.0025, show_fps: false, timescale: 1.0 }
    }
}
