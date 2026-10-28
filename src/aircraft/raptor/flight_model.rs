//! Public-envelope-constrained F-22 game model. Tables are estimates, not an
//! F119 deck, production aerodynamic database or reconstructed flight software.
use super::*;
use crate::aerodynamics::{Coefficients, Conditions, FlowState};

#[derive(Clone)]
pub struct Dynamics {
    pub angles: [f64; SURFACES],
    pub actuators_initialized: bool,
    pub command: DVec3,
    pub nozzle_control: DVec3,
    pub skin_temperature: f64,
}
impl Default for Dynamics {
    fn default() -> Self {
        Self {
            angles: [0.; SURFACES],
            actuators_initialized: false,
            command: DVec3::ZERO,
            nozzle_control: DVec3::ZERO,
            skin_temperature: 288.15,
        }
    }
}
pub fn smooth(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}
fn bracket(axis: &[f64], x: f64) -> (usize, f64) {
    let x = x.clamp(axis[0], axis[axis.len() - 1]);
    let i = axis
        .windows(2)
        .position(|v| x <= v[1])
        .unwrap_or(axis.len() - 2);
    (i, (x - axis[i]) / (axis[i + 1] - axis[i]))
}
pub fn table(axis: &[f64], values: &[f64], x: f64) -> f64 {
    let (i, t) = bracket(axis, x);
    values[i] + (values[i + 1] - values[i]) * t
}

const MACH: [f64; 8] = [0., 0.6, 0.9, 1.2, 1.5, 2., 2.5, 3.];
// Equivalent ISA density-altitude rows. Interpolate in density, avoiding an
// inverse atmosphere solve in the per-substep engine/control path.
const DENSITY: [f64; 6] = [0., 0.0401, 0.0889, 0.1937, 0.4135, 1.225];
const DRY_FACTOR: [f64; 6] = [0., 0.12, 0.20, 0.31, 0.49, 1.];
const WET_FACTOR: [f64; 6] = [0., 0.12, 0.20, 0.32, 0.50, 1.];
const MIL_KN: [f64; 8] = [232., 215., 211., 225., 242., 245., 210., 150.];
const MAX_KN: [f64; 8] = [312., 319., 330., 360., 395., 420., 395., 310.];

/// Installed thrust for both healthy engines, plus the afterburner increment.
/// Continuous across the envelope; no special Mach-2.5 augmentation/recovery cliff.
pub fn engine(power: f64, density: f64, mach: f64) -> (f64, f64) {
    let dry = table(&MACH, &MIL_KN, mach) * table(&DENSITY, &DRY_FACTOR, density) * 1000.;
    let wet = table(&MACH, &MAX_KN, mach) * table(&DENSITY, &WET_FACTOR, density) * 1000.;
    let burner = (wet - dry).max(0.) * ((power - 0.88) / 0.12).clamp(0., 1.);
    (dry * (power / 0.88).clamp(0., 1.) + burner, burner)
}
pub fn inlet_recovery(alpha: f64, beta: f64, mach: f64) -> f64 {
    // Bounded distortion loss, including crossflow. No arbitrary random flameout.
    let alpha_loss = smooth(0.35, 1.30, alpha.abs());
    let beta_loss = smooth(0.12, 0.65, beta.abs());
    (1. - 0.20 * alpha_loss - (0.10 + 0.06 * smooth(0.8, 2., mach)) * beta_loss).max(0.60)
}
pub fn installed(f: &Flight) -> (f64, f64) {
    installed_at(f, f.density, f.mach)
}
pub fn installed_at(f: &Flight, density: f64, mach: f64) -> (f64, f64) {
    let (thrust, burner) = engine(f.spool, density, mach);
    let recovery = inlet_recovery(f.alpha, f.beta, mach);
    (thrust * recovery, burner * recovery)
}
pub fn running_fraction(f: &Flight) -> f64 {
    if !f.engine || (f.fuel <= 0. && !f.infinite_fuel) {
        return 0.;
    }
    (0..2)
        .filter(|&i| f.systems.engine_on[i])
        .map(|i| f.systems.engines[i].clamp(0., 1.))
        .sum::<f64>()
        * 0.5
}
pub fn fuel_flow(f: &Flight) -> f64 {
    let (thrust, _) = installed(f);
    let afterburner = ((f.spool - 0.88) / 0.12).clamp(0., 1.);
    // Estimated installed TSFC, kg/(N s), applied separately to running engines.
    let tsfc = (0.000019 + 0.000024 * afterburner) * (1. + 0.12 * smooth(0.8, 2.5, f.mach));
    thrust * running_fraction(f) * tsfc
}

