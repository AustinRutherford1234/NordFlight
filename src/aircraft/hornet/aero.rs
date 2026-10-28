use super::*;
#[derive(Clone, Copy, Debug)]
pub struct Flow {
    pub attached: f64,
    pub lag: f64,
}
impl Default for Flow {
    fn default() -> Self {
        Self {
            attached: 1.,
            lag: 0.,
        }
    }
}
pub(super) fn targets(f: &Flight, raw: DVec3) -> [f64; 16] {
    [
        f.hornet.trim + raw.x * 0.42 - raw.z * 0.12,
        f.flaps * 0.52,
        raw.z * 0.35,
        -raw.y * 0.40,
        f.hornet.trim + raw.x * 0.42 + raw.z * 0.12,
        f.flaps * 0.52,
        -raw.z * 0.35,
        -raw.y * 0.40,
        0.,
        f.flaps * 0.35,
        f.flaps * 0.35,
        0.,
        0.,
        0.,
        0.,
        0.,
    ]
}
pub fn advance_actuators(f: &mut Flight, q: f64, dt: f64) {
    let commands = super::fbw::commands(f,q,dt);
    for (i, target) in commands.into_iter().enumerate() {
        let health = f.damage.surfaces[i].clamp(0., 1.);
        let authority = f.damage.hydraulics.clamp(0., 1.) * f.hornet.hydraulic_pressure * health * super::fbw::actuator_power(f);
        let target = crate::control_surfaces::effective_angle(&f.damage, i, target);
        if matches!(
            f.damage.surface_state[i],
            crate::combat::damage::SurfaceState::Jammed(_)
        ) {
            f.hornet.angles[i] = target;
            f.hornet.actuator_rates[i] = 0.;
            continue;
        }
        let rate =
            ((target - f.hornet.angles[i]) * 10.).clamp(-0.9, 0.9) * authority / (1. + q / 100000.);
        f.hornet.actuator_rates[i] += (rate - f.hornet.actuator_rates[i]).clamp(-4. * dt, 4. * dt);
        f.hornet.angles[i] =
            (f.hornet.angles[i] + f.hornet.actuator_rates[i] * dt).clamp(-0.55, 0.55);
    }
}
/// Distributed neutral and moving-surface forces; no generic polar is added.
pub fn evaluate(f: &mut Flight, dt: f64) -> Loads {
    let q = 0.5 * f.density * (f.velocity - f.wind).length_squared();
    advance_actuators(f, q, dt);
    forces(f, dt)
}
pub(super) fn forces(f: &mut Flight, dt: f64) -> Loads {
    let height = (f.position.length() - RADIUS - crate::carrier::ground(f, f.position)).max(0.);
    let (cg, _) = mass::properties(f);
    let air = f.orientation.conjugate() * (f.velocity - f.wind);
    let mut loads = Loads::default();
    let q = 0.5 * f.density * air.length_squared();
    let mut wing_lift = [0.; 2];
    for side_i in 0..2 {
        let side = if side_i == 0 { -1. } else { 1. };
        let base = side_i * 4;
        for strip in 0..4 {
            let at = DVec3::new(
                side * (1.5 + strip as f64 * 1.2),
                0.,
                0.25 + strip as f64 * 0.18,
            );
            let v = air + f.rates.cross(at - cg) + DVec3::Y * f.hornet.flex_rate[side_i];
            let speed = v.length();
            if speed < 0.01 {
                continue;
            }
            let alpha = (-v.y).atan2(-v.z) + 0.025 + f.hornet.twist[side_i];
            let flow = &mut f.hornet.flow[side_i * 4 + strip];
            flow.lag += (alpha - flow.lag) * (1. - (-dt / 0.12).exp());
            let target = (1.
                - ((flow.lag.abs() - (0.30 + 0.12 * f.hornet.angles[9 + side_i].abs())) / 0.35)
                    .clamp(0., 1.))
            .powi(2);
            flow.attached += (target - flow.attached)
                * (1. - (-dt / if target < flow.attached { 0.08 } else { 0.35 }).exp());
            let cl_att = config::table(
                alpha.abs(),
                &[0., 0.1, 0.2, 0.3, 0.45, 0.65, 1.0, 1.57, 3.142],
                &[0., 0.48, 0.92, 1.28, 1.55, 1.2, 0.55, 0., 0.],
            ) * alpha.signum();
            let flap = crate::control_surfaces::effective_angle(
                &f.damage,
                base + 1,
                f.hornet.angles[base + 1],
            ) * f.damage.surfaces[base + 1]
                * 1.5
                + crate::control_surfaces::effective_angle(
                    &f.damage,
                    base + 2,
                    f.hornet.angles[base + 2],
                ) * f.damage.surfaces[base + 2]
                    * 0.9;
            let compress = config::table(
                f.mach,
                &[0., 0.65, 0.9, 1.1, 1.5, 2.2],
                &[1., 1., 1.12, 0.85, 0.65, 0.5],
            );
            let cl = ((cl_att + flap * flow.attached) * flow.attached
                + 0.75 * (2. * alpha).sin() * (1. - flow.attached))
                * compress;
            let ground = (height / config::SPAN).clamp(0., 1.);
            let induced = cl * cl / (std::f64::consts::PI * 4.0 * 0.78) * (0.55 + 0.45 * ground);
            let wave = config::table(
                f.mach,
                &[0., 0.75, 0.95, 1.1, 1.5, 2.2],
                &[0., 0., 0.012, 0.024, 0.017, 0.015],
            );
            let cd = 0.018
                + induced
                + wave
                + 1.4 * (1. - flow.attached) * alpha.sin().powi(2)
                + 0.04 * flap.powi(2);
            let lift = DVec3::X.cross(v.normalize()).normalize_or_zero();
            let force = (lift * cl - v.normalize() * cd)
                * (0.5 * f.density * speed * speed * 4.8)
                * f.damage.wings[side_i];
            loads.add(force, at - cg);
            wing_lift[side_i] += force.y;
        }
        // Leading-edge extension vortex contribution is separate from wing strips.
        let vortex = f.damage.wings[side_i]
            * 0.3
            * (2. * f.alpha).sin()
            * (-((f.alpha.abs() - 0.45) / 0.8).powi(2)).exp();
        loads.add(
            DVec3::Y * (q * 3.0 * vortex),
            DVec3::new(side * 1.0, 0., -1.1) - cg,
        );
        // Stabilator: positive deflection means trailing edge up and tail downforce.
        let tail = DVec3::new(side * 2.6, 0., 6.0);
        let v = air + f.rates.cross(tail - cg) + DVec3::Y * f.hornet.modes[2 + side_i].velocity;
        let speed = v.length();
        if speed > 0.01
            && f.damage.surface_state[base] != crate::combat::damage::SurfaceState::Detached
        {
            let a = (-v.y).atan2(-v.z) - 0.4 * f.alpha - f.hornet.angles[base]
                + f.hornet.modes[2 + side_i].twist;
            let cl = (3.2 * a).clamp(-1.4, 1.4);
            let force = (DVec3::X.cross(v.normalize()).normalize_or_zero() * cl
                - v.normalize() * (0.014 + 0.08 * cl * cl))
                * (0.5 * f.density * speed * speed * 4.0)
                * f.damage.surfaces[base];
            loads.add(force, tail - cg);
            loads.elastic[2 + side_i] = force.y;
        }
        let fin = DVec3::new(side * 1.7, 1.7, 4.8);
        let v = air + f.rates.cross(fin - cg) + DVec3::X * f.hornet.modes[4 + side_i].velocity;
        let beta =
            (v.x / v.length().max(0.1)).clamp(-1., 1.).asin() + f.hornet.modes[4 + side_i].twist;
        if f.damage.surface_state[base + 3] != crate::combat::damage::SurfaceState::Detached {
            let force = -DVec3::X
                * (0.5
                    * f.density
                    * v.length_squared()
                    * 3.8
                    * (2.3 * beta + 1.1 * f.hornet.angles[base + 3]))
                * f.damage.surfaces[base + 3];
            loads.add(force, fin - cg);
            loads.elastic[4 + side_i] = force.x;
        }
    }
    let cd = 0.008
        + 0.02 * f.gear_extension
        + 0.05 * if f.airbrake { 1. } else { 0. }
        + crate::aircraft::store_drag(f);
    loads.add(
        -air.normalize_or_zero() * (q * config::AREA * cd),
        DVec3::new(0., -0.05, 0.) - cg,
    );
    loads.add(
        -DVec3::X * (q * config::AREA * 0.18 * f.beta.sin()),
        DVec3::new(0., 0., 0.5) - cg,
    );
    for z in [-4., 0., 4.] {
        let at = DVec3::Z * z;
        let v = air + f.rates.cross(at - cg);
        let alpha = (-v.y).atan2(-v.z);
        loads.add(
            DVec3::Y * (0.5 * f.density * v.length_squared() * 2.5 * 0.6 * (2. * alpha).sin()),
            at - cg,
        );
    }
    loads.wings = wing_lift;
    loads.elastic[..2].copy_from_slice(&wing_lift);
    loads
}
