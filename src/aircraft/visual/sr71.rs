//! Imported SR-71 mechanisms, driven by the shared simulation clock and actuator state.
use super::*;
use serde_json::Value;
use std::sync::LazyLock;
#[derive(Component)]pub struct ImportedSr71;
#[derive(Component)]pub struct Part{index:usize,rest:Transform}
#[derive(Component)]pub struct Ram{index:usize,rod:bool}
struct Def{name:String,kind:String,axis:Vec3,angle:f32,index:usize,rest:Vec3}
struct Link{barrel:String,rod:String,base:Vec3,parent:usize,local:Vec3,length:f32,radius:f32}
struct Rig{parts:Vec<Def>,links:Vec<Link>}
fn vec(v:&Value)->Vec3{Vec3::new(v[0].as_f64().unwrap()as f32,v[1].as_f64().unwrap()as f32,v[2].as_f64().unwrap()as f32)}
static RIG:LazyLock<Rig>=LazyLock::new(||{
 let j:Value=serde_json::from_str(include_str!("../../../assets/aircraft/sr71/rig.json")).expect("SR71 rig metadata");
 let parts:Vec<Def>=j["pivots"].as_array().unwrap().iter().map(|p|Def{name:p["name"].as_str().unwrap().into(),kind:p["kind"].as_str().unwrap().into(),axis:vec(&p["axis"]).normalize(),angle:p["angle"].as_f64().unwrap()as f32,index:p["index"].as_u64().unwrap()as usize,rest:vec(&p["rest_translation"])}).collect();
 let links=j["links"].as_array().unwrap().iter().map(|l|Link{barrel:l["barrel"].as_str().unwrap().into(),rod:l["rod"].as_str().unwrap().into(),base:vec(&l["base"]),parent:parts.iter().position(|p|p.name==l["parent"].as_str().unwrap()).unwrap(),local:vec(&l["local"]),length:l["length"].as_f64().unwrap()as f32,radius:l["radius"].as_f64().unwrap()as f32}).collect();Rig{parts,links}
});
pub fn setup(c:&mut Commands,server:&AssetServer,root:Entity){
 let model=c.spawn((ImportedSr71,Name::new("SR-71 imported articulated airframe"),
 SceneRoot(crate::model_assets::load(server,GltfAssetLabel::Scene(0).from_asset("aircraft/sr71/rigged.glb"))),Transform::default(),Visibility::Inherited)).id();c.entity(root).add_child(model);
}
fn belongs(mut e:Entity,parents:&Query<&ChildOf>,roots:&Query<(),With<ImportedSr71>>)->bool{
 loop{if roots.contains(e){return true;}let Ok(p)=parents.get(e)else{return false;};e=p.parent();}
}
pub fn bind(mut c:Commands,nodes:Query<(Entity,&Name,&Transform),Added<Name>>,parents:Query<&ChildOf>,roots:Query<(),With<ImportedSr71>>){
 for(e,name,rest)in &nodes{
  if !name.as_str().starts_with("Sr71")||!belongs(e,&parents,&roots){continue;}
  if let Some(index)=RIG.parts.iter().position(|d|d.name==name.as_str()){c.entity(e).insert(Part{index,rest:*rest});}
  else if let Some((index,rod))=RIG.links.iter().enumerate().find_map(|(i,d)|if d.barrel==name.as_str(){Some((i,false))}else if d.rod==name.as_str(){Some((i,true))}else{None}){c.entity(e).insert(Ram{index,rod});}
 }
}
fn pose(def:&Def,rest:Transform,f:&Flight,spin:[f32;3])->Transform{
 let mut t=rest;let s=&f.systems.sr71;
 let angle=match def.kind.as_str(){
  "gear"=>def.angle*super::super::sr71::smooth(1.-f.gear_extension)as f32,
  "door"=>def.angle*f.gear_door.max(f.gear_extension).clamp(0.,1.)as f32,
  "wheel"=>spin[def.index],"steering"=>crate::gear::nose_steering(f),
  "elevon"=>s.elevons[match def.name.as_str(){"Sr71ElevonInnerL"=>0,"Sr71ElevonInnerR"=>1,"Sr71ElevonOuterL"=>2,_=>3}]as f32,
  "fin"=>s.fins[def.index]as f32,"canopy"=>s.canopy as f32*def.angle,_=>0.,
 };
 t.rotation=rest.rotation*Quat::from_axis_angle(def.axis,angle);
 if def.kind=="spike"{let spike=if s.propulsion.initialized{s.propulsion.sides[usize::from(def.name.ends_with('R'))].inlet.spike_m}else{s.spike};t.translation+=Vec3::Z*spike as f32;}
 if def.kind=="nozzle"{let n=s.nozzles[def.index];t.scale=Vec3::new(1.+n as f32*0.05,1.+n as f32*0.05,1.);}
 if def.kind=="oleo"||def.kind=="steering"{t.translation+=Vec3::Y*f.gear_compression[def.index]as f32;}
 t
}
fn ram_pose(link:&Link,rod:bool,f:&Flight)->Transform{
 let p=&RIG.parts[link.parent];let parent=pose(p,Transform::from_translation(p.rest),f,[0.;3]);
 let b=parent.transform_point(link.local);let delta=b-link.base;let length=delta.length().max(1e-4);let direction=delta/length;
 let barrel=link.length.min(length*0.85);let overlap=0.045_f32.min(barrel*0.5);
 let (a,len,r)=if rod{(link.base+direction*(barrel-overlap),(length-barrel+overlap).max(0.001),link.radius*0.52)}else{(link.base,barrel,link.radius)};
 Transform{translation:a,rotation:Quat::from_rotation_arc(Vec3::Y,direction),scale:Vec3::new(r,len,r)}
}
#[derive(Default)]pub struct Clock{generation:Option<u64>,elapsed:f64,spin:[f32;3]}
pub fn update(f:Res<Flight>,view:Res<crate::ViewState>,mut clock:Local<Clock>,mut parts:Query<(&Part,&mut Transform),Without<Ram>>,mut rams:Query<(&Ram,&mut Transform),Without<Part>>){
 if f.aircraft!=Aircraft::Sr71||f.vehicle!=Vehicle::Gripen{return;}
 let reset=clock.generation!=Some(view.generation)||f.elapsed<clock.elapsed;
 let dt=if reset||f.paused{0.}else{(f.elapsed-clock.elapsed).max(0.)};
 if reset{clock.spin=[0.;3];}clock.generation=Some(view.generation);clock.elapsed=f.elapsed;
 if f.landed&&f.gear_extension>0.98 {
  let speed=(f.velocity-crate::carrier::ground_velocity(&f,f.position)).dot(f.forward());
  for i in 0..3{clock.spin[i]=(clock.spin[i]-(speed*dt/if i==0{0.285}else{0.36})as f32).rem_euclid(std::f32::consts::TAU);}
 }
 for(part,mut t)in &mut parts{t.set_if_neq(pose(&RIG.parts[part.index],part.rest,&f,clock.spin));}
 for(ram,mut t)in &mut rams{t.set_if_neq(ram_pose(&RIG.links[ram.index],ram.rod,&f));}
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn imported_rig_is_static_mesh_with_real_named_pivots_and_embedded_images(){
  let b=include_bytes!("../../../assets/aircraft/sr71/rigged.glb");let n=u32::from_le_bytes(b[12..16].try_into().unwrap())as usize;let j:Value=serde_json::from_slice(&b[20..20+n]).unwrap();
  assert!(j["images"].as_array().unwrap().iter().all(|i|i.get("bufferView").is_some()));
  for part in &RIG.parts{assert!(j["nodes"].as_array().unwrap().iter().any(|n|n["name"]==part.name),"missing {}",part.name);}
  assert!(RIG.parts.len()>=29);assert!(RIG.links.len()>=10);
 }
 #[test]fn imported_nodes_bind_once_animate_and_reset_without_touching_unrelated_names(){
  let mut app=App::new();app.add_plugins(MinimalPlugins);let mut f=Flight::runway();f.aircraft=Aircraft::Sr71;
  app.insert_resource(f).insert_resource(crate::ViewState::default()).add_systems(Update,(bind,update).chain());
  let root=app.world_mut().spawn((ImportedSr71,Transform::default())).id();
  let i=RIG.parts.iter().position(|p|p.name=="Sr71GearNose").unwrap();
  let gear=app.world_mut().spawn((Name::new("Sr71GearNose"),Transform::from_translation(RIG.parts[i].rest),ChildOf(root))).id();
  let unrelated=app.world_mut().spawn((Name::new("Sr71GearNose"),Transform::default())).id();
  app.update();assert!(app.world().get::<Part>(gear).is_some());assert!(app.world().get::<Part>(unrelated).is_none());
  {let mut f=app.world_mut().resource_mut::<Flight>();f.gear_extension=0.;f.gear_door=0.;}app.update();
  assert!((app.world().get::<Transform>(gear).unwrap().rotation*Vec3::Y).distance(Vec3::Z)<1e-5);
  app.world_mut().resource_mut::<crate::ViewState>().generation+=1;
  app.world_mut().resource_mut::<Flight>().gear_extension=1.;app.update();
  assert!(app.world().get::<Transform>(gear).unwrap().rotation.angle_between(Quat::IDENTITY)<1e-5);
 }
 #[test]fn doors_wait_for_gear_and_rams_remain_finite_through_entire_cycle(){
  let mut f=Flight::runway();f.aircraft=Aircraft::Sr71;
  for step in 0..=200 {
   f.gear_extension=step as f64/200.;f.gear_door=if step==0{0.}else{1.};
   for l in &RIG.links{let a=ram_pose(l,false,&f);let b=ram_pose(l,true,&f);assert!(a.translation.is_finite()&&b.rotation.is_finite());assert!(a.scale.min_element()>0.&&b.scale.min_element()>0.);
    let target=pose(&RIG.parts[l.parent],Transform::from_translation(RIG.parts[l.parent].rest),&f,[0.;3]).transform_point(l.local);
    assert!(b.transform_point(Vec3::Y).distance(target)<1e-4);
   }
   for p in RIG.parts.iter().filter(|p|p.kind=="door"){
    let t=pose(p,Transform::default(),&f,[0.;3]);if step>0{assert!(t.rotation.angle_between(Quat::from_axis_angle(p.axis,p.angle))<1e-3);}
   }
  }
 }
}
