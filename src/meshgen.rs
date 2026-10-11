//! Flat-shaded procedural mesh builder with vertex colour (rgb) + glow (alpha).
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};

pub type Col = [f32; 4];
pub fn col(r: f32, g: f32, b: f32, glow: f32) -> Col {
    [r, g, b, glow]
}

pub struct MB {
    pub pos: Vec<[f32; 3]>,
    pub nrm: Vec<[f32; 3]>,
    pub col: Vec<Col>,
    pub uv: Vec<[f32; 2]>,
    pub idx: Vec<u32>,
    pub xf: Mat4,
    pub use_uv: bool,
}

impl MB {
    pub fn new() -> Self {
        MB { pos: vec![], nrm: vec![], col: vec![], uv: vec![], idx: vec![], xf: Mat4::IDENTITY, use_uv: false }
    }
    pub fn tri(&mut self, a: Vec3, b: Vec3, c: Vec3, color: Col) {
        let (a, b, c) = (self.xf.transform_point3(a), self.xf.transform_point3(b), self.xf.transform_point3(c));
        let n = (b - a).cross(c - a).normalize_or_zero();
        let base = self.pos.len() as u32;
        for p in [a, b, c] {
            self.pos.push(p.to_array());
            self.nrm.push(n.to_array());
            self.col.push(color);
            self.uv.push([0.0, 0.0]);
        }
        self.idx.extend([base, base + 1, base + 2]);
    }
    /// Quad a,b,c,d in CCW order (seen from the front).
    pub fn quad(&mut self, a: Vec3, b: Vec3, c: Vec3, d: Vec3, color: Col) {
        self.tri(a, b, c, color);
        self.tri(a, c, d, color);
    }
    pub fn cuboid(&mut self, center: Vec3, half: Vec3, color: Col) {
        let (x, y, z) = (half.x, half.y, half.z);
        let p = |sx: f32, sy: f32, sz: f32| center + Vec3::new(sx * x, sy * y, sz * z);
        self.quad(p(-1., -1., 1.), p(1., -1., 1.), p(1., 1., 1.), p(-1., 1., 1.), color); // +z
        self.quad(p(1., -1., -1.), p(-1., -1., -1.), p(-1., 1., -1.), p(1., 1., -1.), color); // -z
        self.quad(p(1., -1., 1.), p(1., -1., -1.), p(1., 1., -1.), p(1., 1., 1.), color); // +x
        self.quad(p(-1., -1., -1.), p(-1., -1., 1.), p(-1., 1., 1.), p(-1., 1., -1.), color); // -x
        self.quad(p(-1., 1., 1.), p(1., 1., 1.), p(1., 1., -1.), p(-1., 1., -1.), color); // +y
        self.quad(p(-1., -1., -1.), p(1., -1., -1.), p(1., -1., 1.), p(-1., -1., 1.), color); // -y
    }
    /// Cone / pyramid with `seg` sides. Optional base cap.
    pub fn cone(&mut self, base: Vec3, r: f32, h: f32, seg: usize, color: Col, cap: bool) {
        let tip = base + Vec3::Y * h;
        for i in 0..seg {
            let a0 = i as f32 / seg as f32 * std::f32::consts::TAU;
            let a1 = (i + 1) as f32 / seg as f32 * std::f32::consts::TAU;
            let p0 = base + Vec3::new(a0.cos() * r, 0.0, a0.sin() * r);
            let p1 = base + Vec3::new(a1.cos() * r, 0.0, a1.sin() * r);
            self.tri(p0, tip, p1, color);
            if cap {
                self.tri(base, p0, p1, color);
            }
        }
    }
    /// Frustum / cylinder from base radius r0 to top radius r1.
    pub fn frustum(&mut self, base: Vec3, r0: f32, r1: f32, h: f32, seg: usize, color: Col, caps: bool) {
        let top = base + Vec3::Y * h;
        for i in 0..seg {
            let a0 = i as f32 / seg as f32 * std::f32::consts::TAU;
            let a1 = (i + 1) as f32 / seg as f32 * std::f32::consts::TAU;
            let (c0, s0, c1, s1) = (a0.cos(), a0.sin(), a1.cos(), a1.sin());
            let b0 = base + Vec3::new(c0 * r0, 0.0, s0 * r0);
            let b1 = base + Vec3::new(c1 * r0, 0.0, s1 * r0);
            let t0 = top + Vec3::new(c0 * r1, 0.0, s0 * r1);
            let t1 = top + Vec3::new(c1 * r1, 0.0, s1 * r1);
            self.quad(b1, b0, t0, t1, color);
            if caps {
                self.tri(top, t1, t0, color);
                self.tri(base, b0, b1, color);
            }
        }
    }
    /// Cylinder lying along the X axis (for wheels).
    pub fn wheel(&mut self, center: Vec3, r: f32, w: f32, seg: usize, color: Col) {
        let old = self.xf;
        self.xf = old * Mat4::from_translation(center) * Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);
        self.frustum(Vec3::new(0.0, -w * 0.5, 0.0), r, r, w, seg, color, true);
        self.xf = old;
    }
    pub fn with_xf<F: FnOnce(&mut Self)>(&mut self, xf: Mat4, f: F) {
        let old = self.xf;
        self.xf = old * xf;
        f(self);
        self.xf = old;
    }
    pub fn build(self) -> Mesh {
        let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.pos);
        m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.nrm);
        m.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.col);
        if self.use_uv {
            m.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uv);
        }
        m.insert_indices(Indices::U32(self.idx));
        m
    }
}
