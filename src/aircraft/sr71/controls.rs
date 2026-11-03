use super::{config::*, math::*};
use crate::{
    aerodynamics::Coefficients,
    control_surfaces::{Loads, SurfaceLoad},
    physics::{AircraftDamage, Flight},
};
use bevy::math::DVec3;
pub fn targets(command: DVec3) -> [f64; 4] {
    let a = (-command.x + command.z).clamp(-1., 1.);
    let b = (-command.x - command.z).clamp(-1., 1.);
    let convert = |x: f64, inner: bool| {
        if x < 0. {
            x * CONTROL.elevon_up_rad
        } else {
            x * if inner {
                CONTROL.inner_down_rad
            } else {
                CONTROL.outer_down_rad
            }
        }
    };
    [
        convert(a, true),
        convert(b, true),
        (convert(a, false) - CONTROL.outer_rig_rad).max(-CONTROL.elevon_up_rad),
        (convert(b, false) - CONTROL.outer_rig_rad).max(-CONTROL.elevon_up_rad),
    ]
}
pub fn control_coefficients(
    elevons: [f64; 4],
    fins: [f64; 2],
    mach: f64,
    attached: f64,
) -> Coefficients {
    let left = CONTROL.inner_weight * elevons[0] + (1. - CONTROL.inner_weight) * elevons[2];
    let right = CONTROL.inner_weight * elevons[1] + (1. - CONTROL.inner_weight) * elevons[3];
    let e = (left + right) * 0.5 + (1. - CONTROL.inner_weight) * CONTROL.outer_rig_rad;
    let a = (left - right) * 0.5;
    let r = (fins[0] + fins[1]) * 0.5;
    let k = AERO.control_floor + AERO.control_attached_gain * attached;
    let cn = tab(&CNDE, mach) * DEG * e * k;
    Coefficients {
        lift: cn,
        side: tab(&CYDR, mach) * DEG * r * k,
        pitch: tab(&CMDE, mach) * DEG * e * k,
        yaw: -(tab(&CNDR, mach) * DEG * r + tab(&CNDA, mach) * DEG * a) * k,
        roll: -tab(&CLDA, mach) * DEG * a * k,
        drag: CONTROL.pitch_drag * e * e + CONTROL.roll_drag * a * a + CONTROL.yaw_drag * r * r,
        attached,
        ..Default::default()
    }
}
pub fn control_loads(
    s: &crate::aircraft::sr71::State,
    damage: &AircraftDamage,
    attached: f64,
    b: DVec3,
    rho: f64,
    mach: f64,
    command: Option<DVec3>,
) -> Loads {
    if s.airframe.initialized {
        let angles = command.map(targets).unwrap_or(s.elevons);
        let fins = command
            .map(|c| [c.y * CONTROL.rudder_limit_rad; 2])
            .unwrap_or(s.fins);
        return super::plant::control_loads(&s.airframe, damage, angles, fins);
    }
    if !b.is_finite()
        || !rho.is_finite()
        || rho < 0.
        || !mach.is_finite()
        || command.is_some_and(|c| !c.is_finite())
    {
        return Loads::default();
    }
    let mut angles = command.map(targets).unwrap_or(s.elevons);
    let neutral = targets(DVec3::ZERO);
    let mut fins = command
        .map(|c| [c.y * CONTROL.rudder_limit_rad; 2])
        .unwrap_or(s.fins);
    for (i, slot) in [0, 4, 2, 6].into_iter().enumerate() {
        let health = if damage.surface_state[slot] == crate::combat::damage::SurfaceState::Detached
        {
            0.
        } else {
            damage.surfaces[slot].clamp(0., 1.)
        };
        angles[i] = neutral[i]
            + (crate::control_surfaces::effective_angle(damage, slot, angles[i]).clamp(
                -CONTROL.elevon_up_rad,
                if i < 2 {
                    CONTROL.inner_down_rad
                } else {
                    CONTROL.outer_down_rad
                },
            ) - neutral[i])
                * health;
    }
    for (i, slot) in [3, 7].into_iter().enumerate() {
        let health = if damage.surface_state[slot] == crate::combat::damage::SurfaceState::Detached
        {
            0.
        } else {
            damage.surfaces[slot].clamp(0., 1.)
        };
        fins[i] = crate::control_surfaces::effective_angle(damage, slot, fins[i])
            .clamp(-CONTROL.rudder_limit_rad, CONTROL.rudder_limit_rad)
            * health;
    }
    let c = control_coefficients(angles, fins, mach, attached);
    let qs = 0.5 * rho * b.length_squared() * AREA;
    let force = crate::aerodynamics::force(DVec3::X, b, qs, c);
    let moment = DVec3::new(c.pitch * CHORD, c.yaw * SPAN, c.roll * SPAN) * qs;
    let mut l = Loads {
        force,
        moment,
        ..Default::default()
    };
    if command.is_some() {
        return l;
    } // Jacobian probes need only totals, never per-surface diagnostics.
    // Linear force/moment contributions per surface. Allocate only the nonlinear
    // combined deflection-drag residual equally; totals must close exactly.
    let mut sum_force = DVec3::ZERO;
    for (i, slot) in [0, 4, 2, 6, 3, 7].into_iter().enumerate() {
        let mut e = neutral;
        let mut rudders = [0.; 2];
        if i < 4 {
            e[i] = angles[i];
        } else {
            rudders[i - 4] = fins[i - 4];
        }
        let part = control_coefficients(e, rudders, mach, attached);
        let pf = crate::aerodynamics::force(DVec3::X, b, qs, part);
        l.surfaces[slot] = SurfaceLoad {
            deflection: if i < 4 { angles[i] } else { fins[i - 4] },
            attached,
            force: pf,
            moment: DVec3::new(part.pitch * CHORD, part.yaw * SPAN, part.roll * SPAN) * qs,
        };
        sum_force += pf;
    }
    for slot in [0, 4, 2, 6, 3, 7] {
        l.surfaces[slot].force += (force - sum_force) / 6.;
    }
    l
}
pub fn target_rates(f: &Flight, c: DVec3) -> DVec3 {
    let m = f.mach;
    let bank = f
        .roll()
        .clamp(-REGIMES.bank_limit_rad, REGIMES.bank_limit_rad);
    let coordinated = 9.80665 * bank.tan() / f.airspeed.max(REGIMES.coordination_speed_floor_m_s);
    let alpha_limit = CONTROL.alpha_limit_rad
        - CONTROL.alpha_limit_reduction
            * smooth(REGIMES.authority_mach[0], REGIMES.authority_mach[1], m);
    let mut pitch = c.x
        * (CONTROL.low_mach_pitch_rate
            - CONTROL.pitch_rate_reduction
                * smooth(REGIMES.authority_mach[0], REGIMES.authority_mach[1], m));
    if c.x.abs() < CONTROL.neutral_deadband {
        let speed_tangent = (f.velocity - f.up() * f.vertical_speed()).length();
        let gravity = crate::physics::planet_mu() / (crate::physics::planet_radius() + f.altitude()).powi(2);
        let support =
            (gravity - speed_tangent.powi(2) / (crate::physics::planet_radius() + f.altitude())) / 9.80665;
        let target_load = support * f.alpha.cos() / bank.cos().max(REGIMES.bank_cos_floor);
        pitch += ((f.g_load - target_load) * -CONTROL.load_trim_gain)
            .clamp(-CONTROL.load_trim_rate_limit, CONTROL.load_trim_rate_limit);
    }
    pitch = pitch.min((alpha_limit - f.alpha) * CONTROL.alpha_limit_gain);
    DVec3::new(
        pitch + coordinated * bank.sin(),
        c.y * CONTROL.yaw_rate - f.beta * CONTROL.sideslip_gain - coordinated * bank.cos(),
        -c.z * (CONTROL.low_mach_roll_rate
            - CONTROL.roll_rate_reduction
                * smooth(REGIMES.authority_mach[0], REGIMES.authority_mach[1], m)),
    )
}

pub fn tracking_acceleration(rates: DVec3, target: DVec3) -> DVec3 {
    let limit = DVec3::from_array(CONTROL.acceleration_limits);
    ((target - rates) * CONTROL.rate_gain).clamp(-limit, limit)
}
