//! World rendering: materials, chunk streaming, sky, clouds, distant silhouettes, motes, lights.
use crate::meshgen::*;
use crate::noise::*;
use crate::settings::Cvars;
use crate::terrain::{self, ChunkData, Prop, CHUNK};
use bevy::asset::weak_handle;
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::fxaa::Fxaa;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::pbr::{
    CascadeShadowConfigBuilder, DistanceFog, ExtendedMaterial, FogFalloff, MaterialExtension, MaterialPipeline, MaterialPipelineKey,
    NotShadowCaster, NotShadowReceiver,
};
use bevy::prelude::*;
use bevy::render::mesh::MeshVertexBufferLayoutRef;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderRef, SpecializedMeshPipelineError};
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};
use std::collections::HashMap;
use std::sync::Mutex;

pub const NEON_SHADER: Handle<Shader> = weak_handle!("4b1d2e2e-7c53-4c0e-9f1a-0a5e1c3d9b01");
pub const SKY_SHADER: Handle<Shader> = weak_handle!("4b1d2e2e-7c53-4c0e-9f1a-0a5e1c3d9b02");

pub const SUN_DIR: Vec3 = Vec3::new(0.3, 0.14, -0.94);
pub const FOG_COLOR: Color = Color::srgb(0.62, 0.16, 0.5);

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct NeonExt {
    #[uniform(100)]
    pub params: Vec4,
    #[uniform(101)]
    pub color: Vec4,
}
impl MaterialExtension for NeonExt {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(NEON_SHADER)
    }
}
pub type NeonMat = ExtendedMaterial<StandardMaterial, NeonExt>;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct SkyMat {
    #[uniform(0)]
    pub tint: Vec4,
}
impl Material for SkyMat {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(SKY_SHADER)
    }
    fn specialize(
        _p: &MaterialPipeline<Self>,
        d: &mut RenderPipelineDescriptor,
        _l: &MeshVertexBufferLayoutRef,
        _k: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        d.primitive.cull_mode = None;
        Ok(())
    }
}

pub fn neon(base: StandardMaterial, mode: f32, grid: f32, step: f32, glow: f32, color: Vec4) -> NeonMat {
    ExtendedMaterial { base, extension: NeonExt { params: Vec4::new(mode, grid, step, glow), color } }
}

#[derive(Resource, Clone)]
pub struct WorldMats {
    pub terrain: Handle<NeonMat>,
    pub under: Handle<NeonMat>,
    pub props: Handle<NeonMat>,
    pub water: Handle<NeonMat>,
    pub falls: Handle<NeonMat>,
    pub solid: Handle<NeonMat>,
}

#[derive(Resource, Default)]
pub struct Focus(pub Vec3);

#[derive(Component)]
pub struct MainCam;
#[derive(Component)]
pub struct SkyDome;
#[derive(Component)]
pub struct SkyRoot;
#[derive(Component)]
pub struct Cloud {
    pub speed: f32,
}
#[derive(Component)]
pub struct Mote {
    pub phase: f32,
}
#[derive(Component)]
pub struct ChunkTag;

#[derive(Resource, Default)]
pub struct Chunks {
    pub loaded: HashMap<(i32, i32), Entity>,
    pub pending: HashMap<(i32, i32), Task<ChunkData>>,
    pub colliders: HashMap<(i32, i32), Vec<Prop>>,
}

/// Request a full world rebuild (seed change).
#[derive(Resource, Default)]
pub struct RegenWorld(pub bool);

