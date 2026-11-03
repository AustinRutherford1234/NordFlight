//! Independent shaft, fuel, ignition and inlet dynamics; no random failures.
use super::{
    config::*,
    gaspath::{self, Ambient, Geometry, Output, smooth},
    inlet,
};
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize)]
pub enum Mode {
    #[default]
    Off,
    Cranking,
    Running,
    Flameout,
    Surge,
    Seized,
}
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct Faults {
    pub spike_jammed: Option<f64>,
    pub bypass_jammed: Option<f64>,
    pub bleed_jammed: Option<f64>,
    pub nozzle_jammed_m2: Option<f64>,
    pub fuel_pump_scale: f64,
    pub ignition_failed: bool,
    pub seized: bool,
}
impl Default for Faults {
    fn default() -> Self {
        Self {
            spike_jammed: None,
            bypass_jammed: None,
            bleed_jammed: None,
            nozzle_jammed_m2: None,
            fuel_pump_scale: 1.,
            ignition_failed: false,
            seized: false,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub ambient: Ambient,
    pub alpha: f64,
    pub beta: f64,
    pub throttle: f64,
    pub run: bool,
    pub ignition: bool,
    pub ground_starter: bool,
    pub hydraulic: f64,
    pub health: f64,
    pub nozzle_health: f64,
    pub fuel_available: bool,
}
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct State {
    pub mode: Mode,
    pub n: f64,
    pub main_fuel_kg_s: f64,
    pub ab_fuel_kg_s: f64,
    pub afterburner_lit: bool,
    pub afterburner_fraction: f64,
    pub bleed: f64,
    pub throat_m2: f64,
    pub teb_remaining: u8,
    pub inlet: inlet::State,
    pub output: Output,
    pub faults: Faults,
    pub governor_integral: f64,
    pub ignition_timer_s: f64,
    pub restart_timer_s: f64,
    pub flameout_timer_s: f64,
    pub surge_timer_s: f64,
    pub surge_exposure_s: f64,
    pub surge_margin: f64,
    pub previous_throttle: f64,
    pub thrust_n: f64,
    pub drag_n: f64,
    pub fuel_used_kg: f64,
    pub flameouts: u32,
    pub surges: u32,
    pub invalid_steps: u32,
    pub shaft_energy_j: f64,
    pub shaft_applied_torque_nm: f64,
    pub shaft_work_j: f64,
    pub shaft_balance_residual_j: f64,
    pub shaft_limit_work_j: f64,
}
impl Default for State {
    fn default() -> Self {
        Self {
            mode: Mode::Off,
            n: 0.,
            main_fuel_kg_s: 0.,
            ab_fuel_kg_s: 0.,
            afterburner_lit: false,
            afterburner_fraction: 0.,
            bleed: 0.,
            throat_m2: 0.5,
            teb_remaining: DYN.teb_shots,
            inlet: inlet::State::default(),
            output: Output::default(),
            faults: Faults::default(),
            governor_integral: 0.,
            ignition_timer_s: 0.,
            restart_timer_s: 0.,
            flameout_timer_s: 0.,
            surge_timer_s: 0.,
            surge_exposure_s: 0.,
            surge_margin: DYN.surge_margin,
            previous_throttle: 0.,
            thrust_n: 0.,
            drag_n: 0.,
            fuel_used_kg: 0.,
            flameouts: 0,
            surges: 0,
            invalid_steps: 0,
            shaft_energy_j: 0.,
            shaft_applied_torque_nm: 0.,
            shaft_work_j: 0.,
            shaft_balance_residual_j: 0.,
            shaft_limit_work_j: 0.,
        }
    }
}
fn approach(x: &mut f64, target: f64, tau: f64, dt: f64, rate: f64) {
    let desired = (target - *x) * (1. - (-dt / tau).exp());
    *x += desired.clamp(-rate * dt, rate * dt);
}
impl State {
    pub fn geometry(&self, health: f64) -> Geometry {
        Geometry {
            recovery: self.inlet.recovery,
            bleed: self.bleed,
            throat_m2: self.throat_m2,
            flow_scale: 1.,
            health,
        }
    }
    pub fn equilibrium(input: Input) -> Self {
        let mut s = Self::default();
        if !input.ambient.valid() || !input.throttle.is_finite() {
            s.invalid_steps = 1;
            return s;
        }
        s.n = gaspath::demand_speed(input.throttle, input.ambient.mach);
        s.inlet = inlet::State::equilibrium(input.ambient, input.alpha, input.beta);
        s.bleed = GAS.bleed_fraction * smooth(1.6, 3., input.ambient.mach);
        s.mode = if input.run && input.fuel_available {
            Mode::Running
        } else {
            Mode::Off
        };
        if s.mode == Mode::Running {
            let g = Geometry {
                throat_m2: 0.,
                ..s.geometry(input.health)
            };
            let cold = gaspath::evaluate(input.ambient, s.n, g, 0., 0.);
            let target = gaspath::balanced_t4(&cold).clamp(
                (cold.compressor.total_temperature_k + 1.).min(GAS.max_t4_k),
                GAS.max_t4_k,
            );
            let mut lo = 0.;
            let mut hi = gaspath::fuel_for_t4(&cold, GAS.max_t4_k);
            for _ in 0..24 {
                let fuel = (lo + hi) * 0.5;
                let out = gaspath::evaluate(input.ambient, s.n, g, fuel, 0.);
                if out.shaft_power_w < 0. {
                    lo = fuel
                } else {
                    hi = fuel
                }
            }
            s.main_fuel_kg_s = if cold.compressor.mass_flow_kg_s > 0. {
                (lo + hi) * 0.5
            } else {
                gaspath::fuel_for_t4(&cold, target)
            };
            let dry = gaspath::evaluate(input.ambient, s.n, g, s.main_fuel_kg_s, 0.);
            s.afterburner_fraction = ((input.throttle - gaspath::GAS_BURNER_START)
                / (1. - gaspath::GAS_BURNER_START))
                .clamp(0., 1.);
            s.afterburner_lit = s.afterburner_fraction > 0.;
            s.ab_fuel_kg_s = gaspath::ab_fuel(&dry, s.afterburner_fraction);
            s.output = gaspath::evaluate(input.ambient, s.n, g, s.main_fuel_kg_s, s.ab_fuel_kg_s);
            s.throat_m2 = s
                .output
                .required_throat_m2
                .clamp(DYN.nozzle_min_m2, DYN.nozzle_max_m2);
            s.thrust_n = s.output.net_thrust_n.max(0.);
            s.teb_remaining = DYN.teb_shots - 1 - u8::from(s.afterburner_lit);
        }
        s.previous_throttle = input.throttle;
        s.shaft_energy_j = 0.5
            * DYN.rotor_inertia_kg_m2
            * (s.n * DYN.rpm_max * std::f64::consts::TAU / 60.).powi(2);
        s
    }
    pub fn rpm(&self) -> f64 {
        self.n * DYN.rpm_max
    }
    pub fn visual_power(&self) -> f64 {
        if self.mode != Mode::Running {
            return 0.;
        }
        let idle = DYN.rpm_idle / DYN.rpm_max;
        let dry = (0.28 + 0.60 * ((self.n - idle) / (1. - idle)).clamp(0., 1.)).min(0.88);
        dry + 0.12 * self.afterburner_fraction
    }
    pub fn advance(&mut self, input: Input, dt: f64) {
        if dt <= 0.
            || !dt.is_finite()
            || !input.ambient.valid()
            || ![
                input.alpha,
                input.beta,
                input.throttle,
                input.health,
                input.hydraulic,
                input.nozzle_health,
            ]
            .iter()
            .all(|v| v.is_finite())
            || !self.faults.fuel_pump_scale.is_finite()
            || [
                self.faults.spike_jammed,
                self.faults.bypass_jammed,
                self.faults.bleed_jammed,
                self.faults.nozzle_jammed_m2,
            ]
            .iter()
            .flatten()
            .any(|v| !v.is_finite())
            || !(0.0..=1.0).contains(&input.health)
            || !(0.0..=1.0).contains(&input.hydraulic)
            || !(0.0..=1.0).contains(&input.nozzle_health)
        {
            self.invalid_steps += 1;
            self.mode = Mode::Off;
            self.main_fuel_kg_s = 0.;
            self.ab_fuel_kg_s = 0.;
            self.afterburner_lit = false;
            self.thrust_n = 0.;
            self.fuel_used_kg = 0.;
            self.afterburner_fraction = 0.;
            return;
        }
        let a = input.ambient;
        let run = input.run
            && input.fuel_available
            && input.health > 0.02
            && self.faults.fuel_pump_scale > 0.;
        let rate = (input.throttle - self.previous_throttle) / dt;
        self.previous_throttle = input.throttle;
        self.restart_timer_s = (self.restart_timer_s - dt).max(0.);
        self.inlet.advance(
            a,
            input.alpha,
            input.beta,
            self.output.inlet.mass_flow_kg_s,
            rate,
            input.hydraulic,
            self.faults.spike_jammed,
            self.faults.bypass_jammed,
            dt,
        );
        let bleed_target = GAS.bleed_fraction * smooth(1.6, 3., a.mach);
        approach(
            &mut self.bleed,
            self.faults
                .bleed_jammed
                .unwrap_or(bleed_target)
                .clamp(0., 0.35),
            DYN.bleed_time_s,
            dt,
            1.,
        );
        if self.faults.seized || input.health < 0.02 {
            self.mode = Mode::Seized;
            self.n = 0.;
        } else if !run {
            self.mode = Mode::Off;
            self.afterburner_lit = false;
            self.governor_integral = 0.;
        } else if matches!(self.mode, Mode::Off | Mode::Flameout | Mode::Cranking) {
            let can_restart = input.ground_starter
                || (self.n >= DYN.ignition_speed
                    && a.mach >= 0.25
                    && a.mach < 3.5
                    && self.inlet.recovery > 0.4);
            if input.ground_starter {
                self.mode = Mode::Cranking;
            }
            if can_restart
                && input.ignition
                && !self.faults.ignition_failed
                && self.n >= DYN.ignition_speed
                && self.teb_remaining > 0
                && self.restart_timer_s <= 0.
                && a.pressure >= 1000.
            {
                self.ignition_timer_s += dt;
                if self.ignition_timer_s >= DYN.ignition_delay_s {
                    self.mode = Mode::Running;
                    self.teb_remaining -= 1;
                    self.ignition_timer_s = 0.;
                }
            } else {
                self.ignition_timer_s = 0.;
            }
        }
        self.surge_timer_s = (self.surge_timer_s - dt).max(0.);
        if self.mode == Mode::Surge && self.surge_timer_s <= 0. {
            self.mode = Mode::Flameout;
            self.restart_timer_s = DYN.restart_cooldown_s;
        }
        let cold = gaspath::evaluate(a, self.n, self.geometry(input.health), 0., 0.);
        let desired = gaspath::demand_speed(input.throttle, a.mach);
        let error = desired - self.n;
        let target_t4 =
            (gaspath::balanced_t4(&cold) + DYN.governor_gain_k * error + self.governor_integral)
                .clamp(
                    cold.compressor.total_temperature_k.min(GAS.max_t4_k),
                    GAS.max_t4_k,
                );
        if self.mode == Mode::Running && self.n > DYN.self_sustain_speed && error.abs() < 0.1 {
            self.governor_integral = (self.governor_integral
                + error * DYN.governor_integral_gain * dt)
                .clamp(-DYN.governor_integral_limit, DYN.governor_integral_limit);
        }
        let mut metered = cold;
        // Meter against observed flow as well as the cold compressor demand.
        // A finite-area nozzle can restrict hot flow during a start/AB transient.
        if self.output.compressor.mass_flow_kg_s > 0. {
            metered.compressor.mass_flow_kg_s = metered
                .compressor
                .mass_flow_kg_s
                .min(self.output.compressor.mass_flow_kg_s);
        }
        let fuel_target = if self.mode == Mode::Running && run {
            gaspath::fuel_for_t4(&metered, target_t4) * self.faults.fuel_pump_scale.clamp(0., 1.)
        } else {
            0.
        };
        approach(
            &mut self.main_fuel_kg_s,
            fuel_target,
            DYN.fuel_time_s,
            dt,
            DYN.max_fuel_rate_kg_s2,
        );
        if !run || self.mode == Mode::Seized {
            self.main_fuel_kg_s = 0.;
        }
        let dry = gaspath::evaluate(
            a,
            self.n,
            self.geometry(input.health),
            self.main_fuel_kg_s,
            0.,
        );
        let ab_request = ((input.throttle - gaspath::GAS_BURNER_START)
            / (1. - gaspath::GAS_BURNER_START))
            .clamp(0., 1.);
        let ab_possible = self.mode == Mode::Running
            && run
            && self.n > DYN.ab_min_speed
            && self.inlet.recovery > 0.5;
        if ab_request <= 0. || !ab_possible {
            self.afterburner_lit = false;
        } else if !self.afterburner_lit
            && input.ignition
            && !self.faults.ignition_failed
            && self.teb_remaining > 0
        {
            self.afterburner_lit = true;
            self.teb_remaining -= 1;
        }
        let ab_target = if self.afterburner_lit {
            gaspath::ab_fuel(&dry, ab_request)
        } else {
            0.
        };
        approach(
            &mut self.ab_fuel_kg_s,
            ab_target,
            DYN.ab_time_s,
            dt,
            DYN.max_fuel_rate_kg_s2 * 2.,
        );
        if !run || !ab_possible {
            self.ab_fuel_kg_s = 0.;
        }
        self.afterburner_fraction = if ab_possible && self.afterburner_lit {
            (self.ab_fuel_kg_s / gaspath::ab_fuel(&dry, 1.).max(1e-6)).clamp(0., 1.)
        } else {
            0.
        };
        let unrestricted = gaspath::evaluate(
            a,
            self.n,
            Geometry {
                throat_m2: 0.,
                ..self.geometry(input.health)
            },
            self.main_fuel_kg_s,
            self.ab_fuel_kg_s,
        );
        if let Some(area) = self.faults.nozzle_jammed_m2 {
            self.throat_m2 = area.clamp(DYN.nozzle_min_m2, DYN.nozzle_max_m2);
        } else if input.nozzle_health > 0.1 {
            approach(
                &mut self.throat_m2,
                unrestricted
                    .required_throat_m2
                    .clamp(DYN.nozzle_min_m2, DYN.nozzle_max_m2),
                DYN.nozzle_time_s,
                dt,
                3. * input.hydraulic,
            );
        }
        self.output = gaspath::evaluate(
            a,
            self.n,
            self.geometry(input.health),
            self.main_fuel_kg_s,
            self.ab_fuel_kg_s,
        );
        let restricted = (1. - self.output.nozzle_capacity_ratio).max(0.);
        let geometry_error =
            (self.throat_m2 / unrestricted.required_throat_m2.max(0.01) - 1.).abs();
        self.surge_margin = DYN.surge_margin
            - 0.5 * self.inlet.distortion
            - 0.45 * restricted
            - 0.15 * geometry_error
            - 0.3 * (bleed_target - self.bleed).max(0.)
            - 0.12 * (1. - input.health);
        if self.n > 0.55 && self.mode == Mode::Running {
            self.surge_exposure_s = if self.surge_margin < 0. {
                self.surge_exposure_s + dt
            } else {
                0.
            };
            if self.surge_exposure_s > DYN.surge_dwell_s {
                self.mode = Mode::Surge;
                self.surges += 1;
                self.surge_timer_s = DYN.surge_duration_s;
                self.main_fuel_kg_s *= 0.2;
                self.afterburner_lit = false;
                self.ab_fuel_kg_s = 0.;
            }
        }
        let ratio = self.main_fuel_kg_s / self.output.compressor.mass_flow_kg_s.max(1e-6);
        if self.mode == Mode::Running {
            let adverse = self.n < DYN.ignition_speed
                || self.output.inlet.total_pressure_pa < DYN.ignition_min_pressure_pa
                || ratio > DYN.rich_limit
                || ratio < DYN.lean_limit;
            self.flameout_timer_s = if adverse {
                self.flameout_timer_s + dt
            } else {
                0.
            };
            if self.flameout_timer_s > DYN.flameout_dwell_s {
                self.mode = Mode::Flameout;
                self.flameouts += 1;
                self.restart_timer_s = DYN.restart_cooldown_s;
                self.afterburner_lit = false;
                self.ab_fuel_kg_s = 0.;
            }
        }
        if self.mode != Mode::Running {
            self.main_fuel_kg_s = 0.;
            self.ab_fuel_kg_s = 0.;
            self.afterburner_lit = false;
            self.afterburner_fraction = 0.;
            self.output = gaspath::evaluate(a, self.n, self.geometry(input.health), 0., 0.);
        }
        let omega_max = DYN.rpm_max * std::f64::consts::TAU / 60.;
        let omega = self.n * omega_max;
        let starter_torque = if input.ground_starter && run && self.n < 0.55 {
            DYN.starter_torque_nm
                .min(DYN.starter_power_w / omega.max(1.))
        } else {
            0.
        };
        let windmill = DYN.max_windmill_speed * smooth(0.0, 2.5, a.mach);
        let available_wind_power =
            0.5 * a.density * a.speed.powi(3) * INLET.capture_area_m2 * DYN.windmill_drag_cd;
        let raw_wind_torque =
            -DYN.rotor_inertia_kg_m2 * omega_max * (self.n - windmill) / DYN.windmill_time_s;
        let wind_torque = if self.mode != Mode::Running {
            if raw_wind_torque > 0. {
                raw_wind_torque.min(available_wind_power / omega.max(20.))
            } else {
                raw_wind_torque
            }
        } else {
            0.
        };
        let gas_torque = if self.mode == Mode::Running {
            self.output.shaft_power_w / omega.max(omega_max * 0.10)
        } else {
            0.
        };
        let old_energy = 0.5 * DYN.rotor_inertia_kg_m2 * omega.powi(2);
        self.shaft_applied_torque_nm = gas_torque + starter_torque + wind_torque;
        let free_omega = omega + self.shaft_applied_torque_nm * dt / DYN.rotor_inertia_kg_m2;
        if self.mode != Mode::Seized {
            self.n = (free_omega / omega_max).clamp(0., 1.06);
        }
        let new_omega = self.n * omega_max;
        self.shaft_energy_j = 0.5 * DYN.rotor_inertia_kg_m2 * new_omega.powi(2);
        self.shaft_work_j = self.shaft_applied_torque_nm * 0.5 * (omega + free_omega) * dt;
        self.shaft_limit_work_j =
            self.shaft_energy_j - 0.5 * DYN.rotor_inertia_kg_m2 * free_omega.powi(2);
        self.shaft_balance_residual_j =
            self.shaft_energy_j - old_energy - self.shaft_work_j - self.shaft_limit_work_j;
        let cold_drag = if self.mode != Mode::Running {
            0.5 * a.density * a.speed * a.speed * INLET.capture_area_m2 * DYN.windmill_drag_cd
        } else {
            0.
        };
        self.drag_n = self.inlet.drag_n + cold_drag;
        if self.mode == Mode::Running {
            self.drag_n += (-self.output.net_thrust_n).max(0.);
        }
        self.thrust_n = if self.mode == Mode::Running && run {
            self.output.net_thrust_n.max(0.)
        } else {
            0.
        };
        self.fuel_used_kg = (self.main_fuel_kg_s + self.ab_fuel_kg_s) * dt;
    }
}
