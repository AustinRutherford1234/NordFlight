//! Deterministic component/transient evidence, independent of Bevy rendering.
use super::{config::*, gaspath::*, state::*};
use serde::Serialize;
use std::{
    io::{self, Write},
    path::Path,
    time::Instant,
};
pub fn conditions(alt: f64, mach: f64, throttle: f64) -> Input {
    let (t, _, rho, sound) = crate::physics::atmosphere(alt);
    Input {
        ambient: Ambient {
            density: rho,
            temperature: t,
            pressure: rho * GAS.r * t,
            speed: mach * sound,
            axial_speed: mach * sound,
            mach,
        },
        alpha: 0.,
        beta: 0.,
        throttle,
        run: true,
        ignition: true,
        ground_starter: false,
        hydraulic: 1.,
        health: 1.,
        nozzle_health: 1.,
        fuel_available: true,
    }
}
#[derive(Serialize)]
pub struct Result {
    pub case: &'static str,
    pub hz: usize,
    pub final_state: State,
    pub fuel_kg: f64,
    pub impulse_ns: f64,
    pub peak_thrust_n: f64,
    pub minimum_margin: f64,
    pub max_nozzle_relative_residual: f64,
}
pub fn transient(
    case: &'static str,
    hz: usize,
    mut trace: Option<&mut dyn Write>,
) -> io::Result<Result> {
    let cruise = matches!(
        case,
        "cruise" | "inlet_unstart" | "bleed_jam" | "pump_failure" | "windmill_restart"
    );
    let mut input = if cruise {
        conditions(23000., 3., 0.96)
    } else {
        conditions(0., 0., 0.88)
    };
    let mut s = if matches!(case, "ground_start" | "failed_ignition") {
        State::default()
    } else {
        State::equilibrium(input)
    };
    let mut fuel = 0.;
    let mut impulse = 0.;
    let mut peak: f64 = 0.;
    let mut margin: f64 = 1.;
    let mut residual: f64 = 0.;
    let dt = 1. / hz as f64;
    for k in 0..hz * 60 {
        let time = k as f64 * dt;
        match case {
            "ground_start" | "failed_ignition" => {
                input.ground_starter = true;
                input.throttle = 0.;
                s.faults.ignition_failed = case == "failed_ignition";
            }
            "throttle_step" => {
                input.throttle = if time < 5. {
                    0.88
                } else if time < 25. {
                    1.
                } else {
                    0.5
                }
            }
            "inlet_unstart" => {
                input.beta = if (5.0..7.0).contains(&time) { 0.3 } else { 0. };
                s.faults.spike_jammed = if (5.0..7.0).contains(&time) {
                    Some(0.)
                } else {
                    None
                };
            }
            "nozzle_jam" => s.faults.nozzle_jammed_m2 = if time > 5. { Some(0.12) } else { None },
            "bleed_jam" => s.faults.bleed_jammed = if time > 5. { Some(0.) } else { None },
            "pump_failure" => s.faults.fuel_pump_scale = if time > 5. { 0. } else { 1. },
            "fuel_exhaustion" => input.fuel_available = time < 5.,
            "windmill_restart" => input.run = !(5.0..15.0).contains(&time),
            _ => (),
        }
        s.advance(input, dt);
        fuel += s.fuel_used_kg;
        impulse += s.thrust_n * dt;
        peak = peak.max(s.thrust_n);
        margin = margin.min(s.surge_margin);
        if s.mode == Mode::Running {
            residual = residual.max(
                s.output.nozzle_matching_residual_kg_s.abs()
                    / s.output.nozzle.mass_flow_kg_s.max(1.),
            );
        }
        if let Some(w) = trace.as_deref_mut() {
            if k % (hz / 10).max(1) == 0 {
                writeln!(
                    w,
                    "{case},{hz},{time},{:?},{},{},{},{},{},{},{},{:?},{},{},{},{}",
                    s.mode,
                    s.rpm(),
                    s.thrust_n,
                    s.drag_n,
                    s.main_fuel_kg_s,
                    s.ab_fuel_kg_s,
                    s.bleed,
                    s.throat_m2,
                    s.inlet.mode,
                    s.inlet.recovery,
                    s.surge_margin,
                    s.surges,
                    s.flameouts
                )?;
            }
        }
    }
    Ok(Result {
        case,
        hz,
        final_state: s,
        fuel_kg: fuel,
        impulse_ns: impulse,
        peak_thrust_n: peak,
        minimum_margin: margin,
        max_nozzle_relative_residual: residual,
    })
}
pub fn run(path: &Path) -> io::Result<()> {
    std::fs::create_dir_all(path)?;
    std::fs::write(
        path.join("parameters.json"),
        serde_json::to_vec_pretty(&super::config::register())?,
    )?;
    let mut rows = io::BufWriter::new(std::fs::File::create(path.join("steady.csv"))?);
    writeln!(
        rows,
        "alt_m,mach,throttle,rpm,net_thrust_n,main_fuel_kg_s,ab_fuel_kg_s,t4_k,t7_k,compressor_pr,shaft_residual_w,nozzle_relative_residual,equilibrium_feasible"
    )?;
    for alt in [0., 10000., 20000., 23000., 25000.] {
        for mach in [0., 0.6, 1., 2., 3., 3.2] {
            for throttle in [0., 0.5, 0.88, 0.94, 1.] {
                let s = State::equilibrium(conditions(alt, mach, throttle));
                let o = s.output;
                writeln!(
                    rows,
                    "{alt},{mach},{throttle},{},{},{},{},{},{},{},{},{},{}",
                    s.rpm(),
                    s.thrust_n,
                    o.main_fuel_kg_s,
                    o.ab_fuel_kg_s,
                    o.combustor.total_temperature_k,
                    o.afterburner.total_temperature_k,
                    o.compressor_ratio,
                    o.shaft_power_w,
                    o.nozzle_matching_residual_kg_s / o.nozzle.mass_flow_kg_s.max(1.),
                    o.shaft_power_w.abs() / o.compressor_power_w.max(1.) < 0.001
                        && o.nozzle_capacity_ratio > 0.99
                )?;
            }
        }
    }
    rows.flush()?;
    let mut trace = io::BufWriter::new(std::fs::File::create(path.join("transients.csv"))?);
    writeln!(
        trace,
        "case,hz,time_s,mode,rpm,thrust_n,drag_n,main_fuel_kg_s,ab_fuel_kg_s,bleed_fraction,nozzle_throat_m2,inlet_mode,recovery,surge_margin,surges,flameouts"
    )?;
    let mut results = Vec::new();
    for case in [
        "ground_start",
        "failed_ignition",
        "throttle_step",
        "cruise",
        "inlet_unstart",
        "nozzle_jam",
        "bleed_jam",
        "pump_failure",
        "fuel_exhaustion",
        "windmill_restart",
    ] {
        for hz in [240, 480, 960] {
            results.push(transient(case, hz, Some(&mut trace))?);
        }
    }
    trace.flush()?;
    std::fs::write(
        path.join("transients.json"),
        serde_json::to_vec_pretty(&results)?,
    )?;
    // Four engine steps equal a pair of engines integrated through one 240 Hz aircraft tick.
    let input = conditions(23000., 3., 0.96);
    let mut sides = [State::equilibrium(input); 2];
    let mut times = Vec::with_capacity(6000);
    for k in 0..6500 {
        let now = Instant::now();
        for _ in 0..2 {
            for s in &mut sides {
                s.advance(std::hint::black_box(input), 1. / 480.);
                std::hint::black_box(s.thrust_n);
            }
        }
        if k >= 500 {
            times.push(now.elapsed().as_secs_f64() * 1e6);
        }
    }
    times.sort_by(f64::total_cmp);
    let perf = serde_json::json!({"samples":times.len(),"unit":"microseconds per twin-engine 240 Hz tick (four 480 Hz component steps)","mean":times.iter().sum::<f64>()/times.len() as f64,"p50":times[times.len()/2],"p95":times[times.len()*95/100],"p99":times[times.len()*99/100],"rendering_included":false,"steady_points":150,"transient_cases":results.len()});
    std::fs::write(
        path.join("performance.json"),
        serde_json::to_vec_pretty(&perf)?,
    )?;
    println!("SR-71 engine study written to {}\n{perf}", path.display());
    Ok(())
}
