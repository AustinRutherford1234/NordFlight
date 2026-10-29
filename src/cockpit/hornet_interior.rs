//! Fitted tandem tub and representative NACES-style seats, in cockpit coordinates.
use bevy::prelude::*;
#[derive(Component)]
pub struct Seat(pub usize);
#[derive(Component)]
pub struct CanopyBow;
pub const SPACING: f32 = 1.45;
pub fn block(
    c: &mut Commands,
    m: &mut Assets<Mesh>,
    root: Entity,
    mat: Handle<StandardMaterial>,
    p: Vec3,
    s: Vec3,
    q: Quat,
) {
    let e = c
        .spawn((
            crate::render_batch::RigidGeometry,
            Mesh3d(m.add(Cuboid::from_size(s))),
            MeshMaterial3d(mat),
            Transform::from_translation(p).with_rotation(q),
        ))
        .id();
    c.entity(root).add_child(e);
}
pub fn rod(
    c: &mut Commands,
    m: &mut Assets<Mesh>,
    root: Entity,
    mat: Handle<StandardMaterial>,
    a: Vec3,
    b: Vec3,
    r: f32,
) {
    let d = b - a;
    let e = c
        .spawn((
            crate::render_batch::RigidGeometry,
            Mesh3d(m.add(Cylinder::new(r, d.length()))),
            MeshMaterial3d(mat),
            Transform::from_translation((a + b) * 0.5)
                .with_rotation(Quat::from_rotation_arc(Vec3::Y, d.normalize())),
        ))
        .id();
    c.entity(root).add_child(e);
}
pub fn seat(
    c: &mut Commands,
    m: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    root: Entity,
) {
    let metal = mats.add(StandardMaterial {
        base_color: Color::srgb(0.065, 0.075, 0.08),
        metallic: 0.55,
        perceptual_roughness: 0.65,
        ..default()
    });
    let pad = mats.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.25, 0.15),
        perceptual_roughness: 0.95,
        ..default()
    });
    let belt = mats.add(StandardMaterial {
        base_color: Color::srgb(0.49, 0.48, 0.35),
        perceptual_roughness: 0.9,
        ..default()
    });
    let chrome = mats.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.48, 0.5),
        metallic: 0.85,
        ..default()
    });
    let yellow = mats.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.69, 0.04),
        ..default()
    });
    for (p, s, mat) in [
        (
            Vec3::new(0., -0.08, 0.),
            Vec3::new(0.52, 0.14, 0.69),
            metal.clone(),
        ),
        (
            Vec3::new(0., 0.02, -0.08),
            Vec3::new(0.43, 0.12, 0.58),
            pad.clone(),
        ),
        (
            Vec3::new(0., 0.37, 0.27),
            Vec3::new(0.49, 0.76, 0.12),
            metal.clone(),
        ),
        (
            Vec3::new(0., 0.39, 0.19),
            Vec3::new(0.40, 0.63, 0.08),
            pad.clone(),
        ),
        (
            Vec3::new(0., 0.85, 0.32),
            Vec3::new(0.30, 0.25, 0.22),
            metal.clone(),
        ),
        (
            Vec3::new(0., 0.86, 0.18),
            Vec3::new(0.25, 0.16, 0.06),
            pad.clone(),
        ),
        (
            Vec3::new(0., -0.2, 0.15),
            Vec3::new(0.38, 0.16, 0.4),
            metal.clone(),
        ),
    ] {
        if mat == pad {
            let e = c
                .spawn((
                    crate::render_batch::RigidGeometry,
                    Mesh3d(m.add(super::raptor_interior::padded(s))),
                    MeshMaterial3d(mat),
                    Transform::from_translation(p),
                ))
                .id();
            c.entity(root).add_child(e);
        } else {
            block(c, m, root, mat, p, s, Quat::IDENTITY);
        }
    }
    for side in [-1., 1.] {
        rod(
            c,
            m,
            root,
            chrome.clone(),
            Vec3::new(side * 0.24, -0.25, 0.4),
            Vec3::new(side * 0.24, 0.94, 0.4),
            0.025,
        );
        block(
            c,
            m,
            root,
            metal.clone(),
            Vec3::new(side * 0.25, 0.18, -0.05),
            Vec3::new(0.075, 0.09, 0.38),
            Quat::IDENTITY,
        );
        block(
            c,
            m,
            root,
            belt.clone(),
            Vec3::new(side * 0.12, 0.39, 0.125),
            Vec3::new(0.045, 0.64, 0.016),
            Quat::from_rotation_z(side * 0.12),
        );
        block(
            c,
            m,
            root,
            belt.clone(),
            Vec3::new(side * 0.105, 0.09, -0.16),
            Vec3::new(0.22, 0.018, 0.055),
            Quat::from_rotation_y(side * 0.22),
        );
        for y in [0.13, 0.5, 0.65] {
            block(
                c,
                m,
                root,
                chrome.clone(),
                Vec3::new(side * 0.13, y, 0.105),
                Vec3::new(0.063, 0.045, 0.018),
                Quat::IDENTITY,
            );
        }
        for z in [-0.24, 0., 0.22] {
            rod(
                c,
                m,
                root,
                chrome.clone(),
                Vec3::new(side * 0.265, -0.08, z),
                Vec3::new(side * 0.28, -0.08, z),
                0.017,
            );
        }
        rod(
            c,
            m,
            root,
            belt.clone(),
            Vec3::new(side * 0.21, -0.1, -0.24),
            Vec3::new(side * 0.1, -0.38, -0.47),
            0.012,
        );
    }
    block(
        c,
        m,
        root,
        chrome.clone(),
        Vec3::new(0., 0.1, -0.16),
        Vec3::new(0.07, 0.027, 0.075),
        Quat::IDENTITY,
    );
    // Segmented yellow/black lower firing handle, visibly between the knees.
    for i in 0..24 {
        let a = i as f32 * std::f32::consts::TAU / 24.;
        let b = (i + 1) as f32 * std::f32::consts::TAU / 24.;
        rod(
            c,
            m,
            root,
            if i % 3 == 0 {
                metal.clone()
            } else {
                yellow.clone()
            },
            Vec3::new(a.cos() * 0.085, 0.14 + a.sin() * 0.065, -0.34),
            Vec3::new(b.cos() * 0.085, 0.14 + b.sin() * 0.065, -0.34),
            0.014,
        );
    }
}
pub fn setup(
    c: &mut Commands,
    m: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    root: Entity,
) {
    let tub = mats.add(StandardMaterial {
        base_color: Color::srgb(0.055, 0.065, 0.07),
        perceptual_roughness: 0.85,
        ..default()
    });
    let trim = mats.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.17, 0.17),
        metallic: 0.45,
        ..default()
    });
    let white = mats.add(StandardMaterial {
        base_color: Color::srgb(0.68, 0.72, 0.67),
        ..default()
    });
    let green = mats.add(StandardMaterial {
        base_color: Color::srgb(0.02, 0.11, 0.07),
        emissive: LinearRgba::new(0.04, 0.3, 0.16, 1.),
        ..default()
    });
    for i in 0..2 {
        let z = i as f32 * SPACING;
        let seatroot = c
            .spawn((
                Seat(i),
                Name::new(format!("Hornet NACES seat {i}")),
                Transform::from_xyz(0., 0.06 + i as f32 * 0.12, -2.75 + z),
                Visibility::Inherited,
            ))
            .id();
        c.entity(root).add_child(seatroot);
        seat(c, m, mats, seatroot);
        if i == 1 {
            block(
                c,
                m,
                root,
                tub.clone(),
                Vec3::new(0., -0.13, -2.60 + z),
                Vec3::new(0.42, 0.23, 0.48),
                Quat::IDENTITY,
            );
        }
        block(
            c,
            m,
            root,
            tub.clone(),
            Vec3::new(0., 0.04, -2.65 + z),
            Vec3::new(0.96, 0.08, 1.42),
            Quat::IDENTITY,
        );
        block(
            c,
            m,
            root,
            tub.clone(),
            Vec3::new(0., 0.27, -2.1 + z),
            Vec3::new(0.9, 1.02, 0.05),
            Quat::IDENTITY,
        );
        for side in [-1., 1.] {
            block(
                c,
                m,
                root,
                tub.clone(),
                Vec3::new(side * 0.49, 0.22, -2.65 + z),
                Vec3::new(0.06, 0.9, 1.45),
                Quat::IDENTITY,
            );
            block(
                c,
                m,
                root,
                trim.clone(),
                Vec3::new(side * 0.37, 0.30, -2.85 + z),
                Vec3::new(0.22, 0.12, 1.10),
                Quat::IDENTITY,
            );
            rod(
                c,
                m,
                root,
                trim.clone(),
                Vec3::new(side * 0.47, 0.73, -3.35 + z),
                Vec3::new(side * 0.47, 0.73, -1.98 + z),
                0.035,
            );
            for row in 0..10 {
                for col in 0..2 {
                    let p = Vec3::new(
                        side * (0.32 + col as f32 * 0.095),
                        0.37,
                        -3.28 + row as f32 * 0.09 + z,
                    );
                    block(
                        c,
                        m,
                        root,
                        tub.clone(),
                        p,
                        Vec3::new(0.062, 0.018, 0.064),
                        Quat::IDENTITY,
                    );
                    rod(
                        c,
                        m,
                        root,
                        white.clone(),
                        p + Vec3::Y * 0.012,
                        p + Vec3::new(0., 0.043, -0.012),
                        0.006,
                    );
                }
            }
            for j in 0..7 {
                block(
                    c,
                    m,
                    root,
                    trim.clone(),
                    Vec3::new(side * 0.19, 0.015, -3.31 + j as f32 * 0.055 + z),
                    Vec3::new(0.23, 0.012, 0.015),
                    Quat::IDENTITY,
                );
            }
        }
        if i == 1 {
            block(
                c,
                m,
                root,
                tub.clone(),
                Vec3::new(0., 0.60, -3.3 + z),
                Vec3::new(0.81, 0.44, 0.10),
                Quat::from_rotation_x(-0.12),
            );
            for x in [-0.25, 0., 0.25] {
                block(
                    c,
                    m,
                    root,
                    green.clone(),
                    Vec3::new(x, 0.62, -3.238 + z),
                    Vec3::new(0.20, 0.20, 0.012),
                    Quat::from_rotation_x(-0.12),
                );
                let image = images.add(Image::new_fill(
                    bevy::render::render_resource::Extent3d {
                        width: 256,
                        height: 192,
                        depth_or_array_layers: 1,
                    },
                    bevy::render::render_resource::TextureDimension::D2,
                    &[0, 0, 0, 255],
                    bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                    bevy::asset::RenderAssetUsages::default(),
                ));
                let mat = mats.add(StandardMaterial {
                    base_color_texture: Some(image.clone()),
                    emissive_texture: Some(image.clone()),
                    emissive: LinearRgba::new(1.2, 1.2, 1.2, 1.),
                    unlit: true,
                    ..default()
                });
                let e = c
                    .spawn((
                        crate::render_batch::RigidGeometry,
                        Mesh3d(m.add(Rectangle::new(0.185, 0.185))),
                        MeshMaterial3d(mat.clone()),
                        Transform::from_xyz(x, 0.62, -3.225 + z),
                        super::InternalDisplay,
                        super::Instrument {
                            aircraft: crate::aircraft::Aircraft::Hornet,
                            vehicle: crate::physics::Vehicle::Gripen,
                            kind: if x < -0.1 {
                                0
                            } else if x > 0.1 {
                                4
                            } else {
                                2
                            },
                            image,
                            material: mat,
                        },
                    ))
                    .id();
                c.entity(root).add_child(e);
                for k in 0..5 {
                    block(
                        c,
                        m,
                        root,
                        white.clone(),
                        Vec3::new(x - 0.08 + k as f32 * 0.04, 0.48, -3.38 + z),
                        Vec3::splat(0.014),
                        Quat::IDENTITY,
                    );
                }
            }
        }
    }
    let bow = c
        .spawn((
            CanopyBow,
            Transform::from_xyz(0., 0.775, 0.041),
            Visibility::Inherited,
        ))
        .id();
    c.entity(root).add_child(bow);
    for z in [-3.38, -2.0, -0.75] {
        for j in 0..24 {
            let a = j as f32 / 24. * std::f32::consts::PI;
            let b = (j + 1) as f32 / 24. * std::f32::consts::PI;
            rod(
                c,
                m,
                bow,
                trim.clone(),
                Vec3::new(a.cos() * 0.50, 0.73 + a.sin() * 0.72, z) - Vec3::new(0., 0.775, 0.041),
                Vec3::new(b.cos() * 0.50, 0.73 + b.sin() * 0.72, z) - Vec3::new(0., 0.775, 0.041),
                0.022,
            );
        }
    }
}
pub fn update(
    f: Res<crate::physics::Flight>,
    view: Res<crate::ViewState>,
    mut parts: Query<
        (
            Option<&Seat>,
            Option<&CanopyBow>,
            &mut Visibility,
            &mut Transform,
        ),
        Or<(With<Seat>, With<CanopyBow>)>,
    >,
) {
    for (s, b, mut v, mut t) in &mut parts {
        if let Some(s) = s {
            *v = if f.ejection.active && f.ejection.age >= crate::ejection::delay(s.0) {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
        }
        if b.is_some() {
            *v = if f.ejection.active || view.index != 3 || view.map {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
            t.rotation = Quat::from_rotation_x(f.systems.hornet_canopy as f32 * 0.7);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tandem_seats_and_all_generated_meshes_are_finite() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<Image>>()
            .add_systems(
                Startup,
                |mut c: Commands,
                 mut m: ResMut<Assets<Mesh>>,
                 mut mats: ResMut<Assets<StandardMaterial>>,
                 mut images: ResMut<Assets<Image>>| {
                    let root = c.spawn(Transform::default()).id();
                    setup(&mut c, &mut m, &mut mats, &mut images, root);
                },
            );
        app.update();
        let world = app.world_mut();
        assert_eq!(world.query::<&Seat>().iter(world).count(), 2);
        for (_, mesh) in world.resource::<Assets<Mesh>>().iter() {
            if let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            {
                assert!(p.iter().flatten().all(|x| x.is_finite()));
            }
            assert!(
                mesh.attributes()
                    .all(|(_, a)| a.len() == mesh.count_vertices())
            );
        }
    }
}

#[cfg(test)]
mod optical_regression {
    use super::*;
    #[test]
    fn narrowed_tub_preserves_hud_combiner_scale() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<Image>>()
            .add_systems(
                Startup,
                |mut c: Commands,
                 mut m: ResMut<Assets<Mesh>>,
                 mut mats: ResMut<Assets<StandardMaterial>>,
                 mut images: ResMut<Assets<Image>>| {
                    let root = c
                        .spawn((
                            Transform::from_scale(Vec3::new(0.75, 1., 1.)),
                            Visibility::Inherited,
                        ))
                        .id();
                    super::super::setup_for(
                        &mut c,
                        &mut m,
                        &mut mats,
                        &mut images,
                        root,
                        crate::physics::Vehicle::Gripen,
                        crate::aircraft::Aircraft::Hornet,
                    );
                },
            );
        app.update();
        let world = app.world_mut();
        let entity = world
            .query::<(Entity, &super::super::Instrument)>()
            .iter(world)
            .find(|(_, s)| s.kind == 3)
            .unwrap()
            .0;
        let mut e = entity;
        let mut matrix = Mat4::IDENTITY;
        loop {
            matrix = world.get::<Transform>(e).unwrap().to_matrix() * matrix;
            let Some(p) = world.get::<ChildOf>(e) else {
                break;
            };
            e = p.parent();
        }
        assert!((matrix.transform_vector3(Vec3::X).length() - 1.).abs() < 1e-5);
    }
}
