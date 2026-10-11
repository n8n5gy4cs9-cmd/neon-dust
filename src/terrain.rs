//! Pure procedural floating-island terrain functions + chunk mesh generation.
use crate::meshgen::*;
use crate::noise::*;
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use std::sync::atomic::{AtomicU32, Ordering};

pub static SEED: AtomicU32 = AtomicU32::new(1337);
pub fn seed() -> u32 {
    SEED.load(Ordering::Relaxed)
}

pub const CHUNK: f32 = 128.0;
pub const RES: usize = 48;
pub const WATER_DEPTH: f32 = 1.5;

#[derive(Clone, Copy, Default)]
pub struct Sample {
    pub m: f32,
    pub rim: f32,
    pub top: f32,
    pub bottom: f32,
    pub river: f32,
    pub relief: f32,
}

pub fn rim_y(x: f32, z: f32) -> f32 {
    fbm(x / 5200.0, z / 5200.0, seed() + 3, 2) * 260.0
}

/// returns (mask incl. bridges, island core 0..1, bridge 0..1)
pub fn mask(x: f32, z: f32) -> (f32, f32, f32) {
    let s = seed();
    let n = fbm(x / 2600.0, z / 2600.0, s, 3) * 0.5 + 0.5;
    let isl = smoothstep(0.52, 0.57, n);
    let b = fbm(x / 1700.0 + 31.0, z / 1700.0 - 17.0, s + 7, 2).abs();
    let bridge = (1.0 - smoothstep(0.0, 0.05, b)) * smoothstep(0.30, 0.42, n);
    (isl.max(bridge * 0.75), isl, bridge)
}

pub fn sample(x: f32, z: f32) -> Sample {
    let s = seed();
    let (m, isl, _bridge) = mask(x, z);
    let rim = rim_y(x, z);
    if m <= 0.0 {
        return Sample { m: 0.0, rim, top: rim, bottom: rim, river: 0.0, relief: 0.0 };
    }
    let mount = ridged(x / 650.0, z / 650.0, s + 11, 5);
    let mountainous = smoothstep(0.38, 0.62, fbm(x / 1400.0 + 9.0, z / 1400.0 + 3.0, s + 21, 2) * 0.5 + 0.5);
    let hills = fbm(x / 220.0, z / 220.0, s + 5, 3) * 0.5 + 0.5;
    let detail = fbm(x / 70.0, z / 70.0, s + 8, 2);
    let relief = hills * 9.0 + detail * 1.5 + mountainous * (mount * mount * 190.0 + 4.0);
    let env = smoothstep(0.0, 0.6, isl);
    let mut h = relief * env + (m - isl).max(0.0) * 3.0 * hills;
    let r = fbm(x / 950.0 + 5.0, z / 950.0 - 9.0, s + 31, 2).abs();
    let rv = (1.0 - smoothstep(0.0, 0.022, r)) * smoothstep(0.0, 0.03, m);
    h += (-5.0 - h) * rv;
    let top = rim + h;
    let thick = m.powf(0.7) * (40.0 + 230.0 * ridged(x / 340.0, z / 340.0, s + 41, 3).powf(1.5));
    Sample { m, rim, top, bottom: top - thick, river: rv, relief: h }
}

/// Walkable ground height, None over the void.
pub fn ground(x: f32, z: f32) -> Option<f32> {
    let s = sample(x, z);
    if s.m > 0.015 { Some(s.top) } else { None }
}

pub fn ground_or_rim(x: f32, z: f32) -> f32 {
    sample(x, z).top
}

pub fn normal_at(x: f32, z: f32) -> Vec3 {
    let e = 1.5;
    let hx = ground_or_rim(x + e, z) - ground_or_rim(x - e, z);
    let hz = ground_or_rim(x, z + e) - ground_or_rim(x, z - e);
    Vec3::new(-hx / (2.0 * e), 1.0, -hz / (2.0 * e)).normalize()
}

pub fn water_y(x: f32, z: f32) -> f32 {
    rim_y(x, z) - WATER_DEPTH
}

pub fn is_water(x: f32, z: f32) -> bool {
    let s = sample(x, z);
    s.m > 0.05 && s.top < s.rim - WATER_DEPTH
}

