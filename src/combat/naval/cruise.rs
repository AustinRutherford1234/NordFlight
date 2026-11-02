//! Estimated radar-altimeter/INS altitude channel, not a particular weapon's OFP.
use super::*;
pub(super) fn acceleration(position:DVec3, velocity:DVec3, goal:DVec3, time:f64)->DVec3 {
    let up=position.normalize();
    let speed=velocity.length().max(1.);
    let direction=velocity/speed;
    let horizontal=(velocity-up*velocity.dot(up)).normalize_or(direction);
    let to=goal-position;
    let bearing=(to-up*to.dot(up)).normalize_or(horizontal);
    let range=to.length();
    // Smooth terminal transition; heights are conservative simulation estimates.
    let clearance=5.+7.*((range-3000.)/5000.).clamp(0.,1.);
    // A radar-altimeter channel remains independent of target acquisition.
    // Preview a fixed, bounded set of terrain samples (no allocations). The
    // former 1.5-second preview could meet a coastline before the aircraft-like
    // altitude response had time to climb. Land gets extra obstacle clearance;
    // open-water cruise/terminal heights remain 12/5 m.
    let mut target_height=if carrier::sea(position.x,position.z) {
        clearance+crate::ocean::sample(position,time).height.max(0.)
    }else{terrain_height(position.x,position.z).max(0.)+30.};
    for seconds in [2.,4.,8.,12.,20.,30.,40.] {
        // Stop preview at the navigation goal: terrain beyond the target ship
        // must not pull a terminal sea-skimmer upward before impact.
        let ahead=position+horizontal*(speed*seconds).min(range);
        let desired=if carrier::sea(ahead.x,ahead.z) {clearance}
            else {terrain_height(ahead.x,ahead.z).max(0.)+30.};
        target_height=target_height.max(desired);
    }
    let height=position.length()-RADIUS;
    let error=target_height-height;
    // Retain the bounded physical climb/descent response. Preview raises the
    // commanded altitude early, rather than increasing force or snapping height.
    let vertical=(error/4.).clamp(-60.,25.);
    let vertical_accel=((vertical-velocity.dot(up))*1.5).clamp(-12.,25.);
    let yaw=(bearing-horizontal*horizontal.dot(bearing))*speed*2.;
    let gravity=up*9.81;
    let request=yaw+up*(vertical_accel-speed.powi(2)/position.length())+gravity;
    (request-direction*request.dot(direction)).clamp_length_max(6.*9.81)
}

#[cfg(test)] mod tests {
 use super::*;
 #[test] fn sea_to_land_transition_clears_the_coast() {
  for hz in [60,120,240] {
   let mut p=crate::terrain::point(14500.,-40000.,14.);
   let mut v=-DVec3::X*290.;
   let goal=crate::terrain::point(0.,-40000.,70.);
   for k in 0..hz*35 {
    let speed=v.length();let direction=v/speed;
    let thrust=if speed<290. {45.}else{0.};
    let a=acceleration(p,v,goal,k as f64/hz as f64)-p.normalize()*9.81
      +direction*(thrust-0.000025*speed*speed);
    v+=a/hz as f64;p+=v/hz as f64;
    let floor=terrain_height(p.x,p.z).max(0.);
    assert!(p.length()-RADIUS>floor+1.,"coast collision hz={hz} t={} h={} floor={floor}",k as f64/hz as f64,p.length()-RADIUS);
   }
  }
 }
 #[test] fn high_release_descends_then_tracks_sea_skimming_height() {
  for hz in [60,120,240] {
   let mut p=carrier::world(DVec3::new(0.,10000.,-120000.),0.);
   let mut v=carrier::rotation(0.)*DVec3::Z*290.;
   let mut low=f64::INFINITY;let mut high=0_f64;
   for k in 0..hz*240 {
    let t=k as f64/hz as f64;let goal=carrier::world(DVec3::Y*5.,t);
    let a=acceleration(p,v,goal,t)-p.normalize()*9.81;
    // Hold tangential speed for this altitude-channel unit fixture only.
    v+=a/hz as f64;let up=p.normalize();let vertical=v.dot(up);
    v=(v-up*vertical).normalize()*290.+up*vertical;p+=v/hz as f64;
    let height=p.length()-RADIUS;
    assert!(height>1.,"sea impact hz={hz} t={t} h={height}");
    if t>210. {low=low.min(height);high=high.max(height);}
   }
   println!("SKIM hz={hz} low={low} high={high}");assert!(low>5.&&high<20.);
  }
 }
}
