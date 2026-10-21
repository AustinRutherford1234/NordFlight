use crate::{
    ExtractedAtmosphere, GpuAtmosphereSettings, GpuLights, LightMeta, ViewLightsUniformOffset,
    resources::{
        AtmosphereSampler, AtmosphereTextures, AtmosphereTransform, AtmosphereTransforms,
        AtmosphereTransformsOffset, GpuAtmosphere,
    },
};
use bevy_asset::{AssetServer, Assets, Handle, RenderAssetUsages, load_embedded_asset};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{QueryState, With, Without},
    resource::Resource,
    system::{Commands, Query, Res, ResMut, lifetimeless::Read},
    world::{FromWorld, World},
};
use bevy_image::Image;
use bevy_light::{AtmosphereEnvironmentMapLight, GeneratedEnvironmentMapLight};
use bevy_math::{Quat, UVec2};
use bevy_render::{
    extract_component::{ComponentUniforms, DynamicUniformIndex, ExtractComponent},
    render_asset::RenderAssets,
    render_graph::{Node, NodeRunError, RenderGraphContext},
    render_resource::{binding_types::*, *},
    renderer::{RenderContext, RenderDevice},
    texture::{CachedTexture, GpuImage},
    view::{ViewUniform, ViewUniformOffset, ViewUniforms},
};
use bevy_utils::default;
use tracing::warn;

// Render world representation of an environment map light for the atmosphere
#[derive(Component, ExtractComponent, Clone)]
pub struct AtmosphereEnvironmentMap {
    pub environment_map: Handle<Image>,
    pub size: UVec2,
}

#[derive(Component)]
pub struct AtmosphereProbeTextures {
    pub environment: TextureView,
    pub environment_texture: TextureId,
    pub transmittance_lut: CachedTexture,
    pub multiscattering_lut: CachedTexture,
    pub sky_view_lut: CachedTexture,
    pub aerial_view_lut: CachedTexture,
}

#[derive(Component)]
pub(crate) struct AtmosphereProbeBindGroups {
    pub environment: BindGroup,
    pub sky: BindGroup,
    key: ProbeBindingKey,
    pub output_texture: TextureId,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct ProbeBindingKey {
    buffers: [(BufferId,u64);5],
    views: [TextureViewId;7],
    layouts: [BindGroupLayoutId;2],
    samplers: [SamplerId;2],
}
pub(crate) fn canonical_probe() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(||std::env::var_os("NORD_RENDER_REFERENCE").is_none() && std::env::var_os("NORD_PROBE_REFERENCE").is_none())
}
fn probe_diagnostic(bit:u32, message:impl core::fmt::Debug) {
    static SEEN:std::sync::atomic::AtomicU32=std::sync::atomic::AtomicU32::new(0);
    static ENABLED:std::sync::OnceLock<bool>=std::sync::OnceLock::new();
    if *ENABLED.get_or_init(||std::env::var_os("NORD_PROBE_DIAGNOSTIC").is_some()) && SEEN.fetch_or(bit,std::sync::atomic::Ordering::Relaxed)&bit==0 {
        eprintln!("NORD PROBE {bit}: {message:?}");
    }
}

#[derive(Resource)]
pub struct AtmosphereProbeLayouts {
    pub environment: BindGroupLayoutDescriptor,
}

#[derive(Resource)]
pub struct AtmosphereProbePipeline {
    pub environment: CachedComputePipelineId,
}

