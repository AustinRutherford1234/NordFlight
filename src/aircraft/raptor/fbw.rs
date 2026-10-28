//! F-22 control-envelope diagnostics and scheduled flight controls.
use super::*;

/// Schedule the rate-loop bandwidth from available angular acceleration. A
/// fixed 5/s loop assumes sea-level authority even when every effector saturates.
pub fn tracking_acceleration(rates: DVec3, target: DVec3, response: DMat3) -> DVec3 {
    let authority = response.x_axis.abs() + response.y_axis.abs() + response.z_axis.abs();
    let gain = DVec3::new(authority.x.sqrt(), authority.y.sqrt(), authority.z.sqrt()) * 2.2;
    (target - rates) * gain.clamp(DVec3::splat(0.6), DVec3::splat(5.))
}

/// Bounded allocation: re-solve after a channel reaches its stop instead of
/// independently clipping an unconstrained inverse and losing cancellation.
/// Pitch/yaw stabilization takes precedence over roll when travel is scarce.
pub fn allocate(response: DMat3, acceleration: DVec3) -> DVec3 {
    let row_authority = response.x_axis.abs() + response.y_axis.abs() + response.z_axis.abs();
    let weights = DVec3::new(2., 2., 1.) / row_authority.max(DVec3::splat(0.05));
    let columns = [
        response.x_axis * weights,
        response.y_axis * weights,
        response.z_axis * weights,
    ];
    let matrix = DMat3::from_cols(columns[0], columns[1], columns[2]);
    let demand = acceleration * weights;
    let mut command = DVec3::ZERO;
    for _ in 0..8 {
        for (i, column) in columns.iter().enumerate() {
            let error = demand - matrix * command;
            let delta =
                (column.dot(error) - 0.0004 * command[i]) / (column.length_squared() + 0.0004);
            command[i] = (command[i] + delta).clamp(-1., 1.);
        }
    }
    if command.is_finite() {
        command
    } else {
        DVec3::ZERO
    }
}

fn bvr_trial(
    command: DVec3,
    engines: [bool; 2],
    speed: f64,
    dt: f64,
) -> (Flight, serde_json::Value) {
    let mut f = crate::combat::bvr::flight();
    select(&mut f, Aircraft::F22);
    f.velocity = f.forward() * speed;
    f.infinite_fuel = true;
    f.systems.engine_on = engines;
    let environment = Environment {
        wind_enabled: false,
        ..default()
    };
    let mut samples = Vec::new();
    let mut peak_alpha = 0_f64;
    let mut peak_beta = 0_f64;
    let mut peak_rate = 0_f64;
    let mut min_g = 0_f64;
    let mut max_g = 0_f64;
    for i in 0..(20. / dt).round() as usize {
        let t = i as f64 * dt;
        f.controls = if t < 4. { command } else { DVec3::ZERO };
        crate::physics::simulate(&mut f, &environment, dt);
        peak_alpha = peak_alpha.max(f.alpha.abs());
        peak_beta = peak_beta.max(f.beta.abs());
        peak_rate = peak_rate.max(f.rates.length());
        min_g = min_g.min(f.g_load);
        max_g = max_g.max(f.g_load);
        if i % (0.25 / dt).round() as usize == 0 {
            samples.push(serde_json::json!({"t":t+dt,"alpha":f.alpha.to_degrees(),"beta":f.beta.to_degrees(),"rate":f.rates.to_array(),"g":f.g_load,"bank":f.roll().to_degrees(),"speed":f.airspeed,"height":f.altitude(),"request":f.systems.raptor.command.to_array()}));
        }
    }
    let row = serde_json::json!({"case":"bvr","altitude":11000.,"speed":speed,"command":command.to_array(),"engines":engines,"dt":dt,"peak_alpha":peak_alpha.to_degrees(),"peak_beta":peak_beta.to_degrees(),"peak_rate":peak_rate,"min_g":min_g,"max_g":max_g,"final_alpha":f.alpha.to_degrees(),"final_beta":f.beta.to_degrees(),"final_rate":f.rates.length(),"final_g":f.g_load,"final_height":f.altitude(),"crashed":f.crashed,"finite":f.rates.is_finite()&&f.position.is_finite(),"samples":samples});
    (f, row)
}

