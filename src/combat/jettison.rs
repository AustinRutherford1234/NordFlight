//! Guarded pilot command for dumping the remaining carried weapons.
use super::*;

pub fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    menu: Res<crate::start_screen::Menu>,
    hangar: Res<crate::hangar::Hangar>,
    bvr: Res<bvr::Menu>,
    warp: Res<crate::warp::Warp>,
    view: Res<ViewState>,
    mut f: ResMut<Flight>,
    mut combat: ResMut<Combat>,
    mut gesture: Local<crate::ejection::Gesture>,
) {
    if menu.open || hangar.open || hangar.block_input || bvr.open || warp.editing
        || f.vehicle != Vehicle::Gripen || f.crashed || f.ejection.active
    {
        gesture.reset();
        return;
    }
    if keys.just_pressed(KeyCode::KeyW) {
        if keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight) {
            if gesture.press(time.elapsed_secs_f64(),view.generation) {
                let count=combat.dump_weapons(&mut f);
                f.status=if count==0 {"NO CARRIED WEAPONS TO DUMP".into()}
                    else {format!("WEAPONS DUMP / {count} MISSILES JETTISONED")};
                combat.notify(f.status.clone());
            }
        } else {gesture.reset();}
    }
}
impl Combat {
    fn dump_weapons(&mut self,f:&mut Flight)->usize {
        let count=self.ammo.iter().map(|n|*n as usize).sum();
        f.systems.stores.fill(false);
        self.ammo=[0;2];
        self.pending_side=None;
        self.pending_main=None;
        self.lock=0.;
        // Airborne weapons, gun ammunition and the separate tank/fuel state
        // remain independent. Existing mass/drag/visual paths consume stores.
        count
    }
}
#[cfg(test)] mod tests {
    use super::*;
    use std::time::Duration;
    fn app()->App {
        let mut app=App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(Duration::from_millis(100)))
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<crate::start_screen::Menu>()
            .init_resource::<crate::hangar::Hangar>()
            .init_resource::<bvr::Menu>()
            .init_resource::<crate::warp::Warp>()
            .init_resource::<ViewState>()
            .init_resource::<Combat>();
        let mut f=Flight::approach();crate::aircraft::select(&mut f,crate::aircraft::Aircraft::Hornet);
        crate::aircraft::hornet::stores::fit(&mut f);
        let mut combat=Combat::default();combat.start_air(&f,0,5);
        app.insert_resource(combat).insert_resource(f).add_systems(Update,controls);app
    }
    fn press(app:&mut App,option:Option<KeyCode>) {
        {let mut keys=app.world_mut().resource_mut::<ButtonInput<KeyCode>>();keys.reset_all();
        if let Some(key)=option {keys.press(key);}keys.press(KeyCode::KeyW);}
        app.update();app.world_mut().resource_mut::<ButtonInput<KeyCode>>().clear();
    }
    #[test] fn third_distinct_press_dumps_weapons_without_consuming_fuel_or_gun() {
        for option in [KeyCode::AltLeft,KeyCode::AltRight] {
            let mut app=app();
            let before=app.world().resource::<Flight>().clone();
            let mass=before.mass();
            let gun=app.world().resource::<Combat>().fight.as_ref().map(|v|v.gun);
            {let mut c=app.world_mut().resource_mut::<Combat>();
                c.pending_side=Some(PendingSideLaunch {side:0,target:0,generation:0,requested_at:0.});
                c.pending_main=Some(PendingMainLaunch {slot:0,target:0,generation:0,requested_at:0.});}
            for i in 0..3 {
                press(&mut app,Some(option));
                assert_eq!(app.world().resource::<Combat>().ammo==[0;2],i==2);
            }
            let f=app.world().resource::<Flight>();
            assert!(f.systems.stores.iter().all(|s|!*s));assert!(f.mass()<mass);
            assert_eq!(f.fuel,before.fuel);assert_eq!(f.hornet.tanks.fuel,before.hornet.tanks.fuel);
            assert_eq!((f.position,f.velocity,f.rates),(before.position,before.velocity,before.rates));
            let c=app.world().resource::<Combat>();
            assert_eq!(c.fight.as_ref().map(|v|v.gun),gun);
            assert!(c.pending_side.is_none() && c.pending_main.is_none());
        }
    }
    #[test] fn hold_timeout_reset_and_menu_cannot_complete_a_partial_dump() {
        let mut app=app();press(&mut app,Some(KeyCode::AltLeft));
        for _ in 0..5 {app.update();}
        assert_ne!(app.world().resource::<Combat>().ammo,[0;2]);
        for _ in 0..20 {app.update();}
        press(&mut app,Some(KeyCode::AltLeft));press(&mut app,Some(KeyCode::AltLeft));
        assert_ne!(app.world().resource::<Combat>().ammo,[0;2]);
        app.world_mut().resource_mut::<ViewState>().generation+=1;
        press(&mut app,Some(KeyCode::AltLeft));
        assert_ne!(app.world().resource::<Combat>().ammo,[0;2]);
        app.world_mut().resource_mut::<crate::start_screen::Menu>().open=true;
        press(&mut app,Some(KeyCode::AltLeft));
        app.world_mut().resource_mut::<crate::start_screen::Menu>().open=false;
        press(&mut app,Some(KeyCode::AltLeft));press(&mut app,None);
        press(&mut app,Some(KeyCode::AltLeft));press(&mut app,Some(KeyCode::AltLeft));
        assert_ne!(app.world().resource::<Combat>().ammo,[0;2]);
    }
}
