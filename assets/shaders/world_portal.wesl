import bevy_pbr::render::forward_io::VertexOutput;
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> effect:vec4<f32>;
fn noise(p:vec2<f32>)->f32 {return 0.5+0.24*sin(p.x*1.73+p.y*1.11+sin(p.y*2.31))+0.16*sin(p.x*3.17-p.y*2.43)+0.07*sin(p.x*5.13+p.y*4.17);}
@fragment fn fragment(in:VertexOutput)->@location(0) vec4<f32> {
 let p=(in.uv-0.5)*2.0;let t=effect.x;
 let r=length(p*vec2<f32>(1.0,0.8));
 let angle=atan2(p.y,p.x)+r*4.0-t*0.65;
 let n=noise(vec2<f32>(cos(angle),sin(angle))*r*4.0+vec2<f32>(t*0.2,0.0));
 let edge=(1.0-smoothstep(0.72,1.0,abs(p.x)))*(1.0-smoothstep(0.78,1.0,abs(p.y)));
 let ribbon=smoothstep(0.49,0.8,n);
 return vec4<f32>(mix(vec3<f32>(0.16,0.06,0.25),vec3<f32>(0.66,0.35,0.9),ribbon),edge*(0.10+ribbon*0.62));
}
