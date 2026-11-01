//! Su-27-specific reduced-order aero/propulsion/FBW parameters. Public envelope
//! anchors are separate from estimated derivatives; shared rigid-body/contact
//! integration and physical surface allocation remain authoritative.
use crate::{physics::*,aerodynamics::{Conditions,Coefficients,FlowState}};
use bevy::math::DVec3;
fn smooth(a:f64,b:f64,x:f64)->f64 {let t=((x-a)/(b-a)).clamp(0.,1.);t*t*(3.-2.*t)}
/// AL-31-class twin installed thrust estimate: spool, density, ram recovery,
/// afterburner and high-Mach inlet losses. Per-engine failures use existing plumbing.
pub fn engine(power:f64,rho:f64,mach:f64)->(f64,f64) {
    let power=power.clamp(0.,1.);let m=mach.max(0.);
    let lapse=(rho.max(0.)/1.225).powf(0.62)*(1.+0.40*m*m)/(1.+0.14*m*m*m);
    let recovery=1.-0.28*smooth(2.,2.6,m);
    let dry=149000.*(power/0.88).clamp(0.,1.).powf(1.4)*lapse*recovery;
    let wet=96000.*((power-0.88)/0.12).clamp(0.,1.)*lapse*recovery;
    (dry+wet,wet)
}
/// Preserve unsteady separation/vortex state, replace generic aspect-ratio and
/// wave-drag terms with Su-27 geometry-specific estimates, without double counting.
pub fn evaluate(c:Conditions,state:&mut FlowState,dt:f64)->Coefficients {
    let mut a=crate::aerodynamics::evaluate(c,state,dt);
    let induced=a.lift*a.lift/(std::f64::consts::PI*(14.7*14.7/62.)*0.79)
        *(1.+0.65*(1.-state.attached));
    let wave=0.024*(-((c.mach-1.07)/0.18).powi(2)).exp()+0.018*smooth(0.98,1.55,c.mach);
    a.drag=(a.drag-a.induced-a.wave+induced+wave).max(0.);
    a.induced=induced;a.wave=wave;
    // Fin effectiveness and static directional stability fall in deep separation.
    a.yaw-=0.06*c.beta*(0.3+0.7*state.attached);
    a
}
/// Load-factor/roll-rate command approximation, coordinated yaw damping, dynamic
/// pressure and AoA/load limiting. This is not the proprietary Su-27 control law.
pub fn target_rates(f:&Flight,command:DVec3)->DVec3 {
    let q=0.5*f.density*f.airspeed*f.airspeed;
    let authority=(q/9000.).clamp(0.15,1.);
    let alpha_limit=0.46-0.12*smooth(0.8,1.6,f.mach);
    let nz=if command.x>=0. {1.+command.x*8.}else{1.+command.x*4.};
    let coordinated=9.80665*f.roll().tan().clamp(-5.,5.)/f.airspeed.max(80.);
    let mut pitch=command.x*0.28+(nz-f.g_load)*0.09;
    // Return toward the trimmed incidence when neutral; do not impose attitude.
    if command.x.abs()<0.03 {pitch+=(0.06-f.alpha)*0.3;}
    pitch=pitch.min((alpha_limit-f.alpha)*1.8);
    if f.g_load>9. {pitch=pitch.min(-(f.g_load-9.)*0.15);}
    if f.g_load< -3. {pitch=pitch.max((-3.-f.g_load)*0.15);}
    let roll_limit=2.1*(1.-0.55*smooth(0.25,0.5,f.alpha.abs()));
    let roll=-command.z*roll_limit*authority;
    let yaw=command.y*0.25-f.beta*0.8-coordinated*f.roll().cos();
    DVec3::new((pitch+coordinated*f.roll().sin()).clamp(-0.45,0.65),yaw.clamp(-0.4,0.4),roll)
}
#[cfg(test)]mod tests {
 use super::*;
 #[test]fn engine_has_dry_wet_lapse_and_inlet_loss(){assert_eq!(engine(0.,1.225,0.).0,0.);assert!((engine(1.,1.225,0.).0-245000.).abs()<1.);assert!(engine(1.,0.3,0.).0<engine(1.,1.225,0.).0);assert!(engine(1.,0.3,2.7).0<engine(1.,0.3,2.).0);}
 #[test]fn fbw_limits_load_incidence_and_retains_roll_sign(){let mut f=crate::combat::flight();crate::aircraft::select(&mut f,crate::aircraft::Aircraft::Su27);f.g_load=10.;f.alpha=0.5;let r=target_rates(&f,DVec3::new(1.,0.,1.));assert!(r.x<0.&&r.z<0.);}
}

