use super::*;
use crate::aircraft::{Aircraft, bays};
fn fixture(bvr_mode: bool) -> (Flight, Combat) {
    let mut f = flight();
    crate::aircraft::select(&mut f, Aircraft::F22);
    let mut c = Combat::default();
    if bvr_mode {
        bvr::start(&mut c, &f, 91, &bvr::Menu::default());
    } else {
        c.start(&f, 91);
    }
    c.weapon = Weapon::Heat;
    c.selected = 0;
    c.lock = Weapon::Heat.lock_time();
    c.targets[0].position = f.position + f.forward() * 1400.;
    c.targets[0].orientation = f.orientation;
    c.targets[0].velocity = f.velocity;
    c.targets[0].throttle = 1.;
    (f, c)
}
fn advance(f: &mut Flight, c: &mut Combat, dt: f64) {
    crate::aircraft::main_bay::advance(&mut f.systems,f.damage.hydraulics>0.12,dt);
    for i in 0..2 {
        f.systems.side_bays[i].advance(
            f.systems.manual_bays || f.systems.side_request == Some(i),
            f.damage.hydraulics > 0.12,
            dt,
        );
    }
    c.clock += dt;
    c.service_main_launch(f);
    c.service_side_launch(f);
}

#[test]
fn rotating_airframe_release_inherits_pose_rate_and_station_velocity() {
    for bvr_mode in [false,true] {for kind in [Weapon::Heat,Weapon::Radar] {
        let (mut f,mut c)=fixture(bvr_mode);
        f.rates=DVec3::new(0.3,-0.2,0.4);
        c.weapon=kind;c.lock=kind.lock_time();
        if let Some(b)=c.bvr.as_mut() {
            b.radar.mode=bvr::RadarMode::Tws;
            b.radar.tracks[0].confidence=1.;b.radar.tracks[0].seen=c.clock;
            b.radar.tracks[0].position=c.targets[0].position;
        }
        assert!(c.fire(&f),"{}",c.message);
        for _ in 0..100 {advance(&mut f,&mut c,0.01);if !c.missiles.is_empty(){break;}}
        let m=&c.missiles[0];let rate=f.orientation*f.rates;
        assert!(m.gnc.body_rate.distance(rate)<1e-12);
        let rail=if kind==Weapon::Heat {bays::missile_pose(0,1.).rotation.as_dquat()}else{DQuat::IDENTITY};
        assert!(m.gnc.body.distance(f.orientation*rail*-DVec3::Z)<1e-7);
        let eject=if kind==Weapon::Radar{-(f.orientation*DVec3::Y)*8.}else{f.orientation*rail*-DVec3::Z*18.};
        assert!(m.velocity.distance(f.velocity+rate.cross(m.position-f.position)+eject)<1e-7,"bvr={bvr_mode} kind={kind:?} delta={:?}",m.velocity-(f.velocity+rate.cross(m.position-f.position)+eject));
    }}
}

#[test]
fn main_bay_queues_one_round_ejects_from_visible_pose_and_closes_after_release() {
    for bvr_mode in [false,true] {
        let (mut f,mut c)=fixture(bvr_mode);
        c.weapon=Weapon::Radar;c.lock=Weapon::Radar.lock_time();
        if let Some(b)=c.bvr.as_mut() {
            b.radar.mode=bvr::RadarMode::Tws;
            b.radar.tracks[0].confidence=1.;b.radar.tracks[0].seen=c.clock;
            b.radar.tracks[0].position=c.targets[0].position;
        }
        assert!(c.fire(&f),"{}",c.message);assert!(c.fire(&f));
        assert_eq!(c.ammo,[2,6]);assert!(c.missiles.is_empty());
        let clock=c.clock;f.paused=true;c.tick(&mut f,1.);
        assert_eq!(c.clock,clock);assert_eq!(f.systems.bays,0.);f.paused=false;
        for _ in 0..90 {advance(&mut f,&mut c,0.01);}
        assert_eq!(c.ammo,[2,5],"{}",c.message);assert_eq!(c.fired,1);
        assert!(!f.systems.stores[2]);assert_eq!(f.systems.main_request,None);
        let m=&c.missiles[0];
        assert!(m.position.distance(f.position+f.orientation*crate::aircraft::main_bay::store_position(0,1.))<1e-7);
        assert!((m.velocity-(f.velocity-f.orientation*DVec3::Y*8.)).length()<1e-7);
        for _ in 0..100{advance(&mut f,&mut c,0.01);}
        assert_eq!(f.systems.main_stroke,[0.;6]);assert!(f.systems.bays<1e-8);
    }
}

