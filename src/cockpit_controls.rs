//! Labelled cockpit switches, using the same physical faces for rendering and picking.
use crate::{
    FlightCamera, ViewState,
    cockpit::InternalDisplay,
    combat::{
        Combat, Weapon as MissileWeapon,
        bvr::{Menu, cockpit_radar},
    },
    physics::*,
    world,
};
use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Action {
    Battery,
    Apu,
    Engine,
    Lights,
    Gear,
    Flaps,
    Airbrake,
    Fbw,
    Target,
    Weapon,
    Countermeasures,
    Simple,
    Autopilot,
    Help,
    RadarPower,
    RadarMode,
    Stovl,
    Bays,
    Dump,
    EngineL,
    EngineR,
}
#[derive(Component)]
pub struct Key {
    action: Action,
    position: Vec3,
    size: Vec3,
    rotation: Quat,
}
#[derive(Component)]
pub struct Lever {
    action: Action,
    rotation: Quat,
}
struct Spec {
    action: Action,
    label: &'static str,
    position: Vec3,
    size: Vec3,
    rotation: Quat,
    toggle: bool,
}
fn side_layout() -> Vec<Spec> {
    use Action::*;
    let mut specs = Vec::new();
    for (side, actions) in [
        (
            -1.,
            [
                (Battery, "BAT", true),
                (Lights, "LIGHTS", true),
                (Apu, "APU", true),
                (Engine, "ENGINE", true),
                (Gear, "GEAR", true),
                (Flaps, "FLAPS", false),
                (Airbrake, "AIRBRK", true),
                (Fbw, "FBW", true),
            ],
        ),
        (
            1.,
            [
                (RadarPower, "RADAR", true),
                (RadarMode, "R-MODE", false),
                (Target, "TARGET", false),
                (Weapon, "WEAPON", true),
                (Countermeasures, "FLR/CH", false),
                (Autopilot, "APPR", true),
                (Simple, "SIMPLE", true),
                (Help, "HELP", false),
            ],
        ),
    ] {
        for (i, (action, label, toggle)) in actions.into_iter().enumerate() {
            let row = i / 2;
            let z = if side < 0. {
                [-3.26, -3.12, -2.50, -2.30][row]
            } else {
                -3.14 + row as f32 * 0.20
            };
            specs.push(Spec {
                action,
                label,
                toggle,
                position: Vec3::new(side * (0.32 + (i % 2) as f32 * 0.10), 0.447, z),
                size: Vec3::new(0.080, 0.10, 0.012),
                rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
            });
        }
    }
    specs
}
fn layout() -> Vec<(Action, &'static str, Vec3, Vec3)> {
    use Action::*;
    let mut keys = Vec::new();
    for (x, actions) in [
        (
            -0.425,
            [
                (Battery, "BAT"),
                (Apu, "APU"),
                (Engine, "ENG"),
                (Lights, "LTS"),
            ],
        ),
        (
            -0.135,
            [
                (Gear, "GEAR"),
                (Flaps, "FLAP"),
                (Airbrake, "ABRK"),
                (Fbw, "FBW"),
            ],
        ),
    ] {
        for (i, (action, label)) in actions.into_iter().enumerate() {
            keys.push((
                action,
                label,
                Vec3::new(x, 0.59 - i as f32 * 0.04, -3.42),
                Vec3::new(0.024, 0.034, 0.012),
            ));
        }
    }
    for (i, (action, label)) in [
        (Target, "TARGET"),
        (Weapon, "WEAP"),
        (Countermeasures, "CM"),
        (Simple, "SIMP"),
        (Autopilot, "AP"),
        (Help, "HELP"),
    ]
    .into_iter()
    .enumerate()
    {
        keys.push((
            action,
            label,
            Vec3::new(-0.28 + i as f32 * 0.092, 0.34, -3.42),
            Vec3::new(0.065, 0.034, 0.012),
        ));
    }
    keys
}
pub fn setup(
    c: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    root: Entity,
    aircraft: crate::aircraft::Aircraft,
) {
    let cube = meshes.add(Cuboid::default());
    let socket = mats.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.24, 0.26),
        metallic: 0.7,
        perceptual_roughness: 0.4,
        ..default()
    });
    let metal = mats.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.48, 0.49),
        metallic: 0.8,
        perceptual_roughness: 0.28,
        ..default()
    });
    let grip = mats.add(StandardMaterial {
        base_color: Color::srgb(0.70, 0.53, 0.23),
        perceptual_roughness: 0.45,
        ..default()
    });
    let mut lever_mesh = Mesh::from(Cylinder::new(0.0045, 0.032));
    if let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) =
        lever_mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        for p in p {
            p[1] += 0.016;
        }
    }
    let lever_mesh = meshes.add(lever_mesh);
    let knob = meshes.add(Sphere::new(0.007).mesh().uv(12, 8));
    for side in [-1., 1.] {
        let tray = c
            .spawn((
                Mesh3d(cube.clone()),
                MeshMaterial3d(socket.clone()),
                Transform::from_xyz(side * 0.37, 0.432, -2.70)
                    .with_scale(Vec3::new(0.22, 0.020, 1.28)),
                InternalDisplay,
            ))
            .id();
        c.entity(root).add_child(tray);
    }
    let front = layout()
        .into_iter()
        .map(|(action, label, position, size)| Spec {
            action,
            label,
            position,
            size,
            rotation: Quat::IDENTITY,
            toggle: false,
        });
    let mut specs: Vec<_> = front.chain(side_layout()).collect();
    if aircraft==crate::aircraft::Aircraft::Hornet {specs.retain(|s|s.action!=Action::Autopilot);}
    if aircraft!=crate::aircraft::Aircraft::Gripen {
        let mut extra=vec![(Action::EngineL,"ENG L"),(Action::Dump,"JETT")];
        if aircraft.spec().engines==2{extra.push((Action::EngineR,"ENG R"));}
        if aircraft==crate::aircraft::Aircraft::F35B{extra.push((Action::Stovl,"STOVL"));}
        if matches!(aircraft,crate::aircraft::Aircraft::F22|crate::aircraft::Aircraft::F35B){extra.push((Action::Bays,"BAYS"));}
        for(i,(action,label))in extra.into_iter().enumerate(){specs.push(Spec{action,label,position:Vec3::new(0.32+(i%2)as f32*0.10,0.447,-2.18+(i/2)as f32*0.16),size:Vec3::new(0.08,0.1,0.012),rotation:Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),toggle:true});}
    }
    let mut atlas =
        crate::cockpit_labels::FaceAtlas::new(specs.iter().map(|s| s.label), images, mats);
    for Spec {
        action,
        label,
        position,
        size,
        rotation,
        toggle,
    } in specs
    {
        let rim = c
            .spawn((
                Mesh3d(cube.clone()),
                MeshMaterial3d(socket.clone()),
                Transform::from_translation(position - rotation * Vec3::Z * 0.008)
                    .with_scale(size + Vec3::new(0.004, 0.004, 0.002))
                    .with_rotation(rotation),
                InternalDisplay,
            ))
            .id();
        c.entity(root).add_child(rim);
        let cap = mats.add(StandardMaterial {
            base_color: Color::srgb(0.035, 0.065, 0.085),
            perceptual_roughness: 0.42,
            metallic: 0.35,
            ..default()
        });
        let key = c
            .spawn((
                Mesh3d(cube.clone()),
                MeshMaterial3d(cap),
                Transform::from_translation(position)
                    .with_scale(size)
                    .with_rotation(rotation),
                crate::aircraft::visual::AircraftPart(aircraft),
                Key {
                    action,
                    position,
                    size,
                    rotation,
                },
                InternalDisplay,
            ))
            .id();
        c.entity(root).add_child(key);
        let label = c
            .spawn((
                Mesh3d(atlas.face(label, meshes)),
                MeshMaterial3d(atlas.material.clone()),
                Transform::from_xyz(0., if toggle { -0.14 } else { 0. }, 0.007 / size.z),
                InternalDisplay,
                bevy::light::NotShadowCaster,
            ))
            .id();
        c.entity(key).add_child(label);
        if toggle {
            let lever = c
                .spawn((
                    Mesh3d(lever_mesh.clone()),
                    MeshMaterial3d(metal.clone()),
                    Transform::from_translation(position + rotation * Vec3::new(0., 0.025, 0.009))
                        .with_rotation(rotation),
                    Lever { action, rotation },
                    crate::aircraft::visual::AircraftPart(aircraft),
                    InternalDisplay,
                ))
                .id();
            let tip = c
                .spawn((
                    Mesh3d(knob.clone()),
                    MeshMaterial3d(grip.clone()),
                    Transform::from_xyz(0., 0.032, 0.),
                    InternalDisplay,
                ))
                .id();
            c.entity(lever).add_child(tip);
            c.entity(root).add_child(lever);
        }
    }
}
fn hit(
    origin: bevy::math::DVec3,
    direction: bevy::math::DVec3,
    position: Vec3,
    size: Vec3,
) -> bool {
    if direction.z.abs() < 1e-9 {
        return false;
    }
    let distance = (position.z as f64 + 0.007 - origin.z) / direction.z;
    if distance <= 0. {
        return false;
    }
    let p = origin + direction * distance - position.as_dvec3();
    p.x.abs() <= size.x as f64 * 0.5 + 0.001 && p.y.abs() <= size.y as f64 * 0.5 + 0.001
}
fn active(a: Action, f: &Flight, v: &ViewState) -> bool {
    match a {
        Action::Battery => f.battery,
        Action::Apu => f.apu,
        Action::Engine => f.engine,
        Action::Lights => f.lights,
        Action::Gear => f.gear,
        Action::Flaps => f.flaps > 0.,
        Action::Airbrake => f.airbrake,
        Action::Fbw => f.assist,
        Action::Simple => v.simple,
        Action::Autopilot => f.autopilot,
        Action::Help => v.help,
        Action::Stovl=>f.systems.conversion_target,
        Action::Bays=>f.systems.bays_target,
        Action::EngineL=>f.systems.engine_on[0],
        Action::EngineR=>f.systems.engine_on[1],
        _ => false,
    }
}
fn apply(a: Action, f: &mut Flight, v: &mut ViewState, c: &mut Combat) -> &'static str {
    use Action::*;
    match a {
        Stovl=>{if f.airspeed>150.||f.damage.lift_fan<0.15{return "STOVL INHIBITED / SPEED OR LIFT FAN";}f.systems.conversion_target=!f.systems.conversion_target;"STOVL CONVERSION"},
        Bays=>{f.systems.manual_bays=!f.systems.manual_bays;f.systems.bays_target=f.systems.manual_bays;"WEAPON BAY DOORS"},
        Dump=>{f.systems.stores=[false;10];c.ammo=[0;2];"STORES JETTISONED"},
        EngineL=>{f.systems.engine_on[0]=!f.systems.engine_on[0];"LEFT ENGINE FUEL VALVE"},
        EngineR=>{f.systems.engine_on[1]=!f.systems.engine_on[1];"RIGHT ENGINE FUEL VALVE"},
        Battery => {
            f.battery = !f.battery;
            "BATTERY POWER"
        }
        Apu => {
            if !f.battery {
                return "APU REQUIRES BATTERY";
            }
            f.apu = !f.apu;
            "AUXILIARY POWER UNIT"
        }
        Engine => {
            if !f.battery || !f.apu {
                return "ENGINE START / STOP REQUIRES BATTERY + APU";
            }
            f.engine = !f.engine;
            if !f.engine {
                f.thrust = 0.;
            }
            "ENGINE START / STOP"
        }
        Lights => {
            f.lights = !f.lights;
            "EXTERIOR LIGHTS"
        }
        Gear => {
            f.gear = !f.gear;
            "LANDING GEAR"
        }
        Flaps => {
            f.flaps = if f.flaps < 0.2 {
                0.55
            } else if f.flaps < 0.8 {
                1.
            } else {
                0.
            };
            "FLAPS: UP / TAKEOFF / LANDING"
        }
        Airbrake => {
            f.airbrake = !f.airbrake;
            "AIRBRAKE"
        }
        Fbw => {
            f.assist = !f.assist;
            "FLY-BY-WIRE ASSISTANCE"
        }
        Target => {
            c.cycle();
            "NEXT TARGET"
        }
        Weapon => {
            c.weapon = if c.weapon == MissileWeapon::Heat {
                MissileWeapon::Radar
            } else {
                MissileWeapon::Heat
            };
            c.lock = 0.;
            "MISSILE TYPE: IR / RADAR"
        }
        Countermeasures => {
            c.deploy_pair(f);
            "FLARES + CHAFF"
        }
        Simple => {
            v.simple = !v.simple;
            "SIMPLIFIED VIEW"
        }
        Autopilot => {
            if f.aircraft==crate::aircraft::Aircraft::Hornet {f.autopilot=false;return "AUTOPILOT NOT INSTALLED";}
            f.autopilot = !f.autopilot;
            "APPROACH AUTOPILOT"
        }
        Help => {
            v.help = !v.help;
            "CONTROL HELP"
        }
        RadarPower => {
            cockpit_radar::command(c, cockpit_radar::Command::Power);
            "RADAR POWER"
        }
        RadarMode => {
            cockpit_radar::command(c, cockpit_radar::Command::Mode);
            "RADAR SEARCH / TWS / STT"
        }
    }
}
pub fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<FlightCamera>>,
    menu: Res<Menu>,
    mut f: ResMut<Flight>,
    mut v: ResMut<ViewState>,
    mut c: ResMut<Combat>,
    mut display: ResMut<cockpit_radar::Display>,
    mut buttons: Query<(&Key, &mut Transform, &MeshMaterial3d<StandardMaterial>, &crate::aircraft::visual::AircraftPart)>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut levers: Query<(&Lever, &mut Transform, &crate::aircraft::visual::AircraftPart), Without<Key>>,
) {
    let enabled =
        f.vehicle == Vehicle::Gripen && v.index == 3 && !v.map && !v.following() && !menu.open;
    let ray = if enabled && (keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight)) {
        windows
            .single()
            .ok()
            .and_then(|w| w.cursor_position())
            .and_then(|p| {
                cameras
                    .iter()
                    .next()
                    .and_then(|(c, t)| c.viewport_to_world(t, p).ok())
            })
    } else {
        None
    };
    let local = ray.map(|r| {
        (
            f.orientation.conjugate()
                * (world::render_origin(&f) + world::frame(&f) * r.origin.as_dvec3() - f.position) - f.aircraft.cabin_offset().as_dvec3(),
            f.orientation.conjugate() * (world::frame(&f) * r.direction.as_dvec3()),
        )
    });
    for (button, mut t, mat, aircraft) in &mut buttons {
        if aircraft.0!=f.aircraft{continue;}
        let hover = local.is_some_and(|(o, d)| {
            hit(
                button.rotation.as_dquat().inverse() * (o - button.position.as_dvec3()),
                button.rotation.as_dquat().inverse() * d,
                Vec3::ZERO,
                button.size,
            )
        });
        if hover {
            // Run after the radar picker so a panel switch supplies the final hint.
            display.hint = format!("OPTION / ALT + CLICK: {}", description(button.action));
            if mouse.just_pressed(MouseButton::Left) {
                let result = apply(button.action, &mut f, &mut v, &mut c);
                display.hint = result.into();
            }
        }
        let translation = button.position
            - button.rotation
                * Vec3::Z
                * if hover && mouse.pressed(MouseButton::Left) {
                    0.002
                } else {
                    0.
                };
        if t.translation != translation {
            t.translation = translation;
        }
        {
            let on = enabled && switch_state(button.action, &f, &v, &c);
            let color = if hover {
                Color::srgb(0.12, 0.48, 0.48)
            } else if on {
                Color::srgb(0.08, 0.25, 0.27)
            } else {
                Color::srgb(0.035, 0.065, 0.085)
            };
            let emissive = if hover {
                LinearRgba::new(0.08, 0.5, 0.45, 1.)
            } else if on {
                LinearRgba::new(0.02, 0.16, 0.15, 1.)
            } else {
                LinearRgba::BLACK
            };
            crate::render_cache::optics(&mut mats, &mat.0, color, emissive);
        }
    }
    for (lever, mut t, aircraft) in &mut levers {
        if aircraft.0!=f.aircraft{continue;}
        let angle = if switch_state(lever.action, &f, &v, &c) {
            -0.40
        } else {
            0.40
        };
        let rotation = lever.rotation * Quat::from_rotation_x(angle + std::f32::consts::FRAC_PI_2);
        if t.rotation != rotation {
            t.rotation = rotation;
        }
    }
}
fn switch_state(a: Action, f: &Flight, v: &ViewState, c: &Combat) -> bool {
    match a {
        Action::RadarPower => cockpit_radar::radar(c).powered,
        Action::Weapon => c.weapon == MissileWeapon::Radar,
        _ => active(a, f, v),
    }
}
fn description(a: Action) -> &'static str {
    match a {
        Action::Battery => "BATTERY POWER",
        Action::Apu => "APU (REQUIRES BATTERY)",
        Action::Engine => "ENGINE START / STOP (BATTERY + APU)",
        Action::Lights => "EXTERIOR LIGHTS",
        Action::Gear => "LANDING GEAR",
        Action::Flaps => "CYCLE FLAPS",
        Action::Airbrake => "AIRBRAKE",
        Action::Fbw => "FLY-BY-WIRE",
        Action::Target => "NEXT TARGET",
        Action::Weapon => "IR / RADAR MISSILES",
        Action::Countermeasures => "DEPLOY FLARES + CHAFF",
        Action::Simple => "SIMPLIFY VIEW",
        Action::Autopilot => "APPROACH AUTOPILOT",
        Action::Help => "CONTROL HELP",
        Action::RadarPower => "RADAR POWER",
        Action::RadarMode => "CYCLE SEARCH / TWS / STT",
        Action::Stovl=>"STOVL CONVERSION (BELOW 150 m/s)",
        Action::Bays=>"WEAPON BAY DOORS",
        Action::Dump=>"JETTISON EXTERNAL STORES",
        Action::EngineL=>"LEFT ENGINE FUEL VALVE",
        Action::EngineR=>"RIGHT ENGINE FUEL VALVE",
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hornet_cannot_enable_autopilot_but_other_aircraft_retain_it(){let mut f=Flight::approach();f.aircraft=crate::aircraft::Aircraft::Hornet;let mut view=ViewState::default();let mut combat=Combat::default();apply(Action::Autopilot,&mut f,&mut view,&mut combat);assert!(!f.autopilot);f.aircraft=crate::aircraft::Aircraft::Gripen;apply(Action::Autopilot,&mut f,&mut view,&mut combat);assert!(f.autopilot);}
    #[test]
    fn side_console_rays_use_the_horizontal_face_and_reject_gaps() {
        let eye = crate::cockpit::eye(Vehicle::Gripen).as_dvec3();
        let specs = side_layout();
        assert_eq!(specs.len(), 16);
        for spec in &specs {
            let face =
                spec.position.as_dvec3() + spec.rotation.as_dquat() * bevy::math::DVec3::Z * 0.007;
            let direction = (face - eye).normalize();
            let hits = specs
                .iter()
                .filter(|s| {
                    let q = s.rotation.as_dquat().inverse();
                    hit(
                        q * (eye - s.position.as_dvec3()),
                        q * direction,
                        Vec3::ZERO,
                        s.size,
                    )
                })
                .count();
            assert_eq!(hits, 1, "{}", spec.label);
            let q = spec.rotation.as_dquat().inverse();
            assert!(!hit(
                q * (eye - spec.position.as_dvec3()),
                q * -direction,
                Vec3::ZERO,
                spec.size
            ));
        }
    }
    #[test]
    fn side_radar_toggle_and_weapon_lever_follow_shared_state() {
        let mut f = Flight::runway();
        let mut v = ViewState::default();
        let mut c = Combat::default();
        let old = switch_state(Action::RadarPower, &f, &v, &c);
        apply(Action::RadarPower, &mut f, &mut v, &mut c);
        assert_ne!(switch_state(Action::RadarPower, &f, &v, &c), old);
        let old = switch_state(Action::Weapon, &f, &v, &c);
        apply(Action::Weapon, &mut f, &mut v, &mut c);
        assert_ne!(switch_state(Action::Weapon, &f, &v, &c), old);
    }
    #[test]
    fn left_console_caps_clear_the_full_throttle_travel() {
        for spec in side_layout().iter().filter(|s| s.position.x < 0.) {
            let low = spec.position.z - spec.size.y * 0.5;
            let high = spec.position.z + spec.size.y * 0.5;
            // Throttle grip spans z=-3.02..-2.74 over idle-to-full travel.
            assert!(
                high < -3.02 || low > -2.74,
                "{} overlaps throttle",
                spec.label
            );
        }
    }
    #[test]
    fn all_faces_are_pickable_without_overlapping() {
        let eye = crate::cockpit::eye(Vehicle::Gripen).as_dvec3();
        let buttons = layout();
        for (_, _, p, s) in &buttons {
            let face = p.as_dvec3() + bevy::math::DVec3::Z * 0.007;
            let ray = (face - eye).normalize();
            assert!(hit(eye, ray, *p, *s));
            assert_eq!(
                buttons
                    .iter()
                    .filter(|(_, _, p, s)| hit(eye, ray, *p, *s))
                    .count(),
                1
            );
        }
    }
    #[test]
    fn switches_use_flight_state_and_engine_interlocks() {
        let mut f = Flight::runway();
        let mut v = ViewState::default();
        let mut c = Combat::default();
        f.battery = false;
        f.apu = false;
        f.engine = false;
        apply(Action::Engine, &mut f, &mut v, &mut c);
        assert!(!f.engine);
        apply(Action::Battery, &mut f, &mut v, &mut c);
        apply(Action::Apu, &mut f, &mut v, &mut c);
        apply(Action::Engine, &mut f, &mut v, &mut c);
        assert!(f.engine);
        apply(Action::Gear, &mut f, &mut v, &mut c);
        assert!(!f.gear);
        let weapon = c.weapon;
        apply(Action::Weapon, &mut f, &mut v, &mut c);
        assert_ne!(weapon, c.weapon);
    }
}
