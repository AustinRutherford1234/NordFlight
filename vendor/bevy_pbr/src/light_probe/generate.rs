//! Like [`EnvironmentMapLight`], but filtered in realtime from a cubemap.
//!
//! An environment map needs to be processed to be able to support uses beyond a simple skybox,
//! such as reflections, and ambient light contribution.
//! This process is called filtering, and can either be done ahead of time (prefiltering), or
//! in realtime, although at a reduced quality. Prefiltering is preferred, but not always possible:
//! sometimes you only gain access to an environment map at runtime, for whatever reason.
//! Typically this is from realtime reflection probes, but can also be from other sources.
//!
//! In any case, Bevy supports both modes of filtering.
//! This module provides realtime filtering via [`bevy_light::GeneratedEnvironmentMapLight`].
//! For prefiltered environment maps, see [`bevy_light::EnvironmentMapLight`].
//! These components are intended to be added to a camera.
use bevy_app::{App, Plugin, Update};
use bevy_asset::{AssetServer, Assets, RenderAssetUsages, embedded_asset, load_embedded_asset};
use bevy_camera::Camera3d;
use bevy_core_pipeline::core_3d::graph::{Core3d, Node3d};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{Has, QueryState, With, Without},
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Local, Query, Res, ResMut, lifetimeless::Read},
    world::{FromWorld, World},
};
use bevy_image::Image;
use bevy_math::{Quat, UVec2, Vec2};
use bevy_render::{
    Extract, ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems,
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    diagnostic::RecordDiagnostics,
    render_asset::RenderAssets,
    render_graph::{Node, NodeRunError, RenderGraphContext, RenderGraphExt, RenderLabel},
    render_resource::{
        AddressMode, BindGroup, BindGroupEntries, BindGroupLayoutDescriptor,
        BindGroupLayoutEntries, BindGroupLayoutId, CachedComputePipelineId, ComputePassDescriptor,
        ComputePipelineDescriptor, ComputePipelineId, DownlevelFlags, Extent3d, FilterMode,
        PipelineCache, Sampler, SamplerBindingType, SamplerDescriptor, SamplerId, ShaderStages,
        ShaderType, StorageTextureAccess, Texture, TextureAspect, TextureDescriptor,
        TextureDimension, TextureFormat, TextureFormatFeatureFlags, TextureId, TextureSampleType,
        TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension, TextureViewId,
        UniformBuffer, binding_types::*,
    },
    renderer::{RenderAdapter, RenderContext, RenderDevice, RenderQueue},
    settings::WgpuFeatures,
    sync_component::SyncComponentPlugin,
    sync_world::RenderEntity,
    texture::{CachedTexture, GpuImage, TextureCache},
};

// Implementation: generate diffuse and specular cubemaps required by PBR
// from a given high-res cubemap by
//
// 1. Copying the base mip (level 0) of the source cubemap into an intermediate
//    storage texture.
// 2. Generating mipmaps using [single-pass down-sampling] (SPD).
// 3. Convolving the mip chain twice:
//    * a [Lambertian convolution] for the 32 × 32 diffuse cubemap
//    * a [GGX convolution], once per mip level, for the specular cubemap.
//
// [single-pass down-sampling]: https://gpuopen.com/fidelityfx-spd/
// [Lambertian convolution]: https://bruop.github.io/ibl/#:~:text=Lambertian%20Diffuse%20Component
// [GGX convolution]: https://gpuopen.com/download/Bounded_VNDF_Sampling_for_Smith-GGX_Reflections.pdf

use bevy_light::{EnvironmentMapLight, GeneratedEnvironmentMapLight};
use bevy_shader::ShaderDefVal;
use core::cmp::min;
use tracing::info;

use crate::Bluenoise;
use bevy_platform::{collections::HashMap, sync::Mutex};
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};

static FILTER_HITS: AtomicU64 = AtomicU64::new(0);
static FILTER_MISSES: AtomicU64 = AtomicU64::new(0);
static SOURCE_WRITES: AtomicU64 = AtomicU64::new(0);
static FILTER_BYPASSES: AtomicU64 = AtomicU64::new(0);
static SLICE_CAPTURES: AtomicU64 = AtomicU64::new(0);
static SLICE_PUBLISHES: AtomicU64 = AtomicU64::new(0);
static SLICE_CANCELS: AtomicU64 = AtomicU64::new(0);
static SLICE_MAX_AGE: AtomicU64 = AtomicU64::new(0);
pub fn environment_slice_counts() -> [u64; 4] {
    [&SLICE_CAPTURES, &SLICE_PUBLISHES, &SLICE_CANCELS, &SLICE_MAX_AGE]
        .map(|v| v.load(Relaxed))
}

/// Optional camera contract for coherent, sliced atmosphere lighting. The owner
/// versions all sky inputs; discontinuities require a complete fresh update.
#[derive(Component, Clone, Copy, Debug, Default, ExtractComponent)]
pub struct EnvironmentMapScene {
    pub generation: u64,
    pub source: u64,
    pub enabled: bool,
}
/// Opt-in diagnostic counts: hits, misses, actual source writes, and bypasses.
pub fn environment_filter_cache_counts() -> [u64; 4] {
    [
        &FILTER_HITS,
        &FILTER_MISSES,
        &SOURCE_WRITES,
        &FILTER_BYPASSES,
    ]
    .map(|v| v.load(Relaxed))
}
/// Reset only the opt-in counters; cached results and GPU work are unaffected.
pub fn reset_environment_filter_cache_counts() {
    for value in [
        &FILTER_HITS,
        &FILTER_MISSES,
        &SOURCE_WRITES,
        &FILTER_BYPASSES,
        &SLICE_CAPTURES, &SLICE_PUBLISHES, &SLICE_CANCELS, &SLICE_MAX_AGE,
    ] {
        value.store(0, Relaxed);
    }
}

#[derive(Resource)]
#[doc(hidden)]
pub struct TrackedEnvironmentWrites {
    revisions: Mutex<HashMap<TextureId, u64>>,
    epochs: Mutex<HashMap<TextureId, u64>>,
    single_view: AtomicBool,
    enabled: bool,
    measure: bool,
}
impl Default for TrackedEnvironmentWrites {
    fn default() -> Self {
        Self {
            revisions: Mutex::new(HashMap::default()),
            epochs: Mutex::new(HashMap::default()),
            single_view: AtomicBool::new(false),
            enabled: std::env::var_os("BEVY_DISABLE_ENV_CONTENT_CACHE").is_none(),
            measure: std::env::args().any(|a| a == "--benchmark-render")
                || std::env::var_os("NORD_PROFILE").is_some(),
        }
    }
}
impl TrackedEnvironmentWrites {
    pub(crate) fn invalidate(&self, texture: TextureId) {
        let mut epochs=self.epochs.lock().expect("environment resource epochs");
        let epoch=epochs.entry(texture).or_default();
        *epoch=epoch.wrapping_add(1);
    }
    fn epoch(&self, texture: TextureId) -> u64 {
        self.epochs.lock().expect("environment resource epochs").get(&texture).copied().unwrap_or(0)
    }
    pub(crate) fn written(&self, texture: TextureId) {
        let mut revisions = self.revisions.lock().expect("environment source revisions");
        let revision = revisions.entry(texture).or_default();
        *revision = revision.wrapping_add(1);
        if self.measure {
            SOURCE_WRITES.fetch_add(1, Relaxed);
        }
    }
    fn revision(&self, texture: TextureId) -> u64 {
        self.revisions
            .lock()
            .expect("environment source revisions")
            .get(&texture)
            .copied()
            .unwrap_or(0)
    }
}

