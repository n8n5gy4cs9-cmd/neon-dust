#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::{view, globals},
}

@group(2) @binding(0) var<uniform> tint: vec4<f32>;

const SUN: vec3<f32> = vec3<f32>(0.3, 0.14, -0.94);

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let d = normalize(in.world_position.xyz - view.world_position.xyz);
    let y = d.y;
    let t = globals.time;
    var c: vec3<f32>;
    if y >= 0.0 {
        c = mix(vec3<f32>(1.2, 0.28, 0.55), vec3<f32>(0.30, 0.04, 0.5), smoothstep(0.0, 0.35, y));
        c = mix(c, vec3<f32>(0.015, 0.0, 0.1), smoothstep(0.25, 0.9, y));
        c += vec3<f32>(1.0, 0.45, 0.1) * exp(-y * 18.0) * 0.6;
        // stars
        let p = d.xz / (y + 0.35) * 90.0;
        let h = hash(floor(p));
        let star = step(0.988, h) * smoothstep(0.12, 0.5, y) * (0.6 + 0.4 * sin(t * 3.0 + h * 50.0));
        c += vec3<f32>(0.8, 0.9, 1.0) * star * 1.5;
    } else {
        let a = smoothstep(-0.6, 0.0, y);
        c = mix(vec3<f32>(0.05, 0.01, 0.12), vec3<f32>(0.9, 0.2, 0.5), a * a);
        // faint infinite grid sea
        let p = d.xz / max(-y, 0.02);
        let g = abs(fract(p * 0.6 - 0.5) - 0.5) / (fwidth(p * 0.6) + vec2<f32>(0.0001));
        let line = 1.0 - min(min(g.x, g.y), 1.0);
        c += vec3<f32>(0.1, 0.8, 1.0) * line * 0.15 * smoothstep(-0.25, -0.02, y) * (1.0 - smoothstep(-0.02, 0.0, y));
    }
    // retro sun with cut stripes
    let sd = dot(d, SUN);
    let ang = acos(clamp(sd, -1.0, 1.0));
    let R = 0.11;
    let rel = (y - SUN.y) / R;
    let low = clamp(1.0 - smoothstep(-0.9, 0.2, rel), 0.0, 0.9);
    let keep = step(low, fract(rel * 6.0));
    let sun_c = mix(vec3<f32>(1.0, 0.9, 0.2), vec3<f32>(1.0, 0.1, 0.6), clamp(0.5 - rel * 0.5, 0.0, 1.0)) * 3.0;
    c += vec3<f32>(1.0, 0.35, 0.5) * pow(max(sd, 0.0), 30.0) * 0.8;
    c = mix(c, sun_c, keep * (1.0 - smoothstep(R * 0.96, R, ang)));
    return vec4<f32>(c * tint.rgb, 1.0);
}
