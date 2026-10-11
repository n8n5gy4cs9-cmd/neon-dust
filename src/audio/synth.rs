//! Tiny offline synth: PolyBLEP oscillators, filters, drums, reverb/delay, WAV encoding.
use crate::noise::Rng;
use std::f32::consts::TAU;

pub const SR: f32 = 32000.0;

pub fn midi(n: f32) -> f32 {
    440.0 * 2f32.powf((n - 69.0) / 12.0)
}

fn polyblep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let x = t / dt;
        2.0 * x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + 2.0 * x + 1.0
    } else {
        0.0
    }
}
pub fn saw(ph: f32, dt: f32) -> f32 {
    2.0 * ph - 1.0 - polyblep(ph, dt)
}
pub fn pulse(ph: f32, dt: f32, pw: f32) -> f32 {
    let mut v = if ph < pw { 1.0 } else { -1.0 };
    v += polyblep(ph, dt);
    v -= polyblep((ph - pw).rem_euclid(1.0), dt);
    v
}

pub struct Lp {
    y: f32,
}
impl Lp {
    pub fn new() -> Self {
        Lp { y: 0.0 }
    }
    pub fn run(&mut self, x: f32, fc: f32) -> f32 {
        let a = 1.0 - (-TAU * fc.min(SR * 0.45) / SR).exp();
        self.y += a * (x - self.y);
        self.y
    }
}

#[derive(Clone)]
pub struct Buf {
    pub l: Vec<f32>,
    pub r: Vec<f32>,
}
impl Buf {
    pub fn new(n: usize) -> Self {
        Buf { l: vec![0.0; n], r: vec![0.0; n] }
    }
    pub fn len(&self) -> usize {
        self.l.len()
    }
    pub fn add_at(&mut self, start: usize, sig: &[f32], gl: f32, gr: f32) {
        for (i, s) in sig.iter().enumerate() {
            let k = start + i;
            if k >= self.l.len() {
                break;
            }
            self.l[k] += s * gl;
            self.r[k] += s * gr;
        }
    }
    pub fn mix(&mut self, o: &Buf, g: f32) {
        for i in 0..self.len() {
            self.l[i] += o.l[i] * g;
            self.r[i] += o.r[i] * g;
        }
    }
}

fn pan(p: f32) -> (f32, f32) {
    let a = (p * 0.5 + 0.5) * std::f32::consts::FRAC_PI_2;
    (a.cos(), a.sin())
}
pub fn pan_gains(p: f32) -> (f32, f32) {
    pan(p)
}

/// ADSR-ish: attack a, decay to sustain s over d, release r (all seconds); total length n samples (includes release).
fn env(i: usize, n: usize, a: f32, d: f32, s: f32, r: f32) -> f32 {
    let t = i as f32 / SR;
    let total = n as f32 / SR;
    let hold_end = (total - r).max(0.0);
    let base = if t < a {
        t / a.max(1e-4)
    } else if t < a + d {
        1.0 - (1.0 - s) * ((t - a) / d.max(1e-4))
    } else {
        s
    };
    if t > hold_end {
        base * (1.0 - (t - hold_end) / r.max(1e-4)).max(0.0)
    } else {
        base
    }
}

// ------------------------------------------------------------- instruments
pub fn supersaw(freq: f32, dur: f32, a: f32, r: f32, cutoff: f32, voices: usize, detune: f32, vib: f32) -> Vec<f32> {
    let n = ((dur + r) * SR) as usize;
    let mut out = vec![0.0; n];
    let mut phs: Vec<f32> = (0..voices).map(|k| (k as f32 * 0.37).fract()).collect();
    let mut lp = Lp::new();
    for i in 0..n {
        let t = i as f32 / SR;
        let v = 1.0 + (t * 5.5 * TAU).sin() * vib * (t * 2.0).min(1.0);
        let mut s = 0.0;
        for (k, ph) in phs.iter_mut().enumerate() {
            let off = (k as f32 - (voices as f32 - 1.0) * 0.5) * detune;
            let f = freq * v * 2f32.powf(off / 1200.0);
            let dt = f / SR;
            *ph = (*ph + dt).fract();
            s += saw(*ph, dt);
        }
        s /= voices as f32;
        out[i] = lp.run(s, cutoff) * env(i, n, a, 0.1, 0.85, r);
    }
    out
}

