//! Opt-in measurements. Never changes simulation time or render quality.
use bevy::render::{
    diagnostic::RecordDiagnostics,
    render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext, RenderLabel},
    renderer::RenderContext,
};
use bevy::{diagnostic::DiagnosticsStore, prelude::*};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::time::Instant;
static ADAPTER: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();

static ENABLED: AtomicBool = AtomicBool::new(false);
const NAMES: [&str; 17] = [
    "simulation",
    "aircraft_physics",
    "combat",
    "earth",
    "terrain",
    "cockpit",
    "radar_display",
    "audio",
    "camera",
    "world",
    "display_publish",
    "display_flush",
    "collision",
    "ai_pilot",
    "terrain_build",
    "orbital_coast",
    "asset_prepare_wall",
];
static NANOS: [AtomicU64; 17] = [const { AtomicU64::new(0) }; 17];
static CALLS: [AtomicU64; 17] = [const { AtomicU64::new(0) }; 17];
pub static UPLOAD_BYTES: AtomicU64 = AtomicU64::new(0);
pub static UPLOAD_CALLS: AtomicU64 = AtomicU64::new(0);
pub static MESH_CHANGES: AtomicU64 = AtomicU64::new(0);
pub static MATERIAL_CHANGES: AtomicU64 = AtomicU64::new(0);
pub static IMAGE_CHANGES: AtomicU64 = AtomicU64::new(0);
pub static EARTH_MATERIAL_CHANGES: AtomicU64 = AtomicU64::new(0);
static SIMULATED_NS: AtomicU64 = AtomicU64::new(0);
static FOREST_VIEW_VERTICES: AtomicU64 = AtomicU64::new(0);
static FOREST_SHADOW_VERTICES: AtomicU64 = AtomicU64::new(0);
static FOREST_SAMPLES: AtomicU64 = AtomicU64::new(0);
static TERRAIN_STAGE_FRAMES: AtomicU64 = AtomicU64::new(0);
static TERRAIN_STAGE_BYTES: AtomicU64 = AtomicU64::new(0);
static TERRAIN_STAGE_MESHES: AtomicU64 = AtomicU64::new(0);
static TERRAIN_STAGE_MAX_BYTES: AtomicU64 = AtomicU64::new(0);
static TERRAIN_STAGE_MAX_MESHES: AtomicU64 = AtomicU64::new(0);
static MESH_SLAB_MAX_BYTES: AtomicU64 = AtomicU64::new(0);
static MESH_SLAB_MAX_COUNT: AtomicU64 = AtomicU64::new(0);
/// Track whole-run peaks of the mesh allocator. Diagnostic averages hide the
/// high-water mark that `MeshAllocatorSettings::min_slab_size` must be sized from.
fn mesh_slab_peaks(diagnostics: Res<DiagnosticsStore>) {
    let latest = |name: &'static str| diagnostics
        .get(&bevy::diagnostic::DiagnosticPath::const_new(name))
        .and_then(|d| d.value());
    if let Some(bytes) = latest("mesh_allocator_slabs_size") {
        MESH_SLAB_MAX_BYTES.fetch_max(bytes as u64, Relaxed);
    }
    if let Some(count) = latest("mesh_allocator_slabs") {
        MESH_SLAB_MAX_COUNT.fetch_max(count as u64, Relaxed);
    }
}
/// Per-frame event bits for `NORD_SPIKE_LOG`: which streaming/anchor work ran
/// in the frame, printed next to any frame over 25 ms.
pub static FRAME_EVENTS: AtomicU64 = AtomicU64::new(0);
pub const EV_STAGE: u64 = 1;
pub const EV_PUBLISH: u64 = 2;
pub const EV_RECLAIM: u64 = 4;
pub const EV_REBASE: u64 = 8;
pub const EV_JOB: u64 = 16;
pub const EV_BUILDING: u64 = 32;
pub const EV_ENV_SOURCE: u64 = 64;
pub const EV_ENV_PUBLISH: u64 = 128;
pub const EV_PIPELINE: u64 = 256;
/// Render world: any pipeline queued or compiling, or a new pipeline added.
fn pipeline_activity(cache: Res<bevy::render::render_resource::PipelineCache>, mut count: Local<usize>) {
    let total = cache.pipelines().count();
    if total != *count || cache.waiting_pipelines().next().is_some() {
        frame_event(EV_PIPELINE);
    }
    *count = total;
}
pub fn frame_event(bit: u64) {
    FRAME_EVENTS.fetch_or(bit, Relaxed);
}
fn spike_log(time: Res<Time<Real>>, mut frame: Local<u64>, mut history: Local<Vec<(u64, u64)>>,
    mut env: Local<([u64; 4], [u64; 4])>) {
    *frame += 1;
    let (filter, slices) = (bevy::pbr::generate::environment_filter_cache_counts(),
        bevy::pbr::generate::environment_slice_counts());
    if filter[2] != env.0[2] { frame_event(EV_ENV_SOURCE); }
    if slices[1] != env.1[1] { frame_event(EV_ENV_PUBLISH); }
    *env = (filter, slices);
    let events = FRAME_EVENTS.swap(0, Relaxed);
    history.push((*frame, events));
    if history.len() > 4 { history.remove(0); }
    let ms = time.delta_secs_f64() * 1000.;
    if ms > 25. {
        // The slow frame's delta is measured at the start of the next frame,
        // so print the preceding frames' events as well.
        eprintln!("SPIKE frame {} {:.1} ms events {:?}", *frame, ms, *history);
    }
}
/// Terrain tiles build on the single async worker. Left at default QoS it
/// shares the four M1 performance cores with the main, render and compute
/// threads, and frames whose submission it delays arrive late. Utility QoS
/// lets macOS prefer the efficiency cores for it. Opt-in with
/// `NORD_WORKER_QOS=1`: three idle-machine A/B rounds were inconclusive.
pub fn background_worker_qos() -> Option<std::sync::Arc<dyn Fn() + Send + Sync + 'static>> {
    #[cfg(target_os = "macos")]
    {
        if std::env::var_os("NORD_WORKER_QOS").is_none() {
            return None;
        }
        unsafe extern "C" {
            fn pthread_set_qos_class_self_np(qos_class: u32, relative_priority: i32) -> i32;
        }
        const QOS_CLASS_UTILITY: u32 = 0x11;
        Some(std::sync::Arc::new(|| unsafe {
            pthread_set_qos_class_self_np(QOS_CLASS_UTILITY, 0);
        }))
    }
    #[cfg(not(target_os = "macos"))]
    None
}
pub fn terrain_staged(bytes:usize,meshes:usize) {
    if enabled() && meshes>0 {
        TERRAIN_STAGE_FRAMES.fetch_add(1,Relaxed);
        TERRAIN_STAGE_BYTES.fetch_add(bytes as u64,Relaxed);
        TERRAIN_STAGE_MESHES.fetch_add(meshes as u64,Relaxed);
        TERRAIN_STAGE_MAX_BYTES.fetch_max(bytes as u64,Relaxed);
        TERRAIN_STAGE_MAX_MESHES.fetch_max(meshes as u64,Relaxed);
    }
}
pub fn advanced(seconds: f64) {
    if enabled() && seconds > 0. {
        SIMULATED_NS.fetch_add((seconds * 1e9) as u64, Relaxed);
    }
}
pub fn simulated_seconds() -> f64 {
    SIMULATED_NS.load(Relaxed) as f64 / 1e9
}

