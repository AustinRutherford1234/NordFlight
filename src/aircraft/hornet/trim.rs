use super::*;
#[derive(Clone, Copy, Debug)]
pub struct Trim {
    pub alpha: f64,
    pub stabilator: f64,
    pub throttle: f64,
    pub residual: DVec3,
}
pub fn solve(base: &Flight, height: f64, speed: f64, gamma: f64, bank: f64) -> Option<Trim> {
    let mut x = DVec3::new(0.12, 0.08, 0.5);
    let residual = |x: DVec3| {
        let mut f = base.clone();
        f.position = surface_point(0., 0., height);
        f.orientation = DQuat::from_rotation_x(x.x + gamma);
        f.wind = DVec3::ZERO;
        f.velocity = DVec3::new(0., speed * gamma.sin(), -speed * gamma.cos());
        f.rates = DVec3::ZERO;
        f.assist = false;
        let (temp, p, rho, a) = atmosphere(height);
        f.temperature = temp;
        f.pressure = p;
        f.density = rho;
        f.airspeed = speed;
        f.mach = speed / a;
        f.alpha = x.x;
        f.beta = 0.;
        f.hornet.angles[0] = x.y;
        f.hornet.angles[4] = x.y;
        for i in [1, 5] {
            f.hornet.angles[i] = f.flaps * 0.52;
        }
        let steady = (1. - ((x.x.abs() + 0.025 - 0.30) / 0.35).clamp(0., 1.)).powi(2);
        for flow in &mut f.hornet.flow {
            flow.attached = steady;
            flow.lag = x.x + 0.025;
        }
        let mut loads = aero::evaluate(&mut f, 0.);
        for _ in 0..12 {
            structure::equilibrate(&mut f, &loads);
            for (i, flow) in f.hornet.flow.iter_mut().enumerate() {
                flow.lag = x.x + 0.025 + f.hornet.twist[i / 4];
                flow.attached = (1. - ((flow.lag.abs() - 0.30) / 0.35).clamp(0., 1.)).powi(2);
            }
            loads = aero::forces(&mut f, 0.);
        }
        let thrust = engine::output(x.z, rho, f.mach).0 * 2.;
        let (cg, _) = mass::properties(&f);
        let mut total = loads;
        total.add(
            DVec3::NEG_Z * thrust,
            (engine::position(0) + engine::position(1)) * 0.5 - cg,
        );
        let force = f.orientation * total.force;
        let weight = f.mass() * 9.80665;
        DVec3::new(
            (force.y - weight / bank.cos().max(0.5)) / weight,
            force.z / weight,
            total.moment.x / (0.5 * rho * speed * speed * config::AREA * config::CHORD),
        )
    };
    for _ in 0..40 {
        let r = residual(x);
        if r.x.abs() < 0.005 && r.y.abs() < 0.005 && r.z.abs() < 0.0005 {
            return Some(Trim {
                alpha: x.x,
                stabilator: x.y,
                throttle: x.z,
                residual: r,
            });
        }
        let h = 0.0001;
        let jac = DMat3::from_cols(
            (residual(x + DVec3::X * h) - r) / h,
            (residual(x + DVec3::Y * h) - r) / h,
            (residual(x + DVec3::Z * h) - r) / h,
        );
        if jac.determinant().abs() < 1e-10 {
            return None;
        }
        x -= (jac.inverse() * r).clamp(DVec3::splat(-0.15), DVec3::splat(0.15));
        x = x.clamp(DVec3::new(-0.1, -0.5, 0.01), DVec3::new(0.6, 0.5, 1.));
    }
    None
}
pub fn apply(f: &mut Flight, trim: Trim) {
    f.throttle = trim.throttle;
    f.spool = trim.throttle;
    f.hornet.initialized = true;
    for engine in &mut f.hornet.engines {
        engine.spool = trim.throttle;
    }
    f.hornet.angles[0] = trim.stabilator;
    f.hornet.angles[4] = trim.stabilator;
    for i in [1, 5] {
        f.hornet.angles[i] = f.flaps * 0.52;
    }
    f.hornet.trim = trim.stabilator;
    f.controls = DVec3::ZERO;
    let rho = atmosphere(f.altitude()).2;
    f.density = rho;
    f.alpha = trim.alpha;
    f.airspeed = (f.velocity - f.wind).length();
    f.mach = f.airspeed / atmosphere(f.altitude()).3;
    for _ in 0..12 {
        for (i, flow) in f.hornet.flow.iter_mut().enumerate() {
            flow.lag = trim.alpha + 0.025 + f.hornet.twist[i / 4];
            flow.attached = (1. - ((flow.lag.abs() - 0.30) / 0.35).clamp(0., 1.)).powi(2);
        }
        let loads = aero::forces(f, 0.);
        structure::equilibrate(f, &loads);
    }
}

