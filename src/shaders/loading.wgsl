// Kaleidoscope loading splash. Raymarch structure after "Active Matter loading splash"
// (Gruberk, 2026; free to use/modify, not for resale as a standalone product).
// The mapping from the march to the final colour is our own reconstruction; PI is tunable.
#import bevy_ui::ui_vertex_output::UiVertexOutput

@group(1) @binding(0) var<uniform> p: vec4<f32>; // x=time y=progress z=PI-ish constant w=unused

fn kal_march(ray_dir: vec3<f32>, rot: mat2x2<f32>, t: f32) -> vec4<f32> {
    var acc = vec4<f32>(0.0);
    var ray_dist = 0.0;
    var s = 0.0;
    for (var i = 1.0; i <= 30.0; i += 1.0) {
        var rp = ray_dir * ray_dist;
        rp.z -= 10.0;
        let rxz = rot * vec2<f32>(rp.x, rp.z);
        rp.x = rxz.x;
        rp.z = rxz.y;
        s = 0.04;
        for (; s < 3.0; s += s) {
            rp += cos(2.0 * t + rp.yzx / 10.0) * 0.6;
            rp -= vec3<f32>(abs(dot(sin(0.03 * rp.z + t + rp / s / 3.2), vec3<f32>(s, s, s))));
        }
        rp = vec3<f32>(rp.xy / 4.0, rp.z);
        s = 0.08 + 0.5 * abs(length(rp) - 30.0);
        ray_dist += s;
        acc += vec4<f32>(3.9, 2.0, 1.0, 0.0) / s * ray_dist + 10.0 * (1.0 + cos(i * 0.4 + vec4<f32>(2.0, 1.0, 0.0, 0.0))) / s;
    }
    return acc;
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let size = in.size;
    let pos = in.uv * size;
    let uv = (pos * 2.0 - size) / size.y;
    let time = p.x;
    let PI = p.z;
    let a = time * 0.12 + sin(time * 0.21) * PI * 0.25;
    let rot = mat2x2<f32>(cos(a), sin(a), -sin(a), cos(a));
    let ray_dir = normalize(vec3<f32>(uv, 1.0));
    let acc = kal_march(ray_dir, rot, time * 0.35);
    var col = 1.0 - exp(-acc.rgb / 9000.0);
    // synthwave grade: push toward magenta/cyan, vignette
    col = pow(col, vec3<f32>(1.25));
    col = mix(vec3<f32>(dot(col, vec3<f32>(0.3, 0.5, 0.2))), col, 1.4) * vec3<f32>(1.1, 0.8, 1.2);
    let v = 1.0 - 0.45 * dot(uv * 0.5, uv * 0.5);
    return vec4<f32>(col * v, 1.0);
}
