//! Input abstraction: slot 0 = keyboard + mouse ONLY, slot 1 = gamepad ONLY.
//! Systems read `Inputs` instead of devices so both players share the same code paths.
use crate::game::*;
use crate::players::*;
use crate::settings::Cvars;
use bevy::input::gamepad::{Gamepad, GamepadAxis, GamepadButton};
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::prelude::*;

#[derive(Default, Clone)]
pub struct PInput {
    /// On-foot move (x right, y forward).
    pub mv: Vec2,
    /// Camera delta this frame in radians (x yaw-left+, y pitch-down+).
    pub look: Vec2,
    pub throttle: f32,
    pub steer: f32,
    pub handbrake: bool,
    pub boost: bool,
    pub fire: bool,
    pub alt: bool,
    pub jump: bool,
    pub sprint: bool,
    /// Edge: enter/exit car.
    pub interact: bool,
    /// Held: hack terminal.
    pub hack: bool,
    pub weapon: Option<usize>,
    pub weapon_step: i32,
    /// Edge: Start button (join).
    pub start: bool,
}

#[derive(Resource, Default)]
pub struct Inputs(pub [PInput; MAXP]);

/// The gamepad entity assigned to player 2.
#[derive(Resource, Default)]
pub struct Pad2(pub Option<Entity>);

pub struct InputPlugin;
impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Inputs>().init_resource::<Pad2>().add_systems(PreUpdate, gather_inputs.after(bevy::input::InputSystem));
    }
}

fn dz(v: f32, d: f32) -> f32 {
    if v.abs() < d {
        0.0
    } else {
        (v - d * v.signum()) / (1.0 - d)
    }
}

#[allow(clippy::too_many_arguments)]
fn gather_inputs(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    pads: Query<(Entity, &Gamepad)>,
    mut pad2: ResMut<Pad2>,
    cv: Res<Cvars>,
    ui: Res<UiBlock>,
    time: Res<Time<Real>>,
    pl: Res<Pl>,
    mut inp: ResMut<Inputs>,
) {
    let dt = time.delta_secs().min(0.05);
    // keep the assigned pad valid; otherwise grab the first connected one
    if pad2.0.map_or(true, |e| pads.get(e).is_err()) {
        pad2.0 = pads.iter().next().map(|(e, _)| e);
    }

    // ---- slot 0: keyboard + mouse -----------------------------------------------------
    let mut p = PInput::default();
    if !ui.0 && pl.s[0].dead_t <= 0.0 {
        let k = |c: KeyCode| keys.pressed(c);
        let fw = k(KeyCode::KeyW) || k(KeyCode::ArrowUp);
        let bk = k(KeyCode::KeyS) || k(KeyCode::ArrowDown);
        let l = k(KeyCode::KeyA) || k(KeyCode::ArrowLeft);
        let r = k(KeyCode::KeyD) || k(KeyCode::ArrowRight);
        p.mv = Vec2::new(r as i32 as f32 - l as i32 as f32, fw as i32 as f32 - bk as i32 as f32);
        p.throttle = p.mv.y;
        p.steer = p.mv.x;
        p.handbrake = k(KeyCode::Space);
        p.boost = k(KeyCode::ShiftLeft) || k(KeyCode::ShiftRight);
        p.sprint = p.boost;
        p.jump = k(KeyCode::Space);
        p.fire = mouse.pressed(MouseButton::Left);
        p.alt = mouse.pressed(MouseButton::Right);
        p.interact = keys.just_pressed(KeyCode::KeyF);
        p.hack = k(KeyCode::KeyE);
        p.look = Vec2::new(-motion.delta.x, motion.delta.y) * cv.sens;
        for (i, c) in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4].iter().enumerate() {
            if keys.just_pressed(*c) {
                p.weapon = Some(i);
            }
        }
        if scroll.delta.y != 0.0 {
            p.weapon_step = if scroll.delta.y > 0.0 { 1 } else { -1 };
        }
    }
    p.start = false;
    inp.0[0] = p;

    // ---- slot 1: gamepad ----------------------------------------------------------------
    let mut q = PInput::default();
    if let Some((_, g)) = pad2.0.and_then(|e| pads.get(e).ok()) {
        q.start = g.just_pressed(GamepadButton::Start);
        if !ui.0 && pl.s[1].dead_t <= 0.0 {
            let lx = dz(g.get(GamepadAxis::LeftStickX).unwrap_or(0.0), 0.18);
            let ly = dz(g.get(GamepadAxis::LeftStickY).unwrap_or(0.0), 0.18);
            let rx = dz(g.get(GamepadAxis::RightStickX).unwrap_or(0.0), 0.15);
            let ry = dz(g.get(GamepadAxis::RightStickY).unwrap_or(0.0), 0.15);
            let rt = g.get(GamepadButton::RightTrigger2).unwrap_or(0.0);
            let lt = g.get(GamepadButton::LeftTrigger2).unwrap_or(0.0);
            q.mv = Vec2::new(lx, ly);
            q.steer = lx;
            q.throttle = (rt - lt).clamp(-1.0, 1.0);
            let in_car = pl.s[1].in_car;
            // car:  RT throttle, LT brake, stick steer, A handbrake, B boost, X cannons, RB rockets, Y exit
            // foot: stick move, A jump, L3 sprint, RT fire, LT aim, X hack, LB/RB weapon, Y enter
            q.handbrake = g.pressed(GamepadButton::South);
            q.boost = g.pressed(GamepadButton::East);
            q.jump = g.pressed(GamepadButton::South);
            q.sprint = g.pressed(GamepadButton::LeftThumb);
            q.fire = if in_car { g.pressed(GamepadButton::West) } else { rt > 0.5 };
            q.alt = if in_car { g.pressed(GamepadButton::RightTrigger) } else { lt > 0.5 };
            q.interact = g.just_pressed(GamepadButton::North);
            q.hack = g.pressed(GamepadButton::West);
            q.look = Vec2::new(-rx, -ry) * 2.8 * dt;
            if g.just_pressed(GamepadButton::DPadRight) || (!in_car && g.just_pressed(GamepadButton::RightTrigger)) {
                q.weapon_step = 1;
            }
            if g.just_pressed(GamepadButton::DPadLeft) || (!in_car && g.just_pressed(GamepadButton::LeftTrigger)) {
                q.weapon_step = -1;
            }
        }
    }
    inp.0[1] = q;
}
