import bevy_pbr::render::forward_io::VertexOutput;
import bevy_pbr::render::mesh_view_bindings::view;
// x: time, y: strength 0..1, z: mode (0 field beam, 1 atmosphere, 2 containment,
// 3 engine plume, 4 re-entry shock).
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: vec4<f32>;

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = params.x;
    let strength = params.y;
    let mode = params.z;
    if mode < 0.5 {
        // Magnetic field beam: bright rings rush from the pole (uv.x = 0) into
        // the cell, with bulging field lines and a hot core.
        let x = in.uv.x;
        let y = abs(in.uv.y - 0.5) * 2.0;
        let core = 1.0 - smoothstep(0.0, 1.0, y);
        let rings = pow(0.5 + 0.5 * sin((x * 7.0 - t * 6.0) * 6.2831), 3.0);
        let lines = 0.5 + 0.5 * sin((y * 6.0 - x * x * 3.0 - t * 1.7) * 6.2831);
        let fade = smoothstep(0.0, 0.1, x) * (1.0 - smoothstep(0.7, 1.0, x));
        let flicker = 0.85 + 0.15 * hash(vec2<f32>(floor(t * 20.0), 3.0));
        let a = (core * 0.35 + rings * core * 0.75 + lines * 0.2 * core) * fade * strength * flicker;
        let color = mix(vec3<f32>(0.2, 0.55, 1.0), vec3<f32>(0.85, 0.97, 1.0), rings * core);
        return vec4<f32>(color * 2.4, clamp(a, 0.0, 0.92));
    }
    if mode < 1.5 {
        // Atmosphere: glows where the sphere turns away from the eye.
        let n = normalize(in.world_normal);
        let eye = normalize(view.world_position - in.world_position.xyz);
        let facing = clamp(dot(n, eye), 0.0, 1.0);
        let rim = pow(1.0 - facing, 2.4);
        let haze = 0.08 * facing;
        return vec4<f32>(vec3<f32>(0.35, 0.65, 1.0) * 1.7, clamp(rim * 0.9 + haze, 0.0, 1.0));
    }
    if mode > 2.5 && mode < 3.5 {
        // Engine plume: uv.y 0 at the nozzle, 1 at the tail. A blue-white core,
        // orange flame widening and breaking up into flicker toward the tail.
        let x = abs(in.uv.x - 0.5) * 2.0;
        let y = in.uv.y;
        let width = mix(0.45, 1.0, sqrt(y));
        let body = 1.0 - smoothstep(width * 0.55, width, x);
        let core = 1.0 - smoothstep(0.0, 0.3 * width, x);
        let n = hash(vec2<f32>(floor(y * 22.0 - t * 34.0), floor(x * 7.0)));
        let shock = 0.5 + 0.5 * sin(y * 40.0 - t * 50.0);
        let fade = 1.0 - smoothstep(0.15, 1.0, y);
        var color = mix(vec3<f32>(1.0, 0.45, 0.1), vec3<f32>(1.0, 0.85, 0.5), fade);
        color = mix(color, vec3<f32>(0.75, 0.88, 1.0), core * (1.0 - y) * (0.7 + 0.3 * shock));
        let a = body * fade * (0.55 + 0.45 * n) * strength;
        return vec4<f32>(color * 3.2, clamp(a, 0.0, 1.0));
    }
    if mode > 3.5 {
        // Re-entry shock: a glowing plasma arc bowed over the nose (uv.y 0 is the
        // front), flickering orange to white-hot at its peak.
        let p = vec2<f32>(in.uv.x * 2.0 - 1.0, 1.0 - in.uv.y * 2.0);
        let d = length(p - vec2<f32>(0.0, -0.7));
        let arc = 1.0 - smoothstep(0.0, 0.32, abs(d - 1.15));
        let front = smoothstep(-0.35, 0.4, p.y);
        let n = hash(vec2<f32>(floor(atan2(p.y, p.x) * 18.0), floor(t * 24.0)));
        let hot = 1.0 - smoothstep(0.0, 0.12, abs(d - 1.15));
        let color = mix(vec3<f32>(1.0, 0.4, 0.08), vec3<f32>(1.0, 0.95, 0.85), hot);
        let a = arc * front * (0.6 + 0.4 * n) * strength;
        return vec4<f32>(color * 3.5, clamp(a, 0.0, 1.0));
    }
    // Containment field: a shimmering band of cells around the spinning prison,
    // brighter while a magnet pulls.
    let p = in.uv * 2.0 - 1.0;
    let r = length(p);
    let band = smoothstep(0.86, 0.94, r) * (1.0 - smoothstep(0.97, 1.0, r));
    let ang = atan2(p.y, p.x);
    let cells = 0.5 + 0.5 * sin(ang * 40.0 + t * 1.3) * sin(r * 70.0 - t * 5.0);
    let flicker = 0.7 + 0.3 * hash(vec2<f32>(floor(ang * 10.0), floor(t * 9.0)));
    let a = band * (0.06 + 0.26 * cells) * flicker * (0.4 + 0.6 * strength);
    return vec4<f32>(vec3<f32>(0.4, 0.8, 1.0) * 1.9, a);
}
