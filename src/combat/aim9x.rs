//! Presentation for the AIM-9X. Fixed forward wings, shaft-coupled tail fins
//! and internal jet vanes; deflection limits remain explicit game estimates.
use super::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

const SIDES: usize = 64;
pub const FIN_LIMIT: f32 = 20.0 * std::f32::consts::PI / 180.0;

#[derive(Resource)]
pub struct VisualAssets {
    pub plumes: [Handle<Mesh>; 3],
    pub plume_materials: [Handle<StandardMaterial>; 3],
    pub smoke: Handle<StandardMaterial>,
    tail: Handle<Mesh>,
    vane: Handle<Mesh>,
    metal: Handle<StandardMaterial>,
    lens: Handle<Mesh>,
    glass: Handle<StandardMaterial>,
}
#[derive(Component)]
pub struct Actuator {
    missile: usize,
    quadrant: usize,
    vane: bool,
}
#[derive(Component)]
pub struct Finish(usize);

/// Append a sealed, thin fin prism. Unlike the original triangle this has
/// leading/trailing edges, thickness and an actual hinge at the shaft origin.
pub(super) fn fin(
    p: &mut Vec<Vec3>,
    ix: &mut Vec<u32>,
    outline: &[(f32, f32)],
    thickness: f32,
    transform: Transform,
) {
    let n = p.len() as u32;
    let count = outline.len() as u32;
    for y in [-thickness, thickness] {
        p.extend(
            outline
                .iter()
                .map(|&(x, z)| transform.transform_point(Vec3::new(x, y, z))),
        );
    }
    for j in 1..count - 1 {
        ix.extend([
            n,
            n + j + 1,
            n + j,
            n + count,
            n + count + j,
            n + count + j + 1,
        ]);
    }
    for j in 0..count {
        let k = (j + 1) % count;
        ix.extend([
            n + j,
            n + k,
            n + j + count,
            n + k,
            n + k + count,
            n + j + count,
        ]);
    }
}

