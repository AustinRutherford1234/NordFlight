//! Static fitted cockpit fittings. Shared atlas and rigid batches avoid adding
//! live instrument pages or per-frame work for decorative detail.
use super::{block,rod};
use bevy::{asset::RenderAssetUsages,prelude::*,render::render_resource::{Extent3d,TextureDimension,TextureFormat}};
fn finish(m:&mut Assets<StandardMaterial>,color:Color,metallic:f32)->Handle<StandardMaterial>{
 m.add(StandardMaterial{base_color:color,metallic,perceptual_roughness:0.78,..default()})
}
fn frame(c:&mut Commands,m:&mut Assets<Mesh>,root:Entity,mat:Handle<StandardMaterial>,at:Vec3,size:Vec2,q:Quat){
 for (p,s) in [(Vec3::new(-size.x*0.5,0.,0.),Vec3::new(0.015,size.y+0.015,0.018)),(Vec3::new(size.x*0.5,0.,0.),Vec3::new(0.015,size.y+0.015,0.018)),(Vec3::new(0.,-size.y*0.5,0.),Vec3::new(size.x,0.015,0.018)),(Vec3::new(0.,size.y*0.5,0.),Vec3::new(size.x,0.015,0.018))]{block(c,m,root,mat.clone(),at+q*p,s,q);}
}
pub(crate) fn front_panel(c:&mut Commands,m:&mut Assets<Mesh>,mats:&mut Assets<StandardMaterial>,root:Entity){
 let paint=finish(mats,Color::srgb(0.075,0.085,0.09),0.30);
 let rubber=finish(mats,Color::srgb(0.025,0.029,0.032),0.05);
 let edge=finish(mats,Color::srgb(0.24,0.26,0.26),0.58);
 // Wide backing supports all three screens and the existing switch columns.
 block(c,m,root,paint.clone(),Vec3::new(0.,0.42,-3.50),Vec3::new(0.98,0.43,0.08),Quat::IDENTITY);
 for x in [-0.29,0.,0.29]{
  block(c,m,root,rubber.clone(),Vec3::new(x,0.49,-3.475),Vec3::new(0.282,0.25,0.05),Quat::IDENTITY);
  frame(c,m,root,paint.clone(),Vec3::new(x,0.49,-3.438),Vec2::new(0.26,0.22),Quat::IDENTITY);
  for side in [-1.,1.]{for y in [0.385,0.595]{
   rod(c,m,root,edge.clone(),Vec3::new(x+side*0.133,y,-3.44),Vec3::new(x+side*0.133,y,-3.427),0.004);
  }}
 }
 // Split glare shield leaves the existing central FCS display unobstructed.
 for side in [-1.,1.]{
  block(c,m,root,rubber.clone(),Vec3::new(side*0.333,0.653,-3.55),Vec3::new(0.334,0.075,0.27),Quat::IDENTITY);
  rod(c,m,root,paint.clone(),Vec3::new(side*0.18,0.686,-3.41),Vec3::new(side*0.49,0.686,-3.41),0.009);
  for i in 0..8 {block(c,m,root,rubber.clone(),Vec3::new(side*(0.20+i as f32*0.035),0.6915,-3.55),Vec3::new(0.012,0.003,0.08),Quat::IDENTITY);}
 }
 block(c,m,root,paint.clone(),Vec3::new(0.,0.69,-3.515),Vec3::new(0.29,0.135,0.05),Quat::IDENTITY);
 frame(c,m,root,rubber,Vec3::new(0.,0.69,-3.477),Vec2::new(0.257,0.115),Quat::IDENTITY);
 block(c,m,root,paint.clone(),Vec3::new(0.,0.772,-3.55),Vec3::new(0.32,0.025,0.20),Quat::IDENTITY);
 // Lower knee panel with mounted connector housings and vent slots.
 block(c,m,root,paint,Vec3::new(0.,0.245,-3.48),Vec3::new(0.64,0.055,0.045),Quat::IDENTITY);
 for side in [-1.,1.]{for j in 0..6 {rod(c,m,root,edge.clone(),Vec3::new(side*(0.21+j as f32*0.016),0.244,-3.453),Vec3::new(side*(0.21+j as f32*0.016),0.254,-3.453),0.0025);}}
}
pub(crate) fn rear_display(x:f32,z:f32)->Transform{
 let q=Quat::from_rotation_x(-0.12);
 Transform::from_translation(Vec3::new(0.,0.60,-3.3+z)+q*Vec3::new(x,0.02,0.056)).with_rotation(q)
}
pub(crate) fn rear_bezel(c:&mut Commands,m:&mut Assets<Mesh>,root:Entity,mat:Handle<StandardMaterial>,x:f32,z:f32){
 let t=rear_display(x,z);frame(c,m,root,mat,t.translation+t.rotation*Vec3::Z*0.002,Vec2::splat(0.202),t.rotation);
}
pub(crate) fn seat_back(c:&mut Commands,m:&mut Assets<Mesh>,root:Entity,metal:Handle<StandardMaterial>,chrome:Handle<StandardMaterial>,belt:Handle<StandardMaterial>){
 // Back-face structure remains attached to the same moving/ejecting seat root.
 for side in [-1.,1.]{
  block(c,m,root,metal.clone(),Vec3::new(side*0.19,0.37,0.345),Vec3::new(0.025,0.68,0.03),Quat::IDENTITY);
  for y in [0.10,0.35,0.62]{rod(c,m,root,chrome.clone(),Vec3::new(side*0.19,y,0.352),Vec3::new(side*0.19,y,0.370),0.006);}
  // Cushion stitching and seams stop outside the existing harness webbing.
  for y in [0.17,0.29,0.41,0.53,0.65]{rod(c,m,root,belt.clone(),Vec3::new(side*0.17,y,0.146),Vec3::new(side*0.20,y,0.146),0.0018);}
 }
 for y in [0.19,0.50,0.69]{block(c,m,root,metal.clone(),Vec3::new(0.,y,0.34),Vec3::new(0.36,0.018,0.025),Quat::IDENTITY);}
 // Three connected ribs give the head-box rear a defined, supported surface.
 for x in [-0.09,0.,0.09]{block(c,m,root,chrome.clone(),Vec3::new(x,0.85,0.435),Vec3::new(0.008,0.15,0.012),Quat::IDENTITY);}
}
const LABELS:[&str;12]=["ELECTRICAL","LIGHTING","FLIGHT CTRL","FUEL","RADAR","COMM","AUXILIARY","OXYGEN","CAUTION","CREW SYSTEMS","SERVICE","VENTILATION"];
fn atlas(images:&mut Assets<Image>,mats:&mut Assets<StandardMaterial>)->Handle<StandardMaterial>{
 // 512 x 128 RGBA = 256 KiB, generated once. No new live display rasterization.
 let mut pixels=vec![0u8;512*128*4];
 for px in pixels.chunks_exact_mut(4){px.copy_from_slice(&[24,28,30,255]);}
 for (i,label) in LABELS.iter().enumerate(){
  let ox=(i%4)*128+6;let oy=(i/4)*32+8;
  for (j,ch) in label.chars().enumerate(){for (y,row) in super::super::glyph(ch).iter().enumerate(){for x in 0..5 {if row&(1<<(4-x))!=0 {
   for dy in 0..2 {let xx=ox+j*7+x;let yy=oy+y*2+dy;let k=(yy*512+xx)*4;pixels[k..k+4].copy_from_slice(&[163,172,166,255]);}
  }}}}
 }
 let mut image=Image::new_fill(Extent3d{width:512,height:128,depth_or_array_layers:1},TextureDimension::D2,&[0,0,0,255],TextureFormat::Rgba8UnormSrgb,RenderAssetUsages::default());image.data=Some(pixels);
 let texture=images.add(image);
 mats.add(StandardMaterial{base_color_texture:Some(texture),perceptual_roughness:0.95,..default()})
}
fn placard(c:&mut Commands,m:&mut Assets<Mesh>,root:Entity,mat:Handle<StandardMaterial>,index:usize,p:Vec3,size:Vec2,q:Quat){
 let mut mesh=Mesh::from(Rectangle::from_size(size));
 if let Some(bevy::mesh::VertexAttributeValues::Float32x2(uvs))=mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0){for uv in uvs {uv[0]=(index%4) as f32*0.25+uv[0]*0.25;uv[1]=(index/4) as f32*0.25+uv[1]*0.25;}}
 let e=c.spawn((crate::render_batch::RigidGeometry,Mesh3d(m.add(mesh)),MeshMaterial3d(mat),Transform::from_translation(p).with_rotation(q))).id();c.entity(root).add_child(e);
}
pub(crate) fn tub(c:&mut Commands,m:&mut Assets<Mesh>,mats:&mut Assets<StandardMaterial>,images:&mut Assets<Image>,root:Entity){
 let paint=finish(mats,Color::srgb(0.105,0.12,0.125),0.25);
 let metal=finish(mats,Color::srgb(0.29,0.31,0.32),0.60);
 let dark=finish(mats,Color::srgb(0.025,0.03,0.032),0.05);
 let labels=atlas(images,mats);
 for seat in 0..2 {
  let z=seat as f32*super::SPACING;let floor=-0.09+seat as f32*0.12;
  // Close the footwell below the panel; the exterior shell is culled from
  // the cockpit camera, so a real forward bulkhead and cheek returns matter.
  block(c,m,root,dark.clone(),Vec3::new(0.,0.06+seat as f32*0.12,-3.505+z),Vec3::new(0.96,0.44,0.035),Quat::IDENTITY);
  for side in [-1.,1.]{block(c,m,root,dark.clone(),Vec3::new(side*0.49,0.06+seat as f32*0.12,-3.43+z),Vec3::new(0.06,0.44,0.23),Quat::IDENTITY);}
  for side in [-1.,1.]{
   // Recessed sidewall access panels, piping and captive fasteners.
   for aft in [-3.18,-2.74,-2.30]{
    block(c,m,root,paint.clone(),Vec3::new(side*0.454,0.10,aft+z),Vec3::new(0.012,0.26,0.34),Quat::IDENTITY);
    for y in [0.0,0.20]{for dz in [-0.13,0.13]{rod(c,m,root,metal.clone(),Vec3::new(side*0.442,y,aft+dz+z),Vec3::new(side*0.451,y,aft+dz+z),0.0035);}}
   }
   for y in [0.27,0.30]{rod(c,m,root,dark.clone(),Vec3::new(side*0.448,y,-3.24+z),Vec3::new(side*0.448,y,-2.18+z),0.007);}
   // Lower sill seal and latch bases join the sidewall instead of floating.
   rod(c,m,root,dark.clone(),Vec3::new(side*0.476,0.685,-3.32+z),Vec3::new(side*0.476,0.685,-2.02+z),0.012);
   for aft in [-3.19,-2.64,-2.12]{block(c,m,root,metal.clone(),Vec3::new(side*0.477,0.65,aft+z),Vec3::new(0.034,0.045,0.065),Quat::IDENTITY);}
   // Pedal support brackets meet the recessed floor.
   block(c,m,root,paint.clone(),Vec3::new(side*0.17,floor+0.035,-3.30+z),Vec3::new(0.15,0.07,0.23),Quat::IDENTITY);
   if seat==0 {
    let rows=if side<0.{[-3.26,-3.12,-2.50,-2.30]}else{[-3.14,-2.94,-2.74,-2.54]};
    for aft in rows {
     block(c,m,root,paint.clone(),Vec3::new(side*0.37,0.434,aft),Vec3::new(0.213,0.004,0.123),Quat::IDENTITY);
     for dx in [-0.094,0.094]{for dz in [-0.049,0.049]{rod(c,m,root,metal.clone(),Vec3::new(side*0.37+dx,0.436,aft+dz),Vec3::new(side*0.37+dx,0.440,aft+dz),0.0027);}}
    }
   }
  }
  block(c,m,root,paint.clone(),Vec3::new(0.,floor+0.006,-3.0+z),Vec3::new(0.43,0.012,0.35),Quat::IDENTITY);
  for x in [-0.19,0.19]{for aft in [-3.14,-2.86]{rod(c,m,root,metal.clone(),Vec3::new(x,floor+0.012,aft+z),Vec3::new(x,floor+0.017,aft+z),0.004);}}
 }
 for (index,side,aft) in [(0,-1.,-3.36),(1,-1.,-3.02),(2,-1.,-2.65),(3,-1.,-2.19),(4,1.,-3.34),(5,1.,-3.05),(6,1.,-2.84),(7,1.,-2.22)] {
  placard(c,m,root,labels.clone(),index,Vec3::new(side*0.37,0.433,aft),Vec2::new(0.19,0.035),Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2));
 }
 placard(c,m,root,labels.clone(),8,Vec3::new(0.,0.28,-2.126),Vec2::new(0.24,0.045),Quat::from_rotation_y(std::f32::consts::PI));
 placard(c,m,root,labels,9,Vec3::new(0.,0.32,-0.676),Vec2::new(0.28,0.045),Quat::from_rotation_y(std::f32::consts::PI));
}

