//! Incremental loads from individual surfaces. Neutral-wing lift and stability remain
//! in aerodynamics.rs; deflection loads are evaluated here exactly once.
use bevy::math::{DMat3, DVec3};
#[derive(Clone, Copy, Debug)]
pub enum SurfaceKind {
    Canard,
    Elevon { side: f32, inner: bool },
    Rudder,
    Airbrake,
}
pub fn index(kind: SurfaceKind) -> usize {
    match kind {
        SurfaceKind::Canard => 0,
        SurfaceKind::Elevon { side, inner } => {
            if side < 0.0 {
                if inner { 1 } else { 2 }
            } else {
                if inner { 5 } else { 6 }
            }
        }
        SurfaceKind::Rudder => 8,
        SurfaceKind::Airbrake => 3,
    }
}
pub fn angles(actuator: DVec3, flaps: f64, airbrake: bool) -> [f64; crate::aircraft::SURFACES] {
    std::array::from_fn(|i| if i<9 {deflection(PANELS[i].0, actuator, flaps, airbrake)}else{0.})
}
pub fn effective_angle(d: &crate::physics::AircraftDamage, i: usize, angle: f64) -> f64 {
    match d.surface_state[i] {
        crate::combat::damage::SurfaceState::Jammed(a) => a,
        crate::combat::damage::SurfaceState::Detached => 0.0,
        _ => angle,
    }
}
pub fn deflection(kind: SurfaceKind, actuator: DVec3, flaps: f64, airbrake: bool) -> f64 {
    match kind {
        SurfaceKind::Canard => (-actuator.x * 22.0).to_radians(),
        SurfaceKind::Elevon { side, inner } => (actuator.x * 24.0
            - side as f64 * actuator.z * 20.0
            - flaps * if inner { 22.0 } else { 10.0 })
        .clamp(-38.0, 38.0)
        .to_radians(),
        SurfaceKind::Rudder => (-actuator.y * 28.0).to_radians(),
        SurfaceKind::Airbrake => {
            if airbrake {
                -48.0_f64.to_radians()
            } else {
                0.0
            }
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct SurfaceLoad {
    pub force: DVec3,
    pub moment: DVec3,
    pub deflection: f64,
    pub attached: f64,
}
#[derive(Debug, Default)]
pub struct Loads {
    pub surfaces: [SurfaceLoad; crate::aircraft::SURFACES],
    pub force: DVec3,
    pub moment: DVec3,
}
// Fixed geometry: evaluating a control derivative must never allocate on the
// 120 Hz physics path. Positions are aerodynamic centers in body coordinates.
type Panel = (SurfaceKind, DVec3, f64, DVec3, f64, f64);
const PANELS: [Panel; 9] = [
    (
        SurfaceKind::Canard,
        DVec3::new(-1.35, 0.32, -2.65),
        1.5,
        DVec3::Y,
        3.3,
        1.0,
    ),
    (
        SurfaceKind::Elevon {
            side: -1.0,
            inner: true,
        },
        DVec3::new(-1.58, -0.16, 2.75),
        1.8,
        DVec3::Y,
        3.3,
        0.8,
    ),
    (
        SurfaceKind::Elevon {
            side: -1.0,
            inner: false,
        },
        DVec3::new(-3.1, -0.19, 2.97),
        1.5,
        DVec3::Y,
        3.3,
        0.8,
    ),
    (
        SurfaceKind::Airbrake,
        DVec3::new(-0.60, 0.40, 4.0),
        0.8,
        DVec3::Y,
        0.0,
        0.0,
    ),
    (
        SurfaceKind::Canard,
        DVec3::new(1.35, 0.32, -2.65),
        1.5,
        DVec3::Y,
        3.3,
        1.0,
    ),
    (
        SurfaceKind::Elevon {
            side: 1.0,
            inner: true,
        },
        DVec3::new(1.58, -0.16, 2.75),
        1.8,
        DVec3::Y,
        3.3,
        0.8,
    ),
    (
        SurfaceKind::Elevon {
            side: 1.0,
            inner: false,
        },
        DVec3::new(3.1, -0.19, 2.97),
        1.5,
        DVec3::Y,
        3.3,
        0.8,
    ),
    (
        SurfaceKind::Airbrake,
        DVec3::new(0.60, 0.40, 4.0),
        0.8,
        DVec3::Y,
        0.0,
        0.0,
    ),
    (
        SurfaceKind::Rudder,
        DVec3::new(0.0, 1.7, 5.2),
        1.45,
        DVec3::X,
        3.5,
        0.85,
    ),
];
#[cfg(test)]
pub fn evaluate(
    air: DVec3,
    rates: DVec3,
    density: f64,
    effectiveness: f64,
    actuator: DVec3,
    flaps: f64,
    airbrake: bool,
) -> Loads {
    evaluate_flight(
        air,
        rates,
        density,
        effectiveness,
        actuator,
        flaps,
        airbrake,
        0.0,
    )
}

pub fn evaluate_flight(
    air: DVec3,
    rates: DVec3,
    density: f64,
    effectiveness: f64,
    actuator: DVec3,
    flaps: f64,
    airbrake: bool,
    mach: f64,
) -> Loads {
    evaluate_damage(
        air,
        rates,
        density,
        effectiveness,
        actuator,
        flaps,
        airbrake,
        mach,
        &crate::physics::AircraftDamage::default(),
    )
}
pub fn evaluate_damage(
    air: DVec3,
    rates: DVec3,
    density: f64,
    effectiveness: f64,
    actuator: DVec3,
    flaps: f64,
    airbrake: bool,
    mach: f64,
    damage: &crate::physics::AircraftDamage,
) -> Loads {
    PreparedPanels::new(air, rates, density, effectiveness, mach, damage)
        .evaluate(actuator, flaps, airbrake)
}

#[derive(Clone, Copy)]
struct PanelFlow {
    flow: DVec3,
    pressure: f64,
    incidence: f64,
    lift_axis: DVec3,
    neutral_lift: f64,
}
/// Flow is constant while FBW probes the six command derivatives in one step.
/// Reuse only those invariants: each command still evaluates its own nonlinear
/// deflection polar, jam state, forces and moments in the original order.
pub struct PreparedPanels<'a> {
    flow: [Option<PanelFlow>; 9],
    effectiveness: f64,
    compressibility: f64,
    critical: f64,
    damage: &'a crate::physics::AircraftDamage,
}
impl<'a> PreparedPanels<'a> {
    pub fn new(
        air: DVec3,
        rates: DVec3,
        density: f64,
        effectiveness: f64,
        mach: f64,
        damage: &'a crate::physics::AircraftDamage,
    ) -> Self {
        let mut model = Self {
            flow: [None; 9],
            effectiveness,
            compressibility: 1.0 / (1.0 + 0.18 * mach.max(0.0).powi(2)),
            critical: (0.43 - 0.06 * mach.min(1.5)).max(0.30),
            damage,
        };
        for (i, (_, position, _, normal, slope, _)) in PANELS.into_iter().enumerate() {
            let velocity = air + rates.cross(position);
            let speed = velocity.length();
            if speed < 0.01
                || density <= 0.0
                || damage.surface_state[i] == crate::combat::damage::SurfaceState::Detached
            {
                continue;
            }
            let flow = velocity / speed;
            let incidence = (-velocity.dot(normal)).atan2(-velocity.z);
            model.flow[i] = Some(PanelFlow {
                flow,
                pressure: 0.5 * density * speed * speed,
                incidence,
                lift_axis: (normal - flow * normal.dot(flow)).normalize_or_zero(),
                neutral_lift: model.polar(incidence, slope).0,
            });
        }
        model
    }
    fn attached(&self, a: f64) -> f64 {
        let t = ((a.sin().abs().asin() - self.critical) / 0.24).clamp(0.0, 1.0);
        1.0 - t * t * (3.0 - 2.0 * t)
    }
    fn polar(&self, a: f64, slope: f64) -> (f64, f64) {
        let blend = self.attached(a);
        (
            (blend * slope * 0.5 + (1.0 - blend) * 0.95) * (2.0 * a).sin() * self.compressibility,
            blend,
        )
    }
    pub fn evaluate(&self, actuator: DVec3, flaps: f64, airbrake: bool) -> Loads {
        let mut output = Loads::default();
        for (i, (kind, position, area, _, slope, hinge_efficiency)) in
            PANELS.into_iter().enumerate()
        {
            let Some(flow) = self.flow[i] else { continue };
            let angle =
                effective_angle(self.damage, i, deflection(kind, actuator, flaps, airbrake));
            let incidence = flow.incidence - angle * hinge_efficiency;
            let (lift, attached) = self.polar(incidence, slope);
            let delta_cl = (lift - flow.neutral_lift) * self.effectiveness;
            let drag_coefficient = if matches!(kind, SurfaceKind::Airbrake) {
                4.4 * angle.sin().powi(2)
            } else {
                0.12 * angle.sin().powi(2) + 0.08 * delta_cl.powi(2)
            };
            let force =
                (flow.lift_axis * delta_cl - flow.flow * drag_coefficient) * flow.pressure * area;
            output.surfaces[i] = SurfaceLoad {
                force,
                moment: position.cross(force),
                deflection: angle,
                attached,
            };
        }
        for surface in &output.surfaces {
            output.force += surface.force;
            output.moment += surface.moment;
        }
        output
    }
}

#[cfg(test)]
fn evaluate_damage_reference(
    air: DVec3,
    rates: DVec3,
    density: f64,
    effectiveness: f64,
    actuator: DVec3,
    flaps: f64,
    airbrake: bool,
    mach: f64,
    damage: &crate::physics::AircraftDamage,
) -> Loads {
    let mut output = Loads::default();
    for (i, (kind, position, area, normal, slope, hinge_efficiency)) in
        PANELS.into_iter().enumerate()
    {
        let velocity = air + rates.cross(position);
        let speed = velocity.length();
        if speed < 0.01 || density <= 0.0 {
            continue;
        }
        let flow = velocity / speed;
        let q = 0.5 * density * speed * speed;
        let angle = effective_angle(damage, i, deflection(kind, actuator, flaps, airbrake));
        if damage.surface_state[i] == crate::combat::damage::SurfaceState::Detached {
            continue;
        }
        let incidence = (-velocity.dot(normal)).atan2(-velocity.z);
        // Saturated finite-wing lift curve: the neutral load is already in the
        // whole-airframe model. Only the deflection increment is added here.
        let compressibility = 1.0 / (1.0 + 0.18 * mach.max(0.0).powi(2));
        // Each deflected panel sees its own incidence. Beyond the critical angle,
        // smoothly transition from finite-wing lift to a separated flat-plate curve.
        // This is a bounded game polar, not a calibrated Gripen section database.
        let critical = (0.43 - 0.06 * mach.min(1.5)).max(0.30);
        let attached = |a: f64| {
            let t = ((a.sin().abs().asin() - critical) / 0.24).clamp(0.0, 1.0);
            1.0 - t * t * (3.0 - 2.0 * t)
        };
        let lift_curve = |a: f64| {
            let blend = attached(a);
            (blend * slope * 0.5 + (1.0 - blend) * 0.95) * (2.0 * a).sin() * compressibility
        };
        let delta_cl = (lift_curve(incidence - angle * hinge_efficiency) - lift_curve(incidence))
            * effectiveness;
        let lift_axis = (normal - flow * normal.dot(flow)).normalize_or_zero();
        let drag_coefficient = if matches!(kind, SurfaceKind::Airbrake) {
            4.4 * angle.sin().powi(2)
        } else {
            0.12 * angle.sin().powi(2) + 0.08 * delta_cl.powi(2)
        };
        let force = (lift_axis * delta_cl - flow * drag_coefficient) * q * area;
        let moment = position.cross(force);
        output.surfaces[i] = SurfaceLoad {
            force,
            moment,
            deflection: angle,
            attached: attached(incidence - angle * hinge_efficiency),
        };
    }
    for surface in &output.surfaces {
        output.force += surface.force;
        output.moment += surface.moment;
    }
    output
}

/// Servos combine first-order response with a hard rate limit. Aerodynamic
pub fn damaged(mut loads: Loads, health: &[f64; crate::aircraft::SURFACES]) -> Loads {
    loads.force = DVec3::ZERO;
    loads.moment = DVec3::ZERO;
    for (surface, health) in loads.surfaces.iter_mut().zip(health) {
        surface.force *= health.clamp(0.0, 1.0);
        surface.moment *= health.clamp(0.0, 1.0);
        loads.force += surface.force;
        loads.moment += surface.moment;
    }
    loads
}
/// Servos combine first-order response with a hard rate limit. Aerodynamic
/// loading slows the actuator above 25 kPa without bypassing the deflection limit.
pub fn advance_actuator(current: DVec3, request: DVec3, pressure: f64, dt: f64) -> DVec3 {
    let authority = (1.0 / (1.0 + (pressure.max(0.0) / 25000.0).powi(2))).sqrt();
    let max_delta = dt.max(0.0) * 4.5 * authority;
    let delta = (request - current) * (1.0 - (-dt.max(0.0) / 0.09).exp());
    (current + delta.clamp(DVec3::splat(-max_delta), DVec3::splat(max_delta)))
        .clamp(DVec3::splat(-1.0), DVec3::splat(1.0))
}

/// Solve the three coupled moment channels. At near-zero dynamic pressure or
/// singular configurations, damped least squares yields a bounded request.
pub fn allocate(jacobian: DMat3, moment: DVec3) -> DVec3 {
    let scale = jacobian
        .x_axis
        .length()
        .max(jacobian.y_axis.length())
        .max(jacobian.z_axis.length())
        .max(1.0);
    let matrix = jacobian / scale;
    let demand = moment / scale;
    let transpose = matrix.transpose();
    let regularized = transpose * matrix + DMat3::IDENTITY * 0.0004;
    let solution = regularized.inverse() * transpose * demand;
    if solution.is_finite() {
        solution.clamp(DVec3::splat(-1.0), DVec3::splat(1.0))
    } else {
        DVec3::ZERO
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn prepared_panel_loads_match_original_equations_bit_for_bit() {
        use crate::combat::damage::SurfaceState;
        let mut seed = 0x5471bafe_u64;
        let mut number = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            (seed >> 11) as f64 / (1_u64 << 53) as f64
        };
        for case in 0..400 {
            let air = if case % 19 == 0 {
                DVec3::ZERO
            } else {
                DVec3::new(
                    (number() - 0.5) * 160.,
                    (number() - 0.5) * 300.,
                    -number() * 650.,
                )
            };
            let rates = DVec3::new(number() - 0.5, number() - 0.5, number() - 0.5) * 4.;
            let density = if case % 13 == 0 { 0. } else { number() * 1.4 };
            let mach = number() * 3.;
            let scale = number();
            let flaps = number();
            let brake = case % 2 == 0;
            let mut damage = crate::physics::AircraftDamage::default();
            for i in 0..9 {
                damage.surface_state[i] = match (case + i) % 7 {
                    0 => SurfaceState::Detached,
                    1 => SurfaceState::Jammed((number() - 0.5) * 0.7),
                    _ => SurfaceState::Operational,
                };
            }
            let model = PreparedPanels::new(air, rates, density, scale, mach, &damage);
            for actuator in [
                DVec3::ZERO,
                DVec3::X * 0.02,
                -DVec3::X * 0.02,
                DVec3::Y * 0.02,
                -DVec3::Y * 0.02,
                DVec3::Z * 0.02,
                -DVec3::Z * 0.02,
                DVec3::new(0.65, -0.8, 1.0),
            ] {
                let actual = model.evaluate(actuator, flaps, brake);
                let expected = evaluate_damage_reference(
                    air, rates, density, scale, actuator, flaps, brake, mach, &damage,
                );
                let equal = |a: DVec3, b: DVec3| {
                    assert_eq!(
                        a.to_array().map(f64::to_bits),
                        b.to_array().map(f64::to_bits)
                    )
                };
                equal(actual.force, expected.force);
                equal(actual.moment, expected.moment);
                for (a, b) in actual.surfaces.iter().zip(expected.surfaces) {
                    equal(a.force, b.force);
                    equal(a.moment, b.moment);
                    assert_eq!(a.deflection.to_bits(), b.deflection.to_bits());
                    assert_eq!(a.attached.to_bits(), b.attached.to_bits());
                }
            }
        }
    }
    #[test]
    fn jammed_surface_has_fixed_load_and_no_command_derivative() {
        use crate::{combat::damage::SurfaceState, physics::AircraftDamage};
        let mut damage = AircraftDamage::default();
        damage.surface_state[2] = SurfaceState::Jammed(0.23);
        let load = |command| {
            super::evaluate_damage(
                -DVec3::Z * 200.0,
                DVec3::ZERO,
                1.0,
                1.0,
                command,
                0.0,
                false,
                0.6,
                &damage,
            )
        };
        let a = load(DVec3::ZERO);
        let b = load(DVec3::Z);
        assert!(a.surfaces[2].force.length() > 1.0);
        assert!(a.surfaces[2].force.distance(b.surfaces[2].force) < 1e-10);
        assert_eq!(a.surfaces[2].deflection, 0.23);
        damage.surface_state[2] = SurfaceState::Detached;
        let gone = super::evaluate_damage(
            -DVec3::Z * 200.0,
            DVec3::ZERO,
            1.0,
            1.0,
            DVec3::Z,
            0.0,
            false,
            0.6,
            &damage,
        );
        assert_eq!(gone.surfaces[2].force, DVec3::ZERO);
    }
    use super::*;
    #[test]
    fn coupled_allocation_cancels_rudder_roll() {
        let matrix = DMat3::from_cols(
            DVec3::new(10000.0, 0.0, 0.0),
            DVec3::new(0.0, 8000.0, -2500.0),
            DVec3::new(0.0, 0.0, 12000.0),
        );
        let command = allocate(matrix, DVec3::new(0.0, 2000.0, 0.0));
        let torque = matrix * command;
        assert!((torque.y - 2000.0).abs() < 5.0 && torque.z.abs() < 5.0);
        assert!(command.z > 0.0);
        assert_eq!(allocate(DMat3::ZERO, DVec3::ONE), DVec3::ZERO);
    }
    #[test]
    fn servo_rate_is_bounded_and_high_pressure_slows_motion() {
        let low = advance_actuator(DVec3::ZERO, DVec3::ONE, 0.0, 0.01);
        let high = advance_actuator(DVec3::ZERO, DVec3::ONE, 60000.0, 0.01);
        assert!(low.max_element() <= 0.045 + 1e-10);
        assert!(high.x < low.x && high.y > 0.0);
        assert_eq!(
            advance_actuator(DVec3::ZERO, DVec3::ONE, 0.0, 0.0),
            DVec3::ZERO
        );
    }
    #[test]
    fn surface_stall_is_local_and_loads_remain_finite_in_reverse_flow() {
        let air = DVec3::new(0.0, -100.0 * 0.5_f64.sin(), -100.0 * 0.5_f64.cos());
        let l = evaluate_flight(air, DVec3::ZERO, 1.225, 1.0, DVec3::X, 0.0, false, 0.4);
        assert!(l.surfaces[0].attached < l.surfaces[1].attached);
        assert!(l.surfaces[0].deflection < 0.0);
        for a in 0..360 {
            let angle = (a as f64).to_radians();
            let l = evaluate_flight(
                DVec3::new(15.0, -100.0 * angle.sin(), -100.0 * angle.cos()),
                DVec3::new(0.3, -0.2, 0.4),
                1.225,
                0.8,
                DVec3::ONE,
                0.5,
                true,
                1.7,
            );
            assert!(l.force.is_finite() && l.moment.is_finite());
        }
    }
    fn load(command: DVec3, flaps: f64, brake: bool) -> Loads {
        evaluate(
            DVec3::new(0.0, 0.0, -100.0),
            DVec3::ZERO,
            1.225,
            1.0,
            command,
            flaps,
            brake,
        )
    }
    #[test]
    fn pitch_surfaces_produce_opposing_lift_and_same_pitch_moment() {
        let l = load(DVec3::X, 0.0, false);
        assert!(l.surfaces[0].force.y > 0.0 && l.surfaces[1].force.y < 0.0);
        assert!(l.surfaces[0].moment.x > 0.0 && l.surfaces[1].moment.x > 0.0);
        assert!(l.moment.x > 0.0);
    }
    #[test]
    fn differential_elevons_roll_without_net_lift() {
        let l = load(DVec3::Z, 0.0, false);
        assert!(l.moment.z > 0.0);
        assert!(l.force.y.abs() < 1e-8);
        assert!(l.surfaces[1].force.y * l.surfaces[5].force.y < 0.0);
    }
    #[test]
    fn rudder_creates_side_force_and_yaw() {
        let l = load(DVec3::Y, 0.0, false);
        assert!(l.force.x > 0.0 && l.moment.y > 0.0 && l.surfaces[8].moment.y > 0.0);
    }
    #[test]
    fn flaps_add_lift_and_brakes_add_drag() {
        let neutral = load(DVec3::ZERO, 0.0, false);
        let flaps = load(DVec3::ZERO, 0.7, false);
        let brakes = load(DVec3::ZERO, 0.0, true);
        assert!(flaps.force.y > neutral.force.y && flaps.force.z > neutral.force.z);
        assert!(brakes.force.z > neutral.force.z);
    }
    #[test]
    fn forces_require_air_and_remove_energy() {
        let zero = evaluate(
            DVec3::new(0.0, 0.0, -100.0),
            DVec3::ZERO,
            0.0,
            1.0,
            DVec3::ONE,
            0.7,
            true,
        );
        assert_eq!(zero.force, DVec3::ZERO);
        for command in [DVec3::X, DVec3::Y, DVec3::Z, DVec3::ONE] {
            let l = load(command, 0.5, true);
            assert!(l.force.dot(DVec3::new(0.0, 0.0, -100.0)) <= 0.0);
            for s in l.surfaces {
                assert!(s.force.is_finite() && s.moment.is_finite());
            }
        }
    }
    #[test]
    fn failed_left_canard_changes_pitch_authority_and_adds_asymmetric_roll() {
        let intact = load(DVec3::X, 0.0, false);
        let mut health = [1.0; crate::aircraft::SURFACES];
        health[0] = 0.0;
        let failed = damaged(load(DVec3::X, 0.0, false), &health);
        assert!(failed.moment.x < intact.moment.x);
        assert!(failed.moment.z.abs() > 1.0);
        assert_eq!(failed.surfaces[0].force, DVec3::ZERO);
    }
}
