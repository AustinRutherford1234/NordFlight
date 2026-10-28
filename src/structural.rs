//! Reduced-order elastic lifting surfaces.
//!
//! All geometry, forces and kinematics use aircraft body coordinates and SI
//! units. The F-22 parameters are *estimates*, not aircraft structural data.
//! The model is intended for modest elastic motion, not failure or flutter
//! prediction. Its two modes per assembly are cantilever bending and twist.
use bevy::math::{DQuat, DVec3};
use std::sync::OnceLock;

pub const LEFT_WING: usize = 0;
pub const RIGHT_WING: usize = 1;
pub const LEFT_FIN: usize = 2;
pub const RIGHT_FIN: usize = 3;
pub const LEFT_TAIL: usize = 4;
pub const RIGHT_TAIL: usize = 5;
pub const SURFACE_COUNT: usize = 6;
/// Aircraft control-surface indices include one unused slot (8).
pub const PANEL_COUNT: usize = 11;

/// One elastic assembly. `span` and `normal` are perpendicular unit vectors;
/// `sweep` moves the elastic axis aft from root to tip without changing the
/// span coordinate. Bend is positive along `normal`, twist about `span`.
#[derive(Clone, Copy, Debug)]
pub struct SurfaceConfig {
    pub root: DVec3,
    pub span: DVec3,
    pub normal: DVec3,
    pub length: f64,
    pub sweep: DVec3,
    /// For a chordwise panel cantilever, direction of its supported hinge line.
    /// None describes an ordinary spanwise cantilever fixed at its root.
    pub hinge: Option<DVec3>,
    pub mass: f64,
    /// Offset of the surface mass from its elastic axis (body metres).
    pub mass_offset: DVec3,
    /// Effective modal mass (kg), inertia (kg m²), natural frequencies (Hz),
    /// and damping ratios. These are deliberately independent configuration.
    pub bending_mass: f64,
    pub torsion_inertia: f64,
    pub bending_hz: f64,
    pub torsion_hz: f64,
    pub bending_damping: f64,
    pub torsion_damping: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub surfaces: [SurfaceConfig; SURFACE_COUNT],
    /// Elastic modes of moving panels, expressed in neutral body geometry.
    pub panels: [Option<SurfaceConfig>; PANEL_COUNT],
}

pub fn f22_config() -> &'static Config {
    static CONFIG: OnceLock<Config> = OnceLock::new();
    CONFIG.get_or_init(Config::f22)
}

