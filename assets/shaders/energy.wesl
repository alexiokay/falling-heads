import bevy_pbr::render::forward_io::VertexOutput;
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> effect: vec4<f32>;
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Mirror impact is a brief circular ripple, independent of beam visuals.
    if effect.y>1.5 {
        let p=in.uv*2.0-1.0;let r=length(p);let front=effect.x*0.8+0.08;
        let ring=1.0-smoothstep(0.012,0.075,abs(r-front));
        return vec4<f32>(vec3<f32>(0.35,0.70,0.85),ring*effect.w*0.65);
    }
    // Pixel beams.
    let is_laser = effect.y > 0.5 && effect.y < 1.5;
    let grid = select(vec2<f32>(96.0, 12.0), vec2<f32>(600.0, 64.0), is_laser);
    let pixel = floor(in.uv * grid);
    let uv = (pixel + 0.5) / grid;
    let frame = floor(effect.z * 12.0);
    let distance = abs(uv.y - 0.5);
    let spark = fract(sin(dot(pixel + vec2<f32>(frame, 0.0), vec2<f32>(12.9898,78.233))) * 43758.5453);
    let tip = min(uv.x,1.0-uv.x);
    let width = select(0.25, 0.17, tip < 0.025);
    if distance > width && (spark < 0.94 || distance > width + 0.16) { discard; }
    var edge = vec3<f32>(0.42,0.46,0.50);
    var middle = vec3<f32>(0.84,0.88,0.92);
    if effect.y > 0.5 && effect.y < 1.5 {
        edge = vec3<f32>(0.65,0.05,0.015);
        middle = vec3<f32>(1.0,0.28,0.06);
    }
    var color = select(edge, middle, distance < 0.19);
    let core = select(vec3<f32>(1.0,0.99,0.96), vec3<f32>(1.0,0.90,0.62), is_laser);
    color = select(color, core, distance < 0.075);
    return vec4<f32>(color, effect.w * select(0.8, 1.0, distance < width));
}
