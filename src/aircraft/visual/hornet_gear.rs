//! Visual kinematics in the imported asset's units (ten units per metre).
use bevy::prelude::*;
fn p(x:f32,y:f32,z:f32)->Vec3 {Vec3::new(x,z,-y)}
fn main_axle(side:f32,r:f32)->Vec3 {
 let tuck=(r/0.45).clamp(0.,1.);
 let lift=((r-0.45)/0.55).clamp(0.,1.);
 p(6.1,side*(16.03-9.53*tuck),-20.3+4.3*tuck+12.0*lift)
}
fn nose_axle(side:f32,r:f32)->Vec3 {
 p(61.7+12.95*r+side*2.85*(r/0.45).clamp(0.,1.),side*2.12*(1.-((r-0.4)/0.45).clamp(0.,1.)),-20.17+17.37*r)
}
pub fn pose(name:&str, extension:f32, door:f32)->Option<Transform> {
 // Bevy gives primitive children names such as `...Mesh.airframe`.
 // Animate the authored pivot only, never its mesh or primitive children.
 if name.contains("Mesh") || name.contains('.') || name.contains(' ') {return None;}
 let e=extension.clamp(0.,1.); let r=1.-e;
 let side=if name.ends_with('L'){-1.}else{1.};
 if name.starts_with("HornetMainLegDoor") || name.starts_with("HornetNoseLegDoor") {
  return Some(Transform::from_rotation(Quat::from_rotation_x((if name.starts_with("HornetMain") {-side}else{side})*e.max(door.clamp(0.,1.))*1.5)));
 }
 if name.starts_with("HornetMainDoor") || name.starts_with("HornetNoseDoor") {
  return Some(Transform::from_rotation(Quat::from_rotation_x((if name.starts_with("HornetMainDoor") {-side}else{side})*e.max(door.clamp(0.,1.))*1.65)));
 }
 if matches!(name,"HornetGearNose"|"HornetGearMainL"|"HornetGearMainR") {return Some(Transform::IDENTITY);}
 if name.starts_with("HornetWheel") {
  let nose=name.contains("Nose");
  let root=if nose {p(63.,0.,-6.)} else {p(7.,side*8.,-6.)};
  return Some(Transform::from_translation((if nose {nose_axle(side,r)}else{main_axle(side,r)})-root).with_rotation(if nose {Quat::from_rotation_x(side*((r-0.45)/0.55).clamp(0.,1.)*std::f32::consts::FRAC_PI_2)}else{Quat::from_rotation_x(side*((r-0.45)/0.55).clamp(0.,1.)*std::f32::consts::FRAC_PI_2)}));
 }
 if name.starts_with("HornetStrut") {
  let nose=name.contains("Nose");
  let mount=if nose {p(63.,0.,-3.5)}else{p(7.,side*8.,-5.5)};
  let knee=if nose {p(63.,0.,-13.).lerp(p(68.,0.,-3.5),r)}else{p(8.,side*12.,-13.).lerp(p(11.,side*7.,-3.),r)};
  let axle=if nose {p(61.7,0.,-20.17).lerp(p(74.65,0.,-2.8),r)}else{main_axle(side,r)};
  let scissor=knee.lerp(axle,0.45)+Vec3::new(1.2,0.,0.);
  let (a,b)=if name.contains("Fork"){(axle,nose_axle(if name.contains("ForkL"){-1.}else{1.},r))}else if name.contains("Upper"){(mount,knee)}else if name.contains("Brace"){(mount+Vec3::new(5.,0.,if nose{0.9}else{0.}),knee)}else if name.contains("ScissorA"){(knee,scissor)}else if name.contains("ScissorB"){(scissor,axle)}else if name.contains("Hose"){(mount+Vec3::Z*0.8,axle+Vec3::Z*0.8)}else{(knee,axle)};
  let delta=b-a;
  return Some(Transform::from_translation((a+b)*0.5).with_rotation(Quat::from_rotation_arc(Vec3::Y,delta.normalize()).normalize()).with_scale(Vec3::new(1.,delta.length(),1.)));
 }
 None
}

