import bevy_pbr::render::forward_io::VertexOutput;
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> effect:vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> tint:vec4<f32>;
// Integer hashing avoids float precision loss and intersecting sine ribbons.
fn hash(v:vec2<i32>)->f32 {
 let p=bitcast<vec2<u32>>(v);
 var h=(p.x*1597334677u) ^ (p.y*3812015801u);
 h=(h^(h>>16u))*2246822519u;
 return f32((h^(h>>13u))&65535u)/65535.0;
}
fn noise(p:vec2<f32>)->f32 {
 let i=vec2<i32>(floor(p));let f=fract(p);let u=f*f*f*(f*(f*6.0-15.0)+10.0);
 return mix(mix(hash(i),hash(i+vec2<i32>(1,0)),u.x),mix(hash(i+vec2<i32>(0,1)),hash(i+vec2<i32>(1,1)),u.x),u.y);
}
fn density(p:vec2<f32>)->f32 {
 let rot=mat2x2<f32>(0.8,0.6,-0.6,0.8);let q=rot*p*2.03;
 return noise(p)*0.54+noise(q+vec2<f32>(17.1,9.2))*0.27+noise(rot*q*2.07+vec2<f32>(7.4,22.1))*0.13+noise(p*8.17)*0.06;
}
@fragment fn fragment(in:VertexOutput)->@location(0) vec4<f32> {
 let t=effect.x;let seed=vec2<f32>(effect.w,effect.w*0.37);
 let p=in.uv*vec2<f32>(6.5,2.7)+seed-vec2<f32>(t*0.045,t*0.012);
 let warp=vec2<f32>(density(p*0.6+vec2<f32>(0.0,t*0.025)),density(p*0.6+vec2<f32>(8.3,4.7)));
 let n=density(p+warp*0.65);
 // A wide lower tail extends below the supporting shelf. Foreground terrain
 // occludes it naturally; an exposed drop gets a soft irregular falloff.
 let center=0.48+0.18*(density(vec2<f32>(p.x*0.65,t*0.018)+seed)-0.5);
 let height=(in.uv.y-center)/0.42;
 let vertical=exp(-height*height*3.8);
 let ends=smoothstep(0.0,0.28,in.uv.x)*(1.0-smoothstep(0.72,1.0,in.uv.x));
 let boundary=smoothstep(0.0,0.34,in.uv.y)*(1.0-smoothstep(0.64,1.0,in.uv.y));
 let wisps=0.26+0.34*smoothstep(0.10,0.90,n);
 return vec4<f32>(mix(tint.xyz*0.90,tint.xyz,n),wisps*vertical*ends*boundary*effect.y*0.85);
}
