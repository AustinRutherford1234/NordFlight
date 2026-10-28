//! SI parameters. Values without published measurements are engineering estimates.
pub const AREA: f64 = 46.5;
pub const SPAN: f64 = 13.62;
pub const CHORD: f64 = 3.76;
pub const EMPTY: f64 = 14550.;
pub const CREW: f64 = 180.;
pub const PYLONS: f64 = 160.;
pub const MIL: f64 = 62500.;
pub const MAX: f64 = 97860.; // 22,000 lbf per engine, NAVAIR.
pub fn table(x: f64, axis: &[f64], values: &[f64]) -> f64 {
    let x = x.clamp(axis[0], axis[axis.len() - 1]);
    let i = axis
        .windows(2)
        .position(|a| x <= a[1])
        .unwrap_or(axis.len() - 2);
    let u = (x - axis[i]) / (axis[i + 1] - axis[i]);
    let u = u * u * (3. - 2. * u);
    values[i] + (values[i + 1] - values[i]) * u
}
