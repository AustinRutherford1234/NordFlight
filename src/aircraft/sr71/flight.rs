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
pub mod engine;
#[path = "mass.rs"]
mod mass;
#[path = "math.rs"]
mod math;
#[path = "study.rs"]
pub mod study;
#[path = "trim.rs"]
mod trim;
pub use aero::evaluate;
#[cfg(test)]
use aero::{induced, steady};
pub use config::{AREA, CHORD, EMPTY, FUEL, INCIDENCE, SPAN};
#[cfg(test)]
use controls::control_coefficients;
pub use controls::{control_loads, target_rates, targets, tracking_acceleration};
pub use engine::{engine, fuel_flow, installed};
pub use mass::inertia;
pub use trim::initialize;
#[cfg(test)]
use trim::trim;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[path = "airframe_config.rs"]
pub mod airframe_config;
#[path = "database.rs"]
pub mod database;
#[path = "elastic.rs"]
pub mod elastic;
#[path = "elements.rs"]
pub mod elements;
#[path = "landing.rs"]
pub mod landing;
#[path = "plant.rs"]
pub mod plant;
#[cfg(test)]
#[path = "spatial_tests.rs"]
mod spatial_tests;
#[path = "unsteady.rs"]
pub mod unsteady;