const ALPHA: [f64; 15] = [
    0., 5., 10., 15., 20., 25., 30., 40., 50., 60., 75., 90., 120., 150., 180.,
];
const CL: [f64; 15] = [
    0., 0.36, 0.72, 1.04, 1.27, 1.34, 1.27, 1.10, 0.96, 0.82, 0.45, 0., -0.68, -0.70, 0.,
];
const CD_SEPARATED: [f64; 15] = [
    0., 0.002, 0.009, 0.025, 0.06, 0.12, 0.20, 0.38, 0.55, 0.70, 1.0, 1.20, 1., 0.4, 0.06,
];
const CM: [f64; 15] = [
    0., -0.003, -0.007, -0.012, -0.023, -0.038, -0.055, -0.070, -0.083, -0.090, -0.100, -0.09,
    -0.06, -0.03, 0.,
];

pub fn aero(c: Conditions, state: &mut FlowState, dt: f64) -> Coefficients {
    let speed = c.speed.max(1.);
    let chord = 78. / 13.56;
    let tau = (chord / speed * 1.2).clamp(0.025, 0.20);
    state.lag_alpha += (c.alpha - state.lag_alpha) * (1. - (-dt / tau).exp());
    let beta_rate = (c.beta - state.lag_beta) / (tau * 2.);
    state.lag_beta += (c.beta - state.lag_beta) * (1. - (-dt / (tau * 2.)).exp());
    let critical = 0.42 - 0.055 * smooth(0.75, 1.3, c.mach);
    for (side, attached) in [-1., 1.].into_iter().zip(state.wing_attached.iter_mut()) {
        let local_alpha = state.lag_alpha - side * c.rates.z * 13.56 / (4. * speed)
            + side * c.beta * 0.22 * c.alpha.sin().abs();
        let threshold = critical - 0.045 * (1. - *attached);
        let target = 1. - smooth(threshold - 0.035, threshold + 0.07, local_alpha.abs());
        let relaxation = tau * if target < *attached { 1.4 } else { 3.5 };
        *attached += (target - *attached) * (1. - (-dt / relaxation).exp());
        *attached = attached.clamp(0., 1.);
    }
    state.attached = (state.wing_attached[0] + state.wing_attached[1]) * 0.5;
    let a = c.alpha;
    let deg = a.abs().to_degrees().min(180.);
    let pitch_rate = c.rates.x * chord / (2. * speed);
    let yaw_rate = c.rates.y * 13.56 / (2. * speed);
    let roll_rate = c.rates.z * 13.56 / (2. * speed);
    let lift_mach = table(&MACH, &[1., 1., 0.99, 0.90, 0.83, 0.75, 0.65, 0.58], c.mach);
    let vortex_target = ((a - state.lag_alpha) / tau * chord / speed * 0.65).clamp(-0.15, 0.15)
        * smooth(0.12, 0.28, a.abs())
        * (1. - smooth(0.6, 1.1, a.abs()));
    state.vortex_lift += (vortex_target - state.vortex_lift) * (1. - (-dt / (tau * 2.5)).exp());
    let camber = 0.16 * (1. - smooth(15., 40., deg));
    let attached_bonus = 0.12 * (state.attached - (1. - smooth(20., 32., deg))) * a.sin();
    let lift = (camber
        + table(&ALPHA, &CL, deg) * a.signum() * lift_mach
        + attached_bonus
        + state.vortex_lift
        + 1.1 * pitch_rate)
        * c.beta.cos().powi(2);
    let reynolds = (c.density * speed * chord
        / (1.716e-5 * (c.temperature.max(120.) / 273.15).powf(1.5) * 383.55
            / (c.temperature.max(120.) + 110.4)))
        .max(10000.);
    let cf = 0.455 / (reynolds.log10().powf(2.58) * (1. + 0.144 * c.mach.powi(2)).powf(0.65));
    let parasite = 0.009 + cf * 3.8;
    let wave = table(
        &[0., 0.75, 0.95, 1.08, 1.3, 1.6, 2., 2.5, 3.],
        &[0., 0.002, 0.015, 0.033, 0.025, 0.017, 0.015, 0.017, 0.021],
        c.mach,
    );
    let induced = crate::aerodynamics::raptor_induced(lift, state.attached, c.beta, c.height);
    let drag = parasite
        + wave
        + induced
        + table(&ALPHA, &CD_SEPARATED, deg)
        + 0.22 * c.beta.sin().powi(2)
        + 0.05 * (1. - a.cos());
    let tail = table(
        &[0., 20., 40., 60., 90., 180.],
        &[1., 0.9, 0.55, 0.22, 0.08, 0.12],
        deg,
    );
    let cmq = table(&[0., 30., 60., 90., 180.], &[5.5, 4.7, 2.5, 1.2, 2.], deg);
    let pitch = 0.004 * (1. - smooth(20., 60., deg)) + table(&ALPHA, &CM, deg) * a.signum()
        - cmq * pitch_rate
        - 0.01 * smooth(0.85, 1.4, c.mach) * a.sin();
    let asymmetric = (state.wing_attached[1] - state.wing_attached[0]) * 0.012 * a.sin();
    Coefficients {
        lift,
        drag,
        pitch,
        parasite,
        wave,
        induced,
        side: -0.65 * tail * c.beta - 0.15 * tail * yaw_rate,
        yaw: -0.12 * tail * c.beta
            - 0.80 * tail.max(0.2) * yaw_rate
            - 0.12 * tail * (beta_rate * 13.56 / (2. * speed)).clamp(-0.2, 0.2),
        roll: -0.08 * tail * c.beta - 0.50 * (0.4 + 0.6 * tail) * roll_rate + asymmetric,
        control_scale: (0.25 + 0.75 * state.attached) * (1. - 0.25 * smooth(0.8, 1.4, c.mach)),
        attached: state.attached,
        control_drag: 0.,
    }
}

