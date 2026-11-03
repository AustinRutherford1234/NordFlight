use super::config::*;
use std::f64::consts::PI;
pub const DEG: f64 = 180. / PI;
pub fn smooth(a: f64, b: f64, x: f64) -> f64 {
    crate::aircraft::sr71::smooth((x - a) / (b - a))
}
pub fn table(axis: &[f64], values: &[f64], x: f64) -> f64 {
    let x = x.clamp(axis[0], axis[axis.len() - 1]);
    let i = axis
        .windows(2)
        .position(|a| x <= a[1])
        .unwrap_or(axis.len() - 2);
    let t = (x - axis[i]) / (axis[i + 1] - axis[i]);
    values[i] + (values[i + 1] - values[i]) * t
}
pub fn tab(values: &[f64; 9], mach: f64) -> f64 {
    table(&MACH, values, mach)
}
