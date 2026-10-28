#import bevy_pbr::{forward_io::VertexOutput, mesh_view_bindings::{view,view_transmission_texture,view_transmission_sampler}}
@fragment
fn fragment(in:VertexOutput)->@location(0) vec4<f32> {
    let size=vec2<f32>(textureDimensions(view_transmission_texture));
    let uv=in.position.xy/size;
    let n=normalize(in.world_normal);
    let across=vec2<f32>(dot(n,view.world_from_view[0].xyz),-dot(n,view.world_from_view[1].xyz));
    let angle=abs(dot(n,normalize(view.world_position-in.world_position.xyz)));
    // Suppress grazing silhouettes and break up the interior so the volume
    // never reads as a continuous glass cone. Animated mesh normals carry
    // simulation-time turbulence into this mask without a separate clock.
    let ripple=0.5+0.5*sin(dot(n,vec3<f32>(7.3,11.7,5.1)));
    let breakup=mix(0.18,1.0,smoothstep(0.15,0.85,ripple));
    let fade=pow(in.color.a,1.6)*pow(smoothstep(0.05,0.85,angle),2.0)*breakup;
    let offset=across*fade*10.0/size;
    let warped=textureSampleLevel(view_transmission_texture,view_transmission_sampler,clamp(uv+offset,vec2<f32>(0.001),vec2<f32>(0.999)),0.0);
    return vec4<f32>(warped.rgb,fade*0.12);
}