impl Chunks {
    /// Push-out vector if circle (pos, r) overlaps a prop.
    pub fn collide(&self, pos: Vec2, r: f32) -> Option<(Vec2, f32)> {
        let (cx, cz) = ((pos.x / CHUNK).floor() as i32, (pos.y / CHUNK).floor() as i32);
        let mut best: Option<(Vec2, f32)> = None;
        for dx in -1..=1 {
            for dz in -1..=1 {
                if let Some(v) = self.colliders.get(&(cx + dx, cz + dz)) {
                    for p in v {
                        let d = pos - p.pos;
                        let l = d.length();
                        let pen = p.radius + r - l;
                        if pen > 0.0 && best.map_or(true, |b| pen > b.1) {
                            best = Some((if l > 1e-4 { d / l } else { Vec2::X }, pen));
                        }
                    }
                }
            }
        }
        best
    }
}

static START: Mutex<Option<(u32, Vec2)>> = Mutex::new(None);
pub fn start_pos() -> Vec2 {
    let s = terrain::seed();
    let mut g = START.lock().unwrap();
    if let Some((sd, p)) = *g {
        if sd == s {
            return p;
        }
    }
    let p = terrain::find_land(0.0, 0.0, 0.97, 9000.0);
    *g = Some((s, p));
    p
}