/// Find a flat dry land spot near (x,z).
pub fn find_land(x: f32, z: f32, min_core: f32, max_r: f32) -> Vec2 {
    let mut r = 0.0;
    let mut k = 0;
    while r < max_r {
        let n = (6 + k * 3).max(1);
        for i in 0..n {
            let a = i as f32 / n as f32 * std::f32::consts::TAU + k as f32 * 0.7;
            let (px, pz) = (x + a.cos() * r, z + a.sin() * r);
            if good_spot(px, pz, min_core) {
                return Vec2::new(px, pz);
            }
        }
        r += 60.0;
        k += 1;
    }
    Vec2::new(x, z)
}

fn good_spot(x: f32, z: f32, min_core: f32) -> bool {
    let (_, isl, _) = mask(x, z);
    if isl < min_core {
        return false;
    }
    let c = sample(x, z);
    if c.river > 0.02 || c.relief < -0.5 {
        return false;
    }
    for (dx, dz) in [(30.0, 0.0), (-30.0, 0.0), (0.0, 30.0), (0.0, -30.0)] {
        let o = sample(x + dx, z + dz);
        if o.m < 0.5 || o.river > 0.02 || (o.top - c.top).abs() > 6.0 {
            return false;
        }
    }
    true
}

pub struct Prop {
    pub pos: Vec2,
    pub radius: f32,
}

pub struct ChunkData {
    pub cx: i32,
    pub cz: i32,
    pub top: Mesh,
    pub under: Option<Mesh>,
    pub water: Option<Mesh>,
    pub falls: Option<Mesh>,
    pub props: Option<Mesh>,
    pub colliders: Vec<Prop>,
}

fn top_color(s: &Sample, slope: f32) -> Col {
    let h01 = (s.relief / 150.0).clamp(0.0, 1.0);
    let low = Vec3::new(0.012, 0.05, 0.12);
    let plain = Vec3::new(0.06, 0.02, 0.17);
    let high = Vec3::new(0.30, 0.04, 0.34);
    let peak = Vec3::new(0.35, 0.55, 0.85);
    let mut c = plain.lerp(high, (h01 * 1.6).min(1.0));
    c = c.lerp(peak, smoothstep(0.7, 1.0, h01));
    c = c.lerp(low, smoothstep(0.0, 6.0, -s.relief) * 0.8);
    let cliff = smoothstep(0.55, 0.9, slope);
    c = c.lerp(Vec3::new(0.02, 0.02, 0.09), cliff * 0.7);
    [c.x, c.y, c.z, (h01 + cliff * 0.3).clamp(0.0, 1.0)]
}

