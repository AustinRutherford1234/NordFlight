#import bevy_pbr::{forward_io::VertexOutput, mesh_view_bindings::view}
struct Parameters {local_from_world:mat4x4<f32>,flow:vec4<f32>,shape:vec4<f32>}
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> p:Parameters;
// Retained box back-faces bound a single integration per pixel. Coordinates
// are nozzle-local, so the origin cannot drift when the aircraft moves/rotates.
@fragment
fn fragment(in:VertexOutput)->@location(0) vec4<f32> {
 let camera=(p.local_from_world*vec4<f32>(view.world_position,1.)).xyz;
 let surface=(p.local_from_world*in.world_position).xyz;
 let ray=normalize(surface-camera);
 let inv=1./(ray+vec3<f32>(0.000001));
 let lo=(vec3<f32>(-1.,-1.,0.)-camera)*inv;
 let hi=(vec3<f32>(1.,1.,1.)-camera)*inv;
 let near=min(lo,hi);let far=max(lo,hi);
 let enter=max(0.,max(near.x,max(near.y,near.z)));
 let leave=min(far.x,min(far.y,far.z));
 if leave<=enter {discard;}
 let distance=length(camera*vec3<f32>(p.shape.x*1.65,p.shape.x*1.65,p.shape.y));
 let path_m=(leave-enter)*length(ray*vec3<f32>(p.shape.x*1.65,p.shape.x*1.65,p.shape.y));
 let quality_steps=u32(clamp(ceil(path_m/(p.shape.z*0.20)),12.,48.));
 let steps=select(quality_steps,min(quality_steps,16u),distance>100.);
 let step=(leave-enter)/f32(steps);
 var energy=vec3<f32>(0.);var optical=0.;
 let time=p.flow.x;let wet=p.flow.y;let seed=p.flow.w;
 for(var i=0u;i<steps;i+=1u){
  let q=camera+ray*(enter+(f32(i)+0.5)*step);
  let z=q.z*p.shape.y;
  let u=q.z;
  // Downstream motion grows into the shear layer; the mouth stays centered.
  let a=z*3.6-time*15.+seed;
  let b=z*7.3+time*19.-seed;
  let turbulence=sin(a)*sin(b);
  let bend=vec2<f32>(sin(z*1.5-time*9.+seed),sin(z*2.1-time*11.-seed))*u*u*0.12;
  let r=length(q.xy*1.65-bend);
  let phase=(z/p.shape.z+0.035*z*z/p.shape.y)*6.2831853
    +sin(z*0.8-time+seed)*u*0.65;
  let cell=0.5+0.5*cos(phase);
  let width=(0.78+0.12*sin(phase))*(1.-u*0.58);
  let edge=1.-smoothstep(width*0.20,width,r+0.045*turbulence*u);
  let tail=pow(max(1.-u,0.),1.5)*smoothstep(0.,0.12,z);
  let core=exp(-r*r/(0.035+0.025*u))*(0.45+0.55*pow(cell,6.));
  let shock=pow(cell,4.)*exp(-r*r/(0.018+0.06*(1.-cell)))*exp(-u*1.1);
  let envelope=edge*(0.50+0.30*turbulence)*tail;
  let violet=vec3<f32>(0.22,0.12,0.48);
  let flame=mix(violet,vec3<f32>(1.4,0.18,0.045),pow(u,0.65)*0.65);
  let hot=vec3<f32>(1.35,1.15,0.90)*core*tail;
  let diamond=vec3<f32>(4.00,2.50,1.20)*shock*tail;
  let density=(envelope*0.42+core*0.7+shock*0.5)*wet;
  let ds=step*p.shape.x*1.65;
  energy+=(flame*envelope+hot+diamond)*ds*wet*2.0;
  optical+=density*ds;
 }
 let alpha=1.-exp(-optical*1.6);
 return vec4<f32>(energy*p.flow.z*2.4,alpha);
}