pub fn enabled() -> bool {
    ENABLED.load(Relaxed)
}
pub struct Span {
    index: usize,
    start: Option<Instant>,
}
pub fn span(index: usize) -> Span {
    Span {
        index,
        start: enabled().then(Instant::now),
    }
}
impl Drop for Span {
    fn drop(&mut self) {
        if let Some(start) = self.start {
            NANOS[self.index].fetch_add(start.elapsed().as_nanos() as u64, Relaxed);
            CALLS[self.index].fetch_add(1, Relaxed);
        }
    }
}
pub fn reset() {
    bevy::pbr::generate::reset_environment_filter_cache_counts();
    for a in NANOS.iter().chain(CALLS.iter()).chain([
        &UPLOAD_BYTES,
        &UPLOAD_CALLS,
        &MESH_CHANGES,
        &MATERIAL_CHANGES,
        &IMAGE_CHANGES,
        &SIMULATED_NS,
        &EARTH_MATERIAL_CHANGES,
        &FOREST_VIEW_VERTICES, &FOREST_SHADOW_VERTICES, &FOREST_SAMPLES,
        &TERRAIN_STAGE_FRAMES, &TERRAIN_STAGE_BYTES, &TERRAIN_STAGE_MESHES,
        &TERRAIN_STAGE_MAX_BYTES, &TERRAIN_STAGE_MAX_MESHES,
        &MESH_SLAB_MAX_BYTES, &MESH_SLAB_MAX_COUNT,
    ]) {
        a.store(0, Relaxed);
    }
}
pub fn report(diagnostics: &DiagnosticsStore) -> serde_json::Value {
    let spans: serde_json::Map<String, serde_json::Value> = NAMES
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let calls = CALLS[i].load(Relaxed);
            let ms = NANOS[i].load(Relaxed) as f64 / 1e6;
            (
                (*name).into(),
                serde_json::json!({"calls":calls,"total_ms":ms,"mean_ms":ms / calls.max(1) as f64}),
            )
        })
        .collect();
    let render: serde_json::Map<String, serde_json::Value> = diagnostics
        .iter()
        .filter(|d| {
            (d.path().as_str().starts_with("render")
                || d.path().as_str().starts_with("mesh_allocator"))
                && (!d.path().as_str().ends_with("elapsed_gpu")
                    || d.average().is_some_and(|x| x > 0.))
        })
        .map(|d| (d.path().as_str().to_owned(), serde_json::json!(d.average())))
        .collect();
    serde_json::json!({"cpu_spans":spans,"render_diagnostics":render,
        "environment_filter_cache_hits_misses_writes_bypasses":bevy::pbr::generate::environment_filter_cache_counts(),
        "environment_slices_captured_published_cancelled_max_frames":bevy::pbr::generate::environment_slice_counts(),
        "upload_bytes":UPLOAD_BYTES.load(Relaxed),"upload_calls":UPLOAD_CALLS.load(Relaxed),
        "mesh_modifications":MESH_CHANGES.load(Relaxed),"material_modifications":MATERIAL_CHANGES.load(Relaxed)+EARTH_MATERIAL_CHANGES.load(Relaxed),
        "earth_material_modifications":EARTH_MATERIAL_CHANGES.load(Relaxed),
        "image_modifications":IMAGE_CHANGES.load(Relaxed),
        "terrain_staging":{"frames":TERRAIN_STAGE_FRAMES.load(Relaxed),
            "mesh_source_bytes":TERRAIN_STAGE_BYTES.load(Relaxed),
            "meshes":TERRAIN_STAGE_MESHES.load(Relaxed),
            "max_mesh_source_bytes_per_frame":TERRAIN_STAGE_MAX_BYTES.load(Relaxed),
            "max_meshes_per_frame":TERRAIN_STAGE_MAX_MESHES.load(Relaxed)},
        "mesh_slab_peak":{"bytes":MESH_SLAB_MAX_BYTES.load(Relaxed),
            "slabs":MESH_SLAB_MAX_COUNT.load(Relaxed)},
        "adapter":ADAPTER.get(),
        "gpu_timing_note":"Only positive GPU durations are retained. nord_scene measures the encoder envelope where timestamps are supported; unsupported per-pass timestamps are omitted. Validate with Metal System Trace.",
        "forest_vertices_in_visible_batches_mean":{
            "camera":FOREST_VIEW_VERTICES.load(Relaxed) as f64/FOREST_SAMPLES.load(Relaxed).max(1) as f64,
            "shadow_cascades":FOREST_SHADOW_VERTICES.load(Relaxed) as f64/FOREST_SAMPLES.load(Relaxed).max(1) as f64},
        "draw_calls":null,"cpu_allocations":null})
}
pub fn compute_threads() -> usize {
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
    std::env::var("NORD_COMPUTE_THREADS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(cores.saturating_sub(2).max(1))
        .clamp(1, cores)
}
pub struct PerformancePlugin;
impl Plugin for PerformancePlugin {
    fn build(&self, app: &mut App) {
        let active = (std::env::args().any(|a| a == "--benchmark-render")
            || std::env::var_os("NORD_PROFILE").is_some())
            && std::env::var_os("NORD_BENCH_FPS_ONLY").is_none();
        ENABLED.store(active, Relaxed);
        if std::env::var_os("NORD_SPIKE_LOG").is_some() {
            app.add_systems(First, spike_log);
            if let Some(render) = app.get_sub_app_mut(bevy::render::RenderApp) {
                render.add_systems(bevy::render::Render, pipeline_activity
                    .after(bevy::render::RenderSystems::Prepare));
            }
        }
        if active && let Some(folder) = std::env::var_os("NORD_CAPTURE_SEQUENCE") {
            let folder = std::path::PathBuf::from(folder);
            std::fs::create_dir_all(&folder).expect("create native sequence folder");
            app.insert_resource(FrameSequence {
                folder,
                start: None,
                count: 0,
            })
            .add_systems(PostUpdate, capture_sequence);
        }
        if active {
            app.add_plugins((
                bevy::render::diagnostic::RenderDiagnosticsPlugin,
                bevy::render::diagnostic::MeshAllocatorDiagnosticPlugin,
            ))
            .add_systems(PostUpdate, (asset_changes, mesh_slab_peaks))
            .add_systems(PostUpdate, forest_visibility
                .after(bevy::camera::visibility::VisibilitySystems::CheckVisibility)
                .after(bevy::light::SimulationLightSystems::CheckLightVisibility));
            if let Some(render) = app.get_sub_app_mut(bevy::render::RenderApp) {
                render.init_resource::<PrepareClock>().add_systems(
                    bevy::render::Render,
                    (
                        start_prepare.before(bevy::render::RenderSystems::PrepareAssets),
                        end_prepare.after(bevy::render::RenderSystems::PrepareAssets),
                    ),
                );
                let mut graph = render.world_mut().resource_mut::<RenderGraph>();
                graph.add_node(GpuBegin, TimingNode(true));
                graph.add_node(GpuEnd, TimingNode(false));
                graph.add_node_edge(GpuBegin, bevy::render::graph::CameraDriverLabel);
                graph.add_node_edge(bevy::render::graph::CameraDriverLabel, GpuEnd);
            }
        }
    }
    fn finish(&self, app: &mut App) {
        if !enabled() {
            return;
        }
        if let Some(render) = app.get_sub_app(bevy::render::RenderApp) {
            let world = render.world();
            if let (Some(adapter), Some(device)) = (
                world.get_resource::<bevy::render::renderer::RenderAdapterInfo>(),
                world.get_resource::<bevy::render::renderer::RenderDevice>(),
            ) {
                let _=ADAPTER.set(serde_json::json!({"name":adapter.name,"backend":format!("{:?}",adapter.backend),"features":format!("{:?}",device.features())}));
            }
        }
    }
}
#[derive(Resource)]
struct FrameSequence {
    folder: std::path::PathBuf,
    start: Option<f64>,
    count: usize,
}
fn capture_sequence(
    mut commands: Commands,
    f: Res<crate::physics::Flight>,
    view: Res<crate::ViewState>,
    mut sequence: ResMut<FrameSequence>,
) {
    let start = *sequence.start.get_or_insert(f.elapsed);
    if sequence.count >= 32 || f.elapsed - start < sequence.count as f64 * 2. {
        return;
    }
    let stem = sequence.folder.join(format!("frame-{:03}", sequence.count));
    sequence.count += 1;
    let state = serde_json::json!({"simulation_s":f.elapsed,"position":f.position.to_array(),
        "velocity":f.velocity.to_array(),"orientation":f.orientation.to_array(),
        "altitude_m":f.altitude(),"map":view.map,"camera":view.index});
    commands
        .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
        .observe(
            move |event: On<bevy::render::view::screenshot::ScreenshotCaptured>| {
                event
                    .image
                    .clone()
                    .try_into_dynamic()
                    .expect("native sequence image")
                    .to_rgb8()
                    .save(stem.with_extension("png"))
                    .expect("save native sequence");
                std::fs::write(
                    stem.with_extension("json"),
                    serde_json::to_string_pretty(&state).unwrap(),
                )
                .expect("save native sequence state");
            },
        );
}
#[derive(Resource, Default)]
struct PrepareClock(Option<Instant>);
fn forest_visibility(
    views: Query<&bevy::camera::primitives::Frustum, With<crate::FlightCamera>>,
    lights: Query<&bevy::camera::visibility::CascadesVisibleEntities>,
    forest: Query<(Entity, &Mesh3d), With<crate::terrain::ForestBatch>>,
    meshes: Res<Assets<Mesh>>,
    bounds: Query<(&bevy::camera::primitives::Aabb, &GlobalTransform), With<crate::terrain::ForestBatch>>,
) {
    let vertices = |entity: &Entity| forest.get(*entity).ok()
        .and_then(|(_,m)| meshes.get(&m.0)).map_or(0, |m| m.count_vertices() as u64);
    // VisibleEntities is consumed by GPU visibility extraction. Inspect the
    // same conservative OBB test directly rather than counting a drained list.
    let camera = forest.iter().filter(|(entity,_)| bounds.get(*entity).ok()
        .is_some_and(|(aabb,transform)| views.iter().any(|frustum|
            frustum.intersects_obb(aabb,&transform.affine(),true,false))))
        .map(|(entity,_)|vertices(&entity)).sum();
    let shadow = lights.iter().flat_map(|l|l.entities.values()).flatten()
        .flat_map(|v|v.entities.iter()).map(vertices).sum();
    FOREST_VIEW_VERTICES.fetch_add(camera,Relaxed);
    FOREST_SHADOW_VERTICES.fetch_add(shadow,Relaxed);
    FOREST_SAMPLES.fetch_add(1,Relaxed);
}
fn start_prepare(mut clock: ResMut<PrepareClock>) {
    clock.0 = Some(Instant::now());
}
fn end_prepare(mut clock: ResMut<PrepareClock>) {
    if let Some(start) = clock.0.take() {
        NANOS[16].fetch_add(start.elapsed().as_nanos() as u64, Relaxed);
        CALLS[16].fetch_add(1, Relaxed);
    }
}
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, RenderLabel)]
struct GpuBegin;
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, RenderLabel)]
struct GpuEnd;
struct TimingNode(bool);
impl Node for TimingNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        context: &mut RenderContext,
        _world: &World,
    ) -> Result<(), NodeRunError> {
        let diagnostics = context.diagnostic_recorder();
        if self.0 {
            diagnostics.begin_time_span(context.command_encoder(), "nord_scene".into());
        } else {
            diagnostics.end_time_span(context.command_encoder());
        }
        Ok(())
    }
}
fn asset_changes(
    mut meshes: MessageReader<AssetEvent<Mesh>>,
    mut materials: MessageReader<AssetEvent<StandardMaterial>>,
    mut images: MessageReader<AssetEvent<Image>>,
) {
    MESH_CHANGES.fetch_add(
        meshes
            .read()
            .filter(|e| matches!(e, AssetEvent::Modified { .. }))
            .count() as u64,
        Relaxed,
    );
    MATERIAL_CHANGES.fetch_add(
        materials
            .read()
            .filter(|e| matches!(e, AssetEvent::Modified { .. }))
            .count() as u64,
        Relaxed,
    );
    IMAGE_CHANGES.fetch_add(
        images
            .read()
            .filter(|e| matches!(e, AssetEvent::Modified { .. }))
            .count() as u64,
        Relaxed,
    );
}
