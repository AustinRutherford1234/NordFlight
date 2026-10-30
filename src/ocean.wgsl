#import bevy_pbr::{mesh_functions,forward_io::{Vertex,VertexOutput,FragmentOutput},view_transformations::position_world_to_clip,mesh_view_bindings::view,pbr_fragment::pbr_input_from_standard_material,pbr_functions::{apply_pbr_lighting,main_pass_post_lighting_processing}}
struct Parameters {center:vec4<f32>,detail:vec4<f32>,axis_x:vec4<f32>,axis_z:vec4<f32>,waves:array<vec4<f32>,24>,phases:array<vec4<f32>,24>}
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> ocean:Parameters;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var ripple:texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var ripple_sampler:sampler;
fn strength(p:vec2<f32>)->f32 {return 0.10+0.90*(1.0-smoothstep(6500.0,7000.0,abs(p.x-20000.0)))*(1.0-smoothstep(-8500.0,-8000.0,p.y));}
@vertex fn vertex(v:Vertex)->VertexOutput {
    var out:VertexOutput;
    let model=mesh_functions::get_world_from_local(v.instance_index);
    let xz=v.position.xz;let absolute=xz+ocean.center.xy;
    let y=sqrt(max(ocean.center.w*ocean.center.w-dot(absolute,absolute),1.0));
    // Algebraic difference of roots avoids metre-scale cancellation at Earth's radius.
    let curve=-(2.0*dot(ocean.center.xy,xz)+dot(xz,xz))/(y+ocean.center.z);
    let radial=normalize(vec3<f32>(absolute.x,y,absolute.y));
    var height=0.0;var slope=vec2<f32>(0.0);var curvature=0.0;
    // Short waves fade from geometry before their local grid cells undersample
    // them; fine normal detail is filtered independently in the fragment stage.
    let cell=max(0.10,length(xz)*0.055);
    for(var i=0u;i<24u;i+=1u){let w=ocean.waves[i];let wavelength=6.2831853/length(w.xy);
        let resolved=1.0-smoothstep(wavelength*0.18,wavelength*0.48,cell);
        let phase=dot(w.xy,xz)+ocean.phases[i].x;let s=sin(phase);let c=cos(phase);
        let h=(w.z*c+w.w*s)*resolved; height+=h;slope+=w.xy*(-w.z*s+w.w*c)*resolved;curvature+=dot(w.xy,w.xy)*h;}
    let weight=strength(absolute);
    let grad=vec2<f32>(strength(absolute+vec2<f32>(1.0,0.0))-strength(absolute-vec2<f32>(1.0,0.0)),strength(absolute+vec2<f32>(0.0,1.0))-strength(absolute-vec2<f32>(0.0,1.0)))*0.5;
    slope=slope*weight+height*grad;
    let position=vec3<f32>(xz.x,curve,xz.y)+radial*(0.12+height*weight);
    out.world_position=mesh_functions::mesh_position_local_to_world(model,vec4<f32>(position,1.0));
    out.position=position_world_to_clip(out.world_position.xyz);
    out.world_normal=mesh_functions::mesh_normal_local_to_world(normalize(radial-vec3<f32>(slope.x,0.0,slope.y)),v.instance_index);
    out.uv=absolute;
    out.color=vec4<f32>(height*weight,curvature*weight,weight,1.0);
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index=v.instance_index;
#endif
    return out;
}
@fragment fn fragment(in:VertexOutput,@builtin(front_facing) is_front:bool)->FragmentOutput {
    var clean=in;clean.color=vec4<f32>(1.0);
    var pbr=pbr_input_from_standard_material(clean,is_front);
    let distance=length(view.world_position-in.world_position.xyz);
    // Hardware mip filtering integrates the short-wave slope field. Scrolling
    // two rotated scales breaks repetition without per-pixel trigonometric loops.
    let warp=vec2<f32>(sin(dot(in.uv,vec2<f32>(0.013,0.009))),sin(dot(in.uv,vec2<f32>(0.017,-0.007))))*0.47;
    let uv1=in.uv/96.0+warp+vec2<f32>(ocean.detail.x,ocean.detail.y)*0.0625;
    let uv2=vec2<f32>(in.uv.x*0.8-in.uv.y*0.6,in.uv.x*0.6+in.uv.y*0.8)/37.0-warp.yx*0.7+vec2<f32>(-ocean.detail.y,ocean.detail.x)*0.0625;
    let a=textureSample(ripple,ripple_sampler,uv1).rg*2.0-1.0;
    let b=textureSample(ripple,ripple_sampler,uv2).rg*2.0-1.0;
    let fine=(a+vec2<f32>(b.x*0.8+b.y*0.6,-b.x*0.6+b.y*0.8)*0.55)*in.color.z*(1.0-smoothstep(1500.0,6000.0,distance));
    pbr.N=normalize(pbr.N-ocean.axis_x.xyz*fine.x-ocean.axis_z.xyz*fine.y);
    pbr.world_normal=pbr.N;
    let crest=smoothstep(0.75,1.55,in.color.x)*smoothstep(0.006,0.025,in.color.y);
    let breakup=smoothstep(-0.02,0.22,a.x+b.y);
    let foam=crest*breakup*0.65*(1.0-smoothstep(2500.0,8000.0,distance));
    let trough=smoothstep(-1.1,0.8,in.color.x);
    pbr.material.base_color=vec4<f32>(mix(vec3<f32>(0.006,0.025,0.038),vec3<f32>(0.012,0.065,0.079),trough),pbr.material.base_color.a);
    pbr.material.base_color=vec4<f32>(mix(pbr.material.base_color.rgb,vec3<f32>(0.62,0.74,0.73),foam),pbr.material.base_color.a);
    // Subpixel ripples become roughness rather than unstable glitter.
    pbr.material.perceptual_roughness=mix(0.19+0.08*smoothstep(150.0,4000.0,distance),0.65,foam);
    var out:FragmentOutput;out.color=apply_pbr_lighting(pbr);
    out.color=main_pass_post_lighting_processing(pbr,out.color);return out;
}
