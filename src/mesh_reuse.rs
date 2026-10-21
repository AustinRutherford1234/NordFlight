//! Share bit-identical immutable geometry without merging animation/damage nodes.
use bevy::{mesh::Indices, prelude::*};
use std::{
    collections::{HashMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
};
fn fingerprint(mesh: &Mesh) -> u64 {
    let mut hash = DefaultHasher::new();
    mesh.primitive_topology().hash(&mut hash);
    for (attribute, values) in mesh.attributes() {
        attribute.id.hash(&mut hash);
        values.get_bytes().hash(&mut hash);
    }
    match mesh.indices() {
        Some(Indices::U16(v)) => {
            0u8.hash(&mut hash);
            v.hash(&mut hash)
        }
        Some(Indices::U32(v)) => {
            1u8.hash(&mut hash);
            v.hash(&mut hash)
        }
        None => 2u8.hash(&mut hash),
    }
    hash.finish()
}
fn identical(a: &Mesh, b: &Mesh) -> bool {
    a.primitive_topology() == b.primitive_topology()
        && a.indices() == b.indices()
        && a.attributes().count() == b.attributes().count()
        && a.attributes()
            .zip(b.attributes())
            .all(|((a, av), (b, bv))| a == b && av == bv)
}
pub fn share(
    meshes: Res<Assets<Mesh>>,
    mut parts: Query<&mut Mesh3d, With<crate::aircraft::visual::ImmutableGeometry>>,
) {
    let mut groups: HashMap<u64, Vec<Handle<Mesh>>> = HashMap::new();
    for mut part in &mut parts {
        let Some(mesh) = meshes.get(&part.0) else {
            continue;
        };
        let bucket = groups.entry(fingerprint(mesh)).or_default();
        if let Some(shared) = bucket.iter().find(|h| {
            meshes
                .get(*h)
                .is_some_and(|candidate| identical(mesh, candidate))
        }) {
            if part.0 != *shared {
                part.0 = shared.clone();
            }
        } else {
            bucket.push(part.0.clone());
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sharing_requires_identical_geometry_and_attributes() {
        let a = Mesh::from(Cuboid::new(1., 2., 3.));
        let b = a.clone();
        assert!(identical(&a, &b));
        assert_eq!(fingerprint(&a), fingerprint(&b));
        let c = Mesh::from(Cuboid::new(1., 2., 4.));
        assert!(!identical(&a, &c));
        let mut d = a.clone();
        d.insert_attribute(
            Mesh::ATTRIBUTE_COLOR,
            vec![[1., 0., 0., 1.]; a.count_vertices()],
        );
        assert!(!identical(&a, &d));
    }
}
