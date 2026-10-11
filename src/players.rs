//! Per-player ("slot") state for 1-2 player split-screen. Slot 0 = keyboard + mouse, slot 1 = gamepad.
use crate::camera::{AimInfo, CamRig};
use crate::game::NUM_WEAPONS;
use crate::split::SplitOpts;
use bevy::prelude::*;

pub const MAXP: usize = 2;

#[derive(Clone)]
pub struct Slot {
    pub active: bool,
    pub hp: f32,
    pub max_hp: f32,
    pub ammo: [i32; NUM_WEAPONS],
    pub owned: [bool; NUM_WEAPONS],
    pub cur: usize,
    pub cooldown: f32,
    pub hurt: f32,
    pub combat: f32,
    pub in_car: bool,
    /// > 0 while downed (co-op): seconds until respawn.
    pub dead_t: f32,
    pub rig: CamRig,
    pub aim: AimInfo,
    /// Tracked target position (car or on foot) + velocity, used by enemy AI and pickups.
    pub pos: Vec3,
    pub vel: Vec3,
    pub car_pos: Vec3,
    pub foot_pos: Vec3,
    pub focus: Vec3,
}

impl Slot {
    pub fn new(active: bool) -> Self {
        Slot {
            active,
            hp: 100.0,
            max_hp: 100.0,
            ammo: [0, 180, 24, 10],
            owned: [true, true, false, false],
            cur: 0,
            cooldown: 0.0,
            hurt: 0.0,
            combat: 0.0,
            in_car: false,
            dead_t: 0.0,
            rig: CamRig::default(),
            aim: AimInfo::default(),
            pos: Vec3::ZERO,
            vel: Vec3::ZERO,
            car_pos: Vec3::ZERO,
            foot_pos: Vec3::ZERO,
            focus: Vec3::ZERO,
        }
    }
    pub fn alive(&self) -> bool {
        self.active && self.dead_t <= 0.0
    }
    /// Reset gameplay stats (new game / respawn keeps position).
    pub fn reset_stats(&mut self) {
        let (rig, aim) = (self.rig.clone(), self.aim.clone());
        *self = Slot::new(self.active);
        self.rig = rig;
        self.aim = aim;
    }
}

#[derive(Resource)]
pub struct Pl {
    pub s: [Slot; MAXP],
    pub opts: SplitOpts,
    /// Bumped whenever the player count changes (cameras / HUD / entities re-sync).
    pub version: u32,
}
impl Default for Pl {
    fn default() -> Self {
        Pl { s: [Slot::new(true), Slot::new(false)], opts: SplitOpts::default(), version: 0 }
    }
}
impl Pl {
    pub fn count(&self) -> usize {
        self.s.iter().filter(|s| s.active).count()
    }
    pub fn mask(&self) -> [bool; 4] {
        [self.s[0].active, self.s[1].active, false, false]
    }
    /// Nearest alive player to a world position: (slot, pos, vel).
    pub fn nearest(&self, p: Vec3) -> Option<(usize, Vec3, Vec3)> {
        let mut best: Option<(usize, f32)> = None;
        for (i, s) in self.s.iter().enumerate() {
            if s.alive() {
                let d = s.pos.distance_squared(p);
                if best.map_or(true, |b| d < b.1) {
                    best = Some((i, d));
                }
            }
        }
        best.map(|(i, _)| (i, self.s[i].pos, self.s[i].vel))
    }
    pub fn any_in_car(&self) -> bool {
        self.s.iter().any(|s| s.alive() && s.in_car)
    }
    pub fn best_combat(&self) -> f32 {
        self.s.iter().filter(|s| s.active).map(|s| s.combat).fold(0.0, f32::max)
    }
    pub fn min_dist(&self, p: Vec3) -> f32 {
        self.s.iter().filter(|s| s.alive()).map(|s| s.pos.distance(p)).fold(f32::MAX, f32::min)
    }
}

pub fn hurt(slot: &mut Slot, god: bool, amount: f32, sfx: &mut crate::game::SfxQueue) {
    if god || amount <= 0.0 || slot.dead_t > 0.0 {
        return;
    }
    let a = if slot.in_car { amount * 0.6 } else { amount };
    slot.hp -= a;
    if slot.hurt <= 0.05 {
        sfx.play(crate::game::Sfx::Hurt);
    }
    slot.hurt = 0.45;
    slot.combat = 8.0;
}
