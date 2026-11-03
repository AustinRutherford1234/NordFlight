use super::{config::*, math::*};
use crate::aerodynamics::{Coefficients, Conditions, FlowState};
use std::f64::consts::PI;
pub fn induced(lift: f64, attached: f64, beta: f64, height: f64, mach: f64) -> f64 {
    let efficiency = (AERO.efficiency
        - AERO.separation_efficiency_loss * (1. - attached)
        - AERO.slip_efficiency_loss * beta.abs())
    .clamp(AERO.minimum_efficiency, AERO.maximum_efficiency);
    let ground = (AERO.ground_height_scale * height.max(0.) / SPAN).powi(2);
    let g = (ground / (1. + ground)).clamp(AERO.minimum_ground_factor, 1.);
    lift * lift / (PI * (SPAN * SPAN / AREA) * efficiency)
        * g
        * (1.
            + AERO.supersonic_induced_gain
                * smooth(
                    REGIMES.induced_rise_mach[0],
                    REGIMES.induced_rise_mach[1],
                    mach,
                ))
}
fn attached_target(alpha: f64, mach: f64, history: f64) -> f64 {
    let onset = AERO.separation_onset
        - AERO.supersonic_onset_reduction
            * smooth(REGIMES.authority_mach[0], REGIMES.authority_mach[1], mach)
        - AERO.hysteresis * (1. - history);
    1. - smooth(
        onset - AERO.separation_width_before,
        onset + AERO.separation_width_after,
        alpha.abs(),
    )
}
/// Invalid evaluations produce no aerodynamic load; checked callers receive the reason.
pub fn evaluate(c: Conditions, state: &mut FlowState, dt: f64) -> Coefficients {
    evaluate_checked(c, state, dt).unwrap_or_default()
}
pub fn evaluate_checked(
    c: Conditions,
    state: &mut FlowState,
    dt: f64,
) -> Result<Coefficients, super::diagnostics::EvaluationError> {
    super::diagnostics::validate_conditions(c, dt)?;
    if !super::diagnostics::flow_is_valid(state) {
        *state = FlowState {
            lag_alpha: c.alpha - INCIDENCE,
            lag_beta: c.beta,
            ..Default::default()
        };
    }
    Ok(evaluate_valid(c, state, dt))
}
fn evaluate_valid(c: Conditions, state: &mut FlowState, dt: f64) -> Coefficients {
    let m = c.mach.max(0.);
    let v = c.speed.max(0.01);
    let a = (c.alpha - INCIDENCE)
        .sin()
        .atan2((c.alpha - INCIDENCE).cos());
    let tau = (CHORD / v.max(AERO.lag_speed_floor_m_s) * AERO.convection_factor)
        .clamp(AERO.min_lag_s, AERO.max_lag_s);
    let decay = 1. - (-dt.max(0.) / tau).exp();
    let old = state.lag_alpha;
    state.lag_alpha += (a - state.lag_alpha)
        .sin()
        .atan2((a - state.lag_alpha).cos())
        * decay;
    state.lag_alpha = state.lag_alpha.sin().atan2(state.lag_alpha.cos());
    state.lag_beta +=
        (c.beta - state.lag_beta) * (1. - (-dt.max(0.) / (tau * AERO.sideslip_time_factor)).exp());
    for (i, side) in [-1., 1.].into_iter().enumerate() {
        let local =
            state.lag_alpha - side * c.rates.z * SPAN / (4. * v.max(AERO.lag_speed_floor_m_s));
        let target = attached_target(local, m, state.wing_attached[i]);
        let t = tau
            * if target < state.wing_attached[i] {
                AERO.separation_time_factor
            } else {
                AERO.reattachment_time_factor
            };
        state.wing_attached[i] +=
            (target - state.wing_attached[i]) * (1. - (-dt.max(0.) / t).exp());
    }
    state.attached = (state.wing_attached[0] + state.wing_attached[1]) * 0.5;
    let adot = if dt > 1e-9 {
        (state.lag_alpha - old)
            .sin()
            .atan2((state.lag_alpha - old).cos())
            / dt
    } else {
        0.
    };
    let vortex_target = (adot * CHORD / v.max(AERO.lag_speed_floor_m_s)
        * AERO.vortex_transient_gain)
        .clamp(-AERO.vortex_transient_limit, AERO.vortex_transient_limit)
        * smooth(
            REGIMES.vortex_alpha_rad[0],
            REGIMES.vortex_alpha_rad[1],
            a.abs(),
        );
    state.vortex_lift += (vortex_target - state.vortex_lift)
        * (1. - (-dt.max(0.) / (tau * AERO.vortex_time_factor)).exp());
    let attached = state.attached;
    let p = -c.rates.z * SPAN / (2. * v.max(AERO.lag_speed_floor_m_s));
    let q = c.rates.x * CHORD / (2. * v.max(AERO.lag_speed_floor_m_s));
    let r = -c.rates.y * SPAN / (2. * v.max(AERO.lag_speed_floor_m_s));
    let slope = tab(&CNA, m) * DEG;
    let vortex = AERO.chine_lift_gain
        * a.sin()
        * a.sin().abs()
        * a.cos()
        * smooth(
            REGIMES.vortex_alpha_rad[0],
            REGIMES.vortex_alpha_rad[1],
            a.abs(),
        )
        * (1. - AERO.supersonic_chine_loss * smooth(0.9, 3., m));
    let linear = tab(&CNB, m)
        + slope * state.lag_alpha.sin() * state.lag_alpha.cos()
        + vortex
        + state.vortex_lift;
    let flat = AERO.flat_plate_normal_gain * (2. * a).sin();
    let cn = linear * attached + flat * (1. - attached);
    let cn = cn
        + tab(&CNQ, m)
            * q.clamp(
                -REGIMES.pitch_rate_normalized_limit,
                REGIMES.pitch_rate_normalized_limit,
            );
    // Normal force projected to the wind lift axis; drag is independently dissipative.
    let lift = cn * a.cos();
    let mu = REGIMES.viscosity_reference_pa_s
        * (c.temperature.max(REGIMES.minimum_temperature_k) / REGIMES.viscosity_reference_k)
            .powf(1.5)
        * (REGIMES.viscosity_reference_k + REGIMES.sutherland_k)
        / (c.temperature.max(REGIMES.minimum_temperature_k) + REGIMES.sutherland_k);
    let re = (c.density.max(0.) * v * CHORD / mu).max(AERO.minimum_reynolds);
    let cf = AERO.skin_friction_gain
        / (re.log10().powf(AERO.reynolds_exponent)
            * (1. + AERO.compressibility_gain * m * m).powf(AERO.compressibility_exponent));
    let parasite = AERO.parasite_base
        + cf * AERO.wetted_area_ratio
        + AERO.sideslip_drag * c.beta.sin().powi(2)
        + if c.gear { AERO.gear_drag } else { 0. };
    let wave = AERO.wave_peak
        * (-((m - AERO.wave_center_mach) / AERO.wave_width_mach).powi(2)).exp()
        + AERO.supersonic_wave * smooth(REGIMES.wave_rise_mach[0], REGIMES.wave_rise_mach[1], m);
    let induced = induced(lift, attached, c.beta, c.height, m);
    let separated = (1. - attached) * AERO.separated_drag * a.sin().powi(2)
        + AERO.incidence_drag * (1. - a.cos()).max(0.);
    let pitch = tab(&CMB, m)
        + tab(&CMA, m) * DEG * a
        + AERO.separated_pitch * (-a.sin()) * (1. - attached)
        + tab(&CMQ, m)
            * q.clamp(
                -REGIMES.pitch_damping_normalized_limit,
                REGIMES.pitch_damping_normalized_limit,
            );
    let cy = tab(&CYB, m) * DEG * state.lag_beta.sin()
        - AERO.side_yaw_damping * c.rates.y * SPAN / (2. * v.max(AERO.lag_speed_floor_m_s));
    let cl = tab(&CLB, m) * DEG * c.beta.sin() + tab(&CLP, m) * p + AERO.roll_yaw_coupling * r;
    let cn_yaw =
        tab(&CNBETA, m) * DEG * c.beta.sin() + tab(&CNR, m) * r - AERO.yaw_roll_coupling * p;
    Coefficients {
        lift,
        drag: (parasite + wave + induced + separated).max(0.),
        side: cy,
        pitch,
        yaw: -cn_yaw,
        roll: -cl
            + AERO.asymmetric_separation_roll * (state.wing_attached[0] - state.wing_attached[1]),
        parasite,
        induced,
        wave,
        attached,
        control_scale: (AERO.control_floor + AERO.control_attached_gain * attached)
            * (1. - AERO.control_mach_loss * smooth(0.8, 3., m)),
        ..Default::default()
    }
}
pub fn steady(c: Conditions) -> Coefficients {
    let a = c.alpha - INCIDENCE;
    let mut s = FlowState {
        lag_alpha: a,
        lag_beta: c.beta,
        ..Default::default()
    };
    let target = attached_target(a, c.mach, 1.);
    s.wing_attached = [target; 2];
    s.attached = target;
    evaluate(c, &mut s, 0.)
}
