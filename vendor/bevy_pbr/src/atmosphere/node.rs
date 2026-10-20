use bevy_camera::{MainPassResolutionOverride, Viewport};
use bevy_ecs::{query::QueryItem, system::lifetimeless::Read, world::World};
use bevy_math::{UVec2, Vec3Swizzles};
use bevy_platform::{collections::HashMap, sync::Mutex};
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    extract_component::DynamicUniformIndex,
    render_asset::RenderAssets,
    render_graph::{NodeRunError, RenderGraphContext, RenderLabel, ViewNode},
    render_resource::{
        ComputePass, ComputePassDescriptor, ComputePipelineId, PipelineCache, RenderPassDescriptor,
        TextureViewId,
    },
    renderer::RenderContext,
    view::{ViewTarget, ViewUniformOffset},
};

use crate::{GpuScatteringMedium, ViewLightsUniformOffset, resources::GpuAtmosphere};

use super::{
    ExtractedAtmosphere, GpuAtmosphereSettings,
    resources::{
        AtmosphereBindGroups, AtmosphereLutPipelines, AtmosphereTextures,
        AtmosphereTransformsOffset, RenderSkyPipelineId,
    },
};

#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash, RenderLabel)]
pub enum AtmosphereNode {
    RenderLuts,
    RenderSky,
    Environment,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StaticLutKey {
    // Medium textures and both destination textures identify their actual contents.
    textures: [TextureViewId; 4],
    pipelines: [ComputePipelineId; 2],
    parameters: [u32; 23],
}
fn static_parameters(a: &ExtractedAtmosphere, s: &GpuAtmosphereSettings) -> [u32; 23] {
    [
        a.bottom_radius.to_bits(),
        a.top_radius.to_bits(),
        a.ground_albedo.x.to_bits(),
        a.ground_albedo.y.to_bits(),
        a.ground_albedo.z.to_bits(),
        s.transmittance_lut_size.x,
        s.transmittance_lut_size.y,
        s.multiscattering_lut_size.x,
        s.multiscattering_lut_size.y,
        s.sky_view_lut_size.x,
        s.sky_view_lut_size.y,
        s.aerial_view_lut_size.x,
        s.aerial_view_lut_size.y,
        s.aerial_view_lut_size.z,
        s.transmittance_lut_samples,
        s.multiscattering_lut_dirs,
        s.multiscattering_lut_samples,
        s.sky_view_lut_samples,
        s.aerial_view_lut_samples,
        s.aerial_view_lut_max_distance.to_bits(),
        s.scene_units_to_m.to_bits(),
        s.sky_max_samples,
        s.rendering_method,
    ]
}
#[derive(Default)]
struct StaticLutCache(HashMap<TextureViewId, StaticLutKey>);
impl StaticLutCache {
    fn contains(&self, key: &StaticLutKey) -> bool {
        // Track each output separately: TextureCache can reassign a texture to
        // another view, including a different pairing of the two destinations.
        self.0.get(&key.textures[2]) == Some(key) && self.0.get(&key.textures[3]) == Some(key)
    }
    fn record(&mut self, key: StaticLutKey) {
        // IDs do not own textures. Bound stale entries when views are recreated.
        if self.0.len() > 60 {
            self.0.clear();
        }
        self.0.insert(key.textures[2], key);
        self.0.insert(key.textures[3], key);
    }
}
#[cfg(test)]
mod static_lut_tests {
    use super::*;
    fn key() -> StaticLutKey {
        StaticLutKey {
            textures: core::array::from_fn(|_| TextureViewId::new()),
            pipelines: core::array::from_fn(|_| ComputePipelineId::new()),
            parameters: [0; 23],
        }
    }
    #[test]
    fn static_luts_reuse_only_complete_identical_input_and_destination_sets() {
        let mut cache = StaticLutCache::default();
        let original = key();
        assert!(!cache.contains(&original));
        cache.record(original);
        assert!(cache.contains(&original));
        for i in 0..23 {
            let mut changed = original;
            changed.parameters[i] ^= 1;
            assert!(!cache.contains(&changed));
        }
        for i in 0..4 {
            let mut changed = original;
            changed.textures[i] = TextureViewId::new();
            assert!(!cache.contains(&changed));
        }
        for i in 0..2 {
            let mut changed = original;
            changed.pipelines[i] = ComputePipelineId::new();
            assert!(!cache.contains(&changed));
        }
    }
    #[test]
    fn static_luts_detect_outputs_reassigned_between_views_and_bound_stale_ids() {
        let mut cache = StaticLutCache::default();
        let a = key();
        cache.record(a);
        let mut b = key();
        b.textures[2] = a.textures[2];
        cache.record(b);
        assert!(!cache.contains(&a));
        assert!(cache.contains(&b));
        cache.record(a);
        assert!(cache.contains(&a));
        assert!(!cache.contains(&b));
        let mut c = key();
        c.textures[3] = a.textures[3];
        cache.record(c);
        assert!(!cache.contains(&a));
        for _ in 0..1000 {
            cache.record(key());
            assert!(cache.0.len() <= 62);
        }
    }
}
pub(super) struct AtmosphereLutsNode {
    cache: Mutex<StaticLutCache>,
    cache_enabled: bool,
}
impl Default for AtmosphereLutsNode {
    fn default() -> Self {
        Self {
            cache: Mutex::new(StaticLutCache::default()),
            cache_enabled: std::env::var_os("BEVY_DISABLE_STATIC_ATMOSPHERE_CACHE").is_none(),
        }
    }
}

impl ViewNode for AtmosphereLutsNode {
    type ViewQuery = (
        Read<GpuAtmosphereSettings>,
        Read<AtmosphereBindGroups>,
        Read<DynamicUniformIndex<GpuAtmosphere>>,
        Read<DynamicUniformIndex<GpuAtmosphereSettings>>,
        Read<AtmosphereTransformsOffset>,
        Read<ViewUniformOffset>,
        Read<ViewLightsUniformOffset>,
        Read<ExtractedAtmosphere>,
        Read<AtmosphereTextures>,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (
            settings,
            bind_groups,
            atmosphere_uniforms_offset,
            settings_uniforms_offset,
            atmosphere_transforms_offset,
            view_uniforms_offset,
            lights_uniforms_offset,
            atmosphere,
            textures,
        ): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipelines = world.resource::<AtmosphereLutPipelines>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let (
            Some(transmittance_lut_pipeline),
            Some(multiscattering_lut_pipeline),
            Some(sky_view_lut_pipeline),
            Some(aerial_view_lut_pipeline),
        ) = (
            pipeline_cache.get_compute_pipeline(pipelines.transmittance_lut),
            pipeline_cache.get_compute_pipeline(pipelines.multiscattering_lut),
            pipeline_cache.get_compute_pipeline(pipelines.sky_view_lut),
            pipeline_cache.get_compute_pipeline(pipelines.aerial_view_lut),
        )
        else {
            return Ok(());
        };

        // These two integrals depend on the medium/model/settings, not the
        // camera or sun. Sky and aerial LUTs still dispatch every frame below.
        let key = world
            .resource::<RenderAssets<GpuScatteringMedium>>()
            .get(atmosphere.medium)
            .map(|medium| StaticLutKey {
                textures: [
                    medium.density_lut_view.id(),
                    medium.scattering_lut_view.id(),
                    textures.transmittance_lut.default_view.id(),
                    textures.multiscattering_lut.default_view.id(),
                ],
                pipelines: [
                    transmittance_lut_pipeline.id(),
                    multiscattering_lut_pipeline.id(),
                ],
                parameters: static_parameters(atmosphere, settings),
            });
        let reuse = self.cache_enabled
            && key
                .as_ref()
                .is_some_and(|key| self.cache.lock().expect("atmosphere cache").contains(key));

        let diagnostics = render_context.diagnostic_recorder();

        let command_encoder = render_context.command_encoder();

        let mut luts_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("atmosphere_luts"),
            timestamp_writes: None,
        });
        let pass_span = diagnostics.pass_span(&mut luts_pass, "atmosphere_luts");

