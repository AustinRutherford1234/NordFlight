//! Two bounded exhaust volumes and recessed radiant liners, driven by the J58 states.
use super::*;
use bevy::{
    asset::{load_internal_asset, uuid_handle},
    pbr::{ExtendedMaterial, MaterialExtension},
    render::render_resource::{AsBindGroup, Face, ShaderType},
    shader::{Shader, ShaderRef},
};
const SHADER: Handle<Shader> = uuid_handle!("3e402cb9-ddf1-4b59-a9c2-925ca42d76d2");
#[derive(Clone, Copy, Debug, ShaderType, PartialEq)]
pub struct Parameters {
    local_from_world: Mat4,
    flow: Vec4,
    shape: Vec4,
}
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct Extension {
    #[uniform(100)]
    parameters: Parameters,
}
impl MaterialExtension for Extension {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
}
type ExhaustMaterial = ExtendedMaterial<StandardMaterial, Extension>;
#[derive(Component)]
struct Volume {
    engine: usize,
}
#[derive(Component)]
struct Liner {
    engine: usize,
}
#[derive(Component)]
struct GlowLight {
    engine: usize,
}
pub struct ExhaustPlugin;
impl Plugin for ExhaustPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, SHADER, "sr71_exhaust.wgsl", Shader::from_wgsl);
        app.add_plugins(MaterialPlugin::<ExhaustMaterial>::default())
            .add_systems(PostStartup, setup)
            .add_systems(
                PostUpdate,
                update.before(bevy::transform::TransformSystems::Propagate),
            )
            .add_systems(
                PostUpdate,
                uniforms.after(bevy::transform::TransformSystems::Propagate),
            );
    }
}
/// Source nozzle pivot + measured mouth offset, in the imported aircraft's metres.
pub fn origin(engine: usize) -> Vec3 {
    Vec3::new(if engine == 0 { -4.355 } else { 4.355 }, 0.489, 11.08)
}
#[derive(Clone, Copy, Debug)]
struct Appearance {
    lit: bool,
    wet: f32,
    power: f32,
    length: f32,
    radius: f32,
    spacing: f32,
    radiance: f32,
}
fn appearance(f: &Flight, i: usize) -> Appearance {
    let p = &f.systems.sr71.propulsion;
    let e = &p.sides[i];
    let lit = f.vehicle == Vehicle::Gripen
        && f.aircraft == Aircraft::Sr71
        && !f.crashed
        && f.engine
        && f.systems.engine_on[i]
        && p.initialized
        && e.mode == crate::aircraft::sr71::flight::engine::state::Mode::Running
        && e.thrust_n > 1.;
    let wet = if lit {
        e.afterburner_fraction.clamp(0., 1.) as f32
    } else {
        0.
    };
    let power = if lit { e.visual_power() as f32 } else { 0. };
    let motion = crate::plume_motion::sample(f.elapsed, (i * 73) as u32);
    let radius = 0.70 * (1. + f.systems.sr71.nozzles[i] as f32 * 0.05);
    Appearance {
        lit,
        wet,
        power,
        length: (3.0 + wet * 10.) * motion.length,
        radius,
        spacing: (radius * 1.65 * ((e.output.exit_mach as f32).powi(2) - 1.).max(0.7).sqrt())
            .clamp(0.9, 2.3),
        radiance: motion.radiance,
    }
}
fn liner_mesh() -> Mesh {
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    for ring in 0..=20 {
        let r = ring as f32 / 20.;
        for side in 0..=64 {
            let a = side as f32 / 64. * std::f32::consts::TAU;
            positions.push([r * a.cos(), r * a.sin(), -0.13 * (1. - r * r)]);
            normals.push([0., 0., 1.]);
            let holder = (0.5 + 0.5 * (r * std::f32::consts::TAU * 3.).cos()).powi(2);
            let ribs = (0.5 + 0.5 * (a * 24.).cos()).powi(8);
            let fade = (1. - r).powf(0.55) * (0.20 + 0.25 * holder + 0.08 * ribs) * (0.4 + 0.6 * r);
            colors.push([1., 0.9 + 0.1 * holder, 0.8 + 0.2 * holder, fade]);
            if ring < 20 && side < 64 {
                let n = ring * 65 + side;
                indices.extend_from_slice(&[n, n + 1, n + 65, n + 1, n + 66, n + 65]);
            }
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices))
}
fn setup(
    mut c: Commands,
    roots: Query<(Entity, &Root)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ExhaustMaterial>>,
    mut standard: ResMut<Assets<StandardMaterial>>,
) {
    let Some((root, _)) = roots.iter().find(|(_, r)| r.0 == Aircraft::Sr71) else {
        return;
    };
    // One retained shared bounding mesh, one shared liner mesh, four total draws.
    let mut cube = Mesh::from(Cuboid::new(2., 2., 1.));
    cube.translate_by(Vec3::Z * 0.5);
    let cube = meshes.add(cube);
    let liner = meshes.add(liner_mesh());
    for engine in 0..2 {
        let material = materials.add(ExhaustMaterial {
            base: StandardMaterial {
                alpha_mode: AlphaMode::Add,
                cull_mode: Some(Face::Front),
                unlit: true,
                ..default()
            },
            extension: Extension {
                parameters: Parameters {
                    local_from_world: Mat4::IDENTITY,
                    flow: Vec4::ZERO,
                    shape: Vec4::ONE,
                },
            },
        });
        c.spawn((
            Name::new(format!("SR-71 J58 exhaust volume {engine}")),
            Volume { engine },
            Mesh3d(cube.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(origin(engine)),
            Visibility::Hidden,
            ChildOf(root),
            bevy::light::NotShadowCaster,
            bevy::light::NotShadowReceiver,
        ));
        let glow = standard.add(StandardMaterial {
            base_color: Color::NONE,
            alpha_mode: AlphaMode::Add,
            cull_mode: None,
            unlit: true,
            ..default()
        });
        c.spawn((
            Name::new(format!("SR-71 recessed J58 liner {engine}")),
            Liner { engine },
            Mesh3d(liner.clone()),
            MeshMaterial3d(glow),
            Transform::from_translation(origin(engine) + Vec3::Z * 0.035),
            Visibility::Hidden,
            ChildOf(root),
            bevy::light::NotShadowCaster,
            bevy::light::NotShadowReceiver,
        ));
        c.spawn((
            GlowLight { engine },
            PointLight {
                intensity: 0.,
                range: 8.,
                radius: 0.55,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_translation(origin(engine) + Vec3::Z * 0.10),
            Visibility::Inherited,
            ChildOf(root),
        ));
    }
}
fn update(
    f: Res<Flight>,
    mut volumes: Query<(&Volume, &mut Transform, &mut Visibility), Without<Liner>>,
    mut liners: Query<
        (
            &Liner,
            &mut Transform,
            &mut Visibility,
            &MeshMaterial3d<StandardMaterial>,
        ),
        Without<Volume>,
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut lights: Query<(&GlowLight, &mut PointLight)>,
) {
    for (v, mut t, mut vis) in &mut volumes {
        let a = appearance(&f, v.engine);
        vis.set_if_neq(if a.lit && a.wet > 0.004 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        t.set_if_neq(
            Transform::from_translation(origin(v.engine)).with_scale(Vec3::new(
                a.radius * 1.65,
                a.radius * 1.65,
                a.length,
            )),
        );
    }
    for (v, mut t, mut vis, handle) in &mut liners {
        let a = appearance(&f, v.engine);
        vis.set_if_neq(if a.lit {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if !a.lit {
            continue;
        }
        let mut next = *t;
        next.scale = Vec3::splat(a.radius * 0.96);
        t.set_if_neq(next);
        let intensity = (0.10 + a.power * 0.13 + a.wet * 1.8) * a.radiance;
        let color = Color::linear_rgba(
            2.6 + a.wet * 2.,
            0.07 + a.wet * 1.3,
            0.012 + a.wet * 0.45,
            intensity.min(0.8),
        );
        let emissive = LinearRgba::new(4., 0.12 + a.wet * 2., 0.025 + a.wet * 0.8, 1.) * intensity;
        if materials
            .get(&handle.0)
            .is_some_and(|m| m.base_color != color || m.emissive != emissive)
        {
            if let Some(m) = materials.get_mut(&handle.0) {
                m.base_color = color;
                m.emissive = emissive;
            }
        }
    }
    for (v, mut l) in &mut lights {
        let a = appearance(&f, v.engine);
        let intensity = if a.lit {
            (10. + a.power * 12. + a.wet * 2600.) * a.radiance
        } else {
            0.
        };
        let color = Color::srgb(1., 0.09 + a.wet * 0.28, 0.025 + a.wet * 0.20);
        if l.intensity != intensity {
            l.intensity = intensity;
        }
        if l.color != color {
            l.color = color;
        }
    }
}
fn uniforms(
    f: Res<Flight>,
    volumes: Query<(
        &Volume,
        &GlobalTransform,
        &MeshMaterial3d<ExhaustMaterial>,
        &Visibility,
    )>,
    mut materials: ResMut<Assets<ExhaustMaterial>>,
) {
    for (v, global, handle, vis) in &volumes {
        if *vis == Visibility::Hidden {
            continue;
        }
        let a = appearance(&f, v.engine);
        let parameters = Parameters {
            local_from_world: global.to_matrix().inverse(),
            flow: Vec4::new(
                (f.elapsed % std::f64::consts::TAU) as f32,
                a.wet,
                a.radiance,
                v.engine as f32 * 2.73,
            ),
            shape: Vec4::new(a.radius, a.length, a.spacing, 0.),
        };
        if materials
            .get(&handle.0)
            .is_some_and(|m| m.extension.parameters != parameters)
        {
            if let Some(m) = materials.get_mut(&handle.0) {
                m.extension.parameters = parameters;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn engine_states_drive_independent_plumes_and_shutdown() {
        let mut f = Flight::approach();
        f.aircraft = Aircraft::Sr71;
        f.throttle = 1.;
        crate::aircraft::sr71::flight::engine::initialize(&mut f);
        assert!(appearance(&f, 0).wet > 0.99);
        f.systems.sr71.propulsion.sides[1].mode =
            crate::aircraft::sr71::flight::engine::state::Mode::Flameout;
        assert!(appearance(&f, 0).lit);
        assert!(!appearance(&f, 1).lit);
        f.engine = false;
        assert!(!appearance(&f, 0).lit);
    }
    #[test]
    fn exhaust_starts_within_imported_mouth_and_stays_finite() {
        let mut f = Flight::approach();
        f.aircraft = Aircraft::Sr71;
        f.throttle = 0.5;
        crate::aircraft::sr71::flight::engine::initialize(&mut f);
        let a = appearance(&f, 0);
        assert!(a.lit && a.wet == 0.);
        assert!((10.71..11.22).contains(&origin(0).z));
        assert!(a.radius < 0.845);
        assert!(a.length.is_finite());
        let m = liner_mesh();
        assert!(m.count_vertices() < 1500);
    }
}
#[cfg(test)]
mod integration_tests {
    use super::*;
    #[test]
    fn retained_geometry_follows_world_transform_and_resets_without_accumulating_entities() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TransformPlugin));
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<ExhaustMaterial>>();
        let mut f = Flight::approach();
        f.aircraft = Aircraft::Sr71;
        f.throttle = 1.;
        crate::aircraft::sr71::flight::engine::initialize(&mut f);
        app.insert_resource(f)
            .add_systems(Startup, setup)
            .add_systems(Update, update)
            .add_systems(
                PostUpdate,
                uniforms.after(bevy::transform::TransformSystems::Propagate),
            );
        let root = app
            .world_mut()
            .spawn((
                Root(Aircraft::Sr71),
                Transform::from_xyz(11., 23., 35.).with_rotation(Quat::from_rotation_y(0.7)),
                Visibility::Inherited,
            ))
            .id();
        app.update();
        let w = app.world_mut();
        let mut q = w.query::<(
            &Volume,
            &GlobalTransform,
            &MeshMaterial3d<ExhaustMaterial>,
            &Visibility,
        )>();
        assert_eq!(q.iter(w).count(), 2);
        for (_, global, h, v) in q.iter(w) {
            assert_eq!(*v, Visibility::Inherited);
            let m = w.resource::<Assets<ExhaustMaterial>>().get(&h.0).unwrap();
            assert!(
                (m.extension.parameters.local_from_world * global.to_matrix())
                    .abs_diff_eq(Mat4::IDENTITY, 1e-4)
            );
        }
        let count = w.entities().len();
        w.resource_mut::<Flight>().engine = false;
        app.update();
        let w = app.world_mut();
        for (_, _, _, v) in q.iter(w) {
            assert_eq!(*v, Visibility::Hidden);
        }
        w.entity_mut(root)
            .get_mut::<Transform>()
            .unwrap()
            .translation += Vec3::splat(200.);
        {
            let mut f = w.resource_mut::<Flight>();
            f.engine = true;
            f.elapsed = 0.;
            crate::aircraft::sr71::flight::engine::initialize(&mut f);
        }
        app.update();
        assert_eq!(app.world().entities().len(), count);
    }
}
