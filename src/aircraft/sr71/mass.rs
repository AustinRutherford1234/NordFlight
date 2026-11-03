use super::config::*;
use crate::physics::Flight;
use bevy::math::{DMat3, DVec3};
pub fn inertia(f: &Flight) -> DMat3 {
    let k = MASS.slug_ft2_to_kg_m2;
    let fuel = f.fuel.max(0.);
    DMat3::from_cols(
        DVec3::new(
            MASS.pitch_inertia * k + fuel * MASS.fuel_inertia_m2[0],
            0.,
            0.,
        ),
        DVec3::new(
            0.,
            MASS.yaw_inertia * k + fuel * MASS.fuel_inertia_m2[1],
            -MASS.product_inertia * k,
        ),
        DVec3::new(
            0.,
            -MASS.product_inertia * k,
            MASS.roll_inertia * k + fuel * MASS.fuel_inertia_m2[2],
        ),
    )
}
pub fn cg_fraction(f: &Flight) -> f64 {
    (MASS.zero_fuel_cg_mac * EMPTY + MASS.fuel_cg_mac * f.fuel.max(0.)) / (EMPTY + f.fuel.max(0.))
}
