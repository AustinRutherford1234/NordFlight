use crate::physics::*;
use bevy::{
    asset::RenderAssetUsages,
    math::{DQuat, DVec2, DVec3},
    mesh::Indices,
    prelude::*,
    render::render_resource::PrimitiveTopology,
};
use serde_json::Value;

pub const FUSELAGE_SECTIONS: &[(f32, f32, f32, f32)] = &[
    (-7.5, 0.015, 0.015, 0.02),
    (-6.8, 0.20, 0.18, 0.035),
    (-5.6, 0.41, 0.36, 0.04),
    (-4.0, 0.58, 0.56, 0.04),
    (-2.8, 0.69, 0.62, 0.02),
    (-1.3, 0.81, 0.64, -0.02),
    (0.8, 0.85, 0.66, -0.04),
    (2.8, 0.76, 0.62, 0.0),
    (4.6, 0.64, 0.56, 0.0),
    (5.5, 0.56, 0.51, 0.0),
];
/// Door skins use precisely the same elliptical stations as the airframe.
pub fn lower_skin(x: f32, z: f32) -> f32 {
    let s = FUSELAGE_SECTIONS.windows(2).find(|p| z <= p[1].0).unwrap();
    let t = ((z - s[0].0) / (s[1].0 - s[0].0)).clamp(0.0, 1.0);
    let w = s[0].1 + (s[1].1 - s[0].1) * t;
    let h = s[0].2 + (s[1].2 - s[0].2) * t;
    let cy = s[0].3 + (s[1].3 - s[0].3) * t;
    cy - h * (1.0 - (x / w).powi(2)).max(0.0).sqrt()
}
pub fn upper_skin(x: f32, z: f32) -> f32 {
    let s = FUSELAGE_SECTIONS.windows(2).find(|p| z <= p[1].0).unwrap();
    let t = ((z - s[0].0) / (s[1].0 - s[0].0)).clamp(0.0, 1.0);
    2.0 * (s[0].3 + (s[1].3 - s[0].3) * t) - lower_skin(x, z)
}

#[derive(Component)]
pub struct WorldObject {
    pub position: DVec3,
    pub rotation: DQuat,
}
#[derive(Component)]
pub struct Jet;
#[derive(Component)]
pub struct Spacecraft;
#[derive(Component)]
pub struct Exhaust {
    vehicle: Vehicle,
    layer: usize,
    exit: f32,
    last_expansion: f32,
}
#[derive(Component)]
pub struct EngineGlow {
    vehicle: Vehicle,
}
#[derive(Component)]
pub struct NozzlePetals {
    vehicle: Vehicle,
    radius: f32,
    odd: bool,
    count: usize,
    last_opening: f32,
}
#[derive(Component)]
pub struct NozzleRing {
    vehicle: Vehicle,
    tip_weight: f32,
}
#[derive(Component)]
pub struct Sun;
#[derive(Component)]
pub struct Globe;
#[derive(Component)]
pub struct Cloud;
#[derive(Component)]
pub struct Starfield;

pub fn geo_to_local(lon: f64, lat: f64, h: f64) -> DVec3 {
    let lat = lat.to_radians();
    let lon = lon.to_radians();
    // Geographic east must follow the positive spin direction. With +Y as
    // north, east-positive longitude is -Z, not +Z (which mirrored the globe).
    let p = DVec3::new(lat.cos() * lon.cos(), lat.sin(), -lat.cos() * lon.sin()) * (RADIUS + h);
    let b = 58.428333_f64.to_radians();
    let l = 12.711111_f64.to_radians();
    let up = DVec3::new(b.cos() * l.cos(), b.sin(), -b.cos() * l.sin());
    let north = DVec3::new(-b.sin() * l.cos(), b.cos(), b.sin() * l.sin());
    let east = DVec3::new(-l.sin(), 0.0, -l.cos());
    let forward = north * 184.59_f64.to_radians().cos() + east * 184.59_f64.to_radians().sin();
    let right = forward.cross(up);
    DVec3::new(p.dot(right), p.dot(up), p.dot(-forward))
}
pub fn frame(f: &Flight) -> DQuat {
    render_reference(f).0
}
fn render_reference(f: &Flight) -> (DQuat, DVec3) {
    if f.vehicle == Vehicle::Spacecraft && f.space.body == crate::space::Body::Moon {
        return (frame_uncached(f), f.position);
    }
    thread_local! {
        static CACHE: std::cell::Cell<Option<([u64;3], Vehicle, DQuat, DVec3)>> = const { std::cell::Cell::new(None) };
    }
    let key = f.position.to_array().map(f64::to_bits);
    CACHE.with(|cache| {
        if let Some((cached, vehicle, rotation, origin)) = cache.get()
            && cached == key
            && vehicle == f.vehicle
        {
            return (rotation, origin);
        }
        let rotation = frame_uncached(f);
        let origin = if f.vehicle == Vehicle::Spacecraft && f.altitude() > 100000. {
            f.position
        } else {
            f.up() * RADIUS
        };
        cache.set(Some((key, f.vehicle, rotation, origin)));
        (rotation, origin)
    })
}
fn frame_uncached(f: &Flight) -> DQuat {
    let up = f.up();
    let (north, east) = geographic_axes(up);
    let runway = 184.59_f64.to_radians();
    let right = east * runway.cos() - north * runway.sin();
    let back = right.cross(up).normalize();
    DQuat::from_mat3(&bevy::math::DMat3::from_cols(right, up, back))
}
pub fn local(p: DVec3, f: &Flight) -> Vec3 {
    (frame(f).conjugate() * (p - render_origin(f))).as_vec3()
}
/// Keep the vehicle's GPU transform near zero in deep space. Subtract in f64
/// before converting: adding metre-scale children to a million-metre f32 root
/// otherwise destroys nozzle, canopy and panel precision.
pub fn render_origin(f: &Flight) -> DVec3 {
    render_reference(f).1
}
pub fn local_rot(q: DQuat, f: &Flight) -> Quat {
    (frame(f).conjugate() * q).as_quat()
}

fn mat(
    ms: &mut Assets<StandardMaterial>,
    color: Color,
    metal: f32,
    rough: f32,
) -> Handle<StandardMaterial> {
    ms.add(StandardMaterial {
        base_color: color,
        metallic: metal,
        perceptual_roughness: rough,
        ..default()
    })
}
fn object(
    c: &mut Commands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    p: DVec3,
    q: DQuat,
    scale: Vec3,
) {
    c.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_scale(scale),
        WorldObject {
            position: p,
            rotation: q,
        },
    ));
}

