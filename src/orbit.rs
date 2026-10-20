//! Osculating two-body elements from inertial velocity. These describe the
//! instantaneous orbit; drag, thrust and third-body forces change it over time.
use crate::physics::{Flight, MU, RADIUS, rotation};
use bevy::math::DVec3;

#[derive(Clone, Copy, Debug)]
pub struct Orbit {
    pub mu: f64,
    pub radius: f64,
    pub periapsis: f64,
    pub apoapsis: Option<f64>,
    pub period: Option<f64>,
    pub eccentricity: f64,
    pub speed: f64,
    pub angular_momentum: DVec3,
    pub eccentricity_vector: DVec3,
    pub semi_latus_rectum: f64,
}
impl Orbit {
    pub fn from_flight(f: &Flight) -> Self {
        if f.vehicle == crate::physics::Vehicle::Spacecraft {
            let (r, v) = crate::space::relative(f);
            Self::from_state_about(r, v, f.space.body.mu(), f.space.body.radius())
        } else {
            Self::from_state(f.position, f.velocity + rotation().cross(f.position))
        }
    }
    pub fn from_state(position: DVec3, velocity: DVec3) -> Self {
        Self::from_state_about(position, velocity, MU, RADIUS)
    }
    pub fn from_state_about(position: DVec3, velocity: DVec3, mu: f64, body_radius: f64) -> Self {
        let radius = position.length().max(1.0);
        let momentum = position.cross(velocity);
        let eccentricity_vector = velocity.cross(momentum) / mu - position / radius;
        let eccentricity = eccentricity_vector.length();
        let energy = velocity.length_squared() * 0.5 - mu / radius;
        let p = momentum.length_squared() / mu;
        let a = -mu / (2.0 * energy);
        let bounded = energy < -1e-8 && eccentricity < 1.0;
        Self {
            mu,
            radius: body_radius,
            periapsis: p / (1.0 + eccentricity) - body_radius,
            apoapsis: bounded.then_some(a * (1.0 + eccentricity) - body_radius),
            period: bounded.then_some(std::f64::consts::TAU * (a.powi(3) / mu).sqrt()),
            eccentricity,
            speed: velocity.length(),
            angular_momentum: momentum,
            eccentricity_vector,
            semi_latus_rectum: p,
        }
    }
    pub fn point(self, anomaly: f64, current_position: DVec3) -> Option<DVec3> {
        let normal = self.angular_momentum.normalize_or_zero();
        if normal == DVec3::ZERO {
            return None;
        }
        let x = if self.eccentricity > 1e-6 {
            self.eccentricity_vector.normalize()
        } else {
            current_position.normalize()
        };
        let y = normal.cross(x);
        let denominator = 1.0 + self.eccentricity * anomaly.cos();
        if denominator < 0.05 {
            return None;
        }
        Some((x * anomaly.cos() + y * anomaly.sin()) * self.semi_latus_rectum / denominator)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn starting_spacecraft_is_circular_prograde_and_coplanar_with_moon() {
        let f = Flight::spacecraft();
        let (mp, mv, _, _) = crate::space::moon_inertial(f.elapsed);
        let moon_normal = mp.cross(mv).normalize();
        let orbit = Orbit::from_flight(&f);
        assert!(orbit.angular_momentum.normalize().dot(moon_normal) > 1.0 - 1e-12);
        assert!(f.position.dot(moon_normal).abs() < 1e-6);
        assert!(orbit.eccentricity < 1e-12);
        assert!((orbit.periapsis - 220000.0).abs() < 1e-5);
        let (_, velocity) = crate::space::relative(&f);
        assert!(f.forward().dot(velocity.normalize()) > 1.0 - 1e-12);
    }
    #[test]
    fn circular_orbit_has_equal_apsides() {
        let r = RADIUS + 220000.0;
        let o = Orbit::from_state(DVec3::Y * r, DVec3::X * (MU / r).sqrt());
        assert!((o.periapsis - 220000.0).abs() < 1e-6);
        assert!((o.apoapsis.unwrap() - 220000.0).abs() < 1e-6);
        assert!(o.eccentricity < 1e-10 && o.period.unwrap() > 5000.0);
        for i in 0..24 {
            assert!(
                (o.point(i as f64 / 24.0 * std::f64::consts::TAU, DVec3::Y * r)
                    .unwrap()
                    .length()
                    - r)
                    .abs()
                    < 1e-6
            );
        }
    }
    #[test]
    fn escape_trajectory_has_no_apoapsis() {
        let r = RADIUS + 220000.0;
        let o = Orbit::from_state(DVec3::Y * r, DVec3::X * (2.2 * MU / r).sqrt());
        assert!(o.apoapsis.is_none() && o.period.is_none() && o.eccentricity > 1.0);
    }
    #[test]
    fn orbital_energy_tracks_coasting_spacecraft() {
        let f = Flight::spacecraft();
        let o = Orbit::from_flight(&f);
        assert!((o.speed - (MU / f.position.length()).sqrt()).abs() < 1e-8);
        assert!(o.periapsis > 219999.0);
    }
}