pub struct WorldPlugin;
impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        bevy::asset::load_internal_asset!(app, NEON_SHADER, "shaders/neon.wgsl", Shader::from_wgsl);
        bevy::asset::load_internal_asset!(app, SKY_SHADER, "shaders/sky.wgsl", Shader::from_wgsl);
        app.add_plugins((MaterialPlugin::<NeonMat>::default(), MaterialPlugin::<SkyMat>::default()))
            .init_resource::<Chunks>()
            .init_resource::<Focus>()
            .init_resource::<RegenWorld>()
            .add_systems(Startup, setup_world)
            .add_systems(
                Update,
                (stream_chunks, follow_cam, animate_clouds, animate_motes, apply_cvars, regen_world),
            );
    }
}

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut neon_mats: ResMut<Assets<NeonMat>>,
    mut sky_mats: ResMut<Assets<SkyMat>>,
    mut std_mats: ResMut<Assets<StandardMaterial>>,
) {
    let s = terrain::seed();
    let sd = StandardMaterial { perceptual_roughness: 0.9, reflectance: 0.15, ..default() };
    let mats = WorldMats {
        terrain: neon_mats.add(neon(sd.clone(), 0.0, 1.0, 7.0, 1.0, Vec4::new(0.1, 0.9, 1.0, 1.0))),
        under: neon_mats.add(neon(sd.clone(), 4.0, 1.0, 7.0, 1.0, Vec4::new(1.0, 0.1, 0.8, 1.0))),
        props: neon_mats.add(neon(StandardMaterial { perceptual_roughness: 0.55, ..default() }, 1.0, 0.6, 7.0, 1.0, Vec4::new(0.2, 0.9, 1.0, 1.0))),
        water: neon_mats.add(neon(
            StandardMaterial { base_color: Color::srgba(0.0, 0.05, 0.2, 0.9), alpha_mode: AlphaMode::Blend, perceptual_roughness: 0.05, ..default() },
            2.0, 1.0, 7.0, 1.0, Vec4::new(0.1, 0.9, 1.0, 1.0),
        )),
        falls: neon_mats.add(neon(
            StandardMaterial { alpha_mode: AlphaMode::Blend, double_sided: true, cull_mode: None, ..default() },
            3.0, 1.0, 7.0, 1.0, Vec4::new(0.3, 0.85, 1.0, 1.0),
        )),
        solid: neon_mats.add(neon(StandardMaterial { perceptual_roughness: 0.4, metallic: 0.3, ..default() }, 1.0, 1.0, 7.0, 1.0, Vec4::new(0.3, 0.9, 1.0, 1.0))),
    };
    commands.insert_resource(mats);

    // camera --------------------------------------------------------------
    commands.spawn((
        MainCam,
        Camera3d::default(),
        Camera { hdr: true, ..default() },
        Projection::Perspective(PerspectiveProjection { fov: 70f32.to_radians(), near: 0.1, far: 9000.0, ..default() }),
        Tonemapping::TonyMcMapface,
        Bloom { intensity: 0.28, ..Bloom::NATURAL },
        Msaa::Off,
        Fxaa::default(),
        DistanceFog {
            color: FOG_COLOR,
            directional_light_color: Color::srgb(1.0, 0.5, 0.35),
            directional_light_exponent: 24.0,
            falloff: FogFalloff::from_visibility_colors(900.0, Color::srgb(0.22, 0.05, 0.32), Color::srgb(1.0, 0.32, 0.45)),
        },
        Transform::from_xyz(0.0, 80.0, 0.0),
    ));

    // lights --------------------------------------------------------------
    commands.insert_resource(AmbientLight { color: Color::srgb(0.35, 0.15, 0.8), brightness: 70.0, ..default() });
    commands.spawn((
        DirectionalLight { color: Color::srgb(1.0, 0.62, 0.55), illuminance: 2800.0, shadows_enabled: true, ..default() },
        Transform::IDENTITY.looking_to(-SUN_DIR, Vec3::Y),
        CascadeShadowConfigBuilder { num_cascades: 3, maximum_distance: 280.0, first_cascade_far_bound: 25.0, overlap_proportion: 0.25, ..default() }.build(),
    ));

    let root = commands.spawn((SkyRoot, Transform::IDENTITY, Visibility::default())).id();
    // sky dome ------------------------------------------------------------
    commands.spawn((
        SkyDome,
        ChildOf(root),
        Mesh3d(meshes.add(Sphere::new(1.0).mesh().uv(32, 18))),
        MeshMaterial3d(sky_mats.add(SkyMat { tint: Vec4::ONE })),
        Transform::from_scale(Vec3::splat(4000.0)),
        NotShadowCaster,
        NotShadowReceiver,
    ));

    // distant mountain ridge layers --------------------------------------
    let layers = [(2300.0, 300.0, [0.10, 0.02, 0.22], 0u32), (3100.0, 420.0, [0.07, 0.01, 0.18], 1), (3800.0, 560.0, [0.04, 0.0, 0.12], 2)];
    for (r, hs, c, l) in layers {
        let seg = 256;
        let mut mb = MB::new();
        let mut prev: Option<(Vec3, f32)> = None;
        let first: Option<(Vec3, f32)>;
        let hf_ = |a: f32| -> f32 { 60.0 + ridged(a.cos() * 2.4 + l as f32 * 10.0, a.sin() * 2.4, s + 70 + l, 5).powf(1.6) * hs * 1.4 };
        let mk = |i: usize| -> (Vec3, f32) {
            let a = i as f32 / seg as f32 * std::f32::consts::TAU;
            (Vec3::new(a.cos() * r, 0.0, a.sin() * r), hf_(a))
        };
        first = Some(mk(0));
        let lc = [c[0], c[1], c[2], 0.0];
        let line = [[3.0, 0.4, 2.2, 1.0], [0.3, 1.8, 3.0, 1.0], [2.0, 0.3, 3.0, 1.0]][l as usize];
        for i in 1..=seg {
            let cur = if i == seg { first.unwrap() } else { mk(i) };
            if let Some(p) = prev {
                let (a, ha) = p;
                let (b, hb) = cur;
                mb.quad(Vec3::new(b.x, -600.0, b.z), Vec3::new(a.x, -600.0, a.z), Vec3::new(a.x, ha, a.z), Vec3::new(b.x, hb, b.z), lc);
                mb.quad(Vec3::new(b.x, hb - 9.0, b.z), Vec3::new(a.x, ha - 9.0, a.z), Vec3::new(a.x, ha, a.z), Vec3::new(b.x, hb, b.z), line);
            }
            prev = Some(cur);
        }
        commands.spawn((
            ChildOf(root),
            Mesh3d(meshes.add(mb.build())),
            MeshMaterial3d(std_mats.add(StandardMaterial { base_color: Color::WHITE, unlit: true, fog_enabled: false, cull_mode: None, ..default() })),
            Transform::default(),
            NotShadowCaster,
            NotShadowReceiver,
        ));
    }

    // distant floating islands (sky-box style) ---------------------------
    let mut rng = Rng::new(s as u64 + 99);
    let far_mat = std_mats.add(StandardMaterial { base_color: Color::WHITE, unlit: true, fog_enabled: false, ..default() });
    for k in 0..26 {
        let ang = rng.range(0.0, 6.283);
        let dist = rng.range(1700.0, 3500.0);
        let size = rng.range(90.0, 420.0);
        let y = rng.range(-120.0, 520.0);
        let mut mb = MB::new();
        let hue = rng.f();
        let rim = if hue < 0.5 { [0.2, 2.0, 3.0, 1.0] } else { [3.0, 0.3, 2.2, 1.0] };
        mb.frustum(Vec3::ZERO, size, size * 0.97, size * 0.06, 12, [0.12, 0.04, 0.3, 0.0], true);
        mb.frustum(Vec3::new(0.0, size * 0.05, 0.0), size * 0.98, size * 0.98, size * 0.015, 12, rim, false);
        mb.with_xf(Mat4::from_rotation_x(std::f32::consts::PI), |m| m.cone(Vec3::ZERO, size * 0.97, size * (0.6 + rng.f()), 10, [0.05, 0.02, 0.12, 0.0], false));
        // a few spires on top
        for _ in 0..4 {
            let (ox, oz) = (rng.range(-0.6, 0.6) * size, rng.range(-0.6, 0.6) * size);
            mb.cone(Vec3::new(ox, size * 0.06, oz), size * 0.12, size * rng.range(0.1, 0.4), 5, [0.2, 0.05, 0.4, 0.0], false);
        }
        let _ = k;
        commands.spawn((
            ChildOf(root),
            Mesh3d(meshes.add(mb.build())),
            MeshMaterial3d(far_mat.clone()),
            Transform::from_translation(Vec3::new(ang.cos() * dist, y, ang.sin() * dist)),
            NotShadowCaster,
            NotShadowReceiver,
        ));
    }

    // clouds ---------------------------------------------------------------
    let cloud_mesh = {
        let mut mb = MB::new();
        let mut r = Rng::new(5);
        for _ in 0..7 {
            let (x, z) = (r.range(-1.0, 1.0), r.range(-1.0, 1.0));
            let rad = r.range(0.4, 0.8);
            let y = r.range(-0.1, 0.1);
            mb.with_xf(Mat4::from_translation(Vec3::new(x, y, z)) * Mat4::from_scale(Vec3::new(rad, rad * 0.4, rad)), |m| {
                // low-poly blob: two stacked cones
                m.cone(Vec3::ZERO, 1.0, 0.8, 8, [1.0, 1.0, 1.0, 1.0], true);
                m.with_xf(Mat4::from_rotation_x(std::f32::consts::PI), |m2| m2.cone(Vec3::ZERO, 1.0, 0.6, 8, [1.0, 1.0, 1.0, 1.0], false));
            });
        }
        meshes.add(mb.build())
    };
    let cloud_mats = [
        std_mats.add(StandardMaterial { base_color: Color::srgba(0.9, 0.3, 0.6, 0.3), unlit: true, alpha_mode: AlphaMode::Blend, fog_enabled: false, ..default() }),
        std_mats.add(StandardMaterial { base_color: Color::srgba(0.4, 0.2, 0.9, 0.3), unlit: true, alpha_mode: AlphaMode::Blend, fog_enabled: false, ..default() }),
        std_mats.add(StandardMaterial { base_color: Color::srgba(1.0, 0.5, 0.3, 0.25), unlit: true, alpha_mode: AlphaMode::Blend, fog_enabled: false, ..default() }),
    ];
    let mut rng = Rng::new(s as u64 + 5);
    for i in 0..110 {
        let a = rng.range(0.0, 6.283);
        let d = rng.range(150.0, 2200.0);
        let below = i % 5 != 0;
        let y = if below { rng.range(-420.0, -90.0) } else { rng.range(260.0, 520.0) };
        let sc = rng.range(60.0, 260.0);
        commands.spawn((
            Cloud { speed: rng.range(3.0, 12.0) },
            Mesh3d(cloud_mesh.clone()),
            MeshMaterial3d(cloud_mats[i % 3].clone()),
            Transform::from_translation(Vec3::new(a.cos() * d, y, a.sin() * d)).with_scale(Vec3::new(sc, sc * 0.5, sc)),
            NotShadowCaster,
            NotShadowReceiver,
        ));
    }

    // floating motes -------------------------------------------------------
    let mote_mesh = meshes.add(Sphere::new(0.12).mesh().ico(1).unwrap());
    let mote_mats = [
        std_mats.add(StandardMaterial { base_color: Color::linear_rgb(0.2, 2.5, 3.0), unlit: true, ..default() }),
        std_mats.add(StandardMaterial { base_color: Color::linear_rgb(3.0, 0.3, 2.2), unlit: true, ..default() }),
    ];
    let mut rng = Rng::new(77);
    for i in 0..220 {
        commands.spawn((
            Mote { phase: rng.range(0.0, 6.28) },
            Mesh3d(mote_mesh.clone()),
            MeshMaterial3d(mote_mats[i % 2].clone()),
            Transform::from_translation(Vec3::new(rng.range(-70.0, 70.0), rng.range(-5.0, 45.0), rng.range(-70.0, 70.0))),
            NotShadowCaster,
            NotShadowReceiver,
        ));
    }
}

