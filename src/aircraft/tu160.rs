//! Reduced-order Tu-160 airborne plant. Geometry/mass envelope follows public
//! descriptions; polars, inertia, actuator and FBW gains are engineering estimates.
//! Commands drive rate-limited surfaces and engine spool, never aircraft poses.
use crate::physics::*;
use bevy::math::{DQuat, DVec3};
pub const AREA: f64 = 300.; // effective area at the fixed intermediate-sweep visual pose
pub const SPAN: f64 = 42.97;
pub const DRY: f64 = 110_400.;
pub const INERTIA: DVec3 = DVec3::new(35e6, 50e6, 15e6);
pub fn mass(f: &Flight) -> f64 {
    DRY + f.fuel + f.bomber_stores as f64 * 1500.
}
pub fn trim_alpha(mass: f64, rho: f64, speed: f64) -> f64 {
    let cl = (mass * 9.80665 / (0.5 * rho * speed * speed * AREA)).clamp(-1.15, 1.15);
    ((cl / 1.25).atanh() * 1.25 - 0.12) / 4.5
}
pub fn initialize(f: &mut Flight, stores: u8) {
    f.ai_bomber = true;
    f.bomber_stores = stores;
    f.fuel = 40_000.;
    f.flaps = 0.;
    f.gear = false;
    f.gear_extension = 0.;
    f.landed = false;
    f.rates = DVec3::ZERO;
    let speed = f.velocity.length();
    let rho = atmosphere(f.altitude()).2;
    f.orientation =
        (f.orientation * DQuat::from_rotation_x(trim_alpha(mass(f), rho, speed))).normalize();
    f.surface = DVec3::ZERO;
    f.spool = 0.36;
    f.throttle = 0.36;
    f.assist = true;
}
/// Mission pilot: bounded bank and vertical-path requests; FBW is in `step`.
pub fn pilot(f: &mut Flight, desired: DVec3, speed: f64, dt: f64) {
    let up = f.up();
    let ahead = (f.forward() - up * f.forward().dot(up)).normalize_or(f.forward());
    let right = ahead.cross(up);
    let heading = desired.dot(right).atan2(desired.dot(ahead));
    let stall = f.airspeed < 125. || f.alpha > 0.23;
    let bank = if stall {
        0.
    } else {
        (heading * 0.9).clamp(-0.61, 0.61)
    };
    let path = f
        .velocity
        .dot(up)
        .atan2((f.velocity - up * f.velocity.dot(up)).length().max(1.));
    let demand = if stall {
        -0.06
    } else {
        desired.dot(up).clamp(-0.12, 0.12).asin()
    };
    let load = (1. / f.roll().cos().max(0.75) + (demand - path) * 3.).clamp(0.3, 2.0);
    let request = DVec3::new(
        (load - 1.) / 1.5,
        0.,
        ((bank - f.roll()) * 1.4).clamp(-1., 1.),
    );
    f.controls += (request - f.controls) * (1. - (-dt / 0.35).exp());
    f.throttle = (0.35 + (speed - f.airspeed) * 0.008 + path * 0.8).clamp(0.08, 0.92);
    f.autopilot = false;
}
pub fn step(f: &mut Flight, e: &Environment, dt: f64) {
    f.elapsed += dt;
    crate::combat::damage::advance(&mut f.damage, &mut f.integrity, dt, f.engine, f.fuel > 0.);
    if crate::combat::damage::fatal(&f.damage, f.integrity) {
        crate::collision::destroy(f, "BOMBER BATTLE DAMAGE", f.up());
        return;
    }
    f.wind = e.wind(f.position);
    let air = f.velocity - f.wind;
    let speed = air.length().max(0.1);
    let b = f.orientation.conjugate() * air;
    let alpha = (-b.y).atan2(-b.z);
    let beta = (b.x / speed).clamp(-1., 1.).asin();
    let (temp, pressure, rho, sound) = atmosphere(f.altitude());
    let q = 0.5 * rho * speed * speed;
    let chord = AREA / SPAN;
    let m = mass(f);
    f.alpha = alpha;
    f.beta = beta;
    f.airspeed = speed;
    f.density = rho;
    f.temperature = temp;
    f.pressure = pressure;
    f.mach = speed / sound;
    let health = (f.damage.surfaces[0] + f.damage.surfaces[4]) * 0.5;
    let authority = f.damage.hydraulics;
    let load = (1. + f.controls.x * 1.5).clamp(0.3, 2.2);
    let target_alpha = trim_alpha(m, rho, speed) * load + 0.12 / 4.5 * (load - 1.);
    let heading_rate = 9.80665 * f.roll().tan() / speed.max(125.);
    let target_rate = DVec3::new(
        ((target_alpha - alpha) * 1.0 + heading_rate * f.roll().sin()).clamp(-0.12, 0.12),
        -heading_rate * f.roll().cos(),
        -f.controls.z * 0.35,
    );
    let natural = DVec3::new(
        -0.65 * (alpha - 0.03) - 18. * f.rates.x * chord / (2. * speed),
        -0.12 * beta - 1.6 * f.rates.y * SPAN / (2. * speed),
        -1.2 * f.rates.z * SPAN / (2. * speed),
    );
    let scale = q * AREA * DVec3::new(chord, SPAN, SPAN);
    let gain = DVec3::new(1.1, 0.22, 0.18);
    let demand = if f.assist && f.damage.avionics > 0.25 {
        ((INERTIA * (target_rate - f.rates) * 1.6 + f.rates.cross(INERTIA * f.rates))
            / scale.max(DVec3::splat(1.))
            - natural)
            / gain
    } else {
        DVec3::new(f.controls.x, f.controls.y, -f.controls.z) * 0.3
    };
    let request = demand.clamp(DVec3::splat(-0.45), DVec3::splat(0.45));
    f.surface += (request - f.surface).clamp(
        DVec3::splat(-0.28 * dt * authority),
        DVec3::splat(0.28 * dt * authority),
    );
    let moment = scale * (natural + gain * f.surface * health);
    let acceleration = (moment - f.rates.cross(INERTIA * f.rates)) / INERTIA;
    f.rates += acceleration * dt;
    f.orientation = (f.orientation * DQuat::from_scaled_axis(f.rates * dt)).normalize();
    let attached = (1. - ((alpha.abs() - 0.23).max(0.) / 0.5)).clamp(0.2, 1.);
    let cl = 1.25 * ((0.12 + 4.5 * alpha) / 1.25).tanh() * attached * health;
    let cd = 0.023
        + 0.075 * cl * cl
        + 0.25 * beta * beta
        + 0.14 * ((f.mach - 0.85).max(0.)).powi(2)
        + if attached < 0.8 {
            0.15 * (1. - attached)
        } else {
            0.
        };
    let dir = air / speed;
    let span = f.orientation * DVec3::X;
    let lift = span.cross(dir).normalize_or(f.orientation * DVec3::Y);
    let powered = f.engine && f.fuel > 0.;
    let target = if powered { f.throttle } else { 0. };
    f.spool += (target - f.spool) * (1. - (-dt / 2.8).exp());
    let lapse = (rho / 1.225).powf(0.7) * (1. - 0.15 * f.mach).max(0.5);
    let thrust = 4.
        * (137200. * (f.spool / 0.85).min(1.).powi(1)
            + 107800. * ((f.spool - 0.85) / 0.15).max(0.))
        * lapse
        * f.damage.engine;
    f.thrust = if powered { thrust } else { 0. };
    f.fuel = (f.fuel - f.thrust * 0.000015 * dt).max(0.);
    let aero = (lift * cl - dir * cd - span * (0.8 * beta)) * q * AREA;
    let force = aero + f.forward() * f.thrust;
    let omega = rotation();
    let acc = force / m
        - f.position * (MU / f.position.length().powi(3))
        - 2. * omega.cross(f.velocity)
        - omega.cross(omega.cross(f.position));
    f.g_load = force.dot(f.orientation * DVec3::Y) / (m * 9.80665);
    f.velocity += acc * dt;
    f.position += f.velocity * dt;
    if !f.position.is_finite()
        || !f.velocity.is_finite()
        || !f.orientation.is_finite()
        || f.agl() < 1.
    {
        crate::collision::destroy(f, "BOMBER IMPACT / LOSS OF CONTROL", f.up());
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Flight, Environment) {
        let mut f = Flight::approach();
        f.position = surface_point(0., -1500., 3800.);
        f.velocity = DVec3::NEG_Z * 240.;
        f.orientation = DQuat::IDENTITY;
        f.paused = false;
        f.engine = true;
        initialize(&mut f, 2);
        let mut e = Environment::default();
        e.wind_enabled = false;
        (f, e)
    }
    fn run(hz: f64, turn: bool, seconds: f64) -> Flight {
        let (mut f, e) = fixture();
        for _ in 0..(hz * seconds) as usize {
            let up = f.up();
            let ahead = (f.forward() - up * f.forward().dot(up)).normalize();
            let right = ahead.cross(up);
            let error = (3800. - f.altitude()) / 15000.;
            let desired = (if turn {
                ahead * 0.8 + right * 0.6
            } else {
                DVec3::NEG_Z - up * DVec3::NEG_Z.dot(up)
            } + up * error)
                .normalize();
            pilot(&mut f, desired, 240., 1. / hz);
            simulate(&mut f, &e, 1. / hz);
            assert!(
                !f.crashed,
                "h={} alt={} alpha={} g={}",
                hz,
                f.altitude(),
                f.alpha,
                f.g_load
            );
            assert!(f.rates.length() < 0.8, "rates {}", f.rates);
            assert!(f.g_load < 2.7 && f.g_load > -0.3, "g {}", f.g_load);
        }
        f
    }
    #[test]
    fn heavy_bomber_level_and_banked_flight_are_bounded() {
        for turn in [false, true] {
            let f = run(120., turn, 180.);
            println!(
                "tu160 turn={turn} altitude={} speed={} bank={} g={} alpha={}",
                f.altitude(),
                f.airspeed,
                f.roll(),
                f.g_load,
                f.alpha
            );
            assert!((3300. ..4300.).contains(&f.altitude()));
            assert!((200. ..280.).contains(&f.airspeed));
            assert!(f.fuel < 40000.);
        }
    }
    #[test]
    fn bomber_fbw_converges_across_physics_rates() {
        let a = run(60., true, 60.);
        let b = run(120., true, 60.);
        let c = run(240., true, 60.);
        for f in [&a, &c] {
            assert!(f.position.distance(b.position) < 60.);
            assert!((f.airspeed - b.airspeed).abs() < 2.);
            assert!((f.g_load - b.g_load).abs() < 0.04);
        }
    }
    #[test]
    fn engine_shutdown_removes_power_and_raw_surfaces_remain_physical() {
        let (mut f, e) = fixture();
        let before = f.fuel;
        f.engine = false;
        f.assist = false;
        f.controls.x = 0.5;
        simulate(&mut f, &e, 1. / 120.);
        assert_eq!(f.thrust, 0.);
        assert_eq!(f.fuel, before);
        assert!(f.surface.x > 0. && f.surface.x <= 0.28 / 120. + 1e-8);
        assert!(f.rates.x.abs() < 0.01);
    }
}
