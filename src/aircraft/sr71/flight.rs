//! SR-71 reduced-order aerodynamic/propulsion plant. NASA TP-2002-210718 baseline
//! graphs constrain small-disturbance slopes; nonlinear/drag/engine maps are estimates.
use crate::{
    aerodynamics::{Coefficients, Conditions, FlowState},
    control_surfaces::{Loads, SurfaceLoad},
    physics::*,
};
use bevy::math::{DMat3, DVec3};
use std::f64::consts::PI;
pub const AREA: f64 = 1605. * 0.09290304;
pub const SPAN: f64 = 56.7 * 0.3048;
pub const CHORD: f64 = 37.7 * 0.3048;
pub const EMPTY: f64 = 60728. * 0.45359237;
pub const FUEL: f64 = 80000. * 0.45359237;
pub const INCIDENCE: f64 = 1.2 * PI / 180.;
const DEG: f64 = 180. / PI;
const MACH: [f64; 9] = [0., 0.6, 0.9, 1.1, 1.6, 2., 2.5, 3., 3.5];
// Approximate visual readings of BASELINE figures 15-18,36-40, not raw test data.
const CNA: [f64; 9] = [
    0.042, 0.042, 0.049, 0.044, 0.033, 0.027, 0.0255, 0.025, 0.023,
]; // /deg
const CMA: [f64; 9] = [
    0.001, 0.001, 0.0002, -0.0027, -0.0025, -0.0012, -0.0010, -0.0005, -0.0004,
];
const CMB: [f64; 9] = [
    0.003, 0.003, 0.004, 0.010, 0.005, 0.003, 0.002, 0.001, 0.001,
];
const CNB: [f64; 9] = [
    -0.015, -0.015, -0.025, -0.020, -0.01, 0.004, 0.02, 0.03, 0.03,
];
const CMDE: [f64; 9] = [
    -0.005, -0.005, -0.0044, -0.0021, -0.00165, -0.0013, -0.0011, -0.001, -0.0009,
];
const CNDE: [f64; 9] = [
    0.004, 0.004, 0.0015, -0.0004, 0., 0.0003, 0.0003, 0.0003, 0.0003,
];
const CYB: [f64; 9] = [
    -0.007, -0.007, -0.008, -0.0085, -0.006, -0.0065, -0.0058, -0.006, -0.006,
];
const CLB: [f64; 9] = [
    -0.0009, -0.0009, -0.0007, -0.0011, -0.001, -0.0009, -0.0006, -0.0004, -0.0003,
];
const CNBETA: [f64; 9] = [
    0.0014, 0.0014, 0.0017, 0.0019, 0.0009, 0.001, 0.0006, 0.0005, 0.0004,
];
const CLDA: [f64; 9] = [
    0.0018, 0.0018, 0.0015, 0.0007, 0.0005, 0.00045, 0.0004, 0.00035, 0.0003,
];
const CNDR: [f64; 9] = [
    -0.0011, -0.0011, -0.0015, -0.0014, -0.0009, -0.0007, -0.0005, -0.00035, -0.0003,
];
const CYDR: [f64; 9] = [
    0.003, 0.003, 0.0038, 0.0035, 0.0024, 0.002, 0.0017, 0.0013, 0.0011,
];
pub fn smooth(a: f64, b: f64, x: f64) -> f64 {
    super::smooth((x - a) / (b - a))
}
pub fn table(axis: &[f64], values: &[f64], x: f64) -> f64 {
    let x = x.clamp(axis[0], axis[axis.len() - 1]);
    let i = axis
        .windows(2)
        .position(|a| x <= a[1])
        .unwrap_or(axis.len() - 2);
    let t = (x - axis[i]) / (axis[i + 1] - axis[i]);
    values[i] + (values[i + 1] - values[i]) * t
}
fn tab(values: &[f64; 9], mach: f64) -> f64 {
    table(&MACH, values, mach)
}
pub fn inertia(f: &Flight) -> DMat3 {
    let k = 1.3558179483;
    let fuel = f.fuel.max(0.);
    DMat3::from_cols(
        DVec3::new(954850. * k + fuel * 22., 0., 0.),
        DVec3::new(0., 1172039. * k + fuel * 31., -19200. * k),
        DVec3::new(0., -19200. * k, 220660. * k + fuel * 9.),
    )
}
pub fn cg_fraction(f: &Flight) -> f64 {
    (0.201 * EMPTY + 0.235 * f.fuel.max(0.)) / (EMPTY + f.fuel.max(0.))
}
pub fn induced(lift: f64, attached: f64, beta: f64, height: f64, mach: f64) -> f64 {
    let efficiency = (0.79 - 0.15 * (1. - attached) - 0.1 * beta.abs()).clamp(0.45, 0.82);
    let ground = (16. * height.max(0.) / SPAN).powi(2);
    let g = (ground / (1. + ground)).clamp(0.35, 1.);
    lift * lift / (PI * (SPAN * SPAN / AREA) * efficiency)
        * g
        * (1. + 0.2 * smooth(0.95, 2.5, mach))
}
fn attached_target(alpha: f64, mach: f64, history: f64) -> f64 {
    let onset = 0.43 - 0.10 * smooth(0.8, 2.5, mach) - 0.045 * (1. - history);
    1. - smooth(onset - 0.04, onset + 0.12, alpha.abs())
}
pub fn evaluate(c: Conditions, state: &mut FlowState, dt: f64) -> Coefficients {
    let m = c.mach.max(0.);
    let v = c.speed.max(0.01);
    let a = (c.alpha - INCIDENCE)
        .sin()
        .atan2((c.alpha - INCIDENCE).cos());
    let tau = (CHORD / v.max(25.) * 0.9).clamp(0.035, 0.6);
    let decay = 1. - (-dt.max(0.) / tau).exp();
    let old = state.lag_alpha;
    state.lag_alpha += (a - state.lag_alpha) * decay;
    state.lag_beta += (c.beta - state.lag_beta) * (1. - (-dt.max(0.) / (tau * 1.4)).exp());
    for (i, side) in [-1., 1.].into_iter().enumerate() {
        let local = state.lag_alpha - side * c.rates.z * SPAN / (4. * v.max(25.));
        let target = attached_target(local, m, state.wing_attached[i]);
        let t = tau
            * if target < state.wing_attached[i] {
                1.1
            } else {
                3.
            };
        state.wing_attached[i] +=
            (target - state.wing_attached[i]) * (1. - (-dt.max(0.) / t).exp());
    }
    state.attached = (state.wing_attached[0] + state.wing_attached[1]) * 0.5;
    let adot = if dt > 1e-9 {
        (state.lag_alpha - old) / dt
    } else {
        0.
    };
    let vortex_target =
        (adot * CHORD / v.max(25.) * 0.35).clamp(-0.12, 0.12) * smooth(0.1, 0.3, a.abs());
    state.vortex_lift +=
        (vortex_target - state.vortex_lift) * (1. - (-dt.max(0.) / (tau * 2.)).exp());
    let attached = state.attached;
    let p = -c.rates.z * SPAN / (2. * v.max(25.));
    let q = c.rates.x * CHORD / (2. * v.max(25.));
    let r = -c.rates.y * SPAN / (2. * v.max(25.));
    let slope = tab(&CNA, m) * DEG;
    let vortex = 0.85
        * a.sin()
        * a.sin().abs()
        * a.cos()
        * smooth(0.1, 0.3, a.abs())
        * (1. - 0.55 * smooth(0.9, 3., m));
    let linear = tab(&CNB, m) + slope * a.sin() * a.cos() + vortex + state.vortex_lift;
    let flat = 1.5 * (2. * a).sin();
    let cn = linear * attached + flat * (1. - attached);
    let cn =
        cn + tab(&[-9., -9., -22., -22., -20., -21., -28., -35., -35.], m) * q.clamp(-0.3, 0.3);
    // Normal force projected to the wind lift axis; drag is independently dissipative.
    let lift = cn * a.cos();
    let mu = 1.716e-5 * (c.temperature.max(150.) / 273.15).powf(1.5) * (273.15 + 110.4)
        / (c.temperature.max(150.) + 110.4);
    let re = (c.density.max(0.) * v * CHORD / mu).max(1e4);
    let cf = 0.455 / (re.log10().powf(2.58) * (1. + 0.144 * m * m).powf(0.65));
    let parasite = 0.0042 + cf * 4.2 + 0.2 * c.beta.sin().powi(2) + if c.gear { 0.055 } else { 0. };
    let wave = 0.018 * (-((m - 1.04) / 0.19).powi(2)).exp() + 0.009 * smooth(0.95, 1.6, m);
    let induced = induced(lift, attached, c.beta, c.height, m);
    let separated = (1. - attached) * 1.55 * a.sin().powi(2) + 0.16 * (1. - a.cos()).max(0.);
    let pitch = tab(&CMB, m)
        + tab(&CMA, m) * DEG * a
        + 0.22 * (-a.sin()) * (1. - attached)
        + tab(&[-1.8, -1.8, -2.7, -2.6, -2.1, -1.8, -0.9, -0.9, -0.9], m) * q.clamp(-0.4, 0.4);
    let cy = tab(&CYB, m) * DEG * c.beta.sin() - 0.12 * c.rates.y * SPAN / (2. * v.max(25.));
    let cl = tab(&CLB, m) * DEG * c.beta.sin()
        + tab(
            &[
                -0.16, -0.16, -0.18, -0.19, -0.16, -0.12, -0.10, -0.09, -0.08,
            ],
            m,
        ) * p
        + 0.06 * r;
    let cn_yaw = tab(&CNBETA, m) * DEG * c.beta.sin()
        + tab(
            &[
                -0.13, -0.13, -0.13, -0.12, -0.10, -0.09, -0.08, -0.07, -0.06,
            ],
            m,
        ) * r
        - 0.015 * p;
    Coefficients {
        lift,
        drag: (parasite + wave + induced + separated).max(0.),
        side: cy,
        pitch,
        yaw: -cn_yaw,
        roll: -cl + 0.05 * (state.wing_attached[0] - state.wing_attached[1]),
        parasite,
        induced,
        wave,
        attached,
        control_scale: (0.3 + 0.7 * attached) * (1. - 0.2 * smooth(0.8, 3., m)),
        ..Default::default()
    }
}
pub fn steady(c: Conditions) -> Coefficients {
    let a = c.alpha - INCIDENCE;
    let mut s = FlowState {
        lag_alpha: a,
        lag_beta: c.beta,
        ..Default::default()
    };
    let target = attached_target(a, c.mach, 1.);
    s.wing_attached = [target; 2];
    s.attached = target;
    evaluate(c, &mut s, 0.)
}
pub fn targets(command: DVec3) -> [f64; 4] {
    let a = (-command.x + command.z).clamp(-1., 1.);
    let b = (-command.x - command.z).clamp(-1., 1.);
    let convert = |x: f64, inner: bool| {
        if x < 0. {
            x * 35_f64.to_radians()
        } else {
            x * if inner {
                20_f64.to_radians()
            } else {
                35_f64.to_radians()
            }
        }
    };
    [
        convert(a, true),
        convert(b, true),
        (convert(a, false) - 3_f64.to_radians()).max(-35_f64.to_radians()),
        (convert(b, false) - 3_f64.to_radians()).max(-35_f64.to_radians()),
    ]
}
pub fn control_coefficients(
    elevons: [f64; 4],
    fins: [f64; 2],
    mach: f64,
    attached: f64,
) -> Coefficients {
    let left = 0.6 * elevons[0] + 0.4 * elevons[2];
    let right = 0.6 * elevons[1] + 0.4 * elevons[3];
    let e = (left + right) * 0.5 + 0.4 * 3_f64.to_radians();
    let a = (left - right) * 0.5;
    let r = (fins[0] + fins[1]) * 0.5;
    let k = 0.3 + 0.7 * attached;
    let cn = tab(&CNDE, mach) * DEG * e * k;
    Coefficients {
        lift: cn,
        side: tab(&CYDR, mach) * DEG * r * k,
        pitch: tab(&CMDE, mach) * DEG * e * k,
        yaw: -(tab(&CNDR, mach) * DEG * r
            + tab(
                &[0.00005, 0.00005, 0.0001, 0.00005, 0., 0., 0., 0., 0.],
                mach,
            ) * DEG
                * a)
            * k,
        roll: -tab(&CLDA, mach) * DEG * a * k,
        drag: 0.035 * e * e + 0.025 * a * a + 0.03 * r * r,
        attached,
        ..Default::default()
    }
}
pub fn control_loads(
    s: &super::State,
    damage: &AircraftDamage,
    attached: f64,
    b: DVec3,
    rho: f64,
    mach: f64,
    command: Option<DVec3>,
) -> Loads {
    let mut angles = command.map(targets).unwrap_or(s.elevons);
    let mut fins = command
        .map(|c| [c.y * 20_f64.to_radians(); 2])
        .unwrap_or(s.fins);
    for (i, slot) in [0, 4, 2, 6].into_iter().enumerate() {
        angles[i] = crate::control_surfaces::effective_angle(damage, slot, angles[i])
            * damage.surfaces[slot];
    }
    for (i, slot) in [3, 7].into_iter().enumerate() {
        fins[i] =
            crate::control_surfaces::effective_angle(damage, slot, fins[i]) * damage.surfaces[slot];
    }
    let c = control_coefficients(angles, fins, mach, attached);
    let qs = 0.5 * rho * b.length_squared() * AREA;
    let force = crate::aerodynamics::force(DVec3::X, b, qs, c);
    let moment = DVec3::new(c.pitch * CHORD, c.yaw * SPAN, c.roll * SPAN) * qs;
    let mut l = Loads {
        force,
        moment,
        ..Default::default()
    };
    for (i, slot) in [0, 4, 2, 6].into_iter().enumerate() {
        l.surfaces[slot] = SurfaceLoad {
            deflection: angles[i],
            attached,
            force: force * 0.25,
            moment: moment * 0.25,
        };
    }
    for (i, slot) in [3, 7].into_iter().enumerate() {
        l.surfaces[slot].deflection = fins[i];
        l.surfaces[slot].attached = attached;
    }
    l
}
pub fn engine(power: f64, rho: f64, mach: f64) -> (f64, f64) {
    let axis = [0., 0.6, 0.9, 1.1, 1.6, 2., 2.5, 3.2, 3.6];
    let lapse = (rho.max(0.) / 1.225).powf(0.62);
    let dry = 180000.
        * table(
            &axis,
            &[1., 0.94, 0.86, 0.80, 0.72, 0.66, 0.56, 0.45, 0.25],
            mach,
        )
        * lapse;
    let wet = 289134.
        * table(
            &axis,
            &[1., 0.96, 0.94, 0.99, 1.18, 1.50, 2.0, 2.65, 2.15],
            mach,
        )
        * lapse;
    let burner = (wet - dry).max(0.) * ((power - 0.88) / 0.12).clamp(0., 1.);
    (
        dry * (power / 0.88).clamp(0., 1.).powf(1.35) + burner,
        burner,
    )
}
pub fn recovery(f: &Flight) -> f64 {
    let angular = 0.28 * smooth(0.16, 0.5, f.alpha.abs()) + 0.3 * smooth(0.07, 0.3, f.beta.abs());
    let desired = 0.66 * ((f.mach - 1.6) / 1.6).clamp(0., 1.);
    let mismatch = (f.systems.sr71.spike - desired).abs() / 0.66;
    (1. - angular - 0.25 * mismatch * smooth(1.5, 2.5, f.mach)).clamp(0.35, 1.)
}
pub fn installed(f: &Flight, rho: f64, mach: f64) -> (f64, f64) {
    let (t, b) = engine(f.spool, rho, mach);
    let r = recovery(f);
    (t * r, b * r)
}
pub fn fuel_flow(f: &Flight) -> f64 {
    if !f.engine || (!f.infinite_fuel && f.fuel <= 0.) {
        return 0.;
    }
    let (t, _) = installed(f, f.density, f.mach);
    let fraction = (0..2)
        .map(|i| {
            if f.systems.engine_on[i] {
                f.systems.engines[i]
            } else {
                0.
            }
        })
        .sum::<f64>()
        * 0.5;
    t * fraction * (0.000023 + 0.000021 * ((f.spool - 0.88) / 0.12).clamp(0., 1.))
}
pub fn target_rates(f: &Flight, c: DVec3) -> DVec3 {
    let m = f.mach;
    let bank = f.roll().clamp(-1.2, 1.2);
    let coordinated = 9.80665 * bank.tan() / f.airspeed.max(70.);
    let alpha_limit = 0.32 - 0.13 * smooth(0.8, 2.5, m);
    let mut pitch = c.x * (0.20 - 0.13 * smooth(0.8, 2.5, m));
    if c.x.abs() < 0.025 {
        pitch += ((f.g_load - 1.) * -0.025).clamp(-0.04, 0.04);
    }
    pitch = pitch.min((alpha_limit - f.alpha) * 1.2);
    DVec3::new(
        pitch + coordinated * bank.sin(),
        c.y * 0.10 - f.beta * 0.45 - coordinated * bank.cos(),
        -c.z * (0.65 - 0.4 * smooth(0.8, 2.5, m)),
    )
}
#[derive(Clone, Copy, Debug)]
pub struct Trim {
    pub alpha: f64,
    pub command: f64,
    pub throttle: f64,
    pub residual: f64,
}
fn condition(alpha: f64, alt: f64, speed: f64) -> Conditions {
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
pub fn trim(mass: f64, fuel: f64, alt: f64, speed: f64) -> Option<Trim> {
    let mut x = DVec3::new(0.08, 0.0, 0.8);
    let c = condition(0.08, alt, speed);
    let qs = 0.5 * c.density * speed * speed * AREA;
    let w = mass * 9.80665;
    let residual = |x: DVec3| {
        let mut c = c;
        c.alpha = x.x;
        let mut a = steady(c);
        a.pitch += ((0.201 * EMPTY + 0.235 * fuel) / (EMPTY + fuel) - 0.25) * a.lift;
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
            a.pitch + thrust * 12.4 * INCIDENCE.sin() / (qs * CHORD),
        )
    };
    for _ in 0..35 {
        let r = residual(x);
        if r.length() < 0.0001 {
            return Some(Trim {
                alpha: x.x,
                command: x.y,
                throttle: x.z,
                residual: r.length(),
            });
        }
        let h = 1e-4;
        let jac = DMat3::from_cols(
            (residual(x + DVec3::X * h) - residual(x - DVec3::X * h)) / (2. * h),
            (residual(x + DVec3::Y * h) - residual(x - DVec3::Y * h)) / (2. * h),
            (residual(x + DVec3::Z * h) - residual(x - DVec3::Z * h)) / (2. * h),
        );
        if jac.determinant().abs() < 1e-12 {
            return None;
        }
        let delta = (jac.inverse() * r).clamp(DVec3::splat(-0.15), DVec3::splat(0.15));
        x -= delta;
        x = x.clamp(DVec3::new(-0.1, -1., 0.01), DVec3::new(0.48, 1., 1.));
    }
    None
}
pub fn initialize(f: &mut Flight) {
    f.autopilot = false;
    f.flaps = 0.;
    f.airbrake = false;
    if !f.landed {
        if let Some(t) = trim(f.mass(), f.fuel, f.altitude(), f.airspeed) {
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
    f.systems.sr71.spike = 0.66 * ((f.mach - 1.6) / 1.6).clamp(0., 1.);
}
#[cfg(test)]
mod tests {
    use super::*;
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
            assert!((inverse * i - DMat3::IDENTITY)
                .to_cols_array()
                .iter()
                .all(|x| x.abs() < 1e-9));
        }
    }
    #[test]
    fn trim_envelope_register() {
        let mut rows = String::from(
            "altitude_m,speed_m_s,mass_kg,alpha_deg,elevon_command,throttle,residual\n",
        );
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
            (f.altitude() - alt).abs() < 250.,
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
        println!("SR71 coefficient evaluation: {} ns/call (100000 iterations; test-profile microbenchmark)",start.elapsed().as_nanos()/100000);
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
}