/// Labels for the environment map generation nodes
#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash, RenderLabel)]
pub enum GeneratorNode {
    Downsampling,
    Filtering,
}

/// Stores the bind group layouts for the environment map generation pipelines
#[derive(Resource)]
pub struct GeneratorBindGroupLayouts {
    pub downsampling_first: BindGroupLayoutDescriptor,
    pub downsampling_second: BindGroupLayoutDescriptor,
    pub radiance: BindGroupLayoutDescriptor,
    pub irradiance: BindGroupLayoutDescriptor,
    pub copy: BindGroupLayoutDescriptor,
}

/// Samplers for the environment map generation pipelines
#[derive(Resource)]
pub struct GeneratorSamplers {
    pub linear: Sampler,
}

/// Pipelines for the environment map generation pipelines
#[derive(Resource)]
pub struct GeneratorPipelines {
    pub downsample_first: CachedComputePipelineId,
    pub downsample_second: CachedComputePipelineId,
    pub copy: CachedComputePipelineId,
    pub radiance: CachedComputePipelineId,
    pub irradiance: CachedComputePipelineId,
}

/// Configuration for downsampling strategy based on device limits
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct DownsamplingConfig {
    // can bind ≥12 storage textures and use read-write storage textures
    pub combine_bind_group: bool,
}

pub struct EnvironmentMapGenerationPlugin;

impl Plugin for EnvironmentMapGenerationPlugin {
    fn build(&self, _: &mut App) {}
    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            let adapter = render_app.world().resource::<RenderAdapter>();
            let device = render_app.world().resource::<RenderDevice>();

            // Cubemap SPD requires at least 6 storage textures
            let limit_support = device.limits().max_storage_textures_per_shader_stage >= 6
                && device.limits().max_compute_workgroup_storage_size != 0
                && device.limits().max_compute_workgroup_size_x != 0;

            let downlevel_support = adapter
                .get_downlevel_capabilities()
                .flags
                .contains(DownlevelFlags::COMPUTE_SHADERS);

            if !limit_support || !downlevel_support {
                info!(
                    "Disabling EnvironmentMapGenerationPlugin because compute is not supported on this platform. This is safe to ignore if you are not using EnvironmentMapGenerationPlugin."
                );
                return;
            }
        } else {
            return;
        }

        embedded_asset!(app, "environment_filter.wgsl");
        embedded_asset!(app, "downsample.wgsl");
        embedded_asset!(app, "copy.wgsl");

        app.add_plugins(SyncComponentPlugin::<GeneratedEnvironmentMapLight>::default())
            .add_plugins(ExtractComponentPlugin::<EnvironmentMapScene>::default())
            .add_systems(Update, generate_environment_map_light);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<TrackedEnvironmentWrites>()
            .add_render_graph_node::<DownsamplingNode>(Core3d, GeneratorNode::Downsampling)
            .add_render_graph_node::<FilteringNode>(Core3d, GeneratorNode::Filtering)
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::EndPrepasses,
                    GeneratorNode::Downsampling,
                    GeneratorNode::Filtering,
                    Node3d::StartMainPass,
                ),
            )
            .add_systems(
                ExtractSchedule,
                extract_generated_environment_map_entities.after(generate_environment_map_light),
            )
            .add_systems(
                Render,
                prepare_generated_environment_map_bind_groups
                    .in_set(RenderSystems::PrepareBindGroups),
            )
            .add_systems(
                Render,
                prepare_generated_environment_map_intermediate_textures
                    .in_set(RenderSystems::PrepareResources),
            )
            .add_systems(
                RenderStartup,
                initialize_generated_environment_map_resources,
            );
    }
}

// The number of storage textures required to combine the bind group
const REQUIRED_STORAGE_TEXTURES: u32 = 12;

