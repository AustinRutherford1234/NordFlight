//! Allocation-free per-step instrumentation; explicit CSV/JSON export is headless only.
use super::{config::*, trim::TrimReport};
use crate::{
    aerodynamics::{Conditions, FlowState},
    physics::*,
};
use bevy::math::{DMat3, DVec3};
use serde::Serialize;
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum EvaluationError {
    NonFiniteInput,
    NegativeTimeStep,
    InvalidAtmosphere,
}
pub fn validate_conditions(c: Conditions, dt: f64) -> Result<(), EvaluationError> {
    if ![
        c.alpha,
        c.beta,
        c.mach,
        c.speed,
        c.density,
        c.temperature,
        c.height,
        c.flaps,
        dt,
    ]
    .iter()
    .all(|v| v.is_finite())
        || !c.rates.is_finite()
    {
        return Err(EvaluationError::NonFiniteInput);
    }
    if dt < 0. {
        return Err(EvaluationError::NegativeTimeStep);
    }
    if c.density < 0. || c.temperature <= 0. || c.speed < 0. || c.mach < 0. {
        return Err(EvaluationError::InvalidAtmosphere);
    }
    Ok(())
}
pub fn flow_is_valid(s: &FlowState) -> bool {
    [
        s.lag_alpha,
        s.lag_beta,
        s.vortex_lift,
        s.attached,
        s.wing_attached[0],
        s.wing_attached[1],
    ]
    .iter()
    .all(|v| v.is_finite())
        && (0.0..=1.0).contains(&s.attached)
        && s.wing_attached.iter().all(|v| (0.0..=1.0).contains(v))
        && s.vortex_lift.abs() <= AERO.vortex_transient_limit + 1e-9
}
pub const INVALID_INPUT: u32 = 1;
pub const FLOW_RESET: u32 = 2;
pub const OUTSIDE_MACH_TABLE: u32 = 4;
pub const SEPARATED: u32 = 8;
pub const COMMAND_SATURATED: u32 = 16;
pub const HYDRAULIC_LOSS: u32 = 32;
pub const ENGINE_OUT: u32 = 64;
pub const SURFACE_FAILURE: u32 = 128;
pub const INVALID_LOAD: u32 = 256;
pub const TRIM_FAILED: u32 = 512;
pub const OUTSIDE_EVIDENCE: u32 = 1024;
pub const ELASTIC_OVERLOAD: u32 = 2048;
pub const INVALID_ELASTIC_STATE: u32 = 4096;
pub const LOCAL_DATABASE_CLAMP: u32 = 8192;
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct Snapshot {
    pub simulation_time_s: f64,
    pub dt_s: f64,
    pub altitude_m: f64,
    pub mach: f64,
    pub dynamic_pressure_pa: f64,
    pub alpha_rad: f64,
    pub beta_rad: f64,
    pub baseline_force_body_n: [f64; 3],
    pub control_force_body_n: [f64; 3],
    pub damage_force_body_n: [f64; 3],
    pub engine_force_body_n: [f64; 3],
    pub total_specific_force_body_n: [f64; 3],
    pub baseline_moment_body_nm: [f64; 3],
    pub control_moment_body_nm: [f64; 3],
    pub damage_moment_body_nm: [f64; 3],
    pub engine_moment_body_nm: [f64; 3],
    pub total_moment_body_nm: [f64; 3],
    pub gyroscopic_moment_body_nm: [f64; 3],
    pub angular_acceleration_rad_s2: [f64; 3],
    pub moment_closure_error_nm: f64,
    pub drag_components_n: [f64; 6],
    pub attachment: [f64; 2],
    pub requested_control: [f64; 3],
    pub elevons_rad: [f64; 4],
    pub rudders_rad: [f64; 2],
    pub rates_rad_s: [f64; 3],
    pub g_load: f64,
    pub fuel_kg: f64,
    pub integrity: f64,
    pub inlet_recovery: f64,
    pub flags: u32,
    pub engines:[super::engine::state::State;2],
    pub element_points_body_m: [[f64; 3]; 30],
    pub element_forces_body_n: [[f64; 3]; 30],
    pub actual_element_forces_body_n: [[f64; 3]; 30],
    pub control_induced_force_body_n: [f64; 3],
    pub element_alpha_rad: [f64; 30],
    pub element_attachment: [f64; 30],
    pub element_ground_pressure: [f64; 30],
    pub unresolved_airframe_force_body_n: [f64; 3],
    pub unresolved_airframe_moment_body_nm: [f64; 3],
    pub elastic_modes: [super::elastic::Mode; 4],
    pub wing_sections: [[super::elastic::Section; 6]; 2],
    pub aerodynamic_root_nm: [f64; 2],
    pub inertial_root_nm: [f64; 2],
    pub wing_loading_n_m2: [f64; 2],
    pub aircraft_weight_loading_n_m2: f64,
    pub elastic_energy_j: f64,
    pub elastic_damage: [f64; 4],
}
#[derive(Clone, Default)]
pub struct State {
    pub latest: Snapshot,
    pub steps: u64,
    pub rejected_inputs: u64,
    pub repaired_flow_states: u64,
    pub sticky_flags: u32,
    pub peak_rate_rad_s: f64,
    pub peak_dynamic_pressure_pa: f64,
    pub trim: Option<TrimReport>,
}
/// Diagnose invalid inputs before the checked evaluator repairs a corrupt flow history.
pub fn begin(f: &mut Flight, c: Conditions, dt: f64) {
    let d = &mut f.systems.sr71.diagnostics;
    d.latest.flags = 0;
    if validate_conditions(c, dt).is_err() {
        d.rejected_inputs += 1;
        d.latest.flags |= INVALID_INPUT;
    }
    if !flow_is_valid(&f.flow) {
        d.repaired_flow_states += 1;
        d.latest.flags |= FLOW_RESET;
    }
}
/// Forces are body-frame Newtons; gravity/contact are intentionally outside this airborne budget.
#[derive(Clone, Copy)]
pub struct StepLoads {
    pub dt: f64,
    pub air_body: DVec3,
    pub dynamic_pressure: f64,
    pub aerodynamic_world: DVec3,
    pub thrust_world: DVec3,
    pub controls_body: DVec3,
    pub damage_body: DVec3,
    pub natural_moment: DVec3,
    pub control_moment: DVec3,
    pub damage_moment: DVec3,
    pub engine_moment: DVec3,
    pub total_moment: DVec3,
    pub inertia: DMat3,
    pub angular_acceleration: DVec3,
}
pub fn capture(f: &mut Flight, loads: StepLoads) {
    let StepLoads {
        dt,
        air_body: b,
        dynamic_pressure: q,
        aerodynamic_world: aero_world,
        thrust_world,
        controls_body: control_force,
        damage_body: damage_force,
        natural_moment: natural,
        control_moment,
        damage_moment,
        engine_moment,
        total_moment,
        inertia,
        angular_acceleration: angular_acc,
    } = loads;
    let sr = &f.systems.sr71;
    let mut flags = sr.diagnostics.latest.flags;
    if f.mach > MACH[MACH.len() - 1] || f.mach < MACH[0] {
        flags |= OUTSIDE_MACH_TABLE;
    }
    if !(EVIDENCE_MACH_RANGE[0]..=EVIDENCE_MACH_RANGE[1]).contains(&f.mach) {
        flags |= OUTSIDE_EVIDENCE;
    }
    if f.flow.attached < 0.5 {
        flags |= SEPARATED;
    }
    if f.surface.abs().max_element() > 0.99 {
        flags |= COMMAND_SATURATED;
    }
    if f.damage.hydraulics <= CONTROL.hydraulic_floor {
        flags |= HYDRAULIC_LOSS;
    }
    if !f.engine || f.systems.engine_on.iter().any(|v| !*v) || (!f.infinite_fuel && f.fuel <= 0.) {
        flags |= ENGINE_OUT;
    }
    if [0, 4, 2, 6, 3, 7].iter().any(|&i| {
        f.damage.surface_state[i] != crate::combat::damage::SurfaceState::Operational
            || f.damage.surfaces[i] < 0.999
    }) {
        flags |= SURFACE_FAILURE;
    }
    if sr.diagnostics.trim.is_some_and(|t| t.failure.is_some()) {
        flags |= TRIM_FAILED;
    }
    let aero = f.orientation.conjugate() * aero_world;
    let thrust = f.orientation.conjugate() * thrust_world;
    let gyro = -f.rates.cross(inertia * f.rates);
    let closure = (inertia * angular_acc - total_moment - gyro).length();
    if !aero.is_finite() || !thrust.is_finite() || !total_moment.is_finite() || !closure.is_finite()
    {
        flags |= INVALID_LOAD;
    }
    if sr
        .airframe
        .structure
        .sections
        .iter()
        .flatten()
        .any(|s| s.utilization > 1.)
    {
        flags |= ELASTIC_OVERLOAD;
    }
    if sr.airframe.local.iter().any(|l| {
        l.mach > super::database::MACH_AXIS[super::database::MACH_AXIS.len() - 1]
            || l.beta.abs().to_degrees()
                > super::database::BETA_DEG[super::database::BETA_DEG.len() - 1]
    }) {
        flags |= LOCAL_DATABASE_CLAMP;
    }
    if !sr.airframe.structure.valid {
        flags |= INVALID_ELASTIC_STATE;
    }
    let c = f.aero_coeff;
    let qs = q * AREA;
    let allocated = c.parasite + c.induced + c.wave + c.control_drag;
    let snap = Snapshot {
        simulation_time_s: f.elapsed,
        dt_s: dt,
        altitude_m: f.altitude(),
        mach: f.mach,
        dynamic_pressure_pa: q,
        alpha_rad: f.alpha,
        beta_rad: f.beta,
        baseline_force_body_n: (aero - control_force - damage_force).to_array(),
        control_force_body_n: control_force.to_array(),
        damage_force_body_n: damage_force.to_array(),
        engine_force_body_n: thrust.to_array(),
        total_specific_force_body_n: (aero + thrust).to_array(),
        baseline_moment_body_nm: (natural - damage_moment).to_array(),
        control_moment_body_nm: control_moment.to_array(),
        damage_moment_body_nm: damage_moment.to_array(),
        engine_moment_body_nm: engine_moment.to_array(),
        total_moment_body_nm: total_moment.to_array(),
        gyroscopic_moment_body_nm: gyro.to_array(),
        angular_acceleration_rad_s2: angular_acc.to_array(),
        moment_closure_error_nm: closure,
        drag_components_n: [
            c.parasite * qs,
            c.induced * qs,
            c.wave * qs,
            c.control_drag * qs,
            (c.drag - allocated) * qs,
            -aero.dot(b.normalize_or_zero()),
        ],
        attachment: f.flow.wing_attached,
        requested_control: f.surface.to_array(),
        elevons_rad: sr.elevons,
        rudders_rad: sr.fins,
        rates_rad_s: f.rates.to_array(),
        g_load: f.g_load,
        fuel_kg: f.fuel,
        integrity: f.integrity,
        inlet_recovery: super::engine::recovery(f),
        flags,
        engines:sr.propulsion.sides,
        element_points_body_m: sr.airframe.local.map(|l| l.at.to_array()),
        element_forces_body_n: sr.airframe.local.map(|l| l.force.to_array()),
        actual_element_forces_body_n: std::array::from_fn(|i| {
            sr.airframe.last_loads[i].force.to_array()
        }),
        control_induced_force_body_n: sr.airframe.control_induced_force.to_array(),
        element_alpha_rad: sr.airframe.local.map(|l| l.alpha),
        element_attachment: sr.airframe.local.map(|l| l.flow.attached),
        element_ground_pressure: sr.airframe.local.map(|l| l.pressure),
        unresolved_airframe_force_body_n: (sr.airframe.base.force
            - sr.airframe.local.iter().map(|l| l.force).sum::<DVec3>())
        .to_array(),
        unresolved_airframe_moment_body_nm: (sr.airframe.base.moment
            - sr.airframe.local.iter().map(|l| l.moment).sum::<DVec3>())
        .to_array(),
        elastic_modes: sr.airframe.structure.modes,
        wing_sections: sr.airframe.structure.sections,
        aerodynamic_root_nm: sr.airframe.structure.aerodynamic_root_nm,
        inertial_root_nm: sr.airframe.structure.inertial_root_nm,
        wing_loading_n_m2: sr.airframe.structure.wing_loading_n_m2,
        aircraft_weight_loading_n_m2: f.mass() * planet_mu()
            / f.position.length_squared()
            / super::airframe_config::SPATIAL.estimated_lifting_planform_m2,
        elastic_energy_j: sr.airframe.structure.energy_j,
        elastic_damage: sr.airframe.structure.damage,
    };
    let d = &mut f.systems.sr71.diagnostics;
    d.steps += 1;
    d.sticky_flags |= flags;
    d.peak_rate_rad_s = d.peak_rate_rad_s.max(f.rates.length());
    d.peak_dynamic_pressure_pa = d.peak_dynamic_pressure_pa.max(q);
    d.latest = snap;
}