pub fn bass_note(freq: f32, dur: f32, cut: f32) -> Vec<f32> {
    let n = ((dur + 0.06) * SR) as usize;
    let mut out = vec![0.0; n];
    let (mut p1, mut p2) = (0.0f32, 0.0f32);
    let mut lp = Lp::new();
    for i in 0..n {
        let t = i as f32 / SR;
        let dt = freq / SR;
        p1 = (p1 + dt).fract();
        p2 = (p2 + dt * 0.5).fract();
        let s = saw(p1, dt) * 0.8 + (p2 * TAU).sin() * 0.9;
        let fc = 180.0 + cut * (-t * 9.0).exp();
        out[i] = lp.run(s, fc) * env(i, n, 0.004, 0.08, 0.7, 0.05);
    }
    out
}

pub fn pluck(freq: f32, dur: f32, bright: f32) -> Vec<f32> {
    let n = ((dur + 0.04) * SR) as usize;
    let mut out = vec![0.0; n];
    let mut ph = 0.0f32;
    let mut lp = Lp::new();
    for i in 0..n {
        let t = i as f32 / SR;
        let dt = freq / SR;
        ph = (ph + dt).fract();
        let s = pulse(ph, dt, 0.3 + 0.1 * (t * 3.0).sin());
        out[i] = lp.run(s, 500.0 + bright * (-t * 14.0).exp()) * env(i, n, 0.002, 0.12, 0.25, 0.04);
    }
    out
}

pub fn kick() -> Vec<f32> {
    let n = (0.42 * SR) as usize;
    let mut out = vec![0.0; n];
    let mut ph = 0.0f32;
    for i in 0..n {
        let t = i as f32 / SR;
        let f = 46.0 + 150.0 * (-t * 38.0).exp();
        ph += f / SR;
        let click = if t < 0.004 { (1.0 - t / 0.004) * 0.5 } else { 0.0 };
        out[i] = ((ph * TAU).sin() * (-t * 8.0).exp() + click) * 1.1;
    }
    out
}

pub fn snare(rng: &mut Rng) -> Vec<f32> {
    let n = (0.32 * SR) as usize;
    let mut out = vec![0.0; n];
    let mut prev = 0.0;
    let mut ph = 0.0f32;
    for i in 0..n {
        let t = i as f32 / SR;
        let w = rng.f() * 2.0 - 1.0;
        let hp = w - prev * 0.7;
        prev = w;
        ph += (190.0 - 60.0 * t.min(0.1) * 10.0) / SR;
        out[i] = hp * (-t * 17.0).exp() * 0.7 + (ph * TAU).sin() * (-t * 28.0).exp() * 0.6;
    }
    out
}

pub fn clap(rng: &mut Rng) -> Vec<f32> {
    let n = (0.28 * SR) as usize;
    let mut out = vec![0.0; n];
    let mut lp = Lp::new();
    for i in 0..n {
        let t = i as f32 / SR;
        let w = lp.run(rng.f() * 2.0 - 1.0, 4500.0);
        let burst = ((t * 95.0).fract() < 0.5 && t < 0.035) as i32 as f32;
        out[i] = w * (burst * 0.9 + (-t * 20.0).exp() * 0.5) * 0.7;
    }
    out
}

pub fn hat(rng: &mut Rng, open: bool) -> Vec<f32> {
    let len = if open { 0.22 } else { 0.06 };
    let n = (len * SR) as usize;
    let mut out = vec![0.0; n];
    let mut prev = 0.0;
    for i in 0..n {
        let t = i as f32 / SR;
        let w = rng.f() * 2.0 - 1.0;
        let hp = w - prev;
        prev = w;
        out[i] = hp * (-t * if open { 16.0 } else { 70.0 }).exp() * 0.35;
    }
    out
}

pub fn tom(freq: f32) -> Vec<f32> {
    let n = (0.3 * SR) as usize;
    let mut out = vec![0.0; n];
    let mut ph = 0.0f32;
    for i in 0..n {
        let t = i as f32 / SR;
        ph += (freq * (1.0 + 0.6 * (-t * 25.0).exp())) / SR;
        out[i] = (ph * TAU).sin() * (-t * 11.0).exp();
    }
    out
}

