//! Launch configuration and an isolated, wall-time presentation scene.
use crate::{ViewState, aircraft::Aircraft, combat::dogfight::Difficulty, physics::*};
use bevy::{camera::visibility::RenderLayers, prelude::*};

const LAYER: usize = 5;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Setup,
    Pause,
    Settings,
    Controls,
}
#[derive(Resource)]
pub struct Menu {
    pub open: bool,
    launched: bool,
    saved_pause: bool,
    page: Page,
    back: Page,
    space: bool,
    aircraft: Aircraft,
    scenario: u8,
    difficulty: Difficulty,
    formation: usize,
    defense: bool,
    external: bool,
    fbw: bool,
    infinite: bool,
    dirty: bool,
    preview: Option<(bool, Aircraft, bool)>,
}
impl Default for Menu {
    fn default() -> Self {
        Self {
            open: false,
            launched: false,
            saved_pause: false,
            page: Page::Setup,
            back: Page::Setup,
            space: false,
            aircraft: Aircraft::F22,
            scenario: 1,
            difficulty: Difficulty::Balanced,
            formation: 1,
            defense: false,
            external: false,
            fbw: true,
            infinite: false,
            dirty: true,
            preview: None,
        }
    }
}
impl Menu {
    pub fn configured() -> Self {
        let mut m = Self::default();
        m.open = !std::env::args().skip(1).any(|arg| arg.starts_with("--"))
            || std::env::var_os("NORD_MENU_CAPTURE").is_some();
        if let Ok(mode) = std::env::var("NORD_MENU_CAPTURE") {
            m.space = mode == "space";
            m.scenario = if m.space { 3 } else { 1 };
            if mode == "settings" {
                m.page = Page::Settings;
            }
            if mode == "controls" {
                m.page = Page::Controls;
            }
        }
        m
    }
    fn scenario_id(&self) -> u8 {
        if self.scenario == 5 {
            match self.difficulty {
                Difficulty::Training => 5,
                Difficulty::Balanced => 6,
                Difficulty::Hard => 7,
            }
        } else {
            self.scenario
        }
    }
}
#[derive(Component)]
pub(crate) struct Root;
#[derive(Component)]
pub(crate) struct MenuCamera;
#[derive(Component)]
struct PreviewVehicle;
#[derive(Component)]
struct HangarProp;
#[derive(Component)]
struct SpaceProp;
#[derive(Component)]
struct EarthProp;
#[derive(Component, Clone, Copy)]
pub(crate) struct Action(u16);
pub fn flight_controls(menu: Res<Menu>) -> bool {
    !menu.open
}
pub fn asset_path() -> String {
    let executable = std::env::current_exe().unwrap_or_default();
    if let Some(dir) = executable.parent() {
        for path in [dir.join("../Resources/assets"), dir.join("assets")] {
            if path.is_dir() {
                return path.to_string_lossy().into_owned();
            }
        }
    }
    format!("{}/assets", env!("CARGO_MANIFEST_DIR"))
}
fn ink() -> Color {
    Color::srgb(0.012, 0.024, 0.041)
}
fn label(p: &mut ChildSpawnerCommands, text: impl Into<String>, size: f32, color: Color) {
    p.spawn((
        Text::new(text),
        TextFont {
            font_size: size,
            ..default()
        },
        TextColor(color),
    ));
}
fn button(p: &mut ChildSpawnerCommands, id: u16, text: &str, selected: bool) {
    p.spawn((
        Button,
        Action(id),
        Node {
            min_height: px(36),
            padding: UiRect::axes(px(12), px(9)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(5)),
            ..default()
        },
        BackgroundColor(if selected {
            Color::srgb(0.10, 0.30, 0.32)
        } else {
            Color::srgb(0.035, 0.07, 0.10)
        }),
        BorderColor::all(if selected {
            Color::srgb(0.35, 0.85, 0.78)
        } else {
            Color::srgb(0.12, 0.21, 0.27)
        }),
    ))
    .with_children(|p| label(p, text, 15., Color::srgb(0.88, 0.95, 0.97)));
}
fn row(p: &mut ChildSpawnerCommands, f: impl FnOnce(&mut ChildSpawnerCommands)) {
    p.spawn(Node {
        flex_wrap: FlexWrap::Wrap,
        column_gap: px(7),
        row_gap: px(7),
        ..default()
    })
    .with_children(f);
}
fn heading(p: &mut ChildSpawnerCommands, text: &str) {
    p.spawn((
        Node {
            margin: UiRect::top(px(12)),
            ..default()
        },
        Text::new(text),
        TextFont {
            font_size: 13.,
            ..default()
        },
        TextColor(Color::srgb(0.40, 0.77, 0.78)),
    ));
}

