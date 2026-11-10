//! A lower-resolution 3D target composited beneath native-resolution UI.
use super::GraphicsSettings;
use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    window::PrimaryWindow,
};
#[derive(Resource, Clone)]
pub struct Graphics(pub GraphicsSettings);
#[derive(Resource)]
struct WorldTarget {
    image: Handle<Image>,
    size: UVec2,
}
#[derive(Component)]
struct Composite;
/// The world-image compositor must remain visible when flight UI is suppressed.
#[derive(Component)]pub(crate) struct WorldImage;
/// Convert the scaled world image's viewport into native interface coordinates.
pub(crate) fn project_to_ui(camera:&Camera,transform:&GlobalTransform,point:Vec3,size:Vec2)->Option<Vec2>{
    let viewport=camera.logical_viewport_size()?;
    if viewport.min_element()<=0.{return None;}
    camera.world_to_viewport(transform,point).ok().map(|p|p/viewport*size)
}
pub struct GraphicsPlugin;
impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        super::shadows::install(app);
        super::relief_shadow::install(app);
        app.insert_resource(Graphics(super::active().graphics.clone()))
            .add_systems(First, pace_frame.before(bevy::time::TimeSystems))
            .add_systems(PreStartup, initialize_render_height)
            .add_systems(PostStartup, setup)
            .add_systems(PreUpdate, (resize, apply_shadows).chain())
            .add_systems(PostUpdate, redirect_ui.before(bevy::ui::UiSystems::Prepare));
    }
}
// Vsync alone does not pace main-world updates with pipelined rendering on
// Metal: several fast updates can bunch up before a long present wait. Pace
// starts before Time updates, so ordinary simulation still receives real dt.
fn pace_frame(graphics:Res<Graphics>,mut previous:Local<Option<std::time::Instant>>) {
    let limit=graphics.0.frame_rate_limit;
    if limit==0{*previous=None;return;}
    let now=std::time::Instant::now();
    if let Some(last)=*previous {
        let target=last+std::time::Duration::from_secs_f64(1./limit as f64);
        if target>now{std::thread::sleep(target-now);}
    }
    *previous=Some(std::time::Instant::now());
}
fn dimensions(window: &Window, scale: f32) -> UVec2 {
    UVec2::new(
        ((window.physical_width() as f32 * scale).round() as u32).max(1),
        ((window.physical_height() as f32 * scale).round() as u32).max(1),
    )
}
fn initialize_render_height(windows: Query<&Window, With<PrimaryWindow>>, graphics: Res<Graphics>) {
    if let Ok(window) = windows.single() {
        let size = dimensions(window, graphics.0.render_scale);
        HEIGHT.store(size.y, std::sync::atomic::Ordering::Relaxed);
    }
}
fn target(size: UVec2) -> Image {
    let mut image = Image::new_fill(
        Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Bgra8UnormSrgb,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT;
    image.sampler = bevy::image::ImageSampler::linear();
    image
}
fn setup(
    mut c: Commands,
    mut images: ResMut<Assets<Image>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<Entity, With<crate::FlightCamera>>,
    graphics: Res<Graphics>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok(camera) = cameras.single() else {
        return;
    };
    let size = dimensions(window, graphics.0.render_scale);
    let image = images.add(target(size));
    c.entity(camera)
        .remove::<bevy::ui::IsDefaultUiCamera>()
        .insert((
            Camera {
                order: -1,
                ..default()
            },
            RenderTarget::Image(image.clone().into()),
        ));
    let composite = c
        .spawn((
            Camera2d,
            Camera {
                order: 0,
                ..default()
            },
            RenderLayers::layer(31),
            bevy::ui::IsDefaultUiCamera,
            Composite,
        ))
        .id();
    c.spawn((
        WorldImage,
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            ..default()
        },
        ImageNode::new(image.clone()),
        GlobalZIndex(-10000),
        bevy::ui::UiTargetCamera(composite),
        bevy::ui::FocusPolicy::Pass,
    ));
    c.insert_resource(WorldTarget { image, size });
}
static ERROR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static HEIGHT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(900);
static SHADOW_DISTANCE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
pub fn lod_error() -> f64 {
    let bits = ERROR.load(std::sync::atomic::Ordering::Relaxed);
    if bits == 0 {
        super::active().graphics.terrain_error_pixels
    } else {
        f64::from_bits(bits)
    }
}
pub fn render_height() -> f64 {
    HEIGHT.load(std::sync::atomic::Ordering::Relaxed) as f64
}
pub(crate) fn shadow_distance() -> f32 {
    let bits = SHADOW_DISTANCE.load(std::sync::atomic::Ordering::Relaxed);
    if bits == 0 {
        super::active().graphics.shadow_distance
    } else {
        f32::from_bits(bits)
    }
}
fn resize(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut graphics: ResMut<Graphics>,
    view: Res<crate::ViewState>,
    mut previous: Local<Option<bool>>,
    target: Option<ResMut<WorldTarget>>,
    mut images: ResMut<Assets<Image>>,
) {
    let (Some(mut target), Ok(window)) = (target, windows.single()) else {
        return;
    };
    if previous.is_some_and(|old| old != view.high_quality) {
        let scale = graphics.0.render_scale;
        let frame_limit = graphics.0.frame_rate_limit;
        let grass=(graphics.0.grass_distance,graphics.0.grass_density);
        graphics.0 = if view.high_quality {
            GraphicsSettings::high()
        } else {
            GraphicsSettings::default()
        };
        graphics.0.render_scale = scale;
        graphics.0.frame_rate_limit = frame_limit;
        (graphics.0.grass_distance,graphics.0.grass_density)=grass;
    }
    *previous = Some(view.high_quality);
    ERROR.store(
        graphics.0.terrain_error_pixels.to_bits(),
        std::sync::atomic::Ordering::Relaxed,
    );
    let size = dimensions(window, graphics.0.render_scale);
    HEIGHT.store(size.y, std::sync::atomic::Ordering::Relaxed);
    if target.size != size {
        if let Some(image) = images.get_mut(&target.image) {
            *image = self::target(size);
        }
        target.size = size;
    }
    // Keep the interface at window resolution while only replacing the 3D target.
    // Saves are disabled in disposable diagnostics.
    if graphics.is_changed()
        && std::env::var_os("NORD_BENCH_REPORT").is_none()
        && !std::env::args().any(|a| a == "--visual-check" || a == "--benchmark-render")
    {
        if let Some(path) = settings_path() {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Ok(json) = serde_json::to_vec_pretty(&graphics.0) {
                let _ = std::fs::write(path, json);
            }
        }
    }
}

