// The CPU retains emission samples in a double-precision, nearby anchor frame.
#import bevy_pbr::{mesh_functions, forward_io::{Vertex, VertexOutput}, view_transformations::position_world_to_clip, mesh_view_bindings::view}
struct Parameters { time:vec4<f32>, head:vec4<f32> }
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> smoke:Parameters;
fn unit_or(v:vec3<f32>, fallback:vec3<f32>)->vec3<f32> {
    if dot(v,v)>1e-20 {return normalize(v);} return fallback;
}
@vertex
fn vertex(v:Vertex)->VertexOutput {
    var out:VertexOutput;
    let model=mesh_functions::get_world_from_local(v.instance_index);
    let age=max(smoke.time.x-v.color.x,0.0);
    var center=v.position;
    var tangent=v.normal;
    if v.color.w>=0.0 {tangent=smoke.head.xyz-center;center=mix(center,smoke.head.xyz,v.color.w);}
    center+=v.tangent.xyz*(age*0.40)+vec3<f32>(age*0.25,0.0,0.0);
    var world=mesh_functions::mesh_position_local_to_world(model,vec4<f32>(center,1.0)).xyz;
    let direction=mesh_functions::mesh_position_local_to_world(model,vec4<f32>(tangent,0.0)).xyz;
    let normal=unit_or(view.world_position-world,vec3<f32>(0.0,0.0,1.0));
    let along=unit_or(direction-normal*dot(direction,normal),view.world_from_view[1].xyz);
    let across=unit_or(cross(normal,along),vec3<f32>(1.0,0.0,0.0));
    let phase=v.color.y;
    let width=(0.10+0.64*sqrt(age))*(1.0+0.16*sin(phase))*smoke.time.y;
    let half_length=max(length(direction)/12.0+width*0.8,width);
    world+=across*(0.13*width*sin(phase+age));
    world+=along*half_length*(v.uv.y*2.0-1.0)+across*width*(v.uv.x*2.0-1.0);
    let fade=clamp(1.0-age/5.0,0.0,1.0);
    out.world_position=vec4<f32>(world,1.0);
    out.position=position_world_to_clip(world);
    out.world_normal=normal;
    out.world_tangent=vec4<f32>(across,1.0);
    out.uv=v.uv;
    out.color=vec4<f32>(1.0,1.0,1.0,fade*fade*0.82);
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index=v.instance_index;
#endif
    return out;
}
