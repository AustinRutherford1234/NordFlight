//! Launch-station jet blast deflector. Dimensions/actuation are fitted estimates.
//! Shared ship-local pose drives the visible panels and dynamic collision skin.
use super::*;
pub const WIDTH: f64 = 14.;
pub const DEPTH: f64 = 4.;
pub fn hinge() -> DVec3 {
    DVec3::new(catapult::X, DECK - 0.14, catapult::START.z + 22.)
}
pub fn rotation(amount: f64) -> DQuat {
    DQuat::from_rotation_x(-65f64.to_radians() * smooth(amount))
}
fn smooth(x: f64) -> f64 {
    let x = x.clamp(0., 1.);
    x * x * (3. - 2. * x)
}
pub fn advance(f: &mut Flight, dt: f64) {
    let preparing = f.carrier.operational && (f.carrier.attached || f.carrier.launch.is_some());
    if preparing {
        f.carrier.deflector_hold = f.elapsed + 4.;
    }
    let target = if preparing || (f.carrier.operational && f.elapsed < f.carrier.deflector_hold) {
        1.
    } else {
        0.
    };
    f.carrier.deflector += (target - f.carrier.deflector).clamp(-dt / 4., dt / 4.);
}
/// Closed cuboid, including rear/edge faces; no heap allocation per collision query.
pub fn triangles(amount: f64) -> [[DVec3; 3]; 12] {
    let q = rotation(amount);
    let h = hinge();
    let p: [DVec3; 8] = std::array::from_fn(|i| {
        h + q * DVec3::new(
            if i & 1 == 0 { -WIDTH / 2. } else { WIDTH / 2. },
            if i & 2 == 0 { 0. } else { 0.16 },
            if i & 4 == 0 { 0. } else { DEPTH },
        )
    });
    [
        [0, 1, 3],
        [0, 3, 2],
        [4, 6, 7],
        [4, 7, 5],
        [0, 4, 5],
        [0, 5, 1],
        [2, 3, 7],
        [2, 7, 6],
        [0, 2, 6],
        [0, 6, 4],
        [1, 5, 7],
        [1, 7, 3],
    ]
    .map(|t| t.map(|i| p[i]))
}
#[derive(Component)]
pub struct Panel;
#[derive(Component)]
pub struct Ram {
    x: f32,
}
pub fn setup(
    c: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let cube = meshes.add(Cuboid::new(1., 1., 1.));
    let metal = materials.add(StandardMaterial {
        base_color: Color::srgb(0.27, 0.29, 0.30),
        perceptual_roughness: 0.85,
        ..default()
    });
    let rib = materials.add(StandardMaterial {
        base_color: Color::srgb(0.13, 0.15, 0.16),
        metallic: 0.5,
        perceptual_roughness: 0.6,
        ..default()
    });
    let edge = materials.add(StandardMaterial {
        base_color: Color::srgb(0.65, 0.55, 0.24),
        perceptual_roughness: 0.85,
        ..default()
    });
    c.spawn((Panel, Transform::default(), Visibility::Hidden))
        .with_children(|c| {
            for i in 0..4 {
                let x = (i as f32 - 1.5) * 3.5;
                c.spawn((
                    Mesh3d(cube.clone()),
                    MeshMaterial3d(metal.clone()),
                    Transform::from_xyz(x, 0.08, 2.).with_scale(Vec3::new(3.46, 0.16, 4.)),
                ));
                // Recessed rear stiffeners, hinge bosses and heat-discolored panel edges.
                for z in [0.15, 1.35, 2.65, 3.85] {
                    c.spawn((
                        Mesh3d(cube.clone()),
                        MeshMaterial3d(rib.clone()),
                        Transform::from_xyz(x, -0.07, z).with_scale(Vec3::new(3.35, 0.14, 0.09)),
                    ));
                }
                for side in [-1., 1.] {
                    c.spawn((
                        Mesh3d(cube.clone()),
                        MeshMaterial3d(edge.clone()),
                        Transform::from_xyz(x + side * 1.70, 0.165, 2.)
                            .with_scale(Vec3::new(0.045, 0.015, 3.95)),
                    ));
                    c.spawn((
                        Mesh3d(cube.clone()),
                        MeshMaterial3d(rib.clone()),
                        Transform::from_xyz(x + side * 1.15, -0.05, 0.08)
                            .with_scale(Vec3::new(0.25, 0.25, 0.30)),
                    ));
                }
            }
        });
    for x in [-5.25, -1.75, 1.75, 5.25] {
        c.spawn((
            Ram { x },
            Mesh3d(cube.clone()),
            MeshMaterial3d(rib.clone()),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
}
pub fn update(
    f: Res<Flight>,
    mut parts: Query<(&mut Transform, &mut Visibility, Option<&Ram>), Or<(With<Panel>, With<Ram>)>>,
) {
    let ship = crate::world::local_rot(ship_rotation(&f, 0, f.elapsed), &f);
    let q = rotation(f.carrier.deflector).as_quat();
    let base = crate::world::local(ship_world(&f, 0, hinge(), f.elapsed), &f);
    for (mut t, mut v, ram) in &mut parts {
        *v = if f.carrier.active {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if !f.carrier.active {
            continue;
        }
        if let Some(r) = ram {
            let a = Vec3::new(r.x, -0.25, 0.3);
            let b = q * Vec3::new(r.x, -0.10, 2.6);
            let d = b - a;
            *t = Transform::from_translation(base + ship * (a + b) * 0.5)
                .with_rotation(ship * Quat::from_rotation_arc(Vec3::Y, d.normalize_or_zero()))
                .with_scale(Vec3::new(0.13, d.length(), 0.13));
            if f.carrier.deflector < 0.08 {
                *v = Visibility::Hidden;
            }
        } else {
            *t = Transform::from_translation(base).with_rotation(ship * q);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn raises_holds_lowers_and_resets() {
        let mut f = scenario(true);
        for _ in 0..480 {
            f.elapsed += 1. / 120.;
            advance(&mut f, 1. / 120.);
        }
        assert!((f.carrier.deflector - 1.).abs() < 1e-9);
        f.carrier.attached = false;
        for _ in 0..360 {
            f.elapsed += 1. / 120.;
            advance(&mut f, 1. / 120.);
        }
        assert_eq!(f.carrier.deflector, 1.);
        for _ in 0..660 {
            f.elapsed += 1. / 120.;
            advance(&mut f, 1. / 120.);
        }
        assert_eq!(f.carrier.deflector, 0.);
        assert_eq!(State::default().deflector, 0.);
    }
    #[test]
    fn panel_clearance_and_shared_geometry() {
        let f = scenario(true);
        let tail = ship_relative(&f, 0, f.position + f.orientation * DVec3::Z * 9.5, 0.);
        assert!(hinge().z - tail.z > 3.);
        for amount in [0., 0.25, 0.5, 1.] {
            for p in triangles(amount).into_iter().flatten() {
                assert!(inside(p));
                assert!(p.y >= DECK - 0.141);
            }
        }
        let top = hinge() + rotation(1.) * DVec3::new(0., 0., DEPTH);
        assert!(top.y - DECK > 3.4);
        let closed_top=triangles(0.).into_iter().flatten().map(|p|p.y).fold(f64::NEG_INFINITY,f64::max);
        assert!((closed_top-DECK-0.02).abs()<1e-9,"lowered panel must close flush with the deck");
    }
}
