#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    forward_io::{VertexOutput, FragmentOutput},
    mesh_view_bindings::{globals, view},
}

// np: x=mode (0 terrain,1 solid glow,2 water,3 waterfall,4 underside) y=grid strength z=contour step w=glow mul
@group(2) @binding(100) var<uniform> np: vec4<f32>;
@group(2) @binding(101) var<uniform> ncol: vec4<f32>;

const SUN: vec3<f32> = vec3<f32>(0.3, 0.14, -0.94);

fn sky_col(d: vec3<f32>) -> vec3<f32> {
    let y = d.y;
    var c = mix(vec3<f32>(1.2, 0.28, 0.55), vec3<f32>(0.30, 0.04, 0.5), smoothstep(0.0, 0.35, y));
    c = mix(c, vec3<f32>(0.015, 0.0, 0.1), smoothstep(0.25, 0.9, y));
    c += vec3<f32>(1.0, 0.45, 0.1) * exp(-y * 18.0) * 0.6;
    return c;
}

fn grid_line(p: vec2<f32>, scale: f32, thick: f32) -> f32 {
    let g = p / scale;
    let d = abs(fract(g - 0.5) - 0.5) / (fwidth(g) * thick + vec2<f32>(0.0001));
    return 1.0 - min(min(d.x, d.y), 1.0);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);

    let mode = np.x;
    let wp = in.world_position.xyz;
    let t = globals.time;
    let to_cam = view.world_position.xyz - wp;
    let dist = length(to_cam);
    let vdir = to_cam / max(dist, 0.001);
    let n = normalize(in.world_normal);
    var glowv = 0.0;
#ifdef VERTEX_COLORS
    glowv = in.color.a;
#endif
    var emit = vec3<f32>(0.0);

    if mode < 0.5 {
        // ---- terrain: neon grid + contour wireframe ----
        let flat_k = smoothstep(0.6, 0.9, n.y);
        let fade = 1.0 - smoothstep(120.0, 650.0, dist);
        let g1 = grid_line(wp.xz, 8.0, 1.2) * fade * flat_k;
        let g2 = grid_line(wp.xz, 64.0, 2.0) * (1.0 - smoothstep(300.0, 1000.0, dist));
        let hv = wp.y / np.z;
        let cl = 1.0 - min(abs(fract(hv - 0.5) - 0.5) / (fwidth(hv) * 1.5 + 0.0001), 1.0);
        let cfade = 1.0 - smoothstep(400.0, 1200.0, dist);
        let ccol = mix(vec3<f32>(1.0, 0.1, 0.75), vec3<f32>(0.1, 0.9, 1.0), clamp(glowv, 0.0, 1.0));
        emit += ncol.rgb * (g1 * 0.45 + g2 * 1.3) * np.y;
        emit += ccol * cl * cfade * 1.8 * (0.35 + 0.65 * (1.0 - flat_k));
        emit *= 1.0 + 0.25 * sin(t * 2.0 + wp.x * 0.02 + wp.z * 0.02);
        // fresnel ridge glow on cliffs
        emit += ccol * pow(1.0 - max(dot(n, vdir), 0.0), 4.0) * 0.5 * (1.0 - flat_k);
        out.color = vec4<f32>(out.color.rgb + emit * np.w, out.color.a);
    } else if mode < 1.5 {
        // ---- glowing solids (props, vehicles, enemies) ----
        let base = pbr_input.material.base_color.rgb;
        let fres = pow(1.0 - max(dot(n, vdir), 0.0), 3.0);
        emit = base * glowv * 3.0 * np.w + ncol.rgb * fres * np.y;
        out.color = vec4<f32>(out.color.rgb + emit, out.color.a);
    } else if mode < 2.5 {
        // ---- reflective water ----
        var nn = n;
        nn.x += sin(wp.x * 0.6 + t * 1.5) * 0.05 + sin(wp.z * 0.37 + t * 0.8) * 0.04;
        nn.z += sin(wp.z * 0.9 - t) * 0.05 + sin(wp.x * 0.23 - t * 0.6) * 0.03;
        nn = normalize(nn);
        let r = reflect(-vdir, nn);
        let fres = pow(1.0 - max(dot(nn, vdir), 0.0), 3.0);
        let rc = sky_col(normalize(vec3<f32>(r.x, abs(r.y), r.z)));
        let sun_ref = pow(max(dot(vec3<f32>(r.x, abs(r.y), r.z), SUN), 0.0), 220.0) * 6.0;
        let gl = grid_line(wp.xz + vec2<f32>(t * 2.0, 0.0), 6.0, 1.0) * 0.4 * (1.0 - smoothstep(40.0, 400.0, dist));
        let c = mix(vec3<f32>(0.0, 0.03, 0.14), rc, 0.3 + 0.7 * fres) + vec3<f32>(sun_ref) + ncol.rgb * gl;
        out.color = vec4<f32>(c, mix(0.84, 0.97, fres));
    } else if mode < 3.5 {
        // ---- waterfall ----
        let v = wp.y * 0.12 - t * 1.4 + sin(wp.x * 0.7 + wp.z * 0.7) * 2.0;
        let streak = smoothstep(0.55, 1.0, fract(v));
        out.color = vec4<f32>(ncol.rgb * (0.8 + 2.2 * streak), 0.5 + 0.3 * streak);
    } else {
        // ---- underside rock with glowing veins ----
        let vein = smoothstep(0.92, 1.0, sin(wp.x * 0.11 + sin(wp.z * 0.07) * 3.0) * sin(wp.z * 0.13 + wp.y * 0.05));
        emit = ncol.rgb * vein * 1.6 + vec3<f32>(0.4, 0.05, 0.5) * pow(1.0 - max(dot(n, vdir), 0.0), 3.0) * 0.4;
        out.color = vec4<f32>(out.color.rgb + emit, out.color.a);
    }

    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