pub fn setup(
    mut c: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    assets: Res<AssetServer>,
    menu: Res<Menu>,
    mut f: ResMut<Flight>,
) {
    if menu.open {
        f.paused = true;
    }
    c.spawn((
        MenuCamera,
        Camera3d::default(),
        Camera {
            order: 10,
            is_active: menu.open,
            clear_color: ClearColorConfig::Custom(ink()),
            ..default()
        },
        RenderLayers::layer(LAYER),
        Transform::from_xyz(29., 15., 32.).looking_at(Vec3::new(0., 3., 0.), Vec3::Y),
        bevy::camera::Exposure { ev100: 9.5 },
        Msaa::Sample4,
    ));
    let floor = mats.add(StandardMaterial {
        base_color: Color::srgb(0.045, 0.065, 0.08),
        metallic: 0.5,
        perceptual_roughness: 0.35,
        ..default()
    });
    let wall = mats.add(StandardMaterial {
        base_color: Color::srgb(0.035, 0.05, 0.075),
        metallic: 0.35,
        perceptual_roughness: 0.55,
        ..default()
    });
    let glow = mats.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.65, 0.68),
        emissive: LinearRgba::new(4., 10., 11., 1.),
        ..default()
    });
    let box_mesh = meshes.add(Cuboid::default());
    for (pos, scale, mat) in [
        (
            Vec3::new(0., -0.4, 0.),
            Vec3::new(100., 0.5, 100.),
            floor.clone(),
        ),
        (
            Vec3::new(0., 14., -23.),
            Vec3::new(80., 28., 0.5),
            wall.clone(),
        ),
        (
            Vec3::new(-29., 14., 0.),
            Vec3::new(0.5, 28., 80.),
            wall.clone(),
        ),
    ] {
        c.spawn((
            HangarProp,
            Mesh3d(box_mesh.clone()),
            MeshMaterial3d(mat),
            Transform::from_translation(pos).with_scale(scale),
            RenderLayers::layer(LAYER),
        ));
    }
    for x in -3..=3 {
        for (pos, scale) in [
            (
                Vec3::new(x as f32 * 9., 13., -22.6),
                Vec3::new(0.08, 24., 0.08),
            ),
            (Vec3::new(x as f32 * 9., 0., 0.), Vec3::new(0.06, 0.02, 50.)),
        ] {
            c.spawn((
                HangarProp,
                Mesh3d(box_mesh.clone()),
                MeshMaterial3d(glow.clone()),
                Transform::from_translation(pos).with_scale(scale),
                RenderLayers::layer(LAYER),
            ));
        }
    }
    c.spawn((
        HangarProp,
        Mesh3d(meshes.add(Cylinder::new(11., 0.25))),
        MeshMaterial3d(floor),
        Transform::from_xyz(8., 0., 0.),
        RenderLayers::layer(LAYER),
    ));
    for (at, color, power) in [
        (Vec3::new(8., 18., 5.), Color::srgb(0.80, 0.91, 1.), 5e6),
        (Vec3::new(-8., 9., -13.), Color::srgb(0.25, 0.70, 1.), 3e6),
        (Vec3::new(20., 10., -15.), Color::srgb(1., 0.76, 0.44), 3e6),
    ] {
        c.spawn((
            PointLight {
                color,
                intensity: power,
                range: 90.,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_translation(at),
            RenderLayers::layer(LAYER),
        ));
    }
    for (radius, tex, pos, earth) in [
        (10., "earth.jpg", Vec3::new(-6., -1., -18.), true),
        (2.4, "moon.jpg", Vec3::new(5., 11., -24.), false),
    ] {
        let mat = mats.add(StandardMaterial {
            base_color_texture: Some(assets.load(tex)),
            perceptual_roughness: 0.9,
            ..default()
        });
        let e = c
            .spawn((
                SpaceProp,
                Mesh3d(meshes.add(Sphere::new(radius).mesh().uv(96, 64))),
                MeshMaterial3d(mat),
                Transform::from_translation(pos),
                RenderLayers::layer(LAYER),
            ))
            .id();
        if earth {
            c.entity(e).insert(EarthProp);
        }
    }
    let star = meshes.add(Sphere::new(0.045).mesh().ico(1).unwrap());
    let star_mat = mats.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.8, 1.),
        unlit: true,
        ..default()
    });
    for i in 0..90 {
        let a = i as f32 * 2.39996;
        let y = ((i * 37) % 91) as f32 * 0.55 - 8.;
        c.spawn((
            SpaceProp,
            Mesh3d(star.clone()),
            MeshMaterial3d(star_mat.clone()),
            Transform::from_xyz(a.sin() * 42., y, -35. - a.cos() * 12.),
            RenderLayers::layer(LAYER),
        ));
    }
}

