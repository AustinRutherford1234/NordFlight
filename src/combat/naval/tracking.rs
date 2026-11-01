//! Estimated naval RF seekers. Measurements, support and navigation memory drive
//! the existing force-limited weapon plant; target truth is reserved for sensors/fuzes.
use super::*;
const SENSOR_DT: f64 = 0.05;
const MEMORY_SECONDS: f64 = 6.;
pub(super) fn seed(m: &mut Round, p: DVec3, v: DVec3) {
    let g=&mut m.visual.gnc;g.maritime=m.kind==Kind::AntiShip;g.estimate=p;g.estimated_velocity=v;
    g.initialized=true;g.confidence=0.65;g.seed=m.id.wrapping_mul(0x517cc1b727220a95);
}
pub(super) fn update(m: &mut Round, target: Option<guidance::Contact>, support: Option<(DVec3,DVec3)>,
    decoys: &[dogfight::Decoy], sun: DVec3, dt: f64) -> Option<(DVec3,DVec3)> {
    use guidance::Mode;
    let body=m.velocity.normalize_or(DVec3::NEG_Z);
    let v=&mut m.visual;v.position=m.position;v.velocity=m.velocity;v.age=m.age;
    v.gnc.body=body;v.gnc.seed=m.id.wrapping_mul(0x517cc1b727220a95);
    if v.gnc.gimbal.length_squared()<0.5 {v.gnc.gimbal=body;}
    v.gnc.estimate+=v.gnc.estimated_velocity*dt;
    v.lost_for+=dt;
    let predicted_range=if v.gnc.initialized {v.gnc.estimate.distance(m.position)}else{f64::INFINITY};
    let terminal_range=match m.kind {Kind::Standard=>18000.,Kind::Essm=>12000.,Kind::AntiShip=>16000.,Kind::AirToAir=>18000.};
    if predicted_range<terminal_range && m.age>0.6 {v.gnc.active=true;}
    // Search around the last predicted LOS, with bounded gimbal travel and slew.
    let predicted=(v.gnc.estimate-m.position).normalize_or(body);
    let aim=if v.gnc.initialized {predicted}else{body};
    let angle=body.angle_between(aim);
    let bounded=if angle>60f64.to_radians() {guidance::rotate_toward(body,aim,60f64.to_radians(),1.)}else{aim};
    v.gnc.gimbal=guidance::rotate_toward(v.gnc.gimbal,bounded,80f64.to_radians(),dt);
    if m.age+1e-9>=m.next_sensor {
        m.next_sensor=((m.age/SENSOR_DT+1e-8).floor()+1.)*SENSOR_DT;
        let mut measured=None;
        let mut reason="RADAR SUPPORT LOST";
        let mut signal=0.;
        if v.gnc.active {
            if let Some(t)=target.filter(|t|t.alive) {
                // Check real sensor geometry, horizon/terrain, range, Doppler and return.
                let los=(t.position-m.position).normalize_or_zero();
                let geometry=m.position.distance(t.position)<terminal_range*1.15
                    && body.dot(los)>60f64.to_radians().cos()
                    && v.gnc.gimbal.dot(los)>10f64.to_radians().cos();
                if geometry {
                    let q=if v.gnc.maritime {guidance::maritime_quality(m.position,t)}else{guidance::quality(Weapon::Radar,m.position,t,sun)};
                    signal=q.0;reason=q.1;
                }else{reason="SEEKER RANGE / GIMBAL";}
                if v.decoy.is_some_and(|id| !decoys.iter().any(|d|d.id==id && d.kind==Weapon::Radar
                    && guidance::retain_decoy(v,t,dogfight::decoy_contact(d),d.age,sun))) {v.decoy=None;}
                if v.decoy.is_none() && geometry && v.gnc.evaluated_decoys.len()<64 {
                    for d in decoys.iter().filter(|d|d.kind==Weapon::Radar) {
                        if guidance::consider_decoy(v,t,dogfight::decoy_contact(d),d.age,d.id,sun) {v.decoy=Some(d.id);break;}
                        if v.gnc.evaluated_decoys.len()>=64 {break;}
                    }
                }
                let tracked=v.decoy.and_then(|id|decoys.iter().find(|d|d.id==id)).map(dogfight::decoy_contact).unwrap_or(t);
                if v.decoy.is_some() {
                    let los=(tracked.position-m.position).normalize_or_zero();
                    signal=if m.position.distance(tracked.position)<terminal_range*1.15 && body.dot(los)>60f64.to_radians().cos()
                        && v.gnc.gimbal.dot(los)>10f64.to_radians().cos() {if v.gnc.maritime {guidance::maritime_quality(m.position,tracked).0}else{guidance::quality(Weapon::Radar,m.position,tracked,sun).0}}else{0.};
                    reason="COUNTERMEASURE TRACK";
                }
                if signal>=0.18 {measured=Some((tracked.position,tracked.velocity));}
            }else{reason="TARGET LOST";v.decoy=None;}
        }
        // A datalink uses the launcher's measured track, never live target truth.
        if measured.is_none() {
            if let Some(p)=support {measured=Some(p);signal=0.8;reason=if v.gnc.active {"SEEKER LOST / DATALINK"}else{"RADAR DATALINK UPDATE"};v.decoy=None;}
        }
        v.gnc.signal=signal;v.gnc.reason=reason;
        if let Some((p,velocity))=measured {
            let residual=p-v.gnc.estimate;
            v.gnc.estimate=if v.gnc.initialized {v.gnc.estimate+residual*0.65}else{p};
            v.gnc.estimated_velocity=velocity;v.gnc.initialized=true;
            v.lost_for=0.;v.gnc.confidence=(v.gnc.confidence+SENSOR_DT*2.*signal).min(1.);
        }
    }
    // Anti-ship weapons navigate to the launch solution before terminal search.
    let inertial=m.kind==Kind::AntiShip && !v.gnc.active && v.gnc.initialized;
    if inertial {v.gnc.reason="INERTIAL SEA-SKIM / SEEKER STANDBY";v.lost_for=0.;}
    else if v.lost_for>SENSOR_DT*1.5 {v.gnc.confidence=(v.gnc.confidence-dt*0.18).max(0.);}
    v.guided=v.gnc.initialized && v.gnc.confidence>0.05 && v.lost_for<MEMORY_SECONDS;
    v.gnc.mode=if m.age<0.6 && matches!(m.source,TargetId::Ship(_)) {Mode::Separation}
        else if !v.guided {Mode::Ballistic}
        else if inertial {Mode::Inertial}
        else if v.lost_for>SENSOR_DT*1.5 || v.gnc.signal<0.18 {Mode::Memory}
        else if v.decoy.is_some() {Mode::Decoy}
        else if v.gnc.reason.contains("DATALINK") {Mode::Supported}
        else {Mode::Active};
    if v.guided {Some((v.gnc.estimate,v.gnc.estimated_velocity))}else{None}
}

