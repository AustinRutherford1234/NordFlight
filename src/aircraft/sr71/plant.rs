//! Spatial aircraft force assembly. Whole-aircraft static derivative anchors supply
//! unresolved pressure couples; local-flow, deformation and history increments remain physical.
use super::{
    airframe_config::{LANDING, SPATIAL, UNSTEADY},
    config::*,
    database,
    elastic::{self, LOAD_COUNT, PointLoad},
    elements::{self, COUNT, Kind},
    landing,
    math::*,
    unsteady,
};
use crate::{
    aerodynamics::{Coefficients, Conditions},
    control_surfaces::{Loads, SurfaceLoad},
    physics::{AircraftDamage, Flight},
};
use bevy::math::{DQuat, DVec3};
#[derive(Clone, Copy, Debug, Default)]
pub struct Wrench {
    pub force: DVec3,
    pub moment: DVec3,
}
#[derive(Clone, Copy, Debug)]
pub struct Local {
    pub at: DVec3,
    pub air: DVec3,
    pub frame: DQuat,
    pub speed: f64,
    pub mach: f64,
    pub alpha: f64,
    pub beta: f64,
    pub q: f64,
    pub pressure: f64,
    pub force: DVec3,
    pub moment: DVec3,
    pub flow: unsteady::State,
}
impl Default for Local {
    fn default() -> Self {
        Self {
            at: DVec3::ZERO,
            air: DVec3::ZERO,
            frame: DQuat::IDENTITY,
            speed: 0.,
            mach: 0.,
            alpha: 0.,
            beta: 0.,
            q: 0.,
            pressure: 0.,
            force: DVec3::ZERO,
            moment: DVec3::ZERO,
            flow: unsteady::State::default(),
        }
    }
}
#[derive(Clone)]
pub struct State {
    pub initialized: bool,
    pub elastic_enabled: bool,
    pub unsteady_enabled: bool,
    pub local: [Local; COUNT],
    pub controls: [Local; 6],
    pub structure: elastic::State,
    pub last_loads: [PointLoad; LOAD_COUNT],
    pub base: Wrench,
    pub total: Wrench,
    pub coefficients: Coefficients,
    pub air: DVec3,
    pub cg: DVec3,
    pub q: f64,
    pub mach: f64,
    pub height: f64,
    pub attached: f64,
    pub mean_pressure: f64,
    pub induced_force: DVec3,
    pub induced_center: DVec3,
    pub induction_weights: [f64; COUNT],
    pub total_induced_cd: f64,
    pub control_induced_force: DVec3,
    pub surface: landing::Surface,
    pub gear_extension: f64,
    pub gear_door: f64,
    pub last_specific_acceleration: DVec3,
    pub last_angular_acceleration: DVec3,
}
impl Default for State {
    fn default() -> Self {
        Self {
            initialized: false,
            elastic_enabled: true,
            unsteady_enabled: true,
            local: [Local::default(); COUNT],
            controls: [Local::default(); 6],
            structure: elastic::State::default(),
            last_loads: [PointLoad::default(); LOAD_COUNT],
            base: Wrench::default(),
            total: Wrench::default(),
            coefficients: Coefficients::default(),
            air: DVec3::ZERO,
            cg: DVec3::ZERO,
            q: 0.,
            mach: 0.,
            height: 10000.,
            attached: 1.,
            mean_pressure: 0.,
            induced_force: DVec3::ZERO,
            induced_center: DVec3::ZERO,
            induction_weights: [0.; COUNT],
            total_induced_cd: 0.,
            control_induced_force: DVec3::ZERO,
            surface: landing::Surface::free(),
            gear_extension: 0.,
            gear_door: 0.,
            last_specific_acceleration: DVec3::ZERO,
            last_angular_acceleration: DVec3::ZERO,
        }
    }
}
fn kin(at: DVec3, frame: DQuat, velocity: DVec3, density: f64, sound: f64) -> Local {
    let b = frame.conjugate() * velocity;
    let speed = b.length();
    Local {
        at,
        air: b,
        frame,
        speed,
        mach: speed / sound.max(1.),
        alpha: (-b.y).atan2(-b.z),
        beta: if speed > 1e-8 {
            (b.x / speed).clamp(-1., 1.).asin()
        } else {
            0.
        },
        q: 0.5 * density * speed * speed,
        ..Local::default()
    }
}
fn components(air: DVec3, lift: f64, drag: f64, side: f64) -> DVec3 {
    let flow = air.normalize_or_zero();
    let lift_axis = DVec3::X.cross(flow).normalize_or_zero();
    let side_axis = flow.cross(lift_axis).normalize_or_zero();
    lift_axis * lift - flow * drag + side_axis * side
}
fn state_polar(l: &Local, chord: f64, unsteady_enabled: bool) -> database::Polar {
    let effective = if unsteady_enabled {
        l.flow.effective_alpha()
    } else {
        l.alpha
    };
    let mut p = database::lookup(l.mach, effective, l.beta);
    if unsteady_enabled {
        let equilibrium = p.attached;
        let da = l.flow.attached - equilibrium;
        let incidence = effective - INCIDENCE;
        let plate = 1.85 * incidence.sin() * incidence.sin().abs();
        let attached_lift = (tab(&CNA, l.mach) * DEG * incidence.sin() * incidence.cos()
            + tab(&CNB, l.mach))
            * incidence.cos();
        p.lift += da * (attached_lift - plate * incidence.cos())
            + l.flow.vortex
            + l.flow.added_mass(chord, l.speed)
            + l.flow.buffet();
        p.drag += (equilibrium - l.flow.attached) * plate * incidence.sin();
        p.drag = p.drag.max(0.);
        p.pitch -=
            UNSTEADY.vortex_cp_travel * l.flow.vortex * (l.flow.vortex_age / 8.).clamp(0., 1.);
        p.attached = l.flow.attached;
    }
    p
}
pub fn control_point(i: usize) -> DVec3 {
    match i {
        0 => DVec3::new(-1.9, 0.14, 13.1),
        1 => DVec3::new(1.9, 0.14, 13.1),
        2 => DVec3::new(-6.8, 0.16, 12.2),
        3 => DVec3::new(6.8, 0.16, 12.2),
        4 => DVec3::new(-4., 2., 10.8),
        _ => DVec3::new(4., 2., 10.8),
    }
}
fn control_assembly(i: usize) -> usize {
    if i < 4 { i % 2 } else { i - 2 }
}
fn control_span(i: usize) -> f64 {
    if i < 2 {
        0.13
    } else if i < 4 {
        0.81
    } else {
        0.5
    }
}
pub fn prepare(
    state: &mut State,
    c: Conditions,
    air: DVec3,
    cg_fraction: f64,
    surface: landing::Surface,
    damage: &AircraftDamage,
    specific_acc: DVec3,
    angular_acc: DVec3,
    dt: f64,
    equilibrium: bool,
) {
    if super::diagnostics::validate_conditions(c, dt).is_err() {
        state.base = Wrench::default();
        state.coefficients = Coefficients::default();
        state.initialized = false;
        return;
    }
    if state.initialized && state.elastic_enabled {
        state.structure.advance(
            &state.last_loads,
            specific_acc,
            c.rates,
            angular_acc,
            dt,
            equilibrium,
        );
    }
    state.air = air;
    state.q = 0.5 * c.density * c.speed * c.speed;
    state.mach = c.mach;
    state.height = c.height;
    state.cg = DVec3::Z * ((cg_fraction - MASS.moment_reference_mac) * CHORD);
    state.surface = surface;
    state.attached = 0.;
    state.mean_pressure = 0.;
    state.base = Wrench::default();
    state.induced_center = DVec3::ZERO;
    let sound = if c.mach > 1e-6 {
        c.speed / c.mach
    } else {
        (1.4 * 287.05 * c.temperature).sqrt()
    };
    let reference = database::lookup(c.mach, c.alpha, c.beta);
    let mut reference_moment = DVec3::ZERO;
    let mut kinematic_force = DVec3::ZERO;
    let mut kinematic_moment = DVec3::ZERO;
    let h = 1e-4;
    let dl = (database::lookup(c.mach, c.alpha + h, c.beta).lift
        - database::lookup(c.mach, c.alpha - h, c.beta).lift)
        / (2. * h);
    let dy = (database::lookup(c.mach, c.alpha, c.beta + h).side
        - database::lookup(c.mach, c.alpha, c.beta - h).side)
        / (2. * h);
    let chine_wake: [f64; 2] = std::array::from_fn(|side| {
        let mut sum = 0.;
        for i in 12..18 {
            if (elements::geometry()[i].at.x < 0.) == (side == 0) {
                sum += state.local[i].flow.wake / 3.;
            }
        }
        sum
    });
    for (i, g) in elements::geometry().iter().enumerate() {
        let (at, frame, motion) = if let Some(a) = g.assembly.filter(|_| state.elastic_enabled) {
            (
                g.at + state.structure.displacement(a, g.span_fraction),
                state.structure.frame(a, g.span_fraction),
                state.structure.velocity(a, g.span_fraction, g.at),
            )
        } else {
            (g.at, DQuat::IDENTITY, DVec3::ZERO)
        };
        let mut velocity = air + c.rates.cross(at - state.cg) + motion;
        if g.kind == Kind::Wing {
            velocity.y += UNSTEADY.chine_downwash * chine_wake[usize::from(g.at.x > 0.)] * c.speed;
        }
        let mut l = kin(at, frame, velocity, c.density, sound);
        l.flow = state.local[i].flow;
        if equilibrium || !state.initialized {
            l.flow = unsteady::State::equilibrium(l.alpha, l.beta, l.mach);
        } else if state.unsteady_enabled {
            l.flow
                .advance(l.alpha, l.beta, l.mach, l.speed, g.chord, dt);
        }
        let target = landing::target(surface.clearance(at), l.mach);
        let mut pressure = landing::State {
            pressure: state.local[i].pressure,
        };
        if equilibrium || !state.initialized {
            pressure.pressure = target;
        } else {
            landing::advance(&mut pressure, target, dt);
        }
        l.pressure = pressure.pressure;
        let mut p = state_polar(&l, g.chord, state.unsteady_enabled);
        let viscosity = REGIMES.viscosity_reference_pa_s
            * (c.temperature / REGIMES.viscosity_reference_k).powf(1.5)
            * (REGIMES.viscosity_reference_k + REGIMES.sutherland_k)
            / (c.temperature + REGIMES.sutherland_k);
        let re = (c.density * l.speed * g.chord / viscosity).max(AERO.minimum_reynolds);
        let reynolds = (re / SPATIAL.reynolds_reference)
            .powf(SPATIAL.reynolds_lift_exponent)
            .clamp(0.90, 1.06);
        let skin = AERO.skin_friction_gain
            / (re.log10().powf(AERO.reynolds_exponent)
                * (1. + AERO.compressibility_gain * l.mach * l.mach)
                    .powf(AERO.compressibility_exponent));
        let reference_mu = REGIMES.viscosity_reference_pa_s
            * (230. / REGIMES.viscosity_reference_k).powf(1.5)
            * (REGIMES.viscosity_reference_k + REGIMES.sutherland_k)
            / (230. + REGIMES.sutherland_k);
        let reference_re = (0.4 * 300. * CHORD / reference_mu).max(AERO.minimum_reynolds);
        let reference_skin = AERO.skin_friction_gain
            / (reference_re.log10().powf(AERO.reynolds_exponent)
                * (1. + AERO.compressibility_gain * l.mach * l.mach)
                    .powf(AERO.compressibility_exponent));
        p.drag = (p.drag + (skin - reference_skin) * AERO.wetted_area_ratio).max(0.);
        p.lift *= reynolds
            * landing::lift_gain(
                l.pressure,
                surface.relative_velocity_body + c.rates.cross(at),
                surface.normal_body,
                l.speed,
            );
        let health = if let Some(a) = g.assembly {
            if a < 2 {
                damage.wings[a].min(1. - state.structure.damage[a])
            } else {
                let slot = if a == 2 { 3 } else { 7 };
                if damage.surface_state[slot] == crate::combat::damage::SurfaceState::Detached {
                    0.
                } else {
                    1. - state.structure.damage[a]
                }
            }
        } else {
            1.
        };
        l.force = frame
            * components(
                l.air,
                p.lift * g.normal_share,
                p.drag * g.drag_share,
                p.side * g.side_share,
            )
            * (l.q * AREA * health);
        // Static pressure couples anchor measured whole-aircraft moments, not local rate/elastic increments.
        let rf = components(
            air,
            reference.lift * g.normal_share,
            reference.drag * g.drag_share,
            reference.side * g.side_share,
        ) * (state.q * AREA);
        reference_moment += g.at.cross(rf);
        let dv = c.rates.cross(g.at);
        let da = -dv.y / c.speed.max(SPATIAL.local_speed_floor);
        let db = dv.x / c.speed.max(SPATIAL.local_speed_floor);
        let df = components(air, dl * da * g.normal_share, 0., dy * db * g.side_share)
            * (state.q * AREA);
        kinematic_force += df;
        kinematic_moment += g.at.cross(df);
        l.moment = (at - state.cg).cross(l.force);
        state.base.force += l.force;
        state.base.moment += l.moment;
        state.attached += p.attached * g.normal_share;
        state.mean_pressure += l.pressure * g.normal_share;
        state.induced_center += at * g.normal_share;
        state.last_loads[i] = PointLoad {
            assembly: g.assembly,
            point: at,
            force: l.force,
            moment: DVec3::ZERO,
        };
        state.local[i] = l;
    }
    state.attached = state.attached.clamp(0., 1.);
    state.mean_pressure = state.mean_pressure.clamp(0., 1.);
    let desired_moment = DVec3::new(
        reference.pitch * CHORD,
        reference.yaw * SPAN,
        reference.roll * SPAN,
    ) * (state.q * AREA);
    let pitch_rate = c.rates.x * CHORD / (2. * c.speed.max(SPATIAL.local_speed_floor));
    let roll_rate = -c.rates.z * SPAN / (2. * c.speed.max(SPATIAL.local_speed_floor));
    let yaw_rate = -c.rates.y * SPAN / (2. * c.speed.max(SPATIAL.local_speed_floor));
    let desired_rate_force = components(
        air,
        tab(&CNQ, c.mach) * pitch_rate * c.alpha.cos(),
        0.,
        -AERO.side_yaw_damping * c.rates.y * SPAN / (2. * c.speed.max(SPATIAL.local_speed_floor)),
    ) * (state.q * AREA);
    let desired_rate_moment = DVec3::new(
        tab(&CMQ, c.mach) * pitch_rate * CHORD,
        -(tab(&CNR, c.mach) * yaw_rate - AERO.yaw_roll_coupling * roll_rate) * SPAN,
        -(tab(&CLP, c.mach) * roll_rate + AERO.roll_yaw_coupling * yaw_rate) * SPAN,
    ) * (state.q * AREA);
    let residual_force = desired_rate_force - kinematic_force;
    state.base.force += residual_force;
    state.base.moment += desired_moment - reference_moment + desired_rate_moment
        - kinematic_moment
        - state.cg.cross(residual_force);
    let lift_axis = DVec3::X.cross(air.normalize_or_zero()).normalize_or_zero();
    state.induction_weights = state.local.map(|l| l.force.dot(lift_axis).abs());
    let lifting_load = state.induction_weights.iter().sum::<f64>();
    if lifting_load > 1e-9 {
        state.induction_weights = state.induction_weights.map(|w| w / lifting_load);
    } else {
        state.induction_weights = [0.; COUNT];
    }
    state.induced_center = state
        .local
        .iter()
        .zip(state.induction_weights)
        .map(|(l, w)| l.at * w)
        .sum();
    let lift = if state.q > 1e-9 {
        state.base.force.dot(lift_axis) / (state.q * AREA)
    } else {
        0.
    };
    let cd = super::aero::induced(lift, state.attached, c.beta, 10000., c.mach)
        * landing::induced_factor(state.mean_pressure);
    state.induced_force = -air.normalize_or_zero() * (state.q * AREA * cd);
    state.base.force += state.induced_force;
    state.base.moment += (state.induced_center - state.cg).cross(state.induced_force);
    // Transfer the same induced force into the physical element/structural loads,
    // without adding it a second time to the rigid-body force sum.
    for i in 0..COUNT {
        let force = state.induced_force * state.induction_weights[i];
        state.local[i].force += force;
        state.local[i].moment += (state.local[i].at - state.cg).cross(force);
        state.last_loads[i].force += force;
    }
    let ground_pitch = -LANDING.maximum_nose_down_cm
        * state.mean_pressure
        * (c.alpha / 0.15).clamp(-1.5, 1.5)
        * state.q
        * AREA
        * CHORD;
    state.base.moment.x +=
        ground_pitch + LANDING.gear_pitch_cm * state.gear_extension * state.q * AREA * CHORD;
    let gear_cd = LANDING.gear_drag * state.gear_extension + LANDING.door_drag * state.gear_door;
    for (i, weight) in [0.25, 0.375, 0.375].into_iter().enumerate() {
        let at = crate::aircraft::sr71::mount(i)
            + crate::aircraft::sr71::gear_rotation(state.gear_extension, i)
                * (crate::aircraft::sr71::wheel(i) - crate::aircraft::sr71::mount(i));
        let velocity = air + c.rates.cross(at - state.cg);
        let force = -velocity.normalize_or_zero()
            * (0.5
                * c.density
                * velocity.length_squared()
                * AREA
                * LANDING.gear_drag
                * state.gear_extension
                * weight);
        state.base.force += force;
        state.base.moment += (at - state.cg).cross(force);
    }
    let door_force =
        -air.normalize_or_zero() * (state.q * AREA * LANDING.door_drag * state.gear_door);
    state.base.force += door_force;
    state.base.moment += (DVec3::new(0., -0.35, 4.) - state.cg).cross(door_force);
    for i in 0..6 {
        let a = control_assembly(i);
        let s = control_span(i);
        let point = control_point(i);
        let (frame, at, motion) = if state.elastic_enabled {
            (
                state.structure.frame(a, s),
                point + state.structure.displacement(a, s),
                state.structure.velocity(a, s, point),
            )
        } else {
            (DQuat::IDENTITY, point, DVec3::ZERO)
        };
        let mut l = kin(
            at,
            frame,
            air + c.rates.cross(at - state.cg) + motion,
            c.density,
            sound,
        );
        l.flow.attached = state.attached;
        l.pressure = landing::target(surface.clearance(at), l.mach);
        state.controls[i] = l;
    }
    state.coefficients = to_coefficients(state.base, air, state.q);
    state.coefficients.induced = cd;
    state.coefficients.attached = state.attached;
    state.coefficients.parasite = reference.profile_drag + gear_cd;
    state.coefficients.wave = reference.wave_drag;
    state.initialized = true;
}
pub fn to_coefficients(w: Wrench, air: DVec3, q: f64) -> Coefficients {
    if q * AREA < 1e-9 {
        return Coefficients::default();
    }
    let flow = air.normalize_or_zero();
    let lift_axis = DVec3::X.cross(flow).normalize_or_zero();
    let side_axis = flow.cross(lift_axis).normalize_or_zero();
    let qs = q * AREA;
    Coefficients {
        lift: w.force.dot(lift_axis) / qs,
        drag: -w.force.dot(flow) / qs,
        side: w.force.dot(side_axis) / qs,
        pitch: w.moment.x / (qs * CHORD),
        yaw: w.moment.y / (qs * SPAN),
        roll: w.moment.z / (qs * SPAN),
        ..Default::default()
    }
}
pub fn control_loads(
    s: &State,
    damage: &AircraftDamage,
    elevons: [f64; 4],
    fins: [f64; 2],
) -> Loads {
    let neutral = super::controls::targets(DVec3::ZERO);
    let mut out = Loads::default();
    for i in 0..6 {
        let slot = [0, 4, 2, 6, 3, 7][i];
        let l = s.controls[i];
        let nominal = if i < 4 { neutral[i] } else { 0. };
        let requested = if i < 4 { elevons[i] } else { fins[i - 4] };
        let raw = crate::control_surfaces::effective_angle(damage, slot, requested);
        let down = if i < 2 {
            CONTROL.inner_down_rad
        } else if i < 4 {
            CONTROL.outer_down_rad
        } else {
            CONTROL.rudder_limit_rad
        };
        let up = if i < 4 {
            CONTROL.elevon_up_rad
        } else {
            CONTROL.rudder_limit_rad
        };
        let health = if damage.surface_state[slot] == crate::combat::damage::SurfaceState::Detached
        {
            0.
        } else {
            damage.surfaces[slot].clamp(0., 1.)
        };
        let delta = (raw.clamp(-up, down) - nominal) * health;
        let mut e = neutral;
        let mut r = [0.; 2];
        if i < 4 {
            e[i] += delta;
        } else {
            r[i - 4] = delta;
        }
        let mut c = super::controls::control_coefficients(e, r, l.mach, l.flow.attached);
        let factor = database::control_factor(l.mach, l.alpha - INCIDENCE, delta);
        c.lift *= factor;
        c.side *= factor;
        c.pitch *= factor;
        c.yaw *= factor;
        c.roll *= factor;
        let pressure_gain = landing::lift_gain(
            l.pressure,
            s.surface.relative_velocity_body,
            s.surface.normal_body,
            l.speed,
        );
        c.lift *= pressure_gain;
        let force = l.frame * components(l.air, c.lift, c.drag, c.side) * (l.q * AREA);
        let reference_force = components(s.air, c.lift, c.drag, c.side) * (l.q * AREA);
        let target_moment = DVec3::new(c.pitch * CHORD, c.yaw * SPAN, c.roll * SPAN) * (l.q * AREA);
        let pure = l.frame * (target_moment - control_point(i).cross(reference_force));
        let moment = (l.at - s.cg).cross(force) + pure;
        out.force += force;
        out.moment += moment;
        out.surfaces[slot] = SurfaceLoad {
            force,
            moment,
            deflection: nominal + delta,
            attached: l.flow.attached,
        };
    }
    let flow = s.air.normalize_or_zero();
    let lift_axis = DVec3::X.cross(flow).normalize_or_zero();
    let qs = s.q * AREA;
    if qs > 1e-9 {
        let lift = (s.base.force + out.force).dot(lift_axis) / qs;
        let beta = if s.air.length() > 0. {
            (s.air.x / s.air.length()).clamp(-1., 1.).asin()
        } else {
            0.
        };
        let cd = super::aero::induced(lift, s.attached, beta, 10000., s.mach)
            * landing::induced_factor(s.mean_pressure);
        let added = -flow * (qs * (cd - s.coefficients.induced));
        let moment = (s.induced_center - s.cg).cross(added);
        out.force += added;
        out.moment += moment;
        for slot in [0, 4, 2, 6, 3, 7] {
            out.surfaces[slot].force += added / 6.;
            out.surfaces[slot].moment += moment / 6.;
        }
    }
    out
}
pub fn finish(state: &mut State, loads: &Loads) {
    state.total = Wrench {
        force: state.base.force + loads.force,
        moment: state.base.moment + loads.moment,
    };
    let flow = state.air.normalize_or_zero();
    let lift_axis = DVec3::X.cross(flow).normalize_or_zero();
    let qs = state.q * AREA;
    let beta = if state.air.length() > 0. {
        (state.air.x / state.air.length()).clamp(-1., 1.).asin()
    } else {
        0.
    };
    state.total_induced_cd = if qs > 1e-9 {
        super::aero::induced(
            state.total.force.dot(lift_axis) / qs,
            state.attached,
            beta,
            10000.,
            state.mach,
        ) * landing::induced_factor(state.mean_pressure)
    } else {
        0.
    };
    let induced_delta = -flow * (qs * (state.total_induced_cd - state.coefficients.induced));
    let induced_moment = (state.induced_center - state.cg).cross(induced_delta);
    state.control_induced_force = induced_delta;
    // Control aggregate bookkeeping includes induced drag. Transfer that part to
    // the generating lifting elements for the beam loads, rather than to fin hinges.
    for i in 0..COUNT {
        state.last_loads[i].force += induced_delta * state.induction_weights[i];
    }
    for i in 0..6 {
        let slot = [0, 4, 2, 6, 3, 7][i];
        let l = loads.surfaces[slot];
        let at = state.controls[i].at;
        let force = l.force - induced_delta / 6.;
        let moment = l.moment - induced_moment / 6.;
        state.last_loads[COUNT + i] = PointLoad {
            assembly: Some(control_assembly(i)),
            point: at,
            force,
            moment: moment - (at - state.cg).cross(force),
        };
    }
}
pub fn begin(f: &mut Flight, c: Conditions, b: DVec3, dt: f64) {
    let world_normal = crate::carrier::normal(f, f.position);
    let ground = crate::carrier::ground(f, f.position);
    let origin = f.up() * (crate::physics::RADIUS + ground);
    let surface = landing::Surface {
        normal_body: f.orientation.conjugate() * world_normal,
        distance_m: (f.position - origin).dot(world_normal),
        relative_velocity_body: f.orientation.conjugate()
            * (f.velocity - crate::carrier::ground_velocity(f, f.position)),
    };
    let specific = f.systems.sr71.airframe.last_specific_acceleration;
    let angular = f.systems.sr71.airframe.last_angular_acceleration;
    let cg = super::mass::cg_fraction(f);
    let s = &mut f.systems.sr71.airframe;
    s.gear_extension = f.gear_extension;
    s.gear_door = f.gear_door;
    prepare(
        s, c, b, cg, surface, &f.damage, specific, angular, dt, false,
    );
    f.flow.attached = s.attached;
    f.flow.wing_attached = std::array::from_fn(|side| {
        s.local[side * 6..side * 6 + 6]
            .iter()
            .map(|l| l.flow.attached)
            .sum::<f64>()
            / 6.
    });
    f.flow.lag_alpha = c.alpha - INCIDENCE;
    f.flow.lag_beta = c.beta;
    f.flow.wing_attached = f.flow.wing_attached.map(|a| a.clamp(0., 1.));
    for side in 0..2 {
        f.damage.wing_stress[side] = s.structure.sections[side][0].normal_stress_pa;
        f.damage.wings[side] = f.damage.wings[side].min(1. - s.structure.damage[side]);
    }
}
pub fn steady(
    c: Conditions,
    air: DVec3,
    cg: f64,
    damage: &AircraftDamage,
    elevons: [f64; 4],
    fins: [f64; 2],
    surface: landing::Surface,
    elastic: bool,
) -> State {
    let mut state = State::default();
    state.elastic_enabled = elastic;
    state.unsteady_enabled = false;
    for _ in 0..if elastic { 10 } else { 1 } {
        prepare(
            &mut state,
            c,
            air,
            cg,
            surface,
            damage,
            DVec3::Y * 9.80665,
            DVec3::ZERO,
            0.,
            true,
        );
        let controls = control_loads(&state, damage, elevons, fins);
        finish(&mut state, &controls);
    }
    state.unsteady_enabled = true;
    state.last_specific_acceleration = DVec3::Y * 9.80665;
    state
}
