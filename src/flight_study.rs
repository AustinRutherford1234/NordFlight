//! Repeatable numerical studies. Run without a renderer using --study-flight.
use crate::{aerodynamics as aero, control_surfaces as surfaces, physics::*};
use bevy::math::{DQuat, DVec3};
use serde_json::json;

pub fn run() {
    let started = std::time::Instant::now();
    let mut polars = Vec::new();
    for mach in [0.15, 0.4, 0.8, 1.0, 1.2, 1.8, 2.2] {
        for flap in [0.0, 0.3, 0.65] {
            for degrees in [
                -30.0_f64, -20.0, -10.0, 0.0, 5.0, 10.0, 15.0, 20.0, 25.0, 30.0, 45.0, 70.0, 100.0,
                160.0,
            ] {
                let alpha = degrees.to_radians();
                let speed = mach * 340.0;
                let air = DVec3::new(0.0, -speed * alpha.sin(), -speed * alpha.cos());
                let conditions = aero::Conditions {
                    alpha,
                    beta: 0.0,
                    mach,
                    speed,
                    density: 1.225,
                    temperature: 288.0,
                    flaps: 0.0,
                    gear: false,
                    airbrake: false,
                    height: 1000.0,
                    rates: DVec3::ZERO,
                };
                let mut state = aero::FlowState::default();
                let mut coeff = aero::Coefficients::default();
                for _ in 0..400 {
                    coeff = aero::evaluate(conditions, &mut state, 0.01);
                }
                let panels = surfaces::evaluate_flight(
                    air,
                    DVec3::ZERO,
                    1.225,
                    coeff.control_scale,
                    DVec3::ZERO,
                    flap,
                    false,
                    mach,
                );
                let q_area = 0.5 * 1.225 * speed * speed * aero::AREA;
                let flow = air.normalize();
                let lift = DVec3::X.cross(flow).normalize();
                let cl = coeff.lift + panels.force.dot(lift) / q_area;
                let cd = coeff.drag - panels.force.dot(flow) / q_area
                    + aero::induced_drag(cl, coeff.attached, 0.0, 1000.0)
                    - coeff.induced;
                assert!(
                    cl.is_finite() && cd.is_finite() && cd >= 0.0,
                    "invalid polar at Mach {mach} alpha {degrees}"
                );
                polars.push(json!({"mach":mach,"flaps":flap,"alpha_deg":degrees,"cl":cl,"cd":cd,"attached":coeff.attached,"control_scale":coeff.control_scale,"pitch_moment_nm":panels.moment.x}));
            }
        }
    }
    let mut maneuvers = Vec::new();
    for fbw in [false, true] {
        for (name, axis) in [("pitch", DVec3::X), ("yaw", DVec3::Y), ("roll", DVec3::Z)] {
            let mut f = Flight::approach();
            let mut e = Environment::default();
            e.wind_enabled = false;
            f.position = surface_point(0.0, 0.0, 3000.0);
            f.velocity = -DVec3::Z * 180.0;
            f.orientation = DQuat::from_rotation_x(0.04);
            f.flaps = 0.0;
            f.gear = false;
            f.assist = fbw;
            f.throttle = 0.6;
            f.spool = 0.6;
            let mut max_rates = DVec3::ZERO;
            let mut max_g = 0.0_f64;
            let mut min_attachment = 1.0_f64;
            for step in 0..2400 {
                let t = step as f64 / 120.0;
                f.controls = if (2.0..4.0).contains(&t) {
                    axis * 0.35
                } else if (4.0..6.0).contains(&t) {
                    -axis * 0.35
                } else {
                    DVec3::ZERO
                };
                e.advance(1.0 / 120.0);
                simulate(&mut f, &e, 1.0 / 120.0);
                assert!(
                    f.position.is_finite() && f.velocity.is_finite() && f.orientation.is_finite()
                );
                max_rates = max_rates.max(f.rates.abs());
                max_g = max_g.max(f.g_load.abs());
                min_attachment = min_attachment.min(f.flow.attached);
            }
            maneuvers.push(json!({"axis":name,"fbw":fbw,"max_rates_rad_s":max_rates.to_array(),"max_g":max_g,"min_attached":min_attachment,"final_altitude_m":f.altitude(),"crashed":f.crashed,"final_status":f.status}));
        }
    }
    // CPU-only physics throughput: renderer, VSync and GPU work are excluded.
    let benchmark_start = std::time::Instant::now();
    let mut f = Flight::spacecraft();
    let mut e = Environment::default();
    for _ in 0..432000 {
        e.advance(1.0 / 120.0);
        simulate(&mut f, &e, 1.0 / 120.0);
    }
    let orbit_seconds = benchmark_start.elapsed().as_secs_f64();
    let benchmark_start = std::time::Instant::now();
    let mut jet_steps = 0;
    for _ in 0..100 {
        let mut f = Flight::approach();
        let mut e = Environment::default();
        e.wind_enabled = false;
        f.autopilot = true;
        for _ in 0..14400 {
            e.advance(1.0 / 120.0);
            simulate(&mut f, &e, 1.0 / 120.0);
            jet_steps += 1;
        }
        assert!(
            !f.crashed && f.landed && f.velocity.length() < 1.0,
            "landing study regression"
        );
    }
    let jet_seconds = benchmark_start.elapsed().as_secs_f64();
    let report = json!({"description":"Numerical game-model study; not flight-data validation","polars":polars,"maneuvers":maneuvers,"benchmark":{"jet_steps":jet_steps,"jet_seconds":jet_seconds,"jet_us_per_step":jet_seconds*1e6/jet_steps as f64,"orbit_steps":432000,"orbit_seconds":orbit_seconds},"elapsed_seconds":started.elapsed().as_secs_f64()});
    std::fs::create_dir_all("tests").expect("study output directory");
    std::fs::write(
        "tests/aerodynamic-study.json",
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .expect("write aerodynamic study");
    println!(
        "294 polar points, 6 maneuver scenarios, 100 landing repeats; jet {:.2} us/step, orbit {:.2} s. Report: tests/aerodynamic-study.json",
        jet_seconds * 1e6 / jet_steps as f64,
        orbit_seconds
    );
}
