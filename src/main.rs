// Release builds launch as a normal Windows game; development builds keep console logs.
#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

mod aero_fx;
mod aircraft;
mod hangar;
mod start_screen;
mod atmospheric_render;
mod benchmark;
mod aerodynamics;
mod aircraft_detail;
mod airframe;
mod cockpit;
mod cockpit_controls;
mod cockpit_labels;
mod celestial;
mod collision;
mod combat;
mod control_surfaces;
mod display_upload;
mod earth;
mod earth_gpu;
mod terrain_ready;
mod mesh_reuse;
mod render_batch;
mod flight_study;
mod gear;
mod instruments;
mod lunar;
mod map;
mod navball;
mod optimization_study;
mod orbit;
mod performance;
mod physics;
mod plume_motion;
mod propulsion;
mod render_cache;
mod skin_damage;
mod sound;
mod space;
mod spacecraft_detail;
mod terrain;
mod warp;
mod world;
use bevy::{
    anti_alias::fxaa::Fxaa,
    camera::Exposure,
    core_pipeline::tonemapping::Tonemapping,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    light::{AtmosphereEnvironmentMapLight, CascadeShadowConfigBuilder, DirectionalLightShadowMap},
    math::DVec3,
    pbr::{Atmosphere, AtmosphereSettings, DistanceFog, FogFalloff, ScatteringMedium},
    post_process::bloom::Bloom,
    prelude::*,
    window::{PresentMode, WindowResolution},
};
use physics::*;
use world::*;

#[derive(Resource)]
struct ViewState {
    index: usize,
    yaw: f32,
    pitch: f32,
    zoom: f32,
    high_quality: bool,
    aero_debug: bool,
    collision_debug: bool,
    help: bool,
    simple: bool,
    cockpit_enemy_icons: bool,
    guide: bool,
    left_camera_drag: bool,
    benchmark_msaa: bool,
    map: bool,
    map_yaw: f32,
    map_pitch: f32,
    map_zoom: f32,
    map_focus_ship: bool,
    map_target: u8,
    flyby: DVec3,
    generation: u64,
    scenario: u8,
    missile_follow: Option<MissileFollow>,
    aircraft_follow: Option<usize>,
    aircraft_previous: Option<(usize, f32, f32, f32, bool)>,
}
#[derive(Clone, Copy)]
struct MissileFollow {
    missile: usize,
    previous: (usize, f32, f32, f32, bool),
}
impl Default for ViewState {
    fn default() -> Self {
        Self {
            index: 0,
            yaw: 0.0,
            pitch: 0.0,
            zoom: 1.0,
            high_quality: false,
            aero_debug: false,
            collision_debug: false,
            help: true,
            simple: false,
            cockpit_enemy_icons: false,
            guide: true,
            left_camera_drag: false,
            benchmark_msaa: false,
            map: false,
            map_yaw: 0.0,
            map_pitch: -0.45,
            map_zoom: 1.0,
            map_focus_ship: false,
            map_target: 0,
            flyby: DVec3::ZERO,
            generation: 0,
            scenario: 2,
            missile_follow: None,
            aircraft_follow: None,
            aircraft_previous: None,
        }
    }
}
impl ViewState {
    fn follow_missile(&mut self, missile: usize) {
        if self.aircraft_follow.is_some() {
            self.restore_camera();
        }
        if let Some(follow) = &mut self.missile_follow {
            follow.missile = missile;
        } else {
            self.missile_follow = Some(MissileFollow {
                missile,
                previous: (self.index, self.yaw, self.pitch, self.zoom, self.map),
            });
            self.index = 1;
            self.yaw = 0.35;
            self.pitch = 0.12;
            self.zoom = 1.0;
            self.map = false;
        }
    }
    fn restore_camera(&mut self) {
        if let Some(follow) = self.missile_follow.take() {
            (self.index, self.yaw, self.pitch, self.zoom, self.map) = follow.previous;
        }
        if let Some(previous) = self.aircraft_previous.take() {
            (self.index, self.yaw, self.pitch, self.zoom, self.map) = previous;
        }
        self.aircraft_follow = None;
    }
    fn follow_aircraft(&mut self, aircraft: usize) {
        if self.missile_follow.is_some() {
            self.restore_camera();
        }
        if self.aircraft_follow.is_none() {
            self.aircraft_previous = Some((self.index, self.yaw, self.pitch, self.zoom, self.map));
            self.index = 1;
            self.yaw = 0.35;
            self.pitch = 0.18;
            self.zoom = 1.;
            self.map = false;
        }
        self.aircraft_follow = Some(aircraft);
    }
    fn following(&self) -> bool {
        self.missile_follow.is_some() || self.aircraft_follow.is_some()
    }
    fn follow_identity(&self) -> Option<usize> {
        self.aircraft_follow
            .map(|id| 1000 + id)
            .or_else(|| self.missile_follow.map(|m| m.missile))
    }
    fn render_index(&self) -> usize {
        if self.map {
            8
        } else {
            [10, 0, 7, 1][self.index.min(3)]
        }
    }
    fn toggle_map(&mut self, f: &Flight) {
        let _ = f;
        let open = !self.map;
        self.restore_camera();
        self.map = open;
    }
    fn place_flyby(&mut self, f: &Flight) {
        self.flyby = f.position
            + f.velocity.normalize_or_zero() * 160.0
            + f.up() * 18.0
            + f.orientation * DVec3::X * 35.0;
    }
}
const VIEWS: [&str; 4] = ["LOCKED", "RADIAL UP", "FLY-PAST", "COCKPIT"];
#[derive(Component)]
struct FlightCamera;
#[derive(Component)]
struct Hud(u8);
#[derive(Component)]
pub struct Chrome;
#[derive(Component)]
struct SimplifyButton;
#[derive(Component)]
struct SimplifyLabel;

