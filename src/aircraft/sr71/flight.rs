//! SR-71 aircraft-plant facade. The shared flight loop remains the sole integrator.
//! Pure force evaluation, tuning, actuators, trim and diagnostics have separate ownership.
#[path = "aero.rs"]
mod aero;
#[path = "config.rs"]
pub mod config;
#[path = "controls.rs"]
mod controls;
#[path = "diagnostics.rs"]
pub mod diagnostics;
#[path = "engine.rs"]
mod engine;
#[path = "mass.rs"]
mod mass;
#[path = "math.rs"]
mod math;
#[path = "study.rs"]
pub mod study;
#[path = "trim.rs"]
mod trim;
#[cfg(test)]
use aero::steady;
pub use aero::{evaluate, induced};
pub use config::{AREA, CHORD, EMPTY, FUEL, INCIDENCE, SPAN};
#[cfg(test)]
use controls::control_coefficients;
pub use controls::{control_loads, target_rates, targets, tracking_acceleration};
pub use engine::{engine, fuel_flow, installed};
pub use mass::{cg_fraction, inertia};
pub use trim::initialize;
#[cfg(test)]
use trim::trim;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
