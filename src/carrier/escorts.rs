//! Shared Burke visual assets, rigid 25-knot formation, LODs and restrained night lighting.
use super::*;
use crate::{ViewState, world};
pub const SPACING: f64 = 500.;
pub const NUMBERS: [u16; 2] = [125, 128];
pub fn offset(ship: usize) -> DVec3 {
    DVec3::new(if ship == 0 { -SPACING } else { SPACING }, 0., 0.)
}
pub fn position(ship: usize, time: f64) -> DVec3 {
    super::world(offset(ship), time)
}
#[derive(Component)]
pub struct Escort {
    pub ship: usize,
    pub lod: usize,
}
#[derive(Component)]
pub struct Identity(pub usize);
#[derive(Component)]
pub struct DeckLight {
    ship: usize,
    at: DVec3,
    direction: DVec3,
    power: f32,
}
#[derive(Resource)]
pub struct Materials {
    handles: Vec<(Handle<StandardMaterial>, LinearRgba)>,
    ready: Vec<bool>,
    night: f32,
}
#[derive(Default)]
pub struct Levels([Option<usize>; 2]);
fn desired_lod(distance: f32, old: Option<usize>) -> usize {
    let thresholds = [300., 1000., 3000.];
    let target = thresholds.iter().filter(|d| distance > **d).count();
    if let Some(old) = old {
        if target > old && distance < thresholds[old] * 1.1 {
            return old;
        }
        if target < old && distance > thresholds[old - 1] * 0.9 {
            return old;
        }
    }
    target
}
fn light_color(name: &str) -> Option<LinearRgba> {
    if name.contains("BurkeWindows") {
        Some(LinearRgba::new(65., 35., 12., 1.))
    } else if name.contains("BurkePortLens") {
        Some(LinearRgba::new(80., 0.3, 0.1, 1.))
    } else if name.contains("BurkeStarboardLens") {
        Some(LinearRgba::new(0.15, 70., 0.6, 1.))
    } else if name.contains("BurkeWhiteLens") {
        Some(LinearRgba::new(55., 48., 35., 1.))
    } else {
        None
    }
}
pub fn setup(c: &mut Commands, server: &AssetServer) {
    // Diagnostic-only carrier baseline; production always spawns both escorts.
    if std::env::var_os("NORD_ESCORTS_REFERENCE").is_some() {
        c.insert_resource(Materials {
            handles: vec![],
            ready: vec![],
            night: -1.,
        });
        return;
    }
    let metadata: serde_json::Value =
        serde_json::from_str(include_str!("../../assets/ships/burke/inspection.json"))
            .expect("Burke asset metadata");
    let mut handles = vec![];
    for level in 0..4 {
        let path = format!("ships/burke/lod{level}.gltf");
        for (index, name) in metadata["lods"][level]["materials"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            if let Some(color) = light_color(name.as_str().unwrap_or_default()) {
                handles.push((
                    server.load(
                        GltfAssetLabel::Material {
                            index,
                            is_scale_inverted: false,
                        }
                        .from_asset(path.clone()),
                    ),
                    color,
                ));
            }
        }
    }
    let ready = vec![false; handles.len()];
    c.insert_resource(Materials {
        handles,
        ready,
        night: -1.,
    });
    for ship in 0..2 {
        for lod in 0..4 {
            c.spawn((
                Name::new(format!("DDG {} LOD {}", NUMBERS[ship], lod)),
                Escort { ship, lod },
                SceneRoot(server.load(
                    GltfAssetLabel::Scene(0).from_asset(format!("ships/burke/lod{lod}.gltf")),
                )),
                Transform::default(),
                Visibility::Hidden,
            ));
        }
        c.spawn((
            Name::new(format!("DDG {} hull identity", NUMBERS[ship])),
            Identity(ship),
            SceneRoot(
                server.load(
                    GltfAssetLabel::Scene(0)
                        .from_asset(format!("ships/burke/markings{}.gltf", NUMBERS[ship])),
                ),
            ),
            Transform::default(),
            Visibility::Hidden,
        ));
        for lamp in metadata["lights"].as_array().unwrap() {
            let vector = |key: &str| {
                DVec3::new(
                    lamp[key][0].as_f64().unwrap(),
                    lamp[key][1].as_f64().unwrap(),
                    lamp[key][2].as_f64().unwrap(),
                )
            };
            let color = &lamp["color"];
            let power = lamp["intensity"].as_f64().unwrap() as f32;
            c.spawn((
                DeckLight {
                    ship,
                    at: vector("position"),
                    direction: vector("direction"),
                    power,
                },
                SpotLight {
                    color: Color::srgb(
                        color[0].as_f64().unwrap() as f32,
                        color[1].as_f64().unwrap() as f32,
                        color[2].as_f64().unwrap() as f32,
                    ),
                    intensity: 0.,
                    range: lamp["range"].as_f64().unwrap() as f32,
                    inner_angle: 0.35,
                    outer_angle: 0.85,
                    shadows_enabled: false,
                    ..default()
                },
                Transform::default(),
                Visibility::Hidden,
            ));
        }
    }
}
pub fn update(
    f: Res<Flight>,
    e: Res<Environment>,
    v: Res<ViewState>,
    mut levels: Local<Levels>,
    mut materials: ResMut<Materials>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    cameras: Query<(&GlobalTransform, &Projection), With<crate::FlightCamera>>,
    mut roots: Query<(&Escort, &mut Transform, &mut Visibility)>,
    mut identities: Query<(&Identity, &mut Transform, &mut Visibility), Without<Escort>>,
    mut lamps: Query<
        (&DeckLight, &mut Transform, &mut Visibility, &mut SpotLight),
        (Without<Escort>, Without<Identity>),
    >,
) {
    let mut distances = [f32::INFINITY; 2];
    let mut selected = [3; 2];
    for ship in 0..2 {
        let local = world::local(super::ship_origin(&f,ship+1,f.elapsed), &f);
        let distance = cameras.iter().next().map_or(
            f.position.distance(super::ship_origin(&f,ship+1,f.elapsed)) as f32,
            |(camera, projection)| match projection {
                Projection::Perspective(p) => {
                    (camera.translation() - local).length() * (p.fov * 0.5).tan() / 0.57735
                }
                Projection::Orthographic(p) => p.area.height().abs() / 1.1547,
                _ => (camera.translation() - local).length(),
            },
        );
        distances[ship] = distance;
        selected[ship] = desired_lod(distance, levels.0[ship]);
        levels.0[ship] = Some(selected[ship]);
    }
    for (ship, mut t, mut visible) in &mut roots {
        let show = f.carrier.active && ship.lod == selected[ship.ship];
        visible.set_if_neq(if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if show {
            t.translation = world::local(super::ship_origin(&f,ship.ship+1,f.elapsed), &f);
            t.rotation = world::local_rot(super::ship_rotation(&f,ship.ship+1,f.elapsed), &f);
        }
    }
    for (id, mut t, mut visible) in &mut identities {
        visible.set_if_neq(if f.carrier.active && selected[id.0] < 3 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if f.carrier.active {
            t.translation = world::local(super::ship_origin(&f,id.0+1,f.elapsed), &f);
            t.rotation = world::local_rot(super::ship_rotation(&f,id.0+1,f.elapsed), &f);
        }
    }
    let night = ((0.08 - e.sun_direction().dot(f.up())) / 0.14).clamp(0., 1.) as f32;
    let changed = (night - materials.night).abs() > 0.005;
    if f.carrier.active {
        for i in 0..materials.handles.len() {
            if changed || !materials.ready[i] {
                let (handle, color) = &materials.handles[i];
                if let Some(mat) = mats.get_mut(handle) {
                    mat.emissive = *color * night;
                    mat.unlit = false;
                    materials.ready[i] = true;
                }
            }
        }
        materials.night = night;
    }
    for (lamp, mut t, mut visibility, mut light) in &mut lamps {
        let on = f.carrier.active
            && night > 0.02
            && distances[lamp.ship] < if v.high_quality { 500. } else { 350. };
        visibility.set_if_neq(if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if on {
            t.translation = world::local(super::ship_origin(&f,lamp.ship+1,f.elapsed) + super::ship_rotation(&f,lamp.ship+1,f.elapsed) * lamp.at, &f);
            t.rotation = world::local_rot(
                super::ship_rotation(&f,lamp.ship+1,f.elapsed) * DQuat::from_rotation_arc(DVec3::NEG_Z, lamp.direction.normalize()),
                &f,
            );
            let power = lamp.power * night;
            if (light.intensity - power).abs() > 1. {
                light.intensity = power;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn formation_is_rigid_and_all_modes_use_shared_carrier_activation() {
        for time in [0., 60., 3600.] {
            for ship in 0..2 {
                let local = relative(position(ship, time), time);
                assert!((local - offset(ship)).length() < 1e-7);
                assert!((position(ship, time).distance(origin(time)) - 500.).abs() < 1e-6);
            }
            assert!((position(0, time).distance(position(1, time)) - 1000.).abs() < 1e-6);
        }
    }
    #[test]
    fn lod_hysteresis_does_not_chatter_at_boundaries() {
        assert_eq!(desired_lod(50., None), 0);
        assert_eq!(desired_lod(500., None), 1);
        assert_eq!(desired_lod(1500., None), 2);
        assert_eq!(desired_lod(4000., None), 3);
        assert_eq!(desired_lod(310., Some(0)), 0);
        assert_eq!(desired_lod(280., Some(1)), 1);
        assert_eq!(desired_lod(260., Some(1)), 0);
        assert_eq!(desired_lod(1200., Some(1)), 2);
    }
    #[test]
    fn lods_share_texture_uris_and_reduce_geometry() {
        let m: serde_json::Value =
            serde_json::from_str(include_str!("../../assets/ships/burke/inspection.json")).unwrap();
        let levels = m["lods"].as_array().unwrap();
        let mut count = u64::MAX;
        for l in levels {
            let n = l["triangles"].as_u64().unwrap();
            assert!(n < count);
            count = n;
            assert_eq!(l["texture_uris"], levels[0]["texture_uris"]);
        }
        assert_eq!(m["removed_helicopter_objects"].as_array().unwrap().len(), 5);
        assert!(m["bridge_window_panes"].as_u64().unwrap() > 20);
    }
    #[test]
    fn leaving_carrier_mode_hides_all_lods_markings_and_lights() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(Flight::approach())
            .insert_resource(Environment::default())
            .insert_resource(ViewState::default())
            .init_resource::<Assets<StandardMaterial>>()
            .insert_resource(Materials {
                handles: vec![],
                ready: vec![],
                night: -1.,
            })
            .add_systems(Update, update);
        for ship in 0..2 {
            for lod in 0..4 {
                app.world_mut().spawn((
                    Escort { ship, lod },
                    Transform::default(),
                    Visibility::Visible,
                ));
            }
            app.world_mut()
                .spawn((Identity(ship), Transform::default(), Visibility::Visible));
            app.world_mut().spawn((
                DeckLight {
                    ship,
                    at: DVec3::ZERO,
                    direction: DVec3::NEG_Y,
                    power: 1.,
                },
                Transform::default(),
                Visibility::Visible,
                SpotLight::default(),
            ));
        }
        app.update();
        {
            let w = app.world_mut();
            assert!(
                w.query::<&Visibility>()
                    .iter(w)
                    .all(|v| *v == Visibility::Hidden)
            );
        }
        app.world_mut().resource_mut::<Flight>().carrier.active = true;
        app.update();
        {
            let w = app.world_mut();
            assert_eq!(
                w.query::<(&Escort, &Visibility)>()
                    .iter(w)
                    .filter(|(_, v)| **v == Visibility::Inherited)
                    .count(),
                2
            );
        }
        app.world_mut().resource_mut::<Flight>().carrier.active = false;
        app.update();
        {
            let w = app.world_mut();
            assert!(
                w.query::<&Visibility>()
                    .iter(w)
                    .all(|v| *v == Visibility::Hidden)
            );
        }
    }
}
