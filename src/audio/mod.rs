//! Audio plugin: generates all music + SFX in code at startup (background thread) and plays them.
pub mod music;
pub mod sfx;
pub mod synth;

use crate::game::*;
use crate::noise::Rng;
use crate::settings::Cvars;
use crate::vehicle::Vehicle;
use crate::world::Focus;
use bevy::audio::{AudioSink, AudioSinkPlayback, PlaybackSettings, Volume};
use bevy::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

/// 0..=7 generation progress for the loading screen.
pub static AUDIO_STAGE: AtomicU32 = AtomicU32::new(0);
pub fn audio_progress() -> f32 {
    AUDIO_STAGE.load(Ordering::Relaxed) as f32 / 7.0
}

#[derive(Resource, Default)]
pub struct MusicCtl {
    pub port: bool,
    pub force_track: Option<usize>,
}

struct Generated {
    sfx: Vec<(Sfx, Vec<u8>)>,
    engine: Vec<u8>,
    skid: Vec<u8>,
    tracks: Vec<Vec<u8>>, // menu, drive_a, drive_b, port
    die: Vec<u8>,
}

#[derive(Resource)]
struct GenSlot(Arc<Mutex<Option<Generated>>>);

#[derive(Resource)]
struct Sounds {
    sfx: HashMap<Sfx, Handle<AudioSource>>,
    die: Handle<AudioSource>,
}

#[derive(Component)]
struct MusicLayer(usize);
#[derive(Component)]
struct EngineTag;
#[derive(Component)]
struct SkidTag;
#[derive(Component)]
struct SfxTag;

#[derive(Resource, Default)]
struct MusicVols([f32; 4]);

pub const ALL_SFX: [Sfx; 20] = [
    Sfx::Pistol, Sfx::Smg, Sfx::Shotgun, Sfx::Rocket, Sfx::Explosion, Sfx::Hit, Sfx::Pickup, Sfx::Boost, Sfx::Jump, Sfx::Door,
    Sfx::Blip, Sfx::Deny, Sfx::EnemyShot, Sfx::Hurt, Sfx::Checkpoint, Sfx::Crash, Sfx::Land, Sfx::Splash, Sfx::Alarm, Sfx::Win,
];

pub struct GameAudioPlugin;
impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        let slot: Arc<Mutex<Option<Generated>>> = Arc::new(Mutex::new(None));
        let s2 = slot.clone();
        std::thread::spawn(move || {
            let mut rng = Rng::new(1234);
            let sfx = ALL_SFX.iter().map(|s| (*s, synth::wav_mono(&sfx::make(*s, &mut rng)))).collect();
            AUDIO_STAGE.store(1, Ordering::Relaxed);
            let engine = synth::wav_mono(&sfx::engine_loop());
            let skid = synth::wav_mono(&sfx::noise_loop(2600.0));
            AUDIO_STAGE.store(2, Ordering::Relaxed);
            let menu = synth::wav_stereo(&music::render(&music::song_menu(), u32::MAX));
            AUDIO_STAGE.store(3, Ordering::Relaxed);
            let da = synth::wav_stereo(&music::render(&music::song_drive(), music::PAD | music::ARP | music::BASS | music::KICK | music::HAT));
            AUDIO_STAGE.store(4, Ordering::Relaxed);
            let db = synth::wav_stereo(&music::render(&music::song_drive(), music::SNARE | music::LEAD | music::TOM | music::FX));
            AUDIO_STAGE.store(5, Ordering::Relaxed);
            let port = synth::wav_stereo(&music::render(&music::song_port(), u32::MAX));
            AUDIO_STAGE.store(6, Ordering::Relaxed);
            let die = synth::wav_stereo(&music::stinger(false));
            if let Ok(dir) = std::env::var("ND_DUMP_AUDIO") {
                let _ = std::fs::create_dir_all(&dir);
                for (n, t) in [("menu", &menu), ("drive_a", &da), ("drive_b", &db), ("port", &port), ("die", &die), ("engine", &engine)] {
                    let _ = std::fs::write(format!("{dir}/{n}.wav"), t);
                }
            }
            AUDIO_STAGE.store(7, Ordering::Relaxed);
            *s2.lock().unwrap() = Some(Generated { sfx, engine, skid, tracks: vec![menu, da, db, port], die });
        });
        app.insert_resource(GenSlot(slot))
            .init_resource::<MusicCtl>()
            .init_resource::<MusicVols>()
            .add_systems(Update, (poll_generated, play_sfx, music_mix, engine_sound).chain());
    }
}

fn poll_generated(
    mut commands: Commands,
    slot: Res<GenSlot>,
    sounds: Option<Res<Sounds>>,
    mut assets: ResMut<Assets<AudioSource>>,
    mut toasts: ResMut<Toasts>,
) {
    if sounds.is_some() {
        return;
    }
    let Some(g) = slot.0.lock().unwrap().take() else { return };
    let mut mk = |b: Vec<u8>| assets.add(AudioSource { bytes: b.into() });
    let sfx: HashMap<Sfx, Handle<AudioSource>> = g.sfx.into_iter().map(|(k, v)| (k, mk(v))).collect();
    let die = mk(g.die);
    for (i, t) in g.tracks.into_iter().enumerate() {
        let h = mk(t);
        commands.spawn((MusicLayer(i), AudioPlayer(h), PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0))));
    }
    let eng = mk(g.engine);
    commands.spawn((EngineTag, AudioPlayer(eng), PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0))));
    let sk = mk(g.skid);
    commands.spawn((SkidTag, AudioPlayer(sk), PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0))));
    commands.insert_resource(Sounds { sfx, die });
    toasts.say("AUDIO ENGINE ONLINE");
}