pub fn build_chunk(cx: i32, cz: i32) -> ChunkData {
    let n = RES;
    let step = CHUNK / n as f32;
    let ox = cx as f32 * CHUNK;
    let oz = cz as f32 * CHUNK;
    let w = n + 3;
    let mut grid: Vec<Sample> = Vec::with_capacity(w * w);
    for j in 0..w {
        for i in 0..w {
            grid.push(sample(ox + (i as f32 - 1.0) * step, oz + (j as f32 - 1.0) * step));
        }
    }
    let at = |i: i32, j: i32| grid[((j + 1) as usize) * w + (i + 1) as usize];
    let vn = n + 1;

    let mut tpos = Vec::with_capacity(vn * vn);
    let mut tnrm = Vec::with_capacity(vn * vn);
    let mut tcol: Vec<Col> = Vec::with_capacity(vn * vn);
    let mut bpos = Vec::with_capacity(vn * vn);
    let mut bnrm = Vec::with_capacity(vn * vn);
    let mut bcol: Vec<Col> = Vec::with_capacity(vn * vn);
    let mut wpos = Vec::with_capacity(vn * vn);
    for j in 0..vn as i32 {
        for i in 0..vn as i32 {
            let s = at(i, j);
            let x = ox + i as f32 * step;
            let z = oz + j as f32 * step;
            let dx = at(i + 1, j).top - at(i - 1, j).top;
            let dz = at(i, j + 1).top - at(i, j - 1).top;
            let nm = Vec3::new(-dx / (2.0 * step), 1.0, -dz / (2.0 * step)).normalize();
            tpos.push([x, s.top, z]);
            tnrm.push(nm.to_array());
            tcol.push(top_color(&s, 1.0 - nm.y));
            let bdx = at(i + 1, j).bottom - at(i - 1, j).bottom;
            let bdz = at(i, j + 1).bottom - at(i, j - 1).bottom;
            let bn = Vec3::new(bdx / (2.0 * step), -1.0, bdz / (2.0 * step)).normalize();
            bpos.push([x, s.bottom, z]);
            bnrm.push(bn.to_array());
            let v = (s.m * 0.5 + hf(i + cx * 1000, j + cz * 1000, 5) * 0.1).min(1.0);
            bcol.push([0.05 + v * 0.1, 0.02, 0.12 + v * 0.1, 0.25 + 0.5 * smoothstep(0.0, 0.25, 0.25 - s.m).max(0.0)]);
            wpos.push([x, s.rim - WATER_DEPTH, z]);
        }
    }

    let mut tidx = Vec::new();
    let mut bidx = Vec::new();
    let mut widx = Vec::new();
    let mut falls = MB::new();
    falls.use_uv = true;
    let vi = |i: usize, j: usize| (j * vn + i) as u32;
    for j in 0..n {
        for i in 0..n {
            let corners = [at(i as i32, j as i32), at(i as i32 + 1, j as i32), at(i as i32, j as i32 + 1), at(i as i32 + 1, j as i32 + 1)];
            let any_land = corners.iter().any(|c| c.m > 0.0);
            if !any_land {
                continue;
            }
            let (a, b, c, d) = (vi(i, j), vi(i, j + 1), vi(i + 1, j), vi(i + 1, j + 1));
            tidx.extend([a, b, c, c, b, d]);
            bidx.extend([a, c, b, c, d, b]);
            let wet = corners.iter().filter(|c| c.m > 0.02 && c.top < c.rim - WATER_DEPTH).count();
            if wet >= 2 {
                widx.extend([a, b, c, c, b, d]);
                // waterfall where the river meets the island rim
                if corners.iter().any(|c| c.m < 0.12) && (i + j) % 1 == 0 {
                    let cs = at(i as i32, j as i32);
                    let (x, z) = (ox + (i as f32 + 0.5) * step, oz + (j as f32 + 0.5) * step);
                    let gx = at(i as i32 + 1, j as i32).m - at(i as i32 - 1, j as i32).m;
                    let gz = at(i as i32, j as i32 + 1).m - at(i as i32, j as i32 - 1).m;
                    let out = Vec2::new(-gx, -gz).normalize_or_zero();
                    if out != Vec2::ZERO {
                        let tang = Vec3::new(-out.y, 0.0, out.x) * step * 0.5;
                        let c0 = Vec3::new(x, cs.rim - WATER_DEPTH, z) + Vec3::new(out.x, 0.0, out.y) * step * 0.4;
                        let drop = Vec3::new(0.0, -140.0, 0.0);
                        let base = falls.pos.len() as u32;
                        let pts = [(c0 - tang, 0.0, 0.0), (c0 + tang, 1.0, 0.0), (c0 + tang + drop, 1.0, 1.0), (c0 - tang + drop, 0.0, 1.0)];
                        for (p, u, v) in pts {
                            falls.pos.push(p.to_array());
                            falls.nrm.push([out.x, 0.0, out.y]);
                            falls.col.push([0.5, 0.9, 1.0, 1.0]);
                            falls.uv.push([u, v]);
                        }
                        falls.idx.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
                    }
                }
            }
        }
    }
    let mk = |pos: Vec<[f32; 3]>, nrm: Vec<[f32; 3]>, col: Option<Vec<Col>>, idx: Vec<u32>| {
        let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
        m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, nrm);
        if let Some(c) = col {
            m.insert_attribute(Mesh::ATTRIBUTE_COLOR, c);
        }
        m.insert_indices(Indices::U32(idx));
        m
    };
    let has_land = !tidx.is_empty();
    let wnrm = vec![[0.0, 1.0, 0.0]; vn * vn];
    let wcol: Vec<Col> = vec![[0.0, 0.2, 0.4, 1.0]; vn * vn];
    let top = mk(tpos, tnrm, Some(tcol), tidx);
    let under = has_land.then(|| mk(bpos, bnrm, Some(bcol), bidx));
    let water = (!widx.is_empty()).then(|| mk(wpos, wnrm, Some(wcol), widx));
    let falls_m = (!falls.idx.is_empty()).then(|| falls.build());

    // props -------------------------------------------------------------
    let mut pb = MB::new();
    let mut colliders = Vec::new();
    if has_land {
        let s = seed();
        let mut rng = Rng::new((hash2(cx, cz, s) as u64) << 8 | 7);
        let start = crate::world::start_pos();
        for _ in 0..110 {
            let px = ox + rng.f() * CHUNK;
            let pz = oz + rng.f() * CHUNK;
            let sm = sample(px, pz);
            let kind = rng.f();
            let scale = rng.range(0.8, 1.7);
            if sm.m < 0.55 || sm.river > 0.01 || sm.relief < -0.3 {
                continue;
            }
            if Vec2::new(px - start.x, pz - start.y).length() < 40.0 {
                continue;
            }
            let nrm = normal_at(px, pz);
            if nrm.y < 0.85 {
                continue;
            }
            let forest = fbm(px / 180.0 + 4.0, pz / 180.0, s + 51, 2);
            let pos = Vec3::new(px, sm.top, pz);
            let hue = hf((px * 3.0) as i32, (pz * 3.0) as i32, 9);
            if forest > 0.05 && kind < 0.8 {
                // bioluminescent tree
                let glow = if hue < 0.5 { [0.05, 0.9, 0.8] } else if hue < 0.8 { [0.9, 0.1, 0.8] } else { [0.4, 0.2, 1.0] };
                let rot = rng.f() * 6.28;
                pb.with_xf(Mat4::from_translation(pos) * Mat4::from_rotation_y(rot) * Mat4::from_scale(Vec3::splat(scale)), |m| {
                    m.frustum(Vec3::ZERO, 0.35, 0.2, 3.2, 5, col(0.05, 0.03, 0.1, 0.0), false);
                    for k in 0..3 {
                        let r = 2.6 - k as f32 * 0.7;
                        m.cone(Vec3::new(0.0, 2.0 + k as f32 * 1.9, 0.0), r, 3.2, 7, col(glow[0] * 0.25, glow[1] * 0.25, glow[2] * 0.25, 0.9), true);
                    }
                });
                colliders.push(Prop { pos: Vec2::new(px, pz), radius: 0.6 * scale });
            } else if kind > 0.93 {
                // monolith
                let h = rng.range(8.0, 22.0);
                pb.with_xf(Mat4::from_translation(pos) * Mat4::from_rotation_y(rng.f() * 6.28), |m| {
                    m.cuboid(Vec3::new(0.0, h * 0.5, 0.0), Vec3::new(1.2, h * 0.5, 0.5), col(0.03, 0.02, 0.1, 0.0));
                    m.cuboid(Vec3::new(0.0, h * 0.5, 0.0), Vec3::new(1.25, 0.15, 0.55), col(0.0, 0.9, 1.0, 1.0));
                    m.cuboid(Vec3::new(0.0, h * 0.8, 0.0), Vec3::new(1.25, 0.15, 0.55), col(1.0, 0.1, 0.8, 1.0));
                    m.cuboid(Vec3::new(0.0, h * 0.2, 0.0), Vec3::new(1.25, 0.15, 0.55), col(0.5, 0.2, 1.0, 1.0));
                });
                colliders.push(Prop { pos: Vec2::new(px, pz), radius: 1.6 });
            } else if kind > 0.8 {
                // crystal cluster
                let cc = if hue < 0.5 { [0.05, 0.8, 1.0] } else { [1.0, 0.1, 0.7] };
                for k in 0..3 {
                    let h = rng.range(3.0, 11.0) * scale;
                    let off = Vec3::new(rng.range(-1.5, 1.5), 0.0, rng.range(-1.5, 1.5));
                    let tilt = Mat4::from_rotation_x(rng.range(-0.25, 0.25)) * Mat4::from_rotation_z(rng.range(-0.25, 0.25));
                    pb.with_xf(Mat4::from_translation(pos + off) * tilt, |m| {
                        m.frustum(Vec3::ZERO, 0.7 + k as f32 * 0.1, 0.55, h * 0.7, 6, col(cc[0] * 0.5, cc[1] * 0.5, cc[2] * 0.5, 1.0), false);
                        m.cone(Vec3::new(0.0, h * 0.7, 0.0), 0.55, h * 0.3, 6, col(cc[0], cc[1], cc[2], 1.0), false);
                    });
                }
                colliders.push(Prop { pos: Vec2::new(px, pz), radius: 1.8 });
            }
        }
    }
    let props = (!pb.idx.is_empty()).then(|| pb.build());
    ChunkData { cx, cz, top, under, water, falls: falls_m, props, colliders }
}
