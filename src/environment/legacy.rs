//! Original terrain retained for reproducible regression and reference runs.
use crate::physics::*;
pub fn height(x: f64, z: f64) -> f64 {
    if crate::carrier::legacy_sea(x, z) {
        return -2.;
    }
    if in_lake(x, z) {
        return -2.0;
    }
    // A fictional Nordic island: rolling pine coast, granite ridges and a flattened airfield.
    if x.abs() < 310.0 && z.abs() < 1850.0 {
        return RUNWAY_HEIGHT;
    }
    let coast = 1.0;
    let hills = 55.0
        + 16.0 * (x * 0.00051 + 0.6).sin() * (z * 0.00039).cos()
        + 6.0 * (x * 0.0017 + z * 0.0008).sin()
        + 3.0 * (z * 0.0027 - x * 0.0011).cos();
    let ridge = 230.0
        * (-((x + 11500.0) / 6500.0).powi(2)).exp()
        * (0.45 + 0.55 * (z * 0.00019 + 0.9).sin().powi(2));
    let island = (hills + ridge) * coast - 20.0 * (1.0 - coast);
    // Separate inland BVR theater: broad mountain chains with navigable valleys.
    // Smoothly vanishes around the original airfield and close-range scenarios.
    let theater = ((x - 35000.) / 35000.).clamp(0., 1.);
    let ridges = (0.5 + 0.5 * (x * 0.000075 + z * 0.000018).sin()).powi(4);
    let mountains = theater * (450. + 2600. * ridges * (0.6 + 0.4 * (z * 0.000052).cos().powi(2)));
    let island = island + mountains;
    let detail = 0.30 * (x * 0.043 + z * 0.017).sin() * (z * 0.031 - x * 0.012).cos()
        + 1.4 * (x * 0.009 + z * 0.006).sin() * (z * 0.007).cos();
    let d = ((x.abs() - 310.0).max(0.0).powi(2) + (z.abs() - 1850.0).max(0.0).powi(2)).sqrt();
    let blend = (d / 700.0).clamp(0.0, 1.0);
    let h = RUNWAY_HEIGHT * (1.0 - blend) + (island + detail) * blend;
    // A broad, obstacle-free meadow offers a discoverable off-airfield landing site.
    let edge = ((x - 6500.).abs() - 210.)
        .max(0.)
        .max(((z - 3200.).abs() - 1000.).max(0.));
    let meadow = 1. - (edge / 320.).clamp(0., 1.);
    let meadow = meadow * meadow * (3. - 2. * meadow);
    h * (1. - meadow) + 62. * meadow
}
