//! Batch only explicit rigid leaves; preserve structural IDs and attachment groups.
use bevy::{prelude::*, camera::visibility::{RenderLayers,NoFrustumCulling}, light::{NotShadowCaster,NotShadowReceiver}};
use std::collections::HashMap;
#[derive(Component)]
pub struct RigidGeometry;
#[derive(Component)]
pub struct RigidBatch;
pub fn build(world:&mut World) {
    if std::env::var_os("NORD_RENDER_REFERENCE").is_some() || std::env::var_os("NORD_BATCH_REFERENCE").is_some() {return;}
    let mut groups:HashMap<_,Vec<(Entity,Handle<Mesh>,Transform,Handle<StandardMaterial>,RenderLayers)>>=HashMap::new();
    let mut query=world.query_filtered::<(Entity,&ChildOf,&Mesh3d,&MeshMaterial3d<StandardMaterial>,&Transform,&Visibility,Option<&RenderLayers>,Has<NotShadowCaster>,Has<NotShadowReceiver>,Option<&Children>),(With<RigidGeometry>,Without<NoFrustumCulling>,Without<crate::cockpit_controls::Key>,Without<crate::cockpit_controls::Lever>,Without<crate::cockpit::PilotControl>,Without<crate::combat::bvr::cockpit_radar::Bezel>)>();
    for (entity,parent,mesh,mat,transform,visibility,layers,no_cast,no_receive,children) in query.iter(world) {
        if *visibility!=Visibility::Inherited || children.is_some_and(|c|!c.is_empty()) || transform.to_matrix().determinant()<=0. {continue;}
        if !world.resource::<Assets<StandardMaterial>>().get(&mat.0).is_some_and(|m|m.alpha_mode==AlphaMode::Opaque) {continue;}
        let layers=layers.cloned().unwrap_or_default();
        groups.entry((parent.parent(),mat.0.id(),layers.iter().collect::<Vec<_>>(),no_cast,no_receive)).or_default()
            .push((entity,mesh.0.clone(),*transform,mat.0.clone(),layers));
    }
    for ((parent,_,_,no_cast,no_receive),parts) in groups {
        if parts.len()<2 {continue;}
        let meshes=world.resource::<Assets<Mesh>>();let mut combined:Option<Mesh>=None;let mut valid=true;
        for (_,handle,transform,_,_) in &parts {
            let Some(mesh)=meshes.get(handle) else {valid=false;break};
            let mesh=mesh.clone().transformed_by(*transform);
            if let Some(combined)=&mut combined {if combined.merge(&mesh).is_err(){valid=false;break;}} else {combined=Some(mesh);}
        }
        if !valid {continue;}
        let handle=world.resource_mut::<Assets<Mesh>>().add(combined.unwrap());
        let entity=world.spawn((RigidBatch,Mesh3d(handle),MeshMaterial3d(parts[0].3.clone()),Transform::default(),parts[0].4.clone())).id();
        if no_cast {world.entity_mut(entity).insert(NotShadowCaster);}
        if no_receive {world.entity_mut(entity).insert(NotShadowReceiver);}
        world.entity_mut(parent).add_child(entity);
        for (entity,_,_,_,_) in parts {world.entity_mut(entity).remove::<Mesh3d>();}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn batching_preserves_local_geometry_structural_ids_and_attachment_groups() {
        let mut w=World::new();w.init_resource::<Assets<Mesh>>();w.init_resource::<Assets<StandardMaterial>>();
        let mat=w.resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial::default());
        let transparent=w.resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial{alpha_mode:AlphaMode::Blend,..default()});
        let mesh=w.resource_mut::<Assets<Mesh>>().add(Mesh::from(Cuboid::new(1.,2.,3.)));
        let parent=w.spawn((Transform::from_xyz(100.,20.,10.),Visibility::Hidden)).id();
        let other_parent=w.spawn_empty().id();
        let mut leaves=Vec::new();
        for x in [0.,2.] {
            let e=w.spawn((RigidGeometry,Mesh3d(mesh.clone()),MeshMaterial3d(mat.clone()),Transform::from_xyz(x,0.,0.),Visibility::Inherited)).id();
            w.entity_mut(parent).add_child(e);leaves.push(e);
        }
        let unmarked=w.spawn((Mesh3d(mesh.clone()),MeshMaterial3d(mat.clone()))).id();w.entity_mut(parent).add_child(unmarked);
        let glass=w.spawn((RigidGeometry,Mesh3d(mesh.clone()),MeshMaterial3d(transparent),Transform::default(),Visibility::Inherited)).id();w.entity_mut(parent).add_child(glass);
        let separate=w.spawn((RigidGeometry,Mesh3d(mesh.clone()),MeshMaterial3d(mat),Transform::default(),Visibility::Inherited)).id();w.entity_mut(other_parent).add_child(separate);
        build(&mut w);
        for leaf in leaves {assert!(w.get_entity(leaf).is_ok());assert!(w.get::<Mesh3d>(leaf).is_none());assert_eq!(w.get::<ChildOf>(leaf).unwrap().parent(),parent);}
        for leaf in [unmarked,glass,separate] {assert!(w.get::<Mesh3d>(leaf).is_some());}
        let batches:Vec<_>=w.query_filtered::<(Entity,&Mesh3d,&ChildOf,&Transform),With<RigidBatch>>().iter(&w).map(|(e,m,p,t)|(e,m.0.clone(),p.parent(),*t)).collect();
        assert_eq!(batches.len(),1);assert_eq!(batches[0].2,parent);assert_eq!(batches[0].3,Transform::default());
        let mesh=w.resource::<Assets<Mesh>>().get(&batches[0].1).unwrap();
        let bevy::mesh::VertexAttributeValues::Float32x3(vertices)=mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap() else {panic!("positions")};
        assert_eq!(vertices.len(),48);
        let min=vertices.iter().map(|p|p[0]).fold(f32::INFINITY,f32::min);
        let max=vertices.iter().map(|p|p[0]).fold(f32::NEG_INFINITY,f32::max);
        assert_eq!((min,max),(-0.5,2.5));
        let batch=batches[0].0;w.despawn(parent);assert!(w.get_entity(batch).is_err());
    }
}