pub fn body_mesh(tails: bool) -> Mesh {
    let mut p = Vec::new();
    let mut ix = Vec::new();
    let mut colors = Vec::new();
    // Rounded dark seeker, ceramic/metal collars, joints, live-motor/warhead
    // identification bands and a recessed fixed exhaust throat.
    let rings: &[(f32, f32, [f32; 4])] = &[
        (-1.51, 0.002, [0.025, 0.045, 0.055, 1.]),
        (-1.49, 0.025, [0.03, 0.055, 0.065, 1.]),
        (-1.44, 0.047, [0.04, 0.07, 0.075, 1.]),
        (-1.37, 0.060, [0.06, 0.09, 0.095, 1.]),
        (-1.30, 0.0635, [0.08, 0.11, 0.115, 1.]),
        (-1.28, 0.064, [0.48, 0.50, 0.47, 1.]),
        (-1.255, 0.0635, [0.78, 0.79, 0.74, 1.]),
        (-1.22, 0.0635, [0.86, 0.87, 0.83, 1.]),
        (-0.65, 0.0635, [0.86, 0.87, 0.83, 1.]),
        (-0.64, 0.064, [0.27, 0.28, 0.26, 1.]),
        (-0.63, 0.0635, [0.84, 0.85, 0.80, 1.]),
        (-0.51, 0.0635, [0.84, 0.85, 0.80, 1.]),
        (-0.505, 0.0637, [0.70, 0.51, 0.07, 1.]),
        (-0.475, 0.0637, [0.70, 0.51, 0.07, 1.]),
        (-0.47, 0.0635, [0.84, 0.85, 0.80, 1.]),
        (0.38, 0.0635, [0.84, 0.85, 0.80, 1.]),
        (0.385, 0.0637, [0.25, 0.12, 0.055, 1.]),
        (0.415, 0.0637, [0.25, 0.12, 0.055, 1.]),
        (0.42, 0.0635, [0.82, 0.83, 0.78, 1.]),
        (0.88, 0.0635, [0.82, 0.83, 0.78, 1.]),
        (0.89, 0.064, [0.33, 0.34, 0.31, 1.]),
        (0.91, 0.0635, [0.51, 0.53, 0.49, 1.]),
        (1.39, 0.0635, [0.48, 0.49, 0.44, 1.]),
        (1.48, 0.060, [0.25, 0.26, 0.23, 1.]),
        (1.51, 0.054, [0.36, 0.37, 0.32, 1.]),
        (1.51, 0.042, [0.20, 0.19, 0.16, 1.]),
        (1.38, 0.033, [0.065, 0.06, 0.045, 1.]),
        (1.35, 0.002, [0.025, 0.022, 0.015, 1.]),
    ];
    for &(z, r, color) in rings {
        for j in 0..SIDES {
            let a = j as f32 * std::f32::consts::TAU / SIDES as f32;
            p.push(Vec3::new(a.cos() * r, a.sin() * r, z));
            let grain = 1. - 0.025 * (j as f32 * 2.399 + z * 23.).sin().abs();
            colors.push([color[0] * grain, color[1] * grain, color[2] * grain, 1.]);
        }
    }
    for i in 0..rings.len() - 1 {
        for j in 0..SIDES {
            let a = (i * SIDES + j) as u32;
            let b = (i * SIDES + (j + 1) % SIDES) as u32;
            ix.extend([
                a,
                b,
                a + SIDES as u32,
                b,
                b + SIDES as u32,
                a + SIDES as u32,
            ]);
        }
    }
    for (n, flip) in [(0_u32, true), (((rings.len() - 1) * SIDES) as u32, false)] {
        for j in 1..SIDES as u32 - 1 {
            ix.extend(if flip {
                [n, n + j + 1, n + j]
            } else {
                [n, n + j, n + j + 1]
            });
        }
    }
    for quadrant in 0..4 {
        let rotation = Quat::from_rotation_z(quadrant as f32 * std::f32::consts::FRAC_PI_2);
        fin(
            &mut p,
            &mut ix,
            &[
                (0.061, -1.02),
                (0.145, -0.91),
                (0.160, -0.69),
                (0.061, -0.67),
            ],
            0.0025,
            Transform::from_rotation(rotation),
        );
        if tails {
            fin(
                &mut p,
                &mut ix,
                &tail_outline(),
                0.003,
                Transform::from_translation(rotation * Vec3::X * 0.061 + Vec3::Z * 1.20)
                    .with_rotation(rotation),
            );
        }
        // Exposed circular tail-shaft bearings remain on the fixed airframe.
        let first = p.len() as u32;
        for x in [0.061, 0.073] {
            for j in 0..16 {
                let angle = j as f32 * std::f32::consts::TAU / 16.;
                p.push(rotation * Vec3::new(x, 0.012 * angle.cos(), 1.20 + 0.012 * angle.sin()));
            }
        }
        for j in 0..16 {
            let k = (j + 1) % 16;
            ix.extend([
                first + j,
                first + k,
                first + j + 16,
                first + k,
                first + k + 16,
                first + j + 16,
            ]);
        }
    }
    // Low dorsal wiring cover, tapered ends, not a floating rectangular block.
    fin(
        &mut p,
        &mut ix,
        &[(0.062, -1.15), (0.070, -1.07), (0.070, 1.20), (0.062, 1.31)],
        0.008,
        Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
    );
    colors.resize(p.len(), [0.58, 0.60, 0.55, 1.]);
    let mut result = super::mesh(p, ix);
    result.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    result
}
fn tail_outline() -> [(f32, f32); 4] {
    [(0., -0.25), (0.128, -0.10), (0.164, 0.18), (0., 0.25)]
}

