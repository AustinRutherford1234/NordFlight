//! Gripen C inspired, recessed wheel wells and sequenced conformal doors.
//! Station dimensions and link geometry are fitted to this game's airframe.
use crate::{collision, physics::Flight, world::lower_skin};
use bevy::{
    asset::RenderAssetUsages, mesh::Indices, prelude::*, render::render_resource::PrimitiveTopology,
};

#[derive(Clone, Copy)]
pub struct Bay {
    pub xmin: f32,
    pub xmax: f32,
    pub front: f32,
    pub split: f32,
    pub rear: f32,
    pub roof: f32,
}
pub const BAYS: [Bay; 3] = [
    Bay {
        xmin: -0.45,
        xmax: 0.45,
        front: -5.01,
        split: -3.78,
        rear: -3.11,
        roof: 0.16,
    },
    Bay {
        xmin: -0.80,
        xmax: -0.04,
        front: -0.49,
        split: 1.00,
        rear: 1.71,
        roof: 0.10,
    },
    Bay {
        xmin: 0.04,
        xmax: 0.80,
        front: -0.49,
        split: 1.00,
        rear: 1.71,
        roof: 0.10,
    },
];
#[derive(Component, Clone, Copy)]
pub struct Door {
    pub bay: usize,
    pub side: f32,
    pub small: bool,
}
#[derive(Component)]
pub struct Leg {
    index: usize,
}
#[derive(Component)]
pub struct WheelCarrier {
    index: usize,
}
#[derive(Component)]
pub struct Slider {
    index: usize,
    kind: u8,
}
#[derive(Component)]
pub struct Actuator {
    door: Door,
    rod: bool,
}
#[derive(Component)]
pub struct Mechanism {
    index: usize,
    part: u8,
}

