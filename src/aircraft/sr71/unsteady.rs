//! Per-element indicial lag, pressure/separation lag, vortex convection and wake state.
//! A reduced-order estimated state-space model, not a validated Beddoes-Leishman fit.
use super::{
    airframe_config::{SPATIAL, UNSTEADY},
    database,
};
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct State {
    pub fast: f64,
    pub slow: f64,
    pub pressure_alpha: f64,
    pub attached: f64,
    pub vortex: f64,
    pub vortex_age: f64,
    pub wake: f64,
    pub previous_alpha: f64,
    pub alpha_rate: f64,
    pub phase: f64,
    pub initialized: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            fast: 0.,
            slow: 0.,
            pressure_alpha: 0.,
            attached: 1.,
            vortex: 0.,
            vortex_age: 0.,
            wake: 0.,
            previous_alpha: 0.,
            alpha_rate: 0.,
            phase: 0.,
            initialized: false,
        }
    }
}
fn angle_delta(a: f64, b: f64) -> f64 {
    (a - b).sin().atan2((a - b).cos())
}
fn lag(value: &mut f64, target: f64, dt: f64, tau: f64) {
    *value += (target - *value) * (1. - (-dt / tau.max(0.005)).exp());
}
impl State {
    pub fn equilibrium(alpha: f64, beta: f64, mach: f64) -> Self {
        let p = database::lookup(mach, alpha, beta);
        Self {
            fast: alpha,
            slow: alpha,
            pressure_alpha: alpha,
            attached: p.attached,
            previous_alpha: alpha,
            initialized: true,
            ..Default::default()
        }
    }
    pub fn advance(&mut self, alpha: f64, beta: f64, mach: f64, speed: f64, chord: f64, dt: f64) {
        if !self.initialized
            || ![
                self.fast,
                self.slow,
                self.pressure_alpha,
                self.attached,
                self.vortex,
                self.wake,
            ]
            .iter()
            .all(|v| v.is_finite())
        {
            *self = Self::equilibrium(alpha, beta, mach);
        }
        if dt <= 0. {
            return;
        }
        let time = (chord / speed.max(SPATIAL.local_speed_floor)).clamp(0.012, 1.);
        let da = angle_delta(alpha, self.previous_alpha);
        self.alpha_rate = (da / dt).clamp(-8., 8.);
        self.previous_alpha = alpha;
        for (v, t) in [
            (&mut self.fast, UNSTEADY.fast_time),
            (&mut self.slow, UNSTEADY.slow_time),
            (&mut self.pressure_alpha, UNSTEADY.pressure_time),
        ] {
            let target = *v + angle_delta(alpha, *v);
            lag(v, target, dt, time * t);
            *v = v.sin().atan2(v.cos());
        }
        let effective = self.pressure_alpha - self.alpha_rate * time * UNSTEADY.dynamic_stall_delay;
        let hysteresis = UNSTEADY.stall_hysteresis_rad * (1. - self.attached) * alpha.signum();
        let target = database::lookup(mach, effective + hysteresis, beta).attached;
        let falling = target < self.attached;
        lag(
            &mut self.attached,
            target,
            dt,
            time * if falling {
                UNSTEADY.separation_time
            } else {
                UNSTEADY.recovery_time
            },
        );
        self.attached = self.attached.clamp(0., 1.);
        let loading = (self.alpha_rate * alpha.signum()).max(0.);
        let vortex_target = (UNSTEADY.vortex_gain * loading * time * (1. - self.attached))
            .min(UNSTEADY.vortex_limit)
            * alpha.signum();
        lag(
            &mut self.vortex,
            vortex_target,
            dt,
            time * UNSTEADY.vortex_time,
        );
        self.vortex_age = if vortex_target.abs() > 0.001 {
            (self.vortex_age + dt / time).min(8.)
        } else {
            (self.vortex_age - dt / time).max(0.)
        };
        lag(
            &mut self.wake,
            (1. - self.attached) * alpha.sin(),
            dt,
            time * UNSTEADY.wake_time,
        );
        self.phase = (self.phase
            + std::f64::consts::TAU * UNSTEADY.buffet_strouhal * speed / chord.max(0.5) * dt)
            .rem_euclid(std::f64::consts::TAU);
    }
    pub fn effective_alpha(&self) -> f64 {
        self.slow
            + UNSTEADY.fast_weight / (UNSTEADY.fast_weight + UNSTEADY.slow_weight)
                * angle_delta(self.fast, self.slow)
    }
    pub fn buffet(&self) -> f64 {
        UNSTEADY.buffet_gain
            * (1. - self.attached)
            * (self.phase.sin() + 0.35 * (self.phase * 2.).sin())
    }
    pub fn added_mass(&self, chord: f64, speed: f64) -> f64 {
        (UNSTEADY.added_mass_gain * self.alpha_rate * chord / speed.max(SPATIAL.local_speed_floor))
            .clamp(-UNSTEADY.added_mass_limit, UNSTEADY.added_mass_limit)
    }
}
