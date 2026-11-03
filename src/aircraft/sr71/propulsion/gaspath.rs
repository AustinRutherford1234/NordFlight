//! Quasi-steady station energy/mass balances coupled to dynamic shaft/actuators.
use super::config::*;
use serde::Serialize;
#[derive(Clone, Copy, Debug)]
pub struct Ambient {
    pub density: f64,
    pub temperature: f64,
    pub pressure: f64,
    pub speed: f64,
    pub axial_speed: f64,
    pub mach: f64,
}
impl Ambient {
    pub fn valid(self) -> bool {
        [
            self.density,
            self.temperature,
            self.pressure,
            self.speed,
            self.axial_speed,
            self.mach,
        ]
        .iter()
        .all(|x| x.is_finite())
            && self.density >= 0.
            && self.temperature > 0.
            && self.pressure >= 0.
            && self.speed >= 0.
            && self.mach >= 0.
            && self.mach <= 6.
            && self.density <= 5.
            && self.temperature <= 2000.
            && self.pressure <= 2e6
    }
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct Station {
    pub total_pressure_pa: f64,
    pub total_temperature_k: f64,
    pub mass_flow_kg_s: f64,
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct Output {
    pub inlet: Station,
    pub front_compressor: Station,
    pub compressor: Station,
    pub combustor: Station,
    pub turbine: Station,
    pub afterburner: Station,
    pub nozzle: Station,
    pub compressor_ratio: f64,
    pub turbine_ratio: f64,
    pub corrected_speed: f64,
    pub corrected_flow_kg_s: f64,
    pub main_fuel_kg_s: f64,
    pub ab_fuel_kg_s: f64,
    pub bleed_air_kg_s: f64,
    pub secondary_air_kg_s: f64,
    pub compressor_power_w: f64,
    pub turbine_power_w: f64,
    pub accessory_power_w: f64,
    pub shaft_power_w: f64,
    pub gross_momentum_n: f64,
    pub pressure_thrust_n: f64,
    pub ram_drag_n: f64,
    pub net_thrust_n: f64,
    pub nozzle_matching_residual_kg_s: f64,
    pub nozzle_capacity_ratio: f64,
    pub required_throat_m2: f64,
    pub exit_area_m2: f64,
    pub exit_pressure_pa: f64,
    pub exit_velocity_m_s: f64,
    pub exit_mach: f64,
    pub choked: bool,
    pub air_mass_residual_kg_s: f64,
    pub combustor_energy_residual_w: f64,
    pub turbine_energy_residual_w: f64,
    pub gas_energy_residual_w: f64,
}
pub fn interpolate(axis: &[f64], values: &[f64], x: f64) -> f64 {
    let x = x.clamp(axis[0], axis[axis.len() - 1]);
    let i = axis
        .windows(2)
        .position(|a| x <= a[1])
        .unwrap_or(axis.len() - 2);
    values[i] + (values[i + 1] - values[i]) * (x - axis[i]) / (axis[i + 1] - axis[i])
}
pub fn mass_flux(m: f64, gamma: f64) -> f64 {
    (gamma / GAS.r).sqrt() * m
        / (1. + 0.5 * (gamma - 1.) * m * m).powf((gamma + 1.) / (2. * (gamma - 1.)))
}
pub fn demand_speed(power: f64, mach: f64) -> f64 {
    let idle = DYN.rpm_idle / DYN.rpm_max;
    (idle + (1. - idle) * (power / GAS_BURNER_START).clamp(0., 1.))
        .max(idle + 0.38 * smooth(1.8, 3., mach))
}
pub const GAS_BURNER_START: f64 = 0.88;
pub fn smooth(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}
#[derive(Clone, Copy)]
pub struct Geometry {
    pub recovery: f64,
    pub bleed: f64,
    pub throat_m2: f64,
    pub flow_scale: f64,
    pub health: f64,
}
fn area_ratio(m: f64) -> f64 {
    mass_flux(1., GAS.gamma_hot) / mass_flux(m, GAS.gamma_hot).max(1e-12)
}
/// Component flow before fuel addition; mass bypasses the rear compressor, not the complete engine.
pub fn evaluate(a: Ambient, n: f64, g: Geometry, main_fuel: f64, ab_fuel: f64) -> Output {
    let initial = evaluate_raw(a, n, g, main_fuel, ab_fuel);
    if g.throat_m2 <= 0.
        || initial.nozzle_capacity_ratio >= 0.999
        || initial.nozzle.mass_flow_kg_s < 0.01
    {
        return initial;
    }
    let mut lo = 0.001;
    let mut hi = g.flow_scale;
    let mut best = initial;
    for _ in 0..9 {
        let scale = (lo + hi) * 0.5;
        let trial = evaluate_raw(
            a,
            n,
            Geometry {
                flow_scale: scale,
                ..g
            },
            main_fuel,
            ab_fuel,
        );
        if trial.nozzle_capacity_ratio < 1. {
            hi = scale;
        } else {
            lo = scale;
        }
        best = trial;
    }
    best
}
fn evaluate_raw(a: Ambient, n: f64, g: Geometry, main_fuel: f64, ab_fuel: f64) -> Output {
    if !a.valid() || a.pressure <= 0. || a.density <= 0. || !n.is_finite() {
        return Output::default();
    }
    let total_t = a.temperature * (1. + 0.5 * (GAS.gamma_air - 1.) * a.mach * a.mach);
    let total_p = a.pressure
        * (total_t / a.temperature).powf(GAS.gamma_air / (GAS.gamma_air - 1.))
        * g.recovery.clamp(0.02, 1.);
    let nc = (n / (total_t / GAS.reference_t).sqrt()).clamp(0., 1.1);
    let corrected = GAS.corrected_flow_kg_s
        * interpolate(&FLOW_SPEED_AXIS, &CORRECTED_FLOW, nc)
        * g.health.clamp(0., 1.)
        * g.flow_scale.clamp(0.001, 1.1);
    let demand = corrected * (total_p / GAS.reference_p) / (total_t / GAS.reference_t).sqrt();
    let capture = if a.mach > 0.8 {
        a.density * a.axial_speed.max(0.) * INLET.capture_area_m2 * 1.03
    } else {
        f64::INFINITY
    };
    let flow = demand.min(capture).max(0.);
    let pr = (1.
        + (GAS
            .max_compressor_pr
            .powf((GAS.gamma_air - 1.) / GAS.gamma_air)
            - 1.)
            * nc.powf(GAS.compressor_pr_exponent)
            * g.health.clamp(0., 1.))
    .powf(GAS.gamma_air / (GAS.gamma_air - 1.));
    let efficiency = (GAS.compressor_efficiency - 0.15 * (1. - g.health)).clamp(0.55, 0.90);
    let front_pr = pr.powf(GAS.front_compressor_log_fraction);
    let t25 =
        total_t * (1. + (front_pr.powf((GAS.gamma_air - 1.) / GAS.gamma_air) - 1.) / efficiency);
    let t3 =
        t25 * (1. + ((pr / front_pr).powf((GAS.gamma_air - 1.) / GAS.gamma_air) - 1.) / efficiency);
    let bleed = flow * g.bleed.clamp(0., 0.35);
    let core = (flow - bleed).max(0.);
    let fuel = main_fuel.max(0.);
    let t4 = if core > 1e-6 {
        (core * GAS.cp_air * t3 + fuel * GAS.lhv_j_kg * GAS.burner_efficiency)
            / ((core + fuel) * GAS.cp_hot)
    } else {
        t3
    };
    // Duct losses scale with corrected flow squared; applying full-load losses
    // at cranking speed creates an artificial pressure barrier to starting.
    let loss_fraction = interpolate(&FLOW_SPEED_AXIS, &CORRECTED_FLOW, nc)
        .powi(2)
        .clamp(0., 1.);
    let p4 = total_p * pr * (1. - (1. - GAS.burner_pressure_ratio) * loss_fraction);
    let turbine_pr = (GAS.turbine_pr_base + GAS.turbine_pr_gain * nc)
        .min(
            (p4 / (a.pressure * (1. + (GAS.minimum_nozzle_pr - 1.) * (0.1 + 0.9 * loss_fraction))))
                .max(1.),
        )
        .max(1.);
    let t5 = t4
        * (1.
            - GAS.turbine_efficiency
                * (1. - turbine_pr.powf(-(GAS.gamma_hot - 1.) / GAS.gamma_hot)));
    let p5 = p4 / turbine_pr;
    let pcomp = flow * GAS.cp_air * (t25 - total_t) + core * GAS.cp_air * (t3 - t25);
    let pturb = (core + fuel) * GAS.cp_hot * (t4 - t5);
    let mixed_flow = flow + fuel;
    let t6 = if mixed_flow > 1e-6 {
        ((core + fuel) * GAS.cp_hot * t5 + bleed * GAS.cp_air * t25) / (mixed_flow * GAS.cp_hot)
    } else {
        t5
    };
    let afuel = ab_fuel.max(0.);
    let primary = mixed_flow + afuel;
    let t7 = if primary > 1e-6 {
        (mixed_flow * GAS.cp_hot * t6 + afuel * GAS.lhv_j_kg * GAS.ab_efficiency)
            / (primary * GAS.cp_hot)
    } else {
        t6
    };
    let p7 = p5
        * (1.
            - (1. - GAS.ab_pressure_ratio)
                * (afuel / (mixed_flow * 0.025).max(1e-6)).clamp(0., 1.));
    let secondary = flow * GAS.ejector_fraction * (0.5 + 0.5 * smooth(0., 2., a.mach));
    let outflow = primary + secondary;
    let te = if outflow > 1e-6 {
        (primary * GAS.cp_hot * t7 + secondary * GAS.cp_air * total_t) / (outflow * GAS.cp_hot)
    } else {
        t7
    };
    let pe_total = p7
        * (1. - (1. - GAS.mixer_pressure_ratio) * loss_fraction)
        * (1. - (1. - GAS.nozzle_pressure_recovery) * loss_fraction);
    let critical = ((GAS.gamma_hot + 1.) / 2.).powf(GAS.gamma_hot / (GAS.gamma_hot - 1.));
    let npr = pe_total / a.pressure;
    let choked = npr >= critical;
    let ideal_m = if npr > 1. {
        (2. / (GAS.gamma_hot - 1.) * (npr.powf((GAS.gamma_hot - 1.) / GAS.gamma_hot) - 1.)).sqrt()
    } else {
        0.
    };
    let throat_m = if choked { 1. } else { ideal_m };
    let flux = mass_flux(throat_m, GAS.gamma_hot) * pe_total / te.max(100.).sqrt();
    let required = if flux > 1e-9 {
        outflow / flux
    } else {
        DYN.nozzle_max_m2
    };
    let throat = if g.throat_m2 > 0. {
        g.throat_m2
    } else {
        required.clamp(DYN.nozzle_min_m2, DYN.nozzle_max_m2)
    };
    let capacity = (throat * flux / outflow.max(1e-6)).clamp(0., 2.);
    let effective_throat = throat.min(required);
    let exit_area = if choked {
        (effective_throat * area_ratio(ideal_m.max(1.))).min(DYN.nozzle_exit_max_m2)
    } else {
        effective_throat
    };
    let exit_m = if choked {
        let ar = (exit_area / effective_throat.max(1e-9)).max(1.);
        let mut lo = 1.;
        let mut hi = 5.;
        for _ in 0..16 {
            let mid = (lo + hi) * 0.5;
            if area_ratio(mid) < ar {
                lo = mid
            } else {
                hi = mid
            }
        }
        (lo + hi) * 0.5
    } else {
        ideal_m
    };
    let exit_p = if exit_m > 0. {
        pe_total
            / (1. + 0.5 * (GAS.gamma_hot - 1.) * exit_m * exit_m)
                .powf(GAS.gamma_hot / (GAS.gamma_hot - 1.))
    } else {
        a.pressure
    };
    let exit_t = te / (1. + 0.5 * (GAS.gamma_hot - 1.) * exit_m * exit_m);
    let ve = exit_m * (GAS.gamma_hot * GAS.r * exit_t).sqrt();
    let gross = outflow * ve;
    let pressure = (exit_p - a.pressure) * exit_area;
    let ram = (flow + secondary) * a.axial_speed;
    let accessory = DYN.accessory_w * n * n + DYN.friction_w * n * n * n;
    Output {
        inlet: Station {
            total_pressure_pa: total_p,
            total_temperature_k: total_t,
            mass_flow_kg_s: flow,
        },
        front_compressor: Station {
            total_pressure_pa: total_p * front_pr,
            total_temperature_k: t25,
            mass_flow_kg_s: flow,
        },
        compressor: Station {
            total_pressure_pa: total_p * pr,
            total_temperature_k: t3,
            mass_flow_kg_s: core,
        },
        combustor: Station {
            total_pressure_pa: p4,
            total_temperature_k: t4,
            mass_flow_kg_s: core + fuel,
        },
        turbine: Station {
            total_pressure_pa: p5,
            total_temperature_k: t5,
            mass_flow_kg_s: core + fuel,
        },
        afterburner: Station {
            total_pressure_pa: p7,
            total_temperature_k: t7,
            mass_flow_kg_s: primary,
        },
        nozzle: Station {
            total_pressure_pa: pe_total,
            total_temperature_k: te,
            mass_flow_kg_s: outflow,
        },
        compressor_ratio: pr,
        turbine_ratio: turbine_pr,
        corrected_speed: nc,
        corrected_flow_kg_s: corrected,
        main_fuel_kg_s: fuel,
        ab_fuel_kg_s: afuel,
        bleed_air_kg_s: bleed,
        secondary_air_kg_s: secondary,
        compressor_power_w: pcomp,
        turbine_power_w: pturb,
        accessory_power_w: accessory,
        shaft_power_w: pturb - pcomp - accessory,
        gross_momentum_n: gross,
        pressure_thrust_n: pressure,
        ram_drag_n: ram,
        net_thrust_n: gross + pressure - ram,
        nozzle_matching_residual_kg_s: (capacity - 1.) * outflow,
        nozzle_capacity_ratio: capacity,
        required_throat_m2: required,
        exit_area_m2: exit_area,
        exit_pressure_pa: exit_p,
        exit_velocity_m_s: ve,
        exit_mach: exit_m,
        choked,
        air_mass_residual_kg_s: outflow - (flow + fuel + afuel + secondary),
        combustor_energy_residual_w: (core + fuel) * GAS.cp_hot * t4
            - core * GAS.cp_air * t3
            - fuel * GAS.lhv_j_kg * GAS.burner_efficiency,
        turbine_energy_residual_w: pturb - (core + fuel) * GAS.cp_hot * (t4 - t5),
        gas_energy_residual_w: outflow * GAS.cp_hot * te
            - ((flow + secondary) * GAS.cp_air * total_t
                + fuel * GAS.lhv_j_kg * GAS.burner_efficiency
                + afuel * GAS.lhv_j_kg * GAS.ab_efficiency
                + pcomp
                - pturb),
    }
}

pub fn fuel_for_t4(o: &Output, t: f64) -> f64 {
    let core = o.compressor.mass_flow_kg_s;
    core * (GAS.cp_hot * t - GAS.cp_air * o.compressor.total_temperature_k).max(0.)
        / (GAS.lhv_j_kg * GAS.burner_efficiency - GAS.cp_hot * t).max(1e6)
}
pub fn balanced_t4(o: &Output) -> f64 {
    let extraction =
        GAS.turbine_efficiency * (1. - o.turbine_ratio.powf(-(GAS.gamma_hot - 1.) / GAS.gamma_hot));
    if extraction < 1e-5 {
        return 1000.;
    }
    {
        let work = o.compressor_power_w + o.accessory_power_w;
        let q = GAS.lhv_j_kg * GAS.burner_efficiency;
        work * q
            / (GAS.cp_hot
                * (work
                    + o.compressor.mass_flow_kg_s
                        * (q - GAS.cp_air * o.compressor.total_temperature_k)
                        * extraction))
                .max(1.)
    }
}
pub fn ab_fuel(o: &Output, fraction: f64) -> f64 {
    let mass = o.afterburner.mass_flow_kg_s - o.ab_fuel_kg_s;
    let before = if mass > 1e-6 {
        (o.turbine.mass_flow_kg_s * o.turbine.total_temperature_k
            + o.bleed_air_kg_s * (GAS.cp_air / GAS.cp_hot) * o.front_compressor.total_temperature_k)
            / mass
    } else {
        0.
    };
    let target = before + (GAS.max_t7_k - before).max(0.) * fraction.clamp(0., 1.);
    mass * GAS.cp_hot * (target - before)
        / (GAS.lhv_j_kg * GAS.ab_efficiency - GAS.cp_hot * target).max(1e6)
}