pub fn crash(rng: &mut Rng) -> Vec<f32> {
    let n = (1.8 * SR) as usize;
    let mut out = vec![0.0; n];
    let mut prev = 0.0;
    for i in 0..n {
        let t = i as f32 / SR;
        let w = rng.f() * 2.0 - 1.0;
        let hp = w - prev * 0.9;
        prev = w;
        out[i] = hp * (-t * 2.4).exp() * 0.3;
    }
    out
}

pub fn riser(secs: f32, rng: &mut Rng) -> Vec<f32> {
    let n = (secs * SR) as usize;
    let mut out = vec![0.0; n];
    let mut lp = Lp::new();
    for i in 0..n {
        let k = i as f32 / n as f32;
        let w = lp.run(rng.f() * 2.0 - 1.0, 300.0 + 7000.0 * k * k);
        out[i] = w * k * k * 0.6;
    }
    out
}

// ------------------------------------------------------------------ effects
pub fn reverb(inp: &[f32], wet: f32) -> (Vec<f32>, Vec<f32>) {
    let combs_l = [1687usize, 1931, 2203, 2477];
    let combs_r = [1777usize, 2017, 2297, 2549];
    let run = |delays: &[usize]| -> Vec<f32> {
        let mut out = vec![0.0f32; inp.len()];
        for &d in delays {
            let d = d * 3;
            let mut buf = vec![0.0f32; d];
            let mut idx = 0;
            let mut lp = 0.0f32;
            for i in 0..inp.len() {
                let y = buf[idx];
                lp += 0.45 * (y - lp);
                buf[idx] = inp[i] + lp * 0.80;
                idx = (idx + 1) % d;
                out[i] += y * 0.25;
            }
        }
        // allpass
        for &d in &[225usize, 556] {
            let mut buf = vec![0.0f32; d];
            let mut idx = 0;
            for i in 0..out.len() {
                let y = buf[idx];
                let x = out[i];
                let v = x + y * 0.5;
                buf[idx] = v;
                out[i] = y - v * 0.5;
                idx = (idx + 1) % d;
            }
        }
        out.iter().map(|v| v * wet).collect()
    };
    (run(&combs_l), run(&combs_r))
}

pub fn pingpong(b: &mut Buf, delay_s: f32, fb: f32, mix: f32) {
    let d = (delay_s * SR) as usize;
    let n = b.len();
    let (mut dl, mut dr) = (vec![0.0f32; n], vec![0.0f32; n]);
    for i in 0..n {
        let il = b.l[i];
        let ir = b.r[i];
        let pl = if i >= d { dr[i - d] } else { 0.0 };
        let pr = if i >= d { dl[i - d] } else { 0.0 };
        dl[i] = (il + ir) * 0.5 * 0.0 + pl * fb + il * 0.5;
        dr[i] = pr * fb + ir * 0.5;
    }
    for i in 0..n {
        b.l[i] += dl[i] * mix;
        b.r[i] += dr[i] * mix;
    }
}

pub fn master(b: &mut Buf, target: f32) {
    let peak = b.l.iter().chain(b.r.iter()).fold(0.0f32, |m, v| m.max(v.abs())).max(1e-4);
    let g = target / peak;
    for i in 0..b.len() {
        b.l[i] = (b.l[i] * g * 1.15).tanh();
        b.r[i] = (b.r[i] * g * 1.15).tanh();
    }
}

pub fn wav_stereo(b: &Buf) -> Vec<u8> {
    let n = b.len();
    let mut v = Vec::with_capacity(44 + n * 4);
    let data = (n * 4) as u32;
    v.extend(b"RIFF");
    v.extend((36 + data).to_le_bytes());
    v.extend(b"WAVEfmt ");
    v.extend(16u32.to_le_bytes());
    v.extend(1u16.to_le_bytes());
    v.extend(2u16.to_le_bytes());
    v.extend((SR as u32).to_le_bytes());
    v.extend((SR as u32 * 4).to_le_bytes());
    v.extend(4u16.to_le_bytes());
    v.extend(16u16.to_le_bytes());
    v.extend(b"data");
    v.extend(data.to_le_bytes());
    for i in 0..n {
        v.extend(((b.l[i].clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
        v.extend(((b.r[i].clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    v
}

pub fn wav_mono(s: &[f32]) -> Vec<u8> {
    let b = Buf { l: s.to_vec(), r: s.to_vec() };
    wav_stereo(&b)
}
