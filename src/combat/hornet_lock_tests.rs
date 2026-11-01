use super::*;
#[test]
fn hornet_acquires_and_retains_both_weapons_in_each_air_combat_scenario() {
 for scenario in [4,5,6,7,8] {for kind in [Weapon::Heat,Weapon::Radar] {
  let mut f=if scenario==8 {bvr::flight()}else{flight()};
  crate::aircraft::select(&mut f,crate::aircraft::Aircraft::Hornet);
  f.paused=false;
  let mut c=Combat::default();
  if scenario==8 {
   bvr::start(&mut c,&f,1,&bvr::Menu::default());
   let q=f.orientation;let p=f.position;
   for (i,t) in c.targets.iter_mut().enumerate() {
    t.position=p+q*DVec3::new(i as f64*500.,0.,-5000.-i as f64*1000.);
    t.dynamics.position=t.position;
   }
  }else if scenario==4 {c.start(&f,1);}else{c.start_air(&f,1,scenario);}
  c.weapon=kind;c.sun=-f.up();let mut peak=0f64;let mut held=0.;let mut reasons=std::collections::BTreeMap::new();
  for _ in 0..480 {
   let delta=c.targets[c.selected].position-f.position;
   f.orientation=facing(delta.normalize(),f.up());
   c.tick(&mut f,1./120.);
   peak=peak.max(c.lock);
   if c.lock>=kind.lock_time(){held+=1./120.;}
   if let Err(reason)=c.cue(&f){*reasons.entry(reason).or_insert(0usize)+=1;}
  }
  println!("scenario={scenario} weapon={kind:?} peak={peak} held={held} reasons={reasons:?} battery={} fold={}",f.battery,f.systems.hornet_wing_fold);
  assert!(held>0.25,"scenario={scenario} weapon={kind:?} peak={peak} reasons={reasons:?}");
 }}
}
#[test]
fn hornet_hud_uses_real_acquisition_and_rejects_stale_lock() {
 let mut f=flight();crate::aircraft::select(&mut f,crate::aircraft::Aircraft::Hornet);f.paused=false;
 let mut c=Combat::default();c.start(&f,1);
 let hud=crate::cockpit::hornet_hud::acquisition(&f,&c);
 assert!(!hud.locked&&!hud.ready&&hud.reason.is_none());
 for _ in 0..180 {c.tick(&mut f,1./120.);}
 let hud=crate::cockpit::hornet_hud::acquisition(&f,&c);assert!(hud.locked&&hud.ready);
 assert!(c.fire(&f));assert_eq!(c.fired,1);
 c.weapon=Weapon::Radar;c.lock=Weapon::Radar.lock_time();c.radar.powered=false;
 let hud=crate::cockpit::hornet_hud::acquisition(&f,&c);
 assert_eq!(hud.reason,Some("RADAR STANDBY"));assert!(!hud.locked&&!hud.ready);assert_eq!(hud.fraction,0.);
 c.radar.powered=true;f.systems.hornet_wing_fold=0.5;
 assert_eq!(crate::cockpit::hornet_hud::acquisition(&f,&c).reason,Some("UNFOLD WINGS"));
 f.systems.hornet_wing_fold=0.;c.radar.mode=bvr::RadarMode::Stt;c.clock+=0.3;bvr::sense_local(&mut c,&f);f.paused=true;
 let hud=crate::cockpit::hornet_hud::acquisition(&f,&c);assert!(hud.locked&&!hud.ready);
 c.active=false;assert_eq!(crate::cockpit::hornet_hud::acquisition(&f,&c).reason,Some("SELECT COMBAT SCENARIO"));
}
#[test]
fn hornet_combat_resets_select_air_hud_and_preserve_navigation_elsewhere() {
 for scenario in 0..=12 {
  let mut f=flight();crate::aircraft::select(&mut f,crate::aircraft::Aircraft::Hornet);
  let mut v=crate::ViewState::default();crate::reset(&mut f,scenario,&mut v);
  assert_eq!(f.hornet.hud.mode,if (4..=8).contains(&scenario){crate::cockpit::hornet_hud::MasterMode::Air}else{crate::cockpit::hornet_hud::MasterMode::Nav});
 }
}