/// Initializes all render-world resources used by the environment-map generator once on
/// [`bevy_render::RenderStartup`].
pub fn initialize_generated_environment_map_resources(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_adapter: Res<RenderAdapter>,
    pipeline_cache: Res<PipelineCache>,
    asset_server: Res<AssetServer>,
) {
    // Determine whether we can use a single, large bind group for all mip outputs
    let storage_texture_limit = render_device.limits().max_storage_textures_per_shader_stage;

    // Determine whether we can read and write to the same rgba16f storage texture
    let read_write_support = render_adapter
        .get_texture_format_features(TextureFormat::Rgba16Float)
        .flags
        .contains(TextureFormatFeatureFlags::STORAGE_READ_WRITE);

    // Combine the bind group and use read-write storage if it is supported
    let combine_bind_group =
        storage_texture_limit >= REQUIRED_STORAGE_TEXTURES && read_write_support;

    // Output mips are write-only
    let mips =
        texture_storage_2d_array(TextureFormat::Rgba16Float, StorageTextureAccess::WriteOnly);

    // Bind group layouts
    let (downsampling_first, downsampling_second) = if combine_bind_group {
        // One big bind group layout containing all outputs 1–12
        let downsampling = BindGroupLayoutDescriptor::new(
            "downsampling_bind_group_layout_combined",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<DownsamplingConstants>(false),
                    texture_2d_array(TextureSampleType::Float { filterable: true }),
                    mips, // 1
                    mips, // 2
                    mips, // 3
                    mips, // 4
                    mips, // 5
                    texture_storage_2d_array(
                        TextureFormat::Rgba16Float,
                        StorageTextureAccess::ReadWrite,
                    ), // 6
                    mips, // 7
                    mips, // 8
                    mips, // 9
                    mips, // 10
                    mips, // 11
                    mips, // 12
                ),
            ),
        );

        (downsampling.clone(), downsampling)
    } else {
        // Split layout: first pass outputs 1–6, second pass outputs 7–12 (input mip6 read-only)

        let downsampling_first = BindGroupLayoutDescriptor::new(
            "downsampling_first_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<DownsamplingConstants>(false),
                    // Input mip 0
                    texture_2d_array(TextureSampleType::Float { filterable: true }),
                    mips, // 1
                    mips, // 2
                    mips, // 3
                    mips, // 4
                    mips, // 5
                    mips, // 6
                ),
            ),
        );

        let downsampling_second = BindGroupLayoutDescriptor::new(
            "downsampling_second_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<DownsamplingConstants>(false),
                    // Input mip 6
                    texture_2d_array(TextureSampleType::Float { filterable: true }),
                    mips, // 7
                    mips, // 8
                    mips, // 9
                    mips, // 10
                    mips, // 11
                    mips, // 12
                ),
            ),
        );

        (downsampling_first, downsampling_second)
    };
    let radiance = BindGroupLayoutDescriptor::new(
        "radiance_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                // Source environment cubemap
                texture_2d_array(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering), // Source sampler
                // Output specular map
                texture_storage_2d_array(
                    TextureFormat::Rgba16Float,
                    StorageTextureAccess::WriteOnly,
                ),
                uniform_buffer::<FilteringConstants>(false), // Uniforms
                texture_2d_array(TextureSampleType::Float { filterable: true }), // Blue noise texture
            ),
        ),
    );

    let irradiance = BindGroupLayoutDescriptor::new(
        "irradiance_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                // Source environment cubemap
                texture_2d_array(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering), // Source sampler
                // Output irradiance map
                texture_storage_2d_array(
                    TextureFormat::Rgba16Float,
                    StorageTextureAccess::WriteOnly,
                ),
                uniform_buffer::<FilteringConstants>(false), // Uniforms
                texture_2d_array(TextureSampleType::Float { filterable: true }), // Blue noise texture
            ),
        ),
    );

    let copy = BindGroupLayoutDescriptor::new(
        "copy_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                // Source cubemap
                texture_2d_array(TextureSampleType::Float { filterable: true }),
                // Destination mip0
                texture_storage_2d_array(
                    TextureFormat::Rgba16Float,
                    StorageTextureAccess::WriteOnly,
                ),
            ),
        ),
    );

    let layouts = GeneratorBindGroupLayouts {
        downsampling_first,
        downsampling_second,
        radiance,
        irradiance,
        copy,
    };

    // Samplers
    let linear = render_device.create_sampler(&SamplerDescriptor {
        label: Some("generator_linear_sampler"),
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        address_mode_w: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        mipmap_filter: FilterMode::Linear,
        ..Default::default()
    });

    let samplers = GeneratorSamplers { linear };

    // Pipelines
    let features = render_device.features();
    let mut shader_defs = vec![];
    if features.contains(WgpuFeatures::SUBGROUP) {
        shader_defs.push(ShaderDefVal::Int("SUBGROUP_SUPPORT".into(), 1));
    }
    if combine_bind_group {
        shader_defs.push(ShaderDefVal::Int("COMBINE_BIND_GROUP".into(), 1));
    }
    #[cfg(feature = "bluenoise_texture")]
    {
        shader_defs.push(ShaderDefVal::Int("HAS_BLUE_NOISE".into(), 1));
    }

    let downsampling_shader = load_embedded_asset!(asset_server.as_ref(), "downsample.wgsl");
    let env_filter_shader = load_embedded_asset!(asset_server.as_ref(), "environment_filter.wgsl");
    let copy_shader = load_embedded_asset!(asset_server.as_ref(), "copy.wgsl");

    // First pass for base mip Levels (0-5)
    let downsample_first = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("downsampling_first_pipeline".into()),
        layout: vec![layouts.downsampling_first.clone()],
        push_constant_ranges: vec![],
        shader: downsampling_shader.clone(),
        shader_defs: {
            let mut defs = shader_defs.clone();
            if !combine_bind_group {
                defs.push(ShaderDefVal::Int("FIRST_PASS".into(), 1));
            }
            defs
        },
        entry_point: Some("downsample_first".into()),
        zero_initialize_workgroup_memory: false,
    });

    let downsample_second = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("downsampling_second_pipeline".into()),
        layout: vec![layouts.downsampling_second.clone()],
        push_constant_ranges: vec![],
        shader: downsampling_shader,
        shader_defs: {
            let mut defs = shader_defs.clone();
            if !combine_bind_group {
                defs.push(ShaderDefVal::Int("SECOND_PASS".into(), 1));
            }
            defs
        },
        entry_point: Some("downsample_second".into()),
        zero_initialize_workgroup_memory: false,
    });

    // Radiance map for specular environment maps
    let radiance = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("radiance_pipeline".into()),
        layout: vec![layouts.radiance.clone()],
        push_constant_ranges: vec![],
        shader: env_filter_shader.clone(),
        shader_defs: {
            let mut defs = shader_defs.clone();
            if parallel_irradiance() {
                defs.push(ShaderDefVal::Bool("PARALLEL_RADIANCE".into(), true));
            }
            defs
        },
        entry_point: Some("generate_radiance_map".into()),
        zero_initialize_workgroup_memory: false,
    });

    // Irradiance map for diffuse environment maps
    let irradiance = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("irradiance_pipeline".into()),
        layout: vec![layouts.irradiance.clone()],
        push_constant_ranges: vec![],
        shader: env_filter_shader,
        shader_defs: {
            let mut defs = shader_defs.clone();
            if parallel_irradiance() {
                defs.push(ShaderDefVal::Bool("PARALLEL_IRRADIANCE".into(), true));
            }
            defs
        },
        entry_point: Some("generate_irradiance_map".into()),
        zero_initialize_workgroup_memory: false,
    });

    // Copy pipeline handles format conversion and populates mip0 when formats differ
    let copy_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("copy_pipeline".into()),
        layout: vec![layouts.copy.clone()],
        push_constant_ranges: vec![],
        shader: copy_shader,
        shader_defs: vec![],
        entry_point: Some("copy".into()),
        zero_initialize_workgroup_memory: false,
    });

    let pipelines = GeneratorPipelines {
        downsample_first,
        downsample_second,
        radiance,
        irradiance,
        copy: copy_pipeline,
    };

    // Insert all resources into the render world
    commands.insert_resource(layouts);
    commands.insert_resource(samplers);
    commands.insert_resource(pipelines);
    commands.insert_resource(DownsamplingConfig { combine_bind_group });
}

pub fn extract_generated_environment_map_entities(
    query: Extract<
        Query<(
            RenderEntity,
            &GeneratedEnvironmentMapLight,
            &EnvironmentMapLight,
        )>,
    >,
    mut commands: Commands,
    render_images: Res<RenderAssets<GpuImage>>,
) {
    for (entity, filtered_env_map, env_map_light) in query.iter() {
        let Some(env_map) = render_images.get(&filtered_env_map.environment_map) else {
            continue;
        };

        let diffuse_map = render_images.get(&env_map_light.diffuse_map);
        let specular_map = render_images.get(&env_map_light.specular_map);

        // continue if the diffuse map is not found
        if diffuse_map.is_none() || specular_map.is_none() {
            continue;
        }

        let diffuse_map = diffuse_map.unwrap();
        let specular_map = specular_map.unwrap();

        let render_filtered_env_map = RenderEnvironmentMap {
            environment_map: env_map.clone(),
            diffuse_map: diffuse_map.clone(),
            specular_map: specular_map.clone(),
            intensity: filtered_env_map.intensity,
            rotation: filtered_env_map.rotation,
            affects_lightmapped_mesh_diffuse: filtered_env_map.affects_lightmapped_mesh_diffuse,
        };
        commands
            .get_entity(entity)
            .expect("Entity not synced to render world")
            .insert(render_filtered_env_map);
    }
}

// A render-world specific version of FilteredEnvironmentMapLight that uses CachedTexture
#[derive(Component, Clone)]
pub struct RenderEnvironmentMap {
    pub environment_map: GpuImage,
    pub diffuse_map: GpuImage,
    pub specular_map: GpuImage,
    pub intensity: f32,
    pub rotation: Quat,
    pub affects_lightmapped_mesh_diffuse: bool,
}

#[derive(Component)]
pub struct IntermediateTextures {
    pub environment_map: CachedTexture,
    specular: Texture,
    diffuse: Texture,
    front: [TextureId; 3],
}

/// Returns the total number of mip levels for the provided square texture size.
/// `size` must be a power of two greater than zero. For example, `size = 512` → `9`.
#[inline]
fn compute_mip_count(size: u32) -> u32 {
    debug_assert!(size.is_power_of_two());
    32 - size.leading_zeros()
}

