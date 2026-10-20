//! Bounded terrain LOD, procedural surface detail, and shared off-field contacts.
use crate::{
    physics::*,
    world::{self, WorldObject},
};
use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor},
    math::{DQuat, DVec3},
    mesh::Indices,
    prelude::*,
    render::render_resource::{Extent3d, PrimitiveTopology, TextureDimension, TextureFormat},
    tasks::{AsyncComputeTaskPool, Task, block_on, poll_once},
};
use std::collections::{HashMap, HashSet};

#[derive(Component)]
pub(crate) struct ForestBatch;

pub fn paved(x: f64, z: f64) -> bool {
    x.abs() < 22.5 && z.abs() < RUNWAY_HALF
}
/// Terrain queries use world x/z: retain those coordinates when lifting a vertex.
pub fn point(x: f64, z: f64, h: f64) -> DVec3 {
    DVec3::new(x, ((RADIUS + h).powi(2) - x * x - z * z).max(0.).sqrt(), z)
}
pub fn meadow(x: f64, z: f64) -> bool {
    (x - 6500.).abs() < 420. && (z - 3200.).abs() < 1250.
}
pub fn normal(p: DVec3) -> DVec3 {
    let x = p.x;
    let z = p.z;
    let d = 2.;
    let dx = point(x + d, z, terrain_height(x + d, z).max(0.))
        - point(x - d, z, terrain_height(x - d, z).max(0.));
    let dz = point(x, z + d, terrain_height(x, z + d).max(0.))
        - point(x, z - d, terrain_height(x, z - d).max(0.));
    dz.cross(dx).normalize_or(p.normalize())
}
fn noise(x: i32, z: i32) -> f64 {
    let mut h =
        (x as u64).wrapping_mul(0x9e3779b97f4a7c15) ^ (z as u64).wrapping_mul(0xbf58476d1ce4e5b9);
    h = (h ^ (h >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94d049bb133111eb);
    ((h ^ (h >> 31)) >> 11) as f64 / (1_u64 << 53) as f64
}
pub fn forest(x: f64, z: f64) -> bool {
    if (x.abs() < 650. && z.abs() < 2600.) || meadow(x, z) || in_lake(x, z) {
        return false;
    }
    (x * 0.00071 + 0.7).sin() * (z * 0.00053 - 0.4).cos() > 0.18
}
pub fn tree(ix: i32, iz: i32) -> Option<(f64, f64, f64, f64)> {
    if noise(ix, iz) > 0.62 {
        return None;
    }
    let x = ix as f64 * 32. + 8. + noise(ix + 12, iz - 31) * 16.;
    let z = iz as f64 * 32. + 8. + noise(ix - 81, iz + 17) * 16.;
    if !forest(x, z) {
        return None;
    }
    let h = terrain_height(x, z);
    (h > 2. && h < 1600.).then_some((x, z, h, 10. + noise(ix + 5, iz + 9) * 12.))
}
pub fn obstacles(a: DVec3, b: DVec3) -> Vec<(DVec3, DVec3)> {
    let mut boxes = Vec::new();
    if a.length().min(b.length()) - RADIUS - terrain_height(b.x, b.z) > 30. {
        return boxes;
    }
    for iz in
        ((a.z.min(b.z) - 16.) / 32.).floor() as i32..=((a.z.max(b.z) + 16.) / 32.).floor() as i32
    {
        for ix in ((a.x.min(b.x) - 16.) / 32.).floor() as i32
            ..=((a.x.max(b.x) + 16.) / 32.).floor() as i32
        {
            if let Some((x, z, h, size)) = tree(ix, iz) {
                boxes.push((
                    DVec3::new(x - 0.35, h, z - 0.35),
                    DVec3::new(x + 0.35, h + size, z + 0.35),
                ));
                boxes.push((
                    DVec3::new(x - size * 0.17, h + size * 0.35, z - size * 0.17),
                    DVec3::new(x + size * 0.17, h + size * 0.85, z + size * 0.17),
                ));
            }
        }
    }
    boxes
}
pub fn color(x: f64, z: f64, h: f64) -> [f32; 4] {
    let grain = (0.92 + 0.08 * (x * 0.037 + z * 0.023).sin()) as f32;
    let c = if h < 0. {
        [0.10, 0.15, 0.12]
    } else if h > 1600. {
        [0.48, 0.48, 0.44]
    } else if h > 450. {
        [0.28, 0.30, 0.25]
    } else if forest(x, z) {
        [0.10, 0.20, 0.13]
    } else if meadow(x, z) {
        [0.31, 0.39, 0.19]
    } else {
        let field = noise((x / 420.).floor() as i32, (z / 650.).floor() as i32) as f32;
        [
            0.22 + field * 0.11,
            0.29 + field * 0.10,
            0.12 + field * 0.08,
        ]
    };
    [c[0] * grain, c[1] * grain, c[2] * grain, 1.]
}
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct Tile {
    x: i32,
    z: i32,
    size: i32,
}
fn leaves(tile: Tile, p: DVec3, agl: f64, out: &mut HashSet<Tile>) {
    let dx = (tile.x as f64 - p.x)
        .max(0.)
        .max(p.x - (tile.x + tile.size) as f64);
    let dz = (tile.z as f64 - p.z)
        .max(0.)
        .max(p.z - (tile.z + tile.size) as f64);
    if tile.size > 512 && dx.hypot(dz) < tile.size as f64 * 0.85 && agl < tile.size as f64 * 2. {
        let half = tile.size / 2;
        for (x, z) in [
            (tile.x, tile.z),
            (tile.x + half, tile.z),
            (tile.x, tile.z + half),
            (tile.x + half, tile.z + half),
        ] {
            leaves(Tile { x, z, size: half }, p, agl, out);
        }
    } else {
        out.insert(tile);
    }
}
fn layout(p: DVec3, agl: f64) -> HashSet<Tile> {
    let mut out = HashSet::new();
    for z in -8..4 {
        for x in -2..8 {
            leaves(
                Tile {
                    x: x * 32768,
                    z: z * 32768,
                    size: 32768,
                },
                p,
                agl,
                &mut out,
            );
        }
    }
    out
}
struct Entry {
    entity: Entity,
    mesh: Handle<Mesh>,
    trees: Vec<(Entity, Handle<Mesh>)>,
}
struct Built {
    tile: Tile,
    anchor: DVec3,
    mesh: Mesh,
    forest: Vec<Mesh>,
}
struct Pending {
    cell: (i32, i32, i32),
    wanted: HashSet<Tile>,
    task: Task<Vec<Built>>,
}
#[derive(Resource)]
pub struct Landscape {
    tiles: HashMap<Tile, Entry>,
    material: Handle<StandardMaterial>,
    trees: Handle<StandardMaterial>,
    cell: (i32, i32, i32),
    pending: Option<Pending>,
}
fn cell(f: &Flight) -> (i32, i32, i32) {
    (
        (f.position.x / 512.).floor() as i32,
        (f.position.z / 512.).floor() as i32,
        (f.agl().max(1.).log2() / 2.).floor() as i32,
    )
}
fn forest_mesh(tile: Tile, anchor: DVec3) -> Option<Mesh> {
    if tile.size > 2048 {
        return None;
    }
    let mut p: Vec<[f32; 3]> = vec![];
    let mut colors = vec![];
    let mut idx = vec![];
    let mut triangle = |v: [DVec3; 3], color: [f32; 4]| {
        let n = p.len() as u32;
        for point in v {
            p.push((point - anchor).as_vec3().to_array());
            colors.push(color);
        }
        idx.extend([n, n + 1, n + 2]);
    };
    for iz in tile.z / 32..(tile.z + tile.size) / 32 {
        for ix in tile.x / 32..(tile.x + tile.size) / 32 {
            let Some((x, z, h, height)) = tree(ix, iz) else {
                continue;
            };
            let origin = point(x, z, h);
            let up = origin.normalize();
            let right = DVec3::X;
            let aft = right.cross(up).normalize();
            for tier in 0..3 {
                let base = height * (0.15 + tier as f64 * 0.20);
                let top = height * (0.65 + tier as f64 * 0.175);
                let r = height * (0.24 - tier as f64 * 0.055);
                for n in 0..7 {
                    let a = n as f64 * std::f64::consts::TAU / 7.;
                    let b = (n + 1) as f64 * std::f64::consts::TAU / 7.;
                    let shade = 0.8 + (n % 3) as f32 * 0.1;
                    triangle(
                        [
                            origin + up * base + (right * a.cos() + aft * a.sin()) * r,
                            origin + up * top,
                            origin + up * base + (right * b.cos() + aft * b.sin()) * r,
                        ],
                        [0.09 * shade, 0.22 * shade, 0.115 * shade, 1.],
                    );
                }
            }
            for n in 0..5 {
                let a = n as f64 * std::f64::consts::TAU / 5.;
                let b = (n + 1) as f64 * std::f64::consts::TAU / 5.;
                let aa = (right * a.cos() + aft * a.sin()) * 0.26;
                let bb = (right * b.cos() + aft * b.sin()) * 0.26;
                let a0 = origin + aa;
                let b0 = origin + bb;
                let a1 = a0 + up * height * 0.55;
                let b1 = b0 + up * height * 0.55;
                triangle([a0, a1, b0], [0.19, 0.14, 0.09, 1.]);
                triangle([b0, a1, b1], [0.19, 0.14, 0.09, 1.]);
            }
        }
    }
    if p.is_empty() {
        return None;
    }
    let n = p.len();
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, p);
    m.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.]; n]);
    m.insert_indices(Indices::U32(idx));
    m.compute_smooth_normals();
    Some(m)
}

