//! Shared missile sensor, navigation, guidance and acceleration-control model.
//! Coefficients are explicit gameplay assumptions, not specifications of a real missile.
use super::{Missile, Weapon};
use crate::physics::{MU, RADIUS, atmosphere, terrain_height};
use bevy::math::{DQuat, DVec3};
mod dynamics;

#[derive(Clone, Copy)]
pub struct Contact {
    pub position: DVec3,
    pub velocity: DVec3,
    pub forward: DVec3,
    pub throttle: f64,
    pub engine: bool,
    pub alive: bool,
    pub decoy: bool,
    pub strength: f64,
}
#[derive(Clone, Copy)]
pub struct Support {
    pub position: DVec3,
    pub forward: DVec3,
    pub available: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    Separation,
    Supported,
    Active,
    Infrared,
    Memory,
    Ballistic,
    Decoy,
}
impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Separation => "SEPARATION",
            Self::Supported => "RADAR / DATALINK",
            Self::Active => "ACTIVE RADAR",
            Self::Infrared => "IR TRACK",
            Self::Memory => "TRACK MEMORY",
            Self::Ballistic => "NO TRACK",
            Self::Decoy => "DECOY TRACK",
        }
    }
}
#[derive(Clone)]
pub struct Gnc {
    /// Distinct player F-22 loadout; never inferred from the target or camera.
    pub aim9x: bool,
    pub long_range: bool,
    pub mode: Mode,
    pub body: DVec3,
    pub body_rate: DVec3,
    /// Local atmosphere velocity; world velocity is retained for navigation.
    pub wind: DVec3,
    pub restoring_moment: DVec3,
    pub control_moment: DVec3,
    pub attitude: DQuat,
    pub gimbal: DVec3,
    pub confidence: f64,
    pub signal: f64,
    pub reason: &'static str,
    pub active: bool,
    pub estimate: DVec3,
    pub estimated_velocity: DVec3,
    pub initialized: bool,
    pub measured_acceleration: DVec3,
    pub commanded_acceleration: DVec3,
    pub fins: DVec3,
    integral: DVec3,
    pub mass: f64,
    pub propellant: f64,
    pub thrust: f64,
    pub drag: f64,
    pub available_g: f64,
    pub actual_g: f64,
    pub specific_energy: f64,
    pub peak_speed: f64,
    /// Midcourse path demand, for diagnostics (not an altitude teleport).
    pub loft_altitude: f64,
    pub loft_blend: f64,
    pub seed: u64,
    pub evaluated_decoys: Vec<u64>,
}
impl Default for Gnc {
    fn default() -> Self {
        Self {
            aim9x: false,
            long_range: false,
            mode: Mode::Separation,
            body: DVec3::ZERO,
            body_rate: DVec3::ZERO,
            wind: DVec3::ZERO,
            restoring_moment: DVec3::ZERO,
            control_moment: DVec3::ZERO,
            attitude: DQuat::IDENTITY,
            gimbal: DVec3::ZERO,
            confidence: 0.0,
            signal: 0.0,
            reason: "ACQUIRING",
            active: false,
            estimate: DVec3::ZERO,
            estimated_velocity: DVec3::ZERO,
            initialized: false,
            measured_acceleration: DVec3::ZERO,
            commanded_acceleration: DVec3::ZERO,
            fins: DVec3::ZERO,
            integral: DVec3::ZERO,
            mass: 0.0,
            propellant: 0.0,
            thrust: 0.0,
            drag: 0.0,
            available_g: 0.0,
            actual_g: 0.0,
            specific_energy: 0.0,
            peak_speed: 0.0,
            loft_altitude: 0.0,
            loft_blend: 0.0,
            seed: 1,
            evaluated_decoys: Vec::new(),
        }
    }
}
impl Gnc {
    pub fn inherit(mut self, attitude:DQuat, world_rate:DVec3)->Self {
        self.attitude=attitude.normalize();
        self.body=self.attitude*-DVec3::Z;
        self.gimbal=self.body;
        self.body_rate=world_rate;
        self
    }
    pub fn for_aircraft(mut self, aircraft: crate::aircraft::Aircraft, kind: Weapon) -> Self {
        self.aim9x = matches!(aircraft,crate::aircraft::Aircraft::F22|crate::aircraft::Aircraft::Hornet) && kind == Weapon::Heat;
        self
    }
    pub fn launch(velocity: DVec3, target: Contact, seed: u64) -> Self {
        Self {
            attitude:DQuat::from_rotation_arc(-DVec3::Z,velocity.normalize_or(-DVec3::Z)),
            body: velocity.normalize_or_zero(),
            gimbal: velocity.normalize_or_zero(),
            estimate: target.position,
            estimated_velocity: target.velocity,
            initialized: true,
            confidence: 1.0,
            seed,
            ..Self::default()
        }
    }
}
pub fn target(t: &super::Target) -> Contact {
    Contact {
        position: t.position,
        velocity: t.velocity,
        forward: t.orientation * -DVec3::Z,
        throttle: t.throttle,
        engine: t.damage.engine > 0.12 && t.fuel > 0. && t.dynamics.engine,
        alive: t.alive,
        decoy: false,
        strength: t.health.max(0.2),
    }
}
pub fn player(f: &crate::physics::Flight) -> Contact {
    Contact {
        position: f.position,
        velocity: f.velocity,
        forward: f.forward(),
        throttle: f.spool,
        engine: f.engine && f.thrust > 0.0,
        alive: !f.crashed,
        decoy: false,
        strength: f.damage.engine.max(0.1),
    }
}
pub fn heat_signature(contact: Contact, observer: DVec3) -> f64 {
    if contact.decoy {
        return contact.strength;
    }
    let los = (contact.position - observer).normalize_or_zero();
    let rear = contact.forward.dot(los).max(0.0);
    let engine = if contact.engine {
        0.35 + contact.throttle * 0.9 + (contact.throttle - 0.8).max(0.0) * 18.0
    } else {
        0.10
    };
    // All-aspect seeker: warm airframe and engine radiation remain detectable
    // from the nose and beam, with the strongest signature behind the exhaust.
    (0.08 + (0.65 + rear.powi(2) * 0.35) * engine) * contact.strength
}
pub fn terrain_visible(observer: DVec3, point: DVec3) -> bool {
    let key = [
        observer.x.to_bits(),
        observer.y.to_bits(),
        observer.z.to_bits(),
        point.x.to_bits(),
        point.y.to_bits(),
        point.z.to_bits(),
    ];
    LOS_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        // The only late-initialized terrain input is the immutable lake outline.
        let lake = crate::physics::LAKE.get().is_some();
        if cache.lake != lake {
            cache.entries.fill(None);
            cache.lake = lake;
        }
        let hash = key.iter().fold(0xcbf29ce484222325_u64, |h, b| {
            (h ^ b).wrapping_mul(0x100000001b3)
        });
        let index = ((hash ^ (hash >> 32)) as usize) & 255;
        if let Some(entry) = cache.entries[index] {
            if entry.key == key {
                return entry.visible;
            }
        }
        let visible = terrain_visible_uncached(observer, point);
        cache.entries[index] = Some(VisibilityEntry { key, visible });
        visible
    })
}
#[derive(Clone, Copy)]
struct VisibilityEntry {
    key: [u64; 6],
    visible: bool,
}
struct VisibilityCache {
    entries: [Option<VisibilityEntry>; 256],
    lake: bool,
}
thread_local! {
    // Exact endpoints, fixed memory, and no cross-thread synchronization. Never
    // reuse an approximate position or a result from a different terrain state.
    static LOS_CACHE:std::cell::RefCell<VisibilityCache>=const {std::cell::RefCell::new(VisibilityCache{entries:[None;256],lake:false})};
}
fn terrain_visible_uncached(observer: DVec3, point: DVec3) -> bool {
    let delta = point - observer;
    let closest =
        observer + delta * (-observer.dot(delta) / delta.length_squared().max(1.0)).clamp(0.0, 1.0);
    if closest.length() < RADIUS {
        return false;
    }
    // Sample the terrain along the LOS, including endpoints; curved planet uses f64.
    let steps = (delta.length() / 2500.).ceil().clamp(8., 96.) as usize;
    (0..=steps).all(|i| {
        let p = observer + delta * (i as f64 / steps as f64);
        p.length() - RADIUS > terrain_height(p.x, p.z).max(0.0) + 1.0
    })
}
#[cfg(test)]
mod visibility_cache_tests {
    use super::*;
    #[test]
    fn exact_cache_matches_uncached_rays_through_replacement_and_horizon_cases() {
        for step in 0..1500 {
            let a = crate::physics::surface_point(
                100000.,
                step as f64 * 110.,
                50. + (step % 17) as f64 * 800.,
            );
            let b = crate::physics::surface_point(
                120000.,
                -step as f64 * 170.,
                50. + (step % 19) as f64 * 600.,
            );
            let expected = terrain_visible_uncached(a, b);
            assert_eq!(terrain_visible(a, b), expected);
            assert_eq!(terrain_visible(a, b), expected);
            assert_eq!(terrain_visible(b, a), terrain_visible_uncached(b, a));
        }
        LOS_CACHE.with(|c| assert!(c.borrow().entries.iter().flatten().count() <= 256));
    }
}
pub fn radar_notch(observer: DVec3, contact: Contact) -> f64 {
    let los = (contact.position - observer).normalize_or_zero();
    let look_down = los.dot(observer.normalize()) < -0.012;
    if !look_down {
        return 1.0;
    }
    // Ground-relative target velocity: subtracting radar velocity here would
    // put the notch at zero closure instead of at the ground-clutter Doppler.
    let radial = contact.velocity.dot(los).abs();
    (0.04 + 0.96 * ((radial - 12.0) / 33.0).clamp(0.0, 1.0)).clamp(0.04, 1.0)
}
pub fn quality(kind: Weapon, observer: DVec3, contact: Contact, sun: DVec3) -> (f64, &'static str) {
    if !contact.alive {
        return (0.0, "TARGET LOST");
    }
    if !terrain_visible(observer, contact.position) {
        return (0.0, "TERRAIN MASK");
    }
    let r = contact.position - observer;
    let range = r.length().max(1.0);
    let los = r / range;
    if kind == Weapon::Heat {
        let signature = heat_signature(contact, observer);
        let mut flux = signature * (3500.0 / range).powi(2);
        if sun.dot(observer.normalize()) > 0.0 {
            let angle = los.dot(sun).clamp(-1.0, 1.0).acos().to_degrees();
            let glare = ((angle - 3.0) / 9.0).clamp(0.0, 1.0);
            flux *= glare * glare;
            if glare < 0.35 {
                return (flux.clamp(0.0, 1.0), "SUN GLARE");
            }
        }
        (
            flux.clamp(0.0, 1.0),
            if flux < 0.18 {
                "WEAK IR SIGNAL"
            } else {
                "IR SIGNAL"
            },
        )
    } else {
        let notch = radar_notch(observer, contact);
        let strength = (contact.strength * (16000.0 / range).powi(4)).min(1.0) * notch;
        (
            strength.clamp(0.0, 1.0),
            if notch < 0.3 {
                "DOPPLER NOTCH"
            } else {
                "RADAR RETURN"
            },
        )
    }
}
pub fn support_quality(support: Support, contact: Contact, sun: DVec3) -> f64 {
    if !support.available
        || support.position.distance(contact.position) > 20000.0
        || support
            .forward
            .dot((contact.position - support.position).normalize_or_zero())
            < 55.0_f64.to_radians().cos()
    {
        return 0.0;
    }
    quality(Weapon::Radar, support.position, contact, sun).0
}
fn rotate_toward(current: DVec3, desired: DVec3, rate: f64, dt: f64) -> DVec3 {
    let angle = current.dot(desired).clamp(-1.0, 1.0).acos();
    let axis = current.cross(desired);
    if angle < 1e-10 {
        return desired;
    }
    let axis = if axis.length_squared() < 1e-12 {
        current.any_orthonormal_vector()
    } else {
        axis.normalize()
    };
    (DQuat::from_axis_angle(axis, angle.min(rate * dt)) * current).normalize()
}
fn random(seed: u64) -> f64 {
    let mut x = seed.wrapping_add(0x9e3779b97f4a7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    ((x ^ (x >> 31)) >> 11) as f64 / ((1u64 << 53) as f64)
}
pub fn decoy_probability(
    m: &Missile,
    aircraft: Contact,
    decoy: Contact,
    age: f64,
    sun: DVec3,
) -> f64 {
    let aircraft_los = (aircraft.position - m.position).normalize_or_zero();
    let decoy_los = (decoy.position - m.position).normalize_or_zero();
    let separation = aircraft_los.dot(decoy_los).clamp(-1.0, 1.0).acos();
    if m.position.distance(decoy.position) > 4200.0
        || age < 0.08
        || age > 4.0
        || m.gnc.gimbal.dot(decoy_los) < 12.0_f64.to_radians().cos()
        || separation > 10.0_f64.to_radians()
    {
        return 0.0;
    }
    let aircraft_signal = quality(m.kind, m.position, aircraft, sun).0.max(0.01);
    let decoy_signal = quality(m.kind, m.position, decoy, sun).0;
    if decoy_signal < 0.18 {
        return 0.0;
    }
    let ratio = if m.kind == Weapon::Heat {
        heat_signature(decoy, m.position) / (heat_signature(aircraft, m.position).max(0.05))
            * (m.position.distance_squared(aircraft.position)
                / m.position.distance_squared(decoy.position).max(1.0))
    } else {
        decoy_signal / aircraft_signal
    };
    let salience = (ratio / (ratio + 2.0)).clamp(0.0, 1.0);
    let freshness = (1.0 - age / 5.0).max(0.0);
    let separation_factor = (1.0 - separation / 12.0_f64.to_radians()).max(0.0);
    if m.kind == Weapon::Heat {
        // Track continuity rejects a bright object on the same straight path.
        // Breaking across the LOS separates the airframe from its slowing flare.
        let transverse = aircraft.velocity - aircraft_los * aircraft.velocity.dot(aircraft_los);
        let maneuver = (transverse.length() / 120.0).clamp(0.0, 1.0);
        let imaging_rejection = if m.gnc.aim9x { 0.30 } else { 1.0 };
        (((0.30 + 0.48 * maneuver) * salience * freshness * separation_factor)
            * imaging_rejection).clamp(0.0, 0.65)
    } else {
        (0.88 * salience * freshness * separation_factor).clamp(0.0, 0.88)
    }
}
/// Revisit IR track competition as the flare cools. Radar/chaff behavior is unchanged.
pub fn retain_decoy(m: &Missile, aircraft: Contact, decoy: Contact, age: f64, sun: DVec3) -> bool {
    if m.kind != Weapon::Heat {
        return age < 4.0 && decoy.alive;
    }
    if !decoy.alive || age >= 4.0 || quality(m.kind, m.position, decoy, sun).0 < 0.18 {
        return false;
    }
    if age < if m.gnc.aim9x { 0.25 } else { 0.75 } {
        return true;
    }
    let a = (aircraft.position - m.position).normalize_or_zero();
    let d = (decoy.position - m.position).normalize_or_zero();
    let visible = quality(m.kind, m.position, aircraft, sun).0 >= 0.18
        && m.gnc.body.dot(a) > 45.0_f64.to_radians().cos()
        && m.gnc.gimbal.dot(a) > 6.0_f64.to_radians().cos();
    if !visible {
        return true;
    }
    let flux = |c: Contact| {
        heat_signature(c, m.position) / m.position.distance_squared(c.position).max(1.0)
    };
    let transverse = (aircraft.velocity - a * aircraft.velocity.dot(a)).length();
    let unresolved = a.dot(d) > 1.5_f64.to_radians().cos() && transverse < 18.0;
    !(unresolved || flux(aircraft) > flux(decoy) * 0.55)
}
pub fn consider_decoy(
    m: &mut Missile,
    aircraft: Contact,
    decoy: Contact,
    age: f64,
    id: u64,
    sun: DVec3,
) -> bool {
    if m.gnc.evaluated_decoys.contains(&id) {
        return false;
    }
    let probability = decoy_probability(m, aircraft, decoy, age, sun);
    if probability <= 0.0 {
        return false;
    }
    m.gnc.evaluated_decoys.push(id);
    random(m.gnc.seed ^ id.wrapping_mul(0x517cc1b727220a95)) < probability
}
pub fn fly(m: &mut Missile, contact: Contact, support: Support, sun: DVec3, dt: f64) {
    fly_profile(m, contact, support, sun, dt, true);
}

fn smoothstep(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn fly_profile(
    m: &mut Missile,
    contact: Contact,
    support: Support,
    sun: DVec3,
    dt: f64,
    efficient: bool,
) {
    if !m.alive || dt <= 0.0 {
        return;
    }
    m.age += dt;
    let air_velocity = m.velocity - m.gnc.wind;
    let speed = air_velocity.length().max(1.0);
    let velocity_axis = if air_velocity.length_squared() > 1e-8 {
        air_velocity.normalize()
    } else if m.gnc.body.length_squared() > 0.5 {
        m.gnc.body
    } else {
        DVec3::Z
    };
    if m.gnc.body.length_squared() < 0.5 {
        m.gnc.body = velocity_axis;
    }
    if m.gnc.gimbal.length_squared() < 0.5 {
        m.gnc.gimbal = m.gnc.body;
    }
    let r = contact.position - m.position;
    let range = r.length().max(1.0);
    let los = r / range;
    let predicted_range = if m.gnc.initialized {
        (m.gnc.estimate - m.position).length()
    } else {
        f64::INFINITY
    };
    if m.kind == Weapon::Radar
        && !m.gnc.active
        && predicted_range < (if m.gnc.long_range { 18000. } else { 6500. })
        && m.age > 0.5
    {
        m.gnc.active = true;
    }
    let mut supported = m.kind == Weapon::Radar && !m.gnc.active;
    let gimbal_limit = if m.gnc.aim9x {
        80.0_f64 // Estimated; public sources confirm high off-boresight, not limits.
    } else if m.kind == Weapon::Heat {
        45.0_f64
    } else {
        55.0_f64
    }
    .to_radians();
    let gimbal_rate = if m.gnc.aim9x {
        90.0_f64 // Estimated imaging seeker slew, degrees/s.
    } else if m.kind == Weapon::Heat {
        22.0_f64
    } else {
        40.0_f64
    }
    .to_radians();
    let pointing = if m.gnc.initialized {
        (m.gnc.estimate + m.gnc.estimated_velocity * dt - m.position).normalize_or_zero()
    } else {
        m.gnc.body
    };
    let desired = if m.gnc.body.dot(pointing) > gimbal_limit.cos() {
        pointing
    } else {
        m.gnc.body
    };
    m.gnc.gimbal = rotate_toward(m.gnc.gimbal, desired, gimbal_rate, dt);
    let gimbal_blocked = m.gnc.body.dot(los) < gimbal_limit.cos()
        || m.gnc.gimbal.dot(los) < 6.0_f64.to_radians().cos();
    let out_of_range = range > (if m.gnc.aim9x { 12000.0 } else { m.kind.range() * 1.3 });
    let (mut signal, mut reason) = if supported {
        let score = if m.gnc.long_range {
            if support.available
                && support
                    .forward
                    .dot((contact.position - support.position).normalize_or_zero())
                    > 65_f64.to_radians().cos()
            {
                super::bvr::radar_signal(support.position, contact)
            } else {
                0.
            }
        } else {
            support_quality(support, contact, sun)
        };
        (
            score,
            if score < 0.18 {
                "RADAR SUPPORT LOST"
            } else {
                "DATALINK UPDATE"
            },
        )
    } else if gimbal_blocked || out_of_range {
        // Both overrides below zero the signal regardless, so skip the terrain
        // line-of-sight query inside quality(); they also set the reason.
        (0.0, "")
    } else {
        quality(m.kind, m.position, contact, sun)
    };
    if !supported && gimbal_blocked {
        signal = 0.0;
        reason = "SEEKER GIMBAL LIMIT";
    }
    if out_of_range && !supported {
        signal = 0.0;
        reason = "SEEKER RANGE";
    }
    // IR remains the preferred measurement. A valid launcher radar track can
    // bridge seeker range, glare or pointing loss, but never tracks a flare as
    // an aircraft or grants an autonomous radar seeker to an IR missile.
    if m.kind == Weapon::Heat && signal < 0.18 && !contact.decoy && contact.alive {
        let radar = support_quality(support, contact, sun);
        if radar >= 0.18 {
            supported = true;
            signal = radar;
            reason = "IR LOST / RADAR DATALINK";
        }
    }
    if !contact.alive {
        signal = 0.0;
        reason = "TARGET LOST";
    }
    let sees = signal >= 0.18;
    m.gnc.signal = signal;
    m.gnc.reason = reason;
    let predicted = m.gnc.estimate + m.gnc.estimated_velocity * dt;
    if sees {
        m.lost_for = 0.0;
        if !m.gnc.initialized {
            m.gnc.estimate = contact.position;
            m.gnc.estimated_velocity = contact.velocity;
            m.gnc.initialized = true;
        } else {
            // Alpha-beta navigation filter: guidance receives filtered estimates,
            // and coasts this state when the seeker/datalink has no measurement.
            let residual = contact.position - predicted;
            m.gnc.estimate = predicted + residual * (dt * 12.0).min(1.0);
            m.gnc.estimated_velocity += (residual * (dt * 32.0)).clamp_length_max(80.0 * dt);
        }
        m.gnc.confidence = (m.gnc.confidence + dt * 2.0 * signal).min(1.0);
    } else {
        m.gnc.estimate = predicted;
        m.lost_for += dt;
        m.gnc.confidence =
            (m.gnc.confidence - dt * if m.kind == Weapon::Heat { 1.0 } else { 0.5 }).max(0.0);
    }
    m.guided = m.gnc.initialized && m.gnc.confidence > 0.05;
    m.gnc.mode = if m.age < 0.22 {
        Mode::Separation
    } else if !m.guided {
        Mode::Ballistic
    } else if !sees {
        Mode::Memory
    } else if contact.decoy {
        Mode::Decoy
    } else if supported {
        Mode::Supported
    } else if m.kind == Weapon::Heat {
        Mode::Infrared
    } else {
        Mode::Active
    };
    let gravity = -m.position.normalize() * MU / m.position.length_squared();
    let (_, _, rho, sound) = atmosphere(m.position.length() - RADIUS);
    let duration = if m.gnc.long_range {
        24.0
    } else {
        m.kind.burn()
    };
    let burn_elapsed = (m.age - 0.18).clamp(0.0, duration);
    let wet_mass = if m.gnc.long_range {
        190.0
    } else if m.gnc.aim9x {
        84.37 // NAVAIR launch weight, kg.
    } else if m.kind == Weapon::Heat {
        85.0
    } else {
        150.0
    };
    let propellant = if m.gnc.long_range {
        70.0
    } else if m.kind == Weapon::Heat {
        20.0
    } else {
        35.0
    };
    m.gnc.propellant = propellant * (1.0 - burn_elapsed / duration);
    m.gnc.mass = wet_mass - propellant + m.gnc.propellant;
    let burning = m.age > 0.18 && burn_elapsed < duration;
    m.gnc.thrust = if burning {
        if m.gnc.long_range {
            if burn_elapsed < 7. { 27000. } else { 5500. }
        } else if m.kind == Weapon::Heat {
            14000.0
        } else {
            23000.0
        }
    } else {
        0.0
    };
    let area = if m.gnc.aim9x {
        std::f64::consts::PI * (0.127_f64 / 2.0).powi(2)
    } else if m.kind == Weapon::Heat { 0.020 } else { 0.029 };
    let q = 0.5 * rho * speed * speed;
    let aerodynamic_limit = (q * area * 24.0 / m.gnc.mass)
        .min(if m.gnc.aim9x { 40.0 } else if m.kind == Weapon::Heat { 28.0 } else { 24.0 } * 9.81);
    // Tail jet vanes provide low-speed authority only while the motor burns.
    // A 20-degree equivalent deflection and 40-g structural limit are estimates.
    let vector_limit = if m.gnc.aim9x {
        m.gnc.thrust * 20_f64.to_radians().sin() / m.gnc.mass
    } else { 0.0 };
    let aero_limit = (aerodynamic_limit + vector_limit).min(
        if m.gnc.aim9x { 40.0 * 9.81 } else { f64::INFINITY });
    m.gnc.available_g = aero_limit / 9.81;
    let mut command = DVec3::ZERO;
    m.gnc.loft_blend = 0.0;
    m.gnc.loft_altitude = 0.0;
    if m.guided && m.age > 0.22 {
        let estimated_r = m.gnc.estimate - m.position;
        let estimated_range = estimated_r.length().max(1.0);
        let estimated_los = estimated_r / estimated_range;
        let relative = m.gnc.estimated_velocity - m.velocity;
        let closing = (-relative.dot(estimated_los)).max(0.0);
        command = (estimated_r.cross(relative) / estimated_range.powi(2)).cross(velocity_axis)
            * (3.8 * closing);
        if m.age < 0.8 {
            command +=
                (estimated_los - velocity_axis * velocity_axis.dot(estimated_los)) * speed * 1.2;
        }
        command -= gravity - velocity_axis * gravity.dot(velocity_axis);
        if efficient && m.gnc.long_range && !contact.decoy {
            // Only navigation estimates drive the path. Fade into unmodified PN
            // before active acquisition, rather than dropping a bias at one range.
            let up = m.position.normalize();
            let vertical = (up - velocity_axis * up.dot(velocity_axis)).normalize_or_zero();
            let altitude = m.position.length() - RADIUS;
            let target_altitude = m.gnc.estimate.length() - RADIUS;
            let blend = smoothstep((estimated_range - 18000.0) / 22000.0);
            let height = ((estimated_range - 18000.0) * 0.24).clamp(0.0, 14000.0);
            let ceiling = (target_altitude + height).min(26000.0);
            let lookahead = (estimated_range * 0.35).clamp(8000.0, 30000.0);
            let gamma = ((ceiling - altitude) / lookahead).atan().clamp(-0.35, 0.42);
            let desired_climb = speed * gamma.sin();
            // Modest path curvature avoids spending excessive energy on induced
            // drag. At low q, the actual airframe still limits the achieved load.
            let path_acceleration = ((desired_climb - m.velocity.dot(up)) / 5.0).clamp(-18.0, 18.0);
            let gravity_compensation = -gravity.dot(vertical);
            let vertical_command = path_acceleration + gravity_compensation;
            command += vertical * ((vertical_command - command.dot(vertical)) * blend);
            m.gnc.loft_altitude = ceiling;
            m.gnc.loft_blend = blend;
        } else if !efficient && m.gnc.long_range && range > 25000. {
            // Retained exclusively as the matched diagnostic reference.
            let up = m.position.normalize();
            let ceiling = (contact.position.length() - RADIUS + range * 0.09).min(19000.);
            let loft = ((ceiling - (m.position.length() - RADIUS)) * 0.004).clamp(-6., 12.);
            command += (up - velocity_axis * up.dot(velocity_axis)) * loft;
        }
        command = command.clamp_length_max(aero_limit);
    }
    m.gnc.commanded_acceleration = command;
    // Acceleration PI + feed-forward, anti-windup, fin rate and travel limits.
    let error = command - m.gnc.measured_acceleration;
    if m.age <= 0.22 {
        m.gnc.integral = DVec3::ZERO;
        m.gnc.fins = DVec3::ZERO;
    } else if command.length() < aero_limit * 0.95 {
        m.gnc.integral = (m.gnc.integral + error * dt).clamp_length_max(15.0);
    } else {
        m.gnc.integral *= (-2.0 * dt).exp();
    }
    let desired_fin = if m.age <= 0.22 { DVec3::ZERO } else {
        ((command + error * 0.35 + m.gnc.integral * 1.2) / aero_limit.max(1.0))
        .clamp_length_max(1.0) };
    let fin_step = ((desired_fin - m.gnc.fins) * (dt / 0.065).min(1.0)).clamp_length_max(4.0 * dt);
    m.gnc.fins = (m.gnc.fins + fin_step).clamp_length_max(1.0);
    // Normal force follows body incidence, not a direct sideways velocity
    // command. The acceleration controller demands incidence via body torque.
    let fin_normal=m.gnc.fins-velocity_axis*m.gnc.fins.dot(velocity_axis);
    let max_alpha=20.0_f64.to_radians();
    let demand=(velocity_axis+fin_normal*max_alpha.tan()).normalize_or(velocity_axis);
    let angular_error=m.gnc.body.cross(demand);
    let requested_rate=(velocity_axis.cross(m.gnc.measured_acceleration)/speed
        +angular_error*12.).clamp_length_max(4.0);
    dynamics::advance(m, velocity_axis, q, area, speed, aerodynamic_limit, vector_limit, requested_rate, dt);
    let incidence=m.gnc.body-velocity_axis*m.gnc.body.dot(velocity_axis);
    let alpha=m.gnc.body.dot(velocity_axis).clamp(-1.,1.).acos();
    let normal=incidence.normalize_or_zero()*aerodynamic_limit
        *(alpha/max_alpha).min(1.) + fin_normal*vector_limit;
    let normal=normal.clamp_length_max(aero_limit);
    m.gnc.measured_acceleration = normal;
    m.gnc.actual_g = normal.length() / 9.81;
    let mach = speed / sound;
    let wave = 0.20 * (-((mach - 1.1) / 0.35).powi(2)).exp();
    let vector_accel = fin_normal.length()*vector_limit;
    let aerodynamic_accel = (normal.length() - vector_accel).max(0.0);
    let lift_coeff = aerodynamic_accel * m.gnc.mass / (q * area).max(1.0);
    m.gnc.drag = q * area * (0.32 + wave + 0.012 * lift_coeff * lift_coeff + 2.*alpha.sin().powi(2));
    let axial_thrust = (m.gnc.thrust.powi(2) - (vector_accel * m.gnc.mass).powi(2)).max(0.0).sqrt();
    m.velocity += (gravity + normal + m.gnc.body * (axial_thrust / m.gnc.mass)
        - velocity_axis * (m.gnc.drag / m.gnc.mass))
        * dt;
    m.position += m.velocity * dt;
    m.gnc.specific_energy = 0.5 * m.velocity.length_squared() - MU / m.position.length();
    m.gnc.peak_speed = m.gnc.peak_speed.max(m.velocity.length());
    m.trail_clock += dt;
    if burning && m.trail_clock >= 0.075 {
        m.trail_clock = 0.0;
        m.trail.push(m.position);
        if m.trail.len() > 48 {
            m.trail.remove(0);
        }
    }
    if !m.position.is_finite() || !m.velocity.is_finite() {
        m.alive = false;
    }
}

/// Matched deterministic game trajectories: motor, target and controller are
/// identical; only the midcourse path command changes.
fn trajectory_trial(
    range: f64,
    altitude: f64,
    retreat: bool,
    efficient: bool,
    dt: f64,
) -> serde_json::Value {
    let origin = DVec3::Y * (RADIUS + altitude);
    let mut contact = Contact {
        position: (origin + DVec3::Z * range).normalize() * (RADIUS + altitude),
        velocity: DVec3::Z * if retreat { 280.0 } else { -280.0 },
        forward: if retreat { DVec3::Z } else { -DVec3::Z },
        throttle: 0.8,
        engine: true,
        alive: true,
        decoy: false,
        strength: 1.0,
    };
    let velocity = DVec3::Z * 320.0;
    let mut gnc = Gnc::launch(velocity, contact, 1);
    gnc.long_range = true;
    let mut m = Missile {
        gnc,
        decoy: None,
        position: origin,
        velocity,
        kind: Weapon::Radar,
        target: 0,
        age: 0.0,
        alive: true,
        guided: true,
        lost_for: 0.0,
        trail: vec![],
        trail_clock: 0.0,
    };
    let support = Support {
        position: origin,
        forward: DVec3::Z,
        available: true,
    };
    let mut closest = f64::INFINITY;
    let mut encounter_speed = 0.0;
    let mut peak_altitude = altitude;
    let mut drag_loss = 0.0;
    for _ in 0..(210.0 / dt) as usize {
        let previous = contact.position - m.position;
        contact.position += contact.velocity * dt;
        contact.position = contact.position.normalize() * (RADIUS + altitude);
        let up = contact.position.normalize();
        contact.velocity = (contact.velocity - up * contact.velocity.dot(up)).normalize() * 280.0;
        fly_profile(&mut m, contact, support, -DVec3::Y, dt, efficient);
        let distance = super::swept_distance(previous, contact.position - m.position);
        if distance < closest {
            closest = distance;
            encounter_speed = m.velocity.length();
        }
        peak_altitude = peak_altitude.max(m.position.length() - RADIUS);
        drag_loss += m.gnc.drag / m.gnc.mass * m.velocity.length() * dt;
        if closest < 12.0 || m.position.length() < RADIUS || !m.alive {
            break;
        }
    }
    serde_json::json!({"range_km":range / 1000.0,"launch_altitude_m":altitude,
        "retreating":retreat,"profile":if efficient {"smooth loft"} else {"previous bias"},
        "hit":closest < 12.0,"closest_m":closest,"encounter_speed_mps":encounter_speed,
        "peak_altitude_m":peak_altitude,"drag_loss_J_kg":drag_loss,"seconds":m.age})
}

/// Release transient through the production GNC path, including inherited
/// attitude/rates, wind, motor delay, natural stability and steering handover.
pub fn release_study() {
    let mut cases = Vec::new();
    for (kind, aim9x, label) in [(Weapon::Heat, false, "IR"), (Weapon::Heat, true, "AIM-9X"), (Weapon::Radar, false, "Radar")] {
        for altitude in [3000., 12000.] {
            for alpha in [0_f64, 20., 45.] {
                let position = DVec3::Y * (RADIUS + altitude);
                let mut contact = Contact { position: position - DVec3::Z * 5000., velocity: -DVec3::Z * 230.,
                    forward: -DVec3::Z, throttle: 0.8, engine: true, alive: true, decoy: false, strength: 1. };
                let wind = DVec3::new(12., 0., 0.);
                let velocity = -DVec3::Z * 250. + wind;
                let attitude = DQuat::from_rotation_x(alpha.to_radians());
                let mut gnc = Gnc::launch(velocity, contact, 1).inherit(attitude, DVec3::ZERO);
                gnc.aim9x = aim9x;
                gnc.wind = wind;
                let mut m = Missile { gnc, decoy: None, position, velocity, kind, target: 0,
                    age: 0., alive: true, guided: true, lost_for: 0., trail: vec![], trail_clock: 0. };
                let support = Support { position, forward: -DVec3::Z, available: true };
                let mut samples = Vec::new();
                for frame in 0..=240 {
                    if frame % 2 == 0 {
                        samples.push(serde_json::json!({"t":m.age,
                            "incidence_deg":m.gnc.body.angle_between(m.velocity - wind).to_degrees(),
                            "body_rate_deg_s":m.gnc.body_rate.length().to_degrees(),
                            "restoring_moment_Nm":m.gnc.restoring_moment.length(),
                            "control_moment_Nm":m.gnc.control_moment.length(),
                            "fin":m.gnc.fins.length(),"g":m.gnc.actual_g,
                            "mode":m.gnc.mode.label(),"speed":(m.velocity-wind).length()}));
                    }
                    if frame == 240 { break; }
                    contact.position += contact.velocity / 240.;
                    fly(&mut m, contact, support, -DVec3::X, 1. / 240.);
                }
                cases.push(serde_json::json!({"weapon":label,"altitude_m":altitude,"release_alpha_deg":alpha,"samples":samples}));
            }
        }
    }
    println!("{}", serde_json::to_string_pretty(&cases).unwrap());
}

/// Repeatable end-to-end engagements, without the renderer or AI controller.
pub fn study() {
    let mut runs = Vec::new();
    for name in [
        "ir-dry",
        "ir-afterburner",
        "ir-sun",
        "radar-supported",
        "radar-support-loss",
        "radar-notch",
        "radar-look-up",
    ] {
        let kind = if name.starts_with("ir-") {
            Weapon::Heat
        } else {
            Weapon::Radar
        };
        let origin = DVec3::Y * (RADIUS + 4000.0);
        let range = if kind == Weapon::Heat {
            5000.0
        } else {
            11000.0
        };
        let mut contact = Contact {
            position: origin + DVec3::Z * range,
            velocity: DVec3::Z * 220.0,
            forward: DVec3::Z,
            throttle: if name == "ir-afterburner" { 1.0 } else { 0.65 },
            engine: true,
            alive: true,
            decoy: false,
            strength: 1.0,
        };
        let mut support = Support {
            position: origin,
            forward: DVec3::Z,
            available: true,
        };
        let mut missile = Missile {
            gnc: Gnc::launch(DVec3::Z * 260.0, contact, 1),
            decoy: None,
            position: origin,
            velocity: DVec3::Z * 260.0,
            kind,
            target: 0,
            age: 0.0,
            alive: true,
            guided: true,
            lost_for: 0.0,
            trail: vec![],
            trail_clock: 0.0,
        };
        if name == "ir-sun" {
            contact.position += DVec3::Y * 500.0;
            missile.gnc.estimate = contact.position;
        }
        if name == "radar-notch" || name == "radar-look-up" {
            contact.position = origin
                + DVec3::Z * 4000.0
                + DVec3::Y * if name == "radar-notch" { -500.0 } else { 500.0 };
            contact.velocity = DVec3::X * 250.0;
            missile.gnc = Gnc::launch(missile.velocity, contact, 1);
        }
        let mut closest = f64::INFINITY;
        let mut peak_g = 0.0_f64;
        let mut memory_seconds = 0.0;
        let mut ir_datalink_seconds = 0.0;
        let mut burnout_speed = 0.0;
        let mut hit = false;
        let sun = if name == "ir-sun" {
            (contact.position - missile.position).normalize()
        } else {
            -DVec3::Y
        };
        for _ in 0..7200 {
            let dt = 1.0 / 120.0;
            if name == "radar-support-loss" && missile.age > 0.8 {
                support.available = false;
                contact.velocity = DVec3::new(180.0, 0.0, 180.0);
            }
            if name == "radar-notch" {
                let horizontal = (contact.position - missile.position)
                    .normalize()
                    .cross(contact.position.normalize())
                    .normalize();
                contact.velocity = horizontal * 250.0;
            }
            let previous = contact.position - missile.position;
            contact.position += contact.velocity * dt;
            support.position += DVec3::Z * 230.0 * dt;
            fly(&mut missile, contact, support, sun, dt);
            let separation = super::swept_distance(previous, contact.position - missile.position);
            closest = closest.min(separation);
            peak_g = peak_g.max(missile.gnc.actual_g);
            if missile.gnc.mode == Mode::Memory {
                memory_seconds += dt;
            }
            if kind == Weapon::Heat && missile.gnc.mode == Mode::Supported {
                ir_datalink_seconds += dt;
            }
            if burnout_speed == 0.0 && missile.age > kind.burn() + 0.18 {
                burnout_speed = missile.velocity.length();
            }
            if separation < kind.fuse() && missile.age > 0.4 {
                hit = true;
                break;
            }
            if missile.position.length() - RADIUS
                < terrain_height(missile.position.x, missile.position.z).max(0.0)
                || !missile.alive
            {
                break;
            }
        }
        assert!(missile.position.is_finite() && missile.velocity.is_finite());
        runs.push(serde_json::json!({"case":name,"hit":hit,"closest_m":closest,"seconds":missile.age,
            "peak_speed_m_s":missile.gnc.peak_speed,"final_speed_m_s":missile.velocity.length(),"burnout_speed_m_s":burnout_speed,
            "peak_g":peak_g,"memory_seconds":memory_seconds,"ir_datalink_seconds":ir_datalink_seconds,"active":missile.gnc.active,"final_mode":missile.gnc.mode.label(),
            "propellant_kg":missile.gnc.propellant,"specific_energy_J_kg":missile.gnc.specific_energy}));
    }
    let mut countermeasures = Vec::new();
    for throttle in [0.65, 1.0] {
        let position = DVec3::Y * (RADIUS + 3000.0);
        let aircraft = Contact {
            position: position + DVec3::Z * 1800.0,
            velocity: DVec3::Z * 230.0,
            forward: DVec3::Z,
            throttle,
            engine: true,
            alive: true,
            decoy: false,
            strength: 1.0,
        };
        let decoy = Contact {
            position: aircraft.position - DVec3::Z * 25.0,
            decoy: true,
            strength: 8.0,
            ..aircraft
        };
        let mut diversions = 0;
        let mut probability = 0.0;
        for seed in 1..=200 {
            let velocity = DVec3::Z * 450.0;
            let mut m = Missile {
                gnc: Gnc::launch(velocity, aircraft, seed),
                decoy: None,
                position,
                velocity,
                kind: Weapon::Heat,
                target: 0,
                age: 1.0,
                alive: true,
                guided: true,
                lost_for: 0.0,
                trail: vec![],
                trail_clock: 0.0,
            };
            probability = decoy_probability(&m, aircraft, decoy, 0.3, -DVec3::Y);
            diversions += usize::from(consider_decoy(&mut m, aircraft, decoy, 0.3, 9, -DVec3::Y));
        }
        countermeasures.push(serde_json::json!({"throttle":throttle,"trials":200,"diversions":diversions,"probability":probability}));
    }
    let mut evasion = Vec::new();
    for (name, flares, turn) in [
        ("straight-no-flares", false, false),
        ("straight-flares", true, false),
        ("turn-only", false, true),
        ("flare-and-turn", true, true),
    ] {
        let mut hits = 0;
        let mut diversions = 0;
        let mut reacquired = 0;
        for seed in 1..=100 {
            let outcome = ir_evasion_trial(seed, flares, turn);
            hits += usize::from(outcome.0);
            diversions += outcome.1;
            reacquired += outcome.2;
        }
        evasion.push(serde_json::json!({"case":name,"trials":100,"hits":hits,"diversions":diversions,"reacquisitions":reacquired}));
    }
    let mut trajectories = Vec::new();
    for altitude in [3000.0, 11000.0] {
        for range in [40000.0, 70000.0, 100000.0] {
            for retreat in [false, true] {
                for efficient in [false, true] {
                    trajectories.push(trajectory_trial(
                        range,
                        altitude,
                        retreat,
                        efficient,
                        1.0 / 120.0,
                    ));
                }
            }
        }
    }
    let report = serde_json::json!({"description":"Shared missile GNC deterministic engagements; game model, no real weapon validation","runs":runs,"flare_trials":countermeasures,"ir_evasion_trials":evasion,"trajectory_comparisons":trajectories});
    std::fs::create_dir_all("tests").unwrap();
    std::fs::write(
        "tests/missile-gnc-study.json",
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}

/// Controlled rear-aspect engagement with three finite flare bursts and a 3 g
/// transverse break. Calls the same selection, retention and flight functions as gameplay.
fn ir_evasion_trial(seed: u64, flares: bool, turn: bool) -> (bool, usize, usize) {
    ir_evasion_trial_dt(seed, flares, turn, 1.0 / 120.0)
}
fn ir_evasion_trial_dt(seed: u64, flares: bool, turn: bool, dt: f64) -> (bool, usize, usize) {
    let origin = DVec3::Y * (RADIUS + 3000.0);
    let mut aircraft = Contact {
        position: origin + DVec3::Z * 1800.0,
        velocity: DVec3::Z * 230.0,
        forward: DVec3::Z,
        throttle: 0.65,
        engine: true,
        alive: true,
        decoy: false,
        strength: 1.0,
    };
    let mut m = Missile {
        gnc: Gnc::launch(DVec3::Z * 400.0, aircraft, seed),
        decoy: None,
        position: origin,
        velocity: DVec3::Z * 400.0,
        kind: Weapon::Heat,
        target: 0,
        age: 1.0,
        alive: true,
        guided: true,
        lost_for: 0.0,
        trail: vec![],
        trail_clock: 0.0,
    };
    let support = Support {
        position: origin,
        forward: DVec3::Z,
        available: false,
    };
    let mut decoys: Vec<super::dogfight::Decoy> = Vec::new();
    let mut bursts = 0;
    let mut diverted = 0;
    let mut returns = 0;
    for step in 0..(15.0 / dt).ceil() as usize {
        let t = step as f64 * dt;
        let old_aircraft = aircraft.position;
        let old_missile = m.position;
        if turn && t >= 1.0 {
            aircraft.velocity = DQuat::from_rotation_y(3.0 * 9.81 / 230.0 * dt) * aircraft.velocity;
            aircraft.forward = aircraft.velocity.normalize();
        }
        aircraft.position += aircraft.velocity * dt;
        if flares && bursts < 3 && t >= 1.0 + bursts as f64 * 0.65 {
            decoys.push(super::dogfight::Decoy {
                owner: None,
                id: bursts + 1,
                kind: Weapon::Heat,
                position: aircraft.position,
                velocity: aircraft.velocity - aircraft.forward * 35.0 - DVec3::Y * 12.0,
                age: 0.0,
            });
            bursts += 1;
        }
        for d in &mut decoys {
            d.age += dt;
            d.velocity *= (-0.32 * dt).exp();
            d.velocity -= d.position.normalize() * 9.81 * dt;
            d.position += d.velocity * dt;
        }
        if m.decoy.is_none() {
            for d in &decoys {
                if consider_decoy(
                    &mut m,
                    aircraft,
                    super::dogfight::decoy_contact(d),
                    d.age,
                    d.id,
                    -DVec3::Y,
                ) {
                    m.decoy = Some(d.id);
                    diverted += 1;
                    break;
                }
            }
        }
        if m.decoy.is_some_and(|id| {
            !decoys.iter().any(|d| {
                d.id == id
                    && retain_decoy(
                        &m,
                        aircraft,
                        super::dogfight::decoy_contact(d),
                        d.age,
                        -DVec3::Y,
                    )
            })
        }) {
            m.decoy = None;
            returns += 1;
        }
        let tracked = m
            .decoy
            .and_then(|id| decoys.iter().find(|d| d.id == id))
            .map_or(aircraft, super::dogfight::decoy_contact);
        fly(&mut m, tracked, support, -DVec3::Y, dt);
        let nearest = super::damage::closest_relative(
            old_missile - old_aircraft,
            m.position - aircraft.position,
        )
        .0;
        if nearest.length() < m.kind.fuse() {
            return (true, diverted, returns);
        }
        if m.position.length() - RADIUS < 0.0 {
            break;
        }
    }
    (false, diverted, returns)
}

#[cfg(test)]
mod tests {
    #[test]
    fn separation_has_finite_weathercocking_and_no_hidden_alignment_controller() {
        for kind in [Weapon::Heat, Weapon::Radar] {
            let (mut m, c, s) = scene(kind, 2000.);
            let launch_axis = m.velocity.normalize();
            let attitude = DQuat::from_rotation_arc(-DVec3::Z, launch_axis)
                * DQuat::from_rotation_x(20_f64.to_radians());
            m.gnc = m.gnc.inherit(attitude, DVec3::ZERO);
            let initial = m.gnc.body.angle_between(launch_axis);
            for _ in 0..12 { fly(&mut m, c, s, -DVec3::Y, 1. / 120.); }
            let remaining = m.gnc.body.angle_between(m.velocity);
            assert!(remaining > initial * 0.65, "{kind:?}: {remaining} from {initial}");
            assert_eq!(m.gnc.fins, DVec3::ZERO);
            assert_eq!(m.gnc.control_moment, DVec3::ZERO);
            assert!(m.gnc.body_rate.is_finite());
            assert!(m.gnc.restoring_moment.length() > 0.);
        }
    }
    #[test]
    fn missile_uses_air_relative_velocity_and_retains_inherited_roll() {
        let (mut m, c, s) = scene(Weapon::Heat, 2000.);
        let q = DQuat::from_rotation_arc(-DVec3::Z, m.velocity.normalize());
        m.gnc = m.gnc.inherit(q, m.velocity.normalize() * 0.4);
        m.gnc.wind = DVec3::X * 40.;
        let before = m.gnc.attitude;
        fly(&mut m, c, s, -DVec3::Y, 0.001);
        assert!(m.gnc.actual_g > 0.);
        assert!(m.gnc.attitude != before && m.gnc.body_rate.is_finite());
        assert!(m.gnc.body_rate.dot(m.velocity.normalize()) > 0.35);
    }
    #[test]
    fn release_preserves_attitude_rates_and_incidence_generates_load() {
        let (mut m,c,s)=scene(Weapon::Heat,2000.);
        let attitude=DQuat::from_rotation_arc(-DVec3::Z,m.velocity.normalize())*DQuat::from_rotation_x(0.12);
        let rate=DVec3::new(0.2,-0.1,0.3);
        m.gnc=m.gnc.inherit(attitude,rate);
        assert!(m.gnc.body.distance(attitude*-DVec3::Z)<1e-12);
        assert_eq!(m.gnc.body_rate,rate);
        assert!(m.gnc.body.angle_between(m.velocity)>0.1);
        let before=m.gnc.body;
        fly(&mut m,c,s,-DVec3::Y,0.001);
        assert!(m.gnc.body.angle_between(before)<0.002);
        assert!(m.gnc.actual_g>0.01);
        assert!(m.gnc.body.angle_between(m.velocity)>0.05);
        assert!((m.gnc.attitude*-DVec3::Z).distance(m.gnc.body)<1e-10);
    }
    use super::*;
    #[test]
    fn aim9x_is_used_by_f22_and_hornet_infrared_loadouts() {
        use crate::aircraft::Aircraft;
        for a in Aircraft::ALL {
            for kind in [Weapon::Heat, Weapon::Radar] {
                let g = Gnc::default().for_aircraft(a, kind);
                assert_eq!(g.aim9x, matches!(a,Aircraft::F22|Aircraft::Hornet) && kind == Weapon::Heat);
            }
        }
    }
    #[test]
    fn aim9x_has_published_mass_and_motor_limited_vector_authority() {
        let (mut m, c, s) = scene(Weapon::Heat, 1800.0);
        m.gnc.aim9x = true;
        m.velocity = DVec3::Z * 60.0;
        m.age = 0.17;
        fly(&mut m, c, s, -DVec3::Y, 0.001);
        assert!((m.gnc.mass - 84.37).abs() < 1e-10);
        let mut coast = m.clone();
        coast.age = 8.0;
        m.age = 0.3;
        fly(&mut m, c, s, -DVec3::Y, 0.001);
        fly(&mut coast, c, s, -DVec3::Y, 0.001);
        assert!(m.gnc.thrust > 0.0);
        assert_eq!(coast.gnc.thrust, 0.0);
        assert!(m.gnc.available_g > coast.gnc.available_g + 3.0);
        assert!(m.gnc.actual_g <= m.gnc.available_g + 1e-9);
    }
    #[test]
    fn aim9x_tracks_high_off_boresight_without_radar_support() {
        let (mut m, mut c, mut s) = scene(Weapon::Heat, 1800.0);
        let direction = DVec3::new(60_f64.to_radians().sin(), 0.0, 60_f64.to_radians().cos());
        c.position = m.position + direction * 1800.0;
        m.gnc.estimate = c.position;
        m.gnc.gimbal = direction;
        m.age = 1.0;
        s.available = false;
        let mut generic = m.clone();
        m.gnc.aim9x = true;
        fly(&mut m, c, s, -DVec3::Y, 0.01);
        fly(&mut generic, c, s, -DVec3::Y, 0.01);
        assert_eq!(m.gnc.mode, Mode::Infrared);
        assert_eq!(generic.gnc.reason, "SEEKER GIMBAL LIMIT");
    }
    #[test]
    fn aim9x_rejects_more_flares_without_becoming_immune() {
        let (mut m, c, _) = scene(Weapon::Heat, 2000.0);
        let decoy = Contact { position: c.position - DVec3::Z * 20.0,
            strength: 8.0, decoy: true, ..c };
        m.gnc.gimbal = DVec3::Z;
        let generic = decoy_probability(&m, c, decoy, 0.3, -DVec3::Y);
        m.gnc.aim9x = true;
        let imaging = decoy_probability(&m, c, decoy, 0.3, -DVec3::Y);
        assert!(imaging > 0.0 && imaging < generic);
        assert!((imaging / generic - 0.30).abs() < 1e-10);
    }
    #[test]
    fn infrared_uses_radar_datalink_during_glare_then_returns_to_its_seeker() {
        let (mut m, mut c, s) = scene(Weapon::Heat, 4000.0);
        m.age = 1.0;
        let sun = ((c.position - m.position).normalize() + DVec3::Y * 0.01).normalize();
        assert_eq!(quality(Weapon::Heat, m.position, c, sun).0, 0.0);
        c.position += DVec3::X * 10.0;
        fly(&mut m, c, s, sun, 0.01);
        assert_eq!(m.gnc.mode, Mode::Supported);
        assert_eq!(m.gnc.reason, "IR LOST / RADAR DATALINK");
        assert!(m.guided && m.lost_for == 0.0);
        assert!(m.gnc.estimate.x > 0.0);
        assert!(!m.gnc.active);
        fly(&mut m, c, s, -DVec3::Y, 0.01);
        assert_eq!(m.gnc.mode, Mode::Infrared);
    }

    #[test]
    fn infrared_datalink_bridges_seeker_range_and_gimbal_limits() {
        let (mut m, c, s) = scene(Weapon::Heat, 10000.0);
        m.age = 1.0;
        m.gnc.gimbal = DVec3::X;
        fly(&mut m, c, s, -DVec3::Y, 0.01);
        assert_eq!(m.gnc.mode, Mode::Supported);
        assert!(!m.gnc.active);
        assert!(m.gnc.actual_g <= m.gnc.available_g + 1e-9);
    }

    #[test]
    fn infrared_support_loss_coasts_without_reading_unseen_target() {
        let (mut m, c, mut s) = scene(Weapon::Heat, 10000.0);
        m.age = 1.0;
        fly(&mut m, c, s, -DVec3::Y, 0.01);
        assert_eq!(m.gnc.mode, Mode::Supported);
        s.available = false;
        let mut unseen = c;
        unseen.position += DVec3::X * 3000.0;
        unseen.velocity = DVec3::X * 700.0;
        let predicted = m.gnc.estimate + m.gnc.estimated_velocity * 0.01;
        fly(&mut m, unseen, s, -DVec3::Y, 0.01);
        assert_eq!(m.gnc.mode, Mode::Memory);
        assert_eq!(m.gnc.estimate, predicted);
        for _ in 0..110 {
            fly(&mut m, unseen, s, -DVec3::Y, 0.01);
        }
        assert_eq!(m.gnc.mode, Mode::Ballistic);
    }

    #[test]
    fn infrared_radar_fallback_requires_valid_geometry_and_does_not_support_decoys() {
        let (base, c, s) = scene(Weapon::Heat, 4000.0);
        let sun = ((c.position - base.position).normalize() + DVec3::Y * 0.01).normalize();
        for bad in [
            Support {
                available: false,
                ..s
            },
            Support {
                forward: -s.forward,
                ..s
            },
        ] {
            let mut m = base.clone();
            m.age = 1.0;
            fly(&mut m, c, bad, sun, 0.01);
            assert_eq!(m.gnc.mode, Mode::Memory);
        }
        for unavailable in [
            Contact {
                position: base.position + DVec3::Z * 21000.0,
                ..c
            },
            Contact {
                position: -base.position,
                ..c
            },
            Contact {
                position: base.position + DVec3::Z * 10000.0 - DVec3::Y * 500.0,
                velocity: DVec3::X * 250.0,
                ..c
            },
        ] {
            let mut m = base.clone();
            m.age = 1.0;
            fly(&mut m, unavailable, s, sun, 0.01);
            assert_ne!(m.gnc.mode, Mode::Supported);
        }
        let mut m = base.clone();
        m.age = 1.0;
        fly(&mut m, Contact { decoy: true, ..c }, s, sun, 0.01);
        assert_ne!(m.gnc.mode, Mode::Supported);
        let mut m = base;
        m.age = 1.0;
        fly(&mut m, Contact { alive: false, ..c }, s, sun, 0.01);
        assert_ne!(m.gnc.mode, Mode::Supported);
    }
    #[test]
    fn loft_blends_smoothly_and_leaves_short_range_weapons_unchanged() {
        assert_eq!(smoothstep(-1.0), 0.0);
        assert_eq!(smoothstep(2.0), 1.0);
        assert!((smoothstep(0.5) - 0.5).abs() < 1e-12);
        for kind in [Weapon::Heat, Weapon::Radar] {
            let (mut new, c, s) = scene(kind, 4000.0);
            let mut old = new.clone();
            for _ in 0..300 {
                fly_profile(&mut new, c, s, -DVec3::Y, 0.01, true);
                fly_profile(&mut old, c, s, -DVec3::Y, 0.01, false);
            }
            assert_eq!(new.position, old.position);
            assert_eq!(new.velocity, old.velocity);
        }
    }

    #[test]
    fn loft_memory_uses_estimate_instead_of_unseen_target_altitude() {
        let (mut a, c, mut s) = scene(Weapon::Radar, 70000.0);
        a.gnc.long_range = true;
        a.age = 1.0;
        let mut b = a.clone();
        let mut unseen = c;
        unseen.position += DVec3::Y * 15000.0;
        s.available = false;
        fly(&mut a, c, s, -DVec3::Y, 0.01);
        fly(&mut b, unseen, s, -DVec3::Y, 0.01);
        assert_eq!(a.gnc.loft_altitude, b.gnc.loft_altitude);
        assert_eq!(a.velocity, b.velocity);
        assert!(a.gnc.actual_g <= a.gnc.available_g + 1e-9);
    }

    #[test]
    fn long_range_loft_saves_drag_energy_and_preserves_terminal_intercept() {
        for altitude in [3000.0, 11000.0] {
            let old = trajectory_trial(100000.0, altitude, false, false, 1.0 / 120.0);
            let new = trajectory_trial(100000.0, altitude, false, true, 1.0 / 120.0);
            assert_eq!(new["hit"], true, "{new}");
            assert!(
                new["encounter_speed_mps"].as_f64().unwrap()
                    > old["encounter_speed_mps"].as_f64().unwrap() * 1.2
            );
            assert!(
                new["drag_loss_J_kg"].as_f64().unwrap()
                    < old["drag_loss_J_kg"].as_f64().unwrap() * 0.95
            );
            assert!(new["peak_altitude_m"].as_f64().unwrap() < 26500.0);
        }
    }

    #[test]
    fn loft_intercept_is_stable_when_timestep_is_halved() {
        let coarse = trajectory_trial(70000.0, 11000.0, true, true, 1.0 / 120.0);
        let fine = trajectory_trial(70000.0, 11000.0, true, true, 1.0 / 240.0);
        assert_eq!(coarse["hit"], true);
        assert_eq!(fine["hit"], true);
        for key in ["seconds", "encounter_speed_mps", "peak_altitude_m"] {
            let a = coarse[key].as_f64().unwrap();
            let b = fine[key].as_f64().unwrap();
            assert!((a - b).abs() < a.abs() * 0.01, "{key}: {a} vs {b}");
        }
    }

    #[test]
    fn long_range_terminal_guidance_matches_unlofted_pn() {
        let (mut new, c, s) = scene(Weapon::Radar, 15000.0);
        new.gnc.long_range = true;
        let mut old = new.clone();
        fly_profile(&mut new, c, s, -DVec3::Y, 0.01, true);
        fly_profile(&mut old, c, s, -DVec3::Y, 0.01, false);
        assert_eq!(new.gnc.loft_blend, 0.0);
        assert_eq!(new.velocity, old.velocity);
    }
    #[test]
    fn flares_and_transverse_break_outperform_flare_only_and_turn_only() {
        let mut straight_hits = 0;
        let mut flare_hits = 0;
        let mut turn_hits = 0;
        let mut combined_hits = 0;
        let mut finer_hits = 0;
        let mut returns = 0;
        for seed in 1..=64 {
            straight_hits += usize::from(ir_evasion_trial(seed, false, false).0);
            let flare = ir_evasion_trial(seed, true, false);
            flare_hits += usize::from(flare.0);
            returns += flare.2;
            turn_hits += usize::from(ir_evasion_trial(seed, false, true).0);
            combined_hits += usize::from(ir_evasion_trial(seed, true, true).0);
            finer_hits += usize::from(ir_evasion_trial_dt(seed, true, true, 1.0 / 240.0).0);
        }
        assert!(straight_hits >= 60 && flare_hits >= 48 && turn_hits >= 48);
        assert!(combined_hits + 16 < flare_hits && combined_hits + 16 < turn_hits);
        assert!(combined_hits > 0 && returns > 0);
        assert!(
            combined_hits.abs_diff(finer_hits) <= 8,
            "coarse={combined_hits} fine={finer_hits}"
        );
    }
    #[test]
    fn ir_reacquisition_requires_detectable_aircraft_and_rejects_cooling_flares() {
        let (mut m, c, _) = scene(Weapon::Heat, 1800.0);
        let decoy = Contact {
            position: c.position - DVec3::Z * 30.0,
            decoy: true,
            strength: 8.0,
            ..c
        };
        assert!(retain_decoy(&m, c, decoy, 0.4, -DVec3::Y));
        assert!(!retain_decoy(&m, c, decoy, 0.9, -DVec3::Y));
        m.gnc.gimbal = DVec3::X;
        assert!(retain_decoy(&m, c, decoy, 0.9, -DVec3::Y));
        assert!(!retain_decoy(&m, c, decoy, 4.1, -DVec3::Y));
        let (radar, _, _) = scene(Weapon::Radar, 1800.0);
        assert!(retain_decoy(&radar, c, decoy, 0.9, -DVec3::Y));
    }
    fn scene(kind: Weapon, range: f64) -> (Missile, Contact, Support) {
        let position = DVec3::Y * (RADIUS + 3000.0);
        let contact = Contact {
            position: position + DVec3::Z * range,
            velocity: DVec3::Z * 230.0,
            forward: DVec3::Z,
            throttle: 0.65,
            engine: true,
            alive: true,
            decoy: false,
            strength: 1.0,
        };
        let velocity = DVec3::Z * 400.0;
        let missile = Missile {
            gnc: Gnc::launch(velocity, contact, 1),
            decoy: None,
            position,
            velocity,
            kind,
            target: 0,
            age: 0.0,
            alive: true,
            guided: true,
            lost_for: 0.0,
            trail: vec![],
            trail_clock: 0.0,
        };
        let support = Support {
            position,
            forward: DVec3::Z,
            available: true,
        };
        (missile, contact, support)
    }
    #[test]
    fn afterburner_increases_ir_signal_and_reduces_flare_salience() {
        let (mut m, mut c, _) = scene(Weapon::Heat, 4000.0);
        let dry = quality(Weapon::Heat, m.position, c, -DVec3::Y).0;
        let decoy = Contact {
            position: c.position - DVec3::Z * 20.0,
            strength: 8.0,
            decoy: true,
            ..c
        };
        m.gnc.gimbal = DVec3::Z;
        let dry_flare = decoy_probability(&m, c, decoy, 0.3, -DVec3::Y);
        c.throttle = 1.0;
        assert!(quality(Weapon::Heat, m.position, c, -DVec3::Y).0 > dry);
        assert!(decoy_probability(&m, c, decoy, 0.3, -DVec3::Y) < dry_flare);
        c.engine = false;
        assert!(heat_signature(c, m.position) < 0.3);
    }
    #[test]
    fn sun_glare_limits_ir_but_all_aspects_can_acquire() {
        let (m, mut c, _) = scene(Weapon::Heat, 2000.0);
        c.position += DVec3::Y * 200.0;
        let sun = (c.position - m.position).normalize();
        assert_eq!(quality(Weapon::Heat, m.position, c, sun).1, "SUN GLARE");
        assert!(quality(Weapon::Heat, m.position, c, sun).0 < 0.18);
        assert!(quality(Weapon::Radar, m.position, c, sun).0 > 0.18);
        c.forward = -DVec3::Z;
        assert!(quality(Weapon::Heat, m.position, c, -DVec3::Y).0 > 0.18);
        c.forward = DVec3::X;
        assert!(quality(Weapon::Heat, m.position, c, -DVec3::Y).0 > 0.18);
    }
    #[test]
    fn notch_uses_ground_relative_velocity_and_only_look_down() {
        let (m, mut c, _) = scene(Weapon::Radar, 3000.0);
        c.position -= DVec3::Y * 800.0;
        c.velocity = DVec3::X * 250.0;
        assert!(radar_notch(m.position, c) < 0.18);
        c.velocity = DVec3::Z * 250.0;
        assert!(radar_notch(m.position, c) > 0.9);
        c.position += DVec3::Y * 1600.0;
        c.velocity = DVec3::X * 250.0;
        assert_eq!(radar_notch(m.position, c), 1.0);
    }
    #[test]
    fn terrain_masks_both_seeker_types() {
        let (m, mut c, _) = scene(Weapon::Radar, 3000.0);
        c.position = -m.position;
        assert!(!terrain_visible(m.position, c.position));
        for kind in [Weapon::Heat, Weapon::Radar] {
            assert_eq!(quality(kind, m.position, c, -DVec3::Y).1, "TERRAIN MASK");
        }
    }
    #[test]
    fn datalink_loss_coasts_estimate_without_reading_target_motion_then_goes_ballistic() {
        let (mut m, mut c, mut s) = scene(Weapon::Radar, 12000.0);
        fly(&mut m, c, s, -DVec3::Y, 0.01);
        assert_eq!(m.gnc.mode, Mode::Separation);
        s.available = false;
        c.position += DVec3::X * 5000.0;
        c.velocity = DVec3::X * 1000.0;
        for _ in 0..70 {
            fly(&mut m, c, s, -DVec3::Y, 0.01);
        }
        assert_eq!(m.gnc.mode, Mode::Memory);
        assert!(m.gnc.estimate.x.abs() < 1.0);
        for _ in 0..160 {
            fly(&mut m, c, s, -DVec3::Y, 0.01);
        }
        assert_eq!(m.gnc.mode, Mode::Ballistic);
        assert!(!m.guided);
    }
    #[test]
    fn active_handoff_no_longer_requires_launch_aircraft() {
        let (mut m, c, mut s) = scene(Weapon::Radar, 4000.0);
        s.available = false;
        m.age = 0.55;
        fly(&mut m, c, s, -DVec3::Y, 0.01);
        assert!(m.gnc.active);
        assert_eq!(m.gnc.mode, Mode::Active);
        assert!(m.guided);
    }
    #[test]
    fn flares_have_repeatable_successes_and_failures_and_no_frame_lottery() {
        let (base, c, _) = scene(Weapon::Heat, 1800.0);
        let decoy = Contact {
            position: c.position - DVec3::Z * 25.0,
            decoy: true,
            strength: 8.0,
            ..c
        };
        let mut successes = 0;
        for seed in 1..=200 {
            let mut m = base.clone();
            m.gnc.seed = seed;
            let diverted = consider_decoy(&mut m, c, decoy, 0.3, 9, -DVec3::Y);
            successes += usize::from(diverted);
            assert!(!consider_decoy(&mut m, c, decoy, 0.3, 9, -DVec3::Y));
            let mut same = base.clone();
            same.gnc.seed = seed;
            assert_eq!(
                consider_decoy(&mut same, c, decoy, 0.3, 9, -DVec3::Y),
                diverted
            );
        }
        assert!(successes > 20 && successes < 180, "{successes}");
        let mut outside = decoy;
        outside.position += DVec3::X * 1500.0;
        assert_eq!(decoy_probability(&base, c, outside, 0.3, -DVec3::Y), 0.0);
    }
    #[test]
    fn fin_controller_is_rate_limited_and_low_density_limits_turn_authority() {
        let (mut m, mut c, s) = scene(Weapon::Heat, 2000.0);
        m.age = 1.0;
        c.position += DVec3::X * 100.0;
        c.velocity += DVec3::X * 150.0;
        let mut high = m.clone();
        high.position += DVec3::Y * 30000.0;
        let mut high_c = c;
        high_c.position += DVec3::Y * 30000.0;
        fly(&mut m, c, s, -DVec3::Y, 0.01);
        fly(&mut high, high_c, s, -DVec3::Y, 0.01);
        assert!(m.gnc.fins.length() <= 0.04000001);
        assert!(high.gnc.available_g < m.gnc.available_g * 0.1);
        assert!(m.gnc.actual_g <= m.gnc.available_g);
    }
    #[test]
    fn motor_consumes_propellant_and_burnout_coast_loses_energy() {
        let (mut m, mut c, s) = scene(Weapon::Heat, 6000.0);
        for _ in 0..600 {
            c.position += c.velocity * 0.01;
            fly(&mut m, c, s, -DVec3::Y, 0.01);
        }
        assert_eq!(m.gnc.thrust, 0.0);
        assert_eq!(m.gnc.propellant, 0.0);
        assert!((m.gnc.mass - 65.0).abs() < 1e-9);
        let energy = m.gnc.specific_energy;
        c.alive = false;
        m.gnc.confidence = 0.0;
        for _ in 0..200 {
            fly(&mut m, c, s, -DVec3::Y, 0.01);
        }
        assert!(m.gnc.specific_energy < energy);
        assert!(m.position.is_finite() && m.velocity.is_finite());
    }
    #[test]
    fn turning_consumes_more_energy_than_straight_coast() {
        let (mut straight, mut c, s) = scene(Weapon::Heat, 4000.0);
        straight.age = 7.0;
        straight.velocity = DVec3::Z * 600.0;
        let mut turning = straight.clone();
        let mut turn_c = c;
        turn_c.velocity += DVec3::X * 220.0;
        turning.gnc.estimated_velocity = turn_c.velocity;
        for _ in 0..120 {
            c.position += c.velocity * 0.01;
            turn_c.position += turn_c.velocity * 0.01;
            fly(&mut straight, c, s, -DVec3::Y, 0.01);
            fly(&mut turning, turn_c, s, -DVec3::Y, 0.01);
        }
        assert!(turning.gnc.specific_energy < straight.gnc.specific_energy);
        assert!(turning.gnc.actual_g > straight.gnc.actual_g);
    }
    #[test]
    fn closed_loop_intercepts_steady_targets_with_both_weapons() {
        for kind in [Weapon::Heat, Weapon::Radar] {
            let (mut m, mut c, s) = scene(
                kind,
                if kind == Weapon::Heat {
                    4000.0
                } else {
                    11000.0
                },
            );
            let mut closest = f64::INFINITY;
            for _ in 0..3600 {
                let old = c.position - m.position;
                c.position += c.velocity / 120.0;
                fly(&mut m, c, s, -DVec3::Y, 1.0 / 120.0);
                closest = closest.min(super::super::swept_distance(old, c.position - m.position));
                if closest < kind.fuse() {
                    break;
                }
            }
            assert!(closest < kind.fuse(), "{kind:?}: {closest}");
        }
    }
    #[test]
    fn halving_timestep_preserves_controller_trajectory() {
        let mut endpoints = Vec::new();
        for dt in [1.0 / 120.0, 1.0 / 240.0] {
            let (mut m, mut c, s) = scene(Weapon::Heat, 4000.0);
            c.velocity += DVec3::X * 80.0;
            m.gnc.estimated_velocity = c.velocity;
            for _ in 0..(3.0 / dt) as usize {
                c.position += c.velocity * dt;
                fly(&mut m, c, s, -DVec3::Y, dt);
            }
            endpoints.push((m.position, m.velocity));
        }
        assert!(endpoints[0].0.distance(endpoints[1].0) < 8.0);
        assert!(endpoints[0].1.distance(endpoints[1].1) < 4.0);
    }
}
