//! Estimated component modes. Exact damped response for held loads; aerodynamic
//! coupling is substepped by the flight solver. This is not a flutter solver.
use super::*;
#[derive(Clone, Copy, Debug, Default)]
pub struct Mode {
    pub displacement: f64,
    pub velocity: f64,
    pub twist: f64,
    pub twist_rate: f64,
}
fn parameters(i: usize) -> (f64, f64, f64, f64) {
    if i < 2 {
        (210., 6., 12., 4400000.)
    } else if i < 4 {
        (65., 8., 15., 110000.)
    } else {
        (80., 10., 16., 140000.)
    }
}
pub fn equilibrate(f: &mut Flight, loads: &Loads) {
    for i in 0..6 {
        let (m, hz, _, k) = parameters(i);
        let health = if i < 2 {
            f.damage.wings[i]
        } else {
            f.damage.surfaces[if i < 4 { (i - 2) * 4 } else { (i - 4) * 4 + 3 }]
        };
        let mode = &mut f.hornet.modes[i];
        mode.displacement =
            loads.elastic[i] / (m * (std::f64::consts::TAU * hz).powi(2) * health.max(0.15));
        mode.twist = loads.elastic[i] * 0.12 / (k * health.max(0.15));
        mode.velocity = 0.;
        mode.twist_rate = 0.;
    }
    sync(f);
}
fn sync(f: &mut Flight) {
    for i in 0..2 {
        f.hornet.flex[i] = f.hornet.modes[i].displacement;
        f.hornet.flex_rate[i] = f.hornet.modes[i].velocity;
        f.hornet.twist[i] = f.hornet.modes[i].twist;
    }
}
#[derive(Clone, Debug, Default)]
pub struct Channels {
    pub aerodynamic: [f64; 6],
    pub inertial: [f64; 6],
    pub landing: DVec3,
    pub hook: DVec3,
    pub catapult: DVec3,
    pub mach_excess: f64,
    pub thermal_excess: f64,
}
pub fn advance(f: &mut Flight, loads: &Loads, dt: f64) {
    let equilibrium = f.temperature * (1. + 0.85 * 0.2 * f.mach.powi(2));
    f.hornet.skin_temperature +=
        (equilibrium - f.hornet.skin_temperature) * (1. - (-dt / 40.).exp());
    let thermal = (f.hornet.skin_temperature / 450. - 1.).max(0.);
    let overspeed = (f.mach / 1.85 - 1.).max(0.);
    f.hornet.structural_loads = Channels {
        aerodynamic: loads.elastic,
        inertial: [0.; 6],
        landing: f.hornet.diagnostic.contact.force,
        hook: if f.carrier.arrested {
            f.hornet.diagnostic.attachment.force
        } else {
            DVec3::ZERO
        },
        catapult: if f.carrier.attached || f.carrier.launch.is_some() {
            f.hornet.diagnostic.attachment.force
        } else {
            DVec3::ZERO
        },
        mach_excess: overspeed,
        thermal_excess: thermal,
    };
    for i in 0..6 {
        let (mass, hz, torsion, k) = parameters(i);
        let surface = if i < 2 {
            i * 4 + 1
        } else if i < 4 {
            (i - 2) * 4
        } else {
            (i - 4) * 4 + 3
        };
        let health = if i < 2 {
            f.damage.wings[i]
        } else {
            f.damage.surfaces[surface]
        };
        let scale = (health.max(0.15) * (1. - 0.3 * thermal.min(1.))).sqrt();
        let mode = &mut f.hornet.modes[i];
        let inertial = if i < 4 {
            mass * f.g_load * 9.80665
        } else {
            mass * f.lateral_g * 9.80665
        };
        f.hornet.structural_loads.inertial[i] = inertial;
        crate::structural::oscillator(
            &mut mode.displacement,
            &mut mode.velocity,
            loads.elastic[i] - inertial,
            mass,
            hz * scale,
            0.18,
            dt,
        );
        let rotational_mass = k / (std::f64::consts::TAU * torsion).powi(2);
        crate::structural::oscillator(
            &mut mode.twist,
            &mut mode.twist_rate,
            loads.elastic[i] * 0.12,
            rotational_mass,
            torsion * scale,
            0.22,
            dt,
        );
        if i < 2 {
            let limit = f.mass() * 9.80665 * if loads.elastic[i] >= 0. { 7.5 } else { 3.0 } * 0.5;
            let overload = (loads.elastic[i].abs() / limit - 1.).max(0.);
            let q_excess = (0.5 * f.density * f.airspeed.powi(2) / 90000. - 1.).max(0.);
            f.hornet.fatigue[i] += dt
                * (overload.powi(2) * 0.15
                    + q_excess.powi(2) * 0.03
                    + overspeed.powi(2) * 0.02
                    + thermal.powi(2) * 0.03);
            f.damage.wings[i] = (health - dt * overload * 0.04).max(0.);
            if f.hornet.fatigue[i] > 1. {
                f.damage.wings[i] = 0.;
                f.damage.wing_broken[i] = true;
                for j in [i * 4 + 1, i * 4 + 2] {
                    f.damage.surface_state[j] = crate::combat::damage::SurfaceState::Detached;
                }
            }
        } else {
            let excess =
                (loads.elastic[i].abs() / if i < 4 { 100000. } else { 80000. } - 1.).max(0.);
            f.damage.surfaces[surface] = (health - dt * excess.powi(2) * 0.15).max(0.);
            if f.damage.surfaces[surface] <= 0. {
                f.damage.surface_state[surface] = crate::combat::damage::SurfaceState::Detached;
            }
        }
    }
    let attachment = f.hornet.diagnostic.attachment.force.length();
    if attachment > 900000. {
        f.integrity = (f.integrity - dt * (attachment / 900000. - 1.) * 0.1).max(0.);
    }
    sync(f);
}
