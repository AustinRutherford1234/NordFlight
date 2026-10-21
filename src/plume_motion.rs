//! Small, deterministic presentation fluctuations; never consume physics RNG.
use bevy::prelude::*;

pub(crate) struct Motion {
    pub length: f32,
    pub width: f32,
    pub radiance: f32,
    pub bend: Quat,
}

fn noise(time: f64, seed: u32) -> f32 {
    let tick = time.floor();
    let value = |tick: f64| {
        let mut h = (tick as i64 as u64) ^ (seed as u64).wrapping_mul(0x9e3779b97f4a7c15);
        h = (h ^ (h >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        h = (h ^ (h >> 27)).wrapping_mul(0x94d049bb133111eb);
        ((h ^ (h >> 31)) >> 40) as f32 / 8388607.5 - 1.
    };
    let u = (time - tick) as f32;
    let smooth = u * u * (3. - 2. * u);
    value(tick) * (1. - smooth) + value(tick + 1.) * smooth
}

pub(crate) fn sample(simulation_time: f64, seed: u32) -> Motion {
    let n = |offset: u32| {
        0.50 * noise(simulation_time * 7.3, seed.wrapping_add(offset))
            + 0.32 * noise(simulation_time * 19.7, seed.wrapping_add(offset + 11))
            + 0.18 * noise(simulation_time * 41.3, seed.wrapping_add(offset + 23))
    };
    Motion {
        length: 1. + 0.09 * n(0),
        width: 1. + 0.045 * n(37),
        radiance: 1. + 0.08 * n(71),
        bend: Quat::from_rotation_x(0.009 * n(109))
            * Quat::from_rotation_y(0.009 * n(151)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn motion_is_bounded_repeatable_and_independent_between_layers() {
        for t in (0..500).map(|i| i as f64 * 0.017) {
            let a = sample(t, 0);
            let b = sample(t, 0);
            assert_eq!((a.length, a.width, a.radiance, a.bend), (b.length, b.width, b.radiance, b.bend));
            assert!((0.91..=1.09).contains(&a.length));
            assert!((0.955..=1.045).contains(&a.width));
            assert!((0.92..=1.08).contains(&a.radiance));
            assert!((a.bend * Vec3::Z).distance(Vec3::Z) < 0.014);
        }
        assert_ne!(sample(1., 0).length, sample(1.03, 0).length);
        assert_ne!(sample(1., 0).length, sample(1., 1).length);
        // Keep subframe variation even after long missions; don't cast time to f32.
        assert_ne!(sample(1e8, 0).length, sample(1e8 + 0.017, 0).length);
    }
}
