//! F-22 side-bay kinematics. Visible hardware follows public photographs;
//! dimensions and motion timing are presentation estimates, not production data.
use bevy::prelude::*;
/// Body-space displacement of the complete bay, clear of the inlet throat.
pub const OFFSET: Vec3 = Vec3::new(0., 0.06, 1.60);
pub const BACK: f32 = 1.46;
pub const MISSILE_ROLL: f32 = std::f32::consts::FRAC_PI_4;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct State {
    pub door: f64,
    pub launcher: f64,
}
impl State {
    pub fn ready(self) -> bool {
        self.door >= 1. - 1e-9 && self.launcher >= 1. - 1e-9
    }
    pub fn advance(&mut self, open: bool, powered: bool, dt: f64) {
        if !powered || dt <= 0. {
            return;
        }
        // Do not spend one large step twice: both directions stop at phase boundaries.
        let mut remaining = dt;
        if open {
            let used = remaining.min((1. - self.door) / 3.2);
            self.door = (self.door + used * 3.2).min(1.);
            remaining -= used;
            if self.door >= 1. - 1e-9 {
                self.launcher = (self.launcher + remaining * 2.8).min(1.);
            }
        } else {
            let used = remaining.min(self.launcher / 2.8);
            self.launcher = (self.launcher - used * 2.8).max(0.);
            remaining -= used;
            if self.launcher <= 1e-9 {
                self.door = (self.door - remaining * 3.2).max(0.);
            }
        }
    }
}
pub fn side(index: usize) -> f32 {
    if index == 0 { -1. } else { 1. }
}
pub fn smooth(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    t * t * (3. - 2. * t)
}
pub fn launcher_pivot(index: usize, z: f32) -> Vec3 {
    let s = side(index);
    Vec3::new(s * 1.53, -0.18, -1.74) + OFFSET + Vec3::Z * z
}
pub fn launcher_joint(index: usize, z: f32, extension: f64) -> Vec3 {
    let a = launcher_pivot(index, z);
    let b = rail_pose(index, extension).transform_point(Vec3::new(0., 0.155, z));
    let d = b - a;
    let perpendicular = (Vec3::Z - d * (d.z / d.length_squared())).normalize();
    let height = (0.72_f32.powi(2) - d.length_squared() * 0.25)
        .max(0.)
        .sqrt();
    (a + b) * 0.5 + perpendicular * height * if z < 0. { -1. } else { 1. }
}
pub fn rail_pose(index: usize, extension: f64) -> Transform {
    let t = smooth(extension as f32);
    let s = side(index);
    // Fore/aft folding supports carry a level rail. Their two fixed-length
    // segments fold longitudinally inside the cavity (launcher_joint).
    Transform::from_translation(Vec3::new(s * (1.64 + 1.17 * t), -0.32 - 0.10 * t, -1.74)+OFFSET)
        .with_rotation(Quat::from_rotation_y(s * 0.08*t))
}
pub fn missile_pose(index:usize,extension:f64)->Transform {
    // Clock the round's fins into an X so the full, unscaled missile clears
    // the narrow bay. The launcher remains upright on its suspension links.
    let mut t=rail_pose(index,extension);t.rotation*=Quat::from_rotation_z(MISSILE_ROLL);t
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opening_and_closing_are_ordered_and_timestep_independent() {
        for dt in [0.001_f64, 1. / 120., 0.1, 1.] {
            let mut a = State::default();
            let mut total = 0.;
            while total < 1. {
                let step = dt.min(1. - total);
                a.advance(true, true, step);
                total += step;
                assert!(a.launcher <= 1e-9 || a.door >= 1. - 1e-9);
            }
            assert!(a.ready());
            let saved = a;
            a.advance(false, false, 10.);
            assert_eq!(a, saved);
            a.advance(false, true, 1.);
            assert_eq!(a, State::default());
        }
        let mut a = State::default();
        a.advance(true, true, 0.4);
        let mut b = State::default();
        for _ in 0..40 {
            b.advance(true, true, 0.01);
        }
        assert!((a.door - b.door).abs() < 1e-10 && (a.launcher - b.launcher).abs() < 1e-10);
    }
}