#[cfg(test)]mod flight_checks {
 use super::*;
 #[test] fn high_altitude_acceleration_is_physical_and_finite() {
  for bomber in [false,true] {
   let mut f=crate::combat::bvr::flight();crate::aircraft::select(&mut f,crate::aircraft::Aircraft::Su27);
   if bomber {crate::aircraft::tu160::initialize(&mut f,24);}
   f.paused=false;let e=Environment{wind_enabled:false,..Default::default()};
   let start=f.velocity.length();let mut peak=0f64;
   for _ in 0..36000 {
    let up=f.up();let desired=(f.forward()-up*f.forward().dot(up)).normalize();
    if bomber {crate::aircraft::tu160::pilot(&mut f,desired,500.,1./120.);}
    else {crate::combat::ai::command(&mut f,desired,550.,crate::combat::dogfight::Difficulty::Hard,1./120.);}
    simulate(&mut f,&e,1./120.);peak=peak.max(f.mach);
    assert!(!f.crashed,"bomber={bomber} {} alt={} M={}",f.status,f.altitude(),f.mach);
   }
   println!("ACCEL bomber={bomber} initial_speed={start} final_speed={} peak_M={peak} altitude={} throttle={} fuel={}",f.airspeed,f.altitude(),f.throttle,f.fuel);
   assert!(peak>1.1,"bomber={bomber} peak={peak}");
  }
 }
}

#[cfg(test)]mod numerical_checks {
 use super::*;
 #[test] fn polar_is_finite_continuous_and_drag_nonnegative_through_transonic_range() {
  let mut previous:Option<Coefficients>=None;
  for i in 0..1001 {
   let c=Conditions{alpha:0.08,beta:0.01,mach:0.5+i as f64*0.002,speed:300.,density:0.4,temperature:220.,flaps:0.,gear:false,airbrake:false,height:8000.,rates:DVec3::ZERO};
   let a=evaluate(c,&mut FlowState::default(),0.);
   assert!(a.drag.is_finite()&&a.drag>0.&&a.pitch.is_finite());
   if let Some(p)=previous {assert!((a.drag-p.drag).abs()<0.005);}
   previous=Some(a);
  }
 }
 #[test] fn nominal_maneuver_converges_and_engine_shutdown_removes_power() {
  let mut states=vec![];
  for hz in [60,120,240] {
   let mut f=crate::combat::bvr::flight();crate::aircraft::select(&mut f,crate::aircraft::Aircraft::Su27);f.paused=false;
   f.controls=DVec3::new(0.1,0.,0.15);f.throttle=0.95;
   let e=Environment{wind_enabled:false,..Default::default()};
   for _ in 0..hz*10 {simulate(&mut f,&e,1./hz as f64);}
   assert!(!f.crashed&&f.position.is_finite());states.push(f);
  }
  for f in &states[..2] {assert!(f.position.distance(states[2].position)<50.);assert!(f.velocity.distance(states[2].velocity)<states[2].velocity.length()*0.02);}
  let mut f=states.pop().unwrap();f.engine=false;simulate(&mut f,&Environment::default(),1./120.);assert_eq!(f.thrust,0.);
 }
}
