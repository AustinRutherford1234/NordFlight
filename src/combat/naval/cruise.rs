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
    let ahead=position+horizontal*speed*1.5;
    let height=|p:DVec3| if carrier::sea(p.x,p.z) {
        crate::ocean::sample(p,time).height.max(0.)
    }else{terrain_height(p.x,p.z).max(0.)};
    let floor=height(position).max(height(ahead));
    let error=floor+clearance-(position.length()-RADIUS);
    // Descend early after high-altitude release. Arrest descent before reaching
    // the surface rather than aiming down at a distant point on the sea.
    let vertical=(error/4.).clamp(-60.,25.);
    let vertical_accel=((vertical-velocity.dot(up))*1.5).clamp(-12.,25.);
    let yaw=(bearing-horizontal*horizontal.dot(bearing))*speed*2.;
    let gravity=up*9.81;
    let request=yaw+up*(vertical_accel-speed.powi(2)/position.length())+gravity;
    (request-direction*request.dot(direction)).clamp_length_max(6.*9.81)
}

#[cfg(test)] mod tests {
 use super::*;
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