#[derive(Component)]
struct ScenarioButton(u8);
#[derive(Component)]
struct SunBody;
#[derive(Resource)]
struct SkyProfile(Atmosphere);
#[derive(Resource)]
struct UiFont(Handle<Font>);
#[derive(Resource, Default)]
struct SimClock {
    accumulator: f64,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum FlightStage { Controls, Simulation, World, Camera, Presentation, Interface, Capture }
fn main() {
    if std::env::args().any(|a|a=="--export-f22-nozzle"){aircraft::visual::raptor_detail::export_nozzle_preview();return;}
    if std::env::args().any(|a|a=="--study-f22"){aircraft::raptor::study();return;}
    if std::env::args().any(|a|a=="--study-f22-roll"){aircraft::raptor::roll_study();return;}
    if std::env::args().any(|a|a=="--study-fleet"){aircraft::study();return;}
    if std::env::args().any(|a| a == "--study-lunar") {
        space::study();
        return;
    }
    if std::env::args().any(|a| a == "--study-ai") {
        combat::ai::study();
        return;
    }
    if std::env::args().any(|a| a == "--study-bvr") {
        combat::bvr::study();
        return;
    }
    if std::env::args().any(|a| a == "--study-audio") {
        sound::study();
        return;
    }
    if std::env::args().any(|a| a == "--study-missiles") {
        combat::guidance::study();
        return;
    }
    if std::env::args().any(|a| a == "--study-air-combat") {
        combat::dogfight::study();
        return;
    }
    if std::env::args().any(|a| a == "--study-performance") {
        propulsion::study();
        return;
    }
    if std::env::args().any(|a| a == "--study-flight") {
        flight_study::run();
        return;
    }
    if std::env::args().any(|a| a == "--study-workloads") {
        optimization_study::run();
        return;
    }
    if std::env::args().any(|a| a == "--verify-flight") {
        verify_flight();
        return;
    }
    let preview = std::env::args()
        .skip_while(|a| a != "--visual-check")
        .nth(1).or_else(|| std::env::var("NORD_BENCH_SCENE").ok());
    let render_benchmark = std::env::args()
        .skip_while(|a| a != "--benchmark-render")
        .nth(1);
    let mut initial_flight = Flight::default();
    let mut initial_view = ViewState::default();
    let mut initial_environment = Environment::default();
    if let Some(mode) = &render_benchmark {
        initial_flight.paused = true;
        initial_view.help = false;
        initial_view.high_quality = mode.contains("quality");
        initial_view.benchmark_msaa = mode.contains("msaa");
        if mode.contains("cockpit") {
            initial_view.index = 3;
        }
        if mode.contains("live") {
            initial_flight.paused = false;
        }
        if mode.contains("orbit") {
            initial_flight = Flight::spacecraft();
            initial_flight.paused = true;
            initial_view.map = true;
        }
    }
    if let Some(mode) = &preview {
        assert!(
            [
                "dry",
                "full",
                "surfaces",
                "spacecraft",
                "cockpit",
                "cockpit-bank",
                "space-cockpit",
                "map",
                "gear",
                "far-space",
                "reentry",
                "transonic"
            ]
            .contains(&mode.as_str())
                || mode.starts_with("gear-")
                || mode.starts_with("engine-")
                || mode.starts_with("combat")
                || mode.starts_with("bvr")
                || mode.starts_with("air-")
                || mode.starts_with("terrain-")
                || mode.starts_with("earth-flight-")
                || mode.starts_with("lunar-")
                || mode.starts_with("capsule-"),
            // Lunar diagnostics use posed states with the ordinary renderer.
            // Combat capture modes use the same native renderer as ordinary flight.
            "visual-check mode: dry, full, surfaces, spacecraft"
        );
        if [
            "spacecraft",
            "space-cockpit",
            "map",
            "far-space",
            "engine-space",
        ]
        .contains(&mode.as_str())
        {
            initial_flight = Flight::spacecraft();
        }
        initial_flight.paused = true;
        initial_flight.throttle = if mode == "full"
            || mode == "spacecraft"
            || mode == "engine-jet-full"
            || mode == "engine-space"
        {
            1.0
        } else if mode == "space-cockpit" || mode == "map" {
            0.0
        } else {
            0.25
        };
        initial_flight.spool = initial_flight.throttle;
        let (temperature, pressure, density, sound) = atmosphere(initial_flight.altitude());
        initial_flight.temperature = temperature;
        initial_flight.pressure = pressure;
        initial_flight.density = density;
        initial_flight.airspeed = initial_flight.velocity.length();
        initial_flight.mach = initial_flight.airspeed / sound;
        initial_flight.thrust = if initial_flight.vehicle == Vehicle::Spacecraft {
            (180000.0 - pressure * 0.24).max(0.0) * initial_flight.spool
        } else {
            propulsion::jet_output(initial_flight.spool, density, initial_flight.mach).0
        };
        if mode == "surfaces" {
            initial_flight.surface = DVec3::new(0.6, -0.7, 0.6);
            initial_flight.flaps = 0.5;
            initial_flight.airbrake = true;
        }
        initial_view.index = 1;
        initial_view.yaw = 0.65;
        initial_view.zoom = 0.85;
        if mode.starts_with("engine-") {
            initial_view.zoom = 0.50;
            initial_view.yaw = 0.35;
            initial_view.pitch = 0.20;
            if mode == "engine-jet-off" {
                initial_flight.throttle = 0.0;
                initial_flight.spool = 0.0;
                initial_flight.thrust = 0.0;
                initial_flight.engine = false;
            }
        }
        if mode == "cockpit" || mode == "space-cockpit" || mode == "cockpit-bank" {
            initial_view.index = 3;
            initial_view.yaw = 0.0;
            initial_view.pitch = -0.10;
            initial_view.help = false;
            if mode == "cockpit-bank" {
                initial_flight.orientation *= bevy::math::DQuat::from_rotation_z(0.35);
            }
        }
        if mode == "map" {
            initial_view.map = true;
            initial_flight.throttle = 0.0;
            initial_flight.spool = 0.0;
            initial_flight.thrust = 0.0;
            initial_view.help = false;
        }
        if mode == "gear" || mode.starts_with("gear-") {
            initial_flight.gear_door = 1.0;
            initial_flight.gear_extension = 0.45;
            initial_flight.gear = false;
            initial_view.pitch = 0.9;
            initial_view.yaw = 1.57;
            initial_view.zoom = 0.4;
            initial_flight.throttle = 0.0;
            initial_flight.thrust = 0.0;
            initial_flight.spool = 0.0;
            if mode != "gear" {
                initial_flight.gear_extension = 0.0;
                initial_flight.gear_door = 0.0;
                initial_flight.gear = true;
                let time = match mode.as_str() {
                    "gear-closed" => 0.0,
                    "gear-opening" => 0.325,
                    "gear-25" => 1.275,
                    "gear-50" => 1.90,
                    "gear-75" => 2.525,
                    _ => 3.8,
                };
                collision::update_gear(&mut initial_flight, time);
                initial_view.yaw = 1.2;
                initial_view.zoom = 0.32;
                if mode == "gear-opposite" {
                    initial_view.yaw = -1.2;
                    initial_view.pitch = 0.75;
                }
                if mode == "gear-front" {
                    initial_view.yaw = 3.35;
                    initial_view.pitch = 0.2;
                    initial_view.zoom = 0.4;
                }
                if mode.starts_with("gear-canopy") {
                    initial_view.yaw = if mode.ends_with("opposite") {
                        -1.65
                    } else {
                        1.65
                    };
                    initial_view.pitch = if mode.ends_with("top") { -0.65 } else { -0.12 };
                    initial_view.zoom = 0.28;
                }
                if mode.starts_with("gear-ground") || mode == "gear-night" {
                    initial_flight = Flight::runway();
                    initial_flight.paused = true;
                    initial_flight.engine = false;
                    if mode == "gear-night" {
                        initial_flight.engine = true;
                    }
                    initial_flight.airspeed = 0.0;
                    initial_flight.mach = 0.0;
                    initial_flight.gear_compression = [collision::GEAR_TRAVEL; 3];
                    initial_flight.position = surface_point(0.0, 0.0, RUNWAY_HEIGHT + 1.32);
                    initial_flight.gear_door = if mode == "gear-ground-open" { 1.0 } else { 0.0 };
                    initial_view.pitch = 0.23;
                    initial_view.yaw = 2.0;
                    initial_view.zoom = 0.40;
                }
            }
        }
        initial_environment.day_phase = -0.7;
        if mode == "gear-night" {
            initial_environment.day_phase = 3.0;
            initial_view.yaw = 3.0;
            initial_view.pitch = 0.10;
            initial_view.zoom = 0.50;
        }
        if mode == "engine-jet-night" {
            initial_environment.day_phase = 3.0;
            initial_flight.throttle = 1.0;
            initial_flight.spool = 1.0;
            initial_flight.thrust = propulsion::jet_output(1.0, density, initial_flight.mach).0;
        }
        if mode == "far-space" {
            initial_flight.position = DVec3::Y * 1.0e10;
            initial_view.index = 1;
            initial_view.yaw = 1.5;
            initial_view.zoom = 0.4;
            initial_flight.throttle = 0.0;
            initial_flight.thrust = 0.0;
        }
        if mode == "reentry" || mode == "transonic" {
            initial_view.index = 1;
            initial_view.yaw = 1.5;
            initial_view.pitch = 0.0;
            initial_view.zoom = 0.5;
            initial_view.guide = false;
            initial_flight.density = if mode == "reentry" { 0.001 } else { 1.0 };
            initial_flight.airspeed = if mode == "reentry" { 7500.0 } else { 340.0 };
            initial_flight.mach = if mode == "reentry" { 25.0 } else { 1.0 };
            initial_flight.g_load = 4.0;
            initial_flight.velocity = initial_flight.forward() * initial_flight.airspeed;
        }
        // Refresh instrument data through the real physics path while keeping
        // the deliberately posed render-check geometry and deflections fixed.
        if mode.starts_with("terrain-") {
            initial_flight = Flight::runway();
            initial_flight.paused = true;
            let altitude = if mode == "terrain-field" { 63.48 } else { 650. };
            initial_flight.position = surface_point(6500., 3200., altitude);
            initial_flight.velocity = DVec3::ZERO;
            initial_flight.airspeed = 0.;
            initial_flight.mach = 0.;
            initial_flight.throttle = 0.;
            initial_flight.spool = 0.;
            initial_flight.engine = false;
            initial_flight.thrust = 0.;
            initial_flight.gear_compression = [0.15; 3];
            initial_flight.landed = mode == "terrain-field";
            initial_view.index = 1;
            initial_view.yaw = 1.5;
            initial_view.pitch = -0.45;
            initial_view.zoom = 0.4;
            initial_view.simple = true;
            if mode == "terrain-forest" {
                let h = terrain_height(-8000., 0.);
                initial_flight.position = terrain::point(-8000., 0., h + 260.);
                initial_view.pitch = -0.30;
                initial_view.zoom = 0.65;
            }
            // Disposable terrain profiles can reproduce tree-height and
            // landing-height views, rather than only the old 260 m preset.
            if let Some(agl) = std::env::var("NORD_TERRAIN_AGL").ok()
                .and_then(|s| s.parse::<f64>().ok()).filter(|h| h.is_finite() && *h > 0.)
            {
                let (x,z) = benchmark::terrain_origin();
                let h = terrain_height(x, z);
                initial_flight.position = terrain::point(x, z, h + agl);
                initial_flight.landed = false;
                if std::env::var("NORD_BENCH_LIVE").as_deref() == Ok("1") {
                    initial_flight.velocity = initial_flight.forward() * 180.;
                    initial_flight.engine = true;
                    initial_flight.throttle = 0.8;
                    initial_flight.spool = 0.8;
                    initial_flight.gear = false;
                    initial_flight.gear_extension = 0.;
                }
                initial_view.index = if std::env::var_os("NORD_TERRAIN_COCKPIT").is_some() {3} else {1};
                initial_view.pitch = if initial_view.index == 3 {0.} else {-0.3};
                if initial_view.index == 3 {
                    initial_view.yaw = 0.;
                    initial_view.pitch = -0.10;
                    initial_view.zoom = 1.;
                    initial_view.simple = false;
                    initial_view.help = false;
                }
            }
        }
        let mut telemetry = initial_flight.clone();
        telemetry.paused = false;
        simulate(&mut telemetry, &initial_environment, 1.0 / 120.0);
        initial_flight.alpha = telemetry.alpha;
        initial_flight.beta = telemetry.beta;
        initial_flight.aero_coeff = telemetry.aero_coeff;
        initial_flight.flow = telemetry.flow;
        initial_flight.g_load = telemetry.g_load;
        if mode == "reentry" || mode == "transonic" {
            initial_view.index = 1;
            initial_flight.g_load = 4.0;
            initial_flight.crashed = false;
        }
    }
    if let Some(mode) = preview
        .as_deref()
        .filter(|m| m.starts_with("lunar-") || m.starts_with("capsule-"))
    {
        initial_flight = space::scenario(
            if mode == "lunar-stack"
                || mode.contains("system")
                || mode.contains("transfer")
                || mode.contains("earth")
            {
                3
            } else if mode.contains("orbit")
                || mode.contains("map")
                || mode.contains("solid")
                || mode.contains("legs")
            {
                9
            } else {
                10
            },
        );
        initial_view.scenario = if initial_flight.space.body == space::Body::Earth {
            3
        } else if mode.contains("orbit") || mode.contains("map") {
            9
        } else {
            10
        };
        initial_view.index = 1;
        initial_view.yaw = 0.7;
        initial_view.pitch = 0.2;
        initial_view.zoom = 0.9;
        initial_view.help = false;
        if mode.contains("ground") || mode.contains("ascent") {
            let (c, _, q) = space::moon(0.);
            let site = space::landing_site();
            let n = q * site;
            initial_flight.position = c + n * (space::MOON_RADIUS + space::height(site) + 8.5);
            initial_flight.velocity =
                space::ground_velocity(initial_flight.position, 0., space::Body::Moon);
            for _ in 0..2400 {
                simulate(&mut initial_flight, &initial_environment, 1. / 120.);
            }
            if mode.contains("ascent") {
                space::stage(&mut initial_flight);
                initial_flight.throttle = 0.75;
                for _ in 0..600 {
                    simulate(&mut initial_flight, &initial_environment, 1. / 120.);
                }
            }
        }
        if mode.contains("cockpit") {
            initial_view.index = 3;
            initial_view.yaw = 0.;
            initial_view.pitch = if mode.contains("window") { 0.0 } else { -0.5 };
            initial_view.simple = true;
            if mode.contains("window") {
                initial_flight.orientation =
                    bevy::math::DQuat::from_rotation_arc(-DVec3::Z, -initial_flight.up());
            }
        }
        if mode.contains("map") {
            initial_view.map = true;
            initial_view.map_target = if mode.contains("system") { 3 } else { 0 };
        }
        if mode.contains("earth") {
            initial_view.map_target = 1;
            let (_, equator, east) = lunar::map_axes(&initial_flight, &initial_view);
            let light = (frame(&initial_flight).conjugate() * initial_environment.sun_direction())
                .as_vec3();
            initial_view.map_yaw = light.dot(east).atan2(light.dot(equator));
            initial_view.map_pitch = -0.35;
            if mode.contains("geography") {
                let eye = (frame(&initial_flight).conjugate()
                    * world::geo_to_local(30., 20., 0.).normalize())
                .as_vec3();
                let (north, equator, east) = lunar::map_axes(&initial_flight, &initial_view);
                initial_view.map_yaw = eye.dot(east).atan2(eye.dot(equator));
                initial_view.map_pitch = -eye.dot(north).asin();
            }
        }
        if mode.contains("solid") || mode.contains("legs") {
            initial_view.index = 0;
            initial_view.zoom = 0.65;
            initial_view.yaw = 0.35;
            initial_view.pitch = 0.15;
            if mode.contains("top") {
                initial_view.yaw = 2.75;
            }
            if mode.contains("return") {
                space::stage(&mut initial_flight);
                initial_flight.space.debris.clear();
                initial_view.zoom = 0.38;
            }
            if mode.contains("transfer") {
                initial_view.zoom = 1.0;
            }
        }
        if mode.contains("legs") {
            initial_view.zoom = 0.9;
            initial_view.yaw = 0.9;
            initial_view.pitch = 0.25;
            initial_flight.gear = false;
            initial_flight.gear_extension = 0.;
            if !mode.contains("stowed") {
                initial_flight.gear = true;
                let steps = if mode.contains("half") { 180 } else { 360 };
                for _ in 0..steps {
                    simulate(&mut initial_flight, &initial_environment, 1. / 120.);
                }
            }
        }
        if mode.starts_with("capsule-") {
            initial_flight = Flight::spacecraft();
            initial_flight.space.stage = space::Stage::Capsule;
            initial_flight.fuel = 0.;
            initial_flight.position = surface_point(0., 0., 65000.);
            initial_flight.velocity = DVec3::X * 10500. - DVec3::Y * 250.;
            initial_flight.orientation =
                bevy::math::DQuat::from_rotation_arc(DVec3::Z, initial_flight.velocity.normalize());
            for _ in 0..10 {
                simulate(&mut initial_flight, &initial_environment, 1. / 120.);
            }
        }
        if mode.starts_with("capsule-solid") {
            initial_flight = Flight::spacecraft();
            initial_flight.space.stage = space::Stage::Capsule;
            initial_flight.fuel = 0.;
            initial_view.index = 0;
            initial_view.zoom = 0.28;
            initial_view.yaw = 0.2;
            initial_view.pitch = 0.1;
        }
        initial_flight.paused = true;
    }
    let mut planner = map::Planner::default();
    if preview.as_deref() == Some("lunar-transfer-map") {
        planner.node = space::navigation::moon_transfer(&initial_flight);
        initial_view.map = true;
        initial_view.map_target = 3;
    }
    let mut combat = combat::Combat::default();
    let mut bvr_menu = combat::bvr::Menu::default();
    if preview.as_ref().is_some_and(|m| m.starts_with("bvr")) {
        initial_flight = combat::bvr::flight();
        initial_view.scenario = 8;
        initial_view.generation = 1;
        initial_view.map = preview.as_deref() != Some("bvr-flight");
        initial_view.help = false;
        bvr_menu.formation = 3;
        bvr_menu.reveal = preview.as_deref() == Some("bvr-reveal");
        combat::bvr::start(&mut combat, &initial_flight, 1, &bvr_menu);
        for _ in 0..80 {
            combat.tick(&mut initial_flight, 0.1);
        }
        initial_flight.paused = true;
        if preview.as_deref() == Some("bvr-setup") {
            bvr_menu.open = true;
            bvr_menu.saved_pause = Some(true);
        }
        if preview
            .as_ref()
            .is_some_and(|m| m.starts_with("bvr-cockpit"))
        {
            initial_view.map = false;
            initial_view.index = 3;
            initial_view.yaw = -0.35;
            initial_view.pitch = -0.61;
            initial_view.zoom = 0.45;
            if preview.as_ref().is_some_and(|m| m.contains("rwr")) {
                initial_view.yaw = 0.35;
            }
            if preview.as_ref().is_some_and(|m| m.contains("controls")) {
                initial_view.yaw = 0.;
                initial_view.pitch = -0.59;
                initial_view.zoom = 0.78;
                initial_view.simple = true;
            }
            if preview.as_ref().is_some_and(|m| m.contains("console")) {
                initial_view.yaw = if preview.as_ref().is_some_and(|m| m.ends_with("left")) {
                    1.1
                } else {
                    -1.1
                };
                initial_view.pitch = -1.1;
                initial_view.zoom = 0.90;
                initial_view.simple = true;
            }
        }
        if preview.as_ref().is_some_and(|m| m.starts_with("bvr-hud")) {
            initial_view.map = false;
            initial_view.index = 3;
            initial_view.yaw = 0.;
            initial_view.pitch = 0.;
            initial_view.zoom = 0.75;
            initial_view.simple = preview.as_ref().is_some_and(|m| m.ends_with("simple"));
        }
        if preview.as_ref().is_some_and(|m| m.contains("missiles")) {
            initial_flight.paused = false;
            combat.targets[0].position =
                initial_flight.position + initial_flight.forward() * 25000.;
            combat.targets[1].position = initial_flight.position
                + initial_flight.forward() * 30000.
                + initial_flight.orientation * DVec3::X * 2000.;
            combat::bvr::cockpit_radar::command(
                &mut combat,
                combat::bvr::cockpit_radar::Command::Lock,
            );
            for _ in 0..20 {
                combat.tick(&mut initial_flight, 0.1);
            }
            combat.fire(&initial_flight);
            for _ in 0..65 {
                combat.tick(&mut initial_flight, 0.1);
            }
            initial_flight.paused = true;
        }
    }
    if preview
        .as_ref()
        .is_some_and(|mode| mode.starts_with("combat") || mode.starts_with("air-"))
    {
        initial_flight = combat::flight();
        initial_environment.day_phase = -0.7;
        initial_view.scenario = match preview.as_deref() {
            Some("air-training") => 5,
            Some("air-hard") => 7,
            Some(mode) if mode.starts_with("air-") => 6,
            _ => 4,
        };
        initial_view.generation = 1;
        initial_view.index = 1;
        initial_view.yaw = 0.65;
        initial_view.pitch = 0.22;
        initial_view.zoom = 0.55;
        combat.start(&initial_flight, initial_view.generation);
        if initial_view.scenario >= 5 {
            combat.start_air(
                &initial_flight,
                initial_view.generation,
                initial_view.scenario,
            );
            combat.prepare_air_capture(&mut initial_flight, preview.as_deref().unwrap());
        } else {
            combat.prepare_capture(&mut initial_flight,&initial_environment, preview.as_deref().unwrap());
        }
        if preview.as_deref() == Some("combat-target") {
            initial_flight.position = combat.targets[0].position - initial_flight.forward() * 65.0
                + (initial_flight.orientation * DVec3::X) * 12.0;
        }
        initial_flight.paused = true;
        if preview.as_deref() == Some("air-component-detail") {
            initial_view.yaw = 1.5;
            initial_view.pitch = -0.55;
            initial_view.zoom = 0.27;
            initial_view.simple = true;
        }
        if matches!(
            preview.as_deref(),
            Some("air-hud" | "air-hud-threat" | "air-hud-simple")
        ) {
            initial_view.index = 0;
            initial_view.yaw = 0.0;
            initial_view.pitch = 0.10;
            initial_view.zoom = 1.0;
        }
        if matches!(
            preview.as_deref(),
            Some(
                "combat-cockpit"
                    | "air-cockpit"
                    | "air-hud-cockpit"
                    | "air-hud-cockpit-icons"
                    | "air-hud-cockpit-simple"
                    | "air-component-cockpit-simple"
            )
        ) {
            initial_view.index = 3;
            initial_view.yaw = 0.0;
            initial_view.pitch = -0.10;
            initial_view.cockpit_enemy_icons = preview.as_deref() == Some("air-hud-cockpit-icons");
        }
        if matches!(
            preview.as_deref(),
            Some("air-hud-simple" | "air-hud-cockpit-simple" | "air-component-cockpit-simple")
        ) {
            initial_view.simple = true;
        }
        if preview.as_deref() == Some("combat-follow") {
            initial_view.follow_missile(0);
        }
        if preview.as_deref() == Some("air-follow") {
            initial_view.follow_missile(combat::MISSILE_COUNT);
        }
        if preview.as_ref().is_some_and(|m| m.starts_with("air-radar")) {
            initial_view.index = 3;
            initial_view.yaw = -0.35;
            initial_view.pitch = -0.61;
            initial_view.zoom = 0.45;
            combat::bvr::cockpit_radar::command(
                &mut combat,
                combat::bvr::cockpit_radar::Command::Lock,
            );
            combat.advance(&initial_flight, 0.2);
            if preview.as_deref() == Some("air-radar-hud") {
                initial_view.yaw = 0.;
                initial_view.pitch = 0.;
                initial_view.zoom = 0.75;
            }
        }
    }
    if let Some(mode) = preview.as_deref().filter(|m| m.contains("hit-report")) {
        combat::effects::prepare_hit_capture(&mut combat, mode);
        initial_flight.paused = true;
        initial_view.map = mode.contains("map");
        initial_view.index = if mode.contains("cockpit") { 3 } else { 1 };
        initial_view.yaw = 0.;
        initial_view.pitch = 0.;
        initial_view.simple = mode.contains("simple");
    }
    if preview.as_deref() == Some("map") || preview.as_deref() == Some("lunar-node-map") {
        planner.node = Some(map::Maneuver {
            anomaly: 0.5,
            dv: DVec3::new(120.0, 0.0, 20.0),
            reference: space::relative(&initial_flight).0.normalize(),
            epoch: initial_flight.elapsed,
        });
    }
    if preview.as_deref() == Some("lunar-node-map") {
        planner.node.as_mut().unwrap().anomaly = 4.8;
        let (_, _, q) = space::moon(initial_flight.elapsed);
        let eye = (frame(&initial_flight).conjugate() * (q * -DVec3::Y)).as_vec3();
        let (_, equator, east) = lunar::map_axes(&initial_flight, &initial_view);
        initial_view.map_yaw = eye.dot(east).atan2(eye.dot(equator));
    }
    if preview.as_deref() == Some("air-audio") {
        initial_flight.paused = false;
    }
    if preview
        .as_ref()
        .is_some_and(|mode| mode.ends_with("aircraft-camera"))
    {
        initial_view.follow_aircraft(0);
    }
    if let Some(mode) = preview
        .as_deref()
        .and_then(|m| m.strip_prefix("earth-flight-"))
    {
        if let Ok(km) = mode.split('-').next().unwrap_or("").parse::<f64>() {
            initial_flight = Flight::spacecraft();
            initial_flight.position = DVec3::Y * (RADIUS + km * 1000.0);
            let (t, p, rho, a) = atmosphere(km * 1000.0);
            initial_flight.temperature = t;
            initial_flight.pressure = p;
            initial_flight.density = rho;
            initial_flight.mach = initial_flight.velocity.length() / a;
            initial_flight.paused = true;
            initial_view.index = 0;
            initial_view.pitch = -0.32;
            initial_view.zoom = 1.0;
            initial_view.help = false;
            initial_view.map = false;
            initial_environment.day_phase = 0.0;
            if mode.contains("night") { initial_environment.day_phase = std::f64::consts::PI; }
            if mode.contains("sun") {
                initial_flight.orientation = bevy::math::DQuat::from_rotation_arc(
                    -DVec3::Z, initial_environment.sun_direction());
                initial_view.index = 3;
                initial_view.yaw = 0.;
                initial_view.pitch = -0.2;
                initial_view.simple = true;
                if mode.contains("external") {
                    initial_view.index = 0;
                    initial_view.yaw = 0.4;
                    initial_view.pitch = 0.;
                }
            }
            if mode.contains("map") {
                initial_view.map = true;
                initial_view.map_target = 1;
                let eye = (frame(&initial_flight).conjugate()
                    * world::geo_to_local(15., 58., 0.).normalize()).as_vec3();
                let (north, equator, east) = lunar::map_axes(&initial_flight, &initial_view);
                initial_view.map_yaw = eye.dot(east).atan2(eye.dot(equator));
                initial_view.map_pitch = -eye.dot(north).asin();
                initial_view.map_zoom = 0.7;
            }
        }
    }
    if render_benchmark.is_some() {
        initial_flight.paused = std::env::var("NORD_BENCH_LIVE").as_deref() != Ok("1");
    }
    planner.capture_context(&initial_flight, &initial_view);
    if let Some(a)=std::env::var("NORD_AIRCRAFT").ok().and_then(|s|aircraft::Aircraft::parse(&s)) {
        aircraft::select(&mut initial_flight,a);
        // Prepare weapon captures after selecting the actual launcher; otherwise
        // the prebuilt missiles still belong to the default Gripen loadout.
        if let Some(mode) = preview.as_deref().filter(|m| m.starts_with("combat-")) {
            let paused = initial_flight.paused;
            initial_flight.paused = false;
            combat.start(&initial_flight, initial_view.generation);
            combat.prepare_capture(&mut initial_flight,&initial_environment, mode);
            initial_flight.paused = paused;
        }
        if let Ok(mode)=std::env::var("NORD_FLEET_CAPTURE") {
            initial_view.help=false;initial_view.map=false;initial_view.zoom=0.65;initial_view.yaw=2.30;initial_view.pitch=-0.30;initial_view.index=0;
            if a==aircraft::Aircraft::F22 && mode!="ground" && mode!="gear"{initial_flight.gear=false;initial_flight.gear_extension=0.;initial_flight.flaps=0.;}
            if mode=="cockpit"{initial_view.index=3;initial_view.yaw=0.;initial_view.pitch=-0.13;initial_view.zoom=1.;}
            if mode=="cockpit" && a==aircraft::Aircraft::F22{initial_view.pitch=-0.22;}
            if mode.starts_with("cockpit-") {
                initial_view.index=3;initial_view.zoom=1.;initial_view.yaw=0.;
                initial_view.simple=true;
                initial_view.pitch=match mode.as_str() {"cockpit-floor"=>-1.12,"cockpit-seat"=>-0.72,_=>-0.62};
                if mode=="cockpit-seat"{initial_view.yaw=std::f32::consts::PI;}
                if mode=="cockpit-right"{initial_view.yaw=-0.38;}
                if mode=="cockpit-left"{initial_view.yaw=0.58;}
            }
            if mode=="top"{initial_view.yaw=2.8;initial_view.pitch=-1.1;}
            if mode=="rear"{initial_view.yaw=0.35;initial_view.pitch=-0.12;}
            if mode=="rear-off"{initial_view.yaw=0.35;initial_view.pitch=-0.12;initial_flight.engine=false;initial_flight.spool=0.;initial_flight.thrust=0.;}
            if mode=="front"{initial_view.yaw=3.14;initial_view.pitch=-0.05;}
            if mode=="side"{initial_view.yaw=1.57;initial_view.pitch=-0.05;}
            if mode.starts_with("belly") {
                initial_view.simple=true;
                initial_flight.engine=false;initial_flight.spool=0.;initial_flight.thrust=0.;
                initial_flight.systems.bays=match mode.as_str(){"belly-open"|"belly-bay-side"=>1.,"belly-quarter"=>0.25,"belly-half"=>0.5,_=>0.};
                if mode=="belly-gear" {initial_flight.gear=true;initial_flight.gear_extension=1.;initial_flight.gear_door=0.;}
            }
            if mode=="external"{initial_flight.systems.external_stores=true;}
            if mode=="highalpha"{initial_flight.orientation*=bevy::math::DQuat::from_rotation_x(0.8);initial_flight.surface.x=0.65;initial_flight.systems.nozzle=[-13_f64.to_radians();2];}
            if mode=="hover"{initial_flight.systems.conversion=1.;initial_flight.systems.conversion_target=true;initial_flight.systems.nozzle=[std::f64::consts::FRAC_PI_2;2];initial_flight.spool=1.;initial_flight.thrust=190000.;}
            if mode=="bays"{initial_flight.systems.bays=1.;}
            if mode=="bays-under"{initial_flight.systems.bays=1.;initial_view.yaw=2.3;initial_view.pitch=0.6;}
            if matches!(mode.as_str(),"bays"|"bays-under"|"belly-bay-side"|"belly-open") {
                initial_flight.systems.side_bays=[aircraft::bays::State{door:1.,launcher:1.};2];
            }
            if mode.starts_with("detail-") {
                if mode.starts_with("detail-reference-") {
                    initial_flight=Flight::runway();aircraft::select(&mut initial_flight,a);
                    initial_flight.paused=true;initial_flight.flaps=0.;initial_flight.gear=true;
                    initial_flight.gear_extension=1.;initial_flight.gear_door=0.;
                }
                initial_view.simple=true;initial_view.guide=false;initial_flight.engine=false;initial_flight.spool=0.;initial_flight.thrust=0.;
                let fraction=|name|std::env::var(name).ok().and_then(|s|s.parse::<f64>().ok()).filter(|v|v.is_finite()).unwrap_or(1.).clamp(0.,1.);
                let door=fraction("NORD_BAY_DOOR");let launcher=fraction("NORD_BAY_RAIL");
                initial_flight.systems.side_bays=[aircraft::bays::State{door,launcher};2];
                if mode.starts_with("detail-mainbay") {
                    initial_flight.systems.bays=door;
                    initial_flight.systems.side_bays=default();
                    if std::env::var_os("NORD_BAY_RAIL").is_some(){initial_flight.systems.main_stroke[0]=launcher;}
                }
                if mode=="detail-plan-top" {initial_flight.systems.bays=0.;initial_flight.systems.side_bays=default();}
                if mode.starts_with("detail-reference-") && !mode.ends_with("4") {
                    initial_flight.systems.side_bays=default();
                }
                if std::env::var_os("NORD_BAY_INDEPENDENT").is_some(){initial_flight.systems.side_bays[0]=default();}
                if std::env::var_os("NORD_BAY_EMPTY").is_some(){
                    if mode.starts_with("detail-mainbay"){initial_flight.systems.stores=[false;10];}
                    else{initial_flight.systems.stores[0]=false;initial_flight.systems.stores[1]=false;}
                }
                initial_view.high_quality=std::env::var_os("NORD_DETAIL_QUALITY").is_some();
                if let Some(day)=std::env::var("NORD_DETAIL_DAY_PHASE").ok().and_then(|s|s.parse::<f64>().ok()).filter(|v|v.is_finite()) {initial_environment.day_phase=day;}
                if let Some(seconds)=std::env::var("NORD_BAY_LAUNCH_TIME").ok().and_then(|s|s.parse::<f64>().ok()).filter(|v|v.is_finite()) {
                    initial_flight.systems.side_bays=default();initial_flight.paused=false;
                    initial_flight.systems.bays=0.;initial_flight.systems.main_stroke=[0.;6];
                    combat.start(&initial_flight,initial_view.generation);
                    combat.weapon=if mode.starts_with("detail-mainbay-launch"){combat::Weapon::Radar}else{combat::Weapon::Heat};
                    combat.lock=combat.weapon.lock_time();
                    combat.fire(&initial_flight);
                    for _ in 0..(seconds.clamp(0.,3.)*240.).round()as usize {
                        physics::simulate(&mut initial_flight,&initial_environment,1./240.);combat.tick(&mut initial_flight,1./240.);
                    }
                    println!("Native bay launch: time={seconds:.4}, fired={}, ammo={:?}, side={:?}, main={:.6}, stroke={:?}",combat.fired,combat.ammo,initial_flight.systems.side_bays,initial_flight.systems.bays,initial_flight.systems.main_stroke);
                    initial_flight.paused=true;
                }
            }
            if mode=="gear"{initial_flight.gear_extension=0.5;initial_flight.gear_door=1.;initial_view.pitch=0.65;initial_view.yaw=1.25;}
            if mode.starts_with("gear-") {
                initial_view.pitch=0.45;initial_view.yaw=1.25;initial_view.zoom=0.55;
                initial_flight.gear_extension=match mode.as_str(){"gear-closed"=>0.,"gear-quarter"=>0.25,"gear-half"=>0.5,"gear-three-quarter"=>0.75,_=>1.};
                initial_flight.gear_door=if initial_flight.gear_extension>0. && initial_flight.gear_extension<1.{1.}else{0.};
            }
            if let Some(t)=mode.strip_prefix("gear-cycle-").and_then(|s|s.split('-').next()?.parse::<u32>().ok()) {
                initial_flight.gear=true;initial_flight.gear_extension=0.;initial_flight.gear_door=0.;initial_flight.gear_phase=0;
                for _ in 0..t*12 {collision::update_gear(&mut initial_flight,1./1200.);}
            }
            if mode.starts_with("nozzle-") {
                initial_view.yaw=0.10;initial_view.pitch=-0.03;initial_view.zoom=0.40;
                initial_flight.engine=false;initial_flight.spool=0.;initial_flight.thrust=0.;
                initial_flight.systems.nozzle=match mode.as_str(){"nozzle-up"|"nozzle-up-top"=>[-20_f64.to_radians();2],"nozzle-down"|"nozzle-down-top"|"nozzle-down-under"=>[20_f64.to_radians();2],"nozzle-mixed"=>[-20_f64.to_radians(),20_f64.to_radians()],_=>[0.;2]};
            }
            if mode.starts_with("exhaust-") {
                initial_flight.engine=mode!="exhaust-off";
                initial_flight.spool=if mode=="exhaust-idle"{0.25}else if mode=="exhaust-off"{0.}else{1.};
                initial_flight.throttle=initial_flight.spool;initial_flight.thrust=if initial_flight.engine{312000.*initial_flight.spool}else{0.};
                if mode=="exhaust-vector"{initial_flight.systems.nozzle=[-0.25,0.25];}
            }
            if mode.starts_with("nozzle-burner-") {
                initial_flight.engine=mode!="nozzle-burner-off";initial_flight.spool=if mode=="nozzle-burner-idle"{0.25}else{1.};
                initial_flight.throttle=initial_flight.spool;initial_flight.thrust=if initial_flight.engine{312000.*initial_flight.spool}else{0.};
            }
            if a==aircraft::Aircraft::F22 && mode.starts_with("nozzle-roll-") {
                initial_flight=if mode.starts_with("nozzle-roll-cruise-q-") {
                    aircraft::raptor::roll_case(240.,0.06,-1.,mode.ends_with("fbw"),false,120).0
                }else{aircraft::raptor::roll_trial(if mode.ends_with("left"){-0.6}else{0.6},true,false).0};
                initial_flight.paused=true;
            }
            if mode=="ground"{initial_flight=Flight::runway();aircraft::select(&mut initial_flight,a);initial_flight.paused=true;}
            if mode=="damage"{initial_flight.damage.surface_state[0]=combat::damage::SurfaceState::Jammed(0.4);initial_flight.damage.surface_state[6]=combat::damage::SurfaceState::Detached;initial_flight.damage.surfaces[6]=0.;initial_flight.damage.fire=0.6;let mut fx=std::mem::take(&mut combat.fx);let mut pose=initial_flight.clone();pose.velocity=DVec3::ZERO;for _ in 0..7{fx.advance(&combat,&pose,1./60.);}combat.fx=fx;}
        }
    }
    let initial_warp=benchmark::configured_warp(render_benchmark.is_some());
    let initial_shadow_size=if std::env::var_os("NORD_DETAIL_QUALITY").is_some(){2048}else{1024};
    App::new()
        .insert_resource(if render_benchmark.is_some() {
            bevy::winit::WinitSettings::continuous()
        } else {
            bevy::winit::WinitSettings::game()
        })
        .insert_resource(VisualCheck(if render_benchmark.is_some() { None } else { preview.clone() }))
        .insert_resource(RenderBenchmark {
            mode: render_benchmark,
            frames: Vec::new(),
            start_sim: None,
        })
        .insert_resource(initial_flight)
        .insert_resource(initial_environment)
        .insert_resource(initial_view)
        .insert_resource(SimClock::default())
        .insert_resource(initial_warp)
        .init_resource::<benchmark::Session>()
        .init_resource::<hangar::Hangar>()
        .insert_resource(start_screen::Menu::configured())
        .insert_resource(planner)
        .insert_resource(combat)
        .insert_resource(bvr_menu)
        .insert_resource(combat::bvr::cockpit_radar::Display {
            tactical: preview.as_deref() == Some("bvr-cockpit-tactical"),
            ..default()
        })
        .insert_resource(DirectionalLightShadowMap { size: initial_shadow_size })
        .insert_resource(ClearColor(Color::srgb(0.012, 0.018, 0.04)))
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.65, 0.75, 1.0),
            brightness: 1000.0,
            ..default()
        })
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {file_path:start_screen::asset_path(),..default()})
                .set(bevy::app::TaskPoolPlugin {
                    task_pool_options: bevy::app::TaskPoolOptions {
                        compute: bevy::app::TaskPoolThreadAssignmentPolicy {
                            min_threads: performance::compute_threads(),
                            max_threads: performance::compute_threads(),
                            percent: 0.0,
                            on_thread_spawn: None,
                            on_thread_destroy: None,
                        },
                        io: bevy::app::TaskPoolThreadAssignmentPolicy {
                            min_threads: 1,
                            max_threads: 1,
                            percent: 0.0,
                            on_thread_spawn: None,
                            on_thread_destroy: None,
                        },
                        async_compute: bevy::app::TaskPoolThreadAssignmentPolicy {
                            min_threads: 1,
                            max_threads: 1,
                            percent: 0.0,
                            on_thread_spawn: None,
                            on_thread_destroy: None,
                        },
                        ..default()
                    },
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "NORD / Flight Lab".into(),
                        resolution: WindowResolution::new(1440, 900)
                            .with_scale_factor_override(1.0),
                        mode: if std::env::var_os("NORD_BENCH_FULLSCREEN").is_some() {
                            bevy::window::WindowMode::BorderlessFullscreen(bevy::window::MonitorSelection::Primary)
                        } else { bevy::window::WindowMode::Windowed },
                        desired_maximum_frame_latency: std::env::var("NORD_FRAME_LATENCY").ok().and_then(|s|s.parse::<u32>().ok()).filter(|v|matches!(v,1|2)).and_then(std::num::NonZeroU32::new),
                        present_mode: match std::env::var("NORD_PRESENT_MODE").as_deref() {
                            Ok("no-vsync") => PresentMode::AutoNoVsync,
                            _ => PresentMode::AutoVsync,
                        },
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(performance::PerformancePlugin)
        .add_plugins(earth_gpu::EarthGpuPlugin)
        .add_plugins(combat::smoke_gpu::SmokeGpuPlugin)
        .add_plugins(terrain_ready::TerrainReadyPlugin)
        .add_systems(PostStartup, (mesh_reuse::share,render_batch::build).chain())
        .add_plugins(sound::FlightAudioPlugin)
        .add_plugins(display_upload::DisplayUploadPlugin)
        .insert_resource(atmospheric_render::TransitionRun::configured())
        .add_systems(PreUpdate, benchmark::neutral_inputs.after(bevy::input::InputSystems).before(bevy::ui::UiSystems::Focus))
        .add_systems(PostStartup, benchmark_ablation)
        .add_systems(
            Startup,
            (
                setup_world,
                setup_view,
                setup_ui,
                setup_destruction,
                navball::setup,
                (aircraft::visual::setup,hangar::setup).chain(),
                cockpit::setup_ball,
                instruments::setup,
                warp::setup,
                map::setup,
                lunar::setup,
                aero_fx::setup,
                combat::setup,
                combat::dogfight::setup,
                combat::hud::setup,
                combat::effects::setup,
                combat::bvr::setup,
                start_screen::setup,
            )
                .chain(),
        )
        .configure_sets(Update, (FlightStage::Controls, FlightStage::Simulation, FlightStage::World, FlightStage::Camera, FlightStage::Presentation, FlightStage::Interface, FlightStage::Capture).chain())
        .add_systems(
            Update,
            (
                (start_screen::controls,
                (
                    (apply_font,hangar::controls.run_if(start_screen::flight_controls),input.run_if(start_screen::flight_controls)).chain(),
                    warp::controls.run_if(start_screen::flight_controls),
                    (map::controls, lunar::controls, map::guidance).chain().run_if(start_screen::flight_controls),
                    scenario_buttons.run_if(start_screen::flight_controls),
                    view_controls.run_if(start_screen::flight_controls),
                    combat::cockpit_icon_controls.run_if(start_screen::flight_controls),
                    instruments::buttons.run_if(start_screen::flight_controls),
                    (
                        combat::bvr::controls,
                        combat::bvr::setup_screen::controls,
                        combat::bvr::cockpit_radar::controls,
                        cockpit_controls::controls,
                    )
                        .chain().run_if(start_screen::flight_controls),
                    combat::controls.run_if(start_screen::flight_controls),
                    hangar::systems.run_if(start_screen::flight_controls),
                    combat::dogfight::buttons.run_if(start_screen::flight_controls),
                    sound::ui.run_if(start_screen::flight_controls),
                    (benchmark::exercise, atmospheric_render::exercise).chain(),
                ).chain()).chain().in_set(FlightStage::Controls),
                    step.in_set(FlightStage::Simulation),
                (
                    combat::camera_controls.run_if(start_screen::flight_controls),
                    (
                        terrain::update,
                        update_world,
                        update_nozzle_geometry,
                        update_engine_glow,
                        update_control_surfaces,
                        update_rcs,
                        update_stars,
                        update_destruction,
                    ),
                ).chain().in_set(FlightStage::World),
                (
                    (update_camera, atmospheric_render::synchronize, earth::update).chain(),
                    combat::capture_camera,
                    sound::update,
                    sound::diagnostics,
                    draw_guidance,
                )
                    .chain().in_set(FlightStage::Camera),
                (
                    (gear::update,aircraft::visual::update,aircraft::visual::raptor_bay::update,aircraft::visual::raptor_main_bay::update,aircraft::visual::raptor_detail::update,aircraft::visual::gauges).chain(),
                    aircraft_detail::update,
                    skin_damage::update,
                    (combat::render, combat::aim9x::animate, combat::radar_visual::animate),
                    combat::smoke,
                    aero_fx::update,
                    update_hud,
                    navball::update,
                    instruments::update,
                    warp::ui,
                    (map::draw, map::editor_draw).chain(),
                    cockpit::update,
                    combat::draw,
                    combat::draw_telemetry,
                    combat::dogfight::draw,
                    combat::hud::draw,
                    combat::effects::render,
                    combat::effects::render_hit,
                    (
                        combat::effects::controls.run_if(start_screen::flight_controls),
                        lunar::visuals,
                        lunar::cabin_visibility,
                        lunar::ui,
                    )
                        .chain(),
                    (
                        combat::bvr::draw,
                        combat::bvr::setup_screen::draw,
                        combat::bvr::cockpit_radar::visuals,
                    )
                        .chain(),
                ).in_set(FlightStage::Presentation),
                (declutter_ui,
                start_screen::ui,
                start_screen::presentation).chain().in_set(FlightStage::Interface),
                (capture_visual_check, atmospheric_render::capture, benchmark_render).chain().in_set(FlightStage::Capture),
            )
                ,
        )
        .run();
}

#[derive(Resource)]
struct RenderBenchmark {
    mode: Option<String>,
    frames: Vec<f64>,
    start_sim: Option<f64>,
}
// Explicit diagnostic modes isolate rendering costs; ordinary flight is untouched.
fn benchmark_ablation(
    mut commands: Commands,
    benchmark: Res<RenderBenchmark>,
    surfaces: Query<(Entity, &MeshMaterial3d<StandardMaterial>)>,
    materials: Res<Assets<StandardMaterial>>,
) {
    let Some(mode) = &benchmark.mode else { return };
    for (entity, handle) in &surfaces {
        if mode.contains("noshadows") {
            commands.entity(entity).insert(bevy::light::NotShadowCaster);
        }
        if mode.contains("noglass")
            && materials
                .get(&handle.0)
                .is_some_and(|m| matches!(m.alpha_mode, AlphaMode::Blend | AlphaMode::Add))
        {
            commands.entity(entity).remove::<Mesh3d>();
        }
    }
}
fn benchmark_render(time: Res<Time<bevy::time::Real>>, f: Res<Flight>, v: Res<ViewState>, warp: Res<warp::Warp>, windows: Query<&Window>, session: Res<benchmark::Session>, diagnostics: Res<DiagnosticsStore>, mut benchmark: ResMut<RenderBenchmark>) {
    let Some(mode) = benchmark.mode.clone() else {
        return;
    };
    let elapsed = time.elapsed_secs_f64();
    let warmup = session.warmup;
    let sample = session.seconds;
    if elapsed < warmup {
        return;
    }
    if benchmark.start_sim.is_none() {
        benchmark.start_sim = Some(f.elapsed);
        performance::reset();
    }
    benchmark.frames.push(time.delta_secs_f64() * 1000.0);
    if elapsed < warmup + sample {
        return;
    }
    benchmark.frames.sort_by(f64::total_cmp);
    let frames = &benchmark.frames;
    let mean = frames.iter().sum::<f64>() / frames.len() as f64;
    let p50 = frames[frames.len() / 2];
    let p95 = frames[frames.len() * 95 / 100];
    let p99 = frames[frames.len() * 99 / 100];
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let resolution=windows.iter().next().map(|w|[w.physical_width(),w.physical_height()]);
    let report = serde_json::json!({"profile":profile,"mode":mode,"scene":std::env::var("NORD_BENCH_SCENE").ok(),"paused":f.paused,"resolution":resolution,"quality":v.high_quality,"timestep":warp.timestep,"warp_rates":warp.rates,"exercise_events":session.events,"warmup_seconds":warmup,"sample_seconds":elapsed-warmup,"frames":frames.len(),"mean_ms":mean,"p50_ms":p50,"p95_ms":p95,"p99_ms":p99,"mean_fps":1000.0/mean,
        "simulated_seconds":performance::simulated_seconds(),"simulation_rate":performance::simulated_seconds()/(elapsed-warmup),"over_25_ms":frames.iter().filter(|&&t|t>25.).count(),"over_50_ms":frames.iter().filter(|&&t|t>50.).count(),"measurements":performance::report(&diagnostics),
        "workers":{"compute":bevy::tasks::ComputeTaskPool::get().thread_num(),"async_compute":bevy::tasks::AsyncComputeTaskPool::get().thread_num(),"io":bevy::tasks::IoTaskPool::get().thread_num()},
        "note":"Frame intervals include VSync and scheduling; CPU spans may be nested and must not be summed"});
    std::fs::write(
        std::env::var("NORD_BENCH_REPORT").unwrap_or_else(|_| format!("tests/render-{profile}-{mode}.json")),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .expect("save render benchmark");
    println!(
        "render {profile}/{mode}: {:.1} fps, p50 {:.2} ms, p95 {:.2} ms",
        1000.0 / mean,
        p50,
        p95
    );
    std::process::exit(0);
}
fn setup_view(
    mut c: Commands,
    mut medium: ResMut<Assets<ScatteringMedium>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let mut atmosphere = Atmosphere::earthlike(medium.add(ScatteringMedium::default()));
    atmosphere.bottom_radius = RADIUS as f32;
    atmosphere.top_radius = (RADIUS + 100000.0) as f32;
    c.insert_resource(SkyProfile(atmosphere.clone()));
    let camera=c.spawn((
        Camera3d::default(),
        FlightCamera,
        bevy::ui::IsDefaultUiCamera,
        bevy::pbr::generate::EnvironmentMapScene::default(),
        Transform::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 60_f32.to_radians(),
            near: 0.25,
            far: 40_000_000.0,
            ..default()
        }),
        atmosphere,
        AtmosphereSettings {
            aerial_view_lut_max_distance: 50000.0,
            sky_view_lut_size: UVec2::new(160, 80),
            sky_view_lut_samples: 6,
            aerial_view_lut_size: UVec3::splat(12),
            aerial_view_lut_samples: 4,
            ..default()
        },
        AtmosphereEnvironmentMapLight::default(),
        Exposure { ev100: 12.3 },
        Tonemapping::AcesFitted,
        Bloom::NATURAL,
        DistanceFog {
            color: Color::srgb(0.58, 0.65, 0.69),
            falloff: FogFalloff::Exponential { density: 0.000015 },
            ..default()
        },
        Msaa::Off,
        Fxaa::default(),
    )).id();
    // Developer comparison: reject hidden opaque fragments before their full
    // material shaders, without changing geometry, MSAA or lighting quality.
    if std::env::var_os("NORD_TERRAIN_DEPTH_PREPASS").is_some() {
        c.entity(camera).insert(bevy::core_pipeline::prepass::DepthPrepass);
    }
    c.spawn((
        DirectionalLight {
            color: Color::WHITE,
            illuminance: 95000.0,
            shadows_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            first_cascade_far_bound: 180.0,
            maximum_distance: 2500.0,
            num_cascades: 2,
            ..default()
        }
        .build(),
        Transform::default(),
        Sun,
        // Also sees the shadow-only forest proxies; cameras stay on layer 0.
        bevy::camera::visibility::RenderLayers::from_layers(&[0, terrain::SHADOW_LAYER]),
    ));
    let sun = mats.add(StandardMaterial {
        base_color: Color::BLACK,
        emissive: LinearRgba::new(1.0e8, 9.0e7, 7.5e7, 1.0),
        // StandardMaterial's unlit path bypasses emissive lighting entirely.
        unlit: false,
        reflectance: 0.0,
        fog_enabled: false,
        ..default()
    });
    c.spawn((
        Mesh3d(meshes.add(Sphere::new(celestial::SUN_RADIUS).mesh().uv(32, 24))),
        MeshMaterial3d(sun),
        Transform::default(),
        Visibility::Hidden,
        SunBody,
        // A luminous sky proxy must never shadow its own directional sunlight.
        bevy::light::NotShadowCaster,
        bevy::light::NotShadowReceiver,
    ));
}
fn spawn_text(c: &mut Commands, text: &str, size: f32, color: Color, node: Node, tag: u8) {
    c.spawn((
        Text::new(text),
        TextFont {
            font_size: size,
            ..default()
        },
        TextColor(color),
        TextShadow {
            offset: Vec2::new(1.0, 1.0),
            color: Color::srgba(0.0, 0.025, 0.03, 0.9),
        },
        node,
        Hud(tag),
        ZIndex(20),
    ));
}
fn setup_ui(mut c: Commands, mut fonts: ResMut<Assets<Font>>) {
    c.insert_resource(UiFont(fonts.add(
        Font::try_from_bytes(include_bytes!("../assets/Barlow-Medium.ttf").to_vec()).unwrap(),
    )));
    c.spawn((
        Button,
        SimplifyButton,
        Node {
            position_type: PositionType::Absolute,
            left: px(355),
            top: px(20),
            padding: UiRect::axes(px(12), px(7)),
            border_radius: BorderRadius::all(px(5)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.025, 0.065, 0.075, 0.9)),
        ZIndex(50),
    ))
    .with_children(|p| {
        p.spawn((
            SimplifyLabel,
            Text::new("F1 / SIMPLIFY VIEW"),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::srgb(0.75, 0.93, 0.91)),
        ));
    });
    spawn_text(
        &mut c,
        "",
        13.0,
        Color::srgb(0.7, 0.94, 0.86),
        Node {
            position_type: PositionType::Absolute,
            left: px(28),
            bottom: px(265),
            ..default()
        },
        11,
    );
    let ice = Color::srgb(0.75, 0.93, 0.94);
    let green = Color::srgb(0.44, 1.0, 0.74);
    c.spawn((
        Chrome,
        Node {
            width: percent(100),
            height: px(75),
            position_type: PositionType::Absolute,
            top: px(0),
            left: px(0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.015, 0.025, 0.038, 0.84)),
    ));
    spawn_text(
        &mut c,
        "N O R D   /   FLIGHT LAB",
        24.0,
        ice,
        Node {
            position_type: PositionType::Absolute,
            top: px(17),
            left: px(28),
            ..default()
        },
        0,
    );
    spawn_text(
        &mut c,
        "JAS 39C  |  SÅTENÄS F7  |  SWEDEN",
        12.0,
        Color::srgb(0.48, 0.67, 0.70),
        Node {
            position_type: PositionType::Absolute,
            top: px(49),
            left: px(29),
            ..default()
        },
        1,
    );
    spawn_text(
        &mut c,
        "",
        14.0,
        ice,
        Node {
            position_type: PositionType::Absolute,
            top: px(20),
            right: px(26),
            ..default()
        },
        2,
    );
    let buttons = [
        ("F2  COLD & DARK", 0),
        ("F3  TAKEOFF", 1),
        ("F4  APPROACH", 2),
        ("F5  SPACECRAFT", 3),
        ("F12 TARGET PRACTICE", 4),
        ("4 COMBAT: TRAINING", 5),
        ("5 COMBAT: BALANCED", 6),
        ("8 COMBAT: HARD", 7),
    ];
    for (i, (label, scenario)) in buttons.iter().enumerate() {
        let b = c
            .spawn((
                Button,
                ScenarioButton(*scenario),
                Chrome,
                Node {
                    position_type: PositionType::Absolute,
                    top: px(if i >= 5 { 173.0 } else { 91.0 }),
                    left: px(28.0 + if i >= 5 { (i - 5) as f32 } else { i as f32 } * 178.0),
                    width: px(164),
                    height: px(32),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(px(1)),
                    ..default()
                },
                BorderColor::all(Color::srgba(0.40, 0.70, 0.73, 0.5)),
                BackgroundColor(Color::srgba(0.02, 0.05, 0.07, 0.76)),
            ))
            .id();
        let t = c
            .spawn((
                Text::new(*label),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(ice),
            ))
            .id();
        c.entity(b).add_child(t);
    }
    spawn_text(
        &mut c,
        "",
        26.0,
        green,
        Node {
            position_type: PositionType::Absolute,
            left: px(38),
            top: percent(36),
            ..default()
        },
        3,
    );
    spawn_text(
        &mut c,
        "",
        26.0,
        green,
        Node {
            position_type: PositionType::Absolute,
            right: px(38),
            top: percent(36),
            ..default()
        },
        4,
    );
    spawn_text(
        &mut c,
        "",
        16.0,
        green,
        Node {
            position_type: PositionType::Absolute,
            left: percent(39),
            top: percent(25),
            ..default()
        },
        5,
    );
    spawn_text(
        &mut c,
        "",
        16.0,
        ice,
        Node {
            position_type: PositionType::Absolute,
            bottom: px(184),
            left: px(28),
            ..default()
        },
        6,
    );
    spawn_text(
        &mut c,
        "",
        16.0,
        ice,
        Node {
            position_type: PositionType::Absolute,
            bottom: px(40),
            right: px(28),
            ..default()
        },
        7,
    );
    spawn_text(
        &mut c,
        "",
        16.0,
        green,
        Node {
            position_type: PositionType::Absolute,
            bottom: px(20),
            left: px(28),
            ..default()
        },
        8,
    );
    spawn_text(
        &mut c,
        "",
        13.0,
        Color::srgb(0.81, 0.89, 0.91),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(66),
            left: px(28),
            ..default()
        },
        9,
    );
    spawn_text(
        &mut c,
        "+",
        34.0,
        Color::srgba(0.50, 1.0, 0.72, 0.65),
        Node {
            position_type: PositionType::Absolute,
            top: percent(48),
            left: percent(49),
            ..default()
        },
        10,
    );
}
fn reset(f: &mut Flight, scenario: u8, v: &mut ViewState) {
    v.restore_camera();
    let best = f.best_landing;
    let aircraft = f.aircraft;
    let external_stores=f.systems.external_stores;
    *f = match scenario {
        0 => Flight::cold(),
        1 => Flight::runway(),
        2 => Flight::approach(),
        4 => combat::flight(),
        8 => combat::bvr::flight(),
        5..=7 => {
            let mut f = combat::flight();
            f.status = "AIR COMBAT / TAB TARGET · O WEAPON · SPACE MISSILE · U GUN".into();
            f
        }
        9 | 10 => space::scenario(scenario),
        _ => Flight::spacecraft(),
    };
    f.best_landing = best;
    if f.vehicle==Vehicle::Gripen && aircraft!=aircraft::Aircraft::Gripen {aircraft::select(f,aircraft);}else{f.aircraft=aircraft;}
    if aircraft==aircraft::Aircraft::F22{f.systems.external_stores=external_stores;}
    v.generation = v.generation.wrapping_add(1);
    v.scenario = scenario;
    v.index = 0;
    v.map = false;
    v.map_target = 0;
    v.map_zoom = 1.;
    v.map_focus_ship = false;
    v.place_flyby(f);
    v.yaw = 0.0;
    v.pitch = 0.0;
}
fn view_controls(
    mut view: ResMut<ViewState>,
    buttons: Query<&Interaction, (With<SimplifyButton>, Changed<Interaction>)>,
    preview: Option<Res<VisualCheck>>,
) {
    if view.left_camera_drag || preview.is_some_and(|p| p.0.is_some()) {
        return;
    }
    if buttons.iter().any(|i| *i == Interaction::Pressed) {
        view.simple = !view.simple;
    }
}
fn declutter_ui(
    view: Res<ViewState>,
    f: Res<Flight>,
    mut chrome: Query<&mut Node, (With<Chrome>, Without<SimplifyButton>)>,
    mut button: Query<&mut Node, (With<SimplifyButton>, Without<Chrome>)>,
    mut labels: Query<&mut Text, With<SimplifyLabel>>,
) {
    for mut node in &mut chrome {
        let display = if view.simple && !view.map {
            Display::None
        } else {
            Display::Flex
        };
        if node.display != display { node.display = display; }
    }
    for mut node in &mut button {
        let left = if view.simple { Val::Auto } else { px(355) };
        let right = if view.simple { px(22) } else { Val::Auto };
        if node.left != left { node.left = left; }
        if node.right != right { node.right = right; }
    }
    for mut text in &mut labels {
        let value = if view.simple {
            format!("F1 / FULL VIEW{}", if f.paused { " · PAUSED" } else { "" })
        } else {
            "F1 / SIMPLIFY VIEW".into()
        };
        if text.0 != value {
            text.0 = value;
        }
    }
}
fn input(
    hangar: Option<Res<hangar::Hangar>>,
    bvr_menu: Option<Res<combat::bvr::Menu>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    mut cursor_motion: MessageReader<CursorMoved>,
    buttons: Query<&Interaction, With<Button>>,
    mut shadow_map: ResMut<DirectionalLightShadowMap>,
    time: Res<Time>,
    mut f: ResMut<Flight>,
    mut e: ResMut<Environment>,
    mut v: ResMut<ViewState>,
    warp: Res<warp::Warp>,
    preview: Option<Res<VisualCheck>>,
) {
    if hangar.as_ref().is_some_and(|h|h.open||h.block_input) || warp.editing
        || bvr_menu.as_ref().is_some_and(|m| m.open)
        || preview.is_some_and(|p| p.0.is_some())
    {
        return;
    }
    let dt = time.delta_secs_f64().min(0.1);
    let cursor_delta: Vec2 = cursor_motion.read().filter_map(|event| event.delta).sum();
    let orbit_delta = if motion.delta.length_squared() > 0.0 {
        motion.delta
    } else {
        cursor_delta
    };
    for (key, id) in [
        (KeyCode::F2, 0),
        (KeyCode::F3, 1),
        (KeyCode::F4, 2),
        (KeyCode::F5, 3),
        (KeyCode::F12, 4),
        (KeyCode::Digit4, 5),
        (KeyCode::Digit5, 6),
        (KeyCode::Digit8, 7),
    ] {
        if keys.just_pressed(key) {
            reset(&mut f, id, &mut v);
        }
    }
    if keys.just_pressed(KeyCode::F10) {
        v.aero_debug = !v.aero_debug;
    }
    if keys.just_pressed(KeyCode::F11) {
        v.collision_debug = !v.collision_debug;
    }
    if keys.just_pressed(KeyCode::F9) {
        v.high_quality = !v.high_quality;
        shadow_map.size = if v.high_quality { 2048 } else { 1024 };
    }
    if keys.just_pressed(KeyCode::F7) && f.vehicle == Vehicle::Spacecraft {
        f.sas_mode = (f.sas_mode + 1) % space::SAS_MODES.len() as u8;
        f.assist = true;
    }
    if keys.just_pressed(KeyCode::F8) {
        let aircraft=f.aircraft;
        v.generation = v.generation.wrapping_add(1);
        v.scenario = 2;
        *f = Flight::approach();
        if aircraft!=aircraft::Aircraft::Gripen{aircraft::select(&mut f,aircraft);}
        f.position = surface_point(175.0, 160.0, RUNWAY_HEIGHT + 10.0);
        f.velocity = DVec3::X * 110.0;
        f.orientation = bevy::math::DQuat::from_rotation_y(-std::f64::consts::FRAC_PI_2);
        f.gear = false;
        v.index = 10;
    }
    if keys.just_pressed(KeyCode::KeyR) {
        let id = v.scenario;
        reset(&mut f, id, &mut v);
    }
    if keys.just_pressed(KeyCode::KeyP) || keys.just_pressed(KeyCode::Escape) {
        f.paused = !f.paused;
    }
    if keys.just_pressed(KeyCode::KeyC) {
        v.restore_camera();
        v.index = (v.index + 1) % 4;
        if v.index == 3 {
            v.zoom = 1.0;
        }
        v.map = false;
        v.place_flyby(&f);
        v.yaw = 0.0;
        v.pitch = 0.0;
        if v.index==3&&f.vehicle==Vehicle::Gripen&&f.aircraft==aircraft::Aircraft::F22{v.pitch=-0.18;}
    }
    if keys.just_pressed(KeyCode::KeyJ) && !(keys.pressed(KeyCode::AltLeft)||keys.pressed(KeyCode::AltRight)) {
        v.restore_camera();
        v.index = 0;
        v.map = false;
    }
    if keys.just_pressed(KeyCode::Home) {
        v.yaw = 0.0;
        v.pitch = if v.index==3&&f.vehicle==Vehicle::Gripen&&f.aircraft==aircraft::Aircraft::F22{-0.18}else{0.};
        if v.index == 3 {
            v.zoom = 1.0;
        }
    }
    if keys.just_pressed(KeyCode::KeyH) && !(keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight)) {
        v.help = !v.help;
    }
    if keys.just_pressed(KeyCode::F1) {
        v.simple = !v.simple;
    }
    if keys.just_pressed(KeyCode::KeyV) && v.scenario < 5 {
        v.guide = !v.guide;
    }
    if keys.just_pressed(KeyCode::KeyG) {
        f.gear = !f.gear;
    }
    if keys.just_pressed(KeyCode::KeyF) {
        f.flaps = if f.flaps < 0.2 {
            0.55
        } else if f.flaps < 0.8 {
            1.0
        } else {
            0.0
        };
    }
    if keys.just_pressed(KeyCode::KeyY) && (f.aircraft!=aircraft::Aircraft::F35B || keys.pressed(KeyCode::AltLeft)||keys.pressed(KeyCode::AltRight)) {
        f.airbrake = !f.airbrake;
    }
    if keys.just_pressed(KeyCode::KeyN) {
        f.infinite_fuel = !f.infinite_fuel;
    }
    if keys.just_pressed(KeyCode::KeyT) {
        f.assist = !f.assist;
    }
    if keys.just_pressed(KeyCode::F6) && (keys.pressed(KeyCode::AltLeft)||keys.pressed(KeyCode::AltRight)) {
        f.autopilot = !f.autopilot;
    }
    if keys.just_pressed(KeyCode::Digit1) {
        f.battery = !f.battery;
    }
    if keys.just_pressed(KeyCode::Digit2)
        && (keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight))
        && f.battery
    {
        f.apu = !f.apu;
    }
    if keys.just_pressed(KeyCode::Digit3) && f.battery && f.apu {
        f.engine = !f.engine;
        if !f.engine {
            f.thrust = 0.0;
        }
    }
    if keys.just_pressed(KeyCode::KeyL) {
        e.day_phase = (e.day_phase + 0.30).rem_euclid(std::f64::consts::TAU);
    }
    if keys.just_pressed(KeyCode::KeyI) {
        f.lights = !f.lights;
    }
    if keys.just_pressed(KeyCode::KeyM) {
        v.toggle_map(&f);
    }
    if keys.just_pressed(KeyCode::KeyK) && f.vehicle == Vehicle::Gripen {
        e.wind_enabled = !e.wind_enabled;
    }
    if keys.pressed(KeyCode::BracketRight) {
        e.day_phase += dt * 0.20;
    }
    if keys.pressed(KeyCode::BracketLeft) {
        e.day_phase -= dt * 0.20;
    }
    if !f.paused {
        if keys.pressed(KeyCode::KeyZ) {
            f.throttle = (f.throttle + dt * 0.25).min(1.0);
        }
        if keys.pressed(KeyCode::KeyX) {
            f.throttle = (f.throttle - dt * 0.25).max(0.0);
        }
    }
    if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        f.autopilot = false;
        f.throttle = 1.0;
    }
    if keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight) {
        f.autopilot = false;
        f.throttle = 0.0;
        f.thrust = 0.0;
    }
    f.brakes = keys.pressed(KeyCode::KeyB)&&!(keys.pressed(KeyCode::AltLeft)||keys.pressed(KeyCode::AltRight));
    let pitch = (keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown)) as i32
        - (keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp)) as i32;
    let roll = (keys.pressed(KeyCode::KeyE) || keys.pressed(KeyCode::ArrowRight)) as i32
        - (keys.pressed(KeyCode::KeyQ) || keys.pressed(KeyCode::ArrowLeft)) as i32;
    let yaw = keys.pressed(KeyCode::KeyA) as i32 - keys.pressed(KeyCode::KeyD) as i32;
    f.controls = DVec3::new(pitch as f64, yaw as f64, roll as f64);
    if v.map {
        v.map_zoom = (v.map_zoom * (-scroll.delta.y * 0.08).exp()).clamp(0.03, 100.0);
    } else {
        v.zoom = (v.zoom * (-scroll.delta.y * 0.08).exp()).clamp(0.3, 5.0);
    }
    let interacting = f.vehicle == Vehicle::Gripen
        && v.index == 3
        && (keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight));
    if interacting {
        v.left_camera_drag = false;
    }
    if mouse.just_pressed(MouseButton::Left) && !interacting {
        v.left_camera_drag = buttons.iter().all(|i| *i == Interaction::None);
    }
    if !mouse.pressed(MouseButton::Left) {
        v.left_camera_drag = false;
    }
    if !interacting && (v.left_camera_drag || mouse.pressed(MouseButton::Right)) {
        if v.map {
            v.map_yaw -= orbit_delta.x * 0.004;
            v.map_pitch = (v.map_pitch - orbit_delta.y * 0.004).clamp(-1.45, 1.45);
        } else {
            v.yaw -= orbit_delta.x * 0.004;
            v.pitch = (v.pitch - orbit_delta.y * 0.004).clamp(-1.2, 1.2);
        }
    }
}
fn scenario_buttons(
    hangar:Option<Res<hangar::Hangar>>,
    mut buttons: Query<(&Interaction, &ScenarioButton, &mut BackgroundColor), Changed<Interaction>>,
    mut f: ResMut<Flight>,
    mut v: ResMut<ViewState>,
    preview: Option<Res<VisualCheck>>,
) {
    if hangar.as_ref().is_some_and(|h|h.open||h.block_input) || v.left_camera_drag || preview.is_some_and(|p| p.0.is_some()) {
        return;
    }
    for (i, s, mut color) in &mut buttons {
        match i {
            Interaction::Pressed => {
                reset(&mut f, s.0, &mut v);
                color.0 = Color::srgba(0.05, 0.30, 0.28, 0.9);
            }
            Interaction::Hovered => color.0 = Color::srgba(0.08, 0.19, 0.22, 0.9),
            Interaction::None => color.0 = Color::srgba(0.02, 0.05, 0.07, 0.76),
        }
    }
}
fn step(
    _time: Res<Time>,
    mut clock: ResMut<SimClock>,
    mut f: ResMut<Flight>,
    mut e: ResMut<Environment>,
    mut warp: ResMut<warp::Warp>,
    combat: Option<ResMut<combat::Combat>>,
) {
    let _measure = performance::span(0);
    let mut combat = combat;
    if f.paused {
        clock.accumulator = 0.0;
        return;
    }
    warp.effective = warp.allowed(&f);
    if warp.effective > 4.0 {
        let requested = warp.timestep * warp.effective;
        let duration = f
            .space
            .burn_stop
            .map_or(requested, |at| (at - f.elapsed).max(0.).min(requested));
        let advanced = if duration > 1e-9 {
            warp::coast(&mut f, &mut e, duration)
        } else {
            0.
        };
        performance::advanced(advanced);
        if advanced < requested - 1e-6 {
            warp.normal();
            warp.effective = 1.;
            warp.message = "Warp stopped / planned burn, encounter or entry".into();
        }
        clock.accumulator = 0.0;
        return;
    }
    clock.accumulator = warp.timestep * warp.effective;
    let requested=clock.accumulator;
    while clock.accumulator > 1e-12 {
        let dt = clock.accumulator.min(1.0 / 120.0);
        e.advance(dt);
        simulate(&mut f, &e, dt);
        if let Some(ref mut combat) = combat {
            combat.sun = e.sun_direction();
            combat.environment = e.clone();
            combat.tick(&mut f, dt);
            combat::effects::advance(combat, &f, dt);
        }
        clock.accumulator -= dt;
    }
    performance::advanced(requested);
}
fn update_camera(
    mut ambient: ResMut<GlobalAmbientLight>,
    mut media: ResMut<Assets<ScatteringMedium>>,
    mut last_sky_density: Local<Option<f32>>,
    mut commands: Commands,
    sky: Res<SkyProfile>,
    f: Res<Flight>,
    e: Res<Environment>,
    v: Res<ViewState>,
    combat: Res<combat::Combat>,
    mut cameras: Query<
        (
            Entity,
            &mut Transform,
            &mut Projection,
            &mut DistanceFog,
            Option<&Atmosphere>,
            &mut AtmosphereSettings,
            &mut Msaa,
            &mut Fxaa,
            &mut Exposure,
            &mut AtmosphereEnvironmentMapLight,
            Option<&mut bevy::light::GeneratedEnvironmentMapLight>,
        ),
        With<FlightCamera>,
    >,
    mut suns: Query<
        (&mut Transform, &mut DirectionalLight),
        (With<Sun>, Without<FlightCamera>, Without<SunBody>),
    >,
    mut sunbody: Query<
        (&mut Transform, &mut Visibility),
        (With<SunBody>, Without<FlightCamera>, Without<Sun>),
    >,
) {
    let _measure = performance::span(8);
    let density = earth::sky_density(f.altitude());
    if last_sky_density.is_none_or(|old| {
        (old - density).abs() > 0.003 || (old != density && (density == 0.0 || density == 1.0))
    }) {
        if let Some(medium) = media.get_mut(&sky.0.medium) {
            *medium = ScatteringMedium::default().with_density_multiplier(density);
        }
        *last_sky_density = Some(density);
    }
    let mut pos = local(f.position, &f);
    let rot = local_rot(f.orientation, &f);
    let forward = rot * -Vec3::Z;
    let up = Vec3::Y;
    let right = rot * Vec3::X;
    let look = Quat::from_rotation_y(v.yaw) * Quat::from_rotation_x(v.pitch);
    let index = if v.map && f.vehicle == Vehicle::Gripen {
        [10, 0, 7, 1][v.index.min(3)]
    } else {
        v.render_index()
    };
    if f.vehicle == Vehicle::Spacecraft && index != 1 && !v.map {
        pos += rot
            * Vec3::new(
                0.,
                0.,
                match f.space.stage {
                    space::Stage::Transfer => 7.5,
                    space::Stage::Descent => 2.,
                    space::Stage::Return => 0.5,
                    space::Stage::Capsule => -1.2,
                },
            );
    }
    let daylight = ((e.sun_direction().dot(f.up()) as f32 + 0.06) / 0.18).clamp(0.0, 1.0);
    let surface_scene = f.vehicle == Vehicle::Gripen && !v.map;
    let ambient_brightness = if surface_scene {
        4.0 + 996.0 * daylight
    } else {
        1000.0
    };
    if ambient.brightness != ambient_brightness { ambient.brightness = ambient_brightness; }
    let (mut p, mut target, mut up_view) = match index {
        0 => (pos + Vec3::new(0.0, 9.0, 36.0), pos, up),
        1 => (
            pos + rot * (cockpit::eye(f.vehicle)+if f.vehicle==Vehicle::Gripen{f.aircraft.cabin_offset()}else{Vec3::ZERO}),
            pos + rot * (cockpit::eye(f.vehicle)+if f.vehicle==Vehicle::Gripen{f.aircraft.cabin_offset()}else{Vec3::ZERO}) + rot * (look * -Vec3::Z) * 500.0,
            rot * Vec3::Y,
        ),
        2 => (pos + forward * 30.0 + up * 7.0, pos, up),
        3 => (pos - right * 22.0 + up * 6.0, pos, up),
        4 => (pos + right * 22.0 + up * 6.0, pos, up),
        5 => (pos - forward * 14.0 + rot * Vec3::Y * 3.0, pos, up),
        6 => (
            local(surface_point(130.0, -210.0, RUNWAY_HEIGHT + 38.0), &f),
            pos,
            up,
        ),
        7 => (
            local(
                if v.flyby == DVec3::ZERO {
                    f.position + f.velocity.normalize_or_zero() * 160.0 + f.up() * 18.0
                } else {
                    v.flyby
                },
                &f,
            ),
            pos,
            up,
        ),
        8 => {
            let (center, radius) = lunar::map_focus(&f, &v);
            let focus = local(center, &f);
            let minimum = if v.map_focus_ship {
                100.
            } else {
                radius * 1.08
            };
            let distance = (radius * 3.2 * v.map_zoom as f64).max(minimum) as f32;
            let (north, equator, east) = lunar::map_axes(&f, &v);
            let horizontal = equator * v.map_yaw.cos() + east * v.map_yaw.sin();
            let mut camera =
                focus + (horizontal * v.map_pitch.cos() - north * v.map_pitch.sin()) * distance;
            let primary = local(space::center(f.space.body, f.elapsed), &f);
            let offset = camera - primary;
            let min_radius = f.space.body.radius() as f32 * 1.02;
            if offset.length() < min_radius {
                camera = primary + offset.normalize_or(Vec3::Y) * min_radius;
            }
            (camera, focus, north)
        }
        10 => (
            pos + rot * (look * Vec3::new(0.0, 6.0, 30.0) * v.zoom),
            pos,
            rot * Vec3::Y,
        ),
        _ => {
            let radial = -e.sun_direction();
            let tangent = DVec3::X.cross(radial).normalize();
            let normal = radial.cross(tangent);
            let sun = e.sun_direction() * e.sun_distance();
            (
                local(sun + normal * AU * 1.9 + radial * AU * 0.2, &f),
                local(sun, &f),
                (frame(&f).conjugate() * tangent).as_vec3(),
            )
        }
    };
    if index == 0 {
        let center = pos;
        p = center + look * (p - center) * v.zoom;
    }
    if let Ok(mode)=std::env::var("NORD_FLEET_CAPTURE") {
        if mode.starts_with("detail-") {
            let s=if mode.contains("left"){-1.}else{1.};
            let (at,offset)=if mode.starts_with("detail-reference-") {
                match mode.as_str() {
                    "detail-reference-1"=>(Vec3::new(0.,-0.05,-3.8),Vec3::new(0.,-0.10,-29.)),
                    "detail-reference-2"=>(Vec3::new(0.,0.,-2.0),Vec3::new(-12.,9.,-17.)),
                    "detail-reference-3"=>(Vec3::new(0.,0.,-5.1),Vec3::new(9.,0.4,-9.)),
                    "detail-reference-5"=>(Vec3::new(0.,-0.15,-1.),Vec3::new(-30.,0.,0.)),
                    _=>(Vec3::new(0.,-0.05,-4.3),Vec3::new(-13.,0.35,-2.8)),
                }
            }else if mode=="detail-plan-top" {
                (Vec3::new(0.,0.,-0.2),Vec3::new(0.,32.,0.001))
            }else if mode.starts_with("detail-mainbay") {
                (Vec3::new(0.,-0.6,0.),if mode.contains("under"){Vec3::new(0.,-11.,0.001)}else{Vec3::new(s*5.,-6.,-3.5)})
            }else if mode.contains("inlet") {
                let at=Vec3::new(s*1.55,-0.04,-3.65);
                let offset=if mode.contains("front"){Vec3::new(s*0.12,0.04,-3.5)}
                    else if mode.contains("upper"){Vec3::new(s*2.2,1.7,-2.5)}
                    else if mode.contains("under"){Vec3::new(s*1.8,-1.5,-2.3)}
                    else if mode.contains("join"){Vec3::new(s*3.2,0.3,1.0)}
                    else{Vec3::new(s*2.5,0.15,-2.7)};
                (at,offset)
            }else if mode.contains("launch") {
                (Vec3::new(0.,-0.2,-3.0),Vec3::new(s*13.,-1.8,-7.0))
            }else{
                let at=Vec3::new(s*1.9,-0.45,-1.70);
                let offset=if mode.contains("front"){Vec3::new(s*2.9,-0.9,-2.6)}
                    else if mode.contains("rear"){Vec3::new(s*2.8,-0.8,2.9)}
                    else if mode.contains("under"){Vec3::new(s*1.2,-3.5,0.1)}
                    else if mode.contains("upper"){Vec3::new(s*2.9,1.2,0.7)}
                    else{Vec3::new(s*4.5,-0.3,0.1)};
                (at,offset)
            };
            target=pos+rot*at;p=target+rot*offset;up_view=rot*Vec3::Y;
            if mode=="detail-plan-top" {up_view=rot*Vec3::X;}
            if mode.starts_with("detail-mainbay")&&mode.contains("under"){up_view=rot*Vec3::NEG_Z;}
        }
        if mode.starts_with("belly") {
            target=pos+rot*Vec3::new(0.,-0.35,-0.3);
            let offset=match mode.as_str(){
                "belly-side"|"belly-bay-side"=>Vec3::new(20.,-0.8,-1.5),
                "belly-profile"=>Vec3::new(20.,0.,0.),
                "belly-profile-left"=>Vec3::new(-20.,0.,0.),
                "belly-intake"=>Vec3::new(3.7,-1.0,-9.5),
                "belly-front"=>Vec3::new(11.,-7.,-19.),
                "belly-bottom"=>Vec3::new(0.01,-25.,0.),
                _=>Vec3::new(14.,-10.,17.),
            };
            p=target+rot*offset;
            up_view=rot*if mode=="belly-bottom"{Vec3::NEG_Z}else{Vec3::Y};
        }
        if mode.starts_with("nozzle-") {target=pos+rot*Vec3::new(0.,0.,7.05);p=target+rot*Vec3::new(1.2,1.1,6.8);up_view=rot*Vec3::Y;}
        if mode=="gear-main-close" {target=pos+rot*Vec3::new(1.15,-1.1,1.7);p=target+rot*Vec3::new(2.7,-0.8,3.0);up_view=rot*Vec3::Y;}
        if mode=="gear-nose-close" {target=pos+rot*Vec3::new(0.,-1.1,-5.5);p=target+rot*Vec3::new(2.1,-0.6,2.8);up_view=rot*Vec3::Y;}
        if mode.starts_with("gear-cycle-") {target=pos+rot*Vec3::new(0.,-0.65,-1.5);p=target+rot*Vec3::new(8.,-4.,9.);up_view=rot*Vec3::Y;}
        if mode.starts_with("gear-cycle-") && mode.ends_with("-bay") {target=pos+rot*Vec3::new(1.15,-0.1,0.5);p=target+rot*Vec3::new(0.10,-3.0,0.4);up_view=rot*Vec3::Z;}
        if matches!(mode.as_str(),"nozzle-top"|"nozzle-up-top"|"nozzle-down-top") {target=pos+rot*Vec3::new(0.,0.,7.6);p=target+rot*Vec3::new(2.4,4.0,4.0);up_view=rot*Vec3::Y;}
        if mode=="nozzle-side" {target=pos+rot*Vec3::new(0.,0.,6.7);p=target+rot*Vec3::new(5.0,2.2,3.0);up_view=rot*Vec3::Y;}
        if mode=="nozzle-down-under" {target=pos+rot*Vec3::new(0.,0.,7.3);p=target+rot*Vec3::new(2.4,-3.5,4.0);up_view=rot*Vec3::Y;}
        if mode.starts_with("nozzle-burner-") {target=pos+rot*Vec3::new(0.93,-0.12,7.0);p=target+rot*Vec3::new(0.05,0.04,4.5);up_view=rot*Vec3::Y;}
        if mode.starts_with("exhaust-") {target=pos+rot*Vec3::new(0.,0.,10.2);p=target+rot*Vec3::new(7.5,1.2,5.5);up_view=rot*Vec3::Y;}
    }
    if let Some(missile) = v
        .missile_follow
        .and_then(|follow| combat.render_missile(follow.missile))
        .filter(|m| m.alive)
    {
        let center = local(missile.position, &f);
        let direction = (frame(&f).conjugate() * missile.velocity.normalize()).as_vec3();
        let radial = (frame(&f).conjugate() * missile.position.normalize()).as_vec3();
        let cross = direction.cross(radial);
        let right = if cross.length_squared() > 1e-8 {
            cross.normalize()
        } else {
            direction
                .cross(if direction.x.abs() < 0.9 {
                    Vec3::X
                } else {
                    Vec3::Z
                })
                .normalize()
        };
        let missile_frame = Quat::from_mat3(&Mat3::from_cols(
            right,
            right.cross(direction).normalize_or_zero(),
            -direction,
        ));
        // Stay above and beside the launch aircraft while the missile separates.
        p = center + missile_frame * (look * Vec3::new(3.0, 4.0, 12.0)) * v.zoom;
        target = center + direction * 0.45;
        up_view = radial;
    }
    if let Some(aircraft) = v
        .aircraft_follow
        .and_then(|i| combat.targets.get(i))
        .filter(|t| t.alive)
    {
        let center = local(aircraft.position, &f);
        let radial = (frame(&f).conjugate() * aircraft.position.normalize()).as_vec3();
        let forward = (frame(&f).conjugate() * (aircraft.orientation * -DVec3::Z)).as_vec3();
        let right = forward.cross(radial).normalize_or(Vec3::X);
        let level = Quat::from_mat3(&Mat3::from_cols(
            right,
            right.cross(forward).normalize_or(Vec3::Y),
            -forward,
        ));
        p = center + level * (look * Vec3::new(0., 7., 28.)) * v.zoom;
        target = center;
        up_view = radial;
    }
    let mut sun_pose = None;
    for (
        entity,
        mut t,
        mut projection,
        mut fog,
        atmosphere,
        mut settings,
        mut msaa,
        mut fxaa,
        mut exposure,
        mut environment_light,
        generated_light,
    ) in &mut cameras
    {
        let daylight = ((e.sun_direction().dot(f.up()) as f32 + 0.06) / 0.18).clamp(0.0, 1.0);
        let ev = if f.vehicle == Vehicle::Gripen && !v.map {
            7.7 + 4.6 * daylight
        } else {
            12.3
        };
        if (exposure.ev100 - ev).abs() > 0.001 {
            exposure.ev100 = ev;
        }
        let sky_size = if v.high_quality {
            UVec2::new(400, 200)
        } else {
            UVec2::new(160, 80)
        };
        if settings.sky_view_lut_size != sky_size {
            settings.sky_view_lut_size = sky_size;
            settings.sky_view_lut_samples = if v.high_quality { 16 } else { 6 };
            settings.aerial_view_lut_size = UVec3::splat(if v.high_quality { 32 } else { 12 });
            settings.aerial_view_lut_samples = if v.high_quality { 10 } else { 4 };
        }
        let target_msaa = if v.high_quality {
            Msaa::Sample4
        } else if v.benchmark_msaa {
            Msaa::Sample2
        } else {
            Msaa::Off
        };
        if *msaa != target_msaa {
            *msaa = target_msaa;
        }
        let enable_fxaa = !v.high_quality && !v.benchmark_msaa;
        if fxaa.enabled != enable_fxaa {
            fxaa.enabled = enable_fxaa;
        }
        // Fade sky IBL along with scattering before releasing the atmosphere.
        // Otherwise removing the sky at 100 km abruptly darkens the vehicle.
        let sky_intensity=earth::sky_density(f.altitude());
        if environment_light.intensity!=sky_intensity { environment_light.intensity=sky_intensity; }
        // Bevy initializes this generated component once; changes to the source
        // atmosphere light's intensity are not copied into it afterwards.
        if let Some(mut generated) = generated_light {
            if generated.intensity!=sky_intensity { generated.intensity=sky_intensity; }
        }
        let no_atmosphere = v.map
            || (f.vehicle == Vehicle::Spacecraft
                && (f.space.body == space::Body::Moon || f.altitude() > 100000.0));
        if no_atmosphere && atmosphere.is_some() {
            commands.entity(entity).remove::<Atmosphere>();
        }
        if !no_atmosphere && atmosphere.is_none() {
            commands.entity(entity).insert(sky.0.clone());
        }
        t.set_if_neq(Transform::from_translation(p).looking_at(target, up_view));
        let reference_capture=std::env::var("NORD_FLEET_CAPTURE").ok().filter(|s|s.starts_with("detail-reference-")||s=="detail-plan-top"||s.starts_with("detail-mainbay"));
        let fov = if let Some(mode)=reference_capture { if mode.ends_with("1"){14_f32.to_radians()}else{29_f32.to_radians()} }else if index == 1 {
            (75.0 * v.zoom).clamp(30.0, 85.0).to_radians()
        } else if index==3&&f.vehicle==Vehicle::Gripen&&f.aircraft==aircraft::Aircraft::F22 {
            (72.0*v.zoom).clamp(38.0,82.0).to_radians()
        }else {
            60_f32.to_radians()
        };
        let near = if v.map { 1000.0 } else { 0.025 };
        let far = if v.map && f.vehicle == Vehicle::Spacecraft {
            (space::MOON_DISTANCE * 12. * v.map_zoom.max(1.) as f64).max(f.position.length() * 3.)
                as f32
        } else if v.index == 9 {
            AU as f32 * 5.0
        } else {
            (f.position.length() * 2.5).max(if f.vehicle == Vehicle::Spacecraft {
                space::MOON_DISTANCE * 2.0
            } else {
                45_000_000.0
            }) as f32
        };
        sun_pose = Some(celestial::sun_transform(p, local(e.sun_direction() * e.sun_distance(), &f), far));
        let projection_changed = matches!(&*projection, Projection::Perspective(pp) if pp.fov != fov || pp.near != near || pp.far != far);
        if projection_changed && let Projection::Perspective(pp) = &mut *projection {
            pp.fov = fov;
            pp.near = near;
            pp.far = far;
        }
        let fog_density = if v.map {
                0.0
            } else if e.weather == 2 {
                0.00006 * (f.density / 1.225).clamp(0.0, 1.0) as f32
            } else {
                0.000015 * (f.density / 1.225).clamp(0.0, 1.0) as f32
            };
        if !matches!(fog.falloff, FogFalloff::Exponential { density } if density == fog_density) {
            fog.falloff = FogFalloff::Exponential { density: fog_density };
        }
    }
    let dir = (frame(&f).conjugate() * e.sun_direction()).as_vec3();
    for (mut t, mut light) in &mut suns {
        if light.shadows_enabled==v.map { light.shadows_enabled=!v.map; }
        let illuminance = if surface_scene {
            95000.0 * ((e.sun_direction().dot(f.up()) as f32 + 0.02) / 0.08).clamp(0.0, 1.0)
        } else {
            95000.0
        };
        if light.illuminance!=illuminance { light.illuminance=illuminance; }
        t.set_if_neq(Transform::from_translation(dir * 10000.0).looking_at(Vec3::ZERO, Vec3::Y));
    }
    for (mut t, mut vis) in &mut sunbody {
        if let Some(pose) = sun_pose { t.set_if_neq(pose); }
        // Atmospheric scattering already supplies its own solar disk. Once it
        // has faded away, retain a geometric disk in flight and orbital maps.
        let show = v.index == 9 || v.map || f.space.body == space::Body::Moon
            || (f.vehicle == Vehicle::Spacecraft && earth::sky_density(f.altitude()) == 0.0);
        vis.set_if_neq(if show && sun_pose.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
    }
}
fn draw_guidance(f: Res<Flight>, e: Res<Environment>, v: Res<ViewState>, mut gizmos: Gizmos) {
    if v.collision_debug && !v.map && f.vehicle == Vehicle::Spacecraft {
        gizmos.sphere(
            Isometry3d::from_translation(local(f.position, &f)),
            2.1,
            Color::srgb(1., 0.65, 0.15),
        );
        for (z, r) in space::body_sections(f.space.stage) {
            gizmos.circle(
                Isometry3d::new(
                    local(f.position + f.orientation * DVec3::Z * *z, &f),
                    local_rot(f.orientation, &f),
                ),
                *r as f32,
                Color::srgb(1., 0.65, 0.15),
            );
        }
        if f.space.stage == space::Stage::Descent {
            for i in 0..4 {
                let p = local(f.position + f.orientation * space::leg_point(&f, i), &f);
                gizmos.sphere(
                    Isometry3d::from_translation(p),
                    0.2,
                    Color::srgb(0.3, 1., 0.7),
                );
            }
        }
    }
    if v.collision_debug && !v.map && f.vehicle == Vehicle::Gripen {
        let color = Color::srgb(1.0, 0.65, 0.14);
        for hull in airframe::for_flight(&f) {
            let (q, offset) = hull.pose(&f);
            let point = |i: usize| {
                local(
                    f.position + f.orientation * (q * hull.vertices[i] + offset),
                    &f,
                )
            };
            let n = hull.vertices.len() / 2;
            for i in 0..n {
                let next = (i + 1) % n;
                gizmos.line(point(i), point(next), color);
                gizmos.line(point(i + n), point(next + n), color);
                gizmos.line(point(i), point(i + n), color);
            }
        }
        if f.vehicle == Vehicle::Gripen && f.gear_extension > 0.001 {
            for i in 0..3 {
                let (mount, wheel, _) = collision::gear_pose(&f, i, true);
                let p = |body| local(f.position + f.orientation * body, &f);
                gizmos.line(p(mount), p(wheel), color);
                gizmos.sphere(
                    Isometry3d::from_translation(p(wheel)),
                    collision::WHEEL_RADIUS as f32,
                    color,
                );
            }
        }
    }
    if v.index == 9 {
        let center = e.sun_direction() * e.sun_distance();
        let axis = -e.sun_direction();
        let tangent = DVec3::X.cross(axis).normalize();
        let _normal = axis.cross(tangent);
        let orbit = e.planet_orbit();
        let nu = orbit.z.atan2(orbit.x);
        let plane_x = axis * nu.cos() - tangent * nu.sin();
        let plane_z = axis * nu.sin() + tangent * nu.cos();
        let points = (0..=180).map(|i| {
            let a = i as f64 / 180.0 * std::f64::consts::TAU;
            local(
                center
                    + (plane_x * (a.cos() - 0.0167)
                        + plane_z * (1.0_f64 - 0.0167_f64.powi(2)).sqrt() * a.sin())
                        * AU,
                &f,
            )
        });
        gizmos.linestrip(points, Color::srgb(0.35, 0.72, 0.82));
        return;
    }
    if v.map {
        return;
    }
    if v.guide && f.vehicle == Vehicle::Gripen && v.scenario < 4 {
        for i in 0..12 {
            let z = 1300.0 + i as f64 * 450.0;
            let h = RUNWAY_HEIGHT + 1.55 + (z - 1000.0) * 0.0524;
            let center = surface_point(0.0, z, h);
            let corners = [
                center + DVec3::new(-28.0, -15.0, 0.0),
                center + DVec3::new(28.0, -15.0, 0.0),
                center + DVec3::new(28.0, 15.0, 0.0),
                center + DVec3::new(-28.0, 15.0, 0.0),
                center + DVec3::new(-28.0, -15.0, 0.0),
            ];
            gizmos.linestrip(
                corners.map(|p| local(p, &f)),
                Color::srgba(0.3, 0.95, 0.7, 0.55),
            );
        }
    }
}
fn update_hud(
    time: Res<Time>,
    mut last: Local<render_cache::PresentationClock>,
    f: Res<Flight>,
    _e: Res<Environment>,
    v: Res<ViewState>,
    combat: Res<combat::Combat>,
    diagnostics: Res<DiagnosticsStore>,
    mut texts: Query<(&Hud, &mut Text, &mut Visibility, &mut Node)>,
) {
    if !last.due(time.elapsed_secs(), 0.05, &v) {
        return;
    }
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(60.0);
    for (tag, mut t, mut vis, mut node) in &mut texts {
        if tag.0 == 3 {
            let top = if !v.simple && f.vehicle == Vehicle::Gripen {
                px(390)
            } else {
                percent(36)
            };
            if node.top != top {
                node.top = top;
            }
        }
        let visibility = if ((v.map || v.index == 3) && [3, 4, 5, 6, 7, 8, 10].contains(&tag.0))
            || (combat.active && tag.0 == 10)
            || (v.following() && [3, 4, 5, 6, 7, 9, 10, 11].contains(&tag.0))
            || (v.simple && !v.map && [0, 1, 2, 5, 6, 7, 9, 10, 11].contains(&tag.0))
        {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        vis.set_if_neq(visibility);
        // Hidden telemetry does not need formatting or text-layout invalidation.
        // Reopening the view computes fresh text in this same update.
        if visibility == Visibility::Hidden {
            continue;
        }
        let updated = match tag.0 {
            0 => "N O R D   /   FLIGHT LAB".into(),
            1 => {
                if let Some((id, t)) = v
                    .aircraft_follow
                    .and_then(|id| combat.targets.get(id).map(|t| (id, t)))
                {
                    format!(
                        "T{:02} / {} / {:.0} m/s / {:.0} m ALT",
                        id + 1,
                        if combat.bvr.as_ref().is_some_and(|b| b.friendly(id)) {
                            "ALLY"
                        } else {
                            "TARGET"
                        },
                        t.dynamics.airspeed,
                        t.dynamics.altitude()
                    )
                } else if f.vehicle == Vehicle::Spacecraft {
                    format!(
                        "{} / {} / MANUAL LUNAR MISSION",
                        f.space.body.label(),
                        f.space.stage.label()
                    )
                } else {
                    format!(
                        "{}  |  SÅTENÄS F7  /  ESIB  |  58.4283° N, 12.7111° E",
                        if f.vehicle == Vehicle::Gripen {
                            f.aircraft.name()
                        } else {
                            "RCS SPACECRAFT"
                        }
                    )
                }
            }
            2 => format!(
                "{}  /  {:.0} fps\nSIM {:06.1}s",
                if v.aircraft_follow.is_some() {
                    "AIRCRAFT FOLLOW / 2 NEXT · 7 RETURN"
                } else if v.missile_follow.is_some() {
                    "MISSILE FOLLOW / 6 NEXT · 7 RETURN"
                } else if v.index == 9 {
                    "SOLAR SYSTEM / EARTH DISPLAY x180"
                } else {
                    if v.map {
                        if f.vehicle == Vehicle::Gripen {
                            "TACTICAL MAP"
                        } else {
                            "ORBIT MAP"
                        }
                    } else {
                        VIEWS[v.index]
                    }
                },
                fps,
                f.elapsed
            ),
            3 if f.vehicle == Vehicle::Spacecraft => format!(
                "ORBIT\n{:.0} m/s\n\nSURFACE {:.0} m/s\n{:+.1} G",
                orbit::Orbit::from_flight(&f).speed,
                (f.velocity - space::ground_velocity(f.position, f.elapsed, f.space.body)).length(),
                f.g_load
            ),
            3 => format!(
                "{:>4.0} m/s\n\nM {:.2}\n\n{:+.1} G",
                f.airspeed, f.mach, f.g_load
            ),
            4 => format!(
                "{:>6.0} m\n\nAGL {:>5.0} m\n\nV/S {:+.1} m/s",
                f.altitude(),
                f.agl(),
                f.vertical_speed()
            ),
            5 => format!(
                "HDG {:03.0}°     PITCH {:+.1}°     BANK {:+.1}°",
                f.heading(),
                f.pitch().to_degrees(),
                f.roll().to_degrees()
            ),
            6 if f.vehicle == Vehicle::Spacecraft => format!(
                "MAIN ENGINE {:3.0}%   THRUST {:.0} kN\nFUEL {:.0} kg {}\nSAS {}   MODE {}\nRCS TORQUE {:.0} Nm   HEALTH {:.0}%",
                f.throttle * 100.0,
                f.thrust / 1000.0,
                f.fuel,
                if f.infinite_fuel { "INFINITE" } else { "" },
                if f.assist { "ON" } else { "OFF" },
                space::sas_label(&f),
                f.rcs_torque.length(),
                f.integrity * 100.0
            ),
            6 => format!(
                "THROTTLE {:3.0}%   THRUST {:.0} kN\nFUEL {:.0} kg {}   GEAR {}\nFLAPS {:.0}%   AIRBRAKE {}   FBW {}\nAoA {:+.1}°   SLIP {:+.1}°   HEALTH {:.0}%",
                f.throttle * 100.0,
                f.thrust / 1000.0,
                f.fuel,
                if f.infinite_fuel { "INFINITE" } else { "" },
                if f.gear_health.iter().any(|health| *health <= 0.0) {
                    "FAILED".to_string()
                } else if f.gear_extension > 0.999 {
                    "DOWN / LOCKED".to_string()
                } else if f.gear_extension < 0.001 {
                    "UP".to_string()
                } else {
                    format!(
                        "{} {:.0}%",
                        if f.gear { "EXTENDING" } else { "RETRACTING" },
                        f.gear_extension * 100.0
                    )
                },
                f.flaps * 100.0,
                if f.airbrake { "ON" } else { "OFF" },
                if f.assist { "ON" } else { "OFF" },
                f.alpha.to_degrees(),
                f.beta.to_degrees(),
                f.integrity * 100.0
            ),
            7 if f.vehicle == Vehicle::Spacecraft => {
                let orbital = orbit::Orbit::from_flight(&f);
                format!(
                    "ORBITAL ELEMENTS\nAP {}   PE {:.1} km\nPERIOD {}   ECC {:.4}\nMAIN ENGINE / RCS   FUEL {:.0} kg",
                    orbital
                        .apoapsis
                        .map(|a| format!("{:.1} km", a / 1000.0))
                        .unwrap_or("ESCAPE".into()),
                    orbital.periapsis / 1000.0,
                    orbital
                        .period
                        .map(|p| format!("{:.1} min", p / 60.0))
                        .unwrap_or("—".into()),
                    orbital.eccentricity,
                    f.fuel
                )
            }
            7 => format!(
                "RUNWAY 19  /  SÅTENÄS\nRANGE {:.2} km   /   CROSS TRACK {:+.0} m\nrho {:.3} kg/m3   T {:.0} K   P {:.0} hPa\nWIND {:.1} m/s   BEST LANDING {:.0}",
                ((f.position.z - 1000.0).powi(2) + f.position.x.powi(2)).sqrt() / 1000.0,
                f.position.x,
                f.density,
                f.temperature,
                f.pressure / 100.0,
                f.wind.length(),
                f.best_landing
            ),
            8 => {
                if f.paused {
                    "PAUSED   /   P TO RESUME".into()
                } else {
                    f.status.clone()
                }
            }
            9 => {
                if v.help && f.vehicle == Vehicle::Spacecraft {
                    "SPACEFLIGHT  W/S pitch · Q/E roll · A/D yaw\nPOWER  Shift full · Ctrl cut · Z/X adjust\nSTAGING  Space (throttle off) · G landing legs\nGUIDANCE  F7 SAS mode · T assist · M map\nREENTRY  K arm parachutes · N infinite fuel\nWARP  , / . rate · / normal time\nVIEW  C camera · Home recenter · R restart\nPAUSE  P · H help · 9 audio · 0 mute".into()
                } else if v.help && v.scenario >= 5 {
                    format!("FLIGHT   W/S pitch · Q/E roll · A/D yaw\nPOWER   Shift full · Ctrl cut · Z/X adjust\nWEAPONS   Tab target · O type · Space missile\nCANNON   U fire · {}\nDEFENSE   V flare + chaff · Backspace chaff\nVIEW   C camera · 2 aircraft · 6 missile / 7 return · R restart\nAIRCRAFT   Y STOVL (F-35B) · Alt+Y airbrake · Alt+B bays · Alt+J jettison\nAUDIO   9 volume · 0 mute", if v.index == 3 { "aim large circle at target" } else { "align cross with gun pipper" })
                } else if v.help && v.scenario == 4 {
                    "FLIGHT   W/S pitch · Q/E roll · A/D yaw\nPOWER   Shift full · Ctrl cut · Z/X adjust\nWEAPONS   Tab target · O type · Space missile\nVIEW   C camera · 2 aircraft · 6 missile / 7 return\nRESTART   R · PAUSE   P · HELP   H\nAIRCRAFT   Y STOVL (F-35B) · Alt+Y airbrake · Alt+B bays · Alt+J jettison\nAUDIO   9 volume · 0 mute".into()
                } else if v.help {
                    "FLIGHT   W/S pitch · Q/E roll · A/D yaw\nPOWER   Shift full · Ctrl cut · Z/X adjust\nGROUND   G gear · B brakes\nVIEW   C camera · M map · Home recenter\nWARP   , / . rate · / normal time\nSYSTEMS   1 battery · Alt+2 APU · 3 engine\nASSIST   I lights · T FBW · F6 hangar · Alt+F6 approach · H help\nAIRCRAFT   Y STOVL (F-35B) · Alt+Y airbrake · Alt+B bays · Alt+J jettison\nAUDIO   9 volume · 0 mute".into()
                } else {
                    String::new()
                }
            }
            11 => {
                if v.aero_debug {
                    format!(
                        "AERO / CL {:.3}  CD {:.3}  ATTACHED FLOW {:.0}%\nDRAG / PARASITE {:.3}  INDUCED {:.3}  WAVE {:.3}  SURFACES {:.3}",
                        f.aero_coeff.lift,
                        f.aero_coeff.drag,
                        f.aero_coeff.attached * 100.0,
                        f.aero_coeff.parasite,
                        f.aero_coeff.induced,
                        f.aero_coeff.wave,
                        f.aero_coeff.control_drag
                    )
                } else {
                    String::new()
                }
            }
            _ => {
                if v.index < 6 {
                    "+".into()
                } else {
                    String::new()
                }
            }
        };
        if t.0 != updated {
            t.0 = updated;
        }
    }
}

fn apply_font(font: Res<UiFont>, mut texts: Query<&mut TextFont, Added<TextFont>>) {
    for mut t in &mut texts {
        t.font = font.0.clone();
    }
}

fn verify_flight() {
    let mut f = Flight::approach();
    let mut e = Environment::default();
    e.wind_enabled = false;
    f.autopilot = true;
    for i in 0..18000 {
        e.advance(1.0 / 120.0);
        simulate(&mut f, &e, 1.0 / 120.0);
        if i % 1200 == 0 {
            println!(
                "approach t={:.1} alt={:.1} speed={:.1} vz={:.1} x={:.1} z={:.1} pitch={:.1} crashed={} landed={}",
                f.elapsed,
                f.altitude(),
                f.airspeed,
                f.vertical_speed(),
                f.position.x,
                f.position.z,
                f.pitch().to_degrees(),
                f.crashed,
                f.landed
            );
        }
        if f.crashed {
            break;
        }
    }
    println!("approach result: {}", f.status);
    let mut ship = Flight::spacecraft();
    let initial = orbital_energy(&ship);
    let mut max_error = 0.0_f64;
    for _ in 0..432000 {
        e.advance(1.0 / 120.0);
        simulate(&mut ship, &e, 1.0 / 120.0);
        max_error = max_error.max((orbital_energy(&ship) - initial).abs() / initial.abs());
        assert!(ship.position.is_finite() && ship.velocity.is_finite());
    }
    println!(
        "one-hour spacecraft orbit: relative energy error={:.6}, altitude={:.1} km",
        max_error,
        ship.altitude() / 1000.0
    );
    assert!(max_error < 0.01, "orbital energy drift");
}
#[cfg(test)]
mod lunar_warp_regression {
    use super::*;
    #[test]
    fn slow_motion_advances_all_clocks_at_half_speed_without_changing_setting() {
        let mut app=App::new();let mut w=warp::Warp::default();
        w.selected=w.rates.iter().position(|r|*r==0.5).unwrap();
        app.insert_resource(Flight::spacecraft()).insert_resource(w)
            .init_resource::<Time>().init_resource::<Environment>()
            .init_resource::<SimClock>().add_systems(Update,step);
        for _ in 0..60{app.update();}
        assert!((app.world().resource::<Flight>().elapsed-0.5).abs()<1e-8);
        assert!((app.world().resource::<Environment>().time-0.5).abs()<1e-8);
        assert_eq!(app.world().resource::<warp::Warp>().timestep,1./60.);
    }
    #[test]
    fn high_warp_stops_at_planned_burn_before_skipping_it() {
        let mut app = App::new();
        let mut f = Flight::spacecraft();
        f.space.burn_stop = Some(20.);
        let mut warp = warp::Warp::default();
        warp.selected = warp.rates.len() - 1;
        app.insert_resource(f)
            .insert_resource(warp)
            .init_resource::<Time>()
            .init_resource::<Environment>()
            .init_resource::<SimClock>()
            .add_systems(Update, step);
        app.update();
        assert!((app.world().resource::<Flight>().elapsed - 20.).abs() < 1e-6);
        let w=app.world().resource::<warp::Warp>();
        assert_eq!(w.rates[w.selected],1.);
        assert!((app.world().resource::<Environment>().time - 20.).abs() < 1e-6);
    }
}
fn orbital_energy(f: &Flight) -> f64 {
    let v = f.velocity + physics::rotation().cross(f.position);
    0.5 * v.length_squared() - MU / f.position.length()
}

// App-native render capture avoids OS input automation and makes visual checks repeatable.
#[derive(Resource)]
struct VisualCheck(Option<String>);
fn capture_visual_check(
    mut c: Commands,
    preview: Res<VisualCheck>,
    f: Res<Flight>,
    time: Res<Time>,
    mut captured: Local<bool>,
    screens: Query<&cockpit::Instrument>,
    images: Res<Assets<Image>>,
    suns: Query<(&Transform, &GlobalTransform, &Visibility, &ViewVisibility), With<SunBody>>,
    cameras: Query<&Transform, With<FlightCamera>>,
) {
    let Some(mode) = &preview.0 else {
        return;
    };
    if *captured || time.elapsed_secs() < 8.0 {
        return;
    }
    *captured = true;
    if mode.starts_with("terrain-") {
        println!("terrain capture: position={:?}, agl={}, forest={}, ground={}",
            f.position,f.agl(),terrain::forest(f.position.x,f.position.z),terrain_height(f.position.x,f.position.z));
    }
    if mode.starts_with("earth-flight-") {
        for (t, global, visibility, in_view) in &suns {
            for camera in &cameras {
                let ray = (t.translation - camera.translation).normalize();
                println!("solar capture: pose={t:?}, global={global:?}, visibility={visibility:?}, in_view={in_view:?}, forward_dot={}", ray.dot(*camera.forward()));
            }
        }
    }
    cockpit::dump(&screens, &images);
    let path = format!("tests/visual-{mode}.png");
    c.spawn(bevy::render::view::screenshot::Screenshot::primary_window())
        .observe(
            move |event: On<bevy::render::view::screenshot::ScreenshotCaptured>| {
                event
                    .image
                    .clone()
                    .try_into_dynamic()
                    .expect("convert visual-check screenshot")
                    .to_rgb8()
                    .save(&path)
                    .expect("save visual-check screenshot");
                // This disposable render-check process has no state to persist.
                // Exit after the synchronous save to avoid macOS renderer teardown stalls.
                std::process::exit(0);
            },
        );
}

#[cfg(test)]
mod camera_input_tests {
    use super::*;
    #[test]
    fn restart_key_preserves_scenario_and_selected_aircraft() {
        for aircraft in [aircraft::Aircraft::Gripen, aircraft::Aircraft::F22] {
            for scenario in 0..=10 {
                let mut app = scene(false);
                let mut expected = Flight::runway();
                expected.aircraft = aircraft;
                let mut view = ViewState::default();
                reset(&mut expected, scenario, &mut view);
                app.insert_resource(expected.clone()).insert_resource(view);
                {
                    let mut f = app.world_mut().resource_mut::<Flight>();
                    f.elapsed = 42.;
                    f.crashed = true;
                    f.position += DVec3::splat(100.);
                    f.fuel = 0.;
                }
                app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyR);
                app.update();
                let view = app.world().resource::<ViewState>();
                let f = app.world().resource::<Flight>();
                assert_eq!(view.scenario, scenario, "R changed scenario {scenario}");
                assert_eq!(view.generation, 2);
                assert_eq!(f.vehicle, expected.vehicle);
                assert_eq!(f.aircraft, aircraft);
                assert_eq!(f.position, expected.position);
                assert_eq!(f.velocity, expected.velocity);
                assert_eq!(f.engine, expected.engine);
                assert_eq!(f.fuel, expected.fuel);
                assert_eq!(f.elapsed, 0.);
                assert!(!f.crashed);
            }
        }
    }
    #[test]
    fn simplified_view_toggles_when_paused_and_does_not_change_camera_or_help() {
        let mut app = scene(false);
        app.world_mut().resource_mut::<Flight>().paused = true;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F1);
        app.update();
        assert!(app.world().resource::<ViewState>().simple);
        assert!(app.world().resource::<ViewState>().help);
        assert_eq!(app.world().resource::<ViewState>().index, 0);
        let chrome = app.world_mut().spawn((Chrome, Node::default())).id();
        app.add_systems(Update, declutter_ui);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.update();
        assert_eq!(
            app.world().get::<Node>(chrome).unwrap().display,
            Display::None
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F1);
        app.update();
        assert!(!app.world().resource::<ViewState>().simple);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.update();
        assert_eq!(
            app.world().get::<Node>(chrome).unwrap().display,
            Display::Flex
        );
    }
    #[test]
    fn missile_camera_key_does_not_reset_flight_and_hard_combat_uses_eight() {
        let mut app = scene(false);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Digit6);
        app.update();
        assert_eq!(app.world().resource::<ViewState>().generation, 0);
        assert_eq!(app.world().resource::<ViewState>().scenario, 2);
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.reset_all();
        keys.press(KeyCode::Digit8);
        app.update();
        assert_eq!(app.world().resource::<ViewState>().scenario, 7);
        assert_eq!(app.world().resource::<ViewState>().generation, 1);
    }
    #[test]
    fn plain_two_leaves_apu_unchanged_and_option_two_toggles_it() {
        let mut app = scene(false);
        let initial = app.world().resource::<Flight>().apu;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Digit2);
        app.update();
        assert_eq!(app.world().resource::<Flight>().apu, initial);
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.reset_all();
        keys.press(KeyCode::AltLeft);
        keys.press(KeyCode::Digit2);
        app.update();
        assert_eq!(app.world().resource::<Flight>().apu, !initial);
    }
    #[test]
    fn mixed_follow_views_preserve_original_view_and_map_exits_tracking() {
        let mut v = ViewState {
            index: 3,
            yaw: 0.7,
            pitch: -0.3,
            zoom: 0.6,
            map: true,
            ..default()
        };
        v.follow_aircraft(2);
        v.follow_missile(0);
        v.follow_aircraft(1);
        v.restore_camera();
        assert_eq!(
            (v.index, v.yaw, v.pitch, v.zoom, v.map),
            (3, 0.7, -0.3, 0.6, true)
        );
        v.map = false;
        v.follow_aircraft(0);
        v.toggle_map(&Flight::runway());
        assert!(v.map);
        assert!(!v.following());
        assert_eq!(v.index, 3);
    }
    #[test]
    fn simulation_advances_configured_time_without_wall_clock_catchup() {
        let mut app = App::new();
        let mut warp = warp::Warp::default();
        warp.timestep = 0.01;
        app.add_plugins(MinimalPlugins)
            .insert_resource(warp)
            .insert_resource(Flight::spacecraft())
            .insert_resource(Environment::default())
            .insert_resource(SimClock::default())
            .add_systems(Update, step);
        for _ in 0..5 {
            app.update();
        }
        assert!((app.world().resource::<Flight>().elapsed - 0.05).abs() < 1e-9);
        assert!((app.world().resource::<Environment>().time - 0.05).abs() < 1e-9);
        app.world_mut().resource_mut::<Flight>().paused = true;
        app.update();
        assert!((app.world().resource::<Flight>().elapsed - 0.05).abs() < 1e-9);
    }
    #[test]
    fn camera_cycle_contains_exactly_four_flight_views_and_map_is_separate() {
        let mut app = scene(false);
        for expected in [1, 2, 3, 0, 1, 2, 3, 0] {
            {
                let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
                keys.release(KeyCode::KeyC);
                keys.clear();
                keys.press(KeyCode::KeyC);
            }
            app.update();
            assert_eq!(app.world().resource::<ViewState>().index, expected);
            assert!(!app.world().resource::<ViewState>().map);
        }
        *app.world_mut().resource_mut::<Flight>() = Flight::spacecraft();
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::KeyC);
            keys.clear();
            keys.press(KeyCode::KeyM);
        }
        app.update();
        let view = app.world().resource::<ViewState>();
        assert!(view.map);
        assert_eq!(view.index, 0);
        assert!(!app.world().resource::<Flight>().paused);
    }
    #[test]
    fn throttle_shortcuts_work_while_paused_and_cut_has_priority() {
        let mut app = scene(false);
        app.world_mut().resource_mut::<Flight>().paused = true;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ShiftLeft);
        app.update();
        assert_eq!(app.world().resource::<Flight>().throttle, 1.0);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ControlRight);
        app.update();
        assert_eq!(app.world().resource::<Flight>().throttle, 0.0);
        assert_eq!(app.world().resource::<Flight>().thrust, 0.0);
    }
    fn scene(over_button: bool) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<AccumulatedMouseMotion>()
            .init_resource::<AccumulatedMouseScroll>()
            .insert_resource(DirectionalLightShadowMap { size: 1024 })
            .insert_resource(Flight::runway())
            .insert_resource(Environment::default())
            .insert_resource(ViewState::default())
            .insert_resource(warp::Warp::default())
            .add_message::<CursorMoved>()
            .add_systems(Update, input);
        if over_button {
            app.world_mut().spawn((Button, Interaction::Hovered));
        }
        app
    }
    fn drag(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.world_mut().write_message(CursorMoved {
            window: Entity::PLACEHOLDER,
            position: Vec2::new(300.0, 200.0),
            delta: Some(Vec2::new(70.0, -20.0)),
        });
        app.update();
    }
    #[test]
    fn left_drag_uses_cursor_events_and_release_ends_capture() {
        let mut app = scene(false);
        drag(&mut app);
        let view = app.world().resource::<ViewState>();
        assert!(
            (view.yaw + 0.28).abs() < 1e-6
                && (view.pitch - 0.08).abs() < 1e-6
                && view.left_camera_drag
        );
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        app.update();
        assert!(!app.world().resource::<ViewState>().left_camera_drag);
    }
    #[test]
    fn left_drag_starting_on_a_button_does_not_orbit() {
        let mut app = scene(true);
        drag(&mut app);
        let view = app.world().resource::<ViewState>();
        assert_eq!(view.yaw, 0.0);
        assert_eq!(view.pitch, 0.0);
        assert!(!view.left_camera_drag);
    }
    #[test]
    fn cockpit_modifier_interaction_does_not_rotate_camera() {
        let mut app = scene(false);
        app.world_mut().resource_mut::<ViewState>().index = 3;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::AltLeft);
        drag(&mut app);
        let view = app.world().resource::<ViewState>();
        assert_eq!(view.yaw, 0.0);
        assert_eq!(view.pitch, 0.0);
        assert!(!view.left_camera_drag);
    }
}
