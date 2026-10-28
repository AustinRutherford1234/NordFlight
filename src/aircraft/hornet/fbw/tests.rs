use super::*;
fn flight(speed: f64, flaps: f64) -> Flight {
    let mut f = Flight::approach();
    crate::aircraft::select(&mut f, crate::aircraft::Aircraft::Hornet);
    f.position = surface_point(0., 0., 3000.);
    f.velocity = DVec3::NEG_Z * speed;
    f.airspeed = speed;
    f.gear = false;
    f.gear_extension = 0.;
    f.flaps = flaps;
    f.landed = false;
    f.autopilot = false;
    let t = trim::solve(&f, 3000., speed, 0., 0.).unwrap();
    f.orientation = DQuat::from_rotation_x(t.alpha);
    trim::apply(&mut f, t);
    f.assist = true;
    f.infinite_fuel = true;
    f.hornet.fbw = State::default();
    f
}
fn environment() -> Environment {
    Environment {
        wind_enabled: false,
        ..Default::default()
    }
}
fn run(f: &mut Flight, seconds: f64) {
    let e = environment();
    for _ in 0..(seconds * 240.).round() as usize {
        simulate(f, &e, 1. / 240.);
    }
}
#[test]
fn commands_do_not_move_aircraft_or_change_power_flaps() {
    let mut f = flight(180., 0.55);
    f.controls = DVec3::new(0.3, 0.1, 0.2);
    let before = (
        f.position,
        f.orientation,
        f.velocity,
        f.rates,
        f.throttle,
        f.flaps,
        f.fuel,
    );
    commands(&mut f, 15000., 1. / 240.);
    assert_eq!(
        before,
        (
            f.position,
            f.orientation,
            f.velocity,
            f.rates,
            f.throttle,
            f.flaps,
            f.fuel
        )
    );
}
#[test]
fn redundancy_sensor_outlier_and_split_vote() {
    let mut f = flight(180., 0.);
    f.hornet.fbw.sensors.faults[3].alpha_bias = 0.5;
    run(&mut f, 0.15);
    assert_eq!(f.hornet.fbw.sensors.counts[1], 3);
    assert!(f.hornet.fbw.sensors.rejected[3]);
    assert_eq!(f.hornet.fbw.mode, Mode::Normal);
    let mut f = flight(180., 0.);
    for i in 2..4 {
        f.hornet.fbw.sensors.faults[i].rate_bias = DVec3::splat(1.);
    }
    run(&mut f, 0.05);
    assert_eq!(f.hornet.fbw.sensors.counts[0], 0);
    assert_eq!(f.hornet.fbw.mode, Mode::Direct);
}
#[test]
fn failed_computer_is_rejected_and_rejoins_after_agreement() {
    let mut f = flight(180., 0.);
    f.hornet.fbw.computers[3].command_bias = DVec3::splat(10.);
    run(&mut f, 0.2);
    assert!(f.hornet.fbw.computers[3].isolated);
    assert_eq!(f.hornet.fbw.channels, 3);
    f.hornet.fbw.computers[3].command_bias = DVec3::ZERO;
    run(&mut f, 2.3);
    assert!(!f.hornet.fbw.computers[3].isolated);
}
#[test]
fn channel_modes_and_power_loss() {
    let mut f = flight(180., 0.);
    f.hornet.fbw.computers[3].enabled = false;
    f.hornet.fbw.computers[2].enabled = false;
    run(&mut f, 0.05);
    assert_eq!(f.hornet.fbw.mode, Mode::Degraded);
    f.hornet.fbw.computers[1].enabled = false;
    run(&mut f, 0.05);
    assert_eq!(f.hornet.fbw.mode, Mode::Direct);
    f.hornet.fbw.computers[0].enabled = false;
    let angles = f.hornet.angles;
    f.hornet.actuator_rates = [0.; 16];
    run(&mut f, 0.05);
    assert_eq!(f.hornet.fbw.mode, Mode::Failed);
    assert_eq!(f.hornet.angles, angles);
    let mut f = flight(180., 0.);
    f.battery = false;
    f.apu = false;
    f.engine = false;
    run(&mut f, 0.05);
    assert_eq!(f.hornet.fbw.mode, Mode::Failed);
}
#[test]
fn invalid_airdata_uses_only_rate_damping() {
    let mut f = flight(180., 0.);
    for fault in &mut f.hornet.fbw.sensors.faults {
        fault.air_valid = false;
    }
    run(&mut f, 0.05);
    assert_eq!(f.hornet.fbw.mode, Mode::Degraded);
    assert!(!f.hornet.fbw.protection);
    assert!(f.hornet.angles.iter().all(|a| a.is_finite()));
}
#[test]
fn direct_override_and_bumpless_reengagement() {
    let mut f = flight(180., 0.);
    run(&mut f, 1.);
    f.assist = false;
    f.controls.x = 0.2;
    let old = f.hornet.angles;
    simulate(&mut f, &environment(), 1. / 240.);
    assert_eq!(f.hornet.fbw.mode, Mode::Direct);
    for i in 0..16 {
        assert!((old[i] - f.hornet.angles[i]).abs() < 0.005);
    }
    f.assist = true;
    let old = f.hornet.angles;
    simulate(&mut f, &environment(), 1. / 240.);
    for i in 0..16 {
        assert!((old[i] - f.hornet.angles[i]).abs() < 0.005);
    }
}
#[test]
fn neutral_trim_pitch_and_roll_response() {
    for flaps in [0., 0.55] {
        let mut f = flight(180., flaps);
        run(&mut f, 2.);
        let alpha = f.alpha;
        let tail = f.hornet.trim;
        f.controls.x = 0.15;
        run(&mut f, 4.);
        println!("PITCH flaps={} n={} alpha={} pitch={} rates={:?} trim={} initial_trim={} saturation={}",flaps,f.g_load,f.alpha,f.pitch(),f.rates,f.hornet.trim,tail,f.hornet.fbw.saturation);
        assert!(!f.crashed && f.g_load > 0. && f.alpha > alpha - 0.05);
        f.controls = DVec3::ZERO;
        run(&mut f, 6.);
        println!(
            "RELEASE flaps={} n={} alpha={} rates={:?}",
            flaps, f.g_load, f.alpha, f.rates
        );
        assert!(f.rates.length() < 0.5);
    }
    for roll in [-0.5, 0.5] {
        let mut f = flight(180., 0.);
        run(&mut f, 1.);
        f.controls.z = roll;
        run(&mut f, 1.5);
        println!(
            "ROLL input={} rates={:?} beta={} alpha={}",
            roll, f.rates, f.beta, f.alpha
        );
        assert!(f.rates.z * roll < 0.);
        f.controls = DVec3::ZERO;
        run(&mut f, 4.);
        assert!(f.rates.z.abs() < 0.4);
    }
}
#[test]
fn high_load_protection_and_surface_failure_remain_bounded() {
    for failed in [false, true] {
        let mut f = flight(230., 0.);
        if failed {
            f.damage.surface_state[0] =
                crate::combat::damage::SurfaceState::Jammed(f.hornet.angles[0]);
        }
        run(&mut f, 1.);
        f.controls.x = 1.;
        let mut maxg: f64 = 0.;
        let mut alpha: f64 = 0.;
        for _ in 0..720 {
            simulate(&mut f, &environment(), 1. / 240.);
            maxg = maxg.max(f.g_load);
            alpha = alpha.max(f.alpha);
        }
        println!(
            "ENVELOPE failed={} peak_g={} peak_alpha={} rates={:?}",
            failed, maxg, alpha, f.rates
        );
        assert!(!f.crashed && maxg < 10. && alpha < 0.9);
        assert!(f
            .hornet
            .fbw
            .computers
            .iter()
            .all(|c| c.law.integral.abs().max_element() <= 0.180001));
    }
}
#[test]
fn autopilot_is_not_a_hornet_control_path() {
    let mut f = flight(180., 0.);
    f.autopilot = true;
    let throttle = f.throttle;
    run(&mut f, 0.1);
    assert!(!f.autopilot);
    assert_eq!(f.throttle, throttle);
}