fn components(f: &Flight) -> [(f64, DVec3); 14] {
    let mut points = [(0., DVec3::ZERO); 14];
    // Balanced feed from forebody, wing and aft tanks. Tank geometry and feed
    // fractions are explicit estimates; no production fuel-transfer claim.
    for (i, (fraction, p)) in [
        (0.35, DVec3::new(0., -0.15, -1.1)),
        (0.20, DVec3::new(-1.8, -0.12, 0.)),
        (0.20, DVec3::new(1.8, -0.12, 0.)),
        (0.25, DVec3::new(0., -0.10, 1.7)),
    ]
    .into_iter()
    .enumerate()
    {
        points[i] = (f.fuel * fraction, p);
    }
    for i in 0..8 {
        if f.systems.stores[i] {
            points[i + 4] = (
                super::super::store_unit_mass(Aircraft::F22, i),
                station_for(f, i),
            );
        }
    }
    if f.systems.external_stores {
        points[12] = (35., DVec3::new(-3.35, -0.34, 0.85));
        points[13] = (35., DVec3::new(3.35, -0.34, 0.85));
    }
    points
}
pub fn center(f: &Flight) -> DVec3 {
    components(f).into_iter().map(|(m, p)| p * m).sum::<DVec3>() / f.mass()
}
pub fn inertia(f: &Flight) -> DMat3 {
    let cg = center(f);
    let shift = |m: f64, r: DVec3| {
        (DMat3::IDENTITY * r.length_squared() - DMat3::from_cols(r * r.x, r * r.y, r * r.z)) * m
    };
    let mut tensor = DMat3::from_diagonal(DVec3::new(212000., 258000., 49000.))
        + shift(f.aircraft.spec().empty, -cg);
    for (m, p) in components(f) {
        tensor += shift(m, p - cg);
    }
    tensor
}
pub fn approach_speed(f: &Flight, density: f64) -> f64 {
    // Weight/configuration-dependent equivalent airspeed, then convert to TAS.
    let cl = 0.72 + 0.20 * f.flaps.clamp(0., 1.);
    (2. * f.mass() * 9.80665 / (density.max(0.05) * 78. * cl)).sqrt() * 1.03
}
/// Estimated mechanical travel, radians. Damage jams are applied afterwards.
pub fn limit_angle(index: usize, angle: f64) -> f64 {
    let (low, high): (f64, f64) = match index {
        0 | 4 => (-25., 25.),
        1 | 5 => (-30., 20.),
        2 | 6 => (-25., 25.),
        3 | 7 => (-30., 30.),
        9 | 10 => (-5., 25.),
        _ => (0., 0.),
    };
    angle.clamp(low.to_radians(), high.to_radians())
}
pub fn nozzle_command(f: &Flight) -> DVec3 {
    if f.systems.raptor.actuators_initialized {
        f.systems.raptor.nozzle_control
    } else {
        f.surface
    }
}
pub fn structural_damage(f: &mut Flight, pressure: f64, dt: f64) {
    let positive = ((f.g_load - 9.5) / 2.).max(0.);
    let negative = ((-f.g_load - 3.5) / 2.).max(0.);
    let overspeed = ((pressure / 95000. - 1.).max(0.)).powi(2);
    let mach_excess = (f.mach - 2.35).max(0.) * smooth(500., 2000., pressure);
    let recovery_temp = f.temperature * (1. + 0.18 * f.mach.powi(2));
    let heat_rate = (pressure.max(0.) / 25000.).sqrt() / 45.;
    f.systems.raptor.skin_temperature +=
        (recovery_temp - f.systems.raptor.skin_temperature) * (1. - (-dt * heat_rate).exp());
    let thermal = ((f.systems.raptor.skin_temperature - 465.) / 100.).max(0.);
    f.integrity -= dt
        * (0.005 * positive
            + 0.007 * negative
            + 0.10 * overspeed
            + 0.08 * mach_excess
            + 0.02 * thermal);
}

