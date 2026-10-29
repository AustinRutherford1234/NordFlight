//! Six-degree-of-freedom, SI-unit flight model in a rotating spherical-planet frame.
//! Aircraft coefficients are deliberately an open, tunable game model, not Saab flight data.
use bevy::{
    math::{DMat3, DQuat, DVec3},
    prelude::*,
};
use std::f64::consts::PI;

pub const RADIUS: f64 = 6_371_000.0;
pub const MU: f64 = 3.986_004_418e14;
pub const AU: f64 = 149_597_870_700.0;
pub const SUN_MU: f64 = 1.327_124_400_18e20;
pub const YEAR: f64 = 365.256_363_004 * 86400.0;
pub const RUNWAY_HEIGHT: f64 = 55.1688;
pub const RUNWAY_HALF: f64 = 1132.0;
pub const WING_AREA: f64 = 30.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vehicle {
    Gripen,
    Spacecraft,
}

#[derive(Resource, Clone)]
pub struct Flight {
    pub water: crate::water::State,
    pub ejection: crate::ejection::State,
    pub carrier: crate::carrier::State,
    pub hornet: crate::aircraft::hornet::Dynamics,
    pub aircraft: crate::aircraft::Aircraft,
    pub systems: crate::aircraft::Systems,
    /// Per-aircraft elastic state. Aircraft without a structural configuration
    /// leave this at its default, rigid state.
    pub structure: crate::structural::State,
    pub space: crate::space::State,
    pub damage: AircraftDamage,
    pub vehicle: Vehicle,
    pub infinite_fuel: bool,
    pub battery: bool,
    pub lights: bool,
    pub apu: bool,
    pub engine: bool,
    pub integrity: f64,
    pub surface: DVec3,
    pub flow: crate::aerodynamics::FlowState,
    pub aero_coeff: crate::aerodynamics::Coefficients,
    pub panel_attachment: f64,
    pub actuator_load: f64,
    pub rcs_torque: DVec3,
    pub sas_mode: u8,
    pub gear_health: [f64; 3],
    /// Hydraulic extension (0 stowed, 1 down) and oleo compression in metres.
    pub gear_extension: f64,
    pub gear_compression: [f64; 3],
    pub gear_door: f64,
    pub gear_phase: u8,
    pub wreck_age: f64,
    pub impact_energy: f64,
    pub wreck_origin: DVec3,
    pub wreck_velocity: DVec3,
    pub position: DVec3,
    pub velocity: DVec3,
    pub orientation: DQuat,
    pub rates: DVec3,
    pub throttle: f64,
    pub spool: f64,
    pub fuel: f64,
    pub gear: bool,
    pub flaps: f64,
    pub brakes: bool,
    pub airbrake: bool,
    pub assist: bool,
    pub autopilot: bool,
    pub paused: bool,
    pub crashed: bool,
    pub landed: bool,
    pub alpha: f64,
    pub beta: f64,
    pub mach: f64,
    pub g_load: f64,
    /// Signed body-right accelerometer load, for crew presentation.
    pub lateral_g: f64,
    /// Signed body-forward (-Z) specific force; positive acceleration pushes crew back.
    pub longitudinal_g: f64,
    pub density: f64,
    pub airspeed: f64,
    pub thrust: f64,
    pub temperature: f64,
    pub pressure: f64,
    pub wind: DVec3,
    pub controls: DVec3,
    pub elapsed: f64,
    pub touchdown_sink: f64,
    pub status: String,
    pub best_landing: f64,
}

impl Default for Flight {
    fn default() -> Self {
        Self::approach()
    }
}
impl Flight {
    pub fn approach() -> Self {
        Self {
            water: Default::default(),
            ejection: Default::default(),
            carrier: Default::default(),
            hornet: Default::default(),
            aircraft: crate::aircraft::Aircraft::Gripen,
            systems: crate::aircraft::Systems::default(),
            structure: Default::default(),
            space: crate::space::State::default(),
            damage: AircraftDamage::default(),
            vehicle: Vehicle::Gripen,
            infinite_fuel: false,
            battery: true,
            lights: true,
            apu: true,
            engine: true,
            integrity: 1.0,
            surface: DVec3::ZERO,
            flow: Default::default(),
            aero_coeff: Default::default(),
            panel_attachment: 1.0,
            actuator_load: 0.0,
            rcs_torque: DVec3::ZERO,
            sas_mode: 0,
            gear_health: [1.0; 3],
            gear_extension: 1.0,
            gear_compression: [0.0; 3],
            gear_door: 0.0,
            gear_phase: 0,
            wreck_age: 0.0,
            impact_energy: 0.0,
            wreck_origin: DVec3::ZERO,
            wreck_velocity: DVec3::ZERO,
            position: surface_point(0.0, 6500.0, RUNWAY_HEIGHT + 330.0),
            velocity: DVec3::new(0.0, -4.2, -82.0),
            orientation: DQuat::from_rotation_x(0.026),
            rates: DVec3::ZERO,
            throttle: 0.34,
            spool: 0.34,
            fuel: 1500.0,
            gear: true,
            flaps: 0.55,
            brakes: false,
            airbrake: false,
            assist: true,
            autopilot: false,
            paused: false,
            crashed: false,
            landed: false,
            alpha: 0.0,
            beta: 0.0,
            mach: 0.24,
            g_load: 1.0,
            lateral_g: 0.0,
            longitudinal_g: 0.0,
            density: 1.2,
            airspeed: 82.0,
            thrust: 18000.0,
            temperature: 285.0,
            pressure: 98000.0,
            wind: DVec3::ZERO,
            controls: DVec3::ZERO,
            elapsed: 0.0,
            touchdown_sink: 0.0,
            status: "APPROACH  •  RUNWAY 18".into(),
            best_landing: 0.0,
        }
    }
    pub fn runway() -> Self {
        let mut f = Self::approach();
        f.position = surface_point(0.0, 1050.0, RUNWAY_HEIGHT + 1.55);
        f.velocity = DVec3::ZERO;
        f.orientation = DQuat::IDENTITY;
        f.throttle = 0.0;
        f.spool = 0.0;
        f.thrust = 0.0;
        f.flaps = 0.3;
        f.landed = true;
        f.status = "READY FOR TAKEOFF  •  SHIFT FULL THROTTLE / Z TO ADVANCE".into();
        f
    }
    pub fn cold() -> Self {
        let mut f = Self::runway();
        f.position = surface_point(150.0, 700.0, RUNWAY_HEIGHT + 1.55);
        f.battery = false;
        f.apu = false;
        f.engine = false;
        f.status = "COLD & DARK  •  1 BATTERY → 2 APU → 3 ENGINE".into();
        f
    }
    pub fn spacecraft() -> Self {
        let mut f = Self::approach();
        f.vehicle = Vehicle::Spacecraft;
        f.thrust = 0.0;
        f.position = surface_point(0.0, 0.0, 220000.0);
        // Start prograde in the simulated Moon's orbital plane, rather than in
        // the runway-local eastward plane (which costs a large plane change).
        let (moon_position, moon_velocity, _, _) = crate::space::moon_inertial(f.elapsed);
        let normal = moon_position.cross(moon_velocity).normalize();
        let radial = (f.position - normal * f.position.dot(normal)).normalize();
        f.position = radial * (RADIUS + 220000.0);
        let forward = normal.cross(radial).normalize();
        f.velocity = forward * (MU / f.position.length()).sqrt() - rotation().cross(f.position);
        f.orientation =
            DQuat::from_mat3(&DMat3::from_cols(forward.cross(radial), radial, -forward));
        f.throttle = 0.0;
        f.spool = 0.0;
        f.fuel = crate::space::Stage::Transfer.capacity();
        f.gear = false;
        f.gear_extension = 0.0;
        f.gear_door = 0.0;
        f.flaps = 0.0;
        let (temperature, pressure, density, sound) = atmosphere(f.altitude());
        f.temperature = temperature;
        f.pressure = pressure;
        f.density = density;
        f.airspeed = f.velocity.length();
        f.mach = f.airspeed / sound;
        f.g_load = 0.0;
        f.status = "ORBITAL SPACECRAFT  •  N TOGGLE FINITE / INFINITE FUEL".into();
        f
    }
    pub fn altitude(&self) -> f64 {
        if self.vehicle == Vehicle::Spacecraft {
            (self.position - crate::space::center(self.space.body, self.elapsed)).length()
                - self.space.body.radius()
        } else {
            self.position.length() - RADIUS
        }
    }
    pub fn usable_fuel(&self) -> f64 {self.fuel+if self.vehicle==Vehicle::Gripen&&self.aircraft==crate::aircraft::Aircraft::Hornet{crate::aircraft::hornet::stores::fuel(self)}else{0.}}
    pub fn mass(&self) -> f64 {
        if self.vehicle == Vehicle::Gripen {
            if self.aircraft==crate::aircraft::Aircraft::Hornet {crate::aircraft::hornet::mass::total(self)}else{self.aircraft.spec().empty + self.fuel + crate::aircraft::stores_mass(self)}
        } else {
            crate::space::mass(self)
        }
    }
    pub fn forward(&self) -> DVec3 {
        self.orientation * -DVec3::Z
    }
    pub fn up(&self) -> DVec3 {
        (self.position
            - if self.vehicle == Vehicle::Spacecraft {
                crate::space::center(self.space.body, self.elapsed)
            } else {
                DVec3::ZERO
            })
        .normalize()
    }
    pub fn agl(&self) -> f64 {
        self.altitude()
            - if self.vehicle == Vehicle::Spacecraft && self.space.body == crate::space::Body::Moon
            {
                crate::space::terrain_at(self.position, self.elapsed).0
            } else {
                crate::carrier::ground(self,self.position)
            }
    }
    pub fn vertical_speed(&self) -> f64 {
        (self.velocity
            - if self.vehicle == Vehicle::Spacecraft {
                crate::space::ground_velocity(self.position, self.elapsed, self.space.body)
            } else {
                DVec3::ZERO
            })
        .dot(self.up())
    }
    pub fn heading(&self) -> f64 {
        let v = self.forward();
        let (north, east) = geographic_axes(self.up());
        v.dot(east)
            .atan2(v.dot(north))
            .to_degrees()
            .rem_euclid(360.0)
    }
    pub fn roll(&self) -> f64 {
        let right = self.orientation * DVec3::X;
        (-right.dot(self.up())).atan2((self.orientation * DVec3::Y).dot(self.up()))
    }
    pub fn pitch(&self) -> f64 {
        self.forward().dot(self.up()).clamp(-1.0, 1.0).asin()
    }
    pub fn stall(&self) -> bool {
        self.flow.attached < 0.6 && self.airspeed > 15.0
    }
}
#[derive(Clone)]
pub struct AircraftDamage {
    pub aircraft: crate::aircraft::Aircraft,
    pub surface_state: [crate::combat::damage::SurfaceState; crate::aircraft::SURFACES],
    pub fire_age: f64,
    pub explosion_at: f64,
    pub rng: u64,
    pub wings: [f64; 2],
    pub wing_broken: [bool; 2],
    /// Signed aerodynamic root bending stress in pascals.
    pub wing_stress: [f64; 2],
    pub structure: [f64; 5],
    pub fire: f64,
    pub last_hit: f64,
    pub engine: f64,
    pub engines: [f64;2],
    pub nozzles: [f64;2],
    pub lift_fan: f64,
    pub avionics: f64,
    pub radar: f64,
    pub hydraulics: f64,
    pub surfaces: [f64; crate::aircraft::SURFACES],
    pub fuel_leak: f64,
}
impl Default for AircraftDamage {
    fn default() -> Self {
        Self {
            aircraft: crate::aircraft::Aircraft::Gripen,
            surface_state: [crate::combat::damage::SurfaceState::Operational; crate::aircraft::SURFACES],
            fire_age: 0.0,
            explosion_at: f64::INFINITY,
            rng: 0x72843bad9261,
            wings: [1.0; 2],
            wing_broken: [false; 2],
            wing_stress: [0.; 2],
            structure: [1.0; 5],
            fire: 0.0,
            last_hit: -100.0,
            engine: 1.0,
            engines:[1.;2],
            nozzles:[1.;2],
            lift_fan:1.,
            avionics: 1.0,
            radar: 1.0,
            hydraulics: 1.0,
            surfaces: [1.0; crate::aircraft::SURFACES],
            fuel_leak: 0.0,
        }
    }
}
pub fn geographic_axes(up: DVec3) -> (DVec3, DVec3) {
    let axis = rotation().normalize();
    let projected = axis - up * axis.dot(up);
    let north = if projected.length_squared() > 1e-12 {
        projected.normalize()
    } else {
        let seed = if up.x.abs() < 0.9 { DVec3::X } else { DVec3::Z };
        (seed - up * seed.dot(up)).normalize()
    };
    (north, north.cross(up).normalize())
}

