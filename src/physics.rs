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
    pub aircraft: crate::aircraft::Aircraft,
    pub systems: crate::aircraft::Systems,
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
            aircraft: crate::aircraft::Aircraft::Gripen,
            systems: crate::aircraft::Systems::default(),
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
    pub fn mass(&self) -> f64 {
        if self.vehicle == Vehicle::Gripen {
            self.aircraft.spec().empty + self.fuel + crate::aircraft::stores_mass(self)
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
                terrain_height(self.position.x, self.position.z).max(0.)
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

pub fn simulate(f: &mut Flight, e: &Environment, dt: f64) {
    let _measure = crate::performance::span(1);
    if f.paused {
        return;
    }
    if f.crashed {
        if f.vehicle == Vehicle::Spacecraft {
            crate::space::wreck(f, dt);
            return;
        }
        crate::collision::simulate_wreck(f, dt);
        return;
    }
    let spec = f.aircraft.spec();
    let custom = f.aircraft != crate::aircraft::Aircraft::Gripen;
    let wing_area = spec.area;
    let previous = f.position;
    let previous_rotation = f.orientation;
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
    let evaluate=if f.aircraft==crate::aircraft::Aircraft::F22{crate::aerodynamics::evaluate_raptor}else{crate::aerodynamics::evaluate};
    let mut coefficients = evaluate(
        crate::aerodynamics::Conditions {
            alpha,
            beta,
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
    if custom {
        let lift_gain = match f.aircraft {crate::aircraft::Aircraft::Su27=>1.10,crate::aircraft::Aircraft::F22=>1.08,crate::aircraft::Aircraft::Hornet=>1.15,_=>1.0};
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
        coefficients.pitch=0.004-static_margin*alpha.sin()-post_stall*(1.-f.flow.attached)*alpha.sin()-damping*f.rates.x*c/(2.*speed.max(15.));
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
    let (installed_thrust, burner) = if custom {crate::aircraft::jet_output(f.aircraft,f.spool,rho,f.mach)}else{crate::propulsion::jet_output(f.spool, rho, f.mach)};
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
        let fuel_rate = if f.thrust > 0.0 {
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
    if f.assist && !f.autopilot && !f.landed {
        // FBW protects the usable game envelope; raw controls remain available
        // with T. Predict incidence using pitch rate to avoid waiting for stall.
        let predicted_alpha = alpha + f.rates.x * 0.15;
        if command.x > 0.0 {
            let alpha_margin = ((spec.alpha - predicted_alpha) / 0.10).clamp(0.0, 1.0);
            let load_margin = ((spec.g - previous_g_load) / 2.0).clamp(0.0, 1.0);
            command.x *= if f.aircraft==crate::aircraft::Aircraft::F22{load_margin}else{alpha_margin.min(load_margin)};
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
        let approach_speed=if f.aircraft==crate::aircraft::Aircraft::F22{106.}else{82.};
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
    let span = spec.span;
    let chord = if custom {spec.area/spec.span}else{3.5};
    let (damage_force, damage_moment) =
        crate::combat::damage::aero(&f.damage, b, q, coefficients.lift);
    aero += f.orientation * damage_force;
    acc += f.orientation * damage_force / f.mass();
    f.g_load = aero.dot(f.orientation * DVec3::Y) / (f.mass() * 9.80665);
    let natural = DVec3::new(
        q * wing_area * chord * coefficients.pitch,
        q * wing_area * span * coefficients.yaw,
        q * wing_area * span * coefficients.roll,
    ) + damage_moment;
    let panel_model = crate::control_surfaces::PreparedPanels::new(
        b,
        f.rates,
        rho,
        coefficients.control_scale,
        f.mach,
        &f.damage,
    );
    let panel_loads = |actuator| {
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
        let derivative = |axis: DVec3| {
            (panel_loads(axis * 0.02).moment - panel_loads(-axis * 0.02).moment) / 0.04
        };
        let vector = if f.aircraft==crate::aircraft::Aircraft::F22{crate::aircraft::raptor::vector_jacobian(f)}else{DMat3::ZERO};
        let mut panel_matrix=DMat3::from_cols(derivative(DVec3::X),derivative(DVec3::Y),derivative(DVec3::Z));
        let mut panel_bias=neutral_panels.moment;
        if f.aircraft==crate::aircraft::Aircraft::F22 {
            // Roll effectiveness changes strongly at large mixed stabilator
            // deflections. Update that row and its fixed-load bias at the
            // current state, preserving the tested pitch/yaw allocation model.
            let roll_slope=|axis:DVec3| (panel_loads(f.surface+axis*0.02).moment.z-panel_loads(f.surface-axis*0.02).moment.z)/0.04;
            panel_matrix.x_axis.z=roll_slope(DVec3::X);
            panel_matrix.y_axis.z=roll_slope(DVec3::Y);
            panel_matrix.z_axis.z=roll_slope(DVec3::Z);
            panel_bias.z=panel_loads(f.surface).moment.z-(panel_matrix*f.surface).z;
        }
        let jacobian = panel_matrix+vector;
        let vector_bias=if f.aircraft==crate::aircraft::Aircraft::F22{engine_moment-vector*f.surface}else{DVec3::ZERO};
        crate::control_surfaces::allocate(
            jacobian,
            (target - f.rates) * 5.0 * inertia - natural - panel_bias-vector_bias,
        )
    } else {
        DVec3::new(command.x, command.y, -command.z)
    };
    let request = if f.battery { request } else { DVec3::ZERO };
    let request = request.clamp(DVec3::splat(-1.0), DVec3::splat(1.0));
    if f.damage.hydraulics > 0.12 {
        f.surface = crate::control_surfaces::advance_actuator(
            f.surface,
            request,
            q,
            dt * f.damage.hydraulics,
        );
    }
    let panels = panel_loads(f.surface);
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
    f.g_load = aero.dot(f.orientation * DVec3::Y) / (f.mass() * 9.80665);
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
    let angular_acc = (torque - f.rates.cross(inertia * f.rates)) / inertia;
    f.g_load = aero.dot(f.orientation * DVec3::Y) / (f.mass() * 9.80665);
    f.rates += angular_acc * dt;
    f.orientation = (f.orientation * DQuat::from_scaled_axis(f.rates * dt)).normalize();
    if f.g_load.abs() > spec.g+0.5 {
        f.integrity -= (f.g_load.abs() - spec.g - 0.5) * dt * 0.035;
    }
    if f.mach > spec.limit_mach {
        f.integrity -= (f.mach - spec.limit_mach) * dt * 0.08;
    }
    if f.integrity <= 0.0 {
        crate::collision::destroy(f, "STRUCTURAL FAILURE", up);
        return;
    }

    let ground = terrain_height(f.position.x, f.position.z).max(0.0);
    let clearance = crate::collision::wheel_clearance(f);
    let contact = f.altitude() < ground + clearance
        || (f.agl() < clearance + 12. && crate::collision::wheel_contact(f));
    if contact {
        let normal = crate::terrain::normal(f.position);
        let sink = f.velocity.dot(normal);
        let on_runway = crate::terrain::paved(f.position.x, f.position.z);
        if !f.landed {
            f.touchdown_sink = -sink;
            if f.gear_extension < 0.98
                || sink < -5.5
                || (f.orientation * DVec3::X).dot(normal).abs() > 0.20_f64.sin()
                || normal.dot(up) < 12_f64.to_radians().cos()
                || in_lake(f.position.x, f.position.z)
            {
                for health in &mut f.gear_health {
                    *health = (*health - ((-sink - 3.0).max(0.0) / 3.0).powi(2)).max(0.0);
                }
                crate::collision::destroy(
                    f,
                    if in_lake(f.position.x, f.position.z) {
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
                if on_runway { "RUNWAY" } else { "OFF-FIELD" },
                -sink,
                score
            );
        }

        let (suspension, angular) = crate::collision::suspension(f, dt);
        let normal = suspension.dot(up);
        acc += suspension;
        f.rates += angular * dt;
        let surface_normal = crate::terrain::normal(f.position);
        let tangent = f.velocity - surface_normal * f.velocity.dot(surface_normal);
        let braking = if f.brakes || f.autopilot {
            if on_runway { 0.55 } else { 0.30 }
        } else if on_runway {
            0.015
        } else {
            0.038
        };
        acc -= tangent.normalize_or_zero() * (normal + 9.81) * braking;
        // Tire lateral grip with a mild nose-wheel steering response.
        acc -= right * (f.velocity.dot(right) * 2.0).clamp(-4.0, 4.0);
        f.rates.y += command.y * dt * 0.6;
        if speed < 2.0 && f.throttle < 0.1 {
            f.velocity *= (-dt * 4.0).exp();
        }
    } else if f.agl() > clearance + 4.0 {
        f.landed = false;
        f.gear_compression = [0.0; 3];
    }
    f.velocity += acc * dt;
    f.position += f.velocity * dt;
    // A hard lower bound backs up the compliant landing-gear spring.
    if contact && f.altitude() < ground + clearance - 0.28 {
        f.position = f.position.normalize() * (RADIUS + ground + clearance - 0.28);
        let vn = f.velocity.dot(up);
        if vn < 0.0 {
            f.velocity -= up * vn * 0.8;
        }
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
            "FREE FLIGHT  •  RUNWAY 18"
        };
        if f.status != status {
            f.status.clear();
            f.status.push_str(status);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
