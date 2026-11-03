//! SR-71 actuator state and dedicated aerodynamic plant.
pub mod flight;
use crate::physics::*;
use bevy::math::DVec3;
#[derive(Clone,Default)]
pub struct State {
    pub canopy_command:bool,
    pub canopy:f64,
    pub elevons:[f64;4],
    pub fins:[f64;2],
    pub spike:f64,
    pub nozzles:[f64;2],
}
fn move_to(value:&mut f64,target:f64,rate:f64,dt:f64){*value+=(target-*value).clamp(-rate*dt,rate*dt);}
pub fn advance(f:&mut Flight,dt:f64,mach:f64){
    if dt<=0.||!dt.is_finite(){return;}
    let authority=((f.damage.hydraulics-0.12)/0.88).clamp(0.,1.);
    let commands=f.surface;let state=&mut f.systems.sr71;
    let targets=flight::targets(commands);
    for i in 0..4 {move_to(&mut state.elevons[i],targets[i],30_f64.to_radians()*authority,dt);}
    for i in 0..2 {move_to(&mut state.fins[i],commands.y.clamp(-1.,1.)*20_f64.to_radians(),33_f64.to_radians()*authority,dt);}
    move_to(&mut state.canopy,if state.canopy_command{1.}else{0.},1./3.2*authority,dt);
    // Representative spike travel; a dedicated intake/engine model is future work.
    let target=0.66*((mach-1.6)/1.6).clamp(0.,1.);
    move_to(&mut state.spike,target,0.12*authority,dt);
    for i in 0..2 {
        let target=if f.engine&&f.systems.engine_on[i] {f.spool.clamp(0.,1.)}else{0.};
        move_to(&mut state.nozzles[i],target,0.5*authority,dt);
    }
}
pub fn smooth(x:f64)->f64{let x=x.clamp(0.,1.);x*x*(3.-2.*x)}
pub fn gear_rotation(extension:f64,index:usize)->bevy::math::DQuat{
    let angle=smooth(1.-extension)*std::f64::consts::FRAC_PI_2;
    if index==0 {bevy::math::DQuat::from_rotation_x(angle)}else{bevy::math::DQuat::from_rotation_z(if index==1{angle}else{-angle})}
}
pub fn mount(index:usize)->DVec3{match index{0=>DVec3::new(0.,0.,-5.52),1=>DVec3::new(-2.56,0.21,6.27),_=>DVec3::new(2.56,0.21,6.27)}}
pub fn wheel(index:usize)->DVec3{match index{0=>DVec3::new(0.,-1.714,-5.523),1=>DVec3::new(-2.562,-1.63,6.269),_=>DVec3::new(2.56,-1.63,6.268)}}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn actuator_limits_hydraulics_and_step_convergence(){
  let run=|hz:u32|{let mut f=Flight::runway();f.surface=DVec3::new(1.,-0.8,0.3);f.spool=1.;f.engine=true;f.systems.sr71.canopy_command=true;for _ in 0..hz*8{advance(&mut f,1./hz as f64,3.2);}f};
  let a=run(60);let b=run(240);
  for i in 0..4 {assert!((a.systems.sr71.elevons[i]-b.systems.sr71.elevons[i]).abs()<1e-10);assert!(a.systems.sr71.elevons[i]<=0.);}
  assert_eq!(a.systems.sr71.canopy,1.);assert!((a.systems.sr71.spike-0.66).abs()<1e-10);
  let mut f=Flight::runway();f.damage.hydraulics=0.;f.surface=DVec3::ONE;advance(&mut f,1.,3.2);assert_eq!(f.systems.sr71.elevons,[0.;4]);assert_eq!(f.systems.sr71.spike,0.);
 }
 #[test]fn gear_moves_continuously_to_internal_stow_without_twisting_nose(){
  let n=gear_rotation(0.,0)*(wheel(0)-mount(0))+mount(0);assert!(n.z< -7.1&&n.y>-0.1);assert!(n.x.abs()<1e-9);
  for i in 1..3{let p=gear_rotation(0.,i)*(wheel(i)-mount(i))+mount(i);assert!(p.x.abs()<1.);assert!(p.y>0.1);}
  for i in 0..3{let mut old=wheel(i);for step in 1..=100{let e=1.-step as f64/100.;let p=mount(i)+gear_rotation(e,i)*(wheel(i)-mount(i));assert!(p.distance(old)<0.06);old=p;}}
 }
}