#[cfg(test)] mod tests {
 use super::*;
 fn scene(kind:Kind,range:f64)->(Round,guidance::Contact) {
  let p=DVec3::Y*(RADIUS+8000.);let velocity=-DVec3::Z*500.;
  let t=guidance::Contact{position:p-DVec3::Z*range,velocity:DVec3::Z*180.,forward:DVec3::Z,
   throttle:1.,engine:true,alive:true,decoy:false,strength:1.};
  let mut m=Round{sam:Default::default(),id:1,source:TargetId::Ship(1),target:TargetId::Player,kind,position:p,previous:p,velocity,
   age:2.,alive:true,support_lost:0.,next_sensor:0.,visual:render_round(p,velocity)};
  seed(&mut m,t.position,t.velocity);(m,t)
 }
 fn step(m:&mut Round,t:Option<guidance::Contact>,support:Option<(DVec3,DVec3)>,decoys:&[dogfight::Decoy],dt:f64)->Option<(DVec3,DVec3)> {
  m.age+=dt;update(m,t,support,decoys,-DVec3::Y,dt)
 }
 #[test] fn datalink_loss_predicts_memory_instead_of_following_hidden_target_and_can_reacquire() {
  let(mut m,mut t)=scene(Kind::Standard,60000.);
  for _ in 0..120{step(&mut m,Some(t),Some((t.position,t.velocity)),&[],1./120.);}
  assert_eq!(m.visual.gnc.mode,guidance::Mode::Supported);
  let last=m.visual.gnc.estimate;t.position+=DVec3::X*30000.;
  for _ in 0..60{step(&mut m,Some(t),None,&[],1./120.);}
  assert_eq!(m.visual.gnc.mode,guidance::Mode::Memory);
  assert!((m.visual.gnc.estimate-last).x.abs()<1.);assert!(m.visual.gnc.estimate.distance(t.position)>29000.);
  for _ in 0..800{step(&mut m,Some(t),None,&[],1./120.);}
  assert_eq!(m.visual.gnc.mode,guidance::Mode::Ballistic);assert!(!m.visual.guided);
  for _ in 0..60{step(&mut m,Some(t),Some((t.position,t.velocity)),&[],1./120.);}
  assert_eq!(m.visual.gnc.mode,guidance::Mode::Supported);assert!(m.visual.guided);
 }
 #[test] fn terminal_seeker_loses_and_reacquires_after_gimbal_masking() {
  let(mut m,t)=scene(Kind::Essm,9000.);
  for _ in 0..120{step(&mut m,Some(t),None,&[],1./120.);}
  assert!(m.visual.gnc.active);assert_eq!(m.visual.gnc.mode,guidance::Mode::Active);
  let mut masked=t;masked.position=m.position+DVec3::Z*9000.;
  for _ in 0..60{step(&mut m,Some(masked),None,&[],1./120.);}
  assert_eq!(m.visual.gnc.mode,guidance::Mode::Memory);
  for _ in 0..60{step(&mut m,Some(t),None,&[],1./120.);}
  assert_eq!(m.visual.gnc.mode,guidance::Mode::Active);
 }
 #[test] fn anti_ship_ins_uses_launch_solution_before_terminal_radar() {
  let(mut m,t)=scene(Kind::AntiShip,40000.);
  let mut hidden=t;hidden.position+=DVec3::X*10000.;
  for _ in 0..120{step(&mut m,Some(hidden),None,&[],1./120.);}
  assert_eq!(m.visual.gnc.mode,guidance::Mode::Inertial);assert!(!m.visual.gnc.active);
  assert!((m.visual.gnc.estimate-t.position).x.abs()<1.);
  m.position=t.position+DVec3::Z*10000.;
  for _ in 0..120{step(&mut m,Some(t),None,&[],1./120.);}
  assert_eq!(m.visual.gnc.mode,guidance::Mode::Active);
 }
 #[test] fn chaff_capture_is_seeded_bounded_and_expiry_allows_true_target_reacquisition() {
  let(mut m,mut t)=scene(Kind::AirToAir,2000.);t.strength=0.1;
  let decoys:Vec<_>=(1..=64).map(|id|dogfight::Decoy{owner:None,id,kind:Weapon::Radar,
   position:t.position+DVec3::X*10.,velocity:DVec3::ZERO,age:0.2}).collect();
  for _ in 0..60{step(&mut m,Some(t),None,&decoys,1./120.);}
  assert_eq!(m.visual.gnc.mode,guidance::Mode::Decoy);assert!(m.visual.decoy.is_some());
  assert!(m.visual.gnc.evaluated_decoys.len()<=64);
  for _ in 0..120{step(&mut m,Some(t),None,&[],1./120.);}
  assert_eq!(m.visual.gnc.mode,guidance::Mode::Active);assert!(m.visual.decoy.is_none());
 }
 #[test] fn maritime_seeker_tracks_a_stationary_ship_without_the_airborne_doppler_notch() {
  let(mut m,mut t)=scene(Kind::AntiShip,10000.);
  m.position=carrier::world(DVec3::new(0.,1000.,0.),0.);
  m.velocity=carrier::rotation(0.)*DVec3::NEG_Z*290.;
  t.position=carrier::world(DVec3::new(0.,5.,-10000.),0.);t.velocity=DVec3::ZERO;t.strength=100.;
  seed(&mut m,t.position,t.velocity);
  assert!(guidance::quality(Weapon::Radar,m.position,t,-DVec3::Y).0<0.18);
  assert!(guidance::maritime_quality(m.position,t).0>0.18);
  assert!(guidance::maritime_quality(carrier::world(DVec3::new(0.,18.,0.),0.),t).0>0.18);
  for _ in 0..120{step(&mut m,Some(t),None,&[],1./120.);}
  assert_eq!(m.visual.gnc.mode,guidance::Mode::Active);
  assert_eq!(m.visual.gnc.reason,"MARITIME RADAR RETURN");
  assert!(m.visual.gnc.signal>=0.18);
 }
 #[test] fn target_destruction_does_not_teleport_or_delete_a_weapon_in_flight() {
  let(mut m,t)=scene(Kind::Essm,9000.);step(&mut m,Some(t),None,&[],0.05);
  let before=m.position;
  for _ in 0..20{step(&mut m,None,None,&[],0.05);}
  assert!(m.alive);assert_eq!(m.position,before);assert_eq!(m.visual.gnc.mode,guidance::Mode::Memory);
 }
}