impl Config {
    pub fn f22() -> Self {
        let wing = |side: f64| SurfaceConfig {
            root: DVec3::new(side * 1.65, -0.04, 0.05),
            span: DVec3::X * side,
            normal: DVec3::Y,
            length: 5.13,
            sweep: DVec3::Z * 1.75, hinge: None,
            mass: 780., mass_offset: DVec3::Z * 0.20,
            bending_mass: 195., torsion_inertia: 250.,
            // A stiffer parent wing keeps tip slope close to ten degrees at
            // the nominal 15-G break condition; still allows visible, subtle
            // elastic motion below failure.
            bending_hz: 6.0, torsion_hz: 14.,
            bending_damping: 0.055, torsion_damping: 0.07,
        };
        let fin = |side: f64| SurfaceConfig {
            root: DVec3::new(side * 1.60, 0.44, 5.7),
            span: DVec3::new(side * 1.37, 2.48, 0.).normalize(),
            normal: DVec3::new(side * 2.48, -1.37, 0.).normalize(),
            length: DVec3::new(1.37, 2.48, 0.).length(),
            sweep: DVec3::Z * -0.4, hinge: None,
            mass: 150., mass_offset: DVec3::Z * 0.12,
            bending_mass: 38., torsion_inertia: 28.,
            bending_hz: 10., torsion_hz: 16.,
            bending_damping: 0.06, torsion_damping: 0.07,
        };
        let tail = |side: f64| SurfaceConfig {
            root: DVec3::new(side * 1.65, 0.02, 5.9),
            span: DVec3::X * side,
            normal: DVec3::Y,
            length: 2.77,
            sweep: DVec3::Z * 0.95, hinge: None,
            mass: 210., mass_offset: DVec3::Z * 0.12,
            bending_mass: 52., torsion_inertia: 65.,
            bending_hz: 8., torsion_hz: 15.,
            bending_damping: 0.06, torsion_damping: 0.07,
        };
        let panel = |root: DVec3, span: DVec3, normal: DVec3, length: f64, sweep: DVec3,
                     hinge: Option<DVec3>, mass: f64, inertia: f64| SurfaceConfig {
            root, span, normal, length, sweep, hinge, mass,
            mass_offset: DVec3::Z * 0.04,
            bending_mass: mass * 0.25, torsion_inertia: inertia,
            // Compact, hinge-supported fighter surfaces are modeled as
            // stiffer than their parent wing; values remain estimates, not
            // measured F-22 structural frequencies or damping data.
            bending_hz: 40., torsion_hz: 55.,
            bending_damping: 0.10, torsion_damping: 0.10,
        };
        let mut panels = [None; PANEL_COUNT];
        for (side, base) in [(-1., 0), (1., 4)] {
            let span = DVec3::X * side;
            panels[base] = Some(panel(DVec3::new(side * 1.65, 0.02, 5.9), span, DVec3::Y,
                2.77, DVec3::Z * 0.95, None, 130., 38.));
            // Hinges run along each panel's span. The independent deformation
            // is a chordwise cantilever fixed everywhere on that line.
            let hinged = |a: DVec3, b: DVec3, normal: DVec3, chord_sign: f64,
                          chord: f64, mass: f64, inertia: f64| {
                let hinge = (b-a).normalize();
                let nominal = DVec3::Z * chord_sign;
                let chord_dir = (nominal - hinge * nominal.dot(hinge)).normalize();
                panel((a+b)*0.5, chord_dir, normal, chord, DVec3::ZERO, Some(hinge), mass, inertia)
            };
            panels[base+1] = Some(hinged(DVec3::new(side*1.85,-0.04,2.60),DVec3::new(side*3.85,-0.04,2.30),
                DVec3::Y, 1., 1.1, 45., 9.));
            panels[base+2] = Some(hinged(DVec3::new(side*3.92,-0.04,2.28),DVec3::new(side*6.50,-0.04,1.90),
                DVec3::Y, 1., 0.8, 35., 6.));
            let f = fin(side);
            panels[base+3] = Some(hinged(DVec3::new(side*1.60,0.44,5.35),DVec3::new(side*2.97,2.92,4.95),
                f.normal, 1., 0.85, 35., 8.));
            let leading = if side < 0. { 9 } else { 10 };
            panels[leading] = Some(hinged(DVec3::new(side*1.65,-0.04,-2.45),DVec3::new(side*6.50,-0.04,1.90),
                DVec3::Y, -1., 0.55, 50., 8.));
        }
        Self { surfaces: [wing(-1.), wing(1.), fin(-1.), fin(1.), tail(-1.), tail(1.)], panels }
    }
}

/// An applied point force and optional pure moment, both in the aircraft frame.
#[derive(Clone, Copy, Debug)]
pub struct Load {
    pub surface: usize,
    pub point: DVec3,
    pub force: DVec3,
    pub moment: DVec3,
}

/// Signed aerodynamic bending moment about the root (N·m). Loads for other
/// assemblies are ignored; `point`, `force`, and the applied pure `moment`
/// are all in the same aircraft-body frame as the surface. With mirrored
/// wings and upward normals, positive loading has the same sign on both.
/// This is the root resultant of the supplied loads, not a material stress or
/// a complete stress tensor, and the caller chooses which loads to include.
pub fn root_bending_moment(surface: &SurfaceConfig, assembly: usize, loads: &[Load]) -> f64 {
    let axis = surface.span.cross(surface.normal).normalize_or_zero();
    loads.iter().filter(|load| load.surface == assembly).map(|load| {
        ((load.point - surface.root).cross(load.force) + load.moment).dot(axis)
    }).sum()
}

/// Signed linear-elastic bending stress (Pa) for a section modulus in m³.
/// No assumptions about damage, allowable stress, or failure are made here.
/// Invalid values return `None` so callers can handle them explicitly.
pub fn bending_stress(moment: f64, section_modulus: f64) -> Option<f64> {
    if !moment.is_finite() || !section_modulus.is_finite() || section_modulus <= 0. {
        return None;
    }
    let stress = moment / section_modulus;
    stress.is_finite().then_some(stress)
}

