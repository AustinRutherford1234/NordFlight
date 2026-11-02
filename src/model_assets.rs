//! Immutable imported model textures only need their renderer copy after upload.
//! Meshes and StandardMaterials remain available to animation and lighting code.
use bevy::{asset::{AssetPath, RenderAssetUsages}, prelude::*};

pub fn load<A: Asset>(server: &AssetServer, path: AssetPath<'static>) -> Handle<A> {
    let keep_cpu=std::env::var_os("NORD_MODEL_CPU_REFERENCE").is_some();
    server.load_with_settings(path, move |settings: &mut bevy::gltf::GltfLoaderSettings| {
        // This loader flag is passed to embedded and external model Images.
        // Keep all texture pixels and resolution; release only the CPU copy.
        settings.load_materials = if keep_cpu {RenderAssetUsages::default()}else{RenderAssetUsages::RENDER_WORLD};
    })
}
