//! Lunar terrain, direct-return vehicle meshes, mission controls and displays.
use crate::{
    ViewState,
    physics::*,
    space::{self, Body, Stage},
    world,
};
use bevy::{
    asset::RenderAssetUsages, math::DVec3, mesh::Indices, prelude::*,
    render::render_resource::PrimitiveTopology,
};
#[derive(Component)]
pub struct Moon;
#[derive(Component)]
pub struct Cabin;
/// Planet-fixed north, rather than the observer's changing local horizon.
pub fn map_axes(f: &Flight, v: &ViewState) -> (Vec3, Vec3, Vec3) {
    let moon = v.map_target == 2 || (v.map_target == 0 && f.space.body == Body::Moon);
    let north = if moon {
        space::moon(f.elapsed).2 * -DVec3::Z
    } else {
        rotation().normalize()
    };
    let north = (world::frame(f).conjugate() * north).as_vec3().normalize();
    let seed = (world::frame(f).conjugate() * DVec3::X).as_vec3();
    let equator = (seed - north * seed.dot(north)).normalize_or(Vec3::Z);
    let east = north.cross(equator).normalize();
    (north, equator, east)
}
pub fn map_focus(f: &Flight, v: &ViewState) -> (DVec3, f64) {
    if v.map_focus_ship {
        return (f.position, f.space.body.radius());
    }
    match v.map_target {
        1 => (DVec3::ZERO, RADIUS),
        2 => (space::moon(f.elapsed).0, space::MOON_RADIUS),
        3 => (DVec3::ZERO, space::MOON_DISTANCE * 0.9),
        _ => (
            space::center(f.space.body, f.elapsed),
            f.space.body.radius(),
        ),
    }
}
#[derive(Component)]
pub struct Surface;
#[derive(Component)]
pub struct StagePart(pub Stage);
#[derive(Component)]
pub struct DebrisRoot(pub Stage);
#[derive(Component)]
pub struct Plume(pub Stage, pub u8);
#[derive(Component)]
pub struct Leg(pub usize);
#[derive(Component)]
pub struct LegLink {
    index: usize,
    base: Vec3,
    length: f32,
    sleeve: Option<u8>,
}
fn sleeve_pose(base: Vec3, foot: Vec3, sleeve: Option<u8>, rest: f32) -> (Vec3, f32) {
    let delta = foot - base;
    if let Some(i) = sleeve {
        let length = rest.min(delta.length());
        let spacing = (delta.length() - length) / 3.;
        (
            base + delta.normalize() * (spacing * i as f32 + length * 0.5),
            length,
        )
    } else {
        ((base + foot) * 0.5, delta.length())
    }
}
#[derive(Component)]
pub struct Parachute;
#[derive(Component)]
pub struct MissionPanel;
#[derive(Component)]
pub struct MissionText;
#[derive(Component)]
pub struct Action(pub u8);
#[derive(Resource)]
pub struct Ground {
    mesh: Handle<Mesh>,
    anchor: DVec3,
    cell: (i64, i64),
}
fn material(m: &mut Assets<StandardMaterial>, c: Color, metal: f32) -> Handle<StandardMaterial> {
    m.add(StandardMaterial {
        base_color: c,
        metallic: metal,
        perceptual_roughness: 0.65,
        ..default()
    })
}
pub fn profile(points: &[(f32, f32)], segments: usize) -> Mesh {
    let mut v = Vec::new();
    let mut uv = Vec::new();
    let mut idx = Vec::new();
    for (j, (z, r)) in points.iter().enumerate() {
        for i in 0..=segments {
            let a = i as f32 * std::f32::consts::TAU / segments as f32;
            v.push([r * a.cos(), r * a.sin(), *z]);
            uv.push([
                i as f32 / segments as f32,
                j as f32 / (points.len() - 1) as f32,
            ]);
        }
    }
    for j in 0..points.len() - 1 {
        for i in 0..segments {
            let a = (j * (segments + 1) + i) as u32;
            let b = a + (segments + 1) as u32;
            idx.extend([a, a + 1, b, a + 1, b + 1, b]);
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, v);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    mesh.insert_indices(Indices::U32(idx));
    mesh.compute_smooth_normals();
    mesh
}
/// Closed pressure/tank geometry. No engine or adapter is allowed to expose sky
/// through a missing bulkhead. Separate cap vertices preserve hard rim normals.
pub fn solid_profile(points: &[(f32, f32)], segments: usize) -> Mesh {
    let mesh = profile(points, segments);
    let bevy::mesh::VertexAttributeValues::Float32x3(p) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
    else {
        unreachable!()
    };
    let mut positions = p.clone();
    let Some(Indices::U32(i)) = mesh.indices() else {
        unreachable!()
    };
    let mut indices = i.clone();
    for (end, (z, r)) in [points[0], *points.last().unwrap()].into_iter().enumerate() {
        let base = positions.len() as u32;
        positions.push([0., 0., z]);
        for j in 0..=segments {
            let a = j as f32 * std::f32::consts::TAU / segments as f32;
            positions.push([r * a.cos(), r * a.sin(), z]);
        }
        for j in 0..segments as u32 {
            if end == 0 {
                indices.extend([base, base + j + 2, base + j + 1]);
            } else {
                indices.extend([base, base + j + 1, base + j + 2]);
            }
        }
    }
    let mut result = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    result.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.]; positions.len()]);
    result.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    result.insert_indices(Indices::U32(indices));
    result.compute_smooth_normals();
    result
}
/// Outer bell folds into an inner wall and ends at a closed recessed throat.
fn nozzle(points: &[(f32, f32)], segments: usize) -> Mesh {
    let mut wall = points.to_vec();
    wall.extend(
        points
            .iter()
            .rev()
            .map(|(z, r)| (*z, (r - 0.035).max(0.015))),
    );
    wall.push((points[0].0, 0.));
    profile(&wall, segments)
}
fn inner_shell(mesh: &Mesh) -> Mesh {
    let mut inner = mesh.clone();
    if let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) =
        inner.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        for p in p {
            let r = (p[0] * p[0] + p[1] * p[1]).sqrt();
            if r > 0.06 {
                p[0] *= (r - 0.045) / r;
                p[1] *= (r - 0.045) / r;
            }
        }
    }
    if let Some(Indices::U32(indices)) = inner.indices_mut() {
        for t in indices.chunks_exact_mut(3) {
            t.swap(1, 2);
        }
    }
    inner.compute_smooth_normals();
    inner
}
fn part(
    c: &mut Commands,
    root: Entity,
    mesh: Handle<Mesh>,
    mat: Handle<StandardMaterial>,
    t: Transform,
    stage: Option<Stage>,
) -> Entity {
    let mut entity = c.spawn((Mesh3d(mesh), MeshMaterial3d(mat), t));
    if let Some(s) = stage {
        entity.insert(StagePart(s));
    }
    let id = entity.id();
    c.entity(root).add_child(id);
    id
}
pub(crate) fn plume_mesh(radius: f32, layer: u8) -> Mesh {
    let r = radius * [0.38, 0.75, 1.][layer as usize];
    let length = [7., 11., 15.][layer as usize];
    let mut mesh = profile(
        &[
            (0., r),
            (0.4, r * 1.05),
            (2., r * 1.25),
            (length * 0.6, r * 1.65),
            (length, r * 2.2),
        ],
        48,
    );
    let bevy::mesh::VertexAttributeValues::Float32x3(positions) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
    else {
        unreachable!()
    };
    let colors: Vec<[f32; 4]> = positions
        .iter()
        .map(|p| {
            let t = (p[2] / length).clamp(0., 1.);
            let a = (1. - t).powi(2) * [0.65, 0.3, 0.12][layer as usize];
            if layer == 0 {
                [1., 0.85, 0.65, a]
            } else {
                [0.25, 0.55, 1., a]
            }
        })
        .collect();
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh
}
pub fn vehicle(
    c: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    root: Entity,
) {
    let white = material(mats, Color::srgb(0.82, 0.84, 0.81), 0.25);
    let silver = material(mats, Color::srgb(0.48, 0.53, 0.58), 0.8);
    let dark = material(mats, Color::srgb(0.075, 0.065, 0.06), 0.2);
    let gold = material(mats, Color::srgb(0.66, 0.40, 0.10), 0.65);
    let mut templates: Vec<(Stage, Handle<Mesh>, Handle<StandardMaterial>, Transform)> = Vec::new();
    // A capsule with a real cut-out ahead of the pilot's eye, framed by the
    // existing pressure-cabin interior. Window glass is transparent in both views.
    let mut capsule = solid_profile(
        &[
            (-4.15, 0.35),
            (-3.8, 0.52),
            (-3.1, 0.85),
            (-2., 1.28),
            (-0.8, 1.72),
            (1., 2.1),
            (1.25, 2.08),
        ],
        96,
    );
    // Remove only the forward upper window sectors, preserving the conical skin.
    let positions = match capsule.attribute(Mesh::ATTRIBUTE_POSITION).unwrap() {
        bevy::mesh::VertexAttributeValues::Float32x3(p) => p.clone(),
        _ => unreachable!(),
    };
    let mut window = capsule.clone();
    let mut window_indices = Vec::new();
    if let Some(Indices::U32(indices)) = capsule.indices_mut() {
        let filtered = indices
            .chunks_exact(3)
            .filter(|tri| {
                let center = (Vec3::from(positions[tri[0] as usize])
                    + Vec3::from(positions[tri[1] as usize])
                    + Vec3::from(positions[tri[2] as usize]))
                    / 3.;
                let glazing =
                    center.z < -1.8 && center.z > -3.9 && center.y > 0.25 && center.x.abs() < 0.85;
                if glazing {
                    window_indices.extend_from_slice(tri);
                }
                !glazing
            })
            .flatten()
            .copied()
            .collect();
        *indices = filtered;
    }
    window.insert_indices(Indices::U32(window_indices));
    part(
        c,
        root,
        meshes.add(inner_shell(&capsule)),
        dark.clone(),
        Transform::default(),
        None,
    );
    part(
        c,
        root,
        meshes.add(crate::spacecraft_detail::window_frame(&window)),
        silver.clone(),
        Transform::default(),
        None,
    );
    part(
        c,
        root,
        meshes.add(capsule),
        white.clone(),
        Transform::default(),
        None,
    );
    part(
        c,
        root,
        meshes.add(solid_profile(
            &[(1.24, 2.08), (1.48, 2.14), (1.64, 1.9), (1.72, 0.01)],
            96,
        )),
        dark.clone(),
        Transform::default(),
        None,
    );
    let glass = mats.add(StandardMaterial {
        base_color: Color::srgba(0.2, 0.4, 0.5, 0.08),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        perceptual_roughness: 0.12,
        ..default()
    });
    part(
        c,
        root,
        meshes.add(window),
        glass,
        Transform::default(),
        None,
    );
    let mut add = |stage: Stage,
                   mesh: Handle<Mesh>,
                   mat: Handle<StandardMaterial>,
                   t: Transform,
                   c: &mut Commands| {
        let id = part(c, root, mesh.clone(), mat.clone(), t, Some(stage));
        templates.push((stage, mesh, mat, t));
        id
    };
    add(
        Stage::Return,
        meshes.add(solid_profile(&[(1.8, 1.9), (4.1, 1.9), (4.25, 1.3)], 64)),
        silver.clone(),
        Transform::default(),
        c,
    );
    add(
        Stage::Return,
        meshes.add(nozzle(
            &[(4.1, 0.28), (4.6, 0.35), (5.5, 0.68), (6.1, 0.95)],
            64,
        )),
        dark.clone(),
        Transform::default(),
        c,
    );
    for a in 0..8 {
        let theta = a as f32 * std::f32::consts::TAU / 8.;
        add(
            Stage::Return,
            meshes.add(Cuboid::new(0.08, 0.11, 2.1)),
            white.clone(),
            Transform::from_xyz(theta.cos() * 1.92, theta.sin() * 1.92, 2.95)
                .with_rotation(Quat::from_rotation_z(theta)),
            c,
        );
    }
    add(
        Stage::Return,
        meshes.add(solid_profile(&[(4.34, 0.29), (4.36, 0.29)], 48)),
        dark.clone(),
        Transform::default(),
        c,
    );
    add(
        Stage::Descent,
        meshes.add(solid_profile(
            &[(4.4, 2.7), (4.7, 3.7), (7.1, 3.7), (7.4, 2.8)],
            8,
        )),
        gold.clone(),
        Transform::default(),
        c,
    );
    for (x, y) in [(-2., -2.), (-2., 2.), (2., -2.), (2., 2.)] {
        // The tank bulkhead sits above this recessed chamber floor, so it
        // cannot appear as a gold sheet cutting through the engine mouth.
        add(
            Stage::Descent,
            meshes.add(solid_profile(&[(7.425, 0.625), (7.445, 0.625)], 48)),
            dark.clone(),
            Transform::from_xyz(x, y, 0.),
            c,
        );
        add(
            Stage::Descent,
            meshes.add(nozzle(
                &[(6.5, 0.22), (6.9, 0.28), (7.4, 0.63), (7.7, 0.72)],
                48,
            )),
            dark.clone(),
            Transform::from_xyz(x, y, 0.),
            c,
        );
    }
    for (i, (x, y)) in [(-6.5, -6.5), (-6.5, 6.5), (6.5, -6.5), (6.5, 6.5)]
        .into_iter()
        .enumerate()
    {
        let foot = Vec3::new(x, y, 8.2);
        let shoulder = Vec3::new(x * 0.395, y * 0.395, 5.1);
        let delta = foot - shoulder;
        for sleeve in 0..4 {
            let length = delta.length() / 4. + 0.08;
            let (center, _) = sleeve_pose(shoulder, foot, Some(sleeve), length);
            let strut = add(
                Stage::Descent,
                meshes.add(crate::spacecraft_detail::strut(
                    length,
                    0.13 - sleeve as f32 * 0.019,
                )),
                silver.clone(),
                Transform::from_translation(center)
                    .with_rotation(Quat::from_rotation_arc(Vec3::Y, delta.normalize())),
                c,
            );
            c.entity(strut).insert(LegLink {
                index: i,
                base: shoulder,
                length,
                sleeve: Some(sleeve),
            });
        }
        let brace = Vec3::new(x * 0.4, y * 0.4, 6.9);
        let d = foot - brace;
        let strut = add(
            Stage::Descent,
            meshes.add(crate::spacecraft_detail::strut(d.length(), 0.065)),
            gold.clone(),
            Transform::from_translation((foot + brace) * 0.5)
                .with_rotation(Quat::from_rotation_arc(Vec3::Y, d.normalize())),
            c,
        );
        c.entity(strut).insert(LegLink {
            index: i,
            base: brace,
            length: d.length(),
            sleeve: None,
        });
        let id = add(
            Stage::Descent,
            meshes.add(crate::spacecraft_detail::footpad()),
            silver.clone(),
            Transform::from_translation(foot)
                .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            c,
        );
        c.entity(id).insert(Leg(i));
    }
    add(
        Stage::Transfer,
        meshes.add(solid_profile(
            &[(8.7, 2.7), (9.2, 3.2), (17.5, 3.2), (18.2, 2.5)],
            96,
        )),
        white.clone(),
        Transform::default(),
        c,
    );
    add(
        Stage::Transfer,
        meshes.add(nozzle(
            &[(17.9, 0.55), (18.5, 0.63), (19.3, 1.15), (20.6, 1.65)],
            64,
        )),
        dark.clone(),
        Transform::default(),
        c,
    );
    for z in [9.4, 11.2, 15.4, 17.1] {
        add(
            Stage::Transfer,
            meshes.add(Torus::new(3.17, 3.24)),
            silver.clone(),
            Transform::from_xyz(0., 0., z)
                .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            c,
        );
    }
    add(
        Stage::Transfer,
        meshes.add(solid_profile(&[(18.32, 0.60), (18.34, 0.60)], 48)),
        dark.clone(),
        Transform::default(),
        c,
    );
    let finishes = [
        material(mats, Color::srgb(0.88, 0.89, 0.86), 0.15),
        material(mats, Color::srgb(0.57, 0.63, 0.68), 0.85),
        material(mats, Color::srgb(0.035, 0.042, 0.052), 0.35),
        material(mats, Color::srgb(0.72, 0.43, 0.10), 0.72),
        material(mats, Color::srgb(0.92, 0.68, 0.26), 0.70),
    ];
    for (stage, finish, mesh) in crate::spacecraft_detail::build() {
        let mesh = meshes.add(mesh);
        if let Some(stage) = stage {
            add(
                stage,
                mesh,
                finishes[finish].clone(),
                Transform::default(),
                c,
            );
        } else {
            part(
                c,
                root,
                mesh,
                finishes[finish].clone(),
                Transform::default(),
                None,
            );
        }
    }
    drop(add);
    for stage in [Stage::Transfer, Stage::Descent, Stage::Return] {
        let debris = c
            .spawn((DebrisRoot(stage), Transform::default(), Visibility::Hidden))
            .id();
        for (s, mesh, mat, t) in &templates {
            if *s == stage {
                part(c, debris, mesh.clone(), mat.clone(), *t, None);
            }
        }
    }
    let flame = mats.add(StandardMaterial {
        base_color: Color::srgba(0.35, 0.62, 1., 0.24),
        emissive: LinearRgba::rgb(1.8, 2.6, 4.),
        unlit: true,
        alpha_mode: AlphaMode::Add,
        cull_mode: None,
        ..default()
    });
    for (stage, x, y, z, r) in [
        (Stage::Transfer, 0., 0., 20.6, 1.65),
        (Stage::Return, 0., 0., 6.1, 0.95),
        (Stage::Descent, -2., -2., 7.7, 0.72),
        (Stage::Descent, -2., 2., 7.7, 0.72),
        (Stage::Descent, 2., -2., 7.7, 0.72),
        (Stage::Descent, 2., 2., 7.7, 0.72),
    ] {
        for layer in 0..3 {
            let id = part(
                c,
                root,
                meshes.add(plume_mesh(r, layer)),
                flame.clone(),
                Transform::from_xyz(x, y, z),
                None,
            );
            c.entity(id).insert(Plume(stage, layer));
        }
    }
    let chute = part(
        c,
        root,
        meshes.add(Sphere::new(14.).mesh().uv(48, 24)),
        material(mats, Color::srgb(0.9, 0.55, 0.2), 0.),
        Transform::from_xyz(0., 0., -45.).with_scale(Vec3::new(1., 1., 0.4)),
        None,
    );
    c.entity(chute).insert(Parachute);
    for (p, force) in [
        (Vec3::new(2.05, 0., 1.0), Vec3::Y),
        (Vec3::new(-2.05, 0., 1.0), -Vec3::Y),
        (Vec3::new(0., 2.05, 1.0), Vec3::X),
        (Vec3::new(0., -2.05, 1.0), -Vec3::X),
        (Vec3::new(1.65, 0., -0.8), Vec3::X),
        (Vec3::new(-1.65, 0., -0.8), -Vec3::X),
        (Vec3::new(0., 1.65, -0.8), Vec3::Y),
        (Vec3::new(0., -1.65, -0.8), -Vec3::Y),
    ] {
        part(
            c,
            root,
            meshes.add(Cuboid::new(0.22, 0.22, 0.32)),
            silver.clone(),
            Transform::from_translation(p),
            None,
        );
        let id = part(
            c,
            root,
            meshes.add(profile(&[(0., 0.055), (0.2, 0.09), (0.75, 0.)], 16)),
            flame.clone(),
            Transform::from_translation(p).with_rotation(Quat::from_rotation_arc(Vec3::Z, -force)),
            None,
        );
        c.entity(id).insert(world::RcsPlume {
            torque: p.as_dvec3().cross(force.as_dvec3()).normalize(),
        });
    }
    let cabin = c
        .spawn((Cabin, Transform::default(), Visibility::Hidden))
        .id();
    c.entity(root).add_child(cabin);
    crate::cockpit::setup(c, meshes, mats, images, cabin, Vehicle::Spacecraft);
}
fn moon_uv(n: DVec3) -> [f32; 2] {
    // The lunar pole is body -Z and the Earth-facing prime meridian is -Y.
    [
        (0.5 + (-n.x).atan2(-n.y) / std::f64::consts::TAU) as f32,
        (0.5 + n.z.clamp(-1., 1.).asin() / std::f64::consts::PI) as f32,
    ]
}
fn surface_mesh(anchor: DVec3) -> Mesh {
    let n = anchor.normalize();
    let x = n.cross(DVec3::Y).normalize_or(DVec3::Z);
    let y = n.cross(x);
    let mut v = Vec::new();
    let mut colors = Vec::new();
    let mut uv = Vec::new();
    let mut idx = Vec::new();
    for (size, hole) in [
        (1024., 0.),
        (8192., 512.),
        (65536., 4096.),
        (524288., 32768.),
    ] {
        let start = v.len() as u32;
        let count = 128;
        for j in 0..=count {
            for i in 0..=count {
                let u = (i as f64 / count as f64 - 0.5) * size;
                let w = (j as f64 / count as f64 - 0.5) * size;
                let dir = (n + (x * u + y * w) / space::MOON_RADIUS).normalize();
                let h = space::height(dir);
                v.push(
                    (dir * (space::MOON_RADIUS + h) - anchor)
                        .as_vec3()
                        .to_array(),
                );
                let p = dir * space::MOON_RADIUS;
                let gray = (0.94
                    + 0.035 * (p.x * 0.11 + p.y * 0.17).sin() * (p.z * 0.13 - p.y * 0.07).sin())
                    as f32;
                colors.push([gray, gray, gray, 1.]);
                uv.push(moon_uv(dir));
            }
        }
        for j in 0..count {
            for i in 0..count {
                let u = ((i as f64 + 0.5) / count as f64 - 0.5) * size;
                let w = ((j as f64 + 0.5) / count as f64 - 0.5) * size;
                if u.abs() < hole && w.abs() < hole {
                    continue;
                }
                let a = start + (j * (count + 1) + i) as u32;
                let b = a + (count + 1) as u32;
                idx.extend([a, a + 1, b, a + 1, b + 1, b]);
            }
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, v.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    // Unwrap across the longitude seam locally so interpolation never spans
    // the entire texture on one terrain triangle.
    let origin_u = moon_uv(n)[0];
    for p in &mut uv {
        if p[0] - origin_u > 0.5 {
            p[0] -= 1.;
        } else if p[0] - origin_u < -0.5 {
            p[0] += 1.;
        }
    }
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    mesh.insert_indices(Indices::U32(idx));
    mesh.compute_smooth_normals();
    // Vertices inside the LOD ring holes have no faces. Give those vertices
    // finite outward normals too, rather than retaining zero/NaN normals.
    if let Some(bevy::mesh::VertexAttributeValues::Float32x3(normals)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_NORMAL)
    {
        for (i, normal) in normals.iter_mut().enumerate() {
            let n = Vec3::from(*normal);
            if !n.is_finite() || n.length_squared() < 0.1 {
                *normal = (anchor + Vec3::from(v[i]).as_dvec3())
                    .normalize()
                    .as_vec3()
                    .to_array();
            }
        }
    }
    mesh
}
fn globe() -> Mesh {
    let mut v = Vec::new();
    let mut normals = Vec::new();
    let mut uv = Vec::new();
    let mut idx = Vec::new();
    const NX: usize = 512;
    const NY: usize = 256;
    for j in 0..=NY {
        let lat = std::f64::consts::PI * (j as f64 / NY as f64 - 0.5);
        for i in 0..=NX {
            let lon = std::f64::consts::TAU * (i as f64 / NX as f64 - 0.5);
            let n = DVec3::new(-lat.cos() * lon.sin(), -lat.cos() * lon.cos(), -lat.sin());
            let h = space::height(n);
            v.push((n * (space::MOON_RADIUS + h)).as_vec3().to_array());
            normals.push(n.as_vec3().to_array());
            uv.push([i as f32 / NX as f32, 1. - j as f32 / NY as f32]);
        }
    }
    for j in 0..NY {
        for i in 0..NX {
            let a = (j * (NX + 1) + i) as u32;
            let b = a + (NX + 1) as u32;
            idx.extend([a, a + 1, b, a + 1, b + 1, b]);
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, v.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    mesh.insert_indices(Indices::U32(idx));
    mesh
}
pub fn setup(
    mut c: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let mut image = Image::from_buffer(
        include_bytes!("../assets/moon.jpg"),
        bevy::image::ImageType::Extension("jpg"),
        bevy::image::CompressedImageFormats::NONE,
        true,
        bevy::image::ImageSampler::default(),
        RenderAssetUsages::default(),
    )
    .unwrap();
    image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::Repeat,
        address_mode_v: bevy::image::ImageAddressMode::ClampToEdge,
        ..bevy::image::ImageSamplerDescriptor::linear()
    });
    let mat = mats.add(StandardMaterial {
        base_color: Color::srgb(0.72, 0.72, 0.72),
        base_color_texture: Some(images.add(image)),
        perceptual_roughness: 1.,
        reflectance: 0.,
        ..default()
    });
    c.spawn((
        Moon,
        Mesh3d(meshes.add(globe())),
        MeshMaterial3d(mat.clone()),
        Transform::default(),
        bevy::light::NotShadowCaster,
    ));
    let mesh = meshes.add(Mesh::from(Cuboid::new(1., 1., 1.)));
    c.spawn((
        Surface,
        bevy::camera::primitives::Aabb::from_min_max(Vec3::splat(-400000.), Vec3::splat(400000.)),
        Mesh3d(mesh.clone()),
        MeshMaterial3d(mat.clone()),
        Transform::default(),
        Visibility::Hidden,
    ));
    c.insert_resource(Ground {
        mesh,
        anchor: DVec3::ZERO,
        cell: (i64::MIN, i64::MIN),
    });
    c.spawn((
        MissionPanel,
        Node {
            position_type: PositionType::Absolute,
            right: px(24),
            bottom: px(24),
            width: px(330),
            padding: UiRect::all(px(12)),
            flex_direction: FlexDirection::Column,
            row_gap: px(7),
            border_radius: BorderRadius::all(px(8)),
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::srgba(0.015, 0.03, 0.055, 0.94)),
        ZIndex(48),
    ))
    .with_children(|p| {
        p.spawn((
            MissionText,
            Text::new("LUNAR MISSION"),
            TextFont {
                font_size: 13.,
                ..default()
            },
            TextColor(Color::srgb(0.76, 0.91, 0.96)),
        ));
        for (id, name) in [
            (0, "EARTH ORBIT / FULL STACK"),
            (1, "MOON ORBIT / LANDER"),
            (2, "LANDING APPROACH"),
            (3, "STAGE / SPACE"),
            (4, "ARM PARACHUTES"),
            (5, "EXTEND / RETRACT LEGS · G"),
        ] {
            p.spawn((
                Action(id),
                Button,
                Node {
                    height: px(28),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.06, 0.15, 0.22)),
            ))
            .with_children(|b| {
                b.spawn((
                    Text::new(name),
                    TextFont {
                        font_size: 12.,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });
        }
        p.spawn((
            Text::new("ATTITUDE TARGET / SAS"),
            TextFont {
                font_size: 12.,
                ..default()
            },
            TextColor(Color::srgb(0.5, 0.8, 0.9)),
        ));
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(4),
            row_gap: px(4),
            ..default()
        })
        .with_children(|grid| {
            for (mode, label) in [
                (0, "HOLD / DAMP"),
                (5, "MANEUVER"),
                (1, "PROGRADE"),
                (2, "RETROGRADE"),
                (3, "RADIAL OUT"),
                (6, "RADIAL IN"),
                (4, "NORMAL + / UP"),
                (7, "NORMAL − / DOWN"),
            ] {
                grid.spawn((
                    Action(10 + mode),
                    Button,
                    Node {
                        width: percent(49.),
                        height: px(26),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.06, 0.15, 0.22)),
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new(label),
                        TextFont {
                            font_size: 11.,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
            }
        });
    });
}
pub fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut f: ResMut<Flight>,
    mut view: ResMut<ViewState>,
    warp: Res<crate::warp::Warp>,
    actions: Query<(&Action, &Interaction), Changed<Interaction>>,
    preview: Option<Res<crate::VisualCheck>>,
) {
    if f.vehicle != Vehicle::Spacecraft || warp.editing || preview.is_some_and(|p| p.0.is_some()) {
        return;
    }
    let mut pressed = None;
    for (a, i) in &actions {
        if *i == Interaction::Pressed {
            pressed = Some(a.0);
        }
    }
    if keys.just_pressed(KeyCode::Space) {
        pressed = Some(3);
    }
    if keys.just_pressed(KeyCode::KeyK) {
        pressed = Some(4);
    }
    match pressed {
        Some(id @ 0..=2) => crate::reset(
            &mut f,
            match id {
                0 => 3,
                1 => 9,
                _ => 10,
            },
            &mut view,
        ),
        Some(3) => {
            space::stage(&mut f);
        }
        Some(4) => {
            f.space.chute_armed = !f.space.chute_armed;
        }
        Some(5) => {
            f.gear = !f.gear;
        }
        Some(id @ 10..=17) => {
            f.sas_mode = id - 10;
            f.assist = true;
            f.status = if f.sas_mode == 5 && space::attitude_direction(&f).is_none() {
                "MANEUVER SAS / CREATE A NODE IN MAP".into()
            } else {
                format!("SAS / {}", space::sas_label(&f))
            };
        }
        _ => {}
    }
}
pub fn visuals(
    f: Res<Flight>,
    v: Res<ViewState>,
    mut ground: ResMut<Ground>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut moon: Query<
        &mut Transform,
        (
            With<Moon>,
            Without<Surface>,
            Without<DebrisRoot>,
            Without<Plume>,
            Without<Leg>,
            Without<LegLink>,
        ),
    >,
    mut surface: Query<
        (&mut Transform, &mut Visibility),
        (
            With<Surface>,
            Without<Moon>,
            Without<DebrisRoot>,
            Without<Plume>,
            Without<Leg>,
            Without<LegLink>,
        ),
    >,
    mut parts: Query<(&StagePart, &mut Visibility), Without<Surface>>,
    mut debris: Query<
        (&DebrisRoot, &mut Transform, &mut Visibility),
        (
            Without<Surface>,
            Without<Moon>,
            Without<StagePart>,
            Without<Plume>,
            Without<Leg>,
            Without<LegLink>,
        ),
    >,
    mut plumes: Query<
        (&Plume, &mut Transform, &mut Visibility),
        (
            Without<Surface>,
            Without<Moon>,
            Without<StagePart>,
            Without<DebrisRoot>,
            Without<Leg>,
            Without<LegLink>,
        ),
    >,
    mut legs: Query<
        (&Leg, &mut Transform),
        (
            Without<Moon>,
            Without<Surface>,
            Without<DebrisRoot>,
            Without<Plume>,
            Without<LegLink>,
        ),
    >,
    mut chutes: Query<
        &mut Visibility,
        (
            With<Parachute>,
            Without<StagePart>,
            Without<Surface>,
            Without<DebrisRoot>,
            Without<Plume>,
        ),
    >,
    mut links: Query<
        (&LegLink, &mut Transform),
        (
            Without<Leg>,
            Without<Moon>,
            Without<Surface>,
            Without<DebrisRoot>,
            Without<Plume>,
        ),
    >,
) {
    let (c, _, q) = space::moon(f.elapsed);
    for mut t in &mut moon {
        t.translation = world::local(c, &f);
        t.rotation = world::local_rot(q, &f);
    }
    let near = f.vehicle == Vehicle::Spacecraft
        && f.space.body == Body::Moon
        && f.altitude() < 50000.
        && !v.map;
    if near {
        let n = (q.conjugate() * (f.position - c)).normalize();
        let cell = (
            (n.z.atan2(n.x) * space::MOON_RADIUS / 256.).floor() as i64,
            (n.y.asin() * space::MOON_RADIUS / 256.).floor() as i64,
        );
        if cell != ground.cell {
            ground.anchor = n * (space::MOON_RADIUS + space::height(n));
            ground.cell = cell;
            if let Some(mesh) = meshes.get_mut(&ground.mesh) {
                *mesh = surface_mesh(ground.anchor);
            }
        }
    }
    for (mut t, mut visibility) in &mut surface {
        *visibility = if near {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if near {
            t.translation = world::local(c + q * ground.anchor, &f);
            t.rotation = world::local_rot(q, &f);
        }
    }
    for (tag, mut visible) in &mut parts {
        let show = match tag.0 {
            Stage::Transfer => f.space.stage == Stage::Transfer,
            Stage::Descent => matches!(f.space.stage, Stage::Transfer | Stage::Descent),
            Stage::Return => f.space.stage != Stage::Capsule,
            Stage::Capsule => true,
        };
        *visible = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (tag, mut t, mut visibility) in &mut debris {
        if let Some(d) = f.space.debris.iter().find(|d| d.stage == tag.0) {
            *visibility = if f.vehicle == Vehicle::Spacecraft {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
            if let Some((p, rot)) = space::debris_pose(d, f.elapsed) {
                t.translation = world::local(p, &f);
                t.rotation = world::local_rot(rot, &f);
            } else {
                *visibility = Visibility::Hidden;
            }
        } else {
            *visibility = Visibility::Hidden;
        }
    }
    for (tag, mut t, mut visibility) in &mut plumes {
        let active = f.vehicle == Vehicle::Spacecraft
            && f.space.stage == tag.0
            && f.engine
            && !f.crashed
            && f.thrust > 1.
            && f.throttle > 0.;
        *visibility = if active {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let motion = crate::plume_motion::sample(f.elapsed, tag.1 as u32 + 400);
        t.rotation = motion.bend;
        t.scale = if active {
            Vec3::new(
                motion.width,
                motion.width,
                (0.25 + f.spool as f32 * 0.75)
                    * (1. + 0.5 * (1. - (f.density / 1.225).clamp(0., 1.)) as f32)
                    * motion.length,
            )
        } else {
            Vec3::ZERO
        };
    }
    for (tag, mut t) in &mut legs {
        t.translation = space::leg_point(&f, tag.0).as_vec3();
    }
    for (link, mut t) in &mut links {
        let foot = space::leg_point(&f, link.index).as_vec3();
        let delta = foot - link.base;
        let (center, length) = sleeve_pose(link.base, foot, link.sleeve, link.length);
        t.translation = center;
        t.rotation = Quat::from_rotation_arc(Vec3::Y, delta.normalize());
        t.scale.y = length / link.length;
    }
    for mut visibility in &mut chutes {
        *visibility = if f.space.chute > 0. {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
pub fn cabin_visibility(
    f: Res<Flight>,
    v: Res<ViewState>,
    mut cabins: Query<&mut Visibility, With<Cabin>>,
    mut moons: Query<&mut Visibility, (With<Moon>, Without<Cabin>)>,
) {
    for mut visibility in &mut cabins {
        *visibility = if f.vehicle == Vehicle::Spacecraft && v.index == 3 && !v.map {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for mut visibility in &mut moons {
        *visibility = if f.vehicle == Vehicle::Spacecraft
            && f.space.body == Body::Moon
            && f.altitude() < 10000.
            && !v.map
        {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}
pub fn ui(
    f: Res<Flight>,
    v: Res<ViewState>,
    mut panels: Query<&mut Node, With<MissionPanel>>,
    mut labels: Query<&mut Text, With<MissionText>>,
    mut buttons: Query<(&Action, &mut BackgroundColor)>,
) {
    let visible = f.vehicle == Vehicle::Spacecraft;
    for mut n in &mut panels {
        n.display = if visible {
            Display::Flex
        } else {
            Display::None
        };
        n.bottom = px(20);
        n.right = px(24);
    }
    if !visible {
        return;
    }
    for (action, mut color) in &mut buttons {
        color.0 = if action.0 >= 10 && action.0 - 10 == f.sas_mode && f.assist {
            Color::srgb(0.08, 0.38, 0.32)
        } else {
            Color::srgb(0.06, 0.15, 0.22)
        };
    }
    let (d, r) = space::future_delta_v(&f);
    let engine = f.space.stage.engine();
    let gravity = f.space.body.mu() / (f.altitude() + f.space.body.radius()).powi(2);
    let vv = f.velocity - space::ground_velocity(f.position, f.elapsed, f.space.body);
    let vertical = vv.dot(f.up());
    let horizontal = (vv - f.up() * vertical).length();
    let decel = (engine.0 / f.mass() - gravity).max(0.01);
    let stop = vertical.min(0.).powi(2) / (2. * decel);
    let message = format!(
        "NORD / DIRECT RETURN\n{} · {}\nMASS {:.1} t / FUEL {:.0} kg\nSTAGE Δv {:.0} m/s · Isp {:.0} s\nRESERVE DESC {:.0} / RETURN {:.0} m/s\nRCS {:.0} kg · SHIELD {:.0}%\nq {:.1} kPa · HEAT {:.0} kW/m²\nSAS {} / {}\nLEGS {}\n{}{}",
        f.space.body.label(),
        f.space.stage.label(),
        f.mass() / 1000.,
        f.fuel,
        space::delta_v(&f),
        engine.1,
        d,
        r,
        f.space.rcs_fuel,
        f.space.shield * 100.,
        0.5 * f.density * f.airspeed.powi(2) / 1000.,
        f.space.heat / 1000.,
        if f.assist { "ON" } else { "OFF" },
        space::sas_label(&f),
        if matches!(f.space.stage, Stage::Return | Stage::Capsule) {
            "NO LANDING STAGE".into()
        } else {
            format!(
                "{} {:.0}%",
                if f.gear_extension < 0.001 {
                    "STOWED"
                } else if f.gear_extension > 0.999 {
                    "DOWN"
                } else if f.gear {
                    "DEPLOYING"
                } else {
                    "RETRACTING"
                },
                f.gear_extension * 100.
            )
        },
        if f.space.body == Body::Moon {
            format!(
                "RADAR ALT {:.0} m / V/S {:+.1}\nHORIZ {:.1} m/s / STOP ≈ {:.0} m\nHOVER {:.0}% · {} LEGS CONTACT\n",
                f.agl(),
                vertical,
                horizontal,
                stop,
                (f.mass() * gravity / engine.0.max(1.) * 100.).min(999.),
                f.space.legs.iter().filter(|x| **x > 0.).count()
            )
        } else {
            String::new()
        },
        if f.space.chute_armed {
            "CHUTES ARMED / AUTO BELOW 8 KM"
        } else if v.map {
            "M CLOSE MAP / FLIGHT CONTINUES"
        } else {
            "SPACE STAGE · K CHUTES · F7 SAS"
        }
    );
    for mut label in &mut labels {
        if label.0 != message {
            label.0 = message.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lunar_texture_matches_physical_pole_prime_meridian_and_outward_faces() {
        assert_eq!(moon_uv(-DVec3::Z), [0.5, 0.]);
        assert_eq!(moon_uv(-DVec3::Y), [0.5, 0.5]);
        let mesh = globe();
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!()
        };
        let Some(Indices::U32(i)) = mesh.indices() else {
            panic!()
        };
        for t in i.chunks_exact(3).skip(512).step_by(941) {
            let a = Vec3::from(p[t[0] as usize]);
            let b = Vec3::from(p[t[1] as usize]);
            let c = Vec3::from(p[t[2] as usize]);
            if (b - a).cross(c - a).length_squared() > 1. {
                assert!((b - a).cross(c - a).dot(a) > 0.);
            }
        }
    }
    #[test]
    fn closed_profiles_and_nozzle_throats_block_axial_sightlines() {
        fn hit(mesh: &Mesh, origin: Vec3, dir: Vec3) -> bool {
            let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                panic!()
            };
            let Some(Indices::U32(indices)) = mesh.indices() else {
                panic!()
            };
            indices.chunks_exact(3).any(|t| {
                let a = Vec3::from(p[t[0] as usize]);
                let e1 = Vec3::from(p[t[1] as usize]) - a;
                let e2 = Vec3::from(p[t[2] as usize]) - a;
                let cross = dir.cross(e2);
                let det = e1.dot(cross);
                if det.abs() < 1e-7 {
                    return false;
                }
                let s = origin - a;
                let u = s.dot(cross) / det;
                let q = s.cross(e1);
                let v = dir.dot(q) / det;
                let distance = e2.dot(q) / det;
                u >= -1e-5 && v >= -1e-5 && u + v <= 1.00001 && distance > 0.
            })
        }
        for points in [
            vec![(-4.15, 0.35), (1.25, 2.08)],
            vec![(1.8, 1.9), (4.25, 1.3)],
            vec![(4.4, 2.7), (7.4, 2.8)],
            vec![(8.7, 2.7), (18.2, 2.5)],
        ] {
            let mesh = solid_profile(&points, 64);
            for (z, dir) in [(30., -Vec3::Z), (-20., Vec3::Z)] {
                for x in [0.01, 0.1, 0.25] {
                    assert!(hit(&mesh, Vec3::new(x, 0.017, z), dir));
                }
            }
        }
        let bell = nozzle(&[(4.1, 0.28), (4.6, 0.35), (5.5, 0.68), (6.1, 0.95)], 64);
        assert!(hit(&bell, Vec3::new(0.01, 0.017, 7.), -Vec3::Z));
    }
    #[test]
    fn nested_leg_sleeves_slide_without_gaps_or_uniform_stretching() {
        let mut f = space::scenario(9);
        let base = Vec3::new(-2.5675, -2.5675, 5.1);
        let full = Vec3::new(-6.5, -6.5, 8.2);
        let rest = full.distance(base) / 4. + 0.08;
        for extension in [0., 0.25, 0.5, 0.75, 1.] {
            f.gear_extension = extension;
            let foot = space::leg_point(&f, 0).as_vec3();
            let dir = (foot - base).normalize();
            let mut previous_end = 0.;
            for sleeve in 0..4 {
                let (center, length) = sleeve_pose(base, foot, Some(sleeve), rest);
                let start = (center - base).dot(dir) - length * 0.5;
                let end = start + length;
                assert!(start <= previous_end + 1e-5 && length <= rest + 1e-5);
                previous_end = end;
            }
            assert!((previous_end - base.distance(foot)).abs() < 1e-5);
        }
    }
    #[test]
    fn direct_attitude_buttons_and_leg_command_select_persistent_state() {
        let mut app = App::new();
        app.insert_resource(Flight::spacecraft())
            .init_resource::<ViewState>()
            .init_resource::<crate::warp::Warp>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(Update, controls);
        for mode in 0..8 {
            let button = app
                .world_mut()
                .spawn((Action(10 + mode), Interaction::Pressed))
                .id();
            app.update();
            let f = app.world().resource::<Flight>();
            assert_eq!(f.sas_mode, mode);
            assert!(f.assist);
            app.world_mut().despawn(button);
        }
        app.world_mut().spawn((Action(5), Interaction::Pressed));
        app.update();
        let f = app.world().resource::<Flight>();
        assert!(f.gear);
        assert_eq!(f.gear_extension, 0.);
    }
    #[test]
    fn world_map_keeps_geographic_north_at_screen_top() {
        for scenario in [3, 9] {
            let mut f = space::scenario(scenario);
            for elapsed in [0., 100000., 500000.] {
                f.elapsed = elapsed;
                for target in [0, 1, 2, 3] {
                    let mut v = ViewState::default();
                    v.map_target = target;
                    let (north, equator, east) = map_axes(&f, &v);
                    let expected = if target == 2 || (target == 0 && f.space.body == Body::Moon) {
                        space::moon(elapsed).2 * -DVec3::Z
                    } else {
                        rotation().normalize()
                    };
                    assert!((world::frame(&f) * north.as_dvec3()).dot(expected) > 0.99999);
                    for yaw in [0.0_f32, 1., 4.] {
                        for pitch in [-1.45_f32, 0., 1.45] {
                            let offset = (equator * yaw.cos() + east * yaw.sin()) * pitch.cos()
                                - north * pitch.sin();
                            let camera =
                                Transform::from_translation(offset).looking_at(Vec3::ZERO, north);
                            assert!((camera.rotation * Vec3::Y).dot(north) > 0.1);
                            assert!((camera.rotation * Vec3::X).dot(north).abs() < 1e-5);
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn stage_and_parachute_controls_respect_vehicle_and_settings_focus() {
        let mut app = App::new();
        app.insert_resource(Flight::spacecraft())
            .init_resource::<ViewState>()
            .init_resource::<crate::warp::Warp>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(Update, controls);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
        app.update();
        assert_eq!(app.world().resource::<Flight>().space.stage, Stage::Descent);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.world_mut().resource_mut::<crate::warp::Warp>().editing = true;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
        app.update();
        assert_eq!(app.world().resource::<Flight>().space.stage, Stage::Descent);
        app.world_mut().resource_mut::<crate::warp::Warp>().editing = false;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyK);
        app.update();
        assert!(app.world().resource::<Flight>().space.chute_armed);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        let button = app
            .world_mut()
            .spawn((Action(1), Interaction::Pressed))
            .id();
        app.update();
        assert_eq!(app.world().resource::<Flight>().space.body, Body::Moon);
        assert_eq!(app.world().resource::<ViewState>().scenario, 9);
        app.world_mut().despawn(button);
    }
    #[test]
    fn lunar_surface_mesh_faces_outward_and_matches_contact_heights() {
        let n = space::landing_site();
        let anchor = n * (space::MOON_RADIUS + space::height(n));
        let mesh = surface_mesh(anchor);
        let bevy::mesh::VertexAttributeValues::Float32x3(vertices) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
        else {
            panic!()
        };
        let bevy::mesh::VertexAttributeValues::Float32x3(normals) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap()
        else {
            panic!()
        };
        for i in (0..vertices.len()).step_by(157) {
            let p = anchor + Vec3::from(vertices[i]).as_dvec3();
            assert!((p.length() - space::MOON_RADIUS - space::height(p)).abs() < 0.04);
            assert!(
                Vec3::from(normals[i]).is_finite()
                    && Vec3::from(normals[i]).as_dvec3().dot(n) > 0.5
            );
        }
    }
}
