@group(0) @binding(0) var depth: texture_depth_2d;
@group(0) @binding(1) var motion: texture_2d<f32>;
@group(0) @binding(2) var depth_out: texture_storage_2d<r32float,write>;
@group(0) @binding(3) var motion_out: texture_storage_2d<rg16float,write>;
@compute @workgroup_size(8,8) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
    if any(id.xy >= textureDimensions(depth_out)) { return; }
    let p=vec2<i32>(id.xy);
    textureStore(depth_out,p,vec4<f32>(textureLoad(depth,p,0),0.,0.,0.));
    textureStore(motion_out,p,textureLoad(motion,p,0));
}
