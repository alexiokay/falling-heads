import bevy_pbr::render::forward_io::VertexOutput;
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> effect:vec4<f32>;
@fragment fn fragment(in:VertexOutput)->@location(0) vec4<f32> {
    let y=1.0-in.uv.y;
    let sway=sin(effect.x*6.4-y*5.0)*0.07*y+sin(effect.x*11.1+y*9.0)*0.025*y;
    let x=(in.uv.x-0.5)*2.0-sway;
    let width=0.56*pow(max(0.0,1.0-y),0.7)*(0.92+sin(effect.x*8.0+y*14.0)*0.08);
    let edge=1.0-smoothstep(width*0.68,width+0.06,abs(x));
    let alpha=edge*smoothstep(0.0,0.10,y)*(1.0-smoothstep(0.82,1.0,y));
    let core=(1.0-smoothstep(0.0,width*0.7,abs(x)))*(1.0-smoothstep(0.15,0.65,y));
    let colour=mix(vec3<f32>(1.8,0.24,0.025),vec3<f32>(3.4,1.9,0.5),core);
    return vec4<f32>(colour,alpha);
}
