use super::*;
use super::{math::DEG, trim::condition};
use crate::{aerodynamics::FlowState, physics::*};
use bevy::math::{DMat3, DVec3};
#[test]
fn polars_are_finite_passive_symmetric_and_continuous() {
    let mut last: Option<f64> = None;
    for n in 0..1001 {
        let m = n as f64 * 0.0036;
        let mut c = condition(0.09, 10000., 300.);
        c.mach = m;
        let a = steady(c);
        assert!(a.lift.is_finite() && a.drag >= 0. && a.pitch.is_finite());
        if let Some(old) = last {
            assert!((a.drag - old).abs() < 0.003);
        }
        last = Some(a.drag);
        let air = DVec3::new(0., -c.alpha.sin(), -c.alpha.cos()) * c.speed;
        assert!(crate::aerodynamics::force(DVec3::X, air, 1., a).dot(air) <= 1e-8);
        c.beta = 0.05;
        let right = steady(c);
        c.beta = -0.05;
        let left = steady(c);
        assert!((left.side + right.side).abs() < 1e-10);
        assert!((left.yaw + right.yaw).abs() < 1e-10);
    }
    for alpha in [-3., -1.5, -0.5, 0., 0.5, 1.5, 3.] {
        let mut c = condition(alpha, 5000., 0.);
        c.mach = 0.;
        let a = steady(c);
        let force = crate::aerodynamics::force(DVec3::X, DVec3::ZERO, 0., a);
        assert_eq!(force, DVec3::ZERO);
    }
}
#[test]
fn separation_recovers_with_memory_and_gear_dissipates_energy() {
    let mut c = condition(0.65, 4000., 160.);
    let mut state = FlowState::default();
    for _ in 0..240 {
        evaluate(c, &mut state, 1. / 120.);
    }
    assert!(state.attached < 0.25);
    c.alpha = 0.12;
    let first = evaluate(c, &mut state, 1. / 120.);
    assert!(first.attached < 0.4);
    for _ in 0..1200 {
        evaluate(c, &mut state, 1. / 120.);
    }
    assert!(state.attached > 0.99);
    let clean = steady(c);
    c.gear = true;
    assert!(steady(c).drag > clean.drag + 0.05);
    assert!(induced(0.6, 1., 0., 1., 0.5) < induced(0.6, 1., 0., 1000., 0.5));
}
#[test]
fn inertia_and_control_axes_have_physical_signs() {
    let f = Flight::runway();
    let i = inertia(&f);
    assert!(i.determinant() > 0.);
    assert!(i.x_axis.x > 0. && i.y_axis.y > 0. && i.z_axis.z > 0.);
    for c in [DVec3::X, DVec3::Y, DVec3::Z] {
        let coeff = control_coefficients(targets(c), [c.y * 20_f64.to_radians(); 2], 2., 1.);
        if c.x > 0. {
            assert!(coeff.pitch > 0.);
        }
        if c.y > 0. {
            assert!(coeff.yaw > 0.);
        }
        if c.z > 0. {
            assert!(coeff.roll < 0.);
        }
    }
    for fuel in [0., 10000., FUEL] {
        let mut f = f.clone();
        f.fuel = fuel;
        let i = inertia(&f);
        let inverse = i.inverse();
        assert!(
            (inverse * i - DMat3::IDENTITY)
                .to_cols_array()
                .iter()
                .all(|x| x.abs() < 1e-9)
        );
    }
}
#[test]
fn trim_envelope_register() {
    let mut rows =
        String::from("altitude_m,speed_m_s,mass_kg,alpha_deg,elevon_command,throttle,residual\n");
    let mut solved = 0;
    for fuel in [8000., 20000., 30000.] {
        for (alt, speed) in [
            (1000., 120.),
            (10000., 240.),
            (18000., 600.),
            (23000., 900.),
            (25000., 950.),
        ] {
            let mass = EMPTY + fuel;
            if let Some(t) = trim(mass, fuel, alt, speed) {
                solved += 1;
                rows += &format!(
                    "{alt},{speed},{mass},{},{},{},{}\n",
                    t.alpha * DEG,
                    t.command,
                    t.throttle,
                    t.residual
                );
                assert!(t.residual < 0.001);
            } else {
                rows += &format!("{alt},{speed},{mass},UNTRIMMABLE,,,\n");
            }
        }
    }
    std::fs::create_dir_all("tests/sr71-aero").unwrap();
    std::fs::write("tests/sr71-aero/trim.csv", rows.clone()).unwrap();
    println!("{rows}");
    assert!(solved >= 12, "only {solved}/15 trim points solved");
}
fn airborne(alt: f64, speed: f64) -> Flight {
    let mut f = Flight::approach();
    let up = f.up();
    f.position = up * (RADIUS + alt);
    f.velocity = (f.forward() - up * f.forward().dot(up)).normalize() * speed;
    f.gear = false;
    f.gear_extension = 0.;
    f.gear_door = 0.;
    f.flaps = 0.;
    f.landed = false;
    f.paused = false;
    crate::aircraft::select(&mut f, crate::aircraft::Aircraft::Sr71);
    f
}
#[test]
fn flight_and_control_release_converge_across_step_rates() {
    let mut rows =
        String::from("hz,time,altitude,mach,alpha,pitch_rate,yaw_rate,roll_rate,g,integrity\n");
    let mut results = Vec::new();
    for hz in [60, 120, 240] {
        let mut f = airborne(23000., 900.);
        let env = Environment {
            wind_enabled: false,
            ..Default::default()
        };
        for step in 0..hz * 60 {
            let t = step as f64 / hz as f64;
            f.controls = if (10.0..12.0).contains(&t) {
                DVec3::new(0.15, 0., 0.10)
            } else {
                DVec3::ZERO
            };
            simulate(&mut f, &env, 1. / hz as f64);
            if step % hz == 0 {
                rows += &format!(
                    "{hz},{t},{},{},{},{},{},{},{},{}\n",
                    f.altitude(),
                    f.mach,
                    f.alpha,
                    f.rates.x,
                    f.rates.y,
                    f.rates.z,
                    f.g_load,
                    f.integrity
                );
            }
            assert!(
                !f.crashed && f.position.is_finite() && f.rates.is_finite(),
                "hz={hz} t={t} {} M={} alpha={} rates={:?}",
                f.status,
                f.mach,
                f.alpha,
                f.rates
            );
        }
        println!(
            "SR71 FLIGHT hz={hz} alt={} M={} rates={:?} integrity={}",
            f.altitude(),
            f.mach,
            f.rates,
            f.integrity
        );
        results.push(f);
    }
    std::fs::write("tests/sr71-aero/trajectory.csv", rows).unwrap();
    for f in &results[..2] {
        assert!(f.velocity.distance(results[2].velocity) / results[2].velocity.length() < 0.02);
        assert!(f.position.distance(results[2].position) < 100.);
    }
    assert!(results[2].integrity > 0.98);
    assert!(results[2].rates.length() < 0.08);
}
#[test]
fn neutral_trim_remains_near_level_and_evaluation_is_bounded() {
    let mut f = airborne(23000., 900.);
    let alt = f.altitude();
    let env = Environment {
        wind_enabled: false,
        ..Default::default()
    };
    for _ in 0..120 * 60 {
        simulate(&mut f, &env, 1. / 120.);
    }
    println!(
        "SR71 NEUTRAL altitude_change={} mach={} rates={:?} g={}",
        f.altitude() - alt,
        f.mach,
        f.rates,
        f.g_load
    );
    assert!(
        (f.altitude() - alt).abs() < 100.,
        "neutral trim departed: {}m",
        f.altitude() - alt
    );
    assert!(f.rates.length() < 0.01 && f.integrity > 0.99);
    let start = std::time::Instant::now();
    let mut state = FlowState::default();
    let c = condition(0.1, 23000., 900.);
    for _ in 0..100000 {
        std::hint::black_box(evaluate(std::hint::black_box(c), &mut state, 1. / 120.));
    }
    println!(
        "SR71 coefficient evaluation: {} ns/call (100000 iterations; test-profile microbenchmark)",
        start.elapsed().as_nanos() / 100000
    );
}
#[test]
fn engine_shutdown_and_asymmetry_have_no_free_power() {
    let mut f = airborne(15000., 350.);
    f.engine = false;
    simulate(&mut f, &Environment::default(), 1. / 120.);
    assert_eq!(f.thrust, 0.);
    assert_eq!(fuel_flow(&f), 0.);
    assert_eq!(engine(0., 1.225, 0.).0, 0.);
    f.engine = true;
    f.spool = 1.;
    f.systems.engine_on = [true, false];
    f.systems.engines = [1.; 2];
    let (_, moment, _) = crate::aircraft::propulsion(&mut f, 0.01, 0.2, 1.2);
    assert!(moment.y < 0.);
}

