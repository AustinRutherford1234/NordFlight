//! Sunward heightfield occlusion on a planet-fixed grid. Dynamic shadow maps
//! still handle the smaller silhouettes of objects and vegetation.
use bevy::{
    asset::{AssetId, RenderAssetUsages},
    math::{DVec3, IVec2},
    pbr::TerrainSunShadow,
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        render_asset::{RenderAssets, prepare_assets},
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        texture::GpuImage,
    },
    tasks::{AsyncComputeTaskPool, Task, block_on, poll_once},
};
use std::{collections::VecDeque, sync::Arc};

use crate::{
    FlightCamera, FlightStage, ViewState,
    physics::{Environment, Flight, Vehicle},
};

const STEP: f64 = 128.;
const SIDE: i32 = 512;
const REGION_STRIDE: i32 = 128;
const RAY_STEPS: i32 = 256;
const INPUT_SIDE: i32 = SIDE + 2 * RAY_STEPS;
const MIN_SUN_DOT: f64 = 0.999_990_480_720_734_5; // cos(0.25 degrees)

#[derive(Resource, Default)]
pub(crate) struct ReliefShadow {
    wanted: Option<IVec2>,
    task: Option<Task<Built>>,
    uploading: Option<Atlas>,
    current: Option<Atlas>,
}

#[derive(Resource, Clone, Default)]
struct Ready(crate::terrain_ready::TerrainReady);

struct Heights {
    lower: IVec2,
    values: Vec<f32>,
}

impl Heights {
    fn get(&self, point: IVec2) -> Option<f64> {
        let cell = point - self.lower;
        (cell.cmpge(IVec2::ZERO).all() && cell.cmplt(IVec2::splat(INPUT_SIDE)).all())
            .then(|| f64::from(self.values[(cell.y * INPUT_SIDE + cell.x) as usize]))
    }
}

struct Built {
    region: IVec2,
    sun: DVec3,
    heights: Arc<Heights>,
    zero: f64,
    pixels: Vec<u8>,
}

struct Atlas {
    region: IVec2,
    sun: DVec3,
    heights: Arc<Heights>,
    zero: f64,
    image: Handle<Image>,
}

fn lower(region: IVec2) -> IVec2 {
    region * REGION_STRIDE - IVec2::splat(SIDE / 2)
}

pub(super) fn install(app: &mut App) {
    let ready = Ready::default();
    app.init_resource::<ReliefShadow>()
        .insert_resource(ready.clone())
        .add_systems(
            Update,
            update
                .after(FlightStage::Camera)
                .before(FlightStage::Presentation),
        );
    if let Some(render) = app.get_sub_app_mut(RenderApp) {
        render.insert_resource(ready).add_systems(
            Render,
            check
                .in_set(RenderSystems::PrepareAssets)
                .after(prepare_assets::<GpuImage>),
        );
    }
}

fn check(ready: Res<Ready>, images: Res<RenderAssets<GpuImage>>) {
    ready
        .0
        .poll_all(|_| true, |id| images.get(id).is_some(), |_| true);
}

pub(crate) fn loading_image(state: Option<&ReliefShadow>) -> Option<Option<AssetId<Image>>> {
    let Some(state) = state else {
        return Some(None);
    };
    let Some(wanted) = state.wanted else {
        return Some(None);
    };
    state
        .current
        .as_ref()
        .filter(|map| map.region == wanted)
        .map(|map| Some(map.image.id()))
}