        fn dispatch_2d(compute_pass: &mut ComputePass, size: UVec2) {
            const WORKGROUP_SIZE: u32 = 16;
            let workgroups_x = size.x.div_ceil(WORKGROUP_SIZE);
            let workgroups_y = size.y.div_ceil(WORKGROUP_SIZE);
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        if !reuse {
            // Transmittance LUT

            luts_pass.set_pipeline(transmittance_lut_pipeline);
            luts_pass.set_bind_group(
                0,
                &bind_groups.transmittance_lut,
                &[
                    atmosphere_uniforms_offset.index(),
                    settings_uniforms_offset.index(),
                ],
            );

            dispatch_2d(&mut luts_pass, settings.transmittance_lut_size);

            // Multiscattering LUT

            luts_pass.set_pipeline(multiscattering_lut_pipeline);
            luts_pass.set_bind_group(
                0,
                &bind_groups.multiscattering_lut,
                &[
                    atmosphere_uniforms_offset.index(),
                    settings_uniforms_offset.index(),
                ],
            );

            luts_pass.dispatch_workgroups(
                settings.multiscattering_lut_size.x,
                settings.multiscattering_lut_size.y,
                1,
            );
            if self.cache_enabled
                && let Some(key) = key
            {
                self.cache.lock().expect("atmosphere cache").record(key);
            }
        }

        // Sky View LUT

        luts_pass.set_pipeline(sky_view_lut_pipeline);
        luts_pass.set_bind_group(
            0,
            &bind_groups.sky_view_lut,
            &[
                atmosphere_uniforms_offset.index(),
                settings_uniforms_offset.index(),
                atmosphere_transforms_offset.index(),
                view_uniforms_offset.offset,
                lights_uniforms_offset.offset,
            ],
        );

        dispatch_2d(&mut luts_pass, settings.sky_view_lut_size);

        // Aerial View LUT

        luts_pass.set_pipeline(aerial_view_lut_pipeline);
        luts_pass.set_bind_group(
            0,
            &bind_groups.aerial_view_lut,
            &[
                atmosphere_uniforms_offset.index(),
                settings_uniforms_offset.index(),
                view_uniforms_offset.offset,
                lights_uniforms_offset.offset,
            ],
        );

        dispatch_2d(&mut luts_pass, settings.aerial_view_lut_size.xy());

        pass_span.end(&mut luts_pass);

        Ok(())
    }
}

