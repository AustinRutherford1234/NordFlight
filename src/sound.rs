//! Procedural flight audio with an embedded public-domain A-10 cannon recording.
//! This is a bounded game synthesizer, not a calibrated acoustic/CFD prediction.
use crate::{FlightCamera, ViewState, combat::Combat, physics::*, world};
use bevy::{
    audio::{AddAudioSource, Decodable, Source},
    math::DVec3,
    prelude::*,
};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

const RATE: u32 = 48_000;
const VOICES: usize = 32;
const EMITTERS: usize = 8;
const DREAM_COCKPIT_REDUCTION: f32 = 0.95;
const DREAM_MELODY_LEVEL: f32 = 0.44;
const DREAM_PAD_LEVEL: f32 = 0.16;
// Mono 48 kHz signed little-endian PCM; provenance in assets/audio/README.md.
const CANNON_PCM: &[u8] = include_bytes!("../assets/audio/a10-cannon-body.pcm");

fn cannon_sample(position: f32) -> f32 {
    let length = CANNON_PCM.len() / 2;
    let phase = position.rem_euclid(length as f32);
    let i = phase as usize;
    let sample =
        |n: usize| i16::from_le_bytes([CANNON_PCM[n * 2], CANNON_PCM[n * 2 + 1]]) as f32 / 32768.;
    let a = sample(i);
    a + (sample((i + 1) % length) - a) * phase.fract()
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Cannon,
    Launch,
    Blast,
    Hit,
    Countermeasure,
    Latch,
    Touchdown,
    Switch,
    Ignition,
    Boom,
    Chatter,
}
#[derive(Clone, Copy)]
pub struct Event {
    pub kind: Kind,
    pub position: DVec3,
    pub velocity: DVec3,
    pub own: bool,
}
pub fn emit(queue: &mut VecDeque<Event>, kind: Kind, position: DVec3, velocity: DVec3, own: bool) {
    if queue.len() == 64 {
        queue.pop_front();
    }
    queue.push_back(Event {
        kind,
        position,
        velocity,
        own,
    });
}

#[derive(Resource, Clone, Copy)]
pub struct Settings {
    master: f32,
    engines: f32,
    effects: f32,
    alerts: f32,
    muted: bool,
    open: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            master: 0.65,
            engines: 0.8,
            effects: 0.9,
            alerts: 0.65,
            muted: false,
            open: false,
        }
    }
}
impl Settings {
    pub(crate) fn menu_level(&self) -> f32 { self.master }
    pub(crate) fn menu_muted(&self) -> bool { self.muted }
    pub(crate) fn menu_volume(&mut self, delta:f32) {self.master=(self.master+delta).clamp(0.,1.);self.save();}
    pub(crate) fn menu_mute(&mut self) {self.muted=!self.muted;self.save();}
    fn path() -> std::path::PathBuf {
        if let Some(path) = std::env::var_os("NORD_FLIGHT_AUDIO_SETTINGS") {
            return path.into();
        }
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(std::path::PathBuf::from)
            .unwrap_or_default()
            .join(".nord-flight/audio.json")
    }
    fn from_json(value: &serde_json::Value) -> Self {
        let mut s = Self::default();
        for (key, out) in [
            ("master", &mut s.master),
            ("engines", &mut s.engines),
            ("effects", &mut s.effects),
            ("alerts", &mut s.alerts),
        ] {
            if let Some(x) = value[key].as_f64().filter(|x| x.is_finite()) {
                *out = x.clamp(0., 1.) as f32;
            }
        }
        s.muted = value["muted"].as_bool().unwrap_or(false);
        s
    }
    fn load() -> Self {
        std::fs::read_to_string(Self::path())
            .ok()
            .and_then(|x| serde_json::from_str(&x).ok())
            .map(|v| Self::from_json(&v))
            .unwrap_or_default()
    }
    fn save(&self) {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let data = serde_json::json!({"master":self.master,"engines":self.engines,"effects":self.effects,"alerts":self.alerts,"muted":self.muted});
        let _ = std::fs::write(path, data.to_string());
    }
}
#[derive(Clone, Copy, Default)]
struct Emitter {
    gain: f32,
    pitch: f32,
    pan: f32,
    burner: f32,
    rocket: bool,
}
#[derive(Clone, Copy)]
struct Controls {
    cannon: Option<Trigger>,
    cockpit: bool,
    sources: [Emitter; EMITTERS],
    wind: f32,
    gear: f32,
    rolling: f32,
    apu: f32,
    fire: f32,
    warning: u8,
    lock: f32,
    heat: bool,
    dream: f32,
    settings: Settings,
    paused: bool,
    epoch: u64,
}
impl Default for Controls {
    fn default() -> Self {
        Self {
            cannon: None,
            cockpit: false,
            sources: [Emitter::default(); EMITTERS],
            wind: 0.,
            gear: 0.,
            rolling: 0.,
            apu: 0.,
            fire: 0.,
            warning: 0,
            lock: 0.,
            heat: true,
            dream: 0.,
            settings: Settings::default(),
            paused: true,
            epoch: 0,
        }
    }
}
#[derive(Clone, Copy)]
struct Trigger {
    player: bool,
    kind: Kind,
    gain: f32,
    pan: f32,
    pitch: f32,
}
struct Packet {
    controls: Controls,
    events: VecDeque<Trigger>,
    rendered_frames: u64,
    energy: f64,
}
impl Default for Packet {
    fn default() -> Self {
        Self {
            controls: Controls::default(),
            events: VecDeque::with_capacity(64),
            rendered_frames: 0,
            energy: 0.,
        }
    }
}
#[derive(Resource, Clone, Default)]
pub(crate) struct Bridge(Arc<Mutex<Packet>>);
/// A short presentation voice, mixed through the ordinary volume/mute controls.
pub(crate) fn crew_chatter(bridge: &Bridge, gain: f32, pitch: f32) {
    if let Ok(mut packet) = bridge.0.try_lock() {
        if packet.events.len() < 64 {
            packet.events.push_back(Trigger {
                player: false,
                kind: Kind::Chatter,
                gain,
                pan: 0.,
                pitch,
            });
        }
    }
}
/// Presentation-only dream music, mixed through the normal master and mute.
pub(crate) fn dream_music(bridge: &Bridge, gain: f32, paused: bool) {
    if let Ok(mut packet) = bridge.0.try_lock() {
        packet.controls.dream = if paused { 0. } else { gain.clamp(0., 1.) };
    }
}
struct Pending {
    remaining: f64,
    trigger: Trigger,
}
#[derive(Resource, Default)]
pub struct Scene {
    initialized: bool,
    generation: u64,
    elapsed: f64,
    position: DVec3,
    listener: DVec3,
    vehicle: Option<Vehicle>,
    gear_phase: u8,
    landed: bool,
    engine: bool,
    apu: bool,
    crashed: bool,
    integrity: f64,
    dead: [bool; 11],
    cone: [bool; 12],
    supersonic: [bool; 12],
    camera_view: usize,
    camera_map: bool,
    camera_follow: Option<usize>,
    epoch: u64,
    pending: VecDeque<Pending>,
    dropped: usize,
}

#[derive(Asset, TypePath)]
struct FlightStream {
    bridge: Arc<Mutex<Packet>>,
}
impl Decodable for FlightStream {
    type DecoderItem = f32;
    type Decoder = Mixer;
    fn decoder(&self) -> Mixer {
        Mixer::new(self.bridge.clone())
    }
}
pub struct FlightAudioPlugin;
impl Plugin for FlightAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Bridge>()
            .init_resource::<Scene>()
            .insert_resource(Settings::load())
            .add_audio_source::<FlightStream>()
            .add_systems(Startup, setup);
    }
}
fn setup(
    mut c: Commands,
    bridge: Res<Bridge>,
    mut assets: ResMut<Assets<FlightStream>>,
    preview: Res<crate::VisualCheck>,
    mut settings: ResMut<Settings>,
) {
    if preview.0.as_deref() == Some("air-audio") {
        settings.open = true;
    }
    c.spawn(AudioPlayer(assets.add(FlightStream {
        bridge: bridge.0.clone(),
    })));
    panel(&mut c);
}

