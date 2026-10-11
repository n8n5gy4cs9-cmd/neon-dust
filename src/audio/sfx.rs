//! Procedural sound effects (mono).
use super::synth::*;
use crate::game::Sfx;
use crate::noise::Rng;
use std::f32::consts::TAU;

fn noise_env(rng: &mut Rng, len: f32, decay: f32, cut_start: f32, cut_decay: f32) -> Vec<f32> {
    let n = (len * SR) as usize;
    let mut lp = Lp::new();
    (0..n)
        .map(|i| {
            let t = i as f32 / SR;
            lp.run(rng.f() * 2.0 - 1.0, 120.0 + cut_start * (-t * cut_decay).exp()) * (-t * decay).exp()
        })
        .collect()
}

fn sweep(len: f32, f0: f32, f1: f32, decay: f32, square: bool) -> Vec<f32> {
    let n = (len * SR) as usize;
    let mut ph = 0.0f32;
    (0..n)
        .map(|i| {
            let t = i as f32 / SR;
            let k = (t / len).min(1.0);
            ph += (f0 + (f1 - f0) * k) / SR;
            let s = if square { pulse(ph.fract(), 0.02, 0.5) * 0.6 } else { (ph * TAU).sin() };
            s * (-t * decay).exp()
        })
        .collect()
}

fn mix(parts: &[(&[f32], f32)]) -> Vec<f32> {
    let n = parts.iter().map(|p| p.0.len()).max().unwrap_or(0);
    let mut out = vec![0.0; n];
    for (p, g) in parts {
        for (i, v) in p.iter().enumerate() {
            out[i] += v * g;
        }
    }
    out
}

fn delay(s: &[f32], secs: f32) -> Vec<f32> {
    let mut v = vec![0.0; (secs * SR) as usize];
    v.extend_from_slice(s);
    v
}

