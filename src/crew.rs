//! Dog pilots: presentation-only state, reusable articulated geometry and a
//! live cockpit portrait. First fitted airframe: F-22. No flight RNG is used.
mod animation;
mod conscious;
mod dream;
pub(crate) mod layout;
mod loads;
mod model;
mod render;
mod sculpt;
mod ui;
mod vision;
use crate::{FlightCamera, ViewState, aircraft::Aircraft, combat::Combat, physics::*};
use animation::{Emotion, Pose, Track};
use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    prelude::*,
    render::render_resource::TextureFormat,
};
use std::collections::HashMap;
pub const DOG_LAYER: usize = 6;
pub const PORTRAIT_LAYER: usize = 7;
const CREW: usize = 12;
const STUDIO: Vec3 = Vec3::new(0., -2000., 0.);
pub const BREEDS: [&str; 10] = [
    "BEAGLE",
    "SHIBA",
    "HUSKY",
    "CORGI",
    "LABRADOR",
    "SHEPHERD",
    "TERRIER",
    "DACHSHUND",
    "BULLDOG",
    "MUTT",
];
pub const NAMES: [&str; 10] = [
    "BISCUIT", "MISO", "NOVA", "PIPPIN", "SUNNY", "REX", "RIVET", "OTTO", "TANK", "SCOUT",
];
pub const TRAITS: [&str; 10] = [
    "EARNEST",
    "COOL ACE",
    "THRILL SEEKER",
    "EAGER",
    "OPTIMIST",
    "VETERAN",
    "HIGH STRUNG",
    "BRAVE LITTLE PILOT",
    "UNIMPRESSED",
    "CURIOUS",
];
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Appearance {
    pub breed: usize,
    pub coat: usize,
    pub marking: usize,
    pub equipment: usize,
    pub suit: usize,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            breed: 7,
            coat: 0,
            marking: 0,
            equipment: 0,
            suit: 0,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Preview {
    #[default]
    Live,
    Calm,
    Positive,
    Negative,
    Sideways,
    Explosion,
    Warning,
    Happy,
    Lost,
    Emotion(Emotion),
    Load(loads::Preset),
    Faint(conscious::Preview),
}
impl Preview {
    fn label(self) -> &'static str {
        match self {
            Self::Emotion(e) => e.label(),
            Self::Load(p) => p.label(),
            Self::Faint(p) => p.label(),
            Self::Live => "LIVE",
            Self::Calm => "CALM",
            Self::Positive => "+9 G",
            Self::Negative => "−2 G",
            Self::Sideways => "SIDE G",
            Self::Explosion => "BOOM",
            Self::Warning => "WARNING",
            Self::Happy => "LANDED",
            Self::Lost => "CREW LOST",
        }
    }
}
#[derive(Resource)]
pub struct Crew {
    pub selected: usize,
    pub appearances: [Appearance; 10],
    pub editor: bool,
    pub editor_scroll: f32,
    pub chatter: bool,
    pub control_loss: bool,
    pub cockpit_g_effects: bool,
    pub panel: Vec2,
    pub size: Vec2,
    pub preview: Preview,
    pub dragging: bool,
    pub resizing: bool,
    pub pointer: Option<Vec2>,
    pub dirty: bool,
}
impl Default for Crew {
    fn default() -> Self {
        Self {
            selected: 7,
            appearances: std::array::from_fn(|breed| Appearance {
                breed,
                suit: if breed == 7 {
                    0
                } else if matches!(breed, 1 | 5 | 9) {
                    2
                } else {
                    1
                },
                ..default()
            }),
            editor: false,
            editor_scroll: 0.,
            chatter: true,
            control_loss: false,
            cockpit_g_effects: true,
            panel: Vec2::new(-1., -1.),
            size: Vec2::new(244., 284.),
            preview: Preview::Live,
            dragging: false,
            resizing: false,
            pointer: None,
            dirty: true,
        }
    }
}
impl Crew {
    fn path() -> std::path::PathBuf {
        std::env::var_os("NORD_FLIGHT_CREW_SETTINGS")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::env::var_os("HOME")
                    .or_else(|| std::env::var_os("USERPROFILE"))
                    .map(std::path::PathBuf::from)
                    .unwrap_or_default()
                    .join(".nord-flight/crew.json")
            })
    }
    fn from_json(v: &serde_json::Value) -> Self {
        let mut c = Self::default();
        c.selected = v["selected"].as_u64().unwrap_or(7).min(9) as usize;
        c.chatter = v["chatter"].as_bool().unwrap_or(true);
        c.control_loss = v["control_loss"].as_bool().unwrap_or(false);
        c.cockpit_g_effects = v["cockpit_g_effects"].as_bool().unwrap_or(true);
        if let Some(a) = v["appearances"].as_array() {
            for (i, a) in a.iter().take(10).enumerate() {
                let p = &mut c.appearances[i];
                p.coat = a["coat"].as_u64().unwrap_or(0).min(6) as usize;
                p.marking = a["marking"].as_u64().unwrap_or(0).min(2) as usize;
                p.equipment = a["equipment"].as_u64().unwrap_or(0).min(2) as usize;
                p.suit = a["suit"].as_u64().unwrap_or(p.suit as u64).min(2) as usize;
            }
        }
        if let (Some(x), Some(y)) = (v["x"].as_f64(), v["y"].as_f64()) {
            if x.is_finite() && y.is_finite() {
                c.panel = Vec2::new(x as f32, y as f32);
            }
        }
        if let (Some(w), Some(h)) = (v["width"].as_f64(), v["height"].as_f64()) {
            c.size =
                Vec2::new(w as f32, h as f32).clamp(Vec2::new(190., 220.), Vec2::new(480., 520.));
        }
        c
    }
    fn load() -> Self {
        let mut crew = std::fs::read_to_string(Self::path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .map(|v| Self::from_json(&v))
            .unwrap_or_default();
        if let Ok(mode) = std::env::var("NORD_CREW_CAPTURE") {
            crew = Self::default();
            crew.editor = mode != "exterior" && mode != "flight";
            crew.preview = match mode.as_str() {
                "positive" => Preview::Positive,
                "negative" => Preview::Negative,
                "side" => Preview::Sideways,
                "boom" => Preview::Explosion,
                "lost" => Preview::Lost,
                _ => conscious::Preview::parse(mode.strip_prefix("vision-").unwrap_or(&mode))
                    .map(Preview::Faint)
                    .or_else(|| loads::Preset::parse(&mode).map(Preview::Load))
                    .or_else(|| Emotion::parse(&mode).map(Preview::Emotion))
                    .unwrap_or(Preview::Calm),
            };
            if mode.starts_with("vision-") || mode == "faint-vision-gallery" {
                crew.editor = false;
            }
            if matches!(crew.preview, Preview::Faint(_)) {
                crew.editor_scroll = 600.;
            }
            if matches!(crew.preview, Preview::Load(_)) {
                crew.editor_scroll = 310.;
            }
            if let Ok(breed) = std::env::var("NORD_CREW_BREED") {
                crew.selected = breed.parse::<usize>().unwrap_or(7).min(9);
            }
            if let Ok(equipment) = std::env::var("NORD_CREW_EQUIPMENT") {
                crew.appearances[crew.selected].equipment =
                    equipment.parse::<usize>().unwrap_or(0).min(2);
            }
        }
        crew
    }
    pub(crate) fn set_control_loss(&mut self, enabled: bool) {
        self.control_loss = enabled;
        self.save();
    }
    pub(crate) fn set_cockpit_g_effects(&mut self, enabled: bool) {
        self.cockpit_g_effects = enabled;
        self.save();
    }
    fn save(&self) {
        let path = Self::path();
        if let Some(p) = path.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        let value = serde_json::json!({"version":1,"selected":self.selected,"chatter":self.chatter,"control_loss":self.control_loss,"cockpit_g_effects":self.cockpit_g_effects,"x":self.panel.x,"y":self.panel.y,"width":self.size.x,"height":self.size.y,"appearances":self.appearances.iter().map(|a|serde_json::json!({"coat":a.coat,"marking":a.marking,"equipment":a.equipment,"suit":a.suit})).collect::<Vec<_>>()});
        let _ = std::fs::write(path, serde_json::to_string_pretty(&value).unwrap());
    }
}
#[derive(Clone, Copy, Default, Debug)]
pub struct Face {
    pub strain: f32,
    pub negative: f32,
    pub lateral: f32,
    pub surprise: f32,
    pub worry: f32,
    pub joy: f32,
    pub lost: bool,
}
fn expression(g: f64, side: f64, alarm: f32, joy: f32, breed: usize, lost: bool) -> Face {
    let sensitivity = match breed {
        1 | 5 | 8 => 0.78,
        2 => 0.85,
        6 => 1.2,
        _ => 1.,
    };
    Face {
        strain: (((g - 2.) / 6.5) as f32 * sensitivity).clamp(0., 1.),
        negative: (((-g - 0.1) / 2.5) as f32).clamp(0., 1.),
        lateral: (side as f32 / 3.).clamp(-1., 1.),
        surprise: alarm.clamp(0., 1.),
        worry: (alarm * 0.7).clamp(0., 1.),
        joy: joy.clamp(0., 1.),
        lost,
    }
}
#[derive(Clone, Copy)]
struct Reaction {
    face: Face,
    feeling: Emotion,
    load: loads::Load,
    conscious: conscious::State,
    event: Emotion,
    event_until: f64,
    warning: bool,
    alarm: f32,
    joy: f32,
    previous_integrity: f64,
    previous_engine: bool,
    previous_landed: bool,
    initialized: bool,
    last_flash: u64,
    look: f32,
    lost: bool,
}
impl Default for Reaction {
    fn default() -> Self {
        Self {
            face: default(),
            feeling: default(),
            load: default(),
            conscious: default(),
            event: default(),
            event_until: 0.,
            warning: false,
            alarm: 0.,
            joy: 0.,
            previous_integrity: 1.,
            previous_engine: true,
            previous_landed: false,
            initialized: false,
            last_flash: 0,
            look: 0.,
            lost: false,
        }
    }
}
#[derive(Resource)]
struct Reactions {
    states: [Reaction; CREW],
    generation: u64,
    elapsed: f64,
    chatter_at: f32,
    speaking_until: f32,
    speaker: usize,
}
impl Default for Reactions {
    fn default() -> Self {
        Self {
            states: [Reaction::default(); CREW],
            generation: u64::MAX,
            elapsed: 0.,
            chatter_at: 0.,
            speaking_until: 0.,
            speaker: 0,
        }
    }
}
#[derive(Resource)]
struct Performances {
    emotions: [Track; CREW + 1],
    load_tracks: [loads::Track; CREW + 1],
    generation: u64,
    preview: Preview,
    preview_time: f32,
    demo: conscious::Demo,
    preview_state: conscious::State,
}
impl Default for Performances {
    fn default() -> Self {
        Self {
            emotions: [Track::default(); CREW + 1],
            load_tracks: [loads::Track::default(); CREW + 1],
            generation: u64::MAX,
            preview: Preview::Live,
            preview_time: 0.,
            demo: default(),
            preview_state: default(),
        }
    }
}
#[derive(Resource, Default)]
struct Library {
    models: HashMap<Appearance, Vec<model::Piece>>,
}
#[derive(Component)]
struct Exterior {
    owner: usize,
    seat:usize,
    appearance: Option<Appearance>,
}
#[derive(Component)]
struct PortraitDog {
    appearance: Option<Appearance>,
    owner: usize,
}
#[derive(Component)]
struct PortraitCamera;
#[derive(Resource)]
struct PortraitImage(Handle<Image>);
#[derive(Resource)]
struct StageRoot(Entity);