#[cfg(test)]
mod tests {
 use super::*;
 #[test]
 fn strut_mesh_long_axis_follows_the_link_not_its_radius() {
  for leg in ["Nose","MainL","MainR"] {
   for segment in ["Upper","Lower","Brace","ScissorA","ScissorB","Hose"] {
    for i in 0..=100 {
     let t=pose(&format!("HornetStrut{segment}{leg}"),i as f32/100.,1.).unwrap();
     // Authored cylinders extend from exported local -Y/2 to +Y/2. Cross
     // sections must stay circular while only their axial length changes.
     assert_eq!(t.scale.x,1.);assert_eq!(t.scale.z,1.);
     let a=t.transform_point(Vec3::NEG_Y*0.5);
     let b=t.transform_point(Vec3::Y*0.5);
     assert!((a.distance(b)-t.scale.y).abs()<1e-4,"{leg} {segment} {i}: {:?} distance={} scale={}",t.rotation,a.distance(b),t.scale.y);
     assert!(a.is_finite()&&b.is_finite());
    }
   }
  }
 }
 #[test]
 fn deployed_upper_nose_link_joins_its_actual_mount_and_knee() {
  let t=pose("HornetStrutUpperNose",1.,0.).unwrap();
  assert!(t.transform_point(Vec3::NEG_Y*0.5).distance(p(63.,0.,-3.5))<1e-4);
  assert!(t.transform_point(Vec3::Y*0.5).distance(p(63.,0.,-13.))<1e-4);
 }
 #[test]
 fn stowed_nose_tire_envelope_clears_the_well_roof() {
  for side in [-1.,1.] {
   let n=if side<0.{"HornetWheelNoseL"}else{"HornetWheelNoseR"};
   let t=pose(n,0.,0.).unwrap();
   let axle=p(63.,0.,-6.)+t.translation;
   // Stowed tire rotates flat; its half-width is about .6 units.
   assert!(axle.y+1.0< -1.68);
   assert!(axle.y-1.0> -3.86);
  }
 }
 #[test]
 fn bay_covers_remain_open_with_deployed_gear() {
  for n in ["HornetMainDoorForeL","HornetMainDoorForeR","HornetNoseDoorL","HornetNoseDoorR"] {
   assert_eq!(pose(n,1.,0.).unwrap().rotation,pose(n,0.,1.).unwrap().rotation);
   assert_eq!(pose(n,0.,0.).unwrap().rotation,Quat::IDENTITY);
  }
 }
 #[test]
 fn flattened_nose_wheels_clear_the_narrow_bay_during_stowage(){
  for step in 0..=1000{let r=step as f32/1000.;let t=pose("HornetWheelNoseR",1.-r,1.).unwrap();let center=p(63.,0.,-6.)+t.translation;let angle=((r-0.45)/0.55).clamp(0.,1.)*std::f32::consts::FRAC_PI_2;let vertical=2.75*angle.cos()+1.0*angle.sin();let lateral=2.75*angle.sin()+1.0*angle.cos();if center.y+vertical> -4.4{assert!((-center.z).abs()+lateral<3.6);assert!(center.y+vertical< -1.68);}}
 }
 #[test]
 fn primitive_children_keep_their_authored_offsets() {
  for n in ["HornetWheelNoseLMesh", "HornetWheelMainRMesh.airframe", "HornetStrutUpperNose.airframe"] {assert!(pose(n,0.,0.).is_none());}
 }
 #[test]
 fn doors_open_outward_and_clear_before_the_wheels_move() {
  for side in [-1.,1.] {
   let suffix=if side<0. {"L"}else{"R"};
   for prefix in ["HornetMainLegDoor","HornetNoseLegDoor"] {
    let t=pose(&format!("{prefix}{suffix}"),0.,1.).unwrap();
    let offset=p(0.,if prefix.contains("Main") {side*3.}else{-side*3.6},0.);
    assert!((t.rotation*offset).y<0.,"{prefix}{suffix} must open below the belly");
    assert_eq!(t.rotation,pose(&format!("{prefix}{suffix}"),1.,1.).unwrap().rotation);
   }
   let main=pose(&format!("HornetMainDoorFore{suffix}"),0.5,1.).unwrap();
   // A fully open cover stays inboard of the wheel's narrowest tuck path.
   for u in 0..=100 {let pt=p(0.,side*2.,-8.5)+main.rotation*p(0.,side*9.*u as f32/100.,0.);assert!((-pt.z).abs()<2.8);}
  }
 }
 #[test]
 fn main_tires_clear_bay_walls_when_crossing_the_belly() {
  for step in 0..=1000 {
   let r=step as f32/1000.; let axle=main_axle(1.,r);
   let angle=((r-0.45)/0.55).clamp(0.,1.)*std::f32::consts::FRAC_PI_2;
   let vertical=4.1*angle.cos()+1.5*angle.sin();
   let lateral=4.1*angle.sin()+1.5*angle.cos();
   if axle.y+vertical > -8.5 { let span=-axle.z; assert!(span-lateral>2. && span+lateral<11.,"r={r}"); }
  }
 }
 #[test]
 fn complete_stroke_is_finite_and_stowed_wheels_fit_above_belly() {
  for n in ["HornetWheelNoseL","HornetWheelNoseR","HornetWheelMainL","HornetWheelMainR"] {
   let nose=n.contains("Nose"); let side=if n.ends_with('L'){-1.}else{1.};
   let root=if nose{p(63.,0.,-6.)}else{p(7.,side*8.,-6.)};
   let stowed=pose(n,0.,0.).unwrap();
   assert!((root+stowed.translation).y > -5.);
   let mut last=pose(n,0.,1.).unwrap().translation;
   for i in 1..=100 {let t=pose(n,i as f32/100.,1.).unwrap();assert!(t.translation.is_finite());assert!(last.distance(t.translation)<0.3);last=t.translation;}
  }
  for n in ["HornetStrutUpperNose","HornetStrutLowerNose","HornetStrutUpperMainL","HornetStrutLowerMainL","HornetStrutUpperMainR","HornetStrutLowerMainR"] {
   for i in 0..=100 {let t=pose(n,i as f32/100.,1.).unwrap();assert!(t.rotation.is_finite());assert!(t.scale.y>0.);}
  }
 }
}
