//! Code-native cockpit interiors and low-cost live instrument textures.
pub(crate) mod coverage;
pub(crate) mod hornet_hud;
pub(crate) mod raptor_interior;
pub(crate) mod hornet_interior;
use crate::{orbit::Orbit, physics::*};
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
#[derive(Component)]
pub struct Instrument {
    aircraft: crate::aircraft::Aircraft,
    vehicle: Vehicle,
    kind: u8,
    image: Handle<Image>,
    material: Handle<StandardMaterial>,
}
#[derive(Component)]
pub struct PilotControl {
    aircraft: crate::aircraft::Aircraft,
    vehicle: Vehicle,
    kind: u8,
    rest: Transform,
}
#[derive(Component)]
pub struct AttitudeBall;
#[derive(Component)]
pub struct InternalDisplay;
pub fn setup_ball(
    mut c: Commands,
    roots: Query<
        (Entity, Option<&crate::world::Jet>,Option<&crate::aircraft::visual::CockpitRoot>),
        Or<(With<crate::world::Jet>, With<crate::lunar::Cabin>,With<crate::aircraft::visual::CockpitRoot>)>,
    >,
    ball: Res<crate::navball::Navball>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (root, jet, fleet) in &roots {
        let aircraft=jet.is_some()||fleet.is_some();
        let classic=fleet.is_some_and(|p|p.0==crate::aircraft::Aircraft::Su27);
        let raptor=fleet.is_some_and(|p|p.0==crate::aircraft::Aircraft::F22);
        let z = if aircraft { -3.49 } else { -2.99 };
        let x = if classic{0.}else if raptor{-0.28}else if aircraft { -0.35 } else { -0.33 };
        let y = if classic{0.60}else if raptor{0.24}else if aircraft { 0.84 } else { 0.85 };
        let size = if classic{0.18}else if aircraft { 0.14 } else { 0.18 };
        let mat = materials.add(StandardMaterial {
            base_color_texture: Some(ball.image.clone()),
            emissive_texture: Some(ball.image.clone()),
            emissive: LinearRgba::new(1.2, 1.2, 1.2, 1.0),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None,
            ..default()
        });
        let e = c
            .spawn((
                Mesh3d(meshes.add(Rectangle::new(size, size))),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(x, y, z),
                AttitudeBall,
                InternalDisplay,
                bevy::light::NotShadowCaster,
            ))
            .id();
        c.entity(root).add_child(e);
        let bezel = materials.add(StandardMaterial {
            base_color: Color::srgb(0.12, 0.15, 0.18),
            metallic: 0.5,
            ..default()
        });
        let ring = c
            .spawn((
                Mesh3d(meshes.add(Torus::new(size * 0.5 + 0.001, size * 0.5 + 0.008))),
                MeshMaterial3d(bezel.clone()),
                Transform::from_xyz(x, y, z + 0.002)
                    .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            ))
            .id();
        c.entity(root).add_child(ring);
        for (p, s) in [
            (
                Vec3::new(x, y, z - 0.04),
                Vec3::new(size + 0.02, size + 0.02, 0.075),
            ),
            (
                Vec3::new(x, y - 0.09, z - 0.09),
                Vec3::new(0.10, 0.10, 0.04),
            ),
        ] {
            let e = c
                .spawn((
                    Mesh3d(meshes.add(Cuboid::from_size(s))),
                    MeshMaterial3d(bezel.clone()),
                    Transform::from_translation(p),
                ))
                .id();
            c.entity(root).add_child(e);
        }
    }
}
pub fn eye(vehicle: Vehicle) -> Vec3 {
    if vehicle == Vehicle::Gripen {
        Vec3::new(0.0, 1.08, -2.65)
    } else {
        Vec3::new(0.0, 1.08, -2.15)
    }
}
pub fn setup(
    c: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    root: Entity,
    vehicle: Vehicle,
) {
    setup_for(c,meshes,materials,images,root,vehicle,crate::aircraft::Aircraft::Gripen);
}
pub fn setup_for(c:&mut Commands,meshes:&mut Assets<Mesh>,materials:&mut Assets<StandardMaterial>,images:&mut Assets<Image>,root:Entity,vehicle:Vehicle,aircraft:crate::aircraft::Aircraft){
    let dark = materials.add(StandardMaterial {
        base_color: if aircraft==crate::aircraft::Aircraft::Su27{Color::srgb(0.055,0.15,0.15)}else{Color::srgb(0.035, 0.045, 0.05)},
        perceptual_roughness: 0.85,
        ..default()
    });
    let metal = materials.add(StandardMaterial {
        base_color: Color::srgb(0.19, 0.23, 0.25),
        metallic: 0.6,
        perceptual_roughness: 0.48,
        ..default()
    });
    let cushion = materials.add(StandardMaterial {
        base_color: Color::srgb(0.19, 0.22, 0.15),
        perceptual_roughness: 0.95,
        ..default()
    });
    let strap = materials.add(StandardMaterial {
        base_color: Color::srgb(0.47, 0.43, 0.26),
        ..default()
    });
    let glass = materials.add(StandardMaterial {
        base_color: Color::srgba(0.2, 0.55, 0.45, 0.08),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        perceptual_roughness: 0.15,
        ..default()
    });
    let boxmesh = meshes.add(Cuboid::default());
    let space = vehicle == Vehicle::Spacecraft;
    let raptor = !space && aircraft == crate::aircraft::Aircraft::F22;
    let dz = if space { 0.5 } else { 0.0 };
    let part =
        |c: &mut Commands, pos: Vec3, scale: Vec3, q: Quat, material: Handle<StandardMaterial>| {
            let e = c
                .spawn((
                    Mesh3d(boxmesh.clone()),
                    MeshMaterial3d(material),
                    Transform::from_translation(pos + Vec3::Z * dz)
                        .with_scale(scale)
                        .with_rotation(q),
                ))
                .id();
            c.entity(root).add_child(e);
            e
        };
    if raptor {
        raptor_interior::setup(c, meshes, materials, root);
    }
    // The Raptor has a fitted tub and seat; retain the original shared cabin
    // only for other vehicles rather than overlapping two interiors.
    if aircraft==crate::aircraft::Aircraft::Hornet && !space {hornet_interior::setup(c,meshes,materials,images,root);}
    if !raptor && aircraft!=crate::aircraft::Aircraft::Hornet {
    // Open cockpit tub: floor, sidewalls, forward/rear bulkheads and side consoles.
    for (p, s) in [
        (Vec3::new(0.0, -0.15, -2.65), Vec3::new(0.95, 0.09, 2.55)),
        (Vec3::new(-0.43, 0.16, -2.7), Vec3::new(0.04, 0.55, 2.2)),
        (Vec3::new(0.43, 0.16, -2.7), Vec3::new(0.04, 0.55, 2.2)),
        (Vec3::new(0.0, 0.15, -1.5), Vec3::new(0.88, 0.60, 0.10)),
    ] {
        part(c, p, s, Quat::IDENTITY, dark.clone());
    }
    for side in [-1.0, 1.0] {
        part(
            c,
            Vec3::new(side * 0.37, 0.35, -2.65),
            Vec3::new(0.22, 0.15, if space { 1.65 } else { 1.30 }),
            Quat::IDENTITY,
            metal.clone(),
        );
        for row in 0..9 {
            if !space {
                continue;
            }
            for col in 0..2 {
                let p = Vec3::new(
                    side * (0.32 + col as f32 * 0.09),
                    0.445,
                    if space {
                        -3.30 + row as f32 * 0.16
                    } else {
                        -3.14 + row as f32 * 0.14
                    },
                );
                part(
                    c,
                    p,
                    Vec3::new(0.045, 0.025, 0.06),
                    Quat::IDENTITY,
                    dark.clone(),
                );
                part(
                    c,
                    p + Vec3::Y * 0.025,
                    Vec3::new(0.012, 0.035, 0.012),
                    Quat::from_rotation_x(-0.25),
                    strap.clone(),
                );
            }
        }
    }
    // Seat pan, reclined ejection seat, headrest, harness and overhead handles.
    part(
        c,
        Vec3::new(0.0, 0.05, -2.4),
        Vec3::new(0.49, 0.22, 0.6),
        Quat::IDENTITY,
        cushion.clone(),
    );
    part(
        c,
        Vec3::new(0.0, if space { 0.43 } else { 0.38 }, -2.05),
        Vec3::new(0.49, if space { 0.78 } else { 0.66 }, 0.14),
        Quat::from_rotation_x(-0.16),
        cushion.clone(),
    );
    part(
        c,
        Vec3::new(0.0, if space { 0.92 } else { 0.64 }, -1.98),
        Vec3::new(0.30, 0.25, 0.15),
        Quat::IDENTITY,
        dark.clone(),
    );
    for side in [-1.0, 1.0] {
        part(
            c,
            Vec3::new(side * 0.13, 0.45, -2.15),
            Vec3::new(0.055, 0.76, 0.018),
            Quat::from_rotation_z(side * 0.12),
            strap.clone(),
        );
    }
    }
    // Instrument panel and glare shield are physically in front of the pilot.
    part(
        c,
        Vec3::new(
            0.0,
            if space { 0.47 } else { 0.42 },
            if space { -3.62 } else { -3.56 },
        ),
        Vec3::new(
            if space { 0.92 } else { 0.72 },
            if space { 0.55 } else { 0.40 },
            0.12,
        ),
        Quat::from_rotation_x(-0.10),
        metal.clone(),
    );
    part(
        c,
        Vec3::new(
            0.0,
            if space { 0.72 } else { 0.65 },
            if space { -3.56 } else { -3.58 },
        ),
        Vec3::new(
            if space { 0.88 } else { 0.66 },
            0.07,
            if space { 0.45 } else { 0.20 },
        ),
        Quat::IDENTITY,
        dark.clone(),
    );
    for kind in 0..3 {
        if aircraft==crate::aircraft::Aircraft::Su27 && kind==1{continue;}
        let x = (kind as f32 - 1.0) * 0.29;
        if aircraft!=crate::aircraft::Aircraft::F35B || kind==0 {
        part(
            c,
            Vec3::new(if aircraft==crate::aircraft::Aircraft::F35B{0.}else{x}, if aircraft==crate::aircraft::Aircraft::F22&&kind==1{0.40}else{0.49}, -3.54),
            Vec3::new(if aircraft==crate::aircraft::Aircraft::F35B{0.9}else{0.28}, if aircraft==crate::aircraft::Aircraft::F22&&kind==1{0.42}else{0.25}, 0.03),
            Quat::IDENTITY,
            dark.clone(),
        );
        }
        let image = images.add(Image::new_fill(
            Extent3d {
                width: 256,
                height: 192,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[5, 12, 16, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        ));
        let mat = materials.add(StandardMaterial {
            base_color_texture: Some(image.clone()),
            emissive_texture: Some(image.clone()),
            emissive: LinearRgba::new(2.0, 2.0, 2.0, 1.0),
            unlit: false,
            cull_mode: None,
            ..default()
        });
        let raptor_main=aircraft==crate::aircraft::Aircraft::F22&&kind==1;
        let mesh = meshes.add(Rectangle::new(if aircraft==crate::aircraft::Aircraft::F35B&&kind<2{0.285}else{0.245}, if raptor_main{0.36}else{0.205}));
        let e = c
            .spawn((
                Mesh3d(mesh),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(x, if raptor_main{0.40}else{0.49}, -3.44 + dz),
                Instrument {
                    aircraft,
                    vehicle,
                    kind,
                    image,
                    material: mat,
                },
                InternalDisplay,
                bevy::light::NotShadowCaster,
            ))
            .id();
        c.entity(root).add_child(e);
        if !space && kind == 2 {
            use crate::combat::bvr::cockpit_radar::{BUTTON_FACE_Z, BUTTON_SIZE, label};
            let mut atlas = crate::cockpit_labels::FaceAtlas::new(
                crate::combat::bvr::cockpit_radar::bezel()
                    .into_iter()
                    .map(|(action, _)| label(action)),
                images,
                materials,
            );
            for (action, pos) in crate::combat::bvr::cockpit_radar::bezel() {
                // Recessed metal socket around a separate illuminated, labelled cap.
                part(
                    c,
                    pos - Vec3::Z * 0.008,
                    BUTTON_SIZE + Vec3::new(0.004, 0.004, 0.002),
                    Quat::IDENTITY,
                    metal.clone(),
                );
                let cap = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.035, 0.065, 0.085),
                    perceptual_roughness: 0.42,
                    metallic: 0.35,
                    ..default()
                });
                let e = part(c, pos, BUTTON_SIZE, Quat::IDENTITY, cap);
                c.entity(e).insert((
                    crate::combat::bvr::cockpit_radar::Bezel(action),
                    crate::aircraft::visual::AircraftPart(aircraft),
                    InternalDisplay,
                ));
                let face = c
                    .spawn((
                        Mesh3d(atlas.face(label(action), meshes)),
                        MeshMaterial3d(atlas.material.clone()),
                        Transform::from_xyz(
                            0.,
                            0.,
                            ((BUTTON_FACE_Z as f32) - pos.z) / BUTTON_SIZE.z,
                        ),
                        InternalDisplay,
                        bevy::light::NotShadowCaster,
                    ))
                    .id();
                c.entity(e).add_child(face);
            }
        }
        for side in [-1.0, 1.0] {
            for row in 0..4 {
                if !space {
                    continue;
                }
                part(
                    c,
                    Vec3::new(x + side * 0.135, 0.42 + row as f32 * 0.045, -3.51),
                    Vec3::new(0.015, 0.015, 0.025),
                    Quat::IDENTITY,
                    metal.clone(),
                );
            }
        }
    }
    if !space {
        crate::cockpit_controls::setup(c, meshes, materials, images, root, aircraft);
    }
    // Animated centre stick and port throttle. Pedals sit below the main panel.
    for (kind, p, s) in [
        (
            0,
            Vec3::new(if matches!(aircraft,crate::aircraft::Aircraft::F22|crate::aircraft::Aircraft::F35B){0.34}else{0.0}, 0.29, -2.93),
            Vec3::new(0.055, 0.34, 0.065),
        ),
        (
            1,
            Vec3::new(-0.35, 0.53, -2.8),
            Vec3::new(0.075, 0.09, 0.12),
        ),
    ] {
        let e = part(c, p, s, Quat::IDENTITY, dark.clone());
        c.entity(e).insert(PilotControl {
            aircraft,
            vehicle,
            kind,
            rest: Transform::from_translation(p + Vec3::Z * dz).with_scale(s),
        });
        if kind == 1 {
            // A supported low-profile throttle grip keeps outer console labels visible.
            let stem = c
                .spawn((
                    Mesh3d(meshes.add(Cylinder::default())),
                    MeshMaterial3d(metal.clone()),
                    Transform::from_xyz(0., -0.55, 0.).with_scale(Vec3::new(0.18, 0.90, 0.18)),
                ))
                .id();
            c.entity(e).add_child(stem);
        }
    }
    for side in [-1.0, 1.0] {
        let p = Vec3::new(side * 0.17, if raptor { 0.41 } else { 0.02 }, -3.3);
        let s = Vec3::new(0.16, 0.07, 0.19);
        let e = part(c, p, s, Quat::from_rotation_x(0.35), metal.clone());
        c.entity(e).insert(PilotControl {
            aircraft,
            vehicle,
            kind: 2,
            rest: Transform::from_translation(p + Vec3::Z * dz)
                .with_scale(s)
                .with_rotation(Quat::from_rotation_x(0.35)),
        });
    }
    // Canopy bows, windshield rails and HUD combiner.
    let tube = meshes.add(Cylinder::new(0.012, 1.0));
    for z in [if space { -3.7 } else { -3.05 }, -1.5] {
        // The operational Raptor's one-piece canopy has no forward bow.
        if aircraft==crate::aircraft::Aircraft::F22 {continue;}
        for i in 0..48 {
            let a = i as f32 / 48.0 * std::f32::consts::PI;
            let b = (i + 1) as f32 / 48.0 * std::f32::consts::PI;
            let p = if space {
                Vec3::new(0.51 * a.cos(), 0.57 + 0.70 * a.sin(), z + dz)
            } else {
                crate::world::canopy_point(z, a)
            };
            let end = if space {
                Vec3::new(0.51 * b.cos(), 0.57 + 0.70 * b.sin(), z + dz)
            } else {
                crate::world::canopy_point(z, b)
            };
            let d = end - p;
            let e = c
                .spawn((
                    Mesh3d(tube.clone()),
                    MeshMaterial3d(metal.clone()),
                    Transform::from_translation((p + end) * 0.5)
                        .with_rotation(Quat::from_rotation_arc(Vec3::Y, d.normalize()))
                        .with_scale(Vec3::new(1.0, d.length(), 1.0)),
                ))
                .id();
            if !space { c.entity(e).insert(crate::render_batch::RigidGeometry); }
            c.entity(root).add_child(e);
        }
    }
    if !space && aircraft!=crate::aircraft::Aircraft::F22 {
        // Swept windscreen side posts follow the glass instead of straight
        // cuboid rails projecting through the canopy.
        for angle in [std::f32::consts::PI * 0.18, std::f32::consts::PI * 0.82] {
            for j in 0..24 {
                let a = crate::world::canopy_point(-4.08 + 1.03 * j as f32 / 24.0, angle);
                let b = crate::world::canopy_point(-4.08 + 1.03 * (j + 1) as f32 / 24.0, angle);
                let d = b - a;
                let e = c
                    .spawn((
                        Mesh3d(tube.clone()),
                        crate::render_batch::RigidGeometry,
                        MeshMaterial3d(metal.clone()),
                        Transform::from_translation((a + b) * 0.5)
                            .with_rotation(Quat::from_rotation_arc(Vec3::Y, d.normalize()))
                            .with_scale(Vec3::new(1.0, d.length(), 1.0)),
                    ))
                    .id();
                c.entity(root).add_child(e);
            }
        }
    }
    for side in [-1.0, 1.0] {
        if aircraft==crate::aircraft::Aircraft::F22 {continue;}
        if !space {
            for j in 0..64 {
                let z = -4.08 + 2.73 * j as f32 / 64.0;
                let next = -4.08 + 2.73 * (j + 1) as f32 / 64.0;
                let a = crate::world::canopy_point(
                    z,
                    if side > 0.0 {
                        0.0
                    } else {
                        std::f32::consts::PI
                    },
                );
                let b = crate::world::canopy_point(
                    next,
                    if side > 0.0 {
                        0.0
                    } else {
                        std::f32::consts::PI
                    },
                );
                part(
                    c,
                    (a + b) * 0.5,
                    Vec3::new(0.022, 0.022, a.distance(b)),
                    Quat::from_rotation_arc(Vec3::Z, (b - a).normalize()),
                    metal.clone(),
                );
            }
            continue;
        }
        part(
            c,
            Vec3::new(
                side * if space { 0.46 } else { 0.50 },
                if space { 0.61 } else { 0.64 },
                -2.715,
            ),
            Vec3::new(0.045, 0.045, 2.73),
            Quat::IDENTITY,
            metal.clone(),
        );
    }
    // Preserve the combiner's angular calibration inside the narrower tub.
    let hud_root=if aircraft==crate::aircraft::Aircraft::Hornet {let e=c.spawn((Transform::from_scale(Vec3::new(1./0.75,1.,1.)),Visibility::Inherited)).id();c.entity(root).add_child(e);e}else{root};
    let combiner=part(
        c,
        Vec3::new(
            0.0,
            if space { 1.00 } else if aircraft==crate::aircraft::Aircraft::F22{1.03}else { 1.05 },
            if space { -3.48 } else if aircraft==crate::aircraft::Aircraft::F22{-3.12}else { -3.07 },
        ),
        Vec3::new(
            if space { 0.40 } else { 0.32 },
            if space { 0.24 } else { 0.20 },
            0.009,
        ),
        Quat::from_rotation_x(-0.14),
        glass,
    );
    if hud_root!=root {c.entity(hud_root).add_child(combiner);}
    if aircraft==crate::aircraft::Aircraft::F22{c.entity(combiner).insert(InternalDisplay);}
    let image = images.add(Image::new_fill(
        Extent3d {
            width: if aircraft==crate::aircraft::Aircraft::Hornet {hornet_hud::WIDTH as u32}else{256},
            height: if aircraft==crate::aircraft::Aircraft::Hornet {hornet_hud::HEIGHT as u32}else{192},
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ));
    let hud = materials.add(StandardMaterial {
        base_color: if aircraft==crate::aircraft::Aircraft::Hornet {Color::BLACK}else{Color::WHITE},
        base_color_texture: Some(image.clone()),
        emissive_texture: Some(image.clone()),
        emissive: LinearRgba::new(6.0, 6.0, 6.0, 1.0),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    });
    let hud_size=Vec2::new(if space {0.38}else{0.30},if space {0.22}else{0.18});
    let hud_rest=Transform::from_xyz(0.,if space {1.00}else if aircraft==crate::aircraft::Aircraft::F22{1.03}else{1.05},
        if space {-3.46+dz}else if aircraft==crate::aircraft::Aircraft::F22{-3.10}else{-3.05})
        .with_rotation(Quat::from_rotation_x(-0.14));
    let e = c
        .spawn((
            Mesh3d(meshes.add(Rectangle::new(
                if space { 0.38 } else { 0.30 },
                if space { 0.22 } else { 0.18 },
            ))),
            MeshMaterial3d(hud.clone()),
            Transform::from_xyz(
                0.0,
                if space { 1.00 } else if aircraft==crate::aircraft::Aircraft::F22{1.03}else { 1.05 },
                if space { -3.46 + dz } else if aircraft==crate::aircraft::Aircraft::F22{-3.10}else { -3.05 },
            )
            .with_rotation(Quat::from_rotation_x(-0.14)),
            Instrument {
                aircraft,
                vehicle,
                kind: 3,
                image,
                material: hud,
            },
            InternalDisplay,
            bevy::light::NotShadowCaster,
        ))
        .id();
    c.entity(hud_root).add_child(e);
    // Spacecraft pressure cabin uses an actual forward window, not the opaque hull skin.
    if !space && std::env::var_os("NORD_HUD_COVERAGE_REFERENCE").is_none() {
        c.entity(e).insert(coverage::Panel {rest:hud_rest,size:hud_size,empty:true});
    }
    if space {
        for side in [-1.0, 1.0] {
            part(
                c,
                Vec3::new(side * 0.63, 0.7, -3.78),
                Vec3::new(0.16, 1.0, 0.20),
                Quat::IDENTITY,
                metal.clone(),
            );
        }
        part(
            c,
            Vec3::new(0.0, 1.25, -3.78),
            Vec3::new(1.35, 0.13, 0.20),
            Quat::IDENTITY,
            metal.clone(),
        );
    }
}
pub fn dump(screens: &Query<&Instrument>, images: &Assets<Image>) {
    for screen in screens.iter() {
        if let Some(image) = images.get(&screen.image)
            && let Ok(dynamic) = image.clone().try_into_dynamic()
        {
            let prefix = if screen.vehicle == Vehicle::Gripen {
                "jet"
            } else {
                "space"
            };
            let _ = dynamic.save(format!("tests/{prefix}-panel-{}.png", screen.kind));
        }
    }
}
pub fn fleet_display(c:&mut Commands,meshes:&mut Assets<Mesh>,materials:&mut Assets<StandardMaterial>,images:&mut Assets<Image>,root:Entity,aircraft:crate::aircraft::Aircraft){
    if aircraft==crate::aircraft::Aircraft::Su27{return;}
    let (pos,size)=match aircraft {
        crate::aircraft::Aircraft::F35B=>(Vec3::new(0.,0.23,-3.43),Vec2::new(0.78,0.16)),
        crate::aircraft::Aircraft::Su27=>(Vec3::new(0.,0.78,-3.48),Vec2::new(0.23,0.14)),
        crate::aircraft::Aircraft::F22=>(Vec3::new(0.,0.77,-3.40),Vec2::new(0.30,0.13)),
        _=>(Vec3::new(0.,0.69,-3.48),Vec2::new(0.24,0.10)),
    };
    let image=images.add(Image::new_fill(Extent3d{width:256,height:192,depth_or_array_layers:1},TextureDimension::D2,&[0,0,0,255],TextureFormat::Rgba8UnormSrgb,RenderAssetUsages::default()));
    let material=materials.add(StandardMaterial{base_color_texture:Some(image.clone()),emissive_texture:Some(image.clone()),emissive:LinearRgba::new(2.,2.,2.,1.),unlit:true,..default()});
    let e=c.spawn((Mesh3d(meshes.add(Rectangle::from_size(size))),MeshMaterial3d(material.clone()),Transform::from_translation(pos),InternalDisplay,Instrument{vehicle:Vehicle::Gripen,aircraft,kind:4,image,material},bevy::light::NotShadowCaster)).id();c.entity(root).add_child(e);
}
fn glyph(c: char) -> [u8; 7] {
    match c {
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'J' => [7, 2, 2, 2, 2, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 25, 21, 19, 19, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        '+' => [0, 4, 4, 31, 4, 4, 0],
        '.' => [0, 0, 0, 0, 0, 6, 6],
        '/' => [1, 2, 2, 4, 8, 8, 16],
        '%' => [17, 2, 4, 4, 8, 16, 17],
        _ => [0; 7],
    }
}
fn pixel(data: &mut [u8], x: i32, y: i32, color: [u8; 4]) {
    if (0..256).contains(&x) && (0..192).contains(&y) {
        let i = (y as usize * 256 + x as usize) * 4;
        data[i..i + 4].copy_from_slice(&color);
    }
}
fn text(data: &mut [u8], s: &str, y: i32, color: [u8; 4]) {
    for (index, c) in s.chars().take(20).enumerate() {
        let pattern = glyph(c.to_ascii_uppercase());
        for (row, bits) in pattern.iter().enumerate() {
            for col in 0..5 {
                if bits & (1 << (4 - col)) != 0 {
                    for dy in 0..2 {
                        for dx in 0..2 {
                            pixel(
                                data,
                                8 + index as i32 * 12 + col * 2 + dx,
                                y + row as i32 * 2 + dy,
                                color,
                            );
                        }
                    }
                }
            }
        }
    }
}
fn label(data: &mut [u8], s: &str, x: i32, y: i32, color: [u8; 4]) {
    for (n, c) in s.chars().enumerate() {
        for (row, bits) in glyph(c.to_ascii_uppercase()).iter().enumerate() {
            for col in 0..5 {
                if bits & (1 << (4 - col)) != 0 {
                    pixel(data, x + n as i32 * 6 + col, y + row as i32, color);
                }
            }
        }
    }
}
pub(crate) fn radar_label(data: &mut [u8], s: &str, x: i32, y: i32, color: [u8; 4]) {
    label(data, s, x, y, color);
}
pub(crate) fn radar_line(data: &mut [u8], x0: i32, y0: i32, x1: i32, y1: i32, color: [u8; 4]) {
    let steps = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
    for i in 0..=steps {
        pixel(
            data,
            x0 + (x1 - x0) * i / steps,
            y0 + (y1 - y0) * i / steps,
            color,
        );
    }
}
fn number(data: &mut [u8], s: &str, x: i32, y: i32, color: [u8; 4]) {
    for (n, c) in s.chars().enumerate() {
        for (row, bits) in glyph(c).iter().enumerate() {
            for col in 0..5 {
                if bits & (1 << (4 - col)) != 0 {
                    for dx in 0..2 {
                        for dy in 0..2 {
                            pixel(
                                data,
                                x + n as i32 * 12 + col * 2 + dx,
                                y + row as i32 * 2 + dy,
                                color,
                            );
                        }
                    }
                }
            }
        }
    }
}
fn line(data: &mut [u8], a: Vec2, b: Vec2, color: [u8; 4]) {
    let count = (b - a).length().ceil().max(1.0) as i32;
    for i in 0..=count {
        let p = a.lerp(b, i as f32 / count as f32);
        pixel(data, p.x.round() as i32, p.y.round() as i32, color);
    }
}
fn gun_ring(data: &mut [u8], centre: Vec2, radius: f32, color: [u8; 4]) {
    for i in 0..32 {
        let a = std::f32::consts::TAU * i as f32 / 32.;
        let b = std::f32::consts::TAU * (i + 1) as f32 / 32.;
        line(data, centre + Vec2::new(a.cos(), a.sin()) * radius,
            centre + Vec2::new(b.cos(), b.sin()) * radius, color);
    }
}
pub(crate) fn hud_projection(vehicle: Vehicle, direction: Vec3) -> Option<Vec2> {
    let dz = if vehicle == Vehicle::Spacecraft {
        0.5
    } else {
        0.0
    };
    let q = Quat::from_rotation_x(-0.14).conjugate();
    let space = vehicle == Vehicle::Spacecraft;
    let y = if space { 1.00 } else { 1.05 };
    let z = if space { -3.46 + dz } else { -3.05 };
    let origin = q * (eye(vehicle) - Vec3::new(0.0, y, z));
    let ray = q * direction;
    if ray.z >= -0.01 {
        return None;
    }
    let p = origin + ray * (-origin.z / ray.z);
    Some(Vec2::new(
        128.0 + p.x / if space { 0.38 } else { 0.30 } * 256.0,
        96.0 - p.y / if space { 0.22 } else { 0.18 } * 192.0,
    ))
}
#[cfg(test)]
fn hud(data: &mut [u8], f: &Flight) {
    hud_layout(data, f, false);
}
fn hud_layout(data: &mut [u8], f: &Flight, simple: bool) {
    let green = [100, 245, 165, 255];
    let amber = [255, 190, 65, 255];
    let inverse = f.orientation.conjugate();
    let up = f.up();
    let forward = f.orientation * -bevy::math::DVec3::Z;
    let horizontal = (forward - up * forward.dot(up)).normalize_or_zero();
    let right = horizontal.cross(up).normalize_or_zero();
    for pitch in (-80..=80).step_by(if simple { 10 } else { 5 }) {
        let a = (pitch as f64).to_radians();
        let d = horizontal * a.cos() + up * a.sin();
        let left = hud_projection(f.vehicle, (inverse * (d - right * 0.060)).as_vec3());
        let r = hud_projection(f.vehicle, (inverse * (d + right * 0.060)).as_vec3());
        if let (Some(l), Some(r)) = (left, r) {
            if l.y.min(r.y) > 32.0
                && l.y.max(r.y) < 153.0
                && l.x.min(r.x) > 50.0
                && l.x.max(r.x) < 206.0
            {
                let mid = (l + r) * 0.5;
                if pitch < 0 {
                    line(data, l, l.lerp(mid, 0.65), green);
                    line(data, r, r.lerp(mid, 0.65), green);
                } else {
                    line(data, l, l.lerp(mid, 0.75), green);
                    line(data, r, r.lerp(mid, 0.75), green);
                }
                label(
                    data,
                    &format!("{pitch}"),
                    l.x as i32 - 20,
                    l.y as i32 - 3,
                    green,
                );
            }
        }
    }
    let heading = f.heading() as f32;
    for tick in -4..=4 {
        let h = (heading / 5.0).round() * 5.0 + tick as f32 * 5.0;
        let x = 128.0 + (h - heading) * 3.0;
        line(
            data,
            Vec2::new(x, 13.0),
            Vec2::new(x, if (h as i32) % 10 == 0 { 21.0 } else { 17.0 }),
            green,
        );
        if (h as i32) % 10 == 0 {
            label(
                data,
                &format!("{:02}", ((h + 360.0) % 360.0 / 10.0).round() as i32),
                x as i32 - 6,
                3,
                green,
            );
        }
    }
    label(data, &format!("HDG {:03.0}", heading), 105, 25, green);
    label(data, "M/S", 8, 60, green);
    number(data, &format!("{:.0}", f.airspeed), 6, 70, green);
    label(data, "ALT M", 211, 60, green);
    let alt = if f.altitude() > 99999.0 {
        format!("{:.0}K", f.altitude() / 1000.0)
    } else {
        format!("{:.0}", f.altitude())
    };
    number(data, &alt, 196, 70, green);
    label(data, &format!("AGL {:.0}", f.agl()), 202, 87, green);
    label(
        data,
        &format!("VS {:+.1}", f.vertical_speed()),
        195,
        100,
        green,
    );
    label(data, &format!("M {:.2}", f.mach), 8, 89, green);
    label(data, &format!("G {:+.1}", f.g_load), 8, 101, green);
    if let Some(p) = hud_projection(f.vehicle, -Vec3::Z) {
        line(
            data,
            p + Vec2::new(-15.0, 0.0),
            p + Vec2::new(-5.0, 0.0),
            green,
        );
        line(
            data,
            p + Vec2::new(5.0, 0.0),
            p + Vec2::new(15.0, 0.0),
            green,
        );
        pixel(data, p.x as i32, p.y as i32, green);
    }
    let velocity = if f.vehicle == Vehicle::Spacecraft {
        crate::space::relative(f).1
    } else {
        f.velocity
    };
    if velocity.length() > 0.5 {
        if let Some(p) = hud_projection(f.vehicle, (inverse * velocity.normalize()).as_vec3()) {
            let bounded = p.clamp(Vec2::new(55.0, 40.0), Vec2::new(199.0, 146.0));
            let limited = bounded.distance(p) > 1.0;
            let color = if limited { amber } else { green };
            for i in 0..48 {
                let a = i as f32 * std::f32::consts::TAU / 48.0;
                pixel(
                    data,
                    (bounded.x + 5.0 * a.cos()).round() as i32,
                    (bounded.y + 5.0 * a.sin()).round() as i32,
                    color,
                );
            }
            line(
                data,
                bounded - Vec2::new(13.0, 0.0),
                bounded - Vec2::new(5.0, 0.0),
                color,
            );
            line(
                data,
                bounded + Vec2::new(5.0, 0.0),
                bounded + Vec2::new(13.0, 0.0),
                color,
            );
            line(
                data,
                bounded - Vec2::new(0.0, 12.0),
                bounded - Vec2::new(0.0, 5.0),
                color,
            );
            if limited {
                label(data, "FPM LIMIT", 102, 151, amber);
            }
        }
    }
    label(
        data,
        &format!(
            "AOA {:+.1}  FLP {:.0}  THR {:.0}",
            f.alpha.to_degrees(),
            f.flaps * 100.0,
            f.throttle * 100.0
        ),
        22,
        166,
        green,
    );
    label(
        data,
        &format!(
            "FUEL {:.0}  GEAR {}",
            f.fuel,
            if f.gear_extension > 0.98 {
                "DOWN"
            } else if f.gear_extension < 0.01 {
                "UP"
            } else {
                "MOVING"
            }
        ),
        38,
        179,
        green,
    );
    let warning = if f.crashed {
        "AIRFRAME LOST"
    } else if f.fuel < 100.0 {
        "LOW FUEL"
    } else if f.panel_attachment < 0.55 && f.airspeed > 25.0 {
        "STALL"
    } else if f.gear_extension < 0.98 && f.agl() < 60.0 {
        "GEAR"
    } else if !f.engine {
        "ENGINE OFF"
    } else {
        ""
    };
    label(data, warning, 98, 36, amber);
}
fn combat_combiner(data: &mut [u8], f: &Flight, combat: &crate::combat::Combat, simple: bool) {
    let green = [100, 245, 165, 255];
    if let Some(lines) = crate::combat::hud::cockpit_lines(combat, f) {
        for y in 149..192 {
            for x in 0..256 {
                pixel(data, x, y, [0, 0, 0, 0]);
            }
        }
        label(data, &lines[0], 38, 166, green);
        label(data, &lines[1], 20, 179, [255, 190, 65, 255]);
        if f.vehicle == Vehicle::Gripen {
            if let Some(text) = crate::combat::bvr::cockpit_radar::selected_hud_line(combat, f) {
                label(data, &text, 65, 151, green);
            }
        } else if let Some(target) = combat.targets.get(combat.selected).filter(|t| t.alive) {
            label(
                data,
                &format!(
                    "T{:02} {:.2}KM Vc {:+.0}",
                    combat.selected + 1,
                    target.position.distance(f.position) / 1000.0,
                    crate::combat::hud::closing(f, target)
                ),
                65,
                151,
                green,
            );
        }
        let inverse = f.orientation.conjugate();
        let projection = |p: bevy::math::DVec3| {
            hud_projection(f.vehicle, (inverse * (p - f.position)).as_vec3())
                .filter(|p| p.x > 50.0 && p.x < 206.0 && p.y > 38.0 && p.y < 146.0)
        };
        if f.vehicle == Vehicle::Gripen {
            // Small fixed-direction ring: where the gun points now. Larger
            // director ring: where its rounds arrive relative to the selected
            // aircraft. Place the latter on the aircraft, not on a lead marker.
            let cue = combat.targets.get(combat.selected)
                .and_then(|target| crate::combat::hud::gun_pipper(f, target));
            let bore = cue.as_ref().map_or(
                f.position + f.orientation * bevy::math::DVec3::new(0.4, -0.1, -2006.), |cue| cue.bore);
            if let Some(p) = projection(bore) {
                gun_ring(data, p, 3., [85, 180, 125, 230]);
            }
            if let Some(cue) = cue {
                if let Some(p) = projection(cue.impact)
                    .filter(|p| p.x > 59. && p.x < 197. && p.y > 47. && p.y < 137.)
                {
                    gun_ring(data, p, 6., green);
                    pixel(data, p.x.round() as i32, p.y.round() as i32, green);
                    // Range tick winds counter-clockwise as range decreases.
                    let angle = -std::f32::consts::FRAC_PI_2
                        + std::f32::consts::TAU * (cue.range as f32 / 2000.).clamp(0., 1.);
                    let radial = Vec2::new(angle.cos(), angle.sin());
                    line(data, p + radial * 6., p + radial * 9., green);
                    if cue.aligned && combat.cannon_available(f) {
                        label(data, "SHOOT", 113, 49, green);
                    }
                    if !simple {
                        label(data, &format!("GUN {:.0}M {:.2}S", cue.range, cue.time), 8, 125, green);
                    }
                }
            }
        }
        if !simple {
            let progress = crate::combat::hud::lock_fraction(combat);
            line(
                data,
                Vec2::new(100.0, 160.0),
                Vec2::new(156.0, 160.0),
                [65, 145, 115, 180],
            );
            line(
                data,
                Vec2::new(100.0, 160.0),
                Vec2::new(100.0 + 56.0 * progress, 160.0),
                green,
            );
        }
    }
}
#[derive(Default)]
pub(crate) struct DisplayCache {
    previous: Option<(Vehicle, u64, u64)>,
    pages: std::collections::HashMap<bevy::asset::AssetId<Image>, Vec<String>>,
    radar: [Vec<u8>; 2],
}
fn clear_display(data: &mut [u8], hud: bool, powered: bool) {
    let color = if hud { [0, 0, 0, 0] } else if powered { [5, 14, 18, 255] } else { [1, 2, 3, 255] };
    for pixel in data.chunks_exact_mut(4) { pixel.copy_from_slice(&color); }
}
fn paint_numeric(data:&mut [u8],lines:&[String],old:Option<&Vec<String>>,kind:u8,green:[u8;4])->Option<(u32,u32)> {
    let full=old.is_none_or(|old|old.len()!=lines.len());
    let (mut top,mut bottom)=if full {clear_display(data,false,true);(0,192)}else{(192,0)};
    for (i,line) in lines.iter().enumerate() {
        if !full && old.is_some_and(|old|old[i]==*line) {continue;}
        let y=10+i as i32*if kind==1 {22}else{25};
        let first=y.clamp(0,192) as usize;
        let last=(y+14).clamp(0,192) as usize;
        if first>=last {continue;}
        if !full {
            for p in data[first*256*4..last*256*4].chunks_exact_mut(4) {p.copy_from_slice(&[5,14,18,255]);}
            top=top.min(first as u32);bottom=bottom.max(last as u32);
        }
        text(data,line,y,green);
    }
    (top<bottom).then_some((top,bottom))
}
pub fn update(
    mut rwr: Local<crate::combat::rwr::Receiver>,
    radar: Option<Res<crate::combat::bvr::cockpit_radar::Display>>,
    f: Res<Flight>,
    combat: Res<crate::combat::Combat>,
    environment: Res<Environment>,
    time: Res<Time>,
    mut last: Local<crate::render_cache::PresentationClock>,
    screens: Query<&Instrument>,
    mut images: ResMut<Assets<Image>>,
    uploads: Res<crate::display_upload::DisplayUploads>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cache: Local<DisplayCache>,
    (mut controls,mut coverage): (
        Query<(&PilotControl, &mut Transform)>,
        Query<(&Instrument,&mut coverage::Panel,&mut Transform,&mut Visibility),(Without<PilotControl>,With<InternalDisplay>)>,
    ),
    view: Res<crate::ViewState>,
    mut internal: Query<
        &mut Visibility,
        (
            With<InternalDisplay>,
            Without<coverage::Panel>,
            Without<crate::navball::Overlay>,
            Without<crate::instruments::Overlay>,
        ),
    >,
    mut overlays: Query<
        &mut Visibility,
        (
            Without<InternalDisplay>,
            Or<(
                With<crate::navball::Overlay>,
                With<crate::instruments::Overlay>,
            )>,
        ),
    >,
) {
    for mut v in &mut internal {
        v.set_if_neq(if !view.map && view.index == 3 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
    for (_,panel,_,mut visibility) in &mut coverage {
        visibility.set_if_neq(if !view.map && view.index==3 && !panel.empty {Visibility::Inherited}else{Visibility::Hidden});
    }
    for mut v in &mut overlays {
        v.set_if_neq(
            if view.map || view.index == 3 || view.simple || view.following() {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            },
        );
    }
    for (control, mut transform) in &mut controls {
        if control.vehicle != f.vehicle || (f.vehicle==Vehicle::Gripen && control.aircraft!=f.aircraft) {
            continue;
        }
        let mut desired = control.rest;
        match control.kind {
            0 => {
                desired.rotation = Quat::from_rotation_x(f.controls.x as f32 * 0.20)
                    * Quat::from_rotation_z(-f.controls.z as f32 * 0.20)
            }
            1 => desired.translation.z -= f.throttle as f32 * 0.16,
            _ => {
                desired.translation.z += f.controls.y as f32 * desired.translation.x.signum() * 0.06
            }
        }
        transform.set_if_neq(desired);
    }
    if view.map || view.index != 3 || view.following() {
        return;
    }
    let _measure = crate::performance::span(5);
    if !last.due(time.elapsed_secs(), if f.aircraft==crate::aircraft::Aircraft::Hornet{1./30.}else{0.10}, &view) {
        return;
    }
    rwr.update(&combat, &f);
    let tank_signature=f.hornet.tanks.fuel.iter().enumerate().fold(0u64,|hash,(i,fuel)|hash ^ fuel.unwrap_or(-1.).to_bits().rotate_left((i*17)as u32));
    let signature = tank_signature ^ f.fuel.to_bits().rotate_left(27) ^ (f.hornet.hud.mode as u64).rotate_left(3) ^ (f.hornet.hud.reject as u64).rotate_left(5) ^ (f.hornet.hud.caged as u64).rotate_left(9) ^ (f.hornet.hud.radar_alt as u64).rotate_left(10) ^ (f.hornet.hud.brightness.to_bits() as u64).rotate_left(13) ^ f.elapsed.to_bits()
        ^ rwr.fingerprint().rotate_left(31)
        ^ f.throttle.to_bits().rotate_left(7)
        ^ f.gear_extension.to_bits().rotate_left(19)
        ^ (f.battery as u64)
        ^ ((f.engine as u64) << 1)
        ^ ((f.lights as u64) << 2)
        ^ combat.lock.to_bits().rotate_left(11)
        ^ ((combat.selected as u64) << 32)
        ^ ((combat.ammo[0] as u64) << 40)
        ^ ((combat.ammo[1] as u64) << 48)
        ^ ((combat.weapon as u64) << 56)
        ^ ((view.simple as u64) << 57)
        ^ environment.day_phase.to_bits().rotate_left(29)
        ^ radar.as_ref().map_or(0, |r| r.revision.rotate_left(23))
        ^ {
            let r = crate::combat::bvr::cockpit_radar::radar(&combat);
            r.scan.to_bits()
                ^ (r.mode as u64).rotate_left(17)
                ^ (r.powered as u64).rotate_left(18)
                ^ r.elevation.to_bits()
                ^ (r.range as u64).rotate_left(21)
        };
    if cache.previous
        .as_ref()
        .is_some_and(|p| p.0 == f.vehicle && p.1 == signature && p.2 == view.generation)
    {
        return;
    }
    cache.previous = Some((f.vehicle, signature, view.generation));
    let mut radar_ready = [false; 2];
    let orbit = Orbit::from_flight(&f);
    let green = [100, 245, 165, 255];
    for screen in &screens {
        if screen.vehicle != f.vehicle || (f.vehicle==Vehicle::Gripen && screen.aircraft!=f.aircraft) {
            continue;
        }
        let daylight =
            ((environment.sun_direction().dot(f.up()) as f32 + 0.06) / 0.18).clamp(0.0, 1.0);
        let intensity = (if screen.kind == 3 {6.0} else {2.0}) * (0.055 + 0.945 * daylight);
        let emissive = LinearRgba::new(intensity, intensity, intensity, 1.0);
        if materials
            .get(&screen.material)
            .is_some_and(|m| m.emissive != emissive)
        {
            materials.get_mut(&screen.material).unwrap().emissive = emissive;
        }
        let Some(image) = images.get_mut_untracked(&screen.image) else {
            continue;
        };
        let Some(data) = image.data.as_mut() else {
            continue;
        };
        if !f.battery {
            clear_display(data, screen.kind == 3, false);
            if screen.kind==3 {
                for (instrument,mut panel,mut transform,mut visibility) in &mut coverage {
                    if instrument.image==screen.image {coverage::apply(&mut panel,&mut transform,&mut visibility,&mut materials,&screen.material,data);}
                }
            }
            cache.pages.remove(&screen.image.id());
            uploads.publish(&screen.image, image);
            continue;
        }
        if screen.kind == 3 {
            clear_display(data, true, true);
            if f.aircraft==crate::aircraft::Aircraft::Hornet {
                hornet_hud::draw(data,&f,&combat);
            }else{
            hud_layout(
                data,
                &f,
                view.simple || combat.active || f.vehicle == Vehicle::Gripen,
            );
            combat_combiner(data, &f, &combat, view.simple);
            crate::combat::bvr::cockpit_radar::draw_hud(data, &combat, &f, view.simple);
            }
            for (instrument,mut panel,mut transform,mut visibility) in &mut coverage {
                if instrument.image==screen.image {coverage::apply(&mut panel,&mut transform,&mut visibility,&mut materials,&screen.material,data);}
            }
            uploads.publish(&screen.image, image);
            continue;
        }
        if screen.kind == 0 && f.vehicle == Vehicle::Gripen {
            clear_display(data, false, true);
            crate::combat::rwr::draw(data, &rwr, &f);
            uploads.publish(&screen.image, image);
            continue;
        }
        if (screen.kind == 2 && f.vehicle == Vehicle::Gripen)
            || (screen.aircraft == crate::aircraft::Aircraft::F22 && screen.kind == 1) {
            let tactical = screen.kind == 1 || radar.as_ref().is_some_and(|r| r.tactical);
            let page = tactical as usize;
            if !radar_ready[page] {
                cache.radar[page].resize(data.len(), 0);
                clear_display(&mut cache.radar[page], false, true);
                crate::combat::bvr::cockpit_radar::draw(&mut cache.radar[page], &combat, &f, tactical);
                radar_ready[page] = true;
            }
            data.copy_from_slice(&cache.radar[page]);
            uploads.publish(&screen.image,image);
            continue;
        }
        let mut lines = match screen.kind {
            4 if f.aircraft==crate::aircraft::Aircraft::F22=>vec!["FLIGHT / SYSTEMS".into(),format!("AOA {:+.1} / G {:+.1}",f.alpha.to_degrees(),f.g_load),format!("FBW {}  HYD {:.0}%",if f.assist{"ON"}else{"DIRECT"},f.damage.hydraulics*100.),format!("ENG L {:.0} R {:.0}",f.systems.engines[0]*100.,f.systems.engines[1]*100.),format!("TVC L {:+.0} R {:+.0}",f.systems.nozzle[0].to_degrees(),f.systems.nozzle[1].to_degrees()),format!("FUEL {:.0} KG",f.fuel),format!("BAYS {:.0}% / GEAR {:.0}%",f.systems.bays*100.,f.gear_extension*100.)],
            4 if f.aircraft==crate::aircraft::Aircraft::Hornet=>vec!["HORNET FLIGHT CONTROLS".into(),crate::aircraft::hornet::fbw::summary(&f),format!("HYD {:.0}% / TRIM {:+.1}",f.hornet.hydraulic_pressure*f.damage.hydraulics*100.,f.hornet.trim.to_degrees()),format!("AOA {:+.1} / G {:+.1}",f.alpha.to_degrees(),f.g_load),format!("MANUAL FLAPS {:.0}%",f.flaps*100.),"T NORMAL / DIRECT".into(),format!("EXT FUEL {:.0}KG / {} TK",crate::aircraft::hornet::stores::fuel(&f),f.hornet.tanks.fuel.iter().flatten().count())],
            4=>vec![f.aircraft.name().into(),format!("SPD {:.0} / M {:.2}",f.airspeed,f.mach),format!("ALT {:.0} / G {:.1}",f.altitude(),f.g_load),format!("FBW {}",if f.assist{"ON"}else{"DIRECT"}),format!("ENG L {:.0} R {:.0}",f.systems.engines[0]*100.,f.systems.engines[1]*100.),format!("LIFT {:.0}% BAY {:.0}%",f.systems.conversion*100.,f.systems.bays*100.),format!("FUEL {:.0} KG",f.fuel),"Y STOVL / ALT+B BAYS".into()],
            3 => vec![
                format!("HDG {:03.0}", f.heading()),
                format!("SPD {:.0} M/S", f.airspeed),
                format!("ALT {:.0} M", f.altitude()),
                format!("VS {:+.1} M/S", f.velocity.dot(f.up())),
                format!(
                    "P {:+.0} R {:+.0}",
                    f.pitch().to_degrees(),
                    f.roll().to_degrees()
                ),
            ],
            0 => vec![
                "PRIMARY FLIGHT".into(),
                format!("SPD {:.0} M/S", f.airspeed),
                format!("ALT {:.0} M", f.altitude()),
                format!("HDG {:.0}", f.heading()),
                format!("PITCH {:+.1}", f.pitch().to_degrees()),
                format!("BANK {:+.1}", f.roll().to_degrees()),
                format!("VS {:+.1}", f.velocity.dot(f.up())),
            ],
            1 => vec![
                "ENGINE / SYSTEMS".into(),
                format!("THR {:.0}%", f.throttle * 100.0),
                format!("FORCE {:.0} KN", f.thrust / 1000.0),
                format!("FUEL {:.0} KG", f.fuel),
                format!("SPOOL {:.0}%", f.spool * 100.0),
                format!(
                    "LIGHTS {}",
                    if f.lights && f.battery { "ON" } else { "OFF" }
                ),
                format!("GEAR {:.0}%", f.gear_extension * 100.0),
                format!("HEALTH {:.0}%", f.integrity * 100.0),
            ],
            _ => {
                if f.vehicle == Vehicle::Spacecraft {
                    vec![
                        format!("{} NAVIGATION", f.space.body.label()),
                        format!("PE {:.1} KM", orbit.periapsis / 1000.0),
                        format!("AP {:.1} KM", orbit.apoapsis.unwrap_or(-1.0) / 1000.0),
                        format!("ECC {:.4}", orbit.eccentricity),
                        format!("PER {:.0} S", orbit.period.unwrap_or(0.0)),
                        format!("SAS MODE {}", f.sas_mode),
                        "M / ORBIT MAP".into(),
                    ]
                } else {
                    vec![
                        "NAV / APPROACH".into(),
                        "SATENAS / RWY 19".into(),
                        format!("RANGE {:.1} KM", (f.position.z - 1000.0).max(0.0) / 1000.0),
                        format!("CROSS {:.0} M", f.position.x),
                        format!("AOA {:+.1}", f.alpha.to_degrees()),
                        format!("FLAP {:.0}%", f.flaps * 100.0),
                        format!("FBW {}", if f.aircraft==crate::aircraft::Aircraft::Hornet{crate::aircraft::hornet::fbw::label(&f)}else if f.assist { "ON" } else { "OFF" }),
                    ]
                }
            }
        };
        if screen.kind == 2 {
            if let Some(weapons) = crate::combat::hud::weapon_display(&combat, &f) {
                lines = weapons;
            }
        }
        if cache.pages.get(&screen.image.id()) == Some(&lines) { continue; }
        let old=cache.pages.get(&screen.image.id());
        if let Some((top,bottom))=paint_numeric(data,&lines,old,screen.kind,green) {uploads.publish_rows(&screen.image,image,top,bottom);}
        cache.pages.insert(screen.image.id(), lines);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retained_numeric_rows_match_full_redraw_including_shorter_text_and_power_cycle() {
        for kind in [1,4] {
            let mut retained=vec![0;256*192*4];
            let mut old=None;
            for lines in [vec!["FLIGHT", "FUEL 2400 KG", "GEAR 100%"],vec!["FLIGHT", "FUEL 2 KG", "GEAR 100%"],vec!["FLIGHT", "FUEL 2 KG"],vec!["FLIGHT", "FUEL 2 KG"]] {
                let lines=lines.into_iter().map(String::from).collect::<Vec<_>>();
                let mut reference=vec![0;256*192*4];
                paint_numeric(&mut reference,&lines,None,kind,[100,245,165,255]);
                paint_numeric(&mut retained,&lines,old.as_ref(),kind,[100,245,165,255]);
                assert_eq!(reference,retained);
                assert_eq!(paint_numeric(&mut retained,&lines,Some(&lines),kind,[100,245,165,255]),None);
                old=Some(lines);
            }
            clear_display(&mut retained,false,false);
            let lines=old.unwrap();
            paint_numeric(&mut retained,&lines,None,kind,[100,245,165,255]);
            let mut reference=vec![0;256*192*4];
            paint_numeric(&mut reference,&lines,None,kind,[100,245,165,255]);
            assert_eq!(reference,retained);
        }
    }
    #[test]
    fn hud_projection_places_velocity_on_the_correct_side() {
        let centre = hud_projection(Vehicle::Gripen, -Vec3::Z).unwrap();
        assert!(centre.x > 50.0 && centre.x < 206.0 && centre.y > 38.0 && centre.y < 146.0);
        let right = hud_projection(Vehicle::Gripen, Vec3::new(0.1, 0.0, -1.0)).unwrap();
        let up = hud_projection(Vehicle::Gripen, Vec3::new(0.0, 0.1, -1.0)).unwrap();
        assert!(right.x > centre.x && up.y < centre.y);
        assert!(hud_projection(Vehicle::Gripen, Vec3::Z).is_none());
    }
    #[test]
    fn hud_renders_live_symbols_on_transparent_background() {
        let mut data = vec![0u8; 256 * 192 * 4];
        let f = Flight::approach();
        hud(&mut data, &f);
        assert!(data.chunks_exact(4).filter(|p| p[3] > 0).count() > 300);
        assert_eq!(data[3], 0);
    }
}