pub fn advance_surfaces(f: &mut Flight, pressure: f64, dt: f64) {
    if f.damage.hydraulics > 0.12 {
        // Separate demand filter: aerodynamic hinge loading does not slow the
        // engine-nozzle command. Physical nozzles retain their own 1 rad/s stop.
        f.systems.raptor.nozzle_control = crate::control_surfaces::advance_actuator(
            f.systems.raptor.nozzle_control, f.systems.raptor.command, 0., dt * f.damage.hydraulics);
    }
    let panels = f.aircraft.panels();
    for i in 0..SURFACES {
        let target = super::super::panel_angle(
            Aircraft::F22,
            i,
            panels[i],
            f.systems.raptor.command,
            f.flaps,
            f.airbrake,
            f.alpha,
        );
        let target = crate::control_surfaces::effective_angle(&f.damage, i, target);
        if f.damage.hydraulics <= 0.12 {
            continue;
        }
        let max_rate = match i {
            0 | 4 => 65_f64,
            3 | 7 => 80.,
            9 | 10 => 35.,
            _ => 90.,
        }
        .to_radians();
        let loading = (1. + (pressure / 40000.).powi(2)).sqrt();
        let delta = (target - f.systems.raptor.angles[i]) * (1. - (-dt / 0.09).exp());
        let limit = max_rate * dt * f.damage.hydraulics / loading;
        f.systems.raptor.angles[i] += delta.clamp(-limit, limit);
    }
    f.systems.raptor.actuators_initialized = true;
}
pub fn bay_moment(f: &Flight, pressure: f64, body_air: DVec3) -> DVec3 {
    let drag_axis = -body_air.normalize_or_zero();
    let mut moment =
        DVec3::new(0., -0.5, 0.3).cross(drag_axis * pressure * 78. * f.systems.bays * 0.008);
    for (i, bay) in f.systems.side_bays.iter().enumerate() {
        let cd = 0.0018 * bay.door + 0.0009 * bay.launcher;
        moment += DVec3::new(if i == 0 { -1.8 } else { 1.8 }, -0.2, 0.6)
            .cross(drag_axis * pressure * 78. * cd);
    }
    moment
}