pub fn setup_world(
    mut c: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    f: Res<Flight>,
) {
    // Actual Vänern shoreline from Natural Earth; no claim of a surveyed height map.
    let lakes: Value = serde_json::from_str(include_str!("../assets/lakes.geojson")).unwrap();
    for feature in lakes["features"].as_array().unwrap() {
        let name = feature["properties"]["name"].as_str().unwrap_or("");
        if name.contains("Väner") || name.contains("Vanern") {
            let coords = &feature["geometry"]["coordinates"];
            let ring = if feature["geometry"]["type"] == "MultiPolygon" {
                &coords[0][0]
            } else {
                &coords[0]
            };
            let poly = ring
                .as_array()
                .unwrap()
                .iter()
                .map(|v| {
                    let p = geo_to_local(v[0].as_f64().unwrap(), v[1].as_f64().unwrap(), 0.0);
                    DVec2::new(p.x, p.z)
                })
                .collect();
            let _ = LAKE.set(poly);
            break;
        }
    }
    let mut earth = Image::from_buffer(
        include_bytes!("../assets/earth.jpg"),
        bevy::image::ImageType::Extension("jpg"),
        bevy::image::CompressedImageFormats::NONE,
        true,
        bevy::image::ImageSampler::default(),
        RenderAssetUsages::default(),
    )
    .unwrap();
    crate::earth::readable_oceans(&mut earth);
    earth.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::Repeat,
        address_mode_v: bevy::image::ImageAddressMode::ClampToEdge,
        ..bevy::image::ImageSamplerDescriptor::linear()
    });
    let texture = images.add(earth);
    let globe_mat = materials.add(StandardMaterial {
        base_color_texture: Some(texture),
        perceptual_roughness: 1.0,
        reflectance: 0.0,
        ..default()
    });
    c.spawn((
        Mesh3d(meshes.add(globe_mesh(768, 384))),
        MeshMaterial3d(globe_mat),
        Transform::default(),
        WorldObject {
            position: DVec3::ZERO,
            rotation: DQuat::IDENTITY,
        },
        Globe,
        crate::earth::Surface,
        bevy::light::NotShadowCaster,
    ));
    crate::earth::setup(&mut c, &mut meshes, &mut materials, &mut images);
    // The shipping renderer samples Nordic detail on this same globe surface.
    // Keep the old overlay only for the developer CPU-reference renderer.
    if std::env::var_os("NORD_EARTH_CPU").is_some() {
        crate::earth::setup_region(&mut c, &mut meshes, &mut materials, &mut images);
    }
    crate::terrain::setup(&mut c, &mut meshes, &mut materials, &mut images, &f);
    let water = materials.add(StandardMaterial {
        base_color: Color::srgb(0.025, 0.13, 0.18),
        metallic: 0.55,
        perceptual_roughness: 0.16,
        reflectance: 0.8,
        ..default()
    });
    let ocean_mesh = meshes.add(terrain_water_mesh(90000.0, 100));
    c.spawn((
        Mesh3d(ocean_mesh),
        MeshMaterial3d(water),
        Transform::default(),
        crate::earth::LocalWater,
        WorldObject {
            position: DVec3::new(0.0, RADIUS + 0.2, 0.0),
            rotation: DQuat::IDENTITY,
        },
    ));
    let cube = meshes.add(Cuboid::default());
    let asphalt = mat(&mut materials, Color::srgb(0.075, 0.088, 0.105), 0.08, 0.92);
    let white = mat(&mut materials, Color::srgb(0.87, 0.88, 0.81), 0.0, 0.85);
    let grass = mat(&mut materials, Color::srgb(0.18, 0.22, 0.12), 0.0, 1.0);
    object(
        &mut c,
        cube.clone(),
        grass,
        surface_point(0.0, 0.0, RUNWAY_HEIGHT - 0.35),
        DQuat::IDENTITY,
        Vec3::new(180.0, 0.7, 2900.0),
    );
    object(
        &mut c,
        cube.clone(),
        asphalt.clone(),
        surface_point(0.0, 0.0, RUNWAY_HEIGHT + 0.1),
        DQuat::IDENTITY,
        Vec3::new(45.0, 0.15, (RUNWAY_HALF * 2.0) as f32),
    );
    // Centerline, edge lines, aiming points and piano-key thresholds.
    for i in -18..19 {
        object(
            &mut c,
            cube.clone(),
            white.clone(),
            surface_point(0.0, i as f64 * 60.0, RUNWAY_HEIGHT + 0.20),
            DQuat::IDENTITY,
            Vec3::new(0.7, 0.02, 28.0),
        );
    }
    for x in [-21.8, 21.8] {
        object(
            &mut c,
            cube.clone(),
            white.clone(),
            surface_point(x, 0.0, RUNWAY_HEIGHT + 0.20),
            DQuat::IDENTITY,
            Vec3::new(0.4, 0.02, 2240.0),
        );
    }
    for end in [-1.0, 1.0] {
        for i in -4..5 {
            if i == 0 {
                continue;
            }
            object(
                &mut c,
                cube.clone(),
                white.clone(),
                surface_point(i as f64 * 3.6, end * 1085.0, RUNWAY_HEIGHT + 0.22),
                DQuat::IDENTITY,
                Vec3::new(1.6, 0.025, 28.0),
            );
        }
        for x in [-9.0, 9.0] {
            object(
                &mut c,
                cube.clone(),
                white.clone(),
                surface_point(x, end * 820.0, RUNWAY_HEIGHT + 0.22),
                DQuat::IDENTITY,
                Vec3::new(5.0, 0.025, 38.0),
            );
        }
    }
    let apron = mat(&mut materials, Color::srgb(0.24, 0.26, 0.24), 0.0, 0.9);
    object(
        &mut c,
        cube.clone(),
        apron,
        surface_point(150.0, 620.0, RUNWAY_HEIGHT + 0.12),
        DQuat::IDENTITY,
        Vec3::new(200.0, 0.1, 650.0),
    );
    object(
        &mut c,
        cube.clone(),
        asphalt,
        surface_point(80.0, 500.0, RUNWAY_HEIGHT + 0.15),
        DQuat::IDENTITY,
        Vec3::new(170.0, 0.1, 20.0),
    );
    let hangar = mat(&mut materials, Color::srgb(0.28, 0.34, 0.33), 0.55, 0.7);
    let roof = mat(&mut materials, Color::srgb(0.11, 0.15, 0.17), 0.4, 0.7);
    for i in 0..4 {
        let z = 160.0 + i as f64 * 170.0;
        object(
            &mut c,
            cube.clone(),
            hangar.clone(),
            surface_point(260.0, z, RUNWAY_HEIGHT + 9.0),
            DQuat::IDENTITY,
            Vec3::new(75.0, 18.0, 90.0),
        );
        object(
            &mut c,
            cube.clone(),
            roof.clone(),
            surface_point(260.0, z, RUNWAY_HEIGHT + 19.0),
            DQuat::IDENTITY,
            Vec3::new(80.0, 3.0, 95.0),
        );
    }
    object(
        &mut c,
        cube.clone(),
        hangar,
        surface_point(130.0, -210.0, RUNWAY_HEIGHT + 16.0),
        DQuat::IDENTITY,
        Vec3::new(16.0, 32.0, 16.0),
    );
    let glass = mat(&mut materials, Color::srgb(0.06, 0.16, 0.21), 0.7, 0.12);
    object(
        &mut c,
        cube.clone(),
        glass,
        surface_point(130.0, -210.0, RUNWAY_HEIGHT + 32.0),
        DQuat::IDENTITY,
        Vec3::new(23.0, 7.0, 23.0),
    );
    let light = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.83, 0.46),
        emissive: LinearRgba::new(9.0, 5.0, 1.5, 1.0),
        unlit: true,
        ..default()
    });
    let lens = meshes.add(Sphere::new(0.23).mesh().uv(8, 6));
    for i in -22..23 {
        for x in [-23.3, 23.3] {
            object(
                &mut c,
                lens.clone(),
                light.clone(),
                surface_point(x, i as f64 * 50.0, RUNWAY_HEIGHT + 0.45),
                DQuat::IDENTITY,
                Vec3::ONE,
            );
        }
    }
    // Scattered cumulus banks, deliberately bounded to keep draw cost steady.
    let cloud_mesh = meshes.add(Sphere::new(1.0).mesh().uv(12, 8));
    let cloud_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.94, 0.92, 0.88),
        perceptual_roughness: 1.0,
        ..default()
    });
    for i in 0..26 {
        let x = hash(i * 3 + 100) * 90000.0 - 45000.0;
        let z = hash(i * 11 + 80) * 90000.0 - 45000.0;
        for j in 0..4 {
            c.spawn((
                Mesh3d(cloud_mesh.clone()),
                MeshMaterial3d(cloud_mat.clone()),
                Transform::from_scale(Vec3::new(850.0, 420.0, 580.0) * (0.7 + j as f32 * 0.13)),
                WorldObject {
                    position: surface_point(x + j as f64 * 450.0, z, 2200.0 + hash(i * 7) * 1100.0),
                    rotation: DQuat::IDENTITY,
                },
                Cloud,
                bevy::light::NotShadowCaster,
            ));
        }
    }
    setup_aircraft(&mut c, &mut meshes, &mut materials, &mut images);
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    let mut colors = Vec::new();
    for i in 0..600 {
        let y = hash(i * 31 + 400) * 2.0 - 1.0;
        let azimuth = hash(i * 17 + 300) * std::f64::consts::TAU;
        let direction = DVec3::new(
            (1.0 - y * y).sqrt() * azimuth.cos(),
            y,
            (1.0 - y * y).sqrt() * azimuth.sin(),
        )
        .as_vec3();
        let axis = if direction.y.abs() > 0.9 {
            Vec3::X
        } else {
            Vec3::Y
        };
        let right = direction.cross(axis).normalize();
        let up = right.cross(direction).normalize();
        let center = direction * 30_000_000.0;
        let size = 5000.0 + hash(i * 29 + 17) as f32 * 16000.0;
        let base = positions.len() as u32;
        for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            positions.push((center + (right * x + up * y) * size).to_array());
            let warmth = hash(i * 23 + 31) as f32;
            colors.push([0.60 + 0.40 * warmth, 0.70 + 0.30 * warmth, 1.0, 1.0]);
        }
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    let stars = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices));
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    c.spawn((
        Mesh3d(meshes.add(stars)),
        MeshMaterial3d(material),
        Transform::default(),
        Visibility::Hidden,
        Starfield,
        bevy::light::NotShadowCaster,
        bevy::light::NotShadowReceiver,
    ));
}

pub fn update_stars(
    f: Res<Flight>,
    view: Res<crate::ViewState>,
    mut stars: Query<(&mut Transform, &mut Visibility), With<Starfield>>,
    star_materials: Query<&MeshMaterial3d<StandardMaterial>, With<Starfield>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let opacity = crate::earth::smoothstep(((f.altitude() - 45000.0) / 55000.0) as f32);
    for handle in &star_materials {
        if let Some(mat) = materials.get_mut(&handle.0) {
            mat.alpha_mode = AlphaMode::Blend;
            mat.base_color = Color::srgba(1.0, 1.0, 1.0, opacity);
        }
    }
    for (mut transform, mut visibility) in &mut stars {
        *visibility = if opacity > 0.0 && view.index != 9 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        transform.translation = local(f.position, &f);
        transform.rotation = frame(&f).conjugate().as_quat();
    }
}

fn hash(i: i32) -> f64 {
    let x = (i as f64 * 127.1 + 311.7).sin() * 43758.5453;
    x - x.floor()
}
pub(crate) fn terrain_mesh(x: f64, z: f64, size: f64, n: usize, anchor: DVec3) -> Mesh {
    let mut p = Vec::new();
    let mut colors = Vec::new();
    let mut uv = Vec::new();
    let mut idx = Vec::new();
    for j in 0..=n {
        for i in 0..=n {
            let xx = x + size * i as f64 / n as f64;
            let zz = z + size * j as f64 / n as f64;
            let h = terrain_height(xx, zz);
            p.push(
                (crate::terrain::point(xx, zz, h) - anchor)
                    .as_vec3()
                    .to_array(),
            );
            uv.push([xx as f32 * 0.025, zz as f32 * 0.025]);
            let col = crate::terrain::color(xx, zz, h);
            colors.push(col);
        }
    }
    for j in 0..n {
        for i in 0..n {
            let a = (j * (n + 1) + i) as u32;
            let b = a + 1;
            let d = a + (n + 1) as u32;
            let cc = d + 1;
            idx.extend([a, d, b, b, d, cc]);
        }
    }
    // Skirts conceal T-junctions where neighboring tiles use different grids.
    let perimeter: Vec<usize> = (0..=n)
        .chain((1..=n).map(|j| j * (n + 1) + n))
        .chain((0..n).rev().map(|i| n * (n + 1) + i))
        .chain((1..n).rev().map(|j| j * (n + 1)))
        .collect();
    let skirt_start = p.len();
    for &index in &perimeter {
        let mut vertex = p[index];
        vertex[1] -= 25.0;
        p.push(vertex);
        uv.push(uv[index]);
        colors.push(colors[index]);
    }
    for i in 0..perimeter.len() {
        let next = (i + 1) % perimeter.len();
        let a = perimeter[i] as u32;
        let b = perimeter[next] as u32;
        let c = (skirt_start + i) as u32;
        let d = (skirt_start + next) as u32;
        idx.extend([a, c, b, b, c, d]);
    }
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, p);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    m.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    m.insert_indices(Indices::U32(idx));
    m.compute_smooth_normals();
    // Keep skirt faces out of visible edge normals, including at LOD changes.
    if let Some(bevy::mesh::VertexAttributeValues::Float32x3(normals)) =
        m.attribute_mut(Mesh::ATTRIBUTE_NORMAL)
    {
        for j in 0..=n {
            for i in 0..=n {
                let xx = x + size * i as f64 / n as f64;
                let zz = z + size * j as f64 / n as f64;
                normals[j * (n + 1) + i] =
                    crate::terrain::normal(crate::terrain::point(xx, zz, 0.))
                        .as_vec3()
                        .to_array();
            }
        }
    }
    m
}
fn terrain_water_mesh(size: f64, n: usize) -> Mesh {
    let mut p = Vec::new();
    let mut idx = Vec::new();
    let mut uv = Vec::new();
    for j in 0..=n {
        for i in 0..=n {
            let x = (i as f64 / n as f64 - 0.5) * size;
            let z = (j as f64 / n as f64 - 0.5) * size;
            p.push([
                x as f32,
                (-((x * x + z * z) / (2.0 * RADIUS))) as f32,
                z as f32,
            ]);
            uv.push([i as f32 / n as f32, j as f32 / n as f32]);
        }
    }
    for j in 0..n {
        for i in 0..n {
            let a = (j * (n + 1) + i) as u32;
            let d = a + (n + 1) as u32;
            idx.extend([a, d, a + 1, a + 1, d, d + 1]);
        }
    }
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, p);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    m.insert_indices(Indices::U32(idx));
    m.compute_smooth_normals();
    m
}
pub(crate) fn globe_mesh(nx: usize, ny: usize) -> Mesh {
    let mut p = Vec::new();
    let mut normal = Vec::new();
    let mut uv = Vec::new();
    let mut idx = Vec::new();
    for j in 0..=ny {
        let lat = 90.0 - j as f64 / ny as f64 * 180.0;
        for i in 0..=nx {
            let lon = i as f64 / nx as f64 * 360.0 - 180.0;
            let v = geo_to_local(lon, lat, -80.0);
            p.push(v.as_vec3().to_array());
            normal.push(v.normalize().as_vec3().to_array());
            uv.push([i as f32 / nx as f32, j as f32 / ny as f32]);
        }
    }
    for j in 0..ny {
        for i in 0..nx {
            let a = (j * (nx + 1) + i) as u32;
            let d = a + (nx + 1) as u32;
            idx.extend([a, d, a + 1, a + 1, d, d + 1]);
        }
    }
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, p);
    m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normal);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    m.insert_indices(Indices::U32(idx));
    m
}

