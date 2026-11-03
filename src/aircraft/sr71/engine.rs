use super::{config::*, math::*};
use crate::physics::Flight;
pub fn engine(power: f64, rho: f64, mach: f64) -> (f64, f64) {
    if !power.is_finite() || !rho.is_finite() || !mach.is_finite() || rho < 0. || mach < 0. {
        return (0., 0.);
    }
    let power = power.clamp(0., 1.);
    let lapse = (rho.max(0.) / ENGINE.reference_density).powf(ENGINE.density_exponent);
    let dry = ENGINE.static_dry_n * table(&ENGINE.mach, &ENGINE.dry_factor, mach) * lapse;
    let wet = ENGINE.static_wet_n * table(&ENGINE.mach, &ENGINE.wet_factor, mach) * lapse;
    let burner = (wet - dry).max(0.)
        * ((power - ENGINE.burner_threshold) / (1. - ENGINE.burner_threshold)).clamp(0., 1.);
    (
        dry * (power / ENGINE.burner_threshold)
            .clamp(0., 1.)
            .powf(ENGINE.dry_power_exponent)
            + burner,
        burner,
    )
}
pub fn recovery(f: &Flight) -> f64 {
    let angular = ENGINE.angular_alpha_loss
        * smooth(
            REGIMES.inlet_alpha_rad[0],
            REGIMES.inlet_alpha_rad[1],
            f.alpha.abs(),
        )
        + ENGINE.angular_beta_loss
            * smooth(
                REGIMES.inlet_beta_rad[0],
                REGIMES.inlet_beta_rad[1],
                f.beta.abs(),
            );
    let desired = ENGINE.spike_travel_m
        * ((f.mach - ENGINE.spike_start_mach) / ENGINE.spike_mach_interval).clamp(0., 1.);
    let mismatch = (f.systems.sr71.spike - desired).abs() / ENGINE.spike_travel_m;
    (1. - angular
        - ENGINE.spike_mismatch_loss
            * mismatch
            * smooth(
                REGIMES.inlet_mismatch_mach[0],
                REGIMES.inlet_mismatch_mach[1],
                f.mach,
            ))
    .clamp(ENGINE.minimum_recovery, 1.)
}
pub fn installed(f: &Flight, rho: f64, mach: f64) -> (f64, f64) {
    let (t, b) = engine(f.spool, rho, mach);
    let r = recovery(f);
    (t * r, b * r)
}
pub fn fuel_flow(f: &Flight) -> f64 {
    if !f.engine || (!f.infinite_fuel && f.fuel <= 0.) {
        return 0.;
    }
    let (t, _) = installed(f, f.density, f.mach);
    let fraction = (0..2)
        .map(|i| {
            if f.systems.engine_on[i] {
                f.systems.engines[i]
            } else {
                0.
            }
        })
        .sum::<f64>()
        * 0.5;
    t * fraction
        * (ENGINE.dry_tsfc_kg_n_s
            + ENGINE.burner_tsfc_increment
                * ((f.spool - ENGINE.burner_threshold) / (1. - ENGINE.burner_threshold))
                    .clamp(0., 1.))
}
