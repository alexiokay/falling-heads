import bevy_pbr::render::forward_io::VertexOutput;
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> effect:vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> exposed:vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var<uniform> flow:vec4<f32>;

fn hash(p:vec2<f32>)->f32 { return fract(sin(dot(p,vec2<f32>(127.1,311.7)))*43758.5453); }
fn noise(p:vec2<f32>)->f32 {
    let i=floor(p); let f=fract(p); let u=f*f*(3.0-2.0*f);
    return mix(mix(hash(i),hash(i+vec2<f32>(1.0,0.0)),u.x),
        mix(hash(i+vec2<f32>(0.0,1.0)),hash(i+vec2<f32>(1.0,1.0)),u.x),u.y);
}
@fragment fn fragment(in:VertexOutput)->@location(0) vec4<f32> {
    let p=vec2<f32>((in.uv.x-0.5)*96.0,(0.5-in.uv.y)*112.0+12.0);
    // Touching faces share world-space flow. Only open sides spill beyond a tile.
    if (p.x < -32.0 && exposed.x < 0.5) || (p.x > 32.0 && exposed.y < 0.5)
        || (p.y < -32.0 && exposed.z < 0.5) || (p.y > 32.0 && exposed.w < 0.5) { discard; }
    let t=effect.x;
    let impact=effect.z;
    // Steering changes the curl, never time's phase: no texture jumps when
    // the dragon changes aim or impact transitions into a lingering burn.
    let base=in.world_position.xy*vec2<f32>(0.044,0.025)-vec2<f32>(0.4,2.7)*t;
    let warp=noise(base*0.75+vec2<f32>(t*0.35,0.0));
    let q=base-flow.xy*warp*1.3*impact;
    let broad=noise(q+vec2<f32>(warp*0.8,0.0));
    let detail=noise(q*2.5+vec2<f32>(0.0,-t*1.6));
    let turbulence=broad*0.72+detail*0.28;
    // Erode the boundary beside an unheated neighbor. Two heated faces join
    // seamlessly; neither draws an artificial flame frame at their shared edge.
    let connected=u32(flow.z+0.5);
    let softness=5.0+13.0*detail;
    var boundary=1.0;
    if exposed.x<0.5 && (connected & 1u)==0u {boundary*=smoothstep(0.0,softness,p.x+32.0);}
    if exposed.y<0.5 && (connected & 2u)==0u {boundary*=smoothstep(0.0,softness,32.0-p.x);}
    if exposed.z<0.5 && (connected & 4u)==0u {boundary*=smoothstep(0.0,softness,p.y+32.0);}
    if exposed.w<0.5 && (connected & 8u)==0u {boundary*=smoothstep(0.0,softness,32.0-p.y);}
    let outer=(1.0-smoothstep(33.0,48.0,abs(p.x)))
        *(1.0-smoothstep(45.0+14.0*broad,68.0,p.y))*smoothstep(-44.0,-31.0,p.y);
    let wash=smoothstep(0.17,0.75,turbulence);
    let ember=smoothstep(0.38,0.8,turbulence)*(1.0-smoothstep(8.0,54.0,p.y));
    let coverage=mix(ember,wash,impact);
    // Open turbulent holes as the source ends, without scaling the silhouette.
    let dissolve=smoothstep((1.0-effect.y)*0.62,(1.0-effect.y)*0.62+0.22,turbulence);
    let alpha=outer*boundary*coverage*dissolve*effect.y*0.86;
    let hot=smoothstep(0.38,0.86,turbulence);
    let color=mix(vec3<f32>(1.3,0.075,0.008),vec3<f32>(3.0,1.65,0.35),hot);
    return vec4<f32>(color,alpha);
}