#[test]
fn checked_evaluation_rejects_bad_inputs_and_repairs_only_flow_history() {
    use super::{aero::evaluate_checked, diagnostics::EvaluationError};
    let c = condition(0.1, 10000., 300.);
    let mut s = FlowState::default();
    let original = s;
    let mut bad = c;
    bad.alpha = f64::NAN;
    assert_eq!(
        evaluate_checked(bad, &mut s, 0.01).unwrap_err(),
        EvaluationError::NonFiniteInput
    );
    assert_eq!(s.lag_alpha, original.lag_alpha);
    assert_eq!(
        evaluate_checked(c, &mut s, -0.01).unwrap_err(),
        EvaluationError::NegativeTimeStep
    );
    bad = c;
    bad.density = -1.;
    assert_eq!(
        evaluate_checked(bad, &mut s, 0.).unwrap_err(),
        EvaluationError::InvalidAtmosphere
    );
    s.lag_alpha = f64::NAN;
    s.wing_attached = [2., f64::NAN];
    let result = evaluate_checked(c, &mut s, 1. / 120.).unwrap();
    assert!(result.lift.is_finite());
    assert!(super::diagnostics::flow_is_valid(&s));
    for a in [-3., -1., 0., 0.2, 1., 3.] {
        for m in [0., 0.8, 1., 1.2, 3.2, 4.] {
            let mut c = condition(a, 12000., 300.);
            c.mach = m;
            let mut state = FlowState::default();
            for _ in 0..100 {
                let o = evaluate_checked(c, &mut state, 1. / 120.).unwrap();
                assert!(
                    o.lift.is_finite() && o.drag.is_finite() && o.drag >= 0. && o.pitch.is_finite()
                );
            }
        }
    }
}
#[test]
fn detached_surfaces_have_no_phantom_rigging_load_and_individual_loads_close() {
    use crate::combat::damage::SurfaceState;
    let mut f = airborne(15000., 400.);
    let b = f.orientation.conjugate() * f.velocity;
    for i in [0, 4, 2, 6, 3, 7] {
        f.damage.surface_state[i] = SurfaceState::Detached;
    }
    let load = control_loads(&f.systems.sr71, &f.damage, 1., b, 0.2, 1.4, None);
    assert!(
        load.force.length() < 1e-8 && load.moment.length() < 1e-8,
        "detached surfaces generated {load:?}"
    );
    f.damage = AircraftDamage::default();
    f.damage.surface_state[0] = SurfaceState::Jammed(0.15);
    for command in [DVec3::new(0.1, 0.2, -0.2), DVec3::new(-0.7, -0.3, 0.4)] {
        f.surface = command;
        crate::aircraft::sr71::advance(&mut f, 1., 1.4);
        assert_eq!(f.systems.sr71.elevons[0], 0.15);
        let load = control_loads(&f.systems.sr71, &f.damage, 1., b, 0.2, 1.4, None);
        let sf = load.surfaces.iter().map(|s| s.force).sum::<DVec3>();
        let sm = load.surfaces.iter().map(|s| s.moment).sum::<DVec3>();
        assert!((sf - load.force).length() < 1e-8);
        assert!((sm - load.moment).length() < 1e-8);
    }
}
#[test]
fn trim_reports_invalid_or_unsustainable_cases_instead_of_hiding_them() {
    use super::trim::{TrimFailure, solve};
    assert_eq!(
        solve(EMPTY + 10000., 10000., 10000., 0.).failure,
        Some(TrimFailure::InsufficientSpeed)
    );
    assert_eq!(
        solve(f64::NAN, 10000., 10000., 300.).failure,
        Some(TrimFailure::InvalidInput)
    );
    assert!(
        solve(EMPTY + 30000., 30000., 25000., 200.)
            .failure
            .is_some()
    );
    let valid = solve(EMPTY + 10000., 10000., 23000., 900.);
    assert!(valid.solution.is_some(), "{valid:?}");
    assert!(
        valid
            .residual
            .iter()
            .all(|x| x.abs() < super::config::SOLVER.residual_tolerance)
    );
}
#[test]
fn diagnostic_budget_closes_and_reset_clears_failure_history() {
    use super::diagnostics::*;
    let mut f = airborne(23000., 900.);
    let env = Environment {
        wind_enabled: false,
        ..Default::default()
    };
    f.systems.engine_on[0] = false;
    f.damage.hydraulics = 0.;
    f.flow.lag_alpha = f64::NAN;
    simulate(&mut f, &env, 1. / 120.);
    let d = &f.systems.sr71.diagnostics;
    let s = d.latest;
    assert!(
        s.flags & ENGINE_OUT != 0 && s.flags & HYDRAULIC_LOSS != 0 && s.flags & FLOW_RESET != 0
    );
    assert_eq!(d.repaired_flow_states, 1);
    assert!(s.flags & INVALID_LOAD == 0);
    let forces = [
        s.baseline_force_body_n,
        s.control_force_body_n,
        s.damage_force_body_n,
        s.engine_force_body_n,
    ]
    .into_iter()
    .map(DVec3::from_array)
    .sum::<DVec3>();
    assert!((forces - DVec3::from_array(s.total_specific_force_body_n)).length() < 1e-7);
    let moments = [
        s.baseline_moment_body_nm,
        s.control_moment_body_nm,
        s.damage_moment_body_nm,
        s.engine_moment_body_nm,
    ]
    .into_iter()
    .map(DVec3::from_array)
    .sum::<DVec3>();
    assert!((moments - DVec3::from_array(s.total_moment_body_nm)).length() < 1e-7);
    assert!(s.moment_closure_error_nm < 1e-7);
    let old = f.systems.sr71.elevons;
    f.surface = DVec3::ONE;
    crate::aircraft::sr71::advance(&mut f, 1., 3.);
    assert_eq!(old, f.systems.sr71.elevons);
    crate::aircraft::select(&mut f, crate::aircraft::Aircraft::Sr71);
    assert_eq!(f.systems.sr71.diagnostics.steps, 0);
    assert_eq!(f.systems.sr71.diagnostics.sticky_flags, 0);
}
#[test]
fn parameter_register_and_runtime_defaults_agree() {
    super::config::validate().unwrap();
    let v = super::config::register();
    assert_eq!(
        v["aerodynamics"]["gear_drag"].as_f64().unwrap(),
        super::config::AERO.gear_drag
    );
    for p in [f64::NAN, -1., 2.] {
        let (t, b) = engine(p, 1.225, 0.);
        assert!(t.is_finite() && b.is_finite());
    }
    assert_eq!(engine(1., -1., 0.), (0., 0.));
}