pub fn controls(
    mut menu: ResMut<Menu>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    buttons: Query<(&Interaction, &Action), Changed<Interaction>>,
    mut f: ResMut<Flight>,
    mut view: ResMut<ViewState>,
    mut combat: ResMut<crate::combat::Combat>,
    mut bvr: ResMut<crate::combat::bvr::Menu>,
    mut hangar: ResMut<crate::hangar::Hangar>,
    mut audio: ResMut<crate::sound::Settings>,
    mut quit: MessageWriter<AppExit>,
    scroll: Res<bevy::input::mouse::AccumulatedMouseScroll>,
    mut panels: Query<&mut ScrollPosition, With<Root>>,
) {
    let esc = keys.just_pressed(KeyCode::Escape);
    if !menu.open {
        if esc && !hangar.open && !bvr.open {
            menu.saved_pause = f.paused;
            menu.aircraft = f.aircraft;
            menu.space = f.vehicle == Vehicle::Spacecraft;
            menu.scenario = if (5..=7).contains(&view.scenario) {
                5
            } else {
                view.scenario
            };
            menu.fbw = f.assist;
            menu.infinite = f.infinite_fuel;
            menu.external = f.systems.external_stores;
            menu.formation = bvr.formation;
            menu.defense = bvr.defense;
            menu.difficulty = match view.scenario {
                5 => Difficulty::Training,
                6 => Difficulty::Balanced,
                7 => Difficulty::Hard,
                _ => bvr.difficulty,
            };
            f.paused = true;
            menu.open = true;
            menu.launched = true;
            menu.page = Page::Pause;
            menu.dirty = true;
            combat.stop_gun();
            keys.clear_just_pressed(KeyCode::Escape);
        }
        return;
    }
    for mut pos in &mut panels {
        pos.y = (pos.y - scroll.delta.y * 26.).max(0.);
    }
    let mut actions = buttons
        .iter()
        .filter(|(i, _)| **i == Interaction::Pressed)
        .map(|(_, a)| a.0)
        .collect::<Vec<_>>();
    if esc {
        actions.push(902);
    }
    if keys.just_pressed(KeyCode::Enter) && menu.page == Page::Setup {
        actions.push(900);
    }
    let mut leave = false;
    for id in actions {
        menu.dirty = true;
        crate::sound::emit(
            &mut combat.audio_events,
            crate::sound::Kind::Switch,
            f.position,
            f.velocity,
            true,
        );
        match id {
            1 | 2 => {
                menu.space = id == 2;
                menu.scenario = if menu.space { 3 } else { 1 };
            }
            10..=14 => menu.aircraft = Aircraft::ALL[(id - 10) as usize],
            100..=110 => menu.scenario = (id - 100) as u8,
            200..=202 => {
                menu.difficulty = match id {
                    200 => Difficulty::Training,
                    201 => Difficulty::Balanced,
                    _ => Difficulty::Hard,
                }
            }
            210..=213 => {
                menu.formation = (id - 210) as usize;
                menu.scenario = 8;
            }
            220 | 221 => menu.defense = id == 221,
            230 => menu.fbw = !menu.fbw,
            231 => menu.infinite = !menu.infinite,
            232 => menu.external = !menu.external,
            300 => view.high_quality = !view.high_quality,
            301 => audio.menu_volume(-0.1),
            302 => audio.menu_volume(0.1),
            303 => audio.menu_mute(),
            900 if menu.page == Page::Setup => {
                f.aircraft = menu.aircraft;
                crate::reset(&mut f, menu.scenario_id(), &mut view);
                f.assist = menu.fbw;
                f.infinite_fuel = menu.infinite;
                if f.aircraft == Aircraft::F22 {
                    f.systems.external_stores = menu.external;
                }
                bvr.formation = menu.formation;
                bvr.difficulty = menu.difficulty;
                bvr.defense = menu.defense;
                bvr.open = false;
                bvr.saved_pause = None;
                hangar.open = false;
                hangar.block_input = false;
                f.paused = false;
                menu.launched = true;
                menu.open = false;
                leave = true;
            }
            901 => {
                menu.page = Page::Setup;
                hangar.open = false;
                bvr.open = false;
            }
            902 => {
                if matches!(menu.page, Page::Settings | Page::Controls) {
                    menu.page = menu.back;
                } else if menu.launched {
                    menu.open = false;
                    f.paused = menu.saved_pause;
                    leave = true;
                }
            }
            903 => {
                menu.back = menu.page;
                menu.page = Page::Settings;
            }
            904 => {
                menu.back = menu.page;
                menu.page = Page::Controls;
            }
            905 => {
                quit.write(AppExit::Success);
            }
            _ => {}
        }
    }
    if menu.open {
        f.paused = true;
    }
    if leave {
        keys.reset_all();
        mouse.reset_all();
    }
}