pub fn init_atmosphere_probe_layout(mut commands: Commands) {
    let environment = BindGroupLayoutDescriptor::new(
        "environment_bind_group_layout",
        &BindGroupLayoutEntries::with_indices(
            ShaderStages::COMPUTE,
            (
                // uniforms
                (0, uniform_buffer::<GpuAtmosphere>(true)),
                (1, uniform_buffer::<GpuAtmosphereSettings>(true)),
                (2, uniform_buffer::<AtmosphereTransform>(true)),
                (3, uniform_buffer::<ViewUniform>(true)),
                (4, uniform_buffer::<GpuLights>(true)),
                // atmosphere luts and sampler
                (8, texture_2d(TextureSampleType::default())), // transmittance
                (9, texture_2d(TextureSampleType::default())), // multiscattering
                (10, texture_2d(TextureSampleType::default())), // sky view
                (11, texture_3d(TextureSampleType::default())), // aerial view
                (12, sampler(SamplerBindingType::Filtering)),
                // output 2D array texture
                (
                    13,
                    texture_storage_2d_array(
                        TextureFormat::Rgba16Float,
                        StorageTextureAccess::WriteOnly,
                    ),
                ),
            ),
        ),
    );

    commands.insert_resource(AtmosphereProbeLayouts { environment });
}

pub(super) fn prepare_atmosphere_probe_bind_groups(
    probes: Query<(Entity, &AtmosphereProbeTextures, Option<&AtmosphereProbeBindGroups>), With<AtmosphereEnvironmentMap>>,
    render_device: Res<RenderDevice>,
    layouts: Res<AtmosphereProbeLayouts>,
    atmosphere_sampler: Res<AtmosphereSampler>,
    view_uniforms: Res<ViewUniforms>,
    lights_uniforms: Res<LightMeta>,
    atmosphere_transforms: Res<AtmosphereTransforms>,
    atmosphere_uniforms: Res<ComponentUniforms<GpuAtmosphere>>,
    settings_uniforms: Res<ComponentUniforms<GpuAtmosphereSettings>>,
    pipeline_cache: Res<PipelineCache>,
    mut commands: Commands,
    media: Res<RenderAssets<crate::GpuScatteringMedium>>,
    atmospheres: Query<&ExtractedAtmosphere>,
    sky_layouts: Res<super::resources::AtmosphereBindGroupLayouts>,
    medium_sampler: Res<crate::ScatteringMediumSampler>,
) {
    // A separate menu camera can leave all atmosphere views inactive. Their
    // extracted uniforms then have no GPU bindings, even while probe textures
    // are retained. Drop stale bind groups until the complete inputs return.
    let (Some(atmosphere), Some(settings), Some(transforms), Some(views), Some(lights)) = (
        atmosphere_uniforms.binding(), settings_uniforms.binding(),
        atmosphere_transforms.uniforms().binding(), view_uniforms.uniforms.binding(),
        lights_uniforms.view_gpu_lights.binding(),
    ) else {
        for (entity, _, _) in &probes {
            commands.entity(entity).remove::<AtmosphereProbeBindGroups>();
        }
        return;
    };
    let Some(model)=atmospheres.iter().next() else {return};
    let Some(medium)=media.get(model.medium) else {return};
    for (entity, textures, previous) in &probes {
        let key=ProbeBindingKey {
            buffers:[atmosphere_uniforms.uniforms().buffer().unwrap(),settings_uniforms.uniforms().buffer().unwrap(),atmosphere_transforms.uniforms().buffer().unwrap(),view_uniforms.uniforms.buffer().unwrap(),lights_uniforms.view_gpu_lights.buffer().unwrap()].map(|b|(b.id(),b.size())),
            views:[textures.transmittance_lut.default_view.id(),textures.multiscattering_lut.default_view.id(),textures.sky_view_lut.default_view.id(),textures.aerial_view_lut.default_view.id(),textures.environment.id(),medium.density_lut_view.id(),medium.scattering_lut_view.id()],
            layouts:[pipeline_cache.get_bind_group_layout(&layouts.environment).id(),pipeline_cache.get_bind_group_layout(&sky_layouts.sky_view_lut).id()],
            samplers:[atmosphere_sampler.id(),medium_sampler.sampler().id()],
        };
        if std::env::var_os("NORD_RENDER_REFERENCE").is_none() && previous.is_some_and(|p|p.key==key) {continue;}
        let sky=render_device.create_bind_group("probe_sky_view_bind_group",&pipeline_cache.get_bind_group_layout(&sky_layouts.sky_view_lut),
            &BindGroupEntries::with_indices(((0,atmosphere.clone()),(1,settings.clone()),(2,transforms.clone()),(3,views.clone()),(4,lights.clone()),
                (5,&medium.density_lut_view),(6,&medium.scattering_lut_view),(7,medium_sampler.sampler()),(8,&textures.transmittance_lut.default_view),
                (9,&textures.multiscattering_lut.default_view),(12,&**atmosphere_sampler),(13,&textures.sky_view_lut.default_view))));
        let environment = render_device.create_bind_group(
            "environment_bind_group",
            &pipeline_cache.get_bind_group_layout(&layouts.environment),
            &BindGroupEntries::with_indices((
                // uniforms
                (0, atmosphere.clone()),
                (1, settings.clone()),
                (2, transforms.clone()),
                (3, views.clone()),
                (4, lights.clone()),
                // atmosphere luts and sampler
                (8, &textures.transmittance_lut.default_view),
                (9, &textures.multiscattering_lut.default_view),
                (10, &textures.sky_view_lut.default_view),
                (11, &textures.aerial_view_lut.default_view),
                (12, &**atmosphere_sampler),
                // output 2D array texture
                (13, &textures.environment),
            )),
        );

        commands.entity(entity).insert(AtmosphereProbeBindGroups {
            environment,
            sky,
            key,
            output_texture: textures.environment_texture,
        });
    }
}