#[test]
fn jammed_surfaces_cannot_exceed_stops_and_visual_load_poses_agree() {
    use crate::combat::damage::SurfaceState;
    let mut f = airborne(15000., 400.);
    let b = f.orientation.conjugate() * f.velocity;
    f.damage.surface_state[0] = SurfaceState::Jammed(2.);
    f.damage.surface_state[3] = SurfaceState::Jammed(-2.);
    crate::aircraft::sr71::advance(&mut f, 1., 1.4);
    let l = control_loads(&f.systems.sr71, &f.damage, 1., b, 0.2, 1.4, None);
    assert_eq!(
        f.systems.sr71.elevons[0],
        super::config::CONTROL.inner_down_rad
    );
    assert_eq!(
        f.systems.sr71.fins[0],
        -super::config::CONTROL.rudder_limit_rad
    );
    assert_eq!(l.surfaces[0].deflection, f.systems.sr71.elevons[0]);
    assert_eq!(l.surfaces[3].deflection, f.systems.sr71.fins[0]);
}

#[test]
fn incidence_wrap_does_not_inject_a_full_revolution_vortex_transient() {
    let pi = std::f64::consts::PI;
    let mut c = condition(-pi + 0.001 + INCIDENCE, 10000., 300.);
    c.mach = 0.9;
    let mut state = FlowState {
        lag_alpha: pi - 0.001,
        lag_beta: 0.,
        vortex_lift: 0.,
        attached: 0.,
        wing_attached: [0.; 2],
    };
    let coeff = evaluate(c, &mut state, 0.01);
    assert!(
        state.vortex_lift.abs() < 0.001,
        "wrapped incidence created false transient {}",
        state.vortex_lift
    );
    assert!(coeff.lift.is_finite() && coeff.drag >= 0.);
}