fn atmosphere_gain(density: f64, cockpit: bool) -> f32 {
    if cockpit {
        1.0
    } else {
        (density.max(0.) / 1.225).powf(0.25).clamp(0., 1.) as f32
    }
}
fn jet_level(spool: f32) -> f32 {
    // Audible dry thrust with a continuous transition into the wet range.
    let spool = spool.clamp(0., 1.);
    let wet = ((spool - 0.86) / 0.14).clamp(0., 1.);
    let dry = 0.16 + 0.62 * (spool / 0.86).clamp(0., 1.).powf(1.4);
    dry + (1. - dry) * wet
}
fn spatial(
    source: DVec3,
    velocity: DVec3,
    listener: DVec3,
    listener_velocity: DVec3,
    right: DVec3,
    sound: f64,
) -> (f32, f32, f32) {
    let delta = source - listener;
    let range = delta.length();
    let direction = delta.normalize_or_zero();
    let c = sound.max(250.);
    let doppler = ((c + listener_velocity.dot(direction)).clamp(c * 0.25, c * 1.75)
        / (c + velocity.dot(direction)).clamp(c * 0.25, c * 1.75))
    .clamp(0.5, 2.);
    (
        (1. / (1. + range / 65.)) as f32,
        direction.dot(right).clamp(-1., 1.) as f32,
        doppler as f32,
    )
}
fn mach_cone(source: DVec3, velocity: DVec3, listener: DVec3, c: f64) -> bool {
    let mach = velocity.length() / c.max(250.);
    if mach <= 1.02 {
        return false;
    }
    let r = listener - source;
    let axial = -r.dot(velocity.normalize_or_zero());
    axial > 0.
        && (r.length_squared() - axial * axial).max(0.).sqrt() <= axial / (mach * mach - 1.).sqrt()
}
/// Cinematic shock-pressure falloff, distinct from the short-range engine mix.
/// Cabin attenuation applies to other aircraft; occupants never hear their own
/// aircraft's steady shock cone as a fly-by.
fn boom_gain(range: f64, air: f32, cockpit: bool) -> f32 {
    if range > 30_000. {
        return 0.;
    }
    let cabin = if cockpit { 0.45 } else { 1. };
    1.8 * air * cabin / (1. + range.max(0.) / 1500.).sqrt() as f32
}
/// Steady moving-source audibility, with a small spatial/transonic crossfade.
/// mu = asin(1/M); outside the trailing cone no continuous emission arrives.
fn supersonic_gain(source: DVec3, velocity: DVec3, listener: DVec3, sound: f64) -> f32 {
    let mach = velocity.length() / sound.max(250.);
    if mach <= 1. {
        return 1.;
    }
    let offset = listener - source;
    let axial = -offset.dot(velocity.normalize_or_zero());
    let lateral = (offset.length_squared() - axial * axial).max(0.).sqrt();
    let margin = axial - lateral * (mach * mach - 1.).sqrt();
    let edge = (margin / (offset.length() * 0.025).max(0.5)).clamp(0., 1.);
    let inside = edge * edge * (3. - 2. * edge);
    let sonic = ((mach - 1.) / 0.03).clamp(0., 1.);
    (1. - sonic + sonic * inside) as f32
}
fn schedule(
    scene: &mut Scene,
    event: Event,
    _f: &Flight,
    cockpit: bool,
    listener: DVec3,
    velocity: DVec3,
    right: DVec3,
) {
    let (_, _, density, c) = atmosphere(listener.length() - RADIUS);
    let (mut gain, pan, pitch) =
        spatial(event.position, event.velocity, listener, velocity, right, c);
    let air = atmosphere_gain(density, false);
    let structural = event.own && cockpit;
    gain *= if structural { 0.7 } else { air };
    if structural {
        gain = 0.7;
    }
    if gain < 0.002 {
        return;
    }
    let delay = if structural {
        0.
    } else {
        event.position.distance(listener) / c.max(250.)
    };
    if scene.pending.len() == 256 {
        scene.pending.pop_front();
        scene.dropped += 1;
    }
    scene.pending.push_back(Pending {
        remaining: delay,
        trigger: Trigger {
            player: event.own && event.kind == Kind::Cannon,
            kind: event.kind,
            gain,
            pan: if structural { 0. } else { pan },
            pitch,
        },
    });
}

/// Called after physics and the camera. It never changes flight state or its RNG.
pub fn update(
    menu: Option<Res<crate::start_screen::Menu>>,
    f: Res<Flight>,
    v: Res<ViewState>,
    mut combat: ResMut<Combat>,
    settings: Res<Settings>,
    bridge: Res<Bridge>,
    mut scene: ResMut<Scene>,
    cameras: Query<&Transform, With<FlightCamera>>,
) {
    let _measure = crate::performance::span(7);
    if menu.is_some_and(|m| m.open) {
        if let Ok(mut packet) = bridge.0.try_lock() {
            // Keep menu clicks, but do not run an APU/engine loop on the home screen.
            packet.controls = Controls {
                paused: false,
                settings: *settings,
                ..default()
            };
            for e in combat.audio_events.drain(..) {
                if matches!(e.kind, Kind::Switch) {
                    packet.events.push_back(Trigger {
                        kind: Kind::Switch,
                        gain: 0.12,
                        pan: 0.,
                        pitch: 1.,
                        player: false,
                    });
                }
            }
            packet.events.truncate(8);
        }
        return;
    }
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    let cockpit = v.index == 3 && !v.map && !v.following();
    let listener = world::render_origin(&f) + world::frame(&f) * camera.translation.as_dvec3();
    let right = world::frame(&f) * (camera.rotation * Vec3::X).as_dvec3();
    let dt = (f.elapsed - scene.elapsed).max(0.);
    let same_camera = scene.camera_view == v.index
        && scene.camera_map == v.map
        && scene.camera_follow == v.follow_identity();
    let observer_velocity = v
        .aircraft_follow
        .and_then(|id| combat.targets.get(id))
        .filter(|t| t.alive)
        .map(|t| t.velocity)
        .or_else(|| {
            v.missile_follow
                .and_then(|follow| combat.render_missile(follow.missile))
                .filter(|m| m.alive)
                .map(|m| m.velocity)
        })
        .unwrap_or(f.velocity);
    let (_, _, listener_density, sound) = atmosphere(listener.length() - RADIUS);
    let reset = !scene.initialized
        || scene.generation != v.generation
        || scene.vehicle != Some(f.vehicle)
        || f.elapsed < scene.elapsed
        || f.position.distance(scene.position) > 100_000. + f.velocity.length() * dt * 2.;
    if reset {
        scene.epoch = scene.epoch.wrapping_add(1);
        scene.pending.clear();
        scene.cone = [false; 12];
        scene.supersonic = [false; 12];
        scene.dead = std::array::from_fn(|i| combat.targets.get(i).is_none_or(|t| !t.alive));
        combat.audio_events.clear();
    }
    if !reset && !same_camera {
        // Queued arrivals and playing transients belong to the previous observer.
        scene.epoch = scene.epoch.wrapping_add(1);
        scene.pending.clear();
    }
    let listener_velocity = if !reset && same_camera && dt > 1e-6 {
        ((listener - scene.listener) / dt).clamp_length_max(2000.)
    } else {
        observer_velocity
    };
    let mut controls = Controls {
        cockpit,
        paused: f.paused,
        settings: *settings,
        epoch: scene.epoch,
        ..default()
    };
    if combat.cannon_firing(&f) {
        let muzzle = f.position + f.orientation * DVec3::new(0.40, -0.10, -6.0);
        let (gain, pan, pitch) = spatial(
            muzzle,
            f.velocity,
            listener,
            listener_velocity,
            right,
            sound,
        );
        controls.cannon = Some(Trigger {
            kind: Kind::Cannon,
            player: true,
            gain: if cockpit {
                0.7
            } else {
                gain * atmosphere_gain(listener_density, false)
            },
            pan: if cockpit { 0. } else { pan },
            pitch,
        });
    }
    if listener_density < 1e-6 && !cockpit {
        controls.settings.master = 0.;
    }
    if v.map {
        controls.settings.engines *= 0.25;
        controls.settings.effects *= 0.25;
    }
    let air = atmosphere_gain(listener_density, false);
    let audible = atmosphere_gain(listener_density, cockpit);
    let (gain, pan, doppler) = spatial(
        f.position,
        f.velocity,
        listener,
        listener_velocity,
        right,
        sound,
    );
    let exterior = supersonic_gain(f.position, f.velocity - f.wind, listener, sound);
    let rear = f
        .forward()
        .dot((f.position - listener).normalize_or_zero())
        .max(0.);
    let running = f.engine
        && !f.crashed
        && (f.usable_fuel() > 0. || f.infinite_fuel)
        && (f.vehicle == Vehicle::Spacecraft || f.density > 1e-6);
    let spool = f.spool.clamp(0., 1.) as f32;
    controls.sources[0] = Emitter {
        gain: if running {
            audible
                * if cockpit {
                    0.24
                } else {
                    gain * exterior * (0.3 + 0.7 * rear as f32)
                }
                * if f.vehicle == Vehicle::Gripen {
                    jet_level(spool)
                } else {
                    0.12 + 0.88 * spool
                }
        } else {
            0.
        },
        pitch: (0.45 + 1.35 * spool) * if cockpit { 1. } else { doppler },
        burner: if f.vehicle == Vehicle::Gripen {
            ((spool - 0.86) / 0.14).clamp(0., 1.)
        } else {
            (f.thrust / 180000.).clamp(0., 1.) as f32
        },
        pan: if cockpit { 0. } else { pan },
        rocket: f.vehicle == Vehicle::Spacecraft,
    };
    // A spacecraft without thrust has no main-engine vibration.
    if f.vehicle == Vehicle::Spacecraft {
        controls.sources[0].gain *= (f.thrust / 180000.).clamp(0., 1.) as f32;
    }
    for t in &combat.targets {
        let (gain, pan, pitch) = spatial(
            t.position,
            t.velocity,
            listener,
            listener_velocity,
            right,
            sound,
        );
        let index = (1..4)
            .min_by(|a, b| {
                controls.sources[*a]
                    .gain
                    .total_cmp(&controls.sources[*b].gain)
            })
            .unwrap();
        let source = Emitter {
            gain: if t.alive && t.damage.engine > 0.12 && t.fuel > 0. {
                gain * air
                    * 0.85
                    * supersonic_gain(t.position, t.velocity - f.wind, listener, sound)
            } else {
                0.
            },
            pan,
            pitch: pitch * (0.5 + t.throttle as f32),
            burner: ((t.throttle as f32 - 0.86) / 0.14).clamp(0., 1.),
            rocket: false,
        };
        if source.gain > controls.sources[index].gain {
            controls.sources[index] = source;
        }
    }
    // Keep only the four loudest nearby burning motors, not one stream per missile.
    for m in combat
        .missiles_for_draw()
        .filter(|m| m.alive && m.gnc.thrust > 0.)
    {
        let (gain, pan, pitch) = spatial(
            m.position,
            m.velocity,
            listener,
            listener_velocity,
            right,
            sound,
        );
        let slot = (4..EMITTERS)
            .min_by(|a, b| {
                controls.sources[*a]
                    .gain
                    .total_cmp(&controls.sources[*b].gain)
            })
            .unwrap();
        let gain = gain * supersonic_gain(m.position, m.velocity - f.wind, listener, sound);
        if gain * air > controls.sources[slot].gain {
            controls.sources[slot] = Emitter {
                gain: gain * air,
                pan,
                pitch: pitch * 1.6,
                burner: 0.5,
                rocket: true,
            };
        }
    }
    controls.wind = ((f.density * f.airspeed.powi(2) / 65_000.)
        .sqrt()
        .clamp(0., 1.5) as f32)
        * if cockpit { 0.22 } else { 0.10 };
    controls.gear = if f.gear_phase != 0 && f.damage.hydraulics > 0.12 {
        audible * if cockpit { 0.3 } else { 0.16 * gain }
    } else {
        0.
    };
    controls.rolling = if f.landed {
        (f.velocity.length() / 65.).min(1.) as f32 * audible * 0.2
    } else {
        0.
    };
    controls.apu = if f.apu && f.battery && !f.crashed {
        audible * if cockpit { 0.09 } else { 0.015 * gain }
    } else {
        0.
    };
    controls.fire = f.damage.fire as f32 * audible * if cockpit { 0.12 } else { 0.15 * gain };
    if !cockpit {
        // These are airframe sources, not an omnipresent camera wind loop.
        controls.wind *= gain * exterior;
        controls.rolling *= gain * exterior;
        controls.gear *= exterior;
        controls.apu *= exterior;
        controls.fire *= exterior;
        if let Some(cannon) = &mut controls.cannon {
            cannon.gain *= exterior;
        }
    }
    if !v.following() && !v.map && f.battery && !f.crashed && audible > 0. {
        controls.warning = if combat.threats(&f).0 > 0 {
            3
        } else if f.stall() && !f.landed {
            2
        } else if !f.landed && f.agl() < 120. && f.vertical_speed() < -8. {
            1
        } else {
            0
        };
        if combat.active && combat.targets.get(combat.selected).is_some_and(|t| t.alive) {
            controls.lock = (combat.lock
                / if combat.weapon == crate::combat::Weapon::Heat {
                    0.8
                } else {
                    1.3
                })
            .clamp(0., 1.) as f32;
            controls.heat = combat.weapon == crate::combat::Weapon::Heat;
        }
    }
    if !reset && !f.paused {
        let latch = scene.gear_phase != 0 && f.gear_phase == 0;
        let touchdown = f.landed && !scene.landed;
        let engine = f.engine != scene.engine;
        let apu = f.apu != scene.apu;
        let crashed = f.crashed && !scene.crashed;
        let hit = scene.integrity - f.integrity > 0.015;
        let mut own = |kind| {
            schedule(
                &mut scene,
                Event {
                    kind,
                    position: f.position,
                    velocity: f.velocity,
                    own: true,
                },
                &f,
                cockpit,
                listener,
                listener_velocity,
                right,
            )
        };
        if latch {
            own(Kind::Latch);
        }
        if touchdown {
            own(Kind::Touchdown);
        }
        if engine {
            own(if running {
                Kind::Ignition
            } else {
                Kind::Switch
            });
        }
        if apu {
            own(Kind::Switch);
        }
        if crashed && !f.water.active {
            own(Kind::Blast);
        } else if hit {
            own(Kind::Hit);
        }
        while let Some(event) = combat.audio_events.pop_front() {
            if event.own && event.kind == Kind::Cannon && controls.cannon.is_some() {
                continue;
            }
            schedule(
                &mut scene,
                event,
                &f,
                cockpit,
                listener,
                listener_velocity,
                right,
            );
        }
        for position in combat.sound_impacts(dt) {
            if !crashed || position.distance(f.position) > 30. {
                schedule(
                    &mut scene,
                    Event {
                        kind: Kind::Blast,
                        position,
                        velocity: DVec3::ZERO,
                        own: false,
                    },
                    &f,
                    cockpit,
                    listener,
                    listener_velocity,
                    right,
                );
            }
        }
        for (i, t) in combat.targets.iter().take(11).enumerate() {
            if !t.alive
                && !scene.dead[i]
                && !combat
                    .sound_impacts(dt)
                    .any(|p| p.distance(t.position) < 30.)
            {
                schedule(
                    &mut scene,
                    Event {
                        kind: Kind::Blast,
                        position: t.position,
                        velocity: t.velocity,
                        own: false,
                    },
                    &f,
                    cockpit,
                    listener,
                    listener_velocity,
                    right,
                );
            }
        }
    } else {
        combat.audio_events.clear();
    }
    if !f.paused {
        for p in &mut scene.pending {
            p.remaining -= dt;
        }
    }
    // Observers hear a boom when crossing another aircraft's shock cone, even
    // inside a cockpit. Own-aircraft booms remain external-camera only.
    // View switches and first becoming supersonic are not observer crossings.
    for i in 0..12 {
        let source = if i == 0 {
            (f.vehicle == Vehicle::Gripen && !f.crashed).then_some((f.position, f.velocity))
        } else {
            combat
                .targets
                .get(i - 1)
                .filter(|t| t.alive)
                .map(|t| (t.position, t.velocity))
        };
        let inside =
            source.is_some_and(|(p, velocity)| mach_cone(p, velocity - f.wind, listener, sound));
        let supersonic = source
            .is_some_and(|(_, velocity)| (velocity - f.wind).length() > sound.max(250.) * 1.02);
        if (!cockpit || i > 0)
            && !v.map
            && !reset
            && same_camera
            && !f.paused
            && supersonic
            && scene.supersonic[i]
            && inside
            && !scene.cone[i]
            && air > 0.01
        {
            if let Some((p, velocity)) = source {
                let (_, pan, _) = spatial(p, velocity, listener, listener_velocity, right, sound);
                let gain = boom_gain(p.distance(listener), air, cockpit);
                if gain > 0. && scene.pending.len() < 256 {
                    scene.pending.push_back(Pending {
                        remaining: 0.,
                        trigger: Trigger {
                            player: false,
                            kind: Kind::Boom,
                            gain,
                            pan,
                            // A shock impulse does not need the engine's tonal
                            // Doppler pitch shift, especially near Mach one.
                            pitch: 1.,
                        },
                    });
                }
            }
        }
        scene.cone[i] = inside;
        scene.supersonic[i] = supersonic;
    }
    if let Ok(mut packet) = bridge.0.try_lock() {
        if packet.controls.epoch != controls.epoch {
            packet.events.clear();
        }
        packet.controls = controls;
        if !f.paused {
            scene.pending.retain(|p| {
                if p.remaining > 0. {
                    return true;
                }
                if packet.events.len() == 64 {
                    packet.events.pop_front();
                }
                packet.events.push_back(p.trigger);
                false
            });
        }
    }
    scene.initialized = true;
    scene.generation = v.generation;
    scene.vehicle = Some(f.vehicle);
    scene.elapsed = f.elapsed;
    scene.position = f.position;
    scene.listener = listener;
    scene.camera_view = v.index;
    scene.camera_map = v.map;
    scene.camera_follow = v.follow_identity();
    scene.gear_phase = f.gear_phase;
    scene.landed = f.landed;
    scene.engine = f.engine;
    scene.apu = f.apu;
    scene.crashed = f.crashed;
    scene.integrity = f.integrity;
    scene.dead = std::array::from_fn(|i| combat.targets.get(i).is_none_or(|t| !t.alive));
}