pub fn settings_path() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("APPDATA").or_else(|| std::env::var_os("HOME"))?;
    Some(std::path::PathBuf::from(base).join(".nord-flight/graphics.json"))
}

fn apply_shadows(
    graphics: Res<Graphics>,
    mut map: ResMut<bevy::light::DirectionalLightShadowMap>,
    mut suns: Query<&mut bevy::light::CascadeShadowConfig, With<crate::world::Sun>>,
) {
    if !graphics.is_changed() || !super::real() {
        return;
    }
    SHADOW_DISTANCE.store(
        graphics.0.shadow_distance.to_bits(),
        std::sync::atomic::Ordering::Relaxed,
    );
    map.size = if graphics.0.terrain_error_pixels <= 2. {
        2048
    } else {
        1024
    };
    for mut cascades in &mut suns {
        *cascades = bevy::light::CascadeShadowConfigBuilder {
            first_cascade_far_bound: 180f32.min(graphics.0.shadow_distance * 0.35),
            maximum_distance: graphics.0.shadow_distance,
            num_cascades: 2,
            ..default()
        }
        .build();
    }
}

fn redirect_ui(
    flight: Query<Entity, With<crate::FlightCamera>>,
    composite: Query<Entity, With<Composite>>,
    mut nodes: Query<&mut bevy::ui::UiTargetCamera>,
) {
    let (Ok(flight), Ok(composite)) = (flight.single(), composite.single()) else {
        return;
    };
    for mut target in &mut nodes {
        if target.0 == flight {
            target.0 = composite;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn world_markers_stay_on_aircraft_at_every_render_scale_and_display_density(){
        let ui=Vec2::new(1440.,900.);let transform=GlobalTransform::default();
        let point=Vec3::new(100.,50.,-1000.);
        let mut expected=None;
        for density in [1.,2.]{for scale in [0.5,0.67,1.]{
            let mut camera=Camera::default();
            camera.computed.target_info=Some(bevy::camera::RenderTargetInfo{
                physical_size:(ui*density*scale).as_uvec2(),scale_factor:1.});
            camera.computed.clip_from_view=Mat4::perspective_infinite_reverse_rh(60_f32.to_radians(),ui.x/ui.y,0.1);
            let p=project_to_ui(&camera,&transform,point,ui).unwrap();
            assert!(p.x>720.&&p.y<450.);
            if let Some(reference)=expected{assert!(p.distance(reference)<0.001);}else{expected=Some(p);}
            assert!(project_to_ui(&camera,&transform,-point,ui).is_none());
        }}
    }
    #[test]
    fn explicit_flight_ui_is_native_but_menu_target_is_preserved() {
        let mut app = App::new();
        let flight = app.world_mut().spawn(crate::FlightCamera).id();
        let native = app.world_mut().spawn(Composite).id();
        let menu = app.world_mut().spawn_empty().id();
        let a = app.world_mut().spawn(bevy::ui::UiTargetCamera(flight)).id();
        let b = app.world_mut().spawn(bevy::ui::UiTargetCamera(menu)).id();
        app.add_systems(Update, redirect_ui);
        app.update();
        assert_eq!(
            app.world().get::<bevy::ui::UiTargetCamera>(a).unwrap().0,
            native
        );
        assert_eq!(
            app.world().get::<bevy::ui::UiTargetCamera>(b).unwrap().0,
            menu
        );
    }
}
