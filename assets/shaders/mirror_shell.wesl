import bevy_pbr::render::forward_io::VertexOutput;
import bevy_pbr::render::mesh_view_bindings::view;
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> effect: vec4<f32>;
// Quiet centre, turbulent luminous edge. The surface moves; the shell never trails.
@fragment fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let n=normalize(in.world_normal);
    let eye=normalize(view.world_position-in.world_position.xyz);
    let facing=clamp(dot(n,eye),0.0,1.0);
    let rim=pow(1.0-facing,3.0);
    let t=effect.x;
    let angle=atan2(n.y,n.x);
    let swirl=sin(angle*9.0-t*1.6+sin(angle*5.0+t*0.7));
    let filaments=pow(max(0.0,sin(angle*23.0+t*3.2+swirl*2.0)),8.0);
    let crest=pow(1.0-facing,9.0)*(0.55+0.45*filaments);
    let veil=pow(max(0.0,sin(n.x*17.0+n.y*11.0-t+swirl)),12.0)*rim*0.07;
    let travelling=smoothstep(0.92,0.995,sin(facing*43.0-angle*3.0+t*2.3+swirl))*pow(1.0-facing,1.7);
    let energy=rim*(0.30+0.12*swirl)+crest*0.62+veil+travelling*0.20;
    let color=mix(vec3<f32>(0.15,0.48,0.70),vec3<f32>(0.70,1.10,1.08),crest);
    return vec4<f32>(color*(1.0+crest*0.8),energy*effect.y);
}