#[derive(Clone, Copy)]
struct Voice {
    player: bool,
    kind: Kind,
    life: u32,
    total: u32,
    gain: f32,
    pan: f32,
    pan_gains: [f32;2],
    pitch: f32,
    noise: f32,
    phase: f32,
    sample_position: f32,
    attack: f32,
}
impl Voice {
    fn new(t: Trigger) -> Self {
        let seconds = match t.kind {
            Kind::Blast => 3.,
            Kind::Boom => 0.8,
            Kind::Launch | Kind::Ignition => 1.2,
            Kind::Cannon => 0.16,
            Kind::Chatter => 0.75,
            Kind::Touchdown => 0.65,
            _ => 0.25,
        };
        let total = (seconds * RATE as f32) as u32;
        Self {
            player: t.player,
            kind: t.kind,
            life: total,
            total,
            gain: t.gain,
            pan: t.pan,
            pan_gains: pan_gains(t.pan),
            pitch: t.pitch,
            noise: 0.,
            phase: 0.,
            sample_position: 0.,
            attack: 0.,
        }
    }
}
pub struct Mixer {
    cannon_held: bool,
    bridge: Arc<Mutex<Packet>>,
    controls: Controls,
    emitters: [Emitter; EMITTERS],
    voices: [Option<Voice>; VOICES],
    phase: [f32; EMITTERS],
    rumble_phase: [f32; EMITTERS],
    noise: [f32; EMITTERS],
    exhaust_low: [f32; EMITTERS],
    exhaust_edge: [f32; EMITTERS],
    exhaust_swell: [f32; EMITTERS],
    lut: Box<[f32; 2048]>,
    rng: u64,
    frame: u64,
    right: Option<f32>,
    envelope: f32,
    cabin_low: f32,
    cabin_filter: [f32; 2],
    dream_gain: f32,
    energy: f64,
}
impl Mixer {
    fn new(bridge: Arc<Mutex<Packet>>) -> Self {
        Self {
            cannon_held: false,
            bridge,
            controls: Controls::default(),
            emitters: [Emitter::default(); EMITTERS],
            voices: [None; VOICES],
            phase: [0.; EMITTERS],
            rumble_phase: [0.; EMITTERS],
            noise: [0.; EMITTERS],
            exhaust_low: [0.; EMITTERS],
            exhaust_edge: [0.; EMITTERS],
            exhaust_swell: [0.; EMITTERS],
            lut: Box::new(std::array::from_fn(|i| {
                (i as f32 * std::f32::consts::TAU / 2048.).sin()
            })),
            rng: 0x92835ab4fa,
            frame: 0,
            right: None,
            envelope: 0.,
            cabin_low: 0.,
            cabin_filter: [0.; 2],
            dream_gain: 0.,
            energy: 0.,
        }
    }
    fn random(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        ((self.rng >> 40) as f32 / 8388608.) - 1.
    }
    fn tone(&self, phase: f32) -> f32 {
        self.lut[((phase.fract() * 2048.) as usize) & 2047]
    }
    fn poll(&mut self) {
        if let Ok(mut p) = self.bridge.try_lock() {
            p.rendered_frames = self.frame;
            p.energy = self.energy;
            if self.controls.epoch != p.controls.epoch {
                self.voices.fill(None);
                self.emitters.fill(Emitter::default());
                self.envelope = 0.;
            }
            self.controls = p.controls;
            while let Some(t) = p.events.pop_front() {
                // Keep the recorded burst running while rounds continue arriving.
                // Retriggering its onset every round made the previous gun stutter.
                if t.kind == Kind::Cannon
                    && let Some(v) = self.voices.iter_mut().flatten().find(|v| {
                        v.kind == Kind::Cannon
                            && v.player == t.player
                            && (t.player
                                || ((v.pan - t.pan).abs() < 0.08
                                    && (v.pitch - t.pitch).abs() < 0.08))
                    })
                {
                    v.life = v.total;
                    v.gain = t.gain;
                    v.pan = t.pan;
                    v.pan_gains=pan_gains(t.pan);
                    v.pitch = t.pitch;
                    continue;
                }
                let slot = self
                    .voices
                    .iter()
                    .position(Option::is_none)
                    .unwrap_or_else(|| {
                        self.voices
                            .iter()
                            .enumerate()
                            .min_by_key(|(_, v)| v.unwrap().life)
                            .unwrap()
                            .0
                    });
                self.voices[slot] = Some(Voice::new(t));
            }
        }
        // The audio device runs in wall time; flight may deliberately run slower.
        // Sustain the player's loop from held firing state, not shot-event cadence.
        if let Some(t) = self.controls.cannon {
            if let Some(v) = self.voices.iter_mut().flatten().find(|v| v.player) {
                v.life = v.total;
                v.gain = t.gain;
                v.pan = t.pan;
                v.pan_gains=pan_gains(t.pan);
                v.pitch = t.pitch;
            } else {
                let slot = self.voices.iter().position(Option::is_none).unwrap_or(0);
                self.voices[slot] = Some(Voice::new(t));
            }
        } else if self.cannon_held {
            for v in self.voices.iter_mut().flatten().filter(|v| v.player) {
                v.life = v.life.min((RATE as f32 * 0.045) as u32);
            }
        }
        self.cannon_held = self.controls.cannon.is_some();
    }
    fn stereo(&mut self) -> (f32, f32) {
        if self.frame % 128 == 0 {
            self.poll();
        }
        let target = if self.controls.paused || self.controls.settings.muted {
            0.
        } else {
            self.controls.settings.master
        };
        self.envelope += (target - self.envelope) * (1. / (RATE as f32 * 0.02));
        if self.controls.paused && self.envelope < 0.00005 {
            return (0., 0.);
        }
        let mut left = 0.;
        let mut right = 0.;
        for i in 0..EMITTERS {
            let target = self.controls.sources[i];
            self.emitters[i].gain += (target.gain - self.emitters[i].gain) * 0.00035;
            self.emitters[i].pitch += (target.pitch - self.emitters[i].pitch) * 0.0002;
            self.emitters[i].pan += (target.pan - self.emitters[i].pan) * 0.00035;
            self.emitters[i].burner += (target.burner - self.emitters[i].burner) * 0.00035;
            if target.gain > 0. {
                self.emitters[i].rocket = target.rocket;
            }
            let e = self.emitters[i];
            if e.gain < 0.00001 {
                continue;
            }
            let white = self.random();
            // A spool/jet-velocity frequency proxy shifts the mixing-noise band.
            let center = if e.rocket {
                100. + e.pitch * 60.
            } else {
                60. + e.pitch * 90. + e.burner * 80.
            };
            let rate = std::f32::consts::TAU * center;
            self.noise[i] += (white - self.noise[i]) * (rate / (RATE as f32 + rate));
            self.phase[i] = (self.phase[i] + (180. + e.pitch * 540.) / RATE as f32).fract();
            self.rumble_phase[i] =
                (self.rumble_phase[i] + (45. + e.pitch * 32.) / RATE as f32).fract();
            let tonal = self.tone(self.phase[i]) * 0.08
                + self.tone(self.phase[i] * 2.) * 0.025
                + self.tone(self.phase[i] * 3.) * 0.015;
            let engine = if e.rocket {
                self.noise[i] * 2.5 + white * 0.12
            } else {
                // Broadband turbulent exhaust replaces the burner-dominant hum.
                // Separate bass, body and distorted high-band noise add irregular
                // pressure fluctuations without a repeating pulse or extra voices.
                let wet = e.burner.clamp(0., 1.);
                self.exhaust_low[i] += (white - self.exhaust_low[i]) * 0.007;
                self.exhaust_edge[i] += (white - self.exhaust_edge[i]) * 0.045;
                self.exhaust_swell[i] += (white - self.exhaust_swell[i]) * 0.0006;
                let swell = 1. + (self.exhaust_swell[i] * 18.).clamp(-0.45, 0.45) * wet;
                let edge = self.exhaust_edge[i] - self.noise[i];
                let crackle = edge * edge.abs() * 4.;
                (self.noise[i] * (1.6 + wet * 4.8) + self.exhaust_low[i] * wet * 12.) * swell
                    + crackle * wet * 0.35
                    + white * (0.010 - wet * 0.008)
                    + tonal * (1. - wet * 0.8)
                    + self.tone(self.rumble_phase[i]) * (0.24 + wet * 0.30) * (1. - wet * 0.92)
            };
            pan_mix(
                engine * e.gain * self.controls.settings.engines,
                e.pan,
                &mut left,
                &mut right,
            );
        }
        let white = self.random();
        self.cabin_low += (white - self.cabin_low) * 0.02;
        let t = (self.frame % (RATE as u64 * 16)) as f32 / RATE as f32;
        let mechanical = self.controls.wind * (white * 0.25 + self.cabin_low * 2.)
            + self.controls.gear
                * (white * 0.10 + self.tone(t * 145.) * 0.15 + self.tone(t * 490.) * 0.06)
            + self.controls.rolling * self.cabin_low * 3.
            + self.controls.apu * (self.tone(t * 1170.) * 0.12 + white * 0.05)
            + self.controls.fire * (self.cabin_low * 2. + white * 0.12);
        left += mechanical * self.controls.settings.engines;
        right += mechanical * self.controls.settings.engines;
        for i in 0..VOICES {
            let Some(mut v) = self.voices[i] else {
                continue;
            };
            let white = self.random();
            v.noise += (white - v.noise) * 0.025;
            let age = (v.total - v.life) as f32 / RATE as f32;
            let remaining = v.life as f32 / v.total as f32;
            let (signal, amplitude, frequency) = match v.kind {
                Kind::Cannon => (
                    cannon_sample(v.sample_position),
                    v.attack * (v.life as f32 / (RATE as f32 * 0.045)).min(1.),
                    0.,
                ),
                Kind::Blast => (
                    v.noise * 5. + white * (1. - age / 0.12).max(0.) + self.tone(v.phase) * 0.25,
                    remaining.powi(2),
                    42. + 35. * remaining,
                ),
                Kind::Boom => (
                    v.noise * 4.
                        + if age < 0.10 {
                            0.7
                        } else if age < 0.20 {
                            -0.7
                        } else {
                            0.
                        },
                    remaining.powi(3),
                    50.,
                ),
                Kind::Launch | Kind::Ignition => (
                    v.noise * 3. + white * 0.15,
                    (age / 0.025).min(1.) * remaining.powi(2),
                    80.,
                ),
                Kind::Hit => (
                    white * 0.35 + self.tone(v.phase) * 0.5,
                    remaining.powi(5),
                    620.,
                ),
                Kind::Touchdown => (
                    v.noise * 2. + self.tone(v.phase) * 0.2,
                    remaining.powi(3),
                    160. + 700. * remaining,
                ),
                Kind::Countermeasure => (white * 0.5 + v.noise, remaining.powi(5), 350.),
                Kind::Latch => (
                    v.noise * 2. + self.tone(v.phase) * 0.3,
                    remaining.powi(5),
                    125.,
                ),
                Kind::Switch => (white * 0.3, remaining.powi(10), 1.),
                Kind::Chatter => {
                    // Three rounded, irregular syllables: a small radio-like
                    // canine warble rather than words or an engine loop.
                    let syllable = (age * 5.1).fract();
                    let envelope = (std::f32::consts::PI * syllable).sin().max(0.).powi(2);
                    let vowel = self.tone(v.phase)
                        + self.tone(v.phase * 2.) * 0.32
                        + self.tone(v.phase * 3.) * 0.14
                        + v.noise * 0.15;
                    (
                        vowel * 0.45,
                        envelope * remaining.sqrt(),
                        195. + (age * 21.).sin() * 55. + syllable * 70.,
                    )
                }
            };
            v.phase = (v.phase + frequency * v.pitch / RATE as f32).fract();
            if v.kind == Kind::Cannon {
                v.sample_position =
                    (v.sample_position + v.pitch).rem_euclid((CANNON_PCM.len() / 2) as f32);
                v.attack = (v.attack + 1. / (RATE as f32 * 0.008)).min(1.);
            }
            let sound = signal * amplitude * v.gain * self.controls.settings.effects;
            left+=sound*v.pan_gains[0];
            right+=sound*v.pan_gains[1];
            v.life -= 1;
            self.voices[i] = (v.life > 0).then_some(v);
        }
        if self.controls.cockpit {
            self.cabin_filter[0] += (left - self.cabin_filter[0]) * 0.18;
            self.cabin_filter[1] += (right - self.cabin_filter[1]) * 0.18;
            left = self.cabin_filter[0];
            right = self.cabin_filter[1];
        }
        // Discrete cockpit cues remain recognizable over the cinematic engine mix.
        let warning = match self.controls.warning {
            3 => {
                if (t * 6.).fract() < 0.55 {
                    self.tone(t * 1050.) * 0.16
                } else {
                    0.
                }
            }
            2 => {
                if (t * 3.).fract() < 0.65 {
                    self.tone(t * 420.) * 0.13
                } else {
                    0.
                }
            }
            1 => {
                if (t * 2.).fract() < 0.5 {
                    self.tone(t * 650.) * 0.12
                } else {
                    0.
                }
            }
            _ => {
                if self.controls.lock > 0.02 {
                    self.tone(
                        t * if self.controls.heat {
                            160. + self.controls.lock * 450.
                        } else {
                            880.
                        },
                    ) * if self.controls.lock >= 0.999 || (t * 4.).fract() < self.controls.lock {
                        0.045
                    } else {
                        0.
                    }
                } else {
                    0.
                }
            }
        } * self.controls.settings.alerts;
        left += warning;
        right += warning;
        self.dream_gain += (self.controls.dream - self.dream_gain) * (1. / (RATE as f32 * 0.15));
        if self.dream_gain > 0.00001 {
            // A soft sustained chord sits behind the music-box melody, including
            // the black hold after the photos disappear. Existing ambience is
            // gently lowered so the quiet music remains audible.
            let duck = 1. - DREAM_COCKPIT_REDUCTION * self.dream_gain;
            left *= duck; right *= duck;
            let notes = [261.63, 329.63, 392.0, 329.63, 293.66, 349.23, 440.0, 349.23];
            let phase = (t * 1.25).fract();
            let note = notes[((t * 1.25) as usize) % notes.len()];
            let envelope = (phase * 12.).min(1.) * (-phase * 3.).exp();
            let music = (self.tone(t * note) + 0.25 * self.tone(t * note * 2.))
                * envelope * DREAM_MELODY_LEVEL;
            let pad = (self.tone(t * 130.8125) + self.tone(t * 164.8125) * 0.65
                + self.tone(t * 196.0) * 0.45) * DREAM_PAD_LEVEL;
            let gain = self.dream_gain * self.controls.settings.effects;
            left += (music + pad) * gain;
            right += (music + pad * 0.96) * gain;
        }
        // Bounded soft saturation adds cinematic weight without digital clipping.
        let limit = |x: f32| {
            let x = x * self.envelope;
            x / (1. + x.abs()) * 0.92
        };
        let out = (limit(left), limit(right));
        self.energy += out.0 as f64 * out.0 as f64 + out.1 as f64 * out.1 as f64;
        out
    }
}
fn pan_mix(sample: f32, pan: f32, left: &mut f32, right: &mut f32) {
    *left += sample * ((1. - pan) * 0.5).sqrt();
    *right += sample * ((1. + pan) * 0.5).sqrt();
}
fn pan_gains(pan:f32)->[f32;2]{[((1.-pan)*0.5).sqrt(),((1.+pan)*0.5).sqrt()]}
impl Iterator for Mixer {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if let Some(right) = self.right.take() {
            return Some(right);
        }
        let (left, right) = self.stereo();
        self.frame = self.frame.wrapping_add(1);
        self.right = Some(right);
        Some(left)
    }
}
impl Source for Mixer {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        2
    }
    fn sample_rate(&self) -> u32 {
        RATE
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[derive(Component)]
pub(crate) struct Panel;
#[derive(Component)]
pub(crate) struct AudioButton(u8, f32);
#[derive(Component)]
pub(crate) struct AudioLabel(u8);
fn panel(c: &mut Commands) {
    c.spawn((
        Panel,
        Node {
            position_type: PositionType::Absolute,
            top: px(180),
            right: px(28),
            width: px(300),
            padding: UiRect::all(px(16)),
            flex_direction: FlexDirection::Column,
            row_gap: px(12),
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::srgb(0.025, 0.07, 0.10)),
        GlobalZIndex(100),
    ))
    .with_children(|p| {
        p.spawn((
            Text::new("AUDIO / 9 CLOSE / 0 MUTE"),
            TextFont {
                font_size: 17.,
                ..default()
            },
        ));
        for (i, name) in [
            "MASTER",
            "ENGINES / AIRFRAME",
            "WEAPONS / EFFECTS",
            "WARNINGS",
        ]
        .iter()
        .enumerate()
        {
            p.spawn(Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            })
            .with_children(|row| {
                row.spawn((
                    Text::new(*name),
                    AudioLabel(i as u8),
                    TextFont {
                        font_size: 14.,
                        ..default()
                    },
                ));
                for (label, step) in [("−", -0.1), ("+", 0.1)] {
                    row.spawn((
                        Button,
                        AudioButton(i as u8, step),
                        Node {
                            width: px(28),
                            height: px(26),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.12, 0.25, 0.30)),
                    ))
                    .with_child((
                        Text::new(label),
                        TextFont {
                            font_size: 18.,
                            ..default()
                        },
                    ));
                }
            });
        }
        p.spawn((
            Button,
            AudioButton(4, 0.),
            Node {
                padding: UiRect::all(px(8)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.25, 0.30)),
        ))
        .with_child((
            Text::new("MUTE / UNMUTE"),
            TextFont {
                font_size: 15.,
                ..default()
            },
        ));
    });
}
pub fn ui(
    keys: Res<ButtonInput<KeyCode>>,
    warp: Res<crate::warp::Warp>,
    mut settings: ResMut<Settings>,
    buttons: Query<(&Interaction, &AudioButton), Changed<Interaction>>,
    mut panels: Query<&mut Node, With<Panel>>,
    mut labels: Query<(&AudioLabel, &mut Text)>,
) {
    let mut save = false;
    if !warp.editing {
        if keys.just_pressed(KeyCode::Digit9) {
            settings.open = !settings.open;
        }
        if keys.just_pressed(KeyCode::Digit0) {
            settings.muted = !settings.muted;
            save = true;
        }
    }
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let value = match action.0 {
            0 => &mut settings.master,
            1 => &mut settings.engines,
            2 => &mut settings.effects,
            3 => &mut settings.alerts,
            _ => {
                settings.muted = !settings.muted;
                save = true;
                continue;
            }
        };
        *value = (*value + action.1).clamp(0., 1.);
        save = true;
    }
    if save {
        settings.save();
    }
    if !settings.is_changed() {
        return;
    }
    for mut panel in &mut panels {
        panel.display = if settings.open {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (tag, mut text) in &mut labels {
        let value = [
            settings.master,
            settings.engines,
            settings.effects,
            settings.alerts,
        ][tag.0 as usize];
        let name = ["MASTER", "ENGINES", "EFFECTS", "WARNINGS"][tag.0 as usize];
        let label = format!(
            "{name} {:.0}%{}",
            value * 100.,
            if settings.muted { " MUTED" } else { "" }
        );
        if text.0 != label {
            text.0 = label;
        }
    }
}

pub fn diagnostics(
    bridge: Res<Bridge>,
    preview: Res<crate::VisualCheck>,
    time: Res<Time>,
    mut done: Local<bool>,
) {
    if *done || preview.0.as_deref() != Some("air-audio") || time.elapsed_secs() < 7.0 {
        return;
    }
    if let Ok(p) = bridge.0.try_lock() {
        let report = serde_json::json!({"rendered_stereo_frames":p.rendered_frames,"sample_rate":RATE,
            "output_rms":(p.energy / (p.rendered_frames.max(1) as f64 * 2.)).sqrt(),
            "active_engine_sources":p.controls.sources.iter().filter(|s| s.gain>0.0001).count(),
            "device_is_pulling_stream":p.rendered_frames>0,"paused":p.controls.paused,"muted":p.controls.settings.muted});
        std::fs::write(
            "tests/audio-native-report.json",
            serde_json::to_string_pretty(&report).unwrap(),
        )
        .expect("audio native report");
        *done = true;
    }
}

/// Render the exact game mixer offline, so clips and numerical checks do not
/// depend on a sound device or on externally licensed sample files.
pub fn study() {
    let bridge = Bridge::default();
    let mut mixer = Mixer::new(bridge.0.clone());
    let mut samples = Vec::with_capacity(RATE as usize * 2 * 27);
    let started = std::time::Instant::now();
    let mut peak = 0.0_f32;
    for frame in 0..RATE as usize * 27 {
        if frame % 128 == 0 {
            let t = frame as f32 / RATE as f32;
            let mut p = bridge.0.lock().unwrap();
            let mut c = Controls {
                paused: false,
                ..default()
            };
            c.settings.master = 0.8;
            if t < 7. {
                let spool = (t / 4.).min(1.);
                c.sources[0] = Emitter {
                    gain: jet_level(spool),
                    pitch: 0.5 + 1.3 * spool,
                    burner: ((spool - 0.86) / 0.14).clamp(0., 1.),
                    pan: 0.,
                    rocket: false,
                };
            } else if t < 9. {
                let previous = (frame.saturating_sub(128)) as f32 / RATE as f32;
                if (t * (1. / 0.075)).floor() > (previous * (1. / 0.075)).floor() {
                    p.events.push_back(Trigger {
                        player: false,
                        kind: Kind::Cannon,
                        gain: 1.,
                        pan: 0.,
                        pitch: 1.,
                    });
                }
            } else if t < 12. {
                c.gear = 0.8;
            } else if t < 15. {
                c.sources[4] = Emitter {
                    gain: (1. - ((t - 13.5) / 1.5).abs()).max(0.),
                    pan: ((t - 13.5) / 1.5).clamp(-1., 1.),
                    pitch: 2. - (t - 12.) * 0.4,
                    burner: 0.7,
                    rocket: true,
                };
            } else if t < 19. {
            } else if t < 21. {
                c.warning = 3;
            } else if t < 24. {
                c.cockpit = true;
                c.sources[0] = Emitter {
                    gain: 0.6,
                    pitch: 0.8,
                    pan: 0.,
                    burner: 1.,
                    rocket: true,
                };
            } else if t < 25. {
                c.epoch = 1;
            }
            // external vacuum: silent
            else if t < 26. {
                c.epoch = 1;
                c.paused = true;
            } else {
                c.epoch = 1;
                c.apu = 0.3;
            }
            for (at, kind) in [(11.9, Kind::Latch), (12., Kind::Launch), (15., Kind::Blast)] {
                if t >= at && (frame.saturating_sub(128)) as f32 / (RATE as f32) < at {
                    p.events.push_back(Trigger {
                        player: false,
                        kind,
                        gain: 1.,
                        pan: 0.,
                        pitch: 1.,
                    });
                }
            }
            p.controls = c;
        }
        for _ in 0..2 {
            let value = mixer.next().unwrap();
            assert!(value.is_finite() && value.abs() <= 0.92);
            peak = peak.max(value.abs());
            samples.push((value * i16::MAX as f32).round() as i16);
        }
    }
    let wall = started.elapsed().as_secs_f64();
    let mut bytes = Vec::with_capacity(44 + samples.len() * 2);
    let length = (samples.len() * 2) as u32;
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + length).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&RATE.to_le_bytes());
    bytes.extend_from_slice(&(RATE * 4).to_le_bytes());
    bytes.extend_from_slice(&4u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&length.to_le_bytes());
    for value in &samples {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    std::fs::write("tests/audio-showcase.wav", bytes).expect("save sound showcase");
    let rms = |start: usize, end: usize| {
        let s = &samples[start * RATE as usize * 2..end * RATE as usize * 2];
        (s.iter().map(|v| (*v as f64 / 32767.).powi(2)).sum::<f64>() / s.len() as f64).sqrt()
    };
    let report = serde_json::json!({"sample_rate":RATE,"channels":2,"seconds":27,"render_wall_seconds":wall,"peak":peak,
        "idle_rms":rms(0,1),"afterburner_rms":rms(5,6),"cannon_rms":rms(7,9),"external_vacuum_rms":rms(24,25),"pause_rms":rms(25,26),
        "sections":"0–7 engine spool and afterburner; 7–9 cannon; 9–12 gear; 12–15 launch/flyby; 15–19 explosion; 19–21 missile warning; 21–24 spacecraft cockpit; 24–25 external vacuum; 25–26 paused; 26–27 APU"});
    std::fs::write(
        "tests/audio-study.json",
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .expect("save audio study");
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dream_music_is_audible_and_obeys_mute() {
        assert!((1. - DREAM_COCKPIT_REDUCTION - 0.05).abs() < 1e-6);
        assert!(DREAM_MELODY_LEVEL > 0.15 && DREAM_PAD_LEVEL > 0.055);
        let bridge = Bridge::default();
        { let mut p = bridge.0.lock().unwrap(); p.controls.paused = false; p.controls.dream = 1.; }
        let mut mixer = Mixer::new(bridge.0.clone());
        let energy: f64 = (0..RATE).map(|_| {let (l,r)=mixer.stereo(); mixer.frame += 1; (l*l+r*r) as f64}).sum();
        assert!(energy > 0.01);
        { let mut p = bridge.0.lock().unwrap(); p.controls.settings.muted = true; }
        for _ in 0..RATE { mixer.stereo(); mixer.frame += 1; }
        let (l,r) = mixer.stereo();
        assert!(l.abs() + r.abs() < 0.0001);
    }
    #[test]
    fn dream_reduces_existing_sound_to_five_percent() {
        let render = |dream: f32| {
            let bridge = Bridge::default();
            {
                let mut packet = bridge.0.lock().unwrap();
                packet.controls.paused = false;
                packet.controls.dream = dream;
                packet.controls.settings.effects = 0.; // Isolate the existing engine mix.
                packet.controls.sources[0] = Emitter { gain: 0.2, pitch: 1., ..default() };
            }
            let mut mixer = Mixer::new(bridge.0.clone());
            let mut energy = 0.;
            for frame in 0..RATE * 3 {
                let (l,r) = mixer.stereo();
                mixer.frame += 1;
                if frame >= RATE * 2 { energy += (l*l + r*r) as f64; }
            }
            energy.sqrt()
        };
        let ratio = render(1.) / render(0.);
        assert!((ratio - 0.05).abs() < 0.01, "non-music level ratio {ratio}");
    }
    #[test]
    fn held_cannon_survives_slow_simulation_and_camera_changes_then_releases() {
        let (b, mut m) = mixer();
        b.0.lock().unwrap().controls.cannon = Some(Trigger {
            player: true,
            kind: Kind::Cannon,
            gain: 1.,
            pan: 0.,
            pitch: 1.,
        });
        // Three seconds of device time with no scene update or new shot event.
        // The fixed-step simulation can legitimately run this far behind.
        for _ in 0..30 {
            assert!(energy(&mut m, RATE as usize / 5) > 0.0001);
        }
        let before = m
            .voices
            .iter()
            .flatten()
            .find(|v| v.player)
            .unwrap()
            .sample_position;
        {
            let mut p = b.0.lock().unwrap();
            let t = p.controls.cannon.as_mut().unwrap();
            t.pan = 0.8;
            t.pitch = 0.65;
        }
        m.poll();
        let voice = m.voices.iter().flatten().find(|v| v.player).unwrap();
        assert_eq!(voice.sample_position, before);
        assert_eq!(m.voices.iter().flatten().filter(|v| v.player).count(), 1);
        b.0.lock().unwrap().controls.cannon = None;
        energy(&mut m, RATE as usize / 5);
        assert_eq!(energy(&mut m, RATE as usize / 5), 0.);
    }
    #[test]
    fn cannon_burst_has_no_silent_gaps_between_rounds_and_stops_after_release() {
        let (b, mut m) = mixer();
        let mut windows = Vec::new();
        // Long enough to wrap the recorded loop twice.
        for shot in 0..40 {
            b.0.lock().unwrap().events.push_back(Trigger {
                player: false,
                kind: Kind::Cannon,
                gain: 1.,
                pan: 0.,
                pitch: 1.,
            });
            // Five 15 ms windows span one actual 75 ms firing interval.
            // A 5 ms window can land in a natural trough of the recorded bass.
            for _ in 0..5 {
                let power = energy(&mut m, (RATE as usize * 15 / 1000) * 2);
                if shot >= 2 {
                    windows.push(power);
                }
            }
            assert_eq!(m.voices.iter().flatten().count(), 1);
        }
        let low = windows.iter().copied().fold(f64::INFINITY, f64::min);
        let high = windows.iter().copied().fold(0., f64::max);
        assert!(
            low > high * 0.12,
            "burst envelope has audible gaps: {low}/{high}"
        );
        energy(&mut m, RATE as usize);
        assert_eq!(energy(&mut m, RATE as usize / 10), 0.);
    }
    #[test]
    fn recorded_cannon_loop_is_bounded_and_has_no_abrupt_seam() {
        assert_eq!(CANNON_PCM.len() % 2, 0);
        let count = CANNON_PCM.len() / 2;
        assert!(count > RATE as usize);
        assert!((cannon_sample((count - 1) as f32) - cannon_sample(0.)).abs() < 0.08);
        for i in 0..count {
            assert!(cannon_sample(i as f32).abs() <= 0.851);
        }
    }
    fn mixer() -> (Bridge, Mixer) {
        let b = Bridge::default();
        b.0.lock().unwrap().controls.paused = false;
        (b.clone(), Mixer::new(b.0))
    }
    fn energy(mixer: &mut Mixer, samples: usize) -> f64 {
        (0..samples)
            .map(|_| {
                let x = mixer.next().unwrap();
                assert!(x.is_finite() && x.abs() <= 0.92);
                (x as f64).powi(2)
            })
            .sum::<f64>()
            / samples as f64
    }
    #[test]
    fn stereo_doppler_and_range_follow_relative_geometry() {
        let pos = DVec3::X * 1000.;
        let (near, pan, approaching) = spatial(
            pos,
            -DVec3::X * 150.,
            DVec3::ZERO,
            DVec3::ZERO,
            DVec3::X,
            340.,
        );
        let (far, opposite, departing) = spatial(
            -pos * 2.,
            -DVec3::X * 150.,
            DVec3::ZERO,
            DVec3::ZERO,
            DVec3::X,
            340.,
        );
        assert!(near > far && pan > 0.9 && opposite < -0.9 && approaching > 1. && departing < 1.);
        let (_, _, attached) = spatial(
            pos,
            DVec3::X * 250.,
            DVec3::ZERO,
            DVec3::X * 250.,
            DVec3::X,
            340.,
        );
        assert_eq!(attached, 1.);
        assert!(!mach_cone(
            DVec3::ZERO,
            DVec3::Z * 500.,
            DVec3::Z * 100.,
            340.
        ));
        assert!(mach_cone(
            DVec3::ZERO,
            DVec3::Z * 500.,
            -DVec3::Z * 100.,
            340.
        ));
    }
    #[test]
    fn vacuum_is_silent_externally_but_conducts_own_cockpit_events() {
        let f = Flight::spacecraft();
        let mut s = Scene::default();
        let event = Event {
            kind: Kind::Launch,
            position: f.position,
            velocity: f.velocity,
            own: true,
        };
        assert_eq!(atmosphere_gain(0., false), 0.);
        schedule(&mut s, event, &f, false, f.position, f.velocity, DVec3::X);
        assert!(s.pending.is_empty());
        schedule(&mut s, event, &f, true, f.position, f.velocity, DVec3::X);
        assert_eq!(s.pending.len(), 1);
        assert_eq!(s.pending[0].remaining, 0.);
        assert!(s.pending[0].trigger.gain > 0.);
    }
    #[test]
    fn voices_are_bounded_and_callback_never_waits_for_the_scene_lock() {
        let (b, mut m) = mixer();
        for _ in 0..64 {
            b.0.lock().unwrap().events.push_back(Trigger {
                player: false,
                kind: Kind::Blast,
                gain: 1.,
                pan: 0.,
                pitch: 1.,
            });
        }
        m.poll();
        assert_eq!(m.voices.iter().flatten().count(), VOICES);
        let guard = b.0.lock().unwrap();
        assert!(energy(&mut m, 2000) > 0.);
        drop(guard);
        let mut queue = VecDeque::new();
        for _ in 0..1000 {
            emit(&mut queue, Kind::Cannon, DVec3::ZERO, DVec3::ZERO, false);
        }
        assert_eq!(queue.len(), 64);
    }
    #[test]
    fn supersonic_sound_is_confined_to_a_narrowing_trailing_cone() {
        let source = DVec3::ZERO;
        for mach in [1.1, 1.5, 2., 3.] {
            let velocity = DVec3::X * (340. * mach);
            assert_eq!(supersonic_gain(source, velocity, DVec3::X * 100., 340.), 0.);
            assert_eq!(supersonic_gain(source, velocity, DVec3::Y * 100., 340.), 0.);
            assert_eq!(
                supersonic_gain(source, velocity, -DVec3::X * 100., 340.),
                1.
            );
            let half_angle = (1. / mach).asin();
            for (angle, audible) in [(half_angle * 0.8, true), (half_angle * 1.2, false)] {
                let listener = DVec3::new(-angle.cos(), angle.sin(), 0.) * 1000.;
                let gain = supersonic_gain(source, velocity, listener, 340.);
                assert_eq!(gain > 0.99, audible);
            }
        }
        assert_eq!(
            supersonic_gain(source, DVec3::X * 300., DVec3::X * 100., 340.),
            1.
        );
        // Spatial and transonic boundaries crossfade rather than switching voices.
        let transition = supersonic_gain(source, DVec3::X * 340. * 1.015, DVec3::X * 100., 340.);
        assert!((0.49..0.51).contains(&transition));
    }
    #[test]
    fn dry_engine_is_audible_and_joins_afterburner_continuously() {
        assert!(jet_level(0.7) > 0.60);
        assert!(jet_level(0.85) > 0.75);
        assert!((jet_level(0.8599) - jet_level(0.8601)).abs() < 0.002);
        assert!((jet_level(1.) - 1.).abs() < 1e-6);
    }
    #[test]
    fn external_cone_crossing_booms_once_and_view_switches_do_not() {
        let mut f = Flight::runway();
        f.paused = false;
        f.velocity = f.forward() * 680.;
        let ahead = f.position + f.forward() * 100.;
        let behind = f.position - f.forward() * 100.;
        let mut app = scene(f.clone());
        let set_camera = |app: &mut App, p: DVec3| {
            let pos = world::local(p, &f);
            let mut q = app
                .world_mut()
                .query_filtered::<&mut Transform, With<FlightCamera>>();
            q.single_mut(app.world_mut()).unwrap().translation = pos;
        };
        set_camera(&mut app, ahead);
        app.update();
        set_camera(&mut app, behind);
        app.world_mut().resource_mut::<Flight>().elapsed += 0.01;
        app.update();
        let bridge = app.world().resource::<Bridge>().clone();
        assert_eq!(
            bridge
                .0
                .lock()
                .unwrap()
                .events
                .iter()
                .filter(|e| e.kind == Kind::Boom)
                .count(),
            1
        );
        bridge.0.lock().unwrap().events.clear();
        app.update();
        assert!(
            !bridge
                .0
                .lock()
                .unwrap()
                .events
                .iter()
                .any(|e| e.kind == Kind::Boom)
        );
        set_camera(&mut app, ahead);
        app.update();
        set_camera(&mut app, behind);
        app.world_mut().resource_mut::<ViewState>().index = 1;
        app.world_mut().resource_mut::<Flight>().elapsed += 0.01;
        app.update();
        assert!(
            !bridge
                .0
                .lock()
                .unwrap()
                .events
                .iter()
                .any(|e| e.kind == Kind::Boom)
        );
    }
    #[test]
    fn passing_supersonic_aircraft_booms_in_cockpit_and_exterior_once_per_pass() {
        for cockpit in [false, true] {
            let mut f = Flight::runway();
            f.paused = false;
            let forward = f.forward();
            let right = f.orientation * DVec3::X;
            let mut a = scene(f.clone());
            if cockpit {
                a.world_mut().resource_mut::<ViewState>().index = 3;
            }
            {
                let mut c = a.world_mut().resource_mut::<Combat>();
                c.start(&f, 1);
                for t in &mut c.targets {
                    t.alive = false;
                }
                c.targets[0].alive = true;
                c.targets[0].velocity = forward * 680.;
                c.targets[0].position = f.position - forward * 2000. + right * 1000.;
            }
            a.update();
            let bridge = a.world().resource::<Bridge>().clone();
            assert!(bridge.0.lock().unwrap().events.is_empty());
            // Move the source past a stationary observer, rather than rotating
            // the camera around a stationary supersonic aircraft.
            for pass in 0..2 {
                a.world_mut().resource_mut::<Combat>().targets[0].position =
                    f.position + forward * 2500. + right * 1000.;
                a.world_mut().resource_mut::<Flight>().elapsed += 0.1;
                a.update();
                {
                    let p = bridge.0.lock().unwrap();
                    let booms: Vec<_> = p.events.iter().filter(|e| e.kind == Kind::Boom).collect();
                    assert_eq!(booms.len(), 1, "cockpit={cockpit}, pass={pass}");
                    assert!(booms[0].gain > 0.4);
                    assert_eq!(booms[0].pitch, 1.);
                }
                bridge.0.lock().unwrap().events.clear();
                a.world_mut().resource_mut::<Flight>().elapsed += 0.1;
                a.update();
                assert!(bridge.0.lock().unwrap().events.is_empty());
                a.world_mut().resource_mut::<Combat>().targets[0].position =
                    f.position - forward * 2000. + right * 1000.;
                a.world_mut().resource_mut::<Flight>().elapsed += 0.1;
                a.update();
            }
        }
    }
    #[test]
    fn subsonic_flyby_and_own_cockpit_do_not_boom() {
        let mut f = Flight::runway();
        f.paused = false;
        let forward = f.forward();
        let mut a = scene(f.clone());
        {
            let mut c = a.world_mut().resource_mut::<Combat>();
            c.start(&f, 1);
            for t in &mut c.targets {
                t.alive = false;
            }
            c.targets[0].alive = true;
            c.targets[0].velocity = forward * 250.;
            c.targets[0].position = f.position - forward * 2000.;
        }
        a.update();
        a.world_mut().resource_mut::<Combat>().targets[0].position = f.position + forward * 2500.;
        a.world_mut().resource_mut::<Flight>().elapsed += 0.1;
        a.update();
        assert!(
            !a.world()
                .resource::<Bridge>()
                .0
                .lock()
                .unwrap()
                .events
                .iter()
                .any(|e| e.kind == Kind::Boom)
        );
        // The same camera geometry cannot manufacture an own-aircraft boom
        // while seated inside its cockpit.
        a.world_mut().resource_mut::<ViewState>().index = 3;
        a.world_mut().resource_mut::<Flight>().velocity = forward * 680.;
        for offset in [100., -100.] {
            let mut q = a
                .world_mut()
                .query_filtered::<&mut Transform, With<FlightCamera>>();
            q.single_mut(a.world_mut()).unwrap().translation =
                world::local(f.position + forward * offset, &f);
            a.world_mut().resource_mut::<Flight>().elapsed += 0.1;
            a.update();
        }
        assert!(
            !a.world()
                .resource::<Bridge>()
                .0
                .lock()
                .unwrap()
                .events
                .iter()
                .any(|e| e.kind == Kind::Boom)
        );
    }
    #[test]
    fn boom_has_longer_range_than_engine_and_is_muffled_in_cabin() {
        assert!(boom_gain(5000., 1., false) > 0.8);
        assert!(boom_gain(5000., 1., true) < boom_gain(5000., 1., false));
        assert!(boom_gain(1000., 1., false) > boom_gain(5000., 1., false));
        assert_eq!(boom_gain(31_000., 1., false), 0.);
        assert_eq!(boom_gain(1000., 0., false), 0.);
        let power = |cockpit| {
            let (bridge, mut mixer) = mixer();
            bridge.0.lock().unwrap().events.push_back(Trigger {
                player: false,
                kind: Kind::Boom,
                gain: boom_gain(5000., 1., cockpit),
                pan: 0.,
                pitch: 1.,
            });
            energy(&mut mixer, RATE as usize)
        };
        let outside = power(false);
        let cabin = power(true);
        assert!(outside > 0.0001);
        assert!(cabin > 0.00001 && cabin < outside);
    }
    #[test]
    fn afterburner_roar_is_broadband_rather_than_a_dominant_fan_tone() {
        let (b, mut m) = mixer();
        b.0.lock().unwrap().controls.sources[0] = Emitter {
            gain: 1.,
            pitch: 1.8,
            burner: 1.,
            ..default()
        };
        energy(&mut m, RATE as usize * 2);
        let samples: Vec<f64> = (0..RATE)
            .map(|_| {
                let left = m.next().unwrap() as f64;
                let right = m.next().unwrap() as f64;
                (left + right) * 0.5
            })
            .collect();
        let power = samples.iter().map(|x| x * x).sum::<f64>() / samples.len() as f64;
        let tone_power = |frequency: f64| {
            let (mut re, mut im) = (0., 0.);
            for (i, x) in samples.iter().enumerate() {
                let phase = std::f64::consts::TAU * frequency * i as f64 / RATE as f64;
                re += x * phase.cos();
                im += x * phase.sin();
            }
            2. * (re * re + im * im) / (samples.len() as f64).powi(2)
        };
        assert!(power > 0.01);
        assert!((tone_power(102.6) + tone_power(1152.)) / power < 0.08);
    }
    #[test]
    fn engine_spool_and_burner_raise_energy_and_shutdown_fades_out() {
        let (b, mut m) = mixer();
        b.0.lock().unwrap().controls.sources[0] = Emitter {
            gain: 0.2,
            pitch: 0.5,
            ..default()
        };
        energy(&mut m, RATE as usize);
        let idle = energy(&mut m, RATE as usize);
        b.0.lock().unwrap().controls.sources[0] = Emitter {
            gain: 1.,
            pitch: 1.8,
            burner: 1.,
            ..default()
        };
        energy(&mut m, RATE as usize);
        let wet = energy(&mut m, RATE as usize);
        assert!(wet > idle * 3.);
        b.0.lock()
            .unwrap()
            .controls
            .sources
            .fill(Emitter::default());
        energy(&mut m, RATE as usize);
        assert!(energy(&mut m, RATE as usize) < 1e-8);
    }
    #[test]
    fn pause_and_mute_are_silent_and_reset_discards_old_effects() {
        let (b, mut m) = mixer();
        b.0.lock().unwrap().events.push_back(Trigger {
            player: false,
            kind: Kind::Blast,
            gain: 1.,
            pan: 0.,
            pitch: 1.,
        });
        assert!(energy(&mut m, RATE as usize / 2) > 0.);
        b.0.lock().unwrap().controls.paused = true;
        energy(&mut m, RATE as usize);
        assert!(energy(&mut m, RATE as usize) < 1e-8);
        {
            let mut p = b.0.lock().unwrap();
            p.controls.paused = false;
            p.controls.epoch += 1;
            p.controls.settings.muted = true;
        }
        energy(&mut m, 256);
        assert!(m.voices.iter().all(Option::is_none));
        assert!(energy(&mut m, RATE as usize) < 1e-8);
    }
    #[test]
    fn settings_are_clamped_and_missing_fields_use_defaults() {
        let s = Settings::from_json(
            &serde_json::json!({"master":4.,"engines":-3.,"effects":"bad","muted":true}),
        );
        assert_eq!(s.master, 1.);
        assert_eq!(s.engines, 0.);
        assert_eq!(s.effects, 0.9);
        assert!(s.muted);
    }
    #[test]
    fn safe_high_warp_coast_does_not_reset_the_audio_scene() {
        let mut a = scene(Flight::spacecraft());
        a.world_mut().resource_mut::<ViewState>().index = 3;
        a.update();
        let epoch = a.world().resource::<Scene>().epoch;
        {
            let mut f = a.world_mut().resource_mut::<Flight>();
            let velocity = f.velocity;
            f.position += velocity * 40.;
            f.elapsed += 40.;
        }
        a.update();
        assert_eq!(a.world().resource::<Scene>().epoch, epoch);
    }
    #[test]
    fn spacecraft_main_engine_is_only_heard_in_a_powered_cockpit_in_vacuum() {
        let mut a = scene(Flight::spacecraft());
        a.world_mut().resource_mut::<ViewState>().index = 3;
        a.update();
        let b = a.world().resource::<Bridge>().clone();
        assert_eq!(b.0.lock().unwrap().controls.sources[0].gain, 0.);
        {
            let mut f = a.world_mut().resource_mut::<Flight>();
            f.thrust = 90000.;
            f.spool = 0.8;
            f.elapsed += 0.01;
        }
        a.update();
        assert!(b.0.lock().unwrap().controls.sources[0].gain > 0.);
        a.world_mut().resource_mut::<ViewState>().index = 0;
        a.update();
        let p = b.0.lock().unwrap();
        assert_eq!(p.controls.sources[0].gain, 0.);
        assert_eq!(p.controls.settings.master, 0.);
    }
    #[test]
    fn audio_panel_key_works_without_intercepting_warp_numeric_entry() {
        let mut a = App::new();
        a.init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(crate::warp::Warp::default())
            .insert_resource(Settings::default())
            .add_systems(Update, ui);
        let panel = a.world_mut().spawn((Panel, Node::default())).id();
        a.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Digit9);
        a.update();
        assert!(a.world().resource::<Settings>().open);
        assert_eq!(a.world().get::<Node>(panel).unwrap().display, Display::Flex);
        a.world_mut().resource_mut::<crate::warp::Warp>().editing = true;
        a.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        a.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Digit9);
        a.update();
        assert!(a.world().resource::<Settings>().open);
    }
    fn scene(f: Flight) -> App {
        let mut a = App::new();
        let camera = Transform::from_translation(world::local(f.position, &f));
        a.insert_resource(f)
            .insert_resource(Combat::default())
            .insert_resource(ViewState::default())
            .insert_resource(Settings::default())
            .init_resource::<Bridge>()
            .init_resource::<Scene>()
            .add_systems(Update, update);
        a.world_mut().spawn((FlightCamera, camera));
        a
    }
    #[test]
    fn aircraft_and_missile_camera_switches_relocate_audio_without_doppler_spikes() {
        let f = crate::combat::flight();
        let mut combat = Combat::default();
        combat.start(&f, 1);
        combat.lock = 1.;
        assert!(combat.fire(&f));
        // The motor ignites after release in the simulation.
        combat.missiles[0].gnc.thrust = 10_000.;
        combat.targets.truncate(1);
        let mut a = scene(f.clone());
        a.world_mut().insert_resource(combat);
        a.world_mut().insert_resource(crate::warp::Warp::default());
        a.world_mut().init_resource::<ButtonInput<KeyCode>>();
        a.add_systems(Update, crate::combat::camera_controls.before(update));
        a.update();
        let initial = a.world().resource::<Bridge>().0.lock().unwrap().controls;
        for key in [KeyCode::Digit2, KeyCode::Digit6, KeyCode::Digit7] {
            // Place the camera at the same trailing offset as a following view.
            let (position, velocity) = {
                let c = a.world().resource::<Combat>();
                match key {
                    KeyCode::Digit2 => (c.targets[0].position, c.targets[0].velocity),
                    KeyCode::Digit6 => (c.missiles[0].position, c.missiles[0].velocity),
                    _ => (f.position, f.velocity),
                }
            };
            let listener = position - velocity.normalize() * 30.;
            let mut query = a
                .world_mut()
                .query_filtered::<&mut Transform, With<FlightCamera>>();
            query.single_mut(a.world_mut()).unwrap().translation = world::local(listener, &f);
            {
                let mut keys = a.world_mut().resource_mut::<ButtonInput<KeyCode>>();
                keys.reset_all();
                keys.press(key);
            }
            a.world_mut().resource_mut::<Flight>().elapsed += 0.01;
            a.update();
            let controls = a.world().resource::<Bridge>().0.lock().unwrap().controls;
            assert!((a.world().resource::<Scene>().listener - listener).length() < 1.);
            if key == KeyCode::Digit2 {
                assert!(controls.sources[1].gain > initial.sources[1].gain * 5.);
                let throttle = a.world().resource::<Combat>().targets[0].throttle as f32;
                assert!((controls.sources[1].pitch - (0.5 + throttle)).abs() < 0.001);
            }
            if key == KeyCode::Digit6 {
                assert!((controls.sources[4].pitch - 1.6).abs() < 0.001);
            }
            if key != KeyCode::Digit7 {
                assert_eq!(controls.warning, 0);
                assert_eq!(controls.lock, 0.);
            }
        }
    }
    #[test]
    fn exterior_atmosphere_follows_camera_instead_of_player_altitude() {
        let f = Flight::runway();
        let mut a = scene(f.clone());
        a.update();
        let initial = a.world().resource::<Bridge>().0.lock().unwrap().controls;
        assert!(initial.settings.master > 0.);
        let listener = f.position.normalize() * (RADIUS + 250_000.);
        let mut query = a
            .world_mut()
            .query_filtered::<&mut Transform, With<FlightCamera>>();
        query.single_mut(a.world_mut()).unwrap().translation = world::local(listener, &f);
        a.update();
        let controls = a.world().resource::<Bridge>().0.lock().unwrap().controls;
        assert_eq!(controls.settings.master, 0.);
    }
    #[test]
    fn camera_angle_mutes_supersonic_exterior_but_preserves_cockpit_and_blasts() {
        for (ahead, cockpit) in [(true, false), (false, false), (true, true)] {
            let mut f = Flight::runway();
            f.velocity = f.forward() * 680.;
            f.airspeed = 680.;
            f.spool = 1.;
            f.engine = true;
            let listener = f.position + f.forward() * if ahead { 100. } else { -100. };
            let camera = world::local(listener, &f);
            let mut a = scene(f.clone());
            if cockpit {
                a.world_mut().resource_mut::<ViewState>().index = 3;
            }
            let mut query = a
                .world_mut()
                .query_filtered::<&mut Transform, With<FlightCamera>>();
            query.single_mut(a.world_mut()).unwrap().translation = camera;
            a.update();
            let controls = a.world().resource::<Bridge>().0.lock().unwrap().controls;
            if ahead && !cockpit {
                assert_eq!(controls.sources[0].gain, 0.);
                assert_eq!(controls.wind, 0.);
                emit(
                    &mut a.world_mut().resource_mut::<Combat>().audio_events,
                    Kind::Blast,
                    listener + DVec3::X * 100.,
                    DVec3::ZERO,
                    false,
                );
                a.world_mut().resource_mut::<Flight>().elapsed += 0.01;
                a.update();
                assert!(
                    a.world()
                        .resource::<Scene>()
                        .pending
                        .iter()
                        .any(|p| p.trigger.kind == Kind::Blast && p.trigger.gain > 0.)
                );
            } else {
                assert!(controls.sources[0].gain > 0.);
            }
        }
    }
    #[test]
    fn gear_sound_follows_actual_motion_and_latch_is_emitted_once() {
        let mut a = scene(Flight::runway());
        a.update();
        {
            let mut f = a.world_mut().resource_mut::<Flight>();
            f.gear_phase = 1;
            f.elapsed += 0.01;
        }
        a.update();
        assert!(
            a.world()
                .resource::<Bridge>()
                .0
                .lock()
                .unwrap()
                .controls
                .gear
                > 0.
        );
        {
            let mut f = a.world_mut().resource_mut::<Flight>();
            f.gear_phase = 0;
            f.elapsed += 0.01;
        }
        a.update();
        let b = a.world().resource::<Bridge>().clone();
        {
            let mut p = b.0.lock().unwrap();
            assert_eq!(p.controls.gear, 0.);
            assert_eq!(p.events.iter().filter(|e| e.kind == Kind::Latch).count(), 1);
            p.events.clear();
        }
        a.update();
        assert!(b.0.lock().unwrap().events.is_empty());
    }
    #[test]
    fn atmospheric_transients_wait_for_simulated_propagation_time_and_pause() {
        let mut f = Flight::runway();
        f.engine = false;
        f.apu = false;
        let mut a = scene(f.clone());
        a.update();
        emit(
            &mut a.world_mut().resource_mut::<Combat>().audio_events,
            Kind::Blast,
            f.position + DVec3::X * 340.,
            DVec3::ZERO,
            false,
        );
        a.world_mut().resource_mut::<Flight>().elapsed += 0.01;
        a.update();
        assert!(
            a.world()
                .resource::<Bridge>()
                .0
                .lock()
                .unwrap()
                .events
                .is_empty()
        );
        a.world_mut().resource_mut::<Flight>().paused = true;
        for _ in 0..20 {
            a.update();
        }
        assert_eq!(a.world().resource::<Scene>().pending.len(), 1);
        {
            let mut f = a.world_mut().resource_mut::<Flight>();
            f.paused = false;
            f.elapsed += 1.2;
        }
        a.update();
        assert_eq!(
            a.world()
                .resource::<Bridge>()
                .0
                .lock()
                .unwrap()
                .events
                .len(),
            1
        );
    }
}
