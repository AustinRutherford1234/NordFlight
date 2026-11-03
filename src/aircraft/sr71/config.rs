//! Single source of SR-71 dimensional constants and coefficient tables.
//! NASA TP-2002-210718 baseline graph estimates; see the parameter register.
use std::f64::consts::PI;
pub const AREA: f64 = 1605. * 0.09290304;
pub const SPAN: f64 = 56.7 * 0.3048;
pub const CHORD: f64 = 37.7 * 0.3048;
pub const EMPTY: f64 = 60728. * 0.45359237;
pub const FUEL: f64 = 80000. * 0.45359237;
pub const INCIDENCE: f64 = 1.2 * PI / 180.;
/// Flight-test coverage stated in NASA TP-2002-210718; tables outside this include estimates.
pub const EVIDENCE_MACH_RANGE: [f64; 2] = [0.4, 3.0];
pub const MACH: [f64; 9] = [0., 0.6, 0.9, 1.1, 1.6, 2., 2.5, 3., 3.5];
// Approximate visual readings of BASELINE figures 15-18,36-40, not raw test data.
pub const CNA: [f64; 9] = [
    0.042, 0.042, 0.049, 0.044, 0.033, 0.027, 0.0255, 0.025, 0.023,
]; // /deg
pub const CMA: [f64; 9] = [
    0.001, 0.001, 0.0002, -0.0027, -0.0025, -0.0012, -0.0010, -0.0005, -0.0004,
];
pub const CMB: [f64; 9] = [
    0.003, 0.003, 0.004, 0.010, 0.005, 0.003, 0.002, 0.001, 0.001,
];
pub const CNB: [f64; 9] = [
    -0.015, -0.015, -0.025, -0.020, -0.01, 0.004, 0.02, 0.03, 0.03,
];
pub const CMDE: [f64; 9] = [
    -0.005, -0.005, -0.0044, -0.0021, -0.00165, -0.0013, -0.0011, -0.001, -0.0009,
];
pub const CNDE: [f64; 9] = [
    0.004, 0.004, 0.0015, -0.0004, 0., 0.0003, 0.0003, 0.0003, 0.0003,
];
pub const CYB: [f64; 9] = [
    -0.007, -0.007, -0.008, -0.0085, -0.006, -0.0065, -0.0058, -0.006, -0.006,
];
pub const CLB: [f64; 9] = [
    -0.0009, -0.0009, -0.0007, -0.0011, -0.001, -0.0009, -0.0006, -0.0004, -0.0003,
];
pub const CNBETA: [f64; 9] = [
    0.0014, 0.0014, 0.0017, 0.0019, 0.0009, 0.001, 0.0006, 0.0005, 0.0004,
];
pub const CLDA: [f64; 9] = [
    0.0018, 0.0018, 0.0015, 0.0007, 0.0005, 0.00045, 0.0004, 0.00035, 0.0003,
];
pub const CNDR: [f64; 9] = [
    -0.0011, -0.0011, -0.0015, -0.0014, -0.0009, -0.0007, -0.0005, -0.00035, -0.0003,
];
pub const CYDR: [f64; 9] = [
    0.003, 0.003, 0.0038, 0.0035, 0.0024, 0.002, 0.0017, 0.0013, 0.0011,
];

