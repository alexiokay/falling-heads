import bevy_pbr::render::{pbr_functions::alpha_discard,pbr_fragment::pbr_input_from_standard_material};
@if(PREPASS_PIPELINE)
import bevy_pbr::{prepass::io::{VertexOutput,FragmentOutput},deferred::functions::deferred_output};
@else
import bevy_pbr::render::{forward_io::{VertexOutput,FragmentOutput},pbr_functions::{apply_pbr_lighting,main_pass_post_lighting_processing},pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT};
@group(constants::MATERIAL_BIND_GROUP) @binding(100) var cracks:texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var cracks_sampler:sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var<uniform> parameters:vec4<f32>;
@fragment fn fragment(in:VertexOutput,@builtin(front_facing) front:bool)->FragmentOutput {
 var pbr=pbr_input_from_standard_material(in,front);
 // World-space face coordinates keep cracks consistent on atlas and unit cubes.
 let n=abs(in.world_normal);var uv=fract((in.world_position.xy+vec2(32.))/64.);
 if n.x>0.7 {uv=fract((in.world_position.zy+vec2(32.))/64.);}else if n.y>0.7 {uv=fract((in.world_position.xz+vec2(32.))/64.);}
 if parameters.x>1.5 {uv=vec2(1.-uv.y,uv.x);}else if parameters.x>0.5 {uv.x=1.-uv.x;}
 let core=textureSample(cracks,cracks_sampler,uv).a;
 let px=1./vec2<f32>(textureDimensions(cracks));
 let edge=max(max(textureSample(cracks,cracks_sampler,uv+vec2(px.x,0.)).a,textureSample(cracks,cracks_sampler,uv+vec2(0.,px.y)).a)-core,0.);
 pbr.material.base_color=vec4(mix(pbr.material.base_color.rgb,vec3(0.025,0.02,0.015),core*.97)+vec3(edge*.12),pbr.material.base_color.a);
 pbr.material.base_color=alpha_discard(pbr.material.flags,pbr.material.alpha_cutoff,pbr.material.base_color);
 @if(PREPASS_PIPELINE)
 let out=deferred_output(in,pbr);
 @if(!PREPASS_PIPELINE)
 var out:FragmentOutput;
 @if(!PREPASS_PIPELINE)
 if (pbr.material.flags&STANDARD_MATERIAL_FLAGS_UNLIT_BIT)==0u {out.color=apply_pbr_lighting(pbr);}else{out.color=pbr.material.base_color;}
 @if(!PREPASS_PIPELINE)
 out.color=main_pass_post_lighting_processing(pbr,out.color);
 return out;
}
