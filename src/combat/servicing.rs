//! Instant player refuel/rearm at a friendly runway or usable carrier deck.
use super::*;
#[derive(Clone,Copy,PartialEq,Debug)]pub enum Station {Runway,Deck}
pub fn station(f:&Flight)->Option<Station> {
    if f.vehicle!=Vehicle::Gripen||!f.landed||f.crashed||f.ejection.active||f.throttle>0.05
        ||f.carrier.attached||f.carrier.arrested||f.carrier.launch.is_some()
        ||(f.velocity-crate::carrier::ground_velocity(f,f.position)).length()>1. {return None;}
    if crate::carrier::deck(f,f.position).is_some() {
        f.carrier.operational.then_some(Station::Deck)
    }else{crate::terrain::paved(f.position.x,f.position.z).then_some(Station::Runway)}
}
pub fn refill(c:&mut Combat,f:&mut Flight,tanks:bool) {
    f.fuel=f.aircraft.spec().fuel;c.ammo=f.aircraft.spec().ammo;c.lock=0.;
    c.pending_main=None;c.pending_side=None;f.systems.stores.fill(true);
    if c.fight.is_none(){c.install_naval_weapons(f);}
    if let Some(fight)=&mut c.fight {fight.gun=f.aircraft.spec().gun;fight.flares=24;fight.chaff=24;fight.gun_held=false;}
    if tanks&&f.aircraft==crate::aircraft::Aircraft::Hornet {crate::aircraft::hornet::stores::fit(f);}
}
pub fn radio_input(keys:&mut ButtonInput<KeyCode>,c:&mut Combat,f:&mut Flight) {
    if f.vehicle!=Vehicle::Gripen||f.aircraft!=crate::aircraft::Aircraft::Hornet {c.service_radio_open=false;return;}
    if keys.just_pressed(KeyCode::Backslash) {c.service_radio_open=!c.service_radio_open;keys.clear_just_pressed(KeyCode::Backslash);}
    if !c.service_radio_open {return;}
    if keys.just_pressed(KeyCode::Escape) {c.service_radio_open=false;keys.clear_just_pressed(KeyCode::Escape);}
    if keys.just_pressed(KeyCode::Digit1) {
        if station(f).is_some() {
            let tanks=f.hornet.tanks.loadout;refill(c,f,tanks);c.notify("SERVICE COMPLETE / FUEL, MISSILES, CANNON AND COUNTERMEASURES RESTORED");
        }else{c.notify("SERVICE: STOP ON FRIENDLY RUNWAY OR USABLE DECK / IDLE / RELEASE ATTACHMENTS");}
        f.status=c.message.clone();c.service_radio_open=false;keys.clear_just_pressed(KeyCode::Digit1);
    }
}
#[derive(Component)]pub struct ServiceRadio;
pub fn setup(mut c:Commands) {
    c.spawn((ServiceRadio,Text::new(""),TextFont{font_size:15.,..default()},TextColor(Color::srgb(0.65,0.95,0.88)),
        Node{position_type:PositionType::Absolute,left:px(28),bottom:px(80),max_width:px(600),padding:UiRect::all(px(12)),display:Display::None,..default()},
        BackgroundColor(Color::srgba(0.015,0.04,0.055,0.9)),ZIndex(35)));
}
pub fn ui(c:Res<Combat>,v:Res<ViewState>,menu:Res<crate::start_screen::Menu>,mut panels:Query<(&mut Node,&mut Text),With<ServiceRadio>>) {
    let message=if c.service_radio_open {"RADIO / 1 REFUEL & REARM / ESC CLOSE"}
        else if c.message.starts_with("SERVICE")&&c.clock<c.message_until {&c.message}else{""};
    for (mut node,mut text) in &mut panels {
        node.display=if !menu.open&&!v.map&&c.naval.is_none()&&!message.is_empty(){Display::Flex}else{Display::None};
        if text.0!=message {text.0=message.into();}
    }
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn runway_radio_refills_expanded_cannon_without_resetting_battle_or_toggling_battery(){
        let mut f=Flight::runway();crate::aircraft::select(&mut f,crate::aircraft::Aircraft::Hornet);f.throttle=0.;f.velocity=DVec3::ZERO;
        let mut c=Combat::default();c.start_air(&f,1,5);c.fight.as_mut().unwrap().gun=0;c.ammo=[0;2];
        let before=(f.position,f.elapsed,c.targets[0].position,c.targets.len(),f.battery);
        let mut keys=ButtonInput::default();keys.press(KeyCode::Backslash);radio_input(&mut keys,&mut c,&mut f);
        assert!(c.service_radio_open);keys.reset_all();keys.press(KeyCode::Digit1);radio_input(&mut keys,&mut c,&mut f);
        assert!(!c.service_radio_open);assert!(!keys.just_pressed(KeyCode::Digit1));
        assert_eq!(c.fight.as_ref().unwrap().gun,2060);assert_eq!(c.ammo,f.aircraft.spec().ammo);
        assert_eq!((f.position,f.elapsed,c.targets[0].position,c.targets.len(),f.battery),before);
        f.landed=false;assert_eq!(station(&f),None);f.landed=true;f.throttle=0.5;assert_eq!(station(&f),None);
        f.throttle=0.;f.position=crate::terrain::point(6500.,3200.,62.);assert_eq!(station(&f),None);
    }
}
