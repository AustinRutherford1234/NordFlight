//! Batched airframe surface detail and electrically controlled aircraft lights.
use crate::{
    collision,
    physics::{Flight, Vehicle},
    world::FUSELAGE_SECTIONS,
};
use bevy::{
    asset::RenderAssetUsages, mesh::Indices, prelude::*, render::render_resource::PrimitiveTopology,
};
#[derive(Clone, Copy)]
enum Kind {
    Navigation,
    Formation,
    Beacon,
    Strobe,
    Landing(usize),
}
#[derive(Component)]
pub struct Lamp {
    kind: Kind,
    material: Handle<StandardMaterial>,
    color: LinearRgba,
}
#[derive(Component)]
pub struct Beam;
#[derive(Component)]
pub struct Halo;
#[derive(Component)]
pub struct Spill(Kind);
fn section(z: f32) -> (f32, f32, f32) {
    let s = FUSELAGE_SECTIONS.windows(2).find(|s| z <= s[1].0).unwrap();
    let t = (z - s[0].0) / (s[1].0 - s[0].0);
    (
        s[0].1 + (s[1].1 - s[0].1) * t,
        s[0].2 + (s[1].2 - s[0].2) * t,
        s[0].3 + (s[1].3 - s[0].3) * t,
    )
}
pub(crate) fn surface(z: f32, a: f32) -> Vec3 {
    let (w, h, y) = section(z);
    Vec3::new(w * a.cos() * 1.003, y + h * a.sin() * 1.003, z)
}
fn mesh(p: Vec<Vec3>, idx: Vec<u32>) -> Mesh {
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    let n = p.len();
    m.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        p.into_iter().map(|v| v.to_array()).collect::<Vec<_>>(),
    );
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; n]);
    m.insert_indices(Indices::U32(idx));
    m.compute_smooth_normals();
    m
}
fn quad(p: &mut Vec<Vec3>, idx: &mut Vec<u32>, a: [Vec3; 4]) {
    let n = p.len() as u32;
    p.extend(a);
    idx.extend([n, n + 1, n + 2, n, n + 2, n + 3]);
}
fn fit_patch(z0: f32, z1: f32, a: f32, half: f32) -> Mesh {
    let mut p = vec![];
    let mut i = vec![];
    for n in 0..16 {
        let z = z0 + (z1 - z0) * n as f32 / 16.0;
        let zz = z0 + (z1 - z0) * (n + 1) as f32 / 16.0;
        quad(
            &mut p,
            &mut i,
            [
                surface(z, a - half),
                surface(z, a + half),
                surface(zz, a + half),
                surface(zz, a - half),
            ],
        );
    }
    mesh(p, i)
}
fn visible_skin(p: Vec3) -> bool {
    if p.z >= -4.08 && p.z <= -1.35 && p.x.abs() < 0.57 && p.y > 0.18 {
        return false;
    }
    for b in crate::gear::BAYS {
        if p.x > b.xmin - 0.006
            && p.x < b.xmax + 0.006
            && p.z > b.front - 0.01
            && p.z < b.rear + 0.01
            && p.y < b.roof
        {
            return false;
        }
    }
    true
}
fn seams() -> Mesh {
    let mut p = vec![];
    let mut i = vec![];
    for z in [-6.15, -4.0, -1.3, 0.8, 2.8, 4.6, 5.45] {
        for n in 0..96 {
            let a = n as f32 * std::f32::consts::TAU / 96.0;
            let b = (n + 1) as f32 * std::f32::consts::TAU / 96.0;
            let q = [
                surface(z - 0.002, a),
                surface(z + 0.002, a),
                surface(z + 0.002, b),
                surface(z - 0.002, b),
            ];
            if q.iter().all(|v| visible_skin(*v)) {
                quad(&mut p, &mut i, q);
            }
        }
    }
    for a in [0.35, 1.12, 2.02, 2.80, 3.52, 4.22, 5.18, 5.94] {
        for n in 0..70 {
            let z = -4.0 + n as f32 * 9.4 / 70.0;
            let zz = -4.0 + (n + 1) as f32 * 9.4 / 70.0;
            let q = [
                surface(z, a - 0.003),
                surface(z, a + 0.003),
                surface(zz, a + 0.003),
                surface(zz, a - 0.003),
            ];
            if q.iter().all(|v| visible_skin(*v)) {
                quad(&mut p, &mut i, q);
            }
        }
    }
    mesh(p, i)
}
fn wing_point(side: f32, x: f32, u: f32) -> Vec3 {
    let leading = -1.55 + (x - 0.65) * 3.75 / 3.5;
    let trailing = (2.32 + (x - 0.65) * 0.13).min(2.20 + (4.15 - x) * 0.55 / 0.19);
    let z = leading + (trailing - leading) * u;
    Vec3::new(
        side * x,
        crate::world::wing_surface_y(x, z, true) + 0.004,
        z,
    )
}
fn wing_detail() -> Mesh {
    let mut p = vec![];
    let mut idx = vec![];
    for side in [-1., 1.] {
        for x in [1.05, 1.65, 2.45, 3.35] {
            for i in 0..24 {
                let u = 0.03 + i as f32 * 0.94 / 24.;
                let v = u + 0.94 / 24.;
                quad(
                    &mut p,
                    &mut idx,
                    [
                        wing_point(side, x - 0.003, u),
                        wing_point(side, x + 0.003, u),
                        wing_point(side, x + 0.003, v),
                        wing_point(side, x - 0.003, v),
                    ],
                );
            }
        }
        for u in [0.08, 0.48, 0.88] {
            for i in 0..40 {
                let x = 0.88 + i as f32 * 3.05 / 40.;
                let xx = x + 3.05 / 40.;
                quad(
                    &mut p,
                    &mut idx,
                    [
                        wing_point(side, x, u - 0.0015),
                        wing_point(side, x, u + 0.0015),
                        wing_point(side, xx, u + 0.0015),
                        wing_point(side, xx, u - 0.0015),
                    ],
                );
            }
        }
        // Segmented non-slip root walkway, kept on the fixed wing.
        for i in 0..14 {
            let u = 0.25 + i as f32 * 0.045;
            quad(
                &mut p,
                &mut idx,
                [
                    wing_point(side, 0.94, u),
                    wing_point(side, 1.02, u),
                    wing_point(side, 1.02, u + 0.024),
                    wing_point(side, 0.94, u + 0.024),
                ],
            );
        }
    }
    mesh(p, idx)
}
fn fasteners() -> Mesh {
    let mut p = vec![];
    let mut idx = vec![];
    for z in [-6.15, -4.0, -1.3, 0.8, 2.8, 4.6, 5.35] {
        for n in 0..56 {
            let a = n as f32 * std::f32::consts::TAU / 56.;
            let center = surface(z, a);
            if !visible_skin(center) {
                continue;
            }
            let tangent = Vec3::new(-a.sin(), a.cos(), 0.);
            for j in 0..6 {
                let aa = j as f32 * std::f32::consts::TAU / 6.;
                let bb = (j + 1) as f32 * std::f32::consts::TAU / 6.;
                let base = p.len() as u32;
                p.extend([
                    center,
                    center + (tangent * aa.cos() + Vec3::Z * aa.sin()) * 0.009,
                    center + (tangent * bb.cos() + Vec3::Z * bb.sin()) * 0.009,
                ]);
                idx.extend([base, base + 1, base + 2]);
            }
        }
    }
    for side in [-1., 1.] {
        for x in [1.05, 1.65, 2.45, 3.35] {
            for i in 0..15 {
                let u = 0.05 + i as f32 * 0.90 / 14.;
                let center = wing_point(side, x, u) + Vec3::Y * 0.001;
                quad(
                    &mut p,
                    &mut idx,
                    [
                        center + Vec3::new(-0.009, 0., -0.009),
                        center + Vec3::new(0.009, 0., -0.009),
                        center + Vec3::new(0.009, 0., 0.009),
                        center + Vec3::new(-0.009, 0., 0.009),
                    ],
                );
            }
        }
    }
    mesh(p, idx)
}
fn inlet(side: f32) -> Mesh {
    let mut p = vec![];
    let mut idx = vec![];
    let stations = [
        (-2.30, 0.65, 1.16, -0.46, 0.20),
        (-2.15, 0.67, 1.17, -0.46, 0.20),
        (-0.55, 0.68, 1.05, -0.43, 0.15),
        (0.15, 0.72, 0.92, -0.28, 0.10),
    ];
    for (z, inner, outer, lo, hi) in stations {
        let r = 0.055;
        for (x, y) in [
            (inner + r, lo),
            (outer - r, lo),
            (outer, lo + r),
            (outer, hi - r),
            (outer - r, hi),
            (inner + r, hi),
            (inner, hi - r),
            (inner, lo + r),
        ] {
            p.push(Vec3::new(side * x, y, z));
        }
    }
    for j in 0..3 {
        for k in 0..8 {
            let a = j * 8 + k;
            let b = j * 8 + (k + 1) % 8;
            idx.extend([a, b, a + 8, b, b + 8, a + 8]);
        }
    }
    mesh(p, idx)
}
fn roundels() -> Mesh {
    let mut p = Vec::new();
    let mut idx = Vec::new();
    let mut colors = Vec::new();
    for side in [-1.0, 1.0] {
        let point = |x: f32, z: f32| {
            Vec3::new(
                side * (2.5 + x),
                crate::world::wing_surface_y(2.5 + x, 1.8 + z, true) + 0.002,
                1.8 + z,
            )
        };
        let start = p.len() as u32;
        p.push(point(0.0, 0.0));
        colors.push([0.10, 0.24, 0.43, 1.0]);
        for n in 0..=48 {
            let a = n as f32 * std::f32::consts::TAU / 48.0;
            p.push(point(0.27 * a.cos(), 0.27 * a.sin()));
            colors.push([0.10, 0.24, 0.43, 1.0]);
        }
        for n in 0..48 {
            idx.extend([start, start + n + 1, start + n + 2]);
        }
        for (x, z) in [(-0.083, -0.068), (0.083, -0.068), (0.0, 0.098)] {
            let mut add = |v: &[(f32, f32)], tri: &[u32]| {
                let base = p.len() as u32;
                for (xx, zz) in v {
                    p.push(point(x + xx, z + zz) + Vec3::Y * 0.001);
                    colors.push([0.73, 0.61, 0.22, 1.0]);
                }
                idx.extend(tri.iter().map(|i| i + base));
            };
            add(
                &[
                    (-0.045, 0.004),
                    (0.045, 0.004),
                    (0.042, 0.026),
                    (-0.042, 0.026),
                ],
                &[0, 1, 2, 0, 2, 3],
            );
            for (left, right, tip) in [
                (-0.045, -0.014, -0.044),
                (-0.020, 0.020, 0.0),
                (0.014, 0.045, 0.044),
            ] {
                add(&[(left, 0.004), (right, 0.004), (tip, -0.049)], &[0, 1, 2]);
            }
        }
    }
    let mut m = mesh(p, idx);
    m.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    m
}
fn attach(
    c: &mut Commands,
    root: Entity,
    m: Handle<Mesh>,
    mat: Handle<StandardMaterial>,
    t: Transform,
) -> Entity {
    let e = c.spawn((Mesh3d(m), MeshMaterial3d(mat), t)).id();
    c.entity(root).add_child(e);
    e
}
fn corona() -> Mesh {
    let mut p = vec![];
    let mut idx = vec![];
    let mut colors = vec![];
    for i in 0..32 {
        let a = i as f32 * std::f32::consts::TAU / 32.;
        let b = (i + 1) as f32 * std::f32::consts::TAU / 32.;
        let start = p.len() as u32;
        p.extend([
            Vec3::ZERO,
            Vec3::new(a.cos() * 0.12, a.sin() * 0.12, 0.),
            Vec3::new(b.cos() * 0.12, b.sin() * 0.12, 0.),
        ]);
        idx.extend([start, start + 1, start + 2]);
        colors.extend([[1., 1., 1., 0.7], [1., 1., 1., 0.], [1., 1., 1., 0.]]);
    }
    let mut m = mesh(p, idx);
    m.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    m
}
fn lamp(
    c: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    root: Entity,
    p: Vec3,
    color: Color,
    kind: Kind,
) -> Entity {
    let material = mats.add(StandardMaterial {
        base_color: color,
        emissive: color.to_linear() * 3.0,
        perceptual_roughness: 0.22,
        unlit: true,
        ..default()
    });
    let e = attach(
        c,
        root,
        meshes.add(Sphere::new(0.038).mesh().uv(12, 8)),
        material.clone(),
        Transform::from_translation(p),
    );
    c.entity(e).insert((
        Lamp {
            kind,
            material,
            color: color.to_linear(),
        },
        bevy::light::NotShadowCaster,
    ));
    // A compact coloured lens, dark mounting collar, and soft additive corona.
    let collar = mats.add(StandardMaterial {
        base_color: Color::srgb(0.07, 0.09, 0.10),
        metallic: 0.7,
        perceptual_roughness: 0.26,
        ..default()
    });
    attach(
        c,
        e,
        meshes.add(Sphere::new(0.043).mesh().uv(12, 8)),
        collar,
        Transform::from_xyz(0., -0.018, 0.).with_scale(Vec3::new(0.95, 0.65, 0.85)),
    );
    let glow = mats.add(StandardMaterial {
        base_color: color.with_alpha(0.12),
        emissive: color.to_linear() * 20.,
        alpha_mode: AlphaMode::Add,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    let halo = attach(
        c,
        e,
        meshes.add(corona()),
        glow.clone(),
        Transform::default(),
    );
    c.entity(halo).insert((
        Halo,
        Lamp {
            kind,
            material: glow,
            color: color.to_linear(),
        },
        bevy::light::NotShadowCaster,
    ));
    let spill = c
        .spawn((
            PointLight {
                color,
                intensity: 0.,
                range: if matches!(kind, Kind::Strobe) {
                    12.
                } else {
                    6.
                },
                radius: 0.05,
                shadows_enabled: false,
                ..default()
            },
            Transform::default(),
            Spill(kind),
        ))
        .id();
    c.entity(e).add_child(spill);
    e
}
pub fn setup(
    c: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    root: Entity,
) {
    let shell = mats.add(StandardMaterial {
        base_color: Color::srgb(0.40, 0.47, 0.51),
        metallic: 0.28,
        perceptual_roughness: 0.63,
        cull_mode: None,
        ..default()
    });
    let dark = mats.add(StandardMaterial {
        base_color: Color::srgb(0.035, 0.046, 0.053),
        perceptual_roughness: 0.85,
        cull_mode: None,
        ..default()
    });
    let seam = mats.add(StandardMaterial {
        base_color: Color::srgb(0.16, 0.20, 0.23),
        perceptual_roughness: 0.82,
        cull_mode: None,
        ..default()
    });
    attach(
        c,
        root,
        meshes.add(seams()),
        seam.clone(),
        Transform::default(),
    );
    attach(
        c,
        root,
        meshes.add(wing_detail()),
        seam.clone(),
        Transform::default(),
    );
    let hardware = mats.add(StandardMaterial {
        base_color: Color::srgb(0.32, 0.35, 0.36),
        metallic: 0.72,
        perceptual_roughness: 0.36,
        cull_mode: None,
        ..default()
    });
    attach(
        c,
        root,
        meshes.add(fasteners()),
        hardware,
        Transform::default(),
    );
    crate::skin_damage::setup(c, meshes, mats, root, None);
    let paint = mats.add(StandardMaterial {
        base_color: Color::WHITE,
        cull_mode: None,
        perceptual_roughness: 0.90,
        ..default()
    });
    attach(c, root, meshes.add(roundels()), paint, Transform::default());
    for side in [-1.0, 1.0] {
        attach(
            c,
            root,
            meshes.add(inlet(side)),
            shell.clone(),
            Transform::default(),
        );
        attach(
            c,
            root,
            meshes.add(Cuboid::new(0.41, 0.52, 0.035)),
            dark.clone(),
            Transform::from_xyz(side * 0.91, -0.12, -1.93),
        );
        attach(
            c,
            root,
            meshes.add(Cuboid::new(0.025, 0.47, 0.68)),
            shell.clone(),
            Transform::from_xyz(side * 0.67, -0.08, -2.05),
        );
        // Canard bearing/fairing joins the moving foreplane to the fuselage.
        attach(
            c,
            root,
            meshes.add(Cylinder::new(0.075, 0.32)),
            shell.clone(),
            Transform::from_xyz(side * 0.65, 0.30, -2.70)
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
        );
        for z in [-0.60, -0.30, 3.15, 3.45, 3.75, 4.05] {
            let a = if side > 0.0 {
                0.20
            } else {
                std::f32::consts::PI - 0.20
            };
            attach(
                c,
                root,
                meshes.add(fit_patch(z, z + 0.12, a, 0.035)),
                dark.clone(),
                Transform::default(),
            );
        }
        for z in [-1.15, 0.2, 2.0] {
            let a = if side > 0.0 {
                0.68
            } else {
                std::f32::consts::PI - 0.68
            };
            attach(
                c,
                root,
                meshes.add(fit_patch(z, z + 0.20, a, 0.060)),
                seam.clone(),
                Transform::default(),
            );
        }
        for z in [1.70, 2.70, 3.30] {
            attach(
                c,
                root,
                meshes.add(Cuboid::new(0.15, 0.15, 0.035)),
                shell.clone(),
                Transform::from_xyz(side * 4.10, -0.08, z),
            );
        }
        lamp(
            c,
            meshes,
            mats,
            root,
            Vec3::new(side * 4.14, 0.016, 2.05),
            if side < 0.0 {
                Color::srgb(1.0, 0.04, 0.02)
            } else {
                Color::srgb(0.04, 1.0, 0.10)
            },
            Kind::Navigation,
        );
        lamp(
            c,
            meshes,
            mats,
            root,
            Vec3::new(side * 4.14, 0.016, 2.88),
            Color::WHITE,
            Kind::Strobe,
        );
        let a = if side > 0.0 {
            0.13
        } else {
            std::f32::consts::PI - 0.13
        };
        let color = Color::srgb(0.46, 0.73, 0.46);
        let mat = mats.add(StandardMaterial {
            base_color: color,
            emissive: color.to_linear(),
            cull_mode: None,
            ..default()
        });
        let e = attach(
            c,
            root,
            meshes.add(fit_patch(1.8, 2.8, a, 0.018)),
            mat.clone(),
            Transform::default(),
        );
        c.entity(e).insert(Lamp {
            kind: Kind::Formation,
            material: mat,
            color: color.to_linear(),
        });
    }
    // Structural blades and pitot collar are rooted in the skin, not free-floating.
    for (z, height) in [(0.2, 0.23), (3.0, 0.19)] {
        let y = crate::world::upper_skin(0.0, z);
        let p = vec![
            Vec3::new(-0.018, y, z),
            Vec3::new(0.018, y, z),
            Vec3::new(-0.018, y + height, z + 0.10),
            Vec3::new(0.018, y + height, z + 0.10),
            Vec3::new(-0.018, y, z + 0.23),
            Vec3::new(0.018, y, z + 0.23),
        ];
        attach(
            c,
            root,
            meshes.add(mesh(
                p,
                vec![
                    0, 2, 4, 1, 5, 3, 0, 1, 3, 0, 3, 2, 2, 3, 5, 2, 5, 4, 4, 5, 1, 4, 1, 0,
                ],
            )),
            shell.clone(),
            Transform::default(),
        );
    }
    attach(
        c,
        root,
        meshes.add(Cylinder::new(0.034, 0.12)),
        shell.clone(),
        Transform::from_xyz(0.0, 0.02, -7.50)
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
    );
    for (p, kind, color) in [
        (
            Vec3::new(0.0, crate::world::upper_skin(0., 1.2), 1.2),
            Kind::Beacon,
            Color::srgb(1.0, 0.02, 0.01),
        ),
        (
            Vec3::new(0.0, crate::world::lower_skin(0., 2.4), 2.4),
            Kind::Beacon,
            Color::srgb(1.0, 0.02, 0.01),
        ),
        (Vec3::new(0.0, 2.97, 4.70), Kind::Navigation, Color::WHITE),
    ] {
        attach(
            c,
            root,
            meshes.add(Cuboid::new(0.09, 0.09, 0.09)),
            shell.clone(),
            Transform::from_translation(p),
        );
        lamp(
            c,
            meshes,
            mats,
            root,
            p + Vec3::Y * if p.y < 0.0 { -0.055 } else { 0.055 },
            color,
            kind,
        );
    }
    for index in 1..3 {
        let assembly = c.spawn((Transform::default(), Visibility::Inherited)).id();
        c.entity(root).add_child(assembly);
        attach(
            c,
            assembly,
            meshes.add(Cylinder::new(0.070, 0.09)),
            dark.clone(),
            Transform::default().with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        );
        let color = Color::srgb(1.0, 0.96, 0.82);
        let mat = mats.add(StandardMaterial {
            base_color: color,
            emissive: color.to_linear() * 5.0,
            ..default()
        });
        attach(
            c,
            assembly,
            meshes.add(Cylinder::new(0.054, 0.01)),
            mat.clone(),
            Transform::from_xyz(0.0, 0.0, -0.05)
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        );
        c.entity(assembly).insert(Lamp {
            kind: Kind::Landing(index),
            material: mat,
            color: color.to_linear(),
        });
        let beam = c
            .spawn((
                SpotLight {
                    color: Color::srgb(1.0, 0.96, 0.88),
                    intensity: 200000.0,
                    range: 100.0,
                    radius: 0.04,
                    inner_angle: 0.14,
                    outer_angle: 0.36,
                    shadows_enabled: false,
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, -0.06).with_rotation(Quat::from_rotation_x(-0.20)),
                Beam,
            ))
            .id();
        c.entity(assembly).add_child(beam);
    }
}
fn power(f: &Flight, kind: Kind, seconds: f32) -> f32 {
    if f.vehicle != Vehicle::Gripen || !f.battery || !f.lights || f.crashed {
        return 0.0;
    }
    match kind {
        Kind::Navigation => 8000.0,
        Kind::Formation => 8.0,
        Kind::Landing(_) => {
            if f.gear_extension > 0.95 {
                2200.0
            } else {
                0.0
            }
        }
        Kind::Beacon => {
            if (f.engine || f.apu) && seconds.rem_euclid(1.2) < 0.16 {
                18000.0
            } else {
                0.0
            }
        }
        Kind::Strobe => {
            let t = seconds.rem_euclid(1.2);
            if t < 0.075 || (0.16..0.235).contains(&t) {
                32000.0
            } else {
                0.0
            }
        }
    }
}
pub fn update(
    f: Res<Flight>,
    time: Res<Time>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut lamps: Query<(&Lamp, &mut Transform, Option<&Halo>, &mut Visibility)>,
    mut beams: Query<&mut SpotLight, With<Beam>>,
    mut spills: Query<(&Spill, &mut PointLight)>,
    cameras: Query<&GlobalTransform, With<crate::FlightCamera>>,
) {
    let facing = cameras
        .iter()
        .next()
        .map(|c| crate::world::local_rot(f.orientation, &f).inverse() * c.rotation());
    for (l, mut t, halo, mut visibility) in &mut lamps {
        let p = power(&f, l.kind, time.elapsed_secs());
        let desired = l.color * p * if halo.is_some() { 0.035 } else { 1. };
        if halo.is_some() {
            if let Some(facing) = facing {
                t.rotation = facing;
            }
            *visibility = if p > 0. {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
        if mats.get(&l.material).is_some_and(|m| m.emissive != desired) {
            let m = mats.get_mut(&l.material).unwrap();
            m.emissive = desired;
            if halo.is_none() {
                m.base_color = Color::LinearRgba(l.color * if p > 0. { 1. } else { 0.045 });
            }
        }
        if let Kind::Landing(index) = l.kind {
            let (mount, _, q) = collision::gear_pose(&f, index, true);
            t.translation = mount.as_vec3() + q.as_quat() * Vec3::new(0.0, -0.57, -0.10);
            t.rotation = q.as_quat();
        }
    }
    for (spill, mut light) in &mut spills {
        let intensity = power(&f, spill.0, time.elapsed_secs()) * 0.25;
        if light.intensity != intensity {
            light.intensity = intensity;
        }
    }
    let on = power(&f, Kind::Landing(1), time.elapsed_secs()) > 0.0;
    for mut light in &mut beams {
        let intensity = if on { 450000.0 } else { 0.0 };
        if light.intensity != intensity {
            light.intensity = intensity;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lighting_respects_electrical_power_and_gear() {
        let mut f = Flight::runway();
        assert!(power(&f, Kind::Landing(1), 0.0) > 0.0);
        f.gear_extension = 0.0;
        assert_eq!(power(&f, Kind::Landing(1), 0.0), 0.0);
        f.battery = false;
        assert_eq!(power(&f, Kind::Navigation, 0.0), 0.0);
        f.battery = true;
        f.lights = false;
        assert_eq!(power(&f, Kind::Navigation, 0.0), 0.0);
    }
    #[test]
    fn strobes_have_two_short_flashes() {
        let f = Flight::runway();
        assert!(power(&f, Kind::Strobe, 0.03) > 0.0 && power(&f, Kind::Strobe, 0.18) > 0.0);
        assert_eq!(power(&f, Kind::Strobe, 0.5), 0.0);
    }
}