#[test]
fn frozen_and_nonfinite_sensor_lanes_do_not_poison_controls() {
    let mut f = flight(180., 0.);
    run(&mut f, 0.2);
    f.hornet.fbw.sensors.faults[3].frozen = true;
    f.hornet.fbw.sensors.faults[2].alpha_bias = f64::NAN;
    f.controls.z = 0.5;
    run(&mut f, 1.);
    assert!(f.hornet.fbw.sensors.rejected[3]);
    assert!(f.hornet.fbw.sensors.rejected[2]);
    assert!(f.hornet.angles.iter().all(|x| x.is_finite()));
    assert!(!f.crashed);
}
#[test]
fn failed_surface_is_excluded_and_other_effectors_take_the_load() {
    let mut f = flight(180., 0.);
    let jam = f.hornet.angles[0];
    f.damage.surface_state[0] = crate::combat::damage::SurfaceState::Jammed(jam);
    f.controls.x = 0.2;
    run(&mut f, 2.);
    assert_eq!(f.hornet.angles[0], jam);
    assert_eq!(f.hornet.fbw.mode, Mode::Degraded);
    assert!((f.hornet.angles[4] - jam).abs() > 0.01);
    assert!(f.hornet.fbw.output.iter().all(|a| a.abs() <= 0.550001));
    assert!(!f.crashed);
}
#[test]
fn ground_neutral_does_not_drive_trim_to_a_stop() {
    let mut f = crate::carrier::scenario(true);
    let trim = f.hornet.trim;
    f.throttle = 1.;
    run(&mut f, 4.);
    assert!(f.landed && f.hornet.fbw.ground);
    assert!((f.hornet.angles[0] - trim).abs() < 0.1);
    assert!(!f.hornet.fbw.protection);
    assert_eq!(f.hornet.trim, trim);
}
#[test]
fn pause_reset_and_manual_configuration_are_preserved() {
    let mut f = flight(180., 0.);
    run(&mut f, 0.2);
    let before = format!("{:?}", f.hornet.fbw);
    f.paused = true;
    simulate(&mut f, &environment(), 1.);
    assert_eq!(before, format!("{:?}", f.hornet.fbw));
    f.paused = false;
    f.flaps = 0.55;
    run(&mut f, 1.);
    assert_eq!(f.flaps, 0.55);
    assert!(f.hornet.angles[1] > 0.2);
    crate::aircraft::select(&mut f, crate::aircraft::Aircraft::Hornet);
    assert!(!f.hornet.fbw.initialized);
    assert_eq!(
        f.hornet.fbw.computers.iter().filter(|c| c.enabled).count(),
        4
    );
}
#[test]
fn engine_out_and_gust_recovery() {
    for engine_out in [false, true] {
        let mut f = flight(180., 0.);
        run(&mut f, 1.);
        if engine_out {
            f.systems.engine_on[0] = false;
        } else {
            f.velocity += f.orientation * DVec3::new(12., -8., 0.);
        }
        run(&mut f, 10.);
        println!(
            "RECOVERY engine_out={} g={} beta={} rates={:?}",
            engine_out, f.g_load, f.beta, f.rates
        );
        assert!(!f.crashed && f.beta.abs() < 0.12 && f.rates.length() < 0.2);
    }
}
#[test]
fn frame_partition_and_cost() {
    let initial = flight(180., 0.);
    let mut runs = Vec::new();
    let mut timings = Vec::new();
    for hz in [60, 120, 240] {
        let mut f = initial.clone();
        for i in 0..hz * 8 {
            f.controls = if i < hz * 3 {
                DVec3::new(0.15, 0., 0.1)
            } else {
                DVec3::ZERO
            };
            let start = std::time::Instant::now();
            simulate(&mut f, &environment(), 1. / hz as f64);
            if hz == 60 {
                timings.push(start.elapsed().as_secs_f64() * 1000.);
            }
        }
        assert!(!f.crashed);
        runs.push(f);
    }
    for f in &runs[..2] {
        assert!((f.position - runs[2].position).length() < 0.1);
        assert!((f.rates - runs[2].rates).length() < 0.001);
    }
    timings.sort_by(f64::total_cmp);
    let mean = timings.iter().sum::<f64>() / timings.len() as f64;
    let p95 = timings[timings.len() * 95 / 100];
    println!("FBW_COST mean_ms={mean} p95_ms={p95}");
    // Budget acceptance belongs to an isolated target-hardware run. Shared
    // CI scheduling and parallel geometry tests are not physics cost.
    if std::env::var_os("NORD_FBW_PERF_BUDGET").is_some() {
        assert!(mean < 2. && p95 < 4.);
    }
}
#[test]
fn response_matrix_writes_diagnostic_evidence() {
    let mut rows = Vec::new();
    for (speed, flaps, pitch, roll) in [
        (120., 0.55, 0.15, 0.),
        (180., 0., 0.15, 0.),
        (250., 0., 0.4, 0.),
        (180., 0., 0., 0.5),
        (180., 0., 0., -0.5),
    ] {
        let mut f = flight(speed, flaps);
        let mut peak_g: f64 = 0.;
        let mut peak_alpha: f64 = 0.;
        for i in 0..2400 {
            let t = i as f64 / 240.;
            f.controls = if (1. ..4.).contains(&t) {
                DVec3::new(pitch, 0., roll)
            } else {
                DVec3::ZERO
            };
            simulate(&mut f, &environment(), 1. / 240.);
            peak_g = peak_g.max(f.g_load);
            peak_alpha = peak_alpha.max(f.alpha);
            if i % 60 == 0 {
                rows.push(serde_json::json!({"speed_case":speed,"flaps":flaps,"pitch_input":pitch,"roll_input":roll,"time":t,"g":f.g_load,"alpha_deg":f.alpha.to_degrees(),"beta_deg":f.beta.to_degrees(),"rates":f.rates.to_array(),"command_rates":f.hornet.fbw.requested_rates.to_array(),"mode":format!("{:?}",f.hornet.fbw.mode),"saturation":f.hornet.fbw.saturation,"residual":f.hornet.fbw.allocation_residual.to_array(),"trim_deg":f.hornet.trim.to_degrees()}));
            }
        }
        println!(
            "MATRIX v={} flap={} pitch={} roll={} peak_g={} alpha={} end_rates={:?}",
            speed, flaps, pitch, roll, peak_g, peak_alpha, f.rates
        );
        assert!(!f.crashed);
        assert!(peak_alpha < 0.75 && peak_g < 9.);
    }
    std::fs::create_dir_all("tests/hornet-fbw").unwrap();
    std::fs::write(
        "tests/hornet-fbw/response.json",
        serde_json::to_string_pretty(&rows).unwrap(),
    )
    .unwrap();
}