#[derive(Resource, Clone)]
pub struct Environment {
    pub time: f64,
    pub day_phase: f64,
    pub orbit_phase: f64,
    pub weather: u8,
    pub wind_enabled: bool,
    pub orbit_rate: f64,
}
impl Default for Environment {
    fn default() -> Self {
        Self {
            time: 0.0,
            day_phase: 1.31,
            orbit_phase: 0.0,
            weather: 0,
            wind_enabled: true,
            orbit_rate: 1.0,
        }
    }
}
impl Environment {
    pub fn sun_direction(&self) -> DVec3 {
        thread_local! {
            static CACHE: std::cell::Cell<Option<([u64;2], DVec3)>> = const { std::cell::Cell::new(None) };
        }
        let key = [self.day_phase.to_bits(), self.orbit_phase.to_bits()];
        CACHE.with(|cache| {
            if let Some((cached, value)) = cache.get()
                && cached == key
            {
                return value;
            }
            let value = self.sun_direction_uncached();
            cache.set(Some((key, value)));
            value
        })
    }
    fn sun_direction_uncached(&self) -> DVec3 {
        let orbit = self.planet_orbit();
        let longitude = orbit.z.atan2(orbit.x);
        let declination = (23.44_f64.to_radians().sin() * longitude.sin()).asin();
        let hour = self.day_phase - longitude;
        let latitude = 58.428333_f64.to_radians();
        let east = -declination.cos() * hour.sin();
        let north =
            latitude.cos() * declination.sin() - latitude.sin() * declination.cos() * hour.cos();
        let vertical =
            latitude.sin() * declination.sin() + latitude.cos() * declination.cos() * hour.cos();
        let heading = 184.59_f64.to_radians();
        DVec3::new(
            east * heading.cos() - north * heading.sin(),
            vertical,
            -north * heading.cos() - east * heading.sin(),
        )
        .normalize()
    }
    pub fn sun_distance(&self) -> f64 {
        self.planet_orbit().length()
    }
    pub fn planet_orbit(&self) -> DVec3 {
        thread_local! {
            static CACHE: std::cell::Cell<Option<(u64, DVec3)>> = const { std::cell::Cell::new(None) };
        }
        let key = self.orbit_phase.to_bits();
        CACHE.with(|cache| {
            if let Some((cached, value)) = cache.get()
                && cached == key
            {
                return value;
            }
            let value = self.planet_orbit_uncached();
            cache.set(Some((key, value)));
            value
        })
    }
    fn planet_orbit_uncached(&self) -> DVec3 {
        let eccentricity = 0.0167;
        let mut eccentric_anomaly = self.orbit_phase;
        for _ in 0..6 {
            eccentric_anomaly -=
                (eccentric_anomaly - eccentricity * eccentric_anomaly.sin() - self.orbit_phase)
                    / (1.0 - eccentricity * eccentric_anomaly.cos());
        }
        DVec3::new(
            AU * (eccentric_anomaly.cos() - eccentricity),
            0.0,
            AU * (1.0 - eccentricity * eccentricity).sqrt() * eccentric_anomaly.sin(),
        )
    }
    pub fn advance(&mut self, dt: f64) {
        self.time += dt;
        self.day_phase += dt * 2.0 * PI / 86164.0905;
        self.orbit_phase =
            (self.orbit_phase + dt * self.orbit_rate * 2.0 * PI / YEAR).rem_euclid(2.0 * PI);
    }
    pub fn wind(&self, p: DVec3) -> DVec3 {
        if !self.wind_enabled {
            return DVec3::ZERO;
        }
        let strength = if self.weather == 2 { 3.5 } else { 1.0 };
        let h = (p.length() - RADIUS).max(0.0);
        let shear = (h / 180.0).clamp(0.0, 1.0).powf(0.16);
        let t = self.time;
        DVec3::new(
            (2.0 + (t * 0.71 + p.z * 0.0008).sin() * 0.8) * shear,
            (t * 1.1 + p.x * 0.0012).sin() * 0.22,
            (0.7 + (t * 0.33).sin() * 0.5) * shear,
        ) * strength
    }
}

pub fn atmosphere(h: f64) -> (f64, f64, f64, f64) {
    // Hydrostatic ideal-gas layers of the standard lower atmosphere. Convert
    // geometric altitude to geopotential altitude before selecting the layer.
    // Above 86 km, a smooth game transition to vacuum replaces thermosphere
    // physics; this is not suitable for predicting real satellite drag.
    const GAS: f64 = 287.05287;
    const G0: f64 = 9.80665;
    const GEO_RADIUS: f64 = 6_356_766.0;
    const BASES: [f64; 8] = [
        0.0, 11000.0, 20000.0, 32000.0, 47000.0, 51000.0, 71000.0, 84852.0,
    ];
    const LAPSE: [f64; 7] = [-0.0065, 0.0, 0.001, 0.0028, 0.0, -0.0028, -0.002];
    static LAYERS: std::sync::OnceLock<[(f64, f64); 8]> = std::sync::OnceLock::new();
    let layers = LAYERS.get_or_init(|| {
        let mut out = [(288.15, 101325.0); 8];
        for i in 0..7 {
            let (t, p) = out[i];
            let delta = BASES[i + 1] - BASES[i];
            let next_t = t + LAPSE[i] * delta;
            let next_p = if LAPSE[i] == 0.0 {
                p * (-G0 * delta / (GAS * t)).exp()
            } else {
                p * (next_t / t).powf(-G0 / (GAS * LAPSE[i]))
            };
            out[i + 1] = (next_t, next_p);
        }
        out
    });
    let geometric = h.max(0.0);
    let geopotential = GEO_RADIUS * geometric / (GEO_RADIUS + geometric);
    let (t, p) = if geopotential < BASES[7] {
        let i = (0..7)
            .rev()
            .find(|i| geopotential >= BASES[*i])
            .unwrap_or(0);
        let (base_t, base_p) = layers[i];
        let delta = geopotential - BASES[i];
        let t = base_t + LAPSE[i] * delta;
        let p = if LAPSE[i] == 0.0 {
            base_p * (-G0 * delta / (GAS * base_t)).exp()
        } else {
            base_p * (t / base_t).powf(-G0 / (GAS * LAPSE[i]))
        };
        (t, p)
    } else {
        let (t, base_p) = layers[7];
        let end = ((geometric - 86000.0) / 34000.0).clamp(0.0, 1.0);
        let fade = 1.0 - end * end * (3.0 - 2.0 * end);
        (
            t,
            base_p * (-G0 * (geopotential - BASES[7]) / (GAS * t)).exp() * fade,
        )
    };
    (t, p, p / (287.05287 * t), (1.4 * 287.05287 * t).sqrt())
}
pub fn surface_point(x: f64, z: f64, h: f64) -> DVec3 {
    let y = (RADIUS * RADIUS - x * x - z * z).max(0.0).sqrt();
    DVec3::new(x, y, z).normalize() * (RADIUS + h)
}
struct LakeIndex {
    min: bevy::math::DVec2,
    max: bevy::math::DVec2,
    rows: Vec<Vec<usize>>,
}
impl LakeIndex {
    fn row(&self, z: f64) -> usize {
        (((z-self.min.y) / (self.max.y-self.min.y).max(f64::MIN_POSITIVE) * self.rows.len() as f64).floor() as usize).min(self.rows.len()-1)
    }
    fn new(poly: &[bevy::math::DVec2]) -> Self {
        let mut index = Self { min: bevy::math::DVec2::splat(f64::INFINITY), max: bevy::math::DVec2::splat(f64::NEG_INFINITY), rows: vec![Vec::new(); 256] };
        for p in poly { index.min = index.min.min(*p); index.max = index.max.max(*p); }
        for i in 0..poly.len() {
            let a = poly[i];
            let b = poly[(i+poly.len()-1)%poly.len()];
            // Include adjacent bins conservatively at floating-point boundaries.
            let first = index.row(a.y.min(b.y)).saturating_sub(1);
            let last = (index.row(a.y.max(b.y))+1).min(index.rows.len()-1);
            for row in &mut index.rows[first..=last] { row.push(i); }
        }
        index
    }
    fn contains(&self, poly: &[bevy::math::DVec2], x: f64, z: f64) -> bool {
        if poly.is_empty() || x < self.min.x || x > self.max.x || z < self.min.y || z > self.max.y { return false; }
        let mut inside = false;
        for &i in &self.rows[self.row(z)] {
            let a = poly[i];
            let b = poly[(i+poly.len()-1)%poly.len()];
            // Identical crossing arithmetic and edge order to the original polygon test.
            if (a.y > z) != (b.y > z) && x < (b.x-a.x)*(z-a.y)/(b.y-a.y)+a.x { inside = !inside; }
        }
        inside
    }
}
static LAKE_INDEX: std::sync::OnceLock<LakeIndex> = std::sync::OnceLock::new();
#[cfg(test)]
mod lake_index_tests {
    use super::*;
    #[test]
    fn indexed_crossings_match_full_polygon_at_edges_bins_and_dense_points() {
        let poly: Vec<_> = (0..769).map(|i| {
            let a = i as f64 * std::f64::consts::TAU / 769.;
            let radius = 1000. + 220.*(a*13.).sin();
            bevy::math::DVec2::new(radius*a.cos(), radius*a.sin())
        }).collect();
        let index = LakeIndex::new(&poly);
        let original = |x: f64, z: f64| {
            let mut inside = false;
            for i in 0..poly.len() {
                let a = poly[i]; let b = poly[(i+poly.len()-1)%poly.len()];
                if (a.y > z) != (b.y > z) && x < (b.x-a.x)*(z-a.y)/(b.y-a.y)+a.x { inside = !inside; }
            }
            inside
        };
        for j in 0..=256 {
            let z = index.min.y + (index.max.y-index.min.y)*j as f64/256.;
            for i in -80..=80 {
                let x = i as f64 * 16.;
                for z in [z, z-1e-9, z+1e-9] { assert_eq!(index.contains(&poly,x,z), original(x,z)); }
            }
        }
        for p in &poly { assert_eq!(index.contains(&poly,p.x,p.y), original(p.x,p.y)); }
        assert!(!LakeIndex::new(&[]).contains(&[],0.,0.));
    }
}
pub static LAKE: std::sync::OnceLock<Vec<bevy::math::DVec2>> = std::sync::OnceLock::new();
pub fn in_lake(x: f64, z: f64) -> bool {
    let Some(poly) = LAKE.get() else {
        return false;
    };
    LAKE_INDEX.get_or_init(|| LakeIndex::new(poly)).contains(poly, x, z)
}
pub fn rotation() -> DVec3 {
    DVec3::new(
        7.292115e-5 * 58.428333_f64.to_radians().cos() * (-184.59_f64.to_radians()).sin(),
        7.292115e-5 * 58.428333_f64.to_radians().sin(),
        -7.292115e-5 * 58.428333_f64.to_radians().cos() * (-184.59_f64.to_radians()).cos(),
    )
}
pub fn terrain_height(x: f64, z: f64) -> f64 {
    // Flight, collision, AI and vegetation each ask for the height under the
    // same aircraft within one substep. A one-entry exact-bits memo per thread
    // returns the identical value without re-evaluating the noise.
    thread_local! {
        static LAST: std::cell::Cell<(u64, u64, f64)> = const { std::cell::Cell::new((u64::MAX, u64::MAX, 0.0)) };
    }
    let key = (x.to_bits(), z.to_bits());
    let (lx, lz, h) = LAST.get();
    if (lx, lz) == key {
        return h;
    }
    let h = terrain_height_uncached(x, z);
    LAST.set((key.0, key.1, h));
    h
}
fn terrain_height_uncached(x: f64, z: f64) -> f64 {
    if crate::carrier::sea(x,z) {return -2.;}
    if in_lake(x, z) {
        return -2.0;
    }
    // A fictional Nordic island: rolling pine coast, granite ridges and a flattened airfield.
    if x.abs() < 310.0 && z.abs() < 1850.0 {
        return RUNWAY_HEIGHT;
    }
    let coast = 1.0;
    let hills = 55.0
        + 16.0 * (x * 0.00051 + 0.6).sin() * (z * 0.00039).cos()
        + 6.0 * (x * 0.0017 + z * 0.0008).sin()
        + 3.0 * (z * 0.0027 - x * 0.0011).cos();
    let ridge = 230.0
        * (-((x + 11500.0) / 6500.0).powi(2)).exp()
        * (0.45 + 0.55 * (z * 0.00019 + 0.9).sin().powi(2));
    let island = (hills + ridge) * coast - 20.0 * (1.0 - coast);
    // Separate inland BVR theater: broad mountain chains with navigable valleys.
    // Smoothly vanishes around the original airfield and close-range scenarios.
    let theater = ((x - 35000.) / 35000.).clamp(0., 1.);
    let ridges = (0.5 + 0.5 * (x * 0.000075 + z * 0.000018).sin()).powi(4);
    let mountains = theater * (450. + 2600. * ridges * (0.6 + 0.4 * (z * 0.000052).cos().powi(2)));
    let island = island + mountains;
    let detail = 0.30 * (x * 0.043 + z * 0.017).sin() * (z * 0.031 - x * 0.012).cos()
        + 1.4 * (x * 0.009 + z * 0.006).sin() * (z * 0.007).cos();
    let d = ((x.abs() - 310.0).max(0.0).powi(2) + (z.abs() - 1850.0).max(0.0).powi(2)).sqrt();
    let blend = (d / 700.0).clamp(0.0, 1.0);
    let h = RUNWAY_HEIGHT * (1.0 - blend) + (island + detail) * blend;
    // A broad, obstacle-free meadow offers a discoverable off-airfield landing site.
    let edge = ((x - 6500.).abs() - 210.)
        .max(0.)
        .max(((z - 3200.).abs() - 1000.).max(0.));
    let meadow = 1. - (edge / 320.).clamp(0., 1.);
    let meadow = meadow * meadow * (3. - 2. * meadow);
    h * (1. - meadow) + 62. * meadow
}