// Fixed radar geometry isolates acquisition from AI motion and aircraft control.
fn long_range_scene(range: f64) -> (Flight, Combat) {
 let mut f=bvr::flight();crate::aircraft::select(&mut f,crate::aircraft::Aircraft::Hornet);f.paused=false;
 let mut c=Combat::default();c.start(&f,1);c.weapon=Weapon::Radar;c.radar.mode=bvr::RadarMode::Stt;
 let t=&mut c.targets[0];t.position=f.position+f.forward()*range;t.velocity=-f.forward()*250.;t.alive=true;
 (f,c)
}
#[test]
fn hornet_radar_and_launch_zone_work_beyond_twenty_miles() {
 let (f,mut c)=long_range_scene(60000.);
 assert!(c.cue(&f).is_err());
 bvr::sense_local(&mut c,&f);
 assert!(c.radar.tracks[0].valid(c.clock));assert!(c.cue(&f).is_ok());
 let zone=bvr::cockpit_radar::launch_zone(&c,&f).unwrap();
 assert!((zone.range-60000.).abs()<1e-6);assert!(zone.max>zone.range);assert!(zone.nez<zone.max);
 c.lock=Weapon::Radar.lock_time();assert!(c.fire(&f));assert!(c.missiles[0].gnc.long_range);
 let contact=guidance::target(&c.targets[0]);
 guidance::fly(&mut c.missiles[0],contact,guidance::Support{position:f.position,forward:f.forward(),available:true},c.sun,0.01);
 assert_eq!(c.missiles[0].gnc.reason,"DATALINK UPDATE");
 c.clock+=3.1;assert_eq!(c.cue(&f),Err("NO FRESH RADAR TRACK"));
 assert!(bvr::cockpit_radar::launch_zone(&c,&f).unwrap().coasting);
 c.clock+=18.;assert!(bvr::cockpit_radar::launch_zone(&c,&f).is_none());
}
#[test]
fn hornet_radar_range_power_and_envelope_are_independent() {
 let (mut f,mut c)=long_range_scene(60000.);
 c.radar.range=0;bvr::sense_local(&mut c,&f);assert!(!c.radar.tracks[0].valid(c.clock));
 c.clock+=0.3;c.radar.range=2;bvr::sense_local(&mut c,&f);assert!(c.cue(&f).is_ok());
 let first=bvr::cockpit_radar::launch_zone(&c,&f).unwrap();
 c.targets[0].velocity=f.forward()*350.;c.clock+=0.3;bvr::sense_local(&mut c,&f);
 let retreat=bvr::cockpit_radar::launch_zone(&c,&f).unwrap();assert!(retreat.max<first.max);assert!(retreat.nez<first.nez);
 c.radar.powered=false;assert_eq!(c.cue(&f),Err("RADAR STANDBY"));assert!(bvr::cockpit_radar::launch_zone(&c,&f).is_none());
 c.radar.powered=true;f.damage.radar=0.;assert!(c.cue(&f).is_err());
 let mut other=flight();other.aircraft=crate::aircraft::Aircraft::Gripen;
 assert_eq!(bvr::local_ranges(&other),&bvr::LOCAL_RANGES);
}

#[test]
fn hornet_tws_supports_four_distinct_missiles_after_selection_changes() {
 for bvr_mode in [false,true] {
  let mut f=bvr::flight();crate::aircraft::select(&mut f,crate::aircraft::Aircraft::Hornet);f.paused=false;
  let mut c=Combat::default();
  if bvr_mode {let mut menu=bvr::Menu::default();menu.formation=3;bvr::start(&mut c,&f,1,&menu);}else{c.start_air(&f,1,5);}
  while c.targets.len()<4 {c.targets.push(c.targets[0].clone());}
  for (i,t) in c.targets.iter_mut().enumerate() {t.position=f.position+f.orientation*DVec3::new((i as f64-1.5)*1500.,0.,-50000.);t.velocity=-f.forward()*240.;t.alive=true;}
  // Acquire all four through actual sensor scans before launching.
  for _ in 0..60 {if bvr_mode {let mut b=c.bvr.take().unwrap();bvr::sense(&mut b,&c,&f,0.2);c.bvr=Some(b);}else{bvr::sense_local(&mut c,&f);}c.clock+=0.2;}
  c.weapon=Weapon::Radar;
  for target in 0..4 {
   c.selected=target;
   for _ in 0..6 {if bvr_mode {let mut b=c.bvr.take().unwrap();bvr::sense(&mut b,&c,&f,0.2);c.bvr=Some(b);}else{bvr::sense_local(&mut c,&f);}c.clock+=0.2;}
   c.lock=Weapon::Radar.lock_time();assert!(c.fire(&f),"bvr={bvr_mode} target={target} {}",c.message);
  }
  // Freeze the targets' aircraft dynamics while the ordinary combat advance
  // evaluates guidance. The test checks real missile modes, not just flags.
  for m in c.missiles.iter_mut().take(4) {m.age=1.;}
  c.selected=3;
  for i in 0..4 {let r=if let Some(b)=&mut c.bvr {&mut b.radar}else{&mut c.radar};
   r.tracks[i].seen=c.clock;r.tracks[i].confidence=1.;}
  if bvr_mode {bvr::advance(&mut c,&mut f,1./120.);}else{c.advance(&f,1./120.);}
  for m in c.missiles.iter().take(4) {assert_eq!(m.gnc.mode,guidance::Mode::Supported,"bvr={bvr_mode} target={} reason={}",m.target,m.gnc.reason);assert!(!m.gnc.active);}
  let r=if let Some(b)=&mut c.bvr {&mut b.radar}else{&mut c.radar};
  r.tracks[1].seen=c.clock-4.;
  assert!(!r.supports(1,c.selected,c.clock));assert!(r.supports(0,c.selected,c.clock));
  r.mode=bvr::RadarMode::Stt;assert!(!r.supports(0,3,c.clock));assert!(r.supports(3,3,c.clock));
  r.mode=bvr::RadarMode::Search;assert!(!r.supports(3,3,c.clock));
  r.mode=bvr::RadarMode::Tws;r.powered=false;assert!(!r.supports(0,3,c.clock));
 }
}
