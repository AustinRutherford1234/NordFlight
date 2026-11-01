//! Estimated tyre/contact response; published Super Hornet steering travel.
//! Forces oppose contact-point slip, share a friction circle, and never impose yaw.
use super::*;
#[cfg(test)]
use bevy::math::DVec2;
pub fn speed(f: &Flight) -> f64 {
    let v = f.velocity - crate::carrier::ground_velocity(f, f.position);
    let n = crate::carrier::normal(f, f.position);
    (v - n * v.dot(n)).length()
}
pub fn available(f: &Flight) -> bool {
    f.battery
        && f.damage.hydraulics * f.hornet.hydraulic_pressure > 0.1
        && !f.carrier.attached
        && f.carrier.launch.is_none()
}
pub fn high_active(f: &Flight) -> bool {
    f.hornet.nws_high && speed(f) < 8. && f.landed && available(f)
}
pub fn steer(f: &mut Flight, velocity: DVec3, forward: DVec3, right: DVec3, dt: f64) {
    let v = speed(f);
    // 75 degrees below walking pace, smoothly return to normal by 8 m/s.
    // Speed scheduling and rate feedback are estimates, not an FCC reproduction.
    let blend = ((8. - v) / 3.).clamp(0., 1.);
    let blend = blend * blend * (3. - 2. * blend);
    let normal = 22.5f64.to_radians() / (1. + (v / 20.).powi(2));
    let limit = normal
        + if high_active(f) {
            blend * (75f64.to_radians() - normal)
        } else {
            0.
        };
    let authority = (f.hornet.hydraulic_pressure * f.damage.hydraulics).clamp(0., 1.);
    let target = if f.carrier.attached || f.carrier.launch.is_some() {
        0.
    } else if !f.battery || authority < 0.1 {
        // Failed steering free-casters into local rolling direction.
        velocity
            .dot(right)
            .atan2(velocity.dot(forward).abs().max(0.5))
            .clamp(-1.3, 1.3)
    } else {
        (-f.controls.y * limit + f.rates.y * 0.12 * (v / 12.).clamp(0., 1.)).clamp(-limit, limit)
    };
    let rate = if f.battery && authority >= 0.1 {
        60f64.to_radians() * authority
    } else {
        0.6
    };
    f.hornet.nose_steering += (target - f.hornet.nose_steering).clamp(-rate * dt, rate * dt);
}
pub use crate::ground::tyre;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn combined_grip_is_bounded_and_dissipative_at_all_speeds() {
        for vx in [-80., -4., -0.001, 0., 0.001, 4., 80.] {
            for vy in [-15., -0.001, 0., 0.001, 15.] {
                for brake in [false, true] {
                    let f = tyre(vx, vy, 40000., 0.65, 0.016, brake, 3000., 5000., 1. / 240.);
                    assert!(f.length() <= 26000.00001);
                    assert!(f.dot(DVec2::new(vx, vy)) <= 1e-9);
                    assert!(f.x.abs() / 3000. / 240. <= vx.abs() + 1e-12);
                    assert!(f.y.abs() / 5000. / 240. <= vy.abs() + 1e-12);
                }
            }
        }
    }
    #[test]
    fn full_travel_speed_scheduling_and_catapult_lock() {
        let mut f = crate::carrier::scenario(true);
        f.carrier.attached = false;
        f.landed = true;
        f.hornet.nws_high = true;
        f.controls.y = 1.;
        let v = crate::carrier::ground_velocity(&f, f.position);
        f.velocity = v;
        for _ in 0..600 {
            let forward = f.forward();
            let right = f.orientation * DVec3::X;
            steer(&mut f, DVec3::ZERO, forward, right, 1. / 240.);
        }
        assert!((f.hornet.nose_steering.to_degrees() + 75.).abs() < 0.01);
        f.velocity = v + f.forward() * 25.;
        for _ in 0..600 {
            let forward = f.forward();
            let right = f.orientation * DVec3::X;
            steer(&mut f, DVec3::ZERO, forward, right, 1. / 240.);
        }
        assert!(f.hornet.nose_steering.abs() < 10f64.to_radians());
        f.carrier.attached = true;
        for _ in 0..600 {
            let forward = f.forward();
            let right = f.orientation * DVec3::X;
            steer(&mut f, DVec3::ZERO, forward, right, 1. / 240.);
        }
        assert_eq!(f.hornet.nose_steering, 0.);
    }
    #[test]
    fn idle_engines_keep_hydraulics_and_brakes_hold_on_the_moving_deck() {
        let mut f = crate::carrier::scenario(true);
        f.carrier.attached = false;
        f.apu = false;
        f.throttle = 0.;
        f.brakes = true;
        let e = Environment {
            wind_enabled: false,
            ..Default::default()
        };
        for _ in 0..7200 {
            simulate(&mut f, &e, 1. / 240.);
        }
        println!(
            "IDLE HOLD speed={} hydraulic={} steer={} compression={:?}",
            speed(&f),
            f.hornet.hydraulic_pressure,
            f.hornet.nose_steering,
            f.gear_compression
        );
        assert!(!f.crashed && f.landed && speed(&f) < 0.1);
        assert!(f.hornet.hydraulic_pressure > 0.9);
    }
    fn taxi(high: bool, sign: f64, hz: usize) -> (f64, Flight) {
        let mut f = Flight::runway();
        crate::aircraft::select(&mut f, crate::aircraft::Aircraft::Hornet);
        f.position = surface_point(0., 1050., RUNWAY_HEIGHT + 2.1);
        f.engine = false;
        f.assist = false;
        let e = Environment {
            wind_enabled: false,
            ..Default::default()
        };
        for _ in 0..hz * 3 {
            simulate(&mut f, &e, 1. / hz as f64);
        }
        f.velocity = f.forward() * 3.;
        f.hornet.nws_high = high;
        f.controls.y = sign;
        let mut yaw = 0.;
        for _ in 0..hz * 6 {
            simulate(&mut f, &e, 1. / hz as f64);
            yaw += f.rates.y / hz as f64;
            assert!(!f.crashed, "{}", f.status);
        }
        println!(
            "TAXI high={high} sign={sign} hz={hz} yaw={} speed={} roll={} steer={}",
            yaw.to_degrees(),
            speed(&f),
            f.roll().to_degrees(),
            f.hornet.nose_steering.to_degrees()
        );
        (yaw, f)
    }
    #[test]
    fn high_gain_turns_tightly_and_symmetrically_without_stationary_spin() {
        let (low, _) = taxi(false, 1., 240);
        let (high, f) = taxi(true, 1., 240);
        let (opposite, _) = taxi(true, -1., 240);
        assert!(high.abs() > low.abs() * 1.5, "low={low} high={high}");
        assert!(high.abs() > 0.7 && (high + opposite).abs() < 0.05);
        assert!(f.roll().abs() < 3f64.to_radians());
        let mut stopped = f;
        stopped.velocity = DVec3::ZERO;
        stopped.rates = DVec3::ZERO;
        stopped.brakes = true;
        for _ in 0..2400 {
            simulate(
                &mut stopped,
                &Environment {
                    wind_enabled: false,
                    ..Default::default()
                },
                1. / 240.,
            );
        }
        assert!(!stopped.crashed && stopped.rates.y.abs() < 0.01);
    }
    #[test]
    fn taxi_trajectory_converges_across_frame_rates() {
        let (reference, f) = taxi(true, 1., 240);
        for hz in [60, 120] {
            let (yaw, other) = taxi(true, 1., hz);
            assert!((yaw - reference).abs() < 0.02);
            assert!(other.position.distance(f.position) < 0.10);
        }
    }
}