fn fin_mesh(vertices: Vec<[f32; 3]>, indices: Vec<u32>) -> Mesh {
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    let n = vertices.len();
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; n]);
    m.insert_indices(Indices::U32(indices));
    m.compute_smooth_normals();
    m
}
fn setup_aircraft(
    c: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) {
    let gray = materials.add(StandardMaterial {
        base_color: Color::srgb(0.40, 0.47, 0.51),
        metallic: 0.28,
        perceptual_roughness: 0.63,
        cull_mode: None,
        ..default()
    });
    let dark = mat(materials, Color::srgb(0.09, 0.12, 0.15), 0.7, 0.28);
    let canopy = materials.add(StandardMaterial {
        base_color: Color::srgba(0.16, 0.21, 0.22, 0.30),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        metallic: 0.18,
        perceptual_roughness: 0.08,
        ..default()
    });
    let root = c
        .spawn((Transform::default(), Visibility::default(), Jet))
        .id();
    let mut parts = Vec::new();
    let fuselage = meshes.add(fuselage_mesh(&FUSELAGE_SECTIONS[3..]));
    parts.push((
        fuselage,
        gray.clone(),
        Vec3::ZERO,
        Vec3::ONE,
        Quat::IDENTITY,
    ));
    let radome = mat(materials, Color::srgb(0.28, 0.32, 0.34), 0.18, 0.6);
    let nose = meshes.add(fuselage_mesh(&FUSELAGE_SECTIONS[..4]));
    parts.push((nose, radome, Vec3::ZERO, Vec3::ONE, Quat::IDENTITY));
    let cockpit = meshes.add(canopy_mesh(false));
    parts.push((
        cockpit,
        canopy.clone(),
        Vec3::ZERO,
        Vec3::ONE,
        Quat::IDENTITY,
    ));
    parts.push((
        meshes.add(canopy_mesh(true)),
        gray.clone(),
        Vec3::ZERO,
        Vec3::ONE,
        Quat::IDENTITY,
    ));
    // Fixed wing stops at the elevon hinge: moving panels have their own geometry.
    for side in [-1.0, 1.0] {
        let wing = meshes.add(cut_main_bays(wing_mesh(side)));
        parts.push((wing, gray.clone(), Vec3::ZERO, Vec3::ONE, Quat::IDENTITY));
    }
    let fin = meshes.add(solid_panel(
        &[
            Vec3::new(0.0, 0.45, 2.65),
            Vec3::new(0.0, 3.0, 4.15),
            Vec3::new(0.0, 2.95, 4.75),
            Vec3::new(0.0, 0.5, 4.95),
        ],
        Vec3::X * 0.055,
    ));
    parts.push((fin, gray.clone(), Vec3::ZERO, Vec3::ONE, Quat::IDENTITY));
    aircraft_details(c, meshes, materials, root, &gray, &dark);
    let engine = meshes.add(engine_surface(0.55, 1.2, false, 0));
    parts.push((
        engine,
        dark.clone(),
        Vec3::new(0.0, 0.0, 4.9),
        Vec3::ONE,
        Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
    ));
    let boxmesh = meshes.add(Cuboid::default());
    for side in [-1.0, 1.0] {
        parts.push((
            boxmesh.clone(),
            gray.clone(),
            Vec3::new(side * 4.1, -0.08, 2.5),
            Vec3::new(0.13, 0.13, 2.0),
            Quat::IDENTITY,
        ));
    }
    for (index, (m, mat, p, s, q)) in parts.into_iter().enumerate() {
        let e = c
            .spawn((
                Mesh3d(m),
                MeshMaterial3d(mat),
                Transform::from_translation(p)
                    .with_scale(s)
                    .with_rotation(q),
            ))
            .id();
        if (3..=5).contains(&index) {
            c.entity(e).insert(Fragile);
        }
        c.entity(root).add_child(e);
    }
    engine_detail(c, meshes, materials, root, Vehicle::Gripen, 6.2, 0.55);
    crate::gear::setup(c, meshes, materials, root);
    crate::aircraft_detail::setup(c, meshes, materials, root);
    let rocket = c
        .spawn((Transform::default(), Visibility::Hidden, Spacecraft))
        .id();
    crate::lunar::vehicle(c, meshes, materials, images, rocket);
    crate::cockpit::setup(c, meshes, materials, images, root, Vehicle::Gripen);
}

