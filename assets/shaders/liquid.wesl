import bevy_pbr::render::forward_io::VertexOutput;
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> effect: vec4<f32>;
@fragment fn fragment(in:VertexOutput)->@location(0) vec4<f32> {
    let t=effect.x;
    if effect.y>1.5 {
        let p=in.uv*2.0-1.0;
        let wisps=0.5+0.5*sin(p.x*9.0+p.y*7.0-t*3.0+sin(p.y*8.0+t));
        var life = 1.0;
        @if(VERTEX_COLORS) { life = in.color.a; }
        let mask=exp(-dot(p,p)*3.0)*wisps*0.20*life;
        return vec4<f32>(vec3<f32>(0.72,0.79,0.82),mask);
    }
    let n=normalize(in.world_normal);
    let plane=mix(in.world_position.xy,in.world_position.xz,abs(n.y));
    let p=floor(plane/2.0);
    let warp=vec2<f32>(sin(p.y*0.13+t*0.7),cos(p.x*0.11-t*0.5))*2.0;
    let q=p+warp;
    let flow=sin(q.x*0.16+t*0.8)*cos(q.y*0.19+t*1.5)+sin((q.x+q.y)*0.12-t*0.5);
    let shade=clamp(flow*0.24+0.5,0.0,1.0);
    let face=0.72+0.18*max(n.z,0.0)+0.20*max(n.y,0.0);
    if effect.y>0.5 {
        let veins=pow(max(0.0,1.0-abs(flow)*2.8),3.0);
        let bubbles=pow(max(0.0,sin(q.x*0.31+q.y*0.15+t*1.8)),18.0)*pow(shade,4.0);
        let crust=mix(vec3<f32>(0.12,0.025,0.018),vec3<f32>(0.95,0.19,0.012),shade);
        return vec4<f32>((crust+vec3<f32>(0.65,0.40,0.08)*veins+vec3<f32>(0.45,0.27,0.05)*bubbles)*face*2.2,1.0);
    }
    let caustic=pow(max(0.0,1.0-abs(flow)*3.0),6.0)*0.13;
    let surface=max(n.y,0.0)*(0.05+0.10*sin(p.x*0.25+t*2.0)*sin(p.y*0.23-t));
    return vec4<f32>((mix(vec3<f32>(0.025,0.12,0.22),vec3<f32>(0.10,0.34,0.47),shade)+vec3<f32>(caustic+surface))*face,effect.z);
}