/// Approximate allocation of the existing aircraft-level F-22 loads for the
/// elastic model. This is NOT a second source of rigid-body force: the
/// whole-aircraft model and panel increments still drive the flight dynamics.
/// Its source coefficients do not identify a unique pressure distribution;
/// these fractions and application points are openly assumed estimates.
fn raptor_elastic_loads(
    f: &Flight,
    body_air: DVec3,
    aerodynamic_force: DVec3,
    panels: &crate::control_surfaces::Loads,
) -> [crate::structural::Load; 16] {
    use crate::structural::{Load, LEFT_FIN, LEFT_TAIL, LEFT_WING, RIGHT_FIN, RIGHT_TAIL, RIGHT_WING};
    let empty = Load { surface: LEFT_WING, point: DVec3::ZERO, force: DVec3::ZERO, moment: DVec3::ZERO };
    let mut loads = [empty; 16];
    let flow = body_air.normalize_or_zero();
    let lift_axis = DVec3::X.cross(flow).normalize_or_zero();
    let side_axis = flow.cross(lift_axis).normalize_or_zero();
    // Only the baseline aircraft force is distributed here. Moving-panel
    // increments are transferred separately below, exactly once.
    let baseline = aerodynamic_force - panels.force;
    let lift = lift_axis * baseline.dot(lift_axis);
    let side = side_axis * baseline.dot(side_axis);
    let mut weights = [0.; 2];
    for i in 0..2 {
        weights[i] = f.damage.wings[i].clamp(0., 1.)
            * (0.25 + 0.75 * f.flow.wing_attached[i].clamp(0., 1.));
    }
    let weight_sum = weights[0] + weights[1];
    if weight_sum > 1e-12 && !f.damage.wing_broken.iter().any(|&broken| broken) {
        weights[0] /= weight_sum;
        weights[1] /= weight_sum;
    } else if f.damage.wing_broken.iter().any(|&broken| broken) {
        for i in 0..2 { weights[i] = if f.damage.wing_broken[i] { 0. } else { 0.5 }; }
    }
    for i in 0..2 {
        let sign = if i == 0 { -1. } else { 1. };
        let wing = if i == 0 { LEFT_WING } else { RIGHT_WING };
        let fin = if i == 0 { LEFT_FIN } else { RIGHT_FIN };
        let tail = if i == 0 { LEFT_TAIL } else { RIGHT_TAIL };
        loads[i] = Load { surface: wing, point: DVec3::new(sign * 3.9, -0.04, 0.6), force: lift * (0.84 * weights[i]), moment: DVec3::ZERO };
        loads[2 + i] = Load { surface: fin, point: DVec3::new(sign * 2.3, 1.6, 5.2), force: side * 0.35, moment: DVec3::ZERO };
        loads[4 + i] = Load { surface: tail, point: DVec3::new(sign * 3.0, 0.02, 6.15), force: lift * 0.04, moment: DVec3::ZERO };
    }
    let geometry = f.aircraft.panels();
    for (out, i) in [0_usize, 1, 2, 3, 4, 5, 6, 7, 9, 10].into_iter().enumerate() {
        let surface = match i {
            0 => LEFT_TAIL, 4 => RIGHT_TAIL,
            1 | 2 | 9 => LEFT_WING, 5 | 6 | 10 => RIGHT_WING,
            3 => LEFT_FIN, 7 => RIGHT_FIN,
            _ => unreachable!(),
        };
        loads[6 + out] = Load { surface, point: geometry[i].center, force: panels.surfaces[i].force, moment: DVec3::ZERO };
    }
    loads
}

/// Synthetic neutral, symmetric sea-level 15-G calibration for the default
/// 25,700.74 kg loadout. The 300 MPa allowable and effective section are
/// estimated simulation parameters, not F-22 engineering data.
const F22_WING_ALLOWABLE_PA: f64 = 300_000_000.;
const F22_WING_SECTION_M3: f64 = (0.42 * 15. * 25_700.74 * 9.80665 * 2.25) / F22_WING_ALLOWABLE_PA;

fn update_raptor_wing_failure(f: &mut Flight, loads: &[crate::structural::Load]) {
    let config = crate::structural::f22_config();
    for side in 0..2 {
        if f.damage.wing_broken[side] { continue; }
        let surface = if side == 0 { crate::structural::LEFT_WING } else { crate::structural::RIGHT_WING };
        let moment = crate::structural::root_bending_moment(&config.surfaces[surface], surface, loads);
        let health = f.damage.wings[side].clamp(0., 1.);
        let stress = crate::structural::bending_stress(moment, F22_WING_SECTION_M3 * health.max(1e-9)).unwrap_or(0.);
        f.damage.wing_stress[side] = stress;
        if stress.abs() > F22_WING_ALLOWABLE_PA {
            f.damage.wing_broken[side] = true;
            f.damage.wings[side] = 0.;
            for i in if side == 0 { [1, 2, 9] } else { [5, 6, 10] } {
                f.damage.surface_state[i] = crate::combat::damage::SurfaceState::Detached;
                f.damage.surfaces[i] = 0.;
                f.structure.panel_modes[i] = Default::default();
            }
            f.structure.surfaces[surface] = Default::default();
        }
    }
}

/// Separate panel modes see an estimated share of their supporting assembly's
/// baseline load plus their actual incremental load. These are internal modal
/// inputs, not additional forces on the parent or aircraft.
fn raptor_elastic_panel_loads(
    f: &Flight,
    assembly_loads: &[crate::structural::Load; 16],
    panels: &crate::control_surfaces::Loads,
    shape: &crate::structural::State,
    config: &crate::structural::Config,
) -> [crate::structural::PanelLoad; 10] {
    use crate::structural::PanelLoad;
    let mut result = [PanelLoad { panel: 0, point: DVec3::ZERO, force: DVec3::ZERO, moment: DVec3::ZERO }; 10];
    let geometry = f.aircraft.panels();
    let angles = crate::aircraft::angles(f);
    for (out, i) in [0_usize, 1, 2, 3, 4, 5, 6, 7, 9, 10].into_iter().enumerate() {
        let p = geometry[i];
        let (assembly_index, fraction) = match i {
            0 => (4, 1.), 4 => (5, 1.),
            1 | 2 | 9 => (0, (p.area / 39.).clamp(0., 1.)),
            5 | 6 | 10 => (1, (p.area / 39.).clamp(0., 1.)),
            3 => (2, (p.area / 5.0).clamp(0., 1.)),
            7 => (3, (p.area / 5.0).clamp(0., 1.)),
            _ => unreachable!(),
        };
        let parent = assembly_loads[assembly_index].surface;
        let parent_frame = shape.frame_rotation(config, parent, p.center);
        let hinge = crate::aircraft::surface_rotation(f.aircraft, i, angles[i]);
        let world_to_neutral = (parent_frame * hinge).conjugate();
        let intact = f.damage.surface_state[i] != crate::combat::damage::SurfaceState::Detached;
        let force = if intact { assembly_loads[assembly_index].force * fraction + panels.surfaces[i].force } else { DVec3::ZERO };
        result[out] = PanelLoad { panel: i, point: crate::aircraft::raptor_panel_flex_sample(i, p.center), force: world_to_neutral * force, moment: DVec3::ZERO };
    }
    result
}