pub fn ui(
    mut c: Commands,
    mut menu: ResMut<Menu>,
    view: Res<ViewState>,
    audio: Res<crate::sound::Settings>,
    camera: Query<Entity, With<MenuCamera>>,
    roots: Query<Entity, With<Root>>,
    assets: Res<AssetServer>,
    mut hover: Query<(&Interaction, &mut BorderColor), With<Action>>,
) {
    for (i, mut border) in &mut hover {
        border.set_if_neq(BorderColor::all(if *i == Interaction::Hovered {
            Color::srgb(0.7, 0.95, 0.86)
        } else {
            Color::srgb(0.20, 0.40, 0.45)
        }));
    }
    if !menu.dirty {
        return;
    }
    menu.dirty = false;
    for e in &roots {
        c.entity(e).despawn();
    }
    if !menu.open {
        return;
    }
    let Ok(camera) = camera.single() else {
        return;
    };
    c.spawn((Root,ScrollPosition::default(),UiTargetCamera(camera),GlobalZIndex(500),Node {position_type:PositionType::Absolute,
        left:px(30),top:px(25),bottom:px(25),width:px(460),max_width:percent(48),padding:UiRect::all(px(22)),
        flex_direction:FlexDirection::Column,row_gap:px(8),overflow:Overflow::scroll_y(),border_radius:BorderRadius::all(px(12)),..default()},
        BackgroundColor(Color::srgba(0.009,0.019,0.033,0.95))))
    .with_children(|p| {
        row(p,|p| {
            p.spawn((ImageNode::new(assets.load("branding/nord-flight-icon.png")),Node {width:px(76),height:px(76),border_radius:BorderRadius::all(px(10)),..default()}));
            p.spawn(Node {flex_direction:FlexDirection::Column,justify_content:JustifyContent::Center,..default()}).with_children(|p| {
                label(p,"N O R D  /  F L I G H T",23.,Color::WHITE);
                label(p,"ATMOSPHERE  •  ORBIT  •  BEYOND",11.,Color::srgb(0.48,0.75,0.78));
            });
        });
        if menu.page==Page::Pause {
            heading(p,"FLIGHT PAUSED");
            label(p,"Your current flight is held. Resume it, or configure a new mission from the main menu.",17.,Color::srgb(0.78,0.87,0.91));
            button(p,902,"RESUME FLIGHT",true); button(p,901,"MAIN MENU",false);
        } else if menu.page==Page::Controls {
            heading(p,"FLIGHT CONTROLS");
            label(p,"W / S     Pitch\nQ / E     Roll\nA / D     Yaw\nShift     Full throttle\nCtrl       Cut throttle\nZ / X     Adjust throttle\nG          Landing gear\nB          Wheel brakes\nT          Flight-by-wire\nY          F-35B STOVL conversion\nC          Flight camera\nMouse drag   Look / orbit\nHome      Recenter cockpit\nM          Tactical / orbital map\nTab       Next target\nO          Missile type\nSpace    Launch missile\nU          Cannon\nV          Flares and chaff\n, / .      Decrease / increase warp\n/           Normal time\nP          Pause\nEsc        Pause menu",16.,Color::srgb(0.82,0.90,0.93));
            heading(p,"SPACECRAFT");
            label(p,"Use the in-flight SAS panel for attitude targets and the orbital map for maneuver planning. The warp panel edits rates and simulation timestep.",16.,Color::srgb(0.66,0.80,0.84));
            button(p,902,"BACK",false);
        } else if menu.page==Page::Settings {
            heading(p,"PRESENTATION");
            button(p,300,if view.high_quality {"GRAPHICS: HIGH"}else{"GRAPHICS: STANDARD"},view.high_quality);
            heading(p,&format!("MASTER AUDIO  /  {}%",(audio.menu_level()*100.).round()));
            row(p,|p| {button(p,301,"− VOLUME",false);button(p,302,"+ VOLUME",false);button(p,303,if audio.menu_muted(){"UNMUTE"}else{"MUTE"},audio.menu_muted());});
            label(p,"Audio is saved locally. Graphics uses the existing two presets. In flight, Warp Settings configures timestep and warp rates without changing simulation fidelity here.",15.,Color::srgb(0.64,0.78,0.82));
            button(p,902,"BACK",false);
        } else {
            heading(p,"01 / CHOOSE YOUR DOMAIN");
            row(p,|p| {button(p,1,"ATMOSPHERIC FLIGHT",!menu.space);button(p,2,"SPACEFLIGHT",menu.space);});
            heading(p,"02 / SELECT & CONFIGURE VEHICLE");
            if menu.space {
                button(p,20,"LUNAR EXPEDITION / CAPSULE + LANDER",true);
                label(p,"Two-stage direct-return lunar vehicle. More spacecraft will appear here as they are added.",14.,Color::srgb(0.65,0.80,0.84));
            } else {
                row(p,|p| {for (i,a) in Aircraft::ALL.into_iter().enumerate() {button(p,10+i as u16,a.name(),a==menu.aircraft);}});
                let s=menu.aircraft.spec();
                label(p,format!("{:.1} m span  /  {:.1} m length  /  {} engines",s.span,s.length,s.engines),14.,Color::srgb(0.66,0.80,0.84));
                row(p,|p| {button(p,230,if menu.fbw{"FBW ON"}else{"FBW OFF"},menu.fbw);
                    if menu.aircraft==Aircraft::F22 {button(p,232,if menu.external{"EXTERNAL STORES"}else{"CLEAN / INTERNAL"},menu.external);}});
            }
            button(p,231,if menu.infinite{"FUEL: UNLIMITED"}else{"FUEL: FINITE"},menu.infinite);
            heading(p,"03 / SELECT STARTING SCENARIO");
            if menu.space {
                for (id,name) in [(3,"EARTH ORBIT / LANDER ATTACHED"),(9,"LUNAR ORBIT"),(10,"LUNAR LANDING APPROACH")] {button(p,100+id,name,menu.scenario==id as u8);}
            } else {
                row(p,|p| {for (id,name) in [(0,"COLD & DARK"),(1,"TAKEOFF"),(2,"APPROACH")] {button(p,100+id,name,menu.scenario==id as u8);}});
                heading(p,"COMBAT / TARGET PRACTICE");
                row(p,|p| {button(p,104,"TARGET PRACTICE",menu.scenario==4);button(p,105,"AIR COMBAT",menu.scenario==5);});
                heading(p,"BEYOND VISUAL RANGE");
                row(p,|p| {for (i,name) in ["1 vs 2","2 vs 2","2 vs 4","4 vs 8"].into_iter().enumerate() {button(p,210+i as u16,name,menu.scenario==8&&menu.formation==i);}});
                if menu.scenario>=5 {
                    row(p,|p| {for (i,d) in [Difficulty::Training,Difficulty::Balanced,Difficulty::Hard].into_iter().enumerate(){button(p,200+i as u16,d.label(),menu.difficulty==d);}});
                }
                if menu.scenario==8 {row(p,|p| {button(p,220,"ELIMINATION",!menu.defense);button(p,221,"INTERCEPT / DEFEND",menu.defense);});}
            }
            button(p,900,"F L Y",true);
            label(p,"Enter to fly  /  launching starts a fresh scenario",12.,Color::srgb(0.48,0.68,0.73));
            if menu.launched {button(p,902,"RESUME PREVIOUS FLIGHT",false);}
        }
        row(p,|p| {button(p,903,"SETTINGS",false);button(p,904,"CONTROLS",false);button(p,905,"QUIT",false);});
    });
}