fn bounds(d: Door) -> (f32, f32, f32, f32) {
    let b = BAYS[d.bay];
    let (x0, x1) = if d.bay == 0 && !d.small {
        if d.side < 0.0 {
            (b.xmin, 0.0)
        } else {
            (0.0, b.xmax)
        }
    } else {
        (b.xmin, b.xmax)
    };
    (
        x0,
        x1,
        if d.small { b.split + 0.008 } else { b.front },
        if d.small {
            b.rear - 0.008
        } else {
            b.split - 0.008
        },
    )
}
fn hinge(d: Door) -> Vec3 {
    let (x0, x1, z0, z1) = bounds(d);
    let (x, z) = if d.bay == 0 && d.small {
        (0.0, z1)
    } else {
        (if d.side < 0.0 { x0 } else { x1 }, (z0 + z1) * 0.5)
    };
    let p = Vec3::new(x, lower_skin(x, z), z);
    // Offset gooseneck hinges swing the curved edge away from the shell.
    // A hinge on the skin chord drives the raised rear corners into the body.
    if d.bay != 0 {
        p + Vec3::new(d.side * 0.065, -0.035, 0.0)
    } else {
        p
    }
}
pub fn panel_pose(f: &Flight, d: Door) -> Transform {
    let angle = if d.small {
        f.gear_extension as f32
    } else {
        f.gear_door as f32
    };
    let q = if d.bay == 0 && d.small {
        Quat::from_rotation_x(-angle * 132_f32.to_radians())
    } else {
        Quat::from_rotation_z(
            d.side
                * angle
                * if d.bay == 0 {
                    88_f32.to_radians()
                } else if d.small {
                    132_f32.to_radians()
                } else {
                    112_f32.to_radians()
                },
        )
    };
    Transform::from_translation(hinge(d)).with_rotation(q)
}
/// Steering diminishes with airspeed; the same yaw input already drives ground handling.
pub fn nose_steering(f: &Flight) -> f32 {
    (f.controls.y * 30_f64.to_radians() / (1.0 + (f.airspeed / 18.0).powi(2))) as f32
}
pub fn wheel_rotation(f: &Flight, index: usize) -> Quat {
    let fold = (1.0 - f.gear_extension) as f32;
    let flat = ((fold - 0.20) / 0.80).clamp(0.0, 1.0) * std::f32::consts::FRAC_PI_2;
    Quat::from_rotation_y(if index == 0 {
        nose_steering(f) * f.gear_extension as f32
    } else {
        0.0
    }) * Quat::from_rotation_z(if index == 1 { -flat } else { flat })
}
fn segment(a: Vec3, b: Vec3, r: f32) -> Transform {
    let d = b - a;
    Transform::from_translation((a + b) * 0.5)
        .with_rotation(Quat::from_rotation_arc(Vec3::Y, d.normalize_or_zero()))
        .with_scale(Vec3::new(r, d.length(), r))
}
fn attach(
    c: &mut Commands,
    parent: Entity,
    m: Handle<Mesh>,
    mat: Handle<StandardMaterial>,
    t: Transform,
) -> Entity {
    let e = c.spawn((Mesh3d(m), MeshMaterial3d(mat), t)).id();
    c.entity(parent).add_child(e);
    e
}
fn mesh(points: Vec<Vec3>, indices: Vec<u32>) -> Mesh {
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    let n = points.len();
    m.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        points.iter().map(|p| p.to_array()).collect::<Vec<_>>(),
    );
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; n]);
    m.insert_indices(Indices::U32(indices));
    m.compute_smooth_normals();
    m
}
fn skin(d: Door, inset: f32) -> Mesh {
    let (x0, x1, z0, z1) = bounds(d);
    let h = hinge(d);
    let mut p = Vec::new();
    let mut idx = Vec::new();
    for z in 0..=24 {
        for x in 0..=12 {
            let xx = x0 + (x1 - x0) * x as f32 / 12.0;
            let zz = z0 + (z1 - z0) * z as f32 / 24.0;
            p.push(Vec3::new(xx, lower_skin(xx, zz) + inset, zz) - h);
        }
    }
    for z in 0..24 {
        for x in 0..12 {
            let a = z * 13 + x;
            idx.extend([a, a + 1, a + 13, a + 1, a + 14, a + 13]);
        }
    }
    mesh(p, idx)
}
fn well(b: Bay) -> Mesh {
    let ceiling = |x: f32, z: f32| b.roof.min(crate::world::upper_skin(x, z) - 0.025);
    let mut p = Vec::new();
    let mut idx = Vec::new();
    // Walls run from the cut skin into the airframe. No outside housing or skirt.
    for axis in 0..4 {
        for i in 0..24 {
            let t = i as f32 / 24.0;
            let u = (i + 1) as f32 / 24.0;
            let point = |v: f32| match axis {
                0 => (b.xmin, b.front + (b.rear - b.front) * v),
                1 => (b.xmax, b.front + (b.rear - b.front) * v),
                2 => (b.xmin + (b.xmax - b.xmin) * v, b.front),
                _ => (b.xmin + (b.xmax - b.xmin) * v, b.rear),
            };
            let (x, z) = point(t);
            let (xx, zz) = point(u);
            let a = p.len() as u32;
            p.extend([
                Vec3::new(x, lower_skin(x, z) + 0.014, z),
                Vec3::new(xx, lower_skin(xx, zz) + 0.014, zz),
                Vec3::new(xx, ceiling(xx, zz), zz),
                Vec3::new(x, ceiling(x, z), z),
            ]);
            idx.extend([a, a + 1, a + 2, a, a + 2, a + 3]);
        }
    }
    for i in 0..12 {
        for j in 0..24 {
            let x = b.xmin + (b.xmax - b.xmin) * i as f32 / 12.0;
            let xx = b.xmin + (b.xmax - b.xmin) * (i + 1) as f32 / 12.0;
            let z = b.front + (b.rear - b.front) * j as f32 / 24.0;
            let zz = b.front + (b.rear - b.front) * (j + 1) as f32 / 24.0;
            let a = p.len() as u32;
            p.extend([
                Vec3::new(x, ceiling(x, z), z),
                Vec3::new(xx, ceiling(xx, z), z),
                Vec3::new(xx, ceiling(xx, zz), zz),
                Vec3::new(x, ceiling(x, zz), zz),
            ]);
            idx.extend([a, a + 1, a + 2, a, a + 2, a + 3]);
        }
    }
    mesh(p, idx)
}
fn door_lug(d: Door) -> Vec3 {
    let (x0, x1, z0, z1) = bounds(d);
    let x = if d.side < 0.0 { x0 + 0.11 } else { x1 - 0.11 };
    let z = z0 + (z1 - z0) * 0.58;
    Vec3::new(x, lower_skin(x, z) + 0.055, z) - hinge(d)
}
fn door_anchor(d: Door) -> Vec3 {
    let (x0, x1, z0, z1) = bounds(d);
    Vec3::new(
        if d.side < 0.0 { x0 + 0.025 } else { x1 - 0.025 },
        BAYS[d.bay].roof - 0.08,
        z0 + (z1 - z0) * 0.58,
    )
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
    let inside = mats.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.27, 0.24),
        metallic: 0.25,
        perceptual_roughness: 0.78,
        cull_mode: None,
        ..default()
    });
    let metal = mats.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.35, 0.38),
        metallic: 0.75,
        perceptual_roughness: 0.48,
        ..default()
    });
    let chrome = mats.add(StandardMaterial {
        base_color: Color::srgb(0.57, 0.61, 0.65),
        metallic: 0.92,
        perceptual_roughness: 0.30,
        ..default()
    });
    let dark = mats.add(StandardMaterial {
        base_color: Color::srgb(0.028, 0.035, 0.04),
        perceptual_roughness: 0.95,
        ..default()
    });
    let cube = meshes.add(Cuboid::default());
    let cyl = meshes.add(Cylinder::new(1.0, 1.0).mesh().resolution(24));
    let sphere = meshes.add(Sphere::new(1.0).mesh().uv(12, 8));
    let tire = meshes.add(
        Torus::new(0.19, 0.33)
            .mesh()
            .major_resolution(48)
            .minor_resolution(20),
    );
    let tread = meshes.add(
        Torus::new(0.316, 0.323)
            .mesh()
            .major_resolution(48)
            .minor_resolution(8),
    );
    let rim = meshes.add(Cylinder::new(0.19, 0.09).mesh().resolution(32));
    let hub = meshes.add(Cylinder::new(0.065, 0.115).mesh().resolution(24));
    for (index, b) in BAYS.iter().enumerate() {
        attach(
            c,
            root,
            meshes.add(well(*b)),
            inside.clone(),
            Transform::default(),
        );
        let cx = (b.xmin + b.xmax) * 0.5;
        for n in 0..6 {
            let z = b.front + (b.rear - b.front) * (n as f32 + 0.5) / 6.0;
            attach(
                c,
                root,
                cube.clone(),
                metal.clone(),
                Transform::from_xyz(cx, b.roof - 0.035, z).with_scale(Vec3::new(
                    (b.xmax - b.xmin) * 0.72,
                    0.04,
                    0.025,
                )),
            );
        }
        for x in [b.xmin + 0.06, b.xmax - 0.06] {
            attach(
                c,
                root,
                cyl.clone(),
                chrome.clone(),
                segment(
                    Vec3::new(x, b.roof - 0.065, b.front + 0.1),
                    Vec3::new(x, b.roof - 0.065, b.rear - 0.1),
                    0.008,
                ),
            );
        }
        let sides: &[f32] = if index == 0 {
            &[-1.0, 1.0]
        } else if index == 1 {
            &[-1.0]
        } else {
            &[1.0]
        };
        let mut panels: Vec<Door> = sides
            .iter()
            .map(|&side| Door {
                bay: index,
                side,
                small: false,
            })
            .collect();
        panels.push(Door {
            bay: index,
            side: if index == 1 { -1.0 } else { 1.0 },
            small: true,
        });
        for d in panels {
            let e = c
                .spawn((d, Transform::default(), Visibility::Inherited))
                .id();
            c.entity(root).add_child(e);
            attach(
                c,
                e,
                meshes.add(skin(d, -0.003)),
                shell.clone(),
                Transform::default(),
            );
            attach(
                c,
                e,
                meshes.add(skin(d, 0.014)),
                inside.clone(),
                Transform::default(),
            );
            let (x0, x1, z0, z1) = bounds(d);
            let h = hinge(d);
            // Curved stiffeners follow the skin instead of sticking through it.
            for row in 1..4 {
                let z = z0 + (z1 - z0) * row as f32 / 4.0;
                for n in 0..8 {
                    let x = x0 + (x1 - x0) * n as f32 / 8.0;
                    let xx = x0 + (x1 - x0) * (n + 1) as f32 / 8.0;
                    attach(
                        c,
                        e,
                        cyl.clone(),
                        metal.clone(),
                        segment(
                            Vec3::new(x, lower_skin(x, z) + 0.03, z) - h,
                            Vec3::new(xx, lower_skin(xx, z) + 0.03, z) - h,
                            0.012,
                        ),
                    );
                }
            }
            for t in [0.18, 0.82] {
                let z = z0 + (z1 - z0) * t;
                let x = if d.side < 0.0 { x0 } else { x1 };
                let p = if d.bay == 0 && d.small {
                    h
                } else {
                    Vec3::new(h.x, h.y, z)
                };
                if d.bay != 0 {
                    let skin_anchor = Vec3::new(x, lower_skin(x, z) + 0.015, z);
                    attach(
                        c,
                        root,
                        cyl.clone(),
                        metal.clone(),
                        segment(skin_anchor, p, 0.018),
                    );
                }
                attach(
                    c,
                    root,
                    cube.clone(),
                    metal.clone(),
                    Transform::from_translation(p + Vec3::Y * 0.018)
                        .with_scale(Vec3::new(0.06, 0.045, 0.10)),
                );
                let axis = if d.bay == 0 && d.small {
                    Vec3::X
                } else {
                    Vec3::Z
                };
                attach(
                    c,
                    e,
                    cyl.clone(),
                    chrome.clone(),
                    segment(p - h - axis * 0.045, p - h + axis * 0.045, 0.018),
                );
            }
            if !d.small {
                attach(
                    c,
                    e,
                    cube.clone(),
                    metal.clone(),
                    Transform::from_translation(door_lug(d))
                        .with_scale(Vec3::new(0.045, 0.07, 0.06)),
                );
                attach(
                    c,
                    root,
                    cube.clone(),
                    metal.clone(),
                    Transform::from_translation(door_anchor(d))
                        .with_scale(Vec3::new(0.05, 0.06, 0.07)),
                );
                for rod in [false, true] {
                    let a = attach(
                        c,
                        root,
                        cyl.clone(),
                        if rod { chrome.clone() } else { metal.clone() },
                        Transform::default(),
                    );
                    c.entity(a).insert(Actuator { door: d, rod });
                }
            }
            if d.bay == 0 && d.small {
                let p = Vec3::new(0.0, lower_skin(0.0, -3.38) - 0.018, -3.38) - h;
                let q = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
                attach(
                    c,
                    e,
                    meshes.add(Cylinder::new(0.056, 0.05)),
                    metal.clone(),
                    Transform::from_translation(p).with_rotation(q),
                );
                let lens = mats.add(StandardMaterial {
                    base_color: Color::srgb(0.82, 0.86, 0.72),
                    emissive: LinearRgba::new(0.35, 0.37, 0.30, 1.0),
                    perceptual_roughness: 0.25,
                    ..default()
                });
                attach(
                    c,
                    e,
                    meshes.add(Cylinder::new(0.041, 0.008)),
                    lens,
                    Transform::from_translation(p - Vec3::Z * 0.027).with_rotation(q),
                );
            }
        }
        let mount = collision::gear_mount(index).as_vec3();
        for side in [-1.0, 1.0] {
            attach(
                c,
                root,
                cube.clone(),
                metal.clone(),
                Transform::from_translation(mount + Vec3::new(side * 0.095, 0.12, 0.0))
                    .with_scale(Vec3::new(0.035, 0.28, 0.14)),
            );
        }
        let leg = c
            .spawn((Leg { index }, Transform::default(), Visibility::Inherited))
            .id();
        c.entity(root).add_child(leg);
        attach(
            c,
            leg,
            cyl.clone(),
            shell.clone(),
            segment(Vec3::ZERO, Vec3::new(0.0, -0.67, 0.0), 0.061),
        );
        attach(
            c,
            leg,
            cyl.clone(),
            chrome.clone(),
            segment(Vec3::new(-0.13, 0.0, 0.0), Vec3::new(0.13, 0.0, 0.0), 0.045),
        );
        for y in [-0.05, -0.60, -0.67] {
            attach(
                c,
                leg,
                cyl.clone(),
                metal.clone(),
                Transform::from_xyz(0.0, y, 0.0).with_scale(Vec3::new(0.079, 0.035, 0.079)),
            );
        }
        for kind in 0..4 {
            let e = attach(
                c,
                leg,
                cyl.clone(),
                if kind == 0 {
                    chrome.clone()
                } else if kind == 3 {
                    dark.clone()
                } else {
                    metal.clone()
                },
                Transform::default(),
            );
            c.entity(e).insert(Slider { index, kind });
        }
        let wheel = c
            .spawn((
                WheelCarrier { index },
                Transform::default(),
                Visibility::Inherited,
            ))
            .id();
        c.entity(leg).add_child(wheel);
        attach(
            c,
            wheel,
            cyl.clone(),
            metal.clone(),
            segment(
                Vec3::new(if index == 0 { -0.21 } else { -0.16 }, 0.0, 0.0),
                Vec3::new(if index == 0 { 0.21 } else { 0.16 }, 0.0, 0.0),
                0.04,
            ),
        );
        for side in [-1.0, 1.0] {
            attach(
                c,
                wheel,
                cube.clone(),
                shell.clone(),
                Transform::from_xyz(side * 0.085, 0.11, 0.0)
                    .with_scale(Vec3::new(0.035, 0.22, 0.085)),
            );
        }
        for &x in if index == 0 {
            &[-0.14, 0.14][..]
        } else {
            &[0.0][..]
        } {
            let q = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
            for (m, mat) in [
                (tire.clone(), dark.clone()),
                (rim.clone(), chrome.clone()),
                (hub.clone(), metal.clone()),
            ] {
                attach(
                    c,
                    wheel,
                    m,
                    mat,
                    Transform::from_xyz(x, 0.0, 0.0).with_rotation(q),
                );
            }
            for offset in [-0.035, 0.0, 0.035] {
                attach(
                    c,
                    wheel,
                    tread.clone(),
                    dark.clone(),
                    Transform::from_xyz(x + offset, 0.0, 0.0).with_rotation(q),
                );
            }
            for face in [-1.0, 1.0] {
                for bolt in 0..8 {
                    let a = bolt as f32 * std::f32::consts::TAU / 8.0;
                    attach(
                        c,
                        wheel,
                        cyl.clone(),
                        metal.clone(),
                        Transform::from_xyz(x + face * 0.053, 0.12 * a.cos(), 0.12 * a.sin())
                            .with_rotation(q)
                            .with_scale(Vec3::new(0.011, 0.018, 0.011)),
                    );
                }
            }
            if index != 0 {
                attach(
                    c,
                    wheel,
                    cube.clone(),
                    metal.clone(),
                    Transform::from_xyz(x + 0.095, 0.08, 0.13)
                        .with_scale(Vec3::new(0.06, 0.14, 0.08)),
                );
            }
        }
        // Hydraulic retraction jack and articulated drag/breaker brace.
        for part in 0..if index == 0 { 9 } else { 5 } {
            let e = attach(
                c,
                root,
                if part == 4 {
                    sphere.clone()
                } else {
                    cyl.clone()
                },
                if part == 1 {
                    chrome.clone()
                } else {
                    metal.clone()
                },
                Transform::default(),
            );
            c.entity(e).insert(Mechanism { index, part });
        }
    }
}
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct VisualState {
    extension: u64,
    door: u64,
    compression: [u64; 3],
    health: [u64; 3],
    steering: u32,
}
fn visual_state(f: &Flight) -> VisualState {
    VisualState {
        extension: f.gear_extension.to_bits(),
        door: f.gear_door.to_bits(),
        compression: f.gear_compression.map(f64::to_bits),
        health: f.gear_health.map(f64::to_bits),
        steering: nose_steering(f).to_bits(),
    }
}
pub fn update(
    f: Res<Flight>,
    mut previous: Local<Option<VisualState>>,
    mut q: Query<
        (
            &mut Transform,
            Option<&Door>,
            Option<&Leg>,
            Option<&WheelCarrier>,
            Option<&Slider>,
            Option<&Actuator>,
            Option<&Mechanism>,
            Option<&mut Visibility>,
        ),
        Or<(
            With<Door>,
            With<Leg>,
            With<WheelCarrier>,
            With<Slider>,
            With<Actuator>,
            With<Mechanism>,
        )>,
    >,
) {
    if q.is_empty() {
        return;
    }
    let current = visual_state(&f);
    if *previous == Some(current) {
        return;
    }
    *previous = Some(current);
    for (mut t, door, leg, wheel, slider, act, mechanism, vis) in &mut q {
        if let Some(d) = door {
            *t = panel_pose(&f, *d);
        } else if let Some(l) = leg {
            let (m, _, r) = collision::gear_pose(&f, l.index, true);
            t.translation = m.as_vec3();
            t.rotation = r.as_quat();
            if let Some(mut v) = vis {
                *v = if f.gear_health[l.index] > 0.0 {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }
        } else if let Some(w) = wheel {
            t.translation = -Vec3::Y * collision::gear_length(&f, w.index, true) as f32;
            t.rotation = collision::gear_pose(&f, w.index, true)
                .2
                .as_quat()
                .conjugate()
                * wheel_rotation(&f, w.index);
        } else if let Some(s) = slider {
            let y = -collision::gear_length(&f, s.index, true) as f32;
            let knee = Vec3::new(0.0, (y - 0.55) * 0.5, 0.19);
            *t = match s.kind {
                0 => segment(
                    Vec3::new(0.0, -0.64, 0.0),
                    Vec3::new(0.0, y + 0.12, 0.0),
                    0.035,
                ),
                1 => segment(Vec3::new(0.0, -0.55, 0.07), knee, 0.018),
                2 => segment(knee, Vec3::new(0.0, y + 0.13, 0.05), 0.018),
                _ => segment(
                    Vec3::new(0.055, -0.15, 0.0),
                    Vec3::new(0.065, y + 0.12, 0.07),
                    0.009,
                ),
            };
        } else if let Some(a) = act {
            let pose = panel_pose(&f, a.door);
            let from = door_anchor(a.door);
            let to = pose.translation + pose.rotation * door_lug(a.door);
            let mid = from.lerp(to, 0.60);
            *t = if a.rod {
                segment(mid, to, 0.010)
            } else {
                segment(from, mid, 0.018)
            };
        } else if let Some(m) = mechanism {
            let (mount, _, r) = collision::gear_pose(&f, m.index, true);
            let mount = mount.as_vec3();
            let r = r.as_quat();
            let a = mount + Vec3::new(0.0, 0.16, 0.38);
            let b = mount + r * Vec3::new(0.0, -0.63, 0.05);
            let mid = a.lerp(b, 0.57);
            let anchor = mount + Vec3::new(0.0, 0.16, -0.35);
            let length = collision::gear_length(&f, m.index, true) as f32;
            let end = mount + r * Vec3::new(0.0, -0.61, 0.04);
            let span = end - anchor;
            let lean = if m.index == 0 {
                0.0
            } else {
                (if m.index == 1 { -1.0_f32 } else { 1.0 }) * (1.0_f32 / 1.15).atan()
            };
            let deployed = Quat::from_rotation_z(lean) * Vec3::new(0.0, -0.61, 0.04)
                - Vec3::new(0.0, 0.16, -0.35);
            let link = deployed.length() * 0.5 + 0.002;
            let bend = (link.powi(2) - span.length_squared() * 0.25)
                .max(0.0)
                .sqrt();
            let knee = (anchor + end) * 0.5 + span.cross(Vec3::X).normalize_or_zero() * bend;
            *t = match m.part {
                0 => segment(a, mid, 0.029),
                1 => segment(mid, b, 0.016),
                2 => segment(anchor, knee, 0.025),
                3 => segment(knee, end, 0.025),
                4 => Transform::from_translation(knee).with_scale(Vec3::splat(0.041)),
                _ => {
                    let side = if m.part < 7 { -1.0 } else { 1.0 };
                    let a = mount + r * Vec3::new(side * 0.09, -0.63, 0.0);
                    let b = mount - r * Vec3::Y * length
                        + wheel_rotation(&f, 0) * Vec3::new(side * 0.14, 0.18, 0.07);
                    let mid = a.lerp(b, 0.60);
                    if m.part % 2 == 1 {
                        segment(a, mid, 0.018)
                    } else {
                        segment(mid, b, 0.010)
                    }
                }
            };
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn visual_cache_observes_compression_failures_doors_and_steering() {
        let mut f = Flight::runway();
        let initial = visual_state(&f);
        f.gear_compression[1] = 0.2;
        assert!(visual_state(&f) != initial);
        f = Flight::runway();
        f.gear_health[2] = 0.;
        assert!(visual_state(&f) != initial);
        f = Flight::runway();
        f.gear_door = 0.5;
        assert!(visual_state(&f) != initial);
        f = Flight::runway();
        f.controls.y = 1.;
        assert!(visual_state(&f) != initial);
    }
    fn panels() -> Vec<Door> {
        (0..3)
            .flat_map(|bay| {
                let sides = if bay == 0 {
                    vec![-1.0, 1.0]
                } else {
                    vec![if bay == 1 { -1.0 } else { 1.0 }]
                };
                let mut d: Vec<_> = sides
                    .into_iter()
                    .map(|side| Door {
                        bay,
                        side,
                        small: false,
                    })
                    .collect();
                d.push(Door {
                    bay,
                    side: if bay == 1 { -1.0 } else { 1.0 },
                    small: true,
                });
                d
            })
            .collect()
    }
    #[test]
    fn closed_skins_match_airframe_surface() {
        for d in panels() {
            let (x0, x1, z0, z1) = bounds(d);
            for i in 0..=10 {
                let x = x0 + (x1 - x0) * i as f32 / 10.0;
                let z = z0 + (z1 - z0) * i as f32 / 10.0;
                let p = Vec3::new(x, lower_skin(x, z), z) - hinge(d);
                let f = Flight::runway();
                let pose = panel_pose(
                    &Flight {
                        gear_extension: 0.0,
                        ..f
                    },
                    d,
                );
                assert!((pose.transform_point(p).y - lower_skin(x, z)).abs() < 1e-6);
            }
        }
    }
    #[test]
    fn tires_flatten_inside_fuselage_when_stowed() {
        let mut f = Flight::approach();
        f.gear_extension = 0.0;
        for (index, bay) in BAYS.iter().enumerate() {
            let w = collision::gear_pose(&f, index, false).1.as_vec3();
            assert!((wheel_rotation(&f, index) * Vec3::X).dot(Vec3::Y).abs() > 0.99999);
            for tire in if index == 0 {
                &[-0.14, 0.14][..]
            } else {
                &[0.0][..]
            } {
                for a in 0..48 {
                    for b in 0..16 {
                        let a = a as f32 * std::f32::consts::TAU / 48.0;
                        let b = b as f32 * std::f32::consts::TAU / 16.0;
                        let radius = 0.26 + 0.07 * b.cos();
                        let p = w + wheel_rotation(&f, index)
                            * Vec3::new(*tire + 0.07 * b.sin(), radius * a.cos(), radius * a.sin());
                        assert!(
                            p.x >= bay.xmin - 0.015 && p.x <= bay.xmax + 0.015,
                            "{index}: {p:?}"
                        );
                        assert!(p.z > bay.front && p.z < bay.rear);
                        assert!(
                            p.y < bay.roof && p.y > lower_skin(p.x, p.z) - 0.015,
                            "{index}: {p:?}"
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn doors_clear_ground_with_fully_compressed_suspension() {
        let mut f = Flight::runway();
        f.gear_compression = [collision::GEAR_TRAVEL; 3];
        for opening in 0..=100 {
            f.gear_door = opening as f64 / 100.0;
            let ground = (0..3)
                .map(|i| collision::gear_pose(&f, i, true).1.y as f32 - 0.33)
                .fold(f32::INFINITY, f32::min);
            for d in panels() {
                let pose = panel_pose(&f, d);
                let (x0, x1, z0, z1) = bounds(d);
                for i in 0..=20 {
                    for j in 0..=20 {
                        let x = x0 + (x1 - x0) * i as f32 / 20.0;
                        let z = z0 + (z1 - z0) * j as f32 / 20.0;
                        let p = pose.transform_point(Vec3::new(x, lower_skin(x, z), z) - hinge(d));
                        assert!(
                            p.y > ground + 0.05,
                            "door bay {} small {} clearance {}",
                            d.bay,
                            d.small,
                            p.y - ground
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn wheel_covers_close_ahead_of_deployed_strut_exits() {
        let f = Flight::runway();
        for index in 0..3 {
            let (m, w, _) = collision::gear_pose(&f, index, false);
            let b = BAYS[index];
            assert!(if index == 0 {
                m.z as f32 > b.split + 0.10
            } else {
                m.z as f32 > b.split + 0.10
            });
            assert_eq!(m.z, w.z);
        }
    }
    #[test]
    fn nose_steering_responds_to_input_and_reduces_at_speed() {
        let mut f = Flight::runway();
        f.controls.y = 1.0;
        f.airspeed = 0.0;
        let slow = nose_steering(&f);
        f.airspeed = 80.0;
        assert!(nose_steering(&f) < slow * 0.1);
    }
    #[test]
    fn open_main_doors_clear_the_fuselage_envelope() {
        let mut f = Flight::runway();
        f.gear_door = 1.0;
        for d in panels().into_iter().filter(|d| d.bay != 0) {
            let pose = panel_pose(&f, d);
            let (x0, x1, z0, z1) = bounds(d);
            for x in 0..=30 {
                for z in 0..=30 {
                    let xx = x0 + (x1 - x0) * x as f32 / 30.0;
                    let zz = z0 + (z1 - z0) * z as f32 / 30.0;
                    let p = pose
                        .transform_point(Vec3::new(xx, lower_skin(xx, zz) - 0.003, zz) - hinge(d));
                    assert!(
                        p.y <= lower_skin(p.x, p.z) + 0.004
                            || p.y >= crate::world::upper_skin(p.x, p.z) - 0.004,
                        "bay {} small {} inside body {p:?}",
                        d.bay,
                        d.small
                    );
                }
            }
        }
    }
    #[test]
    fn door_surfaces_do_not_intersect_tires_during_cycle() {
        let mut f = Flight::runway();
        let mut margin = [f32::INFINITY; 3];
        let mut location = [String::new(), String::new(), String::new()];
        for compressed in [false, true] {
            for step in 0..=122 {
                f.gear_extension = if step > 111 {
                    1.0
                } else if step > 100 {
                    0.0
                } else {
                    step as f64 / 100.0
                };
                f.gear_door = if step > 111 {
                    (step - 112) as f64 / 10.0
                } else if step > 100 {
                    (step - 101) as f64 / 10.0
                } else if step == 0 || step == 100 {
                    0.0
                } else {
                    1.0
                };
                f.gear_compression = if compressed && f.gear_extension == 1.0 {
                    [collision::GEAR_TRAVEL; 3]
                } else {
                    [0.0; 3]
                };
                for d in panels() {
                    let pose = panel_pose(&f, d);
                    let (x0, x1, z0, z1) = bounds(d);
                    for inset in [-0.003, 0.014, 0.030] {
                        for i in 0..=20 {
                            for j in 0..=if inset == 0.030 { 3 } else { 30 } {
                                let x = x0 + (x1 - x0) * i as f32 / 20.0;
                                if inset == 0.030 && j == 0 {
                                    continue;
                                }
                                let z = z0
                                    + (z1 - z0) * j as f32
                                        / if inset == 0.030 { 4.0 } else { 30.0 };
                                let p = pose.transform_point(
                                    Vec3::new(x, lower_skin(x, z) + inset, z) - hinge(d),
                                );
                                for index in 0..3 {
                                    let centre = collision::gear_pose(&f, index, true).1.as_vec3();
                                    let local =
                                        wheel_rotation(&f, index).conjugate() * (p - centre);
                                    for &tire in if index == 0 {
                                        &[-0.14, 0.14][..]
                                    } else {
                                        &[0.0][..]
                                    } {
                                        let radius = Vec2::new(local.y, local.z).length();
                                        let distance = (radius - 0.26).hypot(local.x - tire);
                                        let gap =
                                            distance - if inset == 0.030 { 0.084 } else { 0.072 };
                                        if gap < margin[d.bay] {
                                            margin[d.bay] = gap;
                                            location[d.bay] = format!(
                                                "small={} leg={index} extension={} inset={inset} clearance={}",
                                                d.small,
                                                f.gear_extension,
                                                distance - 0.07
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        assert!(
            margin.iter().all(|m| *m > 0.0),
            "margins {margin:?}, locations {location:?}"
        );
    }
}