/// Prepares textures needed for single pass downsampling
pub fn prepare_generated_environment_map_intermediate_textures(
    light_probes: Query<(Entity, &RenderEnvironmentMap, Option<&IntermediateTextures>)>,
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    mut commands: Commands,
    writes: Res<TrackedEnvironmentWrites>,
    views: Query<(), With<Camera3d>>,
) {
    // Multiple view encoders may be submitted in a different order from their
    // CPU recording. Reuse is restricted to a single 3D view; other scenes keep
    // the original dispatch behavior. IDs own no textures and are pruned here.
    writes.single_view.store(views.iter().count() == 1, Relaxed);
    writes
        .revisions
        .lock()
        .expect("environment source revisions")
        .retain(|id, _| {
            light_probes
                .iter()
                .any(|(_, map, _)| map.environment_map.texture.id() == *id)
        });
    writes.epochs.lock().expect("environment resource epochs").retain(|id,_|
        light_probes.iter().any(|(_,map,_)| map.environment_map.texture.id()==*id));
    for (entity, env_map_light, previous) in &light_probes {
        let front = [env_map_light.environment_map.texture.id(),
            env_map_light.specular_map.texture.id(), env_map_light.diffuse_map.texture.id()];
        if previous.is_some_and(|p| p.front == front) { continue; }
        let base_size = env_map_light.environment_map.size.width;
        let mip_level_count = compute_mip_count(base_size);

        let environment_map = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("intermediate_environment_map"),
                size: Extent3d {
                    width: base_size,
                    height: base_size,
                    depth_or_array_layers: 6, // Cubemap faces
                },
                mip_level_count,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::TEXTURE_BINDING
                    | TextureUsages::STORAGE_BINDING
                    | TextureUsages::COPY_DST,
                view_formats: &[],
            },
        );

        let make_back = |front: &GpuImage, label| render_device.create_texture(&TextureDescriptor {
            label: Some(label), size: front.size, mip_level_count: front.mip_level_count,
            sample_count: 1, dimension: TextureDimension::D2, format: TextureFormat::Rgba16Float,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let specular = make_back(&env_map_light.specular_map, "pending_environment_specular");
        let diffuse = make_back(&env_map_light.diffuse_map, "pending_environment_diffuse");
        commands
            .entity(entity)
            .insert(IntermediateTextures { environment_map, specular, diffuse, front });
    }
}

/// Shader constants for downsampling algorithm
#[derive(Clone, Copy, ShaderType)]
#[repr(C)]
pub struct DownsamplingConstants {
    mips: u32,
    inverse_input_size: Vec2,
    _padding: u32,
}

/// Constants for filtering
#[derive(Clone, Copy, ShaderType)]
#[repr(C)]
pub struct FilteringConstants {
    mip_level: f32,
    sample_count: u32,
    roughness: f32,
    noise_size_bits: UVec2,
    face_start: u32,
}

/// Stores bind groups for the environment map generation pipelines
#[derive(Component)]
pub struct GeneratorBindGroups {
    pub downsampling_first: BindGroup,
    pub downsampling_second: BindGroup,
    pub radiance: Vec<Vec<BindGroup>>, // seven face selections × mip levels
    pub irradiance: Vec<BindGroup>,
    pub copy: BindGroup,
}

