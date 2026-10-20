//! Render distant luminous bodies without clipping them at the flight far plane.
use bevy::prelude::*;

pub const SUN_RADIUS: f32 = 696_340_000.0;

/// Place a proxy on the exact eye-to-Sun ray, scaling its radius by the same
/// factor. Angular size and geometric occultation by nearer planets are kept;
/// this does not move the physical Sun or alter any lighting/orbital calculation.
pub fn sun_transform(eye: Vec3, sun: Vec3, far: f32) -> Transform {
    let offset = sun - eye;
    let distance = offset.length();
    let proxy_distance = distance.min(far * 0.75);
    let scale = proxy_distance / distance;
    Transform::from_translation(eye + offset * scale).with_scale(Vec3::splat(scale))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sun_proxy_preserves_direction_angular_size_and_fits_inside_far_plane() {
        for far in [45_000_000., 768_000_000., 4_600_000_000., 7.5e11] {
            let eye = Vec3::new(23., -8., 42.);
            let sun = Vec3::new(8e10, 4e10, -1.1e11);
            let pose = sun_transform(eye, sun, far);
            let actual = sun - eye;
            let proxy = pose.translation - eye;
            assert!(actual.normalize().distance(proxy.normalize()) < 2e-7);
            let angular = SUN_RADIUS / actual.length();
            assert!((SUN_RADIUS * pose.scale.x / proxy.length() - angular).abs() < 1e-8);
            assert!(proxy.length() + SUN_RADIUS * pose.scale.x < far);
        }
    }
    #[test]
    fn flight_proxy_stays_behind_earth_and_moon_for_occultation() {
        let pose = sun_transform(Vec3::ZERO, Vec3::Y * 1.496e11, 768_000_000.);
        assert!(pose.translation.length() - SUN_RADIUS * pose.scale.x > 400_000_000.);
    }
}
