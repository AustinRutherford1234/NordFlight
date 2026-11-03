//! Pressure-recovery and shock-margin state machine. Thresholds/controllers are estimates.
use super::{
    config::*,
    gaspath::{self, Ambient, smooth},
};
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize)]
pub enum Mode {
    #[default]
    Subsonic,
    Started,
    Unstarted,
    Restarting,
}
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct State {
    pub mode: Mode,
    pub spike_m: f64,
    pub bypass: f64,
    pub recovery: f64,
    pub shock: f64,
    pub distortion: f64,
    pub margin: f64,
    pub adverse_time_s: f64,
    pub recovery_time_s: f64,
    pub unstarts: u32,
    pub drag_n: f64,
}
impl Default for State {
    fn default() -> Self {
        Self {
            mode: Mode::Subsonic,
            spike_m: 0.,
            bypass: 0.,
            recovery: 0.985,
            shock: 1.,
            distortion: 0.,
            margin: INLET.nominal_margin,
            adverse_time_s: 0.,
            recovery_time_s: 0.,
            unstarts: 0,
            drag_n: 0.,
        }
    }
}
pub fn spike_target(mach: f64) -> f64 {
    INLET.spike_travel_m
        * ((mach - INLET.spike_start_mach) / (INLET.spike_full_mach - INLET.spike_start_mach))
            .clamp(0., 1.)
}
pub fn distortion(alpha: f64, beta: f64) -> f64 {
    0.5 * smooth(
        INLET.alpha_distortion_start,
        INLET.alpha_distortion_end,
        alpha.abs(),
    ) + 0.65
        * smooth(
            INLET.beta_distortion_start,
            INLET.beta_distortion_end,
            beta.abs(),
        )
}
impl State {
    pub fn equilibrium(a: Ambient, alpha: f64, beta: f64) -> Self {
        let mut s = Self::default();
        s.spike_m = spike_target(a.mach);
        s.bypass = 0.08 * smooth(1.6, 3.2, a.mach);
        s.distortion = distortion(alpha, beta) * smooth(0.25, 1., a.mach);
        s.recovery =
            gaspath::interpolate(&RECOVERY_MACH, &RECOVERY, a.mach) * (1. - 0.30 * s.distortion);
        s.drag_n = 0.5
            * a.density
            * a.speed
            * a.speed
            * INLET.capture_area_m2
            * INLET.spillage_cd
            * s.bypass;
        s.mode = if a.mach > INLET.supersonic_start_mach {
            Mode::Started
        } else {
            Mode::Subsonic
        };
        s
    }
    #[allow(clippy::too_many_arguments)]
    pub fn advance(
        &mut self,
        a: Ambient,
        alpha: f64,
        beta: f64,
        demand_kg_s: f64,
        throttle_rate: f64,
        authority: f64,
        spike_jam: Option<f64>,
        bypass_jam: Option<f64>,
        dt: f64,
    ) {
        self.distortion = distortion(alpha, beta) * smooth(0.25, 1., a.mach);
        let restart = matches!(self.mode, Mode::Unstarted | Mode::Restarting);
        let normal_spike = spike_target(a.mach);
        let target = if self.mode == Mode::Unstarted {
            0.
        } else {
            normal_spike
        };
        if let Some(position) = spike_jam {
            self.spike_m = position.clamp(0., INLET.spike_travel_m);
        } else {
            self.spike_m += (target - self.spike_m).clamp(
                -INLET.spike_rate_m_s * authority * dt,
                INLET.spike_rate_m_s * authority * dt,
            );
        }
        let bypass_target = if restart {
            0.95
        } else {
            (0.08 * smooth(1.6, 3.2, a.mach) + (-throttle_rate * 0.15).max(0.)).clamp(0., 0.8)
        };
        if let Some(position) = bypass_jam {
            self.bypass = position.clamp(0., 1.);
        } else {
            self.bypass += (bypass_target - self.bypass).clamp(
                -INLET.bypass_rate_s * authority * dt,
                INLET.bypass_rate_s * authority * dt,
            );
        }
        let capture = (a.density * a.speed * INLET.capture_area_m2).max(1.);
        let excess = (demand_kg_s / capture - 1.).max(0.);
        let mismatch = (self.spike_m - normal_spike).abs() / INLET.spike_travel_m;
        self.margin = INLET.nominal_margin
            - INLET.spike_mismatch_penalty * mismatch
            - INLET.distortion_penalty * self.distortion
            - INLET.demand_penalty * excess
            + 0.18 * self.bypass;
        if a.mach <= INLET.supersonic_start_mach {
            self.mode = Mode::Subsonic;
            self.adverse_time_s = 0.;
            self.recovery_time_s = 0.;
        } else {
            match self.mode {
                Mode::Subsonic => self.mode = Mode::Started,
                Mode::Started => {
                    self.adverse_time_s = if self.margin < 0. {
                        self.adverse_time_s + dt
                    } else {
                        0.
                    };
                    if self.adverse_time_s >= INLET.unstart_dwell_s {
                        self.mode = Mode::Unstarted;
                        self.unstarts += 1;
                        self.recovery_time_s = 0.;
                    }
                }
                Mode::Unstarted => {
                    self.recovery_time_s += dt;
                    if self.recovery_time_s > INLET.recovery_hold_s
                        && self.spike_m < 0.15
                        && self.bypass > 0.5
                        && self.distortion < 0.35
                    {
                        self.mode = Mode::Restarting;
                        self.recovery_time_s = 0.;
                    }
                }
                Mode::Restarting => {
                    self.recovery_time_s += dt;
                    if self.recovery_time_s > INLET.recovery_hold_s
                        && mismatch < 0.1
                        && self.distortion < 0.35
                    {
                        self.mode = Mode::Started;
                        self.adverse_time_s = 0.;
                    }
                }
            }
        }
        let started =
            gaspath::interpolate(&RECOVERY_MACH, &RECOVERY, a.mach) * (1. - 0.30 * self.distortion);
        let desired_recovery = if self.mode == Mode::Unstarted {
            INLET.unstarted_recovery
        } else if self.mode == Mode::Restarting {
            started * (0.6 + 0.4 * (1. - mismatch))
        } else {
            started
        };
        let desired_shock = if self.mode == Mode::Unstarted { 0. } else { 1. };
        let decay = 1. - (-dt / INLET.shock_time_s).exp();
        self.shock += (desired_shock - self.shock) * decay;
        self.recovery += (desired_recovery - self.recovery) * decay;
        let q = 0.5 * a.density * a.speed * a.speed;
        self.drag_n = q
            * INLET.capture_area_m2
            * (INLET.spillage_cd * (self.bypass + excess)
                + INLET.unstart_drag_cd * (1. - self.shock));
    }
}
