//! Representative scheduled command laws. Every gain/limit is an engineering
//! estimate for this plant, not recovered production F/A-18 software.
use super::*;
#[derive(Clone, Copy, Debug, Default)]
pub struct Law {
    pub integral: DVec3,
    pub load_integral: f64,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Demand {
    pub acceleration: DVec3,
    pub rates: DVec3,
    pub load: f64,
    pub alpha_limit: f64,
    pub protected: bool,
}
pub fn demand(
    f: &Flight,
    s: &sensors::Sample,
    pilot: DVec3,
    mode: Mode,
    ground: bool,
    law: &mut Law,
    saturated: bool,
    dt: f64,
) -> Demand {
    let speed = s.speed.max(35.);
    let q = 0.5 * f.density * s.speed * s.speed;
    let configuration = ((f.flaps - 0.2) / 0.6).clamp(0., 1.);
    let pitch_limit = 0.65 - 0.20 * configuration;
    let max_load = (7.5 * 20000. / f.mass()).clamp(5.5, 7.5);
    let load = if pilot.x >= 0. {
        1. + pilot.x * (max_load - 1.)
    } else {
        1. + pilot.x * 4.
    };
    let alpha_limit = config::table(s.mach, &[0., 0.7, 1.2, 2.2], &[0.60, 0.60, 0.40, 0.28]);
    let normal = mode == Mode::Normal && !ground;
    let error = load - s.load;
    if normal && !saturated && pilot.x.abs() < 0.8 {
        law.load_integral = (law.load_integral + error * dt * 0.025).clamp(-0.10, 0.10);
    } else {
        law.load_integral *= (-dt / 0.8).exp();
    }
    let gravity = (f.orientation.conjugate() * f.up()).y;
    let nz_rate = (load - gravity) * 9.80665 / speed + error * 0.12 + law.load_integral;
    // With manual flaps extended, use a damped pitch-rate response. No target
    // angle of attack, attitude, glide path, altitude, speed or throttle hold.
    let mut pitch = if normal {
        nz_rate * (1. - configuration) + pilot.x * pitch_limit * configuration
    } else {
        pilot.x * pitch_limit
    };
    let protection = !ground && mode != Mode::Direct && s.speed > 35.;
    if protection {
        let predicted = s.alpha + s.rates.x * 0.20;
        let upper = ((alpha_limit - predicted) * 2.).clamp(-0.45, pitch_limit);
        let lower = ((-0.28 - predicted) * 2.).clamp(-pitch_limit, 0.45);
        pitch = pitch.min(upper).max(lower);
        // Feedback limits can command unloading even with the pilot holding aft.
        pitch = pitch.min(((max_load + 0.2 - s.load) * 0.18).clamp(-0.45, pitch_limit));
        pitch = pitch.max(((-3.2 - s.load) * 0.18).clamp(-pitch_limit, 0.45));
    }
    pitch = pitch.clamp(-pitch_limit, pitch_limit);
    let high_alpha = (1. - (s.alpha.abs() / 0.9).powi(2)).clamp(0.18, 1.);
    let roll_max = config::table(
        q,
        &[0., 1500., 6000., 30000., 80000.],
        &[0.5, 1., 2.8, 3.2, 2.0],
    ) * high_alpha
        * (1. - 0.45 * configuration);
    let roll = -pilot.z * roll_max;
    let coordinated = if ground {
        0.
    } else {
        -9.80665 * f.roll().sin() / speed
    };
    let yaw = pilot.y * 0.45 - s.beta * 0.85 + coordinated + roll * s.alpha.sin();
    let target = DVec3::new(pitch, yaw.clamp(-0.6, 0.6), roll * s.alpha.cos());
    let error = target - s.rates;
    if !ground && !saturated {
        law.integral = (law.integral + error * dt).clamp(DVec3::splat(-0.18), DVec3::splat(0.18));
    } else {
        law.integral *= (-dt / 0.35).exp();
    }
    let gain = if ground {
        DVec3::new(1.4, 1.8, 2.)
    } else {
        DVec3::new(2.8, 2.5, 3.0)
    };
    let acceleration =
        (error * gain + law.integral * 0.4).clamp(DVec3::splat(-6.), DVec3::splat(6.));
    Demand {
        acceleration,
        rates: target,
        load,
        alpha_limit,
        protected: protection,
    }
}