#[cfg(test)]
mod tests {
    use super::*;
    fn aircraft() -> Flight {
        let mut f = Flight::approach();
        select(&mut f, Aircraft::F22);
        f.density = 1.225;
        f.mach = 0.;
        f.spool = 1.;
        f.flaps = 0.65;
        f
    }
    #[test]
    fn engine_and_fuel_follow_running_engines_without_a_high_mach_cliff() {
        let mut f = aircraft();
        let both = fuel_flow(&f);
        f.systems.engine_on[0] = false;
        assert!((fuel_flow(&f) - both * 0.5).abs() < 1e-12);
        f.systems.engines[1] = 0.5;
        assert!((fuel_flow(&f) - both * 0.25).abs() < 1e-12);
        f.engine = false;
        assert_eq!(fuel_flow(&f), 0.);
        assert_eq!(engine(1., 0., 2.).0, 0.);
        for i in 0..300 {
            let mach = i as f64 * 0.01;
            let thrust = engine(1., 0.1937, mach).0;
            let next = engine(1., 0.1937, mach + 0.001).0;
            assert!((next - thrust).abs() < 500.);
            assert!(thrust.is_finite() && thrust > 0.);
        }
        assert!(inlet_recovery(1., 0.4, 1.) < inlet_recovery(0., 0., 1.));
    }
    #[test]
    fn side_bay_drag_and_moments_follow_actual_independent_openings() {
        let mut f = aircraft();
        let closed = store_drag(&f);
        f.systems.side_bays[0].door = 1.;
        let left = bay_moment(&f, 20000., -DVec3::Z * 200.);
        assert!(store_drag(&f) > closed && left.y.abs() > 100.);
        f.systems.side_bays.swap(0, 1);
        let right = bay_moment(&f, 20000., -DVec3::Z * 200.);
        assert!((left.y + right.y).abs() < 1e-9);
        f.systems.side_bays[0].door = 1.;
        assert!(bay_moment(&f, 20000., -DVec3::Z * 200.).y.abs() < 1e-9);
    }
    #[test]
    fn inertia_is_symmetric_positive_and_accounts_for_asymmetric_stores() {
        let mut f = aircraft();
        f.systems.external_stores = true;
        f.systems.stores[6] = false;
        let tensor = inertia(&f);
        assert!(
            (tensor - tensor.transpose())
                .to_cols_array()
                .iter()
                .all(|x| x.abs() < 1e-9)
        );
        assert!(tensor.determinant() > 0.);
        let diagonal = DVec3::new(tensor.x_axis.x, tensor.y_axis.y, tensor.z_axis.z);
        assert!(diagonal.element_sum() >= 2. * diagonal.max_element());
        for axis in [DVec3::X, DVec3::Y, DVec3::Z, DVec3::ONE] {
            assert!(axis.dot(tensor * axis) > 0.);
        }
        assert!(center(&f).x > 0.);
        assert!(tensor.x_axis.z.abs() > 1.);
        f.systems.stores.swap(6, 7);
        assert!(center(&f).x < 0.);
        assert!((tensor.x_axis.z - inertia(&f).x_axis.z).abs() > 100.);
        let full = inertia(&f).z_axis.z;
        f.fuel = 0.;
        assert!(inertia(&f).z_axis.z < full);
    }
    #[test]
    fn individual_actuators_are_rate_limited_and_jams_and_hydraulics_hold() {
        let mut f = aircraft();
        f.systems.raptor.actuators_initialized = true;
        f.surface = DVec3::new(1., 0.5, 0.3);
        f.systems.raptor.command = f.surface;
        advance_surfaces(&mut f, 10000., 0.01);
        assert!(f.systems.raptor.angles[0].abs() <= 65_f64.to_radians() * 0.01);
        assert!(f.systems.raptor.angles[9].abs() <= 35_f64.to_radians() * 0.01);
        f.damage.hydraulics = 0.;
        let before = f.systems.raptor.angles;
        advance_surfaces(&mut f, 10000., 1.);
        assert_eq!(before, f.systems.raptor.angles);
        f.damage.surface_state[0] = crate::combat::damage::SurfaceState::Jammed(0.23);
        assert_eq!(angles(&f)[0], 0.23);
        let loads = actual_panel_loads(&f, -DVec3::Z * 200., 1.);
        assert_eq!(loads.surfaces[0].deflection, 0.23);
        for command in [DVec3::ONE, -DVec3::ONE] {
            f.damage = AircraftDamage {
                aircraft: Aircraft::F22,
                ..default()
            };
            f.surface = command;
            f.systems.raptor.command = command;
            advance_surfaces(&mut f, 0., 10.);
            for (i, angle) in angles(&f).into_iter().enumerate() {
                assert_eq!(angle, limit_angle(i, angle));
            }
        }
    }
    #[test]
    fn nozzle_response_is_independent_of_the_pressure_loaded_surface_filter() {
        let mut f = aircraft();
        f.systems.raptor.actuators_initialized = true;
        f.systems.raptor.command = DVec3::Z * 0.5;
        f.surface = DVec3::ZERO;
        advance_surfaces(&mut f, 100000., 0.01);
        propulsion(&mut f, 0.01, 1.225, 0.);
        assert!(f.systems.nozzle[0] < 0. && f.systems.nozzle[1] > 0.);
        let before = f.systems.nozzle;
        f.damage.hydraulics = 0.;
        propulsion(&mut f, 1., 1.225, 0.);
        assert_eq!(before, f.systems.nozzle);
    }
    #[test]
    fn landing_speed_tracks_weight_density_and_configuration() {
        let mut f = aircraft();
        let speed = approach_speed(&f, 1.225);
        assert!((70. ..90.).contains(&speed), "approach {speed}");
        assert!((approach_speed(&f, 1.225 / 2.) / speed - 2_f64.sqrt()).abs() < 1e-10);
        f.fuel = 8200.;
        assert!(approach_speed(&f, 1.225) > speed);
        f.flaps = 0.;
        assert!(approach_speed(&f, 1.225) > speed * 1.1);
    }
    #[test]
    fn negative_load_and_dynamic_pressure_have_separate_structural_limits() {
        let mut positive = aircraft();
        positive.g_load = 6.;
        let mut negative = positive.clone();
        negative.g_load = -6.;
        let start = positive.integrity;
        structural_damage(&mut positive, 20000., 1.);
        structural_damage(&mut negative, 20000., 1.);
        assert_eq!(positive.integrity, start);
        assert!(negative.integrity < start);
        positive.g_load = 1.;
        structural_damage(&mut positive, 200000., 1.);
        assert!(positive.integrity < start);
        let mut vacuum = aircraft();
        vacuum.g_load = 0.;
        vacuum.mach = 30.;
        let temperature = vacuum.systems.raptor.skin_temperature;
        structural_damage(&mut vacuum, 0., 10.);
        assert_eq!(vacuum.integrity, start);
        assert_eq!(vacuum.systems.raptor.skin_temperature, temperature);
    }
    #[test]
    fn signed_g_commands_reach_the_requested_envelope_without_a_second_input_limiter() {
        for command in [-1., 1.] {
            let mut f = aircraft();
            f.position = surface_point(0., 0., 6000.);
            f.orientation = bevy::math::DQuat::IDENTITY;
            f.velocity = -DVec3::Z * 330.;
            f.gear = false;
            f.gear_extension = 0.;
            f.flaps = 0.;
            f.infinite_fuel = true;
            f.controls.x = command;
            let environment = Environment {
                wind_enabled: false,
                ..default()
            };
            let mut held = 0.;
            for i in 0..360 {
                crate::physics::simulate(&mut f, &environment, 1. / 120.);
                if i >= 300 {
                    held += f.g_load / 60.;
                }
            }
            assert!(!f.crashed && f.rates.is_finite());
            println!("F22 signed G command {command}: final-half-second mean {held:.4}");
            if command > 0. {
                assert!((8. ..10.).contains(&held), "positive hold {held}");
            } else {
                assert!((-3.8..-2.4).contains(&held), "negative hold {held}");
            }
        }
    }
}
