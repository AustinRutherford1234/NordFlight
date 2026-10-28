//! F/A-18F simulated flight control system. Commands only physical actuators.
//! Publicly described capabilities guide the architecture; numerical laws and
//! sensor/FCC failure boundaries are documented simulation estimates.
use super::*;
mod allocation;
mod laws;
pub mod sensors;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Normal,
    Degraded,
    Direct,
    Failed,
}
impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::Degraded => "DEGRADED",
            Self::Direct => "DIRECT",
            Self::Failed => "FAILED",
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Computer {
    pub enabled: bool,
    pub command_bias: DVec3,
    pub isolated: bool,
    pub mismatch: f64,
    pub recovery: f64,
    pub law: laws::Law,
}
impl Default for Computer {
    fn default() -> Self {
        Self {
            enabled: true,
            command_bias: DVec3::ZERO,
            isolated: false,
            mismatch: 0.,
            recovery: 0.,
            law: Default::default(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct State {
    pub mode: Mode,
    pub computers: [Computer; 4],
    pub sensors: sensors::Sensors,
    pub channels: usize,
    pub pilot: DVec3,
    pub output: [f64; 16],
    pub requested_rates: DVec3,
    pub requested_load: f64,
    pub alpha_limit: f64,
    pub protection: bool,
    pub allocation_residual: DVec3,
    pub saturation: f64,
    pub authority: DVec3,
    pub ground: bool,
    initialized: bool,
    engagement: f64,
    ground_timer: f64,
}
impl Default for State {
    fn default() -> Self {
        Self {
            mode: Mode::Normal,
            computers: [Computer::default(); 4],
            sensors: Default::default(),
            channels: 4,
            pilot: DVec3::ZERO,
            output: [0.; 16],
            requested_rates: DVec3::ZERO,
            requested_load: 1.,
            alpha_limit: 0.60,
            protection: false,
            allocation_residual: DVec3::ZERO,
            saturation: 0.,
            authority: DVec3::ZERO,
            ground: false,
            initialized: false,
            engagement: 0.,
            ground_timer: 0.,
        }
    }
}
pub fn powered(f: &Flight) -> bool {
    f.battery
        || f.apu
        || f.hornet
            .engines
            .iter()
            .any(|e| e.thrust > 0. && e.spool > 0.2)
}
pub fn actuator_power(f: &Flight) -> f64 {
    if powered(f)
        && f.damage.avionics > 0.1
        && f.hornet
            .fbw
            .computers
            .iter()
            .any(|c| c.enabled && !c.isolated)
    {
        1.
    } else {
        0.
    }
}
pub fn label(f: &Flight) -> &'static str {
    if !f.hornet.fbw.initialized && f.assist {
        return "STANDBY";
    }
    if !f.assist && f.hornet.fbw.mode != Mode::Failed {
        "DIRECT"
    } else {
        f.hornet.fbw.mode.label()
    }
}
pub fn summary(f: &Flight) -> String {
    format!(
        "FCS {} / {} CH / SENS {}/{}/{}",
        label(f),
        f.hornet.fbw.channels,
        f.hornet.fbw.sensors.counts[0],
        f.hornet.fbw.sensors.counts[1],
        f.hornet.fbw.sensors.counts[2]
    )
}
/// Simulation-time state is touched once per plant step. dt=0 coefficient probes
/// and trim solves never advance voting, integrators, or the engagement ramp.
pub fn commands(f: &mut Flight, q: f64, dt: f64) -> [f64; 16] {
    if dt <= 0. {
        return if f.assist {
            f.hornet.angles
        } else {
            aero::targets(f, f.controls)
        };
    }
    let mut state = std::mem::take(&mut f.hornet.fbw);
    state.sensors.sample(f, dt);
    let s = state.sensors.voted;
    let powered = powered(f);
    let thresholds = [0.10, 0.25, 0.50, 0.75];
    let available: [bool; 4] = std::array::from_fn(|i| {
        powered
            && state.computers[i].enabled
            && !state.computers[i].isolated
            && f.damage.avionics > thresholds[i]
    });
    state.channels = available.iter().filter(|x| **x).count();
    let effectors_lost = allocation::EFFECTORS.iter().any(|i| {
        f.damage.surfaces[*i] < 0.6
            || !matches!(
                f.damage.surface_state[*i],
                crate::combat::damage::SurfaceState::Operational
            )
    });
    let prior = state.mode;
    state.mode = if state.channels == 0 {
        Mode::Failed
    } else if !f.assist || state.channels == 1 || state.sensors.counts[0] < 2 {
        Mode::Direct
    } else if state.channels < 3
        || state.sensors.counts.iter().any(|c| *c < 3)
        || effectors_lost
        || f.damage.hydraulics * f.hornet.hydraulic_pressure < 0.5
    {
        Mode::Degraded
    } else {
        Mode::Normal
    };
    if prior != state.mode {
        for c in &mut state.computers {
            c.law = Default::default();
        }
        state.engagement = 0.;
    }
    let contact = f.landed
        || f.gear_compression.iter().any(|x| *x > 0.025)
        || f.carrier.attached
        || f.carrier.launch.is_some();
    state.ground_timer = if contact {
        (state.ground_timer + dt).min(0.2)
    } else {
        (state.ground_timer - dt).max(-0.2)
    };
    if !state.initialized {
        state.ground = contact;
        state.pilot = f.controls;
        state.output = f.hornet.angles;
        state.initialized = true;
    }
    if state.ground_timer > 0.025 {
        state.ground = true;
    }
    if state.ground_timer < -0.10 {
        state.ground = false;
    }
    let filter = 1. - (-dt / 0.10).exp();
    state.pilot += (f.controls - state.pilot) * filter;
    state.pilot = state.pilot.clamp(DVec3::splat(-1.), DVec3::ONE);
    let direct = aero::targets(f, f.controls);
    let mut output = match state.mode {
        Mode::Failed => f.hornet.angles,
        Mode::Direct => direct,
        _ => f.hornet.angles,
    };
    state.protection = false;
    if state.ground && matches!(state.mode, Mode::Normal | Mode::Degraded) {
        // Weight-on-wheels reactions balance static moments. Do not ask a
        // free-flight inverse to cancel engine/catapult loads against the deck.
        let request = (state.pilot
            - DVec3::new(s.rates.x * 0.4, s.rates.y * 0.5, -s.rates.z * 0.3))
        .clamp(DVec3::splat(-1.), DVec3::ONE);
        output = aero::targets(f, request);
        state.engagement = 0.;
        state.saturation = 0.;
        state.allocation_residual = DVec3::ZERO;
        state.requested_rates = DVec3::ZERO;
        state.requested_load = 0.;
        state.protection = false;
        state.output = output;
        f.surface = request;
        f.hornet.fbw = state;
        return output;
    }
    if state.mode == Mode::Degraded && (state.sensors.counts[1] < 2 || state.sensors.counts[2] < 2)
    {
        // Invalid air/load data removes the corresponding protections and
        // model inversion. Retain only gyro damping with conservative gains.
        let request = DVec3::new(
            state.pilot.x * 0.7 - s.rates.x * 0.65,
            state.pilot.y * 0.6 - s.rates.y * 0.6,
            state.pilot.z * 0.8 + s.rates.z * 0.4,
        )
        .clamp(DVec3::splat(-1.), DVec3::ONE);
        output = aero::targets(f, request);
        state.protection = false;
        state.engagement = 0.;
        state.output = output;
        f.surface = request;
        f.hornet.fbw = state;
        return output;
    }
    if matches!(state.mode, Mode::Normal | Mode::Degraded) && q > 40. {
        let mut demands = [laws::Demand::default(); 4];
        for i in 0..4 {
            if state.computers[i].enabled && powered {
                demands[i] = laws::demand(
                    f,
                    &s,
                    state.pilot,
                    state.mode,
                    state.ground,
                    &mut state.computers[i].law,
                    state.saturation > 0.15 || state.allocation_residual.length() > 0.8,
                    dt,
                );
                demands[i].acceleration += state.computers[i].command_bias;
            }
        }
        // Command comparison is independent from sensor comparison. Reject a bad
        // FCC output before it reaches an actuator; latch isolation after 100 ms.
        let mask = sensors::cohort(available, |i, j| {
            (demands[i].acceleration - demands[j].acceleration)
                .abs()
                .max_element()
                < 0.8
        });
        let mut voted = DVec3::ZERO;
        let mut voters = 0.;
        for i in 0..4 {
            if mask & (1 << i) != 0 {
                voted += demands[i].acceleration;
                voters += 1.;
            }
        }
        if voters >= 2. {
            voted /= voters;
            for i in 0..4 {
                let c = &mut state.computers[i];
                let close = (demands[i].acceleration - voted).abs().max_element() < 0.8;
                if available[i] && !close {
                    c.mismatch += dt;
                    if c.mismatch > 0.10 {
                        c.isolated = true;
                    }
                } else {
                    c.mismatch = (c.mismatch - dt).max(0.);
                }
                if c.isolated && c.enabled && close {
                    c.recovery += dt;
                    if c.recovery > 2. {
                        c.isolated = false;
                        c.recovery = 0.;
                        c.law = Default::default();
                    }
                } else {
                    c.recovery = 0.;
                }
            }
            let allocation =
                allocation::allocate(f, &s, voted, 0.5 * f.density * s.speed * s.speed);
            state.engagement = (state.engagement + dt / 0.30).min(1.);
            output = std::array::from_fn(|i| {
                f.hornet.angles[i] + (allocation.angles[i] - f.hornet.angles[i]) * state.engagement
            });
            // Manual flap selection remains authoritative at every computer mode.
            for i in [1, 5, 9, 10] {
                output[i] = direct[i];
            }
            let representative = (0..4).find(|i| mask & (1 << i) != 0).unwrap();
            state.requested_rates = demands[representative].rates;
            state.requested_load = demands[representative].load;
            state.alpha_limit = demands[representative].alpha_limit;
            state.protection = demands[representative].protected;
            state.allocation_residual = allocation.residual;
            state.saturation = allocation.saturation;
            state.authority = allocation.authority;
            // Transfer the settled tail bias to trim for a bumpless direct-mode entry.
            // This is automatic trim, not an attitude or flight-path reference.
            if !state.ground
                && state.pilot.x.abs() < 0.08
                && s.rates.x.abs() < 0.15
                && f.hornet.hydraulic_pressure > 0.3
            {
                let trim = (output[0] + output[4]) * 0.5;
                f.hornet.trim += (trim - f.hornet.trim).clamp(-0.025 * dt, 0.025 * dt);
                f.hornet.trim = f.hornet.trim.clamp(-0.4, 0.4);
            }
        } else {
            state.mode = Mode::Direct;
            output = direct;
            state.engagement = 0.;
        }
    } else if matches!(state.mode, Mode::Normal | Mode::Degraded) {
        output = direct;
    }
    state.output = output;
    f.surface = if state.mode == Mode::Direct {
        f.controls
    } else {
        state.pilot
    };
    f.hornet.fbw = state;
    output
}
