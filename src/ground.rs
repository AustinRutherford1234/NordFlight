//! Shared tyre forces for runway and moving-deck contacts.
use bevy::math::DVec2;
/// Reduced-order combined-slip tyre. x = longitudinal, y = lateral force.
/// Effective point masses bound the impulse at crawl speed (no brake reversal).
#[allow(clippy::too_many_arguments)]
pub fn tyre(
    longitudinal: f64,
    lateral: f64,
    load: f64,
    mu: f64,
    rolling: f64,
    brake: bool,
    mass_long: f64,
    mass_side: f64,
    dt: f64,
) -> DVec2 {
    tyre_with_brake(longitudinal,lateral,load,mu,rolling,brake,mass_long,mass_side,dt,0.60)
}
#[allow(clippy::too_many_arguments)]
pub fn tyre_with_brake(longitudinal:f64,lateral:f64,load:f64,mu:f64,rolling:f64,brake:bool,mass_long:f64,mass_side:f64,dt:f64,brake_effort:f64)->DVec2 {
    if load <= 0. || dt <= 0. {
        return DVec2::ZERO;
    }
    let limit = load * mu;
    let desired = DVec2::new(
        -load
            * (rolling * longitudinal.tanh()
                + if brake {
                    brake_effort * (longitudinal / 0.03).tanh()
                } else {
                    0.
                }),
        -load * 5. * lateral / longitudinal.abs().max(2.),
    );
    let mut force = desired / (desired.length() / limit.max(1e-9)).max(1.);
    force.x = force.x.clamp(
        -longitudinal.abs() * mass_long / dt,
        longitudinal.abs() * mass_long / dt,
    );
    force.y = force.y.clamp(
        -lateral.abs() * mass_side / dt,
        lateral.abs() * mass_side / dt,
    );
    force
}
