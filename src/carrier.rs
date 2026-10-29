//! First playable carrier: estimated deck contacts, forgiving wire and catapult mechanics.
use crate::{physics::*,aircraft::Aircraft};
use bevy::{prelude::*,math::{DVec3,DQuat}};
pub const DECK:f64=21.2*(333./303.842285);
pub const SPEED:f64=12.86; // 25 knots, constant straight course.
const SCALE:f32=333./303.842285;
#[derive(Clone,Debug)]
pub struct State {pub active:bool,pub guidance:bool,pub attached:bool,pub launch:Option<f64>,pub arrested:bool,pub arrest_speed:f64,pub arrest_distance:f64,pub wire:usize,pub deck_position:DVec3}
impl Default for State {fn default()->Self {Self{active:false,guidance:true,attached:false,launch:None,arrested:false,arrest_speed:0.,arrest_distance:0.,wire:0,deck_position:DVec3::ZERO}}}
pub fn origin(t:f64)->DVec3 {surface_point(20000.,-16000.-SPEED*t,0.)}
pub fn rotation(t:f64)->DQuat {DQuat::from_rotation_arc(DVec3::Y,origin(t).normalize())}
pub fn velocity(t:f64)->DVec3 {rotation(t)*DVec3::NEG_Z*SPEED}
pub fn relative(p:DVec3,t:f64)->DVec3 {rotation(t).conjugate()*(p-origin(t))}
pub fn world(p:DVec3,t:f64)->DVec3 {origin(t)+rotation(t)*p}
pub fn sea(x:f64,z:f64)->bool {(x-20000.).abs()<7000. && z< -8000.}
const HULL:&[(f64,f64)]=&[(-39.17,36.52),(-37.84,-60.21),(-12.52,-162.53),(-10.34,-165.6),(11.32,-165.6),(13.49,-162.53),(38.68,-51.23),(38.86,72.93),(37.53,115.04),(29.13,149.4),(24.06,163.14),(3.52,165.61),(-14.17,164.21),(-34.19,122.28)];
pub fn inside(p:DVec3)->bool {(0..HULL.len()).all(|i| {let a=HULL[i];let b=HULL[(i+1)%HULL.len()];(b.0-a.0)*(p.z-a.1)-(b.1-a.1)*(p.x-a.0)>=0.})}
pub fn deck(f:&Flight,p:DVec3)->Option<(f64,DVec3)> {
 if !f.carrier.active{return None;}
 let q=relative(p,f.elapsed);
 (inside(q)&&q.y>DECK-4.).then(||((world(DVec3::new(q.x,DECK,q.z),f.elapsed).length()-RADIUS),rotation(f.elapsed)*DVec3::Y))
}
pub fn ground(f:&Flight,p:DVec3)->f64 {deck(f,p).map_or_else(||terrain_height(p.x,p.z).max(0.),|v|v.0)}
pub fn normal(f:&Flight,p:DVec3)->DVec3 {deck(f,p).map_or_else(||crate::terrain::normal(p),|v|v.1)}
pub fn ground_velocity(f:&Flight,p:DVec3)->DVec3 {if deck(f,p).is_some(){velocity(f.elapsed)}else{DVec3::ZERO}}
// Angled landing lane: target x=-8 at z=75; bow is -Z.
pub fn lane(z:f64)->f64 {-8.+(z-75.)*0.14}
pub fn scenario(deck_start:bool)->Flight {
 let mut f=Flight::approach();crate::aircraft::select(&mut f,Aircraft::Hornet);
 f.carrier.active=true;f.flaps=0.55;f.systems.hook_down=!deck_start;f.hornet.hook_angle=if deck_start{0.}else{0.65};
 if deck_start {f.carrier.attached=true;f.carrier.deck_position=DVec3::new(10.,DECK,15.);f.throttle=0.;f.spool=0.;f.thrust=0.;place_locked(&mut f);f.status="CATAPULT READY / SHIFT FULL POWER THEN ENTER".into();}
 else {let z=2300.;f.position=world(DVec3::new(lane(z),DECK+(z-75.)*0.061+3.,z),0.);f.orientation=rotation(0.)*DQuat::from_rotation_y(0.14)*DQuat::from_rotation_x(0.07);f.velocity=velocity(0.)+rotation(0.)*DVec3::new(-9.8,-4.27,-70.);f.throttle=0.55;f.spool=0.55;f.landed=false;f.status="CARRIER APPROACH / H HOOK / ALT+G GUIDANCE".into();}
 let density=atmosphere(f.altitude()).2;
 let speed=(f.mass()*9.80665*2./(density*46.5*1.05)).sqrt();
 if let Some(trim)=crate::aircraft::hornet::trim::solve(&f,f.altitude(),speed,0.,0.) {
  crate::aircraft::hornet::trim::apply(&mut f,trim);
  if deck_start {f.throttle=0.;f.spool=0.;for e in &mut f.hornet.engines {e.spool=0.;}}
  if !deck_start {let relative_speed=(speed-SPEED).max(40.);let yaw=(relative_speed*0.14).atan2(relative_speed+SPEED);let gamma=(-relative_speed*0.061/speed).asin();f.orientation=rotation(0.)*DQuat::from_rotation_y(yaw)*DQuat::from_rotation_x(trim.alpha+gamma);f.velocity=velocity(0.)+rotation(0.)*DVec3::new(-relative_speed*0.14,-relative_speed*0.061,-relative_speed);}
 }
 f
}
fn place_locked(f:&mut Flight) {let clearance=crate::collision::wheel_clearance(f);let mut p=f.carrier.deck_position;p.y=DECK+clearance-0.12;f.position=world(p,f.elapsed);f.orientation=rotation(f.elapsed);f.velocity=velocity(f.elapsed);f.rates=DVec3::ZERO;f.landed=true;f.airspeed=SPEED;f.alpha=0.;f.beta=0.;}
pub fn launch(f:&mut Flight) {if f.carrier.active&&f.carrier.attached&&f.throttle>0.85&&f.fuel+crate::aircraft::hornet::stores::fuel(f)>0.&&f.engine&&f.gear_extension>0.98 {f.carrier.launch=Some(0.);f.carrier.attached=false;f.systems.hook_down=false;f.status="CATAPULT FIRING".into();}else if f.carrier.active {f.status="LAUNCH: ATTACH ON DECK / ENGINE RUNNING / FULL THROTTLE / GEAR DOWN".into();}}
pub fn attach(f:&mut Flight) {if f.carrier.active&&f.landed&&(f.velocity-velocity(f.elapsed)).length()<3. {f.carrier.arrested=false;f.carrier.attached=true;f.carrier.launch=None;f.carrier.deck_position=DVec3::new(10.,DECK,15.);place_locked(f);f.status="MOVED TO CATAPULT / SHIFT FULL POWER THEN ENTER".into();}}
/// Hook-tip geometry derived from the imported pivot and shank. Shared by
/// attachment loads, crossing detection and wire rendering; animated angle is
/// integrated by the physics clock rather than render time.
pub fn hook_point(f:&Flight)->DVec3 {
 let a=f.hornet.hook_angle;
 DVec3::new(0.,0.15-3.5*a.sin(),5.16+3.5*a.cos())
}
pub const LAUNCH_POINT:DVec3=DVec3::new(0.,-1.4,-3.61);
pub fn after_step(f:&mut Flight,previous:DVec3,dt:f64) {
 if !f.carrier.active||f.crashed{return;}
 let p=relative(f.position,f.elapsed);
 // Island collision, estimated bound; open deck collision uses the sampled outline.
 if (14. ..34.).contains(&p.x)&&(-25. ..55.).contains(&p.z)&&p.y<DECK+34.&&p.y>DECK {crate::collision::destroy(f,"CARRIER ISLAND IMPACT",rotation(f.elapsed)*DVec3::Y);return;}
 let hook=relative(f.position+f.orientation*hook_point(f),f.elapsed);
 let prev_hook=relative(previous+f.orientation*hook_point(f),(f.elapsed-dt).max(0.));
 let speed=(f.velocity-velocity(f.elapsed)).length();
 if !f.carrier.arrested&&!f.carrier.attached&&f.carrier.launch.is_none()&&f.hornet.hook_angle>0.6&&f.gear_extension>0.98&&speed>15. {
  for (i,z) in [112.,100.,88.,76.].iter().enumerate() {
   if prev_hook.z>*z&&hook.z<=*z {
    let fraction=((prev_hook.z-*z)/(prev_hook.z-hook.z)).clamp(0.,1.);let cross=prev_hook.lerp(hook,fraction);
    if (cross.x-lane(*z)).abs()<15.&&(DECK-1.2..DECK+0.35).contains(&cross.y) {
     f.carrier.arrested=true;f.hornet.cable_energy=0.;f.carrier.wire=i;f.carrier.arrest_speed=speed;f.carrier.deck_position=DVec3::new(lane(*z),DECK,*z);f.throttle=0.;f.autopilot=false;f.status=format!("WIRE {} CAUGHT",i+1);break;
    }
   }
  }
 }
}

