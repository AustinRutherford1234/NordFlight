//! Mipmapped material arrays on Bevy PBR; far terrain needs no detail samples.
use bevy::{
    asset::{RenderAssetUsages, load_internal_asset, uuid_handle},
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{
        AsBindGroup, Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor,
        TextureViewDimension,
    },
    shader::{Shader, ShaderRef},
};
const SHADER: Handle<Shader> = uuid_handle!("51ceba12-2816-42c5-b441-f8e697bc1438");
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct SurfaceExtension {
    #[uniform(100)]
    pub detail: Vec4,
    #[texture(101, dimension = "2d_array")]
    #[sampler(102)]
    pub albedo: Handle<Image>,
    #[texture(103, dimension = "2d_array")]
    #[sampler(104)]
    pub normal: Handle<Image>,
    #[texture(105, dimension = "2d_array")]
    #[sampler(106)]
    pub cover: Handle<Image>,
    #[texture(107, sample_type = "u_int")]
    pub cover_lookup: Handle<Image>,
    #[texture(111, dimension = "2d_array", sample_type = "u_int")]
    pub cover_classes: Handle<Image>,
    #[uniform(112)]
    pub palette: [Vec4;8],
}
impl MaterialExtension for SurfaceExtension {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }
}
pub type SurfaceMaterial = ExtendedMaterial<StandardMaterial, SurfaceExtension>;
static BC: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub struct TerrainMaterialPlugin;
impl Plugin for TerrainMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, SHADER, "surface.wgsl", Shader::from_wgsl);
        app.add_plugins(MaterialPlugin::<SurfaceMaterial>::default())
            .add_systems(PreStartup, detect);
    }
}
fn detect(device: Res<bevy::render::renderer::RenderDevice>) {
    BC.store(
        device
            .features()
            .contains(bevy::render::render_resource::WgpuFeatures::TEXTURE_COMPRESSION_BC),
        std::sync::atomic::Ordering::Relaxed,
    );
}
pub fn array(bytes: Vec<u8>, srgb: bool) -> Image {
    let bc = BC.load(std::sync::atomic::Ordering::Relaxed);
    let format = if bc {
        if srgb {
            TextureFormat::Bc3RgbaUnormSrgb
        } else {
            TextureFormat::Bc3RgbaUnorm
        }
    } else if srgb {
        TextureFormat::Rgba8UnormSrgb
    } else {
        TextureFormat::Rgba8Unorm
    };
    let mut image = Image::new_uninit(
        Extent3d {
            width: 512,
            height: 512,
            depth_or_array_layers: 6,
        },
        TextureDimension::D2,
        format,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.data = Some(bytes);
    image.texture_descriptor.mip_level_count = 10;
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::D2Array),
        ..default()
    });
    image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::Repeat,
        address_mode_v: bevy::image::ImageAddressMode::Repeat,
        mag_filter: bevy::image::ImageFilterMode::Linear,
        min_filter: bevy::image::ImageFilterMode::Linear,
        mipmap_filter: bevy::image::ImageFilterMode::Linear,
        anisotropy_clamp: 4,
        ..default()
    });
    image
}
pub fn load(images: &mut Assets<Image>) -> (Handle<Image>, Handle<Image>) {
    let root =
        std::path::PathBuf::from(crate::start_screen::asset_path()).join("environment/materials");
    let suffix = if BC.load(std::sync::atomic::Ordering::Relaxed) {
        "-bc3.bin"
    } else {
        ".bin"
    };
    let albedo =
        std::fs::read(root.join(format!("albedo{suffix}"))).expect("terrain material array");
    let normal = std::fs::read(root.join(format!("normal-roughness{suffix}")))
        .expect("terrain normal array");
    (
        images.add(array(albedo, true)),
        images.add(array(normal, false)),
    )
}
