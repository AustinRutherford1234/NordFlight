//! Reconfigurable six-effector allocation. Solve on the physical aerodynamic
//! Jacobian, with travel/reachability bounds and preference for small motion.
use super::*;
pub const EFFECTORS: [usize; 6] = [0, 2, 3, 4, 6, 7];
#[derive(Clone, Copy, Debug, Default)]
pub struct Result {
    pub angles: [f64; 16],
    pub residual: DVec3,
    pub saturation: f64,
    pub authority: DVec3,
}
pub fn allocate(
    f: &mut Flight,
    measured: &sensors::Sample,
    desired_acceleration: DVec3,
    q: f64,
) -> Result {
    let inputs = (f.velocity, f.rates, f.alpha, f.beta, f.mach);
    let velocity = DVec3::new(
        measured.beta.sin(),
        -measured.alpha.sin() * measured.beta.cos(),
        -measured.alpha.cos() * measured.beta.cos(),
    ) * measured.speed;
    f.velocity = f.wind + f.orientation * velocity;
    f.rates = measured.rates;
    f.alpha = measured.alpha;
    f.beta = measured.beta;
    f.mach = measured.mach;
    let saved = f.hornet.angles;
    let (_, inertia) = mass::properties(f);
    let inverse = inertia.inverse();
    let base = aero::forces(f, 0.);
    let mut response = [DVec3::ZERO; 6];
    let mut lower = [0.; 6];
    let mut upper = [0.; 6];
    for (j, i) in EFFECTORS.into_iter().enumerate() {
        if f.damage.surfaces[i] < 0.05
            || matches!(
                f.damage.surface_state[i],
                crate::combat::damage::SurfaceState::Jammed(_)
                    | crate::combat::damage::SurfaceState::Detached
            )
        {
            continue;
        }
        f.hornet.angles = saved;
        f.hornet.angles[i] += 0.002;
        let plus = aero::forces(f, 0.).moment;
        f.hornet.angles[i] -= 0.004;
        let minus = aero::forces(f, 0.).moment;
        response[j] = inverse * ((plus - minus) / 0.004);
        // A clipped local polar can have zero derivative at a surface stop.
        // A bounded secant around trim retains a direction for backing away
        // from saturation; the real actuator and nonlinear plant still apply.
        f.hornet.angles = saved;
        let nominal = aero::targets(f, DVec3::ZERO)[i];
        f.hornet.angles[i] = (nominal + 0.4).min(0.55);
        let hi = f.hornet.angles[i];
        let wide_plus = aero::forces(f, 0.).moment;
        f.hornet.angles[i] = (nominal - 0.4).max(-0.55);
        let lo = f.hornet.angles[i];
        let wide_minus = aero::forces(f, 0.).moment;
        let secant = inverse * ((wide_plus - wide_minus) / (hi - lo));
        if response[j].length() < secant.length() * 0.15 {
            response[j] = secant;
        }

        let reach =
            (0.9 * f.damage.hydraulics * f.hornet.hydraulic_pressure * f.damage.surfaces[i]
                / (1. + q / 100000.)
                * 0.22)
                .max(0.);
        lower[j] = (-0.55 - saved[i]).max(-reach);
        upper[j] = (0.55 - saved[i]).min(reach);
    }
    f.hornet.angles = saved;
    let base_acc = inverse
        * (base.moment + f.hornet.diagnostic.engine.moment - f.rates.cross(inertia * f.rates));
    let target = desired_acceleration - base_acc;
    let authority = response.iter().fold(DVec3::ZERO, |a, r| a + r.abs() * 0.3);
    // Normalize axes before allocation so roll cannot consume all the pitch/yaw
    // authority simply because it has a larger numerical moment derivative.
    let weights = DVec3::new(1.4, 1.1, 1.) / authority.max(DVec3::splat(0.2));
    let columns = response.map(|r| r * weights);
    let demand = target * weights;
    let mut delta = [0.; 6];
    let mut accumulated = DVec3::ZERO;
    for _ in 0..18 {
        for j in 0..6 {
            let column = columns[j];
            let old = delta[j];
            let error = demand - accumulated;
            let preference = if j == 0 || j == 3 { 0.002 } else { 0.003 };
            delta[j] = (old
                + (column.dot(error) - preference * old) / (column.length_squared() + preference))
                .clamp(lower[j], upper[j]);
            accumulated += column * (delta[j] - old);
        }
    }
    let mut angles = saved;
    let mut achieved = base_acc;
    let mut saturated = 0.;
    for (j, i) in EFFECTORS.into_iter().enumerate() {
        angles[i] = (saved[i] + delta[j]).clamp(-0.55, 0.55);
        achieved += response[j] * delta[j];
        if upper[j] - lower[j] > 1e-8 && (delta[j] - lower[j]).min(upper[j] - delta[j]) < 0.001 {
            saturated += 1.;
        }
    }
    (f.velocity, f.rates, f.alpha, f.beta, f.mach) = inputs;
    Result {
        angles,
        residual: desired_acceleration - achieved,
        saturation: saturated / 6.,
        authority,
    }
}
