//! A mission transition uses a frozen staging Flight to stream the destination.
//! An opaque UI hides both the old scene and aircraft until resources are ready.
use bevy::{asset::{AssetId,UntypedAssetId},prelude::*};
use crate::{physics::Flight,ViewState,FlightStage};
#[derive(Default)]
pub(crate) struct Resources {
    pub meshes:Vec<AssetId<Mesh>>,pub images:Vec<AssetId<Image>>,pub materials:Vec<UntypedAssetId>,
}
impl Resources {pub fn extend(&mut self,other:Self){self.meshes.extend(other.meshes);self.images.extend(other.images);self.materials.extend(other.materials);}}
#[derive(Resource,Default)]
pub(crate) struct Loading {
    pub active:bool,
    generation:Option<u64>,resume_paused:bool,requested:bool,settled:u8,
    frames:u64,started:Option<std::time::Instant>,fade_started_at:f64,fade_out_started:Option<f64>,
    message:String,winit:Option<bevy::winit::WinitSettings>,
}
#[derive(Resource,Default)]struct HiddenUi(std::collections::HashMap<Entity,Visibility>);
#[derive(Resource,Clone,Default)]struct Ready(crate::terrain_ready::TerrainReady);
#[derive(Component)]struct Overlay;
#[derive(Component)]struct Indicator;
#[derive(Component)]struct Status;
#[derive(Component)]enum Fade{Background(f32),Image,Text,Border}
const FADE_SECONDS:f64=2.;
fn opacity(loading:&Loading,time:&Time<Real>)->f32{
    let start=loading.fade_out_started.unwrap_or(loading.fade_started_at);
    let t=((time.elapsed_secs_f64()-start)/FADE_SECONDS).clamp(0.,1.)as f32;
    let eased=t*t*(3.-2.*t);
    if loading.fade_out_started.is_some(){1.-eased}else{eased}
}
pub struct LoadingPlugin;
impl Plugin for LoadingPlugin {
    fn build(&self,app:&mut App){
        let ready=Ready::default();app.init_resource::<Loading>().init_resource::<HiddenUi>().insert_resource(ready.clone())
            .add_systems(First,restore_ui_visibility)
            .add_systems(Startup,setup)
            .add_systems(Update,begin.after(FlightStage::Controls).before(FlightStage::Simulation))
            .add_systems(Update,finish.after(FlightStage::Presentation).before(FlightStage::Interface))
            .add_systems(Update,ui.after(finish))
            .add_systems(PostUpdate,hold_outgoing_frame)
            .add_systems(PostUpdate,hide_flight_ui.before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate));
        if let Some(render)=app.get_sub_app_mut(bevy::render::RenderApp){
            render.insert_resource(ready).add_systems(bevy::render::Render,check
                .in_set(bevy::render::RenderSystems::PrepareAssets)
                .after(bevy::render::render_asset::prepare_assets::<bevy::render::mesh::RenderMesh>));
        }
    }
}
fn check(state:Res<Ready>,meshes:Res<bevy::render::render_asset::RenderAssets<bevy::render::mesh::RenderMesh>>,
    images:Res<bevy::render::render_asset::RenderAssets<bevy::render::texture::GpuImage>>,
    materials:Res<bevy::render::erased_render_asset::ErasedRenderAssets<bevy::pbr::PreparedMaterial>>,
    pipelines:Res<bevy::render::render_resource::PipelineCache>) {
    // Asset preparation and shader compilation both precede the reveal.
    if pipelines.waiting_pipelines().next().is_none(){
        state.0.poll_all(|id|meshes.get(id).is_some(),|id|images.get(id).is_some(),|id|materials.get(id).is_some());
    }
}
fn begin(mut diagnostic_transition:Local<bool>,mut loading:ResMut<Loading>,mut f:ResMut<Flight>,mut v:ResMut<ViewState>,menu:Res<crate::start_screen::Menu>,
    time:Res<Time<Real>>,benchmark:Option<Res<crate::RenderBenchmark>>,ready:Res<Ready>,mut settings:Option<ResMut<bevy::winit::WinitSettings>>) {
    if benchmark.is_some_and(|b|b.mode.is_some()) {return;}
    if menu.open{return;}
    // Native diagnostic exercises an actual large relocation after initial
    // coverage is ready, rather than only checking startup mesh publication.
    if !loading.active&&loading.generation.is_some()&&!*diagnostic_transition
        && std::env::var("NORD_LOADING_TRANSITION").as_deref()==Ok("mountains") {
        *diagnostic_transition=true;crate::reset(&mut f,19,&mut v);
    }
    if loading.generation!=Some(v.generation){
        loading.generation=Some(v.generation);loading.resume_paused=f.paused;
        loading.active=true;loading.requested=false;loading.settled=0;loading.frames=0;
        loading.started=Some(std::time::Instant::now());loading.fade_started_at=time.elapsed_secs_f64();loading.fade_out_started=None;
        loading.message="Preparing landscape…".into();ready.0.clear();
        if let Some(settings)=settings.as_mut(){
            if loading.winit.is_none(){loading.winit=Some((**settings).clone());}
            **settings=bevy::winit::WinitSettings::continuous();
        }
    }
    if loading.active{f.paused=true;loading.frames+=1;}
}
fn finish(mut loading:ResMut<Loading>,mut f:ResMut<Flight>,time:Res<Time<Real>>,
    terrain:Option<Res<crate::terrain::Landscape>>,
    vegetation:Option<Res<crate::terrain::vegetation::Vegetation>>,
    water:Option<Res<crate::ocean::WaterGrid>>,ready:Res<Ready>,
    server:Res<AssetServer>,scenes:Query<&SceneRoot>,meshes:Query<(&Mesh3d,Option<&Visibility>)>,
    materials:Query<(&MeshMaterial3d<StandardMaterial>,Option<&Visibility>)>,
    mut keys:ResMut<ButtonInput<KeyCode>>,mut mouse:ResMut<ButtonInput<MouseButton>>,
    mut settings:Option<ResMut<bevy::winit::WinitSettings>>) {
    if !loading.active{return;}
    f.paused=true;
    if loading.fade_out_started.is_none(){
    if opacity(&loading,&time)<1.{return;}
    if scenes.iter().any(|scene|!server.is_loaded_with_dependencies(scene.0.id())){
        loading.message="Loading aircraft and textures…".into();return;
    }
    let Some(mut resources)=crate::terrain::loading_resources(&f,terrain.as_deref())else{
        loading.message="Building terrain…".into();return;
    };
    let Some(trees)=crate::terrain::vegetation::loading_resources(&f,vegetation.as_deref())else{
        loading.message="Preparing nearby vegetation…".into();return;
    };resources.extend(trees);
    let Some(water)=crate::ocean::loading_resources(&f,water.as_deref())else{
        loading.message="Preparing shorelines and water…".into();return;
    };resources.extend(water);
    if !loading.requested {
        resources.meshes.extend(meshes.iter().filter(|(_,v)|!v.is_some_and(|v|*v==Visibility::Hidden)).map(|(m,_)|m.0.id()));
        resources.materials.extend(materials.iter().filter(|(_,v)|!v.is_some_and(|v|*v==Visibility::Hidden)).map(|(m,_)|m.0.id().untyped()));
        ready.0.request(resources.meshes);ready.0.dependencies(resources.images,resources.materials);
        loading.requested=true;loading.message="Preparing graphics…".into();return;
    }
    if !ready.0.ready(){return;}
    // Let scene spawning, transforms and visibility settle before revealing.
    loading.settled+=1;
    if loading.settled<3{return;}
    loading.fade_out_started=Some(time.elapsed_secs_f64());loading.message="Scene ready".into();
    return;
    }
    if opacity(&loading,&time)>0.{return;}
    f.paused=loading.resume_paused;loading.active=false;ready.0.clear();
    keys.reset_all();mouse.reset_all();
    if let (Some(settings),Some(original))=(settings.as_mut(),loading.winit.take()){**settings=original;}
    if let Ok(path)=std::env::var("NORD_LOADING_REPORT"){
        let report=serde_json::json!({"generation":loading.generation,"loading_frames":loading.frames,
            "loading_wall_s":loading.started.map(|t|t.elapsed().as_secs_f64()),"flight_elapsed_at_release":f.elapsed,
            "position_at_release":f.position.to_array(),"resumed_paused":f.paused});
        if let Err(error)=std::fs::write(path,serde_json::to_string_pretty(&report).unwrap()) {error!("Loading report: {error}");}
    }
    info!("Scene ready: generation {:?}, flight held at {:.3}s",loading.generation,f.elapsed);
}
// Restore before normal UI systems run, then suppress their final visibility
// before extraction. This preserves each panel's current rules, including
// panels intentionally hidden or created/despawned while a scene is loading.
fn restore_ui_visibility(mut saved:ResMut<HiddenUi>,mut nodes:Query<&mut Visibility>){
    for(entity,visibility)in saved.0.drain(){
        if let Ok(mut current)=nodes.get_mut(entity){*current=visibility;}
    }
}
fn hide_flight_ui(loading:Res<Loading>,menu:Option<Res<crate::start_screen::Menu>>,mut saved:ResMut<HiddenUi>,
    keep:Query<(Entity,Option<&crate::environment::graphics::WorldImage>,Option<&crate::start_screen::Root>),Or<(With<Overlay>,With<crate::environment::graphics::WorldImage>,With<crate::start_screen::Root>)>>,
    children:Query<&Children>,mut nodes:Query<(Entity,&mut Visibility),With<Node>>){
    let menu_open=menu.is_some_and(|m|m.open);
    if !loading.active&&!menu_open{return;}
    let mut exempt=std::collections::HashSet::new();
    let mut stack=keep.iter().filter(|(_,world,root)|(world.is_none()||!menu_open)&&(root.is_none()||menu_open)).map(|(e,_,_)|e).collect::<Vec<_>>();
    while let Some(entity)=stack.pop(){
        if exempt.insert(entity){if let Ok(children)=children.get(entity){stack.extend(children.iter());}}
    }
    // Include descendants too: Visibility::Visible children can override a
    // hidden ancestor and would otherwise leak HUD text through the fades.
    for(entity,mut visibility)in &mut nodes{
        if !exempt.contains(&entity){saved.0.insert(entity,*visibility);*visibility=Visibility::Hidden;}
    }
}
// Retain the outgoing world image during the cross-fade. Destination rendering
// starts behind the fully opaque overlay, so unfinished terrain cannot flash.
fn hold_outgoing_frame(loading:Res<Loading>,time:Res<Time<Real>>,mut cameras:Query<&mut Camera,With<crate::FlightCamera>>){
    if loading.active{for mut camera in &mut cameras{camera.is_active=loading.fade_out_started.is_some()||opacity(&loading,&time)>=1.;}}
}
fn setup(mut c:Commands,assets:Res<AssetServer>){
    c.spawn((Overlay,Fade::Background(1.),GlobalZIndex(1_000_000),Node{position_type:PositionType::Absolute,
        width:percent(100),height:percent(100),display:Display::None,flex_direction:FlexDirection::Column,
        align_items:AlignItems::Center,justify_content:JustifyContent::Center,row_gap:px(18),..default()},
        BackgroundColor(Color::srgb(0.009,0.019,0.033)),bevy::ui::FocusPolicy::Block))
        .with_children(|p|{
            // Fit the complete square identity artwork without stretching or
            // cropping its wordmark on wide or portrait windows.
            p.spawn((Node{position_type:PositionType::Absolute,width:percent(100),height:percent(100),
                align_items:AlignItems::Center,justify_content:JustifyContent::Center,..default()},ZIndex(0)))
                .with_children(|p|{p.spawn((Fade::Image,ImageNode::new(assets.load("branding/nord-flight-icon.png")),
                    Node{width:vmin(100),height:vmin(100),..default()}));});
            p.spawn((Node{position_type:PositionType::Absolute,width:percent(100),height:percent(100),..default()},
                BackgroundColor(Color::srgba(0.003,0.008,0.016,0.32)),Fade::Background(0.32),ZIndex(1)));
            p.spawn((Node{width:px(540),max_width:percent(90),padding:UiRect::all(px(28)),
                flex_direction:FlexDirection::Column,align_items:AlignItems::Center,row_gap:px(18),
                border_radius:BorderRadius::all(px(12)),..default()},
                BackgroundColor(Color::srgba(0.004,0.012,0.022,0.88)),Fade::Background(0.88),ZIndex(2)))
                .with_children(|p|{
            p.spawn((Indicator,Fade::Border,Node{width:px(36),height:px(36),border:UiRect::all(px(3)),border_radius:BorderRadius::all(px(36)),..default()},
                BorderColor{top:Color::srgb(0.3,0.85,0.83),right:Color::srgb(0.3,0.85,0.83),
                    bottom:Color::srgb(0.08,0.2,0.23),left:Color::srgb(0.08,0.2,0.23)}));
            p.spawn((Fade::Text,Text::new("LOADING SCENE"),TextFont{font_size:24.,..default()},TextColor(Color::WHITE)));
            p.spawn((Fade::Text,Status,Text::new("Preparing landscape…"),TextFont{font_size:16.,..default()},TextColor(Color::srgb(0.65,0.78,0.82))));
            p.spawn((Fade::Text,Text::new("Your aircraft will start when the scene is ready."),TextFont{font_size:14.,..default()},TextColor(Color::srgb(0.45,0.6,0.65))));
                });
        });
}
fn ui(mut c:Commands,mut captured:Local<bool>,mut captured_early:Local<bool>,mut captured_out:Local<bool>,loading:Res<Loading>,time:Res<Time<Real>>,mut roots:Query<&mut Node,With<Overlay>>,
    mut indicator:Query<&mut UiTransform,With<Indicator>>,mut status:Query<&mut Text,(With<Status>,Without<Indicator>)>,
    mut fades:Query<(&Fade,Option<&mut BackgroundColor>,Option<&mut ImageNode>,Option<&mut TextColor>,Option<&mut BorderColor>)>){
    for mut node in &mut roots{node.display=if loading.active{Display::Flex}else{Display::None};}
    if loading.active{
        let alpha=opacity(&loading,&time);
        for(base,background,image,text,border)in &mut fades{
            // Node adds transparent background/border components automatically;
            // fade only the intended visual, leaving those defaults transparent.
            match base{
                Fade::Background(base)=>if let Some(mut color)=background{color.0.set_alpha(*base*alpha);},
                Fade::Image=>if let Some(mut image)=image{image.color.set_alpha(alpha);},
                Fade::Text=>if let Some(mut color)=text{color.0.set_alpha(alpha);},
                Fade::Border=>if let Some(mut color)=border{color.top.set_alpha(alpha);color.right.set_alpha(alpha);color.bottom.set_alpha(alpha);color.left.set_alpha(alpha);},
            }
        }
        if !*captured_early&&loading.frames>=5&&loading.fade_out_started.is_none()&&alpha>0.25&&alpha<0.7{
            if let Ok(path)=std::env::var("NORD_CAPTURE_LOADING_EARLY"){
                *captured_early=true;c.spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                    .observe(bevy::render::view::screenshot::save_to_disk(path));
            }
        }
        if !*captured_out&&loading.fade_out_started.is_some()&&alpha<0.6&&alpha>0.2 {
            if let Ok(path)=std::env::var("NORD_CAPTURE_LOADING_OUT"){
                *captured_out=true;c.spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                    .observe(bevy::render::view::screenshot::save_to_disk(path));
            }
        }
        for mut transform in &mut indicator{transform.rotation=Rot2::radians(time.elapsed_secs()*4.);}
        if !*captured&&loading.frames>=5&&alpha>=1. {
            if let Ok(path)=std::env::var("NORD_CAPTURE_LOADING"){
                *captured=true;
                c.spawn(bevy::render::view::screenshot::Screenshot::primary_window()).observe(
                    bevy::render::view::screenshot::save_to_disk(path));
            }
        }
        for mut text in &mut status{text.0=loading.message.clone();}
    }
}

