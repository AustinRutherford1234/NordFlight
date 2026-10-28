//! Compatibility adapter for the existing rate-command assistance. No torque,
//! attitude, or velocity is imposed on the plant. Disable with T for raw tests.
use super::*;
pub fn command(f: &mut Flight, q: f64) -> DVec3 {
    let mut command = f.controls;
    if f.autopilot && !f.landed {
        let heading = f.forward().x.atan2(-f.forward().z);
        let bank = (((-f.position.x / 1800.).clamp(-0.35, 0.35) - heading).clamp(-0.4, 0.4)) * 1.4;
        let desired_alt = RUNWAY_HEIGHT + 1.55 + (f.position.z - 1000.).max(0.) * 0.0524;
        let pitch = (0.055 + (desired_alt - f.altitude()) * 0.001
            - (f.vertical_speed() + f.airspeed * 0.0524) * 0.004)
            .clamp(-0.13, 0.20);
        command = DVec3::new(
            ((pitch - f.pitch()) * 3.).clamp(-0.55, 0.55),
            -f.beta * 2.,
            ((bank - f.roll()) * 2.).clamp(-0.65, 0.65),
        );
    }
    if !f.assist || f.damage.avionics <= 0.25 || q < 20. {
        return command;
    }
    if !f.landed && !f.autopilot {
        if command.x > 0. {
            command.x *= ((f.aircraft.spec().alpha - f.alpha - f.rates.x * 0.15) / 0.10)
                .clamp(0., 1.)
                .min(((7.5 - f.g_load) / 2.).clamp(0., 1.));
        } else {
            command.x *= ((f.g_load + 3.) / 1.5).clamp(0., 1.);
        }
    }
    let (_, inertia) = mass::properties(f);
    let angles = f.hornet.angles;
    let mut probe = |u: DVec3| {
        f.hornet.angles = aero::targets(f, u);
        aero::forces(f, 0.).moment
    };
    let zero = probe(DVec3::ZERO);
    let h = 0.02;
    let jac = DMat3::from_cols(
        (probe(DVec3::X * h) - probe(-DVec3::X * h)) / (2. * h),
        (probe(DVec3::Y * h) - probe(-DVec3::Y * h)) / (2. * h),
        (probe(DVec3::Z * h) - probe(-DVec3::Z * h)) / (2. * h),
    );
    f.hornet.angles = angles;
    let target = DVec3::new(command.x * 0.65, command.y * 0.38, -command.z * 1.55);
    let request = inertia * ((target - f.rates) * 5.) + f.rates.cross(inertia * f.rates)
        - zero
        - f.hornet.diagnostic.engine.moment;
    crate::control_surfaces::allocate(jac, request).clamp(DVec3::splat(-1.), DVec3::ONE)
}