#[derive(Component)]pub(crate) struct Ship;
#[derive(Component)]pub(crate) struct CarrierText;
pub fn setup(mut c:Commands,server:Res<AssetServer>) {
 c.spawn((Ship,SceneRoot(server.load(GltfAssetLabel::Scene(0).from_asset("ships/nimitz/source.glb"))),Transform::default(),Visibility::Hidden));
 c.spawn((CarrierText,Text::new(""),TextFont{font_size:14.,..default()},TextColor(Color::srgb(1.,0.8,0.35)),Node{position_type:PositionType::Absolute,right:px(28),top:px(285),width:px(410),..default()}));
}
pub fn visual(f:Res<Flight>,mut ship:Query<(&mut Transform,&mut Visibility),With<Ship>>,mut text:Query<&mut Text,With<CarrierText>>,mut lines:Gizmos) {
 for (mut t,mut v) in &mut ship {*v=if f.carrier.active{Visibility::Inherited}else{Visibility::Hidden};if !f.carrier.active{continue;}
  let q=crate::world::local_rot(rotation(f.elapsed),&f);
  t.translation=crate::world::local(origin(f.elapsed),&f)+q*Vec3::new(222.12742*SCALE,0.,51.34455*SCALE);
  t.rotation=q*Quat::from_rotation_y(std::f32::consts::PI);t.scale=Vec3::splat(SCALE);
 }
 for mut t in &mut text {
  if !f.carrier.active {t.0.clear();continue;}
  let p=relative(f.position,f.elapsed);let speed=(f.velocity-velocity(f.elapsed)).length();
  t.0=format!("CARRIER / 25 KT / H HOOK / ENTER LAUNCH / ALT+C CATAPULT / ALT+G GUIDANCE\n{}",f.status);
  if f.carrier.guidance&&!f.landed {let glide=DECK+(p.z-75.)*0.061+3.;t.0+=&format!("\nLINEUP {:+.1} m / GLIDE {:+.1} m / DECK SPEED {:.0} kt",p.x-lane(p.z),p.y-glide,speed*1.94384);}
 }
 if f.carrier.active {
  for z in [112.,100.,88.,76.] {let x=lane(z);let a=world(DVec3::new(x-15.,DECK+0.12,z),f.elapsed);let b=world(DVec3::new(x+15.,DECK+0.12,z),f.elapsed);let color=Color::srgb(0.15,0.12,0.08);
   if f.carrier.arrested && (z-[112.,100.,88.,76.][f.carrier.wire]).abs()<0.1 {
    let hook=f.position+f.orientation*hook_point(&f);
    lines.line(crate::world::local(a,&f),crate::world::local(hook,&f),color);lines.line(crate::world::local(hook,&f),crate::world::local(b,&f),color);
   }else{lines.line(crate::world::local(a,&f),crate::world::local(b,&f),color);}}
  let track_a=world(DVec3::new(10.,DECK+0.15,20.),f.elapsed);let track_b=world(DVec3::new(10.,DECK+0.15,-105.),f.elapsed);
  lines.line(crate::world::local(track_a,&f),crate::world::local(track_b,&f),Color::srgb(0.7,0.7,0.65));
  let shuttle_z=if f.carrier.launch.is_some(){f.carrier.deck_position.z-5.5}else{9.5};
  lines.line(crate::world::local(world(DVec3::new(9.4,DECK+0.2,shuttle_z),f.elapsed),&f),crate::world::local(world(DVec3::new(10.6,DECK+0.2,shuttle_z),f.elapsed),&f),Color::srgb(1.,0.75,0.05));
  if f.carrier.guidance {for z in (200..=2200).step_by(200) {let p=DVec3::new(lane(z as f64),DECK+(z as f64-75.)*0.061+3.,z as f64);let a=world(p-DVec3::X*6.,f.elapsed);let b=world(p+DVec3::X*6.,f.elapsed);lines.line(crate::world::local(a,&f),crate::world::local(b,&f),Color::srgb(0.2,1.,0.5));}}
 }
}

