//! Deterministic hash + value noise helpers (no external crates).

pub fn hash_u32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

pub fn hash2(x: i32, z: i32, s: u32) -> u32 {
    hash_u32((x as u32).wrapping_mul(0x9E37_79B1) ^ hash_u32((z as u32).wrapping_add(s.wrapping_mul(0x85eb_ca6b))))
}

/// Hash to [0,1).
pub fn hf(x: i32, z: i32, s: u32) -> f32 {
    (hash2(x, z, s) >> 8) as f32 / 16_777_216.0
}

fn q(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Smooth value noise in [-1,1].
pub fn vnoise(x: f32, z: f32, s: u32) -> f32 {
    let xf = x.floor();
    let zf = z.floor();
    let (xi, zi) = (xf as i32, zf as i32);
    let (tx, tz) = (q(x - xf), q(z - zf));
    let a = hf(xi, zi, s);
    let b = hf(xi + 1, zi, s);
    let c = hf(xi, zi + 1, s);
    let d = hf(xi + 1, zi + 1, s);
    let v = a + (b - a) * tx + (c - a) * tz + (a - b - c + d) * tx * tz;
    v * 2.0 - 1.0
}

/// Fractal noise, roughly [-1,1].
pub fn fbm(x: f32, z: f32, s: u32, oct: u32) -> f32 {
    let (mut sum, mut amp, mut f, mut norm) = (0.0, 0.5, 1.0, 0.0);
    for o in 0..oct {
        sum += amp * vnoise(x * f + o as f32 * 17.3, z * f - o as f32 * 9.1, s.wrapping_add(o * 131));
        norm += amp;
        f *= 2.03;
        amp *= 0.5;
    }
    sum / norm
}

/// Ridged multifractal in [0,1].
pub fn ridged(x: f32, z: f32, s: u32, oct: u32) -> f32 {
    let (mut sum, mut amp, mut f, mut norm) = (0.0, 0.5, 1.0, 0.0);
    for o in 0..oct {
        let n = 1.0 - vnoise(x * f + o as f32 * 7.7, z * f + o as f32 * 3.1, s.wrapping_add(o * 977)).abs();
        sum += amp * n * n;
        norm += amp;
        f *= 2.1;
        amp *= 0.5;
    }
    sum / norm
}

pub fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Tiny xorshift RNG.
#[derive(Clone)]
pub struct Rng(pub u64);
impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 16) as u32
    }
    pub fn f(&mut self) -> f32 {
        (self.next_u32() & 0xFFFFFF) as f32 / 16_777_216.0
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f()
    }
    pub fn sign(&mut self) -> f32 {
        if self.f() < 0.5 { -1.0 } else { 1.0 }
    }
}