/// Preserve every original triangle/attribute, but give the renderer tighter
/// bounds. A single 2 km forest batch otherwise submits trees far outside the
/// ground-level camera and shadow cascades. Anchors stay identical, avoiding
/// an extra precision-changing vertex transform.
fn forest_batches(mesh: Mesh, size: f32) -> Vec<Mesh> {
    use bevy::mesh::VertexAttributeValues;
    let Some(VertexAttributeValues::Float32x3(positions)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else { return vec![mesh]; };
    let Some(VertexAttributeValues::Float32x3(normals)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL) else { return vec![mesh]; };
    let Some(VertexAttributeValues::Float32x4(colors)) = mesh.attribute(Mesh::ATTRIBUTE_COLOR) else { return vec![mesh]; };
    let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0) else { return vec![mesh]; };
    let Some(Indices::U32(indices)) = mesh.indices() else { return vec![mesh]; };
    let origin=positions.iter().copied().map(Vec3::from)
        .fold(Vec3::splat(f32::INFINITY),Vec3::min);
    type Batch = (Vec<[f32;3]>,Vec<[f32;3]>,Vec<[f32;4]>,Vec<[f32;2]>,Vec<u32>);
    let mut batches = std::collections::BTreeMap::<(i32,i32),Batch>::new();
    for triangle in indices.chunks_exact(3) {
        let centre = triangle.iter().map(|i|Vec3::from(positions[*i as usize])).sum::<Vec3>() / 3.;
        let key = (((centre.x-origin.x) / size).floor() as i32,((centre.z-origin.z) / size).floor() as i32);
        let batch = batches.entry(key).or_default();
        for index in triangle {
            let index = *index as usize;
            batch.4.push(batch.0.len() as u32);
            batch.0.push(positions[index]); batch.1.push(normals[index]);
            batch.2.push(colors[index]); batch.3.push(uvs[index]);
        }
    }
    batches.into_values().map(|(p,n,c,uv,indices)| {
        Mesh::new(PrimitiveTopology::TriangleList,RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION,p)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL,n)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR,c)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0,uv)
            .with_inserted_indices(Indices::U32(indices))
    }).collect()
}
fn build(tile: Tile) -> Built {
    let _measure = crate::performance::span(14);
    let anchor = surface_point(
        (tile.x + tile.size / 2) as f64,
        (tile.z + tile.size / 2) as f64,
        0.,
    );
    let mesh = world::terrain_mesh(tile.x as f64, tile.z as f64, tile.size as f64, 64, anchor);
    Built {
        tile,
        anchor,
        mesh,
        forest: forest_mesh(tile, anchor).map(|mesh| {
            if std::env::var_os("NORD_TERRAIN_LARGE_BATCHES").is_some()
                || std::env::var_os("NORD_TERRAIN_BATCH_SIZE").is_none() { vec![mesh] }
            else {
                let size=std::env::var("NORD_TERRAIN_BATCH_SIZE").ok()
                    .and_then(|s|s.parse::<f32>().ok()).filter(|s|s.is_finite())
                    .unwrap_or(256.).clamp(128.,2048.);
                forest_batches(mesh,size)
            }
        }).unwrap_or_default(),
    }
}
fn spawn(c: &mut Commands, meshes: &mut Assets<Mesh>, state: &mut Landscape, built: Built, f: &Flight) {
    let Built {
        tile,
        anchor,
        mesh,
        forest,
    } = built;
    // Streaming commits can be deferred until after update_world has run. A
    // visible tile must never spend its first frame at the camera-local origin.
    let transform = Transform::from_translation(world::local(anchor, f))
        .with_rotation(world::local_rot(DQuat::IDENTITY, f));
    let mesh = meshes.add(mesh);
    let entity = c
        .spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(state.material.clone()),
            transform,
            WorldObject {
                position: anchor,
                rotation: DQuat::IDENTITY,
            },
        ))
        .id();
    let trees = forest.into_iter().map(|m| {
        let handle = meshes.add(m);
        let e = c
            .spawn((
                ForestBatch,
                Mesh3d(handle.clone()),
                MeshMaterial3d(state.trees.clone()),
                transform,
                WorldObject {
                    position: anchor,
                    rotation: DQuat::IDENTITY,
                },
            ))
            .id();
        (e, handle)
    }).collect();
    state.tiles.insert(
        tile,
        Entry {
            entity,
            mesh,
            trees,
        },
    );
}
pub fn setup(
    c: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    f: &Flight,
) {
    let mut image = Image::new_fill(
        Extent3d {
            width: 128,
            height: 128,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[255, 255, 255, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    if let Some(data) = image.data.as_mut() {
        for z in 0..128 {
            for x in 0..128 {
                let n = (208. + noise(x, z) * 47.) as u8;
                let i = ((z * 128 + x) * 4) as usize;
                data[i..i + 4].copy_from_slice(&[n, n, n, 255]);
            }
        }
    }
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..ImageSamplerDescriptor::linear()
    });
    let material = mats.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(images.add(image)),
        perceptual_roughness: 0.97,
        ..default()
    });
    let trees = mats.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.95,
        cull_mode: None,
        ..default()
    });
    let mut state = Landscape {
        tiles: HashMap::new(),
        material,
        trees,
        cell: cell(f),
        pending: None,
    };
    for tile in layout(f.position, f.agl()) {
        spawn(c, meshes, &mut state, build(tile), f);
    }
    c.insert_resource(state);
}
pub fn update(
    mut c: Commands,
    f: Res<Flight>,
    mut state: ResMut<Landscape>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let opacity = crate::earth::terrain_opacity(f.altitude());
    let _measure = crate::performance::span(4);
    for handle in [&state.material, &state.trees] {
        let alpha_mode = if opacity > 0.999 { AlphaMode::Opaque } else { AlphaMode::Blend };
        if materials.get(handle).is_some_and(|m| m.base_color.alpha() != opacity || m.alpha_mode != alpha_mode) {
            let material = materials.get_mut(handle).unwrap();
            material.base_color = Color::srgba(1.0, 1.0, 1.0, opacity);
            material.alpha_mode = alpha_mode;
        }
    }
    if f.vehicle == Vehicle::Spacecraft {
        return;
    }
    if let Some(mut pending) = state.pending.take() {
        let Some(built) = block_on(poll_once(&mut pending.task)) else {
            state.pending = Some(pending);
            return;
        };
        // Keep the complete old coverage until every replacement is ready, then
        // retire and install together. At most one bounded job can be in flight.
        retire(&mut c, &mut meshes, &mut state, &pending.wanted);
        for tile in built {
            spawn(&mut c, &mut meshes, &mut state, tile, &f);
        }
        state.cell = pending.cell;
    }
    let next = cell(&f);
    if next == state.cell {
        return;
    }
    let wanted = layout(f.position, f.agl());
    let missing: Vec<_> = wanted
        .iter()
        .filter(|t| !state.tiles.contains_key(t))
        .copied()
        .collect();
    if missing.is_empty() {
        retire(&mut c, &mut meshes, &mut state, &wanted);
        state.cell = next;
        return;
    }
    let task =
        AsyncComputeTaskPool::get().spawn(async move { missing.into_iter().map(build).collect() });
    state.pending = Some(Pending {
        cell: next,
        wanted,
        task,
    });
}
fn retire(
    c: &mut Commands,
    meshes: &mut Assets<Mesh>,
    state: &mut Landscape,
    wanted: &HashSet<Tile>,
) {
    let retired: Vec<_> = state
        .tiles
        .keys()
        .filter(|t| !wanted.contains(t))
        .copied()
        .collect();
    for tile in retired {
        if let Some(entry) = state.tiles.remove(&tile) {
            c.entity(entry.entity).despawn();
            meshes.remove(entry.mesh.id());
            for (e, m) in entry.trees {
                c.entity(e).despawn();
                meshes.remove(m.id());
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forest_partition_preserves_every_triangle_attribute_and_tightens_culling() {
        use bevy::mesh::VertexAttributeValues;
        fn triangles(mesh: &Mesh) -> Vec<Vec<u32>> {
            let Some(VertexAttributeValues::Float32x3(p))=mesh.attribute(Mesh::ATTRIBUTE_POSITION) else {panic!()};
            let Some(VertexAttributeValues::Float32x3(n))=mesh.attribute(Mesh::ATTRIBUTE_NORMAL) else {panic!()};
            let Some(VertexAttributeValues::Float32x4(c))=mesh.attribute(Mesh::ATTRIBUTE_COLOR) else {panic!()};
            let Some(VertexAttributeValues::Float32x2(uv))=mesh.attribute(Mesh::ATTRIBUTE_UV_0) else {panic!()};
            let Some(Indices::U32(indices))=mesh.indices() else {panic!()};
            indices.chunks_exact(3).map(|triangle| triangle.iter().flat_map(|i| {
                let i=*i as usize;
                p[i].into_iter().chain(n[i]).chain(c[i]).chain(uv[i]).map(f32::to_bits)
            }).collect()).collect()
        }
        let tile=Tile{x:9728,z:0,size:512};
        let anchor=surface_point(9984.,256.,0.);
        let original=forest_mesh(tile,anchor).unwrap();
        let mut expected=triangles(&original); expected.sort();
        let batches=forest_batches(original,256.);
        assert!(batches.len()>1);
        let mut actual:Vec<_>=batches.iter().flat_map(triangles).collect();actual.sort();
        assert_eq!(expected,actual);
        let mut rejected=0;
        for batch in &batches {
            let Some(VertexAttributeValues::Float32x3(p))=batch.attribute(Mesh::ATTRIBUTE_POSITION) else {panic!()};
            let min=p.iter().map(|p|Vec3::from(*p)).fold(Vec3::splat(f32::INFINITY),Vec3::min);
            let max=p.iter().map(|p|Vec3::from(*p)).fold(Vec3::splat(f32::NEG_INFINITY),Vec3::max);
            assert!(max.x-min.x<300. && max.z-min.z<300.);
            if max.x < 40. { rejected+=batch.count_vertices(); }
        }
        assert!(rejected>0,"off-screen half of the original batch can now be culled");
    }
    #[test]
    fn background_transition_keeps_old_coverage_then_commits_complete_children() {
        use bevy::asset::{AssetApp, AssetPlugin};
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        let mut app = App::new();
        app.add_plugins((bevy::app::TaskPoolPlugin::default(), AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .insert_resource(Flight::runway())
            .add_systems(Update, update);
        let f = app.world().resource::<Flight>().clone();
        let next = cell(&f);
        let material = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Cuboid::default());
        let old = app.world_mut().spawn_empty().id();
        let wanted: HashSet<_> = [(0, 0), (512, 0), (0, 512), (512, 512)]
            .map(|(x, z)| Tile { x, z, size: 512 })
            .into_iter()
            .collect();
        let work = wanted.clone();
        let release = Arc::new(AtomicBool::new(false));
        let signal = release.clone();
        let task = AsyncComputeTaskPool::get().spawn(async move {
            while !signal.load(Ordering::Acquire) {
                bevy::tasks::futures_lite::future::yield_now().await;
            }
            work.into_iter().map(build).collect()
        });
        app.insert_resource(Landscape {
            tiles: HashMap::from([(
                Tile {
                    x: 0,
                    z: 0,
                    size: 1024,
                },
                Entry {
                    entity: old,
                    mesh,
                    trees: Vec::new(),
                },
            )]),
            material: material.clone(),
            trees: material,
            cell: (next.0 - 1, next.1, next.2),
            pending: Some(Pending {
                cell: next,
                wanted: wanted.clone(),
                task,
            }),
        });
        app.update();
        assert!(app.world().get_entity(old).is_ok());
        assert_eq!(app.world().resource::<Landscape>().tiles.len(), 1);
        release.store(true, Ordering::Release);
        let started = std::time::Instant::now();
        while app.world().resource::<Landscape>().pending.is_some() {
            assert!(started.elapsed().as_secs() < 10);
            std::thread::sleep(std::time::Duration::from_millis(1));
            app.update();
        }
        assert!(app.world().get_entity(old).is_err());
        assert_eq!(
            app.world()
                .resource::<Landscape>()
                .tiles
                .keys()
                .copied()
                .collect::<HashSet<_>>(),
            wanted
        );
        // No update_world system is registered in this test: the committed
        // ground and every tree batch must already be correct on their first
        // visible frame, independent of scheduler/deferred-command ordering.
        let f=app.world().resource::<Flight>();
        let mut count=0;
        for entry in app.world().resource::<Landscape>().tiles.values() {
            for entity in std::iter::once(entry.entity).chain(entry.trees.iter().map(|x|x.0)) {
                let object=app.world().get::<WorldObject>(entity).unwrap();
                let transform=app.world().get::<Transform>(entity).unwrap();
                assert_eq!(transform.translation,world::local(object.position,f));
                assert_eq!(transform.rotation,world::local_rot(object.rotation,f));
                assert_ne!(*transform,Transform::default());
                count+=1;
            }
        }
        assert!(count>=4);
    }
    #[test]
    fn lod_has_no_overlapping_tiles_and_stays_bounded() {
        for p in [
            DVec3::new(0., RADIUS, 0.),
            DVec3::new(100000., RADIUS, -150000.),
        ] {
            let tiles = layout(p, 10.);
            assert!(tiles.len() < 500);
            assert!(tiles.iter().any(|t| t.size == 512));
            for a in &tiles {
                for b in &tiles {
                    if a == b {
                        continue;
                    }
                    assert!(
                        a.x + a.size <= b.x
                            || b.x + b.size <= a.x
                            || a.z + a.size <= b.z
                            || b.z + b.size <= a.z
                    );
                }
            }
        }
    }
    #[test]
    fn meadow_is_flat_open_and_normals_are_outward() {
        assert_eq!(terrain_height(6500., 3200.), 62.);
        assert!(!forest(6500., 3200.));
        let p = surface_point(6500., 3200., 62.);
        assert!(normal(p).dot(p.normalize()) > 0.99999);
        assert!(paved(0., 0.));
        assert!(!paved(6500., 3200.));
    }
    #[test]
    fn off_field_landing_above_old_speed_limit_rolls_to_a_stop() {
        let mut f = Flight::runway();
        f.position = surface_point(6500., 3900., 62. + 1.70);
        f.velocity = f.forward() * 70. - f.up() * 1.5;
        f.landed = false;
        f.throttle = 0.;
        f.spool = 0.;
        f.engine = false;
        f.brakes = true;
        let mut e = Environment::default();
        e.wind_enabled = false;
        for _ in 0..2400 {
            e.advance(1. / 120.);
            simulate(&mut f, &e, 1. / 120.);
            if f.crashed {
                break;
            }
        }
        assert!(
            !f.crashed,
            "{} / altitude {} / speed {}",
            f.status,
            f.agl(),
            f.velocity.length()
        );
        assert!(f.landed);
        assert!(f.velocity.length() < 1.);
        assert!(f.status.contains("OFF-FIELD"));
    }
    #[test]
    fn gear_up_ground_strikes_remain_destructive() {
        let mut f = Flight::runway();
        f.position = surface_point(6500., 3200., 62. + 0.4);
        f.gear = false;
        f.gear_extension = 0.;
        f.landed = false;
        f.velocity = -f.up() * 2.;
        simulate(&mut f, &Environment::default(), 1. / 120.);
        assert!(f.crashed);
    }
    #[test]
    fn generated_trees_have_matching_collision_boxes_and_clearings_stay_clear() {
        let mut found = false;
        'search: for iz in -120..120 {
            for ix in 20..150 {
                if let Some((x, z, h, _)) = tree(ix, iz) {
                    let p = surface_point(x, z, h + 4.);
                    assert!(!obstacles(p, p).is_empty());
                    found = true;
                    break 'search;
                }
            }
        }
        assert!(found);
        let p = surface_point(6500., 3200., 64.);
        assert!(obstacles(p, p).is_empty());
    }
}
