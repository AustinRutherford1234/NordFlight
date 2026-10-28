//! Visual kinematics in the imported asset's units (ten units per metre).
use bevy::prelude::*;
fn p(x:f32,y:f32,z:f32)->Vec3 {Vec3::new(x,z,-y)}
fn main_axle(side:f32,r:f32)->Vec3 {
 let tuck=(r/0.45).clamp(0.,1.);
 let lift=((r-0.45)/0.55).clamp(0.,1.);
 p(6.1,side*(16.03-9.53*tuck),-20.3+4.3*tuck+12.0*lift)
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
  return Some(Transform::from_rotation(Quat::from_rotation_x((if name.starts_with("HornetMainDoor") {-side}else{side})*door.clamp(0.,1.)*1.65)));
 }
 if matches!(name,"HornetGearNose"|"HornetGearMainL"|"HornetGearMainR") {return Some(Transform::IDENTITY);}
 if name.starts_with("HornetWheel") {
  let nose=name.contains("Nose");
  let down=if nose {p(61.7,side*2.12,-20.17)} else {p(6.1,side*16.03,-20.3)};
  let up=if nose {p(75.,side*2.12,0.0)} else {p(6.,side*6.,-4.8)};
  let root=if nose {p(63.,0.,-6.)} else {p(7.,side*8.,-6.)};
  return Some(Transform::from_translation((if nose {down.lerp(up,r)}else{main_axle(side,r)})-root).with_rotation(if nose {Quat::IDENTITY}else{Quat::from_rotation_x(side*((r-0.45)/0.55).clamp(0.,1.)*std::f32::consts::FRAC_PI_2)}));
 }
 if name.starts_with("HornetStrut") {
  let nose=name.contains("Nose");
  let mount=if nose {p(63.,0.,-3.5)}else{p(7.,side*8.,-5.5)};
  let knee=if nose {p(63.,0.,-13.).lerp(p(68.,0.,-1.),r)}else{p(8.,side*12.,-13.).lerp(p(11.,side*7.,-3.),r)};
  let axle=if nose {p(61.7,0.,-20.17).lerp(p(75.,0.,0.0),r)}else{main_axle(side,r)};
  let scissor=knee.lerp(axle,0.45)+Vec3::new(1.2,0.,0.);
  let (a,b)=if name.contains("Upper"){(mount,knee)}else if name.contains("Brace"){(mount+Vec3::new(5.,0.,0.),knee)}else if name.contains("ScissorA"){(knee,scissor)}else if name.contains("ScissorB"){(scissor,axle)}else if name.contains("Hose"){(mount+Vec3::Z*0.8,axle+Vec3::Z*0.8)}else{(knee,axle)};
  let delta=b-a;
  return Some(Transform::from_translation((a+b)*0.5).with_rotation(Quat::from_rotation_arc(Vec3::Y,delta.normalize())).with_scale(Vec3::new(1.,delta.length(),1.)));
 }
 None
}

#[cfg(test)]
mod tests {
 use super::*;
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