fn update(
    mut commands: Commands,
    flight: Res<Flight>,
    environment: Res<Environment>,
    view: Res<ViewState>,
    mut state: ResMut<ReliefShadow>,
    ready: Res<Ready>,
    mut images: ResMut<Assets<Image>>,
    cameras: Query<(Entity, Option<&TerrainSunShadow>), With<FlightCamera>>,
) {
    let off_surface = flight.vehicle == Vehicle::Spacecraft
        && (flight.space.body == crate::space::Body::Moon || flight.altitude() > 30_000.);
    if view.map || off_surface {
        state.wanted = None;
        for (entity, active) in &cameras {
            if active.is_some() {
                commands.entity(entity).remove::<TerrainSunShadow>();
            }
        }
        return;
    }

    let region = (flight.position.xz() / (STEP * f64::from(REGION_STRIDE)))
        .round()
        .as_ivec2();
    let sun = environment.sun_direction();
    state.wanted = Some(region);
    if let Some(task) = state.task.as_mut()
        && let Some(built) = block_on(poll_once(task))
    {
        state.task = None;
        if built.region == region {
            let image = Image::new(
                Extent3d {
                    width: SIDE as u32,
                    height: SIDE as u32,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                built.pixels,
                TextureFormat::Rgba8Unorm,
                RenderAssetUsages::RENDER_WORLD,
            );
            let image = images.add(image);
            ready.0.request_if_changed(vec![], vec![image.id()], vec![]);
            state.uploading = Some(Atlas {
                region,
                sun: built.sun,
                heights: built.heights,
                zero: built.zero,
                image,
            });
        }
    }
    if state.uploading.is_some() && ready.0.ready() {
        let map = state.uploading.take().unwrap();
        if map.region == region {
            state.current = Some(map);
        }
        ready.0.clear();
    }
    if state.task.is_none()
        && state.uploading.is_none()
        && state
            .current
            .as_ref()
            .is_none_or(|map| map.region != region || map.sun.dot(sun) < MIN_SUN_DOT)
    {
        let previous = state.current.as_ref().map(|map| map.heights.clone());
        let world = Arc::clone(super::active());
        state.task = Some(AsyncComputeTaskPool::get().spawn(build(world, region, sun, previous)));
    }

    if let Some(map) = &state.current {
        let frame = crate::world::frame(&flight);
        let origin = crate::world::render_origin(&flight);
        let corner = lower(map.region).as_dvec2() * STEP - STEP / 2.;
        let relative = DVec3::new(
            origin.x - corner.x,
            origin.y - super::radius() - map.zero,
            origin.z - corner.y,
        );
        let scale = (STEP * f64::from(SIDE)).recip() as f32;
        let transform = Mat4::from_scale(Vec3::new(scale, 1., scale))
            * Mat4::from_rotation_translation(frame.as_quat(), relative.as_vec3());
        for (entity, active) in &cameras {
            if active.is_none_or(|old| old.image != map.image || old.field_from_world != transform)
            {
                commands.entity(entity).insert(TerrainSunShadow {
                    image: map.image.clone(),
                    field_from_world: transform,
                });
            }
        }
    }
}

async fn build(
    world: Arc<super::EnvironmentSnapshot>,
    region: IVec2,
    sun: DVec3,
    previous: Option<Arc<Heights>>,
) -> Built {
    let first = lower(region) - IVec2::splat(RAY_STEPS);
    let radius = world.preset.planet.radius;
    let heights = if let Some(previous) = previous.as_ref().filter(|grid| grid.lower == first) {
        previous.clone()
    } else {
        let mut values = Vec::with_capacity((INPUT_SIDE * INPUT_SIDE) as usize);
        for z in 0..INPUT_SIDE {
            for x in 0..INPUT_SIDE {
                let cell = first + IVec2::new(x, z);
                let value = previous
                    .as_ref()
                    .and_then(|grid| grid.get(cell))
                    .unwrap_or_else(|| {
                        let metres = cell.as_dvec2() * STEP;
                        let h = world.height_xz(metres.x, metres.y);
                        super::point(metres.x, metres.y, h, radius).y - radius
                    });
                values.push(value as f32);
            }
            // Share the terrain worker with ordinary streaming during travel.
            if z % 16 == 15 {
                bevy::tasks::futures_lite::future::yield_now().await;
            }
        }
        Arc::new(Heights {
            lower: first,
            values,
        })
    };
    let center = region.as_dvec2() * (STEP * f64::from(REGION_STRIDE));
    let base = super::point(center.x, center.y, 0., radius).y - radius - 16384.;
    // Use the same half-metre lattice in overlapping atlases.
    let zero = (base * 2.).floor() * 0.5;
    let mut pixels = vec![0; (SIDE * SIDE * 4) as usize];
    scan(
        first,
        INPUT_SIDE,
        RAY_STEPS,
        sun,
        |cell| heights.get(cell).unwrap(),
        |cell, ground, blocker| {
            let coord = cell - lower(region);
            if coord.cmpge(IVec2::ZERO).all() && coord.cmplt(IVec2::splat(SIDE)).all() {
                let offset = ((coord.y * SIDE + coord.x) * 4) as usize;
                let encode = |h: f64| ((h - zero) * 2.).round().clamp(0., 65535.) as u16;
                pixels[offset..offset + 2]
                    .copy_from_slice(&encode(blocker.max(ground - 32.)).to_le_bytes());
                pixels[offset + 2..offset + 4].copy_from_slice(&encode(ground).to_le_bytes());
            }
        },
    );
    Built {
        region,
        sun,
        heights,
        zero,
        pixels,
    }
}

/// Sweep along sheared integer scan lines. Their phase is determined by the
/// absolute grid coordinate, so overlapping atlases use the same sun rays.
/// The running maximum is restricted to a fixed upstream distance, including
/// at the edges of an atlas's visible area.
fn scan(
    lower: IVec2,
    side: i32,
    range: i32,
    sun: DVec3,
    mut height: impl FnMut(IVec2) -> f64,
    mut store: impl FnMut(IVec2, f64, f64),
) {
    let swap = sun.z.abs() > sun.x.abs();
    let axes = |p: IVec2| if swap { IVec2::new(p.y, p.x) } else { p };
    let low = axes(lower);
    let high = low + IVec2::splat(side - 1);
    let (major, minor) = if swap { (sun.z, sun.x) } else { (sun.x, sun.z) };
    let ratio = if major.abs() > 1e-6 {
        minor / major
    } else {
        0.
    };
    let rise = STEP * sun.y / major.abs().max(1e-6);
    let shear = |i: i32| (f64::from(i) * ratio).round() as i32;
    let a = shear(low.x);
    let b = shear(high.x);
    let mut peaks = VecDeque::<(i32, f64)>::new();
    for ray in low.y - a.max(b)..=high.y - a.min(b) {
        peaks.clear();
        for step in 0..side {
            let x = if major >= 0. {
                high.x - step
            } else {
                low.x + step
            };
            let y = ray + shear(x);
            if !(low.y..=high.y).contains(&y) {
                continue;
            }
            let point = axes(IVec2::new(x, y));
            let ground = height(point);
            let offset = f64::from(step) * rise;
            while peaks
                .front()
                .is_some_and(|&(index, _)| step - index > range)
            {
                peaks.pop_front();
            }
            let blocker = peaks
                .front()
                .map_or(f64::NEG_INFINITY, |&(_, h)| h - offset);
            store(point, ground, blocker);
            let projected = ground + offset;
            while peaks.back().is_some_and(|&(_, h)| h <= projected) {
                peaks.pop_back();
            }
            peaks.push_back((step, projected));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ridge_casts_down_sun_at_the_expected_height_and_range() {
        let mut shadow = [0.; 7];
        scan(
            IVec2::ZERO,
            7,
            3,
            DVec3::new(1., 0.125, 0.),
            |p| if p.x == 5 { 100. } else { 0. },
            |p, _, h| {
                if p.y == 3 {
                    shadow[p.x as usize] = h;
                }
            },
        );
        assert_eq!(&shadow[2..5], &[52., 68., 84.]);
        assert!(shadow[1] < 0. && shadow[6].is_infinite());
    }

    #[test]
    fn overlapping_atlases_keep_the_same_world_space_shadow_rays() {
        use std::collections::HashMap;
        let height = |p: IVec2| f64::from((p.x * 171 + p.y * 71).rem_euclid(2048));
        for sun in [
            DVec3::new(1., 0.05, 0.31),
            DVec3::new(-1., 0.05, 0.61),
            DVec3::new(-0.48, 0.05, 1.),
            DVec3::new(0.38, 0.05, -1.),
        ] {
            let gather = |start| {
                let mut map = HashMap::new();
                scan(start, 40, 5, sun, height, |cell, _, blocker| {
                    assert!(map.insert(cell, blocker).is_none());
                });
                assert_eq!(map.len(), 40 * 40);
                map
            };
            let a = gather(IVec2::new(-20, -21));
            let b = gather(IVec2::new(-16, -18));
            for z in -12..12 {
                for x in -10..12 {
                    let cell = IVec2::new(x, z);
                    assert!((a[&cell] - b[&cell]).abs() < 1e-9);
                }
            }
        }
    }
}
