//! Quadtree layout, hysteresis and projected geometric-error selection.
use super::*;
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) struct Tile {
    pub(super) x: i32,
    pub(super) z: i32,
    pub(super) size: i32,
}
/// `active` is the published layout, used for split/merge hysteresis: a tile
/// that is currently split stays split until 0.95 of its size, so circling
/// near a boundary does not rebuild the same tiles repeatedly. `coarser` is set
/// once an ancestor is an active leaf. Decisions remain per quadtree node, so
/// the result is always a complete, non-overlapping partition.
pub(super) type Active<'a> = Option<&'a dyn Fn(&Tile) -> bool>;

struct Detail {
    agl: f64,
    real: bool,
    shadow_radius: f64,
    pixels_per_radian: f64,
    error_pixels: f64,
}

pub(super) fn shadow_radius(real: bool) -> f64 {
    // Off-axis rays and streaming latency need coverage outside the cascade.
    if real {
        f64::from(crate::environment::graphics::shadow_distance()) * 2. + 1024.
    } else {
        // Legacy terrain protects its near cascade with 512 m forest tiles.
        768.
    }
}

pub(super) fn shadow_detail(agl: f64, radius: f64) -> bool {
    agl <= radius + 2000.
}

fn leaves(
    tile: Tile,
    p: DVec3,
    detail: &Detail,
    active: Active,
    coarser: bool,
    out: &mut HashSet<Tile>,
) {
    let min_size = if detail.real { 2048 } else { 512 };
    if tile.size <= min_size {
        out.insert(tile);
        return;
    }
    let dx = (tile.x as f64 - p.x)
        .max(0.)
        .max(p.x - (tile.x + tile.size) as f64);
    let dz = (tile.z as f64 - p.z)
        .max(0.)
        .max(p.z - (tile.z + tile.size) as f64);
    let leaf = active.is_some_and(|has| has(&tile));
    let split = active.is_some_and(|_| !coarser && !leaf);
    let reach = if split { 0.95 } else { 0.85 };
    let horizontal = dx.hypot(dz);
    let keep_shadow_detail =
        horizontal <= detail.shadow_radius && shadow_detail(detail.agl, detail.shadow_radius);
    let split_needed = if keep_shadow_detail {
        true
    } else if detail.real {
        let distance = horizontal.hypot(detail.agl.max(10.));
        let pixels = estimated_error(tile) * detail.pixels_per_radian / distance;
        pixels > detail.error_pixels * if split { 0.75 } else { 1. }
    } else {
        horizontal < tile.size as f64 * reach && detail.agl < tile.size as f64 * 2.
    };
    if split_needed {
        let half = tile.size / 2;
        for (x, z) in [
            (tile.x, tile.z),
            (tile.x + half, tile.z),
            (tile.x, tile.z + half),
            (tile.x + half, tile.z + half),
        ] {
            leaves(
                Tile { x, z, size: half },
                p,
                detail,
                active,
                coarser || leaf,
                out,
            );
        }
    } else {
        out.insert(tile);
    }
}
pub(super) fn layout(p: DVec3, agl: f64) -> HashSet<Tile> {
    layout_from(p, agl, None)
}
pub(super) fn layout_from(p: DVec3, agl: f64, active: Active) -> HashSet<Tile> {
    let real = crate::environment::real();
    let detail = Detail {
        agl,
        real,
        shadow_radius: shadow_radius(real),
        pixels_per_radian: crate::environment::graphics::render_height() / (0.577350269 * 2.),
        error_pixels: crate::environment::graphics::lod_error(),
    };
    let mut out = HashSet::new();
    let (cx, cz) = if real {
        ((p.x / 32768.).floor() as i32, (p.z / 32768.).floor() as i32)
    } else {
        (3, -2)
    };
    for z in cz - 6..cz + 6 {
        for x in cx - 5..cx + 5 {
            leaves(
                Tile {
                    x: x * 32768,
                    z: z * 32768,
                    size: 32768,
                },
                p,
                &detail,
                active,
                false,
                &mut out,
            );
        }
    }
    out
}
// Cache geometric interpolation error at the actual 64-segment grid spacing.
// A finite sample estimate guides LOD; skirts remain the conservative crack guard.
fn estimated_error(tile: Tile) -> f64 {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<HashMap<Tile, f64>>> =
        std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(error) = cache.lock().unwrap().get(&tile) {
        return *error;
    }
    let step = tile.size as f64 / 64.;
    let mut error = 0f64;
    let(mut wet,mut dry)=(false,false);
    for j in 0..=4 {
        for i in 0..=4 {
            let x = tile.x as f64 + tile.size as f64 * i as f64 / 4.;
            let z = tile.z as f64 + tile.size as f64 * j as f64 / 4.;
            let h = terrain_height(x, z);
            let water=crate::environment::active().raw(crate::environment::point(x,z,h,crate::environment::radius())).2.is_some();
            wet|=water;dry|=!water;
            let hx = (terrain_height(x - step * 0.5, z) + terrain_height(x + step * 0.5, z)) * 0.5;
            let hz = (terrain_height(x, z - step * 0.5) + terrain_height(x, z + step * 0.5)) * 0.5;
            error = error.max((hx - h).abs()).max((hz - h).abs());
        }
    }
    // A small spacing-dependent floor avoids undersampling unsurveyed features.
    error = error.max(step * 0.035);
    // A flat DEM on both sides can hide an important shoreline from geometric
    // error samples. Retain finer geometry there so water/land intersections
    // do not move hundreds of metres on a quadtree split or merge.
    if wet&&dry{error=error.max(tile.size as f64*0.01);}
    let mut cache = cache.lock().unwrap();
    if cache.len() > 8192 {
        cache.clear();
    }
    cache.insert(tile, error);
    error
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadow_coverage_keeps_the_same_fine_tiles_through_an_approach() {
        let tile = Tile {
            x: 0,
            z: 0,
            size: 32768,
        };
        let detail = Detail {
            agl: 800.,
            real: true,
            shadow_radius: 4624.,
            pixels_per_radian: 0., // Isolate the shadow rule from screen error.
            error_pixels: 4.,
        };
        let protected = Tile {
            x: 4096,
            z: 4096,
            size: 2048,
        };
        let mut active = HashSet::new();
        for x in [0., 500., 2100., 4096.] {
            let mut next = HashSet::new();
            leaves(
                tile,
                DVec3::new(x, 0., 4096.),
                &detail,
                Some(&|t| active.contains(t)),
                false,
                &mut next,
            );
            assert!(next.contains(&protected));
            assert!(next.iter().any(|t| t.size >= 8192));
            active = next;
        }
    }
}