fn trial(
    altitude: f64,
    speed: f64,
    alpha: f64,
    maneuver: bool,
    engine: bool,
    dt: f64,
) -> serde_json::Value {
    let mut f = crate::combat::flight();
    select(&mut f, Aircraft::F22);
    f.position = surface_point(0., 0., altitude);
    f.velocity = -DVec3::Z * speed;
    f.orientation = bevy::math::DQuat::from_rotation_x(alpha);
    f.gear = false;
    f.gear_extension = 0.;
    f.flaps = 0.;
    f.throttle = 1.;
    f.spool = 1.;
    f.engine = engine;
    f.infinite_fuel = true;
    f.rates = if alpha.abs() > 0.3 {
        DVec3::new(0.2, 0.1, -0.3)
    } else {
        DVec3::ZERO
    };
    let environment = Environment {
        wind_enabled: false,
        ..default()
    };
    let mut samples = Vec::new();
    let mut peak_alpha = alpha.abs();
    let mut peak_beta = 0_f64;
    let mut peak_rate = 0_f64;
    let steps = (20. / dt).round() as usize;
    for i in 0..steps {
        let t = i as f64 * dt;
        f.controls = if maneuver && t < 3. {
            DVec3::new(0.5, 0., 0.3)
        } else {
            DVec3::ZERO
        };
        crate::physics::simulate(&mut f, &environment, dt);
        peak_alpha = peak_alpha.max(f.alpha.abs());
        peak_beta = peak_beta.max(f.beta.abs());
        peak_rate = peak_rate.max(f.rates.length());
        if i % (0.5 / dt).round() as usize == 0 {
            samples.push(serde_json::json!({"t":t+dt,"alpha":f.alpha.to_degrees(),"beta":f.beta.to_degrees(),"rate":f.rates.to_array(),"g":f.g_load,"height":f.altitude(),"speed":f.airspeed,"pressure":0.5*f.density*f.airspeed.powi(2),"request":f.systems.raptor.command.to_array(),"surface":f.surface.to_array(),"nozzle":f.systems.nozzle.map(f64::to_degrees)}));
        }
    }
    serde_json::json!({"altitude":altitude,"speed":speed,"initial_alpha":alpha.to_degrees(),"maneuver":maneuver,"engine":engine,"dt":dt,"peak_alpha":peak_alpha.to_degrees(),"peak_beta":peak_beta.to_degrees(),"peak_rate":peak_rate,"final_alpha":f.alpha.to_degrees(),"final_beta":f.beta.to_degrees(),"final_rate":f.rates.length(),"final_g":f.g_load,"final_height":f.altitude(),"crashed":f.crashed,"finite":f.position.is_finite()&&f.rates.is_finite(),"samples":samples})
}