/// Force applied to a moving panel in its neutral, undeformed body frame.
/// Parent assembly load transfer remains the caller's responsibility.
#[derive(Clone, Copy, Debug)]
pub struct PanelLoad {
    pub panel: usize,
    pub point: DVec3,
    pub force: DVec3,
    pub moment: DVec3,
}

/// Rigid-body kinematics transformed into one panel's neutral frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct PanelMotion {
    pub specific_acceleration: DVec3,
    pub angular_velocity: DVec3,
    pub angular_acceleration: DVec3,
    pub mass_center: DVec3,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ModeState {
    /// Tip bending displacement in metres, tip twist in radians.
    pub bend: f64,
    pub bend_velocity: f64,
    pub twist: f64,
    pub twist_velocity: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct State {
    pub surfaces: [ModeState; SURFACE_COUNT],
    pub panel_modes: [ModeState; PANEL_COUNT],
}

impl SurfaceConfig {
    pub fn span_fraction(&self, point: DVec3) -> f64 {
        ((point - self.root).dot(self.span) / self.length).clamp(0., 1.)
    }
    pub fn axis_point(&self, fraction: f64) -> DVec3 {
        self.root + (self.span * self.length + self.sweep) * fraction.clamp(0., 1.)
    }
    fn axis_for_point(&self, point: DVec3, fraction: f64) -> DVec3 {
        self.axis_point(fraction) + self.hinge.map_or(DVec3::ZERO, |axis| axis * (point-self.root).dot(axis))
    }
}

// Deflection shape of a uniform cantilever under tip load. Root displacement
// and slope vanish; normalization makes the modal displacement the tip value.
fn bend_shape(s: f64) -> f64 { s * s * (3. - s) * 0.5 }
fn bend_slope(s: f64, length: f64) -> f64 { 1.5 * s * (2. - s) / length }

impl State {
    pub fn frame_rotation(&self, config: &Config, surface: usize, point: DVec3) -> DQuat {
        let Some(c) = config.surfaces.get(surface) else { return DQuat::IDENTITY; };
        let Some(m) = self.surfaces.get(surface) else { return DQuat::IDENTITY; };
        let s = c.span_fraction(point);
        let bend_axis = c.span.cross(c.normal).normalize_or_zero();
        let bending = DQuat::from_axis_angle(bend_axis, (m.bend * bend_slope(s, c.length)).atan());
        bending * DQuat::from_axis_angle(c.span, m.twist * s)
    }

    pub fn deform_point(&self, config: &Config, surface: usize, point: DVec3) -> DVec3 {
        let Some(c) = config.surfaces.get(surface) else { return point; };
        let Some(m) = self.surfaces.get(surface) else { return point; };
        let s = c.span_fraction(point);
        let axis = c.axis_point(s);
        let rotate = self.frame_rotation(config, surface, point);
        axis + c.normal * (m.bend * bend_shape(s)) + rotate * (point - axis)
    }

    /// Body-frame velocity of a material point due to elastic motion alone.
    /// Add this to rigid-body velocity when evaluating local relative airflow.
    pub fn point_velocity(&self, config: &Config, surface: usize, point: DVec3) -> DVec3 {
        let Some(c) = config.surfaces.get(surface) else { return DVec3::ZERO; };
        let Some(m) = self.surfaces.get(surface) else { return DVec3::ZERO; };
        let s = c.span_fraction(point);
        let slope = bend_slope(s, c.length);
        let angle = (m.bend * slope).atan();
        let bend_axis = c.span.cross(c.normal).normalize_or_zero();
        let bend_rotation = DQuat::from_axis_angle(bend_axis, angle);
        let bend_rate = m.bend_velocity * slope / (1. + (m.bend * slope).powi(2));
        let angular_velocity = bend_axis * bend_rate + (bend_rotation * c.span) * (m.twist_velocity * s);
        c.normal * (m.bend_velocity * bend_shape(s))
            + angular_velocity.cross(self.frame_rotation(config, surface, point) * (point - c.axis_point(s)))
    }

    /// Elastic panel deformation in its neutral geometry. Apply the actuator
    /// hinge rotation and then parent structural deformation separately.
    pub fn deform_panel_point(&self, config: &Config, panel: usize, point: DVec3) -> DVec3 {
        let Some(Some(c)) = config.panels.get(panel) else { return point; };
        let Some(m) = self.panel_modes.get(panel) else { return point; };
        deform_for(c, m, point)
    }
    pub fn panel_frame_rotation(&self, config: &Config, panel: usize, point: DVec3) -> DQuat {
        let Some(Some(c)) = config.panels.get(panel) else { return DQuat::IDENTITY; };
        let Some(m) = self.panel_modes.get(panel) else { return DQuat::IDENTITY; };
        frame_for(c, m, point)
    }
    pub fn panel_point_velocity(&self, config: &Config, panel: usize, point: DVec3) -> DVec3 {
        let Some(Some(c)) = config.panels.get(panel) else { return DVec3::ZERO; };
        let Some(m) = self.panel_modes.get(panel) else { return DVec3::ZERO; };
        velocity_for(c, m, point)
    }

    pub fn advance(&mut self, config: &Config, loads: &[Load], specific_acceleration: DVec3, dt: f64) {
        self.advance_with_motion(config, loads, specific_acceleration, DVec3::ZERO, DVec3::ZERO, DVec3::ZERO, dt);
    }

    /// `specific_acceleration` is acceleration of the aircraft mass centre
    /// minus gravitational acceleration, in body axes. The fictitious inertial
    /// loads also include Euler and centrifugal terms about that centre. The
    /// solver does not apply these internal loads again to the rigid aircraft.
    pub fn advance_with_motion(&mut self, config: &Config, loads: &[Load], specific_acceleration: DVec3,
        angular_velocity: DVec3, angular_acceleration: DVec3, mass_center: DVec3, dt: f64) {
        if !dt.is_finite() || dt <= 0. { return; }
        let mut generalized = [(0., 0.); SURFACE_COUNT];
        for load in loads {
            if let Some(c) = config.surfaces.get(load.surface) {
                if load.point.is_finite() && load.force.is_finite() && load.moment.is_finite() {
                    apply_load(c, &mut generalized[load.surface], load.point, load.force, load.moment);
                }
            }
        }
        // Three-point Gauss integration of distributed mass over the span.
        const STATIONS: [(f64, f64); 3] = [(0.1127016653792583, 5. / 18.), (0.5, 8. / 18.), (0.8872983346207417, 5. / 18.)];
        if specific_acceleration.is_finite() && angular_velocity.is_finite()
            && angular_acceleration.is_finite() && mass_center.is_finite() {
            for (i, c) in config.surfaces.iter().enumerate() {
                for (s, weight) in STATIONS {
                    let point = c.axis_point(s) + c.mass_offset;
                    let r = point - mass_center;
                    let acceleration = specific_acceleration + angular_acceleration.cross(r)
                        + angular_velocity.cross(angular_velocity.cross(r));
                    apply_load(c, &mut generalized[i], point, -acceleration * (c.mass * weight), DVec3::ZERO);
                }
            }
        }
        for (i, (bending_force, torsion_moment)) in generalized.into_iter().enumerate() {
            let c = &config.surfaces[i];
            let m = &mut self.surfaces[i];
            oscillator(&mut m.bend, &mut m.bend_velocity, bending_force, c.bending_mass, c.bending_hz, c.bending_damping, dt);
            oscillator(&mut m.twist, &mut m.twist_velocity, torsion_moment, c.torsion_inertia, c.torsion_hz, c.torsion_damping, dt);
        }
    }

    /// Advance independent modes for hinged surfaces. Loads are in the panel's
    /// neutral frame; rotation into that frame and attachment reaction transfer
    /// to the parent remain with the caller. The fixed mass offset and modal
    /// properties are simulation estimates, not F-22 engineering data.
    pub fn advance_panels(&mut self, config: &Config, loads: &[PanelLoad], specific_acceleration: DVec3,
        angular_velocity: DVec3, angular_acceleration: DVec3, mass_center: DVec3, dt: f64) {
        let motion = PanelMotion { specific_acceleration, angular_velocity, angular_acceleration, mass_center };
        self.advance_panels_with_motion(config, loads, &[motion; PANEL_COUNT], dt);
    }

    pub fn advance_panels_with_motion(&mut self, config: &Config, loads: &[PanelLoad],
        motions: &[PanelMotion; PANEL_COUNT], dt: f64) {
        if !dt.is_finite() || dt <= 0. { return; }
        let mut generalized = [(0., 0.); PANEL_COUNT];
        for load in loads {
            if let Some(Some(c)) = config.panels.get(load.panel) {
                if load.point.is_finite() && load.force.is_finite() && load.moment.is_finite() {
                    apply_load(c, &mut generalized[load.panel], load.point, load.force, load.moment);
                }
            }
        }
        const STATIONS: [(f64, f64); 3] = [(0.1127016653792583, 5. / 18.), (0.5, 8. / 18.), (0.8872983346207417, 5. / 18.)];
        for (i, config) in config.panels.iter().enumerate() {
            let Some(c) = config else { continue; };
            let PanelMotion { specific_acceleration, angular_velocity, angular_acceleration, mass_center } = motions[i];
            if specific_acceleration.is_finite() && angular_velocity.is_finite()
                && angular_acceleration.is_finite() && mass_center.is_finite() {
                for (s, weight) in STATIONS {
                    let point = c.axis_point(s) + c.mass_offset;
                    let r = point - mass_center;
                    let acceleration = specific_acceleration + angular_acceleration.cross(r)
                        + angular_velocity.cross(angular_velocity.cross(r));
                    apply_load(c, &mut generalized[i], point, -acceleration * (c.mass * weight), DVec3::ZERO);
                }
            }
        }
        for (i, (bending_force, torsion_moment)) in generalized.into_iter().enumerate() {
            let Some(c) = config.panels[i] else { continue; };
            let m = &mut self.panel_modes[i];
            oscillator(&mut m.bend, &mut m.bend_velocity, bending_force, c.bending_mass, c.bending_hz, c.bending_damping, dt);
            oscillator(&mut m.twist, &mut m.twist_velocity, torsion_moment, c.torsion_inertia, c.torsion_hz, c.torsion_damping, dt);
        }
    }
}

fn frame_for(c: &SurfaceConfig, m: &ModeState, point: DVec3) -> DQuat {
    let s = c.span_fraction(point);
    let bend_axis = c.span.cross(c.normal).normalize_or_zero();
    let bending = DQuat::from_axis_angle(bend_axis, (m.bend * bend_slope(s, c.length)).atan());
    bending * DQuat::from_axis_angle(c.span, m.twist * s)
}
fn deform_for(c: &SurfaceConfig, m: &ModeState, point: DVec3) -> DVec3 {
    let s = c.span_fraction(point);
    let axis = c.axis_for_point(point, s);
    axis + c.normal * (m.bend * bend_shape(s)) + frame_for(c, m, point) * (point - axis)
}
fn velocity_for(c: &SurfaceConfig, m: &ModeState, point: DVec3) -> DVec3 {
    let s = c.span_fraction(point);
    let slope = bend_slope(s, c.length);
    let bend_axis = c.span.cross(c.normal).normalize_or_zero();
    let bend_rotation = DQuat::from_axis_angle(bend_axis, (m.bend * slope).atan());
    let bend_rate = m.bend_velocity * slope / (1. + (m.bend * slope).powi(2));
    let angular_velocity = bend_axis * bend_rate + (bend_rotation * c.span) * (m.twist_velocity * s);
    c.normal * (m.bend_velocity * bend_shape(s))
        + angular_velocity.cross(frame_for(c, m, point) * (point - c.axis_for_point(point, s)))
}

fn apply_load(c: &SurfaceConfig, result: &mut (f64, f64), point: DVec3, force: DVec3, moment: DVec3) {
    let s = c.span_fraction(point);
    let bend_axis = c.span.cross(c.normal).normalize_or_zero();
    result.0 += force.dot(c.normal) * bend_shape(s) + moment.dot(bend_axis) * bend_slope(s, c.length);
    result.1 += ((point - c.axis_for_point(point, s)).cross(force) + moment).dot(c.span) * s;
}

// Exact constant-load solution of the damped modal equation. It remains stable
// for variable simulation dt rather than relying on a fixed explicit timestep.
pub(crate) fn oscillator(x: &mut f64, velocity: &mut f64, force: f64, mass: f64, hz: f64, damping: f64, dt: f64) {
    if !force.is_finite() || !mass.is_finite() || mass <= 0. || !hz.is_finite() || hz <= 0. { return; }
    let omega = std::f64::consts::TAU * hz;
    let zeta = damping.clamp(0., 0.999_999);
    let damped = omega * (1. - zeta * zeta).sqrt();
    let equilibrium = force / (mass * omega * omega);
    let offset = *x - equilibrium;
    let decay = (-zeta * omega * dt).exp();
    let (sin, cos) = (damped * dt).sin_cos();
    let b = (*velocity + zeta * omega * offset) / damped;
    let next = decay * (offset * cos + b * sin);
    let next_v = decay * ((-offset * damped * sin + b * damped * cos) - zeta * omega * (offset * cos + b * sin));
    if next.is_finite() && next_v.is_finite() { *x = equilibrium + next; *velocity = next_v; }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn root_moment_and_bending_stress_use_signed_si_resultants() {
        let config = Config::f22();
        let left = &config.surfaces[LEFT_WING];
        let right = &config.surfaces[RIGHT_WING];
        let load = |surface, point, force, moment| Load { surface, point, force, moment };
        let loads = [
            load(LEFT_WING, left.root - DVec3::X * 2., DVec3::Y * 1000., -DVec3::Z * 500.),
            load(RIGHT_WING, right.root + DVec3::X * 2., DVec3::Y * 1000., DVec3::Z * 500.),
            load(LEFT_FIN, DVec3::ZERO, DVec3::splat(1e9), DVec3::ZERO),
        ];
        let ml = root_bending_moment(left, LEFT_WING, &loads);
        let mr = root_bending_moment(right, RIGHT_WING, &loads);
        assert_eq!(ml, 2500.);
        assert_eq!(mr, 2500.);
        assert_eq!(bending_stress(ml, 0.01), Some(250000.));
        assert_eq!(bending_stress(-ml, 0.01), Some(-250000.));
        assert_eq!(bending_stress(ml, 0.), None);
        assert_eq!(bending_stress(ml, f64::NAN), None);
        assert_eq!(bending_stress(f64::INFINITY, 0.01), None);
    }
    #[test]
    fn modal_static_load_and_root_clamping() {
        let c = Config::f22(); let wing = c.surfaces[RIGHT_WING];
        let load = Load { surface: RIGHT_WING, point: wing.axis_point(1.), force: wing.normal * 10000., moment: DVec3::ZERO };
        let mut state = State::default();
        for _ in 0..2400 { state.advance(&c, &[load], DVec3::ZERO, 1. / 120.); }
        let expected = 10000. / (wing.bending_mass * (std::f64::consts::TAU * wing.bending_hz).powi(2));
        assert!((state.surfaces[RIGHT_WING].bend - expected).abs() < expected * 1e-8);
        assert_eq!(state.deform_point(&c, RIGHT_WING, wing.root), wing.root);
        assert!((state.deform_point(&c, RIGHT_WING, load.point) - load.point).dot(wing.normal) > 0.);
        assert_eq!(state.surfaces[LEFT_WING].bend, 0.);
    }
    #[test]
    fn constant_load_solution_is_independent_of_step_partition() {
        let c = Config::f22(); let wing = c.surfaces[LEFT_WING];
        let load = Load { surface: LEFT_WING, point: wing.axis_point(1.) + DVec3::Z, force: DVec3::Y * 3000., moment: DVec3::ZERO };
        let mut a = State::default(); let mut b = a;
        for _ in 0..120 { a.advance(&c, &[load], DVec3::ZERO, 1. / 120.); }
        for _ in 0..300 { b.advance(&c, &[load], DVec3::ZERO, 1. / 300.); }
        assert!((a.surfaces[LEFT_WING].bend - b.surfaces[LEFT_WING].bend).abs() < 1e-12);
        assert!((a.surfaces[LEFT_WING].twist - b.surfaces[LEFT_WING].twist).abs() < 1e-12);
    }
    #[test]
    fn point_velocity_matches_deformation_derivative() {
        let c = Config::f22();
        let wing = c.surfaces[RIGHT_WING];
        let point = wing.axis_point(0.7) + DVec3::Z;
        let mut state = State::default();
        state.surfaces[RIGHT_WING] = ModeState { bend: 0.2, bend_velocity: -0.3, twist: 0.04, twist_velocity: 0.2 };
        let expected = state.point_velocity(&c, RIGHT_WING, point);
        let initial = state.deform_point(&c, RIGHT_WING, point);
        let step = 1e-7;
        state.surfaces[RIGHT_WING].bend += state.surfaces[RIGHT_WING].bend_velocity * step;
        state.surfaces[RIGHT_WING].twist += state.surfaces[RIGHT_WING].twist_velocity * step;
        let numerical = (state.deform_point(&c, RIGHT_WING, point) - initial) / step;
        assert!((expected - numerical).length() < 1e-6, "{expected:?} {numerical:?}");
    }
    #[test]
    fn inertia_and_wing_load_opposite_and_release_decays() {
        let c = Config::f22(); let wing = c.surfaces[RIGHT_WING];
        let mut state = State::default();
        for _ in 0..1200 { state.advance(&c, &[], DVec3::Y * 9.81, 1. / 120.); }
        assert!(state.surfaces[RIGHT_WING].bend < 0.);
        let old = state.surfaces[RIGHT_WING].bend.abs();
        for _ in 0..1200 { state.advance(&c, &[], DVec3::ZERO, 1. / 120.); }
        assert!(state.surfaces[RIGHT_WING].bend.abs() < old * 1e-6);
        assert!(state.frame_rotation(&c, RIGHT_WING, wing.axis_point(0.)).abs_diff_eq(DQuat::IDENTITY, 1e-12));
    }
    #[test]
    fn mirrored_loads_give_mirrored_motion_and_torsion() {
        let c = Config::f22();
        let left = c.surfaces[LEFT_WING]; let right = c.surfaces[RIGHT_WING];
        let point = |wing: SurfaceConfig| wing.axis_point(0.75) + DVec3::Z * 0.4;
        let loads = [Load { surface: LEFT_WING, point: point(left), force: DVec3::Y * 6000., moment: DVec3::ZERO },
            Load { surface: RIGHT_WING, point: point(right), force: DVec3::Y * 6000., moment: DVec3::ZERO }];
        let mut state = State::default();
        for _ in 0..300 { state.advance(&c, &loads, DVec3::Y * 9.81, 1. / 120.); }
        let l = state.surfaces[LEFT_WING]; let r = state.surfaces[RIGHT_WING];
        assert!((l.bend - r.bend).abs() < 1e-12);
        assert!((l.twist + r.twist).abs() < 1e-12);
        let lp = state.deform_point(&c, LEFT_WING, point(left));
        let rp = state.deform_point(&c, RIGHT_WING, point(right));
        assert!((lp.x + rp.x).abs() < 1e-12 && (lp.y - rp.y).abs() < 1e-12 && (lp.z - rp.z).abs() < 1e-12);
    }
    #[test]
    fn hinged_panel_is_fixed_along_hinge_and_deflects_at_chord_edge() {
        let c = Config::f22();
        let mut state = State::default();
        for i in [1,2,3,5,6,7,9,10] {
            let panel = c.panels[i].unwrap();
            let hinge = panel.hinge.unwrap();
            state.panel_modes[i].bend = 0.04;
            state.panel_modes[i].twist = 0.03;
            for offset in [-1., 0., 1.] {
                let p = panel.root + hinge * offset;
                assert!((state.deform_panel_point(&c, i, p) - p).length() < 1e-12);
            }
            let edge = panel.root + panel.span * panel.length;
            assert!((state.deform_panel_point(&c, i, edge) - edge).dot(panel.normal) > 0.039);
        }
        assert!(c.panels[0].unwrap().hinge.is_none());
        assert!(c.panels[8].is_none());
    }
    #[test]
    fn panel_load_excitations_are_independent() {
        let c = Config::f22(); let panel = c.panels[2].unwrap();
        let load = PanelLoad { panel: 2, point: panel.axis_point(1.), force: panel.normal*1200., moment: DVec3::ZERO };
        let mut state = State::default();
        for _ in 0..1200 { state.advance_panels(&c, &[load], DVec3::ZERO, DVec3::ZERO, DVec3::ZERO, DVec3::ZERO, 1./120.); }
        let expected = 1200. / (panel.bending_mass * (std::f64::consts::TAU * panel.bending_hz).powi(2));
        assert!((state.panel_modes[2].bend-expected).abs()<1e-8);
        assert_eq!(state.panel_modes[6].bend, 0.);
        assert!(state.surfaces.iter().all(|m| m.bend == 0.));
    }
}
