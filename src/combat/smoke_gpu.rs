//! Retained trail samples; expansion, drift and camera-facing cards run on the GPU.
use super::*;
use bevy::{
    asset::{load_internal_asset, uuid_handle},
    pbr::{ExtendedMaterial, MaterialExtension},
    render::render_resource::{AsBindGroup, ShaderType},
    shader::{Shader, ShaderRef},
};
const SHADER: Handle<Shader> = uuid_handle!("a23e800a-c343-4540-ad91-65b5cc7ec922");
#[derive(Clone, Copy, Debug, ShaderType, PartialEq)]
pub struct Parameters {
    time: Vec4,
    head: Vec4,
}
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct SmokeExtension {
    #[uniform(100)]
    parameters: Parameters,
}
impl MaterialExtension for SmokeExtension {
    fn vertex_shader() -> ShaderRef {
        SHADER.into()
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
}
type SmokeMaterial = ExtendedMaterial<StandardMaterial, SmokeExtension>;
#[derive(Component)]
struct Trail {
    slot: usize,
    anchor: DVec3,
    key: Option<(DVec3, DVec3, usize, bool, u64)>,
    age: f64,
}
pub struct SmokeGpuPlugin;
pub fn enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(||std::env::var_os("NORD_SMOKE_GPU").is_some() && std::env::var_os("NORD_SMOKE_CPU").is_none())
}
impl Plugin for SmokeGpuPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, SHADER, "smoke_vertex.wgsl", Shader::from_wgsl);
        app.add_plugins(MaterialPlugin::<SmokeMaterial>::default())
            .add_systems(PostStartup, setup)
            .add_systems(
                PostUpdate,
                update.before(bevy::transform::TransformSystems::Propagate),
            );
    }
}
fn setup(
    mut c: Commands,
    source: Res<aim9x::VisualAssets>,
    base: Res<Assets<StandardMaterial>>,
    mut materials: ResMut<Assets<SmokeMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if !enabled() {
        return;
    }
    let material = base.get(&source.smoke).unwrap();
    for slot in 0..RENDER_MISSILES {
        c.spawn((
            Trail {
                slot,
                anchor: DVec3::ZERO,
                key: None,
                age: 0.,
            },
            Mesh3d(meshes.add(mesh(vec![], vec![]))),
            MeshMaterial3d(materials.add(SmokeMaterial {
                base: material.clone(),
                extension: SmokeExtension {
                    parameters: Parameters {
                        time: Vec4::ZERO,
                        head: Vec4::ZERO,
                    },
                },
            })),
            Transform::default(),
            Visibility::Hidden,
            bevy::light::NotShadowCaster,
            bevy::light::NotShadowReceiver,
            bevy::camera::visibility::NoAutoAabb,
            Aabb::from_min_max(Vec3::ZERO, Vec3::ONE),
        ));
    }
}
fn update(
    f: Res<Flight>,
    combat: Res<Combat>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<SmokeMaterial>>,
    mut trails: Query<(
        &mut Trail,
        &Mesh3d,
        &MeshMaterial3d<SmokeMaterial>,
        &mut Transform,
        &mut Visibility,
        &mut Aabb,
    )>,
) {
    for (mut trail, handle, material, mut transform, mut visibility, mut bounds) in &mut trails {
        let missile = combat.render_missile(trail.slot).filter(|m| {
            combat.active
                && m.alive
                && (m.gnc.aim9x || m.kind == Weapon::Radar)
                && m.trail.len() > 2
                && m.age < (if m.gnc.long_range { 24. } else { m.kind.burn() }) + 3.8
        });
        visibility.set_if_neq(if missile.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        let Some(m) = missile else {
            trail.key = None;
            continue;
        };
        let powered = m.gnc.thrust > 0.;
        let key = (
            *m.trail.first().unwrap(),
            *m.trail.last().unwrap(),
            m.trail.len(),
            powered,
            combat.generation,
        );
        if trail.key != Some(key) || m.age < trail.age {
            trail.anchor = m.trail[0];
            *bounds = encode(meshes.get_mut(&handle.0).unwrap(), m, trail.anchor);
            trail.key = Some(key);
        }
        trail.age = m.age;
        let t = Transform::from_translation(world::local(trail.anchor, &f))
            .with_rotation(world::local_rot(DQuat::IDENTITY, &f));
        transform.set_if_neq(t);
        let head = m.position - m.gnc.body.normalize_or(m.velocity.normalize_or_zero()) * 1.6;
        let parameters = Parameters {
            time: Vec4::new(
                m.age as f32,
                if m.kind == Weapon::Radar { 1.35 } else { 1. },
                0.,
                0.,
            ),
            head: (head - trail.anchor).as_vec3().extend(0.),
        };
        if materials
            .get(&material.0)
            .is_some_and(|a| a.extension.parameters != parameters)
        {
            materials.get_mut(&material.0).unwrap().extension.parameters = parameters;
        }
    }
}
fn encode(asset: &mut Mesh, m: &Missile, anchor: DVec3) -> Aabb {
    use bevy::mesh::VertexAttributeValues as V;
    let mut p = match asset.remove_attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(V::Float32x3(v)) => v,
        _ => vec![],
    };
    let mut normals = match asset.remove_attribute(Mesh::ATTRIBUTE_NORMAL) {
        Some(V::Float32x3(v)) => v,
        _ => vec![],
    };
    let mut colors = match asset.remove_attribute(Mesh::ATTRIBUTE_COLOR) {
        Some(V::Float32x4(v)) => v,
        _ => vec![],
    };
    let mut uv = match asset.remove_attribute(Mesh::ATTRIBUTE_UV_0) {
        Some(V::Float32x2(v)) => v,
        _ => vec![],
    };
    let mut up = match asset.remove_attribute(Mesh::ATTRIBUTE_TANGENT) {
        Some(V::Float32x4(v)) => v,
        _ => vec![],
    };
    p.clear();
    normals.clear();
    colors.clear();
    uv.clear();
    up.clear();
    let tail = (m.gnc.thrust > 0.).then(|| m.position - m.gnc.body.normalize_or(m.velocity.normalize_or_zero()) * 1.6);
    let count = m.trail.len() + usize::from(tail.is_some());
    let points = m.trail.iter().copied().chain(tail);
    let mut margin = 20_f32;
    for (i, (a, b)) in points.clone().zip(points.skip(1)).enumerate() {
        let dynamic = tail.is_some() && i == count - 2;
        margin = margin.max((b - a).length() as f32 + 20.);
        for j in 0..6 {
            let t = (j as f64 + 0.5) / 6.;
            let behind = (count - 2 - i) as f64 + 1. - t;
            let birth = m.age - m.trail_clock - behind * 0.075;
            let center = if dynamic { a } else { a.lerp(b, t) };
            let phase = (a.x * 0.13 + a.z * 0.19 + j as f64 * 2.4) as f32;
            for corner in [[0., 0.], [1., 0.], [1., 1.], [0., 1.]] {
                p.push((center - anchor).as_vec3().to_array());
                normals.push((b - a).as_vec3().to_array());
                colors.push([
                    birth as f32,
                    phase,
                    0.,
                    if dynamic { t as f32 } else { -1. },
                ]);
                uv.push(corner);
                up.push(a.normalize().as_vec3().extend(1.).to_array());
            }
        }
    }
    let bounds = Aabb::enclosing(p.iter().map(|p| Vec3::from(*p)))
        .unwrap_or_else(|| Aabb::from_min_max(Vec3::ZERO, Vec3::ONE));
    let vertices = p.len();
    asset.insert_attribute(Mesh::ATTRIBUTE_POSITION, p);
    asset.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    asset.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    asset.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    asset.insert_attribute(Mesh::ATTRIBUTE_TANGENT, up);
    if !matches!(asset.indices(), Some(Indices::U32(_))) {
        asset.insert_indices(Indices::U32(vec![]));
    }
    if let Some(Indices::U32(ix)) = asset.indices_mut() {
        if ix.len() != vertices / 4 * 6 {
            ix.clear();
            for i in 0..vertices / 4 {
                let n = (i * 4) as u32;
                ix.extend([n, n + 1, n + 2, n, n + 2, n + 3]);
            }
        }
    }
    Aabb {
        half_extents: bounds.half_extents + bevy::math::Vec3A::splat(margin),
        ..bounds
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use bevy::mesh::VertexAttributeValues as V;
    #[test]
    fn retained_cards_match_cpu_reference_after_motion_without_history_rebuild() {
        let f = super::super::flight();
        let mut combat = Combat::default();
        combat.start(&f, 1);
        combat.lock = Weapon::Heat.lock_time();
        assert!(combat.fire(&f));
        let mut m = combat.missiles.remove(0);
        m.gnc.aim9x = true;
        m.age = 1.;
        m.trail_clock = 0.02;
        m.gnc.thrust = 10000.;
        m.trail = vec![
            m.position - m.velocity * 0.15,
            m.position - m.velocity * 0.075,
            m.position,
        ];
        for powered in [true, false] {
            m.gnc.thrust = if powered { 10000. } else { 0. };
            let anchor = m.trail[0];
            let mut encoded = mesh(vec![], vec![]);
            let bounds = encode(&mut encoded, &m, anchor);
            let Some(V::Float32x3(p)) = encoded.attribute(Mesh::ATTRIBUTE_POSITION) else {
                panic!()
            };
            let Some(V::Float32x3(tangent)) = encoded.attribute(Mesh::ATTRIBUTE_NORMAL) else {
                panic!()
            };
            let Some(V::Float32x4(c)) = encoded.attribute(Mesh::ATTRIBUTE_COLOR) else {
                panic!()
            };
            let Some(V::Float32x4(up)) = encoded.attribute(Mesh::ATTRIBUTE_TANGENT) else {
                panic!()
            };
            let Some(V::Float32x2(uv)) = encoded.attribute(Mesh::ATTRIBUTE_UV_0) else {
                panic!()
            };
            assert_eq!(p.len(), (m.trail.len() - 1 + usize::from(powered)) * 24);
            let mut advanced = m.clone();
            advanced.age += 0.03;
            advanced.trail_clock += 0.03;
            advanced.position += advanced.velocity * 0.03;
            let head = (advanced.position - advanced.gnc.body.normalize_or(advanced.velocity.normalize_or_zero()) * 1.6 - anchor)
                .as_vec3();
            let eye = Vec3::new(80., 20., 30.);
            let mut reference = mesh(vec![], vec![]);
            aim9x::write_smoke(
                &mut reference,
                &advanced,
                DQuat::IDENTITY,
                anchor,
                eye,
                Vec3::Y,
            );
            let Some(V::Float32x3(expected)) = reference.attribute(Mesh::ATTRIBUTE_POSITION) else {
                panic!()
            };
            for i in 0..p.len() {
                let age = (advanced.age as f32 - c[i][0]).max(0.);
                let mut center = Vec3::from(p[i]);
                let mut tangent = Vec3::from(tangent[i]);
                if c[i][3] >= 0. {
                    tangent = head - center;
                    center = center.lerp(head, c[i][3]);
                }
                center += Vec4::from(up[i]).truncate() * (age * 0.4) + Vec3::X * (age * 0.25);
                let normal = (eye - center).normalize_or(Vec3::Z);
                let along = (tangent - normal * tangent.dot(normal)).normalize_or(Vec3::Y);
                let across = normal.cross(along).normalize_or(Vec3::X);
                let width = (0.1 + 0.64 * age.sqrt()) * (1. + 0.16 * c[i][1].sin());
                let half = (tangent.length() / 12. + width * 0.8).max(width);
                center += across * (0.13 * width * (c[i][1] + age).sin());
                let actual = center
                    + along * half * (uv[i][1] * 2. - 1.)
                    + across * width * (uv[i][0] * 2. - 1.);
                assert!(
                    actual.distance(Vec3::from(expected[i])) < 0.002,
                    "card {i}: {actual:?} != {:?}",
                    expected[i]
                );
                assert!(
                    (actual - Vec3::from(bounds.center))
                        .abs()
                        .cmple(Vec3::from(bounds.half_extents))
                        .all()
                );
            }
        }
    }
}