pub(super) fn prepare_probe_textures(
    view_textures: Query<&AtmosphereTextures, With<ExtractedAtmosphere>>,
    probes: Query<(Entity, &AtmosphereEnvironmentMap, Option<&AtmosphereProbeTextures>)>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    mut commands: Commands,
) {
    for (probe, render_env_map, previous) in &probes {
        let Some(environment) = gpu_images.get(&render_env_map.environment_map) else { continue; };
        let Some(view_textures) = view_textures.iter().next() else {
            commands.entity(probe).remove::<(AtmosphereProbeTextures, AtmosphereProbeBindGroups)>();
            continue;
        };
        let sky=if canonical_probe(){&view_textures.probe_sky_view_lut}else{&view_textures.sky_view_lut};
        if previous.is_some_and(|p| p.environment_texture == environment.texture.id()
            && p.transmittance_lut.texture.id() == view_textures.transmittance_lut.texture.id()
            && p.multiscattering_lut.texture.id() == view_textures.multiscattering_lut.texture.id()
            && p.sky_view_lut.texture.id() == sky.texture.id()
            && p.aerial_view_lut.texture.id() == view_textures.aerial_view_lut.texture.id()) {
            continue;
        }
        // create a cube view
        let environment_view = environment.texture.create_view(&TextureViewDescriptor {
            dimension: Some(TextureViewDimension::D2Array),
            ..Default::default()
        });
        // Get the first view entity's textures to borrow
        {
            commands.entity(probe).insert(AtmosphereProbeTextures {
                environment: environment_view,
                environment_texture: environment.texture.id(),
                transmittance_lut: view_textures.transmittance_lut.clone(),
                multiscattering_lut: view_textures.multiscattering_lut.clone(),
                sky_view_lut: sky.clone(),
                aerial_view_lut: view_textures.aerial_view_lut.clone(),
            });
        }
    }
}

pub fn init_atmosphere_probe_pipeline(
    pipeline_cache: Res<PipelineCache>,
    layouts: Res<AtmosphereProbeLayouts>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    let environment = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("environment_pipeline".into()),
        layout: vec![layouts.environment.clone()],
        shader: load_embedded_asset!(asset_server.as_ref(), "environment.wgsl"),
        shader_defs: if canonical_probe(){vec!["NORD_CANONICAL_PROBE".into()]}else{vec![]},
        ..default()
    });
    commands.insert_resource(AtmosphereProbePipeline { environment });
}

// Ensure power-of-two dimensions to avoid edge update issues on cubemap faces
pub fn validate_environment_map_size(size: UVec2) -> UVec2 {
    let new_size = UVec2::new(
        size.x.max(1).next_power_of_two(),
        size.y.max(1).next_power_of_two(),
    );
    if new_size != size {
        warn!(
            "Non-power-of-two AtmosphereEnvironmentMapLight size {}, correcting to {new_size}",
            size
        );
    }
    new_size
}

