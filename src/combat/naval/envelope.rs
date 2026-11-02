//! Estimated simulation envelopes, not published weapon performance claims.
use super::*;
pub const STRIKE_RANGE:f64=150000.;
pub const CRUISE_BURN:f64=620.;
pub const CRUISE_LIFE:f64=720.;
/// Conservative average-speed/time budget. Outer range is geometry-dependent;
/// terminal acquisition still requires the existing radar/seeker tests.
pub fn sam_reachable(kind:Kind,launch:DVec3,target:DVec3,velocity:DVec3)->bool {
 let (cap,speed,time)=match kind {Kind::Standard=>(70000.,600.,85.),Kind::Essm=>(30000.,650.,42.),_=>return false};
 let r=target-launch;let range=r.length();
 let closing=speed-velocity.dot(r.normalize_or_zero());
 let climb=(target.length()-launch.length()).max(0.);
 range<=cap && closing>100. && (range+climb*0.5)/closing<time
}
#[cfg(test)]mod tests {
 use super::*;
 #[test]fn sam_launch_limits_account_for_aspect_and_climb(){
  let p=DVec3::Y*(RADIUS+30.);let t=p+DVec3::NEG_Z*65000.+DVec3::Y*9970.;
  assert!(sam_reachable(Kind::Standard,p,t,DVec3::Z*480.));
  assert!(!sam_reachable(Kind::Standard,p,t,DVec3::NEG_Z*480.));
  assert!(!sam_reachable(Kind::Essm,p,t,DVec3::Z*480.));
  assert!(!sam_reachable(Kind::Standard,p,p+DVec3::NEG_Z*71000.,DVec3::Z*480.));
 }
}
