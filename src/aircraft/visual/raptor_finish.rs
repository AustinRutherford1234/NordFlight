//! Shared, restrained coating microdetail. Generated once; no frame-time edits.
use super::*;
pub(super) struct Detail {
    normal: Handle<Image>,
    roughness: Handle<Image>,
    duct_occlusion: Handle<Image>,
}
impl Detail {
    pub fn new(images: &mut Assets<Image>) -> Self {
        use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
        let mut normal = Vec::with_capacity(512 * 512 * 4);
        let mut rough = Vec::with_capacity(512 * 512 * 4);
        for y in 0..512u32 {
            for x in 0..512u32 {
                let h = (x.wrapping_mul(374761393) ^ y.wrapping_mul(668265263))
                    .wrapping_mul(1274126177);
                let grain = ((h >> 24) as f32 / 255. - 0.5) * 3.;
                let cloud = ((x as f32 * 0.035 + (y as f32 * 0.016).sin()).sin()
                    * (y as f32 * 0.029).sin())
                    * 8.;
                normal.extend([(128. + grain) as u8, (128. + grain * 0.7) as u8, 255, 255]);
                rough.extend([255, (216. + cloud + grain) as u8, 255, 255]);
            }
        }
        let image = |data| {
            Image::new(
                Extent3d {
                    width: 512,
                    height: 512,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                data,
                TextureFormat::Rgba8Unorm,
                RenderAssetUsages::default(),
            )
        };
        let occlusion=(0..512).flat_map(|x| {
            // Authored depth-based indirect-occlusion approximation, not a
            // geometric light bake. PBR still controls direct light/coating.
            let distance=x as f32/511.*9.;
            let value=((0.10+0.90*(-1.4*distance).exp())*255.)as u8;
            [value,255,255,255]
        }).collect();
        let duct_occlusion=images.add(Image::new(Extent3d{width:512,height:1,depth_or_array_layers:1},TextureDimension::D2,occlusion,TextureFormat::Rgba8Unorm,RenderAssetUsages::default()));
        Self {
            normal: images.add(image(normal)),
            roughness: images.add(image(rough)),
            duct_occlusion,
        }
    }
    pub fn apply(
        &self,
        mats: &mut Assets<StandardMaterial>,
        source: &Handle<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        let mut m = mats.get(source).unwrap().clone();
        m.normal_map_texture = Some(self.normal.clone());
        m.perceptual_roughness = (m.perceptual_roughness / (216. / 255.)).min(1.);
        m.metallic_roughness_texture = Some(self.roughness.clone());
        mats.add(m)
    }
    pub fn duct(&self,mats:&mut Assets<StandardMaterial>,source:&Handle<StandardMaterial>)->Handle<StandardMaterial> {
        let mut material=mats.get(source).unwrap().clone();
        material.occlusion_texture=Some(self.duct_occlusion.clone());
        material.occlusion_channel=bevy::pbr::UvChannel::Uv1;
        mats.add(material)
    }
}
pub(super) fn uv(mut mesh: Mesh) -> Mesh {
    if let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    {
        let uv = p
            .iter()
            .map(|p| [(p[2] + 10.) / 20., (p[1] + p[0] * 0.23 + 2.) / 4.])
            .collect::<Vec<_>>();
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    }
    // Degenerate authoring triangles are never used as a reason to remove the
    // mesh; geometry checks cover them separately.
    let _ = mesh.generate_tangents();
    mesh
}
