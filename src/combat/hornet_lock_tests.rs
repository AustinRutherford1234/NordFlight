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
 f.systems.hornet_wing_fold=0.;f.paused=true;
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
