//! Full-resolution switch labels share one texture/material per panel. Padded
//! atlas cells retain the original pixels and filtering at every label edge.
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use std::collections::HashMap;
const WIDTH: u32 = 256;
const HEIGHT: u32 = 192;
const PAD: u32 = 2;
pub struct FaceAtlas {
    pub material: Handle<StandardMaterial>,
    cells: HashMap<&'static str, UVec2>,
    size: UVec2,
    meshes: HashMap<&'static str, Handle<Mesh>>,
}
impl FaceAtlas {
    pub fn new(
        labels: impl IntoIterator<Item = &'static str>,
        images: &mut Assets<Image>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Self {
        let mut cells = HashMap::new();
        let mut ordered = Vec::new();
        for label in labels {
            if !cells.contains_key(label) {
                let i = ordered.len() as u32;
                cells.insert(label, UVec2::new(i % 4, i / 4));
                ordered.push(label);
            }
        }
        assert!(!ordered.is_empty());
        let size = UVec2::new(
            4 * (WIDTH + PAD * 2),
            (ordered.len() as u32).div_ceil(4) * (HEIGHT + PAD * 2),
        );
        let mut data = vec![0; (size.x * size.y * 4) as usize];
        for label in ordered {
            let pixels = crate::combat::bvr::cockpit_radar::face_pixels(label);
            let origin = cells[label] * UVec2::new(WIDTH + PAD * 2, HEIGHT + PAD * 2);
            for y in 0..HEIGHT + PAD * 2 {
                for x in 0..WIDTH + PAD * 2 {
                    let source = ((y.saturating_sub(PAD).min(HEIGHT - 1) * WIDTH
                        + x.saturating_sub(PAD).min(WIDTH - 1))
                        * 4) as usize;
                    let dest = (((origin.y + y) * size.x + origin.x + x) * 4) as usize;
                    data[dest..dest + 4].copy_from_slice(&pixels[source..source + 4]);
                }
            }
        }
        let mut image = Image::new_fill(
            Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        image.data = Some(data);
        let texture = images.add(image);
        let material = materials.add(StandardMaterial {
            base_color_texture: Some(texture.clone()),
            emissive_texture: Some(texture),
            emissive: LinearRgba::new(0.8, 0.8, 0.8, 1.),
            unlit: true,
            ..default()
        });
        Self {
            material,
            cells,
            size,
            meshes: HashMap::new(),
        }
    }
    pub fn face(&mut self, label: &'static str, meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
        self.meshes
            .entry(label)
            .or_insert_with(|| {
                let origin = self.cells[label] * UVec2::new(WIDTH + PAD * 2, HEIGHT + PAD * 2)
                    + UVec2::splat(PAD);
                let mut mesh = Mesh::from(Rectangle::new(0.90, 0.82));
                if let Some(bevy::mesh::VertexAttributeValues::Float32x2(uvs)) =
                    mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0)
                {
                    for uv in uvs {
                        uv[0] = (origin.x as f32 + uv[0] * WIDTH as f32) / self.size.x as f32;
                        uv[1] = (origin.y as f32 + uv[1] * HEIGHT as f32) / self.size.y as f32;
                    }
                }
                meshes.add(mesh)
            })
            .clone()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atlas_preserves_every_label_pixel_and_edge_padding_and_reuses_resources() {
        let mut images = Assets::<Image>::default();
        let mut mats = Assets::<StandardMaterial>::default();
        let mut meshes = Assets::<Mesh>::default();
        let mut atlas = FaceAtlas::new(
            ["BAT", "GEAR", "TARGET", "BAT", "PWR"],
            &mut images,
            &mut mats,
        );
        assert_eq!(mats.len(), 1);
        assert_eq!(images.len(), 1);
        assert_eq!(atlas.cells.len(), 4);
        let texture = mats
            .get(&atlas.material)
            .unwrap()
            .base_color_texture
            .as_ref()
            .unwrap();
        let data = images.get(texture).unwrap().data.as_ref().unwrap();
        for (&label, &cell) in &atlas.cells {
            let source = crate::combat::bvr::cockpit_radar::face_pixels(label);
            let origin = cell * UVec2::new(WIDTH + PAD * 2, HEIGHT + PAD * 2);
            for y in 0..HEIGHT + PAD * 2 {
                for x in 0..WIDTH + PAD * 2 {
                    let a = (((origin.y + y) * atlas.size.x + origin.x + x) * 4) as usize;
                    let b = ((y.saturating_sub(PAD).min(HEIGHT - 1) * WIDTH
                        + x.saturating_sub(PAD).min(WIDTH - 1))
                        * 4) as usize;
                    assert_eq!(&data[a..a + 4], &source[b..b + 4]);
                }
            }
        }
        assert_eq!(
            atlas.face("BAT", &mut meshes),
            atlas.face("BAT", &mut meshes)
        );
        assert_eq!(meshes.len(), 1);
    }
}
