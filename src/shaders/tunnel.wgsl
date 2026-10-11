// Neon tunnel (own implementation, tXGGRD-style): polar tunnel, rings + spokes, palette cycling.
#import bevy_ui::ui_vertex_output::UiVertexOutput

@group(1) @binding(0) var<uniform> p: vec4<f32>; // x=time y=alpha z=speed w=mode (0 boot, 1 ending)

const TAU: f32 = 6.2831853;

fn pal(x: f32) -> vec3<f32> {
    // cyan -> magenta -> orange -> cyan
    let a = vec3<f32>(0.1, 0.95, 1.0);
    let b = vec3<f32>(1.0, 0.1, 0.8);
    let c = vec3<f32>(1.0, 0.55, 0.1);
    let k = fract(x) * 3.0;
    if k < 1.0 { return mix(a, b, k); }
    if k < 2.0 { return mix(b, c, k - 1.0); }
    return mix(c, a, k - 2.0);
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let size = in.size;
    var uv = (in.uv * size * 2.0 - size) / size.y;
    let t = p.x;
    // gentle camera sway
    uv += vec2<f32>(sin(t * 0.7) * 0.12, cos(t * 0.5) * 0.08);
    let r = length(uv);
    let a = atan2(uv.y, uv.x);
    let depth = 1.0 / max(r, 0.02) * 0.35 + t * p.z;
    // square-ish tunnel walls: blend circular and boxy radius
    let box_r = max(abs(uv.x), abs(uv.y));
    let rr = mix(r, box_r, 0.55);
    let d2 = 1.0 / max(rr, 0.02) * 0.35 + t * p.z;
    let twist = a / TAU * 12.0 + d2 * 0.12 * select(1.0, -1.0, p.w > 0.5);
    // rings
    let ring = abs(fract(d2 * 1.4) - 0.5);
    let ring_l = smoothstep(0.06, 0.0, ring) + 0.35 * smoothstep(0.25, 0.0, ring);
    // spokes
    let sp = abs(fract(twist) - 0.5);
    let spoke_l = smoothstep(0.045, 0.0, sp) * smoothstep(0.0, 0.25, rr);
    let col_r = pal(d2 * 0.07 + t * 0.05);
    let col_s = pal(d2 * 0.07 + 0.5 + t * 0.05);
    var col = col_r * ring_l * 1.6 + col_s * spoke_l * 1.1;
    // far glow in the centre
    col += pal(t * 0.1) * 0.9 * exp(-rr * 5.0);
    col += vec3<f32>(0.9, 0.6, 1.0) * exp(-rr * 22.0) * 1.5;
    // checker panels between lines
    let chk = step(0.5, fract(d2 * 0.7)) * step(0.5, fract(twist * 0.5));
    col += pal(d2 * 0.05) * chk * 0.05;
    // darken toward the screen edge, add scanlines
    col *= smoothstep(0.0, 0.35, rr);
    col *= 0.92 + 0.08 * sin(in.uv.y * size.y * 1.5);
    col *= 1.0 - 0.35 * dot(uv * 0.35, uv * 0.35);
    return vec4<f32>(col, p.y);
}
