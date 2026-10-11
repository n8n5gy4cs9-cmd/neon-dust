//! Shared game state, resources and small cross-module APIs (fx / sfx queues).
use bevy::prelude::*;

#[derive(States, Default, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum AppState {
    #[default]
    Menu,
    Playing,
    Paused,
    Dead,
    Ending,
}

#[derive(Component)]
pub struct PlayerCar;
#[derive(Component)]
pub struct Player;
#[derive(Component)]
pub struct Health {
    pub hp: f32,
    pub max: f32,
}

#[derive(Resource)]
pub struct Control {
    pub in_car: bool,
}

pub const NUM_WEAPONS: usize = 4;
pub const WEAPON_NAMES: [&str; NUM_WEAPONS] = ["PULSE PISTOL", "PLASMA SMG", "RAIL SHOTGUN", "ROCKET LAUNCHER"];

#[derive(Resource)]
pub struct Stats {
    pub hp: f32,
    pub max_hp: f32,
    pub kills: u32,
    pub score: u32,
    pub ammo: [i32; NUM_WEAPONS],
    pub owned: [bool; NUM_WEAPONS],
    pub cur: usize,
    pub cooldown: f32,
    pub hurt: f32,
    pub combat: f32,
    pub cars_destroyed: u32,
    pub time: f32,
    pub sentries: u32,
    pub boss_dead: bool,
}
impl Default for Stats {
    fn default() -> Self {
        Stats { hp: 100.0, max_hp: 100.0, kills: 0, score: 0, ammo: [0, 180, 24, 10], owned: [true, true, false, false], cur: 0, cooldown: 0.0, hurt: 0.0, combat: 0.0, cars_destroyed: 0, time: 0.0, sentries: 0, boss_dead: false }
    }
}

#[derive(Resource)]
pub struct Cheats {
    pub god: bool,
    pub noclip: bool,
    pub infammo: bool,
    pub onehit: bool,
    pub peaceful: bool,
    pub infboost: bool,
    pub enabled: bool,
    pub speed_mul: f32,
    pub lowgrav: bool,
}
impl Default for Cheats {
    fn default() -> Self {
        Cheats { god: false, noclip: false, infammo: false, onehit: false, peaceful: false, infboost: false, enabled: false, speed_mul: 1.0, lowgrav: false }
    }
}

/// True while console/pause UI eats gameplay input.
#[derive(Resource, Default)]
pub struct UiBlock(pub bool);

/// Last safe respawn position + yaw.
#[derive(Resource, Default)]
pub struct Checkpoint {
    pub pos: Vec3,
    pub yaw: f32,
}

// ---------- FX queue ----------------------------------------------------
pub enum FxCmd {
    Explosion { pos: Vec3, size: f32 },
    Spark { pos: Vec3, dir: Vec3, n: usize },
    Dust { pos: Vec3, vel: Vec3 },
    Muzzle { pos: Vec3, dir: Vec3, col: u8 },
    Fire { pos: Vec3 },
    Smoke { pos: Vec3 },
    Splash { pos: Vec3 },
    Boost { pos: Vec3, dir: Vec3 },
    Pickup { pos: Vec3 },
}
#[derive(Resource, Default)]
pub struct FxQueue(pub Vec<FxCmd>);
impl FxQueue {
    pub fn push(&mut self, c: FxCmd) {
        if self.0.len() < 400 {
            self.0.push(c);
        }
    }
}

// ---------- SFX queue ---------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Sfx {
    Pistol,
    Smg,
    Shotgun,
    Rocket,
    Explosion,
    Hit,
    Pickup,
    Boost,
    Jump,
    Door,
    Blip,
    Deny,
    EnemyShot,
    Hurt,
    Checkpoint,
    Crash,
    Land,
    Splash,
    Alarm,
    Win,
}
#[derive(Resource, Default)]
pub struct SfxQueue(pub Vec<(Sfx, Option<Vec3>, f32)>);
impl SfxQueue {
    pub fn play(&mut self, s: Sfx) {
        self.0.push((s, None, 1.0));
    }
    pub fn at(&mut self, s: Sfx, p: Vec3, v: f32) {
        self.0.push((s, Some(p), v));
    }
}

/// Camera screen-shake trauma.
#[derive(Resource, Default)]
pub struct Shake(pub f32);

/// Toast messages for HUD.
#[derive(Resource, Default)]
pub struct Toasts(pub Vec<(String, f32)>);
impl Toasts {
    pub fn say(&mut self, s: impl Into<String>) {
        self.0.push((s.into(), 3.5));
    }
}

#[derive(Resource, Default)]
pub struct ConsoleOpen(pub bool);