#[derive(Clone, Copy, serde::Serialize)]
pub struct AeroTuning {
    pub efficiency: f64,
    pub separation_efficiency_loss: f64,
    pub slip_efficiency_loss: f64,
    pub minimum_efficiency: f64,
    pub maximum_efficiency: f64,
    pub ground_height_scale: f64,
    pub minimum_ground_factor: f64,
    pub supersonic_induced_gain: f64,
    pub separation_onset: f64,
    pub supersonic_onset_reduction: f64,
    pub hysteresis: f64,
    pub separation_width_before: f64,
    pub separation_width_after: f64,
    pub convection_factor: f64,
    pub min_lag_s: f64,
    pub max_lag_s: f64,
    pub lag_speed_floor_m_s: f64,
    pub separation_time_factor: f64,
    pub reattachment_time_factor: f64,
    pub sideslip_time_factor: f64,
    pub vortex_transient_gain: f64,
    pub vortex_transient_limit: f64,
    pub vortex_time_factor: f64,
    pub chine_lift_gain: f64,
    pub supersonic_chine_loss: f64,
    pub flat_plate_normal_gain: f64,
    pub parasite_base: f64,
    pub wetted_area_ratio: f64,
    pub sideslip_drag: f64,
    pub gear_drag: f64,
    pub door_drag: f64,
    pub windmill_drag_per_engine: f64,
    pub wave_peak: f64,
    pub wave_center_mach: f64,
    pub wave_width_mach: f64,
    pub supersonic_wave: f64,
    pub separated_drag: f64,
    pub incidence_drag: f64,
    pub separated_pitch: f64,
    pub side_yaw_damping: f64,
    pub roll_yaw_coupling: f64,
    pub yaw_roll_coupling: f64,
    pub asymmetric_separation_roll: f64,
    pub control_floor: f64,
    pub control_attached_gain: f64,
    pub control_mach_loss: f64,
    pub minimum_reynolds: f64,
    pub skin_friction_gain: f64,
    pub reynolds_exponent: f64,
    pub compressibility_gain: f64,
    pub compressibility_exponent: f64,
}
pub const AERO: AeroTuning = AeroTuning {
    efficiency: 0.79,
    separation_efficiency_loss: 0.15,
    slip_efficiency_loss: 0.1,
    minimum_efficiency: 0.45,
    maximum_efficiency: 0.82,
    ground_height_scale: 16.,
    minimum_ground_factor: 0.35,
    supersonic_induced_gain: 0.2,
    separation_onset: 0.43,
    supersonic_onset_reduction: 0.10,
    hysteresis: 0.045,
    separation_width_before: 0.04,
    separation_width_after: 0.12,
    convection_factor: 0.9,
    min_lag_s: 0.035,
    max_lag_s: 0.6,
    lag_speed_floor_m_s: 25.,
    separation_time_factor: 1.1,
    reattachment_time_factor: 3.,
    sideslip_time_factor: 1.4,
    vortex_transient_gain: 0.35,
    vortex_transient_limit: 0.12,
    vortex_time_factor: 2.,
    chine_lift_gain: 0.85,
    supersonic_chine_loss: 0.55,
    flat_plate_normal_gain: 1.5,
    parasite_base: 0.0042,
    wetted_area_ratio: 4.2,
    sideslip_drag: 0.2,
    gear_drag: 0.055,
    door_drag: 0.012,
    windmill_drag_per_engine: 0.004,
    wave_peak: 0.018,
    wave_center_mach: 1.04,
    wave_width_mach: 0.19,
    supersonic_wave: 0.009,
    separated_drag: 1.55,
    incidence_drag: 0.16,
    separated_pitch: 0.22,
    side_yaw_damping: 0.12,
    roll_yaw_coupling: 0.06,
    yaw_roll_coupling: 0.015,
    asymmetric_separation_roll: 0.05,
    control_floor: 0.3,
    control_attached_gain: 0.7,
    control_mach_loss: 0.2,
    minimum_reynolds: 1e4,
    skin_friction_gain: 0.455,
    reynolds_exponent: 2.58,
    compressibility_gain: 0.144,
    compressibility_exponent: 0.65,
};
#[derive(Clone, Copy, serde::Serialize)]
pub struct EngineTuning {
    pub mach: [f64; 9],
    pub dry_factor: [f64; 9],
    pub wet_factor: [f64; 9],
    pub static_dry_n: f64,
    pub static_wet_n: f64,
    pub reference_density: f64,
    pub density_exponent: f64,
    pub dry_power_exponent: f64,
    pub burner_threshold: f64,
    pub angular_alpha_loss: f64,
    pub angular_beta_loss: f64,
    pub spike_mismatch_loss: f64,
    pub minimum_recovery: f64,
    pub spike_travel_m: f64,
    pub spike_start_mach: f64,
    pub spike_mach_interval: f64,
    pub dry_tsfc_kg_n_s: f64,
    pub burner_tsfc_increment: f64,
}
pub const ENGINE: EngineTuning = EngineTuning {
    mach: [0., 0.6, 0.9, 1.1, 1.6, 2., 2.5, 3.2, 3.6],
    dry_factor: [1., 0.94, 0.86, 0.80, 0.72, 0.66, 0.56, 0.45, 0.25],
    wet_factor: [1., 0.96, 0.94, 0.99, 1.18, 1.50, 2., 2.65, 2.15],
    static_dry_n: 180000.,
    static_wet_n: 289134.,
    reference_density: 1.225,
    density_exponent: 0.62,
    dry_power_exponent: 1.35,
    burner_threshold: 0.88,
    angular_alpha_loss: 0.28,
    angular_beta_loss: 0.3,
    spike_mismatch_loss: 0.25,
    minimum_recovery: 0.35,
    spike_travel_m: 0.66,
    spike_start_mach: 1.6,
    spike_mach_interval: 1.6,
    dry_tsfc_kg_n_s: 0.000023,
    burner_tsfc_increment: 0.000021,
};
#[derive(Clone, Copy, serde::Serialize)]
pub struct MassTuning {
    pub zero_fuel_cg_mac: f64,
    pub fuel_cg_mac: f64,
    pub moment_reference_mac: f64,
    pub slug_ft2_to_kg_m2: f64,
    pub roll_inertia: f64,
    pub pitch_inertia: f64,
    pub yaw_inertia: f64,
    pub product_inertia: f64,
    pub fuel_inertia_m2: [f64; 3],
    pub engine_lateral_m: f64,
    pub engine_aft_m: f64,
}
pub const MASS: MassTuning = MassTuning {
    zero_fuel_cg_mac: 0.201,
    fuel_cg_mac: 0.235,
    moment_reference_mac: 0.25,
    slug_ft2_to_kg_m2: 1.3558179483,
    roll_inertia: 220660.,
    pitch_inertia: 954850.,
    yaw_inertia: 1172039.,
    product_inertia: 19200.,
    fuel_inertia_m2: [22., 31., 9.],
    engine_lateral_m: 4.9,
    engine_aft_m: 12.4,
};
#[derive(Clone, Copy, serde::Serialize)]
pub struct ControlTuning {
    pub elevon_up_rad: f64,
    pub inner_down_rad: f64,
    pub outer_down_rad: f64,
    pub outer_rig_rad: f64,
    pub rudder_limit_rad: f64,
    pub elevon_rate_rad_s: f64,
    pub rudder_rate_rad_s: f64,
    pub inner_weight: f64,
    pub pitch_drag: f64,
    pub roll_drag: f64,
    pub yaw_drag: f64,
    pub low_mach_pitch_rate: f64,
    pub pitch_rate_reduction: f64,
    pub low_mach_roll_rate: f64,
    pub roll_rate_reduction: f64,
    pub yaw_rate: f64,
    pub sideslip_gain: f64,
    pub neutral_deadband: f64,
    pub load_trim_gain: f64,
    pub load_trim_rate_limit: f64,
    pub alpha_limit_rad: f64,
    pub alpha_limit_reduction: f64,
    pub alpha_limit_gain: f64,
    pub rate_gain: f64,
    pub acceleration_limits: [f64; 3],
    pub hydraulic_floor: f64,
}
pub const CONTROL: ControlTuning = ControlTuning {
    elevon_up_rad: 35. * PI / 180.,
    inner_down_rad: 20. * PI / 180.,
    outer_down_rad: 35. * PI / 180.,
    outer_rig_rad: 3. * PI / 180.,
    rudder_limit_rad: 20. * PI / 180.,
    elevon_rate_rad_s: 30. * PI / 180.,
    rudder_rate_rad_s: 33. * PI / 180.,
    inner_weight: 0.6,
    pitch_drag: 0.035,
    roll_drag: 0.025,
    yaw_drag: 0.03,
    low_mach_pitch_rate: 0.20,
    pitch_rate_reduction: 0.13,
    low_mach_roll_rate: 0.65,
    roll_rate_reduction: 0.4,
    yaw_rate: 0.10,
    sideslip_gain: 0.45,
    neutral_deadband: 0.025,
    load_trim_gain: 0.025,
    load_trim_rate_limit: 0.04,
    alpha_limit_rad: 0.32,
    alpha_limit_reduction: 0.13,
    alpha_limit_gain: 1.2,
    rate_gain: 1.5,
    acceleration_limits: [0.25, 0.2, 0.6],
    hydraulic_floor: 0.12,
};
#[derive(Clone, Copy, serde::Serialize)]
pub struct SolverTuning {
    pub iterations: usize,
    pub residual_tolerance: f64,
    pub difference_step: f64,
    pub determinant_floor: f64,
    pub max_newton_step: f64,
    pub minimum_trim_speed_m_s: f64,
    pub trace_period_s: f64,
    pub state_reset_attached: f64,
}
pub const SOLVER: SolverTuning = SolverTuning {
    iterations: 50,
    residual_tolerance: 1e-5,
    difference_step: 1e-4,
    determinant_floor: 1e-12,
    max_newton_step: 0.15,
    minimum_trim_speed_m_s: 35.,
    trace_period_s: 0.1,
    state_reset_attached: 1.,
};
pub const CNQ: [f64; 9] = [-9., -9., -22., -22., -20., -21., -28., -35., -35.];
pub const CMQ: [f64; 9] = [-1.8, -1.8, -2.7, -2.6, -2.1, -1.8, -0.9, -0.9, -0.9];
pub const CLP: [f64; 9] = [
    -0.16, -0.16, -0.18, -0.19, -0.16, -0.12, -0.10, -0.09, -0.08,
];
pub const CNR: [f64; 9] = [
    -0.13, -0.13, -0.13, -0.12, -0.10, -0.09, -0.08, -0.07, -0.06,
];
pub const CNDA: [f64; 9] = [0.00005, 0.00005, 0.0001, 0.00005, 0., 0., 0., 0., 0.];
/// Validate offline/startup, not in every force evaluation. Export exactly what runtime uses.
pub fn register() -> serde_json::Value {
    serde_json::json!({"schema":1,"units":"SI except source tables per degree; inertia baseline slug ft2 converted at use", "source":"NASA/TP-2002-210718 baseline graph readings; nonlinear, drag, engine and fuel distribution are estimates", "area_m2":AREA,"span_m":SPAN,"chord_m":CHORD,"empty_kg":EMPTY,"fuel_capacity_kg":FUEL,"incidence_rad":INCIDENCE,"evidence_mach_range":EVIDENCE_MACH_RANGE,"aerodynamics":AERO,"engine":ENGINE,"mass":MASS,"controls":CONTROL,"solver":SOLVER,"regimes":REGIMES,"tables":{"mach":MACH,"CNA":CNA,"CMA":CMA,"CMB":CMB,"CNB":CNB,"CMDE":CMDE,"CNDE":CNDE,"CYB":CYB,"CLB":CLB,"CNBETA":CNBETA,"CLDA":CLDA,"CNDR":CNDR,"CYDR":CYDR,"CNQ":CNQ,"CMQ":CMQ,"CLP":CLP,"CNR":CNR,"CNDA":CNDA}})
}
pub fn validate() -> Result<(), String> {
    if !MACH.windows(2).all(|x| x[1] > x[0]) || !ENGINE.mach.windows(2).all(|x| x[1] > x[0]) {
        return Err("Mach axes must strictly increase".into());
    }
    fn finite(v: &serde_json::Value) -> bool {
        match v {
            serde_json::Value::Null => false,
            serde_json::Value::Array(a) => a.iter().all(finite),
            serde_json::Value::Object(o) => o.values().all(finite),
            _ => true,
        }
    }
    if !finite(&register())
        || AERO.min_lag_s <= 0.
        || AERO.max_lag_s < AERO.min_lag_s
        || ENGINE.burner_threshold >= 1.
        || ENGINE.spike_travel_m <= 0.
        || SOLVER.trace_period_s <= 0.
        || CONTROL.hydraulic_floor >= 1.
    {
        return Err("invalid SR-71 model parameter".into());
    }
    Ok(())
}