/// Approximate local incidence changes of the baseline lifting assemblies.
/// The aggregate F-22 polar has no spatially resolved pressure data, so the
/// weights match the estimated lift/side-force allocation above. Elastic
/// velocity is relative to the body; rigid-body rate terms remain in the polar.
fn raptor_elastic_incidence(f: &Flight, body_air: DVec3) -> (f64, f64, f64) {
    use crate::structural::{LEFT_FIN, LEFT_TAIL, LEFT_WING, RIGHT_FIN, RIGHT_TAIL, RIGHT_WING};
    if body_air.length_squared() < 1. { return (0., 0., 0.); }
    let config = crate::structural::f22_config();
    let alpha = (-body_air.y).atan2(-body_air.z);
    let beta = (body_air.x / body_air.length()).clamp(-1., 1.).asin();
    let wrap = |a: f64| (a + PI).rem_euclid(2. * PI) - PI;
    let incidence = |surface: usize, point: DVec3| {
        let state = &f.structure;
        let rotation = state.frame_rotation(config, surface, point);
        let v = rotation.conjugate() * (body_air + state.point_velocity(config, surface, point));
        if v.length_squared() < 1. { return (0., 0.); }
        let a = (-v.y).atan2(-v.z);
        let b = (v.x / v.length()).clamp(-1., 1.).asin();
        // The small-deformation model is not an extrapolated high-angle polar.
        (wrap(a-alpha).clamp(-0.15, 0.15), (b-beta).clamp(-0.15, 0.15))
    };
    let left = incidence(LEFT_WING, DVec3::new(-3.9, -0.04, 0.6)).0;
    let right = incidence(RIGHT_WING, DVec3::new(3.9, -0.04, 0.6)).0;
    let tail_left = incidence(LEFT_TAIL, DVec3::new(-3., 0.02, 6.15)).0;
    let tail_right = incidence(RIGHT_TAIL, DVec3::new(3., 0.02, 6.15)).0;
    let fin_left = incidence(LEFT_FIN, DVec3::new(-2.3, 1.6, 5.2)).1;
    let fin_right = incidence(RIGHT_FIN, DVec3::new(2.3, 1.6, 5.2)).1;
    (0.42 * (left + right) + 0.04 * (tail_left + tail_right),
     0.35 * (fin_left + fin_right), right - left)
}

pub fn simulate(f: &mut Flight, e: &Environment, dt: f64) {
    // Flexible control surfaces contain faster modes than rigid flight motion.
    // Integrate the coupled aircraft and structure together, not just the
    // oscillators with frozen aerodynamic loads, at a sufficiently small step.
    // Keep the caller's total simulated duration and environment sample intact.
    if f.vehicle == Vehicle::Gripen && matches!(f.aircraft,crate::aircraft::Aircraft::F22|crate::aircraft::Aircraft::Hornet)
        && dt.is_finite() && dt > 1. / 240. {
        let count = (dt * 240.).ceil() as usize;
        let h = dt / count as f64;
        for _ in 0..count { simulate_step(f, e, h); }
    } else {
        simulate_step(f, e, dt);
    }
}

