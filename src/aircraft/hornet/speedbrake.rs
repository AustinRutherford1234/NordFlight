//! Super Hornet-style surface speedbrake; deflection schedule is an estimate.
use super::*;
pub fn advance(f:&mut Flight,dt:f64) {
    let approach=f.flaps>0.2;
    let alpha=f.alpha.to_degrees();
    let envelope=((1.5-f.mach)/0.3).clamp(0.,1.)
        *((22.-alpha)/6.).clamp(0.,1.)*((alpha+15.)/6.).clamp(0.,1.);
    let target=if f.airbrake && !(approach&&!f.landed) {envelope}else{0.};
    let authority=(f.damage.hydraulics*f.hornet.hydraulic_pressure).clamp(0.,1.);
    f.hornet.speedbrake+=(target-f.hornet.speedbrake).clamp(-dt*0.5*authority,dt*0.5*authority);
}
pub fn mix(f:&Flight,mut commands:[f64;16])->[f64;16] {
    // Preserve pilot/FCC differential demand before allocating drag-producing bias.
    let b=f.hornet.speedbrake;if b<=0. || f.flaps>0.2 {return commands;}
    for (aileron,rudder,flap,side) in [(2,3,1,1.),(6,7,5,-1.)] {
        let reserve=(1.-commands[aileron].abs()/0.35).clamp(0.,1.);
        commands[aileron]-=b*23_f64.to_radians()*reserve;
        commands[rudder]+=side*b*20_f64.to_radians()*(1.-commands[rudder].abs()/0.4).clamp(0.,1.);
        commands[flap]+=b*if f.landed {7_f64.to_radians()}else{0.23};
    }
    commands
}
#[cfg(test)]mod tests {
 use super::*;
 #[test]fn fbw_remains_bounded_during_speedbrake_deployment(){
  let mut f=Flight::approach();crate::aircraft::select(&mut f,crate::aircraft::Aircraft::Hornet);
  f.flaps=0.;f.gear=false;f.gear_extension=0.;f.gear_door=0.;f.landed=false;
  let trim=trim::solve(&f,3000.,180.,0.,0.).expect("clean flight trim");trim::apply(&mut f,trim);
  let mut clean=f.clone();f.airbrake=true;
  let e=Environment{wind_enabled:false,..Default::default()};let mut peak=0f64;
  for _ in 0..1200 {simulate(&mut f,&e,1./240.);simulate(&mut clean,&e,1./240.);peak=peak.max(f.rates.length());}
  println!("SPEEDBRAKE FLIGHT speed={} clean={} roll={} alpha={} peak_rate={peak}",f.airspeed,clean.airspeed,f.roll().to_degrees(),f.alpha.to_degrees());
  assert!(!f.crashed&&f.orientation.is_finite()&&peak<2.);
  assert!(f.hornet.speedbrake>0.9&&f.airspeed<clean.airspeed-2.);
  assert!(f.roll().abs()<0.15);
 }
 #[test]fn actual_surfaces_and_gear_increase_drag_without_switch_only_force(){
  let mut f=Flight::approach();crate::aircraft::select(&mut f,crate::aircraft::Aircraft::Hornet);
  f.flaps=0.;f.gear_extension=0.;f.gear_door=0.;f.hornet.angles=[0.;16];f.alpha=0.;f.beta=0.;f.rates=DVec3::ZERO;
  f.velocity=f.orientation*DVec3::NEG_Z*150.;f.wind=DVec3::ZERO;f.density=1.;f.assist=false;f.hornet.trim=0.;
  let axis=(f.orientation.conjugate()*f.velocity).normalize();
  let clean=-aero::forces(&mut f,0.).force.dot(axis);
  f.airbrake=true;assert!((-aero::forces(&mut f,0.).force.dot(axis)-clean).abs()<1e-8);
  f.hornet.speedbrake=1.;f.hornet.angles=mix(&f,[0.;16]);
  let brake=-aero::forces(&mut f,0.).force.dot(axis);
  f.hornet.angles=[0.;16];f.gear_extension=1.;
  let gear=-aero::forces(&mut f,0.).force.dot(axis);
  println!("DRAG at 150m/s rho1 clean={clean} surface_brake={brake} gear={gear} N");
  assert!(brake>clean+15000.);assert!((gear-clean-11250.*2.8).abs()<1.);
 }
 #[test]fn surfaces_deploy_gradually_and_follow_limits(){
  let mut f=Flight::approach();crate::aircraft::select(&mut f,crate::aircraft::Aircraft::Hornet);f.airbrake=true;f.flaps=0.;f.mach=0.6;f.alpha=0.1;
  advance(&mut f,1.);assert!((f.hornet.speedbrake-0.5).abs()<1e-8);
  let c=mix(&f,[0.;16]);assert!(c[2]<0.&&c[6]<0.&&c[3]>0.&&c[7]<0.&&c[1]>0.);
  let mut demand=[0.;16];demand[2]=0.35;assert_eq!(mix(&f,demand)[2],0.35);
  f.flaps=0.55;advance(&mut f,1.);assert_eq!(f.hornet.speedbrake,0.);
  f.flaps=0.;f.mach=1.6;advance(&mut f,2.);assert_eq!(f.hornet.speedbrake,0.);
  f.mach=0.6;f.damage.hydraulics=0.;advance(&mut f,2.);assert_eq!(f.hornet.speedbrake,0.);
 }
}
