use super::{
    aero::{induced, steady},
    config::*,
    controls::{control_coefficients, targets},
    engine::engine,
};
use crate::{
    aerodynamics::{Conditions, FlowState},
    physics::*,
};
use bevy::math::{DMat3, DVec3};
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct Trim {
    pub alpha: f64,
    pub command: f64,
    pub throttle: f64,
    pub residual: f64,
}
pub(super) fn condition(alpha: f64, alt: f64, speed: f64) -> Conditions {
    let (t, _, rho, sound) = atmosphere(alt);
    Conditions {
        alpha,
        beta: 0.,
        mach: speed / sound,
        speed,
        density: rho,
        temperature: t,
        flaps: 0.,
        gear: false,
        airbrake: false,
        height: alt,
        rates: DVec3::ZERO,
    }
}
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub enum TrimFailure {
    InvalidInput,
    InsufficientSpeed,
    SingularJacobian,
    NoDescent,
    IterationLimit,
}
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct TrimReport {
    pub solution: Option<Trim>,
    pub failure: Option<TrimFailure>,
    pub iterations: usize,
    pub residual: [f64; 3],
}
#[cfg(test)]
pub fn trim(mass: f64, fuel: f64, alt: f64, speed: f64) -> Option<Trim> {
    solve(mass, fuel, alt, speed).solution
}
pub fn solve(mass: f64, fuel: f64, alt: f64, speed: f64) -> TrimReport {
    let failed = |why, iterations, residual: DVec3| TrimReport {
        solution: None,
        failure: Some(why),
        iterations,
        residual: residual.to_array(),
    };
    if ![mass, fuel, alt, speed].iter().all(|v| v.is_finite())
        || mass <= 0.
        || fuel < 0.
        || mass < fuel
        || alt < 0.
    {
        return failed(TrimFailure::InvalidInput, 0, DVec3::ZERO);
    }
    if speed < SOLVER.minimum_trim_speed_m_s {
        return failed(TrimFailure::InsufficientSpeed, 0, DVec3::ZERO);
    }
    let mut x = DVec3::from_array(REGIMES.trim_initial);
    let c = condition(REGIMES.trim_initial[0], alt, speed);
    let qs = 0.5 * c.density * speed * speed * AREA;
    let w = mass * (MU / (RADIUS + alt).powi(2) - speed * speed / (RADIUS + alt));
    if !qs.is_finite() || qs <= 0. || !w.is_finite() || w <= 0. {
        return failed(TrimFailure::InvalidInput, 0, DVec3::ZERO);
    }
    let residual = |x: DVec3| {
        let mut c = c;
        c.alpha = x.x;
        let mut a = steady(c);
        a.pitch += ((MASS.zero_fuel_cg_mac * EMPTY + MASS.fuel_cg_mac * fuel) / (EMPTY + fuel)
            - MASS.moment_reference_mac)
            * a.lift;
        let controls = control_coefficients(
            targets(DVec3::new(x.y, 0., 0.)),
            [0.; 2],
            c.mach,
            a.attached,
        );
        let old = a.lift;
        a.lift += controls.lift;
        a.drag += controls.drag + induced(a.lift, a.attached, 0., alt, c.mach)
            - induced(old, a.attached, 0., alt, c.mach);
        a.pitch += controls.pitch;
        let b = DVec3::new(0., -speed * x.x.sin(), -speed * x.x.cos());
        let thrust = engine(x.z, c.density, c.mach).0;
        let force = crate::aerodynamics::force(DVec3::X, b, qs, a)
            + DVec3::new(0., -INCIDENCE.sin(), -INCIDENCE.cos()) * thrust;
        DVec3::new(
            (force.y - w * x.x.cos()) / w,
            (force.z + w * x.x.sin()) / w,
            a.pitch + thrust * MASS.engine_aft_m * INCIDENCE.sin() / (qs * CHORD),
        )
    };
    for iteration in 0..SOLVER.iterations {
        let r = residual(x);
        if r.length() < SOLVER.residual_tolerance {
            return TrimReport {
                solution: Some(Trim {
                    alpha: x.x,
                    command: x.y,
                    throttle: x.z,
                    residual: r.length(),
                }),
                failure: None,
                iterations: iteration,
                residual: r.to_array(),
            };
        }
        let h = SOLVER.difference_step;
        let jac = DMat3::from_cols(
            (residual(x + DVec3::X * h) - residual(x - DVec3::X * h)) / (2. * h),
            (residual(x + DVec3::Y * h) - residual(x - DVec3::Y * h)) / (2. * h),
            (residual(x + DVec3::Z * h) - residual(x - DVec3::Z * h)) / (2. * h),
        );
        if !jac.is_finite() || jac.determinant().abs() < SOLVER.determinant_floor {
            return failed(TrimFailure::SingularJacobian, iteration, r);
        }
        let delta = (jac.inverse() * r).clamp(
            DVec3::splat(-SOLVER.max_newton_step),
            DVec3::splat(SOLVER.max_newton_step),
        );
        let mut scale = 1.;
        let mut next = None;
        for _ in 0..REGIMES.backtracking_steps {
            let candidate = (x - delta * scale).clamp(
                DVec3::from_array(REGIMES.trim_min),
                DVec3::from_array(REGIMES.trim_max),
            );
            if residual(candidate).length() < r.length() {
                next = Some(candidate);
                break;
            }
            scale *= REGIMES.backtracking_ratio;
        }
        match next {
            Some(candidate) => x = candidate,
            None => return failed(TrimFailure::NoDescent, iteration, r),
        }
    }
    failed(TrimFailure::IterationLimit, SOLVER.iterations, residual(x))
}
pub fn initialize(f: &mut Flight) {
    f.autopilot = false;
    f.flaps = 0.;
    f.airbrake = false;
    if !f.landed {
        let report = solve(f.mass(), f.fuel, f.altitude(), f.airspeed);
        f.systems.sr71.diagnostics.trim = Some(report);
        if let Some(t) = report.solution {
            let gamma = (f.velocity.dot(f.up()) / f.velocity.length().max(1.))
                .clamp(-1., 1.)
                .asin();
            f.orientation *= bevy::math::DQuat::from_rotation_x(t.alpha + gamma - f.pitch());
            f.surface = DVec3::new(t.command, 0., 0.);
            f.systems.sr71.elevons = targets(f.surface);
            f.throttle = t.throttle;
            f.spool = t.throttle;
            f.alpha = t.alpha;
            f.flow = FlowState {
                lag_alpha: t.alpha - INCIDENCE,
                ..Default::default()
            };
        }
    }
    f.systems.sr71.spike = ENGINE.spike_travel_m
        * ((f.mach - ENGINE.spike_start_mach) / ENGINE.spike_mach_interval).clamp(0., 1.);
}
