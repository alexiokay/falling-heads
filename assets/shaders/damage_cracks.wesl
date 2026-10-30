import bevy_pbr::render::forward_io::VertexOutput;
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var cracks: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var cracks_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var<uniform> variant:vec4<f32>;
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv=in.uv;
    if variant.x>1.5 {uv=vec2<f32>(1.0-uv.y,uv.x);uv+=vec2<f32>(sin(uv.y*11.0),sin(uv.x*8.0))*0.055;}
    else if variant.x>0.5 {uv=vec2<f32>(1.0-uv.x,uv.y);uv+=vec2<f32>(sin(uv.y*7.0),cos(uv.x*9.0))*0.07;}
    uv=clamp(uv,vec2<f32>(0.0),vec2<f32>(1.0));
    let px = 1.0 / vec2<f32>(textureDimensions(cracks));
    let core = textureSample(cracks, cracks_sampler, uv).a;
    var outline = core;
    for (var y = -1; y <= 1; y += 1) {
        for (var x = -1; x <= 1; x += 1) {
            outline = max(outline, textureSample(cracks, cracks_sampler, uv + vec2<f32>(f32(x), f32(y)) * px).a);
        }
    }
    // A dark split plus a pale chipped edge reads on both light stone and dark ores.
    let edge = max(outline - core, 0.0);
    return vec4<f32>(mix(vec3<f32>(0.48, 0.46, 0.40), vec3<f32>(0.035, 0.028, 0.022), core), core * 0.96 + edge * 0.38);
}
