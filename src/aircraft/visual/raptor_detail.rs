//! F-22 presentation geometry. Authoritative panels, hinges and force laws stay
//! in aircraft::raptor; these meshes refine the skin around those mechanisms.
use super::*;

#[derive(Component)]
pub struct ExhaustFlap(usize, f32);
#[derive(Component)]
pub struct NozzleShoulder(usize, f32);
#[derive(Component)]
pub struct FixedNozzle;

const SHOULDER_LENGTH: f32 = 0.50;
const FLAP_LENGTH: f32 = 0.98;
const FLAP_TIP: f32 = FLAP_LENGTH + 0.36;
const FLAP_SKIN_OFFSET: f32 = 0.04;
fn shoulder_pose(f: &Flight, engine: usize, sign: f32) -> Transform {
    // Presentation linkage inferred from exterior references, not F119 actuator
    // calibration. The forward section and exit flap have different angles;
    // their shared hinge is computed once, so the skin cannot separate.
    Transform::from_xyz(0., sign * 0.40, -0.06).with_rotation(Quat::from_rotation_x(
        0.48 * f.systems.nozzle[engine] as f32 + sign * (0.10 - 0.12 * f.spool as f32),
    ))
}
fn flap_pose(f: &Flight, engine: usize, sign: f32) -> Transform {
    let shoulder = shoulder_pose(f, engine, sign);
    Transform::from_translation(shoulder.transform_point(Vec3::Z * SHOULDER_LENGTH)).with_rotation(
        Quat::from_rotation_x(
            f.systems.nozzle[engine] as f32 + sign * (0.12 - 0.22 * f.spool as f32),
        ),
    )
}
fn nozzle_metal(
    m: &mut Assets<StandardMaterial>,
    rgb: [f32; 3],
    rough: f32,
) -> Handle<StandardMaterial> {
    m.add(StandardMaterial {
        base_color: Color::srgb(rgb[0], rgb[1], rgb[2]),
        metallic: 0.96,
        perceptual_roughness: rough,
        reflectance: 0.55,
        double_sided: true,
        cull_mode: None,
        ..default()
    })
}
fn weathered_plate(mut mesh: Mesh, _sign: f32) -> Mesh {
    // Fine deterministic heat mottling across the broad plate, without raised
    // ribs or high-contrast stripes running through the exhaust passage.
    if let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    {
        let uv = p
            .iter()
            .map(|p| [(p[0] + 0.76) / 1.52, (p[2] + 0.36) / 1.84])
            .collect::<Vec<_>>();
        let colors = p
            .iter()
            .map(|p| {
                let wear = (p[0] * 31. + p[2] * 11.).sin() * (p[2] * 23. - p[0] * 17.).sin();
                let heat = (p[2] / 1.48).clamp(0., 1.);
                let gray = 0.88 + 0.05 * wear - 0.07 * heat;
                [gray * 0.99, gray, gray * 1.005, 1.]
            })
            .collect::<Vec<_>>();
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    }
    mesh
}

