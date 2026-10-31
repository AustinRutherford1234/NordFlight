//! SIpriv CC BY Tu-160 exterior, baked into shared metre-scale LODs.
//! Static flight configuration; only the four bomb-bay doors are articulated.
use super::*;
#[derive(Component)]
pub struct Bomber {
    pub slot: usize,
    pub level: usize,
}
#[derive(Component)]
pub struct Door {
    slot: usize,
    level: usize,
    index: usize,
}
#[derive(Default)]
pub struct Levels([Option<usize>; 12]);
pub(super) fn setup(c: &mut Commands, server: &AssetServer) {
    if std::env::var_os("NORD_BOMBER_REFERENCE").is_some() {
        return;
    }
    let scenes: [Handle<Scene>; 3] = std::array::from_fn(|i| {
        server.load(GltfAssetLabel::Scene(0).from_asset(format!("aircraft/tu160/lod{i}.glb")))
    });
    for slot in 0..12 {
        for (level, scene) in scenes.iter().enumerate() {
            c.spawn((
                Bomber { slot, level },
                SceneRoot(scene.clone()),
                Transform::default(),
                Visibility::Hidden,
            ));
        }
    }
}
fn level(distance: f32, old: Option<usize>) -> usize {
    let target = if distance < 350. {
        0
    } else if distance < 1800. {
        1
    } else {
        2
    };
    if let Some(old) = old {
        if target > old && distance < [350., 1800., f32::INFINITY][old] * 1.1 {
            return old;
        }
        if target < old && distance > [0., 350., 1800.][old] * 0.9 {
            return old;
        }
    }
    target
}
pub fn register(
    mut c: Commands,
    names: Query<(Entity, &Name), Added<Name>>,
    parents: Query<&ChildOf>,
    roots: Query<&Bomber>,
) {
    for (entity, name) in &names {
        let Some(index) = name
            .as_str()
            .strip_prefix("BomberBay")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|i| *i < 6)
        else {
            continue;
        };
        let mut node = entity;
        for _ in 0..16 {
            if let Ok(root) = roots.get(node) {
                c.entity(entity).insert(Door {
                    slot: root.slot,
                    level: root.level,
                    index,
                });
                break;
            }
            let Ok(parent) = parents.get(node) else {
                break;
            };
            node = parent.parent();
        }
    }
}
pub fn update(
    f: Res<Flight>,
    c: Res<Combat>,
    camera: Query<&GlobalTransform, With<crate::FlightCamera>>,
    mut levels: Local<Levels>,
    mut roots: Query<(&Bomber, &mut Transform, &mut Visibility), Without<Door>>,
    mut doors: Query<(&Door, &mut Transform), Without<Bomber>>,
) {
    let n = c.naval.as_ref();
    let mut selected = [None; 12];
    for slot in 0..12 {
        if let Some(r) = n.and_then(|n| {
            n.raiders
                .iter()
                .find(|r| r.slot == slot && r.role == Role::Bomber)
        }) {
            if let Some(t) = c.targets.get(r.slot).filter(|t| t.alive) {
                let p = crate::world::local(t.position, &f);
                let distance = camera
                    .iter()
                    .next()
                    .map_or(f.position.distance(t.position) as f32, |camera| {
                        camera.translation().distance(p)
                    });
                let choice = level(distance, levels.0[slot]);
                levels.0[slot] = Some(choice);
                selected[slot] = Some(choice);
            }
        }
    }
    for (root, mut transform, mut visible) in &mut roots {
        let show = selected[root.slot] == Some(root.level);
        visible.set_if_neq(if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if show {
            let t = &c.targets[root.slot];
            transform.translation = crate::world::local(t.position, &f);
            // The imported Tu-160 nose points +Z; flight body-forward is -Z.
            transform.rotation = crate::world::local_rot(t.orientation, &f) * model_alignment();
        }
    }
    for (door, mut transform) in &mut doors {
        if selected[door.slot] != Some(door.level) {
            continue;
        }
        let amount = n
            .and_then(|n| n.raiders.iter().find(|r| r.slot == door.slot))
            .map_or(0., |r| r.bay as f32);
        let sign = if door.index % 2 == 0 { -1. } else { 1. };
        transform.rotation = Quat::from_rotation_z(sign * std::f32::consts::FRAC_PI_2 * amount);
    }
}
pub fn release_station(remaining: u8) -> DVec3 {
    model_alignment().as_dquat() * DVec3::new(0., -1.8, if remaining % 2 == 0 { -11.12 } else { 3.63 })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lod_hysteresis_and_station_alignment() {
        assert_eq!(level(300., None), 0);
        assert_eq!(level(360., Some(0)), 0);
        assert_eq!(level(400., Some(0)), 1);
        assert_eq!(level(1900., Some(1)), 1);
        assert_eq!(level(2100., Some(1)), 2);
        for n in 1..=super::BOMBER_MISSILES {
            let p = release_station(n);
            assert!(p.y < -1.33 && p.x.abs() < 0.9);
        }
    }
}

// Apply only to the imported exterior, leaving aerodynamic attitude unchanged.
fn model_alignment() -> Quat { Quat::from_rotation_y(std::f32::consts::PI) }
#[cfg(test)] mod alignment_tests {
    use super::*;
    #[test] fn imported_nose_matches_body_forward_and_keeps_up() {
        assert!((model_alignment()*Vec3::Z-Vec3::NEG_Z).length()<1e-6);
        assert!((model_alignment()*Vec3::Y-Vec3::Y).length()<1e-6);
    }
}