#[derive(Component)]
pub struct RcsPlume {
    pub(crate) torque: DVec3,
}
pub fn update_rcs(f: Res<Flight>, mut plumes: Query<(&RcsPlume, &mut Visibility)>) {
    for (plume, mut visibility) in &mut plumes {
        *visibility = if f.vehicle == Vehicle::Spacecraft
            && !f.crashed
            && plume.torque.dot(f.rcs_torque) > 80.0
        {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

pub fn update_world(
    f: Res<Flight>,
    e: Res<Environment>,
    view: Res<crate::ViewState>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut objects: Query<(&WorldObject, &mut Transform, Option<&Globe>)>,
    mut jets: Query<
        (&mut Transform, &mut Visibility),
        (With<Jet>, Without<WorldObject>, Without<Spacecraft>),
    >,
    mut rockets: Query<
        (&mut Transform, &mut Visibility),
        (With<Spacecraft>, Without<WorldObject>, Without<Jet>),
    >,
    mut exhaust: Query<
        (
            &mut Exhaust,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
            &mut Transform,
            &mut Visibility,
        ),
        (
            With<Exhaust>,
            Without<Jet>,
            Without<Spacecraft>,
            Without<WorldObject>,
        ),
    >,
) {
    let _measure = crate::performance::span(9);
    let basis = frame(&f).conjugate();
    let origin = render_origin(&f);
    let daylight = ((e.sun_direction().dot(f.up()) as f32 + 0.06) / 0.18).clamp(0.0, 1.0);
    objects.par_iter_mut().for_each(|(o, mut t, globe)| {
        let p = (basis * (o.position - origin)).as_vec3();
        let q = (basis * o.rotation).as_quat();
        if t.translation != p {
            t.translation = p;
        }
        if t.rotation != q {
            t.rotation = q;
        }
        if globe.is_some() {
            let scale = Vec3::splat(if view.index == 9 { 180.0 } else { 1.0 });
            if t.scale != scale {
                t.scale = scale;
            }
        }
    });
    for (mut t, mut v) in &mut jets {
        let mut desired = *t;
        desired.translation = local(f.position, &f);
        desired.rotation = local_rot(f.orientation, &f);
        t.set_if_neq(desired);
        v.set_if_neq(if f.vehicle == Vehicle::Gripen && f.aircraft==crate::aircraft::Aircraft::Gripen && !f.crashed {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
    }
    for (mut t, mut v) in &mut rockets {
        let mut desired = *t;
        desired.translation = local(f.position, &f);
        desired.rotation = local_rot(f.orientation, &f);
        t.set_if_neq(desired);
        v.set_if_neq(if f.vehicle == Vehicle::Spacecraft {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
    }
    for (mut effect, mesh, material, mut t, mut v) in &mut exhaust {
        let power = f.spool.clamp(0.0, 1.0) as f32;
        let burner = ((power - 0.86) / 0.14).clamp(0.0, 1.0);
        let active = f.vehicle == effect.vehicle
            && f.engine
            && !f.crashed
            && f.throttle > 0.0
            && (f.fuel > 0.0 || f.infinite_fuel)
            && f.thrust > 0.0
            && power > 0.0;
        *v = if active {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if !active {
            // Clear optical output when shutdown leaves stale physics telemetry,
            // including an engine toggle while the simulation is paused.
            t.scale = Vec3::ZERO;
            crate::render_cache::optics(
                &mut materials,
                &material.0,
                Color::NONE,
                LinearRgba::BLACK,
            );
            continue;
        }
        {
            let day_burner = burner * daylight;
            let (r, g, b, alpha) = if effect.vehicle == Vehicle::Gripen {
                [
                    (0.62, 0.82, 1.0, 0.50 + 0.20 * day_burner),
                    (0.15, 0.46, 1.0, 0.27 + 0.18 * day_burner),
                    (1.0, 0.48, 0.14, 0.10 + 0.34 * daylight),
                ][effect.layer]
            } else {
                [
                    (0.75, 0.88, 1.0, 0.44),
                    (0.22, 0.50, 1.0, 0.28),
                    if f.density > 0.08 {
                        (1.0, 0.40, 0.12, 0.14)
                    } else {
                        (0.24, 0.44, 0.95, 0.13)
                    },
                ][effect.layer]
            };
            let brightness = if effect.vehicle == Vehicle::Gripen && effect.layer == 2 {
                burner
            } else {
                0.35 + 0.65 * power
            };
            let color = Color::srgba(r, g, b, alpha * brightness);
            let emission = if effect.vehicle == Vehicle::Gripen {
                1.8 + day_burner * if effect.layer == 2 { 4.2 } else { 1.3 }
            } else {
                1.8
            };
            let emissive = LinearRgba::new(
                r * emission * brightness,
                g * emission * brightness,
                b * emission * brightness,
                1.0,
            );
            crate::render_cache::optics(&mut materials,&material.0,color,emissive);
        }
        let expansion = if effect.vehicle == Vehicle::Spacecraft {
            1.0 + 2.5 * (1.0 - (f.density / 1.225).clamp(0.0, 1.0)) as f32
        } else {
            1.0
        };
        if effect.vehicle == Vehicle::Spacecraft
            && (effect.last_expansion - expansion).abs() > 0.001
        {
            if let Some(mesh) = meshes.get_mut(&mesh.0) {
                if let Some(bevy::mesh::VertexAttributeValues::Float32x3(vertices)) =
                    mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
                {
                    for vertex in vertices {
                        let along = (vertex[1] / 6.0 + 0.5).clamp(0.0, 1.0);
                        let old = 1.0 + (effect.last_expansion - 1.0) * along;
                        let new = 1.0 + (expansion - 1.0) * along;
                        vertex[0] *= new / old;
                        vertex[2] *= new / old;
                    }
                }
            }
            effect.last_expansion = expansion;
        }
        let pulse = 1.0 + 0.035 * (e.time as f32 * 29.0 + effect.layer as f32 * 1.7).sin();
        let length = if effect.vehicle == Vehicle::Gripen {
            // A short dry-engine plume remains visible below the afterburner range.
            (0.20 + 0.18 * power + 0.24 * burner) * pulse
        } else {
            (0.12 + 1.05 * power) * pulse
        };
        let opening = nozzle_opening(&f, effect.vehicle);
        t.scale = Vec3::new(opening, length, opening);
        t.translation.z = effect.exit + 3.0 * length;
    }
}

#[derive(Component)]
pub struct Fragile;
#[derive(Component)]
pub struct CrashPiece(pub usize);
#[derive(Default)]
pub struct CrashState {
    active: bool,
    elapsed: f64,
    positions: [DVec3; 24],
    velocities: [DVec3; 24],
}
pub fn setup_destruction(
    mut c: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let chunk = meshes.add(Cuboid::default());
    let sphere = meshes.add(Sphere::new(1.0).mesh().uv(10, 6));
    let metal = mats.add(StandardMaterial {
        base_color: Color::srgb(0.27, 0.32, 0.36),
        metallic: 0.7,
        ..default()
    });
    let smoke = mats.add(StandardMaterial {
        base_color: Color::srgb(0.04, 0.045, 0.05),
        perceptual_roughness: 1.0,
        ..default()
    });
    let flash = mats.add(StandardMaterial {
        emissive: LinearRgba::new(300000.0, 70000.0, 8000.0, 1.0),
        unlit: true,
        ..default()
    });
    for i in 0..24 {
        c.spawn((
            Mesh3d(if i < 15 {
                chunk.clone()
            } else {
                sphere.clone()
            }),
            MeshMaterial3d(if i < 15 {
                metal.clone()
            } else if i == 15 {
                flash.clone()
            } else {
                smoke.clone()
            }),
            Transform::default(),
            Visibility::Hidden,
            CrashPiece(i),
            bevy::light::NotShadowCaster,
        ));
    }
}
pub fn update_destruction(
    f: Res<Flight>,
    e: Res<Environment>,
    combat: Option<Res<crate::combat::Combat>>,
    mut state: Local<CrashState>,
    mut pieces: Query<(&CrashPiece, &mut Transform, &mut Visibility)>,
    mut fragile: Query<
        &mut Visibility,
        (With<Fragile>, Without<CrashPiece>, Without<ControlSurface>),
    >,
) {
    for mut vis in &mut fragile {
        let desired = if f.crashed {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if *vis != desired {
            *vis = desired;
        }
    }
    if combat.as_ref().is_some_and(|c| c.active) {
        for (_, _, mut vis) in &mut pieces {
            *vis = Visibility::Hidden;
        }
        state.active = false;
        return;
    }
    if !f.crashed {
        state.active = false;
        for (_, _, mut vis) in &mut pieces {
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
        }
        return;
    }
    if !state.active {
        state.active = true;
        state.elapsed = f.elapsed;
        for i in 0..24 {
            let phase = i as f64 * 2.39996;
            let scatter = DVec3::new(phase.cos(), 0.3 + (i % 4) as f64 * 0.25, phase.sin());
            state.positions[i] = f.wreck_origin + scatter * 1.5;
            state.velocities[i] =
                f.wreck_velocity + scatter * (8.0 + (f.impact_energy / 100000.0).sqrt().min(35.0));
        }
    }
    let dt = if f.paused {
        0.0
    } else {
        (f.elapsed - state.elapsed).max(0.0)
    };
    state.elapsed = f.elapsed;
    for (piece, mut t, mut vis) in &mut pieces {
        let i = piece.0;
        if i < 15 {
            let up = state.positions[i].normalize();
            state.velocities[i] -= up * 9.81 * dt;
            state.velocities[i] *= (-dt * 0.05).exp();
            let velocity = state.velocities[i];
            state.positions[i] += velocity * dt;
            let h = terrain_height(state.positions[i].x, state.positions[i].z).max(0.0) + 0.25;
            if state.positions[i].length() - RADIUS < h {
                state.positions[i] = up * (RADIUS + h);
                let v = state.velocities[i].dot(up);
                if v < 0.0 {
                    state.velocities[i] -= up * v * 1.18;
                }
                state.velocities[i] *= (-dt * 1.6).exp();
            }
            t.translation = local(state.positions[i], &f);
            t.scale = Vec3::new(
                0.4 + (i % 3) as f32 * 0.35,
                0.18,
                0.7 + (i % 4) as f32 * 0.4,
            );
            t.rotate_local_x(dt as f32 * (i as f32 * 0.23 + 0.5));
            *vis = Visibility::Visible;
        } else if i == 15 {
            t.translation = local(f.wreck_origin, &f);
            t.scale = Vec3::splat((f.wreck_age as f32 * 25.0).min(10.0));
            *vis = if f.wreck_age < 0.45 && f.fuel > 10.0 && f.impact_energy > 1e6 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        } else {
            let age = (f.wreck_age - (i - 16) as f64 * 0.25).max(0.0);
            let up = f.wreck_origin.normalize();
            t.translation = local(
                f.wreck_origin + up * (age * 4.0 + 2.0) + e.wind(f.wreck_origin) * age,
                &f,
            );
            t.scale = Vec3::splat((1.0 + age * 1.2).min(18.0) as f32);
            *vis = if f.fuel > 10.0 && f.wreck_age > (i - 16) as f64 * 0.25 && f.wreck_age < 25.0 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}

// Open, layered surfaces keep the nozzle interior visible and the plume attached at its exit.
pub(crate) fn engine_surface(radius: f32, length: f32, plume: bool, layer: usize) -> Mesh {
    let rings = 40;
    let segments = 64;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    let mut colors = Vec::new();
    for j in 0..=rings {
        let t = j as f32 / rings as f32;
        let r = if plume {
            let envelope = match layer {
                0 => (1.0 - 0.20 * t) * (1.0 - t).powf(0.45),
                1 => (1.0 + 0.70 * t) * (1.0 - t).powf(0.78),
                _ => (1.0 + 1.40 * t) * (1.0 - t).powf(0.95),
            };
            let ripples = 1.0 + 0.035 * (t * 9.0 * std::f32::consts::PI).sin();
            radius * envelope * ripples
        } else {
            radius * (0.58 + 0.42 * t * t)
        };
        for i in 0..=segments {
            let a = i as f32 / segments as f32 * std::f32::consts::TAU;
            positions.push([r * a.cos(), (t - 0.5) * length, r * a.sin()]);
            normals.push([a.cos(), 0.0, a.sin()]);
            if plume {
                // Smooth axial transparency removes the hard cone silhouette.
                // Brighter periodic bands suggest shock cells without particles.
                let fade = (1.0 - t).powf(if layer == 0 { 1.2 } else { 1.55 });
                let bands = 0.74 + 0.26 * (t * 8.0 * std::f32::consts::PI).cos().powi(2);
                let feather = (t * 28.0).min(1.0);
                colors.push([1.0, 1.0, 1.0, fade * bands * feather]);
            }
            if j < rings && i < segments {
                let n = j * (segments + 1) + i;
                indices.extend([
                    n,
                    n + segments + 1,
                    n + 1,
                    n + 1,
                    n + segments + 1,
                    n + segments + 2,
                ]);
            }
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_indices(Indices::U32(indices));
    if plume {
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }
    mesh
}
fn nozzle_opening(f: &Flight, vehicle: Vehicle) -> f32 {
    let powered =
        f.vehicle == vehicle && f.engine && !f.crashed && (f.fuel > 0.0 || f.infinite_fuel);
    let power = if powered {
        f.spool.clamp(0.0, 1.0) as f32
    } else {
        0.0
    };
    0.82 + 0.30 * power
}
fn nozzle_petals(radius: f32, odd: bool, count: usize, opening: f32) -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for i in (if odd { 1 } else { 0 }..count).step_by(2) {
        let center = i as f32 * std::f32::consts::TAU / count as f32;
        let half = std::f32::consts::PI / count as f32;
        let base = vertices.len() as u32;
        for (z, r, taper, motion) in [
            (-0.80, 0.88, 0.89, 0.0),
            (-0.36, 0.94, 0.85, 0.55),
            (0.0, 1.02, 0.78, 1.0),
        ] {
            for sign in [-1.0, 1.0] {
                let a = center + sign * half * taper;
                let radial = radius * r * (1.0 + (opening - 1.0) * motion);
                vertices.push([radial * a.cos(), radial * a.sin(), z]);
            }
        }
        for j in 0..2 {
            let k = base + (j * 2) as u32;
            indices.extend([k, k + 1, k + 2, k + 1, k + 3, k + 2]);
        }
    }
    fin_mesh(vertices, indices)
}
fn engine_detail(
    c: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    root: Entity,
    vehicle: Vehicle,
    exit: f32,
    radius: f32,
) {
    let metal = materials.add(StandardMaterial {
        base_color: Color::srgb(0.19, 0.21, 0.24),
        metallic: 0.9,
        perceptual_roughness: 0.3,
        cull_mode: None,
        ..default()
    });
    let throat_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.025, 0.032, 0.044),
        metallic: 0.18,
        perceptual_roughness: 0.9,
        cull_mode: None,
        ..default()
    });
    let throat = c
        .spawn((
            Mesh3d(meshes.add(engine_surface(radius * 0.78, 0.77, false, 0))),
            MeshMaterial3d(throat_material),
            Transform::from_xyz(0.0, 0.0, exit - 0.385)
                .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
        ))
        .id();
    c.entity(root).add_child(throat);
    let backplate = c
        .spawn((
            Mesh3d(meshes.add(Circle::new(radius * 0.57))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.008, 0.012, 0.02),
                unlit: true,
                cull_mode: None,
                ..default()
            })),
            Transform::from_xyz(0.0, 0.0, exit - 0.55),
        ))
        .id();
    c.entity(root).add_child(backplate);
    let shroud = c
        .spawn((
            Mesh3d(meshes.add(Cylinder::new(radius * 0.82, 0.78))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.055, 0.065, 0.075),
                metallic: 0.42,
                perceptual_roughness: 0.72,
                cull_mode: None,
                ..default()
            })),
            Transform::from_xyz(0.0, 0.0, exit - 0.40)
                .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
        ))
        .id();
    c.entity(root).add_child(shroud);
    let face_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.28, 0.55),
        emissive: LinearRgba::BLACK,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    let glow = c
        .spawn((
            Mesh3d(meshes.add(Circle::new(radius * 0.53))),
            MeshMaterial3d(face_material),
            PointLight {
                color: Color::srgb(0.45, 0.7, 1.0),
                intensity: 0.0,
                range: 5.0,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, exit - 0.10),
            Visibility::Hidden,
            EngineGlow { vehicle },
        ))
        .id();
    c.entity(root).add_child(glow);
    for odd in [false, true] {
        let petal_material = materials.add(StandardMaterial {
            base_color: if odd {
                Color::srgb(0.20, 0.23, 0.27)
            } else {
                Color::srgb(0.31, 0.35, 0.39)
            },
            metallic: 0.84,
            perceptual_roughness: 0.39,
            cull_mode: None,
            ..default()
        });
        let count = if vehicle == Vehicle::Gripen { 20 } else { 24 };
        let e = c
            .spawn((
                Mesh3d(meshes.add(nozzle_petals(radius, odd, count, 0.82))),
                MeshMaterial3d(petal_material),
                Transform::from_xyz(0.0, 0.0, exit),
                NozzlePetals {
                    vehicle,
                    radius,
                    odd,
                    count,
                    last_opening: 0.82,
                },
            ))
            .id();
        c.entity(root).add_child(e);
    }
    for (z, r, tip_weight) in [
        (exit - 0.81, 0.89, 0.0),
        (exit - 0.20, 0.96, 0.72),
        (exit, 1.01, 1.0),
    ] {
        let ring = c
            .spawn((
                Mesh3d(meshes.add(Torus::new(radius * r - 0.012, radius * r + 0.012))),
                MeshMaterial3d(metal.clone()),
                Transform::from_xyz(0.0, 0.0, z)
                    .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))
                    .with_scale(Vec3::new(
                        1.0 + (0.82 - 1.0) * tip_weight,
                        1.0,
                        1.0 + (0.82 - 1.0) * tip_weight,
                    )),
                NozzleRing {
                    vehicle,
                    tip_weight,
                },
            ))
            .id();
        c.entity(root).add_child(ring);
    }
    for layer in 0..3 {
        let colors = [
            (0.55, 0.8, 1.0, 0.8),
            (0.15, 0.4, 1.0, 0.22),
            (1.0, 0.32, 0.07, 0.12),
        ];
        let (r, g, b, a) = colors[layer];
        let material = materials.add(StandardMaterial {
            base_color: Color::srgba(r, g, b, a),
            emissive: LinearRgba::new(r * 3.0, g * 3.0, b * 3.0, 1.0),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None,
            ..default()
        });
        let plume = c
            .spawn((
                Mesh3d(meshes.add(engine_surface(
                    radius * (0.45 + layer as f32 * 0.26),
                    6.0,
                    true,
                    layer,
                ))),
                MeshMaterial3d(material),
                Transform::from_xyz(0.0, 0.0, exit + 3.0)
                    .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                Visibility::Hidden,
                Exhaust {
                    vehicle,
                    layer,
                    exit,
                    last_expansion: 1.0,
                },
                bevy::light::NotShadowCaster,
                bevy::light::NotShadowReceiver,
                bevy::camera::visibility::NoFrustumCulling,
            ))
            .id();
        c.entity(root).add_child(plume);
    }
}
pub fn update_nozzle_geometry(
    f: Res<Flight>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut petals: Query<(&mut NozzlePetals, &Mesh3d)>,
    mut rings: Query<(&NozzleRing, &mut Transform)>,
) {
    for (mut petal, handle) in &mut petals {
        let opening = nozzle_opening(&f, petal.vehicle);
        if (opening - petal.last_opening).abs() > 0.0005 {
            if let Some(mesh) = meshes.get_mut(&handle.0) {
                *mesh = nozzle_petals(petal.radius, petal.odd, petal.count, opening);
                petal.last_opening = opening;
            }
        }
    }
    for (ring, mut transform) in &mut rings {
        let opening = nozzle_opening(&f, ring.vehicle);
        let radius_scale = 1.0 + (opening - 1.0) * ring.tip_weight;
        let scale=Vec3::new(radius_scale, 1.0, radius_scale);
        if transform.scale!=scale { transform.scale=scale; }
    }
}
pub fn update_engine_glow(
    f: Res<Flight>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut glows: Query<(
        &EngineGlow,
        &MeshMaterial3d<StandardMaterial>,
        &mut PointLight,
        &mut Visibility,
    )>,
) {
    for (glow, material, mut light, mut visibility) in &mut glows {
        let active = f.vehicle == glow.vehicle
            && f.engine
            && !f.crashed
            && f.throttle > 0.0
            && f.spool > 0.0
            && f.thrust > 0.0
            && (f.fuel > 0.0 || f.infinite_fuel);
        visibility.set_if_neq(if active {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if active {
            let power = f.spool.clamp(0.0, 1.0) as f32;
            let burner = ((power - 0.86) / 0.14).clamp(0.0, 1.0);
            let color = if glow.vehicle == Vehicle::Gripen && burner > 0.1 {
                Color::srgb(1.0, 0.44 + 0.12 * (1.0 - burner), 0.18)
            } else {
                Color::srgb(0.40, 0.70, 1.0)
            };
            let intensity=2200.0 * power * (1.0 + 2.5 * burner);
            if light.color!=color { light.color=color; }
            if light.intensity!=intensity { light.intensity=intensity; }
            let linear=color.to_linear();
            crate::render_cache::optics(&mut materials,&material.0,color,LinearRgba::new(
                    linear.red * 2.4 * power,
                    linear.green * 2.4 * power,
                    linear.blue * 2.4 * power,
                    1.0,
                ));
        } else {
            if light.intensity!=0. { light.intensity=0.; }
            crate::render_cache::optics(&mut materials,&material.0,Color::BLACK,LinearRgba::BLACK);
        }
    }
}

/// The canopy and its coaming share one rim; the outer coaming lands on
/// the exact boundary of the fuselage cutout rather than on a floating box.
pub fn canopy_point(z: f32, angle: f32) -> Vec3 {
    let (width, rise) = canopy_profile(z);
    let x = width * angle.cos();
    Vec3::new(x, upper_skin(x, z) + rise * angle.sin(), z)
}
/// Fitted to the C-series windscreen / bubble / rear shoulder proportions
/// shown in Saab's foldout. These are visual stations, not factory CAD.
fn canopy_profile(z: f32) -> (f32, f32) {
    const STATIONS: [(f32, f32, f32); 7] = [
        (-4.08, 0.12, 0.0),
        (-3.72, 0.32, 0.24),
        (-3.38, 0.45, 0.47),
        (-2.95, 0.50, 0.54),
        (-2.45, 0.48, 0.50),
        (-1.95, 0.40, 0.30),
        (-1.35, 0.22, 0.0),
    ];
    let i = STATIONS.windows(2).position(|p| z <= p[1].0).unwrap_or(5);
    let a = STATIONS[i];
    let b = STATIONS[i + 1];
    let span = b.0 - a.0;
    let t = ((z - a.0) / span).clamp(0.0, 1.0);
    let interpolate = |channel: usize| {
        let value = |s: (f32, f32, f32)| if channel == 1 { s.1 } else { s.2 };
        let slope = |j: usize| {
            if j == 0 || j == 6 {
                0.0
            } else {
                (value(STATIONS[j + 1]) - value(STATIONS[j - 1]))
                    / (STATIONS[j + 1].0 - STATIONS[j - 1].0)
            }
        };
        (2.0 * t.powi(3) - 3.0 * t * t + 1.0) * value(a)
            + (t.powi(3) - 2.0 * t * t + t) * span * slope(i)
            + (-2.0 * t.powi(3) + 3.0 * t * t) * value(b)
            + (t.powi(3) - t * t) * span * slope(i + 1)
    };
    (interpolate(1), interpolate(2).max(0.0))
}
fn cockpit_skin_edge(z: f32, side: f32) -> Vec3 {
    let s = FUSELAGE_SECTIONS.windows(2).find(|p| z <= p[1].0).unwrap();
    let t = ((z - s[0].0) / (s[1].0 - s[0].0)).clamp(0.0, 1.0);
    let w = s[0].1 + (s[1].1 - s[0].1) * t;
    let h = s[0].2 + (s[1].2 - s[0].2) * t;
    let cy = s[0].3 + (s[1].3 - s[0].3) * t;
    let x = 0.50_f32.min(w * (1.0 - ((0.18 - cy) / h).powi(2)).sqrt());
    Vec3::new(side * x, upper_skin(x, z), z)
}
fn canopy_mesh(coaming: bool) -> Mesh {
    if !coaming {
        // Shared grid vertices give the bubble continuous reflection normals.
        let mut p = Vec::new();
        let mut indices = Vec::new();
        for j in 0..=96 {
            let z = -4.08 + 2.73 * j as f32 / 96.0;
            for i in 0..=48 {
                p.push(canopy_point(z, std::f32::consts::PI * i as f32 / 48.0).to_array());
            }
        }
        for j in 0..96 {
            for i in 0..48 {
                let a = (j * 49 + i) as u32;
                indices.extend([a, a + 49, a + 50, a, a + 50, a + 1]);
            }
        }
        return fin_mesh(p, indices);
    }
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut quad = |p: [Vec3; 4]| {
        let base = vertices.len() as u32;
        vertices.extend(p.map(|v| v.to_array()));
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    };
    for j in 0..64 {
        let z = -4.08 + 2.73 * j as f32 / 64.0;
        let next = -4.08 + 2.73 * (j + 1) as f32 / 64.0;
        if coaming {
            for (side, angle) in [(1.0, 0.0), (-1.0, std::f32::consts::PI)] {
                let inner = angle + if side > 0.0 { 0.025 } else { -0.025 };
                // Retain the curved airframe shoulder outside the narrower glass.
                for k in 0..8 {
                    let t = k as f32 / 8.0;
                    let nt = (k + 1) as f32 / 8.0;
                    let shoulder = |z: f32, t: f32| {
                        let x =
                            cockpit_skin_edge(z, side).x * (1.0 - t) + canopy_point(z, angle).x * t;
                        Vec3::new(x, upper_skin(x, z), z)
                    };
                    quad([
                        shoulder(z, t),
                        shoulder(next, t),
                        shoulder(next, nt),
                        shoulder(z, nt),
                    ]);
                }
                quad([
                    canopy_point(z, angle),
                    canopy_point(next, angle),
                    canopy_point(next, inner),
                    canopy_point(z, inner),
                ]);
            }
        } else {
            for i in 0..32 {
                let a = std::f32::consts::PI * i as f32 / 32.0;
                let b = std::f32::consts::PI * (i + 1) as f32 / 32.0;
                quad([
                    canopy_point(z, a),
                    canopy_point(next, a),
                    canopy_point(next, b),
                    canopy_point(z, b),
                ]);
            }
        }
    }
    fin_mesh(vertices, indices)
}

// Elliptical cross sections give the airframe a continuous tapered silhouette.
fn fuselage_mesh(sections: &[(f32, f32, f32, f32)]) -> Mesh {
    let sides = 48usize;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut refined = Vec::new();
    for pair in sections.windows(2) {
        let count = ((pair[1].0 - pair[0].0).abs() / 0.12).ceil() as usize;
        for i in 0..count {
            let t = i as f32 / count as f32;
            refined.push((
                pair[0].0 + (pair[1].0 - pair[0].0) * t,
                pair[0].1 + (pair[1].1 - pair[0].1) * t,
                pair[0].2 + (pair[1].2 - pair[0].2) * t,
                pair[0].3 + (pair[1].3 - pair[0].3) * t,
            ));
        }
    }
    refined.push(*sections.last().unwrap());
    for &(z, width, height, center_y) in &refined {
        for i in 0..=sides {
            let a = i as f32 / sides as f32 * std::f32::consts::TAU;
            vertices.push([width * a.cos(), center_y + height * a.sin(), z]);
        }
    }
    for j in 0..refined.len() - 1 {
        for i in 0..sides {
            let a = (j * (sides + 1) + i) as u32;
            let b = a + (sides + 1) as u32;
            for tri in [[a, a + 1, b], [a + 1, b + 1, b]] {
                let p = (Vec3::from(vertices[tri[0] as usize])
                    + Vec3::from(vertices[tri[1] as usize])
                    + Vec3::from(vertices[tri[2] as usize]))
                    / 3.0;
                let _ = p;
                let triangle = tri.map(|i| Vec3::from(vertices[i as usize]));
                let mut pieces = vec![triangle.to_vec()];
                for (min, max) in [
                    (Vec3::new(-0.50, 0.18, -4.08), Vec3::new(0.50, 10.0, -1.35)),
                    (
                        Vec3::new(-0.455, -10.0, -5.02),
                        Vec3::new(0.455, 0.02, -3.10),
                    ),
                    (
                        Vec3::new(-0.805, -10.0, -0.50),
                        Vec3::new(-0.035, 0.10, 1.72),
                    ),
                    (Vec3::new(0.035, -10.0, -0.50), Vec3::new(0.805, 0.10, 1.72)),
                ] {
                    pieces = pieces
                        .into_iter()
                        .flat_map(|poly| subtract_box(poly, min, max))
                        .collect();
                }
                if pieces.len() == 1 && pieces[0] == triangle {
                    indices.extend(tri);
                } else {
                    for poly in pieces {
                        let base = vertices.len() as u32;
                        vertices.extend(poly.iter().map(|p| p.to_array()));
                        for i in 1..poly.len() - 1 {
                            indices.extend([base, base + i as u32, base + i as u32 + 1]);
                        }
                    }
                }
            }
        }
    }
    // Clipped triangles duplicate vertices at cut boundaries. Averaging only
    // their remaining faces produces bright seams; use the original shell's
    // analytic normal at every vertex, including newly clipped vertices.
    let normals: Vec<[f32; 3]> = vertices
        .iter()
        .map(|p| {
            let pair = sections
                .windows(2)
                .find(|s| p[2] <= s[1].0)
                .unwrap_or_else(|| &sections[sections.len() - 2..]);
            let dz = pair[1].0 - pair[0].0;
            let t = ((p[2] - pair[0].0) / dz).clamp(0.0, 1.0);
            let w = pair[0].1 + (pair[1].1 - pair[0].1) * t;
            let h = pair[0].2 + (pair[1].2 - pair[0].2) * t;
            let cy = pair[0].3 + (pair[1].3 - pair[0].3) * t;
            let dw = (pair[1].1 - pair[0].1) / dz;
            let dh = (pair[1].2 - pair[0].2) / dz;
            let dc = (pair[1].3 - pair[0].3) / dz;
            Vec3::new(
                p[0] / (w * w),
                (p[1] - cy) / (h * h),
                -p[0] * p[0] * dw / (w * w * w)
                    - (p[1] - cy) * dc / (h * h)
                    - (p[1] - cy).powi(2) * dh / (h * h * h),
            )
            .normalize()
            .to_array()
        })
        .collect();
    let mut mesh = fin_mesh(vertices, indices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh
}
/// Split triangles on exact bay/tub planes, retaining all outside fragments.
/// Centroid rejection leaves sawtooth openings and can remove whole skin strips.
pub fn wing_surface_y(x: f32, z: f32, upper: bool) -> f32 {
    let x = x.abs();
    let leading = -1.55 + (x - 0.65) * 3.75 / 3.5;
    let trailing = (2.32 + (x - 0.65) * 0.13).min(2.20 + (4.15 - x) * 0.55 / 0.19);
    let chord = (trailing - leading).max(0.005);
    let u = ((z - leading) / chord).clamp(0.0, 1.0);
    let ratio = 0.065 - 0.02 * (x - 0.65) / 3.5;
    let thickness = 5.0
        * ratio
        * chord
        * (0.2969 * u.sqrt() - 0.1260 * u - 0.3516 * u * u + 0.2843 * u.powi(3)
            - 0.1036 * u.powi(4));
    -0.12 - (x - 0.65) * 0.024 + if upper { thickness } else { -thickness }
}
fn wing_mesh(side: f32) -> Mesh {
    let mut p = Vec::new();
    let mut idx = Vec::new();
    let width = 33usize;
    let layer = 19 * width;
    for upper in [true, false] {
        for span in 0..19 {
            let x = 0.65 + (4.14 - 0.65) * span as f32 / 18.0;
            let leading = -1.55 + (x - 0.65) * 3.75 / 3.5;
            let trailing = (2.32 + (x - 0.65) * 0.13).min(2.20 + (4.15 - x) * 0.55 / 0.19);
            for chord in 0..width {
                let u = chord as f32 / 32.0;
                let z = leading + (trailing - leading) * u;
                p.push([side * x, wing_surface_y(x, z, upper), z]);
            }
        }
    }
    for upper in [true, false] {
        let base = if upper { 0 } else { layer };
        for s in 0..18 {
            for c in 0..32 {
                let a = (base + s * width + c) as u32;
                let mut tri = [
                    a,
                    a + 1,
                    a + width as u32,
                    a + 1,
                    a + width as u32 + 1,
                    a + width as u32,
                ];
                if upper != (side > 0.0) {
                    tri.swap(1, 2);
                    tri.swap(4, 5);
                }
                idx.extend(tri);
            }
        }
    }
    for span in [0, 18] {
        for c in 0..32 {
            let a = (span * width + c) as u32;
            let b = a + layer as u32;
            idx.extend([a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    fin_mesh(p, idx)
}
fn cut_main_bays(source: Mesh) -> Mesh {
    let bevy::mesh::VertexAttributeValues::Float32x3(points) =
        source.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
    else {
        unreachable!()
    };
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let source_indices: Vec<usize> = source.indices().unwrap().iter().collect();
    for triangle in source_indices.chunks_exact(3) {
        let mut pieces = vec![
            triangle
                .iter()
                .map(|i| Vec3::from(points[*i]))
                .collect::<Vec<_>>(),
        ];
        for b in &crate::gear::BAYS[1..] {
            let min = Vec3::new(b.xmin - 0.005, -10.0, b.front - 0.01);
            let max = Vec3::new(b.xmax + 0.005, b.roof, b.rear + 0.01);
            pieces = pieces
                .into_iter()
                .flat_map(|p| subtract_box(p, min, max))
                .collect();
        }
        for p in pieces {
            let base = vertices.len() as u32;
            vertices.extend(p.iter().map(|p| p.to_array()));
            for i in 1..p.len() - 1 {
                indices.extend([base, base + i as u32, base + i as u32 + 1]);
            }
        }
    }
    fin_mesh(vertices, indices)
}
pub(crate) fn subtract_box(mut inside: Vec<Vec3>, min: Vec3, max: Vec3) -> Vec<Vec<Vec3>> {
    let mut outside = Vec::new();
    for (axis, bound, sign) in [
        (0, min.x, 1.0),
        (0, max.x, -1.0),
        (1, min.y, 1.0),
        (1, max.y, -1.0),
        (2, min.z, 1.0),
        (2, max.z, -1.0),
    ] {
        if inside.len() < 3 {
            break;
        }
        let mut keep = Vec::new();
        let mut reject = Vec::new();
        for i in 0..inside.len() {
            let a = inside[i];
            let b = inside[(i + 1) % inside.len()];
            let da = (a[axis] - bound) * sign;
            let db = (b[axis] - bound) * sign;
            if da >= 0.0 {
                keep.push(a);
            } else {
                reject.push(a);
            }
            if (da >= 0.0) != (db >= 0.0) {
                let p = a.lerp(b, da / (da - db));
                keep.push(p);
                reject.push(p);
            }
        }
        if reject.len() >= 3 {
            outside.push(reject);
        }
        inside = keep;
    }
    outside
}
// Closed polygon extrusion makes edges visible from below and during deflection.
fn solid_panel(points: &[Vec3], thickness: Vec3) -> Mesh {
    let n = points.len();
    let mut vertices: Vec<[f32; 3]> = points.iter().map(|p| (*p + thickness).to_array()).collect();
    vertices.extend(points.iter().map(|p| (*p - thickness).to_array()));
    let mut indices = Vec::new();
    for i in 1..n - 1 {
        indices.extend([
            0,
            i as u32,
            (i + 1) as u32,
            n as u32,
            (n + i + 1) as u32,
            (n + i) as u32,
        ]);
    }
    for i in 0..n {
        let next = (i + 1) % n;
        indices.extend([
            i as u32,
            next as u32,
            (n + i) as u32,
            next as u32,
            (n + next) as u32,
            (n + i) as u32,
        ]);
    }
    fin_mesh(vertices, indices)
}
use crate::control_surfaces::SurfaceKind;
#[derive(Component)]
pub struct ControlSurface {
    kind: SurfaceKind,
    axis: Vec3,
    index: usize,
}
pub fn update_control_surfaces(
    f: Res<Flight>,
    mut surfaces: Query<(&ControlSurface, &mut Transform, &mut Visibility)>,
) {
    for (surface, mut transform, mut visibility) in &mut surfaces {
        visibility.set_if_neq(if f.crashed
            || f.damage.surface_state[surface.index]
                == crate::combat::damage::SurfaceState::Detached
        {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        });
        let rotation = Quat::from_axis_angle(
            surface.axis,
            crate::control_surfaces::effective_angle(
                &f.damage,
                surface.index,
                crate::control_surfaces::deflection(surface.kind, f.surface, f.flaps, f.airbrake),
            ) as f32,
        );
        if transform.rotation != rotation { transform.rotation = rotation; }
    }
}
fn aircraft_details(
    c: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    root: Entity,
    gray: &Handle<StandardMaterial>,
    dark: &Handle<StandardMaterial>,
) {
    let panel = materials.add(StandardMaterial {
        base_color: Color::srgb(0.31, 0.37, 0.40),
        metallic: 0.45,
        perceptual_roughness: 0.53,
        cull_mode: None,
        ..default()
    });
    let mut fixed = |mesh: Mesh, material: Handle<StandardMaterial>, transform: Transform| {
        let e = c
            .spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material),
                transform,
            ))
            .id();
        c.entity(root).add_child(e);
    };
    // Canopy sill and aft dorsal fairing.
    for side in [-1.0, 1.0] {
        fixed(
            Cuboid::new(0.055, 0.07, 2.6).into(),
            dark.clone(),
            Transform::from_xyz(side * 0.51, 0.51, -2.8),
        );
    }
    fixed(
        Capsule3d::new(0.24, 3.2).into(),
        gray.clone(),
        Transform::from_xyz(0.0, 0.55, 0.35)
            .with_scale(Vec3::new(1.0, 0.65, 1.0))
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
    );
    fixed(
        Cylinder::new(0.018, 0.7).into(),
        dark.clone(),
        Transform::from_xyz(0.0, 0.02, -7.8)
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
    );
    // Drop the fixed-part closure before spawning independently hinged surfaces.
    drop(fixed);
    let mut hinged =
        |points: &[Vec3], origin: Vec3, axis: Vec3, thickness: Vec3, kind: SurfaceKind| {
            let points: Vec<Vec3> = points.iter().map(|p| *p - origin).collect();
            let e = c
                .spawn((
                    Mesh3d(meshes.add(solid_panel(&points, thickness))),
                    MeshMaterial3d(panel.clone()),
                    Transform::from_translation(origin),
                    ControlSurface {
                        kind,
                        axis: axis.normalize(),
                        index: match kind {
                            SurfaceKind::Canard => {
                                if origin.x < 0.0 {
                                    0
                                } else {
                                    4
                                }
                            }
                            SurfaceKind::Airbrake => {
                                if origin.x < 0.0 {
                                    3
                                } else {
                                    7
                                }
                            }
                            _ => crate::control_surfaces::index(kind),
                        },
                    },
                    Fragile,
                    Visibility::Inherited,
                ))
                .id();
            c.entity(root).add_child(e);
        };
    for side in [-1.0, 1.0] {
        hinged(
            &[
                Vec3::new(side * 0.72, 0.3, -3.28),
                Vec3::new(side * 2.08, 0.35, -2.5),
                Vec3::new(side * 1.85, 0.35, -1.84),
                Vec3::new(side * 0.72, 0.3, -2.23),
            ],
            Vec3::new(side * 0.72, 0.3, -2.7),
            Vec3::X,
            Vec3::Y * 0.035,
            SurfaceKind::Canard,
        );
        for (inner, x0, x1) in [(true, 0.87, 2.28), (false, 2.32, 3.90)] {
            let hinge = |x: f32| 2.32 + (x - 0.65) * 0.13;
            let y = |x: f32| -0.12 - (x - 0.65) * 0.024;
            let origin = Vec3::new(side * x0, y(x0), hinge(x0));
            let end = Vec3::new(side * x1, y(x1), hinge(x1));
            // Orient both hinge axes toward +X for a consistent pitch convention.
            let axis = (end - origin) * side;
            hinged(
                &[
                    origin,
                    end,
                    Vec3::new(side * x1, y(x1), 3.25),
                    Vec3::new(side * x0, y(x0), 2.94 + (x0 - 0.87) * 0.1),
                ],
                origin,
                axis,
                Vec3::Y * 0.035,
                SurfaceKind::Elevon { side, inner },
            );
        }
        hinged(
            &[
                Vec3::new(side * 0.45, 0.46, 3.55),
                Vec3::new(side * 0.77, 0.32, 3.55),
                Vec3::new(side * 0.72, 0.32, 4.45),
                Vec3::new(side * 0.43, 0.46, 4.45),
            ],
            Vec3::new(side * 0.45, 0.46, 3.55),
            Vec3::X,
            Vec3::Y * 0.025,
            SurfaceKind::Airbrake,
        );
    }
    let bottom = Vec3::new(0.0, 0.52, 4.98);
    let top = Vec3::new(0.0, 2.94, 4.78);
    hinged(
        &[
            bottom,
            top,
            Vec3::new(0.0, 2.9, 5.42),
            Vec3::new(0.0, 0.52, 5.65),
        ],
        bottom,
        top - bottom,
        Vec3::X * 0.045,
        SurfaceKind::Rudder,
    );
}

#[cfg(test)]
mod visual_contract_tests {
    use super::*;
    #[test]
    fn geographic_east_is_screen_right_and_north_matches_spin_axis() {
        let pole = rotation().normalize();
        for (lon, lat) in [(0., 0.), (30., 20.), (12.711111, 58.428333), (120., -30.)] {
            let p = geo_to_local(lon, lat, 0.);
            let up = p.normalize();
            let east = pole.cross(up).normalize();
            let north = (pole - up * pole.dot(up)).normalize();
            assert!((geo_to_local(lon + 0.01, lat, 0.) - p).dot(east) > 0.);
            assert!((geo_to_local(lon, lat + 0.01, 0.) - p).dot(north) > 0.);
        }
        let camera = geo_to_local(30., 20., RADIUS * 2.).normalize();
        let screen_right = (-camera).cross(pole).normalize();
        assert!((geo_to_local(45., 24., 0.) - geo_to_local(20., 24., 0.)).dot(screen_right) > 0.);
        let mesh = globe_mesh(64, 32);
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!()
        };
        let Some(Indices::U32(indices)) = mesh.indices() else {
            panic!()
        };
        for tri in indices.chunks_exact(3).skip(128).step_by(97) {
            let a = Vec3::from(p[tri[0] as usize]);
            let b = Vec3::from(p[tri[1] as usize]);
            let c = Vec3::from(p[tri[2] as usize]);
            if (b - a).cross(c - a).length_squared() > 1. {
                assert!((b - a).cross(c - a).dot(a) > 0.);
            }
        }
    }
    #[test]
    fn cached_render_frame_preserves_exact_positions_and_vehicle_origin_changes() {
        for position in [
            surface_point(0., 0., 55.),
            surface_point(120000., -80000., 12000.),
            DVec3::Y * (RADIUS + 99999.),
            DVec3::Y * (RADIUS + 100001.),
            DVec3::Y * 1e12,
        ] {
            let mut f = Flight::spacecraft();
            for vehicle in [Vehicle::Gripen, Vehicle::Spacecraft, Vehicle::Gripen] {
                f.vehicle = vehicle;
                for dx in [0., 0.001, -0.001] {
                    f.position = position + DVec3::X * dx;
                    let expected = frame_uncached(&f);
                    let origin = if vehicle == Vehicle::Spacecraft && f.altitude() > 100000. {
                        f.position
                    } else {
                        f.up() * RADIUS
                    };
                    assert_eq!(
                        frame(&f).to_array().map(f64::to_bits),
                        expected.to_array().map(f64::to_bits)
                    );
                    assert_eq!(render_origin(&f), origin);
                    let point = f.position + DVec3::new(1.123, 0.043, -7.11);
                    assert_eq!(
                        local(point, &f),
                        (expected.conjugate() * (point - origin)).as_vec3()
                    );
                }
            }
        }
    }
    #[test]
    fn curved_wing_mesh_normals_remain_finite_after_bay_clipping() {
        for side in [-1.0, 1.0] {
            let m = cut_main_bays(wing_mesh(side));
            let bevy::mesh::VertexAttributeValues::Float32x3(normals) =
                m.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap()
            else {
                unreachable!()
            };
            assert!(normals.iter().flatten().all(|v| v.is_finite()));
        }
        assert!(wing_surface_y(1.0, 0.0, true) > wing_surface_y(1.0, 0.0, false) + 0.10);
    }
    #[test]
    fn wing_root_mesh_does_not_bridge_the_landing_gear_bay() {
        let source = solid_panel(
            &[
                Vec3::new(0.66, -0.12, 1.2),
                Vec3::new(0.78, -0.12, 1.2),
                Vec3::new(0.78, -0.12, 1.5),
                Vec3::new(0.66, -0.12, 1.5),
            ],
            Vec3::Y * 0.045,
        );
        let clipped = cut_main_bays(source);
        assert_eq!(clipped.indices().unwrap().len(), 0);
    }
    #[test]
    fn deep_space_origin_preserves_vehicle_detail_before_float_conversion() {
        for distance in [RADIUS + 220000.0, 1.0e8, 1.0e10, 1.0e12] {
            let mut f = Flight::spacecraft();
            f.position = DVec3::Y * distance;
            assert_eq!(local(f.position, &f), Vec3::ZERO);
            let delta = DVec3::new(0.003, 0.017, 0.004);
            let actual = local(f.position + delta, &f);
            let expected = (frame(&f).conjugate() * delta).as_vec3();
            assert!(
                actual.distance(expected) < 0.00015,
                "distance {distance} error {}",
                actual.distance(expected)
            );
        }
    }
    #[test]
    fn canopy_coaming_joins_glass_and_fuselage_without_gaps() {
        for z in [-4.08, -1.35] {
            for i in 0..=32 {
                let p = canopy_point(z, std::f32::consts::PI * i as f32 / 32.0);
                assert!(
                    (p.y - upper_skin(p.x, z)).abs() < 1e-5,
                    "canopy end must lie directly on the curved airframe"
                );
            }
        }
        let glass = canopy_mesh(false);
        let sill = canopy_mesh(true);
        let positions = |m: &Mesh| match m.attribute(Mesh::ATTRIBUTE_POSITION).unwrap() {
            bevy::mesh::VertexAttributeValues::Float32x3(p) => p.clone(),
            _ => panic!("position format"),
        };
        let glass_points = positions(&glass);
        let sill_points = positions(&sill);
        for j in 0..=32 {
            let z = -4.08 + 2.73 * j as f32 / 32.0;
            for (side, angle) in [(1.0, 0.0), (-1.0, std::f32::consts::PI)] {
                let rim = canopy_point(z, angle);
                for points in [&glass_points, &sill_points] {
                    assert!(points.iter().any(|p| Vec3::from(*p).distance(rim) < 1e-5));
                }
                let edge = cockpit_skin_edge(z, side);
                assert!((edge.y - upper_skin(edge.x, z)).abs() < 1e-5);
                assert!((edge.x.abs() - 0.50).abs() < 1e-5 || (edge.y - 0.18).abs() < 1e-5);
            }
        }
        for m in [&glass, &sill] {
            let bevy::mesh::VertexAttributeValues::Float32x3(normals) =
                m.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap()
            else {
                panic!("normal format")
            };
            assert!(normals.iter().all(|n| Vec3::from(*n).is_finite()));
        }
    }
    #[test]
    fn cockpit_cutouts_clip_edges_and_preserve_outside_skin() {
        let min = Vec3::new(-1.0, -1.0, -1.0);
        let max = Vec3::ONE;
        let inside = vec![Vec3::ZERO, Vec3::X * 0.2, Vec3::Y * 0.2];
        assert!(subtract_box(inside, min, max).is_empty());
        let triangle = vec![
            Vec3::new(-2.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
        ];
        let pieces = subtract_box(triangle, min, max);
        assert!(pieces.len() > 1);
        for piece in pieces {
            for p in piece {
                assert!(p.abs().max_element() >= 0.99999);
                assert!(p.is_finite());
            }
        }
    }
    #[test]
    fn vacuum_expansion_preserves_nozzle_exit_radius() {
        for radius in [0.72f32, 0.95, 1.65] {
            for layer in 0..3 {
                let mesh = crate::lunar::plume_mesh(radius, layer);
                let bevy::mesh::VertexAttributeValues::Float32x3(p) =
                    mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
                else {
                    panic!("plume positions")
                };
                let expected = radius * [0.38, 0.75, 1.][layer as usize];
                assert!((p[0][0].hypot(p[0][1]) - expected).abs() < 1e-5);
                let bevy::mesh::VertexAttributeValues::Float32x4(c) =
                    mesh.attribute(Mesh::ATTRIBUTE_COLOR).unwrap()
                else {
                    panic!("plume colors")
                };
                assert!(c[0][3] > 0. && c.last().unwrap()[3] == 0.);
                assert!(p.iter().all(|p| p.iter().all(|v| v.is_finite())));
            }
        }
    }
    fn scene() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(Assets::<Mesh>::default())
            .insert_resource(Assets::<StandardMaterial>::default())
            .insert_resource(Assets::<Image>::default())
            .insert_resource(Flight::runway())
            .insert_resource(Environment::default())
            .insert_resource(crate::ViewState::default())
            .add_systems(
                Startup,
                |mut c: Commands,
                 mut meshes: ResMut<Assets<Mesh>>,
                 mut materials: ResMut<Assets<StandardMaterial>>,
                 mut images: ResMut<Assets<Image>>| {
                    setup_aircraft(&mut c, &mut meshes, &mut materials, &mut images)
                },
            )
            .add_systems(
                Update,
                (
                    update_world,
                    update_nozzle_geometry,
                    update_engine_glow,
                    update_control_surfaces,
                )
                    .chain(),
            );
        app
    }
    #[test]
    fn exhaust_is_visible_below_afterburner_and_stops_without_thrust() {
        let mut app = scene();
        for power in [0.001, 0.05, 0.25, 0.8, 1.0] {
            {
                let mut f = app.world_mut().resource_mut::<Flight>();
                f.spool = power;
                f.throttle = power;
                f.thrust = power * 50000.0;
            }
            app.update();
            let mut query = app
                .world_mut()
                .query::<(&Exhaust, &Visibility, &Transform)>();
            let jet: Vec<_> = query
                .iter(app.world())
                .filter(|(e, _, _)| e.vehicle == Vehicle::Gripen)
                .collect();
            assert_eq!(jet.len(), 3);
            for (e, v, t) in jet {
                assert_eq!(*v, Visibility::Inherited);
                assert!(t.scale.y > 0.0);
                assert!((t.translation.z - 3.0 * t.scale.y - e.exit).abs() < 1e-5);
            }
        }
        app.world_mut().resource_mut::<Flight>().thrust = 0.0;
        app.update();
        let mut query = app.world_mut().query::<(&Exhaust, &Visibility)>();
        assert!(
            query
                .iter(app.world())
                .all(|(_, v)| *v == Visibility::Hidden)
        );
    }
    #[test]
    fn daytime_afterburner_is_brighter_without_changing_thrust() {
        let mut app = scene();
        {
            let mut f = app.world_mut().resource_mut::<Flight>();
            f.throttle = 1.0;
            f.spool = 1.0;
            f.thrust = 80500.0;
        }
        let mut samples = Vec::new();
        for phase in [3.0, -0.7] {
            app.world_mut().resource_mut::<Environment>().day_phase = phase;
            app.update();
            let mut query = app
                .world_mut()
                .query::<(&Exhaust, &MeshMaterial3d<StandardMaterial>)>();
            let (_, handle) = query
                .iter(app.world())
                .find(|(effect, _)| effect.vehicle == Vehicle::Gripen && effect.layer == 2)
                .unwrap();
            let mat = app
                .world()
                .resource::<Assets<StandardMaterial>>()
                .get(&handle.0)
                .unwrap();
            samples.push((mat.base_color.to_srgba().alpha, mat.emissive.red));
        }
        assert!(samples[1].0 > samples[0].0 * 2.0);
        assert!(samples[1].1 > samples[0].1 * 2.0);
        assert_eq!(app.world().resource::<Flight>().thrust, 80500.0);
    }
    #[test]
    fn nozzle_light_tracks_power_and_shuts_off() {
        let mut app = scene();
        for (power, engine) in [(0.25, true), (1.0, true), (1.0, false)] {
            {
                let mut f = app.world_mut().resource_mut::<Flight>();
                f.throttle = power;
                f.spool = power;
                f.thrust = 50000.0 * power;
                f.engine = engine;
            }
            app.update();
            let mut q = app
                .world_mut()
                .query::<(&EngineGlow, &PointLight, &Visibility)>();
            let jet = q
                .iter(app.world())
                .find(|(g, _, _)| g.vehicle == Vehicle::Gripen)
                .unwrap();
            assert_eq!(
                *jet.2,
                if engine {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                }
            );
            assert_eq!(jet.1.intensity > 0.0, engine);
        }
    }
    #[test]
    fn nozzle_petals_and_lip_expand_with_power_and_close_on_shutdown() {
        let mut app = scene();
        let mut previous_tip = 0.0;
        let mut fixed_root: Option<f32> = None;
        for (power, engine) in [(0.0, true), (0.5, true), (1.0, true), (1.0, false)] {
            {
                let mut f = app.world_mut().resource_mut::<Flight>();
                f.throttle = power;
                f.spool = power;
                f.engine = engine;
            }
            app.update();
            let expected = 0.82 + if engine { 0.30 * power as f32 } else { 0.0 };
            let mut petals = app.world_mut().query::<(&NozzlePetals, &Mesh3d)>();
            let (_, handle) = petals
                .iter(app.world())
                .find(|(p, _)| p.vehicle == Vehicle::Gripen && !p.odd)
                .unwrap();
            let mesh = app
                .world()
                .resource::<Assets<Mesh>>()
                .get(&handle.0)
                .unwrap();
            let bevy::mesh::VertexAttributeValues::Float32x3(vertices) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
            else {
                panic!("petal positions")
            };
            let radius_at = |index: usize| vertices[index][0].hypot(vertices[index][1]);
            let root = radius_at(0);
            let tip = radius_at(4);
            if let Some(initial) = fixed_root {
                assert!((root - initial).abs() < 1e-5);
            } else {
                fixed_root = Some(root);
            }
            assert!((tip - 0.55 * 1.02 * expected).abs() < 1e-5);
            if engine && power > 0.0 {
                assert!(tip > previous_tip);
            }
            if !engine {
                assert!(tip < previous_tip);
            }
            previous_tip = tip;
            let mut rings = app.world_mut().query::<(&NozzleRing, &Transform)>();
            let lip = rings
                .iter(app.world())
                .find(|(r, _)| r.vehicle == Vehicle::Gripen && r.tip_weight == 1.0)
                .unwrap();
            assert!((lip.1.scale.x - expected).abs() < 1e-5);
            assert!((lip.1.scale.z - expected).abs() < 1e-5);
        }
    }
    #[test]
    fn shutdown_clears_plume_with_stale_telemetry_and_restart_restores_it() {
        let mut app = scene();
        for vehicle in [Vehicle::Gripen, Vehicle::Spacecraft] {
            for (engine, throttle, visible) in [
                (true, 0.5, true),
                (false, 0.5, false),
                (true, 0.0, false),
                (true, 0.5, true),
            ] {
                {
                    let mut f = app.world_mut().resource_mut::<Flight>();
                    f.vehicle = vehicle;
                    f.engine = engine;
                    f.throttle = throttle;
                    // No physics step: reproduce shutdown while paused.
                    f.spool = 0.8;
                    f.thrust = 40000.0;
                }
                app.update();
                let mut query = app.world_mut().query::<(
                    &Exhaust,
                    &Visibility,
                    &Transform,
                    &MeshMaterial3d<StandardMaterial>,
                )>();
                for (effect, visibility, transform, material) in query
                    .iter(app.world())
                    .filter(|(e, _, _, _)| e.vehicle == vehicle)
                {
                    let _ = effect;
                    assert_eq!(
                        *visibility,
                        if visible {
                            Visibility::Inherited
                        } else {
                            Visibility::Hidden
                        }
                    );
                    if visible {
                        assert!(transform.scale.y > 0.0);
                    } else {
                        assert_eq!(transform.scale, Vec3::ZERO);
                        let mat = app
                            .world()
                            .resource::<Assets<StandardMaterial>>()
                            .get(&material.0)
                            .unwrap();
                        assert_eq!(mat.base_color, Color::NONE);
                        assert_eq!(mat.emissive, LinearRgba::BLACK);
                    }
                }
            }
        }
    }
    #[test]
    fn every_control_surface_uses_the_physics_deflection() {
        let mut app = scene();
        {
            let mut f = app.world_mut().resource_mut::<Flight>();
            f.surface = DVec3::new(0.4, -0.3, 0.6);
            f.airbrake = true;
        }
        app.update();
        let f = app.world().resource::<Flight>().clone();
        let mut query = app.world_mut().query::<(&ControlSurface, &Transform)>();
        let surfaces: Vec<_> = query.iter(app.world()).collect();
        assert_eq!(surfaces.len(), 9);
        for (s, t) in surfaces {
            let expected = Quat::from_axis_angle(
                s.axis,
                crate::control_surfaces::deflection(s.kind, f.surface, f.flaps, f.airbrake) as f32,
            );
            assert!(t.rotation.dot(expected).abs() > 0.99999);
        }
    }
    #[test]
    fn detached_and_jammed_meshes_keep_their_state_after_wreck_visibility_updates() {
        let mut app = scene();
        app.add_systems(Update, update_destruction.after(update_control_surfaces));
        {
            let mut f = app.world_mut().resource_mut::<Flight>();
            f.damage.surface_state[2] = crate::combat::damage::SurfaceState::Detached;
            f.damage.surface_state[4] = crate::combat::damage::SurfaceState::Jammed(0.31);
        }
        app.update();
        let mut query = app
            .world_mut()
            .query::<(&ControlSurface, &Visibility, &Transform)>();
        for (surface, visible, transform) in query.iter(app.world()) {
            if surface.index == 2 {
                assert_eq!(*visible, Visibility::Hidden);
            }
            if surface.index == 4 {
                assert_eq!(*visible, Visibility::Inherited);
                assert!(
                    transform
                        .rotation
                        .dot(Quat::from_axis_angle(surface.axis, 0.31))
                        .abs()
                        > 0.99999
                );
            }
        }
    }
}