#[cfg(test)]mod tests {
 use super::*;
 #[test]fn outline_and_moving_frame_roundtrip(){assert!(inside(DVec3::ZERO));assert!(!inside(DVec3::new(60.,0.,0.)));for t in [0.,100.,3600.] {let p=DVec3::new(10.,DECK,100.);assert!((relative(world(p,t),t)-p).length()<1e-7);assert!(sea(origin(t).x,origin(t).z));}}
 #[test]fn parked_ship_carries_aircraft_and_catapult_releases(){let mut f=scenario(true);let e=Environment::default();let p=f.position;for _ in 0..600 {simulate(&mut f,&e,1./120.);}assert!((f.position-p).length()>60.);assert!(!f.crashed&&f.landed);f.throttle=1.;launch(&mut f);for _ in 0..720 {simulate(&mut f,&e,1./120.);}assert!(f.carrier.launch.is_none(),"status={} pos={:?} rates={:?} speed={} launch={:?}",f.status,relative(f.position,f.elapsed),f.rates,(f.velocity-velocity(f.elapsed)).length(),f.carrier.launch);assert!(!f.carrier.attached&&!f.landed&&!f.crashed);assert!((f.velocity-velocity(f.elapsed)).length()>75.);}
 #[test]fn hook_crossing_uses_tip_and_requires_extension(){for extended in [false,true] {let mut f=scenario(false);f.hornet.hook_angle=if extended{0.65}else{0.};f.orientation=rotation(0.);let tip=hook_point(&f);f.position=world(DVec3::new(lane(112.),DECK-tip.y,111.-tip.z),0.);f.velocity=velocity(0.)+rotation(0.)*DVec3::NEG_Z*70.;let before=f.position+rotation(0.)*DVec3::Z*2.;after_step(&mut f,before,1./120.);assert_eq!(f.carrier.arrested,extended);}}
 #[test]fn physics_touchdown_catches_wire_and_bolter_remains_flyable(){
  for hook in [false,true] {
   let mut f=scenario(false);let e=Environment{wind_enabled:false,..default()};
   f.systems.hook_down=hook;f.throttle=0.45;f.orientation=rotation(0.)*DQuat::from_rotation_y(0.14)*DQuat::from_rotation_x(0.03);
   let clearance=crate::collision::wheel_clearance(&f);
   f.position=world(DVec3::new(lane(125.),DECK+clearance+0.6,125.),0.);
   f.velocity=velocity(0.)+rotation(0.)*DVec3::new(-9.8,-3.,-70.);
   for _ in 0..120 {simulate(&mut f,&e,1./120.);if f.carrier.arrested {break;}}
   assert!(!f.crashed,"{}",f.status);assert_eq!(f.carrier.arrested,hook,"{}",f.status);
  }
 }
 #[test]fn carrier_reset_forces_hornet_and_preserves_start_choice(){
  for id in [11,12] {let mut f=Flight::runway();let mut view=crate::ViewState::default();crate::reset(&mut f,id,&mut view);assert!(f.carrier.active);assert_eq!(f.aircraft,Aircraft::Hornet);assert_eq!(f.carrier.attached,id==12);assert_eq!(view.scenario,id);assert!(!view.help);}
 }
 #[test]fn missed_deck_has_no_invisible_support(){let f=scenario(false);assert!(deck(&f,world(DVec3::new(60.,DECK+2.,0.),f.elapsed)).is_none());assert!(deck(&f,world(DVec3::new(0.,DECK-8.,0.),f.elapsed)).is_none());}
}

