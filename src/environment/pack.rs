//! CPU-resident read-only samples, loaded before flight. No disk I/O in queries.
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Component, Path},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TileManifest {
    pub file: String,
    pub west: i32,
    pub south: i32,
    pub east: i32,
    pub north: i32,
    pub samples: usize,
    #[serde(default)]
    pub cover_samples: Option<usize>,
    pub sha256: String,
    pub bytes: usize,
    pub elevation_source: String,
    pub landcover_source: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainPackManifest {
    pub version: u32,
    pub id: String,
    pub revision: u64,
    pub bounds: [i32; 4],
    pub vertical_reference: String,
    pub ground_model: String,
    pub tiles: Vec<TileManifest>,
}
#[derive(Debug)]
pub struct Grid {
    pub n: usize,
    pub heights: Vec<i16>,
    pub cover: Vec<u8>,
    pub cover_n: usize,
}
impl Grid {
    pub fn sample(&self, u: f64, v: f64) -> (f64, u8) {
        let x = u.clamp(0., 1.) * (self.n - 1) as f64;
        let y = v.clamp(0., 1.) * (self.n - 1) as f64;
        let ix = (x.floor() as usize).min(self.n - 2);
        let iy = (y.floor() as usize).min(self.n - 2);
        let tx = x - ix as f64;
        let ty = y - iy as f64;
        let at = |dx, dy| self.heights[(iy + dy) * self.n + ix + dx] as f64;
        let h = (at(0, 0) * (1. - tx) + at(1, 0) * tx) * (1. - ty)
            + (at(0, 1) * (1. - tx) + at(1, 1) * tx) * ty;
        let cx = (u.clamp(0., 1.) * (self.cover_n - 1) as f64).round() as usize;
        let cy = (v.clamp(0., 1.) * (self.cover_n - 1) as f64).round() as usize;
        let c = self.cover[cy * self.cover_n + cx];
        (h, c)
    }
}
#[derive(Debug, Default)]
pub struct TerrainData {
    pub grids: HashMap<(i32, i32), Grid>,
    pub manifests: Vec<TerrainPackManifest>,
    pub global: Vec<i16>,
    pub bytes: usize,
}
impl TerrainData {
    pub fn load(root: &Path, packs: &[String], budget: usize) -> Result<Self, String> {
        let mut data = Self::default();
        let global = root.join("global/global.bin");
        if global.exists() {
            let bytes = std::fs::read(&global).map_err(|e| e.to_string())?;
            if bytes.len() != 3600 * 1800 * 2 {
                return Err("invalid global terrain size".into());
            }
            data.bytes += bytes.len();
            data.global = decode(&bytes);
        }
        for id in packs {
            let dir = root.join(id);
            let path = dir.join("manifest.json");
            if !path.exists() {
                eprintln!("Terrain pack {id} unavailable; using coarse global coverage");
                continue;
            }
            let manifest: TerrainPackManifest =
                serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            if manifest.version != 1 || manifest.revision == 0 || manifest.id != *id {
                return Err(format!("invalid pack manifest {id}"));
            }
            for t in &manifest.tiles {
                if t.samples < 33
                    || t.samples > 3601
                    || t.east != t.west + 1
                    || t.north != t.south + 1
                    || !(-180..180).contains(&t.west)
                    || !(-90..90).contains(&t.south)
                    || Path::new(&t.file)
                        .components()
                        .any(|c| !matches!(c, Component::Normal(_)))
                {
                    return Err(format!("invalid tile in {id}"));
                }
                let cover_n = t.cover_samples.unwrap_or(t.samples);
                if !(33..=7201).contains(&cover_n) {
                    return Err("invalid cover grid".into());
                }
                let length = t.samples * t.samples * 2 + cover_n * cover_n;
                if length != t.bytes || data.bytes + length > budget {
                    return Err(format!("terrain cache budget exceeded by {id}"));
                }
                let bytes = std::fs::read(dir.join(&t.file)).map_err(|e| e.to_string())?;
                if bytes.len() != length {
                    return Err(format!("truncated terrain tile {}", t.file));
                }
                if data.grids.contains_key(&(t.west, t.south)) {
                    return Err("overlapping terrain packs".into());
                }
                let cells = t.samples * t.samples;
                data.grids.insert(
                    (t.west, t.south),
                    Grid {
                        n: t.samples,
                        heights: decode(&bytes[..cells * 2]),
                        cover: bytes[cells * 2..].to_vec(),
                        cover_n,
                    },
                );
                data.bytes += length;
            }
            data.manifests.push(manifest);
        }
        if data.bytes > budget {
            return Err("global terrain exceeds CPU budget".into());
        }
        Ok(data)
    }
    pub fn sample(&self, lon: f64, lat: f64) -> (f64, u8, bool) {
        let lon = (lon + 180.).rem_euclid(360.) - 180.;
        let lat = lat.clamp(-89.999999, 89.999999);
        let key = (lon.floor() as i32, lat.floor() as i32);
        if let Some(g) = self.grids.get(&key) {
            let (h, c) = g.sample(lon - key.0 as f64, 1. - (lat - key.1 as f64));
            return (h, c, true);
        }
        if self.global.is_empty() {
            return (0., 80, false);
        }
        let x = (lon + 180.) * 10. - 0.5;
        let y = ((90. - lat) * 10. - 0.5).clamp(0., 1799.);
        let at = |x: i32, y: i32| {
            self.global[y.clamp(0, 1799) as usize * 3600 + x.rem_euclid(3600) as usize] as f64
        };
        let ix = x.floor() as i32;
        let iy = y.floor() as i32;
        let tx = x - ix as f64;
        let ty = y - iy as f64;
        let h = (at(ix, iy) * (1. - tx) + at(ix + 1, iy) * tx) * (1. - ty)
            + (at(ix, iy + 1) * (1. - tx) + at(ix + 1, iy + 1) * tx) * ty;
        (
            h,
            if h <= 0. {
                80
            } else if h > 2500. {
                60
            } else {
                30
            },
            false,
        )
    }
}
fn decode(bytes: &[u8]) -> Vec<i16> {
    bytes
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]))
        .collect()
}