#[test]
fn main_bay_cancels_blocked_invalid_or_changed_launch_without_spending_ammo() {
    for reason in 0..7 {
        let(mut f,mut c)=fixture(false);c.weapon=Weapon::Radar;c.lock=Weapon::Radar.lock_time();
        assert!(c.fire(&f));advance(&mut f,&mut c,0.1);
        match reason {
            0=>f.damage.hydraulics=0.,1=>f.crashed=true,2=>c.weapon=Weapon::Heat,
            3=>c.selected=1,4=>c.generation+=1,5=>f.systems.stores[2]=false,_=>f.gear_extension=1.,
        }
        advance(&mut f,&mut c,0.01);
        assert!(c.pending_main.is_none());assert_eq!(f.systems.main_request,None);
        assert!(c.missiles.is_empty());assert_eq!(c.ammo,[2,6]);
    }
    let(mut f,mut c)=fixture(false);c.weapon=Weapon::Radar;c.lock=Weapon::Radar.lock_time();
    assert!(c.fire(&f));advance(&mut f,&mut c,0.1);
    c.start(&f,92);c.service_main_launch(&mut f);
    assert!(c.pending_main.is_none());assert_eq!(f.systems.main_request,None);
}
#[test]
fn release_occurs_at_the_completed_step_without_advancing_the_new_round() {
    for bvr_mode in [false,true] {
        let (mut f,mut c)=fixture(bvr_mode);assert!(c.fire(&f));
        f.systems.side_bays[0]=bays::State{door:1.,launcher:1.};
        c.tick(&mut f,1./120.);
        assert_eq!(c.missiles.len(),1);assert_eq!(c.missiles[0].age,0.);
        assert!(c.missiles[0].position.distance(f.position+f.orientation*bays::missile_pose(0,1.).translation.as_dvec3())<1e-7);
    }
}
#[test]
fn both_weapon_paths_queue_one_round_and_release_from_the_visible_rail() {
    for bvr_mode in [false, true] {
        let (mut f, mut c) = fixture(bvr_mode);
        assert!(c.fire(&f), "{}", c.message);
        assert!(c.fire(&f));
        assert_eq!(c.ammo, [2, 6]);
        assert!(c.missiles.is_empty());
        let clock = c.clock;
        f.paused = true;
        c.tick(&mut f, 1.);
        assert_eq!(c.clock, clock);
        assert_eq!(f.systems.side_bays, [bays::State::default(); 2]);
        f.paused = false;
        for _ in 0..100 {
            advance(&mut f, &mut c, 0.01);
        }
        assert_eq!(c.ammo, [1, 6]);
        assert_eq!(c.missiles.len(), 1);
        assert_eq!(c.fired, 1);
        let pose = bays::missile_pose(0, 1.);
        let m = &c.missiles[0];
        assert!(
            m.position
                .distance(f.position + f.orientation * pose.translation.as_dvec3())
                < 1e-7
        );
        assert!(
            (m.gnc.body - f.orientation * pose.rotation.as_dquat() * -DVec3::Z).length() < 1e-7
        );
        assert!(!f.systems.stores[0]);
        assert_eq!(f.systems.side_bays[1], bays::State::default());
        // A second explicit command operates the other bay after cooldown.
        c.clock += 1.;
        c.lock = Weapon::Heat.lock_time();
        assert!(c.fire(&f));
        for _ in 0..100 {
            advance(&mut f, &mut c, 0.01);
        }
        assert_eq!(c.ammo, [0, 6]);
        assert_eq!(c.fired, 2);
        assert_eq!(c.missiles.len(), 2);
    }
}
#[test]
fn queued_launch_cancels_without_spending_ammunition() {
    for bvr_mode in [false, true] {
        for reason in 0..8 {
            let (mut f, mut c) = fixture(bvr_mode);
            assert!(c.fire(&f));
            advance(&mut f, &mut c, 0.10);
            match reason {
                0 => f.damage.hydraulics = 0.,
                1 => f.crashed = true,
                2 => c.weapon = Weapon::Radar,
                3 => c.targets[0].alive = false,
                4 => c.selected = 1,
                5 => c.generation += 1,
                6 => c.targets[0].position = f.position - f.forward() * 3000.,
                _ => f.gear_extension = 1.,
            }
            advance(&mut f, &mut c, 0.01);
            assert!(c.pending_side.is_none());
            assert_eq!(f.systems.side_request, None);
            assert_eq!(c.ammo, [2, 6]);
            assert!(c.missiles.is_empty());
        }
    }
    let (mut f, mut c) = fixture(false);
    assert!(c.fire(&f));
    advance(&mut f, &mut c, 0.10);
    c.start(&f, 92);
    c.service_side_launch(&mut f);
    assert!(c.pending_side.is_none());
    assert_eq!(f.systems.side_request, None);
}

#[test]
fn missing_or_changed_round_cannot_release_from_the_wrong_bay() {
    for bvr_mode in [false,true] {
        let(mut f,mut c)=fixture(bvr_mode);f.systems.stores[0]=false;
        assert!(!c.fire(&f));assert!(c.pending_side.is_none());assert_eq!(c.ammo,[2,6]);
        f.systems.stores[0]=true;assert!(c.fire(&f));
        c.ammo[0]=1;c.service_side_launch(&mut f);
        assert!(c.pending_side.is_none());assert!(c.missiles.is_empty());assert_eq!(c.ammo,[1,6]);
    }
}