pub struct CrewPlugin;
impl Plugin for CrewPlugin {
    fn build(&self, app: &mut App) {
        install(app);
    }
}
fn install(app: &mut App) {
    // Diagnostic baseline: omit the entire crew feature for matched A/B runs.
    if std::env::var_os("NORD_CREW_DISABLE").is_some() {
        return;
    }
    app.insert_resource(Crew::load())
        .init_resource::<Library>()
        .init_resource::<Reactions>()
        .init_resource::<Performances>()
        .init_resource::<vision::HeldInput>()
        .add_systems(PostStartup, (vision::setup, dream::setup).after(setup).after(stage_layers))
        .add_systems(
            Update,
            vision::control_loss
                .after(crate::atmospheric_render::exercise)
                .before(crate::step)
                .in_set(crate::FlightStage::Controls),
        )
        .add_systems(
            PostStartup,
            (setup, stage_layers, ui::setup)
                .chain()
                .after(crate::render_batch::build),
        )
        .add_systems(
            Update,
            ui::controls
                .before(crate::input)
                .before(crate::view_controls)
                .after(crate::start_screen::controls)
                .in_set(crate::FlightStage::Controls),
        )
        .add_systems(
            Update,
            capture
                .in_set(crate::FlightStage::Capture)
                .before(crate::capture_visual_check),
        )
        .add_systems(
            Update,
            capture_exterior
                .after(crate::update_camera)
                .in_set(crate::FlightStage::Interface),
        )
        .add_systems(
            Update,
            (
                gallery,
                reactions,
                sync,
                animate,
                portrait_framing,
                vision::draw,
                ui::draw,
                dream::draw,
                ui::avoid_portrait,
                render::resize_target,
            )
                .chain()
                .after(crate::sound::update)
                .after(crate::aircraft::visual::update)
                .after(crate::combat::render)
                .in_set(crate::FlightStage::Interface),
        );
}
fn setup(
    mut c: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let image = images.add(Image::new_target_texture(
        640,
        640,
        TextureFormat::Rgba8Unorm,
        Some(TextureFormat::Rgba8UnormSrgb),
    ));
    c.insert_resource(PortraitImage(image.clone()));
    let root = c
        .spawn((Transform::from_translation(STUDIO), Visibility::Inherited))
        .id();
    // Reuse the actual F-22 seat, tub, console and rear bulkhead builder.
    crate::cockpit::raptor_interior::setup(&mut c, &mut meshes, &mut mats, root);
    c.insert_resource(StageRoot(root));
    let dog = c
        .spawn((
            PortraitDog {
                appearance: None,
                owner: 0,
            },
            Transform::default(),
            Visibility::Inherited,
        ))
        .id();
    c.entity(root).add_child(dog);
    c.spawn((
        PortraitCamera,
        Camera3d::default(),
        bevy::pbr::generate::SkipEnvironmentMapGeneration,
        Camera {
            order: -2,
            is_active: false,
            clear_color: ClearColorConfig::Custom(Color::srgb(0.023, 0.037, 0.048)),
            ..default()
        },
        RenderTarget::Image(image.into()),
        RenderLayers::layer(PORTRAIT_LAYER),
        Msaa::Sample4,
        Projection::Perspective(PerspectiveProjection {
            fov: 35_f32.to_radians(),
            near: 0.025,
            far: 5.,
            ..default()
        }),
        bevy::camera::Exposure { ev100: 9.0 },
        bevy::light::AmbientLight {
            color: Color::srgb(1., 0.91, 0.82),
            brightness: 180.,
            ..default()
        },
        Transform::from_translation(STUDIO + Vec3::new(0.09, 1.015, -3.54))
            .looking_at(STUDIO + Vec3::new(0., 0.965, -2.50), Vec3::Y),
    ));
    c.spawn((
        DirectionalLight {
            illuminance: 7200.,
            color: Color::srgb(1., 0.95, 0.87),
            shadows_enabled: true,
            shadow_depth_bias: 0.02,
            shadow_normal_bias: 1.8,
            ..default()
        },
        Transform::from_xyz(-2., 3., -4.).looking_at(Vec3::ZERO, Vec3::Y),
        RenderLayers::layer(PORTRAIT_LAYER),
        bevy::light::CascadeShadowConfigBuilder {
            first_cascade_far_bound: 3.,
            maximum_distance: 3.,
            num_cascades: 1,
            ..default()
        }
        .build(),
    ));
    c.spawn((
        DirectionalLight {
            illuminance: 2800.,
            color: Color::srgb(0.80, 0.89, 1.),
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(3., 1.5, -2.).looking_at(Vec3::ZERO, Vec3::Y),
        RenderLayers::layer(PORTRAIT_LAYER),
    ));
    for (p, color, intensity) in [
        (
            Vec3::new(-0.65, 1.6, -3.35),
            Color::srgb(1., 0.94, 0.83),
            5000.,
        ),
        (
            Vec3::new(0.6, 1.1, -3.1),
            Color::srgb(0.61, 0.77, 1.),
            3000.,
        ),
        (
            Vec3::new(0., 1.55, -1.7),
            Color::srgb(0.65, 0.77, 1.),
            1800.,
        ),
    ] {
        c.spawn((
            PointLight {
                color,
                intensity,
                range: 4.,
                radius: 0.3,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_translation(STUDIO + p),
            RenderLayers::layer(PORTRAIT_LAYER),
        ));
    }
    for owner in 0..CREW {for seat in 0..2 {
        c.spawn((
            Exterior {
                owner,
                seat,
                appearance: None,
            },
            Transform::default(),
            Visibility::Hidden,
        ));
    }
    }
    // Layer separation removes the player's head from the first-person view,
    // while the exterior cameras still see the complete seated pilot.
}
fn stage_layers(world: &mut World) {
    let root = world.resource::<StageRoot>().0;
    fn visit(world: &mut World, e: Entity) {
        world
            .entity_mut(e)
            .insert(RenderLayers::layer(PORTRAIT_LAYER));
        let children = world
            .get::<Children>(e)
            .map(|c| c.iter().collect::<Vec<_>>())
            .unwrap_or_default();
        for child in children {
            visit(world, child);
        }
    }
    visit(world, root);
    crate::render_batch::build(world);
    let ids = world
        .query_filtered::<Entity, With<FlightCamera>>()
        .iter(world)
        .collect::<Vec<_>>();
    for e in ids {
        world
            .entity_mut(e)
            .insert(RenderLayers::from_layers(&[0, DOG_LAYER]));
    }
    let ids = world
        .query_filtered::<Entity, With<crate::world::Sun>>()
        .iter(world)
        .collect::<Vec<_>>();
    for e in ids {
        world.entity_mut(e).insert(RenderLayers::from_layers(&[
            0,
            crate::terrain::SHADOW_LAYER,
            DOG_LAYER,
        ]));
    }
}
fn owner(view: &ViewState, crew: &Crew) -> usize {
    if crew.editor {
        0
    } else {
        view.aircraft_follow.map_or(0, |i| (i + 1).min(CREW - 1))
    }
}
fn appearance(crew: &Crew, id: usize) -> Appearance {
    if id == 0 {
        crew.appearances[crew.selected]
    } else {
        let breed = ((id - 1) * 7 + 3) % 10;
        Appearance {
            breed,
            suit: 1,
            equipment: 0,
            ..default()
        }
    }
}
fn flight_for<'a>(id: usize, f: &'a Flight, combat: &'a Combat) -> Option<&'a Flight> {
    if id == 0 {
        Some(f)
    } else {
        combat
            .targets
            .get(id - 1)
            .filter(|_| combat.active)
            .map(|t| &t.dynamics)
    }
}
fn supported(f: &Flight) -> bool {
    f.vehicle == Vehicle::Gripen && matches!(f.aircraft, Aircraft::F22 | Aircraft::Hornet)
}
fn reactions(
    time: Res<Time>,
    f: Res<Flight>,
    v: Res<ViewState>,
    combat: Res<Combat>,
    menu: Res<crate::start_screen::Menu>,
    crew: Res<Crew>,
    bridge: Res<crate::sound::Bridge>,
    mut states: ResMut<Reactions>,
) {
    let reset = states.generation != v.generation || f.elapsed < states.elapsed;
    if reset {
        states.states = [Reaction::default(); CREW];
        states.generation = v.generation;
    }
    let dt = if f.paused {
        0.
    } else {
        (f.elapsed - states.elapsed).clamp(0., 0.1) as f32
    };
    states.elapsed = f.elapsed;
    let watched = owner(&v, &crew);
    let mut speak = false;
    for id in 0..CREW {
        let Some(plant) = flight_for(id, &f, &combat) else {
            continue;
        };
        let a = appearance(&crew, id);
        let r = &mut states.states[id];
        let position = if id == 0 {
            f.position
        } else {
            combat.targets[id - 1].position
        };
        let dead = if id == 0 {
            f.crashed && !f.ejection.active
        } else {
            !combat.targets[id - 1].alive
        };
        let running =
            plant.engine && plant.damage.engine > 0.12 && (plant.fuel > 0. || plant.infinite_fuel);
        let mut event = 0_f32;
        if r.initialized && !f.paused {
            if r.previous_integrity - plant.integrity > 0.012 {
                event = 0.75;
                r.event = Emotion::Hurt;
                r.event_until = plant.elapsed + 1.8;
            }
            if r.previous_engine && !running && !plant.landed {
                event = event.max(0.72);
                r.event = Emotion::EngineConcern;
                r.event_until = plant.elapsed + 3.;
            }
            if !r.previous_landed && plant.landed {
                if plant.touchdown_sink > 3.5 {
                    event = event.max(0.85);
                    r.event = Emotion::Dazed;
                    r.event_until = plant.elapsed + 3.5;
                } else {
                    r.joy = 1.;
                    r.event = Emotion::Celebrating;
                    r.event_until = plant.elapsed + 3.5;
                }
            }
            for p in &combat.fx.particles {
                if p.id <= r.last_flash || !matches!(p.kind, crate::combat::effects::Kind::Flash) {
                    continue;
                }
                let delta = p.position - position;
                let range = delta.length();
                if range < 1500. {
                    let visible = delta.normalize_or_zero().dot(plant.forward()) > 0.15
                        && crate::combat::guidance::terrain_visible(position, p.position);
                    event = event.max(
                        ((1. - range / 1500.) as f32 * if visible { 1.2 } else { 0.65 })
                            .clamp(0., 1.),
                    );
                    if event > 0.2 && (r.event != Emotion::Hurt || plant.elapsed >= r.event_until) {
                        r.event = Emotion::Startled;
                        r.event_until = plant.elapsed + 0.65;
                    }
                    r.look = (plant.orientation.conjugate() * delta.normalize_or_zero())
                        .x
                        .clamp(-1., 1.) as f32;
                }
            }
        }
        r.last_flash = combat
            .fx
            .particles
            .iter()
            .map(|p| p.id)
            .max()
            .unwrap_or(r.last_flash);
        if !f.paused {
            r.alarm = (r.alarm - dt * 0.65).max(event);
            r.joy = (r.joy - dt * 0.18).max(0.);
            r.look *= (-dt * 0.8).exp();
        }
        let warning = if id == 0 {
            combat.threats(&f).0 > 0
        } else {
            combat
                .missiles_for_draw()
                .any(|m| m.alive && m.target == id - 1 && (m.position - position).length() < 5000.)
        };
        if r.warning && !warning {
            r.event = Emotion::Relieved;
            r.event_until = plant.elapsed + 2.8;
        }
        r.warning = warning;
        let worry = if warning {
            0.8
        } else if plant.stall() && !plant.landed {
            0.55
        } else if !running && !plant.landed {
            0.35
        } else {
            0.
        };
        // Catastrophic airframe destruction is the first milestone's restrained
        // crew-loss rule. Resetting/starting a mission restores the crew.
        r.lost |= dead;
        r.face = expression(
            plant.g_load,
            plant.lateral_g,
            r.alarm,
            r.joy,
            a.breed,
            r.lost,
        );
        r.face.worry = r.face.worry.max(worry);
        r.load = loads::Load::new(
            plant.g_load,
            plant.lateral_g,
            plant.longitudinal_g,
            !plant.landed,
        );
        let previous_phase = r.conscious.phase;
        if r.lost {
            r.conscious = default();
        } else {
            r.conscious.update(r.load, dt);
        }
        if id == watched
            && previous_phase != r.conscious.phase
            && r.conscious.phase == conscious::Phase::Recovering
        {
            speak = true;
        }
        r.feeling = if plant.elapsed < r.event_until {
            r.event
        } else if warning {
            Emotion::MissileAlarm
        } else if plant.stall() && !plant.landed {
            Emotion::StallWorry
        } else if !running && !plant.landed {
            Emotion::EngineConcern
        } else if !plant.landed
            && plant.throttle > 0.95
            && plant.airspeed < 160.
            && (0.5..=2.).contains(&plant.g_load)
            && plant.longitudinal_g >= 0.
        {
            // Keep the existing takeoff excitement; inertial motion still layers on top.
            Emotion::Excited
        } else if let Some(emotion) = r.load.emotion() {
            emotion
        } else if r.alarm > 0.3 {
            Emotion::Afraid
        } else if r.joy > 0.35 {
            Emotion::Proud
        } else if plant.integrity < 0.55 {
            Emotion::Nervous
        } else if plant.landed {
            match (plant.elapsed as usize / 12 + id) % 5 {
                2 => Emotion::Curious,
                4 if !running => Emotion::Sleepy,
                _ => Emotion::Content,
            }
        } else if plant.agl() < 100. && plant.airspeed > 120. {
            Emotion::Focused
        } else {
            match (plant.elapsed as usize / 15 + id) % 6 {
                2 => Emotion::Curious,
                4 => Emotion::Focused,
                _ => Emotion::Content,
            }
        };

        if id == watched && event > 0.35 {
            speak = true;
        }
        r.previous_integrity = plant.integrity;
        r.previous_engine = running;
        r.previous_landed = plant.landed;
        r.initialized = true;
    }
    let active = supported(flight_for(watched, &f, &combat).unwrap_or(&f));
    if crew.chatter
        && active
        && !menu.open
        && !f.paused
        && v.missile_follow.is_none()
        && time.elapsed_secs() > states.chatter_at
    {
        let face = states.states[watched].face;
        if !face.lost
            && states.states[watched].conscious.responsive()
            && (speak
                || face.worry > 0.4
                || face.strain > 0.5
                || face.negative > 0.4
                || face.lateral.abs() > 0.55
                || states.states[watched].load.longitudinal().abs() > 0.6
                || face.joy > 0.8)
        {
            crate::sound::crew_chatter(
                &bridge,
                0.12 + face.surprise * 0.08,
                0.85 + appearance(&crew, watched).breed as f32 * 0.045,
            );
            states.chatter_at = time.elapsed_secs() + 4.5;
            states.speaking_until = time.elapsed_secs() + 0.75;
            states.speaker = watched;
        }
    }
}
fn sync(
    mut c: Commands,
    crew: Res<Crew>,
    f: Res<Flight>,
    v: Res<ViewState>,
    combat: Res<Combat>,
    menu: Res<crate::start_screen::Menu>,
    mut library: ResMut<Library>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut exterior: Query<
        (Entity, &mut Exterior, &mut Transform, &mut Visibility),
        (Without<PortraitDog>, Without<FlightCamera>),
    >,
    mut portrait: Query<(Entity, &mut PortraitDog), Without<Exterior>>,
    dogs: Query<(Entity, &ChildOf), With<model::DogRoot>>,
    mut cameras: Query<&mut Camera, With<PortraitCamera>>,
    mut main_layers: Query<&mut RenderLayers, With<FlightCamera>>,
    main_pose: Query<&Transform, With<FlightCamera>>,
) {
    let watched = owner(&v, &crew);
    let active = crew.editor
        || (!menu.open
            && v.missile_follow.is_none()
            && flight_for(watched, &f, &combat).is_some_and(supported));
    for mut camera in &mut cameras {
        let enabled = active && std::env::var_os("NORD_CREW_NO_PORTRAIT").is_none();
        if camera.is_active != enabled {
            camera.is_active = enabled;
        }
    }
    for mut layer in &mut main_layers {
        layer.set_if_neq(if v.index == 3 && !v.following() && !v.map && !f.ejection.active {
            RenderLayers::layer(0)
        } else {
            RenderLayers::from_layers(&[0, DOG_LAYER])
        });
    }
    for (e, mut tag, mut t, mut visible) in &mut exterior {
        let plant = flight_for(tag.owner, &f, &combat);
        let alive = tag.owner == 0 || combat.targets.get(tag.owner - 1).is_some_and(|t| t.alive);
        let position = if tag.owner == 0 && f.ejection.active {f.ejection.pilots[tag.seat].position} else if tag.owner == 0 {
            f.position
        } else {
            combat
                .targets
                .get(tag.owner - 1)
                .map_or(f.position, |t| t.position)
        };
        let near = main_pose.iter().next().is_some_and(|camera| {
            camera
                .translation
                .distance(crate::world::local(position, &f))
                < 180.
        });
        let show = !menu.open && alive && near && plant.is_some_and(|p| supported(p) && (!p.crashed||p.ejection.active) && (tag.seat==0||p.aircraft==Aircraft::Hornet));
        visible.set_if_neq(if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if !show {
            continue;
        }
        let plant = plant.unwrap();
        let (p, q) = if tag.owner == 0 {
            (f.position, f.orientation)
        } else {
            let t = &combat.targets[tag.owner - 1];
            (t.position, t.orientation)
        };
        let mut next = *t;
        let visual_offset=if plant.aircraft==Aircraft::Hornet { Vec3::new(0.,0.06+tag.seat as f32*0.12,-0.35+tag.seat as f32*crate::cockpit::hornet_interior::SPACING) } else { Vec3::ZERO };
        next.translation = crate::world::local(p, &f)
            + crate::world::local_rot(q, &f) * (plant.aircraft.cabin_offset()+visual_offset);
        next.rotation = crate::world::local_rot(q, &f);
        if tag.owner==0 && f.ejection.active && f.ejection.age>=crate::ejection::delay(tag.seat){next.rotation=crate::ejection::crew_attitude(&f,tag.seat);next.translation=crate::world::local(f.ejection.pilots[tag.seat].position,&f)-next.rotation*Vec3::new(0.,0.34,-2.40);}
        t.set_if_neq(next);
        let mut a = appearance(&crew, tag.owner);if tag.seat==1{a.breed=(a.breed+3)%10;}
        if tag.appearance != Some(a) {
            for (dog, parent) in &dogs {
                if parent.parent() == e {
                    c.entity(dog).despawn();
                }
            }
            let pieces = library
                .models
                .entry(a)
                .or_insert_with(|| model::build(&mut meshes, &mut mats, &mut images, &a));
            model::spawn_layer(&mut c,e,pieces,a.breed,tag.owner,false,if tag.seat==1{0}else{DOG_LAYER});
            tag.appearance = Some(a);
        }
    }
    if active {
        for (e, mut tag) in &mut portrait {
            let a = appearance(&crew, watched);
            if tag.appearance != Some(a) || tag.owner != watched {
                for (dog, parent) in &dogs {
                    if parent.parent() == e {
                        c.entity(dog).despawn();
                    }
                }
                let pieces = library
                    .models
                    .entry(a)
                    .or_insert_with(|| model::build(&mut meshes, &mut mats, &mut images, &a));
                model::spawn(&mut c, e, pieces, a.breed, watched, true);
                tag.appearance = Some(a);
                tag.owner = watched;
            }
        }
    }
    // Wardrobe exploration must not retain every old high-detail mesh forever.
    if library.models.len() > 16 {
        let used: Vec<_> = (0..CREW).map(|id| appearance(&crew, id)).collect();
        library.models.retain(|a, _| used.contains(a));
    }
}
fn preview_face(p: Preview) -> Face {
    match p {
        Preview::Positive => expression(9., 0., 0., 0., 7, false),
        Preview::Negative => expression(-2., 0., 0., 0., 7, false),
        Preview::Sideways => expression(1., 3., 0., 0., 7, false),
        Preview::Explosion => expression(1., 0., 1., 0., 7, false),
        Preview::Warning => Face {
            worry: 1.,
            ..default()
        },
        Preview::Happy => Face {
            joy: 1.,
            ..default()
        },
        Preview::Lost => Face {
            lost: true,
            ..default()
        },
        _ => Face::default(),
    }
}
fn preview_emotion(p: Preview) -> Emotion {
    match p {
        Preview::Faint(_) => Emotion::Content,
        Preview::Load(p) => p.load().emotion().unwrap_or(Emotion::Content),
        Preview::Emotion(e) => e,
        Preview::Positive => Emotion::PositiveG,
        Preview::Negative => Emotion::NegativeG,
        Preview::Sideways => Emotion::SideG,
        Preview::Explosion => Emotion::Startled,
        Preview::Warning => Emotion::MissileAlarm,
        Preview::Happy => Emotion::Celebrating,
        _ => Emotion::Content,
    }
}
fn preview_load(p: Preview) -> loads::Load {
    match p {
        Preview::Faint(p) => p.load(),
        Preview::Load(p) => p.load(),
        Preview::Positive | Preview::Emotion(Emotion::PositiveG | Emotion::Bracing) => {
            loads::Preset::PositiveExtreme.load()
        }
        Preview::Negative | Preview::Emotion(Emotion::NegativeG) => {
            loads::Preset::NegativeStrong.load()
        }
        Preview::Sideways | Preview::Emotion(Emotion::SideG) => loads::Preset::RightStrong.load(),
        _ => loads::Load::default(),
    }
}
fn demo_capture() -> bool {
    std::env::var("NORD_CREW_CAPTURE")
        .is_ok_and(|m| m.starts_with("vision-") || m == "faint-vision-gallery")
}
fn preview_active(crew: &Crew) -> bool {
    (crew.editor || demo_capture()) && crew.preview != Preview::Live
}
fn animate(
    time: Res<Time>,
    f:Option<Res<Flight>>,
    crew: Res<Crew>,
    states: Res<Reactions>,
    mut tracks: ResMut<Performances>,
    mut bones: Query<(&model::Bone, &mut Transform)>,
) {
    use model::Joint::*;
    let now = time.elapsed_secs();
    let dt = time.delta_secs().min(0.1);
    let preview = preview_active(&crew);
    if tracks.generation != states.generation {
        tracks.emotions = [Track::default(); CREW + 1];
        tracks.load_tracks = [loads::Track::default(); CREW + 1];
        tracks.generation = states.generation;
    }
    if tracks.preview != crew.preview {
        tracks.preview = crew.preview;
        tracks.preview_time = 0.;
        tracks.demo = default();
    }
    tracks.preview_time += dt;
    if crew.preview == Preview::Faint(conscious::Preview::Sequence) {
        tracks.demo.update(dt);
        tracks.preview_state = tracks.demo.state;
    } else {
        tracks.preview_state = if let Preview::Faint(p) = crew.preview {
            p.snapshot(tracks.preview_time)
        } else {
            default()
        };
    }
    let mut poses = [Pose::default(); CREW + 1];
    let mut motions = [loads::Motion::default(); CREW + 1];
    let mut details = [conscious::Detail::default(); CREW + 1];
    for (id, p) in poses.iter_mut().enumerate() {
        let r = states.states[id.min(CREW - 1)];
        let is_preview = id == CREW;
        let emotion = if is_preview {
            preview_emotion(crew.preview)
        } else {
            r.feeling
        };
        let load = if is_preview {
            if crew.preview == Preview::Faint(conscious::Preview::Sequence) {
                tracks.demo.load()
            } else {
                preview_load(crew.preview)
            }
        } else {
            r.load
        };
        let state = if is_preview {
            tracks.preview_state
        } else {
            r.conscious
        };
        let face = if is_preview {
            preview_face(crew.preview)
        } else {
            r.face
        };
        *p = tracks.emotions[id].update(emotion, dt, (id as f32 * 0.71) % 3.);
        if matches!(
            emotion,
            Emotion::PositiveG | Emotion::NegativeG | Emotion::SideG | Emotion::Bracing
        ) || (emotion == Emotion::Focused && load.longitudinal().abs() > 0.06)
        {
            *p = Pose::default().mix(*p, 0.25 + 0.75 * load.strength());
        }
        motions[id] = tracks.load_tracks[id].update(load, dt);
        motions[id].apply(p, now);
        let floppy = model::floppy_ears(appearance(&crew, if is_preview { 0 } else { id }).breed);
        if floppy {
            motions[id].apply_floppy_ears(p, now);
        }
        if face.lost {
            *p = Pose::closed();
            motions[id] = default();
            continue;
        }
        if state.responsive() {
            p.brow += Vec2::splat(face.surprise * 0.006);
            p.jaw += face.surprise * 0.09 * (1. - face.strain * 0.4);
            if !is_preview {
                p.head.y += r.look * 0.12;
                p.gaze.x += r.look * 0.35;
            }
            let phase = (now + id as f32 * 0.71) % 4.7;
            let blink = (1. - ((phase - 0.11) / 0.11).abs()).clamp(0., 1.);
            p.lids = p.lids.max(Vec2::splat(blink));
            if now < states.speaking_until && id == states.speaker && !is_preview {
                p.jaw += (now * 32.).sin().abs() * 0.07;
            }
        }
        details[id] = state.apply(p, now, floppy);
        let passive = 1. - state.weight() * 0.65;
        motions[id].chest *= passive;
        motions[id].head_offset *= passive;
    }
    for (bone, mut t) in &mut bones {
        let index = if bone.portrait && preview {
            CREW
        } else {
            bone.owner
        };
        let p = poses[index];
        let motion = motions[index];
        let detail = details[index];
        let mut out = bone.rest;
        match bone.joint {
            Body => {}
            Chest => {
                out.translation.y += p.breath;
                out.rotation =
                    Quat::from_euler(EulerRot::XYZ, p.chest + motion.chest.x, 0., motion.chest.z);
            }
            Head => {
                out.translation += motion.head_offset + detail.head_offset;
                out.translation.y += p.head_y;
                out.rotation = Quat::from_euler(EulerRot::XYZ, p.head.x, p.head.y, p.head.z);
            }
            Jaw => out.rotation = Quat::from_rotation_x(-p.jaw.clamp(0., 0.34)),
            Tongue => {
                out.translation.z -= p.tongue * 0.025;
                out.rotation = Quat::from_rotation_x(p.tongue * 0.12);
            }
            EarL | EarR => {
                let e = if bone.joint == EarL { p.ear_l } else { p.ear_r };
                out.rotation = Quat::from_euler(EulerRot::XYZ, e.x, e.y, e.z);
            }
            EyeL | EyeR => {
                out.translation.x += p.gaze.x.clamp(-0.75, 0.75) * 0.019;
                out.translation.y += p.gaze.y.clamp(-0.65, 0.65) * 0.018;
            }
            LidL | LidR => {
                let shut = if bone.joint == LidL {
                    p.lids.x
                } else {
                    p.lids.y
                };
                out.rotation = Quat::from_rotation_x(1.02 * (1. - shut.clamp(0., 1.)));
            }
            LowerLidL | LowerLidR => {
                let shut = if bone.joint == LowerLidL {
                    p.lids.x
                } else {
                    p.lids.y
                };
                out.rotation = Quat::from_rotation_x(-0.52 * (1. - shut.clamp(0., 1.)));
            }
            BrowL | BrowR => {
                let (lift, angle) = if bone.joint == BrowL {
                    (p.brow.x, p.brow_angle.x)
                } else {
                    (p.brow.y, p.brow_angle.y)
                };
                out.translation.y += lift;
                out.rotation = Quat::from_rotation_z(angle);
            }
            ArmL | ArmR => {
                let side = if bone.joint == ArmL { -1. } else { 1. };
                let d = if bone.joint == ArmL {
                    detail.arm_l
                } else {
                    detail.arm_r
                };
                out.rotation = Quat::from_euler(
                    EulerRot::XYZ,
                    -p.brace * 0.06 + d.x,
                    side * p.brace * 0.035 + d.y,
                    side * p.brace * 0.04 + d.z,
                );
            }
            WristL | WristR => {
                out.rotation = Quat::from_rotation_x(if bone.joint == WristL {
                    detail.wrist.x
                } else {
                    detail.wrist.y
                });
            }
            FingersL | FingersR => {
                out.rotation = Quat::from_rotation_x(if bone.joint == FingersL {
                    detail.fingers.x
                } else {
                    detail.fingers.y
                });
            }
        }
        if bone.owner==0 && !bone.portrait && f.as_ref().is_some_and(|f|f.ejection.active) {
            let hanging=f.as_ref().unwrap().ejection.age>2.5;
            match bone.joint {
                Chest=>{out.rotation=Quat::from_rotation_x(if hanging{-0.08}else{0.});},
                Head=>{out.rotation=Quat::IDENTITY;},
                ArmL|ArmR=>{let side=if bone.joint==ArmL{-1.}else{1.};out.rotation=Quat::from_euler(EulerRot::XYZ,if hanging{1.3}else{0.2},0.,side*0.18);},
                WristL|WristR|FingersL|FingersR=>{out.rotation=Quat::from_rotation_x(0.25);},
                _=>{}
            }
        }
        t.set_if_neq(out);
    }
}
/// Keep the complete ears in frame as negative G spreads them outward. The
/// portrait still renders through the same single cached camera/target.
fn portrait_framing(
    time: Res<Time>,
    crew: Res<Crew>,
    states: Res<Reactions>,
    view: Res<ViewState>,
    tracks: Res<Performances>,
    mut cameras: Query<&mut Transform, With<PortraitCamera>>,
) {
    let r = states.states[owner(&view, &crew)];
    let load = if preview_active(&crew) {
        if crew.preview == Preview::Faint(conscious::Preview::Sequence) {
            tracks.demo.load()
        } else {
            preview_load(crew.preview)
        }
    } else {
        r.load
    };
    let state = if preview_active(&crew) && matches!(crew.preview, Preview::Faint(_)) {
        tracks.preview_state
    } else {
        r.conscious
    };
    let extra = if r.face.lost && !crew.editor {
        0.
    } else {
        load.negative() * 0.26 + load.side().abs() * 0.04 + state.weight() * 0.35
    };
    let target = STUDIO + Vec3::new(0.09, 1.015, -3.54 - extra);
    for mut t in &mut cameras {
        let position = if t.translation.distance_squared(target) < 1e-8 {
            target
        } else {
            t.translation
                .lerp(target, 1. - (-time.delta_secs().min(0.1) / 0.10).exp())
        };
        t.set_if_neq(Transform::from_translation(position).looking_at(
            STUDIO + Vec3::new(0., 0.965 - state.weight() * 0.11, -2.50),
            Vec3::Y,
        ));
    }
}
/// Deterministic native-render gallery; never touches the player's saved wardrobe.
fn gallery(
    time: Res<Time>,
    mut crew: ResMut<Crew>,
    mut c: Commands,
    image: Res<PortraitImage>,
    mut step: Local<usize>,
) {
    let mode = std::env::var("NORD_CREW_CAPTURE").unwrap_or_default();
    if !matches!(
        mode.as_str(),
        "gallery"
            | "load-gallery"
            | "faint-gallery"
            | "faint-vision-gallery"
            | "faint-sequence-gallery"
    ) {
        return;
    }
    if mode == "faint-sequence-gallery" {
        let t = time.elapsed_secs() - 5.;
        if t < 0. {
            return;
        }
        let next = Preview::Faint(conscious::Preview::Sequence);
        if crew.preview != next {
            crew.preview = next;
            crew.dirty = true;
        }
        let index = ((t * 8.) as usize).min(159);
        if index + 1 <= *step {
            return;
        }
        *step = index + 1;
        let _ = std::fs::create_dir_all("tests/crew-passout/sequence");
        let path = format!("tests/crew-passout/sequence/frame-{index:03}.png");
        let last = index == 159;
        c.spawn(bevy::render::view::screenshot::Screenshot::image(
            image.0.clone(),
        ))
        .observe(
            move |event: On<bevy::render::view::screenshot::ScreenshotCaptured>| {
                event
                    .image
                    .clone()
                    .try_into_dynamic()
                    .expect("sequence image")
                    .save(&path)
                    .expect("save sequence");
                if last {
                    std::process::exit(0);
                }
            },
        );
        return;
    }
    let faint = mode.starts_with("faint-");
    let count = if faint {
        conscious::Preview::ALL.len() - 1
    } else if mode == "load-gallery" {
        loads::Preset::ALL.len()
    } else {
        Emotion::ALL.len()
    };
    let t = time.elapsed_secs() - 5.;
    if t < 0. {
        return;
    }
    let exhausted = t >= count as f32 * 1.8;
    let index = ((t / 1.8) as usize).min(count - 1);
    let (next, slug, folder) = if faint {
        let p = conscious::Preview::ALL[index];
        (
            Preview::Faint(p),
            p.slug().to_string(),
            "tests/crew-passout",
        )
    } else if mode == "load-gallery" {
        let p = loads::Preset::ALL[index];
        (Preview::Load(p), p.slug().to_string(), "tests/crew-loads")
    } else {
        let e = Emotion::ALL[index];
        (Preview::Emotion(e), e.slug(), "tests/crew-v2")
    };
    let _ = std::fs::create_dir_all(folder);
    if crew.preview != next {
        crew.preview = next;
        crew.dirty = true;
    }
    let frame = if exhausted || t % 1.8 > 1.25 {
        2
    } else if t % 1.8 > 0.50 {
        1
    } else {
        0
    };
    let target = index * 2 + frame;
    if frame == 0 || target <= *step {
        return;
    }
    *step = target;
    let path = format!("{folder}/{:02}-{slug}-{frame}.png", index + 1);
    let last = index + 1 == count && frame == 2;
    // Complete both image readbacks before stopping the diagnostic process.
    // Exiting directly also avoids blocking on renderer teardown with pending readbacks.
    let remaining = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(
        if mode == "faint-vision-gallery" { 2 } else { 1 },
    ));
    if mode == "faint-vision-gallery" {
        let scene_path = format!("{folder}/{:02}-{slug}-{frame}-cockpit.png", index + 1);
        let finish = remaining.clone();
        c.spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(
                move |event: On<bevy::render::view::screenshot::ScreenshotCaptured>| {
                    event
                        .image
                        .clone()
                        .try_into_dynamic()
                        .expect("cockpit screenshot")
                        .save(&scene_path)
                        .expect("save cockpit");
                    if last && finish.fetch_sub(1, std::sync::atomic::Ordering::SeqCst) == 1 {
                        std::process::exit(0);
                    }
                },
            );
    }
    c.spawn(bevy::render::view::screenshot::Screenshot::image(
        image.0.clone(),
    ))
    .observe(
        move |event: On<bevy::render::view::screenshot::ScreenshotCaptured>| {
            event
                .image
                .clone()
                .try_into_dynamic()
                .expect("portrait screenshot")
                .save(&path)
                .expect("save portrait");
            if last && remaining.fetch_sub(1, std::sync::atomic::Ordering::SeqCst) == 1 {
                std::process::exit(0);
            }
        },
    );
}
fn capture_exterior(f: Res<Flight>, mut cameras: Query<&mut Transform, With<FlightCamera>>) {
    if std::env::var("NORD_CREW_CAPTURE").as_deref() != Ok("exterior") {
        return;
    }
    let q = crate::world::local_rot(f.orientation, &f);
    let p = crate::world::local(f.position, &f) + q * f.aircraft.cabin_offset();
    for mut camera in &mut cameras {
        *camera = Transform::from_translation(p + q * Vec3::new(0.98, 1.33, -3.53))
            .looking_at(p + q * Vec3::new(0., 0.99, -2.47), q * Vec3::Y);
    }
}
fn capture(
    mut c: Commands,
    time: Res<Time>,
    image: Res<PortraitImage>,
    library: Res<Library>,
    meshes: Res<Assets<Mesh>>,
    mut captured: Local<bool>,
) {
    let Ok(mode) = std::env::var("NORD_CREW_CAPTURE") else {
        return;
    };
    if mode.contains("gallery") || *captured || time.elapsed_secs() < 6.5 {
        return;
    }
    *captured = true;
    println!(
        "crew capture: {} cached appearances, {} vertices",
        library.models.len(),
        library
            .models
            .values()
            .flatten()
            .filter_map(|p| meshes.get(&p.mesh))
            .map(|m| m.count_vertices())
            .sum::<usize>()
    );
    let path = format!(
        "tests/crew-{}-{}-{}.png",
        mode,
        std::env::var("NORD_CREW_BREED").unwrap_or_else(|_| "7".into()),
        std::env::var("NORD_CREW_EQUIPMENT").unwrap_or_else(|_| "0".into())
    );
    c.spawn(bevy::render::view::screenshot::Screenshot::image(
        image.0.clone(),
    ))
    .observe(
        move |event: On<bevy::render::view::screenshot::ScreenshotCaptured>| {
            event
                .image
                .clone()
                .try_into_dynamic()
                .expect("crew image")
                .save(&path)
                .expect("save crew portrait");
        },
    );
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dachshund_is_default_and_all_presets_remain_distinct() {
        let c = Crew::default();
        assert_eq!(c.selected, 7);
        for (i, a) in c.appearances.iter().enumerate() {
            assert_eq!(a.breed, i);
        }
        assert_eq!(c.appearances[7].equipment, 0);
    }
    #[test]
    fn signed_g_and_explosions_combine_without_losing_strain() {
        let quiet = expression(1., 0., 0., 0., 7, false);
        let loaded = expression(9., 2., 1., 0., 7, false);
        assert_eq!(quiet.strain, 0.);
        assert_eq!(loaded.strain, 1.);
        assert_eq!(loaded.surprise, 1.);
        assert!(loaded.lateral > 0.6);
        let negative = expression(-2., -3., 0., 0., 7, false);
        assert!(negative.negative > 0.7);
        assert_eq!(negative.strain, 0.);
        assert_eq!(negative.lateral, -1.);
    }
    #[test]
    fn live_passout_tracks_exposure_pause_recovery_and_mission_reset() {
        let mut app = App::new();
        let mut f = Flight::approach();
        f.aircraft = Aircraft::F22;
        f.paused = false;
        f.g_load = 9.;
        app.insert_resource(f)
            .init_resource::<ViewState>()
            .init_resource::<Combat>()
            .init_resource::<Crew>()
            .init_resource::<crate::start_screen::Menu>()
            .init_resource::<crate::sound::Bridge>()
            .init_resource::<Reactions>()
            .init_resource::<Time>()
            .add_systems(Update, reactions);
        app.update();
        let advance = |app: &mut App, g: f64, count: usize| {
            for _ in 0..count {
                let mut f = app.world_mut().resource_mut::<Flight>();
                f.g_load = g;
                f.elapsed += 0.1;
                drop(f);
                app.update();
            }
        };
        advance(&mut app, 9., 50);
        assert_eq!(
            app.world().resource::<Reactions>().states[0]
                .conscious
                .phase,
            conscious::Phase::Unconscious
        );
        let age = app.world().resource::<Reactions>().states[0].conscious.age;
        app.world_mut().resource_mut::<Flight>().paused = true;
        app.update();
        assert_eq!(
            app.world().resource::<Reactions>().states[0].conscious.age,
            age
        );
        app.world_mut().resource_mut::<Flight>().paused = false;
        advance(&mut app, 1., 140);
        assert_eq!(
            app.world().resource::<Reactions>().states[0]
                .conscious
                .phase,
            conscious::Phase::Alert
        );
        advance(&mut app, 9., 50);
        app.world_mut().resource_mut::<ViewState>().generation += 1;
        app.world_mut().resource_mut::<Flight>().g_load = 1.;
        app.update();
        assert_eq!(
            app.world().resource::<Reactions>().states[0]
                .conscious
                .phase,
            conscious::Phase::Alert
        );
        assert!(!Crew::from_json(&serde_json::json!({})).control_loss);
        assert!(Crew::from_json(&serde_json::json!({"control_loss":true})).control_loss);
        assert!(Crew::from_json(&serde_json::json!({})).cockpit_g_effects);
        let settings=Crew::from_json(&serde_json::json!({"control_loss":true,"cockpit_g_effects":false}));
        assert!(!settings.cockpit_g_effects && settings.control_loss);
    }
    #[test]
    fn live_reactions_follow_signed_loads_and_new_explosion_events() {
        let mut app = App::new();
        let mut f = Flight::runway();
        f.paused = false;
        app.insert_resource(f.clone())
            .init_resource::<ViewState>()
            .init_resource::<Combat>()
            .init_resource::<Crew>()
            .init_resource::<crate::start_screen::Menu>()
            .init_resource::<crate::sound::Bridge>()
            .init_resource::<Reactions>()
            .init_resource::<Time>()
            .add_systems(Update, reactions);
        app.update();
        {
            let mut f = app.world_mut().resource_mut::<Flight>();
            f.g_load = 9.;
            f.lateral_g = -3.;
            f.elapsed += 0.1;
        }
        app.world_mut().resource_mut::<Combat>().fx.particles.push(
            crate::combat::effects::Particle {
                position: f.position + f.forward() * 100.,
                velocity: bevy::math::DVec3::ZERO,
                age: 0.,
                life: 0.6,
                size: Vec3::splat(2.),
                kind: crate::combat::effects::Kind::Flash,
                id: 1,
            },
        );
        app.update();
        let r = app.world().resource::<Reactions>().states[0];
        assert_eq!(r.face.strain, 1.);
        assert_eq!(r.face.lateral, -1.);
        assert!(r.face.surprise > 0.5);
        app.world_mut().resource_mut::<Flight>().paused = true;
        app.update();
        assert_eq!(app.world().resource::<Reactions>().states[0].alarm, r.alarm);
        {
            let mut f = app.world_mut().resource_mut::<Flight>();
            f.paused = false;
            f.elapsed += 0.7;
            f.g_load = 1.;
            f.lateral_g = 0.;
        }
        app.update();
        assert_eq!(
            app.world().resource::<Reactions>().states[0].feeling,
            Emotion::Afraid
        );
        {
            let mut f = app.world_mut().resource_mut::<Flight>();
            f.paused = false;
            f.crashed = true;
            f.elapsed += 0.1;
        }
        app.update();
        assert!(app.world().resource::<Reactions>().states[0].face.lost);
        app.world_mut().resource_mut::<ViewState>().generation += 1;
        app.world_mut().resource_mut::<Flight>().crashed = false;
        app.update();
        assert!(!app.world().resource::<Reactions>().states[0].face.lost);
        {let mut f=app.world_mut().resource_mut::<Flight>();f.ejection.active=true;f.crashed=true;f.elapsed+=0.1;}
        app.update();assert!(!app.world().resource::<Reactions>().states[0].face.lost,"escaped aircrew must not inherit aircraft loss");
    }
    #[test]
    fn live_events_select_damage_landing_failure_and_signed_load_performances() {
        let mut app = App::new();
        let mut f = Flight::runway();
        f.paused = false;
        f.landed = false;
        f.engine = true;
        f.airspeed = 200.;
        app.insert_resource(f)
            .init_resource::<ViewState>()
            .init_resource::<Combat>()
            .init_resource::<Crew>()
            .init_resource::<crate::start_screen::Menu>()
            .init_resource::<crate::sound::Bridge>()
            .init_resource::<Reactions>()
            .init_resource::<Time>()
            .add_systems(Update, reactions);
        app.update();
        for (g, side, forward, want) in [
            (5., 0., 0., Emotion::PositiveG),
            (-2., 0., 0., Emotion::NegativeG),
            (1., -2., 0., Emotion::SideG),
            (9., 0., 0., Emotion::Bracing),
            (1., 0., 2., Emotion::Focused),
            (1., 0., -2., Emotion::Bracing),
        ] {
            {
                let mut f = app.world_mut().resource_mut::<Flight>();
                f.elapsed += 0.1;
                f.g_load = g;
                f.lateral_g = side;
                f.longitudinal_g = forward;
            }
            app.update();
            assert_eq!(app.world().resource::<Reactions>().states[0].feeling, want);
        }
        {
            let mut f = app.world_mut().resource_mut::<Flight>();
            f.elapsed += 0.1;
            f.g_load = 1.;
            f.lateral_g = 0.;
            f.longitudinal_g = 0.5;
            f.throttle = 1.;
            f.airspeed = 100.;
        }
        app.update();
        assert_eq!(
            app.world().resource::<Reactions>().states[0].feeling,
            Emotion::Excited
        );
        assert_eq!(
            app.world().resource::<Reactions>().states[0].load.forward,
            0.5
        );
        {
            let mut f = app.world_mut().resource_mut::<Flight>();
            f.throttle = 0.25;
            f.airspeed = 200.;
            f.longitudinal_g = 0.;
        }
        {
            let mut f = app.world_mut().resource_mut::<Flight>();
            f.elapsed += 0.1;
            f.integrity -= 0.2;
        }
        app.update();
        assert_eq!(
            app.world().resource::<Reactions>().states[0].feeling,
            Emotion::Hurt
        );
        {
            let mut f = app.world_mut().resource_mut::<Flight>();
            f.elapsed += 2.;
            f.engine = false;
        }
        app.update();
        assert_eq!(
            app.world().resource::<Reactions>().states[0].feeling,
            Emotion::EngineConcern
        );
        {
            let mut f = app.world_mut().resource_mut::<Flight>();
            f.elapsed += 4.;
            f.landed = true;
            f.touchdown_sink = 5.;
        }
        app.update();
        assert_eq!(
            app.world().resource::<Reactions>().states[0].feeling,
            Emotion::Dazed
        );
        {
            let mut f = app.world_mut().resource_mut::<Flight>();
            f.elapsed += 4.;
            f.landed = false;
        }
        app.update();
        {
            let mut f = app.world_mut().resource_mut::<Flight>();
            f.elapsed += 0.1;
            f.landed = true;
            f.touchdown_sink = 1.;
        }
        app.update();
        assert_eq!(
            app.world().resource::<Reactions>().states[0].feeling,
            Emotion::Celebrating
        );
    }
    #[test]
    fn acceleration_previews_drive_the_actual_head_and_harness_rig_in_opposite_directions() {
        let mut app = App::new();
        let mut crew = Crew::default();
        crew.editor = true;
        crew.preview = Preview::Load(loads::Preset::ForwardStrong);
        app.insert_resource(crew)
            .init_resource::<Reactions>()
            .init_resource::<Performances>()
            .init_resource::<Time>()
            .add_systems(Update, animate);
        let head = app
            .world_mut()
            .spawn((
                model::Bone {
                    owner: 0,
                    portrait: true,
                    joint: model::Joint::Head,
                    rest: Transform::IDENTITY,
                },
                Transform::IDENTITY,
            ))
            .id();
        let chest = app
            .world_mut()
            .spawn((
                model::Bone {
                    owner: 0,
                    portrait: true,
                    joint: model::Joint::Chest,
                    rest: Transform::IDENTITY,
                },
                Transform::IDENTITY,
            ))
            .id();
        let advance = |app: &mut App| {
            for _ in 0..50 {
                app.world_mut()
                    .resource_mut::<Time>()
                    .advance_by(std::time::Duration::from_secs_f32(1. / 60.));
                app.update();
            }
        };
        advance(&mut app);
        assert!(app.world().get::<Transform>(head).unwrap().translation.z > 0.01);
        assert!(
            app.world()
                .get::<Transform>(chest)
                .unwrap()
                .rotation
                .to_euler(EulerRot::XYZ)
                .0
                > 0.02
        );
        app.world_mut().resource_mut::<Crew>().preview =
            Preview::Load(loads::Preset::BackwardStrong);
        advance(&mut app);
        assert!(app.world().get::<Transform>(head).unwrap().translation.z < -0.01);
        assert!(
            app.world()
                .get::<Transform>(chest)
                .unwrap()
                .rotation
                .to_euler(EulerRot::XYZ)
                .0
                < -0.03
        );
        app.world_mut().resource_mut::<Crew>().preview = Preview::Load(loads::Preset::RightStrong);
        advance(&mut app);
        let right = app
            .world()
            .get::<Transform>(head)
            .unwrap()
            .rotation
            .to_euler(EulerRot::XYZ)
            .2;
        assert!(right > 0.1);
        app.world_mut().resource_mut::<Crew>().preview = Preview::Load(loads::Preset::LeftStrong);
        advance(&mut app);
        assert!(
            app.world()
                .get::<Transform>(head)
                .unwrap()
                .rotation
                .to_euler(EulerRot::XYZ)
                .2
                < -0.1
        );
    }
    #[test]
    fn portrait_blinks_on_wall_time_while_flight_is_paused() {
        let mut app = App::new();
        let mut f = Flight::runway();
        f.paused = true;
        app.insert_resource(f)
            .init_resource::<Crew>()
            .init_resource::<Reactions>()
            .init_resource::<Performances>()
            .init_resource::<Time>()
            .add_systems(Update, animate);
        let rest = Transform::from_xyz(-0.1, 0.13, -0.18);
        let e = app
            .world_mut()
            .spawn((
                model::Bone {
                    owner: 0,
                    portrait: true,
                    joint: model::Joint::LidL,
                    rest,
                },
                rest,
            ))
            .id();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(2.));
        app.update();
        let open = app.world().get::<Transform>(e).unwrap().rotation;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(2.81));
        app.update();
        let closed = app.world().get::<Transform>(e).unwrap().rotation;
        assert!(open.angle_between(closed) > 0.8);
    }
    #[test]
    fn portrait_tracks_followed_aircraft_and_editor_returns_to_player() {
        let mut v = ViewState::default();
        let mut crew = Crew::default();
        assert_eq!(owner(&v, &crew), 0);
        v.follow_aircraft(2);
        assert_eq!(owner(&v, &crew), 3);
        crew.editor = true;
        assert_eq!(owner(&v, &crew), 0);
        crew.editor = false;
        v.restore_camera();
        assert_eq!(owner(&v, &crew), 0);
    }
    #[test]
    fn saved_settings_are_bounded_and_keep_dachshund_default() {
        let c = Crew::from_json(
            &serde_json::json!({"selected":100,"appearances":[{"coat":99,"equipment":99}],"width":10000,"height":1}),
        );
        assert_eq!(c.selected, 9);
        assert_eq!(c.appearances[0].coat, 6);
        assert_eq!(c.appearances[0].equipment, 2);
        assert_eq!(c.size, Vec2::new(480., 220.));
        assert_eq!(Crew::from_json(&serde_json::json!({})).selected, 7);
    }
}