/// Body-coordinate attachment forces for the dedicated Hornet solver.
/// Neither velocity nor attitude is prescribed; ground reactions stay active.
pub fn physics_loads(f:&mut Flight,dt:f64)->crate::aircraft::hornet::Loads {
 use crate::aircraft::hornet::{Loads,mass};
 let mut loads=Loads::default();if !f.carrier.active{return loads;}
 let (cg,_)=mass::properties(f);let basis=rotation(f.elapsed);let p=relative(f.position,f.elapsed);
 let v=basis.conjugate()*(f.velocity-velocity(f.elapsed));
 let to_body=|force:DVec3|f.orientation.conjugate()*(basis*force);
 if f.carrier.attached {
  let target=f.carrier.deck_position;
  let spring=DVec3::new((target.x-p.x)*180000.-v.x*45000.,0.,(target.z-p.z)*180000.-v.z*45000.);
  let force=spring.clamp(DVec3::splat(-500000.),DVec3::splat(500000.));
  loads.add(to_body(force),LAUNCH_POINT-cg);
 }
 if let Some(age)=f.carrier.launch {
  f.carrier.launch=Some(age+dt);let stroke=15.-p.z;
  let forward=-v.z;let target=78.;let required=(f.mass()*target*target/(2.*105.)-f.thrust).clamp(100000.,800000.);
  let schedule=((stroke+1.)/5.).clamp(0.1,1.)*((110.-stroke)/10.).clamp(0.,1.);
  let force=DVec3::new((-100000.*(p.x-10.)-20000.*v.x).clamp(-250000.,250000.),-required*schedule*0.38,-required*schedule);
  loads.add(to_body(force),LAUNCH_POINT-cg);
  f.carrier.deck_position.z=p.z;
  if stroke>105.||forward>82.||age>6.||!f.engine||f.throttle<0.5 {f.carrier.launch=None;f.status="CATAPULT RELEASE / S ROTATE / G GEAR UP".into();}
 }
 if f.carrier.arrested {
  let hook=relative(f.position+f.orientation*hook_point(f),f.elapsed);
  let center=f.carrier.deck_position;let left=DVec3::new(center.x-15.,DECK,center.z)-hook;let right=DVec3::new(center.x+15.,DECK,center.z)-hook;
  let payout=(left.length()+right.length()-30.).max(0.)*0.5;let arm=hook_point(f)-cg;
  let hook_velocity=v+basis.conjugate()*(f.orientation*f.rates.cross(arm));
  let direction=(left.normalize_or_zero()+right.normalize_or_zero()).normalize_or_zero();
  let separating=(-hook_velocity.dot(direction)).max(0.);
  f.carrier.arrest_distance=payout;f.carrier.arrest_speed=v.length();
  // Hydraulic absorber resists payout without elastic recoil. Static hold is
  // velocity-proportional; it never injects the cable's dissipated energy.
  let tension=((180000.+payout*2500.+separating*2000.).min(700000.))*(separating/2.).clamp(0.,1.);
  f.hornet.cable_energy+=tension*separating*dt;
  if payout>120.||f.hornet.cable_energy>90000000. {f.carrier.arrested=false;f.status="ARRESTING CABLE LIMIT EXCEEDED / BOLTER".into();}
  else {loads.add(to_body(direction*tension),arm);}
 }

 loads
}