pub fn prepare_atmosphere_probe_components(
    probes: Query<(Entity, &AtmosphereEnvironmentMapLight), (Without<AtmosphereEnvironmentMap>,)>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    for (entity, env_map_light) in &probes {
        // Create a cubemap image in the main world that we can reference
        let new_size = validate_environment_map_size(env_map_light.size);
        let mut environment_image = Image::new_fill(
            Extent3d {
                width: new_size.x,
                height: new_size.y,
                depth_or_array_layers: 6,
            },
            TextureDimension::D2,
            &[0; 8],
            TextureFormat::Rgba16Float,
            RenderAssetUsages::all(),
        );

        environment_image.texture_view_descriptor = Some(TextureViewDescriptor {
            dimension: Some(TextureViewDimension::Cube),
            ..Default::default()
        });

        environment_image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::COPY_SRC;

        // Add the image to assets to get a handle
        let environment_handle = images.add(environment_image);

        commands.entity(entity).insert(AtmosphereEnvironmentMap {
            environment_map: environment_handle.clone(),
            size: new_size,
        });

        commands
            .entity(entity)
            .insert(GeneratedEnvironmentMapLight {
                environment_map: environment_handle,
                intensity: env_map_light.intensity,
                rotation: Quat::IDENTITY,
                affects_lightmapped_mesh_diffuse: env_map_light.affects_lightmapped_mesh_diffuse,
            });
    }
}

pub(super) struct EnvironmentNode {
    sources: bevy_platform::sync::Mutex<bevy_platform::collections::HashMap<TextureId,
        (u64, u64, [ComputePipelineId; 5], [TextureId; 4], [TextureViewId; 2],crate::AtmosphereLightKey, [u32;23])>>,
    main_view_query: QueryState<(
        Read<DynamicUniformIndex<GpuAtmosphere>>,
        Read<DynamicUniformIndex<GpuAtmosphereSettings>>,
        Read<AtmosphereTransformsOffset>,
        Read<ViewUniformOffset>,
        Read<ViewLightsUniformOffset>,
        Option<Read<crate::generate::EnvironmentMapScene>>,
        Read<ExtractedAtmosphere>,
        Read<crate::AtmosphereLightKey>,
        Read<GpuAtmosphereSettings>,
    )>,
    probe_query: QueryState<(
        Entity,
        Read<AtmosphereProbeBindGroups>,
        Read<AtmosphereEnvironmentMap>,
        Read<AtmosphereProbeTextures>,
    )>,
}

impl FromWorld for EnvironmentNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            sources: Default::default(),
            main_view_query: QueryState::new(world),
            probe_query: QueryState::new(world),
        }
    }
}

