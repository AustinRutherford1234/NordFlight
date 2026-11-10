//! Immutable planet and surface authority shared by simulation and terrain jobs.
pub mod config;
pub mod graphics;
pub(crate) mod relief_shadow;
pub(crate) mod shadows;
pub(crate) mod ground_cover;
pub mod legacy;
pub mod pack;
#[cfg(test)]
mod tests;
pub mod verify;
use bevy::math::{DMat3, DVec3};
pub use config::*;
use std::{
    path::PathBuf,
    sync::{Arc, OnceLock},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LandCover {
    Trees,
    Shrub,
    Grass,
    Cropland,
    Built,
    Rock,
    Snow,
    Water,
    Wetland,
}
impl LandCover {
    pub fn from_code(code: u8) -> Self {
        match code {
            10 => Self::Trees,
            20 => Self::Shrub,
            40 => Self::Cropland,
            50 => Self::Built,
            60 => Self::Rock,
            70 => Self::Snow,
            80 => Self::Water,
            90 | 95 => Self::Wetland,
            _ => Self::Grass,
        }
    }
    pub fn palette(self) -> usize {
        match self {
            Self::Trees => 0,
            Self::Grass | Self::Shrub => 1,
            Self::Cropland => 2,
            Self::Rock => 3,
            Self::Water => 4,
            Self::Snow => 5,
            Self::Wetland => 6,
            Self::Built => 7,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct SurfaceSample {
    pub ground_height: f64,
    pub normal: DVec3,
    pub land_cover: LandCover,
    pub water_surface: Option<f64>,
    pub detailed: bool,
}
#[derive(Debug)]
pub struct EnvironmentSnapshot {
    pub preset: EnvironmentPreset,
    pub data: pack::TerrainData,
    pub graphics: GraphicsSettings,
    pub revision: u64,
}
static ACTIVE: OnceLock<Arc<EnvironmentSnapshot>> = OnceLock::new();
pub fn active() -> &'static Arc<EnvironmentSnapshot> {
    ACTIVE.get_or_init(|| Arc::new(EnvironmentSnapshot::legacy()))
}
pub fn planet_radius() -> f64 {
    radius()
}
pub fn planet_mu() -> f64 {
    mu()
}
pub fn radius() -> f64 {
    active().preset.planet.radius
}
pub fn mu() -> f64 {
    active().preset.planet.mu
}
pub fn real() -> bool {
    !active().preset.legacy
}
pub fn initialize() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    let arg = |key: &str| {
        args.iter()
            .position(|a| a == key)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let name = arg("--environment")
        .or_else(|| std::env::var("NORD_ENVIRONMENT").ok())
        .unwrap_or("earth".into());
    let mut graphics = if arg("--graphics")
        .or_else(|| std::env::var("NORD_GRAPHICS").ok())
        .as_deref()
        == Some("high")
    {
        GraphicsSettings::high()
    } else {
        GraphicsSettings::default()
    };
    if arg("--graphics").is_none() && std::env::var_os("NORD_GRAPHICS").is_none() {
        if let Some(path) = graphics::settings_path() {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(saved) = serde_json::from_slice::<GraphicsSettings>(&bytes) {
                    if saved.validate().is_ok() {
                        graphics = saved;
                    }
                }
            }
        }
    }
    if let Ok(path) = std::env::var("NORD_GRAPHICS_CONFIG") {
        graphics = serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    }
    if let Some(scale) = arg("--render-scale").or_else(|| std::env::var("NORD_RENDER_SCALE").ok()) {
        graphics.render_scale = scale.parse().map_err(|_| "invalid render scale")?;
    }
    graphics.validate()?;
    let snapshot = if name == "legacy" {
        let mut s = EnvironmentSnapshot::legacy();
        s.graphics = graphics;
        s
    } else {
        let root = PathBuf::from(crate::start_screen::asset_path()).join("environment");
        let path = if name.ends_with(".json") {
            PathBuf::from(name)
        } else {
            root.join("presets").join(format!("{name}.json"))
        };
        let preset: EnvironmentPreset =
            serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        preset.validate()?;
        let data = pack::TerrainData::load(
            &root.join("packs"),
            &preset.packs,
            graphics.cpu_cache_mib * 1024 * 1024,
        )?;
        if data.global.is_empty() {
            return Err(
                "global terrain pack missing; run scripts/environment/prepare.py --pack global"
                    .into(),
            );
        }
        let revision = preset.revision;
        EnvironmentSnapshot {
            preset,
            data,
            graphics,
            revision,
        }
    };
    eprintln!(
        "Environment: {} / {} MiB terrain / {} region packs",
        snapshot.preset.label,
        snapshot.data.bytes / 1024 / 1024,
        snapshot.data.manifests.len()
    );
    ACTIVE
        .set(Arc::new(snapshot))
        .map_err(|_| "environment already initialized; restart flight process".into())
}
impl EnvironmentSnapshot {
    pub fn legacy() -> Self {
        let mut preset: EnvironmentPreset =
            serde_json::from_str(include_str!("../../assets/environment/presets/earth.json"))
                .unwrap();
        preset.legacy = true;
        preset.id = "legacy".into();
        preset.label = "LEGACY TERRAIN".into();
        Self {
            preset,
            data: Default::default(),
            graphics: GraphicsSettings {
                render_scale: 1.,
                ..Default::default()
            },
            revision: 1,
        }
    }
    pub fn sample_surface(&self, p: DVec3) -> SurfaceSample {
        let (h, c, w, d) = self.raw(p);
        SurfaceSample {
            ground_height: h,
            normal: self.surface_normal(p),
            land_cover: c,
            water_surface: w,
            detailed: d,
        }
    }
    /// The same four neighbouring samples used by `sample_surface`, exposed
    /// separately so placement can reject unsuitable land cover first.
    pub(crate) fn surface_normal(&self, p: DVec3) -> DVec3 {
        let right = if p.normalize().dot(DVec3::X).abs() < 0.9 {
            DVec3::X
        } else {
            DVec3::Z
        };
        let up = p.normalize_or(DVec3::Y);
        let tx = right.cross(up).normalize();
        let tz = tx.cross(up).normalize();
        let delta = 30.;
        let hx =
            self.raw(
                (up + tx * (delta / self.preset.planet.radius)).normalize()
                    * self.preset.planet.radius,
            )
            .0 - self
                .raw(
                    (up - tx * (delta / self.preset.planet.radius)).normalize()
                        * self.preset.planet.radius,
                )
                .0;
        let hz =
            self.raw(
                (up + tz * (delta / self.preset.planet.radius)).normalize()
                    * self.preset.planet.radius,
            )
            .0 - self
                .raw(
                    (up - tz * (delta / self.preset.planet.radius)).normalize()
                        * self.preset.planet.radius,
                )
                .0;
        (up - tx * hx / (2. * delta) - tz * hz / (2. * delta)).normalize()
    }
    pub fn raw(&self, p: DVec3) -> (f64, LandCover, Option<f64>, bool) {
        if self.preset.legacy {
            let h = legacy::height(p.x, p.z);
            let wet = crate::physics::in_lake(p.x, p.z) || crate::carrier::legacy_sea(p.x, p.z);
            return (
                h,
                if wet {
                    LandCover::Water
                } else {
                    LandCover::Grass
                },
                wet.then_some(0.),
                false,
            );
        }
        let (lon, lat) = local_to_geo(p);
        let (source_height, code, detailed) = self.data.sample(lon, lat);
        let height =
            source_height * self.preset.terrain.relief_scale + self.preset.terrain.height_offset;
        let cover = if height < self.preset.planet.sea_level {
            LandCover::Water
        } else {
            LandCover::from_code(code)
        };
        let water = (cover == LandCover::Water).then_some(
            if source_height < 2. || height < self.preset.planet.sea_level {
                self.preset.planet.sea_level
            } else {
                height
            },
        );
        let h = if let Some(w) = water {
            height.min(w - 3.)
        } else {
            height
        };
        // Airport pavement is a local civil-engineering edit, never a broad
        // fictional hill or sea carve. The runway elevation is shared with contacts.
        let edge = if p.y > 0. {
            let runway = (p.x.abs() - 75.).max(0.).hypot((p.z.abs() - 1250.).max(0.));
            let apron = ((p.x - 175.).abs() - 130.)
                .max(0.)
                .hypot(((p.z - 550.).abs() - 500.).max(0.));
            runway.min(apron)
        } else {
            f64::INFINITY
        };
        let t = (edge / 120.).clamp(0., 1.);
        let blend = t * t * (3. - 2. * t);
        let h = crate::physics::RUNWAY_HEIGHT * (1. - blend) + h * blend;
        (
            h,
            if edge < 1. { LandCover::Grass } else { cover },
            if edge < 120. { None } else { water },
            detailed,
        )
    }
    pub fn height_xz(&self, x: f64, z: f64) -> f64 {
        if self.preset.legacy {
            return legacy::height(x, z);
        }
        // Cartesian x/z are retained when a render vertex is lifted. Solve
        // its radial elevation at that lifted geographic coordinate, rather
        // than sampling latitude at sea level and shifting mountain geometry.
        let mut height = 0.;
        for _ in 0..8 {
            let next = self.raw(point(x, z, height, self.preset.planet.radius)).0;
            if (next - height).abs() < 0.0001 {
                return next;
            }
            height = next;
        }
        height
    }
}
pub fn point(x: f64, z: f64, h: f64, r: f64) -> DVec3 {
    DVec3::new(x, ((r + h).powi(2) - x * x - z * z).max(0.).sqrt(), z)
}
fn geographic_frame() -> DMat3 {
    // The geographic reference is fixed for every planet and terrain worker.
    // Retain the computed matrix rather than rebuilding its trigonometry for
    // each height, normal, colour and texture-coordinate query.
    static FRAME: OnceLock<DMat3> = OnceLock::new();
    *FRAME.get_or_init(|| {
        let b = 58.428333_f64.to_radians();
        let l = 12.711111_f64.to_radians();
        let a = 184.59_f64.to_radians();
        let up = DVec3::new(b.cos() * l.cos(), b.sin(), -b.cos() * l.sin());
        let north = DVec3::new(-b.sin() * l.cos(), b.cos(), b.sin() * l.sin());
        let east = DVec3::new(-l.sin(), 0., -l.cos());
        let forward = north * a.cos() + east * a.sin();
        DMat3::from_cols(forward.cross(up), up, -forward)
    })
}
pub fn geo_to_local(lon: f64, lat: f64, h: f64, r: f64) -> DVec3 {
    let (lon, lat) = (lon.to_radians(), lat.to_radians());
    geographic_frame().transpose()
        * DVec3::new(lat.cos() * lon.cos(), lat.sin(), -lat.cos() * lon.sin())
        * (r + h)
}
pub fn local_to_geo(p: DVec3) -> (f64, f64) {
    let n = geographic_frame() * p.normalize_or(DVec3::Y);
    (
        (-n.z).atan2(n.x).to_degrees(),
        n.y.atan2(n.x.hypot(n.z)).to_degrees(),
    )
}
pub fn hash(x: i32, z: i32, seed: u64) -> f64 {
    let mut h = (x as u64).wrapping_mul(0x9e3779b97f4a7c15)
        ^ (z as u64).wrapping_mul(0xbf58476d1ce4e5b9)
        ^ seed;
    h = (h ^ (h >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94d049bb133111eb);
    ((h ^ (h >> 31)) >> 11) as f64 / (1u64 << 53) as f64
}
pub fn noise(x: f64, z: f64, scale: f64, seed: u64) -> f64 {
    let (x, z) = (x / scale, z / scale);
    let (ix, iz) = (x.floor() as i32, z.floor() as i32);
    let ease = |v: f64| v * v * (3. - 2. * v);
    let (tx, tz) = (ease(x - x.floor()), ease(z - z.floor()));
    let a = hash(ix, iz, seed) * (1. - tx) + hash(ix + 1, iz, seed) * tx;
    let b = hash(ix, iz + 1, seed) * (1. - tx) + hash(ix + 1, iz + 1, seed) * tx;
    a * (1. - tz) + b * tz
}
pub fn forest(x: f64, z: f64) -> bool {
    if x.abs() < 220. && z.abs() < 1500. {
        return false;
    }
    let p = point(x, z, 0., radius());
    let (_, c, w, _) = active().raw(p);
    w.is_none()
        && matches!(c, LandCover::Trees | LandCover::Shrub)
        && noise(x, z, 180., active().preset.seed) > 0.20
}
pub fn tree(ix: i32, iz: i32) -> Option<(f64, f64, f64, f64)> {
    if active().preset.vegetation.density == 0. {
        return None;
    }
    let seed = active().preset.seed;
    let x = ix as f64 * 32. + 3. + hash(ix + 12, iz - 31, seed) * 26.;
    let z = iz as f64 * 32. + 3. + hash(ix - 81, iz + 17, seed) * 26.;
    let density = (noise(x, z, 110., seed + 3) * 0.55 + 0.22) * active().preset.vegetation.density;
    if hash(ix, iz, seed) > density || !forest(x, z) {
        return None;
    }
    let h = active().height_xz(x, z);
    (h < active().preset.terrain.tree_line).then_some((
        x,
        z,
        h,
        (9. + hash(ix + 5, iz + 9, seed) * 15.) * active().preset.vegetation.height_scale,
    ))
}
// Mesh vertex colors are linear; unlike Color::srgb, Mesh attributes do not
// perform a transfer-function conversion for the PBR shader.
pub(crate) fn srgb_to_linear(value: f32) -> f32 {
    if value <= 0.04045 { value / 12.92 } else { ((value + 0.055) / 1.055).powf(2.4) }
}
pub fn surface_color(x: f64, z: f64, h: f64) -> [f32; 4] {
    let p = point(x, z, h, radius());
    let sample = active().sample_surface(p);
    surface_color_sampled(x, z, h, p.normalize(), sample)
}
pub fn surface_color_sampled(x: f64, z: f64, h: f64, up: DVec3, sample: SurfaceSample) -> [f32; 4] {
    let world = active();
    let mut cover = sample.land_cover;
    if h > world.preset.terrain.snow_line {
        cover = LandCover::Snow;
    } else if h > world.preset.terrain.tree_line * 0.82 && cover == LandCover::Trees {
        cover = LandCover::Rock;
    }
    if sample.water_surface.is_some() {
        cover = LandCover::Water;
    }
    let variation = (0.82
        + noise(x, z, 330., world.preset.seed) * 0.20
        + noise(x, z, 2700., world.preset.seed + 5) * 0.12) as f32;
    let mut color = world.preset.palette[cover.palette()];
    // Granite faces remain rock colored even where coarse cover says trees.
    let slope = (1. - sample.normal.dot(up)).max(0.);
    let rock = ((slope - 0.06) / 0.16).clamp(0., 1.) as f32;
    if sample.water_surface.is_none() && cover != LandCover::Snow {
        let granite = world.preset.palette[3];
        for i in 0..3 {
            color[i] = color[i] * (1. - rock) + granite[i] * rock;
        }
    }
    [
        srgb_to_linear(color[0] * variation),
        srgb_to_linear(color[1] * variation),
        srgb_to_linear(color[2] * variation),
        1.,
    ]
}

#[derive(Clone, Copy, Debug)]
pub struct Prop {
    pub position: DVec3,
    pub half: DVec3,
    pub rock: bool,
}
/// Sparse large props use the same fixed cells for rendering and contacts.
/// Buildings are representative settlement detail, not cadastral footprints.
pub fn prop(ix: i32, iz: i32) -> Option<Prop> {
    prop_in(active(),ix,iz)
}
pub(crate) fn prop_in(world:&EnvironmentSnapshot,ix:i32,iz:i32)->Option<Prop>{
    if world.preset.legacy {
        return None;
    }
    let seed = world.preset.seed;
    let x = ix as f64 * 128. + 24. + hash(ix, iz, seed + 17) * 80.;
    let z = iz as f64 * 128. + 24. + hash(ix, iz, seed + 29) * 80.;
    if x.abs() < 250. && z.abs() < 1600. {
        return None;
    }
    let (h, cover, water, _) = world.raw(point(x, z, 0., world.preset.planet.radius));
    if water.is_some() {
        return None;
    }
    let chance = hash(ix, iz, seed + 67) / world.preset.vegetation.prop_density.max(1e-12);
    let rock = match cover {
        LandCover::Rock => chance < 0.25,
        LandCover::Trees => chance < 0.035,
        LandCover::Built => false,
        _ => return None,
    };
    if !rock && (cover != LandCover::Built || chance > 0.30) {
        return None;
    }
    let half = if rock {
        DVec3::new(1.8, 1.2, 1.5) * (0.6 + hash(ix + 7, iz, seed) * 0.8)
    } else {
        DVec3::new(5., 4., 8.)
    };
    Some(Prop {
        position: point(x, z, h + half.y, world.preset.planet.radius),
        half,
        rock,
    })
}

pub fn tree_species(ix: i32, iz: i32) -> usize {
    let weights = active().preset.vegetation.species_weights;
    let v = hash(ix, iz, active().preset.seed + 42) * weights.iter().sum::<f64>();
    if v < weights[0] {
        0
    } else if v < weights[0] + weights[1] {
        1
    } else {
        2
    }
}

/// Build the same configured medium at startup and at altitude-cache refreshes.
pub fn scattering_medium(density: f32) -> bevy::pbr::ScatteringMedium {
    let mut medium = bevy::pbr::ScatteringMedium::default();
    let tint = bevy::prelude::Vec3::from_array(active().preset.planet.sky_tint);
    for term in &mut medium.terms {
        term.scattering *= tint;
    }
    medium.with_density_multiplier(
        if matches!(active().preset.planet.atmosphere, AtmosphereProfile::Vacuum) {
            0.
        } else {
            density
        },
    )
}
