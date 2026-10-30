import bevy_pbr::render::forward_io::VertexOutput;
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> effect: vec4<f32>;
@fragment fn fragment(in:VertexOutput)->@location(0) vec4<f32> {
    let p=in.uv*2.0-1.0;
    let radius=length(p);
    let angle=atan2(p.y,p.x);
    let wave=0.025*sin(angle*8.0-effect.x*6.0);
    let rim=1.0-smoothstep(0.008,0.045,abs(radius-0.77-wave));
    let sparks=pow(max(0.0,sin(angle*19.0+effect.x*9.0)),16.0)*(1.0-smoothstep(0.0,0.2,abs(radius-0.55)));
    let color=mix(vec3<f32>(0.45,0.22,0.85),vec3<f32>(0.65,0.95,1.0),rim);
    return vec4<f32>(color*(rim+sparks*0.6)*effect.y,0.0);
}