#[derive(Default)]
pub(super) struct RenderSkyNode;

impl ViewNode for RenderSkyNode {
    type ViewQuery = (
        Read<ExtractedCamera>,
        Read<AtmosphereBindGroups>,
        Read<ViewTarget>,
        Read<DynamicUniformIndex<GpuAtmosphere>>,
        Read<DynamicUniformIndex<GpuAtmosphereSettings>>,
        Read<AtmosphereTransformsOffset>,
        Read<ViewUniformOffset>,
        Read<ViewLightsUniformOffset>,
        Read<RenderSkyPipelineId>,
        Option<Read<MainPassResolutionOverride>>,
    );

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (
            camera,
            atmosphere_bind_groups,
            view_target,
            atmosphere_uniforms_offset,
            settings_uniforms_offset,
            atmosphere_transforms_offset,
            view_uniforms_offset,
            lights_uniforms_offset,
            render_sky_pipeline_id,
            resolution_override,
        ): QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(render_sky_pipeline) =
            pipeline_cache.get_render_pipeline(render_sky_pipeline_id.0)
        else {
            return Ok(());
        }; //TODO: warning

        let diagnostics = render_context.diagnostic_recorder();

        let mut render_sky_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("render_sky"),
            color_attachments: &[Some(view_target.get_color_attachment())],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        let pass_span = diagnostics.pass_span(&mut render_sky_pass, "render_sky");

        if let Some(viewport) =
            Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override)
        {
            render_sky_pass.set_camera_viewport(&viewport);
        }

        render_sky_pass.set_render_pipeline(render_sky_pipeline);
        render_sky_pass.set_bind_group(
            0,
            &atmosphere_bind_groups.render_sky,
            &[
                atmosphere_uniforms_offset.index(),
                settings_uniforms_offset.index(),
                atmosphere_transforms_offset.index(),
                view_uniforms_offset.offset,
                lights_uniforms_offset.offset,
            ],
        );
        render_sky_pass.draw(0..3, 0..1);

        pass_span.end(&mut render_sky_pass);

        Ok(())
    }
}
