use super::*;
fn landing(bank: f64, offset: f64, assist: bool, hz: usize) -> (Flight, f64, f64) {
    let e = Environment {
        wind_enabled: false,
        ..Default::default()
    };
    let mut f = crate::carrier::scenario(false);
    f.assist = assist;
    f.orientation = crate::carrier::rotation(0.)
        * DQuat::from_rotation_y(0.14)
        * DQuat::from_rotation_x(0.07)
        * DQuat::from_rotation_z(bank.to_radians());
    let clear = crate::collision::wheel_clearance(&f);
    f.position = crate::carrier::world(
        DVec3::new(
            crate::carrier::lane(125.) + offset,
            crate::carrier::DECK + clear + 0.45,
            125.,
        ),
        0.,
    );
    f.velocity =
        crate::carrier::velocity(0.) + crate::carrier::rotation(0.) * DVec3::new(-9.8, -4., -70.);
    f.throttle = 0.45;
    let mut peak = 0f64;
    let mut slow = 0f64;
    for _ in 0..hz * 12 {
        simulate(&mut f, &e, 1. / hz as f64);
        let bank = f.roll().abs().to_degrees();
        peak = peak.max(bank);
        if (f.velocity - crate::carrier::velocity(f.elapsed)).length() < 3. {
            slow = slow.max(bank);
        }
        if f.crashed {
            break;
        }
    }
    (f, peak, slow)
}
#[test]
fn neutral_carrier_arrests_settle_without_roll_divergence() {
    for assist in [false, true] {
        for bank in [-3f64, 0., 3.] {
            for offset in [-6., 0., 6.] {
                let (f, peak, slow) = landing(bank, offset, assist, 240);
                println!(
                    "assist={assist} bank={bank} offset={offset} peak={peak:.2} slow={slow:.2} crash={} status={}",
                    f.crashed, f.status
                );
                assert!(
                    !f.crashed && f.carrier.arrested,
                    "{} bank={bank} offset={offset}",
                    f.status
                );
                assert!(
                    peak < 5. && slow < 2.,
                    "bank={bank} offset={offset} peak={peak} stopped={slow}"
                );
                assert!(f.roll().abs() < 1f64.to_radians() && f.rates.z.abs() < 0.02);
                assert!(
                    f.gear_compression.iter().all(|c| *c > 0.01),
                    "{:?}",
                    f.gear_compression
                );
            }
        }
    }
}
#[test]
fn carrier_stop_is_consistent_at_render_timestep_rates() {
    let (reference, _, _) = landing(1., 6., true, 240);
    for hz in [60, 120] {
        let (f, _, slow) = landing(1., 6., true, hz);
        assert!(!f.crashed && slow < 2.);
        assert!(f.position.distance(reference.position) < 0.01);
        assert!(f.velocity.distance(reference.velocity) < 0.01);
    }
}

#[test]
fn arrest_reaction_is_dissipative_at_the_airframe_pivot() {
    let mut f = crate::carrier::scenario(false);
    f.carrier.arrested = true;
    f.carrier.deck_position = DVec3::new(crate::carrier::lane(112.), crate::carrier::DECK, 112.);
    f.position = crate::carrier::world(DVec3::new(2., crate::carrier::DECK + 2., 70.), 0.);
    f.rates = DVec3::new(0.1, -0.2, 0.15);
    f.velocity = crate::carrier::velocity(0.) + f.orientation * DVec3::new(1., -0.2, -35.);
    let energy = f.hornet.cable_energy;
    let h = 1. / 240.;
    let loads = crate::carrier::physics_loads(&mut f, h);
    let body_velocity =
        f.orientation.conjugate() * (f.velocity - crate::carrier::velocity(f.elapsed));
    let power = loads.force.dot(body_velocity) + loads.moment.dot(f.rates);
    assert!(power < 0.);
    assert!(((f.hornet.cable_energy - energy) / h + power).abs() < 1e-5);
    let (cg, _) = mass::properties(&f);
    assert!((loads.moment - (crate::carrier::HOOK_PIVOT - cg).cross(loads.force)).length() < 1e-8);
}
