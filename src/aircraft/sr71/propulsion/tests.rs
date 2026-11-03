use super::{config::*, gaspath::*, inlet, state::*};
fn input() -> Input {
    Input {
        ambient: Ambient {
            density: 1.225,
            temperature: 288.15,
            pressure: 101325.,
            speed: 0.,
            axial_speed: 0.,
            mach: 0.,
        },
        alpha: 0.,
        beta: 0.,
        throttle: 0.88,
        run: true,
        ignition: true,
        ground_starter: false,
        hydraulic: 1.,
        health: 1.,
        nozzle_health: 1.,
        fuel_available: true,
    }
}
#[test]
fn component_station_budgets_and_static_rating_are_bounded() {
    for power in [0., 0.5, 0.88, 1.] {
        let i = Input {
            throttle: power,
            ..input()
        };
        let s = State::equilibrium(i);
        let o = s.output;
        println!(
            "J58 STEADY throttle={power} rpm={} thrust={} main={} AB={} T4={} T7={} shaft={}",
            s.rpm(),
            s.thrust_n,
            o.main_fuel_kg_s,
            o.ab_fuel_kg_s,
            o.combustor.total_temperature_k,
            o.afterburner.total_temperature_k,
            o.shaft_power_w
        );
        assert!(s.thrust_n.is_finite() && s.thrust_n >= 0.);
        assert!(o.air_mass_residual_kg_s.abs() < 1e-8);
        assert!(o.combustor_energy_residual_w.abs() < 1e-6);
        assert!(o.turbine_energy_residual_w.abs() < 1e-6);
        assert!(o.gas_energy_residual_w.abs() < 1e-5);
        assert!(o.shaft_power_w.abs() < 2.);
    }
    let wet = State::equilibrium(Input {
        throttle: 1.,
        ..input()
    });
    assert!(
        (130000.0..159000.0).contains(&wet.thrust_n),
        "wet thrust {}",
        wet.thrust_n
    );
}
#[test]
fn shaft_and_throttle_release_do_not_produce_nonfinite_states() {
    let mut s = State::equilibrium(input());
    for k in 0..480 * 20 {
        let i = Input {
            throttle: if k < 480 * 5 { 1. } else { 0.5 },
            ..input()
        };
        s.advance(i, 1. / 480.);
        assert!(s.n.is_finite() && s.output.net_thrust_n.is_finite() && s.n <= 1.06);
    }
    assert_eq!(s.mode, Mode::Running);
    assert_eq!((s.surges, s.flameouts), (0, 0));
    println!(
        "J58 DYNAMIC final mode={:?} rpm={} thrust={} surge={} flameout={}",
        s.mode,
        s.rpm(),
        s.thrust_n,
        s.surges,
        s.flameouts
    );
}
#[test]
fn ground_start_and_shutdown() {
    let mut s = State::default();
    for _ in 0..480 * 60 {
        s.advance(
            Input {
                ground_starter: true,
                throttle: 0.,
                ..input()
            },
            1. / 480.,
        );
    }
    println!(
        "J58 START mode={:?} rpm={} fuel={} shots={}",
        s.mode,
        s.rpm(),
        s.main_fuel_kg_s,
        s.teb_remaining
    );
    assert_eq!(s.mode, Mode::Running);
    assert_eq!((s.surges, s.flameouts, s.teb_remaining), (0, 0, 15));
    assert!((s.rpm() - DYN.rpm_idle).abs() < 200.);
    s.advance(
        Input {
            run: false,
            ..input()
        },
        1. / 480.,
    );
    assert_eq!(s.thrust_n, 0.);
    assert_eq!(s.main_fuel_kg_s + s.ab_fuel_kg_s, 0.);
}
#[test]
fn vacuum_invalid_inputs_and_failed_ignition_do_not_create_power() {
    let mut s = State::default();
    let old = s.n;
    s.advance(input(), f64::NAN);
    assert_eq!(old, s.n);
    assert_eq!(s.invalid_steps, 1);
    let out = evaluate(
        Ambient {
            density: 0.,
            pressure: 0.,
            ..input().ambient
        },
        1.,
        Geometry {
            recovery: 1.,
            bleed: 0.,
            throat_m2: 0.,
            flow_scale: 1.,
            health: 1.,
        },
        2.,
        2.,
    );
    assert_eq!(out.net_thrust_n, 0.);
    s.faults.ignition_failed = true;
    for _ in 0..480 * 30 {
        s.advance(
            Input {
                ground_starter: true,
                ..input()
            },
            1. / 480.,
        );
    }
    assert_ne!(s.mode, Mode::Running);
    assert_eq!(s.thrust_n, 0.);
}
#[test]
fn inlet_unstart_is_condition_driven_and_recoverable() {
    let a = Ambient {
        density: 0.06,
        temperature: 220.,
        pressure: 0.06 * GAS.r * 220.,
        speed: 900.,
        axial_speed: 900.,
        mach: 3.,
    };
    let mut s = inlet::State::equilibrium(a, 0., 0.);
    for _ in 0..480 {
        s.advance(a, 0., 0.3, 70., 0., 1., Some(0.), None, 1. / 480.);
    }
    assert!(s.unstarts > 0);
    assert!(s.drag_n > 1000.);
    for _ in 0..480 * 10 {
        s.advance(a, 0., 0., 70., 0., 1., None, None, 1. / 480.);
    }
    assert_eq!(s.mode, inlet::Mode::Started);
    assert!(s.recovery > 0.6);
}
fn cruise() -> Input {
    let temperature: f64 = 220.;
    let mach = 3.;
    let speed = mach * (GAS.gamma_air * GAS.r * temperature).sqrt();
    Input {
        ambient: Ambient {
            density: 0.06,
            temperature,
            pressure: 0.06 * GAS.r * temperature,
            speed,
            axial_speed: speed,
            mach,
        },
        throttle: 0.96,
        ..input()
    }
}
#[test]
fn steady_component_envelope_stays_running_without_unsolicited_failures() {
    for mut i in [input(), cruise()] {
        for power in [0., 0.5, 0.88, 1.] {
            i.throttle = power;
            let mut s = State::equilibrium(i);
            for _ in 0..480 * 60 {
                s.advance(i, 1. / 480.);
                assert_eq!(
                    s.mode,
                    Mode::Running,
                    "M={} throttle={} N={} margin={}",
                    i.ambient.mach,
                    power,
                    s.n,
                    s.surge_margin
                );
                assert!(s.output.gas_energy_residual_w.abs() < 1e-5);
                assert!(s.shaft_balance_residual_j.abs() < 1e-6);
            }
            println!(
                "STABLE M={} throttle={} rpm={} target={} fuel={} T4={}",
                i.ambient.mach,
                power,
                s.rpm(),
                demand_speed(power, i.ambient.mach) * DYN.rpm_max,
                s.main_fuel_kg_s + s.ab_fuel_kg_s,
                s.output.combustor.total_temperature_k
            );
            assert!((s.n - demand_speed(power, i.ambient.mach)).abs() < 0.02);
            assert_eq!((s.surges, s.flameouts), (0, 0));
        }
    }
}
#[test]
fn shutdown_windmilling_and_seizure_have_no_powered_thrust() {
    let i = cruise();
    let mut s = State::equilibrium(i);
    let before = s.shaft_energy_j;
    for _ in 0..480 * 10 {
        s.advance(Input { run: false, ..i }, 1. / 480.);
        assert_eq!(s.thrust_n, 0.);
        assert_eq!(s.fuel_used_kg, 0.);
        assert!(s.drag_n > 0.);
    }
    assert!(s.n > 0. && s.shaft_energy_j < before);
    s.faults.seized = true;
    s.advance(i, 1. / 480.);
    assert_eq!(s.mode, Mode::Seized);
    assert_eq!(s.n, 0.);
    assert_eq!(s.fuel_used_kg, 0.);
}
#[test]
fn teb_inventory_ignition_and_fuel_starvation_are_independent() {
    let mut s = State::equilibrium(input());
    s.teb_remaining = 0;
    for _ in 0..480 * 5 {
        s.advance(
            Input {
                throttle: 1.,
                ..input()
            },
            1. / 480.,
        );
    }
    assert_eq!(s.mode, Mode::Running);
    assert!(!s.afterburner_lit);
    assert_eq!(s.ab_fuel_kg_s, 0.);
    s.advance(
        Input {
            fuel_available: false,
            ..input()
        },
        1. / 480.,
    );
    assert_eq!(s.thrust_n, 0.);
    assert_eq!(s.fuel_used_kg, 0.);
    for _ in 0..480 * 40 {
        s.advance(
            Input {
                ground_starter: true,
                ..input()
            },
            1. / 480.,
        );
    }
    assert_ne!(s.mode, Mode::Running);
    assert_eq!(s.thrust_n, 0.);
}
#[test]
fn backward_taxi_does_not_extinguish_an_engine() {
    let mut s = State::equilibrium(input());
    let speed = 8.;
    let i = Input {
        alpha: std::f64::consts::PI,
        ambient: Ambient {
            speed,
            axial_speed: -speed,
            mach: speed / 340.,
            ..input().ambient
        },
        ..input()
    };
    for _ in 0..480 * 15 {
        s.advance(i, 1. / 480.);
        assert_eq!(s.mode, Mode::Running);
    }
    assert_eq!((s.surges, s.flameouts), (0, 0));
}
#[test]
fn nozzle_fault_is_finite_and_removes_authority() {
    let i = input();
    let mut s = State::equilibrium(i);
    let healthy = s.thrust_n;
    s.faults.nozzle_jammed_m2 = Some(DYN.nozzle_min_m2);
    for _ in 0..480 * 10 {
        s.advance(i, 1. / 480.);
        assert!(s.n.is_finite() && s.thrust_n.is_finite());
    }
    println!(
        "NOZZLE FAULT mode={:?} surge={} flameout={} F={}",
        s.mode, s.surges, s.flameouts, s.thrust_n
    );
    assert!(s.surges > 0 || s.flameouts > 0);
    assert!(s.thrust_n < healthy * 0.5);
}
fn doublet(hz: usize) -> (f64, f64, f64) {
    let mut s = State::equilibrium(input());
    let mut impulse = 0.;
    let mut fuel = 0.;
    let dt = 1. / hz as f64;
    for k in 0..hz * 40 {
        let t = k as f64 * dt;
        let i = Input {
            throttle: if t < 5. {
                0.88
            } else if t < 20. {
                1.
            } else {
                0.5
            },
            ..input()
        };
        s.advance(i, dt);
        impulse += s.thrust_n * dt;
        fuel += s.fuel_used_kg;
        assert_eq!(s.mode, Mode::Running);
    }
    (s.n, impulse, fuel)
}
#[test]
fn independently_integrated_engine_steps_converge() {
    let reference = doublet(960);
    for hz in [240, 480] {
        let x = doublet(hz);
        println!(
            "ENGINE CONVERGENCE {hz}Hz N={} impulse={} fuel={}",
            x.0, x.1, x.2
        );
        for (a, b) in [(x.0, reference.0), (x.1, reference.1), (x.2, reference.2)] {
            assert!((a - b).abs() / b.abs().max(1e-9) < 0.02);
        }
    }
}
