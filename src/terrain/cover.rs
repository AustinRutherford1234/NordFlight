//! Geographic color reconstruction independent of mesh LOD. Categorical detail
//! IDs are kept separate from filtered color: IDs must never be interpolated.
use crate::environment::{EnvironmentSnapshot, LandCover};
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{
        Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
    },
};
const BORDER: usize = 2;
fn material(cover: LandCover) -> u8 {
    match cover {
        LandCover::Rock => 2,
        LandCover::Snow => 5,
        LandCover::Water => 4,
        LandCover::Cropland | LandCover::Wetland => 1,
        LandCover::Built => 3,
        _ => 0,
    }
}
fn linear_to_byte(v: f32) -> u8 {
    let v = v.clamp(0., 1.);
    ((if v <= 0.0031308 {
        12.92 * v
    } else {
        1.055 * v.powf(1. / 2.4) - 0.055
    }) * 255.)
        .round() as u8
}
fn texel(
    world: &EnvironmentSnapshot,
    lon: f64,
    lat: f64,
    step: f64,
    palette: &[[f32; 3]; 8],
) -> [u8; 4] {
    // Small area filter removes isolated classification speckles without the
    // hundreds-of-metres color interpolation previously supplied by mesh triangles.
    let mut color = [0.; 3];
    for dz in [-0.25, 0.25] {
        for dx in [-0.25, 0.25] {
            let cover = LandCover::from_code(world.data.cover_at(lon + dx * step, lat + dz * step));
            for (i, c) in color.iter_mut().enumerate() {
                *c += palette[cover.palette()][i] * 0.25;
            }
        }
    }
    let cover = LandCover::from_code(world.data.cover_at(lon, lat));
    [
        linear_to_byte(color[0]),
        linear_to_byte(color[1]),
        linear_to_byte(color[2]),
        material(cover) * 51,
    ]
}
fn mip(previous: &[u8], n: usize) -> Vec<u8> {
    // Input channels are bytes, so the exact sRGB conversion has only 256
    // possible inputs. Reuse those values across every mip and atlas layer.
    static LINEAR: std::sync::OnceLock<[f32; 256]> = std::sync::OnceLock::new();
    let linear = LINEAR.get_or_init(|| {
        std::array::from_fn(|value| crate::environment::srgb_to_linear(value as f32 / 255.))
    });
    let m = (n / 2).max(1);
    let mut out = vec![0; m * m * 4];
    for y in 0..m {
        for x in 0..m {
            for c in 0..3 {
                let mut sum = 0.;
                for dy in 0..2 {
                    for dx in 0..2 {
                        let value = previous
                            [(((y * 2 + dy).min(n - 1) * n + (x * 2 + dx).min(n - 1)) * 4) + c];
                        sum += linear[value as usize];
                    }
                }
                out[(y * m + x) * 4 + c] = linear_to_byte(sum * 0.25);
            }
            // Preserve diagnostic metadata; rendering reads IDs from the R8 atlas.
            out[(y * m + x) * 4 + 3] = previous[(y * 2 * n + x * 2) * 4 + 3];
        }
    }
    out
}
fn bytes_per_layer(mut n: usize) -> usize {
    let mut bytes = 0;
    loop {
        bytes += n * n * 4;
        if n == 1 {
            return bytes;
        }
        n = (n / 2).max(1);
    }
}
fn resolution(layers: usize, budget: usize) -> usize {
    [1024, 768, 512, 256, 128, 64, 32, 16, 8]
        .into_iter()
        .find(|n| bytes_per_layer(*n) * layers <= budget)
        .unwrap_or(8)
}
fn sizes(layers: usize, budget: usize) -> (usize, usize) {
    let class_n = [2048, 1536, 1024, 768, 512, 256, 128, 64, 32, 16, 8]
        .into_iter()
        .find(|n| n * n * layers <= budget * 7 / 8)
        .unwrap_or(8);
    let color_n = resolution(layers, budget - class_n * class_n * layers).min(512);
    (class_n, color_n)
}
/// Conservative categorical majority: remove isolated one/two-sample flecks,
/// retain mixed edges and narrow features, and never reclassify water/buildings.
fn clean_class(grid: &crate::environment::pack::Grid, x: usize, y: usize) -> u8 {
    let center = LandCover::from_code(grid.cover[y * grid.cover_n + x]).palette() as u8;
    if matches!(center, 4 | 5 | 7) {
        return center;
    }
    if x > 0
        && y > 0
        && x + 1 < grid.cover_n
        && y + 1 < grid.cover_n
        && [
            y * grid.cover_n + x - 1,
            y * grid.cover_n + x + 1,
            (y - 1) * grid.cover_n + x,
            (y + 1) * grid.cover_n + x,
        ]
        .into_iter()
        .all(|i| LandCover::from_code(grid.cover[i]).palette() as u8 == center)
    {
        return center;
    }
    let mut counts = [0u8; 8];
    for yy in y.saturating_sub(1)..=(y + 1).min(grid.cover_n - 1) {
        for xx in x.saturating_sub(1)..=(x + 1).min(grid.cover_n - 1) {
            counts[LandCover::from_code(grid.cover[yy * grid.cover_n + xx]).palette()] += 1;
        }
    }
    let dominant = (0..8).max_by_key(|&i| counts[i]).unwrap();
    if counts[center as usize] <= 2 && counts[dominant] >= 5 && !matches!(dominant, 4 | 5 | 7) {
        dominant as u8
    } else {
        center
    }
}
fn class_at(world: &EnvironmentSnapshot, lon: f64, lat: f64) -> u8 {
    let lon = (lon + 180.).rem_euclid(360.) - 180.;
    let lat = lat.clamp(-89.999999, 89.999999);
    let west = lon.floor() as i32;
    let south = lat.floor() as i32;
    if let Some(grid) = world.data.grids.get(&(west, south)) {
        let x = ((lon - west as f64) * (grid.cover_n - 1) as f64).round() as usize;
        let y = ((1. - (lat - south as f64)) * (grid.cover_n - 1) as f64).round() as usize;
        clean_class(grid, x, y)
    } else {
        LandCover::from_code(world.data.cover_at(lon, lat)).palette() as u8
    }
}
pub struct CoverAtlas {
    pub colors: Handle<Image>,
    pub lookup: Handle<Image>,
    pub classes: Handle<Image>,
}
pub fn load(world: &EnvironmentSnapshot, images: &mut Assets<Image>) -> CoverAtlas {
    let mut keys = world.data.grids.keys().copied().collect::<Vec<_>>();
    keys.sort_unstable();
    // R8 lookup reserves zero for coarse fallback. Respect array-layer limits
    // and cap shared residency and the initial upload independently of mesh LOD.
    keys.truncate(255);
    let layers = keys.len().max(1);
    let budget = (world.graphics.gpu_cache_mib / 2)
        .min(world.graphics.staging_mib)
        .max(1)
        * 1024
        * 1024;
    let (class_n, n) = sizes(layers, budget);
    let inner = (n - 2 * BORDER) as f64;
    let mut classes = Vec::with_capacity(class_n * class_n * layers);
    let palette = world
        .preset
        .palette
        .map(|c| c.map(crate::environment::srgb_to_linear));
    let mut bytes = Vec::with_capacity(bytes_per_layer(n) * layers);
    let mut lookup = vec![0u8; 360 * 180];
    for (layer, &(west, south)) in keys.iter().enumerate() {
        lookup[((89 - south) * 360 + west + 180) as usize] = (layer + 1) as u8;
        let grid = &world.data.grids[&(west, south)];
        let class_inner = (class_n - 2 * BORDER) as f64;
        // The source coordinate for a column/row repeats across the layer.
        // Precompute its exact rounding once, including the geographic border.
        let coordinates: Vec<_> = (0..class_n).map(|i| {
            let u = (i as f64 - BORDER as f64 + 0.5) / class_inner;
            let index = (0. ..=1.).contains(&u)
                .then(|| (u * (grid.cover_n - 1) as f64).round() as usize);
            (u, index)
        }).collect();
        for &(v, iy) in &coordinates {
            for &(u, ix) in &coordinates {
                let id = if let (Some(ix), Some(iy)) = (ix, iy) {
                    clean_class(grid, ix, iy)
                } else {
                    class_at(world, west as f64 + u, south as f64 + 1. - v)
                };
                classes.push(id);
            }
        }
        let mut level = Vec::with_capacity(n * n * 4);
        for y in 0..n {
            for x in 0..n {
                let lon = west as f64 + (x as f64 - BORDER as f64 + 0.5) / inner;
                let lat = south as f64 + 1. - (y as f64 - BORDER as f64 + 0.5) / inner;
                level.extend_from_slice(&texel(world, lon, lat, 1. / inner, &palette));
            }
        }
        let mut width = n;
        loop {
            bytes.extend_from_slice(&level);
            if width == 1 {
                break;
            }
            level = mip(&level, width);
            width = (width / 2).max(1);
        }
    }
    if keys.is_empty() {
        bytes.resize(bytes_per_layer(n), 0);
        classes.resize(class_n * class_n, 0);
    }
    info!(
        "Geographic cover atlas: {} layers, {} px classes / {} px distant color, {:.1} MiB total",
        keys.len(),
        class_n,
        n,
        (bytes.len() + classes.len()) as f64 / 1048576.
    );
    let mut colors = Image::new_uninit(
        Extent3d {
            width: n as u32,
            height: n as u32,
            depth_or_array_layers: layers as u32,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    colors.data = Some(bytes);
    colors.texture_descriptor.mip_level_count = n.ilog2() + 1;
    colors.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::D2Array),
        ..default()
    });
    colors.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        mag_filter: bevy::image::ImageFilterMode::Linear,
        min_filter: bevy::image::ImageFilterMode::Linear,
        mipmap_filter: bevy::image::ImageFilterMode::Linear,
        ..default()
    });
    let lookup = Image::new(
        Extent3d {
            width: 360,
            height: 180,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        lookup,
        TextureFormat::R8Uint,
        RenderAssetUsages::RENDER_WORLD,
    );
    let mut classes = Image::new(
        Extent3d {
            width: class_n as u32,
            height: class_n as u32,
            depth_or_array_layers: layers as u32,
        },
        TextureDimension::D2,
        classes,
        TextureFormat::R8Uint,
        RenderAssetUsages::RENDER_WORLD,
    );
    classes.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::D2Array),
        ..default()
    });
    CoverAtlas {
        colors: images.add(colors),
        lookup: images.add(lookup),
        classes: images.add(classes),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atlas_fits_residency_and_staging_for_current_and_large_packs() {
        for layers in [1, 27, 255] {
            for budget in [8, 64, 128] {
                let budget = budget * 1024 * 1024;
                let (class_n, n) = sizes(layers, budget);
                assert!((bytes_per_layer(n) + class_n * class_n) * layers <= budget);
                assert!(n > 2 * BORDER);
            }
        }
    }
    #[test]
    fn categorical_cleanup_removes_isolated_flecks_but_keeps_edges_and_water() {
        let mut g = crate::environment::pack::Grid {
            n: 2,
            heights: vec![0; 4],
            cover: vec![30; 25],
            cover_n: 5,
        };
        g.cover[12] = 10;
        assert_eq!(clean_class(&g, 2, 2), 1);
        for y in 0..5 {
            for x in 0..2 {
                g.cover[y * 5 + x] = 10;
            }
        }
        g.cover[12] = 30;
        assert_eq!(clean_class(&g, 1, 2), 0);
        assert_eq!(clean_class(&g, 2, 2), 1);
        g.cover[12] = 80;
        assert_eq!(clean_class(&g, 2, 2), 4);
        g.cover[12] = 50;
        assert_eq!(clean_class(&g, 2, 2), 7);
    }
    #[test]
    fn mip_averages_light_not_encoded_srgb_and_keeps_constant_regions() {
        let checker = [
            0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 0,
        ];
        assert_eq!(&mip(&checker, 2)[..3], &[188, 188, 188]);
        assert_eq!(
            mip(&[37, 72, 113, 51].repeat(16), 4),
            [37, 72, 113, 51].repeat(4)
        );
    }
    #[test]
    fn area_filter_preserves_uniform_cover_and_only_mixes_actual_classes() {
        let mut world = EnvironmentSnapshot::legacy();
        world.preset.legacy = false;
        world.data.grids.insert(
            (0, 0),
            crate::environment::pack::Grid {
                n: 2,
                heights: vec![0; 4],
                cover: vec![10, 40, 10, 40],
                cover_n: 2,
            },
        );
        let palette = world
            .preset
            .palette
            .map(|c| c.map(crate::environment::srgb_to_linear));
        let left = texel(&world, 0.1, 0.5, 0.01, &palette);
        let right = texel(&world, 0.9, 0.5, 0.01, &palette);
        assert_eq!(left[3], 0);
        assert_eq!(right[3], 51);
        for i in 0..3 {
            assert!((left[i] as f32 / 255. - world.preset.palette[0][i]).abs() < 0.005);
            assert!((right[i] as f32 / 255. - world.preset.palette[2][i]).abs() < 0.005);
        }
        let edge = texel(&world, 0.5, 0.5, 0.01, &palette);
        for i in 0..3 {
            assert!(edge[i] >= left[i].min(right[i]) && edge[i] <= left[i].max(right[i]));
        }
    }
}