/// Resource identities and constants used by the existing generation bindings.
/// Pixel contents are deliberately excluded: all generation dispatches still run.
#[doc(hidden)]
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct GeneratorBindingKey {
    textures: [TextureId; 5],
    targets: [TextureId; 2],
    intermediate_view: TextureViewId,
    layouts: [BindGroupLayoutId; 5],
    sampler: SamplerId,
    parameters: [u32; 3],
    combined: bool,
}
fn generation_bindings_current(
    previous: Option<&GeneratorBindingKey>,
    present: bool,
    current: &GeneratorBindingKey,
) -> bool {
    present && previous == Some(current)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FilterResultKey {
    bindings: GeneratorBindingKey,
    pipelines: [ComputePipelineId; 5],
    source_revision: u64,
}
#[derive(Component, Default)]
struct FilterResultCache(Mutex<Option<FilterResultKey>>);
// A producer may write between downsampling and filtering. Version the pixels
// actually copied into the intermediate chain, not the later source contents.
#[derive(Component, Default)]
struct DownsampledSource(Mutex<Option<u64>>);

#[derive(Default, Debug)]
struct SliceProgress {
    generation: Option<u64>,
    next_face: u32,
    initialized: bool,
    pipelines: Option<[ComputePipelineId; 5]>,
    age: u32,
    resource_epoch: u64,
}
impl SliceProgress {
    fn reset_for(&mut self, generation: u64) {
        if self.generation != Some(generation) {
            *self = Self { generation: Some(generation), ..Default::default() };
        }
    }
    fn faces(&self, stages: u32) -> (u32, u32) {
        if !self.initialized || stages == 1 { (0, 6) }
        else { (self.next_face, 6 / stages) }
    }
    fn finish(&mut self, start: u32, count: u32) -> bool {
        self.age += 1;
        self.next_face = start + count;
        if self.next_face == 6 {
            self.next_face = 0;
            self.initialized = true;
            true
        } else { false }
    }
}
#[derive(Component, Default)]
struct SlicedFilter(Mutex<SliceProgress>);

fn slice_stages() -> u32 {
    static STAGES: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    *STAGES.get_or_init(|| std::env::var("NORD_ENV_STAGES").ok()
        .and_then(|s| s.parse().ok()).filter(|n| matches!(n, 1 | 3 | 6)).unwrap_or(6))
}
// Developer reference path. Both kernels evaluate the same 1024 samples; the
// parallel kernel only changes how their floating-point sum is associated.
fn parallel_irradiance() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("NORD_ENV_SERIAL_IRRADIANCE").is_none())
}
fn filter_result_key(
    bindings: GeneratorBindingKey,
    pipelines: [Option<ComputePipelineId>; 5],
    copied_revision: Option<u64>,
) -> Option<FilterResultKey> {
    match (pipelines, copied_revision) {
        ([Some(a), Some(b), Some(c), Some(d), Some(e)], Some(source_revision)) => {
            Some(FilterResultKey {
                bindings,
                pipelines: [a, b, c, d, e],
                source_revision,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod binding_cache_tests {
    use super::*;
    #[test]
    fn slices_publish_only_complete_maps_and_cuts_cancel_pending_faces() {
        let mut p = SliceProgress::default();
        p.reset_for(1);
        assert_eq!(p.faces(6), (0,6));
        assert!(p.finish(0,6));
        for face in 0..6 {
            assert_eq!(p.faces(6), (face,1));
            assert_eq!(p.finish(face,1), face==5);
        }
        assert!(!p.finish(0,1));
        p.reset_for(2);
        assert_eq!(p.next_face,0);
        assert_eq!(p.faces(6),(0,6));
        assert!(!p.initialized);
        assert_eq!(p.pipelines,None);
    }
    #[test]
    fn three_stage_and_reference_updates_cover_every_face_without_gaps() {
        for stages in [1,3,6] {
            let mut p=SliceProgress::default();
            p.reset_for(1);
            p.finish(0,6);
            let mut covered=Vec::new();
            for step in 0..stages {
                let (start,count)=p.faces(stages);
                covered.extend(start..start+count);
                assert_eq!(p.finish(start,count),step==stages-1);
            }
            assert_eq!(covered,vec![0,1,2,3,4,5]);
        }
    }
    fn key() -> GeneratorBindingKey {
        GeneratorBindingKey {
            textures: core::array::from_fn(|_| TextureId::new()),
            targets: core::array::from_fn(|_| TextureId::new()),
            intermediate_view: TextureViewId::new(),
            layouts: core::array::from_fn(|_| BindGroupLayoutId::new()),
            sampler: SamplerId::new(),
            parameters: [512, 6, 6],
            combined: false,
        }
    }
    #[test]
    fn producer_between_copy_and_filter_invalidates_on_next_copy() {
        let writes = TrackedEnvironmentWrites::default();
        let bindings = key();
        let source = bindings.textures[0];
        let pipelines = core::array::from_fn(|_| Some(ComputePipelineId::new()));
        let copied = DownsampledSource::default();
        assert!(filter_result_key(bindings, pipelines, None).is_none());
        *copied.0.lock().unwrap() = Some(writes.revision(source));
        // Producer writes after copy: the filter still consumes revision zero.
        writes.written(source);
        let first = filter_result_key(bindings, pipelines, *copied.0.lock().unwrap());
        assert_eq!(first.unwrap().source_revision, 0);
        // Producer now stops. Copy observes its last write; filtering must run
        // once more, even though the live source revision hasn't just changed.
        *copied.0.lock().unwrap() = Some(writes.revision(source));
        let final_result = filter_result_key(bindings, pipelines, *copied.0.lock().unwrap());
        assert_ne!(first, final_result);
        assert_eq!(final_result.unwrap().source_revision, 1);
        assert_eq!(
            final_result,
            filter_result_key(bindings, pipelines, Some(1))
        );
        for index in 0..5 {
            let mut unavailable = pipelines;
            unavailable[index] = None;
            assert!(filter_result_key(bindings, unavailable, Some(1)).is_none());
        }
    }
    #[test]
    fn filter_result_requires_same_source_contents_bindings_and_pipelines() {
        let original = FilterResultKey {
            bindings: key(),
            pipelines: core::array::from_fn(|_| ComputePipelineId::new()),
            source_revision: 0,
        };
        let result = FilterResultCache::default();
        assert_ne!(*result.0.lock().unwrap(), Some(original));
        *result.0.lock().unwrap() = Some(original);
        assert_eq!(*result.0.lock().unwrap(), Some(original));
        for index in 0..5 {
            let mut changed = original;
            changed.pipelines[index] = ComputePipelineId::new();
            assert_ne!(*result.0.lock().unwrap(), Some(changed));
        }
        let mut changed = original;
        changed.source_revision += 1;
        assert_ne!(*result.0.lock().unwrap(), Some(changed));
        changed = original;
        changed.bindings = key();
        assert_ne!(*result.0.lock().unwrap(), Some(changed));
    }
    #[test]
    fn source_revisions_follow_actual_bound_texture_independently() {
        let writes = TrackedEnvironmentWrites::default();
        let old = TextureId::new();
        let replacement = TextureId::new();
        assert_eq!(writes.revision(old), 0);
        writes.written(old);
        assert_eq!(writes.revision(old), 1);
        assert_eq!(writes.revision(replacement), 0);
        writes.written(old);
        writes.written(replacement);
        assert_eq!(writes.revision(old), 2);
        assert_eq!(writes.revision(replacement), 1);
        writes
            .revisions
            .lock()
            .unwrap()
            .retain(|id, _| *id == replacement);
        assert_eq!(writes.revisions.lock().unwrap().len(), 1);
    }
    #[test]
    fn generation_cache_invalidates_every_resource_and_constant_dependency() {
        let original = key();
        assert!(generation_bindings_current(
            Some(&original),
            true,
            &original
        ));
        let mut changes = Vec::new();
        for i in 0..2 {
            let mut changed = original;
            changed.targets[i] = TextureId::new();
            changes.push(changed);
        }
        for i in 0..5 {
            let mut changed = original;
            changed.textures[i] = TextureId::new();
            changes.push(changed);
            let mut changed = original;
            changed.layouts[i] = BindGroupLayoutId::new();
            changes.push(changed);
        }
        for i in 0..3 {
            let mut changed = original;
            changed.parameters[i] += 1;
            changes.push(changed);
        }
        let mut changed = original;
        changed.intermediate_view = TextureViewId::new();
        changes.push(changed);
        let mut changed = original;
        changed.sampler = SamplerId::new();
        changes.push(changed);
        let mut changed = original;
        changed.combined = true;
        changes.push(changed);
        for changed in changes {
            assert!(!generation_bindings_current(
                Some(&original),
                true,
                &changed
            ));
        }
    }
    #[test]
    fn generation_cache_requires_both_key_and_prepared_bindings() {
        let original = key();
        assert!(!generation_bindings_current(None, false, &original));
        assert!(!generation_bindings_current(None, true, &original));
        assert!(!generation_bindings_current(
            Some(&original),
            false,
            &original
        ));
        // Reattaching a view can reuse the same textures, but missing bindings
        // still have to be reconstructed before its generation nodes can run.
        assert!(generation_bindings_current(
            Some(&original),
            true,
            &original
        ));
        assert!(!generation_bindings_current(Some(&original), true, &key()));
    }
}

/// Prepares bind groups for environment map generation pipelines
pub fn prepare_generated_environment_map_bind_groups(
    light_probes: Query<
        (
            Entity,
            &IntermediateTextures,
            &RenderEnvironmentMap,
            Option<&GeneratorBindingKey>,
            Has<GeneratorBindGroups>,
        ),
        With<RenderEnvironmentMap>,
    >,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    queue: Res<RenderQueue>,
    layouts: Res<GeneratorBindGroupLayouts>,
    samplers: Res<GeneratorSamplers>,
    render_images: Res<RenderAssets<GpuImage>>,
    bluenoise: Res<Bluenoise>,
    config: Res<DownsamplingConfig>,
    mut commands: Commands,
    mut cache_enabled: Local<Option<bool>>,
) {
    // Skip until the blue-noise texture is available to avoid panicking.
    // The system will retry next frame once the asset has loaded.
    let Some(stbn_texture) = render_images.get(&bluenoise.texture) else {
        return;
    };

    assert!(stbn_texture.size.width.is_power_of_two());
    assert!(stbn_texture.size.height.is_power_of_two());
    let noise_size_bits = UVec2::new(
        stbn_texture.size.width.trailing_zeros(),
        stbn_texture.size.height.trailing_zeros(),
    );

    let cache_enabled = *cache_enabled
        .get_or_insert_with(|| std::env::var_os("BEVY_DISABLE_ENV_BIND_GROUP_CACHE").is_none());
    for (entity, textures, env_map_light, previous, present) in &light_probes {
        // Determine mip chain based on input size
        let base_size = env_map_light.environment_map.size.width;
        let mip_count = compute_mip_count(base_size);
        let last_mip = mip_count - 1;
        let key = GeneratorBindingKey {
            textures: [
                env_map_light.environment_map.texture.id(),
                textures.environment_map.texture.id(),
                env_map_light.specular_map.texture.id(),
                env_map_light.diffuse_map.texture.id(),
                stbn_texture.texture.id(),
            ],
            targets: [textures.specular.id(), textures.diffuse.id()],
            intermediate_view: textures.environment_map.default_view.id(),
            layouts: [
                &layouts.downsampling_first,
                &layouts.downsampling_second,
                &layouts.radiance,
                &layouts.irradiance,
                &layouts.copy,
            ]
            .map(|layout| pipeline_cache.get_bind_group_layout(layout).id()),
            sampler: samplers.linear.id(),
            parameters: [base_size, noise_size_bits.x, noise_size_bits.y],
            combined: config.combine_bind_group,
        };
        if cache_enabled && generation_bindings_current(previous, present, &key) {
            continue;
        }
        let env_map_texture = env_map_light.environment_map.texture.clone();

        // Create downsampling constants
        let downsampling_constants = DownsamplingConstants {
            mips: mip_count - 1, // Number of mips we are generating (excluding mip 0)
            inverse_input_size: Vec2::new(1.0 / base_size as f32, 1.0 / base_size as f32),
            _padding: 0,
        };

        let mut downsampling_constants_buffer = UniformBuffer::from(downsampling_constants);
        downsampling_constants_buffer.write_buffer(&render_device, &queue);

        let input_env_map_first = env_map_texture.clone().create_view(&TextureViewDescriptor {
            dimension: Some(TextureViewDimension::D2Array),
            ..Default::default()
        });

        // Utility closure to get a unique storage view for a given mip level.
        let mip_storage = |level: u32| {
            if level <= last_mip {
                create_storage_view(&textures.environment_map.texture, level, &render_device)
            } else {
                // Return a fresh 1×1 placeholder view so each binding has its own sub-resource and cannot alias.
                create_placeholder_storage_view(&render_device)
            }
        };

        // Depending on device limits, build either a combined or split bind group layout
        let (downsampling_first_bind_group, downsampling_second_bind_group) =
            if config.combine_bind_group {
                // Combined layout expects destinations 1–12 in both bind groups
                let bind_group = render_device.create_bind_group(
                    "downsampling_bind_group_combined_first",
                    &pipeline_cache.get_bind_group_layout(&layouts.downsampling_first),
                    &BindGroupEntries::sequential((
                        &samplers.linear,
                        &downsampling_constants_buffer,
                        &input_env_map_first,
                        &mip_storage(1),
                        &mip_storage(2),
                        &mip_storage(3),
                        &mip_storage(4),
                        &mip_storage(5),
                        &mip_storage(6),
                        &mip_storage(7),
                        &mip_storage(8),
                        &mip_storage(9),
                        &mip_storage(10),
                        &mip_storage(11),
                        &mip_storage(12),
                    )),
                );

                (bind_group.clone(), bind_group)
            } else {
                // Split path requires a separate view for mip6 input
                let input_env_map_second =
                    textures
                        .environment_map
                        .texture
                        .create_view(&TextureViewDescriptor {
                            dimension: Some(TextureViewDimension::D2Array),
                            base_mip_level: min(6, last_mip),
                            mip_level_count: Some(1),
                            ..Default::default()
                        });

                // Split layout (current behavior)
                let first = render_device.create_bind_group(
                    "downsampling_first_bind_group",
                    &pipeline_cache.get_bind_group_layout(&layouts.downsampling_first),
                    &BindGroupEntries::sequential((
                        &samplers.linear,
                        &downsampling_constants_buffer,
                        &input_env_map_first,
                        &mip_storage(1),
                        &mip_storage(2),
                        &mip_storage(3),
                        &mip_storage(4),
                        &mip_storage(5),
                        &mip_storage(6),
                    )),
                );

                let second = render_device.create_bind_group(
                    "downsampling_second_bind_group",
                    &pipeline_cache.get_bind_group_layout(&layouts.downsampling_second),
                    &BindGroupEntries::sequential((
                        &samplers.linear,
                        &downsampling_constants_buffer,
                        &input_env_map_second,
                        &mip_storage(7),
                        &mip_storage(8),
                        &mip_storage(9),
                        &mip_storage(10),
                        &mip_storage(11),
                        &mip_storage(12),
                    )),
                );

                (first, second)
            };

        // create a 2d array view of the bluenoise texture
        let stbn_texture_view = stbn_texture
            .texture
            .clone()
            .create_view(&TextureViewDescriptor {
                dimension: Some(TextureViewDimension::D2Array),
                ..Default::default()
            });

        // Create radiance map bind groups for each mip level
        let num_mips = mip_count as usize;
        let mut radiance_bind_groups = Vec::with_capacity(7);
        for face_start in [0, 1, 2, 3, 4, 5, 0] {
        let mut face_bind_groups = Vec::with_capacity(num_mips);

        for mip in 0..num_mips {
            // Calculate roughness from 0.0 (mip 0) to 0.889 (mip 8)
            // We don't need roughness=1.0 as a mip level because it's handled by the separate diffuse irradiance map
            let roughness = mip as f32 / (num_mips - 1) as f32;
            let sample_count = 32u32 * 2u32.pow((roughness * 4.0) as u32);

            let radiance_constants = FilteringConstants {
                mip_level: mip as f32,
                sample_count,
                roughness,
                noise_size_bits,
                face_start,
            };

            let mut radiance_constants_buffer = UniformBuffer::from(radiance_constants);
            radiance_constants_buffer.write_buffer(&render_device, &queue);

            let mip_storage_view = create_storage_view(
                &textures.specular,
                mip as u32,
                &render_device,
            );
            let bind_group = render_device.create_bind_group(
                Some(format!("radiance_bind_group_mip_{mip}").as_str()),
                &pipeline_cache.get_bind_group_layout(&layouts.radiance),
                &BindGroupEntries::sequential((
                    &textures.environment_map.default_view,
                    &samplers.linear,
                    &mip_storage_view,
                    &radiance_constants_buffer,
                    &stbn_texture_view,
                )),
            );

            face_bind_groups.push(bind_group);
        }
        radiance_bind_groups.push(face_bind_groups);
        }

        // Create irradiance bind group
        let mut irradiance_bind_groups = Vec::with_capacity(7);
        for face_start in [0, 1, 2, 3, 4, 5, 0] {
        let irradiance_constants = FilteringConstants {
            mip_level: 0.0,
            // 32 phi, 32 theta = 1024 samples total
            sample_count: 1024,
            roughness: 1.0,
            noise_size_bits,
            face_start,
        };

        let mut irradiance_constants_buffer = UniformBuffer::from(irradiance_constants);
        irradiance_constants_buffer.write_buffer(&render_device, &queue);

        // create a 2d array view
        let irradiance_map =
            textures.diffuse
                .create_view(&TextureViewDescriptor {
                    dimension: Some(TextureViewDimension::D2Array),
                    ..Default::default()
                });

        let irradiance_bind_group = render_device.create_bind_group(
            "irradiance_bind_group",
            &pipeline_cache.get_bind_group_layout(&layouts.irradiance),
            &BindGroupEntries::sequential((
                &textures.environment_map.default_view,
                &samplers.linear,
                &irradiance_map,
                &irradiance_constants_buffer,
                &stbn_texture_view,
            )),
        );
        irradiance_bind_groups.push(irradiance_bind_group);
        }

        // Create copy bind group (source env map → destination mip0)
        let src_view = env_map_light
            .environment_map
            .texture
            .create_view(&TextureViewDescriptor {
                dimension: Some(TextureViewDimension::D2Array),
                ..Default::default()
            });

        let dst_view = create_storage_view(&textures.environment_map.texture, 0, &render_device);

        let copy_bind_group = render_device.create_bind_group(
            "copy_bind_group",
            &pipeline_cache.get_bind_group_layout(&layouts.copy),
            &BindGroupEntries::with_indices(((0, &src_view), (1, &dst_view))),
        );

        commands.entity(entity).insert((
            key,
            FilterResultCache::default(),
            DownsampledSource::default(),
            SlicedFilter::default(),
            GeneratorBindGroups {
                downsampling_first: downsampling_first_bind_group,
                downsampling_second: downsampling_second_bind_group,
                radiance: radiance_bind_groups,
                irradiance: irradiance_bind_groups,
                copy: copy_bind_group,
            },
        ));
    }
}

/// Helper function to create a storage texture view for a specific mip level
fn create_storage_view(texture: &Texture, mip: u32, _render_device: &RenderDevice) -> TextureView {
    texture.create_view(&TextureViewDescriptor {
        label: Some(format!("storage_view_mip_{mip}").as_str()),
        format: Some(texture.format()),
        dimension: Some(TextureViewDimension::D2Array),
        aspect: TextureAspect::All,
        base_mip_level: mip,
        mip_level_count: Some(1),
        base_array_layer: 0,
        array_layer_count: Some(texture.depth_or_array_layers()),
        usage: Some(TextureUsages::STORAGE_BINDING),
    })
}

/// To ensure compatibility in web browsers, each call returns a unique resource so that multiple missing mip
/// bindings in the same bind-group never alias.
fn create_placeholder_storage_view(render_device: &RenderDevice) -> TextureView {
    let tex = render_device.create_texture(&TextureDescriptor {
        label: Some("lightprobe_placeholder"),
        size: Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 6,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba16Float,
        usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    tex.create_view(&TextureViewDescriptor::default())
}

/// Downsampling node implementation that handles all parts of the mip chain
pub struct DownsamplingNode {
    query: QueryState<(
        Entity,
        Read<GeneratorBindGroups>,
        Read<RenderEnvironmentMap>,
        Read<GeneratorBindingKey>,
        Read<DownsampledSource>,
        Read<SlicedFilter>,
        Option<Read<EnvironmentMapScene>>,
    )>,
}

impl FromWorld for DownsamplingNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for DownsamplingNode {
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipelines = world.resource::<GeneratorPipelines>();

        let Some(downsample_first_pipeline) =
            pipeline_cache.get_compute_pipeline(pipelines.downsample_first)
        else {
            return Ok(());
        };

        let Some(downsample_second_pipeline) =
            pipeline_cache.get_compute_pipeline(pipelines.downsample_second)
        else {
            return Ok(());
        };

        let diagnostics = render_context.diagnostic_recorder();

        let writes = world.resource::<TrackedEnvironmentWrites>();
        let ready = [pipelines.copy, pipelines.downsample_first, pipelines.downsample_second,
            pipelines.radiance, pipelines.irradiance]
            .map(|id| pipeline_cache.get_compute_pipeline(id).map(|p| p.id()));
        let [Some(a), Some(b), Some(c), Some(d), Some(e)] = ready else { return Ok(()); };
        let ready = [a,b,c,d,e];
        for (_, bind_groups, env_map_light, bindings, copied, sliced, scene) in self.query.iter_manual(world) {
            if scene.is_some_and(|s| !s.enabled) { continue; }
            let mut progress = sliced.0.lock().expect("environment slice progress");
            if progress.next_face != 0 && progress.generation != Some(scene.map_or(0, |s| s.generation)) && writes.measure {
                SLICE_CANCELS.fetch_add(1, Relaxed);
            }
            progress.reset_for(scene.map_or(0, |s| s.generation));
            let resource_epoch=writes.epoch(bindings.textures[0]);
            if progress.resource_epoch != resource_epoch {
                if progress.next_face != 0 && writes.measure {SLICE_CANCELS.fetch_add(1,Relaxed);}
                progress.initialized=false;
                progress.next_face=0;
                progress.resource_epoch=resource_epoch;
            }
            if progress.pipelines != Some(ready) {
                progress.initialized = false;
                progress.next_face = 0;
                progress.pipelines = Some(ready);
            }
            // Retain one immutable downsampled source for the complete cycle.
            if progress.next_face != 0 { continue; }
            let revision = writes.revision(bindings.textures[0]);
            if progress.initialized && *copied.0.lock().expect("downsampled environment source") == Some(revision) {
                continue;
            }
            progress.age = 0;
            if writes.measure { SLICE_CAPTURES.fetch_add(1, Relaxed); }
            // Copy base mip using compute shader with pre-built bind group
            let Some(copy_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.copy) else {
                return Ok(());
            };

            {
                let mut compute_pass =
                    render_context
                        .command_encoder()
                        .begin_compute_pass(&ComputePassDescriptor {
                            label: Some("lightprobe_copy"),
                            timestamp_writes: None,
                        });

                let pass_span = diagnostics.pass_span(&mut compute_pass, "lightprobe_copy");

                compute_pass.set_pipeline(copy_pipeline);
                compute_pass.set_bind_group(0, &bind_groups.copy, &[]);

                let tex_size = env_map_light.environment_map.size;
                let wg_x = tex_size.width.div_ceil(8);
                let wg_y = tex_size.height.div_ceil(8);
                compute_pass.dispatch_workgroups(wg_x, wg_y, 6);

                pass_span.end(&mut compute_pass);
            }

            // First pass - process mips 0-5
            {
                let mut compute_pass =
                    render_context
                        .command_encoder()
                        .begin_compute_pass(&ComputePassDescriptor {
                            label: Some("lightprobe_downsampling_first_pass"),
                            timestamp_writes: None,
                        });

                let pass_span =
                    diagnostics.pass_span(&mut compute_pass, "lightprobe_downsampling_first_pass");

                compute_pass.set_pipeline(downsample_first_pipeline);
                compute_pass.set_bind_group(0, &bind_groups.downsampling_first, &[]);

                let tex_size = env_map_light.environment_map.size;
                let wg_x = tex_size.width.div_ceil(64);
                let wg_y = tex_size.height.div_ceil(64);
                compute_pass.dispatch_workgroups(wg_x, wg_y, 6); // 6 faces

                pass_span.end(&mut compute_pass);
            }

            // Second pass - process mips 6-12
            {
                let mut compute_pass =
                    render_context
                        .command_encoder()
                        .begin_compute_pass(&ComputePassDescriptor {
                            label: Some("lightprobe_downsampling_second_pass"),
                            timestamp_writes: None,
                        });

                let pass_span =
                    diagnostics.pass_span(&mut compute_pass, "lightprobe_downsampling_second_pass");

                compute_pass.set_pipeline(downsample_second_pipeline);
                compute_pass.set_bind_group(0, &bind_groups.downsampling_second, &[]);

                let tex_size = env_map_light.environment_map.size;
                let wg_x = tex_size.width.div_ceil(256);
                let wg_y = tex_size.height.div_ceil(256);
                compute_pass.dispatch_workgroups(wg_x, wg_y, 6);

                pass_span.end(&mut compute_pass);
            }
            *copied.0.lock().expect("downsampled environment source") = Some(revision);
        }

        Ok(())
    }
}

/// Radiance map node for generating specular environment maps
pub struct FilteringNode {
    query: QueryState<(
        Entity,
        Read<GeneratorBindGroups>,
        Read<RenderEnvironmentMap>,
        Read<GeneratorBindingKey>,
        Read<FilterResultCache>,
        Read<DownsampledSource>,
        Option<Read<crate::atmosphere::AtmosphereEnvironmentMap>>,
        Read<IntermediateTextures>,
        Read<SlicedFilter>,
        Option<Read<EnvironmentMapScene>>,
    )>,
}

impl FromWorld for FilteringNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for FilteringNode {
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipelines = world.resource::<GeneratorPipelines>();

        let Some(radiance_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.radiance)
        else {
            return Ok(());
        };
        let Some(irradiance_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.irradiance)
        else {
            return Ok(());
        };

        let diagnostics = render_context.diagnostic_recorder();

        let writes = world.resource::<TrackedEnvironmentWrites>();
        let images = world.resource::<RenderAssets<GpuImage>>();
        for (_, bind_groups, env_map_light, bindings, result, copied, owner, textures, sliced, scene) in
            self.query.iter_manual(world)
        {
            if scene.is_some_and(|s| !s.enabled) { continue; }
            let owned = owner
                .and_then(|owner| images.get(&owner.environment_map))
                .is_some_and(|image| {
                    image.texture.id() == env_map_light.environment_map.texture.id()
                        && bindings.textures[0] == image.texture.id()
                        && bindings.textures[2] == env_map_light.specular_map.texture.id()
                        && bindings.textures[3] == env_map_light.diffuse_map.texture.id()
                });
            let ready = [
                pipelines.copy,
                pipelines.downsample_first,
                pipelines.downsample_second,
                pipelines.radiance,
                pipelines.irradiance,
            ]
            .map(|id| pipeline_cache.get_compute_pipeline(id).map(|p| p.id()));
            let reusable = writes.enabled && writes.single_view.load(Relaxed) && owned;
            let mut progress = sliced.0.lock().expect("environment slice progress");
            if progress.generation != Some(scene.map_or(0, |s| s.generation)) {
                // Downsampling has not captured the current scene yet.
                continue;
            }
            if ready.iter().any(Option::is_none) { continue; }
            let key = if reusable {
                filter_result_key(
                    *bindings,
                    ready,
                    *copied.0.lock().expect("downsampled environment source"),
                )
            } else {
                None
            };
            if progress.initialized && progress.next_face == 0 && key.is_some()
                && *result.0.lock().expect("environment filter result") == key {
                if writes.measure {
                    FILTER_HITS.fetch_add(1, Relaxed);
                }
                continue;
            }
            if writes.measure {
                if key.is_some() {
                    FILTER_MISSES.fetch_add(1, Relaxed);
                } else {
                    FILTER_BYPASSES.fetch_add(1, Relaxed);
                }
            }
            let stages = if reusable && scene.is_some() { slice_stages() } else { 1 };
            let (face_start, face_count) = progress.faces(stages);
            let selection = if face_count == 6 { 6 } else { face_start as usize };
            let mut compute_pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("lightprobe_radiance_map"),
                        timestamp_writes: None,
                    });

            let pass_span = diagnostics.pass_span(&mut compute_pass, "lightprobe_radiance_map");

            compute_pass.set_pipeline(radiance_pipeline);

            let base_size = env_map_light.specular_map.size.width;

            // Radiance convolution pass
            // Process each mip at different roughness levels
            for (mip, bind_group) in bind_groups.radiance[selection].iter().enumerate() {
                compute_pass.set_bind_group(0, bind_group, &[]);

                // Calculate dispatch size based on mip level
                let mip_size = base_size >> mip;
                let workgroup_count = mip_size.div_ceil(8);
                // Parallel kernel: eight texels per group, eight sample lanes
                // per texel. Reference kernel: one lane per texel in an 8x8 tile.
                let rows=if parallel_irradiance() {mip_size} else {workgroup_count};
                compute_pass.dispatch_workgroups(workgroup_count, rows, face_count);
            }
            pass_span.end(&mut compute_pass);
            // End the compute pass before starting the next one
            drop(compute_pass);

            // Irradiance convolution pass
            // Generate the diffuse environment map
            {
                let mut compute_pass =
                    render_context
                        .command_encoder()
                        .begin_compute_pass(&ComputePassDescriptor {
                            label: Some("lightprobe_irradiance_map"),
                            timestamp_writes: None,
                        });

                let irr_span =
                    diagnostics.pass_span(&mut compute_pass, "lightprobe_irradiance_map");

                compute_pass.set_pipeline(irradiance_pipeline);
                compute_pass.set_bind_group(0, &bind_groups.irradiance[selection], &[]);

                // 32×32 texture processed with 8×8 workgroups for all 6 faces
                let groups = if parallel_irradiance() {32} else {4};
                compute_pass.dispatch_workgroups(groups, groups, face_count);

                irr_span.end(&mut compute_pass);
            }
            if progress.finish(face_start, face_count) {
                if writes.measure {
                    SLICE_PUBLISHES.fetch_add(1, Relaxed);
                    SLICE_MAX_AGE.fetch_max(progress.age as u64, Relaxed);
                }
                // Publish both complete maps in the same command stream, before
                // the main pass. The renderer never samples half-updated faces.
                for (back, front, mips) in [
                    (&textures.specular, &env_map_light.specular_map, bind_groups.radiance[selection].len() as u32),
                    (&textures.diffuse, &env_map_light.diffuse_map, 1),
                ] {
                    for mip in 0..mips {
                        render_context.command_encoder().copy_texture_to_texture(
                            bevy_render::render_resource::TexelCopyTextureInfo {
                                texture: back, mip_level: mip, origin: Default::default(),
                                aspect: TextureAspect::All,
                            },
                            bevy_render::render_resource::TexelCopyTextureInfo {
                                texture: &front.texture, mip_level: mip, origin: Default::default(),
                                aspect: TextureAspect::All,
                            },
                            Extent3d { width: (front.size.width >> mip).max(1),
                                height: (front.size.height >> mip).max(1), depth_or_array_layers: 6 },
                        );
                    }
                }
                *result.0.lock().expect("environment filter result") = key;
            }
        }

        Ok(())
    }
}

/// System that generates an `EnvironmentMapLight` component based on the `GeneratedEnvironmentMapLight` component
pub fn generate_environment_map_light(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    query: Query<(Entity, &GeneratedEnvironmentMapLight), Without<EnvironmentMapLight>>,
) {
    for (entity, filtered_env_map) in &query {
        // Validate and fetch the source cubemap so we can size our targets correctly
        let Some(src_image) = images.get(&filtered_env_map.environment_map) else {
            // Texture not ready yet – try again next frame
            continue;
        };

        let base_size = src_image.texture_descriptor.size.width;

        // Sanity checks – square, power-of-two, ≤ 8192
        if src_image.texture_descriptor.size.height != base_size
            || !base_size.is_power_of_two()
            || base_size > 8192
        {
            panic!(
                "GeneratedEnvironmentMapLight source cubemap must be square power-of-two ≤ 8192, got {}×{}",
                base_size, src_image.texture_descriptor.size.height
            );
        }

        let mip_count = compute_mip_count(base_size);

        // Create a placeholder for the irradiance map
        let mut diffuse = Image::new_fill(
            Extent3d {
                width: 32,
                height: 32,
                depth_or_array_layers: 6,
            },
            TextureDimension::D2,
            &[0; 8],
            TextureFormat::Rgba16Float,
            RenderAssetUsages::all(),
        );

        diffuse.texture_descriptor.usage =
            TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING | TextureUsages::COPY_DST;

        diffuse.texture_view_descriptor = Some(TextureViewDescriptor {
            dimension: Some(TextureViewDimension::Cube),
            ..Default::default()
        });

        let diffuse_handle = images.add(diffuse);

        // Create a placeholder for the specular map. It matches the input cubemap resolution.
        let mut specular = Image::new_fill(
            Extent3d {
                width: base_size,
                height: base_size,
                depth_or_array_layers: 6,
            },
            TextureDimension::D2,
            &[0; 8],
            TextureFormat::Rgba16Float,
            RenderAssetUsages::all(),
        );

        // Set up for mipmaps
        specular.texture_descriptor.usage =
            TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING | TextureUsages::COPY_DST;
        specular.texture_descriptor.mip_level_count = mip_count;

        // When setting mip_level_count, we need to allocate appropriate data size
        // For GPU-generated mipmaps, we can set data to None since the GPU will generate the data
        specular.data = None;

        specular.texture_view_descriptor = Some(TextureViewDescriptor {
            dimension: Some(TextureViewDimension::Cube),
            mip_level_count: Some(mip_count),
            ..Default::default()
        });

        let specular_handle = images.add(specular);

        // Add the EnvironmentMapLight component with the placeholder handles
        commands.entity(entity).insert(EnvironmentMapLight {
            diffuse_map: diffuse_handle,
            specular_map: specular_handle,
            intensity: filtered_env_map.intensity,
            rotation: filtered_env_map.rotation,
            affects_lightmapped_mesh_diffuse: filtered_env_map.affects_lightmapped_mesh_diffuse,
        });
    }
}
