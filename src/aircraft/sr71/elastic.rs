//! Load-driven bending/torsion and engineering section stress/strain estimates.
//! Effective beam properties are estimates; no flutter or detailed material certification.
use super::{
    airframe_config::ELASTIC,
    elements::{self, COUNT, SECTIONS},
};
use bevy::math::{DQuat, DVec3};
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct Mode {
    pub bend_m: f64,
    pub edge_m: f64,
    pub edge_velocity_m_s: f64,
    pub bend_velocity_m_s: f64,
    pub twist_rad: f64,
    pub twist_velocity_rad_s: f64,
}
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct Section {
    pub span_fraction: f64,
    pub shear_n: f64,
    pub bending_nm: f64,
    pub edge_bending_nm: f64,
    pub edge_shear_n: f64,
    pub corner_stress_pa: [f64; 4],
    pub corner_strain: [f64; 4],
    pub torsion_nm: f64,
    pub axial_n: f64,
    pub normal_stress_pa: f64,
    pub shear_stress_pa: f64,
    pub equivalent_stress_pa: f64,
    pub normal_strain: f64,
    pub modal_bending_strain: f64,
    pub beam_curvature_1_m: f64,
    pub shear_strain: f64,
    pub utilization: f64,
}
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct State {
    pub modes: [Mode; 4],
    pub sections: [[Section; SECTIONS]; 2],
    pub aerodynamic_root_nm: [f64; 2],
    pub inertial_root_nm: [f64; 2],
    pub wing_loading_n_m2: [f64; 2],
    pub damage: [f64; 4],
    pub energy_j: f64,
    pub valid: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            modes: [Mode::default(); 4],
            sections: [[Section::default(); SECTIONS]; 2],
            aerodynamic_root_nm: [0.; 2],
            inertial_root_nm: [0.; 2],
            wing_loading_n_m2: [0.; 2],
            damage: [0.; 4],
            energy_j: 0.,
            valid: true,
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct PointLoad {
    pub assembly: Option<usize>,
    pub point: DVec3,
    pub force: DVec3,
    pub moment: DVec3,
}
pub const LOAD_COUNT: usize = COUNT + 6;
fn shape(s: f64) -> f64 {
    s * s * (3. - s) * 0.5
}
fn slope(s: f64, l: f64) -> f64 {
    (3. * s - 1.5 * s * s) / l
}
fn modal(a: usize) -> (f64, f64, f64, f64) {
    if a < 2 {
        (
            ELASTIC.wing_bending_mass_kg,
            ELASTIC.wing_torsion_inertia_kg_m2,
            (3. * ELASTIC.young_modulus_pa
                * ELASTIC.wing_root_section_m3
                * ELASTIC.wing_outer_fiber_m
                / (elements::length(a).powi(3) * ELASTIC.wing_bending_mass_kg))
                .sqrt()
                / std::f64::consts::TAU,
            (ELASTIC.shear_modulus_pa * ELASTIC.wing_polar_second_moment_m4
                / (elements::length(a) * ELASTIC.wing_torsion_inertia_kg_m2))
                .sqrt()
                / std::f64::consts::TAU,
        )
    } else {
        (
            ELASTIC.fin_bending_mass_kg,
            ELASTIC.fin_torsion_inertia_kg_m2,
            ELASTIC.fin_bend_hz,
            ELASTIC.fin_twist_hz,
        )
    }
}
impl State {
    pub fn frame(&self, a: usize, s: f64) -> DQuat {
        let m = self.modes[a];
        let normal = elements::normal(a);
        let span = elements::span(a);
        DQuat::from_axis_angle(-normal, (m.edge_m * slope(s, elements::length(a))).atan())
            * DQuat::from_axis_angle(
                span.cross(normal).normalize(),
                (m.bend_m * slope(s, elements::length(a))).atan(),
            )
            * DQuat::from_axis_angle(span, m.twist_rad * s)
    }
    pub fn displacement(&self, a: usize, s: f64) -> DVec3 {
        elements::normal(a) * (self.modes[a].bend_m * shape(s))
            + elements::span(a).cross(elements::normal(a)).normalize()
                * self.modes[a].edge_m
                * shape(s)
    }
    pub fn velocity(&self, a: usize, s: f64, point: DVec3) -> DVec3 {
        let m = self.modes[a];
        let angular = elements::span(a).cross(elements::normal(a)).normalize()
            * m.bend_velocity_m_s
            * slope(s, elements::length(a))
            + elements::span(a) * m.twist_velocity_rad_s * s
            - elements::normal(a) * m.edge_velocity_m_s * slope(s, elements::length(a));
        elements::normal(a) * m.bend_velocity_m_s * shape(s)
            + elements::span(a).cross(elements::normal(a)).normalize()
                * m.edge_velocity_m_s
                * shape(s)
            + angular.cross(point - elements::root(a) - elements::span(a) * elements::length(a) * s)
    }
    pub fn advance(
        &mut self,
        loads: &[PointLoad; LOAD_COUNT],
        specific_acceleration: DVec3,
        rates: DVec3,
        angular_acc: DVec3,
        dt: f64,
        equilibrium: bool,
    ) {
        let mut generalized = [(0., 0., 0.); 4];
        self.aerodynamic_root_nm = [0.; 2];
        self.inertial_root_nm = [0.; 2];
        self.wing_loading_n_m2 = [0.; 2];
        let mut total = [PointLoad::default(); LOAD_COUNT];
        for (i, l) in loads.iter().enumerate() {
            let Some(a) = l.assembly else {
                continue;
            };
            let span = elements::span(a);
            let normal = elements::normal(a);
            let length = elements::length(a);
            let s = ((l.point - elements::root(a)).dot(span) / length).clamp(0., 1.);
            // Only the fixed aerodynamic elements carry estimated distributed mass; controls do not duplicate it.
            let local_mass = if i < COUNT {
                if a < 2 {
                    ELASTIC.wing_bending_mass_kg * 4. / 6.
                } else {
                    ELASTIC.fin_bending_mass_kg * 4. / 2.
                }
            } else {
                0.
            };
            let inertia = -local_mass
                * (specific_acceleration
                    + angular_acc.cross(l.point)
                    + rates.cross(rates.cross(l.point)));
            let force = l.force + inertia;
            total[i] = PointLoad { force, ..*l };
            let axis = span.cross(normal).normalize();
            generalized[a].0 +=
                force.dot(normal) * shape(s) + l.moment.dot(axis) * slope(s, length);
            let elastic_axis = elements::root(a)
                + span * (length * s)
                + DVec3::Z * (if a < 2 { 7.8 * s } else { s });
            generalized[a].2 +=
                force.dot(axis) * shape(s) - l.moment.dot(normal) * slope(s, length);
            generalized[a].1 += ((l.point - elastic_axis).cross(force) + l.moment).dot(span) * s;
            if a < 2 {
                self.aerodynamic_root_nm[a] +=
                    ((l.point - elements::root(a)).cross(l.force) + l.moment).dot(axis);
                self.inertial_root_nm[a] += (l.point - elements::root(a)).cross(inertia).dot(axis);
                self.wing_loading_n_m2[a] += l.force.dot(normal)
                    / (super::airframe_config::SPATIAL.estimated_lifting_planform_m2 * 0.5);
            }
        }
        for a in 0..2 {
            let span = elements::span(a);
            let normal = elements::normal(a);
            let axis = span.cross(normal).normalize();
            for j in 0..SECTIONS {
                let fraction = j as f64 / SECTIONS as f64;
                let station =
                    elements::root(a) + (span * elements::length(a) + DVec3::Z * 7.8) * fraction;
                let mut f = DVec3::ZERO;
                let mut m = DVec3::ZERO;
                for l in total.iter().filter(|l| l.assembly == Some(a)) {
                    if (l.point - elements::root(a)).dot(span) + 1e-8
                        >= fraction * elements::length(a)
                    {
                        f += l.force;
                        m += (l.point - station).cross(l.force) + l.moment;
                    }
                }
                let taper = (1. - ELASTIC.section_taper * fraction).max(0.08);
                let axial = f.dot(span);
                let bend = m.dot(axis);
                let torsion = m.dot(span);
                let shear = f.dot(normal);
                let edge_bending = m.dot(normal);
                let edge_shear = f.dot(axis);
                let axial_stress = axial / (ELASTIC.wing_shear_area_m2 * taper);
                let flap_stress = bend / (ELASTIC.wing_root_section_m3 * taper);
                let edge_stress = edge_bending / (ELASTIC.wing_edge_section_m3 * taper);
                let corners = [
                    axial_stress + flap_stress + edge_stress,
                    axial_stress + flap_stress - edge_stress,
                    axial_stress - flap_stress + edge_stress,
                    axial_stress - flap_stress - edge_stress,
                ];
                let sigma = corners.into_iter().fold(0_f64, |current, x| {
                    if x.abs() > current.abs() { x } else { current }
                });
                let tau = (shear.hypot(edge_shear) / (ELASTIC.wing_shear_area_m2 * taper))
                    .hypot(torsion / (ELASTIC.wing_root_torsion_m3 * taper));
                let utilization = corners
                    .into_iter()
                    .map(|x| {
                        if x >= 0. {
                            x / ELASTIC.tensile_limit_pa
                        } else {
                            -x / ELASTIC.compressive_limit_pa
                        }
                    })
                    .fold(0., f64::max)
                    .max(tau / ELASTIC.shear_limit_pa);
                self.sections[a][j] = Section {
                    span_fraction: fraction,
                    shear_n: shear,
                    bending_nm: bend,
                    edge_bending_nm: edge_bending,
                    edge_shear_n: edge_shear,
                    corner_stress_pa: corners,
                    corner_strain: corners.map(|x| x / ELASTIC.young_modulus_pa),
                    torsion_nm: torsion,
                    axial_n: axial,
                    normal_stress_pa: sigma,
                    shear_stress_pa: tau,
                    equivalent_stress_pa: (sigma * sigma + 3. * tau * tau).sqrt(),
                    normal_strain: sigma / ELASTIC.young_modulus_pa,
                    beam_curvature_1_m: self.modes[a].bend_m * 3. * (1. - fraction)
                        / elements::length(a).powi(2),
                    modal_bending_strain: ELASTIC.wing_outer_fiber_m
                        * self.modes[a].bend_m
                        * 3.
                        * (1. - fraction)
                        / elements::length(a).powi(2),
                    shear_strain: tau / ELASTIC.shear_modulus_pa,
                    utilization,
                };
            }
        }
        self.energy_j = 0.;
        for (a, (force, torque, edge_force)) in generalized.into_iter().enumerate() {
            let (mass, inertia, bhz, thz) = modal(a);
            let stiffness = (1. - self.damage[a]).max(ELASTIC.minimum_stiffness_fraction);
            let bhz = bhz * stiffness.sqrt();
            let thz = thz * stiffness.sqrt();
            let edge_hz = if a < 2 {
                (3. * ELASTIC.young_modulus_pa
                    * ELASTIC.wing_edge_section_m3
                    * ELASTIC.wing_edge_outer_fiber_m
                    / (elements::length(a).powi(3) * mass))
                    .sqrt()
                    / std::f64::consts::TAU
                    * stiffness.sqrt()
            } else {
                bhz
            };
            let m = &mut self.modes[a];
            if equilibrium {
                m.bend_m = force / (mass * (std::f64::consts::TAU * bhz).powi(2));
                m.twist_rad = torque / (inertia * (std::f64::consts::TAU * thz).powi(2));
                m.edge_m = edge_force / (mass * (std::f64::consts::TAU * edge_hz).powi(2));
                m.edge_velocity_m_s = 0.;
                m.bend_velocity_m_s = 0.;
                m.twist_velocity_rad_s = 0.;
            } else {
                crate::structural::oscillator(
                    &mut m.edge_m,
                    &mut m.edge_velocity_m_s,
                    edge_force,
                    mass,
                    edge_hz,
                    ELASTIC.bending_damping,
                    dt,
                );
                crate::structural::oscillator(
                    &mut m.bend_m,
                    &mut m.bend_velocity_m_s,
                    force,
                    mass,
                    bhz,
                    ELASTIC.bending_damping,
                    dt,
                );
                crate::structural::oscillator(
                    &mut m.twist_rad,
                    &mut m.twist_velocity_rad_s,
                    torque,
                    inertia,
                    thz,
                    ELASTIC.twist_damping,
                    dt,
                );
            }
            self.energy_j += 0.5
                * mass
                * (m.edge_velocity_m_s.powi(2)
                    + (std::f64::consts::TAU * edge_hz * m.edge_m).powi(2));
            self.energy_j += 0.5
                * mass
                * (m.bend_velocity_m_s.powi(2) + (std::f64::consts::TAU * bhz * m.bend_m).powi(2))
                + 0.5
                    * inertia
                    * (m.twist_velocity_rad_s.powi(2)
                        + (std::f64::consts::TAU * thz * m.twist_rad).powi(2));
            if !equilibrium {
                let utilization = if a < 2 {
                    self.sections[a]
                        .iter()
                        .map(|s| s.utilization)
                        .fold(0., f64::max)
                } else {
                    (m.bend_m / ELASTIC.bend_soft_limit_m)
                        .abs()
                        .max((m.twist_rad / ELASTIC.twist_soft_limit_rad).abs())
                };
                self.damage[a] = (self.damage[a]
                    + dt * ELASTIC.overload_damage_rate * (utilization - 1.).max(0.).powi(2))
                .min(1.);
            }
        }
        self.valid = self.energy_j.is_finite()
            && self
                .modes
                .iter()
                .all(|m| m.bend_m.is_finite() && m.twist_rad.is_finite() && m.edge_m.is_finite());
    }
}