fn follow_cam(cam: Query<&Transform, (With<MainCam>, Without<SkyRoot>)>, mut q: Query<&mut Transform, With<SkyRoot>>) {
    let (Ok(c), Ok(mut t)) = (cam.single(), q.single_mut()) else { return };
    t.translation = Vec3::new(c.translation.x, 0.0, c.translation.z);
}

fn animate_clouds(time: Res<Time>, cam: Query<&Transform, (With<MainCam>, Without<Cloud>)>, mut q: Query<(&mut Transform, &Cloud)>) {
    let Ok(c) = cam.single() else { return };
    let dt = time.delta_secs();
    for (mut t, cl) in &mut q {
        t.translation.x += cl.speed * dt;
        let d = Vec2::new(t.translation.x - c.translation.x, t.translation.z - c.translation.z);
        if d.length() > 2300.0 {
            // re-seed to the opposite side of the camera
            t.translation.x = c.translation.x - d.x * 0.95;
            t.translation.z = c.translation.z - d.y * 0.95;
        }
    }
}

fn animate_motes(time: Res<Time>, cam: Query<&Transform, (With<MainCam>, Without<Mote>)>, mut q: Query<(&mut Transform, &Mote)>) {
    let Ok(c) = cam.single() else { return };
    let t = time.elapsed_secs();
    for (mut tr, m) in &mut q {
        tr.translation.y += (t * 0.8 + m.phase).sin() * 0.02;
        tr.translation.x += (t * 0.3 + m.phase * 2.0).cos() * 0.02;
        let d = tr.translation - c.translation;
        for (axis, lim) in [(0usize, 75.0f32), (2, 75.0)] {
            if d[axis].abs() > lim {
                tr.translation[axis] = c.translation[axis] - d[axis].signum() * (lim - 2.0);
            }
        }
        if d.y > 50.0 || d.y < -30.0 {
            tr.translation.y = c.translation.y + if d.y > 50.0 { -25.0 } else { 40.0 };
        }
    }
}