pub fn study() {
    let mut rows = Vec::new();
    for altitude in [6000., 12000., 18000., 24000., 32000.] {
        for speed in [150., 300., 550.] {
            rows.push(trial(altitude, speed, 0.06, false, true, 1. / 120.));
            rows.push(trial(altitude, speed, 0.06, true, true, 1. / 120.));
        }
    }
    for altitude in [18000., 24000., 32000.] {
        for alpha in [-0.7, 0.7] {
            for engine in [false, true] {
                rows.push(trial(altitude, 220., alpha, false, engine, 1. / 120.));
            }
        }
    }
    for engines in [[true, true], [false, true], [false, false]] {
        for command in [
            DVec3::ZERO,
            DVec3::X,
            -DVec3::X,
            DVec3::Z,
            -DVec3::Z,
            DVec3::new(0.5, 0., 0.3),
        ] {
            for speed in [290., 362.5] {
                rows.push(bvr_trial(command, engines, speed, 1. / 120.).1);
            }
        }
    }
    println!("{}", serde_json::to_string_pretty(&rows).unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_thin_air_cruise_retains_normal_load_without_diving() {
        let row = trial(18000., 300., 0.06, false, true, 1. / 120.);
        assert!(row["finite"].as_bool().unwrap() && !row["crashed"].as_bool().unwrap());
        assert!(
            (row["final_g"].as_f64().unwrap() - 1.).abs() < 0.12,
            "{row}"
        );
        assert!((row["final_height"].as_f64().unwrap() - 18000.).abs() < 100.);
    }

    #[test]
    fn bvr_pitch_roll_and_engine_out_maneuvers_remain_controlled() {
        for speed in [290., 362.5] {
            for engines in [[true, true], [false, true], [false, false]] {
                for command in [
                    DVec3::ZERO,
                    DVec3::X,
                    -DVec3::X,
                    DVec3::Z,
                    -DVec3::Z,
                    DVec3::new(0.5, 0., 0.3),
                ] {
                    let (f, row) = bvr_trial(command, engines, speed, 1. / 120.);
                    assert!(!f.crashed && f.rates.is_finite(), "{row}");
                    assert!(
                        row["peak_alpha"].as_f64().unwrap() < 40.,
                        "incidence: {row}"
                    );
                    assert!(row["peak_beta"].as_f64().unwrap() < 10., "sideslip: {row}");
                    assert!(
                        f.alpha.abs() < 0.18 && f.beta.abs() < 0.10 && f.rates.length() < 0.18,
                        "release recovery: {row}"
                    );
                    if command == DVec3::ZERO {
                        assert!(
                            (f.altitude() - 11000.).abs() < 160. && (f.g_load - 1.).abs() < 0.15,
                            "cruise: {row}"
                        );
                    } else if command == DVec3::X {
                        assert!(
                            row["max_g"].as_f64().unwrap() > 4.,
                            "no pull authority: {row}"
                        );
                    } else if command.x < 0. {
                        assert!(
                            row["min_g"].as_f64().unwrap() < -2.,
                            "no push authority: {row}"
                        );
                    } else if command.x == 0. {
                        let sample = &row["samples"][4];
                        assert!(
                            sample["rate"][2].as_f64().unwrap() * command.z < -1.,
                            "roll direction/authority: {row}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn bvr_response_is_consistent_across_fixed_step_sizes() {
        let command = DVec3::new(0.5, 0., 0.3);
        let (reference, _) = bvr_trial(command, [true, true], 362.5, 1. / 240.);
        for dt in [1. / 60., 1. / 120.] {
            let (f, row) = bvr_trial(command, [true, true], 362.5, dt);
            assert!(!f.crashed && row["peak_beta"].as_f64().unwrap() < 5.);
            assert!((f.alpha - reference.alpha).abs() < 0.025);
            assert!((f.rates - reference.rates).length() < 0.03);
            assert!(
                f.position.distance(reference.position) < 80.,
                "dt {dt}, distance {}",
                f.position.distance(reference.position)
            );
        }
    }

    #[test]
    fn saturated_allocation_preserves_cross_axis_cancellation() {
        let response = DMat3::from_cols(DVec3::new(1., 0.8, 0.), DVec3::Y, DVec3::Z);
        let command = allocate(response, DVec3::new(3., 0., 0.));
        let achieved = response * command;
        assert!(command.x > 0.99 && achieved.y.abs() < 0.01 && achieved.z.abs() < 0.01);
        assert_eq!(allocate(DMat3::ZERO, DVec3::ONE), DVec3::ZERO);
    }

    #[test]
    fn fbw_cannot_create_control_torque_in_vacuum_with_engines_off() {
        let mut f = crate::combat::bvr::flight();
        select(&mut f, Aircraft::F22);
        f.position = surface_point(0., 0., 200000.);
        f.engine = false;
        f.infinite_fuel = true;
        f.rates = DVec3::new(0.1, 0.07, -0.1);
        f.controls = DVec3::ONE;
        let momentum = f.orientation * (flight_model::inertia(&f) * f.rates);
        let e = Environment {
            wind_enabled: false,
            ..default()
        };
        for _ in 0..240 {
            crate::physics::simulate(&mut f, &e, 1. / 120.);
        }
        let after = f.orientation * (flight_model::inertia(&f) * f.rates);
        assert_eq!(f.density, 0.);
        assert!(after.distance(momentum) / momentum.length() < 0.005);
    }
}