fn panel_texture() -> Image {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let mut image = Image::new_fill(
        Extent3d {
            width: 512,
            height: 512,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    let pixels = image.data.as_mut().unwrap();
    for y in 0..512u32 {
        for x in 0..512u32 {
            let h =
                (x.wrapping_mul(374761393) ^ y.wrapping_mul(668265263)).wrapping_mul(1274126177);
            let grain = ((h >> 24) as f32 / 255. - 0.5) * 0.025;
            let u = x as f32 / 512.;
            let v = y as f32 / 512.;
            let stain = (u * 17. + (v * 9.).sin()).sin() * (v * 13. - u * 5.).sin() * 0.035;
            let scuff = ((u * 177. + (v * 11.).sin() * 0.5).sin().abs()).powi(20) * 0.015;
            let gray = (0.88 + grain + stain + scuff - 0.06 * v).clamp(0.72, 0.96);
            let i = ((y * 512 + x) * 4) as usize;
            pixels[i..i + 4].copy_from_slice(&[
                (gray * 250.) as u8,
                (gray * 253.) as u8,
                (gray * 255.) as u8,
                255,
            ]);
        }
    }
    image
}

fn exit_outline() -> [Vec3; 5] {
    // Body +Z is aft: a rectangular forward section and triangular aft tip.
    // the triangle belongs to the silhouette, not a decal on a square slab.
    [
        Vec3::new(-0.76, 0., 0.),
        Vec3::new(0.76, 0., 0.),
        Vec3::new(0.76, 0., FLAP_LENGTH - 0.12),
        Vec3::new(0., 0., FLAP_TIP),
        Vec3::new(-0.76, 0., FLAP_LENGTH - 0.12),
    ]
}

fn flap_plate(sign: f32, length: f32) -> Mesh {
    let corners = [
        Vec3::new(-0.76, 0., 0.),
        Vec3::new(0.76, 0., 0.),
        Vec3::new(0.76, 0., length),
        Vec3::new(-0.76, 0., length),
    ];
    nozzle_panel(sign, &corners, 0.10)
}

fn nozzle_panel(sign: f32, outline: &[Vec3], depth: f32) -> Mesh {
    // Closed wedge: a shallow inner liner, narrow edge bevel, and broad
    // outer facets meeting at a raised spine. Depth grows away from the gas
    // passage, preserving its existing clearance and the pointed aft perimeter.
    // Proportions are inferred from photographs, not production F119 CAD.
    let mut points = Vec::new();
    let mut indices = Vec::new();
    let mut triangle = |mut p: [Vec3; 3]| {
        if (p[1] - p[0]).cross(p[2] - p[0]).length_squared() < 1e-12 {
            return;
        }
        // The input outline winds around the inner face; mirror both geometry
        // and winding for the lower assembly so all normals remain outward.
        if sign < 0. {
            p.swap(1, 2);
        }
        let n = points.len() as u32;
        points.extend(p.map(|p| Vec3::new(p.x, p.y * sign, p.z)));
        indices.extend([n, n + 1, n + 2]);
    };
    let section = |z: f32, width: f32, height: f32| {
        let bevel = if width == 0. { height } else { height * 0.45 };
        [
            (-width, 0.018),
            (-width * 0.90, bevel),
            (0., height),
            (width * 0.90, bevel),
            (width, 0.018),
            (width, -0.018),
            (-width, -0.018),
        ]
        .map(|(x, y)| Vec3::new(x, y, z))
    };
    // Keep the rectangular roof planar along its length. Otherwise the
    // triangulation introduces a diagonal highlight across each broad face.
    let front = section(0., 0.76, depth);
    let rear = section(outline[2].z, 0.76, depth);
    let mut sections = vec![front, rear];
    if outline.len() == 5 {
        // Collapse the roof onto a thin chevron tip, retaining the existing
        // rectangular base + aft triangle rather than a pyramid-like crown.
        sections.push(section(outline[3].z, 0., 0.018));
    }
    for pair in sections.windows(2) {
        for k in 0..7 {
            let j = (k + 1) % 7;
            triangle([pair[0][k], pair[1][k], pair[1][j]]);
            triangle([pair[0][k], pair[1][j], pair[0][j]]);
        }
    }
    for k in 0..7 {
        let j = (k + 1) % 7;
        triangle([Vec3::ZERO, front[k], front[j]]);
        if outline.len() == 4 {
            triangle([Vec3::Z * outline[2].z, rear[j], rear[k]]);
        }
    }
    weathered_plate(mesh(points, indices), sign)
}
#[derive(Component)]
pub struct GlowLayer(
    pub(super) usize,
    pub(super) usize,
    pub(super) Handle<StandardMaterial>,
);
#[derive(Component)]
pub struct ShockCell(usize, usize, Handle<StandardMaterial>);
#[derive(Component)]
pub struct GearLink(usize, usize);
#[derive(Component)]
pub struct Wheel {
    index: usize,
    angle: f64,
    speed: f64,
    last_time: f64,
}
impl Wheel {
    fn advance(&mut self, f: &Flight) {
        if f.elapsed < self.last_time {
            self.angle = 0.;
            self.speed = 0.;
        }
        let dt = (f.elapsed - self.last_time).max(0.);
        self.last_time = f.elapsed;
        if f.gear_compression[self.index] > 0.001 {
            self.speed = f.velocity.length() / if self.index == 0 { 0.29 } else { 0.34 };
        } else {
            self.speed *= (-0.4 * dt).exp();
        }
        self.angle = (self.angle + self.speed * dt).rem_euclid(std::f64::consts::TAU);
    }
}
#[derive(Component)]
pub struct Fork(usize);
#[derive(Component)]
pub struct GearJoint(usize, usize);
#[derive(Component)]
pub struct GearDoor(usize, f32, Vec3, bool);

pub(super) fn engine_origin(physical: DVec3) -> DVec3 {
    physical - DVec3::Z * 0.95
}
pub(super) const EXHAUST_START: f32 = -0.20;

fn burner_face() -> Mesh {
    let mut v = Vec::new();
    let mut ix = Vec::new();
    let mut color = Vec::new();
    for k in 0..48 {
        let at = |k: usize| {
            let t = k as f32 / 48. * std::f32::consts::TAU;
            Vec3::new(
                t.cos().signum() * t.cos().abs().powf(0.38) * 0.62,
                t.sin().signum() * t.sin().abs().powf(0.46) * 0.265,
                -0.285,
            )
        };
        let first = v.len() as u32;
        v.extend([Vec3::new(0., 0., -0.285), at(k), at(k + 1)]);
        ix.extend(first..first + 3);
        color.extend([
            [1., 0.8, 0.5, 0.8],
            [0.55, 0.30, 0.18, 0.12],
            [0.55, 0.30, 0.18, 0.12],
        ]);
    }
    let mut m = mesh(v, ix);
    m.insert_attribute(Mesh::ATTRIBUTE_COLOR, color);
    m
}

pub(super) fn wing(s: f32) -> Vec<Vec3> {
    [
        (1.90, -0.75),
        (6.78, 3.35),
        (6.78, 4.50),
        (5.87, 5.39),
        (1.50, 6.55),
    ]
    .map(|(x, z)| Vec3::new(s * x, -0.04, z))
    .to_vec()
}
pub(super) fn panels() -> [Panel; SURFACES] {
    let mut p = Aircraft::F22.panels();
    for (s, b) in [(-1., 0), (1., 4)] {
        p[b].outline = Some(
            [
                (1.60, 0.02, 6.35),
                (4.35, 0.02, 7.85),
                (4.35, 0.02, 9.10),
                (1.60, 0.02, 8.15),
            ]
            .map(|(x, y, z)| DVec3::new(s * x, y, z)),
        );
        p[b].center = DVec3::new(s * 2.65, 0.02, 7.65);
        p[b + 2].outline = Some(
            [
                (3.97, -0.04, 4.90),
                (6.45, -0.04, 4.55),
                (5.87, -0.04, 5.39),
                (3.97, -0.04, 5.89),
            ]
            .map(|(x, y, z)| DVec3::new(s * x, y, z)),
        );
        p[b + 1].outline = Some(
            [
                (1.65, -0.04, 5.15),
                (3.90, -0.04, 4.91),
                (3.90, -0.04, 5.91),
                (1.65, -0.04, 6.51),
            ]
            .map(|(x, y, z)| DVec3::new(s * x, y, z)),
        );
        p[b + 3].outline = Some(
            [
                (1.60, 0.44, 5.35),
                (3.03, 3.05, 4.95),
                (3.03, 3.05, 5.65),
                (1.60, 0.44, 6.34),
            ]
            .map(|(x, y, z)| DVec3::new(s * x, y, z)),
        );
        let le = if b == 0 { 9 } else { 10 };
        p[le].outline = Some(
            [
                (1.90, -0.04, -0.75),
                (6.78, -0.04, 3.35),
                (6.78, -0.04, 3.66),
                (1.90, -0.04, -0.28),
            ]
            .map(|(x, y, z)| DVec3::new(s * x, y, z)),
        );
        for i in [b + 1, b + 2, b + 3, le] {
            let q = p[i].outline.unwrap();
            p[i].center = if i == le {
                (q[2] + q[3]) * 0.5
            } else {
                (q[0] + q[1]) * 0.5
            };
        }
    }
    p
}
pub(super) fn surface_rotation(i: usize, angle: f64) -> Quat {
    let p = panels()[i];
    let mut axis = p.axis();
    if i == 9 || i == 10 {
        let q = p.outline.unwrap();
        axis = (q[2] - q[3]).normalize();
        if axis.x < 0. {
            axis = -axis;
        }
    }
    bevy::math::DQuat::from_axis_angle(axis, angle * if p.normal.x.abs() < 0.5 { -1. } else { 1. })
        .as_quat()
}
pub(super) fn surface_polygon(i: usize) -> (Vec<Vec3>, Vec3) {
    if i == 0 || i == 4 {
        let s = if i == 0 { -1. } else { 1. };
        return (
            [
                (1.60, 6.60),
                (3.0, 6.35),
                (4.35, 7.85),
                (4.35, 9.10),
                (3.0, 9.46),
                (1.60, 8.15),
            ]
            .map(|(x, z)| Vec3::new(s * x, 0.02, z))
            .to_vec(),
            Vec3::Y * 0.045,
        );
    }
    let (p, t) = panel_polygon(panels()[i]);
    (p.to_vec(), t)
}
pub(super) fn fin(s: f32) -> Vec<Vec3> {
    [
        (1.56, 0.36, 2.50),
        (1.65, 0.55, 2.68),
        (2.90, 2.82, 3.86),
        (3.03, 3.05, 4.18),
        (3.03, 3.05, 4.95),
        (1.60, 0.44, 5.35),
    ]
    .map(|(x, y, z)| Vec3::new(s * x, y, z))
    .to_vec()
}
pub(super) fn canopy_point(u: f32, t: f32) -> Vec3 {
    let shape = (u * std::f32::consts::PI).sin().max(0.);
    let width = 0.53 * shape.powf(0.58);
    let z = Aircraft::F22.cabin_offset().z - 4.30 + 3.20 * u;
    let base = skin_y(width, z, true) + 0.015;
    Vec3::new(width * t.cos(), base + 0.77 * shape.powf(0.78) * t.sin(), z)
}
fn skin_y(x: f32, z: f32, top: bool) -> f32 {
    let s = super::super::raptor::sections();
    let p = s
        .windows(2)
        .find(|p| z as f64 >= p[0].0 && z as f64 <= p[1].0)
        .unwrap();
    let t = ((z as f64 - p[0].0) / (p[1].0 - p[0].0)) as f32;
    let w = p[0].1 as f32 * (1. - t) + p[1].1 as f32 * t;
    let h = p[0].2 as f32 * (1. - t) + p[1].2 as f32 * t;
    let y = p[0].3 as f32 * (1. - t) + p[1].3 as f32 * t;
    y + h * if top {
        raptor_mesh::upper(x / w)
    } else {
        -0.83 * (1. - (x / w).powi(4)).max(0.).sqrt()
    }
}
pub(super) fn sill() -> Mesh {
    let mut v = Vec::new();
    let mut ix = Vec::new();
    for s in [-1., 1.] {
        for j in 0..64 {
            let at = |j: usize, outer: bool| {
                let u = j as f32 / 64.;
                let mut p = canopy_point(u, if s > 0. { 0. } else { std::f32::consts::PI });
                if outer {
                    p.x += s * 0.22 * (u * std::f32::consts::PI).sin();
                    p.y = skin_y(p.x, p.z, true) + 0.004;
                }
                p
            };
            quad(
                &mut v,
                &mut ix,
                [at(j, false), at(j + 1, false), at(j + 1, true), at(j, true)],
            );
        }
    }
    raptor_mesh::paint(raptor_mesh::smooth(mesh(v, ix)))
}
pub(super) fn cut_cockpit(points: Vec<Vec3>) -> Vec<Vec<Vec3>> {
    fn clip(p: &[Vec3], n: Vec3, d: f32, inside: bool) -> Vec<Vec3> {
        let mut out = Vec::new();
        if p.is_empty() {
            return out;
        }
        let sign = if inside { 1. } else { -1. };
        let mut a = *p.last().unwrap();
        let mut da = (a.dot(n) - d) * sign;
        for &b in p {
            let db = (b.dot(n) - d) * sign;
            if (da >= 0.) != (db >= 0.) {
                out.push(a + (b - a) * (da / (da - db)));
            }
            if db >= 0. {
                out.push(b);
            }
            a = b;
            da = db;
        }
        out
    }
    let mut ring = Vec::new();
    for j in 0..=24 {
        ring.push(canopy_point(j as f32 / 24., 0.));
    }
    for j in (1..24).rev() {
        ring.push(canopy_point(j as f32 / 24., std::f32::consts::PI));
    }
    let center = ring.iter().copied().sum::<Vec3>() / ring.len() as f32;
    // Hollow the cabin down to its lined floor. A roof-only cut left airframe
    // skin through the seat and lower instrument/radar faces.
    let mut planes = vec![(Vec3::Y, -0.30)];
    for j in 0..ring.len() {
        let a = ring[j];
        let b = ring[(j + 1) % ring.len()];
        let edge = b - a;
        let mut n = Vec3::new(-edge.z, 0., edge.x).normalize_or_zero();
        if (center - a).dot(n) < 0. {
            n = -n;
        }
        planes.push((n, a.dot(n)));
    }
    let mut inside = points;
    let mut result = Vec::new();
    for (n, d) in planes {
        if inside.len() < 3 {
            break;
        }
        let outside = clip(&inside, n, d, false);
        if outside.len() >= 3 {
            result.push(outside);
        }
        inside = clip(&inside, n, d, true);
    }
    result
}
pub(super) fn canopy_frame(
    c: &mut Commands,
    m: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    root: Entity,
    skin: Handle<StandardMaterial>,
) {
    let seal = material(mats, Color::srgb(0.08, 0.085, 0.08), 0.05);
    let mut collar = Vec::new();
    let mut indices = Vec::new();
    for j in 0..12 {
        for k in 0..32 {
            let at = |j: usize, k: usize| {
                let u = 0.88 + 0.12 * j as f32 / 12.;
                canopy_point(u, k as f32 / 32. * std::f32::consts::PI) + Vec3::Y * 0.012
            };
            quad(
                &mut collar,
                &mut indices,
                [at(j, k), at(j + 1, k), at(j + 1, k + 1), at(j, k + 1)],
            );
        }
    }
    part(
        c,
        m,
        root,
        raptor_mesh::paint(raptor_mesh::smooth(mesh(collar, indices))),
        skin.clone(),
        Transform::default(),
    );
    // A narrow gasket follows the actual glass boundary, rather than a tall box
    // around the cockpit. Both ends converge continuously onto the dorsal skin.
    for t in [0., std::f32::consts::PI] {
        for j in 0..64 {
            let rail=tube(
                c,
                m,
                root,
                skin.clone(),
                canopy_point(j as f32 / 64., t),
                canopy_point((j + 1) as f32 / 64., t),
                0.026,
            );
            c.entity(rail).insert(crate::render_batch::RigidGeometry);
            let gasket=tube(
                c,
                m,
                root,
                seal.clone(),
                canopy_point(j as f32 / 64., t) + Vec3::Y * 0.012,
                canopy_point((j + 1) as f32 / 64., t) + Vec3::Y * 0.012,
                0.007,
            );
            c.entity(gasket).insert(crate::render_batch::RigidGeometry);
        }
    }
}

fn metal(m: &mut Assets<StandardMaterial>, rgb: [f32; 3], rough: f32) -> Handle<StandardMaterial> {
    m.add(StandardMaterial {
        base_color: Color::srgb(rgb[0], rgb[1], rgb[2]),
        metallic: 0.78,
        perceptual_roughness: rough,
        double_sided: true,
        cull_mode: None,
        ..default()
    })
}
fn batch(
    c: &mut Commands,
    m: &mut Assets<Mesh>,
    root: Entity,
    pieces: Vec<Mesh>,
    mat: Handle<StandardMaterial>,
) {
    let mut iter = pieces.into_iter();
    if let Some(mut mesh) = iter.next() {
        for p in iter {
            mesh.merge(&p).expect("compatible detail attributes");
        }
        part(c, m, root, mesh, mat, Transform::default());
    }
}
pub(super) fn border(points: &[Vec3], normal: Vec3) -> Mesh {
    let mut v = Vec::new();
    let mut ix = Vec::new();
    let mid = points.iter().copied().sum::<Vec3>() / points.len() as f32;
    for j in 0..points.len() {
        let k = (j + 1) % points.len();
        let a = points[j] + normal * 0.048;
        let b = points[k] + normal * 0.048;
        let ai = a + (mid - points[j]).normalize_or_zero() * 0.075;
        let bi = b + (mid - points[k]).normalize_or_zero() * 0.075;
        quad(&mut v, &mut ix, [a, b, bi, ai]);
    }
    mesh(v, ix)
}
pub(super) fn dorsal_details(
    c: &mut Commands,
    m: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    root: Entity,
) {
    let seam = material(mats, Color::srgb(0.30, 0.32, 0.33), 0.25);
    let mut v = Vec::new();
    let mut ix = Vec::new();
    for s in [-1., 1.] {
        for (cx, z, w, l) in [
            (0.84, -3.0, 0.34, 0.70),
            (1.05, -1.0, 0.50, 1.10),
            (0.46, 0.55, 0.42, 0.65),
            (1.05, 2.0, 0.58, 0.8),
            (0.95, 4.6, 0.55, 1.2),
        ] {
            let corners = [
                (-w * 0.5, -l * 0.35),
                (-w * 0.25, -l * 0.5),
                (w * 0.4, -l * 0.5),
                (w * 0.5, -l * 0.25),
                (w * 0.5, l * 0.4),
                (w * 0.3, l * 0.5),
                (-w * 0.5, l * 0.5),
            ];
            for k in 0..corners.len() {
                let j = (k + 1) % corners.len();
                let a = Vec2::new(s * (cx + corners[k].0), z + corners[k].1);
                let b = Vec2::new(s * (cx + corners[j].0), z + corners[j].1);
                let d = (b - a).normalize();
                let n = Vec2::new(-d.y, d.x) * 0.0035;
                let at = |p: Vec2| Vec3::new(p.x, skin_y(p.x, p.y, true) + 0.006, p.y);
                quad(
                    &mut v,
                    &mut ix,
                    [at(a - n), at(b - n), at(b + n), at(a + n)],
                );
            }
        }
    }
    part(c, m, root, mesh(v, ix), seam, Transform::default());
    let stencil = material(mats, Color::srgb(0.53, 0.55, 0.55), 0.15);
    let segments = [
        (Vec2::new(0., 1.), Vec2::new(0.6, 1.)),
        (Vec2::new(0.6, 1.), Vec2::new(0.6, 0.5)),
        (Vec2::new(0.6, 0.5), Vec2::new(0.6, 0.)),
        (Vec2::new(0., 0.), Vec2::new(0.6, 0.)),
        (Vec2::new(0., 0.5), Vec2::new(0., 0.)),
        (Vec2::new(0., 1.), Vec2::new(0., 0.5)),
        (Vec2::new(0., 0.5), Vec2::new(0.6, 0.5)),
    ];
    let masks = [0b0111111u8, 0b1111111, 0b0000110, 0b1101101, 0b1011011];
    let mut v = Vec::new();
    let mut ix = Vec::new();
    for s in [-1., 1.] {
        let up = Vec3::new(s * 0.4848, 0.8746, 0.);
        let across = -Vec3::Z * s;
        let base = Vec3::new(s * 2.35, 1.80, if s > 0. { 4.66 } else { 3.96 })
            + Vec3::new(s * 0.028, -0.015, 0.);
        for (letter, mask) in masks.iter().enumerate() {
            for (bit, (a, b)) in segments.iter().enumerate() {
                if mask & (1 << bit) == 0 {
                    continue;
                }
                let a = base + across * (letter as f32 * 0.14 + a.x * 0.17) + up * a.y * 0.17;
                let b = base + across * (letter as f32 * 0.14 + b.x * 0.17) + up * b.y * 0.17;
                let d = (b - a).normalize();
                let n = d.cross(up.cross(across)).normalize() * 0.005;
                quad(&mut v, &mut ix, [a - n, b - n, b + n, a + n]);
            }
        }
    }
    part(c, m, root, mesh(v, ix), stencil, Transform::default());
}
fn shell(rings: &[(f32, f32, f32)], facets: usize) -> Mesh {
    let mut p = Vec::new();
    let mut ix = Vec::new();
    let at = |j: usize, k: usize| {
        let (z, w, h) = rings[j];
        let t = k as f32 / facets as f32 * std::f32::consts::TAU;
        Vec3::new(
            t.cos().signum() * t.cos().abs().powf(0.38) * w,
            t.sin().signum() * t.sin().abs().powf(0.46) * h,
            z,
        )
    };
    for j in 0..rings.len() - 1 {
        for k in 0..facets {
            quad(
                &mut p,
                &mut ix,
                [at(j, k), at(j, k + 1), at(j + 1, k + 1), at(j + 1, k)],
            );
        }
    }
    mesh(p, ix)
}
fn fairing_point(engine: usize, u: f32, theta: f32) -> Vec3 {
    let origin = engine_origin(Aircraft::F22.engine_positions()[engine]).as_vec3();
    let blend = u * u * u * (u * (u * 6. - 15.) + 10.);
    let z = -1.60 + 1.52 * u;
    let x = theta.cos().signum() * theta.cos().abs().powf(0.38) * (0.84 - 0.03 * blend);
    let vertical = theta.sin().abs().powf(0.46);
    let top = skin_y(origin.x + x, origin.z + z, true) - 0.006;
    let bottom = skin_y(origin.x + x, origin.z + z, false) + 0.006;
    let center = (top + bottom) * 0.5;
    let skin = center
        + (if theta.sin() >= 0. {
            top - center
        } else {
            bottom - center
        }) * vertical
        - origin.y;
    let exit = theta.sin().signum() * vertical * 0.40;
    Vec3::new(x, skin * (1. - blend) + exit * blend, z)
}
fn nozzle_fairing(engine: usize) -> Mesh {
    // Bury the forward rim just inside the actual fuselage skin, then loft
    // smoothly into the unchanged fixed nozzle throat. No raised front collar.
    let mut points = Vec::new();
    let mut indices = Vec::new();
    for j in 0..24 {
        for k in 0..32 {
            let at = |j: usize, k: usize| {
                fairing_point(
                    engine,
                    j as f32 / 24.,
                    k as f32 / 32. * std::f32::consts::TAU,
                )
            };
            quad(
                &mut points,
                &mut indices,
                [at(j, k), at(j, k + 1), at(j + 1, k + 1), at(j + 1, k)],
            );
        }
    }
    weathered_plate(raptor_mesh::smooth(mesh(points, indices)), 1.)
}
fn shock_mesh() -> Mesh {
    let mut v = Vec::new();
    let mut ix = Vec::new();
    let mut colors = Vec::new();
    for (radius, opacity) in [(0.45, 0.55), (0.70, 0.30), (1., 0.10)] {
        for j in 0..8 {
            for k in 0..24 {
                let at = |j: usize, k: usize| {
                    let u = j as f32 / 8.;
                    let r = (1. - (2. * u - 1.).abs()) * radius;
                    let t = k as f32 / 24. * std::f32::consts::TAU;
                    Vec3::new(r * t.cos(), r * t.sin(), u - 0.5)
                };
                quad(
                    &mut v,
                    &mut ix,
                    [at(j, k), at(j + 1, k), at(j + 1, k + 1), at(j, k + 1)],
                );
                for u in [
                    j as f32 / 8.,
                    (j + 1) as f32 / 8.,
                    (j + 1) as f32 / 8.,
                    j as f32 / 8.,
                ] {
                    colors.push([
                        1.,
                        1.,
                        1.,
                        opacity * (u * std::f32::consts::PI).sin().max(0.).powf(0.7),
                    ]);
                }
            }
        }
    }
    let mut m = mesh(v, ix);
    m.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    m
}
fn exhaust_power(f: &Flight, i: usize) -> f32 {
    if !f.engine
        || f.thrust <= 1.
        || !f.systems.engine_on[i]
        || f.systems.engines[i] <= 0.1
        || (!f.infinite_fuel && f.fuel <= 0.)
    {
        0.
    } else {
        f.spool.clamp(0., 1.) as f32
    }
}
pub(super) fn nozzle(
    c: &mut Commands,
    m: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    root: Entity,
    nozzle: Entity,
    engine: usize,
    at: Vec3,
    skin: Handle<StandardMaterial>,
) {
    let texture = images.add(panel_texture());
    let thermal = mats.add(StandardMaterial {
        base_color: Color::srgb(0.65, 0.67, 0.68),
        base_color_texture: Some(texture),
        metallic: 0.52,
        perceptual_roughness: 0.55,
        reflectance: 0.48,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    let dark = nozzle_metal(mats, [0.13, 0.14, 0.15], 0.46);
    let edge = nozzle_metal(mats, [0.42, 0.43, 0.44], 0.25);
    // Painted housings share the fuselage material and body-space camouflage.
    // The moving hot flaps retain a quieter, cool-gray heat-worn metal finish.
    let paint_origin = engine_origin(Aircraft::F22.engine_positions()[engine]).as_vec3();
    // This frame never vectors. Only the two flap assemblies and flow do.
    let fixed = c
        .spawn((
            FixedNozzle,
            Transform::from_translation(at),
            Visibility::Inherited,
        ))
        .id();
    c.entity(root).add_child(fixed);
    let black = mats.add(StandardMaterial {
        base_color: Color::srgb(0.004, 0.005, 0.006),
        unlit: true,
        ..default()
    });
    let fire = mats.add(StandardMaterial {
        base_color: Color::linear_rgba(0., 0., 0., 0.),
        alpha_mode: AlphaMode::Add,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    let burner = part(
        c,
        m,
        fixed,
        burner_face(),
        fire.clone(),
        Transform::default(),
    );
    c.entity(burner)
        .insert((GlowLayer(engine, 7, fire), bevy::light::NotShadowCaster));
    let shock = m.add(shock_mesh());
    for cell in 0..6 {
        let mat = mats.add(StandardMaterial {
            base_color: Color::srgba(0.85, 0.91, 1., 0.),
            emissive: LinearRgba::BLACK,
            alpha_mode: AlphaMode::Add,
            unlit: true,
            cull_mode: None,
            ..default()
        });
        let e = c
            .spawn((
                Mesh3d(shock.clone()),
                MeshMaterial3d(mat.clone()),
                ShockCell(engine, cell, mat),
                Transform::default(),
                Visibility::Hidden,
                bevy::light::NotShadowCaster,
            ))
            .id();
        c.entity(nozzle).add_child(e);
    }
    // The fixed boat-tail follows the fuselage at its forward end and smoothly
    // transitions to the nozzle. Only the inner flaps and flow vector.
    part(
        c,
        m,
        fixed,
        raptor_mesh::paint_at(nozzle_fairing(engine), paint_origin),
        skin.clone(),
        Transform::default(),
    );
    part(
        c,
        m,
        fixed,
        shell(&[(-0.40, 0.69, 0.34), (0.05, 0.72, 0.34)], 24),
        dark.clone(),
        Transform::default(),
    );
    // The liner is a continuous surface; the old repeated bars looked like
    // bands obstructing the nozzle. Keep flame holders deep in the duct.
    for x in [-0.45, -0.225, 0., 0.225, 0.45] {
        tube(
            c,
            m,
            fixed,
            dark.clone(),
            Vec3::new(x, -0.25, -0.24),
            Vec3::new(x, 0.25, -0.24),
            0.012,
        );
    }
    part(
        c,
        m,
        fixed,
        solid(
            &[
                Vec3::new(-0.68, -0.31, -0.32),
                Vec3::new(0.68, -0.31, -0.32),
                Vec3::new(0.68, 0.31, -0.32),
                Vec3::new(-0.68, 0.31, -0.32),
            ],
            Vec3::Z * 0.005,
        ),
        black,
        Transform::default(),
    );
    for side in [-1., 1.] {
        // Angular fixed side cheeks and layered seal plates, not a box endcap.
        let cheek = [
            Vec3::new(side * 0.80, -0.40, -0.30),
            Vec3::new(side * 0.80, 0.40, -0.30),
            Vec3::new(side * 0.80, 0.40, 0.05),
            Vec3::new(side * 0.80, 0.22, 1.39),
            Vec3::new(side * 0.80, 0., 1.48),
            Vec3::new(side * 0.80, -0.22, 1.39),
            Vec3::new(side * 0.80, -0.40, 0.05),
        ];
        part(
            c,
            m,
            fixed,
            raptor_mesh::paint_at(solid(&cheek, Vec3::X * 0.018), paint_origin),
            skin.clone(),
            Transform::default(),
        );
        let facet = [
            Vec3::new(side * 0.825, 0., -0.20),
            Vec3::new(side * 0.825, 0.36, 0.07),
            Vec3::new(side * 0.825, 0., 1.43),
            Vec3::new(side * 0.825, -0.36, 0.07),
        ];
        part(
            c,
            m,
            fixed,
            raptor_mesh::paint_at(solid(&facet, Vec3::X * 0.012), paint_origin),
            skin.clone(),
            Transform::default(),
        );
    }
    for sign in [-1., 1.] {
        let shoulder = c
            .spawn((
                NozzleShoulder(engine, sign),
                Transform::default(),
                Visibility::Inherited,
            ))
            .id();
        c.entity(fixed).add_child(shoulder);
        part(
            c,
            m,
            shoulder,
            flap_plate(sign, SHOULDER_LENGTH),
            thermal.clone(),
            Transform::default(),
        );
        // The triangular extension belongs to the aft panel's perimeter.
        // Its slight exterior lap clears the separately moving backing plate.
        let pivot = Vec3::new(0., sign * 0.35, 0.02);
        let flap = c
            .spawn((
                ExhaustFlap(engine, sign),
                Transform::from_translation(pivot),
                Visibility::Inherited,
            ))
            .id();
        c.entity(fixed).add_child(flap);
        part(
            c,
            m,
            flap,
            nozzle_panel(sign, &exit_outline(), 0.18),
            thermal.clone(),
            Transform::from_translation(Vec3::Y * (sign * FLAP_SKIN_OFFSET)),
        );
        // Thin worn lips catch highlights; they are edge trim, not internal bars.
        let outline = exit_outline();
        for i in 0..outline.len() {
            let a = outline[i] + Vec3::Y * (sign * (FLAP_SKIN_OFFSET + 0.024));
            let b =
                outline[(i + 1) % outline.len()] + Vec3::Y * (sign * (FLAP_SKIN_OFFSET + 0.024));
            tube(c, m, flap, edge.clone(), a, b, 0.009);
        }
    }
}

fn tire(radius: f32, width: f32) -> Mesh {
    let profile = [
        (-0.5, 0.72),
        (-0.56, 0.83),
        (-0.46, 0.96),
        (-0.29, 1.),
        (-0.12, 0.985),
        (0., 1.),
        (0.12, 0.985),
        (0.29, 1.),
        (0.46, 0.96),
        (0.56, 0.83),
        (0.5, 0.72),
        (-0.5, 0.72), // Close the bead barrel as well as the tread/sidewalls.
    ];
    let mut p = Vec::new();
    let mut ix = Vec::new();
    for j in 0..profile.len() - 1 {
        for k in 0..64 {
            let at = |j: usize, k: usize| {
                let t = k as f32 / 64. * std::f32::consts::TAU;
                Vec3::new(
                    profile[j].0 * width,
                    t.cos() * radius * profile[j].1,
                    t.sin() * radius * profile[j].1,
                )
            };
            quad(
                &mut p,
                &mut ix,
                [at(j, k), at(j, k + 1), at(j + 1, k + 1), at(j + 1, k)],
            );
        }
    }
    raptor_mesh::smooth(mesh(p, ix))
}

// A visual stow pocket lifts the folded assembly into the bay. At full
// extension this is exactly the existing contact/compression pose.
fn detail_gear_pose(f: &Flight, i: usize) -> (DVec3, DVec3, bevy::math::DQuat) {
    let (a, b, q) = gear_pose(f, i, true);
    let lift = DVec3::Y * ((if i == 0 { 0.31 } else { 0.20 }) * (1. - f.gear_extension).powi(2));
    (a + lift, b + lift, q)
}
fn gear_visibility(f: &Flight, i: usize) -> Visibility {
    // The doors and airframe occlude stowed geometry. An extension threshold
    // made the open bay empty, then visibly spawned the complete assembly.
    if f.gear_health[i] > 0. {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    }
}
fn strut_mesh(radius: f32, kind: usize) -> Mesh {
    if matches!(kind, 2..=5 | 8 | 9) {
        let mut pieces = Vec::new();
        for side in [-1., 1.] {
            let q = [
                (-0.5, 1.4),
                (-0.39, 1.3),
                (0., 0.9),
                (0.39, 1.3),
                (0.5, 1.4),
                (0.5, 0.45),
                (-0.5, 0.45),
            ]
            .map(|(y, x)| Vec3::new(side * radius * x, y, 0.));
            pieces.push(solid(&q, Vec3::Z * radius * 0.48));
        }
        for y in [-0.44, 0.44] {
            pieces.push(
                Cuboid::new(radius * 2.5, 0.10, radius * 0.95)
                    .mesh()
                    .build()
                    .transformed_by(Transform::from_xyz(0., y, 0.)),
            );
        }
        let mut out = pieces.remove(0);
        for p in pieces {
            out.merge(&p).expect("brace attributes");
        }
        return out;
    }
    if kind > 1 {
        return Cylinder::new(radius, 1.).mesh().resolution(24).build();
    }
    let profile = [
        (-0.5, 1.1),
        (-0.47, 1.26),
        (-0.41, 1.26),
        (-0.39, 1.),
        (0.29, 1.),
        (0.31, 1.22),
        (0.43, 1.22),
        (0.47, 1.05),
        (0.5, 1.05),
    ];
    let mut v = Vec::new();
    let mut ix = Vec::new();
    let at = |j: usize, k: usize| {
        let t = k as f32 / 40. * std::f32::consts::TAU;
        Vec3::new(
            t.cos() * radius * profile[j].1,
            profile[j].0,
            t.sin() * radius * profile[j].1,
        )
    };
    for j in 0..profile.len() - 1 {
        for k in 0..40 {
            quad(
                &mut v,
                &mut ix,
                [at(j, k), at(j + 1, k), at(j + 1, k + 1), at(j, k + 1)],
            );
        }
    }
    for (j, n) in [(0, -Vec3::Y), (profile.len() - 1, Vec3::Y)] {
        for k in 0..40 {
            let a = Vec3::Y * profile[j].0;
            let b = at(j, k);
            let c = at(j, k + 1);
            let first = v.len() as u32;
            if (b - a).cross(c - a).dot(n) > 0. {
                v.extend([a, b, c]);
            } else {
                v.extend([a, c, b]);
            }
            ix.extend(first..first + 3);
        }
    }
    raptor_mesh::smooth(mesh(v, ix))
}

fn nose_well_ceiling(z:f32, mount_z:f32)->f32 {
    let ramp=|a:f32,b:f32|((z-mount_z-a)/(b-a)).clamp(0.,1.);
    // The wheel stows forward of the cockpit; the aft strut pocket needs far
    // less height. Step below the footwell, then below the seat, instead of
    // carrying the wheel-sized ceiling through the entire cabin.
    0.32-0.20*ramp(-1.10,-0.60)-0.44*ramp(0.10,0.35)
}
pub(super) fn gear(
    c: &mut Commands,
    m: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    root: Entity,
    skin: Handle<StandardMaterial>,
) {
    let white = material(mats, Color::srgb(0.76, 0.77, 0.72), 0.32);
    let chrome = metal(mats, [0.67, 0.70, 0.72], 0.18);
    let dark = material(mats, Color::srgb(0.10, 0.115, 0.12), 0.32);
    let rubber = mats.add(StandardMaterial {
        base_color: Color::srgb(0.023, 0.025, 0.027),
        perceptual_roughness: 0.94,
        alpha_mode: AlphaMode::Opaque,
        ..default()
    });
    let pin_mesh = m.add(Cylinder::new(0.026, 0.23).mesh().resolution(24).build());
    let boss_mesh = m.add(Cylinder::new(0.052, 0.035).mesh().resolution(24).build());
    let washer_mesh = m.add(Cylinder::new(0.033, 0.012).mesh().resolution(24).build());
    let hex_mesh = m.add(Cylinder::new(0.025, 0.025).mesh().resolution(6).build());
    for i in 0..3 {
        let at = gear_mount(Aircraft::F22, i).as_vec3();
        let z0 = at.z - 1.85;
        let z1 = at.z + 0.65;
        let w = 0.41;
        // Recessed polygonal wheel well follows the belly contour. No external
        // housing block and no tall cuboid side walls protrude through the skin.
        let mut rim = Vec::new();
        for (x, z) in [
            (-w, z0 + 0.18),
            (-w + 0.13, z0),
            (w - 0.13, z0),
            (w, z0 + 0.18),
            (w, z1 - 0.15),
            (w - 0.13, z1),
            (-w + 0.13, z1),
            (-w, z1 - 0.15),
        ] {
            rim.push(Vec3::new(at.x + x, skin_y(at.x + x, z, false) - 0.003, z));
        }
        if i==0 {
            let breaks=[at.z-1.10,at.z-0.60,at.z+0.10,at.z+0.35];
            let mut split=Vec::new();
            for k in 0..rim.len() {
                let a=rim[k];let b=rim[(k+1)%rim.len()];
                split.push(a);
                let mut cuts=breaks.iter().copied().filter(|z|*z>a.z.min(b.z)&&*z<a.z.max(b.z)).collect::<Vec<_>>();
                if a.z>b.z {cuts.reverse();}
                for z in cuts {let t=(z-a.z)/(b.z-a.z);let x=a.x+(b.x-a.x)*t;
                    split.push(Vec3::new(x,skin_y(x,z,false)-0.003,z));}
            }
            rim=split;
        }
        let roof = rim
            .iter()
            .map(|p| {
                Vec3::new(
                    p.x,
                    (skin_y(p.x, p.z, true) - 0.045).min(if i == 0 { nose_well_ceiling(p.z,at.z) } else { 0.28 }),
                    p.z,
                )
            })
            .collect::<Vec<_>>();
        if i==0 {
            let zs=[z0,z0+0.18,at.z-1.10,at.z-0.60,at.z+0.10,at.z+0.35,z1-0.15,z1];
            let point=|z:f32,side:f32| {
                let width=if z==z0||z==z1{w-0.13}else{w};
                let x=at.x+side*width;
                Vec3::new(x,(skin_y(x,z,true)-0.045).min(nose_well_ceiling(z,at.z)),z)
            };
            for z in zs.windows(2) {
                let e=part(c,m,root,solid(&[point(z[0],-1.),point(z[0],1.),point(z[1],1.),point(z[1],-1.)],Vec3::Y*0.008),white.clone(),Transform::default());
                c.entity(e).insert(crate::render_batch::RigidGeometry);
            }
        } else {part(
            c,
            m,
            root,
            solid(&roof, Vec3::Y * 0.008),
            white.clone(),
            Transform::default(),
        );}
        for k in 0..rim.len() {
            let j = (k + 1) % rim.len();
            let e=part(
                c,
                m,
                root,
                solid(&[rim[k], rim[j], roof[j], roof[k]], Vec3::X * 0.006),
                white.clone(),
                Transform::default(),
            );
            if i==0 {c.entity(e).insert(crate::render_batch::RigidGeometry);}
        }
        for k in 0..5 {
            tube(
                c,
                m,
                root,
                dark.clone(),
                Vec3::new(
                    at.x - 0.33,
                    if i == 0 { nose_well_ceiling(z0+0.25+k as f32*0.40,at.z)-0.035 } else { 0.265 },
                    z0 + 0.25 + k as f32 * 0.40,
                ),
                Vec3::new(
                    at.x + 0.33,
                    if i == 0 { nose_well_ceiling(z0+0.25+k as f32*0.40,at.z)-0.035 } else { 0.265 },
                    z0 + 0.25 + k as f32 * 0.40,
                ),
                0.012,
            );
        }
        let mut vents = Vec::new();
        let mut bolts = Vec::new();
        for side in [-1., 1.] {
            for short in [false, true] {
                let (start, stop) = if short {
                    (at.z - 0.65, z1)
                } else {
                    (z0, at.z - 0.65)
                };
                let center = (start + stop) * 0.5;
                let hinge = Vec3::new(
                    at.x + side * w,
                    skin_y(at.x + side * w, center, false),
                    center,
                );
                let mut q = Vec::new();
                for (x, z) in [
                    (side * w, start + 0.10),
                    (side * (w - 0.08), start),
                    (0., start + 0.01),
                    (0., stop - 0.01),
                    (side * (w - 0.08), stop),
                    (side * w, stop - 0.10),
                ] {
                    let x = at.x + x;
                    q.push(Vec3::new(x, skin_y(x, z, false) - 0.009, z) - hinge);
                }
                let door = part(
                    c,
                    m,
                    root,
                    raptor_mesh::paint_at(solid(&q, Vec3::Y * 0.011), hinge),
                    skin.clone(),
                    Transform::from_translation(hinge),
                );
                c.entity(door).insert(GearDoor(i, side, hinge, short));
                // Inner face, longitudinal stiffening ribs, hinge barrels and latches.
                let inner = q.iter().map(|p| *p + Vec3::Y * 0.024).collect::<Vec<_>>();
                part(
                    c,
                    m,
                    door,
                    solid(&inner, Vec3::Y * 0.004),
                    white.clone(),
                    Transform::default(),
                );
                for z in [-0.36, 0., 0.36] {
                    tube(
                        c,
                        m,
                        door,
                        white.clone(),
                        Vec3::new(-side * 0.04, 0.04, z),
                        Vec3::new(-side * 0.30, 0.04, z),
                        0.012,
                    );
                    tube(
                        c,
                        m,
                        door,
                        chrome.clone(),
                        Vec3::new(0., 0., z - 0.08),
                        Vec3::new(0., 0., z + 0.08),
                        0.025,
                    );
                }
            }
        }
        for kind in 0..15 {
            let radius = match kind {
                0 => 0.081,
                1 => 0.051,
                2 | 3 => 0.030,
                4 | 5 => 0.036,
                6 => 0.045,
                7 => 0.022,
                8 | 9 => 0.035,
                10 => 0.049,
                _ => 0.010,
            };
            let mat = match kind {
                1 | 7 | 10 => chrome.clone(),
                11.. => dark.clone(),
                _ => white.clone(),
            };
            let e = part(
                c,
                m,
                root,
                strut_mesh(radius, kind),
                mat,
                Transform::default(),
            );
            c.entity(e).insert(GearLink(i, kind));
            if kind == 0 {
                for y in [0.31, 0.41] {
                    part(
                        c,
                        m,
                        e,
                        Cylinder::new(radius * 1.24, 0.026)
                            .mesh()
                            .resolution(32)
                            .build(),
                        dark.clone(),
                        Transform::from_xyz(0., y, 0.),
                    );
                }
                // Servicing placard on the shock housing.
                part(
                    c,
                    m,
                    e,
                    Cuboid::new(0.055, 0.12, 0.004).mesh().build(),
                    dark.clone(),
                    Transform::from_xyz(0., -0.03, radius + 0.002),
                );
                for y in [-0.06, -0.025, 0.01] {
                    part(
                        c,
                        m,
                        e,
                        Cuboid::new(0.036, 0.008, 0.004).mesh().build(),
                        white.clone(),
                        Transform::from_xyz(0., y, radius + 0.005),
                    );
                }
            }
        }
        // Clevis bosses, cross pins, washers and hex heads articulate with
        // the brace joints instead of being decorative floating cylinders.
        for k in 0..8 {
            let joint = c
                .spawn((GearJoint(i, k), Transform::default(), Visibility::Inherited))
                .id();
            c.entity(root).add_child(joint);
            let mut fittings = vec![(pin_mesh.clone(), chrome.clone(), 0.)];
            for side in [-1., 1.] {
                fittings.extend([
                    (boss_mesh.clone(), white.clone(), side * 0.075),
                    (washer_mesh.clone(), dark.clone(), side * 0.10),
                    (hex_mesh.clone(), chrome.clone(), side * 0.112),
                ]);
            }
            for (mesh, mat, y) in fittings {
                let e = c
                    .spawn((
                        Mesh3d(mesh),
                        MeshMaterial3d(mat),
                        Transform::from_xyz(0., y, 0.),
                        Visibility::Inherited,
                    ))
                    .id();
                c.entity(joint).add_child(e);
            }
        }
        let wheel = c
            .spawn((
                Wheel {
                    index: i,
                    angle: 0.,
                    speed: 0.,
                    last_time: 0.,
                },
                Transform::default(),
                Visibility::Inherited,
            ))
            .id();
        c.entity(root).add_child(wheel);
        let fork = c
            .spawn((Fork(i), Transform::default(), Visibility::Inherited))
            .id();
        c.entity(root).add_child(fork);
        for s in [-1., 1.] {
            let outline = [
                (0.37, -0.04),
                (0.36, 0.055),
                (0.31, 0.10),
                (0.20, 0.10),
                (0.08, 0.075),
                (-0.02, 0.06),
                (-0.055, 0.025),
                (-0.055, -0.025),
                (-0.02, -0.060),
                (0.10, -0.075),
                (0.23, -0.055),
                (0.34, -0.055),
            ]
            .map(|(y, z)| Vec3::new(s * 0.15, y, z));
            part(
                c,
                m,
                fork,
                solid(&outline, Vec3::X * 0.022),
                white.clone(),
                Transform::default(),
            );
            let rot = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
            part(
                c,
                m,
                fork,
                Cylinder::new(0.069, 0.035).mesh().resolution(32).build(),
                white.clone(),
                Transform::from_xyz(s * 0.16, 0., 0.).with_rotation(rot),
            );
            part(
                c,
                m,
                fork,
                Cylinder::new(0.033, 0.023).mesh().resolution(6).build(),
                chrome.clone(),
                Transform::from_xyz(s * 0.185, 0., 0.).with_rotation(rot),
            );
        }
        tube(
            c,
            m,
            fork,
            white.clone(),
            Vec3::new(-0.15, 0.35, 0.01),
            Vec3::new(0.15, 0.35, 0.01),
            0.045,
        );
        let radius = if i == 0 { 0.29 } else { 0.34 };
        let width = if i == 0 { 0.19 } else { 0.25 };
        if i > 0 {
            let side = if i == 1 { 1. } else { -1. };
            for n in 0..5 {
                part(
                    c,
                    m,
                    wheel,
                    Cylinder::new(radius * 0.60, 0.012)
                        .mesh()
                        .resolution(40)
                        .build(),
                    dark.clone(),
                    Transform::from_xyz(side * (width * 0.50 + 0.014 + n as f32 * 0.014), 0., 0.)
                        .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                );
            }
        }
        part(
            c,
            m,
            wheel,
            tire(radius, width),
            rubber.clone(),
            Transform::default(),
        );
        for side in [-1., 1.] {
            let rot = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
            part(
                c,
                m,
                wheel,
                Cylinder::new(radius * 0.735, 0.025)
                    .mesh()
                    .resolution(48)
                    .build(),
                white.clone(),
                Transform::from_xyz(side * width * 0.50, 0., 0.).with_rotation(rot),
            );
            part(
                c,
                m,
                wheel,
                Cylinder::new(radius * 0.26, 0.045)
                    .mesh()
                    .resolution(32)
                    .build(),
                chrome.clone(),
                Transform::from_xyz(side * width * 0.57, 0., 0.).with_rotation(rot),
            );
            for k in 0..10 {
                let t = k as f32 / 10. * std::f32::consts::TAU;
                vents.push(
                    Cylinder::new(0.018, 0.008)
                        .mesh()
                        .resolution(12)
                        .build()
                        .transformed_by(
                            Transform::from_xyz(
                                side * width * 0.56,
                                t.cos() * radius * 0.49,
                                t.sin() * radius * 0.49,
                            )
                            .with_rotation(rot),
                        ),
                );
                bolts.push(
                    Cylinder::new(0.009, 0.012)
                        .mesh()
                        .resolution(6)
                        .build()
                        .transformed_by(
                            Transform::from_xyz(
                                side * width * 0.58,
                                t.cos() * radius * 0.31,
                                t.sin() * radius * 0.31,
                            )
                            .with_rotation(rot),
                        ),
                );
            }
        }
        batch(c, m, wheel, vents, dark.clone());
        batch(c, m, wheel, bolts, chrome.clone());
    }
}

fn leg_link(f: &Flight, i: usize, k: usize) -> (Vec3, Vec3) {
    let (at, end, q) = detail_gear_pose(f, i);
    let at = at.as_vec3();
    let e = end.as_vec3();
    let q = q.as_quat();
    let p = |v: Vec3| at + q * v;
    let w = |v: Vec3| e + q * v;
    let side = if i == 1 { -1. } else { 1. };
    match k {
        0 => (p(Vec3::new(0., 0.03, 0.)), p(Vec3::new(0., -0.75, 0.))),
        1 => (p(Vec3::new(0., -0.69, 0.)), w(Vec3::new(0., 0.11, 0.))),
        2 => (p(Vec3::new(0., -0.53, 0.08)), w(Vec3::new(0., 0.38, 0.25))),
        3 => (w(Vec3::new(0., 0.38, 0.25)), w(Vec3::new(0., 0.13, 0.07))),
        4 => (
            at + Vec3::new(side * 0.30, 0.10, -0.48),
            p(Vec3::new(side * 0.13, -0.48, -0.17)),
        ),
        5 => (
            p(Vec3::new(side * 0.13, -0.48, -0.17)),
            w(Vec3::new(0., 0.34, 0.)),
        ),
        6 => (
            at + Vec3::new(-side * 0.24, 0.08, -0.45),
            p(Vec3::new(-side * 0.12, -0.39, -0.15)),
        ),
        7 => (
            p(Vec3::new(-side * 0.12, -0.39, -0.15)),
            p(Vec3::new(0., -0.73, -0.04)),
        ),
        8 => (w(Vec3::new(-0.11, 0.26, 0.)), w(Vec3::new(-0.14, 0., 0.))),
        9 => (w(Vec3::new(0.11, 0.26, 0.)), w(Vec3::new(0.14, 0., 0.))),
        10 => (w(Vec3::new(-0.17, 0., 0.)), w(Vec3::new(0.17, 0., 0.))),
        11 => (
            p(Vec3::new(0.10, -0.12, 0.09)),
            p(Vec3::new(0.12, -0.47, 0.10)),
        ),
        12 => (
            p(Vec3::new(0.12, -0.47, 0.10)),
            w(Vec3::new(0.14, 0.48, 0.08)),
        ),
        13 => (
            w(Vec3::new(0.14, 0.48, 0.08)),
            w(Vec3::new(0.16, 0.16, 0.06)),
        ),
        _ => (w(Vec3::new(0.16, 0.16, 0.06)), w(Vec3::new(0.13, 0., 0.04))),
    }
}
pub fn update(
    f: Res<Flight>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<
        (
            Option<&ExhaustFlap>,
            Option<&NozzleShoulder>,
            Option<&GearLink>,
            Option<&mut Wheel>,
            Option<&Fork>,
            Option<&GearDoor>,
            Option<&GearJoint>,
            Option<&ShockCell>,
            Option<&GlowLayer>,
            &mut Transform,
            &mut Visibility,
        ),
        Or<(
            With<ExhaustFlap>,
            With<NozzleShoulder>,
            With<GearLink>,
            With<Wheel>,
            With<Fork>,
            With<GearDoor>,
            With<GearJoint>,
            With<ShockCell>,
            With<GlowLayer>,
        )>,
    >,
) {
    if f.aircraft != Aircraft::F22 {
        return;
    }
    for (flap, shoulder, link, wheel, fork, door, joint, shock, glow, mut t, mut v) in &mut q {
        if let Some(NozzleShoulder(engine, sign)) = shoulder {
            *t = shoulder_pose(&f, *engine, *sign);
            continue;
        }
        if let Some(GearJoint(i, k)) = joint {
            let (_, b) = leg_link(&f, *i, *k);
            let (_, _, q) = detail_gear_pose(&f, *i);
            t.translation = b;
            t.rotation = q.as_quat() * Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
            v.set_if_neq(gear_visibility(&f, *i));
            continue;
        }
        if let Some(ShockCell(engine, cell, mat)) = shock {
            let power = exhaust_power(&f, *engine);
            let wet = ((power - 0.88) / 0.12).clamp(0., 1.);
            let k = *cell as f32;
            let envelope = (wet * 6. - k).clamp(0., 1.);
            let motion = crate::plume_motion::sample(f.elapsed, (*engine * 37 + *cell + 200) as u32);
            let flicker = motion.radiance;
            let energy = envelope * (1. - k * 0.115) * flicker;
            let spacing = (0.72 + wet * 0.23) * (1. + (motion.length - 1.) * 0.35);
            let vector = Quat::from_rotation_x(f.systems.nozzle[*engine] as f32) * motion.bend;
            t.rotation = vector;
            t.translation = Vec3::Z * EXHAUST_START
                + vector * (Vec3::Z * (1.58 - EXHAUST_START + (k + 0.5) * spacing));
            t.scale = Vec3::new(
                (0.25 - k * 0.020) * flicker,
                (0.15 - k * 0.012) * flicker,
                spacing * 0.96,
            );
            v.set_if_neq(if energy > 0.001 {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            });
            // Unlit StandardMaterial bypasses emissive lighting. Put radiance
            // in linear HDR base color so the existing bloom sees the jet.
            let color = Color::linear_rgba(1.5, 2.8, 8., 0.18 * energy);
            let emission = LinearRgba::BLACK;
            let changed = materials
                .get(mat)
                .is_some_and(|m| m.base_color != color || m.emissive != emission);
            if changed {
                if let Some(m) = materials.get_mut(mat) {
                    m.base_color = color;
                    m.emissive = emission;
                }
            }
            continue;
        }
        if let Some(GlowLayer(engine, layer, mat)) = glow {
            let power = exhaust_power(&f, *engine);
            let wet = ((power - 0.88) / 0.12).clamp(0., 1.);
            let pulse = crate::plume_motion::sample(f.elapsed, (*engine * 37 + *layer) as u32).radiance;
            let strength = (0.12 + power * 0.55 + wet * 0.9) * pulse;
            let core = *layer < 2;
            let color = if *layer == 7 {
                Color::linear_rgba(
                    2. + power * 5.,
                    0.8 + wet * 1.8,
                    0.25 + wet * 0.7,
                    (0.14 + 0.26 * power) * pulse,
                )
            } else if core {
                Color::linear_rgba(
                    0.8 + wet * 7.2,
                    1. + wet * 2.,
                    3. - wet * 2.3,
                    0.10 * strength,
                )
            } else {
                Color::linear_rgba(0.20, 0.75, 4., 0.035 * strength)
            };
            let emission = LinearRgba::BLACK;
            let changed = materials
                .get(mat)
                .is_some_and(|m| m.base_color != color || m.emissive != emission);
            if changed {
                if let Some(m) = materials.get_mut(mat) {
                    m.base_color = color;
                    m.emissive = emission;
                }
            }
            v.set_if_neq(if power > 0.001 {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            });
            continue;
        }
        if let Some(ExhaustFlap(engine, sign)) = flap {
            // Variable throat/exit area is separate from the parent pitch-vector
            // transform. No decorative periodic steering or parked demo motion.
            *t = flap_pose(&f, *engine, *sign);
            continue;
        }
        if let Some(GearLink(i, k)) = link {
            let (a, b) = leg_link(&f, *i, *k);
            let d = b - a;
            *t = Transform::from_translation((a + b) * 0.5)
                .with_rotation(Quat::from_rotation_arc(Vec3::Y, d.normalize_or_zero()))
                .with_scale(Vec3::new(1., d.length(), 1.));
            v.set_if_neq(gear_visibility(&f, *i));
            continue;
        }
        if let Some(mut wheel) = wheel {
            wheel.advance(&f);
            let i = wheel.index;
            let (_, end, q) = detail_gear_pose(&f, i);
            let steer = if i == 0 {
                -f.controls.y as f32 * 0.5 / (1. + f.airspeed as f32 * 0.1)
            } else {
                0.
            };
            t.translation = end.as_vec3();
            t.rotation = q.as_quat()
                * Quat::from_rotation_y(steer)
                * Quat::from_rotation_x(wheel.angle as f32);
            v.set_if_neq(gear_visibility(&f, i));
            continue;
        }
        if let Some(Fork(i)) = fork {
            let (_, end, q) = detail_gear_pose(&f, *i);
            t.translation = end.as_vec3();
            let steer = if *i == 0 {
                -f.controls.y as f32 * 0.5 / (1. + f.airspeed as f32 * 0.1)
            } else {
                0.
            };
            t.rotation = q.as_quat() * Quat::from_rotation_y(steer);
            v.set_if_neq(gear_visibility(&f, *i));
            continue;
        }
        if let Some(GearDoor(i, side, hinge, short)) = door {
            let opening = if *short {
                f.gear_door.max((f.gear_extension * 8.).min(1.))
            } else {
                f.gear_door
            };
            let open = if f.gear_health[*i] < 0.1 {
                opening.max(0.7)
            } else {
                opening
            };
            t.rotation = Quat::from_rotation_z(*side * open as f32 * 1.48);
            t.translation = *hinge;
        }
    }
}

/// Export the exact runtime geometry without window, terrain, audio or GPU.
/// The plotting tool consumes this dump; it never reimplements the model.
pub fn export_nozzle_preview() {
    let mut app = App::new();
    let mut f = Flight::runway();
    f.aircraft = Aircraft::F22;
    f.engine = false;
    f.thrust = 0.;
    app.insert_resource(f)
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<Assets<Image>>()
        .add_systems(
            Startup,
            |mut c: Commands,
             mut meshes: ResMut<Assets<Mesh>>,
             mut mats: ResMut<Assets<StandardMaterial>>,
             mut images: ResMut<Assets<Image>>| {
                let root = c.spawn((Transform::default(), Visibility::Inherited)).id();
                let flow = c.spawn((Transform::default(), Visibility::Inherited)).id();
                c.entity(root).add_child(flow);
                let skin = super::material(&mut mats, Color::srgb(0.59, 0.61, 0.62), 0.3);
                nozzle(
                    &mut c,
                    &mut meshes,
                    &mut mats,
                    &mut images,
                    root,
                    flow,
                    0,
                    Vec3::ZERO,
                    skin,
                );
            },
        )
        .add_systems(Update, update);
    let mut poses = Vec::new();
    fn matrix(w: &World, e: Entity) -> Mat4 {
        let local = w
            .get::<Transform>(e)
            .map(Transform::to_matrix)
            .unwrap_or(Mat4::IDENTITY);
        if let Some(parent) = w.get::<ChildOf>(e) {
            matrix(w, parent.parent()) * local
        } else {
            local
        }
    }
    for (label, angle, power) in [
        ("neutral", 0., 0.),
        ("up", -20_f64.to_radians(), 1.),
        ("down", 20_f64.to_radians(), 1.),
    ] {
        {
            let mut f = app.world_mut().resource_mut::<Flight>();
            f.systems.nozzle = [angle; 2];
            f.spool = power;
        }
        app.update();
        let parts = app
            .world_mut()
            .query::<(Entity, &Mesh3d, &MeshMaterial3d<StandardMaterial>)>()
            .iter(app.world())
            .map(|(e, m, mat)| (e, m.0.clone(), mat.0.clone()))
            .collect::<Vec<_>>();
        let w = app.world();
        let meshes = w.resource::<Assets<Mesh>>();
        let mats = w.resource::<Assets<StandardMaterial>>();
        let mut output = Vec::new();
        for (e, mesh, material) in parts {
            if w.get::<GlowLayer>(e).is_some() || w.get::<ShockCell>(e).is_some() {
                continue;
            }
            let m = meshes.get(&mesh).unwrap();
            let mat = mats.get(&material).unwrap();
            let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) =
                m.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                continue;
            };
            let transform = matrix(w, e);
            let positions = p
                .iter()
                .map(|p| transform.transform_point3(Vec3::from_array(*p)).to_array())
                .collect::<Vec<_>>();
            let fixed = w
                .get::<ChildOf>(e)
                .is_some_and(|p| w.get::<FixedNozzle>(p.parent()).is_some());
            output.push(serde_json::json!({"positions":positions,"indices":m.indices().unwrap().iter().collect::<Vec<_>>(),"color":mat.base_color.to_srgba().to_f32_array(),"metallic":mat.metallic,"roughness":mat.perceptual_roughness,"fixed":fixed}));
        }
        poses.push(
            serde_json::json!({"name":label,"vector_radians":angle,"spool":power,"parts":output}),
        );
    }
    println!(
        "{}",
        serde_json::json!({"source":"runtime raptor_detail::nozzle and update; no GPU","poses":poses})
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nozzle_wedges_are_closed_outward_solids_and_mirror_away_from_the_flow() {
        use bevy::mesh::VertexAttributeValues::Float32x3;
        for sign in [-1., 1.] {
            for (panel, depth) in [
                (flap_plate(sign, SHOULDER_LENGTH), 0.10),
                (nozzle_panel(sign, &exit_outline(), 0.18), 0.18),
            ] {
                let Some(Float32x3(p)) = panel.attribute(Mesh::ATTRIBUTE_POSITION) else {
                    panic!("positions")
                };
                let Some(Float32x3(normals)) = panel.attribute(Mesh::ATTRIBUTE_NORMAL) else {
                    panic!("normals")
                };
                assert!(normals.iter().all(|n| Vec3::from_array(*n).is_finite()));
                assert!(p.iter().any(|p| (p[1] * sign - depth).abs() < 1e-6));
                assert!(p.iter().all(|p| p[1] * sign >= -0.018001));
                let ix = panel.indices().unwrap().iter().collect::<Vec<_>>();
                let mut edges = std::collections::HashMap::new();
                let key = |i: usize| p[i].map(|x| (x * 1e6).round() as i32);
                let mut volume = 0.;
                for tri in ix.chunks_exact(3) {
                    let [a, b, c] = [tri[0], tri[1], tri[2]].map(|i| Vec3::from_array(p[i]));
                    assert!((b - a).cross(c - a).length() > 1e-6);
                    volume += a.dot(b.cross(c)) / 6.;
                    for (a, b) in [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
                        let mut e = [key(a), key(b)];
                        e.sort();
                        *edges.entry(e).or_insert(0) += 1;
                    }
                }
                assert!(
                    edges.values().all(|n| *n == 2),
                    "solid must have no open edges"
                );
                assert!(
                    volume > 0.03,
                    "outward winding and meaningful enclosed volume: {volume}"
                );
            }
        }
    }
    #[test]
    fn forward_nozzle_fairings_are_seated_inside_the_airframe_and_keep_the_exit() {
        for engine in 0..2 {
            let origin = engine_origin(Aircraft::F22.engine_positions()[engine]).as_vec3();
            for k in 0..=128 {
                let theta = k as f32 / 128. * std::f32::consts::TAU;
                let p = origin + fairing_point(engine, 0., theta);
                assert!(p.y < skin_y(p.x, p.z, true) && p.y > skin_y(p.x, p.z, false));
                let end = fairing_point(engine, 1., theta);
                let expected = Vec3::new(
                    theta.cos().signum() * theta.cos().abs().powf(0.38) * 0.81,
                    theta.sin().signum() * theta.sin().abs().powf(0.46) * 0.40,
                    -0.08,
                );
                assert!(end.distance(expected) < 1e-5);
                for j in 0..=24 {
                    assert!(fairing_point(engine, j as f32 / 24., theta).is_finite());
                }
            }
        }
    }
    #[test]
    fn nozzle_exit_has_a_triangular_extension_and_opaque_weathering() {
        let outline = exit_outline();
        assert_eq!(outline[0].z, 0.);
        assert_eq!(outline[1].z, 0.);
        assert_eq!(outline[2].z, outline[4].z);
        assert_eq!(outline[3].x, 0.);
        assert!(
            outline
                .iter()
                .enumerate()
                .all(|(i, p)| i == 3 || p.z < outline[3].z),
            "triangular tip must point aft, away from the nose"
        );
        let image = panel_texture();
        let pixels = image.data.unwrap();
        assert_eq!(pixels.len(), 512 * 512 * 4);
        assert!(
            pixels
                .chunks_exact(4)
                .all(|p| p[3] == 255 && p[0] > 140 && p[0].abs_diff(p[2]) < 8)
        );
        let light = pixels.chunks_exact(4).map(|p| p[0]).max().unwrap();
        let dark = pixels.chunks_exact(4).map(|p| p[0]).min().unwrap();
        assert!(light - dark > 20, "weathering must survive texture upload");
        assert!(
            pixels.chunks_exact(4).all(|p| p[0] <= p[1] && p[1] <= p[2]),
            "exhaust weathering should share the airframe's cool gray palette"
        );
        assert_eq!(pixels, panel_texture().data.unwrap());
    }
    #[test]
    fn nozzle_sections_share_hinges_through_vectoring() {
        let mut f = Flight::runway();
        for power in [0., 1.] {
            for angle in [-20_f64.to_radians(), 0., 20_f64.to_radians()] {
                f.spool = power;
                f.systems.nozzle = [angle; 2];
                for sign in [-1., 1.] {
                    let pose = flap_pose(&f, 0, sign);
                    let shoulder = shoulder_pose(&f, 0, sign);
                    assert_eq!(shoulder.translation, Vec3::new(0., sign * 0.40, -0.06));
                    for x in [-0.76, 0., 0.76] {
                        assert!(
                            pose.transform_point(Vec3::X * x).distance(
                                shoulder.transform_point(Vec3::new(x, 0., SHOULDER_LENGTH))
                            ) < 1e-6
                        );
                    }
                    assert!(shoulder.rotation.angle_between(pose.rotation) > 0.001);
                    assert!(
                        (pose.rotation.to_euler(EulerRot::XYZ).0
                            - angle as f32
                            - sign * (0.12 - 0.22 * power as f32))
                            .abs()
                            < 1e-5
                    );
                }
            }
        }
    }
    #[test]
    fn articulated_nozzle_keeps_a_clear_aperture_through_power_and_vector_sweep() {
        let mut f = Flight::runway();
        for power in 0..=20 {
            f.spool = power as f64 / 20.;
            for degrees in -20..=20 {
                f.systems.nozzle = [(degrees as f64).to_radians(); 2];
                let upper = flap_pose(&f, 0, 1.);
                let lower = flap_pose(&f, 0, -1.);
                for z in [0., FLAP_LENGTH * 0.5, FLAP_LENGTH, FLAP_TIP] {
                    let a = upper.transform_point(Vec3::new(0., FLAP_SKIN_OFFSET, z));
                    let b = lower.transform_point(Vec3::new(0., -FLAP_SKIN_OFFSET, z));
                    assert!(a.is_finite() && b.is_finite());
                    assert!(a.y - b.y > 0.40, "power {power}, vector {degrees}, z {z}");
                }
            }
        }
    }
    #[test]
    fn tire_is_closed_and_tread_faces_outward() {
        use bevy::mesh::VertexAttributeValues::Float32x3;
        let m = tire(0.34, 0.25);
        let Some(Float32x3(p)) = m.attribute(Mesh::ATTRIBUTE_POSITION) else {
            panic!("positions")
        };
        let Some(Float32x3(n)) = m.attribute(Mesh::ATTRIBUTE_NORMAL) else {
            panic!("normals")
        };
        for (p, n) in p.iter().zip(n) {
            let radial = Vec3::new(0., p[1], p[2]);
            assert!(Vec3::from_array(*n).is_finite());
            if radial.length() > 0.34 * 0.975 {
                assert!(Vec3::from_array(*n).dot(radial.normalize()) > 0.6);
            }
        }
        let key = |i: usize| p[i].map(|x| (x * 100000.).round() as i32);
        let mut edges = std::collections::HashMap::new();
        let ix = m.indices().unwrap().iter().collect::<Vec<_>>();
        for t in ix.chunks_exact(3) {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                let mut e = [key(a), key(b)];
                e.sort();
                *edges.entry(e).or_insert(0) += 1;
            }
        }
        assert!(edges.values().all(|n| *n == 2));
    }
    #[test]
    fn gear_is_present_and_moves_continuously_through_real_sequence() {
        let mut f = Flight::runway();
        super::super::super::select(&mut f, Aircraft::F22);
        f.gear = true;
        f.gear_extension = 0.;
        f.gear_door = 0.;
        f.gear_phase = 0;
        let mut last = (0..3)
            .map(|i| detail_gear_pose(&f, i).1)
            .collect::<Vec<_>>();
        for _ in 0..4800 {
            crate::collision::update_gear(&mut f, 1. / 1200.);
            for i in 0..3 {
                assert_eq!(gear_visibility(&f, i), Visibility::Inherited);
                let (_, p, _) = detail_gear_pose(&f, i);
                assert!(p.distance(last[i]) < 0.003);
                last[i] = p;
            }
        }
        assert_eq!(f.gear_extension, 1.);
        assert_eq!(f.gear_door, 0.);
        for i in 0..3 {
            assert_eq!(detail_gear_pose(&f, i), gear_pose(&f, i, true));
        }
        f.gear_health[1] = 0.;
        assert_eq!(gear_visibility(&f, 1), Visibility::Hidden);
    }
    #[test]
    fn stowed_tires_fit_above_closed_belly_doors() {
        let mut f = Flight::runway();
        super::super::super::select(&mut f, Aircraft::F22);
        f.gear_extension = 0.;
        f.gear_compression = [0.; 3];
        for i in 0..3 {
            let (_, at, q) = detail_gear_pose(&f, i);
            let m = tire(
                if i == 0 { 0.29 } else { 0.34 },
                if i == 0 { 0.19 } else { 0.25 },
            );
            if let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) =
                m.attribute(Mesh::ATTRIBUTE_POSITION)
            {
                for p in p {
                    let p = (at + q * Vec3::from_array(*p).as_dvec3()).as_vec3();
                    assert!(p.y > skin_y(p.x, p.z, false) + 0.01, "gear {i}: {p:?}");
                }
            }
        }
    }
    #[test]
    fn stowed_nose_wheel_remains_below_the_stepped_cockpit_partition() {
        let mut f=Flight::runway();
        super::super::super::select(&mut f,Aircraft::F22);
        f.gear_extension=0.;f.gear_compression=[0.;3];
        let (_,at,q)=detail_gear_pose(&f,0);
        let wheel=tire(0.29,0.19);
        let mount=gear_mount(Aircraft::F22,0).z as f32;
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(vertices))=wheel.attribute(Mesh::ATTRIBUTE_POSITION) else{panic!("wheel vertices");};
        for p in vertices {
            let p=(at+q*Vec3::from_array(*p).as_dvec3()).as_vec3();
            let ceiling=(skin_y(p.x,p.z,true)-0.045).min(nose_well_ceiling(p.z,mount))-0.008;
            assert!(p.y<ceiling,"nose-wheel/partition intersection: {p:?}, roof {ceiling}");
        }
    }
    #[test]
    fn burner_and_glow_begin_inside_nozzle() {
        assert!(EXHAUST_START < 0.);
        let m = burner_face();
        if let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) =
            m.attribute(Mesh::ATTRIBUTE_POSITION)
        {
            assert!(p.iter().all(|p| p[2] < EXHAUST_START && p[2] > -0.32));
        } else {
            panic!("positions");
        }
    }
    #[test]
    fn exhaust_requires_running_fueled_healthy_engine() {
        let mut f = Flight::runway();
        super::super::super::select(&mut f, Aircraft::F22);
        f.engine = true;
        f.systems.engine_on = [true; 2];
        f.thrust = 1000.;
        f.spool = 0.25;
        assert_eq!(exhaust_power(&f, 0), 0.25);
        f.engine = false;
        assert_eq!(exhaust_power(&f, 0), 0.);
        f.engine = true;
        f.systems.engine_on[0] = false;
        assert_eq!(exhaust_power(&f, 0), 0.);
        assert_eq!(exhaust_power(&f, 1), 0.25);
        f.systems.engine_on[0] = true;
        f.systems.engines[0] = 0.;
        assert_eq!(exhaust_power(&f, 0), 0.);
        f.fuel = 0.;
        f.infinite_fuel = false;
        assert_eq!(exhaust_power(&f, 1), 0.);
        f.infinite_fuel = true;
        assert_eq!(exhaust_power(&f, 1), 0.25);
        f.thrust = 0.;
        assert_eq!(exhaust_power(&f, 1), 0.);
    }
    #[test]
    fn wing_has_broad_root_and_forward_swept_trailing_edge() {
        let w = wing(1.);
        assert!(w[4].z > w[3].z && w[3].z > w[2].z);
        assert!(w[4].z - w[0].z > 6.);
        assert!(w[2].z - w[1].z < 1.5);
    }
    #[test]
    fn nozzle_exits_are_ahead_of_stabilator_trailing_edges() {
        let f = Flight::runway();
        let tip = [-1., 1.]
            .into_iter()
            .map(|s| {
                flap_pose(&f, 0, s)
                    .transform_point(Vec3::new(0., s * FLAP_SKIN_OFFSET, FLAP_TIP))
                    .z
            })
            .fold(f32::NEG_INFINITY, f32::max);
        let exit = engine_origin(Aircraft::F22.engine_positions()[0]).z + tip as f64;
        let tail = surface_polygon(0)
            .0
            .iter()
            .map(|p| p.z)
            .fold(f32::NEG_INFINITY, f32::max) as f64;
        assert!(tail - exit > 0.5 && tail - exit < 2.0);
    }
    #[test]
    fn rendered_wing_hinges_stay_seated_when_controls_move() {
        let p = panels();
        for i in [1, 2, 3, 5, 6, 7, 9, 10] {
            let q = p[i].outline.unwrap();
            let edge = if i >= 9 { [q[2], q[3]] } else { [q[0], q[1]] };
            for angle in [-0.4, 0., 0.4] {
                let r = surface_rotation(i, angle);
                for a in edge {
                    let local = (a - p[i].center).as_vec3();
                    assert!((r * local - local).length() < 1e-5);
                }
            }
        }
    }
    #[test]
    fn wheel_spin_preserves_pause_and_clears_on_reset() {
        let mut f = Flight::runway();
        let mut w = Wheel {
            index: 0,
            angle: 0.,
            speed: 0.,
            last_time: 0.,
        };
        f.gear_compression[0] = 0.1;
        f.velocity = DVec3::X * 10.;
        f.elapsed = 0.1;
        w.advance(&f);
        assert!(w.angle > 0.);
        let angle = w.angle;
        w.advance(&f);
        assert_eq!(angle, w.angle);
        f.elapsed = 0.;
        w.advance(&f);
        assert_eq!(w.angle, 0.);
    }
    #[test]
    fn glazing_and_sill_meet_at_every_sample() {
        for j in 0..65 {
            for t in [0., std::f32::consts::PI] {
                let p = canopy_point(j as f32 / 64., t);
                assert!(p.is_finite());
                assert!((p.y - skin_y(p.x, p.z, true) - 0.015).abs() < 1e-5);
            }
        }
    }
    #[test]
    fn nozzle_and_tire_have_finite_nonempty_geometry() {
        for m in [
            shell(&[(-0.4, 0.7, 0.3), (1.4, 0.75, 0.29)], 24),
            shock_mesh(),
            tire(0.34, 0.25),
            sill(),
        ] {
            assert!(m.count_vertices() > 0);
            if let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) =
                m.attribute(Mesh::ATTRIBUTE_POSITION)
            {
                assert!(p.iter().all(|v| v.iter().all(|x| x.is_finite())));
            }
        }
    }
    #[test]
    fn gear_links_follow_compression_and_retraction() {
        let mut f = Flight::runway();
        super::super::super::select(&mut f, Aircraft::F22);
        for ext in [0., 0.25, 0.5, 0.75, 1.] {
            f.gear_extension = ext;
            for i in 0..3 {
                for k in 0..15 {
                    let (a, b) = leg_link(&f, i, k);
                    assert!(a.is_finite() && b.is_finite());
                    assert!(a.distance(b) > 0.01);
                }
            }
        }
    }
}