fn simulate_step(f: &mut Flight, e: &Environment, dt: f64) {
    let _measure = crate::performance::span(1);
    if f.paused {
        return;
    }
    crate::ejection::advance(f,dt);
    if crate::water::step(f,dt) {return;}
    if f.crashed {
        if f.vehicle == Vehicle::Spacecraft {
            crate::space::wreck(f, dt);
            return;
        }
        crate::collision::simulate_wreck(f, dt);
        return;
    }
    if f.vehicle==Vehicle::Gripen && f.aircraft==crate::aircraft::Aircraft::Hornet {crate::aircraft::hornet::step(f,e,dt);return;}
    let spec = f.aircraft.spec();
    let custom = f.aircraft != crate::aircraft::Aircraft::Gripen;
    let wing_area = spec.area;
    let previous = f.position;
    let previous_velocity = f.velocity;
    let previous_rotation = f.orientation;
    let previous_rates = f.rates;
    if f.vehicle == Vehicle::Gripen {
        if custom {f.systems.engines=f.damage.engines;f.systems.lift_fan=f.damage.lift_fan;}
        crate::combat::damage::advance(&mut f.damage, &mut f.integrity, dt, f.engine, f.fuel > 0.0);
        if crate::combat::damage::fatal(&f.damage, f.integrity) {
            let cause = if f.damage.explosion_at.is_finite()
                && f.damage.fire_age >= f.damage.explosion_at
            {
                "FUEL FIRE / DELAYED EXPLOSION"
            } else {
                "BATTLE DAMAGE / STRUCTURAL FAILURE"
            };
            crate::collision::destroy(f, cause, f.up());
            return;
        }
        if f.damage.engine < 0.12 {
            f.engine = false;
        }
        if f.damage.avionics < 0.25 {
            f.assist = false;
            f.autopilot = false;
        }
    }
    if f.vehicle == Vehicle::Spacecraft {
        simulate_spacecraft(f, e, dt);
        return;
    }
    crate::collision::update_gear(f, dt);
    f.elapsed += dt;
    let up = f.up();
    let h = f.altitude();
    let (temp, pressure, rho, sound) = atmosphere(h);
    f.temperature = temp;
    f.pressure = pressure;
    f.density = rho;
    f.wind = e.wind(f.position);
    let air = f.velocity - f.wind;
    let speed = air.length().max(0.01);
    f.airspeed = speed;
    f.mach = speed / sound;
    let b = f.orientation.conjugate() * air;
    let alpha = (-b.y).atan2(-b.z);
    let beta = (b.x / speed).clamp(-1.0, 1.0).asin();
    f.alpha = alpha;
    f.beta = beta;
    let q = 0.5 * rho * speed * speed;
    if f.aircraft == crate::aircraft::Aircraft::F22 && !f.systems.raptor.actuators_initialized {
        f.systems.raptor.angles = crate::aircraft::angles(f);
        f.systems.raptor.nozzle_control = f.surface;
        f.systems.raptor.actuators_initialized = true;
    }
    let elastic_incidence = if f.aircraft == crate::aircraft::Aircraft::F22 {
        raptor_elastic_incidence(f, b)
    } else { (0., 0., 0.) };
    let evaluate=if f.aircraft==crate::aircraft::Aircraft::F22{crate::aerodynamics::evaluate_raptor}else{crate::aerodynamics::evaluate};
    let mut coefficients = evaluate(
        crate::aerodynamics::Conditions {
            alpha: alpha + elastic_incidence.0,
            beta: beta + elastic_incidence.1,
            mach: f.mach,
            speed,
            density: rho,
            temperature: temp,
            flaps: 0.0, // Moving-panel loads are evaluated separately below.
            gear: false,
            airbrake: false,
            height: f.agl(),
            rates: f.rates,
        },
        &mut f.flow,
        dt,
    );
    if f.aircraft == crate::aircraft::Aircraft::F22 {
        // Estimated differential wing lift acts with a spanwise lever arm.
        // This correction vanishes in the rigid limit; the aggregate polar has
        // no independently measured left/right lift derivatives.
        coefficients.roll += 0.25 * elastic_incidence.2;
    }
    if custom {
        let lift_gain = match f.aircraft {crate::aircraft::Aircraft::Su27=>1.10,crate::aircraft::Aircraft::Hornet=>1.15,_=>1.0};
        coefficients.lift*=lift_gain;
        let c=spec.area/spec.span;
        // Conventional tail stability and pitch damping are distinct terms.
        // Reducing the static margin must not also remove rotational damping.
        // Separated-flow pitch moment returns the nose toward the relative wind.
        let blend=((alpha.abs()-1.2)/0.6).clamp(0.,1.);
        let break_blend=((alpha.abs()-1.45)/0.35).clamp(0.,1.);
        // Aft centre-of-pressure migration during deep separation supplies
        // recovery torque after the inertial pitch-up. Keep this distinct from
        // the low-incidence static margin, rather than imposing an angle cap.
        let (static_margin,damping,post_stall)=if f.aircraft==crate::aircraft::Aircraft::Su27{(0.025,2.+7.*blend*blend*(3.-2.*blend),0.08+0.20*break_blend*break_blend*(3.-2.*break_blend))}else if f.aircraft==crate::aircraft::Aircraft::F22{(0.035,5.5,0.075)}else{(0.075,8.,0.13)};
        if f.aircraft != crate::aircraft::Aircraft::F22 {
            coefficients.pitch=0.004-static_margin*alpha.sin()-post_stall*(1.-f.flow.attached)*alpha.sin()-damping*f.rates.x*c/(2.*speed.max(15.));
        }
        coefficients.drag += crate::aircraft::store_drag(f) + f.systems.conversion*0.035;
        coefficients.parasite += crate::aircraft::store_drag(f);
    }
    // Drag follows the actual deployment rather than changing at the G key press.
    coefficients.parasite += 0.022 * f.gear_extension;
    coefficients.drag += 0.022 * f.gear_extension;
    let battle_drag = 0.012 * (1.0 - f.damage.surfaces.iter().take(crate::aircraft::count(f.aircraft)).sum::<f64>() / crate::aircraft::count(f.aircraft) as f64)
        + 0.008 * (1.0 - f.damage.engine)
        + 0.003
            * f.damage
                .surface_state
                .iter()
                .filter(|s| **s == crate::combat::damage::SurfaceState::Detached)
                .count() as f64;
    coefficients.parasite += battle_drag;
    coefficients.drag += battle_drag;
    f.aero_coeff = coefficients;
    let forward = f.forward();
    let right = f.orientation * DVec3::X;
    let drag = q * wing_area * coefficients.drag;
    f.spool += (f.throttle - f.spool) * (1.0 - (-dt / 0.8).exp());
    let (installed_thrust, burner) = if f.aircraft == crate::aircraft::Aircraft::F22 {
        crate::aircraft::raptor::flight_model::installed(f)
    } else if custom {crate::aircraft::jet_output(f.aircraft,f.spool,rho,f.mach)}else{crate::propulsion::jet_output(f.spool, rho, f.mach)};
    f.thrust = installed_thrust
        * if (f.fuel > 0.0 || f.infinite_fuel) && f.engine {
            f.damage.engine
        } else {
            0.0
        };
    if custom && f.engine && (f.fuel>0.||f.infinite_fuel){f.thrust=installed_thrust*(0..spec.engines).map(|i|f.systems.engines[i]*if f.systems.engine_on[i]{1.}else{0.}).sum::<f64>()/spec.engines as f64;}
    f.fuel = if f.infinite_fuel {
        f.fuel
    } else {
        let fuel_rate = if f.aircraft == crate::aircraft::Aircraft::F22 {
            crate::aircraft::raptor::flight_model::fuel_flow(f)
        } else if f.thrust > 0.0 {
            f.thrust * 0.000019 + burner * 0.000035
        } else {
            0.0
        };
        (f.fuel - dt * (fuel_rate + f.damage.fuel_leak)).max(0.0)
    };
    let mut aero = crate::aerodynamics::force(right, air, q * wing_area, coefficients);
    let (thrust, engine_moment) = if custom {
        let (body,moment,_) = crate::aircraft::propulsion(f,dt,rho,f.mach);
        f.thrust=body.length();
        (f.orientation*body,moment)
    } else {(forward*f.thrust,DVec3::ZERO)};
    let omega = rotation();
    let gravity = -up * MU / f.position.length_squared();
    let rotational = -2.0 * omega.cross(f.velocity) - omega.cross(omega.cross(f.position));
    let sun = e.sun_direction() * e.sun_distance();
    let ds = sun - f.position;
    let tidal = SUN_MU * (ds / ds.length().powi(3) - sun / sun.length().powi(3));
    let mut acc = gravity + rotational + tidal + (aero + thrust) / f.mass();
    let previous_g_load = f.g_load;

    let mut command = f.controls;
    if f.assist && !f.autopilot && !f.landed && f.aircraft != crate::aircraft::Aircraft::F22 {
        // FBW protects the usable game envelope; raw controls remain available
        // with T. Predict incidence using pitch rate to avoid waiting for stall.
        let predicted_alpha = alpha + f.rates.x * 0.15;
        if command.x > 0.0 {
            let alpha_margin = ((spec.alpha - predicted_alpha) / 0.10).clamp(0.0, 1.0);
            let load_margin = ((spec.g - previous_g_load) / 2.0).clamp(0.0, 1.0);
            command.x *= alpha_margin.min(load_margin);
        } else {
            command.x *= ((previous_g_load + 3.0) / 1.5).clamp(0.0, 1.0);
        }
    }
    if f.autopilot && !f.landed {
        let desired_heading = if f.aircraft==crate::aircraft::Aircraft::F22{(-f.position.x/700.-f.wind.x/speed.max(30.)).clamp(-0.35,0.35)}else{(-f.position.x / 1800.0).clamp(-0.35, 0.35)};
        let heading = f.forward().x.atan2(-f.forward().z);
        let bank = (desired_heading - heading).clamp(-0.4, 0.4) * 1.4;
        let distance = (f.position.z - 1000.0).max(0.0);
        let desired_alt = RUNWAY_HEIGHT + 1.55 + distance * 0.0524;
        let approach_pitch=if f.aircraft==crate::aircraft::Aircraft::F22{0.01}else{0.055};
        let pitch_target = (approach_pitch + (desired_alt - f.altitude()) * 0.0010
            - (f.vertical_speed() + speed * 0.0524) * 0.004)
            .clamp(-0.13, 0.20);
        command.x = ((pitch_target - f.pitch()) * 3.0).clamp(-0.55, 0.55);
        command.z = ((bank - f.roll()) * 2.0).clamp(-0.65, 0.65);
        command.y = -beta * 2.0;
        let approach_speed=if f.aircraft==crate::aircraft::Aircraft::F22{crate::aircraft::raptor::flight_model::approach_speed(f,rho)}else{82.};
        let approach_thrust=if f.aircraft==crate::aircraft::Aircraft::F22{spec.dry}else{50400.};
        f.throttle = ((drag + f.mass() * 0.18 * (approach_speed - speed))
            / (approach_thrust * (rho / 1.225).powf(0.65).max(0.1))
            * 0.88)
            .clamp(0.05, 0.86);
        f.gear = true;
        f.flaps = 0.65;
        if f.agl() < 12.0 {
            let flare_pitch = (0.065 + (-1.3 - f.vertical_speed()) * 0.018).clamp(0.025, 0.18);
            command.x = ((flare_pitch - f.pitch()) * 4.0).clamp(-0.45, 0.55);
            if f.agl() < 4.0 {
                f.throttle = 0.08;
            }
        }
        if f.aircraft==crate::aircraft::Aircraft::F22 {
            // Aircraft-specific outer glide/flare controller. Feed normal-load
            // error to the inner rate loop rather than demanding a Gripen trim
            // attitude from a larger wing and different flap/tail installation.
            let sink=if f.agl()<12.{-1.2}else{(-speed*0.0524+(desired_alt-f.altitude())*0.12).clamp(-9.,2.)};
            let load=((1.+(sink-f.vertical_speed())*0.22)/f.roll().cos().max(0.7)).clamp(0.2,2.2);
            command.x=((load-previous_g_load)*0.06-f.rates.x*0.4).clamp(-0.6,0.6);
        }
    }
    // Aerodynamic moments with rate derivatives, servo dynamics and rigid-body Euler terms.
    // Body axes: +X right (pitch), +Y up (yaw), -Z forward (roll).
    let inertia = spec.inertia * (f.mass() / (spec.empty + if custom {spec.fuel*0.6}else{1500.}));
    let inertia_tensor = if f.aircraft == crate::aircraft::Aircraft::F22 {
        crate::aircraft::raptor::flight_model::inertia(f)
    } else { DMat3::from_diagonal(inertia) };
    let raptor_inverse = if f.aircraft == crate::aircraft::Aircraft::F22 { inertia_tensor.inverse() } else { DMat3::ZERO };
    let span = spec.span;
    let chord = if custom {spec.area/spec.span}else{3.5};
    let (damage_force, damage_moment) =
        crate::combat::damage::aero(&f.damage, b, q, coefficients.lift);
    aero += f.orientation * damage_force;
    acc += f.orientation * damage_force / f.mass();
    // F-22 load-command feedback uses the preceding completed accelerometer
    // sample, including moving panels and contact forces. The partial force
    // sum here omits those loads and must not replace that sensor sample.
    if f.aircraft != crate::aircraft::Aircraft::F22 {
        f.g_load = (aero + thrust).dot(f.orientation * DVec3::Y) / (f.mass() * 9.80665);
    }
    let natural = DVec3::new(
        q * wing_area * chord * coefficients.pitch,
        q * wing_area * span * coefficients.yaw,
        q * wing_area * span * coefficients.roll,
    ) + damage_moment + if f.aircraft == crate::aircraft::Aircraft::F22 {
        crate::aircraft::raptor::flight_model::bay_moment(f,q,b)
    } else { DVec3::ZERO };
    let panel_model = crate::control_surfaces::PreparedPanels::new(
        b,
        f.rates,
        rho,
        coefficients.control_scale,
        f.mach,
        &f.damage,
    );
    let elastic_shape = f.structure;
    let panel_loads = |actuator| {
        if f.aircraft == crate::aircraft::Aircraft::F22 {
            return crate::aircraft::panel_loads_structural(f.aircraft,b,f.rates,rho,f.mach,actuator,f.flaps,f.airbrake,&f.damage,&elastic_shape);
        }
        if custom {return crate::aircraft::panel_loads(f.aircraft,b,f.rates,rho,f.mach,actuator,f.flaps,f.airbrake,&f.damage);}
        crate::control_surfaces::damaged(
            panel_model.evaluate(actuator, f.flaps, f.airbrake),
            &f.damage.surfaces,
        )
    };
    let neutral_panels = panel_loads(DVec3::ZERO);
    // Match fly-by-wire authority to the actual surface model, including flap trim.
    let target = if f.aircraft==crate::aircraft::Aircraft::F22 && f.assist && !f.landed {
        crate::aircraft::raptor::target_rates(f,command)
    }else{DVec3::new(command.x * 0.65, command.y * 0.38, -command.z * 1.55)};
    let request = if f.assist && f.damage.avionics > 0.25 {
        let airborne_raptor=f.aircraft==crate::aircraft::Aircraft::F22 && !f.landed;
        let center=if airborne_raptor {f.surface} else {DVec3::ZERO};
        // Large deflections and separated flow change every coupled
        // derivative. Probe the current state once for all three axes.
        let derivative = |axis: DVec3| {
            (panel_loads(center + axis * 0.02).moment - panel_loads(center - axis * 0.02).moment) / 0.04
        };
        let vector = if f.aircraft==crate::aircraft::Aircraft::F22{crate::aircraft::raptor::vector_jacobian(f)}else{DMat3::ZERO};
        let mut panel_matrix=DMat3::from_cols(derivative(DVec3::X),derivative(DVec3::Y),derivative(DVec3::Z));
        let mut panel_bias=if airborne_raptor {panel_loads(center).moment-panel_matrix*center} else {neutral_panels.moment};
        if f.aircraft==crate::aircraft::Aircraft::F22 && f.landed {
            // Preserve the ground roll's existing pitch-trim response until
            // weight is off the wheels; the airborne law is not a taxi mode.
            let slope=|axis:DVec3| (panel_loads(f.surface+axis*0.02).moment.z-panel_loads(f.surface-axis*0.02).moment.z)/0.04;
            panel_matrix.x_axis.z=slope(DVec3::X);
            panel_matrix.y_axis.z=slope(DVec3::Y);
            panel_matrix.z_axis.z=slope(DVec3::Z);
            panel_bias.z=panel_loads(f.surface).moment.z-(panel_matrix*f.surface).z;
        }
        let jacobian = panel_matrix+vector;
        let vector_bias=if f.aircraft==crate::aircraft::Aircraft::F22{engine_moment-vector*crate::aircraft::raptor::flight_model::nozzle_command(f)}else{DVec3::ZERO};
        if airborne_raptor {
            let response=raptor_inverse*jacobian;
            let moment=inertia_tensor*crate::aircraft::raptor::fbw::tracking_acceleration(f.rates,target,response)
                +f.rates.cross(inertia_tensor*f.rates)-natural-panel_bias-vector_bias;
            crate::aircraft::raptor::fbw::allocate(response,raptor_inverse*moment)
        } else {
            let moment=if f.aircraft==crate::aircraft::Aircraft::F22 { inertia_tensor*((target-f.rates)*5.) }
                else { (target-f.rates)*5.*inertia };
            crate::control_surfaces::allocate(jacobian,moment-natural-panel_bias-vector_bias)
        }
    } else {
        DVec3::new(command.x, command.y, -command.z)
    };
    let request = if f.battery { request } else { DVec3::ZERO };
    let request = request.clamp(DVec3::splat(-1.0), DVec3::splat(1.0));
    if f.aircraft != crate::aircraft::Aircraft::F22 && f.damage.hydraulics > 0.12 {
        f.surface = crate::control_surfaces::advance_actuator(
            f.surface,
            request,
            q,
            dt * f.damage.hydraulics,
        );
    }
    let legacy_panels = if f.aircraft == crate::aircraft::Aircraft::F22 { None }
        else { Some(panel_loads(f.surface)) };
    let panels = if let Some(panels) = legacy_panels { panels } else {
        f.systems.raptor.command = request;
        if f.damage.hydraulics > 0.12 {
            f.surface = crate::control_surfaces::advance_actuator(f.surface, request, q, dt * f.damage.hydraulics);
        }
        crate::aircraft::raptor::flight_model::advance_surfaces(f,q,dt);
        crate::aircraft::actual_panel_loads(f,b,rho)
    };
    let surface_force = f.orientation * panels.force;
    f.panel_attachment = crate::aircraft::attachment(f.aircraft,&f.damage,&panels);
    f.actuator_load = panels
        .surfaces
        .iter()
        .take(crate::aircraft::count(f.aircraft))
        .map(|p| p.deflection.abs())
        .fold(0.0, f64::max);
    aero += surface_force;
    acc += surface_force / f.mass();
    f.g_load = (aero + thrust).dot(f.orientation * DVec3::Y) / (f.mass() * 9.80665);
    let torque = natural + panels.moment + engine_moment - crate::aircraft::mass_center(f).cross(f.orientation.conjugate()*(aero+thrust));
    // Publish the total loads so the aerodynamic overlay includes moving panels.
    if q * wing_area > 1e-6 {
        let flow = b.normalize_or_zero();
        let lift_axis = DVec3::X.cross(flow).normalize_or_zero();
        let side_axis = flow.cross(lift_axis).normalize_or_zero();
        f.aero_coeff.lift += panels.force.dot(lift_axis) / (q * wing_area);
        let induced=if f.aircraft==crate::aircraft::Aircraft::F22{crate::aerodynamics::raptor_induced}else{crate::aerodynamics::induced_drag};
        let total_induced = induced(
            f.aero_coeff.lift,
            coefficients.attached,
            beta,
            f.agl(),
        );
        let induced_delta = total_induced - coefficients.induced;
        let induced_force = -air.normalize_or_zero() * (q * wing_area * induced_delta);
        aero += induced_force;
        acc += induced_force / f.mass();
        f.aero_coeff.induced = total_induced;
        f.aero_coeff.drag += induced_delta;
        f.aero_coeff.side += panels.force.dot(side_axis) / (q * wing_area);
        f.aero_coeff.control_drag = -panels.force.dot(flow) / (q * wing_area);
        f.aero_coeff.drag += f.aero_coeff.control_drag;
        f.aero_coeff.pitch = torque.x / (q * wing_area * chord);
        f.aero_coeff.yaw = torque.y / (q * wing_area * span);
        f.aero_coeff.roll = torque.z / (q * wing_area * span);
    }
    let angular_acc = if f.aircraft == crate::aircraft::Aircraft::F22 {
        raptor_inverse * (torque - f.rates.cross(inertia_tensor * f.rates))
    } else { (torque - f.rates.cross(inertia * f.rates)) / inertia };
    f.g_load = (aero + thrust).dot(f.orientation * DVec3::Y) / (f.mass() * 9.80665);
    f.rates += angular_acc * dt;
    f.orientation = (f.orientation * DQuat::from_scaled_axis(f.rates * dt)).normalize();
    if f.aircraft == crate::aircraft::Aircraft::F22 {
        crate::aircraft::raptor::flight_model::structural_damage(f,q,dt);
    } else if f.g_load.abs() > spec.g+0.5 {
        f.integrity -= (f.g_load.abs() - spec.g - 0.5) * dt * 0.035;
    }
    if f.aircraft != crate::aircraft::Aircraft::F22 && f.mach > spec.limit_mach {
        f.integrity -= (f.mach - spec.limit_mach) * dt * 0.08;
    }
    if f.integrity <= 0.0 {
        crate::collision::destroy(f, "STRUCTURAL FAILURE", up);
        return;
    }

    if crate::water::step(f,dt) {return;}
    let ground = crate::carrier::ground(f,f.position);
    let clearance = crate::collision::wheel_clearance(f);
    let contact = f.altitude() < ground + clearance
        || (f.agl() < clearance + 12. && crate::collision::wheel_contact(f));
    if contact {
        let normal = crate::carrier::normal(f,f.position);
        let surface_velocity=crate::carrier::ground_velocity(f,f.position);
        let sink = (f.velocity-surface_velocity).dot(normal);
        let on_deck=crate::carrier::deck(f,f.position).is_some();
        let on_runway = on_deck || crate::terrain::paved(f.position.x, f.position.z);
        if !f.landed {
            f.touchdown_sink = -sink;
            if f.gear_extension < 0.98
                || sink < if on_deck {-9.0}else{-5.5}
                || (f.orientation * DVec3::X).dot(normal).abs() > 0.20_f64.sin()
                || normal.dot(up) < 12_f64.to_radians().cos()
                || (!on_deck && (in_lake(f.position.x, f.position.z)||crate::carrier::sea(f.position.x,f.position.z)))
            {
                for health in &mut f.gear_health {
                    *health = (*health - ((-sink - 3.0).max(0.0) / 3.0).powi(2)).max(0.0);
                }
                crate::collision::destroy(
                    f,
                    if in_lake(f.position.x, f.position.z)||crate::carrier::sea(f.position.x,f.position.z) {
                        "WATER IMPACT"
                    } else {
                        "HARD LANDING / UNSUITABLE TERRAIN"
                    },
                    normal,
                );
                return;
            }
            f.landed = true;
            let score = (100.0
                - (-sink).max(0.0) * 8.0
                - if on_runway {
                    f.position.x.abs() * 0.8
                } else {
                    0.
                })
            .clamp(0.0, 100.0);
            f.best_landing = f.best_landing.max(score);
            f.status = format!(
                "TOUCHDOWN / {}  •  {:.1} m/s SINK  •  SCORE {:.0}  •  HOLD B TO BRAKE",
                if on_deck { "CARRIER DECK" } else if on_runway { "RUNWAY" } else { "OFF-FIELD" },
                -sink,
                score
            );
        }

        let (suspension, angular) = crate::collision::suspension(f, dt);
        let normal = suspension.dot(up);
        acc += suspension;
        f.rates += angular * dt;
        let surface_normal = crate::carrier::normal(f,f.position);
        let relative=f.velocity-surface_velocity;
        let tangent = relative - surface_normal * relative.dot(surface_normal);
        let braking = if f.brakes || f.autopilot {
            if on_runway { 0.55 } else { 0.30 }
        } else if on_runway {
            0.015
        } else {
            0.038
        };
        acc -= tangent.normalize_or_zero() * (normal + 9.81) * braking;
        // Tire lateral grip with a mild nose-wheel steering response.
        acc -= right * (relative.dot(right) * 2.0).clamp(-4.0, 4.0);
        f.rates.y += command.y * dt * 0.6;
        if speed < 2.0 && f.throttle < 0.1 {
            f.velocity = surface_velocity+(f.velocity-surface_velocity)*(-dt * 4.0).exp();
        }
    } else if f.agl() > clearance + 4.0 {
        f.landed = false;
        f.gear_compression = [0.0; 3];
    }
    // A normal accelerometer measures specific force along the aircraft's up
    // axis, not only aerodynamic lift or total coordinate acceleration. Include
    // vectored thrust, suspension, tire and contact-constraint loads, but exclude
    // gravitational and rotating-reference-frame accelerations. Use this step's axis
    // rather than the attitude already advanced above.
    f.velocity += acc * dt;
    f.position += f.velocity * dt;
    // A hard lower bound backs up the compliant landing-gear spring.
    if contact && f.altitude() < ground + clearance - 0.28 {
        f.position = f.position.normalize() * (RADIUS + ground + clearance - 0.28);
        let vn = (f.velocity-crate::carrier::ground_velocity(f,f.position)).dot(up);
        if vn < 0.0 {
            f.velocity -= up * vn * 0.8;
        }
    }
    crate::carrier::after_step(f,previous,dt);
    let measured_acceleration = if dt > 0. {
        (f.velocity - previous_velocity) / dt
    } else {
        acc
    };
    let specific_acceleration = measured_acceleration - gravity - rotational - tidal;
    f.g_load = normal_load(specific_acceleration, previous_rotation);
    let body_load = body_accelerometer(specific_acceleration, previous_rotation);
    f.lateral_g = body_load.x;
    f.longitudinal_g = -body_load.z;
    if f.aircraft == crate::aircraft::Aircraft::F22 && dt > 0. {
        let loads = raptor_elastic_loads(f, b, previous_rotation.conjugate() * aero, &panels);
        update_raptor_wing_failure(f, &loads);
        let specific_body = previous_rotation.conjugate() * specific_acceleration;
        let rate_acceleration = (f.rates - previous_rates) / dt;
        let mass_center = crate::aircraft::mass_center(f);
        let mut config = *crate::structural::f22_config();
        let panel_loads = raptor_elastic_panel_loads(f, &loads, &panels, &elastic_shape, &config);
        let geometry = f.aircraft.panels();
        let angles = crate::aircraft::angles(f);
        // Express rigid-body inertia in each panel's neutral frame. This is a
        // small-deformation approximation: hinge and elastic-frame angular
        // accelerations themselves are not yet included in the inertial term.
        let panel_motion = std::array::from_fn(|i| {
            let Some(parent) = crate::aircraft::raptor_structural_surface(i) else {
                return crate::structural::PanelMotion::default();
            };
            let pivot = geometry[i].center;
            let frame = elastic_shape.frame_rotation(&config, parent, pivot);
            let hinge = crate::aircraft::surface_rotation(f.aircraft, i, angles[i]);
            let inverse = (frame * hinge).conjugate();
            let displaced_pivot = elastic_shape.deform_point(&config, parent, pivot);
            crate::structural::PanelMotion {
                specific_acceleration: inverse * specific_body,
                angular_velocity: inverse * previous_rates,
                angular_acceleration: inverse * rate_acceleration,
                mass_center: pivot + inverse * (mass_center - displaced_pivot),
            }
        });
        for i in 0..config.panels.len() {
            if f.damage.surface_state[i] == crate::combat::damage::SurfaceState::Detached {
                config.panels[i] = None;
                f.structure.panel_modes[i] = Default::default();
            }
        }
        f.structure.advance_with_motion(
            &config,
            &loads,
            specific_body,
            previous_rates,
            rate_acceleration,
            mass_center,
            dt,
        );
        f.structure.advance_panels_with_motion(&config, &panel_loads, &panel_motion, dt);
    }
    crate::collision::check_airframe(f, previous, previous_rotation);
    if !f.landed && !f.crashed {
        let status = if f.autopilot {
            "APPROACH ASSIST  •  FOLLOWING GLIDESLOPE"
        } else if f.stall() {
            "STALL  •  LOWER NOSE AND ADD POWER"
        } else if f.mach > 1.0 {
            "SUPERSONIC FLIGHT"
        } else {
            if f.carrier.active {"CARRIER FLIGHT / H HOOK / ALT+G GUIDANCE"}else{"FREE FLIGHT  •  RUNWAY 18"}
        };
        if f.status != status {
            f.status.clear();
            f.status.push_str(status);
        }
    }
}

