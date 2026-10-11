// Neon city skyline reflected in rippling water (own implementation, fdffz2-style band).
#import bevy_ui::ui_vertex_output::UiVertexOutput

@group(1) @binding(0) var<uniform> p: vec4<f32>; // x=time y=alpha

fn h1(x: f32) -> f32 { return fract(sin(x * 127.1) * 43758.5453); }

fn skyline(x: f32, t: f32, layer: f32) -> vec4<f32> {
    // returns rgb in xyz, coverage in w for given horizontal position (0..1.. wide)
    let scale = 14.0 + layer * 9.0;
    let xs = x * scale + t * (0.05 + layer * 0.03) + layer * 17.0;
    let cell = floor(xs);
    let f = fract(xs);
    let hh = 0.12 + h1(cell + layer * 3.0) * (0.55 - layer * 0.12);
    let w = 0.55 + h1(cell * 1.7) * 0.4;
    let inside = step(abs(f - 0.5), w * 0.5);
    return vec4<f32>(hh, f, inside, cell);
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let t = p.x;
    let uv = in.uv;                       // y: 0 top .. 1 bottom of the band
    let aspect = in.size.x / in.size.y;
    let horizon = 0.5;
    let x = (uv.x - 0.5) * aspect;
    var col = vec3<f32>(0.0);
    var alpha = 0.0;
    let water = uv.y > horizon;
    // mirrored vertical coordinate (distance above/below horizon, upward positive)
    var dy = horizon - uv.y;
    var ripple = 0.0;
    var xx = x;
    if water {
        dy = -dy;
        ripple = sin(uv.y * 90.0 - t * 2.5) * 0.004 * (uv.y - horizon) * 14.0 + sin(x * 25.0 + t) * 0.003;
        xx = x + sin(uv.y * 60.0 + t * 1.5) * 0.012 * (uv.y - horizon);
        dy += ripple;
    }
    // three skyline layers
    for (var l = 0; l < 3; l += 1) {
        let layer = f32(l);
        let s = skyline(xx, t, layer);
        let top = s.x * 0.5 * (1.0 - layer * 0.12);
        if s.z > 0.5 && dy < top && dy > -0.0001 {
            let fade = 1.0 - layer * 0.28;
            var c = vec3<f32>(0.05, 0.0, 0.14) * fade;
            // lit windows
            let wy = dy * 120.0;
            let wx = s.y * 8.0;
            let win = step(0.55, h1(floor(wy) + s.w * 7.0 + floor(wx) * 3.1)) * step(0.35, fract(wy)) * step(0.3, fract(wx)) * step(fract(wx), 0.7);
            let flick = 0.6 + 0.4 * sin(t * (1.0 + h1(s.w)) + s.w);
            let wc = mix(vec3<f32>(1.0, 0.2, 0.8), vec3<f32>(0.1, 0.9, 1.0), h1(s.w * 3.3));
            c += wc * win * 1.2 * flick * fade;
            // roof neon strip
            c += mix(vec3<f32>(1.0, 0.1, 0.6), vec3<f32>(0.1, 0.9, 1.0), h1(s.w + 9.0)) * smoothstep(0.012, 0.0, top - dy) * 2.0;
            col = c;
            alpha = 1.0;
        }
    }
    if water {
        // water base: dark gradient, sun streaks, grid shimmer
        let k = (uv.y - horizon) / (1.0 - horizon);
        var wc = mix(vec3<f32>(0.25, 0.02, 0.3), vec3<f32>(0.01, 0.0, 0.06), k);
        let streak = exp(-abs(x) * 6.0) * (0.5 + 0.5 * sin(uv.y * 140.0 - t * 3.0 + sin(x * 20.0) * 2.0));
        wc += vec3<f32>(1.0, 0.45, 0.2) * streak * (1.0 - k) * 0.7;
        let lines = smoothstep(0.03, 0.0, abs(fract(uv.y * 14.0 + t * 0.1 + k * 6.0) - 0.5) - 0.46);
        wc += vec3<f32>(0.1, 0.8, 1.0) * lines * 0.05;
        if alpha > 0.5 {
            // reflection: dim the mirrored city and tint it
            col = mix(wc, col * 0.8 + vec3<f32>(0.05, 0.0, 0.1), 0.65);
        } else {
            col = wc;
        }
        alpha = 0.92;
        // fade the very bottom to dark
        col *= 1.0 - 0.5 * k;
    }
    return vec4<f32>(col, alpha * p.y);
}