fn play_sfx(
    mut commands: Commands,
    mut q: ResMut<SfxQueue>,
    sounds: Option<Res<Sounds>>,
    cv: Res<Cvars>,
    focus: Res<Focus>,
    active: Query<(), With<SfxTag>>,
    time: Res<Time<Real>>,
) {
    let Some(s) = sounds else {
        q.0.clear();
        return;
    };
    let mut n = active.iter().count();
    for (sfx, pos, vol) in q.0.drain(..) {
        if n > 40 {
            break;
        }
        let Some(h) = s.sfx.get(&sfx) else { continue };
        let att = pos.map_or(1.0, |p| 1.0 / (1.0 + p.distance(focus.0) / 45.0));
        let v = vol * att * cv.sfx_vol * 0.8;
        if v < 0.02 {
            continue;
        }
        let jitter = 0.94 + ((time.elapsed_secs() * 977.0 + n as f32 * 13.1).sin() * 0.5 + 0.5) * 0.12;
        commands.spawn((SfxTag, AudioPlayer(h.clone()), PlaybackSettings::DESPAWN.with_volume(Volume::Linear(v)).with_speed(jitter)));
        n += 1;
    }
}

#[allow(clippy::too_many_arguments)]
fn music_mix(
    time: Res<Time<Real>>,
    state: Res<State<AppState>>,
    ctl: Res<MusicCtl>,
    stats: Res<Stats>,
    cv: Res<Cvars>,
    mut vols: ResMut<MusicVols>,
    mut layers: Query<(&MusicLayer, &mut AudioSink)>,
    mut last_state: Local<Option<AppState>>,
    mut commands: Commands,
    sounds: Option<Res<Sounds>>,
) {
    let dt = time.delta_secs();
    let st = *state.get();
    if *last_state != Some(st) {
        if st == AppState::Dead {
            if let Some(s) = &sounds {
                commands.spawn((SfxTag, AudioPlayer(s.die.clone()), PlaybackSettings::DESPAWN.with_volume(Volume::Linear(cv.music_vol))));
            }
        }
        *last_state = Some(st);
    }
    let m = cv.music_vol;
    let target: [f32; 4] = match st {
        AppState::Menu | AppState::Ending => [1.0, 0.0, 0.0, 0.0],
        AppState::Dead => [0.0; 4],
        AppState::Playing | AppState::Paused => {
            let k = if st == AppState::Paused { 0.3 } else { 1.0 };
            let combat = if stats.combat > 0.0 { 1.0 } else { 0.0 };
            if let Some(t) = ctl.force_track {
                let mut a = [0.0; 4];
                a[t.min(3)] = 1.0;
                a
            } else if ctl.port {
                [0.0, 0.0, 0.0, k]
            } else {
                [0.0, k * 0.9, k * 1.4 * combat, 0.0]
            }
        }
    };
    for i in 0..4 {
        vols.0[i] += (target[i] - vols.0[i]) * (1.5 * dt).min(1.0);
    }
    for (l, mut sink) in &mut layers {
        sink.set_volume(Volume::Linear(vols.0[l.0] * m));
    }
}

fn engine_sound(
    state: Res<State<AppState>>,
    cv: Res<Cvars>,
    ctl: Option<Res<Control>>,
    focus: Res<Focus>,
    car: Query<(&Transform, &Vehicle), With<PlayerCar>>,
    mut eng: Query<&mut AudioSink, (With<EngineTag>, Without<SkidTag>)>,
    mut skid: Query<&mut AudioSink, (With<SkidTag>, Without<EngineTag>)>,
    mut boosting: Local<bool>,
    mut sfx: ResMut<SfxQueue>,
) {
    let Ok(mut e) = eng.single_mut() else { return };
    let Ok(mut s) = skid.single_mut() else { return };
    let playing = *state.get() == AppState::Playing;
    let Ok((t, v)) = car.single() else {
        e.set_volume(Volume::Linear(0.0));
        s.set_volume(Volume::Linear(0.0));
        return;
    };
    if !playing {
        e.set_volume(Volume::Linear(0.0));
        s.set_volume(Volume::Linear(0.0));
        return;
    }
    let in_car = ctl.map_or(true, |c| c.in_car);
    let d = if in_car { 0.0 } else { t.translation.distance(focus.0) };
    let att = 1.0 / (1.0 + d / 25.0);
    let rpm = (v.speed / 38.0).min(1.0);
    e.set_speed(0.55 + rpm * 1.9 + v.throttle.max(0.0) * 0.25);
    e.set_volume(Volume::Linear((0.12 + 0.3 * v.throttle.abs() + 0.15 * rpm) * att * cv.sfx_vol));
    let sk = if v.grounded && v.slip > 0.2 && v.speed > 8.0 { (v.slip * 0.8).min(0.5) } else { 0.0 };
    s.set_volume(Volume::Linear(sk * att * cv.sfx_vol));
    let b = v.boost && v.boost_fuel > 0.02 && v.throttle >= 0.0;
    if b && !*boosting {
        sfx.play(Sfx::Boost);
    }
    *boosting = b;
}
