import bevy_pbr::render::forward_io::VertexOutput;
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> effect:vec4<f32>;
@fragment fn fragment(in:VertexOutput)->@location(0) vec4<f32> {
    let u=in.uv.x; let v=(in.uv.y-0.5)*2.0;
    let wave=sin(u*28.0-effect.x*17.0)*0.06+sin(u*51.0-effect.x*24.0)*0.025;
    let width=0.08+u*0.8;
    let edge=1.0-smoothstep(width*0.5,width,abs(v+wave*u));
    let pulse=0.76+0.24*sin(u*43.0-effect.x*23.0+v*7.0);
    let core=1.0-smoothstep(0.0,width*0.65,abs(v+wave*u));
    let alpha=edge*pulse*effect.y*(1.0-smoothstep(0.83,1.0,u));
    return vec4<f32>(mix(vec3<f32>(2.3,0.12,0.01),vec3<f32>(4.4,2.0,0.35),core*(1.0-u*0.7)),alpha);
}