#[cfg(test)]
mod tests {
 use super::*;
 #[test]
 fn forward_footwells_have_opaque_coverage_below_both_panels(){
  let mut app=App::new();app.init_resource::<Assets<Mesh>>().init_resource::<Assets<StandardMaterial>>().init_resource::<Assets<Image>>().add_systems(Startup,|mut c:Commands,mut m:ResMut<Assets<Mesh>>,mut mats:ResMut<Assets<StandardMaterial>>,mut images:ResMut<Assets<Image>>|{
   let root=c.spawn(Transform::default()).id();tub(&mut c,&mut m,&mut mats,&mut images,root);
  });app.update();
  let w=app.world_mut();let parts:Vec<_>=w.query::<(&Mesh3d,&Transform)>().iter(w).map(|(m,t)|(m.0.clone(),*t)).collect();let meshes=w.resource::<Assets<Mesh>>();
  for seat in 0..2 {for x in [-0.35,0.,0.35] {for y in [-0.06,0.05,0.20] {
   let p=Vec3::new(x,y+seat as f32*0.12,-3.40+seat as f32*super::super::SPACING);let mut covered=false;
   for (h,t) in &parts {
    let Some(bevy::mesh::VertexAttributeValues::Float32x3(v))=meshes.get(h).unwrap().attribute(Mesh::ATTRIBUTE_POSITION)else{panic!("positions")};
    let mut lo=Vec3::splat(f32::INFINITY);let mut hi=Vec3::splat(f32::NEG_INFINITY);for v in v {let a=t.transform_point(Vec3::from_array(*v));lo=lo.min(a);hi=hi.max(a);}
    if (lo.x..=hi.x).contains(&p.x)&&(lo.y..=hi.y).contains(&p.y)&&hi.z<p.z&&p.z-hi.z<0.15 {covered=true;}
   }
   assert!(covered,"open footwell ahead of {p:?}");
  }}}
 }
 #[test]
 fn live_front_display_faces_have_close_backing_and_no_covering_casing(){
  let mut app=App::new();app.init_resource::<Assets<Mesh>>().init_resource::<Assets<StandardMaterial>>().add_systems(Startup,|mut c:Commands,mut m:ResMut<Assets<Mesh>>,mut mats:ResMut<Assets<StandardMaterial>>|{
   let root=c.spawn(Transform::default()).id();front_panel(&mut c,&mut m,&mut mats,root);
  });app.update();
  let w=app.world_mut();
  let parts:Vec<_>=w.query::<(&Mesh3d,&Transform)>().iter(w).map(|(m,t)|(m.0.clone(),*t)).collect();
  let meshes=w.resource::<Assets<Mesh>>();
  for (x,y,z) in [(-0.29,0.49,-3.44),(0.,0.49,-3.44),(0.29,0.49,-3.44),(0.,0.69,-3.48)] {
   for dx in [-0.09,0.,0.09] {for dy in [-0.03,0.,0.03] {
    let mut backing=f32::NEG_INFINITY;
    for (h,t) in &parts {
     let Some(bevy::mesh::VertexAttributeValues::Float32x3(p))=meshes.get(h).unwrap().attribute(Mesh::ATTRIBUTE_POSITION) else {panic!("positions")};
     let mut lo=Vec3::splat(f32::INFINITY);let mut hi=Vec3::splat(f32::NEG_INFINITY);
     for p in p {let v=t.transform_point(Vec3::from_array(*p));lo=lo.min(v);hi=hi.max(v);}
     if (lo.x..=hi.x).contains(&(x+dx))&&(lo.y..=hi.y).contains(&(y+dy)){
      assert!(hi.z<z-0.001,"opaque casing covers a live display: {x} {y} {z} bounds={lo:?}..{hi:?}");backing=backing.max(hi.z);
     }
    }
    assert!(z-backing<0.016,"floating display: gap={} at ({x},{y},{z})",z-backing);
   }}
  }
 }
}
