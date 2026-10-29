//! Dedicated Super Hornet plant: forces in N, moments in N m, body +X right,
//! +Y up, +Z aft. No control law applies angular velocity or attitude directly.
pub mod aero;
pub mod fbw;
pub mod config;
pub mod engine;
pub mod mass;
pub mod stores;
pub mod structure;
pub mod trim;
use crate::physics::*;
use bevy::math::{DMat3, DQuat, DVec3};
#[derive(Clone, Copy, Debug, Default)]
pub struct Loads {
    pub force: DVec3,
    pub moment: DVec3,
    pub wings: [f64; 2],
    pub elastic: [f64; 6],
}
impl Loads {
    pub fn add(&mut self, force: DVec3, arm: DVec3) {
        self.force += force;
        self.moment += arm.cross(force);
    }
}
#[derive(Clone, Debug, Default)]
pub struct Diagnostics {
    pub aero: Loads,
    pub engine: Loads,
    pub contact: Loads,
    pub attachment: Loads,
    pub acceleration: DVec3,
    pub angular_acceleration: DVec3,
}
#[derive(Clone, Debug)]
pub struct Dynamics {
    pub fbw:fbw::State,
    pub hud:crate::cockpit::hornet_hud::State,
    pub tanks: stores::Tanks,
    pub nose_steering: f64,
    pub skin_temperature: f64,
    pub structural_loads: structure::Channels,
    pub hydraulic_pressure: f64,
    pub hook_angle: f64,
    pub modes: [structure::Mode; 6],
    pub trim: f64,
    pub cable_energy: f64,
    pub initialized: bool,
    pub engines: [engine::Engine; 2],
    pub flow: [aero::Flow; 8],
    pub angles: [f64; 16],
    pub actuator_rates: [f64; 16],
    pub flex: [f64; 2],
    pub flex_rate: [f64; 2],
    pub twist: [f64; 2],
    pub fatigue: [f64; 2],
    pub diagnostic: Diagnostics,
}
impl Default for Dynamics {
    fn default() -> Self {
        Self {
            fbw:Default::default(),
            hud:Default::default(),
            tanks: Default::default(),
            nose_steering: 0.,
            skin_temperature: 288.15,
            structural_loads: Default::default(),
            hydraulic_pressure: 1.,
            hook_angle: 0.,
            modes: [structure::Mode::default(); 6],
            trim: 0.,
            cable_energy: 0.,
            initialized: false,
            engines: [engine::Engine::default(); 2],
            flow: [aero::Flow::default(); 8],
            angles: [0.; 16],
            actuator_rates: [0.; 16],
            flex: [0.; 2],
            flex_rate: [0.; 2],
            twist: [0.; 2],
            fatigue: [0.; 2],
            diagnostic: Diagnostics::default(),
        }
    }
}
pub fn wheel(i: usize) -> (DVec3, f64) {
    if i == 0 {
        (DVec3::new(0., -1.667, -3.61), 0.285)
    } else {
        (
            DVec3::new(if i == 1 { -1.603 } else { 1.603 }, -1.68, 1.95),
            0.41,
        )
    }
}
fn contacts(f: &mut Flight, dt: f64) -> Loads {
    let mut l = Loads::default();
    let (cg, _) = mass::properties(f);
    f.gear_compression = [0.; 3];
    let mut contact = false;
    for i in 0..3 {
        if f.gear_extension < 0.98 || f.gear_health[i] <= 0. {
            continue;
        }
        let (point, radius) = wheel(i);
        let p = f.position + f.orientation * point;
        let n = crate::carrier::normal(f, p);
        let gap = p.length() - RADIUS - crate::carrier::ground(f, p) - radius;
        if gap > 0. {
            continue;
        }
        contact = true;
        let arm = point - cg;
        let v =
            f.velocity - crate::carrier::ground_velocity(f, p) + f.orientation * f.rates.cross(arm);
        let vn = v.dot(n);
        let travel = if i == 0 { 0.40 } else { 0.48 };
        let compression = (-gap).clamp(0., travel);
        f.gear_compression[i] = compression;
        let k = if i == 0 { 700000. } else { 470000. };
        let damping = if i == 0 { 45000. } else { 45000. };
        let load = (k * compression - damping * vn + (-gap - travel).max(0.) * 2000000.)
            .clamp(0., 900000.);
        let mut forward = (f.forward() - n * f.forward().dot(n)).normalize_or_zero();
        let mut right = forward.cross(n).normalize_or_zero();
        if i == 0 {
            let speed = v.dot(forward).abs();
            // A passive caster estimate when steering is released at speed.
            // Steering acts by turning the tyre, never by an extra side force.
            let target = if speed > 20. && f.controls.y.abs() < 0.05 {
                v.dot(right).atan2(speed).clamp(-0.5, 0.5)
            } else {
                -f.controls.y * 0.35 / (1. + speed * 0.05)
            };
            f.hornet.nose_steering += (target - f.hornet.nose_steering).clamp(-2. * dt, 2. * dt);
            forward = (forward * f.hornet.nose_steering.cos()
                + right * f.hornet.nose_steering.sin())
            .normalize_or_zero();
            right = forward.cross(n).normalize_or_zero();
        }
        let longitudinal = v.dot(forward);
        let lateral = v.dot(right);
        let rolling = longitudinal.tanh() * load * (0.016 + if f.brakes { 0.55 } else { 0. });
        let friction = if crate::carrier::deck(f, p).is_some() {
            0.35
        } else {
            0.65
        };
        let lateral_force = (lateral * 14000.).clamp(-load * friction, load * friction);
        let force = n * load - forward * rolling - right * lateral_force;
        l.add(f.orientation.conjugate() * force, arm);
        if load > 700000. {
            f.gear_health[i] = (f.gear_health[i] - dt * (load - 700000.) / 150000.).max(0.);
        }
    }
    if contact && !f.landed {
        f.touchdown_sink = (-f.velocity.dot(f.up())).max(0.);
        f.status = "F/A-18F TOUCHDOWN".into();
    }
    f.landed = contact;
    l
}
pub fn step(f: &mut Flight, e: &Environment, dt: f64) {
    if dt <= 0. || !dt.is_finite() || f.paused {
        return;
    }
    crate::combat::damage::advance(&mut f.damage, &mut f.integrity, dt, f.engine, f.fuel > 0.);
    if crate::combat::damage::fatal(&f.damage, f.integrity) {
        crate::collision::destroy(f, "F/A-18F STRUCTURAL FAILURE", f.up());
        return;
    }
    f.autopilot=false;
    let before = f.position;
    let previous_orientation = f.orientation;
    f.elapsed += dt;
    stores::advance(f,dt);
    crate::collision::update_gear(f, dt);
    let hook_target = if f.systems.hook_down { 0.65 } else { 0. };
    f.hornet.hook_angle += (hook_target - f.hornet.hook_angle).clamp(-0.32 * dt, 0.32 * dt);
    if !f.hornet.initialized {
        for engine in &mut f.hornet.engines {
            engine.spool = f.spool;
        }
        f.hornet.initialized = true;
    }
    let (temp, pressure, rho, sound) = atmosphere(f.altitude());
    f.temperature = temp;
    f.pressure = pressure;
    f.density = rho;
    f.wind = e.wind(f.position);
    let air = f.orientation.conjugate() * (f.velocity - f.wind);
    f.airspeed = air.length();
    f.mach = f.airspeed / sound;
    f.alpha = (-air.y).atan2(-air.z);
    f.beta = (air.x / f.airspeed.max(0.1)).clamp(-1., 1.).asin();
    let (cg, inertia) = mass::properties(f);
    let engine = engine::advance(f, dt);
    f.hornet.diagnostic.engine=engine;
    let aero = aero::evaluate(f, dt);
    let contact = contacts(f, dt);
    let attachment = crate::carrier::physics_loads(f, dt);
    let body_force = aero.force + engine.force + contact.force + attachment.force;
    let moment = aero.moment + engine.moment + contact.moment + attachment.moment;
    let omega = rotation();
    let gravity = -f.up() * MU / f.position.length_squared();
    let rotating = -2. * omega.cross(f.velocity) - omega.cross(omega.cross(f.position));
    let sun = e.sun_direction() * e.sun_distance();
    let ds = sun - f.position;
    let tidal = SUN_MU * (ds / ds.length().powi(3) - sun / sun.length().powi(3));
    let acc = gravity + rotating + tidal + f.orientation * body_force / f.mass();
    let angular = inertia.inverse() * (moment - f.rates.cross(inertia * f.rates));
    f.velocity += acc * dt;
    f.position += f.velocity * dt;
    f.rates += angular * dt;
    f.orientation = (f.orientation * DQuat::from_scaled_axis(f.rates * dt)).normalize();
    // Flight.position remains the mesh/body reference point; integrate its motion
    // from CG velocity without introducing a second world-position convention.
    f.position += previous_orientation * cg - f.orientation * cg;
    let specific = body_force / f.mass();
    f.g_load = specific.y / 9.80665;
    f.lateral_g = specific.x / 9.80665;
    f.longitudinal_g = -specific.z / 9.80665;
    f.hornet.diagnostic = Diagnostics {
        aero,
        engine,
        contact,
        attachment,
        acceleration: acc,
        angular_acceleration: angular,
    };
    structure::advance(f, &aero, dt);
    let q = 0.5 * rho * f.airspeed.powi(2);
    f.aero_coeff = crate::aerodynamics::Coefficients::default();
    if q > 0.1 {
        f.aero_coeff.lift = aero.force.y / (q * config::AREA);
        f.aero_coeff.drag = aero.force.dot(-air.normalize_or_zero()) / (q * config::AREA);
    }
    f.panel_attachment = f.hornet.flow.iter().map(|s| s.attached).sum::<f64>() / 8.;
    f.systems.engines = f.damage.engines;
    crate::carrier::after_step(f, before, dt);
    crate::collision::check_airframe(f, before, previous_orientation);
    if (crate::carrier::sea(f.position.x, f.position.z) || in_lake(f.position.x, f.position.z))
        && f.altitude() < 0.7
    {
        crate::collision::destroy(f, "WATER IMPACT", f.up());
    }
    if !f.position.is_finite() || !f.rates.is_finite() {
        crate::collision::destroy(f, "PHYSICS NONFINITE", f.up());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn flight() -> Flight {
        let mut f = Flight::approach();
        crate::aircraft::select(&mut f, crate::aircraft::Aircraft::Hornet);
        f.assist = false;
        f.position = surface_point(0., 0., 3000.);
        f.velocity = DVec3::NEG_Z * 180.;
        f.gear = false;
        f.gear_extension = 0.;
        f.flaps = 0.;
        f
    }
    #[test]
    fn mass_tensor_and_asymmetric_stores() {
        let mut f = flight();
        let (c, i) = mass::properties(&f);
        assert!(i.determinant() > 0.);
        assert!((i - i.transpose()).x_axis.length() < 1e-8);
        f.systems.stores[0] = false;
        let (d, j) = mass::properties(&f);
        assert!((c - d).length() > 0.001);
        assert_ne!(i, j);
        for v in [DVec3::X, DVec3::Y, DVec3::Z, DVec3::ONE] {
            assert!(v.dot(j * v) > 0.);
        }
    }
    #[test]
    fn engine_asymmetry_fuel_and_shutdown() {
        let mut f = flight();
        f.hornet.initialized = true;
        f.spool = 0.9;
        f.density = 1.225;
        f.mach = 0.;
        f.hornet.engines[0].spool = 0.9;
        f.hornet.engines[1].spool = 0.9;
        let fuel = f.fuel;
        let a = engine::advance(&mut f, 0.1);
        assert!(a.force.z < -100000.);
        assert!(f.fuel < fuel);
        f.systems.engine_on[0] = false;
        let b = engine::advance(&mut f, 0.1);
        assert!(b.moment.y.abs() > 10000.);
        f.fuel = 0.;
        engine::advance(&mut f, 0.1);
        assert_eq!(f.thrust, 0.);
    }
    #[test]
    fn stabilator_nose_up_force_and_no_instant_actuation() {
        let mut f = flight();
        f.density = 1.;
        f.alpha = 0.1;
        f.controls.x = 1.;
        let before = f.hornet.angles[0];
        aero::advance_actuators(&mut f, 10000., 1. / 240.);
        assert!(f.hornet.angles[0] > before && f.hornet.angles[0] - before < 0.01);
        let baseline = aero::evaluate(&mut f, 0.);
        f.hornet.angles[0] = 0.3;
        f.hornet.angles[4] = 0.3;
        let raised = aero::evaluate(&mut f, 0.);
        assert!(raised.moment.x > baseline.moment.x);
        assert!(raised.force.y < baseline.force.y);
    }
    #[test]
    fn trim_matrix() {
        let f = flight();
        for (h, v) in [(0., 120.), (3000., 180.), (8000., 250.), (12000., 300.)] {
            let t = trim::solve(&f, h, v, 0., 0.).expect("trim");
            println!(
                "TRIM h={h} v={v} alpha={} tail={} power={} residual={:?}",
                t.alpha, t.stabilator, t.throttle, t.residual
            );
            assert!(
                t.residual.x.abs() < 0.01
                    && t.residual.y.abs() < 0.01
                    && t.residual.z.abs() < 0.001
            );
        }
    }
    #[test]
    fn convergence_and_benchmark() {
        let base = flight();
        let trim = trim::solve(&base, 3000., 180., 0., 0.).unwrap();
        let mut results = Vec::new();
        for hz in [60, 120, 240] {
            let mut f = base.clone();
            trim::apply(&mut f, trim);
            f.orientation = DQuat::from_rotation_x(trim.alpha);
            let e = Environment {
                wind_enabled: false,
                ..Default::default()
            };
            let start = std::time::Instant::now();
            for _ in 0..hz * 3 {
                simulate(&mut f, &e, 1. / hz as f64);
            }
            println!(
                "RUN hz={hz} pos={:?} speed={} rates={:?} cpu_ms={}",
                f.position,
                f.velocity.length(),
                f.rates,
                start.elapsed().as_secs_f64() * 1000.
            );
            assert!(!f.crashed && f.position.is_finite());
            results.push(f);
        }
        for f in &results[..2] {
            assert!((f.position - results[2].position).length() < 2.);
            assert!(
                (f.velocity - results[2].velocity).length() < results[2].velocity.length() * 0.02
            );
        }
    }
    #[test]
    fn zero_flow_and_damage_finite() {
        let mut f = flight();
        f.velocity = DVec3::ZERO;
        f.wind = DVec3::ZERO;
        f.rates = DVec3::ZERO;
        let a = aero::evaluate(&mut f, 0.01);
        assert_eq!(a.force, DVec3::ZERO);
        f.velocity = DVec3::NEG_Z * 100.;
        f.damage.wings[0] = 0.;
        let a = aero::evaluate(&mut f, 0.01);
        assert!(a.force.is_finite() && a.wings[0] == 0.);
    }
}

#[cfg(test)]
mod validation {
    use super::*;
    fn trimmed() -> Flight {
        let mut f = Flight::approach();
        crate::aircraft::select(&mut f, crate::aircraft::Aircraft::Hornet);
        f.assist = false;
        f.position = surface_point(0., 0., 3000.);
        f.gear = false;
        f.gear_extension = 0.;
        f.flaps = 0.;
        f.airspeed = 180.;
        f.velocity = DVec3::NEG_Z * 180.;
        let t = trim::solve(&f, 3000., 180., 0., 0.).unwrap();
        f.orientation = DQuat::from_rotation_x(t.alpha);
        trim::apply(&mut f, t);
        f
    }
    #[test]
    fn table_continuity_and_surface_signs() {
        let mut f = trimmed();
        let base = aero::forces(&mut f, 0.);
        for (u, axis, sign) in [(DVec3::X, 0, 1.), (DVec3::Y, 1, 1.), (DVec3::Z, 2, -1.)] {
            f.hornet.angles = aero::targets(&f, u * 0.2);
            let load = aero::forces(&mut f, 0.);
            assert!((load.moment[axis] - base.moment[axis]) * sign > 0.);
            f.hornet.angles = aero::targets(&f, DVec3::ZERO);
        }
        for x in [0.1, 0.2, 0.3, 0.45, 0.65, 1.] {
            let a = config::table(x - 1e-8, &[0., 0.1, 0.3, 0.65, 1.], &[0., 1., 2., 1., 0.]);
            let b = config::table(x + 1e-8, &[0., 0.1, 0.3, 0.65, 1.], &[0., 1., 2., 1., 0.]);
            assert!((a - b).abs() < 1e-5);
        }
    }
    #[test]
    fn modal_free_decay_removes_energy() {
        for hz in [6., 8., 10., 12., 15., 16.] {
            let (mut x, mut v) = (0.1, 0.);
            let w = std::f64::consts::TAU * hz;
            let start = 0.5 * w * w * x * x;
            for _ in 0..2400 {
                crate::structural::oscillator(&mut x, &mut v, 0., 1., hz, 0.18, 1. / 240.);
            }
            assert!(0.5 * (v * v + w * w * x * x) < start * 1e-10);
        }
    }
    #[test]
    fn jam_hydraulics_and_detachment() {
        let mut f = trimmed();
        f.damage.surface_state[0] = crate::combat::damage::SurfaceState::Jammed(0.12);
        f.controls.x = 1.;
        aero::advance_actuators(&mut f, 20000., 0.1);
        assert_eq!(f.hornet.angles[0], 0.12);
        f.damage.hydraulics = 0.;
        f.hornet.actuator_rates = [0.; 16];
        let before = f.hornet.angles;
        aero::advance_actuators(&mut f, 20000., 0.1);
        assert_eq!(before, f.hornet.angles);
        let mass = f.mass();
        f.damage.wing_broken[0] = true;
        assert_eq!(mass - f.mass(), 750.);
    }
    #[test]
    fn gust_stall_engine_out_and_reversal_are_finite() {
        for kind in 0..5 {
            let mut f = trimmed();
            if kind == 0 {
                f.velocity += DVec3::new(15., 10., 0.);
            }
            if kind == 1 {
                f.orientation = DQuat::from_rotation_x(0.85);
            }
            if kind == 2 {
                f.systems.engine_on[0] = false;
            }
            if kind == 3 {
                f.systems.stores[0] = false;
            }
            if kind == 4 {
                f.damage.surface_state[4] = crate::combat::damage::SurfaceState::Jammed(0.15);
            }
            let e = Environment {
                wind_enabled: false,
                ..Default::default()
            };
            for n in 0..1200 {
                if kind == 0 {
                    f.controls.x = if n < 600 { 0.2 } else { -0.2 };
                }
                simulate(&mut f, &e, 1. / 240.);
                assert!(f.position.is_finite() && f.velocity.is_finite() && f.rates.is_finite());
            }
            assert!(!f.crashed, "case {kind}: {}", f.status);
        }
    }
    #[test]
    fn performance_and_long_trim_evidence() {
        let mut f = trimmed();
        let start = f.position;
        let e = Environment {
            wind_enabled: false,
            ..Default::default()
        };
        let mut samples = Vec::new();
        let mut max_rate: f64 = 0.;
        for _ in 0..1800 {
            let now = std::time::Instant::now();
            simulate(&mut f, &e, 1. / 60.);
            samples.push(now.elapsed().as_secs_f64() * 1000.);
            max_rate = max_rate.max(f.rates.length());
        }
        let raw_altitude = f.altitude();
        println!(
            "RAW_30s altitude={} speed={} max_rate={} drift={}",
            raw_altitude,
            f.airspeed,
            max_rate,
            (f.position - start).length()
        );
        assert!(!f.crashed);
        f.assist = true;
        for _ in 0..1800 {
            let now = std::time::Instant::now();
            simulate(&mut f, &e, 1. / 60.);
            samples.push(now.elapsed().as_secs_f64() * 1000.);
        }
        samples.sort_by(f64::total_cmp);
        let avg = samples.iter().sum::<f64>() / samples.len() as f64;
        let p95 = samples[samples.len() * 95 / 100];
        println!(
            "PERFORMANCE frames={} avg_ms={} p95_ms={} assisted_speed={} altitude={}",
            samples.len(),
            avg,
            p95,
            f.airspeed,
            f.altitude()
        );
        assert!(!f.crashed);
        assert!(avg < 2. && p95 < 4.);
        let d = &f.hornet.diagnostic;
        assert!(d.acceleration.is_finite() && d.angular_acceleration.is_finite());
        assert!(d.aero.force.is_finite() && d.contact.force.is_finite());
    }
}

#[cfg(test)]
mod numerical_envelope {
    use super::*;
    fn base() -> Flight {
        let mut f = Flight::approach();
        crate::aircraft::select(&mut f, crate::aircraft::Aircraft::Hornet);
        f.gear = false;
        f.gear_extension = 0.;
        f.flaps = 0.;
        f.assist = false;
        f
    }
    #[test]
    fn advertised_envelope_plausibility() {
        let mut f = base();
        // Advertised maximum speed is a clean configuration point, not the
        // default eight-missile loadout with its separate parasite drag.
        f.systems.stores=[false;10];
        for (h, v) in [(11000., atmosphere(11000.).3 * 1.8), (15400., 295.)] {
            let t = trim::solve(&f, h, v, 0., 0.)
                .expect("published broad envelope should be sustainable");
            println!(
                "ENVELOPE h={} speed={} Mach={} power={} residual={:?}",
                h,
                v,
                v / atmosphere(h).3,
                t.throttle,
                t.residual
            );
            assert!(t.throttle <= 1.);
        }
    }
    #[test]
    fn refined_plant_integration_converges() {
        let mut initial = base();
        initial.position = surface_point(0., 0., 3000.);
        initial.velocity = DVec3::NEG_Z * 180.;
        initial.airspeed = 180.;
        let t = trim::solve(&initial, 3000., 180., 0., 0.).unwrap();
        initial.orientation = DQuat::from_rotation_x(t.alpha);
        trim::apply(&mut initial, t);
        let e = Environment {
            wind_enabled: false,
            ..Default::default()
        };
        let mut runs = Vec::new();
        for hz in [240, 480, 960] {
            let mut f = initial.clone();
            let mut peak: f64 = 0.;
            for _ in 0..hz * 5 {
                step(&mut f, &e, 1. / hz as f64);
                peak = peak.max(f.g_load.abs());
            }
            runs.push((f, peak));
        }
        for (f, g) in &runs[..2] {
            let fine = &runs[2];
            assert!((f.velocity - fine.0.velocity).length() / fine.0.velocity.length() < 0.02);
            assert!((g - fine.1).abs() / fine.1 < 0.02);
            assert!((f.position - fine.0.position).length() < 0.02 * 900.);
        }
    }
    #[test]
    fn vacuum_free_rotation_conserves_energy_and_momentum() {
        let mut f = base();
        f.position = surface_point(0., 0., 1e6);
        f.velocity = DVec3::ZERO;
        f.engine = false;
        f.fuel = 0.;
        f.rates = DVec3::new(0.1, 0.08, 0.2);
        let (_, i) = mass::properties(&f);
        let energy = 0.5 * f.rates.dot(i * f.rates);
        let momentum = f.orientation * (i * f.rates);
        let e = Environment {
            wind_enabled: false,
            ..Default::default()
        };
        for _ in 0..2400 {
            step(&mut f, &e, 1. / 240.);
        }
        let final_energy = 0.5 * f.rates.dot(i * f.rates);
        let final_momentum = f.orientation * (i * f.rates);
        assert!((final_energy - energy).abs() / energy < 0.01);
        assert!((final_momentum - momentum).length() / momentum.length() < 0.01);
    }
}
