//! Estimated ditching and flooding model; SI units, spherical sea-level surface.
use crate::physics::*;
use bevy::math::{DVec3,DQuat};
#[derive(Clone,Debug,Default)]
pub struct State {pub active:bool,pub age:f64,pub flooding:f64,pub origin:DVec3,pub speed:f64,pub sink:f64}
pub fn surface(f:&Flight)->bool {
 (in_lake(f.position.x,f.position.z)||crate::carrier::sea(f.position.x,f.position.z))
 && crate::carrier::deck(f,f.position).is_none()
}
pub fn step(f:&mut Flight,dt:f64)->bool {
 if f.vehicle!=Vehicle::Gripen || !dt.is_finite() || dt<=0. {return false;}
 if !f.water.active {
  if !surface(f){return false;}
  let reach=crate::collision::wheel_clearance(f).max(0.65+f.aircraft.spec().span*0.5*f.roll().sin().abs());
  if f.altitude()>reach{return false;}
  let up=f.up();let vertical=f.velocity.dot(up);let speed=(f.velocity-up*vertical).length();
  let gentle=speed<=85. && vertical>=-6. && f.roll().abs()<20f64.to_radians() && f.pitch()>-5f64.to_radians() && f.pitch()<18f64.to_radians();
  f.water=State{active:true,age:0.,flooding:if gentle{0.}else{0.65},origin:up*RADIUS,speed,sink:(-vertical).max(0.)};
  if !gentle && !f.crashed {crate::collision::destroy(f,"WATER IMPACT",up);}
  if gentle {
   f.integrity=(f.integrity-0.06-speed.powi(2)*0.000025-(-vertical).max(0.).powi(2)*0.003).max(0.15);
   if f.gear_extension>0.1 && speed>18. {f.gear_health=[0.;3];}
   if speed>60. || vertical< -3.5 {
    let side=usize::from(f.roll()>=0.);
    f.damage.wing_broken[side]=true;f.damage.wings[side]=0.;
   }
  }
  f.engine=false;f.systems.engine_on=[false;2];f.throttle=0.;f.thrust=0.;f.spool=0.;
  f.autopilot=false;f.carrier.attached=false;f.carrier.launch=None;f.carrier.arrested=false;
 }
 f.engine=false;f.systems.engine_on=[false;2];f.throttle=0.;f.thrust=0.;f.spool=0.;f.autopilot=false;
 let up=f.up();f.water.age+=dt;f.elapsed+=dt;f.landed=true;
 f.damage.fire=0.;
 // Flooding progressively removes displaced-air volume. Water drag is
 // integrated exponentially so a fast impact cannot reverse velocity.
 f.water.flooding=(f.water.flooding+dt*(0.022+if f.damage.wing_broken.iter().any(|b|*b){0.018}else{0.})).min(1.);
 let immersion=((1.1-f.altitude())/1.8).clamp(0.,1.);
 let v=f.velocity.dot(up);let tangent=f.velocity-up*v;
 let horizontal=tangent*(-dt*(0.10+0.009*tangent.length())*immersion).exp();
 let buoyancy=9.81*1.12*immersion*(1.-0.95*f.water.flooding);
 let vertical=(v+(buoyancy-9.81)*dt)*(-dt*2.2*immersion).exp();
 f.velocity=horizontal+up*vertical;f.position+=f.velocity*dt;
 let body_up=f.orientation.conjugate()*up;
 f.rates+=(DVec3::Y.cross(body_up)*1.4-f.rates*2.)*dt;
 f.orientation=(f.orientation*DQuat::from_scaled_axis(f.rates*dt)).normalize();
 f.airspeed=f.velocity.length();f.g_load=buoyancy/9.81;
 f.status=if f.altitude()< -2.{"SINKING / R TO RESET"}else if f.crashed{"WATER IMPACT / FLOODING / R TO RESET"}else{"DITCHED / FLOODING / R TO RESET"}.into();
 true
}
#[cfg(test)]
mod tests {
 use super::*;
 fn aircraft(speed:f64,sink:f64)->Flight {let mut f=crate::carrier::scenario(false);f.carrier.active=false;f.position=crate::carrier::world(DVec3::new(1200.,0.5,0.),0.);f.orientation=crate::carrier::rotation(0.);f.gear=false;f.gear_extension=0.;f.velocity=f.orientation*DVec3::new(0.,-sink,-speed);f}
 #[test] fn gentle_contact_floats_then_sinks_without_explosion(){let mut f=aircraft(40.,1.);assert!(step(&mut f,1./240.));assert!(!f.crashed);for _ in 0..240*4{step(&mut f,1./240.);}assert!(f.altitude()>-2.,"{}",f.altitude());for _ in 0..240*50{step(&mut f,1./240.);}assert!(f.altitude()< -10.);assert_eq!(f.damage.fire,0.);assert!(f.position.is_finite());}
 #[test] fn water_state_resets_with_a_new_flight(){let mut f=aircraft(40.,1.);step(&mut f,1./240.);assert!(f.water.active);f=Flight::runway();assert!(!f.water.active);}
 #[test] fn ditching_supports_other_aircraft_and_prevents_underwater_restart(){for a in [crate::aircraft::Aircraft::Gripen,crate::aircraft::Aircraft::F22]{let mut f=aircraft(40.,1.);crate::aircraft::select(&mut f,a);step(&mut f,1./240.);assert!(f.water.active&&!f.crashed);f.engine=true;f.throttle=1.;step(&mut f,1./240.);assert!(!f.engine&&f.thrust==0.);}}
 #[test] fn excessive_speed_breaks_up_and_sinks(){let mut f=aircraft(150.,15.);step(&mut f,1./240.);assert!(f.crashed&&f.water.active);assert_eq!(f.damage.fire,0.);}
 #[test] fn exposed_gear_and_rough_ditch_can_break_parts(){let mut f=aircraft(70.,4.);f.gear_extension=1.;step(&mut f,1./240.);assert!(!f.crashed);assert_eq!(f.gear_health,[0.;3]);assert!(f.damage.wing_broken.iter().any(|v|*v));}
 #[test] fn carrier_deck_does_not_enter_water_state(){let mut f=crate::carrier::scenario(true);assert!(!step(&mut f,1./240.));assert!(!f.water.active);}
 #[test] fn sinking_is_consistent_across_frame_rates(){let mut depths=vec![];for hz in [60,120,240]{let mut f=aircraft(40.,1.);for _ in 0..hz*40{crate::physics::simulate(&mut f,&Environment::default(),1./hz as f64);}depths.push(f.altitude());}assert!((depths[0]-depths[2]).abs()<0.1,"{depths:?}");}
}