pub fn make(s: Sfx, rng: &mut Rng) -> Vec<f32> {
    match s {
        Sfx::Pistol => {
            let n = noise_env(rng, 0.14, 30.0, 9000.0, 20.0);
            let t = sweep(0.14, 1400.0, 180.0, 24.0, false);
            mix(&[(&n, 0.5), (&t, 0.6)])
        }
        Sfx::Smg => {
            let n = noise_env(rng, 0.09, 40.0, 8000.0, 30.0);
            let t = sweep(0.09, 900.0, 260.0, 30.0, true);
            mix(&[(&n, 0.4), (&t, 0.4)])
        }
        Sfx::Shotgun => {
            let n = noise_env(rng, 0.45, 8.0, 7000.0, 9.0);
            let t = sweep(0.3, 220.0, 50.0, 10.0, false);
            mix(&[(&n, 0.9), (&t, 0.8)])
        }
        Sfx::Rocket => {
            let n = noise_env(rng, 0.7, 4.0, 2500.0, -1.5);
            let t = sweep(0.5, 120.0, 400.0, 3.0, true);
            mix(&[(&n, 0.7), (&t, 0.3)])
        }
        Sfx::Explosion => {
            let n = noise_env(rng, 1.6, 2.6, 5000.0, 4.0);
            let t = sweep(1.2, 70.0, 22.0, 3.0, false);
            let c = noise_env(rng, 0.2, 18.0, 12000.0, 10.0);
            mix(&[(&n, 1.2), (&t, 1.0), (&c, 0.5)])
        }
        Sfx::Hit => {
            let t = sweep(0.07, 1800.0, 900.0, 40.0, false);
            let n = noise_env(rng, 0.05, 60.0, 9000.0, 20.0);
            mix(&[(&t, 0.5), (&n, 0.4)])
        }
        Sfx::Pickup => {
            let a = sweep(0.12, 660.0, 660.0, 8.0, true);
            let b = delay(&sweep(0.12, 880.0, 880.0, 8.0, true), 0.07);
            let c = delay(&sweep(0.2, 1320.0, 1320.0, 6.0, true), 0.14);
            mix(&[(&a, 0.4), (&b, 0.4), (&c, 0.4)])
        }
        Sfx::Boost => {
            let n = noise_env(rng, 0.8, 1.5, 6000.0, -2.0);
            let t = sweep(0.8, 200.0, 900.0, 1.5, true);
            mix(&[(&n, 0.8), (&t, 0.15)])
        }
        Sfx::Jump => sweep(0.18, 220.0, 520.0, 10.0, false),
        Sfx::Door => {
            let a = sweep(0.05, 300.0, 200.0, 40.0, true);
            let b = delay(&sweep(0.07, 500.0, 350.0, 30.0, true), 0.09);
            mix(&[(&a, 0.6), (&b, 0.6)])
        }
        Sfx::Blip => sweep(0.06, 1000.0, 1200.0, 25.0, false),
        Sfx::Deny => sweep(0.18, 140.0, 90.0, 10.0, true),
        Sfx::EnemyShot => {
            let t = sweep(0.12, 700.0, 140.0, 18.0, true);
            let n = noise_env(rng, 0.06, 40.0, 5000.0, 20.0);
            mix(&[(&t, 0.5), (&n, 0.3)])
        }
        Sfx::Hurt => {
            let t = sweep(0.25, 400.0, 100.0, 8.0, false);
            let n = noise_env(rng, 0.2, 14.0, 3000.0, 8.0);
            mix(&[(&t, 0.7), (&n, 0.5)])
        }
        Sfx::Checkpoint => {
            let a = sweep(0.5, 523.0, 523.0, 4.0, true);
            let b = delay(&sweep(0.5, 659.0, 659.0, 4.0, true), 0.1);
            let c = delay(&sweep(0.7, 784.0, 784.0, 3.0, true), 0.2);
            let d = delay(&sweep(0.9, 1046.0, 1046.0, 3.0, true), 0.3);
            mix(&[(&a, 0.35), (&b, 0.35), (&c, 0.35), (&d, 0.35)])
        }
        Sfx::Crash => {
            let n = noise_env(rng, 0.5, 8.0, 4000.0, 8.0);
            let t = sweep(0.25, 120.0, 45.0, 14.0, false);
            mix(&[(&n, 0.9), (&t, 0.8)])
        }
        Sfx::Land => {
            let t = sweep(0.2, 90.0, 40.0, 14.0, false);
            let n = noise_env(rng, 0.15, 22.0, 1500.0, 12.0);
            mix(&[(&t, 0.9), (&n, 0.4)])
        }
        Sfx::Splash => noise_env(rng, 0.35, 8.0, 4500.0, 3.0),
        Sfx::Alarm => {
            let a = sweep(0.2, 880.0, 880.0, 1.0, true);
            let b = delay(&sweep(0.2, 660.0, 660.0, 1.0, true), 0.2);
            let c = delay(&sweep(0.2, 880.0, 880.0, 1.0, true), 0.4);
            let d = delay(&sweep(0.2, 660.0, 660.0, 1.0, true), 0.6);
            mix(&[(&a, 0.3), (&b, 0.3), (&c, 0.3), (&d, 0.3)])
        }
        Sfx::Win => {
            let mut parts: Vec<Vec<f32>> = vec![];
            for (i, f) in [523.0, 659.0, 784.0, 1046.0, 1318.0].iter().enumerate() {
                parts.push(delay(&sweep(0.8, *f, *f, 3.0, true), i as f32 * 0.11));
            }
            let refs: Vec<(&[f32], f32)> = parts.iter().map(|p| (p.as_slice(), 0.25)).collect();
            mix(&refs)
        }
    }
}

/// Seamless engine loop: 1 s, fundamental 55 Hz (integer cycles).
pub fn engine_loop() -> Vec<f32> {
    let n = SR as usize;
    let mut out = vec![0.0; n];
    for i in 0..n {
        let t = i as f32 / SR;
        let mut s = 0.0;
        for h in 1..=14 {
            let amp = 1.0 / (h as f32).powf(1.1);
            s += (t * 55.0 * h as f32 * TAU).sin() * amp * if h % 2 == 0 { 0.8 } else { 1.0 };
        }
        let am = 0.75 + 0.25 * (t * 55.0 * 2.0 * TAU).sin();
        out[i] = s * am * 0.35 + (t * 27.5 * TAU).sin() * 0.3;
    }
    out
}

/// Seamless filtered-noise loop (tyre screech / wind).
pub fn noise_loop(cut: f32) -> Vec<f32> {
    let n = SR as usize;
    let mut rng = Rng::new(99);
    let mut lp = Lp::new();
    let mut raw: Vec<f32> = (0..n).map(|_| lp.run(rng.f() * 2.0 - 1.0, cut)).collect();
    let x = (0.08 * SR) as usize;
    for i in 0..x {
        let k = i as f32 / x as f32;
        let tail = raw[n - x + i];
        raw[i] = raw[i] * k + tail * (1.0 - k);
    }
    raw.truncate(n - x);
    raw
}
