#import bevy_pbr::{forward_io::{VertexOutput,FragmentOutput},mesh_view_bindings::view,pbr_fragment::pbr_input_from_standard_material,pbr_functions::{apply_pbr_lighting,main_pass_post_lighting_processing,alpha_discard}}
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> detail:vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var albedo:texture_2d_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var albedo_sampler:sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(103) var normals:texture_2d_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(104) var normal_sampler:sampler;
fn hash(p:vec2<f32>)->f32 {
    var q=fract(vec3<f32>(p.x,p.y,p.x)*vec3<f32>(0.1031,0.1030,0.0973));
    q+=dot(q,q.yzx+33.33);return fract((q.x+q.y)*q.z);
}
fn offset(p:vec2<f32>)->vec2<f32>{return vec2<f32>(hash(p),hash(p+vec2<f32>(17.0,31.0)))*32.0;}
fn stochastic(uv:vec2<f32>,layer:i32,dx:vec2<f32>,dy:vec2<f32>)->vec3<f32>{
    // Triangular patch blending; variance correction reduces contrast loss.
    // This is the Gaussian approximation, not full histogram LUT remapping.
    let skew=vec2<f32>(uv.x-uv.y*0.57735027,uv.y*1.15470054);
    let cell=floor(skew);let f=fract(skew);
    var w=vec3<f32>(1.0-f.x-f.y,f.x,f.y);
    var a=cell;var b=cell+vec2<f32>(1.0,0.0);var c=cell+vec2<f32>(0.0,1.0);
    if f.x+f.y>1.0 {w=vec3<f32>(f.x+f.y-1.0,1.0-f.x,1.0-f.y);a=cell+vec2<f32>(1.0);b=cell+vec2<f32>(0.0,1.0);c=cell+vec2<f32>(1.0,0.0);}
    let x=textureSampleGrad(albedo,albedo_sampler,uv+offset(a),layer,dx,dy).rgb;
    let y=textureSampleGrad(albedo,albedo_sampler,uv+offset(b),layer,dx,dy).rgb;
    let z=textureSampleGrad(albedo,albedo_sampler,uv+offset(c),layer,dx,dy).rgb;
    return clamp((x*w.x+y*w.y+z*w.z-vec3<f32>(0.214))/sqrt(dot(w,w))+vec3<f32>(0.214),vec3<f32>(0.01),vec3<f32>(1.0));
}
@fragment fn fragment(in:VertexOutput,@builtin(front_facing) front:bool)->FragmentOutput{
    var pbr=pbr_input_from_standard_material(in,front);
    let distance=length(view.world_position-in.world_position.xyz);
    let uv=in.uv*0.125;let dx=dpdx(uv);let dy=dpdy(uv);
    let near=(1.0-smoothstep(1000.0,4500.0,distance))*detail.z;
    // Interpolated material selectors must cross-fade. Rounding creates hard
    // triangular/rectangular patches wherever neighboring cover classes differ.
    let selector=clamp(in.uv_b.x,0.0,5.0);
    let layer=i32(floor(selector));let next_layer=min(layer+1,5);
    let blend=smoothstep(0.0,1.0,fract(selector));
    let body=vec3<f32>(in.uv.x,in.uv_b.y,in.uv.y)*0.125;
    let body_dx=dpdx(body);let body_dy=dpdy(body);
    if near>0.001 {
        var a=stochastic(uv,layer,dx,dy);
        var nr=textureSampleGrad(normals,normal_sampler,uv,layer,dx,dy);
        if blend>0.001 {
            a=mix(a,stochastic(uv,next_layer,dx,dy),blend);
            nr=mix(nr,textureSampleGrad(normals,normal_sampler,uv,next_layer,dx,dy),blend);
        }
        if layer==2 && blend<0.001 {
            let n=abs(normalize(cross(body_dx,body_dy)));var w=n*n*n*n;w/=max(w.x+w.y+w.z,0.0001);
            // Only steep rock pays for the extra two projections.
            let side_x=textureSampleGrad(albedo,albedo_sampler,body.zy,2,body_dx.zy,body_dy.zy).rgb;
            let side_z=textureSampleGrad(albedo,albedo_sampler,body.xy,2,body_dx.xy,body_dy.xy).rgb;
            a=a*w.y+side_x*w.x+side_z*w.z;
        }
        // CPU macro vertex colors are supplied in linear space. Detail is centered on
        // linear 0.214 (sRGB 0.5), maintaining consistent far-terrain brightness.
        let variation=clamp(a/vec3<f32>(0.214),vec3<f32>(0.35),vec3<f32>(1.9));
        pbr.material.base_color=vec4<f32>(pbr.material.base_color.rgb*mix(vec3<f32>(1.0),variation,near*0.65),pbr.material.base_color.a);
        pbr.material.perceptual_roughness=mix(0.95,clamp(nr.a,0.65,1.0),near);
        let tangent=normalize(cross(pbr.N,vec3<f32>(0.001,0.0,1.0)));let bitangent=cross(pbr.N,tangent);
        let footprint=max(length(dx),length(dy));let resolved=1.0-smoothstep(0.08,0.5,footprint);
        pbr.N=normalize(pbr.N+(tangent*(nr.r*2.0-1.0)+bitangent*(nr.g*2.0-1.0))*near*resolved*0.45);
        pbr.world_normal=pbr.N;
    }
    pbr.material.base_color=alpha_discard(pbr.material,pbr.material.base_color);
    var out:FragmentOutput;out.color=apply_pbr_lighting(pbr);out.color=main_pass_post_lighting_processing(pbr,out.color);return out;
}
