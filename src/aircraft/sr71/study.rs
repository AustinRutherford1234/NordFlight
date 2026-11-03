//! Bounded offline experiments. No rendering, real-time delays, or live-session control.
use super::config::*;
use crate::physics::*;
use bevy::math::DVec3;
use std::{
    io::{self, BufWriter, Write},
    path::Path,
};
/// Reproducible study setup shared by automated tests and the diagnostic command.
pub fn airborne(alt: f64, speed: f64) -> Flight {
    let mut f = Flight::approach();
    let up = f.up();
    f.position = up * (planet_radius() + alt);
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
pub fn run(path: &Path) -> io::Result<()> {
    config_check()?;
    std::fs::create_dir_all(path)?;
    std::fs::write(
        path.join("parameters.json"),
        serde_json::to_vec_pretty(&super::config::register())?,
    )?;
    super::database::export(&path.join("coefficient_database.csv"))?;
    let mut trace = BufWriter::new(std::fs::File::create(path.join("trace.csv"))?);
    writeln!(
        trace,
        "scenario,hz,t_s,altitude_m,mach,alpha_rad,beta_rad,pitch_rate,yaw_rate,roll_rate,g,fuel_kg,integrity,drag_n,thrust_n,closure_nm,flags"
    )?;
    let mut snapshots = BufWriter::new(std::fs::File::create(path.join("snapshots.jsonl"))?);
    let mut summaries = Vec::with_capacity(21);
    let env = Environment {
        wind_enabled: false,
        ..Default::default()
    };
    for hz in [60, 120, 240] {
        for scenario in [
            "neutral",
            "doublet",
            "left_engine_out",
            "hydraulic_loss",
            "jammed_elevon",
            "detached_elevons",
            "fuel_exhaustion",
        ] {
            let mut f = airborne(23000., 900.);
            let initial_alt = f.altitude();
            let mut peak_closure: f64 = 0.;
            let mut peak_rate: f64 = 0.;
            let mut next_trace = 0.;
            let mut next_detail = 0.;
            let mut injection = false;
            let start = std::time::Instant::now();
            let mut count = 0;
            for step in 0..hz * 60 {
                let t = step as f64 / hz as f64;
                if t >= 10. && !injection {
                    injection = true;
                    match scenario {
                        "left_engine_out" => f.systems.engine_on[0] = false,
                        "hydraulic_loss" => f.damage.hydraulics = 0.,
                        "jammed_elevon" => {
                            f.damage.surface_state[0] =
                                crate::combat::damage::SurfaceState::Jammed(0.15)
                        }
                        "detached_elevons" => {
                            for i in [0, 4, 2, 6] {
                                f.damage.surface_state[i] =
                                    crate::combat::damage::SurfaceState::Detached;
                            }
                        }
                        "fuel_exhaustion" => f.fuel = 0.,
                        _ => {}
                    }
                }
                f.controls = if scenario == "doublet" && (10.0..12.0).contains(&t) {
                    DVec3::new(0.15, 0., 0.1)
                } else {
                    DVec3::ZERO
                };
                simulate(&mut f, &env, 1. / hz as f64);
                count += 1;
                let d = f.systems.sr71.diagnostics.latest;
                peak_closure = peak_closure.max(d.moment_closure_error_nm);
                peak_rate = peak_rate.max(f.rates.length());
                if t + 1e-8 >= next_trace {
                    writeln!(
                        trace,
                        "{scenario},{hz},{t},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                        f.altitude(),
                        f.mach,
                        f.alpha,
                        f.beta,
                        f.rates.x,
                        f.rates.y,
                        f.rates.z,
                        f.g_load,
                        f.fuel,
                        f.integrity,
                        d.drag_components_n[5],
                        DVec3::from_array(d.engine_force_body_n).length(),
                        d.moment_closure_error_nm,
                        d.flags
                    )?;
                    if t + 1e-8 >= next_detail {
                        serde_json::to_writer(
                            &mut snapshots,
                            &serde_json::json!({"scenario":scenario,"hz":hz,"t_s":t,"snapshot":d}),
                        )?;
                        writeln!(snapshots)?;
                        next_detail += 1.;
                    }
                    next_trace += SOLVER.trace_period_s;
                }
                if f.crashed || !f.position.is_finite() || !f.rates.is_finite() {
                    break;
                }
            }
            summaries.push(serde_json::json!({"scenario":scenario,"hz":hz,"altitude_change_m":f.altitude()-initial_alt,"final_mach":f.mach,"crashed":f.crashed,"finite":f.position.is_finite()&&f.rates.is_finite(),"integrity":f.integrity,"peak_rate_rad_s":peak_rate,"peak_moment_closure_nm":peak_closure,"sticky_flags":f.systems.sr71.diagnostics.sticky_flags,"repaired_flow_states":f.systems.sr71.diagnostics.repaired_flow_states,"mean_step_us":start.elapsed().as_secs_f64()*1e6/count as f64}));
        }
    }
    airframe_study(path)?;
    trace.flush()?;
    snapshots.flush()?;
    std::fs::write(
        path.join("summary.json"),
        serde_json::to_vec_pretty(&summaries)?,
    )?;
    let mut trims = Vec::new();
    for mass_fuel in [8000., 20000., 30000.] {
        for (alt, speed) in [
            (1000., 120.),
            (10000., 240.),
            (18000., 600.),
            (23000., 900.),
            (25000., 950.),
            (23000., 20.),
            (25000., 200.),
        ] {
            trims.push(serde_json::json!({"altitude_m":alt,"speed_m_s":speed,"fuel_kg":mass_fuel,"report":super::trim::solve(EMPTY+mass_fuel,mass_fuel,alt,speed)}));
        }
    }
    std::fs::write(path.join("trim.json"), serde_json::to_vec_pretty(&trims)?)?;
    println!(
        "SR-71 headless study written to {} (21 bounded flight cases; no renderer)",
        path.display()
    );
    Ok(())
}
fn config_check() -> io::Result<()> {
    super::config::validate().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Forced-input experiments bypass the FBW and use genuinely different internal steps.
pub fn airframe_study(path: &Path) -> io::Result<()> {
    use super::{controls::targets, landing, plant, trim::condition};
    let mut csv = BufWriter::new(std::fs::File::create(path.join("airframe.csv"))?);
    writeln!(
        csv,
        "case,hz,t_s,fy_n,mx_nm,left_bend_m,right_bend_m,left_twist_rad,root_bending_nm,root_stress_pa,root_strain,attachment,vortex,elastic_energy_j"
    )?;
    let mut results = Vec::new();
    for hz in [240, 480, 960] {
        for case in [
            "free_decay",
            "alpha_doublet",
            "landing_flare",
            "asymmetric_load",
        ] {
            let mut c = condition(0.1, 6000., 250.);
            let mut damage = AircraftDamage::default();
            let mut b = DVec3::new(0., -250. * c.alpha.sin(), -250. * c.alpha.cos());
            let mut state = plant::steady(
                c,
                b,
                0.22,
                &damage,
                targets(DVec3::ZERO),
                [0.; 2],
                landing::Surface::free(),
                true,
            );
            if case == "free_decay" {
                state = plant::State::default();
                state.initialized = true;
                state.structure.modes[0].bend_m = 0.1;
                state.structure.modes[0].twist_rad = 0.01;
            }
            let mut peak_force: f64 = 0.;
            let mut peak_bend: f64 = 0.;
            let mut peak_stress: f64 = 0.;
            let mut peak_strain: f64 = 0.;
            let mut next_sample = 0.;
            let mut initial_energy = 0.;
            for step in 0..hz * 8 {
                let t = (step + 1) as f64 / hz as f64;
                let mut plane = landing::Surface::free();
                let mut angular = DVec3::ZERO;
                let mut control = DVec3::ZERO;
                let specific = DVec3::Y * 9.8;
                c = condition(0.1, 6000., 250.);
                if case == "alpha_doublet" {
                    let freq = std::f64::consts::TAU * 0.7;
                    c.alpha = 0.1 + 0.025 * (freq * t).sin();
                    c.rates.x = 0.025 * freq * (freq * t).cos();
                    angular.x = -0.025 * freq * freq * (freq * t).sin();
                    control.z = 0.02;
                }
                if case == "landing_flare" {
                    c = condition(0.10 + 0.08 * (t / 6.).clamp(0., 1.), 1000., 120.);
                    plane = landing::Surface {
                        distance_m: (20. - 3. * t).max(1.),
                        normal_body: DVec3::Y,
                        relative_velocity_body: DVec3::new(0., -3., -120.),
                    };
                    state.gear_extension = 1.;
                }
                if case == "asymmetric_load" {
                    c.beta = 0.06;
                    control.z = if t > 2. && t < 4. { 0.1 } else { 0. };
                    if t > 4. {
                        damage.wings[0] = 0.6;
                    }
                }
                if case == "free_decay" {
                    c = condition(0., 6000., 0.);
                    c.density = 0.;
                }
                b = DVec3::new(
                    c.speed * c.beta.sin(),
                    -c.speed * c.alpha.sin() * c.beta.cos(),
                    -c.speed * c.alpha.cos() * c.beta.cos(),
                );
                plant::prepare(
                    &mut state,
                    c,
                    b,
                    0.22,
                    plane,
                    &damage,
                    if case == "free_decay" {
                        DVec3::ZERO
                    } else {
                        specific
                    },
                    angular,
                    1. / hz as f64,
                    false,
                );
                let loads = plant::control_loads(&state, &damage, targets(control), [0.; 2]);
                plant::finish(&mut state, &loads);
                if step == 0 {
                    initial_energy = state.structure.energy_j;
                }
                peak_force = peak_force.max(state.total.force.length());
                peak_bend = peak_bend.max(state.structure.modes[0].bend_m.abs());
                peak_stress =
                    peak_stress.max(state.structure.sections[0][0].normal_stress_pa.abs());
                peak_strain = peak_strain.max(state.structure.sections[0][0].normal_strain.abs());
                if t >= next_sample {
                    let s = state.structure;
                    writeln!(
                        csv,
                        "{case},{hz},{t},{},{},{},{},{},{},{},{},{},{},{}",
                        state.total.force.y,
                        state.total.moment.x,
                        s.modes[0].bend_m,
                        s.modes[1].bend_m,
                        s.modes[0].twist_rad,
                        s.sections[0][0].bending_nm,
                        s.sections[0][0].normal_stress_pa,
                        s.sections[0][0].normal_strain,
                        state.local[0].flow.attached,
                        state.local[0].flow.vortex,
                        s.energy_j
                    )?;
                    next_sample += 0.02;
                }
            }
            results.push(serde_json::json!({"case":case,"hz":hz,"finite":state.total.force.is_finite()&&state.total.moment.is_finite()&&state.structure.valid,"peak_force_n":peak_force,"peak_bend_m":peak_bend,"peak_root_stress_pa":peak_stress,"peak_root_strain":peak_strain,"final_energy_j":state.structure.energy_j,"initial_energy_j":initial_energy}));
        }
    }
    csv.flush()?;
    std::fs::write(
        path.join("airframe_summary.json"),
        serde_json::to_vec_pretty(&results)?,
    )?;
    Ok(())
}