/// Signed body XYZ specific force in G; gravity/reference accelerations have
/// already been removed by the caller. This is telemetry only.
fn body_accelerometer(specific_acceleration: DVec3, orientation: DQuat) -> DVec3 {
    orientation.conjugate() * specific_acceleration / 9.80665
}
fn normal_load(specific_acceleration: DVec3, orientation: DQuat) -> f64 {
    specific_acceleration.dot(orientation * DVec3::Y) / 9.80665
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn f22_wing_stress_has_independent_signed_calibrated_failure() {
        use crate::{aircraft::Aircraft, structural::{Load, LEFT_WING, RIGHT_WING}};
        let mut f = Flight::approach();
        crate::aircraft::select(&mut f, Aircraft::F22);
        let reference_force = 0.42 * 15. * 25_700.74 * 9.80665;
        for sign in [-1., 1.] {
            let mut trial = f.clone();
            let loads = [Load { surface: LEFT_WING, point: DVec3::new(-3.9,-0.04,0.6), force: DVec3::Y * (reference_force * sign * 0.999), moment: DVec3::ZERO },
                Load { surface: RIGHT_WING, point: DVec3::new(3.9,-0.04,0.6), force: DVec3::Y * (reference_force * sign * 1.001), moment: DVec3::ZERO }];
            update_raptor_wing_failure(&mut trial, &loads);
            assert!(!trial.damage.wing_broken[0] && trial.damage.wing_broken[1]);
            assert!((trial.damage.wing_stress[0] / 300e6 - sign * 0.999).abs() < 1e-10);
            for i in [5,6,10] { assert_eq!(trial.damage.surface_state[i], crate::combat::damage::SurfaceState::Detached); }
            assert_ne!(trial.damage.surface_state[4], crate::combat::damage::SurfaceState::Detached);
            let before = trial.damage.wing_stress;
            update_raptor_wing_failure(&mut trial, &loads);
            assert_eq!(trial.damage.wing_stress, before);
        }
        f.damage.wings[0] = 0.5;
        let loads = [Load { surface: LEFT_WING, point: DVec3::new(-3.9,0.,0.6), force: DVec3::Y * (reference_force * 0.51), moment: DVec3::ZERO }];
        update_raptor_wing_failure(&mut f, &loads);
        assert!(f.damage.wing_broken[0] && !f.damage.wing_broken[1]);
        crate::aircraft::select(&mut f, Aircraft::Su27);
        assert_eq!(f.damage.wing_broken, [false;2]);
    }
    #[test]
    fn f22_reference_force_reaches_calibrated_stress_through_load_allocation() {
        let mut f = Flight::approach();
        crate::aircraft::select(&mut f, crate::aircraft::Aircraft::F22);
        assert!((f.mass() - 25_700.74).abs() < 0.01);
        f.flow.wing_attached = [1., 1.];
        let panels = crate::control_surfaces::Loads::default();
        for g in [-15., 15.] {
            let mut trial = f.clone();
            let force = DVec3::Y * (g * f.mass() * 9.80665);
            let loads = raptor_elastic_loads(&trial, -DVec3::Z * 200., force, &panels);
            update_raptor_wing_failure(&mut trial, &loads);
            for stress in trial.damage.wing_stress { assert!((stress.abs() / 300e6 - 1.).abs() < 1e-6); }
        }
    }
    #[test]
    fn elastic_load_allocation_uses_baseline_and_panel_increment_once() {
        let mut f = Flight::approach();
        crate::aircraft::select(&mut f, crate::aircraft::Aircraft::F22);
        let mut panels = crate::control_surfaces::Loads::default();
        panels.surfaces[2].force = DVec3::Y * 1200.;
        panels.force = panels.surfaces[2].force;
        let baseline = DVec3::new(2000., 100000., -8000.);
        let loads = raptor_elastic_loads(&f, -DVec3::Z * 200., baseline + panels.force, &panels);
        let net = loads.iter().fold(DVec3::ZERO, |sum, load| sum + load.force);
        // 84% wing + 8% stabilator of baseline lift, 70% fin of baseline
        // side force, and exactly one copy of the control increment.
        assert!(net.abs_diff_eq(DVec3::new(1400., 93200., 0.), 1e-8), "{net:?}");
        assert_eq!(loads[8].surface, crate::structural::LEFT_WING);
        assert_eq!(loads[8].force, panels.surfaces[2].force);
    }
    #[test]
    fn elastic_incidence_is_zero_when_rigid_and_responds_to_twist() {
        let mut f = Flight::approach();
        crate::aircraft::select(&mut f, crate::aircraft::Aircraft::F22);
        let air = -DVec3::Z * 200.;
        assert_eq!(raptor_elastic_incidence(&f, air), (0., 0., 0.));
        // Mirrored span axes require opposite modal signs for equal pitch twist.
        f.structure.surfaces[crate::structural::LEFT_WING].twist = -0.04;
        f.structure.surfaces[crate::structural::RIGHT_WING].twist = 0.04;
        let equal = raptor_elastic_incidence(&f, air);
        assert!(equal.0.abs() > 0.001 && equal.2.abs() < 1e-12, "{equal:?}");
        f.structure.surfaces[crate::structural::RIGHT_WING].twist = 0.;
        let unequal = raptor_elastic_incidence(&f, air);
        assert!(unequal.2.abs() > 0.001, "{unequal:?}");
    }
    #[test]
    fn elastic_state_is_per_aircraft_and_cleared_on_selection() {
        let mut f = Flight::approach();
        crate::aircraft::select(&mut f, crate::aircraft::Aircraft::F22);
        f.structure.surfaces[0].bend = 0.25;
        let copy = f.clone();
        crate::aircraft::select(&mut f, crate::aircraft::Aircraft::Su27);
        assert_eq!(f.structure.surfaces[0].bend, 0.);
        assert_eq!(copy.structure.surfaces[0].bend, 0.25);
        let mut other = Flight::approach();
        simulate(&mut other, &Environment::default(), 1. / 120.);
        assert!(other.structure.surfaces.iter().all(|s| s.bend == 0. && s.twist == 0.));
    }
    #[test]
    fn child_panel_shape_changes_load_and_rigid_shape_matches_old_model() {
        let damage = AircraftDamage::default();
        let air = DVec3::new(0., -25., -220.);
        let command = DVec3::new(0.35, 0., 0.3);
        let rigid = crate::structural::State::default();
        let old = crate::aircraft::panel_loads(crate::aircraft::Aircraft::F22, air, DVec3::ZERO, 0.9, 0.7, command, 0., false, &damage);
        let same = crate::aircraft::panel_loads_structural(crate::aircraft::Aircraft::F22, air, DVec3::ZERO, 0.9, 0.7, command, 0., false, &damage, &rigid);
        assert!(old.force.abs_diff_eq(same.force, 1e-8));
        assert!(old.moment.abs_diff_eq(same.moment, 1e-8));
        let mut flexible = rigid;
        flexible.panel_modes[2].twist = 0.1;
        flexible.panel_modes[2].bend_velocity = 0.3;
        let changed = crate::aircraft::panel_loads_structural(crate::aircraft::Aircraft::F22, air, DVec3::ZERO, 0.9, 0.7, command, 0., false, &damage, &flexible);
        assert!(changed.surfaces[2].force.distance(same.surfaces[2].force) > 1.);
    }
    #[test]
    fn elastic_flight_stays_finite_and_converges_at_high_pressure() {
        let mut initial = Flight::approach();
        crate::aircraft::select(&mut initial, crate::aircraft::Aircraft::F22);
        initial.position = surface_point(0., 0., 3000.);
        initial.orientation = DQuat::IDENTITY;
        initial.velocity = -DVec3::Z * 340.;
        initial.rates = DVec3::ZERO;
        initial.assist = false;
        initial.flaps = 0.;
        initial.controls = DVec3::new(0.1, 0., 0.);
        initial.infinite_fuel = true;
        let mut coarse = initial.clone();
        let mut fine = initial;
        let mut e = Environment::default();
        e.wind_enabled = false;
        for _ in 0..360 { simulate(&mut coarse, &e, 1. / 120.); }
        for _ in 0..720 { simulate(&mut fine, &e, 1. / 240.); }
        assert!(!coarse.crashed && !fine.crashed, "{} / {}", coarse.status, fine.status);
        for mode in coarse.structure.surfaces.iter().chain(coarse.structure.panel_modes.iter()) {
            assert!(mode.bend.is_finite() && mode.twist.is_finite() && mode.bend_velocity.is_finite() && mode.twist_velocity.is_finite());
            assert!(mode.bend.abs() < 5. && mode.twist.abs() < 1., "{mode:?}");
        }
        assert!(coarse.position.distance(fine.position) < 5., "position difference {}", coarse.position.distance(fine.position));
        assert!(coarse.rates.distance(fine.rates) < 0.1, "rate difference {}", coarse.rates.distance(fine.rates));
        assert!((coarse.structure.surfaces[0].bend - fine.structure.surfaces[0].bend).abs() < 0.05);
    }
    #[test]
    fn detached_elastic_panel_has_no_load_or_retained_modes() {
        let mut f = Flight::approach();
        crate::aircraft::select(&mut f, crate::aircraft::Aircraft::F22);
        f.damage.surface_state[2] = crate::combat::damage::SurfaceState::Detached;
        f.structure.panel_modes[2].bend = 0.2;
        f.structure.panel_modes[2].twist = 0.03;
        let panels = crate::control_surfaces::Loads::default();
        let assemblies = raptor_elastic_loads(&f, -DVec3::Z * 200., DVec3::Y * 100000., &panels);
        let panel_loads = raptor_elastic_panel_loads(&f, &assemblies, &panels, &f.structure, crate::structural::f22_config());
        assert_eq!(panel_loads[2].force, DVec3::ZERO);
        simulate(&mut f, &Environment::default(), 1. / 120.);
        assert_eq!(f.structure.panel_modes[2].bend, 0.);
        assert_eq!(f.structure.panel_modes[2].twist, 0.);
    }
    #[test]
    fn baseline_load_drives_all_attached_f22_panel_modes() {
        let mut f = Flight::approach();
        crate::aircraft::select(&mut f, crate::aircraft::Aircraft::F22);
        let panels = crate::control_surfaces::Loads::default();
        let assemblies = raptor_elastic_loads(&f, -DVec3::Z * 200., DVec3::new(30000., 100000., 0.), &panels);
        let config = crate::structural::f22_config();
        let loads = raptor_elastic_panel_loads(&f, &assemblies, &panels, &f.structure, config);
        f.structure.advance_panels(config, &loads, DVec3::ZERO, DVec3::ZERO, DVec3::ZERO, DVec3::ZERO, 1. / 120.);
        for i in [0_usize, 1, 2, 3, 4, 5, 6, 7, 9, 10] {
            assert!(f.structure.panel_modes[i].bend.abs() > 1e-10, "panel {i} received no bending response");
        }
    }
    #[test]
    fn flexible_aircraft_substeps_preserve_total_duration() {
        let mut coarse = Flight::approach();
        crate::aircraft::select(&mut coarse, crate::aircraft::Aircraft::F22);
        let mut direct = coarse.clone();
        let e = Environment { wind_enabled: false, ..default() };
        let start = coarse.elapsed;
        simulate(&mut coarse, &e, 1. / 60.);
        for _ in 0..4 { simulate(&mut direct, &e, 1. / 240.); }
        assert!((coarse.elapsed - start - 1. / 60.).abs() < 1e-12);
        assert!(coarse.position.abs_diff_eq(direct.position, 1e-9));
        assert!(coarse.rates.abs_diff_eq(direct.rates, 1e-12));
    }
    #[test]
    fn normal_accelerometer_is_signed_body_axis_specific_force() {
        assert_eq!(normal_load(DVec3::ZERO, DQuat::IDENTITY), 0.);
        assert!((normal_load(DVec3::Y * 9.80665, DQuat::IDENTITY) - 1.).abs() < 1e-12);
        assert!((normal_load(-DVec3::Y * 19.6133, DQuat::IDENTITY) + 2.).abs() < 1e-12);
        let attitude = DQuat::from_rotation_z(0.7) * DQuat::from_rotation_x(-0.3);
        let force = attitude * DVec3::new(200., 3. * 9.80665, -100.);
        assert!((normal_load(force, attitude) - 3.).abs() < 1e-12);
    }
    #[test]
    fn three_axis_accelerometer_preserves_aircraft_axes_and_forward_sign() {
        for q in [DQuat::IDENTITY,DQuat::from_rotation_z(1.2)*DQuat::from_rotation_y(-0.7)*DQuat::from_rotation_x(0.4)] {
            let body=DVec3::new(-0.7,-1.5,-2.0);
            let measured=body_accelerometer(q*(body*9.80665),q);
            assert!((measured-body).length()<1e-12);
            assert!((-measured.z-2.).abs()<1e-12);
            let reverse=body_accelerometer(q*DVec3::Z*9.80665,q);
            assert!((-reverse.z+1.).abs()<1e-12);
        }
    }
    #[test]
    fn parked_aircraft_g_meter_includes_wheel_support() {
        let mut f = Flight::runway();
        let e = Environment { wind_enabled: false, ..default() };
        let mut sum = 0.;
        for i in 0..1200 {
            simulate(&mut f, &e, 1. / 120.);
            assert!(!f.crashed);
            if i >= 1080 { sum += f.g_load; }
        }
        assert!((sum / 120. - 1.).abs() < 0.04, "parked G {}", sum / 120.);
    }
    #[test]
    fn g_meter_includes_actual_vectored_engine_normal_force() {
        let mut powered = crate::combat::flight();
        crate::aircraft::select(&mut powered, crate::aircraft::Aircraft::F22);
        powered.position = surface_point(0., 0., 6000.);
        powered.orientation = DQuat::IDENTITY;
        powered.velocity = -DVec3::Z * 200.;
        powered.assist = false;
        powered.infinite_fuel = true;
        powered.spool = 1.;
        powered.throttle = 1.;
        powered.surface = -DVec3::X * 0.5;
        powered.controls = powered.surface;
        powered.systems.nozzle = [10_f64.to_radians(); 2];
        let mut unpowered = powered.clone();
        unpowered.engine = false;
        let e = Environment { wind_enabled: false, ..default() };
        // Compare one actual coupled substep, while the airframes still have
        // identical flow, attitude, control and aerodynamic loads. At a larger
        // public step their trajectories diverge before the final G sample.
        simulate(&mut powered, &e, 1. / 240.);
        simulate(&mut unpowered, &e, 1. / 240.);
        let expected = powered.thrust * powered.systems.nozzle[0].sin() / (powered.mass() * 9.80665);
        assert!(expected > 0.1);
        assert!((powered.g_load - unpowered.g_load - expected).abs() < 1e-8);
    }
    #[test]
    fn exact_solar_caches_follow_direct_phase_edits_and_alternating_environments() {
        let bits = |v: DVec3| v.to_array().map(f64::to_bits);
        for i in 0..500 {
            let mut e = Environment::default();
            e.day_phase = i as f64 * 0.027 - 7.;
            e.orbit_phase = i as f64 * 0.019 - 3.;
            for phase in [0., 1.1, -0.7] {
                e.orbit_phase += phase;
                assert_eq!(bits(e.planet_orbit()), bits(e.planet_orbit_uncached()));
                assert_eq!(bits(e.sun_direction()), bits(e.sun_direction_uncached()));
                assert_eq!(bits(e.sun_direction()), bits(e.sun_direction_uncached()));
                e.day_phase += 0.001;
                assert_eq!(bits(e.sun_direction()), bits(e.sun_direction_uncached()));
                let other = Environment::default();
                assert_eq!(
                    bits(other.sun_direction()),
                    bits(other.sun_direction_uncached())
                );
            }
        }
    }
    #[test]
    fn attitude_readouts_handle_inversion_and_polar_positions() {
        let mut f = Flight::runway();
        assert!((f.heading() - 184.59).abs() < 0.1);
        f.orientation = DQuat::from_rotation_z(std::f64::consts::PI);
        assert!((f.roll().abs() - std::f64::consts::PI).abs() < 1e-6);
        f.position = rotation().normalize() * (RADIUS + 1000.0);
        let basis = crate::world::frame(&f);
        assert!(basis.is_finite() && f.heading().is_finite());
        assert!((basis.length() - 1.0).abs() < 1e-10);
    }
    #[test]
    fn atmosphere_layers_are_continuous_and_pressure_falls() {
        for geopotential in [
            11000.0, 20000.0, 32000.0, 47000.0, 51000.0, 71000.0, 84852.0,
        ] {
            let h = 6356766.0 * geopotential / (6356766.0 - geopotential);
            let (tl, pl, _, _) = atmosphere(h - 0.001);
            let (tr, pr, _, _) = atmosphere(h + 0.001);
            assert!((tl - tr).abs() < 0.001 && (pl - pr).abs() / pl < 1e-5);
        }
        let mut previous = 101326.0;
        for h in (0..=120000).step_by(100) {
            let (t, p, rho, sound) = atmosphere(h as f64);
            assert!(t > 0.0 && p >= 0.0 && p <= previous && rho >= 0.0 && sound > 0.0);
            previous = p;
        }
        assert_eq!(atmosphere(220000.0).2, 0.0);
        let h = 6356766.0 * 20000.0 / (6356766.0 - 20000.0);
        let (t, p, _, _) = atmosphere(h);
        assert!((t - 216.65).abs() < 0.01 && (p - 5474.89).abs() < 0.5);
    }
    #[test]
    fn full_throttle_takeoff_clears_airfield() {
        let mut f = Flight::runway();
        let mut e = Environment::default();
        e.wind_enabled = false;
        f.throttle = 1.0;
        f.infinite_fuel = true;
        let mut airborne = false;
        for _ in 0..7200 {
            f.controls.x = if f.airspeed > 85.0 {
                ((0.18 - f.pitch()) * 2.0).clamp(-0.4, 0.4)
            } else {
                0.0
            };
            if !f.landed && f.agl() > 15.0 {
                f.gear = false;
                f.flaps = 0.0;
                airborne = true;
            }
            e.advance(1.0 / 120.0);
            simulate(&mut f, &e, 1.0 / 120.0);
            if f.crashed {
                break;
            }
        }
        assert!(
            airborne && !f.crashed,
            "{} speed={} agl={} elapsed={} airborne={}",
            f.status,
            f.airspeed,
            f.agl(),
            f.elapsed,
            airborne
        );
    }
    #[test]
    fn sas_prograde_uses_rcs_to_align_without_main_engine() {
        let mut f = Flight::spacecraft();
        let mut e = Environment::default();
        f.orientation = DQuat::from_rotation_x(0.8);
        f.sas_mode = 1;
        f.assist = true;
        let initial = f
            .forward()
            .dot((f.velocity + rotation().cross(f.position)).normalize());
        let mut used_rcs = false;
        for _ in 0..7200 {
            e.advance(1.0 / 120.0);
            simulate(&mut f, &e, 1.0 / 120.0);
            used_rcs |= f.rcs_torque.length() > 1.0;
        }
        let final_alignment = f
            .forward()
            .dot((f.velocity + rotation().cross(f.position)).normalize());
        assert!(used_rcs && initial < 0.5 && final_alignment > 0.999);
        assert_eq!(f.thrust, 0.0);
        assert!(
            !f.crashed
                && f.space.rcs_fuel < 400.
                && f.fuel == crate::space::Stage::Transfer.capacity()
        );
    }
    #[test]
    fn sea_level_atmosphere() {
        let (t, p, r, a) = atmosphere(0.0);
        assert!((t - 288.15).abs() < 1e-8);
        assert!((p - 101325.0).abs() < 1.0);
        assert!((r - 1.225).abs() < 0.002);
        assert!((a - 340.3).abs() < 0.2);
    }
    #[test]
    fn spherical_surface() {
        let p = surface_point(10000.0, -8000.0, 300.0);
        assert!((p.length() - RADIUS - 300.0).abs() < 1e-6);
    }
    #[test]
    fn orbital_period() {
        let mut e = Environment::default();
        e.advance(YEAR);
        assert!(e.orbit_phase.abs() < 1e-8);
        assert!((e.planet_orbit().length() - AU * (1.0 - 0.0167)).abs() < 0.1);
    }
    #[test]
    fn flight_stays_finite() {
        let mut f = Flight::approach();
        let mut e = Environment::default();
        for _ in 0..7200 {
            e.advance(1.0 / 120.0);
            simulate(&mut f, &e, 1.0 / 120.0);
            assert!(f.position.is_finite() && f.velocity.is_finite() && f.orientation.is_finite());
        }
        assert!((f.orientation.length() - 1.0).abs() < 1e-8);
    }
    #[test]
    fn no_thrust_without_fuel() {
        let mut f = Flight::approach();
        f.fuel = 0.0;
        f.throttle = 1.0;
        simulate(&mut f, &Environment::default(), 0.01);
        assert_eq!(f.thrust, 0.0);
    }
    #[test]
    fn engine_damage_reduces_thrust_and_failed_avionics_cannot_enable_fbw() {
        let mut intact = Flight::approach();
        intact.infinite_fuel = true;
        let mut damaged = intact.clone();
        damaged.damage.engine = 0.45;
        damaged.damage.avionics = 0.1;
        simulate(&mut intact, &Environment::default(), 0.01);
        simulate(&mut damaged, &Environment::default(), 0.01);
        assert!((damaged.thrust / intact.thrust - 0.45).abs() < 1e-10);
        assert!(!damaged.assist);
    }
    #[test]
    fn asymmetric_wing_damage_changes_six_dof_roll_and_lift() {
        let mut intact = crate::combat::flight();
        intact.assist = false;
        intact.controls = DVec3::ZERO;
        let mut damaged = intact.clone();
        damaged.damage.wings[0] = 0.35;
        let mut environment = Environment::default();
        environment.wind_enabled = false;
        for _ in 0..120 {
            simulate(&mut intact, &environment, 1.0 / 120.0);
            simulate(&mut damaged, &environment, 1.0 / 120.0);
        }
        assert!(!intact.crashed && !damaged.crashed);
        assert!(damaged.rates.z > intact.rates.z + 0.001);
        assert!(damaged.g_load < intact.g_load);
    }
    #[test]
    fn jammed_and_detached_controls_change_real_player_dynamics() {
        let mut healthy = crate::combat::flight();
        healthy.assist = false;
        healthy.controls = DVec3::ZERO;
        let mut jammed = healthy.clone();
        let mut detached = healthy.clone();
        jammed.damage.surface_state[2] = crate::combat::damage::SurfaceState::Jammed(0.3);
        detached.damage.surface_state[2] = crate::combat::damage::SurfaceState::Detached;
        detached.damage.surfaces[2] = 0.0;
        let mut e = Environment::default();
        e.wind_enabled = false;
        for _ in 0..120 {
            simulate(&mut healthy, &e, 1.0 / 120.0);
            simulate(&mut jammed, &e, 1.0 / 120.0);
            simulate(&mut detached, &e, 1.0 / 120.0);
        }
        assert!(jammed.rates.distance(healthy.rates) > 0.02);
        assert!(detached.velocity.distance(healthy.velocity) > 0.01);
        assert!(!jammed.crashed && !detached.crashed);
        assert_eq!(
            jammed.damage.surface_state[2],
            crate::combat::damage::SurfaceState::Jammed(0.3)
        );
    }
    #[test]
    fn shut_down_afterburner_consumes_no_fuel_with_stale_spool() {
        let mut f = Flight::approach();
        f.engine = false;
        f.throttle = 1.0;
        f.spool = 1.0;
        let fuel = f.fuel;
        simulate(&mut f, &Environment::default(), 0.01);
        assert_eq!(f.thrust, 0.0);
        assert_eq!(f.fuel, fuel);
    }
    #[test]
    fn braking_stops_ground_roll() {
        let mut f = Flight::runway();
        f.velocity = -DVec3::Z * 25.0;
        f.brakes = true;
        let e = Environment::default();
        for _ in 0..2400 {
            simulate(&mut f, &e, 1.0 / 120.0);
        }
        assert!(f.velocity.length() < 3.0, "{}", f.velocity.length());
    }
}

fn simulate_spacecraft(f: &mut Flight, e: &Environment, dt: f64) {
    crate::space::simulate(f, e, dt);
}

#[cfg(test)]
mod approach_regression {
    use super::*;
    #[test]
    fn updated_aero_lands_and_stops() {
        let mut f = Flight::approach();
        let mut e = Environment::default();
        e.wind_enabled = false;
        f.autopilot = true;
        for _ in 0..14400 {
            e.advance(1.0 / 120.0);
            simulate(&mut f, &e, 1.0 / 120.0);
        }
        assert!(
            !f.crashed && f.landed,
            "{}; position={:?}, speed={}, sink={}, pitch={}, bank={}",
            f.status,
            f.position,
            f.velocity.length(),
            f.touchdown_sink,
            f.pitch(),
            f.roll()
        );
        assert!(f.velocity.length() < 1.0);
        assert!(f.position.z.abs() < RUNWAY_HALF);
        assert!(f.touchdown_sink < 3.0);
    }
}
