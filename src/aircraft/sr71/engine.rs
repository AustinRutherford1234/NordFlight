//! J58-class integrated inlet/core/bleed/ejector model and flight-loop adapter.
#[path = "propulsion/config.rs"]
pub mod config;
#[path = "propulsion/gaspath.rs"]
pub mod gaspath;
#[path = "propulsion/inlet.rs"]
pub mod inlet;
#[path = "propulsion/state.rs"]
pub mod state;
#[path = "propulsion/study.rs"]
pub mod study;
#[cfg(test)]
#[path = "propulsion/tests.rs"]
mod tests;
use crate::physics::*;
use bevy::math::DVec3;
use config::{DYN, GAS};
use gaspath::Ambient;
use state::{Input, Mode, State};
#[derive(Clone, Default)]
pub struct Twin {
    pub initialized: bool,
    pub sides: [State; 2],
    pub force_body_n: DVec3,
    pub moment_body_nm: DVec3,
    pub fuel_used_kg: f64,
    pub mean_burner: f64,
    pub invalid_inputs: u64,
}
pub fn axis() -> DVec3 {
    DVec3::new(
        0.,
        -super::config::INCIDENCE.sin(),
        -super::config::INCIDENCE.cos(),
    )
}
pub fn intake_point(i: usize) -> DVec3 {
    DVec3::new(if i == 0 { -4.2 } else { 4.2 }, 0.18, 1.4)
}
pub fn ambient(density: f64, temperature: f64, speed: f64, mach: f64, axial_speed: f64) -> Ambient {
    Ambient {
        density,
        temperature,
        pressure: density * GAS.r * temperature,
        speed,
        mach,
        axial_speed,
    }
}
fn healthy_input(a: Ambient, throttle: f64, alpha: f64, beta: f64) -> Input {
    Input {
        ambient: a,
        alpha,
        beta,
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
/// True SI atmosphere query used by trim; no density-only inference on this path.
pub fn equilibrium(
    throttle: f64,
    rho: f64,
    temperature: f64,
    mach: f64,
    alpha: f64,
    beta: f64,
) -> State {
    let speed = mach * (1.4 * GAS.r * temperature).sqrt();
    let a = ambient(
        rho,
        temperature,
        speed,
        mach,
        speed * (alpha - super::config::INCIDENCE).cos() * beta.cos(),
    );
    State::equilibrium(healthy_input(a, throttle, alpha, beta))
}
/// Legacy display/spec query. Zero means off here; a running engine has finite idle.
pub fn engine(power: f64, rho: f64, mach: f64) -> (f64, f64) {
    if power <= 0. || ![power, rho, mach].iter().all(|v| v.is_finite()) || rho <= 0. || mach < 0. {
        return (0., 0.);
    }
    let t = if rho >= 0.3639 {
        (288.15 * (rho / 1.225).powf(1. / 4.25588)).clamp(180., 330.)
    } else {
        216.65
    };
    let e = equilibrium(power, rho, t, mach, 0., 0.);
    (e.thrust_n * 2., e.thrust_n * 2. * e.afterburner_fraction)
}
pub fn installed(f: &Flight, rho: f64, mach: f64) -> (f64, f64) {
    if f.systems.sr71.propulsion.initialized {
        let p = &f.systems.sr71.propulsion;
        (
            p.sides.iter().map(|e| e.thrust_n).sum(),
            p.sides
                .iter()
                .map(|e| e.thrust_n * e.afterburner_fraction)
                .sum(),
        )
    } else {
        engine(f.spool, rho, mach)
    }
}
pub fn recovery(f: &Flight) -> f64 {
    if f.systems.sr71.propulsion.initialized {
        f.systems
            .sr71
            .propulsion
            .sides
            .iter()
            .map(|e| e.inlet.recovery)
            .sum::<f64>()
            * 0.5
    } else {
        1.
    }
}
pub fn fuel_flow(f: &Flight) -> f64 {
    if !f.engine || (!f.infinite_fuel && f.fuel <= 0.) {
        0.
    } else {
        f.systems
            .sr71
            .propulsion
            .sides
            .iter()
            .map(|e| e.main_fuel_kg_s + e.ab_fuel_kg_s)
            .sum()
    }
}
pub fn initialize(f: &mut Flight) {
    let (t, _, rho, sound) = atmosphere(f.altitude());
    let velocity = f.orientation.conjugate() * (f.velocity - f.wind);
    let speed = velocity.length();
    let a = ambient(rho, t, speed, speed / sound, velocity.dot(axis()));
    let sides = std::array::from_fn(|i| {
        let mut input = healthy_input(a, f.throttle, f.alpha, f.beta);
        input.run = f.engine && f.systems.engine_on[i] && (f.fuel > 0. || f.infinite_fuel);
        input.health = f.systems.engines[i].min(f.damage.engines[i]);
        if input.run {
            State::equilibrium(input)
        } else {
            State::default()
        }
    });
    f.systems.sr71.propulsion = Twin {
        sides,
        initialized: true,
        ..Default::default()
    };
    publish(f);
}
fn publish(f: &mut Flight) {
    let p = &f.systems.sr71.propulsion;
    f.spool = p.sides.iter().map(State::visual_power).sum::<f64>() * 0.5;
    f.systems.sr71.spike = p.sides.iter().map(|e| e.inlet.spike_m).sum::<f64>() * 0.5;
    f.systems.sr71.nozzles = std::array::from_fn(|i| {
        ((p.sides[i].throat_m2 - DYN.nozzle_min_m2) / (DYN.nozzle_max_m2 - DYN.nozzle_min_m2))
            .clamp(0., 1.)
    });
    f.thrust = p.sides.iter().map(|e| e.thrust_n).sum();
}
/// Advances once, consumes fuel once, returns averaged physical attachment loads.
pub fn propulsion(f: &mut Flight, dt: f64, rho: f64, mach: f64) -> (DVec3, DVec3, f64) {
    if !dt.is_finite() || dt <= 0. || !rho.is_finite() || rho < 0. || !mach.is_finite() {
        f.systems.sr71.propulsion.invalid_inputs += 1;
        return (DVec3::ZERO, DVec3::ZERO, 0.);
    }
    if !f.systems.sr71.propulsion.initialized {
        if f.landed {
            f.systems.sr71.propulsion.initialized = true;
        } else {
            initialize(f);
        }
    }
    let body = f.orientation.conjugate() * (f.velocity - f.wind);
    let temperature = f.temperature.max(150.);
    let sound = (1.4 * GAS.r * temperature).sqrt();
    let cg = DVec3::Z
        * ((super::mass::cg_fraction(f) - super::config::MASS.moment_reference_mac)
            * super::config::CHORD);
    let positions = f.aircraft.engine_positions();
    let velocity: [DVec3; 2] = std::array::from_fn(|i| body + f.rates.cross(intake_point(i) - cg));
    let authority = ((f.damage.hydraulics - 0.12) / 0.88).clamp(0., 1.);
    let count = (dt / DYN.max_substep_s).ceil().max(1.) as usize;
    let h = dt / count as f64;
    let cart = f.landed
        && f.velocity
            .distance(crate::carrier::ground_velocity(f, f.position))
            < 3.
        && f.battery
        && f.apu;
    let mut force = DVec3::ZERO;
    let mut moment = DVec3::ZERO;
    let mut fuel_burned = 0.;
    let mut remaining = if f.infinite_fuel {
        f64::INFINITY
    } else {
        f.fuel.max(0.)
    };
    for _ in 0..count {
        let mut used = 0.;
        let mut forces = [DVec3::ZERO; 2];
        let mut moments = [DVec3::ZERO; 2];
        let mut passive = [DVec3::ZERO; 2];
        for i in 0..2 {
            let v = velocity[i];
            let speed = v.length();
            let a = ambient(rho, temperature, speed, speed / sound, v.dot(axis()));
            let input = Input {
                ambient: a,
                alpha: if speed > 1e-6 { (-v.y).atan2(-v.z) } else { 0. },
                beta: if speed > 1e-6 {
                    (v.x / speed).clamp(-1., 1.).asin()
                } else {
                    0.
                },
                throttle: f.throttle.clamp(0., 1.),
                run: f.engine && f.systems.engine_on[i],
                ignition: f.battery,
                ground_starter: cart,
                hydraulic: authority,
                health: f.systems.engines[i]
                    .min(f.damage.engines[i])
                    .min(f.damage.engine)
                    .clamp(0., 1.),
                nozzle_health: f.damage.nozzles[i],
                fuel_available: remaining > 0.,
            };
            let e = &mut f.systems.sr71.propulsion.sides[i];
            e.advance(input, h);
            used += e.fuel_used_kg;
            let thrust = axis() * e.thrust_n;
            let capture_side = -(v - axis() * v.dot(axis()))
                * (e.output.inlet.mass_flow_kg_s + e.output.secondary_air_kg_s)
                * if e.mode == Mode::Running { 1. } else { 0. };
            passive[i] = -v.normalize_or_zero() * e.drag_n;
            let drag = passive[i] + capture_side;
            forces[i] = thrust + drag;
            moments[i] = (positions[i] - cg).cross(thrust) + (intake_point(i) - cg).cross(drag);
        }
        let fraction = if used > remaining {
            remaining / used
        } else {
            1.
        };
        fuel_burned += used.min(remaining);
        remaining = (remaining - used).max(0.);
        for i in 0..2 {
            force += ((forces[i] - passive[i]) * fraction + passive[i]) * h / dt;
            let passive_moment = (intake_point(i) - cg).cross(passive[i]);
            moment += ((moments[i] - passive_moment) * fraction + passive_moment) * h / dt;
            if fraction < 1. {
                let e = &mut f.systems.sr71.propulsion.sides[i];
                e.mode = Mode::Off;
                e.thrust_n = 0.;
                e.main_fuel_kg_s = 0.;
                e.ab_fuel_kg_s = 0.;
                e.afterburner_lit = false;
                e.afterburner_fraction = 0.;
            }
        }
    }
    if !f.infinite_fuel {
        f.fuel = (remaining - dt * f.damage.fuel_leak).max(0.);
    }
    let p = &mut f.systems.sr71.propulsion;
    p.force_body_n = force;
    p.moment_body_nm = moment;
    p.fuel_used_kg = fuel_burned;
    p.mean_burner = p.sides.iter().map(|e| e.afterburner_fraction).sum::<f64>() * 0.5;
    publish(f);
    (force, moment, f.systems.sr71.propulsion.mean_burner)
}

/// Healthy steady attachment wrench, shared by trim and spawn initialization.
pub fn equilibrium_loads(
    throttle: f64,
    rho: f64,
    temp: f64,
    mach: f64,
    alpha: f64,
    beta: f64,
    cg_fraction: f64,
) -> (DVec3, DVec3) {
    let e = equilibrium(throttle, rho, temp, mach, alpha, beta);
    let speed = mach * (1.4 * GAS.r * temp).sqrt();
    let v = DVec3::new(
        speed * beta.sin(),
        -speed * alpha.sin() * beta.cos(),
        -speed * alpha.cos() * beta.cos(),
    );
    let cg = DVec3::Z
        * ((cg_fraction - super::config::MASS.moment_reference_mac) * super::config::CHORD);
    let axis = axis();
    let mut force = DVec3::ZERO;
    let mut moment = DVec3::ZERO;
    for i in 0..2 {
        let thrust = axis * e.thrust_n;
        let side = -(v - axis * v.dot(axis))
            * (e.output.inlet.mass_flow_kg_s + e.output.secondary_air_kg_s);
        let spillage = -v.normalize_or_zero() * e.inlet.drag_n;
        let at = DVec3::new(
            if i == 0 {
                -super::config::MASS.engine_lateral_m
            } else {
                super::config::MASS.engine_lateral_m
            },
            0.,
            super::config::MASS.engine_aft_m,
        );
        force += thrust + side + spillage;
        moment += (at - cg).cross(thrust) + (intake_point(i) - cg).cross(side + spillage);
    }
    (force, moment)
}
