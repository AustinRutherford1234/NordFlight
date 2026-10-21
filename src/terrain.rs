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
use std::collections::{HashMap, HashSet, VecDeque};

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
#[cfg(test)]
pub fn obstacles(a: DVec3, b: DVec3) -> Vec<(DVec3, DVec3)> {
    obstacle_iter(a,b).collect()
}
thread_local!{static OBSTACLE_SCRATCH:std::cell::RefCell<Vec<(DVec3,DVec3)>>=const{std::cell::RefCell::new(Vec::new())};}
pub struct ObstacleScratch(Vec<(DVec3,DVec3)>);
impl std::ops::Deref for ObstacleScratch{type Target=[(DVec3,DVec3)];fn deref(&self)->&Self::Target{&self.0}}
impl Drop for ObstacleScratch{fn drop(&mut self){OBSTACLE_SCRATCH.with(|s|{let mut s=s.borrow_mut();if self.0.capacity()>s.capacity(){*s=std::mem::take(&mut self.0);s.clear();}});}}
pub fn obstacles_reused(a:DVec3,b:DVec3)->ObstacleScratch{
    let mut data=OBSTACLE_SCRATCH.with(|s|std::mem::take(&mut *s.borrow_mut()));
    data.clear();data.extend(obstacle_iter(a,b));ObstacleScratch(data)
}
pub fn obstacle_iter(a:DVec3,b:DVec3)->impl Iterator<Item=(DVec3,DVec3)>+Clone {
    let near=a.length().min(b.length()) - RADIUS - terrain_height(b.x,b.z)<=30.;
    let x0=((a.x.min(b.x)-16.)/32.).floor()as i32;
    let x1=((a.x.max(b.x)+16.)/32.).floor()as i32;
    let z0=((a.z.min(b.z)-16.)/32.).floor()as i32;
    let z1=((a.z.max(b.z)+16.)/32.).floor()as i32;
    (z0..=z1).take(if near{usize::MAX}else{0}).flat_map(move|iz|(x0..=x1).filter_map(move|ix|tree(ix,iz)))
        .flat_map(|(x,z,h,size)|[
                (
                    DVec3::new(x - 0.35, h, z - 0.35),
                    DVec3::new(x + 0.35, h + size, z + 0.35),
                ),
                (
                    DVec3::new(x - size * 0.17, h + size * 0.35, z - size * 0.17),
                    DVec3::new(x + size * 0.17, h + size * 0.85, z + size * 0.17),
                ),
        ])
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
    anchor: DVec3,
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
    coalesced: bool,
}
fn retarget_built(mut built:Vec<Built>,wanted:HashSet<Tile>,active:HashSet<Tile>)->Vec<Built> {
    // Run on the worker: obsolete CPU meshes can be large to destroy. Retain
    // byte-identical useful results and build only genuinely missing tiles.
    built.retain(|b|wanted.contains(&b.tile) && !active.contains(&b.tile));
    let retained:HashSet<_>=built.iter().map(|b|b.tile).collect();
    let mut missing:Vec<_>=wanted.into_iter().filter(|t|!active.contains(t)&&!retained.contains(t)).collect();
    missing.sort_by_key(|t|(t.size,t.x,t.z));
    built.extend(missing.into_iter().map(build));
    built.sort_by_key(|b|(b.tile.size,b.tile.x,b.tile.z));
    built
}
struct Preparing {
    cell: (i32,i32,i32),
    wanted: HashSet<Tile>,
    remaining: VecDeque<Built>,
    current: Option<StagingTile>,
    ready: Vec<(Tile,Entry)>,
    requested: bool,
}
struct StagingTile {
    tile: Tile,
    anchor: DVec3,
    ground: Option<Mesh>,
    forest: VecDeque<Mesh>,
    entry: Option<Entry>,
}
impl From<Built> for StagingTile {
    fn from(b:Built)->Self {
        Self {tile:b.tile,anchor:b.anchor,ground:Some(b.mesh),forest:b.forest.into(),entry:None}
    }
}
const STREAM_BYTES:usize=2*1024*1024;
const STREAM_MESHES:usize=4;
#[derive(Default)]
struct StreamBudget { bytes:usize, meshes:usize }
impl StreamBudget {
    fn accept(&mut self,bytes:usize)->bool {
        if self.meshes>=STREAM_MESHES || self.meshes>0 && self.bytes.saturating_add(bytes)>STREAM_BYTES {return false;}
        // An oversized developer/reference mesh must still make progress.
        self.bytes+=bytes;self.meshes+=1;true
    }
}
fn mesh_bytes(mesh:&Mesh)->usize {
    mesh.attributes().map(|(_,v)|v.get_bytes().len()).sum::<usize>()+
        mesh.indices().map_or(0,|i|match i{Indices::U16(v)=>v.len()*2,Indices::U32(v)=>v.len()*4})
}
#[derive(Resource)]
pub struct Landscape {
    tiles: HashMap<Tile, Entry>,
    material: Handle<StandardMaterial>,
    trees: Handle<StandardMaterial>,
    cell: (i32, i32, i32),
    pending: Option<Pending>,
    preparing: Option<Preparing>,
    retiring: VecDeque<Entry>,
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
                { vec![mesh] }
            else {
                let size=std::env::var("NORD_TERRAIN_BATCH_SIZE").ok()
                    .and_then(|s|s.parse::<f32>().ok()).filter(|s|s.is_finite())
                    .unwrap_or(512.).clamp(128.,2048.);
                forest_batches(mesh,size)
            }
        }).unwrap_or_default(),
    }
}
fn spawn(c: &mut Commands, meshes: &mut Assets<Mesh>, state: &Landscape, built: Built, f: &Flight, visible: bool) -> (Tile,Entry) {
    let Built {
        tile,
        anchor,
        mesh,
        forest,
    } = built;
    // Streaming commits can be deferred until after update_world has run. A
    // visible tile must never spend its first frame at the camera-local origin.
    let (entity,mesh)=spawn_piece(c,meshes,state,anchor,mesh,f,visible,false);
    let trees=forest.into_iter().map(|m| {
        let piece=spawn_piece(c,meshes,state,anchor,m,f,visible,true);
        attach_forest(c,entity,piece.0);piece
    }).collect();
    (tile,Entry{anchor,entity,mesh,trees})
}
fn shared_transforms()->bool {
    static ENABLED:std::sync::OnceLock<bool>=std::sync::OnceLock::new();
    *ENABLED.get_or_init(||std::env::var_os("NORD_TERRAIN_TRANSFORM_REFERENCE").is_none())
}
fn attach_forest(c:&mut Commands,root:Entity,child:Entity) {
    if shared_transforms() {
        c.entity(child).remove::<WorldObject>().insert(Transform::IDENTITY);
        c.entity(root).add_child(child);
    }
}
fn spawn_piece(c:&mut Commands,meshes:&mut Assets<Mesh>,state:&Landscape,anchor:DVec3,
    mesh:Mesh,f:&Flight,visible:bool,forest:bool)->(Entity,Handle<Mesh>) {
    let transform = Transform::from_translation(world::local(anchor, f))
        .with_rotation(world::local_rot(DQuat::IDENTITY, f));
    let mesh = meshes.add(mesh);
    let entity = c
        .spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(if forest {state.trees.clone()} else {state.material.clone()}),
            transform,
            if visible {Visibility::Inherited} else {Visibility::Hidden},
            WorldObject {
                position: anchor,
                rotation: DQuat::IDENTITY,
            },
        ))
        .id();
    if forest {c.entity(entity).insert(ForestBatch);}
    (entity,mesh)
}
fn stage(c:&mut Commands,meshes:&mut Assets<Mesh>,state:&Landscape,p:&mut Preparing,f:&Flight)->StreamBudget {
    let mut budget=StreamBudget::default();
    loop {
        if p.current.is_none() {p.current=p.remaining.pop_front().map(Into::into);}
        let Some(tile)=p.current.as_mut() else {break;};
        let next=tile.ground.as_ref().or_else(||tile.forest.front());
        if let Some(next)=next {
            if !budget.accept(mesh_bytes(next)) {break;}
        } else {
            let tile=p.current.take().unwrap();
            p.ready.push((tile.tile,tile.entry.expect("ground precedes forest")));
            continue;
        }
        if let Some(mesh)=tile.ground.take() {
            let (entity,mesh)=spawn_piece(c,meshes,state,tile.anchor,mesh,f,false,false);
            tile.entry=Some(Entry {anchor:tile.anchor,entity,mesh,trees:Vec::new()});
        } else {
            let mesh=tile.forest.pop_front().unwrap();
            let piece=spawn_piece(c,meshes,state,tile.anchor,mesh,f,false,true);
            attach_forest(c,tile.entry.as_ref().unwrap().entity,piece.0);
            tile.entry.as_mut().unwrap().trees.push(piece);
        }
    }
    budget
}
fn reclaim(c:&mut Commands,meshes:&mut Assets<Mesh>,state:&mut Landscape)->StreamBudget {
    let mut budget=StreamBudget::default();
    while let Some(entry)=state.retiring.front_mut() {
        let handle=entry.trees.last().map_or(&entry.mesh,|(_,m)|m);
        if !budget.accept(meshes.get(handle).map_or(0,mesh_bytes)) {break;}
        if let Some((entity,mesh))=entry.trees.pop() {
            c.entity(entity).despawn();meshes.remove(mesh.id());
        } else {
            let entry=state.retiring.pop_front().unwrap();
            c.entity(entry.entity).despawn();meshes.remove(entry.mesh.id());
        }
    }
    budget
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
        preparing: None,
        retiring: VecDeque::new(),
    };
    for tile in layout(f.position, f.agl()) {
        let (tile,entry)=spawn(c, meshes, &state, build(tile), f, true);
        state.tiles.insert(tile,entry);
    }
    c.insert_resource(state);
}
pub fn update(
    mut c: Commands,
    f: Res<Flight>,
    mut state: ResMut<Landscape>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    readiness: Res<crate::terrain_ready::TerrainReady>,
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
    let reclaimed=reclaim(&mut c,&mut meshes,&mut state);
    // Keep memory bounded to the active layout and one retiring/preparing
    // replacement. Hidden old assets are reclaimed before starting another.
    if reclaimed.meshes>0 {return;}
    if let Some(mut pending) = state.pending.take() {
        let Some(built) = block_on(poll_once(&mut pending.task)) else {
            state.pending = Some(pending);
            return;
        };
        let current=cell(&f);
        if !pending.coalesced && pending.cell!=current {
            let wanted=layout(f.position,f.agl());
            if wanted!=pending.wanted {
                let active=state.tiles.keys().copied().collect();let target=wanted.clone();
                let task=AsyncComputeTaskPool::get().spawn(async move {retarget_built(built,target,active)});
                // Coalesce once, then complete that valid coverage. Requiring
                // perfect cell currency every poll would starve moving flight.
                state.pending=Some(Pending {cell:current,wanted,task,coalesced:true});
                return;
            }
            pending.cell=current;
        }
        state.preparing=Some(Preparing {cell:pending.cell,wanted:pending.wanted,remaining:built.into(),current:None,ready:vec![],requested:false});
    }
    if let Some(mut preparing)=state.preparing.take() {
        // Bound individual mesh insertion by bytes and count, not entire tiles.
        // Existing coverage stays visible throughout CPU and GPU preparation.
        let budget=stage(&mut c,&mut meshes,&state,&mut preparing,&f);
        crate::performance::terrain_staged(budget.bytes,budget.meshes);
        if !preparing.remaining.is_empty() || preparing.current.is_some() {
            state.preparing=Some(preparing); return;
        }
        if !preparing.requested {
            let ids=preparing.ready.iter().flat_map(|(_,e)|std::iter::once(e.mesh.id()).chain(e.trees.iter().map(|(_,m)|m.id()))).collect();
            readiness.request(ids); preparing.requested=true;
            state.preparing=Some(preparing); return;
        }
        if !readiness.ready() {state.preparing=Some(preparing);return;}
        retire(&mut c,&mut meshes,&mut state,&preparing.wanted);
        for (tile,entry) in preparing.ready {
            let transform=Transform::from_translation(world::local(entry.anchor,&f)).with_rotation(world::local_rot(DQuat::IDENTITY,&f));
            c.entity(entry.entity).insert((Visibility::Inherited,transform));
            for (entity,_) in &entry.trees {
                c.entity(*entity).insert(Visibility::Inherited);
                if !shared_transforms() {c.entity(*entity).insert(transform);}
            }
            state.tiles.insert(tile,entry);
        }
        readiness.clear();
        state.cell=preparing.cell;
        let reclaimed=reclaim(&mut c,&mut meshes,&mut state);
        // Even when this emptied the queue, a stale completed cell must not
        // retire a second layout with a fresh budget during this same update.
        if reclaimed.meshes>0 {return;}
    }
    let next = cell(&f);
    if next == state.cell {
        return;
    }
    let wanted = layout(f.position, f.agl());
    let mut missing: Vec<_> = wanted
        .iter()
        .filter(|t| !state.tiles.contains_key(t))
        .copied()
        .collect();
    if missing.is_empty() {
        retire(&mut c, &mut meshes, &mut state, &wanted);
        state.cell = next;
        reclaim(&mut c,&mut meshes,&mut state);
        return;
    }
    missing.sort_by_key(|t|(t.size,t.x,t.z));
    let task =
        AsyncComputeTaskPool::get().spawn(async move { missing.into_iter().map(build).collect() });
    state.pending = Some(Pending {
        cell: next,
        wanted,
        task,
        coalesced: false,
    });
}
fn retire(
    c: &mut Commands,
    _meshes: &mut Assets<Mesh>,
    state: &mut Landscape,
    wanted: &HashSet<Tile>,
) {
    let mut retired: Vec<_> = state
        .tiles
        .keys()
        .filter(|t| !wanted.contains(t))
        .copied()
        .collect();
    retired.sort_by_key(|t|(t.size,t.x,t.z));
    for tile in retired {
        if let Some(entry) = state.tiles.remove(&tile) {
            c.entity(entry.entity).insert(Visibility::Hidden);
            for (entity,_) in &entry.trees {c.entity(*entity).insert(Visibility::Hidden);}
            state.retiring.push_back(entry);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_worker_coalescing_reuses_useful_meshes_and_completes_coverage() {
        let a=Tile{x:0,z:0,size:512};let b=Tile{x:512,z:0,size:512};
        let c=Tile{x:1024,z:0,size:512};let d=Tile{x:1536,z:0,size:512};
        let fake=|tile|Built {tile,anchor:DVec3::ZERO,mesh:Mesh::from(Cuboid::default()),forest:vec![]};
        let result=retarget_built(vec![fake(b),fake(c)],HashSet::from([a,b,d]),HashSet::from([a]));
        assert_eq!(result.iter().map(|b|b.tile).collect::<HashSet<_>>(),HashSet::from([b,d]));
        assert_eq!(result.iter().find(|x|x.tile==b).unwrap().mesh.count_vertices(),24,"useful result must not be regenerated");
        let rebuilt=result.iter().find(|x|x.tile==d).unwrap();let reference=build(d);
        assert_eq!(rebuilt.mesh.attribute(Mesh::ATTRIBUTE_POSITION),reference.mesh.attribute(Mesh::ATTRIBUTE_POSITION));
    }
    #[test]
    fn streaming_budget_bounds_meshes_bytes_and_allows_oversized_progress() {
        let mut b=StreamBudget::default();
        assert!(b.accept(STREAM_BYTES-1));
        assert!(!b.accept(2));
        assert!(b.accept(1));
        assert!(!b.accept(1));
        let mut b=StreamBudget::default();
        assert!(b.accept(STREAM_BYTES+1));
        assert!(!b.accept(1));
        let mut b=StreamBudget::default();
        for _ in 0..STREAM_MESHES {assert!(b.accept(1));}
        assert!(!b.accept(1));
        let tile=Tile{x:8192,z:0,size:2048};
        let anchor=surface_point(9216.,1024.,0.);
        let forest=forest_mesh(tile,anchor).unwrap();
        let vertices=forest.count_vertices();
        let batches=forest_batches(forest,512.);
        assert_eq!(vertices,batches.iter().map(Mesh::count_vertices).sum::<usize>());
        assert!(batches.len()>1);
        assert!(batches.iter().all(|m|mesh_bytes(m)<=STREAM_BYTES));
    }
    #[test]
    fn staged_forest_and_retirement_are_bounded_and_publish_atomically() {
        use bevy::asset::{AssetApp,AssetPlugin};
        for old_tree_count in [0_usize,10] {
        let mut app=App::new();
        app.add_plugins(AssetPlugin::default()).init_asset::<Mesh>().init_asset::<StandardMaterial>()
            .insert_resource(Flight::runway()).init_resource::<crate::terrain_ready::TerrainReady>()
            .add_systems(Update,update);
        let f=app.world().resource::<Flight>().clone();
        let material=app.world_mut().resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial::default());
        let old_mesh=app.world_mut().resource_mut::<Assets<Mesh>>().add(Cuboid::default());
        let old=app.world_mut().spawn(Visibility::Inherited).id();
        let mut old_trees=Vec::new();
        for _ in 0..old_tree_count {
            let mesh=app.world_mut().resource_mut::<Assets<Mesh>>().add(Cuboid::default());
            old_trees.push((app.world_mut().spawn(Visibility::Inherited).id(),mesh));
        }
        let old_entities:Vec<_>=std::iter::once(old).chain(old_trees.iter().map(|x|x.0)).collect();
        let tile=Tile{x:0,z:0,size:512};
        app.insert_resource(Landscape {tiles:HashMap::from([(Tile{x:0,z:0,size:1024},Entry {
            anchor:DVec3::ZERO,entity:old,mesh:old_mesh,trees:old_trees,
        })]),material:material.clone(),trees:material,cell:cell(&f),pending:None,retiring:VecDeque::new(),
            preparing:Some(Preparing {cell:(cell(&f).0-1,cell(&f).1,cell(&f).2),wanted:HashSet::from([tile]),remaining:VecDeque::from([Built {
                tile,anchor:f.position,mesh:Mesh::from(Cuboid::default()),
                forest:(0..20).map(|_|Mesh::from(Cuboid::default())).collect(),
            }]),current:None,ready:vec![],requested:false})});
        for i in 0..6 {
            app.update();
            assert_eq!(app.world().resource::<Assets<Mesh>>().len(),old_tree_count+1+((i+1)*4).min(21));
            assert_eq!(*app.world().get::<Visibility>(old).unwrap(),Visibility::Inherited);
            assert_eq!(app.world().resource::<Landscape>().tiles.len(),1);
        }
        app.world().resource::<crate::terrain_ready::TerrainReady>().poll(|_|true);
        app.update();
        assert!(app.world().resource::<Landscape>().tiles.contains_key(&tile));
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(),21+(old_tree_count+1).saturating_sub(4));
        assert!(app.world().resource::<Landscape>().pending.is_none(),"cleanup must finish the update even if the completed cell is stale");
        for entity in &old_entities {
            if let Some(v)=app.world().get::<Visibility>(*entity) {assert_eq!(*v,Visibility::Hidden);}
        }
        for _ in 0..(old_tree_count+1).saturating_sub(4).div_ceil(4) {app.update();}
        assert!(old_entities.iter().all(|e|app.world().get_entity(*e).is_err()));
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(),21);
        assert!(app.world().resource::<Landscape>().retiring.is_empty());
        assert!(app.world().resource::<Landscape>().pending.is_none());
        }
    }
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
        app.add_plugins((bevy::app::TaskPoolPlugin::default(), AssetPlugin::default(),bevy::transform::TransformPlugin))
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .insert_resource(Flight::runway())
            .init_resource::<crate::terrain_ready::TerrainReady>()
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
                    anchor: DVec3::ZERO,
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
                coalesced:false,
            }),
            preparing: None,
            retiring:VecDeque::new(),
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
        while app.world().resource::<Landscape>().preparing.as_ref().is_some_and(|p|!p.requested) {
            app.update();
            assert!(app.world().get_entity(old).is_ok());
        }
        assert!(app.world().get_entity(old).is_ok());
        app.update(); // GPU not ready: no premature publication.
        assert!(app.world().get_entity(old).is_ok());
        app.world().resource::<crate::terrain_ready::TerrainReady>().poll(|_|true);
        app.update();
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
                let transform=app.world().get::<Transform>(entity).unwrap();
                if entity==entry.entity {
                    let object=app.world().get::<WorldObject>(entity).unwrap();
                    assert_eq!(transform.translation,world::local(object.position,f));
                    assert_eq!(transform.rotation,world::local_rot(object.rotation,f));
                    assert_ne!(*transform,Transform::default());
                } else {
                    assert!(app.world().get::<WorldObject>(entity).is_none());
                    assert_eq!(*transform,Transform::IDENTITY);
                    assert_eq!(app.world().get::<GlobalTransform>(entity).unwrap(),app.world().get::<GlobalTransform>(entry.entity).unwrap());
                }
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
