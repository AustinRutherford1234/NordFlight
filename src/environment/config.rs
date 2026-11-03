//! Versioned world data. Physical parameters never live in graphics settings.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "model", rename_all = "snake_case", deny_unknown_fields)]
pub enum AtmosphereProfile {
    Earth,
    Dry {
        temperature: f64,
        pressure: f64,
        gas_constant: f64,
        gamma: f64,
        top: f64,
    },
    Vacuum,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanetDefinition {
    pub version: u32,
    pub id: String,
    pub radius: f64,
    pub mu: f64,
    pub rotation_rate: f64,
    pub sea_level: f64,
    pub atmosphere: AtmosphereProfile,
    pub sky_tint: [f32; 3],
}
impl PlanetDefinition {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || self.id.is_empty()
            || !self.radius.is_finite()
            || self.radius < 1_000_000.
            || !self.mu.is_finite()
            || self.mu <= 0.
            || !self.rotation_rate.is_finite()
            || !self.sea_level.is_finite()
            || self.sky_tint.iter().any(|x| !x.is_finite() || *x < 0.)
        {
            return Err(
                "invalid planet definition (version, radius, gravity, rotation, water or sky)"
                    .into(),
            );
        }
        if let AtmosphereProfile::Dry {
            temperature,
            pressure,
            gas_constant,
            gamma,
            top,
        } = self.atmosphere
        {
            if [temperature, pressure, gas_constant, gamma, top]
                .iter()
                .any(|x| !x.is_finite())
                || temperature < 30.
                || pressure < 0.
                || gas_constant <= 0.
                || gamma <= 1.
                || top <= 0.
            {
                return Err("invalid dry atmosphere".into());
            }
        }
        Ok(())
    }
    pub fn surface_gravity(&self) -> f64 {
        self.mu / self.radius.powi(2)
    }
    pub fn atmosphere(&self, height: f64) -> (f64, f64, f64, f64) {
        match self.atmosphere {
            AtmosphereProfile::Earth => crate::physics::earth_atmosphere(height),
            AtmosphereProfile::Vacuum => (3., 0., 0., 1.),
            AtmosphereProfile::Dry {
                temperature,
                pressure,
                gas_constant,
                gamma,
                top,
            } => {
                let h = height.max(0.);
                let geopotential = self.radius * h / (self.radius + h);
                let t = ((top - h) / (top * 0.15)).clamp(0., 1.);
                let fade = t * t * (3. - 2. * t);
                let p = pressure
                    * (-self.surface_gravity() * geopotential / (gas_constant * temperature)).exp()
                    * fade;
                (
                    temperature,
                    p,
                    p / (gas_constant * temperature),
                    (gamma * gas_constant * temperature).sqrt(),
                )
            }
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TerrainDefinition {
    pub relief_scale: f64,
    pub height_offset: f64,
    pub snow_line: f64,
    pub tree_line: f64,
}
impl Default for TerrainDefinition {
    fn default() -> Self {
        Self {
            relief_scale: 1.,
            height_offset: 0.,
            snow_line: 1900.,
            tree_line: 1600.,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VegetationDefinition {
    pub density: f64,
    pub height_scale: f64,
    pub species_weights: [f64; 3],
    pub prop_density: f64,
}
impl Default for VegetationDefinition {
    fn default() -> Self {
        Self {
            density: 1.,
            height_scale: 1.,
            species_weights: [0.45, 0.35, 0.2],
            prop_density: 1.,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentPreset {
    pub version: u32,
    pub id: String,
    pub label: String,
    pub revision: u64,
    pub planet: PlanetDefinition,
    pub packs: Vec<String>,
    pub seed: u64,
    pub legacy: bool,
    pub palette: [[f32; 3]; 8],
    #[serde(default)]
    pub terrain: TerrainDefinition,
    #[serde(default)]
    pub vegetation: VegetationDefinition,
}
impl EnvironmentPreset {
    pub fn validate(&self) -> Result<(), String> {
        self.planet.validate()?;
        let t = &self.terrain;
        let v = &self.vegetation;
        if !t.relief_scale.is_finite()
            || !(0.1..=3.).contains(&t.relief_scale)
            || !t.height_offset.is_finite()
            || t.height_offset.abs() > 5000.
            || !t.snow_line.is_finite()
            || t.snow_line < 0.
            || !t.tree_line.is_finite()
            || t.tree_line < 0.
            || !v.density.is_finite()
            || !(0. ..=2.).contains(&v.density)
            || !v.height_scale.is_finite()
            || !(0.2..=2.).contains(&v.height_scale)
            || !v.prop_density.is_finite()
            || !(0. ..=2.).contains(&v.prop_density)
            || v.species_weights.iter().any(|x| !x.is_finite() || *x < 0.)
            || v.species_weights.iter().sum::<f64>() <= 0.
        {
            return Err("invalid terrain or vegetation rules".into());
        }
        if self.version != 1
            || self.revision == 0
            || self.id.is_empty()
            || self.packs.len() > 64
            || self
                .packs
                .iter()
                .any(|s| s.is_empty() || s.contains("..") || s.contains('/') || s.contains('\\'))
            || self
                .palette
                .iter()
                .flatten()
                .any(|x| !x.is_finite() || !(0. ..=1.).contains(x))
        {
            return Err("invalid environment preset".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GraphicsSettings {
    pub version: u32,
    pub render_scale: f32,
    pub terrain_error_pixels: f64,
    pub vegetation_distance: f64,
    pub shadow_distance: f32,
    pub texture_detail: u32,
    pub cpu_cache_mib: usize,
    pub gpu_cache_mib: usize,
    pub staging_mib: usize,
}
impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            version: 1,
            render_scale: 0.5,
            terrain_error_pixels: 4.,
            vegetation_distance: 4500.,
            shadow_distance: 1800.,
            texture_detail: 512,
            cpu_cache_mib: 384,
            gpu_cache_mib: 384,
            staging_mib: 128,
        }
    }
}
impl GraphicsSettings {
    pub fn high() -> Self {
        Self {
            render_scale: 1.,
            terrain_error_pixels: 2.,
            vegetation_distance: 8000.,
            shadow_distance: 3500.,
            texture_detail: 1024,
            gpu_cache_mib: 768,
            ..Self::default()
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || !self.render_scale.is_finite()
            || !(0.35..=1.).contains(&self.render_scale)
            || !self.terrain_error_pixels.is_finite()
            || !(1. ..=16.).contains(&self.terrain_error_pixels)
            || !self.vegetation_distance.is_finite()
            || !(500. ..=12000.).contains(&self.vegetation_distance)
            || !self.shadow_distance.is_finite()
            || !(100. ..=5000.).contains(&self.shadow_distance)
            || ![256, 512, 1024, 2048].contains(&self.texture_detail)
            || !(32..=2048).contains(&self.cpu_cache_mib)
            || !(64..=4096).contains(&self.gpu_cache_mib)
            || !(8..=512).contains(&self.staging_mib)
        {
            return Err("invalid graphics settings".into());
        }
        Ok(())
    }
}
