use super::*;
use bevy::{asset::RenderAssetUsages, mesh::Indices, render::render_resource::PrimitiveTopology};
#[derive(Component)]
pub struct NavalText;
#[derive(Component)]
pub struct NavalVisual {
    kind: u8,
    index: usize,
}
#[derive(Resource)]
pub struct Models {
    standard: Handle<Mesh>,
    small: Handle<Mesh>,
}
fn mesh(polys: Vec<Vec<Vec3>>) -> Mesh {
    let mut p = vec![];
    let mut indices = vec![];
    for poly in polys {
        let base = p.len() as u32;
        for v in &poly {
            p.push(v.to_array());
        }
        for i in 1..poly.len() - 1 {
            indices.extend([base, base + i as u32, base + i as u32 + 1]);
        }
    }
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let len = p.len();
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, p);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.]; len]);
    m.insert_indices(Indices::U32(indices));
    m.duplicate_vertices();
    m.compute_flat_normals();
    m
}
fn bomber() -> Mesh {
    let mut polys = vec![];
    let sections = [
        (-19., 0.12, 0.1),
        (-16., 1.1, 0.9),
        (-11., 1.7, 1.7),
        (-5., 1.9, 1.9),
        (6., 1.6, 1.6),
        (14., 0.8, 1.0),
        (19., 0.05, 0.2),
    ];
    for ab in sections.windows(2) {
        for i in 0..32 {
            let a = i as f32 * std::f32::consts::TAU / 32.;
            let b = (i + 1) as f32 * std::f32::consts::TAU / 32.;
            let (z, r, h) = ab[0];
            let (w, s, k) = ab[1];
            polys.push(vec![
                Vec3::new(a.cos() * r, a.sin() * h, z),
                Vec3::new(b.cos() * r, b.sin() * h, z),
                Vec3::new(b.cos() * s, b.sin() * k, w),
                Vec3::new(a.cos() * s, a.sin() * k, w),
            ]);
        }
    }
    for side in [-1., 1.] {
        let wing = [
            Vec3::new(side * 1.4, 0., -6.),
            Vec3::new(side * 16., 0.4, 6.),
            Vec3::new(side * 14.8, 0.4, 8.),
            Vec3::new(side * 1.4, 0., 3.),
        ];
        polys.push(wing.to_vec());
        polys.push(wing.iter().rev().map(|p| *p - Vec3::Y * 0.25).collect());
        for i in 0..4 {
            polys.push(vec![
                wing[i],
                wing[(i + 1) % 4],
                wing[(i + 1) % 4] - Vec3::Y * 0.25,
                wing[i] - Vec3::Y * 0.25,
            ]);
        }
        polys.push(vec![
            Vec3::new(side * 0.8, 0.7, 12.),
            Vec3::new(side * 6., 0.9, 17.),
            Vec3::new(side * 5.5, 0.9, 19.),
            Vec3::new(side * 0.6, 0.7, 17.),
        ]);
        for i in 0..24 {
            let a = i as f32 * std::f32::consts::TAU / 24.;
            let b = (i + 1) as f32 * std::f32::consts::TAU / 24.;
            polys.push(vec![
                Vec3::new(side * 2.25 + a.cos() * 0.85, -0.6 + a.sin() * 0.85, 0.),
                Vec3::new(side * 2.25 + b.cos() * 0.85, -0.6 + b.sin() * 0.85, 0.),
                Vec3::new(side * 2.25 + b.cos() * 0.7, -0.6 + b.sin() * 0.7, 12.),
                Vec3::new(side * 2.25 + a.cos() * 0.7, -0.6 + a.sin() * 0.7, 12.),
            ]);
        }
    }
    polys.push(vec![
        Vec3::new(0., 1., 10.),
        Vec3::new(0., 6.5, 15.5),
        Vec3::new(0., 6.5, 18.),
        Vec3::new(0., 1., 18.),
    ]);
    mesh(polys)
}
pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    commands.init_resource::<Speech>();
    let skin = materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.43, 0.46),
        metallic: 0.35,
        perceptual_roughness: 0.62,
        cull_mode: None,
        ..default()
    });
    let dark = materials.add(StandardMaterial {
        base_color: Color::srgb(0.035, 0.055, 0.07),
        metallic: 0.3,
        perceptual_roughness: 0.2,
        ..default()
    });
    let mut image = Image::new_fill(
        bevy::render::render_resource::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        &[255, 255, 255, 0],
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    if let Some(data) = &mut image.data {
        for y in 0..64 {
            for x in 0..64 {
                let u = x as f32 / 31.5 - 1.;
                let v = y as f32 / 31.5 - 1.;
                let edge = (1. - u * u - v * v).max(0.);
                let noise =
                    0.7 + 0.12 * (u * 19. + (v * 11.).sin()).sin() + 0.1 * (v * 23. + u * 7.).cos();
                let i = (y * 64 + x) * 4;
                data[i + 3] = (edge * edge * noise * 210.).clamp(0., 255.) as u8;
            }
        }
    }
    let texture = images.add(image);
    let smoke = materials.add(StandardMaterial {
        base_color_texture: Some(texture),
        cull_mode: None,
        base_color: Color::srgba(0.12, 0.115, 0.11, 0.62),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 1.,
        ..default()
    });
    let hot = materials.add(StandardMaterial {
        base_color: Color::srgb(1., 0.45, 0.1),
        emissive: LinearRgba::new(15., 3., 0.3, 1.),
        unlit: true,
        ..default()
    });
    let fighter = meshes.add(crate::aircraft::visual::hull_mesh(
        crate::aircraft::Aircraft::Su27,
        false,
    ));
    let radome = meshes.add(crate::aircraft::visual::hull_mesh(
        crate::aircraft::Aircraft::Su27,
        true,
    ));
    let mut wings = vec![];
    for side in [-1., 1.] {
        wings.push(crate::aircraft::visual::wing(crate::aircraft::Aircraft::Su27, side).to_vec());
        wings.push(
            crate::aircraft::visual::fin(crate::aircraft::Aircraft::Su27, side)
                .0
                .to_vec(),
        );
        wings.push(vec![
            Vec3::new(side, 0., 5.),
            Vec3::new(side * 4., 0., 8.),
            Vec3::new(side * 4., 0., 9.),
            Vec3::new(side, 0., 8.),
        ]);
    }
    let surfaces = meshes.add(mesh(wings));
    let bomber = meshes.add(bomber());
    let standard = meshes.add(Capsule3d::new(0.18, 4.2));
    let small = meshes.add(Capsule3d::new(0.13, 2.7));
    let sphere = meshes.add(Sphere::new(1.).mesh().uv(12, 8));
    for index in 0..12 {
        commands
            .spawn((
                NavalVisual { kind: 0, index },
                Transform::default(),
                Visibility::Hidden,
            ))
            .with_children(|p| {
                p.spawn((
                    Mesh3d(fighter.clone()),
                    MeshMaterial3d(skin.clone()),
                    Transform::default(),
                ));
                p.spawn((
                    Mesh3d(radome.clone()),
                    MeshMaterial3d(dark.clone()),
                    Transform::default(),
                ));
                p.spawn((
                    Mesh3d(surfaces.clone()),
                    MeshMaterial3d(skin.clone()),
                    Transform::default(),
                ));
            });
        commands
            .spawn((
                NavalVisual { kind: 1, index },
                Mesh3d(bomber.clone()),
                MeshMaterial3d(skin.clone()),
                Transform::default(),
                Visibility::Hidden,
            ))
            .with_children(|p| {
                p.spawn((
                    Mesh3d(sphere.clone()),
                    MeshMaterial3d(dark.clone()),
                    Transform::from_xyz(0., 1.4, -12.).with_scale(Vec3::new(1.1, 0.5, 2.4)),
                ));
            });
    }
    for index in 0..WEAPON_LIMIT {
        commands.spawn((
            NavalVisual { kind: 2, index },
            Mesh3d(standard.clone()),
            MeshMaterial3d(skin.clone()),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
    for index in 0..32 {
        commands.spawn((
            NavalVisual { kind: 3, index },
            Mesh3d(sphere.clone()),
            MeshMaterial3d(hot.clone()),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
    let cube = meshes.add(Cuboid::new(1., 1., 1.));
    let barrel = meshes.add(Cylinder::new(0.065, 1.8).mesh().resolution(8));
    let pedestal = meshes.add(Cylinder::new(0.9, 1.2).mesh().resolution(16));
    for index in 0..3 {
        commands
            .spawn((
                NavalVisual { kind: 6, index },
                Transform::default(),
                Visibility::Hidden,
            ))
            .with_children(|p| {
                p.spawn((
                    Mesh3d(cube.clone()),
                    MeshMaterial3d(skin.clone()),
                    Transform::from_xyz(0., 0.3, 0.2).with_scale(Vec3::new(1.4, 1.4, 1.9)),
                ));
                p.spawn((
                    Mesh3d(sphere.clone()),
                    MeshMaterial3d(skin.clone()),
                    Transform::from_xyz(0., 1.7, 0.45).with_scale(Vec3::new(0.75, 1.0, 0.8)),
                ));
                for i in 0..6 {
                    let a = i as f32 * std::f32::consts::TAU / 6.;
                    p.spawn((
                        Mesh3d(barrel.clone()),
                        MeshMaterial3d(dark.clone()),
                        Transform::from_xyz(a.cos() * 0.16, 0.1 + a.sin() * 0.16, -1.5)
                            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                    ));
                }
            });
        commands.spawn((
            NavalVisual { kind: 7, index },
            Mesh3d(pedestal.clone()),
            MeshMaterial3d(skin.clone()),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
    let smoke_quad = meshes.add(Rectangle::new(2., 2.));
    for index in 0..72 {
        commands.spawn((
            NavalVisual { kind: 4, index },
            Mesh3d(smoke_quad.clone()),
            MeshMaterial3d(smoke.clone()),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
    for index in 0..24 {
        commands.spawn((
            NavalVisual { kind: 5, index },
            Mesh3d(sphere.clone()),
            MeshMaterial3d(hot.clone()),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
    commands.insert_resource(Models { standard, small });
    commands.spawn((
        NavalText,
        Text::new(""),
        TextFont {
            font_size: 14.,
            ..default()
        },
        TextColor(Color::srgb(0.68, 0.94, 0.91)),
        Node {
            position_type: PositionType::Absolute,
            left: px(28),
            bottom: px(80),
            width: px(590),
            padding: UiRect::all(px(10)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.015, 0.03, 0.045, 0.78)),
        ZIndex(45),
    ));
}
pub fn draw(
    f: Res<Flight>,
    time: Res<Time>,
    mut last_text: Local<f64>,
    c: Res<Combat>,
    models: Res<Models>,
    mut text: Query<(&mut Text, &mut Node), With<NavalText>>,
    mut visuals: Query<(
        &NavalVisual,
        &mut Transform,
        &mut Visibility,
        Option<&mut Mesh3d>,
    )>,
    mut gizmos: Gizmos,
    camera: Query<&GlobalTransform, With<crate::FlightCamera>>,
) {
    let naval = c.naval.as_ref();
    if time.elapsed_secs_f64() - *last_text >= 0.1 || *last_text == 0. {
        *last_text = time.elapsed_secs_f64();
        for (mut text, mut node) in &mut text {
            node.display = if naval.is_some() {
                Display::Flex
            } else {
                Display::None
            };
            let Some(n) = naval else {
                continue;
            };
            text.0 = format!(
                "CARRIER DEFENSE / {} / {:.0}s / RAID {} / DEFEATED {}\nPLAYER {} / FLEET {} / INTERCEPTS {}\n\\ RADIO / 7 SHIP VIEW / TAB CONTACT\n",
                ["EASY", "NORMAL", "HARD"][n.difficulty],
                n.survival,
                n.waves,
                n.defeated,
                n.player_kills,
                n.ship_kills,
                n.interceptions
            );
            for (i, s) in n.ships.iter().enumerate() {
                text.0.push_str(&format!(
                    "{} / HULL {:.0}% / SAM {} / CIWS {}{}\n",
                    ["CARRIER", "DDG 125", "DDG 128"][i],
                    s.hull * 100.,
                    s.standard + s.essm,
                    s.gun,
                    if s.sunk { " / LOST" } else { "" }
                ));
            }
            if n.radio_open {
                text.0
                    .push_str("RADIO: 6 RADAR SWEEP / 1 SERVICE / 2 RESPAWN / 3 END / ESC CLOSE\n");
            }
            if f.crashed || f.ejection.active {
                text.0
                    .push_str("AIRCRAFT LOST / \\ THEN 2 RESPAWN OR 3 END RUN\n");
            }
            if n.clock < n.report_until {
                text.0.push_str(&n.report);
            }
        }
    }
    for (tag, mut transform, mut visible, mesh) in &mut visuals {
        let mut pose = None;
        if let Some(n) = naval {
            match tag.kind {
                0 | 1 => {
                    if let Some(r) = n.raiders.iter().find(|r| {
                        r.slot == tag.index && u8::from(r.role == Role::Bomber) == tag.kind
                    }) {
                        if let Some(t) = c.targets.get(r.slot).filter(|t| t.alive) {
                            pose = Some((t.position, t.orientation, Vec3::ONE));
                        }
                    }
                }
                2 => {
                    if let Some(m) = n.rounds.get(tag.index).filter(|m| m.alive) {
                        let rot =
                            DQuat::from_rotation_arc(DVec3::Y, m.velocity.normalize_or(DVec3::Y));
                        pose = Some((m.position, rot, Vec3::ONE));
                        if let Some(mut mesh) = mesh {
                            let desired = if m.kind == Kind::Standard {
                                models.standard.clone()
                            } else {
                                models.small.clone()
                            };
                            mesh.set_if_neq(Mesh3d(desired));
                        }
                        let p = crate::world::local(m.position, &f);
                        let v = (crate::world::frame(&f).conjugate()
                            * m.velocity.normalize_or(DVec3::Y))
                        .as_vec3();
                        if m.age < 14. {
                            gizmos.line(p, p - v * 14., Color::srgb(1., 0.65, 0.2));
                        }
                        gizmos.line(p - v * 15., p - v * 50., Color::srgba(0.8, 0.82, 0.84, 0.5));
                    }
                }
                3 => {
                    if let Some(x) = n.flashes.get(tag.index) {
                        pose = Some((
                            x.position,
                            DQuat::IDENTITY,
                            Vec3::splat((3. + x.age * 12.) as f32 * (1. - x.age / 3.) as f32)
                                * if x.water { 1.5 } else { 1. },
                        ));
                    }
                }
                4 | 5 => {
                    let id = if tag.kind == 4 {
                        tag.index / 3
                    } else {
                        tag.index
                    };
                    let ship = id / 8;
                    let j = id % 8;
                    let comp = &n.ships[ship].compartments[j];
                    if comp.fire > 0.05 {
                        let half = if ship == 0 { 170. } else { 80. };
                        let local = DVec3::new(
                            if j % 2 == 0 { -6. } else { 6. },
                            if ship == 0 { 24. } else { 9. },
                            -half + (j / 2) as f64 * half * 0.5 + half * 0.25,
                        );
                        let phase = (n.clock * 0.2 + (tag.index % 3) as f64 / 3.).fract();
                        let offset = if tag.kind == 4 {
                            DVec3::new(phase * 9., phase * 32., 0.)
                        } else {
                            DVec3::Y * (n.clock * 9. + id as f64).sin().abs() * 2.
                        };
                        pose = Some((
                            carrier::ship_world(&f, ship, local + offset, f.elapsed),
                            DQuat::IDENTITY,
                            Vec3::splat(if tag.kind == 4 {
                                (3. + phase * 8.) as f32
                            } else {
                                (1. + comp.fire * 3.) as f32
                            }),
                        ));
                    }
                }
                6 | 7 => {
                    let ship = tag.index;
                    let s = &n.ships[ship];
                    if !s.sunk || s.sink < 30. {
                        let at = super::weapons::gun_mount(ship)
                            - if tag.kind == 7 {
                                DVec3::Y * 0.9
                            } else {
                                DVec3::ZERO
                            };
                        let q = carrier::ship_rotation(&f, ship, f.elapsed)
                            * if tag.kind == 6 {
                                DQuat::from_rotation_y(-s.gun_yaw)
                                    * DQuat::from_rotation_x(s.gun_pitch)
                            } else {
                                DQuat::IDENTITY
                            };
                        pose = Some((carrier::ship_world(&f, ship, at, f.elapsed), q, Vec3::ONE));
                    }
                }
                _ => {}
            }
        }
        let desired = if pose.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        visible.set_if_neq(desired);
        if let Some((p, q, s)) = pose {
            transform.translation = crate::world::local(p, &f);
            transform.rotation = if tag.kind == 4 {
                camera
                    .iter()
                    .next()
                    .map_or(Quat::IDENTITY, |t| t.rotation())
            } else {
                crate::world::local_rot(q, &f)
            };
            transform.scale = s;
        }
    }
    if let Some(n) = naval {
        for b in n.bursts.iter().filter(|b| b.alive) {
            let a = crate::world::local(b.position, &f);
            let d =
                (crate::world::frame(&f).conjugate() * b.velocity.normalize_or(DVec3::Y)).as_vec3();
            gizmos.line(a, a - d * 30., Color::srgb(1., 0.85, 0.3));
        }
    }
}
