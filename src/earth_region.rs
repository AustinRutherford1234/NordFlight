//! Higher-resolution Nordic albedo, blended into the global NASA mosaic.
#[cfg(test)]
use crate::physics::RADIUS;
use crate::{earth, world};
use bevy::{
    asset::RenderAssetUsages,
    math::{DQuat, DVec3},
    mesh::Indices,
    prelude::*,
    render::render_resource::PrimitiveTopology,
};

#[derive(Component)]
pub struct Region(pub Vec<f32>);

fn patch() -> (Mesh, Vec<f32>) {
    let (nx, ny) = (192, 192);
    let mut p = Vec::new();
    let mut normals = Vec::new();
    let mut uv = Vec::new();
    let mut edge = Vec::new();
    let mut indices = Vec::new();
    for j in 0..=ny {
        for i in 0..=nx {
            let u = i as f32 / nx as f32;
            let v = j as f32 / ny as f32;
            let point = world::geo_to_local(u as f64 * 35.0, 75.0 - v as f64 * 30.0, -60.0);
            p.push(point.as_vec3().to_array());
            normals.push(point.normalize().as_vec3().to_array());
            uv.push([u, v]);
            edge.push(earth::smoothstep(u.min(1.0 - u).min(v).min(1.0 - v) * 20.0));
        }
    }
    for j in 0..ny {
        for i in 0..nx {
            let a = (j * (nx + 1) + i) as u32;
            let b = a + (nx + 1) as u32;
            indices.extend([a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    (
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, p)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_indices(Indices::U32(indices)),
        edge,
    )
}
pub fn setup(
    c: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) {
    let image = texture(images);
    let (mesh, edge) = patch();
    c.spawn((
        earth::Surface,
        Region(edge),
        world::Globe,
        world::WorldObject {
            position: DVec3::ZERO,
            rotation: DQuat::IDENTITY,
        },
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(mats.add(StandardMaterial {
            base_color_texture: Some(image),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 1.0,
            reflectance: 0.0,
            ..default()
        })),
        Transform::default(),
        bevy::light::NotShadowCaster,
        bevy::light::NotShadowReceiver,
    ));
}

/// High-resolution albedo; used by the globe's single-surface GPU material.
pub fn texture(images: &mut Assets<Image>) -> Handle<Image> {
    let mut image = Image::from_buffer(
        include_bytes!("../assets/earth-nordic.jpg"),
        bevy::image::ImageType::Extension("jpg"),
        bevy::image::CompressedImageFormats::NONE,
        true,
        bevy::image::ImageSampler::default(),
        RenderAssetUsages::default(),
    )
    .unwrap();
    earth::readable_oceans(&mut image);
    image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::ClampToEdge,
        address_mode_v: bevy::image::ImageAddressMode::ClampToEdge,
        ..bevy::image::ImageSamplerDescriptor::linear()
    });
    images.add(image)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn regional_patch_has_outward_normals_and_transparent_seam() {
        let (mesh, alpha) = patch();
        assert_eq!(alpha.len(), 193 * 193);
        assert_eq!(alpha[0], 0.0);
        assert_eq!(alpha[96 * 193 + 96], 1.0);
        let bevy::mesh::VertexAttributeValues::Float32x3(p) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
        else {
            panic!()
        };
        let Some(Indices::U32(indices)) = mesh.indices() else {
            panic!()
        };
        for triangle in indices.chunks_exact(3).step_by(47) {
            let a = Vec3::from(p[triangle[0] as usize]);
            let b = Vec3::from(p[triangle[1] as usize]);
            let c = Vec3::from(p[triangle[2] as usize]);
            assert!((b - a).cross(c - a).dot(a) > 0.0);
            assert!((a.length() - RADIUS as f32 + 60.0).abs() < 1.0);
        }
    }
}