fn apply_cvars(
    cv: Res<Cvars>,
    mut cam: Query<(&mut Bloom, &mut DistanceFog, &mut Projection), With<MainCam>>,
) {
    if !cv.is_changed() {
        return;
    }
    for (mut b, mut f, mut p) in &mut cam {
        b.intensity = cv.bloom;
        f.falloff = FogFalloff::from_visibility_colors(cv.fog_vis, Color::srgb(0.22, 0.05, 0.32), Color::srgb(1.0, 0.32, 0.45));
        let _ = &mut p;
    }
}

fn regen_world(mut commands: Commands, mut regen: ResMut<RegenWorld>, mut chunks: ResMut<Chunks>) {
    if !regen.0 {
        return;
    }
    regen.0 = false;
    for (_, e) in chunks.loaded.drain() {
        commands.entity(e).despawn();
    }
    chunks.pending.clear();
    chunks.colliders.clear();
}

fn stream_chunks(
    mut commands: Commands,
    mut chunks: ResMut<Chunks>,
    focus: Res<Focus>,
    cv: Res<Cvars>,
    mats: Res<WorldMats>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // collect finished
    let keys: Vec<(i32, i32)> = chunks.pending.keys().copied().collect();
    let mut done = 0;
    for k in keys {
        if done >= 3 {
            break;
        }
        let finished = {
            let t = chunks.pending.get_mut(&k).unwrap();
            block_on(future::poll_once(t))
        };
        if let Some(data) = finished {
            chunks.pending.remove(&k);
            done += 1;
            let mut kids: Vec<Entity> = vec![];
            let mut add = |commands: &mut Commands, mesh: Mesh, mat: Handle<NeonMat>, shadow: bool| {
                let mut e = commands.spawn((Mesh3d(meshes.add(mesh)), MeshMaterial3d(mat), Transform::IDENTITY));
                if !shadow {
                    e.insert(NotShadowCaster);
                }
                kids.push(e.id());
            };
            add(&mut commands, data.top, mats.terrain.clone(), false);
            if let Some(m) = data.under {
                add(&mut commands, m, mats.under.clone(), false);
            }
            if let Some(m) = data.water {
                add(&mut commands, m, mats.water.clone(), false);
            }
            if let Some(m) = data.falls {
                add(&mut commands, m, mats.falls.clone(), false);
            }
            if let Some(m) = data.props {
                add(&mut commands, m, mats.props.clone(), true);
            }
            let parent = commands.spawn((ChunkTag, Transform::IDENTITY, Visibility::default())).add_children(&kids).id();
            chunks.loaded.insert(k, parent);
            chunks.colliders.insert(k, data.colliders);
        }
    }
    // request needed
    let (fx, fz) = ((focus.0.x / CHUNK).floor() as i32, (focus.0.z / CHUNK).floor() as i32);
    let r = cv.view_radius;
    let mut want: Vec<((i32, i32), i32)> = vec![];
    for dx in -r..=r {
        for dz in -r..=r {
            let d2 = dx * dx + dz * dz;
            if d2 <= r * r + 1 {
                want.push(((fx + dx, fz + dz), d2));
            }
        }
    }
    want.sort_by_key(|w| w.1);
    let pool = AsyncComputeTaskPool::get();
    for (k, _) in &want {
        if chunks.pending.len() >= 8 {
            break;
        }
        if chunks.loaded.contains_key(k) || chunks.pending.contains_key(k) {
            continue;
        }
        let (cx, cz) = *k;
        chunks.pending.insert(*k, pool.spawn(async move { terrain::build_chunk(cx, cz) }));
    }
    // unload
    let far: Vec<(i32, i32)> = chunks
        .loaded
        .keys()
        .filter(|(x, z)| {
            let (dx, dz) = (x - fx, z - fz);
            dx * dx + dz * dz > (r + 2) * (r + 2)
        })
        .copied()
        .collect();
    for k in far {
        if let Some(e) = chunks.loaded.remove(&k) {
            commands.entity(e).despawn();
        }
        chunks.colliders.remove(&k);
    }
    chunks.pending.retain(|(x, z), _| {
        let (dx, dz) = (x - fx, z - fz);
        dx * dx + dz * dz <= (r + 3) * (r + 3)
    });
}