#[cfg(test)]mod force_validation {
 use super::*;
 #[test]fn arrest_dissipates_energy_and_overstroke_releases(){
 for assist in [false,true] {
  let mut f=scenario(false);f.assist=assist;let e=Environment{wind_enabled:false,..default()};
  f.orientation=rotation(0.)*DQuat::from_rotation_y(0.14)*DQuat::from_rotation_x(0.03);
  let clear=crate::collision::wheel_clearance(&f);f.position=world(DVec3::new(lane(125.),DECK+clear+0.6,125.),0.);f.velocity=velocity(0.)+rotation(0.)*DVec3::new(-9.8,-3.,-70.);
  let mut caught=false;for _ in 0..1440 {simulate(&mut f,&e,1./240.);caught|=f.carrier.arrested;if f.crashed{break;}}
  println!("ARREST caught={} status={} speed={} distance={} energy={} health={:?}",caught,f.status,(f.velocity-velocity(f.elapsed)).length(),f.carrier.arrest_distance,f.hornet.cable_energy,f.gear_health);
  assert!(caught&&!f.crashed,"{} pitch={} rates={:?} compression={:?} contact={:?} attachment={:?}",f.status,f.pitch(),f.rates,f.gear_compression,f.hornet.diagnostic.contact,f.hornet.diagnostic.attachment);assert!((f.velocity-velocity(f.elapsed)).length()<3.);assert!(f.hornet.cable_energy>1000000.&&f.hornet.cable_energy<90000000.);
  f.carrier.arrested=true;f.carrier.deck_position.z+=160.;physics_loads(&mut f,1./240.);assert!(!f.carrier.arrested);
 }
 }
 #[test]fn launch_mass_variation_and_abort(){for fuel in [1000.,6000.] {let mut f=scenario(true);let e=Environment{wind_enabled:false,..default()};f.fuel=fuel;f.throttle=1.;for _ in 0..960 {simulate(&mut f,&e,1./240.);}launch(&mut f);let start=f.velocity;let mut release=None;for i in 0..1440 {simulate(&mut f,&e,1./240.);if f.carrier.launch.is_none(){release=Some(i);break;}}println!("CAT fuel={} release={:?} speed={} status={}",fuel,release,(f.velocity-velocity(f.elapsed)).length(),f.status);assert!(release.is_some()&&!f.crashed);assert!((f.velocity-start).length()>50.);}
  let mut f=scenario(true);f.throttle=1.;launch(&mut f);f.throttle=0.;physics_loads(&mut f,1./240.);assert!(f.carrier.launch.is_none());
 }
}

#[cfg(test)]mod departure_validation {
 use super::*;
 #[test]fn pilot_rotation_leads_to_free_flight_after_release(){for fuel in [1000.,6000.] {
  let mut f=scenario(true);f.fuel=fuel;f.throttle=1.;let e=Environment{wind_enabled:false,..default()};
  for _ in 0..960{simulate(&mut f,&e,1./240.);}launch(&mut f);
  let mut released=0;let mut minimum=f.altitude();for _ in 0..2400 {simulate(&mut f,&e,1./240.);if f.carrier.launch.is_none(){released+=1;f.controls.x=if released<300{0.5}else{0.};minimum=minimum.min(f.altitude());if released>=1200{break;}}}
  println!("DEPART fuel={} alt={} vs={} speed={} pitch={} pos={:?} status={}",fuel,f.altitude(),f.vertical_speed(),f.airspeed,f.pitch(),relative(f.position,f.elapsed),f.status);
  assert!(!f.crashed&&!f.landed);assert!(minimum>DECK-5.);assert!(f.altitude()>DECK+3.&&f.vertical_speed()>0.);assert!(relative(f.position,f.elapsed).z< -165.);
 }}
}