/// Unit-length plume: smooth contracting core, small shock cells, turbulent
/// sheath and alpha falloff baked into shared meshes. Never an opaque cone.
fn plume_mesh(layer: usize) -> Mesh {
    let mut p = Vec::new();
    let mut ix = Vec::new();
    let mut colors = Vec::new();
    let radial = 24;
    let axial = 64;
    for i in 0..=axial {
        let u = i as f32 / axial as f32;
        let envelope = (1. - u).max(0.).powf(0.65);
        let shock = (u * std::f32::consts::TAU * 5.0).cos() * 0.5 + 0.5;
        let r = (0.023 + u * 0.065) * (0.72 + 0.28 * shock) * envelope;
        for j in 0..=radial {
            let angle = j as f32 / radial as f32 * std::f32::consts::TAU;
            let ripple = 1. + 0.045 * (angle * 5. + u * 83. + layer as f32).sin() * u;
            p.push(Vec3::new(
                r * angle.cos() * ripple,
                r * angle.sin() * ripple,
                u,
            ));
            let alpha = (1. - u).powf(1.8) * (0.55 + 0.45 * shock);
            colors.push([1., 1., 1., alpha]);
        }
    }
    for i in 0..axial {
        for j in 0..radial {
            let a = (i * (radial + 1) + j) as u32;
            let b = a + (radial + 1) as u32;
            ix.extend([a, a + 1, b, a + 1, b + 1, b]);
        }
    }
    let mut result = super::mesh(p, ix);
    result.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    result
}
fn smoke_pixels() -> Vec<u8> {
    let mut data = Vec::with_capacity(128 * 128 * 4);
    for y in 0..128 {
        for x in 0..128 {
            let u = (x as f32 + 0.5) / 64. - 1.;
            let v = (y as f32 + 0.5) / 64. - 1.;
            let r = (u * u + v * v).sqrt();
            let noise = 0.68
                + 0.13 * (u * 15. + (v * 11.).sin()).sin()
                + 0.10 * (v * 27. + (u * 19.).cos()).cos()
                + 0.06 * (u * 57. + v * 41.).sin();
            let feather = ((1. - r) / 0.28).clamp(0., 1.);
            let alpha = (-2. * r * r).exp() * feather * feather * noise.clamp(0.2, 1.);
            let shade = (222. + 26. * noise).clamp(0., 255.) as u8;
            data.extend([shade, shade, shade, (alpha * 230.) as u8]);
        }
    }
    data
}
pub fn prepare(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) -> VisualAssets {
    let mut p = Vec::new();
    let mut ix = Vec::new();
    fin(
        &mut p,
        &mut ix,
        &tail_outline(),
        0.003,
        Transform::default(),
    );
    let tail = meshes.add(super::mesh(p, ix));
    let mut p = Vec::new();
    let mut ix = Vec::new();
    fin(
        &mut p,
        &mut ix,
        &[
            (-0.053, -0.04),
            (-0.010, -0.04),
            (-0.010, 0.04),
            (-0.053, 0.04),
        ],
        0.0018,
        Transform::default(),
    );
    let vane = meshes.add(super::mesh(p, ix));
    let metal = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.43, 0.38),
        metallic: 0.80,
        perceptual_roughness: 0.35,
        cull_mode: None,
        ..default()
    });
    let mut lens_points = Vec::new();
    let mut lens_ix = Vec::new();
    for (z, r) in [
        (-1.511, 0.002),
        (-1.49, 0.0252),
        (-1.44, 0.0472),
        (-1.37, 0.0602),
        (-1.301, 0.0637),
    ] {
        for j in 0..SIDES {
            let a = j as f32 * std::f32::consts::TAU / SIDES as f32;
            lens_points.push(Vec3::new(r * a.cos(), r * a.sin(), z));
        }
    }
    for i in 0..4 {
        for j in 0..SIDES {
            let a = (i * SIDES + j) as u32;
            let b = (i * SIDES + (j + 1) % SIDES) as u32;
            lens_ix.extend([
                a,
                b,
                a + SIDES as u32,
                b,
                b + SIDES as u32,
                a + SIDES as u32,
            ]);
        }
    }
    let lens = meshes.add(super::mesh(lens_points, lens_ix));
    let glass = materials.add(StandardMaterial {
        base_color: Color::srgb(0.055, 0.085, 0.075),
        metallic: 0.35,
        perceptual_roughness: 0.12,
        cull_mode: None,
        ..default()
    });
    let plume_materials = [
        (
            Color::srgba(1., 0.94, 0.74, 0.75),
            LinearRgba::new(12., 9., 4., 1.),
        ),
        (
            Color::srgba(0.72, 0.82, 1., 0.24),
            LinearRgba::new(3., 4., 6., 1.),
        ),
        (
            Color::srgba(1., 0.61, 0.28, 0.16),
            LinearRgba::new(4., 1.7, 0.5, 1.),
        ),
    ]
    .map(|(base_color, emissive)| {
        materials.add(StandardMaterial {
            base_color,
            emissive,
            unlit: true,
            alpha_mode: AlphaMode::Add,
            cull_mode: None,
            ..default()
        })
    });
    let mut texture = Image::new_fill(
        Extent3d {
            width: 128,
            height: 128,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    texture.data = Some(smoke_pixels());
    let smoke = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(texture)),
        base_color: Color::srgba(0.87, 0.89, 0.91, 0.62),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    });
    VisualAssets {
        plumes: std::array::from_fn(|i| meshes.add(plume_mesh(i))),
        plume_materials,
        smoke,
        tail,
        vane,
        metal,
        lens,
        glass,
    }
}
pub fn install(c: &mut Commands, root: Entity, missile: usize, assets: &VisualAssets) {
    let lens = c
        .spawn((
            Finish(missile),
            Mesh3d(assets.lens.clone()),
            MeshMaterial3d(assets.glass.clone()),
            Transform::default(),
            Visibility::Hidden,
        ))
        .id();
    c.entity(root).add_child(lens);
    for quadrant in 0..4 {
        for vane in [false, true] {
            let mesh = if vane { &assets.vane } else { &assets.tail };
            let joint = c
                .spawn((
                    Actuator {
                        missile,
                        quadrant,
                        vane,
                    },
                    actuator_transform(quadrant, 0., vane),
                    Visibility::Hidden,
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(assets.metal.clone()),
                    bevy::light::NotShadowCaster,
                ))
                .id();
            c.entity(root).add_child(joint);
        }
    }
}
pub(super) fn local_control(m: &Missile) -> Vec3 {
    let frame = DQuat::from_rotation_arc(m.gnc.attitude * -DVec3::Z,
        m.gnc.body.normalize_or(-DVec3::Z)) * m.gnc.attitude;
    let velocity = (m.velocity - m.gnc.wind).normalize_or_zero();
    let normal = m.gnc.fins - velocity * m.gnc.fins.dot(velocity);
    (frame.conjugate() * normal).as_vec3().clamp_length_max(1.)
}
pub(super) fn fin_angle(control: Vec3, quadrant: usize) -> f32 {
    let radial = Quat::from_rotation_z(quadrant as f32 * std::f32::consts::FRAC_PI_2) * Vec3::X;
    // Aft controls initially push the tail away from the requested turn,
    // producing the moment that points the nose into it. GNC exports a
    // normalized demand, not a calibrated physical shaft-angle measurement.
    control.dot(radial.cross(Vec3::Z)).clamp(-1., 1.) * FIN_LIMIT
}
fn joint_transform(quadrant: usize, angle: f32) -> Transform {
    let radial = Quat::from_rotation_z(quadrant as f32 * std::f32::consts::FRAC_PI_2);
    Transform::from_translation(radial * Vec3::X * 0.061 + Vec3::Z * 1.20)
        .with_rotation(radial * Quat::from_rotation_x(angle))
}
fn actuator_transform(quadrant: usize, angle: f32, vane: bool) -> Transform {
    let mut transform = joint_transform(quadrant, angle);
    if vane {
        transform.translation.z = 1.46;
    }
    transform
}
pub fn animate(
    combat: Res<Combat>,
    mut parts: Query<(&Actuator, &mut Transform, &mut Visibility), (Without<Finish>,Without<super::radar_visual::Tail>,super::MechanismFilter)>,
    mut finishes: Query<(&Finish, &mut Visibility), (Without<Actuator>,Without<super::radar_visual::Tail>,super::MechanismFilter)>,
) {
    for (part, mut visibility) in &mut finishes {
        visibility.set_if_neq(
            if combat.active
                && combat
                    .render_missile(part.0)
                    .is_some_and(|m| m.alive && m.gnc.aim9x)
            {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
        );
    }
    for (part, mut transform, mut visibility) in &mut parts {
        let missile = combat
            .render_missile(part.missile)
            .filter(|m| combat.active && m.alive && m.gnc.aim9x);
        visibility.set_if_neq(if missile.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if let Some(m) = missile {
            let control = if m.age > 0.18 {
                local_control(m)
            } else {
                Vec3::ZERO
            };
            transform.set_if_neq(actuator_transform(
                part.quadrant,
                fin_angle(control, part.quadrant),
                part.vane,
            ));
        }
    }
}
pub fn plume_transform(m: &Missile, layer: usize) -> Transform {
    let control = local_control(m);
    let deflection = control.length() * FIN_LIMIT;
    let exhaust =
        (Vec3::Z * deflection.cos() + control.normalize_or_zero() * deflection.sin()).normalize();
    let ignition = ((m.age - 0.18) / 0.12).clamp(0., 1.) as f32;
    let pressure = (1.225 / atmosphere(m.position.length() - RADIUS).2.max(0.10)).powf(0.16) as f32;
    let pulse = 1. + 0.035 * (m.age as f32 * 79.).sin() + 0.020 * (m.age as f32 * 113.).sin();
    let length = [1.8, 2.7, 3.5][layer] * ignition * pulse;
    let width = [1., 1.8, 3.2][layer] * ignition * pressure;
    Transform::from_xyz(0., 0., 1.47)
        .with_rotation(Quat::from_rotation_arc(Vec3::Z, exhaust))
        .with_scale(Vec3::new(width, width, length))
}

/// Soft overlapping ellipses align with the projected smoke path. Six lobes
/// per retained segment keep the pool bounded and continuous even at high speed.
pub fn write_smoke(
    asset: &mut Mesh,
    m: &Missile,
    basis: DQuat,
    origin: DVec3,
    eye: Vec3,
    camera_up: Vec3,
) -> Aabb {
    use bevy::mesh::VertexAttributeValues as V;
    let mut p = match asset.remove_attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(V::Float32x3(v)) => v,
        _ => Vec::new(),
    };
    let mut col = match asset.remove_attribute(Mesh::ATTRIBUTE_COLOR) {
        Some(V::Float32x4(v)) => v,
        _ => Vec::new(),
    };
    let mut uv = match asset.remove_attribute(Mesh::ATTRIBUTE_UV_0) {
        Some(V::Float32x2(v)) => v,
        _ => Vec::new(),
    };
    p.clear();
    col.clear();
    uv.clear();
    let tail = (m.gnc.thrust > 0.)
        .then(|| m.position - m.gnc.body.normalize_or(m.velocity.normalize_or_zero()) * 1.6);
    let point_count = m.trail.len() + usize::from(tail.is_some());
    let points = m.trail.iter().copied().chain(tail);
    let all: Vec<DVec3> = points.clone().collect();
    let distance = super::smoke_distances(&all, tail.unwrap_or(m.position));
    for (i, (a, b)) in points.clone().zip(points.skip(1)).enumerate() {
        for j in 0..6 {
            let t = (j as f64 + 0.5) / 6.;
            let clearance =
                super::smoke_clearance(distance[i] + (distance[i + 1] - distance[i]) * t);
            let behind = (point_count - 2 - i) as f64 + 1. - t;
            let age = (m.trail_clock + behind * 0.075).max(0.);
            let center_world =
                a.lerp(b, t) + a.normalize() * (age * 0.40) + DVec3::X * (age * 0.25);
            let center = (basis * (center_world - origin)).as_vec3();
            let normal = (eye - center).normalize_or(Vec3::Z);
            let tangent = (basis * (b - a)).as_vec3();
            let along = (tangent - normal * tangent.dot(normal)).normalize_or(camera_up);
            let across = normal.cross(along).normalize_or(Vec3::X);
            let phase = (a.x * 0.13 + a.z * 0.19 + j as f64 * 2.4) as f32;
            let width = (0.10 + 0.64 * (age as f32).sqrt())
                * (1. + 0.16 * phase.sin())
                * if m.kind == Weapon::Radar { 1.35 } else { 1.0 };
            let half = (tangent.length() / 12. + width * 0.8).max(width);
            let center = center + across * (0.13 * width * (phase + age as f32).sin());
            let alpha = ((1. - age / 5.).clamp(0., 1.).powi(2) * 0.82) as f32 * clearance;
            for (x, y) in [(-1., -1.), (1., -1.), (1., 1.), (-1., 1.)] {
                p.push((center + along * half * y + across * width * x).to_array());
                col.push([1., 1., 1., alpha]);
                uv.push([(x + 1.) * 0.5, (y + 1.) * 0.5]);
            }
        }
    }
    let bounds = Aabb::enclosing(p.iter().map(|v| Vec3::from(*v)))
        .unwrap_or_else(|| Aabb::from_min_max(Vec3::ZERO, Vec3::ONE));
    let count = p.len();
    let mut normals = match asset.remove_attribute(Mesh::ATTRIBUTE_NORMAL) {
        Some(V::Float32x3(v)) => v,
        _ => Vec::new(),
    };
    normals.resize(count, [0., 0., 1.]);
    asset.insert_attribute(Mesh::ATTRIBUTE_POSITION, p);
    asset.insert_attribute(Mesh::ATTRIBUTE_COLOR, col);
    asset.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    asset.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    if !matches!(asset.indices(), Some(Indices::U32(_))) {
        asset.insert_indices(Indices::U32(Vec::new()));
    }
    if let Some(Indices::U32(ix)) = asset.indices_mut() {
        if ix.len() != count / 4 * 6 {
            ix.clear();
            for i in 0..count / 4 {
                let n = (i * 4) as u32;
                ix.extend([n, n + 1, n + 2, n, n + 2, n + 3]);
            }
        }
    }
    Aabb {
        half_extents: bounds.half_extents + bevy::math::Vec3A::splat(0.02),
        ..bounds
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn posed_missile() -> Missile {
        let f = super::super::flight();
        let mut combat = Combat::default();
        combat.start(&f, 1);
        combat.lock = Weapon::Heat.lock_time();
        assert!(combat.fire(&f));
        let mut m = combat.missiles.remove(0);
        m.gnc.aim9x = true;
        m.gnc.body = f.forward();
        m.age = 1.;
        m.gnc.thrust = 14000.;
        m.gnc.fins = super::super::facing(m.gnc.body, m.position.normalize()) * DVec3::Y * 0.7;
        m.trail = vec![
            m.position - m.gnc.body * 80.,
            m.position - m.gnc.body * 40.,
            m.position,
        ];
        m
    }
    #[test]
    fn exhaust_bends_for_aft_control_moment_and_pause_is_deterministic() {
        let m = posed_missile();
        let a = plume_transform(&m, 0);
        let b = plume_transform(&m, 0);
        assert_eq!(a, b);
        assert!((a.rotation * Vec3::Z).y > 0.);
        assert!(a.translation.z < 1.51 && a.translation.z > 1.4);
        let mut neutral = m.clone();
        neutral.gnc.fins = DVec3::ZERO;
        assert!((plume_transform(&neutral, 0).rotation * Vec3::Z - Vec3::Z).length() < 1e-6);
    }
    #[test]
    fn vane_hinges_stay_inside_fixed_exhaust() {
        for i in 0..4 {
            for angle in [-FIN_LIMIT, 0., FIN_LIMIT] {
                let t = actuator_transform(i, angle, true);
                for x in [-0.053, -0.010] {
                    for z in [-0.04, 0.04] {
                        let p = t.transform_point(Vec3::new(x, 0., z));
                        assert!(p.xy().length() < 0.055, "vane outside exhaust: {p:?}");
                        assert!(p.z > 1.40 && p.z < 1.51);
                    }
                }
            }
        }
    }
    #[test]
    fn smoke_is_bounded_reuses_storage_and_tracks_camera_while_paused() {
        let m = posed_missile();
        let origin = m.position;
        let mut asset = super::super::mesh(vec![], vec![]);
        let eye = Vec3::new(8., 4., 2.);
        let bounds = write_smoke(&mut asset, &m, DQuat::IDENTITY, origin, eye, Vec3::Y);
        let values = |asset: &Mesh| {
            let bevy::mesh::VertexAttributeValues::Float32x3(v) =
                asset.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
            else {
                panic!()
            };
            v.clone()
        };
        let first = values(&asset);
        assert!(first.len() <= 48 * 6 * 4);
        assert!(first.iter().all(|p| {
            (Vec3::from(*p) - Vec3::from(bounds.center))
                .abs()
                .cmple(Vec3::from(bounds.half_extents))
                .all()
        }));
        write_smoke(&mut asset, &m, DQuat::IDENTITY, origin, eye, Vec3::Y);
        assert_eq!(first, values(&asset));
        write_smoke(
            &mut asset,
            &m,
            DQuat::IDENTITY,
            origin,
            Vec3::new(-8., 2., 1.),
            Vec3::Y,
        );
        assert_ne!(first, values(&asset));
        assert!(asset.indices().unwrap().iter().all(|i| i < first.len()));
    }
    #[test]
    fn paired_fins_generate_consistent_pitch_and_yaw_signs() {
        assert!(fin_angle(Vec3::Y, 0) < 0.);
        assert!(fin_angle(Vec3::Y, 2) > 0.);
        assert!(fin_angle(Vec3::X, 1) > 0.);
        assert!(fin_angle(Vec3::X, 3) < 0.);
        for i in 0..4 {
            assert!(fin_angle(Vec3::splat(9.), i).abs() <= FIN_LIMIT);
        }
    }
    #[test]
    fn hinges_do_not_translate_when_fins_rotate() {
        for i in 0..4 {
            let a = joint_transform(i, -FIN_LIMIT);
            let b = joint_transform(i, FIN_LIMIT);
            assert_eq!(a.translation, b.translation);
            assert!((a.rotation.length() - 1.).abs() < 1e-6);
        }
    }
    #[test]
    fn smoke_texture_is_soft_bounded_and_not_uniform() {
        let p = smoke_pixels();
        assert_eq!(p.len(), 128 * 128 * 4);
        assert_eq!(p[3], 0);
        assert!(p[(64 * 128 + 64) * 4 + 3] > 80);
        assert!(p.chunks(4).map(|c| c[3]).any(|a| a > 0 && a < 80));
    }
    #[test]
    fn model_is_sealed_indexed_and_has_solid_fins() {
        let m = body_mesh(true);
        let bevy::mesh::VertexAttributeValues::Float32x3(p) =
            m.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
        else {
            panic!()
        };
        assert!(p.len() > 1800);
        assert!((p.iter().map(|p| p[2]).fold(f32::INFINITY, f32::min) + 1.51).abs() < 1e-5);
        assert!(p.iter().all(|p| p.iter().all(|v| v.is_finite())));
        assert!(m.indices().unwrap().iter().all(|i| i < p.len()));
    }
}
