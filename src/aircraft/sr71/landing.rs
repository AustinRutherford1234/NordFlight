//! Supporting-plane ground effect: every element sees its own clearance and sink velocity.
use super::{airframe_config::LANDING, config::SPAN};
use bevy::math::DVec3;
#[derive(Clone, Copy, Debug)]
pub struct Surface {
    pub normal_body: DVec3,
    pub distance_m: f64,
    pub relative_velocity_body: DVec3,
}
impl Surface {
    pub fn free() -> Self {
        Self {
            normal_body: DVec3::Y,
            distance_m: 10000.,
            relative_velocity_body: DVec3::ZERO,
        }
    }
    pub fn clearance(&self, point: DVec3) -> f64 {
        (self.distance_m + point.dot(self.normal_body)).max(0.)
    }
}
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct State {
    pub pressure: f64,
}
pub fn target(height: f64, mach: f64) -> f64 {
    (-height.max(0.) / (SPAN * LANDING.height_span_scale)).exp()
        * (1. - super::math::smooth(0.4, LANDING.low_speed_mach_end, mach))
}
pub fn advance(state: &mut State, target: f64, dt: f64) {
    state.pressure +=
        (target - state.pressure) * (1. - (-dt.max(0.) / LANDING.pressure_time_s).exp());
}
pub fn lift_gain(pressure: f64, velocity: DVec3, normal: DVec3, speed: f64) -> f64 {
    let sink = (-velocity.dot(normal) / speed.max(25.))
        .clamp(-LANDING.sink_ratio_limit, LANDING.sink_ratio_limit);
    1. + pressure * LANDING.maximum_lift_gain + pressure * sink * LANDING.sink_lift_gain
}
pub fn induced_factor(pressure: f64) -> f64 {
    1. - pressure * (1. - LANDING.minimum_induced_factor)
}