impl Node for EnvironmentNode {
    fn update(&mut self, world: &mut World) {
        self.main_view_query.update_archetypes(world);
        self.probe_query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipelines = world.resource::<AtmosphereProbePipeline>();
        let view_entity = graph.view_entity();

        let Some(environment_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.environment)
        else {
            probe_diagnostic(1,pipeline_cache.get_compute_pipeline_state(pipelines.environment));
            return Ok(());
        };
        let luts = world.resource::<super::resources::AtmosphereLutPipelines>();
        let ready=[luts.transmittance_lut, luts.multiscattering_lut, luts.sky_view_lut, luts.aerial_view_lut]
            .map(|id| pipeline_cache.get_compute_pipeline(id).map(|p| p.id()));
        let [Some(a),Some(b),Some(c),Some(d)] = ready else {probe_diagnostic(2,"LUT pipelines pending");return Ok(());};

        let (Ok((
            atmosphere_uniforms_offset,
            settings_uniforms_offset,
            atmosphere_transforms_offset,
            view_uniforms_offset,
            lights_uniforms_offset,
            scene,
            atmosphere,
            light_key,
            settings,
        )),) = (self.main_view_query.get_manual(world, view_entity),)
        else {
            probe_diagnostic(4,(view_entity,"view query missing components"));
            return Ok(());
        };
        probe_diagnostic(8,(view_entity,self.probe_query.iter_manual(world).count(),scene));
        let Some(medium)=world.resource::<RenderAssets<crate::GpuScatteringMedium>>().get(atmosphere.medium)
            else {return Ok(());};

        let single_probe=self.probe_query.iter_manual(world).count()==1;
        for (probe_entity, bind_groups, env_map_light, textures) in self.probe_query.iter_manual(world) {
            if scene.is_some_and(|s| !s.enabled) { continue; }
            let mut sources = self.sources.lock().expect("atmosphere source inputs");
            let key = scene.map(|s| (s.generation, s.source, [environment_pipeline.id(),a,b,c,d],
                [textures.transmittance_lut.texture.id(), textures.multiscattering_lut.texture.id(),
                 textures.sky_view_lut.texture.id(), textures.aerial_view_lut.texture.id()],
                [medium.density_lut_view.id(),medium.scattering_lut_view.id()],*light_key, super::node::atmosphere_parameters(atmosphere,settings)));
            if key.is_some() && sources.get(&bind_groups.output_texture).copied() == key {
                continue;
            }
            // A pending filter consumes its retained snapshot. Avoid producing
            // intermediate six-face sources which would be overwritten unread.
            // Resource/model changes must still invalidate and initialize fully.
            let stable_resources=match (sources.get(&bind_groups.output_texture),key.as_ref()) {
                (Some(old),Some(new))=>old.2==new.2 && old.3==new.3 && old.4==new.4 && old.6==new.6,
                _=>false,
            };
            if canonical_probe() && single_probe && stable_resources
                && scene.is_some_and(|s|crate::generate::retain_pending_environment_source(world,
                    probe_entity,bind_groups.output_texture,*s)) {continue;}
            let mut pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("environment_pass"),
                        timestamp_writes: None,
                    });

            let probe_offset=if canonical_probe(){atmosphere_transforms_offset.probe_index}else{atmosphere_transforms_offset.index()};
            if canonical_probe() {
                let Some(sky_pipeline)=pipeline_cache.get_compute_pipeline(luts.sky_view_lut) else {continue};
                // Sizes come from the actual retained destination, not a later view.
                let size=textures.sky_view_lut.texture.size();
                pass.set_pipeline(sky_pipeline);
                pass.set_bind_group(0,&bind_groups.sky,&[atmosphere_uniforms_offset.index(),settings_uniforms_offset.index(),probe_offset,view_uniforms_offset.offset,lights_uniforms_offset.offset]);
                pass.dispatch_workgroups(size.width.div_ceil(16),size.height.div_ceil(16),1);
            }

            pass.set_pipeline(environment_pipeline);
            pass.set_bind_group(
                0,
                &bind_groups.environment,
                &[
                    atmosphere_uniforms_offset.index(),
                    settings_uniforms_offset.index(),
                    probe_offset,
                    view_uniforms_offset.offset,
                    lights_uniforms_offset.offset,
                ],
            );

            pass.dispatch_workgroups(
                env_map_light.size.x / 8,
                env_map_light.size.y / 8,
                6, // 6 cubemap faces
            );
            // Track the texture actually bound for writing, not a potentially
            // newer image handle whose replacement bindings are still pending.
            if let Some(writes) = world.get_resource::<crate::generate::TrackedEnvironmentWrites>()
            {
                if let (Some(old),Some(new))=(sources.get(&bind_groups.output_texture),key.as_ref()) {
                    if old.2!=new.2 || old.3!=new.3 || old.4!=new.4 {
                        writes.invalidate(bind_groups.output_texture);
                    }
                }
                writes.written(bind_groups.output_texture);
            }
            if let Some(key) = key {
                if sources.len() > 16 { sources.clear(); }
                sources.insert(bind_groups.output_texture, key);
            }
        }

        Ok(())
    }
}
