//! Shared opaque vegetation meshes. Bevy batches equal mesh/material handles;
//! cells own stable transforms and bounds, and placement is simulation-owned.
use crate::{
    environment,
    physics::Flight,
    world::{self, WorldObject},
};
use bevy::{
    asset::RenderAssetUsages,
    math::{DQuat, DVec3},
    mesh::Indices,
    prelude::*,
    render::render_resource::PrimitiveTopology,
};
use std::collections::{HashMap, HashSet};
const CELL: i32 = 256;
struct Cell { entity:Option<Entity>, near:Handle<Mesh>, far:Handle<Mesh>, props:Handle<Mesh> }
struct BuiltCell { near:Mesh, far:Mesh, props:Mesh, anchor:DVec3 }
struct Models { near:Vec<Mesh>, far:Vec<Mesh>, rock:Mesh, building:Mesh }
#[derive(Resource)]
pub(crate) struct Vegetation {
    cells:HashMap<(i32,i32),Cell>,
    radius:f64,
    jobs:HashMap<(i32,i32),bevy::tasks::Task<BuiltCell>>,
    source:std::sync::Arc<Models>,
    material:Handle<StandardMaterial>,prop_material:Handle<StandardMaterial>,
}
pub struct VegetationPlugin;
impl Plugin for VegetationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(super::grass::GrassPlugin).add_systems(PostStartup, setup)
            .add_systems(Update, update.in_set(crate::FlightStage::World));
    }
}
fn model(species: usize, far: bool) -> Mesh {
    let mut p = vec![];
    let mut colors = vec![];
    let mut idx = vec![];
    let mut tri = |v: [[f32; 3]; 3], color: [f32; 4]| {
        let base = p.len() as u32;
        p.extend(v);
        colors.extend([color; 3]);
        idx.extend([base, base + 1, base + 2]);
    };
    let seed = environment::active().preset.seed;
    let palette = environment::active().preset.palette[0];
    let leaf = |shade: f32| {
        [
            environment::srgb_to_linear(palette[0] * shade),
            environment::srgb_to_linear(palette[1] * shade),
            environment::srgb_to_linear(palette[2] * shade),
            1.,
        ]
    };
    let sides = if far { 4 } else { 7 };
    // Irregular opaque crown clusters give broadleaf trees a different
    // silhouette from the evergreen branch whorls without foliage overdraw.
    if species == 2 {
        let count = if far { 3 } else { 8 };
        for cluster in 0..count {
            let angle = cluster as f32 * 2.399963;
            let t = cluster as f32 / count as f32;
            let cx = angle.cos() * 0.12 * (1. - t);
            let cz = angle.sin() * 0.12 * (1. - t);
            let cy = 0.44 + t * 0.44;
            let r = 0.15 * (1. - t * 0.35);
            let rings = if far { 2 } else { 3 };
            for ring in 0..rings {
                let phi0 = -std::f32::consts::FRAC_PI_2
                    + std::f32::consts::PI * ring as f32 / rings as f32;
                let phi1 = -std::f32::consts::FRAC_PI_2
                    + std::f32::consts::PI * (ring + 1) as f32 / rings as f32;
                for side in 0..sides {
                    let a = side as f32 * std::f32::consts::TAU / sides as f32;
                    let b = (side + 1) as f32 * std::f32::consts::TAU / sides as f32;
                    let v = |phi: f32, theta: f32| {
                        [
                            cx + theta.cos() * phi.cos() * r,
                            cy + phi.sin() * r * 1.3,
                            cz + theta.sin() * phi.cos() * r,
                        ]
                    };
                    let shade = 0.85 + environment::hash(side, cluster, seed) as f32 * 0.35;
                    tri([v(phi0, a), v(phi1, a), v(phi0, b)], leaf(shade));
                    tri([v(phi0, b), v(phi1, a), v(phi1, b)], leaf(shade));
                }
            }
        }
    } else {
        let tiers = if far { 3 } else { 7 };
        for tier in 0..tiers {
            let f = tier as f32 / tiers as f32;
            let y = 0.16 + f * 0.68;
            let r = (if species == 0 { 0.25 } else { 0.18 }) * (1. - f * 0.84);
            let phase = tier as f32 * 1.71;
            for side in 0..sides {
                let a = phase + side as f32 * std::f32::consts::TAU / sides as f32;
                let b = phase + (side + 1) as f32 * std::f32::consts::TAU / sides as f32;
                let jitter = 0.75 + environment::hash(side, tier, seed) as f32 * 0.5;
                let tip = [a.cos() * r * jitter, y - 0.03, a.sin() * r * jitter];
                let next = [b.cos() * r, y - 0.02, b.sin() * r];
                let top = [0., (y + 0.30).min(1.), 0.];
                tri([tip, top, next], leaf(0.7 + f * 0.3 + jitter * 0.1));
                if !far {
                    tri([tip, next, [0., y - 0.07, 0.]], leaf(0.65));
                }
            }
        }
    }
    if !far {
        for side in 0..5 {
            let a = side as f32 * std::f32::consts::TAU / 5.;
            let b = (side + 1) as f32 * std::f32::consts::TAU / 5.;
            let (a, b) = (
                [a.cos() * 0.015, 0., a.sin() * 0.015],
                [b.cos() * 0.015, 0., b.sin() * 0.015],
            );
            let c = if species == 2 {
                [0.6, 0.58, 0.52, 1.]
            } else {
                [0.19, 0.12, 0.065, 1.]
            };
            tri([a, [a[0], 0.65, a[2]], b], c);
            tri([b, [a[0], 0.65, a[2]], [b[0], 0.65, b[2]]], c);
        }
    }
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, p);
    m.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    m.insert_indices(Indices::U32(idx));
    m.compute_smooth_normals();
    m
}
#[derive(Default)]
struct Batch { positions:Vec<[f32;3]>, normals:Vec<[f32;3]>, colors:Vec<[f32;4]>, indices:Vec<u32> }
impl Batch {
    fn append(&mut self, mesh:&Mesh, transform:Transform) {
        use bevy::mesh::VertexAttributeValues;
        let Some(VertexAttributeValues::Float32x3(p))=mesh.attribute(Mesh::ATTRIBUTE_POSITION)else{return;};
        let Some(VertexAttributeValues::Float32x3(n))=mesh.attribute(Mesh::ATTRIBUTE_NORMAL)else{return;};
        let color=match mesh.attribute(Mesh::ATTRIBUTE_COLOR) {Some(VertexAttributeValues::Float32x4(c))=>Some(c),_=>None};
        let base=self.positions.len()as u32;
        for (i,(p,n))in p.iter().zip(n).enumerate() {
            self.positions.push(transform.transform_point(Vec3::from_array(*p)).to_array());
            self.normals.push((transform.rotation*(Vec3::from_array(*n)/transform.scale)).normalize_or_zero().to_array());
            self.colors.push(color.map_or([1.;4],|c|c[i]));
        }
        if let Some(indices)=mesh.indices(){self.indices.extend(indices.iter().map(|i|base+i as u32));}
        else{self.indices.extend(base..self.positions.len()as u32);}
    }
    fn mesh(self)->Mesh {
        Mesh::new(PrimitiveTopology::TriangleList,RenderAssetUsages::RENDER_WORLD)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION,self.positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL,self.normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR,self.colors)
            .with_inserted_indices(Indices::U32(self.indices))
    }
}
fn build_cell(cx:i32,cz:i32,models:&Models)->BuiltCell {
    let x=cx*CELL;let z=cz*CELL;
    let anchor=environment::point((x+CELL/2)as f64,(z+CELL/2)as f64,0.,environment::radius());
    let(mut near,mut far,mut props)=(Batch::default(),Batch::default(),Batch::default());
    for iz in z/32..(z+CELL)/32 {for ix in x/32..(x+CELL)/32 {
        let Some((tx,tz,h,size))=environment::tree(ix,iz)else{continue;};
        let species=environment::tree_species(ix,iz);
        let origin=environment::point(tx,tz,h,environment::radius());
        let rotation=DQuat::from_rotation_arc(DVec3::Y,origin.normalize())
            *DQuat::from_rotation_y(environment::hash(ix+7,iz,9)*std::f64::consts::TAU);
        let transform=Transform::from_translation((origin-anchor).as_vec3())
            .with_rotation(rotation.as_quat()).with_scale(Vec3::splat(size as f32));
        near.append(&models.near[species],transform);far.append(&models.far[species],transform);
    }}
    for iz in z/128..(z+CELL)/128 {for ix in x/128..(x+CELL)/128 {
        if let Some(prop)=environment::prop(ix,iz){
            props.append(if prop.rock{&models.rock}else{&models.building},
                Transform::from_translation((prop.position-anchor).as_vec3())
                .with_rotation(DQuat::from_rotation_arc(DVec3::Y,prop.position.normalize()).as_quat())
                .with_scale(prop.half.as_vec3()));
        }
    }}
    BuiltCell{near:near.mesh(),far:far.mesh(),props:props.mesh(),anchor}
}
fn setup(mut c:Commands,mut materials:ResMut<Assets<StandardMaterial>>) {
    if !environment::real(){return;}
    c.insert_resource(Vegetation {
        cells:HashMap::new(),jobs:HashMap::new(),radius:0.,
        source:std::sync::Arc::new(Models{near:(0..3).map(|s|model(s,false)).collect(),
            far:(0..3).map(|s|model(s,true)).collect(),rock:Sphere::new(1.).mesh().ico(1).unwrap(),
            building:Cuboid::new(2.,2.,2.).into()}),
        material:materials.add(StandardMaterial{perceptual_roughness:0.95,..default()}),
        prop_material:materials.add(StandardMaterial{base_color:Color::srgb(0.35,0.34,0.31),perceptual_roughness:0.93,..default()}),
    });
}
#[derive(Component)]struct Canopy {near:Handle<Mesh>,far:Handle<Mesh>,distant:bool,anchor:DVec3}
fn update(mut c:Commands,f:Res<Flight>,graphics:Res<environment::graphics::Graphics>,
    state:Option<ResMut<Vegetation>>,mut meshes:ResMut<Assets<Mesh>>,
    mut canopy:Query<(&mut Canopy,&mut Mesh3d)>) {
    use bevy::tasks::{block_on,poll_once,AsyncComputeTaskPool};
    let Some(mut state)=state else{return;};
    let cell=((f.position.x/CELL as f64).floor()as i32,(f.position.z/CELL as f64).floor()as i32);
    let radius=if f.agl()>3500.||f.vehicle==crate::physics::Vehicle::Spacecraft{0.}
        else{graphics.0.vegetation_distance.min(2400.+f.agl().max(0.)*0.2)};
    state.radius=radius;
    let n=(radius/CELL as f64).ceil()as i32;
    let mut wanted=HashSet::new();
    for z in -n..=n {for x in -n..=n {if radius>0.&&((x*x+z*z)as f64).sqrt()*CELL as f64<=radius{wanted.insert((cell.0+x,cell.1+z));}}}
    let old:Vec<_>=state.cells.keys().filter(|k|!wanted.contains(k)).copied().take(4).collect();
    let retired=!old.is_empty();
    for key in old {if let Some(cell)=state.cells.remove(&key){if let Some(entity)=cell.entity{c.entity(entity).despawn();}
        meshes.remove(cell.near.id());meshes.remove(cell.far.id());meshes.remove(cell.props.id());}}
    state.jobs.retain(|key,_|wanted.contains(key));
    // No commands may target children removed by the same deferred despawn.
    if retired{return;}
    let completed=state.jobs.iter_mut().find_map(|(key,task)|block_on(poll_once(task)).map(|built|(*key,built)));
    if let Some((key,built))=completed {
        state.jobs.remove(&key);
        let has_trees=built.near.count_vertices()>0;let has_props=built.props.count_vertices()>0;
        if !has_trees&&!has_props {
            state.cells.insert(key,Cell{entity:None,near:Handle::default(),far:Handle::default(),props:Handle::default()});
        }else{
            let near=if has_trees{meshes.add(built.near)}else{Handle::default()};
            let far=if has_trees{meshes.add(built.far)}else{Handle::default()};
            let props=if has_props{meshes.add(built.props)}else{Handle::default()};
            let distant=built.anchor.distance(f.position)>850.;
            let root=c.spawn((Transform::from_translation(world::local(built.anchor,&f))
                .with_rotation(world::local_rot(DQuat::IDENTITY,&f)),Visibility::Inherited,
                WorldObject{position:built.anchor,rotation:DQuat::IDENTITY})).id();
            if has_trees {
                c.spawn((
                    Mesh3d(if distant { far.clone() } else { near.clone() }),
                    MeshMaterial3d(state.material.clone()),
                    Transform::IDENTITY,
                    Canopy {
                        near: near.clone(),
                        far: far.clone(),
                        distant,
                        anchor: built.anchor,
                    },
                    bevy::light::NotShadowCaster,
                    ChildOf(root),
                ));
                // Reuse the lightweight canopy mesh as a fixed caster at
                // both visual LODs. Camera draws never include this entity.
                c.spawn((
                    Mesh3d(far.clone()),
                    MeshMaterial3d(state.material.clone()),
                    Transform::IDENTITY,
                    bevy::camera::visibility::RenderLayers::layer(super::SHADOW_LAYER),
                    bevy::light::NotShadowReceiver,
                    ChildOf(root),
                ));
            }
            if has_props{c.spawn((Mesh3d(props.clone()),MeshMaterial3d(state.prop_material.clone()),Transform::default(),ChildOf(root)));}
            state.cells.insert(key,Cell{entity:Some(root),near,far,props});
        }
    }
    let mut missing:Vec<_>=wanted.into_iter().filter(|k|!state.cells.contains_key(k)&&!state.jobs.contains_key(k)).collect();
    missing.sort_by_key(|(x,z)|(x-cell.0).pow(2)+(z-cell.1).pow(2));
    // At most two jobs and one cell upload per frame; construction never blocks flight.
    for key in missing.into_iter().take(2usize.saturating_sub(state.jobs.len())) {
        let source=state.source.clone();state.jobs.insert(key,AsyncComputeTaskPool::get().spawn(async move{build_cell(key.0,key.1,&source)}));
    }
    for(mut tree,mut mesh)in &mut canopy {
        let distance=tree.anchor.distance(f.position);
        let distant=if tree.distant{distance>750.}else{distance>950.};
        if distant!=tree.distant {tree.distant=distant;mesh.0=if distant{tree.far.clone()}else{tree.near.clone()};}
    }
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn cell_batch_preserves_geometry_and_transforms(){
        let mesh=Mesh::from(Cuboid::new(2.,2.,2.));let mut batch=Batch::default();
        batch.append(&mesh,Transform::from_xyz(10.,20.,30.).with_scale(Vec3::splat(2.)));
        batch.append(&mesh,Transform::from_xyz(-10.,0.,0.));
        assert_eq!(batch.positions.len(),mesh.count_vertices()*2);
        assert_eq!(batch.indices.len(),mesh.indices().unwrap().len()*2);
        assert!(batch.positions[..mesh.count_vertices()].iter().all(|p|(8. ..=12.).contains(&p[0])));
        assert!(batch.normals.iter().all(|n|(Vec3::from_array(*n).length()-1.).abs()<1e-6));
        assert!(batch.indices.iter().all(|i|(*i as usize)<batch.positions.len()));
    }
}

pub(crate) fn loading_resources(f:&Flight,state:Option<&Vegetation>)->Option<crate::loading::Resources>{
    let mut result=crate::loading::Resources::default();
    if !environment::real()||f.vehicle==crate::physics::Vehicle::Spacecraft||f.agl()>3500.{return Some(result);}
    let state=state?;
    let cell=((f.position.x/CELL as f64).floor()as i32,(f.position.z/CELL as f64).floor()as i32);
    let n=(state.radius/CELL as f64).ceil()as i32;
    for z in -n..=n {for x in -n..=n {
        if ((x*x+z*z)as f64).sqrt()*CELL as f64>state.radius{continue;}
        let entry=state.cells.get(&(cell.0+x,cell.1+z))?;
        if entry.entity.is_some(){for handle in [&entry.near,&entry.far,&entry.props]{
            if *handle!=Handle::default(){result.meshes.push(handle.id());}
        }}
    }}
    result.materials.extend([state.material.id().untyped(),state.prop_material.id().untyped()]);
    Some(result)
}