/// Snapshot render transforms under a temporary preview Flight, then restore
/// the authoritative resource before any simulation system can run.
pub fn presentation(world: &mut World) {
    let (open, space, aircraft, refresh) = {
        let m = world.resource::<Menu>();
        (
            m.open,
            m.space,
            m.aircraft,
            m.preview != Some((m.space, m.aircraft, m.external)),
        )
    };
    let time = world.resource::<Time>().elapsed_secs();
    let mut cameras = world.query::<(
        &mut Camera,
        Option<&MenuCamera>,
        Option<&crate::FlightCamera>,
        &mut Transform,
    )>();
    for (mut camera, menu, flight, mut t) in cameras.iter_mut(world) {
        if menu.is_some() {
            if camera.is_active!=open {camera.is_active = open;}
            if open {*t = Transform::from_xyz(
                26. + (time * 0.07).sin() * 3.,
                12.,
                27. + (time * 0.07).cos() * 2.,
            )
            .looking_at(Vec3::new(0., 3., 0.), Vec3::Y);}
        }
        if flight.is_some() {
            if camera.is_active==open {camera.is_active = !open;}
        }
    }
    if !open {
        return;
    }
    let mut props = world.query_filtered::<(
        &mut Visibility,
        Option<&HangarProp>,
        Option<&SpaceProp>,
        Option<&EarthProp>,
        &mut Transform,
    ),Or<(With<HangarProp>,With<SpaceProp>,With<EarthProp>)>>();
    for (mut v, hangar, stars, earth, mut t) in props.iter_mut(world) {
        if hangar.is_some() {
            v.set_if_neq(if space {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            });
        }
        if stars.is_some() {
            v.set_if_neq(if space {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            });
        }
        if earth.is_some() {
            t.rotation = Quat::from_rotation_y(time * 0.025);
        }
    }
    if refresh {
        let old = world
            .query_filtered::<Entity, With<PreviewVehicle>>()
            .iter(world)
            .collect::<Vec<_>>();
        for e in old {
            world.entity_mut(e).despawn();
        }
        let saved = world.resource::<Flight>().clone();
        let mut preview = if space {
            Flight::spacecraft()
        } else {
            Flight::runway()
        };
        if !space {
            crate::aircraft::select(&mut preview, aircraft);
        }
        preview.systems.external_stores = world.resource::<Menu>().external;
        preview.engine = false;
        preview.spool = 0.;
        preview.thrust = 0.;
        let standing_height = crate::collision::wheel_clearance(&preview) as f32 + 0.14;
        let saved_view = world.remove_resource::<ViewState>().expect("flight view");
        world.insert_resource(ViewState::default());
        world.insert_resource(preview);
        let results = [
            world.run_system_cached(crate::aircraft::visual::update),
            world.run_system_cached(crate::aircraft::visual::raptor_detail::update),
            world.run_system_cached(crate::gear::update),
            world.run_system_cached(crate::lunar::visuals),
        ];
        world.insert_resource(saved);
        world.insert_resource(saved_view);
        for r in results {
            r.expect("preview pose system");
        }
        let source = if space {
            world
                .query_filtered::<Entity, With<crate::world::Spacecraft>>()
                .iter(world)
                .next()
        } else if aircraft == Aircraft::Gripen {
            world
                .query_filtered::<Entity, With<crate::world::Jet>>()
                .iter(world)
                .next()
        } else {
            world
                .query::<(Entity, &crate::aircraft::visual::Root)>()
                .iter(world)
                .find(|(_, r)| r.0 == aircraft)
                .map(|(e, _)| e)
        };
        if let Some(source) = source {
            let root = world
                .spawn((
                    PreviewVehicle,
                    if space {
                        Transform::from_xyz(15., 4., 0.).with_scale(Vec3::splat(0.65))
                    } else {
                        Transform::from_xyz(8., standing_height, 0.)
                    },
                    Visibility::Inherited,
                ))
                .id();
            copy_children(world, source, root);
            let external = world.resource::<Menu>().external;
            world.resource_mut::<Menu>().preview = Some((space, aircraft, external));
        }
    }
    let mut models = world.query_filtered::<&mut Transform, With<PreviewVehicle>>();
    for mut t in models.iter_mut(world) {
        t.rotation = Quat::from_rotation_y(if space {
            -0.3 + (time * 0.04).sin() * 0.6
        } else {
            std::f32::consts::PI - 0.3 + (time * 0.04).sin() * 0.6
        });
    }
}
fn copy_children(world: &mut World, source: Entity, parent: Entity) {
    let children = world
        .get::<Children>(source)
        .map(|c| c.iter().collect::<Vec<_>>())
        .unwrap_or_default();
    for e in children {
        if world.get::<Visibility>(e) == Some(&Visibility::Hidden) {
            continue;
        }
        let transform = world.get::<Transform>(e).copied().unwrap_or_default();
        let mesh = world.get::<Mesh3d>(e).cloned();
        let mat = world.get::<MeshMaterial3d<StandardMaterial>>(e).cloned();
        let clone = world
            .spawn((
                transform,
                Visibility::Inherited,
                RenderLayers::layer(LAYER),
                ChildOf(parent),
            ))
            .id();
        if let (Some(mesh), Some(mat)) = (mesh, mat) {
            world.entity_mut(clone).insert((mesh, mat));
        }
        copy_children(world, e, clone);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Menu>()
            .init_resource::<ViewState>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<bevy::input::mouse::AccumulatedMouseScroll>()
            .init_resource::<crate::combat::Combat>()
            .init_resource::<crate::combat::bvr::Menu>()
            .init_resource::<crate::hangar::Hangar>()
            .init_resource::<crate::sound::Settings>()
            .insert_resource(Flight::approach())
            .add_message::<AppExit>()
            .add_systems(Update, controls);
        app
    }
    fn click(app: &mut App, id: u16) {
        let entity = app
            .world_mut()
            .spawn((Interaction::Pressed, Action(id)))
            .id();
        app.update();
        app.world_mut().entity_mut(entity).despawn();
    }
    #[test]
    fn pause_setup_and_resume_preserve_flight_and_prior_pause_state() {
        for paused in [false, true] {
            let mut app = app();
            app.world_mut().resource_mut::<Flight>().paused = paused;
            let before = app.world().resource::<Flight>().clone();
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::Escape);
            app.update();
            assert!(app.world().resource::<Menu>().open);
            assert!(app.world().resource::<Flight>().paused);
            click(&mut app, 901);
            assert!(app.world().resource::<Menu>().page == Page::Setup);
            click(&mut app, 902);
            assert!(!app.world().resource::<Menu>().open);
            let after = app.world().resource::<Flight>();
            assert_eq!(after.paused, paused);
            assert_eq!(
                (
                    after.position,
                    after.velocity,
                    after.orientation,
                    after.elapsed,
                    after.fuel,
                    after.damage.rng
                ),
                (
                    before.position,
                    before.velocity,
                    before.orientation,
                    before.elapsed,
                    before.fuel,
                    before.damage.rng
                )
            );
        }
    }
    #[test]
    fn launch_uses_selected_aircraft_bvr_and_space_configuration() {
        let mut app = app();
        {
            let mut m = app.world_mut().resource_mut::<Menu>();
            m.open = true;
            m.scenario = 8;
            m.aircraft = Aircraft::Hornet;
            m.formation = 3;
            m.difficulty = Difficulty::Hard;
            m.defense = true;
        }
        click(&mut app, 900);
        assert_eq!(app.world().resource::<Flight>().aircraft, Aircraft::Hornet);
        assert_eq!(app.world().resource::<ViewState>().scenario, 8);
        let bvr = app.world().resource::<crate::combat::bvr::Menu>();
        assert_eq!(
            (bvr.formation, bvr.difficulty, bvr.defense),
            (3, Difficulty::Hard, true)
        );
        assert!(!app.world().resource::<Flight>().paused);
        {
            let mut m = app.world_mut().resource_mut::<Menu>();
            m.open = true;
            m.space = true;
            m.scenario = 9;
        }
        click(&mut app, 900);
        assert_eq!(
            app.world().resource::<Flight>().vehicle,
            Vehicle::Spacecraft
        );
        assert_eq!(app.world().resource::<ViewState>().scenario, 9);
    }
    #[test]
    fn difficulty_changes_only_the_close_combat_scenario_choice() {
        let mut m = Menu::default();
        m.scenario = 5;
        for (d, id) in [
            (Difficulty::Training, 5),
            (Difficulty::Balanced, 6),
            (Difficulty::Hard, 7),
        ] {
            m.difficulty = d;
            assert_eq!(m.scenario_id(), id);
        }
        for id in [0, 1, 2, 3, 4, 8, 9, 10] {
            m.scenario = id;
            assert_eq!(m.scenario_id(), id);
        }
    }
}
