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
#[derive(Resource)]
struct Vegetation {
    cells: HashMap<(i32, i32), Entity>,
    models: Vec<Handle<Mesh>>,
    far: Vec<Handle<Mesh>>,
    material: Handle<StandardMaterial>,
    last: (i32, i32, i32),
    rock: Handle<Mesh>,
    building: Handle<Mesh>,
    prop_material: Handle<StandardMaterial>,
}
#[derive(Component)]
struct TreeVisual {
    species: usize,
}
pub struct VegetationPlugin;
impl Plugin for VegetationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, setup)
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
fn setup(
    mut c: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !environment::real() {
        return;
    }
    let rock = meshes.add(Sphere::new(1.).mesh().ico(1).unwrap());
    let building = meshes.add(Cuboid::new(2., 2., 2.));
    let prop_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.34, 0.31),
        perceptual_roughness: 0.93,
        ..default()
    });
    c.insert_resource(Vegetation {
        rock,
        building,
        prop_material,
        cells: HashMap::new(),
        models: (0..3).map(|s| meshes.add(model(s, false))).collect(),
        far: (0..3).map(|s| meshes.add(model(s, true))).collect(),
        material: materials.add(StandardMaterial {
            perceptual_roughness: 0.95,
            ..default()
        }),
        last: (i32::MAX, 0, 0),
    });
}
fn update(
    mut c: Commands,
    f: Res<Flight>,
    graphics: Res<environment::graphics::Graphics>,
    state: Option<ResMut<Vegetation>>,
    mut trees: Query<(Entity, &TreeVisual, &GlobalTransform, &mut Mesh3d)>,
) {
    let Some(mut state) = state else {
        return;
    };
    let cell = (
        (f.position.x / CELL as f64).floor() as i32,
        (f.position.z / CELL as f64).floor() as i32,
        (f.agl() / 300.) as i32,
    );
    let radius = if f.agl() > 3500. || f.vehicle == crate::physics::Vehicle::Spacecraft {
        0.
    } else {
        graphics
            .0
            .vegetation_distance
            .min(2400. + f.agl().max(0.) * 0.2)
    };
    let n = (radius / CELL as f64).ceil() as i32;
    let mut wanted = HashSet::new();
    for dz in -n..=n {
        for dx in -n..=n {
            if ((dx * dx + dz * dz) as f64).sqrt() * CELL as f64 <= radius {
                wanted.insert((cell.0 + dx, cell.1 + dz));
            }
        }
    }
    let old: Vec<_> = state
        .cells
        .keys()
        .filter(|key| !wanted.contains(key))
        .copied()
        .collect();
    let retired = !old.is_empty();
    for key in old.into_iter().take(8) {
        if let Some(e) = state.cells.remove(&key) {
            c.entity(e).despawn();
        }
    }
    // Despawn commands also remove children. Never queue LOD/shadow commands
    // for those children later in this same deferred command buffer.
    if retired {
        return;
    }
    if state.last == cell && wanted.iter().all(|k| state.cells.contains_key(k)) {
        return;
    }
    let mut missing: Vec<_> = wanted
        .into_iter()
        .filter(|k| !state.cells.contains_key(k))
        .collect();
    missing.sort_by_key(|(x, z)| (x - cell.0).pow(2) + (z - cell.1).pow(2));
    for (cx, cz) in missing.into_iter().take(4) {
        let x = cx * CELL;
        let z = cz * CELL;
        let anchor = environment::point(
            (x + CELL / 2) as f64,
            (z + CELL / 2) as f64,
            0.,
            environment::radius(),
        );
        let root = c
            .spawn((
                Transform::from_translation(world::local(anchor, &f))
                    .with_rotation(world::local_rot(DQuat::IDENTITY, &f)),
                Visibility::Inherited,
                WorldObject {
                    position: anchor,
                    rotation: DQuat::IDENTITY,
                },
            ))
            .id();
        for iz in z / 32..(z + CELL) / 32 {
            for ix in x / 32..(x + CELL) / 32 {
                let Some((tx, tz, h, size)) = environment::tree(ix, iz) else {
                    continue;
                };
                let species = environment::tree_species(ix, iz);
                let origin = environment::point(tx, tz, h, environment::radius());
                let rot = DQuat::from_rotation_arc(DVec3::Y, origin.normalize())
                    * DQuat::from_rotation_y(
                        environment::hash(ix + 7, iz, 9) * std::f64::consts::TAU,
                    );
                let distant = origin.distance(f.position) > 1100.;
                let entity = c
                    .spawn((
                        Mesh3d(if distant {
                            state.far[species].clone()
                        } else {
                            state.models[species].clone()
                        }),
                        MeshMaterial3d(state.material.clone()),
                        Transform::from_translation((origin - anchor).as_vec3())
                            .with_rotation(rot.as_quat())
                            .with_scale(Vec3::splat(size as f32)),
                        TreeVisual { species },
                        ChildOf(root),
                    ))
                    .id();
                if distant {
                    c.entity(entity).insert(bevy::light::NotShadowCaster);
                }
            }
        }
        for iz in z / 128..(z + CELL) / 128 {
            for ix in x / 128..(x + CELL) / 128 {
                let Some(prop) = environment::prop(ix, iz) else {
                    continue;
                };
                c.spawn((
                    Mesh3d(if prop.rock {
                        state.rock.clone()
                    } else {
                        state.building.clone()
                    }),
                    MeshMaterial3d(state.prop_material.clone()),
                    Transform::from_translation((prop.position - anchor).as_vec3())
                        .with_rotation(
                            DQuat::from_rotation_arc(DVec3::Y, prop.position.normalize()).as_quat(),
                        )
                        .with_scale(prop.half.as_vec3()),
                    ChildOf(root),
                ));
            }
        }
        state.cells.insert((cx, cz), root);
    }
    // LOD changes are shared-handle substitutions, not mesh reconstruction.
    let local = world::local(f.position, &f);
    for (entity, tree, global, mut mesh) in &mut trees {
        let far = global.translation().distance(local) > 1100.;
        let desired = if far {
            &state.far[tree.species]
        } else {
            &state.models[tree.species]
        };
        if mesh.0 != *desired {
            mesh.0 = desired.clone();
            if far {
                c.entity(entity).insert(bevy::light::NotShadowCaster);
            } else {
                c.entity(entity).remove::<bevy::light::NotShadowCaster>();
            }
        }
    }
    state.last = cell;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retiring_cell_does_not_queue_commands_for_despawned_children() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let mut meshes = Assets::<Mesh>::default();
        let a = meshes.add(Cuboid::default());
        let b = meshes.add(Sphere::new(1.).mesh().uv(4, 3));
        let mut materials = Assets::<StandardMaterial>::default();
        let material = materials.add(StandardMaterial::default());
        let root = app
            .world_mut()
            .spawn((Transform::default(), Visibility::Inherited))
            .id();
        let child = app
            .world_mut()
            .spawn((
                Mesh3d(a.clone()),
                TreeVisual { species: 0 },
                GlobalTransform::from_translation(Vec3::splat(100000.)),
                ChildOf(root),
            ))
            .id();
        let state = Vegetation {
            cells: HashMap::from([((1000, 1000), root)]),
            models: vec![a.clone(); 3],
            far: vec![b; 3],
            material: material.clone(),
            last: (0, 0, 0),
            rock: a.clone(),
            building: a,
            prop_material: material,
        };
        app.insert_resource(state)
            .insert_resource(Flight::approach())
            .insert_resource(environment::graphics::Graphics(Default::default()))
            .add_systems(Update, update);
        app.update();
        assert!(app.world().get_entity(root).is_err());
        assert!(app.world().get_entity(child).is_err());
        assert!(app.world().resource::<Vegetation>().cells.is_empty());
    }
}