/// Offline six-variable trim. Includes the centripetal force and body rates of
/// a steady banked turn; solves sideslip, aileron and rudder as well as pitch
/// incidence, stabilator and throttle. No runtime controller uses this solver.
#[derive(Clone)]
pub struct Balanced {
    pub aircraft: Flight,
    pub force_error: DVec3,
    pub moment_error: DVec3,
}
pub fn balanced(base: &Flight, height: f64, speed: f64, gamma: f64, bank: f64) -> Option<Balanced> {
    let seed = solve(base, height, speed, gamma, 0.)?;
    let mut x = [seed.alpha, seed.stabilator, seed.throttle, 0., 0., 0.];
    let evaluate = |x: [f64; 6]| {
        let mut f = base.clone();
        f.assist = false;
        f.position = surface_point(0., 0., height);
        f.wind = DVec3::ZERO;
        f.orientation = DQuat::from_rotation_y(x[3])
            * DQuat::from_rotation_x(x[0] + gamma)
            * DQuat::from_rotation_z(bank);
        f.velocity = DVec3::new(0., speed * gamma.sin(), -speed * gamma.cos());
        let yaw_rate = 9.80665 * bank.tan() / speed.max(1.);
        f.rates = f.orientation.conjugate() * DVec3::Y * yaw_rate;
        f.airspeed = speed;
        let (_, _, rho, sound) = atmosphere(height);
        f.density = rho;
        f.mach = speed / sound;
        let air = f.orientation.conjugate() * f.velocity;
        f.alpha = (-air.y).atan2(-air.z);
        f.beta = (air.x / speed).clamp(-1., 1.).asin();
        f.hornet.trim = x[1];
        f.hornet.angles = aero::targets(&f, DVec3::new(0., x[5], x[4]));
        let mut loads = Loads::default();
        for _ in 0..20 {
            for (i, flow) in f.hornet.flow.iter_mut().enumerate() {
                flow.lag = f.alpha + 0.025 + f.hornet.twist[i / 4];
                flow.attached = (1. - ((flow.lag.abs() - 0.30) / 0.35).clamp(0., 1.)).powi(2);
            }
            loads = aero::forces(&mut f, 0.);
            structure::equilibrate(&mut f, &loads);
        }
        let (cg, inertia) = mass::properties(&f);
        let thrust = engine::output(x[2], rho, f.mach).0;
        for i in 0..2 {
            loads.add(DVec3::NEG_Z * thrust, engine::position(i) - cg);
        }
        let weight = f.mass() * 9.80665;
        let required = DVec3::new(-weight * bank.tan() * gamma.cos(), weight, 0.);
        let force = (f.orientation * loads.force - required) / weight;
        let qsc = 0.5 * rho * speed * speed * config::AREA;
        let moment = (loads.moment - f.rates.cross(inertia * f.rates))
            / (qsc * DVec3::new(config::CHORD, config::SPAN, config::SPAN));
        f.throttle = x[2];
        (
            Balanced {
                aircraft: f,
                force_error: force,
                moment_error: moment,
            },
            [force.x, force.y, force.z, moment.x, moment.y, moment.z],
        )
    };
    for _ in 0..35 {
        let (result, r) = evaluate(x);
        if result.force_error.length() < 0.01 && result.moment_error.length() < 0.001 {
            return Some(result);
        }
        let mut a = [[0.; 7]; 6];
        for j in 0..6 {
            let mut next = x;
            next[j] += 0.0001;
            let (_, r2) = evaluate(next);
            for i in 0..6 {
                a[i][j] = (r2[i] - r[i]) / 0.0001;
            }
        }
        for i in 0..6 {
            a[i][6] = r[i];
        }
        for col in 0..6 {
            let pivot = (col..6).max_by(|i, j| a[*i][col].abs().total_cmp(&a[*j][col].abs()))?;
            a.swap(pivot, col);
            let div = a[col][col];
            if div.abs() < 1e-10 {
                return None;
            }
            for j in col..7 {
                a[col][j] /= div;
            }
            for row in 0..6 {
                if row == col {
                    continue;
                }
                let factor = a[row][col];
                for j in col..7 {
                    a[row][j] -= factor * a[col][j];
                }
            }
        }
        for i in 0..6 {
            x[i] -= a[i][6].clamp(-0.1, 0.1);
        }
        x[0] = x[0].clamp(-0.2, 0.6);
        x[1] = x[1].clamp(-0.5, 0.5);
        x[2] = x[2].clamp(0.01, 1.);
        for i in 3..6 {
            x[i] = x[i].clamp(-0.5, 0.5);
        }
    }
    None
}
#[cfg(test)]
mod equilibrium_tests {
    use super::*;
    #[test]
    fn climbs_descents_turns_and_weights() {
        let mut f = Flight::approach();
        crate::aircraft::select(&mut f, crate::aircraft::Aircraft::Hornet);
        f.gear = false;
        f.gear_extension = 0.;
        f.flaps = 0.;
        for fuel in [1000., 5000.] {
            f.fuel = fuel;
            for (gamma, bank) in [(0., 0.), (0.05, 0.), (-0.05, 0.), (0., 0.35), (0., -0.35)] {
                let b = balanced(&f, 3000., 180., gamma, bank).expect("six degree trim");
                println!(
                    "BALANCED fuel={} gamma={} bank={} force={} moment={} alpha={} power={}",
                    fuel,
                    gamma,
                    bank,
                    b.force_error.length(),
                    b.moment_error.length(),
                    b.aircraft.alpha,
                    b.aircraft.throttle
                );
                assert!(b.force_error.length() < 0.01 && b.moment_error.length() < 0.001);
            }
        }
    }
}