#[cfg(test)]mod tests {
    use super::*;
    fn app(paused:bool)->App {
        let mut app=App::new();
        app.add_plugins((MinimalPlugins,bevy::asset::AssetPlugin::default()))
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f64(1./60.)))
            .init_resource::<Loading>().init_resource::<Ready>()
            .init_resource::<ViewState>().init_resource::<crate::start_screen::Menu>()
            .init_resource::<crate::physics::Environment>().init_resource::<crate::warp::Warp>()
            .init_resource::<crate::SimClock>().init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_systems(Update,(begin,crate::step,finish).chain());
        let mut flight=Flight::spacecraft();flight.paused=paused;app.insert_resource(flight);
        app.world_mut().spawn(Mesh3d(Handle::<Mesh>::default()));
        app
    }
    #[test]
    fn setup_menu_hides_flight_ui_and_world_compositor_but_keeps_its_own_controls(){
        let mut app=App::new();app.add_plugins(MinimalPlugins).init_resource::<Loading>().init_resource::<HiddenUi>().init_resource::<crate::start_screen::Menu>()
            .add_systems(First,restore_ui_visibility).add_systems(PostUpdate,hide_flight_ui);
        let hud=app.world_mut().spawn((Node::default(),Visibility::Visible)).id();
        let world=app.world_mut().spawn((Node::default(),crate::environment::graphics::WorldImage)).id();
        let root=app.world_mut().spawn((Node::default(),crate::start_screen::Root)).id();
        let child=app.world_mut().spawn((Node::default(),Visibility::Visible,ChildOf(root))).id();
        app.world_mut().resource_mut::<crate::start_screen::Menu>().open=true;app.update();
        assert_eq!(*app.world().get::<Visibility>(hud).unwrap(),Visibility::Hidden);
        assert_eq!(*app.world().get::<Visibility>(world).unwrap(),Visibility::Hidden);
        assert_eq!(*app.world().get::<Visibility>(root).unwrap(),Visibility::Inherited);
        assert_eq!(*app.world().get::<Visibility>(child).unwrap(),Visibility::Visible);
        app.world_mut().resource_mut::<crate::start_screen::Menu>().open=false;app.update();
        assert_eq!(*app.world().get::<Visibility>(hud).unwrap(),Visibility::Visible);
        assert_eq!(*app.world().get::<Visibility>(world).unwrap(),Visibility::Inherited);
    }
    #[test]fn physics_waits_for_gpu_readiness_then_restores_pause_state(){
        for paused in [false,true]{
            let mut app=app(paused);let before=app.world().resource::<Flight>().clone();
            for _ in 0..150{app.update();}
            let f=app.world().resource::<Flight>();
            assert_eq!(f.elapsed,before.elapsed);assert_eq!(f.position,before.position);assert_eq!(f.velocity,before.velocity);
            assert!(f.paused&&app.world().resource::<Loading>().active);
            app.world().resource::<Ready>().0.poll_all(|_|true,|_|true,|_|true);
            for _ in 0..3{app.update();}
            assert!(app.world().resource::<Loading>().active);
            assert!(app.world().resource::<Loading>().fade_out_started.is_some());
            for _ in 0..60{app.update();}
            assert!(app.world().resource::<Loading>().active);
            assert_eq!(app.world().resource::<Flight>().position,before.position);
            for _ in 0..90{
                app.update();
                assert_eq!(app.world().resource::<Flight>().elapsed,before.elapsed);
                if !app.world().resource::<Loading>().active{break;}
            }
            assert!(!app.world().resource::<Loading>().active);
            assert_eq!(app.world().resource::<Flight>().paused,paused);
            assert_eq!(app.world().resource::<Flight>().elapsed,before.elapsed);
            app.update();app.update();
            if paused{assert_eq!(app.world().resource::<Flight>().elapsed,before.elapsed);}
            else{assert!(app.world().resource::<Flight>().elapsed>before.elapsed);}
        }
    }
    #[test]fn destination_camera_only_renders_after_fade_covers_the_outgoing_frame(){
        let mut app=app(false);app.add_systems(PostUpdate,hold_outgoing_frame);
        let camera=app.world_mut().spawn((Camera::default(),crate::FlightCamera)).id();
        app.update();assert!(!app.world().get::<Camera>(camera).unwrap().is_active);
        for _ in 0..150{app.update();}
        assert!(app.world().resource::<Loading>().active);
        assert!(app.world().get::<Camera>(camera).unwrap().is_active);
        assert_eq!(app.world().resource::<Flight>().elapsed,0.);
    }
    #[test]fn loading_hides_hud_through_both_fades_and_restores_intended_visibility(){
        let mut app=App::new();app.add_plugins(MinimalPlugins).init_resource::<Loading>().init_resource::<HiddenUi>()
            .add_systems(First,restore_ui_visibility).add_systems(PostUpdate,hide_flight_ui);
        let world=app.world_mut().spawn((Node::default(),crate::environment::graphics::WorldImage)).id();
        let overlay=app.world_mut().spawn((Node::default(),Overlay)).id();
        let spinner=app.world_mut().spawn((Node::default(),ChildOf(overlay))).id();
        let hud=app.world_mut().spawn((Node::default(),Visibility::Inherited)).id();
        let label=app.world_mut().spawn((Node::default(),Visibility::Visible,ChildOf(hud))).id();
        let hidden=app.world_mut().spawn((Node::default(),Visibility::Hidden)).id();
        app.world_mut().resource_mut::<Loading>().active=true;
        app.update();
        assert_eq!(*app.world().get::<Visibility>(hud).unwrap(),Visibility::Hidden);
        assert_eq!(*app.world().get::<Visibility>(label).unwrap(),Visibility::Hidden);
        assert_eq!(*app.world().get::<Visibility>(spinner).unwrap(),Visibility::Inherited);
        assert_eq!(*app.world().get::<Visibility>(world).unwrap(),Visibility::Inherited);
        app.world_mut().resource_mut::<Loading>().fade_out_started=Some(1.);
        let new_hud=app.world_mut().spawn(Node::default()).id();
        app.update();assert_eq!(*app.world().get::<Visibility>(new_hud).unwrap(),Visibility::Hidden);
        app.world_mut().resource_mut::<Loading>().active=false;app.update();
        assert_eq!(*app.world().get::<Visibility>(hud).unwrap(),Visibility::Inherited);
        assert_eq!(*app.world().get::<Visibility>(label).unwrap(),Visibility::Visible);
        assert_eq!(*app.world().get::<Visibility>(hidden).unwrap(),Visibility::Hidden);
        assert_eq!(*app.world().get::<Visibility>(new_hud).unwrap(),Visibility::Inherited);
    }
    #[test]fn new_scene_request_discards_older_gpu_acknowledgement(){
        let mut app=app(false);for _ in 0..150{app.update();}
        app.world().resource::<Ready>().0.poll_all(|_|true,|_|true,|_|true);
        assert!(app.world().resource::<Ready>().0.ready());
        app.world_mut().resource_mut::<ViewState>().generation+=1;
        app.world_mut().resource_mut::<Flight>().paused=false;
        app.update();
        assert!(app.world().resource::<Loading>().active);
        assert!(!app.world().resource::<Ready>().0.ready());
        assert_eq!(app.world().resource::<Loading>().settled,0);
    }
}