#[derive(Clone, Copy, serde::Serialize)]
pub struct Regimes {
    pub authority_mach: [f64; 2],
    pub vortex_alpha_rad: [f64; 2],
    pub wave_rise_mach: [f64; 2],
    pub induced_rise_mach: [f64; 2],
    pub inlet_alpha_rad: [f64; 2],
    pub inlet_beta_rad: [f64; 2],
    pub inlet_mismatch_mach: [f64; 2],
    pub pitch_rate_normalized_limit: f64,
    pub pitch_damping_normalized_limit: f64,
    pub viscosity_reference_pa_s: f64,
    pub viscosity_reference_k: f64,
    pub sutherland_k: f64,
    pub minimum_temperature_k: f64,
    pub canopy_rate_s: f64,
    pub spike_rate_m_s: f64,
    pub nozzle_rate_s: f64,
    pub coordination_speed_floor_m_s: f64,
    pub bank_limit_rad: f64,
    pub bank_cos_floor: f64,
    pub trim_initial: [f64; 3],
    pub trim_min: [f64; 3],
    pub trim_max: [f64; 3],
    pub backtracking_steps: usize,
    pub backtracking_ratio: f64,
}
pub const REGIMES: Regimes = Regimes {
    authority_mach: [0.8, 2.5],
    vortex_alpha_rad: [0.1, 0.3],
    wave_rise_mach: [0.95, 1.6],
    induced_rise_mach: [0.95, 2.5],
    inlet_alpha_rad: [0.16, 0.5],
    inlet_beta_rad: [0.07, 0.3],
    inlet_mismatch_mach: [1.5, 2.5],
    pitch_rate_normalized_limit: 0.3,
    pitch_damping_normalized_limit: 0.4,
    viscosity_reference_pa_s: 1.716e-5,
    viscosity_reference_k: 273.15,
    sutherland_k: 110.4,
    minimum_temperature_k: 150.,
    canopy_rate_s: 1. / 3.2,
    spike_rate_m_s: 0.12,
    nozzle_rate_s: 0.5,
    coordination_speed_floor_m_s: 70.,
    bank_limit_rad: 1.2,
    bank_cos_floor: 0.4,
    trim_initial: [0.08, 0., 0.8],
    trim_min: [-0.1, -1., 0.01],
    trim_max: [0.48, 1., 1.],
    backtracking_steps: 12,
    backtracking_ratio: 0.5,
};
