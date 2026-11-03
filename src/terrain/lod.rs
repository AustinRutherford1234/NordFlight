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
fn leaves(tile: Tile, p: DVec3, agl: f64, active: Active, coarser: bool, out: &mut HashSet<Tile>) {
    let dx = (tile.x as f64 - p.x)
        .max(0.)
        .max(p.x - (tile.x + tile.size) as f64);
    let dz = (tile.z as f64 - p.z)
        .max(0.)
        .max(p.z - (tile.z + tile.size) as f64);
    let leaf = active.is_some_and(|has| has(&tile));
    let split = active.is_some_and(|_| !coarser && !leaf);
    let reach = if split { 0.95 } else { 0.85 };
    let split_needed = if crate::environment::real() {
        let distance = dx.hypot(dz).hypot(agl.max(10.));
        let pixels = estimated_error(tile) * crate::environment::graphics::render_height()
            / (distance * 0.577350269 * 2.);
        tile.size > 2048
            && pixels > crate::environment::graphics::lod_error() * if split { 0.75 } else { 1. }
    } else {
        tile.size > 512 && dx.hypot(dz) < tile.size as f64 * reach && agl < tile.size as f64 * 2.
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
                agl,
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
    let mut out = HashSet::new();
    let (cx, cz) = if crate::environment::real() {
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
                agl,
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
    for j in 0..=4 {
        for i in 0..=4 {
            let x = tile.x as f64 + tile.size as f64 * i as f64 / 4.;
            let z = tile.z as f64 + tile.size as f64 * j as f64 / 4.;
            let h = terrain_height(x, z);
            let hx = (terrain_height(x - step * 0.5, z) + terrain_height(x + step * 0.5, z)) * 0.5;
            let hz = (terrain_height(x, z - step * 0.5) + terrain_height(x, z + step * 0.5)) * 0.5;
            error = error.max((hx - h).abs()).max((hz - h).abs());
        }
    }
    // A small spacing-dependent floor avoids undersampling unsurveyed features.
    error = error.max(step * 0.035);
    let mut cache = cache.lock().unwrap();
    if cache.len() > 8192 {
        cache.clear();
    }
    cache.insert(tile, error);
    error
}
